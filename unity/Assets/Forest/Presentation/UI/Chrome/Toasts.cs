using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>The toast stack. Wave 1 stub mounted in slot-toasts; renders nothing (W2-16).</summary>
    public class Toasts : VisualElement, IHudPart
    {
        public Toasts(UiContext ctx)
        {
            Context = ctx;
            pickingMode = PickingMode.Ignore;
        }

        public UiContext Context { get; }

        public void Refresh() { }
    }
}
