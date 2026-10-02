using System;
using UnityEditor;
using UnityEngine;
using Unity.Pipeline.Commands;

namespace IdleCatForest.Editor
{
    /// <summary>
    /// Editor commands. Scene and player settings live in <c>ForestEditor.Setup.cs</c>, the player build in
    /// <c>ForestEditor.Build.cs</c> and runtime inspection in <c>ForestEditor.Inspect.cs</c>.
    /// </summary>
    public static partial class ForestEditor
    {
        public const string ScenePath = "Assets/Scenes/Forest.unity";
        private const string VisualSessionKey = "IdleCatForest.VisualSession";

        [CliCommand("forest_visual_start", "Start Play mode with asset refresh deferred for a stable visual check", MainThreadRequired = true)]
        public static string VisualStart()
        {
            if (!SessionState.GetBool(VisualSessionKey, false)) { AssetDatabase.DisallowAutoRefresh(); SessionState.SetBool(VisualSessionKey, true); }
            EditorApplication.isPlaying = true;
            return "Play mode requested; asset refresh deferred until forest_visual_stop.";
        }
        [CliCommand("forest_visual_stop", "End the visual check and restore normal asset refresh", MainThreadRequired = true)]
        public static string VisualStop()
        {
            EditorApplication.isPlaying = false;
            if (SessionState.GetBool(VisualSessionKey, false)) { AssetDatabase.AllowAutoRefresh(); SessionState.SetBool(VisualSessionKey, false); }
            return "Play mode stopped; asset refresh restored.";
        }
    }

    public sealed class ForestArtImporter : AssetPostprocessor
    {
        private void OnPreprocessModel()
        {
            if (!assetPath.StartsWith("Assets/Resources/ForestArt/", StringComparison.Ordinal)) return;
            var importer = (ModelImporter)assetImporter;
            importer.globalScale = 1; importer.useFileScale = true;
            importer.importCameras = false; importer.importLights = false; importer.importAnimation = false;
            importer.animationType = ModelImporterAnimationType.None;
            importer.materialImportMode = ModelImporterMaterialImportMode.ImportStandard;
            importer.importNormals = ModelImporterNormals.Import;
            importer.importTangents = ModelImporterTangents.None;
            importer.addCollider = false; importer.isReadable = false;
            importer.meshCompression = ModelImporterMeshCompression.Off;
        }
        private void OnPostprocessMaterial(Material material)
        {
            if (!assetPath.StartsWith("Assets/Resources/ForestArt/", StringComparison.Ordinal)) return;
            material.shader = Shader.Find("Standard"); material.SetFloat("_Glossiness", .08f); material.SetFloat("_Metallic", 0);
            material.enableInstancing = true;
        }
    }
}
