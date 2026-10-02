using System;
using System.Globalization;
using IdleCatForest.Simulation;
using UnityEngine;
using UnityEngine.Rendering;

namespace IdleCatForest.Presentation
{
    /// <summary>What the view tells the rig each frame.</summary>
    public struct LookInputs
    {
        /// <summary>Time of day from 0 (midnight) to 1, see <see cref="LookClock"/>.</summary>
        public double DayFraction;
        public int Level;
        public bool DirectControl;
        public float Standoff;
        public float Zoom;
        public bool HasLantern;
        public Vector3 Lantern;
    }

    /// <summary>
    /// The clock the look follows: <see cref="DayClock.DayFraction"/>, unless a capture or QA run overrides it
    /// with <c>FOREST_LOOK_HOUR</c> (a fixed hour, 0 to 24) or <c>FOREST_LOOK_DAY_SCALE</c> (a game-time scale
    /// for the look only, for example 60 to preview the demo day on a classic world). Neither changes the
    /// simulation or the HUD clock.
    /// </summary>
    public static class LookClock
    {
        private static readonly double fixedHour = Read("FOREST_LOOK_HOUR", 0, 24);
        private static readonly double dayScale = Read("FOREST_LOOK_DAY_SCALE", 0, 100000);

        public static double DayFraction(World world)
        {
            if (!double.IsNaN(fixedHour)) return fixedHour / 24;
            if (world == null) return DayClock.StartMinutes / DayClock.MinutesPerDay;
            if (double.IsNaN(dayScale)) return DayClock.DayFraction(world);
            double minutes = (DayClock.StartMinutes + world.TimeSeconds * dayScale / 60) % DayClock.MinutesPerDay;
            return (minutes < 0 ? minutes + DayClock.MinutesPerDay : minutes) / DayClock.MinutesPerDay;
        }

        private static double Read(string name, double min, double max)
        {
            string text;
            try { text = Environment.GetEnvironmentVariable(name); }
            catch (System.Security.SecurityException) { return double.NaN; }
            return double.TryParse(text, NumberStyles.Float, CultureInfo.InvariantCulture, out double value) && value >= min && value <= max ? value : double.NaN;
        }
    }

    /// <summary>
    /// Sun (and moon), trilight ambient, fog, clear colour, direct-control sky, shadow settings and the
    /// underground lantern, driven every frame by <see cref="LookProfile"/>. It lives on the sun's GameObject, so
    /// destroying the sun (ForestView.OnDestroy) also removes the lantern and the runtime sky resources.
    /// Every look value blends toward its target over <see cref="LookProfile.BlendSeconds"/>; the fog mode
    /// switches at once (Linear on the surface, Exp2 underground).
    /// </summary>
    public sealed class LightingRig : MonoBehaviour
    {
        private const int SkyHeight = 128;
        // Unity's Skybox/Panoramic multiplies by _Tint × unity_ColorSpaceDouble; .504 makes that 1.0 in linear space.
        private const float NeutralSkyTint = .504f;
        private const float DirectDistance = 4.55f;

        public LookProfile Profile { get; private set; }
        public Light Sun { get; private set; }
        public Light Lantern { get; private set; }
        /// <summary>The look currently on screen (after blending).</summary>
        public Color FogColor => state.Fog;

        private struct State
        {
            public Quaternion SunRotation;
            public Color SunColor, AmbientSky, AmbientEquator, AmbientGround, Fog, Clear, Zenith, SkyGround;
            public float SunIntensity, ShadowStrength, Lantern, Underground;

            public static State Lerp(in State a, in State b, float t)
            {
                return new State
                {
                    SunRotation = Quaternion.Slerp(a.SunRotation, b.SunRotation, t),
                    SunColor = Color.Lerp(a.SunColor, b.SunColor, t),
                    AmbientSky = Color.Lerp(a.AmbientSky, b.AmbientSky, t),
                    AmbientEquator = Color.Lerp(a.AmbientEquator, b.AmbientEquator, t),
                    AmbientGround = Color.Lerp(a.AmbientGround, b.AmbientGround, t),
                    Fog = Color.Lerp(a.Fog, b.Fog, t),
                    Clear = Color.Lerp(a.Clear, b.Clear, t),
                    Zenith = Color.Lerp(a.Zenith, b.Zenith, t),
                    SkyGround = Color.Lerp(a.SkyGround, b.SkyGround, t),
                    SunIntensity = Mathf.Lerp(a.SunIntensity, b.SunIntensity, t),
                    ShadowStrength = Mathf.Lerp(a.ShadowStrength, b.ShadowStrength, t),
                    Lantern = Mathf.Lerp(a.Lantern, b.Lantern, t),
                    Underground = Mathf.Lerp(a.Underground, b.Underground, t),
                };
            }
        }

