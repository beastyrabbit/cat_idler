using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.UIElements;
using Event = IdleCatForest.Simulation.Event;

namespace IdleCatForest.Presentation
{
    public enum ToastSeverity { Silent, Notable, Alert }

    /// <summary>
    /// The toast stack, top right under the resource pills. Every refresh it reads the selected village's
    /// events newer than the last <see cref="Event.Seq"/> it saw (history present when a village is first
    /// seen is not replayed) and shows notable ones: at most <see cref="MaxVisible"/>, newest at the top.
    /// Notable toasts leave after <see cref="LifeSeconds"/>; alerts stay until clicked or dismissed. A click
    /// focuses the event's cat or building (or opens its section) and dismisses the toast. They slide in and
    /// fade out, without motion under reduced motion. Severity comes from <see cref="Classify"/> until W3-06
    /// switches to <c>Event.Severity</c>.
    /// </summary>
    public class Toasts : VisualElement, IHudPart
    {
        public const string ElementName = "toasts";
        public const int MaxVisible = 3;
        public const double LifeSeconds = 6;
        /// <summary>A backlog longer than this collapses into one "N more events" toast.</summary>
        public const int MaxQueued = 4;
        public const float Width = 340;
        private const long ExitMilliseconds = 220;

        private readonly List<Toast> live = new List<Toast>();
        private readonly List<Event> queue = new List<Event>();
        private string villageId;
        private long lastSeq;

        public Toasts(UiContext ctx)
        {
            Context = ctx;
            name = ElementName;
            pickingMode = PickingMode.Ignore;
            AddToClassList("toasts");
            style.position = Position.Absolute;
            WidgetStyles.Use(this, WidgetStyles.Widgets, WidgetStyles.Toasts);
        }

        public UiContext Context { get; }

        /// <summary>The clock toasts age by: real seconds, so pausing the game does not freeze them.</summary>
        internal Func<double> Clock { get; set; } = () => Time.unscaledTimeAsDouble;

        /// <summary>The toasts on screen (leaving ones excluded), newest first.</summary>
        public IReadOnlyList<VisualElement> Visible => live.Where(t => !t.Leaving).Select(t => t.Element).ToList();

        /// <summary>
        /// The kind-to-severity table. Alerts stay until clicked (raids, deaths); notable news shows for a few
        /// seconds; routine bookkeeping (completed jobs, tithes, trade, notes) stays in the Events log.
        /// </summary>
        public static ToastSeverity Classify(string kind)
        {
            switch (kind)
            {
                case EventKind.Raid:
                case EventKind.RaidWarning:
                case EventKind.RaidBreach:
                case EventKind.RaidLoss:
                case EventKind.Death:
                case EventKind.RecoveryBlocked:
                    return ToastSeverity.Alert;
                case EventKind.Research:
                case EventKind.Birth:
                case EventKind.Pregnancy:
                case EventKind.Migration:
                case EventKind.Arrival:
                case EventKind.Expansion:
                case EventKind.Dungeon:
                case EventKind.ExpeditionLoot:
                case EventKind.Defense:
                case EventKind.RaidRepelled:
                case EventKind.BuildingCompleted:
                case EventKind.Milestone:
                case EventKind.RankUp:
                case EventKind.Upgrade:
                case EventKind.Recovery:
                    return ToastSeverity.Notable;
                default:
                    return ToastSeverity.Silent;
            }
        }

        public void Refresh()
        {
            var village = Context?.Village;
            if (village == null) return;
            if (village.Id != villageId)
            {
                // A new or switched village: its history is old news.
                villageId = village.Id;
                lastSeq = village.Events.Count == 0 ? 0 : village.Events.Max(e => e.Seq);
                queue.Clear();
                foreach (var toast in live.ToList()) Remove(toast);
                Place();
                return;
            }
            foreach (var e in village.Events.Where(e => e.Seq > lastSeq).OrderBy(e => e.Seq))
            {
                lastSeq = e.Seq;
                if (Classify(e.Kind) != ToastSeverity.Silent) queue.Add(e);
            }
            double now = Clock();
            foreach (var toast in live.ToList())
                if (!toast.Leaving && !toast.Sticky && now - toast.ShownAt >= LifeSeconds)
                    Dismiss(toast);
            Collapse(village);
            while (queue.Count > 0 && LiveCount() < MaxVisible)
            {
                var next = queue[0];
                queue.RemoveAt(0);
                Show(next, now);
            }
            // Make room for an alert, or on a short window for the stack itself, by retiring the oldest notable toast.
            if (queue.Count > 0 && Classify(queue[0].Kind) == ToastSeverity.Alert || Overflowing())
            {
                var oldest = live.LastOrDefault(t => !t.Leaving && !t.Sticky);
                if (oldest != null) Dismiss(oldest);
            }
            Place();
        }

        private int LiveCount() => live.Count(t => !t.Leaving);

        private bool Overflowing()
        {
            var root = HudLayout.Root(Context);
            if (root == null || LiveCount() <= 1 || float.IsNaN(layout.yMax) || float.IsNaN(root.layout.height)) return false;
            return layout.yMax > root.layout.height - HudLayout.EdgeMargin(Context);
        }

