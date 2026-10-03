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
        // True only on DemoContent's copies, so a village-only availability check still reads demo unlocks.
        [NonSerialized] internal bool Demo;
        internal Recipe Copy() => new Recipe
        {
            Id = Id,
            Building = Building,
            Name = Name,
            Labor = Labor,
            ItemKind = ItemKind,
            Material = Material,
            Quality = Quality,
            Founding = Founding,
            Seconds = Seconds,
            Inputs = Inputs.Select(s => new Stack(s.Resource, s.Amount)).ToList(),
            Outputs = Outputs.Select(s => new Stack(s.Resource, s.Amount)).ToList(),
            Demo = Demo,
        };
    }
    [Serializable]
    public class ResearchPayload
    {
        public string Kind = "", Id = "", Attribute = "", Operation = ""; public double Value;
        internal ResearchPayload Copy() => new ResearchPayload { Kind = Kind, Id = Id, Attribute = Attribute, Operation = Operation, Value = Value };
    }
    [Serializable]
    public class Study
    {
        public string Id = "", Name = "", Description = "", Category = ""; public double Cost; public int Era, X, Z, LeaderPriority;
        public List<string> Prerequisites = new List<string>(); public List<ResearchPayload> Payloads = new List<ResearchPayload>();
        internal Study Copy() => new Study
        {
            Id = Id,
            Name = Name,
            Description = Description,
            Category = Category,
            Cost = Cost,
            Era = Era,
            X = X,
            Z = Z,
            LeaderPriority = LeaderPriority,
            Prerequisites = Prerequisites.ToList(),
            Payloads = Payloads.Select(p => p.Copy()).ToList(),
        };
    }

    /// <summary>
    /// The authored catalog of buildings, recipes and studies (the data lives in Catalog.Generated.cs,
    /// which is edited by hand; tools/catalog-export only diffs it against the legacy Rust).
    /// Queries that take a <see cref="World"/> read that world's content: classic worlds read these
    /// tables unchanged, demo worlds read <see cref="DemoContent"/>'s overrides. The village-only
    /// overloads read the classic tables, except <see cref="RecipeAvailable(Village, Recipe)"/>,
    /// which follows the recipe it is given.
    /// </summary>
    public static partial class Catalog
    {
        public static readonly string[] Buildings = { "den", "food_storage", "water_bowl", "beds", "herb_garden", "nursery", "elder_corner", "walls", "mouse_farm", "shrine", "workshop", "field", "research_hut", "school", "smithy", "barracks", "accounting_tent", "wood_cutter", "stone_prep", "woodworking", "clothier", "tannery", "smelter", "mill", "sawmill" };
        public static readonly string[] Labors = { "hunt", "fishing", "build", "ritual", "fight", "train", "quarry", "woodcut", "forage", "fetch_water", "mill", "process", "craft", "textile", "metalwork", "farm", "haul", "research", "scout" };
        public static readonly string[] Resources = { "food", "fish", "water", "herbs", "catnip", "grain", "flour", "preserves", "medicine", "brew", "materials", "stone", "refined", "weapons", "armor", "logs", "lumber", "planks", "blocks", "tools", "fibre", "thread", "hide", "bone", "cloth", "leather", "ore", "gem", "clay", "sand", "metal", "blessings" };
        public static readonly string[] Roles = { "steward", "accountant", "forester", "farmer", "captain", "loremaster", "cloth_leader" };
        public static readonly string[] RoleBuildings = { "workshop", "accounting_tent", "sawmill", "field", "barracks", "research_hut", "clothier" };
        public static readonly string[] RoleStudies = { "basic_tools", "basic_tools", "sawmill", "irrigation", "barracks", "research_hut", "textiles" };
        public static readonly string[] Upgrades = { "click_power", "supply_speed", "hunt_mastery", "build_mastery", "ritual_mastery", "resilience" };
        public static readonly List<Recipe> Recipes = CreateRecipes();
        public static readonly List<Study> Research = CreateResearch();
        public static Recipe Recipe(string id) => Recipes.Find(x => x.Id == id);
        public static Study Study(string id) => Research.Find(x => x.Id == id);
        /// <summary>The study as this world sees it: demo worlds get demo cost, prerequisites and payloads.</summary>
        public static Study Study(World w, string id) => DemoContent.Study(w, id);

        private sealed class Resolved
        {
            public int Count = -1; public List<string> Source;
            public readonly HashSet<string> Owned = new HashSet<string>();
            public readonly Dictionary<string, List<ResearchPayload>> Effects = new Dictionary<string, List<ResearchPayload>>();
        }
        private static readonly ConditionalWeakTable<Village, Resolved> ResolvedByVillage = new ConditionalWeakTable<Village, Resolved>();
        private static readonly ConditionalWeakTable<Village, Resolved> DemoResolvedByVillage = new ConditionalWeakTable<Village, Resolved>();
        private static Resolved Resolve(Village v, bool demo)
        {
            var r = (demo ? DemoResolvedByVillage : ResolvedByVillage).GetOrCreateValue(v);
            if (r.Count == v.Research.Count && ReferenceEquals(r.Source, v.Research))
                return r;
            r.Count = v.Research.Count;
            r.Source = v.Research;
            r.Owned.Clear();
            r.Effects.Clear();
            var owned = new HashSet<string>(v.Research);
            foreach (var node in (demo ? DemoContent.DemoStudies : Research).Where(n => owned.Contains(n.Id)))
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

        // Which studies declare each building: a building nobody declares is available from founding.
        private sealed class BuildingAccess
        {
            public bool Declared, Founding; public readonly List<string> Unlockers = new List<string>();
        }
        private static Dictionary<string, BuildingAccess> classicAccess;
        private static Dictionary<string, BuildingAccess> demoAccess;
        private static Dictionary<string, BuildingAccess> Access(bool demo)
        {
            var cached = demo ? demoAccess : classicAccess;
            if (cached != null)
                return cached;
            var access = Buildings.ToDictionary(b => b, b => new BuildingAccess());
            foreach (var node in demo ? DemoContent.DemoStudies : Research)
                foreach (var p in node.Payloads.Where(p => p.Kind == "building_available_at_founding" || p.Kind == "unlock_building"))
                    if (access.TryGetValue(p.Id, out var entry))
                    {
                        entry.Declared = true;
                        if (p.Kind == "building_available_at_founding")
                            entry.Founding = true;
                        else if (!entry.Unlockers.Contains(node.Id))
                            entry.Unlockers.Add(node.Id);
                    }
            if (demo)
                demoAccess = access;
            else
                classicAccess = access;
            return access;
        }
        private static bool BuildingAvailable(Village v, string id, bool demo)
        {
            if (!Access(demo).TryGetValue(id, out var entry))
                return false;
            // Maintained Rust building_placement_research explicitly treats undeclared shells as founding access.
            return !entry.Declared || entry.Founding || entry.Unlockers.Any(v.Research.Contains);
        }
        private static bool RecipeAvailable(Village v, Recipe r, bool demo) => r != null && (v.GrandfatheredRecipes || r.Founding || Resolve(v, demo).Owned.Contains("unlock_recipe:" + r.Id));
        private static double Effect(Village v, string id, double baseline, bool demo)
        {
            double value = baseline;
            if (Resolve(v, demo).Effects.TryGetValue(id, out var effects))
                foreach (var p in effects.Where(p => p.Kind == "modify"))
                    value = p.Operation == "add" ? value + p.Value : value * p.Value;
            return value;
        }
        private static double BuildingEffect(Village v, string building, string attribute, double baseline, bool demo)
        {
            double value = baseline;
            if (Resolve(v, demo).Effects.TryGetValue(building + ":" + attribute, out var effects))
                foreach (var p in effects.Where(p => p.Kind == "modify_building"))
                    value = p.Operation == "add" ? value + p.Value : value * p.Value;
            return value;
        }

        // Classic tables.
        public static bool Owns(Village v, string kind, string id) => Resolve(v, false).Owned.Contains(kind + ":" + id);
        public static bool RecipeAvailable(Village v, Recipe r) => RecipeAvailable(v, r, r != null && r.Demo);
        public static bool BuildingAvailable(Village v, string id) => BuildingAvailable(v, id, false);
        public static double Effect(Village v, string id, double baseline = 1) => Effect(v, id, baseline, false);
        public static double BuildingEffect(Village v, string building, string attribute, double baseline = 1) => BuildingEffect(v, building, attribute, baseline, false);

        // The world's own content (classic tables, or demo overrides for demo worlds).
        public static bool Owns(World w, Village v, string kind, string id) => Resolve(v, DemoContent.IsDemo(w)).Owned.Contains(kind + ":" + id);
        public static bool RecipeAvailable(World w, Village v, Recipe r) => r != null && RecipeAvailable(v, DemoContent.Recipe(w, r.Id) ?? r, DemoContent.IsDemo(w));
        public static bool BuildingAvailable(World w, Village v, string id) => BuildingAvailable(v, id, DemoContent.IsDemo(w));
        public static double Effect(World w, Village v, string id, double baseline = 1) => Effect(v, id, baseline, DemoContent.IsDemo(w));
        public static double BuildingEffect(World w, Village v, string building, string attribute, double baseline = 1) => BuildingEffect(v, building, attribute, baseline, DemoContent.IsDemo(w));
    }
}
