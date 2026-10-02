using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>Per-tile scenery: farm plots, resources, roads, fences, gates, reeds and boundary fences.</summary>
    public sealed partial class ForestView
    {
        private void ReconcileFarms(Village v, int level)
        {
            foreach (var f in v.Farms.Where(f => f.Position.Level == level))
                for (int x = 0; x < f.Width; x++) for (int z = 0; z < f.Depth; z++)
                    Entity("farm:" + f.Id + ":" + x + ":" + z, f.Crop == "grain" ? "grain_plot" : f.Crop == "catnip" ? "catnip_plot" : "herb_plot", At(new Int2(f.Position.X + x, f.Position.Z + z)), .85f + (float)Math.Min(1, f.Growth) * .15f);
        }

        private void ReconcileTiles(World world, Village v, int level, HashSet<Int2> known)
        {
            var tiles = world.Tiles.ToDictionary(t => t.Position);
            bool Wall(Int2 p) => known.Contains(p) && tiles.TryGetValue(p, out var t) && t.Wall;
            foreach (var tile in world.Tiles)
            {
                if (tile.Position.Level != level || tile.Position.Level != 0 || Int2.Distance(tile.Position, groundCenter) > 76 || !known.Contains(tile.Position)) continue;
                if (tile.Wall)
                {
                    Entity("fence:post:" + tile.Position, "fence_post", At(tile.Position), 1, naturalScale: true);
                    foreach (var step in new[] { new Int2(1, 0), new Int2(0, 1) })
                    {
                        var next = new Int2(tile.Position.X + step.X, tile.Position.Z + step.Z);
                        if (!Wall(next)) continue;
                        var rail = Entity("fence:rail:" + tile.Position + ":" + next, "fence_rail", (At(tile.Position) + At(next)) * .5f, 1, naturalScale: true);
                        rail.transform.rotation = Quaternion.Euler(0, step.X == 0 ? 90 : 0, 0);
                    }
                    continue;
                }
                string asset = TileAsset(tile);
                if (asset == "tree_oak" && tile.Biome == "pine_forest") asset = "world_tree_pine";
                else if (asset == "tree_oak" && World.Hash(tile.Position.ToString()) % 3 == 0) asset = "world_tree_birch";
                if (asset != "")
                {
                    var road = RoadModel(tile.Position, known);
                    bool paving = asset == "road" && !tile.Bridge;
                    if (paving) asset = road.Asset;
                    float width = tile.Road || tile.Overlay == "road_built" || tile.Rail || tile.Bridge ? 1 : asset == "tree_oak" || asset.StartsWith("world_tree_", StringComparison.Ordinal) ? 1.7f : .85f;
                    var foliage = Entity("tile:" + tile.Position, asset, At(tile.Position), width, naturalScale: paving);
                    if (paving) foliage.transform.rotation = Quaternion.Euler(0, road.Rotation, 0);
                    if (asset == "berry_bush" || asset == "shrub") foliage.transform.localScale = new Vector3(width, width * .5f, width);
                }
                if (tile.Water && tile.WaterDepth > 0 && tile.WaterDepth <= .45 && World.Hash(tile.Position.ToString()) % 3 == 0)
                    Entity("reeds:" + tile.Position, "world_reeds", At(tile.Position), .65f);
                int dx = tile.Position.X - v.Center.X, dz = tile.Position.Z - v.Center.Z;
                bool cardinalGate = v.LayoutVersion > 0 && (dx == 0 && Math.Abs(dz) == v.Radius || dz == 0 && Math.Abs(dx) == v.Radius);
                if (tile.Road || tile.Dirt || tile.Overlay == "road_built" || cardinalGate)
                {
                    bool horizontal = Wall(new Int2(tile.Position.X - 1, tile.Position.Z)) && Wall(new Int2(tile.Position.X + 1, tile.Position.Z));
                    bool vertical = Wall(new Int2(tile.Position.X, tile.Position.Z - 1)) && Wall(new Int2(tile.Position.X, tile.Position.Z + 1));
                    if (horizontal || vertical)
                    {
                        var gate = Entity("gate:" + tile.Position, "gate", At(tile.Position), 1, naturalScale: true);
                        gate.transform.rotation = Quaternion.Euler(0, horizontal ? 0 : 90, 0);
                    }
                }
            }
        }

        private void ReconcileBoundary(Village v, int level, HashSet<Int2> known)
        {
            foreach (var edge in v.BoundaryEdges)
            {
                if (edge.From.Level != level) continue;
                if (!known.Contains(edge.From) && !known.Contains(edge.To)) continue;
                var go = Entity("boundary:" + edge.From + ":" + edge.To, "fence", (At(edge.From) + At(edge.To)) * .5f, 1.05f);
                go.transform.rotation = Quaternion.Euler(0, edge.From.X == edge.To.X ? 0 : 90, 0);
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
