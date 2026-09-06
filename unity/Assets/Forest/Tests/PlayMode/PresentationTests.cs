using System;
using System.Collections;
using System.IO;
using System.Linq;
using IdleCatForest.Presentation;
using IdleCatForest.Simulation;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.UIElements;

namespace IdleCatForest.Tests
{
    public class PresentationTests
    {
        [Test]
        public void EveryMaintainedBuildingAndCargoTypeHasAuthoredGeometry()
        {
            foreach (var name in Simulation.Catalog.Buildings)
            {
                var asset = Resources.Load<GameObject>("ForestArt/" + name);
                Assert.That(asset, Is.Not.Null, name);
                Assert.That(asset.GetComponentsInChildren<MeshFilter>().Sum(m => m.sharedMesh.vertexCount), Is.GreaterThan(30), name);
            }
            var cat = Resources.Load<GameObject>("ForestArt/cat"); Assert.That(cat, Is.Not.Null);
            foreach (var part in new[] { "Head", "Tail", "PawFrontLeft", "PawFrontRight", "PawBackLeft", "PawBackRight" })
                Assert.That(cat.GetComponentsInChildren<Transform>().Any(t => t.name == part), Is.True, part);
        }

        [Test]
        public void AllManagementCategoriesAreReachableInNormalUi()
        {
            foreach (var section in new[] { "Village", "Cats", "Build", "Work", "Stores", "Research", "Officers", "Shrine", "Defense", "Trade", "Routes", "Events" })
                CollectionAssert.Contains(ForestUI.Sections, section);
        }
    }

    public class LivePresentationTests
    {
        private ForestGame game;
        private string directory;

        [UnitySetUp]
        public IEnumerator StartIsolatedWorld()
        {
            Assert.That(ForestGame.Instance, Is.Null, "The test must never replace a running user's world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-presentation-" + Guid.NewGuid().ToString("N"));
            game = new GameObject("Isolated presentation test").AddComponent<ForestGame>();
            game.InitialSavePath = Path.Combine(directory, "world.json");
            yield return null;
            Assert.That(game.Selected, Is.Not.Null, game.Status);
            game.SetSpeed(0);
        }

        [UnityTearDown]
        public IEnumerator CloseIsolatedWorld()
        {
            if (game != null) UnityEngine.Object.Destroy(game.gameObject);
            yield return null;
            if (directory != null && Directory.Exists(directory)) Directory.Delete(directory, true);
        }

        [UnityTest]
        public IEnumerator LocalLiveSimulationConsumesOneFiftyMillisecondStep()
        {
            var flags = System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic;
            typeof(ForestGame).GetField("accumulated", flags).SetValue(game, .05);
            double before = game.CurrentWorld.TimeSeconds;
            typeof(ForestGame).GetMethod("Update", flags).Invoke(game, null);
            Assert.That(game.CurrentWorld.TimeSeconds - before, Is.EqualTo(.05).Within(1e-8), "Autonomous simulation must advance without waiting for a 100ms presentation batch.");
            yield break;
        }

        [UnityTest]
        public IEnumerator LiveInspectorUpdatesWorkWithoutRebuildingThePanel()
        {
            var cat = game.Selected.Cats[1];
            var job = new Job { Id = "live-inspector-job", Kind = "woodcut", Phase = "working", CatId = cat.Id, Position = cat.Position, Progress = 1, RequiredWork = 20 };
            game.Selected.Jobs.Add(job); cat.JobId = job.Id;
            game.View.InspectCat(cat.Id); game.UI.OpenPanel("Inspect");
            var root = game.GetComponent<UIDocument>().rootVisualElement;
            var progress = root.Q<ProgressBar>("live-work-progress");
            Assert.That(progress, Is.Not.Null, "Working cats need a live progress meter.");
            var flags = System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic;
            typeof(ForestUI).GetField("nextRefresh", flags).SetValue(game.UI, float.MaxValue);
            job.Progress = 1.05;
            typeof(ForestUI).GetMethod("Update", flags).Invoke(game.UI, null);
            Assert.That(root.Q<ProgressBar>("live-work-progress"), Is.SameAs(progress), "Live progress must not rebuild controls or reset interaction.");
            Assert.That(progress.value, Is.EqualTo(1.05f).Within(1e-5));
            yield break;
        }

