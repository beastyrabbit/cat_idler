using IdleCatForest.Authority;
using IdleCatForest.Simulation;
using Stack = IdleCatForest.Simulation.Stack;

bool defense = args.Length == 3 && args[2] == "--defense";
bool logistics = args.Length == 3 && args[2] == "--logistics";
bool officerMaterials = args.Length == 3 && args[2] == "--officer-materials";
bool workshopControls = args.Length == 3 && args[2] == "--workshop-controls";
bool equipmentControls = args.Length == 3 && args[2] == "--equipment-controls";
bool openSlotControls = args.Length == 3 && args[2] == "--open-slot-controls";
bool chainControls = args.Length == 3 && args[2] == "--chain-controls";
if (args.Length != 2 && !defense && !logistics && !officerMaterials && !workshopControls && !equipmentControls && !openSlotControls && !chainControls || !int.TryParse(args[1], out int population) || population != 30 && population != 150 || defense && population != 150 || (logistics || officerMaterials || workshopControls || equipmentControls || openSlotControls || chainControls) && population != 30)
{ Console.Error.WriteLine("Usage: Forest.PresentationFixture <new-absolute-save-path> <30|150> [--defense (150 only) | --logistics (30 only) | --officer-materials (30 only) | --workshop-controls (30 only) | --equipment-controls (30 only) | --open-slot-controls (30 only) | --chain-controls (30 only)]"); return 2; }
string target = Path.GetFullPath(args[0]);
if (!Path.IsPathRooted(args[0]) || File.Exists(target) || Directory.Exists(target + ".identity"))
{ Console.Error.WriteLine("Choose a new absolute save path with no existing save or identity directory."); return 2; }
try
{
    using var authority = LocalAuthority.CreateNew(target, 41);
    if (population == 150) Populate(authority, population);
    else PrepareFounding(authority);
    if (defense) PrepareDefense(authority);
    if (logistics) PrepareLogistics(authority);
    if (officerMaterials) PrepareOfficerMaterials(authority);
    if (workshopControls) PrepareWorkshopControls(authority);
    if (equipmentControls) PrepareEquipmentControls(authority);
    if (openSlotControls) PrepareOpenSlotControls(authority);
    if (chainControls) PrepareChainControls(authority);
    var world = authority.World; var village = world.Villages.Single(); var failures = world.Validate();
    if (failures.Count > 0) throw new InvalidDataException(string.Join("; ", failures));
    ValidateLayout(world, village);
    Check(village.Cats.Where(c => c.Alive).All(c => village.Buildings.Any(b => b.Id == c.BedId && b.Kind == "den" && b.Completed)), "Every living cat needs a real completed Den");
    Check(village.Cats.Select(c => c.Id).Distinct().Count() == population, "Cat identities must be unique");
    authority.Save();
    var restored = SaveStore.Load<World>(target); Check(restored.Villages[0].Cats.Count(c => c.Alive) == population, "Saved population changed"); ValidateLayout(restored, restored.Villages[0]);
    if (openSlotControls) ValidateOpenSlotControls(restored, restored.Villages[0]);
    if (chainControls) ValidateChainControls(restored, restored.Villages[0]);
    Console.WriteLine("Created isolated " + population + "-cat fixture; dens=" + village.Buildings.Count(b => b.Kind == "den") + " buildings=" + village.Buildings.Count + " staffed=" + village.Buildings.Sum(b => b.Slots.Count) + " recipes=" + village.Buildings.Sum(b => b.Slots.Sum(s => s.Queue.Count)) + " research_points=" + village.ResearchPoints + " connected_entrances=" + village.Buildings.Count(b => b.HasEntrance) + " gates=4 validation=passed");
    if (defense) Console.WriteLine("Defense review: pause on opening. North Defender (0,18), South Defender (0,-18); raiders at (0,21)/(0,-21), 1 health each. Captain Candidate and Training Candidate wait at (0,4)/(0,5). Captain is vacant; initial readiness is 1/10. Orders, cancellation, Captain appointment and training use normal controls.");
    if (logistics)
    {
        var cutters = village.Buildings.Where(b => b.Kind == "wood_cutter").ToArray();
        Console.WriteLine("Logistics review: pause on opening. Earlier Wood Cutter at " + At(cutters[0].Position) + " is deliberately paused and empty; later Wood Cutter at " + At(cutters[1].Position) + " is unpaused and empty. Logistics Worker waits at its entrance. The Forester office is vacant; Forester Candidate is free. No work order is injected.");
        foreach (var pile in village.Stockpiles.Where(p => p.Id.Contains("logistics-", StringComparison.Ordinal)).OrderBy(p => p.Id, StringComparer.Ordinal))
            Console.WriteLine(pile.Id + " at " + At(pile.Position) + ": 5 finite logs; route=" + (world.Path(cutters[1].Position, pile.Position, village) == null ? "blocked" : "reachable"));
    }
    if (officerMaterials)
    {
        var clothier = village.Buildings.Single(b => b.Kind == "clothier");
        Console.WriteLine("Officer materials review: pause on opening. Clothier at " + At(clothier.Position) + ", entrance " + At(clothier.Entrance) + ", is completed and empty. Textiles and its prerequisites are owned; Cloth Leader Candidate and Textile Worker are free. Appoint Cloth Leader Candidate through Officers, then resume. Ordinary automation should staff Textile Worker and choose Fibre To Thread. No officer, worker slot, queue or production job is injected.");
        foreach (var pile in village.Stockpiles.Where(p => p.Id.Contains("officer-materials-", StringComparison.Ordinal)).OrderBy(p => p.Id, StringComparer.Ordinal))
            Console.WriteLine(pile.Id + " at " + At(pile.Position) + ": " + string.Join(", ", pile.Goods.Select(s => s.Amount + " finite " + s.Resource)) + "; route=" + (world.Path(clothier.Position, pile.Position, village) == null ? "blocked" : "reachable"));
    }
    if (workshopControls)
    {
        var cutter = village.Buildings.Last(b => b.Kind == "wood_cutter");
        var carpentry = village.Buildings.Single(b => b.Kind == "woodworking");
        Console.WriteLine("Workshop controls review: pause on opening. Workshop Worker waits at the later Wood Cutter entrance " + At(cutter.Entrance) + "; Forester Candidate is free and the office is vacant. Sawmill study and completed Sawmill are prepared. Forester workplaces start paused and empty. Use normal staffing, finite/repeating queue edits, appointment, Resume and Return to Forester. Carpentry at " + At(carpentry.Position) + " shows its two named inputs and existing suppliers. No officer, worker slot, queue or production job is injected.");
        Console.WriteLine("Authored historical report at 0h: Logs 3, Planks 1, Blocks 0, Stone 0. Other storage is uncounted; actual finite goods are separate from those reports.");
        foreach (var pile in village.Stockpiles.Where(p => p.Id.Contains("workshop-controls-", StringComparison.Ordinal)).OrderBy(p => p.Id, StringComparer.Ordinal))
            Console.WriteLine(pile.Id + " at " + At(pile.Position) + ": " + string.Join(", ", pile.Goods.Select(s => s.Amount + " finite " + s.Resource)) + "; route=" + (world.Path(cutter.Position, pile.Position, village) == null ? "blocked" : "reachable"));
    }
    if (equipmentControls)
    {
        var runner = village.Cats.Single(c => c.Name == "Equipment Runner");
        Console.WriteLine("Equipment controls review: pause on opening. Equipment Runner is a free adult at " + At(runner.Position) + "; Captain is vacant. Use Stores to request exact equipment, watch physical travel, cancel the exact order, then retrieve and return it through ordinary controls. No work, equipment order or worn item is injected. Reports at 0h are authored setup data, not Accountant visits.");
        foreach (var pile in village.Stockpiles.Where(p => p.Id.StartsWith("equipment-review-", StringComparison.Ordinal)).OrderBy(p => p.Id, StringComparer.Ordinal))
            Console.WriteLine(pile.Id + " at " + At(pile.Position) + ": " + string.Join(", ", village.Items.Where(i => i.LocationId == pile.Id).Select(i => i.Id + " (" + i.Material + " " + i.Kind + ", quality " + i.Quality + ", condition " + i.Condition + "/" + i.MaxCondition + ")")) + "; route=" + (world.Path(runner.Position, pile.Position, village) == null ? "blocked" : "reachable"));
    }
    if (openSlotControls)
    {
        var clothier = village.Buildings.Single(b => b.Kind == "clothier");
        var fibre = village.Stockpiles.Single(p => p.Id == "open-slot-controls-fibre");
        Console.WriteLine("Open position review: pause on opening. Clothier at " + At(clothier.Position) + ", entrance " + At(clothier.Entrance) + ", has researched capacity two. Position 1 is empty, paused and holds one manual repeating Fibre To Thread order. Position 2 is empty, unpaused and holds finite manual Fibre To Thread then Fibre To Cloth orders. Appoint Cloth Leader Candidate through Officers; automation should staff only position 2 with Textile Worker and preserve both manual queues. The first pause must remain untouched.");
        Console.WriteLine("Finite inputs: five Fibre at " + At(fibre.Position) + "; 1,000 Food and 1,000 Water. Clothier Workflow yields 5.6 Thread from the first batch, then 1.12 Cloth plus 0.6 remaining Thread. No replenishing officer or active production job is prepared. Reports at 0h are authored setup counts. Survival work may consume provisions during actual play.");
        Console.WriteLine("Equipment Runner remains free at " + At(village.Cats.Single(c => c.Name == "Equipment Runner").Position) + ". Four exact unworn items use the equipment-review sources; test cat-inspector Equipment navigation, Retrieve, cancellation and physical return through ordinary controls. No worn equipment, item claims, production jobs or officer is injected; only the two documented empty positions and their manual queues were authored via public actions.");
        foreach (var pile in village.Stockpiles.Where(p => p.Id.StartsWith("equipment-review-", StringComparison.Ordinal)).OrderBy(p => p.Id, StringComparer.Ordinal))
            Console.WriteLine(pile.Id + " at " + At(pile.Position) + ": " + string.Join(", ", village.Items.Where(i => i.LocationId == pile.Id).Select(i => i.Id + " (" + i.Material + " " + i.Kind + ", quality " + i.Quality + ", condition " + i.Condition + "/" + i.MaxCondition + ")")) + "; route=" + (world.Path(village.Center, pile.Position, village) == null ? "blocked" : "reachable"));
    }
    if (chainControls)
    {
        Console.WriteLine("Finite workshop chain review: pause on opening. No station is staffed, and no queue, job, officer or input claim is injected. Preview the founding recipe bills before staffing. Staff Plank Worker at the later Wood Cutter, Block Worker at the northern Stone Prep and Tool Worker at the northwest Woodworking station through normal controls. Queue two finite Logs To Planks orders, two finite Stone To Blocks orders and one finite Planks And Blocks To Tools order; do not enable repeat or buy output/efficiency studies during this conservation check.");
        Console.WriteLine("Ten finite Logs and ten finite Stone must become two Planks and two Blocks, then one exact wooden Tool. Initial Planks, Blocks and Tools are zero. Every baseline batch requires 600 work; authored skill 40 affects work rate only. Stock counts at 0h are authored setup reports, not Accountant visits. Food and Water are finite 1,000 each, and normal survival may consume them. This prepared chain does not establish sustainable production.");
        foreach (var cat in village.Cats.Where(c => new[] { "Plank Worker", "Block Worker", "Tool Worker" }.Contains(c.Name)))
            Console.WriteLine(cat.Name + " waits at " + At(cat.Position) + ".");
        foreach (var pile in village.Stockpiles.Where(p => p.Id.StartsWith("chain-controls-", StringComparison.Ordinal)))
            Console.WriteLine(pile.Id + " at " + At(pile.Position) + ": " + string.Join(", ", pile.Goods.Select(s => s.Amount + " finite " + s.Resource)) + ".");
    }
    return 0;
}
catch (Exception error) { Console.Error.WriteLine("Fixture creation failed: " + error.GetType().Name + ". Destination was preserved for inspection."); return 1; }

