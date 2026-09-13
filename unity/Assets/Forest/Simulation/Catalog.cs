using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.CompilerServices;

namespace IdleCatForest.Simulation
{
    [Serializable]
    public class Recipe
    {
        public string Id = "", Building = "", Name = "", Labor = "process", ItemKind = "", Material = ""; public int Quality = 1;
        public bool Founding; public double Seconds = 600; public List<Stack> Inputs = new List<Stack>(), Outputs = new List<Stack>();
    }
    [Serializable]
    public class ResearchPayload
    {
        public string Kind = "", Id = "", Attribute = "", Operation = ""; public double Value;
    }
    [Serializable]
    public class Study
    {
        public string Id = "", Name = "", Description = "", Category = ""; public double Cost; public int Era, X, Z, LeaderPriority;
        public List<string> Prerequisites = new List<string>(); public List<ResearchPayload> Payloads = new List<ResearchPayload>();
    }
    public static partial class Catalog
    {
        public static readonly string[] Buildings = { "den", "food_storage", "water_bowl", "beds", "herb_garden", "nursery", "elder_corner", "walls", "mouse_farm", "shrine", "workshop", "field", "research_hut", "school", "smithy", "barracks", "accounting_tent", "wood_cutter", "stone_prep", "woodworking", "clothier", "tannery", "smelter", "mill", "sawmill" };
        public static readonly string[] Labors = { "hunt", "fishing", "build", "ritual", "fight", "train", "quarry", "woodcut", "forage", "fetch_water", "mill", "process", "craft", "textile", "metalwork", "farm", "haul", "research", "scout" };
        public static readonly string[] Resources = { "food", "fish", "water", "herbs", "catnip", "grain", "flour", "preserves", "medicine", "brew", "materials", "stone", "refined", "weapons", "armor", "logs", "lumber", "planks", "blocks", "tools", "fibre", "thread", "hide", "bone", "cloth", "leather", "ore", "gem", "clay", "sand", "metal", "blessings" };
        public static readonly string[] Roles = { "steward", "accountant", "forester", "farmer", "captain", "loremaster", "cloth_leader" };
        public static readonly string[] RoleBuildings = { "workshop", "accounting_tent", "sawmill", "field", "barracks", "research_hut", "clothier" };
        public static readonly string[] RoleStudies = { "basic_tools", "basic_tools", "sawmill", "irrigation", "barracks", "research_hut", "textiles" };
        public static readonly string[] Upgrades = { "click_power", "supply_speed", "hunt_mastery", "build_mastery", "ritual_mastery", "resilience" };
        public static readonly string[] Actions = { "Ensure", "Presence", "RequestJob", "DispatchScout", "Boost", "PurchaseUpgrade", "CastVote", "RequestVoteKick", "CreateZone", "RemoveZone", "PlanBuilding", "UnlockNode", "ResearchNode", "OfferTithe", "OfferMaterials", "OfferResource", "HaulGatherSpot", "AssignWorker", "TrainWarrior", "DefendRaid", "BuildRoad", "BuildBridge", "DesignateRail", "BuildDock", "BuildTransportVehicle", "CreateTransportRoute", "CancelTransportRoute", "SetTestAcceleration", "AdvanceTime", "SetTestRngSeed", "FoundVillage", "JoinVillage", "OfferVillageTrade", "AcceptVillageTrade", "CancelVillageTrade", "AssignOfficer", "UnassignOfficer", "DesignateFarm", "ClearFarm", "DesignateStockpile", "RemoveStockpile", "DesignateGatherSpot", "DesignateFishingSpot", "RemoveGatherSpot", "SellGoods", "RepairItem", "EquipItem", "UnequipItem", "BuyResource", "BoostCat", "SetCatLaborPreference", "EditProductionQueue", "EditProductionWorkSlot", "EnterCat", "MoveCat", "InteractCat", "LeaveCat" };
        public static readonly List<Recipe> Recipes = CreateRecipes();
        public static readonly List<Study> Research = CreateResearch();
        public static Recipe Recipe(string id) => Recipes.Find(x => x.Id == id);
        public static Study Study(string id) => Research.Find(x => x.Id == id);
        private sealed class Resolved
        {
            public int Count = -1; public List<string> Source;
            public readonly HashSet<string> Owned = new HashSet<string>();
            public readonly Dictionary<string, List<ResearchPayload>> Effects = new Dictionary<string, List<ResearchPayload>>();
        }
        private static readonly ConditionalWeakTable<Village, Resolved> ResolvedByVillage = new ConditionalWeakTable<Village, Resolved>();
        private static Resolved Resolve(Village v)
        {
            var r = ResolvedByVillage.GetOrCreateValue(v);
            if (r.Count == v.Research.Count && ReferenceEquals(r.Source, v.Research))
                return r;
            r.Count = v.Research.Count;
            r.Source = v.Research;
            r.Owned.Clear();
            r.Effects.Clear();
            var owned = new HashSet<string>(v.Research);
            foreach (var node in Research.Where(n => owned.Contains(n.Id)))
                foreach (var p in node.Payloads)
                {
                    r.Owned.Add(p.Kind + ":" + p.Id);
                    string key = p.Kind == "modify_building" ? p.Id + ":" + p.Attribute : p.Id;
                    if (!r.Effects.TryGetValue(key, out var effects))
                    {
                        effects = new List<ResearchPayload>();
                        r.Effects[key] = effects;
                    }
                    effects.Add(p);
                }
            return r;
        }
        public static bool Owns(Village v, string kind, string id) => Resolve(v).Owned.Contains(kind + ":" + id);
        public static bool RecipeAvailable(Village v, Recipe r) => r != null && (v.GrandfatheredRecipes || r.Founding || Owns(v, "unlock_recipe", r.Id));
        public static bool BuildingAvailable(Village v, string id)
        {
            if (!Buildings.Contains(id))
                return false;
            var declarations = Research.Where(n => n.Payloads.Any(p => p.Id == id && (p.Kind == "building_available_at_founding" || p.Kind == "unlock_building"))).ToArray();
            // Maintained Rust building_placement_research explicitly treats undeclared shells as founding access.
            return declarations.Length == 0 || declarations.Any(n => n.Payloads.Any(p => p.Id == id && p.Kind == "building_available_at_founding") || v.Research.Contains(n.Id));
        }
        public static double Effect(Village v, string id, double baseline = 1)
        {
            double value = baseline;
            if (Resolve(v).Effects.TryGetValue(id, out var effects))
                foreach (var p in effects.Where(p => p.Kind == "modify"))
                    value = p.Operation == "add" ? value + p.Value : value * p.Value;
            return value;
        }
        public static double BuildingEffect(Village v, string building, string attribute, double baseline = 1)
        {
            double value = baseline;
            if (Resolve(v).Effects.TryGetValue(building + ":" + attribute, out var effects))
                foreach (var p in effects.Where(p => p.Kind == "modify_building"))
                    value = p.Operation == "add" ? value + p.Value : value * p.Value;
            return value;
        }
    }
}
