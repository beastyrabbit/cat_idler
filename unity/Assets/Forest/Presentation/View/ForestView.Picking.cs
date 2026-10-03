using System;
using System.Collections.Generic;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Screen-to-world picking without physics layers: cats by screen distance to their projected
    /// bodies, buildings, piles and dungeon entrances by ray tests against their rendered bounds, and
    /// the ground through the terrain raycast. <see cref="SubjectAt"/> is what a world click selects;
    /// <see cref="HoveredSubject"/> is the same pick under the pointer, refreshed every frame.
    /// </summary>
    public sealed partial class ForestView
    {
        /// <summary>Cats are picked within this many points of their projected body centre (wider when zoomed in).</summary>
        internal const float CatPickPoints = 24;
        private const float CatBodyHeight = .28f, CatBodyRadius = .42f;
        private readonly List<Renderer> pickRenderers = new List<Renderer>();

        public Int2 CursorTile { get; private set; }
        public event Action<Int2> TileClicked;
        /// <summary>What the pointer hovers in the world (management view, pointer over the map, no drag in progress).</summary>
        internal Subject HoveredSubject { get; private set; }

        /// <summary>Screen pixels per point: 2 on Retina displays.</summary>
        private static float PixelScale => Screen.dpi >= 160 ? 2 : 1;

        private Vector3 GroundPoint(Vector2 screen)
        {
            var ray = Camera.ScreenPointToRay(screen);
            if (TerrainRaycast(ray, out var point)) return point;
            return new Plane(Vector3.up, new Vector3(0, focus.y, 0)).Raycast(ray, out float distance) ? ray.GetPoint(distance) : focus;
        }

        private Int2 TileUnder(Vector2 screen)
        {
            var point = GroundPoint(screen);
            return new Int2(Mathf.RoundToInt(point.x), Mathf.RoundToInt(point.z), RenderedLevel);
        }

        /// <summary>
        /// The cat, building, pile or dungeon entrance drawn under a screen point (pixels, origin bottom
        /// left), or <see cref="Subject.None"/> for open ground. A cat under the pointer wins over the
        /// building behind it; otherwise the nearest rendered building, pile or entrance along the ray wins,
        /// so a click on a roof selects its building.
        /// </summary>
        public Subject SubjectAt(Vector2 screen)
        {
            if (!WorldReady || Camera == null) return Subject.None;
            var village = Game.Selected;
            int level = RenderedLevel;
            string cat = ""; float best = float.MaxValue;
            foreach (var c in village.Cats)
            {
                if (!c.Alive || c.Position.Level != level || !entities.TryGetValue("cat:" + c.Id, out var go) || go == null) continue;
                var body = go.transform.position + Vector3.up * CatBodyHeight;
                var projected = Camera.WorldToScreenPoint(body);
                if (projected.z <= 0) continue;
                float radius = Mathf.Max(CatPickPoints * PixelScale, ((Vector2)Camera.WorldToScreenPoint(body + Camera.transform.right * CatBodyRadius) - (Vector2)projected).magnitude);
                float distance = Vector2.Distance(screen, projected);
                if (distance <= radius && distance < best) { best = distance; cat = c.Id; }
            }
            if (cat != "") return new Subject(SubjectKind.Cat, cat);

            var ray = Camera.ScreenPointToRay(screen);
            var hit = Subject.None; float nearest = float.MaxValue;
            foreach (var b in village.Buildings)
                if (b.Position.Level == level) Consider(ray, "building:" + b.Id, new Subject(SubjectKind.Building, b.Id), ref hit, ref nearest);
            foreach (var pile in village.Stockpiles)
                if (pile.Position.Level == level) Consider(ray, "pile:" + pile.Id, new Subject(SubjectKind.Pile, pile.Id), ref hit, ref nearest);
            if (level == 0)
                foreach (var site in Game.CurrentWorld.Dungeons)
                    Consider(ray, "dungeon:" + site.Id, new Subject(SubjectKind.Entrance, site.Id), ref hit, ref nearest);
            return hit;
        }

        private void Consider(Ray ray, string entityId, Subject subject, ref Subject hit, ref float nearest)
        {
            if (!entities.TryGetValue(entityId, out var go) || go == null) return;
            go.GetComponentsInChildren(false, pickRenderers);
            if (pickRenderers.Count == 0) return;
            var bounds = pickRenderers[0].bounds;
            for (int i = 1; i < pickRenderers.Count; i++) bounds.Encapsulate(pickRenderers[i].bounds);
            pickRenderers.Clear();
            if (bounds.IntersectRay(ray, out float distance) && distance < nearest) { nearest = distance; hit = subject; }
        }

        /// <summary>Refreshes <see cref="HoveredSubject"/>; called once per frame after the camera moved.</summary>
        internal void UpdateHover()
        {
            if (router == null || !router.DevicesPresent || DirectControl || router.PointerOverUi || router.TextFieldFocused
                || router.PrimaryDragging || dragPanning || router.SecondaryHeld || router.MiddleHeld)
            {
                HoveredSubject = Subject.None;
                return;
            }
            HoveredSubject = SubjectAt(router.PointerPosition);
        }
    }
}
