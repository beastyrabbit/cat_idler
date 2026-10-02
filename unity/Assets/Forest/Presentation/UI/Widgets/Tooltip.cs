using System;
using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Tooltips that work in the player. Runtime panels never draw <see cref="VisualElement.tooltip"/>, so
    /// <see cref="Attach"/> adds a <see cref="TooltipManipulator"/>: after <see cref="DelayMilliseconds"/> of hover
    /// one shared glass label (per HUD root) appears near the pointer, clamped inside the root's layout. The
    /// text is computed when it shows, so it can describe live state.
    /// </summary>
    public static class Tooltip
    {
        public const long DelayMilliseconds = 350;
        public const string ElementName = "hud-tooltip";
        public const string UssClass = "hud-tooltip";
        public const string VisibleClass = "hud-tooltip--visible";
        private const float OffsetX = 14, OffsetY = 20, Margin = 8, MaxWidth = 320;

        public static void Attach(VisualElement element, Func<string> text)
        {
            if (element == null || text == null) return;
            // Keep the native property empty: in the Editor it would show a second tooltip.
            element.tooltip = "";
            element.AddManipulator(new TooltipManipulator(text));
        }

        public static void Attach(VisualElement element, string text)
        {
            if (!string.IsNullOrEmpty(text)) Attach(element, () => text);
        }

        /// <summary>The shared label under the element's HUD root, made on first use.</summary>
        public static Label Shared(VisualElement anyElement)
        {
            var root = WidgetStyles.TopRoot(anyElement);
            if (root == null) return null;
            var label = root.Q<Label>(ElementName);
            if (label != null) return label;
            label = new Label { name = ElementName, pickingMode = PickingMode.Ignore };
            label.AddToClassList(UssClass);
            label.style.position = Position.Absolute;
            label.style.display = DisplayStyle.None;
            label.RegisterCallback<GeometryChangedEvent>(_ => Clamp(label));
            root.Add(label);
            WidgetStyles.Use(label);
            return label;
        }

        /// <summary>Shows the shared label with <paramref name="text"/> near a panel-space pointer position.</summary>
        public static void Show(VisualElement anchor, string text, Vector2 panelPointer)
        {
            if (anchor?.panel == null || string.IsNullOrEmpty(text)) return;
            var label = Shared(anchor);
            if (label == null) return;
            var root = label.parent;
            var local = root.WorldToLocal(panelPointer);
            float room = root.layout.width - 2 * Margin;
            if (!float.IsNaN(room) && room > 0) label.style.maxWidth = Mathf.Min(MaxWidth, room);
            label.userData = anchor;
            if (label.text != text) label.text = text;
            label.style.left = local.x + OffsetX;
            label.style.top = local.y + OffsetY;
            label.style.display = DisplayStyle.Flex;
            label.AddToClassList(VisibleClass);
            label.BringToFront();
            Clamp(label);
        }

        /// <summary>Hides the shared label if <paramref name="anchor"/> is the element it currently describes.</summary>
        public static void Hide(VisualElement anchor)
        {
            if (anchor?.panel == null) return;
            var label = WidgetStyles.TopRoot(anchor)?.Q<Label>(ElementName);
            if (label == null || label.userData != anchor) return;
            label.userData = null;
            label.RemoveFromClassList(VisibleClass);
            label.style.display = DisplayStyle.None;
        }

        /// <summary>Keeps the label inside the root, flipping above the pointer near the bottom edge.</summary>
        internal static void Clamp(Label label)
        {
            var root = label.parent;
            if (root == null) return;
            var bounds = root.layout;
            var size = label.layout.size;
            if (float.IsNaN(size.x) || float.IsNaN(bounds.width) || size.x <= 0) return;
            float left = label.style.left.value.value, top = label.style.top.value.value;
            float clampedLeft = Mathf.Clamp(left, Margin, Mathf.Max(Margin, bounds.width - size.x - Margin));
            float clampedTop = top + size.y + Margin > bounds.height ? top - size.y - OffsetY - OffsetY * .5f : top;
            clampedTop = Mathf.Clamp(clampedTop, Margin, Mathf.Max(Margin, bounds.height - size.y - Margin));
            if (!Mathf.Approximately(clampedLeft, left)) label.style.left = clampedLeft;
            if (!Mathf.Approximately(clampedTop, top)) label.style.top = clampedTop;
        }
    }

    /// <summary>Shows <see cref="Tooltip"/>'s shared label after a short hover over its target.</summary>
    public sealed class TooltipManipulator : Manipulator
    {
        private readonly Func<string> text;
        private IVisualElementScheduledItem pending;
        private Vector2 pointer;
        private bool showing;

        public TooltipManipulator(Func<string> text) => this.text = text;

        protected override void RegisterCallbacksOnTarget()
        {
            target.RegisterCallback<PointerEnterEvent>(OnEnter);
            target.RegisterCallback<PointerMoveEvent>(OnMove);
            target.RegisterCallback<PointerLeaveEvent>(OnLeave);
            target.RegisterCallback<PointerDownEvent>(OnDown);
            target.RegisterCallback<DetachFromPanelEvent>(OnDetach);
        }

        protected override void UnregisterCallbacksFromTarget()
        {
            target.UnregisterCallback<PointerEnterEvent>(OnEnter);
            target.UnregisterCallback<PointerMoveEvent>(OnMove);
            target.UnregisterCallback<PointerLeaveEvent>(OnLeave);
            target.UnregisterCallback<PointerDownEvent>(OnDown);
            target.UnregisterCallback<DetachFromPanelEvent>(OnDetach);
            Cancel();
        }

        /// <summary>Shows the tooltip at once at a panel position, as if the delay had passed.</summary>
        public void ShowNow(Vector2 panelPointer)
        {
            pointer = panelPointer;
            Show();
        }

        private void OnEnter(PointerEnterEvent evt)
        {
            pointer = evt.position;
            pending?.Pause();
            pending = target.schedule.Execute(Show).StartingIn(Tooltip.DelayMilliseconds);
        }

        private void OnMove(PointerMoveEvent evt)
        {
            pointer = evt.position;
            if (showing) Show();
        }

        private void OnLeave(PointerLeaveEvent evt) => Cancel();

        private void OnDown(PointerDownEvent evt) => Cancel();

        private void OnDetach(DetachFromPanelEvent evt) => Cancel();

        private void Show()
        {
            string value;
            try { value = text(); }
            catch (Exception e)
            {
                Debug.LogException(e);
                value = "";
            }
            if (string.IsNullOrEmpty(value)) return;
            showing = true;
            Tooltip.Show(target, value, pointer);
        }

        private void Cancel()
        {
            pending?.Pause();
            pending = null;
            if (showing) Tooltip.Hide(target);
            showing = false;
        }
    }
}
