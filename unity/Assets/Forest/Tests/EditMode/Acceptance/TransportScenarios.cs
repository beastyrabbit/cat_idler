using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>Transport: rail, shipping, caravans and transport capabilities.</summary>
    public static class TransportScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("capability.rail_public_construction", RailCapability);
            yield return new Scenario("capability.shipping_public_construction", ShippingCapability);
            yield return new Scenario("regression.rail_transfers_exact_equipment_identity", EquipmentRail);
            yield return new Scenario("regression.review_rail_stops_at_expanded_wall_and_recovers", RailExpandedWall);
            yield return new Scenario("regression.review_transport_rail_drink_reboards_physically", () => TransportNeed(false));
            yield return new Scenario("regression.review_transport_rail_drink_blocked_reboarding", () => TransportNeed(false, true));
            yield return new Scenario("regression.review_transport_shipping_sleep_docks_and_reboards", () => TransportNeed(true));
            yield return new Scenario("regression.review_transport_caravan_expansion_fence_and_recovery", CaravanExpansionFence);
            foreach (string obstruction in new[] { "water", "boundary", "exact_boundary", "return_boundary" })
            {
                string barrier = obstruction;
                yield return new Scenario("regression.review_rail_passability_" + barrier, () => RailPassability(barrier));
            }
            yield return new Scenario("regression.shipping_cancel_returns_loaded_cargo", () => ShippingCancel(false));
            yield return new Scenario("regression.shipping_cancel_full_source_retains_cargo", () => ShippingCancel(true));
            yield return new Scenario("regression.shipping_driver_death_bridge_salvage", () => ShippingCancel(false, true));
        }
        static void RailCapability()
        {
            var w = StudyFixture("rail", false, out var v, out var c);
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "metal", 100);
            var path = new GameAction { Kind = "DesignateRail", Position = new Int2(-6, 1), End = new Int2(-4, 1), CatId = c.Id };
            Check(!w.Apply(Context(v), path).Success, "Rail construction works without blueprint");
            Act(w, v, new GameAction { Kind = "ResearchNode", NodeId = "rail" });
            Act(w, v, path);
            w.Step(200);
            Check(Enumerable.Range(-6, 3).All(x => w.TileAt(new Int2(x, 1)).Rail), "Researched physical track did not finish");
            Act(w, v, new GameAction { Kind = "BuildTransportVehicle", Mode = "rail", Position = new Int2(-6, 1), CatId = c.Id });
            w.Step(180);
            Check(v.Vehicles.Any(x => x.Mode == "rail"), "Rail vehicle remains unreachable");
            Valid(w);
        }
        static void ShippingCapability()
        {
            var study = Catalog.Research.Single(n => n.Payloads.Any(p => p.Kind == "unlock_capability" && p.Id == "water_travel"));
            var w = StudyFixture(study.Id, false, out var v, out var c);
            Exterior(w, v);
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "lumber", 100);
            w.TileAt(new Int2(8, 9)).Water = true;
            var dock = new GameAction { Kind = "BuildDock", Position = new Int2(8, 8), CatId = c.Id };
            Check(!w.Apply(Context(v), dock).Success, "Dock construction works without Shipping");
            Act(w, v, new GameAction { Kind = "ResearchNode", NodeId = study.Id });
            Act(w, v, dock);
            w.Step(200);
            Check(w.TileAt(new Int2(8, 8)).Dock, "Shipping dock failed physical build");
            Act(w, v, new GameAction { Kind = "BuildTransportVehicle", Mode = "shipping", Position = new Int2(8, 8), CatId = c.Id });
            w.Step(200);
            Check(v.Vehicles.Any(x => x.Mode == "shipping"), "Shipping vessel unreachable");
            Valid(w);
        }
        static void EquipmentRail()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            var source = v.Stockpiles[0];
            source.Position = new Int2(1, 2);
            var destination = new Stockpile { Id = w.Id("destination"), Position = new Int2(3, 2), Accepts = new List<string> { "tools" } };
            v.Stockpiles.Add(destination);
            var path = new List<Int2> { new Int2(1, 2), new Int2(2, 2), new Int2(3, 2) };
            foreach (var p in path)
                w.TileAt(p).Rail = true;
            v.Vehicles.Add(new Vehicle { Id = w.Id("wagon"), Mode = "rail", Position = source.Position });
            var tool = new Item { Id = w.Id("tool"), Kind = "tool", Material = "metal", Quality = 3, Condition = 72, MaxCondition = 125, VillageId = v.Id, LocationId = source.Id };
            v.Items.Add(tool);
            Act(w, v, new GameAction { Kind = "CreateTransportRoute", CatId = c.Id, TargetId = source.Id, BuildingId = destination.Id, Mode = "rail", Resource = "tools", Amount = 1, Path = path });
            w.Step(30);
            Check(v.Items.Count == 1 && tool.LocationId == destination.Id, "Rail route failed to transport exact equipment");
            Near(tool.Condition, 72, "Rail transfer changed exact item condition");
            Near(World.Amount(source.Goods, "tools") + World.Amount(destination.Goods, "tools"), 0, "Rail invented scalar equipment copies");
            Valid(w);
        }
        static void RailExpandedWall()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            World.Add(v.Stockpiles[0].Goods, "materials", 1000);
            World.Add(v.Stockpiles[0].Goods, "metal", 11);
            var helper = v.Cats[1];
            helper.ControlledBy = "";
            helper.BuildingId = "fixture-held";
            helper.Position = new Int2(10, 0);
            helper.X = 10;
            helper.Z = 0;
            helper.Cargo.Add(new Stack("logs", 8));
            for (int x = -13; x <= 13; x++)
                for (int z = -13; z <= 13; z++)
                {
                    var p = new Int2(x, z);
                    var t = w.TileAt(p);
                    if (Math.Abs(x) == 11 || Math.Abs(z) == 11 || Math.Abs(x) == 13 || Math.Abs(z) == 13 || x >= 10 && z == 1)
                        t.Wall = t.Water = t.Mountain = false;
                    if (x >= 10 && z == 1)
                        t.Road = t.Rail = false;
                    if (!v.Known.Contains(p))
                        v.Known.Add(p);
                }
            var sourceAt = new Int2(10, 1);
            var destinationAt = new Int2(12, 1);
            var wall = w.TileAt(new Int2(11, 1));
            var sourceId = Act(w, v, new GameAction { Kind = "DesignateGatherSpot", Resource = "logs", Position = sourceAt, End = sourceAt }).EntityId;
            var destinationId = Act(w, v, new GameAction { Kind = "DesignateGatherSpot", Resource = "logs", Position = destinationAt, End = destinationAt }).EntityId;
            var source = v.Stockpiles.Single(p => p.Id == sourceId);
            var destination = v.Stockpiles.Single(p => p.Id == destinationId);
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = helper.Id });
            Act(w, v, new GameAction { Kind = "InteractCat", CatId = helper.Id, TargetId = sourceId });
            Act(w, v, new GameAction { Kind = "LeaveCat", CatId = helper.Id });
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = helper.Id, BuildingId = v.Buildings.First(b => b.Kind == "den").Id });
            void Complete(GameAction action, int limit)
            {
                var id = Act(w, v, action).EntityId;
                var job = v.Jobs.Single(j => j.Id == id);
                for (int tick = 0; tick < limit && !job.Completed; tick++)
                    w.Step(1);
                Check(job.Completed, "Public rail fixture work did not complete: " + action.Kind);
            }
            Complete(new GameAction { Kind = "DesignateRail", CatId = c.Id, Position = sourceAt, End = destinationAt }, 1000);
            Complete(new GameAction { Kind = "BuildTransportVehicle", CatId = c.Id, Position = sourceAt, Mode = "rail" }, 1000);
            Complete(new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id }, 12000);
            Check(v.Radius == 11 && wall.Rail && wall.Wall, "Public expansion did not retain completed track beneath its new wall");
            var path = new List<Int2> { sourceAt, wall.Position, destinationAt };
            var vehicle = v.Vehicles.Single();
            string vehicleId = vehicle.Id, driverId = c.Id;
            var routeId = Act(w, v, new GameAction { Kind = "CreateTransportRoute", CatId = c.Id, TargetId = sourceId, BuildingId = destinationId, Mode = "rail", Resource = "logs", Amount = 8, Path = path }).EntityId;
            var route = v.Routes.Single(r => r.Id == routeId);
            for (int tick = 0; tick < 1000 && route.Phase != "outbound"; tick++)
                w.Step(1);
            Check(route.Phase == "outbound" && World.Amount(vehicle.Cargo, "logs") == 8 && vehicle.Position.Equals(sourceAt), "Route did not board and load the physical wagon");
            w.Step(3);
            Check(route.BlockedReason == "route_blocked" && route.PathIndex == 0 && vehicle.Position.Equals(sourceAt) && c.Position.Equals(sourceAt), "Loaded rail vehicle or driver crossed the completed expansion wall");
            Check(v.Routes.Single().Id == routeId && vehicle.RouteId == routeId && c.BuildingId == routeId && route.VehicleId == vehicleId && route.CatId == driverId, "Blocked rail lost vehicle or driver ownership");
            Near(World.Amount(vehicle.Cargo, "logs"), 8, "Blocked wagon lost cargo");
            Near(World.Amount(destination.Goods, "logs"), 0, "Blocked wagon delivered through the wall");
            Near(Goods(w, v, "logs"), 8, "Blocked rail duplicated or consumed logs");
            // The next public expansion removes the obsolete interior wall; no save or terrain reset is needed.
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = helper.Id });
            Complete(new GameAction { Kind = "RequestJob", Name = "expand", CatId = helper.Id }, 12000);
            for (int tick = 0; tick < 30 && v.Routes.Any(r => r.Id == routeId); tick++)
                w.Step(1);
            Check(v.Radius == 13 && !wall.Wall && wall.Rail && v.Vehicles.Single().Id == vehicleId && vehicle.Position.Equals(sourceAt), "Rail did not recover on the same physical wagon after expansion removed the wall");
            Check(!v.Routes.Any(r => r.Id == routeId) && vehicle.RouteId == "" && c.BuildingId == "" && vehicle.Cargo.Count == 0, "Recovered rail retained cargo or route ownership");
            Near(World.Amount(destination.Goods, "logs"), 8, "Recovered wagon did not deliver its held cargo");
            Near(Goods(w, v, "logs"), 8, "Rail recovery lost or duplicated cargo");
            Valid(w);
        }
        static void RailPassability(string obstruction)
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            var source = v.Stockpiles[0];
            source.Position = new Int2(-6, 1);
            var destination = new Stockpile { Id = w.Id("rail-destination"), Position = new Int2(-3, 1), Width = 1, Depth = 1 };
            v.Stockpiles.Add(destination);
            var path = Enumerable.Range(-6, 4).Select(x => new Int2(x, 1)).ToList();
            foreach (var p in path)
            {
                var t = w.TileAt(p);
                t.Wall = t.Water = t.Mountain = t.Road = t.Bridge = false;
                t.Rail = true;
            }
            var vehicle = new Vehicle { Id = w.Id("wagon"), Mode = "rail", Position = source.Position };
            v.Vehicles.Add(vehicle);
            bool exact = obstruction == "exact_boundary", returning = obstruction == "return_boundary";
            var item = new Item { Id = w.Id("tool"), Kind = "tool", Material = "metal", Quality = 3, Condition = 57, MaxCondition = 88, VillageId = v.Id, LocationId = source.Id };
            if (exact)
                v.Items.Add(item);
            else
                World.Add(source.Goods, "logs", 8);
            var id = Act(w, v, new GameAction { Kind = "CreateTransportRoute", CatId = c.Id, TargetId = source.Id, BuildingId = destination.Id, Mode = "rail", Resource = exact ? "tools" : "logs", Amount = exact ? 1 : 8, Path = path }).EntityId;
            var route = v.Routes.Single(r => r.Id == id);
            for (int tick = 0; tick < 100 && route.Phase != (returning ? "returning" : "outbound"); tick++)
                w.Step(1);
            Check(route.Phase == (returning ? "returning" : "outbound"), "Rail fixture did not reach its physical interruption stage");
            int index = route.PathIndex;
            var before = vehicle.Position;
            var next = path[index + (returning ? -1 : 1)];
            var tile = w.TileAt(next);
            var edge = new BoundaryEdge { From = before, To = next };
            if (obstruction == "water")
                tile.Water = true;
            else
                v.BoundaryEdges.Add(edge);
            Check(!w.Walkable(v, next) || !w.Crossable(before, next), "Rail fault did not obstruct authoritative movement");
            w.Step(3);
            Check(route.BlockedReason == "route_blocked" && route.PathIndex == index && vehicle.Position.Equals(before) && c.Position.Equals(before) && c.X == before.X && c.Z == before.Z, "Rail bypassed authoritative " + obstruction + " passability");
            Check(v.Routes.Single().Id == id && vehicle.RouteId == id && c.BuildingId == id && route.VehicleId == vehicle.Id && route.CatId == c.Id, "Blocked rail changed route, wagon or driver identity");
            if (exact)
                Check(v.Items.Count == 1 && item.LocationId == vehicle.Id && vehicle.ItemIds.SequenceEqual(new[] { item.Id }) && item.Condition == 57 && item.MaxCondition == 88 && item.Quality == 3, "Blocked rail changed exact cargo identity or condition");
            else
            {
                Near(World.Amount(vehicle.Cargo, "logs"), returning ? 0 : 8, "Blocked rail changed held cargo");
                Near(World.Amount(destination.Goods, "logs"), returning ? 8 : 0, "Blocked rail delivered across the obstruction");
                Near(Goods(w, v, "logs"), 8, "Blocked rail lost or duplicated cargo");
            }
            // Remove only the injected obstruction; the saved route, wagon and cargo are reused.
            if (obstruction == "water")
                tile.Water = false;
            else
                v.BoundaryEdges.Remove(edge);
            for (int tick = 0; tick < 30 && v.Routes.Any(r => r.Id == id); tick++)
                w.Step(1);
            Check(!v.Routes.Any(r => r.Id == id) && vehicle.RouteId == "" && vehicle.Position.Equals(source.Position) && c.BuildingId == "" && vehicle.Cargo.Count == 0 && vehicle.ItemIds.Count == 0, "Cleared rail obstruction did not resume delivery and release the same route");
            if (exact)
                Check(v.Items.Count == 1 && item.LocationId == destination.Id && item.Condition == 57 && item.MaxCondition == 88 && item.Quality == 3, "Recovered rail changed exact delivered cargo");
            else
                Near(World.Amount(destination.Goods, "logs"), 8, "Recovered rail did not deliver its conserved cargo");
            Valid(w);
        }
        static void CaravanExpansionFence()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            World.Add(v.Stockpiles[0].Goods, "materials", 2000);
            World.Add(v.Stockpiles[0].Goods, "planks", 10);
            World.Add(v.Stockpiles[0].Goods, "blocks", 5);
            World.Add(v.Stockpiles[0].Goods, "logs", 8);
            for (int x = -13; x <= 22; x++)
                for (int z = -13; z <= 13; z++)
                {
                    var p = new Int2(x, z);
                    var tile = w.TileAt(p);
                    if (Math.Abs(x) == 11 || Math.Abs(z) == 11 || Math.Abs(x) == 13 || Math.Abs(z) == 13 || x >= 10)
                        tile.Wall = tile.Water = tile.Mountain = false;
                    if (x >= 10)
                    {
                        tile.Road = false;
                        tile.ClaimId = "";
                    }
                    if (!v.Known.Contains(p))
                        v.Known.Add(p);
                }
            w.TileAt(new Int2(10, 0)).Road = true;
            // Preexisting fence geometry selects a detour that both public expansions keep dry.
            v.BoundaryEdges.Add(new BoundaryEdge { From = new Int2(11, 0), To = new Int2(12, 0) });
            v.BoundaryEdges.Add(new BoundaryEdge { From = new Int2(12, 1), To = new Int2(13, 1) });
            var other = new Village { Id = w.Id("trade-village"), Name = "Fence trade partner", OwnerId = Context(v).PlayerId, Center = new Int2(20, 0), Radius = 1, LastLeaderResearch = 0 };
            w.Villages.Add(other);
            var den = new Building { Id = w.Id("trade-den"), Kind = "den", Position = new Int2(20, -2), Completed = true };
            other.Buildings.Add(den);
            other.Buildings.Add(new Building { Id = w.Id("trade-shrine"), Kind = "shrine", Position = other.Center, Completed = true });
            var resident = new Cat { Id = w.Id("trade-resident"), VillageId = other.Id, BedId = den.Id, Position = other.Center, X = 20, Z = 0, AgeHours = 24, ControlledBy = "fixture-held", ControlLeaseUntil = 10000000 };
            other.Cats.Add(resident);
            other.LeaderId = resident.Id;
            var receiver = new Stockpile { Id = w.Id("trade-store"), Position = new Int2(20, 2), Capacity = 1000, Goods = new List<Stack> { new Stack("stone", 5) } };
            other.Stockpiles.Add(receiver);
            v.Contacts.Add(other.Id);
            other.Contacts.Add(v.Id);
            void Complete(GameAction action)
            {
                var id = Act(w, v, action).EntityId;
                var job = v.Jobs.Single(j => j.Id == id);
                for (int tick = 0; tick < 1000 && !job.Completed; tick++)
                    w.Step(1);
                Check(job.Completed, "Public caravan fixture work did not complete: " + action.Kind);
            }
            Complete(new GameAction { Kind = "BuildRoad", Position = new Int2(11, 0), End = new Int2(15, 0), CatId = c.Id });
            Complete(new GameAction { Kind = "BuildRoad", Position = new Int2(11, 1), End = new Int2(12, 1), CatId = c.Id });
            var fieldId = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "field", Position = new Int2(15, -2), CatId = c.Id }).EntityId;
            var field = v.Buildings.Single(b => b.Id == fieldId);
            for (int tick = 0; tick < 1000 && !field.Completed; tick++)
                w.Step(1);
            Check(field.Completed, "Public caravan Field did not complete");
            var farmId = Act(w, v, new GameAction { Kind = "DesignateFarm", Resource = "grain", Position = new Int2(10, 1), End = new Int2(10, 3) }).EntityId;
            var expansionId = Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id }).EntityId;
            var expansion = v.Jobs.Single(j => j.Id == expansionId);
            for (int tick = 0; tick < 12000 / World.SimulationStepSeconds && expansion.PathIndex < expansion.Path.Count; tick++)
                w.Step(World.SimulationStepSeconds);
            var from = new Int2(11, 0);
            var to = new Int2(11, 1);
            Check(!expansion.Completed && expansion.PathIndex == expansion.Path.Count && w.Crossable(from, to), "Caravan was not accepted immediately before public farm-fence completion");
            var tradeId = Act(w, v, new GameAction { Kind = "OfferVillageTrade", OtherVillageId = other.Id, Resource = "logs", Amount = 8, OtherResource = "stone", OtherAmount = 5 }).EntityId;
            Act(w, other, new GameAction { Kind = "AcceptVillageTrade", TargetId = tradeId });
            var trade = w.TradeOffers.Single(t => t.Id == tradeId);
            int crossing = trade.Path.FindIndex(p => p.Equals(from));
            Check(crossing >= 0 && crossing + 1 < trade.Path.Count && trade.Path[crossing + 1].Equals(to), "Public accepted caravan did not cache the future farm-fence edge");
            w.Step(1);
            Check(expansion.Completed && v.Radius == 11 && w.Walkable(from) && w.Walkable(to) && !w.Crossable(from, to), "Public expansion did not create the walkable-tile fence");
            for (int tick = 0; tick < 100 && !(trade.X == from.X && trade.Z == from.Z); tick++)
                w.Step(1);
            Check(trade.X == from.X && trade.Z == from.Z && trade.PathIndex == crossing + 1, "Caravan did not reach the new fence physically");
            double progress = trade.Progress;
            int index = trade.PathIndex;
            w.Step(3);
            Check(trade.Status == "outbound" && trade.PathIndex == index && trade.X == from.X && trade.Z == from.Z && trade.Progress == progress, "Cached caravan path crossed the completed farm boundary");
            Near(Goods(w, v, "logs") + Goods(w, other, "logs"), 0, "Blocked caravan released its offered escrow");
            Near(Goods(w, v, "stone") + Goods(w, other, "stone"), 0, "Blocked caravan released its payment escrow");
            Near(trade.Offered.Amount, 8, "Blocked trade changed offered cargo");
            Near(trade.Requested.Amount, 5, "Blocked trade changed payment cargo");
            Check(!trade.OfferedDelivered, "Blocked caravan credited an outward delivery");
            Act(w, v, new GameAction { Kind = "ClearFarm", TargetId = farmId });
            expansionId = Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id }).EntityId;
            expansion = v.Jobs.Single(j => j.Id == expansionId);
            for (int tick = 0; tick < 12000 && !expansion.Completed; tick++)
                w.Step(1);
            Check(expansion.Completed && v.Radius == 13 && w.Crossable(from, to), "Public expansion did not reopen the obsolete farm fence");
            for (int tick = 0; tick < 500 && trade.Status != "completed"; tick++)
                w.Step(1);
            Check(trade.Status == "completed" && trade.OfferedDelivered, "Reopened caravan did not resume its original exchange");
            w.Step(10);
            Near(Goods(w, other, "logs"), 8, "Resumed caravan did not deliver offered goods exactly once");
            Near(Goods(w, v, "stone"), 5, "Resumed caravan did not deliver payment exactly once");
            Near(Goods(w, v, "logs"), 0, "Resumed caravan returned its spent offer");
            Near(Goods(w, other, "stone"), 0, "Resumed caravan duplicated payment");
            Check(w.TradeOffers.Count(t => t.Id == tradeId) == 1, "Caravan recovery replaced its exchange identity");
            Valid(w);
        }
        static void TransportNeed(bool shipping, bool blockReturn = false)
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            string mode = shipping ? "shipping" : "rail";
            var path = Enumerable.Range(shipping ? 1 : -6, 6).Select(x => new Int2(x, shipping ? 5 : 1)).ToList();
            foreach (var p in path)
            {
                var tile = w.TileAt(p);
                tile.Wall = tile.Mountain = tile.Road = tile.Dirt = tile.Bridge = false;
                tile.Water = shipping && !p.Equals(path[0]) && !p.Equals(path[path.Count - 1]);
                tile.Dock = shipping && !tile.Water;
                tile.Rail = !shipping;
            }
            if (shipping)
                for (int x = 2; x <= 5; x++)
                    foreach (int z in new[] { 4, 6 })
                    {
                        var tile = w.TileAt(new Int2(x, z));
                        tile.Wall = tile.Mountain = tile.Bridge = false;
                        tile.Water = true;
                    }
            var source = new Stockpile { Id = w.Id("need-source"), Position = path[0], Width = 1, Depth = 1, Goods = new List<Stack> { new Stack("logs", 8) } };
            var destination = new Stockpile { Id = w.Id("need-destination"), Position = path[path.Count - 1], Width = 1, Depth = 1 };
            v.Stockpiles.Add(source);
            v.Stockpiles.Add(destination);
            var vehicle = new Vehicle { Id = w.Id("need-vehicle"), Mode = mode, Position = source.Position };
            v.Vehicles.Add(vehicle);
            var id = Act(w, v, new GameAction { Kind = "CreateTransportRoute", CatId = c.Id, TargetId = source.Id, BuildingId = destination.Id, Mode = mode, Resource = "logs", Amount = 8, Path = path }).EntityId;
            var route = v.Routes.Single(r => r.Id == id);
            for (int tick = 0; tick < 100 && !(route.Phase == "outbound" && route.PathIndex == 2); tick++)
                w.Step(1);
            Check(route.Phase == "outbound" && route.PathIndex == 2 && c.Position.Equals(vehicle.Position) && World.Amount(vehicle.Cargo, "logs") == 8, "Needs fixture did not load and physically depart");
            if (shipping)
                c.Rest = 19;
            else
                c.Thirst = 34;
            const double dt = World.SimulationStepSeconds;
            bool Aboard() => c.Position.Equals(vehicle.Position) && Math.Abs(c.X - vehicle.X) < 1e-9 && Math.Abs(c.Z - vehicle.Z) < 1e-9;
            double water = Goods(w, v, "water"), age = c.AgeHours, health = c.Health;
            var parked = vehicle.Position;
            int parkedIndex = route.PathIndex;
            bool departed = false, satisfied = false, reboarded = false;
            var edges = new List<BoundaryEdge>();
            for (int tick = 0; tick < (shipping ? 900 : 150) / dt && !satisfied; tick++)
            {
                var before = vehicle.Position;
                double x = c.X, z = c.Z, rest = c.Rest, hunger = c.Hunger, thirst = c.Thirst, offshoreAge = c.AgeHours, offshoreHealth = c.Health, vehicleX = vehicle.X, vehicleZ = vehicle.Z;
                bool aboard = Aboard();
                w.Step(dt);
                Check(Math.Sqrt((c.X - x) * (c.X - x) + (c.Z - z) * (c.Z - z)) <= 2.80001 * dt, "Need interruption teleported the transport driver");
                if (!aboard)
                    Check(vehicle.Position.Equals(before) && Math.Abs(vehicle.X - vehicleX) + Math.Abs(vehicle.Z - vehicleZ) < 1e-9, "Vehicle moved before its driver physically reboarded");
                if (shipping && !w.Walkable(v, before))
                {
                    Check(c.Position.Equals(vehicle.Position) && c.Rest <= rest, "Offshore sleep left the vessel or restored rest without reaching its bed");
                    Check(c.Hunger < hunger && c.Thirst < thirst && c.AgeHours > offshoreAge && c.Health <= offshoreHealth, "Offshore need deferral stopped aging or restored needs without physical service");
                    Check(Math.Abs(vehicle.X - vehicleX) + Math.Abs(vehicle.Z - vehicleZ) > 1e-9, "Sleep need stranded the loaded vessel offshore");
                }
                if (!c.Position.Equals(vehicle.Position))
                {
                    departed = true;
                    if (blockReturn && edges.Count == 0)
                        foreach (var adjacent in new[] { new Int2(parked.X - 1, parked.Z), new Int2(parked.X + 1, parked.Z), new Int2(parked.X, parked.Z - 1), new Int2(parked.X, parked.Z + 1) })
                        {
                            var edge = new BoundaryEdge { From = parked, To = adjacent };
                            edges.Add(edge);
                            v.BoundaryEdges.Add(edge);
                        }
                }
                Check(v.Routes.Single().Id == id && vehicle.RouteId == id && c.BuildingId == id && route.CatId == c.Id && route.VehicleId == vehicle.Id, "Need interruption lost transport identities or ownership");
                Near(World.Amount(vehicle.Cargo, "logs"), 8, "Need interruption released loaded cargo before reboarding");
                Near(World.Amount(destination.Goods, "logs"), 0, "Needs delivered cargo without its driver");
                Near(Goods(w, v, "logs"), 8, "Need interruption lost or duplicated cargo");
                satisfied = shipping ? c.Rest >= 95 : c.Thirst >= 80;
            }
            Check(departed && satisfied && c.AgeHours > age && c.Health <= health, "Transport need never reached and used its physical supplies or bed");
            if (!shipping)
            {
                Near(Goods(w, v, "water"), water - 1, "Rail drink did not consume exactly one finite serving");
                if (blockReturn)
                {
                    w.Step(5);
                    Check(vehicle.Position.Equals(parked) && route.PathIndex == parkedIndex && !c.Position.Equals(parked) && c.BlockedReason == "blocked_route", "New boundary did not block the driver's physical return to its wagon");
                    foreach (var edge in edges)
                        v.BoundaryEdges.Remove(edge);
                }
            }
            for (int tick = 0; tick < 100 / dt && v.Routes.Any(r => r.Id == id); tick++)
            {
                var before = vehicle.Position;
                bool aboard = Aboard();
                double x = c.X, z = c.Z, vehicleX = vehicle.X, vehicleZ = vehicle.Z;
                w.Step(dt);
                Check(Math.Sqrt((c.X - x) * (c.X - x) + (c.Z - z) * (c.Z - z)) <= 2.80001 * dt, "Reboarding teleported the transport driver");
                if (!aboard)
                    Check(vehicle.Position.Equals(before) && Math.Abs(vehicle.X - vehicleX) + Math.Abs(vehicle.Z - vehicleZ) < 1e-9, "Transport resumed before its driver's return completed");
                if (departed && Aboard())
                    reboarded = true;
            }
            Check(reboarded && !v.Routes.Any(r => r.Id == id) && vehicle.RouteId == "" && c.BuildingId == "" && vehicle.Position.Equals(source.Position) && vehicle.Cargo.Count == 0, "Satisfied driver did not reboard, finish and release the same transport");
            Near(World.Amount(destination.Goods, "logs"), 8, "Resumed needs route did not deliver exactly once");
            Near(Goods(w, v, "logs"), 8, "Resumed needs route changed cargo totals");
            Valid(w);
        }
        static void ShippingCancel(bool fullSource, bool driverDeath = false)
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            v.Stockpiles[0].Position = new Int2(1, 0);
            if (driverDeath)
                World.Add(v.Stockpiles[0].Goods, "lumber", 4);
            var source = new Stockpile { Id = w.Id("port"), Position = new Int2(1, 2), Capacity = 8, Goods = new List<Stack> { new Stack("logs", 8) } };
            var destination = new Stockpile { Id = w.Id("port"), Position = new Int2(6, 2), Capacity = 8 };
            v.Stockpiles.Add(source);
            v.Stockpiles.Add(destination);
            var path = Enumerable.Range(1, 6).Select(x => new Int2(x, 2)).ToList();
            foreach (var p in path)
            {
                var tile = w.TileAt(p);
                tile.Wall = tile.Mountain = false;
                tile.Water = p.X > 1 && p.X < 6;
                tile.Dock = !tile.Water;
            }
            var vehicle = new Vehicle { Id = w.Id("vessel"), Mode = "shipping", Position = source.Position };
            v.Vehicles.Add(vehicle);
            var helper = v.Cats[1];
            helper.ControlledBy = "";
            helper.BuildingId = driverDeath ? "" : "fixture-held";
            helper.Position = new Int2(1, 1);
            helper.X = 1;
            helper.Z = 1;
            if (fullSource)
                helper.Cargo.Add(new Stack("stone", 8));
            Act(w, v, new GameAction { Kind = "CreateTransportRoute", CatId = c.Id, TargetId = source.Id, BuildingId = destination.Id, Mode = "shipping", Resource = "logs", Amount = 8, Path = path, Repeat = true });
            w.Step(3);
            Check(w.TileAt(vehicle.Position).Water && World.Amount(vehicle.Cargo, "logs") == 8, "Shipping fixture never loaded and departed over water");
            if (fullSource)
            {
                Act(w, v, new GameAction { Kind = "EnterCat", CatId = helper.Id });
                Act(w, v, new GameAction { Kind = "InteractCat", CatId = helper.Id, TargetId = source.Id });
            }
            var position = vehicle.Position;
            var route = v.Routes.Single();
            if (driverDeath)
            {
                c.Health = 0;
                w.Step(1);
                Check(!c.Alive, "Driver death did not pass through simulation lifecycle");
            }
            else
                Act(w, v, new GameAction { Kind = "CancelTransportRoute", TargetId = route.Id });
            Check(vehicle.Position.Equals(position) && World.Amount(vehicle.Cargo, "logs") == 8, "Cancelling at sea spilled or teleported loaded cargo");
            if (driverDeath)
            {
                w.Step(20);
                Check(vehicle.Position.Equals(position) && World.Amount(vehicle.Cargo, "logs") == 8, "Crewless vessel must retain cargo at its actual position until physical recovery");
                Act(w, v, new GameAction { Kind = "BuildBridge", CatId = helper.Id, Position = position });
                w.Step(300);
                Check(w.TileAt(position).Bridge, "Physical bridge did not reach the stranded vessel");
                Near(Goods(w, v, "lumber"), 0, "Bridge recovery did not consume its finite lumber cost");
                if (v.Routes.Any(r => r.Id == route.Id))
                    Act(w, v, new GameAction { Kind = "CancelTransportRoute", TargetId = route.Id });
                var salvage = v.Stockpiles.SingleOrDefault(p => p.Kind == "spill" && p.Position.Equals(position) && World.Amount(p.Goods, "logs") == 8);
                Check(salvage != null && w.Path(helper.Position, salvage.Position, v) != null, "Bridge-accessible vessel cargo did not become recoverable salvage");
                Act(w, v, new GameAction { Kind = "HaulGatherSpot", CatId = helper.Id, TargetId = salvage.Id });
                w.Step(120);
                Near(v.Stockpiles.Where(p => p.Kind == "storage").Sum(p => World.Amount(p.Goods, "logs")), 8, "Salvaged vessel cargo failed physical delivery to storage");
                Near(Goods(w, v, "logs"), 8, "Driver death and salvage lost or duplicated cargo");
                Check(vehicle.Cargo.Count == 0 && vehicle.RouteId == "" && !v.Routes.Any(r => r.Id == route.Id), "Salvaged wreck retained route ownership");
                Valid(w);
                return;
            }
            w.Step(20);
            Check(vehicle.Position.Equals(source.Position), "Cancelled vessel did not physically return to its source port");
            Check(!v.Stockpiles.Any(p => p.Kind == "spill" && w.TileAt(p.Position).Water && p.Goods.Count > 0), "Cancelled cargo stranded in a water spill");
            Near(Goods(w, v, "logs"), 8, "Shipping cancellation lost or duplicated cargo");
            if (fullSource)
            {
                Near(World.Amount(vehicle.Cargo, "logs"), 8, "Full source discarded returning vessel cargo");
                Check(v.Routes.Any(r => r.Id == route.Id), "Full source released route ownership before safe unloading");
                Act(w, v, new GameAction { Kind = "InteractCat", CatId = helper.Id, TargetId = source.Id, Resource = "stone", Amount = 8 });
                Act(w, v, new GameAction { Kind = "LeaveCat", CatId = helper.Id });
                w.Step(20);
            }
            Near(World.Amount(source.Goods, "logs"), 8, "Cancelled cargo did not return to source storage");
            Check(vehicle.Cargo.Count == 0 && vehicle.RouteId == "" && c.BuildingId == "" && !v.Routes.Any(r => r.Id == route.Id), "Docked cancellation left cargo or route ownership behind");
            Valid(w);
        }
    }
}
