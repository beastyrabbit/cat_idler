using System;
using System.Collections;
using System.IO;
using System.Linq;
using System.Reflection;
using IdleCatForest.Presentation;
using IdleCatForest.Simulation;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.UIElements;
using Stack = IdleCatForest.Simulation.Stack;

namespace IdleCatForest.Tests
{
    public class ForestWorldTests
    {
        private ForestGame game;
        private string directory;
        [UnitySetUp] public IEnumerator Open()
        {
            Assert.That(ForestGame.Instance, Is.Null);
            directory = Path.Combine(Path.GetTempPath(), "forest-world-ui-" + Guid.NewGuid().ToString("N"));
            game = new GameObject("Isolated generated world").AddComponent<ForestGame>();
            game.InitialSavePath = Path.Combine(directory, "world.json");
            yield return null; game.SetSpeed(0); game.UI.ClosePanel();
        }
        [UnityTearDown] public IEnumerator Close()
        {
            if (game != null) UnityEngine.Object.Destroy(game.gameObject); yield return null;
            if (Directory.Exists(directory)) Directory.Delete(directory, true);
        }
        [UnityTest] public IEnumerator WorldPanelDispatchesPhysicalExplorerAndShowsDiscoveredFloor()
        {
            var cat = game.Selected.Cats[0]; cat.Skills.Add(new Stack("fight", 12));
            game.UI.OpenPanel("World");
            var root = game.GetComponent<UIDocument>().rootVisualElement;
            Assert.That(root.Query<Button>().ToList().Any(b => b.text == "Show entrance"), Is.True);
            var button = root.Query<Button>().ToList().First(b => b.text == "Explore with selected cat");
            using (var e = new NavigationSubmitEvent()) { e.target = button; button.SendEvent(e); }
            yield return null;
            Assert.That(cat.DungeonId, Is.Not.Empty, game.LastAction);
            var original = cat.Position;
            for (int i = 0; i < 2000 && cat.Position.Level == 0; i++) game.CurrentWorld.Step(.05);
            Assert.That(cat.Position.Level, Is.EqualTo(-1), cat.BlockedReason);
            Assert.That(cat.Y, Is.LessThan(-3.9)); Assert.That(cat.Position, Is.Not.EqualTo(original));
            game.View.InspectCat(cat.Id); yield return new WaitForSecondsRealtime(.25f); yield return null;
            Assert.That(game.View.RenderedLevel, Is.EqualTo(-1));
            var scene = UnityEngine.Object.FindObjectsByType<Transform>(FindObjectsSortMode.None);
            Assert.That(scene.Any(t => t.name.StartsWith("floor:", StringComparison.Ordinal)), Is.True);
            Assert.That(scene.Where(t => t.name.StartsWith("building:", StringComparison.Ordinal)).Select(t => t.name).ToArray(), Is.Empty, "Surface houses must not hide the dungeon floor");
            Assert.That(scene.Any(t => t.name.StartsWith("MISSING ASSET", StringComparison.Ordinal)), Is.False);
            var visual = scene.Single(t => t.name == "cat:" + cat.Id);
            cat.X += .1; cat.Y += .1;
            yield return null;
            Assert.That(Math.Abs(visual.forward.y), Is.LessThan(.001), "Walking a slope must leave the cat upright");
            cat.X -= .1; cat.Y -= .1;
            yield return null;
            var selection = scene.Single(t => t.name == "Selected place").GetComponent<LineRenderer>();
            Assert.That(selection.GetPosition(0).y, Is.EqualTo((float)cat.Y + .08f).Within(.01f), "The selection must sit on the cat's actual floor");
            game.View.ViewLevel(0); yield return null; yield return null;
            Assert.That(game.View.RenderedLevel, Is.EqualTo(0), "An inspected explorer must not override an explicit surface view");
        }
        [UnityTest] public IEnumerator TerrainMeshesHaveDepthAndCameraCannotGenerateTiles()
        {
            var world = game.CurrentWorld;
            Tile water = null;
            for (int z = -96; z <= 96 && water == null; z++) for (int x = -96; x <= 96; x++) { var t = world.GenerateSurfaceTile(new Int2(x, z)); if (t.Water && t.WaterDepth > .5) { water = world.TileAt(t.Position); break; } }
            Assert.That(water, Is.Not.Null);
            game.Selected.Known.Add(water.Position);
            for (int x = -4; x <= 4; x++) for (int z = -4; z <= 4; z++)
            {
                var p = new Int2(water.Position.X + x, water.Position.Z + z);
                world.TileAt(p); if (!game.Selected.Known.Contains(p)) game.Selected.Known.Add(p);
            }
            var view = typeof(ForestView); const BindingFlags flags = BindingFlags.Instance | BindingFlags.NonPublic;
            view.GetField("groundCenter", flags).SetValue(game.View, water.Position);
            int tiles = world.Tiles.Count;
            view.GetMethod("MakeGround", flags).Invoke(game.View, null); yield return null;
            var terrain = (Transform)view.GetField("groundRoot", flags).GetValue(game.View);
            var bed = terrain.Find("Ground 11").GetComponent<MeshFilter>().sharedMesh;
            var surface = terrain.Find("Ground 1").GetComponent<MeshFilter>().sharedMesh;
            Assert.That(surface.bounds.max.y - bed.bounds.min.y, Is.GreaterThan(.5));
            Assert.That(surface.colors.Distinct().Count(), Is.GreaterThan(1), "Different water depths need distinct visible tints");
            Assert.That(terrain.Find("Ground 1").GetComponent<MeshRenderer>().sharedMaterial.shader.name, Is.EqualTo("IdleCatForest/Water"), "The native build needs an included water shader");
            Assert.That(world.Tiles.Count, Is.EqualTo(tiles));
            Assert.That(terrain.GetComponentsInChildren<MeshCollider>().Length, Is.LessThanOrEqualTo(12));
        }
        [UnityTest] public IEnumerator DirectDungeonCameraCutsAwayForegroundWalls()
        {
            var w = game.CurrentWorld; var floor = w.Dungeons[0].Floors[0]; var cat = game.Selected.Cats[0];
            cat.Position = floor.Spawn; cat.X = floor.Spawn.X; cat.Z = floor.Spawn.Z; cat.Y = floor.Elevation; cat.HasHeight = true;
            foreach (var p in floor.Cells) { w.TileAt(p); game.Selected.Known.Add(p); }
            game.View.InspectCat(cat.Id); game.View.EnterSelectedCat();
            yield return new WaitForSecondsRealtime(1f); yield return null;
            var walls = UnityEngine.Object.FindObjectsByType<Transform>(FindObjectsSortMode.None).Where(t => t.name.StartsWith("cavewall:", StringComparison.Ordinal)).ToArray();
            var position = new Vector3((float)cat.X, (float)cat.Y, (float)cat.Z);
            var offset = game.View.Camera.transform.position - position; offset.y = 0;
            var front = walls.Where(t => Vector3.Dot(t.position - position, offset) > 0).ToArray();
            var back = walls.Where(t => Vector3.Dot(t.position - position, offset) < 0).ToArray();
            Assert.That(front, Is.Not.Empty); Assert.That(back, Is.Not.Empty);
            Assert.That(front.All(t => t.localScale.y < .3f), Is.True, "Foreground walls must not block the controlled cat: " + string.Join("; ", front.Where(t => t.localScale.y >= .3f).Select(t => t.name + " at " + t.position + " scale " + t.localScale)) + " camera offset " + offset);
            Assert.That(back.All(t => t.localScale.y == 1), Is.True, "Rear walls retain the dungeon's full height");
        }
    }
}
