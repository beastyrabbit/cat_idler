using System;
using System.Collections.Generic;

namespace IdleCatForest.Simulation
{
    [Serializable]
    public sealed class SurfacePlateau
    {
        public string VillageId = "";
        public Int2 Center;
        public int CoreRadius, Shoulder = 20;
    }
    /// <summary>Saved terrain geometry. Heights are world units; water depth is measured from its bed.</summary>
    public partial class Tile
    {
        public double Elevation, WaterSurface, WaterDepth;
        public string Landform = "";
    }

    public partial class World
    {
        // Missing on old saves deliberately means the original flat generator. Never infer
        // a version from explored tiles: exploring after a restart must use the same rules.
        public int GenerationVersion;
        public List<SurfacePlateau> SurfacePlateaus = new List<SurfacePlateau>();
        public bool CanPrepareFoundingTerrain(string villageId, Int2 center, int radius)
        {
            if (GenerationVersion == 0)
                return true;
            long extent = (long)radius + 26;
            if (radius < 0 || extent * 2 + 1 > int.MaxValue || (long)center.X - extent < int.MinValue || (long)center.Z - extent < int.MinValue || (long)center.X + extent > int.MaxValue || (long)center.Z + extent > int.MaxValue)
                return false;
            foreach (var site in Dungeons)
                if (Math.Abs((long)site.Entrance.X - center.X) <= extent + 4 && Math.Abs((long)site.Entrance.Z - center.Z) <= extent)
                {
                    bool existingProfile = false;
                    foreach (var profile in SurfacePlateaus)
                        existingProfile |= profile.VillageId == villageId && profile.Center.Equals(center) && profile.CoreRadius == radius + 6;
                    if (!existingProfile)
                        return false;
                }
            return CanModifyTerrain(villageId, new Int2(center.X - (int)extent, center.Z - (int)extent), (int)(extent * 2 + 1), (int)(extent * 2 + 1));
        }
        /// <summary>Registers a saved grading profile without exploring its whole shoulder.</summary>
        public void PrepareFoundingTerrain(Village village)
        {
            if (GenerationVersion == 0)
                return;
            foreach (var existing in SurfacePlateaus)
                if (existing.VillageId == village.Id && existing.Center.Equals(village.Center) && existing.CoreRadius == village.Radius + 6)
                    return;
            if (!CanPrepareFoundingTerrain(village.Id, village.Center, village.Radius))
                throw new InvalidOperationException("Founding terrain overlaps foreign territory or an existing dungeon");
            var profile = new SurfacePlateau { VillageId = village.Id, Center = village.Center, CoreRadius = village.Radius + 6 };
            SurfacePlateaus.Add(profile);
            // Re-evaluate only existing natural geometry. Finite deposits, ownership and
            // constructed topology survive; only fish lose their habitat when water drains.
            foreach (var tile in Tiles)
            {
                if (PlateauWeight(profile, tile.Position) >= 1)
                    continue;
                var shaped = GenerateSurfaceTile(tile.Position);
                tile.Elevation = shaped.Elevation;
                tile.WaterSurface = shaped.WaterSurface;
                tile.WaterDepth = shaped.WaterDepth;
                tile.Water = shaped.Water;
                tile.Mountain = shaped.Mountain;
                tile.Biome = shaped.Biome;
                tile.Landform = shaped.Landform;
                if (!tile.Water && tile.Resource == "fish")
                {
                    tile.Resource = "";
                    tile.Amount = tile.FishCapacity = 0;
                }
            }
        }

        /// <summary>Coordinate-pure terrain sampling; inserting or claiming the tile belongs to TileAt.</summary>
        public Tile GenerateSurfaceTile(Int2 p)
        {
            if (GenerationVersion == 0)
                return GenerateLegacySurface(p);
            if (GenerationVersion != 1)
                throw new InvalidOperationException("Unsupported world generation version " + GenerationVersion);

            double height = NaturalHeight(p.X, p.Z);
            double moisture = SurfaceNoise(p.X, p.Z, 72, 101) * 0.7 + SurfaceNoise(p.X, p.Z, 32, 103) * 0.3;
            double temperature = SurfaceNoise(p.X, p.Z, 140, 211) * 0.72 + SurfaceNoise(p.X, p.Z, 48, 223) * 0.28;
            var tile = new Tile { Position = p, Elevation = QuantizeHeight(height), Landform = "upland" };
            tile.Biome = height > 1.5 ? "highland" : moisture > 0.64 && height < 0.3 ? "wetland" : temperature < 0.43 && moisture > 0.4 ? "pine_forest" : moisture > 0.43 ? "forest" : "meadow";
            tile.Mountain = height > 3 && SurfaceNoise(p.X, p.Z, 32, 227) > 0.62;
            if (tile.Biome == "meadow")
                tile.Landform = "grassland";
            else if (tile.Biome == "wetland")
                tile.Landform = "marsh";

            ShapeRiver(tile);
            if (!tile.Water)
                ShapeLake(tile);
            if (!tile.Water)
                tile.WaterSurface = tile.Elevation;
            PopulateSurface(tile);
            ApplySurfacePlateaus(tile);
            return tile;
        }

