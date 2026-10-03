using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private static bool RoadSurface(Tile tile) => tile != null && (tile.Road || tile.Overlay == "road_built");
        private static IEnumerable<Int2> EntranceCandidates(Building building)
        {
            for (int x = 0; x < building.Width; x++)
            {
                yield return new Int2(building.Position.X + x, building.Position.Z - 1);
                yield return new Int2(building.Position.X + x, building.Position.Z + building.Depth);
            }
            for (int z = 0; z < building.Depth; z++)
            {
                yield return new Int2(building.Position.X - 1, building.Position.Z + z);
                yield return new Int2(building.Position.X + building.Width, building.Position.Z + z);
            }
        }
        private bool RoadSpace(Village v, Int2 p) => !v.Buildings.Any(b => Contains(b.Position, b.Width, b.Depth, p)) && !v.Stockpiles.Any(s => s.Kind != "spill" && !s.Kind.StartsWith("zone_", StringComparison.Ordinal) && Contains(s.Position, s.Width, s.Depth, p)) && !v.Farms.Any(f => Contains(f.Position, f.Width, f.Depth, p));
        private HashSet<Int2> ConnectedRoads(Village v, HashSet<Int2> excluded = null, int expansionRadius = 0)
        {
            var seen = new HashSet<Int2>();
            var pending = new Queue<Int2>();
            var shrine = v.Buildings.FirstOrDefault(b => b.Kind == "shrine" && b.Completed);
            if (shrine == null)
                return seen;
            for (int x = 0; x < shrine.Width; x++)
                for (int z = 0; z < shrine.Depth; z++)
                    pending.Enqueue(new Int2(shrine.Position.X + x, shrine.Position.Z + z));
            while (pending.Count > 0)
            {
                var at = pending.Dequeue();
                foreach (var p in Neighbors(at))
                    if (!seen.Contains(p) && (excluded == null || !excluded.Contains(p)) && RoadSurface(GetTile(p)) && RoadSpace(v, p) && Walkable(v, p) && (!ReferenceEquals(Village(v.Id), v) || Crossable(at, p, expansionRadius > 0 ? v : null, expansionRadius)))
                    {
                        seen.Add(p);
                        pending.Enqueue(p);
                    }
            }
            return seen;
        }
        public Int2? BuildingEntrance(Village v, Building building)
        {
            if (building.HasEntrance)
                return building.Entrance;
            var roads = ConnectedRoads(v);
            foreach (var p in EntranceCandidates(building).OrderBy(p => Int2.Distance(p, v.Center)).ThenBy(p => p.Z).ThenBy(p => p.X))
                if (roads.Contains(p))
                    return p;
            return null;
        }
        // ---- Version 2 colony layout -------------------------------------------------------

        /// <summary>A footprint relative to the village centre.</summary>
        private struct LayoutRect
        {
            public int X, Z, Width, Depth;
            public LayoutRect(int x, int z, int width, int depth)
            {
                X = x;
                Z = z;
                Width = width;
                Depth = depth;
            }
            public bool Contains(int x, int z) => x >= X && x < X + Width && z >= Z && z < Z + Depth;
            public LayoutRect Offset(int dx, int dz) => new LayoutRect(X + dx, Z + dz, Width, Depth);
        }

        /// <summary>The spare den each version 2 founding adds beyond its founders' beds, beside the north road.</summary>
        private static (string Kind, int X, int Z) SpareDen(bool communal) => communal ? ("den", 1, 5) : ("den", 1, 3);
        private static readonly (int X, int Z)[] JitterSteps = { (1, 0), (-1, 0), (0, 1), (0, -1), (0, 0) };

        /// <summary>
        /// The classic blueprint plus one spare den, mirrored and transposed by seed (eight variants) and each
        /// footprint nudged by at most one tile. Roads, walls and the shrine are symmetric, so every variant
        /// keeps them; a nudge is kept only while every station still reaches a road through free ground.
        /// The last rectangle returned is the founding store.
        /// </summary>
        private List<(string Kind, LayoutRect Rect)> ColonyLayout(Village v)
        {
            int radius = v.Radius;
            var entries = FoundingBlueprint(v.Communal).Concat(new[] { SpareDen(v.Communal) }).Select(e => (e.Kind, Rect: new LayoutRect(e.X, e.Z, 2, 2))).ToList();
            entries.Add(("store", v.Communal ? new LayoutRect(-4, -4, 2, 2) : new LayoutRect(3, -2, 2, 2)));
            uint variant = Hash(Seed + ":" + v.Id + ":layout");
            LayoutRect Transform(LayoutRect r)
            {
                if ((variant & 4) != 0)
                    r = new LayoutRect(r.Z, r.X, r.Depth, r.Width);
                if ((variant & 1) != 0)
                    r.X = -(r.X + r.Width - 1);
                if ((variant & 2) != 0)
                    r.Z = -(r.Z + r.Depth - 1);
                return r;
            }
            var placed = entries.Select(e => (e.Kind, Rect: Transform(e.Rect))).ToList();
            if (!LayoutConnects(radius, placed.Select(e => e.Rect).ToList(), placed.Count - 1))
            {
                // Never expected (the classic blueprint and the spare den connect in every variant), but a
                // founding must not fail: drop the spare den rather than throw.
                placed.RemoveAt(placed.Count - 2);
                return placed;
            }
            var nudged = placed.ToList();
            for (int i = 0; i < nudged.Count; i++)
            {
                var step = JitterSteps[Hash(Seed + ":" + v.Id + ":jitter:" + i) % (uint)JitterSteps.Length];
                var moved = nudged[i].Rect.Offset(step.X, step.Z);
                bool fits = true;
                for (int x = moved.X; x < moved.X + moved.Width && fits; x++)
                    for (int z = moved.Z; z < moved.Z + moved.Depth && fits; z++)
                    {
                        // Off the gate roads and the shrine ring, inside the walls, clear of every other footprint.
                        int ring = Math.Max(Math.Abs(x), Math.Abs(z));
                        int self = i;
                        fits = ring >= 3 && ring < radius && x != 0 && z != 0 && !nudged.Where((e, j) => j != self).Any(e => e.Rect.Contains(x, z));
                    }
                if (fits)
                    nudged[i] = (nudged[i].Kind, moved);
            }
            return LayoutConnects(radius, nudged.Select(e => e.Rect).ToList(), nudged.Count - 1) ? nudged : placed;
        }

        /// <summary>Free interior ground that joins the founding roads (the gate axes and the shrine ring).</summary>
        private static HashSet<(int X, int Z)> LayoutReach(int radius, IReadOnlyList<LayoutRect> rects)
        {
            bool Free(int x, int z) => Math.Abs(x) < radius && Math.Abs(z) < radius && Math.Max(Math.Abs(x), Math.Abs(z)) >= 2 && !rects.Any(r => r.Contains(x, z));
            var reach = new HashSet<(int X, int Z)>();
            var pending = new Queue<(int X, int Z)>();
            for (int x = -radius + 1; x < radius; x++)
                for (int z = -radius + 1; z < radius; z++)
                {
                    int ring = Math.Max(Math.Abs(x), Math.Abs(z));
                    if (((x == 0 || z == 0) && ring >= 2 || ring == 2) && Free(x, z) && reach.Add((x, z)))
                        pending.Enqueue((x, z));
                }
            while (pending.Count > 0)
            {
                var at = pending.Dequeue();
                foreach (var next in new[] { (at.X + 1, at.Z), (at.X - 1, at.Z), (at.X, at.Z + 1), (at.X, at.Z - 1) })
                    if (Free(next.Item1, next.Item2) && reach.Add(next))
                        pending.Enqueue(next);
            }
            return reach;
        }

        /// <summary>Every station except the store (index <paramref name="store"/>) has a doorway on reachable free ground.</summary>
        private static bool LayoutConnects(int radius, IReadOnlyList<LayoutRect> rects, int store)
        {
            var reach = LayoutReach(radius, rects);
            for (int i = 0; i < rects.Count; i++)
            {
                if (i == store)
                    continue;
                var r = rects[i];
                bool door = false;
                for (int x = r.X; x < r.X + r.Width && !door; x++)
                    door = reach.Contains((x, r.Z - 1)) || reach.Contains((x, r.Z + r.Depth));
                for (int z = r.Z; z < r.Z + r.Depth && !door; z++)
                    door = reach.Contains((r.X - 1, z)) || reach.Contains((r.X + r.Width, z));
                if (!door)
                    return false;
            }
            return true;
        }

        /// <summary>
        /// Free ground around the shrine (rings 2 to radius - 2) in a seeded shuffle, open ground before roads,
        /// so founders mill about the village instead of standing in ruled rows on the lanes.
        /// </summary>
        private List<Int2> PlazaTiles(Village v)
        {
            var rects = v.Buildings.Where(b => b.Kind != "shrine").Select(b => new LayoutRect(b.Position.X - v.Center.X, b.Position.Z - v.Center.Z, b.Width, b.Depth))
                .Concat(v.Stockpiles.Where(s => s.Kind != "spill" && !s.Kind.StartsWith("zone_", StringComparison.Ordinal)).Select(s => new LayoutRect(s.Position.X - v.Center.X, s.Position.Z - v.Center.Z, s.Width, s.Depth))).ToList();
            int outer = Math.Max(3, v.Radius - 2);
            return LayoutReach(v.Radius, rects).Where(p => Math.Max(Math.Abs(p.X), Math.Abs(p.Z)) <= outer)
                .Select(p => new Int2(v.Center.X + p.X, v.Center.Z + p.Z))
                .OrderBy(p => RoadSurface(GetTile(p)) ? 1 : 0).ThenBy(p => SurfaceHash(p.X, p.Z, 719u ^ Hash(v.Id))).ThenBy(p => p.Z).ThenBy(p => p.X).ToList();
        }

        private bool ShrineRing(Village v, Int2 p) => v.Buildings.Any(b => b.Kind == "shrine" && Contains(new Int2(b.Position.X - 1, b.Position.Z - 1), b.Width + 2, b.Depth + 2, p) && !Contains(b.Position, b.Width, b.Depth, p));
        private bool ConnectedEntrance(Village v, Building building) => !building.HasEntrance || EntranceCandidates(building).Contains(building.Entrance) && ConnectedRoads(v).Contains(building.Entrance);
    }
}
