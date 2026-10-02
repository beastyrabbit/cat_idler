using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private bool ExpansionFootprintBlocked(Village v, int radius, List<Int2> plannedWalls = null)
        {
            bool WallAt(Int2 p) => plannedWalls != null && plannedWalls.Contains(p) || Math.Max(Math.Abs(p.X - v.Center.X), Math.Abs(p.Z - v.Center.Z)) == radius && !LayoutGate(v, p, radius) && !FarmExterior(v, p, radius);
            var walls = plannedWalls == null ? new HashSet<Int2>() : plannedWalls.ToHashSet();
            for (int offset = -radius; offset <= radius; offset++)
                foreach (var p in new[] { new Int2(v.Center.X + offset, v.Center.Z - radius), new Int2(v.Center.X + offset, v.Center.Z + radius), new Int2(v.Center.X - radius, v.Center.Z + offset), new Int2(v.Center.X + radius, v.Center.Z + offset) })
                    if (WallAt(p))
                        walls.Add(p);
            if (v.Jobs.Any(j => !j.Completed && (j.Kind == JobKind.Road || j.Kind == JobKind.Rail) && j.Path.Any(walls.Contains)))
                return true;
            var currentRoads = ConnectedRoads(v);
            var roads = ConnectedRoads(v, walls, radius);
            bool WallCrosses(Int2 origin, int width, int depth)
            {
                for (int x = 0; x < width; x++)
                    for (int z = 0; z < depth; z++)
                        if (WallAt(new Int2(origin.X + x, origin.Z + z)))
                            return true;
                return false;
            }
            foreach (var building in v.Buildings)
            {
                if (WallCrosses(building.Position, building.Width, building.Depth))
                    return true;
                if (building.Kind != "shrine")
                {
                    var entrance = BuildingEntrance(v, building);
                    if (entrance.HasValue && (walls.Contains(entrance.Value) || currentRoads.Contains(entrance.Value) && !roads.Contains(entrance.Value)))
                        return true;
                }
            }
            return v.Stockpiles.Any(p => p.Kind != "spill" && !p.Kind.StartsWith("zone_", StringComparison.Ordinal) && WallCrosses(p.Position, p.Width, p.Depth));
        }
        private bool ExpansionFarmBoundary(Village v, int radius, Int2 from, Int2 to)
        {
            bool Inside(Int2 p) => Math.Abs(p.X - v.Center.X) <= radius && Math.Abs(p.Z - v.Center.Z) <= radius;
            return Inside(from) && Inside(to) && FarmExterior(v, from, radius) != FarmExterior(v, to, radius);
        }
        private void Expand(Village v)
        {
            v.Radius += 2;
            v.ClaimedTiles.Clear();
            v.AgriculturalTiles.Clear();
            v.BoundaryEdges.Clear();
            for (int z = -v.Radius; z <= v.Radius; z++)
                for (int x = -v.Radius; x <= v.Radius; x++)
                {
                    var p = new Int2(v.Center.X + x, v.Center.Z + z);
                    var t = TileAt(p);
                    if (FarmExterior(v, p, v.Radius))
                    {
                        t.ClaimId = "";
                        t.Wall = false;
                        v.AgriculturalTiles.Add(p);
                        continue;
                    }
                    v.ClaimedTiles.Add(p);
                    t.Water = t.Mountain = false;
                    t.Resource = "";
                    t.Amount = 0;
                    t.ClaimId = v.Id;
                    t.Biome = "meadow";
                    t.Wall = (Math.Abs(x) == v.Radius || Math.Abs(z) == v.Radius) && !LayoutGate(v, p, v.Radius);
                    if (!v.Known.Contains(p))
                        v.Known.Add(p);
                }
            foreach (var p in v.ClaimedTiles)
                foreach (var outside in Neighbors(p))
                    if (v.AgriculturalTiles.Contains(outside))
                        v.BoundaryEdges.Add(new BoundaryEdge { From = p, To = outside });
            Note(v, "expansion", "Settlement grew to radius " + v.Radius);
        }
        private bool FarmExterior(Village v, Int2 p, int radius)
        {
            foreach (var farm in v.Farms)
            {
                if (Contains(farm.Position, farm.Width, farm.Depth, p))
                    return true;
                int left = farm.Position.X - v.Center.X, right = left + farm.Width - 1, bottom = farm.Position.Z - v.Center.Z, top = bottom + farm.Depth - 1;
                int x = p.X - v.Center.X, z = p.Z - v.Center.Z;
                int[] gaps = { radius + left, radius - right, radius + bottom, radius - top };
                int side = Array.IndexOf(gaps, gaps.Min());
                if (side == 0 && z >= bottom && z <= top && x <= left || side == 1 && z >= bottom && z <= top && x >= right || side == 2 && x >= left && x <= right && z <= bottom || side == 3 && x >= left && x <= right && z >= top)
                    return true;
            }
            return false;
        }
        private void ClaimLegacyTile(Village v, Job job)
        {
            var p = job.Position;
            if (!v.ClaimedTiles.Contains(p))
                v.ClaimedTiles.Add(p);
            if (!v.Known.Contains(p))
                v.Known.Add(p);
            var tile = TileAt(p);
            tile.ClaimId = v.Id;
            tile.Water = tile.Mountain = tile.Wall = false;
            if (job.AgriculturalExpansion)
            {
                if (!v.AgriculturalTiles.Contains(p))
                    v.AgriculturalTiles.Add(p);
            }
            else
            {
                tile.Resource = "";
                tile.Amount = 0;
                tile.Deposits.Clear();
                tile.Biome = "meadow";
            }
            v.BoundaryEdges.RemoveAll(e => e.From.Equals(p) || e.To.Equals(p));
            foreach (var next in Neighbors(p))
                if (!v.ClaimedTiles.Contains(next) && !(p.X == v.Center.X && p.Z > v.Center.Z && next.Z > p.Z))
                    v.BoundaryEdges.Add(new BoundaryEdge { From = p, To = next });
            if (job.CompletionBoundaryEdges.Count > 0)
                v.BoundaryEdges = new List<BoundaryEdge>(job.CompletionBoundaryEdges);
            v.Radius = Math.Max(v.Radius, Math.Max(Math.Abs(p.X - v.Center.X), Math.Abs(p.Z - v.Center.Z)));
            Note(v, "expansion", "Restored wall project claimed one tile", p.ToString());
        }
    }
}
