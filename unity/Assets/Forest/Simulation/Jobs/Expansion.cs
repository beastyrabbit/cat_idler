using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>
        /// A tile on the new ring that becomes fence: not a gate, not part of a farm's exterior and, in demo
        /// worlds, not a natural barrier (deep water or a mountain closes the ring by itself).
        /// </summary>
        private bool ExpansionWallTile(Village v, Int2 p, int radius) => Math.Max(Math.Abs(p.X - v.Center.X), Math.Abs(p.Z - v.Center.Z)) == radius && !LayoutGate(v, p, radius) && !FarmExterior(v, p, radius) && !NaturalBarrier(v, p);

        /// <summary>
        /// Demo worlds: a mapped tile that cats cannot walk for natural reasons (deep water or a mountain, not a
        /// built wall). Expansion leaves it as it is instead of demanding that it be cleared first.
        /// </summary>
        private bool NaturalBarrier(Village v, Int2 p)
        {
            if (!DemoConstruction || !v.Known.Contains(p))
                return false;
            var t = GetTile(p);
            return t != null && !t.Wall && (t.Water || t.Mountain) && !Walkable(p);
        }

        /// <summary>
        /// The fence tiles of the ring at <paramref name="radius"/>. Classic worlds keep today's order; demo
        /// worlds walk the ring once round (south edge, east, north, west) so a builder with a full load paves
        /// neighbouring segments in a row, and skip natural barriers.
        /// </summary>
        private List<Int2> ExpansionPerimeter(Village v, int radius)
        {
            var perimeter = new List<Int2>();
            if (DemoConstruction)
            {
                for (int x = -radius; x < radius; x++)
                    perimeter.Add(new Int2(v.Center.X + x, v.Center.Z - radius));
                for (int z = -radius; z < radius; z++)
                    perimeter.Add(new Int2(v.Center.X + radius, v.Center.Z + z));
                for (int x = radius; x > -radius; x--)
                    perimeter.Add(new Int2(v.Center.X + x, v.Center.Z + radius));
                for (int z = radius; z > -radius; z--)
                    perimeter.Add(new Int2(v.Center.X - radius, v.Center.Z + z));
                perimeter.RemoveAll(p => !ExpansionWallTile(v, p, radius));
                return perimeter;
            }
            for (int x = -radius; x <= radius; x++)
            {
                perimeter.Add(new Int2(v.Center.X + x, v.Center.Z - radius));
                if (x != 0)
                    perimeter.Add(new Int2(v.Center.X + x, v.Center.Z + radius));
            }
            for (int z = -radius + 1; z < radius; z++)
            {
                perimeter.Add(new Int2(v.Center.X - radius, v.Center.Z + z));
                perimeter.Add(new Int2(v.Center.X + radius, v.Center.Z + z));
            }
            perimeter.RemoveAll(p => FarmExterior(v, p, radius) || LayoutGate(v, p, radius));
            return perimeter;
        }

        /// <summary>
        /// The expansion request (RequestJob expand): resumes an unowned pending ring, or plans the next ring two
        /// tiles out and starts it with <paramref name="c"/>. Demo worlds stage the materials at the work stand of
        /// the first segment instead of the shrine.
        /// </summary>
        private ActionResult PlanExpansion(Village v, Cat c)
        {
            var pending = v.Jobs.Find(j => j.Kind == JobKind.Expand && !j.Completed);
            if (pending != null)
            {
                if (pending.CatId != "")
                    return ActionResult.Fail(PlacementCode.Occupied, "Expansion already active");
                if (!CanConstructJob(v, pending))
                    return ActionResult.Fail(PlacementCode.OutsideClaim, "Foreign territory blocks expansion");
                if (pending.OriginalKind != JobKind.ExpandVillage && ExpansionFootprintBlocked(v, v.Radius + 2, pending.Path))
                    return ActionResult.Fail(Blocked.ExpansionFootprintBlocked, "Existing footprint or entrance blocks expansion");
                if (c.Cargo.Count > 0)
                    Spill(v, c.Position, c.Cargo);
                pending.CatId = c.Id;
                c.JobId = pending.Id;
                c.Path.Clear();
                return ActionResult.Ok(ResultCode.Assigned, pending.Id);
            }
            int radius = v.Radius + 2;
            if (!CanModifyTerrain(v.Id, new Int2(v.Center.X - radius, v.Center.Z - radius), radius * 2 + 1, radius * 2 + 1))
                return ActionResult.Fail(PlacementCode.OutsideClaim, "Foreign territory blocks expansion");
            if (ExpansionFootprintBlocked(v, radius))
                return ActionResult.Fail(Blocked.ExpansionFootprintBlocked, "Existing footprint or entrance blocks expansion");
            var perimeter = ExpansionPerimeter(v, radius);
            foreach (var p in perimeter)
                if (!v.Known.Contains(p) || !Walkable(p))
                    return ActionResult.Fail(v.Known.Contains(p) ? BarrierCode(TileAt(p)) : PlacementCode.Unmapped, "Survey a dry reachable outer perimeter first", ("x", p.X.ToString(System.Globalization.CultureInfo.InvariantCulture)), ("z", p.Z.ToString(System.Globalization.CultureInfo.InvariantCulture)));
            var shortfall = Shortfalls(v, ExpansionCost(perimeter.Count).Inputs).FirstOrDefault();
            if (shortfall != null)
                return MissingMaterialFailure(shortfall);
            var staging = DemoConstruction && perimeter.Count > 0 ? ExpansionStaging(v, perimeter[0], radius) ?? v.Center : v.Center;
            var result = CreateInputJob(v, c, JobKind.Expand, staging, "materials", perimeter.Count, Pace.ExpansionWork, "");
            if (result.Success)
                v.Jobs.Find(j => j.Id == result.EntityId).Path = perimeter;
            return result;
        }

        /// <summary>
        /// Where the ring's materials are staged: the reachable walkable tile just inside the new ring nearest the
        /// first segment (never a future fence tile, so the stack stays reachable once the ring closes).
        /// </summary>
        private Int2? ExpansionStaging(Village v, Int2 segment, int radius)
        {
            var candidates = new List<Int2>();
            for (int dz = -2; dz <= 2; dz++)
                for (int dx = -2; dx <= 2; dx++)
                {
                    var p = new Int2(segment.X + dx, segment.Z + dz);
                    if (Int2.Distance(p, segment) <= 2 && Math.Max(Math.Abs(p.X - v.Center.X), Math.Abs(p.Z - v.Center.Z)) < radius)
                        candidates.Add(p);
                }
            foreach (var p in candidates.OrderBy(p => Int2.Distance(p, segment)).ThenBy(p => Int2.Distance(p, v.Center)).ThenBy(p => p.Z).ThenBy(p => p.X))
                if (v.Known.Contains(p) && Walkable(p) && RoadSpace(v, p) && Path(v.Center, p, v) != null)
                    return p;
            return null;
        }

        private bool ExpansionFootprintBlocked(Village v, int radius, List<Int2> plannedWalls = null)
        {
            bool WallAt(Int2 p) => plannedWalls != null && plannedWalls.Contains(p) || ExpansionWallTile(v, p, radius);
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
                    bool ring = Math.Abs(x) == v.Radius || Math.Abs(z) == v.Radius;
                    if (ring && NaturalBarrier(v, p))
                    {
                        // Deep water or rock closes this stretch of the ring; it stays natural.
                        t.ClaimId = v.Id;
                        continue;
                    }
                    ClearClaimedSurface(t);
                    t.ClaimId = v.Id;
                    t.Wall = ring && !LayoutGate(v, p, v.Radius);
                    if (!v.Known.Contains(p))
                        v.Known.Add(p);
                }
            foreach (var p in v.ClaimedTiles)
                foreach (var outside in Neighbors(p))
                    if (v.AgriculturalTiles.Contains(outside))
                        v.BoundaryEdges.Add(new BoundaryEdge { From = p, To = outside });
            Note(v, EventKind.Expansion, "Settlement grew to radius " + v.Radius);
        }

        /// <summary>
        /// The one way a newly claimed tile is cleared for settlement: no water, rock or deposit, meadow ground.
        /// Demo worlds also level it to the settlement plateau like founding does, so claimed ground never mixes
        /// natural heights with the plateau depending on which tiles scouts happened to see first.
        /// </summary>
        private void ClearClaimedSurface(Tile t)
        {
            if (DemoConstruction && GenerationVersion > 0)
                ClearSurface(t, 0);
            t.Water = t.Mountain = false;
            t.Resource = "";
            t.Amount = 0;
            t.Biome = "meadow";
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
                ClearClaimedSurface(tile);
                tile.Deposits.Clear();
            }
            v.BoundaryEdges.RemoveAll(e => e.From.Equals(p) || e.To.Equals(p));
            foreach (var next in Neighbors(p))
                if (!v.ClaimedTiles.Contains(next) && !(p.X == v.Center.X && p.Z > v.Center.Z && next.Z > p.Z))
                    v.BoundaryEdges.Add(new BoundaryEdge { From = p, To = next });
            if (job.CompletionBoundaryEdges.Count > 0)
                v.BoundaryEdges = new List<BoundaryEdge>(job.CompletionBoundaryEdges);
            v.Radius = Math.Max(v.Radius, Math.Max(Math.Abs(p.X - v.Center.X), Math.Abs(p.Z - v.Center.Z)));
            Note(v, EventKind.Expansion, "Restored wall project claimed one tile", p.ToString());
        }
    }
}
