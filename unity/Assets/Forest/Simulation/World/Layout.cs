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
        private bool ShrineRing(Village v, Int2 p) => v.Buildings.Any(b => b.Kind == "shrine" && Contains(new Int2(b.Position.X - 1, b.Position.Z - 1), b.Width + 2, b.Depth + 2, p) && !Contains(b.Position, b.Width, b.Depth, p));
        private bool ConnectedEntrance(Village v, Building building) => !building.HasEntrance || EntranceCandidates(building).Contains(building.Entrance) && ConnectedRoads(v).Contains(building.Entrance);
    }
}
