using System.Collections.Generic;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The section dock (GUIDE §5.4), bottom centre: one icon cell per dock section in five groups with a
    /// separator between groups, the label above the active cell or after a 150 ms hover, and a red badge
    /// with the section's attention count. Clicking a cell toggles its drawer. It never scrolls: below the
    /// narrow breakpoint the cells shrink instead. Mounted in slot-dock.
    /// </summary>
    public class Dock : VisualElement, IHudPart
    {
        public const string BarName = "hud-dock";
        public const float HoverLabelDelaySeconds = .15f;

        private readonly VisualElement bar;
        private readonly List<Cell> cells = new List<Cell>();

        public Dock(UiContext ctx)
        {
            Context = ctx;
            name = "dock-row";
            pickingMode = PickingMode.Ignore;
            AddToClassList("dock-row");
            bar = new VisualElement { name = BarName };
            bar.AddToClassList("dock");
            bar.AddToClassList("glass");
            Add(bar);
            Build();
        }

        public UiContext Context { get; }
        public VisualElement Bar => bar;

        private void Build()
        {
            bar.Clear();
            cells.Clear();
            string group = null;
            foreach (var entry in Context.Hud.Registry.Entries)
            {
                if (!entry.Navigable) continue;
                var cell = new Cell(this, entry);
                cells.Add(cell);
                if (group != null && entry.Group != group) cell.Separator = AddSeparator();
                group = entry.Group;
                bar.Add(cell.Button);
            }
            Refresh();
        }

        private VisualElement AddSeparator()
        {
            var sep = new VisualElement { pickingMode = PickingMode.Ignore };
            sep.AddToClassList("dock-sep");
            bar.Add(sep);
            return sep;
        }

        public void Refresh()
        {
            var world = Context.World;
            var village = Context.Village;
            IReadOnlyDictionary<string, int> counts = world != null && village != null ? Attention.CountBySection(world, village) : null;
            string active = Context.State.OpenSection;
            bool previousVisible = false;
            foreach (var cell in cells)
            {
                bool visible = cell.Entry.InDock;
                cell.Button.style.display = visible ? DisplayStyle.Flex : DisplayStyle.None;
                // A separator shows only between two visible groups.
                if (cell.Separator != null) cell.Separator.style.display = visible && previousVisible ? DisplayStyle.Flex : DisplayStyle.None;
                if (visible) previousVisible = true;
                cell.SetActive(cell.Entry.Id == active);
                int badge = counts != null && counts.TryGetValue(cell.Entry.Id, out int n) ? n : 0;
                cell.SetBadge(badge);
            }
        }

        private void Toggle(string id)
        {
            if (Context.State.OpenSection == id) Context.Hud.ClosePanel();
            else Context.Hud.OpenPanel(id);
            Refresh();
        }

        private sealed class Cell
        {
            private readonly Dock dock;
            private readonly Label label, badge;
            private IVisualElementScheduledItem hoverTimer;
            private bool hovered;

            public Cell(Dock dock, SectionRegistry.Entry entry)
            {
                this.dock = dock;
                Entry = entry;
                Button = new Button(() => dock.Toggle(entry.Id)) { name = UiNames.Nav(entry.Id), text = "" };
                Button.AddToClassList("dk");
                Button.AddToClassList("navigation-button");
                Button.Add(HudIcons.Glyph(entry.Icon, "dk-icon", entry.Section.Label));
                label = new Label(entry.Section.Label.ToUpperInvariant()) { pickingMode = PickingMode.Ignore };
                label.AddToClassList("dk-label");
                Button.Add(label);
                badge = new Label("") { pickingMode = PickingMode.Ignore };
                badge.AddToClassList("dk-badge");
                badge.style.display = DisplayStyle.None;
                Button.Add(badge);
                Button.RegisterCallback<PointerEnterEvent>(_ =>
                {
                    hoverTimer?.Pause();
                    hoverTimer = Button.schedule.Execute(() => { hovered = true; Button.AddToClassList("hov"); }).StartingIn((long)(HoverLabelDelaySeconds * 1000));
                });
                Button.RegisterCallback<PointerLeaveEvent>(_ =>
                {
                    hoverTimer?.Pause();
                    hovered = false;
                    Button.RemoveFromClassList("hov");
                });
            }

            public SectionRegistry.Entry Entry { get; }
            public Button Button { get; }
            public VisualElement Separator { get; set; }
            public bool Hovered => hovered;

            public void SetActive(bool on)
            {
                Button.EnableInClassList("on", on);
                Button.EnableInClassList("active", on);
            }

            public void SetBadge(int count)
            {
                string text = count <= 0 ? "" : count > 9 ? "9+" : count.ToString(System.Globalization.CultureInfo.InvariantCulture);
                if (badge.text != text) badge.text = text;
                badge.style.display = count > 0 ? DisplayStyle.Flex : DisplayStyle.None;
            }
        }
    }

    /// <summary>
    /// The HUD's icons: 24 px line glyphs from docs/ui-redesign/mockups/icons.js imported as VectorImage under
    /// Resources/UI/Icons (built by tools/ui-icons), and the resource silhouettes under Resources/UI/Icons/Goods.
    /// Both are white and tinted by USS (-unity-background-image-tint-color).
    /// </summary>
    internal static class HudIcons
    {
        private static readonly Dictionary<string, VectorImage> Glyphs = new Dictionary<string, VectorImage>();
        private static readonly Dictionary<string, Texture2D> GoodsIcons = new Dictionary<string, Texture2D>();

        public static VectorImage LoadGlyph(string glyph)
        {
            if (string.IsNullOrEmpty(glyph)) return null;
            if (!Glyphs.TryGetValue(glyph, out var image))
            {
                image = Resources.Load<VectorImage>("UI/Icons/" + glyph);
                Glyphs[glyph] = image;
            }
            return image;
        }

        public static Texture2D LoadGoods(string resource)
        {
            if (string.IsNullOrEmpty(resource)) return null;
            if (!GoodsIcons.TryGetValue(resource, out var texture))
            {
                texture = Resources.Load<Texture2D>("UI/Icons/Goods/" + resource);
                GoodsIcons[resource] = texture;
            }
            return texture;
        }

        /// <summary>An icon element; when the glyph is missing it shows the fallback's first letter instead.</summary>
        public static VisualElement Glyph(string glyph, string cls, string fallback = "")
        {
            var image = LoadGlyph(glyph);
            VisualElement icon;
            if (image != null)
            {
                icon = new VisualElement();
                icon.style.backgroundImage = new StyleBackground(image);
            }
            else icon = new Label(string.IsNullOrEmpty(fallback) ? "" : fallback.Substring(0, 1));
            icon.pickingMode = PickingMode.Ignore;
            icon.AddToClassList("ic");
            icon.AddToClassList(cls);
            return icon;
        }

        public static VisualElement Goods(string resource, string cls)
        {
            var icon = new VisualElement { pickingMode = PickingMode.Ignore };
            var texture = LoadGoods(resource);
            if (texture != null) icon.style.backgroundImage = new StyleBackground(texture);
            icon.AddToClassList(cls);
            return icon;
        }
    }
}
