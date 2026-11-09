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
        [UnityTest] public IEnumerator StockpileCargoShowsReportedQuantityWithoutRevealingUncountedGoods()
        {
            var pile = game.Selected.Stockpiles.First(p => p.Kind == "storage");
            pile.Report.Clear(); pile.Report.Add(new Stack("logs", 1));
            pile.Goods.Clear(); pile.Goods.Add(new Stack("logs", 100));
            void Refresh() => typeof(ForestView).GetMethod("Reconcile", BindingFlags.Instance | BindingFlags.NonPublic).Invoke(game.View, null);
            int Bundles() => UnityEngine.Object.FindObjectsByType<Transform>(FindObjectsSortMode.None).Count(t => t.name.StartsWith("pilegoods:" + pile.Id + ":logs", StringComparison.Ordinal));
            Refresh(); yield return null;
            Assert.That(Bundles(), Is.EqualTo(1), "Unreported goods must not leak through the visual pile.");
            pile.Report[0].Amount = 100;
            Refresh(); yield return null;
            Assert.That(Bundles(), Is.EqualTo(3), "A large counted stock must look fuller than one log, with a bounded prop budget.");
            pile.Report[0].Amount = 1;
            Refresh(); yield return null;
            Assert.That(Bundles(), Is.EqualTo(1), "Spent reported stock must remove its extra bundles.");
            pile.Report.Clear();
            Refresh(); yield return null;
            Assert.That(Bundles(), Is.Zero, "An empty report must remove all resource bundles.");
        }
        [UnityTest] public IEnumerator StockpileDesignationShowsItsActualGroundFootprintAndKeepsItsAccessPoint()
        {
            var pile = game.Selected.Stockpiles.First(p => p.Kind == "storage");
            pile.Width = 4; pile.Depth = 2;
            typeof(ForestView).GetMethod("Reconcile", BindingFlags.Instance | BindingFlags.NonPublic).Invoke(game.View, null);
            yield return null;
            var visual = GameObject.Find("pile:" + pile.Id);
            Assert.That(visual.transform.position.x, Is.EqualTo(pile.Position.X).Within(.001f), "The visible crates must remain at the authoritative transfer point.");
            Assert.That(visual.transform.position.z, Is.EqualTo(pile.Position.Z).Within(.001f));
            var outline = visual.GetComponent<LineRenderer>();
            Assert.That(outline, Is.Not.Null, "The player must see the designated storage extent.");
            Assert.That(outline.loop, Is.True);
            var corners = Enumerable.Range(0, outline.positionCount).Select(outline.GetPosition).ToArray();
            Assert.That(corners.Max(p => p.x) - corners.Min(p => p.x), Is.EqualTo(4).Within(.001f));
            Assert.That(corners.Max(p => p.z) - corners.Min(p => p.z), Is.EqualTo(2).Within(.001f));
            Assert.That((corners.Max(p => p.x) + corners.Min(p => p.x)) * .5f, Is.EqualTo(pile.Position.X + 1.5f).Within(.001f));
            Assert.That((corners.Max(p => p.z) + corners.Min(p => p.z)) * .5f, Is.EqualTo(pile.Position.Z + .5f).Within(.001f));
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
        [UnityTest] public IEnumerator WorkplaceNavigationReturnsFromDungeonToTheBuildingsOwnLevel()
        {
            var floor = game.CurrentWorld.Dungeons.First().Floors.First();
            game.Selected.Known.Add(floor.Spawn);
            game.View.ViewLevel(floor.Level);
            Assert.That(game.View.RenderedLevel, Is.EqualTo(floor.Level));
            var building = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            game.UI.OpenPanel("Work");
            var button = game.GetComponent<UIDocument>().rootVisualElement.Query<Button>().ToList().First(b => b.text.StartsWith("wood cutter · ", StringComparison.Ordinal));
            using (var e = NavigationSubmitEvent.GetPooled()) { e.target = button; button.SendEvent(e); }
            Assert.That(game.View.SelectedBuilding, Is.SameAs(building));
            Assert.That(game.View.RenderedLevel, Is.EqualTo(building.Position.Level), "Inspecting a workplace must reveal the floor that contains it.");
            yield return null; yield return null;
            Assert.That(GameObject.Find("building:" + building.Id), Is.Not.Null, "The workplace must be rendered beside its inspector.");
            var center = new Vector3(building.Position.X + (building.Width - 1) * .5f, (float)game.CurrentWorld.WalkHeight(building.Position), building.Position.Z + (building.Depth - 1) * .5f);
            var screen = game.View.Camera.WorldToViewportPoint(center);
            Assert.That(screen.x, Is.GreaterThan(game.UI.WorldLeftFraction + .03f));
            Assert.That(screen.y, Is.InRange(.3f, .7f));
        }
        [UnityTest] public IEnumerator TetheredInspectorHidesSubjectsOnOtherFloors()
        {
            var root = game.GetComponent<UIDocument>().rootVisualElement;
            root.style.flexGrow = 0;
            root.style.width = root.style.maxWidth = 1000;
            root.style.height = root.style.maxHeight = 620;
            var floor = game.CurrentWorld.Dungeons.First().Floors.First();
            game.Selected.Known.Add(floor.Spawn);
            var cat = game.Selected.Cats.First(c => c.Alive);
            cat.Position = floor.Spawn; cat.X = floor.Spawn.X; cat.Z = floor.Spawn.Z;
            cat.Y = game.CurrentWorld.WalkHeight(floor.Spawn); cat.HasHeight = true;
            game.View.InspectCat(cat.Id);
            yield return null; yield return null; yield return null;
            var tether = game.GetComponent<UIDocument>().rootVisualElement.Q<VisualElement>(className: "tethered-inspector");
            Assert.That(tether.resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex));
            game.View.ViewLevel(0);
            yield return null; yield return null;
            Assert.That(tether.resolvedStyle.display, Is.EqualTo(DisplayStyle.None), "An underground cat must not leave a floating label over the surface.");
            Assert.That(game.View.SelectedCat, Is.SameAs(cat), "Changing floors keeps inspection available without misplacing its world label.");
            var building = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            game.View.InspectBuilding(building.Id);
            yield return null; yield return null;
            Assert.That(tether.resolvedStyle.display, Is.EqualTo(DisplayStyle.Flex));
            game.View.ViewLevel(floor.Level);
            yield return null; yield return null;
            Assert.That(tether.resolvedStyle.display, Is.EqualTo(DisplayStyle.None), "Surface workplaces must not leave labels over dungeon rooms.");
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
