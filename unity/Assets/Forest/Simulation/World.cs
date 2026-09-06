using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    /// <summary>Authoritative, render-free aggregate. All time and entropy are explicit and saved.</summary>
    [Serializable]
    public partial class World
    {
        public const double SimulationStepSeconds = 0.05;
        // Requested elapsed time remains TimeSeconds; this cursor persists consumed actor time.
        public double SimulationTimeSeconds; public bool ContinuousClockInitialized;
        [NonSerialized] private Dictionary<Cat, double> actorTime;
        public int FormatVersion = 1; public uint Seed, RandomState; public long EpochUnixMs, NextId = 1; public double TimeSeconds, PendingSeconds;
        public List<Village> Villages = new List<Village>(); public List<Player> Players = new List<Player>();
        public List<Tile> Tiles = new List<Tile>(); public List<TradeOffer> TradeOffers = new List<TradeOffer>(); public List<Reservation> Reservations = new List<Reservation>();
        [NonSerialized] private Dictionary<Int2, Tile> tileIndex;
        public static World Create(int seed, long nowUnixMs = 0)
        {
            var w = new World { Seed = unchecked((uint)seed), RandomState = unchecked((uint)seed), EpochUnixMs = nowUnixMs };
            w.Villages.Add(w.Found("communal", "Grand Commons", "", new Int2(0, 0), true));
            return w;
        }
        public string Id(string prefix) => prefix + "-" + (NextId++).ToString(System.Globalization.CultureInfo.InvariantCulture);
        public uint Random()
        {
            RandomState = unchecked(RandomState * 1664525u + 1013904223u);
            return RandomState;
        }
        public double Roll() => Random() / 4294967296.0;
        public static uint Hash(string value)
        {
            uint h = 2166136261;
            foreach (char c in value)
                h = unchecked((h ^ c) * 16777619);
            return h;
        }
        public Village Village(string id) => Villages.Find(v => v.Id == id);
        public Cat Cat(string id) => Villages.SelectMany(v => v.Cats).FirstOrDefault(c => c.Id == id);
        private bool CanModifyTerrain(string villageId, Int2 p, int width = 1, int depth = 1)
        {
            bool Overlaps(Int2 at, int w, int d) => at.X < (long)p.X + width && (long)at.X + w > p.X && at.Z < (long)p.Z + depth && (long)at.Z + d > p.Z;
            if (width < 1 || depth < 1)
                return false;
            // Ownership is checked every actor quantum. Read small footprints through the
            // existing index instead of scanning the entire generated world for each tile.
            if ((long)width * depth < Tiles.Count)
            {
                if (tileIndex == null || tileIndex.Count != Tiles.Count)
                    tileIndex = Tiles.ToDictionary(t => t.Position);
                for (long x = p.X; x < (long)p.X + width; x++)
                    for (long z = p.Z; z < (long)p.Z + depth; z++)
                        if (x <= int.MaxValue && z <= int.MaxValue && tileIndex.TryGetValue(new Int2((int)x, (int)z), out var tile) && tile.ClaimId != "" && tile.ClaimId != villageId)
                            return false;
            }
            else if (Tiles.Any(t => t.ClaimId != "" && t.ClaimId != villageId && Overlaps(t.Position, 1, 1)))
                return false;
            return !Villages.Where(v => v.Id != villageId).Any(v => v.ClaimedTiles.Any(at => Overlaps(at, 1, 1)) || v.Buildings.Any(b => Overlaps(b.Position, b.Width, b.Depth)) || v.Farms.Any(f => Overlaps(f.Position, f.Width, f.Depth)) || v.Stockpiles.Any(s => !s.Kind.StartsWith("zone_", StringComparison.Ordinal) && Overlaps(s.Position, s.Width, s.Depth)));
        }
        private bool CanFoundAt(string villageId, Int2 center, bool communal)
        {
            int radius = communal ? 9 : 6;
            if (!CanModifyTerrain(villageId, new Int2(center.X - radius - 1, center.Z - radius - 1), radius * 2 + 3, radius * 2 + 3) || !CanModifyTerrain(villageId, new Int2(center.X + 2, center.Z + radius + 2)))
                return false;
            return new[] { new Int2(-radius - 2, 4), new Int2(-radius - 2, -4), new Int2(radius + 2, 3), new Int2(radius + 2, -4), new Int2(-radius - 4, -4), new Int2(radius + 4, -4), new Int2(radius + 4, 4), new Int2(-radius - 6, -4) }.All(offset => Enumerable.Range(radius + 1, Math.Abs(offset.X) - radius).All(x => CanModifyTerrain(villageId, new Int2(center.X + Math.Sign(offset.X) * x, center.Z + offset.Z))));
        }
        public Tile GetTile(Int2 p) => Tiles.Find(t => t.Position.Equals(p));
        public Tile TileAt(Int2 p)
        {
            if (tileIndex == null || tileIndex.Count != Tiles.Count)
                tileIndex = Tiles.ToDictionary(t => t.Position);
            if (tileIndex.TryGetValue(p, out var old))
                return old;
            uint h = Hash(Seed + ":" + (p.X / 5) + ":" + (p.Z / 5));
            var t = new Tile { Position = p };
            int biome = (int)(h % 26);
            string[] biomes = { "forest", "meadow", "rainforest", "birch_forest", "taiga", "snow_forest", "flower_forest", "mushroom_forest", "desert", "badlands", "beach", "marsh", "swamp", "lake", "ocean", "river", "hills", "mountains", "snow_mountains", "grassland", "savanna", "jungle", "tundra", "highland", "cave", "woodland" };
            t.Biome = biomes[biome];
            t.Water = biome >= 13 && biome <= 15;
            t.Mountain = biome == 18 || biome == 24;
            uint spot = Hash(Seed + ":" + p.X + ":" + p.Z);
            if (t.Water)
            {
                t.Resource = "fish";
                t.Amount = t.FishCapacity = 24;
            }
            else if (!t.Mountain && spot % 4 == 0)
            {
                t.Resource = biome == 17 ? "gem" : biome == 11 || biome == 12 || biome == 9 ? "clay" : biome == 8 || biome == 10 ? "sand" : biome == 16 ? "ore" : spot % 7 == 0 ? "stone" : "logs";
                t.Amount = 20 + spot % 40;
            }
            else if (!t.Mountain && spot % 11 == 0)
            {
                t.Resource = "food";
                t.Amount = 20;
            }
            foreach (var v in Villages)
                if (Math.Abs(p.X - v.Center.X) <= v.Radius && Math.Abs(p.Z - v.Center.Z) <= v.Radius)
                {
                    t.Biome = "meadow";
                    t.Water = t.Mountain = false;
                    t.Resource = "";
                    t.Amount = 0;
                    t.ClaimId = v.Id;
                }
            Tiles.Add(t);
            tileIndex[p] = t;
            return t;
        }
        private Village Found(string id, string name, string owner, Int2 center, bool communal)
        {
            if (!CanFoundAt(id, center, communal))
                throw new InvalidOperationException("Founding site overlaps foreign territory");
            var v = new Village { Id = id, Name = name, OwnerId = owner, Communal = communal, Center = center, Radius = communal ? 9 : 6, LayoutVersion = 1, FoundedAt = TimeSeconds, NextElection = TimeSeconds + 86400, LastMigration = TimeSeconds };
            int radius = v.Radius;
            for (int z = -radius - 2; z <= radius + 2; z++)
                for (int x = -radius - 2; x <= radius + 2; x++)
                {
                    var p = new Int2(center.X + x, center.Z + z);
                    var t = TileAt(p);
                    v.Known.Add(p);
                    if (Math.Abs(x) <= radius && Math.Abs(z) <= radius)
                    {
                        v.ClaimedTiles.Add(p);
                        t.Water = t.Mountain = false;
                        t.Resource = "";
                        t.Amount = 0;
                        t.Biome = "meadow";
                        t.ClaimId = id;
                        t.Wall = (Math.Abs(x) == radius || Math.Abs(z) == radius) && !LayoutGate(v, p, radius);
                        t.Road = (x == 0 || z == 0) && Math.Max(Math.Abs(x), Math.Abs(z)) >= 2 || Math.Max(Math.Abs(x), Math.Abs(z)) == 2;
                    }
                }
            // Every founding has an exterior footpath from its gate to its finite starter deposits.
            // Clearing one unclaimed ring prevents a seed from marooning the only gate in water or mountains.
            for (int z = -radius - 1; z <= radius + 1; z++)
                for (int x = -radius - 1; x <= radius + 1; x++)
                {
                    if (Math.Max(Math.Abs(x), Math.Abs(z)) != radius + 1)
                        continue;
                    var t = TileAt(new Int2(center.X + x, center.Z + z));
                    t.Water = t.Mountain = t.Wall = false;
                    t.Resource = "";
                    t.Amount = t.FishCapacity = 0;
                    t.Deposits.Clear();
                    t.Biome = "meadow";
                    t.Dirt = true;
                    t.Road = x == 0 || z == 0;
                }
            var water = TileAt(new Int2(center.X + 2, center.Z + radius + 2));
            water.Water = true;
            water.Mountain = false;
            water.Resource = "fish";
            water.Amount = water.FishCapacity = 24;
            var deposits = new[] { ("logs", -radius - 2, 4), ("stone", -radius - 2, -4), ("food", radius + 2, 3), ("fibre", radius + 2, -4), ("ore", -radius - 4, -4), ("clay", radius + 4, -4), ("sand", radius + 4, 4), ("gem", -radius - 6, -4) };
            foreach (var data in deposits)
            {
                for (int x = radius + 2; x <= Math.Abs(data.Item2); x++)
                {
                    var approach = TileAt(new Int2(center.X + Math.Sign(data.Item2) * x, center.Z + data.Item3));
                    approach.Water = approach.Mountain = approach.Wall = false;
                    approach.Resource = "";
                    approach.Amount = approach.FishCapacity = 0;
                    approach.Deposits.Clear();
                    approach.Biome = "meadow";
                    approach.Dirt = true;
                }
            }
            foreach (var data in deposits)
            {
                var t = TileAt(new Int2(center.X + data.Item2, center.Z + data.Item3));
                t.Water = t.Mountain = false;
                t.Resource = data.Item1;
                t.Amount = 200;
            }
            v.Buildings.Add(new Building { Id = Id("shrine"), Kind = "shrine", Position = new Int2(center.X - 1, center.Z - 1), Width = 3, Depth = 3, Completed = true });
            int scale = communal ? 2 : 1;
            var blueprint = communal ? new[] { ("woodworking", -8, -8), ("den", -4, -8), ("wood_cutter", 3, -8), ("den", 7, -8), ("research_hut", -8, -4), ("stone_prep", 3, -4), ("barracks", 7, -4), ("woodworking", -8, 3), ("den", -4, 3), ("wood_cutter", 3, 3), ("den", 7, 3), ("food_storage", -8, 7), ("den", -4, 7), ("stone_prep", 3, 7), ("den", 7, 7) } : new[] { ("woodworking", -5, -5), ("den", -2, -5), ("wood_cutter", 2, -5), ("den", -5, 2), ("den", -2, 3), ("stone_prep", 3, 2) };
            foreach (var entry in blueprint)
                v.Buildings.Add(new Building { Id = Id(entry.Item1), Kind = entry.Item1, Position = new Int2(center.X + entry.Item2, center.Z + entry.Item3), Completed = true, ConstructionConsumed = true });
            var pile = new Stockpile { Id = Id("store"), Position = new Int2(center.X + (communal ? -4 : 3), center.Z + (communal ? -4 : -2)), Capacity = 2000 * scale };
            foreach (var s in new[] { new Stack("food", 50), new Stack("water", 100), new Stack("herbs", 16), new Stack("materials", 60), new Stack("planks", 10), new Stack("blocks", 10) })
                pile.Goods.Add(new Stack(s.Resource, s.Amount * scale));
            v.Stockpiles.Add(pile);
            foreach (var building in v.Buildings.Where(b => b.Kind != "den" && b.Kind != "shrine"))
                Commission(v, building);
            foreach (var building in v.Buildings.Where(b => b.Kind != "shrine"))
                ConnectFoundingBuilding(v, building);
            var arrivals = ConnectedRoads(v).OrderBy(p => Int2.Distance(p, center)).ThenBy(p => p.Z).ThenBy(p => p.X).ToArray();
            string[] names = { "Juniper", "Moss", "Bramble", "Clover", "Pip", "Hazel", "Fern", "Cinder", "Acorn", "Willow", "Mallow", "Birch", "Sorrel", "Poppy", "Sage", "Thistle", "Rowan", "Nettle", "Pebble", "Ash", "Maple", "Honey", "Fennel", "Basil", "Cedar", "Brook", "Aspen", "Yarrow", "Wren", "Cricket", "Dandelion", "Finch", "Oak", "Saffron", "Snowdrop", "Tansy" };
            for (int i = 0; i < 15 * scale; i++)
            {
                var c = NewCat(v, names[(i + (int)(Seed % 6)) % names.Length], arrivals[i % arrivals.Length]);
                c.BedId = v.Buildings.Where(b => b.Kind == "den").ElementAt(i / 5).Id;
                var labor = Catalog.Labors[(int)(Hash(c.Id) % Catalog.Labors.Length)];
                c.Preferences.Add(labor);
                Add(c.Skills, labor, 1 + Hash(c.Id + "skill") % 5);
                v.Cats.Add(c);
            }
            v.LeaderId = v.Cats[0].Id;
            return v;
        }
        private static bool RoadSurface(Tile tile) => tile != null && (tile.Road || tile.Overlay == "road_built");
        private static IEnumerable<Int2> EntranceCandidates(Building building)
        {
            for (int x = 0; x < building.Width; x++)
            {
                yield return new Int2(building.Position.X + x, building.Position.Z - 1);
                yield return new Int2(building.Position.X + x, building.Position.Z + building.Depth);
            }
            for (int z = 0; z < building.Depth; z++)
            {
                yield return new Int2(building.Position.X - 1, building.Position.Z + z);
                yield return new Int2(building.Position.X + building.Width, building.Position.Z + z);
            }
        }
        private bool RoadSpace(Village v, Int2 p) => !v.Buildings.Any(b => Contains(b.Position, b.Width, b.Depth, p)) && !v.Stockpiles.Any(s => s.Kind != "spill" && !s.Kind.StartsWith("zone_", StringComparison.Ordinal) && Contains(s.Position, s.Width, s.Depth, p)) && !v.Farms.Any(f => Contains(f.Position, f.Width, f.Depth, p));
        private HashSet<Int2> ConnectedRoads(Village v, HashSet<Int2> excluded = null, int expansionRadius = 0)
        {
            var seen = new HashSet<Int2>();
            var pending = new Queue<Int2>();
            var shrine = v.Buildings.FirstOrDefault(b => b.Kind == "shrine" && b.Completed);
            if (shrine == null)
                return seen;
            for (int x = 0; x < shrine.Width; x++)
                for (int z = 0; z < shrine.Depth; z++)
                    pending.Enqueue(new Int2(shrine.Position.X + x, shrine.Position.Z + z));
            while (pending.Count > 0)
            {
                var at = pending.Dequeue();
                foreach (var p in Neighbors(at))
                    if (!seen.Contains(p) && (excluded == null || !excluded.Contains(p)) && RoadSurface(GetTile(p)) && RoadSpace(v, p) && Walkable(v, p) && (!ReferenceEquals(Village(v.Id), v) || Crossable(at, p, expansionRadius > 0 ? v : null, expansionRadius)))
                    {
                        seen.Add(p);
                        pending.Enqueue(p);
                    }
            }
            return seen;
        }
        public Int2? BuildingEntrance(Village v, Building building)
        {
            if (building.HasEntrance)
                return building.Entrance;
            var roads = ConnectedRoads(v);
            foreach (var p in EntranceCandidates(building).OrderBy(p => Int2.Distance(p, v.Center)).ThenBy(p => p.Z).ThenBy(p => p.X))
                if (roads.Contains(p))
                    return p;
            return null;
        }
        private bool ShrineRing(Village v, Int2 p) => v.Buildings.Any(b => b.Kind == "shrine" && Contains(new Int2(b.Position.X - 1, b.Position.Z - 1), b.Width + 2, b.Depth + 2, p) && !Contains(b.Position, b.Width, b.Depth, p));
        private bool ConnectedEntrance(Village v, Building building) => !building.HasEntrance || EntranceCandidates(building).Contains(building.Entrance) && ConnectedRoads(v).Contains(building.Entrance);
        private static bool LayoutGate(Village v, Int2 p, int radius)
        {
            int x = p.X - v.Center.X, z = p.Z - v.Center.Z;
            return v.LayoutVersion > 0 ? x == 0 && Math.Abs(z) == radius || z == 0 && Math.Abs(x) == radius : x == 0 && z == radius;
        }
        private void ConnectFoundingBuilding(Village v, Building building)
        {
            var roads = ConnectedRoads(v);
            var seen = new Dictionary<Int2, Int2>();
            var pending = new Queue<Int2>();
            foreach (var p in EntranceCandidates(building).OrderBy(p => Int2.Distance(p, v.Center)).ThenBy(p => p.Z).ThenBy(p => p.X))
                if (RoadSpace(v, p) && Walkable(v, p))
                {
                    seen[p] = p;
                    pending.Enqueue(p);
                }
            while (pending.Count > 0)
            {
                var at = pending.Dequeue();
                if (roads.Contains(at))
                {
                    while (true)
                    {
                        TileAt(at).Road = true;
                        if (seen[at].Equals(at))
                        {
                            building.Entrance = at;
                            building.HasEntrance = true;
                            return;
                        }
                        at = seen[at];
                    }
                }
                foreach (var p in Neighbors(at))
                    if (!seen.ContainsKey(p) && Math.Abs(p.X - v.Center.X) < v.Radius && Math.Abs(p.Z - v.Center.Z) < v.Radius && RoadSpace(v, p) && Walkable(v, p) && (!ReferenceEquals(Village(v.Id), v) || Crossable(at, p)))
                    {
                        seen[p] = at;
                        pending.Enqueue(p);
                    }
            }
            throw new InvalidOperationException("Founding building has no road entrance: " + building.Kind);
        }
        private Cat NewCat(Village v, string name, Int2 p)
        {
            var c = new Cat { Id = Id("cat"), Name = name, VillageId = v.Id, Position = p, X = p.X, Z = p.Z, BirthUnixMilliseconds = EpochUnixMs + (long)((TimeSeconds - 24 * 3600) * 1000) };
            foreach (var stat in new[] { "attack", "defense", "hunting", "medicine", "building", "leadership", "vision", "speed" })
                c.Stats.Add(new Stack(stat, 40 + Random() % 21));
            return c;
        }
        public static double Amount(List<Stack> goods, string resource) => goods.Where(s => s.Resource == resource).Sum(s => s.Amount);
        public static void Add(List<Stack> goods, string resource, double amount)
        {
            if (double.IsNaN(amount) || double.IsInfinity(amount))
                throw new ArgumentException("Finite goods required");
            var s = goods.Find(x => x.Resource == resource);
            if (s == null)
            {
                s = new Stack(resource, 0);
                goods.Add(s);
            }
            s.Amount += amount;
            if (s.Amount < -0.000001)
                throw new InvalidOperationException("Negative " + resource);
            if (Math.Abs(s.Amount) < 0.000001)
                goods.Remove(s);
        }
        public double Total(Village v, string resource) => resource == "blessings" ? v.Blessings : EquipmentKind(resource) != "" ? v.Items.Count(i => i.Kind == EquipmentKind(resource) && v.Stockpiles.Any(s => s.Kind == "storage" && s.Id == i.LocationId)) : v.Stockpiles.Where(s => s.Kind == "storage").Sum(s => Amount(s.Goods, resource));
        public double ReportedTotal(Village v, string resource) => resource == "blessings" ? v.Blessings : v.Stockpiles.Where(s => s.Kind == "storage").Sum(s => Amount(s.Report, resource));
        public double Available(Village v, string resource) => v.Stockpiles.Sum(s => Math.Max(0, Amount(s.Goods, resource) - Reservations.Where(r => r.PileId == s.Id && r.Resource == resource).Sum(r => r.Amount)));
        public bool Reserve(Village v, string ownerId, string resource, double amount)
        {
            if (amount <= 0 || !Finite(amount) || Available(v, resource) + 1e-9 < amount)
                return false;
            foreach (var s in v.Stockpiles.OrderBy(s => s.Id, StringComparer.Ordinal))
            {
                double free = Amount(s.Goods, resource) - Reservations.Where(r => r.PileId == s.Id && r.Resource == resource).Sum(r => r.Amount);
                double take = Math.Min(amount, Math.Max(0, free));
                if (take > 0)
                    Reservations.Add(new Reservation { OwnerId = ownerId, VillageId = v.Id, PileId = s.Id, Resource = resource, Amount = take });
                amount -= take;
                if (amount < 1e-9)
                    return true;
            }
            return false;
        }
        public void Release(string ownerId) => Reservations.RemoveAll(r => r.OwnerId == ownerId);
        public static bool Finite(double value) => !double.IsNaN(value) && !double.IsInfinity(value);
        private bool Spend(Village v, string resource, double amount)
        {
            if (!Finite(amount) || amount < 0 || Available(v, resource) + 1e-9 < amount)
                return false;
            foreach (var s in v.Stockpiles.OrderBy(s => s.Id, StringComparer.Ordinal))
            {
                double free = Math.Max(0, Amount(s.Goods, resource) - Reservations.Where(r => r.PileId == s.Id && r.Resource == resource).Sum(r => r.Amount));
                double take = Math.Min(amount, free);
                if (take > 0)
                    Add(s.Goods, resource, -take);
                amount -= take;
                if (amount < 1e-9)
                    return true;
            }
            return true;
        }
        private Stockpile Storage(Village v, string resource, double amount, Int2 from, string excludedId = null) => v.Stockpiles.Where(s => s.Kind == "storage" && (excludedId == null || s.Id != excludedId) && HasRoom(v, s, resource, amount)).OrderBy(s => Int2.Distance(s.Position, from)).ThenBy(s => s.Id, StringComparer.Ordinal).FirstOrDefault(s => Path(from, s.Position, v) != null);
        public bool HasRoom(Village v, Stockpile s, string resource, double amount)
        {
            if (s.Accepts.Count > 0 && !s.Accepts.Contains(resource))
                return false;
            var limit = s.ResourceLimits.Find(l => l.Resource == resource);
            return limit != null ? Amount(s.Goods, resource) + PileItemCount(v, s, resource) + amount <= limit.Amount + ResourceCapacityBonus(v, resource) : PileLoad(v, s) + amount <= ResourceCapacity(v, s, resource);
        }
        private static int PileItemCount(Village v, Stockpile s, string resource = null)
        {
            bool Matches(Item item) => resource == null || ItemResource(item.Kind) == resource;
            int count = v.Items.Count(i => i.LocationId == s.Id && Matches(i));
            foreach (var job in v.Jobs.Where(j => !j.Completed && j.Phase == "item_fetch" && j.SourceId == s.Id))
                count += v.Items.Count(i => i.LocationId == job.Id && job.ItemIds.Contains(i.Id) && Matches(i));
            return count;
        }
        private static double PileLoad(Village v, Stockpile s) => s.Goods.Sum(g => g.Amount) + PileItemCount(v, s);
        public double Capacity(Village v, Stockpile s)
        {
            var building = v.Buildings.Find(b => b.Id == s.ManagedBy);
            return s.Capacity * Catalog.Effect(v, "storageCapacity", 1) * Catalog.Effect(v, "storagePerLevelMult", 1) * (building == null ? 1 : Catalog.BuildingEffect(v, building.Kind, "capacity", 1));
        }
        public double ResourceCapacity(Village v, Stockpile s, string resource) => Capacity(v, s) + ResourceCapacityBonus(v, resource);
        public double StationCapacity(Village v, string building) => 16 * Catalog.BuildingEffect(v, building, "capacity", 1);
        public bool Walkable(Int2 p)
        {
            var t = TileAt(p);
            return !t.Wall && !t.Mountain && (!t.Water || t.Bridge);
        }
        public List<Int2> Path(Int2 start, Int2 end) => Path(start, end, null);
        public bool Walkable(Village village, Int2 p)
        {
            var t = TileAt(p);
            if (t.Wall || t.Water && !t.Bridge || t.Mountain && !Catalog.Owns(village, "unlock_capability", "mountain_travel"))
                return false;
            return !village.Stockpiles.Any(s => s.Kind == "zone_avoid" && Contains(s.Position, s.Width, s.Depth, p));
        }
        public List<Int2> Path(Int2 start, Int2 end, Village village)
        {
            bool Passable(Int2 p) => village == null ? Walkable(p) : Walkable(village, p);
            if (start.Equals(end))
                return new List<Int2>();
            if (!Passable(end))
                return null;
            var seen = new Dictionary<Int2, Int2> { { start, start } };
            var costs = new Dictionary<Int2, int> { { start, 0 } };
            int sequence = 0;
            var frontier = new SortedSet<(int F, int H, int Order, Int2 Position)>(Comparer<(int F, int H, int Order, Int2 Position)>.Create((a, b) => { int f = a.F.CompareTo(b.F); if (f != 0) return f; int h = a.H.CompareTo(b.H); return h != 0 ? h : a.Order.CompareTo(b.Order); }));
            frontier.Add((Int2.Distance(start, end), Int2.Distance(start, end), sequence++, start));
            var directions = new[] { new Int2(0, 1), new Int2(1, 0), new Int2(0, -1), new Int2(-1, 0) };
            int allowance = Int2.Distance(start, end) + Math.Max(64, (village?.Radius ?? Villages.Select(v => v.Radius).DefaultIfEmpty(6).Max()) * 4 + 16);
            while (frontier.Count > 0 && seen.Count < 20000)
            {
                var current = frontier.Min;
                frontier.Remove(current);
                var at = current.Position;
                foreach (var d in directions)
                {
                    var n = new Int2(at.X + d.X, at.Z + d.Z);
                    int cost = costs[at] + 1;
                    if (costs.TryGetValue(n, out var previous) && previous <= cost || Int2.Distance(start, n) > allowance || !Passable(n) || !Crossable(at, n))
                        continue;
                    seen[n] = at;
                    costs[n] = cost;
                    if (n.Equals(end))
                    {
                        var result = new List<Int2>();
                        while (!n.Equals(start))
                        {
                            result.Add(n);
                            n = seen[n];
                        }
                        result.Reverse();
                        return result;
                    }
                    int h = Int2.Distance(n, end);
                    frontier.Add((cost + h, h, sequence++, n));
                }
            }
            return null;
        }
        private static double CatMovementSpeed(Village village, Tile tile)
        {
            double terrain = tile.Road ? 1.75 : tile.Dirt ? 1.05 : tile.Mountain ? 0.4 : tile.Biome.Contains("forest") ? 0.85 : 1;
            return 1.6 * (village == null ? 1 : Catalog.Effect(village, "movementSpeed", 1) * Catalog.Effect(village, "moveSpeedMult", 1)) * terrain;
        }
        private bool Move(Cat c, Int2 destination, double dt)
        {
            var village = Village(c.VillageId);
            if (c.Position.Equals(destination) && Math.Abs(c.X - destination.X) + Math.Abs(c.Z - destination.Z) < 1e-9)
            {
                c.X = destination.X;
                c.Z = destination.Z;
                c.Path.Clear();
                c.BlockedReason = "";
                return true;
            }
            dt = Math.Min(dt, ActorTime(c));
            if (dt <= 1e-12)
                return false;
            bool changed = c.Path.Count == 0 || !c.Path[c.Path.Count - 1].Equals(destination) || (village == null ? !Walkable(c.Path[0]) : !Walkable(village, c.Path[0])) || !Crossable(c.Position, c.Path[0]);
            if (!changed)
            {
                double x = c.X - c.Position.X, z = c.Z - c.Position.Z;
                int dx = c.Path[0].X - c.Position.X, dz = c.Path[0].Z - c.Position.Z;
                changed = Math.Abs(x * dz - z * dx) > 1e-9 || x * dx + z * dz < -1e-9;
            }
            if (changed && Math.Abs(c.X - c.Position.X) + Math.Abs(c.Z - c.Position.Z) > 1e-9)
            {
                // A changed destination must first retrace the unfinished edge physically.
                double dx = c.Position.X - c.X, dz = c.Position.Z - c.Z, distance = Math.Sqrt(dx * dx + dz * dz);
                var segmentEnd = new Int2(c.Position.X + Math.Sign(c.X - c.Position.X), c.Position.Z + Math.Sign(c.Z - c.Position.Z));
                double speed = CatMovementSpeed(village, TileAt(segmentEnd));
                double used = Math.Min(dt, distance / speed);
                c.X += dx / distance * used * speed;
                c.Z += dz / distance * used * speed;
                UseActorTime(c, used);
                dt -= used;
                if (used * speed + 1e-9 < distance)
                    return false;
                c.X = c.Position.X;
                c.Z = c.Position.Z;
                c.Path.Clear();
                if (c.Position.Equals(destination))
                    return true;
            }
            if (changed)
            {
                c.Path = Path(c.Position, destination, village) ?? new List<Int2>();
                if (c.Path.Count == 0)
                {
                    c.BlockedReason = "blocked_route";
                    return false;
                }
            }
            double remainingSeconds = dt;
            while (remainingSeconds > 0 && c.Path.Count > 0)
            {
                var next = c.Path[0];
                if ((village == null ? !Walkable(next) : !Walkable(village, next)) || !Crossable(c.Position, next))
                {
                    c.Path.Clear();
                    c.BlockedReason = "blocked_route";
                    return false;
                }
                double dx = next.X - c.X, dz = next.Z - c.Z, dist = Math.Sqrt(dx * dx + dz * dz);
                var tile = TileAt(next);
                double speed = CatMovementSpeed(village, tile);
                double step = Math.Min(dist, remainingSeconds * speed);
                if (dist > 1e-9)
                {
                    c.X += dx / dist * step;
                    c.Z += dz / dist * step;
                }
                remainingSeconds -= step / speed;
                UseActorTime(c, step / speed);
                if (step + 1e-9 >= dist)
                {
                    c.Position = next;
                    c.Path.RemoveAt(0);
                    if (!tile.Water && !tile.Mountain && !tile.Road)
                    {
                        tile.PathWear += 1;
                        if (tile.PathWear >= 50)
                            tile.Dirt = true;
                    }
                }
            }
            c.BlockedReason = "";
            return c.Position.Equals(destination);
        }
        public bool Crossable(Int2 a, Int2 b) => Crossable(a, b, null, 0);
        private bool Crossable(Int2 a, Int2 b, Village expanding, int expansionRadius)
        {
            foreach (var v in Villages)
            {
                if (ReferenceEquals(v, expanding) ? ExpansionFarmBoundary(v, expansionRadius, a, b) : v.BoundaryEdges.Any(edge => edge.From.Equals(a) && edge.To.Equals(b) || edge.From.Equals(b) && edge.To.Equals(a)))
                    return false;
                foreach (var building in v.Buildings)
                {
                    if (!building.HasEntrance)
                        continue;
                    bool fromInside = Contains(building.Position, building.Width, building.Depth, a), toInside = Contains(building.Position, building.Width, building.Depth, b);
                    if (fromInside != toInside && !(fromInside ? b : a).Equals(building.Entrance))
                        return false;
                }
            }
            return true;
        }
        private void Note(Village v, string kind, string text, string id = "")
        {
            v.Events.Add(new Event { Time = TimeSeconds, Kind = kind, Text = text, EntityId = id });
            if (v.Events.Count > 100)
                v.Events.RemoveAt(0);
        }
        public void Step(double seconds)
        {
            if (!Finite(seconds) || seconds < 0 || seconds > 86400 * 366)
                throw new ArgumentOutOfRangeException(nameof(seconds));
            if (seconds == 0)
                return;
            if (!ContinuousClockInitialized)
            {
                // Missing fields identify an old save. Never replay its already elapsed time.
                SimulationTimeSeconds = TimeSeconds;
                ContinuousClockInitialized = true;
            }
            double end = Math.Round(TimeSeconds + seconds, 9);
            double next = Math.Round((Math.Floor(SimulationTimeSeconds / SimulationStepSeconds + 1e-8) + 1) * SimulationStepSeconds, 9);
            if (actorTime == null)
                actorTime = new Dictionary<Cat, double>();
            while (next <= end + 1e-9)
            {
                double delta = next - SimulationTimeSeconds;
                TimeSeconds = SimulationTimeSeconds = next;
                bool planning = Math.Abs(next - Math.Round(next)) < 1e-8;
                actorTime.Clear();
                foreach (var v in Villages)
                    foreach (var c in v.Cats)
                        if (c.Alive)
                            actorTime[c] = delta;
                TickControls(delta);
                if (planning)
                    Ecology();
                foreach (var v in Villages.ToArray())
                    TickVillage(v, delta, planning);
                TickTrades(delta, planning);
                next = Math.Round(next + SimulationStepSeconds, 9);
            }
            TimeSeconds = end;
            PendingSeconds = TimeSeconds - Math.Floor(TimeSeconds);
        }
        private double ActorTime(Cat c) => actorTime != null && actorTime.TryGetValue(c, out var remaining) ? remaining : 0;
        private void UseActorTime(Cat c, double seconds)
        {
            if (actorTime != null)
                actorTime[c] = Math.Max(0, ActorTime(c) - seconds);
        }
        private double SpendActorTime(Cat c)
        {
            double remaining = ActorTime(c);
            UseActorTime(c, remaining);
            return remaining;
        }
        public bool CanControl(PlayerContext context, Village village) => context != null && !string.IsNullOrEmpty(context.PlayerId) && village != null && (village.Communal || village.OwnerId == context.PlayerId);
        public List<string> Validate()
        {
            var errors = new List<string>();
            if (!Finite(TimeSeconds) || TimeSeconds < 0)
                errors.Add("Invalid clock");
            if (ContinuousClockInitialized && (!Finite(SimulationTimeSeconds) || SimulationTimeSeconds < 0 || SimulationTimeSeconds > TimeSeconds + 1e-9 || TimeSeconds - SimulationTimeSeconds >= SimulationStepSeconds + 1e-8))
                errors.Add("Invalid simulation cursor");
            foreach (var trade in TradeOffers)
                if (trade.HasContinuousPosition && !ValidContinuousPosition(trade.Position, trade.X, trade.Z))
                    errors.Add("Invalid caravan position " + trade.Id);
            var ids = Villages.SelectMany(v => v.Cats.Select(c => c.Id).Concat(v.Buildings.Select(b => b.Id)).Concat(v.Stockpiles.Select(s => s.Id)).Concat(v.Items.Select(i => i.Id)).Concat(v.Jobs.Select(j => j.Id))).ToList();
            if (ids.Distinct().Count() != ids.Count)
                errors.Add("Duplicate entity identity");
            foreach (var v in Villages)
            {
                foreach (var vehicle in v.Vehicles)
                    if (vehicle.HasContinuousPosition && !ValidContinuousPosition(vehicle.Position, vehicle.X, vehicle.Z))
                        errors.Add("Invalid vehicle position " + vehicle.Id);
                foreach (var raid in v.Raids)
                    if (raid.HasContinuousPosition && !ValidContinuousPosition(raid.Position, raid.X, raid.Z))
                        errors.Add("Invalid raid position " + raid.Id);
                if (v.Trader.HasContinuousPosition && !ValidContinuousPosition(v.Trader.Position, v.Trader.X, v.Trader.Z))
                    errors.Add("Invalid merchant position " + v.Trader.Id);
                foreach (var pile in v.Stockpiles)
                {
                    if (pile.Goods.Any(s => s.Amount < 0 || !Finite(s.Amount)))
                        errors.Add("Invalid stock " + pile.Id);
                    foreach (var s in pile.Goods)
                        if (Reservations.Where(r => r.PileId == pile.Id && r.Resource == s.Resource).Sum(r => r.Amount) > s.Amount + 1e-6)
                            errors.Add("Overclaimed " + pile.Id);
                }
                foreach (var c in v.Cats)
                    if (c.Cargo.Any(s => s.Amount < 0 || !Finite(s.Amount)))
                        errors.Add("Invalid cargo " + c.Id);
                foreach (var bed in v.Buildings.Where(b => b.Kind == "den" && b.Completed))
                    if (v.Cats.Count(c => c.Alive && c.BedId == bed.Id) > DenCapacity(v))
                        errors.Add("Overbooked bed " + bed.Id);
                foreach (var c in v.Cats.Where(c => c.Alive))
                    if (v.Jobs.Count(j => !j.Completed && j.CatId == c.Id) > 1)
                        errors.Add("Double work owner " + c.Id);
            }
            return errors;
        }
        private static bool ValidContinuousPosition(Int2 anchor, double x, double z) => Finite(x) && Finite(z) && Math.Abs(x - anchor.X) + Math.Abs(z - anchor.Z) <= 1 + 1e-8 && (Math.Abs(x - anchor.X) < 1e-8 || Math.Abs(z - anchor.Z) < 1e-8);
    }
}
