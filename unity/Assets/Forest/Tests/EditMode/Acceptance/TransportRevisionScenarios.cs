using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class TransportRevisionScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            foreach (bool exact in new[] { false, true })
            {
                bool equipment = exact;
                string cargo = exact ? "exact" : "scalar";
                yield return new Scenario("regression.shipping_boundary_holds_" + cargo, () => ShippingObstruction(equipment, false, false));
                yield return new Scenario("regression.shipping_walled_destination_holds_" + cargo, () => ShippingObstruction(equipment, false, true));
                yield return new Scenario("regression.shipping_cancel_boundary_holds_" + cargo, () => ShippingObstruction(equipment, true, false));
                yield return new Scenario("regression.shipping_cancel_walled_source_holds_" + cargo, () => ShippingObstruction(equipment, true, true));
                yield return new Scenario("regression.shipping_cancel_reopened_return_clears_warning_while_loaded_" + cargo, () => ShippingCancelledRecovery(equipment));
                yield return new Scenario("regression.shipping_cancel_reboarding_obstruction_keeps_warning_" + cargo, () => ShippingCancelledReboarding(equipment));
                yield return new Scenario("regression.shipping_varying_depth_" + cargo, () => ShippingDepth(equipment));
            }
        }

        static World Fixture(bool exact, out Village village, out Cat driver, out Stockpile source, out Stockpile destination, out Vehicle vessel, out Item item, bool repeat = false)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            village.Stockpiles[0].Position = new Int2(1, 4);
            village.Stockpiles[0].Width = village.Stockpiles[0].Depth = 1;
            World.ClearSurface(world.TileAt(village.Stockpiles[0].Position), 0);
            world.TileAt(village.Stockpiles[0].Position).Wall = false;
            village.Stockpiles[0].Capacity = 100000;
            village.Stockpiles[0].Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000) };
            village.LastLeaderResearch = 0;
            foreach (var cat in village.Cats)
            {
                cat.ControlledBy = "fixture-held";
                cat.ControlLeaseUntil = 10000000;
            }
            driver = village.Cats[0];
            driver.ControlledBy = "";
            driver.BuildingId = "";
            driver.Position = new Int2(1, 5);
            driver.X = 1;
            driver.Z = 5;
            driver.Hunger = driver.Thirst = driver.Rest = 100;
            var path = Enumerable.Range(1, 5).Select(x => new Int2(x, 5)).ToList();
            foreach (var position in path)
            {
                var tile = world.TileAt(position);
                World.ClearSurface(tile, 0);
                tile.Wall = tile.Bridge = tile.Rail = tile.Road = tile.Dirt = false;
                tile.Dock = position.Equals(path[0]) || position.Equals(path[path.Count - 1]);
                if (!tile.Dock)
                    World.SetWaterSurface(tile, 0, position.X == 3 ? 3.2 : .8);
            }
            source = new Stockpile { Id = world.Id("shipping-source"), Position = path[0], Width = 1, Depth = 1, Capacity = 8 };
            destination = new Stockpile { Id = world.Id("shipping-destination"), Position = path[path.Count - 1], Width = 1, Depth = 1, Capacity = 8 };
            village.Stockpiles.Add(source);
            village.Stockpiles.Add(destination);
            foreach (var pile in village.Stockpiles)
                for (int z = 0; z < pile.Depth; z++)
                    for (int x = 0; x < pile.Width; x++)
                    {
                        var at = new Int2(pile.Position.X + x, pile.Position.Z + z, pile.Position.Level);
                        Check(!village.Buildings.Any(b => Contains(b.Position, b.Width, b.Depth, at)) && !village.Stockpiles.Any(other => other != pile && Contains(other.Position, other.Width, other.Depth, at)), "Shipping fixture stockpile footprint overlaps a retained building or another pile");
                        Check(!world.TileAt(at).Water && !world.TileAt(at).Wall && !world.TileAt(at).Mountain, "Shipping fixture stockpile must be on dry usable ground");
                        if (pile != source && pile != destination)
                            Check(!path.Contains(at), "Shipping funding reserve overlaps a dock or water waypoint");
                    }
            item = null;
            if (exact)
            {
                item = new Item { Id = world.Id("shipping-tool"), Kind = "tool", VillageId = village.Id, LocationId = source.Id, Material = "metal", Quality = 3, Condition = 47, MaxCondition = 82 };
                village.Items.Add(item);
            }
            else
                source.Goods.Add(new Stack("logs", 8));
            vessel = new Vehicle { Id = world.Id("shipping-vessel"), Mode = "shipping", Position = source.Position };
            village.Vehicles.Add(vessel);
            var result = world.Apply(Context(village), new GameAction { Kind = "CreateTransportRoute", CatId = driver.Id, TargetId = source.Id, BuildingId = destination.Id, Mode = "shipping", Resource = exact ? "tools" : "logs", Amount = exact ? 1 : 8, Path = path, Repeat = repeat });
            Check(result.Success, "Public shipping route failed: " + result.Error);
            return world;
        }

        static PlayerContext Context(Village village) => new PlayerContext { PlayerId = "transport-acceptance", VillageId = village.Id };
        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static bool Contains(Int2 anchor, int width, int depth, Int2 at) => anchor.Level == at.Level && at.X >= anchor.X && at.X < anchor.X + width && at.Z >= anchor.Z && at.Z < anchor.Z + depth;
        static void Near(double actual, double expected, string message) => Check(Math.Abs(actual - expected) < 0.00001, message + " expected=" + expected + " actual=" + actual);
        static void Until(World world, Func<bool> condition, string message)
        {
            for (int step = 0; step < 400 && !condition(); step++)
                world.Step(World.SimulationStepSeconds);
            Check(condition(), message);
        }
        static void Cargo(Village village, Vehicle vessel, Item item, bool aboard)
        {
            if (item == null)
            {
                Near(World.Amount(vessel.Cargo, "logs"), aboard ? 8 : 0, "Vessel changed its finite scalar load");
                Near(village.Stockpiles.Sum(p => World.Amount(p.Goods, "logs")) + village.Vehicles.Sum(v => World.Amount(v.Cargo, "logs")), 8, "Shipping lost or duplicated scalar cargo");
            }
            else
            {
                Check(village.Items.Count(i => i.Id == item.Id) == 1 && item.VillageId == village.Id && item.Material == "metal" && item.Quality == 3 && item.Condition == 47 && item.MaxCondition == 82, "Shipping changed exact cargo identity, owner or condition");
                Check(aboard ? item.LocationId == vessel.Id && vessel.ItemIds.SequenceEqual(new[] { item.Id }) : vessel.ItemIds.Count == 0, "Vessel changed its exact cargo ownership");
                Near(village.Stockpiles.Sum(p => World.Amount(p.Goods, "tools")) + village.Vehicles.Sum(v => World.Amount(v.Cargo, "tools")), 0, "Shipping created scalar equipment copies");
            }
        }

        static void ShippingObstruction(bool exact, bool cancel, bool wall)
        {
            var world = Fixture(exact, out var village, out var driver, out var source, out var destination, out var vessel, out var item);
            var route = village.Routes.Single();
            int anchor = wall && !cancel ? route.Path.Count - 2 : 1;
            Until(world, () => route.Phase == "outbound" && route.PathIndex == anchor && vessel.X == vessel.Position.X, "Vessel did not physically reach its obstruction approach");
            var reached = vessel.Position;
            world.Step(.25);
            Check(vessel.Position.Equals(reached) && vessel.X > reached.X && vessel.X < reached.X + 1, "Obstruction fixture must interrupt a fractional sailing edge");
            var target = cancel ? route.Path[0] : route.Path[anchor + 1];
            var edge = new BoundaryEdge { From = reached, To = target };
            // These are persisted physical obstacles, introduced after the route was accepted.
            if (wall)
                world.TileAt(target).Wall = true;
            else
                village.BoundaryEdges.Add(edge);
            if (cancel)
            {
                var result = world.Apply(Context(village), new GameAction { Kind = "CancelTransportRoute", TargetId = route.Id });
                Check(result.Success, "Public route cancellation failed: " + result.Error);
            }
            world.Step(3);
            Check(village.Routes.Any(r => r.Id == route.Id) && vessel.RouteId == route.Id && driver.BuildingId == route.Id, "Blocked shipping released its route, vehicle or driver ownership");
            Check(vessel.Position.Equals(reached) && vessel.X == reached.X && vessel.Z == reached.Z && driver.Position.Equals(reached) && driver.X == vessel.X && driver.Z == vessel.Z, "Shipping crossed a blocked " + (wall ? "dock tile" : "boundary edge"));
            Check(route.BlockedReason == (cancel ? "return_route_blocked" : "route_blocked"), "Blocked shipping did not expose its physical route obstruction");
            Cargo(village, vessel, item, true);
            Near(World.Amount(source.Goods, "logs") + World.Amount(destination.Goods, "logs"), 0, "Blocked shipping credited cargo without delivery");
            if (wall)
                world.TileAt(target).Wall = false;
            else
                village.BoundaryEdges.Remove(edge);
            Until(world, () => !village.Routes.Any(r => r.Id == route.Id), "Reopened shipping route did not resume and release its original ownership");
            Check(vessel.Position.Equals(source.Position) && vessel.RouteId == "" && driver.BuildingId == "", "Recovered vessel or driver failed to return and release the same route");
            Cargo(village, vessel, item, false);
            var receiving = cancel ? source : destination;
            if (exact)
                Check(item.LocationId == receiving.Id, "Recovered vessel delivered exact cargo to the wrong owner");
            else
                Near(World.Amount(receiving.Goods, "logs"), 8, "Recovered vessel failed to deliver its held load exactly once");
            world.Step(2);
            Cargo(village, vessel, item, false);
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void HeldCancelledLoad(Village village, TransportRoute route, Cat driver, Stockpile source, Stockpile destination, Vehicle vessel, Item item)
        {
            Check(village.Routes.SingleOrDefault(r => r.Id == route.Id) == route && route.CancelRequested && !route.Repeat && route.Phase == "returning_to_dock", "Cancelled shipping replaced or prematurely released its original route");
            Check(route.VehicleId == vessel.Id && route.CatId == driver.Id && route.SourceId == source.Id && route.DestinationId == destination.Id && vessel.RouteId == route.Id && driver.BuildingId == route.Id, "Cancelled shipping changed its driver, vessel or endpoint ownership");
            Cargo(village, vessel, item, true);
            Near(World.Amount(source.Goods, "logs") + World.Amount(destination.Goods, "logs"), 0, "Cancelled shipping credited a dock before physical unloading");
        }

        static void ShippingCancelledRecovery(bool exact)
        {
            var world = Fixture(exact, out var village, out var driver, out var source, out var destination, out var vessel, out var item, true);
            var route = village.Routes.Single();
            Until(world, () => route.Phase == "outbound" && route.PathIndex == 1 && vessel.X == vessel.Position.X, "Cancellation fixture did not reach the first loaded water waypoint");
            var reached = vessel.Position;
            world.Step(.25);
            Check(vessel.Position.Equals(reached) && vessel.X > reached.X && vessel.X < reached.X + 1, "Cancellation fixture must interrupt fractional outward sailing");
            // Author physical obstructions after public dispatch; this is synthetic
            // boundary preparation, not a claim about a played fence-removal action.
            var outwardEdge = new BoundaryEdge { From = reached, To = route.Path[2] };
            var returnEdge = new BoundaryEdge { From = reached, To = source.Position };
            village.BoundaryEdges.Add(outwardEdge);
            village.BoundaryEdges.Add(returnEdge);
            world.Step(.05);
            Check(route.BlockedReason == "route_blocked", "Closed outward edge did not establish its real obstruction warning");
            double retreatStart = vessel.X;
            var cancelled = world.Apply(Context(village), new GameAction { Kind = "CancelTransportRoute", TargetId = route.Id });
            Check(cancelled.Success, "Public loaded shipping cancellation failed: " + cancelled.Error);
            world.Step(.05);
            Check(vessel.Position.Equals(reached) && vessel.X > reached.X && vessel.X < retreatStart && driver.X == vessel.X && driver.Z == vessel.Z && route.BlockedReason == "route_blocked", "Unfinished outward-edge retrace cleared its warning or moved the unboarded vessel");
            HeldCancelledLoad(village, route, driver, source, destination, vessel, item);
            village.BoundaryEdges.Remove(outwardEdge);
            world.Step(3);
            Check(vessel.Position.Equals(reached) && vessel.X == reached.X && vessel.Z == reached.Z && driver.Position.Equals(reached) && driver.X == vessel.X && driver.Z == vessel.Z && route.BlockedReason == "return_route_blocked", "Genuinely closed return edge failed to retain its warning at the reached tile");
            HeldCancelledLoad(village, route, driver, source, destination, vessel, item);

            village.BoundaryEdges.Remove(returnEdge);
            world.Step(.05);
            Check(vessel.Position.Equals(reached) && vessel.X < reached.X && vessel.X > source.Position.X && driver.Position.Equals(vessel.Position) && driver.X == vessel.X && driver.Z == vessel.Z, "Reopened cancellation did not physically resume with its original loaded vessel and driver");
            HeldCancelledLoad(village, route, driver, source, destination, vessel, item);
            Check(route.BlockedReason == "", "Physically resumed loaded cancellation retained its obsolete return-route warning");

            world.Step(.2);
            double resumedX = vessel.X;
            village.BoundaryEdges.Add(returnEdge);
            world.Step(.05);
            Check(vessel.Position.Equals(reached) && vessel.X > resumedX && vessel.X < reached.X && driver.X == vessel.X && driver.Z == vessel.Z && route.BlockedReason == "return_route_blocked", "Closing the recovered edge did not restore the warning during unfinished physical retrace");
            HeldCancelledLoad(village, route, driver, source, destination, vessel, item);
            village.BoundaryEdges.Remove(returnEdge);
            double blockedX = vessel.X;
            world.Step(.05);
            Check(vessel.X < blockedX && route.BlockedReason == "", "Reopened fractional return did not clear its warning when physical sailing resumed again");
            HeldCancelledLoad(village, route, driver, source, destination, vessel, item);

            source.Capacity = 0;
            Check(!world.HasRoom(village, source, route.Resource, exact ? 1 : 8), "Full return-storage fixture must refuse the finite held load");
            Until(world, () => route.PathIndex == 0 && vessel.Position.Equals(source.Position) && vessel.X == source.Position.X, "Recovered cancellation failed to physically reach its home dock");
            world.Step(.2);
            Check(route.BlockedReason == "return_storage_full" && driver.Position.Equals(source.Position) && driver.X == vessel.X && driver.Z == vessel.Z, "Full home storage lost its distinct return warning or unloaded without space");
            HeldCancelledLoad(village, route, driver, source, destination, vessel, item);
            source.Capacity = 8;
            Until(world, () => !village.Routes.Any(r => r.Id == route.Id), "Freed home storage did not unload and release the same cancelled route");
            Check(vessel.Position.Equals(source.Position) && vessel.RouteId == "" && driver.BuildingId == "", "Recovered cancellation failed to release its original vessel and driver at home");
            Cargo(village, vessel, item, false);
            if (exact)
                Check(item.LocationId == source.Id, "Cancelled shipping returned its exact item to the wrong owner");
            else
                Near(World.Amount(source.Goods, "logs"), 8, "Cancelled shipping did not return its finite load exactly once");
            Near(World.Amount(destination.Goods, "logs"), 0, "Cancelled shipping credited its abandoned destination");
            world.Step(2);
            Cargo(village, vessel, item, false);
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void ShippingCancelledReboarding(bool exact)
        {
            var world = Fixture(exact, out var village, out var driver, out var source, out var destination, out var vessel, out var item);
            var route = village.Routes.Single();
            Until(world, () => route.Phase == "outbound" && route.PathIndex == 1 && vessel.X == vessel.Position.X, "Reboarding fixture did not reach the first loaded water waypoint");
            world.Step(.25);
            var cancelled = world.Apply(Context(village), new GameAction { Kind = "CancelTransportRoute", TargetId = route.Id });
            Check(cancelled.Success, "Public reboarding-fixture cancellation failed: " + cancelled.Error);
            // Prepare a disembarked driver at the reached tile. The deep next
            // water tile prevents a physical walk back to the fractional vessel.
            driver.X = driver.Position.X;
            driver.Z = driver.Position.Z;
            double vesselX = vessel.X, vesselZ = vessel.Z;
            world.Step(.2);
            Check(route.BlockedReason == "blocked_reboarding_route" && vessel.X == vesselX && vessel.Z == vesselZ && driver.X == driver.Position.X && driver.Z == driver.Position.Z, "A cancelled vessel moved or cleared feedback while its driver could not physically reboard");
            HeldCancelledLoad(village, route, driver, source, destination, vessel, item);
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void ShippingDepth(bool exact)
        {
            var world = Fixture(exact, out var village, out var driver, out var source, out var destination, out var vessel, out var item);
            Check(Math.Abs(world.WalkHeight(new Int2(2, 5)) - world.WalkHeight(new Int2(3, 5))) > .65, "Depth fixture must cross an unwalkable lake-bed slope");
            Until(world, () => village.Routes.Count == 0, "Varying water depth blocked a valid surface shipping route");
            Check(vessel.Position.Equals(source.Position) && vessel.RouteId == "" && driver.BuildingId == "", "Valid depth route did not return its vessel and driver");
            Cargo(village, vessel, item, false);
            if (exact)
                Check(item.LocationId == destination.Id, "Valid depth route lost exact cargo");
            else
                Near(World.Amount(destination.Goods, "logs"), 8, "Valid depth route did not deliver its finite cargo");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }
    }
}
