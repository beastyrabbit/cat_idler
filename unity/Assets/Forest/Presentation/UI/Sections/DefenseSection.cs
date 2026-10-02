using IdleCatForest.Simulation;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    public sealed class DefenseSection : ISection
    {
        public string Id => "Defense";
        public string Label => "Defense";
        public string Group => "World";
        public string Icon => "";
        public bool Wide => false;

        public int Badge(UiContext ctx) => 0;

        public void Refresh(UiContext ctx) { }

        public VisualElement Build(UiContext ctx)
        {
            var root = UiKit.SectionRoot(Id);
            var village = ctx.Village;
            var picks = ctx.Picks;
            UiKit.Text(root, "Train warriors and appoint a Captain to organize defense.");
            picks.CatChoice(ctx, root);
            UiKit.Button(root, "Train selected cat", () => ctx.Act(new GameAction { Kind = "TrainWarrior", CatId = picks.SelectedCat })).Named(UiNames.Act("train", "warrior"));
            UiKit.Button(root, "Defend active raid", () => ctx.Act(new GameAction { Kind = "DefendRaid" })).Named(UiNames.Act("defend", "raid"));
            foreach (var raid in village.Raids) UiKit.Text(root, "Raid · health " + raid.Health.ToString("0") + " · " + raid.Position);
            if (village.Raids.Count == 0) UiKit.Text(root, "No active raids.");
            return root;
        }
    }
}
