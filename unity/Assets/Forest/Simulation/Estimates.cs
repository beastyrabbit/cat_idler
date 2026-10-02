using System;
using System.Linq;

namespace IdleCatForest.Simulation
{
    /// <summary>
    /// What the colony believes it holds. Before an Accountant has counted every store, the Leader's
    /// coarse count stands in (refreshed on each direction tick, two significant figures); the
    /// founding stock counts as counted at founding, so a number exists from second 0.
    /// </summary>
    public partial class World
    {
        /// <summary>
        /// Stock of <paramref name="resource"/> as the colony knows it. <paramref name="isEstimate"/> stays
        /// true until an Accountant has counted every store; then the Accountant's reports are returned.
        /// Pure: never changes the world.
        /// </summary>
        public double EstimatedTotal(Village v, string resource, out bool isEstimate)
        {
            if (resource == "blessings")
            {
                isEstimate = false;
                return v.Blessings;
            }
            if (AccountantCounted(v))
            {
                isEstimate = false;
                return ReportedTotal(v, resource);
            }
            isEstimate = true;
            // Before the Leader's first count the founding stock stands in; nothing has moved it yet.
            return v.LeaderEstimateAt >= 0 ? Amount(v.LeaderEstimate, resource) : CoarseCount(Total(v, resource));
        }

        /// <summary>Seconds since the stock behind <see cref="EstimatedTotal"/> was counted.</summary>
        public double EstimateAgeSeconds(Village v)
        {
            if (AccountantCounted(v))
                return Math.Max(0, TimeSeconds - v.Stockpiles.Where(s => s.Kind == "storage").Min(s => s.CountedAt));
            return Math.Max(0, TimeSeconds - (v.LeaderEstimateAt >= 0 ? v.LeaderEstimateAt : v.FoundedAt));
        }

        /// <summary>Rounds a count to two significant figures, the precision of a glance at the piles.</summary>
        public static double CoarseCount(double amount)
        {
            if (!(amount > 0) || !Finite(amount))
                return 0;
            double step = Math.Pow(10, Math.Floor(Math.Log10(amount)) - 1);
            return Math.Round(amount / step, MidpointRounding.AwayFromZero) * step;
        }

        private bool AccountantCounted(Village v) => HasOfficer(v, "accountant") && v.Stockpiles.Any(s => s.Kind == "storage") && v.Stockpiles.Where(s => s.Kind == "storage").All(s => s.CountedAt >= 0);

        /// <summary>The Leader's count of every store, taken on each direction tick.</summary>
        private void CountStockForLeader(Village v)
        {
            v.LeaderEstimate.Clear();
            foreach (var resource in Catalog.Resources)
            {
                if (resource == "blessings")
                    continue;
                double amount = CoarseCount(Total(v, resource));
                if (amount > 0)
                    v.LeaderEstimate.Add(new Stack(resource, amount));
            }
            v.LeaderEstimateAt = TimeSeconds;
        }
    }
}
