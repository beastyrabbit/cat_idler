using System;
using System.Collections;
using System.IO;
using System.Linq;
using System.Reflection;
using IdleCatForest.Authority;
using IdleCatForest.Presentation;
using IdleCatForest.Simulation;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.UIElements;

namespace IdleCatForest.Tests
{
    public class ForestWorkshopRepeatTests
    {
        private ForestGame game;
        private string directory;
        private VisualElement Root => game.GetComponent<UIDocument>().rootVisualElement;

        [UnitySetUp]
        public IEnumerator StartIsolatedWorld()
        {
            Assert.That(ForestGame.Instance, Is.Null, "Never replace a running player's world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-workshop-repeat-" + Guid.NewGuid().ToString("N"));
            game = new GameObject("Isolated workshop Repeat test").AddComponent<ForestGame>();
            game.InitialSavePath = Path.Combine(directory, "world.json");
            yield return null;
            Assert.That(game.Selected, Is.Not.Null, game.Status);
            game.SetSpeed(0);
            Root.style.flexGrow = 0; Root.style.width = 1000; Root.style.maxWidth = 1000;
            Root.style.height = 800; Root.style.maxHeight = 800;
        }

        [UnityTearDown]
        public IEnumerator CloseIsolatedWorld()
        {
            if (game != null) UnityEngine.Object.Destroy(game.gameObject);
            yield return null;
            if (directory != null && Directory.Exists(directory)) Directory.Delete(directory, true);
        }

        [UnityTest]
        public IEnumerator FutureRepeatKeepsItsViewportPositionWhenItsWarningAppears()
        {
            var station = PrepareTwoWorkers();
            var first = station.Slots[0]; var second = station.Slots[1];
            var firstOrders = first.Queue.ToArray(); var secondOrders = second.Queue.ToArray();
            var target = secondOrders[1];
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            yield return null; yield return null;
            var drawer = Root.Q<ScrollView>(className: "drawer-scroll");
            var repeat = Repeat(second.CatId, 1);
            drawer.ScrollTo(repeat); repeat.Focus();
            yield return null; yield return null;
            drawer.ScrollTo(Repeat(second.CatId, 1));
            yield return null;
            repeat = Repeat(second.CatId, 1);
            Assert.That(drawer.scrollOffset.y, Is.GreaterThan(100), "Reach a future order in the second worker's deep inspector.");
            AssertVisible(drawer, repeat);
            Assert.That(Position(second.CatId).Query<Label>().ToList().Any(l => l.text == "Repeating orders continue until you change them."), Is.False);
            float initialY = repeat.worldBound.yMin;
            Submit(repeat);
            yield return null; yield return null; yield return null;

            Assert.That(target.Repeat, Is.True, game.LastAction);
            Assert.That(secondOrders[0].Repeat || secondOrders[2].Repeat, Is.False, "Only the selected future order may repeat.");
            CollectionAssert.AreEqual(firstOrders, first.Queue);
            Assert.That(first.Queue.All(q => !q.Repeat), Is.True, "The other worker's orders must stay finite.");
            CollectionAssert.AreEqual(secondOrders, second.Queue, "A repeat toggle must preserve every order's identity and queue position.");
            Assert.That(Position(second.CatId).Query<Label>().ToList().Any(l => l.text == "Repeating orders continue until you change them."), Is.True);
            var replacement = Repeat(second.CatId, 1);
            Assert.That(replacement.worldBound.yMin, Is.EqualTo(initialY).Within(2),
                "Revealing repeat guidance must keep the exact future-order control under the player's pointer.");
            Assert.That(Root.panel.focusController.focusedElement, Is.SameAs(replacement), "The rebuilt Repeat control must retain focus for a second activation.");
        }

