using System;
using System.Collections.Generic;
using System.Globalization;
using IdleCatForest.Simulation;

namespace IdleCatForest.Presentation
{
    /// <summary>
    /// Player text for the widgets, the attention seal and popover, and the toasts. Pure C# without
    /// UnityEngine, like <see cref="UiText"/>, so W2-INT-b can move it under <c>UI/Text/</c> and expose it
    /// through UiText. Names and reasons already go through UiText; numbers use the invariant culture.
    /// </summary>
    public static class HudCopy
    {
        public const string Yes = "Yes";
        public const string No = "No";
        public const string RemoveQuestion = "Remove?";
        public const string Done = "Done";
        public const string Sent = "Sent";
        public const string Failed = "That did not work";
        public const string AttentionLabel = "Attention";
        public const string AttentionHeader = "Needs attention";
        public const string AttentionOrder = "most urgent first";
        public const string EscapeKey = "Esc";
        public const string Show = "Show";
        public const string Dismiss = "Dismiss";

        public static string Number(double value) => Math.Round(value).ToString("#,0", CultureInfo.InvariantCulture);

        /// <summary>The words beside the seal's count.</summary>
        public static string SealLabel(int count) => count == 0 ? "all calm" : count == 1 ? "needs you" : "need you";

        public static string AttentionCount(int count) => count.ToString(CultureInfo.InvariantCulture);

        /// <summary>The 7th popover row when more items exist than fit.</summary>
        public static string More(int hidden) => hidden.ToString(CultureInfo.InvariantCulture) + " more · open Events";

        public static string Empty(string lastSolved, string at) =>
            string.IsNullOrEmpty(lastSolved) ? "Nothing needs you right now." : "Nothing needs you. Last solved: " + lastSolved + " at " + at + ".";

        // ---- attention items -------------------------------------------------------------------

        /// <summary>The headline of an attention row.</summary>
        public static string Title(AttentionItem item, World world, Village village)
        {
            string count = item.Arg(AttentionArg.Count);
            int n = Int(count);
            switch (RuleOf(item))
            {
                case AttentionRule.Raid:
                    return n > 1 ? n + " raiding bands at the gate" : "Raiders at the gate";
                case AttentionRule.RaidWarning:
                    return "Raiders spotted nearby";
                case AttentionRule.ExplorerDanger:
                    return CatName(world, village, item.Arg(AttentionArg.Cat)) + (item.CauseCode == "attention.explorer_spent" ? " is worn out underground" : " is hurt underground");
                case AttentionRule.NeedThirst:
                    return Names(world, village, item, n) + (n == 1 ? " is thirsty" : " are thirsty");
                case AttentionRule.NeedHunger:
                    return Names(world, village, item, n) + (n == 1 ? " is hungry" : " are hungry");
                case AttentionRule.NeedHealth:
                    return Names(world, village, item, n) + (n == 1 ? " needs care" : " need care");
                case AttentionRule.NoBed:
                    return n == 1 ? CatName(world, village, item.Arg(AttentionArg.Cat)) + " has no bed" : n + " cats have no bed";
                case AttentionRule.WorkplaceBlocked:
                    return BuildingName(village, item.Arg(AttentionArg.Building)) + " is stuck";
                case AttentionRule.ResearchUnstaffed:
                    return BuildingName(village, item.Arg(AttentionArg.Building)) + " has no scholar";
                case AttentionRule.StudyAffordable:
                    return "You can study " + StudyName(item.Arg(AttentionArg.Study));
                case AttentionRule.WorkplaceVacant:
                    return n > 1 ? n + " workplaces have no worker" : BuildingName(village, item.Arg(AttentionArg.Building)) + " has no worker";
                case AttentionRule.IdleCats:
                    return n + " cats are idle";
                case AttentionRule.StaleCounts:
                    return item.CauseCode == "attention.stale_counts.never" ? "Food and water were never counted" : "Food and water counts are old";
                default:
                    return UiText.Pretty(RuleOf(item));
            }
        }