        private static double PlateauWeight(SurfacePlateau profile, Int2 position)
        {
            if (position.Level != profile.Center.Level)
                return 1;
            long distance = Math.Max(Math.Abs((long)position.X - profile.Center.X), Math.Abs((long)position.Z - profile.Center.Z));
            return Smooth(Clamp01((distance - profile.CoreRadius) / (double)Math.Max(1, profile.Shoulder)));
        }
        private void ApplySurfacePlateaus(Tile tile)
        {
            double weight = 1;
            foreach (var profile in SurfacePlateaus)
                weight = Math.Min(weight, PlateauWeight(profile, tile.Position));
            if (weight >= 1)
                return;
            tile.Elevation = QuantizeHeight(tile.Elevation * weight);
            tile.WaterSurface = QuantizeHeight(tile.WaterSurface * weight);
            tile.WaterDepth = QuantizeHeight(tile.WaterDepth * weight);
            if (tile.Water)
            {
                tile.Elevation = tile.WaterSurface - tile.WaterDepth;
                if (tile.WaterDepth == 0)
                {
                    tile.Water = false;
                    tile.Biome = "meadow";
                    tile.Resource = "";
                    tile.Amount = tile.FishCapacity = 0;
                }
            }
            tile.Mountain &= tile.Elevation > 3;
            if (weight == 0)
                tile.Landform = "terrace";
        }

        // The old algorithm intentionally retains truncating division at negative coordinates,
        // its exact biome catalog and resource selection. Played worlds keep their frontier.
        private Tile GenerateLegacySurface(Int2 p)
        {
            uint h = Hash(Seed + ":" + (p.X / 5) + ":" + (p.Z / 5));
            var tile = new Tile { Position = p };
            int biome = (int)(h % 26);
            string[] biomes = { "forest", "meadow", "rainforest", "birch_forest", "taiga", "snow_forest", "flower_forest", "mushroom_forest", "desert", "badlands", "beach", "marsh", "swamp", "lake", "ocean", "river", "hills", "mountains", "snow_mountains", "grassland", "savanna", "jungle", "tundra", "highland", "cave", "woodland" };
            tile.Biome = biomes[biome];
            tile.Water = biome >= 13 && biome <= 15;
            tile.Mountain = biome == 18 || biome == 24;
            uint spot = Hash(Seed + ":" + p.X + ":" + p.Z);
            if (tile.Water)
            {
                tile.Resource = "fish";
                tile.Amount = tile.FishCapacity = 24;
            }
            else if (!tile.Mountain && spot % 4 == 0)
            {
                tile.Resource = biome == 17 ? "gem" : biome == 11 || biome == 12 || biome == 9 ? "clay" : biome == 8 || biome == 10 ? "sand" : biome == 16 ? "ore" : spot % 7 == 0 ? "stone" : "logs";
                tile.Amount = 20 + spot % 40;
            }
            else if (!tile.Mountain && spot % 11 == 0)
            {
                tile.Resource = "food";
                tile.Amount = 20;
            }
            return tile;
        }

