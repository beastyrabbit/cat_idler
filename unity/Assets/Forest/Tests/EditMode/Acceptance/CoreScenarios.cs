using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>Continuous stepping: movement and work budgets, tick partitions, failed paths and fractional rates.</summary>
    public static class CoreScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            foreach (string mode in new[] { "cadence", "recovery", "destination", "start", "partition" })
            {
                string check = mode;
                yield return new Scenario("regression.continuous_failed_path_" + check, () => ContinuousFailedPath(check));
            }
            yield return new Scenario("regression.continuous_npc_movement_and_needs", ContinuousNpc);
            yield return new Scenario("regression.continuous_stationary_work_rate", ContinuousWork);
            yield return new Scenario("regression.continuous_shared_movement_work_budget", ContinuousBudget);
            yield return new Scenario("regression.continuous_direct_turn_retraces_edge", ContinuousDirectTurn);
            foreach (string ground in new[] { "grass", "road", "forest" })
            {
                string terrain = ground;
                yield return new Scenario("regression.continuous_segment_rate_turn_" + terrain, () => ContinuousTurnRate(terrain));
            }
            yield return new Scenario("regression.continuous_segment_rate_remaining_budget", ContinuousTurnRemainingBudget);
            yield return new Scenario("regression.continuous_segment_rate_fractional_reboard", ContinuousReboardRate);
            yield return new Scenario("regression.continuous_missing_supply_preserves_work_route", ContinuousMissingSupply);
            yield return new Scenario("regression.continuous_fractional_partition", ContinuousPartition);
            foreach (string activity in new[] { "research", "farm", "sleep" })
            {
                string kind = activity;
                yield return new Scenario("regression.continuous_" + kind + "_rate", () => ContinuousRate(kind));
            }
            yield return new Scenario("regression.continuous_combat_shares_cat_budget", ContinuousCombatBudget);
            yield return new Scenario("regression.continuous_rail_cancel_mid_edge", () => ContinuousTransportInterruption(false));
            yield return new Scenario("regression.continuous_shipping_needs_and_cancel_mid_edge", () => ContinuousTransportInterruption(true));
            yield return new Scenario("regression.continuous_legacy_caravan_positive", () => ContinuousLegacyCaravan(1));
            yield return new Scenario("regression.continuous_legacy_caravan_negative", () => ContinuousLegacyCaravan(-1));
            foreach (string mover in new[] { "merchant", "raid", "caravan", "rail" })
            {
                string kind = mover;
                yield return new Scenario("regression.continuous_" + kind, () => ContinuousMover(kind));
            }
        }
        static void ContinuousNpc()
        {
            var w = ContinuousFixture(out var v, out var c, false);
            double x = c.X, hunger = c.Hunger;
            Check(v.Cats.All(cat => cat.ControlledBy == ""), "NPC fixture must not rely on a directly controlled cat");
            w.Step(0.1);
            Near(c.X - x, 0.16, "NPC movement did not advance at its physical speed before one second");
            Near(hunger - c.Hunger, 0.1 * 20 / 3600, "Needs did not advance continuously");
            w.Step(0.9);
            Near(c.X - x, 1.6, "Fixed steps changed one-second movement speed");
            Near(hunger - c.Hunger, 20.0 / 3600, "Fixed steps accelerated needs depletion");
            Valid(w);
        }
        static void ContinuousFailedPath(string mode)
        {
            var w = FailedPathFixture(out var v, out var c);
            var job = v.Jobs.Single();
            long before = PathSearches(w);
            w.Step(0.1);
            Check(c.BlockedReason == "blocked_route" && c.Path.Count == 0 && c.X == 1 && job.Progress == 0, "Failed path advanced travel or work");
            Check(PathSearches(w) - before == 1, "Unchanged failed movement searched again before the one-second planning boundary");
            if (mode == "cadence")
            {
                w.Step(0.85);
                Check(PathSearches(w) - before == 1, "Failed search repeated within the planning interval");
                before = PathSearches(w);
                w.Step(0.05);
                Check(PathSearches(w) - before == 1, "Failed movement did not retry at the next planning boundary");
                before = PathSearches(w);
                w.Step(0.95);
                Check(PathSearches(w) == before, "Second failed search was not cached");
            }
            else if (mode == "recovery")
            {
                v.BoundaryEdges.Clear();
                w.Step(0.85);
                Near(c.X, 1, "Blocked route retried before its planning boundary");
                w.Step(0.05);
                Near(c.X, 1.08, "Reopened route did not resume at the next planning boundary");
                w.Step(2);
                Check(c.Position.Equals(job.Position) && job.Progress > 0, "Recovered route did not resume the same physical job");
            }
            else if (mode == "destination")
            {
                job.Position = new Int2(2, 2);
                w.Step(0.05);
                Near(c.X, 1.08, "A changed destination waited for the previous failure deadline");
                Check(PathSearches(w) - before == 2 && c.BlockedReason == "", "Changed destination did not get an immediate successful search");
            }
            else if (mode == "start")
            {
                // A restored/reassigned actor can have a different reached anchor with the same destination.
                c.Position = new Int2(2, 2);
                c.X = 2;
                w.Step(0.05);
                Check(PathSearches(w) - before == 2 && c.BlockedReason == "blocked_route", "A changed start reused the old anchor's failed search");
            }
            else
            {
                var split = FailedPathFixture(out var otherVillage, out var otherCat);
                w.Step(0.93);
                for (int i = 0; i < 20; i++)
                {
                    split.Step(0.03);
                    split.Step(0.02);
                }
                split.Step(0.03);
                Check(Fingerprint(w) == Fingerprint(split), "Fractional partitions changed failed-route timing or authoritative state");
                v.BoundaryEdges.Clear();
                otherVillage.BoundaryEdges.Clear();
                w.Step(1);
                split.Step(0.97);
                split.Step(0.03);
                Check(Fingerprint(w) == Fingerprint(split) && c.X > 1 && otherCat.X > 1, "Fractional partitions changed blocked-route recovery");
            }
            Valid(w);
        }
        static void ContinuousWork()
        {
            var w = ContinuousFixture(out var v, out var c, true);
            var job = v.Jobs.Single();
            w.Step(0.1);
            Near(job.Progress, 0.1, "Stationary work did not advance before one second");
            w.Step(0.9);
            Near(job.Progress, 1, "Fixed steps accelerated work");
            Valid(w);
        }
        static void ContinuousBudget()
        {
            var w = ContinuousFixture(out var v, out var c, true);
            var job = v.Jobs.Single();
            job.Position = new Int2(2, 2);
            c.X = 1.96;
            c.Path = new List<Int2> { job.Position };
            w.Step(0.05);
            Near(c.X, 2, "Cat did not finish its partial edge");
            Near(job.Progress, 0.025, "Travel and work each received a full actor timestep");
            Valid(w);
        }
        static void ContinuousPartition()
        {
            var whole = ContinuousFixture(out var a, out var ca, true);
            var split = ContinuousFixture(out var b, out var cb, true);
            string paused = Fingerprint(whole);
            whole.Step(0);
            Check(Fingerprint(whole) == paused, "Zero elapsed time changed paused authoritative state");
            whole.Step(1.03);
            for (int i = 0; i < 20; i++)
            {
                split.Step(0.03);
                split.Step(0.02);
            }
            split.Step(0.03);
            Check(Fingerprint(whole) == Fingerprint(split), "Fractional partitions changed authoritative state or random order");
            Near(a.Jobs.Single().Progress, 1, "Only complete fixed quanta may advance behavior");
            Near(whole.TimeSeconds, 1.03, "Requested elapsed time was discarded");
            Near(whole.PendingSeconds, 0.03, "Legacy fractional-second save meaning changed");
        }
        static void ContinuousDirectTurn()
        {
            var w = ContinuousFixture(out var v, out var c, false);
            var tile = w.TileAt(new Int2(1, 3));
            tile.Wall = tile.Water = tile.Mountain = tile.Road = tile.Dirt = false;
            tile.Biome = "grass";
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(1, 0) });
            w.Step(0.05);
            Near(c.X, 1.08, "Direct turn fixture did not enter its first edge");
            Act(w, v, new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(0, 1) });
            w.Step(0.05);
            Near(c.X, 1, "Mid-edge turn did not retrace the old segment");
            Near(c.Z, 2, "Mid-edge turn cut diagonally across grid edges");
            w.Step(0.05);
            Near(c.X, 1, "Second segment left the cardinal route");
            Near(c.Z, 2.08, "Second segment did not continue after physical retreat");
            Valid(w);
        }
        static void ContinuousTurnRate(string terrain)
        {
            var w = ContinuousFixture(out var v, out var c, false);
            v.Research.Add("mounted_scouts");
            var oldSegment = w.TileAt(new Int2(2, 2));
            oldSegment.Road = terrain == "road";
            oldSegment.Biome = terrain == "forest" ? "forest" : "grass";
            var newSegment = w.TileAt(new Int2(1, 3));
            newSegment.Wall = newSegment.Water = newSegment.Mountain = newSegment.Road = newSegment.Dirt = false;
            newSegment.Biome = "grass";
            double speed = 1.6 * 1.3 * (terrain == "road" ? 1.75 : terrain == "forest" ? 0.85 : 1);
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(1, 0) });
            w.Step(0.05);
            Near(c.X, 1 + speed * 0.05, "Forward segment did not use research and " + terrain + " movement rates");
            Act(w, v, new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(0, 1) });
            w.Step(0.05);
            Near(c.X, 1, "Retreat changed the researched " + terrain + " segment speed");
            Near(c.Z, 2, "Retreat granted unearned time on the next segment");
            w.Step(0.05);
            Near(c.X, 1, "Turn cut across the previous segment");
            Near(c.Z, 2 + 1.6 * 1.3 * 0.05, "Turn did not resume at the new segment's own speed");
            Valid(w);
        }
        static void ContinuousTurnRemainingBudget()
        {
            var w = ContinuousFixture(out var v, out var c, false);
            v.Research.Add("mounted_scouts");
            c.X = 1 + 1.6 * 1.3 * 0.025;
            var tile = w.TileAt(new Int2(1, 3));
            tile.Wall = tile.Water = tile.Mountain = tile.Road = tile.Dirt = false;
            tile.Biome = "grass";
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(0, 1) });
            w.Step(0.05);
            Near(c.X, 1, "Partial retreat did not return to its anchor");
            Near(c.Z, 2 + 1.6 * 1.3 * 0.025, "Partial retreat did not leave its actual unused movement time");
            Valid(w);
        }
        static void ContinuousReboardRate()
        {
            var w = ContinuousFixture(out var v, out var c, false);
            v.Research.Add("mounted_scouts");
            c.Path.Clear();
            var start = c.Position;
            var next = new Int2(2, 2);
            var end = new Int2(3, 2);
            w.TileAt(next).Road = true;
            var vehicle = new Vehicle { Id = w.Id("rate-cart"), Mode = "rail", Position = start, HasContinuousPosition = true, X = 1.5, Z = 2, Progress = 0.5, Cargo = new List<Stack> { new Stack("logs", 8) } };
            v.Vehicles.Add(vehicle);
            var route = new TransportRoute { Id = w.Id("rate-route"), Mode = "rail", CatId = c.Id, VehicleId = vehicle.Id, SourceId = v.Stockpiles[0].Id, DestinationId = v.Stockpiles[0].Id, Resource = "logs", Amount = 8, Phase = "outbound", Path = new List<Int2> { start, next, end } };
            v.Routes.Add(route);
            vehicle.RouteId = c.BuildingId = route.Id;
            w.Step(0.05);
            Near(c.X, 1 + 1.6 * 1.3 * 1.75 * 0.05, "Fractional reboarding ignored the researched road movement speed");
            Near(vehicle.X, 1.5, "Vehicle moved before physical reboarding");
            Near(World.Amount(vehicle.Cargo, "logs"), 8, "Reboarding changed finite cargo");
            Valid(w);
        }
        static void ContinuousMissingSupply()
        {
            var w = ContinuousFixture(out var v, out var c, true);
            var job = v.Jobs.Single();
            v.Stockpiles[0].Goods.RemoveAll(s => s.Resource == "water");
            c.Thirst = 1;
            job.Position = new Int2(4, 2);
            job.Phase = "travel";
            c.Path = w.Path(c.Position, job.Position, v);
            c.X = 1.95;
            for (int tick = 0; tick < 40; tick++)
            {
                double x = c.X;
                w.Step(World.SimulationStepSeconds);
                Check(c.X + 1e-9 >= x, "A missing need supply repeatedly reversed the active physical work route");
                Check(c.JobId == job.Id && job.CatId == c.Id, "Missing-supply retry changed active work ownership");
            }
            Check(c.Position.Equals(job.Position) && job.Progress > 0, "Missing-supply retries prevented physical arrival and work");
            Near(Goods(w, v, "water"), 0, "Missing-supply retry created water");
            Check(c.Thirst < 1, "Missing-supply retry satisfied thirst without water");
            Valid(w);
        }
        static void ContinuousRate(string activity)
        {
            var w = Fixture(out var v, out var c);
            c.Stats.Clear();
            c.Stats.Add(new Stack("medicine", 50));
            c.Stats.Add(new Stack("building", 50));
            Farm farm = null;
            double before;
            if (activity == "research")
            {
                var station = Station(w, v, "research_hut", c.Position);
                Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = station.Id });
                v.ResearchPoints = 0;
                before = 0;
            }
            else if (activity == "farm")
            {
                farm = new Farm { Id = w.Id("continuous-farm"), WorkerId = c.Id, Position = c.Position, CycleSeconds = 600 };
                v.Farms.Add(farm);
                c.BuildingId = farm.Id;
                before = 0;
            }
            else
            {
                var den = v.Buildings.First(b => b.Id == c.BedId);
                c.Position = den.Position;
                c.X = c.Position.X;
                c.Z = c.Position.Z;
                c.Goal = "need_sleep";
                c.Rest = 50;
                before = c.Rest;
            }
            double Rate() => activity == "research" ? 1.0 / 600 : activity == "farm" ? 1 : 0.15 - 12.0 / 3600;
            double Value() => activity == "research" ? v.ResearchPoints : activity == "farm" ? farm.Growth : c.Rest;
            w.Step(0.2);
            Near(Value() - before, Rate() * 0.2, activity + " did not advance continuously at the original rate");
            w.Step(0.8);
            Near(Value() - before, Rate(), activity + " accelerated with fixed steps");
            Valid(w);
        }
        static void ContinuousCombatBudget()
        {
            var w = ContinuousFixture(out var v, out var c, true);
            foreach (var cat in v.Cats)
                cat.Skills.Clear();
            c.Skills.Add(new Stack("fight", 1));
            var job = v.Jobs.Single();
            var first = new Raid { Id = w.Id("first-raid"), Position = c.Position };
            var second = new Raid { Id = w.Id("second-raid"), Position = c.Position };
            v.Raids.Add(first);
            v.Raids.Add(second);
            w.Step(0.05);
            Near(job.Progress, 0.05, "Existing work lost the first actor quantum");
            Near(first.Health + second.Health, 60, "Work and combat each received the same cat's full quantum");
            w.Step(0.05);
            Check(first.Health < 30 && second.Health == 30, "One cat attacked two raids with the same quantum: first=" + first.Health + " second=" + second.Health + " goal=" + c.Goal + " path=" + c.Path.Count);
            Valid(w);
        }
        static void ContinuousTransportInterruption(bool shipping)
        {
            var w = Fixture(out var v, out var c);
            string mode = shipping ? "shipping" : "rail";
            var start = new Int2(4, 2);
            var middle = new Int2(5, 2);
            var end = new Int2(6, 2);
            foreach (var p in new[] { start, middle, end })
            {
                var tile = w.TileAt(p);
                tile.Wall = tile.Mountain = false;
                tile.Rail = !shipping;
                tile.Water = shipping && p.Equals(middle);
                tile.Dock = shipping && !p.Equals(middle);
            }
            c.Position = start;
            c.X = start.X;
            c.Z = start.Z;
            var station = Station(w, v, "research_hut", new Int2(2, 4));
            var destination = new Stockpile { Id = w.Id("interruption-destination"), Position = end };
            v.Stockpiles.Add(destination);
            v.Stockpiles[0].Position = start;
            var vehicle = new Vehicle { Id = w.Id("interruption-vehicle"), Mode = mode, Position = start, Cargo = new List<Stack> { new Stack("logs", 8) } };
            v.Vehicles.Add(vehicle);
            var route = new TransportRoute { Id = w.Id("interruption-route"), Mode = mode, CatId = c.Id, VehicleId = vehicle.Id, SourceId = v.Stockpiles[0].Id, DestinationId = destination.Id, Resource = "logs", Amount = 8, Phase = "outbound", Path = new List<Int2> { start, middle, end } };
            vehicle.RouteId = c.BuildingId = route.Id;
            v.Routes.Add(route);
            w.Step(0.2);
            Near(vehicle.X, start.X + 0.2, "Interruption fixture did not reach a partial edge");
            Check(!w.Apply(Context(v), new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = station.Id }).Success, "Fractional vehicle driver was reassigned before stopping");
            if (shipping)
            {
                c.Thirst = 0;
                w.Step(0.05);
                Near(c.X, vehicle.X, "Driver disembarked over water while its anchor was still the dock");
                Check(!c.Goal.StartsWith("need_", StringComparison.Ordinal), "Shipping needs interrupted an undocked fractional segment");
            }
            Act(w, v, new GameAction { Kind = "CancelTransportRoute", TargetId = route.Id });
            Check(v.Routes.Contains(route) && route.CancelRequested, "Cancellation discarded a vehicle mid-edge");
            double x = vehicle.X;
            w.Step(0.05);
            Check(vehicle.X < x && vehicle.X >= x - 0.05 - 1e-9, "Cancellation snapped the vehicle to its anchor");
            for (int tick = 0; tick < 400 && v.Routes.Contains(route); tick++)
                w.Step(0.05);
            Check(!v.Routes.Contains(route) && vehicle.RouteId == "" && vehicle.Position.Equals(start), "Vehicle did not complete physical cancellation after the docked driver's need trip: phase=" + route.Phase + " goal=" + c.Goal + " x=" + vehicle.X + " catX=" + c.X);
            Near(vehicle.X, start.X, "Cancelled vehicle retained a fractional position");
            Near(Goods(w, v, "logs"), 8, "Cancellation lost or duplicated finite cargo");
            Valid(w);
        }
        static void ContinuousMover(string kind)
        {
            var w = Fixture(out var v, out var c);
            var start = new Int2(4, 2);
            var next = new Int2(5, 2);
            var end = new Int2(6, 2);
            foreach (var p in new[] { start, next, end })
            {
                var tile = w.TileAt(p);
                tile.Wall = tile.Water = tile.Mountain = false;
                tile.Rail = true;
            }
            object mover;
            double speed;
            if (kind == "merchant")
            {
                var trader = v.Trader;
                trader.Phase = "arriving";
                trader.Position = start;
                trader.Progress = 0.8;
                trader.Path = new List<Int2> { next, end };
                trader.VisitDestination = end;
                trader.Goods.Add(new Stack("logs", 8));
                mover = trader;
                speed = 0.8;
            }
            else if (kind == "raid")
            {
                var raid = new Raid { Id = w.Id("continuous-raid"), Position = start, Progress = 12, Path = new List<Int2> { next, end }, Loot = new List<Stack> { new Stack("logs", 8) } };
                v.Raids.Add(raid);
                mover = raid;
                speed = 0.5;
            }
            else if (kind == "caravan")
            {
                var trade = new TradeOffer { Id = w.Id("continuous-trade"), Status = "outbound", X = start.X, Z = start.Z, FromVillageId = v.Id, ToVillageId = v.Id, Offered = new Stack("logs", 8), Path = new List<Int2> { next, end } };
                w.TradeOffers.Add(trade);
                mover = trade;
                speed = 0.8;
            }
            else
            {
                c.Position = start;
                c.X = start.X;
                c.Z = start.Z;
                var destination = new Stockpile { Id = w.Id("continuous-destination"), Position = end };
                v.Stockpiles.Add(destination);
                var vehicle = new Vehicle { Id = w.Id("continuous-cart"), Mode = "rail", Position = start, Cargo = new List<Stack> { new Stack("logs", 8) } };
                v.Vehicles.Add(vehicle);
                var route = new TransportRoute { Id = w.Id("continuous-route"), Mode = "rail", CatId = c.Id, VehicleId = vehicle.Id, SourceId = v.Stockpiles[0].Id, DestinationId = destination.Id, Resource = "logs", Amount = 8, Phase = "outbound", Path = new List<Int2> { start, next, end } };
                vehicle.RouteId = route.Id;
                v.Routes.Add(route);
                mover = vehicle;
                speed = 1;
            }
            w.Step(0.2);
            var xField = mover.GetType().GetField("X");
            double x = xField == null ? start.X : (double)xField.GetValue(mover);
            Near(x, start.X + speed * 0.2, kind + " did not advance its authoritative fractional position");
            var position = mover.GetType().GetField("Position");
            if (position != null)
                Check(((Int2)position.GetValue(mover)).Equals(start), kind + " grid anchor moved before reaching the next tile");
        }
        static void ContinuousLegacyCaravan(int direction)
        {
            var w = Fixture(out var v, out var c);
            var next = new Int2(direction * 5, 2);
            var tile = w.TileAt(next);
            tile.Wall = tile.Water = tile.Mountain = false;
            var trade = new TradeOffer { Id = w.Id("legacy-fractional-caravan"), Status = "outbound", X = direction * 4.6, Z = 2, Progress = 0.8, FromVillageId = v.Id, ToVillageId = v.Id, Offered = new Stack("logs", 8), Path = new List<Int2> { next, new Int2(direction * 6, 2) } };
            w.TradeOffers.Add(trade);
            w.Step(0.2);
            Near(trade.X, direction * 4.76, "Legacy caravan fractional position was rounded or received old cadence credit twice");
            Check(trade.Position.Equals(new Int2(direction * 4, 2)), "Legacy caravan did not retain the last reached grid anchor");
            Near(trade.Offered.Amount, 8, "Legacy continuous initialization changed finite cargo");
        }
    }
}
