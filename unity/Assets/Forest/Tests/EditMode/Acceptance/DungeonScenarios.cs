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
            yield return new Scenario("dungeon.known_cells_have_tiles", KnownCellsHaveTiles);
            yield return new Scenario("dungeon.stair_endpoints_materialise", StairEndpointsMaterialise);
            yield return new Scenario("dungeon.fresh_adults_enter_floor_one_and_one_training_makes_it_safe", FreshAdultsEnterFloorOne);
            yield return new Scenario("dungeon.untrained_cat_loses_about_35_hp_on_floor_one", () => FloorOneLoss(0, 25, 45));
            yield return new Scenario("dungeon.trained_cat_loses_about_10_hp_on_floor_one", () => FloorOneLoss(3, 5, 16));
            yield return new Scenario("dungeon.started_fight_finishes_above_the_floor", StartedFightFinishes);
            yield return new Scenario("dungeon.fight_below_the_floor_flees_alive", FightBelowFloorFlees);
            yield return new Scenario("dungeon.only_accepted_deadly_expeditions_can_kill", OnlyAcceptedDeadlyKills);
            yield return new Scenario("dungeon.deadly_floor_needs_explicit_accept", DeadlyNeedsAccept);
            yield return new Scenario("dungeon.chest_locked_while_guardian_lives", ChestLocked);
            yield return new Scenario("dungeon.cleared_floor_restocks", ClearedFloorRestocks);
            yield return new Scenario("dungeon.dispatch_starts_at_first_uncleared_floor", FirstUnclearedFloor);
            yield return new Scenario("dungeon.floor_one_loot_is_research_and_a_bone_weapon", FloorOneLoot);
            yield return new Scenario("dungeon.floor_two_relic_buffs_the_colony", FloorTwoRelic);
            yield return new Scenario("dungeon.stranded_explorer_cancels_with_warning", StrandedExplorer);
            yield return new Scenario("dungeon.release_control_continues_expedition", ReleaseContinues);
            yield return new Scenario("dungeon.guardian_returns_home_and_heals", GuardianReturnsHome);
            yield return new Scenario("dungeon.refusal_keeps_current_work", RefusalKeepsWork);
        }

        // ---- W2-09: fair floors, real loot, restock and explorer safety ----

        private const double FightFloor = 15;

        private static DungeonCreature Guardian(World world, DungeonSite site, int index) =>
            world.Creatures.First(e => e.SiteId == site.Id && e.Home.Level == site.Floors[index].Level);

        private static void ClearFloor(World world, DungeonSite site, int index)
        {
            Guardian(world, site, index).Health = 0;
            site.Floors[index].Loot.Clear();
        }

        private static (World World, Village Village, Cat Cat, DungeonSite Site) Trained(double fight)
        {
            var f = Fixture();
            f.Cat.Skills.Clear();
            if (fight > 0)
                f.Cat.Skills.Add(new Stack("fight", fight));
            return f;
        }

        private static double CarriedTreasure(Village village, Cat cat) =>
            cat.Cargo.Sum(s => s.Amount) + cat.DungeonLore + village.Items.Count(i => i.LocationId == cat.Id && !cat.Equipment.Contains(i.Id));

        private static string Snapshot(World world, Village village) =>
            string.Join("|", village.Cats.Select(c => c.Id + ":" + c.DungeonId + ":" + c.DungeonPhase + ":" + c.DungeonFloorIndex + ":" + c.JobId + ":" + c.BuildingId + ":" + c.Goal + ":" + c.Position + ":" + c.Cargo.Count + ":" + c.DungeonAcceptedDeadly))
            + "|" + village.Jobs.Count + "|" + village.Events.Count + "|" + string.Join(",", world.Creatures.Select(e => e.Health));

        private static void Control(Cat cat)
        {
            cat.ControlledBy = "test";
            cat.ControlLeaseUntil = 100000;
        }

        private static void KnownCellsHaveTiles()
        {
            var f = Fixture();
            Place(f.World, f.Cat, f.Site.Floors[0].Spawn);
            Control(f.Cat);
            f.World.Step(1);
            var underground = f.Village.Known.Where(p => p.Level < 0).ToList();
            Check(underground.Count > 4, "Standing on the first floor must reveal its nearby cells");
            Check(underground.All(p => f.World.GetTile(p) != null), "Every revealed dungeon cell must exist as a tile so the floor and walls render");
        }

        private static void StairEndpointsMaterialise()
        {
            var f = Fixture();
            Control(f.Cat);
            var stair = f.Site.Stairs[0];
            Check(f.World.GetTile(stair.To) == null, "Fixture: the landing must not exist before anyone uses the stair");
            f.World.MaterialiseStairEndpoints(f.Village, f.Cat);
            Check(f.World.GetTile(stair.From) != null && f.World.GetTile(stair.To) != null, "Stepping onto a stair must materialise both of its endpoints");
            Check(f.Village.Known.Contains(stair.To), "The landing at the bottom of a used stair must be known");
        }

        private static void FreshAdultsEnterFloorOne()
        {
            var world = World.Create(41);
            var village = world.Villages[0];
            var site = world.Dungeons[0];
            var floor = site.Floors[0];
            var adults = village.Cats.Where(c => c.Alive && c.AgeHours >= world.CurrentPacing().WorkerMinAgeHours && c.Health >= 65 && c.Hunger >= 45 && c.Thirst >= 45 && c.Rest >= 45).ToList();
            Check(adults.Count > 0, "The founding colony must have healthy adults");
            foreach (var adult in adults)
            {
                var skills = adult.Skills.Select(s => new Stack(s.Resource, s.Amount)).ToList();
                adult.Skills.RemoveAll(s => s.Resource == "fight");
                Check(world.RiskOf(adult, floor).Risk != DungeonRisk.Deadly, adult.Name + " (untrained) must be able to try Root Hollow floor 1");
                adult.Skills = skills;
            }
            var cat = adults[0];
            cat.Skills.RemoveAll(s => s.Resource == "fight");
            var untrained = world.RiskOf(cat, floor);
            Check(untrained.Risk == DungeonRisk.Risky, "An untrained adult must read as Risky on floor 1, not " + untrained.Risk);
            var context = new PlayerContext { PlayerId = "fresh-trainer", VillageId = village.Id };
            var train = world.Apply(context, new GameAction { Kind = "TrainWarrior", CatId = cat.Id });
            Check(train.Success, "A fresh colony must be able to train an adult: " + train.Error);
            var job = village.Jobs.Single(j => j.Id == train.EntityId);
            for (int i = 0; i < 900 && !job.Completed; i++)
                world.Step(1);
            Check(job.Completed && World.Amount(cat.Skills, "fight") > 0, "One Train job must finish and add fighting skill");
            cat.Health = Math.Max(cat.Health, 65);
            cat.Hunger = Math.Max(cat.Hunger, 45);
            cat.Thirst = Math.Max(cat.Thirst, 45);
            cat.Rest = Math.Max(cat.Rest, 45);
            var trained = world.RiskOf(cat, floor);
            Check(trained.Risk == DungeonRisk.Safe && trained.ExpectedHpLoss < untrained.ExpectedHpLoss, "One Train job must make floor 1 Safe: " + trained.Risk + " " + trained.ExpectedHpLoss);
            var dispatched = world.Apply(context, new GameAction { Kind = "ExploreDungeon", CatId = cat.Id, TargetId = site.Id });
            Check(dispatched.Success && cat.DungeonId == site.Id, "A trained fresh adult must be dispatched to Root Hollow: " + dispatched.Error);
        }

        // Runs one expedition until the floor-1 guardian falls and returns the health the fight cost.
        private static double FightFloorOne((World World, Village Village, Cat Cat, DungeonSite Site) f)
        {
            var guardian = Guardian(f.World, f.Site, 0);
            for (int i = 0; i < 1200 && guardian.Health > 0 && f.Cat.Alive && f.World.IsDungeonExplorer(f.Cat); i++)
                f.World.Step(.25);
            Check(guardian.Health <= 0, "The floor-1 guardian must fall: health=" + guardian.Health + " phase=" + f.Cat.DungeonPhase + " reason=" + f.Cat.BlockedReason + " catHealth=" + f.Cat.Health);
            return 100 - f.Cat.Health;
        }

        private static void FloorOneLoss(double fight, double low, double high)
        {
            var f = Trained(fight);
            var estimate = f.World.RiskOf(f.Cat, f.Site.Floors[0]);
            Check(f.World.ExploreDungeon(f.Village, f.Cat, f.Site.Id).Success, "A healthy adult with fight " + fight + " must accept floor 1 (" + estimate.Risk + ")");
            double lost = FightFloorOne(f);
            Check(lost >= low && lost <= high, "Fight " + fight + " cat lost " + lost.ToString("0.0") + " HP on floor 1; expected " + low + " to " + high);
            Check(Math.Abs(lost - estimate.ExpectedHpLoss) <= 10, "The shown estimate (" + estimate.ExpectedHpLoss.ToString("0.0") + ") must predict the real loss (" + lost.ToString("0.0") + ")");
            Check(f.Cat.Alive && f.Cat.Health > FightFloor, "A non-deadly floor-1 fight must end above the floor");
        }

        private static void StartedFightFinishes()
        {
            var f = Trained(3);
            var guardian = Guardian(f.World, f.Site, 0);
            Check(f.World.ExploreDungeon(f.Village, f.Cat, f.Site.Id).Success, "Trained cat accepts");
            for (int i = 0; i < 600 && guardian.Health >= guardian.MaxHealth; i++)
                f.World.Step(.05);
            Check(guardian.Health < guardian.MaxHealth && guardian.Health > 0, "Fixture: the fight must have started");
            // Needs falling below the retreat line no longer abandon a fight that is under way.
            f.Cat.Thirst = 29;
            for (int i = 0; i < 400 && guardian.Health > 0; i++)
                f.World.Step(.05);
            Check(guardian.Health <= 0, "A started fight must finish while the cat stays above the floor (phase=" + f.Cat.DungeonPhase + ")");
            for (int i = 0; i < 5; i++)
                f.World.Step(.05);
            Check(f.Cat.DungeonPhase == "returning", "Low needs must still send the cat home once the fight is over");
        }

        private static void FightBelowFloorFlees()
        {
            var f = Trained(0);
            var guardian = Guardian(f.World, f.Site, 0);
            Check(f.World.ExploreDungeon(f.Village, f.Cat, f.Site.Id).Success, "Untrained cat accepts a Risky floor");
            for (int i = 0; i < 600 && guardian.Health >= guardian.MaxHealth; i++)
                f.World.Step(.05);
            Check(guardian.Health < guardian.MaxHealth, "Fixture: the fight must have started");
            f.Cat.Health = FightFloor + 1;
            bool fled = false;
            for (int i = 0; i < 2400 && f.World.IsDungeonExplorer(f.Cat); i++)
            {
                f.World.Step(.05);
                fled |= f.Cat.DungeonPhase == "returning" && f.Cat.Health < FightFloor;
                Check(f.Cat.Alive && f.Cat.Health >= 1, "A cat on a non-deadly expedition must never be killed by a guardian");
            }
            Check(fled, "A cat knocked below the floor must flee");
            Check(f.Cat.Alive && f.Cat.Position.Level == 0 && !f.World.IsDungeonExplorer(f.Cat), "The wounded cat must make it home alive");
            Check(guardian.Health > 0, "The guardian survives a fight its opponent fled");
        }

        private static void OnlyAcceptedDeadlyKills()
        {
            foreach (bool accepted in new[] { false, true })
            {
                var f = Trained(0);
                var guardian = Guardian(f.World, f.Site, 0);
                Place(f.World, f.Cat, guardian.Position);
                f.Cat.DungeonId = f.Site.Id;
                f.Cat.DungeonPhase = "exploring";
                f.Cat.DungeonAcceptedDeadly = accepted;
                f.Cat.Health = 1;
                f.World.Step(2);
                if (accepted)
                    Check(!f.Cat.Alive && f.Village.Events.Any(e => e.Kind == "death" && e.EntityId == f.Cat.Id && e.Text.Contains("dungeon")), "An accepted Deadly expedition can cost the cat's life");
                else
                    Check(f.Cat.Alive && f.Cat.Health >= 1, "Only an explicitly accepted Deadly expedition may kill a cat");
            }
        }

        private static void DeadlyNeedsAccept()
        {
            var f = Trained(0);
            ClearFloor(f.World, f.Site, 0);
            var estimate = f.World.RiskOf(f.Cat, f.Site.Floors[1]);
            Check(estimate.Risk == DungeonRisk.Deadly, "An untrained cat must read floor 2 as Deadly, not " + estimate.Risk);
            var context = new PlayerContext { PlayerId = "deadly", VillageId = f.Village.Id };
            string before = Snapshot(f.World, f.Village);
            var refused = f.World.Apply(context, new GameAction { Kind = "ExploreDungeon", CatId = f.Cat.Id, TargetId = f.Site.Id });
            Check(!refused.Success && refused.Code == "dungeon_deadly", "A Deadly expedition must be refused without explicit acceptance: " + refused.Code + " " + refused.Error);
            Check(Snapshot(f.World, f.Village) == before, "A refused expedition must not change any state");
            var accepted = f.World.Apply(context, new GameAction { Kind = "ExploreDungeon", CatId = f.Cat.Id, TargetId = f.Site.Id, AcceptDeadly = true });
            Check(accepted.Success && f.Cat.DungeonAcceptedDeadly && f.Cat.DungeonFloorIndex == 1, "An accepted Deadly expedition must be dispatched to its floor");
        }

        private static void ChestLocked()
        {
            var f = Fixture();
            var floor = f.Site.Floors[0];
            var guardian = Guardian(f.World, f.Site, 0);
            Control(f.Cat);
            Place(f.World, f.Cat, floor.LootPosition);
            double contents = floor.Loot.Sum(s => s.Amount);
            var locked = f.World.TakeDungeonChest(f.Village, f.Cat);
            Check(!locked.Success && locked.Code == "chest_guarded" && floor.Loot.Sum(s => s.Amount) == contents, "The chest must stay locked while its guardian lives");
            guardian.Health = 0;
            var near = new Int2(floor.LootPosition.X, floor.LootPosition.Z - 1, floor.LootPosition.Level);
            if (f.World.Walkable(near))
                Place(f.World, f.Cat, near);
            var opened = f.World.TakeDungeonChest(f.Village, f.Cat);
            Check(opened.Success && floor.Loot.Count == 0, "A cat beside an unguarded chest must open it: " + opened.Code + " " + opened.Error);
            Check(Math.Abs(CarriedTreasure(f.Village, f.Cat) - contents) < 1e-9, "Opening transfers the finite contents once");
            Check(!f.World.TakeDungeonChest(f.Village, f.Cat).Success && Math.Abs(CarriedTreasure(f.Village, f.Cat) - contents) < 1e-9, "An empty chest yields nothing more");
        }

        private static void ClearedFloorRestocks()
        {
            var f = Fixture();
            var floor = f.Site.Floors[0];
            var guardian = Guardian(f.World, f.Site, 0);
            double contents = floor.Loot.Sum(s => s.Amount);
            var home = guardian.Home;
            guardian.Position = new Int2(home.X - 2, home.Z, home.Level);
            ClearFloor(f.World, f.Site, 0);
            double restock = f.World.CurrentPacing().FloorRestockSeconds;
            f.World.Step(restock - 5);
            Check(guardian.Health == 0 && floor.Loot.Count == 0, "A cleared floor stays cleared until its restock time");
            f.World.Step(10);
            Check(guardian.Health == guardian.MaxHealth && guardian.Position.Equals(home), "The guardian returns at full health to its post");
            Check(floor.Loot.Sum(s => s.Amount) >= contents - 1e-9 && floor.Loot.Count > 0, "The chest refills when the floor restocks");
            Check(f.Village.Events.Any(e => e.Kind == "dungeon" && e.EntityId == f.Site.Id), "The colony hears that the hollow stirs again");
        }

        private static void FirstUnclearedFloor()
        {
            var f = Fixture();
            ClearFloor(f.World, f.Site, 0);
            Check(f.World.ExploreDungeon(f.Village, f.Cat, f.Site.Id).Success && f.Cat.DungeonFloorIndex == 1, "A new trip must target the first floor that still holds a guardian or loot");
            var g = Fixture();
            ClearFloor(g.World, g.Site, 0);
            ClearFloor(g.World, g.Site, 1);
            var cleared = g.World.ExploreDungeon(g.Village, g.Cat, g.Site.Id);
            Check(!cleared.Success && cleared.Code == "dungeon_cleared" && g.Cat.DungeonId == "", "A fully cleared site must refuse with nothing left");
        }

        private static void RunToHome((World World, Village Village, Cat Cat, DungeonSite Site) f, int seconds)
        {
            for (int i = 0; i < seconds * 4 && f.World.IsDungeonExplorer(f.Cat); i++)
                f.World.Step(.25);
            Check(!f.World.IsDungeonExplorer(f.Cat) && f.Cat.Position.Level == 0, "The explorer must come home: phase=" + f.Cat.DungeonPhase + " reason=" + f.Cat.BlockedReason);
        }

        private static void FloorOneLoot()
        {
            var f = Trained(3);
            double research = f.Village.ResearchPoints;
            int items = f.Village.Items.Count;
            Check(f.World.ExploreDungeon(f.Village, f.Cat, f.Site.Id).Success, "Trained cat accepts");
            RunToHome(f, 400);
            Check(f.Village.ResearchPoints - research >= 5 - 1e-9, "Floor 1 must pay research points on unloading: gained " + (f.Village.ResearchPoints - research));
            var weapon = f.Village.Items.Skip(items).FirstOrDefault(i => i.Kind == "weapon" && i.Material == "bone");
            Check(weapon != null && f.Village.Stockpiles.Any(p => p.Id == weapon.LocationId), "Floor 1 must pay a bone weapon delivered to colony storage");
            var loot = f.Village.Events.LastOrDefault(e => e.Kind == "expedition_loot");
            Check(loot != null && loot.EntityId == f.Cat.Id && loot.Text.Contains("floor 1"), "Unloading must post an expedition_loot event naming floor 1");
            Check(f.Village.DeepestLootFloor == 1, "The colony must record floor-1 loot as unloaded");
            Check(Guardian(f.World, f.Site, 1).Health > 0 && f.Site.Floors[1].Loot.Count > 0, "A fight-3 cat must not push on into a Risky floor 2 by itself");
        }

        private static void FloorTwoRelic()
        {
            var f = Fixture();
            double before = f.World.ColonyRelicFactor(f.Village);
            Check(f.World.ExploreDungeon(f.Village, f.Cat, f.Site.Id).Success, "Seasoned cat accepts");
            RunToHome(f, 400);
            var relic = f.Village.Items.FirstOrDefault(i => i.Kind == "relic");
            Check(relic != null && f.Village.Stockpiles.Any(p => p.Id == relic.LocationId), "Floor 2 must pay a named relic into colony storage");
            Check(before == 1 && f.World.ColonyRelicFactor(f.Village) > 1, "A stored relic must give the colony a small combat buff");
            Check(f.Village.DeepestLootFloor == 2, "The colony must record floor-2 loot");
            Check(f.Village.Events.Any(e => e.Kind == "expedition_loot" && e.Text.Contains("floor 2")), "Unloading must name floor 2");
        }

        private static void StrandedExplorer()
        {
            var f = Fixture();
            var at = f.Site.Entrance;
            Place(f.World, f.Cat, at);
            f.Cat.DungeonId = f.Site.Id;
            f.Cat.DungeonPhase = "returning";
            f.Cat.Cargo.Add(new Stack("gem", 2));
            f.Cat.Thirst = 10;
            foreach (var p in new[] { new Int2(at.X - 1, at.Z), new Int2(at.X + 1, at.Z), new Int2(at.X, at.Z - 1), new Int2(at.X, at.Z + 1) })
                f.World.TileAt(p).Wall = true;
            f.World.Step(3);
            Check(f.Cat.DungeonId == "" && !f.World.IsDungeonExplorer(f.Cat), "A blocked route home with critical needs must cancel the expedition");
            Check(f.Village.Events.Any(e => e.Kind == "emergency" && e.EntityId == f.Cat.Id), "The colony must be warned about the stranded explorer");
            Check(f.Village.Stockpiles.Where(p => p.Kind == "spill").Sum(p => World.Amount(p.Goods, "gem")) == 2, "Its cargo must stay recoverable");
        }

        private static void ReleaseContinues()
        {
            var f = Fixture();
            Place(f.World, f.Cat, f.Site.Floors[0].Spawn);
            Control(f.Cat);
            f.World.Step(.05);
            Check(f.Cat.DungeonPhase == "manual", "Fixture: a controlled cat underground is in manual mode");
            f.Cat.ControlledBy = "";
            f.Cat.Goal = "idle";
            f.World.ContinueExpeditionAfterControl(f.Village, f.Cat);
            Check(f.Cat.DungeonPhase == "exploring" && f.Cat.Goal == GoalKey.ExploringDungeon, "Releasing control underground must continue the expedition");
            var guardian = Guardian(f.World, f.Site, 0);
            for (int i = 0; i < 400 && guardian.Health > 0; i++)
                f.World.Step(.25);
            Check(guardian.Health <= 0, "The released cat must carry on and clear the floor");
        }

        private static void GuardianReturnsHome()
        {
            var f = Fixture();
            var guardian = Guardian(f.World, f.Site, 0);
            var home = guardian.Home;
            var away = f.Site.Floors[0].Cells.Where(p => Int2.Distance(p, home) == 3).OrderBy(p => p.X).ThenBy(p => p.Z).First();
            guardian.Position = away;
            guardian.X = away.X;
            guardian.Z = away.Z;
            guardian.Health = 5;
            f.World.Step(40);
            Check(guardian.Position.Equals(home), "An idle guardian must walk back to its post");
            Check(guardian.Health > 5, "An idle guardian at its post must heal");
        }

        private static void RefusalKeepsWork()
        {
            var f = Trained(0);
            ClearFloor(f.World, f.Site, 0);
            var job = new Job { Id = f.World.Id("dungeon-work"), Kind = "hunt", Phase = "working", CatId = f.Cat.Id, Position = f.Cat.Position, RequiredWork = 1000 };
            f.Village.Jobs.Add(job);
            f.Cat.JobId = job.Id;
            Check(!f.World.ExploreDungeon(f.Village, f.Cat, f.Site.Id).Success, "Deadly floor refused");
            Check(f.Cat.JobId == job.Id && !job.Completed && job.CatId == f.Cat.Id, "A refused expedition must not cancel the cat's current work");
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
            // The chest opens only once its guardian is down (W2-09).
            Guardian(f.World, f.Site, 0).Health = 0;
            Place(f.World, f.Cat, floor.LootPosition);
            double finite = floor.Loot.Sum(s => s.Amount);
            var pickup = new GameAction { Kind = "InteractCat", CatId = f.Cat.Id };
            Check(f.World.Apply(identity, pickup).Success, "A controlled cat must collect the reached dungeon chest through ordinary interaction");
            Check(Math.Abs(CarriedTreasure(f.Village, f.Cat) - finite) < 1e-9 && floor.Loot.Count == 0, "Pickup transfers finite chest contents once");
            f.World.Apply(identity, pickup);
            Check(Math.Abs(CarriedTreasure(f.Village, f.Cat) - finite) < 1e-9 && floor.Loot.Count == 0, "Repeated interaction must not recreate loot");
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
            // W2-09: floor 1 is Risky for an untrained cat and may be tried; the deadly second floor is refused.
            var f = Fixture();
            f.Cat.Skills.Clear();
            ClearFloor(f.World, f.Site, 0);
            var position = f.Cat.Position;
            Check(!f.World.ExploreDungeon(f.Village, f.Cat, f.Site.Id).Success, "An untrained cat must refuse a Deadly floor it did not accept");
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
