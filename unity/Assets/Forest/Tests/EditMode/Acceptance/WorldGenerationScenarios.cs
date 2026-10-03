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
            yield return new Scenario("regression.world_generation_version_1_samples_unchanged", () =>
            {
                // Recorded before generation version 2 existed; version 1 worlds must never move.
                var text = new System.Text.StringBuilder();
                foreach (uint seed in new uint[] { 7, 41, 4242 })
                {
                    var w = New(seed);
                    for (int z = -150; z <= 150; z += 3)
                        for (int x = -150; x <= 150; x += 3)
                            text.Append(Fingerprint(w.GenerateSurfaceTile(new Int2(x, z)))).Append(':').Append(Q(w.GenerateSurfaceTile(new Int2(x, z)).FishCapacity)).Append('\n');
                }
                string actual = Sha256(text.ToString());
                Require(actual == Version1SampleHash, "Version 1 terrain samples changed: " + actual);
            });
            yield return new Scenario("regression.world_generation_v2_banks_replace_film_water", BanksReplaceFilmWater);
            yield return new Scenario("regression.world_generation_v2_outcrops_and_ridges", OutcropsAndRidges);
            yield return new Scenario("regression.world_generation_v2_coordinate_pure_with_datum", CoordinatePureVersion2);
            yield return new Scenario("regression.world_generation_v2_founding_grade", FoundingGradeVersion2);
            yield return new Scenario("regression.world_generation_rejects_unsupported_version", () =>
            {
                var world = World.Create(41); world.GenerationVersion = 99;
                Require(world.Validate().Any(e => e.Contains("generation version")), "Unsupported generation version passed save validation");
            });
        }

        const string Version1SampleHash = "9b4a167531ae7d5a483cf394ebec71071135758635bea13d530c7a21fa5eea26";
        private static World New(uint seed = 41) => new World { Seed = seed, RandomState = 1234, GenerationVersion = 1 };
        private static string Sha256(string value)
        {
            using (var sha = System.Security.Cryptography.SHA256.Create())
                return string.Concat(sha.ComputeHash(System.Text.Encoding.UTF8.GetBytes(value)).Select(b => b.ToString("x2", System.Globalization.CultureInfo.InvariantCulture)));
        }
        private static void Require(bool value, string message)
        {
            if (!value)
                throw new InvalidOperationException(message);
        }
        private static string Fingerprint(Tile t) => t.Biome + ":" + t.Landform + ":" + t.Resource + ":" + Q(t.Amount) + ":" + Q(t.Elevation) + ":" + Q(t.WaterSurface) + ":" + Q(t.WaterDepth) + ":" + t.Water + ":" + t.Mountain;
        // CoreCLR prints negative zero as "-0" and Mono (Unity) as "0", so the same world hashed differently under
        // Unity EditMode. Adding +0.0 turns -0 into +0; the rounding keeps "R" free of last-digit noise.
        private static string Q(double value) => (Math.Round(value, 9) + 0.0).ToString("R", System.Globalization.CultureInfo.InvariantCulture);

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

        private static World New2(uint seed = 41, double datum = 0) => new World { Seed = seed, RandomState = 1234, GenerationVersion = 2, SurfaceDatum = datum };
        private static int Chebyshev(Int2 a, Int2 b) => Math.Max(Math.Abs(a.X - b.X), Math.Abs(a.Z - b.Z));

        /// <summary>Version 2 never leaves millimetre water: thin edges become dry banks and fish follow the real depth.</summary>
        private static void BanksReplaceFilmWater()
        {
            int water = 0, converted = 0;
            foreach (uint seed in new uint[] { 7, 41, 4242 })
            {
                var v1 = New(seed);
                var v2 = New2(seed);
                for (int z = -48; z <= 48; z++)
                    for (int x = -96; x <= 96; x++)
                    {
                        var p = new Int2(x, z);
                        var t = v2.GenerateSurfaceTile(p);
                        if (t.Water)
                        {
                            water++;
                            Require(t.WaterDepth >= 0.15, "Film water at " + p + " depth " + t.WaterDepth);
                            Require(t.Resource == "fish" && t.Amount == t.FishCapacity && t.FishCapacity == 24 + Math.Floor(t.WaterDepth * 4), "Fish stock ignores the real water depth at " + p);
                            Require(Math.Abs(t.WaterSurface - t.Elevation - t.WaterDepth) < 1e-9, "Water bed and surface disagree at " + p);
                            continue;
                        }
                        Require(t.WaterDepth == 0 && t.Resource != "fish" && t.FishCapacity == 0, "Dry land keeps water or fish at " + p);
                        var old = v1.GenerateSurfaceTile(p);
                        if (old.Water && old.WaterDepth < 0.15)
                        {
                            converted++;
                            Require(t.Landform.EndsWith("_bank", StringComparison.Ordinal) && t.Biome == "wetland", "Thin water at " + p + " did not become a bank: " + t.Landform);
                        }
                    }
            }
            Require(water > 1000 && converted > 50, "Sample lacks real water or converted banks: water=" + water + " converted=" + converted);
        }

        /// <summary>Version 2 adds rock landmarks: clustered outcrops everywhere and ridges in the highland band, without blocking ordinary walking.</summary>
        private static void OutcropsAndRidges()
        {
            int ridges = 0;
            foreach (uint seed in new uint[] { 7, 41, 4242 })
            {
                var w = New2(seed);
                int outcrops = 0, scree = 0, stone = 0, mountains = 0, tiles = 0;
                var row = new Tile[193];
                Tile[] previous = null;
                for (int z = -96; z <= 96; z++)
                {
                    for (int x = -96; x <= 96; x++)
                    {
                        var t = w.GenerateSurfaceTile(new Int2(x, z));
                        row[x + 96] = t;
                        tiles++;
                        if (t.Mountain)
                        {
                            mountains++;
                            Require(!t.Water && t.Resource == "", "Mountain rock holds water or a deposit at " + t.Position);
                            if (t.Landform == "outcrop")
                                outcrops++;
                            else if (t.Landform == "ridge")
                                ridges++;
                        }
                        else if (t.Landform == "outcrop")
                        {
                            scree++;
                            if (t.Resource == "stone")
                                stone++;
                        }
                        Require(t.Resource != "logs" || t.Biome.Contains("forest"), "Trees ignored climate biome at " + t.Position);
                        Require(t.Resource != "ore" && t.Resource != "gem" || t.Biome == "highland", "Deep minerals ignored geology at " + t.Position);
                    }
                    for (int x = 0; x < row.Length; x++)
                    {
                        var t = row[x];
                        bool Walk(Tile o) => !o.Water && !o.Mountain;
                        if (x > 0 && Walk(t) && Walk(row[x - 1]))
                            Require(Math.Abs(t.Elevation - row[x - 1].Elevation) <= 0.65, "Unwalkable dry seam at " + t.Position);
                        if (previous != null && Walk(t) && Walk(previous[x]))
                            Require(Math.Abs(t.Elevation - previous[x].Elevation) <= 0.65, "Unwalkable dry seam at " + t.Position);
                    }
                    previous = (Tile[])row.Clone();
                }
                Require(outcrops > 0 && scree > 0 && stone > 0, "Seed " + seed + " lacks clustered rock outcrops with stone scree");
                Require(mountains > tiles * 0.004 && mountains < tiles * 0.08, "Seed " + seed + " mountain share is " + mountains + " of " + tiles);
            }
            Require(ridges > 0, "No highland ridge in three sampled worlds");
        }

        private static void CoordinatePureVersion2()
        {
            var first = New2();
            var second = New2();
            var points = new[] { new Int2(-1, -1), new Int2(-5, 0), new Int2(0, -5), new Int2(5, 0), new Int2(0, 5), new Int2(-113, 127), new Int2(113, -127), new Int2(int.MinValue + 2, int.MaxValue - 2), new Int2(int.MaxValue - 2, int.MinValue + 2) };
            var expected = points.ToDictionary(p => p, p => Fingerprint(first.GenerateSurfaceTile(p)));
            foreach (var point in points.Reverse())
                Require(expected[point] == Fingerprint(second.GenerateSurfaceTile(point)), "Exploration order changed version 2 terrain at " + point);
            Require(first.RandomState == 1234 && second.RandomState == 1234 && first.Tiles.Count == 0 && second.Tiles.Count == 0, "Version 2 sampling consumed actor entropy or inserted hidden tiles");
            Require(points.Any(p => expected[p] != Fingerprint(New2(42).GenerateSurfaceTile(p))), "Different seeds generated identical version 2 terrain");
            Require(points.Any(p => expected[p] != Fingerprint(New(41).GenerateSurfaceTile(p))), "Version 2 is identical to version 1");
            var shifted = New2(41, 1.25);
            foreach (var point in points.Take(7))
            {
                var a = first.GenerateSurfaceTile(point);
                var b = shifted.GenerateSurfaceTile(point);
                Require(Math.Abs(a.Elevation - 1.25 - b.Elevation) < 1e-9 && Math.Abs(a.WaterSurface - 1.25 - b.WaterSurface) < 1e-9 && a.WaterDepth == b.WaterDepth && a.Biome == b.Biome && a.Resource == b.Resource && a.Mountain == b.Mountain, "Surface datum did not shift version 2 heights uniformly at " + point);
            }
        }

        /// <summary>Version 2 grades a small core to the datum and blends a short shoulder, without draining water.</summary>
        private static void FoundingGradeVersion2()
        {
            foreach (uint seed in new uint[] { 7, 41, 4242 })
            {
                var a = New2(seed);
                var b = New2(seed);
                var natural = New2(seed);
                var village = new Village { Id = "test", Center = new Int2(-170, 151), Radius = 9 };
                var points = Enumerable.Range(-32, 65).SelectMany(z => Enumerable.Range(-32, 65).Select(x => new Int2(village.Center.X + x, village.Center.Z + z))).ToArray();
                foreach (var point in points.Where((p, index) => index % 13 == 0))
                    a.TileAt(point);
                a.PrepareFoundingTerrain(village);
                b.PrepareFoundingTerrain(village);
                var profile = b.SurfacePlateaus.Single();
                Require(profile.CoreRadius == village.Radius + 2 && profile.Shoulder >= 8 && profile.Shoulder <= 20, "Version 2 plateau is not radius + 2 with an 8 to 20 tile shoulder");
                foreach (var point in points)
                {
                    var left = a.GetTile(point) ?? a.GenerateSurfaceTile(point);
                    var right = b.GenerateSurfaceTile(point);
                    Require(left.Elevation == right.Elevation && left.WaterDepth == right.WaterDepth && left.WaterSurface == right.WaterSurface && left.Water == right.Water && left.Mountain == right.Mountain, "Exploration before founding changed version 2 grading at " + point);
                    int ring = Chebyshev(point, village.Center);
                    if (ring <= profile.CoreRadius)
                        Require(right.Water ? right.WaterSurface == 0 : right.Elevation == 0, "Version 2 core is not graded to the datum at " + point);
                    if (ring < profile.CoreRadius + profile.Shoulder / 2)
                        Require(!right.Mountain, "Version 2 grading left rock beside the walls at " + point);
                    if (ring < profile.CoreRadius + profile.Shoulder)
                        Require(!right.Mountain || right.Landform != "outcrop" && right.Landform != "ridge", "Version 2 relief rock stands inside the graded shoulder at " + point);
                    var raw = natural.GenerateSurfaceTile(point);
                    Require(right.Water == raw.Water && right.WaterDepth == raw.WaterDepth, "Version 2 grading drained or deepened water at " + point);
                    if (ring > profile.CoreRadius + profile.Shoulder)
                        Require(Fingerprint(right) == Fingerprint(raw), "Version 2 grading reached beyond its shoulder at " + point);
                    var next = b.GenerateSurfaceTile(new Int2(point.X + 1, point.Z));
                    if (!right.Water && !next.Water && !right.Mountain && !next.Mountain)
                        Require(Math.Abs(right.Elevation - next.Elevation) <= 0.65, "Version 2 shoulder has a blocked dry seam at " + point);
                }
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
