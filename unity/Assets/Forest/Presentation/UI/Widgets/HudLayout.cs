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

        /// <summary>The seal's top-left corner.</summary>
        public static Vector2 SealOrigin(UiContext ctx)
        {
            if (TryMounted(ctx, UiSlots.Chip, out var chip)) return new Vector2(chip.xMax + Gap, chip.y);
            float margin = EdgeMargin(ctx);
            if (TryLegacyTop(ctx, out var legacy)) return new Vector2(legacy.x, legacy.yMax + Gap);
            return new Vector2(margin, Compact(ctx) ? CompactMargin : 18);
        }

        /// <summary>The left edge popovers align to: the chip's, else the seal's own.</summary>
        public static float PopoverLeft(UiContext ctx, float sealLeft) => TryMounted(ctx, UiSlots.Chip, out var chip) ? chip.x : sealLeft;

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
