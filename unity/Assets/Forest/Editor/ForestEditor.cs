using System;
using System.IO;
using System.Linq;
using IdleCatForest.Presentation;
using UnityEditor;
using UnityEditor.Build;
using UnityEditor.Build.Reporting;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;
using UnityEngine.UIElements;
using Unity.Pipeline.Commands;

namespace IdleCatForest.Editor
{
    public static class ForestEditor
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

        [MenuItem("Forest/Prepare native project")]
        [CliCommand("forest_setup", "Configure the native Apple Silicon game and author its entry scene", MainThreadRequired = true)]
        public static string Setup()
        {
            if (EditorApplication.isPlaying) throw new InvalidOperationException("Stop Play mode before editing the entry scene.");
            PrepareResources();
            PlayerSettings.companyName = "Idle Cat Forest";
            PlayerSettings.productName = "Idle Cat Forest";
            PlayerSettings.bundleVersion = "1.0.0";
            PlayerSettings.SetApplicationIdentifier(NamedBuildTarget.Standalone, "dev.idlecatforest.game");
            PlayerSettings.SetArchitecture(NamedBuildTarget.Standalone, 1);
            PlayerSettings.SetScriptingBackend(NamedBuildTarget.Standalone, ScriptingImplementation.IL2CPP);
            PlayerSettings.defaultScreenWidth = 1440; PlayerSettings.defaultScreenHeight = 900;
            PlayerSettings.fullScreenMode = FullScreenMode.Windowed;
            PlayerSettings.resizableWindow = true; PlayerSettings.runInBackground = true;
            PlayerSettings.colorSpace = ColorSpace.Linear;
            PlayerSettings.usePlayerLog = true;
            EditorSettings.serializationMode = SerializationMode.ForceText;
            QualitySettings.antiAliasing = 4; QualitySettings.shadowResolution = ShadowResolution.Medium;
            QualitySettings.shadowDistance = 45;
            var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            new GameObject("Forest authority and presentation").AddComponent<ForestGame>();
            Directory.CreateDirectory("Assets/Scenes");
            if (!EditorSceneManager.SaveScene(scene, ScenePath)) throw new IOException("Could not save Forest scene.");
            EditorBuildSettings.scenes = new[] { new EditorBuildSettingsScene(ScenePath, true) };
            AssetDatabase.SaveAssets();
            return "Forest scene saved; Apple Silicon IL2CPP player configured.";
        }

        [MenuItem("Forest/Build Apple Silicon app")]
        [CliCommand("forest_build", "Build the configured native Apple Silicon application", MainThreadRequired = true)]
        public static string Build()
        {
            if (EditorApplication.isPlaying) throw new InvalidOperationException("Stop Play mode before building.");
            PrepareResources();
            if (!File.Exists(ScenePath)) Setup();
            var args = Environment.GetCommandLineArgs(); int at = Array.IndexOf(args, "-buildOutput");
            string output = at >= 0 && at + 1 < args.Length ? args[at + 1] : Path.GetFullPath("../artifacts/macos/Idle Cat Forest.app");
            Directory.CreateDirectory(Path.GetDirectoryName(output));
            PlayerSettings.SetArchitecture(NamedBuildTarget.Standalone, 1);
            PlayerSettings.SetScriptingBackend(NamedBuildTarget.Standalone, ScriptingImplementation.IL2CPP);
            var report = BuildPipeline.BuildPlayer(new BuildPlayerOptions { scenes = new[] { ScenePath }, locationPathName = output, target = BuildTarget.StandaloneOSX, options = BuildOptions.None });
            if (report.summary.result != BuildResult.Succeeded) throw new InvalidOperationException("Native build " + report.summary.result + ", errors=" + report.summary.totalErrors);
            return "Apple Silicon build succeeded; bytes=" + report.summary.totalSize + "; seconds=" + report.summary.totalTime.TotalSeconds.ToString("0.0");
        }

