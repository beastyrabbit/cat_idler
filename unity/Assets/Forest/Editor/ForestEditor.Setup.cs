using System;
using System.IO;
using System.Linq;
using IdleCatForest.Presentation;
using UnityEditor;
using UnityEditor.Build;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;
using UnityEngine.UIElements;
using Unity.Pipeline.Commands;

namespace IdleCatForest.Editor
{
    /// <summary>Player settings, shared resources and the entry scene.</summary>
    public static partial class ForestEditor
    {
        /// <summary>
        /// Idempotent: player settings and shared resources are (re)applied on every run, but the entry scene is
        /// never replaced. <c>Assets/Scenes/Forest.unity</c> is created only when it is missing; an existing scene
        /// is opened (or reused when already loaded) and gets the ForestGame object only if none exists in it.
        /// The scene is saved only when this run created it or added that object, so authored lighting, skybox
        /// or post-processing in the scene survive repeated runs.
        /// </summary>
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

            bool created = !File.Exists(ScenePath);
            Scene scene;
            if (created) scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            else
            {
                scene = SceneManager.GetSceneByPath(ScenePath);
                if (!scene.IsValid() || !scene.isLoaded) scene = EditorSceneManager.OpenScene(ScenePath, OpenSceneMode.Single);
            }
            bool added = false;
            if (!scene.GetRootGameObjects().Any(root => root.GetComponentInChildren<ForestGame>(true) != null))
            {
                var game = new GameObject("Forest authority and presentation");
                SceneManager.MoveGameObjectToScene(game, scene);
                game.AddComponent<ForestGame>();
                added = true;
            }
            if (created || added)
            {
                Directory.CreateDirectory("Assets/Scenes");
                if (!EditorSceneManager.SaveScene(scene, ScenePath)) throw new IOException("Could not save Forest scene.");
            }
            EditorBuildSettings.scenes = new[] { new EditorBuildSettingsScene(ScenePath, true) };
            AssetDatabase.SaveAssets();
            string sceneResult = created ? "Forest scene created" : added ? "ForestGame added to the existing Forest scene" : "Forest scene kept unchanged";
            string result = sceneResult + "; Apple Silicon IL2CPP player configured.";
            Debug.Log("forest_setup: " + result);
            return result;
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
    }
}
