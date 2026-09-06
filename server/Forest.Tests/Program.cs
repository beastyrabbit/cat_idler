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
        Check((await client.SendAsync(new GameAction { Kind = "EquipItem", CatId = bearer.Id, TargetId = "counted-tool" })).Success, "authorized client could not select and equip its counted item");
        Check(client.LatestWorld.Village(village.Id).Items.Count == 0 && client.LatestWorld.Village(village.Id).Cats.Single(c => c.Id == bearer.Id).Equipment.Count == 0, "stale equipment count exposed exact inventory");
        CountStorage(); await client.SendAsync(new GameAction { Kind = "Presence" });
        visible = client.LatestWorld.Village(village.Id);
        Check(visible.Items.Single(i => i.Id == "counted-tool").LocationId == bearer.Id && visible.Cats.Single(c => c.Id == bearer.Id).Equipment.Contains("counted-tool"), "recount failed to restore the equipped item identity");
        lock (runtime.Sync) World.Add(pile.Goods, "logs", 1);
        await client.SendAsync(new GameAction { Kind = "Presence" });
        Check(client.LatestWorld.Village(village.Id).Items.Count == 0, "stale scalar count exposed exact equipment");
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
                foreach (var building in village.Buildings) building.Position = Move(building.Position);
                foreach (var pile in village.Stockpiles) pile.Position = Move(pile.Position);
                foreach (var p in village.Known) { var tile = runtime.World.TileAt(p); tile.Water = tile.Mountain = false; tile.Wall = false; }
            }
            Rebase(first, new Int2(30, 0)); Rebase(second, new Int2(45, 0));
            for (int x = 30; x <= 45; x++) { var tile = runtime.World.TileAt(new Int2(x, 0)); tile.Water = tile.Mountain = tile.Wall = false; }
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
ImportScenarios.Run(Test, Check);
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
