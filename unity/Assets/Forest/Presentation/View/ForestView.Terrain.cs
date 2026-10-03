using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.Rendering;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The rendered level and the shared ground. Ground is drawn in 16x16 chunks cached by key: a chunk is
    /// rebuilt only when its own tiles, the explored tiles near it or the level change, and a frame spends at
    /// most a small budget on building. Unexplored land is sampled from the coordinate-pure generator (never
    /// <see cref="World.TileAt"/>) and drawn as fog of war that fades in over three tiles at the explored edge;
    /// a dark <see cref="WorldSkirt"/> plane fills everything beyond the chunk window. There are no colliders:
    /// <see cref="TerrainRaycast"/> marches the ray across the rendered heightfield.
    /// </summary>
    public sealed partial class ForestView
    {
        private const int TerrainLayer = 8;
        internal const int TerrainChunkSize = 16;
        // Samples around a chunk: corner normals need two tiles, the fog fade needs three.
        private const int TerrainPad = 3;
        private const int TerrainSamples = TerrainChunkSize + 2 * TerrainPad;
        private const int TerrainCorners = TerrainChunkSize + 1;
        private const int FogFadeTiles = 3;
        private const float GroundInset = .025f;
        private const float TerrainBudgetMilliseconds = 3f;
        private const int TerrainChecksPerTick = 6;
        private const int GroundCenterReach = 38;
        private const int MaxWindowChunks = 14;
        private const float WindowMarginTiles = 6;
        // ClearWorld and ViewLevel write terrainSignature to request a full refresh; TerrainTick restores this value.
        private const int TerrainSignatureCurrent = 1;

        private int viewedLevel;
        private int terrainSignature;
        internal Transform groundRoot;
        internal Int2 groundCenter;

        private readonly Dictionary<TerrainKey, TerrainChunk> terrainChunks = new Dictionary<TerrainKey, TerrainChunk>();
        private readonly List<TerrainChunk> terrainPool = new List<TerrainChunk>();
        private readonly List<TerrainKey> terrainQueue = new List<TerrainKey>();
        private readonly List<TerrainKey> terrainScratch = new List<TerrainKey>();
        private readonly HashSet<Int2> terrainKnown = new HashSet<Int2>();
        private readonly HashSet<TerrainKey> terrainKnownChunks = new HashSet<TerrainKey>();
        private readonly Dictionary<Int2, TerrainTile> generatedTerrain = new Dictionary<Int2, TerrainTile>();
        private readonly Dictionary<long, float> terrainNoise = new Dictionary<long, float>();
        private readonly Dictionary<Int2, DungeonFloor> terrainFloors = new Dictionary<Int2, DungeonFloor>();
        private readonly Dictionary<Int2, DungeonStair> terrainOpenings = new Dictionary<Int2, DungeonStair>();
        private readonly Dictionary<DungeonFloor, DungeonSite> terrainFloorSites = new Dictionary<DungeonFloor, DungeonSite>();
        private World terrainWorld;
        private Int2 terrainKnownLast;
        private int terrainKnownProcessed, terrainLevel = int.MinValue, terrainDungeonShape = -1, terrainPlateaus = -1, terrainCheckCursor;
        private bool terrainQueueStale = true, terrainFirstBuild = true;
        private RectInt terrainWindow;
        private Material terrainMaterial, terrainWaterMaterial, skirtMaterial;
        private WorldSkirt skirt;
        private float terrainWorkMilliseconds;

        public int RenderedLevel => ControlledCat?.Position.Level ?? viewedLevel;

        /// <summary>Tiles covered by the current chunk window (camera footprint plus margin, and the reconcile radius).</summary>
        internal RectInt TerrainWindow => new RectInt(terrainWindow.x * TerrainChunkSize, terrainWindow.y * TerrainChunkSize, terrainWindow.width * TerrainChunkSize, terrainWindow.height * TerrainChunkSize);
        /// <summary>Milliseconds the terrain spent building in the last frame (telemetry and tests).</summary>
        internal float TerrainWorkMilliseconds => terrainWorkMilliseconds;
        /// <summary>Number of built chunks on the rendered level.</summary>
        internal int TerrainChunkCount => terrainChunks.Count;
        /// <summary>Sky-free background colour of unexplored land and the skirt, in sRGB.</summary>
        internal static Color TerrainSkirtColor(int level) => level < 0 ? new Color(.035f, .032f, .036f) : new Color(.085f, .125f, .118f);

        public void ViewLevel(int level)
        {
            if (DirectControl || level > 0 || level < -8 || level != 0 && !Game.Selected.Known.Any(p => p.Level == level)) return;
            viewedLevel = level; followSelectedCat = false; nextReconcile = 0;
            var place = level == 0 ? Game.Selected.Center : Game.Selected.Known.Where(p => p.Level == level).OrderBy(p => Math.Abs(p.X - focus.x) + Math.Abs(p.Z - focus.z)).First();
            focus = At(place); terrainSignature = int.MinValue; ApplyManagementCamera();
        }

        partial void AwakeTerrain()
        {
            var shader = Resources.Load<Shader>("ForestTerrain");
            if (shader == null || !shader.isSupported)
            {
                Debug.LogError("ForestTerrain shader is missing or unsupported; terrain falls back to the plain surface material.");
                terrainMaterial = new Material(Material("ground:fallback", Color.white));
                skirtMaterial = new Material(Material("ground:skirt-fallback", TerrainSkirtColor(0), true));
            }
            else
            {
                terrainMaterial = new Material(shader) { name = "Forest terrain", enableInstancing = false };
                skirtMaterial = new Material(shader) { name = "Forest world skirt" };
                skirtMaterial.SetFloat("_Skirt", 1); skirtMaterial.SetFloat("_ZWrite", 0);
                // Drawn first and without depth so every piece of real ground and scenery covers it.
                skirtMaterial.renderQueue = (int)RenderQueue.Geometry - 10;
            }
            materials["terrain:ground"] = terrainMaterial; materials["terrain:skirt"] = skirtMaterial;
            terrainWaterMaterial = Material("ground:water", Color.white);
            var water = Resources.Load<Shader>("ForestWater");
            if (water != null) terrainWaterMaterial.shader = water;
            skirt = new WorldSkirt(groundRoot, skirtMaterial, TerrainLayer);
            ApplyTerrainPalette(0);
        }

        partial void LevelChangedTerrain(int level)
        {
            ApplyTerrainPalette(level);
            terrainFirstBuild = true; terrainQueueStale = true;
        }

        partial void UpdateTerrain(float dt)
        {
            var timer = System.Diagnostics.Stopwatch.StartNew();
            PrepareTerrain();
            UpdateTerrainWindow();
            BuildQueuedChunks(terrainFirstBuild ? float.PositiveInfinity : TerrainBudgetMilliseconds, timer);
            terrainFirstBuild = false;
            skirt?.Place(Camera, RenderedLevel * 4 + (RenderedLevel < 0 ? -.1f : -.05f));
            terrainWorkMilliseconds = (float)timer.Elapsed.TotalMilliseconds;
        }

        /// <summary>Marks chunks whose tiles changed; called from Reconcile (five times a second).</summary>
        private void TerrainTick()
        {
            PrepareTerrain();
            CheckTerrainSignatures();
        }

        /// <summary>Rebuilds every chunk of the current window now, without the frame budget.</summary>
        internal void MakeGround()
        {
            PrepareTerrain();
            foreach (var chunk in terrainChunks.Values) chunk.Dirty = true;
            UpdateTerrainWindow(force: true);
            BuildQueuedChunks(float.PositiveInfinity, System.Diagnostics.Stopwatch.StartNew());
        }

        /// <summary>Where a screen ray meets the rendered ground of the rendered level. Marches the heightfield; no physics.</summary>
        internal bool TerrainRaycast(Ray ray, out Vector3 point)
        {
            point = default;
            if (!WorldReady) return false;
            int level = RenderedLevel;
            float top = level * 4 + 12, bottom = level * 4 - 10;
            foreach (var chunk in terrainChunks.Values) if (chunk.Built && chunk.Key.Level == level) { top = Mathf.Max(top, chunk.MaxY + 1); bottom = Mathf.Min(bottom, chunk.MinY - 1); }
            Vector3 o = ray.origin, d = ray.direction;
            float start = 0, end = 240;
            if (Mathf.Abs(d.y) > 1e-5f)
            {
                float a = (top - o.y) / d.y, b = (bottom - o.y) / d.y;
                start = Mathf.Max(0, Mathf.Min(a, b)); end = Mathf.Min(end, Mathf.Max(a, b));
            }
            else if (o.y > top || o.y < bottom) return false;
            if (end <= start) return false;
            float horizontal = new Vector2(d.x, d.z).magnitude;
            float step = horizontal > 1e-4f ? Mathf.Min(.2f / horizontal, end - start) : end - start;
            float Above(float t) { var p = o + d * t; return p.y - SurfaceHeightAt(p.x, p.z, level); }
            float previous = start;
            if (Above(start) <= 0) { point = o + d * start; return true; }
            for (float t = start + step; ; t += step)
            {
                t = Mathf.Min(t, end);
                if (Above(t) <= 0)
                {
                    float lo = previous, hi = t;
                    for (int i = 0; i < 14; i++) { float mid = (lo + hi) * .5f; if (Above(mid) <= 0) hi = mid; else lo = mid; }
                    point = o + d * hi; return true;
                }
                if (t >= end) return false;
                previous = t;
            }
        }

        /// <summary>The rendered ground height at a world point (water reads as its surface); built chunks only, else the walk height.</summary>
        internal float SurfaceHeightAt(float x, float z, int level)
        {
            int tx = Mathf.RoundToInt(x), tz = Mathf.RoundToInt(z);
            if (!terrainChunks.TryGetValue(TerrainKey.Of(tx, tz, level), out var chunk) || !chunk.Built)
                return WorldReady ? (float)Game.CurrentWorld.WalkHeight(new Int2(tx, tz, level)) - GroundInset : level * 4;
            int i = tx - chunk.Key.X * TerrainChunkSize, j = tz - chunk.Key.Z * TerrainChunkSize;
            float water = chunk.WaterTop[i + j * TerrainChunkSize];
            if (!float.IsNaN(water)) return water;
            float u = x - tx, v = z - tz, c = chunk.Centers[i + j * TerrainChunkSize];
            float Corner(int ci, int cj) => chunk.Corners[i + ci + (j + cj) * TerrainCorners];
            float a = Corner(0, 0), b = Corner(1, 0), dd = Corner(1, 1), e = Corner(0, 1);
            if (Mathf.Abs(u) >= Mathf.Abs(v))
                return u >= 0 ? c + (u - v) * (b - c) + (u + v) * (dd - c) : c + (-u - v) * (a - c) + (v - u) * (e - c);
            return v >= 0 ? c + (v - u) * (e - c) + (v + u) * (dd - c) : c + (-u - v) * (a - c) + (u - v) * (b - c);
        }

        /// <summary>
        /// Rendered height and exploration (1 explored, 0 deep fog) of a tile from the built chunks; never generates
        /// terrain. Edge scenery uses it to sit on, and fade with, the fogged ground.
        /// </summary>
        internal bool TryTerrainSample(int x, int z, out float height, out float explored)
        {
            height = 0; explored = 0;
            if (!terrainChunks.TryGetValue(TerrainKey.Of(x, z, RenderedLevel), out var chunk) || !chunk.Built) return false;
            int index = x - chunk.Key.X * TerrainChunkSize + (z - chunk.Key.Z * TerrainChunkSize) * TerrainChunkSize;
            height = chunk.Centers[index]; explored = chunk.Explored[index];
            return true;
        }

        // ---------------------------------------------------------------------------------------------
        // State and invalidation

        private void PrepareTerrain()
        {
            if (!WorldReady) return;
            var world = Game.CurrentWorld; var village = Game.Selected;
            if (!ReferenceEquals(world, terrainWorld))
            {
                terrainWorld = world;
                generatedTerrain.Clear(); terrainNoise.Clear();
                ResetTerrainKnown(); terrainDungeonShape = -1; terrainPlateaus = -1;
                foreach (var chunk in terrainChunks.Values) RetireChunk(chunk);
                terrainChunks.Clear(); terrainFirstBuild = true;
            }
            if (terrainSignature != TerrainSignatureCurrent)
            {
                terrainSignature = TerrainSignatureCurrent;
                ResetTerrainKnown();
                foreach (var chunk in terrainChunks.Values) chunk.Dirty = true;
                terrainQueueStale = true;
            }
            if (world.SurfacePlateaus.Count != terrainPlateaus)
            {
                terrainPlateaus = world.SurfacePlateaus.Count; generatedTerrain.Clear();
                foreach (var chunk in terrainChunks.Values) chunk.Dirty = true;
                terrainQueueStale = true;
            }
            int shape = DungeonShape(world);
            if (shape != terrainDungeonShape)
            {
                terrainDungeonShape = shape; terrainFloors.Clear(); terrainOpenings.Clear(); terrainFloorSites.Clear();
                foreach (var site in world.Dungeons)
                {
                    foreach (var floor in site.Floors) { terrainFloorSites[floor] = site; foreach (var cell in floor.Cells) terrainFloors[cell] = floor; }
                    foreach (var stair in site.Stairs) foreach (var opening in stair.Opening) terrainOpenings[opening] = stair;
                }
                foreach (var chunk in terrainChunks.Values) chunk.Dirty = true;
                terrainQueueStale = true;
            }
            var known = village.Known;
            // Known only grows by appending; a shorter or rewritten list (a load, a test fixture) restarts the scan.
            if (known.Count < terrainKnownProcessed || terrainKnownProcessed > 0 && !known[terrainKnownProcessed - 1].Equals(terrainKnownLast))
            { ResetTerrainKnown(); foreach (var chunk in terrainChunks.Values) chunk.Dirty = true; }
            for (; terrainKnownProcessed < known.Count; terrainKnownProcessed++)
            {
                var p = terrainKnownLast = known[terrainKnownProcessed];
                if (!terrainKnown.Add(p)) continue;
                terrainKnownChunks.Add(TerrainKey.Of(p.X, p.Z, p.Level));
                // A newly explored tile changes its own colour and the fog fade of tiles up to four away.
                int reach = FogFadeTiles + 1;
                for (int cz = FloorDiv(p.Z - reach, TerrainChunkSize); cz <= FloorDiv(p.Z + reach, TerrainChunkSize); cz++)
                    for (int cx = FloorDiv(p.X - reach, TerrainChunkSize); cx <= FloorDiv(p.X + reach, TerrainChunkSize); cx++)
                        if (terrainChunks.TryGetValue(new TerrainKey(cx, cz, p.Level), out var chunk) && !chunk.Dirty) { chunk.Dirty = true; terrainQueueStale = true; }
            }
        }

        private void ResetTerrainKnown() { terrainKnown.Clear(); terrainKnownChunks.Clear(); terrainKnownProcessed = 0; terrainQueueStale = true; }

        private static int DungeonShape(World world)
        {
            int shape = world.Dungeons.Count;
            foreach (var site in world.Dungeons) { shape = unchecked(shape * 31 + site.Floors.Count); shape = unchecked(shape * 31 + site.Stairs.Count); foreach (var floor in site.Floors) shape = unchecked(shape * 31 + floor.Cells.Count); }
            return shape;
        }

        /// <summary>Round-robin content check: a few built chunks per tick compare a hash of their persisted tiles.</summary>
        private void CheckTerrainSignatures()
        {
            if (!WorldReady || terrainChunks.Count == 0) return;
            terrainScratch.Clear(); terrainScratch.AddRange(terrainChunks.Keys);
            for (int n = 0; n < Mathf.Min(TerrainChecksPerTick, terrainScratch.Count); n++)
            {
                terrainCheckCursor = (terrainCheckCursor + 1) % terrainScratch.Count;
                var chunk = terrainChunks[terrainScratch[terrainCheckCursor]];
                if (chunk.Dirty || !chunk.Built) continue;
                if (ContentSignature(chunk.Key) != chunk.Signature) { chunk.Dirty = true; terrainQueueStale = true; }
            }
        }

        private int ContentSignature(TerrainKey key)
        {
            var world = Game.CurrentWorld; double wearToDirt = Math.Max(1, world.CurrentPacing().PathWearToDirt);
            int x0 = key.X * TerrainChunkSize, z0 = key.Z * TerrainChunkSize, signature = 17;
            for (int z = z0 - 1; z <= z0 + TerrainChunkSize; z++)
                for (int x = x0 - 1; x <= x0 + TerrainChunkSize; x++)
                {
                    var t = world.GetTile(new Int2(x, z, key.Level));
                    signature = unchecked(signature * 31 + (t == null ? 0 : TileSignature(t, wearToDirt)));
                }
            return signature;
        }

        private static int TileSignature(Tile t, double wearToDirt)
        {
            int flags = (t.Water ? 1 : 0) | (t.Mountain ? 2 : 0) | (t.Dirt ? 4 : 0) | (t.Road ? 8 : 0) | (t.Bridge ? 16 : 0) | (t.Wall ? 32 : 0) | (t.Amount <= 0 ? 64 : 0) | (t.Resource == "" ? 128 : 0);
            int wear = (int)Math.Min(8, t.PathWear / wearToDirt * 8);
            return unchecked(((t.Elevation.GetHashCode() * 31 + t.WaterDepth.GetHashCode()) * 31 + t.WaterSurface.GetHashCode()) * 31 + flags * 16 + wear + (t.Biome?.GetHashCode() ?? 0) * 7 + (t.Overlay?.Length ?? 0) * 13 + (t.Landform?.GetHashCode() ?? 0) * 3);
        }

        // ---------------------------------------------------------------------------------------------
        // Window, queue and chunk lifetime

        private void UpdateTerrainWindow(bool force = false)
        {
            if (!WorldReady) return;
            int level = RenderedLevel;
            if (level != terrainLevel) { terrainLevel = level; ApplyTerrainPalette(level); terrainQueueStale = true; }
            float baseY = level * 4;
            RectInt chunks;
            var reach = new RectInt(FloorDiv(groundCenter.X - GroundCenterReach, TerrainChunkSize), FloorDiv(groundCenter.Z - GroundCenterReach, TerrainChunkSize), 0, 0);
            reach.width = FloorDiv(groundCenter.X + GroundCenterReach, TerrainChunkSize) - reach.x + 1;
            reach.height = FloorDiv(groundCenter.Z + GroundCenterReach, TerrainChunkSize) - reach.y + 1;
            if (Camera != null && WorldSkirt.TryFootprint(Camera, baseY - 3, baseY + 5, 150, out var footprint))
            {
                int xMin = FloorDiv(Mathf.FloorToInt(footprint.xMin - WindowMarginTiles), TerrainChunkSize), xMax = FloorDiv(Mathf.CeilToInt(footprint.xMax + WindowMarginTiles), TerrainChunkSize);
                int zMin = FloorDiv(Mathf.FloorToInt(footprint.yMin - WindowMarginTiles), TerrainChunkSize), zMax = FloorDiv(Mathf.CeilToInt(footprint.yMax + WindowMarginTiles), TerrainChunkSize);
                // A runaway footprint (a ray grazing the horizon) is trimmed around the frustum centre.
                int cx = FloorDiv(Mathf.RoundToInt(footprint.center.x), TerrainChunkSize), cz = FloorDiv(Mathf.RoundToInt(footprint.center.y), TerrainChunkSize);
                int half = MaxWindowChunks / 2;
                xMin = Mathf.Max(xMin, cx - half); xMax = Mathf.Min(xMax, cx + half); zMin = Mathf.Max(zMin, cz - half); zMax = Mathf.Min(zMax, cz + half);
                chunks = new RectInt(Mathf.Min(xMin, reach.x), Mathf.Min(zMin, reach.y), 0, 0);
                chunks.width = Mathf.Max(xMax, reach.xMax - 1) - chunks.x + 1; chunks.height = Mathf.Max(zMax, reach.yMax - 1) - chunks.y + 1;
            }
            else chunks = reach;
            if (!force && chunks.Equals(terrainWindow) && !terrainQueueStale) return;
            terrainWindow = chunks;
            // Retire chunks of other levels and those well outside the window (one chunk of hysteresis).
            terrainScratch.Clear();
            foreach (var pair in terrainChunks)
            {
                var k = pair.Key;
                if (k.Level != level || k.X < chunks.xMin - 1 || k.X > chunks.xMax || k.Z < chunks.yMin - 1 || k.Z > chunks.yMax) terrainScratch.Add(k);
            }
            foreach (var k in terrainScratch) { RetireChunk(terrainChunks[k]); terrainChunks.Remove(k); }
            terrainQueue.Clear();
            for (int z = chunks.yMin; z < chunks.yMax; z++)
                for (int x = chunks.xMin; x < chunks.xMax; x++)
                {
                    var k = new TerrainKey(x, z, level);
                    if (!terrainChunks.TryGetValue(k, out var chunk) || chunk.Dirty || !chunk.Built) terrainQueue.Add(k);
                }
            var center = Camera != null && WorldSkirt.TryFootprint(Camera, baseY, baseY, 150, out var view) ? view.center : new Vector2(groundCenter.X, groundCenter.Z);
            float Distance(TerrainKey k) { float dx = (k.X + .5f) * TerrainChunkSize - center.x, dz = (k.Z + .5f) * TerrainChunkSize - center.y; return dx * dx + dz * dz; }
            terrainQueue.Sort((a, b) => Distance(a).CompareTo(Distance(b)));
            terrainQueueStale = false;
        }

        private void BuildQueuedChunks(float budgetMilliseconds, System.Diagnostics.Stopwatch timer)
        {
            if (!WorldReady) return;
            int built = 0;
            while (terrainQueue.Count > 0)
            {
                if (built > 0 && timer.Elapsed.TotalMilliseconds >= budgetMilliseconds) break;
                var key = terrainQueue[0]; terrainQueue.RemoveAt(0);
                if (key.Level != RenderedLevel) continue;
                if (!terrainChunks.TryGetValue(key, out var chunk)) { chunk = TakeChunk(key); terrainChunks[key] = chunk; }
                else if (chunk.Built && !chunk.Dirty) continue;
                BuildChunk(chunk); built++;
            }
        }

        private TerrainChunk TakeChunk(TerrainKey key)
        {
            TerrainChunk chunk;
            if (terrainPool.Count > 0) { chunk = terrainPool[terrainPool.Count - 1]; terrainPool.RemoveAt(terrainPool.Count - 1); }
            else
            {
                chunk = new TerrainChunk();
                var root = new GameObject("Terrain", typeof(MeshFilter), typeof(MeshRenderer)) { layer = TerrainLayer };
                root.transform.SetParent(groundRoot, false);
                chunk.Root = root; chunk.Renderer = root.GetComponent<MeshRenderer>();
                chunk.Ground = new Mesh { name = "Forest terrain chunk" }; chunk.Ground.MarkDynamic();
                root.GetComponent<MeshFilter>().sharedMesh = chunk.Ground;
                chunk.Renderer.sharedMaterial = terrainMaterial;
                chunk.Renderer.shadowCastingMode = ShadowCastingMode.On; chunk.Renderer.receiveShadows = true;
                chunk.Renderer.lightProbeUsage = LightProbeUsage.Off; chunk.Renderer.reflectionProbeUsage = ReflectionProbeUsage.Off;
                var water = new GameObject("Water", typeof(MeshFilter), typeof(MeshRenderer)) { layer = TerrainLayer };
                water.transform.SetParent(root.transform, false);
                chunk.Water = new Mesh { name = "Forest water chunk" };
                water.GetComponent<MeshFilter>().sharedMesh = chunk.Water;
                chunk.WaterRenderer = water.GetComponent<MeshRenderer>();
                chunk.WaterRenderer.sharedMaterial = terrainWaterMaterial;
                chunk.WaterRenderer.shadowCastingMode = ShadowCastingMode.Off; chunk.WaterRenderer.receiveShadows = false;
                chunk.WaterRenderer.lightProbeUsage = LightProbeUsage.Off; chunk.WaterRenderer.reflectionProbeUsage = ReflectionProbeUsage.Off;
            }
            chunk.Key = key; chunk.Built = false; chunk.Dirty = true;
            chunk.Root.name = ChunkName(key);
            return chunk;
        }

        // Pooled chunks stay active with disabled renderers so ForestView.OnDestroy still finds and frees their meshes.
        private void RetireChunk(TerrainChunk chunk)
        {
            chunk.Renderer.enabled = false; chunk.WaterRenderer.enabled = false; chunk.Built = false;
            chunk.Root.name = "Terrain (pooled)";
            if (terrainPool.Count < 96) { terrainPool.Add(chunk); return; }
            Destroy(chunk.Ground); Destroy(chunk.Water); Destroy(chunk.Root);
        }

        internal static string ChunkName(TerrainKey key) => "Terrain " + key.X + "," + key.Z + "," + key.Level;

        private void ApplyTerrainPalette(int level)
        {
            var color = TerrainSkirtColor(level);
            if (terrainMaterial != null && terrainMaterial.HasProperty("_SkirtColor")) terrainMaterial.SetColor("_SkirtColor", color);
            if (skirtMaterial != null) { if (skirtMaterial.HasProperty("_SkirtColor")) skirtMaterial.SetColor("_SkirtColor", color); else skirtMaterial.color = color; }
        }

        private static int FloorDiv(int value, int size) => value >= 0 ? value / size : (value - size + 1) / size;

        // ---------------------------------------------------------------------------------------------
        // Sampling

        private struct TerrainTile
        {
            public float Elevation, WaterSurface, WaterDepth, Soil;
            public string Biome, Landform;
            public bool Water, Mountain;
        }

        private struct GroundSample
        {
            public float Height, WaterSurface, Depth, Explored;
            public Color Color;
            public bool Water, Opening, Known;
        }

        private static readonly GroundSample[] samples = new GroundSample[TerrainSamples * TerrainSamples];
        private static readonly bool[] knownGrid = new bool[(TerrainSamples + 2 * FogFadeTiles) * (TerrainSamples + 2 * FogFadeTiles)];

        private static TerrainTile FromTile(Tile t, double wearToDirt)
        {
            float soil = (float)Math.Min(1, t.PathWear / wearToDirt);
            // Founding rings and deposit approaches are marked dirt without wear: a light trodden tint, not a hole.
            if (t.Dirt && t.PathWear < wearToDirt) soil = Mathf.Max(soil, t.Landform == "trail" ? .55f : .32f);
            if (t.Road || t.Overlay == "road_built") soil = Mathf.Max(soil, .6f);
            if (!t.Water && t.Resource != "" && t.Resource != "fish" && t.Amount <= 0) soil = Mathf.Max(soil, .45f);
            return new TerrainTile { Elevation = (float)t.Elevation, WaterSurface = (float)t.WaterSurface, WaterDepth = (float)t.WaterDepth, Water = t.Water, Mountain = t.Mountain, Biome = t.Biome, Landform = t.Landform, Soil = soil };
        }

        private TerrainTile SurfaceTile(Int2 p, World world, double wearToDirt)
        {
            var tile = world.GetTile(p);
            if (tile != null) return FromTile(tile, wearToDirt);
            if (!generatedTerrain.TryGetValue(p, out var generated))
            {
                if (generatedTerrain.Count > 160000) generatedTerrain.Clear();
                generated = FromTile(world.GenerateSurfaceTile(p), wearToDirt);
                generatedTerrain[p] = generated;
            }
            return generated;
        }

        private float LowNoise(int x, int z, int salt)
        {
            const int cell = 6;
            int gx = FloorDiv(x, cell), gz = FloorDiv(z, cell);
            float fx = (x - gx * cell) / (float)cell, fz = (z - gz * cell) / (float)cell;
            float Lattice(int lx, int lz)
            {
                long key = ((long)lx << 34) ^ ((long)(lz & 0x3FFFFFFF) << 2) ^ (uint)salt;
                if (!terrainNoise.TryGetValue(key, out var value))
                {
                    if (terrainNoise.Count > 60000) terrainNoise.Clear();
                    value = World.Hash(lx + ":" + lz + ":ground:" + salt) % 10007 / 5003.5f - 1;
                    terrainNoise[key] = value;
                }
                return value;
            }
            fx = fx * fx * (3 - 2 * fx); fz = fz * fz * (3 - 2 * fz);
            return Mathf.Lerp(Mathf.Lerp(Lattice(gx, gz), Lattice(gx + 1, gz), fx), Mathf.Lerp(Lattice(gx, gz + 1), Lattice(gx + 1, gz + 1), fx), fz);
        }

        private static readonly Color Soil = new Color(.45f, .35f, .23f);

        private static Color BiomeColor(string biome, string landform, bool mountain)
        {
            if (mountain) return new Color(.47f, .47f, .44f);
            switch (biome)
            {
                case "meadow": case "grassland": case "flower_forest": return landform == "settlement" || landform == "terrace" ? new Color(.47f, .57f, .31f) : new Color(.50f, .58f, .32f);
                case "savanna": return new Color(.58f, .57f, .35f);
                case "pine_forest": case "taiga": return new Color(.25f, .39f, .29f);
                case "snow_forest": case "tundra": return new Color(.62f, .66f, .62f);
                case "wetland": case "marsh": case "swamp": return new Color(.31f, .42f, .30f);
                case "highland": case "hills": return new Color(.50f, .51f, .40f);
                case "mountains": return new Color(.47f, .47f, .44f);
                case "snow_mountains": return new Color(.84f, .86f, .87f);
                case "desert": case "beach": return new Color(.70f, .63f, .45f);
                case "badlands": return new Color(.62f, .45f, .32f);
                case "woodland": return new Color(.38f, .49f, .29f);
                default: return new Color(.33f, .47f, .28f);
            }
        }

        private GroundSample SampleSurface(Int2 p, World world, double wearToDirt)
        {
            var t = SurfaceTile(p, world, wearToDirt);
            var s = new GroundSample { Opening = terrainOpenings.ContainsKey(p) };
            if (t.Water)
            {
                bool deep = t.WaterDepth > 0;
                s.Water = true; s.WaterSurface = deep ? t.WaterSurface : t.Elevation; s.Depth = deep ? t.WaterDepth : .3f;
                s.Height = deep ? t.Elevation : t.Elevation - .3f;
                s.Color = Color.Lerp(new Color(.34f, .36f, .27f), new Color(.13f, .19f, .21f), Mathf.Clamp01(s.Depth / 3));
            }
            else
            {
                s.Height = t.Elevation;
                var color = Color.Lerp(BiomeColor(t.Biome, t.Landform, t.Mountain), Soil, t.Soil);
                float value = 1 + .06f * LowNoise(p.X, p.Z, 1) + Mathf.Clamp(t.Elevation * .025f, -.08f, .08f);
                float hue = .025f * LowNoise(p.X + 101, p.Z - 57, 2);
                s.Color = new Color(color.r * value + hue, color.g * value, color.b * value - hue * .5f);
            }
            return s;
        }

        private GroundSample SampleUnderground(Int2 p, World world)
        {
            var s = new GroundSample();
            if (terrainFloors.TryGetValue(p, out var floor))
            {
                // Floor models cover these cells; the ground beneath is bare soil or stone, never grass.
                bool vault = terrainFloorSites.TryGetValue(floor, out var site) && site.Theme == "stone_vault";
                s.Height = (float)floor.Elevation - .06f;
                var color = vault ? new Color(.34f, .33f, .32f) : new Color(.36f, .28f, .20f);
                float value = 1 + .07f * LowNoise(p.X, p.Z, 3);
                s.Color = color * value; s.Color.a = 1;
                return s;
            }
            if (terrainOpenings.TryGetValue(p, out var stair))
            {
                s.Opening = true; s.Height = (float)world.DungeonHeight(p);
                s.Color = new Color(.30f, .24f, .18f);
                return s;
            }
            // Bedrock: a dark stone plane level with the nearest site's floors.
            float baseElevation = 0; int best = int.MaxValue;
            foreach (var d in world.Dungeons) { int distance = Math.Abs(d.Entrance.X - p.X) + Math.Abs(d.Entrance.Z - p.Z); if (distance < best) { best = distance; baseElevation = (float)d.BaseElevation; } }
            s.Height = baseElevation + p.Level * 4 - .03f;
            float rock = 1 + .08f * LowNoise(p.X, p.Z, 4);
            s.Color = new Color(.115f * rock, .11f * rock, .11f * rock);
            return s;
        }

        // ---------------------------------------------------------------------------------------------
        // Mesh building

        private static readonly List<Vector3> meshVertices = new List<Vector3>(1024);
        private static readonly List<Vector3> meshNormals = new List<Vector3>(1024);
        private static readonly List<Color> meshColors = new List<Color>(1024);
        private static readonly List<int> meshIndices = new List<int>(4096);
        private static readonly float[] cornerHeights = new float[(TerrainCorners + 2) * (TerrainCorners + 2)];

        private void BuildChunk(TerrainChunk chunk)
        {
            var world = Game.CurrentWorld; double wearToDirt = Math.Max(1, world.CurrentPacing().PathWearToDirt);
            var key = chunk.Key; int level = key.Level;
            int x0 = key.X * TerrainChunkSize, z0 = key.Z * TerrainChunkSize;
            int sx0 = x0 - TerrainPad, sz0 = z0 - TerrainPad;

            // Explored flags around the chunk, then the Chebyshev distance to the nearest explored tile (capped).
            int kw = TerrainSamples + 2 * FogFadeTiles, kx0 = sx0 - FogFadeTiles, kz0 = sz0 - FogFadeTiles;
            bool anyKnown = false;
            for (int cz = FloorDiv(kz0, TerrainChunkSize); cz <= FloorDiv(kz0 + kw - 1, TerrainChunkSize) && !anyKnown; cz++)
                for (int cx = FloorDiv(kx0, TerrainChunkSize); cx <= FloorDiv(kx0 + kw - 1, TerrainChunkSize); cx++)
                    if (terrainKnownChunks.Contains(new TerrainKey(cx, cz, level))) { anyKnown = true; break; }
            for (int j = 0; j < kw; j++)
                for (int i = 0; i < kw; i++)
                    knownGrid[i + j * kw] = anyKnown && terrainKnown.Contains(new Int2(kx0 + i, kz0 + j, level));

            for (int j = 0; j < TerrainSamples; j++)
                for (int i = 0; i < TerrainSamples; i++)
                {
                    var p = new Int2(sx0 + i, sz0 + j, level);
                    var s = level == 0 ? SampleSurface(p, world, wearToDirt) : SampleUnderground(p, world);
                    int distance = FogFadeTiles + 1;
                    if (anyKnown)
                        for (int r = 0; r <= FogFadeTiles && distance > FogFadeTiles; r++)
                            for (int dz = -r; dz <= r && distance > FogFadeTiles; dz++)
                                for (int dx = -r; dx <= r; dx++)
                                {
                                    if (Math.Max(Math.Abs(dx), Math.Abs(dz)) != r) continue;
                                    if (knownGrid[i + FogFadeTiles + dx + (j + FogFadeTiles + dz) * kw]) { distance = r; break; }
                                }
                    s.Known = distance == 0;
                    s.Explored = 1 - distance / (float)(FogFadeTiles + 1);
                    samples[i + j * TerrainSamples] = s;
                }

            GroundSample Sample(int i, int j) => samples[i + TerrainPad + (j + TerrainPad) * TerrainSamples];
            // Corner (ci, cj) sits at (x0 + ci - .5, z0 + cj - .5), between tiles ci - 1 and ci; ci runs -1..17.
            int cw = TerrainCorners + 2;
            float minY = float.MaxValue, maxY = float.MinValue;
            for (int cj = -1; cj <= TerrainCorners; cj++)
                for (int ci = -1; ci <= TerrainCorners; ci++)
                {
                    float h = (Sample(ci - 1, cj - 1).Height + Sample(ci, cj - 1).Height + Sample(ci - 1, cj).Height + Sample(ci, cj).Height) * .25f - GroundInset;
                    cornerHeights[ci + 1 + (cj + 1) * cw] = h;
                }
            float CornerHeight(int ci, int cj) => cornerHeights[ci + 1 + (cj + 1) * cw];

            meshVertices.Clear(); meshNormals.Clear(); meshColors.Clear(); meshIndices.Clear();
            for (int cj = 0; cj < TerrainCorners; cj++)
                for (int ci = 0; ci < TerrainCorners; ci++)
                {
                    float h = CornerHeight(ci, cj);
                    chunk.Corners[ci + cj * TerrainCorners] = h;
                    meshVertices.Add(new Vector3(x0 + ci - .5f, h, z0 + cj - .5f));
                    meshNormals.Add(new Vector3(CornerHeight(ci - 1, cj) - CornerHeight(ci + 1, cj), 2, CornerHeight(ci, cj - 1) - CornerHeight(ci, cj + 1)).normalized);
                    var a = Sample(ci - 1, cj - 1); var b = Sample(ci, cj - 1); var c = Sample(ci - 1, cj); var d = Sample(ci, cj);
                    var color = (a.Color + b.Color + c.Color + d.Color) * .25f;
                    meshColors.Add(Vertex(color, (a.Explored + b.Explored + c.Explored + d.Explored) * .25f));
                    minY = Mathf.Min(minY, h); maxY = Mathf.Max(maxY, h);
                }
            int centers = meshVertices.Count;
            for (int j = 0; j < TerrainChunkSize; j++)
                for (int i = 0; i < TerrainChunkSize; i++)
                {
                    var s = Sample(i, j);
                    float h = s.Height - GroundInset;
                    int index = i + j * TerrainChunkSize;
                    chunk.Centers[index] = h; chunk.Explored[index] = s.Explored; chunk.WaterTop[index] = s.Water ? s.WaterSurface - .015f : float.NaN;
                    meshVertices.Add(new Vector3(x0 + i, h, z0 + j));
                    var n = meshNormals[i + j * TerrainCorners] + meshNormals[i + 1 + j * TerrainCorners] + meshNormals[i + (j + 1) * TerrainCorners] + meshNormals[i + 1 + (j + 1) * TerrainCorners];
                    meshNormals.Add(n.normalized);
                    // Half the tile's own colour keeps a faint texture without hard tile edges.
                    var corners = (meshColors[i + j * TerrainCorners] + meshColors[i + 1 + j * TerrainCorners] + meshColors[i + (j + 1) * TerrainCorners] + meshColors[i + 1 + (j + 1) * TerrainCorners]) * .25f;
                    var own = Vertex(s.Color, s.Explored);
                    var center = Color.Lerp(own, corners, .5f); center.a = own.a;
                    meshColors.Add(center);
                    minY = Mathf.Min(minY, h); maxY = Mathf.Max(maxY, h);
                }
            for (int j = 0; j < TerrainChunkSize; j++)
                for (int i = 0; i < TerrainChunkSize; i++)
                {
                    if (Sample(i, j).Opening) continue;
                    int c = centers + i + j * TerrainChunkSize;
                    int a = i + j * TerrainCorners, b = a + 1, e = a + TerrainCorners, d = e + 1;
                    meshIndices.Add(c); meshIndices.Add(a); meshIndices.Add(e);
                    meshIndices.Add(c); meshIndices.Add(e); meshIndices.Add(d);
                    meshIndices.Add(c); meshIndices.Add(d); meshIndices.Add(b);
                    meshIndices.Add(c); meshIndices.Add(b); meshIndices.Add(a);
                }
            // Stair openings become a shaft: inward walls from the surrounding ground down to the floor below, and a dark pit.
            for (int j = 0; j < TerrainChunkSize; j++)
                for (int i = 0; i < TerrainChunkSize; i++)
                {
                    if (!Sample(i, j).Opening) continue;
                    var p = new Int2(x0 + i, z0 + j, level);
                    float bottom = terrainOpenings.TryGetValue(p, out var stair) ? (float)world.DungeonHeight(stair.To) - .02f : CornerHeight(i, j) - 4;
                    minY = Mathf.Min(minY, bottom);
                    var c00 = new Vector3(x0 + i - .5f, CornerHeight(i, j), z0 + j - .5f);
                    var c10 = new Vector3(x0 + i + .5f, CornerHeight(i + 1, j), z0 + j - .5f);
                    var c11 = new Vector3(x0 + i + .5f, CornerHeight(i + 1, j + 1), z0 + j + .5f);
                    var c01 = new Vector3(x0 + i - .5f, CornerHeight(i, j + 1), z0 + j + .5f);
                    if (!Sample(i - 1, j).Opening) ShaftWall(c00, c01, bottom, Vector3.right);
                    if (!Sample(i + 1, j).Opening) ShaftWall(c10, c11, bottom, Vector3.left);
                    if (!Sample(i, j - 1).Opening) ShaftWall(c00, c10, bottom, Vector3.forward);
                    if (!Sample(i, j + 1).Opening) ShaftWall(c01, c11, bottom, Vector3.back);
                    var pit = Vertex(new Color(.035f, .03f, .03f), 1);
                    int v = meshVertices.Count;
                    meshVertices.Add(new Vector3(c00.x, bottom, c00.z)); meshVertices.Add(new Vector3(c01.x, bottom, c01.z)); meshVertices.Add(new Vector3(c11.x, bottom, c11.z)); meshVertices.Add(new Vector3(c10.x, bottom, c10.z));
                    for (int k = 0; k < 4; k++) { meshNormals.Add(Vector3.up); meshColors.Add(pit); }
                    meshIndices.Add(v); meshIndices.Add(v + 1); meshIndices.Add(v + 2); meshIndices.Add(v); meshIndices.Add(v + 2); meshIndices.Add(v + 3);
                }

            var ground = chunk.Ground;
            ground.Clear();
            ground.SetVertices(meshVertices); ground.SetNormals(meshNormals); ground.SetColors(meshColors);
            ground.SetTriangles(meshIndices, 0, true);

            // Water surfaces share corners: a corner takes the mean surface and tint of the water tiles around it and
            // fades to clear where land meets it, so shores follow the banks instead of tile squares. Unexplored water
            // darkens into the fog like the land around it.
            meshVertices.Clear(); meshNormals.Clear(); meshColors.Clear(); meshIndices.Clear();
            var fogWater = TerrainSkirtColor(level).linear;
            Color WaterTint(GroundSample s)
            {
                var tint = Color.Lerp(new Color(.31f, .65f, .60f, .48f), new Color(.07f, .26f, .34f, .92f), Mathf.Clamp01(s.Depth / 3.5f));
                float fog = 1 - s.Explored;
                var fogged = new Color(tint.r * .08f + fogWater.r * .5f, tint.g * .08f + fogWater.g * .5f, tint.b * .08f + fogWater.b * .5f, .92f);
                return Color.Lerp(tint, fogged, fog);
            }
            bool WaterNear(int i, int j) { for (int dj = -1; dj <= 1; dj++) for (int di = -1; di <= 1; di++) if (Sample(i + di, j + dj).Water) return true; return false; }
            for (int j = 0; j < TerrainChunkSize; j++)
                for (int i = 0; i < TerrainChunkSize; i++)
                {
                    if (!WaterNear(i, j)) continue;
                    int v = meshVertices.Count;
                    for (int corner = 0; corner < 4; corner++)
                    {
                        // Corner order (-,-), (-,+), (+,+), (+,-) faces up with the indices below.
                        int ci = i + (corner >= 2 ? 1 : 0), cj = j + (corner == 1 || corner == 2 ? 1 : 0);
                        int count = 0; float surface = 0; var tint = new Color(0, 0, 0, 0);
                        for (int k = 0; k < 4; k++)
                        {
                            var w = Sample(ci - 1 + (k & 1), cj - 1 + (k >> 1));
                            if (!w.Water) continue;
                            count++; surface += w.WaterSurface; tint += WaterTint(w);
                        }
                        float y;
                        if (count > 0) { y = surface / count - .015f; tint /= count; tint.a *= count / 4f; }
                        else
                        {
                            // A dry corner of a shore tile: level with the nearest water, fully clear.
                            float nearest = float.MinValue;
                            for (int dj = -1; dj <= 1; dj++) for (int di = -1; di <= 1; di++) { var w = Sample(i + di, j + dj); if (w.Water) nearest = Mathf.Max(nearest, w.WaterSurface); }
                            y = nearest - .015f; tint = new Color(0, 0, 0, 0);
                        }
                        maxY = Mathf.Max(maxY, y);
                        meshVertices.Add(new Vector3(x0 + ci - .5f, y, z0 + cj - .5f)); meshNormals.Add(Vector3.up); meshColors.Add(tint);
                    }
                    meshIndices.Add(v); meshIndices.Add(v + 1); meshIndices.Add(v + 2); meshIndices.Add(v); meshIndices.Add(v + 2); meshIndices.Add(v + 3);
                }
            chunk.Water.Clear();
            if (meshVertices.Count > 0)
            {
                chunk.Water.SetVertices(meshVertices); chunk.Water.SetNormals(meshNormals); chunk.Water.SetColors(meshColors);
                chunk.Water.SetTriangles(meshIndices, 0, true);
            }
            chunk.WaterRenderer.enabled = meshVertices.Count > 0;
            chunk.Renderer.enabled = true;
            chunk.MinY = minY; chunk.MaxY = maxY;
            chunk.Signature = ContentSignature(key);
            chunk.Built = true; chunk.Dirty = false;
            chunk.Root.name = ChunkName(key);
        }

        private static Color Vertex(Color srgb, float explored)
        {
            var linear = new Color(Mathf.Clamp01(srgb.r), Mathf.Clamp01(srgb.g), Mathf.Clamp01(srgb.b)).linear;
            linear.a = Mathf.Clamp01(explored);
            return linear;
        }

        private static void ShaftWall(Vector3 top0, Vector3 top1, float bottom, Vector3 inward)
        {
            var b0 = new Vector3(top0.x, bottom, top0.z); var b1 = new Vector3(top1.x, bottom, top1.z);
            if (Vector3.Dot(Vector3.Cross(top1 - top0, b1 - top0), inward) < 0) { var swap = top0; top0 = top1; top1 = swap; var swapBottom = b0; b0 = b1; b1 = swapBottom; }
            int v = meshVertices.Count;
            meshVertices.Add(top0); meshVertices.Add(top1); meshVertices.Add(b1); meshVertices.Add(b0);
            var rim = Vertex(new Color(.30f, .23f, .16f), 1); var deep = Vertex(new Color(.05f, .04f, .035f), 1);
            meshColors.Add(rim); meshColors.Add(rim); meshColors.Add(deep); meshColors.Add(deep);
            for (int k = 0; k < 4; k++) meshNormals.Add(inward);
            meshIndices.Add(v); meshIndices.Add(v + 1); meshIndices.Add(v + 2); meshIndices.Add(v); meshIndices.Add(v + 2); meshIndices.Add(v + 3);
        }

        internal readonly struct TerrainKey : IEquatable<TerrainKey>
        {
            public readonly int X, Z, Level;
            public TerrainKey(int x, int z, int level) { X = x; Z = z; Level = level; }
            public static TerrainKey Of(int tileX, int tileZ, int level) => new TerrainKey(FloorDiv(tileX, TerrainChunkSize), FloorDiv(tileZ, TerrainChunkSize), level);
            public bool Equals(TerrainKey other) => X == other.X && Z == other.Z && Level == other.Level;
            public override bool Equals(object obj) => obj is TerrainKey other && Equals(other);
            public override int GetHashCode() => unchecked((X * 73856093) ^ (Z * 19349663) ^ (Level * 83492791));
        }

        private sealed class TerrainChunk
        {
            public TerrainKey Key;
            public GameObject Root;
            public MeshRenderer Renderer, WaterRenderer;
            public Mesh Ground, Water;
            public bool Built, Dirty = true;
            public int Signature;
            public float MinY, MaxY;
            // Rendered heights for picking and edge scenery.
            public readonly float[] Corners = new float[TerrainCorners * TerrainCorners];
            public readonly float[] Centers = new float[TerrainChunkSize * TerrainChunkSize];
            public readonly float[] WaterTop = new float[TerrainChunkSize * TerrainChunkSize];
            public readonly float[] Explored = new float[TerrainChunkSize * TerrainChunkSize];
        }
    }
}
