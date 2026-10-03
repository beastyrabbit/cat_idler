using System;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.Rendering;

namespace IdleCatForest.Presentation
{
    /// <summary>A 16x16 tile chunk on one level; static props are built, cached and invalidated per chunk.</summary>
    internal readonly struct ChunkKey : IEquatable<ChunkKey>
    {
        public const int Size = 16;
        public readonly int X, Z, Level;
        public ChunkKey(int x, int z, int level) { X = x; Z = z; Level = level; }
        public static ChunkKey Of(int tileX, int tileZ, int level) => new ChunkKey(FloorDiv(tileX), FloorDiv(tileZ), level);
        public static ChunkKey Of(Int2 p) => Of(p.X, p.Z, p.Level);
        public int MinX => X * Size;
        public int MinZ => Z * Size;
        public static int FloorDiv(int v) => v >= 0 ? v / Size : -((-v + Size - 1) / Size);
        public bool Equals(ChunkKey o) => X == o.X && Z == o.Z && Level == o.Level;
        public override bool Equals(object o) => o is ChunkKey k && Equals(k);
        public override int GetHashCode() => unchecked((X * 73856093) ^ (Z * 19349663) ^ (Level * 83492791));
        public override string ToString() => X + "," + Z + "," + Level;
    }

    /// <summary>How an instanced prop is lit and coloured: shadows, a ±5 % tint step and a fog step toward the world edge.</summary>
    internal readonly struct PropLook
    {
        public const int Tints = 3, FogSteps = 4;
        public readonly bool Shadows;
        public readonly byte Tint, Fog;
        public PropLook(bool shadows, int tint, int fog) { Shadows = shadows; Tint = (byte)Mathf.Clamp(tint, 0, Tints - 1); Fog = (byte)Mathf.Clamp(fog, 0, FogSteps - 1); }
        public static readonly PropLook Plain = new PropLook(true, 1, 0);
    }

    /// <summary>
    /// Instanced rendering of static scenery (trees, rocks, bushes, reeds, roads, fences, gates, scatter). Authored
    /// models are flattened once into (mesh, submesh, material, local matrix) parts; each chunk keeps its instance
    /// matrices per draw bucket and is rebuilt only when invalidated; the active chunks are concatenated into one
    /// list per bucket and drawn with <see cref="Graphics.RenderMeshInstanced{T}(in RenderParams, Mesh, int, List{T}, int, int)"/>.
    /// No GameObjects, colliders or per-frame allocations.
    /// </summary>
    internal sealed class StaticProps : IDisposable
    {
        private const int MaxPerDraw = 1023;

        /// <summary>One draw of an authored model: a submesh with its material, relative to the model's ground pivot.</summary>
        internal readonly struct Part
        {
            public readonly Mesh Mesh; public readonly int Submesh; public readonly Material Material; public readonly Matrix4x4 Local;
            public Part(Mesh mesh, int submesh, Material material, Matrix4x4 local) { Mesh = mesh; Submesh = submesh; Material = material; Local = local; }
        }

        /// <summary>An authored model ready for instancing, either at its natural metre scale or normalised to a 1 m footprint.</summary>
        internal sealed class Model
        {
            public string Asset;
            public Part[] Parts = Array.Empty<Part>();
            public bool Missing;
        }

        internal readonly struct BucketKey : IEquatable<BucketKey>
        {
            public readonly Mesh Mesh; public readonly int Submesh; public readonly Material Material; public readonly bool Shadows;
            public BucketKey(Mesh mesh, int submesh, Material material, bool shadows) { Mesh = mesh; Submesh = submesh; Material = material; Shadows = shadows; }
            public bool Equals(BucketKey o) => ReferenceEquals(Mesh, o.Mesh) && Submesh == o.Submesh && ReferenceEquals(Material, o.Material) && Shadows == o.Shadows;
            public override bool Equals(object o) => o is BucketKey k && Equals(k);
            public override int GetHashCode() => unchecked((RuntimeHelpers.GetHashCode(Mesh) * 397) ^ (Submesh * 31) ^ RuntimeHelpers.GetHashCode(Material) * 7919 ^ (Shadows ? 1 : 0));
        }

