using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>The placement and control mode chip. Wave 1 stub mounted in slot-mode; renders nothing (W3-07).</summary>
    public class ModeChip : VisualElement, IHudPart
    {
        public ModeChip(UiContext ctx)
        {
            Context = ctx;
            pickingMode = PickingMode.Ignore;
        }

        public UiContext Context { get; }

        public void Refresh() { }
    }
}
