using System;
using System.IO;
using UnityEditor;
using UnityEditor.Build;
using UnityEditor.Build.Reporting;
using Unity.Pipeline.Commands;

namespace IdleCatForest.Editor
{
    /// <summary>The native Apple Silicon player build.</summary>
    public static partial class ForestEditor
    {
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
    }
}
