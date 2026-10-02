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
    }

    /// <summary>
    /// What needs the player's attention in a village. Stub: nothing yet, so the list and
    /// the per-section counts are empty.
    /// </summary>
    public static class Attention
    {
        private static readonly IReadOnlyList<AttentionItem> NoItems = Array.Empty<AttentionItem>();
        private static readonly IReadOnlyDictionary<string, int> NoCounts =
            new ReadOnlyDictionary<string, int>(new Dictionary<string, int>());

        public static IReadOnlyList<AttentionItem> Compute(World world, Village village) => NoItems;

        /// <summary>Item counts keyed by <see cref="AttentionItem.Section"/>; sections with none are absent.</summary>
        public static IReadOnlyDictionary<string, int> CountBySection(World world, Village village) => NoCounts;
    }
}
