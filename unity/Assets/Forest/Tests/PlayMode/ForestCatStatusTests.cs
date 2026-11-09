using System;
using System.Collections;
using System.IO;
using System.Linq;
using System.Reflection;
using IdleCatForest.Presentation;
using IdleCatForest.Simulation;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.UIElements;

namespace IdleCatForest.Tests
{
    public class ForestCatStatusTests
    {
        private const BindingFlags Private = BindingFlags.Instance | BindingFlags.NonPublic;
        private ForestGame game;
        private string directory;
        private Building station;
        private Cat worker;
        private Job job;
        private WorkSlot slot;
        private VisualElement Root => game.GetComponent<UIDocument>().rootVisualElement;

        [UnitySetUp]
        public IEnumerator StartIsolatedProduction()
        {
            Assert.That(ForestGame.Instance, Is.Null, "Never replace a running player's world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-cat-status-" + Guid.NewGuid().ToString("N"));
            game = new GameObject("Isolated cat status test").AddComponent<ForestGame>();
            game.InitialSavePath = Path.Combine(directory, "world.json");
            yield return null;
            Assert.That(game.Selected, Is.Not.Null, game.Status);
            game.SetSpeed(0);
            station = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            worker = game.Selected.Cats.First(c => c.Alive && c.AgeHours >= 12 && c.JobId == "" && c.BuildingId == "" && c.Id != game.Selected.LeaderId);
            worker.Name = "Status Worker";
            Assert.That(game.Send(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = worker.Id }).Result.Success, Is.True);
            Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "add", RecipeId = "logs_to_planks" }).Result.Success, Is.True);
            slot = station.Slots.Single(s => s.CatId == worker.Id);
            job = new Job { Id = "cat-status-current-batch", Kind = "production", TargetId = station.Id, CatId = worker.Id, RecipeId = "logs_to_planks", Phase = "working", Progress = 38.4, RequiredWork = 600 };
            game.Selected.Jobs.Add(job); worker.JobId = slot.JobId = job.Id;
            worker.Goal = "production · working";
        }

        [UnityTearDown]
        public IEnumerator CloseIsolatedWorld()
        {
            if (game != null) UnityEngine.Object.Destroy(game.gameObject);
            yield return null;
            if (directory != null && Directory.Exists(directory)) Directory.Delete(directory, true);
        }

        [TestCase(true)]
        [TestCase(false)]
        public void PauseAndResumeRefreshEveryCatSurfaceWithoutAdvancingTime(bool remainPaused)
        {
            double time = game.CurrentWorld.TimeSeconds;
            var orders = slot.Queue.ToArray();
            Pause(true);
            if (!remainPaused)
            {
                worker.Goal = "production paused";
                worker.BlockedReason = slot.BlockedReason = job.BlockedReason = "queue_paused";
                Pause(false);
            }
            foreach (var surface in CatSurfaces())
            {
                if (remainPaused)
                    Assert.That(surface.Text, Does.Contain("Paused"), surface.Name + " must show the current pause immediately, before a simulation step.");
                else
                {
                    Assert.That(surface.Text, Does.Contain("logs to planks · Working"), surface.Name + " must describe the retained current batch immediately after Resume.");
                    Assert.That(surface.Text.ToLowerInvariant(), Does.Not.Contain("paused"), surface.Name + " must not repeat an obsolete goal or pause blocker.");
                }
            }
            Assert.That(game.Speed, Is.Zero);
            Assert.That(game.CurrentWorld.TimeSeconds, Is.EqualTo(time));
            Assert.That(job.Progress, Is.EqualTo(38.4));
            Assert.That(slot.Paused, Is.EqualTo(remainPaused));
            Assert.That(worker.JobId, Is.EqualTo(job.Id));
            Assert.That(slot.CatId, Is.EqualTo(worker.Id));
            CollectionAssert.AreEqual(orders, slot.Queue);
            if (!remainPaused)
            {
                Assert.That(worker.Goal, Is.EqualTo("production paused"), "Presentation must not rewrite historical authoritative goals.");
                Assert.That(worker.BlockedReason, Is.EqualTo("queue_paused"));
                Assert.That(job.BlockedReason, Is.EqualTo("queue_paused"));
                Assert.That(slot.BlockedReason, Is.EqualTo("queue_paused"));
            }
        }

        [Test]
        public void CurrentProductionFeedbackPreservesNeedsAndRealBlockers()
        {
            worker.Goal = "need_drink"; worker.BlockedReason = "need_supply_missing";
            job.BlockedReason = "queue_paused";
            var needs = CatSurfaces();
            foreach (var surface in needs)
                Assert.That(surface.Text, Does.Contain("need drink").Or.Contain("Getting water"), surface.Name + " must retain the cat's current need instead of claiming production.");
            foreach (var surface in needs.Where(s => s.Name != "Defense selection"))
                Assert.That(surface.Text, Does.Contain("Food or water is missing or unreachable"), surface.Name + " must retain the current need's explanation.");

            Pause(true); worker.Goal = "production · working";
            worker.BlockedReason = "blocked_route"; job.BlockedReason = "input_unreachable";
            var paused = CatSurfaces();
            foreach (var surface in paused.Where(s => s.Name != "Defense selection"))
                Assert.That(surface.Text, Does.Contain("No walkable route"), surface.Name + " must preserve a real cat blocker even while its queue is paused.");
            Assert.That(paused.Single(s => s.Name == "Cat inspector").Text, Does.Contain("Recipe ingredients cannot reach this workshop"),
                "The inspector must preserve its separate real job blocker during a current pause.");

            Pause(false); job.BlockedReason = "queue_paused";
            worker.Goal = "production paused"; worker.BlockedReason = "blocked_route";
            var blocked = CatSurfaces();
            foreach (var surface in blocked.Where(s => s.Name != "Defense selection"))
                Assert.That(surface.Text, Does.Contain("No walkable route"), surface.Name + " must preserve the real current route blocker when discarding the obsolete pause.");
            Assert.That(worker.BlockedReason, Is.EqualTo("blocked_route"));
            Assert.That(job.BlockedReason, Is.EqualTo("queue_paused"));
        }

        [Test]
        public void PausedQueueStillShowsAnAlreadyFinishedBatchBeingDelivered()
        {
            Pause(true); job.Phase = "output_delivery"; worker.Goal = "production · output_delivery";
            worker.BlockedReason = job.BlockedReason = "queue_paused";
            string work = (string)typeof(ForestUI).GetMethod("WorkerStatus", Private).Invoke(game.UI, new object[] { station, slot, worker });
            Assert.That(work, Does.Contain("Delivering finished goods").And.Not.Contain("Paused"));
            foreach (var surface in CatSurfaces())
            {
                Assert.That(surface.Text, Does.Contain("Delivering finished goods").And.Not.Contain("Paused"),
                    surface.Name + " must distinguish a paused future queue from output delivery, which the simulation permits to continue.");
                Assert.That(surface.Text.ToLowerInvariant(), Does.Not.Contain("paused"), surface.Name + " must not retain a pause warning while finished goods are being delivered.");
            }
            Assert.That(slot.Paused, Is.True);
            Assert.That(worker.BlockedReason, Is.EqualTo("queue_paused"));
            Assert.That(job.BlockedReason, Is.EqualTo("queue_paused"));
        }

        [Test]
        public void UnrelatedOrHistoricalGoalsKeepTheirOriginalMeaning()
        {
            job.Completed = true; worker.Goal = "returning from dungeon";
            foreach (var surface in CatSurfaces())
                Assert.That(surface.Text, Does.Contain("returning from dungeon"), surface.Name + " must not reinterpret a completed job as current workshop work.");
            job.Completed = false; job.Kind = "haul"; worker.Goal = "haul · travel";
            foreach (var surface in CatSurfaces())
                Assert.That(surface.Text, Does.Contain("haul · travel"), surface.Name + " must preserve nonproduction goals.");
            job.Kind = "production"; worker.Goal = "returning from dungeon"; job.CatId = "different-cat";
            foreach (var surface in CatSurfaces())
                Assert.That(surface.Text, Does.Contain("returning from dungeon"), surface.Name + " must not borrow another cat's current job.");
            job.CatId = worker.Id; worker.BuildingId = "different-workplace";
            foreach (var surface in CatSurfaces())
                Assert.That(surface.Text, Does.Contain("returning from dungeon"), surface.Name + " must not borrow a job at a different assigned workplace.");
            worker.BuildingId = station.Id; slot.CatId = "different-worker";
            foreach (var surface in CatSurfaces())
                Assert.That(surface.Text, Does.Contain("returning from dungeon"), surface.Name + " requires the cat's own staffed position.");
            slot.CatId = worker.Id;
            worker.ControlledBy = "status-test-player"; worker.Goal = "player_control";
            foreach (var surface in CatSurfaces())
                Assert.That(surface.Text, Does.Contain("player control").Or.Contain("Under direct control"), surface.Name + " must retain direct control.");
            worker.ControlledBy = ""; worker.Alive = false;
            game.View.InspectCat(worker.Id); game.UI.OpenPanel("Inspect");
            Assert.That(Root.Q<Label>(className: "drawer-subtitle").text, Is.EqualTo("Dead"));
            game.UI.ClosePanel();
            typeof(ForestUI).GetMethod("UpdateTetheredInspector", Private).Invoke(game.UI, null);
            Assert.That(((Label)typeof(ForestUI).GetField("tetheredInspectorBody", Private).GetValue(game.UI)).text,
                Does.Contain("Dead").And.Not.Contain("Working"), "A dead cat's tether must not report its previous work or direct control.");
        }

        [Test]
        public void ActiveWorkJobWarningUsesOnlyItsOwnCurrentProductionContext()
        {
            double time = game.CurrentWorld.TimeSeconds;
            var orders = slot.Queue.ToArray();
            WorkshopPause(true);
            job.BlockedReason = "queue_paused";
            Assert.That(WorkJobReason(job), Does.Contain("Production is paused"), "A current pause must remain visible in the exact active job's row.");
            WorkshopPause(false);
            Assert.That(WorkJobReason(job), Is.Empty, "Resume must clear this obsolete active-job warning while global time remains paused.");

            job.BlockedReason = "blocked_route";
            Assert.That(WorkJobReason(job), Does.Contain("No walkable route"));
            job.BlockedReason = "input_unreachable";
            Assert.That(WorkJobReason(job), Does.Contain("Recipe ingredients cannot reach this workshop"));
            WorkshopPause(true); job.Phase = "output_delivery"; job.BlockedReason = "queue_paused";
            Assert.That(WorkJobReason(job), Is.Empty, "A paused future queue must not claim that delivery of finished goods is paused.");
            WorkshopPause(false);

            var unowned = new Job { Id = "cat-status-unowned-job", Kind = "production", CatId = worker.Id, TargetId = station.Id, BlockedReason = "queue_paused" };
            game.Selected.Jobs.Add(unowned);
            Assert.That(WorkJobReason(unowned), Does.Contain("Production is paused"), "Another displayed job must not borrow this cat's current unpaused job as proof.");
            job.CatId = "missing-cat";
            Assert.That(WorkJobReason(job), Does.Contain("Production is paused"), "A job without its actual owner must retain its recorded reason.");
            job.CatId = worker.Id; job.TargetId = "different-workplace";
            Assert.That(WorkJobReason(job), Does.Contain("Production is paused"), "A job at another station must retain its recorded reason.");
            job.TargetId = station.Id; job.Kind = "haul";
            Assert.That(WorkJobReason(job), Does.Contain("Production is paused"), "Nonproduction warnings keep their existing behavior.");
            Assert.That(game.Speed, Is.Zero);
            Assert.That(game.CurrentWorld.TimeSeconds, Is.EqualTo(time));
            Assert.That(job.Progress, Is.EqualTo(38.4));
            Assert.That(worker.JobId, Is.EqualTo(job.Id));
            Assert.That(slot.CatId, Is.EqualTo(worker.Id));
            Assert.That(job.BlockedReason, Is.EqualTo("queue_paused"));
            Assert.That(unowned.BlockedReason, Is.EqualTo("queue_paused"));
            CollectionAssert.AreEqual(orders, slot.Queue);
        }

        [Test]
        public void RecoveredOutputDeliveryRemovesObsoleteWarningFromPlayerSurfaces()
        {
            var world = game.CurrentWorld;
            var village = game.Selected;
            foreach (var cat in village.Cats.Where(c => c != worker))
            { cat.ControlledBy = "isolated-test"; cat.ControlLeaseUntil = 1000000; }
            worker.Hunger = worker.Thirst = worker.Rest = 100;
            foreach (var pile in village.Stockpiles.Where(p => p.Kind == "storage"))
                pile.Accepts = new System.Collections.Generic.List<string> { "food", "water" };
            bool Inside(Int2 origin, int width, int depth, Int2 p) => p.Level == origin.Level && p.X >= origin.X && p.X < origin.X + width && p.Z >= origin.Z && p.Z < origin.Z + depth;
            var destination = world.Tiles.Select(t => t.Position).Where(p => world.TileAt(p).ClaimId == village.Id && village.Known.Contains(p) && world.Walkable(village, p) && !world.TileAt(p).Road && !world.TileAt(p).Rail &&
                !village.Buildings.Any(b => Inside(b.Position, b.Width, b.Depth, p) || b.HasEntrance && b.Entrance.Equals(p)) &&
                !village.Stockpiles.Any(s => Inside(s.Position, s.Width, s.Depth, p)))
                .Where(p => world.Path(station.Position, p, village)?.Count >= 6)
                .OrderByDescending(p => Int2.Distance(station.Position, p)).ThenBy(p => p.Z).ThenBy(p => p.X).First();
            var storage = game.Send(new GameAction { Kind = "DesignateStockpile", Position = destination, End = destination, Accepts = new System.Collections.Generic.List<string> { "planks" } }).Result;
            Assert.That(storage.Success, Is.True, storage.Error);
            var blocked = game.Send(new GameAction { Kind = "CreateZone", Resource = "avoid", Position = destination, End = destination }).Result;
            Assert.That(blocked.Success, Is.True, blocked.Error);
            job.Phase = "output_delivery"; job.Position = station.Position; job.Progress = job.RequiredWork;
            worker.Position = station.Position; worker.X = station.Position.X; worker.Z = station.Position.Z; worker.Path.Clear();
            worker.Cargo.Add(new IdleCatForest.Simulation.Stack("planks", 1));
            slot.Queue.Clear();
            world.Step(.05);
            Assert.That(job.BlockedReason, Is.EqualTo("output_storage_full_or_unreachable"));
            Assert.That(WorkJobReason(job), Does.Contain("Output needs space"));

            Assert.That(game.Send(new GameAction { Kind = "RemoveZone", TargetId = blocked.EntityId }).Result.Success, Is.True);
            for (int i = 0; i < 25 && (worker.BlockedReason != "" || worker.Path.Count == 0); i++) world.Step(.05);
            Assert.That(worker.BlockedReason, Is.Empty, "The actual executor must have recovered its route.");
            Assert.That(worker.Path.Count, Is.GreaterThan(0));
            Assert.That(job.Completed, Is.False, "Observe recovery before delivery completes.");
            Assert.That(World.Amount(worker.Cargo, "planks"), Is.EqualTo(1));
            Assert.That(World.Amount(village.Stockpiles.Single(p => p.Id == storage.EntityId).Goods, "planks"), Is.Zero);
            Assert.That(WorkJobReason(job), Is.Empty, "The active job must stop asking for storage repair during recovered transit.");
            foreach (var surface in CatSurfaces())
                Assert.That(surface.Text, Does.Not.Contain("Output needs space"), surface.Name + " must not retain the obsolete storage warning.");
            Assert.That(game.Speed, Is.Zero);
            Assert.That(worker.JobId, Is.EqualTo(job.Id));
            Assert.That(slot.CatId, Is.EqualTo(worker.Id));
        }

        private void WorkshopPause(bool enabled)
        {
            game.View.InspectBuilding(station.Id); game.UI.OpenPanel("Inspect");
            var button = Root.Query<Button>().ToList().Single(b => b.text == (enabled ? "Pause" : "Resume work"));
            Assert.That(button.enabledInHierarchy, Is.True);
            button.Focus();
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = button; button.SendEvent(submit); }
            Assert.That(slot.Paused, Is.EqualTo(enabled), "Use the actual workshop control for this exact position.");
        }

        private string WorkJobReason(Job displayed)
        {
            game.UI.OpenPanel("Work");
            var boost = Root.Query<Button>().ToList().Single(b => b.text == "Boost " + ForestGame.Describe(displayed.Kind) + " · " + displayed.Id);
            var preceding = boost.parent.ElementAt(boost.parent.IndexOf(boost) - 1) as Label;
            Assert.That(preceding, Is.Not.Null, "Inspect the actual active-job row immediately before its exact Boost control.");
            return preceding.ClassListContains("warning") ? preceding.text : "";
        }

        private void Pause(bool enabled)
            => Assert.That(game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "pause", Enabled = enabled }).Result.Success, Is.True);

        private (string Name, string Text)[] CatSurfaces()
        {
            game.View.InspectCat(worker.Id); game.UI.OpenPanel("Inspect");
            string inspector = Root.Q<Label>(className: "drawer-subtitle").text + "\n"
                + string.Join("\n", Root.Q<ScrollView>(className: "drawer-scroll").Query<Label>(className: "warning").ToList().Select(l => l.text));

            game.UI.OpenPanel("Cats");
            var card = Root.Query<VisualElement>(className: "card").ToList()
                .Single(c => c.Query<Button>().ToList().Any(b => b.text == "Inspect " + worker.Name));
            string census = string.Join("\n", card.Query<Label>().ToList().Select(l => l.text));

            game.UI.OpenPanel("Defense");
            Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat").value = worker.Name + " · " + worker.Id;
            game.UI.OpenPanel("Defense");
            string defense = Root.Query<Label>().ToList().Single(l => l.text.StartsWith(worker.Name + " · health ", StringComparison.Ordinal)).text;

            game.UI.ClosePanel();
            typeof(ForestUI).GetMethod("UpdateTetheredInspector", Private).Invoke(game.UI, null);
            string hover = ((Label)typeof(ForestUI).GetField("tetheredInspectorBody", Private).GetValue(game.UI)).text;
            return new[] { ("Cat inspector", inspector), ("Cats roster", census), ("Defense selection", defense), ("Cat hover", hover) };
        }
    }
}
