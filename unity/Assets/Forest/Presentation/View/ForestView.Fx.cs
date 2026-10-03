using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Fx hooks: creates the effect layer (<see cref="ForestFx"/>) on Awake, observes the world and advances
    /// effects in Update, and draws emotes, blob shadows and chips in LateUpdate once cats and the camera have
    /// moved. Also the narrow bridge the effect classes use to read view state they must not own.
    /// </summary>
    public sealed partial class ForestView
    {
        private ForestFx fx;

        /// <summary>The effect layer (bursts, chips, punches, emotes, shadows, ambient life).</summary>
        public ForestFx Fx => fx;

        /// <summary>Typed events diffed from the world each frame; subscribe for effects, audio or UI reactions.</summary>
        public WorldEventFeed Feed => fx != null ? fx.Feed : null;

        partial void AwakeFx()
        {
            fx = gameObject.AddComponent<ForestFx>();
            fx.Bind(this);
        }

        partial void UpdateFx(float dt) { if (fx != null) fx.Tick(dt); }

        partial void LateUpdateFx() { if (fx != null) fx.LateTick(Time.unscaledDeltaTime); }

        internal ForestGame FxGame => Game;

        /// <summary>The point the camera frames: the controlled cat in direct control, else the management focus.</summary>
        internal Vector3 FxFocus { get { var cat = ControlledCat; return cat != null ? Position(cat) : focus; } }

        /// <summary>The rendered root of an entity id (for example <c>cat:&lt;id&gt;</c>), if it exists this frame.</summary>
        internal bool FxEntity(string id, out Transform root)
        {
            if (entities.TryGetValue(id, out var go) && go != null) { root = go.transform; return true; }
            root = null; return false;
        }
    }
}
