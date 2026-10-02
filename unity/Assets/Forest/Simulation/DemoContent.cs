using System;
using System.Collections.Generic;
using System.Linq;

namespace IdleCatForest.Simulation
{
    /// <summary>
    /// The demo's content layer over the authored catalog. Demo worlds (pacing profile "demo") see a
    /// curated slice of studies with demo costs and prerequisites, faster and kinder recipes, merged
    /// stewardship studies and real founding effects for service buildings. Classic worlds never see
    /// any of it: every query here returns the catalog unchanged for them, so classic numbers and
    /// the mechanics scenarios stay valid. Overrides are copies; the catalog tables are never edited.
    /// </summary>
    public static class DemoContent
    {
        public const string LogsToMaterials = "logs_to_materials";

        /// <summary>The curated study slice in its recommended order. The first entry is owned at founding.</summary>
        public static readonly IReadOnlyList<string> Slice;
        private static readonly string[] SliceIds;

        /// <summary>Studies a demo colony owns from founding, so the first purchase already does something.</summary>
        public static readonly IReadOnlyList<string> FoundingStudies = new[] { "research_hut" };

        // Demo research cost and prerequisites, in the order an average player buys them. Costs follow
        // the demo research rate (one point per scholar-minute, two founding scholars) so each study
        // lands about two minutes after the last; DemoContentScenarios checks that budget.
        private static readonly (string Id, double Cost, string[] Requires)[] SliceRules =
        {
            ("research_hut", 0, new string[0]),
            ("basic_tools", 3, new string[0]),
            ("den_insulation", 3, new string[0]),
            ("water_carriers", 3, new string[0]),
            ("foraging_lore", 4, new[] { "basic_tools" }),
            ("school", 4, new[] { "den_insulation" }),
            ("barracks", 5, new[] { "basic_tools" }),
            ("sawmill", 5, new[] { "foraging_lore" }),
            ("accounting_tent_foundations", 5, new[] { "basic_tools" }),
            ("irrigation", 5, new[] { "water_carriers" }),
            ("research_hut_crews", 6, new[] { "school" }),
            ("den_foundations", 7, new[] { "den_insulation" }),
            ("textiles", 7, new[] { "foraging_lore" }),
            ("masonry", 8, new[] { "sawmill" }),
            ("water_bowl_foundations", 8, new[] { "water_carriers" }),
            ("milling", 8, new[] { "irrigation" }),
            ("smithy", 8, new[] { "sawmill" }),
            ("scholars_guild", 9, new[] { "school" }),
            ("walls_foundations", 10, new[] { "masonry", "barracks" }),
            ("housing_tier_2", 10, new[] { "masonry" }),
            ("carpentry_specialty", 11, new[] { "sawmill" }),
            ("mountaineering", 11, new[] { "masonry" }),
        };

        // Demo payload values that replace the catalog's for the same study and effect.
        private static readonly (string Study, string Effect, double Value)[] PayloadValues =
        {
            ("basic_tools", "huntYieldMult", 0.25),
            ("water_carriers", "waterCarryCapacity", 4),
        };

        // Extra unlocks for demo worlds: the Smithy also opens the Smelter so metal has a source.
        private static readonly (string Study, string Kind, string Id)[] ExtraPayloads =
        {
            ("smithy", "unlock_building", "smelter"),
        };

        /// <summary>The recipes a station starts with once it is unlocked (demo worlds).</summary>
        private static readonly Dictionary<string, string[]> BaseRecipes = new Dictionary<string, string[]>
        {
            { "wood_cutter", new[] { "logs_to_planks", LogsToMaterials } },
            { "stone_prep", new[] { "stone_to_blocks" } },
            { "woodworking", new[] { "planks_and_blocks_to_tools" } },
            { "workshop", new[] { "herbal_poultice" } },
            { "sawmill", new[] { "logs_to_lumber" } },
            { "mill", new[] { "grain_to_flour", "flour_to_food" } },
            { "clothier", new[] { "fibre_to_thread", "fibre_to_cloth" } },
            { "tannery", new[] { "hide_to_leather" } },
            { "smelter", new[] { "ore_to_metal" } },
            { "smithy", new[] { "smithy_tool", "smithy_weapon", "smithy_armor" } },
        };

