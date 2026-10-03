using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>Whether a mover may stand on <paramref name="p"/>. Inserts the tile (a mover about to step there explores it).</summary>
        public bool Walkable(Int2 p) => WalkableTile(TileAt(p));
        public List<Int2> Path(Int2 start, Int2 end) => Path(start, end, null);
        /// <summary>Whether a cat of <paramref name="village"/> may stand on <paramref name="p"/>. Inserts the tile like <see cref="Walkable(Int2)"/>.</summary>
        public bool Walkable(Village village, Int2 p) => WalkableTile(village, p, TileAt(p));
        private bool WalkableTile(Tile t) => !t.Wall && !t.Mountain && (!t.Water || t.Bridge || GenerationVersion > 0 && t.WaterDepth > 0 && t.WaterDepth <= .45);
        private bool WalkableTile(Village village, Int2 p, Tile t)
        {
            if (t.Landform == "dungeon_opening")
                return false;
            if (t.Wall || t.Water && !t.Bridge && !(GenerationVersion > 0 && t.WaterDepth > 0 && t.WaterDepth <= .45) || t.Mountain && (p.Level != 0 || !Catalog.Owns(this, village, "unlock_capability", "mountain_travel")))
                return false;
            return !village.Stockpiles.Any(s => s.Kind == "zone_avoid" && Contains(s.Position, s.Width, s.Depth, p));
        }
        /// <summary>
        /// A 4-connected route from <paramref name="start"/> to <paramref name="end"/>, or null. The search
        /// reads saved tiles and cached generated ones and never inserts a tile, so a failed search toward an
        /// unreachable target leaves the saved world unchanged.
        /// </summary>
        public List<Int2> Path(Int2 start, Int2 end, Village village)
        {
            pathSearchCount++;
            RefreshGeneratedTiles();
            generatedTilesChecked = true;
            try
            {
                return Search(start, end, village);
            }
            finally
            {
                generatedTilesChecked = false;
            }
        }
        private List<Int2> Search(Int2 start, Int2 end, Village village)
        {
            bool Passable(Int2 p) => village == null ? WalkableTile(ReadTile(p)) : WalkableTile(village, p, ReadTile(p));
            if (start.Equals(end))
                return new List<Int2>();
            if (!Passable(end))
                return null;
            var seen = new Dictionary<Int2, Int2> { { start, start } };
            var costs = new Dictionary<Int2, int> { { start, 0 } };
            int sequence = 0;
            var frontier = new SortedSet<(int F, int H, int Order, Int2 Position)>(Comparer<(int F, int H, int Order, Int2 Position)>.Create((a, b) => { int f = a.F.CompareTo(b.F); if (f != 0) return f; int h = a.H.CompareTo(b.H); return h != 0 ? h : a.Order.CompareTo(b.Order); }));
            frontier.Add((Int2.Distance(start, end), Int2.Distance(start, end), sequence++, start));
            int allowance = Int2.Distance(start, end) + Math.Max(64, (village?.Radius ?? Villages.Select(v => v.Radius).DefaultIfEmpty(6).Max()) * 4 + 16);
            while (frontier.Count > 0 && seen.Count < 20000)
            {
                var current = frontier.Min;
                frontier.Remove(current);
                var at = current.Position;
                foreach (var n0 in Neighbors(at).Concat(start.Level != 0 || end.Level != 0 ? DungeonNeighbors(at) : Enumerable.Empty<Int2>()))
                {
                    var n = n0;
                    int cost = costs[at] + Int2.Distance(at, n);
                    if (costs.TryGetValue(n, out var previous) && previous <= cost || Int2.Distance(start, n) > allowance || !Passable(n) || !Crossable(at, n))
                        continue;
                    seen[n] = at;
                    costs[n] = cost;
                    if (n.Equals(end))
                    {
                        var result = new List<Int2>();
                        while (!n.Equals(start))
                        {
                            result.Add(n);
                            n = seen[n];
                        }
                        result.Reverse();
                        return result;
                    }
                    int h = Int2.Distance(n, end);
                    frontier.Add((cost + h, h, sequence++, n));
                }
            }
            return null;
        }
        private double CatMovementSpeed(Village village, Tile tile)
        {
            var pace = Pace;
            double terrain = tile.Road ? pace.RoadSpeedFactor : tile.Dirt ? pace.DirtSpeedFactor : tile.Mountain ? pace.MountainSpeedFactor : tile.Biome.Contains("forest") ? pace.ForestSpeedFactor : pace.OpenGroundSpeedFactor;
            if (tile.Water && !tile.Bridge)
                terrain *= pace.WadingSpeedFactor;
            return pace.CatBaseSpeed * (village == null ? 1 : Catalog.Effect(this, village, "movementSpeed", 1) * Catalog.Effect(this, village, "moveSpeedMult", 1)) * terrain;
        }
        private static void ClearPathFailure(Cat c)
        {
            c.HasFailedPath = false;
            c.FailedPathStart = c.FailedPathDestination = default;
            c.NextPathAttemptAt = 0;
        }
        private bool Move(Cat c, Int2 destination, double dt)
        {
            faultActor = c;
            var village = Village(c.VillageId);
            // The tile a cat stands on is explored: route searches no longer insert their start.
            TileAt(c.Position);
            if (!c.HasHeight)
            {
                c.Y = WalkHeight(c.Position);
                c.HasHeight = true;
            }
            if (c.Position.Equals(destination) && Math.Abs(c.X - destination.X) + Math.Abs(c.Z - destination.Z) < 1e-9)
            {
                c.X = destination.X;
                c.Z = destination.Z;
                c.Y = WalkHeight(destination);
                c.Path.Clear();
                c.BlockedReason = "";
                ClearPathFailure(c);
                return true;
            }
            dt = Math.Min(dt, ActorTime(c));
            if (dt <= 1e-12)
                return false;
            bool changed = c.Path.Count == 0 || !c.Path[c.Path.Count - 1].Equals(destination) || (village == null ? !Walkable(c.Path[0]) : !Walkable(village, c.Path[0])) || !Crossable(c.Position, c.Path[0]);
            if (!changed)
            {
                double x = c.X - c.Position.X, z = c.Z - c.Position.Z;
                int dx = c.Path[0].X - c.Position.X, dz = c.Path[0].Z - c.Position.Z;
                changed = Math.Abs(x * dz - z * dx) > 1e-9 || x * dx + z * dz < -1e-9;
            }
            if (changed && Math.Abs(c.X - c.Position.X) + Math.Abs(c.Z - c.Position.Z) > 1e-9)
            {
                // A changed destination must first retrace the unfinished edge physically.
                double dx = c.Position.X - c.X, dz = c.Position.Z - c.Z, dy = WalkHeight(c.Position) - c.Y, distance = Math.Sqrt(dx * dx + dz * dz + dy * dy);
                var segmentEnd = new Int2(c.Position.X + Math.Sign(c.X - c.Position.X), c.Position.Z + Math.Sign(c.Z - c.Position.Z), c.Position.Level);
                double speed = CatMovementSpeed(village, TileAt(segmentEnd));
                double used = Math.Min(dt, distance / speed);
                c.X += dx / distance * used * speed;
                c.Z += dz / distance * used * speed;
                c.Y += dy / distance * used * speed;
                UseActorTime(c, used);
                dt -= used;
                if (used * speed + 1e-9 < distance)
                    return false;
                c.X = c.Position.X;
                c.Z = c.Position.Z;
                c.Y = WalkHeight(c.Position);
                c.Path.Clear();
                if (c.Position.Equals(destination))
                {
                    ClearPathFailure(c);
                    return true;
                }
            }
            if (changed)
            {
                if (c.HasFailedPath && c.FailedPathStart.Equals(c.Position) && c.FailedPathDestination.Equals(destination) && TimeSeconds + 1e-9 < c.NextPathAttemptAt)
                {
                    c.BlockedReason = "blocked_route";
                    return false;
                }
                c.Path = Path(c.Position, destination, village) ?? new List<Int2>();
                if (c.Path.Count == 0)
                {
                    c.HasFailedPath = true;
                    c.FailedPathStart = c.Position;
                    c.FailedPathDestination = destination;
                    c.NextPathAttemptAt = Math.Floor(TimeSeconds) + 1;
                    c.BlockedReason = "blocked_route";
                    return false;
                }
                ClearPathFailure(c);
            }
            double remainingSeconds = dt;
            while (remainingSeconds > 0 && c.Path.Count > 0)
            {
                var next = c.Path[0];
                if ((village == null ? !Walkable(next) : !Walkable(village, next)) || !Crossable(c.Position, next))
                {
                    c.Path.Clear();
                    c.BlockedReason = "blocked_route";
                    return false;
                }
                double dx = next.X - c.X, dz = next.Z - c.Z, dy = WalkHeight(next) - c.Y, dist = Math.Sqrt(dx * dx + dz * dz + dy * dy);
                var tile = TileAt(next);
                double speed = CatMovementSpeed(village, tile);
                double step = Math.Min(dist, remainingSeconds * speed);
                if (dist > 1e-9)
                {
                    c.X += dx / dist * step;
                    c.Z += dz / dist * step;
                    c.Y += dy / dist * step;
                }
                remainingSeconds -= step / speed;
                UseActorTime(c, step / speed);
                if (step + 1e-9 >= dist)
                {
                    c.Position = next;
                    c.Y = WalkHeight(next);
                    c.Path.RemoveAt(0);
                    if (!tile.Water && !tile.Mountain && !tile.Road)
                    {
                        tile.PathWear += 1;
                        if (tile.PathWear >= Pace.PathWearToDirt)
                            tile.Dirt = true;
                    }
                }
            }
            c.BlockedReason = "";
            ClearPathFailure(c);
            return c.Position.Equals(destination);
        }
        /// <summary>
        /// <see cref="WalkHeight"/> for route checks: an unexplored tile reads as what TileAt would insert
        /// (a route search no longer inserts the tiles it compares) instead of the presentation's 0.
        /// </summary>
        private double RouteHeight(Int2 p)
        {
            if (p.Level < 0)
                return DungeonHeight(p);
            var tile = ReadTile(p);
            return tile.Bridge && tile.Water ? tile.WaterSurface + 0.08 : tile.Elevation;
        }
        public bool Crossable(Int2 a, Int2 b) => Crossable(a, b, null, 0);
        private bool Crossable(Int2 a, Int2 b, Village expanding, int expansionRadius)
        {
            if (a.Level != b.Level)
                return DungeonEdge(a, b);
            if (GenerationVersion > 0 && Math.Abs(RouteHeight(a) - RouteHeight(b)) > .65)
                return false;
            foreach (var v in Villages)
            {
                if (ReferenceEquals(v, expanding) ? ExpansionFarmBoundary(v, expansionRadius, a, b) : v.BoundaryEdges.Any(edge => edge.From.Equals(a) && edge.To.Equals(b) || edge.From.Equals(b) && edge.To.Equals(a)))
                    return false;
                foreach (var building in v.Buildings)
                {
                    if (!building.HasEntrance)
                        continue;
                    bool fromInside = Contains(building.Position, building.Width, building.Depth, a), toInside = Contains(building.Position, building.Width, building.Depth, b);
                    if (fromInside != toInside && !(fromInside ? b : a).Equals(building.Entrance))
                        return false;
                }
            }
            return true;
        }
    }
}