static void Check(bool condition, string reason) { if (!condition) throw new InvalidDataException(reason); }
static string At(Int2 position) => position.X + "," + position.Z;
static void Apply(LocalAuthority authority, GameAction action)
{ var result = authority.Apply(action); Check(result.Success, action.Kind + " rejected: " + result.Error); }
static void PrepareFounding(LocalAuthority authority)
{
    var v = authority.World.Villages[0];
    // Finite review supplies and research currency allow the normal UI to demonstrate actions.
    v.ResearchPoints = 20000; v.LastLeaderResearch = authority.World.TimeSeconds; v.Blessings = 200;
    foreach (var resource in new[] { "planks", "blocks", "logs", "stone", "fibre", "ore", "materials" }) World.Add(v.Stockpiles[0].Goods, resource, 100);
}
static void PrepareLogistics(LocalAuthority authority)
{
    var world = authority.World; var village = world.Villages.Single();
    Check(village.Cats.Count(c => c.Alive) == 30 && village.Jobs.Count == 0 && village.Officers.Count == 0, "Logistics review must begin with the idle communal founding");
    foreach (var pile in village.Stockpiles)
    {
        pile.Goods.RemoveAll(s => s.Resource != "food" && s.Resource != "water");
        pile.Report.Clear(); pile.CountedAt = -1;
    }
    var store = village.Stockpiles[0];
    World.Add(store.Goods, "food", 1000 - World.Amount(store.Goods, "food"));
    World.Add(store.Goods, "water", 1000 - World.Amount(store.Goods, "water"));
    var cutters = village.Buildings.Where(b => b.Kind == "wood_cutter").ToArray();
    Check(cutters.Length == 2 && cutters.All(b => b.Completed && b.HasEntrance && b.Slots.Count == 0), "Logistics review needs the two normal empty founding Wood Cutters");
    Apply(authority, new GameAction { Kind = "EditProductionQueue", BuildingId = cutters[0].Id, Edit = "pause", Enabled = true });
    var worker = village.Cats[1]; worker.Name = "Logistics Worker"; worker.Position = cutters[1].Entrance; worker.X = worker.Position.X; worker.Z = worker.Position.Z; worker.Path.Clear();
    worker.Skills.Clear(); World.Add(worker.Skills, "process", 25); worker.Preferences.Clear(); worker.Preferences.Add("process");
    village.Cats[2].Name = "Forester Candidate";
    var spaces = world.Tiles.Where(t => t.ClaimId == village.Id && !t.Wall && !t.Water && !t.Mountain && !t.Road && !t.Rail && Math.Max(Math.Abs(t.Position.X), Math.Abs(t.Position.Z)) > 2 &&
        !village.Buildings.Any(b => Inside(b.Position, b.Width, b.Depth, t.Position) || b.Entrance.Equals(t.Position)) &&
        !village.Stockpiles.Any(p => Inside(p.Position, p.Width, p.Depth, t.Position)) && world.Path(cutters[1].Position, t.Position, village) != null)
        .OrderBy(t => Int2.Distance(t.Position, cutters[1].Position)).ThenBy(t => t.Position.Z).ThenBy(t => t.Position.X).Take(3).Select(t => t.Position).ToArray();
    Check(spaces.Length == 3, "Logistics review needs three reachable off-road supply spaces");
    var ids = new[] { "aaa-logistics-blocked-logs", "bbb-logistics-first-logs", "ccc-logistics-alternate-logs" };
    for (int i = 0; i < spaces.Length; i++)
        village.Stockpiles.Add(new Stockpile { Id = ids[i], Position = spaces[i], Width = 1, Depth = 1, Capacity = 20, Accepts = new List<string> { "logs" }, Goods = new List<Stack> { new Stack("logs", 5) } });
    world.TileAt(spaces[0]).Wall = true;
    Check(world.Path(worker.Position, spaces[0], village) == null && spaces.Skip(1).All(p => world.Path(worker.Position, p, village) != null), "Logistics review must leave the early supply inaccessible and both alternatives reachable");
    Check(world.Total(village, "logs") == 15 && world.Total(village, "planks") == 0 && world.Reservations.Count == 0 && village.Jobs.Count == 0, "Logistics review must begin with finite unclaimed logs and no prepared production");
}
static void PrepareOfficerMaterials(LocalAuthority authority)
{
    var world = authority.World; var village = world.Villages.Single();
    Check(village.Cats.Count(c => c.Alive) == 30 && village.Jobs.Count == 0 && village.Officers.Count == 0 && village.Buildings.All(b => b.Slots.Count == 0), "Officer materials review must begin with the idle communal founding");
    foreach (var pile in village.Stockpiles)
    {
        pile.Goods.RemoveAll(s => s.Resource != "food" && s.Resource != "water");
        pile.Report.Clear(); pile.CountedAt = -1;
    }
    var store = village.Stockpiles[0];
    World.Add(store.Goods, "food", 1000 - World.Amount(store.Goods, "food"));
    World.Add(store.Goods, "water", 1000 - World.Amount(store.Goods, "water"));
    // Reuse the duplicate founding workshop's real entrance and road spur.
    var clothier = village.Buildings.Single(b => b.Kind == "woodworking" && b.Position.Equals(new Int2(village.Center.X - 8, village.Center.Z + 3)));
    clothier.Kind = "clothier";
    var stationStore = village.Stockpiles.Single(p => p.ManagedBy == clothier.Id);
    stationStore.Accepts = Catalog.Recipes.Where(r => r.Building == clothier.Kind).SelectMany(r => r.Inputs.Concat(r.Outputs)).Select(s => s.Resource).Distinct().ToList();
    Own(village, "textiles");
    var worker = village.Cats[1];
    worker.Name = "Textile Worker"; worker.Position = clothier.Entrance; worker.X = worker.Position.X; worker.Z = worker.Position.Z; worker.Path.Clear();
    worker.Skills.Clear(); World.Add(worker.Skills, "textile", 25); worker.Preferences.Clear(); worker.Preferences.Add("textile");
    village.Cats[2].Name = "Cloth Leader Candidate";
    var spaces = world.Tiles.Where(t => t.ClaimId == village.Id && !t.Wall && !t.Water && !t.Mountain && !t.Road && !t.Rail && Math.Max(Math.Abs(t.Position.X - village.Center.X), Math.Abs(t.Position.Z - village.Center.Z)) > 2 &&
        !village.Buildings.Any(b => Inside(b.Position, b.Width, b.Depth, t.Position) || b.Entrance.Equals(t.Position)) &&
        !village.Stockpiles.Any(p => Inside(p.Position, p.Width, p.Depth, t.Position)) && world.Path(clothier.Position, t.Position, village) != null)
        .OrderBy(t => Int2.Distance(t.Position, clothier.Position)).ThenBy(t => t.Position.Z).ThenBy(t => t.Position.X).Take(2).Select(t => t.Position).ToArray();
    Check(spaces.Length == 2, "Officer materials review needs two reachable off-road supply spaces");
    village.Stockpiles.Add(new Stockpile { Id = "aaa-officer-materials-blocked-thread", Position = spaces[0], Width = 1, Depth = 1, Capacity = 20, Accepts = new List<string> { "thread" }, Goods = new List<Stack> { new Stack("thread", 5) } });
    village.Stockpiles.Add(new Stockpile { Id = "zzz-officer-materials-reachable-fibre", Position = spaces[1], Width = 1, Depth = 1, Capacity = 20, Accepts = new List<string> { "fibre" }, Goods = new List<Stack> { new Stack("fibre", 5) } });
    world.TileAt(spaces[0]).Wall = true;
    Check(world.Path(worker.Position, spaces[0], village) == null && world.Path(worker.Position, spaces[1], village) != null && world.Path(spaces[1], clothier.Position, village) != null, "Officer materials review must block Thread while retaining both Fibre pickup and delivery routes");
    var availableRecipes = Catalog.Recipes.Where(r => r.Building == "clothier" && Catalog.RecipeAvailable(village, r)).Select(r => r.Id).OrderBy(id => id, StringComparer.Ordinal);
    Check(clothier.Completed && clothier.HasEntrance && village.Research.Contains("textiles") && availableRecipes.SequenceEqual(new[] { "fibre_to_cloth", "fibre_to_thread" }), "Officer materials review must unlock the two basic Clothier recipes without later competing recipes");
    Check(new[] { worker, village.Cats[2] }.All(c => c.Alive && c.AgeHours >= 12 && c.JobId == "" && c.ResumeJobId == "" && c.BuildingId == "" && c.ControlledBy == "" && c.Cargo.Count == 0), "Officer materials review candidates must be free living adults with no cargo");
    Check(world.Total(village, "fibre") == 5 && world.Total(village, "thread") == 5 && world.Total(village, "cloth") == 0 && world.Reservations.Count == 0 && village.Jobs.Count == 0 && village.Officers.Count == 0 && clothier.Slots.Count == 0, "Officer materials review must begin with finite unclaimed inputs and no prepared work or office");
}
static void PrepareWorkshopControls(LocalAuthority authority)
{
    var world = authority.World; var village = world.Villages.Single();
    Check(village.Cats.Count(c => c.Alive) == 30 && village.Jobs.Count == 0 && village.Officers.Count == 0 && village.Buildings.All(b => b.Slots.Count == 0), "Workshop controls review must begin with the idle communal founding");
    foreach (var pile in village.Stockpiles)
    {
        pile.Goods.RemoveAll(s => s.Resource != "food" && s.Resource != "water");
        pile.Report.Clear(); pile.CountedAt = -1;
    }
    var store = village.Stockpiles[0];
    World.Add(store.Goods, "food", 1000 - World.Amount(store.Goods, "food"));
    World.Add(store.Goods, "water", 1000 - World.Amount(store.Goods, "water"));
    store.CountedAt = 0;
    store.Report = new List<Stack> { new Stack("food", 1000), new Stack("water", 1000), new Stack("logs", 3), new Stack("planks", 1) };
    var sawmill = village.Buildings.Single(b => b.Kind == "woodworking" && b.Position.Equals(new Int2(village.Center.X - 8, village.Center.Z + 3)));
    sawmill.Kind = "sawmill";
    var stationStore = village.Stockpiles.Single(p => p.ManagedBy == sawmill.Id);
    stationStore.Accepts = Catalog.Recipes.Where(r => r.Building == sawmill.Kind).SelectMany(r => r.Inputs.Concat(r.Outputs)).Select(s => s.Resource).Distinct().ToList();
    Own(village, "sawmill");
    foreach (var station in village.Buildings.Where(b => World.WorkplaceOfficerRole(b.Kind) == "forester"))
        Apply(authority, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, Edit = "pause", Enabled = true });
    var cutter = village.Buildings.Last(b => b.Kind == "wood_cutter");
    var worker = village.Cats[1];
    worker.Name = "Workshop Worker"; worker.Position = cutter.Entrance; worker.X = worker.Position.X; worker.Z = worker.Position.Z; worker.Path.Clear();
    worker.Skills.Clear(); World.Add(worker.Skills, "process", 25); worker.Preferences.Clear(); worker.Preferences.Add("process");
    village.Cats[2].Name = "Forester Candidate";
    var spaces = world.Tiles.Where(t => t.ClaimId == village.Id && !t.Wall && !t.Water && !t.Mountain && !t.Road && !t.Rail && Math.Max(Math.Abs(t.Position.X - village.Center.X), Math.Abs(t.Position.Z - village.Center.Z)) > 2 &&
        !village.Buildings.Any(b => Inside(b.Position, b.Width, b.Depth, t.Position) || b.Entrance.Equals(t.Position)) &&
        !village.Stockpiles.Any(p => Inside(p.Position, p.Width, p.Depth, t.Position)) && world.Path(cutter.Position, t.Position, village) != null)
        .OrderBy(t => Int2.Distance(t.Position, cutter.Position)).ThenBy(t => t.Position.Z).ThenBy(t => t.Position.X).Take(3).Select(t => t.Position).ToArray();
    Check(spaces.Length == 3, "Workshop controls review needs three reachable off-road supply spaces");
    var supplies = new[] { (Resource: "logs", Amount: 10.0), (Resource: "stone", Amount: 10.0), (Resource: "blocks", Amount: 2.0) };
    for (int i = 0; i < supplies.Length; i++)
        village.Stockpiles.Add(new Stockpile { Id = "workshop-controls-" + supplies[i].Resource, Position = spaces[i], Width = 1, Depth = 1, Capacity = 20, Accepts = new List<string> { supplies[i].Resource }, Goods = new List<Stack> { new Stack(supplies[i].Resource, supplies[i].Amount) } });
    Check(spaces.All(p => world.Path(worker.Position, p, village) != null && world.Path(p, cutter.Position, village) != null), "Workshop controls supplies need physical pickup and delivery routes");
    Check(new[] { worker, village.Cats[2] }.All(c => c.Alive && c.AgeHours >= 12 && c.JobId == "" && c.ResumeJobId == "" && c.BuildingId == "" && c.ControlledBy == "" && c.Cargo.Count == 0), "Workshop controls candidates must be free living adults with no cargo");
    Check(village.Buildings.Single(b => b.Kind == "sawmill").Completed && village.Research.Contains("sawmill") && world.Total(village, "logs") == 10 && world.Total(village, "stone") == 10 && world.Total(village, "blocks") == 2 && world.Total(village, "planks") == 0, "Workshop controls fixture changed finite material or appointment prerequisites");
    Check(world.Reservations.Count == 0 && village.Jobs.Count == 0 && village.Officers.Count == 0 && village.Buildings.All(b => b.Slots.Count == 0 && b.Queue.Count == 0), "Workshop controls review must not inject office, labor, production or claims");
}
static void PrepareEquipmentControls(LocalAuthority authority)
{
    var world = authority.World; var village = world.Villages.Single();
    Check(village.Cats.Count(c => c.Alive) == 30 && village.Jobs.Count == 0 && village.Officers.Count == 0 && village.Items.Count == 0 && village.Buildings.All(b => b.Slots.Count == 0), "Equipment review must start with the idle communal founding");
    foreach (var pile in village.Stockpiles)
    {
        pile.Goods.RemoveAll(s => s.Resource != "food" && s.Resource != "water");
        pile.Report.Clear(); pile.CountedAt = 0;
    }
    var store = village.Stockpiles[0];
    World.Add(store.Goods, "food", 1000 - World.Amount(store.Goods, "food"));
    World.Add(store.Goods, "water", 1000 - World.Amount(store.Goods, "water"));
    var runner = village.Cats[1]; runner.Name = "Equipment Runner"; runner.Position = new Int2(village.Center.X, village.Center.Z + 3); runner.X = runner.Position.X; runner.Z = runner.Position.Z; runner.Path.Clear();
    runner.Hunger = runner.Thirst = runner.Rest = 80; runner.Health = 100;
    var spaces = world.Tiles.Where(t => t.ClaimId == village.Id && !t.Wall && !t.Water && !t.Mountain && !t.Road && !t.Rail &&
        Int2.Distance(t.Position, runner.Position) >= 6 &&
        !village.Buildings.Any(b => Inside(b.Position, b.Width, b.Depth, t.Position) || b.Entrance.Equals(t.Position)) &&
        !village.Stockpiles.Any(p => Inside(p.Position, p.Width, p.Depth, t.Position)) && world.Path(runner.Position, t.Position, village) != null)
        .OrderBy(t => Int2.Distance(t.Position, runner.Position)).ThenBy(t => t.Position.Z).ThenBy(t => t.Position.X).Take(2).Select(t => t.Position).ToArray();
    Check(spaces.Length == 2, "Equipment review needs two separate known off-road source spaces");
    var reachable = new Stockpile { Id = "equipment-review-reachable-store", Position = spaces[0], Width = 1, Depth = 1, Capacity = 10, Accepts = new List<string> { "tools", "weapons", "armor" }, CountedAt = 0 };
    var blocked = new Stockpile { Id = "equipment-review-blocked-store", Position = spaces[1], Width = 1, Depth = 1, Capacity = 10, Accepts = new List<string> { "weapons" }, CountedAt = 0 };
    village.Stockpiles.Add(reachable); village.Stockpiles.Add(blocked);
    foreach (var data in new[] { (Id: "equipment-review-tool", Kind: "tool", Material: "wood", Quality: 3, Condition: 17.0, Maximum: 42.0), (Id: "equipment-review-weapon", Kind: "weapon", Material: "iron", Quality: 2, Condition: 31.0, Maximum: 60.0), (Id: "equipment-review-armor", Kind: "armor", Material: "leather", Quality: 4, Condition: 23.0, Maximum: 50.0) })
        village.Items.Add(new Item { Id = data.Id, Kind = data.Kind, Material = data.Material, VillageId = village.Id, LocationId = reachable.Id, Quality = data.Quality, Condition = data.Condition, MaxCondition = data.Maximum });
    village.Items.Add(new Item { Id = "equipment-review-blocked-weapon", Kind = "weapon", Material = "stone", VillageId = village.Id, LocationId = blocked.Id, Quality = 1, Condition = 9, MaxCondition = 20 });
    world.TileAt(blocked.Position).Wall = true;
    foreach (var pile in village.Stockpiles)
    {
        pile.Report = pile.Goods.Select(s => new Stack(s.Resource, s.Amount)).ToList();
        foreach (var item in village.Items.Where(i => i.LocationId == pile.Id)) World.Add(pile.Report, item.Kind == "armor" ? "armor" : item.Kind + "s", 1);
    }
    Check(world.Walkable(village, runner.Position) && world.Path(runner.Position, reachable.Position, village) != null && world.Path(runner.Position, blocked.Position, village) == null, "Equipment review needs an open retrieval route and a separate blocked source");
    Check(runner.Alive && runner.AgeHours >= 12 && runner.JobId == "" && runner.ResumeJobId == "" && runner.BuildingId == "" && runner.ControlledBy == "" && runner.Cargo.Count == 0 && runner.Equipment.Count == 0, "Equipment Runner must be a genuinely free adult without cargo or worn equipment");
    Check(village.Items.Count == 4 && village.Cats.All(c => c.Equipment.Count == 0) && village.Jobs.Count == 0 && world.Reservations.Count == 0 && village.Officers.Count == 0 && village.Buildings.All(b => b.Slots.Count == 0 && b.Queue.Count == 0), "Equipment review must not inject jobs, claims, officers, staffing or equipped items");
}
static void PrepareOpenSlotControls(LocalAuthority authority)
{
    PrepareEquipmentControls(authority);
    var world = authority.World; var village = world.Villages.Single();
    var clothier = village.Buildings.Single(b => b.Kind == "woodworking" && b.Position.Equals(new Int2(village.Center.X - 8, village.Center.Z + 3)));
    clothier.Kind = "clothier";
    var stationStore = village.Stockpiles.Single(p => p.ManagedBy == clothier.Id);
    stationStore.Accepts = Catalog.Recipes.Where(r => r.Building == clothier.Kind).SelectMany(r => r.Inputs.Concat(r.Outputs)).Select(s => s.Resource).Distinct().ToList();
    void Buy(string id)
    {
        if (village.Research.Contains(id)) return;
        foreach (var parent in Catalog.Study(id).Prerequisites) Buy(parent);
        Apply(authority, new GameAction { Kind = "ResearchNode", NodeId = id });
    }
    Buy("clothier_crews");
    village.Cats[2].Name = "Cloth Leader Candidate";
    var textileWorker = village.Cats[3]; textileWorker.Name = "Textile Worker";
    textileWorker.Position = clothier.Entrance; textileWorker.X = textileWorker.Position.X; textileWorker.Z = textileWorker.Position.Z; textileWorker.Path.Clear();
    textileWorker.Skills.Clear(); World.Add(textileWorker.Skills, "textile", 40); textileWorker.Preferences.Clear(); textileWorker.Preferences.Add("textile");
    var source = world.Tiles.Where(t => t.ClaimId == village.Id && !t.Wall && !t.Water && !t.Mountain && !t.Road && !t.Rail &&
        !village.Buildings.Any(b => Inside(b.Position, b.Width, b.Depth, t.Position) || b.Entrance.Equals(t.Position)) &&
        !village.Stockpiles.Any(p => Inside(p.Position, p.Width, p.Depth, t.Position)) && world.Path(clothier.Position, t.Position, village) != null)
        .OrderBy(t => Int2.Distance(t.Position, clothier.Position)).ThenBy(t => t.Position.Z).ThenBy(t => t.Position.X).Select(t => (Int2?)t.Position).FirstOrDefault();
    Check(source.HasValue, "Open-position review needs a reachable off-road finite Fibre source");
    village.Stockpiles.Add(new Stockpile { Id = "open-slot-controls-fibre", Position = source.Value, Width = 1, Depth = 1, Capacity = 20, Accepts = new List<string> { "fibre" }, Goods = new List<Stack> { new Stack("fibre", 5) }, Report = new List<Stack> { new Stack("fibre", 5) }, CountedAt = 0 });
    var first = village.Cats[6]; var second = village.Cats[7];
    Apply(authority, new GameAction { Kind = "AssignWorker", BuildingId = clothier.Id, CatId = first.Id });
    Apply(authority, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = clothier.Id, CatId = first.Id, Edit = "add", RecipeId = "fibre_to_thread", Repeat = true });
    Apply(authority, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = clothier.Id, CatId = first.Id, Edit = "pause", Enabled = true });
    Apply(authority, new GameAction { Kind = "AssignWorker", BuildingId = clothier.Id, CatId = second.Id });
    foreach (string recipe in new[] { "fibre_to_thread", "fibre_to_cloth" })
        Apply(authority, new GameAction { Kind = "EditProductionWorkSlot", BuildingId = clothier.Id, CatId = second.Id, Edit = "add", RecipeId = recipe });
    Apply(authority, new GameAction { Kind = "AssignWorker", CatId = first.Id });
    Apply(authority, new GameAction { Kind = "AssignWorker", CatId = second.Id });
    ValidateOpenSlotControls(world, village);
}
static void ValidateOpenSlotControls(World world, Village village)
{
    AuthorityRuntime.ValidateWorld(world);
    var clothier = village.Buildings.Single(b => b.Kind == "clothier");
    Check(Catalog.BuildingEffect(village, "clothier", "worker_slots", 1) == 2 && clothier.Slots.Count == 2 && !clothier.Paused && clothier.Slots.All(s => s.CatId == "" && s.JobId == "" && s.AutomatedBy == ""), "Open-position fixture must have two exact empty manual positions within researched capacity");
    Check(clothier.Slots[0].Paused && clothier.Slots[0].Queue.Count == 1 && clothier.Slots[0].Queue[0].RecipeId == "fibre_to_thread" && clothier.Slots[0].Queue[0].Repeat && clothier.Slots[0].Queue[0].AutomatedBy == "", "First empty position must retain its deliberate paused repeating manual order");
    Check(!clothier.Slots[1].Paused && clothier.Slots[1].Queue.Select(q => q.RecipeId).SequenceEqual(new[] { "fibre_to_thread", "fibre_to_cloth" }) && clothier.Slots[1].Queue.All(q => !q.Repeat && q.AutomatedBy == ""), "Later empty position must retain the exact finite manual chain");
    Check(village.Buildings.Where(b => b.Id != clothier.Id).All(b => b.Slots.Count == 0 && b.Queue.Count == 0) && village.Buildings.All(b => b.Inputs.Count == 0 && b.Outputs.Count == 0), "Open-position fixture must not prepare unrelated production or local goods");
    Check(world.Total(village, "food") == 1000 && world.Total(village, "water") == 1000 && world.Total(village, "fibre") == 5 && world.Total(village, "thread") == 0 && world.Total(village, "cloth") == 0 && village.Stockpiles.SelectMany(p => p.Goods).All(s => new[] { "food", "water", "fibre" }.Contains(s.Resource)), "Open-position fixture must retain only its documented finite provisions and Fibre");
    Check(village.Jobs.Count == 0 && world.Reservations.Count == 0 && village.Officers.Count == 0 && village.Items.Count == 4 && village.Items.All(i => i.LocationId.StartsWith("equipment-review-", StringComparison.Ordinal)) && village.Cats.All(c => c.JobId == "" && c.ResumeJobId == "" && c.BuildingId == "" && c.ControlledBy == "" && c.Equipment.Count == 0 && c.Cargo.Count == 0), "Open-position review must not inject active jobs, item claims, worn gear, cargo, officers or staffing");
    Check(world.Path(clothier.Position, village.Stockpiles.Single(p => p.Id == "open-slot-controls-fibre").Position, village) != null, "Open-position finite Fibre must be reachable through the ordinary entrance");
}
static void PrepareChainControls(LocalAuthority authority)
{
    var world = authority.World; var village = world.Villages.Single();
    Check(village.Jobs.Count == 0 && village.Officers.Count == 0 && village.Items.Count == 0 && village.Buildings.All(b => b.Slots.Count == 0), "Finite chain review must start with idle founding workplaces");
    foreach (var pile in village.Stockpiles)
    {
        pile.Goods.RemoveAll(s => s.Resource != "food" && s.Resource != "water");
        pile.Report.Clear(); pile.CountedAt = 0;
    }
    var store = village.Stockpiles[0];
    World.Add(store.Goods, "food", 1000 - World.Amount(store.Goods, "food"));
    World.Add(store.Goods, "water", 1000 - World.Amount(store.Goods, "water"));
    var stations = new[] { village.Buildings.Last(b => b.Kind == "wood_cutter"), village.Buildings.First(b => b.Kind == "stone_prep"), village.Buildings.First(b => b.Kind == "woodworking") };
    var workers = new[] { (Name: "Plank Worker", Labor: "process"), (Name: "Block Worker", Labor: "process"), (Name: "Tool Worker", Labor: "craft") };
    for (int i = 0; i < workers.Length; i++)
    {
        var cat = village.Cats[i + 1]; cat.Name = workers[i].Name;
        cat.Position = stations[i].Entrance; cat.X = cat.Position.X; cat.Z = cat.Position.Z; cat.Path.Clear();
        cat.Skills.Clear(); World.Add(cat.Skills, workers[i].Labor, 40); cat.Preferences.Clear(); cat.Preferences.Add(workers[i].Labor);
    }
    var spaces = world.Tiles.Where(t => t.ClaimId == village.Id && !t.Wall && !t.Water && !t.Mountain && !t.Road && !t.Rail &&
        !village.Buildings.Any(b => Inside(b.Position, b.Width, b.Depth, t.Position) || b.Entrance.Equals(t.Position)) &&
        !village.Stockpiles.Any(p => Inside(p.Position, p.Width, p.Depth, t.Position)) && stations.All(b => world.Path(b.Position, t.Position, village) != null))
        .OrderBy(t => Int2.Distance(t.Position, village.Center)).ThenBy(t => t.Position.Z).ThenBy(t => t.Position.X).Take(2).Select(t => t.Position).ToArray();
    Check(spaces.Length == 2, "Finite chain review requires two reachable off-road raw-material piles");
    for (int i = 0; i < spaces.Length; i++)
    {
        string resource = i == 0 ? "logs" : "stone";
        village.Stockpiles.Add(new Stockpile { Id = "chain-controls-" + resource, Position = spaces[i], Width = 1, Depth = 1, Capacity = 20, Accepts = new List<string> { resource }, Goods = new List<Stack> { new Stack(resource, 10) }, CountedAt = 0 });
    }
    foreach (var pile in village.Stockpiles) pile.Report = pile.Goods.Select(s => new Stack(s.Resource, s.Amount)).ToList();
    ValidateChainControls(world, village);
}
static void ValidateChainControls(World world, Village village)
{
    AuthorityRuntime.ValidateWorld(world);
    foreach (string id in new[] { "logs_to_planks", "stone_to_blocks", "planks_and_blocks_to_tools" })
    {
        var recipe = Catalog.Recipe(id);
        Check(recipe != null && recipe.Founding && Catalog.RecipeAvailable(village, recipe) && recipe.Inputs.All(s => world.RecipeInput(village, recipe, s.Amount) == s.Amount) && recipe.Outputs.All(s => world.RecipeOutput(village, recipe, s.Amount) == s.Amount) && world.RecipeDuration(village, recipe) == 600, "Finite chain recipe must retain its unmodified founding bill and work: " + id);
    }
    Check(world.Total(village, "logs") == 10 && world.Total(village, "stone") == 10 && world.Total(village, "planks") == 0 && world.Total(village, "blocks") == 0 && world.Total(village, "tools") == 0 && world.Total(village, "food") == 1000 && world.Total(village, "water") == 1000, "Finite chain fixture changed its exact starting goods");
    Check(village.Stockpiles.SelectMany(p => p.Goods).All(s => new[] { "food", "water", "logs", "stone" }.Contains(s.Resource)), "Finite chain fixture contains undocumented materials");
    Check(village.Jobs.Count == 0 && village.Items.Count == 0 && village.Officers.Count == 0 && world.Reservations.Count == 0 && village.Buildings.All(b => b.Slots.Count == 0 && b.Queue.Count == 0 && b.Inputs.Count == 0 && b.Outputs.Count == 0) && village.Cats.All(c => c.JobId == "" && c.ResumeJobId == "" && c.BuildingId == "" && c.ControlledBy == "" && c.Cargo.Count == 0 && c.Equipment.Count == 0), "Finite chain fixture must not inject staffing, queues, officers, active jobs or ownership");
    foreach (var cat in village.Cats.Where(c => new[] { "Plank Worker", "Block Worker", "Tool Worker" }.Contains(c.Name)))
        Check(cat.Alive && cat.AgeHours >= 12 && world.Walkable(village, cat.Position) && village.Stockpiles.Where(p => p.Id.StartsWith("chain-controls-", StringComparison.Ordinal)).All(p => world.Path(cat.Position, p.Position, village) != null), "Named finite chain worker needs real physical supply routes");
}
static void PrepareDefense(LocalAuthority authority)
{
    var world = authority.World; var village = world.Villages.Single();
    Check(!village.Officers.Any(o => o.Role == "captain"), "Defense review must begin with a vacant Captain office");
    foreach (var cat in village.Cats) { cat.Skills.RemoveAll(s => s.Resource == "fight"); cat.Preferences.Remove("fight"); }
    var roles = new[]
    {
        (Index: 20, Name: "North Defender", Position: new Int2(0, 18), Fight: 2.0),
        (Index: 21, Name: "South Defender", Position: new Int2(0, -18), Fight: 3.0),
        (Index: 22, Name: "Captain Candidate", Position: new Int2(0, 4), Fight: 0.0),
        (Index: 23, Name: "Training Candidate", Position: new Int2(0, 5), Fight: 0.0)
    };
    foreach (var role in roles)
    {
        var cat = village.Cats[role.Index];
        Apply(authority, new GameAction { Kind = "AssignWorker", CatId = cat.Id });
        Check(cat.JobId == "" && cat.ResumeJobId == "" && cat.BuildingId == "" && cat.Cargo.Count == 0, "Defense reviewer cat must be genuinely free");
        cat.Name = role.Name; cat.Position = role.Position; cat.X = role.Position.X; cat.Z = role.Position.Z; cat.Path.Clear();
        cat.Hunger = cat.Thirst = cat.Rest = 80; cat.Health = 100;
        if (role.Fight > 0) World.Add(cat.Skills, "fight", role.Fight);
        if (role.Name == "Training Candidate") { cat.Preferences.Clear(); cat.Preferences.Add("fight"); }
        Check(world.Walkable(village, cat.Position), "Defense reviewer cat must stand on a real walkable road");
    }
    foreach (var position in new[] { new Int2(0, 21), new Int2(0, -21) })
    {
        Check(world.Path(position, village.Center, village) != null, "Defense review raid must have a physical shrine route");
        village.Raids.Add(new Raid { Id = world.Id(position.Z > 0 ? "north-review-raid" : "south-review-raid"), Position = position, Health = 1 });
    }
    var barracks = village.Buildings.Single(b => b.Kind == "barracks");
    Check(barracks.Completed && !barracks.Paused && world.Path(new Int2(0, 5), barracks.Position, village) != null, "Captain preparation needs a reachable unpaused Barracks");
    Check(world.CaptainPreparationStatus(village).Target == 10 && world.CaptainPreparationStatus(village).Trained == 1, "Defense readiness fixture must leave a real training shortfall");
}
static void Populate(LocalAuthority authority, int count)
{
    var w = authority.World; var v = w.Villages[0]; v.Radius = 23; v.LayoutVersion = 1; v.Buildings.Clear(); v.Stockpiles.Clear(); v.Cats.Clear(); v.Jobs.Clear(); v.Farms.Clear(); v.Items.Clear(); v.Officers.Clear(); v.Known.Clear(); v.ClaimedTiles.Clear(); v.Routes.Clear(); v.Vehicles.Clear(); v.Raids.Clear(); v.BoundaryEdges.Clear(); w.Reservations.Clear();
    w.Dungeons.Clear(); w.Creatures.Clear(); w.Tiles.Clear(); w.SurfacePlateaus.Clear();
    w.PrepareFoundingTerrain(v);
    for (int z = -27; z <= 27; z++) for (int x = -27; x <= 27; x++)
    {
        var p = new Int2(x, z); var t = w.TileAt(p); t.Wall = t.Water = t.Mountain = false; t.Road = t.Rail = t.Dock = t.Bridge = t.Dirt = false; t.Overlay = ""; t.PathWear = 0; t.Resource = ""; t.Amount = t.FishCapacity = 0; t.Deposits.Clear(); t.ClaimId = Math.Abs(x) <= v.Radius && Math.Abs(z) <= v.Radius ? v.Id : ""; t.Biome = "meadow"; v.Known.Add(p);
        World.ClearSurface(t, 0);
        if (t.ClaimId == v.Id)
        {
            v.ClaimedTiles.Add(p);
            t.Wall = (Math.Abs(x) == v.Radius || Math.Abs(z) == v.Radius) && !(x == 0 || z == 0);
            t.Road = (x == 0 || z == 0) && Math.Max(Math.Abs(x), Math.Abs(z)) >= 2 || Math.Max(Math.Abs(x), Math.Abs(z)) == 2;
        }
        else if (Math.Max(Math.Abs(x), Math.Abs(z)) == v.Radius + 1) { t.Dirt = true; t.Road = x == 0 || z == 0; }
    }
    foreach (var data in new[] { ("water", 2, 25), ("food", 25, 2), ("logs", -25, 2), ("stone", -25, -2), ("fibre", 25, -2), ("ore", -25, 6), ("clay", 25, 6), ("sand", 25, 10), ("gem", -25, 10) })
    { var tile = w.TileAt(new Int2(data.Item2, data.Item3)); tile.Resource = data.Item1 == "water" ? "fish" : data.Item1; tile.Amount = data.Item1 == "water" ? 24 : 2000; if (data.Item1 == "water") World.SetWaterSurface(tile, 0, .35); }
    v.Buildings.Add(new Building { Id = w.Id("shrine"), Kind = "shrine", Position = new Int2(v.Center.X - 1, v.Center.Z - 1), Width = 3, Depth = 3, Completed = true, ConstructionConsumed = true });
    var store = new Stockpile { Id = w.Id("review-store"), Position = new Int2(3, 3), Width = 4, Depth = 4, Capacity = 40000 }; v.Stockpiles.Add(store);
    var sites = new Queue<Int2>();
    for (int z = -20; z <= 18; z += 4) for (int x = -20; x <= 18; x += 4)
        if (x != 0 && z != 0 && !Footprint(new Int2(x, z), 2, 2).Any(p => Inside(store.Position, store.Width, store.Depth, p))) sites.Enqueue(new Int2(x, z));
    for (int i = 0; i < count / 5; i++) AddBuilding(w, v, "den", sites.Dequeue());
    foreach (string kind in Catalog.Buildings.Where(kind => kind != "den" && kind != "shrine")) AddBuilding(w, v, kind, kind == "field" ? new Int2(24, 14) : sites.Dequeue());
    foreach (var building in v.Buildings.Where(b => b.Kind != "shrine")) ConnectBuilding(w, v, building);
    ValidateLayout(w, v);
    foreach (var resource in Catalog.Resources.Where(r => r != "blessings" && r != "weapons" && r != "armor" && r != "tools")) World.Add(store.Goods, resource, resource == "food" || resource == "water" ? 5000 : 400);
    var dens = v.Buildings.Where(b => b.Kind == "den").ToArray();
    var catPositions = w.Tiles.Where(t => t.ClaimId == v.Id && !t.Wall && !v.Buildings.Any(b => Inside(b.Position, b.Width, b.Depth, t.Position)) && !Inside(store.Position, store.Width, store.Depth, t.Position)).OrderBy(t => Int2.Distance(t.Position, v.Center)).ThenBy(t => t.Position.Z).ThenBy(t => t.Position.X).Take(count).Select(t => t.Position).ToArray();
    for (int i = 0; i < count; i++)
    {
        var p = catPositions[i]; var c = new Cat { Id = w.Id("cat"), Name = "Review Cat " + (i + 1), VillageId = v.Id, Position = p, X = p.X, Z = p.Z, BedId = dens[i / 5].Id, AgeHours = 24, Hunger = 80, Thirst = 80, Rest = 80, Health = 100 };
        c.Skills.Add(new Stack(Catalog.Labors[i % Catalog.Labors.Length], 20)); c.Preferences.Add(Catalog.Labors[i % Catalog.Labors.Length]); v.Cats.Add(c);
    }
    v.LeaderId = v.Cats[0].Id; v.ResearchPoints = 20000; v.LastLeaderResearch = w.TimeSeconds; v.Blessings = 200; v.Coins = 1000; v.Research.Clear();
    var chains = new Dictionary<string, string[]> { { "wood_cutter", new[] { "logs_to_planks" } }, { "stone_prep", new[] { "stone_to_blocks" } }, { "woodworking", new[] { "planks_and_blocks_to_tools" } }, { "clothier", new[] { "fibre_to_thread", "fibre_to_cloth" } }, { "mill", new[] { "grain_to_flour", "flour_to_food" } }, { "smelter", new[] { "ore_to_metal" } }, { "smithy", new[] { "smithy_weapon" } }, { "tannery", new[] { "hide_to_leather" } }, { "sawmill", new[] { "logs_to_lumber" } }, { "workshop", new[] { "materials_to_refined" } } };
    foreach (var recipeId in chains.Values.SelectMany(x => x)) foreach (var study in Catalog.Research.Where(n => n.Payloads.Any(p => p.Kind == "unlock_recipe" && p.Id == recipeId))) Own(v, study.Id);
    foreach (string node in new[] { "basic_tools", "sawmill", "irrigation", "barracks", "research_hut", "textiles" }) Own(v, node);
    int worker = 1; foreach (var chain in chains)
    { var station = v.Buildings.First(b => b.Kind == chain.Key); var cat = v.Cats[worker++]; Apply(authority, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = cat.Id }); foreach (var recipe in chain.Value) Apply(authority, new GameAction { Kind = "EditProductionQueue", BuildingId = station.Id, RecipeId = recipe, Edit = "add", Repeat = true }); }
    foreach (string stationKind in new[] { "research_hut", "school", "accounting_tent" }) { var station = v.Buildings.First(b => b.Kind == stationKind); Apply(authority, new GameAction { Kind = "AssignWorker", BuildingId = station.Id, CatId = v.Cats[worker++].Id }); }
    Apply(authority, new GameAction { Kind = "AssignOfficer", Role = "accountant", CatId = v.Cats[worker - 1].Id });
    Check(Catalog.Research.Any(n => !v.Research.Contains(n.Id) && n.Prerequisites.All(v.Research.Contains) && n.Cost <= v.ResearchPoints), "Fixture must leave a normal research purchase available");
    w.EnsureVillageDungeon(v);
    authority.Advance(15);
}
static void AddBuilding(World w, Village v, string kind, Int2 position)
{ v.Buildings.Add(new Building { Id = w.Id(kind), Kind = kind, Position = position, Completed = true, ConstructionConsumed = true }); }
static IEnumerable<Int2> Footprint(Int2 origin, int width, int depth)
{ for (int z = 0; z < depth; z++) for (int x = 0; x < width; x++) yield return new Int2(origin.X + x, origin.Z + z); }
static IEnumerable<Int2> Neighbors(Int2 p)
{ yield return new Int2(p.X - 1, p.Z); yield return new Int2(p.X, p.Z - 1); yield return new Int2(p.X + 1, p.Z); yield return new Int2(p.X, p.Z + 1); }
static IEnumerable<Int2> Entrances(Building building)
{
    for (int x = 0; x < building.Width; x++) { yield return new Int2(building.Position.X + x, building.Position.Z - 1); yield return new Int2(building.Position.X + x, building.Position.Z + building.Depth); }
    for (int z = 0; z < building.Depth; z++) { yield return new Int2(building.Position.X - 1, building.Position.Z + z); yield return new Int2(building.Position.X + building.Width, building.Position.Z + z); }
}
static bool RoadSpace(Village village, Tile tile) => !tile.Wall && !tile.Water && !tile.Mountain &&
    !village.Buildings.Any(b => Inside(b.Position, b.Width, b.Depth, tile.Position)) &&
    !village.Stockpiles.Any(p => Inside(p.Position, p.Width, p.Depth, tile.Position)) &&
    !village.Farms.Any(f => Inside(f.Position, f.Width, f.Depth, tile.Position));
