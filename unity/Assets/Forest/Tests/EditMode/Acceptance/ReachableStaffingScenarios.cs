using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class ReachableStaffingScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.reachable_staffing_skips_blocked_preference", RankedFallback);
            yield return new Scenario("acceptance.reachable_staffing_finite_physical_production", PhysicalProduction);
            yield return new Scenario("regression.reachable_staffing_preserves_labor_ranking", () => ReachableRanking(false));
            yield return new Scenario("regression.reachable_staffing_preserves_identity_tiebreak", () => ReachableRanking(true));
            yield return new Scenario("regression.reachable_staffing_waits_for_route_without_claiming", RouteRecovery);
            yield return new Scenario("regression.reachable_staffing_retains_blocked_manual_worker", () => RetainAssignedWorker(false));
            yield return new Scenario("regression.reachable_staffing_retains_blocked_officer_worker", () => RetainAssignedWorker(true));
            yield return new Scenario("regression.reachable_staffing_researcher_reaches_workplace", ResearchWorkplace);
            yield return new Scenario("regression.reachable_staffing_requires_walkable_workplace", BlockedWorkplace);
            yield return new Scenario("regression.farmer_staffing_skips_unreachable_preferred_cat", FarmRankedFallback);
            yield return new Scenario("regression.farmer_staffing_preserves_labor_ranking", () => FarmRanking(false));
            yield return new Scenario("regression.farmer_staffing_preserves_identity_tiebreak", () => FarmRanking(true));
            yield return new Scenario("regression.farmer_staffing_waits_for_route_without_claiming", FarmRouteRecovery);
            yield return new Scenario("regression.farmer_staffing_retains_blocked_manual_worker", () => FarmRetainAssignedWorker(false));
            yield return new Scenario("regression.farmer_staffing_retains_blocked_officer_worker", () => FarmRetainAssignedWorker(true));
            yield return new Scenario("regression.farmer_staffing_requires_walkable_plot", FarmBlockedPlot);
            yield return new Scenario("acceptance.farmer_staffing_physical_crop_handoff", FarmPhysicalHandoff);
            yield return new Scenario("regression.farmer_labor_allocation_funded_mill_and_scaled_reserve", () => FarmerLaborAllocation(false));
            yield return new Scenario("regression.farmer_labor_allocation_kittens_do_not_fund_reserve", () => FarmerLaborAllocation(true));
            yield return new Scenario("acceptance.farmer_labor_chain_finite_grain_to_flour_with_working_plots", FarmerLaborChain);
            yield return new Scenario("regression.farm_manual_assignment_replaces_farmer_owner", () => FarmManualAssignment(false));
            yield return new Scenario("regression.farm_manual_assignment_takes_over_same_farmer_worker", () => FarmManualAssignment(true));
            foreach (string leg in new[] { "plot", "harvest", "handoff" })
            {
                string travelLeg = leg;
                yield return new Scenario("regression.farm_travel_open_" + leg, () => FarmTravelOpen(travelLeg));
                yield return new Scenario("regression.farm_travel_reopens_" + leg, () => FarmTravelRecovery(travelLeg));
            }
            yield return new Scenario("regression.farm_travel_exhausted_budget_preserves_feedback", FarmTravelExhaustedBudget);
            yield return new Scenario("regression.farm_travel_retrace_clears_stale_route_warning", FarmTravelRetrace);
            yield return new Scenario("regression.farm_travel_pickup_clears_old_route_warning", FarmTravelPickup);
            yield return new Scenario("regression.farm_travel_keeps_full_handoff_and_need_feedback", FarmTravelOtherBlockers);
            yield return new Scenario("regression.stale_work_cleared_farm_preserves_surviving_assignment", () => StaleWorkClearedFarm(false));
            yield return new Scenario("regression.stale_work_redesignation_preserves_exact_plot_identity", () => StaleWorkClearedFarm(true));
            yield return new Scenario("regression.stale_work_missing_building_preserves_funded_production", StaleWorkMissingBuilding);
            yield return new Scenario("regression.stale_work_missing_preferred_building_cannot_fall_back_to_farm", StaleWorkMixedDestination);
            yield return new Scenario("regression.stale_work_existing_destinations_keep_preferred_building", StaleWorkExistingDestinations);
            yield return new Scenario("regression.stale_work_destinationless_release_stays_explicit", StaleWorkIntentionalRelease);
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static void Act(World world, Village village, GameAction action)
        {
            var result = world.Apply(new PlayerContext { PlayerId = "reachable-staffing-acceptance", VillageId = village.Id }, action);
            Check(result.Success, action.Kind + ": " + result.Error);
        }
        static void Place(Cat cat, Int2 position)
        {
            cat.Position = position;
            cat.X = position.X;
            cat.Z = position.Z;
            cat.Path.Clear();
        }
        static void Valid(World world) => Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));

        static World Fixture(string kind, out Village village, out Building station, out Cat blocked, out Cat reachable)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            var store = village.Stockpiles[0];
            store.Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000), new Stack("fibre", 5) };
            store.Capacity = 100000;
            store.Position = new Int2(3, 3);
            village.LastLeaderResearch = 0;
            var knownLogs = world.Tiles.First(t => t.Resource == "logs" && t.Amount > 0).Position;
            if (!village.Known.Contains(knownLogs)) village.Known.Add(knownLogs);
            for (int index = 0; index < village.Cats.Count; index++)
            {
                var cat = village.Cats[index];
                cat.ControlledBy = index >= 1 && index <= 6 ? "" : "fixture-held";
                cat.ControlLeaseUntil = 10000000;
                cat.Skills.Clear();
                cat.Preferences.Clear();
                cat.Boosted = false;
                cat.Hunger = cat.Thirst = cat.Rest = 100;
                Place(cat, new Int2(1, 3));
            }
            blocked = village.Cats[1];
            reachable = village.Cats[2];
            string labor = kind == "research_hut" ? "research" : "textile";
            blocked.Preferences.Add(labor);
            World.Add(blocked.Skills, labor, 50);
            World.Add(reachable.Skills, labor, 1);
            Place(blocked, new Int2(-5, 1));
            // Two ordinary walkable tiles form a closed component. A failed route
            // explores only that component, without putting a cat inside a wall.
            foreach (var position in new[] { new Int2(-6, 1), new Int2(-5, 0), new Int2(-5, 2), new Int2(-4, 0), new Int2(-4, 2), new Int2(-3, 1) })
                world.TileAt(position).Wall = true;
            station = new Building { Id = world.Id("reachable-staffing-station"), Kind = kind, Position = new Int2(1, 3), Completed = true };
            village.Buildings.Add(station);
            village.Research.Add(kind == "research_hut" ? "research_hut" : "textiles");
            Act(world, village, new GameAction { Kind = "AssignOfficer", Role = kind == "research_hut" ? "loremaster" : "cloth_leader", CatId = village.Cats[0].Id });
            Check(world.Path(blocked.Position, station.Position, village) == null && world.Path(reachable.Position, station.Position, village) != null && world.Path(store.Position, station.Position, village) != null, "Staffing fixture must separate its preferred cat from the reachable workplace and finite inputs");
            Check(village.Cats.Count(c => c.Alive && c.ControlledBy == "" && c.JobId == "" && c.BuildingId == "") == 6, "Staffing fixture needs its four-cat reserve and two eligible candidates");
            return world;
        }

        static WorkSlot ExpectWorker(Village village, Building station, Cat expected)
        {
            Check(station.Slots.Count(s => s.CatId != "") == 1 && station.Slots.Single(s => s.CatId != "").CatId == expected.Id, "An unreachable preferred cat occupied the automatic workplace instead of its reachable eligible alternate");
            var slot = station.Slots.Single(s => s.CatId == expected.Id);
            Check(expected.BuildingId == station.Id && station.WorkerId == expected.Id && slot.AutomatedBy == World.WorkplaceOfficerRole(station.Kind), "Reachable staffing lost the exact cat, station or officer ownership");
            Check(village.Cats.Count(c => c.Alive && c.ControlledBy == "" && c.JobId == "" && c.BuildingId == "") == 5, "One automatic assignment consumed extra workers or its four-cat reserve");
            return slot;
        }

        static void RankedFallback()
        {
            var world = Fixture("clothier", out var village, out var station, out var blocked, out var reachable);
            world.Step(10);
            var slot = ExpectWorker(village, station, reachable);
            Check(slot.Queue.Count == 1 && slot.Queue[0].RecipeId == "fibre_to_thread" && slot.Queue[0].AutomatedBy == "cloth_leader", "The reachable worker did not receive its funded finite officer recipe");
            Check(blocked.BuildingId == "" && blocked.JobId == "" && blocked.Cargo.Count == 0 && blocked.Preferences.Contains("textile") && World.Amount(blocked.Skills, "textile") == 50, "Staffing changed the blocked cat's work, goods or labor ranking");
            Check(world.Total(village, "fibre") == 5 && world.Reservations.Count == 0 && reachable.Cargo.Count == 0, "Selecting the reachable worker moved or claimed goods before the recipe began");
            Valid(world);
        }

        static double TextileUnits(Village village) => village.Stockpiles.Sum(p => World.Amount(p.Goods, "fibre") + World.Amount(p.Goods, "thread"))
            + village.Cats.Sum(c => World.Amount(c.Cargo, "fibre") + World.Amount(c.Cargo, "thread"))
            + village.Jobs.Where(j => !j.Completed).Sum(j => World.Amount(j.Local, "fibre") + World.Amount(j.Local, "thread"))
            + village.Buildings.Sum(b => World.Amount(b.Inputs, "fibre") + World.Amount(b.Inputs, "thread") + World.Amount(b.Outputs, "fibre") + World.Amount(b.Outputs, "thread")
                + b.Slots.Sum(s => World.Amount(s.Inputs, "fibre") + World.Amount(s.Inputs, "thread") + World.Amount(s.Outputs, "fibre") + World.Amount(s.Outputs, "thread")));

        static void PhysicalProduction()
        {
            var world = Fixture("clothier", out var village, out var station, out var blocked, out var reachable);
            world.Step(11);
            ExpectWorker(village, station, reachable);
            var job = village.Jobs.SingleOrDefault(j => !j.Completed && j.Kind == "production" && j.CatId == reachable.Id);
            Check(job != null && job.RecipeId == "fibre_to_thread", "Reachable staffing did not begin the exact physical batch");
            bool carried = World.Amount(reachable.Cargo, "fibre") == 5 && !reachable.Position.Equals(station.Position), worked = job.Progress > 0;
            for (int tick = 0; tick < 18000 && !job.Completed; tick++)
            {
                world.Step(.05);
                carried |= World.Amount(reachable.Cargo, "fibre") == 5 && !reachable.Position.Equals(station.Position);
                worked |= job.Progress > 0 && job.Phase == "working" && reachable.Position.Equals(station.Position);
                Check(Math.Abs(TextileUnits(village) - 5) < 1e-8, "Reachable staffing production changed the five finite Fibre/Thread units");
            }
            Check(carried && worked && job.Completed && world.Total(village, "fibre") == 0 && world.Total(village, "thread") == 5, "Reachable staffing did not physically fetch, work and deliver exactly five Thread");
            Check(reachable.Cargo.Count == 0 && job.Local.Count == 0 && station.Inputs.Count == 0 && station.Outputs.Count == 0 && world.Reservations.Count == 0, "Completed reachable production left cargo, buffers or input claims");
            Check(blocked.JobId == "" && blocked.BuildingId == "" && village.Events.Count(e => e.Kind == "completed" && e.EntityId == job.Id) == 1, "Physical production took the blocked cat or completed the same batch twice");
            Valid(world);
        }

        static void ReachableRanking(bool tie)
        {
            var world = Fixture("clothier", out var village, out var station, out _, out var reachable);
            var other = village.Cats[3];
            reachable.Preferences.Add("textile");
            if (tie)
            {
                other.Preferences.Add("textile");
                World.Add(other.Skills, "textile", 1);
            }
            else
            {
                other.Boosted = true;
                World.Add(other.Skills, "textile", 30);
            }
            var expected = tie ? new[] { reachable, other }.OrderBy(c => c.Id, StringComparer.Ordinal).First() : reachable;
            world.Step(10);
            ExpectWorker(village, station, expected);
            Valid(world);
        }

        static void RouteRecovery()
        {
            var world = Fixture("clothier", out var village, out var station, out var blocked, out _);
            foreach (var cat in village.Cats.Where(c => c.ControlledBy == ""))
                Place(cat, blocked.Position);
            Act(world, village, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = "fibre_to_thread", Repeat = false });
            var queue = station.Queue;
            var order = queue.Single();
            world.Step(20);
            Check(station.Slots.Count == 0 && station.WorkerId == "" && ReferenceEquals(station.Queue, queue) && ReferenceEquals(station.Queue.Single(), order) && !order.Repeat && order.AutomatedBy == "", "An unreachable automatic candidate occupied the workplace or changed the exact manual queue");
            Check(village.Cats.Where(c => c.ControlledBy == "").All(c => c.JobId == "" && c.BuildingId == "" && c.Cargo.Count == 0) && world.Total(village, "fibre") == 5 && world.Reservations.Count == 0, "Unreachable automatic staffing changed finite goods, claims or work ownership");
            world.TileAt(new Int2(-3, 1)).Wall = false;
            Check(world.Path(blocked.Position, station.Position, village) != null, "Opening the fixture gate must restore its workplace route");
            world.Step(10);
            var slot = ExpectWorker(village, station, blocked);
            Check(ReferenceEquals(slot.Queue, queue) && ReferenceEquals(slot.Queue.Single(), order) && order.AutomatedBy == "" && !order.Repeat, "Route recovery replaced or relabeled the player's finite station queue");
            Valid(world);
        }

        static void RetainAssignedWorker(bool automatic)
        {
            var world = Fixture("clothier", out var village, out var station, out var blocked, out var reachable);
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = blocked.Id });
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = blocked.Id, Edit = "add", RecipeId = "fibre_to_thread", Repeat = true });
            var slot = station.Slots.Single();
            var order = slot.Queue.Single();
            if (automatic)
                slot.AutomatedBy = order.AutomatedBy = "cloth_leader";
            world.Step(20);
            Check(station.Slots.Count == 1 && ReferenceEquals(station.Slots.Single(), slot) && slot.CatId == blocked.Id && blocked.BuildingId == station.Id && station.WorkerId == blocked.Id, "Automatic reachability replaced an already assigned worker or its exact position");
            Check(ReferenceEquals(slot.Queue.Single(), order) && order.Repeat && slot.AutomatedBy == (automatic ? "cloth_leader" : "") && order.AutomatedBy == (automatic ? "cloth_leader" : "") && !slot.Paused, "Automatic reachability altered the retained worker's queue, pause or provenance");
            Check(reachable.BuildingId == "" && reachable.JobId == "" && blocked.JobId == "" && world.Total(village, "fibre") == 5 && world.Reservations.Count == 0, "Retaining unreachable assigned work moved finite inputs or stole the alternate cat");
            Valid(world);
        }

        static void ResearchWorkplace()
        {
            var world = Fixture("research_hut", out var village, out var station, out var blocked, out var reachable);
            Place(reachable, new Int2(3, 3));
            Check(!reachable.Position.Equals(station.Position) && world.Path(reachable.Position, station.Position, village) != null, "Research acceptance must start away from its reachable workplace");
            world.Step(10);
            ExpectWorker(village, station, reachable);
            double points = village.ResearchPoints, skill = World.Amount(reachable.Skills, "research");
            for (int second = 0; second < 60 && village.ResearchPoints == points; second++)
                world.Step(1);
            Check(village.ResearchPoints > points && World.Amount(reachable.Skills, "research") > skill && reachable.Position.Equals(station.Position), "The reachable nonrecipe researcher did not physically reach its workplace and advance real research");
            Check(blocked.BuildingId == "" && blocked.JobId == "", "Nonrecipe staffing assigned the blocked preferred researcher");
            Valid(world);
        }

        static void BlockedWorkplace()
        {
            var world = Fixture("research_hut", out var village, out var station, out var blocked, out var reachable);
            blocked.ControlledBy = "fixture-held";
            world.TileAt(station.Position).Wall = true;
            Check(reachable.Position.Equals(station.Position) && world.Path(reachable.Position, station.Position, village) != null && !world.Walkable(village, station.Position), "Blocked-workplace fixture must exercise Path's start-equals-end shortcut");
            double points = village.ResearchPoints;
            world.Step(20);
            Check(station.Slots.Count == 0 && station.WorkerId == "" && reachable.BuildingId == "" && village.ResearchPoints == points, "Automatic staffing treated an impassable workplace as reachable because a free cat already stood on its tile");
            Check(world.Total(village, "fibre") == 5 && world.Reservations.Count == 0, "Blocked-workplace refusal changed finite goods or input claims");
            Valid(world);
        }

        static World FarmFixture(out Village village, out Farm farm, out Cat blocked, out Cat reachable)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            var store = village.Stockpiles[0];
            store.Position = new Int2(3, -4);
            store.Width = store.Depth = 1;
            store.Capacity = 100000;
            store.Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000), new Stack("fibre", 10) };
            village.LastLeaderResearch = 0;
            var knownLogs = world.Tiles.First(t => t.Resource == "logs" && t.Amount > 0).Position;
            if (!village.Known.Contains(knownLogs)) village.Known.Add(knownLogs);
            for (int index = 0; index < village.Cats.Count; index++)
            {
                var cat = village.Cats[index];
                cat.ControlledBy = index >= 1 && index <= 3 || index >= 5 && index <= 8 ? "" : "fixture-held";
                cat.ControlLeaseUntil = 10000000;
                cat.Skills.Clear();
                cat.Preferences.Clear();
                cat.Boosted = false;
                cat.Hunger = cat.Thirst = cat.Rest = 100;
                Place(cat, new Int2(1, 3));
            }
            blocked = village.Cats[1];
            reachable = village.Cats[2];
            blocked.Preferences.Add("farm");
            World.Add(blocked.Skills, "farm", 50);
            World.Add(reachable.Skills, "farm", 1);
            Place(blocked, new Int2(-5, 1));
            foreach (var position in new[] { new Int2(-6, 1), new Int2(-5, 0), new Int2(-5, 2), new Int2(-4, 0), new Int2(-4, 2), new Int2(-3, 1) })
                world.TileAt(position).Wall = true;
            // Finite setup credit buys the real prerequisite chain before time starts.
            village.ResearchPoints = 23;
            foreach (string study in new[] { "research_hut", "water_carriers", "irrigation" })
                Act(world, village, new GameAction { Kind = "ResearchNode", NodeId = study });
            Check(village.ResearchPoints == 0 && world.TimeSeconds == 0, "Farm prerequisite research must spend its finite setup credit before the clock starts");
            village.Buildings.Add(new Building { Id = world.Id("farmer-staffing-field"), Kind = "field", Position = new Int2(4, 4), Width = 2, Depth = 2, Completed = true });
            Act(world, village, new GameAction { Kind = "DesignateFarm", Position = new Int2(10, 1), End = new Int2(10, 1), Resource = "grain" });
            farm = village.Farms.Single();
            Act(world, village, new GameAction { Kind = "AssignOfficer", Role = "farmer", CatId = village.Cats[0].Id });
            Check(world.Walkable(village, blocked.Position) && world.Walkable(village, farm.Position) && world.Path(blocked.Position, farm.Position, village) == null && world.Path(reachable.Position, farm.Position, village) != null && world.Path(farm.Position, farm.Handoff, village) != null, "Farm fixture needs a legal reachable exterior plot, physical handoff and separate walkable preferred-cat component");
            var plot = farm;
            var isolated = blocked;
            Check(!village.Buildings.Any(b => InFootprint(b.Position, b.Width, b.Depth, plot.Position) || InFootprint(b.Position, b.Width, b.Depth, plot.Handoff) || InFootprint(b.Position, b.Width, b.Depth, store.Position) || InFootprint(b.Position, b.Width, b.Depth, isolated.Position)), "Farm, handoff, supply store and isolated candidate must not overlap building footprints");
            Check(village.Jobs.Count == 0 && world.Reservations.Count == 0 && farm.WorkerId == "" && farm.Growth == 0 && farm.Harvest == 0, "Farm fixture must begin vacant without work, goods claims or crop credit");
            Valid(world);
            return world;
        }

        // Match Work.AvailableCats/FreeWorker, including obligations that do not
        // necessarily have a JobId. Needs and an office alone do not exclude cats.
        static IEnumerable<Cat> FarmerLaborAvailable(World world, Village village) => village.Cats.Where(c => c.Alive && c.AgeHours >= 12
            && !world.IsDungeonExplorer(c) && c.Position.Level == 0 && c.JobId == "" && c.BuildingId == "" && c.ControlledBy == ""
            && c.Migration != "arriving" && c.Migration != "departing" && !village.Routes.Any(r => r.CatId == c.Id));

        static World FarmerLaborFixture(bool kittens, out Village village, out Building mill)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            var store = village.Stockpiles[0];
            store.Position = new Int2(3, -4);
            store.Width = store.Depth = 1;
            store.Capacity = 100000;
            store.Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000), new Stack("fibre", 10), new Stack("grain", 4) };
            village.LastLeaderResearch = 0;
            for (int index = 0; index < village.Cats.Count; index++)
            {
                var cat = village.Cats[index];
                cat.AgeHours = kittens && index >= 4 ? 1 : 24;
                cat.Hunger = cat.Thirst = cat.Rest = cat.Health = 100;
                cat.Skills.Clear();
                cat.Preferences.Clear();
                cat.Boosted = false;
                Place(cat, new Int2(3, 0));
            }
            // Authored support ground, not crop/work credit. Preserve the village's
            // claims, walls and east gate; every new plot is publicly designated.
            for (int x = 10; x <= 23; x++)
                for (int z = -7; z <= 5; z++)
                {
                    var tile = world.TileAt(new Int2(x, z));
                    Check(tile.ClaimId == "", "Labor fixture exterior ground must be unclaimed");
                    World.ClearSurface(tile, 0);
                    tile.Wall = tile.Road = tile.Dirt = tile.Rail = tile.Bridge = tile.Dock = false;
                    tile.Overlay = "";
                    if (!village.Known.Contains(tile.Position)) village.Known.Add(tile.Position);
                }
            var knownLogs = world.Tiles.First(t => t.Resource == "logs" && t.Amount > 0).Position;
            if (!village.Known.Contains(knownLogs)) village.Known.Add(knownLogs);
            string[] studies = { "research_hut", "basic_tools", "foraging_lore", "sawmill", "masonry", "water_carriers", "irrigation", "milling", "grain_milling_sources", "grain_milling_preparation" };
            village.ResearchPoints = studies.Sum(id => Catalog.Study(id).Cost);
            foreach (string study in studies)
                Act(world, village, new GameAction { Kind = "ResearchNode", NodeId = study });
            Check(village.ResearchPoints == 0 && world.TimeSeconds == 0 && Catalog.BuildingAvailable(village, "mill") && Catalog.RecipeAvailable(village, Catalog.Recipe("grain_to_flour")), "Finite public research must unlock and fund the actual Mill recipe before time starts");
            village.Buildings.Add(new Building { Id = world.Id("farmer-labor-field"), Kind = "field", Position = new Int2(4, 4), Completed = true });
            mill = new Building { Id = world.Id("farmer-labor-mill"), Kind = "mill", Position = new Int2(4, -4), Completed = true };
            village.Buildings.Add(mill);
            for (int x = 12; x <= 22; x += 2)
                for (int z = -6; z <= 4; z += 2)
                    Act(world, village, new GameAction { Kind = "DesignateFarm", Position = new Int2(x, z), End = new Int2(x, z), Resource = "grain" });
            Act(world, village, new GameAction { Kind = "SetCatLaborPreference", CatId = village.Cats[1].Id, Labor = "mill", Enabled = true });
            Act(world, village, new GameAction { Kind = "AssignOfficer", Role = "farmer", CatId = village.Cats[0].Id });
            var owner = village;
            Check(village.Cats.Count == 30 && village.Cats.All(c => c.Alive) && FarmerLaborAvailable(world, village).Count() == (kittens ? 4 : 30), "Labor fixture must start with thirty living cats and the exact eligible adult workforce");
            Check(village.Farms.Count == 36 && village.Farms.All(f => f.Width == 1 && f.Depth == 1 && f.WorkerId == "" && f.Growth == 0 && f.Harvest == 0
                && world.Walkable(owner, f.Position) && world.Path(owner.Center, f.Position, owner) != null && world.Path(f.Position, f.Handoff, owner) != null), "Labor fixture needs thirty-six legal, reachable, vacant exterior plots without crop credit");
            Check(world.Path(village.Cats[1].Position, mill.Position, village) != null && world.Path(store.Position, mill.Position, village) != null
                && mill.Slots.Count == 0 && village.Jobs.Count == 0 && world.Reservations.Count == 0 && village.Cats.All(c => c.JobId == "" && c.BuildingId == "" && c.Cargo.Count == 0), "Labor fixture must have a reachable funded Mill without injected workers, jobs, claims or cargo");
            Valid(world);
            return world;
        }

        static void FarmerLaborOwnership(World world, Village village, Building mill)
        {
            var farms = village.Farms.Where(f => f.WorkerId != "").ToArray();
            var slots = mill.Slots.Where(s => s.CatId != "").ToArray();
            string[] owners = farms.Select(f => f.WorkerId).Concat(slots.Select(s => s.CatId)).ToArray();
            Check(owners.Distinct().Count() == owners.Length, "Farmer duplicated a cat across plots or Mill positions");
            Check(slots.Length <= 1 && mill.ExtraWorkerIds.Count == 0 && mill.WorkerId == (slots.Length == 0 ? "" : slots[0].CatId), "Mill's primary/extra ownership markers do not match its exact researched position");
            foreach (var farm in farms)
            {
                var cat = village.Cats.Single(c => c.Id == farm.WorkerId);
                Check(cat.Alive && cat.AgeHours >= 12 && cat.BuildingId == farm.Id && cat.JobId == "" && cat.ControlledBy == "" && farm.AutomatedBy == "farmer"
                    && !village.Jobs.Any(j => !j.Completed && j.CatId == cat.Id) && !village.Routes.Any(r => r.CatId == cat.Id), "Farm lost its exact eligible cat or duplicated a job/route commitment");
            }
            foreach (var slot in slots)
            {
                var cat = village.Cats.Single(c => c.Id == slot.CatId);
                Check(cat.BuildingId == mill.Id && mill.WorkerId == cat.Id && slot.AutomatedBy == "farmer" && !village.Routes.Any(r => r.CatId == cat.Id), "Mill lost its exact cat, building or Farmer ownership");
            }
            foreach (var cat in village.Cats)
                Check(cat.BuildingId == (farms.FirstOrDefault(f => f.WorkerId == cat.Id)?.Id ?? (slots.Any(s => s.CatId == cat.Id) ? mill.Id : "")), "A cat retained a workplace without its reciprocal exact assignment");
            Check(village.Buildings.Where(b => b.Id != mill.Id).All(b => b.WorkerId == "" && b.ExtraWorkerIds.Count == 0 && b.Slots.All(s => s.CatId == "")), "Farmer claimed an unrelated workplace");
            Valid(world);
        }

        static void ExpectFarmerLaborAllocation(World world, Village village, Building mill, bool kittens)
        {
            FarmerLaborOwnership(world, village, mill);
            Check(world.TimeSeconds == 10 && world.Total(village, "grain") == 4 && FarmGrain(village) == 4 && world.Total(village, "flour") == 0
                && world.Total(village, "fibre") == 10 && world.Total(village, "food") == 1000 && world.Total(village, "water") == 1000
                && village.Farms.All(f => f.Growth == 0 && f.Harvest == 0) && village.Cats.All(c => c.Cargo.Count == 0)
                && village.Jobs.Count == 0 && world.Reservations.Count == 0, "Allocation moved finite supplies, created work/claims or credited a crop before physical pickup");
            int reserve = Math.Max(2, (int)Math.Ceiling(village.Cats.Count(c => c.Alive) * 2.0 / 15));
            int free = FarmerLaborAvailable(world, village).Count(), farms = village.Farms.Count(f => f.WorkerId != ""), mills = mill.Slots.Count(s => s.CatId != "");
            Check(reserve == 4 && free == reserve && farms == (kittens ? 0 : 25) && mills == (kittens ? 0 : 1),
                "Farmer allocation must leave four genuinely eligible free adults and " + (kittens ? "no new worker at reserve" : "one funded Mill worker plus twenty-five plot workers") + "; actual free=" + free + ", plots=" + farms + ", Mill=" + mills);
            if (!kittens)
                Check(mill.Slots.Single(s => s.CatId != "").CatId == village.Cats[1].Id, "Farmer did not reserve the exact reachable Mill-preferred adult before allocating plots");
        }

        static void FarmerLaborAllocation(bool kittens)
        {
            var world = FarmerLaborFixture(kittens, out var village, out var mill);
            world.Step(10);
            ExpectFarmerLaborAllocation(world, village, mill, kittens);
        }

        static void FarmerLaborChain()
        {
            var world = FarmerLaborFixture(false, out var village, out var mill);
            world.Step(10);
            ExpectFarmerLaborAllocation(world, village, mill, false);
            var recipe = Catalog.Recipe("grain_to_flour");
            double expectedOutput = world.RecipeOutput(village, recipe, recipe.Outputs.Single(s => s.Resource == "flour").Amount);
            var miller = village.Cats[1];
            var plots = village.Farms.Where(f => f.WorkerId != "").ToDictionary(f => f.Id, f => f.WorkerId);
            Job batch = null;
            bool pickedUp = false, worked = false, carriedFlour = false;
            for (int tick = 0; tick < 24000 && (batch == null || !batch.Completed); tick++)
            {
                // Observe every real pickup and delivery at the ordinary actor quantum.
                world.Step(World.SimulationStepSeconds);
                batch = batch ?? village.Jobs.SingleOrDefault(j => j.TargetId == mill.Id && j.RecipeId == "grain_to_flour");
                if (!pickedUp && World.Amount(miller.Cargo, "grain") > 0)
                {
                    Check(miller.Position.Equals(village.Stockpiles[0].Position) && world.Total(village, "grain") == 0 && World.Amount(miller.Cargo, "grain") == 4, "Finite Grain left storage before the exact miller's physical pickup");
                    pickedUp = true;
                }
                worked |= batch != null && batch.Progress > 0 && miller.Position.Equals(mill.Position);
                carriedFlour |= World.Amount(miller.Cargo, "flour") > 0;
                double grain = FarmGrain(village) + village.Jobs.Sum(j => World.Amount(j.Local, "grain")) + village.Buildings.Sum(b => World.Amount(b.Inputs, "grain"));
                double flour = village.Stockpiles.Sum(p => World.Amount(p.Goods, "flour")) + village.Cats.Sum(c => World.Amount(c.Cargo, "flour"))
                    + village.Jobs.Sum(j => World.Amount(j.Local, "flour")) + village.Buildings.Sum(b => World.Amount(b.Outputs, "flour"));
                Check(Math.Abs(grain + (4 / expectedOutput) * flour - 4) < 1e-8, "Physical Mill chain lost or duplicated its finite four-Grain equivalent");
                Check(village.Farms.All(f => f.Harvest == 0 && f.WorkerId == (plots.TryGetValue(f.Id, out var workerId) ? workerId : "")), "Mill processing displaced plot owners or received injected/new harvest inputs");
            }
            Check(batch != null && batch.Completed && batch.RequiredWork == 600 && batch.CatId == miller.Id && pickedUp && worked && carriedFlour
                && miller.Position.Equals(village.Stockpiles[0].Position) && world.Total(village, "flour") == expectedOutput && world.Total(village, "grain") == 0,
                "Farmer must physically pick up four Grain, complete 600 Mill work and deliver exactly " + expectedOutput + " Flour while plots stay staffed");
            Check(village.Farms.Where(f => plots.ContainsKey(f.Id)).All(f => f.Growth > 0 && village.Cats.Single(c => c.Id == f.WorkerId).Position.Equals(f.Position))
                && world.Reservations.Count == 0 && miller.Cargo.Count == 0, "Composed food processing lost real farm work or retained its released finite input claims");
            FarmerLaborOwnership(world, village, mill);
        }

        static bool InFootprint(Int2 position, int width, int depth, Int2 at) => position.Level == at.Level && at.X >= position.X && at.X < position.X + width && at.Z >= position.Z && at.Z < position.Z + depth;

        static void ExpectFarmWorker(World world, Village village, Farm farm, Cat expected, string provenance = "farmer")
        {
            Check(farm.WorkerId == expected.Id, "Farmer assigned an unreachable preferred cat instead of the exact reachable ranked worker");
            Check(farm.AutomatedBy == provenance && expected.BuildingId == farm.Id && expected.JobId == "" && expected.ControlledBy == "", "Farm staffing lost the exact cat, plot or officer ownership");
            string farmId = farm.Id;
            Check(village.Cats.Count(c => c.BuildingId == farmId) == 1 && village.Farms.Count(f => f.WorkerId == expected.Id) == 1 && !village.Buildings.Any(b => b.WorkerId == expected.Id || b.ExtraWorkerIds.Contains(expected.Id) || b.Slots.Any(s => s.CatId == expected.Id)) && !village.Jobs.Any(j => !j.Completed && j.CatId == expected.Id) && !village.Routes.Any(r => r.CatId == expected.Id), "Farm staffing duplicated its worker across plots, stations, jobs or routes");
            Check(world.Reservations.Count == 0, "Farm staffing created unrelated resource reservations");
        }

        static void FarmRankedFallback()
        {
            var world = FarmFixture(out var village, out var farm, out var blocked, out var reachable);
            world.Step(10);
            ExpectFarmWorker(world, village, farm, reachable);
            Check(blocked.BuildingId == "" && blocked.JobId == "" && blocked.Cargo.Count == 0 && blocked.Preferences.Contains("farm") && World.Amount(blocked.Skills, "farm") == 50, "Vacant farm staffing changed its blocked preferred cat");
            Check(farm.Growth == 0 && farm.Harvest == 0 && reachable.Cargo.Count == 0 && world.Total(village, "grain") == 0 && world.Total(village, "fibre") == 10 && world.Total(village, "food") == 1000 && world.Total(village, "water") == 1000 && village.Jobs.Count == 0, "Selecting a farm worker moved supplies, grew or delivered crop, or created unrelated labor");
            Valid(world);
        }

        static void FarmRanking(bool tie)
        {
            var world = FarmFixture(out var village, out var farm, out _, out var reachable);
            var other = village.Cats[3];
            reachable.Preferences.Add("farm");
            if (tie)
            {
                other.Preferences.Add("farm");
                World.Add(other.Skills, "farm", 1);
                village.Cats.Reverse();
            }
            else
            {
                other.Boosted = true;
                World.Add(other.Skills, "farm", 30);
            }
            var expected = tie ? new[] { reachable, other }.OrderBy(c => c.Id, StringComparer.Ordinal).First() : reachable;
            world.Step(10);
            ExpectFarmWorker(world, village, farm, expected);
            Valid(world);
        }

        static void FarmRouteRecovery()
        {
            var world = FarmFixture(out var village, out var farm, out var blocked, out _);
            foreach (var cat in village.Cats.Where(c => c.ControlledBy == ""))
                Place(cat, blocked.Position);
            world.Step(20);
            Check(farm.WorkerId == "" && farm.AutomatedBy == "" && farm.Growth == 0 && farm.Harvest == 0 && farm.BlockedReason == "worker_required", "Farmer claimed an unreachable vacant plot instead of leaving it available for a future retry");
            Check(village.Cats.Where(c => c.ControlledBy == "").All(c => c.JobId == "" && c.BuildingId == "" && c.Cargo.Count == 0) && village.Jobs.Count == 0 && world.Reservations.Count == 0 && world.Total(village, "fibre") == 10, "Unreachable farm candidates acquired work, cargo or input claims");
            world.TileAt(new Int2(-3, 1)).Wall = false;
            Check(world.Path(blocked.Position, farm.Position, village) != null, "Opening the isolated component must restore a real route to the same farm");
            world.Step(10);
            ExpectFarmWorker(world, village, farm, blocked);
            Check(farm.Growth == 0 && farm.Harvest == 0, "Reopening access grew a crop before the newly assigned farmer traveled to the plot");
            Valid(world);
        }

        static void FarmRetainAssignedWorker(bool automatic)
        {
            var world = FarmFixture(out var village, out var farm, out var blocked, out var reachable);
            if (automatic)
            {
                var candidates = village.Cats.Where(c => c.ControlledBy == "").ToArray();
                foreach (var cat in candidates) cat.ControlledBy = "fixture-held";
                world.Step(9.95);
                foreach (var cat in candidates) cat.ControlledBy = "";
                world.TileAt(new Int2(-3, 1)).Wall = false;
                world.Step(.05);
                ExpectFarmWorker(world, village, farm, blocked);
                world.TileAt(new Int2(-3, 1)).Wall = true;
            }
            else
                Act(world, village, new GameAction { Kind = "AssignWorker", TargetId = farm.Id, CatId = blocked.Id });
            string provenance = automatic ? "farmer" : "";
            Check(world.Path(blocked.Position, farm.Position, village) == null && world.Path(reachable.Position, farm.Position, village) != null, "Retained farm worker needs closed access while its alternate remains reachable");
            world.Step(20);
            ExpectFarmWorker(world, village, farm, blocked, provenance);
            Check(reachable.BuildingId == "" && reachable.JobId == "" && blocked.Cargo.Count == 0 && farm.Growth == 0 && farm.Harvest == 0 && farm.BlockedReason == "blocked_route", "Vacant-only farm staffing replaced assigned ownership, stole an alternate or grew unreachable crop");
            Valid(world);
        }

        static void FarmBlockedPlot()
        {
            var world = FarmFixture(out var village, out var farm, out var blocked, out var reachable);
            var candidates = village.Cats.Where(c => c.ControlledBy == "").ToArray();
            foreach (var cat in candidates) cat.ControlledBy = "fixture-held";
            world.Step(9.95);
            foreach (var cat in candidates) cat.ControlledBy = "";
            Place(reachable, farm.Position);
            world.TileAt(farm.Position).Wall = true;
            Check(world.Path(reachable.Position, farm.Position, village) != null && !world.Walkable(village, farm.Position), "Blocked plot must exercise Path's start-equals-end shortcut");
            world.Step(.05);
            Check(farm.WorkerId == "" && farm.AutomatedBy == "" && reachable.BuildingId == "" && farm.Growth == 0 && farm.Harvest == 0 && blocked.BuildingId == "", "Farmer treated an impassable plot as reachable because an available cat stood on its tile");
            Check(village.Jobs.Count == 0 && world.Reservations.Count == 0 && world.Total(village, "fibre") == 10, "Impassable plot refusal changed supplies or claims");
            Valid(world);
        }

        static void FarmManualAssignment(bool sameWorker)
        {
            var world = FarmFixture(out var village, out var farm, out var blocked, out var automaticWorker);
            Act(world, village, new GameAction { Kind = "DesignateFarm", Position = new Int2(10, 3), End = new Int2(10, 3), Resource = "grain" });
            var other = village.Farms.Single(f => f.Id != farm.Id);
            var otherWorker = village.Cats[3];
            world.Step(10);
            // Both ownership markers must come from the real Farmer planner,
            // rather than authored provenance that only resembles its output.
            ExpectFarmWorker(world, village, farm, automaticWorker);
            ExpectFarmWorker(world, village, other, otherWorker);
            Check(!farm.Position.Equals(other.Position) && farm.Growth == 0 && other.Growth == 0 && farm.Harvest == 0 && other.Harvest == 0, "Manual farm takeover must start with two distinct, genuinely officer-staffed plots without crop credit");
            var manualWorker = sameWorker ? automaticWorker : village.Cats[4];
            if (!sameWorker)
            {
                manualWorker.ControlledBy = "";
                Place(manualWorker, new Int2(1, 3));
            }
            string otherPhase = other.Phase, otherBlocker = other.BlockedReason;
            double otherGrowth = other.Growth, otherHarvest = other.Harvest, beforeTime = world.TimeSeconds;
            Act(world, village, new GameAction { Kind = "AssignWorker", TargetId = farm.Id, CatId = manualWorker.Id });
            Check(farm.AutomatedBy == "", "Explicit manual farm assignment retained Farmer provenance and can be canceled by vacating that office");
            ExpectFarmWorker(world, village, farm, manualWorker, "");
            ExpectFarmWorker(world, village, other, otherWorker);
            Check(other.Phase == otherPhase && other.BlockedReason == otherBlocker && other.Growth == otherGrowth && other.Harvest == otherHarvest && world.TimeSeconds == beforeTime, "Taking manual ownership changed the other exact plot or advanced simulation time");
            Check(sameWorker || automaticWorker.BuildingId == "" && automaticWorker.JobId == "" && automaticWorker.Cargo.Count == 0, "Replacing an officer farm worker left its previous owner attached to work or cargo");
            Check(farm.Growth == 0 && farm.Harvest == 0 && village.Jobs.Count == 0 && world.Total(village, "grain") == 0 && world.Total(village, "fibre") == 10 && world.Total(village, "food") == 1000 && world.Total(village, "water") == 1000, "Manual farm takeover grew crop, created labor or changed finite supplies");
            Act(world, village, new GameAction { Kind = "UnassignOfficer", Role = "farmer" });
            ExpectFarmWorker(world, village, farm, manualWorker, "");
            Check(!village.Officers.Any(o => o.Role == "farmer") && other.WorkerId == "" && other.AutomatedBy == "" && otherWorker.BuildingId == "", "Vacating Farmer must release the other officer-owned plot while retaining the exact manual assignment");
            world.Step(10);
            ExpectFarmWorker(world, village, farm, manualWorker, "");
            Check(other.WorkerId == "" && other.AutomatedBy == "" && other.Growth == otherGrowth && other.Harvest == otherHarvest && blocked.BuildingId == "" && blocked.JobId == "", "Manual farm ownership did not survive normal planning after Farmer was vacated");
            Valid(world);
        }

        static double FarmGrain(Village village) => village.Farms.Sum(f => f.Crop == "grain" ? f.Harvest : 0) + village.Stockpiles.Sum(p => World.Amount(p.Goods, "grain")) + village.Cats.Sum(c => World.Amount(c.Cargo, "grain"));

        static string StaleWorkSnapshot(World world)
        {
            var result = new System.Text.StringBuilder();
            void Append(object value)
            {
                if (value == null) { result.Append("null;"); return; }
                if (value is string text) { result.Append(text.Length).Append(':').Append(text).Append(';'); return; }
                if (value is double number) { result.Append(number.ToString("R", System.Globalization.CultureInfo.InvariantCulture)).Append(';'); return; }
                if (value.GetType().IsPrimitive) { result.Append(Convert.ToString(value, System.Globalization.CultureInfo.InvariantCulture)).Append(';'); return; }
                if (value is System.Collections.IEnumerable sequence) { result.Append('['); foreach (var entry in sequence) Append(entry); result.Append(']'); return; }
                result.Append('{');
                foreach (var field in value.GetType().GetFields(System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.Public).OrderBy(f => f.Name, StringComparer.Ordinal))
                {
                    result.Append(field.Name).Append('=');
                    Append(field.GetValue(value));
                }
                result.Append('}');
            }
            Append(world);
            return result.ToString();
        }

        static void StaleWorkRefused(World world, Village village, GameAction action, string error)
        {
            string before = StaleWorkSnapshot(world);
            var result = world.Apply(new PlayerContext { PlayerId = "reachable-staffing-acceptance", VillageId = village.Id }, action);
            Check(!result.Success && result.Error == error, "Missing explicit workplace must refuse with " + error + ", received success=" + result.Success + " error=" + result.Error);
            Check(StaleWorkSnapshot(world) == before, "Rejected assignment changed complete authoritative state, including current work, finite goods, claims or identity allocation");
            Valid(world);
        }

        static void StaleWorkClearedFarm(bool redesignate)
        {
            var world = FarmFixture(out var village, out var removed, out _, out var worker);
            Act(world, village, new GameAction { Kind = "UnassignOfficer", Role = "farmer" });
            Act(world, village, new GameAction { Kind = "DesignateFarm", Position = new Int2(10, 3), End = new Int2(10, 3), Resource = "grain" });
            var surviving = village.Farms.Single(f => f.Id != removed.Id);
            Act(world, village, new GameAction { Kind = "ClearFarm", TargetId = removed.Id });
            Farm replacement = null;
            if (redesignate)
            {
                Act(world, village, new GameAction { Kind = "DesignateFarm", Position = removed.Position, End = removed.Position, Resource = removed.Crop });
                replacement = village.Farms.Single(f => f.Position.Equals(removed.Position));
                Check(replacement.Id != removed.Id && replacement.WorkerId == "", "Redesignating identical coordinates must create a distinct vacant plot");
            }
            Act(world, village, new GameAction { Kind = "AssignWorker", TargetId = surviving.Id, CatId = worker.Id });
            ExpectFarmWorker(world, village, surviving, worker, "");
            StaleWorkRefused(world, village, new GameAction { Kind = "AssignWorker", TargetId = removed.Id, CatId = worker.Id }, "Unknown farm");
            ExpectFarmWorker(world, village, surviving, worker, "");
            Check(replacement == null || replacement.WorkerId == "", "An old plot identity assigned its replacement at the same coordinates");
            // No work credit: the retained public assignment must still walk
            // to its own plot and begin real growth after the rejected command.
            for (int tick = 0; tick < 240 && surviving.Growth == 0; tick++) world.Step(.25);
            Check(surviving.Growth > 0 && worker.Position.Equals(surviving.Position) && surviving.Harvest == 0 && world.Total(village, "grain") == 0, "Rejected stale assignment interrupted the surviving plot's physical approach or uncredited growth");
            ExpectFarmWorker(world, village, surviving, worker, "");
            Valid(world);
        }

        static void StaleWorkMissingBuilding()
        {
            var world = Fixture("clothier", out var village, out var station, out _, out var worker);
            world.Step(11);
            ExpectWorker(village, station, worker);
            var job = village.Jobs.Single(j => !j.Completed && j.Kind == "production" && j.CatId == worker.Id);
            Check(job.RecipeId == "fibre_to_thread" && worker.JobId == job.Id && (world.Reservations.Count > 0 || worker.Cargo.Count > 0 || job.Local.Count > 0), "Missing-building refusal needs a real funded batch with finite goods or claims");
            StaleWorkRefused(world, village, new GameAction { Kind = "AssignWorker", BuildingId = station.Id + "-removed", CatId = worker.Id }, "Unknown building");
            Check(worker.JobId == job.Id && !job.Completed && TextileUnits(village) == 5, "Rejected missing building canceled or changed the funded production batch");
        }

        static void StaleWorkMixedDestination()
        {
            var world = FarmFixture(out var village, out var first, out _, out var worker);
            Act(world, village, new GameAction { Kind = "UnassignOfficer", Role = "farmer" });
            Act(world, village, new GameAction { Kind = "DesignateFarm", Position = new Int2(10, 3), End = new Int2(10, 3), Resource = "grain" });
            var second = village.Farms.Single(f => f.Id != first.Id);
            var other = village.Cats[3];
            Act(world, village, new GameAction { Kind = "AssignWorker", TargetId = first.Id, CatId = worker.Id });
            Act(world, village, new GameAction { Kind = "AssignWorker", TargetId = second.Id, CatId = other.Id });
            StaleWorkRefused(world, village, new GameAction { Kind = "AssignWorker", BuildingId = "missing-preferred-building", TargetId = second.Id, CatId = worker.Id }, "Unknown building");
            ExpectFarmWorker(world, village, first, worker, "");
            ExpectFarmWorker(world, village, second, other, "");
        }

        static Building StaleWorkResearchStation(World world, Village village)
        {
            var station = new Building { Id = world.Id("stale-work-research"), Kind = "research_hut", Position = new Int2(1, 3), Completed = true };
            village.Buildings.Add(station);
            return station;
        }

        static void StaleWorkExistingDestinations()
        {
            var world = FarmFixture(out var village, out var farm, out _, out var worker);
            Act(world, village, new GameAction { Kind = "UnassignOfficer", Role = "farmer" });
            var station = StaleWorkResearchStation(world, village);
            Act(world, village, new GameAction { Kind = "AssignWorker", TargetId = farm.Id, CatId = worker.Id });
            ExpectFarmWorker(world, village, farm, worker, "");
            Act(world, village, new GameAction { Kind = "AssignWorker", TargetId = station.Id, CatId = worker.Id });
            Check(farm.WorkerId == "" && worker.BuildingId == station.Id && station.WorkerId == worker.Id && station.Slots.Single().CatId == worker.Id, "An existing TargetId building failed to replace the exact farm assignment");
            var other = village.Cats[3];
            Act(world, village, new GameAction { Kind = "AssignWorker", TargetId = farm.Id, CatId = other.Id });
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, TargetId = farm.Id, CatId = worker.Id });
            Check(worker.BuildingId == station.Id && station.WorkerId == worker.Id && station.Slots.Single().CatId == worker.Id && farm.WorkerId == other.Id && other.BuildingId == farm.Id, "An existing preferred BuildingId lost precedence to the incidental TargetId farm");
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, TargetId = "missing-incidental-target", CatId = worker.Id });
            Check(worker.BuildingId == station.Id && station.Slots.Single().CatId == worker.Id && farm.WorkerId == other.Id && world.TimeSeconds == 0 && village.Jobs.Count == 0 && world.Reservations.Count == 0, "An incidental missing TargetId blocked a valid preferred building or changed unrelated work");
            Valid(world);
        }

        static void StaleWorkIntentionalRelease()
        {
            var world = FarmFixture(out var village, out var farm, out _, out var worker);
            Act(world, village, new GameAction { Kind = "UnassignOfficer", Role = "farmer" });
            Act(world, village, new GameAction { Kind = "AssignWorker", TargetId = farm.Id, CatId = worker.Id });
            Act(world, village, new GameAction { Kind = "AssignWorker", CatId = worker.Id });
            Check(farm.WorkerId == "" && worker.BuildingId == "" && worker.JobId == "" && worker.ResumeJobId == "", "Destinationless release failed to release the exact farm worker");
            var station = StaleWorkResearchStation(world, village);
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = worker.Id });
            Act(world, village, new GameAction { Kind = "AssignWorker", CatId = worker.Id });
            Check(station.WorkerId == "" && station.Slots.Single().CatId == "" && worker.BuildingId == "" && worker.JobId == "" && worker.ResumeJobId == "" && worker.Cargo.Count == 0, "Destinationless release failed to release the exact staffed workplace");
            Check(world.TimeSeconds == 0 && world.Total(village, "food") == 1000 && world.Total(village, "water") == 1000 && world.Total(village, "fibre") == 10 && village.Jobs.Count == 0 && world.Reservations.Count == 0, "Intentional release changed finite provisions, time, unrelated work or claims");
            Valid(world);
        }

        static string FarmTravelWarning(string leg) => leg == "plot" ? "blocked_route" : leg == "harvest" ? "blocked_harvest_route" : "blocked_handoff_route";
        static Int2 FarmTravelTarget(Farm farm, string leg) => leg == "handoff" ? farm.Handoff : farm.Position;
        static bool FarmTravelMoved(Cat cat, double x, double z) => Math.Abs(cat.X - x) + Math.Abs(cat.Z - z) > 1e-8;

        static World FarmTravelFixture(string leg, out Village village, out Farm farm, out Cat worker)
        {
            var world = FarmFixture(out village, out farm, out _, out worker);
            Act(world, village, new GameAction { Kind = "UnassignOfficer", Role = "farmer" });
            foreach (var cat in village.Cats) cat.ControlledBy = "fixture-held";
            worker.ControlledBy = "";
            Act(world, village, new GameAction { Kind = "AssignWorker", TargetId = farm.Id, CatId = worker.Id });
            if (leg == "plot") return world;
            // Finite remaining-work credit only. Ordinary movement, work, crop
            // completion and pickup create the real eight- and 1.6-Grain loads.
            farm.Growth = farm.CycleSeconds - 1;
            string farmId = farm.Id;
            for (int tick = 0; tick < 1600; tick++)
            {
                world.Step(World.SimulationStepSeconds);
                if (leg == "handoff" && World.Amount(worker.Cargo, "grain") > 0 || leg == "harvest" && village.Stockpiles.Any(p => p.ManagedBy == farmId && World.Amount(p.Goods, "grain") > 0))
                {
                    Check(Math.Abs(FarmGrain(village) - 9.6) < 1e-8 && farm.Growth == 0 && farm.Harvest > 0, "Travel fixture must physically finish one credited crop and retain its finite remainder");
                    Check(leg == "handoff" ? worker.Position.Equals(farm.Position) && World.Amount(worker.Cargo, "grain") == 8 : worker.Position.Equals(farm.Handoff) && worker.Cargo.Count == 0 && Math.Abs(farm.Harvest - 1.6) < 1e-8, "Travel fixture skipped physical pickup or first handoff");
                    Valid(world);
                    return world;
                }
            }
            throw new InvalidOperationException("Travel fixture did not physically reach its requested crop leg");
        }

        static void FarmTravelConserved(World world, Village village, Farm farm, Cat worker, string leg)
        {
            ExpectFarmWorker(world, village, farm, worker, "");
            Check(Math.Abs(FarmGrain(village) - (leg == "plot" ? 0 : 9.6)) < 1e-8 && world.Total(village, "grain") == 0 && village.Jobs.Count == 0, "Farm travel changed finite crop, general storage, ownership or unrelated work");
            Valid(world);
        }

        static void FinishFarmTravel(World world, Village village, Farm farm, Cat worker, string leg)
        {
            Int2 target = FarmTravelTarget(farm, leg);
            for (int tick = 0; tick < 1200 && !worker.Position.Equals(target); tick++)
            {
                world.Step(World.SimulationStepSeconds);
                Check(farm.BlockedReason == "", "Open physical travel retained a farm route warning before arrival");
                FarmTravelConserved(world, village, farm, worker, leg);
            }
            Check(worker.Position.Equals(target) && Math.Abs(worker.X - target.X) + Math.Abs(worker.Z - target.Z) < 1e-8 && farm.BlockedReason == "", "Farm worker did not arrive physically with clear route feedback");
            if (leg == "plot" && farm.Growth == 0) world.Step(World.SimulationStepSeconds);
            Check(leg == "plot" ? farm.Growth > 0 && farm.Harvest == 0 && worker.Cargo.Count == 0 : leg == "harvest" ? farm.Harvest == 0 && Math.Abs(World.Amount(worker.Cargo, "grain") - 1.6) < 1e-8 : worker.Cargo.Count == 0 && Math.Abs(village.Stockpiles.Where(p => p.ManagedBy == farm.Id).Sum(p => World.Amount(p.Goods, "grain")) - 8) < 1e-8, "Arrival failed to work, pick up or hand off the exact physical crop leg");
            FarmTravelConserved(world, village, farm, worker, leg);
        }

        static void FarmTravelOpen(string leg)
        {
            var world = FarmTravelFixture(leg, out var village, out var farm, out var worker);
            double x = worker.X, z = worker.Z, growth = farm.Growth, harvest = farm.Harvest, cargo = World.Amount(worker.Cargo, "grain");
            var start = worker.Position;
            Check(world.Path(start, FarmTravelTarget(farm, leg), village)?.Count > 0, "Open travel needs a real nonempty route");
            world.Step(World.SimulationStepSeconds);
            Check(FarmTravelMoved(worker, x, z) && worker.Position.Equals(start), "Open farm travel must physically advance part of an ordinary edge without completing even its first edge in one simulation step");
            Check(farm.BlockedReason == "", "Successful partial " + leg + " travel falsely reports " + farm.BlockedReason);
            Check(farm.Growth == growth && farm.Harvest == harvest && World.Amount(worker.Cargo, "grain") == cargo, "Partial farm travel grew, picked up or delivered crop before arrival");
            FinishFarmTravel(world, village, farm, worker, leg);
        }

        static void FarmTravelRecovery(string leg)
        {
            var world = FarmTravelFixture(leg, out var village, out var farm, out var worker);
            Int2 target = FarmTravelTarget(farm, leg);
            double x = worker.X, z = worker.Z, growth = farm.Growth, harvest = farm.Harvest, cargo = World.Amount(worker.Cargo, "grain");
            // The chosen leg has just begun at an exact physical endpoint with
            // no cached path, so closing its target genuinely closes all routes.
            Check(worker.Path.Count == 0 && !worker.Position.Equals(target), "Recovery fixture needs an unstarted physical leg");
            world.TileAt(target).Wall = true;
            Check(world.Path(worker.Position, target, village) == null, "Target closure must block every route");
            world.Step(World.SimulationStepSeconds);
            Check(farm.BlockedReason == FarmTravelWarning(leg) && worker.BlockedReason == "blocked_route" && !FarmTravelMoved(worker, x, z), "Genuine " + leg + " obstruction lost its route warning or moved its worker");
            Check(farm.Growth == growth && farm.Harvest == harvest && World.Amount(worker.Cargo, "grain") == cargo, "Blocked farm travel changed finite work or crop");
            double retryAt = worker.NextPathAttemptAt;
            world.TileAt(target).Wall = false;
            bool resumed = false;
            for (int tick = 0; tick < 30 && !resumed; tick++)
            {
                world.Step(World.SimulationStepSeconds);
                resumed = FarmTravelMoved(worker, x, z);
                Check(farm.Growth == growth && farm.Harvest == harvest && World.Amount(worker.Cargo, "grain") == cargo, "Route reopening skipped physical travel before arrival");
                if (world.TimeSeconds + 1e-9 < retryAt)
                    Check(!resumed && farm.BlockedReason == FarmTravelWarning(leg), "An unresolved route warning disappeared before the actual retry");
                FarmTravelConserved(world, village, farm, worker, leg);
            }
            Check(resumed && !worker.Position.Equals(target) && worker.BlockedReason == "", "Reopened farm travel did not physically resume before arrival");
            Check(farm.BlockedReason == "", "Resumed partial " + leg + " travel retained an obsolete farm route warning");
            FinishFarmTravel(world, village, farm, worker, leg);
        }

        static void TickFarmWithNoBudget(World world, Village village, Cat worker)
        {
            // White-box scheduling guard, not ordinary play: no actor time is
            // available and TickFarms must not infer a new path result.
            var flags = System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic;
            typeof(World).GetField("actorTime", flags).SetValue(world, new Dictionary<Cat, double> { { worker, 0 } });
            typeof(World).GetMethod("TickFarms", flags).Invoke(world, new object[] { village, World.SimulationStepSeconds });
        }

        static void FarmTravelExhaustedBudget()
        {
            foreach (string leg in new[] { "plot", "harvest", "handoff" })
            {
                var world = FarmTravelFixture(leg, out var village, out var farm, out var worker);
                double x = worker.X, z = worker.Z, growth = farm.Growth, harvest = farm.Harvest, cargo = World.Amount(worker.Cargo, "grain");
                // An old cat-level marker cannot establish that this leg was
                // attempted. Neither create a warning nor erase unresolved one.
                worker.BlockedReason = "blocked_route";
                farm.BlockedReason = "";
                TickFarmWithNoBudget(world, village, worker);
                Check(farm.BlockedReason == "", "An exhausted actor budget created a " + leg + " warning from a stale cat blocker");
                farm.BlockedReason = FarmTravelWarning(leg);
                worker.BlockedReason = "";
                TickFarmWithNoBudget(world, village, worker);
                Check(farm.BlockedReason == FarmTravelWarning(leg), "An exhausted actor budget cleared an unresolved " + leg + " warning without attempting movement");
                Check(!FarmTravelMoved(worker, x, z) && farm.Growth == growth && farm.Harvest == harvest && World.Amount(worker.Cargo, "grain") == cargo, "Zero actor time advanced travel, work or crop");
                FarmTravelConserved(world, village, farm, worker, leg);
            }
        }

        static void FarmTravelRetrace()
        {
            var world = FarmTravelFixture("plot", out var village, out var farm, out var worker);
            var anchor = worker.Position;
            for (int tick = 0; tick < 5; tick++) world.Step(World.SimulationStepSeconds);
            double before = Math.Abs(worker.X - anchor.X) + Math.Abs(worker.Z - anchor.Z);
            Check(worker.Position.Equals(anchor) && before > .2 && before < 1 && worker.Path.Count > 0, "Retrace guard needs a naturally traveled unfinished edge");
            // Interrupted-path snapshot credit: the real partial position is
            // retained, while its cached path and old route markers are stale.
            worker.Path.Clear();
            worker.BlockedReason = "blocked_route";
            farm.BlockedReason = "blocked_route";
            world.Step(World.SimulationStepSeconds);
            double after = Math.Abs(worker.X - anchor.X) + Math.Abs(worker.Z - anchor.Z);
            Check(worker.Position.Equals(anchor) && after > 0 && after < before && farm.Growth == 0 && farm.Harvest == 0, "Changed-edge retrace must physically return partway without working");
            Check(farm.BlockedReason == "", "Physical retracing treated the stored cat blocker as a newly tested route obstruction");
            FarmTravelConserved(world, village, farm, worker, "plot");
        }

        static void FarmTravelPickup()
        {
            var world = FarmTravelFixture("harvest", out var village, out var farm, out var worker);
            // Separate arrival guard: ignore intermediate route feedback here
            // so this case independently exposes stale feedback at real pickup.
            farm.BlockedReason = "blocked_harvest_route";
            for (int tick = 0; tick < 600 && worker.Cargo.Count == 0; tick++) world.Step(World.SimulationStepSeconds);
            Check(worker.Position.Equals(farm.Position) && farm.Harvest == 0 && Math.Abs(World.Amount(worker.Cargo, "grain") - 1.6) < 1e-8, "Pickup guard must physically collect the exact outstanding crop remainder");
            Check(farm.BlockedReason == "", "Physical harvest pickup retained the old return-to-plot route warning");
            FarmTravelConserved(world, village, farm, worker, "harvest");
        }

        static void FarmTravelOtherBlockers()
        {
            var world = FarmTravelFixture("harvest", out var village, out var farm, out var worker);
            var pile = village.Stockpiles.Single(p => p.ManagedBy == farm.Id);
            // Finite presentation-independent capacity fixture; the eight Grain
            // already delivered remain real and the 1.6 remainder must be held.
            pile.Capacity = 8;
            for (int tick = 0; tick < 1200 && farm.BlockedReason != "handoff_full"; tick++) world.Step(World.SimulationStepSeconds);
            Check(worker.Position.Equals(farm.Handoff) && farm.Harvest == 0 && farm.BlockedReason == "handoff_full" && Math.Abs(World.Amount(worker.Cargo, "grain") - 1.6) < 1e-8 && World.Amount(pile.Goods, "grain") == 8, "Full handoff must retain its real capacity warning and exact undelivered load");
            double x = worker.X, z = worker.Z;
            worker.Rest = 50;
            worker.Goal = "need_sleep";
            world.Step(World.SimulationStepSeconds);
            Check(worker.Goal == "need_sleep" && farm.BlockedReason == "handoff_full" && FarmTravelMoved(worker, x, z) && Math.Abs(World.Amount(worker.Cargo, "grain") - 1.6) < 1e-8, "A need interruption changed the full handoff feedback or lost its held crop");
            FarmTravelConserved(world, village, farm, worker, "handoff");
        }

        static void FarmPhysicalHandoff()
        {
            var world = FarmFixture(out var village, out var farm, out var blocked, out var reachable);
            // Setup credit bounds the scenario. The default 7,200-second cycle and
            // base eight-Grain yield remain unchanged. Required Irrigation raises
            // the real yield to 9.6, carried in ordinary eight- and 1.6-Grain loads.
            double credit = farm.Growth = farm.CycleSeconds - 8;
            double yield = farm.YieldPerTile * farm.Width * farm.Depth * Catalog.Effect(village, "farmYield", 1) * Catalog.Effect(village, "farmYieldMult", 1) * Catalog.BuildingEffect(village, "field", "output", 1);
            Check(world.TimeSeconds == 0 && farm.CycleSeconds == 7200 && farm.YieldPerTile == 8 && Math.Abs(yield - 9.6) < 1e-8, "Crop credit must precede the clock without changing the normal cycle, base yield or researched Irrigation effect");
            world.Step(10);
            ExpectFarmWorker(world, village, farm, reachable);
            Check(farm.Growth == credit && farm.Harvest == 0 && FarmGrain(village) == 0 && !reachable.Position.Equals(farm.Position), "Staffing skipped travel or created instant crop growth or delivery");
            bool worked = false, harvested = false, carried = false, carriedFirst = false, carriedRemainder = false, delivered = false;
            int deliveries = 0;
            for (int tick = 0; tick < 2400 && !delivered; tick++)
            {
                double beforeGrowth = farm.Growth;
                double beforeOutput = village.Stockpiles.Where(p => p.ManagedBy == farm.Id).Sum(p => World.Amount(p.Goods, "grain"));
                world.Step(World.SimulationStepSeconds);
                if (farm.Growth > beforeGrowth)
                {
                    Check(reachable.Position.Equals(farm.Position), "Farm growth advanced while its assigned cat was away from the plot");
                    worked = true;
                }
                if (!harvested && farm.Harvest > 0)
                {
                    Check(worked && reachable.Position.Equals(farm.Position) && Math.Abs(farm.Harvest - yield) < 1e-8, "Harvest appeared without measured work at the physical plot or changed its researched yield");
                    harvested = true;
                }
                if (World.Amount(reachable.Cargo, "grain") > 0)
                {
                    Check(harvested && farm.Growth == 0, "Farm cargo appeared before the single completed crop cycle");
                    carriedFirst |= Math.Abs(World.Amount(reachable.Cargo, "grain") - 8) < 1e-8;
                    carriedRemainder |= Math.Abs(World.Amount(reachable.Cargo, "grain") - 1.6) < 1e-8;
                    carried |= Math.Abs(reachable.X - farm.Position.X) + Math.Abs(reachable.Z - farm.Position.Z) > 1e-8 && Math.Abs(reachable.X - farm.Handoff.X) + Math.Abs(reachable.Z - farm.Handoff.Z) > 1e-8;
                }
                var pile = village.Stockpiles.SingleOrDefault(p => p.ManagedBy == farm.Id);
                if (pile != null && World.Amount(pile.Goods, "grain") > beforeOutput)
                {
                    Check(pile.Kind == "farm_output" && pile.Position.Equals(farm.Handoff) && reachable.Position.Equals(farm.Handoff) && World.Amount(pile.Goods, "grain") <= yield && reachable.Cargo.Count == 0, "Crop handoff skipped physical arrival, exceeded its finite yield or retained duplicate cargo");
                    deliveries++;
                    delivered = Math.Abs(World.Amount(pile.Goods, "grain") - yield) < 1e-8 && farm.Harvest == 0;
                }
                Check(Math.Abs(FarmGrain(village) - (harvested ? yield : 0)) < 1e-8, "Crop harvest, carried load and physical output did not conserve the single completed yield");
                ExpectFarmWorker(world, village, farm, reachable);
                Check(blocked.JobId == "" && blocked.BuildingId == "" && village.Jobs.Count == 0 && world.Total(village, "grain") == 0, "Crop handoff stole blocked labor or teleported output into general storage");
            }
            Check(worked && harvested && carried && carriedFirst && carriedRemainder && deliveries == 2 && delivered && world.TimeSeconds <= 130, "Reachable farmer did not physically work, harvest, carry and hand off both finite crop loads within the bounded scenario");
            Valid(world);
        }
    }
}
