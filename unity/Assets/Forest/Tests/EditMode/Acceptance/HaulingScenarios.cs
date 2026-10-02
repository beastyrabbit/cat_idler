using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>Jobs, hauling and storage: scalar and exact-item hauls, claims, spills and storage acceptance.</summary>
    public static class HaulingScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.continuous_storage_input_retry", () => ContinuousInputStorage(false));
            yield return new Scenario("regression.continuous_storage_input_immediate_alternate", () => ContinuousInputStorage(true));
            yield return new Scenario("regression.continuous_storage_removed_scalar_destination", () => ContinuousRemovedStorage(false));
            yield return new Scenario("regression.continuous_storage_removed_exact_destination", () => ContinuousRemovedStorage(true));
            yield return new Scenario("regression.continuous_storage_removed_need_source", ContinuousRemovedNeedSource);
            yield return new Scenario("regression.continuous_storage_missing_scaffold_retry", ContinuousMissingScaffoldRetry);
            yield return new Scenario("regression.busy_cat_job_ownership", BusyCat);
            yield return new Scenario("regression.item_only_stockpile_removal", ItemPile);
            yield return new Scenario("regression.review_armor_production_accepting_storage", ArmorProductionStorage);
            yield return new Scenario("regression.review_armor_unequip_accepting_storage", ArmorUnequipStorage);
            yield return new Scenario("regression.exact_haul_produced_mug_remove_recover_sell", () => ProducedItemRecovery(false));
            yield return new Scenario("regression.exact_haul_cancelled_production_recover_sell", () => ProducedItemRecovery(true));
            yield return new Scenario("regression.exact_haul_cancel_before_pickup", () => ExactHaulInterruption(false, false));
            yield return new Scenario("regression.exact_haul_cancel_after_pickup", () => ExactHaulInterruption(true, false));
            yield return new Scenario("regression.exact_haul_death_before_pickup", () => ExactHaulInterruption(false, true));
            yield return new Scenario("regression.exact_haul_death_after_pickup", () => ExactHaulInterruption(true, true));
            yield return new Scenario("regression.exact_haul_claims_and_full_storage", ExactHaulClaims);
            yield return new Scenario("regression.exact_haul_steward_recovers_item_only_spill", ExactHaulSteward);
            yield return new Scenario("regression.exact_haul_source_capacity_until_pickup", ExactHaulSourceCapacity);
            yield return new Scenario("regression.exact_haul_between_existing_stores", ExactHaulStoredTransfer);
            yield return new Scenario("regression.review_scalar_haul_between_existing_stores", ScalarHaulStoredTransfer);
            yield return new Scenario("regression.review_scalar_haul_partial_reservations_and_destination_change", ScalarHaulPartialReservations);
            yield return new Scenario("regression.review_scalar_haul_full_blocked_destination_preserves_source", ScalarHaulFullDestination);
            yield return new Scenario("regression.review_scalar_haul_avoid_reroute", () => ScalarHaulAvoidReroute(false));
            yield return new Scenario("regression.review_scalar_haul_avoid_reroute_partial_claim", () => ScalarHaulAvoidReroute(true));
        }
        static void ContinuousInputStorage(bool alternative)
        {
            var w = FailedPathFixture(out var v, out var c);
            var job = v.Jobs.Single();
            job.Kind = "haul";
            job.Phase = "input_delivery";
            var destination = v.Stockpiles[0];
            destination.Position = job.Position;
            job.TargetId = destination.Id;
            c.Cargo.Add(new Stack("logs", 8));
            if (alternative)
            {
                // One eligible alternate makes the exact lookup count independent of LINQ's predicate evaluation strategy.
                destination.Accepts.Add("water");
                destination = new Stockpile { Id = w.Id("alternate-store"), Position = new Int2(2, 2), Width = 1, Depth = 1 };
                v.Stockpiles.Add(destination);
            }
            long before = PathSearches(w);
            w.Step(0.05);
            Check(PathSearches(w) - before == 2 && c.X == 1, "Blocked haul must try its route and an initial alternate lookup immediately; searches=" + (PathSearches(w) - before) + "; x=" + c.X);
            if (alternative)
            {
                Check(job.TargetId == destination.Id && job.Position.Equals(destination.Position), "Reachable alternate storage was not selected immediately");
                w.Step(0.05);
                Near(c.X, 1.08, "Immediate alternate lookup did not resume physical carrying");
            }
            else
            {
                before = PathSearches(w);
                w.Step(0.85);
                Check(PathSearches(w) == before, "Blocked scalar haul repeated alternate-storage path searches within the planning interval");
                Near(World.Amount(c.Cargo, "logs"), 8, "Blocked alternate lookup changed finite carried goods");
                v.BoundaryEdges.Clear();
                w.Step(0.05);
                Near(c.X, 1, "Unchanged blocked destination retried before planning");
                w.Step(0.05);
                Near(c.X, 1.08, "Reopened storage route did not resume at the next planning boundary");
            }
            w.Step(4);
            Check(job.Completed && c.JobId == "" && c.Cargo.Count == 0, "Scalar haul did not finish and release its original worker");
            Near(World.Amount(destination.Goods, "logs"), 8, "Scalar haul did not deliver exactly once to its physical destination");
            Near(Goods(w, v, "logs"), 8, "Retargeted scalar haul lost or duplicated goods");
            Valid(w);
        }
        static void ContinuousRemovedStorage(bool exact)
        {
            var w = ContinuousFixture(out var v, out var c, true);
            var job = v.Jobs.Single();
            job.Kind = "haul";
            job.Phase = "output_delivery";
            var removed = v.Stockpiles[0];
            var destination = new Stockpile { Id = w.Id("remaining-store"), Position = new Int2(5, 2), Width = 1, Depth = 1 };
            v.Stockpiles.Add(destination);
            Item item = null;
            if (exact)
            {
                item = new Item { Id = w.Id("carried-mug"), Kind = "mug", Material = "clay", Quality = 2, Condition = 54.25, MaxCondition = 120, VillageId = v.Id, LocationId = job.Id };
                v.Items.Add(item);
                job.ItemIds.Add(item.Id);
            }
            else
                c.Cargo.Add(new Stack("logs", 8));
            w.Step(0.1);
            Check(job.DeliveryPileId == removed.Id && c.X > 1 && c.X < removed.Position.X, "Delivery did not cache a store while physically approaching");
            string original = string.Join("|", removed.Goods.Select(s => s.Resource + ":" + s.Amount));
            Act(w, v, new GameAction { Kind = "RemoveStockpile", TargetId = removed.Id });
            Check(removed.Kind == "spill" && v.Stockpiles.Contains(removed), "Public nonempty removal did not preserve the physical pile identity");
            w.Step(5);
            Check(job.Completed && c.JobId == "" && job.ItemIds.Count == 0 && c.Cargo.Count == 0, "Removed-store delivery did not finish its original job");
            Check(string.Join("|", removed.Goods.Select(s => s.Resource + ":" + s.Amount)) == original, "Cached delivery deposited cargo into a removed stockpile");
            if (exact)
                Check(item.LocationId == destination.Id && item.Condition == 54.25 && item.Quality == 2 && v.Items.Count(i => i.Id == item.Id) == 1, "Removed-store retargeting lost exact cargo identity or condition");
            else
            {
                Near(World.Amount(destination.Goods, "logs"), 8, "Removed-store scalar cargo did not reach valid storage");
                Near(Goods(w, v, "logs"), 8, "Removed-store delivery lost or duplicated scalar cargo");
            }
            Valid(w);
        }
        static void ContinuousRemovedNeedSource()
        {
            var w = ContinuousFixture(out var v, out var c, true);
            c.Thirst = 34;
            var removed = v.Stockpiles[0];
            var destination = new Stockpile { Id = w.Id("remaining-water"), Position = new Int2(5, 2), Width = 1, Depth = 1, Goods = new List<Stack> { new Stack("water", 10) } };
            v.Stockpiles.Add(destination);
            w.Step(0.1);
            Check(c.NeedSourceId == removed.Id && c.X > 1 && c.X < removed.Position.X, "Thirst did not cache its source while approaching");
            double oldWater = World.Amount(removed.Goods, "water");
            string jobId = c.JobId;
            Act(w, v, new GameAction { Kind = "RemoveStockpile", TargetId = removed.Id });
            for (int tick = 0; tick < 300 && c.Thirst < 80; tick++)
                w.Step(0.05);
            Check(c.Thirst >= 80 && c.JobId == jobId && removed.Kind == "spill", "Need source invalidation lost the suspended job or failed to drink");
            Near(World.Amount(removed.Goods, "water"), oldWater, "Cached need consumed a serving from a removed stockpile");
            Near(World.Amount(destination.Goods, "water"), 9, "Need retargeting did not consume one finite serving from valid storage");
            Check(c.Cargo.Count == 0, "Completed need retained a duplicate serving");
            Valid(w);
        }
        static void ContinuousMissingScaffoldRetry()
        {
            var w = ContinuousFixture(out var v, out var c, true);
            var job = v.Jobs.Single();
            job.Kind = "build";
            job.PendingBuildingKind = "wood_cutter";
            job.TargetId = "";
            job.RequiredWork = 1;
            var helper = v.Cats[1];
            var held = new Job { Id = w.Id("held-construction-input"), Kind = "hunt", Phase = "working", CatId = helper.Id, Position = helper.Position, RequiredWork = 1000 };
            v.Jobs.Add(held);
            helper.JobId = held.Id;
            foreach (var resource in new[] { new Stack("planks", 4), new Stack("blocks", 2) })
            {
                World.Add(v.Stockpiles[0].Goods, resource.Resource, resource.Amount);
                w.Reservations.Add(new Reservation { OwnerId = held.Id, VillageId = v.Id, PileId = v.Stockpiles[0].Id, Resource = resource.Resource, Amount = resource.Amount });
            }
            long before = PathSearches(w);
            w.Step(0.1);
            Check(job.TargetId == "" && job.BlockedReason == "missing_scaffold_input", "Missing scaffold did not retain its imported work while inputs were claimed");
            Check(PathSearches(w) - before == 1, "Missing scaffold repeated its site/path resolution within the planning interval");
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = helper.Id });
            Check(held.Completed && w.Reservations.Count == 0, "Public interruption did not release the competing input claims");
            w.Step(0.85);
            Check(job.TargetId == "", "Missing scaffold resolved before its next planning boundary");
            w.Step(0.05);
            Check(job.TargetId != "" && job.CatId == c.Id, "Released finite inputs did not resume the same imported building job");
            for (int tick = 0; tick < 400 && !job.Completed; tick++)
                w.Step(0.05);
            Check(job.Completed && v.Buildings.Single(b => b.Id == job.TargetId).Completed, "Retried scaffold did not complete through its original worker");
            Near(Goods(w, v, "planks"), 0, "Scaffold retry changed its exact timber bill");
            Near(Goods(w, v, "blocks"), 0, "Scaffold retry changed its exact block bill");
            Check(w.Reservations.Count == 0, "Completed scaffold retained claims");
            Valid(w);
        }
        static void BusyCat()
        {
            var w = Fixture(out var v, out var c);
            c.BuildingId = "";
            World.Add(v.Stockpiles[0].Goods, "planks", 100);
            World.Add(v.Stockpiles[0].Goods, "blocks", 100);
            var spill = new Stockpile { Id = w.Id("spill"), Kind = "spill", Position = new Int2(2, 2), Goods = new List<Stack> { new Stack("logs", 8) } };
            v.Stockpiles.Add(spill);
            Act(w, v, new GameAction { Kind = "RequestJob", Name = "hunt", CatId = c.Id });
            w.Apply(Context(v), new GameAction { Kind = "PlanBuilding", Name = "wood_cutter", Position = new Int2(-5, 1), CatId = c.Id });
            Valid(w);
            Check(v.Jobs.Count(j => !j.Completed && j.CatId == c.Id) <= 1, "Builder replaced job without releasing owner");
            w.Apply(Context(v), new GameAction { Kind = "HaulGatherSpot", TargetId = spill.Id, CatId = c.Id });
            Valid(w);
        }
        static void ItemPile()
        {
            var w = Fixture(out var v, out var c);
            var p = v.Stockpiles[0];
            p.Goods.Clear();
            var item = new Item { Id = w.Id("tool"), Kind = "tool", Material = "wood", VillageId = v.Id, LocationId = p.Id };
            v.Items.Add(item);
            Act(w, v, new GameAction { Kind = "RemoveStockpile", TargetId = p.Id });
            Check(v.Items.Single().Id == item.Id && v.Stockpiles.Any(x => x.Id == item.LocationId), "Removing item-only pile orphaned item");
        }
        static Stockpile ArmorPile(World w, Village v)
        {
            var at = new Int2(-5, 1);
            var result = Act(w, v, new GameAction { Kind = "DesignateStockpile", Position = at, End = at, Accepts = new List<string> { "armor" } });
            return v.Stockpiles.Single(p => p.Id == result.EntityId);
        }
        static void ArmorProductionStorage()
        {
            var w = Fixture(out var v, out var c);
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            var recipe = Catalog.Recipe("smithy_armor");
            var station = Station(w, v, recipe.Building);
            var source = v.Stockpiles[0];
            source.Accepts.AddRange(new[] { "food", "water", "metal" });
            World.Add(source.Goods, "metal", w.RecipeInput(v, recipe, recipe.Inputs.Single().Amount));
            var destination = ArmorPile(w, v);
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = station.Id });
            Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = recipe.Id });
            for (int tick = 0; tick < 1800 && v.Items.Count == 0; tick++)
                w.Step(1);
            Check(v.Items.Count == 1, "Armor recipe did not create exactly one physical item");
            var item = v.Items.Single();
            string id = item.Id;
            double condition = item.Condition, maximum = item.MaxCondition;
            Check(item.Kind == "armor" && item.LocationId != destination.Id, "Fixture must observe produced armor before physical delivery");
            for (int tick = 0; tick < 100 && !v.Jobs.Any(j => j.Kind == "production" && j.Completed); tick++)
                w.Step(1);
            Check(v.Jobs.Any(j => j.Kind == "production" && j.Completed), "Armor production is blocked despite reachable armor-only storage");
            Check(v.Items.Count == 1 && v.Items.Single().Id == id && item.LocationId == destination.Id, "Armor delivery changed identity or missed accepting storage");
            Near(item.Condition, condition, "Armor delivery changed condition");
            Near(item.MaxCondition, maximum, "Armor delivery changed maximum condition");
            Near(Goods(w, v, "armor"), 0, "Armor delivery duplicated an exact item as scalar goods");
            Near(Goods(w, v, "metal"), 0, "Armor production did not consume its exact material bill");
            Valid(w);
        }
        static void ArmorUnequipStorage()
        {
            var w = Fixture(out var v, out var c);
            v.Stockpiles[0].Accepts.AddRange(new[] { "food", "water" });
            var destination = ArmorPile(w, v);
            var item = new Item { Id = w.Id("armor"), Kind = "armor", Material = "metal", Quality = 2, VillageId = v.Id, LocationId = destination.Id, Condition = 37.5, MaxCondition = 73 };
            v.Items.Add(item);
            string id = item.Id;
            Act(w, v, new GameAction { Kind = "EquipItem", CatId = c.Id, TargetId = id });
            Check(c.Equipment.Contains(id) && item.LocationId == c.Id, "Armor did not enter the selected cat's equipment slot");
            Act(w, v, new GameAction { Kind = "UnequipItem", CatId = c.Id, TargetId = id });
            Check(v.Items.Count == 1 && v.Items.Single().Id == id && item.LocationId == destination.Id && !c.Equipment.Contains(id), "Unequipping lost exact armor identity or failed to use accepting storage");
            Near(item.Condition, 37.5, "Unequipping changed armor condition");
            Near(item.MaxCondition, 73, "Unequipping changed armor maximum condition");
            Near(Goods(w, v, "armor"), 0, "Unequipping duplicated an exact item as scalar goods");
            Valid(w);
        }
        static Stockpile MugPile(World w, Village v, Int2 at)
        {
            var result = Act(w, v, new GameAction { Kind = "DesignateStockpile", Position = at, End = at });
            return v.Stockpiles.Single(p => p.Id == result.EntityId);
        }
        static World ExactHaulFixture(out Village v, out Cat c, out Item item, out Stockpile source, out Stockpile destination)
        {
            var w = Fixture(out v, out c);
            c.BuildingId = "";
            v.Stockpiles[0].Accepts.AddRange(new[] { "food", "water" });
            source = MugPile(w, v, new Int2(-5, 1));
            destination = MugPile(w, v, new Int2(5, 1));
            item = new Item { Id = w.Id("mug"), Kind = "mug", Material = "clay", Quality = 2, Condition = 54.25, MaxCondition = 120, VillageId = v.Id, LocationId = source.Id };
            v.Items.Add(item);
            return w;
        }
        static void FinishExactHaul(World w, Village v, Cat c, Job job, Item item, Stockpile destination)
        {
            string id = item.Id, material = item.Material;
            double condition = item.Condition, maximum = item.MaxCondition;
            int quality = item.Quality;
            bool carried = false;
            for (int tick = 0; tick < 300 && !job.Completed; tick++)
            {
                w.Step(1);
                carried |= job.Phase == "output_delivery" && item.LocationId == job.Id && job.ItemIds.Contains(id);
            }
            Check(job.Completed && carried, "Exact haul did not physically pick up and carry its item before completing");
            Check(v.Items.Count(i => i.Id == id) == 1 && item.LocationId == destination.Id && job.ItemIds.Count == 0, "Exact haul did not release one unchanged identity to its destination");
            Check(item.Material == material && item.Quality == quality, "Exact haul changed item material or quality");
            Near(item.Condition, condition, "Exact haul changed item condition");
            Near(item.MaxCondition, maximum, "Exact haul changed maximum condition");
            Near(Goods(w, v, "mugs"), 0, "Exact haul created scalar item copies");
            Valid(w);
        }
        static void ProducedItemRecovery(bool interruptProduction)
        {
            var w = Fixture(out var v, out var c);
            v.Research = Catalog.Research.Select(n => n.Id).ToList();
            v.Trader.Phase = "trading";
            v.Trader.Until = 100000;
            var recipe = Catalog.Recipe("clay_mug");
            var station = Station(w, v, recipe.Building);
            var source = MugPile(w, v, new Int2(-5, 1));
            v.Stockpiles[0].Accepts.AddRange(new[] { "food", "water", "clay" });
            World.Add(v.Stockpiles[0].Goods, "clay", w.RecipeInput(v, recipe, recipe.Inputs.Single().Amount));
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id, BuildingId = station.Id });
            Act(w, v, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "add", RecipeId = recipe.Id });
            for (int tick = 0; tick < 1800 && !(interruptProduction ? v.Items.Count > 0 : v.Jobs.Any(j => j.Kind == "production" && j.Completed)); tick++)
                w.Step(1);
            var item = v.Items.Single();
            string id = item.Id;
            Check(item.Kind == "mug", "Production did not create a mug");
            Check(interruptProduction ? v.Jobs.Any(j => !j.Completed && j.ItemIds.Contains(id) && item.LocationId == j.Id) : item.LocationId == source.Id, "Produced mug did not reach the requested interruption stage");
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "RemoveStockpile", TargetId = source.Id });
            if (interruptProduction)
                source = v.Stockpiles.Single(p => p.Id == item.LocationId);
            Check(source.Kind == "spill" && source.Goods.Count == 0 && item.LocationId == source.Id, "Removing mug-only storage did not preserve the exact item in a spill");
            var destination = MugPile(w, v, new Int2(5, 1));
            Act(w, v, new GameAction { Kind = "HaulGatherSpot", CatId = c.Id, TargetId = source.Id });
            var haul = v.Jobs.Single(j => !j.Completed && j.CatId == c.Id);
            FinishExactHaul(w, v, c, haul, item, destination);
            double coins = v.Coins;
            Act(w, v, new GameAction { Kind = "SellGoods", TargetId = id });
            Check(v.Items.All(i => i.Id != id) && v.Trader.Items.Count(i => i.Id == id) == 1 && v.Coins > coins, "Recovered produced mug could not be sold under its original identity");
        }
        static void ExactHaulInterruption(bool pickedUp, bool death)
        {
            var w = ExactHaulFixture(out var v, out var c, out var item, out var source, out var destination);
            var helper = v.Cats[1];
            helper.ControlledBy = "";
            helper.BuildingId = "fixture-held";
            Act(w, v, new GameAction { Kind = "RemoveStockpile", TargetId = source.Id });
            Act(w, v, new GameAction { Kind = "HaulGatherSpot", CatId = c.Id, TargetId = source.Id });
            var job = v.Jobs.Single(j => !j.Completed && j.CatId == c.Id);
            if (pickedUp)
                for (int tick = 0; tick < 100 && job.Phase != "output_delivery"; tick++)
                    w.Step(1);
            if (pickedUp)
                for (int tick = 0; tick < 10 && c.Position.Equals(source.Position) && !job.Completed; tick++)
                    w.Step(1);
            Check(pickedUp ? job.Phase == "output_delivery" && !job.Completed && !c.Position.Equals(source.Position) : !c.Position.Equals(source.Position), "Exact haul fixture did not reach requested interruption stage");
            var carrierPosition = c.Position;
            if (death)
            {
                c.Health = 0;
                w.Step(1);
                Check(!c.Alive, "Exact carrier death did not use the normal lifecycle");
            }
            else
                Act(w, v, new GameAction { Kind = "EnterCat", CatId = c.Id });
            Check(job.Completed && job.ItemIds.Count == 0 && v.Items.Count(i => i.Id == item.Id) == 1, "Interrupted exact haul retained a claim or lost its item");
            var recovered = v.Stockpiles.Single(p => p.Id == item.LocationId);
            Check(pickedUp ? recovered.Position.Equals(carrierPosition) : recovered.Id == source.Id, "Interruption teleported exact cargo before pickup or lost its physical carrier position");
            Near(item.Condition, 54.25, "Interruption changed exact item condition");
            Near(item.MaxCondition, 120, "Interruption changed maximum condition");
            Act(w, v, new GameAction { Kind = "AssignWorker", CatId = helper.Id });
            Act(w, v, new GameAction { Kind = "HaulGatherSpot", CatId = helper.Id, TargetId = recovered.Id });
            var resumed = v.Jobs.Single(j => !j.Completed && j.CatId == helper.Id);
            FinishExactHaul(w, v, helper, resumed, item, destination);
        }
        static void ExactHaulClaims()
        {
            var w = ExactHaulFixture(out var v, out var c, out var item, out var source, out var destination);
            var helper = v.Cats[1];
            helper.ControlledBy = "";
            helper.BuildingId = "";
            var filler = new Item { Id = w.Id("mug"), Kind = "mug", VillageId = v.Id, LocationId = destination.Id };
            v.Items.Add(filler);
            destination.Capacity = 1;
            Act(w, v, new GameAction { Kind = "RemoveStockpile", TargetId = source.Id });
            Check(!w.Apply(Context(v), new GameAction { Kind = "HaulGatherSpot", CatId = c.Id, TargetId = source.Id }).Success && item.LocationId == source.Id, "Full destination accepted or claimed an undeliverable item");
            Act(w, v, new GameAction { Kind = "RemoveStockpile", TargetId = destination.Id });
            destination = MugPile(w, v, new Int2(5, 4));
            Act(w, v, new GameAction { Kind = "HaulGatherSpot", CatId = c.Id, TargetId = source.Id });
            var job = v.Jobs.Single(j => !j.Completed && j.CatId == c.Id);
            Check(item.LocationId == job.Id && job.ItemIds.SequenceEqual(new[] { item.Id }), "Exact haul did not claim one identity exclusively");
            v.Trader.Phase = "trading";
            v.Trader.Until = 100000;
            Check(!w.Apply(Context(v), new GameAction { Kind = "SellGoods", TargetId = item.Id }).Success && item.LocationId == job.Id, "Trader sold an exact item already claimed for physical hauling");
            Check(!w.Apply(Context(v), new GameAction { Kind = "HaulGatherSpot", CatId = helper.Id, TargetId = source.Id }).Success, "Second carrier double-claimed an exact item");
            Check(!w.Apply(Context(v), new GameAction { Kind = "RemoveGatherSpot", TargetId = source.Id }).Success, "Removing a source discarded a pending exact pickup");
            Act(w, v, new GameAction { Kind = "RemoveStockpile", TargetId = destination.Id });
            for (int tick = 0; tick < 100 && job.Phase != "output_delivery"; tick++)
                w.Step(1);
            w.Step(10);
            Check(!job.Completed && item.LocationId == job.Id && job.ItemIds.Count == 1 && c.BlockedReason == "output_storage_full_or_unreachable", "Losing accepting storage discarded carried exact cargo");
            destination = MugPile(w, v, new Int2(5, 6));
            FinishExactHaul(w, v, c, job, item, destination);
            Check(filler.LocationId != destination.Id, "Recovery silently moved an unrelated exact item");
        }
        static void ExactHaulSteward()
        {
            var w = ExactHaulFixture(out var v, out var c, out var item, out var source, out var destination);
            Station(w, v, "workshop", new Int2(5, 6));
            v.Research.Add("basic_tools");
            Act(w, v, new GameAction { Kind = "AssignOfficer", Role = "steward", CatId = c.Id });
            Act(w, v, new GameAction { Kind = "RemoveStockpile", TargetId = source.Id });
            w.Step(10);
            var job = v.Jobs.SingleOrDefault(j => !j.Completed && j.Kind == "haul" && j.SourceId == source.Id);
            Check(job != null && job.AutomatedBy == "steward" && item.LocationId == job.Id, "Steward did not claim a recoverable item-only spill");
            FinishExactHaul(w, v, c, job, item, destination);
        }
        static void ExactHaulSourceCapacity()
        {
            var w = ExactHaulFixture(out var v, out var c, out var item, out var source, out var destination);
            source.Capacity = 1;
            Check(!w.HasRoom(v, source, "logs", 1), "Fixture item did not occupy its physical source capacity");
            Act(w, v, new GameAction { Kind = "HaulGatherSpot", CatId = c.Id, TargetId = source.Id });
            var job = v.Jobs.Single(j => !j.Completed && j.CatId == c.Id);
            Check(job.Phase == "item_fetch" && item.LocationId == job.Id && !c.Position.Equals(source.Position), "Stored item fixture did not reserve before physical pickup");
            Check(!w.HasRoom(v, source, "logs", 1), "Exact item claim freed source capacity before physical pickup");
            for (int tick = 0; tick < 100 / World.SimulationStepSeconds && job.Phase == "item_fetch"; tick++)
                w.Step(World.SimulationStepSeconds);
            Check(job.Phase == "output_delivery" && c.Position.Equals(source.Position) && w.HasRoom(v, source, "logs", 1), "Physical pickup did not free the source capacity");
            FinishExactHaul(w, v, c, job, item, destination);
        }
        static World ScalarHaulFixture(double quantity, double sourceCapacity, double destinationCapacity, out Village v, out Cat c, out Stockpile source, out Stockpile destination)
        {
            var w = Fixture(out v, out c);
            c.BuildingId = "";
            v.Stockpiles[0].Accepts.AddRange(new[] { "food", "water" });
            source = new Stockpile { Id = w.Id("scalar-source"), Position = new Int2(-5, 1), Width = 1, Depth = 1, Capacity = sourceCapacity, Accepts = new List<string> { "logs" }, Goods = new List<Stack> { new Stack("logs", quantity) } };
            destination = new Stockpile { Id = w.Id("scalar-destination"), Position = new Int2(5, 1), Width = 1, Depth = 1, Capacity = destinationCapacity, Accepts = new List<string> { "logs" } };
            v.Stockpiles.Add(source);
            v.Stockpiles.Add(destination);
            return w;
        }
        static void ScalarHaulStoredTransfer()
        {
            var w = ScalarHaulFixture(12, 100, 100, out var v, out var c, out var source, out var destination);
            c.Cargo.Add(new Stack("stone", 3));
            var origin = c.Position;
            var result = Act(w, v, new GameAction { Kind = "HaulGatherSpot", TargetId = source.Id });
            var job = v.Jobs.Single(j => j.Id == result.EntityId);
            Check(job.CatId == c.Id && job.SourceId == source.Id && job.TargetId == destination.Id && job.Amount == 8 && job.Phase == "fetch", "Stores Haul did not assign another accepting store to its available carrier");
            Near(World.Amount(source.Goods, "logs"), 12, "Scalar haul removed source goods before physical pickup");
            Near(w.Reservations.Where(r => r.OwnerId == job.Id && r.PileId == source.Id && r.Resource == "logs").Sum(r => r.Amount), 8, "Scalar haul did not exclusively claim its finite load");
            Check(!w.Apply(Context(v), new GameAction { Kind = "RemoveStockpile", TargetId = source.Id }).Success && source.Kind == "storage", "Source removal discarded a pending scalar pickup");
            var unrelated = v.Stockpiles.Single(p => p.Kind == "spill" && p.Position.Equals(origin) && World.Amount(p.Goods, "stone") == 3);
            Check(c.Cargo.Count == 0, "New scalar haul kept unrelated carried goods mixed with its claim");
            for (int tick = 0; tick < 100 / World.SimulationStepSeconds && World.Amount(c.Cargo, "logs") == 0; tick++)
                w.Step(World.SimulationStepSeconds);
            Check(c.Position.Equals(source.Position) && World.Amount(c.Cargo, "logs") == 8 && !w.Reservations.Any(r => r.OwnerId == job.Id), "Scalar source stock did not become the same cat's physical cargo");
            Near(World.Amount(source.Goods, "logs"), 4, "Scalar pickup debited the wrong amount");
            Near(World.Amount(destination.Goods, "logs"), 0, "Scalar pickup credited delivery before travel");
            for (int tick = 0; tick < 100 && !job.Completed; tick++)
                w.Step(1);
            Check(job.Completed && c.JobId == "" && c.Cargo.Count == 0 && source.Kind == "storage" && v.Stockpiles.Contains(source), "Scalar haul did not finish while preserving its source store");
            Near(World.Amount(source.Goods, "logs"), 4, "Scalar delivery returned cargo to the original store");
            Near(World.Amount(destination.Goods, "logs"), 8, "Scalar haul did not physically deliver to a different store");
            Near(Goods(w, v, "logs"), 12, "Scalar store transfer lost or duplicated goods");
            Near(World.Amount(unrelated.Goods, "stone"), 3, "Scalar haul consumed unrelated cargo");
            Valid(w);
        }
        static void ScalarHaulPartialReservations()
        {
            var w = ScalarHaulFixture(12, 12, 8, out var v, out var c, out var source, out var destination);
            var small = new Stockpile { Id = w.Id("small-destination"), Position = new Int2(5, 4), Width = 1, Depth = 1, Capacity = 4, Accepts = new List<string> { "logs" } };
            v.Stockpiles.Add(small);
            var second = v.Cats[1];
            second.ControlledBy = "";
            second.BuildingId = "";
            var third = v.Cats[2];
            third.ControlledBy = "";
            third.BuildingId = "";
            var firstId = Act(w, v, new GameAction { Kind = "HaulGatherSpot", TargetId = source.Id, CatId = c.Id }).EntityId;
            var first = v.Jobs.Single(j => j.Id == firstId);
            Check(first.TargetId == destination.Id && first.Amount == 8, "Initial scalar haul did not reserve its eight-unit destination");
            Act(w, v, new GameAction { Kind = "RemoveStockpile", TargetId = destination.Id });
            var secondId = Act(w, v, new GameAction { Kind = "HaulGatherSpot", TargetId = source.Id, CatId = second.Id }).EntityId;
            var partial = v.Jobs.Single(j => j.Id == secondId);
            Check(partial.Amount == 4 && partial.TargetId == small.Id, "Partial scalar claim was tested against an eight-unit capacity instead of its unclaimed amount");
            Near(w.Reservations.Where(r => r.PileId == source.Id && r.Resource == "logs").Sum(r => r.Amount), 12, "Competing scalar hauls duplicated or dropped source claims");
            int jobs = v.Jobs.Count, claims = w.Reservations.Count;
            Check(!w.Apply(Context(v), new GameAction { Kind = "HaulGatherSpot", TargetId = source.Id, CatId = third.Id }).Success && v.Jobs.Count == jobs && w.Reservations.Count == claims, "A third carrier double-claimed the already reserved source");
            Near(World.Amount(source.Goods, "logs"), 12, "Competing claims removed goods before pickup");
            for (int tick = 0; tick < 100 && !(partial.Completed && first.Phase == "output_delivery" && c.BlockedReason == "output_storage_full_or_unreachable"); tick++)
                w.Step(1);
            Check(partial.Completed && !first.Completed && World.Amount(c.Cargo, "logs") == 8 && c.BlockedReason == "output_storage_full_or_unreachable", "Changed destinations did not retain the larger load while delivering the fitting partial load");
            Near(World.Amount(small.Goods, "logs"), 4, "Partial reservation did not reach its four-unit store");
            Near(World.Amount(source.Goods, "logs"), 0, "Blocked delivery silently returned goods to the source");
            Near(Goods(w, v, "logs"), 12, "Competing scalar hauls changed conserved goods");
            Check(w.Reservations.Count == 0, "Physical pickups left stale source reservations");
            var recoveredId = Act(w, v, new GameAction { Kind = "DesignateStockpile", Position = destination.Position, End = destination.Position, Accepts = new List<string> { "logs" } }).EntityId;
            var recovered = v.Stockpiles.Single(p => p.Id == recoveredId);
            for (int tick = 0; tick < 100 && !first.Completed; tick++)
                w.Step(1);
            Check(first.Completed && first.Id == firstId && c.JobId == "" && c.Cargo.Count == 0 && source.Kind == "storage", "Replacement storage did not resume the same held scalar haul");
            Near(World.Amount(recovered.Goods, "logs"), 8, "Replacement destination did not receive the retained larger load");
            Near(World.Amount(small.Goods, "logs"), 4, "Recovery changed the delivered partial load");
            Near(Goods(w, v, "logs"), 12, "Destination replacement lost or duplicated scalar cargo");
            Valid(w);
        }
        static void ScalarHaulAvoidReroute(bool partialClaim)
        {
            var w = ScalarHaulFixture(partialClaim ? 12 : 8, 100, 8, out var v, out var c, out var source, out var destination);
            var alternate = new Stockpile { Id = w.Id("alternate-store"), Position = new Int2(5, 4), Width = 1, Depth = 1, Capacity = partialClaim ? 4 : 8, Accepts = new List<string> { "logs" } };
            v.Stockpiles.Add(alternate);
            var helper = v.Cats[1];
            Job competing = null;
            if (partialClaim)
            {
                helper.ControlledBy = "";
                helper.BuildingId = "";
                helper.Position = c.Position;
                helper.X = c.X;
                helper.Z = c.Z;
                var claimed = Act(w, v, new GameAction { Kind = "HaulGatherSpot", CatId = helper.Id, TargetId = source.Id });
                competing = v.Jobs.Single(j => j.Id == claimed.EntityId);
            }
            var result = Act(w, v, new GameAction { Kind = "HaulGatherSpot", CatId = c.Id, TargetId = source.Id });
            var job = v.Jobs.Single(j => j.Id == result.EntityId);
            double load = partialClaim ? 4 : 8;
            string identity = job.Id, carrier = c.Id;
            Check(job.Amount == load && job.TargetId == destination.Id && job.Position.Equals(destination.Position), "Alternate-store fixture did not initially select the nearer original store");
            Near(w.Reservations.Where(r => r.PileId == source.Id).Sum(r => r.Amount), partialClaim ? 12 : 8, "Initial scalar source claims changed");
            for (int tick = 0; tick < 100 && !(World.Amount(c.Cargo, "logs") == load && (!partialClaim || World.Amount(helper.Cargo, "logs") == 8)); tick++)
                w.Step(1);
            w.Step(1);
            Check(job.Phase == "input_delivery" && World.Amount(c.Cargo, "logs") == load && World.Amount(source.Goods, "logs") == 0, "Alternate-store fixture did not reach carried input_delivery");
            var zone = Act(w, v, new GameAction { Kind = "CreateZone", Resource = "avoid", Position = destination.Position, End = destination.Position }).EntityId;
            Check(!w.Walkable(v, destination.Position) && w.Path(c.Position, alternate.Position, v) != null, "Public avoid zone did not block only the original receiving store");
            for (int tick = 0; tick < 100 && !job.Completed; tick++)
            {
                w.Step(1);
                Check(v.Stockpiles.Any(p => p.Id == zone) && !w.Walkable(v, destination.Position), "Haul recovery removed the original store's obstruction");
                Check(job.Id == identity && job.CatId == carrier && (job.Completed || c.JobId == identity), "Alternate-store recovery replaced its job or carrier");
                Near(Goods(w, v, "logs"), partialClaim ? 12 : 8, "Alternate-store recovery lost or duplicated scalar goods");
                Near(World.Amount(source.Goods, "logs"), 0, "Alternate-store recovery returned picked-up goods to source");
            }
            Check(job.Completed && job.TargetId == alternate.Id && job.Position.Equals(alternate.Position) && c.Position.Equals(alternate.Position) && c.Cargo.Count == 0, "Loaded scalar haul remained stuck at avoided store instead of physically retargeting");
            Check(source.Kind == "storage" && destination.Kind == "storage" && v.Stockpiles.Contains(source) && v.Stockpiles.Contains(destination) && v.Stockpiles.Contains(alternate), "Retargeting replaced a store identity");
            Near(World.Amount(destination.Goods, "logs"), 0, "Haul delivered into the avoided store");
            Near(World.Amount(alternate.Goods, "logs"), load, "Alternate store did not receive the actual carried quantity");
            Check(w.Reservations.Count == 0, "Retargeting recreated consumed source claims");
            if (partialClaim)
                Check(!competing.Completed && World.Amount(helper.Cargo, "logs") == 8 && helper.JobId == competing.Id && helper.BlockedReason == "blocked_route", "Four-unit alternate accepted or released the competing eight-unit load");
            Valid(w);
        }
        static void ScalarHaulFullDestination()
        {
            var w = ScalarHaulFixture(8, 8, 8, out var v, out var c, out var source, out var destination);
            var helper = v.Cats[1];
            helper.ControlledBy = "";
            helper.BuildingId = "fixture-held";
            helper.Position = new Int2(5, 2);
            helper.X = 5;
            helper.Z = 2;
            helper.Cargo.Add(new Stack("logs", 8));
            var id = Act(w, v, new GameAction { Kind = "HaulGatherSpot", TargetId = source.Id, CatId = c.Id }).EntityId;
            var job = v.Jobs.Single(j => j.Id == id);
            Act(w, v, new GameAction { Kind = "EnterCat", CatId = helper.Id });
            Act(w, v, new GameAction { Kind = "InteractCat", CatId = helper.Id, TargetId = destination.Id });
            void Tick(int count)
            {
                for (int tick = 0; tick < count; tick++)
                {
                    if (helper.ControlLeaseUntil - w.TimeSeconds <= 2)
                        Act(w, v, new GameAction { Kind = "KeepCatControl", CatId = helper.Id });
                    w.Step(1);
                }
            }
            for (int tick = 0; tick < 100 / World.SimulationStepSeconds && World.Amount(c.Cargo, "logs") == 0; tick++)
            {
                if (helper.ControlLeaseUntil - w.TimeSeconds <= 2)
                    Act(w, v, new GameAction { Kind = "KeepCatControl", CatId = helper.Id });
                w.Step(World.SimulationStepSeconds);
            }
            Check(World.Amount(c.Cargo, "logs") == 8 && c.Position.Equals(source.Position), "Full-destination fixture did not perform its finite source pickup");
            var edges = new[] { new Int2(4, 1), new Int2(6, 1), new Int2(5, 0), new Int2(5, 2) }.Select(p => new BoundaryEdge { From = destination.Position, To = p }).ToArray();
            foreach (var edge in edges)
                v.BoundaryEdges.Add(edge);
            Tick(4);
            Check(!job.Completed && World.Amount(c.Cargo, "logs") == 8 && c.BlockedReason == "blocked_route", "New destination obstruction bypassed the carrier's physical route");
            Near(World.Amount(source.Goods, "logs"), 0, "Blocked carrier returned its picked-up load to source");
            foreach (var edge in edges)
                v.BoundaryEdges.Remove(edge);
            Tick(25);
            Check(!job.Completed && job.Phase == "output_delivery" && c.BlockedReason == "output_storage_full_or_unreachable" && World.Amount(c.Cargo, "logs") == 8, "Full destination returned scalar cargo to its source instead of retaining it");
            Near(World.Amount(source.Goods, "logs"), 0, "Full destination refilled the source store");
            Near(World.Amount(destination.Goods, "logs"), 8, "Waiting haul overfilled the destination");
            Near(Goods(w, v, "logs"), 16, "Blocked/full destination changed finite cargo totals");
            Act(w, v, new GameAction { Kind = "InteractCat", CatId = helper.Id, TargetId = destination.Id, Resource = "logs", Amount = 8 });
            for (int tick = 0; tick < 100 && !job.Completed; tick++)
                Tick(1);
            Check(job.Completed && job.Id == id && c.Cargo.Count == 0 && c.JobId == "" && source.Kind == "storage", "Freed destination did not finish the original scalar haul");
            Near(World.Amount(destination.Goods, "logs"), 8, "Freed destination did not receive held cargo exactly once");
            Near(World.Amount(helper.Cargo, "logs"), 8, "Haul consumed the helper's unrelated withdrawn goods");
            Near(World.Amount(source.Goods, "logs"), 0, "Completed haul restored spent source goods");
            Near(Goods(w, v, "logs"), 16, "Destination recovery lost or duplicated scalar goods");
            Valid(w);
        }
        static void ExactHaulStoredTransfer()
        {
            var w = ExactHaulFixture(out var v, out var c, out var item, out var source, out var destination);
            Check(w.HasRoom(v, source, "logs", 1), "Fixture source must retain spare storage capacity");
            Act(w, v, new GameAction { Kind = "HaulGatherSpot", CatId = c.Id, TargetId = source.Id });
            var job = v.Jobs.Single(j => !j.Completed && j.CatId == c.Id);
            FinishExactHaul(w, v, c, job, item, destination);
            Check(source.Kind == "storage" && item.LocationId != source.Id, "Exact haul returned its cargo to the original source instead of moving it");
        }
    }
}
