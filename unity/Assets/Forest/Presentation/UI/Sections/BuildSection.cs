using System;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    public sealed class BuildSection : ISection
    {
        public string Id => "Build";
        public string Label => "Build";
        public string Group => "Production";
        public string Icon => "";
        public bool Wide => false;

        public int Badge(UiContext ctx) => 0;

        public void Refresh(UiContext ctx) { }

        public VisualElement Build(UiContext ctx)
        {
            var root = UiKit.SectionRoot(Id);
            var village = ctx.Village;
            UiKit.Text(root, "Choose a clear 2 × 2 site inside the village. Its entrance needs a road connected to the shrine. Cats carry materials to the site; farms belong outside the fence.");
            UiKit.Label(root, "Paths and space", "subtitle");
            var paths = UiKit.Element(root, "row");
            UiKit.Button(paths, "Build stone road", () => ctx.BeginPlacement("Road: choose start, then end", p => new GameAction { Kind = "BuildRoad", Position = p }, true)).Named(UiNames.Placement("road"));
            UiKit.Button(paths, "Build bridge", () => ctx.BeginPlacement("Place bridge", p => new GameAction { Kind = "BuildBridge", Position = p })).Named(UiNames.Placement("bridge"));
            UiKit.Button(root, "Expand village", () => ctx.Act(new GameAction { Kind = "RequestJob", Name = "expand" })).Named(UiNames.Act("expand", "village"));
            UiKit.Separator(root);
            UiKit.Label(root, "Buildings", "subtitle");
            foreach (var kind in Catalog.Buildings.Where(k => k != "shrine"))
            {
                string id = kind;
                bool available = Catalog.BuildingAvailable(village, id);
                var card = UiKit.Card(root, UiText.Pretty(id));
                UiKit.Label(card, available ? "Available" : "Research required", "muted");
                string label = "Place " + UiText.Pretty(id) + (id == "field" ? " on known exterior land" : " on clear land inside the village");
                UiKit.Button(card, "Place " + UiText.Pretty(id), () => ctx.BeginPlacement(label, p => new GameAction { Kind = "PlanBuilding", Name = id, Position = p })).Named(UiNames.Placement(id));
            }
            UiKit.Separator(root);
            UiKit.Label(root, "Gathering and boundaries", "subtitle");
            UiKit.Button(root, "Mark gathering zone", () => ctx.BeginPlacement("Gathering zone: choose corners", p => new GameAction { Kind = "CreateZone", Position = p, Resource = "gather" }, true)).Named(UiNames.Placement("zone-gather"));
            UiKit.Button(root, "Mark avoid zone", () => ctx.BeginPlacement("Avoid zone: choose corners", p => new GameAction { Kind = "CreateZone", Position = p, Resource = "avoid" }, true)).Named(UiNames.Placement("zone-avoid"));
            foreach (var zone in village.Stockpiles.Where(p => p.Kind.StartsWith("zone_", StringComparison.Ordinal)))
            {
                string id = zone.Id;
                UiKit.Button(root, "Remove " + zone.Kind + " " + zone.Position, () => ctx.Act(new GameAction { Kind = "RemoveZone", TargetId = id })).Named(UiNames.Act("remove-zone", id));
            }
            return root;
        }
    }
}
