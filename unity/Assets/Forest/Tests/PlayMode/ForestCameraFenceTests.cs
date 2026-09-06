using System;
using System.Collections;
using System.IO;
using System.Linq;
using System.Reflection;
using IdleCatForest.Presentation;
using IdleCatForest.Simulation;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.LowLevel;
using UnityEngine.TestTools;
using UnityEngine.UIElements;

namespace IdleCatForest.Tests
{
    public class ForestCameraFenceTests
    {
        private ForestGame game;
        private string directory;
        private Mouse mouse;
        private Keyboard keyboard;
        private const BindingFlags Private = BindingFlags.NonPublic | BindingFlags.Instance;

        [UnitySetUp]
        public IEnumerator StartIsolatedWorld()
        {
            Assert.That(ForestGame.Instance, Is.Null, "Never replace the player's running world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-camera-fence-" + Guid.NewGuid().ToString("N"));
            game = new GameObject("Isolated camera and fence test").AddComponent<ForestGame>();
            game.InitialSavePath = Path.Combine(directory, "world.json");
            yield return null;
            game.SetSpeed(0); game.UI.ClosePanel();
            mouse = InputSystem.AddDevice<Mouse>(); keyboard = InputSystem.AddDevice<Keyboard>();
            mouse.MakeCurrent(); keyboard.MakeCurrent();
        }

        [UnityTearDown]
        public IEnumerator CloseIsolatedWorld()
        {
            if (mouse != null) InputSystem.RemoveDevice(mouse);
            if (keyboard != null) InputSystem.RemoveDevice(keyboard);
            if (game != null) UnityEngine.Object.Destroy(game.gameObject);
            yield return null;
            if (Directory.Exists(directory)) Directory.Delete(directory, true);
        }

        [Test]
        public void OneNormalizedWheelStepChangesScaleImmediatelyAroundThePointer()
        {
            game.View.enabled = false;
            // This focused input check is independent of the batch Editor's Game-view viewport.
            foreach (var element in game.GetComponent<UIDocument>().rootVisualElement.Query<VisualElement>().ToList()) element.RemoveFromClassList("interactive");
            var cursor = new Vector2(Screen.width * .7f, Screen.height * .5f);
            var before = GroundAt(cursor); float initial = game.View.Camera.orthographicSize;
            InputSystem.QueueStateEvent(mouse, new MouseState { position = cursor, scroll = new Vector2(0, 1) });
            InputSystem.Update();
            Assert.That(mouse.scroll.ReadValue().y, Is.EqualTo(1f), "Exercise the normalized Input System value.");
            Assert.That(game.UI.PointerOverPanel, Is.False, "The test pointer must be over the game world.");
            typeof(ForestView).GetMethod("ReadInput", Private).Invoke(game.View, null);
            typeof(ForestView).GetMethod("UpdateCamera", Private).Invoke(game.View, null);
            Assert.That(game.View.Camera.orthographicSize, Is.LessThan(initial * .95f), "One normalized wheel step should visibly zoom on the next frame.");
            Assert.That(Vector3.Distance(before, GroundAt(cursor)), Is.LessThan(.03f), "The world point under the cursor must stay under the cursor while zooming.");
        }

        [Test]
        public void ZoomButtonsAreReversibleAndRespectCloseAndWideLimits()
        {
            var zoomBy = typeof(ForestView).GetMethod("ZoomBy");
            Assert.That(zoomBy, Is.Not.Null, "Management needs an accessible alternative to mouse-wheel input.");
            var zoom = typeof(ForestView).GetField("zoom", Private);
            float initial = (float)zoom.GetValue(game.View);
            zoomBy.Invoke(game.View, new object[] { 1f });
            Assert.That((float)zoom.GetValue(game.View), Is.LessThan(initial * .95f));
            zoomBy.Invoke(game.View, new object[] { -1f });
            Assert.That((float)zoom.GetValue(game.View), Is.EqualTo(initial).Within(.001f));
            zoomBy.Invoke(game.View, new object[] { 1000f });
            Assert.That((float)zoom.GetValue(game.View), Is.InRange(3f, 5f));
            zoomBy.Invoke(game.View, new object[] { -1000f });
            Assert.That((float)zoom.GetValue(game.View), Is.InRange(30f, 42f));
        }

        [Test]
        public void FootTrafficDoesNotTurnIntoPavingOrHideUncollectedResources()
        {
            var tileAsset = typeof(ForestView).GetMethod("TileAsset", Private);
            var tile = new Tile { Dirt = true, Resource = "logs", Amount = 200 };
            Assert.That(tileAsset.Invoke(game.View, new object[] { tile }), Is.EqualTo("tree_oak"), "Walking past a tree must not replace it with a paved road.");
            tile.Resource = ""; tile.Amount = 0;
            Assert.That(tileAsset.Invoke(game.View, new object[] { tile }), Is.EqualTo(""), "Worn earth belongs to the terrain. Only built roads get stone paving.");
            tile.Road = true;
            Assert.That(tileAsset.Invoke(game.View, new object[] { tile }), Is.EqualTo("road"));
        }

