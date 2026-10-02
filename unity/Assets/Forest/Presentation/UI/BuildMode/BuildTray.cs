using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>The build tray. Wave 1 stub mounted in slot-mode; renders nothing (W3-07).</summary>
    public class BuildTray : VisualElement, IHudPart
    {
        public BuildTray(UiContext ctx)
        {
            Context = ctx;
            pickingMode = PickingMode.Ignore;
        }

        public UiContext Context { get; }

        public void Refresh() { }
    }
}
