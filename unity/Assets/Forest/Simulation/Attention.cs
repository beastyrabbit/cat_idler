using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;

namespace IdleCatForest.Simulation
{
    public enum AttentionSeverity
    {
        Info,
        Notice,
        Warning,
        Critical
    }

    /// <summary>One thing the player should look at, with the codes the UI turns into text. Immutable.</summary>
    public sealed class AttentionItem
    {
        public AttentionItem(string id, AttentionSeverity severity, string titleCode, string causeCode, string section, string anchor,
            string actionKind, IReadOnlyList<string> args)
        {
            Id = id ?? "";
            Severity = severity;
            TitleCode = titleCode ?? "";
            CauseCode = causeCode ?? "";
            Section = section ?? "";
            Anchor = anchor ?? "";
            ActionKind = actionKind ?? "";
            Args = args ?? Array.Empty<string>();
        }

        /// <summary>Stable across ticks while the condition lasts, so the UI can track it.</summary>
        public readonly string Id;
        public readonly AttentionSeverity Severity;
        /// <summary>UiText code for the headline.</summary>
        public readonly string TitleCode;
        /// <summary>UiText code for the cause, usually a blocked or result code.</summary>
        public readonly string CauseCode;
        /// <summary>The drawer section that resolves it.</summary>
        public readonly string Section;
        /// <summary>The entity id the item points at; empty for the village as a whole.</summary>
        public readonly string Anchor;
        /// <summary>The action the item offers; empty for none.</summary>
        public readonly string ActionKind;
        /// <summary>Arguments for the text codes.</summary>
        public readonly IReadOnlyList<string> Args;

        /// <summary>
        /// The value of a <c>key=value</c> argument (see <see cref="AttentionArg"/>), or "" when absent.
        /// Arguments name entities by id and carry plain numbers; the UI turns them into words.
        /// </summary>
        public string Arg(string key)
        {
            if (string.IsNullOrEmpty(key))
                return "";
            foreach (var arg in Args)
                if (arg != null && arg.Length > key.Length && arg[key.Length] == '=' && arg.StartsWith(key, StringComparison.Ordinal))
                    return arg.Substring(key.Length + 1);
            return "";
        }
    }

    /// <summary>Rule ids. An item's <see cref="AttentionItem.Id"/> is the rule id, or the rule id, ':' and the entity it is about.</summary>
    public static class AttentionRule
    {
        public const string Raid = "raid";
        public const string RaidWarning = "raid_warning";
        public const string ExplorerDanger = "explorer_danger";
        public const string NeedThirst = "need_thirst";
        public const string NeedHunger = "need_hunger";
        public const string NeedHealth = "need_health";
        public const string NoBed = "no_bed";
        public const string WorkplaceBlocked = "workplace_blocked";
        public const string ResearchUnstaffed = "research_unstaffed";
        public const string StudyAffordable = "study_affordable";
        public const string WorkplaceVacant = "workplace_vacant";
        public const string IdleCats = "idle_cats";
        public const string StaleCounts = "stale_counts";

        /// <summary>Every rule in evaluation order (also the order within one severity).</summary>
        public static readonly IReadOnlyList<string> All = new[]
        {
            Raid, RaidWarning, ExplorerDanger, NeedThirst, NeedHunger, NeedHealth, NoBed, WorkplaceBlocked, ResearchUnstaffed,
            StudyAffordable, WorkplaceVacant, IdleCats, StaleCounts
        };

        /// <summary>The <see cref="AttentionItem.TitleCode"/> of a rule: <c>attention.&lt;rule&gt;</c>.</summary>
        public static string TitleCode(string rule) => "attention." + rule;
    }

    /// <summary>Keys of the <c>key=value</c> entries in <see cref="AttentionItem.Args"/>.</summary>
    public static class AttentionArg
    {
        /// <summary>A cat id (the action's CatId when the item has an action).</summary>
        public const string Cat = "cat";
        /// <summary>Further cat ids named in the text (the second and third affected cat).</summary>
        public const string Cat2 = "cat2";
        public const string Cat3 = "cat3";
        /// <summary>A building id (the action's BuildingId).</summary>
        public const string Building = "building";
        /// <summary>A study id (the action's NodeId).</summary>
        public const string Study = "study";
        /// <summary>A job kind (the action's Name for RequestJob).</summary>
        public const string Job = "job";
        /// <summary>How many cats, buildings or raiders the item covers, as an invariant integer.</summary>
        public const string Count = "count";
        /// <summary>A resource id the text names.</summary>
        public const string Resource = "resource";
        /// <summary>Game seconds, as an invariant integer (age of a count, time to a raid).</summary>
        public const string Seconds = "seconds";
    }

