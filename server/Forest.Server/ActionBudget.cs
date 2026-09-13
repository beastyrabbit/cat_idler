namespace IdleCatForest.Server;

public sealed class ActionBudget
{
    private readonly Dictionary<string, Queue<long>> windows = new();
    private long nextCleanup;
    public bool Allow(string ip, string socket, string player, bool movement, long now)
    {
        lock (windows)
        {
            if (now >= nextCleanup) { foreach (var entry in windows.Where(entry => entry.Value.Count == 0 || now - entry.Value.Last() >= 10000).ToArray()) windows.Remove(entry.Key); nextCleanup = now + 10000; }
            string bucket = movement ? "move:" : "action:";
            var keys = new[] { (bucket + "ip:" + ip, movement ? 800 : 120), (bucket + "socket:" + socket, movement ? 120 : 30), (bucket + "player:" + (player.Length > 0 ? player : socket), movement ? 120 : 30) };
            foreach (var (key, maximum) in keys) { if (!windows.TryGetValue(key, out var q)) windows[key] = q = new Queue<long>(); while (q.Count > 0 && now - q.Peek() >= 10000) q.Dequeue(); if (q.Count >= maximum) return false; }
            foreach (var (key, _) in keys) windows[key].Enqueue(now); return true;
        }
    }
}
