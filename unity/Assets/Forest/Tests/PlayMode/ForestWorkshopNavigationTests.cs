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
    public class ForestWorkshopNavigationTests
    {
        private ForestGame game;
        private string directory;
        private VisualElement Root => game.GetComponent<UIDocument>().rootVisualElement;

        [UnitySetUp]
        public IEnumerator StartIsolatedWorld()
        {
            Assert.That(ForestGame.Instance, Is.Null, "Never replace a running player's world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-workshop-navigation-" + Guid.NewGuid().ToString("N"));
            game = new GameObject("Isolated workshop navigation test").AddComponent<ForestGame>();
            game.InitialSavePath = Path.Combine(directory, "world.json");
            yield return null;
            Assert.That(game.Selected, Is.Not.Null, game.Status);
            game.SetSpeed(0);
            Root.style.flexGrow = 0; Root.style.width = Root.style.maxWidth = 1000;
            Root.style.height = Root.style.maxHeight = 620;
        }

        [UnityTearDown]
        public IEnumerator CloseIsolatedWorld()
        {
            if (game != null) UnityEngine.Object.Destroy(game.gameObject);
            yield return null;
            if (directory != null && Directory.Exists(directory)) Directory.Delete(directory, true);
        }

        [UnityTest]
        public IEnumerator FixedWorkshopDestinationsReachExactSecondWorkerWorkQueueAndInputs()
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
            var first = station.Slots.Single(s => s.CatId == cats[0].Id);
            var second = station.Slots.Single(s => s.CatId == cats[1].Id);
            var firstOrders = first.Queue.ToArray(); var secondOrders = second.Queue.ToArray();
            var job = new Job { Id = "ui-navigation-current-batch", Kind = "production", TargetId = station.Id, CatId = cats[1].Id, RecipeId = "logs_to_planks", Phase = "working", Progress = 51.3, RequiredWork = 600 };
            village.Jobs.Add(job); cats[1].JobId = job.Id;
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            yield return null; yield return null;

            var drawer = Root.Q<ScrollView>(className: "drawer-scroll");
            var navigation = Root.Q<DropdownField>("workshop-navigation");
            Assert.That(navigation, Is.Not.Null,
                "A long workshop inspector needs a fixed destination control for current work, ownership, finite queues and ingredient reports.");
            Assert.That(drawer.Contains(navigation), Is.False, "The destination control must remain reachable while the work positions scroll.");
            string Destination(string section) => "Position 2 · " + cats[1].Name + " · " + section;
            foreach (var section in new[] { "Work & controls", "Queue", "Inputs" })
                Assert.That(navigation.choices, Does.Contain(Destination(section)), "Each destination must identify the exact numbered worker.");
            navigation.value = Destination("Queue");
            yield return null; yield return null;
            Assert.That(drawer.scrollOffset.y, Is.GreaterThan(100), "The second worker must be reached through a genuinely deep inspector.");
            AssertVisible(drawer, SecondPosition(cats[1]).Query<Button>().ToList().Single(b => b.text == "Pause"));
            AssertVisible(drawer, SecondPosition(cats[1]).Query<VisualElement>(className: "queue-entry").ToList().First().Q<Label>());
            Assert.That(navigation.worldBound.yMax, Is.LessThanOrEqualTo(drawer.contentViewport.worldBound.yMin), "Go to must remain fixed above the scrolling viewport.");

            navigation.value = Destination("Work & controls");
            yield return null; yield return null;
            var position = SecondPosition(cats[1]);
            AssertVisible(drawer, position.Q<Label>(className: "worker-status"));
            AssertVisible(drawer, position.Q<ProgressBar>("live-work-progress"));
            AssertVisible(drawer, position.Query<Button>().ToList().Single(b => b.text == "Release worker · " + cats[1].Name));
            Assert.That(position.Q<ProgressBar>("live-work-progress").value, Is.EqualTo(51.3f).Within(.01f));

            navigation.value = Destination("Inputs");
            yield return null; yield return null;
            AssertVisible(drawer, SecondPosition(cats[1]).Q<VisualElement>(className: "recipe-supply").Q<Label>(className: "subtitle"));
            // A jump remains an action after the player scrolls away from that same destination.
            drawer.scrollOffset = Vector2.zero;
            yield return null;
            navigation.value = Destination("Inputs");
            yield return null; yield return null;
            AssertVisible(drawer, SecondPosition(cats[1]).Q<VisualElement>(className: "recipe-supply").Q<Label>(className: "subtitle"));
            CollectionAssert.AreEqual(firstOrders, first.Queue, "Navigation must preserve every other worker's exact orders.");
            CollectionAssert.AreEqual(secondOrders, second.Queue, "Navigation must preserve the selected worker's exact finite orders.");
            Assert.That(first.Queue.Concat(second.Queue).All(q => !q.Repeat && q.AutomatedBy == ""), Is.True);
            Assert.That(first.AutomatedBy == "" && second.AutomatedBy == "" && !first.Paused && !second.Paused, Is.True,
                "Choosing a destination must not take, delegate or pause either position.");
            Assert.That(job.Progress, Is.EqualTo(51.3));
        }

        [UnityTest]
        public IEnumerator VacantStationQueueAndResearchWorkHaveOnlyTheirAvailableDestinations()
        {
            var village = game.Selected;
            var station = village.Buildings.First(b => b.Kind == "wood_cutter");
            for (int order = 0; order < 3; order++)
                Assert.That(game.Send(new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = "logs_to_planks" }).Result.Success, Is.True);
            var orders = station.Queue.ToArray();
            Assert.That(station.Slots, Is.Empty, "An unstaffed station retains its queue before a worker slot exists.");
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            yield return null; yield return null;
            var drawer = Root.Q<ScrollView>(className: "drawer-scroll");
            var navigation = Root.Q<DropdownField>("workshop-navigation");
            Assert.That(navigation.choices, Does.Contain("Position 1 · Vacant · Queue"));
            navigation.value = "Position 1 · Vacant · Queue";
            yield return null; yield return null;
            AssertVisible(drawer, Root.Query<Button>().ToList().Single(b => b.text == "Pause"));
            AssertVisible(drawer, Root.Q<VisualElement>(className: "queue-entry").Q<Label>());
            CollectionAssert.AreEqual(orders, station.Queue);
            Assert.That(station.Slots, Is.Empty, "Navigating a vacant position must not create a worker slot.");

            var hut = village.Buildings.First(b => b.Kind == "research_hut");
            var cat = village.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "" && c.Id != village.LeaderId);
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = hut.Id, CatId = cat.Id }).Result.Success, Is.True);
            game.View.InspectBuilding(hut.Id); game.UI.OpenPanel("Inspect");
            yield return null; yield return null;
            navigation = Root.Q<DropdownField>("workshop-navigation");
            Assert.That(navigation.choices, Does.Contain("Staffing"));
            Assert.That(navigation.choices.Any(c => c.EndsWith(" · Queue", StringComparison.Ordinal) || c.EndsWith(" · Inputs", StringComparison.Ordinal)), Is.False,
                "Research work must not advertise empty production queues or ingredient bills.");
            navigation.value = "Position 1 · " + cat.Name + " · Work & controls";
            yield return null; yield return null;
            AssertVisible(drawer, Root.Query<Button>().ToList().Single(b => b.text == "Pause work"));
            Assert.That(hut.Slots.Single().CatId, Is.EqualTo(cat.Id));
            Assert.That(hut.Slots.Single().Paused, Is.False);
        }

        [UnityTest]
        public IEnumerator DetachedWorkshopDestinationCannotScrollANewlySelectedSubject()
        {
            var village = game.Selected;
            var station = village.Buildings.First(b => b.Kind == "wood_cutter");
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            yield return null; yield return null;
            var previousNavigation = Root.Q<DropdownField>("workshop-navigation");
            Assert.That(previousNavigation, Is.Not.Null);
            var hut = village.Buildings.First(b => b.Kind == "research_hut");
            game.View.InspectBuilding(hut.Id); game.UI.OpenPanel("Inspect");
            yield return null; yield return null;
            var drawer = Root.Q<ScrollView>(className: "drawer-scroll");
            var currentNavigation = Root.Q<DropdownField>("workshop-navigation");
            Assert.That(currentNavigation, Is.Not.SameAs(previousNavigation), "A destination callback must retain the identity of its original workshop.");
            Assert.That(previousNavigation.panel, Is.Null);
            var before = drawer.scrollOffset;
            previousNavigation.value = "Position 1 · Vacant · Queue";
            yield return null;
            Assert.That(game.View.SelectedBuilding.Id, Is.EqualTo(hut.Id));
            Assert.That(drawer.scrollOffset, Is.EqualTo(before), "A stale workshop choice must not scroll a different selected building.");

            var cat = village.Cats.First(c => c.Alive);
            game.View.InspectCat(cat.Id); game.UI.OpenPanel("Inspect");
            yield return null; yield return null;
            before = drawer.scrollOffset;
            currentNavigation.value = "Staffing";
            yield return null;
            Assert.That(game.View.SelectedCat.Id, Is.EqualTo(cat.Id));
            Assert.That(drawer.scrollOffset, Is.EqualTo(before), "A hidden workshop choice must not move a cat inspector.");
            Assert.That(currentNavigation.resolvedStyle.display, Is.Not.EqualTo(DisplayStyle.None), "The subject guard must work even when only the chooser's parent is hidden.");
            Assert.That(currentNavigation.parent.resolvedStyle.display, Is.EqualTo(DisplayStyle.None));
        }

        [UnityTest]
        public IEnumerator FixedNavigationCoexistsWithLiveRepeatFocusAndWorkerBack()
        {
            var village = game.Selected;
            var station = village.Buildings.First(b => b.Kind == "wood_cutter");
            var cat = village.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "" && c.Id != village.LeaderId);
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            for (int order = 0; order < 6; order++)
                Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = cat.Id, Edit = "add", RecipeId = "logs_to_planks" }).Result.Success, Is.True);
            var orders = station.Slots.Single().Queue.ToArray();
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            yield return null; yield return null;
            var drawer = Root.Q<ScrollView>(className: "drawer-scroll");
            var navigation = Root.Q<DropdownField>("workshop-navigation");
            navigation.value = "Position 1 · " + cat.Name + " · Queue";
            yield return null; yield return null;
            var repeat = Root.Query<VisualElement>(className: "queue-entry").ToList()[2].Query<Button>().ToList().Single(b => b.text == "Repeat off");
            drawer.ScrollTo(repeat); repeat.Focus();
            yield return null; yield return null;
            string exactOrderControl = repeat.name;
            float initialY = repeat.worldBound.yMin;
            double time = game.CurrentWorld.TimeSeconds;
            game.SetSpeed(1);
            yield return new WaitForSecondsRealtime(.6f);
            game.SetSpeed(0);
            Assert.That(game.CurrentWorld.TimeSeconds, Is.GreaterThan(time));
            Assert.That(Root.Q<DropdownField>("workshop-navigation"), Is.SameAs(navigation), "Refreshing the same workshop must retain its fixed chooser.");
            var focused = Root.panel.focusController.focusedElement as Button;
            Assert.That(focused?.name, Is.EqualTo(exactOrderControl), "The new fixed control must preserve existing exact-order keyboard focus.");
            Assert.That(focused.worldBound.yMin, Is.EqualTo(initialY).Within(2));
            CollectionAssert.AreEqual(orders, station.Slots.Single().Queue);

            navigation.value = "Position 1 · " + cat.Name + " · Work & controls";
            yield return null; yield return null;
            float offset = drawer.scrollOffset.y;
            Assert.That(offset, Is.GreaterThan(100));
            Submit(Root.Query<Button>().ToList().Single(b => b.text == "Inspect worker · " + cat.Name));
            yield return null; yield return null;
            Submit(Root.Query<Button>().ToList().Single(b => b.text == "Back"));
            yield return null; yield return null;
            Assert.That(game.View.SelectedBuilding?.Id, Is.EqualTo(station.Id));
            Assert.That(drawer.scrollOffset.y, Is.EqualTo(offset).Within(.5f), "Back from the worker must retain the workshop destination's viewport.");
            CollectionAssert.AreEqual(orders, station.Slots.Single().Queue);
        }

        [UnityTest]
        public IEnumerator FocusedFixedDestinationKeepsPeriodicWorkerStatusFresh()
        {
            var village = game.Selected;
            var station = village.Buildings.First(b => b.Kind == "wood_cutter");
            var cat = village.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "" && c.Id != village.LeaderId);
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = cat.Id, Edit = "add", RecipeId = "logs_to_planks" }).Result.Success, Is.True);
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            yield return null; yield return null;
            var navigation = Root.Q<DropdownField>("workshop-navigation");
            Assert.That(navigation, Is.Not.Null);
            navigation.Focus();
            Assert.That(Root.panel.focusController.focusedElement, Is.SameAs(navigation));
            Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = cat.Id, Edit = "pause", Enabled = true }).Result.Success, Is.True);
            Assert.That(Root.Q<Label>(className: "worker-status").text, Does.Not.Contain("Paused"), "The displayed status must begin stale so periodic refresh is observable.");
            yield return new WaitForSecondsRealtime(.6f);
            Assert.That(Root.Q<Label>(className: "worker-status").text, Does.Contain("Paused"),
                "Leaving focus on the fixed destination chooser must not freeze worker status or completed queues.");
            Assert.That(Root.Q<DropdownField>("workshop-navigation"), Is.SameAs(navigation));
            Assert.That(Root.panel.focusController.focusedElement, Is.SameAs(navigation), "Refreshing content must retain the fixed navigation control's keyboard focus.");
        }

        private VisualElement SecondPosition(Cat cat) => Root.Query<VisualElement>(className: "work-position").ToList()
            .Single(p => p.Query<Button>().ToList().Any(b => b.text == "Release worker · " + cat.Name));

        private static void Submit(Button button)
        {
            Assert.That(button.enabledInHierarchy, Is.True);
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = button; button.SendEvent(submit); }
        }

        private static void AssertVisible(ScrollView drawer, VisualElement element)
        {
            Assert.That(element, Is.Not.Null);
            Assert.That(element.panel, Is.SameAs(drawer.panel));
            var viewport = drawer.contentViewport.worldBound;
            Assert.That(element.worldBound.yMin, Is.GreaterThanOrEqualTo(viewport.yMin - 1));
            Assert.That(element.worldBound.yMax, Is.LessThanOrEqualTo(viewport.yMax + 1), "The selected destination must appear inside the inspector viewport.");
        }
    }
}
