using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Retired. The tethered card is the only inspector; this registry entry builds an empty body and the card
    /// closes the drawer if anything still opens it. W2-INT-b deletes the section and its registry entry.
    /// </summary>
    public sealed class InspectSection : ISection
    {
        public string Id => DrawerHost.InspectId;
        public string Label => DrawerHost.InspectId;
        public string Group => "";
        public string Icon => "";
        public bool Wide => false;

        public int Badge(UiContext ctx) => 0;

        public void Refresh(UiContext ctx) { }

        public VisualElement Build(UiContext ctx) => UiKit.SectionRoot(Id);
    }
}
