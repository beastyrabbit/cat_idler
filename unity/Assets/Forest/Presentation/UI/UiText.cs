using System.Collections.Generic;
using System.Linq;
using System.Text.RegularExpressions;
using IdleCatForest.Simulation;

namespace IdleCatForest.Presentation
{
    /// <summary>What kind of identifier <see cref="UiText.Name"/> turns into player text.</summary>
    public enum NameKind { Other, Building, Resource, Recipe, Study, Labor, Role, Job, Phase, Goal, Biome, Upgrade, Item, Pile, Village, Cat }

    /// <summary>
    /// Every player-facing string goes through here. Pure C# without UnityEngine so it can be tested
    /// outside Unity. The Wave 1 stubs return exactly the strings the UI showed before the split;
    /// W2-15 replaces them with the authored copy.
    /// </summary>
    public static class UiText
    {
        public static string Pretty(string name)
        {
            if (string.IsNullOrEmpty(name)) return "";
            return Regex.Replace(name.Replace('_', ' '), "(?<=[a-z])(?=[A-Z])", " ");
        }

        public static string Name(NameKind kind, string id)
        {
            if (kind == NameKind.Study)
                return Catalog.Research.Find(study => study.Id == id)?.Name ?? id;
            return Pretty(id);
        }

        public static string Num(double v, int decimals = 0) => v.ToString(decimals <= 0 ? "0" : "0." + new string('0', decimals));

        public static string Whole(double v) => v.ToString("0");

        public static string Clock(double timeSeconds, double dayScale) => "Hour " + (timeSeconds / 3600).ToString("0.0");

        public static string Age(double seconds) => (seconds / 3600).ToString("0.0") + "h";

        public static string Plural(int n, string one, string many) => n + " " + (n == 1 ? one : many);

        public static string NowLine(World w, Village v, Cat c) => c == null ? "" : "Doing " + Pretty(c.Goal);

        public static string Reason(string code) => HumanReason(code);

        public static string Confirm(string actionKind) => Pretty(actionKind) + " accepted";

        public static string ActionError(ActionResult r) => r == null ? "" : r.Error;

        public static string Effect(string payloadKind, string target, double value) =>
            Pretty(payloadKind) + ": " + target + (value != 0 ? " " + value.ToString("0.##") : "");

        public static string EventLine(Event e) => (e.Time / 3600).ToString("0.0") + "h  " + e.Text;

        public static string Goods(IEnumerable<Stack> goods) =>
            goods.Any() ? string.Join(", ", goods.Select(s => s.Amount.ToString("0.#") + " " + s.Resource)) : "none";

        /// <summary>The last physical count of a resource, marked with an asterisk.</summary>
        public static string Reported(World world, Village village, string resource, bool compact = false)
        {
            bool any = village.Stockpiles.Any(p => p.CountedAt >= 0);
            if (any) return world.ReportedTotal(village, resource).ToString("0") + "*";
            return compact ? "—" : "uncounted";
        }

        public static string SectionSummary(string section)
        {
            switch (section)
            {
                case "Village": return "Your home, neighbors and shared world";
                case "Cats": return "Every pair of paws has a purpose";
                case "Build": return "Plan a place, connect it, bring materials";
                case "Work": return "Direct today's work and staff workshops";
                case "Stores": return "Physical goods, counts and equipment";
                case "Research": return "New craft, buildings and ways to thrive";
                case "Officers": return "Give a specialist responsibility";
                case "Shrine": return "Offerings, blessings and rituals";
                case "World": return "Terrain, exploration and dungeon levels";
                case "Defense": return "Train and protect your village";
                case "Trade": return "Exchange goods with passing visitors";
                case "Routes": return "Move cargo by rail and water";
                case "Events": return "What has happened in your village";
                default: return "Needs, work and next actions";
            }
        }

        public static string HumanReason(string reason)
        {
            switch (reason)
            {
                case "": return "";
                case "blocked_route": case "route_blocked": return "No walkable route. Check the gate, roads and any water along the way.";
                case "foreign_territory": return "This site belongs to another village. Choose land within your village.";
                case "building_entrance_disconnected": return "The entrance is cut off. Connect its road back to the shrine to resume construction.";
                case "infrastructure_footprint_blocked": return "Road or rail work is paused. A building, stockpile or farm occupies the reserved route. Clear that footprint to resume.";
                case "expansion_footprint_blocked": return "Expansion is paused because the new fence would cross a building, entrance or stockpile. Its materials and completed fence sections remain reserved.";
                case "queue_empty": return "Ready for work. Add a recipe to this worker's queue.";
                case "queue_paused": return "Production is paused. Resume the queue when you are ready.";
                case "worker_required": return "Waiting for a worker. Assign an available cat.";
                case "recipe_locked": case "building_research_required": return "Research required. Open Research to unlock this work.";
                case "missing_input": case "missing_recipe_input": return "Waiting for recipe ingredients in a reachable stockpile.";
                case "missing_grain": return "Waiting for grain. Grow or gather grain and deliver it to storage.";
                case "need_supply_missing": return "Food or water is missing or unreachable. Check supplies and paths.";
                case "output_storage_full": case "output_storage_full_or_unreachable": return "Finished goods are waiting. Make room in a matching stockpile and check its route.";
                case "station_local_full": return "The workshop is full. Move finished goods into storage.";
                case "input_claimed": return "Another worker has reserved these ingredients. Waiting for more supplies.";
                case "missing_scaffold_input": case "wall_materials_missing": return "Construction is waiting for its reserved materials to arrive.";
                case "paving_input_missing": return "Road work is waiting for paving materials.";
                case "blocked_harvest_route": return "The harvest cannot reach storage. Clear a walkable route.";
                case "blocked_handoff_route": return "The field's collection point is unreachable. Check the path.";
                case "handoff_full": return "The field's collection point is full. Haul its goods to storage.";
                case "return_route_blocked": return "The return route is blocked. Restore a path for this cargo.";
                case "return_storage_full": case "storage_full": return "Destination storage is full. Free space or add a matching stockpile.";
                case "source_changed": return "The original supply source has changed. Check its remaining goods.";
                case "site_required": return "Choose a clear construction site connected to the village.";
                default: return Pretty(reason);
            }
        }
    }
}
