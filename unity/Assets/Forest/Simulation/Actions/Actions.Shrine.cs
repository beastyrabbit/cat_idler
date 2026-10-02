using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        /// <summary>Goods the shrine turns into blessings.</summary>
        public static readonly string[] OfferableResources = { "food", "herbs", "materials" };

        // Goods a tithe takes from storage, and the work an offering takes at the shrine (both unchanged from before).
        private const double TitheAmount = 10;
        private const double OfferingJobWork = 10;

        /// <summary>
        /// The one offering rate used by tithes, offering jobs and shrine interaction:
        /// amount / Pace.OfferingHerbsPerBlessing, times the shrine's blessing yield.
        /// </summary>
        public double OfferingBlessings(Village v, double amount) => Math.Max(0, amount) / Pace.OfferingHerbsPerBlessing * Catalog.Effect(v, "shrineBlessingYield", 1);

        /// <summary>Credits the blessings for offered goods and returns them.</summary>
        private double ConvertOffering(Village v, double amount)
        {
            double blessings = OfferingBlessings(v, amount);
            v.Blessings += blessings;
            return blessings;
        }

        /// <summary>Stock the colony keeps back from offerings: survival food per cat, or a construction float.</summary>
        private double OfferingReserve(Village v, string resource) => resource == "food" ? v.Cats.Count(c => c.Alive) * Pace.FoodReservePerCat : 10;

        private ActionResult HandleOfferTithe(Village village)
        {
            double reserve = village.Cats.Count(c => c.Alive) * Pace.FoodReservePerCat + TitheAmount;
            double total = Total(village, "food");
            if (total < reserve)
                return ActionResult.Fail(ResultCode.ReserveProtected, "Food reserve protected", (ResultArg.Resource, "food"), (ResultArg.Need, ActionResult.Number(reserve)), (ResultArg.Have, ActionResult.Number(total)));
            double available = Available(village, "food");
            if (available < TitheAmount || !Spend(village, "food", TitheAmount))
                return ActionResult.Fail(ResultCode.MissingMaterial, "Food claimed", (ResultArg.Resource, "food"), (ResultArg.Need, ActionResult.Number(TitheAmount)), (ResultArg.Have, ActionResult.Number(available)));
            double blessings = ConvertOffering(village, TitheAmount);
            Note(village, EventKind.Tithe, "Food tithe earned " + ActionResult.Number(blessings) + " blessings");
            return ActionResult.Ok(ResultCode.Offered, village.Id, (ResultArg.Resource, "food"), (ResultArg.Amount, ActionResult.Number(TitheAmount)), (ResultArg.Blessings, ActionResult.Number(blessings)));
        }

        /// <summary>
        /// Sends a cat to carry an offering to the shrine. Honours Amount, at least one blessing's worth,
        /// limited to what one cat carries and never below the reserve.
        /// </summary>
        private ActionResult HandleOfferResource(Village village, GameAction action, string kind, Cat cat)
        {
            string offering = kind == "offermaterials" ? "materials" : action.Resource;
            if (!OfferableResources.Contains(offering))
                return ActionResult.Fail(ResultCode.InvalidResource, "Offering must be food, herbs, or materials", (ResultArg.Resource, offering ?? ""));
            var refusal = MissingCat(action, cat) ?? OrderRefusal(village, cat);
            if (refusal != null)
                return refusal;
            double minimum = Pace.OfferingHerbsPerBlessing;
            double reserve = OfferingReserve(village, offering), total = Total(village, offering), available = Available(village, offering);
            if (total - reserve < minimum)
                return ActionResult.Fail(ResultCode.ReserveProtected, "Survival and construction reserve protected", (ResultArg.Resource, offering), (ResultArg.Need, ActionResult.Number(reserve + minimum)), (ResultArg.Have, ActionResult.Number(total)));
            double amount = Math.Min(Math.Min(Math.Max(minimum, action.Amount), total - reserve), Math.Min(available, CarryCapacity(village, offering)));
            if (amount < minimum)
                return ActionResult.Fail(ResultCode.MissingMaterial, "Required finite input unavailable", (ResultArg.Resource, offering), (ResultArg.Need, ActionResult.Number(minimum)), (ResultArg.Have, ActionResult.Number(available)));
            var worker = cat ?? AvailableCat(village, JobLabor(JobKind.Offering));
            if (!FreeWorker(village, worker))
                return cat != null
                    ? ActionResult.Fail(ResultCode.CatBusy, "The cat is busy", (ResultArg.Target, cat.Id))
                    : ActionResult.Fail(ResultCode.NoWorker, "No available worker", (ResultArg.Kind, JobKind.Offering));
            var result = CreateInputJob(village, worker, JobKind.Offering, village.Center, offering, amount, OfferingJobWork, "");
            if (!result.Success)
                return result;
            return ActionResult.Ok(ResultCode.Queued, result.EntityId, (ResultArg.Kind, JobKind.Offering), (ResultArg.Resource, offering), (ResultArg.Amount, ActionResult.Number(amount)), (ResultArg.Blessings, ActionResult.Number(OfferingBlessings(village, amount))));
        }

        private ActionResult HandlePurchaseUpgrade(Village village, GameAction action)
        {
            string key = Snake(!string.IsNullOrEmpty(action.Name) ? action.Name : !string.IsNullOrEmpty(action.Resource) ? action.Resource : action.NodeId);
            int index = Array.IndexOf(Catalog.Upgrades, key);
            if (index < 0)
                return ActionResult.Fail(ResultCode.UnknownTarget, "Unknown upgrade", (ResultArg.Target, key));
            double level = UpgradeLevel(village, key);
            // Same caps as PurchaseUpgrade in Progression.cs: click power 20 levels, the others 10.
            double cap = index == 0 ? 20 : 10;
            if (level >= cap)
                return ActionResult.Fail(ResultCode.UpgradeMaxed, "Upgrade already maxed", (ResultArg.Target, key), (ResultArg.Have, ActionResult.Number(level)));
            double cost = UpgradeCost(village, key);
            if (village.Blessings < cost)
                return ActionResult.Fail(ResultCode.CannotAfford, "Insufficient blessings", (ResultArg.Resource, "blessings"), (ResultArg.Need, ActionResult.Number(cost)), (ResultArg.Have, ActionResult.Number(village.Blessings)), (ResultArg.Target, key));
            var result = PurchaseUpgrade(village, key);
            return result.Success ? ActionResult.Ok(ResultCode.Upgraded, key, (ResultArg.Target, key), (ResultArg.Amount, ActionResult.Number(level + 1))) : result;
        }
    }
}