        // Stations whose building is available from founding get their base recipes from founding.
        private static readonly string[] FoundingRecipes = { "herbal_poultice", LogsToMaterials };

        // Moved recipes: the Sawmill, not the wood cutter, cuts the efficient planks.
        private static readonly Dictionary<string, string> RecipeStation = new Dictionary<string, string> { { "carpentry_specialty", "sawmill" } };

        // Recipes in the metal item tiers take more metal for better quality, not less.
        private static readonly string[] MetalTierFamilies = { "toolmaking", "weaponcraft", "armorcraft" };
        private static readonly string[] Tiers = { "preparation", "staples", "quality", "specialty", "masterwork" };
        private static readonly double[] MetalTierAmounts = { 2, 3, 4, 5, 6 };

        /// <summary>Founding effect of a completed service building in a demo world (multiplies the service).</summary>
        private static readonly Dictionary<string, (string Effect, double Factor)> FoundingService = new Dictionary<string, (string, double)>
        {
            { "beds", ("restRecovery", 1.25) },
            { "nursery", ("kittenGrowth", 2.0) },
            { "herb_garden", ("herbMedicineEfficacy", 1.5) },
            { "water_bowl", ("waterStewardship", 1.2) },
            { "walls", ("wallDefense", 2.0) },
        };

        private static readonly string[] StewardshipStages = { "foundations", "workflow", "timing", "crews", "reinforcement", "mastery" };
        /// <summary>A merged stewardship study grants at least this much.</summary>
        public const double MergedStewardshipValue = 0.2;

        // Item kinds no system reads: they only sell to the trader, so the demo hides their recipes.
        private static readonly HashSet<string> DecorativeItems = new HashSet<string> { "mug", "bowl", "toy", "trinket", "furniture", "clothing", "brick" };
        private static readonly HashSet<string> EquipmentItems = new HashSet<string> { "tool", "weapon", "armor" };
        // Resources a simulation system consumes directly (needs, construction, expansion, offerings, farms, transport).
        private static readonly HashSet<string> ConsumedResources = new HashSet<string> { "food", "fish", "preserves", "water", "brew", "herbs", "medicine", "grain", "materials", "planks", "lumber", "blocks", "metal" };

        internal static readonly List<Study> DemoStudies;
        private static readonly Dictionary<string, Study> DemoStudyById;
        private static readonly List<Study> SliceStudies;
        private static readonly List<Recipe> DemoRecipes;
        private static readonly Dictionary<string, Recipe> DemoRecipeById;
        private static readonly List<Recipe> VisibleRecipes;
        private static readonly HashSet<string> VisibleRecipeIds;
        private static readonly HashSet<string> FunctionalRecipeIds;
        private static readonly HashSet<string> VisibleBuildingIds;

