using System;
using System.IO;
using System.Threading.Tasks;
using IdleCatForest.Authority;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>Connecting to and leaving a shared-world server.</summary>
    public sealed partial class ForestGame
    {
        private bool connecting;

        public async Task Connect(string address)
        {
            if (connecting) return;
            connecting = true;
            WorldClient candidate = null;
            try
            {
                var uri = new Uri(address);
                if (uri.Scheme != "ws" && uri.Scheme != "wss") throw new ArgumentException("Use a ws:// or wss:// server address.");
                Status = "Connecting to shared world…";
                var credentialPath = Path.Combine(Application.persistentDataPath, "unity", "servers", SafeServerKey(uri) + ".json");
                candidate = new WorldClient(uri.ToString(), credentialPath);
                await candidate.ConnectAsync(cancellation.Token);
                Save();
                remote?.Dispose(); remote = candidate; candidate = null;
                Revision++; View.FocusVillage();
            }
            catch (Exception e) { LastAction = "Connection failed: " + e.Message; Status = "Local world retained"; candidate?.Dispose(); }
            finally { connecting = false; }
        }

        public void Disconnect()
        {
            remote?.Dispose(); remote = null; Status = "Local shared world · saved on this Mac"; Revision++; View.FocusVillage();
        }

        private static string SafeServerKey(Uri uri)
        {
            using var hash = System.Security.Cryptography.SHA256.Create();
            return BitConverter.ToString(hash.ComputeHash(System.Text.Encoding.UTF8.GetBytes(uri.GetLeftPart(UriPartial.Authority)))).Replace("-", "").ToLowerInvariant();
        }
    }
}
