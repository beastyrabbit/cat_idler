// Ground for Idle Cat Forest (built-in render pipeline).
// Vertex colour carries the linear albedo; vertex alpha carries exploration (1 explored, 0 deep fog of war).
// Explored ground is lit (Lambert, shadows, fog) with a world-space grass noise; unexplored ground is drawn as a
// darkened, desaturated, relief-shaded version of itself that leans toward the skirt colour and does not depend on
// the sun's colour, so it reads as fog of war at any time of day. With _Skirt = 1 the same shader draws the flat
// background skirt in _SkirtColor.
Shader "IdleCatForest/Terrain"
{
    Properties
    {
        _SkirtColor ("Skirt and fog-of-war colour", Color) = (0.085, 0.125, 0.118, 1)
        _FogBrightness ("Unexplored brightness", Range(0, 1)) = 0.35
        _FogDesaturation ("Unexplored desaturation", Range(0, 1)) = 0.5
        _FogToSkirt ("Unexplored lean toward the skirt", Range(0, 1)) = 0.25
        _GrassNoise ("Grass noise strength", Range(0, 0.3)) = 0.09
        _GrassScale ("Grass noise scale", Float) = 0.42
        [Toggle] _Skirt ("Draw as the flat skirt", Float) = 0
        [Enum(Off, 0, On, 1)] _ZWrite ("Depth write", Float) = 1
    }
    SubShader
    {
        Tags { "RenderType" = "Opaque" "Queue" = "Geometry" }
        LOD 200
        ZWrite [_ZWrite]

        CGPROGRAM
        #pragma surface surf Lambert vertex:vert finalcolor:fogOfWar fullforwardshadows addshadow nofog exclude_path:deferred exclude_path:prepass nolightmap nodynlightmap nodirlightmap nometa
        #pragma multi_compile_fog
        #pragma target 3.0

        fixed4 _SkirtColor;
        half _FogBrightness, _FogDesaturation, _FogToSkirt, _GrassNoise, _GrassScale, _Skirt;

        struct Input
        {
            float4 color : COLOR;
            float3 worldPos;
            float3 worldNormal;
            float fogCoord;
        };

        float hash12(float2 p)
        {
            float3 p3 = frac(float3(p.xyx) * 0.1031);
            p3 += dot(p3, p3.yzx + 33.33);
            return frac((p3.x + p3.y) * p3.z);
        }

        float valueNoise(float2 p)
        {
            float2 i = floor(p);
            float2 f = frac(p);
            float2 u = f * f * (3 - 2 * f);
            return lerp(lerp(hash12(i), hash12(i + float2(1, 0)), u.x), lerp(hash12(i + float2(0, 1)), hash12(i + float2(1, 1)), u.x), u.y);
        }

        void vert(inout appdata_full v, out Input o)
        {
            UNITY_INITIALIZE_OUTPUT(Input, o);
            o.color = v.color;
            float4 clip = UnityObjectToClipPos(v.vertex);
            UNITY_TRANSFER_FOG(o, clip);
        }

        void surf(Input IN, inout SurfaceOutput o)
        {
            half3 albedo = IN.color.rgb;
            // Green-dominant ground reads as grass and gets the strongest variation; soil and stone a little.
            half grass = saturate((albedo.g - max(albedo.r, albedo.b)) * 10);
            float2 p = IN.worldPos.xz * _GrassScale;
            half broad = valueNoise(p) * 0.65 + valueNoise(p * 3.7 + 17.3) * 0.35 - 0.5;
            half fine = valueNoise(IN.worldPos.xz * 6.3 + 41.7) - 0.5;
            half strength = _GrassNoise * (0.35 + 0.65 * grass);
            albedo *= 1 + broad * 2 * strength + fine * strength;
            // Sun-dried patches lean warm, damp ones cool.
            albedo.r += grass * broad * strength * 0.12;
            o.Albedo = saturate(albedo);
            o.Alpha = 1;
        }

        void fogOfWar(Input IN, SurfaceOutput o, inout fixed4 color)
        {
            half fog = _Skirt > 0.5 ? 1 : saturate(1 - IN.color.a);
        #ifdef UNITY_PASS_FORWARDADD
            color.rgb *= 1 - fog;
            UNITY_APPLY_FOG_COLOR(IN.fogCoord, color, fixed4(0, 0, 0, 0));
        #else
            half luminance = dot(o.Albedo, half3(0.2126, 0.7152, 0.0722));
            half3 grey = lerp(o.Albedo, luminance.xxx, _FogDesaturation) * _FogBrightness;
            half relief = 0.72 + 0.4 * saturate(dot(normalize(IN.worldNormal), normalize(half3(-0.35, 0.85, -0.4))));
            half daylight = saturate(dot(_LightColor0.rgb, half3(0.3, 0.59, 0.11)) * 0.6 + 0.4);
            half3 fogged = lerp(grey * relief * daylight, _SkirtColor.rgb, _FogToSkirt);
            if (_Skirt > 0.5) fogged = _SkirtColor.rgb;
            color.rgb = lerp(color.rgb, fogged, fog);
            UNITY_APPLY_FOG(IN.fogCoord, color);
        #endif
        }
        ENDCG
    }
    FallBack Off
}
