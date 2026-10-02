using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>The tethered card body for a building. Wave 1 stub; renders nothing (W2-14).</summary>
    public class BuildingCard : VisualElement, IHudPart
    {
        public BuildingCard(UiContext ctx)
        {
            Context = ctx;
            name = UiNames.Card("building");
            pickingMode = PickingMode.Ignore;
        }

        public UiContext Context { get; }

        public void Refresh() { }
    }
}
