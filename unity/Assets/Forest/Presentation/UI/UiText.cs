using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;

namespace IdleCatForest.Presentation
{
    /// <summary>What kind of identifier <see cref="UiText.Name"/> turns into player text.</summary>
    public enum NameKind
    {
        Other, Building, Resource, Recipe, Study, Labor, Role, Job, Phase, Goal, Biome, Upgrade, Item, Pile, Village, Cat,
        Category, Material, Creature, Crop, Family, Capability, Stat,
    }

    /// <summary>
    /// Every player-facing string goes through here. Pure C# without UnityEngine so it is tested outside
    /// Unity (<c>dotnet test tools/presentation-tests</c>). The tables and templates live in
    /// <c>UI/Text/</c>: <see cref="Names"/>, <see cref="Numbers"/>, <see cref="GameClock"/>,
    /// <see cref="NowLines"/>, <see cref="Reasons"/>, <see cref="Confirmations"/>, <see cref="Effects"/> and
    /// <see cref="EventLines"/>. Numbers always use the invariant culture; no output carries an id, a
    /// coordinate or snake_case.
    /// </summary>
    public static class UiText
    {
        /// <summary>
        /// The readable form of any id: catalog names first (buildings, recipes, resources, labors, roles,
        /// blessings, biomes, goals), otherwise the id in words with entity numbers dropped.
        /// </summary>
        public static string Pretty(string name)
        {
            if (string.IsNullOrEmpty(name))
                return "";
            foreach (var kind in PrettyOrder)
                if (Names.IsAuthored(kind, name))
                    return Names.Of(kind, name);
            if (NowLines.IsAuthoredGoal(name))
                return NowLines.GoalText(name);
            return Names.Humanize(name);
        }

        private static readonly NameKind[] PrettyOrder =
        {
            NameKind.Building, NameKind.Recipe, NameKind.Resource, NameKind.Labor, NameKind.Role, NameKind.Upgrade,
            NameKind.Phase, NameKind.Biome, NameKind.Category, NameKind.Creature, NameKind.Material,
        };

        /// <summary>The display name of an id of a known kind; never the id itself.</summary>
        public static string Name(NameKind kind, string id) => Names.Of(kind, id);

        /// <summary>A building in a village, numbered among its kind: "Den 3".</summary>
        public static string Name(Village village, Building building) => Names.Building(village, building);

        /// <summary>A stockpile by what it belongs to: "Food Store", "Log pile", "Stockpile 2".</summary>
        public static string Name(Village village, Stockpile pile) => Names.Pile(village, pile);

        /// <summary>A number with at most <paramref name="decimals"/> decimals, invariant culture, grouped.</summary>
        public static string Num(double v, int decimals = 0) => Numbers.Num(v, decimals);

        /// <summary>A whole number, invariant culture, grouped: "19,995".</summary>
        public static string Whole(double v) => Numbers.Whole(v);

        /// <summary>"Day 1 · 06:00" from the world time and the pacing day scale.</summary>
        public static string Clock(double timeSeconds, double dayScale) => GameClock.Clock(timeSeconds, dayScale);

        /// <summary>"06:00".</summary>
        public static string TimeOfDay(double timeSeconds, double dayScale) => GameClock.TimeOfDay(timeSeconds, dayScale);

        /// <summary>A duration such as the age of a count: "40 s", "12 min", "6 h".</summary>
        public static string Age(double seconds) => GameClock.Duration(seconds);

        /// <summary>"12 min ago", "just now".</summary>
        public static string Ago(double seconds) => GameClock.Ago(seconds);

        /// <summary>"1 study", "2 studies".</summary>
        public static string Plural(int n, string one, string many) => Numbers.Plural(n, one, many);

        /// <summary>"3 logs", "1 log", "5 food".</summary>
        public static string Amount(double amount, string resource) => Names.Amount(amount, resource);

        /// <summary>"10 %" from 0.1.</summary>
        public static string Percent(double fraction) => Numbers.Percent(fraction);

        /// <summary>What the cat is doing now: "Bringing water to the Food Store · 9 tiles".</summary>
        public static string NowLine(World w, Village v, Cat c) => NowLines.NowLine(w, v, c);

        /// <summary>"Because there is no path", "Because Steward Hazel ordered a haul", or "".</summary>
        public static string CauseLine(World w, Village v, Cat c) => NowLines.CauseLine(w, v, c);

        /// <summary>The full sentence for a blocked or failure code, with a next step.</summary>
        public static string Reason(string code) => Reasons.Reason(code);

