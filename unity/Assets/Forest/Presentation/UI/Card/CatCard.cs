using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>The tethered card body for a cat. Wave 1 stub; renders nothing (W2-14).</summary>
    public class CatCard : VisualElement, IHudPart
    {
        public CatCard(UiContext ctx)
        {
            Context = ctx;
            name = UiNames.Card("cat");
            pickingMode = PickingMode.Ignore;
        }

        public UiContext Context { get; }

        public void Refresh() { }
    }
}
