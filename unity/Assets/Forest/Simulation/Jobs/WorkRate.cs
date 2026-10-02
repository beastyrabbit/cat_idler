using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private double WorkRate(Village v, Cat c, string labor, string station)
        {
            double skill = Amount(c.Skills, labor), rate = (1 + Math.Min(Pace.WorkSkillBonusCap, skill / Pace.WorkSkillDivisor)) * UpgradeWorkRate(v, labor);
            string stat = labor == "hunt" || labor == "fishing" ? "hunting" : labor == "research" ? "medicine" : labor == "scout" ? "vision" : labor == "fight" ? "attack" : "building";
            rate *= Pace.WorkStatBase + Math.Clamp(Amount(c.Stats, stat), 0, 100) / Pace.WorkStatDivisor;
            rate *= Catalog.Effect(v, labor == "research" ? "researchRate" : labor == "build" ? "constructionSpeed" : "productionRate", 1);
            if (!string.IsNullOrEmpty(station))
                rate /= Math.Max(Pace.WorkCycleTimeFloor, Catalog.BuildingEffect(v, station, "cycle_time", 1));
            if (v.Items.Any(i => c.Equipment.Contains(i.Id) && i.Kind == "tool" && i.Condition > 0))
                rate *= Pace.WorkToolBonus;
            return Math.Clamp(rate, Pace.WorkRateMin, Pace.WorkRateMax);
        }
    }
}
