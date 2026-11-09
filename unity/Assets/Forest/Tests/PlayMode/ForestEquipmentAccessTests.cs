using System;
using System.Collections;
using System.Collections.Generic;
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
    public class ForestEquipmentAccessTests
    {
        private ForestGame game;
        private string directory;
        private Cat cat;
        private Stockpile pile;
        private Item item;
        private VisualElement Root => game.GetComponent<UIDocument>().rootVisualElement;

        [UnitySetUp]
        public IEnumerator StartIsolatedWorld()
        {
            Assert.That(ForestGame.Instance, Is.Null, "Never replace a running player's world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-equipment-access-" + Guid.NewGuid().ToString("N"));
            game = new GameObject("Isolated equipment access UI test").AddComponent<ForestGame>();
            game.InitialSavePath = Path.Combine(directory, "world.json");
            yield return null;
            Assert.That(game.Selected, Is.Not.Null, game.Status);
            game.SetSpeed(0);
            var village = game.Selected;
            cat = village.Cats.First(c => c.Alive && c.AgeHours >= 12 && c.Id != village.LeaderId && c.JobId == "" && c.BuildingId == "");
            cat.Hunger = cat.Thirst = cat.Rest = 80;
            foreach (var storage in village.Stockpiles.Where(p => p.Kind == "storage"))
                storage.Accepts = new List<string> { "food", "water" };
            bool Free(Int2 p) => village.Known.Contains(p) && game.CurrentWorld.TileAt(p)?.ClaimId == village.Id && game.CurrentWorld.Walkable(village, p)
                && !game.CurrentWorld.TileAt(p).Road && !game.CurrentWorld.TileAt(p).Rail
                && !village.Buildings.Any(b => Inside(b.Position, b.Width, b.Depth, p) || b.HasEntrance && b.Entrance.Equals(p))
                && !village.Stockpiles.Any(s => Inside(s.Position, s.Width, s.Depth, p));
            var anchor = game.CurrentWorld.Tiles.Select(t => t.Position)
                .Where(p => Int2.Distance(cat.Position, p) >= 5 && Free(p) && Free(new Int2(p.X + 1, p.Z, p.Level)))
                .OrderBy(p => Int2.Distance(cat.Position, p)).ThenBy(p => p.Z).ThenBy(p => p.X)
                .First(p => game.CurrentWorld.Path(cat.Position, new Int2(p.X + 1, p.Z, p.Level), village) != null);
            var alternate = new Int2(anchor.X + 1, anchor.Z, anchor.Level);
            var designated = game.Send(new GameAction { Kind = "DesignateStockpile", Position = anchor, End = alternate, Accepts = new List<string> { "tools" } }).Result;
            Assert.That(designated.Success, Is.True, designated.Error);
            pile = village.Stockpiles.Single(p => p.Id == designated.EntityId);
            var avoided = game.Send(new GameAction { Kind = "CreateZone", Resource = "avoid", Position = anchor, End = anchor }).Result;
            Assert.That(avoided.Success, Is.True, avoided.Error);
            Assert.That(game.CurrentWorld.Path(cat.Position, anchor, village), Is.Null);
            Assert.That(game.CurrentWorld.Path(cat.Position, alternate, village), Is.Not.Null);
            item = new Item { Id = "ui-footprint-tool", Kind = "tool", Material = "wood", Quality = 3, Condition = 17, MaxCondition = 42, VillageId = village.Id, LocationId = pile.Id };
            village.Items.Add(item);
        }

        [UnityTearDown]
        public IEnumerator CloseIsolatedWorld()
        {
            if (game != null) UnityEngine.Object.Destroy(game.gameObject);
            yield return null;
            if (directory != null && Directory.Exists(directory)) Directory.Delete(directory, true);
        }

        [Test]
        public void EquipmentRetrievalControlClaimsTheNamedItemThroughAnOpenFootprintTile()
        {
            game.View.InspectCat(cat.Id);
            game.UI.OpenPanel("Inspect");
            Submit(Root.Query<Button>().ToList().Single(b => b.text == "Choose equipment for this cat"));
            var card = Root.Q<VisualElement>("equipment-item-" + item.Id);
            Assert.That(card, Is.Not.Null);
            Submit(card.Query<Button>().ToList().Single(b => b.text == "Retrieve for selected cat"));
            var job = game.Selected.Jobs.SingleOrDefault(j => !j.Completed && j.Kind == "equip" && j.CatId == cat.Id);
            Assert.That(job, Is.Not.Null, "The normal retrieval control must accept the open tile despite a closed anchor.");
            Assert.That(job.Position, Is.EqualTo(pile.Position));
            Assert.That(item.LocationId, Is.EqualTo(job.Id));
            Assert.That(cat.Equipment, Does.Not.Contain(item.Id));
            var order = Root.Q<VisualElement>("equipment-order-" + job.Id);
            Assert.That(order, Is.Not.Null);
            Assert.That(string.Join("\n", order.Query<Label>().ToList().Select(l => l.text)), Does.Contain(pile.Id).And.Contain("after arrival"));
            Submit(order.Query<Button>().ToList().Single(b => b.text == "Cancel equipment order"));
            Assert.That(job.Completed, Is.True);
            Assert.That(item.LocationId, Is.EqualTo(pile.Id));
            Assert.That(cat.Equipment, Does.Not.Contain(item.Id));
            Assert.That(cat.JobId, Is.Empty);
        }

        [Test]
        public void EquipmentReturnControlRetainsWornGearAndExactCancellationWithAClosedAnchor()
        {
            item.LocationId = cat.Id;
            cat.Equipment.Add(item.Id);
            game.View.InspectCat(cat.Id);
            game.UI.OpenPanel("Inspect");
            Submit(Root.Query<Button>().ToList().Single(b => b.text == "Return tool to storage"));
            var job = game.Selected.Jobs.SingleOrDefault(j => !j.Completed && j.Kind == "unequip" && j.CatId == cat.Id);
            Assert.That(job, Is.Not.Null, "The normal return control must accept the open tile despite a closed anchor.");
            Assert.That(job.TargetId, Is.EqualTo(pile.Id));
            Assert.That(job.Position, Is.EqualTo(pile.Position));
            Assert.That(item.LocationId, Is.EqualTo(cat.Id));
            Assert.That(cat.Equipment, Does.Contain(item.Id));
            var order = Root.Q<VisualElement>("equipment-order-" + job.Id);
            Assert.That(order, Is.Not.Null);
            Assert.That(string.Join("\n", order.Query<Label>().ToList().Select(l => l.text)), Does.Contain(pile.Id).And.Contain("Still equipped until delivered"));
            Submit(order.Query<Button>().ToList().Single(b => b.text == "Cancel equipment order"));
            Assert.That(job.Completed, Is.True);
            Assert.That(item.LocationId, Is.EqualTo(cat.Id));
            Assert.That(cat.Equipment, Does.Contain(item.Id));
            Assert.That(item.Condition, Is.EqualTo(17));
            Assert.That(item.MaxCondition, Is.EqualTo(42));
            Assert.That(cat.JobId, Is.Empty);
        }

        private static bool Inside(Int2 origin, int width, int depth, Int2 p) => p.Level == origin.Level && p.X >= origin.X && p.X < origin.X + width && p.Z >= origin.Z && p.Z < origin.Z + depth;
        private static void Submit(Button button)
        {
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = button; button.SendEvent(submit); }
        }
    }
}
