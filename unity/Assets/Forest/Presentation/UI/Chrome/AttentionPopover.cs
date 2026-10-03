using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The attention popover (GUIDE section 5.2) under the seal: "Needs attention" with the count, up to six
    /// rows most urgent first (severity, then oldest), each with a severity dot, a title, a cause line, one
    /// verb-first action (only the first row's is the sun primary) and a Show pin that selects and pans to the
    /// item's subject. A 7th row opens Events; with nothing to do it names the last solved item. Opens and
    /// closes through <see cref="UiState.PopoverOpen"/>; while open, Escape at Popover priority, a click
    /// outside and any row action close it. Rows are keyed by item id, so they persist and fade out when
    /// their condition clears. <c>ForestUI.OpenPanel("Attention")</c> opens it too (tools and captures).
    /// </summary>
    public class AttentionPopover : VisualElement, IHudPart
    {
        public const string PanelId = "Attention";
        public const string ElementName = "attention-popover";
        public const int MaxRows = 6;
        public const float Width = 470;
        private const long CloseAfterSuccessMilliseconds = 700;
        private const long RowFadeMilliseconds = 150;

        private readonly VisualElement caret;
        private readonly Label subtitle, empty;
        private readonly Table<AttentionItem> rows;
        private readonly Button more;
        private readonly Dictionary<string, double> firstSeen = new Dictionary<string, double>();
        private readonly Dictionary<string, string> lastTitles = new Dictionary<string, string>();
        private List<AttentionItem> shown = new List<AttentionItem>();
        private string lastSolved = "", lastSolvedAt = "";
        private IDisposable escape, worldPress;
        private VisualElement trickleRoot;
        private bool open;
        private int rowLimit = MaxRows;

        public AttentionPopover(UiContext ctx)
        {
            Context = ctx;
            name = ElementName;
            AddToClassList("pop");
            AddToClassList("interactive");
            style.position = Position.Absolute;
            style.display = DisplayStyle.None;

            caret = new VisualElement { pickingMode = PickingMode.Ignore };
            caret.AddToClassList("pop__caret");
            Add(caret);

            var header = new VisualElement { pickingMode = PickingMode.Ignore };
            header.AddToClassList("pop__header");
            var title = new Label(HudCopy.AttentionHeader) { pickingMode = PickingMode.Ignore };
            title.AddToClassList("pop__title");
            subtitle = new Label("") { name = "attention-popover-count", pickingMode = PickingMode.Ignore };
            subtitle.AddToClassList("pop__subtitle");
            var key = new Label(HudCopy.EscapeKey) { pickingMode = PickingMode.Ignore };
            key.AddToClassList("pop__key");
            header.Add(title);
            header.Add(subtitle);
            header.Add(key);
            Add(header);

            rows = new Table<AttentionItem>(() => shown.Select(i => (i.Id, i)), MakeRow, BindRow) { name = "attention-rows" };
            rows.AddToClassList("pop__rows");
            rows.Leave = (element, done) =>
            {
                element.AddToClassList("pop-row--leaving");
                element.schedule.Execute(done).StartingIn(WidgetStyles.Exit(RowFadeMilliseconds));
            };
            Add(rows);

            more = new Button(OpenEvents) { name = UiNames.Act("attention", "more") };
            more.AddToClassList("pop__more");
            more.style.display = DisplayStyle.None;
            Add(more);

            empty = new Label("") { name = "attention-empty", pickingMode = PickingMode.Ignore };
            empty.AddToClassList("pop__empty");
            Add(empty);

            WidgetStyles.Use(this, WidgetStyles.Widgets, WidgetStyles.Attention);
            if (ctx?.State != null) ctx.State.Changed += OnStateChanged;
            RegisterCallback<AttachToPanelEvent>(_ => HookRoot());
            RegisterCallback<DetachFromPanelEvent>(_ => UnhookRoot());
            RegisterCallback<GeometryChangedEvent>(_ => FitRows());
        }

        public UiContext Context { get; }
        public bool IsOpen => open;

        /// <summary>The items currently shown as rows, in display order.</summary>
        public IReadOnlyList<AttentionItem> Shown => shown;

        /// <summary>The row table (keyed by item id).</summary>
        public Table<AttentionItem> Rows => rows;

        public void Refresh()
        {
            var items = Order(AttentionFeed.Items(Context));
            TrackSolved(items);
            shown = items.Take(rowLimit).ToList();
            if (!open) return;
            rows.Refresh();
            for (int i = 0; i < rows.Keys.Count; i++)
                if (rows.RowOf(rows.Keys[i])?.userData is RowParts parts)
                    parts.Action.Primary = i == 0;
            subtitle.SetText(HudCopy.AttentionCount(items.Count) + " · " + HudCopy.AttentionOrder);
            int hidden = items.Count - shown.Count;
            more.SetText(HudCopy.More(hidden));
            more.SetVisible(hidden > 0);
            empty.SetText(HudCopy.Empty(lastSolved, lastSolvedAt));
            empty.SetVisible(items.Count == 0);
            Place();
        }

        public void Open()
        {
            if (Context?.State != null && !Context.State.PopoverOpen)
            {
                Context.State.PopoverOpen = true;
                return;
            }
            if (open) return;
            open = true;
            rowLimit = MaxRows;
            style.display = DisplayStyle.Flex;
            BringToFront();
            Refresh();
            var router = InputRouter.Current;
            if (router != null)
            {
                escape = router.Bind(InputIntent.Escape, InputPriority.Popover, () =>
                {
                    if (!open) return false;
                    Close();
                    return true;
                });
                worldPress = router.BindPointer(PointerAction.PrimaryPress, InputPriority.Popover, _ =>
                {
                    if (open) Close();
                    return false;
                });
            }
            RemoveFromClassList("pop--shown");
            schedule.Execute(() => EnableInClassList("pop--shown", open));
            UiEvents.RaiseOpened();
        }

        public void Close()
        {
            if (Context?.State != null && Context.State.PopoverOpen)
            {
                Context.State.PopoverOpen = false;
                return;
            }
            if (!open) return;
            open = false;
            escape?.Dispose();
            worldPress?.Dispose();
            escape = worldPress = null;
            RemoveFromClassList("pop--shown");
            style.display = DisplayStyle.None;
            UiEvents.RaiseClosed();
        }

        private void OnStateChanged()
        {
            var state = Context.State;
            if (state.OpenSection == PanelId)
            {
                // OpenPanel("Attention") routes here: drop the empty drawer page, then open the popover.
                schedule.Execute(() =>
                {
                    if (state.OpenSection == PanelId) Context.Hud.ClosePanel();
                    state.PopoverOpen = true;
                });
            }
            if (state.PopoverOpen) Open();
            else Close();
        }

        // ---- rows ------------------------------------------------------------------------------

        private sealed class RowParts
        {
            public AttentionItem Item;
            public VisualElement Dot;
            public Label Title, Cause;
            public ActionButton Action;
            public Button Pin;
        }

        private VisualElement MakeRow(AttentionItem item)
        {
            var row = new VisualElement();
            row.AddToClassList("pop-row");
            var parts = new RowParts();
            row.userData = parts;
            parts.Dot = new VisualElement { pickingMode = PickingMode.Ignore };
            parts.Dot.AddToClassList("pop-row__dot");
            row.Add(parts.Dot);
            var text = new VisualElement { pickingMode = PickingMode.Ignore };
            text.AddToClassList("pop-row__text");
            parts.Title = new Label { pickingMode = PickingMode.Ignore };
            parts.Title.AddToClassList("pop-row__title");
            parts.Cause = new Label { pickingMode = PickingMode.Ignore };
            parts.Cause.AddToClassList("pop-row__cause");
            text.Add(parts.Title);
            text.Add(parts.Cause);
            row.Add(text);
            var acts = new VisualElement { pickingMode = PickingMode.Ignore };
            acts.AddToClassList("pop-row__acts");
            parts.Action = new ActionButton("", () => Perform(parts));
            parts.Action.AddToClassList("pop-row__act");
            parts.Pin = new Button(() => Show(parts));
            parts.Pin.AddToClassList("pop-row__pin");
            parts.Pin.Add(new Glyph(GlyphKind.Pin));
            Tooltip.Attach(parts.Pin, HudCopy.Show);
            acts.Add(parts.Action);
            acts.Add(parts.Pin);
            row.Add(acts);
            return row;
        }

        private void BindRow(VisualElement row, AttentionItem item)
        {
            var parts = (RowParts)row.userData;
            parts.Item = item;
            string key = Safe(item.Id);
            row.name = "attention-row-" + key;
            parts.Dot.EnableInClassList("pop-row__dot--danger", item.Severity == AttentionSeverity.Critical);
            parts.Dot.EnableInClassList("pop-row__dot--warning", item.Severity == AttentionSeverity.Warning);
            parts.Dot.EnableInClassList("pop-row__dot--info", item.Severity < AttentionSeverity.Warning);
            var world = Context.World;
            var village = Context.Village;
            parts.Title.SetText(HudCopy.Title(item, world, village));
            string cause = HudCopy.Cause(item, world, village);
            parts.Cause.SetText(cause);
            parts.Cause.SetVisible(cause != "");
            string action = HudCopy.Action(item, world, village);
            parts.Action.Label = action;
            parts.Action.ActionKind = item.ActionKind;
            parts.Action.ConfirmedLabel = HudCopy.Confirmed(item);
            parts.Action.name = UiNames.Act("attend", key);
            parts.Action.SetVisible(action != "" && item.ActionKind != "");
            parts.Pin.name = UiNames.Act("show", key);
            parts.Pin.SetVisible(item.Anchor != "" || item.Section != "");
        }

        private async Task<ActionResult> Perform(RowParts parts)
        {
            var item = parts.Item;
            if (item == null || item.ActionKind == "") return ActionResult.Fail(HudCopy.Failed);
            if (item.ActionKind == AttentionAction.OpenSection)
            {
                Close();
                if (item.Section != "") Context.Hud.OpenPanel(item.Section);
                return ActionResult.Ok();
            }
            var action = new GameAction
            {
                Kind = item.ActionKind,
                CatId = item.Arg(AttentionArg.Cat),
                BuildingId = item.Arg(AttentionArg.Building),
                NodeId = item.Arg(AttentionArg.Study),
                Name = item.Arg(AttentionArg.Job),
            };
            var result = await Context.Send(action);
            AttentionFeed.Invalidate();
            if (result != null && result.Success) schedule.Execute(Close).StartingIn(CloseAfterSuccessMilliseconds);
            return result;
        }

        /// <summary>The pin: close, then select and pan to the item's subject (or open its section).</summary>
        private void Show(RowParts parts)
        {
            var item = parts.Item;
            if (item == null) return;
            Close();
            Focus(Context, item.Anchor, item.Section);
        }

        /// <summary>Selects what an id names in the selected village: a cat or building opens its card; anything else pans home.</summary>
        internal static void Focus(UiContext ctx, string anchor, string section)
        {
            var village = ctx?.Village;
            if (village == null) return;
            if (!string.IsNullOrEmpty(anchor))
            {
                if (village.Cats.Any(c => c.Id == anchor && c.Alive))
                {
                    ctx.Hud.Select(new Subject(SubjectKind.Cat, anchor));
                    return;
                }
                if (village.Buildings.Any(b => b.Id == anchor))
                {
                    ctx.Hud.Select(new Subject(SubjectKind.Building, anchor));
                    return;
                }
                if (village.Stockpiles.Any(p => p.Id == anchor))
                {
                    ctx.Hud.Select(new Subject(SubjectKind.Pile, anchor));
                    return;
                }
                ctx.Game.View?.FocusVillage();
                return;
            }
            if (!string.IsNullOrEmpty(section)) ctx.Hud.OpenPanel(section);
        }

        private void OpenEvents()
        {
            Close();
            Context.Hud.OpenPanel("Events");
        }

        // ---- order, solved tracking and placement ---------------------------------------------

        private List<AttentionItem> Order(IReadOnlyList<AttentionItem> items)
        {
            double now = Time.unscaledTimeAsDouble;
            var live = new HashSet<string>();
            foreach (var item in items)
            {
                live.Add(item.Id);
                if (!firstSeen.ContainsKey(item.Id)) firstSeen[item.Id] = now;
            }
            foreach (var gone in firstSeen.Keys.Where(k => !live.Contains(k)).ToList()) firstSeen.Remove(gone);
            return items.Select((item, index) => (item, index))
                .OrderByDescending(x => x.item.Severity)
                .ThenBy(x => firstSeen[x.item.Id])
                .ThenBy(x => x.index)
                .Select(x => x.item)
                .ToList();
        }

        private void TrackSolved(List<AttentionItem> items)
        {
            var world = Context.World;
            var village = Context.Village;
            if (world == null || village == null) return;
            var live = new HashSet<string>(items.Select(i => i.Id));
            foreach (var pair in lastTitles)
                if (!live.Contains(pair.Key))
                {
                    lastSolved = pair.Value;
                    lastSolvedAt = UiText.Clock(world.TimeSeconds, world.CurrentPacing().DayScale);
                }
            lastTitles.Clear();
            foreach (var item in items) lastTitles[item.Id] = HudCopy.Title(item, world, village);
        }

        private void Place()
        {
            var root = HudLayout.Root(Context);
            var seal = parent?.Q<AttentionSeal>();
            if (root == null || seal == null) return;
            var sealRect = seal.layout;
            if (float.IsNaN(sealRect.width) || sealRect.width < 1) return;
            float margin = HudLayout.EdgeMargin(Context);
            float width = Mathf.Min(Width, root.layout.width - 2 * margin);
            if (!Mathf.Approximately(style.width.value.value, width)) style.width = width;
            float left = HudLayout.PopoverLeft(Context, sealRect, width);
            // Below both the seal and the chip it is aligned with, so the chip's census line stays readable.
            float top = sealRect.yMax;
            if (HudLayout.TryMounted(Context, UiSlots.Chip, out var chip)) top = Mathf.Max(top, chip.yMax);
            HudLayout.Place(this, left, top + 10);
            float caretLeft = Mathf.Clamp(sealRect.center.x - left - 6, 16, width - 28);
            if (!Mathf.Approximately(caret.style.left.value.value, caretLeft)) caret.style.left = caretLeft;
        }

        // On a short window, drop rows (they move into "N more") until the popover ends above the bottom margin.
        private void FitRows()
        {
            var root = HudLayout.Root(Context);
            if (!open || root == null || float.IsNaN(layout.yMax) || float.IsNaN(root.layout.height)) return;
            if (layout.yMax <= root.layout.height - HudLayout.EdgeMargin(Context) || shown.Count <= 1) return;
            rowLimit = shown.Count - 1;
            schedule.Execute(Refresh);
        }

        // ---- click outside ---------------------------------------------------------------------

        private void HookRoot()
        {
            UnhookRoot();
            trickleRoot = WidgetStyles.TopRoot(this);
            trickleRoot?.RegisterCallback<PointerDownEvent>(OnRootPointerDown, TrickleDown.TrickleDown);
        }

        private void UnhookRoot()
        {
            trickleRoot?.UnregisterCallback<PointerDownEvent>(OnRootPointerDown, TrickleDown.TrickleDown);
            trickleRoot = null;
        }

        private void OnRootPointerDown(PointerDownEvent evt)
        {
            if (!open) return;
            for (var e = evt.target as VisualElement; e != null; e = e.parent)
                if (e == this || e is AttentionSeal)
                    return;
            Close();
        }

        private static string Safe(string id) => (id ?? "").Replace(':', '-');
    }
}
