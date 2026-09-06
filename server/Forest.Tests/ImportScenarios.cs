using IdleCatForest.Authority;
using IdleCatForest.SaveImport;
using IdleCatForest.Simulation;
using Newtonsoft.Json.Linq;

static class ImportScenarios
{
    static JObject Row(params object[] values) { var row = new JObject(); for (int i = 0; i < values.Length; i += 2) row[(string)values[i]] = values[i + 1] == null ? JValue.CreateNull() : JToken.FromObject(values[i + 1]); return row; }
    static string Json(object value) => WireJson.Encode(value);
    static JObject Base()
    {
        var tables = new JObject(); foreach (var name in new[] { "world", "colonies", "cats", "jobs", "buildings", "world_tiles", "shared_world_tiles", "events", "player_names", "zones", "elections", "votes", "raiders" }) tables[name] = new JArray();
        ((JArray)tables["world"]).Add(Row("id", 1, "worldSeed", 42, "sharedFishHabitats", "[]"));
        AddVillage(tables, "c", true, 0); Cat(tables, "worker", 0); Cat(tables, "keeper", 1);
        for (int x = -8; x <= 16; x++) for (int z = -8; z <= 8; z++) ((JArray)tables["shared_world_tiles"]).Add(Row("id", x + "," + z, "x", x, "y", z, "type", "meadow", "resources", "{}", "maxResources", "{}"));
        return new JObject { { "Format", "idle-cat-forest-normalized-sqlite" }, { "Version", 1 }, { "Tables", tables } };
    }
    static void AddVillage(JObject tables, string id, bool common, int x)
    {
        var known = new JArray(); for (int a = -8; a <= 16; a++) for (int b = -8; b <= 8; b++) known.Add(Row("x", a, "y", b));
        ((JArray)tables["colonies"]).Add(Row("id", id, "name", "Synthetic " + id, "ownerPlayerId", common ? null : "owner-" + id, "isGlobal", common ? 1 : 0, "anchorX", x, "anchorY", 0, "createdAt", 1000000, "lastTick", 1001000, "runStartedAt", 1000000, "resources", Json(new { food = 100, water = 100, logs = 20 }), "stockpiles", Json(new[] { Row("id", "store", "rect", Row("x1", x, "x2", x + 3, "y1", 0, "y2", 3), "accepts", new JArray(), "contents", Row("food", 100, "water", 100, "logs", 20)) }), "revealedTiles", known.ToString(), "lastTraderDepartedAt", 1000000));
    }
    static JObject Cat(JObject tables, string id, int x)
    { var cat = Row("id", id, "colonyId", "c", "name", "Synthetic " + id, "birthTime", 900000, "ageHours", 48, "needs", Json(new { hunger = 100, thirst = 100, rest = 100, health = 100 }), "position", Json(new { map = "world", x, y = 0 }), "parentIds", "[null,null]"); ((JArray)tables["cats"]).Add(cat); return cat; }
    static JObject Tables(JObject document) => (JObject)document["Tables"];
    static JObject Colony(JObject document) => (JObject)document["Tables"]["colonies"][0];
    static JObject Building(JObject document, string id, string kind, string worker, int x = 0)
    { var b = Row("id", id, "colonyId", "c", "type", kind, "level", 1, "position", Json(new { x, y = 0 }), "isComplete", 1, "constructionProgress", 100, "assignedCatId", worker, "productionQueue", Json(new[] { new { recipe_id = "logs_to_planks", repeat = false } })); ((JArray)Tables(document)["buildings"]).Add(b); return b; }
    static void Pile(JObject document, string id, int x, params object[] goods)
    { var colony = Colony(document); var piles = JArray.Parse((string)colony["stockpiles"]); piles.Add(Row("id", id, "rect", Row("x1", x, "x2", x, "y1", 0, "y2", 0), "accepts", new JArray(), "contents", Row(goods))); colony["stockpiles"] = piles.ToString(); }
    static World Convert(JObject source) => LegacyImport.Convert(source.ToString());
    static World Restart(World world)
    { var path = Path.Combine(Path.GetTempPath(), "forest-import-roundtrip-" + Guid.NewGuid().ToString("N") + ".json"); try { SaveStore.Save(path, world, true); return SaveStore.Load<World>(path); } finally { if (File.Exists(path)) File.Delete(path); } }
    static JObject ReturningTrade(string phase = "returning", bool exactPayment = false)
    {
        var input = Base(); AddVillage(Tables(input), "d", false, 10);
        var payment = exactPayment ? new JArray(Row("id", "d\u001fpayment-tool", "item", "tool:wood:3", "durability", 17, "maxDurability", 42, "location", Row("kind", "caravan"))) : new JArray();
        Colony(input)["villageTradeCaravans"] = Json(new[] { Row("id", "trade", "actorId", "actor", "fromColonyId", "c", "toColonyId", "d", "offeredKind", "food", "offeredAmount", 3, "requestedKind", exactPayment ? "tools" : "logs", "requestedAmount", exactPayment ? 1 : 2, "offeredSources", new JArray(), "requestedSources", new JArray(), "offeredItems", new JArray(), "requestedItems", payment, "outboundRoute", new[] { new { x = 10, y = 0 } }, "returnRoute", new[] { new { x = 4, y = 0 }, new { x = 2, y = 0 }, new { x = 0, y = 0 } }, "nextWaypoint", 0, "phase", phase, "position", new { x = phase == "returning" ? 6 : 0, y = 0 }, "acceptedAt", 1000000, "lastAdvancedAt", 1001000) });
        return input;
    }
    public static void Run(Action<string, Action> test, Action<bool, string> check)
    {
        test("import station outbound destination stock is not mirrored transit", () =>
        {
            var input = Base(); Building(input, "bench", "wood_cutter", "worker", 6)["productionQueue"] = "[]";
            var piles = JArray.Parse((string)Colony(input)["stockpiles"]); piles[0]["contents"]["planks"] = 10; Colony(input)["stockpiles"] = piles.ToString();
            Tables(input)["cats"][0]["carrying"] = Json(new { kind = "planks", amount = 3, jobEndedAt = 1000000, sourceGatherSpot = "station-out|bench|store" });
            var world = Restart(Convert(input)); var v = world.Village("c");
            check(World.Amount(v.Stockpiles.Single().Goods, "planks") == 10 && World.Amount(v.Cats.Single(c => c.Id == "c\u001fworker").Cargo, "planks") == 3 && world.Total(v, "planks") + v.Cats.Sum(c => World.Amount(c.Cargo, "planks")) == 13, "outbound import debited existing destination goods as mirrored transit");
            world.Step(2); world = Restart(world); v = world.Village("c");
            check(World.Amount(v.Stockpiles.Single().Goods, "planks") == 13 && v.Cats.All(c => World.Amount(c.Cargo, "planks") == 0) && world.Validate().Count == 0, "outbound delivery or restart lost finite preexisting destination stock");
        });
        foreach (var equipment in new[] { (Kind: "tool", Resource: "tools"), (Kind: "weapon", Resource: "weapons"), (Kind: "armor", Resource: "armor") })
            test("import unsuffixed station outbound exact " + equipment.Resource + " preserves carrier ownership", () =>
            {
                var input = Base(); Cat(Tables(input), "receiver", 0); Building(input, "bench", "woodworking", "worker", 6)["productionQueue"] = "[]";
                Pile(input, "station-output:bench", 6, equipment.Resource, 1, "planks", 2);
                input["Derived"] = Row("c", Row("PileResourceLimits", Row("store", Row(equipment.Resource, 1))));
                Tables(input)["cats"][0]["carrying"] = Json(new { kind = equipment.Resource, amount = 3, jobEndedAt = 1000000, sourceGatherSpot = "station-out|bench|store" });
                var items = new JArray();
                for (int i = 0; i < 3; i++) items.Add(Row("id", "carried-" + i, "item", equipment.Kind + ":wood:" + i, "durability", 17 + i, "maxDurability", 42 + i, "location", Row("kind", "carrier", "cat_id", "worker")));
                items.Add(Row("id", "equipped", "item", equipment.Kind + ":bone:3", "durability", 21, "maxDurability", 46, "location", Row("kind", "equipped", "cat_id", "worker")));
                items.Add(Row("id", "leftover", "item", equipment.Kind + ":metal:2", "durability", 22, "maxDurability", 47, "location", Row("kind", "station", "building_id", "bench", "compartment", "local_output")));
                Colony(input)["items"] = Json(new { nextSerial = 6, instances = items });
                var world = Restart(Convert(input)); var v = world.Village("c"); var job = v.Jobs.Single(); var store = v.Stockpiles.Single();
                check(job.ItemIds.Count == 3 && v.Items.Where(i => i.Id.StartsWith("c\u001fcarried-", StringComparison.Ordinal)).All(i => i.LocationId == job.Id) && v.Cats.Single(c => c.Id == "c\u001fworker").Cargo.Count == 0, "unsuffixed functional cargo left exact identities on the cat or retained scalar compatibility goods");
                check(World.Amount(v.Buildings.Single().Outputs, equipment.Resource) == 0 && World.Amount(v.Buildings.Single().Outputs, "planks") == 2, "resident exact station output retained its functional scalar mirror or lost unrelated planks");
                check(v.Items.Single(i => i.Id == "c\u001fequipped").LocationId == "c\u001fworker" && v.Cats.Single(c => c.Id == "c\u001fworker").Equipment.SequenceEqual(new[] { "c\u001fequipped" }) && v.Items.Single(i => i.Id == "c\u001fleftover").LocationId == "c\u001fbench", "outbound adoption took equipped or station-owned items");
                world.Step(2); world = Restart(world); v = world.Village("c"); job = v.Jobs.Single();
                check(!job.Completed && job.ItemIds.Count == 2 && v.Items.Count(i => i.LocationId == store.Id) == 1 && job.BlockedReason == "output_storage_full_or_unreachable", "multi-item outbound delivery bypassed one-item receiving capacity");
                var context = new PlayerContext { PlayerId = "fixture-owner", VillageId = v.Id };
                foreach (var receiver in new[] { "keeper", "receiver" })
                {
                    var stored = v.Items.Single(i => i.LocationId == store.Id);
                    check(world.Apply(context, new GameAction { Kind = "EquipItem", CatId = "c\u001f" + receiver, TargetId = stored.Id }).Success, "public equip could not free exact outbound capacity");
                    world.Step(1); world = Restart(world); v = world.Village("c"); job = v.Jobs.Single(j => j.Id == job.Id);
                }
                check(job.Completed && job.ItemIds.Count == 0 && v.Items.Count == 5 && v.Items.Count(i => i.LocationId == store.Id) == 1 && v.Items.Single(i => i.Id == "c\u001fleftover").LocationId == "c\u001fbench" && v.Items.Single(i => i.Id == "c\u001fequipped").LocationId == "c\u001fworker", "outbound completion lost identities, moved station leftovers, or unequipped the worker");
                for (int i = 0; i < 3; i++) { var item = v.Items.Single(item => item.Id == "c\u001fcarried-" + i); check(item.Condition == 17 + i && item.MaxCondition == 42 + i && item.Quality == i, "functional outbound restart changed exact condition or quality"); }
                check(World.Amount(v.Stockpiles.Single().Goods, equipment.Resource) == 0 && World.Amount(v.Buildings.Single().Outputs, equipment.Resource) == 0 && World.Amount(v.Buildings.Single().Outputs, "planks") == 2 && v.Cats.All(c => World.Amount(c.Cargo, equipment.Resource) == 0) && world.Validate().Count == 0, "functional output minted scalar copies or invalid ownership");
            });
        foreach (var kind in new[] { "mug", "trinket" })
            test("import suffixed station outbound " + kind + " replaces refined compatibility cargo", () =>
            {
                var input = Base(); Building(input, "bench", "woodworking", "worker", 6)["productionQueue"] = "[]";
                var piles = JArray.Parse((string)Colony(input)["stockpiles"]); piles[0]["contents"]["stone"] = 420; Colony(input)["stockpiles"] = piles.ToString();
                Tables(input)["cats"][0]["carrying"] = Json(new { kind = "refined", amount = 1, jobEndedAt = 1000000, sourceGatherSpot = "station-out|bench|store|item:variant" });
                Colony(input)["items"] = Json(new
                {
                    nextSerial = 3,
                    instances = new[] {
                    Row("id", "variant", "item", kind + ":gem:2", "durability", 17, "maxDurability", 42, "location", Row("kind", "carrier", "cat_id", "worker")),
                    Row("id", "leftover", "item", kind + ":wood:1", "durability", 19, "maxDurability", 43, "location", Row("kind", "station", "building_id", "bench", "compartment", "local_output")) }
                });
                var world = Restart(Convert(input)); var v = world.Village("c"); var job = v.Jobs.Single(); var store = v.Stockpiles.Single();
                check(job.ItemIds.SequenceEqual(new[] { "c\u001fvariant" }) && v.Items.Single(i => i.Id == "c\u001fvariant").LocationId == job.Id && v.Cats.Single(c => c.Id == "c\u001fworker").Cargo.Count == 0, "suffixed variant retained its refined compatibility cargo");
                world.Step(2); world = Restart(world); v = world.Village("c"); job = v.Jobs.Single();
                check(!job.Completed && job.ItemIds.Count == 1 && job.BlockedReason == "output_storage_full_or_unreachable" && v.Items.Single(i => i.Id == "c\u001fleftover").LocationId == "c\u001fbench", "full variant storage changed exact cargo or station ownership");
                var context = new PlayerContext { PlayerId = "fixture-owner", VillageId = v.Id };
                check(world.Apply(context, new GameAction { Kind = "EnterCatControl", CatId = "c\u001fkeeper" }).Success && world.Apply(context, new GameAction { Kind = "InteractCat", CatId = "c\u001fkeeper", TargetId = store.Id, Resource = "stone", Amount = 1 }).Success, "public pickup could not free variant receiving capacity");
                world.Step(1); world = Restart(world); v = world.Village("c");
                var item = v.Items.Single(i => i.Id == "c\u001fvariant");
                check(item.LocationId == store.Id && item.Condition == 17 && item.MaxCondition == 42 && item.Quality == 2 && v.Jobs.Single(j => j.Id == job.Id).Completed && v.Items.Count == 2 && v.Items.Single(i => i.Id == "c\u001fleftover").LocationId == "c\u001fbench", "variant output recovery changed identity, condition, or station leftovers");
                check(world.Total(v, "refined") == 0 && v.Cats.All(c => World.Amount(c.Cargo, "refined") == 0) && world.Total(v, "stone") + v.Cats.Sum(c => World.Amount(c.Cargo, "stone")) == 420 && world.Validate().Count == 0, "variant output created refined goods or lost finite capacity-release cargo");
            });
        foreach (var outbound in new[] { "none", "scalar", "exact" })
            test("import staffed exact station output waits for physical pickup with " + outbound + " outbound cargo", () =>
            {
                var input = Base(); Building(input, "bench", "woodworking", "worker", 6)["productionQueue"] = "[]";
                var items = new JArray(Row("id", "leftover", "item", "tool:wood:3", "durability", 17, "maxDurability", 42, "location", Row("kind", "station", "building_id", "bench", "compartment", "local_output")));
                if (outbound != "none")
                {
                    Pile(input, "station-output:bench", 6, "planks", 2);
                    Tables(input)["cats"][0]["carrying"] = Json(new { kind = outbound == "exact" ? "tools" : "planks", amount = outbound == "exact" ? 1 : 3, jobEndedAt = 1000000, sourceGatherSpot = "station-out|bench|store" + (outbound == "exact" ? "|item:carried" : "") });
                    if (outbound == "exact") items.Add(Row("id", "carried", "item", "tool:bone:2", "durability", 19, "maxDurability", 43, "location", Row("kind", "carrier", "cat_id", "worker")));
                }
                Colony(input)["items"] = Json(new { nextSerial = 3, instances = items });
                var world = Restart(Convert(input)); var v = world.Village("c"); var station = v.Buildings.Single(); var store = v.Stockpiles.Single(); var worker = v.Cats.Single(c => c.Id == "c\u001fworker");
                var context = new PlayerContext { PlayerId = "fixture-owner", VillageId = v.Id };
                check(v.Items.Single(i => i.Id == "c\u001fleftover").LocationId == station.Id && v.Jobs.All(j => !j.ItemIds.Contains("c\u001fleftover")), "import assigned station-resident exact output to a distant worker before pickup");
                check(worker.BuildingId == station.Id && station.Slots.Single().CatId == worker.Id && station.Queue.Count == 0, "import lost the preassigned worker or empty queue");
                if (outbound == "none")
                {
                    check(world.Apply(context, new GameAction { Kind = "CreateZone", Resource = "avoid", Position = station.Position, End = station.Position }).Success, "public avoid zone could not block station access");
                    var zoneId = v.Stockpiles.Single(p => p.Kind == "zone_avoid").Id;
                    for (int restart = 0; restart < 2; restart++)
                    {
                        world.Step(4);
                        check(v.Items.Single().LocationId == station.Id && !v.Jobs.Any(j => j.Kind == "production") && World.Amount(v.Stockpiles.Single(p => p.Id == store.Id).Goods, "tools") == 0, "blocked station output changed ownership or delivered before pickup");
                        world = Restart(world); v = world.Village("c");
                    }
                    check(world.Apply(context, new GameAction { Kind = "RemoveZone", TargetId = zoneId }).Success, "public zone removal could not reopen the pickup route");
                }
                if (outbound != "none")
                {
                    var outgoing = v.Jobs.Single();
                    check(outgoing.Local.Count == 0 && World.Amount(station.Outputs, "planks") == 2, "outgoing cargo job prematurely adopted station leftovers");
                    world.Step(1);
                    check(outgoing.Completed && (outbound == "exact" ? v.Items.Single(i => i.Id == "c\u001fcarried").LocationId == store.Id : World.Amount(store.Goods, "planks") == 3), "already-carried output was forced back to the distant station before delivery");
                    check(v.Items.Single(i => i.Id == "c\u001fleftover").LocationId == station.Id && World.Amount(station.Outputs, "planks") == 2, "outgoing delivery teleported station leftovers");
                }
                world = Restart(world); v = world.Village("c"); station = v.Buildings.Single(); worker = v.Cats.Single(c => c.Id == worker.Id);
                check(world.Apply(context, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = worker.Id }).Success, "public staffing could not continue the imported station assignment");
                world.Step(1);
                check(v.Items.Single(i => i.Id == "c\u001fleftover").LocationId == station.Id, "distant assigned worker skipped the physical pickup walk");
                world = Restart(world); v = world.Village("c"); station = v.Buildings.Single(); worker = v.Cats.Single(c => c.Id == worker.Id);
                bool pickedUp = false;
                for (int tick = 0; tick < 60 && v.Items.Single(i => i.Id == "c\u001fleftover").LocationId != store.Id; tick++)
                {
                    world.Step(1);
                    var item = v.Items.Single(i => i.Id == "c\u001fleftover");
                    if (!pickedUp && item.LocationId != station.Id)
                    {
                        var delivery = v.Jobs.Single(j => j.Id == item.LocationId);
                        check(worker.Position.Equals(station.Position) && delivery.CatId == worker.Id && delivery.ItemIds.Contains(item.Id), "exact output changed owner before the worker physically reached the station");
                        pickedUp = true;
                    }
                }
                check(pickedUp && v.Items.Single(i => i.Id == "c\u001fleftover").LocationId == store.Id && v.Jobs.Where(j => j.Kind == "production").All(j => j.Completed), "station pickup did not finish physical delivery");
                world = Restart(world); v = world.Village("c");
                var leftover = v.Items.Single(i => i.Id == "c\u001fleftover");
                check(leftover.LocationId == store.Id && leftover.Condition == 17 && leftover.MaxCondition == 42 && leftover.Quality == 3, "pickup or restart changed exact output identity or condition");
                if (outbound == "exact") check(v.Items.Single(i => i.Id == "c\u001fcarried").Condition == 19 && v.Items.Single(i => i.Id == "c\u001fcarried").MaxCondition == 43, "outbound recovery changed carried equipment condition");
                check(v.Items.Count == (outbound == "exact" ? 2 : 1) && World.Amount(v.Stockpiles.Single().Goods, "planks") == (outbound == "none" ? 0 : outbound == "scalar" ? 5 : 2) && World.Amount(v.Stockpiles.Single().Goods, "tools") == 0 && world.Validate().Count == 0, "mixed output recovery lost or duplicated finite goods");
            });
        test("import inbound station cargo keeps its target beside pending exact output", () =>
        {
            var input = Base(); Building(input, "bench", "wood_cutter", "worker", 6)["productionProgress"] = 599;
            Pile(input, "station-input:bench", 6, "logs", 2); Pile(input, "station-transit:bench", 6, "logs", 3);
            Tables(input)["cats"][0]["carrying"] = Json(new { kind = "logs", amount = 3, jobEndedAt = 1000000, sourceGatherSpot = "station-in|bench|station-transit:bench" });
            Colony(input)["items"] = Json(new { nextSerial = 2, instances = new[] { Row("id", "leftover", "item", "tool:wood:3", "durability", 17, "maxDurability", 42, "location", Row("kind", "station", "building_id", "bench", "compartment", "local_output")) } });
            var world = Restart(Convert(input)); var v = world.Village("c"); var station = v.Buildings.Single(); var worker = v.Cats.Single(c => c.Id == "c\u001fworker");
            var inbound = v.Jobs.Single();
            check(inbound.Kind == "production" && inbound.TargetId == station.Id && inbound.Phase == "input_delivery" && inbound.RecipeId == "logs_to_planks" && inbound.Progress == 599, "pending output discarded the inbound station target or active recipe");
            check(World.Amount(inbound.Local, "logs") == 2 && World.Amount(worker.Cargo, "logs") == 3 && v.Items.Single().LocationId == station.Id && inbound.ItemIds.Count == 0, "inbound adoption mixed pending outputs with recipe inputs");
            world.Step(1);
            check(v.Items.Single().LocationId == station.Id && World.Amount(worker.Cargo, "logs") == 3, "inbound cargo or pending output skipped the trip to the station");
            world = Restart(world); v = world.Village("c");
            for (int tick = 0; tick < 100; tick++) world.Step(1);
            var item = v.Items.Single();
            check(item.Id == "c\u001fleftover" && item.LocationId == "c\u001fstore" && item.Condition == 17 && item.MaxCondition == 42 && item.Quality == 3, "inbound work stranded or changed the pending exact output");
            check(world.Total(v, "planks") == 1 && world.Total(v, "logs") == 20 && v.Jobs.Where(j => j.Kind == "production").All(j => j.Completed) && World.Amount(v.Stockpiles.Single().Goods, "tools") == 0 && world.Validate().Count == 0, "inbound continuation duplicated finite inputs or failed either output delivery");
        });
        foreach (var workerCount in new[] { 1, 2 })
            test("import unstaffed exact station output resumes with " + workerCount + " workers", () =>
            {
                var input = Base(); Cat(Tables(input), "receiver", 0);
                Building(input, "bench", "woodworking", "", 6)["productionQueue"] = "[]";
                Colony(input)["upgradeTree"] = Json(new { ownedNodeIds = new[] { "woodworking_crews" }, researchPoints = 0 });
                input["Derived"] = Row("c", Row("PileResourceLimits", Row("store", Row("tools", 1))));
                Colony(input)["items"] = Json(new
                {
                    nextSerial = 3,
                    instances = new[]
                    {
                        Row("id", "output-a", "item", "tool:wood:3", "durability", 17, "maxDurability", 42, "location", Row("kind", "station", "building_id", "bench", "compartment", "local_output")),
                        Row("id", "output-b", "item", "tool:wood:2", "durability", 19, "maxDurability", 43, "location", Row("kind", "station", "building_id", "bench", "compartment", "local_output"))
                    }
                });
                var world = Restart(Convert(input)); var v = world.Village("c"); var station = v.Buildings.Single(); var store = v.Stockpiles.Single();
                var context = new PlayerContext { PlayerId = "fixture-owner", VillageId = v.Id };
                check(station.Queue.Count == 0 && station.Slots.All(s => s.CatId == "" && s.Queue.Count == 0) && v.Jobs.Count == 0 && v.Items.All(i => i.LocationId == station.Id), "unstaffed import did not retain exact station output with an empty queue");
                foreach (var worker in new[] { "worker", "keeper" }.Take(workerCount))
                    check(world.Apply(context, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = "c\u001f" + worker }).Success, "public staffing failed");
                world.Step(1);
                check(v.Items.All(i => i.LocationId == station.Id), "distant worker picked up exact output without reaching the station");
                for (int tick = 0; tick < 40; tick++)
                {
                    world.Step(1);
                    check(v.Items.Count == 2 && v.Items.Count(i => i.LocationId == store.Id) <= 1, "station recovery duplicated items or exceeded receiving capacity");
                    check(v.Jobs.Where(j => j.Kind == "production").SelectMany(j => j.ItemIds).Distinct().Count() == v.Jobs.Where(j => j.Kind == "production").Sum(j => j.ItemIds.Count), "workers double-claimed exact output");
                }
                var delivery = v.Jobs.SingleOrDefault(j => j.Kind == "production");
                check(delivery != null && !delivery.Completed && delivery.ItemIds.Count == 1 && v.Items.Count(i => i.LocationId == store.Id) == 1, "staffing an empty queue stranded the imported exact outputs instead of delivering up to capacity");
                var deliveryId = delivery.Id;
                check(v.Items.Single(i => i.LocationId != store.Id).LocationId == deliveryId && delivery.BlockedReason == "output_storage_full_or_unreachable", "full receiving storage lost pending output ownership");
                world = Restart(world); v = world.Village("c"); delivery = v.Jobs.Single(j => j.Id == deliveryId);
                world.Step(2);
                check(!delivery.Completed && delivery.ItemIds.Count == 1 && v.Jobs.Count(j => j.Kind == "production") == 1, "restart duplicated the station delivery or discarded pending output");
                var first = v.Items.Single(i => i.LocationId == store.Id);
                check(world.Apply(context, new GameAction { Kind = "EquipItem", CatId = "c\u001freceiver", TargetId = first.Id }).Success, "public equip did not free receiving capacity");
                for (int tick = 0; tick < 40 && !delivery.Completed; tick++) world.Step(1);
                check(delivery.Completed && delivery.ItemIds.Count == 0, "freeing capacity did not finish the adopted output delivery");
                world = Restart(world); v = world.Village("c");
                var a = v.Items.Single(i => i.Id == "c\u001foutput-a"); var b = v.Items.Single(i => i.Id == "c\u001foutput-b");
                check(v.Items.Count == 2 && v.Items.Single(i => i.Id == first.Id).LocationId == "c\u001freceiver" && v.Items.Single(i => i.Id != first.Id).LocationId == store.Id, "delivery restart changed exact identities or final ownership");
                check(a.Condition == 17 && a.MaxCondition == 42 && a.Quality == 3 && b.Condition == 19 && b.MaxCondition == 43 && b.Quality == 2, "output adoption changed item condition or quality");
                check(v.Stockpiles.All(p => World.Amount(p.Goods, "tools") == 0) && v.Jobs.Count(j => j.Kind == "production") == 1 && world.Validate().Count == 0, "output recovery created scalar duplicates, duplicate work, or invalid state");
            });
        test("import multiple exact station outputs respect receiving capacity", () =>
        {
            var input = Base();
            Building(input, "bench", "woodworking", "worker", 2)["productionQueue"] = Json(new[] { new { recipe_id = "planks_and_blocks_to_tools", repeat = false } });
            Pile(input, "armory", 5, "weapons", 0);
            var piles = JArray.Parse((string)Colony(input)["stockpiles"]);
            piles[0]["accepts"] = new JArray("food", "water", "logs", "tools"); piles[1]["accepts"] = new JArray("weapons"); Colony(input)["stockpiles"] = piles.ToString();
            input["Derived"] = Row("c", Row("PileResourceLimits", Row("store", Row("tools", 1), "armory", Row("weapons", 1))));
            Colony(input)["items"] = Json(new
            {
                nextSerial = 4,
                instances = new[]
                {
                    Row("id", "output-a", "item", "tool:wood:3", "durability", 17, "maxDurability", 42, "location", Row("kind", "station", "building_id", "bench", "compartment", "local_output")),
                    Row("id", "output-b", "item", "tool:wood:2", "durability", 19, "maxDurability", 43, "location", Row("kind", "station", "building_id", "bench", "compartment", "local_output")),
                    Row("id", "output-c", "item", "weapon:bone:1", "durability", 23, "maxDurability", 47, "location", Row("kind", "station", "building_id", "bench", "compartment", "local_output"))
                }
            });
            var world = Restart(Convert(input)); var v = world.Village("c"); var store = v.Stockpiles.Single(p => p.Id == "c\u001fstore"); var armory = v.Stockpiles.Single(p => p.Id == "c\u001farmory");
            var station = v.Buildings.Single();
            check(v.Jobs.Count == 0 && v.Items.Count == 3 && v.Items.All(i => i.LocationId == station.Id), "import moved exact station output before physical pickup");
            for (int tick = 0; tick < 20 && v.Jobs.Count == 0; tick++)
            {
                check(v.Items.All(i => i.LocationId == station.Id), "unclaimed exact output left the station");
                world.Step(1);
            }
            var job = v.Jobs.Single(j => j.Kind == "production");
            check(v.Cats.Single(c => c.Id == job.CatId).Position.Equals(station.Position) && job.Phase == "output_delivery" && job.ItemIds.Count == 3 && v.Items.All(i => i.LocationId == job.Id), "physical pickup did not transfer the complete exact output batch to its worker");
            for (int tick = 0; tick < 8; tick++)
            {
                world.Step(1);
                check(v.Items.Count(i => i.LocationId == store.Id && i.Kind == "tool") <= 1, "imported station delivered multiple exact items into one-item receiving capacity");
                check(v.Items.Count == 3 && v.Items.All(i => i.LocationId == store.Id || i.LocationId == armory.Id || i.LocationId == job.Id), "capacity blocking lost exact output ownership");
                check(!v.Items.Any(i => i.LocationId == store.Id && i.Kind != "tool" || i.LocationId == armory.Id && i.Kind != "weapon"), "mixed station output bypassed the receiving pile's acceptance filter");
            }
            world = Restart(world); v = world.Village("c");
            check(v.Items.Single(i => i.Id == "c\u001foutput-a").Condition == 17 && v.Items.Single(i => i.Id == "c\u001foutput-a").MaxCondition == 42 && v.Items.Single(i => i.Id == "c\u001foutput-b").Condition == 19 && v.Items.Single(i => i.Id == "c\u001foutput-b").MaxCondition == 43, "blocked output restart changed exact identity or condition");
            check(v.Stockpiles.All(p => World.Amount(p.Goods, "tools") + World.Amount(p.Goods, "weapons") == 0) && world.Validate().Count == 0, "blocked exact outputs became scalar goods or invalid ownership");
            job = v.Jobs.Single(j => j.Kind == "production");
            check(!job.Completed && job.ItemIds.Count > 0, "partially delivered station job finished with pending exact items");
            var first = v.Items.Single(i => i.LocationId == store.Id && i.Kind == "tool");
            var context = new PlayerContext { PlayerId = "fixture-owner", VillageId = v.Id };
            check(world.Apply(context, new GameAction { Kind = "EquipItem", CatId = "c\u001fkeeper", TargetId = first.Id }).Success, "public equip failed to free receiving capacity for the pending output");
            world = Restart(world); v = world.Village("c"); job = v.Jobs.Single(j => j.Kind == "production");
            for (int tick = 0; tick < 60 && !job.Completed; tick++)
            {
                world.Step(1);
                check(v.Items.Count(i => i.LocationId == store.Id) <= 1 && v.Items.Count(i => i.LocationId == armory.Id) <= 1, "continued mixed output exceeded per-kind receiving capacity");
                check(!v.Items.Any(i => i.LocationId == store.Id && i.Kind != "tool" || i.LocationId == armory.Id && i.Kind != "weapon"), "continued mixed output entered incompatible storage");
            }
            check(job.Completed && job.ItemIds.Count == 0, "freeing one exact-item slot did not allow all remaining outputs to finish");
            world = Restart(world); v = world.Village("c");
            var second = v.Items.Single(i => i.Kind == "tool" && i.Id != first.Id); var weapon = v.Items.Single(i => i.Id == "c\u001foutput-c");
            check(v.Items.Count == 3 && v.Items.Single(i => i.Id == first.Id).LocationId == "c\u001fkeeper" && second.LocationId == store.Id && weapon.LocationId == armory.Id, "completed mixed output restart changed identities or locations");
            check(v.Items.Single(i => i.Id == "c\u001foutput-a").Condition == 17 && v.Items.Single(i => i.Id == "c\u001foutput-a").MaxCondition == 42 && v.Items.Single(i => i.Id == "c\u001foutput-b").Condition == 19 && v.Items.Single(i => i.Id == "c\u001foutput-b").MaxCondition == 43 && weapon.Condition == 23 && weapon.MaxCondition == 47, "completed output recovery changed condition");
            check(v.Stockpiles.All(p => World.Amount(p.Goods, "tools") + World.Amount(p.Goods, "weapons") == 0) && world.Validate().Count == 0, "completed exact outputs became duplicate scalar goods or invalid ownership");
        });
        foreach (var equipment in new[] { (Kind: "tool", Resource: "tools"), (Kind: "weapon", Resource: "weapons"), (Kind: "armor", Resource: "armor") })
            test("import exact equipment per-kind capacity " + equipment.Resource, () =>
            {
                var input = Base();
                input["Derived"] = Row("c", Row("PileResourceLimits", Row("store", Row("tools", 1, "weapons", 1, "armor", 1))));
                var piles = JArray.Parse((string)Colony(input)["stockpiles"]);
                foreach (var resource in new[] { "tools", "weapons", "armor" }) piles[0]["contents"][resource] = 1;
                Colony(input)["stockpiles"] = piles.ToString();
                var items = new JArray(
                    Row("id", "stored", "item", equipment.Kind + ":metal:3", "durability", 17, "maxDurability", 42, "location", Row("kind", "stockpile", "stockpile_id", "store")),
                    Row("id", "equipped", "item", equipment.Kind + ":metal:2", "durability", 9, "maxDurability", 31, "location", Row("kind", "equipped", "cat_id", "worker")));
                foreach (var kind in new[] { "tool", "weapon", "armor" }.Where(k => k != equipment.Kind))
                    items.Add(Row("id", "other-" + kind, "item", kind + ":wood:1", "durability", 13, "maxDurability", 27, "location", Row("kind", "stockpile", "stockpile_id", "store")));
                Colony(input)["items"] = Json(new { nextSerial = 5, instances = items });
                var world = Restart(Convert(input)); var v = world.Village("c"); var store = v.Stockpiles.Single();
                var context = new PlayerContext { PlayerId = "fixture-owner", VillageId = v.Id };
                check(v.Items.Count == 4 && new[] { "tools", "weapons", "armor" }.All(r => World.Amount(store.Goods, r) == 0), "import did not replace scalar equipment counters with exact identities");
                check(store.ResourceLimits.Count == 3 && store.ResourceLimits.All(l => l.Amount == 1), "import lost the one-item per-kind limits");
                for (int attempt = 0; attempt < 2; attempt++)
                {
                    var before = Json(v);
                    check(!world.Apply(context, new GameAction { Kind = "UnequipItem", CatId = "c\u001fworker", TargetId = "c\u001fequipped" }).Success, "full imported " + equipment.Resource + " limit admitted a second exact item");
                    check(Json(v) == before, "rejected unequip changed item ownership or condition");
                    world = Restart(world); v = world.Village("c");
                }
                check(world.Apply(context, new GameAction { Kind = "EquipItem", CatId = "c\u001fkeeper", TargetId = "c\u001fstored" }).Success, "public equip did not free the occupied per-kind slot");
                check(world.Apply(context, new GameAction { Kind = "UnequipItem", CatId = "c\u001fworker", TargetId = "c\u001fequipped" }).Success, "unrelated exact item kinds incorrectly consumed the freed slot");
                world = Restart(world); v = world.Village("c"); store = v.Stockpiles.Single();
                var stored = v.Items.Single(i => i.Id == "c\u001fstored"); var returned = v.Items.Single(i => i.Id == "c\u001fequipped");
                check(v.Items.Count == 4 && stored.LocationId == "c\u001fkeeper" && returned.LocationId == store.Id && v.Cats.Single(c => c.Id == "c\u001fkeeper").Equipment.Contains(stored.Id), "restart changed exact item identity or location");
                check(stored.Condition == 17 && stored.MaxCondition == 42 && stored.Quality == 3 && returned.Condition == 9 && returned.MaxCondition == 31 && returned.Quality == 2, "capacity rejection or transfer changed equipment condition");
                check(new[] { "tools", "weapons", "armor" }.All(r => World.Amount(store.Goods, r) == 0), "capacity recovery invented scalar equipment counters");
                check(!world.Apply(context, new GameAction { Kind = "UnequipItem", CatId = "c\u001fkeeper", TargetId = stored.Id }).Success, "reloaded receiving pile exceeded its exact-item limit");
                check(world.Validate().Count == 0, "capacity recovery violated world invariants");
            });
        test("import keeps colony coordinates recipe queues and refunded retired research", () =>
        {
            var input = Base(); Colony(input)["anchorX"] = 6; Tables(input)["cats"][0]["position"] = Json(new { map = "colony", x = 2.25, y = 2.5 }); Colony(input)["upgradeTree"] = Json(new { ownedNodeIds = new[] { "den_stores", "basic_tools" }, researchPoints = 3 });
            Building(input, "bench", "wood_cutter", "worker"); var world = Convert(input); var v = world.Villages[0];
            check(v.Cats[0].X == 8.25 && v.Cats[0].Z == 2.5, "colony coordinate not rebased"); check(v.Buildings[0].Queue.Single().RecipeId == "logs_to_planks", "maintained snakecase recipe lost"); check(v.ResearchPoints == 17 && v.Research.SequenceEqual(new[] { "basic_tools" }), "retired points were not refunded exactly");
        });
        test("import resumes inbound station cargo and consumes the finite recipe once", () =>
        {
            var input = Base(); Building(input, "bench", "wood_cutter", "worker", 2)["productionProgress"] = 599; Pile(input, "station-input:bench", 2, "logs", 2); Pile(input, "station-transit:bench", 2, "logs", 3);
            Tables(input)["cats"][0]["carrying"] = Json(new { kind = "logs", amount = 3, jobEndedAt = 1000000, sourceGatherSpot = "station-in|bench|station-transit:bench" });
            var world = Restart(Convert(input)); var village = world.Villages[0]; var job = village.Jobs.Single(j => j.Kind == "production");
            check(World.Amount(job.Local, "logs") == 2 && World.Amount(village.Cats[0].Cargo, "logs") == 3, "station transit double-counted or lost"); check(village.Stockpiles.All(p => p.Kind != "legacy_local"), "hidden transit remained spendable");
            for (int i = 0; i < 20 && !job.Completed; i++) world.Step(1);
            check(job.Completed && world.Total(village, "planks") == 1, $"station failed exactly one output delivery: {job.Phase}, {job.Progress}, {job.BlockedReason}, {village.Cats[0].Goal}, planks={world.Total(village, "planks")}"); check(world.Total(village, "logs") == 20, "inbound goods charged visible source twice");
        });
        test("import resumes exact outbound equipment and construction reservations", () =>
        {
            var input = Base(); Building(input, "bench", "wood_cutter", "worker", 2); var c = Tables(input)["cats"][0]; c["carrying"] = Json(new { kind = "tools", amount = 1, jobEndedAt = 1000000, sourceGatherSpot = "station-out|bench|store|item:tool-a" });
            Colony(input)["items"] = Json(new { nextSerial = 2, instances = new[] { new { id = "tool-a", item = "tool:wood:3", durability = 17, maxDurability = 42, location = new { kind = "carrier", cat_id = "worker" }, credited = false, autoIssued = false } } });
            var construction = Building(input, "scaffold", "den", "", 4); construction["isComplete"] = 0; construction["constructionProgress"] = 0; construction["productionQueue"] = "[]"; construction["constructionCargo"] = Json(new { requiredLumber = 2, requiredPlanks = 0, requiredBlocks = 0, deliveredLumber = 1, deliveredPlanks = 0, deliveredBlocks = 0, consumed = false, reservations = new[] { new { sourceStockpileId = "timber", kind = "lumber", amount = 1 } } }); Pile(input, "timber", 0, "lumber", 1); Pile(input, "construction-input:scaffold", 4, "lumber", 1);
            ((JArray)Tables(input)["jobs"]).Add(Row("id", "builder-job", "colonyId", "c", "kind", "build_house", "status", "active", "requestedByType", "player", "assignedCatId", "keeper", "baseDurationSec", 100, "speedMultiplier", 1, "yieldMultiplier", 1, "metadata", Json(new { kind = "construction", phase = "construct_house", buildingType = "den", buildingId = "scaffold", site = new { x = 4, y = 0 } })));
            var world = Restart(Convert(input)); var v = world.Villages[0]; check(world.Reservations.Single().PileId == "c\u001ftimber", "construction claim source lost"); check(World.Amount(v.Buildings.Single(b => b.Kind == "den").Inputs, "lumber") == 1, "construction local material doubled");
            world.Step(8); var item = v.Items.Single(); check(item.Id == "c\u001ftool-a" && item.LocationId == "c\u001fstore" && item.Condition == 17 && item.MaxCondition == 42 && item.Quality == 3, $"outbound finite identity changed: {item.Id}, {item.LocationId}, {item.Condition}, {v.Cats[0].Goal}, {v.Cats[0].BlockedReason}"); check(v.Cats[0].Cargo.Count == 0, "delivered item still carried");
        });
        test("import retains migration farm accountant and weighted election continuation", () =>
        {
            var input = Base(); Building(input, "tent", "accounting_tent", "keeper")["productionQueue"] = "[]"; Colony(input)["officers"] = Json(new { accountant = "keeper" });
            Colony(input)["migrationState"] = Json(new { lastEvaluatedCohortBucket = 4, probationaryMigrants = new[] { new { id = "worker", arrivedGameMinute = 5, housingDeadlineGameMinute = 2165, phase = "arriving", routeExterior = new[] { 0, 7 } } } });
            Colony(input)["farms"] = Json(new[] { new { id = "farm", rect = new { x1 = 3, y1 = 3, x2 = 3, y2 = 3 }, crop = "grain", plantedAt = 1000000, stage = "mature", growthHours = 12.5, fertility = 0.75, workerId = (string)null, workPhase = "waiting_for_worker", pendingOutput = 4 } });
            Colony(input)["stockLedger"] = Json(new { pileReports = new Dictionary<string, object> { { "store", new { reported = new { food = 3 }, lastCounted = 900000 } } }, activeRound = new { workerId = "keeper", tentId = "tent", phase = "counting", targetStockpileId = "store", pendingStockpileIds = Array.Empty<string>(), unreachableStockpileIds = Array.Empty<string>(), dwellElapsedMs = 4000, topologySignature = 12 }, stewardManagedPiles = new { } });
            ((JArray)Tables(input)["elections"]).Add(Row("id", "election", "colonyId", "c", "kind", "scheduled", "runtimeKind", "scheduled", "startedAt", 1000000, "endsAt", 1005000, "candidateCatIds", "[\"worker\",\"keeper\"]"));
            ((JArray)Tables(input)["votes"]).Add(Row("id", "vote", "colonyId", "c", "electionId", "election", "playerId", "voter", "catId", "keeper", "weight", 2.5));
            var world = Restart(Convert(input)); var v = world.Villages[0]; check(v.Cats[0].ProbationUntil == 2165 * 60 && v.Cats[0].MigrationExterior.Equals(new Int2(0, 7)), "migration phase/deadline changed"); check(v.Farms[0].Growth == 45000 && v.Farms[0].Harvest == 4 && v.Farms[0].Fertility == 0.75, "farm biological progress changed"); check(v.Accounting.DwellSeconds == 4 && v.Stockpiles[0].Report.Single().Amount == 3, "accounting freshness reset"); check(v.Election.Votes.Single().Weight == 2.5, "weighted ballot lost"); world.Step(5); check(v.LeaderId == "c\u001fkeeper", "saved election did not resolve");
        });
        test("import resumes loaded transport without a second source debit", () =>
        {
            var input = Base(); Pile(input, "destination", 3, "logs", 0);
            Colony(input)["transportState"] = Json(new { trackTiles = new[] { new { x = 0, y = 0 }, new { x = 1, y = 0 }, new { x = 2, y = 0 }, new { x = 3, y = 0 } }, docks = new { }, vehicles = new[] { new { id = "wagon", mode = "rail", home = new { x = 0, y = 0 }, assignedRouteId = "route" } }, projects = Array.Empty<object>(), routes = new[] { new { id = "route", mode = "rail", sourceStockpileId = "store", destinationStockpileId = "destination", resource = "logs", amount = 2, assignedCatId = "worker", phase = "outbound", path = new[] { new { x = 0, y = 0 }, new { x = 1, y = 0 }, new { x = 2, y = 0 }, new { x = 3, y = 0 } }, pathIndex = 1, segmentProgress = 0.25, cargoLoaded = 2, vehicleId = "wagon", position = new { x = 1, y = 0 }, repeat = false } } });
            var world = Restart(Convert(input)); var v = world.Villages[0]; check(World.Amount(v.Vehicles.Single().Cargo, "logs") == 2, "loaded transport lost cargo"); world.Step(12); check(World.Amount(v.Stockpiles.Single(p => p.Id == "c\u001fdestination").Goods, "logs") == 2, "transport cargo did not deliver"); check(World.Amount(v.Stockpiles[0].Goods, "logs") == 20, "loaded transport debited source again");
        });
        test("import returning barter does not replay delivered side", () =>
        {
            var world = Restart(Convert(ReturningTrade())); world.Step(5); check(world.TradeOffers.Single().Status == "completed", "returning caravan did not complete: " + world.TradeOffers.Single().Status); check(world.Total(world.Village("c"), "logs") == 22, "requested escrow failed delivery"); check(world.Total(world.Village("d"), "food") == 100, "previously delivered offered side was duplicated");
        });
        foreach (var phase in new[] { "returning", "waiting_at_source" })
            foreach (var exact in new[] { false, true })
                test("import delivered trade rejects cancellation and preserves " + (exact ? "exact" : "scalar") + " payment while " + phase, () =>
                {
                    var input = ReturningTrade(phase, exact);
                    var piles = JArray.Parse((string)Colony(input)["stockpiles"]); piles[0]["contents"]["stone"] = 420; Colony(input)["stockpiles"] = piles.ToString();
                    var world = Restart(Convert(input)); var trade = world.TradeOffers.Single(); var sender = world.Village("c"); var recipient = world.Village("d");
                    var senderContext = new PlayerContext { PlayerId = "fixture-sender", VillageId = sender.Id };
                    var recipientContext = new PlayerContext { PlayerId = "owner-d", VillageId = recipient.Id };
                    check(world.Apply(senderContext, new GameAction { Kind = "EnterCatControl", CatId = sender.Cats[0].Id }).Success, "fixture sender could not wait beside its storage");
                    check(trade.OfferedDelivered, "fixture did not retain completed outward delivery");
                    var beforeTrade = Json(trade); var beforeSender = Json(sender); var beforeRecipient = Json(recipient);
                    foreach (var controller in new[] { recipientContext, senderContext })
                        check(!world.Apply(controller, new GameAction { Kind = "CancelVillageTrade", TargetId = trade.Id }).Success, "delivered trade cancellation refunded the recipient's earned payment");
                    check(Json(trade) == beforeTrade && Json(sender) == beforeSender && Json(recipient) == beforeRecipient, "rejected cancellation changed delivery or village state");
                    if (phase == "returning")
                    {
                        world.Step(1);
                        check(trade.Status == "returning" && trade.X == 6 && world.Total(sender, exact ? "tools" : "logs") == (exact ? 0 : 20), "returning payment skipped its physical route");
                    }
                    world.Step(8);
                    check(trade.Status == "unloading", "full source storage did not retain the return payment");
                    world = Restart(world); trade = world.TradeOffers.Single(); sender = world.Village("c"); recipient = world.Village("d");
                    check(!world.Apply(recipientContext, new GameAction { Kind = "CancelVillageTrade", TargetId = trade.Id }).Success && trade.Status == "unloading", "restart allowed a waiting payment to be refunded");
                    if (exact) check(trade.RequestedItems.Single().Id == "d\u001fpayment-tool" && trade.RequestedItems.Single().Condition == 17 && trade.RequestedItems.Single().MaxCondition == 42, "waiting escrow lost exact item identity or condition");
                    var cat = sender.Cats[0]; var store = sender.Stockpiles.Single();
                    check(world.Apply(senderContext, new GameAction { Kind = "EnterCatControl", CatId = cat.Id }).Success, "sender could not approach its storage");
                    check(world.Apply(senderContext, new GameAction { Kind = "InteractCat", CatId = cat.Id, TargetId = store.Id, Resource = "stone", Amount = 2 }).Success, "sender could not free finite receiving capacity");
                    world.Step(2);
                    check(trade.Status == "completed" && world.Total(sender, exact ? "tools" : "logs") == (exact ? 1 : 22), "earned return payment did not arrive after storage was freed");
                    check(world.Total(recipient, "food") == 100 && world.Total(recipient, exact ? "tools" : "logs") == (exact ? 0 : 20), "recipient received duplicate goods or retained its payment");
                    if (exact)
                    {
                        var item = sender.Items.Single(i => i.Id == "d\u001fpayment-tool");
                        check(item.Condition == 17 && item.MaxCondition == 42 && item.Quality == 3 && item.LocationId == store.Id && trade.RequestedItems.Count == 0, "completed exact payment changed identity or remained in escrow");
                    }
                    world = Restart(world); world.Step(2);
                    check(world.Total(world.Village("c"), exact ? "tools" : "logs") == (exact ? 1 : 22), "completed payment replayed after restart");
                });
        test("import completes only the saved frontier tile and exact boundary", () =>
        {
            var input = Base(); Colony(input)["claimedTiles"] = Json(new[] { new { x = 0, y = 0 } });
            ((JArray)Tables(input)["jobs"]).Add(Row("id", "expansion", "colonyId", "c", "kind", "expand_village", "status", "active", "requestedByType", "player", "assignedCatId", "worker", "baseDurationSec", 2, "speedMultiplier", 1, "yieldMultiplier", 1, "metadata", Json(new { kind = "expansion", target = new { x = 1, y = 0 }, accepted = true, sourceBuildJobId = (string)null, wallWorkMs = 1000 })));
            input["Derived"] = Row("c", Row("BoundaryEdges", new JArray(), "Expansions", Row("expansion", Row("Agricultural", false, "BoundaryEdges", new[] { Row("From", Row("x", 1, "y", 0), "To", Row("x", 2, "y", 0)) }))));
            var world = Restart(Convert(input)); var v = world.Village("c"); world.Step(10);
            check(v.ClaimedTiles.Count == 2 && v.ClaimedTiles.Contains(new Int2(1, 0)), "one-tile legacy expansion grew a whole ring");
            check(v.BoundaryEdges.Count == 1 && !world.Crossable(new Int2(1, 0), new Int2(2, 0)), "saved completion boundary not installed");
        });
        test("import keeps simultaneous election and five-player vote kick", () =>
        {
            var input = Base(); Colony(input)["leaderId"] = "worker";
            ((JArray)Tables(input)["elections"]).Add(Row("id", "leadership", "colonyId", "c", "runtimeKind", "scheduled", "startedAt", 1000000, "endsAt", 1100000, "candidateCatIds", "[]"));
            ((JArray)Tables(input)["elections"]).Add(Row("id", "kick", "colonyId", "c", "runtimeKind", "vote_kick", "startedAt", 1000000, "endsAt", 1005000, "candidateCatIds", "[]"));
            for (int i = 0; i < 5; i++) ((JArray)Tables(input)["votes"]).Add(Row("id", "vote-" + i, "colonyId", "c", "electionId", "kick", "playerId", "voter-" + i, "catId", "worker", "weight", 1));
            var world = Restart(Convert(input)); var v = world.Village("c"); check(v.Election != null && v.KickPetition != null, "concurrent governance owner lost"); world.Step(5);
            check(v.LeaderId == "c\u001fkeeper" && v.Election.Id == "c\u001fleadership" && v.KickPetition == null, "five-player kick did not preserve pending election");
        });
        test("import mature farm retains its remaining biological work", () =>
        {
            var input = Base(); Colony(input)["farms"] = Json(new[] { new { id = "farm", rect = new { x1 = 0, y1 = 0, x2 = 0, y2 = 0 }, crop = "grain", plantedAt = 1000000, stage = "mature", growthHours = 12.5, fertility = 0.75, workerId = "worker", workPhase = "tending", pendingOutput = 0 } });
            var world = Restart(Convert(input)); var v = world.Village("c"); world.Step(10);
            check(v.Farms[0].Growth >= 45000 && v.Farms[0].Harvest == 0 && v.Stockpiles.Sum(p => World.Amount(p.Goods, "grain")) == 0, "mature saved farm harvested before its24hour cycle finished");
        });
    }
}
