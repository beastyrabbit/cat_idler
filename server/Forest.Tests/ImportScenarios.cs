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
            var input = Base(); Building(input, "bench", "wood_cutter", "worker", 2); var c = Tables(input)["cats"][0]; c["carrying"] = Json(new { kind = "tools", amount = 1, jobEndedAt = 1000000, sourceGatherSpot = "station-out|bench|station-output:bench|item:tool-a" });
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