        [UnityTest]
        public IEnumerator InspectedCatStaysInTheVisibleWorldBesideItsDrawer()
        {
            var cat = game.Selected.Cats.First(c => c.Alive);
            game.View.InspectCat(cat.Id); game.UI.OpenPanel("Inspect");
            yield return null; yield return null; yield return null;
            var pixel = game.View.Camera.WorldToViewportPoint(new Vector3((float)cat.X, 0, (float)cat.Z));
            Assert.That(pixel.x, Is.GreaterThan(game.UI.WorldLeftFraction + .04f));
            Assert.That(pixel.x, Is.LessThan(.98f));
            float initial = game.View.ManagementZoom;
            game.View.EnterSelectedCat(); yield return null;
            Assert.That(game.View.DirectControl, Is.True);
            game.View.ZoomBy(1); game.View.LeaveCat(); yield return null;
            Assert.That(game.View.Camera.orthographic, Is.True);
            Assert.That(game.View.ManagementZoom, Is.EqualTo(initial).Within(.001f), "Management zoom survives the same-cat control roundtrip.");
        }

        [UnityTest]
        public IEnumerator LegacyBuildingsKeepTheirOrientationWithoutInventingADoor()
        {
            game.View.enabled = false;
            var building = game.Selected.Buildings.First(b => b.Kind == "den");
            building.HasEntrance = false;
            typeof(ForestView).GetMethod("Reconcile", Private).Invoke(game.View, null);
            yield return null;
            Assert.That(GameObject.Find("entrance:" + building.Id), Is.Null, "Legacy buildings have no stored doorway; presentation must not guess one by searching the road network every frame.");
        }

        [Test]
        public void ControlHeartbeatPreservesThePlayersLastActionFeedback()
        {
            var cat = game.Selected.Cats.First(c => c.Alive);
            Assert.That(game.Send(new GameAction { Kind = "EnterCatControl", CatId = cat.Id }).Result.Success, Is.True);
            string feedback = game.LastAction;
            Assert.That(game.Send(new GameAction { Kind = "KeepCatControl", CatId = cat.Id }).Result.Success, Is.True);
            Assert.That(game.LastAction, Is.EqualTo(feedback), "An automatic lease renewal must not replace the player's action result every second.");
        }

        [UnityTest]
        public IEnumerator ClickingAWorldBuildingFramesItBesideTheInspector()
        {
            game.View.enabled = false;
            game.Selected.Cats.Clear();
            foreach (var element in game.GetComponent<UIDocument>().rootVisualElement.Query<VisualElement>().ToList()) element.RemoveFromClassList("interactive");
            var building = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            var center = new Vector3(building.Position.X + (building.Width - 1) * .5f, 0, building.Position.Z + (building.Depth - 1) * .5f);
            typeof(ForestView).GetMethod("UpdateCamera", Private).Invoke(game.View, null);
            Vector2 cursor = game.View.Camera.WorldToScreenPoint(center);
            // Target the player's Dynamic buffer; a batch Editor can default to its separate Editor buffer.
            InputState.Change(mouse.position, cursor, InputUpdateType.Dynamic);
            Assert.That(mouse.leftButton.wasPressedThisFrame, Is.False);
            using (StateEvent.From(mouse, out var click))
            {
                mouse.leftButton.WriteValueIntoEvent(1f, click);
                InputState.Change(mouse, click, InputUpdateType.Dynamic);
            }
            Assert.That(mouse.leftButton.wasPressedThisFrame, Is.True, "The synthetic mouse must deliver a fresh click.");
            Assert.That(game.UI.PointerOverPanel || game.UI.TextInputFocused || game.UI.HasPlacement, Is.False, "The click must reach world selection.");
            typeof(ForestView).GetMethod("ReadInput", Private).Invoke(game.View, null);
            Assert.That(game.View.SelectedBuilding, Is.SameAs(building), "Clicked tile: " + game.View.CursorTile + "; intended center: " + center);
            Assert.That(game.UI.OpenSection, Is.EqualTo("Inspect"));
            yield return null; yield return null;
            typeof(ForestView).GetMethod("UpdateCamera", Private).Invoke(game.View, null);
            var screen = game.View.Camera.WorldToViewportPoint(center);
            Assert.That(screen.x, Is.GreaterThan(game.UI.WorldLeftFraction + .03f), "A world click must keep the inspected building clear of the drawer.");
            Assert.That(screen.y, Is.InRange(.3f, .7f));
        }