        static DemoContent()
        {
            SliceIds = SliceRules.Select(r => r.Id).ToArray();
            Slice = SliceIds;
            DemoRecipes = Catalog.Recipes.Select(DemoRecipe).ToList();
            DemoRecipes.Add(new Recipe { Id = LogsToMaterials, Name = "Bundle Building Materials", Building = "wood_cutter", Labor = "process", Founding = true, Seconds = 60, Inputs = new List<Stack> { new Stack("logs", 2) }, Outputs = new List<Stack> { new Stack("materials", 3) }, Demo = true });
            DemoRecipeById = DemoRecipes.ToDictionary(r => r.Id);
            DemoStudies = Catalog.Research.Select(n => n.Copy()).ToList();
            DemoStudyById = DemoStudies.ToDictionary(n => n.Id);
            foreach (var rule in SliceRules)
            {
                var n = DemoStudyById[rule.Id];
                n.Cost = rule.Cost;
                n.Prerequisites = rule.Requires.ToList();
            }
            foreach (var change in PayloadValues)
                DemoStudyById[change.Study].Payloads.Single(p => p.Kind == "modify" && p.Id == change.Effect).Value = change.Value;
            foreach (var extra in ExtraPayloads)
                DemoStudyById[extra.Study].Payloads.Add(new ResearchPayload { Kind = extra.Kind, Id = extra.Id });
            MergeStewardship();
            foreach (var n in DemoStudies)
                foreach (var building in n.Payloads.Where(p => p.Kind == "unlock_building").Select(p => p.Id).ToArray())
                    if (BaseRecipes.TryGetValue(building, out var recipes))
                        foreach (var recipe in recipes.Where(r => !DemoRecipeById[r].Founding && !n.Payloads.Any(p => p.Kind == "unlock_recipe" && p.Id == r)))
                            n.Payloads.Add(new ResearchPayload { Kind = "unlock_recipe", Id = recipe });
            SliceStudies = Slice.Select(id => DemoStudyById[id]).ToList();
            FunctionalRecipeIds = Functional(DemoRecipes);
            var sliceUnlocks = new HashSet<string>(SliceStudies.SelectMany(n => n.Payloads).Where(p => p.Kind == "unlock_recipe").Select(p => p.Id));
            VisibleRecipes = DemoRecipes.Where(r => FunctionalRecipeIds.Contains(r.Id) && (r.Founding || sliceUnlocks.Contains(r.Id))).ToList();
            VisibleRecipeIds = new HashSet<string>(VisibleRecipes.Select(r => r.Id));
            VisibleBuildingIds = new HashSet<string>(Catalog.Buildings.Where(b =>
            {
                var declarations = DemoStudies.Where(n => n.Payloads.Any(p => p.Id == b && (p.Kind == "unlock_building" || p.Kind == "building_available_at_founding"))).ToArray();
                return declarations.Length == 0 || declarations.Any(n => n.Payloads.Any(p => p.Id == b && p.Kind == "building_available_at_founding") || Slice.Contains(n.Id));
            }));
        }

        private static Recipe DemoRecipe(Recipe classic)
        {
            var r = classic.Copy();
            r.Demo = true;
            r.Seconds = Math.Clamp(Math.Round(classic.Seconds / 10), 45, 90);
            if (r.Inputs.Count == 1 && r.Inputs[0].Amount == 5 && r.Outputs.Count == 1 && r.Outputs[0].Amount == 1)
                r.Inputs[0].Amount = 3;
            int split = r.Id.LastIndexOf('_');
            if (split > 0 && MetalTierFamilies.Contains(r.Id.Substring(0, split)) && r.Inputs.Count == 1 && r.Inputs[0].Resource == "metal")
            {
                int tier = Array.IndexOf(Tiers, r.Id.Substring(split + 1));
                if (tier >= 0)
                    r.Inputs[0].Amount = MetalTierAmounts[tier];
            }
            if (RecipeStation.TryGetValue(r.Id, out var station))
                r.Building = station;
            if (FoundingRecipes.Contains(r.Id))
                r.Founding = true;
            return r;
        }

        private static void MergeStewardship()
        {
            foreach (var building in Catalog.Buildings)
            {
                var chain = StewardshipStages.Select(stage => DemoStudyById.TryGetValue(building + "_" + stage, out var n) ? n : null)
                    .Where(n => n != null && n.Payloads.Count == 1 && n.Payloads[0].Kind == "modify").ToList();
                foreach (var effect in chain.GroupBy(n => n.Payloads[0].Id))
                {
                    var steps = effect.ToList();
                    double total = steps.Sum(n => n.Payloads[0].Value);
                    steps[0].Payloads[0].Value = Math.Max(MergedStewardshipValue, total);
                    foreach (var later in steps.Skip(1))
                        later.Payloads.Clear();
                }
            }
        }

        private static HashSet<string> Functional(List<Recipe> recipes)
        {
            var useful = new HashSet<string>(ConsumedResources);
            var functional = new HashSet<string>();
            bool changed = true;
            while (changed)
            {
                changed = false;
                foreach (var r in recipes.Where(r => !functional.Contains(r.Id)))
                    if (EquipmentItems.Contains(r.ItemKind) || r.ItemKind == "" && r.Outputs.Any(s => useful.Contains(s.Resource)))
                    {
                        functional.Add(r.Id);
                        foreach (var input in r.Inputs)
                            useful.Add(input.Resource);
                        changed = true;
                    }
            }
            return functional;
        }

