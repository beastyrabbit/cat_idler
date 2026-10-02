using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    public sealed class StoresSection : ISection
    {
        public string Id => "Stores";
        public string Label => "Stores";
        public string Group => "Production";
        public string Icon => "";
        public bool Wide => false;

        public int Badge(UiContext ctx) => 0;

        public void Refresh(UiContext ctx) => ctx.Drawer.RebuildKeepingScroll();

        public VisualElement Build(UiContext ctx)
        {
            var root = UiKit.SectionRoot(Id);
            var village = ctx.Village;
            var picks = ctx.Picks;
            UiKit.Text(root, "* Inventory is the last physical count. An Accountant refreshes each pile by visiting it; stale counts are not current stock.");
            foreach (var resource in Catalog.Resources.Where(r => r != "blessings"))
                UiKit.Label(root, UiText.Pretty(resource) + "   " + UiText.Reported(ctx.World, village, resource), "stock-row");
            UiKit.Separator(root);
            picks.ResourceChoice(root);
            UiKit.Button(root, "Designate general stockpile", () => ctx.BeginPlacement("Stockpile: choose opposite corners inside the village", p => new GameAction { Kind = "DesignateStockpile", Position = p, Name = "General" }, true)).Named(UiNames.Placement("stockpile"));
            UiKit.Button(root, "Designate selected-resource pile", () => ctx.BeginPlacement("Filtered stockpile: choose opposite corners inside the village", p => new GameAction { Kind = "DesignateStockpile", Position = p, Accepts = new List<string> { picks.SelectedResource } }, true)).Named(UiNames.Placement("stockpile-filtered"));
            foreach (var p in village.Stockpiles)
            {
                var pile = p;
                var card = UiKit.Card(root, p.Kind + " · " + p.Id);
                UiKit.Label(card, "Capacity " + p.Capacity.ToString("0") + " · " + (p.CountedAt < 0 ? "uncounted" : "counted at " + (p.CountedAt / 3600).ToString("0.0") + "h"), "muted");
                UiKit.Label(card, UiText.Goods(p.Report), "body");
                UiKit.Button(card, "Haul from pile", () => ctx.Act(new GameAction { Kind = "HaulGatherSpot", TargetId = pile.Id })).Named(UiNames.Act("haul-pile", pile.Id));
                UiKit.Button(card, "Remove pile", () => ctx.Act(new GameAction { Kind = pile.Kind == "storage" ? "RemoveStockpile" : "RemoveGatherSpot", TargetId = pile.Id })).Named(UiNames.Act("remove-pile", pile.Id));
            }
            UiKit.Separator(root);
            UiKit.Label(root, "Equipment", "subtitle");
            picks.CatChoice(ctx, root);
            foreach (var item in village.Items)
            {
                var i = item;
                var card = UiKit.Card(root, UiText.Pretty(i.Material) + " " + i.Kind + " · quality " + i.Quality);
                UiKit.Label(card, i.Condition.ToString("0") + " / " + i.MaxCondition.ToString("0") + " condition", "body");
                UiKit.Button(card, "Equip selected cat", () => ctx.Act(new GameAction { Kind = "EquipItem", TargetId = i.Id, CatId = picks.SelectedCat })).Named(UiNames.Act("equip", i.Id));
                UiKit.Button(card, "Repair at staffed station", () => ctx.Act(new GameAction { Kind = "RepairItem", TargetId = i.Id })).Named(UiNames.Act("repair", i.Id));
            }
            return root;
        }
    }
}
