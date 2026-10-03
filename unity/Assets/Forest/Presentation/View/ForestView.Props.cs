using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using IdleCatForest.Simulation;
using Unity.Profiling;
using UnityEngine;
using Debug = UnityEngine.Debug;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Static scenery on the surface, drawn by <see cref="StaticProps"/>: per 16x16 chunk, cached and rebuilt only when
    /// the chunk's signature (its explored tiles, the village footprint or the explored edge) changes. The window of
    /// chunks follows the camera frustum on the ground, so the forest continues past the explored edge without a seam.
    /// Explored chunks are checked and rebuilt during reconcile; unexplored edge chunks are built in
    /// <see cref="UpdateProps"/> within a per-frame time budget, nearest first.
    /// </summary>
    public sealed partial class ForestView
    {
        private const float PropBuildBudgetMs = 2f, PropFirstFillBudgetMs = 250f;
        private const int WindowMarginTiles = 6, WindowMaxRadiusTiles = 120;

        private StaticProps staticProps;
        private uint canopySeed;
        private string propsVillage = "";
        private uint propsSeed;
        private int propsGeneration = int.MinValue;
        private readonly Dictionary<ChunkKey, int> knownPerChunk = new Dictionary<ChunkKey, int>();
        private readonly HashSet<Int2> occupied = new HashSet<Int2>();
        private int footprintSignature;
        private readonly List<StaticProps.Chunk> activeChunks = new List<StaticProps.Chunk>();
        private readonly List<ChunkKey> propQueue = new List<ChunkKey>();
        private readonly List<ChunkKey> propEvict = new List<ChunkKey>();
        private bool propsActiveDirty = true, hasPropWindow;
        private int propMinX, propMaxX, propMinZ, propMaxZ, propCenterX, propCenterZ;
        private Comparison<ChunkKey> byDistanceToCenter;
        private readonly Stopwatch propWatch = new Stopwatch();

        /// <summary>Milliseconds spent building chunks in the last frame that built any, and the number built.</summary>
        internal double LastPropBuildMilliseconds { get; private set; }
        internal int LastPropBuildChunks { get; private set; }
        internal int StaticPropDrawCalls => staticProps?.DrawCalls ?? 0;
        internal int StaticPropInstances => staticProps?.DrawnInstances ?? 0;
        internal int StaticPropChunks => staticProps?.ChunkCount ?? 0;

        partial void AwakeProps()
        {
            // The view destroys every material in its cache on destroy, the instanced variants included.
            staticProps = new StaticProps { OwnMaterial = (key, material) => materials[key] = material };
            byDistanceToCenter = (a, b) => ChunkDistance(a).CompareTo(ChunkDistance(b));
            AwakeViewStats();
        }

        partial void LevelChangedProps(int level) => propsActiveDirty = true;

        partial void UpdateProps(float dt)
        {
            TickViewStats(dt);
            if (staticProps == null || RenderedLevel != 0 || propsVillage != Game.Selected.Id) return;
            var world = Game.CurrentWorld; var v = Game.Selected;
            if (UpdatePropWindow()) propsActiveDirty = true;
            BuildPropWindow(world, v, activeChunks.Count == 0 ? PropFirstFillBudgetMs : PropBuildBudgetMs);
            if (propsActiveDirty)
            {
                activeChunks.Clear();
                for (int cz = propMinZ; cz <= propMaxZ; cz++) for (int cx = propMinX; cx <= propMaxX; cx++)
                    if (staticProps.TryGetChunk(new ChunkKey(cx, cz, 0), out var chunk) && chunk.Built) { chunk.LastUsedFrame = Time.frameCount; activeChunks.Add(chunk); }
                staticProps.Assemble(activeChunks);
                propsActiveDirty = false;
                EvictPropChunks();
            }
            if (viewStats) propWatch.Restart();
            if (!propsHiddenForStats) staticProps.Draw(0);
            if (viewStats) statSubmit.Add(propWatch.Elapsed.TotalMilliseconds);
        }

        // ------------------------------------------------------------------ reconcile side

        /// <summary>Checks the explored chunks of the window and rebuilds the ones whose signature changed. Runs inside reconcile.</summary>
        private void ReconcileStaticProps(World world, Village v, int level, bool force)
        {
            if (staticProps == null || level != 0) return;
            if (propsVillage != v.Id || propsSeed != world.Seed || propsGeneration != world.GenerationVersion)
            {
                staticProps.Clear();
                propsVillage = v.Id; propsSeed = world.Seed; propsGeneration = world.GenerationVersion;
                canopySeed = Canopy.Seed(world);
                footprintSignature = 0; occupied.Clear(); stagingChunk = null;
                propsActiveDirty = true; hasPropWindow = false;
            }
            UpdateFootprint(world, v);
            if (!hasPropWindow) WindowAround(groundCenter.X, groundCenter.Z, 3 * ChunkKey.Size);
            for (int cz = propMinZ; cz <= propMaxZ; cz++) for (int cx = propMinX; cx <= propMaxX; cx++)
            {
                var key = new ChunkKey(cx, cz, 0);
                if (!TouchesKnown(key)) continue;
                var chunk = staticProps.GetOrCreate(key);
                if (chunk.Built && !chunk.Dirty && KnownSignature(world, key) != chunk.Signature) chunk.Dirty = true;
                if (chunk.Dirty || !chunk.Built)
                {
                    if (stagingChunk != null && stagingChunk.Key.Equals(key)) stagingChunk = null;
                    BuildChunk(world, v, chunk); propsActiveDirty = true;
                }
            }
            if (force && !isActiveAndEnabled)
            {
                // Tools and tests call reconcile on a disabled view; the window must still be complete afterwards.
                BuildPropWindow(world, v, float.MaxValue);
                propsActiveDirty = true;
            }
        }

        private void KnownRebuilt()
        {
            knownPerChunk.Clear();
            foreach (var p in knownSet)
                if (p.Level == 0) { var key = ChunkKey.Of(p); knownPerChunk.TryGetValue(key, out int n); knownPerChunk[key] = n + 1; }
            staticProps?.MarkAllDirty();
            propsActiveDirty = true;
        }

        private void KnownAdded(Int2 p)
        {
            if (p.Level != 0) return;
            var key = ChunkKey.Of(p); knownPerChunk.TryGetValue(key, out int n); knownPerChunk[key] = n + 1;
            // The tile itself, its road and fence neighbours and the fog fade three tiles out.
            for (int cz = ChunkKey.FloorDiv(p.Z - 3); cz <= ChunkKey.FloorDiv(p.Z + 3); cz++)
                for (int cx = ChunkKey.FloorDiv(p.X - 3); cx <= ChunkKey.FloorDiv(p.X + 3); cx++)
                    staticProps?.MarkDirty(new ChunkKey(cx, cz, 0));
        }

        /// <summary>
        /// Buildings, farms, piles, doorways, dungeon mouths and the village outline decide where canopy and scatter may
        /// grow and where boundary fences stand. A change rebuilds every explored chunk.
        /// </summary>
        private void UpdateFootprint(World world, Village v)
        {
            int h = 17;
            unchecked
            {
                foreach (var b in v.Buildings) h = h * 31 + b.Position.GetHashCode() * 7 + b.Width * 3 + b.Depth + (b.HasEntrance ? b.Entrance.GetHashCode() : 0);
                foreach (var f in v.Farms) h = h * 31 + f.Position.GetHashCode() * 5 + f.Width * 3 + f.Depth;
                foreach (var s in v.Stockpiles) h = h * 31 + s.Position.GetHashCode() * 11 + s.Width * 3 + s.Depth;
                foreach (var d in world.Dungeons) h = h * 31 + d.Entrance.GetHashCode();
                h = h * 31 + v.BoundaryEdges.Count;
                if (v.BoundaryEdges.Count > 0) h = h * 31 + v.BoundaryEdges[0].From.GetHashCode() + v.BoundaryEdges[v.BoundaryEdges.Count - 1].To.GetHashCode() * 3;
                h = h * 31 + v.Center.GetHashCode() + v.Radius * 7 + v.LayoutVersion * 13;
            }
            if (h == footprintSignature) return;
            footprintSignature = h;
            occupied.Clear();
            void Area(Int2 at, int w, int d, int pad)
            {
                for (int z = at.Z - pad; z < at.Z + d + pad; z++) for (int x = at.X - pad; x < at.X + w + pad; x++) occupied.Add(new Int2(x, z, at.Level));
            }
            foreach (var b in v.Buildings) { Area(b.Position, b.Width, b.Depth, 0); if (b.HasEntrance) Area(b.Entrance, 1, 1, 0); }
            foreach (var f in v.Farms) Area(f.Position, f.Width, f.Depth, 0);
            foreach (var s in v.Stockpiles) Area(s.Position, Math.Max(1, s.Width), Math.Max(1, s.Depth), 0);
            foreach (var d in world.Dungeons) Area(new Int2(d.Entrance.X - 1, d.Entrance.Z - 1, d.Entrance.Level), 3, 3, 1);
            foreach (var pair in knownPerChunk) staticProps?.MarkDirty(pair.Key);
            propsActiveDirty = true;
        }

        private bool TouchesKnown(ChunkKey key)
        {
            for (int dz = -1; dz <= 1; dz++) for (int dx = -1; dx <= 1; dx++)
                if (knownPerChunk.ContainsKey(new ChunkKey(key.X + dx, key.Z + dz, key.Level))) return true;
            return false;
        }

        /// <summary>A hash of every explored tile in the chunk and its one-tile border (roads and fences look at neighbours).</summary>
        private int KnownSignature(World world, ChunkKey key)
        {
            int h = 23;
            unchecked
            {
                for (int z = key.MinZ - 1; z <= key.MinZ + ChunkKey.Size; z++)
                    for (int x = key.MinX - 1; x <= key.MinX + ChunkKey.Size; x++)
                    {
                        var p = new Int2(x, z, key.Level);
                        if (!knownSet.Contains(p)) continue;
                        var t = world.GetTile(p);
                        h = h * 31 + x * 73 + z * 37 + (t == null ? 1 : TileSignature(t));
                    }
            }
            return h;
        }

        private void BuildChunk(World world, Village v, StaticProps.Chunk chunk)
        {
            chunk.Clear();
            bool nearKnown = TouchesKnown(chunk.Key);
            BuildRows(world, v, chunk, 0, ChunkKey.Size, nearKnown);
            FinishChunk(world, v, chunk, nearKnown);
        }

        private void BuildRows(World world, Village v, StaticProps.Chunk chunk, int fromRow, int toRow, bool nearKnown)
        {
            var key = chunk.Key;
            for (int z = key.MinZ + fromRow; z < key.MinZ + toRow; z++)
                for (int x = key.MinX; x < key.MinX + ChunkKey.Size; x++)
                {
                    var p = new Int2(x, z, key.Level);
                    if (knownSet.Contains(p))
                    {
                        chunk.HasKnown = true;
                        var tile = world.GetTile(p);
                        if (tile != null) AddKnownTile(chunk, world, v, tile);
                    }
                    else AddUnknownTile(chunk, world, x, z, nearKnown ? FogStep(p) : PropLook.FogSteps - 1);
                }
        }

        private void FinishChunk(World world, Village v, StaticProps.Chunk chunk, bool nearKnown)
        {
            if (nearKnown) AddBoundary(chunk, world, v);
            chunk.Signature = nearKnown ? KnownSignature(world, chunk.Key) : 0;
            chunk.Built = true; chunk.Dirty = false;
        }

        /// <summary>Distance in tiles (Chebyshev) from an unexplored tile to the explored edge, as a fog step 1 to 3.</summary>
        private int FogStep(Int2 p)
        {
            for (int r = 1; r < PropLook.FogSteps - 1; r++)
                for (int dz = -r; dz <= r; dz++)
                    for (int dx = -r; dx <= r; dx++)
                    {
                        if (Math.Abs(dx) != r && Math.Abs(dz) != r) continue;
                        if (knownSet.Contains(new Int2(p.X + dx, p.Z + dz, p.Level))) return r;
                    }
            return PropLook.FogSteps - 1;
        }

        // ------------------------------------------------------------------ window and streaming

        /// <summary>The chunk window: the camera frustum's footprint on the ground plus a margin. True when it changed.</summary>
        private bool UpdatePropWindow()
        {
            if (Camera == null) return false;
            float groundY = focus.y;
            var plane = new Plane(Vector3.up, new Vector3(0, groundY, 0));
            float far = Camera.orthographic ? 400 : Mathf.Max(30, RenderSettings.fogEndDistance + 8);
            float minX = float.MaxValue, maxX = float.MinValue, minZ = float.MaxValue, maxZ = float.MinValue;
            for (int i = 0; i < 5; i++)
            {
                var vp = i == 4 ? new Vector3(.5f, .5f, 0) : new Vector3(i & 1, i >> 1, 0);
                var ray = Camera.ViewportPointToRay(vp);
                Vector3 hit;
                if (plane.Raycast(ray, out float d) && d <= far) hit = ray.GetPoint(d);
                else { var flat = new Vector3(ray.direction.x, 0, ray.direction.z); hit = ray.origin + (flat.sqrMagnitude > 1e-6f ? flat.normalized : Vector3.forward) * far; }
                minX = Mathf.Min(minX, hit.x); maxX = Mathf.Max(maxX, hit.x); minZ = Mathf.Min(minZ, hit.z); maxZ = Mathf.Max(maxZ, hit.z);
            }
            if (!Camera.orthographic) { var c = Camera.transform.position; minX = Mathf.Min(minX, c.x); maxX = Mathf.Max(maxX, c.x); minZ = Mathf.Min(minZ, c.z); maxZ = Mathf.Max(maxZ, c.z); }
            float fx = focus.x, fz = focus.z;
            minX = Mathf.Max(minX - WindowMarginTiles, fx - WindowMaxRadiusTiles); maxX = Mathf.Min(maxX + WindowMarginTiles, fx + WindowMaxRadiusTiles);
            minZ = Mathf.Max(minZ - WindowMarginTiles, fz - WindowMaxRadiusTiles); maxZ = Mathf.Min(maxZ + WindowMarginTiles, fz + WindowMaxRadiusTiles);
            int a = ChunkKey.FloorDiv(Mathf.FloorToInt(minX)), b = ChunkKey.FloorDiv(Mathf.CeilToInt(maxX)), c2 = ChunkKey.FloorDiv(Mathf.FloorToInt(minZ)), e = ChunkKey.FloorDiv(Mathf.CeilToInt(maxZ));
            propCenterX = ChunkKey.FloorDiv(Mathf.RoundToInt(fx)); propCenterZ = ChunkKey.FloorDiv(Mathf.RoundToInt(fz));
            bool changed = !hasPropWindow || a != propMinX || b != propMaxX || c2 != propMinZ || e != propMaxZ;
            propMinX = a; propMaxX = b; propMinZ = c2; propMaxZ = e; hasPropWindow = true;
            return changed;
        }

        private void WindowAround(int x, int z, int radius)
        {
            propMinX = ChunkKey.FloorDiv(x - radius); propMaxX = ChunkKey.FloorDiv(x + radius);
            propMinZ = ChunkKey.FloorDiv(z - radius); propMaxZ = ChunkKey.FloorDiv(z + radius);
            propCenterX = ChunkKey.FloorDiv(x); propCenterZ = ChunkKey.FloorDiv(z);
        }

        private int ChunkDistance(ChunkKey k) => Math.Max(Math.Abs(k.X - propCenterX), Math.Abs(k.Z - propCenterZ));

        // A chunk built a few rows per frame off to the side, then swapped in, so streaming never costs a frame more
        // than the budget plus one row and never shows a half-built chunk.
        private StaticProps.Chunk stagingChunk, spareChunk;
        private int stagingRow, stagingInvalidations;
        private bool stagingNearKnown;

        /// <summary>Builds missing or invalidated chunks of the window, nearest first, until the time budget is spent.</summary>
        private void BuildPropWindow(World world, Village v, float budgetMs)
        {
            propQueue.Clear();
            for (int cz = propMinZ; cz <= propMaxZ; cz++) for (int cx = propMinX; cx <= propMaxX; cx++)
            {
                var key = new ChunkKey(cx, cz, 0);
                if (!staticProps.TryGetChunk(key, out var chunk) || chunk.Dirty || !chunk.Built) propQueue.Add(key);
            }
            if (stagingChunk != null && !propQueue.Contains(stagingChunk.Key)) { spareChunk = stagingChunk; stagingChunk = null; }
            if (propQueue.Count == 0) return;
            propQueue.Sort(byDistanceToCenter);
            propWatch.Restart();
            int built = 0, next = 0;
            while (propWatch.Elapsed.TotalMilliseconds < budgetMs)
            {
                if (stagingChunk == null)
                {
                    bool found = false; var key = default(ChunkKey);
                    while (!found && next < propQueue.Count)
                    {
                        key = propQueue[next++];
                        found = !(staticProps.TryGetChunk(key, out var current) && current.Built && !current.Dirty);
                    }
                    if (!found) break;
                    stagingChunk = spareChunk ?? new StaticProps.Chunk();
                    spareChunk = null;
                    stagingChunk.Key = key; stagingChunk.Clear(); stagingChunk.Built = false; stagingChunk.Dirty = true;
                    stagingRow = 0; stagingNearKnown = TouchesKnown(key); stagingInvalidations = staticProps.Invalidations;
                }
                BuildRows(world, v, stagingChunk, stagingRow, stagingRow + 1, stagingNearKnown);
                if (++stagingRow < ChunkKey.Size) continue;
                FinishChunk(world, v, stagingChunk, stagingNearKnown);
                // Invalidated while it was being built: install it (better than a hole) but build it again later.
                if (staticProps.Invalidations != stagingInvalidations) stagingChunk.Dirty = true;
                spareChunk = staticProps.Replace(stagingChunk);
                stagingChunk = null;
                built++;
            }
            LastPropBuildMilliseconds = propWatch.Elapsed.TotalMilliseconds; LastPropBuildChunks = built;
            if (built > 0) propsActiveDirty = true;
        }

        /// <summary>Forgets cached chunks well outside the window once the cache is much larger than the window.</summary>
        private void EvictPropChunks()
        {
            int window = (propMaxX - propMinX + 1) * (propMaxZ - propMinZ + 1);
            if (staticProps.ChunkCount <= window * 2 + 32) return;
            propEvict.Clear();
            foreach (var chunk in staticProps.Chunks)
            {
                var k = chunk.Key;
                if (k.X < propMinX - 2 || k.X > propMaxX + 2 || k.Z < propMinZ - 2 || k.Z > propMaxZ + 2) propEvict.Add(k);
            }
            foreach (var k in propEvict) staticProps.Remove(k);
        }

        // ------------------------------------------------------------------ queries for tests and tools

        /// <summary>
        /// The tile's own static prop as drawn (resource, stump, road piece, fence post or gate), if any. A gate stands
        /// on its road piece; the prop added last, the gate, names the tile.
        /// </summary>
        internal bool TryGetStaticProp(Int2 tile, out string asset)
        {
            asset = "";
            if (staticProps == null || !staticProps.TryGetChunk(ChunkKey.Of(tile), out var chunk) || !chunk.Built) return false;
            foreach (var entry in chunk.Primary) if (entry.Tile.Equals(tile)) asset = entry.Asset;
            return asset != "";
        }

        /// <summary>
        /// Creates collider proxies for every drawn static prop whose asset passes the filter, under
        /// <paramref name="parent"/>, so tests and tools can ray-test the instanced scenery. Returns how many.
        /// </summary>
        internal int BuildStaticPropColliders(Transform parent, Predicate<string> asset)
        {
            if (staticProps == null || !WorldReady) return 0;
            staticProps.RecordAll = true;
            ReconcileWith(true);
            if (isActiveAndEnabled) BuildPropWindow(Game.CurrentWorld, Game.Selected, float.MaxValue);
            int count = 0;
            foreach (var chunk in staticProps.Chunks)
            {
                if (!chunk.Built || chunk.Dirty) continue;
                foreach (var entry in chunk.All)
                {
                    if (!asset(entry.Asset)) continue;
                    foreach (var part in staticProps.ModelOf(entry.Asset).Parts)
                    {
                        if (part.Submesh != 0) continue;
                        var m = entry.Matrix * part.Local;
                        var go = new GameObject(entry.Asset);
                        go.transform.SetParent(parent, false);
                        go.transform.SetPositionAndRotation(m.GetColumn(3), m.rotation);
                        go.transform.localScale = m.lossyScale;
                        go.AddComponent<MeshCollider>().sharedMesh = part.Mesh;
                        count++;
                    }
                }
            }
            return count;
        }

        // ------------------------------------------------------------------ measurement (opt-in)

        // `--forest-view-stats` logs reconcile, chunk-build and frame timings every 10 s and uncaps the frame rate so
        // frame time measures the work, not vsync. Off by default; used for the performance evidence only.
        private bool viewStats, propsHiddenForStats;
        private float nextViewStats;
        private readonly List<double> statFrames = new List<double>(), statReconcile = new List<double>(), statSkipped = new List<double>(), statBuild = new List<double>();
        private int statRuns, statSkips;
        private ProfilerRecorder mainThreadRecorder;
        private readonly FrameTiming[] frameTimings = new FrameTiming[1];
        private readonly List<double> statMain = new List<double>(), statGpu = new List<double>(), statSubmit = new List<double>();

        private void AwakeViewStats()
        {
            var args = Environment.GetCommandLineArgs();
            viewStats = Array.IndexOf(args, "--forest-view-stats") >= 0;
            if (!viewStats) return;
            // A/B measurement only: `--forest-view-stats --forest-props-off` builds everything but draws no static props.
            propsHiddenForStats = Array.IndexOf(args, "--forest-props-off") >= 0;
            mainThreadRecorder = ProfilerRecorder.StartNew(ProfilerCategory.Internal, "CPU Main Thread Frame Time");
        }

        private void TickViewStats(float dt)
        {
            if (!viewStats) return;
            QualitySettings.vSyncCount = 0; Application.targetFrameRate = -1;
            statFrames.Add(dt * 1000);
            if (mainThreadRecorder.Valid && mainThreadRecorder.LastValue > 0) statMain.Add(mainThreadRecorder.LastValue / 1e6);
            FrameTimingManager.CaptureFrameTimings();
            if (FrameTimingManager.GetLatestTimings(1, frameTimings) > 0 && frameTimings[0].gpuFrameTime > 0) statGpu.Add(frameTimings[0].gpuFrameTime);
            if (ReconcileRuns != statRuns) { statRuns = ReconcileRuns; statReconcile.Add(LastReconcileMilliseconds); }
            if (ReconcileSkips != statSkips) { statSkips = ReconcileSkips; statSkipped.Add(LastSkippedReconcileMilliseconds); }
            if (LastPropBuildChunks > 0) { statBuild.Add(LastPropBuildMilliseconds); LastPropBuildChunks = 0; }
            if (Time.unscaledTime < nextViewStats) return;
            nextViewStats = Time.unscaledTime + 10;
            string P(List<double> s) { if (s.Count == 0) return "n=0"; s.Sort(); return string.Format(CultureInfo.InvariantCulture, "n={0} p50={1:0.000} p95={2:0.000} max={3:0.000}", s.Count, s[s.Count / 2], s[Math.Min(s.Count - 1, (int)(s.Count * .95))], s[s.Count - 1]); }
            Debug.Log("view_stats: frame_ms[" + P(statFrames) + "] main_thread_ms[" + P(statMain) + "] gpu_ms[" + P(statGpu) + "] reconcile_ms[" + P(statReconcile) + "] reconcile_skipped_ms[" + P(statSkipped)
                + "] prop_build_ms[" + P(statBuild) + "] prop_submit_ms[" + P(statSubmit) + "] props draws=" + StaticPropDrawCalls + " instances=" + StaticPropInstances + " chunks=" + StaticPropChunks
                + " entities=" + entities.Count + " screen=" + Screen.width + "x" + Screen.height + " speed=" + Game.Speed.ToString(CultureInfo.InvariantCulture));
            statFrames.Clear(); statMain.Clear(); statGpu.Clear(); statReconcile.Clear(); statSkipped.Clear(); statBuild.Clear(); statSubmit.Clear();
        }
    }
}
