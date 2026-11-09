using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class WorkshopSupplyScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.workshop_supply_uses_counted_storage_history", CountedStorageHistory);
            yield return new Scenario("regression.workshop_supply_retains_uncounted_and_mixed_reports", UncountedAndMixed);
            yield return new Scenario("regression.workshop_supply_hides_live_stock_claims_and_routes", HiddenLiveState);
            yield return new Scenario("regression.workshop_supply_tracks_report_updates_and_count_times", ReportUpdates);
            yield return new Scenario("regression.workshop_supply_uses_research_scaled_recipe_bill", ScaledRecipeBill);
            yield return new Scenario("regression.workshop_supply_is_detached_and_does_not_mutate_world", DetachedQuery);
            yield return new Scenario("regression.workshop_supply_explains_locked_matching_recipe_only", MatchingRecipe);
        }

        private static World Fixture(out Village village, out Building workshop)
        {
            var world = new World { TimeSeconds = 3600, RandomState = 1234, NextId = 77 };
            village = new Village { Id = "supply-village" };
            workshop = new Building { Id = "supply-workshop", Kind = "woodworking", Completed = true };
            village.Buildings.Add(workshop);
            world.Villages.Add(village);
            return world;
        }

        private static WorkshopSupplyReport Query(World world, Village village, Building workshop) => world.WorkshopSupply(village, workshop, "planks_and_blocks_to_tools");
        private static WorkshopSupplyInput Input(WorkshopSupplyReport report, string resource) => report.Inputs.Single(row => row.Resource == resource);
        private static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        private static void Near(double actual, double expected, string message) => Check(Math.Abs(actual - expected) < 1e-9, message + " expected=" + expected + " actual=" + actual);

        private static void CountedStorageHistory()
        {
            var world = Fixture(out var village, out var workshop);
            village.Stockpiles.Add(new Stockpile { Id = "older", CountedAt = 600, Report = new List<Stack> { new Stack("planks", 1), new Stack("blocks", 2) } });
            village.Stockpiles.Add(new Stockpile { Id = "newer", CountedAt = 1800, Report = new List<Stack> { new Stack("planks", 2), new Stack("blocks", 1) } });
            village.Stockpiles.Add(new Stockpile { Id = "gather", Kind = "gather", CountedAt = 0, Report = new List<Stack> { new Stack("planks", 999), new Stack("blocks", 999) } });
            var report = Query(world, village, workshop);
            Check(report.RecipeId == "planks_and_blocks_to_tools" && report.Inputs.Count == 2, "Recipe bill lost one of its named ingredients");
            Near(Input(report, "planks").Required, 2, "Plank bill changed");
            Near(Input(report, "blocks").Required, 2, "Block bill changed");
            Near(Input(report, "planks").Reported, 3, "Storage plank reports were not summed independently of gather reports");
            Near(Input(report, "blocks").Reported, 3, "Storage block reports were not summed independently of gather reports");
            Check(report.CountedPiles == 2 && report.UncountedPiles == 0 && report.OldestCountedAt == 600 && report.NewestCountedAt == 1800, "Storage history lost its mixed count times");
        }

        private static void UncountedAndMixed()
        {
            var world = Fixture(out var village, out var workshop);
            var unknown = new Stockpile { Id = "unknown", Report = new List<Stack> { new Stack("planks", 999) }, Goods = new List<Stack> { new Stack("blocks", 99) } };
            village.Stockpiles.Add(unknown);
            var report = Query(world, village, workshop);
            Check(report.CountedPiles == 0 && report.UncountedPiles == 1 && report.OldestCountedAt == -1 && report.NewestCountedAt == -1, "Uncounted storage was presented as a known zero report");
            Check(report.Inputs.All(row => row.Reported == 0), "An uncounted pile exposed quantities");

            village.Stockpiles.Add(new Stockpile { Id = "known-empty", CountedAt = 0 });
            report = Query(world, village, workshop);
            Check(report.CountedPiles == 1 && report.UncountedPiles == 1 && report.OldestCountedAt == 0 && report.NewestCountedAt == 0, "Mixed report failed to distinguish a count at zero from uncounted storage");
            Check(report.Inputs.All(row => row.Reported == 0), "Mixed report included unrecorded quantities");
        }

        private static void HiddenLiveState()
        {
            var world = Fixture(out var village, out var workshop);
            var pile = new Stockpile { Id = "counted", CountedAt = 600, Report = new List<Stack> { new Stack("planks", 1), new Stack("blocks", 4) } };
            village.Stockpiles.Add(pile);
            var before = Query(world, village, workshop);
            pile.Goods.Add(new Stack("planks", 1000));
            pile.Goods.Add(new Stack("blocks", 1000));
            world.Reservations.Add(new Reservation { OwnerId = "hidden-claim", VillageId = village.Id, PileId = pile.Id, Resource = "blocks", Amount = 1000 });
            workshop.Inputs.Add(new Stack("planks", 2));
            workshop.Outputs.Add(new Stack("blocks", 8));
            world.Tiles.Add(new Tile { Position = pile.Position, Wall = true });
            var after = Query(world, village, workshop);
            Check(before.CountedPiles == after.CountedPiles && before.UncountedPiles == after.UncountedPiles && before.OldestCountedAt == after.OldestCountedAt && before.NewestCountedAt == after.NewestCountedAt, "Hidden live state changed report metadata");
            foreach (var input in before.Inputs)
            {
                Near(Input(after, input.Resource).Required, input.Required, "Hidden live state changed the recipe bill");
                Near(Input(after, input.Resource).Reported, input.Reported, "Hidden live stock, claims, staged goods or blocked routes changed a historical report");
            }
        }

        private static void ReportUpdates()
        {
            var world = Fixture(out var village, out var workshop);
            var older = new Stockpile { Id = "older", CountedAt = 600, Report = new List<Stack> { new Stack("planks", 1) } };
            var newer = new Stockpile { Id = "newer", CountedAt = 1800, Report = new List<Stack> { new Stack("blocks", 4) } };
            village.Stockpiles.Add(older);
            village.Stockpiles.Add(newer);
            var before = Query(world, village, workshop);
            older.Report = new List<Stack> { new Stack("planks", 5) };
            older.CountedAt = 3000;
            var after = Query(world, village, workshop);
            Near(Input(after, "planks").Reported, 5, "New count did not update the reported amount");
            Near(Input(after, "blocks").Reported, 4, "New count changed another pile's report");
            Check(after.OldestCountedAt == 1800 && after.NewestCountedAt == 3000 && after.CountedPiles == 2, "New count did not update the retained report time range");
            Near(Input(before, "planks").Reported, 1, "New count mutated a previously returned report");
            Check(before.OldestCountedAt == 600 && before.NewestCountedAt == 1800, "New count rewrote previously returned history");
        }

        private static void ScaledRecipeBill()
        {
            var world = Fixture(out var village, out var workshop);
            village.Research.Add("toolmaking_sources");
            village.Research.Add("woodworking_workflow");
            var report = Query(world, village, workshop);
            double expected = 2 * 0.9 / 1.12;
            Near(Input(report, "planks").Required, expected, "Plank bill ignored item material efficiency studies");
            Near(Input(report, "blocks").Required, expected, "Block bill ignored item material efficiency studies");
            Near(Catalog.Recipe(report.RecipeId).Inputs[0].Amount, 2, "Reporting scaled the catalog input in place");
        }

        private static void DetachedQuery()
        {
            var world = Fixture(out var village, out var workshop);
            var pile = new Stockpile { Id = "counted", CountedAt = 600, Report = new List<Stack> { new Stack("planks", 3) }, Goods = new List<Stack> { new Stack("planks", 1) } };
            village.Stockpiles.Add(pile);
            var reports = pile.Report;
            var goods = pile.Goods;
            var reservations = world.Reservations;
            var recipe = Catalog.Recipe("planks_and_blocks_to_tools");
            var recipeInputs = recipe.Inputs;
            var first = Query(world, village, workshop);
            var second = Query(world, village, workshop);
            Check(!ReferenceEquals(first, second) && !ReferenceEquals(first.Inputs, second.Inputs) && !ReferenceEquals(first.Inputs[0], second.Inputs[0]), "Two queries shared mutable report objects");
            first.Inputs[0].Resource = "food";
            first.Inputs[0].Required = 500;
            first.Inputs[0].Reported = 500;
            first.Inputs.Clear();
            Check(world.TimeSeconds == 3600 && world.RandomState == 1234 && world.NextId == 77 && world.Tiles.Count == 0 && world.Reservations.Count == 0, "Report query advanced the world, consumed entropy, planned routes or created claims");
            Check(village.Jobs.Count == 0 && village.Cats.Count == 0 && village.Buildings.Count == 1 && village.Stockpiles.Count == 1 && workshop.Inputs.Count == 0 && workshop.Outputs.Count == 0 && workshop.Queue.Count == 0 && workshop.Slots.Count == 0, "Report query created work or changed the workshop");
            Check(ReferenceEquals(reports, pile.Report) && ReferenceEquals(goods, pile.Goods) && ReferenceEquals(reservations, world.Reservations) && ReferenceEquals(recipeInputs, recipe.Inputs), "Report query replaced authoritative collections");
            Near(World.Amount(pile.Report, "planks"), 3, "Returned report mutation changed Accountant history");
            Near(World.Amount(pile.Goods, "planks"), 1, "Report query changed live goods");
            Near(Input(second, "planks").Reported, 3, "Returned report mutation changed another returned report");
            Near(recipe.Inputs[0].Amount, 2, "Returned report mutation changed the catalog bill");
            Check(recipe.Inputs[0].Resource == "planks" && pile.CountedAt == 600, "Returned report mutation changed catalog identity or count time");
        }

        private static void MatchingRecipe()
        {
            var world = Fixture(out var village, out var workshop);
            workshop.Kind = "clothier";
            Check(!Catalog.RecipeAvailable(village, Catalog.Recipe("fibre_to_cloth")), "Locked recipe fixture unexpectedly owns Textiles");
            var locked = world.WorkshopSupply(village, workshop, "fibre_to_cloth");
            Check(locked.RecipeId == "fibre_to_cloth" && locked.Inputs.Count == 1 && locked.Inputs[0].Resource == "thread" && locked.Inputs[0].Required == 5, "A historical locked recipe lost its explainable bill");
            Check(world.WorkshopSupply(village, workshop, "planks_and_blocks_to_tools").Inputs.Count == 0, "A different workshop recipe supplied a misleading bill");
            Check(world.WorkshopSupply(village, workshop, "missing-recipe").Inputs.Count == 0, "An unknown recipe supplied a misleading bill");
        }
    }
}
