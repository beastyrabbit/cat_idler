using IdleCatForest.Authority;
using IdleCatForest.Simulation;
using Stack = IdleCatForest.Simulation.Stack;

if (args.Length != 2 || !int.TryParse(args[1], out int population) || population != 30 && population != 150)
{ Console.Error.WriteLine("Usage: Forest.PresentationFixture <new-absolute-save-path> <30|150>"); return 2; }
string target = Path.GetFullPath(args[0]);
if (!Path.IsPathRooted(args[0]) || File.Exists(target) || Directory.Exists(target + ".identity"))
{ Console.Error.WriteLine("Choose a new absolute save path with no existing save or identity directory."); return 2; }
try
{
    using var authority = LocalAuthority.CreateNew(target, 41);
    if (population == 150) Populate(authority, population);
    else PrepareFounding(authority);
    var world = authority.World; var village = world.Villages.Single(); var failures = world.Validate();
    if (failures.Count > 0) throw new InvalidDataException(string.Join("; ", failures));
    Check(village.Cats.Where(c => c.Alive).All(c => village.Buildings.Any(b => b.Id == c.BedId && b.Kind == "den" && b.Completed)), "Every living cat needs a real completed Den");
    Check(village.Cats.Select(c => c.Id).Distinct().Count() == population, "Cat identities must be unique");
    authority.Save();
    var restored = SaveStore.Load<World>(target); Check(restored.Villages[0].Cats.Count(c => c.Alive) == population, "Saved population changed");
    Console.WriteLine("Created isolated " + population + "-cat fixture; dens=" + village.Buildings.Count(b => b.Kind == "den") + " buildings=" + village.Buildings.Count + " staffed=" + village.Buildings.Sum(b => b.Slots.Count) + " recipes=" + village.Buildings.Sum(b => b.Slots.Sum(s => s.Queue.Count)) + " research_points=" + village.ResearchPoints + " validation=passed");
    return 0;
}
catch (Exception error) { Console.Error.WriteLine("Fixture creation failed: " + error.GetType().Name + ". Destination was preserved for inspection."); return 1; }

static void Check(bool condition, string reason) { if (!condition) throw new InvalidDataException(reason); }
static void Apply(LocalAuthority authority, GameAction action)
{ var result = authority.Apply(action); Check(result.Success, action.Kind + " rejected: " + result.Error); }
static void PrepareFounding(LocalAuthority authority)
{
    var v = authority.World.Villages[0];
    // Finite review supplies and research currency allow the normal UI to demonstrate actions.
    v.ResearchPoints = 20000; v.LastLeaderResearch = authority.World.TimeSeconds; v.Blessings = 200;
    foreach (var resource in new[] { "planks", "blocks", "logs", "stone", "fibre", "ore", "materials" }) World.Add(v.Stockpiles[0].Goods, resource, 100);
}
static void Populate(LocalAuthority authority, int count)
{
    var w = authority.World; var v = w.Villages[0]; v.Radius = 23; v.Buildings.Clear(); v.Stockpiles.Clear(); v.Cats.Clear(); v.Jobs.Clear(); v.Farms.Clear(); v.Items.Clear(); v.Officers.Clear(); v.Known.Clear(); v.ClaimedTiles.Clear(); v.Routes.Clear(); v.Vehicles.Clear(); v.Raids.Clear(); w.Reservations.Clear();
    for (int z = -27; z <= 27; z++) for (int x = -27; x <= 27; x++)
    {
        var p = new Int2(x, z); var t = w.TileAt(p); t.Wall = t.Water = t.Mountain = false; t.Road = t.Rail = t.Dock = t.Bridge = false; t.Resource = ""; t.Amount = 0; t.Deposits.Clear(); t.ClaimId = Math.Abs(x) <= v.Radius && Math.Abs(z) <= v.Radius ? v.Id : ""; t.Biome = "meadow"; v.Known.Add(p);
        if (t.ClaimId == v.Id) { v.ClaimedTiles.Add(p); t.Wall = (Math.Abs(x) == v.Radius || Math.Abs(z) == v.Radius) && !(x == 0 && z == v.Radius); t.Road = x == 0 || z == 0; }
    }
    foreach (var data in new[] { ("water", 2, 25), ("food", 25, 2), ("logs", -25, 2), ("stone", -25, -2), ("fibre", 25, -2), ("ore", -25, 6), ("clay", 25, 6), ("sand", 25, 10), ("gem", -25, 10) })
    { var tile = w.TileAt(new Int2(data.Item2, data.Item3)); tile.Resource = data.Item1 == "water" ? "fish" : data.Item1; tile.Amount = data.Item1 == "water" ? 24 : 2000; tile.Water = data.Item1 == "water"; tile.FishCapacity = tile.Water ? 24 : 0; }
    v.Buildings.Add(new Building { Id = w.Id("shrine"), Kind = "shrine", Position = v.Center, Completed = true, ConstructionConsumed = true });
    var sites = new Queue<Int2>();
    for (int z = -20; z <= 18; z += 4) for (int x = -20; x <= 18; x += 4) if (x != 0 && z != 0) sites.Enqueue(new Int2(x, z));
    for (int i = 0; i < count / 5; i++) AddBuilding(w, v, "den", sites.Dequeue());
    foreach (string kind in Catalog.Buildings.Where(kind => kind != "den" && kind != "shrine")) AddBuilding(w, v, kind, kind == "field" ? new Int2(24, 14) : sites.Dequeue());
    var store = new Stockpile { Id = w.Id("review-store"), Position = new Int2(2, 2), Width = 4, Depth = 4, Capacity = 40000 }; v.Stockpiles.Add(store);
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
    authority.Advance(15);
}
static void AddBuilding(World w, Village v, string kind, Int2 position)
{ v.Buildings.Add(new Building { Id = w.Id(kind), Kind = kind, Position = position, Completed = true, ConstructionConsumed = true }); }
static bool Inside(Int2 origin, int width, int depth, Int2 tile) => tile.X >= origin.X && tile.X < origin.X + width && tile.Z >= origin.Z && tile.Z < origin.Z + depth;
static void Own(Village v, string id)
{ if (v.Research.Contains(id)) return; var node = Catalog.Study(id); if (node == null) throw new InvalidDataException("Fixture study missing"); foreach (var parent in node.Prerequisites) Own(v, parent); v.Research.Add(id); }
