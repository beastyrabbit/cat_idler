using System;
using System.Collections;
using System.IO;
using System.Linq;
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
        private const int TerrainLayer = 8;
        private ForestGame game;
        private string directory;
        private Mouse mouse;
        private Keyboard keyboard;

        [UnitySetUp]
        public IEnumerator StartIsolatedWorld()
        {
            Assert.That(ForestGame.Instance, Is.Null, "Never replace the player's running world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-camera-fence-" + Guid.NewGuid().ToString("N"));
            game = new GameObject("Isolated camera and fence test").AddComponent<ForestGame>();
            game.InitialSavePath = Path.Combine(directory, "world.json");
            // Fixed-coordinate rendering fixtures are written against the classic founding at the origin; the demo colony
            // is covered by CardTests, GameLoopTests, WidgetTests and ForestUiRevisionTests.
            game.InitialClassicWorld = true;
            yield return null;
            game.SetSpeed(0); game.UI.ClosePanel();
            mouse = InputSystem.AddDevice<Mouse>(); keyboard = InputSystem.AddDevice<Keyboard>();
            mouse.MakeCurrent(); keyboard.MakeCurrent();
        }

        [UnityTearDown]
        public IEnumerator CloseIsolatedWorld()
        {
            GameSettings.ReducedMotion = false;
            InputRouter.ScrollStyle = ScrollStyle.Auto;
            if (mouse != null) InputSystem.RemoveDevice(mouse);
            if (keyboard != null) InputSystem.RemoveDevice(keyboard);
            if (game != null) UnityEngine.Object.Destroy(game.gameObject);
            yield return null;
            if (Directory.Exists(directory)) Directory.Delete(directory, true);
        }

        [UnityTest]
        public IEnumerator OneNormalizedWheelStepZoomsAroundThePointer()
        {
            game.View.enabled = false;
            WorldOnly();
            yield return null;
            game.View.UpdateCamera();
            var cursor = new Vector2(Screen.width * .7f, Screen.height * .4f);
            float height = TerrainHeight(cursor);
            var before = PlaneAt(cursor, height); float initial = game.View.Camera.orthographicSize;
            InputSystem.QueueStateEvent(mouse, new MouseState { position = cursor, scroll = new Vector2(0, 1) });
            InputSystem.Update();
            Assert.That(mouse.scroll.ReadValue().y, Is.EqualTo(1f), "Exercise the normalized Input System value.");
            Assert.That(game.UI.PointerOverPanel, Is.False, "The test pointer must be over the game world.");
            game.View.TickInput(); game.View.ApplyContinuousInput();
            Assert.That(InputRouter.Current.TrackpadScrolling, Is.False, "A whole wheel step is a wheel, not a trackpad pan.");
            Assert.That(game.View.Zoom, Is.LessThan(initial * .95f), "One wheel step sets the new zoom at once.");
            for (float until = Time.realtimeSinceStartup + 1.2f; Time.realtimeSinceStartup < until;)
            {
                yield return null;
                game.View.UpdateCamera();
                Assert.That(Vector3.Distance(before, PlaneAt(cursor, height)), Is.LessThan(.03f), "The world point under the cursor stays under it on every eased frame.");
            }
            Assert.That(game.View.Camera.orthographicSize, Is.EqualTo(game.View.Zoom).Within(.02f), "The eased zoom arrives within a second.");
            Assert.That(game.View.Camera.orthographicSize, Is.LessThan(initial * .95f));
        }

        [Test]
        public void ZoomButtonsAreReversibleAndRespectCloseAndWideLimits()
        {
            // ZoomBy is public: management needs an accessible alternative to mouse-wheel input.
            float initial = game.View.Zoom;
            game.View.ZoomBy(1f);
            Assert.That(game.View.Zoom, Is.LessThan(initial * .95f));
            game.View.ZoomBy(-1f);
            Assert.That(game.View.Zoom, Is.EqualTo(initial).Within(.001f));
            game.View.ZoomBy(1000f);
            Assert.That(game.View.Zoom, Is.InRange(3f, 5f));
            game.View.ZoomBy(-1000f);
            Assert.That(game.View.Zoom, Is.InRange(20f, 24f), "Fully zoomed out stays inside the built ground instead of showing its edge.");
        }

        [UnityTest]
        public IEnumerator DefaultFramingIsADioramaAtReadableCatSize()
        {
            game.View.FocusVillage();
            yield return Settle();
            Assert.That(game.View.Zoom, Is.EqualTo(ForestView.DefaultZoom));
            Assert.That(game.View.Camera.orthographic, Is.True);
            Assert.That(game.View.CameraYaw, Is.EqualTo(40).Within(.5f), "The default view looks at the colony diagonally, not as a floorplan.");
            Assert.That(game.View.CameraPitch, Is.EqualTo(50).Within(.5f));
            Assert.That(game.View.CameraStandoff, Is.EqualTo(25).Within(.01f), "A close standoff keeps cast shadows inside the shadow distance.");
            float catPixelsAt1080 = 1080f / (2 * game.View.Camera.orthographicSize) * .85f;
            Assert.That(catPixelsAt1080, Is.InRange(45f, 60f), "A cat reads about 50 px tall at 1920x1080.");
            game.View.ZoomBy(1000);
            yield return Settle();
            Assert.That(game.View.CameraPitch, Is.EqualTo(40).Within(.5f), "Zoomed in, the view tilts toward the cats.");
        }

        [UnityTest]
        public IEnumerator ProgrammaticMovesGlideInsteadOfTeleporting()
        {
            yield return Settle();
            var start = game.View.CameraFocus;
            var building = game.Selected.Buildings.OrderByDescending(b => Vector3.Distance(Center(b), start)).First();
            game.View.InspectBuilding(building.Id);
            var target = game.View.CameraTarget;
            Assume.That(Vector3.Distance(start, target), Is.GreaterThan(2f), "The test needs a building away from the framed point.");
            yield return null;
            float travelled = Vector3.Distance(start, game.View.CameraFocus);
            Assert.That(travelled, Is.LessThan(Vector3.Distance(start, target) * .75f), "The first frame must not jump to the target.");
            yield return Settle();
            Assert.That(Vector3.Distance(game.View.CameraFocus, target), Is.LessThan(.05f), "The camera arrives within about a second.");
        }

        [UnityTest]
        public IEnumerator OpeningADrawerLeavesTheWorldWhereItIs()
        {
            yield return Settle();
            var framed = game.View.CameraFocus; var position = game.View.Camera.transform.position;
            game.UI.OpenPanel("Cats");
            yield return Settle();
            Assert.That(Vector3.Distance(game.View.CameraFocus, framed), Is.LessThan(.01f), "Drawers float over the map; the camera does not shift for them.");
            Assert.That(Vector3.Distance(game.View.Camera.transform.position, position), Is.LessThan(.01f));
        }

        [UnityTest]
        public IEnumerator LeftDragPansWithTheGrabbedPointUnderThePointerAndProducesNoClick()
        {
            game.View.enabled = false;
            WorldOnly();
            yield return null;
            game.View.UpdateCamera();
            int clicks = 0;
            using (InputRouter.Current.BindPointer(PointerAction.PrimaryClick, InputPriority.Selection + 1, p => { clicks++; return true; }))
            {
                var from = new Vector2(Screen.width * .5f, Screen.height * .5f);
                var to = from + new Vector2(Screen.width * .12f, -Screen.height * .1f);
                float height = TerrainHeight(from);
                var grabbed = PlaneAt(from, height);
                Press(from); game.View.TickInput(); yield return null;
                for (int i = 1; i <= 6; i++)
                {
                    Move(Vector2.Lerp(from, to, i / 6f)); game.View.TickInput(); yield return null;
                    Assert.That(Vector3.Distance(PlaneAt(Vector2.Lerp(from, to, i / 6f), height), grabbed), Is.LessThan(.03f), "The grabbed ground stays under the pointer while dragging.");
                }
                Release(to); game.View.TickInput(); yield return null;
                Assert.That(clicks, Is.Zero, "A drag past the threshold is not a click.");
                Assert.That(game.View.CameraTarget, Is.EqualTo(game.View.CameraFocus), "Drag pans move the camera 1:1 without easing lag.");
                Press(to); game.View.TickInput(); yield return null; Release(to); game.View.TickInput();
                Assert.That(clicks, Is.EqualTo(1), "A press and release in place is a click.");
            }
        }

        [UnityTest]
        public IEnumerator PanningStopsAtTheKnownForestPlusAMargin()
        {
            game.View.enabled = false;
            yield return null;
            game.View.UpdateCamera();
            var known = game.Selected.Known.Where(p => p.Level == 0).ToList();
            float minX = known.Min(p => p.X) - ForestView.FocusMargin, maxX = known.Max(p => p.X) + ForestView.FocusMargin;
            float minZ = known.Min(p => p.Z) - ForestView.FocusMargin, maxZ = known.Max(p => p.Z) + ForestView.FocusMargin;
            var router = InputRouter.Current;
            var from = new Vector2(Screen.width * .5f, Screen.height * .5f);
            for (int i = 0; i < 12; i++)
            {
                router.DispatchPointer(PointerAction.PrimaryDragStart, new PointerInfo(from, Vector2.zero, from, false, false));
                router.DispatchPointer(PointerAction.PrimaryDragEnd, new PointerInfo(from + new Vector2(-Screen.width * .4f, Screen.height * .4f), Vector2.zero, from, false, false));
            }
            var target = game.View.CameraTarget;
            Assert.That(target.x, Is.InRange(minX - .001f, maxX + .001f));
            Assert.That(target.z, Is.InRange(minZ - .001f, maxZ + .001f));
            bool onEdge = Mathf.Abs(target.x - minX) < .01f || Mathf.Abs(target.x - maxX) < .01f || Mathf.Abs(target.z - minZ) < .01f || Mathf.Abs(target.z - maxZ) < .01f;
            Assert.That(onEdge, Is.True, "Dragging far away ends on the clamp, not in the void: " + target);
            Assert.That(game.View.CameraFocus, Is.EqualTo(target), "The rendered camera stops at the same edge.");
        }

        [UnityTest]
        public IEnumerator RotateKeysTurnTheViewInEasedQuarterTurns()
        {
            yield return Settle();
            PressKey(Key.E); game.View.TickInput(); ReleaseKeys();
            Assert.That(Mathf.DeltaAngle(game.View.CameraTargetYaw, 130), Is.EqualTo(0).Within(.01f), "E turns the management view a quarter turn.");
            yield return null;
            Assert.That(Mathf.Abs(Mathf.DeltaAngle(game.View.CameraYaw, 130)), Is.GreaterThan(5), "The turn eases instead of snapping.");
            yield return Settle();
            Assert.That(Mathf.DeltaAngle(game.View.CameraYaw, 130), Is.EqualTo(0).Within(.5f));
            PressKey(Key.Q); game.View.TickInput(); ReleaseKeys(); yield return null;
            PressKey(Key.Q); game.View.TickInput(); ReleaseKeys();
            yield return Settle(); yield return Settle();
            Assert.That(Mathf.DeltaAngle(game.View.CameraYaw, -50), Is.EqualTo(0).Within(.5f), "Q turns back the other way.");
        }

        [UnityTest]
        public IEnumerator TypedPlusAndMinusZoomOnEveryLayout()
        {
            game.View.enabled = false;
            yield return null;
            game.View.TickInput();
            float initial = game.View.Zoom;
            InputSystem.QueueTextEvent(keyboard, '+'); InputSystem.Update(); game.View.TickInput();
            Assert.That(game.View.Zoom, Is.LessThan(initial * .95f), "'+' typed from any physical key zooms in (German '+' is not the US Equals key).");
            float closer = game.View.Zoom;
            InputSystem.QueueTextEvent(keyboard, '-'); InputSystem.Update(); game.View.TickInput();
            Assert.That(game.View.Zoom, Is.EqualTo(initial).Within(.001f));
            InputSystem.QueueTextEvent(keyboard, '='); InputSystem.Update(); game.View.TickInput();
            Assert.That(game.View.Zoom, Is.EqualTo(closer).Within(.001f), "'=' zooms in too.");
        }

        [UnityTest]
        public IEnumerator TrackpadScrollPansAndCommandScrollZooms()
        {
            game.View.enabled = false;
            WorldOnly();
            yield return null;
            game.View.UpdateCamera();
            Assert.That(InputRouter.ReadsAsTrackpad(new Vector2(0, 1), false, false, ScrollStyle.Auto), Is.False, "A whole wheel notch zooms.");
            Assert.That(InputRouter.ReadsAsTrackpad(new Vector2(0, .37f), false, false, ScrollStyle.Auto), Is.True, "Fractional two-finger motion pans.");
            Assert.That(InputRouter.ReadsAsTrackpad(new Vector2(.2f, 0), false, false, ScrollStyle.Auto), Is.True, "Horizontal motion pans.");
            Assert.That(InputRouter.ReadsAsTrackpad(new Vector2(0, 2.6f), true, false, ScrollStyle.Auto), Is.False, "A fast wheel spin stays a zoom.");
            Assert.That(InputRouter.ReadsAsTrackpad(new Vector2(0, 1), false, false, ScrollStyle.TrackpadPans), Is.True);
            Assert.That(InputRouter.ReadsAsTrackpad(new Vector2(.3f, .4f), false, false, ScrollStyle.WheelZooms), Is.False);

            var cursor = new Vector2(Screen.width * .5f, Screen.height * .5f);
            float zoom = game.View.Zoom; var framed = game.View.CameraFocus;
            InputSystem.QueueStateEvent(mouse, new MouseState { position = cursor, scroll = new Vector2(.3f, -.45f) });
            InputSystem.Update();
            game.View.TickInput(); game.View.ApplyContinuousInput();
            Assert.That(InputRouter.Current.TrackpadScrolling, Is.True);
            Assert.That(game.View.Zoom, Is.EqualTo(zoom), "A two-finger swipe does not zoom.");
            Assert.That(Vector3.Distance(game.View.CameraFocus, framed), Is.GreaterThan(.05f), "A two-finger swipe pans the map.");

            var command = Application.platform == RuntimePlatform.OSXEditor || Application.platform == RuntimePlatform.OSXPlayer ? Key.LeftMeta : Key.LeftCtrl;
            InputSystem.QueueStateEvent(keyboard, new KeyboardState(command));
            InputSystem.QueueStateEvent(mouse, new MouseState { position = cursor, scroll = new Vector2(0, .45f) });
            InputSystem.Update();
            game.View.TickInput(); game.View.ApplyContinuousInput();
            Assert.That(InputRouter.Current.CommandHeld, Is.True);
            Assert.That(game.View.Zoom, Is.LessThan(zoom), "Command with a two-finger swipe zooms.");
            InputSystem.QueueStateEvent(keyboard, new KeyboardState());
            InputSystem.Update();
        }

        [UnityTest]
        public IEnumerator HomeRecentresAndFocusSelectedFollowsTheCatUntilAManualPan()
        {
            var cat = game.Selected.Cats.First(c => c.Alive);
            game.View.InspectCat(cat.Id);
            PressKey(Key.F); game.View.TickInput(); ReleaseKeys();
            Assert.That(game.View.FollowedCatId, Is.EqualTo(cat.Id), "F frames and follows the selected cat.");
            cat.X += 2; cat.Z += 1;
            yield return Settle();
            Assert.That(Vector2.Distance(new Vector2(game.View.CameraFocus.x, game.View.CameraFocus.z), new Vector2((float)cat.X, (float)cat.Z)), Is.LessThan(.1f), "A followed cat stays framed while it walks.");
            PressKey(Key.D); yield return null; ReleaseKeys(); yield return null;
            Assert.That(game.View.FollowedCatId, Is.Empty, "A manual pan ends following.");
            PressKey(Key.H); game.View.TickInput(); ReleaseKeys();
            yield return Settle();
            var center = game.Selected.Center;
            Assert.That(Vector2.Distance(new Vector2(game.View.CameraFocus.x, game.View.CameraFocus.z), new Vector2(center.X, center.Z)), Is.LessThan(.1f), "H recentres on the village.");
            Assert.That(game.View.SelectedCat, Is.SameAs(cat), "Going home keeps the selection.");
        }

        [UnityTest]
        public IEnumerator ClickingARoofPicksItsBuildingThroughTheRouter()
        {
            game.View.enabled = false;
            WorldOnly();
            game.Selected.Cats.Clear();
            var building = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            game.View.InspectBuilding(building.Id);
            game.View.ForceReconcile();
            yield return Ease();
            // Framed; now deselect so the tethered card (a bottom sheet in the small batch window) does not cover the roof.
            game.UI.Select(Subject.None); UiTestKit.Tick(game);
            Assert.That(game.View.TryGetEntity("building:" + building.Id, out var go), Is.True);
            var bounds = go.GetComponentsInChildren<Renderer>().Select(r => r.bounds).Aggregate((a, b) => { a.Encapsulate(b); return a; });
            Vector2 roof = game.View.Camera.WorldToScreenPoint(new Vector3(bounds.center.x, bounds.max.y - .05f, bounds.center.z));
            var groundTile = new Int2(Mathf.RoundToInt(PlaneAt(roof, 0).x), Mathf.RoundToInt(PlaneAt(roof, 0).z));
            TestContext.WriteLine("Roof pixel " + roof + " lies over ground tile " + groundTile + "; footprint " + building.Position + " " + building.Width + "x" + building.Depth);
            Assert.That(game.View.SubjectAt(roof), Is.EqualTo(new Subject(SubjectKind.Building, building.Id)), "The roof selects its building, not the ground behind it.");

            Subject clicked = Subject.None;
            using (InputRouter.Current.BindPointer(PointerAction.PrimaryClick, InputPriority.Selection + 1, p => { clicked = game.View.SubjectAt(p.Position); return true; }))
            {
                Move(roof); game.View.TickInput(); game.View.UpdateHover();
                Assert.That(game.View.HoveredSubject, Is.EqualTo(new Subject(SubjectKind.Building, building.Id)), "Hovering the roof reports the building.");
                yield return null;
                Press(roof); game.View.TickInput(); yield return null;
                Assert.That(clicked.IsNone, Is.True, "Selection waits for the release.");
                Release(roof); game.View.TickInput();
            }
            Assert.That(clicked, Is.EqualTo(new Subject(SubjectKind.Building, building.Id)));
        }

        [UnityTest]
        public IEnumerator CatsArePickedByScreenDistanceToTheirBodies()
        {
            game.View.enabled = false;
            game.View.ForceReconcile();
            yield return Ease();
            var picked = game.View.CatTransforms().Select(t => (t.id, screen: (Vector2)game.View.Camera.WorldToScreenPoint(t.t.position + Vector3.up * .28f)))
                .Where(t => t.screen.x > 40 && t.screen.y > 40 && t.screen.x < Screen.width - 40 && t.screen.y < Screen.height - 40)
                .OrderBy(t => t.id).ToList();
            Assume.That(picked, Is.Not.Empty, "Some cat must be on screen.");
            // The cat whose body is furthest from every other cat on screen.
            float Neighbour((string id, Vector2 screen) a) => picked.Where(b => b.id != a.id).Select(b => Vector2.Distance(a.screen, b.screen)).DefaultIfEmpty(9999).Min();
            var lonely = picked.OrderByDescending(Neighbour).First();
            var nearest = picked.Where(b => b.id != lonely.id).OrderBy(b => Vector2.Distance(b.screen, lonely.screen)).Select(b => b.screen).DefaultIfEmpty(lonely.screen + Vector2.right).First();
            float scale = Screen.dpi >= 160 ? 2 : 1;
            // A click a little off the body centre, away from the nearest other cat, still picks this cat.
            var away = (lonely.screen - nearest).normalized * Mathf.Min(8 * scale, Neighbour(lonely) * .3f);
            Assert.That(game.View.SubjectAt(lonely.screen + away), Is.EqualTo(new Subject(SubjectKind.Cat, lonely.id)), "A click on a cat's body picks that cat.");
            Assert.That(game.View.SubjectAt(lonely.screen), Is.EqualTo(new Subject(SubjectKind.Cat, lonely.id)));
            var far = game.View.SubjectAt(lonely.screen + new Vector2(80, 0) * scale);
            Assert.That(far.Kind == SubjectKind.Cat && far.Id == lonely.id, Is.False, "A click well beside a cat does not pick it.");
        }

        [UnityTest]
        public IEnumerator KnownDungeonEntranceIsPickable()
        {
            game.View.enabled = false;
            var site = game.CurrentWorld.Dungeons.FirstOrDefault(d => game.Selected.Known.Contains(d.Entrance));
            Assume.That(site, Is.Not.Null, "The founding world reveals its first entrance.");
            game.View.FocusDungeon(site.Id);
            game.View.ForceReconcile();
            yield return Ease();
            Assert.That(game.View.TryGetEntity("dungeon:" + site.Id, out var arch), Is.True);
            var bounds = arch.GetComponentsInChildren<Renderer>().Select(r => r.bounds).Aggregate((a, b) => { a.Encapsulate(b); return a; });
            Vector2 screen = game.View.Camera.WorldToScreenPoint(bounds.center + Vector3.up * bounds.extents.y * .5f);
            Assert.That(game.View.SubjectAt(screen), Is.EqualTo(new Subject(SubjectKind.Entrance, site.Id)));
        }

        [UnityTest]
        public IEnumerator IntroSwoopsFromTheCanopyToTheDefaultFramingAndAnyKeySkipsIt()
        {
            game.View.PlayIntro();
            Assert.That(game.View.IntroPlaying, Is.True);
            yield return null;
            Assert.That(game.View.CameraPitch, Is.GreaterThan(70), "The swoop starts looking down from canopy height.");
            Assert.That(game.View.Camera.orthographicSize, Is.GreaterThan(18));
            yield return new WaitForSecondsRealtime(ForestView.IntroSeconds + .4f);
            Assert.That(game.View.IntroPlaying, Is.False);
            Assert.That(game.View.Camera.orthographicSize, Is.EqualTo(ForestView.DefaultZoom).Within(.05f));
            Assert.That(game.View.CameraPitch, Is.EqualTo(50).Within(.5f));
            Assert.That(game.View.CameraYaw, Is.EqualTo(40).Within(.5f));
            var leader = game.Selected.Cats.First(c => c.Id == game.Selected.LeaderId);
            Assert.That(Vector2.Distance(new Vector2(game.View.CameraTarget.x, game.View.CameraTarget.z), new Vector2((float)leader.X, (float)leader.Z)), Is.LessThan(.6f), "The swoop ends framing the colony leader.");

            game.View.PlayIntro(); yield return null; yield return null;
            Assert.That(game.View.IntroPlaying, Is.True);
            PressKey(Key.Space); game.View.TickInput(); ReleaseKeys();
            Assert.That(game.View.IntroPlaying, Is.False, "Any key skips the swoop.");

            GameSettings.ReducedMotion = true;
            game.View.PlayIntro();
            Assert.That(game.View.IntroPlaying, Is.False, "Reduced motion starts framed.");
            Assert.That(game.View.Camera.orthographicSize, Is.EqualTo(ForestView.DefaultZoom).Within(.01f));
        }

        [UnityTest]
        public IEnumerator LeavingControlFramesTheCatAndKeepsTheManagementZoom()
        {
            var cat = game.Selected.Cats.First(c => c.Alive);
            game.View.InspectCat(cat.Id);
            game.View.GoHome();
            yield return Settle();
            float initial = game.View.ManagementZoom;
            game.View.EnterSelectedCat(); yield return null;
            Assert.That(game.View.DirectControl, Is.True);
            game.View.ZoomBy(1); game.View.LeaveCat(); yield return null;
            Assert.That(game.View.Camera.orthographic, Is.True);
            Assert.That(game.View.ManagementZoom, Is.EqualTo(initial).Within(.001f), "Management zoom survives the same-cat control roundtrip.");
            Assert.That(Vector2.Distance(new Vector2(game.View.CameraFocus.x, game.View.CameraFocus.z), new Vector2((float)cat.X, (float)cat.Z)), Is.LessThan(.6f), "Leaving control frames the cat where it stands, not where control began.");
        }

        [Test]
        public void TabNoLongerMovesHudFocus()
        {
            game.View.TickInput();
            var root = game.GetComponent<UIDocument>().rootVisualElement;
            var first = root.Query<Button>().ToList().FirstOrDefault(b => b.focusable && b.resolvedStyle.display != DisplayStyle.None);
            Assume.That(first, Is.Not.Null);
            first.Focus();
            var focused = root.focusController.focusedElement;
            using (var tab = KeyDownEvent.GetPooled('\t', KeyCode.Tab, EventModifiers.None))
            {
                tab.target = first;
                root.panel.visualTree.SendEvent(tab);
            }
            using (var next = NavigationMoveEvent.GetPooled(NavigationMoveEvent.Direction.Next))
            {
                next.target = first;
                root.panel.visualTree.SendEvent(next);
            }
            Assert.That(root.focusController.focusedElement, Is.SameAs(focused), "Tab toggles direct control; it must not also walk focus across HUD buttons.");
        }

        [Test]
        public void FootTrafficDoesNotTurnIntoPavingOrHideUncollectedResources()
        {
            var tile = new Tile { Dirt = true, Resource = "logs", Amount = 200 };
            Assert.That(game.View.TileAsset(tile), Is.EqualTo("tree_oak"), "Walking past a tree must not replace it with a paved road.");
            tile.Resource = ""; tile.Amount = 0;
            Assert.That(game.View.TileAsset(tile), Is.EqualTo(""), "Worn earth belongs to the terrain. Only built roads get stone paving.");
            tile.Road = true;
            Assert.That(game.View.TileAsset(tile), Is.EqualTo("road"));
        }

        [UnityTest]
        public IEnumerator LegacyBuildingsKeepTheirOrientationWithoutInventingADoor()
        {
            game.View.enabled = false;
            var building = game.Selected.Buildings.First(b => b.Kind == "den");
            building.HasEntrance = false;
            game.View.ForceReconcile();
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
            Assert.That(world.Crossable(before, next), Is.False);
            game.View.ForceReconcile(); yield return null;
            var cart = GameObject.Find("trade:" + trade.Id);
            Assert.That(cart, Is.Not.Null);
            Assert.That(cart.transform.position, Is.EqualTo(new Vector3(11, 0, 0)), "A stopped caravan must remain on the near side of its blocked edge.");
            trade.Status = "returning"; game.View.ForceReconcile();
            Assert.That(cart.transform.position, Is.EqualTo(new Vector3(11, 0, 0)), "Returning cargo uses its actual position too.");
            village.BoundaryEdges.Clear(); trade.X = next.X; trade.Z = next.Z; trade.PathIndex = 1;
            game.View.ForceReconcile(); yield return null;
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
            game.View.ForceReconcile();
            yield return null;
            var world = game.View.WorldRoot;
            foreach (Transform entity in world)
            {
                Assert.That(entity.childCount, Is.EqualTo(1), entity.name + " keeps its model under a single visual child");
                Assert.That(ForestView.VisualOf(entity.gameObject), Is.SameAs(entity.GetChild(0)), entity.name);
                Assert.That(entity.GetChild(0).name, Is.EqualTo(ForestView.VisualName));
            }
            // Fences and gates are instanced static props (W2-20): ask the view what it draws and ray-test collider proxies of it.
            Assert.That(game.View.TryGetStaticProp(new Int2(2, 0), out var gateAsset) ? gateAsset : "", Is.EqualTo("gate"), "The road between the fence arms is drawn as a gate");
            var proxies = new GameObject("Static prop colliders").transform;
            Assert.That(game.View.BuildStaticPropColliders(proxies, asset => asset == "fence_post" || asset == "fence_rail" || asset == "gate"), Is.GreaterThan(0));
            Physics.SyncTransforms();
            foreach (var p in new[] { new Vector3(.3f, 3, 0), new Vector3(.7f, 3, 0), new Vector3(0, 3, .3f), new Vector3(0, 3, .7f) })
                Assert.That(Physics.Raycast(p, Vector3.down, 2.95f), Is.True, "Visible fence rails must meet through the corner at " + p);
            Assert.That(Physics.Raycast(new Vector3(2, .45f, -1), Vector3.forward, 2), Is.False, "The gate needs a visibly clear passage at cat height.");
            Assert.That(world.GetComponentsInChildren<Transform>().Any(t => t.name.StartsWith("MISSING ASSET")), Is.False);
            UnityEngine.Object.Destroy(proxies.gameObject);
        }

        /// <summary>Lets the enabled view run for about a second, long enough for every eased camera move.</summary>
        private static IEnumerator Settle()
        {
            float until = Time.realtimeSinceStartup + 1.1f;
            while (Time.realtimeSinceStartup < until) yield return null;
        }

        /// <summary>Steps the camera of a disabled view for about a second.</summary>
        private IEnumerator Ease()
        {
            for (float until = Time.realtimeSinceStartup + 1.1f; Time.realtimeSinceStartup < until;)
            {
                game.View.UpdateCamera();
                yield return null;
            }
        }

        /// <summary>Makes the HUD transparent to the pointer, so the test pointer is over the world in any batch viewport.</summary>
        private void WorldOnly()
        {
            foreach (var element in game.GetComponent<UIDocument>().rootVisualElement.Query<VisualElement>().ToList()) element.RemoveFromClassList("interactive");
        }

        private static Vector3 Center(Building b) => new Vector3(b.Position.X + (b.Width - 1) * .5f, 0, b.Position.Z + (b.Depth - 1) * .5f);

        private float TerrainHeight(Vector2 cursor)
        {
            var ray = game.View.Camera.ScreenPointToRay(cursor);
            return Physics.Raycast(ray, out var hit, 240, 1 << TerrainLayer) ? hit.point.y : 0;
        }

        private Vector3 PlaneAt(Vector2 cursor, float height)
        {
            var ray = game.View.Camera.ScreenPointToRay(cursor);
            Assert.That(new Plane(Vector3.up, new Vector3(0, height, 0)).Raycast(ray, out float distance), Is.True);
            return ray.GetPoint(distance);
        }

        // Synthetic devices write the player's Dynamic buffer; a batch Editor can default to its separate Editor buffer.
        private void Move(Vector2 position) => InputState.Change(mouse.position, position, InputUpdateType.Dynamic);

        private void Press(Vector2 position) => SetPrimary(position, 1);

        private void Release(Vector2 position) => SetPrimary(position, 0);

        private void SetPrimary(Vector2 position, float value)
        {
            Move(position);
            using (StateEvent.From(mouse, out var e))
            {
                mouse.leftButton.WriteValueIntoEvent(value, e);
                InputState.Change(mouse, e, InputUpdateType.Dynamic);
            }
        }

        private void PressKey(Key key) => InputState.Change(keyboard, new KeyboardState(key), InputUpdateType.Dynamic);

        private void ReleaseKeys() => InputState.Change(keyboard, new KeyboardState(), InputUpdateType.Dynamic);
    }
}
