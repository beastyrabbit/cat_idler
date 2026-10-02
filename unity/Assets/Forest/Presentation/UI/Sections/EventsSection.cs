using System.Linq;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    public sealed class EventsSection : ISection
    {
        public string Id => "Events";
        public string Label => "Events";
        public string Group => "World";
        public string Icon => "";
        public bool Wide => false;

        public int Badge(UiContext ctx) => 0;

        public void Refresh(UiContext ctx) => ctx.Drawer.RebuildKeepingScroll();

        public VisualElement Build(UiContext ctx)
        {
            var root = UiKit.SectionRoot(Id);
            foreach (var e in ctx.Village.Events.AsEnumerable().Reverse().Take(100)) UiKit.Label(root, UiText.EventLine(e), "body");
            return root;
        }
    }
}
