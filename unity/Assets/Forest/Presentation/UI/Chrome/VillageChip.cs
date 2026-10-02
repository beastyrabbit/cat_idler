using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>Colony name, clock, counts and speed. Wave 1 stub mounted in slot-chip; renders nothing (W2-13).</summary>
    public class VillageChip : VisualElement, IHudPart
    {
        public VillageChip(UiContext ctx)
        {
            Context = ctx;
            pickingMode = PickingMode.Ignore;
        }

        public UiContext Context { get; }

        public void Refresh() { }
    }
}
