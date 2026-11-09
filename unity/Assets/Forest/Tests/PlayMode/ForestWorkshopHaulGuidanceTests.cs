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
    public class ForestWorkshopHaulGuidanceTests
    {
        private ForestGame game;
        private string directory;
        private VisualElement Root => game.GetComponent<UIDocument>().rootVisualElement;

        [UnitySetUp]
        public IEnumerator StartIsolatedWorld()
        {
            Assert.That(ForestGame.Instance, Is.Null, "Never replace a running player's world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-workshop-haul-guidance-" + Guid.NewGuid().ToString("N"));
            game = new GameObject("Isolated workshop hauling guidance test").AddComponent<ForestGame>();
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

        [TestCase(false, false)]
        [TestCase(true, true)]
        public void InputBillExplainsDirectPileSupplyAndOptionalHaulingWithoutTreatingReportsAsLiveStock(bool staffed, bool reportedEnough)
        {
            var village = game.Selected;
            var station = village.Buildings.First(b => b.Kind == "wood_cutter");
            var cat = village.Cats.First(c => c.Alive && c.AgeHours >= 12 && c.Id != village.LeaderId && c.JobId == "" && c.BuildingId == "");
            if (staffed)
                Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            var add = new GameAction { Kind = staffed ? "EditProductionWorkSlot" : "EditProductionQueue", BuildingId = station.Id, CatId = staffed ? cat.Id : "", Edit = "add", RecipeId = "logs_to_planks" };
            Assert.That(game.Send(add).Result.Success, Is.True);
            var queue = staffed ? station.Slots.Single(s => s.CatId == cat.Id).Queue : station.Queue;
            var orders = queue.ToArray();
            foreach (var pile in village.Stockpiles)
            {
                pile.Goods.Clear(); pile.Report.Clear();
                pile.CountedAt = game.CurrentWorld.TimeSeconds;
            }
            var countedStorage = village.Stockpiles.First(p => p.Kind == "storage");
            World.Add(countedStorage.Report, "logs", reportedEnough ? 50 : 0);
            double report = game.CurrentWorld.WorkshopSupply(village, station, "logs_to_planks").Inputs.Single(i => i.Resource == "logs").Reported;

            game.UI.OpenPanel("Stores");
            Submit(Root.Q<Button>("stores-equipment-tab"));
            Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat").value = cat.Name + " · " + cat.Id;
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            var bill = Root.Q<VisualElement>(className: "recipe-supply");
            Assert.That(bill, Is.Not.Null, "Both an unstaffed preview and a worker's finite order need the supply explanation.");
            var hauling = bill.Q<Label>("recipe-hauling-guidance");
            Assert.That(hauling, Is.Not.Null, "The ingredient bill must explain direct ingredient collection and optional hauling.");
            Assert.That(hauling.text, Does.Contain("reachable piles"));
            Assert.That(hauling.text, Does.Contain("including external piles"), "A reachable external pile can supply a workshop directly.");
            Assert.That(hauling.text, Does.Contain("Haul from pile"));
            Assert.That(hauling.text, Does.Contain("can move goods"), "Hauling consolidates goods rather than being a required prerequisite for every workshop batch.");
            Assert.That(hauling.text, Does.Contain("accepting storage"), "An optional haul delivers to storage, rather than remotely feeding this workshop.");
            var history = bill.Q<Label>("recipe-reports-guidance");
            Assert.That(history, Is.Not.Null);
            Assert.That(history.text, Does.Contain("Storage reports"));
            Assert.That(history.text, Does.Contain("exclude external piles"), "The bill reports storage history even though direct ingredient collection can also use external piles.");
            Assert.That(history.text, Does.Contain("historical"));
            Assert.That(history.text, Does.Contain("moved goods"));
            Assert.That(history.text, Does.Contain("counted again"), "A report above the bill must not promise live ingredients, and a stale external pile may still show moved goods.");

            int jobs = village.Jobs.Count, claims = game.CurrentWorld.Reservations.Count;
            long nextId = game.CurrentWorld.NextId;
            double time = game.CurrentWorld.TimeSeconds;
            var open = bill.Q<Button>("recipe-inventory-hauling");
            Assert.That(open, Is.Not.Null);
            Assert.That(open.text, Does.Contain("inventory & hauling"));
            Submit(open);
            Assert.That(game.UI.OpenSection, Is.EqualTo("Stores"));
            Assert.That(Root.Query<Button>().ToList().Any(b => b.text == "Haul from pile"), Is.True,
                "The bill's action must open Inventory even when Equipment was the previous Stores destination.");
            Assert.That(Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat").value, Is.EqualTo(cat.Name + " · " + cat.Id),
                "Navigation must preserve the explicit carrier, including a busy workshop worker.");
            Assert.That(Root.Query<Label>().ToList().Any(l => l.text.Contains("Choose available cat") && l.text.Contains("automatically")), Is.True,
                "Inventory must retain its explicit route to automatic assignment without silently replacing the named cat.");
            Assert.That(village.Jobs.Count, Is.EqualTo(jobs));
            Assert.That(game.CurrentWorld.Reservations.Count, Is.EqualTo(claims));
            Assert.That(game.CurrentWorld.NextId, Is.EqualTo(nextId));
            Assert.That(game.CurrentWorld.TimeSeconds, Is.EqualTo(time));
            Assert.That(game.Speed, Is.Zero);
            CollectionAssert.AreEqual(orders, queue, "Supply navigation must preserve the exact finite queue.");
            Assert.That(cat.BuildingId, Is.EqualTo(staffed ? station.Id : ""));
            Assert.That(cat.Cargo, Is.Empty);
            Assert.That(game.CurrentWorld.WorkshopSupply(village, station, "logs_to_planks").Inputs.Single(i => i.Resource == "logs").Reported, Is.EqualTo(report),
                "Explaining historical counts must not refresh or rewrite them.");
            Assert.That(World.Amount(countedStorage.Goods, "logs"), Is.Zero);
        }

        [Test]
        public void FiniteWorkshopBatchReservesAndPhysicallyCollectsExternalSpillWithoutHauling()
        {
            var world = game.CurrentWorld;
            var village = game.Selected;
            var station = village.Buildings.First(b => b.Kind == "wood_cutter");
            var cat = village.Cats.First(c => c.Alive && c.AgeHours >= 12 && c.Id != village.LeaderId && c.JobId == "" && c.BuildingId == "");
            foreach (var actor in village.Cats) actor.Hunger = actor.Thirst = actor.Rest = 100;
            foreach (var pile in village.Stockpiles)
            {
                World.Add(pile.Goods, "logs", -World.Amount(pile.Goods, "logs"));
                pile.Report.Clear(); pile.CountedAt = world.TimeSeconds;
            }
            station.Inputs.Clear();
            var at = world.Tiles.Where(t => t.ClaimId == village.Id && Int2.Distance(cat.Position, t.Position) == 1 && world.Walkable(village, t.Position))
                .Select(t => t.Position).First(p => world.Path(cat.Position, p, village) != null && world.Path(p, station.Position, village) != null);
            var spill = new Stockpile { Id = "workshop-guidance-spill", Kind = "spill", Position = at, Width = 1, Depth = 1, Capacity = 100, CountedAt = world.TimeSeconds };
            spill.Goods.Add(new IdleCatForest.Simulation.Stack("logs", 5));
            spill.Report.Add(new IdleCatForest.Simulation.Stack("logs", 5));
            village.Stockpiles.Add(spill);
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = cat.Id, Edit = "add", RecipeId = "logs_to_planks" }).Result.Success, Is.True);
            var slot = station.Slots.Single(s => s.CatId == cat.Id);
            var order = slot.Queue.Single();
            Assert.That(order.Repeat, Is.False);
            Assert.That(world.WorkshopSupply(village, station, order.RecipeId).Inputs.Single(i => i.Resource == "logs").Reported, Is.Zero,
                "The bill excludes an external pile's historical report even when its live goods can supply the batch.");

            world.Step(Math.Floor(world.TimeSeconds) + 1 - world.TimeSeconds);
            var job = village.Jobs.Single(j => !j.Completed && j.Kind == "production" && j.CatId == cat.Id);
            var claim = world.Reservations.Single(r => r.OwnerId == job.Id && r.Resource == "logs");
            Assert.That(claim.PileId, Is.EqualTo(spill.Id));
            Assert.That(claim.Amount, Is.EqualTo(5));
            Assert.That(World.Amount(spill.Goods, "logs"), Is.EqualTo(5), "Reservation must not remotely move the external goods.");
            Assert.That(cat.Cargo, Is.Empty);
            for (int step = 0; step < 200 && World.Amount(cat.Cargo, "logs") < 5; step++) world.Step(.05);
            Assert.That(World.Amount(cat.Cargo, "logs"), Is.EqualTo(5), "The production worker must collect the exact external bill without an intervening haul job.");
            Assert.That(cat.Position, Is.EqualTo(spill.Position));
            Assert.That(cat.X, Is.EqualTo(spill.Position.X).Within(1e-9));
            Assert.That(cat.Z, Is.EqualTo(spill.Position.Z).Within(1e-9));
            Assert.That(World.Amount(spill.Goods, "logs"), Is.Zero);
            Assert.That(World.Amount(spill.Report, "logs"), Is.EqualTo(5), "Pickup preserves the external pile's historical report.");
            Assert.That(world.Reservations.Any(r => r.OwnerId == job.Id), Is.False);
            Assert.That(village.Jobs.Any(j => j.Kind == "haul"), Is.False);
            Assert.That(slot.Queue.Single(), Is.SameAs(order), "Collecting direct ingredients retains the player's finite order.");
            Assert.That(job.Completed, Is.False);
        }

        [Test]
        public void OutputStoragePreflightExplainsSpaceAndRouteWithoutClaimingFinishedGoods()
        {
            var world = game.CurrentWorld;
            var village = game.Selected;
            var station = village.Buildings.First(b => b.Kind == "wood_cutter");
            var cat = village.Cats.First(c => c.Alive && c.AgeHours >= 12 && c.Id != village.LeaderId && c.JobId == "" && c.BuildingId == "");
            foreach (var actor in village.Cats) actor.Hunger = actor.Thirst = actor.Rest = 100;
            foreach (var pile in village.Stockpiles.Where(p => p.Kind == "storage")) pile.Accepts = new System.Collections.Generic.List<string> { "food", "water" };
            station.Inputs.Clear(); World.Add(station.Inputs, "logs", 5);
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }).Result.Success, Is.True);
            Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = cat.Id, Edit = "add", RecipeId = "logs_to_planks" }).Result.Success, Is.True);
            world.Step(Math.Floor(world.TimeSeconds) + 1 - world.TimeSeconds);
            var slot = station.Slots.Single(s => s.CatId == cat.Id);
            Assert.That(slot.BlockedReason, Is.EqualTo("output_storage_full"));
            Assert.That(village.Jobs.Any(j => j.Kind == "production" && j.CatId == cat.Id), Is.False);
            Assert.That(slot.Queue.Count, Is.EqualTo(1));
            Assert.That(station.Outputs, Is.Empty);
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            var advice = Root.Query<Label>().ToList().Where(l => l.text.Contains("matching stockpile")).Select(l => l.text).ToArray();
            Assert.That(advice, Is.Not.Empty, "A batch waiting before work needs a visible storage action.");
            Assert.That(advice.All(text => text.Contains("space") && text.Contains("route")), Is.True);
            Assert.That(advice.Any(text => text.Contains("Finished goods")), Is.False,
                "Output preflight has made no goods. Its shared advice must also make sense before the batch starts.");
            Assert.That(slot.Queue.Count, Is.EqualTo(1));
            Assert.That(World.Amount(station.Inputs, "logs"), Is.EqualTo(5));
            Assert.That(game.Speed, Is.Zero);
        }

        private static void Submit(Button button)
        {
            Assert.That(button, Is.Not.Null);
            Assert.That(button.enabledInHierarchy, Is.True);
            button.Focus();
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = button; button.SendEvent(submit); }
        }
    }
}