        [UnityTest]
        public IEnumerator FocusedFutureRepeatSurvivesLiveRefreshAndFollowsItsExactOrder()
        {
            var station = PrepareTwoWorkers();
            var first = station.Slots[0]; var second = station.Slots[1];
            var firstOrders = first.Queue.ToArray(); var secondOrders = second.Queue.ToArray();
            var target = secondOrders[1];
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            yield return null; yield return null;
            var repeat = Repeat(second.CatId, 1);
            Root.Q<ScrollView>(className: "drawer-scroll").ScrollTo(repeat); repeat.Focus();
            Assert.That(Root.panel.focusController.focusedElement, Is.SameAs(repeat));
            // Another accepted queue edit moves the same-recipe future order before the next UI refresh.
            Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = second.CatId, Edit = "move", Index = 2, Direction = -1 }).Result.Success, Is.True);
            double before = game.CurrentWorld.TimeSeconds;
            game.SetSpeed(1);
            yield return new WaitForSecondsRealtime(.6f);
            Assert.That(game.CurrentWorld.TimeSeconds, Is.GreaterThan(before), "Run actual Update-driven simulation and inspector refresh.");
            Assert.That(second.Queue[2], Is.SameAs(target), "The selected exact order must now be third, despite its identical recipe.");
            var focused = Root.panel.focusController.focusedElement as Button;
            Assert.That(focused, Is.Not.Null, "Live inspector refresh must retain the focused future Repeat action.");
            Assert.That(focused, Is.SameAs(Repeat(second.CatId, 2)), "Focus must follow the order, rather than whichever order inherits its index.");
            Submit(focused);

            Assert.That(target.Repeat, Is.True, game.LastAction);
            Assert.That(secondOrders[0].Repeat || secondOrders[2].Repeat, Is.False, "Same-recipe neighbours must stay finite.");
            CollectionAssert.AreEqual(firstOrders, first.Queue);
            Assert.That(first.Queue.All(q => !q.Repeat), Is.True, "No action may leak into another cat's queue.");
            Assert.That(second.Queue, Has.Count.EqualTo(3));
        }

        [UnityTest]
        public IEnumerator RemovedFutureRepeatCannotToggleItsSameRecipeNeighbourBeforeRefresh()
        {
            var station = PrepareTwoWorkers();
            var first = station.Slots[0]; var second = station.Slots[1];
            var firstOrders = first.Queue.ToArray(); var secondOrders = second.Queue.ToArray();
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            yield return null; yield return null;
            var repeat = Repeat(second.CatId, 1);
            Root.Q<ScrollView>(className: "drawer-scroll").ScrollTo(repeat); repeat.Focus();
            Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = second.CatId, Edit = "remove", Index = 1 }).Result.Success, Is.True);
            Assert.That(repeat.panel, Is.SameAs(Root.panel), "The pre-refresh control is still attached when the queue changes.");
            Submit(repeat);

            Assert.That(secondOrders[2].Repeat, Is.False,
                "An attached control for a removed order must not toggle the same-recipe neighbour that inherits its index.");
            Assert.That(secondOrders[0].Repeat || secondOrders[1].Repeat, Is.False);
            CollectionAssert.AreEqual(new[] { secondOrders[0], secondOrders[2] }, second.Queue);
            CollectionAssert.AreEqual(firstOrders, first.Queue);
            Assert.That(first.Queue.All(q => !q.Repeat), Is.True);
        }

        [UnityTest]
        public IEnumerator RemoteRepeatStillForwardsAfterAnEquivalentSnapshotReplacesItsObjects()
        {
            var station = PrepareTwoWorkers();
            string catId = station.Slots[1].CatId;
            var frameA = new WorldFrame { Sequence = 1, World = WireJson.Clone(game.CurrentWorld), SelectedVillageId = game.SelectedVillageId };
            frameA.ControlledVillageIds.Add(frameA.SelectedVillageId);
            var frameB = WireJson.Clone(frameA); frameB.Sequence = 2;
            var latest = typeof(WorldClient).GetField("latest", BindingFlags.Instance | BindingFlags.NonPublic);
            var remote = typeof(ForestGame).GetField("remote", BindingFlags.Instance | BindingFlags.NonPublic);
            Assert.That(latest, Is.Not.Null); Assert.That(remote, Is.Not.Null);
            // Construction does not connect or load credentials. This fixture proves forwarding, not socket acceptance.
            var client = new WorldClient("ws://127.0.0.1:1/ws", Path.Combine(directory, "unused-remote-credential.json"));
            try
            {
                latest.SetValue(client, frameA); remote.SetValue(game, client);
                game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
                yield return null; yield return null;
                var repeat = Repeat(catId, 1);
                Assert.That(repeat.panel, Is.SameAs(Root.panel));
                Assert.That(frameB.World.Villages.Single(v => v.Id == frameB.SelectedVillageId),
                    Is.Not.SameAs(frameA.World.Villages.Single(v => v.Id == frameA.SelectedVillageId)));
                latest.SetValue(client, frameB);
                // Submit the still-attached control before any inspector refresh can replace it.
                Submit(repeat);
                Assert.That(game.LastAction, Is.EqualTo("Connect to a shared world first."),
                    "An equivalent snapshot must not silently discard an ordinary queue click before normal Send dispatch.");
            }
            finally
            {
                remote.SetValue(game, null); client.Dispose();
            }
        }

        private Building PrepareTwoWorkers()
        {
            var village = game.Selected;
            var station = village.Buildings.First(b => b.Kind == "wood_cutter");
            village.Research.Add("wood_cutter_crews");
            var cats = village.Cats.Where(c => c.Alive && c.AgeHours >= 12 && c.JobId == "" && c.BuildingId == "" && c.Id != village.LeaderId).Take(2).ToArray();
            Assert.That(cats.Length, Is.EqualTo(2));
            foreach (var cat in cats)
                Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            for (int worker = 0; worker < cats.Length; worker++)
                for (int order = 0; order < (worker == 0 ? 6 : 3); order++)
                    Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = cats[worker].Id, Edit = "add", RecipeId = "logs_to_planks" }).Result.Success, Is.True);
            return station;
        }

        private VisualElement Position(string catId)
        {
            string name = game.Selected.Cats.Single(c => c.Id == catId).Name;
            return Root.Query<VisualElement>(className: "work-position").ToList()
                .Single(p => p.Query<Button>().ToList().Any(b => b.text == "Release worker · " + name));
        }

        private Button Repeat(string catId, int index) => Position(catId).Query<VisualElement>(className: "queue-entry").ToList()[index]
            .Query<Button>().ToList().Single(b => b.text.StartsWith("Repeat ", StringComparison.Ordinal));

        private static void Submit(Button button)
        {
            Assert.That(button.enabledInHierarchy, Is.True);
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = button; button.SendEvent(submit); }
        }

        private static void AssertVisible(ScrollView drawer, Button button)
        {
            Assert.That(button.panel, Is.SameAs(drawer.panel));
            var viewport = drawer.contentViewport.worldBound;
            Assert.That(button.worldBound.yMin, Is.GreaterThanOrEqualTo(viewport.yMin - 1));
            Assert.That(button.worldBound.yMax, Is.LessThanOrEqualTo(viewport.yMax + 1), "The Repeat control must be fully visible before activation.");
        }
    }
}