        [UnityTest]
        public IEnumerator LiveMoversFollowSubsecondPositionsBetweenReconciles()
        {
            game.View.enabled = false;
            var point = new Int2(4, 3);
            var vehicle = new Vehicle { Id = "live-cart", Mode = "rail", Position = point, HasContinuousPosition = true, X = 4, Z = 3 };
            var raid = new Raid { Id = "live-raider", Position = point, HasContinuousPosition = true, X = 4, Z = 3 };
            var trade = new TradeOffer { Id = "live-caravan", Status = "outbound", Position = point, HasContinuousPosition = true, X = 4, Z = 3 };
            trade.Path.Add(new Int2(5, 3)); game.Selected.Vehicles.Add(vehicle); game.Selected.Raids.Add(raid); game.CurrentWorld.TradeOffers.Add(trade);
            var flags = System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic;
            typeof(ForestView).GetMethod("Reconcile", flags).Invoke(game.View, null);
            var objects = new[] { GameObject.Find("vehicle:" + vehicle.Id), GameObject.Find("raid:" + raid.Id), GameObject.Find("trade:" + trade.Id) };
            foreach (var entity in objects) Assert.That(entity, Is.Not.Null);
            vehicle.X = raid.X = trade.X = 4.05;
            typeof(ForestView).GetMethod("AnimateCats", flags).Invoke(game.View, null);
            foreach (var entity in objects)
            {
                Assert.That(entity.transform.position.x, Is.GreaterThan(4), "Movers must update between the slower scenery reconciliations.");
                Assert.That(entity.transform.position.x, Is.LessThanOrEqualTo(4.05001f), "Presentation must not predict beyond authoritative progress.");
                Assert.That(entity.transform.position.z, Is.EqualTo(3).Within(1e-6));
            }
            yield break;
        }

        [UnityTest]
        public IEnumerator GroundRebuildPreservesTerrainAndNeverGeneratesUnknownTiles()
        {
            game.View.enabled = false;
            var world = game.CurrentWorld;
            world.Tiles.Clear(); game.Selected.Known.Clear();
            for (int z = -27; z <= 27; z++) for (int x = -27; x <= 27; x++)
            {
                var point = new Int2(x, z);
                world.Tiles.Add(new Tile { Position = point, Water = x == 1 && z == 0, Mountain = x == 2 && z == 0 });
                game.Selected.Known.Add(point);
            }
            game.Selected.Known.Add(new Int2(28, 0));
            var flags = System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic;
            var center = typeof(ForestView).GetField("groundCenter", flags);
            var rebuild = typeof(ForestView).GetMethod("MakeGround", flags);
            var ground = (Transform)typeof(ForestView).GetField("groundRoot", flags).GetValue(game.View);
            int tileCount = world.Tiles.Count;
            double elapsed = 0;
            for (int i = 0; i < 3; i++)
            {
                center.SetValue(game.View, new Int2(i * 8, 0));
                var timer = System.Diagnostics.Stopwatch.StartNew();
                rebuild.Invoke(game.View, null); elapsed += timer.Elapsed.TotalMilliseconds;
                yield return null;
                var meshes = ground.GetComponentsInChildren<MeshFilter>();
                Assert.That(meshes.Sum(m => m.sharedMesh.vertexCount), Is.EqualTo(77 * 77 * 4));
                Assert.That(meshes.Min(m => m.sharedMesh.bounds.min.x), Is.EqualTo(i * 8 - 38.5f));
                Assert.That(meshes.Max(m => m.sharedMesh.bounds.max.x), Is.EqualTo(i * 8 + 38.5f));
                Assert.That(GroundCategory(meshes, new Int2(1, 0)), Is.EqualTo(1));
                Assert.That(GroundCategory(meshes, new Int2(2, 0)), Is.EqualTo(2));
                Assert.That(GroundCategory(meshes, new Int2(3, 0)), Is.EqualTo(3));
                Assert.That(GroundCategory(meshes, new Int2(28, 0)), Is.EqualTo(4), "Known missing terrain stays visible without generating it.");
                Assert.That(GroundCategory(meshes, new Int2(29, 0)), Is.EqualTo(0));
                Assert.That(world.Tiles.Count, Is.EqualTo(tileCount), "Camera movement cannot generate authoritative terrain.");
            }
            TestContext.WriteLine("Three expanded-terrain rebuilds, milliseconds: " + elapsed.ToString("F3", System.Globalization.CultureInfo.InvariantCulture));
        }

