using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private ActionResult HandleOfferTithe(Village village)
        {
            if (Total(village, "food") < village.Cats.Count(c => c.Alive) * 6 + 10)
                return ActionResult.Fail("Food reserve protected");
            if (!Spend(village, "food", 10))
                return ActionResult.Fail("Food claimed");
            village.Blessings += 1;
            Note(village, EventKind.Tithe, "Food tithe earned one blessing");
            return ActionResult.Ok();
        }
        private ActionResult HandleOfferResource(Village village, GameAction action, string kind, Cat cat)
        {
            string offering = kind == "offermaterials" ? "materials" : action.Resource;
            if (!new[] { "food", "herbs", "materials" }.Contains(offering))
                return ActionResult.Fail("Offering must be food, herbs, or materials");
            if (Total(village, offering) < (offering == "food" ? village.Cats.Count(c => c.Alive) * 6 : 10) + 5)
                return ActionResult.Fail("Survival and construction reserve protected");
            return CreateInputJob(village, cat, "offering", village.Center, offering, 5, 10, "");
        }
        private ActionResult HandlePurchaseUpgrade(Village village, GameAction action) => PurchaseUpgrade(village, !string.IsNullOrEmpty(action.Name) ? action.Name : !string.IsNullOrEmpty(action.Resource) ? action.Resource : action.NodeId);
    }
}
