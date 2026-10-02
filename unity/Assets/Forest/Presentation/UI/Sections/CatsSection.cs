using System.Linq;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    public sealed class CatsSection : ISection
    {
        public string Id => "Cats";
        public string Label => "Cats";
        public string Group => "Colony";
        public string Icon => "";
        public bool Wide => false;

        public int Badge(UiContext ctx) => 0;

        public void Refresh(UiContext ctx) => ctx.Drawer.RebuildKeepingScroll();

        public VisualElement Build(UiContext ctx)
        {
            var root = UiKit.SectionRoot(Id);
            var village = ctx.Village;
            UiKit.Text(root, "Select a cat to inspect needs, work, preferences and carried goods.");
            foreach (var c in village.Cats.Where(c => c.Alive))
            {
                var card = UiKit.Element(root, "card");
                var row = UiKit.Element(card, "card-heading row");
                string id = c.Id;
                UiKit.Label(row, c.Name + (c.Id == village.LeaderId ? " · Leader" : ""), "subtitle");
                UiKit.Button(row, "Inspect " + c.Name, () =>
                {
                    ctx.Game.View.InspectCat(id);
                    ctx.Hud.OpenPanel(DrawerHost.InspectId);
                }).Named(UiNames.Act("inspect", id));
                UiKit.Label(card, UiText.Pretty(c.Goal) + (c.OfficerRole != "" ? " · " + UiText.Pretty(c.OfficerRole) : ""), "body");
                UiKit.Label(card, "Hunger " + c.Hunger.ToString("0") + "  Thirst " + c.Thirst.ToString("0") + "  Rest " + c.Rest.ToString("0"), "muted");
                if (c.BlockedReason != "") UiKit.Label(card, UiText.HumanReason(c.BlockedReason), "warning");
            }
            return root;
        }
    }
}