    /// <summary>Drawer section ids items point at (the ids ForestUI.OpenPanel takes).</summary>
    public static class AttentionSection
    {
        public const string Work = "Work";
        public const string Research = "Research";
        public const string Build = "Build";
        public const string Stores = "Stores";
        public const string Defense = "Defense";
        public const string World = "World";
        public const string Cats = "Cats";
    }

    /// <summary>Action kinds an item may carry besides real GameAction kinds.</summary>
    public static class AttentionAction
    {
        /// <summary>Not a game action: the UI opens <see cref="AttentionItem.Section"/>.</summary>
        public const string OpenSection = "OpenSection";
    }

    /// <summary>
    /// What needs the player's attention in a village: a pure read of the world (it changes nothing,
    /// draws no randomness and runs no path search), so the HUD can call it on its refresh cadence.
    /// Items come ordered by severity, most severe first, then by rule order (<see cref="AttentionRule.All"/>);
    /// each offers at most one action, and only when that action is possible now.
    /// </summary>
    public static class Attention
    {
        /// <summary>A need below this (0-100) is urgent when nothing in reach can fix it.</summary>
        public const double NeedFloor = 30;
        /// <summary>Fewer free, idle adults than this is ordinary slack, not something to fix.</summary>
        public const int IdleCatsMin = 3;
        /// <summary>A food or water count older than this many game hours is stale.</summary>
        public const double StaleCountGameHours = 4;
        /// <summary>A raid warning stays up this long past its own lead time when no raid follows.</summary>
        public const double RaidWarningGraceSeconds = 60;

        private static readonly IReadOnlyList<AttentionItem> NoItems = Array.Empty<AttentionItem>();
        private static readonly IReadOnlyDictionary<string, int> NoCounts =
            new ReadOnlyDictionary<string, int>(new Dictionary<string, int>());
        private static readonly string[] StaffedWithoutRecipes = { "research_hut", "school", "accounting_tent", "mouse_farm" };
        private static readonly string[] ScholarBuildings = { "research_hut", "school" };

        public static IReadOnlyList<AttentionItem> Compute(World world, Village village)
        {
            if (world == null || village == null)
                return NoItems;
            var items = new List<AttentionItem>();
            var free = FreeCats(world, village);
            RaidRules(world, village, items);
            ExplorerRule(world, village, items);
            NeedRules(world, village, items);
            BedRule(world, village, items);
            WorkplaceBlockedRule(village, items);
            ResearchUnstaffedRule(village, free, items);
            StudyAffordableRule(village, items);
            WorkplaceVacantRule(village, free, items);
            IdleCatsRule(world, village, free, items);
            StaleCountsRule(world, village, free, items);
            // A stable sort: severity first, rule order within one severity.
            var ordered = new List<AttentionItem>(items.Count);
            for (var severity = AttentionSeverity.Critical; severity >= AttentionSeverity.Info; severity--)
                foreach (var item in items)
                    if (item.Severity == severity)
                        ordered.Add(item);
            return ordered.AsReadOnly();
        }

        /// <summary>Item counts keyed by <see cref="AttentionItem.Section"/>; sections with none are absent.</summary>
        public static IReadOnlyDictionary<string, int> CountBySection(World world, Village village)
        {
            var items = Compute(world, village);
            if (items.Count == 0)
                return NoCounts;
            var counts = new Dictionary<string, int>(StringComparer.Ordinal);
            foreach (var item in items)
            {
                counts.TryGetValue(item.Section, out int n);
                counts[item.Section] = n + 1;
            }
            return new ReadOnlyDictionary<string, int>(counts);
        }

        // ---- rules -----------------------------------------------------------------------------

        private static void RaidRules(World w, Village v, List<AttentionItem> items)
        {
            foreach (var raid in v.Raids)
                items.Add(Item(AttentionRule.Raid, raid.Id, AttentionSeverity.Critical, "", AttentionSection.Defense, raid.Id, "DefendRaid",
                    Arg(AttentionArg.Count, v.Raids.Count)));
            if (v.Raids.Count > 0)
                return;
            // The warning lasts from its event until the raid arrives, is repelled, or the lead time runs out.
            Event warning = null;
            foreach (var e in v.Events)
            {
                if (e.Kind == EventKind.RaidWarning)
                    warning = e;
                else if (warning != null && (e.Kind == EventKind.Raid || e.Kind == EventKind.RaidRepelled || e.Kind == EventKind.Defense))
                    warning = null;
            }
            if (warning == null)
                return;
            double age = w.TimeSeconds - warning.Time;
            if (age < 0 || age > w.Pace.RaidWarningSeconds + RaidWarningGraceSeconds)
                return;
            items.Add(Item(AttentionRule.RaidWarning, "", AttentionSeverity.Warning, "", AttentionSection.Defense, warning.EntityId, AttentionAction.OpenSection,
                Arg(AttentionArg.Seconds, Math.Max(0, w.Pace.RaidWarningSeconds - age))));
        }

