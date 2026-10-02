using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>
        /// Moves steered cats. While a direction is held (MoveCat re-sent during each step) the cat chains
        /// tile to tile without stopping at tile centres. Movement time is the step divided by
        /// <see cref="ControlTimeScale"/>, so a steered cat walks in real time at any game speed and stands
        /// still at 0.
        /// </summary>
        private void TickControls(double seconds)
        {
            foreach (var v in Villages)
                foreach (var c in v.Cats.Where(c => c.Alive && c.ControlledBy != ""))
                {
                    if (c.ControlLeaseUntil <= TimeSeconds)
                    {
                        c.ControlledBy = "";
                        c.ControlDirection = default;
                        c.Path.Clear();
                        c.Goal = GoalKey.Idle;
                        continue;
                    }
                    if (c.Path.Count == 0 && c.ControlDirection.Equals(default(Int2)))
                    {
                        if (c.Goal == GoalKey.PlayerRest)
                        {
                            var den = v.Buildings.Find(b => b.Id == c.BedId && b.Completed);
                            if (den != null && InReach(c.Position, den.Position, den.Width, den.Depth))
                                c.Rest = Math.Min(100, c.Rest + SpendActorTime(c) * 0.15 * Service(v, "beds", "restRecovery", 1));
                            else
                                c.Goal = GoalKey.PlayerControl;
                        }
                        continue;
                    }
                    double scale = ControlTimeScale;
                    if (!Finite(scale) || scale <= 0)
                        continue;
                    double budget = seconds / scale;
                    // A few hops per step: arriving at a tile centre hands the leftover time to the next tile.
                    for (int hop = 0; hop < 8 && budget > 1e-12; hop++)
                    {
                        if (c.Path.Count == 0)
                        {
                            // Walk on only when the hold was refreshed during the step just finished.
                            if (!c.ControlDirection.Equals(default(Int2)) && c.ControlHeldAt <= c.ControlStepAt + 1e-9)
                                c.ControlDirection = default;
                            if (c.ControlDirection.Equals(default(Int2)))
                                break;
                            var next = ControlStep(v, c.Position, c.ControlDirection);
                            if (!next.HasValue)
                            {
                                c.ControlDirection = default;
                                break;
                            }
                            StepTo(v, c, next.Value);
                        }
                        c.Goal = GoalKey.PlayerControl;
                        double before = ActorTime(c);
                        Move(c, c.Path[c.Path.Count - 1], budget);
                        double used = before - ActorTime(c);
                        budget -= used;
                        if (used <= 1e-12 && c.Path.Count > 0)
                            break;
                    }
                }
        }

        /// <summary>The carried serving a controlled eat or drink would use, or null.</summary>
        private Stack Serving(Village v, Cat c, string mode)
        {
            var choices = mode == "drink" ? (!c.PersonalBrewUsed && Available(v, "water") >= Pace.WaterServingAfterBrew ? new[] { "water", "brew" } : new[] { "water" }) : new[] { "fish", "preserves", "food" };
            return c.Cargo.FirstOrDefault(s => choices.Contains(s.Resource) && s.Amount >= ServingSize(c, s.Resource));
        }
        private double ServingSize(Cat c, string resource) => resource == "brew" ? Pace.BrewServing : resource == "water" && c.PersonalBrewUsed ? Pace.WaterServingAfterBrew : Pace.NeedServing;

        private ActionResult ControlledRest(Village v, Cat c)
        {
            var den = v.Buildings.Find(b => b.Id == c.BedId && b.Completed);
            if (den == null)
                return ActionResult.Fail(ResultCode.NoReservedBed, "This cat has no bed in a finished Den yet");
            if (!InReach(c.Position, den.Position, den.Width, den.Depth))
                return ActionResult.Fail(ResultCode.OutOfReach, "Rest at this cat's reserved Den bed", (ResultArg.Target, den.Id), (ResultArg.Building, den.Kind));
            c.Path.Clear();
            c.ControlDirection = default;
            c.Goal = GoalKey.PlayerRest;
            return ActionResult.Ok(ResultCode.Resting, den.Id);
        }

        private ActionResult ControlledNeed(Village v, Cat c, string mode)
        {
            if (!AtShrine(v, c.Position))
                return ActionResult.Fail(ResultCode.GoToShrine, "Carry the serving to the shrine dining area", (ResultArg.Target, v.Center.ToString()));
            var serving = Serving(v, c, mode);
            if (serving == null)
                return mode == "drink"
                    ? ActionResult.Fail(ResultCode.NeedWaterInCargo, "Carry water to drink; Brew also requires clean Water", (ResultArg.Resource, "water"))
                    : ActionResult.Fail(ResultCode.NeedFoodInCargo, "Carry food to eat", (ResultArg.Resource, "food"));
            string resource = serving.Resource;
            double amount = ServingSize(c, resource);
            Add(c.Cargo, resource, -amount);
            if (mode == "drink")
            {
                c.Thirst = Math.Min(100, c.Thirst + Pace.DrinkRestorePerServing * amount);
                c.PersonalBrewUsed = resource == "brew";
                return ActionResult.Ok(ResultCode.Drank, c.Id, (ResultArg.Resource, resource), (ResultArg.Amount, ActionResult.Number(amount)));
            }
            c.Hunger = Math.Min(100, c.Hunger + Pace.EatRestorePerServing);
            return ActionResult.Ok(ResultCode.Ate, c.Id, (ResultArg.Resource, resource), (ResultArg.Amount, ActionResult.Number(amount)));
        }
    }
}
