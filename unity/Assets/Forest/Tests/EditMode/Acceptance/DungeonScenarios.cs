using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Acceptance
{
    public static class DungeonScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("regression.dungeons_generated_multilevel", GeneratedMultilevel);
            yield return new Scenario("regression.dungeons_seeded_geometry", SeededGeometry);
            yield return new Scenario("regression.dungeons_risk_refuses_unready_cat", RiskRefusal);
            yield return new Scenario("regression.dungeons_stairs_require_physical_time", PhysicalStairs);
            yield return new Scenario("regression.dungeons_combat_loot_return_chain", ExpeditionChain);
            yield return new Scenario("regression.dungeons_recall_carries_goods_home", RecallGoods);
            yield return new Scenario("regression.dungeons_needs_force_safe_return", NeedsReturn);
            yield return new Scenario("regression.dungeons_wall_blocks_combat", WallCombat);
            yield return new Scenario("regression.dungeons_direct_control_release_returns", DirectRelease);
            yield return new Scenario("regression.dungeons_legacy_world_untouched", LegacyWorld);
            yield return new Scenario("regression.dungeons_stair_closure_keeps_cat_physical", StairClosure);
            yield return new Scenario("regression.dungeons_full_storage_releases_critical_needs", FullStorageNeeds);
            yield return new Scenario("regression.dungeons_death_preserves_carried_goods", DeathGoods);
            yield return new Scenario("regression.dungeons_layer_separates_combat", LayerCombat);
            yield return new Scenario("regression.dungeons_review_emergency_keeps_expedition", EmergencyOwnership);
            yield return new Scenario("regression.dungeons_review_assignment_waits_for_return", AssignmentOwnership);
            yield return new Scenario("regression.dungeons_review_equipment_waits_for_return", EquipmentOwnership);
            yield return new Scenario("regression.dungeons_stairs_carve_upper_floor_opening", StairOpening);
            yield return new Scenario("regression.dungeons_manual_loot_once", ManualLoot);
            yield return new Scenario("regression.dungeons_entrance_has_visible_clearing", () =>
            {
                var world = World.Create(41); var village = world.Villages[0]; var entrance = world.Dungeons[0].Entrance;
                for (int x = -3; x <= 3; x++) for (int z = -3; z <= 3; z++)
                    Check(village.Known.Contains(new Int2(entrance.X + x, entrance.Z + z)), "The founding entrance must be visible in its surrounding clearing");
                Check(village.Known.TrueForAll(p => p.Level == 0), "The clearing must not reveal underground rooms");
            });
        }

        private static void GeneratedMultilevel()
        {
            var world = World.Create(41);
            Check(world.Dungeons.Count > 0, "A fresh seeded world must contain persistent multilevel dungeon sites");
            var site = world.Dungeons[0];
            Check(site.Depth == 2 && site.Floors.Count == 2, "The near dungeon must contain two real underground floors");
            Check(site.Stairs.Count == 2 && site.Stairs.All(s => Math.Abs(s.To.X - s.From.X) == 4 && s.From.Z == s.To.Z && s.To.Level == s.From.Level - 1), "Each descent must spend four horizontal and four vertical metres");
            Check(site.Floors[1].Threat > site.Floors[0].Threat, "Danger must increase with depth");
            Check(site.Floors.All(f => f.Cells.All(p => p.Level == f.Level) && world.Path(f.Spawn, f.LootPosition) != null), "Generated rooms and corridors must connect their physical loot chest");
            Check(world.Path(world.Villages[0].Center, site.Entrance, world.Villages[0]) != null, "The first entrance must connect to the founding gate");
        }

        private static void Check(bool success, string message)
        {
            if (!success) throw new InvalidOperationException(message);
        }

        private static (World World, Village Village, Cat Cat, DungeonSite Site) Fixture()
        {
            var world = World.Create(41);
            var village = world.Villages[0];
            var cat = village.Cats[0];
            village.Cats.RemoveAll(c => c.Id != cat.Id);
            cat.Skills.Clear();
            cat.Skills.Add(new Stack("fight", 12));
            cat.Hunger = cat.Thirst = cat.Rest = cat.Health = 100;
            var site = world.Dungeons[0];
            Place(world, cat, site.Entrance);
            return (world, village, cat, site);
        }

        private static void Place(World world, Cat cat, Int2 at)
        {
            cat.Position = at;
            cat.X = at.X;
            cat.Z = at.Z;
            cat.Y = world.WalkHeight(at);
            cat.HasHeight = true;
            cat.Path.Clear();
        }
        private static void ManualLoot()
        {
            var f = Fixture(); var floor = f.Site.Floors[0];
            var identity = new PlayerContext { PlayerId = "dungeon-manual", VillageId = f.Village.Id };
            Check(f.World.Apply(identity, new GameAction { Kind = "EnterCatControl", CatId = f.Cat.Id }).Success, "Control same explorer");
            Place(f.World, f.Cat, floor.LootPosition);
            double finite = floor.Loot.Sum(s => s.Amount);
            var pickup = new GameAction { Kind = "InteractCat", CatId = f.Cat.Id };
            Check(f.World.Apply(identity, pickup).Success, "A controlled cat must collect the reached dungeon chest through ordinary interaction");
            Check(f.Cat.Cargo.Sum(s => s.Amount) == finite && floor.Loot.Count == 0, "Pickup transfers finite chest contents once");
            f.World.Apply(identity, pickup);
            Check(f.Cat.Cargo.Sum(s => s.Amount) == finite && floor.Loot.Count == 0, "Repeated interaction must not recreate loot");
        }

        private static void SeededGeometry()
        {
            string Geometry(World world) => string.Join("|", world.Dungeons.SelectMany(d => d.Floors).SelectMany(f => f.Cells).Select(p => p.ToString()));
            var first = World.Create(41);
            var twin = World.Create(41);
            Check(Geometry(first) == Geometry(twin), "Equal seeds must generate identical floor cells");
            Check(Geometry(first) != Geometry(World.Create(7)), "Different seeds must change room/corridor layouts");
            Check(first.Creatures.Select(c => c.Id).SequenceEqual(twin.Creatures.Select(c => c.Id)), "Creature identities must be stable across deterministic twins");
        }

        private static void RiskRefusal()
        {
            var f = Fixture();
            f.Cat.Skills.Clear();
            var position = f.Cat.Position;
            Check(!f.World.ExploreDungeon(f.Village, f.Cat, f.Site.Id).Success, "An untrained cat must refuse danger above its readiness");
            Check(f.Cat.Position.Equals(position) && f.Cat.DungeonId == "" && f.Cat.Cargo.Count == 0, "Risk refusal must not mutate actor or cargo ownership");
        }

        private static void PhysicalStairs()
        {
            var f = Fixture();
            Check(f.World.ExploreDungeon(f.Village, f.Cat, f.Site.Id).Success, "Ready cat must accept the reachable expedition");
            f.World.Step(.05);
            Check(f.Cat.Position.Level == 0 && f.Cat.X > f.Site.Entrance.X && f.Cat.X < f.Site.Entrance.X + 1, "The first quantum must move along the stair without teleporting to its endpoint");
            Check(f.Cat.Y < 0 && f.Cat.Y > -1, "Descending must interpolate actual height");
            f.World.Step(4);
            Check(f.Cat.Position.Level == -1, "The cat must reach the first floor after physically traversing the stair");
        }

        private static void ExpeditionChain()
        {
            var f = Fixture();
            double before = f.World.Total(f.Village, "gem"), initial = f.Site.Floors.Sum(level => level.Loot.Where(s => s.Resource == "gem").Sum(s => s.Amount));
            Check(f.World.ExploreDungeon(f.Village, f.Cat, f.Site.Id).Success, "Ready explorer accepts");
            bool underground = false, fought = false, carried = false;
            for (int i = 0; i < 150 && f.World.IsDungeonExplorer(f.Cat); i++)
            {
                f.World.Step(1);
                underground |= f.Cat.Position.Level < 0;
                fought |= f.World.Creatures.Any(c => c.SiteId == f.Site.Id && c.Health < c.MaxHealth);
                carried |= f.Cat.Cargo.Any(s => s.Resource == "gem");
            }
            Check(underground && fought && carried, "Exploration must physically descend, fight and carry the finite loot");
            Check(!f.World.IsDungeonExplorer(f.Cat) && f.Cat.Position.Level == 0 && f.Cat.Cargo.Count == 0, "The same cat must return and unload into village storage");
            Check(Math.Abs(f.World.Total(f.Village, "gem") - before - initial) < 1e-8, "The village gains exactly the finite generated gem chest after physical delivery");
            Check(f.World.Creatures.Where(c => c.SiteId == f.Site.Id).All(c => c.Health == 0), "Defeated creatures remain present and dead rather than respawning");
        }

        private static void RecallGoods()
        {
            var f = Fixture();
            Place(f.World, f.Cat, f.Site.Floors[0].Spawn);
            f.Cat.DungeonId = f.Site.Id;
            f.Cat.DungeonPhase = "exploring";
            f.Cat.Cargo.Add(new Stack("gem", 2));
            double before = f.World.Total(f.Village, "gem");
            Check(f.World.RecallExplorer(f.Village, f.Cat).Success, "Recall accepts an owned explorer");
            for (int i = 0; i < 60 && f.World.IsDungeonExplorer(f.Cat); i++) f.World.Step(1);
            Check(f.Cat.Position.Level == 0 && f.Cat.DungeonId == "" && f.Cat.Cargo.Count == 0, "Recall must retrace stairs and unload cargo");
            Check(Math.Abs(f.World.Total(f.Village, "gem") - before - 2) < 1e-8, "Recall transfers the two carried gems into storage without creating or losing goods");
        }

        private static void NeedsReturn()
        {
            var f = Fixture();
            Place(f.World, f.Cat, f.Site.Floors[0].Spawn);
            f.Cat.DungeonId = f.Site.Id;
            f.Cat.DungeonPhase = "exploring";
            f.Cat.Thirst = 29;
            f.World.Step(.05);
            Check(f.Cat.DungeonPhase == "returning" && f.Cat.X < f.Site.Floors[0].Spawn.X, "Low needs must trigger physical retreat before pursuing another fight");
        }

        private static void WallCombat()
        {
            var f = Fixture();
            var enemy = f.World.Creatures.First(c => c.SiteId == f.Site.Id);
            Place(f.World, f.Cat, new Int2(enemy.Position.X - 1, enemy.Position.Z, enemy.Position.Level));
            f.Cat.ControlledBy = "test";
            f.Cat.ControlLeaseUntil = 100;
            var wall = f.World.TileAt(enemy.Position);
            wall.Wall = true;
            Check(!f.World.AttackCreature(f.Village, f.Cat, enemy.Id).Success, "A blocked dungeon edge must prevent direct attacks through its wall");
        }

        private static void DirectRelease()
        {
            var f = Fixture();
            Place(f.World, f.Cat, f.Site.Floors[0].Spawn);
            f.Cat.ControlledBy = "test";
            f.Cat.ControlLeaseUntil = .1;
            f.World.Step(.05);
            Check(f.Cat.DungeonPhase == "manual", "Entering an underground space uses the same controlled cat");
            f.World.Step(.1);
            Check(f.Cat.ControlledBy == "" && f.Cat.DungeonPhase == "returning", "An expired direct-control lease must bring an underground cat home");
        }

        private static void LegacyWorld()
        {
            var world = new World();
            var village = new Village { Id = "legacy", Center = new Int2(0, 0) };
            var tile = new Tile { Position = new Int2(0, 13), Water = true, Amount = 17 };
            world.Tiles.Add(tile);
            world.Villages.Add(village);
            world.EnsureVillageDungeon(village);
            Check(world.Dungeons.Count == 0 && tile.Water && tile.Amount == 17, "Legacy founding terrain must not be regenerated to add a dungeon");
        }

        private static void StairClosure()
        {
            var f = Fixture();
            Check(f.World.ExploreDungeon(f.Village, f.Cat, f.Site.Id).Success, "Ready explorer accepts");
            f.World.Step(.2);
            double prior = f.Cat.X;
            f.World.TileAt(f.Site.Floors[0].Spawn).Wall = true;
            f.World.Step(.05);
            Check(f.Cat.Position.Level == 0 && f.Cat.X < prior && f.Cat.X > f.Site.Entrance.X, "Closing the stair must physically retrace its partly travelled segment");
        }

        private static void FullStorageNeeds()
        {
            var f = Fixture();
            Place(f.World, f.Cat, f.Village.Center);
            f.Cat.DungeonId = f.Site.Id;
            f.Cat.DungeonPhase = "unloading";
            f.Cat.Cargo.Add(new Stack("gem", 2));
            f.Cat.Thirst = 10;
            foreach (var pile in f.Village.Stockpiles) pile.Accepts = new List<string> { "food", "water" };
            f.World.Step(.05);
            Check(f.Cat.DungeonId == "", "A returning cat blocked on storage must be released to satisfy critical needs");
            Check(f.Village.Stockpiles.Where(p => p.Kind == "spill" && p.Position.Equals(f.Village.Center)).Sum(p => p.Goods.Where(s => s.Resource == "gem").Sum(s => s.Amount)) == 2, "Its rejected cargo must remain in a physical recoverable spill at home");
        }

        private static void DeathGoods()
        {
            var f = Fixture();
            var at = f.Site.Floors[0].Spawn;
            Place(f.World, f.Cat, at);
            f.Cat.DungeonId = f.Site.Id;
            f.Cat.DungeonPhase = "manual";
            f.Cat.Cargo.Add(new Stack("gem", 3));
            f.Cat.Health = 0;
            f.World.Step(.05);
            Check(!f.Cat.Alive && f.Cat.Cargo.Count == 0 && f.Cat.DungeonId == "", "Death must release the same cat's expedition and cargo ownership");
            Check(f.Village.Stockpiles.Where(p => p.Kind == "spill" && p.Position.Equals(at)).Sum(p => p.Goods.Where(s => s.Resource == "gem").Sum(s => s.Amount)) == 3, "Death must leave exact finite carried cargo on the actual dungeon floor");
        }

        private static void LayerCombat()
        {
            var f = Fixture();
            var enemy = f.World.Creatures.First(c => c.SiteId == f.Site.Id);
            Place(f.World, f.Cat, new Int2(enemy.Position.X, enemy.Position.Z, 0));
            f.Cat.ControlledBy = "test";
            f.Cat.ControlLeaseUntil = 100;
            Check(!f.World.AttackCreature(f.Village, f.Cat, enemy.Id).Success, "Matching XZ coordinates on a different floor must not allow attacks");
            double health = f.Cat.Health;
            f.World.Step(.05);
            Check(f.Cat.Health == health, "A creature underground must not attack a surface cat above it");
        }

        private static void EmergencyOwnership()
        {
            var f = Fixture();
            Place(f.World, f.Cat, f.Site.Floors[0].Spawn);
            f.Cat.DungeonId = f.Site.Id;
            f.Cat.DungeonPhase = "exploring";
            f.Cat.Cargo.Add(new Stack("gem", 3));
            typeof(World).GetMethod("Emergency", System.Reflection.BindingFlags.Instance | System.Reflection.BindingFlags.NonPublic).Invoke(f.World, new object[] { f.Village, "water" });
            Check(f.Cat.DungeonId == f.Site.Id && f.Cat.Cargo.Sum(s => s.Amount) == 3 && f.Cat.JobId == "", "Emergency staffing must not cancel an underground expedition or drop its cargo to claim a surface job");
        }

        private static void AssignmentOwnership()
        {
            var f = Fixture();
            Place(f.World, f.Cat, f.Site.Floors[0].Spawn);
            f.Cat.DungeonId = f.Site.Id;
            f.Cat.DungeonPhase = "exploring";
            var result = f.World.Apply(new PlayerContext { PlayerId = "test", VillageId = f.Village.Id }, new GameAction { Kind = "AssignWorker", CatId = f.Cat.Id, BuildingId = f.Village.Buildings.First(b => b.Kind == "woodworking").Id });
            Check(!result.Success && f.Cat.DungeonId == f.Site.Id && f.Cat.BuildingId == "", "Station reassignment must wait until a dungeon explorer has physically returned");
        }

        private static void EquipmentOwnership()
        {
            var f = Fixture();
            Place(f.World, f.Cat, f.Site.Floors[0].Spawn);
            f.Cat.DungeonId = f.Site.Id;
            f.Cat.DungeonPhase = "exploring";
            var pile = f.Village.Stockpiles.First(p => p.Kind == "storage");
            var item = new Item { Id = "fixture-dungeon-weapon", VillageId = f.Village.Id, LocationId = pile.Id, Kind = "weapon", Material = "metal", Quality = 3 };
            f.Village.Items.Add(item);
            var result = f.World.Apply(new PlayerContext { PlayerId = "test", VillageId = f.Village.Id }, new GameAction { Kind = "EquipItem", CatId = f.Cat.Id, TargetId = item.Id });
            Check(!result.Success && item.LocationId == pile.Id && !f.Cat.Equipment.Contains(item.Id), "Equipment must not jump from surface storage to a dungeon fight");
        }

        private static void StairOpening()
        {
            var f = Fixture();
            foreach (var stair in f.Site.Stairs)
                for (int step = 1; step <= 4; step++)
                {
                    var aperture = new Int2(stair.From.X + Math.Sign(stair.To.X - stair.From.X) * step, stair.From.Z, stair.From.Level);
                    Check(!f.World.Walkable(aperture) && !f.World.Walkable(f.Village, aperture), "A real stair must carve an opening through the upper floor instead of passing below its solid tiles");
                    if (aperture.Level < 0)
                        Check(!f.Site.Floors.First(level => level.Level == aperture.Level).Cells.Contains(aperture), "Upper dungeon floor geometry must omit the stair's open shaft");
                }
            Check(f.Site.Floors.All(level => f.World.Path(level.Spawn, level.LootPosition) != null), "The aperture must preserve a route around its rim to each chest");
        }
    }
}
