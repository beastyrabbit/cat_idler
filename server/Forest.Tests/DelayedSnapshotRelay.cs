using System.Net.WebSockets;
using System.Text;
using IdleCatForest.Authority;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Hosting.Server;
using Microsoft.AspNetCore.Hosting.Server.Features;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

// Reorders real host projections deterministically, without a production transport hook.
sealed class DelayedSnapshotRelay : IAsyncDisposable
{
    private readonly object sync = new();
    private WebApplication app;
    private TaskCompletionSource<bool> capture;
    private ServerMessage delayed;
    private bool suppressBroadcasts;
    public string Address { get; private set; }

    public static async Task<DelayedSnapshotRelay> StartAsync(string upstream)
    {
        var relay = new DelayedSnapshotRelay();
        var builder = WebApplication.CreateBuilder();
        builder.WebHost.UseUrls("http://127.0.0.1:0");
        builder.Logging.ClearProviders();
        relay.app = builder.Build();
        relay.app.UseWebSockets();
        relay.app.Map("/ws", context => relay.Pump(context, upstream));
        await relay.app.StartAsync();
        relay.Address = relay.app.Services.GetRequiredService<IServer>().Features.Get<IServerAddressesFeature>().Addresses.Single().Replace("http://", "ws://") + "/ws";
        return relay;
    }

    public Task CaptureNextBroadcast()
    {
        lock (sync)
        {
            if (capture != null) throw new InvalidOperationException("An earlier broadcast is still pending.");
            capture = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
            suppressBroadcasts = false;
            return capture.Task.WaitAsync(TimeSpan.FromSeconds(5));
        }
    }

    private async Task Pump(HttpContext context, string upstreamAddress)
    {
        using var downstream = await context.WebSockets.AcceptWebSocketAsync();
        using var upstream = new ClientWebSocket();
        using var lifetime = CancellationTokenSource.CreateLinkedTokenSource(context.RequestAborted);
        await upstream.ConnectAsync(new Uri(upstreamAddress), lifetime.Token);
        var input = Task.Run(async () =>
        {
            while (await Read(downstream, lifetime.Token) is string request)
                await Write(upstream, request, lifetime.Token);
        });
        var output = Task.Run(async () =>
        {
            while (await Read(upstream, lifetime.Token) is string json)
            {
                var message = WireJson.Decode<ServerMessage>(json);
                ServerMessage stale = null;
                lock (sync)
                {
                    if (message.Type == "snapshot" && message.Frame != null)
                    {
                        if (capture != null && delayed == null)
                        {
                            delayed = message;
                            suppressBroadcasts = true;
                            capture.TrySetResult(true);
                        }
                        if (suppressBroadcasts) continue;
                    }
                    if (message.Type == "result" && message.Frame != null && delayed != null)
                    {
                        stale = delayed;
                        delayed = null;
                        capture = null;
                    }
                }
                if (stale != null)
                {
                    // Complete the request only after both new and old projections arrived.
                    await Write(downstream, WireJson.Encode(new ServerMessage { Frame = message.Frame }), lifetime.Token);
                    await Write(downstream, WireJson.Encode(stale), lifetime.Token);
                    message.Frame = stale.Frame;
                    json = WireJson.Encode(message);
                }
                await Write(downstream, json, lifetime.Token);
            }
        });
        await Task.WhenAny(input, output);
        lifetime.Cancel();
        upstream.Abort();
        downstream.Abort();
        try { await Task.WhenAll(input, output); }
        catch (Exception error) when (error is OperationCanceledException || error is WebSocketException) { }
    }

    private static async Task<string> Read(WebSocket socket, CancellationToken cancellation)
    {
        using var stream = new MemoryStream();
        var buffer = new byte[16384];
        WebSocketReceiveResult received;
        do
        {
            received = await socket.ReceiveAsync(new ArraySegment<byte>(buffer), cancellation);
            if (received.MessageType == WebSocketMessageType.Close) return null;
            if (received.MessageType != WebSocketMessageType.Text || stream.Length + received.Count > 32 * 1024 * 1024)
                throw new IOException("Unsupported test relay message.");
            stream.Write(buffer, 0, received.Count);
        } while (!received.EndOfMessage);
        return Encoding.UTF8.GetString(stream.ToArray());
    }

    private static Task Write(WebSocket socket, string message, CancellationToken cancellation) =>
        socket.SendAsync(new ArraySegment<byte>(Encoding.UTF8.GetBytes(message)), WebSocketMessageType.Text, true, cancellation);

    public async ValueTask DisposeAsync()
    {
        await app.StopAsync();
        await app.DisposeAsync();
    }
}
