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
            var bill = j.Reserved.Count > 0 ? j.Reserved.Select(s => new Stack(s.Resource, s.Amount)).ToList() : BuildCost(v, kind).Inputs;
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
        private Int2? FreeBuildingEntrance(Village v, Int2 position, int width = 2, int depth = 2)
        {
            var expansions = v.Jobs.Where(j => !j.Completed && j.Kind == JobKind.Expand && j.OriginalKind != JobKind.ExpandVillage).ToArray();
            var walls = expansions.SelectMany(j => j.Path).ToHashSet();
            var roads = ConnectedRoads(v, walls, expansions.Length > 0 ? v.Radius + 2 : 0);
            foreach (var at in EntranceCandidates(new Building { Position = position, Width = width, Depth = depth }).OrderBy(p => Int2.Distance(p, v.Center)).ThenBy(p => p.Z).ThenBy(p => p.X))
                if (roads.Contains(at))
                    return at;
            return null;
        }

        /// <summary>
        /// The construction safety net, run once per planning second (demo worlds). Every incomplete scaffold
        /// without a build job claims what it can and, once its whole bill is secured, gets a job: a free
        /// builder starts it, otherwise it waits unowned with <see cref="Blocked.WorkerRequired"/>. A scaffold
        /// whose bill is short waits without a job, marked <see cref="Blocked.MissingScaffoldInput"/>, so no cat
        /// stands holding work it cannot do. A secured site with every job staffed takes one more free builder
        /// until it has <c>BuildersPerSite</c>.
        /// </summary>
        private void EnsureScaffoldJobs(Village v)
        {
            if (!DemoConstruction)
                return;
            foreach (var b in v.Buildings.Where(b => !b.Completed && b.Kind != "shrine").ToArray())
            {
                var jobs = ScaffoldJobs(v, b).ToList();
                bool secured = SecureScaffoldInputs(v, b);
                if (jobs.Count == 0)
                {
                    b.BlockedReason = secured ? "" : Blocked.MissingScaffoldInput;
                    if (secured)
                        StartScaffoldJob(v, b, AvailableCat(v, "build"));
                    continue;
                }
                if (secured && jobs.Count < Pace.BuildersPerSite && jobs.All(j => j.CatId != ""))
                {
                    var helper = AvailableCat(v, "build");
                    if (helper != null)
                        StartScaffoldJob(v, b, helper);
                }
            }
        }

        /// <summary>The active build jobs of one scaffold.</summary>
        private IEnumerable<Job> ScaffoldJobs(Village v, Building b) => v.Jobs.Where(j => !j.Completed && j.Kind == JobKind.Build && j.TargetId == b.Id);

        /// <summary>
        /// Adds a build job for a scaffold: owned by <paramref name="builder"/>, or unowned and waiting for a
        /// worker when no builder is given. Keeps the manual and requester flags of the site's previous job.
        /// </summary>
        private Job StartScaffoldJob(Village v, Building b, Cat builder)
        {
            var previous = v.Jobs.LastOrDefault(j => j.Kind == JobKind.Build && j.TargetId == b.Id);
            var job = new Job { Kind = JobKind.Build, TargetId = b.Id, Position = b.Position, RequiredWork = b.RequiredWork, Manual = previous?.Manual ?? true, Requester = previous?.Requester ?? "player" };
            if (builder != null)
            {
                StartJob(v, builder, job);
                return job;
            }
            job.Id = Id("job");
            job.Origin = b.Position;
            job.StartedAt = TimeSeconds;
            job.BlockedReason = Blocked.WorkerRequired;
            v.Jobs.Add(job);
            return job;
        }

        /// <summary>
        /// Claims what is free toward a scaffold's bill; true when every input is staged, claimed or on the way.
        /// </summary>
        private bool SecureScaffoldInputs(Village v, Building b)
        {
            bool secured = true;
            foreach (var input in b.Required)
            {
                double need = input.Amount - ScaffoldSecured(v, b, input.Resource);
                if (need > 1e-8)
                    need -= ReserveUpTo(v, b.Id, input.Resource, need);
                if (need > 1e-8)
                    secured = false;
            }
            return secured;
        }

        /// <summary>
        /// How much of a resource a scaffold already has toward its bill: staged at the site, claimed in storage,
        /// carried by its builders, or held in a builder's suspended-cargo claim.
        /// </summary>
        private double ScaffoldSecured(Village v, Building b, string resource)
        {
            double secured = Amount(b.Inputs, resource) + Reservations.Where(r => r.OwnerId == b.Id && r.Resource == resource).Sum(r => r.Amount);
            foreach (var job in ScaffoldJobs(v, b))
            {
                var builder = v.Cats.Find(c => c.Id == job.CatId && job.CatId != "");
                if (builder != null)
                    secured += Amount(builder.Cargo, resource);
                secured += Reservations.Where(r => r.OwnerId == job.Id + ":resume" && r.Resource == resource).Sum(r => r.Amount);
            }
            return secured;
        }
    }
}
