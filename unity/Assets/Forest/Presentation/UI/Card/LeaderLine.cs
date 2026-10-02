using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>The line from the tethered card to its subject. Wave 1 stub; renders nothing (W2-14).</summary>
    public class LeaderLine : VisualElement, IHudPart
    {
        public LeaderLine(UiContext ctx)
        {
            Context = ctx;
            pickingMode = PickingMode.Ignore;
        }

        public UiContext Context { get; }

        public void Refresh() { }
    }
}
