using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private void StopOfficerWork(Village v, string role)
        {
            var workers = v.Buildings.SelectMany(b => b.Slots).Where(s => s.AutomatedBy == role).Select(s => s.CatId)
                .Concat(v.Farms.Where(f => f.AutomatedBy == role).Select(f => f.WorkerId))
                .Concat(v.Jobs.Where(j => !j.Completed && j.AutomatedBy == role).Select(j => j.CatId)).Where(id => id != "").Distinct().ToArray();
            foreach (var id in workers)
            {
                var c = v.Cats.Find(c => c.Id == id);
                if (c != null)
                    CancelWork(v, c);
            }
            foreach (var b in v.Buildings)
                foreach (var s in b.Slots)
                {
                    if (s.Queue.RemoveAll(q => q.AutomatedBy == role) > 0)
                        MirrorPrimaryQueue(b, s);
                    if (s.AutomatedBy == role)
                        s.AutomatedBy = "";
                }
            foreach (var f in v.Farms.Where(f => f.AutomatedBy == role))
                f.AutomatedBy = "";
        }
        private void MarkOfficerJob(Village v, ActionResult result, string role)
        {
            if (result.Success)
            {
                var job = v.Jobs.Find(j => j.Id == result.EntityId || j.TargetId == result.EntityId && !j.Completed);
                if (job != null)
                {
                    job.AutomatedBy = role;
                    job.Requester = role;
                    job.Manual = false;
                }
            }
        }
        private static string StationOfficer(string kind) => new[] { "wood_cutter", "stone_prep", "woodworking", "sawmill" }.Contains(kind) ? "forester" : new[] { "clothier", "tannery" }.Contains(kind) ? "cloth_leader" : new[] { "smithy", "smelter", "barracks" }.Contains(kind) ? "captain" : new[] { "research_hut", "school" }.Contains(kind) ? "loremaster" : kind == "mill" || kind == "mouse_farm" ? "farmer" : kind == "accounting_tent" ? "accountant" : "steward";
        private double RecipeDeficit(Village v, Recipe r)
        {
            if (r.ItemKind != "")
                return Math.Max(0, v.Cats.Count(c => c.Alive) * 0.5 - v.Items.Count(i => i.Kind == r.ItemKind && i.Material == r.Material && i.Condition > 0));
            return r.Outputs.Count == 0 ? 0 : r.Outputs.Max(s => Math.Max(0, (s.Resource == "food" ? v.Cats.Count(c => c.Alive) * 8 : 32) - Total(v, s.Resource)));
        }
        private bool OfficerRecipeFunded(Village v, Building b, Cat cat, Recipe recipe, Dictionary<string, bool> inputAccess)
        {
            if (recipe == null || !Catalog.RecipeAvailable(v, recipe) || recipe.Inputs.Any(input => Available(v, input.Resource) + Amount(b.Inputs, input.Resource) < RecipeInput(v, recipe, input.Amount)))
                return false;
            foreach (var input in recipe.Inputs)
            {
                double missing = Math.Max(0, RecipeInput(v, recipe, input.Amount) - Amount(b.Inputs, input.Resource));
                if (missing <= 1e-9)
                    continue;
                foreach (var pile in v.Stockpiles)
                {
                    double free = Amount(pile.Goods, input.Resource) - Reservations.Where(r => r.PileId == pile.Id && r.Resource == input.Resource).Sum(r => r.Amount);
                    if (free <= 0)
                        continue;
                    if (!inputAccess.TryGetValue(pile.Id, out bool reachable))
                        inputAccess[pile.Id] = reachable = ProductionInputAccess(v, pile, cat.Position, b.Position).HasValue;
                    if (reachable)
                        missing -= free;
                    if (missing <= 1e-9)
                        break;
                }
                if (missing > 1e-9)
                    return false;
            }
            return true;
        }
        private Recipe NextOfficerRecipe(Village v, Building b, Cat cat, Dictionary<string, bool> inputAccess) => Catalog.Recipes.Where(r => r.Building == b.Kind && OfficerRecipeFunded(v, b, cat, r, inputAccess))
            .OrderByDescending(r => RecipeDeficit(v, r)).ThenBy(r => r.Id, StringComparer.Ordinal).FirstOrDefault(r => RecipeDeficit(v, r) > 0);
        private static string StationLabor(string kind) => kind == "research_hut" || kind == "school" ? "research" : Catalog.Recipes.FirstOrDefault(r => r.Building == kind)?.Labor ?? "process";
        public sealed class CaptainReadiness
        {
            public int Target, Trained, Training;
            public string Blocker = "";
        }
        private bool AvailableForCaptainTraining(Village v, Cat c) => c.Alive && c.AgeHours >= 12 && FreeWorker(v, c)
            && c.Migration != "arriving" && c.Migration != "departing" && c.Health >= 50 && c.Hunger >= 35 && c.Thirst >= 35 && c.Rest >= 20
            && !c.Goal.StartsWith("need_", StringComparison.Ordinal);
        public CaptainReadiness CaptainPreparationStatus(Village v) => CaptainPreparationPlan(v, out _, out _);
        private CaptainReadiness CaptainPreparationPlan(Village v, out Cat worker, out Building station)
        {
            worker = null;
            station = null;
            var living = v.Cats.Where(c => c.Alive).ToArray();
            var trained = new HashSet<string>(living.Where(c => Amount(c.Skills, "fight") >= 3).Select(c => c.Id));
            var training = v.Jobs.Where(j => !j.Completed && j.Kind == "train" && living.Any(c => c.Id == j.CatId)).ToArray();
            var result = new CaptainReadiness
            {
                Target = Math.Max(1, (int)Math.Ceiling(living.Length / 15.0)),
                Trained = trained.Count,
                Training = training.Where(j => !trained.Contains(j.CatId)).Select(j => j.CatId).Distinct().Count()
            };
            if (!HasOfficer(v, "captain"))
                result.Blocker = "captain_unassigned";
            else if (v.Raids.Any(r => r.Health > 0))
                result.Blocker = "raid_active";
            else if (result.Trained >= result.Target)
                result.Blocker = "ready";
            else
            {
                var active = training.FirstOrDefault(j => j.AutomatedBy == "captain");
                if (active != null)
                {
                    var barracks = v.Buildings.Find(b => b.Id == active.TargetId && b.Kind == "barracks" && b.Completed);
                    result.Blocker = barracks == null ? "barracks_missing" : barracks.Paused ? "barracks_paused" : active.BlockedReason == "blocked_route" || active.BlockedReason == "training_barracks_unreachable" || living.Any(c => c.Id == active.CatId && c.BlockedReason == "blocked_route") ? "barracks_unreachable" : "training_active";
                }
                else if (result.Trained + result.Training >= result.Target)
                    result.Blocker = "training_active";
                else
                {
                    var sites = v.Buildings.Where(b => b.Kind == "barracks" && b.Completed).ToArray();
                    var available = living.Where(c => AvailableForCaptainTraining(v, c)).ToArray();
                    int reserve = Math.Max(2, (int)Math.Ceiling(living.Length * 2.0 / 15));
                    if (sites.Length == 0)
                        result.Blocker = "barracks_missing";
                    else if (sites.All(b => b.Paused))
                        result.Blocker = "barracks_paused";
                    else if (available.Length <= reserve)
                        result.Blocker = "idle_reserve";
                    else
                    {
                        var candidates = available.Where(c => !trained.Contains(c.Id)).OrderByDescending(c => (c.Preferences.Contains("fight") ? 100 : 0) + Amount(c.Skills, "fight")).ThenBy(c => c.Id, StringComparer.Ordinal).ToArray();
                        result.Blocker = candidates.Length == 0 ? "no_eligible_trainee" : "barracks_unreachable";
                        foreach (var candidate in candidates)
                        {
                            var reachable = sites.Where(b => !b.Paused && Walkable(v, b.Position)).OrderBy(b => Int2.Distance(candidate.Position, b.Position)).ThenBy(b => b.Id, StringComparer.Ordinal).FirstOrDefault(b => Path(candidate.Position, b.Position, v) != null);
                            if (reachable == null)
                                continue;
                            worker = candidate;
                            station = reachable;
                            result.Blocker = "";
                            break;
                        }
                    }
                }
            }
            return result;
        }
        private void PlanCaptainPreparation(Village v)
        {
            var readiness = CaptainPreparationPlan(v, out var worker, out var station);
            if (readiness.Blocker != "" || worker == null || station == null)
                return;
            var result = Request(v, "train", worker, false);
            if (!result.Success)
                return;
            MarkOfficerJob(v, result, "captain");
            var job = v.Jobs.Find(j => j.Id == result.EntityId);
            job.TargetId = station.Id;
            job.Position = station.Position;
        }
        private bool TrainingCanAdvance(Village v, Cat c, Job j)
        {
            var station = v.Buildings.Find(b => b.Kind == "barracks" && b.Completed && (j.TargetId == "" ? b.Position.Equals(j.Position) : b.Id == j.TargetId));
            if (station != null && j.TargetId == "")
                j.TargetId = station.Id;
            string blocker = station == null ? "training_barracks_missing" : station.Paused ? "training_barracks_paused" : !Walkable(v, station.Position) ? "training_barracks_unreachable" : "";
            if (blocker != "")
            {
                j.BlockedReason = c.BlockedReason = blocker;
                c.Goal = blocker == "training_barracks_paused" ? "training paused" : "training blocked";
                return false;
            }
            if (j.BlockedReason.StartsWith("training_barracks_", StringComparison.Ordinal))
                j.BlockedReason = "";
            if (c.BlockedReason.StartsWith("training_barracks_", StringComparison.Ordinal))
                c.BlockedReason = "";
            return true;
        }
        private void StaffOfficers(Village v, bool farmerOnly)
        {
            foreach (var b in v.Buildings.Where(b => b.Completed))
            {
                string officer = StationOfficer(b.Kind);
                if ((officer == "farmer") != farmerOnly)
                    continue;
                bool productive = Catalog.Recipes.Any(r => r.Building == b.Kind) || new[] { "research_hut", "school", "accounting_tent", "mouse_farm" }.Contains(b.Kind);
                if (!productive || !HasOfficer(v, officer))
                    continue;
                int reserve = Math.Max(2, (int)Math.Ceiling(v.Cats.Count(c => c.Alive) * 2.0 / 15));
                if (!b.Paused && b.Slots.Count(s => s.CatId != "") < (int)Catalog.BuildingEffect(v, b.Kind, "worker_slots", 1)
                    && (officer == "farmer" ? AvailableCats(v, StationLabor(b.Kind)).Count() : v.Cats.Count(c => c.Alive && c.JobId == "" && c.BuildingId == "" && c.ControlledBy == "")) > reserve)
                {
                    var worker = Walkable(v, b.Position) ? AvailableCats(v, StationLabor(b.Kind))
                        .FirstOrDefault(candidate => Path(candidate.Position, b.Position, v) != null) : null;
                    if (worker != null && Assign(v, worker, b, new GameAction(), officerAssignment: true).Success)
                        b.Slots.Find(s => s.CatId == worker.Id).AutomatedBy = officer;
                }
                foreach (var slot in b.Slots.Where(s => s.AutomatedBy == officer && s.CatId != "" && !s.Paused && !b.Paused))
                {
                    var cat = v.Cats.Find(c => c.Id == slot.CatId);
                    if (cat == null)
                        continue;
                    if (slot.Queue.Count > 0 && (cat.JobId != "" || slot.Queue.Count != 1 || slot.Queue[0].AutomatedBy != officer))
                        continue;
                    // Reuse each cat/pile/station route within this decision only. The
                    // next planning pass must observe closures and changed claims.
                    var inputAccess = new Dictionary<string, bool>(StringComparer.Ordinal);
                    bool stale = slot.Queue.Count == 1 && !OfficerRecipeFunded(v, b, cat, Catalog.Recipe(slot.Queue[0].RecipeId), inputAccess);
                    if (slot.Queue.Count > 0 && !stale)
                        continue;
                    var recipe = NextOfficerRecipe(v, b, cat, inputAccess);
                    if (recipe != null)
                    {
                        if (stale)
                            slot.Queue.RemoveAt(0);
                        slot.Queue.Add(new QueueEntry { RecipeId = recipe.Id, AutomatedBy = officer });
                        MirrorPrimaryQueue(b, slot);
                    }
                }
            }
        }
        private void PlanHousing(Village v)
        {
            int adults = v.Cats.Count(c => c.Alive), beds = v.Buildings.Count(b => b.Kind == "den" && b.Completed) * DenCapacity(v);
            if (TimeSeconds - v.FoundedAt < 6 * 3600 || beds - adults - v.Cats.Count(c => c.PregnantUntil >= 0) > 1 || Total(v, "food") < adults * 3 || Total(v, "water") < adults * 4 || v.Buildings.Any(b => b.Kind == "den" && !b.Completed))
                return;
            foreach (var p in v.Known.OrderBy(p => Int2.Distance(v.Center, p)).ThenBy(p => p.Z).ThenBy(p => p.X))
                if (FreeSite(v, p, 2, 2) && FreeBuildingEntrance(v, p).HasValue)
                {
                    var result = Plan(v, "den", p, null);
                    if (result.Success)
                        MarkOfficerJob(v, result, "leader");
                    return;
                }
        }
    }
}
