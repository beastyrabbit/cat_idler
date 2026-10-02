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
                    if (job.Local.Count > 0)
                        Spill(v, job.Position, job.Local);
                    foreach (var itemId in job.ItemIds)
                    {
                        var item = v.Items.Find(i => i.Id == itemId);
                        if (item != null)
                        {
                            var source = job.Phase == JobPhase.ItemFetch ? v.Stockpiles.Find(p => p.Id == job.SourceId) : null;
                            if (source != null)
                            {
                                item.LocationId = source.Id;
                                continue;
                            }
                            var spill = new Stockpile { Id = Id("spill"), Kind = "spill", Width = 1, Depth = 1, Position = job.Phase == JobPhase.ItemFetch ? job.Position : c.Position, Capacity = Pace.ItemSpillPileCapacity };
                            v.Stockpiles.Add(spill);
                            item.LocationId = spill.Id;
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
            c.Goal = c.BuildingId == "" ? "idle" : "transport returning";
        }
        private void Finish(Village v, Cat c, Job j)
        {
            j.Completed = true;
            Release(j.Id);
            c.JobId = "";
            c.Path.Clear();
            c.Goal = "idle";
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
            Note(v, "completed", c.Name + " completed " + j.Kind, j.Id);
        }
    }
}
