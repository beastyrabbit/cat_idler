using IdleCatForest.Simulation;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    public sealed class OfficersSection : ISection
    {
        public string Id => "Officers";
        public string Label => "Officers";
        public string Group => "Growth";
        public string Icon => "";
        public bool Wide => false;

        public int Badge(UiContext ctx) => 0;

        public void Refresh(UiContext ctx) { }

        public VisualElement Build(UiContext ctx)
        {
            var root = UiKit.SectionRoot(Id);
            var village = ctx.Village;
            var picks = ctx.Picks;
            UiKit.Text(root, "The founding Leader keeps primitive food, water and scouting work moving. Specialist offices automate their own category after the required study and building.");
            picks.CatChoice(ctx, root);
            for (int i = 0; i < Catalog.Roles.Length; i++)
            {
                string role = Catalog.Roles[i];
                var officer = village.Officers.Find(o => o.Role == role);
                var card = UiKit.Card(root, UiText.Pretty(role));
                UiKit.Label(card, officer == null ? "Vacant · manual control" : village.Cats.Find(c => c.Id == officer.CatId)?.Name ?? "Missing holder", "body");
                UiKit.Label(card, "Requires " + UiText.Pretty(Catalog.RoleBuildings[i]) + " + " + UiText.Pretty(Catalog.RoleStudies[i]), "muted");
                UiKit.Button(card, "Appoint selected cat", () => ctx.Act(new GameAction { Kind = "AssignOfficer", Role = role, CatId = picks.SelectedCat })).Named(UiNames.Act("appoint", role));
                UiKit.Button(card, "Vacate office", () => ctx.Act(new GameAction { Kind = "UnassignOfficer", Role = role })).Named(UiNames.Act("vacate", role));
            }
            return root;
        }
    }
}
