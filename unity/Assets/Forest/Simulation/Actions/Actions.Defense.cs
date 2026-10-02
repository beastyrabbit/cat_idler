

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private ActionResult HandleTrainWarrior(Village village, Cat cat) => Request(village, "train", cat, true);
        private ActionResult HandleDefendRaid(Village village)
        {
            if (village.Raids.Count == 0)
                return ActionResult.Fail("No active raid");
            village.Raids[0].Health -= 6;
            return ActionResult.Ok();
        }
    }
}
