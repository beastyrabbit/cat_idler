using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>Resource pills. Wave 1 stub mounted in slot-pills; renders nothing (W2-13).</summary>
    public class ResourcePills : VisualElement, IHudPart
    {
        public ResourcePills(UiContext ctx)
        {
            Context = ctx;
            pickingMode = PickingMode.Ignore;
        }

        public UiContext Context { get; }

        public void Refresh() { }
    }
}
