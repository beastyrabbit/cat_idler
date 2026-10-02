using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>Each farm's worker tends the field, harvests, and carries the crop to the handoff pile.</summary>
        private void TickFarms(Village v, double dt)
        {
            foreach (var f in v.Farms)
            {
                var c = v.Cats.Find(c => c.Id == f.WorkerId && c.Alive);
                if (c == null)
                {
                    f.BlockedReason = "worker_required";
                    continue;
                }
                if (c.JobId != "" || c.ControlledBy != "" || c.Goal.StartsWith(GoalKey.NeedPrefix, StringComparison.Ordinal))
                    continue;
                c.Goal = GoalKey.FarmPrefix + f.Phase;
                if (f.Harvest > 0 || c.Cargo.Count > 0)
                {
                    if (c.Cargo.Count == 0)
                    {
                        if (!Move(c, f.Position, dt))
                        {
                            f.BlockedReason = "blocked_harvest_route";
                            continue;
                        }
                        double take = Math.Min(Pace.FarmHarvestCarry, f.Harvest);
                        Add(c.Cargo, f.Crop, take);
                        f.Harvest -= take;
                        f.Phase = "hauling";
                        continue;
                    }
                    if (!Move(c, f.Handoff, dt))
                    {
                        f.BlockedReason = "blocked_handoff_route";
                        continue;
                    }
                    var pile = v.Stockpiles.Find(s => s.ManagedBy == f.Id);
                    if (pile == null)
                    {
                        pile = new Stockpile { Id = Id("harvest"), ManagedBy = f.Id, Kind = "farm_output", Position = f.Handoff, Width = 1, Depth = 1, Capacity = 100 };
                        v.Stockpiles.Add(pile);
                    }
                    if (pile.Goods.Sum(s => s.Amount) + c.Cargo.Sum(s => s.Amount) > pile.Capacity)
                    {
                        f.BlockedReason = "handoff_full";
                        continue;
                    }
                    foreach (var cargo in c.Cargo)
                        Add(pile.Goods, cargo.Resource, cargo.Amount);
                    c.Cargo.Clear();
                    GainSkill(c, "haul", Pace.FarmHaulSkillGain);
                    f.BlockedReason = "";
                    continue;
                }
                if (!Move(c, f.Position, dt))
                {
                    f.BlockedReason = "blocked_route";
                    continue;
                }
                f.BlockedReason = "";
                double worked = SpendActorTime(c);
                f.Growth += worked * WorkRate(v, c, "farm", "field") * Service(v, "field", "fieldStewardship", 1) * (f.FertilityAffectsGrowth ? Math.Max(0, f.Fertility) : 1);
                GainSkill(c, "farm", worked / Pace.FarmSkillSecondsPerPoint);
                double cycle = Math.Max(1, f.CycleSeconds), fraction = f.Growth / cycle;
                f.Phase = fraction < 1.0 / 12 ? "soil" : fraction < 0.25 ? "sprout" : fraction < 0.5 ? "growing" : fraction < 0.75 ? "mature" : "flowering";
                if (f.Growth + 1e-9 >= cycle)
                {
                    f.Harvest = f.YieldPerTile * f.Width * f.Depth * (f.FertilityAffectsGrowth ? 1 : Math.Max(0, f.Fertility)) * Catalog.Effect(v, "farmYield", 1) * Catalog.Effect(v, "farmYieldMult", 1) * Catalog.BuildingEffect(v, "field", "output", 1);
                    f.Growth = 0;
                    f.Phase = "harvesting";
                }
            }
        }
    }
}
