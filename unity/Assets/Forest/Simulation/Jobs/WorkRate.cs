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
            rate *= Catalog.Effect(this, v, labor == "research" ? "researchRate" : labor == "build" ? "constructionSpeed" : "productionRate", 1);
            // A station's cycle_time speeds its recipes. Construction passes the building kind too, but in demo
            // worlds a production upgrade must not change how fast that building is built.
            if (!string.IsNullOrEmpty(station) && !(labor == "build" && DemoConstruction))
                rate /= Math.Max(Pace.WorkCycleTimeFloor, Catalog.BuildingEffect(this, v, station, "cycle_time", 1));
            rate *= ToolWorkBonus(v, c);
            return Math.Clamp(rate, Pace.WorkRateMin, Pace.WorkRateMax);
        }
    }
}
