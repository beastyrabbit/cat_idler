using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class DefenseRevisionScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.second_defense_unreachable_warrior_yields_to_reachable_defender", ReachableDefender);
            yield return new Scenario("regression.second_defense_concurrent_raids_use_distinct_warrior_budgets", ConcurrentDefenders);
            yield return new Scenario("regression.second_defense_pursuit_preserves_raid_movement", PursuingDefender);
            yield return new Scenario("regression.second_defense_exhausted_fallback_preserves_other_work", ExhaustedWorkers);
        }

        static World Fixture(out Village village, out Cat strongest, out Cat other)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            village.LastLeaderResearch = 0;
            village.Officers.Add(new Officer { Role = "captain", CatId = village.Cats[2].Id });
            foreach (var cat in village.Cats)
            {
                cat.ControlledBy = "fixture-held";
                cat.ControlLeaseUntil = 10000000;
                cat.Skills.Clear();
            }
            strongest = village.Cats[0];
            other = village.Cats[1];
            foreach (var cat in new[] { strongest, other })
            {
                cat.ControlledBy = "";
                cat.BuildingId = "";
                cat.Position = new Int2(cat == strongest ? -5 : 5, 1);
                cat.X = cat.Position.X;
                cat.Z = cat.Position.Z;
                cat.Skills.Add(new Stack("fight", cat == strongest ? 10 : 1));
            }
            return world;
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static void Near(double actual, double expected, string message) => Check(Math.Abs(actual - expected) < 0.000001, message + " expected=" + expected + " actual=" + actual);

        static Item Armor(World world, Village village, Cat cat)
        {
            var item = new Item { Id = world.Id("defender-armor"), Kind = "armor", VillageId = village.Id, LocationId = cat.Id, Material = "leather", Condition = 42, MaxCondition = 80, Quality = 2 };
            village.Items.Add(item);
            cat.Equipment.Add(item.Id);
            return item;
        }

        static void ReachableDefender()
        {
            var world = Fixture(out var village, out var strongest, out var defender);
            foreach (var offset in new[] { new Int2(1, 0), new Int2(-1, 0), new Int2(0, 1), new Int2(0, -1) })
                village.BoundaryEdges.Add(new BoundaryEdge { From = strongest.Position, To = new Int2(strongest.Position.X + offset.X, strongest.Position.Z + offset.Z) });
            Check(world.Path(strongest.Position, defender.Position, village) == null, "Stronger warrior must be physically stranded");
            var armor = Armor(world, village, defender);
            var raid = new Raid { Id = world.Id("isolated-raid"), Position = defender.Position, Health = 0.001, Loot = new List<Stack> { new Stack("logs", 7) } };
            village.Raids.Add(raid);
            double skill = World.Amount(defender.Skills, "fight");

            world.Step(World.SimulationStepSeconds);
            Check(raid.Health <= 0, "A stranded higher-skill warrior prevented the physically present defender from fighting");
            Near(World.Amount(defender.Skills, "fight") - skill, World.SimulationStepSeconds / 3600, "Reachable defender received the wrong combat time");
            Near(armor.Condition, 42 - World.SimulationStepSeconds * 0.005, "Reachable defender did not wear its own exact equipment");
            Near(strongest.X, strongest.Position.X, "Stranded warrior crossed its fence while another cat defended");
            Near(strongest.Z, strongest.Position.Z, "Stranded warrior crossed its fence while another cat defended");
            world.Step(World.SimulationStepSeconds);
            Check(!village.Raids.Contains(raid) && village.Events.Count(e => e.Kind == "defense" && e.EntityId == raid.Id) == 1, "Repelled raid did not finish exactly once");
            Near(village.Stockpiles.Sum(p => World.Amount(p.Goods, "logs")), 7, "Repelling the raid lost or duplicated its finite stolen cargo");
            Check(village.Stockpiles.Any(p => p.Position.Equals(defender.Position) && World.Amount(p.Goods, "logs") == 7), "Recovered raid cargo did not remain at the physical combat site");
            Check(armor.LocationId == defender.Id && defender.Equipment.Contains(armor.Id) && world.Validate().Count == 0, "Defense changed exact equipment ownership or world validity");
        }

        static void ConcurrentDefenders()
        {
            var world = Fixture(out var village, out var strongest, out var other);
            var firstArmor = Armor(world, village, strongest);
            var secondArmor = Armor(world, village, other);
            var first = new Raid { Id = world.Id("first-raid"), Position = strongest.Position, Health = 100 };
            var second = new Raid { Id = world.Id("second-raid"), Position = other.Position, Health = 100 };
            village.Raids.Add(first);
            village.Raids.Add(second);
            double firstSkill = World.Amount(strongest.Skills, "fight"), secondSkill = World.Amount(other.Skills, "fight");

            world.Step(World.SimulationStepSeconds);
            Check(first.Health < 100 && second.Health < 100, "Concurrent raids repeatedly selected the already-exhausted strongest warrior instead of the other physically present defender");
            Near(World.Amount(strongest.Skills, "fight") - firstSkill, World.SimulationStepSeconds / 3600, "Strongest warrior fought twice in one actor quantum");
            Near(World.Amount(other.Skills, "fight") - secondSkill, World.SimulationStepSeconds / 3600, "Second warrior did not receive its independent actor quantum");
            Near(firstArmor.Condition, 42 - World.SimulationStepSeconds * 0.005, "Concurrent defense overcharged first equipment wear");
            Near(secondArmor.Condition, 42 - World.SimulationStepSeconds * 0.005, "Concurrent defense missed second equipment wear");
            Near(strongest.X, strongest.Position.X, "Strongest warrior moved toward another raid after spending its combat quantum");
            Near(other.X, other.Position.X, "Other warrior left its physically present raid");
            Check(world.Validate().Count == 0, "Concurrent defense invalidated authoritative world state");
        }

        static void PursuingDefender()
        {
            var world = Fixture(out var village, out var strongest, out var other);
            strongest.Position = new Int2(4, 1);
            strongest.X = 4;
            other.BuildingId = "fixture-held";
            var raid = new Raid { Id = world.Id("pursued-raid"), Position = other.Position, Health = 100 };
            village.Raids.Add(raid);

            world.Step(World.SimulationStepSeconds);
            Check(strongest.X > 4 && strongest.X < 5, "Primary defender did not physically pursue the raid");
            double movement = Math.Abs(raid.X - 5) + Math.Abs(raid.Z - 1);
            Check(movement > 0 && movement <= 0.5 * World.SimulationStepSeconds + 0.000001, "Raid stopped or jumped while its defender was still travelling");
            Near(raid.Health, 100, "Pursuit also gave a second warrior this raid's combat quantum");
            Near(World.Amount(strongest.Skills, "fight"), 10, "Travelling defender gained combat experience without a hit");
            Near(World.Amount(other.Skills, "fight"), 1, "Unassigned fallback fought while another warrior pursued this raid");
            Check(other.BuildingId == "fixture-held" && world.Validate().Count == 0, "Pursuit unnecessarily interrupted another worker or invalidated state");
        }

        static void ExhaustedWorkers()
        {
            var world = Fixture(out var village, out var strongest, out var other);
            foreach (var cat in new[] { strongest, other })
            {
                cat.BuildingId = "";
                cat.Stats.Clear();
                cat.Stats.Add(new Stack("hunting", 50));
                var job = new Job { Id = world.Id("working-hunter"), Kind = "hunt", CatId = cat.Id, Position = cat.Position, Phase = "working", RequiredWork = 1000 };
                village.Jobs.Add(job);
                cat.JobId = job.Id;
            }
            var primaryJob = village.Jobs[0];
            var otherJob = village.Jobs[1];
            var raid = new Raid { Id = world.Id("work-interruption-raid"), Position = strongest.Position, Health = 100 };
            village.Raids.Add(raid);

            world.Step(World.SimulationStepSeconds);
            Near(primaryJob.Progress, World.SimulationStepSeconds, "Primary defender lost its already paid work quantum");
            Check(!primaryJob.Completed && strongest.JobId == primaryJob.Id, "Captain interrupted the strongest warrior's existing work");
            Near(otherJob.Progress, World.SimulationStepSeconds, "Exhausted fallback lost its already paid work quantum");
            Check(!otherJob.Completed && other.JobId == otherJob.Id, "Trying exhausted fallback defenders cancelled every trained worker");
            Near(raid.Health, 100, "An exhausted worker fought with a second copy of its actor quantum");
            Check(world.Validate().Count == 0, "Work interruption invalidated authoritative ownership");
        }
    }
}
