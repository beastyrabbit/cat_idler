using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        // PACING-HANDOFF BoostJobFraction: the share of a job's work one boost removes (classic 0 keeps the flat
        // Pace.BoostSeconds; demo 0.05). For a research hut the "job" is one research point of scholar work.
        private const double BoostJobFractionDemo = 0.05;
        private double BoostJobFraction => Pace.Profile == Pacing.DemoProfile ? BoostJobFractionDemo : 0;

        /// <summary>Completed buildings that take workers without having recipes.</summary>
        private static readonly string[] StaffedKinds = { "research_hut", "school", "accounting_tent", "mouse_farm", "field" };

        /// <summary>True for a building a cat can be assigned to: a scaffold, a recipe station or a staffed service.</summary>
        public static bool IsWorkplace(Building b) => b != null && (!b.Completed || StaffedKinds.Contains(b.Kind) || Catalog.Recipes.Any(r => r.Building == b.Kind));

        private static bool IsStudyHall(Building b) => b != null && b.Completed && (b.Kind == "research_hut" || b.Kind == "school");

        /// <summary>Why this cat cannot take a player order, or null. A null cat means "pick one" and passes.</summary>
        private ActionResult OrderRefusal(Village v, Cat c)
        {
            if (c == null)
                return null;
            if (c.AgeHours < Pace.WorkerMinAgeHours)
                return ActionResult.Fail(ResultCode.Kitten, "Kittens are too young to work", (ResultArg.Target, c.Id), (ResultArg.Need, ActionResult.Number(Pace.WorkerMinAgeHours)), (ResultArg.Have, ActionResult.Number(c.AgeHours)));
            if (c.Migration == "arriving" || c.Migration == "departing")
                return ActionResult.Fail(ResultCode.Migrating, "The cat is still on the road", (ResultArg.Target, c.Id));
            if (c.ControlledBy != "")
                return ActionResult.Fail(ResultCode.CatControlled, "Release direct control first", (ResultArg.Target, c.Id));
            if (IsDungeonExplorer(c))
                return ActionResult.Fail(ResultCode.ExplorerAway, "Recall the explorer and wait for its return before assigning work", (ResultArg.Target, c.Id));
            return null;
        }

        /// <summary>A missing explicitly named cat; null when no cat was named or it was found.</summary>
        private static ActionResult MissingCat(GameAction a, Cat c) => c == null && !string.IsNullOrEmpty(a.CatId) ? ActionResult.Fail(ResultCode.CatRequired, "Living cat required", (ResultArg.Target, a.CatId)) : null;

        private ActionResult HandleRequestJob(Village village, GameAction action, Cat cat)
        {
            var refusal = MissingCat(action, cat) ?? OrderRefusal(village, cat);
            if (refusal != null)
                return refusal;
            var result = Request(village, string.IsNullOrEmpty(action.Name) ? action.Resource : action.Name, cat, true, action.Resource);
            return result.Success ? result.Coded(ResultCode.Queued) : result;
        }
        private ActionResult HandleDispatchScout(Village village, GameAction action, Cat cat)
        {
            var refusal = MissingCat(action, cat) ?? OrderRefusal(village, cat);
            if (refusal != null)
                return refusal;
            var result = Request(village, JobKind.Scout, cat, true, action.Resource);
            return result.Success ? result.Coded(ResultCode.Queued) : result;
        }

        /// <summary>
        /// Boost by job, cat or building id: a job, the cat's active job (or its research hut), a scaffold's
        /// build job, a station's active job, or a research hut's scholars. Returns the seconds of work saved.
        /// </summary>
        private ActionResult HandleBoost(Village v, GameAction a)
        {
            string id = !string.IsNullOrEmpty(a.TargetId) ? a.TargetId : !string.IsNullOrEmpty(a.BuildingId) ? a.BuildingId : a.CatId;
            var job = v.Jobs.Find(j => j.Id == id && !j.Completed);
            if (job != null)
                return BoostWork(v, job);
            var cat = v.Cats.Find(c => c.Id == id && c.Alive);
            var building = v.Buildings.Find(b => b.Id == id);
            if (cat == null && building == null)
                return ActionResult.Fail(ResultCode.UnknownTarget, "Nothing to boost there", (ResultArg.Target, id ?? ""));
            if (cat != null)
            {
                job = v.Jobs.Find(j => j.Id == cat.JobId && !j.Completed);
                if (job != null)
                    return BoostWork(v, job);
                building = v.Buildings.Find(b => b.Id == cat.BuildingId && IsStudyHall(b));
                if (building == null)
                    return ActionResult.Fail(ResultCode.NoActiveJob, "This cat is not working right now", (ResultArg.Target, cat.Id));
            }
            if (IsStudyHall(building))
                return BoostResearch(v, building);
            job = v.Jobs.Where(j => !j.Completed && (j.TargetId == building.Id || building.Slots.Any(s => s.JobId == j.Id))).OrderBy(j => j.Id, StringComparer.Ordinal).FirstOrDefault();
            if (job == null)
                return ActionResult.Fail(ResultCode.NoActiveJob, "Nothing is being worked on here", (ResultArg.Target, building.Id), (ResultArg.Building, building.Kind));
            return BoostWork(v, job);
        }

        /// <summary>The next boost's seconds after click power and the per-minute falloff; counts the click.</summary>
        private (double seconds, bool diminished) NextBoost(Village v, double work)
        {
            long minute = (long)(TimeSeconds / 60);
            if (v.BoostMinute != minute)
            {
                v.BoostMinute = minute;
                v.BoostsUsed = 0;
            }
            v.BoostsUsed++;
            double fraction = BoostJobFraction;
            double seconds = (fraction > 0 ? fraction * Math.Max(0, work) : Pace.BoostSeconds) + UpgradeLevel(v, "click_power") * Pace.BoostSecondsPerClickPower;
            bool diminished = v.BoostsUsed > Pace.BoostFreePerMinute;
            if (diminished)
                seconds = Math.Floor(seconds * Math.Pow(Pace.BoostDecay, (v.BoostsUsed - Pace.BoostFreePerMinute) / Pace.BoostDecayClicks));
            return (Math.Max(Pace.BoostMinSeconds, seconds), diminished);
        }

        private ActionResult BoostWork(Village v, Job job)
        {
            var building = job.Kind == JobKind.Build ? v.Buildings.Find(b => b.Id == job.TargetId) : null;
            var (seconds, diminished) = NextBoost(v, building?.RequiredWork ?? job.RequiredWork);
            double saved;
            if (building != null)
            {
                double before = building.RequiredWork;
                building.RequiredWork = Math.Max(building.Progress + 1, building.RequiredWork - seconds);
                saved = before - building.RequiredWork;
            }
            else
            {
                double before = job.RequiredWork;
                job.RequiredWork = Math.Max(job.Progress + 1, job.RequiredWork - seconds);
                saved = before - job.RequiredWork;
            }
            job.BoostCount++;
            return ActionResult.Ok(ResultCode.Boosted, job.Id, (ResultArg.Seconds, ActionResult.Number(Math.Max(0, saved))), (ResultArg.Diminished, diminished ? "true" : "false"), (ResultArg.Kind, job.Kind), (ResultArg.Target, building?.Id ?? job.CatId));
        }

        /// <summary>Research points per second the staffed scholars of these study halls produce now.</summary>
        private double StudyRate(Village v, IEnumerable<Building> halls)
        {
            double rate = 0;
            foreach (var b in halls.Where(b => IsStudyHall(b) && !b.Paused))
                foreach (var slot in b.Slots.Where(s => !s.Paused))
                {
                    var c = v.Cats.Find(x => x.Id == slot.CatId && x.Alive);
                    if (c == null || c.ControlledBy != "" || c.JobId != "" || IsDungeonExplorer(c))
                        continue;
                    rate += WorkRate(v, c, "research", b.Kind) * Catalog.Effect(v, "researchRateMult", 1) * Catalog.BuildingEffect(v, b.Kind, "output", 1) / Pace.ResearchDivisor;
                }
            return rate;
        }

        private ActionResult BoostResearch(Village v, Building hall)
        {
            double rate = StudyRate(v, new[] { hall });
            if (rate <= 0)
                return ActionResult.Fail(ResultCode.NoActiveJob, "Nobody is studying here", (ResultArg.Target, hall.Id), (ResultArg.Building, hall.Kind));
            var (seconds, diminished) = NextBoost(v, Pace.ResearchDivisor);
            double points = seconds * rate;
            v.ResearchPoints += points;
            return ActionResult.Ok(ResultCode.Boosted, hall.Id, (ResultArg.Seconds, ActionResult.Number(seconds)), (ResultArg.Points, ActionResult.Number(points)), (ResultArg.Diminished, diminished ? "true" : "false"), (ResultArg.Kind, "research"), (ResultArg.Target, hall.Id));
        }

        /// <summary>"Prefer for work": the cat is picked first for new jobs. The state is <see cref="Cat.Boosted"/>.</summary>
        private ActionResult HandleBoostCat(GameAction action, Cat cat)
        {
            if (cat == null)
                return ActionResult.Fail(ResultCode.CatRequired, "Living cat required", (ResultArg.Target, action.CatId ?? ""));
            cat.Boosted = action.Enabled;
            return ActionResult.Ok(ResultCode.Preferred, cat.Id, (ResultArg.Enabled, cat.Boosted ? "true" : "false"));
        }
        private ActionResult HandleSetCatLaborPreference(GameAction action, Cat cat)
        {
            if (cat == null)
                return ActionResult.Fail(ResultCode.CatRequired, "Living cat required", (ResultArg.Target, action.CatId ?? ""));
            if (!Catalog.Labors.Contains(action.Labor))
                return ActionResult.Fail(ResultCode.UnknownLabor, "Unknown kind of work", (ResultArg.Kind, action.Labor ?? ""));
            cat.Preferences.Remove(action.Labor);
            if (action.Enabled)
                cat.Preferences.Add(action.Labor);
            return ActionResult.Ok(ResultCode.PreferenceSet, cat.Id, (ResultArg.Kind, action.Labor), (ResultArg.Enabled, action.Enabled ? "true" : "false"));
        }
        private ActionResult HandleAssignWorker(Village village, GameAction action, Cat cat, Building building) => Assign(village, cat, building, action);
        private ActionResult HandleEditProductionQueue(Village village, GameAction action, string kind, Cat cat, Building building) => EditQueue(village, building, cat, action, kind == "editproductionworkslot");
        private ActionResult HandleHaulGatherSpot(Village village, GameAction action, Cat cat)
        {
            var source = village.Stockpiles.Find(s => s.Id == action.TargetId && (s.Goods.Any(g => g.Amount > 0) || village.Items.Any(i => i.LocationId == s.Id)));
            if (source == null)
                return ActionResult.Fail(ResultCode.NoSourceGoods, "No source goods", (ResultArg.Target, action.TargetId ?? ""));
            var refusal = MissingCat(action, cat) ?? OrderRefusal(village, cat);
            if (refusal != null)
                return refusal;
            var result = StartHaul(village, cat, source);
            return result.Success ? result.Coded(ResultCode.Queued) : result;
        }

        /// <summary>Every check runs before the first change, so a refused edit keeps officer automation.</summary>
        private ActionResult EditQueue(Village v, Building b, Cat c, GameAction a, bool exact)
        {
            if (b == null || !b.Completed)
                return ActionResult.Fail(ResultCode.StationRequired, "Completed station required", (ResultArg.Target, b?.Id ?? (string.IsNullOrEmpty(a.BuildingId) ? a.TargetId : a.BuildingId) ?? ""));
            if (exact && (c == null || !b.Slots.Any(s => s.CatId == c.Id)))
                return ActionResult.Fail(ResultCode.SlotRequired, "This cat does not work here", (ResultArg.Target, a.CatId ?? ""), (ResultArg.Building, b.Kind));
            var slot = exact ? b.Slots.Find(s => s.CatId == c.Id) : b.Slots.FirstOrDefault();
            var queue = slot?.Queue ?? b.Queue;
            string edit = (a.Edit ?? "").Replace("_", "").ToLowerInvariant();
            Recipe recipe = null;
            int to = a.Index;
            switch (edit)
            {
                case "add":
                case "addonce":
                    recipe = RecipeOf(a.RecipeId);
                    if (recipe == null || recipe.Building != b.Kind || !Catalog.RecipeAvailable(v, recipe))
                        return ActionResult.Fail(ResultCode.RecipeLocked, "Recipe locked or wrong station", (ResultArg.Target, a.RecipeId ?? ""), (ResultArg.Building, b.Kind));
                    if (queue.Count >= 64)
                        return ActionResult.Fail(ResultCode.QueueFull, "Queue limit reached", (ResultArg.Need, "64"), (ResultArg.Have, ActionResult.Number(queue.Count)));
                    break;
                case "setpaused":
                case "pause":
                    break;
                case "remove":
                case "setrepeat":
                case "repeat":
                case "move":
                    to = edit == "move" ? a.Index + (a.Direction < 0 ? -1 : 1) : a.Index;
                    if (a.Index < 0 || a.Index >= queue.Count || to < 0 || to >= queue.Count)
                        return ActionResult.Fail(ResultCode.QueueIndex, edit == "move" ? "Queue edge" : "Queue index out of range", (ResultArg.Target, ActionResult.Number(edit == "move" ? to : a.Index)), (ResultArg.Have, ActionResult.Number(queue.Count)));
                    break;
                default:
                    return ActionResult.Fail(ResultCode.UnknownEdit, "Unknown queue edit", (ResultArg.Kind, a.Edit ?? ""));
            }
            if (slot != null)
                slot.AutomatedBy = "";
            foreach (var entry in queue)
                entry.AutomatedBy = "";
            if (recipe != null)
                // Demo stations keep producing: an added recipe repeats unless the player adds it once.
                queue.Add(new QueueEntry { RecipeId = recipe.Id, Repeat = edit != "addonce" && (a.Repeat || Pace.Profile == Pacing.DemoProfile) });
            else if (edit == "setpaused" || edit == "pause")
            {
                if (slot != null)
                    slot.Paused = a.Enabled;
                else
                    b.Paused = a.Enabled;
            }
            else if (edit == "remove")
                queue.RemoveAt(a.Index);
            else if (edit == "setrepeat" || edit == "repeat")
                queue[a.Index].Repeat = a.Enabled;
            else
            {
                var q = queue[a.Index];
                queue.RemoveAt(a.Index);
                queue.Insert(to, q);
            }
            if (slot != null && b.Slots.IndexOf(slot) == 0)
                b.Queue = slot.Queue;
            return ActionResult.Ok(ResultCode.QueueUpdated, b.Id, (ResultArg.Kind, edit), (ResultArg.Target, slot?.CatId ?? ""));
        }

        /// <summary>The kind of the active job a reassignment would interrupt, or null.</summary>
        private static string InterruptedJob(Village v, Cat c) => v.Jobs.Find(j => j.Id == c.JobId && !j.Completed)?.Kind;

        private bool DrivingMidRoute(Village v, Cat c) => v.Routes.Any(r => r.CatId == c.Id && (r.Mode == "shipping" && r.PathIndex > 0 || v.Vehicles.Any(vehicle => vehicle.Id == r.VehicleId && vehicle.HasContinuousPosition && Math.Abs(vehicle.X - vehicle.Position.X) + Math.Abs(vehicle.Z - vehicle.Position.Z) > 1e-9)));

        private ActionResult Assign(Village v, Cat c, Building b, GameAction a)
        {
            if (c == null)
                return ActionResult.Fail(ResultCode.CatRequired, "Available living cat required", (ResultArg.Target, a.CatId ?? ""));
            var refusal = OrderRefusal(v, c);
            if (refusal != null)
                return refusal;
            if (DrivingMidRoute(v, c))
                return ActionResult.Fail(ResultCode.TransportReturning, "Return the vehicle to a safe stopping point before reassigning its driver", (ResultArg.Target, c.Id));
            string target = !string.IsNullOrEmpty(a.BuildingId) ? a.BuildingId : a.TargetId ?? "";
            string interrupted = InterruptedJob(v, c);
            if (b == null && target != "")
            {
                var farm = v.Farms.Find(f => f.Id == target);
                if (farm == null)
                    return ActionResult.Fail(ResultCode.UnknownTarget, "That workplace no longer exists", (ResultArg.Target, target));
                var previous = v.Cats.Find(x => x.Id == farm.WorkerId);
                if (previous != null && previous.Id != c.Id)
                    CancelWork(v, previous);
                CancelWork(v, c);
                farm.WorkerId = c.Id;
                c.BuildingId = farm.Id;
                return Assigned(c, farm.Id, "farm", interrupted);
            }
            if (b == null)
            {
                // Legacy stand-down: AssignWorker without a target releases the cat from all work.
                CancelWork(v, c);
                return interrupted == null ? ActionResult.Ok(ResultCode.Released, c.Id) : ActionResult.Ok(ResultCode.Released, c.Id, (ResultArg.Interrupted, interrupted));
            }
            // Demo worlds refuse phantom workers; classic worlds (server, mechanics fixtures that park a cat on a
            // den) keep accepting any building.
            if (!IsWorkplace(b) && Pace.Profile == Pacing.DemoProfile)
                return ActionResult.Fail(ResultCode.NotAWorkplace, "Cats cannot work at this building", (ResultArg.Target, b.Id), (ResultArg.Building, b.Kind));
            if (!b.Completed)
            {
                if (v.Jobs.Any(j => !j.Completed && j.TargetId == b.Id))
                    return ActionResult.Fail(ResultCode.ScaffoldHasBuilder, "Scaffold already has a builder", (ResultArg.Target, b.Id), (ResultArg.Building, b.Kind));
                CancelWork(v, c);
                StartJob(v, c, new Job { Kind = JobKind.Build, TargetId = b.Id, Position = b.Position, RequiredWork = b.RequiredWork, Manual = true });
                return Assigned(c, b.Id, b.Kind, interrupted);
            }
            int positions = (int)Catalog.BuildingEffect(v, b.Kind, "worker_slots", 1);
            if (b.Slots.Count(s => s.CatId != "" && s.CatId != c.Id) >= positions)
                return ActionResult.Fail(ResultCode.AllSlotsStaffed, "All work positions staffed", (ResultArg.Target, b.Id), (ResultArg.Building, b.Kind), (ResultArg.Need, ActionResult.Number(positions)));
            CancelWork(v, c);
            var vacancy = b.Slots.FirstOrDefault(s => s.CatId == "");
            if (vacancy != null)
            {
                vacancy.CatId = c.Id;
                vacancy.AutomatedBy = "";
            }
            else
                b.Slots.Add(new WorkSlot { CatId = c.Id, Queue = b.Slots.Count == 0 ? b.Queue : new List<QueueEntry>() });
            if (b.WorkerId == "")
                b.WorkerId = c.Id;
            else
                b.ExtraWorkerIds.Add(c.Id);
            c.BuildingId = b.Id;
            if (b.Kind == "field")
            {
                var plot = v.Farms.FirstOrDefault(f => f.WorkerId == "");
                if (plot != null)
                    plot.WorkerId = c.Id;
            }
            return Assigned(c, b.Id, b.Kind, interrupted);
        }

        private static ActionResult Assigned(Cat c, string target, string kind, string interrupted) => interrupted == null
            ? ActionResult.Ok(ResultCode.Assigned, c.Id, (ResultArg.Target, target), (ResultArg.Building, kind))
            : ActionResult.Ok(ResultCode.Assigned, c.Id, (ResultArg.Target, target), (ResultArg.Building, kind), (ResultArg.Interrupted, interrupted));

        /// <summary>Releases a cat from its workplace (station, farm or transport route) and nothing else.</summary>
        private ActionResult HandleReleaseWorker(Village v, GameAction a, Cat c)
        {
            if (c == null)
                return ActionResult.Fail(ResultCode.CatRequired, "Living cat required", (ResultArg.Target, a.CatId ?? ""));
            if (c.BuildingId == "")
                return ActionResult.Fail(ResultCode.NotAssigned, "This cat has no workplace", (ResultArg.Target, c.Id));
            if (c.ControlledBy != "")
                return ActionResult.Fail(ResultCode.CatControlled, "Release direct control first", (ResultArg.Target, c.Id));
            if (IsDungeonExplorer(c))
                return ActionResult.Fail(ResultCode.ExplorerAway, "Recall the explorer and wait for its return", (ResultArg.Target, c.Id));
            if (DrivingMidRoute(v, c))
                return ActionResult.Fail(ResultCode.TransportReturning, "Return the vehicle to a safe stopping point before releasing its driver", (ResultArg.Target, c.Id));
            string workplace = c.BuildingId;
            string kind = v.Buildings.Find(x => x.Id == workplace)?.Kind ?? (v.Farms.Any(f => f.Id == workplace) ? "farm" : v.Routes.Any(r => r.Id == workplace) ? "route" : "");
            string interrupted = InterruptedJob(v, c);
            CancelWork(v, c);
            return interrupted == null
                ? ActionResult.Ok(ResultCode.Released, c.Id, (ResultArg.Target, workplace), (ResultArg.Building, kind))
                : ActionResult.Ok(ResultCode.Released, c.Id, (ResultArg.Target, workplace), (ResultArg.Building, kind), (ResultArg.Interrupted, interrupted));
        }
    }
}
