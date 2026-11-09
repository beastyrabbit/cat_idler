using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class DeliveryRecoveryScenarios
    {
        const string StorageBlocked = "output_storage_full_or_unreachable";

        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.delivery_recovery_loaded_scalar_warning_clears_in_flight", LoadedScalarRecovery);
            yield return new Scenario("regression.delivery_recovery_scalar_warning_clears_in_flight", () => FiniteRecipeRecovery(false));
            yield return new Scenario("regression.delivery_recovery_exact_warning_clears_in_flight", () => FiniteRecipeRecovery(true));
            yield return new Scenario("regression.delivery_recovery_cached_endpoint_preserves_route_failure", CachedEndpointFailure);
            yield return new Scenario("regression.delivery_recovery_retrace_arrival_clears_pending_output_warning", RetraceArrival);
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static ActionResult Act(World world, Village village, GameAction action)
        {
            var result = world.Apply(new PlayerContext { PlayerId = "delivery-recovery-acceptance", VillageId = village.Id }, action);
            Check(result.Success, action.Kind + ": " + result.Error);
            return result;
        }
        static string Avoid(World world, Village village, Int2 position) => Act(world, village, new GameAction { Kind = "CreateZone", Resource = "avoid", Position = position, End = position }).EntityId;
        static void Wait(World world, Func<bool> ready, double step, int limit, string message)
        {
            for (int count = 0; count < limit && !ready(); count++) world.Step(step);
            Check(ready(), message);
        }
        static World Fixture(bool exact, out Village village, out Cat worker, out Building station, out Stockpile destination)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            var pantry = village.Stockpiles[0];
            pantry.Position = new Int2(3, 2);
            pantry.Width = pantry.Depth = 1;
            pantry.Accepts = new List<string> { "food", "water" };
            pantry.Capacity = 100000;
            pantry.Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000) };
            village.LastLeaderResearch = 0;
            var logs = world.Tiles.First(t => t.Resource == "logs" && t.Amount > 0).Position;
            if (!village.Known.Contains(logs)) village.Known.Add(logs);
            foreach (var cat in village.Cats)
            {
                cat.ControlledBy = "fixture-held";
                cat.ControlLeaseUntil = 10000000;
                cat.Hunger = cat.Thirst = cat.Rest = 100;
                cat.Skills.Clear();
                cat.Preferences.Clear();
            }
            station = new Building { Id = world.Id("delivery-recovery-station"), Kind = exact ? "woodworking" : "wood_cutter", Completed = true, Position = new Int2(3, 5) };
            destination = new Stockpile { Id = world.Id("delivery-recovery-destination"), Position = new Int2(-3, 5), Width = 1, Depth = 1, Capacity = 100, Accepts = new List<string> { exact ? "tools" : "planks" } };
            village.Buildings.Add(station);
            village.Stockpiles.Add(destination);
            foreach (var at in new[] { station.Position, destination.Position })
            {
                World.ClearSurface(world.TileAt(at), 0);
                world.TileAt(at).Wall = false;
                if (!village.Known.Contains(at)) village.Known.Add(at);
            }
            var receiving = destination;
            Check(!village.Buildings.Any(b => b.Position.Level == receiving.Position.Level && b.Position.X < receiving.Position.X + receiving.Width && b.Position.X + b.Width > receiving.Position.X && b.Position.Z < receiving.Position.Z + receiving.Depth && b.Position.Z + b.Depth > receiving.Position.Z), "Recovery destination overlaps a workplace or home");
            worker = village.Cats[1];
            worker.ControlledBy = "";
            worker.Position = station.Position;
            worker.X = station.Position.X;
            worker.Z = station.Position.Z;
            worker.Path.Clear();
            return world;
        }
        static void Held(Village village, Cat worker, Building station, Stockpile destination, Job job, Item item, double amount)
        {
            Check(!job.Completed && job.CatId == worker.Id && worker.JobId == job.Id && job.TargetId == station.Id && job.Position.Equals(station.Position) && destination.Goods.Count == 0, "Blocked recovery changed work ownership, its station or destination goods");
            if (item == null)
                Check(World.Amount(worker.Cargo, "planks") + World.Amount(job.Local, "planks") == amount && village.Stockpiles.Sum(s => World.Amount(s.Goods, "planks")) == 0, "Blocked recovery lost or remotely transferred finite scalar output");
            else
                Check(village.Items.Count(i => i.Id == item.Id) == 1 && item.LocationId == job.Id && job.ItemIds.SequenceEqual(new[] { item.Id }) && item.Kind == "tool" && item.Material == "wood" && item.Quality == 1 && item.Condition == 100 && item.MaxCondition == 100, "Blocked recovery changed exact output identity, ownership or condition");
        }
        static void FiniteRecipeRecovery(bool exact)
        {
            var world = Fixture(exact, out var village, out var worker, out var station, out var destination);
            var recipe = Catalog.Recipe(exact ? "planks_and_blocks_to_tools" : "logs_to_planks");
            foreach (var input in recipe.Inputs) World.Add(station.Inputs, input.Resource, world.RecipeInput(village, recipe, input.Amount));
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = worker.Id });
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "add", RecipeId = recipe.Id });
            world.Step(1.05);
            var job = village.Jobs.Single(j => !j.Completed && j.Kind == "production" && j.CatId == worker.Id);
            Check(job.Phase == "working" && job.Progress > 0 && station.Slots.Single().Queue.Count == 1, "Finite recovery recipe did not start through its real accepted queue");
            string closed = Avoid(world, village, destination.Position);
            Wait(world, () => job.Phase == "output_delivery" && job.BlockedReason == StorageBlocked, 1, 900, "Finite recipe did not finish work and retain its blocked output");
            var item = exact ? village.Items.Single(i => i.LocationId == job.Id) : null;
            string itemId = item?.Id;
            double amount = world.RecipeOutput(village, recipe, 1);
            Held(village, worker, station, destination, job, item, amount);
            Check(station.Slots.Single().Queue.Count == 0 && station.Inputs.Count == 0 && world.Reservations.Count == 0 && worker.BlockedReason == StorageBlocked, "Finished batch replayed its order, retained inputs or lost its storage warning");
            // Retry remains authoritative even after access is reopened. The old
            // warning must survive this waiting period, with the same finite load.
            world.Step(Math.Ceiling(world.TimeSeconds) - world.TimeSeconds + .1);
            double nextAttempt = job.NextStorageAttemptAt;
            long searches = (long)typeof(World).GetField("pathSearchCount", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic).GetValue(world);
            var position = worker.Position;
            double x = worker.X, z = worker.Z;
            Act(world, village, new GameAction { Kind = "RemoveZone", TargetId = closed });
            world.Step(.2);
            Check(world.TimeSeconds < nextAttempt && job.NextStorageAttemptAt == nextAttempt && job.BlockedReason == StorageBlocked && worker.BlockedReason == StorageBlocked && worker.Position.Equals(position) && worker.X == x && worker.Z == z, "Recovery cleared a real storage hold or moved before its retry boundary");
            Check((long)typeof(World).GetField("pathSearchCount", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic).GetValue(world) == searches, "Recovery repeated storage path searches inside the retry interval");
            Held(village, worker, station, destination, job, item, amount);
            Wait(world, () => worker.Path.Count > 0 && worker.BlockedReason == "" && (worker.X != x || worker.Z != z), .05, 50, "Reopened destination did not resume physical loaded travel");
            Held(village, worker, station, destination, job, item, amount);
            Check(job.DeliveryPileId == destination.Id && !worker.Position.Equals(destination.Position), "Recovery feedback fixture did not observe a real in-flight delivery");
            Check(job.BlockedReason == "", "Recovered output delivery still reports obsolete storage blockage while its load physically moves");
            for (int step = 0; step < 1000 && !job.Completed; step++)
            {
                Held(village, worker, station, destination, job, item, amount);
                world.Step(.05);
            }
            Check(job.Completed && worker.Position.Equals(destination.Position) && Math.Abs(worker.X - destination.Position.X) + Math.Abs(worker.Z - destination.Position.Z) < 1e-9, "Recovered delivery completed before physical destination arrival");
            Check(worker.JobId == "" && worker.BlockedReason == "" && job.BlockedReason == "" && worker.Cargo.Count == 0 && job.Local.Count == 0 && job.ItemIds.Count == 0 && world.Reservations.Count == 0 && village.Events.Count(e => e.Kind == "completed" && e.EntityId == job.Id) == 1, "Recovered delivery retained a warning, finite claim or duplicate completion");
            if (exact)
                Check(village.Items.Count == 1 && item.Id == itemId && item.LocationId == destination.Id && item.Material == "wood" && item.Quality == 1 && item.Condition == 100 && item.MaxCondition == 100, "Recovered exact finite batch changed its output");
            else
                Check(world.Total(village, "planks") == amount && World.Amount(destination.Goods, "planks") == amount, "Recovered scalar finite batch changed its output");
            world.Step(3);
            Check(station.Slots.Single().Queue.Count == 0 && !village.Jobs.Any(j => !j.Completed && j.Kind == "production") && village.Events.Count(e => e.Kind == "completed" && e.EntityId == job.Id) == 1 && world.Validate().Count == 0, "Recovered finite batch replayed or left invalid authority");
        }
        static void CachedEndpointFailure()
        {
            foreach (string marker in new[] { StorageBlocked, "blocked_route" })
            {
                var world = Fixture(false, out var village, out var worker, out var station, out var destination);
                var job = new Job { Id = world.Id("delivery-recovery-loaded"), Kind = "production", Phase = "output_delivery", CatId = worker.Id, TargetId = station.Id, RecipeId = "logs_to_planks", Position = station.Position, DeliveryPileId = destination.Id, BlockedReason = marker };
                village.Jobs.Add(job);
                worker.JobId = job.Id;
                worker.BlockedReason = marker;
                worker.Cargo.Add(new Stack("planks", 1));
                worker.Path = world.Path(worker.Position, destination.Position, village);
                Check(worker.Path != null && worker.Path.Count > 1 && worker.Path.Last().Equals(destination.Position), "Cached route fixture has no retained endpoint");
                foreach (var at in new[] { new Int2(3, 4), new Int2(4, 5), new Int2(3, 6), new Int2(2, 5) }) world.TileAt(at).Wall = true;
                Check(world.Walkable(village, destination.Position), "Cached destination endpoint must remain walkable during the intermediate route failure");
                world.Step(.05);
                Check(worker.BlockedReason == "blocked_route" && worker.Path.Count == 0 && worker.Position.Equals(station.Position) && worker.X == station.Position.X && worker.Z == station.Position.Z, "Cached endpoint bypassed a genuine closed intermediate route");
                Check(job.BlockedReason == marker && job.DeliveryPileId == "", "A walkable endpoint erased unrecovered storage feedback or a genuine job route warning");
                Held(village, worker, station, destination, job, null, 1);
                Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
            }
        }
        static void LoadedScalarRecovery()
        {
            var world = Fixture(false, out var village, out var worker, out var station, out var destination);
            var job = new Job { Id = world.Id("delivery-recovery-loaded-scalar"), Kind = "production", Phase = "output_delivery", CatId = worker.Id, TargetId = station.Id, RecipeId = "logs_to_planks", Position = station.Position, DeliveryPileId = destination.Id, BlockedReason = StorageBlocked };
            village.Jobs.Add(job);
            worker.JobId = job.Id;
            worker.BlockedReason = StorageBlocked;
            worker.Cargo.Add(new Stack("planks", 1));
            world.Step(.05);
            Held(village, worker, station, destination, job, null, 1);
            Check(worker.Path.Count > 0 && worker.BlockedReason == "" && (worker.X != station.Position.X || worker.Z != station.Position.Z), "Loaded scalar regression did not observe physical recovered movement");
            Check(job.BlockedReason == "", "Recovered loaded scalar delivery retains obsolete job storage blockage while physically moving");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }
        static void RetraceArrival()
        {
            var world = Fixture(false, out var village, out var worker, out var station, out var destination);
            var job = new Job { Id = world.Id("delivery-recovery-retrace"), Kind = "production", Phase = "output_delivery", CatId = worker.Id, TargetId = station.Id, RecipeId = "logs_to_planks", Position = station.Position, DeliveryPileId = destination.Id, BlockedReason = StorageBlocked, Local = new List<Stack> { new Stack("planks", 1) } };
            village.Jobs.Add(job);
            worker.JobId = job.Id;
            worker.BlockedReason = StorageBlocked;
            worker.Cargo.Add(new Stack("planks", 1));
            worker.Position = destination.Position;
            worker.X = destination.Position.X + .5;
            worker.Z = destination.Position.Z;
            worker.Path = new List<Int2> { new Int2(destination.Position.X + 1, destination.Position.Z), new Int2(destination.Position.X + 2, destination.Position.Z) };
            // A recovered destination at the reached anchor still requires the
            // physical return along an unfinished edge before it accepts cargo.
            world.Step(.05);
            Check(worker.X > destination.Position.X && worker.X < destination.Position.X + .5 && worker.BlockedReason == StorageBlocked && job.BlockedReason == StorageBlocked, "An unfinished retrace cleared the storage hold before destination arrival");
            Held(village, worker, station, destination, job, null, 2);
            Wait(world, () => worker.Cargo.Count == 0, .05, 30, "Loaded recovery never completed its physical retrace to receiving storage");
            Check(!job.Completed && worker.Position.Equals(destination.Position) && worker.X == destination.Position.X && worker.Z == destination.Position.Z && World.Amount(destination.Goods, "planks") == 1 && World.Amount(job.Local, "planks") == 1, "Retrace recovery remotely deposited or discarded the remaining output chunk");
            Check(job.BlockedReason == "" && worker.BlockedReason == "", "Proven retrace arrival retained an obsolete storage warning while more output awaited collection");
            for (int step = 0; step < 2000 && !job.Completed; step++)
            {
                Check(village.Stockpiles.Sum(p => World.Amount(p.Goods, "planks")) + village.Cats.Sum(c => World.Amount(c.Cargo, "planks")) + village.Jobs.Where(j => !j.Completed).Sum(j => World.Amount(j.Local, "planks")) == 2, "Multi-chunk recovery lost or duplicated finite output");
                world.Step(.05);
            }
            Check(job.Completed && worker.Position.Equals(destination.Position) && World.Amount(destination.Goods, "planks") == 2 && world.Total(village, "planks") == 2 && worker.Cargo.Count == 0 && job.Local.Count == 0 && worker.JobId == "" && world.Reservations.Count == 0 && village.Events.Count(e => e.Kind == "completed" && e.EntityId == job.Id) == 1 && world.Validate().Count == 0, "Multi-chunk retrace recovery failed finite physical single completion");
        }
    }
}
