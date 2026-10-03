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
            yield return new Scenario("regression.direct_control_held_direction_chains_tiles", HeldDirection);
            yield return new Scenario("regression.direct_control_reversal_turns_back_mid_edge", MidEdgeReversal);
            yield return new Scenario("regression.direct_control_diagonal_needs_open_corners", DiagonalCorners);
            yield return new Scenario("regression.direct_control_moves_on_real_time", RealTimeControl);
            yield return new Scenario("regression.direct_control_lease_expiry_clears_intent", LeaseExpiry);
            yield return new Scenario("regression.direct_control_chest_reach_is_adjacent", ChestReach);
            yield return new Scenario("regression.direct_control_interact_folds_needs", InteractFolds);
            yield return new Scenario("regression.direct_control_pile_reach_covers_footprint", PileReach);
        }

        /// <summary>A flat, open grass meadow around the fixture cat (x 0..7, z 0..5) with nothing built on it.</summary>
        static World Meadow(out Village v, out Cat c)
        {
            var w = Fixture(out v, out c);
            c.BuildingId = "";
            v.Buildings.RemoveAll(b => b.Kind != "shrine" && b.Position.X <= 8 && b.Position.X + b.Width >= 0 && b.Position.Z <= 6 && b.Position.Z + b.Depth >= 0);
            for (int x = 0; x <= 7; x++)
                for (int z = 0; z <= 5; z++)
                {
                    var tile = w.TileAt(new Int2(x, z));
                    tile.Road = tile.Dirt = tile.Water = tile.Wall = tile.Mountain = tile.Rail = tile.Bridge = false;
                    tile.Biome = "grass";
                    tile.Elevation = 0;
                }
            v.BoundaryEdges.RemoveAll(e => e.From.X >= 0 && e.From.X <= 7 && e.From.Z >= 0 && e.From.Z <= 5);
            c.Y = 0;
            c.HasHeight = true;
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            return w;
        }
        static void Move(World w, Village v, Cat c, int x, int z) => Act(w, v, new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(x, z) });
        static void HeldDirection()
        {
            var w = Meadow(out var v, out var c);
            Move(w, v, c, 1, 0);
            Check(c.ControlDirection.Equals(new Int2(1, 0)), "MoveCat must store the held direction");
            double last = c.X;
            for (int tick = 0; tick < 40; tick++)
            {
                // The client re-sends the held key every 0.1 s.
                if (tick % 2 == 1)
                    Move(w, v, c, 1, 0);
                w.Step(World.SimulationStepSeconds);
                Check(c.X > last + 1e-9, "A held direction must keep the cat moving through tile centres (tick " + tick + " x=" + c.X + ")");
                last = c.X;
            }
            Check(c.X - 1 > 2.5, "Two seconds of a held key must cover several tiles, moved " + (c.X - 1));
            Check(Math.Abs(c.Z - 2) < 1e-9, "A held east key must not drift");
            Act(w, v, new GameAction { Kind = "StopCat", CatId = c.Id });
            Check(c.ControlDirection.Equals(default(Int2)), "StopCat clears the held direction");
            w.Step(1);
            Check(c.Path.Count == 0 && Math.Abs(c.X - Math.Round(c.X)) < 1e-9 && c.Position.X == (int)Math.Round(c.X), "StopCat lets the cat finish its edge and stand on a tile");
            double rest = c.X;
            w.Step(1);
            Near(c.X, rest, "A stopped cat stays put");
            Move(w, v, c, -1, 0);
            w.Step(3);
            Check(c.ControlDirection.Equals(default(Int2)) && c.Path.Count == 0 && rest - c.X >= 1 && rest - c.X < 2.5, "A hold that is not refreshed (a missed key release) lapses and the cat stands, moved " + (rest - c.X));
        }
        static void MidEdgeReversal()
        {
            var w = Meadow(out var v, out var c);
            Move(w, v, c, 1, 0);
            w.Step(0.3);
            Check(c.X > 1.2 && c.X < 2 && c.Position.X == 1, "Fixture must be mid-edge");
            double x = c.X;
            Move(w, v, c, -1, 0);
            w.Step(World.SimulationStepSeconds);
            Check(c.X < x, "Reversing mid-edge must turn back at once, not finish the edge first");
            w.Step(2);
            Check(c.X < 1, "The cat keeps walking west after turning back");
        }
        static void DiagonalCorners()
        {
            var w = Meadow(out var v, out var c);
            Move(w, v, c, 1, 1);
            Act(w, v, new GameAction { Kind = "StopCat", CatId = c.Id });
            w.Step(2);
            Check(c.Position.Equals(new Int2(2, 3)) && Math.Abs(c.X - 2) < 1e-9 && Math.Abs(c.Z - 3) < 1e-9, "An open diagonal is one straight step, at " + c.Position);
            w.TileAt(new Int2(3, 3)).Wall = true;
            var cut = w.Apply(Context(v), new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(1, -1) });
            Check(!cut.Success && cut.Code == ResultCode.BlockedRoute, "A diagonal must not cut a walled corner, got " + cut.Code);
            var straight = w.Apply(Context(v), new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(1, 0) });
            Check(!straight.Success && straight.Code == ResultCode.BlockedRoute && c.ControlDirection.Equals(default(Int2)), "Walking into a wall refuses and holds nothing");
            var wild = w.Apply(Context(v), new GameAction { Kind = "MoveCat", CatId = c.Id, Position = new Int2(2, 0) });
            Check(!wild.Success && wild.Code == ResultCode.InvalidDirection, "Steps are single tiles");
        }
        static void RealTimeControl()
        {
            double Walked(double scale, double seconds)
            {
                var w = Meadow(out var v, out var c);
                w.ControlTimeScale = scale;
                // The client holds the key for `seconds` of real time, re-sending every 0.1 s.
                for (int i = 0; i < Math.Round(seconds / 0.1); i++)
                {
                    Move(w, v, c, 1, 0);
                    // At 0x the game does not step; stepping anyway proves the rule lives in the simulation.
                    w.Step(0.1 * (scale > 0 ? scale : 1));
                }
                return c.X - 1;
            }
            double normal = Walked(1, 1);
            Check(normal > 1.2, "The fixture must walk well over a tile in one held second at 1x, walked " + normal);
            Near(Walked(4, 1), normal, "At 4x a steered cat still walks at real-time pace", 1e-6);
            Near(Walked(2, 1), normal, "At 2x a steered cat still walks at real-time pace", 1e-6);
            Near(Walked(0, 1), 0, "At 0x a steered cat does not move");
        }
        static void LeaseExpiry()
        {
            var w = Meadow(out var v, out var c);
            Move(w, v, c, 0, 1);
            Check(!c.ControlDirection.Equals(default(Int2)), "MoveCat must hold the direction");
            w.Step(31);
            Check(c.ControlledBy == "" && c.ControlDirection.Equals(default(Int2)), "Lease expiry releases the cat and its held direction");
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            double z = c.Z;
            w.Step(1);
            Near(c.Z, z, "A new lease starts without a stale direction");
        }
        static void ChestReach()
        {
            var w = World.Create(41);
            var v = w.Villages[0];
            var c = v.Cats[0];
            v.Cats.RemoveAll(x => x.Id != c.Id);
            var floor = w.Dungeons[0].Floors[0];
            // The chest stays locked while its guardian lives (W2-09); this case is about reach only.
            foreach (var guardian in w.Creatures.Where(e => e.SiteId == w.Dungeons[0].Id && e.Position.Level == floor.Level))
                guardian.Health = 0;
            var beside = floor.Cells.First(p => Int2.Distance(p, floor.LootPosition) == 1 && w.Walkable(v, p));
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            c.Position = beside;
            c.X = beside.X + (floor.LootPosition.X - beside.X) * 0.3;
            c.Z = beside.Z + (floor.LootPosition.Z - beside.Z) * 0.3;
            c.Y = w.WalkHeight(beside);
            c.HasHeight = true;
            double finite = floor.Loot.Sum(s => s.Amount);
            Check(finite > 0, "Fixture chest must hold loot");
            var r = w.Apply(Context(v), new GameAction { Kind = "InteractCat", CatId = c.Id });
            Check(r.Success && r.Code == ResultCode.Collected, "A cat beside the chest, still settling, must collect it: " + r.Code + " " + r.Error);
            // Chest notes become the cat's lore and weapons or relics become carried items (W2-09).
            double carried = c.Cargo.Sum(s => s.Amount) + c.DungeonLore + v.Items.Count(i => i.LocationId == c.Id);
            Near(carried + floor.Loot.Sum(s => s.Amount), finite, "Collection conserves the chest");
            Check(carried > 0, "Collection moved loot to the cat");
            c.Position = new Int2(floor.LootPosition.X + 3, floor.LootPosition.Z, floor.LootPosition.Level);
            var far = w.Apply(Context(v), new GameAction { Kind = "InteractCat", CatId = c.Id });
            Check(!far.Success && far.Code == ResultCode.NothingInReach, "A distant chest is out of reach");
        }
        static void InteractFolds()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            var atShrine = new Int2(v.Center.X + 1, v.Center.Z);
            c.Position = atShrine;
            c.X = atShrine.X;
            c.Z = atShrine.Z;
            c.Hunger = 20;
            c.Thirst = 100;
            c.Cargo.Add(new Stack("food", 1));
            var ate = w.Apply(Context(v), new GameAction { Kind = "InteractCat", CatId = c.Id });
            Check(ate.Code == ResultCode.Ate && c.Hunger > 60 && c.Cargo.Count == 0, "E at the shrine with food in cargo and an empty belly eats: " + ate.Code);
            c.Thirst = 20;
            c.Cargo.Add(new Stack("water", 1));
            var drank = w.Apply(Context(v), new GameAction { Kind = "InteractCat", CatId = c.Id });
            Check(drank.Code == ResultCode.Drank && c.Thirst > 60, "E at the shrine with water and thirst drinks: " + drank.Code);
            var hungry = w.Apply(Context(v), new GameAction { Kind = "InteractCat", CatId = c.Id, Mode = "eat" });
            Check(hungry.Code == ResultCode.NeedFoodInCargo, "Eating without food says what to carry: " + hungry.Code);
            var den = v.Buildings.First(b => b.Kind == "den" && b.Completed);
            c.BedId = den.Id;
            var away = new Int2(den.Position.X + 20, den.Position.Z + 20);
            c.Position = away;
            c.X = away.X;
            c.Z = away.Z;
            var noBed = w.Apply(Context(v), new GameAction { Kind = "InteractCat", CatId = c.Id, Mode = "rest" });
            Check(noBed.Code == ResultCode.OutOfReach, "Resting away from the bed says to walk there: " + noBed.Code);
            var thirsty = w.Apply(Context(v), new GameAction { Kind = "InteractCat", CatId = c.Id, Mode = "drink" });
            Check(thirsty.Code == ResultCode.GoToShrine, "Drinking away from the shrine says where to go: " + thirsty.Code);
            var beside = new Int2(den.Position.X - 1, den.Position.Z);
            c.Position = beside;
            c.X = beside.X;
            c.Z = beside.Z;
            c.Rest = 30;
            var rest = w.Apply(Context(v), new GameAction { Kind = "InteractCat", CatId = c.Id });
            Check(rest.Code == ResultCode.Resting && c.Goal == GoalKey.PlayerRest, "E beside the cat's own den rests: " + rest.Code);
            c.BedId = "";
            var nobody = w.Apply(Context(v), new GameAction { Kind = "InteractCat", CatId = c.Id, Mode = "rest" });
            Check(nobody.Code == ResultCode.NoReservedBed, "A cat without a bed is told so: " + nobody.Code);
        }
        static void PileReach()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            var pile = v.Stockpiles[0];
            pile.Position = new Int2(3, 2);
            pile.Width = pile.Depth = 4;
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            var inside = new Int2(5, 4);
            c.Position = inside;
            c.X = inside.X;
            c.Z = inside.Z;
            Check(w.InReach(inside, pile) && w.InReach(new Int2(7, 4), pile) && w.InReach(new Int2(7, 6), pile) && !w.InReach(new Int2(8, 4), pile) && !w.InReach(new Int2(7, 7), pile) && !w.InReach(new Int2(5, 4, -1), pile), "Reach is the footprint plus the ring of tiles touching it, on its level");
            var take = w.Apply(Context(v), new GameAction { Kind = "InteractCat", CatId = c.Id, TargetId = pile.Id, Resource = "food", Amount = 3 });
            Check(take.Code == ResultCode.Taken && World.Amount(c.Cargo, "food") == 3, "A cat inside a large pile reaches it: " + take.Code + " " + take.Error);
            var put = w.Apply(Context(v), new GameAction { Kind = "InteractCat", CatId = c.Id, TargetId = pile.Id });
            Check(put.Code == ResultCode.Deposited && c.Cargo.Count == 0, "Carrying cargo deposits it: " + put.Code);
            var plenty = w.Apply(Context(v), new GameAction { Kind = "InteractCat", CatId = c.Id, TargetId = pile.Id, Resource = "", Amount = 100 });
            Check(plenty.Code == ResultCode.Taken && c.Cargo.Count == 1 && World.Amount(c.Cargo, "food") == w.CarryCapacity(v, "food"), "An empty resource takes a full load of the most plentiful good");
            c.Cargo.Clear();
            c.Position = new Int2(12, 12);
            var far = w.Apply(Context(v), new GameAction { Kind = "InteractCat", CatId = c.Id, TargetId = pile.Id, Resource = "food" });
            Check(far.Code == ResultCode.OutOfReach && TargetArg(far) == pile.Id, "A distant pile is out of reach and named");
        }
        static string TargetArg(ActionResult r) => r.Args != null && r.Args.TryGetValue(ResultArg.Target, out var value) ? value : null;
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
