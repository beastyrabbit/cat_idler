using IdleCatForest.Simulation;

namespace IdleCatForest.Authority
{
    /// <summary>
    /// What the demo build shows and allows. Wave 1 stub: everything is visible and every action passes;
    /// W3-06 narrows it to the demo slice.
    /// </summary>
    public static class DemoScope
    {
        public static bool SharedWorldEnabled => true;

        /// <summary>Deepest dungeon level (counted from 1) an expedition may enter.</summary>
        public static int MaxDungeonDepth => int.MaxValue;

        public static bool IsSectionVisible(string section) => true;

        public static bool IsStudyVisible(string studyId) => true;

        public static bool IsBuildingVisible(string building) => true;

        /// <summary>A successful result when the action is allowed in this build, otherwise the refusal.</summary>
        public static ActionResult Check(GameAction action) => ActionResult.Ok();
    }
}
