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
    public class ForestWorkshopContextTests
    {
        private ForestGame game;
        private string directory;
        private Building station;
        private Cat worker;
        private VisualElement Root => game.GetComponent<UIDocument>().rootVisualElement;
        private string WorkDestination => "Position 2 · " + worker.Name + " · Work & controls";

        [UnitySetUp]
        public IEnumerator StartIsolatedWorkshop()
        {
            Assert.That(ForestGame.Instance, Is.Null, "Never replace a running player's world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-workshop-context-" + Guid.NewGuid().ToString("N"));
            game = new GameObject("Isolated workshop context test").AddComponent<ForestGame>();
            game.InitialSavePath = Path.Combine(directory, "world.json");
            yield return null;
            Assert.That(game.Selected, Is.Not.Null, game.Status);
            game.SetSpeed(0);
            Root.style.flexGrow = 0; Root.style.width = Root.style.maxWidth = 1000;
            Root.style.height = Root.style.maxHeight = 620;

            var village = game.Selected;
            station = village.Buildings.First(b => b.Kind == "wood_cutter");
            village.Research.Add("wood_cutter_crews");
            village.Research.Add("sawmill");
            village.Buildings.Add(new Building { Id = "ui-context-sawmill", Kind = "sawmill", Completed = true });
            var cats = village.Cats.Where(c => c.Alive && c.AgeHours >= 12 && c.JobId == "" && c.BuildingId == "" && c.Id != village.LeaderId).Take(3).ToArray();
            Assert.That(cats.Length, Is.EqualTo(3));
            worker = cats[1]; worker.Name = "Textile Worker";
            Assert.That(game.Send(new GameAction { Kind = "AssignOfficer", Role = "forester", CatId = cats[2].Id }).Result.Success, Is.True);
            foreach (var cat in cats.Take(2))
                Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            for (int i = 0; i < 6; i++)
                Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = cats[0].Id, Edit = "add", RecipeId = "logs_to_planks" }).Result.Success, Is.True);
            for (int i = 0; i < 2; i++)
                Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "add", RecipeId = "logs_to_planks" }).Result.Success, Is.True);
            var job = new Job { Id = "ui-context-current-batch", Kind = "production", TargetId = station.Id, CatId = worker.Id, RecipeId = "logs_to_planks", Phase = "working", Progress = 51.3, RequiredWork = 600 };
            village.Jobs.Add(job); worker.JobId = job.Id;
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            yield return null; yield return null;
        }

        [UnityTearDown]
        public IEnumerator CloseIsolatedWorld()
        {
            if (game != null) UnityEngine.Object.Destroy(game.gameObject);
            yield return null;
            if (directory != null && Directory.Exists(directory)) Directory.Delete(directory, true);
        }

        [UnityTest]
        public IEnumerator DestinationMenuShowsTheEntireNumberedWorkerAndSection()
        {
            var navigation = Root.Q<DropdownField>("workshop-navigation");
            Assert.That(navigation.choices, Does.Contain(WorkDestination));
            Submit(navigation);
            yield return null; yield return null;
            var menu = Root.panel.visualTree.Q<VisualElement>(className: GenericDropdownMenu.ussClassName);
            Assert.That(menu, Is.Not.Null, "Open the real destination popup, rather than measuring a detached label.");
            var destination = menu.Query<Label>().ToList().Single(l => l.text == WorkDestination);
            float textWidth = destination.MeasureTextSize(destination.text, 0, VisualElement.MeasureMode.Undefined, 0, VisualElement.MeasureMode.Undefined).x;
            Assert.That(destination.contentRect.width + 1, Is.GreaterThanOrEqualTo(textWidth),
                "The full position, cat name and Work & controls section must fit without horizontal clipping.");
            Assert.That(destination.worldBound.xMin, Is.GreaterThanOrEqualTo(Root.worldBound.xMin - 1));
            Assert.That(destination.worldBound.xMax, Is.LessThanOrEqualTo(Root.worldBound.xMax + 1));
            var viewport = menu.Q<ScrollView>()?.contentViewport;
            Assert.That(viewport, Is.Not.Null, "Measure the real menu's clipping viewport.");
            var clip = viewport.LocalToWorld(viewport.contentRect);
            Assert.That(destination.worldBound.xMin, Is.GreaterThanOrEqualTo(clip.xMin - 1), "The popup must not hide the start of the exact destination.");
            Assert.That(destination.worldBound.xMin + textWidth, Is.LessThanOrEqualTo(clip.xMax + 1),
                "The full destination must fit inside the menu viewport, not merely its own overflowing label.");
            Assert.That(station.Slots.Single(s => s.CatId == worker.Id).AutomatedBy, Is.Empty);
        }

        [UnityTest]
        public IEnumerator WorkDestinationShowsProgressAndExactOfficerHandbackTogether()
        {
            var slot = station.Slots.Single(s => s.CatId == worker.Id);
            var orders = slot.Queue.ToArray();
            var navigation = Root.Q<DropdownField>("workshop-navigation");
            navigation.value = WorkDestination;
            yield return null; yield return null;
            var drawer = Root.Q<ScrollView>(className: "drawer-scroll");
            Assert.That(drawer.scrollOffset.y, Is.GreaterThan(100), "This must exercise the deep second work position.");
            var position = Root.Query<VisualElement>(className: "work-position").ToList()
                .Single(p => p.Query<Button>().ToList().Any(b => b.text == "Release worker · " + worker.Name));
            AssertVisible(drawer, position.Q<Label>(className: "worker-status"));
            AssertVisible(drawer, position.Q<ProgressBar>("live-work-progress"));
            var handback = position.Query<Button>().ToList().Single(b => b.text == "Return to Forester");
            AssertVisible(drawer, handback);
            Submit(handback);
            yield return null; yield return null;
            Assert.That(slot.AutomatedBy, Is.EqualTo("forester"));
            Assert.That(slot.CatId, Is.EqualTo(worker.Id));
            CollectionAssert.AreEqual(orders, slot.Queue, "Handback must preserve the exact existing finite orders.");
            Assert.That(slot.Queue.All(q => !q.Repeat && q.AutomatedBy == ""), Is.True);
            Assert.That(game.Selected.Jobs.Single(j => j.Id == worker.JobId).Progress, Is.EqualTo(51.3));
        }

        [UnityTest]
        public IEnumerator LastJumpNamesTheExactPositionAndSectionThroughRefreshAndClearsWhenInvalid()
        {
            var drawer = Root.Q<ScrollView>(className: "drawer-scroll");
            var navigation = Root.Q<DropdownField>("workshop-navigation");
            string prefix = "Position 2 · " + worker.Name + " · ";
            foreach (string section in new[] { "Queue", "Inputs" })
            {
                navigation.value = prefix + section;
                yield return null; yield return null;
                var context = Root.Q<Label>("workshop-navigation-context");
                Assert.That(context, Is.Not.Null, "After the chooser resets, a deep queue or input bill must still identify its exact worker.");
                Assert.That(context.text, Is.EqualTo("Jumped to " + prefix + section), "Describe the last deliberate jump, without claiming to track manual scrolling.");
                Assert.That(drawer.Contains(context), Is.False);
                Assert.That(context.worldBound.yMax, Is.LessThanOrEqualTo(drawer.contentViewport.worldBound.yMin));
                drawer.scrollOffset += new Vector2(0, 20);
                yield return new WaitForSecondsRealtime(.6f);
                Assert.That(Root.Q<Label>("workshop-navigation-context").text, Is.EqualTo("Jumped to " + prefix + section));
                Assert.That(navigation.value, Is.EqualTo("Choose a section"));
            }

            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", CatId = worker.Id }).Result.Success, Is.True);
            yield return new WaitForSecondsRealtime(.6f);
            Assert.That(Root.Q<Label>("workshop-navigation-context").text, Is.Empty,
                "A released worker's old numbered destination must not survive the refreshed position list.");
            var hut = game.Selected.Buildings.First(b => b.Kind == "research_hut");
            game.View.InspectBuilding(hut.Id); game.UI.OpenPanel("Inspect");
            yield return null; yield return null;
            navigation.value = prefix + "Inputs";
            yield return null;
            Assert.That(Root.Q<Label>("workshop-navigation-context").text, Is.Empty,
                "A detached callback must not restore another workshop's last-jump context.");
        }

        [UnityTest]
        public IEnumerator PausedWorkDestinationShowsExactResumeAndHandbackWithoutQueueHunting()
        {
            var slot = station.Slots.Single(s => s.CatId == worker.Id);
            var orders = slot.Queue.ToArray();
            Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "pause", Enabled = true }).Result.Success, Is.True);
            var pausedJob = game.Selected.Jobs.Single(j => j.Id == worker.JobId);
            pausedJob.BlockedReason = "queue_paused";
            game.UI.OpenPanel("Inspect");
            yield return null; yield return null;
            Root.Q<DropdownField>("workshop-navigation").value = WorkDestination;
            yield return null; yield return null;
            var drawer = Root.Q<ScrollView>(className: "drawer-scroll");
            var position = Root.Query<VisualElement>(className: "work-position").ToList()
                .Single(p => p.Query<Button>().ToList().Any(b => b.text == "Release worker · " + worker.Name));
            AssertVisible(drawer, position.Q<ProgressBar>("live-work-progress"));
            var resume = position.Query<Button>().ToList().SingleOrDefault(b => b.text == "Resume work");
            Assert.That(resume, Is.Not.Null, "Work must expose this paused position's Resume without returning to its deep queue.");
            AssertVisible(drawer, resume);
            var handback = position.Query<Button>().ToList().Single(b => b.text == "Return to Forester");
            Assert.That(handback.enabledSelf, Is.False, "Resume retains the existing manual-control rule before officer handback.");
            AssertVisible(drawer, handback);
            Submit(resume);
            yield return null; yield return null;
            Assert.That(slot.Paused, Is.False);
            Assert.That(slot.AutomatedBy, Is.Empty);
            position = Root.Query<VisualElement>(className: "work-position").ToList()
                .Single(p => p.Query<Button>().ToList().Any(b => b.text == "Release worker · " + worker.Name));
            var enabledHandback = position.Query<Button>().ToList().Single(b => b.text == "Return to Forester");
            AssertVisible(drawer, enabledHandback);
            Assert.That(position.Query<Label>().ToList().Any(l => l.text.Contains("Production is paused")), Is.False,
                "Resume must report the current unpaused state even while global time stays paused.");
            Assert.That(pausedJob.BlockedReason, Is.EqualTo("queue_paused"), "Feedback must not advance or rewrite authoritative state.");
            Submit(enabledHandback);
            yield return null; yield return null;
            Assert.That(slot.AutomatedBy, Is.EqualTo("forester"));
            CollectionAssert.AreEqual(orders, slot.Queue);
            Assert.That(slot.Queue.All(q => !q.Repeat && q.AutomatedBy == ""), Is.True);
            Assert.That(game.Selected.Jobs.Single(j => j.Id == worker.JobId).Progress, Is.EqualTo(51.3));
        }

        private static void Submit(VisualElement element)
        {
            Assert.That(element.enabledInHierarchy, Is.True);
            element.Focus();
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = element; element.SendEvent(submit); }
        }

        private static void AssertVisible(ScrollView drawer, VisualElement element)
        {
            Assert.That(element, Is.Not.Null);
            Assert.That(element.panel, Is.SameAs(drawer.panel));
            var viewport = drawer.contentViewport.worldBound;
            Assert.That(element.worldBound.yMin, Is.GreaterThanOrEqualTo(viewport.yMin - 1));
            Assert.That(element.worldBound.yMax, Is.LessThanOrEqualTo(viewport.yMax + 1),
                "Work must expose the current batch and its officer action in one viewport.");
        }
    }

    public class ForestWorkerFeedbackTests
    {
        private static string Reason(Building building, WorkSlot slot, Cat cat, Job job)
        {
            var policy = typeof(ForestUI).GetMethod("ReportedWorkerReason", System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
            Assert.That(policy, Is.Not.Null);
            return (string)policy.Invoke(null, new object[] { building, slot, cat, job });
        }

        [Test]
        public void ResumeFeedbackUsesCurrentPauseFlagsWithoutAdvancingTheWorld()
        {
            var building = new Building { Kind = "wood_cutter" };
            var slot = new WorkSlot();
            var cat = new Cat { Alive = true };
            var job = new Job { Kind = "production", BlockedReason = "queue_paused" };
            Assert.That(Reason(building, slot, cat, job), Is.Empty,
                "A resumed position must not report its last simulation step's pause while global time stays paused.");
            Assert.That(job.BlockedReason, Is.EqualTo("queue_paused"), "Presentation must leave authoritative history untouched.");
            job.BlockedReason = ""; cat.BlockedReason = "queue_paused";
            Assert.That(Reason(building, slot, cat, job), Is.Empty);
            Assert.That(cat.BlockedReason, Is.EqualTo("queue_paused"));
            cat.BlockedReason = ""; slot.BlockedReason = "queue_paused";
            Assert.That(Reason(building, slot, cat, null), Is.Empty);
            Assert.That(slot.BlockedReason, Is.EqualTo("queue_paused"));
        }

        [Test]
        public void ResumeFeedbackKeepsCurrentNeedsAndRouteBlockers()
        {
            var building = new Building { Kind = "wood_cutter" };
            var slot = new WorkSlot { BlockedReason = "missing_workshop_inputs" };
            var cat = new Cat { Alive = true, BlockedReason = "blocked_route" };
            var job = new Job { Kind = "production", BlockedReason = "queue_paused" };
            Assert.That(Reason(building, slot, cat, job), Is.EqualTo("blocked_route"));
            cat.BlockedReason = "queue_paused";
            Assert.That(Reason(building, slot, cat, null), Is.EqualTo("missing_workshop_inputs"));
            job.BlockedReason = "foreign_territory";
            Assert.That(Reason(building, slot, cat, job), Is.EqualTo("foreign_territory"));
            cat.Goal = "need_drink"; cat.BlockedReason = "water_source_unavailable";
            Assert.That(Reason(building, slot, cat, job), Is.EqualTo("water_source_unavailable"));
            cat.Goal = ""; slot.Paused = true;
            Assert.That(Reason(building, slot, cat, job), Is.Empty);
            slot.Paused = false; building.Paused = true;
            Assert.That(Reason(building, slot, cat, job), Is.Empty);
            Assert.That(job.BlockedReason, Is.EqualTo("foreign_territory"));
            Assert.That(cat.BlockedReason, Is.EqualTo("water_source_unavailable"));
            Assert.That(slot.BlockedReason, Is.EqualTo("missing_workshop_inputs"));
        }
    }
}