        private static void ExplorerRule(World w, Village v, List<AttentionItem> items)
        {
            var pace = w.Pace;
            foreach (var c in v.Cats)
            {
                if (!c.Alive || !w.IsDungeonExplorer(c))
                    continue;
                double lowestNeed = Math.Min(c.Hunger, Math.Min(c.Thirst, c.Rest));
                bool hurt = c.Health < pace.ExpeditionRetreatHealth, spent = lowestNeed < pace.ExpeditionRetreatNeed;
                if (!hurt && !spent)
                    continue;
                var severity = c.Health < pace.ExpeditionGiveUpHealth || lowestNeed < pace.ExpeditionGiveUpNeed ? AttentionSeverity.Critical : AttentionSeverity.Warning;
                bool homeward = c.DungeonPhase == "returning" || c.DungeonPhase == "unloading";
                string cause = hurt ? "attention.explorer_hurt" : "attention.explorer_spent";
                items.Add(Item(AttentionRule.ExplorerDanger, c.Id, severity, homeward ? "attention.explorer_homeward" : cause, AttentionSection.World, c.Id,
                    homeward ? "" : "RecallExplorer", Arg(AttentionArg.Cat, c.Id)));
            }
        }

        private static void NeedRules(World w, Village v, List<AttentionItem> items)
        {
            NeedRule(w, v, items, AttentionRule.NeedThirst, c => c.Thirst, new[] { "water", "brew" }, JobKind.Water);
            NeedRule(w, v, items, AttentionRule.NeedHunger, c => c.Hunger, new[] { "fish", "preserves", "food" }, JobKind.Hunt);
            NeedRule(w, v, items, AttentionRule.NeedHealth, c => c.Health, new[] { "medicine", "herbs" }, "");
        }

        private static void NeedRule(World w, Village v, List<AttentionItem> items, string rule, Func<Cat, double> need, string[] supplies, string job)
        {
            bool supplied = false;
            foreach (var resource in supplies)
                if (w.Available(v, resource) >= 1)
                    supplied = true;
            var cats = new List<Cat>();
            string cause = "";
            foreach (var c in v.Cats)
            {
                if (!c.Alive || w.IsDungeonExplorer(c) || c.ControlledBy != "" || need(c) >= NeedFloor)
                    continue;
                bool stranded = c.BlockedReason == Blocked.BlockedRoute || c.BlockedReason == Blocked.RouteBlocked;
                if (supplied && !stranded)
                    continue;
                cats.Add(c);
                if (cause == "")
                    cause = supplied ? c.BlockedReason : "need_supply_missing";
            }
            if (cats.Count == 0)
                return;
            var args = new List<string> { Arg(AttentionArg.Count, cats.Count), Arg(AttentionArg.Cat, cats[0].Id) };
            if (cats.Count > 1)
                args.Add(Arg(AttentionArg.Cat2, cats[1].Id));
            if (cats.Count > 2)
                args.Add(Arg(AttentionArg.Cat3, cats[2].Id));
            // A supply run only helps when the supply is missing; a cut-off route needs the player's eyes.
            string action = job != "" && !supplied ? "RequestJob" : AttentionAction.OpenSection;
            if (action == "RequestJob")
                args.Add(Arg(AttentionArg.Job, job));
            items.Add(Item(rule, "", AttentionSeverity.Critical, cause, AttentionSection.Stores, cats[0].Id, action, args.ToArray()));
        }

