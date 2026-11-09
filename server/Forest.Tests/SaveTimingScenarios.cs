using IdleCatForest.Authority;
using IdleCatForest.Simulation;
using System.Text;

static class SaveTimingScenarios
{
    static readonly SaveStage[] StorageStages =
    {
        SaveStage.PayloadEncode, SaveStage.PayloadUtf8Hash, SaveStage.EnvelopeEncode,
        SaveStage.TempSetupProtection, SaveStage.EnvelopeUtf8Write, SaveStage.FlushClose,
        SaveStage.MoveReplace, SaveStage.DirectorySync
    };

    public static void Run(Action<string, Action> test, Action<bool, string> check)
    {
        test("save stage timing smoke measures exact bytes and ordered durable stages", () =>
        {
            InDirectory(directory =>
            {
                var path = Path.Combine(directory, "world.json");
                var state = new TimingFixture { Label = "synthetic non-ASCII payload 猫", Cargo = 2.25 };
                SaveStore.Save(path, state, out var timing, true);
                CheckTiming(timing, StorageStages, true, SaveStage.None, check);
                check(timing.PayloadBytes == Encoding.UTF8.GetByteCount(WireJson.Encode(state)), "payload UTF-8 byte count differs from encoded state");
                check(timing.EnvelopeBytes == new FileInfo(path).Length && timing.EnvelopeBytes > timing.PayloadBytes, "envelope UTF-8 byte count differs from durable bytes");
                var ordinary = Path.Combine(directory, "ordinary.json");
                SaveStore.Save(ordinary, state, true);
                check(File.ReadAllBytes(path).SequenceEqual(File.ReadAllBytes(ordinary)), "opt-in timings changed the v1 save document");
                check(WireJson.Encode(SaveStore.Load<TimingFixture>(path)) == WireJson.Encode(state), "timed save changed strict roundtrip state");
                check(!WireJson.Encode(timing).Contains(state.Label) && !WireJson.Encode(timing).Contains(directory), "diagnostics retained payload or path information");
            });
        });

        test("save stage timing smoke attributes refused installation and cleans temporary state", () =>
        {
            InDirectory(directory =>
            {
                var path = Path.Combine(directory, "world.json");
                SaveStore.Save(path, new TimingFixture { Label = "durable", Cargo = 2.25 }, true);
                var original = File.ReadAllBytes(path);
                SaveTiming timing = null;
                try
                {
                    SaveStore.Save(path, new TimingFixture { Label = "refused", Cargo = 9 }, out timing, true);
                    throw new Exception("requireNew overwrote a durable save");
                }
                catch (IOException) { }
                CheckTiming(timing, StorageStages.Take(7).ToArray(), false, SaveStage.MoveReplace, check);
                check(timing.PayloadBytes > 0 && timing.EnvelopeBytes > timing.PayloadBytes, "failed installation lost the encoded byte counts");
                check(File.ReadAllBytes(path).SequenceEqual(original) && !File.Exists(path + ".previous"), "refused installation changed the durable save or created a backup");
                check(!Directory.EnumerateFiles(directory, "world.json.tmp-*").Any(), "refused installation retained temporary save state");
            });
        });

        test("save stage timing smoke attributes validation failure before serialization", () =>
        {
            InDirectory(directory =>
            {
                var path = Path.Combine(directory, "world.json");
                using var runtime = new AuthorityRuntime(path, 41, "synthetic-fixture-key-not-a-credential");
                var original = File.ReadAllBytes(path);
                runtime.World.TimeSeconds = double.NaN;
                SaveTiming timing = null;
                try { runtime.Save(out timing); throw new Exception("invalid authority state was saved"); }
                catch (InvalidDataException) { }
                CheckTiming(timing, new[] { SaveStage.LockWait, SaveStage.Validation }, false, SaveStage.Validation, check);
                check(timing.PayloadBytes == 0 && timing.EnvelopeBytes == 0, "validation failure began serialization");
                check(File.ReadAllBytes(path).SequenceEqual(original) && !File.Exists(path + ".previous"), "validation failure changed durable state");
            });
        });

        test("save stage timing authority roundtrip retains full state and writer exclusion", () =>
        {
            InDirectory(directory =>
            {
                var path = Path.Combine(directory, "world.json");
                using var runtime = new AuthorityRuntime(path, 41, "synthetic-fixture-key-not-a-credential");
                var village = runtime.World.Villages.Single();
                village.Cats[0].Cargo.Add(new IdleCatForest.Simulation.Stack("logs", 2.25));
                check(runtime.World.Reserve(village, "synthetic-reservation", "water", 1.5), "fixture could not reserve water");
                village.Buildings[0].Queue.Add(new QueueEntry { RecipeId = "synthetic-paused-order", Repeat = true });
                village.Buildings[0].Paused = true;
                runtime.World.Random();
                var expected = WireJson.Encode(runtime.World);
                runtime.Save(out var timing);
                CheckTiming(timing, new[] { SaveStage.LockWait, SaveStage.Validation }.Concat(StorageStages).ToArray(), true, SaveStage.None, check);
                var saved = SaveStore.Load<World>(path);
                AuthorityRuntime.ValidateWorld(saved);
                check(WireJson.Encode(saved) == expected, "timed save changed full authoritative state");
                check(File.Exists(path + ".previous"), "timed replacement did not retain the prior durable document");
                try
                {
                    using var duplicate = new AuthorityRuntime(path, 99, "synthetic-fixture-key-not-a-credential");
                    throw new Exception("timed save released the single-writer lease");
                }
                catch (IOException) { }
            });
        });

        test("save stage timing records stay independent across ordinary and failed saves", () =>
        {
            InDirectory(directory =>
            {
                var path = Path.Combine(directory, "world.json");
                var state = new TimingFixture { Label = "first", Cargo = 2.25 };
                SaveStore.Save(path, state, out var first, true);
                var firstRecord = WireJson.Encode(first);
                state.Label = "ordinary";
                SaveStore.Save(path, state);
                SaveTiming refused = null;
                try { SaveStore.Save(path, state, out refused, true); throw new Exception("requireNew admitted a later save"); }
                catch (IOException) { }
                check(!ReferenceEquals(first, refused) && WireJson.Encode(first) == firstRecord, "a later ordinary or failed save mutated an earlier trace");
                CheckTiming(first, StorageStages, true, SaveStage.None, check);
                CheckTiming(refused, StorageStages.Take(7).ToArray(), false, SaveStage.MoveReplace, check);
                check(SaveStore.Load<TimingFixture>(path).Label == "ordinary", "failed diagnostics changed the ordinary save");
            });
        });

        test("save stage timing attributes payload exceptions without beginning a write", () =>
        {
            InDirectory(directory =>
            {
                var path = Path.Combine(directory, "world.json");
                SaveStore.Save(path, new TimingFixture { Label = "durable", Cargo = 2.25 }, true);
                var original = File.ReadAllBytes(path);
                var cyclic = new CyclicFixture(); cyclic.Next = cyclic;
                SaveTiming timing = null;
                try { SaveStore.Save(path, cyclic, out timing); throw new Exception("reference-loop payload was saved"); }
                catch (Newtonsoft.Json.JsonSerializationException) { }
                CheckTiming(timing, new[] { SaveStage.PayloadEncode }, false, SaveStage.PayloadEncode, check);
                check(timing.PayloadBytes == 0 && timing.EnvelopeBytes == 0, "failed payload encoding reported completed byte buffers");
                check(File.ReadAllBytes(path).SequenceEqual(original) && !File.Exists(path + ".previous"), "payload exception changed durable state");
                check(!Directory.EnumerateFiles(directory, "world.json.tmp-*").Any(), "payload failure created temporary state");
            });
        });

        test("save stage timing attributes denied temporary setup and recovers durably", () =>
        {
            InDirectory(directory =>
            {
                if (OperatingSystem.IsWindows()) return;
                var path = Path.Combine(directory, "world.json");
                SaveStore.Save(path, new TimingFixture { Label = "durable", Cargo = 2.25 }, true);
                var original = File.ReadAllBytes(path);
                var permissions = File.GetUnixFileMode(directory);
                SaveTiming denied = null;
                try
                {
                    File.SetUnixFileMode(directory, UnixFileMode.UserRead | UnixFileMode.UserExecute);
                    try { SaveStore.Save(path, new TimingFixture { Label = "denied", Cargo = 9 }, out denied); throw new Exception("fixture failed to deny temporary-file creation"); }
                    catch (UnauthorizedAccessException) { }
                    CheckTiming(denied, StorageStages.Take(4).ToArray(), false, SaveStage.TempSetupProtection, check);
                    check(denied.PayloadBytes > 0 && denied.EnvelopeBytes == 0, "denied setup reported written envelope bytes");
                    check(File.ReadAllBytes(path).SequenceEqual(original) && !File.Exists(path + ".previous"), "denied setup changed durable state");
                    check(!Directory.EnumerateFiles(directory, "world.json.tmp-*").Any(), "denied setup retained temporary state");
                }
                finally { File.SetUnixFileMode(directory, permissions); }
                SaveStore.Save(path, new TimingFixture { Label = "recovered", Cargo = 3.5 }, out var recovered);
                CheckTiming(recovered, StorageStages, true, SaveStage.None, check);
                check(SaveStore.Load<TimingFixture>(path).Cargo == 3.5 && SaveStore.Load<TimingFixture>(path + ".previous").Cargo == 2.25, "permission recovery lost current or prior durable state");
            });
        });
    }

