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
    public class ForestOfficerWaitTests
    {
        private ForestGame game;
        private string directory;
        private VisualElement Root => game.GetComponent<UIDocument>().rootVisualElement;

        [UnitySetUp]
        public IEnumerator StartIsolatedWorld()
        {
            Assert.That(ForestGame.Instance, Is.Null, "Never replace a running player's world.");
            directory = Path.Combine(Path.GetTempPath(), "forest-officer-wait-" + Guid.NewGuid().ToString("N"));
            Assert.That(Directory.Exists(directory), Is.False, "Refuse an existing fixture directory.");
            game = new GameObject("Isolated officer wait test").AddComponent<ForestGame>();
            game.InitialSavePath = Path.Combine(directory, "world.json");
            game.SetSpeed(0);
            yield return null;
            Assert.That(game.Selected, Is.Not.Null, game.Status);
            Assert.That(game.IsRemote, Is.False);
            Assert.That(game.CurrentWorld.TimeSeconds, Is.Zero, "Pause before the first automatic simulation update.");
        }

        [UnityTearDown]
        public IEnumerator CloseIsolatedWorld()
        {
            if (game != null) UnityEngine.Object.Destroy(game.gameObject);
            yield return null;
            if (directory != null && Directory.Exists(directory)) Directory.Delete(directory, true);
        }

        // Freeze this primary and its setup after authoring; root owns RED execution.
        [Test]
        public void EmptyDelegatedMillExplainsFarmerChoiceAndManualAddTakeover()
        {
            var world = game.CurrentWorld;
            var village = game.Selected;
            foreach (var cat in village.Cats) cat.Health = cat.Hunger = cat.Thirst = cat.Rest = 100;
            foreach (var pile in village.Stockpiles)
            {
                pile.Goods.Clear(); pile.Report.Clear(); pile.CountedAt = -1;
            }
            // Finite supplies prevent survival preemption and unrelated Farmer foraging.
            // Neither unlocked Mill recipe has its input; no job or blocker is authored.
            var store = village.Stockpiles.First(p => p.Kind == "storage" && p.ManagedBy == "");
            World.Add(store.Goods, "food", 1000);
            World.Add(store.Goods, "water", 1000);
            World.Add(store.Goods, "fibre", 10);
            string[] studies = { "research_hut", "basic_tools", "water_carriers", "foraging_lore", "sawmill", "masonry", "irrigation", "milling", "grain_milling_sources", "grain_milling_preparation", "grain_milling_staples" };
            Assert.That(village.Research, Is.Empty);
            village.ResearchPoints = studies.Sum(id => Catalog.Study(id).Cost);
            foreach (string study in studies) Act(new GameAction { Kind = "ResearchNode", NodeId = study });
            Assert.That(village.ResearchPoints, Is.Zero);
            CompletedSupport("field", new Int2(10, 5));
            var mill = CompletedSupport("mill", new Int2(-6, -6));
            var adults = village.Cats.Where(c => c.Alive && c.AgeHours >= 12 && c.Id != village.LeaderId && c.OfficerRole == ""
                && c.ControlledBy == "" && c.JobId == "" && c.ResumeJobId == "" && c.BuildingId == "").Take(2).ToArray();
            Assert.That(adults.Length, Is.EqualTo(2));
            var officer = adults[0]; var worker = adults[1];
            Assert.That(world.Path(worker.Position, mill.Position, village), Is.Not.Null);
            var recipes = Catalog.Recipes.Where(r => r.Building == "mill" && Catalog.RecipeAvailable(village, r)).ToArray();
            CollectionAssert.AreEquivalent(new[] { "grain_to_flour", "flour_to_food" }, recipes.Select(r => r.Id).ToArray());
            Assert.That(recipes.All(r => r.Inputs.Any(s => world.Available(village, s.Resource) + World.Amount(mill.Inputs, s.Resource)
                < world.RecipeInput(village, r, s.Amount))), Is.True, "Every unlocked recipe must be unfunded without route obstruction or fabricated claims.");
            Assert.That(village.Jobs, Is.Empty);
            Assert.That(world.Reservations, Is.Empty);

            Act(new GameAction { Kind = "AssignOfficer", Role = "farmer", CatId = officer.Id });
            Act(new GameAction { Kind = "AssignWorker", BuildingId = mill.Id, CatId = worker.Id });
            var slot = mill.Slots.Single();
            Assert.That(slot.AutomatedBy, Is.Empty, "Public assignment must start manual.");
            Act(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = mill.Id, CatId = worker.Id, Edit = "delegate" });
            // Cross the ordinary ten-second officer decision and next station planner.
            world.Step(11.05);
            Assert.That(world.TimeSeconds, Is.EqualTo(11.05).Within(1e-9));
            Assert.That(village.Officers.Single(o => o.Role == "farmer").CatId, Is.EqualTo(officer.Id));
            Assert.That(officer.Alive, Is.True);
            Assert.That(officer.OfficerRole, Is.EqualTo("farmer"));
            Assert.That(mill.Completed, Is.True);
            Assert.That(mill.Paused || slot.Paused, Is.False);
            Assert.That(mill.Slots.Single(), Is.SameAs(slot));
            Assert.That(mill.WorkerId, Is.EqualTo(worker.Id));
            Assert.That(slot.CatId, Is.EqualTo(worker.Id));
            Assert.That(worker.BuildingId, Is.EqualTo(mill.Id));
            Assert.That(slot.AutomatedBy, Is.EqualTo("farmer"), "The real public handback must still own this exact position.");
            Assert.That(worker.Alive && worker.Health > 90 && worker.Hunger > 90 && worker.Thirst > 90 && worker.Rest > 90, Is.True);
            Assert.That(worker.ControlledBy, Is.Empty);
            Assert.That(worker.Goal.StartsWith("need_", StringComparison.Ordinal), Is.False);
            Assert.That(worker.JobId, Is.Empty);
            Assert.That(worker.ResumeJobId, Is.Empty);
            Assert.That(slot.JobId, Is.Empty);
            Assert.That(village.Jobs.Any(j => !j.Completed && (j.CatId == worker.Id || j.TargetId == mill.Id)), Is.False);
            Assert.That(slot.Queue, Is.Empty);
            Assert.That(mill.Queue, Is.SameAs(slot.Queue));
            Assert.That(slot.BlockedReason, Is.EqualTo("queue_empty"), "The normal station planner must produce the reason being explained.");
            Assert.That(mill.BlockedReason, Is.EqualTo("queue_empty"));
            Assert.That(worker.BlockedReason, Is.Empty);
            Assert.That(slot.Progress, Is.Zero);
            Assert.That(worker.Cargo, Is.Empty);
            Assert.That(mill.Inputs.Concat(mill.Outputs).Concat(slot.Inputs).Concat(slot.Outputs), Is.Empty);
            Assert.That(world.Reservations, Is.Empty);
            Assert.That(world.Validate(), Is.Empty);

            string authority = JsonUtility.ToJson(world), lastAction = game.LastAction;
            int revision = game.Revision;
            game.View.InspectBuilding(mill.Id);
            game.UI.OpenPanel("Inspect");
            Assert.That(game.UI.OpenSection, Is.EqualTo("Inspect"));
            Assert.That(game.View.SelectedBuilding, Is.SameAs(mill));
            var position = Root.Query<VisualElement>(className: "work-position").ToList()
                .Single(p => p.Query<Button>().ToList().Any(b => b.text == "Release worker · " + worker.Name));
            Assert.That(position.panel, Is.SameAs(Root.panel));
            Assert.That(JsonUtility.ToJson(world), Is.EqualTo(authority), "Reading the real inspector must preserve full authority, time, RNG, supplies, reports, claims and ownership.");
            Assert.That(game.Revision, Is.EqualTo(revision));
            Assert.That(game.LastAction, Is.EqualTo(lastAction), "Inspection must issue no gameplay command.");
            Assert.That(game.Speed, Is.Zero);

            // The intended first RED is missing officer-aware status, after every guard above.
            var status = position.Q<Label>(className: "worker-status");
            Assert.That(status, Is.Not.Null);
            Assert.That(status.text, Is.EqualTo("Waiting for Farmer to choose work"),
                "An officer-owned empty Mill must name Farmer as the next decision maker instead of implying the player must supply a recipe.");
            var guidance = position.Query<Label>().ToList().Select(l => l.text).ToArray();
            Assert.That(guidance, Does.Contain("The Farmer chooses work automatically. Add a recipe to take manual control of this position."),
                "The exact position must explain automatic work choice and the ownership consequence of manual Add without guessing a live shortage.");
            Assert.That(guidance, Does.Not.Contain("Ready for work. Add a recipe to this worker's queue."));
            Assert.That(guidance, Does.Not.Contain("No recipes yet. Add the selected recipe to begin."));
        }

        [Test]
        public void ManualEmptyVacancyAndPublicOfficerRemovalKeepManualGuidance()
        {
            var fixture = PrepareWaitingMill();
            Act(new GameAction { Kind = "UnassignOfficer", Role = "farmer" });
            Assert.That(game.Selected.Officers.Any(o => o.Role == "farmer"), Is.False);
            Assert.That(fixture.Worker.BuildingId, Is.Empty, "Public removal releases officer-owned work.");
            game.View.InspectBuilding(fixture.Mill.Id); game.UI.OpenPanel("Inspect");
            var vacant = Root.Query<VisualElement>(className: "work-position").ToList().First();
            Assert.That(Labels(vacant), Does.Contain("Assign a cat above to fill this position.")
                .And.Contain("No recipes yet. Add the selected recipe to begin."));
            Act(new GameAction { Kind = "AssignWorker", BuildingId = fixture.Mill.Id, CatId = fixture.Worker.Id });
            var manual = RenderPosition(fixture.Mill, fixture.Worker);
            Assert.That(manual.Q<Label>(className: "worker-status").text, Is.EqualTo("Waiting for a recipe"));
            Assert.That(Labels(manual), Does.Contain("No recipes yet. Add the selected recipe to begin."));
            AssertNoOfficerWait(manual);
            Act(new GameAction { Kind = "AssignOfficer", Role = "farmer", CatId = fixture.Farmer.Id });
            manual = RenderPosition(fixture.Mill, fixture.Worker);
            AssertNoOfficerWait(manual);
            Assert.That(Labels(manual), Does.Contain("No recipes yet. Add the selected recipe to begin."),
                "An appointment elsewhere must not turn a manual position into delegated work.");
        }

        [Test]
        public void InvalidOwnerAppointmentAndPositionCannotPromiseAutomaticWork()
        {
            var fixture = PrepareWaitingMill();
            var appointment = game.Selected.Officers.Single(o => o.Role == "farmer");
            fixture.Slot.AutomatedBy = "forester";
            AssertNoOfficerWait(RenderPosition(fixture.Mill, fixture.Worker));
            fixture.Slot.AutomatedBy = "farmer";
            string holder = appointment.CatId;
            appointment.CatId = "missing-officer-holder";
            AssertNoOfficerWait(RenderPosition(fixture.Mill, fixture.Worker));
            appointment.CatId = holder; fixture.Farmer.Alive = false;
            AssertNoOfficerWait(RenderPosition(fixture.Mill, fixture.Worker));
            fixture.Farmer.Alive = true;
            game.Selected.Cats.Remove(fixture.Farmer);
            AssertNoOfficerWait(RenderPosition(fixture.Mill, fixture.Worker));
            game.Selected.Cats.Add(fixture.Farmer);
            fixture.Worker.BuildingId = "different-station";
            AssertNoOfficerWait(RenderPosition(fixture.Mill, fixture.Worker));
            fixture.Worker.BuildingId = fixture.Mill.Id;
            var detached = JsonUtility.FromJson<WorkSlot>(JsonUtility.ToJson(fixture.Slot));
            Assert.That(WorkerStatus(fixture.Mill, detached, fixture.Worker), Is.EqualTo("Waiting for a recipe"),
                "An equivalent detached slot is not the current exact position.");
            var formerCat = JsonUtility.FromJson<Cat>(JsonUtility.ToJson(fixture.Worker));
            Assert.That(WorkerStatus(fixture.Mill, fixture.Slot, formerCat), Is.EqualTo("Waiting for a recipe"));
            fixture.Mill.Completed = false;
            Assert.That(WorkerStatus(fixture.Mill, fixture.Slot, fixture.Worker), Is.EqualTo("Waiting for a recipe"));
            fixture.Mill.Completed = true;
            fixture.Worker.Alive = false;
            Assert.That(WorkerStatus(fixture.Mill, fixture.Slot, fixture.Worker), Is.EqualTo("Worker unavailable"));
            fixture.Worker.Alive = true;
            Assert.That(RenderPosition(fixture.Mill, fixture.Worker).Q<Label>(className: "worker-status").text,
                Is.EqualTo("Waiting for Farmer to choose work"));
        }

        [Test]
        public void AnotherRecipeOfficerUsesItsMappedTitleInPositionQueueAndHover()
        {
            PrepareWaitingMill();
            CompletedSupport("sawmill", new Int2(10, -6));
            var adults = game.Selected.Cats.Where(c => c.Alive && c.Id != game.Selected.LeaderId && c.OfficerRole == ""
                && c.ControlledBy == "" && c.JobId == "" && c.ResumeJobId == "" && c.BuildingId == "").Take(2).ToArray();
            Assert.That(adults.Length, Is.EqualTo(2));
            var station = game.Selected.Buildings.First(b => b.Kind == "wood_cutter");
            Act(new GameAction { Kind = "AssignOfficer", Role = "forester", CatId = adults[0].Id });
            Act(new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = adults[1].Id });
            Act(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = adults[1].Id, Edit = "delegate" });
            var position = RenderPosition(station, adults[1]);
            Assert.That(position.Q<Label>(className: "worker-status").text, Is.EqualTo("Waiting for Forester to choose work"));
            Assert.That(position.Query<Label>().ToList().Count(l => l.text == OfficerGuidance("Forester")), Is.EqualTo(1),
                "An otherwise clear position without a recorded reason still explains its empty queue.");
            Assert.That(BuildingHover(station), Does.Contain("Waiting for Forester to choose work").And.Not.Contain("Farmer"));
        }

        [Test]
        public void PublicAddRemoveAndHandbackPreserveExactOwnershipAndRejectedEdits()
        {
            var fixture = PrepareWaitingMill();
            var position = RenderPosition(fixture.Mill, fixture.Worker);
            Submit(position.Q<Button>("worker-add-" + fixture.Mill.Id + "-" + fixture.Worker.Id));
            Assert.That(fixture.Slot.AutomatedBy, Is.Empty);
            var order = fixture.Slot.Queue.Single();
            Assert.That(order.AutomatedBy, Is.Empty);
            Assert.That(fixture.Mill.Slots.Single(), Is.SameAs(fixture.Slot));
            Assert.That(fixture.Worker.BuildingId, Is.EqualTo(fixture.Mill.Id));
            Submit(RenderPosition(fixture.Mill, fixture.Worker).Query<Button>().ToList().Single(b => b.text == "Return to Farmer"));
            Assert.That(fixture.Slot.AutomatedBy, Is.EqualTo("farmer"));
            Assert.That(fixture.Slot.Queue.Single(), Is.SameAs(order));
            Assert.That(order.AutomatedBy, Is.Empty, "Delegation preserves deliberate manual orders.");
            string before = JsonUtility.ToJson(fixture.Slot);
            var rejected = game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = fixture.Mill.Id,
                CatId = fixture.Worker.Id, Edit = "add", RecipeId = "logs_to_planks" }).Result;
            Assert.That(rejected.Success, Is.False);
            Assert.That(JsonUtility.ToJson(fixture.Slot), Is.EqualTo(before));
            rejected = game.Send(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = fixture.Mill.Id,
                CatId = fixture.Worker.Id, Edit = "remove", Index = 8 }).Result;
            Assert.That(rejected.Success, Is.False);
            Assert.That(JsonUtility.ToJson(fixture.Slot), Is.EqualTo(before));
            Act(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = fixture.Mill.Id, CatId = fixture.Worker.Id, Edit = "remove", Index = 0 });
            Assert.That(fixture.Slot.AutomatedBy, Is.Empty);
            Assert.That(fixture.Slot.Queue, Is.Empty);
            AssertNoOfficerWait(RenderPosition(fixture.Mill, fixture.Worker));
            Submit(RenderPosition(fixture.Mill, fixture.Worker).Query<Button>().ToList().Single(b => b.text == "Return to Farmer"));
            Assert.That(RenderPosition(fixture.Mill, fixture.Worker).Q<Label>(className: "worker-status").text,
                Is.EqualTo("Waiting for Farmer to choose work"));
            Submit(Root.Q<Button>("station-add-" + fixture.Mill.Id));
            Assert.That(fixture.Slot.AutomatedBy, Is.Empty);
            Assert.That(fixture.Slot.Queue.Single().AutomatedBy, Is.Empty);
            Assert.That(fixture.Slot.CatId, Is.EqualTo(fixture.Worker.Id));
        }

        [Test]
        public void NeedsResumedWorkDirectControlAndPausesKeepTheirPrecedence()
        {
            var fixture = PrepareWaitingMill();
            fixture.Worker.Goal = "need_drink"; fixture.Worker.BlockedReason = "need_supply_missing";
            fixture.Worker.ResumeJobId = "suspended-work";
            var position = RenderPosition(fixture.Mill, fixture.Worker);
            Assert.That(position.Q<Label>(className: "worker-status").text, Is.EqualTo("Getting water · work position kept"));
            Assert.That(Labels(position), Does.Contain("Food or water is missing or unreachable."));
            AssertNoOfficerWait(position);
            fixture.Worker.Goal = "idle"; fixture.Worker.BlockedReason = "";
            AssertNoOfficerWait(RenderPosition(fixture.Mill, fixture.Worker));
            fixture.Worker.ResumeJobId = ""; fixture.Worker.ControlledBy = "officer-wait-test";
            position = RenderPosition(fixture.Mill, fixture.Worker);
            Assert.That(position.Q<Label>(className: "worker-status").text, Is.EqualTo("Under direct control"));
            AssertNoOfficerWait(position);
            fixture.Worker.ControlledBy = "";
            Act(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = fixture.Mill.Id, CatId = fixture.Worker.Id, Edit = "pause", Enabled = true });
            Act(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = fixture.Mill.Id, CatId = fixture.Worker.Id, Edit = "delegate" });
            position = RenderPosition(fixture.Mill, fixture.Worker);
            Assert.That(position.Q<Label>(className: "worker-status").text, Is.EqualTo("Paused · resume work to continue"));
            AssertNoOfficerWait(position);
            Submit(position.Query<Button>().ToList().Single(b => b.text == "Resume work"));
            Assert.That(fixture.Slot.AutomatedBy, Is.Empty, "Ordinary Resume intentionally takes manual control.");
            Submit(RenderPosition(fixture.Mill, fixture.Worker).Query<Button>().ToList().Single(b => b.text == "Return to Farmer"));
            fixture.Mill.Paused = true;
            AssertNoOfficerWait(RenderPosition(fixture.Mill, fixture.Worker));
            fixture.Mill.Paused = false;
            fixture.Slot.BlockedReason = "queue_paused";
            Assert.That(game.Speed, Is.Zero);
            Assert.That(RenderPosition(fixture.Mill, fixture.Worker).Q<Label>(className: "worker-status").text,
                Is.EqualTo("Waiting for Farmer to choose work"), "Global pause keeps officer ownership; existing Resume proof discards only the obsolete production pause.");
        }

        [Test]
        public void GenuineBlockersKeepTheirReasonAndNeutralEmptyQueueText()
        {
            var fixture = PrepareWaitingMill();
            var reasons = new[] { "missing_input", "input_unreachable", "input_claimed", "output_storage_full_or_unreachable", "station_local_full" };
            var explanations = new[] { "Waiting for recipe ingredients", "Recipe ingredients cannot reach", "Another worker has reserved", "Output needs space", "The workshop is full" };
            for (int i = 0; i < reasons.Length; i++)
            {
                fixture.Slot.BlockedReason = reasons[i];
                var position = RenderPosition(fixture.Mill, fixture.Worker);
                Assert.That(Labels(position), Does.Contain(explanations[i]));
                Assert.That(Labels(position), Does.Contain("No recipes queued. Add a recipe to take manual control of this position."));
                AssertNoOfficerWait(position);
                Assert.That(BuildingHover(fixture.Mill), Does.Contain(explanations[i]).And.Not.Contain("to choose work"));
            }
            fixture.Worker.BlockedReason = "blocked_route";
            fixture.Slot.BlockedReason = "missing_input";
            var blocked = RenderPosition(fixture.Mill, fixture.Worker);
            Assert.That(Labels(blocked), Does.Contain("No walkable route").And.Not.Contain("Waiting for recipe ingredients"));
            AssertNoOfficerWait(blocked);
        }

        [Test]
        public void ActiveJobsAndEmptyOutputDeliveryNeverBecomeOfficerIdle()
        {
            var fixture = PrepareWaitingMill();
            var job = new Job { Id = "officer-wait-active-job", Kind = "production", CatId = fixture.Worker.Id,
                TargetId = fixture.Mill.Id, RecipeId = "grain_to_flour", RequiredWork = 600, Progress = 12 };
            game.Selected.Jobs.Add(job); fixture.Worker.JobId = fixture.Slot.JobId = job.Id;
            var phases = new[] { "fetch", "input_delivery", "working", "output_delivery" };
            var statuses = new[] { "Collecting ingredients", "Carrying ingredients to workshop", "Working", "Delivering finished goods" };
            for (int i = 0; i < phases.Length; i++)
            {
                job.Phase = phases[i];
                var position = RenderPosition(fixture.Mill, fixture.Worker);
                Assert.That(position.Q<Label>(className: "worker-status").text, Is.EqualTo("grain to flour · " + statuses[i]));
                AssertNoOfficerWait(position);
                Assert.That(Labels(position), Does.Contain("No recipes queued. Add a recipe to take manual control of this position."));
                Assert.That(BuildingHover(fixture.Mill), Does.Contain(statuses[i]).And.Not.Contain("to choose work"));
            }
            fixture.Slot.Paused = true;
            Assert.That(RenderPosition(fixture.Mill, fixture.Worker).Q<Label>(className: "worker-status").text,
                Does.Contain("Delivering finished goods").And.Not.Contain("Paused"));
            fixture.Slot.Paused = false; job.Phase = "fetch"; job.BlockedReason = "input_unreachable";
            var blocked = RenderPosition(fixture.Mill, fixture.Worker);
            Assert.That(Labels(blocked), Does.Contain("Recipe ingredients cannot reach this workshop").And.Not.Contain("Ready for work"));
            job.Kind = "haul"; job.RecipeId = ""; job.TargetId = "unrelated-pile"; job.Phase = "travel";
            Assert.That(RenderPosition(fixture.Mill, fixture.Worker).Q<Label>(className: "worker-status").text, Is.EqualTo("haul · Walking to work"));
            AssertNoOfficerWait(RenderPosition(fixture.Mill, fixture.Worker));
            fixture.Worker.JobId = "";
            AssertNoOfficerWait(RenderPosition(fixture.Mill, fixture.Worker));
        }

        [Test]
        public void NonemptyQueueSuppressesStaleEmptyAdviceAndHistoricalJobsDoNotBlockWaiting()
        {
            var fixture = PrepareWaitingMill();
            Act(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = fixture.Mill.Id, CatId = fixture.Worker.Id,
                Edit = "add", RecipeId = "grain_to_flour", Repeat = true });
            Act(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = fixture.Mill.Id, CatId = fixture.Worker.Id, Edit = "delegate" });
            Assert.That(fixture.Slot.BlockedReason, Is.EqualTo("queue_empty"));
            var position = RenderPosition(fixture.Mill, fixture.Worker);
            Assert.That(position.Q<Label>(className: "worker-status").text, Is.EqualTo("Ready for next batch"));
            Assert.That(Labels(position), Does.Contain("Repeating orders continue until you change them.")
                .And.Not.Contain("Ready for work. Add a recipe").And.Not.Contain("No recipes yet"));
            AssertNoOfficerWait(position);
            Assert.That(BuildingHover(fixture.Mill), Does.Not.Contain("Ready for work").And.Not.Contain("to choose work"));
            Act(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = fixture.Mill.Id, CatId = fixture.Worker.Id, Edit = "remove", Index = 0 });
            Act(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = fixture.Mill.Id, CatId = fixture.Worker.Id, Edit = "delegate" });
            fixture.Slot.JobId = "historical-completed-work";
            game.Selected.Jobs.Add(new Job { Id = fixture.Slot.JobId, Kind = "production", CatId = fixture.Worker.Id, TargetId = fixture.Mill.Id, Completed = true });
            Assert.That(RenderPosition(fixture.Mill, fixture.Worker).Q<Label>(className: "worker-status").text,
                Is.EqualTo("Waiting for Farmer to choose work"));
        }

        [Test]
        public void SameIdSnapshotRefreshReadsCurrentOwnershipAndAppointment()
        {
            var fixture = PrepareWaitingMill();
            RenderPosition(fixture.Mill, fixture.Worker);
            var original = game.Selected; string originalBytes = JsonUtility.ToJson(original);
            var current = JsonUtility.FromJson<Village>(originalBytes);
            game.CurrentWorld.Villages[game.CurrentWorld.Villages.IndexOf(original)] = current;
            var mill = current.Buildings.Single(b => b.Id == fixture.Mill.Id);
            var worker = current.Cats.Single(c => c.Id == fixture.Worker.Id);
            var slot = mill.Slots.Single(s => s.CatId == worker.Id);
            Assert.That(game.View.SelectedBuilding, Is.SameAs(mill));
            slot.AutomatedBy = "";
            RefreshWithoutAuthorityChange();
            AssertNoOfficerWait(CurrentPosition(worker));
            slot.AutomatedBy = "farmer";
            var appointment = current.Officers.Single(o => o.Role == "farmer");
            current.Officers.Remove(appointment);
            RefreshWithoutAuthorityChange();
            AssertNoOfficerWait(CurrentPosition(worker));
            current.Officers.Add(appointment);
            RefreshWithoutAuthorityChange();
            Assert.That(CurrentPosition(worker).Q<Label>(className: "worker-status").text, Is.EqualTo("Waiting for Farmer to choose work"));
            Assert.That(BuildingHover(mill), Does.Contain(OfficerGuidance("Farmer")).And.Not.Contain("Ready for work. Add a recipe"));
            Assert.That(JsonUtility.ToJson(original), Is.EqualTo(originalBytes), "Rendering must leave the obsolete snapshot untouched.");
        }

        [UnityTest]
        public IEnumerator FullTargetsNavigationAndOrdinaryRefreshPreserveAuthorityWithoutStockClaims()
        {
            var fixture = PrepareWaitingMill(true);
            Assert.That(game.CurrentWorld.Available(game.Selected, "flour"), Is.GreaterThan(32));
            Assert.That(game.CurrentWorld.Available(game.Selected, "food"), Is.GreaterThan(8 * game.Selected.Cats.Count(c => c.Alive)));
            Assert.That(Catalog.Recipes.Where(r => r.Building == "mill" && Catalog.RecipeAvailable(game.Selected, r))
                .All(r => r.Inputs.All(s => game.CurrentWorld.Available(game.Selected, s.Resource) >= game.CurrentWorld.RecipeInput(game.Selected, r, s.Amount))), Is.True);
            var position = RenderPosition(fixture.Mill, fixture.Worker);
            Assert.That(position.Query<Label>().ToList().Count(l => l.text == OfficerGuidance("Farmer")), Is.EqualTo(2),
                "Both the contextual reason and empty queue use the same ownership explanation as the no-input fixture.");
            Assert.That(Labels(position).ToLowerInvariant(), Does.Not.Contain("missing").And.Not.Contain("target").And.Not.Contain("1000").And.Not.Contain("40"));
            Assert.That(position.Q<Button>("worker-add-" + fixture.Mill.Id + "-" + fixture.Worker.Id).enabledSelf, Is.True);
            Assert.That(Root.Q<Button>("station-add-" + fixture.Mill.Id).enabledSelf, Is.True);
            string authority = JsonUtility.ToJson(game.CurrentWorld), action = game.LastAction;
            int revision = game.Revision;
            yield return null;
            foreach (string section in new[] { "Work & controls", "Queue" })
            {
                var navigation = Root.Q<DropdownField>("workshop-navigation");
                Assert.That(Root.Q<ScrollView>(className: "drawer-scroll").Contains(navigation), Is.False);
                navigation.value = "Position 1 · " + fixture.Worker.Name + " · " + section;
                yield return null;
                Assert.That(Root.Q<Label>("workshop-navigation-context").text, Does.Contain(fixture.Worker.Name).And.Contain(section));
            }
            Submit(CurrentPosition(fixture.Worker).Query<Button>().ToList().Single(b => b.text == "Inspect worker · " + fixture.Worker.Name));
            Assert.That(game.View.SelectedCat, Is.SameAs(fixture.Worker));
            Submit(Root.Query<Button>().ToList().Single(b => b.text == "Back"));
            Assert.That(game.View.SelectedBuilding, Is.SameAs(fixture.Mill));
            RefreshWithoutAuthorityChange();
            Assert.That(BuildingHover(fixture.Mill), Does.Contain("Waiting for Farmer to choose work").And.Contain(OfficerGuidance("Farmer")));
            game.UI.OpenPanel("Work");
            game.UI.OpenPanel("Officers");
            RenderPosition(fixture.Mill, fixture.Worker);
            Assert.That(JsonUtility.ToJson(game.CurrentWorld), Is.EqualTo(authority));
            Assert.That(game.LastAction, Is.EqualTo(action));
            Assert.That(game.Revision, Is.EqualTo(revision));
            Assert.That(game.Speed, Is.Zero);
        }

        private const System.Reflection.BindingFlags Private = System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic;
        private static string OfficerGuidance(string role) => "The " + role + " chooses work automatically. Add a recipe to take manual control of this position.";
        private static string Labels(VisualElement element) => string.Join("\n", element.Query<Label>().ToList().Select(label => label.text));
        private static void AssertNoOfficerWait(VisualElement position)
        {
            Assert.That(position.Q<Label>(className: "worker-status").text, Does.Not.Contain("to choose work"));
            Assert.That(Labels(position), Does.Not.Contain(OfficerGuidance("Farmer")).And.Not.Contain(OfficerGuidance("Forester")));
        }
        private VisualElement CurrentPosition(Cat worker) => Root.Query<VisualElement>(className: "work-position").ToList()
            .Single(position => position.Query<Button>().ToList().Any(button => button.text == "Release worker · " + worker.Name));
        private VisualElement RenderPosition(Building building, Cat worker)
        {
            game.View.InspectBuilding(building.Id); game.UI.OpenPanel("Inspect");
            return CurrentPosition(worker);
        }
        private string WorkerStatus(Building building, WorkSlot slot, Cat cat)
            => (string)typeof(ForestUI).GetMethod("WorkerStatus", Private).Invoke(game.UI, new object[] { building, slot, cat });
        private string BuildingHover(Building building)
        {
            game.View.InspectBuilding(building.Id); game.UI.ClosePanel();
            typeof(ForestUI).GetMethod("UpdateTetheredInspector", Private).Invoke(game.UI, null);
            return ((Label)typeof(ForestUI).GetField("tetheredInspectorBody", Private).GetValue(game.UI)).text;
        }
        private void Submit(Button button)
        {
            Assert.That(button, Is.Not.Null);
            Assert.That(button.panel, Is.SameAs(Root.panel));
            Assert.That(button.enabledInHierarchy, Is.True);
            button.Focus();
            using (var submit = NavigationSubmitEvent.GetPooled()) { submit.target = button; button.SendEvent(submit); }
        }
        private void RefreshWithoutAuthorityChange()
        {
            string before = JsonUtility.ToJson(game.CurrentWorld), action = game.LastAction;
            int revision = game.Revision;
            Root.panel.focusController.focusedElement?.Blur();
            typeof(ForestUI).GetField("nextRefresh", Private).SetValue(game.UI, 0f);
            typeof(ForestUI).GetMethod("Update", Private).Invoke(game.UI, null);
            Assert.That(JsonUtility.ToJson(game.CurrentWorld), Is.EqualTo(before));
            Assert.That(game.LastAction, Is.EqualTo(action));
            Assert.That(game.Revision, Is.EqualTo(revision));
        }
        private (Building Mill, Cat Worker, Cat Farmer, WorkSlot Slot) PrepareWaitingMill(bool fullTargets = false)
        {
            var world = game.CurrentWorld; var village = game.Selected;
            foreach (var cat in village.Cats) cat.Health = cat.Hunger = cat.Thirst = cat.Rest = 100;
            foreach (var pile in village.Stockpiles) { pile.Goods.Clear(); pile.Report.Clear(); pile.CountedAt = -1; }
            var store = village.Stockpiles.First(p => p.Kind == "storage" && p.ManagedBy == "");
            World.Add(store.Goods, "food", 1000); World.Add(store.Goods, "water", 1000); World.Add(store.Goods, "fibre", 10);
            if (fullTargets) { World.Add(store.Goods, "flour", 40); World.Add(store.Goods, "grain", 10); }
            string[] studies = { "research_hut", "basic_tools", "water_carriers", "foraging_lore", "sawmill", "masonry", "irrigation", "milling", "grain_milling_sources", "grain_milling_preparation", "grain_milling_staples" };
            village.ResearchPoints = studies.Sum(id => Catalog.Study(id).Cost);
            foreach (string study in studies) Act(new GameAction { Kind = "ResearchNode", NodeId = study });
            CompletedSupport("field", new Int2(10, 5));
            var mill = CompletedSupport("mill", new Int2(-6, -6));
            var adults = village.Cats.Where(c => c.Alive && c.AgeHours >= 12 && c.Id != village.LeaderId && c.OfficerRole == ""
                && c.ControlledBy == "" && c.JobId == "" && c.ResumeJobId == "" && c.BuildingId == "").Take(2).ToArray();
            Assert.That(adults.Length, Is.EqualTo(2));
            Act(new GameAction { Kind = "AssignOfficer", Role = "farmer", CatId = adults[0].Id });
            Act(new GameAction { Kind = "AssignWorker", BuildingId = mill.Id, CatId = adults[1].Id });
            Act(new GameAction { Kind = "EditProductionWorkSlot", BuildingId = mill.Id, CatId = adults[1].Id, Edit = "delegate" });
            world.Step(11.05);
            var slot = mill.Slots.Single(s => s.CatId == adults[1].Id);
            Assert.That(slot.AutomatedBy, Is.EqualTo("farmer"));
            Assert.That(slot.Queue, Is.Empty); Assert.That(slot.BlockedReason, Is.EqualTo("queue_empty"));
            Assert.That(adults[1].JobId, Is.Empty); Assert.That(adults[1].ResumeJobId, Is.Empty);
            Assert.That(world.Validate(), Is.Empty);
            return (mill, adults[1], adults[0], slot);
        }

        private void Act(GameAction action)
        {
            var result = game.Send(action).Result;
            Assert.That(result.Success, Is.True, action.Kind + ": " + result.Error);
        }

        private Building CompletedSupport(string kind, Int2 at)
        {
            var world = game.CurrentWorld; var village = game.Selected;
            for (int x = 0; x < 2; x++)
                for (int z = 0; z < 2; z++)
                {
                    var tile = new Int2(at.X + x, at.Z + z);
                    Assert.That(village.Buildings.Any(b => tile.X >= b.Position.X && tile.X < b.Position.X + b.Width
                        && tile.Z >= b.Position.Z && tile.Z < b.Position.Z + b.Depth), Is.False, "Completed fixture support must not overlap an existing building.");
                    World.ClearSurface(world.TileAt(tile), 0);
                    if (!village.Known.Contains(tile)) village.Known.Add(tile);
                }
            var building = new Building { Id = world.Id("officer-wait-" + kind), Kind = kind, Position = at, Completed = true, ConstructionConsumed = true };
            village.Buildings.Add(building);
            return building;
        }
    }
}