        /// <summary>The dim cause line under the headline.</summary>
        public static string Cause(AttentionItem item, World world, Village village)
        {
            switch (item.CauseCode)
            {
                case "":
                    break;
                case "attention.explorer_hurt":
                case "attention.explorer_spent":
                    return "Bring them home before the next fight";
                case "attention.explorer_homeward":
                    return "Already on the way home";
                case "attention.no_bed.no_den":
                    return "Every den is full · they sleep badly and tire fast";
                case "attention.no_bed.den_building":
                    return "A new den is being built";
                case "attention.research_unstaffed.cause":
                    return "No research comes in until a cat studies there";
                case "attention.study_affordable.cause":
                    var study = Catalog.Study(item.Arg(AttentionArg.Study));
                    return study == null ? "" : "Costs " + Number(study.Cost) + " research · you have " + Number(Math.Floor(village?.ResearchPoints ?? 0));
                case "attention.workplace_vacant.cause":
                    return CatName(world, village, item.Arg(AttentionArg.Cat)) + " is free to work there";
                case "attention.idle_cats.cause":
                    return "No job matches them right now";
                case "attention.stale_counts.never":
                case "attention.stale_counts.old":
                    return "The Accounting Tent has nobody counting";
                case "need_supply_missing":
                    switch (RuleOf(item))
                    {
                        case AttentionRule.NeedThirst: return "No water in reach of the colony";
                        case AttentionRule.NeedHunger: return "No food in reach of the colony";
                        case AttentionRule.NeedHealth: return "No herbs or medicine in storage";
                    }
                    break;
                default:
                    return UiText.Reason(item.CauseCode);
            }
            switch (RuleOf(item))
            {
                case AttentionRule.Raid:
                    return "Rally your adults to drive them off";
                case AttentionRule.RaidWarning:
                    int seconds = Int(item.Arg(AttentionArg.Seconds));
                    return seconds > 60 ? "They reach the gate in about " + (seconds + 59) / 60 + " min" : "They are almost at the gate";
                default:
                    return "";
            }
        }

        /// <summary>The verb-first label of the row's one action; "" when the item has none.</summary>
        public static string Action(AttentionItem item, World world, Village village)
        {
            switch (RuleOf(item))
            {
                case AttentionRule.Raid:
                    return "Rally defenders";
                case AttentionRule.RaidWarning:
                    return "Prepare defenses";
                case AttentionRule.ExplorerDanger:
                    return item.ActionKind == "" ? "" : "Recall " + CatName(world, village, item.Arg(AttentionArg.Cat));
                case AttentionRule.NeedThirst:
                    return item.ActionKind == "RequestJob" ? "Send a water run" : "Open stores";
                case AttentionRule.NeedHunger:
                    return item.ActionKind == "RequestJob" ? "Send a hunt" : "Open stores";
                case AttentionRule.NeedHealth:
                    return "Open stores";
                case AttentionRule.NoBed:
                    return item.ActionKind == "" ? "" : "Plan a Den";
                case AttentionRule.WorkplaceBlocked:
                    return "Open work";
                case AttentionRule.ResearchUnstaffed:
                    return "Let " + CatName(world, village, item.Arg(AttentionArg.Cat)) + " study";
                case AttentionRule.StudyAffordable:
                    var study = Catalog.Study(item.Arg(AttentionArg.Study));
                    return "Study now · " + Number(study?.Cost ?? 0);
                case AttentionRule.WorkplaceVacant:
                    return "Let " + CatName(world, village, item.Arg(AttentionArg.Cat)) + " work";
                case AttentionRule.IdleCats:
                    return "Find them work";
                case AttentionRule.StaleCounts:
                    return "Let " + CatName(world, village, item.Arg(AttentionArg.Cat)) + " count";
                default:
                    return item.ActionKind == "" ? "" : UiText.Pretty(item.ActionKind);
            }
        }