        // A long backlog (a catch-up, a busy minute) becomes the alerts plus one summary toast.
        private void Collapse(Village village)
        {
            if (queue.Count <= MaxQueued) return;
            var alerts = queue.Where(e => Classify(e.Kind) == ToastSeverity.Alert).ToList();
            var newest = queue.Where(e => Classify(e.Kind) != ToastSeverity.Alert).LastOrDefault();
            int hidden = queue.Count - alerts.Count - (newest != null ? 1 : 0);
            queue.Clear();
            queue.AddRange(alerts);
            if (newest != null) queue.Add(newest);
            if (hidden > 0) queue.Add(new Event { Kind = "", Text = HudCopy.ToastOverflowBody, Seq = -hidden, Time = newest?.Time ?? 0 });
        }

        private sealed class Toast
        {
            public VisualElement Element;
            public Event Event;
            public double ShownAt;
            public bool Sticky, Leaving;
        }

        private void Show(Event e, double now)
        {
            bool overflow = e.Kind == "" && e.Seq < 0;
            var severity = overflow ? ToastSeverity.Notable : Classify(e.Kind);
            var toast = new Toast { Event = e, ShownAt = now, Sticky = severity == ToastSeverity.Alert };
            var element = new VisualElement { name = overflow ? "toast-more" : "toast-" + e.Seq };
            element.AddToClassList("toast");
            element.AddToClassList("interactive");
            element.AddToClassList(severity == ToastSeverity.Alert ? "toast--alert" : "toast--notable");
            element.EnableInClassList("toast--clickable", overflow || Focusable(e));
            var stripe = new VisualElement { pickingMode = PickingMode.Ignore };
            stripe.AddToClassList("toast__stripe");
            element.Add(stripe);
            var text = new VisualElement { pickingMode = PickingMode.Ignore };
            text.AddToClassList("toast__text");
            var title = new Label(overflow ? HudCopy.ToastOverflow(-(int)e.Seq) : HudCopy.ToastTitle(e.Kind)) { pickingMode = PickingMode.Ignore };
            title.AddToClassList("toast__title");
            var body = new Label(overflow ? e.Text ?? "" : UiText.EventLine(Context.Village, e)) { pickingMode = PickingMode.Ignore };
            body.AddToClassList("toast__body");
            text.Add(title);
            if (!string.IsNullOrEmpty(e.Text)) text.Add(body);
            element.Add(text);
            if (toast.Sticky)
            {
                var close = new Button(() => Dismiss(toast)) { name = UiNames.Act("dismiss", "toast", e.Seq.ToString(System.Globalization.CultureInfo.InvariantCulture)) };
                close.AddToClassList("toast__close");
                close.Add(new Glyph(GlyphKind.Close));
                close.RegisterCallback<ClickEvent>(evt => evt.StopPropagation());
                Tooltip.Attach(close, HudCopy.Dismiss);
                element.Add(close);
            }
            element.RegisterCallback<ClickEvent>(_ => Activate(toast));
            toast.Element = element;
            live.Insert(0, toast);
            Insert(0, element);
            if (WidgetStyles.ReducedMotion) element.AddToClassList("toast--in");
            else element.schedule.Execute(() => element.AddToClassList("toast--in")).StartingIn(16);
            if (severity == ToastSeverity.Alert) UiEvents.RaiseAttention();
            else if (!overflow) UiEvents.RaiseFlourish(e.Kind);
        }

        private bool Focusable(Event e)
        {
            var village = Context?.Village;
            if (village == null) return false;
            return e.Kind == EventKind.Research || village.Cats.Any(c => c.Id == e.EntityId && c.Alive) || village.Buildings.Any(b => b.Id == e.EntityId)
                || village.Raids.Any(r => r.Id == e.EntityId);
        }

        private void Activate(Toast toast)
        {
            if (toast.Leaving) return;
            var e = toast.Event;
            Dismiss(toast);
            if (e.Kind == "" && e.Seq < 0)
            {
                Context.Hud.OpenPanel("Events");
                return;
            }
            if (e.Kind == EventKind.Research)
            {
                Context.Hud.OpenPanel("Research");
                return;
            }
            if (Focusable(e)) AttentionPopover.Focus(Context, e.EntityId, "");
        }

        /// <summary>Starts a toast's exit; it leaves the hierarchy when the fade ends.</summary>
        private void Dismiss(Toast toast)
        {
            if (toast.Leaving) return;
            toast.Leaving = true;
            toast.Element.RemoveFromClassList("toast--in");
            toast.Element.AddToClassList("toast--out");
            toast.Element.pickingMode = PickingMode.Ignore;
            toast.Element.schedule.Execute(() => Remove(toast)).StartingIn(WidgetStyles.Exit(ExitMilliseconds));
        }

        private void Remove(Toast toast)
        {
            live.Remove(toast);
            toast.Element.RemoveFromHierarchy();
        }

        /// <summary>Dismisses every toast at once (tests, village switches).</summary>
        internal void ClearToasts()
        {
            queue.Clear();
            foreach (var toast in live.ToList()) Remove(toast);
        }

        private void Place()
        {
            var root = HudLayout.Root(Context);
            if (root == null || float.IsNaN(root.layout.width)) return;
            float margin = HudLayout.EdgeMargin(Context);
            float width = Mathf.Min(Width, root.layout.width - 2 * margin);
            if (!Mathf.Approximately(style.width.value.value, width)) style.width = width;
            if (!Mathf.Approximately(style.right.value.value, margin)) style.right = margin;
            float top = HudLayout.ToastTop(Context);
            if (!Mathf.Approximately(style.top.value.value, top)) style.top = top;
        }
    }
}
