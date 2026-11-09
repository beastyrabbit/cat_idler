using System;
using System.IO;
using System.Text;
using System.Security.Cryptography;
using System.Runtime.InteropServices;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace IdleCatForest.Authority
{
    public static class SaveStore
    {
        public const int Version = 1;
        public const long MaximumBytes = 256L * 1024 * 1024;
        public static void Save<T>(string path, T state, bool requireNew = false)
            => Save(path, state, requireNew, null);
        internal static void Save<T>(string path, T state, bool requireNew, SaveTiming timing)
        {
            if (state == null) throw new ArgumentNullException(nameof(state));
            timing?.Begin(SaveStage.PayloadEncode);
            var payload = WireJson.Encode(state);
            timing?.EndStage(true);
            timing?.Begin(SaveStage.PayloadUtf8Hash);
            var document = new SaveDocument { Payload = payload, Sha256 = Hash(payload, timing) };
            timing?.EndStage(true);
            timing?.Begin(SaveStage.EnvelopeEncode);
            var envelope = WireJson.Encode(document);
            timing?.EndStage(true);
            AtomicFile.Write(path, envelope, requireNew, timing);
        }
        public static void Save<T>(string path, T state, out SaveTiming timing, bool requireNew = false)
        {
            timing = SaveTiming.Start();
            var success = false;
            try { Save(path, state, requireNew, timing); success = true; }
            catch { timing.EndStage(false); throw; }
            finally { timing.Complete(success); }
        }
        public static T Load<T>(string path)
        {
            var file = new FileInfo(path);
            if (file.Length > MaximumBytes) throw new InvalidDataException("Save exceeds the supported size.");
            try
            {
                var document = WireJson.Decode<SaveDocument>(File.ReadAllText(path));
                if (document == null || document.Format != "idle-cat-forest-unity" || document.Version != Version)
                    throw new InvalidDataException("Unsupported save format or version. The original was left untouched.");
                if (document.Payload == null || !string.Equals(document.Sha256, Hash(document.Payload), StringComparison.Ordinal))
                    throw new InvalidDataException("Save checksum does not match. The original was left untouched.");
                return WireJson.Decode<T>(document.Payload);
            }
            catch (JsonException error) { throw new InvalidDataException("Save JSON is invalid. The original was left untouched.", error); }
        }
        private static string Hash(string value, SaveTiming timing = null)
        {
            using (var hash = SHA256.Create())
            {
                const int maximumCharacters = 4096;
                var buffer = new byte[Encoding.UTF8.GetMaxByteCount(Math.Min(maximumCharacters, value.Length))];
                long totalBytes = 0;
                for (int offset = 0; offset < value.Length;)
                {
                    int count = Math.Min(maximumCharacters, value.Length - offset);
                    // Keep a valid pair together so chunking retains the v1 UTF-8 fallback behavior.
                    if (offset + count < value.Length && char.IsHighSurrogate(value[offset + count - 1]) && char.IsLowSurrogate(value[offset + count])) count--;
                    int bytes = Encoding.UTF8.GetBytes(value, offset, count, buffer, 0);
                    hash.TransformBlock(buffer, 0, bytes, buffer, 0);
                    totalBytes += bytes;
                    offset += count;
                }
                if (timing != null) timing.PayloadBytes = totalBytes;
                hash.TransformFinalBlock(Array.Empty<byte>(), 0, 0);
                return SessionAuthority.Hex(hash.Hash);
            }
        }
        [Serializable]
        private sealed class SaveDocument
        {
            public string Format = "idle-cat-forest-unity";
            public int Version = SaveStore.Version;
            public string Payload = "", Sha256 = "";
        }
    }

    public static class WireJson
    {
        private static readonly JsonSerializerSettings Settings = new JsonSerializerSettings
        {
            TypeNameHandling = TypeNameHandling.None,
            MetadataPropertyHandling = MetadataPropertyHandling.Ignore,
            MissingMemberHandling = MissingMemberHandling.Error,
            ReferenceLoopHandling = ReferenceLoopHandling.Error,
            MaxDepth = 128,
            Culture = System.Globalization.CultureInfo.InvariantCulture,
            Converters = { new FiniteNumberConverter() }
        };
        public static string Encode(object value) => JsonConvert.SerializeObject(value, Settings);
        public static T Decode<T>(string value)
        {
            using (var reader = new JsonTextReader(new StringReader(value)) { MaxDepth = 128, DateParseHandling = DateParseHandling.None })
            {
                var token = JToken.Load(reader, new JsonLoadSettings { DuplicatePropertyNameHandling = DuplicatePropertyNameHandling.Error });
                CheckFinite(token);
                if (reader.Read()) throw new JsonException("Multiple JSON roots are not supported.");
                return token.ToObject<T>(JsonSerializer.Create(Settings));
            }
        }
        private static void CheckFinite(JToken token)
        {
            if (token is JObject obj)
            {
                var names = new System.Collections.Generic.HashSet<string>(StringComparer.OrdinalIgnoreCase);
                foreach (var property in obj.Properties())
                    if (!names.Add(property.Name)) throw new JsonException("Duplicate JSON member.");
            }
            if (token.Type == JTokenType.Float)
            {
                var number = token.Value<double>();
                if (double.IsInfinity(number) || double.IsNaN(number)) throw new JsonException("Non-finite numeric value.");
            }
            if (token is JContainer container) foreach (var child in container.Children()) CheckFinite(child);
        }
        public static T Clone<T>(T state) => Decode<T>(Encode(state));

        private sealed class FiniteNumberConverter : JsonConverter
        {
            public override bool CanWrite => false;
            public override bool CanConvert(Type objectType) => objectType == typeof(double) || objectType == typeof(float);
            public override object ReadJson(JsonReader reader, Type objectType, object existingValue, JsonSerializer serializer)
            {
                if (reader.TokenType != JsonToken.Integer && reader.TokenType != JsonToken.Float)
                    throw new JsonSerializationException("A finite JSON number is required.");
                double number;
                try { number = Convert.ToDouble(reader.Value, System.Globalization.CultureInfo.InvariantCulture); }
                catch (Exception error) when (error is FormatException || error is OverflowException || error is InvalidCastException)
                { throw new JsonSerializationException("Unsupported JSON number.", error); }
                if (double.IsNaN(number) || double.IsInfinity(number) || objectType == typeof(float) && float.IsInfinity((float)number))
                    throw new JsonSerializationException("Non-finite numeric value.");
                if (objectType == typeof(float)) return (float)number;
                return number;
            }
            public override void WriteJson(JsonWriter writer, object value, JsonSerializer serializer) => throw new NotSupportedException();
        }
    }

    /// <summary>Writes a private temporary file, flushes it, then atomically installs it.</summary>
    public static class AtomicFile
    {
        public static void Write(string path, string contents, bool requireNew = false)
            => Write(path, contents, requireNew, null);
        internal static void Write(string path, string contents, bool requireNew, SaveTiming timing)
        {
            timing?.Begin(SaveStage.TempSetupProtection);
            path = Path.GetFullPath(path);
            var parent = Path.GetDirectoryName(path);
            Directory.CreateDirectory(parent);
            var temporary = path + ".tmp-" + SessionAuthority.RandomHex(12);
            try
            {
                using (var stream = new FileStream(temporary, FileMode.CreateNew, FileAccess.Write, FileShare.None))
                {
                    Protect(temporary);
                    timing?.EndStage(true);
                    timing?.Begin(SaveStage.EnvelopeUtf8Write);
                    var bytes = Encoding.UTF8.GetBytes(contents);
                    if (timing != null) timing.EnvelopeBytes = bytes.Length;
                    stream.Write(bytes, 0, bytes.Length);
                    timing?.EndStage(true);
                    timing?.Begin(SaveStage.FlushClose);
                    stream.Flush(true);
                }
                timing?.EndStage(true);
                timing?.Begin(SaveStage.MoveReplace);
                if (requireNew || !File.Exists(path)) File.Move(temporary, path);
                else File.Replace(temporary, path, path + ".previous");
                timing?.EndStage(true);
                timing?.Begin(SaveStage.DirectorySync);
                FlushDirectory(parent);
                timing?.EndStage(true);
            }
            catch { timing?.EndStage(false); throw; }
            finally { if (File.Exists(temporary)) File.Delete(temporary); }
        }
        public static void Protect(string path)
        {
            if (IsUnix && chmod(path, Convert.ToUInt32("600", 8)) != 0) throw new IOException("Cannot protect private state file.");
        }
        private static bool IsUnix => Environment.OSVersion.Platform == PlatformID.Unix || Environment.OSVersion.Platform == PlatformID.MacOSX;
        private static void FlushDirectory(string path)
        {
            if (!IsUnix) return;
            var descriptor = open(path, 0);
            if (descriptor < 0) throw new IOException("Cannot open state directory for sync.");
            try { if (fsync(descriptor) != 0) throw new IOException("Cannot sync state directory."); }
            finally { close(descriptor); }
        }
        [DllImport("libc", SetLastError = true)] private static extern int chmod(string path, uint mode);
        [DllImport("libc", SetLastError = true)] private static extern int open(string path, int flags);
        [DllImport("libc", SetLastError = true)] private static extern int fsync(int descriptor);
        [DllImport("libc", SetLastError = true)] private static extern int close(int descriptor);
    }
}
