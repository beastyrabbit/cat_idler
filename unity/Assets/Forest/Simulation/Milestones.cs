namespace IdleCatForest.Simulation
{
    /// <summary>Where a village stands on the demo's milestone spine. Immutable.</summary>
    public sealed class MilestoneStatus
    {
        /// <summary>No milestone: the spine is not active for this village.</summary>
        public static readonly MilestoneStatus None = new MilestoneStatus("", 0, 0, 0, "", false);

        public MilestoneStatus(string id, int index, int count, double progress, string hintCode, bool complete)
        {
            Id = id ?? "";
            Index = index;
            Count = count;
            Progress = progress;
            HintCode = hintCode ?? "";
            Complete = complete;
        }

        public readonly string Id;
        /// <summary>Zero-based position of this milestone on the spine.</summary>
        public readonly int Index;
        /// <summary>How many milestones the spine has.</summary>
        public readonly int Count;
        /// <summary>Progress towards this milestone, from 0 to 1.</summary>
        public readonly double Progress;
        /// <summary>A UiText code for the next-step hint; empty for none.</summary>
        public readonly string HintCode;
        public readonly bool Complete;
    }

    /// <summary>
    /// The milestone spine. Stub: no milestones exist yet, so every village reports
    /// <see cref="MilestoneStatus.None"/> and the demo is never complete.
    /// </summary>
    public static class Milestones
    {
        public static MilestoneStatus Current(World world, Village village) => MilestoneStatus.None;

        public static bool DemoComplete(World world, Village village) => false;
    }
}
