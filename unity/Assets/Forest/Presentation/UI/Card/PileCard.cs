using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>The tethered card body for a stockpile. Wave 1 stub; renders nothing (W2-14).</summary>
    public class PileCard : VisualElement, IHudPart
    {
        public PileCard(UiContext ctx)
        {
            Context = ctx;
            name = UiNames.Card("pile");
            pickingMode = PickingMode.Ignore;
        }

        public UiContext Context { get; }

        public void Refresh() { }
    }
}
