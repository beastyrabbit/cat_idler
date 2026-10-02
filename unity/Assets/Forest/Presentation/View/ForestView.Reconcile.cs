using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The reconcile driver: every 0.2 s it touches the entity of everything visible on the rendered
    /// level, then destroys entities nothing touched. Each domain reconciles in its own partial.
    /// </summary>
    public sealed partial class ForestView
    {
        /// <summary>Reconciles now instead of waiting for the next 0.2 s tick.</summary>
        internal void ForceReconcile()
        {
            if (!WorldReady) return;
            Reconcile();
        }

        internal void Reconcile()
        {
            var world = Game.CurrentWorld; var v = Game.Selected;
            int level = RenderedLevel;
            touched.Clear();
            var center = ControlledCat == null ? focus : Position(ControlledCat);
            groundCenter = new Int2(Mathf.RoundToInt(center.x / 8) * 8, Mathf.RoundToInt(center.z / 8) * 8, level);
            TerrainTick();
            ReconcileBuildings(v, level);
            ReconcilePiles(v, level);
            ReconcileFarms(v, level);
            var known = new HashSet<Int2>(v.Known);
            ReconcileTiles(world, v, level, known);
            ReconcileBoundary(v, level, known);
            ReconcileCats(v, level);
            ReconcileCarriedCargo(v, level);
            ReconcileMovers(world, v, level, known);
            RenderDungeonPlaces(known);
            foreach (var key in entities.Keys.Where(k => !touched.Contains(k)).ToArray()) { Destroy(entities[key]); entities.Remove(key); entityAssets.Remove(key); }
        }
    }
}
