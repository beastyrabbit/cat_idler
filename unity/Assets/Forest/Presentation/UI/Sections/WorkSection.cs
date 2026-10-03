using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    public sealed class WorkSection : ISection
    {
        private static readonly string[] ManualLabors = { "hunt", "fetch_water", "woodcut", "quarry", "forage", "fish", "replant", "scout", "train" };
        private static readonly string[] Crops = { "grain", "catnip", "herbs" };

        public string Id => "Work";
        public string Label => "Work";
        public string Group => "Production";
        public string Icon => "";
        public bool Wide => false;

        public int Badge(UiContext ctx) => 0;

        public void Refresh(UiContext ctx) { }

        public VisualElement Build(UiContext ctx)
        {
            var root = UiKit.SectionRoot(Id);
            var village = ctx.Village;
            var picks = ctx.Picks;
            UiKit.Text(root, "Manual jobs remain available while an officer's seat is vacant. Personal needs can interrupt work.");
            picks.CatChoice(ctx, root, true);
            foreach (var labor in ManualLabors)
            {
                string name = labor;
                string job = name == "woodcut" ? "logs" : name == "forage" ? "fibre" : name;
                UiKit.Button(root, UiText.Pretty(name), () => ctx.Act(new GameAction { Kind = name == "scout" ? "DispatchScout" : "RequestJob", Name = job, CatId = picks.SelectedCat })).Named(UiNames.Act("job", name));
            }
            UiKit.Button(root, "Haul goods · choose source", () => ctx.Hud.OpenPanel("Stores")).Named(UiNames.Act("open", "stores"));
            UiKit.Button(root, "Research work · choose workplace", () =>
            {
                var hut = ctx.Village.Buildings.FirstOrDefault(b => b.Kind == "research_hut");
                if (hut == null)
                {
                    ctx.Hud.OpenPanel("Build");
                    return;
                }
                ctx.Hud.Select(new Subject(SubjectKind.Building, hut.Id));
            }).Named(UiNames.Act("open", "research-work"));
            UiKit.Separator(root);
            UiKit.Label(root, "Farms and gathering", "subtitle");
            picks.ResourceChoice(root);
            UiKit.Button(root, "Quarry selected resource", () => ctx.Act(new GameAction { Kind = "RequestJob", Name = "quarry", CatId = picks.SelectedCat, Resource = picks.SelectedResource })).Named(UiNames.Act("quarry", "resource"));
            UiKit.Button(root, "Scout for selected resource", () => ctx.Act(new GameAction { Kind = "DispatchScout", CatId = picks.SelectedCat, Resource = picks.SelectedResource })).Named(UiNames.Act("scout", "resource"));
            foreach (var crop in Crops)
            {
                string id = crop;
                UiKit.Button(root, "Designate " + crop + " farm", () => ctx.BeginPlacement("Exterior " + id + " farm: choose opposite corners", p => new GameAction { Kind = "DesignateFarm", Name = id, Resource = id, Position = p, End = p }, true)).Named(UiNames.Placement("farm-" + id));
            }
            picks.ResourceChoice(root);
            UiKit.Button(root, "Designate gathering", () => ctx.BeginPlacement("Gathering area: choose opposite corners on known exterior land", p => new GameAction { Kind = "DesignateGatherSpot", Position = p, Resource = picks.SelectedResource }, true)).Named(UiNames.Placement("gather-spot"));
            UiKit.Button(root, "Designate fishing", () => ctx.BeginPlacement("Choose a known dry shore tile beside water", p => new GameAction { Kind = "DesignateFishingSpot", Position = p, Resource = "fish" })).Named(UiNames.Placement("fishing-spot"));
            foreach (var farm in village.Farms)
            {
                var f = farm;
                var card = UiKit.Card(root, UiText.Pretty(f.Crop) + " · " + UiText.Pretty(f.Phase));
                if (f.BlockedReason != "") UiKit.Label(card, UiText.HumanReason(f.BlockedReason), "warning");
                UiKit.Button(card, "Assign selected cat", () => ctx.Act(new GameAction { Kind = "AssignWorker", CatId = picks.SelectedCat, TargetId = f.Id })).Named(UiNames.Act("assign-farm", f.Id));
                UiKit.Button(card, "Clear farm", () => ctx.Act(new GameAction { Kind = "ClearFarm", TargetId = f.Id })).Named(UiNames.Act("clear-farm", f.Id));
            }
            UiKit.Separator(root);
            UiKit.Label(root, "Workplaces", "subtitle");
            foreach (var b in village.Buildings)
            {
                string id = b.Id;
                UiKit.Button(root, UiText.Name(ctx.Village, b) + " · " + (b.WorkerId == "" ? "vacant" : "staffed"), () =>
                {
                    ctx.Hud.Select(new Subject(SubjectKind.Building, id));
                }).Named(UiNames.Act("inspect", id));
            }
            foreach (var job in village.Jobs.Where(j => !j.Completed))
            {
                string id = job.Id;
                UiKit.Text(root, UiText.Pretty(job.Kind) + " · " + UiText.Pretty(job.Phase));
                if (job.BlockedReason != "") UiKit.Text(root, UiText.HumanReason(job.BlockedReason), "warning");
                UiKit.Button(root, "Help " + UiText.Pretty(job.Kind), () => ctx.Act(new GameAction { Kind = "Boost", TargetId = id })).Named(UiNames.Act("boost", id));
            }
            return root;
        }
    }
}
