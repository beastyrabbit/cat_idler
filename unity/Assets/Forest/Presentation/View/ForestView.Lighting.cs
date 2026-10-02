using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Sun, ambient light, fog, sky and the underground lantern. The look itself lives in
    /// <see cref="LightingRig"/> and <see cref="LookProfile"/>; this partial feeds the rig the clock, the
    /// rendered level, the camera and the lantern's cat every frame. Scene lights are left alone.
    /// </summary>
    public sealed partial class ForestView
    {
        private Light sun;
        private LightingRig lighting;
        private bool lightingSawWorld;
        private string lanternCatId = "", lanternEntityId = "";

        /// <summary>The lighting rig (sun, lantern, fog and sky); W4-01 extends it.</summary>
        internal LightingRig Lighting => lighting;

        private void CreateLights()
        {
            var host = new GameObject("Sun and moon");
            sun = host.AddComponent<Light>();
            lighting = host.AddComponent<LightingRig>();
            lighting.Initialise(LookProfile.Load(), sun, Camera);
        }

        partial void LevelChangedLighting(int level)
        {
            if (lighting == null || lightingSawWorld) return;
            lightingSawWorld = true;
            lighting.Snap();
        }

        partial void UpdateLighting(float dt)
        {
            if (lighting == null) return;
            var inputs = new LookInputs
            {
                DayFraction = LookClock.DayFraction(Game.CurrentWorld),
                Level = RenderedLevel,
                DirectControl = DirectControl,
                Standoff = CameraStandoff,
                Zoom = Zoom,
            };
            inputs.HasLantern = LanternPosition(inputs.Level, out inputs.Lantern);
            lighting.Tick(inputs, dt);
        }

        /// <summary>Fog distances for the active camera; called every frame by UpdateCamera.</summary>
        private void ApplyFogForCamera(bool directControl)
        {
            if (lighting != null) lighting.ApplyFog(directControl, CameraStandoff, Zoom);
        }

        /// <summary>Underground, the lantern follows the controlled cat, else the selected cat on this level,
        /// else the first cat on this level (an explorer).</summary>
        private bool LanternPosition(int level, out Vector3 position)
        {
            position = default;
            if (level >= 0) return false;
            var cat = ControlledCat;
            if (cat == null)
            {
                var selected = SelectedCat;
                if (selected != null && selected.Alive && selected.Position.Level == level) cat = selected;
            }
            if (cat == null)
                foreach (var candidate in Game.Selected.Cats)
                    if (candidate.Alive && candidate.Position.Level == level) { cat = candidate; break; }
            if (cat == null) return false;
            if (cat.Id != lanternCatId) { lanternCatId = cat.Id; lanternEntityId = "cat:" + cat.Id; }
            position = TryGetEntity(lanternEntityId, out var go) && go != null ? go.transform.position : Position(cat);
            return true;
        }
    }
}
