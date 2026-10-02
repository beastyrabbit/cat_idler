using UnityEngine;
using UnityEngine.UIElements;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The small inspector that follows the selected cat or building on screen. Mounted in slot-card and
    /// refreshed every frame while a village is selected.
    /// </summary>
    public class TetheredCard : VisualElement, IHudPart
    {
        private readonly Label heading, body;

        public TetheredCard(UiContext ctx)
        {
            Context = ctx;
            name = UiNames.Card("tethered");
            AddToClassList("tethered-inspector");
            AddToClassList("glass-panel");
            AddToClassList("interactive");
            style.display = DisplayStyle.None;
            heading = UiKit.Label(this, "", "title");
            body = UiKit.Label(this, "", "body");
            UiKit.Button(this, "Inspect", () => ctx.Hud.OpenPanel(DrawerHost.InspectId)).Named(UiNames.Act("inspect", "card"));
        }

        public UiContext Context { get; }

        public void Refresh()
        {
            var game = Context.Game;
            if (panel == null || game.View.Camera == null) return;
            var cat = game.View.SelectedCat;
            var building = game.View.SelectedBuilding;
            if (cat == null && building == null)
            {
                style.display = DisplayStyle.None;
                return;
            }
            Vector3 world;
            if (cat != null)
                world = new Vector3((float)cat.X, (float)game.CurrentWorld.WalkHeight(cat.Position) + .7f, (float)cat.Z);
            else
                world = new Vector3(building.Position.X + building.Width * .5f, (float)game.CurrentWorld.WalkHeight(building.Position) + .7f, building.Position.Z + building.Depth * .5f);
            var screen = game.View.Camera.WorldToScreenPoint(world);
            if (screen.z <= 0)
            {
                style.display = DisplayStyle.None;
                return;
            }
            var panelPoint = RuntimePanelUtils.ScreenToPanel(panel, new Vector2(screen.x, Screen.height - screen.y));
            style.left = Mathf.Clamp(panelPoint.x + 18, 20, Mathf.Max(20, Screen.width - 340));
            style.top = Mathf.Clamp(panelPoint.y - 78, 120, Mathf.Max(120, Screen.height - 190));
            style.display = DisplayStyle.Flex;
            if (cat != null)
            {
                heading.text = cat.Name;
                body.text = UiText.NowLine(game.CurrentWorld, game.Selected, cat) + (cat.BlockedReason == "" ? "" : "\nBecause " + UiText.HumanReason(cat.BlockedReason));
            }
            else
            {
                heading.text = UiText.Pretty(building.Kind);
                body.text = (building.WorkerId == "" ? "Waiting for a worker" : "Working") + (building.BlockedReason == "" ? "" : "\nBecause " + UiText.HumanReason(building.BlockedReason));
            }
        }
    }
}
