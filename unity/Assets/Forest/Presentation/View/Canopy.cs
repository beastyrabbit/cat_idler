using System;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Decorative forest canopy and ground scatter. Presentation-only and deterministic: every choice comes from an
    /// integer hash of the tile seeded by <see cref="World.Hash"/> of the world seed, so a world always shows the same
    /// forest, independent of the finite harvestable logs (which keep their own tree) and of frame timing.
    /// </summary>
    internal static class Canopy
    {
        /// <summary>One decorative instance on a tile: an authored kit asset at a tile-relative offset.</summary>
        internal struct Placement
        {
            public string Asset;
            public float X, Z, Yaw, Width, HeightFactor;
            public int Tint;
            public bool Shadows;
        }

        internal enum Cover { None, Broadleaf, Birch, Pine, Wetland, Highland, Meadow }

        public const float TreeWidth = 1.7f;
        public const int MaxTrees = 3, MaxScatter = 6;

        public static uint Seed(World world) => World.Hash("canopy:" + world.Seed + ":" + world.GenerationVersion);

        // ------------------------------------------------------------------ hashing and noise

        public static uint Mix(uint seed, int x, int z, uint salt)
        {
            unchecked
            {
                uint h = seed ^ (salt * 0x9E3779B9u);
                h ^= (uint)x * 0x85EBCA6Bu; h = (h << 13) | (h >> 19); h *= 5; h += 0xE6546B64u;
                h ^= (uint)z * 0xC2B2AE35u; h = (h << 13) | (h >> 19); h *= 5; h += 0xE6546B64u;
                h ^= h >> 16; h *= 0x85EBCA6Bu; h ^= h >> 13; h *= 0xC2B2AE35u; h ^= h >> 16;
                return h;
            }
        }

        public static float Unit(uint seed, int x, int z, uint salt) => (Mix(seed, x, z, salt) & 0xFFFFFF) / 16777216f;

        private static int FloorDiv(int v, int d) => v >= 0 ? v / d : -((-v + d - 1) / d);

        private static float ValueNoise(uint seed, int x, int z, int cell, uint salt)
        {
            int cx = FloorDiv(x, cell), cz = FloorDiv(z, cell);
            float fx = (x - cx * cell) / (float)cell, fz = (z - cz * cell) / (float)cell;
            fx = fx * fx * (3 - 2 * fx); fz = fz * fz * (3 - 2 * fz);
            float a = Unit(seed, cx, cz, salt), b = Unit(seed, cx + 1, cz, salt), c = Unit(seed, cx, cz + 1, salt), d = Unit(seed, cx + 1, cz + 1, salt);
            return Mathf.Lerp(Mathf.Lerp(a, b, fx), Mathf.Lerp(c, d, fx), fz);
        }

        /// <summary>Low-frequency forest density in [0, 1]: large stands broken by clearings.</summary>
        public static float Density(uint seed, int x, int z) => ValueNoise(seed, x, z, 14, 11) * .65f + ValueNoise(seed, x, z, 6, 13) * .35f;

        /// <summary>The share of tiles in a stand that carry trees: 70 to 90 % in forest cores, falling to open clearings.</summary>
        public static float ForestCover(float density)
        {
            if (density < .32f) return .06f;
            if (density < .44f) return Mathf.Lerp(.06f, .7f, (density - .32f) / .12f);
            return Mathf.Lerp(.7f, .9f, Mathf.Clamp01((density - .44f) / .4f));
        }

        public static Cover CoverOf(Tile tile)
        {
            if (tile.Water || tile.Mountain || tile.Wall || tile.Road || tile.Rail || tile.Bridge || tile.Dock || tile.Overlay == "road_built") return Cover.None;
            switch (tile.Biome)
            {
                case "pine_forest": case "taiga": case "snow_forest": return Cover.Pine;
                case "birch_forest": return Cover.Birch;
                case "forest": case "woodland": case "rainforest": case "flower_forest": case "mushroom_forest": case "jungle": return Cover.Broadleaf;
                case "wetland": case "marsh": case "swamp": return Cover.Wetland;
                case "highland": case "hills": return Cover.Highland;
                case "meadow": case "grassland": case "savanna": return Cover.Meadow;
                default: return Cover.None;
            }
        }

        private static float CoverChance(Cover cover, float density)
        {
            switch (cover)
            {
                case Cover.Broadleaf: case Cover.Birch: case Cover.Pine: return ForestCover(density);
                case Cover.Wetland: return .12f + .15f * Mathf.Clamp01((density - .5f) * 3);
                case Cover.Highland: return .14f + .2f * Mathf.Clamp01((density - .5f) * 3);
                case Cover.Meadow: return .03f + .22f * Mathf.Clamp01((density - .62f) * 4);
                default: return 0;
            }
        }

        private static string TreeAsset(Cover cover, float pick)
        {
            switch (cover)
            {
                case Cover.Pine: return pick < .62f ? "world_tree_pine" : pick < .82f ? "tree_pine" : "world_tree_birch";
                case Cover.Birch: return pick < .7f ? "world_tree_birch" : "tree_oak";
                case Cover.Wetland: return pick < .6f ? "world_tree_birch" : "tree_oak";
                case Cover.Highland: return pick < .7f ? "world_tree_pine" : "tree_pine";
                case Cover.Meadow: return pick < .55f ? "tree_oak" : "world_tree_birch";
                default: return pick < .45f ? "tree_oak" : pick < .75f ? "world_tree_birch" : "world_tree_pine";
            }
        }

        // Slot layouts for one, two and three trees on a tile, rotated per tile.
        private static readonly Vector2[][] Slots =
        {
            new[] { Vector2.zero },
            new[] { new Vector2(-.24f, -.16f), new Vector2(.24f, .18f) },
            new[] { new Vector2(-.26f, -.2f), new Vector2(.27f, -.12f), new Vector2(0, .28f) },
        };

        /// <summary>
        /// Decorative trees for a free tile (1 to 3 mixed trees on a covered tile): per-instance yaw, scale 0.8 to 1.25,
        /// ±0.25 offset and a ±5 % tint step. Returns how many were written.
        /// </summary>
        public static int Trees(uint seed, Tile tile, Placement[] into, int maxTrees = MaxTrees)
        {
            var cover = CoverOf(tile);
            if (cover == Cover.None) return 0;
            int x = tile.Position.X, z = tile.Position.Z;
            float density = Density(seed, x, z);
            if (Unit(seed, x, z, 1) >= CoverChance(cover, density)) return 0;
            float roll = Unit(seed, x, z, 2);
            bool forest = cover == Cover.Broadleaf || cover == Cover.Birch || cover == Cover.Pine;
            int count = !forest ? 1 : roll < .4f ? 1 : roll < .8f ? 2 : 3;
            if (forest && density > .7f && count < 3 && Unit(seed, x, z, 3) < .35f) count++;
            count = Math.Min(count, Math.Max(1, maxTrees));
            var slots = Slots[count - 1];
            float turn = Unit(seed, x, z, 4) * Mathf.PI * 2, cos = Mathf.Cos(turn), sin = Mathf.Sin(turn);
            for (int i = 0; i < count; i++)
            {
                uint salt = (uint)(10 + i * 8);
                var s = slots[i];
                float jx = (Unit(seed, x, z, salt) - .5f) * (count == 1 ? .5f : .16f);
                float jz = (Unit(seed, x, z, salt + 1) - .5f) * (count == 1 ? .5f : .16f);
                float scale = Mathf.Lerp(.8f, 1.25f, Unit(seed, x, z, salt + 2)) * (count == 1 ? 1 : count == 2 ? .88f : .8f);
                into[i] = new Placement
                {
                    Asset = TreeAsset(cover, Unit(seed, x, z, salt + 3)),
                    X = s.x * cos - s.y * sin + jx,
                    Z = s.x * sin + s.y * cos + jz,
                    Yaw = Unit(seed, x, z, salt + 4) * 360,
                    Width = TreeWidth * scale,
                    HeightFactor = 1,
                    Tint = (int)(Unit(seed, x, z, salt + 5) * PropLook.Tints) % PropLook.Tints,
                    Shadows = true,
                };
            }
            return count;
        }

        /// <summary>
        /// Ground scatter for a free meadow or forest tile: 2 to 6 small kit pieces (shrubs, berry clumps standing in for
        /// flowers, pebble rocks, mushrooms under trees), thinned by path wear. Returns how many were written.
        /// </summary>
        public static int Scatter(uint seed, Tile tile, bool underTrees, Placement[] into)
        {
            var cover = CoverOf(tile);
            if (cover == Cover.None || tile.Dirt) return 0;
            int x = tile.Position.X, z = tile.Position.Z;
            int count = 2 + (int)(Unit(seed, x, z, 40) * 5);
            float wear = Mathf.Clamp01((float)tile.PathWear / 50f);
            count = Mathf.RoundToInt(count * (1 - wear));
            if (underTrees) count = Mathf.Min(count, 3);
            for (int i = 0; i < count; i++)
            {
                uint salt = (uint)(41 + i * 8);
                float pick = Unit(seed, x, z, salt);
                string asset; float width, height = 1;
                bool woods = cover != Cover.Meadow;
                if (woods && pick < .3f) { asset = "world_mushrooms"; width = Mathf.Lerp(.16f, .26f, Unit(seed, x, z, salt + 1)); }
                else if (pick < (woods ? .65f : .28f)) { asset = "shrub"; width = Mathf.Lerp(.22f, .4f, Unit(seed, x, z, salt + 1)); height = .55f; }
                else if (!woods && pick < .66f) { asset = "berry_bush"; width = Mathf.Lerp(.16f, .26f, Unit(seed, x, z, salt + 1)); height = .6f; }
                else { asset = "rock"; width = Mathf.Lerp(.1f, .22f, Unit(seed, x, z, salt + 1)); height = .7f; }
                into[i] = new Placement
                {
                    Asset = asset,
                    X = (Unit(seed, x, z, salt + 2) - .5f) * .9f,
                    Z = (Unit(seed, x, z, salt + 3) - .5f) * .9f,
                    Yaw = Unit(seed, x, z, salt + 4) * 360,
                    Width = width,
                    HeightFactor = height,
                    Tint = (int)(Unit(seed, x, z, salt + 5) * PropLook.Tints) % PropLook.Tints,
                    Shadows = false,
                };
            }
            return count;
        }

        /// <summary>Per-tile jitter for authored resource props (harvestable trees, rocks, bushes, reeds).</summary>
        public static void Jitter(uint seed, Int2 p, float offset, float minScale, float maxScale, out float dx, out float dz, out float yaw, out float scale, out int tint)
        {
            dx = (Unit(seed, p.X, p.Z, 90) - .5f) * 2 * offset;
            dz = (Unit(seed, p.X, p.Z, 91) - .5f) * 2 * offset;
            yaw = Unit(seed, p.X, p.Z, 92) * 360;
            scale = Mathf.Lerp(minScale, maxScale, Unit(seed, p.X, p.Z, 93));
            tint = (int)(Unit(seed, p.X, p.Z, 94) * PropLook.Tints) % PropLook.Tints;
        }

        /// <summary>The harvestable tree model for a logs tile, by biome.</summary>
        public static string HarvestTree(uint seed, Tile tile)
        {
            var cover = CoverOf(tile);
            float pick = Unit(seed, tile.Position.X, tile.Position.Z, 95);
            if (cover == Cover.Pine || cover == Cover.Highland) return pick < .8f ? "world_tree_pine" : "tree_pine";
            return pick < .34f ? "world_tree_birch" : "tree_oak";
        }
    }

    /// <summary>Canopy placement on known tiles and the forest beyond the explored edge.</summary>
    public sealed partial class ForestView
    {
        private readonly Canopy.Placement[] placements = new Canopy.Placement[Math.Max(Canopy.MaxTrees, Canopy.MaxScatter)];

        /// <summary>
        /// An unexplored tile, sampled from the coordinate-pure generator (never <c>TileAt</c>): its trees and rocks,
        /// tinted toward the fog look by distance to the explored edge. Instances over the explored boundary are drawn
        /// once, by the chunk that owns the tile, so nothing doubles at the seam.
        /// </summary>
        private void AddUnknownTile(StaticProps.Chunk chunk, World world, int x, int z, int fog)
        {
            var tile = world.GenerateSurfaceTile(new Int2(x, z));
            float ground = (float)tile.Elevation;
            if (tile.Water) return;
            // Only the first fade step casts shadows; deeper fog is drawn as light as possible: no shadows, at most
            // two trees a tile and the lighter pine model.
            bool shadows = fog <= 1;
            bool deep = fog >= PropLook.FogSteps - 2;
            if (tile.Resource == "logs" && tile.Amount > 0)
            {
                Canopy.Jitter(canopySeed, tile.Position, .25f, .8f, 1.25f, out float dx, out float dz, out float yaw, out float scale, out int tint);
                AddProp(chunk, Canopy.HarvestTree(canopySeed, tile), new Vector3(x + dx, ground, z + dz), yaw, Canopy.TreeWidth * scale, 1, new PropLook(shadows, tint, fog));
                return;
            }
            if (tile.Mountain || tile.Resource == "stone" && tile.Amount > 0)
            {
                Canopy.Jitter(canopySeed, tile.Position, .2f, .7f, 1.2f, out float dx, out float dz, out float yaw, out float scale, out int tint);
                AddProp(chunk, "rock", new Vector3(x + dx, ground, z + dz), yaw, .85f * scale, 1, new PropLook(shadows, tint, fog));
                return;
            }
            int trees = Canopy.Trees(canopySeed, tile, placements, deep ? 2 : Canopy.MaxTrees);
            for (int i = 0; i < trees; i++)
            {
                var p = placements[i];
                if (deep && p.Asset == "tree_pine") p.Asset = "world_tree_pine";
                AddProp(chunk, p.Asset, new Vector3(x + p.X, ground, z + p.Z), p.Yaw, p.Width, p.HeightFactor, new PropLook(shadows && p.Shadows, p.Tint, fog));
            }
        }

        /// <summary>Decorative trees and scatter on a free explored tile.</summary>
        private void AddCanopy(StaticProps.Chunk chunk, Tile tile, float ground, bool trees)
        {
            int count = trees ? Canopy.Trees(canopySeed, tile, placements) : 0;
            for (int i = 0; i < count; i++)
            {
                var p = placements[i];
                AddProp(chunk, p.Asset, new Vector3(tile.Position.X + p.X, ground, tile.Position.Z + p.Z), p.Yaw, p.Width, p.HeightFactor, new PropLook(p.Shadows, p.Tint, 0));
            }
            int scatter = Canopy.Scatter(canopySeed, tile, count > 0, placements);
            for (int i = 0; i < scatter; i++)
            {
                var p = placements[i];
                AddProp(chunk, p.Asset, new Vector3(tile.Position.X + p.X, ground, tile.Position.Z + p.Z), p.Yaw, p.Width, p.HeightFactor, new PropLook(p.Shadows, p.Tint, 0));
            }
        }

        private void AddProp(StaticProps.Chunk chunk, string asset, Vector3 at, float yaw, float width, float heightFactor, PropLook look, Int2? primary = null)
        {
            var root = Matrix4x4.TRS(at, Quaternion.Euler(0, yaw, 0), new Vector3(width, width * heightFactor, width));
            staticProps.Add(chunk, asset, root, false, look, primary);
        }
    }
}
