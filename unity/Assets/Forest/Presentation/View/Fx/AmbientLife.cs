using System;
using System.Collections.Generic;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// One camera-following set of ambient particles (about 200 at most) that keeps the forest alive while
    /// nothing else moves: pollen motes drifting in the light, leaves falling from trees, butterflies over
    /// meadows and berry bushes, fireflies after dusk (from <see cref="DayClock"/>) and glints on water.
    /// Underground only slow dust motes remain. Spawn spots come from persisted tiles near the view, scanned
    /// a few hundred tiles per frame so the cost stays flat; nothing here reads or changes simulation rules.
    /// </summary>
    internal sealed class AmbientLife : IDisposable
    {
        private const int ScanPerFrame = 360;
        private const int MaxSpots = 160;
        private readonly ParticleSystem pollen, leaves, butterflies, fireflies, glints;
        private float pollenDue, leavesDue, butterfliesDue, firefliesDue, glintsDue;

        // Candidate spots: the list being scanned and the last complete one.
        private List<Vector3> trees = new List<Vector3>(), meadows = new List<Vector3>(), waters = new List<Vector3>();
        private List<Vector3> nextTrees = new List<Vector3>(), nextMeadows = new List<Vector3>(), nextWaters = new List<Vector3>();
        private int scanX, scanZ, scanMinX, scanMaxX, scanMinZ, scanMaxZ, scanLevel = int.MinValue;
        private bool scanning;
        private readonly System.Random random = new System.Random(4242);
        private readonly ParticleSystem[] systems;
        private static readonly Color[] Wings = { new Color(1, .97f, .9f), new Color(1, .86f, .42f), new Color(.62f, .8f, 1), new Color(1, .64f, .4f) };

        public int Particles
        {
            get
            {
                int n = 0;
                foreach (var system in systems) if (system != null) n += system.particleCount;
                return n;
            }
        }

        public AmbientLife(Transform root, FxAssets assets)
        {
            var dot = assets.Particle(FxSprites.Dot);
            var leaf = assets.Particle(FxSprites.Leaf);
            var butterfly = assets.Particle(FxSprites.Butterfly);
            var glow = assets.Glow(FxSprites.Glow);
            var star = assets.Glow(FxSprites.Star);
            if (dot != null)
            {
                pollen = FxAmbientSystem.Create(root, "Ambient pollen", dot, 70);
                Fade(pollen, .25f, .7f);
                Noise(pollen, .18f, .35f);
            }
            if (leaf != null)
            {
                leaves = FxAmbientSystem.Create(root, "Ambient falling leaves", leaf, 40);
                Fade(leaves, .12f, .8f);
                Noise(leaves, .55f, .6f);
                var spin = leaves.rotationOverLifetime; spin.enabled = true; spin.z = new ParticleSystem.MinMaxCurve(-2.4f, 2.4f);
                var main = leaves.main; main.gravityModifier = .045f;
            }
            if (butterfly != null)
            {
                butterflies = FxAmbientSystem.Create(root, "Ambient butterflies", butterfly, 22);
                Fade(butterflies, .12f, .85f);
                Noise(butterflies, 1.1f, 1.4f);
                var size = butterflies.sizeOverLifetime; size.enabled = true; size.separateAxes = true;
                size.x = new ParticleSystem.MinMaxCurve(1, Flap(34)); size.y = new ParticleSystem.MinMaxCurve(1, AnimationCurve.Constant(0, 1, 1)); size.z = new ParticleSystem.MinMaxCurve(1, AnimationCurve.Constant(0, 1, 1));
                var main = butterflies.main; main.startSize3D = true;
            }
            if (glow != null)
            {
                fireflies = FxAmbientSystem.Create(root, "Ambient fireflies", glow, 50);
                var color = fireflies.colorOverLifetime; color.enabled = true; color.color = Blink();
                Noise(fireflies, .45f, .5f);
            }
            if (star != null)
            {
                glints = FxAmbientSystem.Create(root, "Ambient water glints", star, 30);
                Fade(glints, .3f, .55f);
                var size = glints.sizeOverLifetime; size.enabled = true; size.size = new ParticleSystem.MinMaxCurve(1, new AnimationCurve(new Keyframe(0, .2f), new Keyframe(.4f, 1), new Keyframe(1, .1f)));
            }
            systems = new[] { pollen, leaves, butterflies, fireflies, glints };
        }

        public struct Frame
        {
            public World World; public Vector3 Center, Right, Forward; public float HalfWidth, HalfDepth, SizeScale, Hour;
            public int Level; public bool ReducedMotion;
        }

        public void Tick(Frame f, float dt)
        {
            if (f.World == null) return;
            Scan(f);
            float day = Smooth(4.5f, 6.5f, f.Hour) * (1 - Smooth(19.5f, 21, f.Hour));
            float night = Mathf.Max(Smooth(19.5f, 21.5f, f.Hour), 1 - Smooth(4.5f, 6, f.Hour));
            float motion = f.ReducedMotion ? .35f : 1;
            float area = Mathf.Clamp(f.HalfWidth * f.HalfDepth / 300f, .35f, 2.2f);
            bool underground = f.Level < 0;
            float s = f.SizeScale;

            if (pollen != null)
            {
                float rate = (underground ? 9 : 14 * (.35f + .65f * day)) * area * motion;
                for (int n = Due(ref pollenDue, rate, dt); n > 0; n--)
                {
                    var p = RandomGround(f, out bool ok); if (!ok) continue;
                    var color = underground ? new Color(.62f, .7f, .72f, .45f) : Color.Lerp(new Color(1, .93f, .62f, .85f), new Color(.9f, .97f, .82f, .7f), (float)random.NextDouble());
                    Emit(pollen, p + Vector3.up * Range(.3f, 2.6f), new Vector3(Range(-.12f, .12f), Range(-.02f, .08f), Range(-.12f, .12f)), Range(4, 7), Range(.09f, .16f) * s, color);
                }
            }
            if (leaves != null && !underground && trees.Count > 0)
            {
                float rate = Mathf.Min(trees.Count, 60) * .08f * motion;
                for (int n = Due(ref leavesDue, rate, dt); n > 0; n--)
                {
                    var tree = trees[random.Next(trees.Count)];
                    var p = tree + new Vector3(Range(-.6f, .6f), Range(1.2f, 2.1f), Range(-.6f, .6f));
                    var color = Color.Lerp(new Color(.52f, .7f, .3f), new Color(.86f, .7f, .32f), (float)random.NextDouble() * .8f);
                    Emit(leaves, p, new Vector3(Range(-.25f, .25f), -.25f, Range(-.25f, .25f)), Range(4.5f, 6.5f), Range(.15f, .22f) * s, color);
                }
            }
            if (butterflies != null && !underground && meadows.Count > 0)
            {
                float rate = Mathf.Min(meadows.Count, 80) * .035f * day * motion;
                for (int n = Due(ref butterfliesDue, rate, dt); n > 0; n--)
                {
                    var p = meadows[random.Next(meadows.Count)] + new Vector3(Range(-.4f, .4f), Range(.35f, 1.1f), Range(-.4f, .4f));
                    float size = Range(.2f, .27f) * s;
                    Emit(butterflies, p, new Vector3(Range(-.35f, .35f), .05f, Range(-.35f, .35f)), Range(6, 9), size, Wings[random.Next(Wings.Length)], new Vector3(size, size * .8f, size));
                }
            }
            if (fireflies != null && !underground && night > .02f && (meadows.Count > 0 || trees.Count > 0))
            {
                float rate = 16 * night * area * motion;
                for (int n = Due(ref firefliesDue, rate, dt); n > 0; n--)
                {
                    var spots = meadows.Count > 0 && (trees.Count == 0 || random.NextDouble() < .55) ? meadows : trees;
                    var p = spots[random.Next(spots.Count)] + new Vector3(Range(-.8f, .8f), Range(.25f, 1.4f), Range(-.8f, .8f));
                    Emit(fireflies, p, new Vector3(Range(-.1f, .1f), Range(-.03f, .05f), Range(-.1f, .1f)), Range(4, 7), Range(.24f, .36f) * s, new Color(.86f, 1, .45f, 1));
                }
            }
            if (glints != null && !underground && waters.Count > 0)
            {
                float rate = Mathf.Min(waters.Count, 80) * .09f * (.3f + .7f * day) * motion;
                for (int n = Due(ref glintsDue, rate, dt); n > 0; n--)
                {
                    var p = waters[random.Next(waters.Count)] + new Vector3(Range(-.45f, .45f), .04f, Range(-.45f, .45f));
                    Emit(glints, p, Vector3.zero, Range(.5f, .9f), Range(.14f, .24f) * s, new Color(1, .98f, .88f, .9f));
                }
            }
        }

        /// <summary>Drops every spawn spot and live particle (village or level change).</summary>
        public void Clear()
        {
            trees.Clear(); meadows.Clear(); waters.Clear(); scanning = false; scanLevel = int.MinValue;
            foreach (var system in systems) if (system != null) system.Clear();
        }

        private void Scan(Frame f)
        {
            var world = f.World;
            if (!scanning)
            {
                // Scan a box a little larger than the view so particles drift in from the edges.
                float reach = Mathf.Max(f.HalfWidth, f.HalfDepth) * 1.1f + 2;
                scanMinX = Mathf.FloorToInt(f.Center.x - reach); scanMaxX = Mathf.CeilToInt(f.Center.x + reach);
                scanMinZ = Mathf.FloorToInt(f.Center.z - reach); scanMaxZ = Mathf.CeilToInt(f.Center.z + reach);
                scanX = scanMinX; scanZ = scanMinZ; scanning = true;
                nextTrees.Clear(); nextMeadows.Clear(); nextWaters.Clear();
                if (f.Level != scanLevel) { trees.Clear(); meadows.Clear(); waters.Clear(); scanLevel = f.Level; }
            }
            int budget = ScanPerFrame;
            // Coarse stride when the box is huge (zoomed out) keeps a full pass under a second.
            int stride = (scanMaxX - scanMinX) > 70 ? 2 : 1;
            while (budget-- > 0 && scanning)
            {
                var tile = world.GetTile(new Int2(scanX, scanZ, f.Level));
                if (tile != null && f.Level == 0) Classify(world, tile);
                scanX += stride;
                if (scanX > scanMaxX) { scanX = scanMinX; scanZ += stride; }
                if (scanZ > scanMaxZ)
                {
                    scanning = false;
                    Swap(ref trees, ref nextTrees); Swap(ref meadows, ref nextMeadows); Swap(ref waters, ref nextWaters);
                }
            }
        }

        private void Classify(World world, Tile tile)
        {
            var p = tile.Position;
            if (tile.Water && !tile.Bridge)
            {
                if (tile.WaterDepth > .05) Keep(nextWaters, new Vector3(p.X, (float)tile.WaterSurface, p.Z));
                return;
            }
            if (tile.Wall || tile.Road || tile.Mountain) return;
            string resource = tile.Resource ?? "";
            if (tile.Amount > 0 && (resource == "logs" || resource == "wood")) Keep(nextTrees, new Vector3(p.X, (float)tile.Elevation, p.Z));
            else if (tile.Amount > 0 && resource == "food" || tile.Biome == "meadow" && resource == "" && !tile.Dirt) Keep(nextMeadows, new Vector3(p.X, (float)tile.Elevation, p.Z));
        }

        // Reservoir sampling keeps an even spread of at most MaxSpots spots without a second pass.
        private void Keep(List<Vector3> spots, Vector3 spot)
        {
            if (spots.Count < MaxSpots) { spots.Add(spot); return; }
            int slot = random.Next(MaxSpots * 3);
            if (slot < MaxSpots) spots[slot] = spot;
        }

        private Vector3 RandomGround(Frame f, out bool ok)
        {
            var p = f.Center + f.Right * Range(-f.HalfWidth, f.HalfWidth) + f.Forward * Range(-f.HalfDepth, f.HalfDepth);
            var cell = new Int2(Mathf.RoundToInt(p.x), Mathf.RoundToInt(p.z), f.Level);
            var tile = f.World.GetTile(cell);
            ok = tile != null;
            if (!ok) return p;
            p.y = (float)(tile.Water && !tile.Bridge ? tile.WaterSurface : f.World.WalkHeight(cell));
            return p;
        }

        // How many particles are owed this frame at the given rate per second (at most 8, so a hitch never floods).
        private static int Due(ref float due, float rate, float dt)
        {
            if (rate <= 0) { due = 0; return 0; }
            due += rate * dt;
            int count = Mathf.Min(8, (int)due);
            due = Mathf.Min(due - count, 1);
            return count;
        }

        private void Emit(ParticleSystem system, Vector3 position, Vector3 velocity, float life, float size, Color color, Vector3? size3D = null)
        {
            var emit = new ParticleSystem.EmitParams
            {
                position = position, velocity = velocity, startLifetime = life, startColor = color,
                rotation = Range(0, 360), applyShapeToPosition = false
            };
            if (size3D.HasValue) emit.startSize3D = size3D.Value; else emit.startSize = size;
            system.Emit(emit, 1);
        }

        private float Range(float a, float b) => a + (float)random.NextDouble() * (b - a);
        private static float Smooth(float a, float b, float x) { float t = Mathf.Clamp01((x - a) / (b - a)); return t * t * (3 - 2 * t); }
        private static void Swap(ref List<Vector3> a, ref List<Vector3> b) { var t = a; a = b; b = t; }

        private static void Fade(ParticleSystem system, float fadeIn, float fadeOut)
        {
            var color = system.colorOverLifetime; color.enabled = true; color.color = FxAmbientSystem.FadeInOut(fadeIn, fadeOut);
        }

        private static void Noise(ParticleSystem system, float strength, float frequency)
        {
            var noise = system.noise; noise.enabled = true; noise.strength = strength; noise.frequency = frequency; noise.scrollSpeed = .3f; noise.quality = ParticleSystemNoiseQuality.Medium; noise.damping = true;
        }

        // Wing beats: the sprite's width swings between full and narrow, cycles times over the particle's life.
        private static AnimationCurve Flap(int cycles)
        {
            var keys = new Keyframe[cycles * 2 + 1];
            for (int i = 0; i < keys.Length; i++) keys[i] = new Keyframe(i / (float)(keys.Length - 1), i % 2 == 0 ? 1 : .35f);
            return new AnimationCurve(keys);
        }

        // Fireflies glow up, blink twice and fade.
        private static Gradient Blink()
        {
            var g = new Gradient();
            g.SetKeys(new[] { new GradientColorKey(Color.white, 0), new GradientColorKey(Color.white, 1) },
                new[] { new GradientAlphaKey(0, 0), new GradientAlphaKey(1, .15f), new GradientAlphaKey(.15f, .32f), new GradientAlphaKey(1, .5f), new GradientAlphaKey(.2f, .68f), new GradientAlphaKey(.9f, .82f), new GradientAlphaKey(0, 1) });
            return g;
        }

        public void Dispose()
        {
            foreach (var system in systems) if (system != null) UnityEngine.Object.Destroy(system.gameObject);
        }
    }
}
