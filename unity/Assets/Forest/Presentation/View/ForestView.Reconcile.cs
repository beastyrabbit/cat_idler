using System.Collections.Generic;
using System.Diagnostics;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The reconcile driver. Every 0.2 s it checks a cheap gate (game revision, world, village, level, ground centre,
    /// controlled cat and its camera yaw); when nothing changed it returns at once. Otherwise each domain touches the
    /// entity of everything visible on the rendered level and entities nothing touched are released to a pool or
    /// destroyed. Static scenery is not an entity: it is invalidated per chunk (ForestView.Props.cs).
    /// </summary>
    public sealed partial class ForestView
    {
        private const int PoolPerAsset = 16;

        private readonly struct ReconcileGate
        {
            public readonly int Revision, Level, Yaw; public readonly World World; public readonly string Village, Controlled; public readonly Int2 Ground;
            public ReconcileGate(int revision, World world, string village, int level, Int2 ground, string controlled, int yaw)
            { Revision = revision; World = world; Village = village; Level = level; Ground = ground; Controlled = controlled; Yaw = yaw; }
            public bool Same(in ReconcileGate o) => Revision == o.Revision && ReferenceEquals(World, o.World) && Village == o.Village && Level == o.Level && Ground.Equals(o.Ground) && Controlled == o.Controlled && Yaw == o.Yaw;
        }

        private ReconcileGate lastGate;
        private bool hasGate;
        private readonly List<string> staleEntities = new List<string>();
        private readonly Dictionary<string, Stack<GameObject>> entityPool = new Dictionary<string, Stack<GameObject>>();

        // The explored tiles of the selected village, kept in step with the append-only Village.Known list.
        private readonly HashSet<Int2> knownSet = new HashSet<Int2>();
        private List<Int2> knownSource;
        private string knownVillage = "";
        private int knownCount;

        /// <summary>Wall-clock cost of the last reconcile that did work, and of the last skipped one, in milliseconds.</summary>
        internal double LastReconcileMilliseconds { get; private set; }
        internal double LastSkippedReconcileMilliseconds { get; private set; }
        internal int ReconcileRuns { get; private set; }
        internal int ReconcileSkips { get; private set; }
        private readonly Stopwatch reconcileWatch = new Stopwatch();

        /// <summary>Reconciles now instead of waiting for the next 0.2 s tick, even if nothing visible changed.</summary>
        internal void ForceReconcile()
        {
            if (!WorldReady) return;
            ReconcileWith(true);
        }

        /// <summary>The periodic reconcile. While the view is enabled it is gated; on a disabled view (tools, tests) it always runs.</summary>
        internal void Reconcile() => ReconcileWith(!isActiveAndEnabled);

        private void ReconcileWith(bool force)
        {
            reconcileWatch.Restart();
            var world = Game.CurrentWorld; var v = Game.Selected;
            int level = RenderedLevel;
            var controlled = ControlledCat;
            var center = controlled == null ? focus : Position(controlled);
            groundCenter = new Int2(Mathf.RoundToInt(center.x / 8) * 8, Mathf.RoundToInt(center.z / 8) * 8, level);
            var gate = new ReconcileGate(Game.Revision, world, v.Id, level, groundCenter, controlled?.Id ?? "", controlled == null ? 0 : Mathf.RoundToInt(yaw / 15));
            if (!force && hasGate && gate.Same(lastGate))
            {
                ReconcileSkips++;
                LastSkippedReconcileMilliseconds = reconcileWatch.Elapsed.TotalMilliseconds;
                return;
            }
            lastGate = gate; hasGate = true;
            touched.Clear();
            SyncKnown(v);
            TerrainTick();
            ReconcileBuildings(v, level);
            ReconcilePiles(v, level);
            ReconcileFarms(v, level);
            ReconcileStaticProps(world, v, level, force);
            ReconcileCats(v, level);
            ReconcileCarriedCargo(v, level);
            ReconcileMovers(world, v, level, knownSet);
            RenderDungeonPlaces(knownSet);
            staleEntities.Clear();
            foreach (var key in entities.Keys) if (!touched.Contains(key)) staleEntities.Add(key);
            foreach (var key in staleEntities) ReleaseEntity(key);
            ReconcileRuns++;
            LastReconcileMilliseconds = reconcileWatch.Elapsed.TotalMilliseconds;
        }

        /// <summary>
        /// Brings the persistent Known set up to date. Known only grows, so normally only the new tail is added; a
        /// replaced, shrunk or other village's list rebuilds the set. Returns false when the set was rebuilt.
        /// </summary>
        private bool SyncKnown(Village v)
        {
            var list = v.Known;
            if (!ReferenceEquals(list, knownSource) || knownVillage != v.Id || list.Count < knownCount)
            {
                knownSet.Clear();
                foreach (var p in list) knownSet.Add(p);
                knownSource = list; knownVillage = v.Id; knownCount = list.Count;
                KnownRebuilt();
                return false;
            }
            for (int i = knownCount; i < list.Count; i++)
                if (knownSet.Add(list[i])) KnownAdded(list[i]);
            knownCount = list.Count;
            return true;
        }

        /// <summary>
        /// Removes an entity that nothing touched. Short-lived markers (carried cargo, pile goods, busy-workshop tools)
        /// are deactivated into a per-asset pool for <see cref="TryTakePooled"/>; everything else is destroyed.
        /// </summary>
        private void ReleaseEntity(string id)
        {
            if (!entities.TryGetValue(id, out var go)) return;
            entities.Remove(id);
            entityAssets.TryGetValue(id, out var asset);
            entityAssets.Remove(id);
            if (go == null) return;
            if (asset != null && Poolable(id))
            {
                if (!entityPool.TryGetValue(asset, out var pool)) entityPool[asset] = pool = new Stack<GameObject>();
                if (pool.Count < PoolPerAsset)
                {
                    go.SetActive(false);
                    go.name = "pooled " + asset;
                    go.transform.rotation = Quaternion.identity;
                    var visual = VisualOf(go);
                    if (visual != null) { visual.localPosition = Vector3.zero; visual.localRotation = Quaternion.identity; visual.localScale = Vector3.one; }
                    pool.Push(go);
                    return;
                }
            }
            Destroy(go);
        }

        /// <summary>A pooled marker root of this asset, renamed and re-activated for <paramref name="id"/>, or false.</summary>
        private bool TryTakePooled(string id, string asset, out GameObject go)
        {
            go = null;
            if (!Poolable(id) || !entityPool.TryGetValue(asset, out var pool)) return false;
            while (pool.Count > 0)
            {
                go = pool.Pop();
                if (go == null) continue;
                go.name = id;
                go.SetActive(true);
                return true;
            }
            go = null;
            return false;
        }

        private static bool Poolable(string id) =>
            id.StartsWith("cargo:", System.StringComparison.Ordinal) || id.StartsWith("pilegoods:", System.StringComparison.Ordinal) || id.StartsWith("activity:", System.StringComparison.Ordinal);
    }
}