        private sealed class Bucket
        {
            public BucketKey Key;
            public readonly List<Matrix4x4> Matrices = new List<Matrix4x4>();
            // The union of the contributing chunks, so cameras and shadow cascades can cull a whole bucket.
            public Bounds Bounds;
        }

        /// <summary>The cached props of one chunk: matrices per draw bucket plus the per-tile identity used by tests and tools.</summary>
        internal sealed class Chunk
        {
            public ChunkKey Key;
            public int Signature;
            public bool Built, Dirty = true, HasKnown;
            public int Instances;
            public int LastUsedFrame;
            internal readonly Dictionary<BucketKey, List<Matrix4x4>> Buckets = new Dictionary<BucketKey, List<Matrix4x4>>();
            internal readonly List<(Int2 Tile, string Asset, Matrix4x4 Matrix)> Primary = new List<(Int2, string, Matrix4x4)>();
            internal readonly List<(string Asset, Matrix4x4 Matrix)> All = new List<(string, Matrix4x4)>();

            public void Clear()
            {
                foreach (var list in Buckets.Values) list.Clear();
                Primary.Clear(); All.Clear(); Instances = 0; HasKnown = false;
            }
        }

        private readonly Dictionary<string, Model> models = new Dictionary<string, Model>();
        private readonly Dictionary<(string, int, int), Material> variants = new Dictionary<(string, int, int), Material>();
        private readonly Dictionary<BucketKey, Bucket> buckets = new Dictionary<BucketKey, Bucket>();
        private readonly List<Bucket> drawList = new List<Bucket>();
        private readonly Dictionary<ChunkKey, Chunk> chunks = new Dictionary<ChunkKey, Chunk>();
        private readonly HashSet<string> reportedMissing = new HashSet<string>();
        private bool recordAll;
        /// <summary>Hands every created material to an owner that destroys it with the view.</summary>
        public Action<string, Material> OwnMaterial;

        public int ChunkCount => chunks.Count;
        public int DrawCalls { get; private set; }
        public int DrawnInstances { get; private set; }
        public int BucketCount => drawList.Count;
        /// <summary>When true, chunks also record every instance with its asset (collision proxies in tests).</summary>
        public bool RecordAll { get => recordAll; set { if (recordAll != value) { recordAll = value; foreach (var c in chunks.Values) c.Dirty = true; } } }
        public IEnumerable<Chunk> Chunks => chunks.Values;

        public bool TryGetChunk(ChunkKey key, out Chunk chunk) => chunks.TryGetValue(key, out chunk);

        public Chunk GetOrCreate(ChunkKey key)
        {
            if (!chunks.TryGetValue(key, out var chunk)) chunks[key] = chunk = new Chunk { Key = key };
            return chunk;
        }

        public void Remove(ChunkKey key) => chunks.Remove(key);

        /// <summary>Installs a chunk built off to the side and returns the one it replaces (for reuse), or null.</summary>
        public Chunk Replace(Chunk chunk)
        {
            chunks.TryGetValue(chunk.Key, out var old);
            chunks[chunk.Key] = chunk;
            return old;
        }

        /// <summary>Counts invalidations, so a chunk built across several frames knows it may already be stale.</summary>
        public int Invalidations { get; private set; }

        public void Clear() { chunks.Clear(); foreach (var b in buckets.Values) b.Matrices.Clear(); drawList.Clear(); Invalidations++; }

        public void MarkAllDirty() { foreach (var c in chunks.Values) c.Dirty = true; Invalidations++; }

        public void MarkDirty(ChunkKey key) { if (chunks.TryGetValue(key, out var c)) c.Dirty = true; Invalidations++; }

        // ------------------------------------------------------------------ models

