using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class CaptainPreparationScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.captain_preparation_physical_suitable_training", PhysicalTraining);
            yield return new Scenario("regression.captain_preparation_manual_orders_and_office_removal", ManualOwnership);
            yield return new Scenario("regression.captain_preparation_scaled_idle_reserve", IdleReserve);
            yield return new Scenario("regression.captain_preparation_reachable_unpaused_barracks", UsableBarracks);
            yield return new Scenario("regression.captain_preparation_active_training_pause_recovery", TrainingPause);
            yield return new Scenario("regression.captain_preparation_healthy_adults_preserve_other_labor", EligibleAdults);
            yield return new Scenario("regression.captain_preparation_target_manual_trainees_and_combat", ReadinessAndCombat);
            yield return new Scenario("regression.captain_preparation_manual_training_uses_reachable_unpaused_site", ManualTrainingSite);
            yield return new Scenario("regression.captain_preparation_manual_training_pause_recovery", ManualTrainingPause);
            yield return new Scenario("regression.captain_preparation_manual_training_rejects_missing_selected_cat", ManualTrainingSelection);
            yield return new Scenario("regression.captain_preparation_manual_training_pause_action_preserves_legacy_slots", TrainingPauseAction);
            yield return new Scenario("regression.captain_preparation_training_to_physical_defense_seeded_partition_twins", TrainingToDefense);
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static ActionResult Act(World world, Village village, GameAction action)
        {
            var result = world.Apply(new PlayerContext { PlayerId = "captain-acceptance", VillageId = village.Id }, action);
            Check(result.Success, action.Kind + ": " + result.Error);
            return result;
        }

        static World Fixture(out Village village, int free = 7)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            village.Stockpiles[0].Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000) };
            village.Stockpiles[0].Capacity = 100000;
            village.LastLeaderResearch = 0;
            var knownWood = new Int2(village.Center.X + village.Radius + 3, village.Center.Z);
            world.TileAt(knownWood).Resource = "logs";
            world.TileAt(knownWood).Amount = 40;
            if (!village.Known.Contains(knownWood)) village.Known.Add(knownWood);
            foreach (var cat in village.Cats)
            {
                int index = village.Cats.IndexOf(cat);
                cat.ControlledBy = index >= 1 && index <= free ? "" : "fixture-held";
                cat.ControlLeaseUntil = 10000000;
                cat.Position = new Int2(1, 3);
                cat.X = 1;
                cat.Z = 3;
                cat.Skills.Clear();
                cat.Preferences.Clear();
                cat.Stats.Clear();
                cat.Stats.Add(new Stack("building", 50));
            }
            return world;
        }

        static Building Barracks(World world, Village village, Int2 position, bool paused = false)
        {
            var building = new Building { Id = world.Id("captain-barracks"), Kind = "barracks", Completed = true, Position = position, Paused = paused };
            village.Buildings.Add(building);
            return building;
        }

        static void Captain(World world, Village village)
        {
            var study = Catalog.RoleStudies[Array.IndexOf(Catalog.Roles, "captain")];
            if (!village.Research.Contains(study)) village.Research.Add(study);
            Act(world, village, new GameAction { Kind = "AssignOfficer", Role = "captain", CatId = village.Cats[0].Id });
        }

        static Job AutoTraining(Village village) => village.Jobs.SingleOrDefault(j => !j.Completed && j.Kind == "train" && j.AutomatedBy == "captain");

        static void PhysicalTraining()
        {
            var world = Fixture(out var village);
            var barracks = Barracks(world, village, new Int2(1, 3));
            var specialist = village.Cats[6];
            specialist.Skills.Add(new Stack("fight", 2));
            specialist.Preferences.Add("fight");
            Captain(world, village);
            world.Step(10);
            var job = AutoTraining(village);
            Check(job != null && job.CatId == specialist.Id, "Captain did not choose a free cat suited to fighting for real training");
            Check(job.TargetId == barracks.Id && job.Position.Equals(barracks.Position) && !job.Manual && job.RequiredWork == 120, "Captain training did not retain exact physical Barracks and officer ownership");
            Check(World.Amount(specialist.Skills, "fight") == 2, "Captain granted combat skill before physical training");
            for (int second = 0; second < 20 && job.Progress == 0; second++) world.Step(1);
            Check(job.Progress > 0 && job.Progress < job.RequiredWork, "Captain training did not spend physical work time");
            world.Step(140);
            Check(job.Completed && World.Amount(specialist.Skills, "fight") >= 3, "Physical Captain training did not finish and improve its worker");
            Check(village.Jobs.Count(j => !j.Completed && j.Kind == "train" && j.AutomatedBy == "captain") <= 1, "Captain scheduled several simultaneous trainees");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void ManualOwnership()
        {
            var world = Fixture(out var village);
            Barracks(world, village, new Int2(1, 3));
            var manualCat = village.Cats[1];
            Act(world, village, new GameAction { Kind = "TrainWarrior", CatId = manualCat.Id });
            var manual = village.Jobs.Single(j => j.Id == manualCat.JobId);
            world.Step(10);
            Check(AutoTraining(village) == null && manualCat.JobId == manual.Id && manual.Manual, "Training automated without a Captain or replaced manual work");
            Captain(world, village);
            world.Step(10);
            var automatic = AutoTraining(village);
            Check(automatic != null && automatic.CatId != manualCat.Id, "Captain did not count but preserve the existing manual trainee");
            var automaticCat = village.Cats.Single(c => c.Id == automatic.CatId);
            Act(world, village, new GameAction { Kind = "UnassignOfficer", Role = "captain" });
            Check(automatic.Completed && automatic.CatId == "" && automaticCat.JobId == "", "Removing the Captain retained its automatic labor claim");
            Check(!manual.Completed && manualCat.JobId == manual.Id && manual.Manual, "Removing Captain cancelled the player's manual training");
        }

        static void IdleReserve()
        {
            var world = Fixture(out var village, 4);
            Barracks(world, village, new Int2(1, 3));
            Captain(world, village);
            world.Step(10);
            Check(AutoTraining(village) == null, "Captain consumed the scaled four-cat idle reserve for thirty living cats");
            village.Cats[5].ControlledBy = "";
            world.Step(10);
            Check(AutoTraining(village) != null, "Captain failed to use one worker above the scaled idle reserve");
            Check(village.Cats.Count(c => c.Alive && c.ControlledBy == "" && c.JobId == "" && c.BuildingId == "") == 4, "Captain did not preserve the scaled idle reserve");
        }

        static void UsableBarracks()
        {
            var world = Fixture(out var village);
            var stranded = Barracks(world, village, new Int2(-5, 1));
            foreach (var offset in new[] { new Int2(1, 0), new Int2(-1, 0), new Int2(0, 1), new Int2(0, -1) })
                village.BoundaryEdges.Add(new BoundaryEdge { From = stranded.Position, To = new Int2(stranded.Position.X + offset.X, stranded.Position.Z + offset.Z) });
            var reachable = Barracks(world, village, new Int2(1, 3), true);
            Check(world.Path(village.Cats[1].Position, stranded.Position, village) == null, "Stranded Barracks fixture must be physically unreachable");
            Captain(world, village);
            world.Step(10);
            Check(AutoTraining(village) == null, "Captain claimed labor for paused or unreachable Barracks");
            reachable.Paused = false;
            world.Step(10);
            var job = AutoTraining(village);
            Check(job != null && job.TargetId == reachable.Id && job.Position.Equals(reachable.Position), "Captain selected the first Barracks instead of an available reachable training site");
        }

        static void TrainingPause()
        {
            var world = Fixture(out var village);
            var barracks = Barracks(world, village, new Int2(1, 3));
            Captain(world, village);
            var cat = village.Cats[1];
            Act(world, village, new GameAction { Kind = "TrainWarrior", CatId = cat.Id });
            var job = village.Jobs.Single(j => j.Id == cat.JobId);
            // Fixture represents an already persisted Captain order; all work still uses the real executor.
            job.AutomatedBy = "captain";
            job.Manual = false;
            job.TargetId = barracks.Id;
            world.Step(1);
            double progress = job.Progress;
            Check(progress > 0, "Active training fixture did not begin physical work");
            barracks.Paused = true;
            world.Step(1);
            Check(job.Progress == progress && job.BlockedReason == "training_barracks_paused", "Paused Barracks advanced Captain training or hid its blocker");
            barracks.Paused = false;
            world.Step(1);
            Check(job.Progress > progress && job.BlockedReason == "" && cat.BlockedReason == "", "Unpausing Barracks failed to resume the same training or retained a stale blocker");
            Check(!job.Completed && cat.JobId == job.Id, "Pause recovery replaced the actual Captain training order");
        }

        static void EligibleAdults()
        {
            var world = Fixture(out var village, 12);
            Barracks(world, village, new Int2(1, 3));
            for (int index = 1; index <= 7; index++)
            {
                village.Cats[index].Skills.Add(new Stack("fight", 2.9));
                village.Cats[index].Preferences.Add("fight");
                village.Cats[index].NextNeedAttemptAt = 10000;
            }
            village.Cats[1].Health = 10;
            village.Cats[2].Hunger = 10;
            village.Cats[3].Thirst = 10;
            village.Cats[4].Rest = 5;
            village.Cats[5].AgeHours = 1;
            village.Cats[6].Migration = "departing";
            village.Cats[6].MigrationExterior = new Int2(100, 100);
            var workshop = new Building { Id = world.Id("manual-workshop"), Kind = "wood_cutter", Completed = true, Position = new Int2(3, 3) };
            village.Buildings.Add(workshop);
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = workshop.Id, CatId = village.Cats[7].Id });
            var suitable = village.Cats[8];
            suitable.Skills.Add(new Stack("fight", 2));
            suitable.Preferences.Add("fight");
            Captain(world, village);
            world.Step(10);
            var job = AutoTraining(village);
            Check(job != null && job.CatId == suitable.Id, "Captain drafted a cat with critical needs, a kitten, a migrant or another workplace's worker");
            Check(village.Cats[7].BuildingId == workshop.Id && workshop.Slots.Single(s => s.CatId == village.Cats[7].Id).AutomatedBy == "", "Captain replaced manual workshop ownership");
            var readiness = world.CaptainPreparationStatus(village);
            Check(readiness.Target == 2 && readiness.Trained == 0 && readiness.Training == 1 && readiness.Blocker == "training_active", "Readiness did not report its real single trainee and population target");
        }

        static void ReadinessAndCombat()
        {
            var world = Fixture(out var village);
            Barracks(world, village, new Int2(1, 3));
            village.Cats[1].Skills.Add(new Stack("fight", 3));
            village.Cats[2].Skills.Add(new Stack("fight", 3));
            Captain(world, village);
            world.Step(10);
            var readiness = world.CaptainPreparationStatus(village);
            Check(AutoTraining(village) == null && readiness.Target == 2 && readiness.Trained == 2 && readiness.Training == 0 && readiness.Blocker == "ready", "Captain kept drafting workers after reaching its modest training target");

            world = Fixture(out village);
            Barracks(world, village, new Int2(1, 3));
            village.Cats[2].Skills.Add(new Stack("fight", 3));
            Act(world, village, new GameAction { Kind = "TrainWarrior", CatId = village.Cats[1].Id });
            Captain(world, village);
            world.Step(10);
            readiness = world.CaptainPreparationStatus(village);
            Check(AutoTraining(village) == null && readiness.Trained == 1 && readiness.Training == 1 && readiness.Blocker == "training_active", "Captain ignored a player's existing trainee when calculating the target");

            world = Fixture(out village);
            Barracks(world, village, new Int2(1, 3));
            village.Raids.Add(new Raid { Id = world.Id("training-interruption-raid"), Position = new Int2(-5, 1), Health = 1000 });
            Captain(world, village);
            Check(world.CaptainPreparationStatus(village).Blocker == "raid_active", "Readiness hid active combat");
            world.Step(10);
            Check(AutoTraining(village) == null, "Captain drafted a new trainee during an active raid");
        }

        static void ManualTrainingSite()
        {
            var world = Fixture(out var village);
            var stranded = Barracks(world, village, new Int2(-5, 1));
            foreach (var offset in new[] { new Int2(1, 0), new Int2(-1, 0), new Int2(0, 1), new Int2(0, -1) })
                village.BoundaryEdges.Add(new BoundaryEdge { From = stranded.Position, To = new Int2(stranded.Position.X + offset.X, stranded.Position.Z + offset.Z) });
            var reachable = Barracks(world, village, new Int2(1, 3), true);
            var cat = village.Cats[1];
            var action = new GameAction { Kind = "TrainWarrior", CatId = cat.Id };
            var result = world.Apply(new PlayerContext { PlayerId = "captain-acceptance", VillageId = village.Id }, action);
            Check(!result.Success && cat.JobId == "" && !village.Jobs.Any(j => !j.Completed && j.Kind == "train"), "Manual training claimed paused or unreachable Barracks");
            reachable.Paused = false;
            Act(world, village, action);
            var job = village.Jobs.Single(j => j.Id == cat.JobId);
            Check(job.TargetId == reachable.Id && job.Position.Equals(reachable.Position) && job.Manual && job.AutomatedBy == "", "Manual training ignored the reachable unpaused alternate or lost exact site/manual ownership");
        }

        static void ManualTrainingPause()
        {
            var world = Fixture(out var village);
            var barracks = Barracks(world, village, new Int2(1, 3));
            var cat = village.Cats[1];
            Act(world, village, new GameAction { Kind = "TrainWarrior", CatId = cat.Id });
            var job = village.Jobs.Single(j => j.Id == cat.JobId);
            // Older saves recorded only the original Barracks position.
            job.TargetId = "";
            world.Step(1);
            double progress = job.Progress;
            Check(progress > 0, "Manual training fixture did not begin real work");
            barracks.Paused = true;
            world.Step(1);
            Check(job.Progress == progress && job.BlockedReason == "training_barracks_paused", "Pausing Barracks did not hold manual training and expose its reason");
            barracks.Paused = false;
            world.Step(1);
            Check(job.Progress > progress && job.BlockedReason == "" && cat.BlockedReason == "" && cat.JobId == job.Id && job.TargetId == barracks.Id && !job.Completed && job.Manual, "Manual training did not recover the same order and original legacy site after unpausing");
        }

        static void ManualTrainingSelection()
        {
            var world = Fixture(out var village);
            Barracks(world, village, new Int2(1, 3));
            var result = world.Apply(new PlayerContext { PlayerId = "captain-acceptance", VillageId = village.Id }, new GameAction { Kind = "TrainWarrior", CatId = "missing-or-foreign-cat" });
            Check(!result.Success && !village.Jobs.Any(j => !j.Completed && j.Kind == "train") && village.Cats.All(c => c.JobId == ""), "Explicit missing manual trainee silently fell back to another cat");
        }

        static void TrainingPauseAction()
        {
            var world = Fixture(out var village);
            var barracks = Barracks(world, village, new Int2(1, 3));
            var legacyWorker = village.Cats[2];
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = barracks.Id, CatId = legacyWorker.Id });
            var occupied = barracks.Slots.Single(s => s.CatId == legacyWorker.Id);
            occupied.AutomatedBy = "steward";
            var existingQueue = barracks.Queue;
            var entry = new QueueEntry { RecipeId = Catalog.Recipes.First(r => r.Building == "wood_cutter").Id, Repeat = true, AutomatedBy = "forester" };
            existingQueue.Add(entry);
            var vacancy = new WorkSlot { AutomatedBy = "captain", Paused = true };
            barracks.Slots.Insert(0, vacancy);
            var trainee = village.Cats[1];
            Act(world, village, new GameAction { Kind = "TrainWarrior", CatId = trainee.Id });
            var job = village.Jobs.Single(j => j.Id == trainee.JobId);
            world.Step(1);
            double progress = job.Progress;
            Act(world, village, new GameAction { Kind = "EditProductionQueue", BuildingId = barracks.Id, Edit = "pause", Enabled = true });
            Check(barracks.Paused, "Barracks pause action toggled a legacy position instead of the training site");
            world.Step(1);
            Check(progress > 0 && job.Progress == progress && job.BlockedReason == "training_barracks_paused", "Barracks pause action did not hold real manual training");
            Act(world, village, new GameAction { Kind = "EditProductionQueue", BuildingId = barracks.Id, Edit = "pause", Enabled = false });
            world.Step(1);
            Check(!barracks.Paused && job.Progress > progress && job.BlockedReason == "" && trainee.JobId == job.Id && job.Manual && !job.Completed, "Barracks resume action failed to recover the same manual training");
            Check(barracks.Slots.Count == 2 && ReferenceEquals(barracks.Slots[0], vacancy) && vacancy.CatId == "" && vacancy.AutomatedBy == "captain" && vacancy.Paused && vacancy.Queue.Count == 0, "Global training pause changed a legacy vacant position");
            Check(ReferenceEquals(barracks.Slots[1], occupied) && occupied.CatId == legacyWorker.Id && occupied.AutomatedBy == "steward" && !occupied.Paused && legacyWorker.BuildingId == barracks.Id && barracks.WorkerId == legacyWorker.Id, "Global training pause changed legacy worker ownership or automation");
            Check(ReferenceEquals(barracks.Queue, existingQueue) && ReferenceEquals(occupied.Queue, existingQueue) && existingQueue.Count == 1 && ReferenceEquals(existingQueue[0], entry) && entry.Repeat && entry.AutomatedBy == "forester", "Global training pause seized or replaced a legacy queue");
        }

        static void TrainingToDefense()
        {
            var first = TrainingToDefenseTwin(World.SimulationStepSeconds);
            var second = TrainingToDefenseTwin(0.5);
            Check(Math.Abs(first.Skill - second.Skill) < 0.0000001 && Math.Abs(first.Fighting - second.Fighting) < 0.0000001 && first.Loot == second.Loot, "Seeded training-to-defense chain changed when elapsed time used different caller partitions");
        }

        static (double Skill, double Fighting, double Loot) TrainingToDefenseTwin(double partition)
        {
            var world = Fixture(out var village);
            Barracks(world, village, new Int2(1, 3));
            var trainee = village.Cats[6];
            trainee.Preferences.Add("fight");
            trainee.Stats.Add(new Stack("attack", 50));
            var manualWorker = village.Cats[7];
            manualWorker.Skills.Add(new Stack("fight", 20));
            var workshop = new Building { Id = world.Id("defense-chain-workshop"), Kind = "wood_cutter", Completed = true, Position = new Int2(3, 3) };
            village.Buildings.Add(workshop);
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = workshop.Id, CatId = manualWorker.Id });
            var raid = new Raid { Id = world.Id("trained-defense-raid"), Health = 4, Loot = new List<Stack> { new Stack("logs", 7) } };
            Captain(world, village);
            world.Step(10);
            var training = AutoTraining(village);
            Check(training != null && training.CatId == trainee.Id && training.RequiredWork == 120 && World.Amount(trainee.Skills, "fight") == 0, "Composed defense did not start with a real untrained automatic trainee");
            int steps = (int)Math.Round(170 / partition);
            for (int step = 0; step < steps; step++) world.Step(partition);
            Check(training.Completed && training.Progress >= 120 && World.Amount(trainee.Skills, "fight") == 3 && trainee.JobId == "", "Captain training failed to produce one free physically trained defender");
            int exterior = village.Radius + 3;
            raid.Position = new[] { new Int2(-exterior, 0), new Int2(exterior, 0), new Int2(0, -exterior), new Int2(0, exterior) }.OrderByDescending(p => Int2.Distance(trainee.Position, p)).First(p => Int2.Distance(trainee.Position, p) >= 3 && world.Path(trainee.Position, p, village) != null);
            village.Raids.Add(raid);
            double startedAt = world.TimeSeconds, startX = trainee.X, startZ = trainee.Z;
            Job defense = null;
            // An existing idle walk may finish spending its own movement quanta before dispatch.
            for (int step = 0; step < 400 && defense == null; step++)
            {
                startX = trainee.X;
                startZ = trainee.Z;
                world.Step(World.SimulationStepSeconds);
                defense = village.Jobs.SingleOrDefault(j => !j.Completed && j.Kind == "defend_raid" && j.TargetId == raid.Id && j.CatId == trainee.Id);
                Check(raid.Health == 4 && World.Amount(trainee.Skills, "fight") == 3, "Idle movement or defense dispatch dealt remote damage before physical pursuit");
            }
            Check(defense != null && !defense.Manual && defense.AutomatedBy == "captain", "Captain did not dispatch the newly trained free cat while preserving the stronger manual worker");
            double moved = Math.Abs(trainee.X - startX) + Math.Abs(trainee.Z - startZ);
            Check(moved > 0 && moved <= 0.5 && defense.Progress == 0 && raid.Health == 4 && World.Amount(trainee.Skills, "fight") == 3, "Training-to-defense skipped physical pursuit or also attacked with its travel quantum");
            for (int step = 0; step < Math.Ceiling(180 / partition) && village.Raids.Contains(raid); step++) world.Step(partition);
            Check(!village.Raids.Contains(raid) && defense.Completed && trainee.JobId == "" && defense.Progress > 0 && defense.Progress <= world.TimeSeconds - startedAt, "Trained defender did not repel the durable raid with bounded physical combat work");
            Check(Math.Abs(World.Amount(trainee.Skills, "fight") - 3 - defense.Progress / 3600) < 0.0000001, "Physical combat skill copied or lost the defender's actual combat time");
            Check(village.Events.Count(e => e.Kind == "defense" && e.EntityId == raid.Id) == 1 && village.Stockpiles.Sum(p => World.Amount(p.Goods, "logs")) == 7 && village.Stockpiles.Count(p => World.Amount(p.Goods, "logs") == 7) == 1, "Training-to-defense lost or duplicated the raid's seven finite Logs or completion event");
            Check(manualWorker.BuildingId == workshop.Id && manualWorker.JobId == "" && workshop.Slots.Single(s => s.CatId == manualWorker.Id).AutomatedBy == "" && World.Amount(manualWorker.Skills, "fight") == 20, "Captain interrupted or trained the player's stronger manually assigned workshop worker");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
            return (World.Amount(trainee.Skills, "fight"), defense.Progress, village.Stockpiles.Sum(p => World.Amount(p.Goods, "logs")));
        }
    }
}
