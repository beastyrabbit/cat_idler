using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>The colony overview. Registered but not in today's navigation; W3-09 builds it.</summary>
    public sealed class OverviewSection : ISection
    {
        public string Id => "Overview";
        public string Label => "Overview";
        public string Group => "Colony";
        public string Icon => "";
        public bool Wide => false;

        public int Badge(UiContext ctx) => 0;

        public void Refresh(UiContext ctx) { }

        public VisualElement Build(UiContext ctx) => UiKit.SectionRoot(Id);
    }
}
