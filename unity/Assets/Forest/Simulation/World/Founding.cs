using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private bool CanModifyTerrain(string villageId, Int2 p, int width = 1, int depth = 1)
        {
            bool Overlaps(Int2 at, int w, int d) => at.Level == p.Level && at.X < (long)p.X + width && (long)at.X + w > p.X && at.Z < (long)p.Z + depth && (long)at.Z + d > p.Z;
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
            if (!CanPrepareFoundingTerrain(villageId, center, radius))
                return false;
            // Version 2 upgrades natural deposits it is allowed to touch instead of carving fixed ones.
            if (GenerationVersion >= 2)
                return CanModifyTerrain(villageId, new Int2(center.X - radius - 1, center.Z - radius - 1), radius * 2 + 3, radius * 2 + 3);
            if (!CanModifyTerrain(villageId, new Int2(center.X - radius - 1, center.Z - radius - 1), radius * 2 + 3, radius * 2 + 3) || !CanModifyTerrain(villageId, new Int2(center.X + 2, center.Z + radius + 2)))
                return false;
            return new[] { new Int2(-radius - 2, 4), new Int2(-radius - 2, -4), new Int2(radius + 2, 3), new Int2(radius + 2, -4), new Int2(-radius - 4, -4), new Int2(radius + 4, -4), new Int2(radius + 4, 4), new Int2(-radius - 6, -4) }.All(offset => Enumerable.Range(radius + 1, Math.Abs(offset.X) - radius).All(x => CanModifyTerrain(villageId, new Int2(center.X + Math.Sign(offset.X) * x, center.Z + offset.Z))));
        }
        private Village Found(string id, string name, string owner, Int2 center, bool communal)
        {
            if (!CanFoundAt(id, center, communal))
                throw new InvalidOperationException("Founding site overlaps foreign territory");
            var v = new Village { Id = id, Name = name, OwnerId = owner, Communal = communal, Center = center, Radius = communal ? 9 : 6, LayoutVersion = 1, FoundedAt = TimeSeconds, NextElection = TimeSeconds + Pace.ElectionIntervalSeconds, LastMigration = TimeSeconds };
            PrepareFoundingTerrain(v);
            if (GenerationVersion >= 2)
                return FoundColony(v);
            int radius = v.Radius;
            for (int z = -radius - 2; z <= radius + 2; z++)
                for (int x = -radius - 2; x <= radius + 2; x++)
                {
                    var p = new Int2(center.X + x, center.Z + z);
                    var t = TileAt(p);
                    v.Known.Add(p);
                    if (Math.Abs(x) <= radius && Math.Abs(z) <= radius)
                    {
                        if (GenerationVersion > 0)
                            ClearSurface(t, 0);
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
                    if (GenerationVersion > 0)
                        ClearSurface(t, 0);
                    t.Water = t.Mountain = t.Wall = false;
                    t.Resource = "";
                    t.Amount = t.FishCapacity = 0;
                    t.Deposits.Clear();
                    t.Biome = "meadow";
                    t.Dirt = true;
                    t.Road = x == 0 || z == 0;
                }
            var water = TileAt(new Int2(center.X + 2, center.Z + radius + 2));
            if (GenerationVersion > 0)
                SetWaterSurface(water, 0, .8);
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
                    if (GenerationVersion > 0)
                        ClearSurface(approach, 0);
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
                if (GenerationVersion > 0)
                    ClearSurface(t, 0);
                t.Water = t.Mountain = false;
                t.Resource = data.Item1;
                t.Amount = 200;
            }
            v.Buildings.Add(new Building { Id = Id("shrine"), Kind = "shrine", Position = new Int2(center.X - 1, center.Z - 1), Width = 3, Depth = 3, Completed = true });
            int scale = communal ? 2 : 1;
            foreach (var entry in FoundingBlueprint(communal))
                v.Buildings.Add(new Building { Id = Id(entry.Kind), Kind = entry.Kind, Position = new Int2(center.X + entry.X, center.Z + entry.Z), Completed = true, ConstructionConsumed = true });
            FoundingStore(v, new Int2(center.X + (communal ? -4 : 3), center.Z + (communal ? -4 : -2)));
            foreach (var building in v.Buildings.Where(b => b.Kind != "den" && b.Kind != "shrine"))
                Commission(v, building);
            foreach (var building in v.Buildings.Where(b => b.Kind != "shrine"))
                ConnectFoundingBuilding(v, building);
            var arrivals = ConnectedRoads(v).OrderBy(p => Int2.Distance(p, center)).ThenBy(p => p.Z).ThenBy(p => p.X).ToArray();
            AddFounders(v, arrivals);
            return v;
        }

        private static readonly (string Kind, int X, int Z)[] CommunalBlueprint = { ("woodworking", -8, -8), ("den", -4, -8), ("wood_cutter", 3, -8), ("den", 7, -8), ("research_hut", -8, -4), ("stone_prep", 3, -4), ("barracks", 7, -4), ("woodworking", -8, 3), ("den", -4, 3), ("wood_cutter", 3, 3), ("den", 7, 3), ("food_storage", -8, 7), ("den", -4, 7), ("stone_prep", 3, 7), ("den", 7, 7) };
        private static readonly (string Kind, int X, int Z)[] PersonalBlueprint = { ("woodworking", -5, -5), ("den", -2, -5), ("wood_cutter", 2, -5), ("den", -5, 2), ("den", -2, 3), ("stone_prep", 3, 2) };
        private static readonly string[] FounderNames = { "Juniper", "Moss", "Bramble", "Clover", "Pip", "Hazel", "Fern", "Cinder", "Acorn", "Willow", "Mallow", "Birch", "Sorrel", "Poppy", "Sage", "Thistle", "Rowan", "Nettle", "Pebble", "Ash", "Maple", "Honey", "Fennel", "Basil", "Cedar", "Brook", "Aspen", "Yarrow", "Wren", "Cricket", "Dandelion", "Finch", "Oak", "Saffron", "Snowdrop", "Tansy" };
        private static (string Kind, int X, int Z)[] FoundingBlueprint(bool communal) => communal ? CommunalBlueprint : PersonalBlueprint;

        private Stockpile FoundingStore(Village v, Int2 position)
        {
            int scale = v.Communal ? 2 : 1;
            var pile = new Stockpile { Id = Id("store"), Position = position, Capacity = 2000 * scale };
            foreach (var s in new[] { new Stack("food", 50), new Stack("water", 100), new Stack("herbs", 16), new Stack("materials", 60), new Stack("planks", 10), new Stack("blocks", 10) })
                pile.Goods.Add(new Stack(s.Resource, s.Amount * scale));
            v.Stockpiles.Add(pile);
            return pile;
        }

        /// <summary>Founders: 15 per scale step, five to a den in blueprint order, the first cat leads.</summary>
        private void AddFounders(Village v, IReadOnlyList<Int2> arrivals)
        {
            int scale = v.Communal ? 2 : 1;
            for (int i = 0; i < 15 * scale; i++)
            {
                var c = NewCat(v, FounderNames[(i + (int)(Seed % 6)) % FounderNames.Length], arrivals[i % arrivals.Count]);
                c.BedId = v.Buildings.Where(b => b.Kind == "den").ElementAt(i / 5).Id;
                var labor = Catalog.Labors[(int)(Hash(c.Id) % Catalog.Labors.Length)];
                c.Preferences.Add(labor);
                Add(c.Skills, labor, 1 + Hash(c.Id + "skill") % 5);
                v.Cats.Add(c);
            }
            v.LeaderId = v.Cats[0].Id;
        }

        // ---- Version 2 founding --------------------------------------------------------------

        /// <summary>The starter resources every founding provides as one 200-unit deposit (food: three).</summary>
        private static readonly string[] StarterResources = { "logs", "stone", "food", "fibre", "ore", "clay", "sand", "gem" };
        private const int StarterFoodDeposits = 3;
        private static int Ring(Int2 center, Int2 p) => Math.Max(Math.Abs(p.X - center.X), Math.Abs(p.Z - center.Z));

        /// <summary>
        /// Version 2 founding. The natural ring to <see cref="ColonyRevealRadius"/> is known at once, so a
        /// stranger sees forest, water and hills beyond the fence. Inside, the classic walls, gates and roads;
        /// the seeded blueprint with a spare den; founders scattered over the plaza. Outside, only a dry
        /// footpath ring is cleared and the nearest natural deposits are upgraded, off the expansion rings.
        /// </summary>
        private Village FoundColony(Village v)
        {
            var center = v.Center;
            int radius = v.Radius;
            foreach (var p in RevealOrder(center, ColonyRevealRadius))
            {
                TileAt(p);
                v.Known.Add(p);
            }
            for (int z = -radius; z <= radius; z++)
                for (int x = -radius; x <= radius; x++)
                {
                    var p = new Int2(center.X + x, center.Z + z);
                    var t = TileAt(p);
                    ClearSurface(t, 0);
                    v.ClaimedTiles.Add(p);
                    t.ClaimId = v.Id;
                    t.Wall = (Math.Abs(x) == radius || Math.Abs(z) == radius) && !LayoutGate(v, p, radius);
                    t.Road = (x == 0 || z == 0) && Math.Max(Math.Abs(x), Math.Abs(z)) >= 2 || Math.Max(Math.Abs(x), Math.Abs(z)) == 2;
                }
            // One dry footpath ring keeps every gate usable; it is worn to dirt only where the gate roads cross it.
            for (int z = -radius - 1; z <= radius + 1; z++)
                for (int x = -radius - 1; x <= radius + 1; x++)
                {
                    if (Math.Max(Math.Abs(x), Math.Abs(z)) != radius + 1)
                        continue;
                    var t = TileAt(new Int2(center.X + x, center.Z + z));
                    ClearSurface(t, 0);
                    t.Wall = false;
                    t.Road = t.Dirt = x == 0 || z == 0;
                }
            var reach = FoundingReach(v);
            PlaceStarterDeposits(v, reach);
            if (!v.Known.Any(p => Ring(center, p) <= radius + 8 && GetTile(p).Water))
                PlaceFoundingPond(v, reach);

            v.Buildings.Add(new Building { Id = Id("shrine"), Kind = "shrine", Position = new Int2(center.X - 1, center.Z - 1), Width = 3, Depth = 3, Completed = true });
            var layout = ColonyLayout(v);
            foreach (var entry in layout.Take(layout.Count - 1))
                v.Buildings.Add(new Building { Id = Id(entry.Kind), Kind = entry.Kind, Position = new Int2(center.X + entry.Rect.X, center.Z + entry.Rect.Z), Width = entry.Rect.Width, Depth = entry.Rect.Depth, Completed = true, ConstructionConsumed = true });
            var store = layout[layout.Count - 1].Rect;
            FoundingStore(v, new Int2(center.X + store.X, center.Z + store.Z));
            foreach (var building in v.Buildings.Where(b => b.Kind != "den" && b.Kind != "shrine"))
                Commission(v, building);
            foreach (var building in v.Buildings.Where(b => b.Kind != "shrine"))
                ConnectFoundingBuilding(v, building);
            AddFounders(v, PlazaTiles(v));
            return v;
        }

        /// <summary>Square rings outward from the centre: nearest-first scans over Known stay cheap and stable.</summary>
        private static IEnumerable<Int2> RevealOrder(Int2 center, int reveal)
        {
            for (int d = 0; d <= reveal; d++)
                for (int z = -d; z <= d; z++)
                    for (int x = -d; x <= d; x += Math.Abs(z) == d || d == 0 ? 1 : 2 * d)
                        yield return new Int2(center.X + x, center.Z + z);
        }

        /// <summary>Revealed ground a cat can walk to from the footpath ring.</summary>
        private HashSet<Int2> FoundingReach(Village v)
        {
            var reach = new HashSet<Int2>();
            var pending = new Queue<Int2>();
            foreach (var p in v.Known)
                if (Ring(v.Center, p) == v.Radius + 1 && Walkable(p) && reach.Add(p))
                    pending.Enqueue(p);
            while (pending.Count > 0)
            {
                var at = pending.Dequeue();
                foreach (var next in Neighbors(at))
                {
                    int ring = Ring(v.Center, next);
                    if (ring > v.Radius + 1 && ring <= ColonyRevealRadius && !reach.Contains(next) && Walkable(next) && Crossable(at, next))
                    {
                        reach.Add(next);
                        pending.Enqueue(next);
                    }
                }
            }
            return reach;
        }

        /// <summary>The Root Hollow trail leaves the north gate; deposits and ponds keep clear of it.</summary>
        private static bool DungeonApron(Village v, Int2 p) => Math.Abs(p.X - v.Center.X) <= 5 && p.Z - v.Center.Z >= v.Radius + 1 && p.Z - v.Center.Z <= v.Radius + 9;

        /// <summary>
        /// Rings radius + 3, + 5 and + 7 (and anything beyond + 7) never carry a future expansion wall,
        /// so starter deposits there survive the first expansion.
        /// </summary>
        private bool StarterSite(Village v, HashSet<Int2> reach, Int2 p)
        {
            int ring = Ring(v.Center, p);
            if (ring < v.Radius + 3 || ring <= v.Radius + 7 && (ring - v.Radius) % 2 == 0 || ring > ColonyRevealRadius - 4)
                return false;
            if (!reach.Contains(p) || DungeonApron(v, p))
                return false;
            var t = GetTile(p);
            return t != null && !t.Water && !t.Mountain && !t.Wall && !t.Road && (t.ClaimId == "" || t.ClaimId == v.Id) && !t.Landform.StartsWith("dungeon", StringComparison.Ordinal) && CanModifyTerrain(v.Id, p);
        }

        /// <summary>
        /// Upgrades the nearest natural tile of each starter resource to 200 units; three food tiles sit on
        /// ring radius + 3. Only when the reveal lacks a resource does a free tile on ring + 3, + 5 or + 7
        /// receive it, so the starter goods still follow the environment wherever it allows.
        /// </summary>
        private void PlaceStarterDeposits(Village v, HashSet<Int2> reach)
        {
            var chosen = new HashSet<Int2>();
            uint Order(Int2 p) => SurfaceHash(p.X, p.Z, 701);
            long Euclid(Int2 p) => (long)(p.X - v.Center.X) * (p.X - v.Center.X) + (long)(p.Z - v.Center.Z) * (p.Z - v.Center.Z);
            foreach (string resource in StarterResources)
            {
                bool food = resource == "food";
                int wanted = food ? StarterFoodDeposits : 1;
                var sites = v.Known.Where(p => (!food || Ring(v.Center, p) == v.Radius + 3) && !chosen.Contains(p) && StarterSite(v, reach, p)).ToList();
                var picks = sites.Where(p => GetTile(p).Resource == resource && GetTile(p).Amount > 0)
                    .OrderBy(p => Ring(v.Center, p)).ThenBy(Euclid).ThenBy(Order).Take(wanted).ToList();
                foreach (int ring in food ? new[] { v.Radius + 3 } : new[] { v.Radius + 3, v.Radius + 5, v.Radius + 7 })
                {
                    if (picks.Count >= wanted)
                        break;
                    picks.AddRange(sites.Where(p => Ring(v.Center, p) == ring && !picks.Contains(p) && Array.IndexOf(StarterResources, GetTile(p).Resource) < 0)
                        .OrderBy(p => GetTile(p).Resource == "" ? 0 : 1).ThenBy(Order).Take(wanted - picks.Count));
                }
                foreach (var p in picks)
                {
                    var t = GetTile(p);
                    t.Resource = resource;
                    t.Amount = Math.Max(t.Amount, 200);
                    t.RegrowAt = -1;
                    chosen.Add(p);
                }
            }
        }

        /// <summary>Only when no natural water lies within 8 tiles of the fence: a pond off the expansion rings.</summary>
        private void PlaceFoundingPond(Village v, HashSet<Int2> reach)
        {
            var sites = v.Known.Where(p => Ring(v.Center, p) == v.Radius + 3 && StarterSite(v, reach, p) && Array.IndexOf(StarterResources, GetTile(p).Resource) < 0)
                .OrderBy(p => SurfaceHash(p.X, p.Z, 709)).Take(1).ToList();
            foreach (var site in sites)
            {
                var t = GetTile(site);
                SetWaterSurface(t, t.Elevation, .8);
            }
        }

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
    }
}
