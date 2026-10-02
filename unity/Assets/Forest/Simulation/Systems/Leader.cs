using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        // PACING-HANDOFF LeaderStockTarget: logs and stone the Leader keeps gathered while no Forester holds office.
        private const double LeaderStockTarget = 60;
        // PACING-HANDOFF LeaderGathersPerResource: concurrent Leader gather jobs per resource.
        private const int LeaderGathersPerResource = 2;
        // PACING-HANDOFF EmergencyCooldownSeconds: at most one Emergency pre-emption per job kind in this window.
        private const double EmergencyCooldownSeconds = 60;
        // PACING-HANDOFF EmergencyProtectedProgress: build, expand and production work past this share is never pre-empted.
        private const double EmergencyProtectedProgress = 0.5;

        private bool HasOfficer(Village v, string role) => v.Officers.Any(o => o.Role == role && v.Cats.Any(c => c.Id == o.CatId && c.Alive));

        /// <summary>
        /// Pre-empts one cat for physical hunting or water when requests fail: only when a source is
        /// reachable, preferring idle, hauling or scouting cats, never late build or production work, never
        /// a cat carrying its meal, at most once per kind per cooldown.
        /// </summary>
        private void Emergency(Village v, string kind)
        {
            if (v.Jobs.Any(j => j.Kind == kind && !j.Completed))
                return;
            var last = v.EmergencyAt.Find(s => s.Resource == kind);
            if (last != null && TimeSeconds < last.Amount + EmergencyCooldownSeconds)
                return;
            var candidates = v.Cats.Where(c => c.Alive && c.ControlledBy == "" && !IsDungeonExplorer(c) && c.Position.Level == 0 && c.AgeHours >= Pace.WorkerMinAgeHours && c.Migration != "arriving" && c.Migration != "departing" && EmergencyMayTake(v, c))
                .OrderBy(c => EmergencyRank(v, c)).ThenByDescending(c => c.Health + c.Thirst + c.Hunger).ThenBy(c => c.Id, StringComparer.Ordinal);
            // Path searches are bounded: only the best few candidates are tried.
            foreach (var worker in candidates.Take(3))
            {
                // Dry run first: a cat is only taken from its work when the new job can actually start.
                if (!EmergencySourceReachable(v, kind, worker))
                    continue;
                CancelWork(v, worker);
                if (last == null)
                    v.EmergencyAt.Add(last = new Stack(kind, 0));
                last.Amount = TimeSeconds;
                var result = Request(v, kind, worker, false);
                if (result.Success)
                    Note(v, "emergency", worker.Name + " preempted work for physical " + kind, worker.Id);
                return;
            }
        }
        private bool EmergencyMayTake(Village v, Cat c)
        {
            var job = v.Jobs.Find(j => j.Id == c.JobId && !j.Completed);
            if (v.Jobs.Any(j => j.CatId == c.Id && !j.Completed && (j.Kind == JobKind.Hunt || j.Kind == JobKind.Water)))
                return false;
            if (job != null && (job.Kind == JobKind.Build || job.Kind == JobKind.Expand || job.Kind == JobKind.Production) && job.Progress >= job.RequiredWork * EmergencyProtectedProgress)
                return false;
            // A cat on a need trip is only taken while it carries nothing and its need cannot be met from storage.
            if (c.Goal.StartsWith(GoalKey.NeedPrefix, StringComparison.Ordinal))
                return c.Cargo.Count == 0 && NeedSupplyMissing(v, c);
            return true;
        }
        private bool NeedSupplyMissing(Village v, Cat c)
        {
            string[] choices = c.Goal == GoalKey.NeedDrink ? new[] { "brew", "water" } : c.Goal == GoalKey.NeedEat ? new[] { "fish", "preserves", "food" } : c.Goal == GoalKey.NeedHeal ? new[] { "medicine", "herbs" } : null;
            if (choices == null)
                return false;
            return !choices.Any(resource => v.Stockpiles.Any(s => s.Kind == "storage" && Amount(s.Goods, resource) - Reservations.Where(r => r.PileId == s.Id && r.Resource == resource).Sum(r => r.Amount) >= NeedServingSize(c, resource)));
        }
        /// <summary>0: idle, hauling or scouting; 1: other work; 2: a stalled need trip.</summary>
        private int EmergencyRank(Village v, Cat c)
        {
            if (c.Goal.StartsWith(GoalKey.NeedPrefix, StringComparison.Ordinal))
                return 2;
            var job = v.Jobs.Find(j => j.Id == c.JobId && !j.Completed);
            if (job == null)
                return c.BuildingId == "" ? 0 : 1;
            return job.Kind == JobKind.Haul || job.Kind == JobKind.Scout ? 0 : 1;
        }
        /// <summary>The source search of the gather request, without starting anything.</summary>
        private bool EmergencySourceReachable(Village v, string kind, Cat c)
        {
            if (kind == JobKind.Water)
                return v.Known.Select(TileAt).Where(t => t.Water).SelectMany(t => Neighbors(t.Position)).Any(p => Walkable(p) && Path(c.Position, p) != null);
            if (kind == JobKind.Hunt)
                return v.Known.Select(TileAt).Where(t => SourceAmount(t, "food") > 0 && Walkable(v, t.Position) && !v.Jobs.Any(j => !j.Completed && j.Kind == JobKind.Hunt && j.Position.Equals(t.Position)))
                    .OrderBy(t => Int2.Distance(c.Position, t.Position)).Any(t => Path(c.Position, t.Position, v) != null);
            return false;
        }

        /// <summary>The leader's standing orders: food, water and scouting, officer duties, the founding fallback, housing, and the daily study.</summary>
        private void Direct(Village v)
        {
            CountStockForLeader(v);
            int alive = v.Cats.Count(c => c.Alive), hunts = (int)Math.Ceiling(alive * Pace.HuntCrewPerGroup / Pace.CrewGroupSize), waters = (int)Math.Ceiling(alive * Pace.WaterCrewPerGroup / Pace.CrewGroupSize);
            if (Total(v, "food") + Total(v, "fish") + Total(v, "preserves") < alive * Pace.FoodReservePerCat && v.Jobs.Count(j => j.Kind == "hunt" && !j.Completed) < hunts)
            {
                var result = Request(v, "hunt", null, false);
                if (!result.Success && Total(v, "food") + Total(v, "fish") + Total(v, "preserves") < alive * Pace.EmergencyStockPerCat)
                    Emergency(v, "hunt");
            }
            if (Total(v, "water") < alive * Pace.WaterReservePerCat && v.Jobs.Count(j => j.Kind == "water" && !j.Completed) < waters)
            {
                var result = Request(v, "water", null, false);
                if (!result.Success && Total(v, "water") < alive * Pace.EmergencyStockPerCat)
                    Emergency(v, "water");
            }
            if (!v.Jobs.Any(j => j.Kind == "scout" && !j.Completed) && ((long)TimeSeconds % Pace.ScoutIntervalSeconds == 0 || !v.Known.Any(p => TileAt(p).Resource == "logs")))
                Request(v, "scout", null, false, "logs");
            if (HasOfficer(v, "forester"))
            {
                foreach (var resource in new[] { "logs", "stone" })
                    if (Total(v, resource) < Pace.ForesterStockTarget && !v.Jobs.Any(j => j.Resource == resource && !j.Completed))
                        MarkOfficerJob(v, Request(v, resource == "stone" ? "quarry" : "logs", null, false), "forester");
                if ((long)TimeSeconds % Pace.ReplantIntervalSeconds == 0)
                    MarkOfficerJob(v, Request(v, "replant", null, false), "forester");
            }
            if (HasOfficer(v, "farmer"))
            {
                if (Total(v, "fibre") < Pace.FibreStockTarget && !v.Jobs.Any(j => j.Kind == "fibre" && !j.Completed))
                    MarkOfficerJob(v, Request(v, "fibre", null, false), "farmer");
                foreach (var f in v.Farms.Where(f => f.WorkerId == ""))
                {
                    var c = AvailableCat(v, "farm");
                    if (c != null)
                    {
                        f.WorkerId = c.Id;
                        f.AutomatedBy = "farmer";
                        c.BuildingId = f.Id;
                    }
                }
            }
            if (HasOfficer(v, "steward"))
            {
                foreach (var pile in v.Stockpiles.Where(s => new[] { "gather", "spill", "farm_output", "fishing" }.Contains(s.Kind) && (s.Goods.Count > 0 || v.Items.Any(i => i.LocationId == s.Id))).ToArray())
                {
                    if (!v.Jobs.Any(j => j.SourceId == pile.Id && !j.Completed))
                        MarkOfficerJob(v, StartHaul(v, null, pile), "steward");
                }
            }
            AutoAppointOffices(v);
            StaffOfficers(v);
            LeaderFallback(v);
            PlanHousing(v);
            if (HasOfficer(v, "loremaster") && v.Blessings < Pace.LoremasterOfferingBlessingCap && Total(v, "materials") > Pace.LoremasterOfferingMaterialsMin && !v.Jobs.Any(j => j.Kind == "offering" && !j.Completed))
                MarkOfficerJob(v, CreateInputJob(v, null, "offering", v.Center, "materials", Pace.LoremasterOfferingMaterials, Pace.LoremasterOfferingWork, ""), "loremaster");
            if (TimeSeconds - v.LastLeaderResearch >= Pace.LeaderStudyIntervalSeconds && v.Cats.Any(c => c.Id == v.LeaderId && c.Alive))
            {
                var node = Catalog.Research.Where(n => !v.Research.Contains(n.Id) && n.Cost <= v.ResearchPoints && n.Prerequisites.All(v.Research.Contains)).OrderBy(n => n.LeaderPriority).ThenBy(n => n.Id, StringComparer.Ordinal).FirstOrDefault();
                if (node != null && Research(v, node.Id, false).Success)
                    v.LastLeaderResearch = TimeSeconds;
            }
            ForgetVacatedQueueDefaults(v);
        }

        /// <summary>Stations of the founding blueprint: the Research Hut and every station with a founding recipe.</summary>
        private static bool FoundingStation(string kind) => kind == "research_hut" || Catalog.Recipes.Any(r => r.Building == kind && r.Founding);

        /// <summary>
        /// While an office is vacant, the Leader runs its founding stations: one scholar per Research Hut,
        /// founding workshops on a repeating recipe, logs and stone gathered to a target, and spills
        /// re-hauled. All of it only while idle cats exceed the reserve.
        /// </summary>
        private void LeaderFallback(Village v)
        {
            foreach (var b in v.Buildings.Where(b => b.Completed && FoundingStation(b.Kind)).OrderBy(b => b.Kind == "research_hut" ? 0 : 1).ToArray())
            {
                if (HasOfficer(v, StationOfficer(b.Kind)) || b.Paused)
                    continue;
                if (b.Kind == "research_hut")
                {
                    if (!b.Slots.Any(s => s.CatId != ""))
                        StaffStation(v, b, LeaderRole, 1, "research");
                    continue;
                }
                foreach (var slot in b.Slots.Where(s => s.AutomatedBy == LeaderRole && s.CatId != "" && !s.Paused && AutomatedSlotIdle(v, s)).ToArray())
                    if (LeaderSlotDone(v, slot))
                        ReleaseAutomatedWorker(v, slot);
                var recipe = AutomatedRecipe(v, b);
                if (recipe == null)
                    continue;
                StaffStation(v, b, LeaderRole, WorkerSlots(v, b), recipe.Labor);
                foreach (var slot in b.Slots.Where(s => s.AutomatedBy == LeaderRole && s.CatId != "" && s.Queue.Count == 0 && !s.Paused))
                    slot.Queue.Add(new QueueEntry { RecipeId = recipe.Id, Repeat = true, AutomatedBy = LeaderRole });
            }
            if (!HasOfficer(v, "forester"))
                foreach (var (resource, kind) in new[] { ("logs", JobKind.Logs), ("stone", JobKind.Quarry) })
                    if (IdleCats(v) > IdleReserve(v) && Total(v, resource) < LeaderStockTarget && v.Jobs.Count(j => !j.Completed && j.Kind == kind && j.Resource == resource) < LeaderGathersPerResource)
                        MarkOfficerJob(v, Request(v, kind, null, false, resource == "stone" ? "stone" : ""), LeaderRole);
            if (!HasOfficer(v, "steward"))
                foreach (var pile in v.Stockpiles.Where(s => s.Kind == "spill" && (s.Goods.Count > 0 || v.Items.Any(i => i.LocationId == s.Id))).ToArray())
                    if (IdleCats(v) > IdleReserve(v) && !v.Jobs.Any(j => j.SourceId == pile.Id && !j.Completed))
                        MarkOfficerJob(v, StartHaul(v, null, pile), LeaderRole);
        }
        /// <summary>A Leader-run slot is done when its recipe has nothing left to make or cannot start.</summary>
        private bool LeaderSlotDone(Village v, WorkSlot slot)
        {
            if (slot.Queue.Count == 0)
                return true;
            if (slot.BlockedReason == "missing_input" || slot.BlockedReason == "construction_reserve" || slot.BlockedReason == "recipe_locked")
                return true;
            var head = slot.Queue[0];
            return head.AutomatedBy == LeaderRole && RecipeDeficit(v, RecipeOf(head.RecipeId) ?? new Recipe()) <= 0;
        }
        /// <summary>Drops the queue-default marks of slots whose worker left, so the next assignment gets a default again.</summary>
        private void ForgetVacatedQueueDefaults(Village v)
        {
            if (v.QueueDefaultsApplied.Count == 0)
                return;
            var staffed = new HashSet<string>(v.Buildings.SelectMany(b => b.Slots.Where(s => s.CatId != "").Select(s => QueueDefaultKey(b, s))));
            v.QueueDefaultsApplied.RemoveAll(key => !staffed.Contains(key));
        }
        private static string QueueDefaultKey(Building b, WorkSlot slot) => b.Id + "|" + slot.CatId;
    }
}
