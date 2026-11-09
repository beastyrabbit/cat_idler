using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class EconomyRevisionScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.economy_mixed_haul_skips_claimed_scalar", () => MixedHaul(true, false));
            yield return new Scenario("regression.economy_mixed_haul_skips_unaccepted_scalar", () => MixedHaul(false, false));
            yield return new Scenario("regression.economy_mixed_haul_claimed_scalar_preserves_exact_item", () => MixedHaul(true, true));
            yield return new Scenario("regression.economy_mixed_haul_unaccepted_scalar_preserves_exact_item", () => MixedHaul(false, true));
            yield return new Scenario("regression.economy_scalar_haul_rejects_unreachable_source", () => UnreachableSource(false));
            yield return new Scenario("regression.economy_exact_haul_rejects_unreachable_source", () => UnreachableSource(true));
        }

        static World Fixture(out Village village, out Cat carrier, out Stockpile source, out Stockpile destination)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            village.Stockpiles[0].Accepts = new List<string> { "food", "water" };
            village.Stockpiles[0].Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000) };
            village.Stockpiles[0].Capacity = 100000;
            village.LastLeaderResearch = 0;
            foreach (var cat in village.Cats)
            {
                cat.ControlledBy = "fixture-held";
                cat.ControlLeaseUntil = 10000000;
            }
            carrier = village.Cats[0];
            carrier.ControlledBy = "";
            carrier.BuildingId = "";
            carrier.Position = new Int2(1, 2);
            carrier.X = 1;
            carrier.Z = 2;
            source = new Stockpile { Id = world.Id("mixed-source"), Position = new Int2(-5, 1), Width = 1, Depth = 1, Capacity = 100 };
            destination = new Stockpile { Id = world.Id("mixed-destination"), Position = new Int2(5, 1), Width = 1, Depth = 1, Capacity = 100 };
            village.Stockpiles.Add(source);
            village.Stockpiles.Add(destination);
            Check(world.Path(carrier.Position, source.Position, village) != null && world.Path(source.Position, destination.Position, village) != null, "Fixture routes must be physically reachable");
            return world;
        }

        static PlayerContext Context(Village village) => new PlayerContext { PlayerId = "economy-acceptance", VillageId = village.Id };
        static ActionResult Haul(World world, Village village, Cat carrier, Stockpile source) => world.Apply(Context(village), new GameAction { Kind = "HaulGatherSpot", CatId = carrier.Id, TargetId = source.Id });
        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static void Near(double actual, double expected, string message) => Check(Math.Abs(actual - expected) < 0.00001, message + " expected=" + expected + " actual=" + actual);

        static void MixedHaul(bool claimed, bool exact)
        {
            var world = Fixture(out var village, out var carrier, out var source, out var destination);
            source.Goods.Add(new Stack("logs", 8));
            destination.Accepts.Add(exact ? "mugs" : "stone");
            Item item = null;
            if (exact)
            {
                item = new Item { Id = world.Id("mixed-mug"), Kind = "mug", VillageId = village.Id, LocationId = source.Id, Material = "stone", Quality = 2, Condition = 42, MaxCondition = 80 };
                village.Items.Add(item);
            }
            else
                source.Goods.Add(new Stack("stone", 3));

            Job first = null;
            if (claimed)
            {
                destination.Accepts.Add("logs");
                var other = village.Cats[1];
                other.ControlledBy = "";
                other.BuildingId = "";
                other.Position = carrier.Position;
                other.X = carrier.X;
                other.Z = carrier.Z;
                var claimedResult = Haul(world, village, other, source);
                Check(claimedResult.Success, "First carrier could not reserve the initial scalar stack: " + claimedResult.Error);
                first = village.Jobs.Single(j => j.Id == claimedResult.EntityId);
                Near(world.Reservations.Where(r => r.OwnerId == first.Id && r.Resource == "logs").Sum(r => r.Amount), 8, "First carrier did not exclusively claim the scalar load");
            }

            var result = Haul(world, village, carrier, source);
            Check(result.Success, "A " + (claimed ? "fully claimed" : "unaccepted") + " first scalar stack blocked the pile's available " + (exact ? "exact item" : "second scalar resource") + ": " + result.Error);
            var job = village.Jobs.Single(j => j.Id == result.EntityId);
            Check(job.Resource == (exact ? "mugs" : "stone") && job.CatId == carrier.Id && job.SourceId == source.Id, "Fallback selected the wrong resource, source or carrier");
            Near(World.Amount(source.Goods, "logs"), 8, "Fallback removed the first scalar stack before physical pickup");
            if (exact)
            {
                Check(item.LocationId == job.Id && job.ItemIds.SequenceEqual(new[] { item.Id }) && job.Phase == "item_fetch", "Fallback did not exclusively claim the exact item before pickup");
                var competing = village.Cats[2];
                competing.ControlledBy = "";
                competing.BuildingId = "";
                int jobs = village.Jobs.Count, reservations = world.Reservations.Count;
                Check(!Haul(world, village, competing, source).Success && village.Jobs.Count == jobs && world.Reservations.Count == reservations, "A second fallback carrier double-claimed unavailable cargo");
                competing.ControlledBy = "fixture-held";
            }
            else
            {
                Near(World.Amount(source.Goods, "stone"), 3, "Fallback removed scalar cargo before physical pickup");
                Near(world.Reservations.Where(r => r.OwnerId == job.Id && r.Resource == "stone").Sum(r => r.Amount), 3, "Fallback did not reserve only its own finite scalar cargo");
            }

            for (int second = 0; second < 100 && (!job.Completed || first != null && !first.Completed); second++)
                world.Step(1);
            Check(job.Completed && carrier.JobId == "" && carrier.Cargo.Count == 0 && (first == null || first.Completed), "Mixed-pile fallback did not complete both physical deliveries");
            Near(World.Amount(source.Goods, "logs"), claimed ? 0 : 8, "Mixed-pile delivery changed ownership of the first stack");
            Near(World.Amount(destination.Goods, "logs"), claimed ? 8 : 0, "Mixed-pile delivery lost or duplicated the first load");
            if (exact)
                Check(village.Items.Count(i => i.Id == item.Id) == 1 && item.LocationId == destination.Id && item.Condition == 42 && item.MaxCondition == 80 && item.Quality == 2 && item.Material == "stone", "Fallback delivery changed the exact item's identity or condition");
            else
            {
                Near(World.Amount(source.Goods, "stone"), 0, "Fallback scalar delivery retained spent source goods");
                Near(World.Amount(destination.Goods, "stone"), 3, "Fallback scalar delivery lost or duplicated its finite load");
            }
            Check(world.Reservations.Count == 0, "Completed mixed-pile deliveries retained cargo claims");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void UnreachableSource(bool exact)
        {
            var world = Fixture(out var village, out var carrier, out var source, out var destination);
            destination.Accepts.Add(exact ? "mugs" : "logs");
            destination.Position = new Int2(-4, 1);
            Item item = null;
            if (exact)
            {
                item = new Item { Id = world.Id("inaccessible-mug"), Kind = "mug", VillageId = village.Id, LocationId = source.Id, Condition = 37 };
                village.Items.Add(item);
            }
            else
                source.Goods.Add(new Stack("logs", 8));
            var barriers = new[] { new Int2(-6, 1), new Int2(-5, 0), new Int2(-5, 2), new Int2(-4, 0), new Int2(-4, 2), new Int2(-3, 1) };
            foreach (var position in barriers)
                world.TileAt(position).Wall = true;
            Check(world.Path(carrier.Position, source.Position, village) == null && world.Path(source.Position, destination.Position, village) != null, "Source must be unreachable to the carrier while its destination remains reachable from the source");
            int jobs = village.Jobs.Count, reservations = world.Reservations.Count;
            var result = Haul(world, village, carrier, source);
            Check(!result.Success, "Hauling accepted an unreachable " + (exact ? "exact item" : "scalar") + " source");
            Check(village.Jobs.Count == jobs && world.Reservations.Count == reservations && carrier.JobId == "" && carrier.Cargo.Count == 0, "Rejected source claimed a worker, finite cargo or work identity");
            if (exact)
                Check(item.LocationId == source.Id && item.Condition == 37, "Rejected source changed exact item ownership");
            else
                Near(World.Amount(source.Goods, "logs"), 8, "Rejected source changed finite goods");
            foreach (var position in barriers)
                world.TileAt(position).Wall = false;
            result = Haul(world, village, carrier, source);
            Check(result.Success, "Reopening the physical source route did not allow hauling: " + result.Error);
            var job = village.Jobs.Single(j => j.Id == result.EntityId);
            for (int second = 0; second < 100 && !job.Completed; second++)
                world.Step(1);
            Check(job.Completed && carrier.JobId == "" && carrier.Cargo.Count == 0 && world.Reservations.Count == 0, "Reopened source did not finish its physical haul");
            if (exact)
                Check(item.LocationId == destination.Id && item.Condition == 37 && village.Items.Count(i => i.Id == item.Id) == 1, "Reopened route lost or duplicated exact cargo");
            else
            {
                Near(World.Amount(source.Goods, "logs"), 0, "Reopened route retained picked-up cargo at its source");
                Near(World.Amount(destination.Goods, "logs"), 8, "Reopened route did not deliver its finite cargo exactly once");
            }
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }
    }
}
