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
    }

    public partial class Cat
    {
        public string DungeonId = "", DungeonPhase = "", DungeonTargetId = "", DungeonStorageId = "";
        public int DungeonFloorIndex;
        public double DungeonNextPlanAt;
    }

    public partial class World
    {
        public List<DungeonSite> Dungeons = new List<DungeonSite>();
        public List<DungeonCreature> Creatures = new List<DungeonCreature>();
        [NonSerialized] private Dictionary<Int2, DungeonFloor> dungeonFloors;
        [NonSerialized] private int dungeonFloorCount = -1;

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
            var site = new DungeonSite { Id = id, Name = distanceBand == 0 ? "Root Hollow" : "Deepstone Vault", Theme = distanceBand == 0 ? "root_cavern" : "stone_vault", Entrance = entrance, Depth = depth, Threat = 1.15 + distanceBand * 0.65, BaseElevation = TileAt(entrance).Elevation };
            site.RumoredBy.Add(villageId);
            Int2 upper = entrance;
            for (int index = 0; index < depth; index++)
            {
                int level = -index - 1;
                int direction = index % 2 == 0 ? 1 : -1;
                var spawn = new Int2(upper.X + direction * 4, upper.Z, level);
                var floor = new DungeonFloor { Level = level, Spawn = spawn, Threat = site.Threat + index * 0.85, Elevation = site.BaseElevation + level * 4 };
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
                floor.Loot.Add(new Stack(index == 0 ? "ore" : "gem", 3 + index));
                floor.Loot.Add(new Stack(index == 0 ? "herbs" : "stone", 2));
                site.Floors.Add(floor);
                var guard = new Int2(spawn.X + direction * 10, spawn.Z + bend - 1, level);
                Creatures.Add(new DungeonCreature { Id = id + ":guardian:" + index, SiteId = id, Position = guard, Home = guard, X = guard.X, Z = guard.Z, Health = 15 + index * 9 + distanceBand * 8, MaxHealth = 15 + index * 9 + distanceBand * 8, Power = floor.Threat, Kind = index == 0 ? "cave_beetle" : "stone_guardian" });
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
        }

        public double DungeonReadiness(Village village, Cat cat) => cat == null ? 0 : EquipmentPower(village, cat, "weapon") * (1 + Amount(cat.Skills, "fight") * 0.25) * Math.Clamp(cat.Health / 100, 0, 1) * Math.Min(1, Math.Min(cat.Hunger, Math.Min(cat.Thirst, cat.Rest)) / 45);

        public ActionResult ExploreDungeon(Village village, Cat cat, string siteId)
        {
            var site = Dungeons.Find(d => d.Id == siteId && d.RumoredBy.Contains(village.Id));
            if (cat == null || !cat.Alive || !village.Cats.Contains(cat) || cat.ControlledBy != "" || cat.AgeHours < 12 || cat.Migration == "arriving" || cat.Migration == "departing")
                return ActionResult.Fail("Choose a living adult cat in this village and release direct control");
            if (site == null)
                return ActionResult.Fail("No known dungeon entrance");
            if (IsDungeonExplorer(cat))
                return ActionResult.Fail("Recall the current expedition first");
            if (DungeonReadiness(village, cat) < site.Threat || cat.Health < 65 || cat.Hunger < 45 || cat.Thirst < 45 || cat.Rest < 45)
                return ActionResult.Fail("Dungeon risk exceeds this cat's health, needs, fighting skill and equipment");
            if (Path(cat.Position, site.Entrance, village) == null)
                return ActionResult.Fail("The dungeon entrance has no accessible surface route");
            CancelWork(village, cat);
            if (cat.BuildingId != "")
                return ActionResult.Fail("The cat must finish physically returning its transport first");
            cat.DungeonId = site.Id;
            cat.DungeonPhase = "exploring";
            cat.DungeonFloorIndex = 0;
            cat.BlockedReason = "";
            cat.Goal = "exploring dungeon";
            Note(village, "dungeon", cat.Name + " set out for " + site.Name, cat.Id);
            return ActionResult.Ok(cat.Id);
        }

        public ActionResult RecallExplorer(Village village, Cat cat)
        {
            if (cat == null || !cat.Alive || !village.Cats.Contains(cat) || !IsDungeonExplorer(cat))
                return ActionResult.Fail("Choose a living explorer in this village");
            cat.DungeonPhase = "returning";
            cat.DungeonTargetId = "";
            cat.Goal = "returning from dungeon";
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

        private bool InDungeonCombatRange(Cat cat, DungeonCreature creature) => cat.Position.Level == creature.Position.Level && Walkable(cat.Position) && Walkable(creature.Position) && Math.Abs(cat.X - creature.X) + Math.Abs(cat.Z - creature.Z) <= 1.1 && (cat.Position.Equals(creature.Position) || Crossable(cat.Position, creature.Position));

        private void RetreatDungeon(Cat cat, string reason)
        {
            cat.DungeonPhase = "returning";
            cat.DungeonTargetId = "";
            cat.BlockedReason = reason;
            cat.Goal = "returning from dungeon";
        }

        public void TickDungeons(double dt, bool planning)
        {
            foreach (var village in Villages)
                foreach (var cat in village.Cats.Where(c => c.Alive && c.Health > 0 && IsDungeonExplorer(c)).ToArray())
                {
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
                        foreach (var cell in site.Floors.SelectMany(f => f.Cells).Where(p => p.Level == cat.Position.Level && Int2.Distance(p, cat.Position) <= 4))
                            if (!village.Known.Contains(cell))
                                village.Known.Add(cell);
                    if (cat.ControlledBy != "")
                    {
                        var target = Creatures.Find(e => e.Id == cat.DungeonTargetId && e.Health > 0);
                        if (target != null && InDungeonCombatRange(cat, target))
                            FightDungeon(village, cat, target);
                        continue;
                    }
                    if (cat.DungeonPhase == "manual" || cat.Health < 50 || cat.Hunger < 30 || cat.Thirst < 30 || cat.Rest < 30)
                        RetreatDungeon(cat, "expedition_return_for_safety");
                    if (cat.DungeonPhase == "returning" || cat.DungeonPhase == "unloading")
                    {
                        ReturnDungeon(village, cat, dt);
                        continue;
                    }
                    var floor = site.Floors[Math.Clamp(cat.DungeonFloorIndex, 0, site.Floors.Count - 1)];
                    var enemy = Creatures.Where(e => e.SiteId == site.Id && e.Position.Level == floor.Level && e.Health > 0).OrderBy(e => Int2.Distance(cat.Position, e.Position)).ThenBy(e => e.Id, StringComparer.Ordinal).FirstOrDefault();
                    if (enemy != null && DungeonReadiness(village, cat) < enemy.Power)
                    {
                        RetreatDungeon(cat, "expedition_enemy_too_dangerous");
                        continue;
                    }
                    if (cat.Position.Level != floor.Level)
                    {
                        cat.Goal = "descending dungeon stairs";
                        var stair = site.Stairs.FirstOrDefault(s => s.From.Equals(cat.Position) && s.To.Level == floor.Level);
                        if (stair != null)
                        {
                            // The immediate landing is visible down an open staircase.
                            TileAt(stair.To);
                            if (!village.Known.Contains(stair.From))
                                village.Known.Add(stair.From);
                            if (!village.Known.Contains(stair.To))
                                village.Known.Add(stair.To);
                        }
                        Move(cat, floor.Spawn, dt);
                        continue;
                    }
                    if (enemy != null)
                    {
                        cat.Goal = "fighting dungeon guardian";
                        if (InDungeonCombatRange(cat, enemy))
                            FightDungeon(village, cat, enemy);
                        else
                            Move(cat, enemy.Position, dt);
                        if (cat.BlockedReason == "blocked_route")
                            RetreatDungeon(cat, "expedition_guardian_unreachable");
                        continue;
                    }
                    cat.Goal = "collecting dungeon supplies";
                    if (!Move(cat, floor.LootPosition, dt))
                        continue;
                    // Goods move once from this authoritative chest into this cat's cargo.
                    foreach (var stack in floor.Loot.ToArray())
                    {
                        double take = Math.Min(stack.Amount, Math.Max(0, CarryCapacity(village, stack.Resource) - Amount(cat.Cargo, stack.Resource)));
                        Add(cat.Cargo, stack.Resource, take);
                        Add(floor.Loot, stack.Resource, -take);
                    }
                    if (cat.DungeonFloorIndex + 1 < site.Floors.Count && DungeonReadiness(village, cat) >= site.Floors[cat.DungeonFloorIndex + 1].Threat && cat.Cargo.Sum(s => s.Amount) < 8)
                    {
                        cat.DungeonFloorIndex++;
                        cat.Path.Clear();
                    }
                    else
                        RetreatDungeon(cat, "");
                }
            foreach (var creature in Creatures.Where(e => e.Health > 0))
                TickDungeonCreature(creature, dt, planning);
        }

        private void FightDungeon(Village village, Cat cat, DungeonCreature creature)
        {
            double seconds = SpendActorTime(cat);
            if (seconds <= 0)
                return;
            creature.Health = Math.Max(0, creature.Health - seconds * 1.8 * EquipmentPower(village, cat, "weapon") * (1 + Amount(cat.Skills, "fight") * 0.25) * Catalog.Effect(village, "combatPower", 1) * Catalog.Effect(village, "combatPowerMult", 1));
            Add(cat.Skills, "fight", seconds / 300);
            foreach (var item in village.Items.Where(i => cat.Equipment.Contains(i.Id)))
                item.Condition = Math.Max(0, item.Condition - seconds * 0.025);
            if (creature.Health <= 0)
            {
                creature.TargetCatId = "";
                creature.Path.Clear();
                Note(village, "dungeon", cat.Name + " defeated a " + creature.Kind.Replace('_', ' '), creature.Id);
            }
        }

        private void TickDungeonCreature(DungeonCreature creature, double dt, bool planning)
        {
            var target = Cat(creature.TargetCatId);
            if (target == null || !target.Alive || target.Position.Level != creature.Position.Level || Int2.Distance(target.Position, creature.Home) > 12)
            {
                target = null;
                creature.TargetCatId = "";
                if (TimeSeconds + 1e-9 >= creature.NextPlanningAt)
                {
                    creature.NextPlanningAt = Math.Floor(TimeSeconds) + 1;
                    target = Villages.SelectMany(v => v.Cats).Where(c => c.Alive && c.Position.Level == creature.Position.Level && Int2.Distance(c.Position, creature.Position) <= 5).OrderBy(c => Int2.Distance(c.Position, creature.Position)).ThenBy(c => c.Id, StringComparer.Ordinal).FirstOrDefault();
                }
                if (target != null)
                    creature.TargetCatId = target.Id;
            }
            if (target == null)
                return;
            if (InDungeonCombatRange(target, creature))
            {
                var village = Village(target.VillageId);
                target.Health = Math.Max(0, target.Health - dt * 0.45 * creature.Power / Math.Max(1, EquipmentPower(village, target, "armor") * Catalog.Effect(village, "defensePower", 1) * Catalog.Effect(village, "defenseMult", 1)));
                if (target.Health <= 0)
                    Die(village, target, "dungeon");
                return;
            }
            bool invalid = creature.Path.Count == 0 || !creature.Path[creature.Path.Count - 1].Equals(target.Position) || !Walkable(creature.Path[0]) || !Crossable(creature.Position, creature.Path[0]);
            if (invalid)
            {
                if (!AtTravelPoint(creature.X, creature.Z, creature.Position))
                {
                    AdvanceTraveler(ref creature.X, ref creature.Z, creature.Position, 0.65, dt);
                    return;
                }
                if (TimeSeconds + 1e-9 < creature.NextPlanningAt)
                    return;
                creature.NextPlanningAt = Math.Floor(TimeSeconds) + 1;
                creature.Path = Path(creature.Position, target.Position) ?? new List<Int2>();
                if (creature.Path.Any(p => p.Level != creature.Position.Level))
                    creature.Path.Clear();
            }
            if (creature.Path.Count > 0 && Walkable(creature.Path[0]) && Crossable(creature.Position, creature.Path[0]) && AdvanceTraveler(ref creature.X, ref creature.Z, creature.Path[0], 0.65, dt))
            {
                creature.Position = creature.Path[0];
                creature.Path.RemoveAt(0);
            }
        }

        private void ReturnDungeon(Village village, Cat cat, double dt)
        {
            if (cat.DungeonPhase != "unloading")
            {
                if (!Move(cat, village.Center, dt))
                    return;
                cat.DungeonPhase = "unloading";
                cat.DungeonStorageId = "";
            }
            if (cat.Cargo.Count == 0)
            {
                CancelDungeon(cat);
                cat.Goal = "idle";
                cat.BlockedReason = "";
                Note(village, "dungeon", cat.Name + " returned from the dungeon", cat.Id);
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
                cat.DungeonNextPlanAt = Math.Floor(TimeSeconds) + 1;
                store = village.Stockpiles.Where(Accepts).OrderBy(p => Int2.Distance(cat.Position, p.Position)).FirstOrDefault(p => Path(cat.Position, p.Position, village) != null);
                cat.DungeonStorageId = store?.Id ?? "";
            }
            if (store == null)
            {
                cat.BlockedReason = "expedition_storage_required";
                if (cat.Health < 40 || cat.Hunger < 25 || cat.Thirst < 25 || cat.Rest < 25)
                {
                    Spill(village, cat.Position, cat.Cargo);
                    CancelDungeon(cat);
                    cat.Goal = "idle";
                    Note(village, "dungeon", cat.Name + " left recovered supplies beside the full storage and sought care", cat.Id);
                }
                return;
            }
            if (!Move(cat, store.Position, dt))
            {
                if (cat.BlockedReason == "blocked_route")
                    cat.DungeonStorageId = "";
                return;
            }
            double quantity = cargo.Amount;
            Add(store.Goods, cargo.Resource, quantity);
            Add(cat.Cargo, cargo.Resource, -quantity);
        }
    }
}
