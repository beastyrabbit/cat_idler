using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private ActionResult HandleResearchNode(Village village, GameAction action, string kind) => Research(village, action.NodeId, kind == "unlocknode");
        private ActionResult Research(Village v, string id, bool blessings)
        {
            var node = Catalog.Study(id);
            if (node == null)
                return ActionResult.Fail("Unknown study");
            if (v.Research.Contains(id))
                return ActionResult.Fail("Already researched");
            if (node.Prerequisites.Any(p => !v.Research.Contains(p)))
                return ActionResult.Fail("Missing prerequisites");
            if (blessings && Catalog.Research.IndexOf(node) >= 24)
                return ActionResult.Fail("Blessings purchase original studies only");
            if ((blessings ? v.Blessings : v.ResearchPoints) < node.Cost)
                return ActionResult.Fail("Insufficient " + (blessings ? "blessings" : "research points"));
            if (blessings)
                v.Blessings -= node.Cost;
            else
                v.ResearchPoints -= node.Cost;
            v.Research.Add(id);
            Note(v, EventKind.Research, node.Name, id);
            return ActionResult.Ok(id);
        }
    }
}
