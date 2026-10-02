using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>Direct control and player work orders: routes, cargo under control, zones and scouting.</summary>
    public static class ControlScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.direct_control_route_ownership", RouteControl);
            yield return new Scenario("regression.gather_and_avoid_zones", Zones);
            yield return new Scenario("regression.resource_targeted_scout", TargetedScout);
            yield return new Scenario("regression.direct_control_leaves_recoverable_cargo", ControlledCargo);
            yield return new Scenario("regression.review_scaffold_preserves_controlled_food", () => ControlledScaffoldCargo("food"));
            yield return new Scenario("regression.review_scaffold_preserves_surplus_planks", () => ControlledScaffoldCargo("planks"));
            yield return new Scenario("regression.review_new_building_preserves_controlled_food", () => ControlledScaffoldCargo("food", true));
            yield return new Scenario("regression.review_new_building_preserves_surplus_planks", () => ControlledScaffoldCargo("planks", true));
            yield return new Scenario("regression.review_new_job_preserves_cargo_during_offering", () => ControlledNewJobCargo("offering"));
            yield return new Scenario("regression.review_new_job_preserves_cargo_during_scouting", () => ControlledNewJobCargo("scout"));
            yield return new Scenario("regression.review_new_job_preserves_cargo_during_hauling", () => ControlledNewJobCargo("haul"));
            yield return new Scenario("regression.review_resume_road_preserves_controlled_food", () => ControlledResumeCargo("road"));
            yield return new Scenario("regression.review_resume_rail_preserves_controlled_food", () => ControlledResumeCargo("rail"));
            yield return new Scenario("regression.review_resume_expansion_preserves_controlled_food", () => ControlledResumeCargo("expand"));
            yield return new Scenario("regression.direct_control_flood_cannot_advance_time", ControlFlood);
        }
        static void RouteControl()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            var source = v.Stockpiles[0];
            source.Position = new Int2(1, 2);
            source.Goods.Add(new Stack("logs", 20));
            var destination = new Stockpile { Id = w.Id("destination"), Position = new Int2(3, 2) };
            v.Stockpiles.Add(destination);
            var path = new List<Int2> { new Int2(1, 2), new Int2(2, 2), new Int2(3, 2) };
            foreach (var p in path)
                w.TileAt(p).Rail = true;
            var vehicle = new Vehicle { Id = w.Id("wagon"), Mode = "rail", Position = source.Position };
            v.Vehicles.Add(vehicle);
            Act(w, v, new GameAction { Kind = "CreateTransportRoute", CatId = c.Id, TargetId = source.Id, BuildingId = destination.Id, Mode = "rail", Resource = "logs", Amount = 8, Path = path, Repeat = true });
            w.Step(3);
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "LeaveCat", CatId = c.Id });
            for (int tick = 0; tick < 2 / World.SimulationStepSeconds && v.Routes.Any(r => r.CatId == c.Id); tick++)
            {
                double x = vehicle.X, z = vehicle.Z;
                w.Step(World.SimulationStepSeconds);
                Check(Math.Sqrt((vehicle.X - x) * (vehicle.X - x) + (vehicle.Z - z) * (vehicle.Z - z)) <= World.SimulationStepSeconds + 1e-8, "Direct-control cancellation teleported its fractional wagon");
            }
            Check(!v.Routes.Any(r => r.CatId == c.Id) && vehicle.X == vehicle.Position.X && vehicle.Z == vehicle.Position.Z, "Direct-control cancellation did not physically settle its wagon");
            var station = Station(w, v, "research_hut");
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = station.Id });
            w.Step(10);
            Check(!v.Routes.Any(r => r.CatId == c.Id), "Route retained reassigned cat ownership");
            Near(Goods(w, v, "logs"), 20, "Direct-control transport conservation");
            Valid(w);
        }
        static void Zones()
        {
            var w = Fixture(out var v, out var c);
            Exterior(w, v);
            c.BuildingId = "";
            c.Position = new Int2(8, 8);
            c.X = 8;
            c.Z = 8;
            foreach (var t in w.Tiles.Where(t => t.Resource == "logs"))
            {
                t.Resource = "";
                t.Amount = 0;
            }
            var near = new Int2(9, 8);
            var preferred = new Int2(12, 8);
            w.TileAt(near).Resource = w.TileAt(preferred).Resource = "logs";
            w.TileAt(near).Amount = w.TileAt(preferred).Amount = 20;
            Act(w, v, new GameAction { Kind = "CreateZone", Resource = "gather", Position = preferred, End = preferred });
            Act(w, v, new GameAction { Kind = "RequestJob", Name = "logs", CatId = c.Id });
            Check(v.Jobs.Single(j => j.CatId == c.Id && !j.Completed).Position.Equals(preferred), "Gather zone does not influence source selection");
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "CreateZone", Resource = "avoid", Position = near, End = near });
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Check(!w.Apply(Context(v), new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(1, 0) }).Success, "Avoid zone did not constrain ordinary route");
        }
        static void TargetedScout()
        {
            var w = Fixture(out var v, out var c);
            Exterior(w, v);
            c.BuildingId = "";
            c.Position = new Int2(8, 8);
            c.X = 8;
            c.Z = 8;
            var target = new Int2(11, 10);
            foreach (var t in w.Tiles)
            {
                if (t.Resource == "ore")
                {
                    t.Resource = "";
                    t.Amount = 0;
                }
            }
            w.TileAt(target).Resource = "ore";
            w.TileAt(target).Amount = 30;
            v.Known.RemoveAll(p => p.X >= 9 && p.Z >= 9);
            Act(w, v, new GameAction { Kind = "DispatchScout", CatId = c.Id, Resource = "ore" });
            w.Step(240);
            Check(v.Known.Contains(target), "Resource-targeted scout failed to return nearby ore discovery to shrine");
            Check(c.ScoutNotes.Count == 0, "Scout discovery not committed by shrine return");
        }
        static void ControlledCargo()
        {
            var w = Fixture(out var v, out var c);
            c.Thirst = 1;
            c.Cargo.Add(new Stack("logs", 8));
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "LeaveCat", CatId = c.Id });
            w.Step(60);
            Check(c.Alive && c.Thirst > 35, "Leaving direct control with cargo prevents AI need recovery");
            Near(Goods(w, v, "logs"), 8, "Direct-control cargo disappeared");
        }
        static void ControlledScaffoldCargo(string resource, bool planAfterControl = false)
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            c.Position = new Int2(2, 2);
            c.X = 2;
            c.Z = 2;
            var source = v.Stockpiles[0];
            World.Add(source.Goods, "planks", 12);
            World.Add(source.Goods, "blocks", 2);
            Building scaffold = null;
            if (!planAfterControl)
            {
                var planned = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id });
                scaffold = v.Buildings.Single(b => b.Id == planned.EntityId);
            }
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "InteractCat", CatId = c.Id, TargetId = source.Id, Resource = resource, Amount = 8 });
            Act(w, v, new GameAction { Kind = "LeaveCat", CatId = c.Id });
            Check(c.JobId == "" && World.Amount(c.Cargo, resource) == 8, "Fixture must leave direct control with cargo and no active job");
            double initialFood = Goods(w, v, "food"), initialPlanks = Goods(w, v, "planks"), initialBlocks = Goods(w, v, "blocks");
            if (planAfterControl)
            {
                var planned = Act(w, v, new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id });
                scaffold = v.Buildings.Single(b => b.Id == planned.EntityId);
            }
            else
                Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = scaffold.Id });
            bool preservedAtHandoff = c.Cargo.Count == 0 && v.Stockpiles.Any(p => p.Kind == "spill" && p.Position.Equals(new Int2(2, 2)) && World.Amount(p.Goods, resource) == 8);
            for (int tick = 0; tick < 600 && !scaffold.Completed; tick++)
                w.Step(1);
            Check(scaffold.Completed, "Reassigned scaffold did not complete with its finite material bill");
            Near(Goods(w, v, "food"), initialFood * Math.Pow(1 - 0.0002, Math.Floor(w.TimeSeconds / 60)), "Construction destroyed unrelated food beyond ordinary spoilage");
            Near(Goods(w, v, "planks"), initialPlanks - World.Amount(scaffold.Required, "planks"), "Construction consumed surplus planks beyond its material bill");
            Near(Goods(w, v, "blocks"), initialBlocks - World.Amount(scaffold.Required, "blocks"), "Construction block bill was not conserved");
            Check(preservedAtHandoff, "New work must leave prior cargo in a physical spill before fetching construction materials");
            Check(c.Cargo.Count == 0 && scaffold.Inputs.Count == 0 && w.Reservations.Count == 0, "Completed construction left cargo or input claims");
            Valid(w);
        }
        static void ControlledNewJobCargo(string kind)
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            c.Position = new Int2(2, 2);
            c.X = 2;
            c.Z = 2;
            var storage = v.Stockpiles[0];
            storage.Accepts.AddRange(new[] { "food", "water", "materials", "logs" });
            World.Add(storage.Goods, "stone", 8);
            World.Add(storage.Goods, "materials", 15);
            var haulSource = new Stockpile { Id = w.Id("haul-source"), Kind = "spill", Position = new Int2(-2, 1), Goods = new List<Stack> { new Stack("logs", 8) } };
            v.Stockpiles.Add(haulSource);
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "InteractCat", CatId = c.Id, TargetId = storage.Id, Resource = "stone", Amount = 8 });
            Act(w, v, new GameAction { Kind = "LeaveCat", CatId = c.Id });
            Check(c.JobId == "" && World.Amount(c.Cargo, "stone") == 8, "Fixture must start new work with unassigned controlled cargo");
            var action = kind == "offering" ? new GameAction { Kind = "OfferResource", Resource = "materials", CatId = c.Id } : kind == "scout" ? new GameAction { Kind = "DispatchScout", CatId = c.Id } : new GameAction { Kind = "HaulGatherSpot", CatId = c.Id, TargetId = haulSource.Id };
            Act(w, v, action);
            var job = v.Jobs.Single(j => j.CatId == c.Id && !j.Completed);
            bool preservedAtStart = c.Cargo.Count == 0 && v.Stockpiles.Any(p => p.Kind == "spill" && p.Position.Equals(new Int2(2, 2)) && World.Amount(p.Goods, "stone") == 8);
            bool carriedClaimedInput = false;
            for (int tick = 0; tick < 600 && !job.Completed; tick++)
            {
                w.Step(1);
                carriedClaimedInput |= World.Amount(c.Cargo, kind == "offering" ? "materials" : "logs") > 0;
            }
            Check(job.Completed, "Public " + kind + " failed to complete while preserving prior cargo");
            Near(Goods(w, v, "stone"), 8, "New " + kind + " consumed prior cargo");
            Check(preservedAtStart, "New " + kind + " must preserve prior cargo before claiming work ownership");
            Check(c.Cargo.Count == 0 && w.Reservations.Count == 0, "New " + kind + " left cargo or reservations after completion");
            if (kind == "offering")
            {
                Check(carriedClaimedInput, "Offering bypassed physical claimed input pickup");
                Near(Goods(w, v, "materials"), 10, "Offering consumed the wrong finite bill");
                Near(v.Blessings, 1, "Offering converted unrelated cargo into blessings");
            }
            if (kind == "haul")
            {
                Check(carriedClaimedInput, "Haul bypassed physical claimed source pickup");
                Near(World.Amount(storage.Goods, "logs"), 8, "Haul did not deliver its claimed logs to accepting storage");
                Near(World.Amount(haulSource.Goods, "logs"), 0, "Haul duplicated its finite source");
            }
            if (kind == "scout")
                Check(c.ScoutNotes.Count == 0 && c.Position.Equals(v.Center), "Scout failed to return its knowledge to the shrine");
            Valid(w);
        }
        static void ControlledResumeCargo(string kind)
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            string material = kind == "rail" ? "metal" : "materials";
            var replacement = v.Cats[1];
            replacement.ControlledBy = "";
            replacement.BuildingId = "fixture-held";
            replacement.Position = new Int2(2, 2);
            replacement.X = 2;
            replacement.Z = 2;
            var holdStation = Station(w, v, "wood_cutter", new Int2(5, 1));
            World.Add(v.Stockpiles[0].Goods, material, 1000);
            var first = new Int2(1, 2);
            var last = new Int2(1, 3);
            foreach (var at in new[] { first, last })
            {
                var tile = w.TileAt(at);
                tile.Road = tile.Rail = tile.Wall = tile.Water = tile.Mountain = false;
            }
            w.TileAt(new Int2(0, 2)).Road = true;
            if (kind == "rail")
                v.Research.Add("rail");
            int newRadius = v.Radius + 2;
            if (kind == "expand")
                foreach (var tile in w.Tiles.Where(t => Math.Abs(t.Position.X) == newRadius || Math.Abs(t.Position.Z) == newRadius))
                {
                    tile.Wall = tile.Water = tile.Mountain = false;
                    tile.Resource = "";
                    tile.Amount = 0;
                }
            var action = kind == "expand" ? new GameAction { Kind = "RequestJob", Name = "expand", CatId = c.Id } : new GameAction { Kind = kind == "road" ? "BuildRoad" : "DesignateRail", Position = first, End = last, CatId = c.Id };
            Act(w, v, action);
            var pending = v.Jobs.Single(j => j.CatId == c.Id && !j.Completed);
            for (int tick = 0; tick < 1800 && !(pending.Phase == "working" && pending.Progress > 0 && World.Amount(c.Cargo, material) == 1); tick++)
                w.Step(1);
            Check(pending.Phase == "working" && pending.Progress > 0 && World.Amount(c.Cargo, material) == 1, "Fixture must interrupt active " + kind + " work with its real segment material in cargo");
            int builtBefore = pending.PathIndex;
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Check(pending.CatId == "" && w.Reservations.Any(r => r.OwnerId == pending.Id + ":resume" && r.Resource == material && r.Amount == 1), "Interruption did not reserve the carried segment material for resumption");
            Act(w, v, new GameAction { Kind = "LeaveCat", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = holdStation.Id });
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = replacement.Id });
            Act(w, v, new GameAction { Kind = "InteractCat", CatId = replacement.Id, TargetId = v.Stockpiles[0].Id, Resource = "food", Amount = 8 });
            Act(w, v, new GameAction { Kind = "LeaveCat", CatId = replacement.Id });
            Check(replacement.JobId == "" && World.Amount(replacement.Cargo, "food") == 8, "Replacement must carry unrelated food before pending work adoption");
            if (kind == "expand")
                Act(w, v, new GameAction { Kind = "RequestJob", Name = "expand", CatId = replacement.Id });
            else
                w.Step(1);
            bool preservedAtAdoption = World.Amount(replacement.Cargo, "food") == 0 && v.Stockpiles.Any(p => p.Kind == "spill" && p.Position.Equals(new Int2(2, 2)) && World.Amount(p.Goods, "food") == 8);
            Check(pending.CatId == replacement.Id && replacement.JobId == pending.Id, "Pending " + kind + " was not assigned to its replacement worker");
            for (int tick = 0; tick < 12000 && !pending.Completed; tick++)
                w.Step(1);
            Check(pending.Completed && pending.PathIndex == pending.Path.Count && pending.PathIndex > builtBefore, "Resumed " + kind + " failed to finish its physical path");
            Near(Goods(w, v, material), 1000 - pending.Path.Count, "Resumed " + kind + " failed to consume exactly one real material per completed segment");
            Check(preservedAtAdoption, "Pending " + kind + " adoption must preserve unrelated cargo before claiming the worker");
            Check(w.Reservations.Count == 0 && replacement.Cargo.Count == 0, "Resumed " + kind + " retained cargo or suspended claims after completion");
            Check(pending.Path.All(at => kind == "road" ? w.TileAt(at).Road : kind == "rail" ? w.TileAt(at).Rail : w.TileAt(at).Wall), "Resumed " + kind + " lost a completed physical segment");
            if (kind == "expand")
            {
                Check(v.Radius == newRadius, "Resumed expansion never committed its complete perimeter");
                Check(!w.TileAt(new Int2(0, newRadius)).Wall, "Expansion closed the required south gate");
                Check(!w.Tiles.Any(tile => Math.Abs(tile.Position.X) < newRadius && Math.Abs(tile.Position.Z) < newRadius && tile.Wall), "Expansion retained an obsolete interior wall");
            }
            Valid(w);
        }
        static void ControlFlood()
        {
            var w = Fixture(out var v, out var c);
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            double time = w.TimeSeconds, x = c.X, z = c.Z;
            for (int i = 0; i < 1000; i++)
                Act(w, v, new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(1, 0) });
            Near(w.TimeSeconds, time, "Action flood advanced authority clock");
            Near(c.X, x, "Action flood directly moved cat");
            Near(c.Z, z, "Action flood directly moved cat");
            w.Step(0.1);
            Check(c.X > x && c.X - x < 1, "Continuous movement lacks bounded explicit-time integration");
        }
    }
}
