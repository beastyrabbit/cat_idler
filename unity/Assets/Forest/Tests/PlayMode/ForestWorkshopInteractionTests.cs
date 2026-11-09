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
    public class ForestWorkshopInteractionTests
    {
        private ForestGame game;
        private string directory;
        private VisualElement Root => game.GetComponent<UIDocument>().rootVisualElement;

        [UnitySetUp]
        public IEnumerator StartIsolatedWorld()
        {
            Assert.That(ForestGame.Instance, Is.Null, "Never replace a running player's world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-workshop-interaction-" + Guid.NewGuid().ToString("N"));
            game = new GameObject("Isolated workshop interaction test").AddComponent<ForestGame>();
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
        public IEnumerator FocusedSecondWorkerAddSurvivesLiveRefreshAndChangesOnlyItsQueue()
        {
            var village = game.Selected;
            var station = village.Buildings.First(b => b.Kind == "wood_cutter");
            village.Research.Add("wood_cutter_crews");
            village.Research.Add("carpentry_specialty");
            var cats = village.Cats.Where(c => c.Alive && c.AgeHours >= 12 && c.JobId == "" && c.BuildingId == "" && c.Id != village.LeaderId).Take(2).ToArray();
            Assert.That(cats.Length, Is.EqualTo(2));
            foreach (var cat in cats)
                Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = cats[0].Id, Edit = "add", RecipeId = "logs_to_planks" }).Result.Success, Is.True);
            Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = cats[1].Id, Edit = "add", RecipeId = "carpentry_specialty" }).Result.Success, Is.True);
            var first = station.Slots.Single(s => s.CatId == cats[0].Id);
            var second = station.Slots.Single(s => s.CatId == cats[1].Id);
            var firstOrders = first.Queue.ToArray();
            var secondOrder = second.Queue.Single();
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            Root.Query<DropdownField>().ToList().Single(f => f.label == "Recipe").value = "logs_to_planks";
            yield return null;
            var position = Root.Query<VisualElement>(className: "work-position").ToList()
                .Single(p => p.Query<Button>().ToList().Any(b => b.text == "Release worker · " + cats[1].Name));
            var add = position.Query<Button>().ToList().Single(b => b.text == "Add selected recipe to this cat's queue");
            Assert.That(add.panel, Is.SameAs(Root.panel), "The regression must use the attached inspector, not a detached control.");
            Assert.That(add.enabledInHierarchy, Is.True);
            Root.Q<ScrollView>(className: "drawer-scroll").ScrollTo(add);
            add.Focus();
            Assert.That(Root.panel.focusController.focusedElement, Is.SameAs(add));
            double before = game.CurrentWorld.TimeSeconds;
            game.SetSpeed(1);
            yield return new WaitForSecondsRealtime(.6f);
            Assert.That(game.CurrentWorld.TimeSeconds, Is.GreaterThan(before), "The inspector must remain usable while the world advances through Update.");

            // Resolve actual current focus after refresh. Submitting the saved button would hide focus loss.
            var focused = Root.panel.focusController.focusedElement as Button;
            Assert.That(focused, Is.Not.Null, "Live inspector refresh must retain the player's focused action until they activate it.");
            Assert.That(focused.panel, Is.SameAs(Root.panel));
            Assert.That(focused.text, Is.EqualTo("Add selected recipe to this cat's queue"));
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = focused; focused.SendEvent(submit); }
            Assert.That(game.LastAction, Does.EndWith(" accepted"));
            CollectionAssert.AreEqual(firstOrders, first.Queue, "Adding for the second cat must preserve the first cat's exact orders.");
            Assert.That(second.Queue.Count, Is.EqualTo(2));
            Assert.That(second.Queue[0], Is.SameAs(secondOrder), "The second cat's existing order must retain its identity and position.");
            Assert.That(second.Queue[1].RecipeId, Is.EqualTo("logs_to_planks"), "Only the selected recipe belongs in the selected cat's new order.");
        }

        [UnityTest]
        public IEnumerator FirstWorkerOrderKeepsItsAddControlAtTheSameViewportPosition()
        {
            Root.style.flexGrow = 0; Root.style.width = 1000; Root.style.maxWidth = 1000;
            Root.style.height = 800; Root.style.maxHeight = 800;
            var village = game.Selected;
            var station = village.Buildings.First(b => b.Kind == "wood_cutter");
            village.Research.Add("wood_cutter_crews");
            var cats = village.Cats.Where(c => c.Alive && c.AgeHours >= 12 && c.JobId == "" && c.BuildingId == "" && c.Id != village.LeaderId).Take(2).ToArray();
            Assert.That(cats.Length, Is.EqualTo(2));
            foreach (var cat in cats)
                Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            for (int index = 0; index < 6; index++)
                Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = cats[0].Id, Edit = "add", RecipeId = "logs_to_planks" }).Result.Success, Is.True);
            var first = station.Slots.Single(s => s.CatId == cats[0].Id);
            var second = station.Slots.Single(s => s.CatId == cats[1].Id);
            Assert.That(second.Queue, Is.Empty);
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            Root.Query<DropdownField>().ToList().Single(f => f.label == "Recipe").value = "logs_to_planks";
            yield return null; yield return null;
            VisualElement Position() => Root.Query<VisualElement>(className: "work-position").ToList()
                .Single(p => p.Query<Button>().ToList().Any(b => b.text == "Release worker · " + cats[1].Name));
            Button Add() => Position().Query<Button>().ToList().Single(b => b.text == "Add selected recipe to this cat's queue");
            var drawer = Root.Q<ScrollView>(className: "drawer-scroll");
            var initialAdd = Add();
            drawer.ScrollTo(initialAdd);
            initialAdd.Focus();
            yield return null; yield return null;
            drawer.ScrollTo(Add());
            yield return null;
            Assert.That(Root.worldBound.width, Is.EqualTo(1000).Within(.5f));
            Assert.That(Root.worldBound.height, Is.EqualTo(800).Within(.5f));
            Assert.That(drawer.scrollOffset.y, Is.GreaterThan(100), "The setup must reach the second worker through a genuinely deep inspector.");
            initialAdd = Add();
            Assert.That(initialAdd.panel, Is.SameAs(Root.panel));
            var viewport = drawer.contentViewport.worldBound;
            Assert.That(initialAdd.worldBound.yMin, Is.GreaterThanOrEqualTo(viewport.yMin - 1));
            Assert.That(initialAdd.worldBound.yMax, Is.LessThanOrEqualTo(viewport.yMax + 1), "The first Add must be fully visible before activation.");
            float initialY = initialAdd.worldBound.yMin;
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = initialAdd; initialAdd.SendEvent(submit); }
            yield return null; yield return null; yield return null;
            Assert.That(second.Queue.Count, Is.EqualTo(1), game.LastAction);
            Assert.That(second.Queue.Single().RecipeId, Is.EqualTo("logs_to_planks"));
            Assert.That(first.Queue.Count, Is.EqualTo(6), "The other worker's orders must remain untouched.");
            Assert.That(Position().Query<VisualElement>(className: "recipe-supply").ToList(), Is.Not.Empty,
                "The new order must reveal its actual ingredient report above the action.");
            var replacementAdd = Add();
            Assert.That(replacementAdd.panel, Is.SameAs(Root.panel));
            Assert.That(replacementAdd.worldBound.yMin, Is.EqualTo(initialY).Within(2),
                "Showing the first order's supply report must keep this worker's Add action under the player's pointer.");
        }
    }
}
