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
        public double UpgradeLevel(Village v, string key) => v.UpgradeLevels.Where(s => Snake(s.Resource) == Snake(key)).Sum(s => s.Amount);
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
            v.UpgradeLevels.RemoveAll(s => Snake(s.Resource) == key);
            v.UpgradeLevels.Add(new Stack(key, level + 1));
            Note(v, "upgrade", key + " reached level " + (level + 1));
            return ActionResult.Ok(key);
        }
        private double UpgradeWorkRate(Village v, string labor)
        {
            if (labor == "fetch_water")
                return 1 / Math.Max(0.55, 1 - UpgradeLevel(v, "supply_speed") * 0.1);
            if (labor == "hunt")
                return 1 / Math.Max(0.45, 1 - UpgradeLevel(v, "hunt_mastery") * 0.1);
            if (labor == "build")
                return 1 / Math.Max(0.45, 1 - UpgradeLevel(v, "build_mastery") * 0.1);
            if (labor == "ritual")
                return 1 / Math.Max(0.4, 1 - UpgradeLevel(v, "ritual_mastery") * 0.12);
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
            double seconds = 10 + UpgradeLevel(v, "click_power") * 2;
            if (v.BoostsUsed > 30)
                seconds = Math.Max(1, Math.Floor(seconds * Math.Pow(0.95, (v.BoostsUsed - 30) / 10.0)));
            var building = job.Kind == "build" ? v.Buildings.Find(b => b.Id == job.TargetId) : null;
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
        public double RecipeInput(Village v, Recipe recipe, double amount) => amount * (recipe.ItemKind != "" && FamilyOwned(v, recipe, "sources") ? 0.9 : 1) / (recipe.ItemKind != "" ? Math.Max(1, Catalog.BuildingEffect(v, recipe.Building, "output", 1)) : 1);
        public double RecipeOutput(Village v, Recipe recipe, double amount) => amount * (recipe.ItemKind == "" && FamilyOwned(v, recipe, "sources") ? 1.1 : 1) * Catalog.BuildingEffect(v, recipe.Building, "output", 1);
        public double RecipeDuration(Village v, Recipe recipe) => recipe.Seconds * (FamilyOwned(v, recipe, "bulk") ? 0.9 : 1);
        private static bool Completed(Village v, string kind) => v.Buildings.Any(b => b.Kind == kind && b.Completed);
        public int DenCapacity(Village v) => Math.Max(5, (int)Catalog.Effect(v, "housingPerDen", 5));
        public double CarryCapacity(Village v, string resource = "") => Math.Clamp(Catalog.Effect(v, "haulCapacity", 8) + (resource == "water" ? Catalog.Effect(v, "waterCarryCapacity", 0) : 0), 8, 64);
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
                    double rate = s.Resource == "preserves" ? 0.00002 : 0.0002;
                    double resistance = Catalog.Effect(v, "spoilageResistance", 1) * Service(v, "food_storage", "foodStorekeeping", 1);
                    double loss = Math.Max(0, s.Amount - held) * rate / resistance;
                    if (loss > 0)
                        Add(pile.Goods, s.Resource, -loss);
                }
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
            if ((long)TimeSeconds % 60 != 0)
                return;
            foreach (var tile in Tiles.Where(t => t.Water && t.FishCapacity > 0))
            {
                double elapsed = Math.Max(0, TimeSeconds - tile.FishReplenishedAt);
                SetFish(tile, Math.Min(tile.FishCapacity, FishStock(tile) + elapsed / 3600 * 0.5));
                tile.FishReplenishedAt = TimeSeconds;
            }
        }
    }
}
