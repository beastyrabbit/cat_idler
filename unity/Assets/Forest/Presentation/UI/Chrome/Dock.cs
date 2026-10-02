using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>The section dock. Wave 1 stub mounted in slot-dock; renders nothing (W2-13).</summary>
    public class Dock : VisualElement, IHudPart
    {
        public Dock(UiContext ctx)
        {
            Context = ctx;
            pickingMode = PickingMode.Ignore;
        }

        public UiContext Context { get; }

        public void Refresh() { }
    }
}
