using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>
    /// Founding: classic worlds keep their exact layout; new demo colonies (generation version 2)
    /// start in a revealed, forested landscape with water, a landmark and room to expand.
    /// </summary>
    public static class FoundingScenarios
    {
        // SHA-256 of the founded layout of World.Create(41), recorded before generation version 2
        // existed. A change here means classic geometry moved and every classic coordinate is suspect.
        const string ClassicLayoutHash41 = "d87c8aa9b8fd592c765894e0b50c102ae8338efae0e2d6edff5fe4a00a766c04";

        // The starter resources every founding upgrades to a 200-unit deposit.
        static readonly string[] StarterResources = { "logs", "stone", "food", "fibre", "ore", "clay", "sand", "gem" };

        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("founding.classic_layout_hash_unchanged", () =>
            {
                string actual = LayoutHash(World.Create(41));
                Check(actual == ClassicLayoutHash41, "Classic founded layout of World.Create(41) changed: " + actual);
            });
            yield return new Scenario("founding.colony_name_and_profile", NameAndProfile);
            // 150 seeds in three parallel cases: 1-50, 51-100, 101-149 plus the shipped default 4242.
            yield return new Scenario("founding.demo_colony_seeds_001_050", () => DemoColonySeeds("001-050", Enumerable.Range(1, 50)));
            yield return new Scenario("founding.demo_colony_seeds_051_100", () => DemoColonySeeds("051-100", Enumerable.Range(51, 50)));
            yield return new Scenario("founding.demo_colony_seeds_101_149_and_4242", () => DemoColonySeeds("101-149+4242", Enumerable.Range(101, 49).Concat(new[] { 4242 })));
            yield return new Scenario("founding.colony_blueprint_varies_and_connects", BlueprintVariesAndConnects);
            yield return new Scenario("founding.colony_refounds_after_extinction", RefoundsAfterExtinction);
        }

        private static string R(double value) => value.ToString("R", CultureInfo.InvariantCulture);
        private static string P(Int2 p) => p.X.ToString(CultureInfo.InvariantCulture) + "," + p.Z.ToString(CultureInfo.InvariantCulture) + "," + p.Level.ToString(CultureInfo.InvariantCulture);
        private static string Goods(IEnumerable<Stack> stacks) => string.Join(";", stacks.Select(s => s.Resource + "=" + R(s.Amount)));
        internal static int Ring(Village v, Int2 p) => Math.Max(Math.Abs(p.X - v.Center.X), Math.Abs(p.Z - v.Center.Z));
        private static bool Inside(Building b, Int2 p) => p.Level == b.Position.Level && p.X >= b.Position.X && p.X < b.Position.X + b.Width && p.Z >= b.Position.Z && p.Z < b.Position.Z + b.Depth;

        private static void NameAndProfile()
        {
            var named = World.CreateColony(5, "  Fernhold ", Pacing.DemoProfile, 1700000000000);
            var v = named.Villages.Single();
            Check(v.Name == "Fernhold" && v.Id == "communal" && v.Communal, "Colony name was not trimmed onto the founding colony");
            Check(named.GenerationVersion == 2 && named.PacingProfile == Pacing.DemoProfile && named.EpochUnixMs == 1700000000000, "Colony did not use generation version 2, the demo profile and the given epoch");
            Check(v.NextElection == named.TimeSeconds + named.CurrentPacing().ElectionIntervalSeconds, "Founding ignored the profile's election interval");
            Check(World.CreateColony(5, "   ", Pacing.DemoProfile, 0).Villages.Single().Name == World.DefaultColonyName, "A blank colony name did not fall back to the default");
            Check(World.CreateColony(5, null, "unknown", 0).PacingProfile == Pacing.ClassicProfile, "An unknown pacing profile was stored instead of classic");
            bool rejected = false;
            try { World.CreateColony(5, new string('a', World.MaxColonyNameLength + 1), Pacing.DemoProfile, 0); }
            catch (ArgumentOutOfRangeException) { rejected = true; }
            Check(rejected, "An over-long colony name was accepted");
            // Same seed, same world: the new-game path is deterministic.
            Check(LayoutHash(World.CreateColony(5, "Fernhold", Pacing.DemoProfile, 0)) == LayoutHash(World.CreateColony(5, "Fernhold", Pacing.DemoProfile, 0)), "CreateColony is not deterministic for one seed");
            // Validation must accept a fresh colony. W2-INT-a: remove this one exception once
            // World.Validate accepts generation version 2 (W2-07 handoff).
            var errors = named.Validate().Where(e => e != "Unsupported world generation version").ToList();
            Check(errors.Count == 0, "Fresh colony failed validation: " + string.Join("; ", errors));
        }

        internal sealed class ColonyStats
        {
            public int Seed, Logs, Landmarks, Highland, NearestWater, Food;
            public double Relief;
            public bool Lake;
            public double Score => Logs / 10.0 + Math.Min(Landmarks, 30) + Math.Min(Highland, 400) / 20.0 + Relief * 4 + (Lake ? 8 : 0) + (NearestWater <= 14 ? 6 : 0);
            public override string ToString() => "seed " + Seed + " score " + Score.ToString("F1", CultureInfo.InvariantCulture) + " logs " + Logs + " landmark tiles " + Landmarks + " highland " + Highland + " relief " + Relief.ToString("F2", CultureInfo.InvariantCulture) + " water at " + NearestWater + (Lake ? " (lake)" : " (river)");
        }

        private static void DemoColonySeeds(string label, IEnumerable<int> seeds)
        {
            var stats = seeds.Select(CheckDemoColony).ToList();
            Console.WriteLine("Best demo seeds " + label + ":");
            foreach (var s in stats.OrderByDescending(s => s.Score).ThenBy(s => s.Seed).Take(3))
                Console.WriteLine("  " + s);
            var shipped = stats.FirstOrDefault(s => s.Seed == 4242);
            if (shipped != null)
                Console.WriteLine("Shipped default: " + shipped);
        }

        /// <summary>Every guarantee a stranger's first look depends on, for one seed.</summary>
        internal static ColonyStats CheckDemoColony(int seed)
        {
            var w = World.CreateColony(seed, World.DefaultColonyName, Pacing.DemoProfile, 0);
            var v = w.Villages.Single();
            string at = " (seed " + seed + ")";
            Check(w.GenerationVersion == 2 && w.PacingProfile == Pacing.DemoProfile && v.Name == World.DefaultColonyName, "Colony is not a named demo colony" + at);
            var known = new HashSet<Int2>(v.Known);
            Check(known.Count == v.Known.Count, "Known tiles contain duplicates" + at);
            for (int z = -30; z <= 30; z++)
                for (int x = -30; x <= 30; x++)
                    Check(known.Contains(new Int2(v.Center.X + x, v.Center.Z + z)), "Founding reveal does not cover radius 30" + at);
            var tiles = v.Known.Select(w.GetTile).ToList();
            Check(tiles.All(t => t != null), "A known tile was never generated" + at);

            int logs = tiles.Count(t => t.Biome.Contains("forest") && t.Resource == "logs" && t.Amount > 0);
            Check(logs >= 300, "Reveal holds only " + logs + " forest tiles with trees" + at);

            var water = tiles.Where(t => t.Water).ToList();
            Check(water.Count > 0, "No river or lake in the reveal" + at);
            int nearestWater = water.Min(t => Ring(v, t.Position));
            Check(nearestWater >= v.Radius + 3 && nearestWater <= v.Radius + 8, "Nearest water is " + (nearestWater - v.Radius) + " tiles outside the fence, not 3 to 8" + at);

            int landmarks = tiles.Count(t => t.Mountain && (t.Landform == "outcrop" || t.Landform == "ridge"));
            Check(landmarks > 0, "No outcrop or ridge landmark in the reveal" + at);

            // Outside the walls, grading never leaves a step a cat cannot climb between two walkable tiles.
            var plateau = w.SurfacePlateaus.Single(p => p.VillageId == v.Id);
            Check(plateau.CoreRadius == v.Radius + 2 && plateau.Shoulder >= 8 && plateau.Shoulder <= 20, "Colony plateau is not radius + 2 with an 8 to 20 tile shoulder" + at);
            foreach (var p in v.Known.Where(p => Ring(v, p) > v.Radius && Ring(v, p) <= plateau.CoreRadius + plateau.Shoulder))
                foreach (var next in new[] { new Int2(p.X + 1, p.Z), new Int2(p.X, p.Z + 1) })
                    if (known.Contains(next) && Ring(v, next) > v.Radius && w.Walkable(p) && w.Walkable(next))
                        Check(w.Crossable(p, next), "Graded ground has an unclimbable step between " + p + " and " + next + at);

            var film = w.Tiles.FirstOrDefault(t => t.Position.Level == 0 && t.Water && t.WaterDepth < 0.15);
            Check(film == null, "Film water at " + film?.Position + " depth " + film?.WaterDepth + at);

            int food = tiles.Count(t => t.Resource == "food" && t.Amount > 0 && !t.Water && Ring(v, t.Position) <= 12);
            Check(food >= 3, "Only " + food + " food deposits within 12 tiles" + at);

            var site = w.Dungeons.SingleOrDefault(d => d.Id == "dungeon:" + v.Id);
            Check(site != null, "Colony has no Root Hollow entrance" + at);
            for (int dz = -2; dz <= 2; dz++)
                for (int dx = -2; dx <= 2; dx++)
                {
                    var near = w.GetTile(new Int2(site.Entrance.X + dx, site.Entrance.Z + dz));
                    Check(near != null && !near.Water, "Dungeon entrance is within 2 tiles of water" + at);
                }

            var start = new Int2(v.Center.X, v.Center.Z - 2);
            foreach (string resource in StarterResources)
            {
                var deposit = tiles.Where(t => t.Resource == resource && t.Amount >= 200).OrderBy(t => Ring(v, t.Position)).FirstOrDefault();
                Check(deposit != null, "No starter " + resource + " deposit" + at);
                Check(Ring(v, deposit.Position) > v.Radius + 2 && (Ring(v, deposit.Position) - v.Radius) % 2 == 1 || Ring(v, deposit.Position) > v.Radius + 7, "Starter " + resource + " sits on a future expansion wall ring" + at);
                Check(w.Path(start, deposit.Position, v) != null, "Starter " + resource + " deposit at " + deposit.Position + " is unreachable" + at);
            }

            int highland = tiles.Count(t => t.Biome == "highland");
            double relief = tiles.Where(t => !t.Water).Max(t => t.Elevation) - tiles.Where(t => !t.Water).Min(t => t.Elevation);
            bool lake = water.Any(t => t.Biome == "lake");

            // Last, because it changes the colony: a fresh colony can expand without any clearing.
            var builder = v.Cats.First(c => c.Alive && c.JobId == "");
            var expand = w.Apply(Context(v), new GameAction { Kind = "RequestJob", Name = "expand", CatId = builder.Id });
            Check(expand.Success, "A fresh colony cannot expand: " + expand.Error + at);
            return new ColonyStats { Seed = seed, Logs = logs, Landmarks = landmarks, Highland = highland, NearestWater = nearestWater - v.Radius, Food = food, Relief = relief, Lake = lake };
        }

        private static string Blueprint(Village v) => string.Join(" ", v.Buildings.Select(b => b.Kind + "@" + (b.Position.X - v.Center.X) + "," + (b.Position.Z - v.Center.Z)));

        private static void BlueprintVariesAndConnects()
        {
            var layouts = new HashSet<string>();
            var classic = World.Create(41).Villages.Single();
            bool moved = false;
            foreach (int seed in Enumerable.Range(1, 12))
            {
                var w = World.CreateColony(seed, World.DefaultColonyName, Pacing.DemoProfile, 0);
                var v = w.Villages.Single();
                string at = " (seed " + seed + ")";
                layouts.Add(Blueprint(v));
                moved |= Blueprint(v) != Blueprint(classic);
                var dens = v.Buildings.Where(b => b.Kind == "den").ToList();
                Check(dens.Count == classic.Buildings.Count(b => b.Kind == "den") + 1, "Colony lacks exactly one spare den" + at);
                Check(dens.Any(d => v.Cats.All(c => c.BedId != d.Id)) && v.Cats.All(c => dens.Any(d => d.Id == c.BedId)), "Founders are not all housed with one den to spare" + at);
                Check(dens.All(d => v.Cats.Count(c => c.BedId == d.Id) <= w.DenCapacity(v)), "A founding den is overbooked" + at);
                Check(v.Buildings.Select(b => b.Kind).OrderBy(k => k).SequenceEqual(classic.Buildings.Select(b => b.Kind).Concat(new[] { "den" }).OrderBy(k => k)), "Colony blueprint lost or gained a station" + at);
                foreach (var b in v.Buildings)
                {
                    for (int x = 0; x < b.Width; x++)
                        for (int z = 0; z < b.Depth; z++)
                        {
                            var p = new Int2(b.Position.X + x, b.Position.Z + z);
                            Check(Ring(v, p) < v.Radius, b.Kind + " leaves the walls" + at);
                            Check(b.Kind == "shrine" || !w.GetTile(p).Road, b.Kind + " covers a road" + at);
                            Check(v.Buildings.Where(o => o != b).All(o => !Inside(o, p)), b.Kind + " overlaps another building" + at);
                        }
                    if (b.Kind != "shrine")
                        Check(b.HasEntrance && w.BuildingEntrance(v, b) != null, b.Kind + " has no connected road entrance" + at);
                }
            }
            Check(layouts.Count >= 4, "Only " + layouts.Count + " distinct colony layouts across 12 seeds");
            Check(moved, "Every colony repeats the classic blueprint");
        }

        private static void RefoundsAfterExtinction()
        {
            var w = World.CreateColony(4242, World.DefaultColonyName, Pacing.DemoProfile, 0);
            var v = w.Villages.Single();
            int known = v.Known.Count, plateaus = w.SurfacePlateaus.Count;
            foreach (var c in v.Cats)
                c.Alive = false;
            w.Step(61);
            var replacement = w.Villages.Single();
            Check(replacement != v && replacement.Cats.Count(c => c.Alive) == v.Cats.Count, "Extinct colony did not refound");
            Check(replacement.Known.Count == known && w.SurfacePlateaus.Count == plateaus, "Refounding changed the reveal or registered a second terrain profile");
            Check(StarterResources.All(r => replacement.Known.Select(w.GetTile).Any(t => t.Resource == r && t.Amount >= 200)), "Refounding did not restore every starter deposit");
        }

        /// <summary>Geometry and identities a founding produces; actor entropy and stats are deliberately excluded.</summary>
        internal static string LayoutHash(World w)
        {
            var text = new StringBuilder();
            text.Append("gen=").Append(w.GenerationVersion).Append('\n');
            foreach (var plateau in w.SurfacePlateaus)
                text.Append("plateau ").Append(plateau.VillageId).Append(' ').Append(P(plateau.Center)).Append(' ').Append(plateau.CoreRadius).Append(' ').Append(plateau.Shoulder).Append('\n');
            foreach (var t in w.Tiles)
                text.Append("tile ").Append(P(t.Position)).Append(' ').Append(t.Biome).Append(' ').Append(t.Landform).Append(' ').Append(t.Resource).Append(' ').Append(R(t.Amount)).Append(' ').Append(R(t.FishCapacity))
                    .Append(' ').Append(R(t.Elevation)).Append(' ').Append(R(t.WaterSurface)).Append(' ').Append(R(t.WaterDepth)).Append(' ').Append(t.Water).Append(t.Mountain).Append(t.Wall).Append(t.Road).Append(t.Dirt)
                    .Append(' ').Append(t.ClaimId).Append(' ').Append(Goods(t.Deposits)).Append('\n');
            foreach (var v in w.Villages)
            {
                text.Append("village ").Append(v.Id).Append(' ').Append(v.Name).Append(' ').Append(P(v.Center)).Append(' ').Append(v.Radius).Append(' ').Append(v.LayoutVersion).Append(' ').Append(v.LeaderId).Append(' ').Append(R(v.NextElection)).Append('\n');
                text.Append("known ").Append(string.Join(" ", v.Known.Select(P))).Append('\n');
                text.Append("claimed ").Append(string.Join(" ", v.ClaimedTiles.Select(P))).Append('\n');
                foreach (var b in v.Buildings)
                    text.Append("building ").Append(b.Id).Append(' ').Append(b.Kind).Append(' ').Append(P(b.Position)).Append(' ').Append(b.Width).Append('x').Append(b.Depth).Append(' ').Append(b.HasEntrance).Append(' ').Append(P(b.Entrance)).Append(' ').Append(b.Completed).Append('\n');
                foreach (var s in v.Stockpiles)
                    text.Append("pile ").Append(s.Id).Append(' ').Append(s.Kind).Append(' ').Append(s.ManagedBy).Append(' ').Append(P(s.Position)).Append(' ').Append(s.Width).Append('x').Append(s.Depth).Append(' ').Append(R(s.Capacity)).Append(' ').Append(Goods(s.Goods)).Append('\n');
                foreach (var c in v.Cats)
                    text.Append("cat ").Append(c.Id).Append(' ').Append(c.Name).Append(' ').Append(P(c.Position)).Append(' ').Append(R(c.X)).Append(' ').Append(R(c.Z)).Append(' ').Append(c.BedId).Append('\n');
            }
            foreach (var d in w.Dungeons)
            {
                text.Append("dungeon ").Append(d.Id).Append(' ').Append(P(d.Entrance)).Append(' ').Append(d.Depth).Append(' ').Append(R(d.BaseElevation)).Append('\n');
                foreach (var f in d.Floors)
                    text.Append("floor ").Append(f.Level).Append(' ').Append(P(f.Spawn)).Append(' ').Append(P(f.LootPosition)).Append(' ').Append(string.Join(" ", f.Cells.Select(P))).Append('\n');
                foreach (var s in d.Stairs)
                    text.Append("stair ").Append(P(s.From)).Append(' ').Append(P(s.To)).Append(' ').Append(string.Join(" ", s.Opening.Select(P))).Append('\n');
            }
            using (var sha = SHA256.Create())
                return string.Concat(sha.ComputeHash(Encoding.UTF8.GetBytes(text.ToString())).Select(b => b.ToString("x2", CultureInfo.InvariantCulture)));
        }
    }
}
