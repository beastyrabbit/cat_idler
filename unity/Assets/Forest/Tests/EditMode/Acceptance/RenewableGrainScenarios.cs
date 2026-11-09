using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    /// <summary>Two uncredited crops, witnessed Steward consolidation and three finite Mill batches.</summary>
    public static class RenewableGrainScenarios
    {
        public const double MaximumSeconds = 20000;
        static readonly string[] Studies =
        {
            "research_hut", "basic_tools", "foraging_lore", "sawmill", "masonry", "water_carriers",
            "irrigation", "milling", "grain_milling_sources", "grain_milling_preparation"
        };

        static void Check(bool condition, string message)
        {
            if (!condition) throw new InvalidOperationException("Renewable Grain: " + message);
        }
        static void Near(double actual, double expected, string message) => Check(Math.Abs(actual - expected) < 1e-7,
            message + $"; expected {expected:R}, got {actual:R}");
        static bool At(Cat cat, Int2 position) => cat.Position.Equals(position)
            && Math.Abs(cat.X - position.X) + Math.Abs(cat.Z - position.Z) < 1e-8;
        static void Act(World world, Village village, GameAction action)
        {
            var result = world.Apply(new PlayerContext { PlayerId = "renewable-grain-acceptance", VillageId = village.Id }, action);
            Check(result.Success, action.Kind + " rejected: " + result.Error);
        }
        static void Valid(World world)
        {
            var errors = world.Validate();
            Check(errors.Count == 0, string.Join("; ", errors));
        }
        static IEnumerable<Cat> Eligible(World world, Village village) => village.Cats.Where(c => c.Alive && c.AgeHours >= 12
            && !world.IsDungeonExplorer(c) && c.Position.Level == 0 && c.JobId == "" && c.BuildingId == "" && c.ControlledBy == ""
            && c.Migration != "arriving" && c.Migration != "departing" && !village.Routes.Any(r => r.CatId == c.Id));
        static double Physical(Village village, string resource) => village.Stockpiles.Sum(p => World.Amount(p.Goods, resource))
            + village.Cats.Sum(c => World.Amount(c.Cargo, resource)) + village.Jobs.Sum(j => World.Amount(j.Local, resource))
            + village.Buildings.Sum(b => World.Amount(b.Inputs, resource) + World.Amount(b.Outputs, resource))
            + village.Buildings.SelectMany(b => b.Slots).Sum(s => World.Amount(s.Inputs, resource) + World.Amount(s.Outputs, resource))
            + village.Farms.Where(f => f.Crop == resource).Sum(f => f.Harvest);

        public static World CreateFixture()
        {
            var world = World.Create(41);
            var village = world.Villages.Single();
            village.Buildings.RemoveAll(b => b.Kind != "den" && b.Kind != "shrine");
            village.Stockpiles.Clear();
            village.LastLeaderResearch = 0;
            foreach (var cat in village.Cats)
            {
                cat.Skills.Clear();
                cat.Preferences.Clear();
                cat.Boosted = false;
                cat.Hunger = cat.Thirst = cat.Rest = cat.Health = 100;
                cat.Position = new Int2(3, 0);
                cat.X = 3; cat.Z = 0;
                cat.Path.Clear();
            }
            // Only twelve unclaimed exterior tiles are cleared. Keep founding
            // walls, gates, shrine, beds, stats, ecology and normal needs intact.
            for (int x = 10; x <= 12; x++)
                for (int z = 0; z <= 3; z++)
                {
                    var tile = world.TileAt(new Int2(x, z));
                    Check(tile.ClaimId == "", "support ground must be unclaimed");
                    World.ClearSurface(tile, 0);
                    tile.Wall = false;
                    if (!village.Known.Contains(tile.Position)) village.Known.Add(tile.Position);
                }
            var knownLogs = world.Tiles.First(t => t.Resource == "logs" && t.Amount > 0).Position;
            if (!village.Known.Contains(knownLogs)) village.Known.Add(knownLogs);
            village.Stockpiles.Add(new Stockpile
            {
                Id = world.Id("renewable-provisions"),
                Position = new Int2(3, 2),
                Width = 1,
                Depth = 1,
                Capacity = 1500,
                Accepts = new List<string> { "preserves", "water", "fibre" },
                Goods = new List<Stack> { new Stack("preserves", 600), new Stack("water", 600), new Stack("fibre", 10) }
            });
            village.Stockpiles.Add(new Stockpile
            {
                Id = world.Id("renewable-grain-store"),
                Position = new Int2(3, -4),
                Width = 1,
                Depth = 1,
                Capacity = 100,
                Accepts = new List<string> { "grain" }
            });
            village.Stockpiles.Add(new Stockpile
            {
                Id = world.Id("renewable-flour-store"),
                Position = new Int2(3, -3),
                Width = 1,
                Depth = 1,
                Capacity = 64,
                Accepts = new List<string> { "flour" }
            });
            // Finite setup credit buys only the prerequisites and first milling
            // recipe. No crop, material, work, skill or speed credit is granted.
            village.ResearchPoints = Studies.Sum(id => Catalog.Study(id).Cost);
            Near(village.ResearchPoints, 109.5, "minimal research budget");
            foreach (string study in Studies) Act(world, village, new GameAction { Kind = "ResearchNode", NodeId = study });
            Near(village.ResearchPoints, 0, "research must spend the finite budget");
            village.Buildings.Add(new Building { Id = world.Id("renewable-workshop"), Kind = "workshop", Position = new Int2(4, 4), Completed = true });
            village.Buildings.Add(new Building { Id = world.Id("renewable-field"), Kind = "field", Position = new Int2(6, 4), Completed = true });
            var mill = new Building { Id = world.Id("renewable-mill"), Kind = "mill", Position = new Int2(4, -4), Completed = true };
            village.Buildings.Add(mill);
            Act(world, village, new GameAction { Kind = "DesignateFarm", Position = new Int2(12, 1), End = new Int2(12, 1), Resource = "grain" });
            Act(world, village, new GameAction { Kind = "AssignOfficer", Role = "steward", CatId = village.Cats[0].Id });
            Act(world, village, new GameAction { Kind = "AssignOfficer", Role = "farmer", CatId = village.Cats[1].Id });
            Act(world, village, new GameAction { Kind = "SetCatLaborPreference", CatId = village.Cats[2].Id, Labor = "farm", Enabled = true });
            Act(world, village, new GameAction { Kind = "AssignWorker", BuildingId = mill.Id, CatId = village.Cats[3].Id });
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = mill.Id, CatId = mill.WorkerId, Edit = "pause", Enabled = true });
            Act(world, village, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = mill.Id, CatId = mill.WorkerId, Edit = "delegate" });
            var farm = village.Farms.Single();
            Check(world.TimeSeconds == 0 && farm.Growth == 0 && farm.Harvest == 0 && farm.WorkerId == ""
                && farm.CycleSeconds == 7200 && farm.YieldPerTile == 8 && farm.Width == 1 && farm.Depth == 1,
                "fixture must have a vacant default plot with zero crop credit");
            Check(village.Cats.Count == 30 && village.Cats.All(c => c.Alive && c.ControlledBy == "" && c.Cargo.Count == 0)
                && Eligible(world, village).Count() == 29 && village.Jobs.Count == 0 && world.Reservations.Count == 0,
                "fixture must retain thirty living cats, normal needs and eligible reserve/carrier labor");
            foreach (string resource in new[] { "grain", "flour", "food" }) Near(Physical(village, resource), 0, "authored " + resource);
            var grain = village.Stockpiles.Single(p => p.Accepts.SequenceEqual(new[] { "grain" }));
            var flour = village.Stockpiles.Single(p => p.Accepts.SequenceEqual(new[] { "flour" }));
            foreach (var leg in new[] { (village.Center, farm.Position), (farm.Position, farm.Handoff), (farm.Handoff, grain.Position),
                (village.Cats[3].Position, grain.Position), (grain.Position, mill.Position), (mill.Position, flour.Position) })
                Check(world.Path(leg.Item1, leg.Item2, village) != null, "missing physical fixture route " + leg);
            Check(world.HasRoom(village, grain, "grain", 19.2) && world.HasRoom(village, flour, "flour", 6.6)
                && !world.HasRoom(village, grain, "flour", 1) && !world.HasRoom(village, flour, "grain", 1), "accepting storage filters/capacity");
            Valid(world);
            return world;
        }

        sealed class HaulWitness
        {
            public string CarrierId;
            public double Amount;
            public bool Picked, Delivered;
        }

        /// <summary>Callback may replace the entire world only with an exact checkpoint round trip.</summary>
        public static void RunChain(Func<World, string, World> checkpoint = null)
        {
            var world = CreateFixture();
            string villageId = world.Villages.Single().Id;
            Village Village() => world.Villages.Single(v => v.Id == villageId);
            string farmId = Village().Farms.Single().Id, millId = Village().Buildings.Single(b => b.Kind == "mill").Id;
            string farmerId = Village().Cats[2].Id, millerId = Village().Cats[3].Id;
            string grainId = Village().Stockpiles.Single(p => p.Accepts.SequenceEqual(new[] { "grain" })).Id;
            string flourId = Village().Stockpiles.Single(p => p.Accepts.SequenceEqual(new[] { "flour" })).Id;
            Farm Plot() => Village().Farms.Single(f => f.Id == farmId);
            Building Mill() => Village().Buildings.Single(b => b.Id == millId);
            WorkSlot Slot() => Mill().Slots.Single(s => s.CatId == millerId);
            Cat Cat(string id) => Village().Cats.Single(c => c.Id == id);
            Stockpile Pile(string id) => Village().Stockpiles.Single(p => p.Id == id);
            Job Order(string id) => Village().Jobs.Single(j => j.Id == id);
            var recipe = Catalog.Recipe("grain_to_flour");
            double bill = world.RecipeInput(Village(), recipe, recipe.Inputs.Single().Amount);
            double output = world.RecipeOutput(Village(), recipe, recipe.Outputs.Single().Amount);
            Near(bill, 4, "researched Grain bill"); Near(output, 2.2, "researched Flour output");
            Near(world.RecipeDuration(Village(), recipe), 600, "normal Mill work");
            double yield = Plot().YieldPerTile * Catalog.Effect(Village(), "farmYield", 1)
                * Catalog.Effect(Village(), "farmYieldMult", 1) * Catalog.BuildingEffect(Village(), "field", "output", 1);
            Near(yield, 9.6, "Irrigation harvest");
            int harvests = 0, farmLoads = 0, handoffs = 0;
            double grown = 0, grainLoss = 0, flourLoss = 0, consumed = 0, produced = 0;
            string handoffId = "";
            var hauls = new Dictionary<string, HaulWitness>(StringComparer.Ordinal);
            var batches = new HashSet<string>(StringComparer.Ordinal);
            var converted = new HashSet<string>(StringComparer.Ordinal);
            var finished = new HashSet<string>(StringComparer.Ordinal);
            double SourceGrain() => handoffId == "" ? 0 : World.Amount(Pile(handoffId).Goods, "grain");
            GameAction Edit(string edit) => new GameAction { Kind = "EditProductionWorkSlot", BuildingId = millId, CatId = millerId, Edit = edit };
            void ActHere(GameAction action) => Act(world, Village(), action);
            void PauseAndHandBack()
            {
                var hold = Edit("pause"); hold.Enabled = true; ActHere(hold);
                ActHere(Edit("delegate"));
                Check(Slot().Paused && Slot().AutomatedBy == "farmer", "public pause/handback lost the same Miller");
            }
            void Conserved()
            {
                var village = Village();
                Near(Physical(village, "grain") + grainLoss + consumed, harvests * yield, "crop Grain including outstanding harvest, cargo, local goods, buffers and spoilage");
                Near(Physical(village, "flour") + flourLoss, produced, "produced Flour including all physical compartments and spoilage");
                Near(Physical(village, "grain") + grainLoss + bill / output * (Physical(village, "flour") + flourLoss), harvests * yield, "raw-Grain equivalent");
                Near(Physical(village, "food"), 0, "this chain must not author, hunt, produce or consume Food");
                Check(village.Cats.Count == 30 && village.Cats.All(c => c.Alive && c.Health > 0 && c.ControlledBy == "" && !c.Boosted), "living population/control policy changed");
                Check(village.Research.Count == Studies.Length && Studies.All(village.Research.Contains), "unbudgeted research");
                Check(village.Farms.Count == 1 && Plot().CycleSeconds == 7200 && Plot().YieldPerTile == 8
                    && (Plot().WorkerId == "" && world.TimeSeconds < 10 || Plot().WorkerId == farmerId && Plot().AutomatedBy == "farmer" && Cat(farmerId).BuildingId == farmId), "plot identity, defaults or Farmer staffing changed");
                Check(Mill().WorkerId == millerId && Mill().ExtraWorkerIds.Count == 0 && Mill().Slots.Count == 1 && Cat(millerId).BuildingId == millId,
                    "exact Miller/position ownership changed");
                Check(Eligible(world, village).Count() >= 4, "thirty-cat village lost its four eligible free-adult reserve");
                Check(village.Jobs.Where(j => !j.Completed && j.CatId != "").GroupBy(j => j.CatId).All(g => g.Count() == 1), "duplicate active cat ownership");
                Check(village.Jobs.Where(j => !j.Completed && j.TargetId == millId).Count() <= 1, "duplicate Mill batch");
                Check(world.Reservations.GroupBy(r => (r.OwnerId, r.PileId, r.Resource)).All(g => g.Count() == 1), "duplicate source claims");
                Check(world.Reservations.All(r => village.Jobs.Any(j => !j.Completed && (r.OwnerId == j.Id || r.OwnerId == j.Id + ":resume"))), "orphaned source claim");
                foreach (var group in world.Reservations.GroupBy(r => (r.PileId, r.Resource)))
                    Check(group.All(r => r.Amount > 0) && group.Sum(r => r.Amount) <= World.Amount(Pile(group.Key.PileId).Goods, group.Key.Resource) + 1e-8, "overclaimed physical source");
                foreach (var pile in village.Stockpiles)
                    Check(pile.Goods.All(s => s.Amount >= 0) && pile.Goods.Sum(s => s.Amount) <= world.Capacity(village, pile) + 1e-8, "negative goods or overfilled pile");
                Check(village.Jobs.Where(j => j.Completed).All(j => j.Local.Count == 0), "completed job retained goods");
                if (handoffId != "") Check(village.Stockpiles.Single(p => p.ManagedBy == farmId).Id == handoffId, "later crop replaced/duplicated the managed handoff");
            }
            void Checkpoint(string label)
            {
                Conserved(); Valid(world);
                if (checkpoint != null)
                {
                    world = checkpoint(world, label);
                    Check(world != null, "checkpoint returned no World at " + label);
                    Conserved(); Valid(world);
                }
            }
            void Step()
            {
                Check(world.TimeSeconds + World.SimulationStepSeconds <= MaximumSeconds + 1e-8, "exceeded 20,000 simulated seconds");
                double beforeGrowth = Plot().Growth, beforeHarvest = Plot().Harvest;
                double beforeCargo = World.Amount(Cat(farmerId).Cargo, "grain"), beforeSource = SourceGrain();
                var priorJobs = Village().Jobs.Where(j => !j.Completed).ToArray();
                var priorProgress = priorJobs.Where(j => j.Kind == "production").ToDictionary(j => j.Id, j => j.Progress);
                var labels = new List<string>();
                world.Step(World.SimulationStepSeconds);
                var village = Village();
                bool minute = Math.Abs(world.TimeSeconds / 60 - Math.Round(world.TimeSeconds / 60)) < 1e-9;
                double Loss(Stockpile pile, string resource) => minute ? Math.Max(0, World.Amount(pile.Goods, resource)
                    - world.Reservations.Where(r => r.PileId == pile.Id && r.Resource == resource).Sum(r => r.Amount)) * 0.0002 / (1 - 0.0002) : 0;
                // Spoilage runs last at minute boundaries. Its independent inverse
                // excludes claims and includes same-quantum pickups/deposits.
                Near(Catalog.Effect(village, "spoilageResistance", 1), 1, "spoilage resistance");
                Check(!village.Buildings.Any(b => b.Kind == "food_storage" && b.Completed), "unexpected spoilage service");
                grainLoss += village.Stockpiles.Sum(p => Loss(p, "grain"));
                flourLoss += village.Stockpiles.Sum(p => Loss(p, "flour"));
                if (Plot().Growth > beforeGrowth)
                {
                    Check(At(Cat(farmerId), Plot().Position) && Plot().Harvest == 0 && beforeHarvest == 0 && beforeCargo == 0,
                        "growth advanced away from plot work or before clearing harvest/cargo");
                    Check(Plot().Growth - beforeGrowth <= 20 * World.SimulationStepSeconds + 1e-8, "growth skipped actor-time budget");
                    grown += Plot().Growth - beforeGrowth;
                }
                if (Plot().Harvest > beforeHarvest)
                {
                    Check(At(Cat(farmerId), Plot().Position) && beforeHarvest == 0 && Plot().Growth == 0 && grown >= 7199 && harvests < 2,
                        "harvest lacked a complete uncredited physical growth cycle");
                    Near(Plot().Harvest, yield, "new researched harvest");
                    harvests++; grown = 0;
                    labels.Add("cycle-" + harvests + ": harvest outstanding at plot");
                }
                else Check(Plot().Growth >= beforeGrowth, "growth reset without a new harvest");
                double cropCargo = World.Amount(Cat(farmerId).Cargo, "grain");
                if (cropCargo > beforeCargo)
                {
                    Check(At(Cat(farmerId), Plot().Position) && Plot().Growth == 0, "farmer picked up remotely");
                    Near(cropCargo - beforeCargo, beforeHarvest - Plot().Harvest, "harvest-to-cargo transfer");
                    Near(cropCargo, farmLoads % 2 == 0 ? 8 : 1.6, "eight/remainder crop load");
                    farmLoads++;
                    labels.Add("cycle-" + harvests + ": farmer loaded " + (farmLoads % 2 == 1 ? "eight" : "remainder"));
                }
                var handoff = village.Stockpiles.SingleOrDefault(p => p.ManagedBy == farmId);
                if (handoff != null)
                {
                    if (handoffId == "") handoffId = handoff.Id;
                    Check(handoff.Kind == "farm_output" && handoff.Position.Equals(Plot().Handoff), "crop handoff identity/kind/location");
                    double picked = 0;
                    foreach (var job in village.Jobs.Where(j => !j.Completed && j.Kind == "haul" && j.SourceId == handoffId))
                    {
                        if (!hauls.TryGetValue(job.Id, out var witness))
                        {
                            witness = new HaulWitness { CarrierId = job.CatId, Amount = job.Amount };
                            hauls.Add(job.Id, witness);
                            Check(job.AutomatedBy == "steward" && !job.Manual && job.Phase == "fetch" && job.TargetId == grainId
                                && Cat(job.CatId).Cargo.Count == 0 && job.Amount > 0 && job.Amount <= 8 && Slot().Paused,
                                "new haul was not a witnessed Steward source claim while Mill paused");
                            Near(world.Reservations.Where(r => r.OwnerId == job.Id && r.PileId == handoffId && r.Resource == "grain").Sum(r => r.Amount), job.Amount, "Steward source reservation");
                            Check(SourceGrain() + 1e-8 >= job.Amount, "Steward claim moved goods before physical pickup");
                            labels.Add("cycle-" + harvests + ": Steward claim " + job.Id);
                        }
                        Check(job.CatId == witness.CarrierId && job.TargetId == grainId && job.AutomatedBy == "steward", "Steward leg changed owner/destination");
                        if (!witness.Picked && World.Amount(Cat(job.CatId).Cargo, "grain") > 0)
                        {
                            Check(At(Cat(job.CatId), handoff.Position) && world.Reservations.All(r => r.OwnerId != job.Id), "Steward pickup was remote or retained its claim");
                            Near(World.Amount(Cat(job.CatId).Cargo, "grain"), witness.Amount, "Steward loaded amount");
                            witness.Picked = true; picked += witness.Amount;
                            labels.Add("cycle-" + harvests + ": Steward carrying " + job.Id);
                        }
                    }
                    Check(village.Jobs.Count(j => !j.Completed && j.Kind == "haul" && j.SourceId == handoffId) <= 1, "duplicate Steward source owner");
                    double deposited = SourceGrain() + Loss(handoff, "grain") - beforeSource + picked;
                    if (deposited > 1e-7)
                    {
                        Check(At(Cat(farmerId), Plot().Handoff) && cropCargo == 0, "farmer handoff occurred before physical arrival");
                        Near(deposited, beforeCargo, "farmer cargo-to-handoff transfer");
                        handoffs++;
                        labels.Add("cycle-" + harvests + ": farmer handoff " + handoffs);
                    }
                    else Near(deposited, 0, "unexplained handoff source decrease");
                }
                // Include pre-step objects: normal minute cleanup can retire a
                // batch on the very quantum in which it physically finishes.
                foreach (var job in priorJobs.Concat(village.Jobs).GroupBy(j => j.Id).Select(g => g.First()))
                {
                    if (job.Kind == "production" && job.TargetId == millId)
                    {
                        batches.Add(job.Id);
                        Check(job.RecipeId == recipe.Id && job.CatId == millerId && batches.Count <= (harvests == 0 ? 0 : harvests == 1 ? 2 : 3), "unexpected/replayed Mill batch");
                        Near(job.RequiredWork, 600, "actual batch required work");
                        if (priorProgress.TryGetValue(job.Id, out double progress) && job.Progress > progress)
                            Check(At(Cat(millerId), Mill().Position) && job.Progress - progress <= 20 * World.SimulationStepSeconds + 1e-8, "Mill work was remote or skipped time");
                        if (job.Phase == "output_delivery" && converted.Add(job.Id))
                        {
                            Check(job.Progress >= 600 - 1e-8 && At(Cat(millerId), Mill().Position), "output skipped actual station work");
                            Near(World.Amount(job.Reserved, "grain"), bill, "actual consumed batch bill");
                            Near(World.Amount(job.Local, "grain"), 0, "converted batch kept its input");
                            Near(World.Amount(job.Local, "flour") + World.Amount(Cat(millerId).Cargo, "flour"), output, "actual produced batch quantity");
                            consumed += bill; produced += output;
                        }
                    }
                    if (!job.Completed || finished.Contains(job.Id)) continue;
                    Check(village.Events.Count(e => e.Kind == "completed" && e.EntityId == job.Id) == 1, "missing/duplicate physical completion");
                    finished.Add(job.Id);
                    if (hauls.TryGetValue(job.Id, out var haul))
                    {
                        Check(haul.Picked && !haul.Delivered && At(Cat(haul.CarrierId), Pile(grainId).Position)
                            && World.Amount(Cat(haul.CarrierId).Cargo, "grain") == 0 && world.Reservations.All(r => r.OwnerId != job.Id), "Steward delivery skipped arrival or retained ownership");
                        haul.Delivered = true;
                        labels.Add("cycle-" + harvests + ": Steward arrived " + job.Id);
                    }
                }
                Conserved();
                foreach (string label in labels) Checkpoint(label);
            }
            void Reach(Func<bool> predicate, string reason)
            {
                while (!predicate() && world.TimeSeconds < MaximumSeconds - 1e-8) Step();
                Check(predicate(), reason + $" at {world.TimeSeconds:R}s");
            }

            Checkpoint("zero-credit paused fixture");
            Reach(() => Plot().WorkerId != "", "Farmer did not staff the reachable plot");
            Check(Plot().WorkerId == farmerId && Plot().AutomatedBy == "farmer" && Eligible(world, Village()).Count() >= 4, "Farmer ignored preferred eligible labor/reserve");
            Checkpoint("Farmer staffed plot with eligible reserve");
            for (int cycle = 1; cycle <= 2; cycle++)
            {
                int completedBefore = cycle == 1 ? 0 : 2;
                if (cycle == 2)
                {
                    Check(harvests == 1 && converted.Count == 2 && batches.All(finished.Contains), "first crop's two finite batches must finish before renewal");
                    Near(consumed, 8, "first crop's two consumed bills");
                    Check(Physical(Village(), "grain") < bill, "old crop alone can still fund a batch; renewal is not causally required");
                    Checkpoint("first crop exhausted below one bill before second harvest");
                }
                Reach(() => harvests == cycle, "natural crop cycle did not complete");
                Reach(() => farmLoads == cycle * 2 && handoffs == cycle * 2 && Plot().Harvest == 0
                    && World.Amount(Cat(farmerId).Cargo, "grain") == 0 && SourceGrain() == 0 && hauls.Values.All(h => h.Delivered),
                    "crop did not complete both farmer loads and all Steward deposits");
                Check(hauls.Count >= cycle * 2 && Slot().Paused && batches.Count == completedBefore, "Mill bypassed witnessed Steward consolidation");
                Near(World.Amount(Pile(grainId).Goods, "grain"), harvests * yield - grainLoss - consumed, "entire remaining crop must physically reach accepting Grain storage");
                Checkpoint("cycle-" + cycle + ": crop consolidated while Mill paused");
                int count = cycle == 1 ? 2 : 1;
                for (int batch = 1; batch <= count; batch++)
                {
                    int ordinal = completedBefore + batch;
                    string label = "cycle-" + cycle + " batch-" + batch;
                    var add = Edit("add"); add.RecipeId = recipe.Id; add.Repeat = false; ActHere(add);
                    var resume = Edit("pause"); resume.Enabled = false; ActHere(resume);
                    Check(!Slot().Paused && Slot().AutomatedBy == "" && Slot().Queue.Count == 1 && !Slot().Queue[0].Repeat,
                        "public Resume must retain the finite manual order");
                    Reach(() => batches.Count == ordinal, "Mill did not claim the delivered Grain");
                    string batchId = batches.Single(id => !converted.Contains(id));
                    Check(Order(batchId).Phase == "fetch" && Slot().JobId == batchId && Cat(millerId).Cargo.Count == 0, "Mill skipped input reservation");
                    Near(world.Reservations.Where(r => r.OwnerId == batchId && r.PileId == grainId && r.Resource == "grain").Sum(r => r.Amount), bill, "Mill exact storage claim");
                    Checkpoint(label + ": Mill claimed stored Grain");
                    Reach(() => World.Amount(Cat(millerId).Cargo, "grain") == bill, "Miller did not pick up the physical input");
                    Check(At(Cat(millerId), Pile(grainId).Position) && world.Reservations.All(r => r.OwnerId != batchId), "Miller fetched remotely or retained its input claim");
                    Checkpoint(label + ": Miller carrying input from storage");
                    Reach(() => Order(batchId).Phase == "working" && Order(batchId).Progress > 0, "Miller never staged and worked the batch");
                    Check(At(Cat(millerId), Mill().Position) && Order(batchId).Progress < 600 && Cat(millerId).Cargo.Count == 0, "Mill work skipped physical staging");
                    Near(World.Amount(Order(batchId).Local, "grain"), bill, "staged unconsumed bill");
                    Checkpoint(label + ": Mill working with staged input");
                    Reach(() => converted.Contains(batchId), "Mill never produced its researched output");
                    Check(Order(batchId).Phase == "output_delivery" && !Order(batchId).Completed && Slot().Queue.Count == 0, "finite order did not commit exactly once");
                    Near(World.Amount(Cat(millerId).Cargo, "flour"), output, "Miller researched output load");
                    PauseAndHandBack();
                    Checkpoint(label + ": Mill output loaded, paused and handed back");
                    Reach(() => finished.Contains(batchId), "Miller never delivered output to accepting storage");
                    Check(At(Cat(millerId), Pile(flourId).Position) && Cat(millerId).Cargo.Count == 0 && Cat(millerId).JobId == ""
                        && Slot().Paused && Slot().AutomatedBy == "farmer", "Flour delivery skipped arrival or lost paused handback");
                    Near(World.Amount(Pile(flourId).Goods, "flour") + flourLoss, ordinal * output, "researched stored Flour");
                    Checkpoint(label + ": Flour physically stored");
                }
            }
            double settledAt = world.TimeSeconds;
            while (world.TimeSeconds < settledAt + 61 - 1e-8) Step();
            Check(harvests == 2 && farmLoads == 4 && handoffs == 4 && batches.Count == 3 && converted.Count == 3
                && batches.All(finished.Contains) && hauls.Values.All(h => h.Picked && h.Delivered) && Slot().Queue.Count == 0
                && world.Reservations.Count == 0 && !Village().Jobs.Any(j => !j.Completed && (j.TargetId == millId || j.SourceId == handoffId)),
                "settled continuation replayed crops, loads, finite batches or claims");
            Check(grainLoss > 0 && flourLoss > 0 && Plot().Growth > 0, "scenario missed spoilage or natural continuation after its second crop");
            Near(consumed, 12, "three actual consumed bills exceed one crop's 9.6 Grain"); Near(produced, 6.6, "three actual produced batches");
            Checkpoint("two crops and three batches settled for 61 seconds without replay");
        }
    }
}
