using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    public partial class World
    {
        private static readonly string[] RecipeFamilies = { "hunting", "foraging", "grain_milling", "baking", "herbalism", "textile_work", "leatherworking", "carpentry", "stonecraft", "metallurgy", "toolmaking", "weaponcraft", "armorcraft", "food_preservation", "brewing", "waterworks", "trade_goods", "animal_husbandry", "field_craft", "expedition_supplies" };
        private static readonly Dictionary<string, string> FamilyCache = new Dictionary<string, string>();
        private static readonly int[] UpgradeBaseCosts = { 2, 3, 5, 5, 6, 7 };
        /// <summary>One village's upgrade levels by normalised key, valid while its level list is unchanged.</summary>
        private sealed class UpgradeLevelSnapshot
        {
            public List<Stack> Source;
            public int Signature;
            public readonly Dictionary<string, double> Levels = new Dictionary<string, double>(StringComparer.Ordinal);
        }
        private sealed class UpgradeLevelCache
        {
            public UpgradeLevelSnapshot Current;
        }
        // Not serialized (private) and keyed weakly, so saves and cloned worlds never carry it.
        private readonly System.Runtime.CompilerServices.ConditionalWeakTable<Village, UpgradeLevelCache> upgradeLevelCaches = new System.Runtime.CompilerServices.ConditionalWeakTable<Village, UpgradeLevelCache>();

        // Called per cat per step (resilience, work rates), so it neither allocates nor re-normalises keys.
        public double UpgradeLevel(Village v, string key) => UpgradeLevels(v).Levels.TryGetValue(UpgradeKey(key), out var level) ? level : 0;

        private UpgradeLevelSnapshot UpgradeLevels(Village v)
        {
            int signature = 17;
            foreach (var s in v.UpgradeLevels)
                signature = unchecked(signature * 31 + (s.Resource ?? "").GetHashCode() * 7 + s.Amount.GetHashCode());
            var cache = upgradeLevelCaches.GetValue(v, _ => new UpgradeLevelCache());
            var current = cache.Current;
            if (current != null && ReferenceEquals(current.Source, v.UpgradeLevels) && current.Signature == signature)
                return current;
            var fresh = new UpgradeLevelSnapshot { Source = v.UpgradeLevels, Signature = signature };
            foreach (var s in v.UpgradeLevels)
            {
                string normal = UpgradeKey(s.Resource);
                fresh.Levels[normal] = (fresh.Levels.TryGetValue(normal, out var sum) ? sum : 0) + s.Amount;
            }
            cache.Current = fresh;
            return fresh;
        }

        /// <summary>Upgrade keys are stored snake_case; legacy rows and callers may still use camelCase.</summary>
        private static string UpgradeKey(string key)
        {
            key = key ?? "";
            foreach (char ch in key)
                if (char.IsUpper(ch) || ch == ' ')
                    return Snake(key);
            return key;
        }
        public double UpgradeCost(Village v, string key)
        {
            int index = Array.IndexOf(Catalog.Upgrades, Snake(key));
            return index < 0 ? double.PositiveInfinity : UpgradeBaseCosts[index] * (UpgradeLevel(v, key) + 1);
        }
        private ActionResult PurchaseUpgrade(Village v, string key)
        {
            key = Snake(key);
            int index = Array.IndexOf(Catalog.Upgrades, key);
            if (index < 0)
                return ActionResult.Fail("Unknown upgrade");
            double level = UpgradeLevel(v, key);
            if (level >= (index == 0 ? 20 : 10))
                return ActionResult.Fail("Upgrade already maxed");
            double cost = UpgradeCost(v, key);
            if (v.Blessings < cost)
                return ActionResult.Fail("Insufficient blessings");
            v.Blessings -= cost;
            v.UpgradeLevels.RemoveAll(s => UpgradeKey(s.Resource) == key);
            v.UpgradeLevels.Add(new Stack(key, level + 1));
            Note(v, EventKind.Upgrade, key + " reached level " + (level + 1));
            return ActionResult.Ok(key);
        }
        private double UpgradeWorkRate(Village v, string labor)
        {
            if (labor == "fetch_water")
                return 1 / Math.Max(Pace.UpgradeSupplySpeedFloor, 1 - UpgradeLevel(v, "supply_speed") * Pace.UpgradeSupplySpeedStep);
            if (labor == "hunt")
                return 1 / Math.Max(Pace.UpgradeHuntFloor, 1 - UpgradeLevel(v, "hunt_mastery") * Pace.UpgradeHuntStep);
            if (labor == "build")
                return 1 / Math.Max(Pace.UpgradeBuildFloor, 1 - UpgradeLevel(v, "build_mastery") * Pace.UpgradeBuildStep);
            if (labor == "ritual")
                return 1 / Math.Max(Pace.UpgradeRitualFloor, 1 - UpgradeLevel(v, "ritual_mastery") * Pace.UpgradeRitualStep);
            return 1;
        }
        private ActionResult BoostJob(Village v, Job job)
        {
            if (job == null)
                return ActionResult.Fail("Job no longer active");
            long minute = (long)(TimeSeconds / 60);
            if (v.BoostMinute != minute)
            {
                v.BoostMinute = minute;
                v.BoostsUsed = 0;
            }
            v.BoostsUsed++;
            double seconds = Pace.BoostSeconds + UpgradeLevel(v, "click_power") * Pace.BoostSecondsPerClickPower;
            if (v.BoostsUsed > Pace.BoostFreePerMinute)
                seconds = Math.Max(Pace.BoostMinSeconds, Math.Floor(seconds * Math.Pow(Pace.BoostDecay, (v.BoostsUsed - Pace.BoostFreePerMinute) / Pace.BoostDecayClicks)));
            var building = job.Kind == JobKind.Build ? v.Buildings.Find(b => b.Id == job.TargetId) : null;
            if (building != null)
                building.RequiredWork = Math.Max(building.Progress + 1, building.RequiredWork - seconds);
            else
                job.RequiredWork = Math.Max(job.Progress + 1, job.RequiredWork - seconds);
            job.BoostCount++;
            return ActionResult.Ok();
        }
        public static string RecipeFamily(Recipe recipe)
        {
            if (FamilyCache.TryGetValue(recipe.Id, out var cached))
                return cached;
            foreach (var n in Catalog.Research.Where(n => n.Payloads.Any(p => p.Kind == "unlock_recipe" && p.Id == recipe.Id)))
            {
                var family = RecipeFamilies.FirstOrDefault(f => n.Id.StartsWith(f + "_", StringComparison.Ordinal));
                if (family != null)
                {
                    FamilyCache[recipe.Id] = family;
                    return family;
                }
            }
            string fallback = recipe.Building == "clothier" ? "textile_work" : recipe.Building == "tannery" ? "leatherworking" : recipe.Building == "wood_cutter" || recipe.Building == "sawmill" ? "carpentry" : recipe.Building == "stone_prep" ? "stonecraft" : recipe.Building == "smelter" ? "metallurgy" : recipe.Building == "smithy" ? recipe.ItemKind == "armor" ? "armorcraft" : recipe.ItemKind == "weapon" ? "weaponcraft" : "toolmaking" : recipe.Building == "woodworking" ? "toolmaking" : recipe.Building == "mill" ? "grain_milling" : "trade_goods";
            FamilyCache[recipe.Id] = fallback;
            return fallback;
        }
        private static bool FamilyOwned(Village v, Recipe recipe, string stage) => Catalog.Owns(v, "unlock_resource", RecipeFamily(recipe) + "_" + stage);
        // Exact goods stay whole identities. Output studies improve material efficiency instead of minting fractional items.
        public double RecipeInput(Village v, Recipe recipe, double amount) => amount * (recipe.ItemKind != "" && FamilyOwned(v, recipe, "sources") ? Pace.FamilySourcesInputFactor : 1) / (recipe.ItemKind != "" ? Math.Max(1, Catalog.BuildingEffect(v, recipe.Building, "output", 1)) : 1);
        public double RecipeOutput(Village v, Recipe recipe, double amount) => amount * (recipe.ItemKind == "" && FamilyOwned(v, recipe, "sources") ? Pace.FamilySourcesOutputFactor : 1) * Catalog.BuildingEffect(v, recipe.Building, "output", 1);
        public double RecipeDuration(Village v, Recipe recipe) => recipe.Seconds * (FamilyOwned(v, recipe, "bulk") ? Pace.FamilyBulkDurationFactor : 1);
        private static bool Completed(Village v, string kind) => v.Buildings.Any(b => b.Kind == kind && b.Completed);
        public int DenCapacity(Village v) => Math.Max(5, (int)Catalog.Effect(v, "housingPerDen", 5));
        // haulCapacity studies are multipliers on the base load of 8 (legacy haul_capacity_mult starts at 1).
        public double CarryCapacity(Village v, string resource = "") => Math.Clamp(BaseCarryLoad * Catalog.Effect(v, "haulCapacity", 1) + (resource == "water" ? Catalog.Effect(v, "waterCarryCapacity", 0) : 0), BaseCarryLoad, 64);
        private const double BaseCarryLoad = 8;

        // PACING-HANDOFF ToolQualityWorkScale: Pacing.Jobs knob, classic 0 (flat WorkToolBonus), demo 0.375.
        private const double DemoToolQualityWorkScale = 0.375;
        private double ToolQualityWorkScale => Pace.Profile == Pacing.DemoProfile ? DemoToolQualityWorkScale : 0;

        /// <summary>
        /// The work-rate factor of the cat's equipped tool: 1 without a working tool. Classic gives every tool the
        /// flat <c>WorkToolBonus</c>; the demo scales it with quality, material and condition like
        /// <c>EquipmentPower</c>, so a basic quality-1 wood tool keeps the flat bonus and better tools beat it.
        /// </summary>
        public double ToolWorkBonus(Village v, Cat c)
        {
            if (!v.Items.Any(i => c.Equipment.Contains(i.Id) && i.Kind == "tool" && i.Condition > 0))
                return 1;
            double scale = ToolQualityWorkScale;
            return scale <= 0 ? Pace.WorkToolBonus : 1 + (EquipmentPower(v, c, "tool") - 1) * scale;
        }

        /// <summary>
        /// Demo only: a cat starting work that has no tool takes the best free tool from storage, as the Equip
        /// action would, so crafted tools reach workers without micromanagement.
        /// </summary>
        private void AutoEquipTool(Village v, Cat c, Job j)
        {
            if (Pace.Profile != Pacing.DemoProfile || j.Kind == JobKind.Haul || j.Kind == JobKind.Scout || IsDungeonExplorer(c))
                return;
            if (v.Items.Any(i => i.LocationId == c.Id && i.Kind == "tool"))
                return;
            var best = v.Items.Where(i => i.Kind == "tool" && i.Condition > 0 && v.Stockpiles.Any(p => p.Id == i.LocationId))
                .OrderByDescending(ToolRank).ThenBy(i => i.Id, StringComparer.Ordinal).FirstOrDefault();
            if (best == null)
                return;
            best.LocationId = c.Id;
            c.Equipment.Add(best.Id);
        }

        private static double ToolRank(Item item) => (item.Quality + 1) * (item.Material == "metal" ? 1.5 : item.Material == "stone" ? 1.2 : item.Material == "bone" ? 1.1 : 1) * Math.Min(1, item.Condition / Math.Max(1, item.MaxCondition));
        private double Service(Village v, string building, string effect, double baseline = 1) => Completed(v, building) ? Catalog.Effect(v, effect, baseline) : baseline;
        public double ResourceCapacityBonus(Village v, string resource)
        {
            double result = 0;
            foreach (var recipe in Catalog.Recipes.Where(r => r.Outputs.Any(s => s.Resource == resource) || EquipmentKind(resource) != "" && EquipmentKind(resource) == r.ItemKind))
            {
                if (FamilyOwned(v, recipe, "preservation"))
                {
                    result += 50;
                    break;
                }
            }
            if (new[] { "food", "hide", "bone" }.Contains(resource) && Catalog.Owns(v, "unlock_resource", "hunting_reserves"))
                result += 50;
            if (new[] { "fibre", "herbs", "catnip" }.Contains(resource) && Catalog.Owns(v, "unlock_resource", "foraging_reserves"))
                result += 50;
            if (resource == "grain" && Catalog.Owns(v, "unlock_resource", "grain_milling_reserves"))
                result += 50;
            if (resource == "food" && Catalog.Owns(v, "unlock_resource", "baking_reserves"))
                result += 50;
            return result;
        }
        private void Commission(Village v, Building b)
        {
            b.Completed = true;
            b.ConstructionConsumed = true;
            if (b.Kind == "food_storage" || b.Kind == "water_bowl")
            {
                var pile = new Stockpile { Id = Id("container"), ManagedBy = b.Id, Position = b.Position, Width = b.Width, Depth = b.Depth, Capacity = 250 };
                pile.Accepts = b.Kind == "water_bowl" ? new List<string> { "water" } : new List<string> { "food", "fish", "grain", "flour", "preserves", "brew" };
                v.Stockpiles.Add(pile);
            }
            else if (Catalog.Recipes.Any(r => r.Building == b.Kind))
            {
                var pile = new Stockpile { Id = Id("station_store"), ManagedBy = b.Id, Kind = "storage", Position = b.Position, Width = 1, Depth = 1, Capacity = 16 };
                pile.Accepts = Catalog.Recipes.Where(r => r.Building == b.Kind).SelectMany(r => r.Inputs.Concat(r.Outputs)).Select(s => s.Resource).Distinct().ToList();
                v.Stockpiles.Add(pile);
            }
        }
        private void Spoilage(Village v)
        {
            foreach (var pile in v.Stockpiles)
                foreach (var s in pile.Goods.Where(s => new[] { "food", "fish", "herbs", "grain", "flour", "preserves", "brew" }.Contains(s.Resource)).ToArray())
                {
                    double held = Reservations.Where(r => r.PileId == pile.Id && r.Resource == s.Resource).Sum(r => r.Amount);
                    double rate = s.Resource == "preserves" ? Pace.PreservesSpoilageRate : Pace.SpoilageRate;
                    double resistance = Catalog.Effect(v, "spoilageResistance", 1) * Service(v, "food_storage", "foodStorekeeping", 1);
                    double loss = Math.Max(0, s.Amount - held) * rate / resistance;
                    if (loss > 0)
                        Add(pile.Goods, s.Resource, -loss);
                }
            // Spills emptied by spoilage, spending or need trips go away on the same upkeep tick.
            PruneEmptySpills(v);
        }
        private double SourceAmount(Tile tile, string resource) => tile.Resource == resource ? tile.Amount : Amount(tile.Deposits, resource);
        private void TakeSource(Tile tile, string resource, double amount)
        {
            if (tile.Resource == resource)
                tile.Amount = Math.Max(0, tile.Amount - amount);
            else
                Add(tile.Deposits, resource, -amount);
        }
        private double FishStock(Tile tile) => tile.FishCapacity > 0 && tile.Deposits.Any(s => s.Resource == "fish") ? Amount(tile.Deposits, "fish") : SourceAmount(tile, "fish");
        private void SetFish(Tile tile, double amount)
        {
            if (tile.Deposits.Any(s => s.Resource == "fish"))
                Add(tile.Deposits, "fish", amount - Amount(tile.Deposits, "fish"));
            if (tile.Resource == "fish")
                tile.Amount = amount;
            else if (!tile.Deposits.Any(s => s.Resource == "fish"))
                Add(tile.Deposits, "fish", amount);
        }
        private void Ecology()
        {
            if ((long)TimeSeconds % Pace.EcologyTickSeconds != 0)
                return;
            foreach (var tile in Tiles.Where(t => t.Water && t.FishCapacity > 0))
            {
                double elapsed = Math.Max(0, TimeSeconds - tile.FishReplenishedAt);
                SetFish(tile, Math.Min(tile.FishCapacity, FishStock(tile) + elapsed / 3600 * Pace.FishRegenPerHour));
                tile.FishReplenishedAt = TimeSeconds;
            }
        }
    }
}
