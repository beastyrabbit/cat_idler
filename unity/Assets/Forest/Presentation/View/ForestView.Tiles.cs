using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.CompilerServices;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Per-tile scenery. Farm plots grow and stay entities; everything else on an explored tile (resources, stumps,
    /// roads, fences, gates, reeds, boundary fences, canopy and scatter) is a static instanced prop of its chunk.
    /// </summary>
    public sealed partial class ForestView
    {
        private static readonly Int2[] FenceSteps = { new Int2(1, 0), new Int2(0, 1) };

        private void ReconcileFarms(Village v, int level)
        {
            foreach (var f in v.Farms)
            {
                if (f.Position.Level != level) continue;
                string asset = f.Crop == "grain" ? "grain_plot" : f.Crop == "catnip" ? "catnip_plot" : "herb_plot";
                float width = .85f + (float)Math.Min(1, f.Growth) * .15f;
                for (int x = 0; x < f.Width; x++) for (int z = 0; z < f.Depth; z++)
                    Entity("farm:" + f.Id + ":" + x + ":" + z, asset, At(new Int2(f.Position.X + x, f.Position.Z + z)), width);
            }
        }

        /// <summary>The static props of one explored surface tile.</summary>
        private void AddKnownTile(StaticProps.Chunk chunk, World world, Village v, Tile tile)
        {
            var p = tile.Position;
            var at = new Vector3(p.X, (float)world.WalkHeight(p), p.Z);
            if (tile.Wall)
            {
                staticProps.Add(chunk, "fence_post", Matrix4x4.Translate(at), true, PropLook.Plain, p);
                foreach (var step in FenceSteps)
                {
                    var next = new Int2(p.X + step.X, p.Z + step.Z);
                    if (!WallAt(world, next)) continue;
                    var mid = (at + new Vector3(next.X, (float)world.WalkHeight(next), next.Z)) * .5f;
                    staticProps.Add(chunk, "fence_rail", Matrix4x4.TRS(mid, Quaternion.Euler(0, step.X == 0 ? 90 : 0, 0), Vector3.one), true, PropLook.Plain);
                }
                return;
            }
            string asset = TileAsset(tile);
            bool hasProp = asset != "";
            bool paved = tile.Road || tile.Overlay == "road_built" || tile.Rail || tile.Bridge;
            if (asset == "tree_oak") asset = Canopy.HarvestTree(canopySeed, tile);
            if (hasProp)
            {
                if (asset == "road" && !tile.Bridge)
                {
                    var road = RoadModel(p, knownSet);
                    staticProps.Add(chunk, road.Asset, Matrix4x4.TRS(at, Quaternion.Euler(0, road.Rotation, 0), Vector3.one), true, new PropLook(false, 1, 0), p);
                }
                else if (paved || asset == "dock")
                    AddProp(chunk, asset, at, 0, paved ? 1 : .85f, 1, new PropLook(false, 1, 0), p);
                else
                {
                    bool tree = asset == "tree_oak" || asset.StartsWith("tree_", StringComparison.Ordinal) || asset.StartsWith("world_tree_", StringComparison.Ordinal);
                    bool bush = asset == "berry_bush" || asset == "shrub";
                    Canopy.Jitter(canopySeed, p, tree ? .25f : .15f, tree ? .8f : .85f, tree ? 1.25f : 1.15f, out float dx, out float dz, out float yaw, out float scale, out int tint);
                    AddProp(chunk, asset, at + new Vector3(dx, 0, dz), yaw, (tree ? Canopy.TreeWidth : .85f) * scale, bush ? .5f : 1, new PropLook(true, tint, 0), p);
                }
            }
            else if (tile.Resource == "logs" && !tile.Water)
            {
                // A felled tree leaves its stump until it regrows.
                Canopy.Jitter(canopySeed, p, .15f, .85f, 1.1f, out float dx, out float dz, out float yaw, out float scale, out int tint);
                AddProp(chunk, "stump", at + new Vector3(dx, 0, dz), yaw, .55f * scale, 1, new PropLook(true, tint, 0), p);
                hasProp = true;
            }
            if (tile.Water && tile.WaterDepth > 0 && tile.WaterDepth <= .45 && Canopy.Unit(canopySeed, p.X, p.Z, 60) < .34f)
            {
                Canopy.Jitter(canopySeed, p, .2f, .8f, 1.2f, out float dx, out float dz, out float yaw, out float scale, out int tint);
                AddProp(chunk, "world_reeds", at + new Vector3(dx, 0, dz), yaw, .65f * scale, 1, new PropLook(false, tint, 0));
            }
            int dxc = p.X - v.Center.X, dzc = p.Z - v.Center.Z;
            bool cardinalGate = v.LayoutVersion > 0 && (dxc == 0 && Math.Abs(dzc) == v.Radius || dzc == 0 && Math.Abs(dxc) == v.Radius);
            if (tile.Road || tile.Dirt || tile.Overlay == "road_built" || cardinalGate)
            {
                bool horizontal = WallAt(world, new Int2(p.X - 1, p.Z)) && WallAt(world, new Int2(p.X + 1, p.Z));
                bool vertical = WallAt(world, new Int2(p.X, p.Z - 1)) && WallAt(world, new Int2(p.X, p.Z + 1));
                if (horizontal || vertical)
                {
                    staticProps.Add(chunk, "gate", Matrix4x4.TRS(at, Quaternion.Euler(0, horizontal ? 0 : 90, 0), Vector3.one), true, PropLook.Plain, p);
                    return;
                }
            }
            if (hasProp || paved || tile.Water || occupied.Contains(p)) return;
            // Inside the fence the colony keeps its clearing: scatter only, no decorative trees.
            bool inside = Math.Abs(dxc) <= v.Radius + 1 && Math.Abs(dzc) <= v.Radius + 1;
            AddCanopy(chunk, tile, at.y, !inside && !tile.Dirt && !NearWall(world, p));
        }

        /// <summary>Boundary fences whose start tile lies in the chunk.</summary>
        private void AddBoundary(StaticProps.Chunk chunk, World world, Village v)
        {
            foreach (var edge in v.BoundaryEdges)
            {
                if (edge.From.Level != chunk.Key.Level || !ChunkKey.Of(edge.From).Equals(chunk.Key)) continue;
                if (!knownSet.Contains(edge.From) && !knownSet.Contains(edge.To)) continue;
                var mid = (At(edge.From) + At(edge.To)) * .5f;
                AddProp(chunk, "fence", mid, edge.From.X == edge.To.X ? 0 : 90, 1.05f, 1, PropLook.Plain);
            }
        }

        private bool WallAt(World world, Int2 p) => knownSet.Contains(p) && world.GetTile(p) is Tile t && t.Wall;

        private bool NearWall(World world, Int2 p)
        {
            for (int dz = -1; dz <= 1; dz++) for (int dx = -1; dx <= 1; dx++)
                if ((dx != 0 || dz != 0) && WallAt(world, new Int2(p.X + dx, p.Z + dz, p.Level))) return true;
            return false;
        }

        /// <summary>Everything about an explored tile that its static props depend on.</summary>
        private static int TileSignature(Tile t)
        {
            unchecked
            {
                int flags = (t.Wall ? 1 : 0) | (t.Road ? 2 : 0) | (t.Dirt ? 4 : 0) | (t.Rail ? 8 : 0) | (t.Bridge ? 16 : 0) | (t.Dock ? 32 : 0)
                    | (t.Water ? 64 : 0) | (t.Mountain ? 128 : 0) | (t.Amount > 0 ? 256 : 0) | ((int)Math.Min(3, Math.Max(0, t.PathWear / 50 * 4)) << 9);
                int h = flags;
                h = h * 31 + RuntimeHelpers.GetHashCode(t.Resource);
                h = h * 31 + RuntimeHelpers.GetHashCode(t.Overlay);
                h = h * 31 + RuntimeHelpers.GetHashCode(t.Biome);
                h = h * 31 + t.Elevation.GetHashCode();
                h = h * 31 + t.WaterDepth.GetHashCode();
                return h;
            }
        }

        internal string TileAsset(Tile tile)
        {
            if (tile.Wall) return "fence";
            if (tile.Dock) return "dock";
            if (tile.Rail) return "rail";
            if (tile.Bridge) return "road";
            if (tile.Road || tile.Overlay == "road_built") return "road";
            if (tile.Amount <= 0) return "";
            switch (tile.Resource.ToLowerInvariant())
            {
                case "logs": case "wood": return "tree_oak";
                case "materials": case "stone": return "rock";
                case "ore": case "iron": return "ore_iron";
                case "coal": return "ore_coal";
                case "food": return "berry_bush";
                case "herbs": return "herb_plot";
                case "fibre": return "shrub";
                case "water": case "fish": return tile.Water ? "" : "reeds";
                default: return "rock";
            }
        }

        private (string Asset, float Rotation) RoadModel(Int2 p, HashSet<Int2> known)
        {
            bool Road(int x, int z) { var next = new Int2(p.X + x, p.Z + z, p.Level); var tile = known.Contains(next) ? Game.CurrentWorld.GetTile(next) : null; return tile != null && (tile.Road || tile.Overlay == "road_built" || tile.Bridge); }
            int mask = (Road(0, 1) ? 1 : 0) | (Road(1, 0) ? 2 : 0) | (Road(0, -1) ? 4 : 0) | (Road(-1, 0) ? 8 : 0);
            return mask switch { 15 => ("world_road_cross", 0), 11 => ("world_road_junction", 0), 7 => ("world_road_junction", 90), 14 => ("world_road_junction", 180), 13 => ("world_road_junction", 270), 3 => ("world_road_corner", 0), 6 => ("world_road_corner", 90), 12 => ("world_road_corner", 180), 9 => ("world_road_corner", 270), 10 => ("world_road_straight", 90), 5 => ("world_road_straight", 0), 2 => ("world_road_end", 90), 4 => ("world_road_end", 180), 8 => ("world_road_end", 270), _ => ("world_road_end", 0) };
        }
    }
}
