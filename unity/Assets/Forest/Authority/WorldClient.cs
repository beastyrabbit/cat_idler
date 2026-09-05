using System;
using System.Collections.Generic;
using System.IO;
using System.Net;
using System.Net.WebSockets;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using IdleCatForest.Simulation;

namespace IdleCatForest.Authority
{
    /// <summary>Receives authorized snapshots and sends input to the shared simulation host.</summary>
    public sealed class WorldClient : IDisposable
    {
        private readonly Uri address;
        private readonly string credentialPath;
        private readonly ClientWebSocket socket = new ClientWebSocket();
        private readonly CancellationTokenSource lifetime = new CancellationTokenSource();
        private readonly SemaphoreSlim sender = new SemaphoreSlim(1, 1);
        private readonly Dictionary<string, TaskCompletionSource<ServerMessage>> pending = new Dictionary<string, TaskCompletionSource<ServerMessage>>();
        private volatile WorldFrame latest;
        private int disposed;
        private string savedSelection = "";
        public World LatestWorld => latest == null ? null : latest.World;
        public string SelectedVillageId => latest == null ? "" : latest.SelectedVillageId;
        public WorldFrame LatestFrame => latest;
        public SessionCredential Credential { get; private set; }
        public string Status { get; private set; } = "Disconnected";

        public WorldClient(string address, string credentialPath)
        {
            this.address = new Uri(address);
            var loopback = this.address.Host.Equals("localhost", StringComparison.OrdinalIgnoreCase)
                || IPAddress.TryParse(this.address.Host, out var ip) && IPAddress.IsLoopback(ip);
            if (this.address.Scheme != "wss" && !(this.address.Scheme == "ws" && loopback))
                throw new ArgumentException("Remote shared worlds require a secure wss address.");
            this.credentialPath = Path.GetFullPath(credentialPath);
        }

        public async Task ConnectAsync(CancellationToken cancellationToken = default)
        {
            Status = "Connecting";
            Credential = CredentialStore.Load(credentialPath);
            await socket.ConnectAsync(address, cancellationToken).ConfigureAwait(false);
            _ = ReceiveLoop();
            var response = await Request(new ClientRequest { Type = "presence", SessionId = Credential == null ? "" : Credential.SessionId, Sig = Credential == null ? "" : Credential.Sig }, cancellationToken).ConfigureAwait(false);
            if (response.Credential == null) throw new UnauthorizedAccessException(response.Error.Length == 0 ? "The shared world refused this identity." : response.Error);
            Credential = response.Credential;
            CredentialStore.Save(credentialPath, Credential, SelectedVillageId);
            savedSelection = SelectedVillageId;
            Status = "Connected";
        }

        public async Task<ActionResult> SendAsync(GameAction action)
        {
            if (Credential == null || socket.State != WebSocketState.Open) return ActionResult.Fail("Connect to a shared world first.");
            var response = await Request(new ClientRequest { Action = action, SessionId = Credential.SessionId, Sig = Credential.Sig }, lifetime.Token).ConfigureAwait(false);
            if (response.Frame != null && savedSelection != SelectedVillageId) { CredentialStore.Save(credentialPath, Credential, SelectedVillageId); savedSelection = SelectedVillageId; }
            return response.Result ?? ActionResult.Fail(response.Error.Length == 0 ? "The action had no result." : response.Error);
        }

        private async Task<ServerMessage> Request(ClientRequest request, CancellationToken cancellationToken)
        {
            request.RequestId = Guid.NewGuid().ToString("N");
            var completion = new TaskCompletionSource<ServerMessage>(TaskCreationOptions.RunContinuationsAsynchronously);
            lock (pending) pending.Add(request.RequestId, completion);
            using (var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken, lifetime.Token))
            {
                timeout.CancelAfter(TimeSpan.FromSeconds(20));
                using (timeout.Token.Register(() => completion.TrySetCanceled()))
                {
                    try
                    {
                        var bytes = Encoding.UTF8.GetBytes(WireJson.Encode(request));
                        if (bytes.Length > 64 * 1024) throw new InvalidDataException("Action is too large.");
                        await sender.WaitAsync(timeout.Token).ConfigureAwait(false);
                        try { await socket.SendAsync(new ArraySegment<byte>(bytes), WebSocketMessageType.Text, true, timeout.Token).ConfigureAwait(false); }
                        finally { sender.Release(); }
                        return await completion.Task.ConfigureAwait(false);
                    }
                    finally { lock (pending) pending.Remove(request.RequestId); }
                }
            }
        }

        private async Task ReceiveLoop()
        {
            var buffer = new byte[16384];
            try
            {
                while (!lifetime.IsCancellationRequested && socket.State == WebSocketState.Open)
                {
                    using (var message = new MemoryStream())
                    {
                        WebSocketReceiveResult received;
                        do
                        {
                            received = await socket.ReceiveAsync(new ArraySegment<byte>(buffer), lifetime.Token).ConfigureAwait(false);
                            if (received.MessageType == WebSocketMessageType.Close) { Status = "Disconnected"; return; }
                            if (received.MessageType != WebSocketMessageType.Text || message.Length + received.Count > 32 * 1024 * 1024)
                                throw new InvalidDataException("Unsupported shared-world message.");
                            message.Write(buffer, 0, received.Count);
                        } while (!received.EndOfMessage);
                        var decoded = WireJson.Decode<ServerMessage>(Encoding.UTF8.GetString(message.ToArray()));
                        if (decoded == null || decoded.ProtocolVersion != 3) throw new InvalidDataException("Shared-world protocol differs. Update the game.");
                        if (decoded.Frame != null)
                        {
                            if (decoded.Frame.Sequence <= 0) throw new InvalidDataException("Shared-world snapshot has no valid sequence.");
                            if (latest == null || decoded.Frame.Sequence > latest.Sequence) latest = decoded.Frame;
                        }
                        if (!string.IsNullOrEmpty(decoded.RequestId))
                            lock (pending) if (pending.TryGetValue(decoded.RequestId, out var completion)) completion.TrySetResult(decoded);
                    }
                }
            }
            catch (OperationCanceledException) { Status = "Disconnected"; }
            catch (Exception) { Status = "Shared-world connection failed. Existing identity was preserved."; }
            finally
            {
                lock (pending) foreach (var completion in pending.Values) completion.TrySetException(new IOException("Shared-world connection closed."));
            }
        }
        public void Dispose()
        {
            if (Interlocked.Exchange(ref disposed, 1) != 0) return;
            lifetime.Cancel(); socket.Abort(); socket.Dispose();
        }
    }
}
