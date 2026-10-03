using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private void CancelWork(Village v, Cat c, bool preserveUnassignedCargo = false)
        {
            CancelDungeon(c);
            if (v.Accounting?.WorkerId == c.Id)
                v.Accounting = null;
            foreach (var route in v.Routes.Where(r => r.CatId == c.Id).ToArray())
                CancelRoute(v, route.Id);
            var job = v.Jobs.Find(j => j.Id == c.JobId && !j.Completed);
            if (job != null)
            {
                bool preserve = job.Kind == JobKind.Road || job.Kind == JobKind.Rail || job.Kind == JobKind.Expand;
                if (preserve)
                {
                    if (c.Cargo.Count > 0)
                    {
                        var quantities = c.Cargo.Select(s => new Stack(s.Resource, s.Amount)).ToArray();
                        var spill = Spill(v, c.Position, c.Cargo);
                        job.SuspendedCargoPileId = spill.Id;
                        foreach (var s in quantities)
                            Reservations.Add(new Reservation { OwnerId = job.Id + ":resume", VillageId = v.Id, PileId = spill.Id, Resource = s.Resource, Amount = s.Amount });
                    }
                    job.CatId = "";
                    job.BlockedReason = Blocked.WorkerRequired;
                }
                else
                {
                    if (c.Cargo.Count > 0)
                        Spill(v, c.Position, c.Cargo);
                    Release(job.Id);
                    Release(job.Id + ":resume");
                    if (job.Kind == JobKind.Build)
                        Release(job.TargetId);
                    // Inputs of unfinished production stay at the station for its next batch instead of spilling.
                    var station = job.Kind == JobKind.Production && job.Phase != JobPhase.OutputDelivery ? v.Buildings.Find(b => b.Id == job.TargetId) : null;
                    if (station != null)
                    {
                        foreach (var input in job.Local)
                            Add(station.Inputs, input.Resource, input.Amount);
                        job.Local.Clear();
                    }
                    if (job.Local.Count > 0)
                        Spill(v, job.Position, job.Local);
                    foreach (var itemId in job.ItemIds)
                    {
                        var item = v.Items.Find(i => i.Id == itemId);
                        if (item != null)
                        {
                            var source = job.Phase == JobPhase.ItemFetch ? v.Stockpiles.Find(p => p.Id == job.SourceId) : null;
                            item.LocationId = (source ?? SpillPile(v, job.Phase == JobPhase.ItemFetch ? job.Position : c.Position, Pace.ItemSpillPileCapacity)).Id;
                        }
                    }
                    job.ItemIds.Clear();
                    job.Completed = true;
                    job.CatId = "";
                }
            }
            else if (!preserveUnassignedCargo && c.Cargo.Count > 0)
                Spill(v, c.Position, c.Cargo);
            foreach (var b in v.Buildings)
            {
                foreach (var slot in b.Slots.Where(s => s.CatId == c.Id))
                {
                    slot.CatId = "";
                    slot.JobId = "";
                    slot.BlockedReason = Blocked.WorkerRequired;
                }
                b.ExtraWorkerIds.Remove(c.Id);
                if (b.WorkerId == c.Id)
                    b.WorkerId = b.Slots.FirstOrDefault(s => s.CatId != "")?.CatId ?? "";
            }
            foreach (var f in v.Farms)
                if (f.WorkerId == c.Id)
                    f.WorkerId = "";
            c.JobId = "";
            c.BuildingId = v.Routes.FirstOrDefault(r => r.CatId == c.Id)?.Id ?? "";
            c.ResumeJobId = "";
            c.Path.Clear();
            c.Goal = c.BuildingId == "" ? GoalKey.Idle : GoalKey.TransportReturning;
        }
        private void Finish(Village v, Cat c, Job j)
        {
            j.Completed = true;
            Release(j.Id);
            c.JobId = "";
            c.Path.Clear();
            c.Goal = GoalKey.Idle;
            c.BlockedReason = "";
            string labor = j.Kind == JobKind.Production ? RecipeOf(j.RecipeId)?.Labor ?? "process" : JobLabor(j.Kind);
            if (Catalog.Labors.Contains(labor))
                GainSkill(c, labor, Pace.LaborSkillPerJob);
            string role = labor == "hunt" ? "hunter" : labor == "build" || labor == "craft" ? "architect" : labor == "research" ? "healer" : labor == "fight" ? "warrior" : "";
            if (role != "")
            {
                Add(c.RoleExperience, role, Pace.RoleExperiencePerJob);
                if (c.Specialization == "" && Amount(c.RoleExperience, role) >= Pace.SpecializationRoleExperience)
                    c.Specialization = role;
            }
            foreach (var item in v.Items.Where(i => c.Equipment.Contains(i.Id) && i.Kind == "tool"))
                item.Condition = Math.Max(0, item.Condition - Pace.ToolWearPerJob);
            Note(v, EventKind.Completed, c.Name + " completed " + j.Kind, j.Id);
            if (j.Kind == JobKind.Haul)
                PruneSpill(v, v.Stockpiles.Find(s => s.Id == j.SourceId));
        }
    }
}
