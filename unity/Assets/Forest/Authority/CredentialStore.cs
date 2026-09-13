using System;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using Newtonsoft.Json;

namespace IdleCatForest.Authority
{
    public static class CredentialStore
    {
        public static SessionCredential Load(string path)
        {
            if (!File.Exists(path)) return null;
            var stored = WireJson.Decode<StoredCredential>(File.ReadAllText(path));
            if (stored == null || string.IsNullOrWhiteSpace(stored.SessionId) || string.IsNullOrWhiteSpace(stored.Sig))
                throw new InvalidDataException("Stored identity is invalid. It has not been replaced.");
            return new SessionCredential { SessionId = stored.SessionId, Sig = stored.Sig, PlayerId = SessionAuthority.PlayerId(stored.SessionId) };
        }
        public static void Save(string path, SessionCredential credential, string selectedVillageId = "")
        {
            var stored = File.Exists(path) ? WireJson.Decode<StoredCredential>(File.ReadAllText(path)) : new StoredCredential();
            stored.SessionId = credential.SessionId; stored.Sig = credential.Sig; stored.SelectedColonyId = selectedVillageId;
            AtomicFile.Write(path, WireJson.Encode(stored));
        }
        /// <summary>Imports the maintained Rust native bearer without printing or changing its source.</summary>
        public static void ImportLegacy(string source, string destination)
        {
            var credential = Load(source);
            if (credential == null) throw new FileNotFoundException("The selected identity file does not exist.");
            var raw = File.ReadAllText(source);
            AtomicFile.Write(destination, raw, true);
        }
        internal static byte[] Key(string directory, string injectedSecret = null)
        {
            if (!string.IsNullOrEmpty(injectedSecret)) return Encoding.UTF8.GetBytes(injectedSecret);
            var path = Path.Combine(directory, "authority.key");
            if (File.Exists(path))
            {
                var existing = Convert.FromBase64String(File.ReadAllText(path));
                if (existing.Length < 24) throw new InvalidDataException("Stored authority key is invalid. It has not been replaced.");
                return existing;
            }
            var key = new byte[32];
            using (var random = RandomNumberGenerator.Create()) random.GetBytes(key);
            AtomicFile.Write(path, Convert.ToBase64String(key), true);
            return key;
        }
        [Serializable]
        private sealed class StoredCredential
        {
            [JsonProperty("sessionId")] public string SessionId = "";
            [JsonProperty("sig")] public string Sig = "";
            [JsonProperty("selectedColonyId")] public string SelectedColonyId = "";
            [JsonProperty("nickname")] public string Nickname = "";
            [JsonProperty("textScale")] public double TextScale = 1;
        }
    }
}
