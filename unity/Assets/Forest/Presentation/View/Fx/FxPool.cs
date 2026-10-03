using System;
using System.Collections.Generic;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>Burst kinds understood by <see cref="ForestFx.Burst(string, Vector3, Color, int)"/>.</summary>
    public static class FxKind
    {
        /// <summary>A soft brown puff, for footfalls, drops and small impacts.</summary>
        public const string Dust = "dust";
        /// <summary>A flat ring of dust rolling outwards, for placements and completions.</summary>
        public const string DustRing = "dust_ring";
        /// <summary>Green leaves tossed up and fluttering down, for felled trees and bushes.</summary>
        public const string Leaves = "leaves";
        /// <summary>Wood chips flicked up with gravity, for chopping and sawing.</summary>
        public const string Chips = "chips";
        /// <summary>Grey stone chips, for quarrying and crumbling rock.</summary>
        public const string Stone = "stone";
        /// <summary>Gold glints that pop and fade, for rewards and discoveries.</summary>
        public const string Sparkle = "sparkle";
        /// <summary>Pink hearts rising, for births, pregnancies and affection.</summary>
        public const string Hearts = "hearts";
        /// <summary>Hot orange sparks with gravity, for smithing and hits.</summary>
        public const string Sparks = "sparks";
        /// <summary>A white cloud, for spawns and despawns.</summary>
        public const string Poof = "poof";
        /// <summary>Paw prints bursting outwards, for a click on a cat (the first-minute hook).</summary>
        public const string Paw = "paw";

        public static readonly string[] All = { Dust, DustRing, Leaves, Chips, Stone, Sparkle, Hearts, Sparks, Poof, Paw };
    }

    /// <summary>How one burst kind looks. Colours are multiplied by the caller's tint.</summary>
    internal struct FxPreset
    {
        public string Sprite; public bool Additive;
        public int Count; public float LifeMin, LifeMax, SpeedMin, SpeedMax, SizeMin, SizeMax, Gravity, Radius, Drag, Noise, Spin;
        public ParticleSystemShapeType Shape; public bool Upward, Grow; public Color ColorA, ColorB;

        public static FxPreset For(string kind)
        {
            switch (kind)
            {
                case FxKind.Dust: return new FxPreset { Sprite = FxSprites.Puff, Count = 10, LifeMin = .55f, LifeMax = .95f, SpeedMin = .35f, SpeedMax = 1.1f, SizeMin = .22f, SizeMax = .42f, Gravity = -.05f, Radius = .25f, Drag = .5f, Shape = ParticleSystemShapeType.Hemisphere, Upward = true, Grow = true, ColorA = new Color(.78f, .68f, .52f, .85f), ColorB = new Color(.66f, .58f, .46f, .75f), Spin = 60 };
                case FxKind.DustRing: return new FxPreset { Sprite = FxSprites.Puff, Count = 18, LifeMin = .5f, LifeMax = .8f, SpeedMin = 1.6f, SpeedMax = 2.4f, SizeMin = .28f, SizeMax = .46f, Gravity = -.02f, Radius = .35f, Drag = .75f, Shape = ParticleSystemShapeType.Circle, Grow = true, ColorA = new Color(.82f, .74f, .58f, .85f), ColorB = new Color(.7f, .62f, .5f, .7f), Spin = 40 };
                case FxKind.Leaves: return new FxPreset { Sprite = FxSprites.Leaf, Count = 12, LifeMin = 1.1f, LifeMax = 1.8f, SpeedMin = 1, SpeedMax = 2.1f, SizeMin = .12f, SizeMax = .2f, Gravity = .35f, Radius = .4f, Drag = .4f, Noise = .6f, Shape = ParticleSystemShapeType.Hemisphere, Upward = true, ColorA = new Color(.45f, .66f, .28f), ColorB = new Color(.78f, .74f, .3f), Spin = 220 };
                case FxKind.Chips: return new FxPreset { Sprite = FxSprites.Chip, Count = 8, LifeMin = .45f, LifeMax = .75f, SpeedMin = 1.6f, SpeedMax = 2.6f, SizeMin = .07f, SizeMax = .12f, Gravity = 1.6f, Radius = .12f, Shape = ParticleSystemShapeType.Cone, Upward = true, ColorA = new Color(.86f, .66f, .4f), ColorB = new Color(.66f, .45f, .25f), Spin = 540 };
                case FxKind.Stone: return new FxPreset { Sprite = FxSprites.Chip, Count = 8, LifeMin = .45f, LifeMax = .75f, SpeedMin = 1.4f, SpeedMax = 2.4f, SizeMin = .07f, SizeMax = .13f, Gravity = 1.8f, Radius = .15f, Shape = ParticleSystemShapeType.Cone, Upward = true, ColorA = new Color(.66f, .67f, .64f), ColorB = new Color(.48f, .5f, .5f), Spin = 400 };
                case FxKind.Sparkle: return new FxPreset { Sprite = FxSprites.Star, Additive = true, Count = 10, LifeMin = .45f, LifeMax = .9f, SpeedMin = .2f, SpeedMax = .7f, SizeMin = .16f, SizeMax = .3f, Gravity = -.1f, Radius = .4f, Shape = ParticleSystemShapeType.Sphere, ColorA = new Color(1, .86f, .45f), ColorB = new Color(1, .96f, .78f), Spin = 90 };
                case FxKind.Hearts: return new FxPreset { Sprite = FxSprites.Heart, Count = 5, LifeMin = 1, LifeMax = 1.4f, SpeedMin = .5f, SpeedMax = .9f, SizeMin = .16f, SizeMax = .24f, Gravity = -.08f, Radius = .25f, Noise = .35f, Shape = ParticleSystemShapeType.Hemisphere, Upward = true, ColorA = new Color(1, .56f, .69f), ColorB = new Color(1, .72f, .8f) };
                case FxKind.Sparks: return new FxPreset { Sprite = FxSprites.Dot, Additive = true, Count = 12, LifeMin = .25f, LifeMax = .55f, SpeedMin = 2, SpeedMax = 3.6f, SizeMin = .04f, SizeMax = .08f, Gravity = 2.2f, Radius = .08f, Shape = ParticleSystemShapeType.Hemisphere, Upward = true, ColorA = new Color(1, .62f, .2f), ColorB = new Color(1, .86f, .4f) };
                case FxKind.Poof: return new FxPreset { Sprite = FxSprites.Puff, Count = 9, LifeMin = .45f, LifeMax = .75f, SpeedMin = .5f, SpeedMax = 1.1f, SizeMin = .3f, SizeMax = .55f, Gravity = -.08f, Radius = .25f, Drag = .6f, Shape = ParticleSystemShapeType.Sphere, Grow = true, ColorA = new Color(.96f, .97f, .94f, .9f), ColorB = new Color(.84f, .87f, .84f, .8f), Spin = 50 };
                case FxKind.Paw: return new FxPreset { Sprite = FxSprites.Paw, Count = 6, LifeMin = .6f, LifeMax = .85f, SpeedMin = 1.1f, SpeedMax = 1.7f, SizeMin = .16f, SizeMax = .24f, Gravity = -.2f, Radius = .1f, Drag = .7f, Shape = ParticleSystemShapeType.Circle, ColorA = new Color(1, .82f, .4f), ColorB = new Color(1, .9f, .62f), Spin = 30 };
                default: return For(FxKind.Poof);
            }
        }
    }

    /// <summary>
    /// A fixed pool of code-built one-shot particle systems. Each burst borrows one system, configures it
    /// for its kind, emits and returns it once its particles have died. At most <see cref="MaxConcurrent"/>
    /// bursts play at once; further requests are dropped.
    /// </summary>
    internal sealed class FxPool : IDisposable
    {
        public const int MaxConcurrent = 32;
        private readonly Transform root;
        private readonly FxAssets assets;
        private readonly Stack<ParticleSystem> free = new Stack<ParticleSystem>();
        private readonly List<ParticleSystem> active = new List<ParticleSystem>();
        private readonly List<float> startedAt = new List<float>();
        private readonly Dictionary<string, FxPreset> presets = new Dictionary<string, FxPreset>();
        private int created;

        public FxPool(Transform root, FxAssets assets) { this.root = root; this.assets = assets; }

        public int Active => active.Count;
        public int Dropped { get; private set; }
        public int Played { get; private set; }

        public int ParticleCount
        {
            get { int n = 0; for (int i = 0; i < active.Count; i++) n += active[i].particleCount; return n; }
        }

        public bool Burst(string kind, Vector3 position, Color tint, int count, float sizeScale)
        {
            if (!presets.TryGetValue(kind ?? "", out var preset)) { preset = FxPreset.For(kind); presets[kind ?? ""] = preset; }
            var material = preset.Additive ? assets.Glow(preset.Sprite) : assets.Particle(preset.Sprite);
            if (material == null) return false;
            var system = Take();
            if (system == null) { Dropped++; return false; }
            Configure(system, preset, tint, sizeScale, material);
            system.transform.position = position;
            system.Play(false);
            system.Emit(Mathf.Clamp(count > 0 ? count : preset.Count, 1, 64));
            active.Add(system);
            startedAt.Add(Time.unscaledTime);
            Played++;
            return true;
        }

        /// <summary>Returns systems whose particles have all died to the pool.</summary>
        public void Tick()
        {
            float now = Time.unscaledTime;
            for (int i = active.Count - 1; i >= 0; i--)
            {
                var system = active[i];
                if (system == null) { active.RemoveAt(i); startedAt.RemoveAt(i); continue; }
                if (now - startedAt[i] < .05f || system.particleCount > 0 && now - startedAt[i] < 6) continue;
                system.Stop(false, ParticleSystemStopBehavior.StopEmittingAndClear);
                system.gameObject.SetActive(false);
                active.RemoveAt(i); startedAt.RemoveAt(i);
                free.Push(system);
            }
        }

        public void Clear()
        {
            for (int i = active.Count - 1; i >= 0; i--)
            {
                var system = active[i];
                if (system == null) continue;
                system.Stop(false, ParticleSystemStopBehavior.StopEmittingAndClear);
                system.gameObject.SetActive(false);
                free.Push(system);
            }
            active.Clear(); startedAt.Clear();
        }

        private ParticleSystem Take()
        {
            while (free.Count > 0) { var pooled = free.Pop(); if (pooled != null) { pooled.gameObject.SetActive(true); return pooled; } }
            if (created >= MaxConcurrent) return null;
            created++;
            var go = new GameObject("Effect burst " + created);
            go.transform.SetParent(root, false);
            var system = go.AddComponent<ParticleSystem>();
            system.Stop(true, ParticleSystemStopBehavior.StopEmittingAndClear);
            var main = system.main;
            main.playOnAwake = false; main.loop = false; main.duration = 1; main.maxParticles = 64;
            main.simulationSpace = ParticleSystemSimulationSpace.World; main.scalingMode = ParticleSystemScalingMode.Hierarchy;
            var emission = system.emission; emission.enabled = false;
            var renderer = go.GetComponent<ParticleSystemRenderer>();
            renderer.renderMode = ParticleSystemRenderMode.Billboard;
            renderer.alignment = ParticleSystemRenderSpace.View;
            renderer.shadowCastingMode = UnityEngine.Rendering.ShadowCastingMode.Off;
            renderer.receiveShadows = false;
            renderer.sortMode = ParticleSystemSortMode.None;
            renderer.maxParticleSize = 1;
            return system;
        }

        private static readonly Gradient Fade = MakeFade();

        private static Gradient MakeFade()
        {
            var g = new Gradient();
            g.SetKeys(new[] { new GradientColorKey(Color.white, 0), new GradientColorKey(Color.white, 1) },
                new[] { new GradientAlphaKey(0, 0), new GradientAlphaKey(1, .08f), new GradientAlphaKey(1, .6f), new GradientAlphaKey(0, 1) });
            return g;
        }

        private static void Configure(ParticleSystem system, FxPreset p, Color tint, float sizeScale, Material material)
        {
            var main = system.main;
            main.startLifetime = new ParticleSystem.MinMaxCurve(p.LifeMin, p.LifeMax);
            main.startSpeed = new ParticleSystem.MinMaxCurve(p.SpeedMin * sizeScale, p.SpeedMax * sizeScale);
            // Presets are authored in tiles; sprites carry soft margins, so draw them half again as large.
            main.startSize = new ParticleSystem.MinMaxCurve(p.SizeMin * sizeScale * 1.5f, p.SizeMax * sizeScale * 1.5f);
            main.startColor = new ParticleSystem.MinMaxGradient(p.ColorA * tint, p.ColorB * tint);
            main.gravityModifier = p.Gravity;
            main.startRotation = p.Spin > 0 ? new ParticleSystem.MinMaxCurve(0, Mathf.PI * 2) : new ParticleSystem.MinMaxCurve(0);

            var shape = system.shape;
            shape.enabled = true;
            shape.shapeType = p.Shape;
            shape.radius = p.Radius * sizeScale;
            shape.radiusThickness = p.Shape == ParticleSystemShapeType.Circle ? 0 : 1;
            shape.arc = 360;
            shape.angle = 25;
            // Circle lies in the XY plane and cones and hemispheres point along +Z: turn them to the ground and the sky.
            shape.rotation = p.Shape == ParticleSystemShapeType.Circle ? new Vector3(90, 0, 0) : p.Upward ? new Vector3(-90, 0, 0) : Vector3.zero;

            var color = system.colorOverLifetime; color.enabled = true; color.color = Fade;
            var size = system.sizeOverLifetime; size.enabled = true;
            size.size = new ParticleSystem.MinMaxCurve(1, p.Grow ? AnimationCurve.Linear(0, .7f, 1, 1.35f) : AnimationCurve.EaseInOut(0, 1, 1, .45f));
            var rotation = system.rotationOverLifetime; rotation.enabled = p.Spin > 0;
            if (p.Spin > 0) rotation.z = new ParticleSystem.MinMaxCurve(-p.Spin * Mathf.Deg2Rad, p.Spin * Mathf.Deg2Rad);
            var limit = system.limitVelocityOverLifetime; limit.enabled = p.Drag > 0;
            if (p.Drag > 0) { limit.limit = 100; limit.dampen = 0; limit.drag = p.Drag * 2.5f; limit.multiplyDragByParticleSize = false; limit.multiplyDragByParticleVelocity = false; }
            var noise = system.noise; noise.enabled = p.Noise > 0;
            if (p.Noise > 0) { noise.strength = p.Noise; noise.frequency = .8f; noise.scrollSpeed = .4f; noise.quality = ParticleSystemNoiseQuality.Low; }

            var renderer = system.GetComponent<ParticleSystemRenderer>();
            if (renderer.sharedMaterial != material) renderer.sharedMaterial = material;
        }

        public void Dispose()
        {
            foreach (var system in free) if (system != null) UnityEngine.Object.Destroy(system.gameObject);
            foreach (var system in active) if (system != null) UnityEngine.Object.Destroy(system.gameObject);
            free.Clear(); active.Clear(); startedAt.Clear();
        }
    }
}
