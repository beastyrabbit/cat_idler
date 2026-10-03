using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>
        /// The working phase: gathers production inputs, runs segmented wall and paving work, or accumulates
        /// progress and completes the job. True when the tick ends here, false to start output delivery.
        /// </summary>
        private bool TickJobWorking(Village v, Cat c, Job j, bool planning, double dt)
        {
            if (j.Kind == JobKind.Production && JobAwaitingProductionInputs(v, c, j))
                return true;
            if (j.Kind == JobKind.Expand)
            {
                if (j.OriginalKind == JobKind.ExpandVillage)
                    WorkLegacyExpansion(v, c, j, dt);
                else
                    WorkExpansionWall(v, c, j, planning, dt);
                return true;
            }
            if (j.Kind == JobKind.Road || j.Kind == JobKind.Rail)
            {
                WorkPaving(v, c, j, dt);
                return true;
            }
            if (!Move(c, j.Position, dt))
                return true;
            j.Progress += SpendActorTime(c) * WorkRate(v, c, j.Kind == JobKind.Production ? RecipeOf(j.RecipeId)?.Labor ?? "process" : JobLabor(j.Kind), j.Kind == JobKind.Production ? RecipeOf(j.RecipeId)?.Building : "") * j.SpeedMultiplier;
            if (j.Progress + 1e-9 < j.RequiredWork)
                return true;
            if (CompleteJob(v, c, j))
                return true;
            j.Phase = JobPhase.OutputDelivery;
            c.Path.Clear();
            return false;
        }

        /// <summary>
        /// Pulls recipe inputs from the station or reserves missing ones; true when the job must wait or
        /// fetch first, false when every input is at hand.
        /// </summary>
        private bool JobAwaitingProductionInputs(Village v, Cat c, Job j)
        {
            var recipe = RecipeOf(j.RecipeId);
            if (recipe == null)
            {
                j.BlockedReason = Blocked.UnknownRecipe;
                return true;
            }
            if (j.Reserved.Count == 0)
                j.Reserved = recipe.Inputs.Select(s => new Stack(s.Resource, RecipeInput(v, recipe, s.Amount))).ToList();
            var station = v.Buildings.Find(b => b.Id == j.TargetId);
            foreach (var required in j.Reserved)
            {
                double missing = Math.Max(0, required.Amount - Amount(j.Local, required.Resource));
                if (missing > 0 && station != null)
                {
                    double fromStation = Math.Min(missing, Amount(station.Inputs, required.Resource));
                    if (fromStation > 0)
                    {
                        Add(station.Inputs, required.Resource, -fromStation);
                        Add(j.Local, required.Resource, fromStation);
                        missing -= fromStation;
                    }
                }
                if (missing > 1e-8)
                {
                    double reserved = Reservations.Where(r => r.OwnerId == j.Id && r.Resource == required.Resource).Sum(r => r.Amount);
                    if (reserved + 1e-8 < missing && !ReserveNearest(v, j.Id, required.Resource, missing - reserved, c.Position))
                    {
                        j.BlockedReason = Blocked.MissingRecipeInput;
                        return true;
                    }
                    j.Phase = JobPhase.Fetch;
                    c.Path.Clear();
                    return true;
                }
            }
            return false;
        }

        /// <summary>
        /// Construction of a planned building: resolves the scaffold, collects its bill of materials, then
        /// accumulates build progress until the building is commissioned. Several builders may share a site:
        /// each adds its own work rate, and materials already carried by a fellow builder count toward the bill.
        /// </summary>
        private void TickJobBuild(Village v, Cat c, Job j, bool planning, double dt)
        {
            var b = v.Buildings.Find(x => x.Id == j.TargetId) ?? (planning ? ResolveScaffold(v, c, j) : null);
            if (b == null)
                return;
            if (b.Completed)
            {
                // A fellow builder commissioned the site; this share of the work is done.
                Finish(v, c, j);
                return;
            }
            if (b.Required.All(s => Amount(b.Inputs, s.Resource) >= s.Amount))
            {
                if (!Move(c, b.Position, dt))
                    return;
                j.Phase = JobPhase.Working;
                ClearScaffoldShortage(j, b);
                b.Progress += SpendActorTime(c) * WorkRate(v, c, "build", b.Kind);
                if (b.Progress + 1e-9 >= b.RequiredWork)
                    CompleteJobBuild(v, c, j, b);
                return;
            }
            if (c.Cargo.Count > 0)
            {
                if (!Move(c, b.Position, dt))
                    return;
                foreach (var s in c.Cargo)
                    Add(b.Inputs, s.Resource, s.Amount);
                c.Cargo.Clear();
                return;
            }
            bool shortage = false;
            foreach (var input in b.Required)
            {
                double need = input.Amount - ScaffoldSecured(v, b, input.Resource);
                if (need <= 1e-8 || ReserveNearest(v, b.Id, input.Resource, need, c.Position))
                    continue;
                if (!DemoConstruction)
                {
                    j.BlockedReason = Blocked.MissingScaffoldInput;
                    return;
                }
                // Demo: stage what exists now and come back for the rest.
                ReserveUpTo(v, b.Id, input.Resource, need);
                shortage = true;
            }
            if (shortage && !Reservations.Any(r => r.OwnerId == b.Id))
            {
                // Nothing left to fetch: hand the site back to the safety net instead of standing idle on it.
                b.BlockedReason = Blocked.MissingScaffoldInput;
                j.Completed = true;
                j.CatId = "";
                j.BlockedReason = Blocked.MissingScaffoldInput;
                Release(j.Id);
                c.JobId = "";
                c.Path.Clear();
                c.Goal = GoalKey.Idle;
                c.BlockedReason = "";
                return;
            }
            if (!shortage)
                ClearScaffoldShortage(j, b);
            PickupClaim(v, c, b.Id, j, dt);
        }

        private static void ClearScaffoldShortage(Job j, Building b)
        {
            if (j.BlockedReason == Blocked.MissingScaffoldInput)
                j.BlockedReason = "";
            if (b.BlockedReason == Blocked.MissingScaffoldInput)
                b.BlockedReason = "";
        }

        /// <summary>
        /// Materials a wall or road builder takes per trip: one in classic worlds; in demo worlds a full load,
        /// limited by what is staged and by the segments left.
        /// </summary>
        private double SegmentLoad(Village v, Job j, string resource)
        {
            double staged = Math.Floor(Amount(j.Local, resource) + 1e-9);
            return DemoConstruction ? Math.Max(1, Math.Min(Math.Min(CarryCapacity(v, resource), j.Path.Count - j.PathIndex), staged)) : 1;
        }

        /// <summary>Wall and road work speed: the builder's work rate in demo worlds, flat in classic worlds.</summary>
        private double SegmentRate(Village v, Cat c) => DemoConstruction ? WorkRate(v, c, "build", "") : 1;

        /// <summary>A restored legacy wall project: one tile of work at the job position, then claim it.</summary>
        private void WorkLegacyExpansion(Village v, Cat c, Job j, double dt)
        {
            if (!Move(c, j.Position, dt))
                return;
            j.Progress += SpendActorTime(c) * WorkRate(v, c, "build", "");
            if (j.Progress + 1e-9 >= j.RequiredWork)
                CompleteJobLegacyExpansion(v, c, j);
        }

        /// <summary>
        /// Builds the expansion ring one wall segment at a time: take materials from the staging point (one unit
        /// in classic worlds, a full load in demo worlds), walk to a work stand beside the segment, work it,
        /// advance, and keep building consecutive segments until the load is used. Completes when every segment
        /// stands.
        /// </summary>
        private void WorkExpansionWall(Village v, Cat c, Job j, bool planning, double dt)
        {
            if (j.PathIndex >= j.Path.Count)
            {
                CompleteJobExpand(v, c, j);
                return;
            }
            var target = j.Path[j.PathIndex];
            if (!j.HasWorkStand || j.WorkStandIndex != j.PathIndex || !Walkable(j.WorkStand))
            {
                if (!planning && j.WorkStandIndex == j.PathIndex)
                    return;
                j.WorkStandIndex = j.PathIndex;
                var stand = Neighbors(target).Where(Walkable).OrderBy(p => Int2.Distance(p, c.Position)).Where(p => Path(c.Position, p) != null).Select(p => (Int2?)p).FirstOrDefault();
                j.HasWorkStand = stand.HasValue;
                if (!stand.HasValue)
                {
                    j.BlockedReason = c.BlockedReason = Blocked.BlockedRoute;
                    return;
                }
                j.WorkStand = stand.Value;
            }
            if (c.Cargo.Count == 0)
            {
                if (!Move(c, j.Position, dt))
                    return;
                if (Amount(j.Local, "materials") < 1)
                {
                    j.BlockedReason = Blocked.WallMaterialsMissing;
                    return;
                }
                double load = SegmentLoad(v, j, "materials");
                Add(j.Local, "materials", -load);
                Add(c.Cargo, "materials", load);
            }
            if (!Move(c, j.WorkStand, dt))
            {
                if (Blocked.IsRoute(c.BlockedReason))
                    j.HasWorkStand = false;
                return;
            }
            j.Progress += SpendActorTime(c) * SegmentRate(v, c);
            if (j.Progress + 1e-9 < Pace.WallSegmentWork)
                return;
            j.Progress = 0;
            TileAt(target).Wall = true;
            Add(c.Cargo, "materials", -1);
            j.PathIndex++;
        }

        /// <summary>
        /// Paves a road or rail one tile at a time from the staging point at the first tile: one unit of the
        /// paving input per trip in classic worlds, a full load paving consecutive tiles in demo worlds.
        /// </summary>
        private void WorkPaving(Village v, Cat c, Job j, double dt)
        {
            if (j.PathIndex >= j.Path.Count)
            {
                CompleteJobPaving(v, c, j);
                return;
            }
            var target = j.Path[j.PathIndex];
            if (c.Cargo.Count == 0)
            {
                if (!Move(c, j.Position, dt))
                    return;
                if (Amount(j.Local, j.Resource) < 1)
                {
                    j.BlockedReason = Blocked.PavingInputMissing;
                    return;
                }
                double load = SegmentLoad(v, j, j.Resource);
                Add(j.Local, j.Resource, -load);
                Add(c.Cargo, j.Resource, load);
            }
            if (!Move(c, target, dt))
                return;
            j.Progress += SpendActorTime(c) * SegmentRate(v, c);
            if (j.Progress + 1e-9 < Pace.PavingSegmentWork)
                return;
            j.Progress = 0;
            var tile = TileAt(target);
            if (j.Kind == JobKind.Road)
            {
                tile.Road = true;
                tile.Dirt = false;
            }
            else
                tile.Rail = true;
            Add(c.Cargo, j.Resource, -1);
            j.PathIndex++;
        }
    }
}
