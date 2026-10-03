using System;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// One point of the day cycle. <see cref="Hour"/> is game time (0 to 24); the sun's direction is its
    /// <see cref="Elevation"/> above the horizon and its <see cref="Azimuth"/> measured from the reference
    /// camera yaw (<see cref="LookProfile.ReferenceYaw"/>): 90 means the light travels to the camera's right,
    /// so shadows fall sideways across the screen.
    /// </summary>
    [Serializable]
    public struct LookKey
    {
        public float Hour;
        [Range(0, 90)] public float Elevation;
        [Range(-180, 180)] public float Azimuth;
        public Color SunColor;
        [Range(0, 3)] public float SunIntensity;
        [Range(0, 1)] public float ShadowStrength;
        public Color AmbientSky, AmbientEquator, AmbientGround;
        /// <summary>Fog colour, management clear colour and the horizon of the direct-control sky.</summary>
        public Color Fog;
        /// <summary>Top of the direct-control sky gradient.</summary>
        public Color Zenith;

        public static LookKey Lerp(in LookKey a, in LookKey b, float t)
        {
            return new LookKey
            {
                Hour = Mathf.Lerp(a.Hour, b.Hour, t),
                Elevation = Mathf.Lerp(a.Elevation, b.Elevation, t),
                Azimuth = Mathf.LerpAngle(a.Azimuth, b.Azimuth, t),
                SunColor = Color.Lerp(a.SunColor, b.SunColor, t),
                SunIntensity = Mathf.Lerp(a.SunIntensity, b.SunIntensity, t),
                ShadowStrength = Mathf.Lerp(a.ShadowStrength, b.ShadowStrength, t),
                AmbientSky = Color.Lerp(a.AmbientSky, b.AmbientSky, t),
                AmbientEquator = Color.Lerp(a.AmbientEquator, b.AmbientEquator, t),
                AmbientGround = Color.Lerp(a.AmbientGround, b.AmbientGround, t),
                Fog = Color.Lerp(a.Fog, b.Fog, t),
                Zenith = Color.Lerp(a.Zenith, b.Zenith, t),
            };
        }
    }

    /// <summary>
    /// Below the surface: a dim sun, dark ambient, near-black clear colour, tight fog and a lantern. Direct control
    /// uses the dark ambient (the lantern lights the cat's surroundings); the management view, which has no
    /// lantern when nobody is down there, keeps a brighter cave ambient so the floor stays readable.
    /// </summary>
    [Serializable]
    public sealed class UndergroundLook
    {
        public Color SunColor = new Color(.62f, .66f, .85f);
        [Range(0, 1)] public float SunIntensity = .1f;
        [Range(0, 1)] public float ManagementSunIntensity = .2f;
        [Range(0, 90)] public float SunElevation = 62;
        [Range(0, 1)] public float ShadowStrength = .5f;
        public Color AmbientSky = new Color(.14f, .12f, .17f);
        public Color AmbientEquator = new Color(.11f, .10f, .14f);
        public Color AmbientGround = new Color(.06f, .055f, .07f);
        public Color ManagementAmbientSky = new Color(.36f, .32f, .40f);
        public Color ManagementAmbientEquator = new Color(.28f, .25f, .32f);
        public Color ManagementAmbientGround = new Color(.12f, .10f, .13f);
        public Color Clear = new Color(.025f, .022f, .03f);
        public Color Fog = new Color(.03f, .026f, .035f);
        /// <summary>Exp2 density at the direct-control camera distance; scaled down for farther cameras so the
        /// fog at the framed point stays the same.</summary>
        [Range(0, .3f)] public float FogDensity = .08f;
        public Color LanternColor = new Color(1f, .70f, .40f);
        [Range(0, 8)] public float LanternIntensity = 2.2f;
        [Range(1, 20)] public float LanternRange = 6;
        [Range(0, 3)] public float LanternHeight = 1.3f;
    }

    /// <summary>Linear fog on the surface. Management distances follow the camera so the haze always starts
    /// just past the framed point and thickens toward the top of the screen.</summary>
    [Serializable]
    public sealed class FogLook
    {
        public float ManagementStartZoom = .25f;
        public float ManagementEndZoom = 2.2f;
        public float ManagementEndPad = 12;
        public float DirectStart = 18;
        public float DirectEnd = 40;
    }

    [Serializable]
    public sealed class ShadowLook
    {
        [Range(0, 2)] public float Bias = .03f;
        [Range(0, 3)] public float NormalBias = .3f;
        /// <summary>Shadow distance = camera standoff + zoom × this + <see cref="DistancePad"/>.</summary>
        public float DistanceZoom = 1.8f;
        public float DistancePad = 8;
        /// <summary>The first of two cascades ends this far past the framed point (in zoom units), so the
        /// centre of the screen gets the sharp cascade.</summary>
        public float NearCascadeZoom = .15f;
    }

    /// <summary>
    /// The authored look: a day cycle over <c>DayClock.DayFraction</c>, the underground look, fog, shadows and
    /// the direct-control sky. Loaded from <c>Resources/ForestLook.asset</c> (created by the
    /// <c>forest_look_assets</c> editor command); the field initialisers are the code defaults and the fallback
    /// when the asset is missing.
    /// </summary>
    [CreateAssetMenu(menuName = "Forest/Look profile", fileName = ResourceName)]
    public sealed class LookProfile : ScriptableObject
    {
        public const string ResourceName = "ForestLook";

        /// <summary>Seconds to blend between looks (level changes, entering direct control).</summary>
        [Range(0, 3)] public float BlendSeconds = .4f;
        /// <summary>Default management camera yaw the sun azimuths are measured from.</summary>
        public float ReferenceYaw = 40;
        [Range(0, 1)] public float ReflectionIntensity = .3f;
        /// <summary>Day-cycle keys sorted by hour; the cycle wraps from the last key to the first.</summary>
        public LookKey[] Day = DefaultDay();
        public UndergroundLook Underground = new UndergroundLook();
        public FogLook Fog = new FogLook();
        public ShadowLook Shadows = new ShadowLook();
        /// <summary>Darkening of the sky below the horizon (0 keeps the fog colour, 1 reaches the ambient ground).</summary>
        [Range(0, 1)] public float SkyGroundDarkening = .55f;
        /// <summary>Skybox/Panoramic material (latitude-longitude) the rig copies and feeds a generated gradient.</summary>
        public Material Sky;

        public static LookProfile Load()
        {
            var profile = Resources.Load<LookProfile>(ResourceName);
            return profile != null ? profile : CreateInstance<LookProfile>();
        }

        /// <summary>The blended day-cycle key at a time of day from 0 (midnight) to 1.</summary>
        public LookKey Evaluate(double dayFraction)
        {
            var keys = Day;
            if (keys == null || keys.Length == 0) keys = Day = DefaultDay();
            if (keys.Length == 1) return keys[0];
            double wrapped = dayFraction - Math.Floor(dayFraction);
            float hour = (float)(wrapped * 24);
            int next = 0;
            while (next < keys.Length && keys[next].Hour <= hour) next++;
            int previous = next == 0 ? keys.Length - 1 : next - 1;
            if (next == keys.Length) next = 0;
            float from = keys[previous].Hour, to = keys[next].Hour;
            if (to <= from) to += 24;
            float at = hour < from ? hour + 24 : hour;
            float t = to - from <= 0 ? 0 : Mathf.Clamp01((at - from) / (to - from));
            var key = LookKey.Lerp(keys[previous], keys[next], Mathf.SmoothStep(0, 1, t));
            key.Hour = hour;
            return key;
        }

        private void OnValidate()
        {
            if (Day != null) Array.Sort(Day, (a, b) => a.Hour.CompareTo(b.Hour));
        }

        private static LookKey Key(float hour, float elevation, float azimuth, Color sun, float intensity, float shadow,
            Color sky, Color equator, Color ground, Color fog, Color zenith)
        {
            return new LookKey
            {
                Hour = hour, Elevation = elevation, Azimuth = azimuth, SunColor = sun, SunIntensity = intensity,
                ShadowStrength = shadow, AmbientSky = sky, AmbientEquator = equator, AmbientGround = ground,
                Fog = fog, Zenith = zenith,
            };
        }

        /// <summary>
        /// Sun model: up from dawn (06:00) to dusk (19:00), highest at 12:30; the azimuth sweeps from 60° to 120°
        /// off the reference view direction, so shadows always fall across the screen. A dim blue moon at 52°
        /// lights the night (21:00 to 04:30); the light swaps direction while it is dimmest (05:00, 20:40).
        /// </summary>
        public static LookKey[] DefaultDay()
        {
            var moon = new Color(.60f, .70f, 1f);
            var nightSky = new Color(.32f, .38f, .58f);
            var nightEquator = new Color(.23f, .27f, .40f);
            var nightGround = new Color(.10f, .10f, .14f);
            var nightFog = new Color(.10f, .14f, .22f);
            var nightZenith = new Color(.04f, .07f, .17f);
            var daySky = new Color(.60f, .70f, .85f);
            var dayEquator = new Color(.48f, .54f, .46f);
            var dayGround = new Color(.25f, .22f, .18f);
            return new[]
            {
                Key(0, 52, 100, moon, .45f, .45f, nightSky, nightEquator, nightGround, nightFog, nightZenith),
                Key(4.5f, 52, 100, moon, .42f, .45f, nightSky, nightEquator, nightGround, nightFog, nightZenith),
                Key(5f, 36, 80, new Color(.80f, .72f, .90f), .25f, .35f, new Color(.38f, .40f, .56f), new Color(.30f, .31f, .38f), new Color(.13f, .12f, .14f), new Color(.24f, .27f, .35f), new Color(.10f, .15f, .32f)),
                Key(5.5f, 20, 58, new Color(1f, .64f, .50f), .70f, .55f, new Color(.52f, .50f, .62f), new Color(.44f, .40f, .42f), new Color(.18f, .15f, .14f), new Color(.38f, .37f, .42f), new Color(.22f, .30f, .52f)),
                Key(6f, 30, 60, new Color(1f, .75f, .54f), 1.25f, .70f, new Color(.66f, .66f, .76f), new Color(.56f, .53f, .48f), new Color(.25f, .21f, .18f), new Color(.46f, .45f, .46f), new Color(.36f, .50f, .74f)),
                Key(8f, 42, 70, new Color(1f, .83f, .64f), 1.2f, .75f, daySky, dayEquator, dayGround, new Color(.39f, .48f, .48f), new Color(.34f, .55f, .82f)),
                Key(12.5f, 58, 90, new Color(1f, .85f, .65f), 1.1f, .75f, daySky, dayEquator, dayGround, new Color(.38f, .48f, .47f), new Color(.30f, .52f, .84f)),
                Key(17f, 42, 110, new Color(1f, .82f, .60f), 1.2f, .75f, new Color(.62f, .68f, .80f), new Color(.50f, .53f, .45f), dayGround, new Color(.41f, .47f, .45f), new Color(.34f, .52f, .80f)),
                Key(19f, 28, 120, new Color(1f, .65f, .42f), 1.15f, .70f, new Color(.62f, .58f, .68f), new Color(.52f, .46f, .44f), new Color(.23f, .18f, .16f), new Color(.45f, .38f, .40f), new Color(.30f, .36f, .62f)),
                Key(20.25f, 14, 126, new Color(.95f, .48f, .40f), .55f, .50f, new Color(.42f, .39f, .56f), new Color(.32f, .29f, .36f), new Color(.12f, .11f, .13f), new Color(.25f, .24f, .32f), new Color(.14f, .18f, .38f)),
                Key(20.6f, 32, 112, new Color(.72f, .66f, .90f), .25f, .35f, new Color(.34f, .36f, .54f), new Color(.25f, .27f, .36f), new Color(.11f, .11f, .14f), new Color(.17f, .20f, .28f), new Color(.08f, .11f, .26f)),
                Key(21f, 52, 100, moon, .45f, .45f, nightSky, nightEquator, nightGround, nightFog, nightZenith),
            };
        }
    }
}
