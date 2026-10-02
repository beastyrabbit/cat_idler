namespace IdleCatForest.Simulation
{
    /// <summary>Research, leadership and blessing progression. Initialisers are the classic values.</summary>
    public sealed partial class Pacing
    {
        /// <summary>Scholar work seconds per research point, before rate effects.</summary>
        public double ResearchDivisor = 600;
        public double LeaderStudyIntervalSeconds = 86400;

        // Elections.
        public double ElectionIntervalSeconds = 86400;
        public double ElectionDurationSeconds = 1800;

        // Recipe family studies.
        public double FamilySourcesInputFactor = 0.9;
        public double FamilySourcesOutputFactor = 1.1;
        public double FamilyBulkDurationFactor = 0.9;

        // Blessing upgrades: each level shortens the labour by Step, down to Floor of the base time.
        public double UpgradeSupplySpeedStep = 0.1;
        public double UpgradeSupplySpeedFloor = 0.55;
        public double UpgradeHuntStep = 0.1;
        public double UpgradeHuntFloor = 0.45;
        public double UpgradeBuildStep = 0.1;
        public double UpgradeBuildFloor = 0.45;
        public double UpgradeRitualStep = 0.12;
        public double UpgradeRitualFloor = 0.4;

        // Player boosts: seconds removed per click, with diminishing returns past the free clicks each minute.
        public double BoostSeconds = 10;
        public double BoostSecondsPerClickPower = 2;
        public int BoostFreePerMinute = 30;
        public double BoostDecay = 0.95;
        public double BoostDecayClicks = 10.0;
        public double BoostMinSeconds = 1;

        /// <summary>
        /// Demo: one research point per scholar-minute, the leader studies at most every 10 minutes, and
        /// elections never come round (they are hidden in the demo).
        /// </summary>
        partial void DemoResearch()
        {
            ResearchDivisor = 60;
            LeaderStudyIntervalSeconds = 10 * 60;
            ElectionIntervalSeconds = 100 * 365 * 86400.0;
        }
    }
}
