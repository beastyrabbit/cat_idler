using System;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using IdleCatForest.Authority;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>Connecting to and leaving a shared-world server.</summary>
    public sealed partial class ForestGame
    {
        /// <summary>How long a connection attempt may take before it fails with <see cref="FeedbackCode.Timeout"/>.</summary>
        public const int ConnectTimeoutMilliseconds = 10000;

        private bool connecting;

        public async Task Connect(string address)
        {
            if (connecting) return;
            connecting = true;
            WorldClient candidate = null;
            try
            {
                var uri = new Uri(address);
                if (uri.Scheme != "ws" && uri.Scheme != "wss") throw new UriFormatException("Use a ws:// or wss:// server address.");
                Status = "Connecting to shared world…";
                var credentialPath = Path.Combine(Application.persistentDataPath, "unity", "servers", SafeServerKey(uri) + ".json");
                candidate = new WorldClient(uri.ToString(), credentialPath);
                using (var attempt = CancellationTokenSource.CreateLinkedTokenSource(cancellation.Token))
                {
                    attempt.CancelAfter(ConnectTimeoutMilliseconds);
                    await candidate.ConnectAsync(attempt.Token);
                }
                // A faulted local world is never written; the last good save stays on disk.
                if (!Faulted) Save();
                remote?.Dispose(); remote = candidate; candidate = null;
                remoteFrame = null; remoteSequence = 0;
                Revision++; View.FocusVillage();
            }
            catch (Exception e)
            {
                ForestLog.Exception(e, "connecting to a shared world");
                Publish(new GameAction { Kind = "Connect" }, ActionResult.Fail(Feedback.CodeFor(e), ""));
                Status = "Local world retained";
                candidate?.Dispose();
            }
            finally { connecting = false; }
        }

        public void Disconnect()
        {
            remote?.Dispose(); remote = null; remoteFrame = null; remoteSequence = 0;
            Status = "Local shared world · saved on this Mac"; Revision++; View.FocusVillage();
        }

        private static string SafeServerKey(Uri uri)
        {
            using var hash = System.Security.Cryptography.SHA256.Create();
            return BitConverter.ToString(hash.ComputeHash(System.Text.Encoding.UTF8.GetBytes(uri.GetLeftPart(UriPartial.Authority)))).Replace("-", "").ToLowerInvariant();
        }
    }
}
