using IdleCatForest.Authority;
using IdleCatForest.Simulation;
using IdleCatForest.Server;
using Microsoft.AspNetCore.Hosting.Server;
using Microsoft.AspNetCore.Hosting.Server.Features;
using Microsoft.Extensions.DependencyInjection;
using IdleCatForest.SaveImport;
using Newtonsoft.Json.Linq;

var failures = new List<string>();
void Test(string name, Action test)
{
    if (Environment.GetEnvironmentVariable("FOREST_TEST") is string selected && !name.Contains(selected, StringComparison.OrdinalIgnoreCase)) return;
    try { test(); Console.WriteLine($"PASS {name}"); }
    catch (Exception error) { failures.Add(name); Console.Error.WriteLine($"FAIL {name}: {error.GetType().Name}: {error.Message}"); }
}
void Check(bool value, string reason) { if (!value) throw new Exception(reason); }
async Task AsyncTest(string name, Func<Task> test)
{
    if (Environment.GetEnvironmentVariable("FOREST_TEST") is string selected && !name.Contains(selected, StringComparison.OrdinalIgnoreCase)) return;
    try { await test(); Console.WriteLine($"PASS {name}"); }
    catch (Exception error) { failures.Add(name); Console.Error.WriteLine($"FAIL {name}: {error.GetType().Name}: {error.Message}"); }
}

Test("signed identity survives renewal without admitting forgery", () =>
{
    var authority = new SessionAuthority(System.Text.Encoding.UTF8.GetBytes("synthetic-fixture-key-not-a-credential"));
    var original = authority.Issue(1000);
    Check(authority.Verify(original, 1000), "issued session rejected");
    Check(!authority.Verify(original, 999), "future token accepted");
    Check(!authority.Verify(original, 1001 + SessionAuthority.MaximumAgeMilliseconds), "expired token accepted");
    var renewed = authority.Renew(original, 1001 + SessionAuthority.MaximumAgeMilliseconds);
    Check(renewed != null && renewed.PlayerId == original.PlayerId, "renewal lost village owner");
    original.Sig = new string('0', 64);
    Check(!authority.Verify(original, 1000), "forged signature accepted");
    Check(authority.Renew(original, 1000) == null, "forgery renewed");
});
Test("legacy signed bearer renews into same stable village owner", () =>
{
    var key = System.Text.Encoding.UTF8.GetBytes("synthetic-fixture-key-not-a-credential"); var authority = new SessionAuthority(key);
    var id = "session_0123456789abcdef0123456789abcdef";
    using var hmac = new System.Security.Cryptography.HMACSHA256(key);
    var sig = Convert.ToHexString(hmac.ComputeHash(System.Text.Encoding.UTF8.GetBytes(id))).ToLowerInvariant();
    var old = authority.Restore(id, sig); Check(!authority.Verify(old, 1000), "permanent legacy token remains usable without upgrade");
    var renewed = authority.Renew(old, 1000); Check(renewed != null && renewed.PlayerId == old.PlayerId, "legacy renewal changed owner");
    Check(authority.Verify(renewed, 1000), "renewed bearer invalid");
    Check(authority.Renew(renewed, 1001 + SessionAuthority.MaximumAgeMilliseconds + SessionAuthority.RenewalGraceMilliseconds) == null, "over-grace token renewed");
});
Test("strict JSON rejects duplicate properties nonfinite values and type metadata", () =>
{
    foreach (var json in new[] { "{\"Cargo\":1,\"Cargo\":2}", "{\"Cargo\":NaN}", "{\"Cargo\":Infinity}", "{\"Unexpected\":1}", "{\"$type\":\"System.IO.FileInfo, mscorlib\",\"Cargo\":1}" })
    {
        try { WireJson.Decode<SaveFixture>(json); throw new Exception("invalid payload accepted"); } catch (Newtonsoft.Json.JsonException) { }
    }
});
Test("strict JSON rejects numeric strings and case variant duplicate members", () =>
{
    var accepted = new List<string>();
    foreach (var json in new[] { "{\"Cargo\":\"NaN\"}", "{\"Cargo\":\"Infinity\"}", "{\"Cargo\":\"-Infinity\"}", "{\"Cargo\":\"1e400\"}", "{\"Cargo\":1,\"cargo\":2}" })
    {
        try { WireJson.Decode<SaveFixture>(json); accepted.Add(json); }
        catch (Newtonsoft.Json.JsonException) { }
    }
    Check(accepted.Count == 0, "ambiguous or nonfinite payloads accepted: " + string.Join("; ", accepted));
    var valid = WireJson.Decode<SaveFixture>("{\"Identity\":\"NaN\",\"Cargo\":2.25,\"Reserved\":1}");
    Check(valid.Identity == "NaN" && valid.Cargo == 2.25 && valid.Reserved == 1, "strict numeric parsing rejected valid strings or numbers");
    var directory = Path.Combine(Path.GetTempPath(), "forest-nonfinite-save-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        var path = Path.Combine(directory, "world.json");
        SaveStore.Save(path, new { Identity = "cat-13", Cargo = "NaN", Reserved = 1.5 }, true);
        var original = File.ReadAllBytes(path);
        try { SaveStore.Load<SaveFixture>(path); throw new Exception("checksummed save admitted a quoted nonfinite payload"); }
        catch (InvalidDataException) { }
        Check(File.ReadAllBytes(path).SequenceEqual(original) && !File.Exists(path + ".previous"), "rejected save was changed or replaced");
    }
    finally { Directory.Delete(directory, true); }
});

await AsyncTest("shared host awaits final durable save before completing shutdown", async () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-shutdown-test-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        var path = Path.Combine(directory, "world.json");
        using var runtime = new AuthorityRuntime(path, 41);
        var owner = runtime.Connect(null, 1000);
        var cat = runtime.World.Villages.Single().Cats[0];
        cat.Cargo.Add(new IdleCatForest.Simulation.Stack("logs", 2.25));
        await using var app = HostEntry.Build(runtime, "http://127.0.0.1:0", true);
        await app.StartAsync();
        using var acquired = new ManualResetEventSlim();
        using var release = new ManualResetEventSlim();
        var holder = Task.Run(() =>
        {
            lock (runtime.Sync) { acquired.Set(); release.Wait(); }
        });
        acquired.Wait();
        Task stopped = null;
        using var stopping = new ManualResetEventSlim();
        using var stoppingRegistration = app.Lifetime.ApplicationStopping.Register(stopping.Set);
        try
        {
            // Cancellation may run the worker's final save synchronously. Keep the
            // lock-holding test thread free to release it after observing shutdown.
            stopped = Task.Run(() => app.StopAsync());
            Check(stopping.Wait(TimeSpan.FromSeconds(5)), "host did not begin shutdown");
            var first = await Task.WhenAny(stopped, Task.Delay(50));
            Check(first != stopped, "host completed shutdown while its final save was still blocked");
        }
        finally
        {
            release.Set();
            await holder;
            if (stopped != null) await stopped;
        }
        var saved = SaveStore.Load<World>(path);
        Check(saved.Players.Any(player => player.Id == owner.Credential.PlayerId), "shutdown returned before persisting the current authority identity");
        var expected = WireJson.Encode(runtime.World);
        Check(WireJson.Encode(saved) == expected, "shutdown omitted part of the final authoritative world");
        AuthorityRuntime.ValidateWorld(saved);
        runtime.Dispose();
        using var resumed = new AuthorityRuntime(path, 999);
        Check(WireJson.Encode(resumed.World) == expected && World.Amount(resumed.World.Cat(cat.Id).Cargo, "logs") == 2.25, "immediate restart changed the saved identity or exact cargo");
    }
    finally { Directory.Delete(directory, true); }
});

