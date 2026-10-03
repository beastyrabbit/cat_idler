using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.UIElements;
using Debug = UnityEngine.Debug;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// The effect layer: one place every visual flourish goes through, so later work hangs juice on it instead
    /// of inventing its own. It owns a pooled burst service (<see cref="Burst(string, Vector3, Color, int)"/>,
    /// at most <see cref="FxPool.MaxConcurrent"/> at once, throttled at higher game speeds), rising world chips
    /// (<see cref="Float"/>), scale punches on an entity's <c>Visual</c> child (<see cref="Punch"/>), the
    /// <see cref="WorldEventFeed"/>, cat emotes, blob shadows and ambient life. Everything honours
    /// <see cref="GameSettings.ReducedMotion"/>. The view drives it from its Fx hooks
    /// (<c>ForestView.Fx.cs</c>); it has no Update of its own and never touches simulation state.
    /// </summary>
    public sealed class ForestFx : MonoBehaviour
    {
        /// <summary>Bursts per real second the bucket refills; one burst costs more tokens at 4x and above.</summary>
        public const float BurstsPerSecond = 12;
        private const float TelemetrySeconds = 10;

        private ForestView view;
        private Transform root;
        private FxAssets assets;
        private FxPool pool;
        private FloatLabels floats;
        private Emotes emotes;
        private BlobShadows shadows;
        private AmbientLife ambient;
        private Mesh quad;
        private float tokens = BurstsPerSecond;
        private string villageId = "";
        private int level = int.MinValue;

        private struct Tween { public Transform Target; public Vector3 Base; public float Age, Duration, Amount; }
        private readonly List<Tween> tweens = new List<Tween>();

        // Telemetry: CPU time of the effect layer per frame (feed, pool, emotes, shadows, ambient, chips).
        private readonly Stopwatch watch = new Stopwatch();
        private readonly float[] cost = new float[600];
        private int costCount, costNext;
        private float nextReport;
        private string evidenceDirectory = "";
        private float hourOverride = -1;
        private bool showcase;
        private float nextShowcase;

        /// <summary>Presentation-side diff of the world into typed events (see <see cref="WorldEventFeed"/>).</summary>
        public WorldEventFeed Feed { get; } = new WorldEventFeed();

        public int ActiveBursts => pool?.Active ?? 0;
        public int Throttled { get; private set; }
        public int EmotesDrawn => emotes?.Drawn ?? 0;
        public int ShadowsDrawn => shadows?.Drawn ?? 0;
        public int AmbientParticles => ambient?.Particles ?? 0;
        public int LiveFloats => floats?.Live ?? 0;

        internal void Bind(ForestView owner)
        {
            view = owner;
            root = new GameObject("Forest effects").transform;
            assets = new FxAssets();
            quad = MakeQuad();
            pool = new FxPool(root, assets);
            floats = new FloatLabels();
            emotes = new Emotes(this, root, assets, quad);
            shadows = new BlobShadows(assets, quad);
            ambient = new AmbientLife(root, assets);
            emotes.Subscribe(Feed);
            Feed.Reseeded += OnReseeded;
            var args = Environment.GetCommandLineArgs();
            evidenceDirectory = Argument(args, "--forest-capture-out") ?? Argument(args, "--forest-evidence") ?? "";
            if (float.TryParse(Argument(args, "--forest-fx-hour"), NumberStyles.Float, CultureInfo.InvariantCulture, out float hour)) hourOverride = Mathf.Repeat(hour, 24);
            showcase = Array.IndexOf(args, "--forest-fx-showcase") >= 0;
            nextReport = Time.unscaledTime + TelemetrySeconds;
            var particle = assets.Particle(FxSprites.Puff); var billboard = assets.Billboard(FxSprites.Blob);
            Debug.Log("[fx] ready: particle shader " + Describe(particle) + ", billboard shader " + Describe(billboard) + ", instancing " + SystemInfo.supportsInstancing);
        }

        private static string Describe(Material m) => m == null ? "missing" : m.shader.name + (m.shader.isSupported ? " (supported)" : " (NOT supported)");

        /// <summary>
        /// Plays a one-shot burst of <paramref name="kind"/> (an <see cref="FxKind"/> value) at a world point.
        /// <paramref name="tint"/> multiplies the kind's colours; <paramref name="count"/> 0 uses the kind's default.
        /// Returns false when dropped (pool full, throttled, or particles unavailable). Halved under reduced motion
        /// and thinned at 4x and above.
        /// </summary>
        public bool Burst(string kind, Vector3 position, Color tint, int count = 0)
        {
            if (pool == null) return false;
            float speed = SpeedOf();
            float price = speed >= 8 ? 3 : speed >= 4 ? 2 : 1;
            if (tokens < price) { Throttled++; return false; }
            tokens -= price;
            int n = count > 0 ? count : FxPreset.For(kind).Count;
            if (GameSettings.ReducedMotion) n = Mathf.Max(1, n / 2);
            if (speed >= 4) n = Mathf.Max(1, Mathf.RoundToInt(n * .6f));
            return pool.Burst(kind, position, tint, n, SizeScale());
        }

        public bool Burst(string kind, Vector3 position) => Burst(kind, position, Color.white, 0);

        /// <summary>
        /// A chip that rises from <paramref name="worldPosition"/> and fades over 1.5 s, with an optional icon.
        /// The same text near a live chip refreshes it instead of stacking (wider merge radius at 4x and above).
        /// </summary>
        public void Float(Vector3 worldPosition, string text, Background icon = default, Color color = default)
        {
            if (floats == null) return;
            floats.Float(worldPosition, text, icon, color, SpeedOf() >= 4 ? 3 : 1.2f);
        }

        /// <summary>
        /// Scale punch (1 → 1 + amount → 1 with a small undershoot) on the entity's <c>Visual</c> child, or on the
        /// transform itself when it has none. Restarting a running punch keeps its original scale. Skipped under
        /// reduced motion.
        /// </summary>
        public void Punch(Transform entityRoot, float amount = .12f, float duration = .35f)
        {
            if (entityRoot == null || GameSettings.ReducedMotion) return;
            var target = ForestView.VisualOf(entityRoot.gameObject) ?? entityRoot;
            for (int i = 0; i < tweens.Count; i++)
                if (tweens[i].Target == target) { var t = tweens[i]; t.Age = 0; t.Amount = amount; t.Duration = Mathf.Max(.05f, duration); tweens[i] = t; return; }
            tweens.Add(new Tween { Target = target, Base = target.localScale, Duration = Mathf.Max(.05f, duration), Amount = amount });
        }

        /// <summary>Effect-layer CPU cost over the last frames, in milliseconds.</summary>
        public (float p50, float p95, float max, int frames) Cost()
        {
            if (costCount == 0) return (0, 0, 0, 0);
            var sorted = new float[costCount];
            Array.Copy(cost, sorted, costCount);
            Array.Sort(sorted);
            return (sorted[costCount / 2], sorted[Mathf.Min(costCount - 1, (int)(costCount * .95f))], sorted[costCount - 1], costCount);
        }

        internal void Tick(float dt)
        {
            if (view == null) return;
            watch.Restart();
            var game = view.FxGame;
            var world = game.CurrentWorld; var village = game.Selected;
            if (world == null || village == null) { watch.Stop(); return; }
            if (village.Id != villageId) { villageId = village.Id; ClearTransient(); }
            if (view.RenderedLevel != level) { level = view.RenderedLevel; ambient.Clear(); emotes.Clear(); }
            tokens = Mathf.Min(BurstsPerSecond, tokens + dt * BurstsPerSecond);
            Feed.Observe(world, village);
            pool.Tick();
            TickTweens(dt);
            ambient.Tick(AmbientFrame(world), dt);
            if (showcase) Showcase(world, village);
            watch.Stop();
        }

        internal void LateTick(float dt)
        {
            if (view == null) return;
            watch.Start();
            var game = view.FxGame;
            var world = game.CurrentWorld; var village = game.Selected;
            var camera = view.Camera;
            if (world != null && village != null)
            {
                bool reduced = GameSettings.ReducedMotion;
                shadows.Render(view, world, village, camera);
                emotes.Render(view, world, village, camera, dt, MarkScale(), reduced, showcase);
                floats.Tick(dt, camera, reduced);
            }
            watch.Stop();
            Record((float)watch.Elapsed.TotalMilliseconds);
        }

        private void ClearTransient()
        {
            pool.Clear(); floats.Clear(); emotes.Clear(); ambient.Clear();
            for (int i = 0; i < tweens.Count; i++) if (tweens[i].Target != null) tweens[i].Target.localScale = tweens[i].Base;
            tweens.Clear();
        }

        private void OnReseeded(string reason) { if (reason != "load") floats.Clear(); }

        private void TickTweens(float dt)
        {
            for (int i = tweens.Count - 1; i >= 0; i--)
            {
                var t = tweens[i];
                if (t.Target == null) { tweens.RemoveAt(i); continue; }
                t.Age += dt;
                float x = Mathf.Clamp01(t.Age / t.Duration);
                if (x >= 1 || GameSettings.ReducedMotion) { t.Target.localScale = t.Base; tweens.RemoveAt(i); continue; }
                // Damped sine normalised to peak at 1 near x = .2, with a small undershoot before settling.
                float wave = Mathf.Sin(x * Mathf.PI * 2.2f) * Mathf.Pow(1 - x, 1.6f) / .69f;
                t.Target.localScale = t.Base * (1 + t.Amount * wave);
                tweens[i] = t;
            }
        }

        private AmbientLife.Frame AmbientFrame(World world)
        {
            var camera = view.Camera;
            var forward = camera != null ? camera.transform.forward : Vector3.forward;
            var flatForward = new Vector3(forward.x, 0, forward.z);
            flatForward = flatForward.sqrMagnitude < 1e-4f ? Vector3.forward : flatForward.normalized;
            var right = Vector3.Cross(Vector3.up, flatForward);
            float halfWidth, halfDepth;
            if (view.DirectControl || camera == null || !camera.orthographic) { halfWidth = 10; halfDepth = 12; }
            else
            {
                float zoom = camera.orthographicSize;
                halfWidth = zoom * camera.aspect;
                halfDepth = zoom / Mathf.Max(.3f, Mathf.Sin(camera.transform.eulerAngles.x * Mathf.Deg2Rad));
            }
            return new AmbientLife.Frame
            {
                World = world, Center = view.FxFocus, Right = right, Forward = flatForward,
                HalfWidth = halfWidth, HalfDepth = halfDepth, SizeScale = SizeScale(), Hour = Hour(world),
                Level = view.RenderedLevel, ReducedMotion = GameSettings.ReducedMotion
            };
        }

        private float Hour(World world) => hourOverride >= 0 ? hourOverride : (float)(DayClock.DayFraction(world) * 24);

        private float SpeedOf() { var game = view != null ? view.FxGame : null; return game != null ? game.Speed : 1; }

        // Particles grow a little when zoomed out so they stay legible; direct control uses natural size.
        private float SizeScale()
        {
            var camera = view != null ? view.Camera : null;
            if (camera == null || !camera.orthographic || view.DirectControl) return 1;
            return Mathf.Clamp(camera.orthographicSize / 14f, 1, 2.2f);
        }

        private float MarkScale()
        {
            var camera = view != null ? view.Camera : null;
            if (camera == null || !camera.orthographic || view.DirectControl) return 1;
            return Mathf.Clamp(camera.orthographicSize / 12f, .9f, 2.4f);
        }

        private void Record(float milliseconds)
        {
            cost[costNext] = milliseconds;
            costNext = (costNext + 1) % cost.Length;
            costCount = Mathf.Min(costCount + 1, cost.Length);
            if (Time.unscaledTime < nextReport) return;
            nextReport = Time.unscaledTime + TelemetrySeconds;
            Report();
        }

        private void Report()
        {
            var (p50, p95, max, frames) = Cost();
            var sample = new FxPerformance
            {
                frames = frames, costP50 = p50, costP95 = p95, costMax = max,
                activeBursts = ActiveBursts, throttled = Throttled, emotes = EmotesDrawn, shadows = ShadowsDrawn,
                ambientParticles = AmbientParticles, floats = LiveFloats,
                feed = string.Join(" ", FeedCounts())
            };
            Debug.Log(string.Format(CultureInfo.InvariantCulture, "[fx] cost p50={0:F3} ms p95={1:F3} ms max={2:F3} ms over {3} frames; bursts={4} throttled={5} emotes={6} shadows={7} ambient={8} floats={9}; feed {10}",
                p50, p95, max, frames, sample.activeBursts, sample.throttled, sample.emotes, sample.shadows, sample.ambientParticles, sample.floats, sample.feed));
            if (evidenceDirectory == "") return;
            try { Directory.CreateDirectory(evidenceDirectory); File.WriteAllText(Path.Combine(evidenceDirectory, "fx-performance.json"), JsonUtility.ToJson(sample, true)); }
            catch (Exception e) { Debug.LogWarning("[fx] could not write fx-performance.json: " + e.Message); evidenceDirectory = ""; }
        }

        private IEnumerable<string> FeedCounts()
        {
            foreach (var pair in Feed.Counts) yield return pair.Key + "=" + pair.Value.ToString(CultureInfo.InvariantCulture);
        }

        [Serializable]
        private class FxPerformance
        {
            public int frames, activeBursts, throttled, emotes, shadows, ambientParticles, floats;
            public float costP50, costP95, costMax;
            public string feed;
        }

        // Developer evidence only (--forest-fx-showcase): every burst kind and a chip near the village centre.
        private void Showcase(World world, Village village)
        {
            if (Time.unscaledTime < nextShowcase) return;
            nextShowcase = Time.unscaledTime + 1.2f;
            var centre = WorldEventFeed.At(world, village.Center);
            for (int i = 0; i < FxKind.All.Length; i++)
            {
                float angle = i * Mathf.PI * 2 / FxKind.All.Length;
                var p = centre + new Vector3(Mathf.Cos(angle), 0, Mathf.Sin(angle)) * 3.2f + Vector3.up * .3f;
                tokens = BurstsPerSecond;
                Burst(FxKind.All[i], p);
            }
            Float(centre + Vector3.up * 1.2f, "+3 logs");
        }

        private static string Argument(string[] args, string key)
        {
            int at = Array.IndexOf(args, key);
            return at >= 0 && at + 1 < args.Length ? args[at + 1] : null;
        }

        private static Mesh MakeQuad()
        {
            var mesh = new Mesh { name = "Effect quad" };
            mesh.vertices = new[] { new Vector3(-.5f, -.5f, 0), new Vector3(.5f, -.5f, 0), new Vector3(.5f, .5f, 0), new Vector3(-.5f, .5f, 0) };
            mesh.uv = new[] { new Vector2(0, 0), new Vector2(1, 0), new Vector2(1, 1), new Vector2(0, 1) };
            mesh.triangles = new[] { 0, 2, 1, 0, 3, 2 };
            mesh.RecalculateBounds();
            return mesh;
        }

        // Entity ids are "<prefix><simulation id>"; cache them so per-frame lookups do not allocate.
        private static readonly Dictionary<string, Dictionary<string, string>> keys = new Dictionary<string, Dictionary<string, string>>();
        internal static string CatKey(string id) => Key("cat:", id);
        internal static string Key(string prefix, string id)
        {
            if (!keys.TryGetValue(prefix, out var byId)) { byId = new Dictionary<string, string>(); keys[prefix] = byId; }
            if (!byId.TryGetValue(id, out var key)) { if (byId.Count > 4096) byId.Clear(); key = prefix + id; byId[id] = key; }
            return key;
        }

        private void OnDestroy()
        {
            Feed.Reseeded -= OnReseeded;
            emotes?.Unsubscribe(Feed);
            for (int i = 0; i < tweens.Count; i++) if (tweens[i].Target != null) tweens[i].Target.localScale = tweens[i].Base;
            tweens.Clear();
            pool?.Dispose(); floats?.Dispose(); emotes?.Dispose(); shadows?.Dispose(); ambient?.Dispose(); assets?.Dispose();
            if (quad != null) Destroy(quad);
            if (root != null) Destroy(root.gameObject);
        }
    }

    /// <summary>One instanced draw of tinted sprite quads, rebuilt every frame (emotes, blob shadows).</summary>
    internal sealed class FxInstances : IDisposable
    {
        private readonly Material material;
        private readonly Mesh mesh;
        private readonly Matrix4x4[] matrices;
        private readonly Vector4[] colors;
        private readonly MaterialPropertyBlock block = new MaterialPropertyBlock();
        private static readonly int ColorId = Shader.PropertyToID("_Color");

        public int Count { get; private set; }

        public FxInstances(Material material, Mesh mesh, int capacity)
        {
            this.material = material; this.mesh = mesh;
            matrices = new Matrix4x4[Mathf.Min(capacity, 1023)];
            colors = new Vector4[matrices.Length];
        }

        public void Clear() => Count = 0;

        public void Add(Matrix4x4 matrix, Color color)
        {
            if (Count >= matrices.Length) return;
            matrices[Count] = matrix; colors[Count] = color; Count++;
        }

        public void Draw()
        {
            if (Count == 0 || material == null || mesh == null) return;
            // DrawMeshInstanced takes per-instance property arrays from the block (the instanced _Color).
            block.SetVectorArray(ColorId, colors);
#pragma warning disable 0618
            Graphics.DrawMeshInstanced(mesh, 0, material, matrices, Count, block, UnityEngine.Rendering.ShadowCastingMode.Off, false);
#pragma warning restore 0618
        }

        public void Dispose() { Count = 0; }
    }

    /// <summary>Shared setup for long-lived, manually emitted particle systems (ambient life, trails).</summary>
    internal static class FxAmbientSystem
    {
        public static ParticleSystem Create(Transform root, string name, Material material, int maxParticles)
        {
            var go = new GameObject(name);
            go.transform.SetParent(root, false);
            var system = go.AddComponent<ParticleSystem>();
            system.Stop(true, ParticleSystemStopBehavior.StopEmittingAndClear);
            var main = system.main;
            main.playOnAwake = false; main.loop = true; main.duration = 5; main.maxParticles = maxParticles;
            main.simulationSpace = ParticleSystemSimulationSpace.World; main.scalingMode = ParticleSystemScalingMode.Hierarchy;
            main.startSpeed = 0;
            var emission = system.emission; emission.enabled = false;
            var shape = system.shape; shape.enabled = false;
            var renderer = go.GetComponent<ParticleSystemRenderer>();
            renderer.renderMode = ParticleSystemRenderMode.Billboard;
            renderer.alignment = ParticleSystemRenderSpace.View;
            renderer.shadowCastingMode = UnityEngine.Rendering.ShadowCastingMode.Off;
            renderer.receiveShadows = false;
            renderer.sortMode = ParticleSystemSortMode.None;
            renderer.maxParticleSize = 1;
            renderer.sharedMaterial = material;
            system.Play(false);
            return system;
        }

        public static Gradient FadeInOut(float fadeIn, float fadeOutStart)
        {
            var g = new Gradient();
            g.SetKeys(new[] { new GradientColorKey(Color.white, 0), new GradientColorKey(Color.white, 1) },
                new[] { new GradientAlphaKey(0, 0), new GradientAlphaKey(1, fadeIn), new GradientAlphaKey(1, fadeOutStart), new GradientAlphaKey(0, 1) });
            return g;
        }
    }
}
