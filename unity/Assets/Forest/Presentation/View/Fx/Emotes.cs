using System;
using System.Collections.Generic;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// State marks above cats, drawn as instanced billboards (one draw per mark kind, no per-cat objects):
    /// rising "Z"s while a cat sleeps or naps, a sun "!" when it has been blocked for a moment, a sweat drop
    /// when a need is below <see cref="NeedThreshold"/>, a slow heart while pregnant, and a sparkle trail
    /// behind boosted cats. Births and pregnancies also get a heart burst, and switching a boost on a
    /// sparkle burst, from the <see cref="WorldEventFeed"/>. A cat shows at most one mark; sleep wins, then
    /// the blocked mark, then the need drop, then the heart.
    /// </summary>
    internal sealed class Emotes : IDisposable
    {
        public const double NeedThreshold = 30;
        /// <summary>A cat must stay blocked this long before its "!" shows, so routine one-frame waits do not flicker.</summary>
        public const float BlockedDelay = 1.2f;

        private static readonly Color Sun = new Color(1, .82f, .4f);          // --sun #ffd166
        private static readonly Color Info = new Color(.49f, .78f, .9f);      // --info #7cc6e6
        private static readonly Color Sleep = new Color(.9f, .94f, 1f);
        private static readonly Color Love = new Color(1, .56f, .69f);

        private sealed class CatState
        {
            public Vector3 Last; public float BlockedFor, Moving, Trail, Shown; public int Mark = -1, Seen; public float Phase;
        }

        private const int Z = 0, Bang = 1, Drop = 2, Heart = 3;
        private readonly FxInstances[] marks;
        private readonly Dictionary<string, CatState> states = new Dictionary<string, CatState>();
        private readonly List<string> stale = new List<string>();
        private readonly ForestFx fx;
        private readonly ParticleSystem trail;
        private int stamp;

        public int Drawn { get; private set; }

        public Emotes(ForestFx fx, Transform root, FxAssets assets, Mesh quad)
        {
            this.fx = fx;
            marks = new[]
            {
                new FxInstances(assets.Billboard(FxSprites.Zzz), quad, 256),
                new FxInstances(assets.Billboard(FxSprites.Bang), quad, 256),
                new FxInstances(assets.Billboard(FxSprites.Drop), quad, 256),
                new FxInstances(assets.Billboard(FxSprites.Heart), quad, 256),
            };
            var glow = assets.Glow(FxSprites.Star);
            if (glow != null) trail = FxAmbientSystem.Create(root, "Boost sparkle trail", glow, 120);
            if (trail != null)
            {
                var color = trail.colorOverLifetime; color.enabled = true; color.color = FxAmbientSystem.FadeInOut(.1f, .55f);
                var size = trail.sizeOverLifetime; size.enabled = true; size.size = new ParticleSystem.MinMaxCurve(1, AnimationCurve.EaseInOut(0, 1, 1, .2f));
            }
        }

        public void Subscribe(WorldEventFeed feed)
        {
            feed.Noted += OnNoted;
            feed.CatBoostChanged += OnBoost;
        }

        public void Unsubscribe(WorldEventFeed feed)
        {
            feed.Noted -= OnNoted;
            feed.CatBoostChanged -= OnBoost;
        }

        private void OnNoted(FeedNote note)
        {
            if (!note.HasEntityPosition) return;
            if (note.Kind == EventKind.Birth) fx.Burst(FxKind.Hearts, note.Position + Vector3.up * .45f, Color.white, 7);
            else if (note.Kind == EventKind.Pregnancy) fx.Burst(FxKind.Hearts, note.Position + Vector3.up * .5f, Color.white, 4);
        }

        private void OnBoost(FeedCat cat)
        {
            if (cat.Cat != null && cat.Cat.Boosted) fx.Burst(FxKind.Sparkle, cat.Position + Vector3.up * .35f, Color.white, 12);
        }

        /// <summary>Builds this frame's marks for the cats on the rendered level and draws them.</summary>
        public void Render(ForestView view, World world, Village village, Camera camera, float dt, float sizeScale, bool reducedMotion, bool showcase = false)
        {
            foreach (var batch in marks) batch.Clear();
            Drawn = 0;
            if (camera == null) return;
            stamp++;
            int level = view.RenderedLevel;
            float time = Time.unscaledTime;
            var right = camera.transform.right; var up = camera.transform.up;
            var facing = camera.transform.rotation;
            double now = world.TimeSeconds;
            int index = -1;
            foreach (var c in village.Cats)
            {
                index++;
                if (!c.Alive || c.Position.Level != level) continue;
                if (!view.FxEntity(ForestFx.CatKey(c.Id), out var root)) continue;
                if (!states.TryGetValue(c.Id, out var state))
                {
                    state = new CatState { Last = root.position, Phase = (World.Hash(c.Id) % 1000) / 1000f };
                    states[c.Id] = state;
                }
                state.Seen = stamp;
                var p = root.position;
                float speed = dt > 0 ? (p - state.Last).magnitude / dt : 0;
                state.Last = p;
                state.Moving = Mathf.Lerp(state.Moving, speed > .25f ? 1 : 0, 1 - Mathf.Exp(-dt * 8));
                state.BlockedFor = c.BlockedReason != "" ? state.BlockedFor + dt : 0;
                if ((c.Boosted || showcase && index % 5 == 4) && trail != null) EmitTrail(state, p, dt, reducedMotion);

                var viewport = camera.WorldToViewportPoint(p);
                if (viewport.z <= 0 || viewport.x < -.1f || viewport.x > 1.1f || viewport.y < -.1f || viewport.y > 1.15f) { state.Mark = -1; continue; }

                bool sleeping = (c.Goal == GoalKey.NeedSleep || c.Goal == GoalKey.IdleNap) && state.Moving < .5f;
                double lowest = Math.Min(c.Hunger, Math.Min(c.Thirst, c.Rest));
                int mark = sleeping ? Z
                    : state.BlockedFor >= BlockedDelay ? Bang
                    : lowest < NeedThreshold ? Drop
                    : c.PregnantUntil > now ? Heart
                    : -1;
                // Developer evidence (--forest-fx-showcase): cycle every mark across the colony.
                if (showcase) mark = index % 5 == 4 ? -1 : index % 5;
                if (mark != state.Mark) { state.Mark = mark; state.Shown = 0; }
                state.Shown += dt;
                if (mark < 0) continue;

                float s = root.localScale.y;
                var head = p + Vector3.up * (.62f * s);
                float pop = reducedMotion ? 1 : Pop(state.Shown);
                float phase = state.Phase;
                switch (mark)
                {
                    case Z:
                        if (reducedMotion) { Add(Z, head + right * .16f + up * .22f, facing, .2f * sizeScale, Sleep, .9f); break; }
                        for (int k = 0; k < 2; k++)
                        {
                            float f = Frac(time * .5f + phase + k * .5f);
                            float a = Mathf.Sin(f * Mathf.PI);
                            Add(Z, head + right * (.12f + .2f * f) + up * (.12f + .42f * f), facing, (.18f + .15f * f) * sizeScale, Sleep, a);
                        }
                        break;
                    case Bang:
                    {
                        float bob = reducedMotion ? 0 : Mathf.Sin(time * 3.2f + phase * 6.28f) * .03f;
                        Add(Bang, head + right * .26f + up * (.3f + bob), facing, .34f * sizeScale * pop, Sun, 1);
                        break;
                    }
                    case Drop:
                    {
                        float f = reducedMotion ? .3f : Frac(time * .7f + phase);
                        float a = f < .15f ? f / .15f : 1 - (f - .15f) / .85f;
                        Add(Drop, head - right * .24f + up * (.2f - .14f * f), facing, .27f * sizeScale * pop, Info, reducedMotion ? 1 : a);
                        break;
                    }
                    case Heart:
                    {
                        float f = reducedMotion ? .4f : Frac(time * .32f + phase);
                        float a = Mathf.Sin(f * Mathf.PI);
                        Add(Heart, head + up * (.22f + .4f * f) + right * (Mathf.Sin(f * 9) * .05f), facing, (.22f + .07f * f) * sizeScale, Love, reducedMotion ? 1 : a);
                        break;
                    }
                }
            }
            stale.Clear();
            foreach (var pair in states) if (pair.Value.Seen != stamp) stale.Add(pair.Key);
            foreach (var id in stale) states.Remove(id);
            foreach (var batch in marks) { batch.Draw(); Drawn += batch.Count; }
        }

        private void Add(int mark, Vector3 position, Quaternion facing, float size, Color color, float alpha)
        {
            if (alpha <= .01f) return;
            color.a *= Mathf.Clamp01(alpha);
            marks[mark].Add(Matrix4x4.TRS(position, facing, new Vector3(size, size, size)), color);
        }

        private void EmitTrail(CatState state, Vector3 position, float dt, bool reducedMotion)
        {
            float rate = (state.Moving > .5f ? 7 : 2.5f) * (reducedMotion ? .4f : 1);
            state.Trail += rate * dt;
            while (state.Trail >= 1)
            {
                state.Trail -= 1;
                var jitter = UnityEngine.Random.insideUnitSphere * .18f;
                var emit = new ParticleSystem.EmitParams
                {
                    position = position + Vector3.up * (.25f + UnityEngine.Random.value * .3f) + new Vector3(jitter.x, 0, jitter.z),
                    velocity = Vector3.up * (.15f + UnityEngine.Random.value * .25f),
                    startLifetime = .7f + UnityEngine.Random.value * .4f,
                    startSize = .1f + UnityEngine.Random.value * .1f,
                    startColor = Color.Lerp(new Color(1, .84f, .42f), new Color(1, .96f, .8f), UnityEngine.Random.value),
                    rotation = UnityEngine.Random.value * 360,
                    applyShapeToPosition = false
                };
                trail.Emit(emit, 1);
            }
        }

        private static float Frac(float x) => x - Mathf.Floor(x);
        private static float Pop(float age) => age >= .22f ? 1 : 1 + .35f * Mathf.Sin(age / .22f * Mathf.PI) - (1 - age / .22f) * .3f;

        public void Clear() { states.Clear(); if (trail != null) trail.Clear(); }

        public void Dispose()
        {
            foreach (var batch in marks) batch.Dispose();
            if (trail != null) UnityEngine.Object.Destroy(trail.gameObject);
        }
    }
}
