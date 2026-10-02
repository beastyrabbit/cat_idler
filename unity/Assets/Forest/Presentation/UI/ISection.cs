using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>One drawer section. The drawer builds it on open and calls Refresh on its 0.5 s cadence.</summary>
    public interface ISection
    {
        /// <summary>The id ForestUI.OpenPanel takes ("Cats", "Research", ...).</summary>
        string Id { get; }
        string Label { get; }
        string Group { get; }
        string Icon { get; }

        /// <summary>A wide section fills the window instead of the side drawer.</summary>
        bool Wide { get; }

        VisualElement Build(UiContext ctx);
        void Refresh(UiContext ctx);
        int Badge(UiContext ctx);
    }
}
