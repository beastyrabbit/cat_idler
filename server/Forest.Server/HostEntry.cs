using System.Collections.Concurrent;
using System.Diagnostics;
using System.Net;
using System.Net.WebSockets;
using System.Text;
using IdleCatForest.Authority;
using IdleCatForest.Simulation;
using Newtonsoft.Json;

namespace IdleCatForest.Server;

public static class HostEntry
{
    public static async Task<int> RunAsync(string[] args)
    {
        var address = Environment.GetEnvironmentVariable("FOREST_LISTEN") ?? "http://127.0.0.1:8788";
        var path = Environment.GetEnvironmentVariable("FOREST_SAVE_PATH") ?? Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "IdleCatForest", "shared", "world-v1.json");
        var secret = Environment.GetEnvironmentVariable("SESSION_HMAC_SECRET");
        var uri = new Uri(address);
        var loopback = uri.Host == "localhost" || IPAddress.TryParse(uri.Host, out var ip) && IPAddress.IsLoopback(ip);
        if (!loopback && string.IsNullOrEmpty(secret))
        {
            Console.Error.WriteLine("SESSION_HMAC_SECRET is required for a public listener."); return 2;
        }
        try
        {
            using var runtime = new AuthorityRuntime(path, 4242, secret);
            var app = Build(runtime, address, true, args);
            await app.RunAsync();
            runtime.Save();
            return 0;
        }
        catch (Exception error) when (error is IOException || error is UnauthorizedAccessException || error is ArgumentException || error is InvalidDataException)
        {
            Console.Error.WriteLine($"Shared world could not start ({error.GetType().Name}). Existing data was preserved."); return 2;
        }
    }

    public static WebApplication Build(AuthorityRuntime runtime, string address, bool automaticTicks, string[] args = null)
    {
        var builder = WebApplication.CreateBuilder(args ?? Array.Empty<string>());
        builder.WebHost.UseUrls(address);
        // HTTP request logging never includes bodies, session credentials, or snapshots.
        builder.Logging.ClearProviders();
        var app = builder.Build();
        var peers = new ConcurrentDictionary<string, int>();
        var actionBudget = new ActionBudget();
        var identityLimits = new SlidingLimits(8, TimeSpan.FromHours(1));
        var clients = new ConcurrentDictionary<Guid, SocketPeer>();
        var failures = 0;
        app.UseWebSockets(new WebSocketOptions { KeepAliveInterval = TimeSpan.FromSeconds(20) });
        app.MapGet("/health", () => Results.Text("ok"));
        app.MapGet("/ready", () => Volatile.Read(ref failures) < 3 ? Results.Ok() : Results.StatusCode(503));
        app.Map("/ws", async context =>
        {
            if (!context.WebSockets.IsWebSocketRequest) { context.Response.StatusCode = 400; return; }
            var origin = context.Request.Headers.Origin.ToString();
            var permittedOrigins = (Environment.GetEnvironmentVariable("FOREST_ALLOWED_ORIGINS") ?? "").Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
            if (origin.Length > 0 && !permittedOrigins.Contains(origin, StringComparer.Ordinal)) { context.Response.StatusCode = 403; return; }
            var peerIp = context.Connection.RemoteIpAddress?.ToString() ?? "unknown";
            if (peers.AddOrUpdate(peerIp, 1, (_, count) => count + 1) > 8) { peers.AddOrUpdate(peerIp, 0, (_, count) => Math.Max(0, count - 1)); context.Response.StatusCode = 429; return; }
            try
            {
                using var socket = await context.WebSockets.AcceptWebSocketAsync();
                var peer = new SocketPeer(socket, peerIp);
                clients.TryAdd(peer.Id, peer);
                try
                {
                    await peer.Send(new ServerMessage { Frame = runtime.Project(null) }, context.RequestAborted);
                    while (socket.State == WebSocketState.Open && !context.RequestAborted.IsCancellationRequested)
                    {
                        var text = await Read(socket, context.RequestAborted);
                        if (text == null) break;
                        ClientRequest request;
                        try { request = WireJson.Decode<ClientRequest>(text); }
                        catch (JsonException) { await peer.Send(new ServerMessage { Type = "error", Error = "Invalid action JSON." }, context.RequestAborted); continue; }
                        if (request == null || request.ProtocolVersion != 3 || request.RequestId == null || request.RequestId.Length > 128)
                        { await peer.Send(new ServerMessage { Type = "error", Error = "Unsupported action protocol." }, context.RequestAborted); continue; }
                        var now = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
                        var response = new ServerMessage { Type = "result", RequestId = request.RequestId };
                        var authenticated = runtime.Authenticate(peer.Identity, request.SessionId, request.Sig, now);
                        var movement = authenticated && request.Action?.Kind?.Replace("_", "").Equals("movecat", StringComparison.OrdinalIgnoreCase) == true;
                        if (!actionBudget.Allow(peerIp, peer.Id.ToString(), peer.Identity?.Credential.PlayerId ?? "", movement, now)) response.Result = ActionResult.Fail("Too many actions. Wait before retrying.");
                        else if (request.Type == "presence")
                        {
                            if (peer.Identity != null) response.Result = ActionResult.Fail("This connection already has an identity.");
                            else if (string.IsNullOrEmpty(request.SessionId) && !identityLimits.Allow(peerIp, now)) response.Result = ActionResult.Fail("Too many new identities from this address.");
                            else
                            {
                                try
                                {
                                    lock (runtime.Sync) peer.Identity = runtime.Connect(string.IsNullOrEmpty(request.SessionId) ? null : new SessionCredential { SessionId = request.SessionId, Sig = request.Sig }, now);
                                    response.Credential = peer.Identity.Credential;
                                    response.Result = ActionResult.Ok();
                                }
                                catch (UnauthorizedAccessException) { response.Error = "Stored identity was not accepted. Its file was preserved."; response.Result = ActionResult.Fail(response.Error); }
                            }
                        }
                        else if (!authenticated) response.Result = ActionResult.Fail("The action must use this connection's valid signed identity.");
                        else if (request.Action?.Kind == "FoundVillage" && runtime.World.Villages.Count >= 256) response.Result = ActionResult.Fail("The shared world has reached its village limit.");
                        else response.Result = runtime.Apply(peer.Identity, request.Action);
                        lock (runtime.Sync) response.Frame = runtime.Project(peer.Identity);
                        await peer.Send(response, context.RequestAborted);
                    }
                }
                catch (Exception error) when (error is WebSocketException || error is OperationCanceledException || error is IOException) { }
                finally { clients.TryRemove(peer.Id, out _); }
            }
            finally { peers.AddOrUpdate(peerIp, 0, (_, count) => Math.Max(0, count - 1)); }
        });
        if (automaticTicks)
            app.Lifetime.ApplicationStarted.Register(() => _ = Task.Run(async () =>
            {
                var clock = Stopwatch.StartNew();
                var last = clock.Elapsed.TotalSeconds;
                var saveAt = last + 5;
                var snapshotAt = last + 1;
                Task broadcast = Task.CompletedTask;
                using var timer = new PeriodicTimer(TimeSpan.FromMilliseconds(100));
                try
                {
                    while (await timer.WaitForNextTickAsync(app.Lifetime.ApplicationStopping))
                    {
                        var current = clock.Elapsed.TotalSeconds;
                        runtime.Advance(Math.Min(1, current - last)); last = current;
                        if (current >= snapshotAt && broadcast.IsCompleted)
                        {
                            bool controlling; lock (runtime.Sync) controlling = runtime.World.Villages.Any(v => v.Cats.Any(c => c.ControlledBy.Length > 0));
                            snapshotAt = current + (controlling ? 0.1 : 0.5);
                            broadcast = Task.Run(async () =>
                            {
                                await Task.WhenAll(clients.Values.Select(async peer =>
                                {
                                    try
                                    {
                                        WorldFrame frame;
                                        lock (runtime.Sync) frame = runtime.Project(peer.Identity);
                                        await peer.Send(new ServerMessage { Frame = frame }, app.Lifetime.ApplicationStopping);
                                    }
                                    catch (Exception error) when (error is WebSocketException || error is OperationCanceledException || error is IOException) { peer.Socket.Abort(); }
                                }));
                            });
                        }
                        if (current >= saveAt)
                        {
                            saveAt = current + 5;
                            try { runtime.Save(); Interlocked.Exchange(ref failures, 0); }
                            catch (IOException) { Interlocked.Increment(ref failures); }
                        }
                    }
                }
                catch (OperationCanceledException) { }
                finally { try { runtime.Save(); } catch (IOException) { Interlocked.Increment(ref failures); } }
            }));
        return app;
    }

    private static async Task<string> Read(WebSocket socket, CancellationToken cancellation)
    {
        var buffer = new byte[4096];
        using var message = new MemoryStream();
        WebSocketReceiveResult frame;
        do
        {
            frame = await socket.ReceiveAsync(new ArraySegment<byte>(buffer), cancellation);
            if (frame.MessageType == WebSocketMessageType.Close) return null;
            if (frame.MessageType != WebSocketMessageType.Text || message.Length + frame.Count > 64 * 1024) throw new IOException("Unsupported action frame.");
            message.Write(buffer, 0, frame.Count);
        } while (!frame.EndOfMessage);
        return new UTF8Encoding(false, true).GetString(message.ToArray());
    }

    private sealed class SocketPeer(WebSocket socket, string peerIp)
    {
        public readonly Guid Id = Guid.NewGuid();
        public readonly WebSocket Socket = socket;
        public readonly string PeerIp = peerIp;
        public ConnectionIdentity Identity;
        private readonly SemaphoreSlim sender = new(1, 1);
        public async Task Send(ServerMessage message, CancellationToken cancellation)
        {
            using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellation);
            timeout.CancelAfter(TimeSpan.FromSeconds(5));
            var bytes = Encoding.UTF8.GetBytes(WireJson.Encode(message));
            await sender.WaitAsync(timeout.Token);
            try { await Socket.SendAsync(new ArraySegment<byte>(bytes), WebSocketMessageType.Text, true, timeout.Token); }
            finally { sender.Release(); }
        }
    }
    private sealed class SlidingLimits(int maximum, TimeSpan period)
    {
        private readonly Dictionary<string, Queue<long>> windows = new();
        public bool Allow(string identity, long now)
        {
            lock (windows)
            {
                if (!windows.TryGetValue(identity, out var window)) windows[identity] = window = new Queue<long>();
                while (window.Count > 0 && now - window.Peek() >= period.TotalMilliseconds) window.Dequeue();
                if (window.Count >= maximum) return false;
                window.Enqueue(now); return true;
            }
        }
    }
}
