using System;
using System.Linq;
using IdleCatForest.Presentation;
using UnityEditor;
using UnityEngine;
using Unity.Pipeline.Commands;

namespace IdleCatForest.Editor
{
    /// <summary>Read-only inspection of the running game, its performance and the imported art.</summary>
    public static partial class ForestEditor
    {
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
}
