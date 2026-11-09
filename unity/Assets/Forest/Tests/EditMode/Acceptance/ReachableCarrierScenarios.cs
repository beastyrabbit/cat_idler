using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class ReachableCarrierScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.reachable_carrier_unnamed_skips_blocked_preference", UnnamedFallback);
            yield return new Scenario("regression.reachable_carrier_steward_skips_blocked_preference", StewardFallback);
            yield return new Scenario("regression.reachable_carrier_explicit_blocked_refusal", ExplicitRefusal);
            yield return new Scenario("regression.reachable_carrier_preserves_labor_ranking", () => ReachableRanking(false));
            yield return new Scenario("regression.reachable_carrier_preserves_identity_tiebreak", () => ReachableRanking(true));
            yield return new Scenario("regression.reachable_carrier_none_reachable_changes_nothing", NoReachableCarrier);
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static ActionResult Apply(World world, Village village, GameAction action) => world.Apply(new PlayerContext { PlayerId = "reachable-carrier-acceptance", VillageId = village.Id }, action);
        static void Act(World world, Village village, GameAction action)
        {
            var result = Apply(world, village, action);
            Check(result.Success, action.Kind + ": " + result.Error);
        }
        static void Place(Cat cat, Int2 position)
        {
            cat.Position = position;
            cat.X = position.X;
            cat.Z = position.Z;
            cat.Path.Clear();
        }

        static World Fixture(out Village village, out Cat blocked, out Cat reachable, out Stockpile source, out Stockpile destination)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            village.Stockpiles[0].Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000) };
            village.Stockpiles[0].Accepts = new List<string> { "food", "water" };
            village.Stockpiles[0].Capacity = 100000;
            village.LastLeaderResearch = 0;
            var knownLogs = world.Tiles.First(t => t.Resource == "logs" && t.Amount > 0).Position;
            if (!village.Known.Contains(knownLogs)) village.Known.Add(knownLogs);
            foreach (var cat in village.Cats)
            {
                cat.ControlledBy = "fixture-held";
                cat.ControlLeaseUntil = 10000000;
                cat.Skills.Clear();
                cat.Preferences.Clear();
                cat.Boosted = false;
                cat.Hunger = cat.Thirst = cat.Rest = 100;
            }
            blocked = village.Cats[1];
            reachable = village.Cats[2];
            blocked.ControlledBy = reachable.ControlledBy = "";
            blocked.Preferences.Add("haul");
            World.Add(blocked.Skills, "haul", 50);
            Place(blocked, new Int2(1, 2));
            Place(reachable, new Int2(-4, 1));
            source = new Stockpile { Id = world.Id("carrier-source"), Kind = "spill", Position = new Int2(-5, 1), Width = 1, Depth = 1, Capacity = 100, Goods = new List<Stack> { new Stack("logs", 8) } };
            destination = new Stockpile { Id = world.Id("carrier-destination"), Position = new Int2(-4, 1), Width = 1, Depth = 1, Capacity = 100, Accepts = new List<string> { "logs" } };
            village.Stockpiles.Add(source);
            village.Stockpiles.Add(destination);
            foreach (var position in new[] { new Int2(-6, 1), new Int2(-5, 0), new Int2(-5, 2), new Int2(-4, 0), new Int2(-4, 2), new Int2(-3, 1) })
                world.TileAt(position).Wall = true;
            Check(world.Path(blocked.Position, source.Position, village) == null && world.Path(reachable.Position, source.Position, village) != null && world.Path(source.Position, destination.Position, village) != null, "Carrier fixture must separate the preferred cat from a reachable finite haul");
            return world;
        }

        static void CheckClaim(World world, Village village, Cat carrier, Stockpile source, ActionResult result)
        {
            Check(result.Success, "An unreachable preferred cat blocked the reachable free carrier: " + result.Error);
            var job = village.Jobs.Single(j => j.Id == result.EntityId);
            Check(job.CatId == carrier.Id && job.SourceId == source.Id && job.Phase == "fetch" && job.Amount == 8 && carrier.JobId == job.Id, "Unnamed haul did not choose the expected reachable carrier");
            Check(World.Amount(source.Goods, "logs") == 8 && carrier.Cargo.Count == 0 && world.Reservations.Where(r => r.OwnerId == job.Id && r.PileId == source.Id && r.Resource == "logs").Sum(r => r.Amount) == 8, "Selecting a carrier moved goods before pickup or failed to claim its exact finite load");
        }

        static void UnnamedFallback()
        {
            var world = Fixture(out var village, out var blocked, out var reachable, out var source, out _);
            var result = Apply(world, village, new GameAction { Kind = "HaulGatherSpot", TargetId = source.Id });
            CheckClaim(world, village, reachable, source, result);
            Check(blocked.JobId == "" && blocked.Cargo.Count == 0 && blocked.Preferences.Contains("haul") && World.Amount(blocked.Skills, "haul") == 50, "Fallback changed the unreachable candidate's work or labor ranking");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void StewardFallback()
        {
            var world = Fixture(out var village, out var blocked, out var reachable, out var source, out var destination);
            village.Research.Add("basic_tools");
            village.Buildings.Add(new Building { Id = world.Id("carrier-office"), Kind = "workshop", Completed = true, Position = new Int2(5, 6) });
            Act(world, village, new GameAction { Kind = "AssignOfficer", Role = "steward", CatId = village.Cats[0].Id });
            world.Step(10);
            var job = village.Jobs.SingleOrDefault(j => !j.Completed && j.Kind == "haul" && j.SourceId == source.Id);
            Check(job != null && job.CatId == reachable.Id && job.AutomatedBy == "steward" && !job.Manual, "Steward left reachable finite cargo waiting behind its unreachable preferred carrier");
            CheckClaim(world, village, reachable, source, ActionResult.Ok(job.Id));
            bool carried = false;
            for (int tick = 0; tick < 1000 && !job.Completed; tick++)
            {
                world.Step(.05);
                carried |= World.Amount(reachable.Cargo, "logs") == 8 && !reachable.Position.Equals(destination.Position);
                Check(village.Stockpiles.Sum(p => World.Amount(p.Goods, "logs")) + village.Cats.Sum(c => World.Amount(c.Cargo, "logs")) + village.Jobs.Where(j => !j.Completed).Sum(j => World.Amount(j.Local, "logs")) == 8, "Steward pickup or delivery changed the eight finite Logs");
            }
            Check(carried && job.Completed && reachable.JobId == "" && reachable.Cargo.Count == 0 && blocked.JobId == "", "Steward did not physically carry and finish the same haul while leaving the blocked candidate free");
            Check(World.Amount(source.Goods, "logs") == 0 && World.Amount(destination.Goods, "logs") == 8 && world.Reservations.Count == 0 && village.Events.Count(e => e.Kind == "completed" && e.EntityId == job.Id) == 1, "Steward did not deliver its exact finite load once and release all source claims");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void RefusalPreservesState(World world, Village village, Cat blocked, Cat reachable, Stockpile source, GameAction action)
        {
            long nextId = world.NextId;
            uint entropy = world.RandomState;
            double time = world.TimeSeconds;
            int jobs = village.Jobs.Count, claims = world.Reservations.Count;
            var result = Apply(world, village, action);
            Check(!result.Success, "Haul accepted a carrier without a physical source route");
            Check(world.NextId == nextId && world.RandomState == entropy && world.TimeSeconds == time && village.Jobs.Count == jobs && world.Reservations.Count == claims && World.Amount(source.Goods, "logs") == 8 && blocked.JobId == "" && reachable.JobId == "" && blocked.Cargo.Count == 0 && reachable.Cargo.Count == 0, "Refused unreachable haul changed finite goods, work, claims, identity, entropy or time");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void ExplicitRefusal()
        {
            var world = Fixture(out var village, out var blocked, out var reachable, out var source, out _);
            RefusalPreservesState(world, village, blocked, reachable, source, new GameAction { Kind = "HaulGatherSpot", CatId = blocked.Id, TargetId = source.Id });
        }

        static void NoReachableCarrier()
        {
            var world = Fixture(out var village, out var blocked, out var reachable, out var source, out _);
            Place(reachable, blocked.Position);
            RefusalPreservesState(world, village, blocked, reachable, source, new GameAction { Kind = "HaulGatherSpot", TargetId = source.Id });
        }

        static void ReachableRanking(bool tie)
        {
            var world = Fixture(out var village, out var blocked, out var reachable, out var source, out var destination);
            blocked.ControlledBy = "fixture-held";
            var other = village.Cats[3];
            other.ControlledBy = "";
            Place(other, destination.Position);
            reachable.Preferences.Add("haul");
            World.Add(reachable.Skills, "haul", 1);
            if (tie)
            {
                other.Preferences.Add("haul");
                World.Add(other.Skills, "haul", 1);
            }
            else
                World.Add(other.Skills, "haul", 30);
            var expected = tie ? new[] { reachable, other }.OrderBy(c => c.Id, StringComparer.Ordinal).First() : reachable;
            CheckClaim(world, village, expected, source, Apply(world, village, new GameAction { Kind = "HaulGatherSpot", TargetId = source.Id }));
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }
    }
}
