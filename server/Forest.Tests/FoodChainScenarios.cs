using IdleCatForest.Authority;
using IdleCatForest.Simulation;
using SimStack = IdleCatForest.Simulation.Stack;

static class FoodChainScenarios
{
    public static void Run(Action<string, Action> test, Action<bool, string> check)
    {
        test("finite legal Grain Flour Food chain retains physical authority through durable restarts", () => RunChain(check));
    }

    static void RunChain(Action<bool, string> check)
    {
        var world = World.Create(41);
        var village = world.Villages.Single();
        string villageId = village.Id;
        var context = new PlayerContext { PlayerId = "synthetic-food-chain", VillageId = villageId };
        void Near(double actual, double expected, string message) => check(Math.Abs(actual - expected) < 1e-8, $"{message}: expected {expected:R}, got {actual:R}");
        void Act(GameAction action)
        {
            var result = world.Apply(context, action);
            check(result.Success, action.Kind + " rejected: " + result.Error);
        }

        // Authored support only. No work, crop credit, cargo or reservations are seeded.
        // Keep the founding shrine, permanent beds and boundary/gates.
        village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
        village.Stockpiles.Clear();
        village.LastLeaderResearch = 0;
        foreach (var cat in village.Cats)
        {
            cat.ControlledBy = "fixture-held";
            cat.ControlLeaseUntil = 10000000;
            cat.Skills.Clear();
            cat.Preferences.Clear();
            cat.Boosted = false;
            cat.Hunger = cat.Thirst = cat.Rest = cat.Health = 100;
            cat.Position = new Int2(3, 0);
            cat.X = 3;
            cat.Z = 0;
            cat.Path.Clear();
        }
        string carrierId = village.Cats[2].Id, workerId = village.Cats[3].Id;
        village.Cats[2].ControlledBy = village.Cats[3].ControlledBy = "";
        village.Cats[2].Position = new Int2(10, 0);
        village.Cats[2].X = 10;
        // A short unclaimed exterior approach connects to the unchanged east gate.
        for (int x = 10; x <= 12; x++)
        {
            var tile = world.TileAt(new Int2(x, 0));
            check(tile.ClaimId == "", "Exterior support must not replace an owned tile");
            World.ClearSurface(tile, 0);
            tile.Wall = tile.Mountain = tile.Water = false;
            if (!village.Known.Contains(tile.Position)) village.Known.Add(tile.Position);
        }
        var logs = world.Tiles.First(t => t.Resource == "logs" && t.Amount > 0).Position;
        if (!village.Known.Contains(logs)) village.Known.Add(logs);

        var provisions = new Stockpile
        {
            Id = world.Id("food-chain-provisions"),
            Position = new Int2(3, 2),
            Width = 1,
            Depth = 1,
            Capacity = 10000,
            Accepts = new List<string> { "preserves", "water", "fibre" },
            Goods = new List<SimStack> { new SimStack("preserves", 1000), new SimStack("water", 1000), new SimStack("fibre", 10) }
        };
        var spill = new Stockpile
        {
            Id = world.Id("food-chain-spill"),
            Kind = "spill",
            Position = new Int2(12, 0),
            Width = 1,
            Depth = 1,
            Goods = new List<SimStack> { new SimStack("grain", 4) }
        };
        var ingredients = new Stockpile
        {
            Id = world.Id("food-chain-ingredients"),
            Position = new Int2(3, -4),
            Width = 1,
            Depth = 1,
            Accepts = new List<string> { "grain", "flour" }
        };
        var receiver = new Stockpile
        {
            Id = world.Id("food-chain-food"),
            Position = new Int2(3, -3),
            Width = 1,
            Depth = 1,
            Accepts = new List<string> { "food" }
        };
        village.Stockpiles.AddRange(new[] { provisions, spill, ingredients, receiver });
        string spillId = spill.Id, ingredientsId = ingredients.Id, receiverId = receiver.Id;
        string[] studies =
        {
            "research_hut", "basic_tools", "foraging_lore", "sawmill", "masonry", "water_carriers",
            "irrigation", "milling", "grain_milling_sources", "grain_milling_preparation", "grain_milling_staples"
        };
        Near(studies.Sum(id => Catalog.Study(id).Cost), 133.5, "Legal research cost changed");
        village.ResearchPoints = 133.5;
        foreach (string id in studies) Act(new GameAction { Kind = "ResearchNode", NodeId = id });
        Near(village.ResearchPoints, 0, "Research did not spend the exact finite budget");
        check(village.Research.Count == studies.Length && studies.All(village.Research.Contains), "Fixture gained unpurchased studies");
        var milling = Catalog.Recipe("grain_to_flour");
        var cooking = Catalog.Recipe("flour_to_food");
        check(Catalog.BuildingAvailable(village, "mill") && Catalog.RecipeAvailable(village, milling) && Catalog.RecipeAvailable(village, cooking), "Public studies did not unlock both Mill recipes");
        Near(world.RecipeInput(village, milling, milling.Inputs.Single().Amount), 4, "Milling bill");
        Near(world.RecipeInput(village, cooking, cooking.Inputs.Single().Amount), 2, "Food bill");
        Near(world.RecipeOutput(village, milling, milling.Outputs.Single().Amount), 2.2, "Researched Flour yield");
        Near(world.RecipeOutput(village, cooking, cooking.Outputs.Single().Amount), 4.4, "Researched Food yield");
        Near(world.RecipeDuration(village, milling), 600, "Milling work");
        Near(world.RecipeDuration(village, cooking), 600, "Food work");
        village.Buildings.Add(new Building { Id = world.Id("food-chain-workshop"), Kind = "workshop", Position = new Int2(4, 4), Completed = true });
        village.Buildings.Add(new Building { Id = world.Id("food-chain-field"), Kind = "field", Position = new Int2(6, 4), Completed = true });
        var mill = new Building { Id = world.Id("food-chain-mill"), Kind = "mill", Position = new Int2(4, -4), Completed = true };
        village.Buildings.Add(mill);
        string millId = mill.Id;
        Act(new GameAction { Kind = "AssignOfficer", Role = "steward", CatId = village.Cats[0].Id });
        Act(new GameAction { Kind = "AssignOfficer", Role = "farmer", CatId = village.Cats[1].Id });
        Act(new GameAction { Kind = "AssignWorker", BuildingId = millId, CatId = workerId });
        GameAction Edit(string edit) => new GameAction { Kind = "EditProductionWorkSlot", BuildingId = millId, CatId = workerId, Edit = edit };
        var pause = Edit("pause"); pause.Enabled = true; Act(pause);
        foreach (string recipe in new[] { milling.Id, cooking.Id })
        {
            var add = Edit("add"); add.RecipeId = recipe; add.Repeat = false; Act(add);
        }

        // All accessors resolve stable identities after every full-world load.
        Cat Carrier() => village.Cats.Single(c => c.Id == carrierId);
        Cat Worker() => village.Cats.Single(c => c.Id == workerId);
        Building Mill() => village.Buildings.Single(b => b.Id == millId);
        WorkSlot Slot() => Mill().Slots.Single(s => s.CatId == workerId);
        Stockpile Pile(string id) => village.Stockpiles.Single(p => p.Id == id);
        Job Order(string id) => village.Jobs.Single(j => j.Id == id);
        double Physical(string resource) => village.Stockpiles.Sum(p => World.Amount(p.Goods, resource))
            + village.Cats.Sum(c => World.Amount(c.Cargo, resource))
            + village.Jobs.Sum(j => World.Amount(j.Local, resource))
            + village.Buildings.Sum(b => World.Amount(b.Inputs, resource) + World.Amount(b.Outputs, resource));
        check(world.TimeSeconds == 0 && village.Jobs.Count == 0 && world.Reservations.Count == 0 && village.Farms.Count == 0
            && village.Cats.All(c => c.Cargo.Count == 0 && c.JobId == "") && village.Buildings.All(b => b.Inputs.Count == 0 && b.Outputs.Count == 0), "Support fixture injected observed work, claims, cargo or crops");
        check(Slot().Paused && Slot().Queue.Count == 2 && Slot().Queue.All(q => !q.Repeat && q.AutomatedBy == "") && Slot().AutomatedBy == "", "Finite manual orders or initial pause changed");
        foreach (var leg in new[] { (Carrier().Position, spill.Position), (spill.Position, ingredients.Position), (Worker().Position, ingredients.Position), (ingredients.Position, mill.Position), (mill.Position, ingredients.Position), (mill.Position, receiver.Position) })
            check(world.Path(leg.Item1, leg.Item2, village) != null, "Food-chain fixture lacks physical route " + leg);
        check(world.HasRoom(village, ingredients, "grain", 4) && world.HasRoom(village, ingredients, "flour", 2.2) && world.HasRoom(village, receiver, "food", 4.4), "Fixture lacks researched output space");
        AuthorityRuntime.ValidateWorld(world); // Before the first simulation step.

        double grainLoss = 0, flourLoss = 0, foodLoss = 0;
        var productionIds = new Dictionary<string, string>(StringComparer.Ordinal);
        var completions = new Dictionary<string, Job>(StringComparer.Ordinal);
        string haulId = "";
        void Conserved()
        {
            Near(Physical("grain") + grainLoss + (4 / 2.2) * (Physical("flour") + flourLoss)
                + (4 / 2.2 * 2 / 4.4) * (Physical("food") + foodLoss), 4, "Physical raw-Grain equivalent changed");
            Near(grainLoss, 0, "Exact four-Grain bill was left unprotected at a spoilage boundary");
            check(flourLoss >= 0 && flourLoss < 0.2 && foodLoss >= 0, "Spoilage exceeded the finite remainder");
            check(village.Farms.Count == 0 && village.Cats.All(c => c.Alive && c.Health > 0), "Chain received crop growth or lost a cat");
            check(Mill().WorkerId == workerId && Mill().ExtraWorkerIds.Count == 0 && Mill().Slots.Count == 1
                && Worker().BuildingId == millId && Slot().AutomatedBy == "" && Slot().Queue.All(q => !q.Repeat && q.AutomatedBy == ""), "Restart or planning changed the exact manual worker/slot/orders");
            check(world.Reservations.GroupBy(r => (r.OwnerId, r.PileId, r.Resource)).All(g => g.Count() == 1), "Duplicated a source reservation");
            check(village.Jobs.Where(j => j.Completed).All(j => j.Local.Count == 0), "Completed job retained physical goods outside delivery");
            foreach (var group in world.Reservations.GroupBy(r => (r.PileId, r.Resource)))
                check(group.Sum(r => r.Amount) <= World.Amount(Pile(group.Key.PileId).Goods, group.Key.Resource) + 1e-8, "Source goods were double claimed");
            check(village.Jobs.Count(j => !j.Completed && j.TargetId == millId) <= 1
                && village.Jobs.Count(j => !j.Completed && j.SourceId == spillId) <= 1, "Duplicated an active batch or Steward haul");
        }
        void Step()
        {
            check(world.TimeSeconds + World.SimulationStepSeconds <= 1800 + 1e-8, "Finite chain exceeded 1800 simulated seconds");
            world.Step(World.SimulationStepSeconds);
            // TickVillage spoils last on minute boundaries. Invert that independent
            // rate against the post-step pile, excluding still-reserved goods.
            // This also handles pickup/delivery in the same actor quantum.
            if (Math.Abs(world.TimeSeconds / 60 - Math.Round(world.TimeSeconds / 60)) < 1e-9)
            {
                Near(Catalog.Effect(village, "spoilageResistance", 1), 1, "Unexpected spoilage research");
                check(!village.Buildings.Any(b => b.Kind == "food_storage" && b.Completed), "Unexpected food-store spoilage service");
                double Lost(string resource) => village.Stockpiles.Sum(p => Math.Max(0, World.Amount(p.Goods, resource)
                    - world.Reservations.Where(r => r.PileId == p.Id && r.Resource == resource).Sum(r => r.Amount)) * 0.0002 / (1 - 0.0002));
                grainLoss += Lost("grain"); flourLoss += Lost("flour"); foodLoss += Lost("food");
            }
            foreach (var job in village.Jobs.Where(j => j.Kind == "production" && j.TargetId == millId))
            {
                check(job.RecipeId == milling.Id || job.RecipeId == cooking.Id, "Unexpected production recipe");
                if (productionIds.TryGetValue(job.RecipeId, out string id)) check(id == job.Id, "Finite order started a duplicate batch");
                else productionIds.Add(job.RecipeId, job.Id);
                if (job.Completed && !completions.ContainsKey(job.Id))
                {
                    check(village.Events.Count(e => e.Kind == "completed" && e.EntityId == job.Id) == 1, "Production completion was duplicated or missing");
                    completions.Add(job.Id, WireJson.Decode<Job>(WireJson.Encode(job)));
                }
            }
            Conserved();
        }
        void Reach(Func<bool> predicate, string reason)
        {
            while (!predicate() && world.TimeSeconds < 1800) Step();
            check(predicate(), reason + $" at {world.TimeSeconds:R}s");
        }

        var directory = Path.Combine(Path.GetTempPath(), "forest-food-chain-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(directory);
        int checkpoint = 0;
        void Restart(string label)
        {
            AuthorityRuntime.ValidateWorld(world);
            Conserved();
            string expected = WireJson.Encode(world);
            string path = Path.Combine(directory, $"checkpoint-{++checkpoint}.json");
            SaveStore.Save(path, world, requireNew: true);
            check(WireJson.Encode(world) == expected, "Save mutated authoritative state at " + label);
            world = SaveStore.Load<World>(path); // Strict checksummed load, no runtime/identity initialization.
            check(WireJson.Encode(world) == expected, "Full serialized authority changed at " + label);
            AuthorityRuntime.ValidateWorld(world);
            village = world.Villages.Single(v => v.Id == villageId);
            Conserved();
            string active = string.Join(",", village.Jobs.Where(j => !j.Completed && (j.TargetId == millId || j.SourceId == spillId)).Select(j => $"{j.Id}/{j.Phase}/{j.Progress:R}/{j.RequiredWork:R}"));
            Console.WriteLine($"Food chain checkpoint {checkpoint}: {label}; t={world.TimeSeconds:R}; haul={haulId}; worker={Worker().Id}; active={active}; claims={world.Reservations.Count}; grain={Physical("grain"):R}; flour={Physical("flour"):R}; food={Physical("food"):R}; grainSpoilage={grainLoss:R}; flourSpoilage={flourLoss:R}; foodSpoilage={foodLoss:R}");
        }
        try
        {
            Restart("validated paused fixture");
            Reach(() => village.Jobs.Any(j => !j.Completed && j.Kind == "haul" && j.SourceId == spillId), "Steward did not claim the finite external Grain");
            haulId = village.Jobs.Single(j => !j.Completed && j.Kind == "haul" && j.SourceId == spillId).Id;
            check(Order(haulId).AutomatedBy == "steward" && !Order(haulId).Manual && Order(haulId).CatId == carrierId
                && Order(haulId).TargetId == ingredientsId && Order(haulId).Phase == "fetch", "Observed haul was not the Steward's exact source/carrier/destination");
            Near(Order(haulId).Amount, 4, "Steward finite load");
            Near(world.Reservations.Where(r => r.OwnerId == haulId && r.PileId == spillId && r.Resource == "grain").Sum(r => r.Amount), 4, "Steward claim");
            Near(World.Amount(Pile(spillId).Goods, "grain"), 4, "Claim moved Grain before pickup");
            check(Carrier().Cargo.Count == 0 && productionIds.Count == 0 && Slot().Paused, "Mill or carrier bypassed the source claim");
            Restart("Steward claimed four Grain, still in spill");
            Reach(() => World.Amount(Carrier().Cargo, "grain") == 4, "Steward carrier never physically picked up four Grain");
            check(Carrier().Position.Equals(Pile(spillId).Position) && !Carrier().Position.Equals(Pile(ingredientsId).Position)
                && world.Reservations.All(r => r.OwnerId != haulId), "Steward pickup did not occur at the claimed physical source");
            Near(World.Amount(Pile(spillId).Goods, "grain"), 0, "Pickup left duplicate source Grain");
            Restart("Steward carries four Grain at external source");
            Reach(() => Order(haulId).Completed, "Steward never delivered the same finite haul");
            check(Carrier().Position.Equals(Pile(ingredientsId).Position) && Carrier().Cargo.Count == 0 && Carrier().JobId == ""
                && productionIds.Count == 0 && Slot().Paused && world.Reservations.Count == 0, "Mill fetched directly or haul failed to release ownership");
            Near(World.Amount(Pile(ingredientsId).Goods, "grain"), 4, "Steward storage delivery");
            check(village.Events.Count(e => e.Kind == "completed" && e.EntityId == haulId) == 1, "Steward completion was not recorded exactly once");
            Restart("Steward delivered four Grain while Mill stayed paused");
            var resume = Edit("pause"); resume.Enabled = false; Act(resume);

            foreach (var batch in new[] { (recipe: milling.Id, input: "grain", amount: 4d, output: "flour", outputAmount: 2.2), (recipe: cooking.Id, input: "flour", amount: 2d, output: "food", outputAmount: 4.4) })
            {
                Reach(() => productionIds.ContainsKey(batch.recipe), batch.recipe + " never claimed its delivered input");
                string jobId = productionIds[batch.recipe];
                check(Order(jobId).CatId == workerId && Order(jobId).TargetId == millId && Slot().JobId == jobId
                    && Order(jobId).Phase == "fetch" && Worker().Cargo.Count == 0 && Order(jobId).Local.Count == 0, "Batch lost its exact worker/slot or skipped source claims");
                Near(Order(jobId).RequiredWork, 600, "Actual required station work");
                Near(world.Reservations.Where(r => r.OwnerId == jobId && r.PileId == ingredientsId && r.Resource == batch.input).Sum(r => r.Amount), batch.amount, "Exact batch ingredient claim");
                if (batch.recipe == cooking.Id) Near(World.Amount(Pile(ingredientsId).Goods, "flour"), 2.2, "Upstream Flour must arrive before downstream reservation");
                Restart(batch.recipe + " claimed input in storage");
                Reach(() => World.Amount(Worker().Cargo, batch.input) == batch.amount, batch.recipe + " never physically picked up its input");
                check(Worker().Position.Equals(Pile(ingredientsId).Position) && world.Reservations.All(r => r.OwnerId != jobId), "Batch input pickup was remote or retained duplicate claims");
                Restart(batch.recipe + " carries exact input from storage");
                Reach(() => Order(jobId).Phase == "working" && Order(jobId).Progress > 0, batch.recipe + " never worked at its station");
                check(Worker().Position.Equals(Mill().Position) && Worker().Cargo.Count == 0 && Order(jobId).Progress < 600, "Station work was not physical or finite");
                Near(World.Amount(Order(jobId).Local, batch.input) + World.Amount(Mill().Inputs, batch.input), batch.amount, "Station must retain the whole unconsumed bill during work");
                Restart(batch.recipe + " working with staged input");
                Reach(() => Order(jobId).Phase == "output_delivery" && World.Amount(Worker().Cargo, batch.output) == batch.outputAmount, batch.recipe + " never carried its researched output");
                check(!Order(jobId).Completed && Order(jobId).Progress >= 600 - 1e-8 && Worker().Position.Equals(Mill().Position)
                    && !Slot().Queue.Any(q => q.RecipeId == batch.recipe), "Output skipped station work or replayed its finite queue entry");
                Near(World.Amount(Order(jobId).Local, batch.input), 0, "Completed recipe retained its consumed bill");
                Near(Physical(batch.output), batch.outputAmount, "Physical researched output before delivery");
                // Pause future work while the committed Flour is delivered. This
                // exposes storage delivery even if it lands on a planning second.
                if (batch.recipe == milling.Id)
                {
                    var hold = Edit("pause"); hold.Enabled = true; Act(hold);
                }
                Restart(batch.recipe + " carries researched output from station");
                Reach(() => completions.ContainsKey(jobId), batch.recipe + " never physically delivered output");
                string destinationId = batch.output == "flour" ? ingredientsId : receiverId;
                check(Worker().Position.Equals(Pile(destinationId).Position) && Worker().Cargo.Count == 0 && Worker().JobId == ""
                    && Slot().JobId == jobId && world.Reservations.Count == 0, "Completed batch lost its completed slot marker or retained cargo/claims");
                Near(World.Amount(Pile(destinationId).Goods, batch.output), batch.outputAmount, "Exact output storage delivery");
                Restart(batch.recipe + " delivered researched output once");
                if (batch.recipe == milling.Id)
                {
                    var release = Edit("pause"); release.Enabled = false; Act(release);
                }
            }
            check(productionIds.Count == 2 && completions.Count == 2 && completions.Values.All(j => j.RequiredWork == 600 && j.Progress >= 600 - 1e-8), "Finite chain did not complete exactly two 600-work batches");
            Near(Physical("grain"), 0, "Final Grain");
            Near(Physical("flour") + flourLoss, 0.2, "Remaining Flour plus explicit spoilage");
            Near(Physical("food") + foodLoss, 4.4, "Produced Food is separate from Preserves provisions");
            check(flourLoss > 0 && Slot().Queue.Count == 0 && Mill().Inputs.Count == 0 && Mill().Outputs.Count == 0, "Finite chain retained staged materials/orders or did not cover remainder spoilage");
            double settledAt = world.TimeSeconds;
            Restart("both finite batches complete, 0.2 Flour remainder reconciled");
            while (world.TimeSeconds < settledAt + 61) Step();
            check(productionIds.Count == 2 && completions.Count == 2 && Slot().Queue.Count == 0 && world.Reservations.Count == 0
                && !village.Jobs.Any(j => !j.Completed && (j.TargetId == millId || j.SourceId == spillId))
                && Worker().Cargo.Count == 0 && Carrier().Cargo.Count == 0, "Post-completion restart replayed work, claims or outputs");
            foreach (string id in completions.Keys.Append(haulId))
                check(village.Events.Count(e => e.Kind == "completed" && e.EntityId == id) == 1, "A completed finite job was replayed after reload");
            Near(Physical("flour") + flourLoss, 0.2, "Settled Flour remainder");
            Near(Physical("food") + foodLoss, 4.4, "Settled produced Food");
            Restart("61-second settled continuation, no duplicate completion");
        }
        finally { Directory.Delete(directory, recursive: true); }
    }
}