        [UnityTest]
        public IEnumerator CaravanRenderingKeepsBlockedAndResumedSnapshotsOnTheirAuthoritativeTile()
        {
            game.View.enabled = false;
            var world = game.CurrentWorld; var village = game.Selected;
            var before = new Int2(11, 0); var next = new Int2(11, 1);
            village.BoundaryEdges.Add(new BoundaryEdge { From = before, To = next });
            foreach (var p in new[] { before, next, new Int2(11, 2) })
            {
                var tile = world.TileAt(p); tile.Wall = tile.Water = tile.Mountain = false;
                if (!village.Known.Contains(p)) village.Known.Add(p);
            }
            // Isolated snapshots from an accepted route: the simulation regressions exercise
            // its public expansion and recovery. The renderer must not predict its next step.
            var trade = new TradeOffer { Id = "rendered-blocked-caravan", FromVillageId = village.Id, ToVillageId = "snapshot-destination", Status = "outbound", X = before.X, Z = before.Z, Offered = new IdleCatForest.Simulation.Stack("logs", 8), Requested = new IdleCatForest.Simulation.Stack("stone", 5) };
            trade.Path.Add(next); trade.Path.Add(new Int2(11, 2)); world.TradeOffers.Add(trade);
            var reconcile = typeof(ForestView).GetMethod("Reconcile", Private);
            Assert.That(world.Crossable(before, next), Is.False);
            reconcile.Invoke(game.View, null); yield return null;
            var cart = GameObject.Find("trade:" + trade.Id);
            Assert.That(cart, Is.Not.Null);
            Assert.That(cart.transform.position, Is.EqualTo(new Vector3(11, 0, 0)), "A stopped caravan must remain on the near side of its blocked edge.");
            trade.Status = "returning"; reconcile.Invoke(game.View, null);
            Assert.That(cart.transform.position, Is.EqualTo(new Vector3(11, 0, 0)), "Returning cargo uses its actual position too.");
            village.BoundaryEdges.Clear(); trade.X = next.X; trade.Z = next.Z; trade.PathIndex = 1;
            reconcile.Invoke(game.View, null); yield return null;
            Assert.That(GameObject.Find("trade:" + trade.Id), Is.SameAs(cart));
            Assert.That(cart.transform.position, Is.EqualTo(new Vector3(11, 0, 1)), "The next snapshot moves the same cart to the reached tile, not its following waypoint.");
            Assert.That(trade.Offered.Amount, Is.EqualTo(8)); Assert.That(trade.Requested.Amount, Is.EqualTo(5));
        }

        [UnityTest]
        public IEnumerator FenceRailsMeetAtBothArmsOfACornerAndGatesStayOpen()
        {
            game.View.enabled = false;
            game.CurrentWorld.Tiles.Clear(); game.Selected.Known.Clear(); game.Selected.BoundaryEdges.Clear();
            game.Selected.Buildings.Clear(); game.Selected.Stockpiles.Clear(); game.Selected.Farms.Clear(); game.Selected.Cats.Clear();
            var points = new[] { new Int2(0, 0), new Int2(1, 0), new Int2(0, 1), new Int2(2, 0), new Int2(3, 0) };
            foreach (var p in points)
            {
                game.CurrentWorld.Tiles.Add(new Tile { Position = p, Wall = p.X != 2, Road = p.X == 2 }); game.Selected.Known.Add(p);
            }
            typeof(ForestView).GetMethod("Reconcile", Private).Invoke(game.View, null);
            yield return null;
            var world = (Transform)typeof(ForestView).GetField("worldRoot", Private).GetValue(game.View);
            foreach (Transform entity in world)
                if (entity.name.StartsWith("tile:") || entity.name.StartsWith("fence:") || entity.name.StartsWith("gate:"))
                    foreach (var filter in entity.GetComponentsInChildren<MeshFilter>())
                        filter.gameObject.AddComponent<MeshCollider>().sharedMesh = filter.sharedMesh;
            Physics.SyncTransforms();
            foreach (var p in new[] { new Vector3(.3f, 3, 0), new Vector3(.7f, 3, 0), new Vector3(0, 3, .3f), new Vector3(0, 3, .7f) })
                Assert.That(Physics.Raycast(p, Vector3.down, 2.95f), Is.True, "Visible fence rails must meet through the corner at " + p);
            Assert.That(Physics.Raycast(new Vector3(2, .45f, -1), Vector3.forward, 2), Is.False, "The gate needs a visibly clear passage at cat height.");
            Assert.That(world.GetComponentsInChildren<Transform>().Any(t => t.name.StartsWith("MISSING ASSET")), Is.False);
        }

        private Vector3 GroundAt(Vector2 cursor)
        {
            var ray = game.View.Camera.ScreenPointToRay(cursor);
            Assert.That(new Plane(Vector3.up, Vector3.zero).Raycast(ray, out float distance), Is.True);
            return ray.GetPoint(distance);
        }
    }
}
