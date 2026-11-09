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
    public class ForestDefenseUiTests
    {
        private ForestGame game;
        private string directory;
        private VisualElement Root => game.GetComponent<UIDocument>().rootVisualElement;

        [UnitySetUp]
        public IEnumerator StartIsolatedWorld()
        {
            Assert.That(ForestGame.Instance, Is.Null, "Never replace a running player's world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-defense-ui-" + Guid.NewGuid().ToString("N"));
            game = new GameObject("Isolated defense UI test").AddComponent<ForestGame>();
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
        public void DefenseOrdersUseTheSelectedCatAndSpecificRaidWithoutInstantDamage()
        {
            var cat = game.Selected.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "" && c.Id != game.Selected.LeaderId);
            var first = new Raid { Id = "ui-first-raid", Position = new Int2(game.Selected.Center.X + 6, game.Selected.Center.Z), Health = 30 };
            var target = new Raid { Id = "ui-selected-raid", Position = new Int2(game.Selected.Center.X - 6, game.Selected.Center.Z), Health = 24 };
            game.Selected.Raids.Add(first); game.Selected.Raids.Add(target);
            game.UI.OpenPanel("Defense");
            Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat").value = cat.Name + " · " + cat.Id;
            Assert.That(Root.Query<Button>().ToList().Any(b => b.text == "Defend active raid"), Is.False, "Defense must commit a physical cat to a chosen raid rather than damage an unspecified enemy.");
            var card = Root.Query<VisualElement>(className: "raid-card").ToList().SingleOrDefault(p => p.name == target.Id);
            Assert.That(card, Is.Not.Null, "Each active raid needs its own target-specific order control.");
            Assert.That(card.Query<Label>().ToList().Any(l => l.text.Contains("24") && l.text.Contains(target.Position.ToString())), Is.True, "A raid card must identify its current health and position.");
            Submit(card.Query<Button>().ToList().Single(b => b.text == "Defend with selected cat"));
            var order = game.Selected.Jobs.SingleOrDefault(j => !j.Completed && j.Kind == "defend_raid" && j.CatId == cat.Id);
            Assert.That(order, Is.Not.Null, "The selected free adult must receive an authoritative physical defense order.");
            Assert.That(order.TargetId, Is.EqualTo(target.Id));
            Assert.That(first.Health, Is.EqualTo(30)); Assert.That(target.Health, Is.EqualTo(24));
        }

        [Test]
        public void DefenseShowsReadinessAndOwnedOrdersWithInspectAndCancelControls()
        {
            var cat = game.Selected.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "" && c.Id != game.Selected.LeaderId);
            var raid = new Raid { Id = "ui-order-raid", Position = new Int2(game.Selected.Center.X + 6, game.Selected.Center.Z) };
            game.Selected.Raids.Add(raid);
            var order = new Job { Id = "ui-defense-order", Kind = "defend_raid", CatId = cat.Id, TargetId = raid.Id, Phase = "travel", Manual = true, BlockedReason = "blocked_route" };
            game.Selected.Jobs.Add(order); cat.JobId = order.Id;
            var automaticCat = game.Selected.Cats.First(c => c.Alive && c.JobId == "" && c.BuildingId == "" && c.Id != game.Selected.LeaderId);
            var automatic = new Job { Id = "ui-captain-defense-order", Kind = "defend_raid", CatId = automaticCat.Id, TargetId = raid.Id, Phase = "fighting", AutomatedBy = "captain" };
            game.Selected.Jobs.Add(automatic); automaticCat.JobId = automatic.Id;
            game.UI.OpenPanel("Defense");
            var readiness = Root.Q<VisualElement>(className: "defense-readiness");
            Assert.That(readiness, Is.Not.Null, "Idle defense needs a visible Captain and preparedness summary.");
            Assert.That(readiness.Query<Label>().ToList().Any(l => l.text.Contains("Captain · Vacant")), Is.True);
            Assert.That(readiness.Query<Label>().ToList().Any(l => l.text.Contains("trained") && l.text.Contains("target") && l.text.Contains("training")), Is.True, "The player must see trained coverage, the bounded target and current training.");
            var defender = Root.Query<VisualElement>(className: "defender-order").ToList().SingleOrDefault(p => p.name == order.Id);
            Assert.That(defender, Is.Not.Null, "Actual defenders must be visible independently of the selected cat.");
            var copy = string.Join("\n", defender.Query<Label>().ToList().Select(l => l.text));
            Assert.That(copy, Does.Contain(cat.Name).And.Contain(raid.Id).And.Contain("Manual").And.Contain("No walkable route"));
            Assert.That(defender.Query<Button>().ToList().Any(b => b.text == "Inspect defender"), Is.True);
            Assert.That(defender.Query<Button>().ToList().Any(b => b.text == "Cancel defense order"), Is.True);
            var captainOrder = Root.Query<VisualElement>(className: "defender-order").ToList().Single(p => p.name == automatic.Id);
            Assert.That(captainOrder.Query<Label>().ToList().Any(l => l.text.Contains("Captain order")), Is.True);
            Assert.That(captainOrder.Query<Button>().ToList().Any(b => b.text == "Cancel defense order"), Is.False, "A Captain would immediately reassign a canceled automatic defender; manage the office instead.");
            Assert.That(captainOrder.Query<Button>().ToList().Any(b => b.text == "Manage Captain in Officers"), Is.True);
            Submit(defender.Query<Button>().ToList().Single(b => b.text == "Inspect defender"));
            Assert.That(game.UI.OpenSection, Is.EqualTo("Inspect"));
            Assert.That(game.View.SelectedCat?.Id, Is.EqualTo(cat.Id));
            Submit(Root.Query<Button>().ToList().Single(b => b.text == "Back"));
            Assert.That(game.UI.OpenSection, Is.EqualTo("Defense"));
            defender = Root.Query<VisualElement>(className: "defender-order").ToList().Single(p => p.name == order.Id);
            Submit(defender.Query<Button>().ToList().Single(b => b.text == "Cancel defense order"));
            Assert.That(order.Completed, Is.True, "Cancel must address this manual order through the authority.");
            Assert.That(cat.JobId, Is.Empty);
            Assert.That(automatic.Completed, Is.False, "Canceling one manual order must preserve Captain-owned defense.");
        }

        [Test]
        public void DefenseUpdatesHealthAndRaidMembershipWithoutReplacingAFocusedCatChoice()
        {
            var raid = new Raid { Id = "ui-live-raid", Position = new Int2(game.Selected.Center.X + 6, game.Selected.Center.Z), Health = 30 };
            game.Selected.Raids.Add(raid); game.UI.OpenPanel("Defense");
            var choice = Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat");
            raid.Health = 17.5;
            RefreshUi();
            Assert.That(Root.Query<VisualElement>(className: "raid-card").ToList().Single(p => p.name == raid.Id).Query<Label>().ToList().Any(l => l.text.Contains("17.5")), Is.True, "Health must advance while the player leaves Defense open.");
            Assert.That(Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat"), Is.SameAs(choice), "Live values must not rebuild the selected-cat control.");
            choice.Focus();
            var newcomer = new Raid { Id = "ui-new-raid", Position = game.Selected.Center, Health = 20 };
            game.Selected.Raids.Add(newcomer); RefreshUi();
            Assert.That(Root.Query<DropdownField>().ToList().Single(f => f.label == "Cat"), Is.SameAs(choice), "An arriving raid must not tear down a focused dropdown.");
            choice.Blur(); RefreshUi();
            Assert.That(Root.Query<VisualElement>(className: "raid-card").ToList().Any(p => p.name == newcomer.Id), Is.True, "New raids must appear without closing and reopening Defense.");
            game.Selected.Raids.Remove(raid); RefreshUi();
            Assert.That(Root.Query<VisualElement>(className: "raid-card").ToList().Any(p => p.name == raid.Id), Is.False, "Ended raids must leave the list.");
        }

        [Test]
        public void BarracksInspectorCanPauseAndResumeTheSamePhysicalTrainingJob()
        {
            var barracks = game.Selected.Buildings.First(b => b.Kind == "barracks" && b.Completed);
            barracks.Slots.Add(new WorkSlot());
            var cat = game.Selected.Cats.First(c => c.Alive && c.AgeHours >= 12 && c.JobId == "" && c.BuildingId == "" && c.Id != game.Selected.LeaderId);
            cat.Position = barracks.Position; cat.X = barracks.Position.X; cat.Z = barracks.Position.Z;
            Assert.That(game.Send(new GameAction { Kind = "TrainWarrior", CatId = cat.Id }).Result.Success, Is.True);
            var job = game.Selected.Jobs.Single(j => j.Id == cat.JobId);
            game.CurrentWorld.Step(.2);
            Assert.That(job.Progress, Is.GreaterThan(0), "This regression must start with a real advancing training job.");
            game.View.InspectBuilding(barracks.Id); game.UI.OpenPanel("Inspect");
            var pause = Root.Query<Button>().ToList().SingleOrDefault(b => b.text == "Pause training");
            Assert.That(pause, Is.Not.Null, "The Barracks needs an accessible training pause without assigning a permanent worker.");
            Assert.That(Root.Query<Button>().ToList().Any(b => b.text == "Assign worker"), Is.False);
            Submit(pause);
            Assert.That(barracks.Paused, Is.True, "Pause training must use the authoritative Barracks pause.");
            double held = job.Progress;
            game.CurrentWorld.Step(.4);
            Assert.That(job.Progress, Is.EqualTo(held), "Paused Barracks must hold real manual training progress.");
            Assert.That(cat.JobId, Is.EqualTo(job.Id));
            Submit(Root.Query<Button>().ToList().Single(b => b.text == "Resume training"));
            Assert.That(barracks.Paused, Is.False);
            game.CurrentWorld.Step(.4);
            Assert.That(cat.JobId, Is.EqualTo(job.Id), "Resume must continue the same assigned training job.");
            Assert.That(job.Progress, Is.GreaterThan(held));
            Assert.That(job.Completed, Is.False);
        }

        [Test]
        public void BarracksHoverFollowsPhysicalTrainingNeedsAndPauseWithoutPermanentWorker()
        {
            var barracks = game.Selected.Buildings.First(b => b.Kind == "barracks" && b.Completed);
            var cat = game.Selected.Cats.First(c => c.Alive && c.AgeHours >= 12 && c.JobId == "" && c.BuildingId == "" && c.Id != game.Selected.LeaderId);
            game.View.InspectBuilding(barracks.Id); game.UI.OpenPanel("Defense");
            string idle = HoverBody();
            cat.Position = barracks.Position; cat.X = barracks.Position.X; cat.Z = barracks.Position.Z;
            Assert.That(game.Send(new GameAction { Kind = "TrainWarrior", CatId = cat.Id }).Result.Success, Is.True);
            var job = game.Selected.Jobs.Single(j => j.Id == cat.JobId);
            game.CurrentWorld.Step(.2);
            Assert.That(job.Progress, Is.GreaterThan(0), "The hover regression must include real advancing training.");
            Assert.That(barracks.WorkerId, Is.Empty, "Training must not require a permanent worker.");
            Assert.That(HoverBody(), Does.Contain("1 training").And.Not.Contain("Waiting for a worker"));
            Assert.That(idle, Does.Contain("Training enabled").And.Contain("no trainees"));

            cat.Thirst = 1;
            game.CurrentWorld.Step(.05);
            Assert.That(cat.Goal, Is.EqualTo("need_drink"));
            Assert.That(cat.ResumeJobId, Is.EqualTo(job.Id));
            Assert.That(HoverBody(), Does.Contain("1 attending to needs").And.Not.Contain("1 training"));
            Assert.That(game.Send(new GameAction { Kind = "EditProductionQueue", BuildingId = barracks.Id, Edit = "pause", Enabled = true }).Result.Success, Is.True);
            Assert.That(HoverBody(), Does.Contain("Training paused").And.Not.Contain("1 training"));
            Assert.That(game.Send(new GameAction { Kind = "EditProductionQueue", BuildingId = barracks.Id, Edit = "pause", Enabled = false }).Result.Success, Is.True);

            job.TargetId = "";
            Assert.That(HoverBody(), Does.Contain("1 attending to needs"), "Legacy training belongs to its original Barracks position.");
            job.Position = new Int2(barracks.Position.X + 10, barracks.Position.Z);
            Assert.That(HoverBody(), Does.Contain("no trainees"), "Another site's legacy job must not appear here.");
            job.TargetId = barracks.Id;
            Assert.That(HoverBody(), Does.Contain("1 attending to needs"), "Explicit target identity takes precedence over the legacy position.");
        }

        private string HoverBody()
        {
            typeof(ForestUI).GetMethod("UpdateTetheredInspector", BindingFlags.Instance | BindingFlags.NonPublic).Invoke(game.UI, null);
            return ((Label)typeof(ForestUI).GetField("tetheredInspectorBody", BindingFlags.Instance | BindingFlags.NonPublic).GetValue(game.UI)).text;
        }

        private void RefreshUi()
        {
            typeof(ForestUI).GetField("nextRefresh", BindingFlags.Instance | BindingFlags.NonPublic).SetValue(game.UI, 0f);
            typeof(ForestUI).GetMethod("Update", BindingFlags.Instance | BindingFlags.NonPublic).Invoke(game.UI, null);
        }

        private static void Submit(Button button)
        {
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = button; button.SendEvent(submit); }
        }
    }
}
