using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>Every study as cards or as a dependency map. Cards are named <c>study-&lt;id&gt;</c>.</summary>
    public sealed class ResearchSection : ISection
    {
        public const string AllCategories = "All categories";

        public string Id => "Research";
        public string Label => "Research";
        public string Group => "Growth";
        public string Icon => "";
        public bool Wide => true;

        public int Badge(UiContext ctx) => 0;

        public void Refresh(UiContext ctx) { }

        public VisualElement Build(UiContext ctx)
        {
            var root = UiKit.SectionRoot(Id);
            var picks = ctx.Picks;
            UiKit.Text(root, "All " + Catalog.Research.Count + " studies. A purchase applies its listed effects immediately; workshops still need workers, materials and queues.");
            var controls = UiKit.Element(root, "row");
            UiKit.Button(controls, "Study cards", () =>
            {
                picks.ResearchMap = false;
                ctx.Drawer.Render();
            }).Named(UiNames.Act("research-view", "cards"));
            UiKit.Button(controls, "Dependency map", () =>
            {
                picks.ResearchMap = true;
                ctx.Drawer.Render();
            }).Named(UiNames.Act("research-view", "map"));
            var categories = new[] { AllCategories }.Concat(Catalog.Research.Select(n => n.Category).Distinct());
            var category = UiKit.Choice(root, "Category", categories, picks.ResearchCategory, x => picks.ResearchCategory = x).Named("choice-research-category");
            category.RegisterValueChangedCallback(_ => ctx.Drawer.Render());
            var search = UiKit.Field(root, "Search studies", picks.Query).Named("field-search-studies");
            var researchCards = UiKit.Element(root, picks.ResearchMap ? "research-map" : "research-cards");
            search.RegisterValueChangedCallback(e =>
            {
                picks.Query = e.newValue;
                RenderCards(ctx, researchCards);
            });
            RenderCards(ctx, researchCards);
            return root;
        }

        private static void RenderCards(UiContext ctx, VisualElement parent)
        {
            var picks = ctx.Picks;
            var village = ctx.Village;
            parent.Clear();
            var nodes = Catalog.Research.Where(n => (picks.ResearchCategory == AllCategories || n.Category == picks.ResearchCategory) && (picks.Query == "" || (n.Id + " " + n.Name + " " + n.Description + " " + n.Category).IndexOf(picks.Query, StringComparison.OrdinalIgnoreCase) >= 0)).ToList();
            var positions = new Dictionary<string, Vector2>();
            VisualElement canvas = parent;
            if (picks.ResearchMap) canvas = BuildMap(parent, nodes, positions);
            foreach (var node in nodes)
            {
                var n = node;
                bool owned = village.Research.Contains(n.Id);
                bool prereqs = n.Prerequisites.All(village.Research.Contains);
                var card = UiKit.Element(canvas, "research-card").Named("study-" + n.Id);
                card.EnableInClassList("complete", owned);
                card.EnableInClassList("locked", !prereqs);
                if (picks.ResearchMap)
                {
                    var p = positions[n.Id];
                    card.style.position = Position.Absolute;
                    card.style.left = p.x;
                    card.style.top = p.y;
                    card.style.width = 300;
                    card.style.height = 310;
                }
                UiKit.Label(card, n.Category.ToUpperInvariant(), "eyebrow");
                UiKit.Label(card, n.Name, "subtitle");
                UiKit.Label(card, n.Description, "body");
                UiKit.Label(card, "Cost " + n.Cost.ToString("0") + " research · " + (owned ? "complete" : prereqs ? "prerequisites met" : "needs " + string.Join(", ", n.Prerequisites.Where(p => !village.Research.Contains(p)))), owned ? "muted" : "body");
                UiKit.Label(card, string.Join(" · ", n.Payloads.Select(p => UiText.Effect(p))), "muted");
                foreach (var prerequisite in n.Prerequisites)
                {
                    string id = prerequisite;
                    string title = Catalog.Research.Find(study => study.Id == id)?.Name ?? id;
                    UiKit.Button(card, "Requires " + title, () =>
                    {
                        picks.Query = id;
                        picks.ResearchCategory = AllCategories;
                        ctx.Drawer.RenderAtTop();
                    }).Named(UiNames.Act("requires", id));
                }
                if (!owned) UiKit.Button(card, "Research " + n.Name, () => ctx.Act(new GameAction { Kind = "ResearchNode", NodeId = n.Id })).Named(UiNames.Act("research", n.Id));
            }
        }

        /// <summary>Lays the studies out by prerequisite depth on a scrolling canvas with Bezier links.</summary>
        private static VisualElement BuildMap(VisualElement parent, List<Study> nodes, Dictionary<string, Vector2> positions)
        {
            var viewport = new ScrollView(ScrollViewMode.VerticalAndHorizontal);
            viewport.style.height = 420;
            parent.Add(viewport);
            var canvas = new VisualElement();
            viewport.Add(canvas);
            var levels = new Dictionary<string, int>();
            int Level(string id)
            {
                if (levels.TryGetValue(id, out int existing)) return existing;
                var study = Catalog.Research.Find(n => n.Id == id);
                int level = study == null || study.Prerequisites.Count == 0 ? 0 : study.Prerequisites.Max(Level) + 1;
                levels[id] = level;
                return level;
            }
            var rows = new Dictionary<int, int>();
            foreach (var node in nodes)
            {
                int level = Level(node.Id);
                rows.TryGetValue(level, out int row);
                positions[node.Id] = new Vector2(level * 330, row * 330);
                rows[level] = row + 1;
            }
            canvas.style.width = positions.Count == 0 ? 300 : positions.Values.Max(p => p.x) + 310;
            canvas.style.height = positions.Count == 0 ? 300 : positions.Values.Max(p => p.y) + 320;
            canvas.generateVisualContent += context =>
            {
                var painter = context.painter2D;
                painter.strokeColor = new Color(.55f, .63f, .39f);
                painter.lineWidth = 2;
                foreach (var node in nodes)
                    foreach (var prerequisite in node.Prerequisites)
                    {
                        if (!positions.TryGetValue(prerequisite, out var from)) continue;
                        var to = positions[node.Id];
                        from += new Vector2(300, 130);
                        to += new Vector2(0, 130);
                        painter.BeginPath();
                        painter.MoveTo(from);
                        painter.BezierCurveTo(from + Vector2.right * 20, to - Vector2.right * 20, to);
                        painter.Stroke();
                    }
            };
            return canvas;
        }
    }
}