        private Camera view;
        private State state;
        private bool hasState, underground, direct;
        private Material skyMaterial;
        private Texture2D skyTexture;
        private Color32[] skyPixels;
        private Color skyZenith, skyHorizon, skyGround;
        private float nextSkyUpdate;

        /// <summary>Called once by ForestView.CreateLights.</summary>
        public void Initialise(LookProfile profile, Light sun, Camera camera)
        {
            Profile = profile != null ? profile : LookProfile.Load();
            Sun = sun; view = camera;
            Sun.type = LightType.Directional;
            Sun.shadows = LightShadows.Soft;
            Sun.renderMode = LightRenderMode.ForcePixel;
            var lanternHost = new GameObject("Lantern");
            lanternHost.transform.SetParent(transform, false);
            Lantern = lanternHost.AddComponent<Light>();
            Lantern.type = LightType.Point;
            Lantern.shadows = LightShadows.Soft;
            Lantern.renderMode = LightRenderMode.ForcePixel;
            Lantern.enabled = false;
            CreateSky();
            RenderSettings.ambientMode = AmbientMode.Trilight;
            RenderSettings.fog = true;
            ApplyQuality();
            var first = new LookInputs { DayFraction = LookClock.DayFraction(null), Level = 0, Standoff = 50, Zoom = 14 };
            Tick(first, 0);
        }

        /// <summary>Jumps to the target look on the next tick instead of blending (used on the first world frame).</summary>
        public void Snap() => hasState = false;

        public void Tick(in LookInputs inputs, float dt)
        {
            if (Profile == null) return;
            ApplyQuality();
            underground = inputs.Level < 0;
            direct = inputs.DirectControl;
            var target = Target(inputs);
            if (!hasState || Profile.BlendSeconds <= 0) { state = target; hasState = true; }
            else state = State.Lerp(state, target, 1 - Mathf.Exp(-Mathf.Max(0, dt) * 3 / Profile.BlendSeconds));
            Apply(inputs);
        }

        /// <summary>Fog for the active camera; also called by ForestView.UpdateCamera every frame.</summary>
        public void ApplyFog(bool directControl, float standoff, float zoom)
        {
            if (Profile == null) return;
            RenderSettings.fogColor = state.Fog;
            if (underground)
            {
                RenderSettings.fogMode = FogMode.ExponentialSquared;
                float distance = Mathf.Max(DirectDistance, standoff);
                RenderSettings.fogDensity = Profile.Underground.FogDensity * DirectDistance / distance;
                return;
            }
            RenderSettings.fogMode = FogMode.Linear;
            var fog = Profile.Fog;
            if (directControl) { RenderSettings.fogStartDistance = fog.DirectStart; RenderSettings.fogEndDistance = fog.DirectEnd; }
            else
            {
                RenderSettings.fogStartDistance = standoff + zoom * fog.ManagementStartZoom;
                RenderSettings.fogEndDistance = standoff + zoom * fog.ManagementEndZoom + fog.ManagementEndPad;
            }
        }

        private State Target(in LookInputs inputs)
        {
            var key = Profile.Evaluate(inputs.DayFraction);
            float yaw = Profile.ReferenceYaw;
            if (underground)
            {
                var u = Profile.Underground;
                bool close = inputs.DirectControl;
                return new State
                {
                    SunRotation = Quaternion.Euler(u.SunElevation, yaw + 90, 0),
                    SunColor = u.SunColor, SunIntensity = close ? u.SunIntensity : u.ManagementSunIntensity, ShadowStrength = u.ShadowStrength,
                    AmbientSky = close ? u.AmbientSky : u.ManagementAmbientSky,
                    AmbientEquator = close ? u.AmbientEquator : u.ManagementAmbientEquator,
                    AmbientGround = close ? u.AmbientGround : u.ManagementAmbientGround,
                    Fog = u.Fog, Clear = u.Clear, Zenith = u.Clear, SkyGround = u.Clear,
                    Lantern = inputs.HasLantern ? 1 : 0, Underground = 1,
                };
            }
            return new State
            {
                SunRotation = Quaternion.Euler(key.Elevation, yaw + key.Azimuth, 0),
                SunColor = key.SunColor, SunIntensity = key.SunIntensity, ShadowStrength = key.ShadowStrength,
                AmbientSky = key.AmbientSky, AmbientEquator = key.AmbientEquator, AmbientGround = key.AmbientGround,
                Fog = key.Fog, Clear = key.Fog, Zenith = key.Zenith,
                SkyGround = Color.Lerp(key.Fog, key.AmbientGround, Profile.SkyGroundDarkening),
                Lantern = 0, Underground = 0,
            };
        }

