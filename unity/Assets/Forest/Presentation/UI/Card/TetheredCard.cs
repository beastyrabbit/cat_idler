using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Threading.Tasks;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The only inspector (GUIDE 5.5): one glass card for the selected cat, building or pile, tied to its subject by a
    /// <see cref="LeaderLine"/>. It shows a portrait, the name, one line of facts, the now line and its cause, then the
    /// subject's body (<see cref="CatCard"/>, <see cref="BuildingCard"/>, <see cref="PileCard"/>) with up to three
    /// actions and "More…". It sits 120 right of and 180 above the anchor, mirrors and drops near the edges, stays
    /// inside the margins, above the dock and beside an open drawer, re-places when the camera pans or zooms and keeps
    /// still while its subject walks. Below 1100 panel units it becomes a full-width bottom sheet. It hides in direct
    /// control and during placement. Mounted in slot-card and refreshed every frame.
    /// </summary>
    public class TetheredCard : VisualElement, IHudPart
    {
        /// <summary>Preferred offset of the card's top-left corner from the anchor, in panel units.</summary>
        public const float Right = 120, Above = 180;
        /// <summary>How far below the anchor a dropped card starts.</summary>
        public const float Below = 44;
        /// <summary>Gap between the card and a drawer or other chrome it must not overlap.</summary>
        public const float Gap = 12;
        /// <summary>Panel widths below this use the compact card (300 wide, 44 px rings) and 12 px margins.</summary>
        public const float CompactBelow = 1600;
        /// <summary>Panel widths below this turn the card into a full-width bottom sheet without a leader line.</summary>
        public const float SheetBelow = 1100;

        private readonly VisualElement head, titles;
        private readonly Label title, facts, now, why;
        private readonly Button pin;
        private readonly ScrollView scroll;
        private readonly CardPortrait portrait;
        private readonly LeaderLine leader;
        private readonly CatCard catBody;
        private readonly BuildingCard buildingBody;
        private readonly PileCard pileBody;
        private CardBody active;
        private Subject shown;
        private bool placed, anchorVisible, sheet;
        private Vector2 anchor, lastSize, lastArea;
        private Vector3 cameraPosition;
        private Quaternion cameraRotation;
        private float cameraSize;
        private Rect lastBlocked, cardRect;

        public TetheredCard(UiContext ctx)
        {
            Context = ctx;
            name = UiNames.Card("tethered");
            AddToClassList("tethered-card");
            AddToClassList("glass-panel");
            AddToClassList("interactive");
            AddToClassList("card-hidden");
            var sheet = Resources.Load<StyleSheet>("UI/Card");
            if (sheet != null) styleSheets.Add(sheet);
            style.position = Position.Absolute;

            head = UiKit.Element(this, "card-head");
            portrait = new CardPortrait();
            head.Add(portrait);
            titles = UiKit.Element(head, "card-titles");
            title = UiKit.Label(titles, "", "card-name").Named("card-name");
            facts = UiKit.Label(titles, "", "card-facts").Named("card-facts");
            pin = UiKit.Button(head, "Off screen ↗", () => Context.Game.View?.FocusSubject(shown)).Named(UiNames.Act("pin", "card"));
            pin.AddToClassList("card-pin");
            var close = UiKit.Button(head, "×", () => Context.Hud.Select(Subject.None)).Named(UiNames.Act("close", "card"));
            close.AddToClassList("card-close");
            // Everything under the header scrolls when the card meets its height limit (70 % of the window, or the
            // space between the chrome on a short one), so actions are never clipped away.
            scroll = new ScrollView(ScrollViewMode.Vertical) { name = "card-scroll" };
            scroll.AddToClassList("card-scroll");
            scroll.horizontalScrollerVisibility = ScrollerVisibility.Hidden;
            Add(scroll);
            now = UiKit.Label(scroll, "", "card-now").Named("card-now");
            why = UiKit.Label(scroll, "", "card-why").Named("card-why");

            catBody = new CatCard(ctx);
            buildingBody = new BuildingCard(ctx);
            pileBody = new PileCard(ctx);
            foreach (var body in new CardBody[] { catBody, buildingBody, pileBody })
            {
                body.AddToClassList("card-hidden");
                scroll.Add(body);
            }
            leader = new LeaderLine(ctx);
            RegisterCallback<AttachToPanelEvent>(_ => AttachLeader());
            RegisterCallback<DetachFromPanelEvent>(_ => leader.RemoveFromHierarchy());
        }

        public UiContext Context { get; }

        /// <summary>The line to the subject (a sibling just below the card in its slot).</summary>
        public LeaderLine Leader => leader;

        /// <summary>The subject the card shows, or None while it is hidden.</summary>
        public Subject Shown => shown;

        /// <summary>True while the card is visible.</summary>
        public bool Showing => !ClassListContains("card-hidden");

        /// <summary>The anchor in the card's slot coordinates and whether it is on screen.</summary>
        public Vector2 Anchor => anchor;

        /// <summary>Where the card was last placed, in slot coordinates.</summary>
        public Rect Placement => cardRect;
        public bool AnchorVisible => anchorVisible;

        /// <summary>True while the panel is narrower than <see cref="SheetBelow"/> and the card is a bottom sheet.</summary>
        public bool IsSheet => sheet;

        /// <summary>Tests pin the anchor to a slot point (for example a screen corner) instead of the subject's.</summary>
        internal Vector2? AnchorOverride { get; set; }

        internal CardBody ActiveBody => active;
        internal CardPortrait Portrait => portrait;

        public void Refresh()
        {
            var game = Context.Game;
            var view = game.View;
            AttachLeader();
            var subject = view != null ? view.SelectedSubject : Subject.None;
            var body = BodyFor(subject.Kind);
            bool hide = panel == null || parent == null || game.Selected == null || view == null || view.Camera == null || body == null
                || view.DirectControl || Context.Hud.HasPlacement;
            if (!hide && subject != shown) Show(subject, body);
            if (!hide) { active.Refresh(); hide = !active.Exists; }
            if (hide) { Hide(); return; }

            SetText(title, active.Title);
            SetText(facts, active.Facts);
            SetText(now, active.Now);
            SetText(why, active.Why);
            why.style.display = active.Why == "" ? DisplayStyle.None : DisplayStyle.Flex;
            why.EnableInClassList("card-problem", active.WhyIsProblem);
            portrait.Show(view, active.PortraitEntity, Initial(active.Title), subject.Kind == SubjectKind.Cat);

            if (AnchorOverride.HasValue)
            {
                anchor = AnchorOverride.Value;
                anchorVisible = parent.contentRect.Contains(anchor);
            }
            else anchorVisible = view.TryGetAnchor(subject, out var world) && ToSlot(view.Camera, world, out anchor);
            Place(view.Camera);
            pin.style.display = anchorVisible ? DisplayStyle.None : DisplayStyle.Flex;
            if (anchorVisible && placed && !sheet) leader.Show(NearEdge(), anchor);
            else leader.Hide();
        }

        private CardBody BodyFor(SubjectKind kind)
        {
            switch (kind)
            {
                case SubjectKind.Cat: return catBody;
                case SubjectKind.Building: return buildingBody;
                case SubjectKind.Pile: return pileBody;
                default: return null;
            }
        }

        private void Show(Subject subject, CardBody body)
        {
            if (active != null && active != body) active.AddToClassList("card-hidden");
            active = body;
            active.RemoveFromClassList("card-hidden");
            active.Bind(subject.Id);
            shown = subject;
            placed = false;
            RemoveFromClassList("card-hidden");
        }

        private void Hide()
        {
            AddToClassList("card-hidden");
            leader.Hide();
            shown = Subject.None;
            placed = false;
        }

        private void AttachLeader()
        {
            if (parent == null || leader.parent == parent && parent.IndexOf(leader) == parent.IndexOf(this) - 1) return;
            leader.RemoveFromHierarchy();
            parent.Insert(parent.IndexOf(this), leader);
        }

        /// <summary>World point to the card's slot coordinates; false when behind the camera or outside the slot.</summary>
        private bool ToSlot(Camera camera, Vector3 world, out Vector2 point)
        {
            point = default;
            var screen = camera.WorldToScreenPoint(world);
            if (screen.z <= 0 || panel == null) return false;
            // pixelHeight is the screen height for the game camera, and the texture height when it renders offscreen.
            var panelPoint = RuntimePanelUtils.ScreenToPanel(panel, new Vector2(screen.x, camera.pixelHeight - screen.y));
            point = parent.WorldToLocal(panelPoint);
            var area = parent.contentRect;
            return area.width > 0 && area.Contains(point);
        }

        private void Place(Camera camera)
        {
            var area = parent.contentRect;
            if (area.width <= 0 || area.height <= 0 || float.IsNaN(area.width)) return;
            bool compact = area.width < CompactBelow;
            sheet = area.width < SheetBelow;
            EnableInClassList("card-compact", compact);
            EnableInClassList("card-sheet", sheet);
            float margin = compact ? 12 : 20;
            var usable = Usable(area, margin);
            // GUIDE: "More…" may grow the card to 70 % of the window, never past the chrome.
            style.maxHeight = Mathf.Min(area.height * .7f, Mathf.Max(160, usable.height));
            if (sheet) style.width = area.width - 24;
            else style.width = StyleKeyword.Null;
            var size = CardSize(compact);
            if (sheet) size.x = area.width - 24;
            var (hard, soft) = Blocked();
            var blocked = hard.Concat(soft).ToList();
            var blockedUnion = blocked.Count == 0 ? Rect.zero : blocked.Aggregate(Union);
            bool cameraMoved = (camera.transform.position - cameraPosition).sqrMagnitude > 1e-6f || Quaternion.Angle(camera.transform.rotation, cameraRotation) > .01f
                || Mathf.Abs((camera.orthographic ? camera.orthographicSize : camera.fieldOfView) - cameraSize) > 1e-4f;
            var current = new Rect(cardRect.position, size);
            bool covers = !sheet && placed && anchorVisible && Inflate(current, 8).Contains(anchor);
            bool replace = !placed || !sheet && cameraMoved || covers || (size - lastSize).sqrMagnitude > 1 || (area.size - lastArea).sqrMagnitude > 1 || blockedUnion != lastBlocked;
            if (!replace) return;
            cameraPosition = camera.transform.position;
            cameraRotation = camera.transform.rotation;
            cameraSize = camera.orthographic ? camera.orthographicSize : camera.fieldOfView;
            lastSize = size; lastArea = area.size; lastBlocked = blockedUnion;
            Vector2 at;
            bool fits;
            if (sheet)
            {
                // Below 1100 the card is a bottom sheet without a leader line, hidden while a drawer is open.
                at = new Vector2(area.xMin + 12, Mathf.Max(usable.yMin, usable.yMax - size.y));
                fits = hard.Count == 0;
            }
            else
            {
                var target = anchorVisible ? anchor : new Vector2(usable.xMax - size.x - Right, usable.center.y);
                if (!anchorVisible && placed) { at = Clamp(new Vector2(current.x, current.y), size, usable); fits = !hard.Any(b => new Rect(at, size).Overlaps(b)); }
                else fits = PlaceNear(target, size, usable, blocked, out at) || PlaceNear(target, size, usable, hard, out at);
            }
            // No room beside an open drawer: the card waits hidden (keeping its subject) until there is.
            EnableInClassList("card-hidden", !fits);
            if (!fits) { placed = false; return; }
            style.left = at.x;
            style.top = at.y;
            cardRect = new Rect(at, size);
            placed = true;
        }

        private Vector2 CardSize(bool compact)
        {
            float width = resolvedStyle.width, height = resolvedStyle.height;
            if (float.IsNaN(width) || width <= 0) width = compact ? 300 : 340;
            if (float.IsNaN(height) || height <= 0) height = 260;
            return new Vector2(width, height);
        }

        /// <summary>The slot area the card may use: inside the margin, below the top chrome and above the dock.</summary>
        private Rect Usable(Rect area, float margin)
        {
            float top = area.yMin + margin, bottom = area.yMax - margin;
            var root = Context.Hud.Slot(UiSlots.Card)?.parent ?? parent;
            foreach (var element in Chrome(root, UiSlots.Chip, UiSlots.Pills, "hud-top"))
                top = Mathf.Max(top, parent.WorldToLocal(element.worldBound).yMax + 8);
            foreach (var element in Chrome(root, UiSlots.Dock, "hud-bottom"))
                bottom = Mathf.Min(bottom, parent.WorldToLocal(element.worldBound).yMin - 8);
            if (bottom - top < 160) { top = area.yMin + margin; bottom = area.yMax - margin; }
            return Rect.MinMaxRect(area.xMin + margin, top, area.xMax - margin, bottom);
        }

        /// <summary>Visible chrome under the given slots (their children) or names.</summary>
        private static IEnumerable<VisualElement> Chrome(VisualElement root, params string[] names)
        {
            foreach (var key in names)
            {
                var element = root.Q(key);
                if (element == null) continue;
                var parts = key.StartsWith("slot-", StringComparison.Ordinal) ? element.Children() : new[] { element };
                foreach (var part in parts)
                    if (Visible(part)) yield return part;
            }
        }

        /// <summary>
        /// Rectangles the card avoids: the open drawer (never overlapped; with no room beside it the card waits) and,
        /// as a soft obstacle, a legacy navigation column (overlapped only when nothing else fits).
        /// </summary>
        private (List<Rect> hard, List<Rect> soft) Blocked()
        {
            var hard = new List<Rect>();
            var soft = new List<Rect>();
            var drawer = Context.Hud.Drawer?.Element;
            if (drawer != null && Context.State.OpenSection != "" && Visible(drawer)) hard.Add(parent.WorldToLocal(drawer.worldBound));
            var root = Context.Hud.Slot(UiSlots.Card)?.parent ?? parent;
            var navigation = root.Q("hud-nav");
            if (navigation != null && Visible(navigation)) soft.Add(parent.WorldToLocal(navigation.worldBound));
            return (hard, soft);
        }

        private static bool Visible(VisualElement e)
        {
            if (e.resolvedStyle.display == DisplayStyle.None || e.resolvedStyle.visibility == Visibility.Hidden) return false;
            var r = e.worldBound;
            return r.width > 1 && r.height > 1 && !float.IsNaN(r.width);
        }

        /// <summary>The card edge facing the anchor, 60 below the card's top (GUIDE 5.5).</summary>
        private Vector2 NearEdge()
        {
            var r = cardRect;
            float x = anchor.x >= r.center.x ? r.xMax : r.xMin;
            float y = Mathf.Clamp(r.yMin + 60, r.yMin + 16, r.yMax - 16);
            return new Vector2(x, y);
        }

        /// <summary>
        /// GUIDE 5.5 placement. Prefers 120 right of and 180 above <paramref name="anchor"/>; mirrors to the left when the
        /// right edge would be crossed, drops below when the top would be crossed; clamps into <paramref name="usable"/>;
        /// never overlaps a <paramref name="blocked"/> rectangle (it slides beside one instead) and avoids covering the
        /// anchor. False when no position fits (the card hides).
        /// </summary>
        public static bool PlaceNear(Vector2 anchor, Vector2 size, Rect usable, IReadOnlyList<Rect> blocked, out Vector2 topLeft)
        {
            float right = anchor.x + Right, left = anchor.x - Right - size.x;
            float above = anchor.y - Above, below = anchor.y + Below;
            float x = right + size.x <= usable.xMax ? right : left;
            float y = above >= usable.yMin ? above : below;
            float otherX = x == right ? left : right, otherY = y == above ? below : above;
            var candidates = new List<Vector2> { new Vector2(x, y), new Vector2(otherX, y), new Vector2(x, otherY), new Vector2(otherX, otherY) };
            foreach (var rect in blocked)
            {
                candidates.Add(new Vector2(rect.xMax + Gap, y));
                candidates.Add(new Vector2(rect.xMax + Gap, otherY));
            }
            Vector2? fallback = null;
            foreach (var candidate in candidates)
            {
                var at = Clamp(candidate, size, usable);
                var r = new Rect(at, size);
                if (blocked.Any(b => Overlaps(r, b))) continue;
                if (!Inflate(r, 8).Contains(anchor)) { topLeft = at; return true; }
                fallback ??= at;
            }
            topLeft = fallback ?? default;
            return fallback.HasValue;
        }

        private static Vector2 Clamp(Vector2 at, Vector2 size, Rect usable) =>
            new Vector2(Mathf.Clamp(at.x, usable.xMin, Mathf.Max(usable.xMin, usable.xMax - size.x)), Mathf.Clamp(at.y, usable.yMin, Mathf.Max(usable.yMin, usable.yMax - size.y)));

        private static bool Overlaps(Rect a, Rect b) => a.Overlaps(b);

        private static Rect Inflate(Rect r, float by) => new Rect(r.x - by, r.y - by, r.width + by * 2, r.height + by * 2);

        private static Rect Union(Rect a, Rect b) => Rect.MinMaxRect(Mathf.Min(a.xMin, b.xMin), Mathf.Min(a.yMin, b.yMin), Mathf.Max(a.xMax, b.xMax), Mathf.Max(a.yMax, b.yMax));

        private static string Initial(string text) => string.IsNullOrEmpty(text) ? "?" : text.Substring(0, 1).ToUpperInvariant();

        internal static void SetText(Label label, string text)
        {
            text ??= "";
            if (label.text != text) label.text = text;
        }
    }

    /// <summary>
    /// One card body: the subject's facts for the shared header, its rings or key-value block, up to three actions and
    /// "More…". A body rebuilds its elements only when its <see cref="Shape"/> changes and updates values in
    /// <see cref="Update"/>, so buttons keep hover and focus between frames.
    /// </summary>
    public abstract class CardBody : VisualElement, IHudPart
    {
        private string shape;
        private string confirming = "";

        protected CardBody(UiContext ctx, string kind)
        {
            Context = ctx;
            name = UiNames.Card(kind);
            AddToClassList("card-body");
        }

        public UiContext Context { get; }
        public string SubjectId { get; private set; } = "";
        public string Title { get; protected set; } = "";
        public string Facts { get; protected set; } = "";
        public string Now { get; protected set; } = "";
        public string Why { get; protected set; } = "";
        public bool WhyIsProblem { get; protected set; }
        public string PortraitEntity { get; protected set; } = "";
        public abstract bool Exists { get; }

        /// <summary>Whether "More…" (or an inline editor) is expanded.</summary>
        public bool MoreOpen { get; private set; }

        protected World World => Context.World;
        protected Village Village => Context.Village;

        internal void Bind(string id)
        {
            SubjectId = id ?? "";
            MoreOpen = false;
            confirming = "";
            shape = null;
            OnBind();
        }

        /// <summary>Resets body-specific state when the card shows a new subject.</summary>
        protected virtual void OnBind() { }

        public void Refresh()
        {
            if (Village == null || World == null || !Exists) return;
            Describe();
            string next = Shape() + "|" + MoreOpen + "|" + confirming;
            if (next != shape)
            {
                shape = next;
                Clear();
                Build();
            }
            Update();
        }

        /// <summary>Sets the header strings from the subject.</summary>
        protected abstract void Describe();

        /// <summary>A string that changes exactly when the body's elements must be rebuilt.</summary>
        protected abstract string Shape();

        protected abstract void Build();

        /// <summary>Per-frame values on the elements Build made.</summary>
        protected virtual void Update() { }

        protected void ToggleMore() { MoreOpen = !MoreOpen; shape = null; }

        protected void Rebuild() => shape = null;

        /// <summary>The action row; actions after the third are ignored by design (GUIDE: up to three plus More…).</summary>
        protected VisualElement Actions() => UiKit.Element(this, "card-actions");

        /// <summary>A button that sends one action; W2-16's ActionButton shows the result in place.</summary>
        protected ActionButton Act(VisualElement parent, string label, string elementName, Func<GameAction> action, bool primary = false)
        {
            var button = new ActionButton(label, () => Send(action()));
            button.name = elementName;
            button.AddToClassList("card-act");
            if (primary) button.AddToClassList("card-primary");
            parent.Add(button);
            return button;
        }

        protected Button Plain(VisualElement parent, string label, string elementName, Action click, bool primary = false)
        {
            var button = UiKit.Button(parent, label, click).Named(elementName);
            button.AddToClassList("card-act");
            if (primary) button.AddToClassList("card-primary");
            return button;
        }

        protected Button MoreButton(VisualElement actions, string label = "More…")
        {
            var button = Plain(actions, MoreOpen ? "Less" : label, UiNames.Act("more", "card"), ToggleMore);
            button.AddToClassList("card-quiet");
            return button;
        }

        /// <summary>The expandable area under the actions; the card's own scroll view keeps it inside the height limit.</summary>
        protected VisualElement MoreArea()
        {
            var more = new VisualElement { name = "card-more" };
            more.AddToClassList("card-more");
            if (!MoreOpen) more.style.display = DisplayStyle.None;
            Add(more);
            return more;
        }

        /// <summary>
        /// A destructive action asked once inline (GUIDE 5.5): the button becomes "<paramref name="question"/> Yes · No".
        /// Elements: <c>act-&lt;verb&gt;-&lt;id&gt;</c>, then <c>act-&lt;verb&gt;-yes-&lt;id&gt;</c> and <c>act-&lt;verb&gt;-no-&lt;id&gt;</c>.
        /// </summary>
        protected void Destructive(VisualElement parent, string label, string question, string verb, string id, Func<GameAction> action)
        {
            string key = verb + ":" + id;
            if (confirming != key)
            {
                var ask = Plain(parent, label, UiNames.Act(verb, id), () => { confirming = key; shape = null; });
                ask.AddToClassList("card-danger");
                return;
            }
            var row = UiKit.Element(parent, "card-confirm");
            UiKit.Label(row, question, "card-confirm-text");
            var yes = new ActionButton("Yes", async () =>
            {
                var result = await Send(action());
                confirming = "";
                shape = null;
                return result;
            }) { name = UiNames.Act(verb, "yes", id) };
            yes.AddToClassList("card-act");
            yes.AddToClassList("card-danger");
            row.Add(yes);
            Plain(row, "No", UiNames.Act(verb, "no", id), () => { confirming = ""; shape = null; }).AddToClassList("card-quiet");
        }

        /// <summary>Sends an action; the body re-reads the subject once the result is in.</summary>
        protected async Task<ActionResult> Send(GameAction action)
        {
            var result = await Context.Send(action);
            shape = null;
            return result;
        }

        /// <summary>A key-value block row (GUIDE .kv): an uppercase dim key and a value.</summary>
        protected Label Row(VisualElement kv, string key, string value, string elementName = null, bool problem = false)
        {
            var row = UiKit.Element(kv, "card-kv-row");
            UiKit.Label(row, key, "card-kv-key");
            var label = UiKit.Label(row, value, "card-kv-value");
            if (elementName != null) label.name = elementName;
            label.EnableInClassList("card-problem", problem);
            return label;
        }

        protected VisualElement Kv() => UiKit.Element(this, "card-kv");

        protected static ProgressBar Progress(VisualElement parent, string elementName)
        {
            var bar = new ProgressBar { name = elementName, lowValue = 0, highValue = 1 };
            bar.AddToClassList("card-progress");
            parent.Add(bar);
            return bar;
        }

        protected static void SetProgress(ProgressBar bar, double value, double total, string text)
        {
            if (bar == null) return;
            float high = (float)Math.Max(1e-6, total);
            if (Math.Abs(bar.highValue - high) > 1e-6f) bar.highValue = high;
            float v = (float)Math.Max(0, Math.Min(total, value));
            if (Math.Abs(bar.value - v) > 1e-6f) bar.value = v;
            if (bar.title != text) bar.title = text;
        }
    }

    /// <summary>Invariant-culture numbers and short durations for the card (W2-15 may move these into UiText).</summary>
    internal static class CardFormat
    {
        public static string Whole(double v) => Math.Round(v).ToString("#,0", CultureInfo.InvariantCulture);

        public static string Amount(double v) => Math.Abs(v - Math.Round(v)) < .05 ? Whole(v) : v.ToString("#,0.#", CultureInfo.InvariantCulture);

        /// <summary>"45 s", "3 min", "2 h 10 min".</summary>
        public static string Duration(double seconds)
        {
            if (double.IsNaN(seconds) || double.IsInfinity(seconds)) return "";
            seconds = Math.Max(0, seconds);
            if (seconds < 60) return Whole(Math.Ceiling(seconds)) + " s";
            if (seconds < 3600) return Whole(Math.Ceiling(seconds / 60)) + " min";
            int hours = (int)(seconds / 3600), minutes = (int)Math.Round((seconds - hours * 3600) / 60);
            return hours + " h" + (minutes > 0 ? " " + minutes + " min" : "");
        }

        public static string Count(int n, string one, string many) => n.ToString(CultureInfo.InvariantCulture) + " " + (n == 1 ? one : many);

        /// <summary>Sentence case: first letter upper.</summary>
        public static string Sentence(string text) => string.IsNullOrEmpty(text) ? "" : char.ToUpperInvariant(text[0]) + text.Substring(1);

        /// <summary>A building's display name, numbered when the village has more than one of its kind ("Den 2").</summary>
        public static string BuildingName(Village v, Building b)
        {
            string name = Sentence(UiText.Name(NameKind.Building, b.Kind));
            var same = v.Buildings.Where(x => x.Kind == b.Kind).ToList();
            return same.Count > 1 ? name + " " + (same.IndexOf(b) + 1).ToString(CultureInfo.InvariantCulture) : name;
        }

        public static string Resource(string id) => Sentence(UiText.Name(NameKind.Resource, id));

        /// <summary>"3 logs, 2 stone" from stacks, or "nothing".</summary>
        public static string Goods(IEnumerable<Stack> goods)
        {
            var list = goods.Where(s => s.Amount > 1e-6).Select(s => Amount(s.Amount) + " " + UiText.Name(NameKind.Resource, s.Resource)).ToList();
            return list.Count == 0 ? "nothing" : string.Join(", ", list);
        }
    }
}