        private static int GroundCategory(MeshFilter[] meshes, Int2 point)
        {
            foreach (var filter in meshes)
            {
                var vertices = filter.sharedMesh.vertices;
                for (int i = 0; i < vertices.Length; i += 4)
                {
                    var midpoint = (vertices[i] + vertices[i + 2]) * .5f;
                    if (midpoint.x == point.X && midpoint.z == point.Z)
                        return int.Parse(filter.name.Substring("Ground ".Length));
                }
            }
            Assert.Fail("No terrain at " + point); return -1;
        }

        [UnityTest]
        public IEnumerator FarmRenderingCoversBothDesignatedCornersAndClearsAllCells()
        {
            var world = game.CurrentWorld; var village = game.Selected;
            var origin = new Int2(village.Center.X + village.Radius + 6, village.Center.Z);
            for (int x = village.Radius + 1; x <= village.Radius + 10; x++)
                for (int z = -4; z <= 3; z++)
                {
                    var at = new Int2(village.Center.X + x, village.Center.Z + z);
                    var tile = world.TileAt(at);
                    tile.Wall = tile.Water = tile.Mountain = tile.Road = tile.Rail = false;
                    tile.ClaimId = tile.Resource = ""; tile.Amount = 0;
                    if (!village.Known.Contains(at)) village.Known.Add(at);
                }
            village.Buildings.Add(new Building { Id = world.Id("fixture-field"), Kind = "field", Position = new Int2(origin.X, origin.Z - 4), Completed = true });
            var result = game.Send(new GameAction { Kind = "DesignateFarm", Resource = "grain", Position = origin, End = new Int2(origin.X + 3, origin.Z + 2) }).Result;
            Assert.That(result.Success, Is.True, result.Error);
            var farm = village.Farms.Single(f => f.Id == result.EntityId);
            Assert.That(farm.Width, Is.EqualTo(4)); Assert.That(farm.Depth, Is.EqualTo(3));
            yield return new WaitForSecondsRealtime(.25f);
            Transform[] Plots() => UnityEngine.Object.FindObjectsByType<Transform>(FindObjectsSortMode.None).Where(t => t.name.StartsWith("farm:" + farm.Id, StringComparison.Ordinal)).ToArray();
            var plots = Plots();
            Assert.That(plots, Is.Not.Empty, "The public farm must have a rendered crop before checking its full extent.");
            var visible = plots.SelectMany(t => t.GetComponentsInChildren<Renderer>()).Select(r => r.bounds).ToArray();
            for (int x = 0; x < farm.Width; x++) for (int z = 0; z < farm.Depth; z++)
            {
                float px = origin.X + x, pz = origin.Z + z;
                Assert.That(visible.Any(b => b.min.x <= px && b.max.x >= px && b.min.z <= pz && b.max.z >= pz), Is.True, "The crop footprint must visibly cover designated tile " + px + "," + pz);
            }
            Assert.That(game.Send(new GameAction { Kind = "ClearFarm", TargetId = farm.Id }).Result.Success, Is.True);
            yield return new WaitForSecondsRealtime(.25f);
            Assert.That(Plots(), Is.Empty, "Clearing a farm must remove its entire visible footprint.");
        }