        public Model ModelOf(string asset)
        {
            if (models.TryGetValue(asset, out var model)) return model;
            model = new Model { Asset = asset };
            var prefab = Resources.Load<GameObject>("ForestArt/" + asset);
            var parts = new List<Part>();
            if (prefab != null)
                foreach (var filter in prefab.GetComponentsInChildren<MeshFilter>(true))
                {
                    var renderer = filter.GetComponent<MeshRenderer>(); var mesh = filter.sharedMesh;
                    if (renderer == null || mesh == null) continue;
                    var mats = renderer.sharedMaterials;
                    var local = filter.transform.localToWorldMatrix;
                    for (int s = 0; s < mesh.subMeshCount; s++)
                    {
                        var m = mats.Length == 0 ? null : mats[Mathf.Min(s, mats.Length - 1)];
                        if (m != null) parts.Add(new Part(mesh, s, m, local));
                    }
                }
            model.Parts = parts.ToArray();
            model.Missing = parts.Count == 0;
            if (model.Missing && reportedMissing.Add(asset)) Debug.LogError("Static prop art missing: ForestArt/" + asset + " (run forest_setup or the Blender export).");
            models[asset] = model;
            return model;
        }

        /// <summary>
        /// The matrix that maps a model's parts into an entity root of footprint 1, matching how
        /// <c>ForestView.Entity</c> normalises authored models: the larger horizontal extent becomes 1 and the model
        /// stands on its root with its footprint centred.
        /// </summary>
        public Matrix4x4 Normalizer(Model model)
        {
            if (model.Parts.Length == 0) return Matrix4x4.identity;
            bool first = true; var b = new Bounds();
            foreach (var part in model.Parts)
            {
                if (part.Submesh != 0) continue;
                var mb = part.Mesh.bounds;
                for (int i = 0; i < 8; i++)
                {
                    var corner = part.Local.MultiplyPoint3x4(mb.center + Vector3.Scale(mb.extents, new Vector3((i & 1) == 0 ? -1 : 1, (i & 2) == 0 ? -1 : 1, (i & 4) == 0 ? -1 : 1)));
                    if (first) { b = new Bounds(corner, Vector3.zero); first = false; } else b.Encapsulate(corner);
                }
            }
            float scale = 1 / Mathf.Max(.01f, Mathf.Max(b.size.x, b.size.z));
            return Matrix4x4.Scale(Vector3.one * scale) * Matrix4x4.Translate(-new Vector3(b.center.x, b.min.y, b.center.z));
        }

        private readonly Dictionary<string, Matrix4x4> normalizers = new Dictionary<string, Matrix4x4>();

        // ------------------------------------------------------------------ building a chunk

        /// <summary>
        /// Adds one model instance to a chunk. <paramref name="root"/> is the entity-root transform (position, yaw,
        /// footprint scale); normalised models are fitted to a 1 m footprint first, natural ones keep metre scale.
        /// </summary>
        public void Add(Chunk chunk, string asset, Matrix4x4 root, bool naturalScale, PropLook look, Int2? primaryTile = null)
        {
            var model = ModelOf(asset);
            if (model.Missing) return;
            Matrix4x4 m = root;
            if (!naturalScale)
            {
                if (!normalizers.TryGetValue(asset, out var normalizer)) normalizers[asset] = normalizer = Normalizer(model);
                m = root * normalizer;
            }
            foreach (var part in model.Parts)
            {
                var key = new BucketKey(part.Mesh, part.Submesh, Variant(part.Material, look.Tint, look.Fog), look.Shadows);
                if (!chunk.Buckets.TryGetValue(key, out var list)) chunk.Buckets[key] = list = new List<Matrix4x4>();
                list.Add(m * part.Local);
            }
            chunk.Instances++;
            if (primaryTile.HasValue) chunk.Primary.Add((primaryTile.Value, asset, m));
            if (recordAll) chunk.All.Add((asset, m));
        }

        /// <summary>A shared copy of a palette material with a tint and fog step. Palette materials with the same name and colour share one copy across models.</summary>
        private Material Variant(Material source, int tint, int fog)
        {
            if (variantsBySource.TryGetValue((source, tint, fog), out var fast)) return fast;
            if (!paletteKeys.TryGetValue(source, out var palette))
                paletteKeys[source] = palette = source.shader.name + "#" + source.name + "#" + ColorUtility.ToHtmlStringRGBA(source.color) + "#" + (source.mainTexture != null ? RuntimeHelpers.GetHashCode(source.mainTexture) : 0);
            if (!variants.TryGetValue((palette, tint, fog), out var m))
            {
                m = new Material(source) { name = source.name + " t" + tint + " f" + fog, enableInstancing = true };
                m.color = Shade(source.color, tint, fog);
                variants[(palette, tint, fog)] = m;
                OwnMaterial?.Invoke("prop:" + palette + ":" + tint + ":" + fog, m);
            }
            variantsBySource[(source, tint, fog)] = m;
            return m;
        }

