using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Where the seal, popover and toasts sit relative to the other chrome (GUIDE section 4): the seal 8
    /// points right of the village chip, the popover under the seal, toasts under the resource pills. Slots
    /// cover the root, so a slot child's layout is in root coordinates. Until the Glass chrome is mounted the
    /// positions fall back to below the legacy top bar.
    /// </summary>
    public static class HudLayout
    {
        public const float Margin = 20, CompactMargin = 12, Gap = 8;
        private const string LegacyTopBar = "hud-top";

        public static VisualElement Root(UiContext ctx) => ctx?.Hud?.Slot(UiSlots.Seal)?.parent;

        public static bool Compact(UiContext ctx)
        {
            var root = Root(ctx);
            return root != null && (root.ClassListContains("compact") || root.layout.width < 1100);
        }

        public static float EdgeMargin(UiContext ctx) => Compact(ctx) ? CompactMargin : Margin;

        /// <summary>The laid-out rectangle of the first visible element mounted in a slot.</summary>
        public static bool TryMounted(UiContext ctx, string slot, out Rect rect)
        {
            rect = default;
            var host = ctx?.Hud?.Slot(slot);
            if (host == null) return false;
            foreach (var child in host.Children())
            {
                var layout = child.layout;
                if (child.resolvedStyle.display == DisplayStyle.None || float.IsNaN(layout.width) || layout.width < 1 || layout.height < 1) continue;
                rect = layout;
                return true;
            }
            return false;
        }

        /// <summary>The legacy top bar's rectangle while it still exists (W2-13 removes it).</summary>
        public static bool TryLegacyTop(UiContext ctx, out Rect rect)
        {
            rect = default;
            var bar = Root(ctx)?.Q(LegacyTopBar);
            if (bar == null || bar.resolvedStyle.display == DisplayStyle.None || float.IsNaN(bar.layout.height) || bar.layout.height < 1) return false;
            rect = bar.layout;
            return true;
        }

        /// <summary>
        /// The legacy top bar's seal button while it exists, in root coordinates. The new seal covers it, so
        /// the player sees and clicks one seal until W2-13 removes the old bar.
        /// </summary>
        public static bool TryLegacySeal(UiContext ctx, out Rect rect)
        {
            rect = default;
            var old = LegacySeal(ctx);
            if (old == null || old.resolvedStyle.display == DisplayStyle.None || float.IsNaN(old.worldBound.width) || old.worldBound.width < 1) return false;
            rect = Root(ctx).WorldToLocal(old.worldBound);
            return true;
        }

        /// <summary>The legacy top bar's seal button (class <c>attention-seal</c>), or null once it is gone.</summary>
        public static Button LegacySeal(UiContext ctx) => Root(ctx)?.Q<Button>(className: LegacySealClass);

        private const string LegacySealClass = "attention-seal";

        /// <summary>
        /// Retires the legacy seal while it still exists: renamed to <c>seal-legacy</c> so <see cref="UiNames.Seal"/>
        /// names exactly one element, hidden and click-through so the new seal on top of it is the only one.
        /// A no-op once W2-13 removes the legacy top bar.
        /// </summary>
        public static void RetireLegacySeal(UiContext ctx)
        {
            var old = LegacySeal(ctx);
            if (old == null) return;
            old.name = "seal-legacy";
            old.pickingMode = PickingMode.Ignore;
            old.style.visibility = Visibility.Hidden;
        }

        /// <summary>The seal's top-left corner for a seal of <paramref name="size"/>.</summary>
        public static Vector2 SealOrigin(UiContext ctx, Vector2 size)
        {
            if (TryMounted(ctx, UiSlots.Chip, out var chip)) return new Vector2(chip.xMax + Gap, chip.y);
            if (float.IsNaN(size.x)) size = Vector2.zero;
            if (TryLegacySeal(ctx, out var old)) return new Vector2(old.xMax - size.x, old.center.y - size.y * .5f);
            if (TryLegacyTop(ctx, out var legacy)) return new Vector2(legacy.x, legacy.yMax + Gap);
            return new Vector2(EdgeMargin(ctx), Compact(ctx) ? CompactMargin : 18);
        }

        /// <summary>
        /// The popover's left edge: aligned with the chip, else with the seal on its side of the window
        /// (left-aligned on the left half, right-aligned on the right half), kept inside the margins.
        /// </summary>
        public static float PopoverLeft(UiContext ctx, Rect seal, float width)
        {
            var root = Root(ctx);
            float rootWidth = root == null || float.IsNaN(root.layout.width) ? seal.xMax + width : root.layout.width;
            float margin = EdgeMargin(ctx);
            float left = TryMounted(ctx, UiSlots.Chip, out var chip) ? chip.x : seal.center.x < rootWidth * .5f ? seal.x : seal.xMax - width;
            return Mathf.Clamp(left, margin, Mathf.Max(margin, rootWidth - width - margin));
        }

        /// <summary>The top of the toast stack on the right: under the pills, else under the legacy bar.</summary>
        public static float ToastTop(UiContext ctx)
        {
            if (TryMounted(ctx, UiSlots.Pills, out var pills)) return pills.yMax + Gap + 2;
            if (TryLegacyTop(ctx, out var legacy)) return legacy.yMax + Gap + 2;
            return Compact(ctx) ? 64 : 82;
        }

        public static void Place(VisualElement element, float left, float top)
        {
            if (element == null || float.IsNaN(left) || float.IsNaN(top)) return;
            if (element.style.left.keyword != StyleKeyword.Undefined || !Mathf.Approximately(element.style.left.value.value, left)) element.style.left = left;
            if (element.style.top.keyword != StyleKeyword.Undefined || !Mathf.Approximately(element.style.top.value.value, top)) element.style.top = top;
        }
    }
}
