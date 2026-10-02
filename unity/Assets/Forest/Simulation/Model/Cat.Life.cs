namespace IdleCatForest.Simulation
{
    /// <summary>
    /// Saved state of a cat's own life: its need trip (where it eats, drinks or sleeps, how long it
    /// lingers, how often a blocked trip has failed) and its idle activity. Saves written before these
    /// fields existed load with the defaults: no trip spot, no idle spot and a metabolism of 1.
    /// </summary>
    public partial class Cat
    {
        /// <summary>True once the first life tick seeded this cat's needs and metabolism (demo profile only).</summary>
        public bool LifeSeeded;
        /// <summary>Need decay multiplier; 1 for classic worlds and old saves.</summary>
        public double Metabolism = 1;

        /// <summary>Where the current need trip is served: a store's access tile, a water edge or a sleeping spot.</summary>
        public bool HasNeedSpot; public Int2 NeedSpot;
        /// <summary>The current drink is taken at open water rather than from a store.</summary>
        public bool NeedAtWater;
        /// <summary>When the serving at <see cref="NeedSpot"/> applies; negative until the cat arrives.</summary>
        public double NeedDwellUntil = -1;
        /// <summary>When the walk to the need spot first failed; negative while the route is open.</summary>
        public double NeedBlockedSince = -1;
        /// <summary>Consecutive need trips that found no supply or no route; doubles the next retry delay.</summary>
        public int NeedBackoff;
        /// <summary>Storage that receives this cat's work cargo before a critical need trip; empty when none.</summary>
        public string NeedDepositPileId = "";

        /// <summary>Where the current idle activity happens.</summary>
        public bool HasIdleSpot; public Int2 IdleSpot;
        /// <summary>When the idle activity was chosen, and when its dwell at the spot ends (negative until arrival).</summary>
        public double IdleStartedAt, IdleUntil = -1;
    }
}
