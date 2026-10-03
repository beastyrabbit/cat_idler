using System.Collections.Generic;

namespace IdleCatForest.Simulation
{
    /// <summary>
    /// Presentation metadata set by <c>Note</c>: where it happened, how loudly to surface it and typed
    /// arguments for player copy. <c>ActorName</c> (in LegacyContinuity.cs) is set by <c>Note</c> too.
    /// </summary>
    public partial class Event
    {
        /// <summary>The tile the event happened at, when it has one (the cat, building, raid or job position).</summary>
        public Int2? Position;

        /// <summary>
        /// <see cref="EventSeverity"/> value; "" on events saved before severities existed. Read it through
        /// <see cref="EffectiveSeverity"/>.
        /// </summary>
        public string Severity = "";

        /// <summary>Typed details for player copy (job, building, resource, amount, skill, rank ...); null when there are none.</summary>
        public Dictionary<string, string> Args;

        /// <summary>The saved severity, or the kind's default for events saved before severities existed.</summary>
        public string EffectiveSeverity() => string.IsNullOrEmpty(Severity) ? EventSeverity.ForKind(Kind) : Severity;
    }
}
