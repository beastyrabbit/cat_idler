using IdleCatForest.Simulation;
using IdleCatForest.Authority;
using Newtonsoft.Json.Linq;
using System.Globalization;
using Stack = IdleCatForest.Simulation.Stack;

namespace IdleCatForest.SaveImport;

public static partial class LegacyImport
{
    // Retired building Stores studies cost family.costBase + 4 * stage index 1.
    // Source: research_catalog_tracks.json and expand_building_family in research_catalog.rs.
    private static readonly Dictionary<string, double> RetiredResearch = new()
    {{"den_stores",14},{"beds_stores",15},{"herb_garden_stores",16},{"nursery_stores",19},{"elder_corner_stores",19},{"walls_stores",22},{"mouse_farm_stores",17},{"shrine_stores",21},{"field_stores",17},{"research_hut_stores",12},{"school_stores",18},{"barracks_stores",20},{"accounting_tent_stores",24}};
    public static World Convert(string normalizedJson)
    {
        var document = WireJson.Decode<JObject>(normalizedJson);
        if (S(document, "Format") != "idle-cat-forest-normalized-sqlite" || N(document, "Version") != 1) throw new InvalidDataException("Expected normalized SQLite version 1. Run the read-only compatibility normalizer first.");
        var tables = document["Tables"] as JObject ?? throw new InvalidDataException("Normalized tables are absent.");
        var required = new[] { "world", "colonies", "cats", "jobs", "buildings", "world_tiles", "shared_world_tiles", "events", "player_names", "zones", "elections", "votes", "raiders" };
        if (required.Any(name => tables[name] is not JArray) || tables.Properties().Any(p => !required.Contains(p.Name))) throw new InvalidDataException("Unsupported normalized table inventory.");
        var worldRows = Rows(tables, "world"); if (worldRows.Count != 1) throw new InvalidDataException("Expected one shared world row.");
        var colonies = Rows(tables, "colonies"); if (colonies.Count == 0) throw new InvalidDataException("No colonies in normalized world.");
        long epoch = colonies.Select(c => (long)N(c, "createdAt")).Min(), last = colonies.Select(c => (long)N(c, "lastTick")).Max();
        var world = new World { Seed = unchecked((uint)N(worldRows[0], "worldSeed")), RandomState = unchecked((uint)N(worldRows[0], "worldSeed")), EpochUnixMs = epoch, TimeSeconds = Math.Max(0, (last - epoch) / 1000d), NextId = 1 };
        var context = new Context(world, tables, document["Derived"] as JObject);
        foreach (var row in colonies) context.Colony(row);
        context.Tiles();
        foreach (var village in world.Villages)
        {
            var row = colonies.Single(c => S(c, "id") == village.Id);
            context.Buildings(village); context.Cats(village); context.Piles(village, row); context.Items(village, J(row, "items"), village.Items);
            context.Jobs(village); context.Stations(village); context.Cargo(village); context.Farms(village, row); context.Migration(village, row);
            context.Governance(village); context.Trader(village, row); context.Trades(village, row); context.Transport(village, row); context.Accounting(village, row);
            if (village.OwnerId != "") world.Players.Add(new Player { Id = village.OwnerId, SelectedVillageId = village.Id, PersonalVillageId = village.Id });
            // Old housing was derived rather than persisted. Stable assignment reserves existing beds, never creates capacity.
            var dens = village.Buildings.Where(b => b.Completed && b.Kind == "den").ToArray(); int bed = 0;
            foreach (var cat in village.Cats.Where(c => c.Alive && c.Migration == "resident").OrderBy(c => c.BirthUnixMilliseconds).ThenBy(c => c.Id, StringComparer.Ordinal))
            { if (bed < dens.Length * 5) cat.BedId = dens[bed++ / 5].Id; }
        }
        context.References();
        AuthorityRuntime.ValidateWorld(world); return world;
    }
    private static string S(JToken row, string key, string fallback = "") => row?[key]?.Type == JTokenType.Null ? fallback : row?[key]?.Value<string>() ?? fallback;
    private static double N(JToken row, string key, double fallback = 0) => row?[key] == null || row[key].Type == JTokenType.Null ? fallback : row[key].Value<double>();
    private static bool B(JToken row, string key) => row?[key] != null && row[key].Type != JTokenType.Null && (row[key].Type == JTokenType.Boolean ? row[key].Value<bool>() : row[key].Value<long>() != 0);
    private static JToken J(JToken row, string key)
    {
        var token = row?[key]; if (token == null || token.Type == JTokenType.Null) return null;
        return token.Type == JTokenType.String ? WireJson.Decode<JToken>(token.Value<string>()) : token;
    }
    private static List<JToken> Entries(JToken token) => token is JObject obj ? obj.Properties().Select(p => p.Value).ToList() : token is JArray array ? array.ToList() : new List<JToken>();
    private static List<JToken> Rows(JObject tables, string table) => Entries(tables[table]);
    private static List<string> Strings(JToken token) => token is JArray array ? array.Select(t => t.Type == JTokenType.Null ? null : t.Value<string>()).ToList() : new List<string>();
    private static Int2 Point(JToken token)
    {
        if (token is JArray a && a.Count >= 2) return new Int2((int)a[0], (int)a[1]);
        return new Int2((int)N(token, "x"), (int)N(token, "y", N(token, "z")));
    }
    private static List<Int2> Points(JToken token) => Entries(token).Select(Point).ToList();
    private static string Q(string colony, string id) => string.IsNullOrEmpty(id) ? "" : id.Contains('\u001f') || id.StartsWith("world-item:", StringComparison.Ordinal) ? id : colony + "\u001f" + id;
    private static string Raw(string colony, string id) => id.StartsWith(colony + "\u001f", StringComparison.Ordinal) ? id.Substring(colony.Length + 1) : id;
    private static List<Stack> Goods(JToken token)
    {
        if (token is not JObject obj) return new List<Stack>();
        var goods = new List<Stack>(); foreach (var property in obj.Properties())
        {
            if (property.Value.Type == JTokenType.Null) continue;
            var amount = property.Value.Value<double>(); if (!World.Finite(amount) || amount < 0) throw new InvalidDataException("Legacy inventory contains an invalid quantity.");
            if (amount > 0) goods.Add(new Stack(property.Name, amount));
        }
        return goods;
    }
    private sealed partial class Context(World world, JObject tables, JObject derived)
    {
        private readonly World world = world;
        private readonly JObject tables = tables;
        private readonly JObject derived = derived;
        private readonly Dictionary<string, JToken> buildingRows = new(), catRows = new();
        private double Time(JToken row, string field, double fallback = -1) => row?[field] == null || row[field].Type == JTokenType.Null ? fallback : (N(row, field) - world.EpochUnixMs) / 1000d;
        private IEnumerable<JToken> Of(string table, Village village) => Rows(tables, table).Where(row => S(row, "colonyId") == village.Id);
        public void Colony(JToken row)
        {
            var id = S(row, "id"); if (id.Length == 0) throw new InvalidDataException("Colony identity is missing.");
            var v = new Village { Id = id, Name = S(row, "name"), OwnerId = S(row, "ownerPlayerId"), Communal = B(row, "isGlobal"), Center = new Int2((int)N(row, "anchorX", 6), (int)N(row, "anchorY", 6)), LeaderId = Q(id, S(row, "leaderId")), FoundedAt = Time(row, "runStartedAt", Time(row, "createdAt", 0)), Run = (int)N(row, "runNumber", 1), Coins = N(row, "coin"), LastLeaderResearch = Time(row, "lastLoremasterUnlockAt", -86400), AutomationTier = N(row, "automationTier"), GlobalUpgradePoints = N(row, "globalUpgradePoints"), ThreatPressure = N(row, "threatPressure"), LastRaidAt = Time(row, "lastRaidAt"), ActiveRaidId = Q(id, S(row, "activeRaidId")), RaidClicks = N(row, "raidClicks"), LastTitheAt = Time(row, "lastTitheAt"), LastOfferingAt = Time(row, "lastOfferingAt"), LastPlayerActivityAt = Time(row, "lastPlayerActivityAt"), CriticalSince = Time(row, "criticalSince"), RitualRequestedAt = Time(row, "ritualRequestedAt"), Status = S(row, "status", "starting"), MigrationDepartures = (long)N(row, "migrationDepartures"), TraderVisitCount = (long)N(row, "traderVisitCount") };
            v.Blessings = N(row, "globalUpgradePoints"); v.GrandfatheredRecipes = N(row, "recipeEntitlementRulesVersion") == 0; v.ClaimedTiles = Points(J(row, "claimedTiles")); v.AgriculturalTiles = Points(J(row, "agriculturalTiles")); v.Known = Points(J(row, "revealedTiles"));
            v.Radius = v.ClaimedTiles.Count == 0 ? 6 : Math.Max(1, v.ClaimedTiles.Max(p => Math.Max(Math.Abs(p.X - v.Center.X), Math.Abs(p.Z - v.Center.Z))));
            v.Contacts = Strings(J(row, "knownVillageIds")); v.UpgradeLevels = Goods(J(row, "upgradeLevels"));
            var tree = J(row, "upgradeTree"); v.Research = Strings(tree?["ownedNodeIds"] ?? tree?["owned_node_ids"]); v.ResearchPoints = N(tree, "researchPoints", N(tree, "research_points"));
            foreach (var retired in v.Research.Where(id => Catalog.Study(id) == null).ToArray())
            { if (!RetiredResearch.TryGetValue(retired, out double refund)) throw new NotSupportedException("Unknown owned legacy research node."); v.Research.Remove(retired); v.ResearchPoints += refund; v.Events.Add(new Event { Time = world.TimeSeconds, Kind = "migration", Text = "Retired research " + retired + " refunded " + refund.ToString(CultureInfo.InvariantCulture) + " research points." }); }
            var officers = J(row, "officers") as JObject; if (officers != null) foreach (var officer in officers.Properties()) v.Officers.Add(new Officer { Role = officer.Name, CatId = Q(id, officer.Value.Value<string>()) });
            world.Villages.Add(v);
        }
        public void Tiles()
        {
            foreach (var row in Rows(tables, "shared_world_tiles"))
            {
                var resources = Goods(J(row, "resources")); var maximum = Goods(J(row, "maxResources")); foreach (var s in resources.Concat(maximum)) { if (s.Resource == "wood") s.Resource = "logs"; }
                var type = S(row, "type"); var tile = new Tile { Position = new Int2((int)N(row, "x"), (int)N(row, "y")), Biome = type, Water = type == "water" || type == "ocean" || type == "river" || type == "lake", Mountain = type == "mountain" || type == "mountains", Deposits = resources, MaximumDeposits = maximum, Danger = N(row, "dangerLevel"), PathWear = N(row, "pathWear"), LastDepletedAt = Time(row, "lastDepleted"), Overlay = S(row, "overlayFeature") };
                var primary = resources.FirstOrDefault(); if (primary != null) { tile.Resource = primary.Resource; tile.Amount = primary.Amount; }
                tile.Road = tile.Overlay == "road"; tile.Dirt = tile.PathWear > 0 && !tile.Road; tile.Bridge = tile.Overlay.Contains("bridge", StringComparison.Ordinal); world.Tiles.Add(tile);
            }
            foreach (var village in world.Villages)
            {
                foreach (var p in village.ClaimedTiles) world.TileAt(p).ClaimId = village.Id;
                if (village.ClaimedTiles.Count > 0 && derived?[village.Id]?["BoundaryEdges"] is not JArray) throw new InvalidDataException("Claimed villages require exact boundary geometry from the current normalizer.");
                foreach (var edge in Entries(derived?[village.Id]?["BoundaryEdges"])) village.BoundaryEdges.Add(new BoundaryEdge { From = Point(edge["From"]), To = Point(edge["To"]) });
            }
            foreach (var habitat in Entries(J(Rows(tables, "world")[0], "sharedFishHabitats")))
            { if (habitat is not JArray a || a.Count != 3) throw new InvalidDataException("Invalid shared Fish ledger."); var tile = world.TileAt(new Int2((int)a[0], (int)a[1])); tile.FishCapacity = N(a[2], "capacity"); tile.FishReplenishedAt = Time(a[2], "lastReplenishedAtMs"); World.Add(tile.Deposits, "fish", N(a[2], "stock") - World.Amount(tile.Deposits, "fish")); }
        }
        public void Buildings(Village v)
        {
            foreach (var row in Of("buildings", v))
            {
                var kind = S(row, "type"); if (!Catalog.Buildings.Contains(kind)) throw new NotSupportedException("Unsupported legacy building type.");
                var b = new Building { Id = Q(v.Id, S(row, "id")), Kind = kind, Position = Point(J(row, "position")), Completed = B(row, "isComplete"), Level = (int)N(row, "level", 1), WorkerId = Q(v.Id, S(row, "assignedCatId")), Paused = B(row, "productionPaused"), AutomatedBy = J(row, "automatedOfficerRole")?.Value<string>() ?? "", Progress = N(row, "constructionProgress"), RequiredWork = 100 };
                b.Width = new[] { "shrine", "workshop", "smithy", "food_storage", "wood_cutter", "stone_prep", "woodworking", "clothier", "tannery", "smelter", "mill", "sawmill" }.Contains(kind) ? 3 : kind == "water_bowl" || kind == "walls" ? 1 : 2; b.Depth = b.Width == 1 ? 1 : 3;
                b.Queue = Queue(J(row, "productionQueue")); v.Buildings.Add(b); buildingRows[b.Id] = row;
                b.Slots.Add(new WorkSlot { CatId = b.WorkerId, Queue = b.Queue.Select(q => new QueueEntry { RecipeId = q.RecipeId, Repeat = q.Repeat }).ToList(), Paused = b.Paused, Progress = N(row, "productionProgress"), AutomatedBy = b.AutomatedBy });
                foreach (var slot in Entries(J(row, "additionalWorkSlots")))
                { var worker = Q(v.Id, S(slot, "assignedCat")); if (worker == "") worker = Q(v.Id, S(slot, "assignedCatId")); b.Slots.Add(new WorkSlot { CatId = worker, Queue = Queue(slot["productionQueue"]), Paused = B(slot, "productionPaused"), Progress = N(slot, "productionProgress"), AutomatedBy = S(slot, "automatedBy") }); b.ExtraWorkerIds.Add(worker); }
                var contract = J(row, "constructionCargo"); if (contract != null)
                {
                    b.ConstructionConsumed = B(contract, "consumed"); foreach (var pair in new[] { ("lumber", "requiredLumber", "deliveredLumber"), ("planks", "requiredPlanks", "deliveredPlanks"), ("blocks", "requiredBlocks", "deliveredBlocks") }) { if (N(contract, pair.Item2) > 0) b.Required.Add(new Stack(pair.Item1, N(contract, pair.Item2))); if (N(contract, pair.Item3) > 0) b.Inputs.Add(new Stack(pair.Item1, N(contract, pair.Item3))); }
                    foreach (var r in Entries(contract["reservations"])) world.Reservations.Add(new Reservation { OwnerId = b.Id, VillageId = v.Id, PileId = Q(v.Id, S(r, "sourceStockpileId")), Resource = S(r, "kind"), Amount = N(r, "amount") });
                }
                else if (!b.Completed) { b.Required.Clear(); b.ConstructionConsumed = true; } // Legacy null contract is a funded scaffold.
            }
        }
        private static List<QueueEntry> Queue(JToken token)
        {
            var result = new List<QueueEntry>(); foreach (var entry in Entries(token))
            {
                var id = entry.Type == JTokenType.String ? entry.Value<string>() : S(entry, "recipeId", S(entry, "recipe_id")); if (Catalog.Recipe(id) == null) throw new NotSupportedException("Unsupported legacy recipe queue ID."); result.Add(new QueueEntry { RecipeId = id, Repeat = entry.Type == JTokenType.String || B(entry, "repeat") });
            }
            return result;
        }
        public void Cats(Village v)
        {
            foreach (var row in Of("cats", v))
            {
                var position = J(row, "position"); var needs = J(row, "needs"); var c = new Cat { Id = Q(v.Id, S(row, "id")), VillageId = v.Id, Name = S(row, "name"), Position = Point(position), X = N(position, "x"), Z = N(position, "y"), AgeHours = N(row, "ageHours"), Hunger = N(needs, "hunger", 100), Thirst = N(needs, "thirst", 100), Rest = N(needs, "rest", 100), Health = N(needs, "health", 100), Alive = row["deathTime"] == null || row["deathTime"].Type == JTokenType.Null, Boosted = B(row, "boosted"), BirthUnixMilliseconds = (long)N(row, "birthTime"), Stats = Goods(J(row, "stats")), RoleExperience = Goods(J(row, "roleXp")), Specialization = S(row, "specialization"), AppearanceJson = J(row, "spriteParams")?.ToString(Newtonsoft.Json.Formatting.None) ?? "", PregnancyMateId = Q(v.Id, S(row, "pregnancyMateId")), ParentIds = Strings(J(row, "parentIds")).Select(id => id == null ? null : Q(v.Id, id)).ToList(), Preferences = Strings(J(row, "preferredLabors")), Skills = Goods(J(row, "skills")) };
                if (!c.Alive) c.DeathUnixMilliseconds = (long)N(row, "deathTime");
                if (S(position, "map") == "colony") { c.X += v.Center.X; c.Z += v.Center.Z; }
                c.Position = new Int2((int)Math.Round(c.X, MidpointRounding.AwayFromZero), (int)Math.Round(c.Z, MidpointRounding.AwayFromZero));
                if (B(row, "isPregnant")) c.PregnantUntil = world.TimeSeconds + Math.Max(0, N(row, "pregnancyDueAgeHours", c.AgeHours + 18) - c.AgeHours) * 3600;
                var cargo = J(row, "carrying"); if (cargo != null) { c.ImportedCargoSource = S(cargo, "sourceGatherSpot"); if (N(cargo, "amount") > 0) c.Cargo.Add(new Stack(S(cargo, "kind"), N(cargo, "amount"))); }
                var notebook = J(Rows(tables, "colonies").Single(r => S(r, "id") == v.Id), "provisionalTiles"); c.ScoutNotes = Points(notebook?[Raw(v.Id, c.Id)]);
                var role = v.Officers.FirstOrDefault(o => o.CatId == c.Id); if (role != null) c.OfficerRole = role.Role;
                var station = v.Buildings.FirstOrDefault(b => b.Slots.Any(s => s.CatId == c.Id)); if (station != null) c.BuildingId = station.Id;
                v.Cats.Add(c); catRows[c.Id] = row;
            }
        }
        public void Piles(Village v, JToken row)
        {
            foreach (var pile in Entries(J(row, "stockpiles")))
            {
                var rect = pile["rect"]; var id = S(pile, "id"); var p = new Stockpile { Id = Q(v.Id, id), Position = new Int2((int)N(rect, "x1"), (int)N(rect, "y1")), Width = (int)(N(rect, "x2") - N(rect, "x1") + 1), Depth = (int)(N(rect, "y2") - N(rect, "y1") + 1), Accepts = Strings(pile["accepts"]), Goods = Goods(pile["contents"]), Kind = id.StartsWith("station-", StringComparison.Ordinal) || id.StartsWith("construction-", StringComparison.Ordinal) ? "legacy_local" : "storage" };
                p.ResourceLimits = Goods(derived?[v.Id]?["PileResourceLimits"]?[id]); p.Capacity = id == "stockpile-storehouse" ? 360 : p.Width * p.Depth * 40; v.Stockpiles.Add(p);
            }
            if (v.Stockpiles.Count == 0) throw new InvalidDataException("Normalized village has no physical stockpile. Reconcile it through the maintained loader first.");
            foreach (var gather in Entries(J(row, "gatherSpots")))
            { var p = v.Stockpiles.FirstOrDefault(p => p.Id == Q(v.Id, S(gather, "stockpileId"))); if (p != null) { p.Kind = S(gather, "purpose") == "fishing" ? "fishing" : "gather"; p.ExpiresAt = Time(gather, "expiresAtMs"); } }
        }
        public void References()
        {
            var allIds = world.Villages.SelectMany(v => v.Cats.Select(c => c.Id).Concat(v.Buildings.Select(b => b.Id)).Concat(v.Stockpiles.Select(p => p.Id)).Concat(v.Jobs.Select(j => j.Id)).Concat(v.Items.Select(i => i.Id))).ToHashSet();
            // Imported IDs keep their colony-qualified original identity; fresh IDs never reuse their suffixes.
            foreach (var id in allIds) { var suffix = id.Substring(id.LastIndexOf('-') + 1); if (long.TryParse(suffix, NumberStyles.None, CultureInfo.InvariantCulture, out long serial) && serial >= world.NextId) world.NextId = checked(serial + 1); }
            foreach (var v in world.Villages) foreach (var job in v.Jobs.Where(j => !j.Completed && j.CatId != ""))
            { var cat = v.Cats.FirstOrDefault(c => c.Id == job.CatId); if (cat == null) throw new InvalidDataException("Imported active job references a missing cat."); if (cat.JobId != "" && cat.JobId != job.Id) throw new InvalidDataException("Imported cat has competing active work owners."); cat.JobId = job.Id; }
        }
    }
}
