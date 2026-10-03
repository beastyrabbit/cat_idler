using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using IdleCatForest.Simulation;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Display names for catalog ids and world entities. Buildings, roles and places are proper nouns
    /// ("the Sawmill", "Steward Hazel"); resources are lowercase in running text and sentence case as
    /// row labels; recipes and studies use their catalog names. Never returns an id or a coordinate.
    /// </summary>
    public static class Names
    {
        private static readonly Dictionary<string, string> Buildings = new Dictionary<string, string>
        {
            ["den"] = "Den",
            ["food_storage"] = "Food Store",
            ["water_bowl"] = "Water Bowl",
            ["beds"] = "Bed Nook",
            ["herb_garden"] = "Herb Garden",
            ["nursery"] = "Nursery",
            ["elder_corner"] = "Elder Corner",
            ["walls"] = "Fence",
            ["mouse_farm"] = "Mouse Farm",
            ["shrine"] = "Shrine",
            ["workshop"] = "Workshop",
            ["field"] = "Field",
            ["research_hut"] = "Research Hut",
            ["school"] = "School",
            ["smithy"] = "Smithy",
            ["barracks"] = "Barracks",
            ["accounting_tent"] = "Accounting Tent",
            ["wood_cutter"] = "Woodcutter",
            ["stone_prep"] = "Stone Prep",
            ["woodworking"] = "Woodworker",
            ["clothier"] = "Clothier",
            ["tannery"] = "Tannery",
            ["smelter"] = "Smelter",
            ["mill"] = "Mill",
            ["sawmill"] = "Sawmill",
        };

        /// <summary>Resource row labels (sentence case) and the singular used for a count of one.</summary>
        private static readonly Dictionary<string, (string label, string one, bool countable)> Resources = new Dictionary<string, (string, string, bool)>
        {
            ["food"] = ("Food", "food", false),
            ["fish"] = ("Fish", "fish", false),
            ["water"] = ("Water", "water", false),
            ["herbs"] = ("Herbs", "herb", true),
            ["catnip"] = ("Catnip", "catnip", false),
            ["grain"] = ("Grain", "grain", false),
            ["flour"] = ("Flour", "flour", false),
            ["preserves"] = ("Preserves", "jar of preserves", true),
            ["medicine"] = ("Medicine", "medicine", false),
            ["brew"] = ("Brew", "brew", false),
            ["materials"] = ("Materials", "material", true),
            ["stone"] = ("Stone", "stone", false),
            ["refined"] = ("Refined goods", "refined good", true),
            ["weapons"] = ("Weapons", "weapon", true),
            ["armor"] = ("Armor", "armor", false),
            ["logs"] = ("Logs", "log", true),
            ["lumber"] = ("Lumber", "lumber", false),
            ["planks"] = ("Planks", "plank", true),
            ["blocks"] = ("Stone blocks", "stone block", true),
            ["tools"] = ("Tools", "tool", true),
            ["fibre"] = ("Fibre", "fibre", false),
            ["thread"] = ("Thread", "thread", false),
            ["hide"] = ("Hide", "hide", false),
            ["bone"] = ("Bone", "bone", false),
            ["cloth"] = ("Cloth", "cloth", false),
            ["leather"] = ("Leather", "leather", false),
            ["ore"] = ("Ore", "ore", false),
            ["gem"] = ("Gems", "gem", true),
            ["clay"] = ("Clay", "clay", false),
            ["sand"] = ("Sand", "sand", false),
            ["metal"] = ("Metal", "metal", false),
            ["blessings"] = ("Blessings", "blessing", true),
            ["research"] = ("Research", "research", false),
            ["coins"] = ("Coins", "coin", true),
            ["items"] = ("Items", "item", true),
        };

        private static readonly Dictionary<string, string> Labors = new Dictionary<string, string>
        {
            ["hunt"] = "Hunting",
            ["fishing"] = "Fishing",
            ["build"] = "Building",
            ["ritual"] = "Rituals",
            ["fight"] = "Fighting",
            ["train"] = "Training",
            ["quarry"] = "Quarrying",
            ["woodcut"] = "Woodcutting",
            ["forage"] = "Foraging",
            ["fetch_water"] = "Fetching water",
            ["mill"] = "Milling",
            ["process"] = "Processing",
            ["craft"] = "Crafting",
            ["textile"] = "Weaving",
            ["metalwork"] = "Metalwork",
            ["farm"] = "Farming",
            ["haul"] = "Hauling",
            ["research"] = "Research",
            ["scout"] = "Scouting",
            ["transport"] = "Driving",
            ["leadership"] = "Leadership",
            ["explore"] = "Exploring",
        };

        private static readonly Dictionary<string, string> Roles = new Dictionary<string, string>
        {
            ["leader"] = "Leader",
            ["steward"] = "Steward",
            ["accountant"] = "Accountant",
            ["forester"] = "Forester",
            ["farmer"] = "Farmer",
            ["captain"] = "Captain",
            ["loremaster"] = "Loremaster",
            ["cloth_leader"] = "Weaver",
        };

        /// <summary>Job kinds as short nouns for lists and summaries ("a water run").</summary>
        private static readonly Dictionary<string, string> Jobs = new Dictionary<string, string>
        {
            [JobKind.Hunt] = "Hunt",
            [JobKind.Water] = "Water run",
            [JobKind.Logs] = "Wood run",
            [JobKind.Quarry] = "Quarrying",
            [JobKind.Fibre] = "Fibre gathering",
            [JobKind.Fish] = "Fishing trip",
            [JobKind.Scout] = "Scouting trip",
            [JobKind.Replant] = "Replanting",
            [JobKind.Train] = "Training",
            [JobKind.Expand] = "Fence building",
            [JobKind.Haul] = "Haul",
            [JobKind.Build] = "Construction",
            [JobKind.Production] = "Workshop shift",
            [JobKind.Offering] = "Offering",
            [JobKind.MouseFarm] = "Mouse farming",
            [JobKind.Road] = "Road building",
            [JobKind.Rail] = "Track laying",
            [JobKind.Bridge] = "Bridge building",
            [JobKind.Dock] = "Dock building",
            [JobKind.Wagon] = "Wagon building",
            [JobKind.Vessel] = "Boat building",
            [JobKind.SupplyFood] = "Hunt",
            [JobKind.LeaderPlanHunt] = "Hunt",
            [JobKind.HuntExpedition] = "Hunt",
            [JobKind.SupplyWater] = "Water run",
            [JobKind.FetchWater] = "Water run",
            [JobKind.GatherLogs] = "Wood run",
            [JobKind.ForageFibre] = "Fibre gathering",
            [JobKind.ReplantTree] = "Replanting",
            [JobKind.Explore] = "Scouting trip",
            [JobKind.TrainWarrior] = "Training",
            [JobKind.ExpandVillage] = "Fence building",
            [JobKind.LeaderPlanHouse] = "Construction",
            [JobKind.BuildHouse] = "Construction",
            [JobKind.Ritual] = "Offering",
            [JobKind.CarryOffering] = "Offering",
            [JobKind.PerformOffering] = "Offering",
            ["gather"] = "Gathering",
            ["zone_gather"] = "Gathering",
        };

        private static readonly Dictionary<string, string> Phases = new Dictionary<string, string>
        {
            [JobPhase.Travel] = "On the way",
            [JobPhase.Fetch] = "Fetching supplies",
            [JobPhase.ItemFetch] = "Picking up an item",
            [JobPhase.InputDelivery] = "Delivering materials",
            [JobPhase.Working] = "At work",
            [JobPhase.OutputDelivery] = "Bringing it home",
            [JobPhase.Return] = "Heading home",
        };

        private static readonly Dictionary<string, string> Biomes = new Dictionary<string, string>
        {
            ["forest"] = "Forest",
            ["meadow"] = "Meadow",
            ["wetland"] = "Wetland",
            ["woodland"] = "Woodland",
            ["highland"] = "Highland",
            ["pine_forest"] = "Pine forest",
            ["rainforest"] = "Rainforest",
            ["birch_forest"] = "Birch forest",
            ["taiga"] = "Taiga",
            ["snow_forest"] = "Snowy forest",
            ["flower_forest"] = "Flower forest",
            ["mushroom_forest"] = "Mushroom forest",
            ["desert"] = "Desert",
            ["badlands"] = "Badlands",
            ["beach"] = "Beach",
            ["marsh"] = "Marsh",
            ["swamp"] = "Swamp",
            ["lake"] = "Lake",
            ["ocean"] = "Ocean",
            ["river"] = "River",
            ["hills"] = "Hills",
            ["mountains"] = "Mountains",
            ["snow_mountains"] = "Snowy mountains",
            ["grassland"] = "Grassland",
            ["savanna"] = "Savanna",
            ["jungle"] = "Jungle",
            ["tundra"] = "Tundra",
            ["cave"] = "Cave",
            ["root_cavern"] = "Root cavern",
        };

        private static readonly Dictionary<string, string> Upgrades = new Dictionary<string, string>
        {
            ["click_power"] = "Helping paws",
            ["supply_speed"] = "Swift supply",
            ["hunt_mastery"] = "Hunt mastery",
            ["build_mastery"] = "Build mastery",
            ["ritual_mastery"] = "Ritual mastery",
            ["resilience"] = "Resilience",
        };

        private static readonly Dictionary<string, string> Items = new Dictionary<string, string>
        {
            ["armor"] = "Armor",
            ["bowl"] = "Bowl",
            ["brick"] = "Brick",
            ["clothing"] = "Cloak",
            ["furniture"] = "Furniture",
            ["mug"] = "Mug",
            ["tool"] = "Tool",
            ["toy"] = "Toy",
            ["trinket"] = "Trinket",
            ["weapon"] = "Weapon",
        };

        private static readonly Dictionary<string, string> Materials = new Dictionary<string, string>
        {
            ["bone"] = "Bone",
            ["clay"] = "Clay",
            ["fibre"] = "Fibre",
            ["gem"] = "Gem",
            ["leather"] = "Leather",
            ["metal"] = "Metal",
            ["sand"] = "Glass",
            ["stone"] = "Stone",
            ["wood"] = "Wooden",
        };

        private static readonly Dictionary<string, string> Categories = new Dictionary<string, string>
        {
            ["building"] = "Buildings",
            ["recipe_resource"] = "Crafts",
            ["upgrade"] = "Colony",
        };

        private static readonly Dictionary<string, string> Creatures = new Dictionary<string, string>
        {
            ["cave_beetle"] = "Cave beetle",
            ["stone_guardian"] = "Stone guardian",
            ["fox"] = "Fox",
            ["raider"] = "Raider",
        };

        private static readonly Dictionary<string, string> Crops = new Dictionary<string, string>
        {
            ["grain"] = "Grain",
            ["herbs"] = "Herb",
            ["herb"] = "Herb",
            ["catnip"] = "Catnip",
        };

        /// <summary>Recipe and study families as they read in effect sentences.</summary>
        private static readonly Dictionary<string, string> Families = new Dictionary<string, string>
        {
            ["hunting"] = "Hunting",
            ["foraging"] = "Foraging",
            ["grain_milling"] = "Milling",
            ["baking"] = "Baking",
            ["herbalism"] = "Herbalism",
            ["textile_work"] = "Weaving",
            ["leatherworking"] = "Leatherwork",
            ["carpentry"] = "Carpentry",
            ["stonecraft"] = "Stonecraft",
            ["metallurgy"] = "Metallurgy",
            ["toolmaking"] = "Toolmaking",
            ["weaponcraft"] = "Weaponcraft",
            ["armorcraft"] = "Armorcraft",
            ["food_preservation"] = "Food preserving",
            ["brewing"] = "Brewing",
            ["waterworks"] = "Waterworks",
            ["trade_goods"] = "Trade goods",
            ["animal_husbandry"] = "Animal care",
            ["field_craft"] = "Field craft",
            ["expedition_supplies"] = "Expedition supplies",
        };

        private static readonly Dictionary<string, string> Capabilities = new Dictionary<string, string>
        {
            ["mountain_travel"] = "Mountain paths",
            ["rail_logistics"] = "Rail carts",
            ["water_travel"] = "Boats",
        };

        /// <summary>The display name of <paramref name="id"/> as <paramref name="kind"/>; never the id itself.</summary>
        public static string Of(NameKind kind, string id)
        {
            if (string.IsNullOrEmpty(id))
                return "";
            switch (kind)
            {
                case NameKind.Building:
                    return Building(id);
                case NameKind.Resource:
                    return ResourceLabel(id);
                case NameKind.Recipe:
                    return Recipe(id);
                case NameKind.Study:
                    return Study(id);
                case NameKind.Labor:
                    return Lookup(Labors, id);
                case NameKind.Role:
                    return Role(id);
                case NameKind.Job:
                    return Job(id);
                case NameKind.Phase:
                    return Lookup(Phases, id);
                case NameKind.Goal:
                    return NowLines.GoalText(id);
                case NameKind.Biome:
                    return Lookup(Biomes, id);
                case NameKind.Upgrade:
                    return Lookup(Upgrades, Snake(id));
                case NameKind.Item:
                    return Lookup(Items, id);
                case NameKind.Pile:
                    return "Stockpile";
                case NameKind.Village:
                    return id == "communal" ? "the Commons" : "a neighbouring colony";
                case NameKind.Cat:
                    return "a cat";
                case NameKind.Category:
                    return Lookup(Categories, id);
                case NameKind.Material:
                    return Lookup(Materials, id);
                case NameKind.Creature:
                    return Lookup(Creatures, id);
                case NameKind.Crop:
                    return Lookup(Crops, id);
                case NameKind.Family:
                    return Lookup(Families, id);
                case NameKind.Capability:
                    return Lookup(Capabilities, id);
                case NameKind.Stat:
                    return Sentence(Effects.StatName(id));
                default:
                    return Humanize(id);
            }
        }

        /// <summary>True when <paramref name="id"/> has an authored name of <paramref name="kind"/>.</summary>
        public static bool IsAuthored(NameKind kind, string id)
        {
            switch (kind)
            {
                case NameKind.Building:
                    return Buildings.ContainsKey(id);
                case NameKind.Resource:
                    return Resources.ContainsKey(id);
                case NameKind.Recipe:
                    return Catalog.Recipe(id) != null && Catalog.Recipe(id).Name != "";
                case NameKind.Study:
                    return Catalog.Study(id) != null && Catalog.Study(id).Name != "";
                case NameKind.Labor:
                    return Labors.ContainsKey(id);
                case NameKind.Role:
                    return Roles.ContainsKey(id);
                case NameKind.Job:
                    return Jobs.ContainsKey(id);
                case NameKind.Phase:
                    return Phases.ContainsKey(id);
                case NameKind.Biome:
                    return Biomes.ContainsKey(id);
                case NameKind.Upgrade:
                    return Upgrades.ContainsKey(Snake(id));
                case NameKind.Item:
                    return Items.ContainsKey(id);
                case NameKind.Category:
                    return Categories.ContainsKey(id);
                case NameKind.Material:
                    return Materials.ContainsKey(id);
                case NameKind.Creature:
                    return Creatures.ContainsKey(id);
                case NameKind.Crop:
                    return Crops.ContainsKey(id);
                case NameKind.Family:
                    return Families.ContainsKey(id);
                case NameKind.Capability:
                    return Capabilities.ContainsKey(id);
                case NameKind.Stat:
                    return Effects.IsAuthoredStat(id);
                default:
                    return false;
            }
        }

        /// <summary>A building kind as a proper noun: "Research Hut".</summary>
        public static string Building(string kind) => Lookup(Buildings, kind);

        /// <summary>A resource as a row label: "Logs", "Stone blocks".</summary>
        public static string ResourceLabel(string resource) =>
            resource != null && Resources.TryGetValue(resource, out var r) ? r.label : Humanize(resource);

        /// <summary>A resource in running text: "logs", "stone blocks", "water".</summary>
        public static string Resource(string resource) =>
            resource != null && Resources.TryGetValue(resource, out var r) ? Lower(r.label) : Humanize(resource).ToLowerInvariant();

        /// <summary>True for resources counted in pieces ("3 logs"); false for masses ("water").</summary>
        public static bool Countable(string resource) => resource != null && Resources.TryGetValue(resource, out var r) && r.countable;

        /// <summary>
        /// An amount of a resource in running text: "3 logs", "1 log", "5 food", "0.5 water".
        /// </summary>
        public static string Amount(double amount, string resource)
        {
            string number = Numbers.Amount(amount);
            string noun = number == "1" && resource != null && Resources.TryGetValue(resource, out var r) ? r.one : Resource(resource);
            return number + " " + noun;
        }

        /// <summary>A recipe by its catalog name.</summary>
        public static string Recipe(string id)
        {
            var recipe = id == null ? null : Catalog.Recipe(id);
            return recipe != null && recipe.Name != "" ? recipe.Name : Humanize(id);
        }

        /// <summary>A study by its catalog name.</summary>
        public static string Study(string id)
        {
            var study = id == null ? null : Catalog.Study(id);
            return study != null && study.Name != "" ? study.Name : Humanize(id);
        }

        /// <summary>An office: "Steward".</summary>
        public static string Role(string role) => Lookup(Roles, role);

        /// <summary>A job kind as a short noun: "Water run".</summary>
        public static string Job(string kind) => Lookup(Jobs, kind);

        /// <summary>"Wooden tool", "Glass mug": a crafted item from its kind and material.</summary>
        public static string Item(string kind, string material)
        {
            string item = Lookup(Items, kind);
            if (string.IsNullOrEmpty(material) || !Materials.TryGetValue(material, out var m))
                return item;
            return m + " " + Lower(item);
        }

        /// <summary>A crafted item in the village by its kind and material.</summary>
        public static string Item(Item item) => item == null ? "" : Item(item.Kind, item.Material);

        /// <summary>
        /// A building in the village as a proper noun, numbered when the village has more than one of that
        /// kind in list order: "Den 3", "Research Hut".
        /// </summary>
        public static string Building(Village village, Building building)
        {
            if (building == null)
                return "";
            string name = Building(building.Kind);
            if (village == null)
                return name;
            int count = 0, index = 0;
            foreach (var b in village.Buildings)
                if (b.Kind == building.Kind)
                {
                    count++;
                    if (ReferenceEquals(b, building) || b.Id == building.Id && index == 0)
                        index = count;
                }
            return count > 1 && index > 0 ? name + " " + Numbers.Whole(index) : name;
        }

        /// <summary>A building in the village by id, or "" when it is gone.</summary>
        public static string Building(Village village, string buildingId)
        {
            var building = village?.Buildings.Find(b => b.Id == buildingId);
            return building == null ? "" : Building(village, building);
        }

        /// <summary>
        /// A stockpile by what it belongs to: the building or field that manages it, the zone it marks, its
        /// single accepted resource ("Log pile"), or "Stockpile", numbered among unnamed piles.
        /// </summary>
        public static string Pile(Village village, Stockpile pile)
        {
            if (pile == null)
                return "";
            if (village != null && pile.ManagedBy != "")
            {
                var building = village.Buildings.Find(b => b.Id == pile.ManagedBy);
                if (building != null)
                    return Building(village, building);
                var farm = village.Farms.Find(f => f.Id == pile.ManagedBy);
                if (farm != null)
                    return Farm(village, farm);
            }
            switch (pile.Kind)
            {
                case "spill":
                    return "Dropped goods";
                case "farm_output":
                    return "Harvest pile";
                case "fishing":
                    return "Fishing spot";
                case "gather":
                case "zone_gather":
                    return "Gathering spot";
                case "zone_avoid":
                    return "No-go zone";
            }
            if (pile.Kind.StartsWith("zone_", StringComparison.Ordinal))
                return "Zone";
            if (pile.Accepts.Count == 1)
                return Sentence(Singular(pile.Accepts[0])) + " pile";
            if (village == null)
                return "Stockpile";
            int count = 0, index = 0;
            foreach (var p in village.Stockpiles)
                if (p.Kind == "storage" && p.ManagedBy == "" && p.Accepts.Count != 1)
                {
                    count++;
                    if (ReferenceEquals(p, pile))
                        index = count;
                }
            return count > 1 && index > 0 ? "Stockpile " + Numbers.Whole(index) : "Stockpile";
        }

        /// <summary>A stockpile in the village by id, or "" when it is gone.</summary>
        public static string Pile(Village village, string pileId)
        {
            var pile = village?.Stockpiles.Find(p => p.Id == pileId);
            return pile == null ? "" : Pile(village, pile);
        }

        /// <summary>"Grain field", numbered when there are several of that crop: "Grain field 2".</summary>
        public static string Farm(Village village, Farm farm)
        {
            if (farm == null)
                return "";
            string name = Lookup(Crops, farm.Crop) + " field";
            if (village == null)
                return name;
            int count = 0, index = 0;
            foreach (var f in village.Farms)
                if (f.Crop == farm.Crop)
                {
                    count++;
                    if (ReferenceEquals(f, farm))
                        index = count;
                }
            return count > 1 && index > 0 ? name + " " + Numbers.Whole(index) : name;
        }

        /// <summary>
        /// The place a job or need points at: a building, a stockpile or a field in the village, or ""
        /// when the id names nothing the player can see (for example a tile).
        /// </summary>
        public static string Place(Village village, string id)
        {
            if (village == null || string.IsNullOrEmpty(id))
                return "";
            string name = Building(village, id);
            if (name != "")
                return name;
            name = Pile(village, id);
            if (name != "")
                return name;
            var farm = village.Farms.Find(f => f.Id == id);
            return farm == null ? "" : Farm(village, farm);
        }

        /// <summary>A cat's name, or "a cat" when it is unknown.</summary>
        public static string Cat(Village village, string catId)
        {
            var cat = village?.Cats.Find(c => c.Id == catId);
            return cat == null || string.IsNullOrEmpty(cat.Name) ? "a cat" : cat.Name;
        }

        /// <summary>"Steward Hazel" for an officer, the plain name otherwise.</summary>
        public static string Titled(Village village, Cat cat)
        {
            if (cat == null)
                return "";
            if (village != null && village.LeaderId == cat.Id)
                return "Leader " + cat.Name;
            if (cat.OfficerRole != "" && Roles.ContainsKey(cat.OfficerRole))
                return Roles[cat.OfficerRole] + " " + cat.Name;
            return cat.Name;
        }

        /// <summary>The colony's name, or "your colony".</summary>
        public static string Village(Village village) => village == null || string.IsNullOrEmpty(village.Name) ? "your colony" : village.Name;

        /// <summary>"Basic Tools and Sawmill" from study ids; "Sawmill" once when ids repeat.</summary>
        public static string Studies(IEnumerable<string> ids) => List(ids.Distinct().Select(Study));

        /// <summary>"a, b and c".</summary>
        public static string List(IEnumerable<string> parts)
        {
            var items = parts.Where(p => !string.IsNullOrEmpty(p)).ToList();
            if (items.Count == 0)
                return "";
            if (items.Count == 1)
                return items[0];
            return string.Join(", ", items.Take(items.Count - 1)) + " and " + items[items.Count - 1];
        }

        /// <summary>The singular in running text: "log", "stone block", "water".</summary>
        public static string Singular(string resource) =>
            resource != null && Resources.TryGetValue(resource, out var r) ? r.one : Humanize(resource).ToLowerInvariant();

        /// <summary>Capitalises the first letter.</summary>
        public static string Sentence(string text) =>
            string.IsNullOrEmpty(text) ? "" : char.ToUpperInvariant(text[0]) + text.Substring(1);

        /// <summary>Lowercases the first letter unless the word is an acronym.</summary>
        public static string Lower(string text) =>
            string.IsNullOrEmpty(text) || text.Length > 1 && char.IsUpper(text[1]) ? text ?? "" : char.ToLowerInvariant(text[0]) + text.Substring(1);

        /// <summary>
        /// The last-resort readable form of an unknown id: snake_case, kebab-case and camelCase become
        /// words in sentence case, and trailing entity numbers ("cat-25") are dropped.
        /// </summary>
        public static string Humanize(string id)
        {
            if (string.IsNullOrEmpty(id))
                return "";
            var words = new StringBuilder(id.Length + 4);
            char previous = ' ';
            foreach (char ch in id)
            {
                if (ch == '_' || ch == '-')
                {
                    words.Append(' ');
                    previous = ' ';
                    continue;
                }
                if (char.IsUpper(ch) && char.IsLower(previous))
                    words.Append(' ');
                words.Append(ch);
                previous = ch;
            }
            var parts = words.ToString().Split(new[] { ' ' }, StringSplitOptions.RemoveEmptyEntries).ToList();
            // An entity id such as "cat-25" or "station_store-18" ends in a number: never show it.
            while (parts.Count > 1 && parts[parts.Count - 1].All(char.IsDigit))
                parts.RemoveAt(parts.Count - 1);
            if (parts.Count == 1 && parts[0].All(char.IsDigit))
                return "";
            return Sentence(string.Join(" ", parts).ToLowerInvariant());
        }

        private static string Lookup(Dictionary<string, string> table, string id) =>
            id != null && table.TryGetValue(id, out var name) ? name : Humanize(id);

        private static string Snake(string key)
        {
            if (string.IsNullOrEmpty(key))
                return "";
            var text = new StringBuilder();
            foreach (char ch in key)
            {
                if (char.IsUpper(ch) && text.Length > 0)
                    text.Append('_');
                text.Append(char.ToLowerInvariant(ch));
            }
            return text.ToString();
        }
    }
}
