using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>The attention popover. Wave 1 stub mounted in slot-seal; renders nothing (W2-16).</summary>
    public class AttentionPopover : VisualElement, IHudPart
    {
        public AttentionPopover(UiContext ctx)
        {
            Context = ctx;
            pickingMode = PickingMode.Ignore;
        }

        public UiContext Context { get; }

        public void Refresh() { }
    }
}
