

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private ActionResult HandleExploreDungeon(Village village, GameAction action, Cat cat) => ExploreDungeon(village, cat, action.TargetId);
        private ActionResult HandleRecallExplorer(Village village, Cat cat) => RecallExplorer(village, cat);
        private ActionResult HandleAttackCreature(PlayerContext context, Village village, GameAction action, Cat cat)
        {
            if (cat == null || cat.ControlledBy != context.PlayerId)
                return ActionResult.Fail("Control lease required");
            return AttackCreature(village, cat, action.TargetId);
        }
    }
}
