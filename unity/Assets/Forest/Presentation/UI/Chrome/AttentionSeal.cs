using System;
using System.Collections.Generic;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The attention seal (GUIDE section 5.2), mounted in slot-seal right of the village chip: a bell, a red
    /// dot while any item is a danger or warning, the count and a short label. Hovering shows "Attention"
    /// above it; a click toggles the popover through <see cref="UiState.PopoverOpen"/>. A new urgent item
    /// pulses the dot once (not under reduced motion) and raises <see cref="UiEvents.Attention"/>.
    /// </summary>
    public class AttentionSeal : VisualElement, IHudPart
    {
        public const string UssClass = "seal-host";
        public const string OpenClass = "seal--open";
        public const string EmptyClass = "seal--empty";
        public const string UrgentClass = "seal--urgent";
        public const string PulseClass = "seal__dot--pulse";

        private readonly Button button;
        private readonly VisualElement dot;
        private readonly Label count, words, tip;
        private readonly HashSet<string> urgentSeen = new HashSet<string>();
        private bool primed;

        public AttentionSeal(UiContext ctx)
        {
            Context = ctx;
            name = "attention-seal";
            pickingMode = PickingMode.Ignore;
            AddToClassList(UssClass);
            style.position = Position.Absolute;

            button = new Button(Toggle) { name = UiNames.Seal };
            button.AddToClassList("seal");
            button.AddToClassList("interactive");
            var bell = new VisualElement { pickingMode = PickingMode.Ignore };
            bell.AddToClassList("seal__bell");
            bell.Add(new Glyph(GlyphKind.Bell));
            dot = new VisualElement { name = "seal-dot", pickingMode = PickingMode.Ignore };
            dot.AddToClassList("seal__dot");
            bell.Add(dot);
            count = new Label("0") { name = "seal-count", pickingMode = PickingMode.Ignore };
            count.AddToClassList("seal__count");
            words = new Label(HudCopy.SealLabel(0)) { name = "seal-label", pickingMode = PickingMode.Ignore };
            words.AddToClassList("seal__label");
            tip = new Label(HudCopy.AttentionLabel) { pickingMode = PickingMode.Ignore };
            tip.AddToClassList("seal__tip");
            button.Add(bell);
            button.Add(count);
            button.Add(words);
            button.Add(tip);
            Add(button);

            WidgetStyles.Use(this, WidgetStyles.Widgets, WidgetStyles.Attention);
            if (ctx?.State != null) ctx.State.Changed += SyncOpen;
            // Sits 8 points right of the village chip (HudLayout.SealOrigin).
            RegisterCallback<AttachToPanelEvent>(_ => schedule.Execute(Place));
            RegisterCallback<GeometryChangedEvent>(_ => Place());
        }

        public UiContext Context { get; }

        /// <summary>The seal button (named <see cref="UiNames.Seal"/>).</summary>
        public Button Button => button;

        /// <summary>The number the seal shows.</summary>
        public int Count { get; private set; }

        public void Refresh()
        {
            var items = AttentionFeed.Items(Context);
            Count = items.Count;
            count.SetText(HudCopy.AttentionCount(Count));
            words.SetText(HudCopy.SealLabel(Count));
            bool urgent = false;
            bool fresh = false;
            var current = new HashSet<string>();
            foreach (var item in items)
            {
                if (item.Severity < AttentionSeverity.Warning) continue;
                urgent = true;
                current.Add(item.Id);
                if (!urgentSeen.Contains(item.Id) && primed) fresh = true;
            }
            urgentSeen.Clear();
            urgentSeen.UnionWith(current);
            primed = true;
            button.EnableInClassList(EmptyClass, Count == 0);
            button.EnableInClassList(UrgentClass, urgent);
            dot.SetVisible(urgent);
            if (fresh) Pulse();
            SyncOpen();
            Place();
        }

        private void Toggle()
        {
            if (Context?.State == null) return;
            Context.State.PopoverOpen = !Context.State.PopoverOpen;
        }

        private void SyncOpen() => button.EnableInClassList(OpenClass, Context?.State?.PopoverOpen == true);

        private void Pulse()
        {
            UiEvents.RaiseAttention();
            if (WidgetStyles.ReducedMotion) return;
            dot.AddToClassList(PulseClass);
            dot.schedule.Execute(() => dot.RemoveFromClassList(PulseClass)).StartingIn(420);
        }

        private void Place()
        {
            if (panel == null) return;
            var origin = HudLayout.SealOrigin(Context, layout.size);
            HudLayout.Place(this, origin.x, origin.y);
        }
    }

    /// <summary>
    /// One attention computation shared by the seal, the popover and anything else on the same frame:
    /// recomputed when the world, village, simulated time or revision changes.
    /// </summary>
    public static class AttentionFeed
    {
        private static readonly IReadOnlyList<AttentionItem> None = Array.Empty<AttentionItem>();
        private static World world;
        private static string villageId = "";
        private static double time = double.NaN;
        private static int revision = -1;
        private static IReadOnlyList<AttentionItem> items = None;

        public static IReadOnlyList<AttentionItem> Items(UiContext ctx)
        {
            var w = ctx?.Game != null ? ctx.World : null;
            var v = w != null ? ctx.Village : null;
            if (w == null || v == null) return None;
            int rev = ctx.Game.Revision;
            if (ReferenceEquals(w, world) && v.Id == villageId && w.TimeSeconds.Equals(time) && rev == revision) return items;
            try { items = Attention.Compute(w, v); }
            catch (Exception e)
            {
                Debug.LogException(e);
                items = None;
            }
            world = w;
            villageId = v.Id;
            time = w.TimeSeconds;
            revision = rev;
            return items;
        }

        /// <summary>Forgets the cached list, so the next read recomputes (tests that edit the world in place).</summary>
        public static void Invalidate()
        {
            world = null;
            items = None;
        }
    }
}
