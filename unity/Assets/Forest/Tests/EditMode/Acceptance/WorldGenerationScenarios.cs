using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    /// <summary>Seeded terrain is independent of exploration and preserves the legacy generator.</summary>
    public static class WorldGenerationScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.world_generation_elevation_and_biomes", ElevationAndBiomes);
            yield return new Scenario("regression.world_generation_coordinate_order_and_entropy", CoordinateOrder);
            yield return new Scenario("regression.world_generation_connected_river_and_fords", RiverAndFords);
            yield return new Scenario("regression.world_generation_lake_depth_shelves", LakeShelves);
            yield return new Scenario("regression.world_generation_ecological_resources", EcologicalResources);
            yield return new Scenario("regression.world_generation_legacy_coordinates", LegacyCoordinates);
            yield return new Scenario("regression.world_generation_saved_height_read_only", SavedHeight);
            yield return new Scenario("regression.world_generation_founding_clear_and_pond", ClearAndPond);
            yield return new Scenario("regression.world_generation_lazy_founding_grade", LazyFoundingGrade);
            yield return new Scenario("regression.world_generation_grade_preserves_foreign", GradePreservesForeign);
            yield return new Scenario("regression.world_generation_grade_preserves_dungeon_site", () =>
            {
                var world = New(); var site = new DungeonSite { Id = "existing", Entrance = new Int2(500, 500) }; world.Dungeons.Add(site);
                Require(!world.CanPrepareFoundingTerrain("other", site.Entrance, 6), "Founding can grade over an existing dungeon entrance and its stairs");
            });
            yield return new Scenario("regression.world_generation_rejects_unsupported_version", () =>
            {
                var world = World.Create(41); world.GenerationVersion = 99;
                Require(world.Validate().Any(e => e.Contains("generation version")), "Unsupported generation version passed save validation");
            });
        }

        private static World New(uint seed = 41) => new World { Seed = seed, RandomState = 1234, GenerationVersion = 1 };
        private static void Require(bool value, string message)
        {
            if (!value)
                throw new InvalidOperationException(message);
        }
        private static string Fingerprint(Tile t) => t.Biome + ":" + t.Landform + ":" + t.Resource + ":" + t.Amount + ":" + t.Elevation.ToString("R", System.Globalization.CultureInfo.InvariantCulture) + ":" + t.WaterSurface.ToString("R", System.Globalization.CultureInfo.InvariantCulture) + ":" + t.WaterDepth.ToString("R", System.Globalization.CultureInfo.InvariantCulture) + ":" + t.Water + ":" + t.Mountain;

        private static void ElevationAndBiomes()
        {
            var w = New();
            var tiles = new List<Tile>();
            int adjacent = 0, same = 0;
            foreach (int z in Enumerable.Range(-64, 129))
                foreach (int x in Enumerable.Range(-64, 129))
                {
                    var tile = w.GenerateSurfaceTile(new Int2(x, z));
                    var next = w.GenerateSurfaceTile(new Int2(x + 1, z));
                    tiles.Add(tile);
                    adjacent++;
                    if (tile.Biome == next.Biome)
                        same++;
                    if (!tile.Water && !next.Water)
                        Require(Math.Abs(tile.Elevation - next.Elevation) <= 0.65, "Ordinary land contains an unwalkable elevation seam at " + tile.Position);
                }
            Require(tiles.Max(t => t.Elevation) - tiles.Min(t => t.Elevation) > 3, "Terrain still has one flat ground height");
            Require(same > adjacent * 0.9, "Biome regions lack coherent neighboring climate");
            var biomes = new HashSet<string>();
            foreach (int seed in new[] { 7, 41, 127 })
            {
                w = New((uint)seed);
                for (int z = -256; z <= 256; z += 8)
                    for (int x = -256; x <= 256; x += 8)
                        biomes.Add(w.GenerateSurfaceTile(new Int2(x, z)).Biome);
            }
            foreach (string biome in new[] { "forest", "meadow", "pine_forest", "highland", "wetland" })
                Require(biomes.Contains(biome), "Missing climate biome " + biome);
        }

        private static void CoordinateOrder()
        {
            var first = New();
            var second = New();
            var points = new[] { new Int2(-1, -1), new Int2(-5, 0), new Int2(0, -5), new Int2(5, 0), new Int2(0, 5), new Int2(-113, 127), new Int2(113, -127), new Int2(int.MinValue + 2, int.MaxValue - 2), new Int2(int.MaxValue - 2, int.MinValue + 2) };
            var expected = points.ToDictionary(p => p, p => Fingerprint(first.GenerateSurfaceTile(p)));
            foreach (var point in points.Reverse())
                Require(expected[point] == Fingerprint(second.GenerateSurfaceTile(point)), "Exploration order changed terrain at " + point);
            Require(first.RandomState == 1234 && second.RandomState == 1234 && first.Tiles.Count == 0 && second.Tiles.Count == 0, "Terrain sampling consumed actor entropy or inserted hidden tiles");
            Require(points.Any(p => expected[p] != Fingerprint(New(42).GenerateSurfaceTile(p))), "Different seeds generated identical terrain");
            double extreme = first.GenerateSurfaceTile(points[7]).Elevation;
            Require(!double.IsNaN(extreme) && !double.IsInfinity(extreme), "Extreme signed coordinates generated invalid height");
        }

        private static void RiverAndFords()
        {
            var w = New();
            bool ford = false, deep = false;
            HashSet<int> previous = null;
            for (int z = -64; z <= 64; z++)
            {
                var row = new List<Tile>();
                for (int x = -96; x <= 96; x++)
                {
                    var t = w.GenerateSurfaceTile(new Int2(x, z));
                    if (t.Landform == "river_bank")
                        Require(!t.Water && t.WaterDepth == 0, "River bank is submerged");
                    else if (t.Landform.StartsWith("river", StringComparison.Ordinal))
                        row.Add(t);
                }
                Require(row.Count > 0, "River channel is interrupted at row " + z);
                var positions = new HashSet<int>(row.Select(t => t.Position.X));
                if (previous != null)
                    Require(positions.Overlaps(previous), "River rows are diagonally disconnected");
                previous = positions;
                ford |= row.Any(t => t.Landform == "river_ford") && row.All(t => t.WaterDepth <= 0.45);
                deep |= row.Any(t => t.WaterDepth >= 1.2);
                Require(row.All(t => t.Water && t.WaterDepth > 0 && Math.Abs(t.WaterSurface - t.Elevation - t.WaterDepth) < 1e-9), "River has inconsistent authoritative bed and water levels");
            }
            Require(ford && deep, "River lacks both a complete wading crossing and a deep channel");
        }

        private static void LakeShelves()
        {
            var w = New(7);
            var depths = new HashSet<string>();
            double deepest = 0;
            for (int z = -96; z <= 96; z++)
                for (int x = -96; x <= 96; x++)
                {
                    var t = w.GenerateSurfaceTile(new Int2(x, z));
                    if (t.Landform == "lake_bank")
                    {
                        Require(!t.Water && t.WaterDepth == 0, "Lake bank is submerged");
                        continue;
                    }
                    if (!t.Landform.StartsWith("lake_", StringComparison.Ordinal))
                        continue;
                    depths.Add(t.Landform);
                    deepest = Math.Max(deepest, t.WaterDepth);
                    Require(t.Water && t.WaterDepth > 0 && Math.Abs(t.WaterSurface - t.Elevation - t.WaterDepth) < 1e-9, "Lake does not preserve bed and water height");
                }
            Require(depths.Contains("lake_shallow") && depths.Contains("lake_shelf") && depths.Contains("lake_deep") && deepest >= 3, "Lake lacks shore, middle shelf or a genuinely deep basin");
        }

        private static void EcologicalResources()
        {
            var w = New();
            var resources = new HashSet<string>();
            for (int z = -128; z <= 128; z += 2)
                for (int x = -128; x <= 128; x += 2)
                {
                    var t = w.GenerateSurfaceTile(new Int2(x, z));
                    resources.Add(t.Resource);
                    Require(t.Resource != "fish" || t.Water, "Fish generated on dry land");
                    Require(!t.Water || t.Resource == "fish", "Submerged trees or mineral deposits generated in water");
                    Require(t.Resource != "logs" || t.Biome.Contains("forest"), "Trees ignored climate biome");
                    Require(t.Resource != "ore" && t.Resource != "gem" || t.Biome == "highland", "Deep mineral resources ignored geology");
                }
            foreach (var resource in new[] { "logs", "food", "fish", "stone", "fibre", "clay", "sand", "ore", "gem", "herbs" })
                Require(resources.Contains(resource), "Landscape lacks reachable natural resource " + resource);
        }

        private static void LegacyCoordinates()
        {
            var w = new World { Seed = 41 };
            Require(w.GenerationVersion == 0, "Deserializing an old save silently opts into new generation");
            var examples = new[] { (-4, -4, "beach", "sand", 48, false, false), (-5, -5, "snow_mountains", "", 0, false, true), (0, 0, "beach", "", 0, false, false), (5, 5, "swamp", "", 0, false, false), (73, -109, "lake", "fish", 24, true, false) };
            foreach (var expected in examples)
            {
                var point = new Int2(expected.Item1, expected.Item2);
                var actual = w.GenerateSurfaceTile(point);
                Require(actual.Biome == expected.Item3 && actual.Resource == expected.Item4 && actual.Amount == expected.Item5 && actual.Water == expected.Item6 && actual.Mountain == expected.Item7, "Legacy unexplored terrain changed at " + point);
                Require(actual.Elevation == 0 && actual.WaterDepth == 0, "Legacy world acquired unrequested terrain heights");
            }
        }

        private static void SavedHeight()
        {
            var w = New();
            var point = new Int2(0, 0);
            var tile = new Tile { Position = point, Elevation = -1.7, WaterSurface = 0.3, WaterDepth = 2, Water = true };
            w.Tiles.Add(tile);
            Require(w.SurfaceHeight(point) == -1.7 && w.WalkHeight(point) == -1.7, "Terrain sampling ignored saved bed elevation");
            tile.Bridge = true;
            Require(Math.Abs(w.WalkHeight(point) - 0.38) < 1e-9, "Bridge deck does not clear the water surface");
            Require(w.SurfaceHeight(new Int2(1000, 1000)) == 0 && w.Tiles.Count == 1 && w.RandomState == 1234, "Camera height lookup generated hidden terrain");
        }

        private static void ClearAndPond()
        {
            var tile = new Tile { Water = true, Mountain = true, Elevation = -4, WaterDepth = 6, WaterSurface = 2, Resource = "fish", Amount = 24, FishCapacity = 24, ClaimId = "communal", Road = true };
            tile.Deposits.Add(new Stack("clay", 2));
            World.ClearSurface(tile, 1.25);
            Require(!tile.Water && !tile.Mountain && tile.WaterDepth == 0 && tile.Elevation == 1.25 && tile.WaterSurface == 1.25 && tile.Resource == "" && tile.Amount == 0 && tile.FishCapacity == 0 && tile.Deposits.Count == 0, "Founding clearance retained underwater height or hidden goods");
            Require(tile.ClaimId == "communal" && tile.Road, "Terrain clearance destroyed ownership or built topology");
            World.SetWaterSurface(tile, 1.25, 0.35);
            Require(tile.Water && tile.WaterDepth == 0.35 && Math.Abs(tile.Elevation - 0.9) < 1e-9 && tile.Resource == "fish" && tile.FishCapacity == 24, "Founding pond has no physical bed or finite fish");
        }

        private static void LazyFoundingGrade()
        {
            var a = New();
            var b = New();
            var village = new Village { Id = "test", Center = new Int2(-170, 151), Radius = 6 };
            var underground = new Tile { Position = new Int2(village.Center.X, village.Center.Z, -1), Elevation = -4, Biome = "cave", Resource = "gem", Amount = 7.25, Dirt = true };
            a.Tiles.Add(underground);
            string caveBefore = Fingerprint(underground);
            var points = Enumerable.Range(-34, 69).SelectMany(z => Enumerable.Range(-34, 69).Select(x => new Int2(village.Center.X + x, village.Center.Z + z))).ToArray();
            foreach (var point in points.Where((p, index) => index % 17 == 0))
                a.TileAt(point);
            int generated = a.Tiles.Count;
            a.PrepareFoundingTerrain(village);
            b.PrepareFoundingTerrain(village);
            Require(a.Tiles.Count == generated && b.Tiles.Count == 0, "Preparing one settlement eagerly generated its entire terrain shoulder");
            Require(Fingerprint(underground) == caveBefore && underground.Position.Level == -1 && underground.Dirt, "Founding surface grading changed an existing dungeon floor or its finite deposit");
            foreach (var point in points)
            {
                var left = a.GetTile(point) ?? a.GenerateSurfaceTile(point);
                var right = b.GenerateSurfaceTile(point);
                Require(left.Elevation == right.Elevation && left.WaterDepth == right.WaterDepth && left.WaterSurface == right.WaterSurface && left.Water == right.Water, "Exploration before founding changed graded geometry at " + point);
                if (Math.Max(Math.Abs(point.X - village.Center.X), Math.Abs(point.Z - village.Center.Z)) <= village.Radius + 6)
                    Require(left.Elevation == 0 && !left.Water && !left.Mountain, "Founding plateau does not contain usable exterior resource paths");
                var next = a.GenerateSurfaceTile(new Int2(point.X + 1, point.Z));
                if (!left.Water && !next.Water)
                    Require(Math.Abs(left.Elevation - next.Elevation) <= 0.65, "Founding shoulder has a blocked dry elevation seam");
            }
        }

        private static void GradePreservesForeign()
        {
            var w = New();
            var village = new Village { Id = "new", Center = new Int2(0, 0), Radius = 6 };
            var tile = w.TileAt(new Int2(23, 0));
            tile.ClaimId = "foreign";
            tile.Resource = "logs";
            tile.Amount = 71;
            string before = Fingerprint(tile);
            Require(!w.CanPrepareFoundingTerrain(village.Id, village.Center, village.Radius), "Founding ignored foreign territory in its graded shoulder");
            bool rejected = false;
            try { w.PrepareFoundingTerrain(village); }
            catch (InvalidOperationException) { rejected = true; }
            Require(rejected && Fingerprint(tile) == before && tile.ClaimId == "foreign" && w.SurfacePlateaus.Count == 0, "Blocked founding mutated a foreign tile or registered its terrain profile");
        }
    }
}
