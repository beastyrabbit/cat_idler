using System;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
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
            game.View.groundCenter = water.Position;
            int tiles = world.Tiles.Count;
            game.View.MakeGround(); yield return null;
            var chunk = TerrainMeshes().FirstOrDefault(f => Covers(f.sharedMesh.bounds, water.Position));
            Assert.That(chunk, Is.Not.Null, "A terrain chunk covers the lake");
            var bed = chunk.sharedMesh;
            var surfaceFilter = chunk.transform.Find("Water").GetComponent<MeshFilter>();
            var surface = surfaceFilter.sharedMesh;
            Assert.That(surface.bounds.max.y - bed.bounds.min.y, Is.GreaterThan(.5));
            Assert.That(surface.colors.Distinct().Count(), Is.GreaterThan(1), "Different water depths need distinct visible tints");
            Assert.That(surfaceFilter.GetComponent<MeshRenderer>().sharedMaterial.shader.name, Is.EqualTo("IdleCatForest/Water"), "The native build needs an included water shader");
            Assert.That(chunk.GetComponent<MeshRenderer>().sharedMaterial.shader.name, Is.EqualTo("IdleCatForest/Terrain"), "The native build needs the included terrain shader");
            Assert.That(world.Tiles.Count, Is.EqualTo(tiles));
            Assert.That(game.View.groundRoot.GetComponentsInChildren<MeshCollider>(true), Is.Empty, "Ground picking marches the heightfield; no physics meshes are cooked");
            Assert.That(Raycast(new Ray(new Vector3(water.Position.X + .1f, 40, water.Position.Z - .1f), Vector3.down), out var hit), Is.True);
            Assert.That(hit.y, Is.EqualTo((float)water.WaterSurface - .015f).Within(.02f), "A click on a lake lands on its surface");
        }
        [UnityTest] public IEnumerator UnexploredLandIsFoggedGroundNotAVoid()
        {
            var v = game.Selected; var world = game.CurrentWorld;
            int tiles = world.Tiles.Count;
            game.View.FocusVillage(); game.View.ZoomBy(-100);
            yield return null; yield return null;
            game.View.MakeGround(); yield return null;
            int edge = v.Known.Where(p => p.Level == 0 && p.Z == v.Center.Z).Max(p => p.X);
            Assert.That(TryTileVertex(v.Center, out _, out var home), Is.True, "The village centre is drawn");
            Assert.That(home.a, Is.EqualTo(1).Within(.001f), "Explored ground is fully lit");
            Assert.That(TryTileVertex(new Int2(edge + 2, v.Center.Z), out _, out var fade), Is.True, "Ground just past the explored edge is drawn");
            Assert.That(fade.a, Is.GreaterThan(.05f).And.LessThan(.95f), "Fog fades in over the three tiles past the explored edge");
            Assert.That(TryTileVertex(new Int2(edge + 12, v.Center.Z), out _, out var far), Is.True, "Unexplored land is drawn, not a void");
            Assert.That(far.a, Is.LessThan(.05f), "Unexplored land is fully fogged");
            Assert.That(world.Tiles.Count, Is.EqualTo(tiles), "Drawing unexplored land must never generate authoritative tiles");
            var camera = game.View.Camera;
            foreach (var corner in new[] { new Vector3(0, 0), new Vector3(1, 0), new Vector3(0, 1), new Vector3(1, 1) })
            {
                Assert.That(Raycast(camera.ViewportPointToRay(corner), out var point), Is.True, "Every screen corner shows ground at the widest zoom: " + corner);
                Assert.That(TryTileVertex(new Int2(Mathf.RoundToInt(point.x), Mathf.RoundToInt(point.z)), out _, out _), Is.True, "Built terrain reaches the screen corner " + corner + " at " + point);
            }
            var skirt = game.View.groundRoot.Find("World skirt");
            Assert.That(skirt != null && skirt.gameObject.activeSelf, Is.True, "A skirt plane backs the whole view");
            Assert.That(skirt.GetComponent<MeshRenderer>().shadowCastingMode, Is.EqualTo(UnityEngine.Rendering.ShadowCastingMode.Off));
        }
        [UnityTest] public IEnumerator GroundRebuildPreservesTerrainAndNeverGeneratesUnknownTiles()
        {
            game.View.enabled = false;
            var world = game.CurrentWorld;
            world.Tiles.Clear(); world.Dungeons.Clear(); world.Creatures.Clear(); game.Selected.Known.Clear();
            for (int z = -27; z <= 27; z++) for (int x = -27; x <= 27; x++)
            {
                var point = new Int2(x, z);
                world.Tiles.Add(new Tile { Position = point, Water = x == 1 && z == 0, Mountain = x == 2 && z == 0 });
                game.Selected.Known.Add(point);
            }
            game.Selected.Known.Add(new Int2(28, 0));
            int tileCount = world.Tiles.Count;
            double elapsed = 0;
            for (int i = 0; i < 3; i++)
            {
                game.View.groundCenter = new Int2(i * 8, 0);
                var timer = System.Diagnostics.Stopwatch.StartNew();
                game.View.MakeGround(); elapsed += timer.Elapsed.TotalMilliseconds;
                yield return null;
                Assert.That(TryTileVertex(new Int2(i * 8 - 38, 0), out _, out _) && TryTileVertex(new Int2(i * 8 + 38, 0), out _, out _), Is.True, "Ground reaches 38 tiles around the ground centre");
                Assert.That(WaterCovers(new Int2(1, 0)), Is.True, "Water tiles carry a water surface");
                Assert.That(TryTileVertex(new Int2(3, 0), out _, out var grass), Is.True);
                Assert.That(TryTileVertex(new Int2(2, 0), out _, out var rock), Is.True);
                Assert.That(Saturation(rock), Is.LessThan(Saturation(grass) * .85f), "Mountain ground is greyer than grass");
                Assert.That(TryTileVertex(new Int2(28, 0), out _, out var missing), Is.True);
                Assert.That(missing.a, Is.EqualTo(1).Within(.001f), "Known missing terrain stays visible without generating it.");
                Assert.That(TryTileVertex(new Int2(29, 0), out _, out var unknown) && unknown.a < 1, Is.True, "Unknown ground next to the explored edge is fogged");
                Assert.That(world.Tiles.Count, Is.EqualTo(tileCount), "Camera movement cannot generate authoritative terrain.");
            }
            TestContext.WriteLine("Three expanded-terrain rebuilds, milliseconds: " + elapsed.ToString("F3", CultureInfo.InvariantCulture));
        }
        [UnityTest] public IEnumerator RevealedDungeonFloorRendersOnStoneWithAShaftAtTheSurface()
        {
            var world = game.CurrentWorld; var site = world.Dungeons[0]; var floor = site.Floors[0];
            // Revealed by scouting but never walked: the cells are known, their tiles were never materialized.
            foreach (var p in floor.Cells) if (!game.Selected.Known.Contains(p)) game.Selected.Known.Add(p);
            int tiles = world.Tiles.Count;
            game.View.ViewLevel(-1);
            yield return new WaitForSecondsRealtime(.3f); yield return null;
            Assert.That(game.View.RenderedLevel, Is.EqualTo(-1));
            game.View.MakeGround(); game.View.ForceReconcile(); yield return null;
            var scene = UnityEngine.Object.FindObjectsByType<Transform>(FindObjectsSortMode.None);
            Assert.That(scene.Count(t => t.name.StartsWith("floor:", StringComparison.Ordinal)), Is.EqualTo(floor.Cells.Count), "Every revealed floor cell gets a floor slab");
            Assert.That(scene.Any(t => t.name.StartsWith("cavewall:", StringComparison.Ordinal)), Is.True, "Revealed rooms are walled");
            foreach (var p in floor.Cells)
            {
                Assert.That(TryTileVertex(p, out var position, out var color), Is.True, "Ground under the floor at " + p);
                Assert.That(color.g, Is.LessThanOrEqualTo(Mathf.Max(color.r, color.b) + .005f), "No grass underground at " + p);
                Assert.That(position.y, Is.LessThan((float)floor.Elevation), "The ground sits under the floor slab at " + p);
            }
            Assert.That(world.Tiles.Count, Is.EqualTo(tiles), "Looking underground must not materialize tiles");
            game.View.ViewLevel(0);
            yield return new WaitForSecondsRealtime(.3f); yield return null;
            game.View.MakeGround(); yield return null;
            var opening = site.Stairs[0].Opening[1];
            float rim = (float)world.WalkHeight(site.Entrance);
            var shaft = TerrainMeshes().SelectMany(f => f.sharedMesh.vertices).Where(p => Mathf.Abs(p.x - opening.X) <= .51f && Mathf.Abs(p.z - opening.Z) <= .51f).ToArray();
            Assert.That(shaft.Any(p => p.y < rim - 3), Is.True, "The stair opening is a dark shaft into the floor below, not a hole into the void");
        }
        [UnityTest] public IEnumerator PanningFortyTilesStaysWithinTheFrameBudget()
        {
            var view = game.View;
            var focusField = typeof(ForestView).GetField("focus", BindingFlags.Instance | BindingFlags.NonPublic);
            Assert.That(focusField, Is.Not.Null, "The pan test drives the management camera focus");
            var work = typeof(ForestView).GetProperty("TerrainWorkMilliseconds", BindingFlags.Instance | BindingFlags.NonPublic | BindingFlags.Public);
            view.FocusVillage();
            for (int i = 0; i < 10; i++) yield return null;
            var start = (Vector3)focusField.GetValue(view);
            var frames = new List<float>();
            var terrain = new List<float>();
            foreach (float zoom in new[] { 0f, -100f })
            {
                if (zoom != 0) { view.ZoomBy(zoom); for (int i = 0; i < 10; i++) yield return null; }
                for (int i = 0; i <= 120; i++)
                {
                    float travelled = 40f * i / 120;
                    focusField.SetValue(view, start + new Vector3(travelled, 0, travelled * (zoom == 0 ? .5f : -.5f)));
                    int collections = GC.CollectionCount(0);
                    yield return null;
                    if (i == 0) continue;
                    frames.Add(Time.unscaledDeltaTime * 1000);
                    terrain.Add(work == null ? 0 : (float)work.GetValue(view));
                    if (frames[frames.Count - 1] > 28) TestContext.WriteLine("slow frame " + frames.Count + ": " + frames[frames.Count - 1].ToString("F1", CultureInfo.InvariantCulture) + " ms, terrain " + terrain[terrain.Count - 1].ToString("F2", CultureInfo.InvariantCulture) + " ms, gc " + (GC.CollectionCount(0) - collections));
                }
            }
            string report = "frame max " + frames.Max().ToString("F1", CultureInfo.InvariantCulture) + " ms, p95 " + frames.OrderBy(f => f).ElementAt((int)(frames.Count * .95f)).ToString("F1", CultureInfo.InvariantCulture) + " ms; terrain work max " + terrain.Max().ToString("F2", CultureInfo.InvariantCulture) + " ms";
            TestContext.WriteLine("Panning 40 tiles at default and widest zoom: " + report);
            Assert.That(work, Is.Not.Null, "The view reports its terrain work per frame");
            Assert.That(terrain.Max(), Is.LessThan(8f), "Terrain streaming stays within its frame budget: " + report);
            Assert.That(frames.Max(), Is.LessThan(33f), "No frame over 33 ms while panning: " + report);
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

        private IEnumerable<MeshFilter> TerrainMeshes() =>
            game.View.groundRoot.GetComponentsInChildren<MeshFilter>().Where(f => f.name.StartsWith("Terrain ", StringComparison.Ordinal) && f.GetComponent<MeshRenderer>().enabled);
        private static bool Covers(Bounds b, Int2 p) => p.X >= b.min.x && p.X <= b.max.x && p.Z >= b.min.z && p.Z <= b.max.z;
        /// <summary>The centre vertex of a tile's ground fan: its rendered height and vertex colour (alpha = explored).</summary>
        private bool TryTileVertex(Int2 p, out Vector3 position, out Color color)
        {
            foreach (var filter in TerrainMeshes())
            {
                var mesh = filter.sharedMesh; if (!Covers(mesh.bounds, p)) continue;
                var vertices = mesh.vertices; var colors = mesh.colors;
                for (int i = 0; i < vertices.Length && i < colors.Length; i++)
                    if (vertices[i].x == p.X && vertices[i].z == p.Z) { position = vertices[i]; color = colors[i]; return true; }
            }
            position = default; color = default; return false;
        }
        private bool WaterCovers(Int2 p) => game.View.groundRoot.GetComponentsInChildren<MeshFilter>().Any(f => f.name == "Water" && f.GetComponent<MeshRenderer>().enabled && f.sharedMesh.vertexCount > 0 && Covers(f.sharedMesh.bounds, p));
        private static float Saturation(Color c) => Mathf.Max(c.r, Mathf.Max(c.g, c.b)) - Mathf.Min(c.r, Mathf.Min(c.g, c.b));
        private bool Raycast(Ray ray, out Vector3 point)
        {
            var method = typeof(ForestView).GetMethod("TerrainRaycast", BindingFlags.Instance | BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic);
            var args = new object[] { ray, null };
            bool hit = (bool)method.Invoke(method.IsStatic ? null : game.View, args);
            point = (Vector3)args[1];
            return hit;
        }
    }
}
