using IdleCatForest.Acceptance;
using IdleCatForest.Authority;
using IdleCatForest.Simulation;

static class PrimaryQueuePersistenceScenarios
{
    public static void Run(Action<string, Action> test, Action<bool, string> check)
    {
        test("primary queue automatic production matches uninterrupted authority through durable checkpoints", () =>
            VerifyTwins(PrimaryQueueContinuityScenarios.RunChain, 8, check));
        test("primary queue historical divergent mirror survives strict save and domain mutation", () =>
            VerifyTwins(PrimaryQueueContinuityScenarios.RunHistoricalContinuity, 2, check));
    }

    static void VerifyTwins(Action<Func<World, string, World>> run, int minimum, Action<bool, string> check)
    {
        var expected = new List<(string Label, string Authority)>();
        run((world, label) =>
        {
            AuthorityRuntime.ValidateWorld(world);
            expected.Add((label, WireJson.Encode(world)));
            return world;
        });

        string directory = Path.Combine(Path.GetTempPath(), "forest-primary-queue-chain-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(directory);
        int index = 0;
        try
        {
            run((world, label) =>
            {
                AuthorityRuntime.ValidateWorld(world);
                string before = WireJson.Encode(world);
                check(index < expected.Count && expected[index].Label == label,
                    "durable chain changed checkpoint order at " + label);
                check(expected[index].Authority == before,
                    "automatic production differs from uninterrupted whole authority at " + label);
                string path = Path.Combine(directory, "checkpoint-" + (++index) + ".json");
                SaveStore.Save(path, world, requireNew: true);
                check(WireJson.Encode(world) == before, "Save changed authority at " + label);
                var restored = SaveStore.Load<World>(path);
                check(WireJson.Encode(restored) == before, "Strict reload changed authority at " + label);
                AuthorityRuntime.ValidateWorld(restored);
                Console.WriteLine($"Primary queue checkpoint {index}: {label}; t={restored.TimeSeconds:R}; claims={restored.Reservations.Count}");
                return restored;
            });
            check(index == expected.Count && index >= minimum, "primary queue chain missed durable causal boundaries");
        }
        finally
        {
            // Only the unique disposable checkpoint directory owned above.
            Directory.Delete(directory, recursive: true);
        }
    }
}
