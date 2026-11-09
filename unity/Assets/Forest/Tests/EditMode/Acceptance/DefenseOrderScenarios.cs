using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class DefenseOrderScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.hourly_defense_manual_order_is_physical_and_idempotent", ManualOrder);
            yield return new Scenario("regression.hourly_defense_no_captain_keeps_trained_cats_manual", NoCaptain);
            yield return new Scenario("regression.hourly_defense_blocked_order_spends_no_remote_combat", BlockedOrder);
            yield return new Scenario("regression.hourly_defense_captain_preserves_busy_worker_ownership", BusyOwnership);
            yield return new Scenario("regression.hourly_defense_explicit_invalid_selection_cannot_fall_back", ExplicitSelection);
            yield return new Scenario("regression.hourly_defense_manual_cancel_checks_exact_owned_job", CancelOrder);
            yield return new Scenario("regression.hourly_defense_escaped_target_clears_suspended_need_order", EscapedDuringNeeds);
            yield return new Scenario("regression.hourly_defense_manual_combat_uses_equipment_and_finite_loot", ManualCombat);
            yield return new Scenario("regression.hourly_defense_captain_orders_stop_when_office_is_removed", CaptainRemoval);
            yield return new Scenario("regression.hourly_defense_manual_reinforcements_all_travel", ReinforcementsTravel);
            yield return new Scenario("regression.hourly_defense_manual_reinforcements_have_independent_combat_budgets", ReinforcementsFight);
            yield return new Scenario("regression.hourly_defense_manual_reinforcements_stop_after_repulsion", ReinforcementsStop);
        }

        static World Fixture(out Village village, out Cat cat)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.LastLeaderResearch = 0;
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            foreach (var other in village.Cats)
            {
                other.ControlledBy = "fixture-held";
                other.ControlLeaseUntil = 10000000;
                other.Skills.Clear();
            }
            cat = village.Cats[0];
            cat.ControlledBy = "";
            cat.Position = new Int2(-5, 1);
            cat.X = -5;
            cat.Z = 1;
            return world;
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static ActionResult Order(World world, Village village, string catId, string raidId) => world.Apply(new PlayerContext { PlayerId = "defense-player", VillageId = village.Id }, new GameAction { Kind = "DefendRaid", CatId = catId, TargetId = raidId });
        static Raid AddRaid(World world, Village village, Int2 position)
        {
            var raid = new Raid { Id = world.Id("order-raid"), Position = position, Health = 100 };
            village.Raids.Add(raid);
            return raid;
        }

        static void ManualOrder()
        {
            var world = Fixture(out var village, out var cat);
            var raid = AddRaid(world, village, new Int2(5, 1));
            var result = Order(world, village, cat.Id, raid.Id);
            Check(result.Success, "Manual defense must accept a free untrained adult without a Captain");
            Check(raid.Health == 100, "DefendRaid dealt damage before its selected cat travelled or spent combat time");
            var job = village.Jobs.Single(j => !j.Completed && j.CatId == cat.Id);
            Check(job.Kind == "defend_raid" && job.TargetId == raid.Id && job.Manual && job.AutomatedBy == "" && cat.JobId == job.Id && result.EntityId == job.Id, "Defense did not persist the selected real cat and target as manual authoritative work");
            var repeat = Order(world, village, cat.Id, raid.Id);
            Check(repeat.Success && repeat.EntityId == job.Id && village.Jobs.Count(j => !j.Completed && j.Kind == "defend_raid") == 1 && raid.Health == 100, "Repeated defense stacked damage or duplicate work");
            world.Step(World.SimulationStepSeconds);
            Check(cat.X > -5 && cat.X < 5 && raid.Health == 100 && World.Amount(cat.Skills, "fight") == 0, "Manual defense failed to travel physically before combat");
        }

        static void NoCaptain()
        {
            var world = Fixture(out var village, out var cat);
            cat.Skills.Add(new Stack("fight", 10));
            var raid = AddRaid(world, village, cat.Position);
            world.Step(World.SimulationStepSeconds);
            Check(raid.Health == 100 && World.Amount(cat.Skills, "fight") == 10 && !village.Jobs.Any(j => j.Kind == "defend_raid"), "Unfilled Captain office silently automated defense");
        }

        static void BlockedOrder()
        {
            var world = Fixture(out var village, out var cat);
            foreach (var offset in new[] { new Int2(1, 0), new Int2(-1, 0), new Int2(0, 1), new Int2(0, -1) })
                village.BoundaryEdges.Add(new BoundaryEdge { From = cat.Position, To = new Int2(cat.Position.X + offset.X, cat.Position.Z + offset.Z) });
            var raid = AddRaid(world, village, new Int2(5, 1));
            Check(world.Path(cat.Position, raid.Position, village) == null, "Defense regression needs an actually obstructed route");
            Check(Order(world, village, cat.Id, raid.Id).Success, "Manual order should remain pending while its route is blocked");
            world.Step(World.SimulationStepSeconds);
            Check(cat.X == -5 && cat.Z == 1 && raid.Health == 100 && cat.JobId != "" && World.Amount(cat.Skills, "fight") == 0, "Blocked defender teleported, attacked remotely or lost its order");
            Check(cat.BlockedReason != "" && village.Jobs.Any(j => j.Id == cat.JobId && !j.Completed), "Blocked defense did not expose its physical obstruction while preserving ownership");
        }

        static void BusyOwnership()
        {
            var world = Fixture(out var village, out var cat);
            village.Officers.Add(new Officer { Role = "captain", CatId = village.Cats[2].Id });
            cat.Skills.Add(new Stack("fight", 10));
            var work = new Job { Id = world.Id("manual-hunt"), Kind = "hunt", Manual = true, CatId = cat.Id, Position = cat.Position, Phase = "working", RequiredWork = 1000 };
            village.Jobs.Add(work);
            cat.JobId = work.Id;
            var raid = AddRaid(world, village, cat.Position);
            world.Step(World.SimulationStepSeconds);
            Check(!work.Completed && cat.JobId == work.Id && work.Progress > 0 && raid.Health == 100, "Captain silently cancelled manual work or copied its already-spent actor budget");
        }

        static void ExplicitSelection()
        {
            var world = Fixture(out var village, out var cat);
            var raid = AddRaid(world, village, cat.Position);
            Check(!Order(world, village, "foreign-or-missing-cat", raid.Id).Success, "Explicit missing cat fell back to another worker");
            Check(!Order(world, village, cat.Id, "foreign-or-missing-raid").Success, "Explicit missing raid fell back to the first enemy");
            cat.AgeHours = 4;
            Check(!Order(world, village, cat.Id, raid.Id).Success && raid.Health == 100 && village.Jobs.Count == 0, "Kitten or invalid selection altered raid health or claimed work");
        }

        static void CancelOrder()
        {
            var world = Fixture(out var village, out var cat);
            var raid = AddRaid(world, village, new Int2(5, 1));
            var job = new Job { Id = world.Id("manual-defense"), Kind = "defend_raid", CatId = cat.Id, TargetId = raid.Id, Position = raid.Position, Manual = true };
            village.Jobs.Add(job);
            cat.JobId = job.Id;
            var context = new PlayerContext { PlayerId = "defense-player", VillageId = village.Id };
            Check(!world.Apply(context, new GameAction { Kind = "CancelDefense", TargetId = "foreign-or-stale-job", CatId = cat.Id }).Success && !job.Completed && cat.JobId == job.Id, "Stale or foreign CancelDefense cancelled unrelated work");
            Check(!world.Apply(context, new GameAction { Kind = "CancelDefense", TargetId = job.Id, CatId = village.Cats[1].Id }).Success && !job.Completed, "CancelDefense ignored its expected cat owner");
            Check(world.Apply(context, new GameAction { Kind = "CancelDefense", TargetId = job.Id, CatId = cat.Id }).Success && job.Completed && cat.JobId == "" && cat.ResumeJobId == "" && raid.Health == 100, "Manual defense cannot be cancelled without remote damage or stale ownership");
            var replacement = new Job { Id = world.Id("replacement-hunt"), Kind = "hunt", CatId = cat.Id, Position = cat.Position, RequiredWork = 1000 };
            village.Jobs.Add(replacement);
            cat.JobId = replacement.Id;
            Check(!world.Apply(context, new GameAction { Kind = "CancelDefense", TargetId = job.Id, CatId = cat.Id }).Success && cat.JobId == replacement.Id && !replacement.Completed, "Repeated CancelDefense cancelled replacement work");
        }

        static void EscapedDuringNeeds()
        {
            var world = Fixture(out var village, out var cat);
            var raid = AddRaid(world, village, new Int2(5, 1));
            var result = Order(world, village, cat.Id, raid.Id);
            Check(result.Success, "Need interruption fixture could not order defense");
            var job = village.Jobs.Single(j => j.Id == result.EntityId);
            cat.Thirst = 34;
            world.Step(World.SimulationStepSeconds);
            Check(cat.Goal == "need_drink" && cat.ResumeJobId == job.Id && !job.Completed && raid.Health == 100, "Defense ignored real needs or lost the suspended target");
            raid.Phase = "departing";
            raid.Position = new Int2(village.Center.X, village.Center.Z + village.Radius + 3);
            raid.X = raid.Position.X;
            raid.Z = raid.Position.Z;
            raid.Path.Clear();
            world.Step(World.SimulationStepSeconds);
            Check(!village.Raids.Contains(raid) && job.Completed && cat.JobId == "" && cat.ResumeJobId == "" && cat.Goal == "need_drink", "Escaped target left a ghost defense resume or interrupted the cat's physical need");
            Check(world.Validate().Count == 0, "Need/escape cleanup invalidated authoritative work ownership");
        }

        static void ManualCombat()
        {
            var world = Fixture(out var village, out var cat);
            var armor = new Item { Id = world.Id("manual-armor"), VillageId = village.Id, Kind = "armor", Material = "leather", LocationId = cat.Id, Condition = 42, MaxCondition = 80, Quality = 2 };
            village.Items.Add(armor);
            cat.Equipment.Add(armor.Id);
            var raid = AddRaid(world, village, cat.Position);
            raid.Health = 0.001;
            raid.Loot.Add(new Stack("logs", 7));
            double logs = village.Stockpiles.Sum(p => World.Amount(p.Goods, "logs"));
            var result = Order(world, village, cat.Id, raid.Id);
            Check(result.Success && raid.Health == 0.001, "Manual combat fixture dealt action-time damage");
            world.Step(World.SimulationStepSeconds);
            var job = village.Jobs.Single(j => j.Id == result.EntityId);
            Check(raid.Health <= 0 && job.Completed && cat.JobId == "", "Untrained manual adult did not complete real combat or release its order");
            Check(Math.Abs(World.Amount(cat.Skills, "fight") - World.SimulationStepSeconds / 3600) < 0.0000001 && Math.Abs(armor.Condition - (42 - World.SimulationStepSeconds * 0.005)) < 0.0000001, "Manual combat duplicated actor time, awarded a completion bonus or missed exact equipment wear");
            world.Step(World.SimulationStepSeconds * 2);
            Check(!village.Raids.Contains(raid) && village.Events.Count(e => e.Kind == "defense" && e.EntityId == raid.Id) == 1, "Manual repulsion did not resolve exactly once");
            Check(Math.Abs(village.Stockpiles.Sum(p => World.Amount(p.Goods, "logs")) - logs - 7) < 0.0000001 && village.Stockpiles.Any(p => p.Position.Equals(cat.Position) && World.Amount(p.Goods, "logs") == 7), "Manual defense lost or duplicated finite physical raid loot");
            Check(armor.LocationId == cat.Id && cat.Equipment.Contains(armor.Id) && world.Validate().Count == 0, "Manual defense changed item ownership or world validity");
        }

        static void CaptainRemoval()
        {
            var world = Fixture(out var village, out var cat);
            village.Officers.Add(new Officer { Role = "captain", CatId = village.Cats[2].Id });
            cat.Skills.Add(new Stack("fight", 1));
            var raid = AddRaid(world, village, cat.Position);
            world.Step(World.SimulationStepSeconds);
            var job = village.Jobs.Single(j => !j.Completed && j.Kind == "defend_raid" && j.CatId == cat.Id);
            Check(job.AutomatedBy == "captain" && !job.Manual && job.TargetId == raid.Id && cat.JobId == job.Id && raid.Health < 100, "Captain did not persist its physical defense assignment");
            var context = new PlayerContext { PlayerId = "defense-player", VillageId = village.Id };
            Check(!world.Apply(context, new GameAction { Kind = "CancelDefense", TargetId = job.Id, CatId = cat.Id }).Success && !job.Completed, "Per-job cancellation silently overrode the Captain policy");
            Check(world.Apply(context, new GameAction { Kind = "UnassignOfficer", Role = "captain" }).Success && job.Completed && cat.JobId == "", "Removing Captain did not stop its persisted defense assignment");
            double health = raid.Health;
            world.Step(World.SimulationStepSeconds);
            Check(raid.Health == health && !village.Jobs.Any(j => !j.Completed && j.Kind == "defend_raid") && world.Validate().Count == 0, "Captain removal silently redrafted the freed cat");
        }

        static Cat Reinforcement(Village village, Cat first)
        {
            var second = village.Cats[1];
            second.ControlledBy = "";
            second.Position = first.Position;
            second.X = first.X;
            second.Z = first.Z;
            return second;
        }

        static void ReinforcementsTravel()
        {
            var world = Fixture(out var village, out var first);
            first.Position = new Int2(4, 1);
            first.X = 4;
            var second = Reinforcement(village, first);
            var raid = AddRaid(world, village, new Int2(5, 1));
            Check(Order(world, village, first.Id, raid.Id).Success && Order(world, village, second.Id, raid.Id).Success, "Both explicit manual defense orders must be accepted");
            world.Step(World.SimulationStepSeconds);
            Check(first.X > 4 && second.X > 4 && Math.Abs(first.X - second.X) < 0.0000001, "A second accepted manual defender did not receive its own physical travel quantum");
            Check(raid.Health == 100 && World.Amount(first.Skills, "fight") == 0 && World.Amount(second.Skills, "fight") == 0, "Travelling reinforcements attacked remotely");
            Check(Math.Abs(raid.X - 5) + Math.Abs(raid.Z - 1) > 0 && village.Jobs.Count(j => !j.Completed && j.Kind == "defend_raid" && j.TargetId == raid.Id) == 2 && world.Validate().Count == 0, "Physical pursuit stopped the raid, duplicated orders or invalidated ownership");
        }

        static Item ReinforcementArmor(World world, Village village, Cat cat)
        {
            var armor = new Item { Id = world.Id("reinforcement-armor"), VillageId = village.Id, Kind = "armor", Material = "leather", LocationId = cat.Id, Condition = 42, MaxCondition = 80, Quality = 2 };
            village.Items.Add(armor);
            cat.Equipment.Add(armor.Id);
            return armor;
        }

        static void ReinforcementsFight()
        {
            var world = Fixture(out var village, out var first);
            var second = Reinforcement(village, first);
            var firstArmor = ReinforcementArmor(world, village, first);
            var secondArmor = ReinforcementArmor(world, village, second);
            var raid = AddRaid(world, village, first.Position);
            var firstOrder = Order(world, village, first.Id, raid.Id);
            var secondOrder = Order(world, village, second.Id, raid.Id);
            Check(firstOrder.Success && secondOrder.Success && firstOrder.EntityId != secondOrder.EntityId, "Two selected cats did not retain distinct defense orders");
            world.Step(World.SimulationStepSeconds);
            foreach (var cat in new[] { first, second })
            {
                var job = village.Jobs.Single(j => j.Id == cat.JobId);
                Check(job.Phase == "fighting" && Math.Abs(job.Progress - World.SimulationStepSeconds) < 0.0000001 && Math.Abs(World.Amount(cat.Skills, "fight") - World.SimulationStepSeconds / 3600) < 0.0000001, "Each explicit reinforcement must fight with exactly its own actor quantum");
            }
            Check(Math.Abs(firstArmor.Condition - (42 - World.SimulationStepSeconds * 0.005)) < 0.0000001 && Math.Abs(secondArmor.Condition - (42 - World.SimulationStepSeconds * 0.005)) < 0.0000001, "Reinforcement combat missed or duplicated a cat's exact equipment wear");
            Check(raid.Health < 100 && raid.X == first.X && raid.Z == first.Z && firstArmor.LocationId == first.Id && secondArmor.LocationId == second.Id && world.Validate().Count == 0, "Real reinforcement combat moved the engaged raid or changed equipment ownership");
        }

        static void ReinforcementsStop()
        {
            var world = Fixture(out var village, out var first);
            var second = Reinforcement(village, first);
            var raid = AddRaid(world, village, first.Position);
            raid.Health = 0.001;
            Check(Order(world, village, first.Id, raid.Id).Success && Order(world, village, second.Id, raid.Id).Success, "Repulsion fixture could not order both defenders");
            world.Step(World.SimulationStepSeconds);
            Check(raid.Health <= 0 && village.Jobs.Where(j => j.Kind == "defend_raid").All(j => j.Completed) && first.JobId == "" && second.JobId == "", "Repelling the raid did not release every reinforcement order");
            Check(new[] { first, second }.Count(c => World.Amount(c.Skills, "fight") > 0) == 1 && Math.Abs(new[] { first, second }.Sum(c => World.Amount(c.Skills, "fight")) - World.SimulationStepSeconds / 3600) < 0.0000001, "Later reinforcements attacked an already-repelled raid");
        }
    }
}
