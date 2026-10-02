using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>The controls and mode hint. Wave 1 stub mounted in slot-hint; renders nothing (W2-13).</summary>
    public class HintLine : VisualElement, IHudPart
    {
        public HintLine(UiContext ctx)
        {
            Context = ctx;
            pickingMode = PickingMode.Ignore;
        }

        public UiContext Context { get; }

        public void Refresh() { }
    }
}
