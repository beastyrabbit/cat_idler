using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>Selection and inspection entry points, direct-control entry and the selection outline.</summary>
    public sealed partial class ForestView
    {
        public string SelectedCatId { get; private set; } = "";
        public string SelectedBuildingId { get; private set; } = "";
        public Cat SelectedCat => Game.Selected?.Cats.Find(c => c.Id == SelectedCatId);
        public Building SelectedBuilding => Game.Selected?.Buildings.Find(b => b.Id == SelectedBuildingId);
        private bool followSelectedCat;
        private LineRenderer selection;

        private void CreateSelectionMarker()
        {
            selection = new GameObject("Selected place").AddComponent<LineRenderer>();
            selection.material = Material("selection", new Color(1, .78f, .38f), true);
            selection.widthMultiplier = .045f; selection.loop = true; selection.positionCount = 4;
            selection.shadowCastingMode = UnityEngine.Rendering.ShadowCastingMode.Off;
            selection.receiveShadows = false;
        }

        public void InspectCat(string id) { SelectedCatId = id; SelectedBuildingId = ""; followSelectedCat = true; var cat = SelectedCat; if (cat != null && !DirectControl) { viewedLevel = cat.Position.Level; focus = Position(cat); nextReconcile = 0; } }
        public void InspectBuilding(string id) { SelectedBuildingId = id; SelectedCatId = ""; var b = SelectedBuilding; if (b != null && !DirectControl) focus = At(b.Position) + new Vector3((b.Width - 1) * .5f, 0, (b.Depth - 1) * .5f); }
        public void EnterSelectedCat() { if (SelectedCat != null) Game.Act(new GameAction { Kind = "EnterCatControl", CatId = SelectedCat.Id }); }
        public void LeaveCat() { var cat = ControlledCat; if (cat != null) Game.Act(new GameAction { Kind = "LeaveCatControl", CatId = cat.Id }); }

        private void UpdateSelection()
        {
            Vector3 p; float w = 1, d = 1;
            if (SelectedCat != null && SelectedCat.Position.Level == RenderedLevel) { p = Position(SelectedCat); w = .85f; d = .85f; }
            else if (SelectedBuilding != null && SelectedBuilding.Position.Level == RenderedLevel) { var b = SelectedBuilding; p = At(b.Position) + new Vector3((b.Width - 1) * .5f, 0, (b.Depth - 1) * .5f); w = b.Width; d = b.Depth; }
            else if (Game.UI != null && Game.UI.HasPlacement) p = At(CursorTile);
            else { selection.enabled = false; return; }
            selection.enabled = true; p.y += .08f;
            selection.SetPositions(new[] { p + new Vector3(-w / 2, 0, -d / 2), p + new Vector3(-w / 2, 0, d / 2), p + new Vector3(w / 2, 0, d / 2), p + new Vector3(w / 2, 0, -d / 2) });
        }
    }
}
