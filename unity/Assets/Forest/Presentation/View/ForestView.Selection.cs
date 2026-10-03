using System;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Selection: what is selected, the world click that selects it (router PrimaryClick at Selection priority,
    /// handed to <see cref="ForestUI.Select"/>), the anchors the tethered card points at, direct-control entry and the
    /// selection outline. Selecting never opens a drawer; the tethered card is the only inspector.
    /// </summary>
    public sealed partial class ForestView
    {
        public string SelectedCatId { get; private set; } = "";
        public string SelectedBuildingId { get; private set; } = "";
        public string SelectedPileId { get; private set; } = "";
        public Cat SelectedCat => Game.Selected?.Cats.Find(c => c.Id == SelectedCatId);
        public Building SelectedBuilding => Game.Selected?.Buildings.Find(b => b.Id == SelectedBuildingId);
        public Stockpile SelectedPile => Game.Selected?.Stockpiles.Find(p => p.Id == SelectedPileId);

        /// <summary>The one selected subject, or <see cref="Subject.None"/>.</summary>
        public Subject SelectedSubject
        {
            get
            {
                if (SelectedCatId != "") return new Subject(SubjectKind.Cat, SelectedCatId);
                if (SelectedBuildingId != "") return new Subject(SubjectKind.Building, SelectedBuildingId);
                if (SelectedPileId != "") return new Subject(SubjectKind.Pile, SelectedPileId);
                return Subject.None;
            }
        }

        private bool followSelectedCat;
        private LineRenderer selection;
        private InputRouter selectionRouter;
        private IDisposable selectionClick;

        private void CreateSelectionMarker()
        {
            selection = new GameObject("Selected place").AddComponent<LineRenderer>();
            selection.material = Material("selection", new Color(1, .78f, .38f), true);
            selection.widthMultiplier = .045f; selection.loop = true; selection.positionCount = 4;
            selection.shadowCastingMode = UnityEngine.Rendering.ShadowCastingMode.Off;
            selection.receiveShadows = false;
        }

        /// <summary>Selects a cat. The camera pans only when the cat is off screen.</summary>
        public void InspectCat(string id)
        {
            SelectedCatId = id ?? ""; SelectedBuildingId = ""; SelectedPileId = ""; followSelectedCat = true;
            var cat = SelectedCat;
            if (cat == null || DirectControl) return;
            if (cat.Position.Level != viewedLevel) { viewedLevel = cat.Position.Level; nextReconcile = 0; focus = Position(cat); }
            else if (!OnScreen(Position(cat))) focus = Position(cat);
        }

        /// <summary>Selects a building ("" clears every selection). The camera pans only when it is off screen.</summary>
        public void InspectBuilding(string id)
        {
            SelectedBuildingId = id ?? ""; SelectedCatId = ""; SelectedPileId = ""; followSelectedCat = false;
            var b = SelectedBuilding;
            if (b != null && !DirectControl && !OnScreen(Centre(b))) focus = Centre(b);
        }

        /// <summary>Selects a stockpile. The camera pans only when it is off screen.</summary>
        public void InspectPile(string id)
        {
            SelectedPileId = id ?? ""; SelectedCatId = ""; SelectedBuildingId = ""; followSelectedCat = false;
            var p = SelectedPile;
            if (p != null && !DirectControl && !OnScreen(Centre(p))) focus = Centre(p);
        }

        /// <summary>Pans the management camera to the selected subject (the card's off-screen pin).</summary>
        internal void FocusSubject(Subject s)
        {
            if (DirectControl || !TryGetAnchor(s, out var anchor)) return;
            focus = new Vector3(anchor.x, focus.y, anchor.z);
        }

        public void EnterSelectedCat() { if (SelectedCat != null) Game.Act(new GameAction { Kind = "EnterCatControl", CatId = SelectedCat.Id }); }
        public void LeaveCat() { var cat = ControlledCat; if (cat != null) Game.Act(new GameAction { Kind = "LeaveCatControl", CatId = cat.Id }); }

        /// <summary>
        /// The world point the tethered card's leader line ends at: a cat's feet, a building's entrance tile, a pile's
        /// centre. Uses the rendered entity when there is one, so the line follows interpolated motion.
        /// </summary>
        internal bool TryGetAnchor(Subject s, out Vector3 anchor)
        {
            anchor = default;
            var village = Game.Selected;
            if (village == null || Game.CurrentWorld == null) return false;
            switch (s.Kind)
            {
                case SubjectKind.Cat:
                    var cat = village.Cats.Find(c => c.Id == s.Id);
                    if (cat == null || cat.Position.Level != RenderedLevel) return false;
                    anchor = entities.TryGetValue("cat:" + cat.Id, out var go) && go != null ? go.transform.position : Position(cat);
                    return true;
                case SubjectKind.Building:
                    var b = village.Buildings.Find(x => x.Id == s.Id);
                    if (b == null || b.Position.Level != RenderedLevel) return false;
                    anchor = b.HasEntrance && b.Entrance.Level == b.Position.Level ? At(b.Entrance) : Centre(b);
                    return true;
                case SubjectKind.Pile:
                    var p = village.Stockpiles.Find(x => x.Id == s.Id);
                    if (p == null || p.Position.Level != RenderedLevel) return false;
                    anchor = Centre(p);
                    return true;
                default:
                    return false;
            }
        }

        /// <summary>
        /// What a click at this screen point selects. W2-18's render-based pick (<see cref="SubjectAt"/>) at the press
        /// point decides; a station store or food container selects its building, a dungeon entrance selects nothing (it
        /// has no card yet), and open ground falls back to the building footprint or free stockpile under the ground point.
        /// </summary>
        internal Subject SelectableAt(Vector2 screen)
        {
            var village = Game.Selected;
            if (village == null || Camera == null) return Subject.None;
            var picked = SubjectAt(screen);
            if (picked.Kind == SubjectKind.Pile)
            {
                var pile = village.Stockpiles.Find(p => p.Id == picked.Id);
                if (pile != null && pile.ManagedBy != "" && village.Buildings.Exists(b => b.Id == pile.ManagedBy))
                    return new Subject(SubjectKind.Building, pile.ManagedBy);
                if (pile == null || !Pickable(pile)) picked = Subject.None;
            }
            if (picked.Kind == SubjectKind.Entrance) return Subject.None;
            if (!picked.IsNone) return picked;

            int level = RenderedLevel;
            var ray = Camera.ScreenPointToRay(screen);
            var ground = TerrainRaycast(ray, out var terrain) ? terrain : new Plane(Vector3.up, Vector3.zero).Raycast(ray, out float flat) ? ray.GetPoint(flat) : Vector3.positiveInfinity;
            if (float.IsInfinity(ground.x)) return Subject.None;
            var tile = new Int2(Mathf.RoundToInt(ground.x), Mathf.RoundToInt(ground.z), level);
            var footprint = village.Buildings.Find(p => Covers(p.Position, p.Width, p.Depth, tile));
            if (footprint != null) return new Subject(SubjectKind.Building, footprint.Id);
            var free = village.Stockpiles.Find(p => p.ManagedBy == "" && Pickable(p) && Covers(p.Position, p.Width, p.Depth, tile));
            return free != null ? new Subject(SubjectKind.Pile, free.Id) : Subject.None;
        }

        private static bool Pickable(Stockpile p) => p.Kind == "storage" || p.Kind == "gather" || p.Kind == "fishing" || p.Kind == "spill";

        private static bool Covers(Int2 origin, int width, int depth, Int2 tile) =>
            origin.Level == tile.Level && tile.X >= origin.X && tile.X < origin.X + width && tile.Z >= origin.Z && tile.Z < origin.Z + depth;

        /// <summary>The router binding for world clicks; created once the router exists and moved with it.</summary>
        private void BindSelectionClick()
        {
            if (selectionRouter == router) return;
            selectionClick?.Dispose();
            selectionRouter = router;
            selectionClick = router?.BindPointer(PointerAction.PrimaryClick, InputPriority.Selection, SelectAtPointer);
        }

        /// <summary>A world click selects the subject under it, or deselects on empty ground. No drawer opens.</summary>
        private bool SelectAtPointer(PointerInfo pointer)
        {
            var ui = Game.UI;
            if (!WorldReady || ui == null || ui.HasPlacement || DirectControl) return false;
            var subject = SelectableAt(pointer.PressPosition);
            if (subject.Kind == SubjectKind.Pile) InspectPile(subject.Id);
            ui.Select(subject);
            return !subject.IsNone;
        }

        private bool OnScreen(Vector3 world)
        {
            if (Camera == null) return false;
            var p = Camera.WorldToViewportPoint(world);
            return p.z > 0 && p.x > .08f && p.x < .92f && p.y > .1f && p.y < .9f;
        }

        private Vector3 Centre(Building b) => At(b.Position) + new Vector3((b.Width - 1) * .5f, 0, (b.Depth - 1) * .5f);

        private Vector3 Centre(Stockpile p) => At(p.Position) + new Vector3((p.Width - 1) * .5f, 0, (p.Depth - 1) * .5f);

        private void UpdateSelection()
        {
            BindSelectionClick();
            Vector3 p; float w = 1, d = 1;
            var cat = SelectedCat;
            if (cat != null && cat.Position.Level == RenderedLevel && cat.ControlledBy == "") { p = Position(cat); w = .85f; d = .85f; }
            else if (SelectedBuilding != null && SelectedBuilding.Position.Level == RenderedLevel) { var b = SelectedBuilding; p = Centre(b); w = b.Width; d = b.Depth; }
            else if (SelectedPile != null && SelectedPile.Position.Level == RenderedLevel) { var pile = SelectedPile; p = Centre(pile); w = pile.Width; d = pile.Depth; }
            else if (Game.UI != null && Game.UI.HasPlacement) p = At(CursorTile);
            else { selection.enabled = false; return; }
            selection.enabled = true; p.y += .08f;
            selection.SetPositions(new[] { p + new Vector3(-w / 2, 0, -d / 2), p + new Vector3(-w / 2, 0, d / 2), p + new Vector3(w / 2, 0, d / 2), p + new Vector3(w / 2, 0, -d / 2) });
        }
    }
}
