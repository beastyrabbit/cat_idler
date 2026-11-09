using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class ComposedWorkshopScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            foreach (string refusal in new[] { "locked_recipe", "wrong_station", "unknown_recipe", "negative_index", "missing_index", "move_edge", "unknown_edit", "queue_limit" })
            {
                string edit = refusal;
                yield return new Scenario("regression.composed_workshop_rejected_queue_edit_" + edit, () => RejectedEditPreservesOfficer(edit, true));
            }
            yield return new Scenario("regression.composed_workshop_rejected_station_queue_edit", () => RejectedEditPreservesOfficer("move_edge", false));
            foreach (string phase in new[] { "claimed", "carried", "working", "needs" })
            {
                string ownership = phase;
                yield return new Scenario("regression.composed_workshop_active_order_remove_" + ownership, () => ActiveOrderEdit(ownership, "remove"));
            }
            yield return new Scenario("regression.composed_workshop_active_order_move", () => ActiveOrderEdit("claimed", "move"));
            yield return new Scenario("regression.composed_workshop_active_order_crossing", () => ActiveOrderEdit("claimed", "crossing"));
            yield return new Scenario("regression.composed_workshop_active_order_future_edits", FutureOrderEdits);
            yield return new Scenario("regression.composed_workshop_committed_order_allows_edits", CommittedOrderEdits);
            yield return new Scenario("acceptance.composed_workshop_finite_concurrent_tool_chain", FiniteConcurrentToolChain);
            yield return new Scenario("acceptance.composed_workshop_duplicate_finite_orders_deliver_twice", DuplicateFiniteOrders);
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static ActionResult Apply(World world, Village village, GameAction action) => world.Apply(new PlayerContext { PlayerId = "composed-workshop-acceptance", VillageId = village.Id }, action);
        static void Act(World world, Village village, GameAction action)
        {
            var result = Apply(world, village, action);
            Check(result.Success, action.Kind + ": " + result.Error);
        }
        static GameAction Edit(Building station, Cat worker, string edit) => new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = edit };

        static World Fixture(out Village village, out Building station, out Cat worker)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            village.Stockpiles[0].Position = new Int2(3, 2);
            village.Stockpiles[0].Capacity = 100000;
            village.Stockpiles[0].Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000) };
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
            station = new Building { Id = world.Id("composed-bench"), Kind = "woodworking", Completed = true, Position = new Int2(1, 5) };
            village.Buildings.Add(station);
            village.Buildings.Add(new Building { Id = world.Id("composed-office"), Kind = "sawmill", Completed = true, Position = new Int2(-2, 5) });
            village.Research.Add("sawmill");
            Act(world, village, new GameAction { Kind = "AssignOfficer", CatId = village.Cats[0].Id, Role = "forester" });
            Act(world, village, new GameAction { Kind = "AssignWorker", CatId = worker.Id, BuildingId = station.Id });
            return world;
        }

        static void RejectedEditPreservesOfficer(string refusal, bool exact)
        {
            var world = Fixture(out var village, out var station, out var worker);
            var add = Edit(station, worker, "add");
            add.RecipeId = "planks_and_blocks_to_tools";
            add.Repeat = true;
            Act(world, village, add);
            var pause = Edit(station, worker, "pause");
            pause.Enabled = true;
            Act(world, village, pause);
            Act(world, village, Edit(station, worker, "delegate"));
            var slot = station.Slots.Single();
            slot.Queue[0].AutomatedBy = "forester";
            if (refusal == "queue_limit")
                for (int i = 1; i < 64; i++) slot.Queue.Add(new QueueEntry { RecipeId = "planks_and_blocks_to_tools", AutomatedBy = "forester", Repeat = i % 2 == 0 });
            var orders = slot.Queue.ToArray();
            var provenance = orders.Select(q => (q.RecipeId, q.AutomatedBy, q.Repeat)).ToArray();
            long nextId = world.NextId;
            uint entropy = world.RandomState;
            double time = world.TimeSeconds;
            var action = Edit(station, worker, "remove");
            if (!exact) action.Kind = "EditProductionQueue";
            if (refusal == "locked_recipe" || refusal == "wrong_station" || refusal == "unknown_recipe" || refusal == "queue_limit")
            {
                action.Edit = "add";
                action.RecipeId = refusal == "locked_recipe" ? "bone_tool" : refusal == "wrong_station" ? "logs_to_planks" : refusal == "unknown_recipe" ? "missing-recipe" : "planks_and_blocks_to_tools";
            }
            else if (refusal == "negative_index") action.Index = -1;
            else if (refusal == "missing_index") action.Index = slot.Queue.Count;
            else if (refusal == "move_edge") { action.Edit = "move"; action.Index = 0; action.Direction = -1; }
            else action.Edit = "missing-edit";
            var result = Apply(world, village, action);
            Check(!result.Success, "Invalid queue edit was accepted: " + refusal);
            Check(slot.AutomatedBy == "forester" && orders.Select(q => q.AutomatedBy).SequenceEqual(provenance.Select(q => q.AutomatedBy)), "Rejected " + refusal + " silently disabled officer ownership");
            Check(slot.Paused && !station.Paused && slot.Queue.SequenceEqual(orders) && ReferenceEquals(station.Queue, slot.Queue) && orders.Select(q => (q.RecipeId, q.AutomatedBy, q.Repeat)).SequenceEqual(provenance), "Rejected edit changed the paused finite queue, order identity or repeat settings");
            Check(slot.CatId == worker.Id && worker.BuildingId == station.Id && worker.JobId == "" && slot.JobId == "" && village.Jobs.Count == 0 && world.Reservations.Count == 0 && worker.Cargo.Count == 0, "Rejected edit changed worker or physical material ownership");
            Check(world.NextId == nextId && world.RandomState == entropy && world.TimeSeconds == time, "Rejected edit consumed identity, entropy or simulation time");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void Wait(World world, Func<bool> ready, int seconds, string message)
        {
            for (int elapsed = 0; elapsed < seconds && !ready(); elapsed++) world.Step(1);
            Check(ready(), message);
        }

        static World ActiveFixture(string phase, out Village village, out Building station, out Cat worker, out Job job)
        {
            var world = Fixture(out village, out station, out worker);
            village.Cats.RemoveRange(2, village.Cats.Count - 2);
            World.Add(village.Stockpiles[0].Goods, "planks", 6);
            World.Add(village.Stockpiles[0].Goods, "blocks", 6);
            for (int i = 0; i < 3; i++)
            {
                var action = Edit(station, worker, "add");
                action.RecipeId = "planks_and_blocks_to_tools";
                Act(world, village, action);
            }
            Act(world, village, Edit(station, worker, "delegate"));
            world.Step(1.05);
            var activeWorker = worker;
            job = village.Jobs.Single(j => !j.Completed && j.CatId == activeWorker.Id);
            var activeJob = job;
            if (phase == "carried" || phase == "needs")
            {
                for (int tick = 0; tick < 1000 && worker.Cargo.Count == 0; tick++) world.Step(.05);
                Check(worker.Cargo.Count > 0, "Active-order fixture never physically picked up its finite input");
            }
            else if (phase == "working")
                Wait(world, () => activeJob.Phase == "working" && activeJob.Progress > 0, 60, "Active-order fixture never physically staged its finite bill");
            if (phase == "needs")
            {
                worker.Thirst = 1;
                world.Step(.05);
                Check(worker.Goal == "need_drink" && worker.ResumeJobId == activeJob.Id && activeJob.SuspendedCargoPileId != "" && world.Reservations.Any(r => r.OwnerId == activeJob.Id + ":resume"), "Critical thirst did not suspend the same physical production batch");
            }
            return world;
        }

        static void ActiveOrderEdit(string phase, string edit)
        {
            var world = ActiveFixture(phase, out var village, out var station, out var worker, out var job);
            var slot = station.Slots.Single();
            var orders = slot.Queue.ToArray();
            var claims = world.Reservations.Select(r => (r.OwnerId, r.PileId, r.Resource, r.Amount)).ToArray();
            var cargo = worker.Cargo.Select(s => (s.Resource, s.Amount)).ToArray();
            var local = job.Local.Select(s => (s.Resource, s.Amount)).ToArray();
            string jobId = worker.JobId, resumeJobId = worker.ResumeJobId, jobPhase = job.Phase;
            double progress = job.Progress;
            var action = Edit(station, worker, edit == "remove" ? "remove" : "move");
            action.Index = edit == "crossing" ? 1 : 0;
            action.Direction = edit == "crossing" ? -1 : 1;
            var result = Apply(world, village, action);
            Check(!result.Success && result.Error.Contains("progress", StringComparison.OrdinalIgnoreCase), "An " + phase + " batch allowed its finite order to be removed, moved or crossed without an in-progress explanation");
            Check(slot.Queue.SequenceEqual(orders) && slot.AutomatedBy == "forester" && orders.All(q => q.AutomatedBy == "" && !q.Repeat), "Refused active-order edit replaced an order or changed officer/manual ownership");
            Check(worker.JobId == jobId && worker.ResumeJobId == resumeJobId && job.Phase == jobPhase && job.Progress == progress && world.Reservations.Select(r => (r.OwnerId, r.PileId, r.Resource, r.Amount)).SequenceEqual(claims) && worker.Cargo.Select(s => (s.Resource, s.Amount)).SequenceEqual(cargo) && job.Local.Select(s => (s.Resource, s.Amount)).SequenceEqual(local), "Refused active-order edit changed the real batch, its needs resume or physical materials");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void FutureOrderEdits()
        {
            var world = ActiveFixture("claimed", out var village, out var station, out var worker, out var job);
            var slot = station.Slots.Single();
            var activeOrder = slot.Queue[0];
            var second = slot.Queue[1];
            var third = slot.Queue[2];
            var move = Edit(station, worker, "move");
            move.Index = 2;
            move.Direction = -1;
            Act(world, village, move);
            Check(ReferenceEquals(slot.Queue[0], activeOrder) && ReferenceEquals(slot.Queue[1], third) && ReferenceEquals(slot.Queue[2], second), "Editing future order priority changed the active batch");
            var remove = Edit(station, worker, "remove");
            remove.Index = 1;
            Act(world, village, remove);
            Check(slot.Queue.Count == 2 && ReferenceEquals(slot.Queue[0], activeOrder) && ReferenceEquals(slot.Queue[1], second) && !job.Completed && worker.JobId == job.Id && world.Reservations.Where(r => r.OwnerId == job.Id).Sum(r => r.Amount) == 4 && slot.AutomatedBy == "", "Successful future removal lost the pinned active order, source claims or deliberate manual takeover");
        }

        static void CommittedOrderEdits()
        {
            var world = Fixture(out var village, out var station, out var worker);
            var order = new QueueEntry { RecipeId = "planks_and_blocks_to_tools" };
            station.Slots.Single().Queue.Add(order);
            Act(world, village, Edit(station, worker, "delegate"));
            // This fixture represents a batch whose recipe and queue commit already
            // completed. Its exact output is still owned by the delivery job.
            var job = new Job { Id = world.Id("committed-production"), CatId = worker.Id, Kind = "production", RecipeId = order.RecipeId, TargetId = station.Id, Position = station.Position, Phase = "output_delivery" };
            var item = new Item { Id = world.Id("committed-tool"), Kind = "tool", Material = "wood", VillageId = village.Id, LocationId = job.Id };
            job.ItemIds.Add(item.Id);
            village.Jobs.Add(job);
            village.Items.Add(item);
            worker.JobId = job.Id;
            station.Slots.Single().JobId = job.Id;
            Act(world, village, Edit(station, worker, "remove"));
            Check(station.Slots.Single().Queue.Count == 0 && station.Slots.Single().AutomatedBy == "" && worker.JobId == job.Id && job.ItemIds.Single() == item.Id && item.LocationId == job.Id && !job.Completed, "Removing a future order during output delivery disturbed the already committed exact item");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void DuplicateFiniteOrders()
        {
            var world = Fixture(out var village, out var station, out var worker);
            village.Cats.RemoveRange(2, village.Cats.Count - 2);
            World.Add(village.Stockpiles[0].Goods, "planks", 4);
            World.Add(village.Stockpiles[0].Goods, "blocks", 4);
            for (int i = 0; i < 2; i++)
            {
                var add = Edit(station, worker, "add");
                add.RecipeId = "planks_and_blocks_to_tools";
                Act(world, village, add);
            }
            var orders = station.Slots.Single().Queue.ToArray();
            world.Step(1.05);
            var first = village.Jobs.Single(j => !j.Completed && j.CatId == worker.Id);
            var remove = Edit(station, worker, "remove");
            Check(!Apply(world, village, remove).Success && station.Slots.Single().Queue.SequenceEqual(orders), "Removing the active finite order consumed or exposed its identical successor");
            Wait(world, () => station.Slots.Single().Queue.Count == 1, 900, "The first finite batch did not commit its exact order");
            Check(ReferenceEquals(station.Slots.Single().Queue.Single(), orders[1]) && !ReferenceEquals(station.Slots.Single().Queue.Single(), orders[0]), "The first completed batch removed the wrong identical finite order");
            Wait(world, () => worker.JobId == "" && station.Slots.Single().Queue.Count == 0, 900, "The second identical finite order failed to deliver");
            world.Step(31);
            Check(village.Items.Count == 2 && village.Items.Select(i => i.Id).Distinct().Count() == 2 && village.Items.All(i => i.Kind == "tool" && i.Material == "wood" && i.Quality == 1 && i.Condition == 100 && village.Stockpiles.Any(p => p.Id == i.LocationId)), "Two finite orders did not physically deliver exactly two distinct unchanged tools");
            Check(world.Total(village, "planks") == 0 && world.Total(village, "blocks") == 0 && worker.Cargo.Count == 0 && station.Inputs.Count == 0 && world.Reservations.Count == 0 && !village.Jobs.Any(j => !j.Completed) && village.Events.Count(e => e.Kind == "completed" && e.EntityId == first.Id) == 1, "Duplicate finite recipe orders lost conservation, replayed completion or retained unfinished work");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void FiniteConcurrentToolChain()
        {
            var world = Fixture(out var village, out var bench, out var crafter);
            village.Cats.RemoveRange(4, village.Cats.Count - 4);
            World.Add(village.Stockpiles[0].Goods, "logs", 10);
            World.Add(village.Stockpiles[0].Goods, "stone", 10);
            var cutter = new Building { Id = world.Id("composed-cutter"), Kind = "wood_cutter", Completed = true, Position = new Int2(1, 3) };
            var prep = new Building { Id = world.Id("composed-prep"), Kind = "stone_prep", Completed = true, Position = new Int2(3, 4) };
            village.Buildings.Add(cutter);
            village.Buildings.Add(prep);
            var sawyer = village.Cats[2];
            var mason = village.Cats[3];
            sawyer.ControlledBy = mason.ControlledBy = "";
            foreach (var work in new[] { (station: cutter, worker: sawyer, recipe: "logs_to_planks"), (station: prep, worker: mason, recipe: "stone_to_blocks") })
            {
                Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = work.station.Id, CatId = work.worker.Id });
                for (int batch = 0; batch < 2; batch++)
                {
                    var order = Edit(work.station, work.worker, "add");
                    order.RecipeId = work.recipe;
                    Act(world, village, order);
                }
            }
            var pause = Edit(bench, crafter, "pause");
            pause.Enabled = true;
            Act(world, village, pause);
            Act(world, village, Edit(bench, crafter, "delegate"));
            var invalid = Edit(bench, crafter, "add");
            invalid.RecipeId = "bone_tool";
            Check(!Apply(world, village, invalid).Success && bench.Slots.Single().AutomatedBy == "forester", "Rejected locked recipe stole the waiting downstream position from its officer");
            Wait(world, () => village.Jobs.Count(j => !j.Completed && j.Kind == "production" && j.Phase == "working" && j.Progress > 0) == 2, 60, "Upstream workshops never worked concurrently");
            var pausedJob = village.Jobs.Single(j => !j.Completed && j.CatId == mason.Id);
            var runningJob = village.Jobs.Single(j => !j.Completed && j.CatId == sawyer.Id);
            pause = Edit(prep, mason, "pause");
            pause.Enabled = true;
            Act(world, village, pause);
            double pausedProgress = pausedJob.Progress, runningProgress = runningJob.Progress;
            world.Step(3);
            Check(pausedJob.Progress == pausedProgress && runningJob.Progress > runningProgress && bench.Slots.Single().Queue.Count == 0 && crafter.JobId == "", "Pausing one upstream workshop failed to retain its work or allowed paused downstream production");
            pause.Enabled = false;
            Act(world, village, pause);
            Wait(world, () => cutter.Slots.Single().Queue.Count == 0 && prep.Slots.Single().Queue.Count == 0 && sawyer.JobId == "" && mason.JobId == "", 1600, "Finite upstream queues did not exhaust and physically deliver their intermediates");
            Check(world.Total(village, "planks") == 2 && world.Total(village, "blocks") == 2 && world.Total(village, "logs") == 0 && world.Total(village, "stone") == 0 && world.Reservations.Count == 0, "Concurrent finite upstream bills failed to conserve raw goods and delivered intermediates");
            pause = Edit(bench, crafter, "pause");
            pause.Enabled = false;
            Act(world, village, pause);
            Act(world, village, Edit(bench, crafter, "delegate"));
            Wait(world, () => village.Items.Any(i => i.Kind == "tool" && i.Material == "wood" && village.Stockpiles.Any(p => p.Id == i.LocationId)) && crafter.JobId == "", 900, "Returned downstream officer did not craft and physically deliver the finite tool");
            world.Step(31);
            Check(village.Items.Count == 1 && village.Items.Single().Kind == "tool" && village.Items.Single().Material == "wood" && village.Items.Single().Quality == 1 && village.Items.Single().Condition == 100, "Finite composed chain created duplicate equipment or changed its metadata");
            Check(world.Total(village, "planks") == 0 && world.Total(village, "blocks") == 0 && world.Reservations.Count == 0 && !village.Jobs.Any(j => !j.Completed) && village.Cats.All(c => c.Cargo.Count == 0) && new[] { cutter, prep, bench }.All(b => b.Inputs.Count == 0 && b.Outputs.Count == 0 && b.Slots.Single().Queue.Count == 0), "Completed finite chain retained duplicate intermediates, claims, cargo or unfinished orders");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }
    }
}
