using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private void TickControls(double seconds)
        {
            foreach (var v in Villages)
                foreach (var c in v.Cats.Where(c => c.Alive && c.ControlledBy != ""))
                {
                    if (c.ControlLeaseUntil <= TimeSeconds)
                    {
                        c.ControlledBy = "";
                        c.Path.Clear();
                        c.Goal = "idle";
                        continue;
                    }
                    if (c.Path.Count > 0)
                    {
                        c.Goal = "player_control";
                        Move(c, c.Path[c.Path.Count - 1], seconds);
                    }
                    else if (c.Goal == "player_rest")
                    {
                        var den = v.Buildings.Find(b => b.Id == c.BedId && b.Completed);
                        if (den != null && Int2.Distance(c.Position, den.Position) <= 1)
                            c.Rest = Math.Min(100, c.Rest + seconds * 0.15 * Service(v, "beds", "restRecovery", 1));
                        else
                            c.Goal = "player_control";
                    }
                }
        }
        private ActionResult ControlledNeed(Village v, Cat c, string mode)
        {
            if (mode == "rest")
            {
                var den = v.Buildings.Find(b => b.Id == c.BedId && b.Completed);
                if (den == null || Int2.Distance(c.Position, den.Position) > 1)
                    return ActionResult.Fail("Rest at this cat's reserved Den bed");
                c.Path.Clear();
                c.Goal = "player_rest";
                return ActionResult.Ok();
            }
            if (Int2.Distance(c.Position, v.Center) > 1)
                return ActionResult.Fail("Carry the serving to the shrine dining area");
            var choices = mode == "drink" ? (!c.PersonalBrewUsed && Available(v, "water") >= 0.75 ? new[] { "water", "brew" } : new[] { "water" }) : new[] { "fish", "preserves", "food" };
            var serving = c.Cargo.FirstOrDefault(s => choices.Contains(s.Resource) && s.Amount >= (s.Resource == "brew" ? 0.25 : s.Resource == "water" && c.PersonalBrewUsed ? 0.75 : 1));
            if (serving == null)
                return ActionResult.Fail("Carry a matching finite serving; Brew also requires clean Water");
            double amount = serving.Resource == "brew" ? 0.25 : serving.Resource == "water" && c.PersonalBrewUsed ? 0.75 : 1;
            Add(c.Cargo, serving.Resource, -amount);
            if (mode == "drink")
            {
                c.Thirst = Math.Min(100, c.Thirst + 50 * amount);
                c.PersonalBrewUsed = serving.Resource == "brew";
            }
            else
                c.Hunger = Math.Min(100, c.Hunger + 50);
            return ActionResult.Ok();
        }
    }
}
