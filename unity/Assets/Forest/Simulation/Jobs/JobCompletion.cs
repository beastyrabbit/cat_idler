using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>
        /// Completes a job whose work is done, by kind. True when the job finished outright; false when its
        /// output (in <c>Local</c> or <c>ItemIds</c>) still has to be delivered.
        /// </summary>
        private bool CompleteJob(Village v, Cat c, Job j)
        {
            if (j.Kind == JobKind.Production)
            {
                CompleteJobProduction(v, c, j);
                return false;
            }
            if (j.Kind == JobKind.Offering)
            {
                CompleteJobOffering(v, c, j);
                return true;
            }
            if (j.Kind == JobKind.MouseFarm)
            {
                CompleteJobMouseFarm(v, j);
                return false;
            }
            if (j.Kind == JobKind.Expand)
            {
                CompleteJobExpand(v, c, j);
                return true;
            }
            if (j.Kind == JobKind.Train)
            {
                CompleteJobTrain(v, c, j);
                return true;
            }
            if (j.Kind == JobKind.Replant)
            {
                CompleteJobReplant(v, c, j);
                return true;
            }
            if (new[] { JobKind.Road, JobKind.Rail, JobKind.Bridge, JobKind.Dock, JobKind.Wagon, JobKind.Vessel }.Contains(j.Kind))
            {
                CompleteJobInfrastructure(v, c, j);
                return true;
            }
            CompleteJobGather(v, j);
            return false;
        }

        /// <summary>Consumes the reserved inputs, returns surplus to the station, adds outputs and advances the queue.</summary>
        private void CompleteJobProduction(Village v, Cat c, Job j)
        {
            var recipe = RecipeOf(j.RecipeId);
            foreach (var input in j.Reserved)
                Add(j.Local, input.Resource, -input.Amount);
            var inputStation = v.Buildings.Find(b => b.Id == j.TargetId);
            if (inputStation != null)
            {
                foreach (var surplus in j.Local)
                    Add(inputStation.Inputs, surplus.Resource, surplus.Amount);
                j.Local.Clear();
            }
            else
                Spill(v, j.Position, j.Local);
            foreach (var output in recipe.Outputs)
                Add(j.Local, output.Resource, RecipeOutput(v, recipe, output.Amount));
            if (recipe.ItemKind != "")
            {
                var item = new Item { Id = Id("item"), Kind = recipe.ItemKind, Material = recipe.Material, Quality = recipe.Quality, VillageId = v.Id, LocationId = j.Id, MaxCondition = Pace.ItemBaseCondition * Catalog.BuildingEffect(this, v, recipe.Building, "durability", 1) * (FamilyOwned(v, recipe, "preservation") ? Pace.PreservationDurabilityMult : 1) };
                item.Condition = item.MaxCondition;
                v.Items.Add(item);
                j.ItemIds.Add(item.Id);
            }
            var station = v.Buildings.Find(b => b.Id == j.TargetId);
            if (station != null)
            {
                var slot = station.Slots.Find(s => s.CatId == c.Id);
                var queue = slot?.Queue ?? station.Queue;
                if (queue.Count > 0 && queue[0].RecipeId == j.RecipeId)
                {
                    var completed = queue[0];
                    queue.RemoveAt(0);
                    if (completed.Repeat)
                        queue.Add(completed);
                }
            }
        }

        private void CompleteJobOffering(Village v, Cat c, Job j)
        {
            ConvertOffering(v, j.Local.Sum(s => s.Amount));
            j.Local.Clear();
            Finish(v, c, j);
        }

        private void CompleteJobMouseFarm(Village v, Job j)
        {
            j.Local.Clear();
            Add(j.Local, "food", Pace.MouseFarmFoodPerCycle * Service(v, "mouse_farm", "mouseFarmFood", 1));
        }

        /// <summary>Grows the settlement by one ring once every wall segment stands.</summary>
        private void CompleteJobExpand(Village v, Cat c, Job j)
        {
            j.Local.Clear();
            Expand(v);
            Finish(v, c, j);
        }

        /// <summary>Claims a restored legacy wall tile and hands the cat to the linked follow-up job.</summary>
        private void CompleteJobLegacyExpansion(Village v, Cat c, Job j)
        {
            ClaimLegacyTile(v, j);
            Finish(v, c, j);
            var linked = v.Jobs.Find(job => job.Id == j.LinkedJobId && !job.Completed && job.CatId == "");
            if (linked != null)
            {
                linked.CatId = c.Id;
                c.JobId = linked.Id;
            }
        }

        /// <summary>
        /// Commissions the building, finishes every builder's share of the work and logs the milestone. A fellow
        /// builder still carrying materials (only possible after a fault) leaves them in a spill at its feet.
        /// </summary>
        private void CompleteJobBuild(Village v, Cat c, Job j, Building b)
        {
            b.Inputs.Clear();
            Commission(v, b);
            Release(b.Id);
            b.BlockedReason = "";
            foreach (var other in ScaffoldJobs(v, b).Where(o => o.Id != j.Id).ToArray())
            {
                var helper = v.Cats.Find(x => x.Id == other.CatId && other.CatId != "");
                Release(other.Id + ":resume");
                if (helper == null)
                {
                    other.Completed = true;
                    Release(other.Id);
                    continue;
                }
                if (helper.Cargo.Count > 0)
                    Spill(v, helper.Position, helper.Cargo);
                Finish(v, helper, other);
            }
            Finish(v, c, j);
            NoteMilestone(v, EventKind.BuildingCompleted, c.Name + " completed " + b.Kind, b.Id, null, ("building", b.Kind), ("cat", c.Name));
        }

        /// <summary>
        /// The completion log line for a finished job, written only for milestone work: roads and rails, offerings
        /// and crafted items. Buildings and expansions log their own milestone events; routine gathering, hauling
        /// and production stay out of the log. <c>Finish</c> calls this in place of its unconditional note.
        /// </summary>
        private void NoteJobCompleted(Village v, Cat c, Job j)
        {
            bool milestone = j.Kind == JobKind.Road || j.Kind == JobKind.Rail || j.Kind == JobKind.Offering || j.Kind == JobKind.Production && RecipeOf(j.RecipeId)?.ItemKind is string item && item != "";
            if (milestone)
                Note(v, EventKind.Completed, c.Name + " completed " + j.Kind, j.Id);
        }

        /// <summary>
        /// A scout's report on returning: how many tiles it added to the village map, logged as a discovery about
        /// the scout ("Hazel mapped 12 tiles"), not as research.
        /// </summary>
        private void NoteScoutReport(Village v, Cat c, int mappedTiles) => NoteEvent(v, EventKind.Discovery, c.Name + " mapped " + mappedTiles + " tiles", c.Id, null, "", new Dictionary<string, string> { ["cat"] = c.Name, ["tiles"] = mappedTiles.ToString(System.Globalization.CultureInfo.InvariantCulture) });

        /// <summary>Road and rail are paved tile by tile, so finishing the last tile only ends the job (Finish logs it).</summary>
        private void CompleteJobPaving(Village v, Cat c, Job j) => Finish(v, c, j);

        private void CompleteJobInfrastructure(Village v, Cat c, Job j)
        {
            CompleteInfrastructure(v, j);
            j.Local.Clear();
            Finish(v, c, j);
        }

        private void CompleteJobTrain(Village v, Cat c, Job j)
        {
            GainSkill(c, "fight", Pace.TrainFightSkill);
            Finish(v, c, j);
        }

        private void CompleteJobReplant(Village v, Cat c, Job j)
        {
            var t = TileAt(j.Position);
            if (t.Resource == "logs" && t.Amount <= 0 && t.RegrowAt < 0)
                t.RegrowAt = TimeSeconds + Pace.ReplantRegrowSeconds;
            Finish(v, c, j);
        }

        /// <summary>Hunt, fish, water and gathering: takes the yield from the source into the job's output.</summary>
        private void CompleteJobGather(Village v, Job j)
        {
            var tile = TileAt(j.Position);
            double quantity = j.Amount > 0 ? j.Amount : CarryCapacity(v, j.Resource) * (j.Kind == JobKind.Hunt ? Catalog.Effect(this, v, "huntYieldMult", 1) : j.Kind == JobKind.Fibre ? Catalog.Effect(this, v, "gatherYieldMult", 1) : j.Kind == JobKind.Quarry || j.Kind == JobKind.Logs ? Catalog.Effect(this, v, "materialYieldMult", 1) : 1) * j.YieldMultiplier;
            if (j.Kind == JobKind.Hunt)
                quantity *= 1 + UpgradeLevel(v, "hunt_mastery") * Pace.HuntMasteryYieldPerLevel;
            if (j.Kind != JobKind.Water && j.Kind != JobKind.Fish)
            {
                quantity = Math.Min(quantity, SourceAmount(tile, j.Resource));
                TakeSource(tile, j.Resource, quantity);
            }
            if (j.Kind == JobKind.Fish)
            {
                var habitat = Neighbors(j.Position).Select(TileAt).FirstOrDefault(t => t.Water && FishStock(t) > 0);
                quantity = habitat == null ? 0 : Math.Min(Pace.FishCatchMax, FishStock(habitat));
                if (habitat != null)
                    SetFish(habitat, FishStock(habitat) - quantity);
            }
            if (quantity > 0)
                Add(j.Local, j.Resource, quantity);
            if (j.Kind == JobKind.Hunt && quantity > 0)
            {
                Add(j.Local, "hide", quantity * Pace.HuntHidePerFood);
                Add(j.Local, "bone", quantity * Pace.HuntBonePerFood);
                // Only an emptied hunting ground waits to regrow; hunting a full deposit must not reset it.
                if (SourceAmount(tile, j.Resource) <= 1e-9 && tile.RegrowAt < 0)
                    tile.RegrowAt = TimeSeconds + Pace.HuntRegrowSeconds;
            }
        }
    }
}
