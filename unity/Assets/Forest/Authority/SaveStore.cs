using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Security.Cryptography;
using System.Runtime.InteropServices;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace IdleCatForest.Authority
{
    /// <summary>Stable codes for a save that cannot be opened or created. The UI maps them to player text.</summary>
    public static class SaveFailure
    {
        /// <summary>Not a save, invalid JSON or a checksum mismatch.</summary>
        public const string Damaged = "damaged";
        /// <summary>Written by a newer build; this build must not guess at it.</summary>
        public const string MadeByNewerVersion = "made_by_newer_version";
        /// <summary>Older than the oldest version a migration can read.</summary>
        public const string UnsupportedVersion = "unsupported_version";
        public const string TooLarge = "too_large";
        /// <summary>Another authority (another window or process) has this save open.</summary>
        public const string AlreadyRunning = "already_running";
        /// <summary>A new world was requested where a world already exists.</summary>
        public const string AlreadyExists = "already_exists";
        /// <summary>The main save is gone but a backup of it exists, so nothing new was created.</summary>
        public const string MissingSaveWithBackup = "missing_save_with_backup";
        /// <summary>Village ownership is structurally broken (communal village, owners or identities).</summary>
        public const string InvalidOwnership = "invalid_ownership";
    }

    /// <summary>A save could not be opened or created. The original file is never changed.</summary>
    public sealed class SaveException : IOException
    {
        public string Code { get; }
        public SaveException(string code, string message, Exception inner = null) : base(message, inner) { Code = code; }
    }

    /// <summary>What loading a save had to tolerate.</summary>
    public sealed class SaveLoadReport
    {
        public int FromVersion { get; internal set; }
        /// <summary>Members present in the file but unknown to this build, as <c>Type.Member</c>; their values were skipped.</summary>
        public IReadOnlyList<string> DroppedMembers { get; internal set; } = Array.Empty<string>();
        internal string Payload = "";
    }

    /// <summary>A world encoded on the thread that owns it, ready to be written on any thread.</summary>
    public sealed class SaveSnapshot
    {
        public string Payload { get; }
        /// <summary>Simulation invariant failures found when the snapshot was taken; the world is saved anyway.</summary>
        public IReadOnlyList<string> Failures { get; }
        public double TimeSeconds { get; }
        public double CaptureMilliseconds { get; }
        /// <summary>Broken ownership structure; such a world could not be reopened, so it is never written over the save.</summary>
        public string StructureError { get; }
        public SaveSnapshot(string payload, IReadOnlyList<string> failures, double timeSeconds, double captureMilliseconds, string structureError = "")
        {
            Payload = payload ?? throw new ArgumentNullException(nameof(payload));
            Failures = failures ?? Array.Empty<string>();
            TimeSeconds = timeSeconds;
            CaptureMilliseconds = captureMilliseconds;
            StructureError = structureError ?? "";
        }
    }

    /// <summary>The outcome of one successful save write.</summary>
    public sealed class SaveResult
    {
        public long Bytes { get; internal set; }
        public double WriteMilliseconds { get; internal set; }
        /// <summary>Set when this write first copied the session start save to its backup.</summary>
        public string BackupPath { get; internal set; } = "";
        /// <summary>Set when this write also kept a diagnostic copy of a world that breaks invariants.</summary>
        public string InvalidCopyPath { get; internal set; } = "";
    }

    public static class SaveStore
    {
        /// <summary>Envelope version. A breaking payload change bumps it and adds a step in <see cref="SaveMigrations"/>.</summary>
        public const int Version = 1;
        public const string Format = "idle-cat-forest-unity";
        public const long MaximumBytes = 256L * 1024 * 1024;

        public static void Save<T>(string path, T state, bool requireNew = false) => Write(path, Encode(state), requireNew);

        /// <summary>Encodes the payload. Call on the thread that owns <paramref name="state"/>.</summary>
        public static string Encode<T>(T state)
        {
            if (state == null) throw new ArgumentNullException(nameof(state));
            return WireJson.Encode(state);
        }

        /// <summary>Writes an encoded payload in the versioned, checksummed envelope. Safe on a background thread.</summary>
        public static long Write(string path, string payload, bool requireNew = false)
        {
            if (payload == null) throw new ArgumentNullException(nameof(payload));
            var document = new SaveDocument { Payload = payload, Sha256 = Hash(payload) };
            return AtomicFile.Write(path, WireJson.Encode(document), requireNew);
        }

        public static T Load<T>(string path) => Load<T>(path, out _);

        public static T Load<T>(string path, out SaveLoadReport report)
        {
            var file = new FileInfo(path);
            if (file.Length > MaximumBytes) throw new SaveException(SaveFailure.TooLarge, "Save exceeds the supported size. The original was left untouched.");
            var envelope = ReadEnvelope(File.ReadAllText(path), out var version);
            return DecodePayload<T>(envelope, version, out report);
        }

        /// <summary>Migrates and decodes a payload written at <paramref name="fromVersion"/>. Unknown members are skipped and reported.</summary>
        public static T DecodePayload<T>(string payload, int fromVersion, out SaveLoadReport report)
        {
            try
            {
                if (!(WireJson.Parse(payload) is JObject root)) throw new SaveException(SaveFailure.Damaged, "Save payload is not an object. The original was left untouched.");
                var dropped = new List<string>();
                var value = SaveMigrations.Migrate(root, fromVersion).ToObject<T>(JsonSerializer.Create(SaveJson.Settings(dropped)));
                report = new SaveLoadReport { FromVersion = fromVersion, DroppedMembers = dropped, Payload = payload };
                return value;
            }
            catch (JsonException error) { throw new SaveException(SaveFailure.Damaged, "Save JSON is invalid. The original was left untouched.", error); }
        }

        private static string ReadEnvelope(string text, out int version)
        {
            JObject envelope;
            try { envelope = WireJson.Parse(text) as JObject; }
            catch (JsonException error) { throw new SaveException(SaveFailure.Damaged, "Save JSON is invalid. The original was left untouched.", error); }
            if (envelope == null || envelope.Value<string>("Format") != Format || envelope["Version"]?.Type != JTokenType.Integer)
                throw new SaveException(SaveFailure.Damaged, "Unsupported save format. The original was left untouched.");
            var declared = envelope.Value<long>("Version");
            if (declared > Version) throw new SaveException(SaveFailure.MadeByNewerVersion, "This world was saved by a newer version of the game. Update the game to open it. The original was left untouched.");
            if (declared < SaveMigrations.OldestSupported) throw new SaveException(SaveFailure.UnsupportedVersion, "Unsupported save version. The original was left untouched.");
            version = (int)declared;
            var payload = envelope["Payload"]?.Type == JTokenType.String ? envelope.Value<string>("Payload") : null;
            if (payload == null || !string.Equals(envelope["Sha256"]?.Type == JTokenType.String ? envelope.Value<string>("Sha256") : null, Hash(payload), StringComparison.Ordinal))
                throw new SaveException(SaveFailure.Damaged, "Save checksum does not match. The original was left untouched.");
            return payload;
        }

        private static string Hash(string value)
        {
            using (var hash = SHA256.Create()) return SessionAuthority.Hex(hash.ComputeHash(Encoding.UTF8.GetBytes(value)));
        }

        [Serializable]
        private sealed class SaveDocument
        {
            public string Format = SaveStore.Format;
            public int Version = SaveStore.Version;
            public string Payload = "", Sha256 = "";
        }
    }

    /// <summary>
    /// Save payloads tolerate members this build does not know (a removed or future field), which wire input never does.
    /// Each skipped member is recorded so the authority can log it.
    /// </summary>
    internal static class SaveJson
    {
        private const string MissingMember = "Could not find member";

        internal static JsonSerializerSettings Settings(List<string> dropped)
        {
            var settings = WireJson.CreateSettings();
            settings.Error = (sender, args) =>
            {
                var context = args.ErrorContext;
                if (!(context.Error is JsonSerializationException) || !(context.Member is string member) || !context.Error.Message.StartsWith(MissingMember, StringComparison.Ordinal)) return;
                var name = (args.CurrentObject == null ? "?" : args.CurrentObject.GetType().Name) + "." + member;
                if (!dropped.Contains(name)) dropped.Add(name);
                context.Handled = true;
            };
            return settings;
        }
    }

    public static class WireJson
    {
        private static readonly JsonSerializerSettings Settings = CreateSettings();
        internal static JsonSerializerSettings CreateSettings() => new JsonSerializerSettings
        {
            TypeNameHandling = TypeNameHandling.None,
            MetadataPropertyHandling = MetadataPropertyHandling.Ignore,
            MissingMemberHandling = MissingMemberHandling.Error,
            ReferenceLoopHandling = ReferenceLoopHandling.Error,
            MaxDepth = 128,
            Culture = System.Globalization.CultureInfo.InvariantCulture
        };
        public static string Encode(object value) => JsonConvert.SerializeObject(value, Settings);
        public static T Decode<T>(string value) => Parse(value).ToObject<T>(JsonSerializer.Create(Settings));

        /// <summary>Parses one JSON root, rejecting duplicate properties and non-finite numbers.</summary>
        internal static JToken Parse(string value)
        {
            using (var reader = new JsonTextReader(new StringReader(value)) { MaxDepth = 128, DateParseHandling = DateParseHandling.None })
            {
                var token = JToken.Load(reader, new JsonLoadSettings { DuplicatePropertyNameHandling = DuplicatePropertyNameHandling.Error });
                CheckFinite(token);
                if (reader.Read()) throw new JsonException("Multiple JSON roots are not supported.");
                return token;
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
        /// <summary>Marks a temporary file; <see cref="Backups.RemoveStaleTemporaries"/> recognises it after a crash.</summary>
        internal const string TemporaryMarker = ".tmp-";

        public static long Write(string path, string contents, bool requireNew = false, bool keepPrevious = true)
            => Write(path, Encoding.UTF8.GetBytes(contents), requireNew, keepPrevious);

        /// <summary>
        /// Installs <paramref name="bytes"/> at <paramref name="path"/>. An existing file moves to <c>&lt;path&gt;.previous</c>
        /// unless <paramref name="keepPrevious"/> is false. Returns the number of bytes written.
        /// </summary>
        public static long Write(string path, byte[] bytes, bool requireNew = false, bool keepPrevious = true)
        {
            path = Path.GetFullPath(path);
            var parent = Path.GetDirectoryName(path);
            Directory.CreateDirectory(parent);
            var temporary = path + TemporaryMarker + SessionAuthority.RandomHex(12);
            try
            {
                using (var stream = new FileStream(temporary, FileMode.CreateNew, FileAccess.Write, FileShare.None))
                {
                    Protect(temporary);
                    stream.Write(bytes, 0, bytes.Length);
                    stream.Flush(true);
                }
                if (requireNew || !File.Exists(path)) File.Move(temporary, path);
                else File.Replace(temporary, path, keepPrevious ? path + ".previous" : null);
                FlushDirectory(parent);
                return bytes.Length;
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