        private static void BedRule(World w, Village v, List<AttentionItem> items)
        {
            Cat first = null;
            int homeless = 0;
            foreach (var c in v.Cats)
                if (c.Alive && c.BedId == "" && c.Migration != "arriving" && c.Migration != "departing")
                {
                    homeless++;
                    if (first == null)
                        first = c;
                }
            if (homeless == 0)
                return;
            Building site = null;
            foreach (var b in v.Buildings)
                if (b.Kind == "den" && !b.Completed)
                {
                    site = b;
                    break;
                }
            if (site != null)
                items.Add(Item(AttentionRule.NoBed, "", AttentionSeverity.Info, "attention.no_bed.den_building", AttentionSection.Build, site.Id, "",
                    Arg(AttentionArg.Count, homeless), Arg(AttentionArg.Cat, first.Id), Arg(AttentionArg.Building, site.Id)));
            else
                items.Add(Item(AttentionRule.NoBed, "", AttentionSeverity.Warning, "attention.no_bed.no_den", AttentionSection.Build, first.Id, "RequestJob",
                    Arg(AttentionArg.Count, homeless), Arg(AttentionArg.Cat, first.Id), Arg(AttentionArg.Job, JobKind.BuildHouse)));
        }

        private static void WorkplaceBlockedRule(Village v, List<AttentionItem> items)
        {
            foreach (var b in v.Buildings)
            {
                if (!b.Completed || b.Paused || !Staffed(b))
                    continue;
                string reason = b.BlockedReason;
                if (reason == "")
                    foreach (var slot in b.Slots)
                        if (slot.CatId != "" && !slot.Paused && slot.BlockedReason != "")
                        {
                            reason = slot.BlockedReason;
                            break;
                        }
                if (reason == "")
                    continue;
                items.Add(Item(AttentionRule.WorkplaceBlocked, b.Id, AttentionSeverity.Warning, reason, AttentionSection.Work, b.Id, AttentionAction.OpenSection,
                    Arg(AttentionArg.Building, b.Id)));
            }
        }

        private static void ResearchUnstaffedRule(Village v, List<Cat> free, List<AttentionItem> items)
        {
            if (free.Count == 0)
                return;
            foreach (var b in v.Buildings)
            {
                if (!b.Completed || b.Paused || Array.IndexOf(ScholarBuildings, b.Kind) < 0 || Staffed(b))
                    continue;
                var scholar = Best(free, "research");
                items.Add(Item(AttentionRule.ResearchUnstaffed, b.Id, AttentionSeverity.Warning, "attention.research_unstaffed.cause", AttentionSection.Work, b.Id,
                    "AssignWorker", Arg(AttentionArg.Building, b.Id), Arg(AttentionArg.Cat, scholar.Id)));
            }
        }

        private static void StudyAffordableRule(Village v, List<AttentionItem> items)
        {
            Study pick = null;
            foreach (var study in Catalog.Research)
            {
                if (study.Cost > v.ResearchPoints || v.Research.Contains(study.Id))
                    continue;
                bool ready = true;
                foreach (var p in study.Prerequisites)
                    if (!v.Research.Contains(p))
                        ready = false;
                if (!ready)
                    continue;
                // The earliest era first, then the cheapest; catalog order breaks ties.
                if (pick == null || study.Era < pick.Era || study.Era == pick.Era && study.Cost < pick.Cost)
                    pick = study;
            }
            if (pick == null)
                return;
            items.Add(Item(AttentionRule.StudyAffordable, pick.Id, AttentionSeverity.Notice, "attention.study_affordable.cause", AttentionSection.Research, "",
                "ResearchNode", Arg(AttentionArg.Study, pick.Id)));
        }

        private static void WorkplaceVacantRule(Village v, List<Cat> free, List<AttentionItem> items)
        {
            if (free.Count == 0)
                return;
            Building first = null;
            int vacant = 0;
            foreach (var b in v.Buildings)
            {
                if (!b.Completed || b.Paused || Staffed(b) || Array.IndexOf(ScholarBuildings, b.Kind) >= 0 || b.Kind == "accounting_tent" || !HasWork(v, b))
                    continue;
                vacant++;
                if (first == null)
                    first = b;
            }
            if (first == null)
                return;
            var worker = Best(free, LaborOf(first.Kind));
            items.Add(Item(AttentionRule.WorkplaceVacant, "", AttentionSeverity.Info, "attention.workplace_vacant.cause", AttentionSection.Work, first.Id, "AssignWorker",
                Arg(AttentionArg.Count, vacant), Arg(AttentionArg.Building, first.Id), Arg(AttentionArg.Cat, worker.Id)));
        }

        private static void IdleCatsRule(World w, Village v, List<Cat> free, List<AttentionItem> items)
        {
            if (free.Count < IdleCatsMin)
                return;
            items.Add(Item(AttentionRule.IdleCats, "", AttentionSeverity.Info, "attention.idle_cats.cause", AttentionSection.Work, free[0].Id, AttentionAction.OpenSection,
                Arg(AttentionArg.Count, free.Count), Arg(AttentionArg.Cat, free[0].Id)));
        }

