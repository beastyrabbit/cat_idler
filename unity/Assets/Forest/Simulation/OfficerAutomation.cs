using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private double HousingSitingRetrySeconds => Pace.HousingSitingRetrySeconds;
        private const string LeaderRole = "leader";

        // Indexed like Catalog.Roles: steward, accountant, forester, farmer, captain, loremaster, cloth_leader.
        private static readonly string[] OfficeLabors = { "haul", "process", "woodcut", "farm", "fight", "research", "textile" };
        private static readonly string[] OfficeStats = { "leadership", "vision", "building", "hunting", "attack", "medicine", "building" };
        private static readonly string[] OfficeTitles = { "Steward", "Accountant", "Forester", "Farmer", "Captain", "Loremaster", "Cloth Leader" };
        // Items that do something (WorkRate, defense); every other item kind is decorative.
        private static readonly string[] FunctionalItemKinds = { "tool", "weapon", "armor" };

        [NonSerialized] private long housingSitingScans;

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
                    s.Queue.RemoveAll(q => q.AutomatedBy == role);
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
        private static string StationOfficer(string kind) => new[] { "wood_cutter", "stone_prep", "woodworking", "sawmill" }.Contains(kind) ? "forester" : new[] { "clothier", "tannery" }.Contains(kind) ? "cloth_leader" : new[] { "smithy", "smelter", "barracks" }.Contains(kind) ? "captain" : new[] { "research_hut", "school" }.Contains(kind) ? "loremaster" : kind == "mill" ? "farmer" : kind == "accounting_tent" ? "accountant" : "steward";
        private double RecipeDeficit(Village v, Recipe r)
        {
            if (r.ItemKind != "")
                return Math.Max(0, v.Cats.Count(c => c.Alive) * Pace.OfficerItemsPerCat - v.Items.Count(i => i.Kind == r.ItemKind && i.Material == r.Material && i.Condition > 0));
            return r.Outputs.Count == 0 ? 0 : r.Outputs.Max(s => Math.Max(0, (s.Resource == "food" ? v.Cats.Count(c => c.Alive) * Pace.OfficerFoodPerCat : Pace.OfficerStockTarget) - Total(v, s.Resource)));
        }

        private static bool HasRecipes(string kind) => Catalog.Recipes.Any(r => r.Building == kind);
        private int WorkerSlots(Village v, Building b) => Math.Max(1, DemoContent.SlotsFor(this, v, b.Kind));
        /// <summary>Unlocked recipes of a station kind, with any demo overrides applied.</summary>
        private IEnumerable<Recipe> StationRecipes(Village v, string kind) => DemoContent.Recipes(this).Where(r => r.Building == kind).Select(r => RecipeOf(r.Id)).Where(r => r != null && Catalog.RecipeAvailable(this, v, r));
        private bool InputsReady(Village v, Recipe r, Building b) => r.Inputs.All(input => Available(v, input.Resource) + Amount(b.Inputs, input.Resource) >= RecipeInput(v, r, input.Amount));
        /// <summary>Output per unit of input: resource output, or item quality, divided by the inputs.</summary>
        private double OutputPerInput(Village v, Recipe r)
        {
            double input = r.Inputs.Sum(s => RecipeInput(v, r, s.Amount));
            double output = r.ItemKind != "" ? r.Quality : r.Outputs.Sum(s => RecipeOutput(v, r, s.Amount));
            return input > 0 ? output / input : output;
        }
        /// <summary>Automation never makes decorative items or refined goods, and preserves food only from a surplus.</summary>
        private bool AutomationMayMake(Village v, Recipe r)
        {
            if (r.ItemKind != "" && Array.IndexOf(FunctionalItemKinds, r.ItemKind) < 0)
                return false;
            if (r.Outputs.Any(s => s.Resource == "refined"))
                return false;
            if (r.Outputs.Any(s => s.Resource == "preserves") && r.Inputs.Any(s => s.Resource == "food") && Total(v, "food") <= v.Cats.Count(c => c.Alive) * Pace.OfficerFoodPerCat)
                return false;
            return true;
        }
        /// <summary>Construction stock automation leaves alone: enough timber and blocks for the next Den.</summary>
        private double ConstructionReserve(Village v, string resource)
        {
            if (resource != "planks" && resource != "lumber" && resource != "blocks")
                return 0;
            int dens = v.Buildings.Count(b => b.Kind == "den");
            return resource == "blocks" ? Pace.ScaffoldBlocksBase + dens * Pace.ScaffoldBlocksPerExisting : Pace.ScaffoldTimberBase + dens * Pace.ScaffoldTimberPerExisting;
        }
        private bool KeepsConstructionReserve(Village v, Recipe r, Building b) => r.Inputs.All(s =>
        {
            double reserve = ConstructionReserve(v, s.Resource);
            return reserve <= 0 || Available(v, s.Resource) - Math.Max(0, RecipeInput(v, r, s.Amount) - Amount(b.Inputs, s.Resource)) + 1e-9 >= reserve;
        });
        /// <summary>
        /// The recipe an officer or the Leader makes next at a station: something in deficit that can
        /// start now, ordered by deficit, then output per input, then id.
        /// </summary>
        private Recipe AutomatedRecipe(Village v, Building b)
        {
            Recipe best = null;
            double bestDeficit = 0, bestYield = 0;
            foreach (var r in StationRecipes(v, b.Kind))
            {
                if (!AutomationMayMake(v, r) || !InputsReady(v, r, b) || !KeepsConstructionReserve(v, r, b))
                    continue;
                double deficit = RecipeDeficit(v, r);
                if (deficit <= 0)
                    continue;
                double yield = OutputPerInput(v, r);
                if (best == null || deficit > bestDeficit || deficit == bestDeficit && (yield > bestYield || yield == bestYield && string.CompareOrdinal(r.Id, best.Id) < 0))
                {
                    best = r;
                    bestDeficit = deficit;
                    bestYield = yield;
                }
            }
            return best;
        }
        /// <summary>The repeating recipe a player-assigned worker starts when its queue is empty.</summary>
        private Recipe DefaultRecipe(Village v, Building b) => StationRecipes(v, b.Kind)
            .OrderByDescending(r => AutomationMayMake(v, r)).ThenByDescending(r => InputsReady(v, r, b)).ThenByDescending(r => RecipeDeficit(v, r))
            .ThenByDescending(r => OutputPerInput(v, r)).ThenBy(r => r.Id, StringComparer.Ordinal).FirstOrDefault();

        private int IdleCats(Village v) => v.Cats.Count(c => c.Alive && c.JobId == "" && c.BuildingId == "" && c.ControlledBy == "");
        /// <summary>Idle cats automation leaves free for the player and for survival work.</summary>
        private int IdleReserve(Village v) => Math.Max(Pace.OfficerReserveMin, (int)Math.Ceiling(v.Cats.Count(c => c.Alive) * Pace.OfficerReservePerGroup / Pace.CrewGroupSize));
        /// <summary>Staffs free work positions with the best cat for <paramref name="labor"/> while idle cats exceed the reserve.</summary>
        private void StaffStation(Village v, Building b, string role, int slots, string labor)
        {
            int reserve = IdleReserve(v);
            while (b.Slots.Count(s => s.CatId != "") < slots && IdleCats(v) > reserve)
            {
                var worker = AvailableCat(v, labor);
                if (worker == null || !Assign(v, worker, b, new GameAction()).Success)
                    return;
                b.Slots.Find(s => s.CatId == worker.Id).AutomatedBy = role;
            }
        }
        /// <summary>Sends an automated worker with nothing to make back to the colony; its automated queue entries go with it.</summary>
        private void ReleaseAutomatedWorker(Village v, WorkSlot slot)
        {
            string role = slot.AutomatedBy;
            slot.Queue.RemoveAll(q => q.AutomatedBy == role);
            var c = v.Cats.Find(x => x.Id == slot.CatId);
            if (c != null)
                CancelWork(v, c);
            else
                slot.CatId = "";
            slot.AutomatedBy = "";
            slot.BlockedReason = "";
        }
        private bool AutomatedSlotIdle(Village v, WorkSlot slot)
        {
            var c = v.Cats.Find(x => x.Id == slot.CatId);
            return c == null || c.JobId == "" && !c.Goal.StartsWith(GoalKey.NeedPrefix, StringComparison.Ordinal);
        }

        private void StaffOfficers(Village v)
        {
            foreach (var b in v.Buildings.Where(b => b.Completed).ToArray())
            {
                string officer = StationOfficer(b.Kind);
                bool scholarly = b.Kind == "research_hut" || b.Kind == "school", recipes = HasRecipes(b.Kind);
                bool productive = recipes || scholarly || b.Kind == "accounting_tent" || b.Kind == "mouse_farm";
                if (!productive || !HasOfficer(v, officer))
                    continue;
                // An appointed officer takes over what the Leader staffed while the office was vacant.
                foreach (var slot in b.Slots.Where(s => s.AutomatedBy == LeaderRole))
                {
                    slot.AutomatedBy = officer;
                    slot.Queue.RemoveAll(q => q.AutomatedBy == LeaderRole);
                }
                if (recipes)
                    foreach (var slot in b.Slots.Where(s => s.AutomatedBy == officer && s.CatId != "" && !s.Paused && AutomatedSlotIdle(v, s)).ToArray())
                        if (slot.Queue.Count == 0 && AutomatedRecipe(v, b) == null || slot.BlockedReason == "missing_input" || slot.BlockedReason == "construction_reserve")
                            ReleaseAutomatedWorker(v, slot);
                var recipe = recipes ? AutomatedRecipe(v, b) : null;
                if (b.Paused || recipes && recipe == null)
                    continue;
                StaffStation(v, b, officer, WorkerSlots(v, b), scholarly ? "research" : recipe?.Labor ?? "process");
                foreach (var slot in b.Slots.Where(s => s.AutomatedBy == officer && s.CatId != "" && s.Queue.Count == 0 && !s.Paused))
                {
                    var next = recipes ? AutomatedRecipe(v, b) : null;
                    if (next != null)
                        slot.Queue.Add(new QueueEntry { RecipeId = next.Id, AutomatedBy = officer });
                }
            }
        }

        /// <summary>
        /// Fills an office the first time its study and building are complete, with the best-matching
        /// resident adult. A vacated office is never refilled automatically, so the vacate action undoes it.
        /// </summary>
        private void AutoAppointOffices(Village v)
        {
            foreach (var held in v.Officers)
                if (!v.OfficesFilled.Contains(held.Role))
                    v.OfficesFilled.Add(held.Role);
            for (int i = 0; i < Catalog.Roles.Length; i++)
            {
                string role = Catalog.Roles[i];
                if (v.OfficesFilled.Contains(role) || !v.Research.Contains(Catalog.RoleStudies[i]) || !v.Buildings.Any(b => b.Kind == Catalog.RoleBuildings[i] && b.Completed))
                    continue;
                int office = i;
                var cat = v.Cats.Where(c => EligibleForAutomaticOffice(v, c)).OrderByDescending(c => OfficeScore(c, office)).ThenBy(c => c.Id, StringComparer.Ordinal).FirstOrDefault();
                if (cat == null)
                    continue;
                v.Officers.Add(new Officer { Role = role, CatId = cat.Id });
                cat.OfficerRole = role;
                v.OfficesFilled.Add(role);
                Note(v, EventKind.Appointment, cat.Name + " was appointed " + OfficeTitles[i] + ".", cat.Id);
            }
        }
        private bool EligibleForAutomaticOffice(Village v, Cat c) => c.Alive && c.AgeHours >= Pace.KittenUntilHours && c.Migration == "resident" && c.ControlledBy == "" && c.OfficerRole == "" && c.Id != v.LeaderId && !IsDungeonExplorer(c) && !v.Officers.Any(o => o.CatId == c.Id);
        private double OfficeScore(Cat c, int office) => (c.Preferences.Contains(OfficeLabors[office]) ? Pace.AssignPreferenceScore : 0) + Amount(c.Skills, OfficeLabors[office]) + Amount(c.Stats, OfficeStats[office]);

        private void PlanHousing(Village v)
        {
            int adults = v.Cats.Count(c => c.Alive), beds = v.Buildings.Count(b => b.Kind == "den" && b.Completed) * DenCapacity(v);
            if (TimeSeconds - v.FoundedAt < Pace.HousingPlanAfterSeconds || beds - adults - v.Cats.Count(c => c.PregnantUntil >= 0) > Pace.HousingSpareBeds || Total(v, "food") < adults * Pace.HousingFoodPerCat || Total(v, "water") < adults * Pace.HousingWaterPerCat || v.Buildings.Any(b => b.Kind == "den" && !b.Completed))
                return;
            // A scan that found no site is not repeated until the layout changes (or the retry time passes).
            string layout = LayoutSignature(v);
            if (layout == v.HousingSitingLayout && TimeSeconds < v.HousingSitingRetryAt)
                return;
            housingSitingScans++;
            // Connected roads and an occupancy grid once per scan, instead of once per candidate.
            var expansions = v.Jobs.Where(j => !j.Completed && j.Kind == JobKind.Expand && j.OriginalKind != JobKind.ExpandVillage).ToArray();
            var roads = ConnectedRoads(v, expansions.SelectMany(j => j.Path).ToHashSet(), expansions.Length > 0 ? v.Radius + 2 : 0);
            var known = new HashSet<Int2>(v.Known);
            var occupied = OccupiedTiles(v);
            foreach (var p in v.Known.OrderBy(p => Int2.Distance(v.Center, p)).ThenBy(p => p.Z).ThenBy(p => p.X))
            {
                if (!FootprintClear(p, 2, 2, known, occupied) || !FreeSite(v, p, 2, 2) || !EntranceCandidates(new Building { Position = p }).Any(roads.Contains))
                    continue;
                v.HousingSitingLayout = "";
                var result = Plan(v, "den", p, null);
                if (result.Success)
                    MarkOfficerJob(v, result, LeaderRole);
                return;
            }
            v.HousingSitingLayout = layout;
            v.HousingSitingRetryAt = TimeSeconds + HousingSitingRetrySeconds;
        }
        private static string LayoutSignature(Village v) => string.Join(",", new[]
        {
            v.Buildings.Count, v.Buildings.Count(b => b.Completed), v.Stockpiles.Count, v.Farms.Count, v.Known.Count, v.Radius, v.ClaimedTiles.Count,
            v.Jobs.Count(j => !j.Completed && (j.Kind == JobKind.Road || j.Kind == JobKind.Rail || j.Kind == JobKind.Expand))
        }.Select(n => n.ToString(CultureInfo.InvariantCulture)));
        /// <summary>Every tile under a building, stockpile or farm footprint.</summary>
        private static HashSet<Int2> OccupiedTiles(Village v)
        {
            var occupied = new HashSet<Int2>();
            void Mark(Int2 p, int width, int depth)
            {
                for (int x = 0; x < width; x++)
                    for (int z = 0; z < depth; z++)
                        occupied.Add(new Int2(p.X + x, p.Z + z, p.Level));
            }
            foreach (var b in v.Buildings)
                Mark(b.Position, b.Width, b.Depth);
            foreach (var s in v.Stockpiles)
                Mark(s.Position, s.Width, s.Depth);
            foreach (var f in v.Farms)
                Mark(f.Position, f.Width, f.Depth);
            return occupied;
        }
        /// <summary>Cheap prefilter for FreeSite: every footprint tile mapped and unoccupied.</summary>
        private static bool FootprintClear(Int2 p, int width, int depth, HashSet<Int2> known, HashSet<Int2> occupied)
        {
            for (int x = 0; x < width; x++)
                for (int z = 0; z < depth; z++)
                {
                    var at = new Int2(p.X + x, p.Z + z, p.Level);
                    if (!known.Contains(at) || occupied.Contains(at))
                        return false;
                }
            return true;
        }
    }
}
