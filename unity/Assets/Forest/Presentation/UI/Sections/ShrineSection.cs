using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    public sealed class ShrineSection : ISection
    {
        /// <summary>The shrine offers blessing unlocks for the first studies of the catalog.</summary>
        public const int BlessingStudyCount = 24;

        private static readonly string[] Offerings = { "food", "herbs", "materials" };

        public string Id => "Shrine";
        public string Label => "Shrine";
        public string Group => "Growth";
        public string Icon => "";
        public bool Wide => false;

        public int Badge(UiContext ctx) => 0;

        public void Refresh(UiContext ctx) { }

        public VisualElement Build(UiContext ctx)
        {
            var root = UiKit.SectionRoot(Id);
            var world = ctx.World;
            var village = ctx.Village;
            var picks = ctx.Picks;
            UiKit.Text(root, "Blessings " + village.Blessings.ToString("0.0") + " · separate from research points. Offerings travel with a living cat to the shrine.");
            UiKit.Choice(root, "Offering", Offerings, picks.SelectedResource, x => picks.SelectedResource = x).Named("choice-offering");
            picks.AmountField(root);
            UiKit.Button(root, "Carry offering to shrine", () => ctx.Act(new GameAction { Kind = "OfferResource", Resource = picks.SelectedResource, Amount = picks.Amount })).Named(UiNames.Act("offer", "resource"));
            UiKit.Button(root, "Offer materials", () => ctx.Act(new GameAction { Kind = "OfferMaterials", Amount = picks.Amount })).Named(UiNames.Act("offer", "materials"));
            UiKit.Button(root, "Offer surplus tithe", () => ctx.Act(new GameAction { Kind = "OfferTithe" })).Named(UiNames.Act("offer", "tithe"));
            UiKit.Button(root, "Assign ritual work", () => ctx.Act(new GameAction { Kind = "RequestJob", Name = "ritual" })).Named(UiNames.Act("assign", "ritual"));
            foreach (var upgrade in Catalog.Upgrades)
            {
                string key = upgrade;
                UiKit.Button(root, UiText.Pretty(key) + " · level " + world.UpgradeLevel(village, key) + " · " + world.UpgradeCost(village, key) + " blessings", () => ctx.Act(new GameAction { Kind = "PurchaseUpgrade", Name = key })).Named(UiNames.Act("upgrade", key));
            }
            foreach (var n in Catalog.Research.Take(BlessingStudyCount).Where(n => !village.Research.Contains(n.Id)))
            {
                string id = n.Id;
                UiKit.Button(root, "Blessing: " + n.Name + " · " + n.Cost.ToString("0"), () => ctx.Act(new GameAction { Kind = "UnlockNode", NodeId = id })).Named(UiNames.Act("unlock", id));
            }
            return root;
        }
    }
}