Test("versioned save roundtrip preserves exact cargo and fails closed on corruption", () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-authority-test-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        var path = Path.Combine(directory, "world.json");
        SaveStore.Save(path, new SaveFixture { Identity = "cat-13", Cargo = 2.25, Reserved = 1.5 }, true);
        var restored = SaveStore.Load<SaveFixture>(path);
        Check(restored.Identity == "cat-13" && restored.Cargo == 2.25 && restored.Reserved == 1.5, "save lost authoritative fields");
        var original = File.ReadAllText(path);
        try { SaveStore.Save(path, new SaveFixture(), true); throw new Exception("overwrote existing world"); }
        catch (IOException) { }
        Check(File.ReadAllText(path) == original, "refused overwrite still changed source");
        File.WriteAllText(path, original.Replace("2.25", "9.25"));
        try { SaveStore.Load<SaveFixture>(path); throw new Exception("corrupt save accepted"); }
        catch (InvalidDataException) { }
    }
    finally { Directory.Delete(directory, true); }
});
Test("legacy native bearer import is private and never rewrites its source", () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-bearer-test-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        var source = Path.Combine(directory, "legacy.json"); var target = Path.Combine(directory, "new.json");
        var authority = new SessionAuthority(System.Text.Encoding.UTF8.GetBytes("synthetic-fixture-key-not-a-credential"));
        var credential = authority.Issue(1000); CredentialStore.Save(source, credential, "personal-village");
        var original = File.ReadAllBytes(source); CredentialStore.ImportLegacy(source, target);
        Check(File.ReadAllBytes(source).SequenceEqual(original), "source bearer changed");
        var imported = CredentialStore.Load(target); Check(imported.PlayerId == credential.PlayerId && imported.SessionId == credential.SessionId, "owner changed");
        if (!OperatingSystem.IsWindows()) Check((File.GetUnixFileMode(target) & (UnixFileMode.GroupRead | UnixFileMode.OtherRead)) == 0, "bearer readable by other users");
        try { CredentialStore.ImportLegacy(source, target); throw new Exception("duplicate import overwrote destination"); } catch (IOException) { }
        var preferences = JObject.Parse(File.ReadAllText(target)); preferences["nickname"] = "Fixture nickname"; preferences["textScale"] = 1.25; AtomicFile.Write(target, preferences.ToString());
        CredentialStore.Save(target, credential, "personal-village"); var saved = JObject.Parse(File.ReadAllText(target));
        Check((string)saved["nickname"] == "Fixture nickname" && (double)saved["textScale"] == 1.25, "bearer renewal reset native preferences");
    }
    finally { Directory.Delete(directory, true); }
});
await AsyncTest("shared host survives save permission loss and recovers readiness", async () =>
{
    if (OperatingSystem.IsWindows()) return;
    var directory = Path.Combine(Path.GetTempPath(), "forest-save-permission-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    var permissions = File.GetUnixFileMode(directory);
    try
    {
        var path = Path.Combine(directory, "world.json");
        using var runtime = new AuthorityRuntime(path, 41);
        var original = File.ReadAllBytes(path);
        var catIds = runtime.World.Villages.Single().Cats.Select(cat => cat.Id).ToArray();
        await using var app = HostEntry.Build(runtime, "http://127.0.0.1:0", true);
        await app.StartAsync();
        try
        {
            var address = app.Services.GetRequiredService<IServer>().Features.Get<IServerAddressesFeature>().Addresses.Single();
            using var http = new HttpClient { BaseAddress = new Uri(address), Timeout = TimeSpan.FromSeconds(2) };
            File.SetUnixFileMode(directory, UnixFileMode.UserRead | UnixFileMode.UserExecute);
            try { runtime.Save(); throw new Exception("fixture did not deny save access"); }
            catch (UnauthorizedAccessException) { }
            Check((await http.GetAsync("/ready")).IsSuccessStatusCode, "host began unready before automatic save failures");
            await Task.Delay(TimeSpan.FromSeconds(6.5));
            lock (runtime.Sync) Check(runtime.World.TimeSeconds > 5.7, "save permission failure stopped automatic simulation ticks");
            async Task WaitForReadiness(System.Net.HttpStatusCode expected, TimeSpan timeout)
            {
                var clock = System.Diagnostics.Stopwatch.StartNew();
                while (clock.Elapsed < timeout)
                {
                    using var response = await http.GetAsync("/ready");
                    if (response.StatusCode == expected) return;
                    await Task.Delay(100);
                }
                throw new Exception("readiness did not become " + expected);
            }
            await WaitForReadiness(System.Net.HttpStatusCode.ServiceUnavailable, TimeSpan.FromSeconds(12));
            Check(File.ReadAllBytes(path).SequenceEqual(original), "failed saves changed the last durable world");
            double beforeRecovery;
            lock (runtime.Sync) beforeRecovery = runtime.World.TimeSeconds;
            Check(beforeRecovery >= 14, "repeated save failures stopped simulation");
            File.SetUnixFileMode(directory, permissions);
            await WaitForReadiness(System.Net.HttpStatusCode.OK, TimeSpan.FromSeconds(7));
            var saved = SaveStore.Load<World>(path);
            Check(saved.TimeSeconds > beforeRecovery, "readiness recovered without saving the advanced world");
            Check(saved.Villages.Single().Cats.Select(cat => cat.Id).SequenceEqual(catIds), "save recovery replaced founding identities");
            AuthorityRuntime.ValidateWorld(saved);
        }
        finally
        {
            File.SetUnixFileMode(directory, permissions);
            await app.StopAsync();
            // The automatic loop observes shutdown on its next timer continuation.
            await Task.Delay(150);
        }
    }
    finally { File.SetUnixFileMode(directory, permissions); Directory.Delete(directory, true); }
});
Test("full simulation save retains cargo reservations queues identities and entropy", () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-runtime-test-" + Guid.NewGuid().ToString("N")); Directory.CreateDirectory(directory);
    try
    {
        var path = Path.Combine(directory, "world.json"); string expected;
        using (var authority = new LocalAuthority(path, 4242))
        {
            var village = authority.World.Villages[0]; var cat = village.Cats[0];
            cat.Cargo.Add(new IdleCatForest.Simulation.Stack("logs", 2.25));
            Check(authority.World.Reserve(village, "synthetic-job", "water", 1.5), "could not reserve fixture water");
            village.Buildings[0].Queue.Add(new QueueEntry { RecipeId = "fixture-paused-queue", Repeat = true }); village.Buildings[0].Paused = true;
            authority.World.Random(); authority.Save(); expected = WireJson.Encode(authority.World);
            try { using var duplicate = new LocalAuthority(path, 99); throw new Exception("second writer acquired live save"); } catch (IOException) { }
        }
        using (var resumed = new LocalAuthority(path, 99)) Check(WireJson.Encode(resumed.World) == expected, "reload changed full authoritative aggregate");
    }
    finally { Directory.Delete(directory, true); }
});
Test("socket projection never exposes private villages owners or stale exact stock", () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-projection-test-" + Guid.NewGuid().ToString("N")); Directory.CreateDirectory(directory);
    try
    {
        using var runtime = new AuthorityRuntime(Path.Combine(directory, "world.json"), 7);
        var first = runtime.Connect(null, 1000); var second = runtime.Connect(null, 1000);
        var found = runtime.Apply(first, new GameAction { Kind = "FoundVillage", Name = "Private Sentinel" }); Check(found.Success, found.Error);
        var village = runtime.World.Villages.First(v => v.Id == found.VillageId); var pile = village.Stockpiles[0];
        pile.Goods.Clear(); pile.Goods.Add(new IdleCatForest.Simulation.Stack("water", 731.234)); pile.Report.Clear(); pile.Report.Add(new IdleCatForest.Simulation.Stack("water", 1.25)); pile.CountedAt = 0;
        village.Election = new Election { Votes = new List<Ballot> { new Ballot { PlayerId = "private-election-voter" } } };
        village.KickPetition = new Election { Votes = new List<Ballot> { new Ballot { PlayerId = "private-petition-voter" } } };
        village.ElectionHistory.Add(new HistoricElection { Votes = new List<Ballot> { new Ballot { PlayerId = "private-history-voter" } } });
        var canonical = WireJson.Encode(runtime.World); var own = runtime.Project(first); var other = runtime.Project(second); var anonymous = runtime.Project(null);
        Check(own.Sequence > 0 && other.Sequence > own.Sequence && anonymous.Sequence > other.Sequence, "projection sequence did not advance while simulation was paused");
        Check(!WireJson.Encode(own).Contains("731.234"), "owner received stale exact stock");
        Check(!WireJson.Encode(other).Contains("Private Sentinel") && !WireJson.Encode(anonymous).Contains("Private Sentinel"), "private village leaked");
        Check(own.World.Villages.All(v => v.OwnerId == ""), "owner identifier leaked");
        Check(!WireJson.Encode(own).Contains("private-election-voter") && !WireJson.Encode(own).Contains("private-petition-voter") && !WireJson.Encode(own).Contains("private-history-voter"), "voter identity leaked from projected governance");
        Check(!runtime.Authenticate(first, second.Credential.SessionId, second.Credential.Sig, 1000), "socket accepted another valid session");
        Check(WireJson.Encode(runtime.World) == canonical, "projection mutated canonical state");
    }
    finally { Directory.Delete(directory, true); }
});
Test("known equipment orders and worn items remain actionable without revealing stale storage", () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-known-equipment-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        using var runtime = new AuthorityRuntime(Path.Combine(directory, "world.json"), 41);
        var owner = runtime.Connect(null, DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
        var village = runtime.World.Village(owner.SelectedVillageId); var pile = village.Stockpiles[0];
        var bearer = village.Cats[3]; var returner = village.Cats[4];
        var fetch = new Job { Id = "known-equipment-fetch", Kind = "equip", Phase = "item_fetch", SourceId = pile.Id, TargetId = "known-equipment-tool", Position = pile.Position, CatId = bearer.Id, Manual = true, ItemIds = new List<string> { "known-equipment-tool" } };
        village.Jobs.Add(fetch); bearer.JobId = fetch.Id;
        var returning = new Job { Id = "known-equipment-return", Kind = "unequip", Phase = "item_return", SourceId = returner.Id, TargetId = pile.Id, Position = pile.Position, CatId = returner.Id, Manual = true, ItemIds = new List<string> { "known-equipment-armor" } };
        village.Jobs.Add(returning); returner.JobId = returning.Id;
        village.Items.Add(new Item { Id = "known-equipment-tool", Kind = "tool", VillageId = village.Id, LocationId = fetch.Id, Quality = 3, Condition = 17, MaxCondition = 42 });
        village.Items.Add(new Item { Id = "known-equipment-weapon", Kind = "weapon", VillageId = village.Id, LocationId = bearer.Id }); bearer.Equipment.Add("known-equipment-weapon");
        village.Items.Add(new Item { Id = "known-equipment-armor", Kind = "armor", VillageId = village.Id, LocationId = returner.Id }); returner.Equipment.Add("known-equipment-armor");
        village.Items.Add(new Item { Id = "hidden-stored-tool", Kind = "tool", VillageId = village.Id, LocationId = pile.Id });
        var unreported = new Stockpile { Id = "hidden-equipment-spill", Kind = "spill", Position = village.Center }; village.Stockpiles.Add(unreported);
        village.Items.Add(new Item { Id = "hidden-spill-tool", Kind = "tool", VillageId = village.Id, LocationId = unreported.Id });
        string canonical = WireJson.Encode(runtime.World);
        var visible = runtime.Project(owner).World.Village(village.Id);
        Check(visible.Items.Select(i => i.Id).OrderBy(id => id).SequenceEqual(new[] { "known-equipment-armor", "known-equipment-tool", "known-equipment-weapon" }), "stale books hid an exact equipment order or visibly worn gear, or exposed unrelated storage");
        Check(visible.Jobs.Single(j => j.Id == fetch.Id).ItemIds.Single() == "known-equipment-tool" && visible.Jobs.Single(j => j.Id == returning.Id).ItemIds.Single() == "known-equipment-armor", "projection removed the exact cancelable equipment order identity");
        Check(visible.Cats.Single(c => c.Id == bearer.Id).Equipment.Single() == "known-equipment-weapon" && visible.Cats.Single(c => c.Id == returner.Id).Equipment.Single() == "known-equipment-armor", "projection removed visibly worn equipment references");
        Check(WireJson.Encode(runtime.World) == canonical, "equipment projection mutated authoritative ownership");
        bearer.JobId = ""; bearer.ResumeJobId = fetch.Id; bearer.Goal = "need_drink";
        visible = runtime.Project(owner).World.Village(village.Id);
        Check(visible.Items.Any(i => i.Id == "known-equipment-tool") && visible.Cats.Single(c => c.Id == bearer.Id).ResumeJobId == fetch.Id, "needs interruption hid the pending exact retrieval");
        Check(runtime.Project(null).World.Village(village.Id).Items.Count == 0, "anonymous projection revealed known equipment");
        string selected = owner.SelectedVillageId; owner.SelectedVillageId = "not-selected";
        Check(runtime.Project(owner).World.Village(village.Id).Items.Count == 0, "unselected village revealed known equipment"); owner.SelectedVillageId = selected;
        foreach (var storage in village.Stockpiles.Where(p => p.Kind == "storage"))
        {
            storage.CountedAt = runtime.World.TimeSeconds;
            storage.Report = storage.Goods.Select(s => new IdleCatForest.Simulation.Stack(s.Resource, s.Amount)).ToList();
        }
        World.Add(pile.Report, "tools", 2);
        visible = runtime.Project(owner).World.Village(village.Id);
        Check(visible.Items.Any(i => i.Id == "hidden-stored-tool") && visible.Items.All(i => i.Id != "hidden-spill-tool"), "an uncollected source claim falsely made unchanged physical storage stale or exposed the spill");
        World.Add(pile.Goods, "logs", 1);
        visible = runtime.Project(owner).World.Village(village.Id);
        Check(visible.Items.All(i => i.Id != "hidden-stored-tool") && visible.Items.Any(i => i.Id == "known-equipment-tool"), "real storage changes exposed stale inventory or removed the known order");
        AuthorityRuntime.ValidateWorld(runtime.World);
    }
    finally { Directory.Delete(directory, true); }
});
Test("known equipment retrieval from a spill preserves only its ordered item", () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-known-spill-equipment-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        using var runtime = new AuthorityRuntime(Path.Combine(directory, "world.json"), 41);
        var owner = runtime.Connect(null, DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
        var village = runtime.World.Village(owner.SelectedVillageId); var runner = village.Cats[1];
        var spill = new Stockpile { Id = "known-equipment-spill-source", Kind = "spill", Position = runner.Position }; village.Stockpiles.Add(spill);
        var job = new Job { Id = "known-equipment-spill-order", Kind = "equip", Phase = "item_fetch", SourceId = spill.Id, TargetId = "known-equipment-spill-tool", Position = spill.Position, CatId = runner.Id, Manual = true, ItemIds = new List<string> { "known-equipment-spill-tool" } };
        village.Jobs.Add(job); runner.JobId = job.Id;
        village.Items.Add(new Item { Id = job.TargetId, Kind = "tool", VillageId = village.Id, LocationId = job.Id });
        village.Items.Add(new Item { Id = "hidden-equipment-spill-weapon", Kind = "weapon", VillageId = village.Id, LocationId = spill.Id });
        var visible = runtime.Project(owner).World.Village(village.Id);
        Check(visible.Items.Count == 1 && visible.Items.Single().Id == job.TargetId && visible.Jobs.Single(j => j.Id == job.Id).ItemIds.Single() == job.TargetId, "projection hid a known spill-source order or exposed unrelated spill inventory");
        AuthorityRuntime.ValidateWorld(runtime.World);
    }
    finally { Directory.Delete(directory, true); }
});
await AsyncTest("counted equipment remains selectable over a real socket until its ledger becomes stale", async () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-counted-equipment-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        using var runtime = new AuthorityRuntime(Path.Combine(directory, "world.json"), 7);
        var owner = runtime.Connect(null, DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
        var village = runtime.World.Village(owner.SelectedVillageId);
        var pile = village.Stockpiles[0]; var accountant = village.Cats[2]; var bearer = village.Cats[3];
        // Keep this socket projection case away from emergency labor preemption.
        World.Add(pile.Goods, "food", 400); World.Add(pile.Goods, "water", 400); World.Add(pile.Goods, "logs", 20);
        village.LastLeaderResearch = runtime.World.TimeSeconds;
        bearer.Hunger = bearer.Thirst = bearer.Rest = 80;
        var tent = new Building { Id = "fixture-accounting-tent", Kind = "accounting_tent", Completed = true, Position = pile.Position };
        tent.Slots.Add(new WorkSlot { CatId = accountant.Id }); village.Buildings.Add(tent); accountant.BuildingId = tent.Id;
        village.Officers.Add(new Officer { Role = "accountant", CatId = accountant.Id });
        foreach (var kind in new[] { "tool", "weapon", "armor" })
            village.Items.Add(new Item { Id = "counted-" + kind, Kind = kind, VillageId = village.Id, LocationId = pile.Id, Material = "wood", Quality = 3, Condition = 17, MaxCondition = 42 });
        village.Items.Add(new Item { Id = "worn-weapon", Kind = "weapon", VillageId = village.Id, LocationId = bearer.Id });
        bearer.Equipment.Add("worn-weapon");
        var gather = new Stockpile { Id = "unreported-gather-pile", Kind = "zone_gather", Position = village.Center };
        gather.Goods.Add(new IdleCatForest.Simulation.Stack("logs", 17)); village.Stockpiles.Add(gather);
        village.Items.Add(new Item { Id = "unreported-pile-tool", Kind = "tool", VillageId = village.Id, LocationId = gather.Id });
        void CountStorage()
        {
            // Start each real accounting tick at its finite final dwell, with the worker present.
            foreach (var storage in village.Stockpiles.Where(p => p.Kind == "storage"))
            {
                accountant.Position = storage.Position; accountant.X = storage.Position.X; accountant.Z = storage.Position.Z; accountant.Path.Clear();
                village.Accounting = new AccountingRound { WorkerId = accountant.Id, BuildingId = tent.Id, TargetId = storage.Id, Phase = "counting", DwellSeconds = 4 };
                runtime.Advance(1);
                Check(storage.CountedAt == runtime.World.TimeSeconds, "physical Accountant did not finish its count");
            }
        }
        CountStorage();
        Check(World.Amount(pile.Report, "tools") == 1 && World.Amount(pile.Report, "weapons") == 1 && World.Amount(pile.Report, "armor") == 1, "Accountant omitted exact item categories");
        var credentialPath = Path.Combine(directory, "client.json"); CredentialStore.Save(credentialPath, owner.Credential, village.Id);
        await using var app = HostEntry.Build(runtime, "http://127.0.0.1:0", false); await app.StartAsync();
        var address = app.Services.GetRequiredService<IServer>().Features.Get<IServerAddressesFeature>().Addresses.Single().Replace("http://", "ws://") + "/ws";
        using var client = new WorldClient(address, credentialPath); await client.ConnectAsync();
        var visible = client.LatestWorld.Village(village.Id);
        Check(visible.Items.Count(i => i.Id.StartsWith("counted-", StringComparison.Ordinal)) == 3, "fresh counted stored equipment was removed from authorized socket snapshot");
        Check(visible.Cats.Single(c => c.Id == bearer.Id).Equipment.Contains("worn-weapon"), "fresh counted snapshot removed equipped item references");
        Check(visible.Items.All(i => i.Id != "unreported-pile-tool") && visible.Stockpiles.Single(p => p.Id == gather.Id).Goods.Count == 0, "unreported nonstorage pile contents became visible");
        var retrieval = await client.SendAsync(new GameAction { Kind = "EquipItem", CatId = bearer.Id, TargetId = "counted-tool" });
        Check(retrieval.Success, "authorized client could not order retrieval of its counted item");
        Check(village.Items.Single(i => i.Id == "counted-tool").LocationId == retrieval.EntityId && !bearer.Equipment.Contains("counted-tool"), "socket equipment action teleported its reserved item to the bearer");
        visible = client.LatestWorld.Village(village.Id);
        Check(visible.Items.Count(i => i.Id.StartsWith("counted-", StringComparison.Ordinal)) == 3 && visible.Jobs.Single(j => j.Id == retrieval.EntityId).ItemIds.Single() == "counted-tool", "source reservation falsely invalidated its unchanged physical count or hid its exact order");
        for (int tick = 0; tick < 600 && !village.Jobs.Single(j => j.Id == retrieval.EntityId).Completed; tick++) runtime.Advance(.05);
        Check(village.Jobs.Single(j => j.Id == retrieval.EntityId).Completed && bearer.Equipment.Contains("counted-tool"), "socket equipment retrieval did not physically arrive");
        await client.SendAsync(new GameAction { Kind = "Presence" });
        visible = client.LatestWorld.Village(village.Id);
        Check(visible.Items.Count == 2 && visible.Items.All(i => i.LocationId == bearer.Id) && visible.Cats.Single(c => c.Id == bearer.Id).Equipment.Count == 2, "stale counts hid worn gear or exposed other exact stored inventory");
        CountStorage(); await client.SendAsync(new GameAction { Kind = "Presence" });
        visible = client.LatestWorld.Village(village.Id);
        Check(visible.Items.Single(i => i.Id == "counted-tool").LocationId == bearer.Id && visible.Cats.Single(c => c.Id == bearer.Id).Equipment.Contains("counted-tool"), "recount failed to restore the equipped item identity");
        lock (runtime.Sync) World.Add(pile.Goods, "logs", 1);
        await client.SendAsync(new GameAction { Kind = "Presence" });
        visible = client.LatestWorld.Village(village.Id);
        Check(visible.Items.Count == 2 && visible.Items.All(i => i.LocationId == bearer.Id), "stale scalar count exposed stored exact equipment or removed visibly worn gear");
        Check(village.Items.Count == 5 && gather.Goods.Single().Amount == 17, "projection changed the canonical item or pile ledger");
        client.Dispose(); await app.StopAsync();
    }
    finally { Directory.Delete(directory, true); }
});
await AsyncTest("real loopback identities village privacy signed actions and restart", async () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-socket-test-" + Guid.NewGuid().ToString("N")); Directory.CreateDirectory(directory);
    var path = Path.Combine(directory, "world.json"); string firstVillage = "", secondVillage = "", firstPlayer = "";
    try
    {
        using (var runtime = new AuthorityRuntime(path, 4242))
        {
            await using var app = HostEntry.Build(runtime, "http://127.0.0.1:0", false);
            await app.StartAsync();
            var address = app.Services.GetRequiredService<IServer>().Features.Get<IServerAddressesFeature>().Addresses.Single().Replace("http://", "ws://") + "/ws";
            using var first = new WorldClient(address, Path.Combine(directory, "first.json")); using var second = new WorldClient(address, Path.Combine(directory, "second.json"));
            await first.ConnectAsync(); await second.ConnectAsync();
            firstPlayer = first.Credential.PlayerId; Check(firstPlayer != second.Credential.PlayerId, "two installations share identity");
            var found = await first.SendAsync(new GameAction { Kind = "FoundVillage", Name = "Private Alder" }); Check(found.Success, found.Error); firstVillage = found.VillageId;
            var other = await second.SendAsync(new GameAction { Kind = "FoundVillage", Name = "Private Birch" }); Check(other.Success, other.Error); secondVillage = other.VillageId;
            Check(first.LatestWorld.Villages.All(v => v.Id != secondVillage), "first client sees other private village");
            Check(second.LatestWorld.Villages.All(v => v.Id != firstVillage), "second client sees other private village");
            var denied = await second.SendAsync(new GameAction { Kind = "JoinVillage", TargetId = firstVillage }); Check(!denied.Success, "foreign village control granted");
            var test = await first.SendAsync(new GameAction { Kind = "AdvanceTime", Amount = 86400 }); Check(!test.Success, "release host accepted test time control");
            var cat = first.LatestWorld.Villages.First(v => v.Id == firstVillage).Cats[2];
            Check((await first.SendAsync(new GameAction { Kind = "EnterCatControl", CatId = cat.Id })).Success, "direct control denied for owned cat");
            Check(!(await second.SendAsync(new GameAction { Kind = "EnterCatControl", CatId = cat.Id })).Success, "foreign player possessed cat");
            Check((await first.SendAsync(new GameAction { Kind = "LeaveCatControl", CatId = cat.Id })).Success, "direct control handoff denied");
            var owned = runtime.World.Village(firstVillage); var foreign = runtime.World.Village(secondVillage);
            Int2 RoadSite(Village village) => runtime.World.Tiles.First(t => t.ClaimId == village.Id && !t.Wall && !t.Water && !t.Mountain && !t.Road && !t.Rail && runtime.World.Tiles.Any(n => n.Road && Int2.Distance(n.Position, t.Position) == 1)).Position;
            var foreignSite = RoadSite(foreign); owned.Known.Add(foreignSite); World.Add(owned.Stockpiles[0].Goods, "materials", 2);
            var beforeForeign = WireJson.Encode(runtime.World.GetTile(foreignSite)); var beforeJobs = WireJson.Encode(owned.Jobs); var beforeClaims = WireJson.Encode(runtime.World.Reservations);
            var foreignRoad = await first.SendAsync(new GameAction { Kind = "BuildRoad", CatId = cat.Id, Position = foreignSite, End = foreignSite });
            Check(!foreignRoad.Success, "signed coordinate construction bypassed another village's ownership");
            Check(WireJson.Encode(runtime.World.GetTile(foreignSite)) == beforeForeign && WireJson.Encode(owned.Jobs) == beforeJobs && WireJson.Encode(runtime.World.Reservations) == beforeClaims, "denied foreign construction changed terrain, jobs or claims");
            var ownSite = RoadSite(owned);
            Check((await first.SendAsync(new GameAction { Kind = "BuildRoad", CatId = cat.Id, Position = ownSite, End = ownSite })).Success, "owned mapped road construction was incorrectly forbidden");
            Check(runtime.World.TimeSeconds == 0, "client advanced server clock"); runtime.Advance(1); runtime.Save();
            first.Dispose(); second.Dispose(); await app.StopAsync();
        }
        using (var runtime = new AuthorityRuntime(path, 99))
        {
            await using var app = HostEntry.Build(runtime, "http://127.0.0.1:0", false); await app.StartAsync();
            var address = app.Services.GetRequiredService<IServer>().Features.Get<IServerAddressesFeature>().Addresses.Single().Replace("http://", "ws://") + "/ws";
            using var client = new WorldClient(address, Path.Combine(directory, "first.json")); await client.ConnectAsync();
            Check(client.Credential.PlayerId == firstPlayer, "restart changed owner");
            Check((await client.SendAsync(new GameAction { Kind = "JoinVillage", TargetId = firstVillage })).Success, "reconnected owner lost village");
            Check(client.LatestWorld.Villages.All(v => v.Id != secondVillage), "restart leaked foreign village");
            client.Dispose(); await app.StopAsync();
        }
    }
    finally { Directory.Delete(directory, true); }
});
await AsyncTest("autonomous live snapshots arrive throughout each second", async () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-live-snapshot-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        using var runtime = new AuthorityRuntime(Path.Combine(directory, "world.json"), 41);
        await using var app = HostEntry.Build(runtime, "http://127.0.0.1:0", true);
        await app.StartAsync();
        var address = app.Services.GetRequiredService<IServer>().Features.Get<IServerAddressesFeature>().Addresses.Single().Replace("http://", "ws://") + "/ws";
        using var client = new WorldClient(address, Path.Combine(directory, "client.json"));
        await client.ConnectAsync();
        var samples = new List<double>(); var clock = System.Diagnostics.Stopwatch.StartNew(); long sequence = client.LatestFrame.Sequence;
        while (clock.Elapsed.TotalSeconds < 1.25)
        {
            await Task.Delay(10);
            var frame = client.LatestFrame;
            if (frame.Sequence == sequence) continue;
            sequence = frame.Sequence; samples.Add(frame.World.TimeSeconds);
            Check(frame.World.Villages.SelectMany(v => v.Cats).All(c => c.ControlledBy == ""), "snapshot test accidentally enabled direct-control cadence");
        }
        Check(samples.Count >= 6, "autonomous world delivered only " + samples.Count + " snapshots in 1.25 seconds");
        Check(samples.Zip(samples.Skip(1), (a, b) => b - a).All(delta => delta > 0 && delta < 0.4), "autonomous snapshots remained on a coarse update cadence");
        client.Dispose(); await app.StopAsync();
    }
    finally { Directory.Delete(directory, true); }
});
await AsyncTest("automatic broadcasts cannot roll back newer selection and cat control", async () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-frame-order-test-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        using var runtime = new AuthorityRuntime(Path.Combine(directory, "world.json"), 4242);
        await using var app = HostEntry.Build(runtime, "http://127.0.0.1:0", true);
        await app.StartAsync();
        var address = app.Services.GetRequiredService<IServer>().Features.Get<IServerAddressesFeature>().Addresses.Single().Replace("http://", "ws://") + "/ws";
        await using var relay = await DelayedSnapshotRelay.StartAsync(address);
        using var client = new WorldClient(relay.Address, Path.Combine(directory, "client.json"));
        await client.ConnectAsync();
        await relay.CaptureNextBroadcast();
        var found = await client.SendAsync(new GameAction { Kind = "FoundVillage", Name = "Ordered Alder" });
        Check(found.Success, found.Error);
        Check(client.SelectedVillageId == found.VillageId, "delayed broadcast rolled back selected village");
        Check(JObject.Parse(File.ReadAllText(Path.Combine(directory, "client.json")))["selectedColonyId"]?.Value<string>() == found.VillageId, "delayed broadcast persisted the wrong selected village");
        var cat = client.LatestWorld.Village(found.VillageId).Cats[2];
        await relay.CaptureNextBroadcast();
        Check((await client.SendAsync(new GameAction { Kind = "EnterCatControl", CatId = cat.Id })).Success, "owned cat control failed");
        Check(client.LatestWorld.Village(found.VillageId).Cats.Single(c => c.Id == cat.Id).ControlledBy == client.Credential.PlayerId, "delayed broadcast rolled back entered cat control");
        await relay.CaptureNextBroadcast();
        Check((await client.SendAsync(new GameAction { Kind = "LeaveCatControl", CatId = cat.Id })).Success, "cat handoff failed");
        Check(client.LatestWorld.Village(found.VillageId).Cats.Single(c => c.Id == cat.Id).ControlledBy == "", "delayed broadcast restored released cat control");
        lock (runtime.Sync) Check(runtime.World.TimeSeconds > 0, "scenario did not use automatic simulation ticks");
        client.Dispose();
        await app.StopAsync();
    }
    finally { Directory.Delete(directory, true); }
});
await AsyncTest("signed physical trade resumes escrow after restart and transfers exact equipment once", async () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-trade-test-" + Guid.NewGuid().ToString("N")); Directory.CreateDirectory(directory);
    var path = Path.Combine(directory, "world.json"); string sourceId = "", targetId = "", tradeId = "";
    try
    {
        using (var runtime = new AuthorityRuntime(path, 42))
        {
            var source = runtime.Connect(null, DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()); var target = runtime.Connect(null, DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
            sourceId = runtime.Apply(source, new GameAction { Kind = "FoundVillage", Name = "Alder Trade" }).VillageId;
            targetId = runtime.Apply(target, new GameAction { Kind = "FoundVillage", Name = "Birch Trade" }).VillageId;
            CredentialStore.Save(Path.Combine(directory, "source.json"), source.Credential); CredentialStore.Save(Path.Combine(directory, "target.json"), target.Credential);
            var first = runtime.World.Village(sourceId); var second = runtime.World.Village(targetId);
            // All scenario setup precedes the listener. Actual actions, travel and restart follow.
            void Rebase(Village village, Int2 center)
            {
                var delta = new Int2(center.X - village.Center.X, center.Z - village.Center.Z);
                Int2 Move(Int2 p) => new(p.X + delta.X, p.Z + delta.Z);
                village.Center = center; village.Known = village.Known.Select(Move).ToList();
                foreach (var cat in village.Cats) { cat.Position = Move(cat.Position); cat.X += delta.X; cat.Z += delta.Z; }
                foreach (var building in village.Buildings) { building.Position = Move(building.Position); if (building.HasEntrance) building.Entrance = Move(building.Entrance); }
                foreach (var pile in village.Stockpiles) pile.Position = Move(pile.Position);
                foreach (var p in village.Known) { var tile = runtime.World.TileAt(p); World.ClearSurface(tile, 0); tile.Water = tile.Mountain = false; tile.Wall = false; }
            }
            Rebase(first, new Int2(30, 0)); Rebase(second, new Int2(45, 0));
            for (int x = 30; x <= 45; x++) { var tile = runtime.World.TileAt(new Int2(x, 0)); World.ClearSurface(tile, 0); tile.Water = tile.Mountain = tile.Wall = false; }
            first.Contacts.Add(second.Id); second.Contacts.Add(first.Id);
            first.Items.Add(new Item { Id = "exact-trade-tool", Kind = "tool", Material = "wood", Quality = 3, Condition = 17, MaxCondition = 42, Weight = 2.5, VillageId = first.Id, LocationId = first.Stockpiles[0].Id });
            second.Stockpiles[0].Goods.Add(new IdleCatForest.Simulation.Stack("gem", 5));
            await using var app = HostEntry.Build(runtime, "http://127.0.0.1:0", false); await app.StartAsync();
            var address = app.Services.GetRequiredService<IServer>().Features.Get<IServerAddressesFeature>().Addresses.Single().Replace("http://", "ws://") + "/ws";
            using var firstClient = new WorldClient(address, Path.Combine(directory, "source.json")); using var secondClient = new WorldClient(address, Path.Combine(directory, "target.json")); await firstClient.ConnectAsync(); await secondClient.ConnectAsync();
            var offered = await firstClient.SendAsync(new GameAction { Kind = "OfferVillageTrade", OtherVillageId = targetId, Resource = "tools", Amount = 1, OtherResource = "gem", OtherAmount = 2 }); Check(offered.Success, offered.Error); tradeId = offered.EntityId;
            var accepted = await secondClient.SendAsync(new GameAction { Kind = "AcceptVillageTrade", TargetId = tradeId }); Check(accepted.Success, accepted.Error);
            Check(first.Items.All(item => item.Id != "exact-trade-tool") && second.Items.All(item => item.Id != "exact-trade-tool"), "escrow item remained in village inventory");
            Check(runtime.World.TradeOffers.Single(t => t.Id == tradeId).OfferedItems.Single().Id == "exact-trade-tool", "escrow lost exact identity");
            runtime.Advance(3); Check(runtime.World.TradeOffers.Single(t => t.Id == tradeId).Status != "completed", "trade teleported to completion"); runtime.Save();
            firstClient.Dispose(); secondClient.Dispose(); await app.StopAsync();
        }
        using (var runtime = new AuthorityRuntime(path, 99))
        {
            var trade = runtime.World.TradeOffers.Single(t => t.Id == tradeId); Check(trade.OfferedItems.Single().Condition == 17, "restart reset equipment condition");
            for (int i = 0; i < 100 && trade.Status != "completed"; i++) runtime.Advance(1);
            Check(trade.Status == "completed", "caravan failed bounded physical delivery");
            var first = runtime.World.Village(sourceId); var second = runtime.World.Village(targetId);
            var exact = second.Items.Single(item => item.Id == "exact-trade-tool"); Check(exact.Condition == 17 && exact.Quality == 3 && exact.VillageId == targetId, "equipment metadata changed during trade");
            Check(first.Items.All(item => item.Id != "exact-trade-tool"), "source retained duplicated equipment");
            Check(runtime.World.Total(first, "gem") == 2 && runtime.World.Total(second, "gem") == 3, "barter scalar conservation failed");
            runtime.Advance(2); Check(second.Items.Count(item => item.Id == "exact-trade-tool") == 1 && runtime.World.Total(first, "gem") == 2, "delivery replayed");
            runtime.Save();
        }
    }
    finally { Directory.Delete(directory, true); }
});
Test("typed legacy world conversion preserves source cat cargo needs and simultaneous deposits", () =>
{
    var tableNames = new[] { "world", "colonies", "cats", "jobs", "buildings", "world_tiles", "shared_world_tiles", "events", "player_names", "zones", "elections", "votes", "raiders" };
    var tables = new JObject(); foreach (var table in tableNames) tables[table] = new JArray();
    ((JArray)tables["world"]).Add(new JObject { { "id", 1 }, { "worldSeed", 42 }, { "sharedFishHabitats", "[]" } });
    ((JArray)tables["colonies"]).Add(new JObject { { "id", "colony-1" }, { "name", "Imported Commons" }, { "isGlobal", 1 }, { "createdAt", 1000000 }, { "lastTick", 1001000 }, { "runStartedAt", 1000000 }, { "resources", "{\"food\":3,\"water\":5}" }, { "stockpiles", "[{\"id\":\"stockpile-storehouse\",\"rect\":{\"x1\":3,\"y1\":3,\"x2\":4,\"y2\":4},\"accepts\":[],\"contents\":{\"food\":3,\"water\":5}}]" }, { "revealedTiles", "[{\"x\":2,\"y\":2}]" } });
    ((JArray)tables["cats"]).Add(new JObject { { "id", "colony-1\u001fcat-17" }, { "colonyId", "colony-1" }, { "name", "Moss" }, { "birthTime", 900000 }, { "ageHours", 47.25 }, { "needs", "{\"hunger\":71,\"thirst\":62,\"rest\":83,\"health\":94}" }, { "position", "{\"map\":\"world\",\"x\":2.25,\"y\":2.5}" }, { "carrying", "{\"kind\":\"logs\",\"amount\":2.5,\"jobEndedAt\":1000000}" }, { "parentIds", "[\"parent-a\",null]" } });
    ((JArray)tables["shared_world_tiles"]).Add(new JObject { { "id", "2,2" }, { "x", 2 }, { "y", 2 }, { "type", "forest" }, { "resources", "{\"food\":4,\"materials\":8,\"wood\":9}" }, { "maxResources", "{\"food\":10,\"materials\":20,\"wood\":30}" } });
    var source = new JObject { { "Format", "idle-cat-forest-normalized-sqlite" }, { "Version", 1 }, { "Tables", tables } }.ToString();
    var world = LegacyImport.Convert(source); var cat = world.Villages[0].Cats.Single();
    Check(cat.Id == "colony-1\u001fcat-17" && cat.Hunger == 71 && cat.AgeHours == 47.25 && cat.X == 2.25, "cat identity or personal state lost");
    Check(cat.Cargo.Single().Resource == "logs" && cat.Cargo.Single().Amount == 2.5, "on-paw cargo lost");
    Check(cat.ParentIds[0] == "colony-1\u001fparent-a" && cat.ParentIds[1] == null, "lineage lost");
    var tile = world.GetTile(new Int2(2, 2)); Check(tile.Deposits.Count >= 2, "simultaneous deposits collapsed");
    Check(World.Amount(tile.MaximumDeposits, "logs") == 30 && tile.MaximumDeposits.All(s => s.Resource != "wood"), "deposit capacity uses a different resource identity from its finite goods");
    Check(world.Villages[0].Jobs.Any(job => job.CatId == cat.Id && job.Phase == "output_delivery"), "carried goods have no resumable delivery");
});
Test("movement rate budget preserves ordinary action limits", () =>
{
    var budget = new ActionBudget();
    for (int i = 0; i < 100; i++) Check(budget.Allow("ip", "socket", "owner", true, 1000 + i * 100), "10Hz signed movement was throttled");
    for (int i = 0; i < 30; i++) Check(budget.Allow("ip", "socket", "owner", false, 11000), "movement consumed economy action budget");
    Check(!budget.Allow("ip", "socket", "owner", false, 11001), "ordinary action limit bypassed");
    for (int i = 0; i < 120; i++) budget.Allow("ip", "socket", "owner", true, 12000);
    Check(!budget.Allow("ip", "other-socket", "owner", true, 12000), "reconnection bypassed signed player movement limit");
});
foreach (bool automaticTicks in new[] { false, true })
    await AsyncTest(automaticTicks ? "rate-limited socket still receives scheduled snapshots" : "rate-limited socket keeps its frame without building projections", async () =>
    {
        var directory = Path.Combine(Path.GetTempPath(), "forest-socket-budget-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(directory);
        try
        {
            using var runtime = new AuthorityRuntime(Path.Combine(directory, "world.json"), 41);
            await using var app = HostEntry.Build(runtime, "http://127.0.0.1:0", automaticTicks);
            await app.StartAsync();
            try
            {
                var address = app.Services.GetRequiredService<IServer>().Features.Get<IServerAddressesFeature>().Addresses.Single().Replace("http://", "ws://") + "/ws";
                using var client = new WorldClient(address, Path.Combine(directory, "client.json"));
                await client.ConnectAsync();
                int denied = 0;
                for (int i = 0; i < 35; i++)
                {
                    var before = client.LatestFrame;
                    var result = await client.SendAsync(new GameAction { Kind = "Presence" });
                    if (result.Success) Check(client.LatestFrame.Sequence > before.Sequence, "accepted action did not update the client frame");
                    else
                    {
                        Check(result.Error.StartsWith("Too many actions.", StringComparison.Ordinal), "burst failed for a reason other than its action budget");
                        denied++;
                        if (!automaticTicks) Check(ReferenceEquals(client.LatestFrame, before), "rate-limited request replaced the client's latest frame");
                    }
                }
                Check(denied >= 6, "socket burst did not exhaust the ordinary action budget");
                if (!automaticTicks)
                    Check(runtime.Project(null).Sequence == client.LatestFrame.Sequence + 1, "rejected requests built hidden full-world projections");
                else
                {
                    double before = client.LatestWorld.TimeSeconds;
                    var clock = System.Diagnostics.Stopwatch.StartNew();
                    while (client.LatestWorld.TimeSeconds <= before && clock.Elapsed < TimeSpan.FromSeconds(3)) await Task.Delay(100);
                    Check(client.LatestWorld.TimeSeconds > before && client.Status == "Connected", "throttling stopped scheduled snapshots or disconnected the client");
                }
                using var other = new WorldClient(address, Path.Combine(directory, "other.json"));
                await other.ConnectAsync();
                var previous = other.LatestFrame.Sequence;
                Check((await other.SendAsync(new GameAction { Kind = "Presence" })).Success && other.LatestFrame.Sequence > previous, "throttled peer blocked another player's accepted action");
            }
            finally { await app.StopAsync(); if (automaticTicks) await Task.Delay(150); }
        }
        finally { Directory.Delete(directory, true); }
    });
Test("live simulation resumes fractional saves without replaying elapsed work", () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-live-restart-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        var path = Path.Combine(directory, "world.json"); var whole = World.Create(41); var village = whole.Villages.Single(); var cat = village.Cats[1];
        var job = new Job { Id = whole.Id("job"), Kind = "woodcut", Phase = "working", Position = cat.Position, CatId = cat.Id, RequiredWork = 100 };
        village.Jobs.Add(job); cat.JobId = job.Id;
        whole.Step(.075); Check(job.Progress > 0 && job.Progress < 1, "live work did not advance before the first full second");
        AuthorityRuntime.ValidateWorld(whole); SaveStore.Save(path, whole);
        var resumed = SaveStore.Load<World>(path); AuthorityRuntime.ValidateWorld(resumed);
        Check(resumed.TimeSeconds == whole.TimeSeconds && resumed.SimulationTimeSeconds == whole.SimulationTimeSeconds, "save lost its consumed step or fractional remainder");
        whole.Step(.125); resumed.Step(.025); resumed.Step(.1);
        Check(JToken.DeepEquals(JToken.Parse(WireJson.Encode(whole)), JToken.Parse(WireJson.Encode(resumed))), "fractional restart changed timing, identities, work or finite ownership");
        var legacy = JObject.Parse(WireJson.Encode(whole)); legacy.Remove("SimulationTimeSeconds"); legacy.Remove("ContinuousClockInitialized");
        legacy["TimeSeconds"] = 43200.025; legacy["PendingSeconds"] = .025;
        SaveStore.Save(path, legacy); var oldSave = SaveStore.Load<World>(path);
        double before = oldSave.Villages.Single().Jobs.Single(j => j.Id == job.Id).Progress;
        oldSave.Step(.075); AuthorityRuntime.ValidateWorld(oldSave);
        var retained = oldSave.Villages.Single().Jobs.Single(j => j.Id == job.Id);
        Check(retained.CatId == cat.Id && retained.Progress >= before && retained.Progress - before < .2, "legacy save replayed elapsed work or replaced its carrier");
        SaveStore.Save(path, oldSave); var again = SaveStore.Load<World>(path);
        Check(again.ContinuousClockInitialized && again.SimulationTimeSeconds == oldSave.SimulationTimeSeconds, "upgraded fractional clock did not persist");
    }
    finally { Directory.Delete(directory, true); }
});
Test("physical defense survives need interruption projection restart and exact cancellation", () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-defense-restart-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        var path = Path.Combine(directory, "world.json");
        SessionCredential credential; string catId, raidId, jobId, armorId, expected; World uninterrupted;
        using (var runtime = new AuthorityRuntime(path, 41))
        {
            var owner = runtime.Connect(null, 1000); credential = owner.Credential;
            var world = runtime.World; var village = world.Villages.Single(); var cat = village.Cats[1]; var pile = village.Stockpiles[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            foreach (var other in village.Cats.Where(c => c.Id != cat.Id)) { other.ControlledBy = "fixture-observer"; other.ControlLeaseUntil = 10000; }
            cat.Position = pile.Position; cat.X = pile.Position.X; cat.Z = pile.Position.Z; cat.BuildingId = ""; cat.Path.Clear();
            // Keep finite supplies above the founding Leader's emergency floors,
            // so this case exercises need/restart recovery rather than preemption.
            pile.Goods.Clear(); World.Add(pile.Goods, "water", 400); World.Add(pile.Goods, "food", 400); World.Add(pile.Goods, "logs", 7);
            var armor = new Item { Id = world.Id("defense-armor"), Kind = "armor", Material = "leather", VillageId = village.Id, LocationId = cat.Id, Condition = 17, MaxCondition = 42, Quality = 3 };
            village.Items.Add(armor); cat.Equipment.Add(armor.Id);
            var raid = new Raid { Id = world.Id("defense-raid"), Position = cat.Position, Health = 100 };
            raid.Loot.Add(new Stack("logs", 3)); village.Raids.Add(raid);
            var order = runtime.Apply(owner, new GameAction { Kind = "DefendRaid", CatId = cat.Id, TargetId = raid.Id });
            Check(order.Success && raid.Health == 100, "defense action failed or dealt damage before actor time");
            var job = village.Jobs.Single(j => j.Id == order.EntityId);
            Check(runtime.Apply(owner, new GameAction { Kind = "DefendRaid", CatId = cat.Id, TargetId = raid.Id }).EntityId == job.Id && village.Jobs.Count(j => j.Kind == "defend_raid") == 1, "repeated signed order duplicated the defender");
            runtime.Advance(.075);
            Check(job.Progress > 0 && raid.Health < 100 && armor.Condition < 17, "physical combat did not spend time and equipment condition");
            cat.Thirst = 34; runtime.Advance(.05);
            Check(cat.Goal == "need_drink" && cat.ResumeJobId == job.Id && !job.Completed, "defender did not suspend its exact order for a real need");
            var canonical = WireJson.Encode(world); var visible = runtime.Project(owner).World.Villages.Single();
            var projectedCat = visible.Cats.Single(c => c.Id == cat.Id); var projectedJob = visible.Jobs.Single(j => j.Id == job.Id);
            Check(projectedCat.ResumeJobId == job.Id && projectedJob.CatId == cat.Id && projectedJob.TargetId == raid.Id && projectedJob.Manual && !projectedJob.Completed, "authorized projection lost actionable suspended-defense ownership");
            Check(WireJson.Encode(world) == canonical && village.Items.Single(i => i.Id == armor.Id).LocationId == cat.Id, "projection mutated real equipment or work ownership");
            catId = cat.Id; raidId = raid.Id; jobId = job.Id; armorId = armor.Id;
            runtime.Save(); expected = WireJson.Encode(world); uninterrupted = WireJson.Clone(world);
        }
        using (var runtime = new AuthorityRuntime(path, 999))
        {
            var owner = runtime.Connect(credential, 1001); var world = runtime.World; var village = world.Villages.Single();
            Check(WireJson.Encode(world) == expected, "restart changed the durable full defense and need state");
            var cat = village.Cats.Single(c => c.Id == catId); var job = village.Jobs.Single(j => j.Id == jobId); var raid = village.Raids.Single(r => r.Id == raidId); var armor = village.Items.Single(i => i.Id == armorId);
            double before = job.Progress, condition = armor.Condition;
            var repeated = runtime.Apply(owner, new GameAction { Kind = "DefendRaid", CatId = catId, TargetId = raidId });
            Check(repeated.Success && repeated.EntityId == jobId && village.Jobs.Count(j => !j.Completed && j.Kind == "defend_raid") == 1, "restart made a suspended order claimable twice");
            runtime.Advance(.025); runtime.Advance(.025); uninterrupted.Step(.05);
            Check(cat.Goal == "need_drink" && job.Progress == before && armor.Condition == condition, "need travel also spent combat time or equipment");
            for (int i = 0; i < 120 && job.Progress == before; i++)
            { runtime.Advance(.25); runtime.Advance(.25); uninterrupted.Step(.5); }
            Check(cat.Goal == "defending" && cat.ResumeJobId == "" && cat.JobId == jobId && job.Progress > before && raid.Health < 100 && armor.Condition < condition,
                "restarted need did not physically resume its original combat target; goal=" + cat.Goal + " resume=" + cat.ResumeJobId + " job=" + cat.JobId + " progress=" + job.Progress + " before=" + before + " completed=" + job.Completed + " cat=" + cat.X + "," + cat.Z + " raid=" + raid.X + "," + raid.Z + " raid_present=" + village.Raids.Contains(raid) + " thirst=" + cat.Thirst + " cargo=" + string.Join(",", cat.Cargo.Select(s => s.Resource + ":" + s.Amount)) + " blocked=" + cat.BlockedReason);
            Check(JToken.DeepEquals(JToken.Parse(WireJson.Encode(world)), JToken.Parse(WireJson.Encode(uninterrupted))), "restart or elapsed partitions changed defense, needs, wear or identities");
            Check(village.Stockpiles.Sum(p => World.Amount(p.Goods, "water")) == 399 && cat.Cargo.Count == 0, "need recovery lost or duplicated its finite water serving");
            Check(village.Stockpiles.Sum(p => World.Amount(p.Goods, "logs")) + village.Cats.Sum(c => World.Amount(c.Cargo, "logs")) + village.Raids.Sum(r => World.Amount(r.Loot, "logs")) == 10, "defense/restart created or lost finite goods");
            Check(!runtime.Apply(owner, new GameAction { Kind = "CancelDefense", CatId = village.Cats[2].Id, TargetId = jobId }).Success && !job.Completed, "restarted cancellation ignored the exact owner");
            Check(runtime.Apply(owner, new GameAction { Kind = "CancelDefense", CatId = catId, TargetId = jobId }).Success && job.Completed && cat.JobId == "" && cat.ResumeJobId == "", "restarted manual defense could not release its exact ownership");
            Check(village.Items.Count(i => i.Id == armorId) == 1 && armor.LocationId == catId && cat.Equipment.Contains(armorId) && world.Reservations.Count == 0, "cancellation duplicated equipment or retained resource claims");
            runtime.Save();
        }
        using (var runtime = new AuthorityRuntime(path, 123))
        {
            var owner = runtime.Connect(credential, 1002); var village = runtime.World.Villages.Single(); var cat = village.Cats.Single(c => c.Id == catId);
            double health = village.Raids.Single(r => r.Id == raidId).Health;
            Check(!runtime.Apply(owner, new GameAction { Kind = "CancelDefense", CatId = catId, TargetId = jobId }).Success, "completed cancellation replayed after restart");
            runtime.Advance(.05); runtime.Save();
            Check(cat.JobId == "" && cat.ResumeJobId == "" && village.Jobs.Single(j => j.Id == jobId).Completed && village.Raids.Single(r => r.Id == raidId).Health == health, "cancelled defender resumed combat after another restart");
            AuthorityRuntime.ValidateWorld(SaveStore.Load<World>(path));
        }
    }
    finally { Directory.Delete(directory, true); }
});
Test("failed path retry cache survives fractional save and resumes once", () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-path-retry-restart-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        var path = Path.Combine(directory, "world.json"); var whole = World.Create(41); var village = whole.Villages.Single(); var cat = village.Cats[1];
        foreach (var other in village.Cats.Where(c => c.Id != cat.Id)) { other.ControlledBy = "fixture-observer"; other.ControlLeaseUntil = 10000; }
        var start = new Int2(20, 20); var destination = new Int2(21, 20);
        cat.Position = start; cat.X = start.X; cat.Z = start.Z; cat.BuildingId = ""; cat.Path.Clear();
        foreach (var offset in new[] { new Int2(1, 0), new Int2(-1, 0), new Int2(0, 1), new Int2(0, -1) })
        {
            var neighbor = new Int2(start.X + offset.X, start.Z + offset.Z); var tile = whole.TileAt(neighbor);
            tile.Wall = tile.Water = tile.Mountain = tile.Road = tile.Dirt = false; tile.Biome = "grass";
            village.BoundaryEdges.Add(new BoundaryEdge { From = start, To = neighbor });
        }
        var origin = whole.TileAt(start); origin.Wall = origin.Water = origin.Mountain = false;
        var job = new Job { Id = whole.Id("job"), Kind = "woodcut", Phase = "travel", Position = destination, CatId = cat.Id, RequiredWork = 1000 };
        village.Jobs.Add(job); cat.JobId = job.Id; cat.Cargo.Add(new Stack("logs", 8));
        whole.Step(.075);
        Check(cat.HasFailedPath && cat.FailedPathStart.Equals(start) && cat.FailedPathDestination.Equals(destination) && cat.NextPathAttemptAt == 1, "blocked movement did not record its retry boundary");
        Check(cat.Position.Equals(start) && job.Progress == 0, "blocked work moved or advanced before saving");
        AuthorityRuntime.ValidateWorld(whole); SaveStore.Save(path, whole);
        var resumed = SaveStore.Load<World>(path); AuthorityRuntime.ValidateWorld(resumed);
        void SameState() => Check(JToken.DeepEquals(JToken.Parse(WireJson.Encode(whole)), JToken.Parse(WireJson.Encode(resumed))), "restart or elapsed partitions changed retry state, work, cargo or identities");
        SameState();
        whole.Step(.325); resumed.Step(.125); resumed.Step(.2); SameState();
        foreach (var world in new[] { whole, resumed }) world.Villages.Single().BoundaryEdges.RemoveAll(edge => edge.From.Equals(start));
        whole.Step(.575); resumed.Step(.2); resumed.Step(.375); SameState();
        Check(cat.HasFailedPath && cat.Position.Equals(start) && cat.X == start.X && job.Progress == 0, "cleared route bypassed the persisted retry deadline");
        whole.Step(.025); resumed.Step(.025); SameState();
        Check(!cat.HasFailedPath && cat.X > start.X && cat.X < destination.X && job.Progress == 0, "retry boundary did not resume fractional physical movement");
        whole.Step(1.025); resumed.Step(.425); resumed.Step(.6); SameState();
        Check(cat.Position.Equals(destination) && job.Progress > 0 && !job.Completed && cat.JobId == job.Id && job.CatId == cat.Id && World.Amount(cat.Cargo, "logs") == 8, "recovered route lost its work owner or finite cargo");
        AuthorityRuntime.ValidateWorld(resumed); SaveStore.Save(path, resumed); resumed = SaveStore.Load<World>(path);
        whole.Step(.975); resumed.Step(.475); resumed.Step(.5); SameState(); AuthorityRuntime.ValidateWorld(resumed);
        Check(resumed.Villages.Single().Jobs.Count(j => j.Id == job.Id) == 1, "second restart duplicated the recovered job");
    }
    finally { Directory.Delete(directory, true); }
});
foreach (bool exactPayment in new[] { false, true })
    Test("blocked caravan unload retry survives restart " + (exactPayment ? "exact" : "scalar"), () =>
    {
        var directory = Path.Combine(Path.GetTempPath(), "forest-unload-retry-" + Guid.NewGuid().ToString("N")); Directory.CreateDirectory(directory);
        try
        {
            var whole = new World { Seed = 41 };
            var source = new Village { Id = "unload-source", Center = new Int2(0, 0), Communal = true };
            var target = new Village { Id = "unload-target", Center = new Int2(10, 0), OwnerId = "unload-owner" };
            whole.Villages.AddRange(new[] { source, target });
            for (int x = -1; x <= 3; x++) for (int z = -1; z <= 1; z++)
                whole.Tiles.Add(new Tile { Position = new Int2(x, z), Biome = "grass", Wall = x == -1 || x == 3 || z != 0 });
            whole.Tiles.Add(new Tile { Position = target.Center, Biome = "grass" });
            source.Stockpiles.Add(new Stockpile { Id = "unload-receiving", Position = new Int2(2, 0), Kind = "storage", Capacity = 100 });
            target.Stockpiles.Add(new Stockpile { Id = "unload-peer-store", Position = target.Center, Kind = "storage", Capacity = 100 });
            source.BoundaryEdges.Add(new BoundaryEdge { From = new Int2(1, 0), To = new Int2(2, 0) });
            // Explicit persisted state: the caravan has returned, but its receiving store is fenced off.
            var trade = new TradeOffer { Id = "unload-trade", FromVillageId = source.Id, ToVillageId = target.Id, Status = "unloading", Position = source.Center, HasContinuousPosition = true, Offered = new Stack("logs", 3), Requested = new Stack(exactPayment ? "tools" : "gem", exactPayment ? 1 : 2) };
            if (exactPayment) trade.RequestedItems.Add(new Item { Id = "unload-exact-tool", Kind = "tool", Material = "wood", Quality = 3, Condition = 17, MaxCondition = 42, VillageId = target.Id, LocationId = trade.Id });
            whole.TradeOffers.Add(trade);
            long Searches(World world) => (long)typeof(World).GetField("pathSearchCount", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic)!.GetValue(world)!;
            whole.Step(.075); Check(Searches(whole) == 0, "waiting caravan searched storage between planning boundaries");
            var path = Path.Combine(directory, "world.json"); AuthorityRuntime.ValidateWorld(whole); SaveStore.Save(path, whole); var resumed = SaveStore.Load<World>(path);
            void SameState() => Check(JToken.DeepEquals(JToken.Parse(WireJson.Encode(whole)), JToken.Parse(WireJson.Encode(resumed))), "unloading retry changed authoritative state across partitions/restart");
            whole.Step(.925); resumed.Step(.425); resumed.Step(.5); SameState();
            Check(Searches(whole) == 1 && Searches(resumed) == 1 && trade.Status == "unloading", "unreachable storage did not get one planning search");
            whole.Step(.95); resumed.Step(.45); resumed.Step(.5); SameState();
            Check(Searches(whole) == 1 && Searches(resumed) == 1 && source.Stockpiles[0].Goods.Count == 0 && target.Stockpiles[0].Goods.Count == 0 && source.Items.Count == 0, "blocked unload repeated searches or credited escrow");
            source.BoundaryEdges.Clear(); resumed.Villages[0].BoundaryEdges.Clear();
            whole.Step(.05); resumed.Step(.025); resumed.Step(.025); SameState();
            Check(trade.Status == "completed" && trade.OfferedDelivered && World.Amount(target.Stockpiles[0].Goods, "logs") == 3, "cleared unload failed exactly-once delivery");
            if (exactPayment)
            {
                var item = source.Items.Single(); Check(item.Id == "unload-exact-tool" && item.Condition == 17 && item.Quality == 3 && item.LocationId == source.Stockpiles[0].Id && item.VillageId == source.Id && trade.RequestedItems.Count == 0, "unload changed exact payment identity or ownership");
            }
            else Check(World.Amount(source.Stockpiles[0].Goods, "gem") == 2, "unload changed scalar payment");
            SaveStore.Save(path, resumed); resumed = SaveStore.Load<World>(path); whole.Step(1.075); resumed.Step(.475); resumed.Step(.6); SameState(); AuthorityRuntime.ValidateWorld(resumed);
            Check(World.Amount(target.Stockpiles[0].Goods, "logs") == 3 && (exactPayment ? source.Items.Count == 1 : World.Amount(source.Stockpiles[0].Goods, "gem") == 2), "completed unload replayed after restart");
        }
        finally { Directory.Delete(directory, true); }
    });
