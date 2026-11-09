using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class WorkshopPauseRevisionScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.workshop_pause_before_staffing_resumes_production", ResumeAfterStaffing);
            yield return new Scenario("regression.workshop_pause_legacy_crew_resume_preserves_other_position", () => ResumeCrew(true));
            yield return new Scenario("regression.workshop_pause_slot_resume_preserves_running_position", () => ResumeCrew(false));
            yield return new Scenario("regression.workshop_pause_resume_clears_obsolete_job_blocker", ResumeActiveProduction);
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static void Act(World world, Village village, GameAction action)
        {
            var result = world.Apply(new PlayerContext { PlayerId = "workshop-pause-acceptance", VillageId = village.Id }, action);
            Check(result.Success, action.Kind + ": " + result.Error);
        }

        static World Fixture(out Village village, out Building station)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            var store = village.Stockpiles[0];
            store.Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000), new Stack("logs", 5) };
            store.Capacity = 100000;
            store.Position = new Int2(3, 2);
            village.LastLeaderResearch = 0;
            foreach (var cat in village.Cats)
            {
                cat.ControlledBy = "fixture-held";
                cat.ControlLeaseUntil = 10000000;
                cat.Position = new Int2(1, 3);
                cat.X = 1;
                cat.Z = 3;
            }
            station = new Building { Id = world.Id("workshop-pause-station"), Kind = "wood_cutter", Position = new Int2(1, 3), Completed = true };
            village.Buildings.Add(station);
            Check(world.Path(village.Center, station.Position, village) != null, "Workshop fixture must be physically reachable");
            return world;
        }

        static void Staff(World world, Village village, Building station, Cat cat)
        {
            cat.ControlledBy = "";
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id });
        }

        static void Queue(World world, Village village, Building station, Cat cat, bool repeat)
        {
            Act(world, village, new GameAction { Kind = cat == null ? "EditProductionQueue" : "EditProductionWorkSlot", BuildingId = station.Id, CatId = cat?.Id ?? "", Edit = "add", RecipeId = "logs_to_planks", Repeat = repeat });
        }

        static void CompleteFiniteRecipe(World world, Village village, Building station, Cat worker)
        {
            for (int second = 0; second < 800 && !village.Jobs.Any(j => j.Kind == "production" && j.TargetId == station.Id && j.CatId == worker.Id && j.Completed); second++) world.Step(1);
            Check(village.Jobs.Count(j => j.Kind == "production" && j.TargetId == station.Id && j.CatId == worker.Id && j.Completed) == 1, "Resume did not complete the worker's finite physical recipe");
            Check(world.Total(village, "planks") == world.RecipeOutput(village, Catalog.Recipe("logs_to_planks"), 1), "Resumed recipe did not deliver exactly one finite batch of planks");
            Check(world.Total(village, "logs") == 0 && world.Reservations.All(r => r.Resource != "logs"), "Resumed production duplicated or retained its five-log claim");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void ResumeActiveProduction()
        {
            var world = Fixture(out var village, out var station);
            var worker = village.Cats[0];
            Staff(world, village, station, worker);
            Queue(world, village, station, worker, false);
            Job job = null;
            for (int second = 0; second < 100; second++)
            {
                world.Step(1);
                job = village.Jobs.Find(j => j.Id == worker.JobId);
                if (job != null && job.Phase == "working" && job.Progress > 0) break;
            }
            Check(job != null && job.Phase == "working" && job.Progress > 0, "Finite production did not reach its physical working phase");
            double progress = job.Progress;
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "set_paused", Enabled = true });
            world.Step(2);
            Check(job.Progress == progress && job.BlockedReason == "queue_paused", "Paused production must hold progress and explain why");
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "set_paused", Enabled = false });
            world.Step(2);
            Check(job.Progress > progress, "Resuming the active job did not advance physical work");
            Check(job.BlockedReason != "queue_paused", "Resumed work still reports the obsolete paused blocker");
            Check(worker.JobId == job.Id, "Resume replaced the finite job rather than continuing its progress");
            CompleteFiniteRecipe(world, village, station, worker);
            Check(station.Slots.Single().Queue.Count == 0, "Resumed one-shot work remained queued after delivery");
        }

        static void ResumeAfterStaffing()
        {
            var world = Fixture(out var village, out var station);
            Act(world, village, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "pause", Enabled = true });
            Queue(world, village, station, null, false);
            var worker = village.Cats[0];
            Staff(world, village, station, worker);
            var slot = station.Slots.Single();
            var order = slot.Queue.Single();
            world.Step(2);
            Check(!village.Jobs.Any(j => j.Kind == "production" && j.TargetId == station.Id) && world.Total(village, "logs") == 5 && world.Reservations.Count == 0, "Staffing started production while the empty workshop's pause was active");
            Act(world, village, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "pause", Enabled = false });
            Check(!station.Paused && !slot.Paused, "Normal Resume left the staffed workshop frozen by its earlier building pause");
            Check(ReferenceEquals(slot.Queue.Single(), order) && !order.Repeat && slot.CatId == worker.Id, "Resume replaced the player's finite order or worker");
            CompleteFiniteRecipe(world, village, station, worker);
            Check(slot.Queue.Count == 0, "The resumed one-shot recipe did not leave its queue");
        }

        static void ResumeCrew(bool legacyBuildingPause)
        {
            var world = Fixture(out var village, out var station);
            village.Research.Add("wood_cutter_crews");
            var first = village.Cats[0];
            var selected = village.Cats[1];
            Staff(world, village, station, first);
            Staff(world, village, station, selected);
            Queue(world, village, station, first, true);
            Queue(world, village, station, selected, false);
            var untouched = station.Slots.Single(s => s.CatId == first.Id);
            var resumed = station.Slots.Single(s => s.CatId == selected.Id);
            var order = untouched.Queue.Single();
            // Old saves can combine an effective whole-building pause with unpaused work-slot flags.
            station.Paused = legacyBuildingPause;
            resumed.Paused = !legacyBuildingPause;
            untouched.AutomatedBy = "forester";
            order.AutomatedBy = "forester";
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = selected.Id, Edit = "set_paused", Enabled = false });
            Check(!station.Paused && !resumed.Paused, "Exact-slot Resume retained the legacy building pause");
            Check(untouched.Paused == legacyBuildingPause, "Resuming one position changed the other position's effective pause");
            Check(untouched.CatId == first.Id && untouched.AutomatedBy == "forester" && ReferenceEquals(untouched.Queue.Single(), order) && order.Repeat && order.AutomatedBy == "forester", "Resuming one position took over the other worker or its repeating officer order");
            if (!legacyBuildingPause) return;
            CompleteFiniteRecipe(world, village, station, selected);
            Check(!village.Jobs.Any(j => j.Kind == "production" && j.CatId == first.Id), "Resuming one legacy position also started its paused neighbor");
            Check(untouched.Paused && untouched.Queue.Single() == order && first.BuildingId == station.Id, "Completion changed the other position's paused manual staffing");
        }
    }
}
