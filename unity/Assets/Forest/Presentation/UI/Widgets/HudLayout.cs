using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Where the seal, popover and toasts sit relative to the other chrome (GUIDE section 4): the seal 8
    /// points right of the village chip, the popover under the seal, toasts under the resource pills. Slots
    /// cover the root, so a slot child's layout is in root coordinates. Before the chrome has laid out the
    /// positions fall back to the edge margins.
    /// </summary>
    public static class HudLayout
    {
        public const float Margin = 20, CompactMargin = 12, Gap = 8;

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

        /// <summary>The seal's top-left corner for a seal of <paramref name="size"/>.</summary>
        public static Vector2 SealOrigin(UiContext ctx, Vector2 size)
        {
            if (TryMounted(ctx, UiSlots.Chip, out var chip)) return new Vector2(chip.xMax + Gap, chip.y);
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

        /// <summary>The top of the toast stack on the right: under the pills, else at the default chrome height.</summary>
        public static float ToastTop(UiContext ctx)
        {
            if (TryMounted(ctx, UiSlots.Pills, out var pills)) return pills.yMax + Gap + 2;
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
