using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class PausedOfficerLaborScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.paused_officer_station_preserves_last_automatic_worker", () => LastAutomaticWorker(false));
            yield return new Scenario("regression.paused_officer_vacancy_preserves_last_automatic_worker", () => LastAutomaticWorker(true));
            yield return new Scenario("regression.paused_officer_station_preserves_manual_crew", () => RetainPausedCrew(false));
            yield return new Scenario("regression.paused_officer_station_preserves_automatic_crew", () => RetainPausedCrew(true));
            yield return new Scenario("regression.open_slot_officer_uses_later_unpaused_position", () => LaterOpenPosition(false));
            yield return new Scenario("regression.open_slot_officer_preserves_exact_manual_queues", () => LaterOpenPosition(true));
            yield return new Scenario("regression.open_slot_officer_appends_only_with_remaining_capacity", AppendOpenPosition);
            yield return new Scenario("regression.open_slot_officer_all_paused_positions_remain_empty", AllPositionsPaused);
            yield return new Scenario("regression.open_slot_officer_does_not_refill_stale_position", StalePositionAfterCapacityReduction);
            yield return new Scenario("regression.open_slot_officer_retains_stale_staffed_position", RetainStaleStaffedPosition);
            yield return new Scenario("regression.open_slot_officer_respects_station_pause", StationPauseWithOpenPosition);
            yield return new Scenario("regression.open_slot_officer_preserves_manual_first_position_assignment", ManualFirstPositionAssignment);
            yield return new Scenario("acceptance.open_slot_officer_finite_physical_chain", FiniteOpenPositionChain);
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static void Act(World world, Village village, GameAction action)
        {
            var result = world.Apply(new PlayerContext { PlayerId = "paused-officer-acceptance", VillageId = village.Id }, action);
            Check(result.Success, action.Kind + ": " + result.Error);
        }

        static World Fixture(out Village village, out Building paused, out Building active, out Cat specialist)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            var store = village.Stockpiles[0];
            store.Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000), new Stack("fibre", 5) };
            store.Capacity = 100000;
            store.Position = new Int2(3, 2);
            village.LastLeaderResearch = 0;
            var knownLogs = world.Tiles.First(t => t.Resource == "logs" && t.Amount > 0).Position;
            if (!village.Known.Contains(knownLogs)) village.Known.Add(knownLogs);
            for (int i = 0; i < village.Cats.Count; i++)
            {
                var cat = village.Cats[i];
                // Thirty living adults retain four free paws; only one may be automatically staffed.
                cat.ControlledBy = i >= 1 && i <= 5 ? "" : "fixture-held";
                cat.ControlLeaseUntil = 10000000;
                cat.Position = new Int2(1, 3);
                cat.X = 1;
                cat.Z = 3;
                cat.Skills.Clear();
                cat.Preferences.Clear();
            }
            World.Add(village.Cats[1].Skills, "process", 250);
            specialist = village.Cats[5];
            World.Add(specialist.Skills, "textile", 40);
            specialist.Preferences.Add("textile");
            paused = new Building { Id = world.Id("paused-officer-station"), Kind = "clothier", Position = new Int2(-5, 1), Completed = true };
            active = new Building { Id = world.Id("active-officer-station"), Kind = "clothier", Position = new Int2(1, 3), Completed = true };
            village.Buildings.Add(paused);
            village.Buildings.Add(active);
            Check(world.Path(village.Center, paused.Position, village) != null && world.Path(village.Center, active.Position, village) != null, "Both officer stations must be physically reachable");
            village.Research.Add("textiles");
            Act(world, village, new GameAction { Kind = "AssignOfficer", Role = "cloth_leader", CatId = village.Cats[0].Id });
            Act(world, village, new GameAction { Kind = "EditProductionQueue", BuildingId = paused.Id, Edit = "pause", Enabled = true });
            Check(village.Cats.Count(c => c.Alive && c.ControlledBy == "" && c.JobId == "" && c.BuildingId == "") == 5, "Fixture must have the idle reserve plus exactly one automatic worker");
            return world;
        }

        static void ProductiveStation(World world, Village village, Building active, Cat specialist)
        {
            world.Step(11);
            Check(active.Slots.Count(s => s.CatId != "") == 1 && active.Slots[0].CatId == specialist.Id && active.Slots[0].AutomatedBy == "cloth_leader", "Funded unpaused station did not receive the remaining textile specialist");
            var job = village.Jobs.SingleOrDefault(j => !j.Completed && j.CatId == specialist.Id && j.Kind == "production");
            Check(job != null && job.TargetId == active.Id && job.RecipeId == "fibre_to_thread", "Funded station did not start its exact finite physical recipe");
            for (int second = 0; second < 60 && job.Progress == 0; second++) world.Step(1);
            Check(job.Progress > 0 && job.Phase == "working", "Selected specialist did not fetch the finite fibre and work at its station");
            Check(world.Total(village, "fibre") == 0 && job.Local.Sum(s => s.Resource == "fibre" ? s.Amount : 0) == 5 && world.Reservations.All(r => r.Resource != "fibre"), "Physical input pickup duplicated, stranded or retained a claim on the five finite fibre");
            Check(village.Cats.Count(c => c.Alive && c.ControlledBy == "" && c.JobId == "" && c.BuildingId == "") == 4, "Automatic staffing consumed the four-cat idle reserve");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void LastAutomaticWorker(bool vacantSlot)
        {
            var world = Fixture(out var village, out var paused, out var active, out var specialist);
            QueueEntry retainedOrder = null;
            if (vacantSlot)
            {
                var formerWorker = village.Cats[6];
                formerWorker.ControlledBy = "";
                Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = paused.Id, CatId = formerWorker.Id });
                Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = paused.Id, CatId = formerWorker.Id, Edit = "add", RecipeId = "fibre_to_thread", Repeat = true });
                Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = paused.Id, CatId = formerWorker.Id, Edit = "pause", Enabled = true });
                retainedOrder = paused.Slots.Single().Queue.Single();
                Act(world, village, new GameAction { Kind = "AssignWorker", CatId = formerWorker.Id });
                formerWorker.ControlledBy = "fixture-held";
                Check(!paused.Paused && paused.Slots.Single().Paused && paused.Slots.Single().CatId == "", "Public Pause then Release must leave the intended empty paused position");
            }
            world.Step(10);
            Check((paused.Paused || paused.Slots.Any(s => s.Paused)) && paused.Slots.All(s => s.CatId == ""), "Paused station took the last automatic worker before funded unpaused work");
            ProductiveStation(world, village, active, specialist);
            Check(paused.Slots.All(s => s.CatId == "") && (vacantSlot ? !paused.Paused && paused.Slots.Single().Paused && ReferenceEquals(paused.Slots.Single().Queue.Single(), retainedOrder) && retainedOrder.Repeat : paused.Paused && paused.Queue.Count == 0), "Officer changed the empty station's deliberate pause or queue");
        }

        static void RetainPausedCrew(bool automatic)
        {
            var world = Fixture(out var village, out var paused, out var active, out var specialist);
            var retained = village.Cats[6];
            retained.ControlledBy = "";
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = paused.Id, CatId = retained.Id });
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = paused.Id, CatId = retained.Id, Edit = "add", RecipeId = "fibre_to_thread", Repeat = true });
            var slot = paused.Slots.Single();
            var order = slot.Queue.Single();
            if (automatic) slot.AutomatedBy = order.AutomatedBy = "cloth_leader";
            ProductiveStation(world, village, active, specialist);
            Check(paused.Paused && slot.CatId == retained.Id && retained.BuildingId == paused.Id && retained.JobId == "", "Automatic planning released or started the retained paused crew");
            Check(ReferenceEquals(slot.Queue.Single(), order) && order.Repeat && slot.AutomatedBy == (automatic ? "cloth_leader" : "") && order.AutomatedBy == (automatic ? "cloth_leader" : ""), "Automatic planning replaced the retained manual/officer pause ownership or recipe");
        }

        static World OpenPositionFixture(out Village village, out Building station, out Cat specialist, out WorkSlot pausedPosition)
        {
            var world = Fixture(out village, out station, out var inactive, out specialist);
            inactive.Paused = true;
            village.Research.Add("clothier_crews");
            Check((int)Catalog.BuildingEffect(village, station.Kind, "worker_slots", 1) == 2, "Fixture must have two physical work positions");
            var formerWorker = village.Cats[6];
            formerWorker.ControlledBy = "";
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = formerWorker.Id });
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = formerWorker.Id, Edit = "add", RecipeId = "fibre_to_cloth", Repeat = true });
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = formerWorker.Id, Edit = "pause", Enabled = true });
            pausedPosition = station.Slots.Single();
            Act(world, village, new GameAction { Kind = "AssignWorker", CatId = formerWorker.Id });
            formerWorker.ControlledBy = "fixture-held";
            Check(!station.Paused && pausedPosition.Paused && pausedPosition.CatId == "" && station.Queue == pausedPosition.Queue, "Pause and release must preserve the first position and its queue alias");
            return world;
        }

        static void CheckPausedPosition(Building station, WorkSlot pausedPosition, QueueEntry order)
        {
            Check(ReferenceEquals(station.Slots[0], pausedPosition) && pausedPosition.CatId == "" && pausedPosition.JobId == "" && pausedPosition.Paused && pausedPosition.AutomatedBy == "", "Officer filled, replaced, resumed or took ownership of the reserved paused position");
            Check(ReferenceEquals(station.Queue, pausedPosition.Queue) && ReferenceEquals(pausedPosition.Queue.Single(), order) && order.RecipeId == "fibre_to_cloth" && order.Repeat && order.AutomatedBy == "", "Officer changed the paused position's exact manual repeating queue");
        }

        static void CheckSelectedPosition(World world, Village village, Building station, Cat specialist, WorkSlot selected)
        {
            Check(selected.CatId == specialist.Id && selected.AutomatedBy == "cloth_leader" && specialist.BuildingId == station.Id && station.WorkerId == specialist.Id, "Officer did not select the later unpaused position with the preferred Textile worker");
            Check(station.Slots.Count(s => s.CatId != "") == 1 && village.Cats.Count(c => c.Alive && c.ControlledBy == "" && c.JobId == "" && c.BuildingId == "") == 4, "Open-position staffing exceeded capacity or consumed the four-cat idle reserve");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void LaterOpenPosition(bool manualQueue)
        {
            var world = OpenPositionFixture(out var village, out var station, out var specialist, out var pausedPosition);
            var pausedOrder = pausedPosition.Queue.Single();
            var selected = new WorkSlot();
            var manualOrder = new QueueEntry { RecipeId = "fibre_to_thread" };
            if (manualQueue) selected.Queue.Add(manualOrder);
            station.Slots.Add(selected);
            world.Step(11.05);
            CheckSelectedPosition(world, village, station, specialist, selected);
            Check(station.Slots.Count == 2 && ReferenceEquals(station.Slots[1], selected), "Officer appended a duplicate instead of reusing the existing open position");
            CheckPausedPosition(station, pausedPosition, pausedOrder);
            Check(selected.Queue.Count == 1 && selected.Queue[0].RecipeId == "fibre_to_thread", "Selected position did not retain or receive its funded finite recipe");
            Check(manualQueue ? ReferenceEquals(selected.Queue[0], manualOrder) && manualOrder.AutomatedBy == "" && !manualOrder.Repeat : selected.Queue[0].AutomatedBy == "cloth_leader", "Officer relabeled or replaced the selected position's manual queue ownership");
            Check(village.Jobs.Any(j => !j.Completed && j.CatId == specialist.Id && j.TargetId == station.Id && j.RecipeId == "fibre_to_thread"), "Later selected position did not start physical production");
        }

        static void AppendOpenPosition()
        {
            var world = OpenPositionFixture(out var village, out var station, out var specialist, out var pausedPosition);
            var order = pausedPosition.Queue.Single();
            world.Step(21.05);
            Check(station.Slots.Count == 2, "Officer did not add the remaining legal position beside a paused vacancy");
            CheckSelectedPosition(world, village, station, specialist, station.Slots[1]);
            CheckPausedPosition(station, pausedPosition, order);
        }

        static void AllPositionsPaused()
        {
            var world = OpenPositionFixture(out var village, out var station, out _, out var pausedPosition);
            var order = pausedPosition.Queue.Single();
            var otherOrder = new QueueEntry { RecipeId = "fibre_to_thread", Repeat = true };
            var other = new WorkSlot { Paused = true, Queue = new List<QueueEntry> { otherOrder } };
            station.Slots.Add(other);
            world.Step(21.05);
            Check(station.Slots.Count == 2 && station.Slots.All(s => s.CatId == "") && other.Paused && other.AutomatedBy == "" && ReferenceEquals(other.Queue.Single(), otherOrder) && otherOrder.AutomatedBy == "" && otherOrder.Repeat, "Officer filled a paused position, altered its queue or appended beyond its two-position limit");
            CheckPausedPosition(station, pausedPosition, order);
            Check(world.Total(village, "fibre") == 5 && world.Reservations.Count == 0 && !village.Jobs.Any(j => !j.Completed && j.Kind == "production"), "All-paused positions consumed or claimed finite material");
        }

        static void StalePositionAfterCapacityReduction()
        {
            var world = OpenPositionFixture(out var village, out var station, out _, out var pausedPosition);
            var order = pausedPosition.Queue.Single();
            var staleOrder = new QueueEntry { RecipeId = "fibre_to_thread" };
            var stale = new WorkSlot { Queue = new List<QueueEntry> { staleOrder } };
            station.Slots.Add(stale);
            village.Research.Remove("clothier_crews");
            Check((int)Catalog.BuildingEffect(village, station.Kind, "worker_slots", 1) == 1, "Fixture must retain two persisted positions after capacity falls to one");
            world.Step(21.05);
            Check(station.Slots.Count == 2 && station.Slots.All(s => s.CatId == "") && ReferenceEquals(station.Slots[1], stale) && ReferenceEquals(stale.Queue.Single(), staleOrder) && stale.AutomatedBy == "" && !stale.Paused, "Officer filled, replaced or deleted an out-of-capacity persisted position");
            CheckPausedPosition(station, pausedPosition, order);
            Check(world.Total(village, "fibre") == 5 && world.Reservations.Count == 0, "Out-of-capacity work consumed or claimed finite material");
        }

        static void RetainStaleStaffedPosition()
        {
            var world = OpenPositionFixture(out var village, out var station, out _, out var pausedPosition);
            var order = pausedPosition.Queue.Single();
            var temporary = village.Cats[6];
            var retained = village.Cats[7];
            temporary.ControlledBy = retained.ControlledBy = "";
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = temporary.Id });
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = retained.Id });
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = retained.Id, Edit = "pause", Enabled = true });
            var retainedPosition = station.Slots[1];
            Act(world, village, new GameAction { Kind = "AssignWorker", CatId = temporary.Id });
            temporary.ControlledBy = "fixture-held";
            village.Research.Remove("clothier_crews");
            world.Step(21.05);
            Check(station.Slots.Count == 2 && retainedPosition.CatId == retained.Id && retainedPosition.Paused && retained.BuildingId == station.Id && retained.JobId == "" && station.Slots.Count(s => s.CatId != "") == 1, "Officer released a persisted over-capacity crew or used it to justify another worker");
            CheckPausedPosition(station, pausedPosition, order);
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void StationPauseWithOpenPosition()
        {
            var world = OpenPositionFixture(out var village, out var station, out _, out var pausedPosition);
            var order = pausedPosition.Queue.Single();
            station.Slots.Add(new WorkSlot());
            station.Paused = true;
            world.Step(21.05);
            Check(station.Paused && station.Slots.Count == 2 && station.Slots.All(s => s.CatId == "") && world.Total(village, "fibre") == 5 && world.Reservations.Count == 0, "Officer used a later unpaused position through the player's station-wide pause");
            CheckPausedPosition(station, pausedPosition, order);
        }

        static void ManualFirstPositionAssignment()
        {
            var world = OpenPositionFixture(out var village, out var station, out var specialist, out var pausedPosition);
            var order = pausedPosition.Queue.Single();
            var other = new WorkSlot();
            station.Slots.Add(other);
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = specialist.Id });
            Check(pausedPosition.CatId == specialist.Id && pausedPosition.Paused && pausedPosition.AutomatedBy == "" && specialist.BuildingId == station.Id && other.CatId == "" && ReferenceEquals(pausedPosition.Queue.Single(), order) && order.Repeat && order.AutomatedBy == "", "Manual assignment stopped choosing the first vacancy or changed its deliberate pause and queue");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void FiniteOpenPositionChain()
        {
            var world = OpenPositionFixture(out var village, out var station, out var specialist, out var pausedPosition);
            var order = pausedPosition.Queue.Single();
            var selected = new WorkSlot();
            station.Slots.Add(selected);
            world.Step(11.05);
            CheckSelectedPosition(world, village, station, specialist, selected);
            var first = village.Jobs.SingleOrDefault(j => !j.Completed && j.CatId == specialist.Id && j.RecipeId == "fibre_to_thread");
            Check(first != null && first.TargetId == station.Id, "Officer never began the finite chain in the later legal position");
            bool sawCarriedInput = false, sawWorkingAtStation = false;
            for (int second = 0; second < 900 && !first.Completed; second++)
            {
                world.Step(1);
                sawCarriedInput |= World.Amount(specialist.Cargo, "fibre") == 5;
                sawWorkingAtStation |= first.Phase == "working" && first.Progress > 0 && specialist.Position.Equals(station.Position) && World.Amount(first.Local, "fibre") == 5;
            }
            Check(first.Completed && sawCarriedInput && sawWorkingAtStation && world.Total(village, "fibre") == 0 && world.Total(village, "thread") == 5, "The later worker did not carry five finite Fibre, work physically and deliver exactly five Thread");
            Job secondJob = null;
            for (int second = 0; second < 900 && (secondJob == null || !secondJob.Completed); second++)
            {
                world.Step(1);
                secondJob ??= village.Jobs.FirstOrDefault(j => j.CatId == specialist.Id && j.RecipeId == "fibre_to_cloth");
            }
            Check(secondJob != null && secondJob.Completed && secondJob.TargetId == station.Id && world.Total(village, "fibre") == 0 && world.Total(village, "thread") == 0 && world.Total(village, "cloth") == 1, "Later-position automation did not turn its exact finite Thread batch into one delivered Cloth");
            Check(selected.CatId == specialist.Id && selected.AutomatedBy == "cloth_leader" && selected.Queue.Count == 0 && specialist.Cargo.Count == 0 && specialist.JobId == "" && world.Reservations.Count == 0 && station.Inputs.Count == 0 && station.Outputs.Count == 0 && !village.Jobs.Any(j => !j.Completed && j.Kind == "production"), "Finite later-position production retained duplicate claims, cargo, inputs, outputs or work");
            CheckPausedPosition(station, pausedPosition, order);
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }
    }
}