        public static bool IsDemo(World w) => w != null && w.PacingProfile == Pacing.DemoProfile;

        /// <summary>The recipe as this world runs it; null when the world has no such recipe.</summary>
        public static Recipe Recipe(World w, string id)
        {
            if (!IsDemo(w))
                return Catalog.Recipe(id);
            return id != null && DemoRecipeById.TryGetValue(id, out var r) ? r : null;
        }

        /// <summary>Recipes a player can see and queue in this world: the catalog, or the demo's functional slice.</summary>
        public static IReadOnlyList<Recipe> Recipes(World w) => IsDemo(w) ? (IReadOnlyList<Recipe>)VisibleRecipes : Catalog.Recipes;

        /// <summary>Every recipe the world can run, hidden ones included (classic: the catalog).</summary>
        public static IReadOnlyList<Recipe> AllRecipes(World w) => IsDemo(w) ? (IReadOnlyList<Recipe>)DemoRecipes : Catalog.Recipes;

        /// <summary>The study as this world sees it (demo cost, prerequisites and payloads in demo worlds).</summary>
        public static Study Study(World w, string id)
        {
            if (!IsDemo(w))
                return Catalog.Study(id);
            return id != null && DemoStudyById.TryGetValue(id, out var n) ? n : null;
        }

        /// <summary>Studies this world can research: the whole catalog, or the demo slice in its recommended order.</summary>
        public static IReadOnlyList<Study> Studies(World w) => IsDemo(w) ? (IReadOnlyList<Study>)SliceStudies : Catalog.Research;

        /// <summary>Leader auto-research order: catalog priority, or the slice order in demo worlds.</summary>
        public static int PriorityOf(World w, Study study)
        {
            if (!IsDemo(w))
                return study.LeaderPriority;
            int index = Array.IndexOf(SliceIds, study.Id);
            return index < 0 ? int.MaxValue : index;
        }

        public static bool IsDemoVisibleStudy(string id) => id != null && Slice.Contains(id);
        public static bool IsDemoVisible(Study study) => study != null && IsDemoVisibleStudy(study.Id);
        public static bool IsDemoVisibleRecipe(string id) => id != null && VisibleRecipeIds.Contains(id);
        public static bool IsDemoVisible(Recipe recipe) => recipe != null && IsDemoVisibleRecipe(recipe.Id);
        public static bool IsDemoVisibleBuilding(string kind) => kind != null && VisibleBuildingIds.Contains(kind);

        /// <summary>False for recipes whose output nothing uses (decorative items, refined goods).</summary>
        public static bool IsFunctional(Recipe recipe) => recipe != null && FunctionalRecipeIds.Contains(recipe.Id);

        /// <summary>True when a demo colony owns the study from founding.</summary>
        public static bool OwnedAtFounding(World w, string study) => IsDemo(w) && FoundingStudies.Contains(study);

        /// <summary>Worker slots a building starts with before research (the demo Research Hut seats two).</summary>
        public static int SlotsFor(World w, string building) => IsDemo(w) && building == "research_hut" ? 2 : 1;

        /// <summary>Worker slots a village's building has now, research included.</summary>
        public static int SlotsFor(World w, Village v, string building) => (int)Catalog.BuildingEffect(w, v, building, "worker_slots", SlotsFor(w, building));

        /// <summary>
        /// Founding multiplier of a service building's effect: in demo worlds a completed Beds, Nursery,
        /// Herb Garden, Water Bowl or Walls works before any study. 1 everywhere else.
        /// </summary>
        public static double ServiceFactor(World w, Village v, string building, string effect)
        {
            if (!IsDemo(w) || v == null || !FoundingService.TryGetValue(building ?? "", out var service) || service.Effect != effect)
                return 1;
            return v.Buildings.Any(b => b.Kind == building && b.Completed) ? service.Factor : 1;
        }

        /// <summary>The founding factor a demo service building grants, for display; 1 when it has none.</summary>
        public static double FoundingServiceFactor(string building, out string effect)
        {
            if (FoundingService.TryGetValue(building ?? "", out var service))
            {
                effect = service.Effect;
                return service.Factor;
            }
            effect = "";
            return 1;
        }
    }
}