        private static void PrepareResources()
        {
            Directory.CreateDirectory("Assets/Resources");
            foreach (var entry in new[] { ("ForestSurface", "Standard"), ("ForestSelection", "Unlit/Color") })
            {
                string path = "Assets/Resources/" + entry.Item1 + ".mat";
                if (AssetDatabase.LoadAssetAtPath<Material>(path) != null) continue;
                var shader = Shader.Find(entry.Item2);
                if (shader == null) throw new InvalidOperationException("Required built-in shader missing: " + entry.Item2);
                AssetDatabase.CreateAsset(new Material(shader) { name = entry.Item1, enableInstancing = true }, path);
            }
            const string panelPath = "Assets/Resources/ForestPanel.asset";
            var panel = AssetDatabase.LoadAssetAtPath<PanelSettings>(panelPath);
            if (panel == null)
            {
                panel = ScriptableObject.CreateInstance<PanelSettings>();
                panel.scaleMode = PanelScaleMode.ScaleWithScreenSize; panel.referenceResolution = new Vector2Int(1440, 900); panel.match = .4f;
                panel.themeStyleSheet = Resources.Load<ThemeStyleSheet>("ForestTheme");
                AssetDatabase.CreateAsset(panel, panelPath);
            }
            AssetDatabase.SaveAssets();
        }

        [CliCommand("forest_inspect", "Inspect current simulated game, accounting and controller state without credentials", MainThreadRequired = true)]
        public static string Inspect()
        {
            var game = ForestGame.Instance;
            if (game?.Selected == null) return "Game is not running.";
            var village = game.Selected;
            return JsonUtility.ToJson(new Inspection
            {
                village = village.Name,
                cats = village.Cats.Count(c => c.Alive),
                hours = game.CurrentWorld.TimeSeconds / 3600,
                buildings = village.Buildings.Count,
                research = village.Research.Count,
                jobs = village.Jobs.Count(j => !j.Completed),
                reservations = game.CurrentWorld.Reservations.Count,
                controlledCat = game.View.ControlledCat?.Id ?? "",
                tickMilliseconds = game.TickMilliseconds,
                status = game.Status
            });
        }

        [CliCommand("forest_art_audit", "Inspect imported authored geometry for missing assets, axes and rendering cost", MainThreadRequired = true)]
        public static string ArtAudit()
        {
            var paths = AssetDatabase.FindAssets("t:Model", new[] { "Assets/Resources/ForestArt" }).Select(AssetDatabase.GUIDToAssetPath).ToArray();
            int triangles = 0, missingMaterials = 0, meshes = 0;
            foreach (var path in paths)
            {
                var model = AssetDatabase.LoadAssetAtPath<GameObject>(path);
                foreach (var filter in model.GetComponentsInChildren<MeshFilter>()) { meshes++; triangles += filter.sharedMesh.triangles.Length / 3; }
                foreach (var renderer in model.GetComponentsInChildren<Renderer>()) missingMaterials += renderer.sharedMaterials.Count(m => m == null || m.shader == null || m.shader.name.Contains("Error"));
            }
            if (paths.Length < 80 || missingMaterials > 0) throw new InvalidOperationException("Incomplete art import: models=" + paths.Length + ", missing materials=" + missingMaterials);
            return "Imported " + paths.Length + " authored models, " + meshes + " meshes, " + triangles + " unique-library triangles, no missing materials.";
        }
        [Serializable] private class Inspection { public string village, status, controlledCat; public int cats, buildings, research, jobs, reservations; public double hours, tickMilliseconds; }

        [CliCommand("forest_performance", "Report measured recent render frame and complete simulation step timings", MainThreadRequired = true)]
        public static string Performance()
        {
            var game = ForestGame.Instance; if (game?.Selected == null) throw new InvalidOperationException("Run the game first.");
            return JsonUtility.ToJson(game.MeasurePerformance());
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
