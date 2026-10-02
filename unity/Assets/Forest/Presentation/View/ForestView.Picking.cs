using System;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>Screen-to-world picking: the cursor tile and what a world click selects.</summary>
    public sealed partial class ForestView
    {
        public Int2 CursorTile { get; private set; }
        public event Action<Int2> TileClicked;
        /// <summary>What the pointer hovers in the world. Hover picking does not exist yet.</summary>
        internal Subject HoveredSubject => Subject.None;

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

        /// <summary>Selects the living cat or the building at the tile and opens the inspector.</summary>
        private void PickAt(Int2 tile)
        {
            var cat = Game.Selected.Cats.Where(c => c.Alive && c.Position.Level == RenderedLevel).OrderBy(c => Vector2.Distance(new Vector2((float)c.X, (float)c.Z), new Vector2(tile.X, tile.Z))).FirstOrDefault();
            if (cat != null && Vector2.Distance(new Vector2((float)cat.X, (float)cat.Z), new Vector2(tile.X, tile.Z)) < .8f) { InspectCat(cat.Id); Game.UI?.OpenPanel("Inspect"); }
            else
            {
                var b = Game.Selected.Buildings.Find(p => p.Position.Level == RenderedLevel && tile.X >= p.Position.X && tile.X < p.Position.X + p.Width && tile.Z >= p.Position.Z && tile.Z < p.Position.Z + p.Depth);
                InspectBuilding(b?.Id ?? "");
                if (b != null) Game.UI?.OpenPanel("Inspect");
            }
        }
    }
}
