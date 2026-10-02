using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>Runs or starts a need trip. Returns true while the trip consumes this cat's step.</summary>
        private bool TickNeeds(Village v, Cat c, double dt)
        {
            // Stay aboard over water; the same needs can be met physically after docking.
            if (v.Routes.Any(r => r.CatId == c.Id && r.Mode == "shipping" && r.Phase != "boarding" && v.Vehicles.Any(vehicle => vehicle.Id == r.VehicleId && vehicle.Position.Equals(c.Position) && (!Walkable(v, vehicle.Position) || vehicle.HasContinuousPosition && Math.Abs(vehicle.X - vehicle.Position.X) + Math.Abs(vehicle.Z - vehicle.Position.Z) > 1e-9))))
            {
                if (c.Goal.StartsWith(GoalKey.NeedPrefix, StringComparison.Ordinal))
                    Resume(c);
                return false;
            }
            if (c.Goal.StartsWith(GoalKey.NeedPrefix, StringComparison.Ordinal))
            {
                if (c.Goal == GoalKey.NeedSleep)
                {
                    var den = v.Buildings.Find(b => b.Id == c.BedId && b.Completed);
                    if (den == null)
                    {
                        c.Goal = GoalKey.Idle;
                        return false;
                    }
                    if (Move(c, den.Position, dt))
                        c.Rest = Math.Min(100, c.Rest + SpendActorTime(c) * Pace.SleepRecoveryPerSecond * Catalog.Effect(v, "restRecovery", 1));
                    if (c.Rest >= Pace.SleepWakeRest)
                        Resume(c);
                    return true;
                }
                string resource = c.Goal == GoalKey.NeedDrink ? "water" : c.Goal == GoalKey.NeedHeal ? "medicine" : "food";
                if (c.Cargo.Count == 0)
                {
                    var choices = c.Goal == GoalKey.NeedDrink ? (!c.PersonalBrewUsed && Available(v, "water") >= Pace.WaterServingAfterBrew ? new[] { "brew", "water" } : new[] { "water" }) : c.Goal == GoalKey.NeedHeal ? new[] { "medicine", "herbs" } : new[] { "fish", "preserves", "food" };
                    Stockpile pile = v.Stockpiles.Find(s => s.Id == c.NeedSourceId && s.Kind == "storage");
                    if (pile != null)
                    {
                        resource = choices.FirstOrDefault(choice => Amount(pile.Goods, choice) - Reservations.Where(r => r.PileId == pile.Id && r.Resource == choice).Sum(r => r.Amount) >= NeedServingSize(c, choice));
                        if (resource == null)
                            pile = null;
                    }
                    if (pile == null && TimeSeconds < c.NextNeedAttemptAt)
                        return true;
                    if (pile == null)
                        foreach (var choice in choices)
                        {
                            resource = choice;
                            double serving = NeedServingSize(c, resource);
                            pile = v.Stockpiles.Where(s => s.Kind == "storage" && Amount(s.Goods, resource) - Reservations.Where(r => r.PileId == s.Id && r.Resource == resource).Sum(r => r.Amount) >= serving).OrderBy(s => Int2.Distance(c.Position, s.Position)).FirstOrDefault(s => Path(c.Position, s.Position, v) != null);
                            if (pile != null)
                                break;
                        }
                    if (pile == null)
                    {
                        c.BlockedReason = "need_supply_missing";
                        c.NextNeedAttemptAt = Math.Floor(TimeSeconds) + Pace.NeedRetrySeconds;
                        Resume(c, preservePath: true);
                        return false;
                    }
                    c.NeedSourceId = pile.Id;
                    if (!Move(c, pile.Position, dt))
                    {
                        if (c.BlockedReason == "blocked_route")
                        {
                            c.NeedSourceId = "";
                            c.NextNeedAttemptAt = Math.Floor(TimeSeconds) + Pace.NeedRetrySeconds;
                        }
                        return true;
                    }
                    double wanted = NeedServingSize(c, resource);
                    double free = Amount(pile.Goods, resource) - Reservations.Where(r => r.PileId == pile.Id && r.Resource == resource).Sum(r => r.Amount);
                    if (free < wanted)
                    {
                        Resume(c);
                        return false;
                    }
                    Add(pile.Goods, resource, -wanted);
                    Add(c.Cargo, resource, wanted);
                    c.NeedSourceId = "";
                    return true;
                }
                if (!Move(c, v.Center, dt))
                    return true;
                if (c.Goal == GoalKey.NeedDrink)
                {
                    var serving = c.Cargo.Find(s => s.Resource == "water" || s.Resource == "brew");
                    if (serving == null)
                    {
                        Resume(c);
                        return false;
                    }
                    double dose = Math.Min(NeedServingSize(c, serving.Resource), serving.Amount);
                    c.Thirst = Math.Min(100, c.Thirst + Pace.DrinkRestorePerServing * dose);
                    Add(c.Cargo, serving.Resource, -dose);
                    if (serving.Resource == "brew")
                    {
                        c.PersonalBrewUsed = true;
                        c.Path.Clear();
                        return true;
                    }
                    c.PersonalBrewUsed = false;
                }
                else if (c.Goal == GoalKey.NeedHeal)
                {
                    var serving = c.Cargo.Find(s => s.Resource == "herbs" || s.Resource == "medicine");
                    if (serving == null)
                    {
                        Resume(c);
                        return false;
                    }
                    double dose = Math.Min(Pace.NeedServing, serving.Amount);
                    c.Health = Math.Min(100, c.Health + Pace.HealRestorePerServing * dose * Service(v, "herb_garden", "herbMedicineEfficacy", 1));
                    Add(c.Cargo, serving.Resource, -dose);
                }
                else
                {
                    var serving = c.Cargo.Find(s => s.Resource == "food" || s.Resource == "fish" || s.Resource == "preserves");
                    if (serving == null)
                    {
                        Resume(c);
                        return false;
                    }
                    double dose = Math.Min(Pace.NeedServing, serving.Amount);
                    c.Hunger = Math.Min(100, c.Hunger + Pace.EatRestorePerServing * dose);
                    Add(c.Cargo, serving.Resource, -dose);
                }
                Resume(c);
                return true;
            }
            if (c.Cargo.Count > 0)
            {
                if (c.Thirst >= Pace.CargoKeepThirst && c.Hunger >= Pace.CargoKeepHunger && c.Health >= Pace.CargoKeepHealth)
                    return false;
                bool providesNeed = c.Thirst < Pace.CargoKeepThirst && Amount(c.Cargo, "water") > 0 || c.Thirst >= Pace.CargoKeepThirst && c.Hunger < Pace.CargoKeepHunger && c.Cargo.Any(s => new[] { "food", "fish", "preserves" }.Contains(s.Resource));
                if (!providesNeed)
                {
                    var suspended = v.Jobs.Find(j => j.Id == c.JobId && !j.Completed);
                    var quantities = c.Cargo.Select(s => new Stack(s.Resource, s.Amount)).ToArray();
                    var pile = Spill(v, c.Position, c.Cargo);
                    if (suspended != null && pile != null)
                    {
                        suspended.SuspendedCargoPileId = pile.Id;
                        foreach (var s in quantities)
                            Reservations.Add(new Reservation { OwnerId = suspended.Id + ":resume", VillageId = v.Id, PileId = pile.Id, Resource = s.Resource, Amount = s.Amount });
                    }
                }
            }
            if (TimeSeconds < c.NextNeedAttemptAt)
                return false;
            string need = c.Thirst < Pace.DrinkThreshold ? GoalKey.NeedDrink : c.Hunger < Pace.EatThreshold ? GoalKey.NeedEat : c.Health < Pace.HealThreshold ? GoalKey.NeedHeal : c.Rest < Pace.SleepThreshold && c.BedId != "" ? GoalKey.NeedSleep : "";
            if (need == "")
                return false;
            c.ResumeJobId = c.JobId;
            c.Goal = need;
            c.NeedSourceId = "";
            // Keep moving work recoverable until a real need destination is selected.
            // Move replaces the path when that destination differs from the active route.
            return true;
        }
        /// <summary>One serving of a need resource: a small brew, a smaller water after a brew, or a whole unit.</summary>
        private double NeedServingSize(Cat c, string resource) => resource == "brew" ? Pace.BrewServing : resource == "water" && c.PersonalBrewUsed ? Pace.WaterServingAfterBrew : Pace.NeedServing;
        private void Resume(Cat c, bool preservePath = false)
        {
            c.NeedSourceId = "";
            c.JobId = c.ResumeJobId;
            c.ResumeJobId = "";
            if (!preservePath)
                c.Path.Clear();
            c.Goal = GoalKey.Idle;
        }
    }
}
