using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The hint line (GUIDE §5.10), bottom left: the controls legend for the first 20 s, then the current
    /// mode (placement or direct control), the result of the player's last action for a few seconds, and a
    /// save failure or the shared-world connection. Mounted in slot-hint.
    /// </summary>
    public class HintLine : VisualElement, IHudPart
    {
        public const string LineName = "hud-hint";
        public const float LegendSeconds = 20;
        public const float FeedbackSeconds = 6;
        /// <summary>Narrower than this beside the dock, the line moves above the dock.</summary>
        public const float MinimumRoom = 180;
        public const string Legend = "WASD pan · wheel zoom · click to inspect · Tab to control · Space pause";

        private readonly Label text;
        private float shownAt = -1;

        public HintLine(UiContext ctx)
        {
            Context = ctx;
            name = LineName;
            AddToClassList("hint");
            pickingMode = PickingMode.Ignore;
            text = new Label("") { pickingMode = PickingMode.Ignore };
            text.AddToClassList("hint-text");
            Add(text);
            Refresh();
        }

        public UiContext Context { get; }

        /// <summary>What the line says now, and whether it is a problem (shown in red).</summary>
        public (string text, bool bad) Current()
        {
            var game = Context.Game;
            var hud = Context.Hud;
            var view = game.View;
            if (Context.Village == null) return (game.Status, false);
            if (shownAt < 0) shownAt = Time.unscaledTime;
            if (hud.HasPlacement) return (hud.Placement.Label + " · click to place · Esc cancels", false);
            if (view != null && view.DirectControl)
                return ("Steering " + (view.ControlledCat?.Name ?? "a cat") + " · WASD walk · right drag look · E interact · Tab hand back", false);
            if (game.Status.StartsWith("Save failed", System.StringComparison.Ordinal) || game.Status.StartsWith("Could not", System.StringComparison.Ordinal))
                return ("Could not save the forest · it will try again", true);
            if (game.IsRemote) return (game.Status, false);
            if (game.LastAction.Length > 0 && Time.unscaledTime - hud.LastActionAt < FeedbackSeconds)
                return (UiText.HumanReason(game.LastAction), hud.LastActionFailed);
            if (Time.unscaledTime - shownAt < LegendSeconds) return (Legend, false);
            return ("", false);
        }

        public void Refresh()
        {
            var (line, bad) = Current();
            if (text.text != line) text.text = line;
            EnableInClassList("bad", bad);
            EnableInClassList("empty", line.Length == 0);
            KeepClearOfDock();
        }

        /// <summary>
        /// The line never runs under the dock: it takes the room left of the dock, or sits just above the
        /// dock when that room is too small (narrow windows).
        /// </summary>
        private void KeepClearOfDock()
        {
            var slot = parent;
            var bar = Context.Hud.Slot(UiSlots.Dock)?.Q(Dock.BarName);
            if (slot == null || bar == null || bar.resolvedStyle.display == DisplayStyle.None || float.IsNaN(bar.worldBound.width) || bar.worldBound.width <= 0)
            {
                RemoveFromClassList("above-dock");
                style.maxWidth = StyleKeyword.Null;
                style.bottom = StyleKeyword.Null;
                return;
            }
            float left = worldBound.xMin - slot.worldBound.xMin;
            float room = bar.worldBound.xMin - slot.worldBound.xMin - left - 12;
            bool above = room < MinimumRoom;
            EnableInClassList("above-dock", above);
            style.maxWidth = above ? StyleKeyword.Null : new StyleLength(room);
            style.bottom = above ? new StyleLength(slot.worldBound.yMax - bar.worldBound.yMin + 8) : StyleKeyword.Null;
        }
    }
}