static HashSet<Int2> ConnectedRoads(World world, Village village)
{
    var tiles = world.Tiles.ToDictionary(t => t.Position);
    var shrine = village.Buildings.Single(b => b.Kind == "shrine");
    var pending = new Queue<Int2>(Footprint(shrine.Position, shrine.Width, shrine.Depth));
    var connected = new HashSet<Int2>();
    while (pending.Count > 0)
        foreach (var at in Neighbors(pending.Dequeue()))
            if (tiles.TryGetValue(at, out var tile) && (tile.Road || tile.Overlay == "road_built") && RoadSpace(village, tile) && connected.Add(at)) pending.Enqueue(at);
    return connected;
}
// Synthetic authoring only: carve a shortest road spur around every physical footprint.
// Runtime placement still uses the public material-consuming construction actions.
static void ConnectBuilding(World world, Village village, Building building)
{
    var tiles = world.Tiles.ToDictionary(t => t.Position); var known = village.Known.ToHashSet();
    var roads = ConnectedRoads(world, village); var parents = new Dictionary<Int2, Int2>(); var pending = new Queue<Int2>();
    foreach (var at in Entrances(building).OrderBy(p => Int2.Distance(p, village.Center)).ThenBy(p => p.Z).ThenBy(p => p.X))
        if (known.Contains(at) && tiles.TryGetValue(at, out var tile) && RoadSpace(village, tile)) { parents[at] = at; pending.Enqueue(at); }
    while (pending.Count > 0)
    {
        var at = pending.Dequeue();
        if (roads.Contains(at))
        {
            while (true)
            {
                tiles[at].Road = true; tiles[at].Dirt = false;
                if (parents[at].Equals(at)) { building.Entrance = at; building.HasEntrance = true; return; }
                at = parents[at];
            }
        }
        foreach (var next in Neighbors(at))
            if (!parents.ContainsKey(next) && known.Contains(next) && tiles.TryGetValue(next, out var tile) && RoadSpace(village, tile)) { parents[next] = at; pending.Enqueue(next); }
    }
    throw new InvalidDataException("Fixture building cannot reach shrine roads: " + building.Kind);
}
static void ValidateLayout(World world, Village village)
{
    var shrine = village.Buildings.Single(b => b.Kind == "shrine"); var roads = ConnectedRoads(world, village);
    Check(village.LayoutVersion == 1 && shrine.Width == 3 && shrine.Depth == 3 && shrine.Position.Equals(new Int2(village.Center.X - 1, village.Center.Z - 1)), "Fixture needs the centered 3 × 3 shrine");
    var occupied = new Dictionary<Int2, string>();
    foreach (var building in village.Buildings)
        foreach (var at in Footprint(building.Position, building.Width, building.Depth)) Check(occupied.TryAdd(at, building.Id), "Fixture buildings overlap");
    foreach (var pile in village.Stockpiles)
        foreach (var at in Footprint(pile.Position, pile.Width, pile.Depth))
            Check(occupied.TryAdd(at, pile.Id) || occupied[at] == pile.ManagedBy, "Fixture stockpile overlaps an unrelated footprint");
    for (int z = -2; z <= 2; z++) for (int x = -2; x <= 2; x++)
        if (Math.Max(Math.Abs(x), Math.Abs(z)) == 2) Check(roads.Contains(new Int2(village.Center.X + x, village.Center.Z + z)), "The full shrine ring must remain clear and connected");
    foreach (var building in village.Buildings.Where(b => b.Kind != "shrine"))
    {
        Check(building.HasEntrance && Entrances(building).Contains(building.Entrance) && roads.Contains(building.Entrance), "Every fixture building needs an adjacent shrine-connected entrance");
        var route = world.Path(village.Center, building.Position, village);
        Check(route != null && route.Contains(building.Entrance), "A cat cannot reach the workplace through its entrance");
    }
    int openings = 0;
    for (int z = -village.Radius; z <= village.Radius; z++) for (int x = -village.Radius; x <= village.Radius; x++)
    {
        if (Math.Max(Math.Abs(x), Math.Abs(z)) != village.Radius) continue;
        var at = new Int2(village.Center.X + x, village.Center.Z + z); var tile = world.TileAt(at);
        if (tile.Wall) continue;
        openings++; Check((x == 0 || z == 0) && roads.Contains(at), "An enclosure opening is not a connected cardinal gate");
        var outside = new Int2(at.X + Math.Sign(x), at.Z + Math.Sign(z));
        Check(world.Path(village.Center, outside, village) != null, "A gate has no walkable exterior exit");
    }
    Check(openings == 4, "Fixture must have exactly four gates and a closed perimeter");
}
static bool Inside(Int2 origin, int width, int depth, Int2 tile) => tile.X >= origin.X && tile.X < origin.X + width && tile.Z >= origin.Z && tile.Z < origin.Z + depth;
static void Own(Village v, string id)
{ if (v.Research.Contains(id)) return; var node = Catalog.Study(id); if (node == null) throw new InvalidDataException("Fixture study missing"); foreach (var parent in node.Prerequisites) Own(v, parent); v.Research.Add(id); }
