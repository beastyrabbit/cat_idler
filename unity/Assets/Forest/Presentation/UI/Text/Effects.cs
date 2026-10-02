using System;
using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Effect sentences for research payloads: "+10 % hunt yield", "Unlocks the Clothier",
    /// "Sawmill works 10 % faster". Values are fractions in the catalog and percentages on screen.
    /// </summary>
    public static class Effects
    {
        /// <summary>Colony-wide modifiers by stat key, in running text.</summary>
        private static readonly Dictionary<string, string> Stats = new Dictionary<string, string>
        {
            ["accountingSpeed"] = "counting speed",
            ["barracksReadiness"] = "guard readiness",
            ["combatPower"] = "fighting strength",
            ["combatPowerMult"] = "fighting strength",
            ["constructionSpeed"] = "building speed",
            ["defenseMult"] = "defence",
            ["defensePower"] = "defence",
            ["denStewardship"] = "Den upkeep",
            ["elderProtection"] = "elder care",
            ["farmYield"] = "harvests",
            ["farmYieldMult"] = "harvests",
            ["fieldStewardship"] = "field care",
            ["foodStorekeeping"] = "food keeping",
            ["gatherYieldMult"] = "gathering yield",
            ["haulCapacity"] = "carrying capacity",
            ["healthRecovery"] = "healing",
            ["herbMedicineEfficacy"] = "herb medicine strength",
            ["housingPerDen"] = "beds per Den",
            ["huntYieldMult"] = "hunt yield",
            ["kittenGrowth"] = "kitten growth",
            ["materialYieldMult"] = "material yield",
            ["mouseFarmFood"] = "Mouse Farm food",
            ["moveSpeedMult"] = "walking speed",
            ["movementSpeed"] = "walking speed",
            ["productionRate"] = "workshop speed",
            ["researchRate"] = "research speed",
            ["researchRateMult"] = "research speed",
            ["restRecovery"] = "sleep quality",
            ["shrineBlessingYield"] = "blessings from offerings",
            ["spoilageResistance"] = "spoilage resistance",
            ["storageCapacity"] = "storage space",
            ["storagePerLevelMult"] = "storage space",
            ["tradeValue"] = "trade value",
            ["wallDefense"] = "fence strength",
            ["waterCarryCapacity"] = "water per trip",
            ["waterEfficiency"] = "water efficiency",
            ["waterStewardship"] = "water care",
        };

        /// <summary>Stats whose catalog value is a count rather than a fraction.</summary>
        private static readonly HashSet<string> Absolute = new HashSet<string> { "housingPerDen", "waterCarryCapacity" };

        private static readonly Dictionary<string, string> Jobs = new Dictionary<string, string>
        {
            ["gather_logs"] = "Cats can cut wood for logs",
        };

        private static readonly Dictionary<string, string> Capabilities = new Dictionary<string, string>
        {
            ["mountain_travel"] = "Opens mountain paths",
            ["rail_logistics"] = "Unlocks rail carts",
            ["water_travel"] = "Unlocks boats and docks",
        };

        /// <summary>The stat in running text: "hunt yield".</summary>
        public static string StatName(string stat) =>
            stat != null && Stats.TryGetValue(stat, out var name) ? name : Names.Humanize(stat).ToLowerInvariant();

        /// <summary>True when the stat has an authored name (Coverage).</summary>
        public static bool IsAuthoredStat(string stat) => stat != null && Stats.ContainsKey(stat);

        /// <summary>
        /// One effect from a payload kind, its target and value. Building modifiers need the attribute
        /// and read better through <see cref="Of(ResearchPayload)"/>.
        /// </summary>
        public static string Effect(string payloadKind, string target, double value) =>
            Of(new ResearchPayload { Kind = payloadKind ?? "", Id = target ?? "", Value = value, Operation = "add" });

        /// <summary>One sentence for one research payload.</summary>
        public static string Of(ResearchPayload payload)
        {
            if (payload == null)
                return "";
            string id = payload.Id ?? "";
            switch (payload.Kind)
            {
                case "modify":
                    if (Absolute.Contains(id))
                        return Numbers.Signed(payload.Value) + " " + StatName(id);
                    return Numbers.SignedPercent(Fraction(payload)) + " " + StatName(id);
                case "modify_building":
                    return Building(id, payload.Attribute, payload.Operation, payload.Value);
                case "unlock_building":
                    return "Unlocks the " + Names.Building(id);
                case "building_available_at_founding":
                    return "The " + Names.Building(id) + " can be built from the start";
                case "unlock_recipe":
                    return Recipe(id);
                case "unlock_resource":
                    return Resource(id);
                case "unlock_capability":
                    return Capabilities.TryGetValue(id, out var capability) ? capability : "Unlocks " + Names.Humanize(id).ToLowerInvariant();
                case "unlock_job":
                    return Jobs.TryGetValue(id, out var job) ? job : "Unlocks " + Names.Lower(Names.Job(id));
                default:
                    return Names.Humanize(payload.Kind) + (id == "" ? "" : ": " + Names.Humanize(id).ToLowerInvariant());
            }
        }

        /// <summary>All of a study's effects, one sentence each, with duplicates merged.</summary>
        public static IReadOnlyList<string> Of(Study study) =>
            study == null ? new string[0] : study.Payloads.Select(Of).Where(s => s != "").Distinct().ToList();

        /// <summary>True when every payload kind of the catalog has a sentence (Coverage).</summary>
        public static bool IsAuthoredPayload(string payloadKind)
        {
            switch (payloadKind)
            {
                case "modify":
                case "modify_building":
                case "unlock_building":
                case "building_available_at_founding":
                case "unlock_recipe":
                case "unlock_resource":
                case "unlock_capability":
                case "unlock_job":
                    return true;
                default:
                    return false;
            }
        }

        private static double Fraction(ResearchPayload payload) =>
            payload.Operation == "multiply" ? payload.Value - 1 : payload.Value;

        private static string Building(string building, string attribute, string operation, double value)
        {
            string name = Names.Building(building);
            double fraction = operation == "multiply" ? value - 1 : value;
            switch (attribute)
            {
                case "cycle_time":
                    // A shorter cycle is faster work: × 0.9 reads "10 % faster".
                    return fraction <= 0
                        ? name + " works " + Numbers.Percent(-fraction) + " faster"
                        : name + " works " + Numbers.Percent(fraction) + " slower";
                case "output":
                    return Numbers.SignedPercent(fraction) + " " + name + " output";
                case "capacity":
                    return Numbers.SignedPercent(fraction) + " " + name + " storage";
                case "worker_slots":
                    return Numbers.Signed(value) + " " + Numbers.Noun(Math.Abs(value), "worker", "workers") + " at the " + name;
                case "durability":
                    return name + " goods last " + Numbers.Percent(fraction) + " longer";
                default:
                    return "Improves the " + name;
            }
        }

        private static string Recipe(string id)
        {
            var recipe = Catalog.Recipe(id);
            if (recipe == null)
                return "Unlocks " + Names.Humanize(id).ToLowerInvariant();
            string station = recipe.Building == "" ? "" : " at the " + Names.Building(recipe.Building);
            return "Unlocks " + Names.Recipe(id) + station;
        }

        private static string Resource(string id)
        {
            foreach (var (suffix, template) in new[]
            {
                ("_sources", "{0} recipes use less and make more"),
                ("_preservation", "{0} goods keep longer"),
                ("_bulk", "{0} work goes faster"),
                ("_reserves", "More storage for {0} goods"),
            })
                if (id.EndsWith(suffix, StringComparison.Ordinal))
                {
                    string family = id.Substring(0, id.Length - suffix.Length);
                    string name = Names.Of(NameKind.Family, family);
                    return string.Format(Numbers.Culture, template, template.StartsWith("{0}", StringComparison.Ordinal) ? name : Names.Lower(name));
                }
            return "Unlocks " + Names.Resource(id);
        }
    }
}
