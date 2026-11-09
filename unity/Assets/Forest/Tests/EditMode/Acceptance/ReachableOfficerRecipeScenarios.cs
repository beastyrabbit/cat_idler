using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class ReachableOfficerRecipeScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.reachable_officer_recipe_fresh_choice", () => ReachableChoice(false));
            yield return new Scenario("regression.reachable_officer_recipe_stale_automatic_choice", () => ReachableChoice(true));
            yield return new Scenario("regression.reachable_officer_recipe_preserves_manual_queue", PreserveManualQueue);
            yield return new Scenario("regression.reachable_officer_recipe_respects_existing_claim", RespectExistingClaim);
            yield return new Scenario("regression.reachable_officer_recipe_counts_staged_inputs", CountStagedInputs);
            yield return new Scenario("regression.reachable_officer_recipe_accepts_footprint_access", AcceptFootprintAccess);
            yield return new Scenario("regression.reachable_officer_recipe_requires_station_delivery", RequireStationDelivery);
            yield return new Scenario("regression.reachable_officer_recipe_preserves_pause", PreservePause);
            yield return new Scenario("acceptance.reachable_officer_recipe_finite_material_chain", FiniteMaterialChain);
            yield return new Scenario("regression.mill_food_preflight_uses_researched_output", FlourFoodPreflightUsesResearchedOutput);
            yield return new Scenario("acceptance.mill_food_capacity_recovery", MillFoodCapacityRecovery);
            yield return new Scenario("regression.mill_food_multiload_preflight", MillFoodMultiloadPreflight);
            yield return new Scenario("acceptance.mill_food_multiload_delivery", MillFoodMultiloadDelivery);
        }

        static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
        static void Act(World world, Village village, GameAction action)
        {
            var result = world.Apply(new PlayerContext { PlayerId = "reachable-officer-acceptance", VillageId = village.Id }, action);
            Check(result.Success, action.Kind + ": " + result.Error);
        }

        static World Fixture(out Village village, out Building station, out Cat worker)
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
            worker = village.Cats[1];
            worker.ControlledBy = "";
            station = new Building { Id = world.Id("reachable-officer-station"), Kind = "clothier", Position = worker.Position, Completed = true };
            village.Buildings.Add(station);
            village.Research.Add("textiles");
            Act(world, village, new GameAction { Kind = "AssignOfficer", Role = "cloth_leader", CatId = village.Cats[0].Id });
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = worker.Id });
            station.Slots.Single().AutomatedBy = "cloth_leader";
            return world;
        }

        static Stockpile Supply(Village village, string id, string resource, double amount, Int2 at, int width = 1)
        {
            var pile = new Stockpile { Id = id, Position = at, Width = width, Depth = 1, Capacity = 100000, Goods = new List<Stack> { new Stack(resource, amount) } };
            village.Stockpiles.Add(pile);
            return pile;
        }

        static World SelectReachableRecipe(bool stale, out Village village, out Building station, out Cat worker, out Stockpile blocked, out Job job)
        {
            var world = Fixture(out village, out station, out worker);
            blocked = Supply(village, "aaa-blocked-thread", "thread", 5, new Int2(-3, 2));
            var reachable = Supply(village, "zzz-reachable-fibre", "fibre", 5, new Int2(3, 3));
            world.TileAt(blocked.Position).Wall = true;
            if (stale)
                station.Slots.Single().Queue.Add(new QueueEntry { RecipeId = "fibre_to_cloth", AutomatedBy = "cloth_leader" });
            world.Step(11.05);
            string workerId = worker.Id;
            job = village.Jobs.SingleOrDefault(j => !j.Completed && j.Kind == "production" && j.CatId == workerId);
            Check(job != null && job.RecipeId == "fibre_to_thread", stale ? "Inaccessible global Thread pinned the old automatic Cloth queue instead of choosing reachable Fibre" : "The officer chose inaccessible Thread instead of funding its alternative from reachable Fibre");
            Check(station.Slots.Single().Queue.Single().RecipeId == "fibre_to_thread" && station.Slots.Single().Queue.Single().AutomatedBy == "cloth_leader", "Reachable automatic choice lost the selected queue's officer ownership");
            string blockedId = blocked.Id;
            Check(World.Amount(blocked.Goods, "thread") == 5 && world.Reservations.All(r => r.PileId != blockedId), "Recipe choice claimed or consumed inaccessible finite Thread");
            Check(World.Amount(reachable.Goods, "fibre") + World.Amount(worker.Cargo, "fibre") + World.Amount(job.Local, "fibre") == 5, "Recipe choice changed the five finite reachable Fibre");
            return world;
        }

        static void ReachableChoice(bool stale)
        {
            var world = SelectReachableRecipe(stale, out _, out _, out _, out _, out _);
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void PreserveManualQueue()
        {
            var world = Fixture(out var village, out var station, out var worker);
            var blocked = Supply(village, "blocked-manual-thread", "thread", 5, new Int2(-3, 2));
            Supply(village, "reachable-manual-fibre", "fibre", 5, new Int2(3, 3));
            world.TileAt(blocked.Position).Wall = true;
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = station.Id, CatId = worker.Id, Edit = "add", RecipeId = "fibre_to_cloth", Repeat = true });
            var slot = station.Slots.Single();
            var order = slot.Queue.Single();
            // Automatic staffing can own a position while retaining its manual order.
            slot.AutomatedBy = "cloth_leader";
            world.Step(21.05);
            Check(ReferenceEquals(slot.Queue.Single(), order) && order.Repeat && order.AutomatedBy == "" && worker.JobId == "", "Reachable alternatives replaced the player's deliberate inaccessible Cloth order");
            Check(world.Reservations.Count == 0 && world.Total(village, "fibre") == 5 && World.Amount(blocked.Goods, "thread") == 5, "Preserved manual work changed finite supplies or retained claims");
        }

        static void RespectExistingClaim()
        {
            var world = Fixture(out var village, out var station, out var worker);
            var claimed = Supply(village, "aaa-claimed-thread", "thread", 5, new Int2(2, 3));
            var blocked = Supply(village, "bbb-blocked-thread", "thread", 5, new Int2(-3, 2));
            Supply(village, "zzz-reachable-fibre", "fibre", 5, new Int2(3, 3));
            world.TileAt(blocked.Position).Wall = true;
            Check(world.Reserve(village, "other-work", "thread", 5), "Existing finite Thread claim could not be established");
            world.Step(11.05);
            var job = village.Jobs.SingleOrDefault(j => !j.Completed && j.CatId == worker.Id && j.Kind == "production");
            Check(job != null && job.RecipeId == "fibre_to_thread", "Automatic choice counted another owner's reachable Thread claim as free stock");
            Check(world.Reservations.Any(r => r.OwnerId == "other-work" && r.PileId == claimed.Id && r.Amount == 5) && World.Amount(claimed.Goods, "thread") == 5, "Automatic choice stole the preexisting finite Thread claim");
            world.Release("other-work");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void CountStagedInputs()
        {
            var world = Fixture(out var village, out var station, out var worker);
            World.Add(station.Inputs, "thread", 5);
            world.Step(11.05);
            var job = village.Jobs.SingleOrDefault(j => !j.Completed && j.CatId == worker.Id && j.Kind == "production");
            Check(job != null && job.RecipeId == "fibre_to_cloth" && World.Amount(job.Local, "thread") == 5, "The officer ignored already delivered finite Thread when no source pile held it");
            Check(station.Inputs.Count == 0 && world.Reservations.Count == 0, "Staged recipe funding duplicated inputs or requested an unnecessary claim");
        }

        static void AcceptFootprintAccess()
        {
            var world = Fixture(out var village, out var station, out var worker);
            var pile = Supply(village, "wide-thread", "thread", 5, new Int2(3, 3), 2);
            Supply(village, "reachable-fibre", "fibre", 5, new Int2(-3, 2));
            world.TileAt(pile.Position).Wall = true;
            world.Step(11.05);
            var job = village.Jobs.SingleOrDefault(j => !j.Completed && j.CatId == worker.Id && j.Kind == "production");
            Check(job != null && job.RecipeId == "fibre_to_cloth", "A reachable stockpile footprint tile did not fund the higher-deficit automatic Cloth recipe");
            Check(world.Reservations.Any(r => r.OwnerId == job.Id && r.PileId == pile.Id && r.Amount == 5), "The automatic Cloth order did not claim its accessible finite Thread");
        }

        static void RequireStationDelivery()
        {
            var world = Fixture(out var village, out var station, out var worker);
            Supply(village, "reachable-fibre", "fibre", 5, new Int2(3, 3));
            world.TileAt(station.Position).Wall = true;
            world.Step(11.05);
            Check(station.Slots.Single().Queue.Count == 0 && worker.JobId == "" && world.Reservations.Count == 0, "The officer queued ingredients its worker could not deliver to the station");
        }

        static void PreservePause()
        {
            var world = Fixture(out var village, out var station, out var worker);
            var blocked = Supply(village, "blocked-paused-thread", "thread", 5, new Int2(-3, 2));
            Supply(village, "reachable-paused-fibre", "fibre", 5, new Int2(3, 3));
            world.TileAt(blocked.Position).Wall = true;
            var order = new QueueEntry { RecipeId = "fibre_to_cloth", AutomatedBy = "cloth_leader" };
            station.Slots.Single().Queue.Add(order);
            station.Slots.Single().Paused = true;
            world.Step(21.05);
            Check(station.Slots.Single().Paused && ReferenceEquals(station.Slots.Single().Queue.Single(), order) && worker.JobId == "", "Recipe reachability rewrote the paused automatic order");
        }

        // Phase-one food preflight regression. Recovery awaits root-recorded RED.
        static void FlourFoodPreflightUsesResearchedOutput()
        {
            var world = World.Create(41);
            var village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            var provisions = village.Stockpiles[0];
            provisions.Position = new Int2(3, 2);
            provisions.Width = provisions.Depth = 1;
            provisions.Capacity = 10000;
            provisions.Accepts = new List<string> { "preserves", "water", "fibre" };
            provisions.Goods = new List<Stack> { new Stack("preserves", 1000), new Stack("water", 1000), new Stack("fibre", 10) };
            village.LastLeaderResearch = 0;
            foreach (var cat in village.Cats)
            {
                cat.AgeHours = 24;
                cat.Hunger = cat.Thirst = cat.Rest = cat.Health = 100;
                cat.Skills.Clear();
                cat.Preferences.Clear();
                cat.Boosted = false;
                cat.ControlledBy = "fixture-held";
                cat.ControlLeaseUntil = 10000000;
                cat.Position = new Int2(3, 0);
                cat.X = 3;
                cat.Z = 0;
            }
            var worker = village.Cats[1];
            worker.Name = "Food Miller";
            worker.ControlledBy = "";
            worker.ControlLeaseUntil = 0;
            var knownLogs = world.Tiles.First(t => t.Resource == "logs" && t.Amount > 0).Position;
            if (!village.Known.Contains(knownLogs)) village.Known.Add(knownLogs);

            var recipe = Catalog.Recipe("flour_to_food");
            Check(!Catalog.BuildingAvailable(village, "mill") && !Catalog.RecipeAvailable(village, recipe), "Food preflight fixture must begin without grandfathered Mill or Food recipe access");
            string[] studies = { "research_hut", "basic_tools", "foraging_lore", "sawmill", "masonry", "water_carriers", "irrigation", "milling", "grain_milling_sources", "grain_milling_preparation", "grain_milling_staples" };
            Check(village.Research.Count == 0 && studies.Sum(id => Catalog.Study(id).Cost) == 133.5, "Food preflight requires the exact finite legal study bill");
            village.ResearchPoints = 133.5;
            foreach (string study in studies)
                Act(world, village, new GameAction { Kind = "ResearchNode", NodeId = study });
            Check(village.ResearchPoints == 0 && village.Research.SequenceEqual(studies) && world.TimeSeconds == 0
                && Catalog.BuildingAvailable(village, "mill") && Catalog.RecipeAvailable(village, Catalog.Recipe("grain_to_flour")) && Catalog.RecipeAvailable(village, recipe), "Public purchases must unlock the Mill and both recipes without advancing time or adding unrelated studies");
            Check(World.RecipeFamily(recipe) == "grain_milling" && recipe.Inputs.Single().Resource == "flour" && recipe.Inputs.Single().Amount == 2
                && world.RecipeInput(village, recipe, 2) == 2 && recipe.Outputs.Single().Resource == "food" && recipe.Outputs.Single().Amount == 4, "Food preflight must use the real two-Flour/four-base-Food recipe");
            double output = world.RecipeOutput(village, recipe, recipe.Outputs.Single().Amount);
            Check(Math.Abs(output - 4.4) < 1e-9 && Math.Abs(output / 4 - 1.1) < 1e-9 && output <= world.CarryCapacity(village, "food"), "Legal Sources research must produce one physical 4.4-Food load");

            var mill = new Building { Id = world.Id("food-preflight-mill"), Kind = "mill", Position = new Int2(4, -4), Completed = true };
            village.Buildings.Add(mill);
            var flour = new Stockpile
            {
                Id = world.Id("food-preflight-flour"),
                Position = new Int2(3, -4),
                Width = 1,
                Depth = 1,
                Accepts = new List<string> { "flour" },
                Goods = new List<Stack> { new Stack("flour", 2) }
            };
            village.Stockpiles.Add(flour);
            var receiver = new Stockpile
            {
                Id = world.Id("food-preflight-receiver"),
                Position = new Int2(3, -3),
                Width = 1,
                Depth = 1,
                Accepts = new List<string> { "food", "stone" }
            };
            village.Stockpiles.Add(receiver);
            double capacity = world.ResourceCapacity(village, receiver, "food");
            receiver.Goods.Add(new Stack("stone", capacity - 4.2));
            Check(receiver.Capacity == 500 && capacity > receiver.Capacity && receiver.ResourceLimits.Count == 0
                && Math.Abs(capacity - receiver.Goods.Sum(s => s.Amount) - 4.2) < 1e-9, "Masonry-adjusted Food room must be exactly 4.2, without per-resource limits");
            Check(village.Stockpiles.Where(p => p.Kind == "storage" && (p.Accepts.Count == 0 || p.Accepts.Contains("food"))).SequenceEqual(new[] { receiver })
                && world.HasRoom(village, receiver, "food", 4) && !world.HasRoom(village, receiver, "food", output)
                && !world.HasRoom(village, provisions, "food", 4) && !world.HasRoom(village, flour, "food", 4), "Only the reachable receiver may accept Food, and it must fit base output but reject researched output");
            Check(world.Walkable(village, worker.Position) && world.Walkable(village, flour.Position) && world.Walkable(village, mill.Position) && world.Walkable(village, receiver.Position)
                && world.Path(worker.Position, flour.Position, village) != null && world.Path(flour.Position, mill.Position, village) != null
                && world.Path(worker.Position, mill.Position, village) != null && world.Path(mill.Position, receiver.Position, village) != null
                && world.Path(worker.Position, provisions.Position, village) != null && world.Path(provisions.Position, village.Center, village) != null, "Food preflight must have real worker, input, station, receiver and support routes");
            Check(mill.Slots.Count == 0 && village.Jobs.Count == 0 && world.Reservations.Count == 0
                && village.Cats.All(c => c.JobId == "" && c.BuildingId == "" && c.Cargo.Count == 0)
                && village.Buildings.All(b => b.Inputs.Count == 0 && b.Outputs.Count == 0), "Food preflight fixture must contain no injected work, cargo, staged inputs, outputs or claims");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));

            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = mill.Id, CatId = worker.Id });
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = mill.Id, CatId = worker.Id, Edit = "add", RecipeId = recipe.Id, Repeat = false });
            var slot = mill.Slots.Single();
            var order = slot.Queue.Single();
            Check(slot.CatId == worker.Id && mill.WorkerId == worker.Id && worker.BuildingId == mill.Id && slot.AutomatedBy == "" && !slot.Paused && !mill.Paused
                && order.RecipeId == recipe.Id && !order.Repeat && order.AutomatedBy == "", "Public actions must create the exact unpaused manual worker and one finite Food order");
            Check(world.TimeSeconds == 0 && world.Available(village, "flour") == 2 && World.Amount(flour.Goods, "flour") == 2 && world.Total(village, "flour") == 2
                && world.Total(village, "food") == 0 && world.Reservations.Count == 0 && village.Jobs.Count == 0 && worker.JobId == "" && slot.JobId == ""
                && world.StationCapacity(village, "mill") >= 2 && worker.Cargo.Count == 0 && mill.Inputs.Count == 0 && mill.Outputs.Count == 0
                && slot.Inputs.Count == 0 && slot.Outputs.Count == 0 && slot.Progress == 0, "Immediately before RED, exactly two unclaimed Flour must remain physically stored with no work or progress");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));

            // One planning boundary plus one actor quantum. No batch work is credited.
            world.Step(1.05);
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
            Check(world.TimeSeconds == 1.05 && slot.Queue.Count == 1 && ReferenceEquals(slot.Queue[0], order) && !order.Repeat && order.AutomatedBy == ""
                && slot.CatId == worker.Id && worker.BuildingId == mill.Id, "Insufficient output space must retain the same finite order and exact worker");
            Check(slot.BlockedReason == "output_storage_full" && mill.BlockedReason == "output_storage_full",
                "Researched 4.4-Food output must be blocked by the sole receiver's 4.2 room before a batch starts; actual slot blocker='" + slot.BlockedReason + "', production jobs=" + village.Jobs.Count(j => j.Kind == "production") + ", claims=" + world.Reservations.Count);
            Check(village.Jobs.Count == 0 && world.Reservations.Count == 0 && worker.JobId == "" && slot.JobId == "" && slot.Progress == 0,
                "Output preflight must refuse before creating a production job, input claim or work credit");
            Check(World.Amount(flour.Goods, "flour") == 2 && world.Total(village, "flour") == 2 && world.Available(village, "flour") == 2
                && world.Total(village, "food") == 0 && village.Cats.All(c => c.Cargo.Count == 0)
                && village.Buildings.All(b => b.Inputs.Count == 0 && b.Outputs.Count == 0 && b.Slots.All(s => s.Inputs.Count == 0 && s.Outputs.Count == 0))
                && World.Amount(receiver.Goods, "stone") == capacity - 4.2 && Math.Abs(world.ResourceCapacity(village, receiver, "food") - receiver.Goods.Sum(s => s.Amount) - 4.2) < 1e-9,
                "Blocked Food preflight must neither move nor consume finite Flour, create Food, stage goods or alter receiver room");
        }

        // Separate composed recovery; the phase-one RED/GREEN method above stays frozen.
        static void MillFoodCapacityRecovery()
        {
            var world = World.Create(41);
            var village = world.Villages[0];
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.RemoveRange(1, village.Stockpiles.Count - 1);
            var provisions = village.Stockpiles[0];
            provisions.Position = new Int2(3, 2);
            provisions.Width = provisions.Depth = 1;
            provisions.Capacity = 10000;
            provisions.Accepts = new List<string> { "preserves", "water", "fibre" };
            provisions.Goods = new List<Stack> { new Stack("preserves", 1000), new Stack("water", 1000), new Stack("fibre", 10) };
            village.LastLeaderResearch = 0;
            foreach (var cat in village.Cats)
            {
                cat.AgeHours = 24;
                cat.Hunger = cat.Thirst = cat.Rest = cat.Health = 100;
                cat.Skills.Clear();
                cat.Preferences.Clear();
                cat.Boosted = false;
                cat.ControlledBy = "fixture-held";
                cat.ControlLeaseUntil = 10000000;
                cat.Position = new Int2(3, 0);
                cat.X = 3;
                cat.Z = 0;
            }
            var worker = village.Cats[1];
            worker.Name = "Food Miller";
            worker.ControlledBy = "";
            worker.ControlLeaseUntil = 0;
            var knownLogs = world.Tiles.First(t => t.Resource == "logs" && t.Amount > 0).Position;
            if (!village.Known.Contains(knownLogs)) village.Known.Add(knownLogs);

            var recipe = Catalog.Recipe("flour_to_food");
            Check(!Catalog.BuildingAvailable(village, "mill") && !Catalog.RecipeAvailable(village, recipe), "Food preflight fixture must begin without grandfathered Mill or Food recipe access");
            string[] studies = { "research_hut", "basic_tools", "foraging_lore", "sawmill", "masonry", "water_carriers", "irrigation", "milling", "grain_milling_sources", "grain_milling_preparation", "grain_milling_staples" };
            Check(village.Research.Count == 0 && studies.Sum(id => Catalog.Study(id).Cost) == 133.5, "Food preflight requires the exact finite legal study bill");
            village.ResearchPoints = 133.5;
            foreach (string study in studies)
                Act(world, village, new GameAction { Kind = "ResearchNode", NodeId = study });
            Check(village.ResearchPoints == 0 && village.Research.SequenceEqual(studies) && world.TimeSeconds == 0
                && Catalog.BuildingAvailable(village, "mill") && Catalog.RecipeAvailable(village, Catalog.Recipe("grain_to_flour")) && Catalog.RecipeAvailable(village, recipe), "Public purchases must unlock the Mill and both recipes without advancing time or adding unrelated studies");
            Check(World.RecipeFamily(recipe) == "grain_milling" && recipe.Inputs.Single().Resource == "flour" && recipe.Inputs.Single().Amount == 2
                && world.RecipeInput(village, recipe, 2) == 2 && recipe.Outputs.Single().Resource == "food" && recipe.Outputs.Single().Amount == 4, "Food preflight must use the real two-Flour/four-base-Food recipe");
            double output = world.RecipeOutput(village, recipe, recipe.Outputs.Single().Amount);
            Check(Math.Abs(output - 4.4) < 1e-9 && Math.Abs(output / 4 - 1.1) < 1e-9 && output <= world.CarryCapacity(village, "food"), "Legal Sources research must produce one physical 4.4-Food load");

            var mill = new Building { Id = world.Id("food-preflight-mill"), Kind = "mill", Position = new Int2(4, -4), Completed = true };
            village.Buildings.Add(mill);
            var flour = new Stockpile
            {
                Id = world.Id("food-preflight-flour"),
                Position = new Int2(3, -4),
                Width = 1,
                Depth = 1,
                Accepts = new List<string> { "flour" },
                Goods = new List<Stack> { new Stack("flour", 2) }
            };
            village.Stockpiles.Add(flour);
            var receiver = new Stockpile
            {
                Id = world.Id("food-preflight-receiver"),
                Position = new Int2(3, -3),
                Width = 1,
                Depth = 1,
                Accepts = new List<string> { "food", "stone" }
            };
            village.Stockpiles.Add(receiver);
            double capacity = world.ResourceCapacity(village, receiver, "food");
            receiver.Goods.Add(new Stack("stone", capacity - 4.2));
            Check(receiver.Capacity == 500 && capacity > receiver.Capacity && receiver.ResourceLimits.Count == 0
                && Math.Abs(capacity - receiver.Goods.Sum(s => s.Amount) - 4.2) < 1e-9, "Masonry-adjusted Food room must be exactly 4.2, without per-resource limits");
            Check(village.Stockpiles.Where(p => p.Kind == "storage" && (p.Accepts.Count == 0 || p.Accepts.Contains("food"))).SequenceEqual(new[] { receiver })
                && world.HasRoom(village, receiver, "food", 4) && !world.HasRoom(village, receiver, "food", output)
                && !world.HasRoom(village, provisions, "food", 4) && !world.HasRoom(village, flour, "food", 4), "Only the reachable receiver may accept Food, and it must fit base output but reject researched output");
            Check(world.Walkable(village, worker.Position) && world.Walkable(village, flour.Position) && world.Walkable(village, mill.Position) && world.Walkable(village, receiver.Position)
                && world.Path(worker.Position, flour.Position, village) != null && world.Path(flour.Position, mill.Position, village) != null
                && world.Path(worker.Position, mill.Position, village) != null && world.Path(mill.Position, receiver.Position, village) != null
                && world.Path(worker.Position, provisions.Position, village) != null && world.Path(provisions.Position, village.Center, village) != null, "Food preflight must have real worker, input, station, receiver and support routes");
            Check(mill.Slots.Count == 0 && village.Jobs.Count == 0 && world.Reservations.Count == 0
                && village.Cats.All(c => c.JobId == "" && c.BuildingId == "" && c.Cargo.Count == 0)
                && village.Buildings.All(b => b.Inputs.Count == 0 && b.Outputs.Count == 0), "Food preflight fixture must contain no injected work, cargo, staged inputs, outputs or claims");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));

            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = mill.Id, CatId = worker.Id });
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = mill.Id, CatId = worker.Id, Edit = "add", RecipeId = recipe.Id, Repeat = false });
            var slot = mill.Slots.Single();
            var order = slot.Queue.Single();
            Check(slot.CatId == worker.Id && mill.WorkerId == worker.Id && worker.BuildingId == mill.Id && slot.AutomatedBy == "" && !slot.Paused && !mill.Paused
                && order.RecipeId == recipe.Id && !order.Repeat && order.AutomatedBy == "", "Public actions must create the exact unpaused manual worker and one finite Food order");
            Check(world.TimeSeconds == 0 && world.Available(village, "flour") == 2 && World.Amount(flour.Goods, "flour") == 2 && world.Total(village, "flour") == 2
                && world.Total(village, "food") == 0 && world.Reservations.Count == 0 && village.Jobs.Count == 0 && worker.JobId == "" && slot.JobId == ""
                && world.StationCapacity(village, "mill") >= 2 && worker.Cargo.Count == 0 && mill.Inputs.Count == 0 && mill.Outputs.Count == 0
                && slot.Inputs.Count == 0 && slot.Outputs.Count == 0 && slot.Progress == 0, "Immediately before RED, exactly two unclaimed Flour must remain physically stored with no work or progress");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));

            // One planning boundary plus one actor quantum. No batch work is credited.
            for (int quantum = 0; quantum < 21; quantum++) world.Step(0.05);
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
            Check(world.TimeSeconds == 1.05 && slot.Queue.Count == 1 && ReferenceEquals(slot.Queue[0], order) && !order.Repeat && order.AutomatedBy == ""
                && slot.CatId == worker.Id && worker.BuildingId == mill.Id, "Insufficient output space must retain the same finite order and exact worker");
            Check(slot.BlockedReason == "output_storage_full" && mill.BlockedReason == "output_storage_full",
                "Researched 4.4-Food output must be blocked by the sole receiver's 4.2 room before a batch starts; actual slot blocker='" + slot.BlockedReason + "', production jobs=" + village.Jobs.Count(j => j.Kind == "production") + ", claims=" + world.Reservations.Count);
            Check(village.Jobs.Count == 0 && world.Reservations.Count == 0 && worker.JobId == "" && slot.JobId == "" && slot.Progress == 0,
                "Output preflight must refuse before creating a production job, input claim or work credit");
            Check(World.Amount(flour.Goods, "flour") == 2 && world.Total(village, "flour") == 2 && world.Available(village, "flour") == 2
                && world.Total(village, "food") == 0 && village.Cats.All(c => c.Cargo.Count == 0)
                && village.Buildings.All(b => b.Inputs.Count == 0 && b.Outputs.Count == 0 && b.Slots.All(s => s.Inputs.Count == 0 && s.Outputs.Count == 0))
                && World.Amount(receiver.Goods, "stone") == capacity - 4.2 && Math.Abs(world.ResourceCapacity(village, receiver, "food") - receiver.Goods.Sum(s => s.Amount) - 4.2) < 1e-9,
                "Blocked Food preflight must neither move nor consume finite Flour, create Food, stage goods or alter receiver room");

            bool Near(double actual, double expected) => Math.Abs(actual - expected) < 1e-8;
            bool At(Int2 position) => worker.Position.Equals(position) && Near(worker.X, position.X) && Near(worker.Z, position.Z);
            // Claims and Reserved are ownership/bills, never additional physical goods.
            double Physical(string resource) =>
                village.Stockpiles.Sum(p => World.Amount(p.Goods, resource))
                + village.Cats.Sum(c => World.Amount(c.Cargo, resource))
                + village.Jobs.Sum(j => World.Amount(j.Local, resource))
                + village.Buildings.Sum(b => World.Amount(b.Inputs, resource) + World.Amount(b.Outputs, resource)
                    + b.Slots.Sum(s => World.Amount(s.Inputs, resource) + World.Amount(s.Outputs, resource)));
            void ManualOwnership()
            {
                Check(mill.Slots.Count == 1 && ReferenceEquals(mill.Slots[0], slot) && slot.CatId == worker.Id
                    && mill.WorkerId == worker.Id && worker.BuildingId == mill.Id && slot.AutomatedBy == ""
                    && order.RecipeId == recipe.Id && !order.Repeat && order.AutomatedBy == "",
                    "Capacity recovery changed the exact manual worker, slot or finite order");
            }
            void Refused(bool paused)
            {
                ManualOwnership();
                Check(slot.Paused == paused && !mill.Paused && slot.Queue.Count == 1 && ReferenceEquals(slot.Queue[0], order)
                    && slot.BlockedReason == "output_storage_full" && mill.BlockedReason == "output_storage_full",
                    "Pause/resume must retain the same finite order and its capacity refusal");
                Check(village.Jobs.Count == 0 && world.Reservations.Count == 0 && worker.JobId == "" && slot.JobId == ""
                    && slot.Progress == 0 && Near(Physical("flour"), 2) && Near(Physical("food"), 0)
                    && World.Amount(flour.Goods, "flour") == 2 && village.Cats.All(c => c.Cargo.Count == 0)
                    && village.Buildings.All(b => b.Inputs.Count == 0 && b.Outputs.Count == 0
                        && b.Slots.All(s => s.Inputs.Count == 0 && s.Outputs.Count == 0))
                    && Near(World.Amount(receiver.Goods, "stone"), capacity - 4.2),
                    "Refused pause cycles moved finite goods, credited work, created a job or retained a claim");
            }
            Refused(false);
            for (int cycle = 0; cycle < 2; cycle++)
            {
                foreach (bool paused in new[] { true, false })
                {
                    Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = mill.Id, CatId = worker.Id, Edit = "set_paused", Enabled = paused });
                    Refused(paused);
                    for (int quantum = 0; quantum < 21; quantum++)
                    {
                        world.Step(0.05);
                        Refused(paused);
                    }
                }
            }

            // Explicit synthetic capacity intervention only. Production remains public/physical.
            double stoneBefore = World.Amount(receiver.Goods, "stone");
            World.Add(receiver.Goods, "stone", -0.3);
            double remainingStone = stoneBefore - 0.3;
            Check(Near(World.Amount(receiver.Goods, "stone"), remainingStone)
                && Near(world.ResourceCapacity(village, receiver, "food") - receiver.Goods.Sum(s => s.Amount), 4.5)
                && world.HasRoom(village, receiver, "food", output) && Near(Physical("flour"), 2) && Near(Physical("food"), 0),
                "Removing exactly 0.3 synthetic Stone must open 4.5 room without supplying recipe goods or work");
            Check(recipe.Seconds == 600 && world.RecipeDuration(village, recipe) == 600,
                "Recovery must require the real unboosted 600-work Food batch");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));

            Job batch = null;
            bool sawClaim = false, sawPickup = false, sawWork = false, sawFoodCargo = false, delivered = false;
            double creditedWork = 0;
            for (int quantum = 0; quantum < 24000 && !delivered && world.TimeSeconds + 0.05 <= 1200; quantum++)
            {
                double beforeTime = world.TimeSeconds;
                double beforeFlour = World.Amount(flour.Goods, "flour");
                double beforeProgress = batch?.Progress ?? 0;
                bool wasAtMill = At(mill.Position);
                world.Step(0.05);
                ManualOwnership();
                Check(Near(world.TimeSeconds - beforeTime, 0.05) && world.TimeSeconds <= 1200
                    && !slot.Paused && !mill.Paused && Near(World.Amount(receiver.Goods, "stone"), remainingStone),
                    "Recovery exceeded its quantum/time bound, changed pause state or moved the remaining Stone");
                var jobs = village.Jobs.Where(j => j.Kind == "production").ToArray();
                Check(village.Jobs.Count == jobs.Length && jobs.Length <= 1, "Recovery created unrelated work or replayed the finite batch");
                if (jobs.Length == 0)
                {
                    Check(batch == null && !sawClaim && !sawPickup && slot.Queue.Count == 1 && ReferenceEquals(slot.Queue[0], order)
                        && world.Reservations.Count == 0 && Near(Physical("flour"), 2) && Near(Physical("food"), 0),
                        "Waiting for the next planner lost the finite order or changed its goods");
                    continue;
                }
                batch ??= jobs[0];
                Check(ReferenceEquals(jobs[0], batch) && batch.CatId == worker.Id && batch.TargetId == mill.Id
                    && batch.Position.Equals(mill.Position) && batch.RecipeId == recipe.Id && batch.AutomatedBy == ""
                    && batch.RequiredWork == 600 && batch.BoostCount == 0 && batch.SpeedMultiplier == 1 && slot.JobId == batch.Id
                    && (batch.Completed ? worker.JobId == "" : worker.JobId == batch.Id),
                    "Recovery replaced, boosted or reassigned the single real production job");
                Check(world.Reservations.All(r => r.OwnerId == batch.Id && r.VillageId == village.Id
                    && r.PileId == flour.Id && r.Resource == "flour" && Near(r.Amount, 2)) && world.Reservations.Count <= 1,
                    "Recovery duplicated or redirected the exact Flour claim");
                if (world.Reservations.Count == 1)
                {
                    sawClaim = true;
                    Check(!sawPickup && World.Amount(flour.Goods, "flour") == 2 && worker.Cargo.Count == 0
                        && batch.Local.Count == 0 && batch.Progress == 0 && Near(world.Available(village, "flour"), 0),
                        "The input claim must reserve the two stored Flour before physical pickup");
                }
                double flourCargo = World.Amount(worker.Cargo, "flour");
                double foodCargo = World.Amount(worker.Cargo, "food");
                double foodStored = World.Amount(receiver.Goods, "food");
                if (beforeFlour > World.Amount(flour.Goods, "flour"))
                {
                    Check(sawClaim && !sawPickup && beforeFlour == 2 && World.Amount(flour.Goods, "flour") == 0
                        && Near(flourCargo, 2) && At(flour.Position) && world.Reservations.Count == 0
                        && batch.Local.Count == 0 && batch.Progress == 0,
                        "Flour must move once into real cargo only on physical arrival at its source");
                    sawPickup = true;
                }
                double deltaWork = batch.Progress - beforeProgress;
                Check(deltaWork >= 0 && deltaWork <= 0.05 * 20 + 1e-8, "Food work jumped or rewound instead of using bounded actor time");
                creditedWork += deltaWork;
                if (deltaWork > 0)
                {
                    Check(sawPickup && (At(mill.Position) || (batch.Phase == "output_delivery" && wasAtMill)),
                        "Food work was credited before Flour pickup or physical Mill arrival");
                    sawWork = true;
                }
                bool produced = batch.Phase == "output_delivery" || batch.Completed;
                Check(Near(Physical("flour"), produced ? 0 : 2) && Near(Physical("food"), produced ? output : 0),
                    "Finite ledger lost/duplicated Flour or Food; only the real batch may consume two Flour and produce 4.4 Food");
                if (!produced)
                {
                    Check(slot.Queue.Count == 1 && ReferenceEquals(slot.Queue[0], order) && !batch.Completed
                        && foodCargo == 0 && foodStored == 0 && batch.Progress < 600,
                        "Finite order left its queue or created Food before completing 600 work");
                    if (batch.Progress > 0)
                        Check(At(mill.Position) && Near(World.Amount(batch.Local, "flour"), 2) && flourCargo == 0,
                            "Working batch must hold all two Flour locally at the Mill");
                }
                else
                {
                    Check(sawWork && batch.Progress + 1e-9 >= 600 && Near(creditedWork, batch.Progress)
                        && slot.Queue.Count == 0 && mill.Queue.Count == 0 && world.Reservations.Count == 0
                        && flourCargo == 0 && World.Amount(batch.Local, "flour") == 0,
                        "Food output preceded real work/input consumption or replayed its finite order");
                    if (foodCargo > 0 && !sawFoodCargo)
                    {
                        Check(!batch.Completed && wasAtMill && worker.Position.Equals(mill.Position)
                            && Near(foodCargo, 4.4) && foodStored == 0 && batch.Local.Count == 0
                            && batch.DeliveryPileId == receiver.Id && !At(receiver.Position),
                            "The completed 4.4 Food must leave the Mill as real cargo before receiver arrival");
                        sawFoodCargo = true;
                    }
                }
                Check(village.Cats.Where(c => c.Id != worker.Id).All(c => c.Cargo.Count == 0)
                    && village.Buildings.All(b => b.Inputs.Count == 0 && b.Outputs.Count == 0
                        && b.Slots.All(s => s.Inputs.Count == 0 && s.Outputs.Count == 0)),
                    "Recovery hid recipe goods in another cat or station compartment");
                if (foodStored > 0)
                {
                    // Stop in this quantum, before any later minute can spoil delivered Food.
                    Check(sawFoodCargo && At(receiver.Position) && batch.Completed && Near(foodStored, 4.4)
                        && (long)(beforeTime / 60) == (long)(world.TimeSeconds / 60)
                        && worker.Cargo.Count == 0 && batch.Local.Count == 0 && batch.ItemIds.Count == 0
                        && batch.SuspendedCargoPileId == "" && worker.ResumeJobId == "",
                        "Delivery must finish physically once, with all 4.4 Food intact and no cargo/input/needs residue");
                    delivered = true;
                }
                else
                    Check(!batch.Completed, "The job completed before physical Food delivery");
            }
            Check(delivered && sawClaim && sawPickup && sawWork && sawFoodCargo && batch != null
                && batch.Completed && village.Jobs.Count == 1 && ReferenceEquals(village.Jobs[0], batch)
                && world.TimeSeconds <= 1200 && Near(creditedWork, batch.Progress) && creditedWork + 1e-9 >= 600
                && Near(Physical("flour"), 0) && Near(Physical("food"), 4.4) && world.Total(village, "flour") == 0
                && Near(world.Total(village, "food"), 4.4) && Near(Physical("stone"), remainingStone)
                && Near(capacity - receiver.Goods.Sum(s => s.Amount), 0.1)
                && world.Reservations.Count == 0 && worker.JobId == "" && worker.Cargo.Count == 0
                && batch.Local.Count == 0 && slot.Queue.Count == 0 && mill.Queue.Count == 0,
                "Bounded recovery must consume exactly two Flour, produce/deliver exactly 4.4 Food and exhaust only this one manual batch");
            ManualOwnership();
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        // Used only by the two multiple-load cases. All setup precedes time and public work actions.
        static World MillFoodMultiloadFixture(out Village village, out Building mill, out Cat worker, out Stockpile grain, out Stockpile[] receivers)
        {
            var world = World.Create(41);
            var v = world.Villages[0];
            v.Buildings.RemoveAll(building => building.Kind != "den" && building.Kind != "shrine");
            v.Stockpiles.RemoveRange(1, v.Stockpiles.Count - 1);
            var provisions = v.Stockpiles[0];
            provisions.Position = new Int2(3, 2);
            provisions.Width = provisions.Depth = 1;
            provisions.Capacity = 10000;
            provisions.Accepts = new List<string> { "preserves", "water", "fibre" };
            provisions.Goods = new List<Stack> { new Stack("preserves", 1000), new Stack("water", 1000), new Stack("fibre", 10) };
            v.LastLeaderResearch = 0;
            foreach (var cat in v.Cats)
            {
                cat.AgeHours = 24;
                cat.Hunger = cat.Thirst = cat.Rest = cat.Health = 100;
                cat.Skills.Clear();
                cat.Preferences.Clear();
                cat.Boosted = false;
                cat.ControlledBy = "fixture-held";
                cat.ControlLeaseUntil = 10000000;
                cat.Position = new Int2(3, 0);
                cat.X = 3;
                cat.Z = 0;
            }
            var c = v.Cats[1];
            c.Name = "Multiple Load Miller";
            c.ControlledBy = "";
            c.ControlLeaseUntil = 0;
            var knownLogs = world.Tiles.First(t => t.Resource == "logs" && t.Amount > 0).Position;
            if (!v.Known.Contains(knownLogs)) v.Known.Add(knownLogs);
            var recipe = Catalog.Recipe("masterwork_flour");
            Check(v.Research.Count == 0 && !Catalog.BuildingAvailable(v, "mill") && !Catalog.RecipeAvailable(v, recipe)
                && world.CarryCapacity(v, "flour") == 8, "Multiple-load fixture must start with locked Masterwork and default eight-unit carrying");
            string[] studies = { "research_hut", "basic_tools", "foraging_lore", "sawmill", "masonry", "water_carriers",
                "irrigation", "milling", "grain_milling_sources", "grain_milling_preparation", "grain_milling_staples",
                "grain_milling_preservation", "grain_milling_quality", "grain_milling_bulk", "grain_milling_specialty", "grain_milling_masterwork" };
            Check(studies.Sum(id => Catalog.Study(id).Cost) == 306, "Masterwork must use its exact legal 306-point study chain");
            v.ResearchPoints = 306;
            foreach (string study in studies)
                Act(world, v, new GameAction { Kind = "ResearchNode", NodeId = study });
            Check(v.ResearchPoints == 0 && v.Research.SequenceEqual(studies) && world.TimeSeconds == 0
                && Catalog.BuildingAvailable(v, "mill") && Catalog.RecipeAvailable(v, recipe), "Public purchases must unlock Masterwork without unrelated research or elapsed time");
            double output = world.RecipeOutput(v, recipe, recipe.Outputs.Single().Amount);
            Check(World.RecipeFamily(recipe) == "grain_milling" && recipe.Building == "mill" && recipe.Labor == "mill" && recipe.ItemKind == ""
                && recipe.Inputs.Single().Resource == "grain" && recipe.Inputs.Single().Amount == 5 && world.RecipeInput(v, recipe, 5) == 5
                && recipe.Outputs.Single().Resource == "flour" && recipe.Outputs.Single().Amount == 8 && Math.Abs(output - 8.8) < 1e-9
                && world.CarryCapacity(v, "flour") == 8 && output > world.CarryCapacity(v, "flour")
                && recipe.Seconds == 600 && world.RecipeDuration(v, recipe) == 540, "Sources must yield 8.8 Flour from five Grain and Bulk must require 540 real work with unchanged eight-unit carrying");
            Check(world.ResourceCapacityBonus(v, "flour") == 50 && Catalog.Effect(v, "spoilageResistance", 1) == 1
                && !v.Buildings.Any(b => b.Kind == "food_storage"), "Preservation adds fifty Flour headroom, not spoilage resistance or a food-storage service");
            var b = new Building { Id = world.Id("multiload-mill"), Kind = "mill", Position = new Int2(4, -4), Completed = true };
            v.Buildings.Add(b);
            var source = new Stockpile
            {
                Id = world.Id("multiload-grain"),
                Position = new Int2(3, -4),
                Width = 1,
                Depth = 1,
                Accepts = new List<string> { "grain" },
                Goods = new List<Stack> { new Stack("grain", 5) }
            };
            v.Stockpiles.Add(source);
            var stores = new[] {
                new Stockpile { Id = world.Id("multiload-first"), Position = new Int2(3, -3), Width = 1, Depth = 1, Accepts = new List<string> { "flour", "stone" } },
                new Stockpile { Id = world.Id("multiload-second"), Position = new Int2(3, -1), Width = 1, Depth = 1, Accepts = new List<string> { "flour", "stone" } }
            };
            foreach (var store in stores)
            {
                v.Stockpiles.Add(store);
                double capacity = world.ResourceCapacity(v, store, "flour");
                store.Goods.Add(new Stack("stone", capacity - 8.4));
                Check(store.Capacity == 500 && store.ResourceLimits.Count == 0 && Math.Abs(capacity - 675) < 1e-9
                    && Math.Abs(capacity - store.Goods.Sum(s => s.Amount) - 8.4) < 1e-9
                    && world.HasRoom(v, store, "flour", 8) && !world.HasRoom(v, store, "flour", output),
                    "Each Masonry/Preservation-adjusted receiver must have 8.4 effective room, fitting eight but refusing 8.8");
            }
            Check(Math.Abs(stores.Sum(p => world.ResourceCapacity(v, p, "flour") - p.Goods.Sum(s => s.Amount)) - 16.8) < 1e-9
                && v.Stockpiles.Where(p => p.Kind == "storage" && (p.Accepts.Count == 0 || p.Accepts.Contains("flour"))).SequenceEqual(stores)
                && !world.HasRoom(v, provisions, "flour", 0.8) && !world.HasRoom(v, source, "flour", 0.8)
                && Int2.Distance(b.Position, stores[0].Position) < Int2.Distance(b.Position, stores[1].Position),
                "Only the two ordered receivers may accept Flour, with 16.8 total room and a strictly nearer first receiver");
            Check(world.Walkable(v, c.Position) && world.Walkable(v, source.Position) && world.Walkable(v, b.Position)
                && world.Path(c.Position, source.Position, v) != null && world.Path(source.Position, b.Position, v) != null
                && world.Path(c.Position, b.Position, v) != null && world.Path(c.Position, provisions.Position, v) != null
                && world.Path(provisions.Position, v.Center, v) != null
                && stores.All(p => world.Walkable(v, p.Position) && world.Path(b.Position, p.Position, v) != null && world.Path(p.Position, b.Position, v) != null),
                "Multiple-load fixture needs physical input, workshop, support and both return/delivery routes");
            Check(v.Jobs.Count == 0 && world.Reservations.Count == 0 && b.Slots.Count == 0
                && v.Cats.All(cat => cat.JobId == "" && cat.BuildingId == "" && cat.Cargo.Count == 0)
                && v.Buildings.All(station => station.Inputs.Count == 0 && station.Outputs.Count == 0)
                && world.Total(v, "grain") == 5 && world.Total(v, "flour") == 0 && world.StationCapacity(v, "mill") >= 5,
                "Fixture must inject no work, claims, cargo, station goods or extra recipe inputs");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
            Act(world, v, new GameAction { Kind = "AssignWorker", BuildingId = b.Id, CatId = c.Id });
            Act(world, v, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = b.Id, CatId = c.Id, Edit = "add", RecipeId = recipe.Id, Repeat = false });
            var slot = b.Slots.Single();
            var order = slot.Queue.Single();
            Check(slot.CatId == c.Id && b.WorkerId == c.Id && c.BuildingId == b.Id && slot.AutomatedBy == "" && !slot.Paused && !b.Paused
                && order.RecipeId == recipe.Id && !order.Repeat && order.AutomatedBy == ""
                && world.TimeSeconds == 0 && v.Jobs.Count == 0 && world.Reservations.Count == 0 && c.JobId == "" && slot.JobId == "" && slot.Progress == 0
                && slot.Inputs.Count == 0 && slot.Outputs.Count == 0 && c.Cargo.Count == 0 && world.Available(v, "grain") == 5,
                "Only public actions may assign the exact worker and create one finite manual Masterwork order without advancing or funding work");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
            village = v; mill = b; worker = c; grain = source; receivers = stores;
            return world;
        }

        // Freeze this focused method after root records the second RED.
        static void MillFoodMultiloadPreflight()
        {
            var world = MillFoodMultiloadFixture(out var village, out var mill, out var worker, out var grain, out var receivers);
            var slot = mill.Slots.Single();
            var order = slot.Queue.Single();
            world.Step(1.05);
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
            Check(Math.Abs(world.TimeSeconds - 1.05) < 1e-9 && slot.Queue.Count == 1 && ReferenceEquals(slot.Queue[0], order)
                && !order.Repeat && order.AutomatedBy == "" && slot.CatId == worker.Id && worker.BuildingId == mill.Id,
                "Deliverable multiple-load preflight must retain the same finite manual order and worker");
            var jobs = village.Jobs.Where(j => j.Kind == "production").ToArray();
            Check(jobs.Length == 1 && village.Jobs.Count == 1,
                "Deliverable 8.8-Flour batch must start with two 8.4-room receivers and eight-unit carrying; actual slot blocker='" + slot.BlockedReason
                + "', production jobs=" + jobs.Length + ", claims=" + world.Reservations.Count);
            var batch = jobs.Single();
            Check(!batch.Completed && batch.CatId == worker.Id && batch.TargetId == mill.Id && batch.Position.Equals(mill.Position)
                && batch.RecipeId == "masterwork_flour" && batch.AutomatedBy == "" && batch.Phase == "fetch"
                && batch.RequiredWork == 540 && batch.Progress == 0 && batch.BoostCount == 0 && batch.SpeedMultiplier == 1
                && slot.JobId == batch.Id && worker.JobId == batch.Id && slot.BlockedReason == "" && mill.BlockedReason == ""
                && batch.Reserved.Count == 1 && batch.Reserved[0].Resource == "grain" && batch.Reserved[0].Amount == 5,
                "Preflight must create the exact unboosted real production job with the five-Grain bill and no work credit");
            Check(world.Reservations.Count == 1 && world.Reservations.All(r => r.OwnerId == batch.Id && r.VillageId == village.Id
                && r.PileId == grain.Id && r.Resource == "grain" && r.Amount == 5)
                && world.Available(village, "grain") == 0 && World.Amount(grain.Goods, "grain") == 5
                && world.Total(village, "grain") == 5 && world.Total(village, "flour") == 0 && batch.Local.Count == 0
                && village.Cats.All(c => c.Cargo.Count == 0) && slot.Progress == 0
                && village.Buildings.All(b => b.Inputs.Count == 0 && b.Outputs.Count == 0 && b.Slots.All(s => s.Inputs.Count == 0 && s.Outputs.Count == 0))
                && receivers.All(p => Math.Abs(world.ResourceCapacity(village, p, "flour") - p.Goods.Sum(s => s.Amount) - 8.4) < 1e-9),
                "The real job must claim exactly five still-stored Grain before pickup, leaving both receivers and all physical compartments unchanged");
        }

        static void MillFoodMultiloadDelivery()
        {
            var world = MillFoodMultiloadFixture(out var village, out var mill, out var worker, out var grain, out var receivers);
            var slot = mill.Slots.Single();
            var order = slot.Queue.Single();
            var stones = receivers.Select(p => World.Amount(p.Goods, "stone")).ToArray();
            bool Near(double actual, double expected) => Math.Abs(actual - expected) < 1e-8;
            bool At(Int2 p) => worker.Position.Equals(p) && Near(worker.X, p.X) && Near(worker.Z, p.Z);
            // Reserved bills and claims are metadata; slot mirrors are required empty below.
            double Physical(string resource) => village.Stockpiles.Sum(p => World.Amount(p.Goods, resource))
                + village.Cats.Sum(c => World.Amount(c.Cargo, resource)) + village.Jobs.Sum(j => World.Amount(j.Local, resource))
                + village.Buildings.Sum(b => World.Amount(b.Inputs, resource) + World.Amount(b.Outputs, resource));
            Job batch = null;
            bool sawClaim = false, sawPickup = false, sawWork = false, sawOutput = false;
            bool sawFirstArrival = false, sawReturnTravel = false, sawSecondPickup = false, delivered = false;
            double creditedWork = 0, grainLoss = 0, flourLoss = 0;
            var receiverLosses = new double[2];
            for (int quantum = 0; quantum < 24000 && !delivered; quantum++)
            {
                double beforeTime = world.TimeSeconds;
                double beforeGrain = World.Amount(grain.Goods, "grain");
                double beforeProgress = batch?.Progress ?? 0;
                double beforeCargo = World.Amount(worker.Cargo, "flour");
                double beforeLocal = batch == null ? 0 : World.Amount(batch.Local, "flour");
                var beforeStored = receivers.Select(p => World.Amount(p.Goods, "flour")).ToArray();
                bool wasAtMill = At(mill.Position);
                world.Step(0.05);
                bool minute = (long)(beforeTime / 60) != (long)(world.TimeSeconds / 60);
                var receiverLoss = new double[2];
                // Spoilage runs after actor transfers. Invert that known rate on unreserved
                // post-spoilage stock; no loss is inferred from a conservation discrepancy.
                double MinuteLoss(Stockpile pile, string resource)
                {
                    if (!minute) return 0;
                    double held = world.Reservations.Where(r => r.PileId == pile.Id && r.Resource == resource).Sum(r => r.Amount);
                    return Math.Max(0, World.Amount(pile.Goods, resource) - held) * 0.0002 / (1 - 0.0002);
                }
                grainLoss += village.Stockpiles.Sum(p => MinuteLoss(p, "grain"));
                flourLoss += village.Stockpiles.Sum(p => MinuteLoss(p, "flour"));
                for (int i = 0; i < receivers.Length; i++)
                {
                    receiverLoss[i] = MinuteLoss(receivers[i], "flour");
                    receiverLosses[i] += receiverLoss[i];
                }
                Check(Near(world.TimeSeconds - beforeTime, 0.05) && world.TimeSeconds <= 1200
                    && Near(grainLoss, 0) && receivers.Select((p, i) => Near(World.Amount(p.Goods, "stone"), stones[i])).All(same => same)
                    && Near(Physical("stone"), stones.Sum()), "Multiple-load observation exceeded its bound, spoiled finite input or moved inert Stone");
                Check(mill.Slots.Count == 1 && ReferenceEquals(mill.Slots[0], slot) && mill.WorkerId == worker.Id
                    && slot.CatId == worker.Id && worker.BuildingId == mill.Id && slot.AutomatedBy == "" && !slot.Paused && !mill.Paused
                    && order.RecipeId == "masterwork_flour" && !order.Repeat && order.AutomatedBy == ""
                    && village.Cats.Where(c => c.Id != worker.Id).All(c => c.Cargo.Count == 0 && c.JobId == "")
                    && village.Buildings.All(b => b.Inputs.Count == 0 && b.Outputs.Count == 0 && b.Slots.All(s => s.Inputs.Count == 0 && s.Outputs.Count == 0)),
                    "Multiple-load flow changed manual ownership or hid goods/work in another cat or a station compartment");
                var jobs = village.Jobs.Where(j => j.Kind == "production").ToArray();
                Check(village.Jobs.Count == jobs.Length && jobs.Length <= 1, "Finite Masterwork created unrelated work or an extra production start");
                if (jobs.Length == 0)
                {
                    Check(world.TimeSeconds < 1 && batch == null && slot.Queue.Count == 1 && ReferenceEquals(slot.Queue[0], order)
                        && world.Reservations.Count == 0 && worker.Cargo.Count == 0 && Near(Physical("grain"), 5) && Near(Physical("flour"), 0),
                        "Deliverable multiple-load order must start at the first planner without losing or duplicating its finite inputs");
                    continue;
                }
                batch ??= jobs[0];
                Check(ReferenceEquals(jobs[0], batch) && batch.CatId == worker.Id && batch.TargetId == mill.Id
                    && batch.Position.Equals(mill.Position) && batch.RecipeId == "masterwork_flour" && batch.AutomatedBy == ""
                    && batch.RequiredWork == 540 && batch.BoostCount == 0 && batch.SpeedMultiplier == 1 && slot.JobId == batch.Id
                    && (batch.Completed ? worker.JobId == "" : worker.JobId == batch.Id)
                    && batch.Reserved.Count == 1 && batch.Reserved[0].Resource == "grain" && Near(batch.Reserved[0].Amount, 5),
                    "The single job, exact owner, finite bill or real 540-work requirement changed");
                Check(world.Reservations.Count <= 1 && world.Reservations.All(r => r.OwnerId == batch.Id && r.VillageId == village.Id
                    && r.PileId == grain.Id && r.Resource == "grain" && Near(r.Amount, 5)), "Masterwork duplicated or redirected its five-Grain claim");
                if (world.Reservations.Count == 1)
                {
                    Check(!sawPickup && Near(World.Amount(grain.Goods, "grain"), 5) && worker.Cargo.Count == 0
                        && batch.Local.Count == 0 && batch.Progress == 0 && Near(world.Available(village, "grain"), 0),
                        "Claimed Grain must remain physically in its source until pickup");
                    sawClaim = true;
                }
                if (beforeGrain > World.Amount(grain.Goods, "grain") + 1e-9)
                {
                    Check(sawClaim && !sawPickup && Near(beforeGrain, 5) && Near(World.Amount(grain.Goods, "grain"), 0)
                        && Near(World.Amount(worker.Cargo, "grain"), 5) && At(grain.Position) && world.Reservations.Count == 0
                        && batch.Local.Count == 0 && batch.Progress == 0, "All five Grain must enter real cargo once, at actual source arrival");
                    sawPickup = true;
                }
                double deltaWork = batch.Progress - beforeProgress;
                Check(deltaWork >= 0 && deltaWork <= 0.05 * 20 + 1e-8, "Masterwork progress jumped or rewound outside actor time");
                creditedWork += deltaWork;
                bool produced = batch.Phase == "output_delivery" || batch.Completed;
                if (deltaWork > 0)
                {
                    Check(sawPickup && (At(mill.Position) || produced && wasAtMill), "Positive real work requires physical Grain pickup and Mill arrival");
                    sawWork = true;
                }
                Check(Near(Physical("grain"), produced ? 0 : 5) && Near(Physical("flour") + flourLoss, produced ? 8.8 : 0)
                    && Near(Physical("grain") + grainLoss + (Physical("flour") + flourLoss) * 5 / 8.8, 5),
                    "Per-quantum physical Grain-to-Flour conservation failed after independent minute spoilage accounting");
                if (!produced)
                {
                    Check(!batch.Completed && slot.Queue.Count == 1 && ReferenceEquals(slot.Queue[0], order)
                        && batch.Progress < 540 && Near(Physical("flour"), 0), "The finite order left its queue or created Flour before real completion");
                    if (batch.Progress > 0)
                        Check(At(mill.Position) && Near(World.Amount(batch.Local, "grain"), 5) && worker.Cargo.Count == 0,
                            "Working Mill must physically own all five Grain locally");
                }
                else
                {
                    Check(sawWork && batch.Progress + 1e-9 >= 540 && Near(creditedWork, batch.Progress)
                        && slot.Queue.Count == 0 && mill.Queue.Count == 0 && world.Reservations.Count == 0
                        && Near(World.Amount(worker.Cargo, "grain"), 0) && Near(World.Amount(batch.Local, "grain"), 0),
                        "Output preceded real work/input consumption or retained a replayable finite order/claim");
                    if (!sawOutput)
                    {
                        Check(wasAtMill && At(mill.Position) && !batch.Completed && Near(World.Amount(worker.Cargo, "flour"), 8)
                            && Near(World.Amount(batch.Local, "flour"), 0.8) && Near(beforeStored.Sum(), 0)
                            && Near(flourLoss, 0) && batch.DeliveryPileId == receivers[0].Id,
                            "Creation must produce exactly 8.8 Flour, take the first eight as cargo and retain 0.8 owned at the Mill");
                        sawOutput = true;
                    }
                }
                double firstDeposit = World.Amount(receivers[0].Goods, "flour") + receiverLoss[0] - beforeStored[0];
                double secondDeposit = World.Amount(receivers[1].Goods, "flour") + receiverLoss[1] - beforeStored[1];
                if (firstDeposit > 1e-9)
                {
                    Check(sawOutput && !sawFirstArrival && !sawSecondPickup && Near(firstDeposit, 8) && Near(secondDeposit, 0)
                        && At(receivers[0].Position) && !batch.Completed && worker.Cargo.Count == 0 && Near(World.Amount(batch.Local, "flour"), 0.8),
                        "The first eight-unit load must physically arrive in the nearer store while 0.8 remains station-owned Local");
                    Check(!world.HasRoom(village, receivers[0], "flour", 0.8) && world.HasRoom(village, receivers[1], "flour", 0.8),
                        "The remainder must require the other receiver after the first arrival");
                    sawFirstArrival = true;
                }
                if (sawFirstArrival && !sawSecondPickup && !At(receivers[0].Position) && !At(mill.Position) && worker.Cargo.Count == 0)
                {
                    Check(Near(World.Amount(batch.Local, "flour"), 0.8) && !batch.Completed,
                        "The second trip must return empty toward the Mill without remotely taking the remainder");
                    sawReturnTravel = true;
                }
                if (sawFirstArrival && beforeCargo < 1e-9 && World.Amount(worker.Cargo, "flour") > 1e-9)
                {
                    Check(sawReturnTravel && !sawSecondPickup && At(mill.Position) && Near(beforeLocal, 0.8)
                        && Near(World.Amount(worker.Cargo, "flour"), 0.8) && batch.Local.Count == 0 && !batch.Completed
                        && batch.DeliveryPileId == receivers[1].Id, "The remaining 0.8 must be picked up only after physically returning to the Mill");
                    sawSecondPickup = true;
                }
                if (secondDeposit > 1e-9)
                {
                    Check(sawSecondPickup && !delivered && Near(secondDeposit, 0.8) && Near(firstDeposit, 0)
                        && At(receivers[1].Position) && batch.Completed && worker.Cargo.Count == 0 && batch.Local.Count == 0,
                        "The second 0.8-unit load must physically arrive in the other store and complete the same job");
                    delivered = true; // Stop this quantum; no post-completion simulation or pruning.
                }
                Check(!batch.Completed || delivered, "Masterwork completed before both real physical deliveries");
            }
            Check(delivered && sawClaim && sawPickup && sawWork && sawOutput && sawFirstArrival && sawReturnTravel && sawSecondPickup
                && batch != null && batch.Completed && village.Jobs.Count == 1 && ReferenceEquals(village.Jobs[0], batch)
                && world.TimeSeconds <= 1200 && Near(grainLoss, 0) && Near(Physical("grain"), 0)
                && Near(Physical("flour") + flourLoss, 8.8) && Near(World.Amount(receivers[0].Goods, "flour") + receiverLosses[0], 8)
                && Near(World.Amount(receivers[1].Goods, "flour") + receiverLosses[1], 0.8) && Near(creditedWork, batch.Progress) && creditedWork + 1e-9 >= 540
                && Near(Physical("stone"), stones.Sum()) && world.Reservations.Count == 0 && worker.JobId == ""
                && worker.ResumeJobId == "" && batch.SuspendedCargoPileId == "" && batch.ItemIds.Count == 0
                && worker.Cargo.Count == 0 && batch.Local.Count == 0 && slot.Queue.Count == 0 && mill.Queue.Count == 0,
                "Bounded delivery must finish one finite five-Grain/8.8-Flour batch with both observed trips and no replay, claim or cargo residue");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }

        static void FiniteMaterialChain()
        {
            var world = SelectReachableRecipe(true, out var village, out var station, out var worker, out var blocked, out var threadJob);
            for (int second = 0; second < 900 && !threadJob.Completed; second++) world.Step(1);
            Check(threadJob.Completed && world.Total(village, "fibre") == 0 && world.Total(village, "thread") == 10, "The chosen automatic recipe did not physically deliver five Thread from its five finite Fibre");
            Job clothJob = null;
            for (int second = 0; second < 900 && (clothJob == null || !clothJob.Completed); second++)
            {
                world.Step(1);
                clothJob ??= village.Jobs.FirstOrDefault(j => j.CatId == worker.Id && j.RecipeId == "fibre_to_cloth");
            }
            Check(clothJob != null && clothJob.Completed && world.Total(village, "cloth") == 1 && world.Total(village, "thread") == 5, "The physically delivered Thread did not fund and deliver exactly one Cloth");
            Check(World.Amount(blocked.Goods, "thread") == 5 && world.Reservations.Count == 0 && worker.Cargo.Count == 0 && station.Inputs.Count == 0 && !village.Jobs.Any(j => !j.Completed && j.Kind == "production"), "The finite automatic chain consumed inaccessible stock or retained duplicate goods, claims or work");
            Check(world.Validate().Count == 0, string.Join("; ", world.Validate()));
        }
    }
}