        private static void StaleCountsRule(World w, Village v, List<Cat> free, List<AttentionItem> items)
        {
            if (free.Count == 0)
                return;
            Building tent = null;
            foreach (var b in v.Buildings)
                if (b.Kind == "accounting_tent" && b.Completed && !b.Paused && !Staffed(b))
                {
                    tent = b;
                    break;
                }
            if (tent == null)
                return;
            double newest = -1;
            bool stocked = false;
            foreach (var pile in v.Stockpiles)
            {
                if (pile.Kind != "storage")
                    continue;
                if (World.Amount(pile.Goods, "food") + World.Amount(pile.Goods, "water") > 0)
                    stocked = true;
                newest = Math.Max(newest, pile.CountedAt);
            }
            double limit = StaleCountGameHours * 3600 / Math.Max(1e-9, w.Pace.DayScale);
            if (!stocked || newest >= 0 && w.TimeSeconds - newest <= limit)
                return;
            var counter = Best(free, "research");
            items.Add(Item(AttentionRule.StaleCounts, "", AttentionSeverity.Info, newest < 0 ? "attention.stale_counts.never" : "attention.stale_counts.old", AttentionSection.Stores,
                tent.Id, "AssignWorker", Arg(AttentionArg.Building, tent.Id), Arg(AttentionArg.Cat, counter.Id),
                Arg(AttentionArg.Seconds, newest < 0 ? -1 : w.TimeSeconds - newest)));
        }

        // ---- helpers ---------------------------------------------------------------------------

        /// <summary>
        /// Adults the player can give work right now: alive, resident, on the surface, not steered, not on a
        /// job, workplace, office, need trip or route, and not the Leader. Ordered by id for determinism.
        /// </summary>
        private static List<Cat> FreeCats(World w, Village v)
        {
            var free = new List<Cat>();
            foreach (var c in v.Cats)
            {
                if (!c.Alive || c.Id == v.LeaderId || c.ControlledBy != "" || c.JobId != "" || c.BuildingId != "" || c.OfficerRole != "")
                    continue;
                if (c.AgeHours < w.Pace.WorkerMinAgeHours || c.Migration == "arriving" || c.Migration == "departing" || c.Position.Level != 0 || w.IsDungeonExplorer(c))
                    continue;
                if ((c.Goal ?? "").StartsWith(GoalKey.NeedPrefix, StringComparison.Ordinal) || c.Goal == GoalKey.Defending)
                    continue;
                bool driving = false;
                foreach (var route in v.Routes)
                    if (route.CatId == c.Id)
                        driving = true;
                if (!driving)
                    free.Add(c);
            }
            free.Sort((a, b) => string.CompareOrdinal(a.Id, b.Id));
            return free;
        }

        /// <summary>The free cat with the most skill in <paramref name="labor"/>; the lowest id breaks ties.</summary>
        private static Cat Best(List<Cat> free, string labor)
        {
            Cat best = null;
            double bestSkill = double.NegativeInfinity;
            foreach (var c in free)
            {
                double skill = World.Amount(c.Skills, labor);
                if (skill > bestSkill)
                {
                    best = c;
                    bestSkill = skill;
                }
            }
            return best;
        }

        private static bool Staffed(Building b)
        {
            if (b.WorkerId != "")
                return true;
            foreach (var slot in b.Slots)
                if (slot.CatId != "")
                    return true;
            return false;
        }

        /// <summary>A workplace worth staffing: one available recipe, or a station that works without recipes.</summary>
        private static bool HasWork(Village v, Building b)
        {
            if (Array.IndexOf(StaffedWithoutRecipes, b.Kind) >= 0)
                return true;
            foreach (var r in Catalog.Recipes)
                if (r.Building == b.Kind && Catalog.RecipeAvailable(v, r))
                    return true;
            return false;
        }

        private static string LaborOf(string buildingKind)
        {
            foreach (var r in Catalog.Recipes)
                if (r.Building == buildingKind)
                    return r.Labor;
            return "process";
        }

        private static AttentionItem Item(string rule, string subject, AttentionSeverity severity, string cause, string section, string anchor, string action, params string[] args) =>
            new AttentionItem(subject == "" ? rule : rule + ":" + subject, severity, AttentionRule.TitleCode(rule), cause, section, anchor, action, args);

        private static string Arg(string key, string value) => key + "=" + (value ?? "");

        private static string Arg(string key, double value) =>
            key + "=" + Math.Round(value).ToString(System.Globalization.CultureInfo.InvariantCulture);
    }
}
