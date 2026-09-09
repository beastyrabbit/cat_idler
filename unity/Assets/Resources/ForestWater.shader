Shader "IdleCatForest/Water"
{
    Properties { _Color ("Tint", Color) = (1,1,1,1) }
    SubShader
    {
        Tags { "Queue"="Transparent" "RenderType"="Transparent" }
        Blend SrcAlpha OneMinusSrcAlpha
        ZWrite Off
        Pass
        {
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #include "UnityCG.cginc"
            struct Input { float4 vertex : POSITION; fixed4 color : COLOR; };
            struct Output { float4 position : SV_POSITION; fixed4 color : COLOR; float2 world : TEXCOORD0; };
            fixed4 _Color;
            Output vert(Input v)
            {
                Output o;
                o.position = UnityObjectToClipPos(v.vertex);
                o.world = mul(unity_ObjectToWorld, v.vertex).xz;
                o.color = v.color * _Color;
                return o;
            }
            fixed4 frag(Output i) : SV_Target
            {
                float ripple = sin(i.world.x * 3.1 + i.world.y * 2.3 + _Time.y * .7)
                    * sin(i.world.y * 4.7 - _Time.y * .5);
                return fixed4(i.color.rgb + ripple * .012, i.color.a);
            }
            ENDCG
        }
    }
}
