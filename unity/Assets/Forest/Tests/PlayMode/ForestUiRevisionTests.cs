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
    public class ForestUiRevisionTests
    {
        private ForestGame game;
        private string directory;
        private VisualElement Root => game.GetComponent<UIDocument>().rootVisualElement;

        [UnitySetUp]
        public IEnumerator StartIsolatedWorld()
        {
            Assert.That(ForestGame.Instance, Is.Null, "Never replace a running player's world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-ui-revision-" + Guid.NewGuid().ToString("N"));
            game = new GameObject("Isolated UI revision test").AddComponent<ForestGame>();
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

        [Test]
        public void WindowResizingPreservesPhysicalTextSize()
        {
            var panel = game.GetComponent<UIDocument>().panelSettings;
            Assert.That(panel.scaleMode, Is.EqualTo(PanelScaleMode.ConstantPhysicalSize), "Small windows must reflow rather than shrink the entire management UI.");
            Assert.That(panel.referenceDpi, Is.InRange(96f, 120f));
        }

        [UnityTest]
        public IEnumerator WorldFocusReservesOnlyTheVisibleSidebarAndDrawer()
        {
            var property = typeof(ForestUI).GetProperty("WorldLeftFraction");
            Assert.That(property, Is.Not.Null, "The camera needs the actual visible management bounds to keep its subject out from under the drawer.");
            float Fraction() => (float)property.GetValue(game.UI);
            yield return null;
            float navigation = Fraction();
            Assert.That(navigation, Is.GreaterThan(0));
            game.UI.OpenPanel("Cats");
            yield return null; yield return null;
            Assert.That(Fraction(), Is.GreaterThan(navigation), "An open cat roster occupies more of the world than navigation alone.");
            Assert.That(Fraction(), Is.InRange(0, .8f));
            game.UI.ClosePanel();
            yield return null; yield return null;
            Assert.That(Fraction(), Is.EqualTo(navigation).Within(.01f));
            game.UI.OpenPanel("Research");
            yield return null; yield return null;
            Assert.That(Fraction(), Is.EqualTo(navigation).Within(.01f), "A full-page research map must not move the camera toward an invisible sliver of forest.");
        }

        [Test]
        public void InspectReturnsToItsRosterAndKeepsTheActiveCategoryVisible()
        {
            game.UI.OpenPanel("Cats");
            Assert.That(FindButton("Cats").ClassListContains("active"), Is.True);
            var cat = game.Selected.Cats.First(c => c.Alive);
            Submit("Inspect " + cat.Name);
            Assert.That(game.UI.OpenSection, Is.EqualTo("Inspect"));
            Assert.That(FindButton("Cats").ClassListContains("active"), Is.True);
            Submit("Back");
            Assert.That(game.UI.OpenSection, Is.EqualTo("Cats"));
            Submit("Close");
            Assert.That(Root.Query<Button>().ToList().Any(b => b.ClassListContains("navigation-button") && b.ClassListContains("active")), Is.False);
        }

        [Test]
        public void BlockedCatExplainsTheRouteProblemWithoutInternalCodes()
        {
            var cat = game.Selected.Cats.First(c => c.Alive);
            cat.BlockedReason = "blocked_route";
            game.View.InspectCat(cat.Id); game.UI.OpenPanel("Inspect");
            var labels = Root.Query<Label>().ToList();
            Assert.That(labels.Any(l => l.text.Contains("No walkable route")), Is.True, "A player needs to understand why the cat cannot reach its work.");
            Assert.That(labels.Any(l => l.text == "blocked_route"), Is.False);
        }

        [Test]
        public void FirstWorkerQueueAppearsOnceWithEveryEntryControl()
        {
            var station = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            var cat = game.Selected.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "");
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            Assert.That(game.Send(new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = "logs_to_planks" }).Result.Success, Is.True);
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            Assert.That(Root.Query<Label>().ToList().Count(l => l.text == "1. logs to planks"), Is.EqualTo(1), "The first worker's authoritative queue must not be duplicated as a second station queue.");
            foreach (var text in new[] { "Pause", "Resume", "↑", "↓", "Repeat off", "Remove" }) Assert.That(FindButton(text), Is.Not.Null);
            Submit("Repeat off");
            Assert.That(station.Slots[0].Queue[0].Repeat, Is.True);
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", CatId = cat.Id }).Result.Success, Is.True);
            game.UI.OpenPanel("Inspect");
            Submit("Remove");
            Assert.That(station.Slots[0].Queue, Is.Empty, "The station queue must remain editable while its first work position is vacant.");
        }

        [UnityTest]
        public IEnumerator NavigationStartsAtTheTopWhileLiveRefreshKeepsItsPlace()
        {
            var station = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            for (int i = 0; i < 8; i++) Assert.That(game.Send(new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = "logs_to_planks" }).Result.Success, Is.True);
            game.UI.OpenPanel("Build");
            yield return null; yield return null;
            var drawer = Root.Q<ScrollView>(className: "drawer-scroll");
            drawer.scrollOffset = new Vector2(0, 250);
            yield return null;
            Assert.That(drawer.scrollOffset.y, Is.GreaterThan(100), "The setup must actually scroll the building list.");
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            Assert.That(drawer.scrollOffset.y, Is.EqualTo(0).Within(.1f), "A new inspector must show its staffing and status before the queue.");
            yield return null; yield return null;
            drawer.scrollOffset = new Vector2(0, 150);
            yield return null;
            Assert.That(drawer.scrollOffset.y, Is.GreaterThan(100));
            game.View.InspectCat(game.Selected.Cats.First(c => c.Alive).Id); game.UI.OpenPanel("Inspect");
            Assert.That(drawer.scrollOffset.y, Is.EqualTo(0).Within(.1f), "Changing the inspected subject must also start at the top.");
            yield return null; yield return null;
            drawer.scrollOffset = new Vector2(0, 100);
            yield return null;
            float offset = drawer.scrollOffset.y;
            Assert.That(offset, Is.GreaterThan(25));
            yield return new WaitForSecondsRealtime(.6f);
            Assert.That(drawer.scrollOffset.y, Is.EqualTo(offset).Within(.1f), "The ordinary live refresh must preserve the player's place.");
        }

        [UnityTest]
        public IEnumerator ResearchPrerequisiteNavigationStartsAtTheRequestedStudy()
        {
            game.UI.OpenPanel("Research");
            yield return null; yield return null;
            var drawer = Root.Q<ScrollView>(className: "drawer-scroll");
            drawer.scrollOffset = new Vector2(0, 250);
            yield return null;
            Assert.That(drawer.scrollOffset.y, Is.GreaterThan(100), "The setup must actually scroll the research catalog.");
            var prerequisite = Root.Query<Button>().ToList().First(b => b.text == "Requires Research Hut");
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = prerequisite; prerequisite.SendEvent(submit); }
            Assert.That(drawer.scrollOffset.y, Is.EqualTo(0).Within(.1f), "Following a prerequisite must reveal the requested study at the top, not reuse the old catalog offset.");
            Assert.That(Root.Query<TextField>().ToList().Single(f => f.label == "Search studies").value, Is.EqualTo("research_hut"));
            var firstCard = Root.Query<VisualElement>(className: "research-card").ToList().First();
            Assert.That(firstCard.Q<Label>(className: "subtitle").text, Is.EqualTo("Research Hut"));
        }

        [UnityTest]
        public IEnumerator NarrowWindowKeepsHeaderRowsAndScrollablePanelsSeparate()
        {
            game.Selected.ResearchPoints = 19995; game.Selected.Blessings = 200;
            Root.style.flexGrow = 0; Root.style.width = 600; Root.style.maxWidth = 600; Root.style.height = 360; Root.style.maxHeight = 360;
            game.UI.OpenPanel("Build");
            yield return null; yield return null; yield return null;
            Assert.That(Root.worldBound.width, Is.EqualTo(600).Within(.5f), "The regression must exercise the narrow panel width.");
            var header = Root.Q<VisualElement>(className: "top");
            var brand = Root.Q<VisualElement>(className: "brand");
            var clock = Root.Q<VisualElement>(className: "clock");
            var resources = Root.Q<VisualElement>(className: "resources");
            Assert.That(resources.worldBound.yMin, Is.GreaterThanOrEqualTo(Mathf.Max(brand.worldBound.yMax, clock.worldBound.yMax)), "Narrow windows need resources below the village name and time controls.");
            Assert.That(brand.worldBound.xMax, Is.LessThanOrEqualTo(clock.worldBound.xMin), "The village summary must not overlap the clock.");
            var chips = resources.Query<VisualElement>(className: "resource").ToList();
            for (int i = 0; i < chips.Count; i++)
            {
                AssertInside(header.worldBound, chips[i].worldBound, "Every resource chip must fit inside the header.");
                foreach (var label in chips[i].Query<Label>().ToList()) AssertInside(chips[i].worldBound, label.worldBound, "Resource values and names must fit their row without inherited label padding.");
                if (i > 0) Assert.That(chips[i - 1].worldBound.xMax, Is.LessThanOrEqualTo(chips[i].worldBound.xMin + .5f), "Resource values must not overlap adjacent chips.");
            }
            var speeds = clock.Query<Button>().ToList();
            foreach (var button in speeds)
            {
                AssertInside(header.worldBound, button.worldBound, "All four time controls must fit inside the header.");
                Assert.That(button.worldBound.yMin, Is.EqualTo(speeds[0].worldBound.yMin).Within(.5f), "Time controls must stay on one row.");
            }
            var nav = Root.Q<ScrollView>(className: "navigation");
            var drawer = Root.Q<VisualElement>(className: "drawer");
            var bottom = Root.Q<VisualElement>(className: "bottom");
            foreach (var panel in new VisualElement[] { nav, drawer })
            {
                Assert.That(panel.worldBound.yMin, Is.GreaterThanOrEqualTo(header.worldBound.yMax), "Management panels must begin below the taller narrow header.");
                Assert.That(panel.worldBound.yMax, Is.LessThanOrEqualTo(bottom.worldBound.yMin), "The scrollable panels must not cover the footer.");
                Assert.That(panel.worldBound.height, Is.GreaterThan(40), "Short windows still need a usable scroll viewport.");
            }
            foreach (var size in new[] { new Vector2(600, 360), new Vector2(550, 336) })
            {
                Root.style.width = Root.style.maxWidth = size.x; Root.style.height = Root.style.maxHeight = size.y;
                foreach (var section in new[] { "Cats", "Inspect" })
                {
                    if (section == "Inspect") game.View.InspectCat(game.Selected.Cats.First(c => c.Alive).Id);
                    game.UI.OpenPanel(section);
                    yield return null; yield return null; yield return null;
                    Assert.That(Root.Q<ScrollView>(className: "drawer-scroll").contentViewport.worldBound.height, Is.GreaterThanOrEqualTo(70), section + " needs readable content beneath its heading at " + size);
                    AssertInside(drawer.worldBound, FindButton("Close").worldBound, "Close must remain visible in short inspectors.");
                    Assert.That(drawer.worldBound.yMax, Is.LessThanOrEqualTo(bottom.worldBound.yMin), "The narrow drawer must remain above the footer.");
                }
            }
        }

        private static void AssertInside(Rect outer, Rect inner, string reason)
        {
            Assert.That(inner.xMin, Is.GreaterThanOrEqualTo(outer.xMin - .5f), reason);
            Assert.That(inner.xMax, Is.LessThanOrEqualTo(outer.xMax + .5f), reason);
            Assert.That(inner.yMin, Is.GreaterThanOrEqualTo(outer.yMin - .5f), reason);
            Assert.That(inner.yMax, Is.LessThanOrEqualTo(outer.yMax + .5f), reason);
        }

        private Button FindButton(string text) => Root.Query<Button>().ToList().Single(b => b.text == text);
        private void Submit(string text)
        {
            var button = FindButton(text);
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = button; button.SendEvent(submit); }
        }
    }
}
