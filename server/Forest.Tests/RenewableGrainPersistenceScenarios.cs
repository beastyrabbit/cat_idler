using IdleCatForest.Acceptance;
using IdleCatForest.Authority;
using IdleCatForest.Simulation;

static class RenewableGrainPersistenceScenarios
{
    public static void Run(Action<string, Action> test, Action<bool, string> check)
    {
        test("renewable Grain farm Steward Mill chain survives strict durable checkpoints", () =>
        {
            string directory = Path.Combine(Path.GetTempPath(), "forest-renewable-grain-" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(directory);
            int checkpoints = 0;
            try
            {
                RenewableGrainScenarios.RunChain((world, label) =>
                {
                    AuthorityRuntime.ValidateWorld(world);
                    string expected = WireJson.Encode(world);
                    string path = Path.Combine(directory, $"checkpoint-{++checkpoints}.json");
                    SaveStore.Save(path, world, requireNew: true);
                    check(WireJson.Encode(world) == expected, "Save changed authority at " + label);
                    var restored = SaveStore.Load<World>(path);
                    check(WireJson.Encode(restored) == expected, "Strict durable round trip changed authority at " + label);
                    AuthorityRuntime.ValidateWorld(restored);
                    Console.WriteLine($"Renewable Grain checkpoint {checkpoints}: {label}; t={restored.TimeSeconds:R}; claims={restored.Reservations.Count}");
                    return restored;
                });
                check(checkpoints >= 8, "Renewable chain did not exercise enough distinct durable boundaries");
            }
            finally
            {
                // This unique test directory contains only checkpoints we created.
                Directory.Delete(directory, recursive: true);
            }
        });
    }
}
