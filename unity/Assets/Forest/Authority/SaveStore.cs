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
        {
            if (state == null) throw new ArgumentNullException(nameof(state));
            var payload = WireJson.Encode(state);
            var document = new SaveDocument { Payload = payload, Sha256 = Hash(payload) };
            AtomicFile.Write(path, WireJson.Encode(document), requireNew);
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
        private static string Hash(string value)
        {
            using (var hash = SHA256.Create()) return SessionAuthority.Hex(hash.ComputeHash(Encoding.UTF8.GetBytes(value)));
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
            Culture = System.Globalization.CultureInfo.InvariantCulture
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
            if (token.Type == JTokenType.Float)
            {
                var number = token.Value<double>();
                if (double.IsInfinity(number) || double.IsNaN(number)) throw new JsonException("Non-finite numeric value.");
            }
            if (token is JContainer container) foreach (var child in container.Children()) CheckFinite(child);
        }
        public static T Clone<T>(T state) => Decode<T>(Encode(state));
    }

    /// <summary>Writes a private temporary file, flushes it, then atomically installs it.</summary>
    public static class AtomicFile
    {
        public static void Write(string path, string contents, bool requireNew = false)
        {
            path = Path.GetFullPath(path);
            var parent = Path.GetDirectoryName(path);
            Directory.CreateDirectory(parent);
            var temporary = path + ".tmp-" + SessionAuthority.RandomHex(12);
            try
            {
                using (var stream = new FileStream(temporary, FileMode.CreateNew, FileAccess.Write, FileShare.None))
                {
                    Protect(temporary);
                    var bytes = Encoding.UTF8.GetBytes(contents);
                    stream.Write(bytes, 0, bytes.Length);
                    stream.Flush(true);
                }
                if (requireNew || !File.Exists(path)) File.Move(temporary, path);
                else File.Replace(temporary, path, path + ".previous");
                FlushDirectory(parent);
            }
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
