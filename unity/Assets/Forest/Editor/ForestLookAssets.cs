using System;
using System.Linq;
using IdleCatForest.Presentation;
using UnityEditor;
using UnityEngine;
using Unity.Pipeline.Commands;

namespace IdleCatForest.Editor
{
    /// <summary>
    /// Creates <c>Assets/Resources/ForestLook.asset</c>: the <see cref="LookProfile"/> the lighting rig loads,
    /// with its Skybox/Panoramic material as a sub-asset (so the built-in shader ships in the player). An
    /// existing profile keeps its authored values; only a missing sky material is added.
    /// </summary>
    public static class ForestLookAssets
    {
        public const string AssetPath = "Assets/Resources/" + LookProfile.ResourceName + ".asset";
        private const string SkyShader = "Skybox/Panoramic";
        // Panoramic multiplies by _Tint × unity_ColorSpaceDouble; .504 makes the product 1.0 in linear space.
        private const float NeutralTint = .504f;

        [MenuItem("Forest/Create look profile")]
        [CliCommand("forest_look_assets", "Create the lighting look profile (ForestLook.asset) and its sky material", MainThreadRequired = true)]
        public static string Create() => Ensure(false);

        [MenuItem("Forest/Reset look profile to code defaults")]
        public static string Reset() => Ensure(true);

        internal static string Ensure(bool reset)
        {
            if (EditorApplication.isPlaying) throw new InvalidOperationException("Stop Play mode before editing the look profile.");
            var profile = AssetDatabase.LoadAssetAtPath<LookProfile>(AssetPath);
            bool created = profile == null, changed = created || reset;
            if (created)
            {
                profile = ScriptableObject.CreateInstance<LookProfile>();
                profile.name = LookProfile.ResourceName;
                AssetDatabase.CreateAsset(profile, AssetPath);
            }
            else if (reset)
            {
                var defaults = ScriptableObject.CreateInstance<LookProfile>();
                EditorUtility.CopySerialized(defaults, profile);
                UnityEngine.Object.DestroyImmediate(defaults);
                profile.name = LookProfile.ResourceName;
            }
            if (profile.Sky == null)
            {
                profile.Sky = AssetDatabase.LoadAllAssetsAtPath(AssetPath).OfType<Material>().FirstOrDefault();
                if (profile.Sky == null)
                {
                    var shader = Shader.Find(SkyShader);
                    if (shader == null) throw new InvalidOperationException("Required built-in shader missing: " + SkyShader);
                    var sky = new Material(shader) { name = "ForestSky" };
                    AssetDatabase.AddObjectToAsset(sky, profile);
                    profile.Sky = sky;
                }
                changed = true;
            }
            if (ConfigureSky(profile.Sky)) changed = true;
            if (changed) { EditorUtility.SetDirty(profile); AssetDatabase.SaveAssets(); }
            string result = (created ? "Look profile created" : reset ? "Look profile reset to code defaults" : changed ? "Look profile updated" : "Look profile kept unchanged")
                + " at " + AssetPath + "; day keys=" + profile.Day.Length + ".";
            Debug.Log("forest_look_assets: " + result);
            return result;
        }

        /// <summary>Latitude-longitude mapping, 360°, neutral tint and exposure. Returns whether anything changed.</summary>
        private static bool ConfigureSky(Material sky)
        {
            bool changed = false;
            changed |= Set(sky, "_Mapping", 1);
            changed |= Set(sky, "_ImageType", 0);
            changed |= Set(sky, "_MirrorOnBack", 0);
            changed |= Set(sky, "_Exposure", 1);
            changed |= Set(sky, "_Rotation", 0);
            var tint = new Color(NeutralTint, NeutralTint, NeutralTint, 1);
            if (sky.HasProperty("_Tint") && sky.GetColor("_Tint") != tint) { sky.SetColor("_Tint", tint); changed = true; }
            if (sky.IsKeywordEnabled("_MAPPING_6_FRAMES_LAYOUT")) { sky.DisableKeyword("_MAPPING_6_FRAMES_LAYOUT"); changed = true; }
            if (changed) EditorUtility.SetDirty(sky);
            return changed;
        }

        private static bool Set(Material material, string property, float value)
        {
            if (!material.HasProperty(property) || Mathf.Approximately(material.GetFloat(property), value)) return false;
            material.SetFloat(property, value);
            return true;
        }
    }
}
