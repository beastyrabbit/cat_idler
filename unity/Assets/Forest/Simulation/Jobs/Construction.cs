using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private bool CanConstructJob(Village v, Job j)
        {
            if (j.Kind == JobKind.Expand)
            {
                int radius = v.Radius + 2;
                return j.OriginalKind == JobKind.ExpandVillage ? CanModifyTerrain(v.Id, j.Position) : CanModifyTerrain(v.Id, new Int2(v.Center.X - radius, v.Center.Z - radius), radius * 2 + 1, radius * 2 + 1);
            }
            if (j.Kind == JobKind.Build)
            {
                var scaffold = v.Buildings.Find(b => b.Id == j.TargetId);
                return scaffold == null || CanModifyTerrain(v.Id, scaffold.Position, scaffold.Width, scaffold.Depth);
            }
            if (j.Kind == JobKind.Road || j.Kind == JobKind.Rail || j.Kind == JobKind.Bridge || j.Kind == JobKind.Dock || j.Kind == JobKind.Wagon || j.Kind == JobKind.Vessel)
                return j.Path.All(p => CanModifyTerrain(v.Id, p)) && (j.Kind != JobKind.Wagon && j.Kind != JobKind.Vessel || CanModifyTerrain(v.Id, j.Position));
            return true;
        }
        private Building ResolveScaffold(Village v, Cat c, Job j)
        {
            string kind = Snake(j.PendingBuildingKind == "" ? "den" : j.PendingBuildingKind);
            if (!Catalog.BuildingAvailable(v, kind) || kind == "shrine")
            {
                j.BlockedReason = Blocked.BuildingResearchRequired;
                return null;
            }
            Int2? site = null;
            foreach (var p in v.Known.OrderBy(p => Int2.Distance(v.Center, p)).ThenBy(p => p.Z).ThenBy(p => p.X))
                if (FreeSite(v, p, 2, 2, kind == "field") && FreeBuildingEntrance(v, p).HasValue && Path(c.Position, p, v) != null)
                {
                    site = p;
                    break;
                }
            if (!site.HasValue)
            {
                j.BlockedReason = Blocked.SiteRequired;
                return null;
            }
            int count = v.Buildings.Count(b => b.Kind == kind);
            double timber = Pace.ScaffoldTimberBase + count * Pace.ScaffoldTimberPerExisting, blocks = Pace.ScaffoldBlocksBase + count * Pace.ScaffoldBlocksPerExisting;
            string timberKind = Available(v, "lumber") >= timber ? "lumber" : "planks";
            var bill = j.Reserved.Count > 0 ? j.Reserved.Select(s => new Stack(s.Resource, s.Amount)).ToList() : new List<Stack> { new Stack(timberKind, timber), new Stack("blocks", blocks) };
            if (bill.Any(s => Available(v, s.Resource) + Amount(j.Local, s.Resource) + Amount(c.Cargo, s.Resource) < s.Amount))
            {
                j.BlockedReason = Blocked.MissingScaffoldInput;
                return null;
            }
            var scaffold = new Building { Id = Id(kind), Kind = kind, Position = site.Value, Entrance = FreeBuildingEntrance(v, site.Value).Value, HasEntrance = true, Required = bill, RequiredWork = Math.Max(Pace.ScaffoldMinWork, j.RequiredWork), Progress = j.Progress, Inputs = j.Local };
            j.Local = new List<Stack>();
            v.Buildings.Add(scaffold);
            j.TargetId = scaffold.Id;
            j.Position = scaffold.Position;
            j.BlockedReason = "";
            return scaffold;
        }
        private bool PendingExpansionWall(Village v, Int2 at) => v.Jobs.Any(j => !j.Completed && j.Kind == JobKind.Expand && j.OriginalKind != JobKind.ExpandVillage && j.Path.Contains(at));
        private Int2? FreeBuildingEntrance(Village v, Int2 position)
        {
            var expansions = v.Jobs.Where(j => !j.Completed && j.Kind == JobKind.Expand && j.OriginalKind != JobKind.ExpandVillage).ToArray();
            var walls = expansions.SelectMany(j => j.Path).ToHashSet();
            var roads = ConnectedRoads(v, walls, expansions.Length > 0 ? v.Radius + 2 : 0);
            foreach (var at in EntranceCandidates(new Building { Position = position }).OrderBy(p => Int2.Distance(p, v.Center)).ThenBy(p => p.Z).ThenBy(p => p.X))
                if (roads.Contains(at))
                    return at;
            return null;
        }
    }
}
