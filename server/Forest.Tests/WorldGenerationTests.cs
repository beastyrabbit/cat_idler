using IdleCatForest.Authority;
using IdleCatForest.Simulation;

static class WorldGenerationTests
{
    public static void Run(Action<string, Action> test, Action<bool, string> check)
    {
        test("world generation authority saves heights profiles and layered frontier", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, "world.json");
            var sample = new Int2(-88, 73);
            var frontier = new Int2(-173, -149);
            string expected, expectedFrontier;
            int count;
            using (var runtime = new AuthorityRuntime(path, 41))
            {
                var world = runtime.World;
                check(world.GenerationVersion == 1 && world.SurfacePlateaus.Count > 0 && world.Dungeons.Count > 0, "new authority omitted generation version, plateau or dungeon");
                world.TileAt(sample);
                foreach (var floor in world.Dungeons[0].Floors)
                    world.TileAt(floor.Spawn);
                var underground = world.Dungeons[0].Floors[0].Spawn;
                world.TileAt(new Int2(underground.X, underground.Z));
                check(!world.GetTile(underground).Position.Equals(world.GetTile(new Int2(underground.X, underground.Z)).Position), "surface and dungeon layer aliased in the authority tile index");
                expectedFrontier = WireJson.Encode(world.GenerateSurfaceTile(frontier));
                runtime.Save();
                expected = WireJson.Encode(world);
                count = world.Tiles.Count;
            }
            using (var runtime = new AuthorityRuntime(path, 999))
            {
                check(WireJson.Encode(runtime.World) == expected, "restart changed saved terrain, dungeon or plateau state");
                check(WireJson.Encode(runtime.World.GenerateSurfaceTile(frontier)) == expectedFrontier && runtime.World.Tiles.Count == count, "restart changed the unexplored frontier or generated it during sampling");
                check(runtime.World.GetTile(sample).Elevation == runtime.World.SurfaceHeight(sample), "height read ignored persisted authority geometry");
                runtime.Save();
            }
            check(WireJson.Encode(SaveStore.Load<World>(path)) == expected, "second save changed the unchanged authoritative state");
        }));

        test("world generation authority fractional dungeon combat resumes same finite state", () => WithDirectory(directory =>
        {
            var path = Path.Combine(directory, "world.json");
            string catId, creatureId, encoded;
            World uninterrupted;
            double enemyHealth, catHealth;
            using (var runtime = new AuthorityRuntime(path, 41))
            {
                var owner = runtime.Connect(null, 1000);
                var village = runtime.World.Village(owner.SelectedVillageId);
                var cat = village.Cats[0];
                var creature = runtime.World.Creatures.First(e => e.SiteId == "dungeon:" + village.Id && e.Position.Level == -1);
                catId = cat.Id;
                creatureId = creature.Id;
                cat.Position = creature.Position;
                cat.X = creature.X;
                cat.Z = creature.Z;
                cat.Cargo.Add(new Stack("ore", 2.25));
                village.Items.Add(new Item { Id = "fixture-recovered-mug", Kind = "mug", Material = "stone", VillageId = village.Id, LocationId = cat.Id, Condition = 17, MaxCondition = 42, Quality = 3 });
                check(runtime.Apply(owner, new GameAction { Kind = "EnterCatControl", CatId = catId }).Success, "authorized cat could not enter control");
                check(runtime.Apply(owner, new GameAction { Kind = "AttackCreature", CatId = catId, TargetId = creatureId }).Success, "authorized physical dungeon attack rejected");
                double beforeEnemy = creature.Health, beforeCat = cat.Health;
                runtime.Advance(0.075);
                enemyHealth = creature.Health;
                catHealth = cat.Health;
                check(enemyHealth > 0 && enemyHealth < beforeEnemy && catHealth > 0 && catHealth < beforeCat, "fractional combat did not change both living actors' finite health");
                check(Math.Abs(runtime.World.TimeSeconds - runtime.World.SimulationTimeSeconds - 0.025) < 1e-8, "fixture did not stop between canonical actor quanta");
                runtime.Save();
                encoded = WireJson.Encode(runtime.World);
                uninterrupted = WireJson.Clone(runtime.World);
            }
            using (var runtime = new AuthorityRuntime(path, 7))
            {
                check(WireJson.Encode(runtime.World) == encoded, "restart replaced dungeon actors, cargo or their fractional clock");
                runtime.Advance(0.01);
                check(runtime.World.Cat(catId).Health == catHealth && runtime.World.Creatures.Single(e => e.Id == creatureId).Health == enemyHealth, "restart replayed already consumed combat time");
                runtime.Advance(0.065);
                uninterrupted.Step(0.075);
                var actual = runtime.World.Cat(catId);
                var expected = uninterrupted.Cat(catId);
                check(actual.Id == catId && actual.Position.Level == -1 && actual.Position.Equals(expected.Position) && actual.Health == expected.Health, "restart changed the same cat's layer or combat outcome");
                check(runtime.World.Creatures.Single(e => e.Id == creatureId).Health == uninterrupted.Creatures.Single(e => e.Id == creatureId).Health, "partitioned restart changed enemy health");
                check(World.Amount(actual.Cargo, "ore") == 2.25 && runtime.World.Village(actual.VillageId).Items.Single(i => i.Id == "fixture-recovered-mug").LocationId == catId, "dungeon restart lost or duplicated physical carried goods");
                runtime.Save();
            }
            var second = SaveStore.Load<World>(path);
            check(World.Amount(second.Cat(catId).Cargo, "ore") == 2.25 && second.Creatures.Count(e => e.Id == creatureId) == 1 && second.Cat(catId).Health > 0, "second restart duplicated cargo or guardian identity");
            AuthorityRuntime.ValidateWorld(second);
        }));

        test("world generation authority rejects unauthorized expedition control", () => WithDirectory(directory =>
        {
            using var runtime = new AuthorityRuntime(Path.Combine(directory, "world.json"), 41);
            var owner = runtime.Connect(null, 1000);
            var other = runtime.Connect(null, 1000);
            var founded = runtime.Apply(owner, new GameAction { Kind = "FoundVillage", Name = "Synthetic private expedition" });
            check(founded.Success, "private village fixture could not found");
            var village = runtime.World.Village(founded.VillageId);
            var cat = village.Cats[0];
            World.Add(cat.Skills, "fight", 8);
            var site = runtime.World.Dungeons.First(d => d.RumoredBy.Contains(village.Id));
            var creature = runtime.World.Creatures.First(e => e.SiteId == site.Id);
            foreach (var kind in new[] { "ExploreDungeon", "RecallExplorer", "AttackCreature" })
            {
                var action = new GameAction { Kind = kind, CatId = cat.Id, TargetId = kind == "AttackCreature" ? creature.Id : site.Id };
                string before = WireJson.Encode(runtime.World);
                check(!runtime.Apply(null, action).Success && !runtime.Apply(other, action).Success, "anonymous or foreign identity controlled a private expedition");
                other.SelectedVillageId = village.Id;
                check(!runtime.Apply(other, action).Success, "forged selected village bypassed expedition ownership");
                other.SelectedVillageId = "communal";
                check(WireJson.Encode(runtime.World) == before, "rejected expedition action changed authoritative state");
            }
            check(runtime.Apply(owner, new GameAction { Kind = "ExploreDungeon", CatId = cat.Id, TargetId = site.Id }).Success, "owner could not dispatch its prepared adult cat");
            string dispatched = WireJson.Encode(runtime.World);
            check(!runtime.Apply(other, new GameAction { Kind = "RecallExplorer", CatId = cat.Id }).Success && WireJson.Encode(runtime.World) == dispatched, "foreign recall interrupted a real expedition");
            check(runtime.Apply(owner, new GameAction { Kind = "RecallExplorer", CatId = cat.Id }).Success && cat.DungeonPhase == "returning", "owner could not recall the same explorer");
        }));

        test("world generation authority projection hides unknown dungeon cells loot and rumors", () => WithDirectory(directory =>
        {
            using var runtime = new AuthorityRuntime(Path.Combine(directory, "world.json"), 41);
            var owner = runtime.Connect(null, 1000);
            var other = runtime.Connect(null, 1000);
            var founded = runtime.Apply(owner, new GameAction { Kind = "FoundVillage", Name = "Synthetic hidden cavern" });
            check(founded.Success, "private dungeon fixture could not found");
            var privateVillage = runtime.World.Village(founded.VillageId);
            var communal = runtime.World.Village("communal");
            var site = runtime.World.Dungeons.First(d => d.Id == "dungeon:" + communal.Id);
            var floor = site.Floors[0];
            var creature = runtime.World.Creatures.First(e => e.SiteId == site.Id && e.Position.Level == floor.Level);
            site.RumoredBy.Add(privateVillage.Id);
            foreach (var cell in floor.Cells)
                runtime.World.TileAt(cell);
            string canonical = WireJson.Encode(runtime.World);
            var unseen = runtime.Project(other).World;
            var rumor = unseen.Dungeons.Single(d => d.Id == site.Id);
            check(rumor.Entrance.Equals(site.Entrance) && rumor.Floors.Count == 0 && rumor.Stairs.Count == 0, "entrance rumor exposed undiscovered dungeon layout or floor coordinates");
            check(unseen.Creatures.Count == 0 && unseen.Tiles.All(t => t.Position.Level == 0), "unseen dungeon creatures or generated hidden floor tiles leaked");
            check(unseen.Dungeons.All(d => d.RumoredBy.All(id => id == communal.Id)) && unseen.Dungeons.All(d => !d.RumoredBy.Contains(privateVillage.Id)), "another village's dungeon rumors leaked");
            check(unseen.SurfacePlateaus.All(p => p.VillageId == communal.Id) && unseen.Dungeons.All(d => runtime.World.Dungeons.Single(source => source.Id == d.Id).RumoredBy.Contains(communal.Id)), "private founding profile or private dungeon site leaked");
            check(WireJson.Encode(runtime.World) == canonical, "unseen projection mutated canonical dungeon state");

            communal.Known.Add(floor.Spawn);
            var partlyKnown = runtime.Project(other).World;
            var visibleFloor = partlyKnown.Dungeons.Single(d => d.Id == site.Id).Floors.Single();
            check(visibleFloor.Cells.SequenceEqual(new[] { floor.Spawn }) && visibleFloor.Loot.Count == 0 && partlyKnown.Creatures.Count == 0, "one discovered stair landing revealed the rest of its floor, chest or guardian");
            var knowledge = communal.Known.ToHashSet();
            check(partlyKnown.Dungeons.SelectMany(d => d.Stairs).All(stair => knowledge.Contains(stair.From) && knowledge.Contains(stair.To)), "partial discovery exposed an unknown stair endpoint");

            communal.Known.Add(creature.Position);
            communal.Known.Add(floor.LootPosition);
            canonical = WireJson.Encode(runtime.World);
            var known = runtime.Project(other).World;
            check(known.Creatures.Single(e => e.Id == creature.Id).Health == creature.Health, "visible guardian lost its actual finite health");
            check(known.Dungeons.Single(d => d.Id == site.Id).Floors.Single().Loot.Sum(s => s.Amount) == floor.Loot.Sum(s => s.Amount), "discovered chest lost its remaining finite loot");
            check(known.Dungeons.SelectMany(d => d.Floors).All(f => f.Level == floor.Level), "discovering one floor exposed deeper floors");
            check(WireJson.Encode(runtime.World) == canonical, "known projection consumed loot or changed actor state");
        }));
    }

    private static void WithDirectory(Action<string> test)
    {
        var directory = Path.Combine(Path.GetTempPath(), "forest-world-generation-test-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(directory);
        try { test(directory); }
        finally { Directory.Delete(directory, true); }
    }
}