        private void Apply(in LookInputs inputs)
        {
            Sun.transform.rotation = state.SunRotation;
            Sun.color = state.SunColor;
            Sun.intensity = state.SunIntensity;
            Sun.shadowStrength = state.ShadowStrength;
            Sun.shadowBias = Profile.Shadows.Bias;
            Sun.shadowNormalBias = Profile.Shadows.NormalBias;
            RenderSettings.ambientSkyColor = state.AmbientSky;
            RenderSettings.ambientEquatorColor = state.AmbientEquator;
            RenderSettings.ambientGroundColor = state.AmbientGround;
            RenderSettings.reflectionIntensity = Profile.ReflectionIntensity;
            RenderSettings.fog = true;
            ApplyFog(inputs.DirectControl, inputs.Standoff, inputs.Zoom);

            var shadows = Profile.Shadows;
            float distance = Mathf.Max(10, inputs.Standoff + inputs.Zoom * shadows.DistanceZoom + shadows.DistancePad);
            QualitySettings.shadowDistance = distance;
            QualitySettings.shadowCascade2Split = Mathf.Clamp((inputs.Standoff + inputs.Zoom * shadows.NearCascadeZoom) / distance, .1f, .9f);

            var u = Profile.Underground;
            float lantern = state.Lantern * state.Underground;
            Lantern.enabled = inputs.HasLantern && lantern > .01f;
            if (Lantern.enabled)
            {
                Lantern.transform.position = inputs.Lantern + Vector3.up * u.LanternHeight;
                Lantern.color = u.LanternColor; Lantern.range = u.LanternRange; Lantern.intensity = u.LanternIntensity * lantern;
            }

            if (view == null) return;
            bool sky = direct && !underground && skyMaterial != null;
            if (sky)
            {
                UpdateSky();
                RenderSettings.skybox = skyMaterial;
                view.clearFlags = CameraClearFlags.Skybox;
            }
            else view.clearFlags = CameraClearFlags.SolidColor;
            view.backgroundColor = state.Clear;
        }

        /// <summary>Keeps the quality level on the player's preset (Low, Medium, High = levels 0, 1, 2).</summary>
        private static void ApplyQuality()
        {
            int wanted = Mathf.Clamp((int)GameSettings.Quality, 0, QualitySettings.count - 1);
            if (QualitySettings.GetQualityLevel() != wanted) QualitySettings.SetQualityLevel(wanted, true);
        }

        private void CreateSky()
        {
            var template = Profile.Sky;
            if (template == null)
            {
                var shader = Shader.Find("Skybox/Panoramic");
                if (shader == null) return;
                template = new Material(shader);
                template.SetFloat("_Mapping", 1);
                template.SetColor("_Tint", new Color(NeutralSkyTint, NeutralSkyTint, NeutralSkyTint));
                skyMaterial = template;
            }
            else skyMaterial = new Material(template);
            skyMaterial.name = "Forest sky (runtime)";
            skyMaterial.DisableKeyword("_MAPPING_6_FRAMES_LAYOUT");
            skyTexture = new Texture2D(4, SkyHeight, TextureFormat.RGBA32, false, false)
            { name = "Forest sky gradient", wrapModeU = TextureWrapMode.Repeat, wrapModeV = TextureWrapMode.Clamp, filterMode = FilterMode.Bilinear };
            skyPixels = new Color32[4 * SkyHeight];
            skyMaterial.SetTexture("_MainTex", skyTexture);
        }

        /// <summary>Regenerates the three-colour gradient (zenith, horizon = fog colour, ground) when it changed.</summary>
        private void UpdateSky()
        {
            if (skyTexture == null) return;
            bool changed = Different(skyZenith, state.Zenith) || Different(skyHorizon, state.Fog) || Different(skyGround, state.SkyGround);
            if (!changed || Time.unscaledTime < nextSkyUpdate && skyHorizon.a > 0) return;
            skyZenith = state.Zenith; skyHorizon = state.Fog; skyGround = state.SkyGround; skyHorizon.a = 1;
            nextSkyUpdate = Time.unscaledTime + .25f;
            for (int y = 0; y < SkyHeight; y++)
            {
                // Latitude-longitude: row 0 looks straight down, the middle row is the horizon.
                float elevation = (y + .5f) / SkyHeight * 2 - 1;
                Color c = elevation >= 0
                    ? Color.Lerp(skyHorizon, skyZenith, Mathf.Pow(elevation, .55f))
                    : Color.Lerp(skyHorizon, skyGround, Mathf.Pow(-elevation, .35f));
                c.a = 1;
                Color32 pixel = c;
                for (int x = 0; x < 4; x++) skyPixels[y * 4 + x] = pixel;
            }
            skyTexture.SetPixels32(skyPixels);
            skyTexture.Apply(false);
        }

        private static bool Different(Color a, Color b) =>
            Mathf.Abs(a.r - b.r) + Mathf.Abs(a.g - b.g) + Mathf.Abs(a.b - b.b) > 1f / 255;

        private void OnDestroy()
        {
            if (skyMaterial != null && RenderSettings.skybox == skyMaterial) RenderSettings.skybox = null;
            if (skyMaterial != null) Destroy(skyMaterial);
            if (skyTexture != null) Destroy(skyTexture);
        }
    }
}
