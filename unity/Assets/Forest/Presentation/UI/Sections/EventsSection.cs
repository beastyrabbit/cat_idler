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
            // Notable and alert events (W2-08 keeps routine completions in a separate log), folded and stamped, newest first.
            foreach (var g in EventLines.Collapse(ctx.Village.Events, ctx.Village).Reverse().Take(100))
                UiKit.Label(root, UiText.EventStamp(ctx.World, g.Events[g.Count - 1]) + "  " + g.Line, "body");
            return root;
        }
    }
}
