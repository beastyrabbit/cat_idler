using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class ReachableMaterialScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.reachable_material_skips_blocked_early_pile", SkipBlockedPile);
            yield return new Scenario("regression.reachable_material_uses_accessible_pile_tile", UseAccessiblePileTile);
            yield return new Scenario("regression.reachable_material_reselects_closed_claim", ReselectClosedClaim);
            yield return new Scenario("regression.reachable_material_preserves_delivered_input", PreserveDeliveredInput);
            yield return new Scenario("regression.reachable_material_unfunded_route_is_atomic", UnfundedRouteIsAtomic);
            yield return new Scenario("regression.reachable_material_requires_station_delivery", RequireStationDelivery);
            yield return new Scenario("regression.reachable_material_cancel_releases_reselected_claim", CancelReselectedClaim);
            yield return new Scenario("regression.reachable_material_needs_reclaims_exact_spill", ReclaimExactNeedsSpill);
            yield return new Scenario("acceptance.reachable_material_finite_wooden_tool_chain", FiniteWoodenToolChain);
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static ActionResult Act(World world, Village village, GameAction action)
        {
            var result = world.Apply(new PlayerContext { PlayerId = "reachable-material-acceptance", VillageId = village.Id }, action);
            Check(result.Success, action.Kind + ": " + result.Error);
            return result;
        }

        static World Fixture(string kind, out Village village, out Building station, out Cat worker)
        {
            var world = World.Create(41);
            village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            village.Stockpiles[0].Goods = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000) };
            village.Stockpiles[0].Capacity = 100000;
            village.Stockpiles[0].Position = new Int2(3, 2);
            village.LastLeaderResearch = 0;
            foreach (var cat in village.Cats)
            {
                cat.ControlledBy = "fixture-held";
                cat.ControlLeaseUntil = 10000000;
                cat.Position = new Int2(1, 3);
                cat.X = 1;
                cat.Z = 3;
            }
            worker = village.Cats[0];
            worker.ControlledBy = "";
            station = new Building { Id = world.Id("reachable-material-station"), Kind = kind, Position = worker.Position, Completed = true };
            village.Buildings.Add(station);
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = worker.Id });
            return world;
        }

        static Stockpile Supply(Village village, string id, string resource, double amount, Int2 at, int width = 1)
        {
            var pile = new Stockpile { Id = id, Position = at, Width = width, Depth = 1, Capacity = 100000, Goods = new List<Stack> { new Stack(resource, amount) } };
            village.Stockpiles.Add(pile);
            return pile;
        }

        static Job Start(World world, Village village, Building station, Cat worker, string recipe = "logs_to_planks")
        {
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "add", RecipeId = recipe });
            world.Step(1.05);
            var job = village.Jobs.Find(j => !j.Completed && j.Kind == "production" && j.CatId == worker.Id);
            Check(job != null, "The finite manual recipe did not create physical work");
            return job;
        }

        static void SkipBlockedPile()
        {
            var world = Fixture("wood_cutter", out var village, out var station, out var worker);
            var blocked = Supply(village, "aaa-blocked-logs", "logs", 5, new Int2(-3, 2));
            var reachable = Supply(village, "zzz-reachable-logs", "logs", 5, new Int2(3, 3));
            world.TileAt(blocked.Position).Wall = true;
            var job = Start(world, village, station, worker);
            Check(world.Reservations.Where(r => r.OwnerId == job.Id).Sum(r => r.Amount) == 5 && world.Reservations.All(r => r.OwnerId != job.Id || r.PileId == reachable.Id), "Recipe reserved the unreachable early-ID pile instead of five reachable logs");
            Check(World.Amount(blocked.Goods, "logs") == 5 && World.Amount(reachable.Goods, "logs") == 5 && worker.Cargo.Count == 0, "Choosing a source moved finite goods before physical pickup");
        }

        static void UseAccessiblePileTile()
        {
            var world = Fixture("wood_cutter", out var village, out var station, out var worker);
            var pile = Supply(village, "aaa-wide-logs", "logs", 5, new Int2(3, 3), 2);
            world.TileAt(pile.Position).Wall = true;
            Start(world, village, station, worker);
            world.Step(5);
            Check(World.Amount(worker.Cargo, "logs") + village.Jobs.Where(j => !j.Completed).Sum(j => World.Amount(j.Local, "logs")) == 5, "A reachable tile in the stockpile footprint did not permit physical ingredient pickup");
            Check(World.Amount(pile.Goods, "logs") == 0 && world.TileAt(pile.Position).Wall, "Pickup altered the blocked origin or duplicated the wide pile's goods");
        }

        static void ReselectClosedClaim()
        {
            var world = Fixture("wood_cutter", out var village, out var station, out var worker);
            var first = Supply(village, "aaa-first-logs", "logs", 5, new Int2(3, 3));
            var alternate = Supply(village, "zzz-alternate-logs", "logs", 5, new Int2(-3, 2));
            var job = Start(world, village, station, worker);
            Check(world.Reservations.Any(r => r.OwnerId == job.Id && r.PileId == first.Id), "Closure fixture did not start with a real finite source claim");
            var order = station.Slots.Single().Queue.Single();
            world.TileAt(first.Position).Wall = true;
            world.Step(1);
            Check(!world.Reservations.Any(r => r.OwnerId == job.Id && r.PileId == first.Id), "A closed ingredient route retained its unpicked claim");
            Check(world.Reservations.Any(r => r.OwnerId == job.Id && r.PileId == alternate.Id), "A closed ingredient route did not reselect reachable finite stock");
            Check(worker.JobId == job.Id && ReferenceEquals(station.Slots.Single().Queue.Single(), order), "Route recovery replaced the player's job or finite order");
            Check(World.Amount(first.Goods, "logs") == 5, "Route recovery lost the inaccessible unpicked logs");
        }

        static void PreserveDeliveredInput()
        {
            var world = Fixture("woodworking", out var village, out var station, out var worker);
            Supply(village, "aaa-planks", "planks", 2, new Int2(2, 3));
            var first = Supply(village, "bbb-first-blocks", "blocks", 2, new Int2(3, 3));
            var alternate = Supply(village, "zzz-alternate-blocks", "blocks", 2, new Int2(-3, 2));
            var job = Start(world, village, station, worker, "planks_and_blocks_to_tools");
            for (int tick = 0; tick < 160 && (World.Amount(job.Local, "planks") != 2 || !world.Reservations.Any(r => r.OwnerId == job.Id && r.PileId == first.Id)); tick++) world.Step(.05);
            Check(World.Amount(job.Local, "planks") == 2 && world.Reservations.Any(r => r.OwnerId == job.Id && r.PileId == first.Id), "Composed fixture did not physically deliver its first ingredient before claiming the second");
            world.TileAt(first.Position).Wall = true;
            world.Step(1);
            Check(World.Amount(job.Local, "planks") == 2 && !world.Reservations.Any(r => r.OwnerId == job.Id && r.Resource == "planks"), "Reselecting blocks duplicated or discarded already delivered planks");
            Check(world.Reservations.Any(r => r.OwnerId == job.Id && r.PileId == alternate.Id) && World.Amount(first.Goods, "blocks") == 2, "Partial-input recovery failed to replace only its inaccessible unpicked blocks");
        }

        static void UnfundedRouteIsAtomic()
        {
            var world = Fixture("wood_cutter", out var village, out var station, out var worker);
            var reachable = Supply(village, "aaa-four-reachable-logs", "logs", 4, new Int2(3, 3));
            var blocked = Supply(village, "zzz-five-blocked-logs", "logs", 5, new Int2(-3, 2));
            world.TileAt(blocked.Position).Wall = true;
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "add", RecipeId = "logs_to_planks" });
            var order = station.Slots.Single().Queue.Single();
            world.Step(1.05);
            Check(worker.JobId == "" && world.Reservations.Count == 0 && station.BlockedReason == "input_unreachable", "An incomplete reachable bill stranded a partial source claim or concealed its route blocker");
            Check(World.Amount(reachable.Goods, "logs") == 4 && World.Amount(blocked.Goods, "logs") == 5 && ReferenceEquals(station.Slots.Single().Queue.Single(), order), "An unreachable recipe altered finite supplies or replaced the player's order");
            world.TileAt(blocked.Position).Wall = false;
            world.Step(1);
            Check(worker.JobId != "" && world.Reservations.Where(r => r.OwnerId == worker.JobId).Sum(r => r.Amount) == 5, "Reopening the route did not fund the preserved five-log order");
        }

        static void RequireStationDelivery()
        {
            var world = Fixture("wood_cutter", out var village, out var station, out var worker);
            var pile = Supply(village, "aaa-accessible-logs", "logs", 5, new Int2(3, 3));
            worker.Position = pile.Position;
            worker.X = pile.Position.X;
            worker.Z = pile.Position.Z;
            world.TileAt(station.Position).Wall = true;
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "add", RecipeId = "logs_to_planks" });
            world.Step(1.05);
            Check(worker.JobId == "" && world.Reservations.Count == 0 && World.Amount(pile.Goods, "logs") == 5, "A cat able to reach stock claimed goods it could not deliver to its workshop");
            Check(station.BlockedReason == "input_unreachable", "An inaccessible workshop did not report the physical ingredient route blocker");
        }

        static void CancelReselectedClaim()
        {
            var world = Fixture("wood_cutter", out var village, out var station, out var worker);
            var first = Supply(village, "aaa-cancel-first", "logs", 5, new Int2(3, 3));
            Supply(village, "zzz-cancel-alternate", "logs", 5, new Int2(-3, 2));
            var job = Start(world, village, station, worker);
            var order = station.Slots.Single().Queue.Single();
            world.TileAt(first.Position).Wall = true;
            world.Step(1);
            Act(world, village, new GameAction { Kind = "AssignWorker", CatId = worker.Id });
            Check(job.Completed && worker.JobId == "" && worker.BuildingId == "" && !world.Reservations.Any(r => r.OwnerId == job.Id), "Cancelling recovered production retained a worker or finite ingredient claim");
            Check(ReferenceEquals(station.Slots.Single().Queue.Single(), order) && !order.Repeat, "Cancelling recovered work removed or rewrote its manual one-shot queue");
            Check(village.Stockpiles.Sum(p => World.Amount(p.Goods, "logs")) + World.Amount(worker.Cargo, "logs") == 10, "Cancelling a recovered input route duplicated or discarded finite logs");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void ReclaimExactNeedsSpill()
        {
            var world = Fixture("wood_cutter", out var village, out var station, out var worker);
            Supply(village, "aaa-needs-source", "logs", 5, new Int2(2, 3));
            var spare = Supply(village, "zzz-needs-spare", "logs", 5, new Int2(-3, 2));
            var job = Start(world, village, station, worker);
            for (int tick = 0; tick < 100 && World.Amount(worker.Cargo, "logs") != 5; tick++) world.Step(.05);
            Check(World.Amount(worker.Cargo, "logs") == 5, "Needs fixture did not physically pick up its exact five-log load");
            worker.Thirst = 1;
            world.Step(.05);
            var spill = village.Stockpiles.Find(p => p.Id == job.SuspendedCargoPileId);
            Check(spill != null && World.Amount(spill.Goods, "logs") == 5 && world.Reservations.Any(r => r.OwnerId == job.Id + ":resume" && r.PileId == spill.Id && r.Amount == 5), "Critical thirst did not preserve the carried load as exact physical suspended cargo");
            world.TileAt(spill.Position).Wall = true;
            world.Step(10);
            Check(!worker.Goal.StartsWith("need_", StringComparison.Ordinal) && worker.Thirst > 35, "The worker did not physically meet its critical need");
            Check(world.Reservations.Any(r => r.OwnerId == job.Id + ":resume" && r.PileId == spill.Id && r.Amount == 5) && World.Amount(spill.Goods, "logs") == 5 && World.Amount(spare.Goods, "logs") == 5 && World.Amount(job.Local, "logs") == 0, "An inaccessible needs spill was released or replaced by unrelated reachable stock");
            world.TileAt(spill.Position).Wall = false;
            for (int tick = 0; tick < 200 && World.Amount(job.Local, "logs") != 5; tick++) world.Step(.05);
            Check(World.Amount(job.Local, "logs") == 5 && World.Amount(spare.Goods, "logs") == 5 && !world.Reservations.Any(r => r.OwnerId == job.Id + ":resume"), "Reopened needs cargo did not resume the same recipe with its exact physical load");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void CompleteRecipe(World world, Village village, Building station, Cat worker, string recipe)
        {
            var job = Start(world, village, station, worker, recipe);
            for (int second = 0; second < 1200 && !job.Completed; second++) world.Step(1);
            Check(job.Completed && worker.Cargo.Count == 0 && station.Slots.Single().Queue.Count == 0, "The physical one-shot material chain did not complete and deliver " + recipe);
        }

        static void FiniteWoodenToolChain()
        {
            var world = Fixture("wood_cutter", out var village, out var cutter, out var worker);
            // Keep the composed fixture small; the full founding population runs in campaigns.
            village.Cats.RemoveRange(1, village.Cats.Count - 1);
            var blockedLogs = Supply(village, "aaa-chain-blocked-logs", "logs", 10, new Int2(-3, 2));
            var blockedStone = Supply(village, "aaa-chain-blocked-stone", "stone", 10, new Int2(-3, 3));
            Supply(village, "zzz-chain-reachable-logs", "logs", 10, new Int2(3, 3));
            Supply(village, "zzz-chain-reachable-stone", "stone", 10, new Int2(3, 4));
            world.TileAt(blockedLogs.Position).Wall = true;
            world.TileAt(blockedStone.Position).Wall = true;
            var prep = new Building { Id = world.Id("reachable-stone-prep"), Kind = "stone_prep", Position = new Int2(1, 5), Completed = true };
            var bench = new Building { Id = world.Id("reachable-woodworking"), Kind = "woodworking", Position = new Int2(-2, 5), Completed = true };
            village.Buildings.Add(prep);
            village.Buildings.Add(bench);
            int tools = village.Items.Count(i => i.Kind == "tool");
            CompleteRecipe(world, village, cutter, worker, "logs_to_planks");
            CompleteRecipe(world, village, cutter, worker, "logs_to_planks");
            Check(world.Total(village, "planks") == 2, "Two finite log batches did not physically supply two planks");
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = prep.Id, CatId = worker.Id });
            CompleteRecipe(world, village, prep, worker, "stone_to_blocks");
            CompleteRecipe(world, village, prep, worker, "stone_to_blocks");
            Check(world.Total(village, "blocks") == 2, "Two finite stone batches did not physically supply two blocks");
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = bench.Id, CatId = worker.Id });
            CompleteRecipe(world, village, bench, worker, "planks_and_blocks_to_tools");
            Check(village.Items.Count(i => i.Kind == "tool") == tools + 1 && village.Items.Any(i => i.Kind == "tool" && i.Material == "wood" && village.Stockpiles.Any(p => p.Kind == "storage" && p.Id == i.LocationId)), "The connected finite chain did not deliver exactly one real wooden tool to storage");
            Check(world.Total(village, "planks") == 0 && world.Total(village, "blocks") == 0 && World.Amount(blockedLogs.Goods, "logs") == 10 && World.Amount(blockedStone.Goods, "stone") == 10, "Crafting consumed inaccessible stock or retained duplicated intermediate goods");
            Check(village.Stockpiles.Where(p => p.Id != blockedLogs.Id).Sum(p => World.Amount(p.Goods, "logs")) == 0 && village.Stockpiles.Where(p => p.Id != blockedStone.Id).Sum(p => World.Amount(p.Goods, "stone")) == 0 && world.Reservations.Count == 0 && !village.Jobs.Any(j => !j.Completed), "The finite material chain left raw inputs, claims or unfinished work behind");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }
    }
}