        /// <summary>What the action button says for 2 s after it worked; reuses the verb.</summary>
        public static string Confirmed(AttentionItem item)
        {
            switch (RuleOf(item))
            {
                case AttentionRule.Raid: return "Rallied";
                case AttentionRule.NeedThirst:
                case AttentionRule.NeedHunger: return Sent;
                case AttentionRule.NoBed: return "Den planned";
                case AttentionRule.StudyAffordable: return "Studied";
                case AttentionRule.ExplorerDanger: return "Recalled";
                default: return item.ActionKind == AttentionAction.OpenSection ? "" : "Assigned";
            }
        }

        /// <summary>The rule id of an item (its id up to the first ':').</summary>
        public static string RuleOf(AttentionItem item)
        {
            if (item == null) return "";
            int colon = item.Id.IndexOf(':');
            return colon < 0 ? item.Id : item.Id.Substring(0, colon);
        }

        // ---- toasts ----------------------------------------------------------------------------

        /// <summary>The bold first line of a toast for an event kind.</summary>
        public static string ToastTitle(string kind)
        {
            switch (kind)
            {
                case EventKind.Raid: return "Raiders are coming";
                case EventKind.RaidWarning: return "Raiders spotted";
                case EventKind.RaidBreach: return "Raiders broke in";
                case EventKind.RaidLoss: return "Raiders got away";
                case EventKind.RaidRepelled:
                case EventKind.Defense: return "Raid repelled";
                case EventKind.Death: return "A cat has died";
                case EventKind.RecoveryBlocked: return "The colony cannot regrow";
                case EventKind.Recovery: return "A new beginning";
                case EventKind.Research: return "Study complete";
                case EventKind.Birth: return "A kitten is born";
                case EventKind.Pregnancy: return "A kitten is on the way";
                case EventKind.Migration: return "Wanderers";
                case EventKind.Arrival: return "A wanderer joined";
                case EventKind.Expansion: return "The colony grew";
                case EventKind.Dungeon: return "Expedition";
                case EventKind.ExpeditionLoot: return "Expedition loot";
                case EventKind.BuildingCompleted: return "Building finished";
                case EventKind.Milestone: return "Milestone reached";
                case EventKind.RankUp: return "Skill up";
                case EventKind.Upgrade: return "Upgrade";
                default: return UiText.Pretty(kind);
            }
        }

        /// <summary>The toast that stands in for events that did not fit.</summary>
        public static string ToastOverflow(int hidden) => hidden.ToString(CultureInfo.InvariantCulture) + (hidden == 1 ? " more event" : " more events");

        public const string ToastOverflowBody = "Open Events to read them all";

        // ---- helpers ---------------------------------------------------------------------------

        private static int Int(string text) => int.TryParse(text, NumberStyles.Integer, CultureInfo.InvariantCulture, out int n) ? n : 0;

        private static string Names(World world, Village village, AttentionItem item, int count)
        {
            var names = new List<string>();
            foreach (var key in new[] { AttentionArg.Cat, AttentionArg.Cat2, AttentionArg.Cat3 })
            {
                string id = item.Arg(key);
                if (id != "") names.Add(CatName(world, village, id));
            }
            if (names.Count == 0) return count + " cats";
            if (count <= 1) return names[0];
            if (count == 2 && names.Count >= 2) return names[0] + " and " + names[1];
            return names[0] + ", " + (names.Count > 1 ? names[1] + " and " : "") + (count - Math.Min(2, names.Count)) + " more";
        }

        public static string CatName(World world, Village village, string id)
        {
            var cat = village?.Cats.Find(c => c.Id == id) ?? world?.Cat(id);
            return cat != null && cat.Name != "" ? cat.Name : "A cat";
        }

        public static string BuildingName(Village village, string id)
        {
            var building = village?.Buildings.Find(b => b.Id == id);
            return Sentence(building == null ? "The workplace" : UiText.Name(NameKind.Building, building.Kind));
        }

        public static string StudyName(string id) => UiText.Name(NameKind.Study, id);

        /// <summary>Upper-cases the first letter, the way a building name starts a sentence.</summary>
        public static string Sentence(string text) =>
            string.IsNullOrEmpty(text) ? "" : char.ToUpperInvariant(text[0]) + text.Substring(1);
    }
}