        [UnityTest]
        public IEnumerator ExistingResourceViewChangesWhenInfrastructureReplacesIt()
        {
            var tile = game.CurrentWorld.Tiles.First(t => game.Selected.Known.Contains(t.Position) && t.Resource == "logs" && t.Amount > 0 && !t.Wall && !t.Road);
            yield return new WaitForSecondsRealtime(.25f);
            string name = "tile:" + tile.Position;
            Assert.That(GameObject.Find(name).transform.Find("tree_oak"), Is.Not.Null);
            tile.Road = true;
            yield return new WaitForSecondsRealtime(.25f);
            Assert.That(GameObject.Find(name).transform.Find("road"), Is.Not.Null, "A tile keeps its identity when its visible asset changes.");
        }

        [UnityTest]
        public IEnumerator ExactCraftedCargoAppearsOnlyAfterPickupAndFollowsItsCat()
        {
            var village = game.Selected;
            var cat = village.Cats.First(c => c.Alive && c.Id != village.LeaderId);
            var source = village.Stockpiles.First();
            var job = new Job { Id = "visible-item-haul", Kind = "haul", Phase = "item_fetch", CatId = cat.Id, SourceId = source.Id };
            var item = new Item { Id = "visible-mug", Kind = "mug", Material = "wood", LocationId = job.Id };
            job.ItemIds.Add(item.Id); village.Jobs.Add(job); village.Items.Add(item); cat.JobId = job.Id;
            yield return new WaitForSecondsRealtime(.25f);
            Assert.That(GameObject.Find("cargo:" + cat.Id), Is.Null, "A reserved item is still at its source before pickup.");
            game.View.InspectCat(cat.Id); game.UI.OpenPanel("Inspect");
            Assert.That(game.GetComponent<UIDocument>().rootVisualElement.Query<Label>().ToList().Any(label => label.text.StartsWith("Carried item:", StringComparison.Ordinal)), Is.False);
            job.Phase = "output_delivery";
            yield return new WaitForSecondsRealtime(.25f);
            var cargo = GameObject.Find("cargo:" + cat.Id);
            Assert.That(cargo, Is.Not.Null, "Finished exact goods must visibly travel with their carrier.");
            Assert.That(cargo.GetComponentsInChildren<MeshFilter>().Sum(m => m.sharedMesh.vertexCount), Is.GreaterThan(30), "Carried goods use authored geometry.");
            game.View.InspectCat(cat.Id); game.UI.OpenPanel("Inspect");
            Assert.That(game.GetComponent<UIDocument>().rootVisualElement.Query<Label>().ToList().Any(label => label.text.StartsWith("Carried item:", StringComparison.Ordinal) && label.text.Contains(item.Id)), Is.True, "Inspection must identify the same carried exact item.");
            var before = cargo.transform.position;
            cat.X += 1;
            yield return new WaitForSecondsRealtime(.25f);
            Assert.That(cargo.transform.position.x, Is.GreaterThan(before.x + .5f));
            item.LocationId = source.Id; job.ItemIds.Clear(); job.Completed = true; cat.JobId = "";
            yield return new WaitForSecondsRealtime(.25f);
            Assert.That(GameObject.Find("cargo:" + cat.Id), Is.Null, "Delivered items cannot remain on the cat.");
            game.UI.OpenPanel("Inspect");
            Assert.That(game.GetComponent<UIDocument>().rootVisualElement.Query<Label>().ToList().Any(label => label.text.StartsWith("Carried item:", StringComparison.Ordinal)), Is.False);
        }

