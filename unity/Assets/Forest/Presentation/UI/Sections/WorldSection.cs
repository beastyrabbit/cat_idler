using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    public sealed class WorldSection : ISection
    {
        public string Id => "World";
        public string Label => "World";
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
            var view = ctx.Game.View;
            var picks = ctx.Picks;
            UiKit.Text(root, world.GenerationVersion > 0 ? "Explore the forest, follow rivers and descend into the ruins. Cats carry recovered goods home." : "This saved forest keeps its original terrain. The new landscape generator is used for new worlds.");
            var levels = UiKit.Element(root, "row");
            UiKit.Button(levels, "Surface", () =>
            {
                view.ViewLevel(0);
                ctx.Drawer.Render();
            }).Named(UiNames.Act("view-depth", "0"));
            foreach (int depth in village.Known.Select(p => p.Level).Where(l => l < 0).Distinct().OrderByDescending(l => l))
            {
                int level = depth;
                UiKit.Button(levels, "Depth " + -level, () =>
                {
                    view.ViewLevel(level);
                    ctx.Drawer.Render();
                }).Named(UiNames.Act("view-depth", (-level).ToString(System.Globalization.CultureInfo.InvariantCulture)));
            }
            UiKit.Text(root, "Viewing " + (view.RenderedLevel == 0 ? "surface" : "depth " + -view.RenderedLevel) + " · Page Up / Page Down changes discovered levels");
            var terrain = UiKit.Label(root, "", "body");
            ctx.Live(() =>
            {
                var tile = world.GetTile(view.CursorTile);
                terrain.text = tile == null || !ctx.Village.Known.Contains(tile.Position) ? "Unexplored terrain" : UiText.Pretty(tile.Biome) + " · elevation " + tile.Elevation.ToString("0.0") + " m" + (tile.Water ? " · water depth " + tile.WaterDepth.ToString("0.0") + " m" : "");
            });
            picks.CatChoice(ctx, root);
            foreach (var site in world.Dungeons.Where(d => d.RumoredBy.Contains(village.Id)))
            {
                var dungeon = site;
                var card = UiKit.Card(root, site.Name);
                UiKit.Label(card, site.Depth + " levels · entrance danger " + site.Threat.ToString("0.0"), "body");
                var readiness = UiKit.Label(card, "", "body");
                ctx.Live(() =>
                {
                    var cat = ctx.Village.Cats.Find(c => c.Id == picks.SelectedCat);
                    double strength = world.DungeonReadiness(ctx.Village, cat);
                    readiness.text = cat == null ? "Choose an explorer" : cat.Name + " · readiness " + strength.ToString("0.0") + (strength < dungeon.Threat ? " · train or equip this cat first" : " · ready for the entrance");
                });
                UiKit.Button(card, "Show entrance", () => view.FocusDungeon(dungeon.Id)).Named(UiNames.Act("show-entrance", dungeon.Id));
                UiKit.Button(card, "Explore with selected cat", () => ctx.Act(new GameAction { Kind = "ExploreDungeon", CatId = picks.SelectedCat, TargetId = dungeon.Id })).Named(UiNames.Act("explore", dungeon.Id));
            }
            foreach (var cat in village.Cats.Where(c => c.Alive && world.IsDungeonExplorer(c)))
            {
                string id = cat.Id;
                var card = UiKit.Card(root, cat.Name + " · " + UiText.Pretty(cat.DungeonPhase));
                UiKit.Label(card, (cat.Position.Level == 0 ? "Surface" : "Depth " + -cat.Position.Level) + " · " + UiText.Goods(cat.Cargo), "body");
                if (cat.BlockedReason != "") UiKit.Label(card, UiText.HumanReason(cat.BlockedReason), "warning");
                UiKit.Button(card, "Follow explorer", () =>
                {
                    view.InspectCat(id);
                    ctx.Hud.OpenPanel(DrawerHost.InspectId);
                }).Named(UiNames.Act("follow", id));
                UiKit.Button(card, "Recall to village", () => ctx.Act(new GameAction { Kind = "RecallExplorer", CatId = id })).Named(UiNames.Act("recall", id));
            }
            UiKit.Text(root, "Depth increases danger. Explorers compare it with their skills, equipment, health and needs and return before taking on a stronger floor. In direct control, walk along stairs to change level and press E beside an enemy to fight.");
            return root;
        }
    }
}
