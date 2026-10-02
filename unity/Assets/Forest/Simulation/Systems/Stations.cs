using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        // BLOCKED-HANDOFF Blocked.ConstructionReserve: automated work waits so the next Den keeps its timber and blocks.
        private const string ConstructionReserveBlock = "construction_reserve";

        /// <summary>
        /// The first time a player-assigned worker is seen at a station with an empty queue, queue the
        /// station's default recipe on repeat. Once per assignment, so a queue the player empties stays
        /// empty; automated slots are queued by their officer or the Leader.
        /// </summary>
        private void ApplyQueueDefault(Village v, Building b, WorkSlot slot)
        {
            string key = QueueDefaultKey(b, slot);
            if (v.QueueDefaultsApplied.Contains(key))
                return;
            if (slot.AutomatedBy == "" && slot.Queue.Count == 0)
            {
                var recipe = DefaultRecipe(v, b);
                if (recipe == null)
                    return;
                slot.Queue.Add(new QueueEntry { RecipeId = recipe.Id, Repeat = true });
            }
            v.QueueDefaultsApplied.Add(key);
        }

        /// <summary>Staffed stations: scholars study, finished output is delivered, and the next queued recipe is funded and started.</summary>
        private void TickStations(Village v, double dt, bool planning)
        {
            foreach (var b in v.Buildings.Where(b => b.Completed))
                foreach (var slot in b.Slots.ToArray())
                {
                    var c = v.Cats.Find(c => c.Id == slot.CatId && c.Alive);
                    if (c == null || c.ControlledBy != "" || c.JobId != "" || c.Goal.StartsWith(GoalKey.NeedPrefix, StringComparison.Ordinal) || slot.Paused || b.Paused)
                        continue;
                    if (b.Kind == "research_hut" || b.Kind == "school")
                    {
                        c.Goal = GoalKey.Research;
                        if (Move(c, b.Position, dt))
                        {
                            double worked = SpendActorTime(c);
                            v.ResearchPoints += worked * WorkRate(v, c, "research", b.Kind) * Catalog.Effect(v, "researchRateMult", 1) * Catalog.BuildingEffect(v, b.Kind, "output", 1) / Pace.ResearchDivisor;
                            GainSkill(c, "research", worked / Pace.ResearchSkillSecondsPerPoint);
                        }
                        continue;
                    }
                    if (b.Kind == "accounting_tent")
                        continue;
                    var outputItems = v.Items.Where(i => i.LocationId == b.Id && i.StationCompartment == "local_output").ToArray();
                    if (b.Outputs.Count > 0 || outputItems.Length > 0)
                    {
                        if (outputItems.Length > 0 && !Move(c, b.Position, dt))
                            continue;
                        var delivery = new Job { Kind = "production", Phase = "output_delivery", TargetId = b.Id, Position = b.Position, Local = b.Outputs.Select(s => new Stack(s.Resource, s.Amount)).ToList() };
                        b.Outputs.Clear();
                        StartJob(v, c, delivery);
                        slot.JobId = delivery.Id;
                        foreach (var item in outputItems)
                        {
                            item.LocationId = delivery.Id;
                            delivery.ItemIds.Add(item.Id);
                        }
                        continue;
                    }
                    if (!planning)
                        continue;
                    if (b.Kind == "mouse_farm")
                    {
                        if (Available(v, "grain") < Pace.MouseFarmGrain)
                        {
                            b.BlockedReason = "missing_grain";
                            continue;
                        }
                        string mouseJob = Id("job");
                        if (!Reserve(v, mouseJob, "grain", Pace.MouseFarmGrain))
                            continue;
                        StartJob(v, c, new Job { Id = mouseJob, Kind = "mouse_farm", Phase = "fetch", Position = b.Position, TargetId = b.Id, RequiredWork = Pace.MouseFarmWorkSeconds });
                        continue;
                    }
                    var queue = slot.Queue;
                    ApplyQueueDefault(v, b, slot);
                    var recipe = queue.Count > 0 ? RecipeOf(queue[0].RecipeId) : null;
                    if (recipe == null)
                    {
                        slot.BlockedReason = b.BlockedReason = "queue_empty";
                        continue;
                    }
                    // Cheapest checks first; the storage route search (A*) runs last.
                    if (!Catalog.RecipeAvailable(v, recipe))
                    {
                        slot.BlockedReason = b.BlockedReason = "recipe_locked";
                        continue;
                    }
                    if (recipe.Inputs.Any(s => Available(v, s.Resource) + Amount(b.Inputs, s.Resource) < RecipeInput(v, recipe, s.Amount)))
                    {
                        slot.BlockedReason = b.BlockedReason = "missing_input";
                        continue;
                    }
                    // Automated work never spends the timber and blocks the next Den needs.
                    if (slot.AutomatedBy != "" && !KeepsConstructionReserve(v, recipe, b))
                    {
                        slot.BlockedReason = b.BlockedReason = ConstructionReserveBlock;
                        continue;
                    }
                    double localLoad = v.Jobs.Where(j => !j.Completed && j.Kind == "production" && j.TargetId == b.Id).Sum(j => j.Local.Sum(s => s.Amount));
                    if (localLoad + recipe.Inputs.Sum(s => RecipeInput(v, recipe, s.Amount)) > StationCapacity(v, b.Kind))
                    {
                        slot.BlockedReason = b.BlockedReason = "station_local_full";
                        continue;
                    }
                    if (recipe.Outputs.Any(s => Storage(v, s.Resource, s.Amount, b.Position) == null))
                    {
                        slot.BlockedReason = b.BlockedReason = "output_storage_full";
                        continue;
                    }
                    string id = Id("job");
                    bool funded = true;
                    foreach (var input in recipe.Inputs)
                    {
                        double missing = Math.Max(0, RecipeInput(v, recipe, input.Amount) - Amount(b.Inputs, input.Resource));
                        if (missing > 0 && !Reserve(v, id, input.Resource, missing))
                        {
                            funded = false;
                            break;
                        }
                    }
                    if (!funded)
                    {
                        Release(id);
                        slot.BlockedReason = b.BlockedReason = "input_claimed";
                        continue;
                    }
                    var production = new Job { Id = id, Kind = "production", RecipeId = recipe.Id, Position = b.Position, TargetId = b.Id, Phase = "fetch", RequiredWork = RecipeDuration(v, recipe), Reserved = recipe.Inputs.Select(s => new Stack(s.Resource, RecipeInput(v, recipe, s.Amount))).ToList() };
                    foreach (var input in production.Reserved)
                    {
                        double take = Math.Min(input.Amount, Amount(b.Inputs, input.Resource));
                        if (take > 0)
                        {
                            Add(b.Inputs, input.Resource, -take);
                            Add(production.Local, input.Resource, take);
                        }
                    }
                    StartJob(v, c, production);
                    slot.JobId = id;
                    slot.BlockedReason = b.BlockedReason = "";
                }
        }
    }
}