        /// <summary>A reason with its typed arguments: "Needs 5 research · you have 0.4".</summary>
        public static string Reason(string code, IReadOnlyDictionary<string, string> args) => Reasons.Reason(code, args);

        /// <summary>The short lowercase cause of a blocked code: "there is no path".</summary>
        public static string Cause(string code) => Reasons.Cause(code);

        /// <summary>The text of any action result code: "Planned" or a refusal sentence.</summary>
        public static string Result(string code) => Reasons.Result(code);

        /// <summary>The confirmation verb for an action kind: "Planned"; "" for routine steering.</summary>
        public static string Confirm(string actionKind) => Confirmations.Confirm(actionKind);

        /// <summary>The confirmation with its subject: "Den planned", "Studied Basic Tools".</summary>
        public static string Confirm(GameAction action, Village village = null) => Confirmations.Confirm(action, village);

        /// <summary>A refused action in the colony's voice; never an engine message or exception text.</summary>
        public static string ActionError(ActionResult r) => Reasons.ActionError(r);

        /// <summary>An effect sentence: "+10 % hunt yield", "Unlocks the Clothier".</summary>
        public static string Effect(string payloadKind, string target, double value) => Effects.Effect(payloadKind, target, value);

        /// <summary>An effect sentence with the building attribute: "Sawmill works 10 % faster".</summary>
        public static string Effect(ResearchPayload payload) => Effects.Of(payload);

        /// <summary>One line per event in the colony's voice, without a time stamp.</summary>
        public static string EventLine(Event e) => EventLines.Line(e);

        /// <summary>One line per event with names resolved from the village.</summary>
        public static string EventLine(Village v, Event e) => EventLines.Line(e, v);

        /// <summary>"06:21" for today's events, "Day 1 · 06:21" for older ones.</summary>
        public static string EventStamp(World w, Event e) =>
            w == null || e == null ? "" : GameClock.Stamp(e.Time, w.TimeSeconds, w.CurrentPacing().DayScale);

        /// <summary>"3 logs, 5 food"; "nothing" when empty.</summary>
        public static string Goods(IEnumerable<Stack> goods)
        {
            var parts = goods == null ? new List<string>() : goods.Where(s => s != null && s.Amount > 1e-9).Select(s => Names.Amount(s.Amount, s.Resource)).ToList();
            return parts.Count == 0 ? "nothing" : string.Join(", ", parts);
        }

        /// <summary>"Hunting 4 · Building 2": skill levels by labor, best first.</summary>
        public static string Skills(IEnumerable<Stack> skills)
        {
            var parts = skills == null ? new List<string>() : skills.Where(s => s != null && s.Amount >= 0.5).OrderByDescending(s => s.Amount)
                .Select(s => Names.Of(NameKind.Labor, s.Resource) + " " + Numbers.Floor(s.Amount)).ToList();
            return parts.Count == 0 ? "No skills yet" : string.Join(" · ", parts);
        }

        /// <summary>
        /// The last physical count of a resource: the number (compact) or the number with the count's age.
        /// Before any count: "—" (compact) or "not counted yet".
        /// </summary>
        public static string Reported(World world, Village village, string resource, bool compact = false)
        {
            var counted = village.Stockpiles.Where(p => p.CountedAt >= 0).ToList();
            if (counted.Count == 0)
                return compact ? "—" : "not counted yet";
            string value = Numbers.Whole(world.ReportedTotal(village, resource));
            if (compact)
                return value;
            double age = world.TimeSeconds - counted.Max(p => p.CountedAt);
            return value + " · counted " + GameClock.Ago(age);
        }

        public static string SectionSummary(string section)
        {
            switch (section)
            {
                case "Village": return "Your colony at a glance";
                case "Cats": return "Every pair of paws has a purpose";
                case "Build": return "Plan a place, connect it, bring materials";
                case "Work": return "Direct today's work and staff workshops";
                case "Stores": return "Goods, counts and equipment";
                case "Research": return "New crafts, buildings and ways to thrive";
                case "Officers": return "Give a specialist responsibility";
                case "Shrine": return "Offerings, blessings and rituals";
                case "World": return "Terrain, scouting and the dungeon";
                case "Defense": return "Train guards and protect the colony";
                case "Trade": return "Exchange goods with passing visitors";
                case "Routes": return "Move cargo by rail and water";
                case "Events": return "What has happened in your colony";
                default: return "Needs, work and next steps";
            }
        }

        /// <summary>The player sentence for a blocked reason or message; same as <see cref="Reason(string)"/>.</summary>
        public static string HumanReason(string reason) => Reasons.Reason(reason);
    }
}