        private readonly Dictionary<(Material, int, int), Material> variantsBySource = new Dictionary<(Material, int, int), Material>();
        private readonly Dictionary<Material, string> paletteKeys = new Dictionary<Material, string>();

        // Trees stand above the fogged ground and catch light, so they fade a little less than the terrain's 35 % / 50 %.
        internal const float FogBrightness = .45f, FogDesaturation = .45f;

        /// <summary>±5 % value per tint step, then toward the fog look (<see cref="FogBrightness"/>, <see cref="FogDesaturation"/>) at full fog.</summary>
        internal static Color Shade(Color c, int tint, int fog)
        {
            float value = 1 + (tint - 1) * .05f;
            var shaded = new Color(c.r * value, c.g * value * (tint == 0 ? .99f : 1), c.b * value * (tint == 2 ? .98f : 1), c.a);
            float f = fog / (float)(PropLook.FogSteps - 1);
            if (f <= 0) return shaded;
            float grey = shaded.r * .299f + shaded.g * .587f + shaded.b * .114f;
            var desaturated = Color.Lerp(shaded, new Color(grey, grey, grey, c.a), FogDesaturation * f);
            float brightness = Mathf.Lerp(1, FogBrightness, f);
            return new Color(desaturated.r * brightness, desaturated.g * brightness, desaturated.b * brightness, c.a);
        }

        // ------------------------------------------------------------------ drawing

        /// <summary>Concatenates the given chunks into the per-bucket draw lists. Call when the active set or a chunk changed.</summary>
        public void Assemble(List<Chunk> active)
        {
            foreach (var b in buckets.Values) b.Matrices.Clear();
            drawList.Clear();
            foreach (var chunk in active)
            {
                if (chunk.Instances == 0) continue;
                var min = new Vector3(chunk.Key.MinX - 2, -12, chunk.Key.MinZ - 2);
                var max = new Vector3(chunk.Key.MinX + ChunkKey.Size + 2, 40, chunk.Key.MinZ + ChunkKey.Size + 2);
                foreach (var pair in chunk.Buckets)
                {
                    if (pair.Value.Count == 0) continue;
                    if (!buckets.TryGetValue(pair.Key, out var bucket)) buckets[pair.Key] = bucket = new Bucket { Key = pair.Key };
                    if (bucket.Matrices.Count == 0) { drawList.Add(bucket); bucket.Bounds.SetMinMax(min, max); }
                    else { bucket.Bounds.Encapsulate(min); bucket.Bounds.Encapsulate(max); }
                    bucket.Matrices.AddRange(pair.Value);
                }
            }
        }

        public void Draw(int layer)
        {
            int calls = 0, instances = 0;
            if (SystemInfo.supportsInstancing)
                foreach (var bucket in drawList)
                {
                    int count = bucket.Matrices.Count;
                    var rp = new RenderParams(bucket.Key.Material)
                    {
                        layer = layer,
                        worldBounds = bucket.Bounds,
                        shadowCastingMode = bucket.Key.Shadows ? ShadowCastingMode.On : ShadowCastingMode.Off,
                        receiveShadows = true,
                        lightProbeUsage = LightProbeUsage.Off,
                        reflectionProbeUsage = ReflectionProbeUsage.Off,
                    };
                    for (int start = 0; start < count; start += MaxPerDraw)
                    {
                        Graphics.RenderMeshInstanced(rp, bucket.Key.Mesh, bucket.Key.Submesh, bucket.Matrices, Mathf.Min(MaxPerDraw, count - start), start);
                        calls++;
                    }
                    instances += count;
                }
            DrawCalls = calls; DrawnInstances = instances;
        }

        public void Dispose()
        {
            foreach (var m in variants.Values) if (m != null) UnityEngine.Object.Destroy(m);
            variants.Clear(); variantsBySource.Clear(); paletteKeys.Clear(); Clear(); models.Clear(); normalizers.Clear();
        }
    }
}
