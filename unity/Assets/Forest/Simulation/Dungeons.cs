using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    [Serializable]
    public sealed class DungeonStair
    {
        public Int2 From, To;
        public List<Int2> Opening = new List<Int2>();
    }

    [Serializable]
    public sealed class DungeonFloor
    {
        public int Level;
        public Int2 Spawn, LootPosition;
        public double Threat, Elevation;
        public List<Int2> Cells = new List<Int2>();
        public List<Stack> Loot = new List<Stack>();

        /// <summary>Game time at which this cleared floor restocks; 0 while it is not cleared.</summary>
        public double RestockAt;

        /// <summary>How often this floor has restocked; unique rewards come only from the first stock.</summary>
        public int Restocks;
    }

    [Serializable]
    public sealed class DungeonSite
    {
        public string Id = "", Name = "", Theme = "root_cavern";
        public Int2 Entrance;
        public int Depth;
        public double Threat, BaseElevation;
        public List<DungeonFloor> Floors = new List<DungeonFloor>();
        public List<DungeonStair> Stairs = new List<DungeonStair>();
        public List<string> RumoredBy = new List<string>();
    }

    [Serializable]
    public sealed class DungeonCreature
    {
        public string Id = "", SiteId = "", Kind = "cave_beetle", TargetCatId = "";
        public Int2 Position, Home;
        public double X, Z, Health, MaxHealth, Power, NextPlanningAt;
        public List<Int2> Path = new List<Int2>();

        /// <summary>Damage per second against an unarmoured, untrained cat; 0 in saves before W2-09 (derived from Power).</summary>
        public double Attack;

        /// <summary>Divides the damage cats deal; 0 in saves before W2-09 (treated as 1).</summary>
        public double Defense;
    }

    public partial class Cat
    {
        public string DungeonId = "", DungeonPhase = "", DungeonTargetId = "", DungeonStorageId = "";
        public int DungeonFloorIndex;
        public double DungeonNextPlanAt;

        /// <summary>The player accepted a Deadly estimate for this trip; only then may a guardian kill the cat.</summary>
        public bool DungeonAcceptedDeadly;

        /// <summary>Research notes carried home from chests; paid into the colony's research points on unloading.</summary>
        public double DungeonLore;

        /// <summary>Deepest floor number (1 = first floor) whose chest this cat opened on the current trip.</summary>
        public int DungeonLootFloor;
    }

    public partial class Village
    {
        /// <summary>Deepest dungeon floor number whose loot an explorer has brought home (0 = none).</summary>
        public int DeepestLootFloor;
    }

    /// <summary>How an expedition is expected to go, from time to kill versus damage taken.</summary>
    public enum DungeonRisk
    {
        Safe,
        Risky,
        Deadly
    }

    public struct DungeonRiskEstimate
    {
        public DungeonRisk Risk;

        /// <summary>Expected health lost before the floor's guardian falls.</summary>
        public double ExpectedHpLoss;

        /// <summary>Expected seconds of fighting to defeat the guardian.</summary>
        public double SecondsToWin;
    }

    /// <summary>Chest contents that are not stored goods. Values are save data: never rename one.</summary>
    public static class DungeonLoot
    {
        /// <summary>Research notes; each unit is one research point.</summary>
        public const string Lore = "lore";
        public const string BoneWeapon = "bone_weapon";
        public const string Relic = "relic";

        /// <summary>The named relic of Root Hollow's second floor.</summary>
        public const string RelicName = "Rootheart Charm";
        public const string RelicMaterial = "rootheart";
    }

    public partial class World
    {
        public List<DungeonSite> Dungeons = new List<DungeonSite>();
        public List<DungeonCreature> Creatures = new List<DungeonCreature>();
        [NonSerialized] private Dictionary<Int2, DungeonFloor> dungeonFloors;
        [NonSerialized] private int dungeonFloorCount = -1;

        private double RootHollowThreat => Pace.RootHollowThreat;
        private double GuardianAttackPerThreat => Pace.GuardianAttackPerThreat;
        private double GuardianDefensePerFloor => Pace.GuardianDefensePerFloor;
        private double FightFloorHealth => Pace.FightFloorHealth;
        private double CombatVariance => Pace.CombatVariance;
        private double SafeLossShare => Pace.SafeLossShare;
        private double GuardianRegenPerSecond => Pace.GuardianRegenPerSecond;
        private double FloorOneLore => Pace.FloorOneLore;
        private double FloorTwoLore => Pace.FloorTwoLore;

        /// <summary>Only a new founding generates sites. Existing saves retain their terrain and identities.</summary>
        public void EnsureVillageDungeon(Village village)
        {
            if (GenerationVersion == 0 || village == null || Dungeons.Any(d => d.RumoredBy.Contains(village.Id)))
                return;
            var entrance = new Int2(village.Center.X, village.Center.Z + village.Radius + 7);
            var approach = Enumerable.Range(village.Radius + 1, 7).Select(z => new Int2(village.Center.X, village.Center.Z + z)).ToArray();
            if (approach.Any(p => !CanModifyTerrain(village.Id, p)) || !DungeonSiteSpace(entrance, village.Id))
                return;
            foreach (var p in approach)
            {
                var tile = TileAt(p);
                tile.Water = tile.Mountain = tile.Wall = false;
                tile.Resource = "";
                tile.Amount = tile.FishCapacity = 0;
                tile.Deposits.Clear();
                tile.Dirt = true;
                tile.Biome = "woodland";
                // The approach joins the village's protected flat founding apron.
                tile.Elevation = 0;
                tile.WaterDepth = 0;
                tile.WaterSurface = 0;
                tile.Landform = p.Equals(entrance) ? "dungeon_entrance" : "trail";
                if (!village.Known.Contains(p))
                    village.Known.Add(p);
            }
            CreateDungeonSite("dungeon:" + village.Id, entrance, village.Id, 2, 0);
            for (int x = -4; x <= 4; x++)
                for (int z = -4; z <= 4; z++)
                {
                    var p = new Int2(entrance.X + x, entrance.Z + z);
                    TileAt(p);
                    if (!village.Known.Contains(p))
                        village.Known.Add(p);
                }
            // A second site in the next regional band establishes distance scaling.
            // Its entrance remains natural terrain: an expedition must find a real route.
            var distant = new Int2(village.Center.X + 48, village.Center.Z + 40);
            if (CanModifyTerrain(village.Id, distant))
            {
                var tile = TileAt(distant);
                if (!tile.Water && !tile.Mountain && !tile.Wall && DungeonSiteSpace(distant, village.Id))
                    CreateDungeonSite("dungeon:far:" + village.Id, distant, village.Id, 3, 2);
            }
        }

        private bool DungeonSiteSpace(Int2 entrance, string villageId) =>
            !Dungeons.Any(d => Math.Abs(d.Entrance.X - entrance.X) < 40 && Math.Abs(d.Entrance.Z - entrance.Z) < 24)
            && CanModifyTerrain(villageId, new Int2(entrance.X - 3, entrance.Z - 10), 25, 21);

        private void CreateDungeonSite(string id, Int2 entrance, string villageId, int depth, int distanceBand)
        {
            if (Dungeons.Any(d => d.Id == id))
                return;
            uint shape = Hash(Seed + ":" + entrance.X + ":" + entrance.Z + ":dungeon");
            double threat = distanceBand == 0 ? RootHollowThreat : Pace.DungeonBaseThreat + distanceBand * Pace.DungeonBandThreat;
            var site = new DungeonSite { Id = id, Name = distanceBand == 0 ? "Root Hollow" : "Deepstone Vault", Theme = distanceBand == 0 ? "root_cavern" : "stone_vault", Entrance = entrance, Depth = depth, Threat = threat, BaseElevation = TileAt(entrance).Elevation };
            site.RumoredBy.Add(villageId);
            Int2 upper = entrance;
            for (int index = 0; index < depth; index++)
            {
                int level = -index - 1;
                int direction = index % 2 == 0 ? 1 : -1;
                var spawn = new Int2(upper.X + direction * 4, upper.Z, level);
                var floor = new DungeonFloor { Level = level, Spawn = spawn, Threat = site.Threat + index * Pace.DungeonFloorThreat, Elevation = site.BaseElevation + level * 4 };
                var stair = new DungeonStair { From = upper, To = spawn };
                for (int step = 1; step <= 4; step++)
                    stair.Opening.Add(new Int2(upper.X + direction * step, upper.Z, upper.Level));
                if (upper.Level < 0)
                    site.Floors.First(f => f.Level == upper.Level).Cells.RemoveAll(p => stair.Opening.Contains(p));
                site.Stairs.Add(stair);
                var cells = new HashSet<Int2>();
                void Room(int cx, int cz, int half)
                {
                    for (int x = -half; x <= half; x++)
                        for (int z = -half; z <= half; z++)
                            cells.Add(new Int2(cx + x, cz + z, level));
                }
                Room(spawn.X + direction * 2, spawn.Z, 2);
                int bend = ((shape >> (index * 2)) & 1) == 0 ? 3 : -3;
                for (int step = 0; step <= 9; step++)
                    cells.Add(new Int2(spawn.X + direction * step, spawn.Z, level));
                for (int z = Math.Min(0, bend); z <= Math.Max(0, bend); z++)
                    cells.Add(new Int2(spawn.X + direction * 9, spawn.Z + z, level));
                Room(spawn.X + direction * 11, spawn.Z + bend, 2);
                floor.LootPosition = new Int2(spawn.X + direction * 12, spawn.Z + bend + 1, level);
                floor.Cells = cells.OrderBy(p => p.Z).ThenBy(p => p.X).ToList();
                floor.Loot = LootTable(site, index, 0);
                site.Floors.Add(floor);
                var guard = new Int2(spawn.X + direction * 10, spawn.Z + bend - 1, level);
                double health = Pace.GuardianBaseHealth + index * Pace.GuardianFloorHealth + distanceBand * Pace.GuardianBandHealth;
                Creatures.Add(new DungeonCreature { Id = id + ":guardian:" + index, SiteId = id, Position = guard, Home = guard, X = guard.X, Z = guard.Z, Health = health, MaxHealth = health, Power = floor.Threat, Attack = floor.Threat * GuardianAttackPerThreat, Defense = 1 + index * GuardianDefensePerFloor, Kind = index == 0 ? "cave_beetle" : "stone_guardian" });
                upper = new Int2(spawn.X + direction * 13, spawn.Z + bend, level);
            }
            Dungeons.Add(site);
            dungeonFloorCount = -1;
            foreach (var aperture in site.Stairs.SelectMany(s => s.Opening).Where(p => p.Level == 0))
            {
                var tile = TileAt(aperture);
                tile.Landform = "dungeon_opening";
                tile.Wall = tile.Water = false;
                tile.Mountain = true;
                tile.WaterDepth = 0;
                tile.Resource = "";
                tile.Amount = tile.FishCapacity = 0;
                tile.Deposits.Clear();
            }
        }

        /// <summary>
        /// What a floor's chest holds. Root Hollow pays research notes and a bone weapon on floor 1 and
        /// its named relic on floor 2 (once; restocks pay notes instead). Other sites keep stored goods.
        /// </summary>
        private List<Stack> LootTable(DungeonSite site, int index, int restocks)
        {
            if (site.Theme == "root_cavern" && index == 0)
                return new List<Stack> { new Stack(DungeonLoot.Lore, FloorOneLore), new Stack(DungeonLoot.BoneWeapon, 1) };
            if (site.Theme == "root_cavern" && index == 1)
                return restocks == 0
                    ? new List<Stack> { new Stack(DungeonLoot.Relic, 1), new Stack("gem", Pace.DungeonLootBase + index), new Stack("stone", Pace.DungeonLootSecondary) }
                    : new List<Stack> { new Stack(DungeonLoot.Lore, FloorTwoLore), new Stack("gem", Pace.DungeonLootBase + index) };
            return new List<Stack> { new Stack(index == 0 ? "ore" : "gem", Pace.DungeonLootBase + index), new Stack(index == 0 ? "herbs" : "stone", Pace.DungeonLootSecondary) };
        }

        private DungeonFloor DungeonFloorAt(Int2 p)
        {
            int count = Dungeons.Sum(d => d.Floors.Count);
            if (dungeonFloors == null || dungeonFloorCount != count)
            {
                dungeonFloors = new Dictionary<Int2, DungeonFloor>();
                foreach (var floor in Dungeons.SelectMany(d => d.Floors))
                    foreach (var cell in floor.Cells)
                        dungeonFloors[cell] = floor;
                dungeonFloorCount = count;
            }
            return dungeonFloors.TryGetValue(p, out var found) ? found : null;
        }

        public Tile DungeonTileAt(Int2 p)
        {
            var floor = DungeonFloorAt(p);
            bool opening = DungeonOpeningAt(p);
            return new Tile { Position = p, Biome = floor == null ? "bedrock" : "cave", Landform = opening ? "dungeon_opening" : "", Wall = floor == null && !opening, Mountain = floor == null, Dirt = floor != null, Elevation = floor?.Elevation ?? p.Level * 4 };
        }

        public bool DungeonOpeningAt(Int2 p) => Dungeons.Any(d => d.Stairs.Any(s => s.Opening.Contains(p)));

        public IEnumerable<Int2> DungeonNeighbors(Int2 p)
        {
            foreach (var stair in Dungeons.SelectMany(d => d.Stairs))
                if (stair.From.Equals(p))
                    yield return stair.To;
                else if (stair.To.Equals(p))
                    yield return stair.From;
        }

        public bool DungeonEdge(Int2 a, Int2 b)
        {
            if (a.Level == b.Level)
                return a.Level == 0 || DungeonFloorAt(a) != null && DungeonFloorAt(b) != null && Int2.Distance(a, b) == 1;
            return Dungeons.SelectMany(d => d.Stairs).Any(s => s.From.Equals(a) && s.To.Equals(b) || s.From.Equals(b) && s.To.Equals(a));
        }

        public double DungeonHeight(Int2 p) => p.Level == 0 ? WalkHeight(p) : DungeonFloorAt(p)?.Elevation ?? (Dungeons.FirstOrDefault(d => d.Stairs.Any(s => s.Opening.Contains(p)))?.BaseElevation ?? 0) + p.Level * 4;
        public bool IsDungeonExplorer(Cat cat) => cat != null && (cat.DungeonId != "" || cat.Position.Level < 0);
        public static void CancelDungeon(Cat cat)
        {
            cat.DungeonId = cat.DungeonPhase = cat.DungeonTargetId = cat.DungeonStorageId = "";
            cat.DungeonFloorIndex = 0;
            cat.DungeonNextPlanAt = 0;
            cat.DungeonAcceptedDeadly = false;
        }

        /// <summary>
        /// The legacy readiness number (weapon, skill, health and needs). Kept for display only; dispatch
        /// and combat use <see cref="RiskOf"/>.
        /// </summary>
        public double DungeonReadiness(Village village, Cat cat) => cat == null ? 0 : EquipmentPower(village, cat, "weapon") * (1 + Amount(cat.Skills, "fight") * Pace.ReadinessFightSkillFactor) * Math.Clamp(cat.Health / 100, 0, 1) * Math.Min(1, Math.Min(cat.Hunger, Math.Min(cat.Thirst, cat.Rest)) / Pace.ReadinessNeedNorm);

        // ---- One combat model for guardians and explorers ----

        private double FightSkillFactor(Cat cat) => 1 + Amount(cat.Skills, "fight") * Pace.ReadinessFightSkillFactor;

        /// <summary>A cat's offence: weapon, fighting skill, colony combat effects and the relic.</summary>
        private double CatProwess(Village v, Cat cat) => EquipmentPower(v, cat, "weapon") * FightSkillFactor(cat) * Catalog.Effect(this, v, "combatPower", 1) * Catalog.Effect(this, v, "combatPowerMult", 1) * ColonyRelicFactor(v);

        /// <summary>A cat's guard: armour, colony defence effects and fighting skill.</summary>
        private double CatGuard(Village v, Cat cat) => Math.Max(1, EquipmentPower(v, cat, "armor") * Catalog.Effect(this, v, "defensePower", 1) * Catalog.Effect(this, v, "defenseMult", 1)) * FightSkillFactor(cat);

        private double GuardianAttack(DungeonCreature creature) => creature.Attack > 0 ? creature.Attack : creature.Power * GuardianAttackPerThreat;
        private static double GuardianDefense(DungeonCreature creature) => creature.Defense > 0 ? creature.Defense : 1;

        /// <summary>A seeded blow multiplier in [1 - variance, 1 + variance] for this actor and half-second.</summary>
        private double BlowVariance(string actorId) => 1 + CombatVariance * (2 * (Hash(Seed + ":" + actorId + ":" + (long)Math.Floor(TimeSeconds * 2)) / 4294967296.0) - 1);

        private DungeonSite SiteOf(DungeonFloor floor) => Dungeons.FirstOrDefault(d => d.Floors.Contains(floor));

        private IEnumerable<DungeonCreature> FloorGuardians(DungeonSite site, DungeonFloor floor) => Creatures.Where(e => e.SiteId == site.Id && e.Home.Level == floor.Level);

        /// <summary>True while any guardian of the floor is alive; its chest stays shut until then.</summary>
        public bool DungeonChestLocked(DungeonFloor floor)
        {
            var site = SiteOf(floor);
            return site != null && FloorGuardians(site, floor).Any(e => e.Health > 0);
        }

        private bool FloorCleared(DungeonSite site, DungeonFloor floor) => !FloorGuardians(site, floor).Any(e => e.Health > 0) && !floor.Loot.Any(s => s.Amount > 0);

        /// <summary>Index of the first floor that still holds a living guardian or loot; -1 when the site is cleared.</summary>
        public int FirstUnclearedFloor(DungeonSite site) => site == null ? -1 : site.Floors.FindIndex(f => !FloorCleared(site, f));

        /// <summary>
        /// Outcome estimate for this cat fighting the floor's guardian now: the expected seconds to win and
        /// the health it costs. Deadly when the loss would take the cat below the fight floor; Safe when it
        /// costs at most a quarter of the cat's current health; otherwise Risky.
        /// </summary>
        public DungeonRiskEstimate RiskOf(Cat cat, DungeonFloor floor)
        {
            var village = cat == null ? null : Village(cat.VillageId);
            var site = floor == null ? null : SiteOf(floor);
            if (village == null || site == null)
                return new DungeonRiskEstimate { Risk = DungeonRisk.Deadly, ExpectedHpLoss = double.PositiveInfinity, SecondsToWin = double.PositiveInfinity };
            double seconds = 0, loss = 0;
            foreach (var guardian in FloorGuardians(site, floor).Where(e => e.Health > 0))
            {
                double strike = Pace.ExplorerStrikePerSecond * CatProwess(village, cat) / GuardianDefense(guardian);
                double time = guardian.Health / Math.Max(1e-9, strike);
                seconds += time;
                loss += time * GuardianAttack(guardian) / CatGuard(village, cat);
            }
            var risk = loss >= cat.Health - FightFloorHealth ? DungeonRisk.Deadly : loss > cat.Health * SafeLossShare ? DungeonRisk.Risky : DungeonRisk.Safe;
            return new DungeonRiskEstimate { Risk = risk, ExpectedHpLoss = loss, SecondsToWin = seconds };
        }

        private static bool NeedsCritical(Cat cat, double need) => cat.Hunger < need || cat.Thirst < need || cat.Rest < need;

        // ---- Dispatch, recall and direct control ----

        public ActionResult ExploreDungeon(Village village, Cat cat, string siteId, bool acceptDeadly = false)
        {
            // Every precondition is checked before the first mutation.
            var site = Dungeons.Find(d => d.Id == siteId && d.RumoredBy.Contains(village.Id));
            if (cat == null || !cat.Alive || !village.Cats.Contains(cat))
                return ActionResult.Fail(ResultCode.UnknownTarget, "Choose a living cat in this village", ("target", cat?.Id ?? ""));
            if (cat.ControlledBy != "")
                return ActionResult.Fail(ResultCode.ReleaseControl, "Release direct control first", ("cat", cat.Id));
            if (cat.AgeHours < Pace.WorkerMinAgeHours)
                return ActionResult.Fail(ResultCode.Kitten, "Kittens cannot join expeditions", ("cat", cat.Id));
            if (cat.Migration == "arriving" || cat.Migration == "departing")
                return ActionResult.Fail(ResultCode.Migrating, "This cat is still arriving or leaving", ("cat", cat.Id));
            if (site == null)
                return ActionResult.Fail(ResultCode.UnknownTarget, "No known dungeon entrance", ("target", siteId ?? ""));
            if (IsDungeonExplorer(cat))
                return ActionResult.Fail(ResultCode.AlreadyExploring, "Recall the current expedition first", ("cat", cat.Id));
            if (village.Routes.Any(r => r.CatId == cat.Id))
                return ActionResult.Fail(ResultCode.TransportReturning, "The cat must finish physically returning its transport first", ("cat", cat.Id));
            if (cat.Health < Pace.ExpeditionMinHealth)
                return ActionResult.Fail(ResultCode.NeedsRest, "This cat is too hurt for an expedition", ("need", "health"));
            if (NeedsCritical(cat, Pace.ExpeditionMinNeed))
                return ActionResult.Fail(ResultCode.NeedsRest, "This cat must eat, drink and rest before an expedition", ("need", cat.Hunger < Pace.ExpeditionMinNeed ? "hunger" : cat.Thirst < Pace.ExpeditionMinNeed ? "thirst" : "rest"));
            int index = FirstUnclearedFloor(site);
            if (index < 0)
            {
                double restock = site.Floors.Where(f => f.RestockAt > 0).Select(f => f.RestockAt - TimeSeconds).DefaultIfEmpty(Pace.FloorRestockSeconds).Min();
                return ActionResult.Fail(ResultCode.DungeonCleared, "Nothing left; " + site.Name + " restocks soon", ("seconds", Math.Max(0, Math.Ceiling(restock)).ToString(System.Globalization.CultureInfo.InvariantCulture)));
            }
            var estimate = RiskOf(cat, site.Floors[index]);
            if (estimate.Risk == DungeonRisk.Deadly && !acceptDeadly)
                return ActionResult.Fail(ResultCode.DungeonDeadly, "This expedition could kill " + cat.Name + "; accept the risk to send them anyway", ("floor", (index + 1).ToString(System.Globalization.CultureInfo.InvariantCulture)), ("loss", Math.Round(estimate.ExpectedHpLoss).ToString(System.Globalization.CultureInfo.InvariantCulture)));
            if (Path(cat.Position, site.Entrance, village) == null)
                return ActionResult.Fail(Blocked.BlockedRoute, "The dungeon entrance has no accessible surface route", ("target", site.Id));
            CancelWork(village, cat);
            if (cat.BuildingId != "")
                return ActionResult.Fail(ResultCode.TransportReturning, "The cat must finish physically returning its transport first", ("cat", cat.Id));
            cat.DungeonId = site.Id;
            cat.DungeonPhase = "exploring";
            cat.DungeonFloorIndex = index;
            cat.DungeonTargetId = "";
            cat.DungeonAcceptedDeadly = estimate.Risk == DungeonRisk.Deadly;
            cat.BlockedReason = "";
            cat.Goal = GoalKey.ExploringDungeon;
            Note(village, EventKind.Dungeon, cat.Name + " set out for " + site.Name, cat.Id);
            return ActionResult.Ok(cat.Id);
        }

        public ActionResult RecallExplorer(Village village, Cat cat)
        {
            if (cat == null || !cat.Alive || !village.Cats.Contains(cat) || !IsDungeonExplorer(cat))
                return ActionResult.Fail("Choose a living explorer in this village");
            cat.DungeonPhase = "returning";
            cat.DungeonTargetId = "";
            cat.Goal = GoalKey.ReturningFromDungeon;
            return ActionResult.Ok(cat.Id);
        }

        public ActionResult AttackCreature(Village village, Cat cat, string creatureId)
        {
            var creature = Creatures.Find(e => e.Id == creatureId && e.Health > 0);
            if (cat == null || !cat.Alive || !village.Cats.Contains(cat) || cat.ControlledBy == "" || creature == null || !InDungeonCombatRange(cat, creature))
                return ActionResult.Fail("Control this village's cat beside a living dungeon enemy");
            cat.DungeonId = creature.SiteId;
            cat.DungeonPhase = "manual";
            cat.DungeonTargetId = creature.Id;
            return ActionResult.Ok(creature.Id);
        }

        /// <summary>
        /// Opens the chest on or beside a cat's tile once its guardian is down. Direct control's
        /// interaction calls this; autonomous explorers use the same transfer.
        /// </summary>
        public ActionResult TakeDungeonChest(Village v, Cat c)
        {
            if (v == null || c == null || !c.Alive || c.Position.Level >= 0)
                return ActionResult.Fail(ResultCode.NoChestInReach, "Walk to a dungeon chest or select a nearby pile");
            var floor = Dungeons.SelectMany(d => d.Floors).FirstOrDefault(f => f.LootPosition.Level == c.Position.Level && Math.Abs(f.LootPosition.X - c.Position.X) + Math.Abs(f.LootPosition.Z - c.Position.Z) <= 1 && f.Loot.Any(s => s.Amount > 0));
            if (floor == null)
                return ActionResult.Fail(ResultCode.NoChestInReach, "Walk to a dungeon chest or select a nearby pile");
            if (DungeonChestLocked(floor))
                return ActionResult.Fail(ResultCode.ChestGuarded, "The guardian still watches this chest");
            OpenChest(v, c, floor);
            return ActionResult.Ok(ResultCode.Collected, c.Id);
        }

        /// <summary>Moves a chest's finite contents once: notes to the cat's lore, weapons and relics as carried items, goods to cargo.</summary>
        private void OpenChest(Village v, Cat c, DungeonFloor floor)
        {
            foreach (var stack in floor.Loot.ToArray())
            {
                double take = stack.Resource == DungeonLoot.Lore || IsTreasureItem(stack.Resource)
                    ? stack.Amount
                    : Math.Min(stack.Amount, Math.Max(0, CarryCapacity(v, stack.Resource) - Amount(c.Cargo, stack.Resource)));
                if (take <= 0)
                    continue;
                if (stack.Resource == DungeonLoot.Lore)
                    c.DungeonLore += take;
                else if (IsTreasureItem(stack.Resource))
                    for (int i = 0; i < (int)Math.Round(take); i++)
                        v.Items.Add(TreasureItem(v, stack.Resource, c.Id));
                else
                    Add(c.Cargo, stack.Resource, take);
                Add(floor.Loot, stack.Resource, -take);
            }
            c.DungeonLootFloor = Math.Max(c.DungeonLootFloor, -floor.Level);
        }

        private static bool IsTreasureItem(string resource) => resource == DungeonLoot.BoneWeapon || resource == DungeonLoot.Relic;

        private Item TreasureItem(Village v, string resource, string locationId) => resource == DungeonLoot.Relic
            ? new Item { Id = Id("item"), Kind = "relic", Material = DungeonLoot.RelicMaterial, Quality = 3, VillageId = v.Id, LocationId = locationId, Condition = Pace.ItemBaseCondition, MaxCondition = Pace.ItemBaseCondition }
            : new Item { Id = Id("item"), Kind = "weapon", Material = "bone", Quality = 1, VillageId = v.Id, LocationId = locationId, Condition = Pace.ItemBaseCondition, MaxCondition = Pace.ItemBaseCondition };

        /// <summary>
        /// Direct control: makes both ends of every stair beside the cat real tiles and known, then
        /// reveals the cells around it. W2-INT-a calls this from the MoveCat handler.
        /// </summary>
        public void MaterialiseStairEndpoints(Village v, Cat c)
        {
            if (v == null || c == null)
                return;
            bool nearStair = false;
            foreach (var site in Dungeons)
                foreach (var stair in site.Stairs.Where(s => Int2.Distance(s.From, c.Position) <= 1 || Int2.Distance(s.To, c.Position) <= 1))
                    foreach (var end in new[] { stair.From, stair.To })
                    {
                        nearStair = true;
                        TileAt(end);
                        if (!v.Known.Contains(end))
                            v.Known.Add(end);
                    }
            if (nearStair && c.Position.Level < 0)
            {
                var here = Dungeons.Find(d => d.Floors.Any(f => f.Level == c.Position.Level && f.Cells.Contains(c.Position)));
                if (here != null)
                    RevealAround(v, c, here);
            }
        }

        /// <summary>
        /// When the player releases a cat underground, it carries on exploring from the floor it stands
        /// on, or heads home when it is hurt or needy. W2-INT-a calls this from the LeaveCatControl handler.
        /// </summary>
        public void ContinueExpeditionAfterControl(Village v, Cat c)
        {
            if (v == null || c == null || !c.Alive || c.ControlledBy != "" || !IsDungeonExplorer(c))
                return;
            var site = Dungeons.Find(d => d.Id == c.DungeonId) ?? Dungeons.Find(d => d.Floors.Any(f => f.Cells.Contains(c.Position)));
            if (site == null)
                return;
            c.DungeonId = site.Id;
            c.DungeonTargetId = "";
            if (c.Health < Pace.ExpeditionRetreatHealth || NeedsCritical(c, Pace.ExpeditionRetreatNeed))
            {
                RetreatDungeon(c, "expedition_return_for_safety");
                return;
            }
            int here = site.Floors.FindIndex(f => f.Level == c.Position.Level);
            int next = FirstUnclearedFloor(site);
            int index = here >= 0 && !FloorCleared(site, site.Floors[here]) ? here : next >= 0 && next >= here ? next : -1;
            if (index < 0)
            {
                RetreatDungeon(c, "");
                return;
            }
            c.DungeonFloorIndex = index;
            c.DungeonPhase = "exploring";
            c.BlockedReason = "";
            c.Goal = GoalKey.ExploringDungeon;
        }

        private bool InDungeonCombatRange(Cat cat, DungeonCreature creature) => cat.Position.Level == creature.Position.Level && Walkable(cat.Position) && Walkable(creature.Position) && Math.Abs(cat.X - creature.X) + Math.Abs(cat.Z - creature.Z) <= 1.1 && (cat.Position.Equals(creature.Position) || Crossable(cat.Position, creature.Position));

        private void RetreatDungeon(Cat cat, string reason)
        {
            cat.DungeonPhase = "returning";
            cat.DungeonTargetId = "";
            cat.BlockedReason = reason;
            cat.Goal = GoalKey.ReturningFromDungeon;
        }

        /// <summary>Reveals and materialises the cells of the cat's floor within reveal distance.</summary>
        private void RevealAround(Village village, Cat cat, DungeonSite site)
        {
            foreach (var cell in site.Floors.Where(f => f.Level == cat.Position.Level).SelectMany(f => f.Cells).Where(p => Int2.Distance(p, cat.Position) <= Pace.DungeonRevealTiles))
            {
                // Known cells must be real tiles, or the floor and its walls never render.
                TileAt(cell);
                if (!village.Known.Contains(cell))
                    village.Known.Add(cell);
            }
        }

        // ---- The expedition tick ----

        public void TickDungeons(double dt, bool planning)
        {
            foreach (var village in Villages)
                foreach (var cat in village.Cats.Where(c => c.Alive && c.Health > 0 && IsDungeonExplorer(c)).ToArray())
                {
                    BeginActor(cat);
                    var site = Dungeons.Find(d => d.Id == cat.DungeonId) ?? Dungeons.Find(d => d.Floors.Any(f => f.Cells.Contains(cat.Position)));
                    if (site == null)
                        continue;
                    if (cat.DungeonId == "")
                    {
                        if (cat.JobId != "" || cat.BuildingId != "")
                            CancelWork(village, cat, preserveUnassignedCargo: true);
                        cat.DungeonId = site.Id;
                        cat.DungeonPhase = cat.ControlledBy == "" ? "returning" : "manual";
                    }
                    if (planning)
                        RevealAround(village, cat, site);
                    if (cat.ControlledBy != "")
                    {
                        var target = Creatures.Find(e => e.Id == cat.DungeonTargetId && e.Health > 0);
                        if (target != null && InDungeonCombatRange(cat, target))
                            FightDungeon(village, cat, target);
                        continue;
                    }
                    var committed = Creatures.Find(e => e.Id == cat.DungeonTargetId && e.Health > 0 && e.Position.Level == cat.Position.Level);
                    bool active = cat.DungeonPhase != "returning" && cat.DungeonPhase != "unloading";
                    if (cat.DungeonPhase == "manual")
                        RetreatDungeon(cat, "expedition_return_for_safety");
                    else if (active && cat.Health < FightFloorHealth)
                        RetreatDungeon(cat, Blocked.ExpeditionWounded);
                    // A fight under way finishes unless the cat falls below the fight floor.
                    else if (active && committed == null && NeedsCritical(cat, Pace.ExpeditionRetreatNeed))
                        RetreatDungeon(cat, "expedition_return_for_safety");
                    if (cat.DungeonPhase == "returning" || cat.DungeonPhase == "unloading")
                    {
                        ReturnDungeon(village, cat, dt);
                        continue;
                    }
                    var floor = site.Floors[Math.Clamp(cat.DungeonFloorIndex, 0, site.Floors.Count - 1)];
                    if (cat.Position.Level != floor.Level)
                    {
                        cat.Goal = GoalKey.DescendingDungeonStairs;
                        MaterialiseStairEndpoints(village, cat);
                        Move(cat, floor.Spawn, dt);
                        continue;
                    }
                    var enemy = committed ?? Creatures.Where(e => e.SiteId == site.Id && e.Position.Level == floor.Level && e.Health > 0).OrderBy(e => Int2.Distance(cat.Position, e.Position)).ThenBy(e => e.Id, StringComparer.Ordinal).FirstOrDefault();
                    if (enemy != null)
                    {
                        if (committed == null)
                        {
                            // Before engaging, the cat re-reads the odds with its current health.
                            var estimate = RiskOf(cat, floor);
                            if (estimate.Risk == DungeonRisk.Deadly && !cat.DungeonAcceptedDeadly)
                            {
                                RetreatDungeon(cat, "expedition_enemy_too_dangerous");
                                continue;
                            }
                        }
                        cat.Goal = GoalKey.FightingDungeonGuardian;
                        if (InDungeonCombatRange(cat, enemy))
                        {
                            cat.DungeonTargetId = enemy.Id;
                            FightDungeon(village, cat, enemy);
                        }
                        else
                            Move(cat, enemy.Position, dt);
                        if (cat.BlockedReason == Blocked.BlockedRoute)
                            RetreatDungeon(cat, "expedition_guardian_unreachable");
                        continue;
                    }
                    cat.DungeonTargetId = "";
                    if (floor.Loot.Any(s => s.Amount > 0))
                    {
                        cat.Goal = GoalKey.CollectingDungeonSupplies;
                        if (!Move(cat, floor.LootPosition, dt))
                            continue;
                        // Goods move once from this authoritative chest into this cat's keeping.
                        OpenChest(village, cat, floor);
                    }
                    int next = cat.DungeonFloorIndex + 1;
                    if (next < site.Floors.Count && !FloorCleared(site, site.Floors[next]) && cat.Cargo.Sum(s => s.Amount) < Pace.ExpeditionDescendCargoLimit && RiskOf(cat, site.Floors[next]).Risk == DungeonRisk.Safe)
                    {
                        cat.DungeonFloorIndex = next;
                        cat.Path.Clear();
                    }
                    else
                        RetreatDungeon(cat, "");
                }
            EndActor();
            foreach (var creature in Creatures.Where(e => e.Health > 0))
                TickDungeonCreature(creature, dt, planning);
            if (planning)
                TickFloorRestock();
        }

        private void FightDungeon(Village village, Cat cat, DungeonCreature creature)
        {
            double seconds = SpendActorTime(cat);
            if (seconds <= 0 || cat.Health < FightFloorHealth)
                return;
            creature.Health = Math.Max(0, creature.Health - seconds * Pace.ExplorerStrikePerSecond * CatProwess(village, cat) / GuardianDefense(creature) * BlowVariance(cat.Id));
            GainSkill(cat, "fight", seconds / Pace.ExplorerSkillSecondsPerPoint);
            foreach (var item in village.Items.Where(i => cat.Equipment.Contains(i.Id)))
                item.Condition = Math.Max(0, item.Condition - seconds * Pace.ExplorerWeaponWearPerSecond);
            if (creature.Health <= 0)
            {
                creature.TargetCatId = "";
                creature.Path.Clear();
                if (cat.DungeonTargetId == creature.Id)
                    cat.DungeonTargetId = "";
                Note(village, EventKind.Dungeon, cat.Name + " defeated a " + creature.Kind.Replace('_', ' '), creature.Id);
            }
        }

        /// <summary>A guardian may hunt this cat: on its level, within its leash, and not a wounded cat it lets flee.</summary>
        private bool GuardianMayTarget(DungeonCreature creature, Cat cat) =>
            cat != null && cat.Alive && cat.Position.Level == creature.Position.Level && Int2.Distance(cat.Position, creature.Home) <= Pace.CreatureLeashTiles
            && (cat.Health >= FightFloorHealth || cat.DungeonAcceptedDeadly);

        private void TickDungeonCreature(DungeonCreature creature, double dt, bool planning)
        {
            var target = Cat(creature.TargetCatId);
            if (!GuardianMayTarget(creature, target))
            {
                target = null;
                creature.TargetCatId = "";
                if (TimeSeconds + 1e-9 >= creature.NextPlanningAt)
                {
                    creature.NextPlanningAt = Math.Floor(TimeSeconds) + Pace.CreatureReplanSeconds;
                    target = Villages.SelectMany(v => v.Cats).Where(c => GuardianMayTarget(creature, c) && Int2.Distance(c.Position, creature.Position) <= Pace.CreatureAggroTiles).OrderBy(c => Int2.Distance(c.Position, creature.Position)).ThenBy(c => c.Id, StringComparer.Ordinal).FirstOrDefault();
                }
                if (target != null)
                    creature.TargetCatId = target.Id;
            }
            if (target == null)
            {
                ReturnGuardianHome(creature, dt, planning);
                return;
            }
            var village = Village(target.VillageId);
            if (InDungeonCombatRange(target, creature))
            {
                target.Health -= dt * GuardianAttack(creature) * BlowVariance(creature.Id) / CatGuard(village, target);
                if (target.Health <= 0)
                {
                    // Only an explicitly accepted Deadly expedition can cost a cat its life.
                    if (target.DungeonAcceptedDeadly)
                    {
                        target.Health = 0;
                        Die(village, target, "dungeon");
                    }
                    else
                        target.Health = 1;
                }
                return;
            }
            // A guardian keeps pace with the cat it hunts, so direct control cannot outrun it in its room.
            double speed = Math.Max(Pace.CreatureSpeed, CatMovementSpeed(village, TileAt(target.Position)));
            StepCreature(creature, target.Position, speed, dt, homeward: false, planning);
        }

        private void ReturnGuardianHome(DungeonCreature creature, double dt, bool planning)
        {
            if (creature.Position.Equals(creature.Home) && AtTravelPoint(creature.X, creature.Z, creature.Home))
            {
                creature.Path.Clear();
                creature.Health = Math.Min(creature.MaxHealth, creature.Health + dt * GuardianRegenPerSecond);
                return;
            }
            StepCreature(creature, creature.Home, Pace.CreatureSpeed, dt, homeward: true, planning);
        }

        /// <summary>Moves a creature along a same-level path. A chase replans on the creature's own cadence; the walk home replans on planning ticks.</summary>
        private void StepCreature(DungeonCreature creature, Int2 destination, double speed, double dt, bool homeward, bool planning)
        {
            bool invalid = creature.Path.Count == 0 || !creature.Path[creature.Path.Count - 1].Equals(destination) || !Walkable(creature.Path[0]) || !Crossable(creature.Position, creature.Path[0]);
            if (invalid)
            {
                if (!AtTravelPoint(creature.X, creature.Z, creature.Position))
                {
                    AdvanceTraveler(ref creature.X, ref creature.Z, creature.Position, speed, dt);
                    return;
                }
                if (creature.Position.Equals(destination))
                {
                    creature.Path.Clear();
                    return;
                }
                if (homeward ? !planning : TimeSeconds + 1e-9 < creature.NextPlanningAt)
                    return;
                if (!homeward)
                    creature.NextPlanningAt = Math.Floor(TimeSeconds) + Pace.CreatureReplanSeconds;
                creature.Path = Path(creature.Position, destination) ?? new List<Int2>();
                if (creature.Path.Any(p => p.Level != creature.Position.Level))
                    creature.Path.Clear();
            }
            if (creature.Path.Count > 0 && Walkable(creature.Path[0]) && Crossable(creature.Position, creature.Path[0]) && AdvanceTraveler(ref creature.X, ref creature.Z, creature.Path[0], speed, dt))
            {
                creature.Position = creature.Path[0];
                creature.Path.RemoveAt(0);
            }
        }

        /// <summary>A cleared floor (guardian down, chest empty) restocks after FloorRestockSeconds once no cat stands on it.</summary>
        private void TickFloorRestock()
        {
            foreach (var site in Dungeons)
                for (int index = 0; index < site.Floors.Count; index++)
                {
                    var floor = site.Floors[index];
                    if (!FloorCleared(site, floor))
                    {
                        floor.RestockAt = 0;
                        continue;
                    }
                    if (floor.RestockAt <= 0)
                    {
                        floor.RestockAt = TimeSeconds + Pace.FloorRestockSeconds;
                        continue;
                    }
                    if (TimeSeconds + 1e-9 < floor.RestockAt || Villages.Any(v => v.Cats.Any(c => c.Alive && c.Position.Level == floor.Level && floor.Cells.Contains(c.Position))))
                        continue;
                    foreach (var guardian in FloorGuardians(site, floor))
                    {
                        guardian.Health = guardian.MaxHealth;
                        guardian.Position = guardian.Home;
                        guardian.X = guardian.Home.X;
                        guardian.Z = guardian.Home.Z;
                        guardian.Path.Clear();
                        guardian.TargetCatId = "";
                        guardian.NextPlanningAt = 0;
                    }
                    floor.Restocks++;
                    floor.Loot = LootTable(site, index, floor.Restocks);
                    floor.RestockAt = 0;
                    foreach (var village in Villages.Where(v => site.RumoredBy.Contains(v.Id)))
                        Note(village, EventKind.Dungeon, site.Name + " stirs: floor " + (index + 1) + " has a new guardian and a full chest", site.Id);
                }
        }

        // ---- Coming home ----

        private void ReturnDungeon(Village village, Cat cat, double dt)
        {
            if (cat.DungeonPhase != "unloading")
            {
                if (!Move(cat, village.Center, dt))
                {
                    if (cat.BlockedReason == Blocked.BlockedRoute && (cat.Health < Pace.ExpeditionGiveUpHealth || NeedsCritical(cat, Pace.ExpeditionGiveUpNeed)))
                        StrandExplorer(village, cat);
                    return;
                }
                cat.DungeonPhase = "unloading";
                cat.DungeonStorageId = "";
                PayExpeditionLoot(village, cat);
            }
            if (cat.Cargo.Count == 0)
            {
                CancelDungeon(cat);
                cat.Goal = GoalKey.Idle;
                cat.BlockedReason = "";
                Note(village, EventKind.Dungeon, cat.Name + " returned from the dungeon", cat.Id);
                return;
            }
            var cargo = cat.Cargo.FirstOrDefault(s => s.Amount > 0);
            if (cargo == null)
                return;
            bool Accepts(Stockpile pile) => pile.Kind == "storage" && HasRoom(village, pile, cargo.Resource, cargo.Amount);
            var store = village.Stockpiles.Find(p => p.Id == cat.DungeonStorageId && Accepts(p));
            if (store == null)
            {
                if (TimeSeconds + 1e-9 < cat.DungeonNextPlanAt)
                    return;
                cat.DungeonNextPlanAt = Math.Floor(TimeSeconds) + Pace.DungeonReplanSeconds;
                store = village.Stockpiles.Where(Accepts).OrderBy(p => Int2.Distance(cat.Position, p.Position)).FirstOrDefault(p => Path(cat.Position, p.Position, village) != null);
                cat.DungeonStorageId = store?.Id ?? "";
            }
            if (store == null)
            {
                cat.BlockedReason = "expedition_storage_required";
                if (cat.Health < Pace.ExpeditionGiveUpHealth || NeedsCritical(cat, Pace.ExpeditionGiveUpNeed))
                {
                    Spill(village, cat.Position, cat.Cargo);
                    CancelDungeon(cat);
                    cat.Goal = GoalKey.Idle;
                    Note(village, EventKind.Dungeon, cat.Name + " left recovered supplies beside the full storage and sought care", cat.Id);
                }
                return;
            }
            if (!Move(cat, store.Position, dt))
            {
                if (cat.BlockedReason == Blocked.BlockedRoute)
                    cat.DungeonStorageId = "";
                return;
            }
            double quantity = cargo.Amount;
            Add(store.Goods, cargo.Resource, quantity);
            Add(cat.Cargo, cargo.Resource, -quantity);
        }

        /// <summary>
        /// On arriving home: research notes become research points, carried weapons and relics go into
        /// storage, and one expedition_loot event names the floor and the totals.
        /// </summary>
        private void PayExpeditionLoot(Village village, Cat cat)
        {
            // Chest tokens picked up as cargo (by an unwired direct-control interaction) count the same.
            foreach (var stack in cat.Cargo.Where(s => s.Resource == DungeonLoot.Lore || IsTreasureItem(s.Resource)).ToArray())
            {
                if (stack.Resource == DungeonLoot.Lore)
                    cat.DungeonLore += stack.Amount;
                else
                    for (int i = 0; i < (int)Math.Round(stack.Amount); i++)
                        village.Items.Add(TreasureItem(village, stack.Resource, cat.Id));
                cat.Cargo.Remove(stack);
            }
            double research = cat.DungeonLore;
            village.ResearchPoints += research;
            cat.DungeonLore = 0;
            var store = village.Stockpiles.Where(p => p.Kind == "storage").OrderBy(p => Int2.Distance(cat.Position, p.Position)).ThenBy(p => p.Id, StringComparer.Ordinal).FirstOrDefault();
            var carried = village.Items.Where(i => i.LocationId == cat.Id && !cat.Equipment.Contains(i.Id)).ToList();
            if (store != null)
                foreach (var item in carried)
                    item.LocationId = store.Id;
            double goods = cat.Cargo.Sum(s => s.Amount);
            int floor = cat.DungeonLootFloor;
            cat.DungeonLootFloor = 0;
            if (research <= 0 && carried.Count == 0 && goods <= 0)
                return;
            village.DeepestLootFloor = Math.Max(village.DeepestLootFloor, floor);
            var site = Dungeons.Find(d => d.Id == cat.DungeonId);
            var parts = new List<string>();
            if (research > 0)
                parts.Add(Math.Round(research).ToString(System.Globalization.CultureInfo.InvariantCulture) + " research");
            int weapons = carried.Count(i => i.Kind == "weapon"), relics = carried.Count(i => i.Kind == "relic");
            if (weapons > 0)
                parts.Add(weapons == 1 ? "a bone weapon" : weapons + " bone weapons");
            if (relics > 0)
                parts.Add("the " + DungeonLoot.RelicName);
            if (goods > 0)
                parts.Add(Math.Round(goods).ToString(System.Globalization.CultureInfo.InvariantCulture) + " goods");
            string from = floor > 0 ? " from " + (site?.Name ?? "the dungeon") + " floor " + floor : " from " + (site?.Name ?? "the dungeon");
            Note(village, EventKind.ExpeditionLoot, cat.Name + " brought home " + string.Join(", ", parts) + from, cat.Id);
        }

        /// <summary>A returning explorer with no route home and critical needs gives up the trip and the colony is warned.</summary>
        private void StrandExplorer(Village village, Cat cat)
        {
            bool warned = village.Events.Any(e => e.Kind == EventKind.Emergency && e.EntityId == cat.Id && e.Time >= TimeSeconds - 300);
            if (cat.Position.Level < 0)
            {
                // Underground there is nowhere else to go: keep trying the stair and warn once.
                if (!warned)
                    Note(village, EventKind.Emergency, cat.Name + " is trapped underground and needs a way home", cat.Id);
                return;
            }
            // The notes are knowledge the cat already brought back; weapons and relics stay in its paws.
            village.ResearchPoints += cat.DungeonLore;
            cat.DungeonLore = 0;
            cat.DungeonLootFloor = 0;
            Spill(village, cat.Position, cat.Cargo);
            CancelDungeon(cat);
            cat.Path.Clear();
            cat.Goal = GoalKey.Idle;
            cat.BlockedReason = Blocked.ExpeditionStranded;
            if (!warned)
                Note(village, EventKind.Emergency, cat.Name + " could not find a way home from the expedition and stopped to tend its needs", cat.Id);
        }
    }
}
