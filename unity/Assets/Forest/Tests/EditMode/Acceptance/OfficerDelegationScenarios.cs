using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class OfficerDelegationScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.officer_delegation_retains_paused_manual_order", DelegatePausedManualOrder);
            yield return new Scenario("regression.officer_delegation_missing_officer_preserves_provenance", MissingOfficerPreservesProvenance);
            yield return new Scenario("regression.officer_delegation_empty_queue_returns_to_planner", EmptyQueueReturnsToPlanner);
            yield return new Scenario("regression.officer_delegation_preserves_claimed_inputs", () => PreserveActiveProduction("claimed"));
            yield return new Scenario("regression.officer_delegation_preserves_carried_inputs", () => PreserveActiveProduction("carried"));
            yield return new Scenario("regression.officer_delegation_preserves_working_progress", () => PreserveActiveProduction("working"));
            yield return new Scenario("regression.officer_delegation_preserves_multiple_manual_orders", PreserveMultipleManualOrders);
            yield return new Scenario("regression.officer_delegation_rejects_passive_station", () => RejectUnsupportedStation("den"));
            yield return new Scenario("regression.officer_delegation_rejects_training_station", () => RejectUnsupportedStation("barracks"));
            yield return new Scenario("regression.officer_delegation_rejects_station_wide_target", () => RejectInvalidWorker("station"));
            yield return new Scenario("regression.officer_delegation_rejects_unstaffed_worker", () => RejectInvalidWorker("unstaffed"));
            yield return new Scenario("regression.officer_delegation_rejects_dead_worker", () => RejectInvalidWorker("dead"));
            yield return new Scenario("regression.officer_delegation_rejects_stale_assignment", () => RejectInvalidWorker("stale"));
            yield return new Scenario("regression.officer_delegation_rejects_dead_officer", RejectDeadOfficer);
            yield return new Scenario("regression.officer_delegation_rejects_foreign_village", RejectForeignVillage);
            yield return new Scenario("regression.officer_delegation_research_retains_exact_worker", DelegateResearchWorker);
            yield return new Scenario("acceptance.officer_delegation_finite_manual_then_automatic_chain", FiniteManualThenAutomaticChain);
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }

        static ActionResult Apply(World world, Village village, GameAction action)
        {
            return world.Apply(new PlayerContext { PlayerId = "officer-delegation-acceptance", VillageId = village.Id }, action);
        }

        static void Act(World world, Village village, GameAction action)
        {
            var result = Apply(world, village, action);
            Check(result.Success, action.Kind + ": " + result.Error);
        }

        static World Fixture(bool appointOfficer, out Village village, out Building station, out Cat worker)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            village.Stockpiles[0].Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000), new Stack("fibre", 5) };
            village.Stockpiles[0].Capacity = 100000;
            village.Stockpiles[0].Position = new Int2(3, 2);
            village.LastLeaderResearch = 0;
            foreach (var cat in village.Cats)
            {
                cat.ControlledBy = "fixture-held";
                cat.ControlLeaseUntil = 10000000;
                cat.Position = new Int2(1, 3);
                cat.X = 1;
                cat.Z = 3;
            }
            worker = village.Cats[1];
            worker.ControlledBy = "";
            station = new Building { Id = world.Id("officer-delegation-station"), Kind = "clothier", Position = worker.Position, Completed = true };
            village.Buildings.Add(station);
            village.Research.Add("textiles");
            if (appointOfficer)
                Act(world, village, new GameAction { Kind = "AssignOfficer", Role = "cloth_leader", CatId = village.Cats[0].Id });
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = worker.Id });
            return world;
        }

        static GameAction Delegate(Building station, Cat worker) => new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "delegate" };

        static void DelegatePausedManualOrder()
        {
            var world = Fixture(true, out var village, out var station, out var worker);
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "add", RecipeId = "fibre_to_thread", Repeat = true });
            var slot = station.Slots.Single();
            var order = slot.Queue.Single();
            slot.Paused = true;
            station.Paused = true;
            var result = Apply(world, village, Delegate(station, worker));
            Check(result.Success, "An explicit return to the appointed Cloth Leader was refused: " + result.Error);
            Check(slot.AutomatedBy == "cloth_leader" && slot.CatId == worker.Id && worker.BuildingId == station.Id, "Delegation did not retain the exact worker under its relevant officer");
            Check(slot.Paused && station.Paused && worker.JobId == "" && world.Reservations.Count == 0, "Delegation resumed or started the player's paused work");
            Check(ReferenceEquals(slot.Queue.Single(), order) && order.Repeat && order.AutomatedBy == "" && order.RecipeId == "fibre_to_thread", "Delegation replaced or relabeled the player's deliberate repeating order");
        }

        static void MissingOfficerPreservesProvenance()
        {
            var world = Fixture(false, out var village, out var station, out var worker);
            var slot = station.Slots.Single();
            slot.AutomatedBy = "cloth_leader";
            var order = new QueueEntry { RecipeId = "fibre_to_thread", AutomatedBy = "cloth_leader", Repeat = true };
            slot.Queue.Add(order);
            slot.Paused = true;
            var result = Apply(world, village, Delegate(station, worker));
            Check(!result.Success && result.Error.Contains("officer", StringComparison.OrdinalIgnoreCase), "Delegation without a living relevant officer must explain that vacancy");
            Check(slot.AutomatedBy == "cloth_leader" && ReferenceEquals(slot.Queue.Single(), order) && order.AutomatedBy == "cloth_leader" && order.Repeat && slot.Paused, "A refused delegation cleared existing slot or queue provenance");
            Check(slot.CatId == worker.Id && worker.BuildingId == station.Id && worker.JobId == "" && world.Reservations.Count == 0, "A refused delegation changed the worker or created work");
        }

        static void EmptyQueueReturnsToPlanner()
        {
            var world = Fixture(true, out var village, out var station, out var worker);
            var slot = station.Slots.Single();
            Act(world, village, Delegate(station, worker));
            Check(slot.Queue.Count == 0 && worker.JobId == "", "Delegation itself created a recipe or bypassed normal planning");
            world.Step(11.05);
            var job = village.Jobs.SingleOrDefault(j => j.Id == worker.JobId && j.Kind == "production");
            Check(slot.AutomatedBy == "cloth_leader" && slot.CatId == worker.Id && worker.BuildingId == station.Id, "The officer replaced its explicitly retained worker");
            Check(job != null && job.RecipeId == "fibre_to_thread" && slot.Queue.Single().AutomatedBy == "cloth_leader", "The delegated empty position did not receive a normal funded officer recipe");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static Job ReachProduction(World world, Village village, Cat worker, string phase)
        {
            for (int tick = 0; tick < 4000; tick++)
            {
                world.Step(0.05);
                var job = village.Jobs.Find(j => j.Id == worker.JobId && j.Kind == "production");
                if (job != null && (phase == "claimed" ? world.Reservations.Any(r => r.OwnerId == job.Id) : phase == "carried" ? World.Amount(worker.Cargo, "fibre") > 0 : job.Phase == "working" && job.Progress > 0))
                    return job;
            }
            throw new InvalidOperationException("Manual finite production did not reach " + phase + " ownership");
        }

        static void PreserveActiveProduction(string phase)
        {
            var world = Fixture(true, out var village, out var station, out var worker);
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "add", RecipeId = "fibre_to_thread" });
            var slot = station.Slots.Single();
            var order = slot.Queue.Single();
            var job = ReachProduction(world, village, worker, phase);
            string jobId = worker.JobId, slotJobId = slot.JobId, jobOfficer = job.AutomatedBy, requester = job.Requester, jobPhase = job.Phase;
            double progress = job.Progress, required = job.RequiredWork, x = worker.X, z = worker.Z;
            bool manual = job.Manual;
            var claims = world.Reservations.Select(r => (r.OwnerId, r.VillageId, r.PileId, r.Resource, r.Amount)).ToArray();
            var cargo = worker.Cargo.Select(s => (s.Resource, s.Amount)).ToArray();
            var local = job.Local.Select(s => (s.Resource, s.Amount)).ToArray();
            var reserved = job.Reserved.Select(s => (s.Resource, s.Amount)).ToArray();
            var path = worker.Path.ToArray();
            Act(world, village, Delegate(station, worker));
            Check(slot.AutomatedBy == "cloth_leader" && worker.JobId == jobId && slot.JobId == slotJobId && ReferenceEquals(village.Jobs.Single(j => j.Id == jobId), job), "Delegation replaced the exact active production owner during " + phase);
            Check(job.Progress == progress && job.RequiredWork == required && job.Phase == jobPhase && job.AutomatedBy == jobOfficer && job.Manual == manual && job.Requester == requester, "Delegation reset or relabeled active production during " + phase);
            Check(world.Reservations.Select(r => (r.OwnerId, r.VillageId, r.PileId, r.Resource, r.Amount)).SequenceEqual(claims) && worker.Cargo.Select(s => (s.Resource, s.Amount)).SequenceEqual(cargo) && job.Local.Select(s => (s.Resource, s.Amount)).SequenceEqual(local) && job.Reserved.Select(s => (s.Resource, s.Amount)).SequenceEqual(reserved), "Delegation moved, released or duplicated physical ingredients during " + phase);
            Check(worker.X == x && worker.Z == z && worker.Path.SequenceEqual(path) && ReferenceEquals(slot.Queue.Single(), order) && order.AutomatedBy == "" && !order.Repeat, "Delegation changed travel or the deliberate finite order during " + phase);
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void PreserveMultipleManualOrders()
        {
            var world = Fixture(true, out var village, out var station, out var worker);
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "add", RecipeId = "fibre_to_cloth", Repeat = true });
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "add", RecipeId = "fibre_to_thread" });
            var slot = station.Slots.Single();
            var orders = slot.Queue.ToArray();
            Act(world, village, Delegate(station, worker));
            world.Step(21.05);
            Check(slot.AutomatedBy == "cloth_leader" && slot.Queue.SequenceEqual(orders) && orders[0].RecipeId == "fibre_to_cloth" && orders[0].Repeat && orders[1].RecipeId == "fibre_to_thread" && !orders[1].Repeat && orders.All(q => q.AutomatedBy == ""), "The officer reordered, repeated, dropped or relabeled the player's manual queue");
            Check(worker.JobId == "" && world.Reservations.Count == 0 && world.Total(village, "fibre") == 5, "The officer bypassed the player's unfunded first order");
        }

        static QueueEntry MarkExistingProvenance(WorkSlot slot)
        {
            slot.AutomatedBy = "cloth_leader";
            slot.Paused = true;
            var order = new QueueEntry { RecipeId = "fibre_to_thread", Repeat = true, AutomatedBy = "cloth_leader" };
            slot.Queue.Add(order);
            return order;
        }

        static void CheckRejectedPosition(ActionResult result, Building station, Cat worker, WorkSlot slot, QueueEntry order)
        {
            Check(!result.Success, "An invalid delegation target was accepted");
            Check(slot.CatId == worker.Id && slot.AutomatedBy == "cloth_leader" && slot.Paused && ReferenceEquals(slot.Queue.Single(), order) && order.AutomatedBy == "cloth_leader" && order.Repeat, "Refused delegation changed the exact worker, pause or queue provenance");
            Check(station.Slots.Single() == slot && station.Queue == slot.Queue && worker.JobId == "", "Refused delegation replaced the slot, queue or active job");
        }

        static void RejectUnsupportedStation(string kind)
        {
            var world = Fixture(true, out var village, out var station, out var worker);
            var slot = station.Slots.Single();
            var order = MarkExistingProvenance(slot);
            station.Kind = kind;
            var result = Apply(world, village, Delegate(station, worker));
            CheckRejectedPosition(result, station, worker, slot, order);
            Check(result.Error.Contains("station", StringComparison.OrdinalIgnoreCase), "Unsupported delegation did not explain the station restriction");
        }

        static void RejectInvalidWorker(string target)
        {
            var world = Fixture(true, out var village, out var station, out var worker);
            var slot = station.Slots.Single();
            var order = MarkExistingProvenance(slot);
            var action = Delegate(station, worker);
            if (target == "station") action.Kind = "EditProductionQueue";
            else if (target == "unstaffed") action.CatId = village.Cats[2].Id;
            else if (target == "dead") worker.Alive = false;
            else worker.BuildingId = "";
            var result = Apply(world, village, action);
            CheckRejectedPosition(result, station, worker, slot, order);
            Check(result.Error.Contains("slot", StringComparison.OrdinalIgnoreCase), "Invalid delegation did not explain the exact staffed position requirement");
        }

        static void RejectDeadOfficer()
        {
            var world = Fixture(true, out var village, out var station, out var worker);
            village.Cats[0].Alive = false;
            var slot = station.Slots.Single();
            var order = MarkExistingProvenance(slot);
            var result = Apply(world, village, Delegate(station, worker));
            CheckRejectedPosition(result, station, worker, slot, order);
            Check(result.Error.Contains("officer", StringComparison.OrdinalIgnoreCase), "A dead officer must not accept delegated work");
        }

        static void RejectForeignVillage()
        {
            var world = Fixture(true, out var village, out var station, out var worker);
            var owner = new PlayerContext { PlayerId = "delegation-private-owner", VillageId = village.Id };
            var founded = world.Apply(owner, new GameAction { Kind = "FoundVillage", Name = "Private delegation village" });
            Check(founded.Success, "Private ownership fixture could not found its village");
            var foreign = world.Villages.Single(v => v.Id == founded.VillageId);
            var foreignStation = foreign.Buildings.First(b => b.Completed && b.Kind == "wood_cutter");
            var foreignWorker = foreign.Cats[1];
            var staffed = world.Apply(new PlayerContext { PlayerId = owner.PlayerId, VillageId = foreign.Id }, new GameAction { Kind = "AssignWorker", BuildingId = foreignStation.Id, CatId = foreignWorker.Id });
            Check(staffed.Success, "Private ownership fixture could not assign its worker");
            var slot = foreignStation.Slots.Single();
            var order = MarkExistingProvenance(slot);
            var result = world.Apply(new PlayerContext { PlayerId = "delegation-other-player", VillageId = foreign.Id }, Delegate(foreignStation, foreignWorker));
            CheckRejectedPosition(result, foreignStation, foreignWorker, slot, order);
            Check(result.Error.Contains("ownership", StringComparison.OrdinalIgnoreCase), "Foreign delegation bypassed village authority");
        }

        static void DelegateResearchWorker()
        {
            var world = Fixture(false, out var village, out var station, out var worker);
            station.Kind = "research_hut";
            village.Research.Add("research_hut");
            Act(world, village, new GameAction { Kind = "AssignOfficer", Role = "loremaster", CatId = village.Cats[0].Id });
            var slot = station.Slots.Single();
            Act(world, village, Delegate(station, worker));
            Check(slot.AutomatedBy == "loremaster" && slot.Queue.Count == 0 && slot.CatId == worker.Id && worker.BuildingId == station.Id, "Research delegation did not retain its exact worker under the Loremaster");
            double points = village.ResearchPoints;
            world.Step(11.05);
            Check(village.ResearchPoints > points && slot.CatId == worker.Id && slot.Queue.Count == 0 && slot.AutomatedBy == "loremaster", "Delegated research failed to continue without a fabricated production recipe");
        }

        static void FiniteManualThenAutomaticChain()
        {
            var world = Fixture(true, out var village, out var station, out var worker);
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "add", RecipeId = "fibre_to_thread" });
            var slot = station.Slots.Single();
            var manualOrder = slot.Queue.Single();
            var manualJob = ReachProduction(world, village, worker, "claimed");
            Act(world, village, Delegate(station, worker));
            for (int second = 0; second < 900 && !manualJob.Completed; second++) world.Step(1);
            Check(manualJob.Completed && manualOrder.AutomatedBy == "" && !manualOrder.Repeat && world.Total(village, "fibre") == 0 && world.Total(village, "thread") == 5, "Delegation did not complete the retained finite manual batch exactly once");
            Job automaticJob = null;
            for (int second = 0; second < 900 && (automaticJob == null || !automaticJob.Completed); second++)
            {
                world.Step(1);
                automaticJob ??= village.Jobs.FirstOrDefault(j => j.CatId == worker.Id && j.RecipeId == "fibre_to_cloth");
            }
            Check(automaticJob != null && automaticJob.Completed && world.Total(village, "thread") == 0 && world.Total(village, "cloth") == 1, "The delegated officer did not continue the finite manual Thread batch into exactly one physical Cloth");
            Check(slot.CatId == worker.Id && worker.BuildingId == station.Id && slot.AutomatedBy == "cloth_leader" && slot.Queue.Count == 0 && worker.JobId == "" && worker.Cargo.Count == 0 && world.Reservations.Count == 0 && station.Inputs.Count == 0 && !village.Jobs.Any(j => !j.Completed && j.Kind == "production"), "Delegated finite production retained duplicate claims, cargo, queued work or changed worker");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }
    }
}
