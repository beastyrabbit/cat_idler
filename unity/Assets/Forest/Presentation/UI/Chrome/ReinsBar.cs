using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>The direct-control reins bar. Wave 1 stub mounted in slot-reins; renders nothing (W3-12).</summary>
    public class ReinsBar : VisualElement, IHudPart
    {
        public ReinsBar(UiContext ctx)
        {
            Context = ctx;
            pickingMode = PickingMode.Ignore;
        }

        public UiContext Context { get; }

        public void Refresh() { }
    }
}