    static void CheckTiming(SaveTiming timing, SaveStage[] expected, bool success, SaveStage failedStage, Action<bool, string> check)
    {
        check(timing != null, "opt-in save did not return call-scoped timing");
        var attempted = timing.Stages.Where(stage => stage.Attempted).ToArray();
        check(attempted.Select(stage => stage.Stage).SequenceEqual(expected), "missing or out-of-order attempted save stages: " + string.Join(",", attempted.Select(stage => stage.Stage)));
        check(timing.Success == success && timing.FailedStage == failedStage, "save success or failed-stage attribution differs from the actual result");
        check(attempted.All(stage => double.IsFinite(stage.Milliseconds) && stage.Milliseconds >= 0), "a save stage has an invalid duration");
        check(attempted.All(stage => stage.Succeeded == (stage.Stage != failedStage)), "stage completion differs from the actual failure boundary");
        var sum = attempted.Sum(stage => stage.Milliseconds);
        check(double.IsFinite(timing.TotalMilliseconds) && timing.TotalMilliseconds >= sum && timing.TotalMilliseconds > 0, "outer total omits stage duration");
        check(double.IsFinite(timing.ResidualMilliseconds) && timing.ResidualMilliseconds >= 0 && Math.Abs(timing.TotalMilliseconds - sum - timing.ResidualMilliseconds) < 0.001, "outer residual overlaps stages or differs from the total");
    }

    static void InDirectory(Action<string> test)
    {
        var directory = Path.Combine(Path.GetTempPath(), "forest-save-timing-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(directory);
        try { test(directory); }
        finally { Directory.Delete(directory, true); }
    }

    sealed class TimingFixture { public string Label = ""; public double Cargo; }
    sealed class CyclicFixture { public CyclicFixture Next; }
}
