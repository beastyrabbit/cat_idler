using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    /// <summary>Authoritative, render-free aggregate. All time and entropy are explicit and saved.</summary>
    [Serializable]
    public partial class World
    {
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
            var v = new Village { Id = id, Name = name, OwnerId = owner, Communal = communal, Center = center, Radius = communal ? 9 : 6, FoundedAt = TimeSeconds, NextElection = TimeSeconds + 86400, LastMigration = TimeSeconds };
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
                        t.Wall = (Math.Abs(x) == radius || Math.Abs(z) == radius) && !(x == 0 && z == radius);
                        t.Road = x == 0 || z == 0;
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
                }
            var water = TileAt(new Int2(center.X + 2, center.Z + radius + 2));
            water.Water = true;
            water.Mountain = false;
            water.Resource = "fish";
            water.Amount = water.FishCapacity = 24;
            foreach (var data in new[] { ("logs", -radius - 2, 4), ("stone", -radius - 2, -4), ("food", radius + 2, 3), ("fibre", radius + 2, -4), ("ore", -radius - 4, -4), ("clay", radius + 4, -4), ("sand", radius + 4, 4), ("gem", -radius - 6, -4) })
            {
                var t = TileAt(new Int2(center.X + data.Item2, center.Z + data.Item3));
                t.Water = t.Mountain = false;
                t.Resource = data.Item1;
                t.Amount = 200;
            }
            v.Buildings.Add(new Building { Id = Id("shrine"), Kind = "shrine", Position = center, Completed = true });
            int scale = communal ? 2 : 1;
            var blueprint = communal ? new[] { ("woodworking", -8, -8), ("den", -4, -8), ("wood_cutter", 3, -8), ("den", 7, -8), ("research_hut", -8, -4), ("stone_prep", 3, -4), ("barracks", 7, -4), ("woodworking", -8, 3), ("den", -4, 3), ("wood_cutter", 3, 3), ("den", 7, 3), ("food_storage", -8, 7), ("den", -4, 7), ("stone_prep", 3, 7), ("den", 7, 7) } : new[] { ("woodworking", -5, -5), ("den", -2, -5), ("wood_cutter", 2, -5), ("den", -5, 2), ("den", -2, 2), ("stone_prep", 2, 2) };
            foreach (var entry in blueprint)
                v.Buildings.Add(new Building { Id = Id(entry.Item1), Kind = entry.Item1, Position = new Int2(center.X + entry.Item2, center.Z + entry.Item3), Completed = true, ConstructionConsumed = true });
            var pile = new Stockpile { Id = Id("store"), Position = new Int2(center.X + (communal ? -4 : 2), center.Z + (communal ? -4 : -2)), Capacity = 2000 * scale };
            foreach (var s in new[] { new Stack("food", 50), new Stack("water", 100), new Stack("herbs", 16), new Stack("materials", 60), new Stack("planks", 10), new Stack("blocks", 10) })
                pile.Goods.Add(new Stack(s.Resource, s.Amount * scale));
            v.Stockpiles.Add(pile);
            foreach (var building in v.Buildings.Where(b => b.Kind != "den" && b.Kind != "shrine"))
                Commission(v, building);
            string[] names = { "Juniper", "Moss", "Bramble", "Clover", "Pip", "Hazel", "Fern", "Cinder", "Acorn", "Willow", "Mallow", "Birch", "Sorrel", "Poppy", "Sage", "Thistle", "Rowan", "Nettle", "Pebble", "Ash", "Maple", "Honey", "Fennel", "Basil", "Cedar", "Brook", "Aspen", "Yarrow", "Wren", "Cricket", "Dandelion", "Finch", "Oak", "Saffron", "Snowdrop", "Tansy" };
            for (int i = 0; i < 15 * scale; i++)
            {
                var c = NewCat(v, names[(i + (int)(Seed % 6)) % names.Length], new Int2(center.X + (i % 5) - 2, center.Z + 2 + (i / 5) % 3));
                c.BedId = v.Buildings.Where(b => b.Kind == "den").ElementAt(i / 5).Id;
                var labor = Catalog.Labors[(int)(Hash(c.Id) % Catalog.Labors.Length)];
                c.Preferences.Add(labor);
                Add(c.Skills, labor, 1 + Hash(c.Id + "skill") % 5);
                v.Cats.Add(c);
            }
            v.LeaderId = v.Cats[0].Id;
            return v;
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
        private Stockpile Storage(Village v, string resource, double amount, Int2 from) => v.Stockpiles.Where(s => s.Kind == "storage" && HasRoom(v, s, resource, amount)).OrderBy(s => Int2.Distance(s.Position, from)).ThenBy(s => s.Id, StringComparer.Ordinal).FirstOrDefault(s => Path(from, s.Position, v) != null);
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
        private bool Move(Cat c, Int2 destination, double dt)
        {
            var village = Village(c.VillageId);
            if (c.Position.Equals(destination))
            {
                c.X = destination.X;
                c.Z = destination.Z;
                c.Path.Clear();
                c.BlockedReason = "";
                return true;
            }
            if (c.Path.Count == 0 || !c.Path[c.Path.Count - 1].Equals(destination) || (village == null ? !Walkable(c.Path[0]) : !Walkable(village, c.Path[0])))
            {
                c.Path = Path(c.Position, destination, village) ?? new List<Int2>();
                if (c.Path.Count == 0)
                {
                    c.BlockedReason = "blocked_route";
                    return false;
                }
            }
            double travel = dt * 1.6 * (village == null ? 1 : Catalog.Effect(village, "movementSpeed", 1) * Catalog.Effect(village, "moveSpeedMult", 1));
            while (travel > 0 && c.Path.Count > 0)
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
                double speed = tile.Road ? 1.75 : tile.Dirt ? 1.05 : tile.Mountain ? 0.4 : tile.Biome.Contains("forest") ? 0.85 : 1;
                double step = Math.Min(dist, travel * speed);
                if (dist > 1e-9)
                {
                    c.X += dx / dist * step;
                    c.Z += dz / dist * step;
                }
                travel -= step / speed;
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
        public bool Crossable(Int2 a, Int2 b) => !Villages.Any(v => v.BoundaryEdges.Any(edge => edge.From.Equals(a) && edge.To.Equals(b) || edge.From.Equals(b) && edge.To.Equals(a)));
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
            double end = TimeSeconds + seconds;
            while (TimeSeconds + 1e-9 < end)
            {
                double boundary = Math.Floor(TimeSeconds + 1e-9) + 1;
                double next = Math.Min(end, boundary);
                bool controlling = Villages.Any(v => v.Cats.Any(c => c.Alive && c.ControlledBy != ""));
                if (controlling)
                    next = Math.Min(next, TimeSeconds + 0.05);
                double delta = next - TimeSeconds;
                TimeSeconds = Math.Round(next, 9);
                if (controlling)
                    TickControls(delta);
                if (Math.Abs(TimeSeconds - boundary) < 1e-8)
                {
                    TimeSeconds = boundary;
                    Ecology();
                    foreach (var v in Villages.ToArray())
                        TickVillage(v);
                    TickTrades();
                }
            }
            PendingSeconds = TimeSeconds - Math.Floor(TimeSeconds);
        }
        public bool CanControl(PlayerContext context, Village village) => context != null && !string.IsNullOrEmpty(context.PlayerId) && village != null && (village.Communal || village.OwnerId == context.PlayerId);
        public List<string> Validate()
        {
            var errors = new List<string>();
            if (!Finite(TimeSeconds) || TimeSeconds < 0)
                errors.Add("Invalid clock");
            var ids = Villages.SelectMany(v => v.Cats.Select(c => c.Id).Concat(v.Buildings.Select(b => b.Id)).Concat(v.Stockpiles.Select(s => s.Id)).Concat(v.Items.Select(i => i.Id)).Concat(v.Jobs.Select(j => j.Id))).ToList();
            if (ids.Distinct().Count() != ids.Count)
                errors.Add("Duplicate entity identity");
            foreach (var v in Villages)
            {
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
    }
}