        [UnityTest]
        public IEnumerator InspectButtonControlsTheSameCatAndRestoresManagement()
        {
            var cat = game.Selected.Cats.First(c => c.Alive && c.Id != game.Selected.LeaderId);
            cat.Cargo.Add(new Simulation.Stack("food", 1));
            game.View.InspectCat(cat.Id);
            game.UI.OpenPanel("Inspect");
            Submit("Control this cat · Tab");
            yield return null;
            Assert.That(game.View.ControlledCat, Is.SameAs(cat));
            Assert.That(game.View.Camera.orthographic, Is.False);
            Assert.That(cat.Cargo.Single(s => s.Resource == "food").Amount, Is.EqualTo(1));
            game.UI.OpenPanel("Inspect");
            Submit("Return to management");
            yield return null;
            Assert.That(cat.ControlledBy, Is.Empty);
            Assert.That(game.View.Camera.orthographic, Is.True);
            Assert.That(game.Selected.Cats.Count(c => c.Id == cat.Id), Is.EqualTo(1));
        }

        [UnityTest]
        public IEnumerator ResearchControlsSpendPointsAndDisplayTheDependencyMap()
        {
            var study = Catalog.Research.First(n => n.Prerequisites.Count == 0 && !game.Selected.Research.Contains(n.Id));
            game.Selected.ResearchPoints = study.Cost + 17;
            game.UI.OpenPanel("Research");
            Submit("Research " + study.Name);
            yield return null;
            CollectionAssert.Contains(game.Selected.Research, study.Id);
            Assert.That(game.Selected.ResearchPoints, Is.EqualTo(17).Within(.000001));
            Submit("Dependency map");
            yield return null;
            var maps = game.GetComponent<UIDocument>().rootVisualElement.Query<ScrollView>().ToList();
            Assert.That(maps.Any(view => view.mode == ScrollViewMode.VerticalAndHorizontal), Is.True);
            Assert.That(game.GetComponent<UIDocument>().rootVisualElement.Query<Button>().ToList().Any(button => button.text.StartsWith("Requires ", StringComparison.Ordinal)), Is.True);
        }

        [UnityTest]
        public IEnumerator ManualLoggingButtonCreatesSupportedPhysicalWork()
        {
            game.UI.OpenPanel("Work");
            Submit("woodcut");
            yield return null;
            Assert.That(game.Selected.Jobs.Any(j=>j.Kind=="logs"&&!j.Completed),Is.True,game.LastAction);
        }

        [UnityTest]
        public IEnumerator RestoredStationInspectorUsesTheAuthoritativeSlotQueue()
        {
            var station=game.Selected.Buildings.First(b=>b.Kind=="wood_cutter");
            var cat=game.Selected.Cats.First(c=>c.Alive&&c.JobId==""&&c.BuildingId=="");
            Assert.That(game.Send(new GameAction{Kind="AssignWorker",BuildingId=station.Id,CatId=cat.Id}).Result.Success,Is.True);
            station.Queue.Add(new QueueEntry{RecipeId="logs_to_planks"});
            var restored=Authority.WireJson.Clone(station);
            restored.Slots[0].Queue.Clear();
            game.Selected.Buildings[game.Selected.Buildings.IndexOf(station)]=restored;
            game.View.InspectBuilding(station.Id);game.UI.OpenPanel("Inspect");
            yield return null;
            Assert.That(game.GetComponent<UIDocument>().rootVisualElement.Query<Label>().ToList().Any(label=>label.text=="1. logs to planks"),Is.False,"A stale building queue must not hide the real empty first-slot queue.");
        }

        [UnityTest]
        public IEnumerator StationaryDirectControlSurvivesEightTimesSpeed()
        {
            var cat=game.Selected.Cats.First(c=>c.Alive&&c.Id!=game.Selected.LeaderId);
            game.View.InspectCat(cat.Id);game.View.EnterSelectedCat();
            game.SetSpeed(8);
            yield return new WaitForSecondsRealtime(4.15f);
            Assert.That(cat.ControlledBy,Is.EqualTo(game.PlayerId),"Heartbeat must follow simulation speed.");
        }

        private void Submit(string text)
        {
            var button = game.GetComponent<UIDocument>().rootVisualElement.Query<Button>().ToList().Single(b => b.text == text);
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = button; button.SendEvent(submit); }
        }