Test("scalar haul retargets blocked destination across restart", () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-scalar-retarget-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        var path = Path.Combine(directory, "world.json"); var world = World.Create(41); var v = world.Villages.Single();
        v.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine"); v.Stockpiles.RemoveRange(1, v.Stockpiles.Count - 1); v.Stockpiles[0].Accepts = new List<string> { "food", "water" };
        var cat = v.Cats.First(c => c.Id != v.LeaderId); cat.Position = new Int2(1, 2); cat.X = 1; cat.Z = 2;
        foreach (var other in v.Cats.Where(c => c.Id != cat.Id)) { other.ControlledBy = "fixture-observer"; other.ControlLeaseUntil = 10000; }
        var source = new Stockpile { Id = world.Id("source-store"), Position = new Int2(-5, 1), Width = 1, Depth = 1, Capacity = 100, Accepts = new List<string> { "logs" }, Goods = new List<Stack> { new Stack("logs", 8) } };
        var blocked = new Stockpile { Id = world.Id("original-store"), Position = new Int2(5, 1), Width = 1, Depth = 1, Capacity = 8, Accepts = new List<string> { "logs" } };
        var alternate = new Stockpile { Id = world.Id("alternate-store"), Position = new Int2(5, 4), Width = 1, Depth = 1, Capacity = 8, Accepts = new List<string> { "logs" } };
        v.Stockpiles.Add(source); v.Stockpiles.Add(blocked); v.Stockpiles.Add(alternate);
        var context = new PlayerContext { PlayerId = "fixture-scalar-hauler", VillageId = v.Id }; string catId = cat.Id, sourceId = source.Id, blockedId = blocked.Id, alternateId = alternate.Id;
        var result = world.Apply(context, new GameAction { Kind = "HaulGatherSpot", CatId = cat.Id, TargetId = source.Id }); Check(result.Success, "scalar haul rejected: " + result.Error); string jobId = result.EntityId; var job = v.Jobs.Single(j => j.Id == jobId);
        Check(job.TargetId == blockedId && job.Amount == 8, "haul did not choose the initial nearer store");
        for (int tick = 0; tick < 100 && job.Phase != "input_delivery"; tick++) world.Step(1);
        Check(job.Phase == "input_delivery" && World.Amount(cat.Cargo, "logs") == 8 && World.Amount(source.Goods, "logs") == 0, "scalar haul did not enter its carried input_delivery phase");
        result = world.Apply(context, new GameAction { Kind = "CreateZone", Resource = "avoid", Position = blocked.Position, End = blocked.Position }); Check(result.Success, "public original-store obstruction failed: " + result.Error); string zoneId = result.EntityId;
        void Restart()
        {
            AuthorityRuntime.ValidateWorld(world); SaveStore.Save(path, world); world = SaveStore.Load<World>(path); AuthorityRuntime.ValidateWorld(world);
            v = world.Villages.Single(); cat = v.Cats.Single(c => c.Id == catId); job = v.Jobs.Single(j => j.Id == jobId); source = v.Stockpiles.Single(p => p.Id == sourceId); blocked = v.Stockpiles.Single(p => p.Id == blockedId); alternate = v.Stockpiles.Single(p => p.Id == alternateId);
        }
        Restart();
        Check(job.Phase == "input_delivery" && job.TargetId == blockedId && job.Position.Equals(blocked.Position) && cat.JobId == jobId && World.Amount(cat.Cargo, "logs") == 8, "restart did not retain the already-saved original input-delivery target and cargo");
        for (int tick = 0; tick < 100 && !job.Completed; tick++)
        {
            world.Step(1); Check(v.Stockpiles.Any(p => p.Id == zoneId) && !world.Walkable(v, blocked.Position), "recovery removed the saved obstruction");
            Check(job.CatId == catId && (job.Completed || cat.JobId == jobId), "recovery replaced the saved carrier or work owner");
            Check(World.Amount(source.Goods, "logs") == 0 && World.Amount(blocked.Goods, "logs") == 0, "recovery returned cargo to source or delivered through the obstruction");
            Check(v.Stockpiles.Sum(p => World.Amount(p.Goods, "logs")) + v.Cats.Sum(c => World.Amount(c.Cargo, "logs")) + v.Jobs.Where(j => !j.Completed).Sum(j => World.Amount(j.Local, "logs")) == 8, "restarted retargeting changed finite scalar ownership");
        }
        Check(job.Completed && job.TargetId == alternateId && job.Position.Equals(alternate.Position) && cat.Position.Equals(alternate.Position) && cat.Cargo.Count == 0 && World.Amount(alternate.Goods, "logs") == 8, "saved scalar input_delivery never recovered to the reachable alternate store");
        Restart(); world.Step(2);
        Check(v.Jobs.Count(j => j.Id == jobId) == 1 && job.Completed && cat.JobId == "" && World.Amount(alternate.Goods, "logs") == 8 && World.Amount(source.Goods, "logs") == 0 && World.Amount(blocked.Goods, "logs") == 0 && world.Reservations.Count == 0, "completed scalar recovery replayed or retained stale claims after another restart");
    }
    finally { Directory.Delete(directory, true); }
});
Test("manual production retargets closed ingredient claims across cargo restarts", () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-production-claims-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        var path = Path.Combine(directory, "world.json"); var world = World.Create(41); var village = world.Villages.Single();
        village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
        village.Stockpiles.Clear(); village.LastLeaderResearch = world.TimeSeconds;
        var worker = village.Cats.First(c => c.Id != village.LeaderId);
        foreach (var cat in village.Cats)
        {
            cat.ControlledBy = cat.Id == worker.Id ? "" : "fixture-observer"; cat.ControlLeaseUntil = 100000;
            cat.Position = new Int2(1, 3); cat.X = 1; cat.Z = 3; cat.Path.Clear();
        }
        var store = new Stockpile { Id = "production-receiving-store", Position = new Int2(3, 2), Width = 1, Depth = 1, Capacity = 10000, Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000) } };
        Stockpile Supply(string id, string resource, double amount, Int2 position)
        {
            var pile = new Stockpile { Id = id, Position = position, Width = 1, Depth = 1, Capacity = 100, Accepts = new List<string> { resource }, Goods = new List<Stack> { new Stack(resource, amount) } };
            village.Stockpiles.Add(pile); return pile;
        }
        village.Stockpiles.Add(store);
        var blocked = Supply("aaa-inaccessible-planks", "planks", 2, new Int2(-5, 1));
        var first = Supply("bbb-original-planks", "planks", 2, new Int2(3, 3));
        var alternate = Supply("ccc-alternate-planks", "planks", 2, new Int2(-1, 3));
        var blocks = Supply("ddd-finite-blocks", "blocks", 2, new Int2(3, 5));
        world.TileAt(blocked.Position).Wall = true;
        var station = new Building { Id = world.Id("production-restart-station"), Kind = "woodworking", Completed = true, Position = worker.Position };
        village.Buildings.Add(station);
        var context = new PlayerContext { PlayerId = "fixture-production-restart", VillageId = village.Id };
        void Apply(GameAction action) { var result = world.Apply(context, action); Check(result.Success, action.Kind + " rejected: " + result.Error); }
        Apply(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = worker.Id });
        Apply(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "add", RecipeId = "planks_and_blocks_to_tools" });
        world.Step(1.05);
        var job = village.Jobs.Single(j => j.Kind == "production" && !j.Completed && j.CatId == worker.Id);
        string jobId = job.Id, catId = worker.Id, stationId = station.Id;
        string queued = WireJson.Encode(station.Slots.Single().Queue);
        Check(world.Reservations.Any(r => r.OwnerId == jobId && r.PileId == first.Id && r.Amount == 2) && world.Reservations.Any(r => r.OwnerId == jobId && r.PileId == blocks.Id && r.Amount == 2), "manual recipe did not claim its reachable finite ingredient piles");
        Check(!world.Reservations.Any(r => r.OwnerId == jobId && r.PileId == blocked.Id) && worker.Cargo.Count == 0, "recipe claimed inaccessible goods or moved inputs before pickup");
        void Restart()
        {
            AuthorityRuntime.ValidateWorld(world); SaveStore.Save(path, world); string expected = WireJson.Encode(world);
            world = SaveStore.Load<World>(path); Check(WireJson.Encode(world) == expected, "restart changed complete production claims, work, cargo or queue state");
            AuthorityRuntime.ValidateWorld(world); village = world.Villages.Single(); worker = village.Cats.Single(c => c.Id == catId); job = village.Jobs.Single(j => j.Id == jobId); station = village.Buildings.Single(b => b.Id == stationId);
            blocked = village.Stockpiles.Single(p => p.Id == blocked.Id); first = village.Stockpiles.Single(p => p.Id == first.Id); alternate = village.Stockpiles.Single(p => p.Id == alternate.Id); blocks = village.Stockpiles.Single(p => p.Id == blocks.Id); store = village.Stockpiles.Single(p => p.Id == store.Id);
        }
        void CheckUnconsumed()
        {
            Check(worker.JobId == jobId && job.CatId == catId && station.Slots.Single().CatId == catId && WireJson.Encode(station.Slots.Single().Queue) == queued, "ingredient recovery replaced the player's worker, job, position or finite queue");
            foreach (var material in new[] { (Name: "planks", Amount: 6.0), (Name: "blocks", Amount: 2.0) })
                Check(village.Stockpiles.Sum(p => World.Amount(p.Goods, material.Name)) + village.Cats.Sum(c => World.Amount(c.Cargo, material.Name)) + village.Jobs.Where(j => !j.Completed).Sum(j => World.Amount(j.Local, material.Name)) + village.Buildings.Sum(b => World.Amount(b.Inputs, material.Name)) == material.Amount, "production recovery lost or duplicated finite " + material.Name);
        }
        Restart(); CheckUnconsumed();
        world.TileAt(first.Position).Wall = true;
        world.Step(1);
        Check(!world.Reservations.Any(r => r.OwnerId == jobId && (r.PileId == blocked.Id || r.PileId == first.Id)) && world.Reservations.Any(r => r.OwnerId == jobId && r.PileId == alternate.Id && r.Amount == 2), "saved claim did not recover to the remaining reachable planks");
        CheckUnconsumed(); Restart();
        for (int tick = 0; tick < 200 && World.Amount(worker.Cargo, "planks") == 0; tick++) { world.Step(.05); CheckUnconsumed(); }
        Check(World.Amount(worker.Cargo, "planks") == 2 && World.Amount(alternate.Goods, "planks") == 0 && World.Amount(job.Local, "planks") == 0, "replacement source did not require a real exclusive pickup before delivery");
        Restart(); CheckUnconsumed();
        for (int tick = 0; tick < 300 && World.Amount(job.Local, "planks") != 2; tick++) { world.Step(.05); CheckUnconsumed(); }
        Check(World.Amount(job.Local, "planks") == 2 && !world.Reservations.Any(r => r.OwnerId == jobId && r.Resource == "planks"), "restarted carrier did not stage its exact planks at the original workshop");
        Restart(); CheckUnconsumed();
        for (int second = 0; second < 900 && !job.Completed; second++) world.Step(1);
        Check(job.Completed && worker.JobId == "" && worker.Cargo.Count == 0 && job.Local.Count == 0 && world.Reservations.All(r => r.OwnerId != jobId), "finite restarted recipe failed to finish or release its cargo and claims");
        var tool = village.Items.Single(i => i.Kind == "tool"); string toolId = tool.Id;
        Check(tool.Material == "wood" && tool.LocationId == store.Id && tool.Condition == tool.MaxCondition && World.Amount(store.Goods, "tools") == 0, "recipe did not deliver exactly one intact physical wooden tool");
        Check(World.Amount(blocked.Goods, "planks") == 2 && World.Amount(first.Goods, "planks") == 2 && world.Total(village, "planks") == 4 && world.Total(village, "blocks") == 0 && station.Slots.Single().Queue.Count == 0, "completion changed inaccessible goods or consumed more than the manual recipe bill");
        Restart(); world.Step(2);
        Check(village.Items.Count(i => i.Id == toolId) == 1 && village.Items.Single(i => i.Id == toolId).LocationId == store.Id && village.Jobs.Count(j => j.Id == jobId) == 1 && job.Completed && station.Slots.Single().Queue.Count == 0 && world.Reservations.Count == 0 && world.Total(village, "planks") == 4 && world.Total(village, "blocks") == 0, "completed production replayed or duplicated output after the final restart");
        AuthorityRuntime.ValidateWorld(world);
    }
    finally { Directory.Delete(directory, true); }
});
Test("officer reachable recipe choice and finite textile chain survive durable restarts", () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-officer-inputs-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        var path = Path.Combine(directory, "world.json"); var world = World.Create(41); var village = world.Villages.Single();
        village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
        village.Stockpiles.Clear(); village.LastLeaderResearch = world.TimeSeconds;
        var worker = village.Cats.First(c => c.Id != village.LeaderId); string workerId = worker.Id;
        foreach (var cat in village.Cats)
        {
            cat.ControlledBy = cat.Id == workerId ? "" : "fixture-observer"; cat.ControlLeaseUntil = 100000;
            cat.Position = new Int2(1, 3); cat.X = 1; cat.Z = 3; cat.Path.Clear();
        }
        var store = new Stockpile { Id = "officer-receiving-store", Position = new Int2(3, 2), Width = 1, Depth = 1, Capacity = 10000, Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000), new Stack("fibre", 5) } };
        var blocked = new Stockpile { Id = "aaa-officer-blocked-thread", Position = new Int2(-3, 2), Width = 1, Depth = 1, Capacity = 100, Goods = new List<Stack> { new Stack("thread", 5) } };
        village.Stockpiles.Add(store); village.Stockpiles.Add(blocked); world.TileAt(blocked.Position).Wall = true;
        var station = new Building { Id = world.Id("officer-restart-station"), Kind = "clothier", Completed = true, Position = worker.Position }; string stationId = station.Id;
        village.Buildings.Add(station); village.Research.Add("textiles");
        var context = new PlayerContext { PlayerId = "fixture-officer-restart", VillageId = village.Id };
        void Apply(GameAction action) { var result = world.Apply(context, action); Check(result.Success, action.Kind + " rejected: " + result.Error); }
        Apply(new GameAction { Kind = "AssignOfficer", Role = "cloth_leader", CatId = village.LeaderId });
        Apply(new GameAction { Kind = "AssignWorker", BuildingId = stationId, CatId = workerId });
        station.Slots.Single().AutomatedBy = "cloth_leader";
        station.Slots.Single().Queue.Add(new QueueEntry { RecipeId = "fibre_to_cloth", AutomatedBy = "cloth_leader" });
        void Restart()
        {
            AuthorityRuntime.ValidateWorld(world); SaveStore.Save(path, world); string expected = WireJson.Encode(world);
            world = SaveStore.Load<World>(path); Check(WireJson.Encode(world) == expected, "durable restart changed officer work, exact claims or inventory");
            AuthorityRuntime.ValidateWorld(world); village = world.Villages.Single(); worker = village.Cats.Single(c => c.Id == workerId); station = village.Buildings.Single(b => b.Id == stationId);
            store = village.Stockpiles.Single(p => p.Id == "officer-receiving-store"); blocked = village.Stockpiles.Single(p => p.Id == "aaa-officer-blocked-thread");
        }
        Restart(); world.Step(11.05);
        var job = village.Jobs.Single(j => !j.Completed && j.Kind == "production" && j.CatId == workerId); string jobId = job.Id;
        Check(job.RecipeId == "fibre_to_thread" && station.Slots.Single().Queue.Single().AutomatedBy == "cloth_leader", "restored stale automatic queue did not choose the reachable physical ingredient");
        Check(World.Amount(blocked.Goods, "thread") == 5 && world.Reservations.All(r => r.PileId != blocked.Id), "automatic choice claimed inaccessible thread");
        Restart();
        for (int tick = 0; tick < 400 && World.Amount(worker.Cargo, "fibre") == 0; tick++) world.Step(.05);
        Check(World.Amount(worker.Cargo, "fibre") == 5 && World.Amount(store.Goods, "fibre") == 0, "restored worker did not physically collect its finite fibre");
        Restart(); world.Step(1600);
        Check(world.Total(village, "fibre") == 0 && world.Total(village, "thread") == 5 && world.Total(village, "cloth") == 1, "finite reachable fibre did not become five Thread and then one Cloth while blocked Thread stayed intact");
        Check(World.Amount(blocked.Goods, "thread") == 5 && worker.Cargo.Count == 0 && world.Reservations.Count == 0 && !village.Jobs.Any(j => !j.Completed && j.Kind == "production"), "completed finite chain retained cargo, claims or an active recipe");
        Check(village.Events.Any(e => e.Kind == "completed" && e.EntityId == jobId), "physical production did not retain completion evidence");
        Restart(); world.Step(20);
        Check(world.Total(village, "thread") == 5 && world.Total(village, "cloth") == 1 && world.Reservations.Count == 0, "restart replayed the finite textile chain or claimed blocked Thread");
        AuthorityRuntime.ValidateWorld(world);
    }
    finally { Directory.Delete(directory, true); }
});
Test("officer delegation preserves exact production ownership and manual queue across durable restarts", () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-workshop-delegation-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        var path = Path.Combine(directory, "world.json"); var world = World.Create(41); var village = world.Villages.Single();
        village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
        village.Stockpiles.Clear(); village.LastLeaderResearch = world.TimeSeconds;
        var worker = village.Cats.First(c => c.Id != village.LeaderId); string workerId = worker.Id;
        foreach (var cat in village.Cats)
        {
            cat.ControlledBy = cat.Id == workerId ? "" : "fixture-observer"; cat.ControlLeaseUntil = 100000;
            cat.Position = new Int2(1, 3); cat.X = 1; cat.Z = 3; cat.Path.Clear();
        }
        var store = new Stockpile { Id = "delegation-receiving-store", Position = new Int2(3, 2), Width = 1, Depth = 1, Capacity = 10000, Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000) } };
        village.Stockpiles.Add(store);
        village.Stockpiles.Add(new Stockpile { Id = "delegation-finite-planks", Position = new Int2(3, 3), Width = 1, Depth = 1, Capacity = 100, Accepts = new List<string> { "planks" }, Goods = new List<Stack> { new Stack("planks", 2) } });
        village.Stockpiles.Add(new Stockpile { Id = "delegation-finite-blocks", Position = new Int2(3, 5), Width = 1, Depth = 1, Capacity = 100, Accepts = new List<string> { "blocks" }, Goods = new List<Stack> { new Stack("blocks", 2) } });
        var station = new Building { Id = world.Id("delegation-station"), Kind = "woodworking", Completed = true, Position = worker.Position }; string stationId = station.Id;
        village.Buildings.Add(station);
        village.Buildings.Add(new Building { Id = world.Id("delegation-office"), Kind = "sawmill", Completed = true, Position = new Int2(-6, 0) });
        village.Research.Add("sawmill");
        var context = new PlayerContext { PlayerId = "fixture-workshop-delegation", VillageId = village.Id };
        void Apply(GameAction action) { var result = world.Apply(context, action); Check(result.Success, action.Kind + " rejected: " + result.Error); }
        GameAction Edit(string edit, bool enabled = true) => new GameAction { Kind = "EditProductionWorkSlot", BuildingId = stationId, CatId = workerId, Edit = edit, Enabled = enabled };
        Apply(new GameAction { Kind = "AssignOfficer", Role = "forester", CatId = village.LeaderId });
        Apply(new GameAction { Kind = "AssignWorker", BuildingId = stationId, CatId = workerId });
        Apply(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = stationId, CatId = workerId, Edit = "add", RecipeId = "planks_and_blocks_to_tools" });
        Apply(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = stationId, CatId = workerId, Edit = "add", RecipeId = "planks_and_blocks_to_tools", Repeat = true });
        string manualQueue = WireJson.Encode(station.Slots.Single().Queue);
        string jobId = "";
        Job Job() => village.Jobs.Single(j => j.Id == jobId);
        void Restart()
        {
            AuthorityRuntime.ValidateWorld(world); SaveStore.Save(path, world); string expected = WireJson.Encode(world);
            world = SaveStore.Load<World>(path); Check(WireJson.Encode(world) == expected, "durable restart changed delegated production, exact claims, cargo, pause or queue state");
            AuthorityRuntime.ValidateWorld(world); village = world.Villages.Single(); worker = village.Cats.Single(c => c.Id == workerId); station = village.Buildings.Single(b => b.Id == stationId);
            store = village.Stockpiles.Single(p => p.Id == "delegation-receiving-store");
        }
        void CheckUnconsumed()
        {
            Check(worker.JobId == jobId && Job().CatId == workerId && station.Slots.Single().CatId == workerId && WireJson.Encode(station.Slots.Single().Queue) == manualQueue, "delegation or restart replaced the exact job, worker or deliberate manual orders");
            foreach (string material in new[] { "planks", "blocks" })
                Check(village.Stockpiles.Sum(p => World.Amount(p.Goods, material)) + village.Cats.Sum(c => World.Amount(c.Cargo, material)) + village.Jobs.Where(j => !j.Completed).Sum(j => World.Amount(j.Local, material)) + village.Buildings.Sum(b => World.Amount(b.Inputs, material)) == 2, "delegation lost or duplicated unconsumed " + material);
        }
        void DelegateUnchanged()
        {
            var expected = WireJson.Decode<World>(WireJson.Encode(world));
            expected.Villages.Single().Buildings.Single(b => b.Id == stationId).Slots.Single().AutomatedBy = "forester";
            Apply(Edit("delegate"));
            Check(JToken.DeepEquals(JToken.Parse(WireJson.Encode(world)), JToken.Parse(WireJson.Encode(expected))), "delegation changed more than the exact slot's officer provenance");
        }
        void Reach(string phase)
        {
            for (int tick = 0; tick < 4000; tick++)
            {
                world.Step(.05);
                var active = village.Jobs.Find(j => !j.Completed && j.Kind == "production" && j.CatId == workerId);
                if (active == null) continue;
                if (jobId == "") jobId = active.Id;
                Check(active.Id == jobId, "production changed identity between ownership checkpoints");
                CheckUnconsumed();
                if (phase == "claimed" ? world.Reservations.Any(r => r.OwnerId == jobId) : phase == "carried" ? World.Amount(worker.Cargo, "planks") == 2 : phase == "staged" ? World.Amount(active.Local, "planks") == 2 : active.Phase == "working" && active.Progress > 0)
                    return;
            }
            throw new Exception("delegated production did not reach its " + phase + " checkpoint");
        }
        foreach (string phase in new[] { "claimed", "carried", "staged", "working" })
        {
            Reach(phase);
            Apply(Edit("pause"));
            DelegateUnchanged(); CheckUnconsumed(); Restart(); CheckUnconsumed();
            Check(station.Slots.Single().Paused && station.Slots.Single().AutomatedBy == "forester", "delegation restart lost a deliberate pause or officer handback during " + phase);
            var heldJob = WireJson.Decode<Job>(WireJson.Encode(Job()));
            string heldCargo = WireJson.Encode(worker.Cargo), heldClaims = WireJson.Encode(world.Reservations), heldPath = WireJson.Encode(worker.Path);
            var heldPosition = worker.Position; double heldX = worker.X, heldZ = worker.Z;
            world.Step(.2);
            Check(Job().BlockedReason == "queue_paused" && Job().Phase == heldJob.Phase && Job().Progress == heldJob.Progress, "paused delegated work lost its diagnostic or advanced its phase/progress after restart during " + phase);
            // Paused ticks still refresh the retry clock and publish the pause reason.
            // Every other job field and all physical ownership must remain unchanged.
            heldJob.NextPlanningAt = Job().NextPlanningAt;
            heldJob.BlockedReason = "queue_paused";
            Check(WireJson.Encode(Job()) == WireJson.Encode(heldJob) && worker.Position.Equals(heldPosition) && worker.X == heldX && worker.Z == heldZ && WireJson.Encode(worker.Path) == heldPath && WireJson.Encode(worker.Cargo) == heldCargo && WireJson.Encode(world.Reservations) == heldClaims, "paused delegated work changed physical state or ingredient ownership after restart during " + phase);
            Apply(Edit("pause", false));
            DelegateUnchanged(); Restart(); CheckUnconsumed();
        }
        for (int second = 0; second < 900 && !Job().Completed; second++) world.Step(1);
        Check(Job().Completed && worker.JobId == "" && worker.Cargo.Count == 0 && Job().Local.Count == 0 && world.Reservations.Count == 0, "delegated finite batch did not finish and release exact ingredient ownership");
        var tool = village.Items.Single(i => i.Kind == "tool"); string toolId = tool.Id;
        Check(tool.Material == "wood" && tool.Condition == tool.MaxCondition && world.Total(village, "planks") == 0 && world.Total(village, "blocks") == 0, "delegated finite material accounting failed: material=" + tool.Material + " condition=" + tool.Condition + "/" + tool.MaxCondition + " planks=" + world.Total(village, "planks") + " blocks=" + world.Total(village, "blocks"));
        Check(tool.LocationId == store.Id, "delegated tool was not physically delivered to its designated accepting store: actual=" + tool.LocationId + " expected=" + store.Id);
        var tail = station.Slots.Single().Queue.Single();
        Check(station.Slots.Single().AutomatedBy == "forester" && tail.RecipeId == "planks_and_blocks_to_tools" && tail.Repeat && tail.AutomatedBy == "", "delegation replaced, relabeled or lost the player's repeating tail order");
        Restart(); world.Step(20);
        Check(village.Items.Count(i => i.Id == toolId) == 1 && village.Items.Single(i => i.Id == toolId).LocationId == store.Id && village.Jobs.Count(j => j.Id == jobId) == 1 && Job().Completed && !village.Jobs.Any(j => !j.Completed && j.Kind == "production") && world.Reservations.Count == 0, "completed delegated production replayed after its final restart");
        tail = station.Slots.Single().Queue.Single();
        Check(tail.Repeat && tail.AutomatedBy == "" && station.Slots.Single().AutomatedBy == "forester" && station.Slots.Single().CatId == workerId, "the officer replaced a deliberate unfunded repeating order after restart");
        AuthorityRuntime.ValidateWorld(world);
    }
    finally { Directory.Delete(directory, true); }
});
Test("officer reuses later open position and preserves paused manual orders across durable restarts", () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-officer-open-position-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        var path = Path.Combine(directory, "world.json"); var world = World.Create(41); var village = world.Villages.Single();
        village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
        village.Stockpiles.Clear(); village.LastLeaderResearch = world.TimeSeconds; village.ResearchPoints = 20000;
        var worker = village.Cats[1]; string workerId = worker.Id;
        foreach (var cat in village.Cats)
        {
            cat.Position = new Int2(1, 3); cat.X = 1; cat.Z = 3; cat.Path.Clear();
            cat.Skills.Clear(); cat.Preferences.Clear(); cat.Hunger = cat.Thirst = cat.Rest = 100;
        }
        World.Add(worker.Skills, "textile", 40); worker.Preferences.Add("textile");
        var knownLogs = world.Tiles.First(t => t.Resource == "logs" && t.Amount > 0).Position;
        if (!village.Known.Contains(knownLogs)) village.Known.Add(knownLogs);
        var store = new Stockpile { Id = "open-position-receiving-store", Position = new Int2(3, 2), Width = 1, Depth = 1, Capacity = 10000, Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000) } };
        var fibre = new Stockpile { Id = "open-position-finite-fibre", Position = new Int2(-3, 3), Width = 1, Depth = 1, Capacity = 100, Accepts = new List<string> { "fibre" }, Goods = new List<Stack> { new Stack("fibre", 5) } };
        village.Stockpiles.Add(store); village.Stockpiles.Add(fibre);
        var station = new Building { Id = world.Id("open-position-clothier"), Kind = "clothier", Completed = true, Position = worker.Position }; string stationId = station.Id;
        village.Buildings.Add(station);
        var context = new PlayerContext { PlayerId = "fixture-open-position", VillageId = village.Id };
        void Apply(GameAction action) { var result = world.Apply(context, action); Check(result.Success, action.Kind + " rejected: " + result.Error); }
        void Research(string id)
        {
            if (village.Research.Contains(id)) return;
            foreach (var prerequisite in Catalog.Study(id).Prerequisites) Research(prerequisite);
            Apply(new GameAction { Kind = "ResearchNode", NodeId = id });
        }
        Research("clothier_crews");
        Check(Catalog.BuildingEffect(village, "clothier", "worker_slots", 1) == 2, "Fixture requires ordinary researched two-cat capacity");
        var first = village.Cats[6]; var second = village.Cats[7];
        Apply(new GameAction { Kind = "AssignWorker", BuildingId = stationId, CatId = first.Id });
        Apply(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = stationId, CatId = first.Id, Edit = "add", RecipeId = "fibre_to_thread", Repeat = true });
        Apply(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = stationId, CatId = first.Id, Edit = "pause", Enabled = true });
        Apply(new GameAction { Kind = "AssignWorker", BuildingId = stationId, CatId = second.Id });
        foreach (string recipe in new[] { "fibre_to_thread", "fibre_to_cloth" })
            Apply(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = stationId, CatId = second.Id, Edit = "add", RecipeId = recipe });
        Apply(new GameAction { Kind = "AssignWorker", CatId = first.Id });
        Apply(new GameAction { Kind = "AssignWorker", CatId = second.Id });
        Check(station.Slots.Count == 2 && station.Slots.All(s => s.CatId == "") && station.Slots[0].Paused && !station.Slots[1].Paused && !station.Paused, "Public edits did not leave a paused first vacancy and later open vacancy");
        string pausedSlot = WireJson.Encode(station.Slots[0]), manualQueue = WireJson.Encode(station.Slots[1].Queue);
        var heldIds = village.Cats.Where(c => c.Id != workerId && !village.Cats.Skip(2).Take(4).Any(free => free.Id == c.Id)).Select(c => c.Id).ToArray();
        foreach (string heldId in heldIds)
        {
            var holder = new PlayerContext { PlayerId = "open-position-holder-" + heldId, VillageId = village.Id };
            Check(world.Apply(holder, new GameAction { Kind = "EnterCatControl", CatId = heldId }).Success, "Could not reserve a nonparticipant through public control");
        }
        Apply(new GameAction { Kind = "AssignOfficer", Role = "cloth_leader", CatId = village.LeaderId });
        void Step(double seconds)
        {
            foreach (string heldId in heldIds)
                Check(world.Apply(new PlayerContext { PlayerId = "open-position-holder-" + heldId, VillageId = village.Id }, new GameAction { Kind = "KeepCatControl", CatId = heldId }).Success, "Nonparticipant control lease expired");
            world.Step(seconds);
        }
        void CheckSlots(bool beforeCompletion)
        {
            Check(station.Slots.Count == 2 && WireJson.Encode(station.Slots[0]) == pausedSlot, "Officer or restart changed the paused empty position or deliberate repeating order");
            Check(station.Slots[1].CatId == workerId && station.Slots[1].AutomatedBy == "cloth_leader" && worker.BuildingId == stationId && station.WorkerId == workerId, "Officer failed to reuse only the later open position with the intended specialist");
            if (beforeCompletion) Check(WireJson.Encode(station.Slots[1].Queue) == manualQueue, "Officer changed or relabeled the later position's finite manual chain");
        }
        void Restart()
        {
            AuthorityRuntime.ValidateWorld(world); SaveStore.Save(path, world); string expected = WireJson.Encode(world);
            world = SaveStore.Load<World>(path); Check(WireJson.Encode(world) == expected, "Durable restart changed open-position work, exact inputs, cargo, pause or manual queues");
            AuthorityRuntime.ValidateWorld(world); village = world.Villages.Single(); worker = village.Cats.Single(c => c.Id == workerId); station = village.Buildings.Single(b => b.Id == stationId);
            store = village.Stockpiles.Single(p => p.Id == "open-position-receiving-store"); fibre = village.Stockpiles.Single(p => p.Id == "open-position-finite-fibre");
        }
        Restart(); Step(10.05); CheckSlots(true);
        string jobId = "";
        foreach (string phase in new[] { "claimed", "carried", "working" })
        {
            bool reached = false;
            for (int tick = 0; tick < 4000 && !reached; tick++)
            {
                Step(.05);
                var active = village.Jobs.Find(j => !j.Completed && j.Kind == "production" && j.CatId == workerId);
                if (active == null) continue;
                if (jobId == "") jobId = active.Id;
                Check(active.Id == jobId && active.RecipeId == "fibre_to_thread", "Officer changed the finite job identity before its restart checkpoints");
                CheckSlots(true);
                double unconsumed = village.Stockpiles.Sum(p => World.Amount(p.Goods, "fibre")) + village.Cats.Sum(c => World.Amount(c.Cargo, "fibre")) + village.Jobs.Where(j => !j.Completed).Sum(j => World.Amount(j.Local, "fibre")) + village.Buildings.Sum(b => World.Amount(b.Inputs, "fibre"));
                Check(unconsumed == 5, "Restarted open-position job lost or duplicated the five finite Fibre");
                reached = phase == "claimed" ? world.Reservations.Any(r => r.OwnerId == jobId && r.PileId == fibre.Id && r.Amount == 5) : phase == "carried" ? World.Amount(worker.Cargo, "fibre") == 5 : active.Phase == "working" && active.Progress > 0;
            }
            Check(reached, "Later open position did not reach its " + phase + " checkpoint");
            CheckSlots(true);
            if (phase == "claimed") Check(village.Cats.Count(c => c.Alive && c.ControlledBy == "" && c.JobId == "" && c.BuildingId == "") == 4, "Automatic staffing consumed the four-cat idle reserve");
            Restart(); CheckSlots(true);
        }
        // The simulation prunes completed jobs after ten minutes from start.
        // Keep the exact jobs observed during this finite chain, including their
        // completed snapshots, rather than requiring both tombstones to coexist.
        var observedProduction = new Dictionary<string, Job>();
        void ObserveProduction()
        {
            foreach (var production in village.Jobs.Where(j => j.Kind == "production" && j.TargetId == stationId))
                observedProduction[production.Id] = WireJson.Decode<Job>(WireJson.Encode(production));
        }
        ObserveProduction();
        for (int secondElapsed = 0; secondElapsed < 1600 && observedProduction.Values.Count(j => j.Completed) < 2; secondElapsed++)
        {
            Step(1); ObserveProduction();
        }
        CheckSlots(false);
        var finished = observedProduction.Values.OrderBy(j => j.StartedAt).ThenBy(j => j.Id, StringComparer.Ordinal).ToArray();
        double output = Catalog.BuildingEffect(village, "clothier", "output", 1);
        Check(finished.Length == 2 && finished.All(j => j.Completed) && finished.Select(j => j.RecipeId).SequenceEqual(new[] { "fibre_to_thread", "fibre_to_cloth" }), "Finite manual chain did not finish exactly two jobs in order: " + string.Join(", ", finished.Select(j => j.Id + "/" + j.RecipeId + "/" + j.Phase + "/completed=" + j.Completed + "/work=" + j.Progress + "/" + j.RequiredWork)) + "; time=" + world.TimeSeconds + "; worker=" + worker.Goal + "; queue=" + WireJson.Encode(station.Slots[1].Queue));
        Check(finished.Single(j => j.Id == jobId).Completed && worker.JobId == "" && worker.Cargo.Count == 0 && finished.All(j => j.Local.Count == 0) && world.Reservations.Count == 0 && !village.Jobs.Any(j => !j.Completed && j.Kind == "production" && j.TargetId == stationId), "Finished open-position chain retained active ownership");
        Check(world.Total(village, "fibre") == 0 && Math.Abs(World.Amount(store.Goods, "thread") - (5 * output - 5)) < 1e-8 && Math.Abs(World.Amount(store.Goods, "cloth") - output) < 1e-8 && station.Slots[1].Queue.Count == 0, "Finite Fibre did not become the exact researched Thread surplus and Cloth output in accepting storage");
        var completedIds = finished.Select(j => j.Id).ToHashSet(StringComparer.Ordinal);
        var completionEvents = village.Events.Where(e => e.Kind == "completed" && completedIds.Contains(e.EntityId)).ToArray();
        Check(completionEvents.Select(e => e.EntityId).SequenceEqual(finished.Select(j => j.Id)), "Finite chain did not retain exactly one completion event per observed job in order");
        string completionEvidence = WireJson.Encode(completionEvents); double thread = world.Total(village, "thread"), cloth = world.Total(village, "cloth");
        Restart(); Step(20); CheckSlots(false);
        Check(village.Jobs.Where(j => j.Kind == "production" && j.TargetId == stationId).All(j => completedIds.Contains(j.Id) && j.Completed && WireJson.Encode(j) == WireJson.Encode(observedProduction[j.Id])) && WireJson.Encode(village.Events.Where(e => e.Kind == "completed" && completedIds.Contains(e.EntityId)).ToArray()) == completionEvidence && world.Total(village, "thread") == thread && world.Total(village, "cloth") == cloth && world.Reservations.Count == 0 && station.Slots[1].Queue.Count == 0, "Completed finite chain replayed after restart");
        AuthorityRuntime.ValidateWorld(world);
    }
    finally { Directory.Delete(directory, true); }
});
Test("finite three-workshop chain preserves rejected officer edits and exact output across durable restarts", () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-finite-workshop-chain-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        var path = Path.Combine(directory, "world.json"); var world = World.Create(41); var village = world.Villages.Single();
        village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
        village.Stockpiles.Clear(); village.LastLeaderResearch = world.TimeSeconds;
        var workers = village.Cats.Where(c => c.Id != village.LeaderId).Take(3).ToArray();
        var workerIds = workers.Select(c => c.Id).ToArray();
        foreach (var cat in village.Cats)
        {
            cat.ControlledBy = workerIds.Contains(cat.Id) ? "" : "fixture-observer"; cat.ControlLeaseUntil = 100000;
            cat.Position = new Int2(1, 3); cat.X = 1; cat.Z = 3; cat.Path.Clear();
        }
        village.Stockpiles.Add(new Stockpile { Id = "finite-chain-provisions-and-tools", Position = new Int2(3, 2), Width = 1, Depth = 1, Capacity = 10000, Accepts = new List<string> { "food", "water", "tools" }, Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000) } });
        village.Stockpiles.Add(new Stockpile { Id = "finite-chain-logs", Position = new Int2(3, 3), Width = 1, Depth = 1, Capacity = 20, Accepts = new List<string> { "logs" }, Goods = new List<Stack> { new Stack("logs", 10) } });
        village.Stockpiles.Add(new Stockpile { Id = "finite-chain-stone", Position = new Int2(3, 5), Width = 1, Depth = 1, Capacity = 20, Accepts = new List<string> { "stone" }, Goods = new List<Stack> { new Stack("stone", 10) } });
        village.Stockpiles.Add(new Stockpile { Id = "finite-chain-planks", Position = new Int2(-1, 3), Width = 1, Depth = 1, Capacity = 10, Accepts = new List<string> { "planks" } });
        village.Stockpiles.Add(new Stockpile { Id = "finite-chain-blocks", Position = new Int2(-1, 5), Width = 1, Depth = 1, Capacity = 10, Accepts = new List<string> { "blocks" } });
        var stations = new[]
        {
            new Building { Id = world.Id("finite-chain-cutter"), Kind = "wood_cutter", Completed = true, Position = new Int2(-3, 3) },
            new Building { Id = world.Id("finite-chain-stone-prep"), Kind = "stone_prep", Completed = true, Position = new Int2(-3, 6) },
            new Building { Id = world.Id("finite-chain-woodworking"), Kind = "woodworking", Completed = true, Position = new Int2(-6, 3) }
        };
        village.Buildings.AddRange(stations); var stationIds = stations.Select(b => b.Id).ToArray();
        village.Buildings.Add(new Building { Id = world.Id("finite-chain-office"), Kind = "sawmill", Completed = true, Position = new Int2(-6, 0) });
        village.Research.Add("sawmill");
        var context = new PlayerContext { PlayerId = "fixture-finite-chain", VillageId = village.Id };
        void Apply(GameAction action) { var result = world.Apply(context, action); Check(result.Success, action.Kind + " rejected: " + result.Error); }
        Apply(new GameAction { Kind = "AssignOfficer", Role = "forester", CatId = village.LeaderId });
        var recipeIds = new[] { "logs_to_planks", "stone_to_blocks", "planks_and_blocks_to_tools" };
        for (int i = 0; i < workers.Length; i++)
        {
            var recipe = Catalog.Recipe(recipeIds[i]);
            Check(recipe.Inputs.All(s => world.RecipeInput(village, recipe, s.Amount) == s.Amount) && recipe.Outputs.All(s => world.RecipeOutput(village, recipe, s.Amount) == s.Amount), "Finite chain requires unmodified founding recipe bills and yields");
            Apply(new GameAction { Kind = "AssignWorker", BuildingId = stationIds[i], CatId = workerIds[i] });
            for (int order = 0; order < (i == 2 ? 1 : 2); order++)
                Apply(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = stationIds[i], CatId = workerIds[i], Edit = "add", RecipeId = recipeIds[i] });
        }
        Apply(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = stationIds[2], CatId = workerIds[2], Edit = "delegate" });
        string beforeRejected = WireJson.Encode(world);
        var rejected = world.Apply(context, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = stationIds[2], CatId = workerIds[2], Edit = "add", RecipeId = "fibre_to_cloth" });
        Check(!rejected.Success && rejected.Error == "Recipe locked or wrong station", "Wrong-station recipe edit was not rejected for its recipe");
        Check(WireJson.Encode(world) == beforeRejected, "Rejected queue edit changed officer ownership before the finite upstream chain could fund its work");
        var observed = new Dictionary<string, Job>(StringComparer.Ordinal);
        double Physical(string resource) => village.Stockpiles.Sum(p => World.Amount(p.Goods, resource)) + village.Cats.Sum(c => World.Amount(c.Cargo, resource)) + village.Jobs.Where(j => !j.Completed).Sum(j => World.Amount(j.Local, resource)) + village.Buildings.Sum(b => World.Amount(b.Inputs, resource) + World.Amount(b.Outputs, resource));
        void Conserved()
        {
            int tools = village.Items.Count(i => i.Kind == "tool");
            Check(Math.Abs(Physical("logs") + 5 * Physical("planks") + 10 * tools - 10) < 1e-8 && Math.Abs(Physical("stone") + 5 * Physical("blocks") + 10 * tools - 10) < 1e-8, "Finite three-workshop chain lost or duplicated a raw-material equivalent");
            Check(village.Items.All(i => i.Kind == "tool") && village.Stockpiles.Sum(p => World.Amount(p.Goods, "tools")) == 0 && village.Cats.Sum(c => World.Amount(c.Cargo, "tools")) == 0, "Exact tool gained a scalar duplicate or unrelated item");
            Check(stations.All(b => b.Slots.Single().CatId == workerIds[Array.IndexOf(stations, b)]) && stations[2].Slots.Single().AutomatedBy == "forester", "Chain changed exact labor or lost officer ownership");
        }
        void Step(double seconds)
        {
            world.Step(seconds);
            foreach (var job in village.Jobs.Where(j => j.Completed && j.Kind == "production" && stationIds.Contains(j.TargetId) && !observed.ContainsKey(j.Id)))
                observed.Add(job.Id, WireJson.Decode<Job>(WireJson.Encode(job)));
            Conserved();
        }
        void Reach(Func<bool> predicate, int maximumSeconds, string reason)
        {
            for (int tick = 0; tick < maximumSeconds * 20 && !predicate(); tick++) Step(.05);
            Check(predicate(), reason);
        }
        void Restart()
        {
            AuthorityRuntime.ValidateWorld(world); SaveStore.Save(path, world); string expected = WireJson.Encode(world);
            world = SaveStore.Load<World>(path); Check(WireJson.Encode(world) == expected, "Durable restart changed finite upstream work, queues, ownership or exact output");
            AuthorityRuntime.ValidateWorld(world); village = world.Villages.Single();
            workers = workerIds.Select(id => village.Cats.Single(c => c.Id == id)).ToArray();
            stations = stationIds.Select(id => village.Buildings.Single(b => b.Id == id)).ToArray(); Conserved();
        }
        Conserved(); Restart();
        Reach(() => village.Jobs.Any(j => !j.Completed && j.TargetId == stationIds[0] && j.Phase == "output_delivery" && World.Amount(j.Local, "planks") + World.Amount(workers[0].Cargo, "planks") == 1), 900, "Upstream workshop never exposed its physical first Plank delivery");
        Check(world.Total(village, "planks") == 0 && !village.Jobs.Any(j => j.TargetId == stationIds[2]), "Downstream work used an upstream output before storage delivery"); Restart();
        Reach(() => village.Jobs.Any(j => !j.Completed && j.TargetId == stationIds[2]) && world.Reservations.Any(r => village.Jobs.Any(j => !j.Completed && j.TargetId == stationIds[2] && j.Id == r.OwnerId)), 1500, "Downstream station never claimed both physically delivered finite inputs");
        string toolJobId = village.Jobs.Single(j => !j.Completed && j.TargetId == stationIds[2]).Id;
        Job ToolJob() => village.Jobs.Single(j => j.Id == toolJobId);
        Check(world.Reservations.Where(r => r.OwnerId == toolJobId).Sum(r => r.Amount) == 4 && workers[2].Cargo.Count == 0 && ToolJob().Local.Count == 0, "Downstream claims did not exclusively own two Planks and two Blocks before pickup");
        string beforeActiveRemoval = WireJson.Encode(world);
        var rejectedRemoval = world.Apply(context, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = stationIds[2], CatId = workerIds[2], Edit = "remove", Index = 0 });
        Check(!rejectedRemoval.Success && WireJson.Encode(world) == beforeActiveRemoval, "Removing an in-flight finite order changed its exact queue or officer ownership"); Restart();
        Reach(() => World.Amount(workers[2].Cargo, "planks") == 2, 60, "Downstream worker never physically carried its two upstream Planks"); Restart();
        Reach(() => World.Amount(ToolJob().Local, "planks") == 2, 60, "Downstream worker never staged the exact upstream Planks");
        Apply(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = stationIds[2], CatId = workerIds[2], Edit = "pause", Enabled = true });
        Apply(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = stationIds[2], CatId = workerIds[2], Edit = "delegate" }); Restart();
        string pausedCargo = WireJson.Encode(workers[2].Cargo), pausedClaims = WireJson.Encode(world.Reservations), pausedLocal = WireJson.Encode(ToolJob().Local);
        double pausedProgress = ToolJob().Progress; var pausedPosition = workers[2].Position; double pausedX = workers[2].X, pausedZ = workers[2].Z;
        Step(.2);
        Check(ToolJob().BlockedReason == "queue_paused" && ToolJob().Progress == pausedProgress && WireJson.Encode(ToolJob().Local) == pausedLocal && WireJson.Encode(workers[2].Cargo) == pausedCargo && WireJson.Encode(world.Reservations) == pausedClaims && workers[2].Position.Equals(pausedPosition) && workers[2].X == pausedX && workers[2].Z == pausedZ, "Restarted downstream pause advanced physical work or changed ingredient ownership");
        Apply(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = stationIds[2], CatId = workerIds[2], Edit = "pause", Enabled = false });
        Apply(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = stationIds[2], CatId = workerIds[2], Edit = "delegate" }); Restart();
        Reach(() => ToolJob().Phase == "output_delivery" && village.Items.Any(i => i.LocationId == toolJobId), 900, "Finite tool did not become one exact in-flight output");
        var tool = village.Items.Single(); string toolId = tool.Id;
        Check(tool.Kind == "tool" && tool.Material == "wood" && tool.Quality == 1 && tool.Condition == tool.MaxCondition && tool.Condition == 100 && world.Total(village, "tools") == 0, "Tool output identity, metadata or pre-delivery ownership is incorrect"); Restart();
        Reach(() => ToolJob().Completed, 60, "Exact finite tool was never physically delivered");
        Check(village.Items.Single().Id == toolId && village.Items.Single().LocationId == "finite-chain-provisions-and-tools" && world.Total(village, "tools") == 1 && new[] { "logs", "stone", "planks", "blocks" }.All(r => Physical(r) == 0) && world.Reservations.Count == 0 && workers.All(c => c.JobId == "" && c.Cargo.Count == 0) && stations.All(b => b.Slots.Single().Queue.Count == 0), "Completed chain retained input material, orders or claims, or delivered the wrong exact item");
        Check(observed.Values.Count(j => j.RecipeId == "logs_to_planks" && j.Completed) == 2 && observed.Values.Count(j => j.RecipeId == "stone_to_blocks" && j.Completed) == 2 && observed.Values.Count(j => j.RecipeId == "planks_and_blocks_to_tools" && j.Completed) == 1 && observed.Count == 5, "Finite chain did not record exactly its five completed public orders");
        string completedTool = WireJson.Encode(village.Items.Single()); int completions = village.Events.Count(e => e.Kind == "completed" && observed.ContainsKey(e.EntityId));
        Check(completions == 5, "Finite chain completion events do not match all five observed jobs"); Restart(); Step(20);
        Check(village.Items.Count == 1 && WireJson.Encode(village.Items.Single()) == completedTool && village.Events.Count(e => e.Kind == "completed" && observed.ContainsKey(e.EntityId)) == completions && !village.Jobs.Any(j => !j.Completed && j.Kind == "production") && world.Reservations.Count == 0 && stations.All(b => b.Slots.Single().Queue.Count == 0), "Restart replayed a completed finite workshop order or duplicated its exact output");
        AuthorityRuntime.ValidateWorld(world);
    }
    finally { Directory.Delete(directory, true); }
});
Test("physical equipment LocalAuthority restarts conserve exact claims needs and returns", () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-equipment-restart-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory); LocalAuthority authority = null;
    try
    {
        var path = Path.Combine(directory, "world.json"); authority = LocalAuthority.CreateNew(path, 41);
        var world = authority.World; var village = world.Villages.Single(); var runner = village.Cats.First(c => c.Id != village.LeaderId); string runnerId = runner.Id;
        var other = village.Cats.First(c => c.Id != runnerId && c.Id != village.LeaderId); string otherId = other.Id;
        foreach (var cat in village.Cats.Where(c => c.Id != runnerId)) { cat.ControlledBy = "fixture-observer"; cat.ControlLeaseUntil = 100000; }
        village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine"); village.Stockpiles.Clear(); village.LastLeaderResearch = world.TimeSeconds;
        runner.Position = new Int2(0, 3); runner.X = 0; runner.Z = 3; runner.Path.Clear(); runner.Hunger = runner.Thirst = runner.Rest = 80;
        var source = new Stockpile { Id = "equipment-restart-source", Position = new Int2(6, 3), Width = 1, Depth = 1, Capacity = 1, Accepts = new List<string> { "tools" } };
        var destination = new Stockpile { Id = "equipment-restart-destination", Position = new Int2(-6, 3), Width = 1, Depth = 1, Capacity = 1, Accepts = new List<string> { "tools" } };
        village.Stockpiles.Add(source); village.Stockpiles.Add(destination);
        village.Stockpiles.Add(new Stockpile { Id = "equipment-restart-needs", Position = runner.Position, Width = 1, Depth = 1, Capacity = 1000, Accepts = new List<string> { "food", "water", "logs" }, Goods = new List<IdleCatForest.Simulation.Stack> { new IdleCatForest.Simulation.Stack("food", 400), new IdleCatForest.Simulation.Stack("water", 400), new IdleCatForest.Simulation.Stack("logs", 20) } });
        string itemId = "equipment-restart-exact-tool";
        village.Items.Add(new Item { Id = itemId, Kind = "tool", Material = "wood", VillageId = village.Id, LocationId = source.Id, Quality = 3, Condition = 17, MaxCondition = 42 });
        Check(world.Path(runner.Position, source.Position, village) != null && world.Path(source.Position, destination.Position, village) != null, "equipment restart fixture lacks physical routes");
        ActionResult Apply(GameAction action) { var result = authority.Apply(action); Check(result.Success, action.Kind + " rejected: " + result.Error); return result; }
        Item Exact() => village.Items.Single(i => i.Id == itemId);
        Job Order(string id) => village.Jobs.Single(j => j.Id == id);
        void Conserved()
        {
            Check(village.Items.Count == 1 && Exact().Kind == "tool" && Exact().Material == "wood" && Exact().Quality == 3 && Exact().Condition == 17 && Exact().MaxCondition == 42, "physical equipment lost or copied its exact identity or metadata");
            Check(village.Stockpiles.All(p => World.Amount(p.Goods, "tools") == 0) && village.Cats.All(c => World.Amount(c.Cargo, "tools") == 0) && world.Reservations.Count == 0, "exact equipment acquired a duplicate scalar or resource claim");
            AuthorityRuntime.ValidateWorld(world);
        }
        void Restart()
        {
            Conserved(); authority.Save(); string expected = WireJson.Encode(world); authority.Dispose(); authority = null;
            authority = new LocalAuthority(path, 999); world = authority.World; village = world.Villages.Single(); runner = village.Cats.Single(c => c.Id == runnerId); other = village.Cats.Single(c => c.Id == otherId);
            source = village.Stockpiles.Find(p => p.Id == "equipment-restart-source"); destination = village.Stockpiles.Single(p => p.Id == "equipment-restart-destination");
            Check(WireJson.Encode(world) == expected, "LocalAuthority restart changed exact equipment ownership, needs, route or consumed clock"); Conserved();
        }
        void Finish(string jobId)
        {
            for (int tick = 0; tick < 2400 && !Order(jobId).Completed; tick++) { authority.Advance(.05); Conserved(); }
            Check(Order(jobId).Completed, "physical equipment order did not finish after bounded travel: " + jobId + " phase=" + Order(jobId).Phase + " reason=" + Order(jobId).BlockedReason); Conserved();
        }
        string firstJob = Apply(new GameAction { Kind = "EquipItem", CatId = runnerId, TargetId = itemId }).EntityId;
        Check(Order(firstJob).Kind == "equip" && Order(firstJob).Phase == "item_fetch" && Exact().LocationId == firstJob && !runner.Equipment.Contains(itemId), "retrieval did not reserve the exact item at its physical source");
        Check(!world.HasRoom(village, source, "tools", 1), "equipment claim prematurely freed its source capacity");
        other.ControlledBy = "";
        Check(!authority.Apply(new GameAction { Kind = "EquipItem", CatId = otherId, TargetId = itemId }).Success, "a second cat claimed the same exact equipment"); other.ControlledBy = "fixture-observer";
        Check(!authority.Apply(new GameAction { Kind = "CancelEquipment", CatId = otherId, TargetId = firstJob }).Success && !Order(firstJob).Completed, "another cat canceled the exact equipment owner");
        authority.Advance(.2); Restart();
        Check(Exact().LocationId == firstJob && runner.JobId == firstJob && !runner.Equipment.Contains(itemId), "travel restart teleported equipment or replaced its claim");
        runner.Thirst = 34; authority.Advance(.05);
        Check(runner.Goal == "need_drink" && runner.ResumeJobId == firstJob && !Order(firstJob).Completed && Exact().LocationId == firstJob, "need interruption dropped or equipped its reserved item"); Restart();
        for (int tick = 0; tick < 1000 && runner.ResumeJobId == firstJob; tick++) { authority.Advance(.05); Conserved(); }
        Check(runner.JobId == firstJob && runner.ResumeJobId == "" && !Order(firstJob).Completed, "restarted need did not resume the same uncollected equipment order");
        Apply(new GameAction { Kind = "CancelEquipment", CatId = runnerId, TargetId = firstJob }); Restart();
        Check(Order(firstJob).Completed && Order(firstJob).ItemIds.Count == 0 && runner.JobId == "" && runner.ResumeJobId == "" && Exact().LocationId == source.Id && !runner.Equipment.Contains(itemId), "cancelled retrieval did not restore its exact physical source");
        string fetchJob = Apply(new GameAction { Kind = "EquipItem", CatId = runnerId, TargetId = itemId }).EntityId;
        Check(!authority.Apply(new GameAction { Kind = "CancelEquipment", CatId = runnerId, TargetId = firstJob }).Success && runner.JobId == fetchJob, "a stale cancellation removed the replacement equipment order"); Restart();
        var corrupt = WireJson.Clone(world); corrupt.Villages.Single().Items.Single(i => i.Id == itemId).LocationId = source.Id;
        string corruptPath = Path.Combine(directory, "corrupt-equipment.json"); SaveStore.Save(corruptPath, corrupt, true); var corruptBytes = File.ReadAllBytes(corruptPath);
        bool rejected = false; try { using var invalid = new LocalAuthority(corruptPath, 41); } catch (InvalidDataException) { rejected = true; }
        Check(rejected && File.ReadAllBytes(corruptPath).SequenceEqual(corruptBytes) && !File.Exists(corruptPath + ".previous"), "corrupt exact equipment claim was accepted or rewritten");
        Finish(fetchJob); Restart();
        Check(Exact().LocationId == runnerId && runner.Equipment.Count(id => id == itemId) == 1 && runner.JobId == "" && runner.ResumeJobId == "", "arrival restart did not retain exactly one physically worn item");
        Apply(new GameAction { Kind = "RemoveStockpile", TargetId = source.Id }); source = null;
        string returnJob = Apply(new GameAction { Kind = "UnequipItem", CatId = runnerId, TargetId = itemId }).EntityId;
        Check(Order(returnJob).Kind == "unequip" && Order(returnJob).Phase == "item_return" && Order(returnJob).TargetId == destination.Id && Exact().LocationId == runnerId && runner.Equipment.Contains(itemId), "return teleported the worn item before physical arrival");
        authority.Advance(.2); Restart();
        Check(Exact().LocationId == runnerId && runner.Equipment.Contains(itemId) && runner.JobId == returnJob, "return restart lost the exact bearer");
        Apply(new GameAction { Kind = "CancelEquipment", CatId = runnerId, TargetId = returnJob }); Restart();
        Check(Order(returnJob).Completed && Exact().LocationId == runnerId && runner.Equipment.Contains(itemId) && runner.JobId == "", "canceling a return detached or duplicated worn equipment");
        string finalReturn = Apply(new GameAction { Kind = "UnequipItem", CatId = runnerId, TargetId = itemId }).EntityId;
        Check(!authority.Apply(new GameAction { Kind = "CancelEquipment", CatId = runnerId, TargetId = returnJob }).Success && runner.JobId == finalReturn, "stale return cancellation removed the new physical order"); Restart();
        Finish(finalReturn); Restart(); authority.Advance(2); Conserved();
        Check(Exact().LocationId == destination.Id && runner.Equipment.Count == 0 && runner.JobId == "" && runner.ResumeJobId == "" && Order(finalReturn).Completed && village.Jobs.Count(j => j.Id == finalReturn) == 1 && village.Jobs.All(j => j.Completed), "completed equipment return replayed or retained exact work ownership");
    }
    finally { authority?.Dispose(); Directory.Delete(directory, true); }
});
Test("exact crafted haul survives claimed pickup and full-storage restarts", () =>
{
    var directory = Path.Combine(Path.GetTempPath(), "forest-exact-haul-" + Guid.NewGuid().ToString("N"));
    Directory.CreateDirectory(directory);
    try
    {
        var path = Path.Combine(directory, "world.json");
        var world = World.Create(41); var v = world.Villages.Single();
        var cat = v.Cats.First(c => c.Id != v.LeaderId);
        cat.X = -4; cat.Z = 0;
        foreach (var other in v.Cats.Where(c => c.Id != cat.Id)) { other.ControlledBy = "fixture-observer"; other.ControlLeaseUntil = 10000; }
        foreach (var pile in v.Stockpiles) pile.Accepts = new List<string> { "food", "water" };
        var source = new Stockpile { Id = world.Id("store"), Kind = "storage", Position = new Int2(0, 0), Width = 1, Depth = 1, Capacity = 1, Accepts = new List<string> { "mugs" } };
        var destination = new Stockpile { Id = world.Id("store"), Kind = "storage", Position = new Int2(4, 0), Width = 1, Depth = 1, Capacity = 1, Accepts = new List<string> { "mugs" } };
        v.Stockpiles.Add(source); v.Stockpiles.Add(destination);
        var item = new Item { Id = world.Id("mug"), Kind = "mug", Material = "wood", VillageId = v.Id, LocationId = source.Id, Quality = 3, Condition = 17, MaxCondition = 42 };
        v.Items.Add(item);
        string catId = cat.Id, itemId = item.Id, destinationId = destination.Id, sourceId = source.Id;
        var context = new PlayerContext { PlayerId = "fixture-hauler", VillageId = v.Id };
        var result = world.Apply(context, new GameAction { Kind = "HaulGatherSpot", CatId = cat.Id, TargetId = source.Id });
        Check(result.Success, "exact crafted recovery rejected: " + result.Error);
        string jobId = result.EntityId;
        void RestartHaul()
        {
            AuthorityRuntime.ValidateWorld(world); SaveStore.Save(path, world); world = SaveStore.Load<World>(path);
            AuthorityRuntime.ValidateWorld(world); v = world.Villages.Single(); cat = v.Cats.Single(c => c.Id == catId); item = v.Items.Single(i => i.Id == itemId);
        }
        RestartHaul();
        Check(v.Jobs.Single(j => j.Id == jobId).Phase == "item_fetch" && item.LocationId == jobId, "restart lost exclusive pre-pickup ownership");
        Check(!world.HasRoom(v, v.Stockpiles.Single(p => p.Id == sourceId), "mugs", 1), "Claiming an item freed its physical source space before pickup");
        Check(!world.Apply(context, new GameAction { Kind = "HaulGatherSpot", TargetId = sourceId }).Success, "restart made claimed item available twice");
        for (int i = 0; i < 30 && v.Jobs.Single(j => j.Id == jobId).Phase == "item_fetch"; i++) world.Step(1);
        Check(v.Jobs.Single(j => j.Id == jobId).Phase == "output_delivery" && item.LocationId == jobId, "physical pickup failed");
        Check(world.HasRoom(v, v.Stockpiles.Single(p => p.Id == sourceId), "mugs", 1), "Physical pickup did not free source capacity");
        Check(world.Apply(context, new GameAction { Kind = "RemoveStockpile", TargetId = sourceId }).Success, "Could not retire the emptied source before delivery");
        RestartHaul();
        var blocker = new Item { Id = world.Id("occupied"), Kind = "mug", VillageId = v.Id, LocationId = destinationId };
        v.Items.Add(blocker); world.Step(10);
        Check(!v.Jobs.Single(j => j.Id == jobId).Completed && item.LocationId == jobId, "full storage consumed or duplicated the returning item");
        RestartHaul(); v.Items.RemoveAll(i => i.Id == blocker.Id);
        for (int i = 0; i < 30 && !v.Jobs.Single(j => j.Id == jobId).Completed; i++) world.Step(1);
        Check(v.Jobs.Single(j => j.Id == jobId).Completed && item.LocationId == destinationId, "restarted delivery never used the freed capacity");
        RestartHaul(); world.Step(2);
        Check(v.Items.Count(i => i.Id == itemId) == 1 && item.LocationId == destinationId && item.Condition == 17 && item.MaxCondition == 42 && item.Quality == 3, "delivered identity, quality or condition changed or replayed");
    }
    finally { Directory.Delete(directory, true); }
});
FoodChainScenarios.Run(Test, Check);
RenewableGrainPersistenceScenarios.Run(Test, Check);
PrimaryQueueScenarios.Run(Test, Check);
PrimaryQueuePersistenceScenarios.Run(Test, Check);
SaveTimingScenarios.Run(Test, Check);
SaveEncodingScenarios.Run(Test, Check);
ImportScenarios.Run(Test, Check);
WorldGenerationTests.Run(Test, Check);
if (Environment.GetEnvironmentVariable("FOREST_IMPORTED_WORLD") is string importedWorld)
    Test("external synthetic SQLite world resumes deterministically", () =>
    {
        var whole = SaveStore.Load<World>(importedWorld); var split = SaveStore.Load<World>(importedWorld);
        whole.Step(5); for (int i = 0; i < 10; i++) split.Step(0.5);
        AuthorityRuntime.ValidateWorld(whole); AuthorityRuntime.ValidateWorld(split);
        Check(WireJson.Encode(whole) == WireJson.Encode(split), "imported world changed with tick partition");
        Check(whole.Villages.Single().Cats.Count == 30, "real SQLite roster reset during continuation");
    });
Console.WriteLine($"{failures.Count} failed");
return failures.Count == 0 ? 0 : 1;

sealed class SaveFixture { public string Identity = ""; public double Cargo, Reserved; }