        private uint SurfaceHash(int x, int z, uint salt)
        {
            uint value = Seed ^ salt;
            value ^= unchecked((uint)x * 0x9e3779b1u);
            value = (value << 13) | (value >> 19);
            value ^= unchecked((uint)z * 0x85ebca77u);
            value ^= value >> 16;
            value = unchecked(value * 0x7feb352du);
            value ^= value >> 15;
            value = unchecked(value * 0x846ca68bu);
            return value ^ (value >> 16);
        }
        private static double Blend(double a, double b, double weight) => a + (b - a) * weight;
        private static double Smooth(double x) => x * x * (3 - 2 * x);
        private static double Clamp01(double x) => Math.Max(0, Math.Min(1, x));
        private static double QuantizeHeight(double height) => Math.Round(height, 3, MidpointRounding.AwayFromZero);
        private double SurfaceNoise(double x, double z, double scale, uint salt)
        {
            double sx = x / scale, sz = z / scale;
            int ix = (int)Math.Floor(sx), iz = (int)Math.Floor(sz);
            double dx = Smooth(sx - ix), dz = Smooth(sz - iz);
            double a = Blend(SurfaceHash(ix, iz, salt) / 4294967295.0, SurfaceHash(ix + 1, iz, salt) / 4294967295.0, dx);
            double b = Blend(SurfaceHash(ix, iz + 1, salt) / 4294967295.0, SurfaceHash(ix + 1, iz + 1, salt) / 4294967295.0, dx);
            return Blend(a, b, dz);
        }
        private double NaturalHeight(double x, double z) => (SurfaceNoise(x, z, 96, 13) - 0.5) * 5.5 + (SurfaceNoise(x, z, 40, 29) - 0.5) * 2.2 + (SurfaceNoise(x, z, 16, 47) - 0.5) * 0.55;

        private double RiverCenter(int basin, double z) => basin * 128.0 + (SurfaceHash(basin, 0, 307) % 33 - 16.0) + (SurfaceNoise(basin * 61.0, z, 64, 311) - 0.5) * 24 + (SurfaceNoise(basin * 61.0, z, 24, 313) - 0.5) * 6;
        private void NearestRiver(double x, double z, out int basin, out double center, out double distance)
        {
            int first = (int)Math.Floor(x / 128);
            basin = first;
            center = RiverCenter(first, z);
            distance = Math.Abs(x - center);
            for (int candidate = first - 1; candidate <= first + 1; candidate++)
            {
                double possible = RiverCenter(candidate, z), delta = Math.Abs(x - possible);
                if (delta < distance)
                {
                    basin = candidate;
                    center = possible;
                    distance = delta;
                }
            }
        }
        private void ShapeRiver(Tile tile)
        {
            NearestRiver(tile.Position.X, tile.Position.Z, out int basin, out double center, out double distance);
            double width = 3.4 + SurfaceNoise(basin * 61.0, tile.Position.Z, 64, 317) * 0.8;
            if (distance >= width + 5)
                return;
            double surface = QuantizeHeight(NaturalHeight(center, tile.Position.Z) - 0.1);
            tile.Mountain = false;
            if (distance < width)
            {
                double fromCenter = 1 - distance / width;
                double fordSpacing = 43;
                double phase = tile.Position.Z + SurfaceHash(basin, 0, 331) % 43;
                double fordDistance = Math.Abs(phase - Math.Round(phase / fordSpacing) * fordSpacing);
                double channel = 0.1 + fromCenter * 1.55;
                double ford = 0.12 + fromCenter * 0.16;
                double depth = QuantizeHeight(Blend(ford, channel, Smooth(Clamp01((fordDistance - 2) / 5))));
                SetWaterSurface(tile, surface, depth, fordDistance <= 2 ? "river_ford" : depth <= 0.55 ? "river_shallow" : "river_deep");
            }
            else
            {
                double bank = distance - width;
                tile.Elevation = QuantizeHeight(Blend(surface + 0.04 + bank * 0.1, tile.Elevation, Smooth(bank / 5)));
                tile.Landform = "river_bank";
                tile.Biome = "wetland";
            }
        }
        private void ShapeLake(Tile tile)
        {
            // Each basin stays inside its 112-tile cell, including its graded bank. No
            // explored-neighbor dependency or square-cell biome selection is involved.
            int cx = (int)Math.Floor(tile.Position.X / 112.0), cz = (int)Math.Floor(tile.Position.Z / 112.0);
            uint lake = SurfaceHash(cx, cz, 401);
            double x = cx * 112.0 + 56 + lake % 33 - 16;
            double z = cz * 112.0 + 56 + (lake >> 8) % 33 - 16;
            double radius = 12 + (lake >> 16) % 8;
            double dx = tile.Position.X - x, dz = tile.Position.Z - z;
            double distance = Math.Sqrt(dx * dx + dz * dz);
            if (distance >= radius + 10)
                return;
            NearestRiver(x, z, out _, out _, out double riverDistance);
            // Separate lake banks from river banks, preventing mismatched water surfaces.
            if (riverDistance < radius + 20)
                return;
            double surface = QuantizeHeight(NaturalHeight(x, z) - 0.1);
            tile.Mountain = false;
            if (distance < radius)
            {
                double inward = radius - distance;
                double depth = inward < 2 ? 0.1 + inward * 0.2 : inward < 5 ? 0.5 + (inward - 2) * 0.25 : inward < 8 ? 1.25 + (inward - 5) * 0.45 : Math.Min(4.5, 2.6 + (inward - 8) * 0.12);
                depth = QuantizeHeight(depth);
                SetWaterSurface(tile, surface, depth, depth <= 0.55 ? "lake_shallow" : depth < 2.6 ? "lake_shelf" : "lake_deep");
            }
            else
            {
                double bank = distance - radius;
                tile.Elevation = QuantizeHeight(Blend(surface + 0.04 + bank * 0.1, tile.Elevation, Smooth(bank / 10)));
                tile.Biome = "wetland";
                tile.Landform = "lake_bank";
            }
        }

