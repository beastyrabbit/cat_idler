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
    public class ForestHaulControlsTests
    {
        private ForestGame game;
        private string directory;
        private Cat selected, preferred;
        private Stockpile source;
        private VisualElement Root => game.GetComponent<UIDocument>().rootVisualElement;

        [UnitySetUp]
        public IEnumerator StartIsolatedWorld()
        {
            Assert.That(ForestGame.Instance, Is.Null, "Never replace a running player's world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-haul-controls-" + Guid.NewGuid().ToString("N"));
            game = new GameObject("Isolated hauling controls test").AddComponent<ForestGame>();
            game.InitialSavePath = Path.Combine(directory, "world.json");
            yield return null;
            Assert.That(game.Selected, Is.Not.Null, game.Status);
            game.SetSpeed(0);
            var village = game.Selected;
            var free = village.Cats.Where(c => c.Alive && c.AgeHours >= 12 && c.Id != village.LeaderId && c.JobId == "" && c.BuildingId == "").Take(2).ToArray();
            Assert.That(free.Length, Is.EqualTo(2));
            selected = free[0]; preferred = free[1];
            selected.Name = "Chosen carrier"; preferred.Name = "Preferred automatic carrier";
            preferred.Preferences.Add("haul"); World.Add(preferred.Skills, "haul", 50);
            var at = game.CurrentWorld.Tiles.Where(t => t.ClaimId == village.Id && game.CurrentWorld.Walkable(village, t.Position))
                .Select(t => t.Position).First(p => !village.Stockpiles.Any(s => Inside(s, p)) && game.CurrentWorld.Path(selected.Position, p, village) != null && game.CurrentWorld.Path(preferred.Position, p, village) != null);
            source = new Stockpile { Id = "haul-controls-source", Kind = "spill", Position = at, Width = 1, Depth = 1, Capacity = 100 };
            source.Goods.Add(new IdleCatForest.Simulation.Stack("logs", 8));
            village.Stockpiles.Add(source);
            village.Stockpiles[0].Accepts.Clear(); village.Stockpiles[0].Capacity = 100000;
        }

        [UnityTearDown]
        public IEnumerator CloseIsolatedWorld()
        {
            if (game != null) UnityEngine.Object.Destroy(game.gameObject);
            yield return null;
            if (directory != null && Directory.Exists(directory)) Directory.Delete(directory, true);
        }

        [TestCase(false)]
        [TestCase(true)]
        public void WorkToStoresHaulPreservesTheChosenCarrierOrAutomaticChoice(bool automatic)
        {
            double time = game.CurrentWorld.TimeSeconds;
            ChooseInWork(automatic ? "Choose available cat" : selected.Name + " · " + selected.Id);
            Submit(Root.Query<Button>().ToList().Single(b => b.text == "Haul goods · choose source"));
            Submit(HaulButton());
            var job = game.Selected.Jobs.Single(j => !j.Completed && j.Kind == "haul" && j.SourceId == source.Id);
            Assert.That(job.CatId, Is.EqualTo(automatic ? preferred.Id : selected.Id), "Hauling must honor the named cat selected in Work; automatic ranking applies only to the explicit available-cat choice.");
            Assert.That(game.CurrentWorld.TimeSeconds, Is.EqualTo(time));
            Assert.That(game.Speed, Is.Zero);
            Assert.That(World.Amount(source.Goods, "logs"), Is.EqualTo(8), "A paused assignment must claim goods without moving them.");
            Assert.That(game.Selected.Cats.All(c => c.Cargo.Count == 0), Is.True);
            Assert.That(game.CurrentWorld.Reservations.Single(r => r.OwnerId == job.Id).Amount, Is.EqualTo(8));
            var choice = Root.Query<DropdownField>().ToList().FirstOrDefault(f => f.label == "Cat");
            Assert.That(choice, Is.Not.Null, "Stores must expose its carrier choice beside the hauling controls.");
            Assert.That(choice.value, Is.EqualTo(automatic ? "Choose available cat" : selected.Name + " · " + selected.Id), "Stores must expose the same carrier choice beside its hauling controls.");
        }

        [Test]
        public void ChosenBusyCarrierRefusesWithoutSilentlyAssigningAnotherCat()
        {
            var station = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = selected.Id }).Result.Success, Is.True);
            ChooseInWork(selected.Name + " · " + selected.Id);
            Submit(Root.Query<Button>().ToList().Single(b => b.text == "Haul goods · choose source"));
            int jobs = game.Selected.Jobs.Count, claims = game.CurrentWorld.Reservations.Count;
            long nextId = game.CurrentWorld.NextId;
            Submit(HaulButton());
            Assert.That(game.Selected.Jobs.Count, Is.EqualTo(jobs), "An unavailable named cat must refuse instead of assigning an automatic replacement.");
            Assert.That(game.CurrentWorld.Reservations.Count, Is.EqualTo(claims));
            Assert.That(game.CurrentWorld.NextId, Is.EqualTo(nextId));
            Assert.That(selected.BuildingId, Is.EqualTo(station.Id));
            Assert.That(preferred.JobId, Is.Empty);
            Assert.That(World.Amount(source.Goods, "logs"), Is.EqualTo(8));
        }

        private void ChooseInWork(string value)
        {
            game.UI.OpenPanel("Work");
            Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat").value = value;
        }

        private Button HaulButton() => Root.Query<VisualElement>(className: "card").ToList()
            .Single(c => c.Query<Label>().ToList().Any(l => l.text == source.Kind + " · " + source.Id))
            .Query<Button>().ToList().Single(b => b.text == "Haul from pile");

        private static bool Inside(Stockpile pile, Int2 p) => p.Level == pile.Position.Level && p.X >= pile.Position.X && p.Z >= pile.Position.Z && p.X < pile.Position.X + pile.Width && p.Z < pile.Position.Z + pile.Depth;
        private static void Submit(Button button)
        {
            Assert.That(button.enabledInHierarchy, Is.True);
            button.Focus();
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = button; button.SendEvent(submit); }
        }
    }
}
