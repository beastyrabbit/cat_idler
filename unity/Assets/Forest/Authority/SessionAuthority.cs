using System;
using System.Globalization;
using System.Security.Cryptography;
using System.Text;

namespace IdleCatForest.Authority
{
    [Serializable]
    public sealed class SessionCredential
    {
        public string SessionId = "";
        public string Sig = "";
        public string PlayerId = "";
    }

    public sealed class SessionAuthority
    {
        public const long MaximumAgeMilliseconds = 30L * 24 * 60 * 60 * 1000;
        public const long RenewalGraceMilliseconds = 7L * 24 * 60 * 60 * 1000;
        private readonly byte[] secret;
        public SessionAuthority(byte[] secret)
        {
            if (secret == null || secret.Length == 0) throw new ArgumentException("An authority key is required.");
            this.secret = (byte[])secret.Clone();
        }
        public SessionCredential Issue(long now) => IssueForToken(RandomHex(8), now);
        public bool Verify(SessionCredential credential, long now)
        {
            return SignatureValid(credential) && IssuedAt(credential.SessionId, out var issued)
                && now >= issued && now - issued <= MaximumAgeMilliseconds;
        }
        public SessionCredential Renew(SessionCredential credential, long now)
        {
            if (!SignatureValid(credential)) return null;
            if (IssuedAt(credential.SessionId, out var issued))
            {
                if (now < issued || now - issued > MaximumAgeMilliseconds + RenewalGraceMilliseconds) return null;
            }
            else if (!IsLegacy(credential.SessionId)) return null;
            return IssueForToken(PlayerId(credential.SessionId).Substring("player_".Length), now);
        }
        public SessionCredential Restore(string sessionId, string signature)
        {
            return new SessionCredential { SessionId = sessionId ?? "", Sig = signature ?? "", PlayerId = PlayerId(sessionId ?? "") };
        }
        private SessionCredential IssueForToken(string token, long now)
        {
            var id = "session_v2_" + now.ToString(CultureInfo.InvariantCulture) + "_" + token + "_" + RandomHex(16);
            return new SessionCredential { SessionId = id, Sig = Sign(id, secret), PlayerId = PlayerId(id) };
        }
        private bool SignatureValid(SessionCredential credential)
        {
            if (credential == null || credential.SessionId == null || credential.SessionId.Length > 256 || credential.Sig == null || credential.Sig.Length != 64) return false;
            var actual = credential.Sig.ToLowerInvariant();
            var expected = Sign(credential.SessionId, secret);
            var difference = 0;
            for (var index = 0; index < expected.Length; index++) difference |= expected[index] ^ actual[index];
            return difference == 0;
        }
        public static string PlayerId(string id)
        {
            var parts = id.Split('_');
            if (parts.Length == 5 && parts[0] == "session" && parts[1] == "v2" && IsHex(parts[3], 16) && parts[4].Length > 0)
                return "player_" + parts[3];
            return "player_" + Sign(id, Encoding.UTF8.GetBytes("cat-server-player-id")).Substring(0, 16);
        }
        private static bool IssuedAt(string id, out long timestamp)
        {
            timestamp = 0;
            var parts = id.Split('_');
            return parts.Length >= 4 && parts[0] == "session" && (parts[1] == "v1" || parts[1] == "v2")
                && long.TryParse(parts[2], NumberStyles.None, CultureInfo.InvariantCulture, out timestamp) && timestamp >= 0;
        }
        private static bool IsLegacy(string id) => id.StartsWith("session_", StringComparison.Ordinal) && IsHex(id.Substring(8), 32);
        private static bool IsHex(string value, int length)
        {
            if (value.Length != length) return false;
            foreach (var character in value) if (!((character >= '0' && character <= '9') || (character >= 'a' && character <= 'f') || (character >= 'A' && character <= 'F'))) return false;
            return true;
        }
        private static string Sign(string input, byte[] key)
        {
            using (var hmac = new HMACSHA256(key)) return Hex(hmac.ComputeHash(Encoding.UTF8.GetBytes(input)));
        }
        internal static string RandomHex(int count)
        {
            var bytes = new byte[count];
            using (var random = RandomNumberGenerator.Create()) random.GetBytes(bytes);
            return Hex(bytes);
        }
        internal static string Hex(byte[] bytes)
        {
            var result = new StringBuilder(bytes.Length * 2);
            foreach (var value in bytes) result.Append(value.ToString("x2", CultureInfo.InvariantCulture));
            return result.ToString();
        }
    }
}