        [UnityTest]
        public IEnumerator MerchantSellingControlsTransferTheSelectedStoredItem()
        {
            var item = new Item { Id = "review-sale", Kind = "tool", Material = "wood", VillageId = game.Selected.Id, LocationId = game.Selected.Stockpiles[0].Id };
            game.Selected.Items.Add(item);
            game.Selected.Trader.Phase = "trading"; game.Selected.Trader.Coins = 100;
            double coins = game.Selected.Coins;
            game.UI.OpenPanel("Trade"); Submit("Sell goods");
            yield return null;
            Assert.That(game.Selected.Trader.Items.Contains(item), Is.True, game.LastAction);
            Assert.That(game.Selected.Items.Contains(item), Is.False);
            Assert.That(game.Selected.Coins, Is.GreaterThan(coins));
        }

        [UnityTest]
        public IEnumerator VillagePanelTracksAuthoritativeSelectionChanges()
        {
            game.UI.OpenPanel("Village");
            Assert.That(game.Send(new GameAction { Kind = "FoundVillage", Name = "New Grove" }).Result.Success, Is.True);
            yield return new WaitForSecondsRealtime(.6f);
            var buttons = game.GetComponent<UIDocument>().rootVisualElement.Query<Button>().ToList();
            Assert.That(buttons.Any(b => b.text == "New Grove · selected"), Is.True, "Network-driven selection changes must refresh the open panel.");
        }

        [UnityTest]
        public IEnumerator WorkBoostControlsTargetTheDisplayedJob()
        {
            game.UI.OpenPanel("Work"); Submit("woodcut");
            var job = game.Selected.Jobs.Single(j => j.Kind == "logs" && !j.Completed);
            Submit("Boost logs · " + job.Id);
            yield return null;
            Assert.That(job.BoostCount, Is.EqualTo(1), game.LastAction);
        }

        [UnityTest]
        public IEnumerator StockpileDesignationRetainsBothSelectedCorners()
        {
            game.UI.OpenPanel("Stores"); Submit("Designate general stockpile");
            var place = typeof(ForestUI).GetMethod("Place", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic);
            place.Invoke(game.UI, new object[] { new Int2(-2, -6) });
            Assert.That(game.UI.HasPlacement, Is.True, "The first click chooses the first corner.");
            place.Invoke(game.UI, new object[] { new Int2(-1, -5) });
            yield return null;
            Assert.That(game.LastAction, Does.Contain("accepted"));
            var pile = game.Selected.Stockpiles.Single(p => p.Position.Equals(new Int2(-2, -6)));
            Assert.That(pile.Width, Is.EqualTo(2)); Assert.That(pile.Depth, Is.EqualTo(2));
        }

        [UnityTest]
        public IEnumerator AllBlessingUnlocksHaveReachableControls()
        {
            game.Selected.Research.Clear(); game.UI.OpenPanel("Shrine");
            yield return null;
            var buttons = game.GetComponent<UIDocument>().rootVisualElement.Query<Button>().ToList();
            foreach (var study in Catalog.Research.Take(24))
                Assert.That(buttons.Any(b => b.text.StartsWith("Blessing: " + study.Name + " · ", StringComparison.Ordinal)), Is.True, study.Id);
        }

        [UnityTest]
        public IEnumerator VacantSecondarySlotDoesNotOfferFirstSlotEdits()
        {
            var station = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            station.Slots.Add(new WorkSlot { CatId = "" });
            station.Slots.Add(new WorkSlot { CatId = "", Queue = new System.Collections.Generic.List<QueueEntry> { new QueueEntry { RecipeId = "logs_to_planks" } } });
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            yield return null;
            var buttons = game.GetComponent<UIDocument>().rootVisualElement.Query<Button>().ToList();
            Assert.That(buttons.Count(b => b.text == "Remove"), Is.EqualTo(0), "An unstaffed secondary queue cannot route edits to the first slot.");
        }
    }
}
