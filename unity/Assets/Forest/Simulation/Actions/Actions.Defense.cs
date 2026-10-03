
namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private ActionResult HandleTrainWarrior(Village village, Cat cat) => Request(village, "train", cat, true);

        /// <summary>The DefendRaid action kind rallies defenders; it never damages raiders directly.</summary>
        private ActionResult HandleDefendRaid(Village village) => RallyDefenders(village);
    }
}
