namespace IdleCatForest.Simulation
{
    /// <summary>The visiting trader, caravans and transport. Initialisers are the classic values.</summary>
    public sealed partial class Pacing
    {
        /// <summary>
        /// Pre-declared: the first trader visit. Today a new village takes it from the
        /// <c>Trader.NextAt</c> field initialiser in Model.cs, which equals this value.
        /// </summary>
        public double TraderFirstAtSeconds = 3600;
        public double TraderIntervalSeconds = 12 * 3600;
        public double TraderWindowSeconds = 900;
        public int TraderReplanSeconds = 10;
        public double TraderSpeed = 0.8;
        public double TraderPurse = 1000;
        public double TraderStockBase = 20;
        public int TraderStockSpread = 40;
        /// <summary>Coins per unit bought from the trader.</summary>
        public double TraderBuyPrice = 2;
        /// <summary>Coins per quality step for an item sold to the trader.</summary>
        public double TraderSellPricePerQuality = 4;

        // Caravans between villages and vehicles on routes, in tiles per second.
        public double CaravanSpeed = 0.8;
        public double VehicleSpeed = 1;
        public double TransportSkillSecondsPerPoint = 3600;

        // Build work for transport infrastructure.
        public double LinearWorkPerTile = 30;
        public double InfrastructureWork = 120;

        partial void DemoTrade()
        {
        }
    }
}
