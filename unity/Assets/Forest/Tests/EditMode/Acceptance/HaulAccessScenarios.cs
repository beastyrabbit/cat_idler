using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class HaulAccessScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.haul_access_scalar_blocked_anchor", () => BlockedAnchor(false));
            yield return new Scenario("regression.haul_access_exact_blocked_anchor", () => BlockedAnchor(true));
            yield return new Scenario("regression.haul_access_ranked_footprint_carrier", RankedCarrier);
            yield return new Scenario("regression.haul_access_scalar_closed_route_holds_and_recovers", () => ClosedRoute(false));
            yield return new Scenario("regression.haul_access_exact_closed_route_holds_and_recovers", () => ClosedRoute(true));
            yield return new Scenario("regression.haul_access_fully_blocked_refusal_is_atomic", BlockedRefusal);
            yield return new Scenario("regression.destination_access_scalar_blocked_anchor", () => DestinationBlockedAnchor(false));
            yield return new Scenario("regression.destination_access_exact_blocked_anchor", () => DestinationBlockedAnchor(true));
            yield return new Scenario("regression.destination_access_scalar_closed_footprint_recovers", () => DestinationClosedRoute(false));
            yield return new Scenario("regression.destination_access_exact_closed_footprint_recovers", () => DestinationClosedRoute(true));
            yield return new Scenario("regression.destination_access_scalar_changed_capacity_and_filter", () => DestinationChanged(false));
            yield return new Scenario("regression.destination_access_exact_changed_capacity_and_filter", () => DestinationChanged(true));
            yield return new Scenario("regression.destination_access_finite_scalar_recipe", () => ProductionDestination(false));
            yield return new Scenario("regression.destination_access_finite_exact_recipe", () => ProductionDestination(true));
            yield return new Scenario("regression.destination_access_closed_refusal_is_atomic", DestinationRefusal);
            yield return new Scenario("acceptance.haul_access_steward_finite_tool_chain", StewardFiniteToolChain);
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static ActionResult Apply(World world, Village village, GameAction action) => world.Apply(new PlayerContext { PlayerId = "haul-access-acceptance", VillageId = village.Id }, action);
        static ActionResult Act(World world, Village village, GameAction action)
        {
            var result = Apply(world, village, action);
            Check(result.Success, action.Kind + ": " + result.Error);
            return result;
        }
        static void Place(Cat cat, Int2 position)
        {
            cat.Position = position;
            cat.X = position.X;
            cat.Z = position.Z;
            cat.Path.Clear();
        }
        static Int2 Alternate(Stockpile source) => new Int2(source.Position.X + 1, source.Position.Z, source.Position.Level);
        static string Avoid(World world, Village village, Int2 position) => Act(world, village, new GameAction { Kind = "CreateZone", Resource = "avoid", Position = position, End = position }).EntityId;
        static void Wait(World world, Func<bool> ready, int seconds, string message)
        {
            for (int elapsed = 0; elapsed < seconds && !ready(); elapsed++) world.Step(1);
            Check(ready(), message);
        }

        static World Fixture(bool exact, out Village village, out Cat carrier, out Stockpile source, out Stockpile destination, out Item item)
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
            var knownLogs = world.Tiles.First(t => t.Resource == "logs" && t.Amount > 0).Position;
            if (!village.Known.Contains(knownLogs)) village.Known.Add(knownLogs);
            foreach (var cat in village.Cats)
            {
                cat.ControlledBy = "fixture-held";
                cat.ControlLeaseUntil = 10000000;
                cat.Hunger = cat.Thirst = cat.Rest = 100;
                cat.Skills.Clear();
                cat.Preferences.Clear();
            }
            carrier = village.Cats[1];
            carrier.ControlledBy = "";
            Place(carrier, new Int2(1, 3));
            // A normal multi-tile storage footprint becomes a recoverable spill
            // through RemoveStockpile. Its goods are finite fixture credit.
            source = new Stockpile { Id = world.Id("haul-access-source"), Position = new Int2(3, 3), Width = 2, Depth = 1, Capacity = 100 };
            destination = new Stockpile { Id = world.Id("haul-access-destination"), Position = new Int2(-3, 5), Width = 1, Depth = 1, Capacity = 100, Accepts = new List<string> { "logs", "stone", "planks", "blocks", "tools" } };
            village.Stockpiles.Add(source);
            village.Stockpiles.Add(destination);
            item = exact ? new Item { Id = world.Id("haul-access-tool"), Kind = "tool", VillageId = village.Id, LocationId = source.Id, Material = "metal", Quality = 3, Condition = 47, MaxCondition = 82 } : null;
            if (item == null) source.Goods.Add(new Stack("logs", 8));
            else village.Items.Add(item);
            foreach (var position in new[] { source.Position, Alternate(source), destination.Position })
            {
                World.ClearSurface(world.TileAt(position), 0);
                world.TileAt(position).Wall = false;
                if (!village.Known.Contains(position)) village.Known.Add(position);
            }
            Act(world, village, new GameAction { Kind = "RemoveStockpile", TargetId = source.Id });
            Check(source.Kind == "spill" && source.Width == 2 && source.Depth == 1, "Removing the multi-tile storage did not retain its real source footprint");
            return world;
        }

        static Job Start(World world, Village village, Cat carrier, Stockpile source, Item item, bool unnamed = false)
        {
            var result = Apply(world, village, new GameAction { Kind = "HaulGatherSpot", CatId = unnamed ? "" : carrier.Id, TargetId = source.Id });
            Check(result.Success, "A reachable source-footprint tile did not permit physical hauling: " + result.Error);
            var job = village.Jobs.Single(j => j.Id == result.EntityId);
            Check(job.Kind == "haul" && job.CatId == carrier.Id && carrier.JobId == job.Id && job.SourceId == source.Id, "Footprint selection changed source or carrier ownership");
            Check(carrier.Cargo.Count == 0 && !job.Completed, "Dispatch moved finite goods before physical pickup");
            if (item == null)
                Check(job.Phase == "fetch" && World.Amount(source.Goods, "logs") == 8 && world.Reservations.Single(r => r.OwnerId == job.Id).PileId == source.Id && world.Reservations.Single(r => r.OwnerId == job.Id).Amount == 8, "Scalar dispatch changed its finite load instead of reserving the same source");
            else
                Check(job.Phase == "item_fetch" && item.LocationId == job.Id && job.ItemIds.SequenceEqual(new[] { item.Id }), "Exact dispatch did not exclusively claim the source item");
            return job;
        }

        static double Logs(Village village) => village.Stockpiles.Sum(p => World.Amount(p.Goods, "logs")) + village.Cats.Sum(c => World.Amount(c.Cargo, "logs")) + village.Jobs.Where(j => !j.Completed).Sum(j => World.Amount(j.Local, "logs"));
        static void ExactUnchanged(Village village, Item item) => Check(village.Items.Count(i => i.Id == item.Id) == 1 && item.VillageId == village.Id && item.Material == "metal" && item.Quality == 3 && item.Condition == 47 && item.MaxCondition == 82, "Footprint hauling changed exact item identity, ownership or condition");
        static void Pickup(World world, Village village, Cat carrier, Stockpile source, Item item, Job job)
        {
            for (int step = 0; step < 1000 && (item == null ? carrier.Cargo.Count == 0 : job.Phase == "item_fetch"); step++)
            {
                world.Step(.05);
                if (item == null) Check(Logs(village) == 8, "Source access changed the finite scalar load before pickup");
                else ExactUnchanged(village, item);
            }
            var access = Alternate(source);
            Check(!job.Completed && carrier.Position.Equals(access) && Math.Abs(carrier.X - access.X) < 1e-9 && Math.Abs(carrier.Z - access.Z) < 1e-9, "Haul did not physically reach the open footprint tile before pickup");
            Check(!world.Walkable(village, source.Position), "Pickup removed the original anchor obstruction");
            if (item == null)
                Check(World.Amount(carrier.Cargo, "logs") == 8 && World.Amount(source.Goods, "logs") == 0 && !world.Reservations.Any(r => r.OwnerId == job.Id), "Physical scalar pickup did not consume its exact source claim once");
            else
                Check(job.Phase == "output_delivery" && item.LocationId == job.Id && job.ItemIds.SequenceEqual(new[] { item.Id }), "Exact item left pickup before its carrier reached the physical footprint");
        }
        static void Finish(World world, Village village, Cat carrier, Stockpile source, Stockpile destination, Item item, Job job)
        {
            Wait(world, () => job.Completed, 60, "Hauling from an open source-footprint tile did not physically finish");
            Check(carrier.Position.Equals(destination.Position) && carrier.JobId == "" && carrier.Cargo.Count == 0 && job.ItemIds.Count == 0 && world.Reservations.Count == 0 && village.Events.Count(e => e.Kind == "completed" && e.EntityId == job.Id) == 1, "Footprint delivery lost original work ownership, retained cargo or completed twice");
            if (item == null) Check(World.Amount(destination.Goods, "logs") == 8 && World.Amount(source.Goods, "logs") == 0 && Logs(village) == 8, "Footprint delivery failed finite scalar conservation");
            else { ExactUnchanged(village, item); Check(item.LocationId == destination.Id, "Footprint delivery placed the exact item in the wrong store"); }
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void BlockedAnchor(bool exact)
        {
            var world = Fixture(exact, out var village, out var carrier, out var source, out var destination, out var item);
            Avoid(world, village, source.Position);
            Check(world.Path(carrier.Position, source.Position, village) == null && world.Path(carrier.Position, Alternate(source), village) != null, "Blocked-anchor fixture must retain an alternate source-footprint route");
            var job = Start(world, village, carrier, source, item);
            Pickup(world, village, carrier, source, item, job);
            Finish(world, village, carrier, source, destination, item, job);
        }

        static void RankedCarrier()
        {
            var world = Fixture(false, out var village, out var blocked, out var source, out var destination, out var item);
            var reachable = village.Cats[2];
            reachable.ControlledBy = "";
            Place(reachable, new Int2(5, 3));
            blocked.Preferences.Add("haul");
            World.Add(blocked.Skills, "haul", 50);
            foreach (var at in new[] { new Int2(0, 3), new Int2(2, 3), new Int2(1, 2), new Int2(1, 4) }) world.TileAt(at).Wall = true;
            Avoid(world, village, source.Position);
            Check(world.Path(blocked.Position, Alternate(source), village) == null && world.Path(reachable.Position, Alternate(source), village) != null, "Ranked fixture must separate the preferred cat from the accessible footprint");
            var job = Start(world, village, reachable, source, item, true);
            Check(blocked.JobId == "" && blocked.Cargo.Count == 0 && blocked.Preferences.Contains("haul") && World.Amount(blocked.Skills, "haul") == 50, "Footprint fallback changed the blocked preferred cat");
            Pickup(world, village, reachable, source, item, job);
            Finish(world, village, reachable, source, destination, item, job);
        }

        static void ClosedRoute(bool exact)
        {
            var world = Fixture(exact, out var village, out var carrier, out var source, out var destination, out var item);
            var job = Start(world, village, carrier, source, item);
            for (int step = 0; step < 100 && carrier.X == carrier.Position.X; step++) world.Step(.05);
            Check(carrier.X > carrier.Position.X && carrier.X < carrier.Position.X + 1, "Closure fixture did not start a real fractional approach");
            var claim = exact ? null : world.Reservations.Single(r => r.OwnerId == job.Id);
            Avoid(world, village, source.Position);
            world.Step(.05);
            Check(carrier.Cargo.Count == 0 && (exact ? job.Phase == "item_fetch" : World.Amount(source.Goods, "logs") == 8), "Changed source access picked up goods remotely");
            string alternateZone = Avoid(world, village, Alternate(source));
            world.Step(3);
            Check(!job.Completed && carrier.JobId == job.Id && job.CatId == carrier.Id && carrier.Cargo.Count == 0 && carrier.BlockedReason == "blocked_route", "Fully closed source did not hold the same carrier and physical pickup");
            if (exact)
            {
                ExactUnchanged(village, item);
                Check(job.Phase == "item_fetch" && item.LocationId == job.Id && job.ItemIds.SequenceEqual(new[] { item.Id }), "Blocked exact pickup lost its exclusive item claim");
            }
            else
                Check(World.Amount(source.Goods, "logs") == 8 && ReferenceEquals(world.Reservations.Single(r => r.OwnerId == job.Id), claim) && claim.Amount == 8 && Logs(village) == 8, "Blocked scalar pickup replaced or consumed its untouched source claim");
            Act(world, village, new GameAction { Kind = "RemoveZone", TargetId = alternateZone });
            Pickup(world, village, carrier, source, item, job);
            Finish(world, village, carrier, source, destination, item, job);
        }

        static void BlockedRefusal()
        {
            var world = Fixture(false, out var village, out var carrier, out var source, out _, out _);
            Avoid(world, village, source.Position);
            Avoid(world, village, Alternate(source));
            long nextId = world.NextId;
            uint entropy = world.RandomState;
            double time = world.TimeSeconds;
            foreach (bool unnamed in new[] { false, true })
            {
                var result = Apply(world, village, new GameAction { Kind = "HaulGatherSpot", CatId = unnamed ? "" : carrier.Id, TargetId = source.Id });
                Check(!result.Success && world.NextId == nextId && world.RandomState == entropy && world.TimeSeconds == time && village.Jobs.Count == 0 && world.Reservations.Count == 0 && carrier.JobId == "" && carrier.Cargo.Count == 0 && World.Amount(source.Goods, "logs") == 8, "Refused fully blocked footprint changed finite goods, work, claims, identity, entropy or time");
            }
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void WidenDestination(World world, Village village, Stockpile destination)
        {
            destination.Width = 2;
            var alternate = Alternate(destination);
            World.ClearSurface(world.TileAt(alternate), 0);
            world.TileAt(alternate).Wall = false;
            if (!village.Known.Contains(alternate)) village.Known.Add(alternate);
        }
        static void CollectForDelivery(World world, Village village, Cat carrier, Stockpile source, Item item, Job job)
        {
            for (int step = 0; step < 1000 && (item == null ? carrier.Cargo.Count == 0 : job.Phase == "item_fetch"); step++)
            {
                world.Step(.05);
                if (item == null) Check(Logs(village) == 8, "Destination selection changed finite source goods before pickup");
                else ExactUnchanged(village, item);
            }
            Check(carrier.Position.Equals(source.Position) && Math.Abs(carrier.X - source.Position.X) + Math.Abs(carrier.Z - source.Position.Z) < 1e-9 && !job.Completed, "Destination delivery bypassed physical source arrival");
            Check(item == null ? World.Amount(carrier.Cargo, "logs") == 8 && source.Goods.Count == 0 && !world.Reservations.Any(r => r.OwnerId == job.Id) : job.Phase == "output_delivery" && item.LocationId == job.Id, "Destination delivery did not retain the exact picked-up load");
        }
        static void FinishAtFootprint(World world, Village village, Cat carrier, Stockpile destination, Item item, Job job)
        {
            var access = Alternate(destination);
            for (int step = 0; step < 2000 && !job.Completed; step++)
            {
                Check(item == null ? World.Amount(destination.Goods, "logs") == 0 && Logs(village) == 8 : item.LocationId == job.Id, "Delivery transferred goods before physical footprint arrival");
                world.Step(.05);
                if (item != null) ExactUnchanged(village, item);
            }
            Check(job.Completed && carrier.Position.Equals(access) && Math.Abs(carrier.X - access.X) + Math.Abs(carrier.Z - access.Z) < 1e-9 && !world.Walkable(village, destination.Position), "Carrier did not physically deliver at the reachable destination-footprint tile");
            Check(carrier.JobId == "" && carrier.Cargo.Count == 0 && job.Local.Count == 0 && job.ItemIds.Count == 0 && world.Reservations.Count == 0 && village.Events.Count(e => e.Kind == "completed" && e.EntityId == job.Id) == 1, "Destination-footprint delivery retained work or completed twice");
            if (item == null) Check(World.Amount(destination.Goods, "logs") == 8 && Logs(village) == 8, "Destination-footprint delivery changed its finite scalar load");
            else Check(item.LocationId == destination.Id, "Destination-footprint delivery lost exact item ownership");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }
        static void DestinationBlockedAnchor(bool exact)
        {
            var world = Fixture(exact, out var village, out var carrier, out var source, out var destination, out var item);
            WidenDestination(world, village, destination);
            Avoid(world, village, destination.Position);
            Check(world.Path(source.Position, destination.Position, village) == null && world.Path(source.Position, Alternate(destination), village) != null, "Destination fixture did not retain a reachable alternate footprint tile");
            var job = Start(world, village, carrier, source, item);
            CollectForDelivery(world, village, carrier, source, item, job);
            FinishAtFootprint(world, village, carrier, destination, item, job);
        }
        static void DestinationClosedRoute(bool exact)
        {
            var world = Fixture(exact, out var village, out var carrier, out var source, out var destination, out var item);
            WidenDestination(world, village, destination);
            var job = Start(world, village, carrier, source, item);
            CollectForDelivery(world, village, carrier, source, item, job);
            for (int step = 0; step < 100 && Math.Abs(carrier.X - carrier.Position.X) + Math.Abs(carrier.Z - carrier.Position.Z) < 1e-9; step++) world.Step(.05);
            Check(!job.Completed && carrier.Path.Count > 0 && Math.Abs(carrier.X - carrier.Position.X) + Math.Abs(carrier.Z - carrier.Position.Z) > 1e-9, "Destination closure did not interrupt a real fractional loaded approach");
            // Interrupt a loaded physical approach. The carrier must keep its
            // load, then recover into the same pile's remaining open tile.
            Avoid(world, village, destination.Position);
            string alternateZone = Avoid(world, village, Alternate(destination));
            world.Step(3);
            Check(!job.Completed && job.CatId == carrier.Id && carrier.JobId == job.Id && destination.Goods.Count == 0 && (item == null ? World.Amount(carrier.Cargo, "logs") == 8 && Logs(village) == 8 : item.LocationId == job.Id && job.ItemIds.SequenceEqual(new[] { item.Id })), "Closed destination lost, transferred remotely or released its original physical load");
            world.Step(Math.Ceiling(world.TimeSeconds) - world.TimeSeconds + .1);
            long searches = (long)typeof(World).GetField("pathSearchCount", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic).GetValue(world);
            world.Step(.2);
            Check((long)typeof(World).GetField("pathSearchCount", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic).GetValue(world) == searches, "Closed destination searched again within the same retry interval");
            Act(world, village, new GameAction { Kind = "RemoveZone", TargetId = alternateZone });
            FinishAtFootprint(world, village, carrier, destination, item, job);
        }
        static void DestinationRefusal()
        {
            foreach (bool exact in new[] { false, true })
            {
                var world = Fixture(exact, out var village, out var carrier, out var source, out var destination, out var item);
                WidenDestination(world, village, destination);
                Avoid(world, village, destination.Position);
                Avoid(world, village, Alternate(destination));
                long nextId = world.NextId;
                uint entropy = world.RandomState;
                double time = world.TimeSeconds;
                foreach (bool unnamed in new[] { false, true })
                {
                    var result = Apply(world, village, new GameAction { Kind = "HaulGatherSpot", CatId = unnamed ? "" : carrier.Id, TargetId = source.Id });
                    Check(!result.Success && world.NextId == nextId && world.RandomState == entropy && world.TimeSeconds == time && village.Jobs.Count == 0 && world.Reservations.Count == 0 && carrier.JobId == "" && carrier.Cargo.Count == 0 && (item == null ? World.Amount(source.Goods, "logs") == 8 : item.LocationId == source.Id), "Refused fully closed destination changed finite goods, claims, identity, entropy or time");
                }
                if (item != null) ExactUnchanged(village, item);
                Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
            }
        }
        static void DestinationChanged(bool exact)
        {
            foreach (bool fill in new[] { true, false })
            {
                var world = Fixture(exact, out var village, out var carrier, out var source, out var destination, out var item);
                WidenDestination(world, village, destination);
                Avoid(world, village, destination.Position);
                destination.Capacity = exact ? 1 : 8;
                var job = Start(world, village, carrier, source, item);
                CollectForDelivery(world, village, carrier, source, item, job);
                for (int step = 0; step < 100 && carrier.Path.Count == 0; step++) world.Step(.05);
                Check(!job.Completed && carrier.Path.Count > 0 && carrier.Path.Last().Equals(Alternate(destination)), "Changed destination fixture did not begin a physical footprint delivery");
                var spare = new Stockpile { Id = world.Id("destination-access-spare"), Position = new Int2(5, 5), Width = 2, Depth = 1, Capacity = 100, Accepts = new List<string> { exact ? "tools" : "logs" } };
                village.Stockpiles.Add(spare);
                foreach (var at in new[] { spare.Position, Alternate(spare) }) { World.ClearSurface(world.TileAt(at), 0); world.TileAt(at).Wall = false; if (!village.Known.Contains(at)) village.Known.Add(at); }
                Avoid(world, village, spare.Position);
                if (fill) World.Add(destination.Goods, "stone", exact ? 1 : 8);
                else destination.Accepts = new List<string> { "food", "water" };
                FinishAtFootprint(world, village, carrier, spare, item, job);
                Check(World.Amount(destination.Goods, "logs") == 0 && World.Amount(destination.Goods, "stone") == (fill ? exact ? 1 : 8 : 0) && (item == null || item.LocationId != destination.Id), "Changed capacity or filter accepted an invalid arriving load");
            }
        }
        static void ProductionDestination(bool exact)
        {
            var world = Fixture(false, out var village, out var worker, out var source, out var destination, out _);
            source.Goods.Clear();
            WidenDestination(world, village, destination);
            Avoid(world, village, destination.Position);
            var station = new Building { Id = world.Id("destination-access-station"), Kind = exact ? "woodworking" : "wood_cutter", Completed = true, Position = new Int2(3, 5) };
            village.Buildings.Add(station);
            var recipe = Catalog.Recipe(exact ? "planks_and_blocks_to_tools" : "logs_to_planks");
            foreach (var input in recipe.Inputs) World.Add(station.Inputs, input.Resource, world.RecipeInput(village, recipe, input.Amount));
            Place(worker, station.Position);
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = worker.Id });
            var add = Edit(station, worker, "add"); add.RecipeId = recipe.Id; Act(world, village, add);
            world.Step(1.05);
            var job = village.Jobs.Find(j => !j.Completed && j.Kind == "production" && j.CatId == worker.Id);
            Check(job != null, "Reachable destination-footprint capacity did not start the player's finite recipe");
            Wait(world, () => job.Phase == "output_delivery", 900, "Finite recipe did not perform its real physical work");
            Check(job.Position.Equals(station.Position) && !job.Completed && station.Slots.Single().Queue.Count == 0 && World.Amount(destination.Goods, "planks") == 0, "Output routing moved the station, replayed its finite order or transferred before arrival");
            var produced = exact ? village.Items.Single(i => i.LocationId == job.Id) : null;
            string itemId = produced?.Id;
            for (int step = 0; step < 2000 && !job.Completed; step++)
            {
                Check(exact ? produced.LocationId == job.Id && produced.Id == itemId : World.Amount(destination.Goods, "planks") == 0 && World.Amount(worker.Cargo, "planks") + World.Amount(job.Local, "planks") == world.RecipeOutput(village, recipe, 1), "Finite output transferred before physical footprint arrival");
                world.Step(.05);
            }
            var access = Alternate(destination);
            Check(job.Completed && worker.Position.Equals(access) && Math.Abs(worker.X - access.X) + Math.Abs(worker.Z - access.Z) < 1e-9 && !world.Walkable(village, destination.Position) && job.Position.Equals(station.Position), "Finite recipe failed physical destination-footprint delivery or replaced its source station");
            Check(exact ? village.Items.Count == 1 && produced.Id == itemId && produced.LocationId == destination.Id && produced.Kind == "tool" && produced.Material == "wood" && produced.Quality == 1 && produced.Condition == 100 && produced.MaxCondition == 100 : world.Total(village, "planks") == world.RecipeOutput(village, recipe, 1), "Finite recipe changed its exact or scalar output");
            Check(job.Local.Count == 0 && worker.Cargo.Count == 0 && job.ItemIds.Count == 0 && station.Inputs.Count == 0 && world.Reservations.Count == 0 && village.Events.Count(e => e.Kind == "completed" && e.EntityId == job.Id) == 1 && world.Validate().Count == 0, "Finite recipe retained ingredients, output claims or duplicate completions");
        }

        static GameAction Edit(Building station, Cat worker, string edit) => new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = edit };
        static void StewardFiniteToolChain()
        {
            var world = Fixture(false, out var village, out var carrier, out var source, out var destination, out _);
            WidenDestination(world, village, destination);
            Avoid(world, village, destination.Position);
            village.Cats.RemoveRange(5, village.Cats.Count - 5);
            World.Add(source.Goods, "logs", 2);
            World.Add(source.Goods, "stone", 10);
            var office = new Building { Id = world.Id("haul-access-office"), Kind = "workshop", Completed = true, Position = new Int2(5, 6) };
            var cutter = new Building { Id = world.Id("haul-access-cutter"), Kind = "wood_cutter", Completed = true, Position = new Int2(1, 5) };
            var prep = new Building { Id = world.Id("haul-access-prep"), Kind = "stone_prep", Completed = true, Position = new Int2(3, 5) };
            var bench = new Building { Id = world.Id("haul-access-bench"), Kind = "woodworking", Completed = true, Position = new Int2(-1, 3) };
            village.Buildings.AddRange(new[] { office, cutter, prep, bench });
            Check(!village.Buildings.Any(b => b.Position.Level == destination.Position.Level && b.Position.X < destination.Position.X + destination.Width && b.Position.X + b.Width > destination.Position.X && b.Position.Z < destination.Position.Z + destination.Depth && b.Position.Z + b.Depth > destination.Position.Z), "Finite chain receiving storage must be disjoint from every workshop and home");
            village.Research.Add("basic_tools");
            var sawyer = village.Cats[2];
            var mason = village.Cats[3];
            var crafter = village.Cats[4];
            foreach (var work in new[] { (station: cutter, worker: sawyer, recipe: "logs_to_planks", batches: 2), (station: prep, worker: mason, recipe: "stone_to_blocks", batches: 2), (station: bench, worker: crafter, recipe: "planks_and_blocks_to_tools", batches: 1) })
            {
                work.worker.ControlledBy = "";
                Place(work.worker, work.station.Position);
                Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = work.station.Id, CatId = work.worker.Id });
                for (int batch = 0; batch < work.batches; batch++)
                {
                    var add = Edit(work.station, work.worker, "add");
                    add.RecipeId = work.recipe;
                    Act(world, village, add);
                }
                var pause = Edit(work.station, work.worker, "pause");
                pause.Enabled = true;
                Act(world, village, pause);
            }
            Avoid(world, village, source.Position);
            Act(world, village, new GameAction { Kind = "AssignOfficer", Role = "steward", CatId = village.Cats[0].Id });
            Wait(world, () => village.Jobs.Any(j => !j.Completed && j.Kind == "haul" && j.SourceId == source.Id), 12, "Steward did not dispatch reachable footprint cargo for the finite chain");
            var first = village.Jobs.Single(j => !j.Completed && j.Kind == "haul" && j.SourceId == source.Id);
            Check(first.CatId == carrier.Id && first.AutomatedBy == "steward" && !first.Manual, "Finite chain bypassed its real Steward carrier");
            for (int step = 0; step < 1000 && carrier.Cargo.Count == 0; step++) world.Step(.05);
            Check(World.Amount(carrier.Cargo, "logs") == 8 && carrier.Position.Equals(Alternate(source)), "Steward did not physically pick up its first eight Logs at the open footprint");
            carrier.Thirst = 1;
            world.Step(.05);
            string spillId = first.SuspendedCargoPileId;
            Check(carrier.Goal == "need_drink" && carrier.ResumeJobId == first.Id && spillId != "" && world.Reservations.Any(r => r.OwnerId == first.Id + ":resume" && r.PileId == spillId && r.Amount == 8), "Needs did not suspend the same finite hauling load in an exact physical spill");
            Wait(world, () => World.Amount(destination.Goods, "logs") == 10 && World.Amount(destination.Goods, "stone") == 10 && !village.Jobs.Any(j => !j.Completed && j.Kind == "haul"), 180, "Steward did not resume and physically supply both finite raw bills");
            Check(source.Goods.Count == 0 && world.Reservations.Count == 0 && carrier.Cargo.Count == 0 && first.Completed && first.SuspendedCargoPileId == "" && village.Jobs.Count(j => j.Kind == "haul" && j.Completed && j.SourceId == source.Id) == 4, "Steward supply retained claims, replayed a trip or lost its original needs-suspended load");
            var haulIds = village.Jobs.Where(j => j.Kind == "haul" && j.Completed && j.SourceId == source.Id).Select(j => j.Id).ToArray();
            Check(new[] { cutter, prep, bench }.All(b => b.Slots.Single().Paused && b.Slots.Single().Queue.All(q => !q.Repeat)) && !village.Jobs.Any(j => j.Kind == "production"), "Finite workshop chain bypassed its deliberately paused receiving queues");
            // Parking the finished carrier at the empty office prevents unrelated
            // founding scouting from entering this finite material experiment.
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = office.Id, CatId = carrier.Id });
            foreach (var work in new[] { (station: cutter, worker: sawyer), (station: prep, worker: mason), (station: bench, worker: crafter) })
            {
                var resume = Edit(work.station, work.worker, "pause");
                resume.Enabled = false;
                Act(world, village, resume);
            }
            for (int step = 0; step < 48000 && !(new[] { cutter, prep, bench }.All(b => b.Slots.Single().Queue.Count == 0) && !village.Jobs.Any(j => !j.Completed && j.Kind == "production")); step++) world.Step(.05);
            Check(new[] { cutter, prep, bench }.All(b => b.Slots.Single().Queue.Count == 0) && !village.Jobs.Any(j => !j.Completed && j.Kind == "production"), "Steward-supplied finite workshops failed to deliver the connected wooden Tool");
            var receivingTile = Alternate(destination);
            Check(crafter.Position.Equals(receivingTile) && Math.Abs(crafter.X - receivingTile.X) + Math.Abs(crafter.Z - receivingTile.Z) < 1e-9 && !bench.Position.Equals(receivingTile), "Final exact Tool must travel from its distinct crafting station and arrive physically at the open receiving tile");
            Check(village.Items.Count == 1 && village.Items.Single().Kind == "tool" && village.Items.Single().Material == "wood" && village.Items.Single().Quality == 1 && village.Items.Single().Condition == 100 && village.Items.Single().LocationId == destination.Id, "Connected physical chain did not deliver exactly one unchanged wooden Tool");
            var tool = village.Items.Single();
            string toolId = tool.Id;
            // Completed jobs deliberately expire after ten minutes. Completion
            // events outlive this bounded chain and retain their exact job IDs.
            var completions = village.Events.Where(e => e.Kind == "completed").Select(e => (e.EntityId, e.Text)).ToArray();
            var productionIds = village.Events.Where(e => e.Kind == "completed" && e.Text.EndsWith(" completed production", StringComparison.Ordinal)).Select(e => e.EntityId).ToArray();
            Check(productionIds.Length == 5 && productionIds.Distinct().Count() == 5 && haulIds.Length == 4 && haulIds.All(id => completions.Count(e => e.EntityId == id) == 1) && completions.Length == 9, "Finite material chain did not complete its five exact production batches and four original hauls once");
            long nextId = world.NextId;
            world.Step(31);
            Check(village.Items.Count == 1 && ReferenceEquals(village.Items.Single(), tool) && tool.Id == toolId && tool.Kind == "tool" && tool.Material == "wood" && tool.Quality == 1 && tool.Condition == 100 && tool.LocationId == destination.Id && world.NextId == nextId && village.Events.Where(e => e.Kind == "completed").Select(e => (e.EntityId, e.Text)).SequenceEqual(completions), "Completed finite queues replayed a batch, haul or exact Tool during later planning");
            foreach (string resource in new[] { "logs", "stone", "planks", "blocks" })
                Check(village.Stockpiles.Sum(p => World.Amount(p.Goods, resource)) + village.Cats.Sum(c => World.Amount(c.Cargo, resource)) + village.Jobs.Where(j => !j.Completed).Sum(j => World.Amount(j.Local, resource)) + village.Buildings.Sum(b => World.Amount(b.Inputs, resource) + World.Amount(b.Outputs, resource)) == 0, "Connected chain retained or duplicated finite " + resource);
            Check(world.Reservations.Count == 0 && village.Cats.All(c => c.Cargo.Count == 0) && !village.Jobs.Any(j => !j.Completed) && productionIds.All(id => village.Events.Count(e => e.Kind == "completed" && e.EntityId == id) == 1) && village.Stockpiles.Single(p => p.Id == spillId).Goods.Count == 0, "Completed connected chain retained cargo, needs goods, claims or unfinished work");
            Check(!world.Walkable(village, source.Position) && !world.Walkable(village, destination.Position) && world.Validate().Count == 0, "Connected chain changed the source or destination obstruction or violated authoritative state: " + string.Join("; ", world.Validate()));
        }
    }
}
