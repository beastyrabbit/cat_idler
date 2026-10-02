using System.Collections.Generic;

namespace IdleCatForest.Simulation
{
    /// <summary>
    /// Saved colony-automation state. Every field has a default that reproduces the behaviour of a
    /// save written before it existed, so old saves load unchanged.
    /// </summary>
    public partial class Village
    {
        /// <summary>The Leader's coarse stock count (two significant figures) and when it was taken; -1 before the first count.</summary>
        public List<Stack> LeaderEstimate = new List<Stack>();
        public double LeaderEstimateAt = -1;

        /// <summary>Offices that have been held at least once. Automatic appointment only fills an office that never was, so a vacate sticks.</summary>
        public List<string> OfficesFilled = new List<string>();

        /// <summary>"buildingId|catId" for each staffed slot whose first queue was already seen, so a default recipe is queued once per assignment.</summary>
        public List<string> QueueDefaultsApplied = new List<string>();

        /// <summary>The last time Emergency pre-empted a cat, per job kind (Resource = kind, Amount = time).</summary>
        public List<Stack> EmergencyAt = new List<Stack>();

        /// <summary>Layout signature of the last housing siting scan that found no site, and when it may be retried regardless.</summary>
        public string HousingSitingLayout = "";
        public double HousingSitingRetryAt;
    }
}