        private void PopulateSurface(Tile tile)
        {
            uint spot = SurfaceHash(tile.Position.X, tile.Position.Z, 503), kind = spot % 100;
            if (tile.Water)
            {
                tile.Resource = "fish";
                tile.Amount = tile.FishCapacity = 24 + Math.Floor(tile.WaterDepth * 4);
                return;
            }
            if (tile.Mountain)
                return;
            if (tile.Biome == "highland")
                tile.Resource = kind < 3 ? "gem" : kind < 23 ? "ore" : kind < 58 ? "stone" : "";
            else if (tile.Biome == "wetland")
                tile.Resource = kind < 24 ? "clay" : kind < 35 ? "sand" : kind < 49 ? "fibre" : kind < 59 ? "herbs" : kind < 65 ? "food" : "";
            else if (tile.Biome.Contains("forest"))
                tile.Resource = kind < 42 ? "logs" : kind < 50 ? "food" : kind < 57 ? "herbs" : kind < 64 ? "fibre" : kind < 69 ? "stone" : "";
            else
                tile.Resource = kind < 14 ? "food" : kind < 26 ? "fibre" : kind < 32 ? "herbs" : kind < 36 ? "stone" : "";
            if (tile.Resource != "")
                tile.Amount = 20 + (spot >> 8) % 40;
        }

        /// <summary>Reads persisted terrain only. Presentation must not generate unexplored terrain.</summary>
        public double SurfaceHeight(Int2 p) => GetTile(p)?.Elevation ?? 0;
        public double WalkHeight(Int2 p)
        {
            if (p.Level < 0)
                return DungeonHeight(p);
            var tile = GetTile(p);
            return tile == null ? 0 : tile.Bridge && tile.Water ? tile.WaterSurface + 0.08 : tile.Elevation;
        }
        public static void ClearSurface(Tile tile, double height)
        {
            if (tile == null)
                throw new ArgumentNullException(nameof(tile));
            tile.Elevation = tile.WaterSurface = height;
            tile.WaterDepth = 0;
            tile.Water = tile.Mountain = false;
            tile.Biome = "meadow";
            tile.Landform = "settlement";
            tile.Resource = "";
            tile.Amount = tile.FishCapacity = 0;
            tile.Deposits.Clear();
        }
        public static void SetWaterSurface(Tile tile, double surface, double depth, string landform = "lake_shallow")
        {
            if (tile == null)
                throw new ArgumentNullException(nameof(tile));
            if (double.IsNaN(surface) || double.IsInfinity(surface) || double.IsNaN(depth) || double.IsInfinity(depth) || depth <= 0)
                throw new ArgumentOutOfRangeException(nameof(depth));
            tile.WaterSurface = surface;
            tile.WaterDepth = depth;
            tile.Elevation = surface - depth;
            tile.Water = true;
            tile.Mountain = false;
            tile.Landform = landform;
            tile.Biome = landform.StartsWith("river", StringComparison.Ordinal) ? "river" : "lake";
            tile.Resource = "fish";
            tile.Amount = tile.FishCapacity = 24;
            tile.Deposits.Clear();
        }
    }
}
