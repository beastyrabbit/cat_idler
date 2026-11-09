using IdleCatForest.Authority;
using System.Reflection;
using System.Security.Cryptography;
using System.Text;

static class SaveEncodingScenarios
{
    static readonly Func<string, SaveTiming, string> Checksum = typeof(SaveStore)
        .GetMethod("Hash", BindingFlags.NonPublic | BindingFlags.Static)
        .CreateDelegate<Func<string, SaveTiming, string>>();

    public static void Run(Action<string, Action> test, Action<bool, string> check)
    {
        test("save encoding smoke checksum allocation stays bounded for large payloads", () =>
        {
            var payload = new string('x', 2 * 1024 * 1024) + "猫😺";
            var expected = OriginalChecksum(payload);
            Checksum(payload, null);
            long before = GC.GetAllocatedBytesForCurrentThread();
            string actual = Checksum(payload, null);
            long allocated = GC.GetAllocatedBytesForCurrentThread() - before;
            check(actual == expected, "bounded checksum differs from the v1 UTF-8 digest");
            Console.WriteLine($"Checksum managed allocation: {allocated} bytes for {Encoding.UTF8.GetByteCount(payload)} payload bytes");
            check(allocated < 128 * 1024, $"checksum allocated {allocated} bytes; scratch storage must stay below 128 KiB independently of payload size");
        });

        test("save encoding checksum preserves UTF-8 replacement and surrogate boundaries", () =>
        {
            foreach (int length in new[] { 0, 1, 4094, 4095, 4096, 4097, 8190, 8191, 8192, 8193 })
                foreach (string suffix in new[] { "", "猫", "😺", "\ud83d", "\ude3a", "\ud83d\ud83d\ude3a", "\ud83dx\ude3a", "\ud83d\ude3a\ud83d\ude3a" })
                {
                    string payload = new string('x', length) + suffix + "\"\\\r\n\t\0\u2028\u2029";
                    check(Checksum(payload, null) == OriginalChecksum(payload), $"v1 UTF-8 checksum changed at prefix {length} and suffix length {suffix.Length}");
                }
            check(Checksum("", null) == OriginalChecksum(""), "empty payload checksum changed");
        });

        test("save encoding retains exact v1 envelope bytes timings and prior durable state", () =>
        {
            InDirectory(directory =>
            {
                string path = Path.Combine(directory, "world.json");
                var state = new EncodingFixture
                {
                    Label = new string('x', 4095) + "😺猫\"\\\r\n\t\0\u2028\u2029",
                    Cargo = 2.25,
                    Numbers = new[] { 0d, -0d, double.Epsilon, double.MaxValue, 1e-6 }
                };
                string payload = WireJson.Encode(state);
                byte[] expected = OriginalEnvelope(state);
                SaveStore.Save(path, state, out var timing, true);
                check(File.ReadAllBytes(path).SequenceEqual(expected), "current save differs from exact original v1 envelope bytes");
                check(timing.Success && timing.PayloadBytes == Encoding.UTF8.GetByteCount(payload), "bounded checksum lost successful timing or exact payload-byte count");
                check(timing.EnvelopeBytes == expected.Length, "envelope timing byte count differs from durable bytes");
                check(WireJson.Encode(SaveStore.Load<EncodingFixture>(path)) == payload, "exact v1 fixture did not strictly roundtrip");
                state.Label = "replacement 猫😺";
                state.Cargo = 9.5;
                SaveStore.Save(path, state);
                check(File.ReadAllBytes(path).SequenceEqual(OriginalEnvelope(state)), "replacement differs from original v1 envelope bytes");
                check(File.ReadAllBytes(path + ".previous").SequenceEqual(expected), "replacement changed the exact previous durable document");
                check(SaveStore.Load<EncodingFixture>(path).Cargo == 9.5 && SaveStore.Load<EncodingFixture>(path + ".previous").Cargo == 2.25, "current or prior durable document failed strict load");
                check(!Directory.EnumerateFiles(directory, "*.tmp-*").Any(), "successful save retained temporary state");
            });
        });
    }

    static string OriginalChecksum(string payload)
    {
        using var hash = SHA256.Create();
        return Convert.ToHexString(hash.ComputeHash(Encoding.UTF8.GetBytes(payload))).ToLowerInvariant();
    }

    static byte[] OriginalEnvelope(EncodingFixture state)
    {
        string payload = WireJson.Encode(state);
        return Encoding.UTF8.GetBytes(WireJson.Encode(new
        {
            Format = "idle-cat-forest-unity",
            Version = 1,
            Payload = payload,
            Sha256 = OriginalChecksum(payload)
        }));
    }

    static void InDirectory(Action<string> test)
    {
        string directory = Path.Combine(Path.GetTempPath(), "forest-save-encoding-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(directory);
        try { test(directory); }
        finally { Directory.Delete(directory, true); }
    }

    sealed class EncodingFixture { public string Label = ""; public double Cargo; public double[] Numbers = Array.Empty<double>(); }
}
