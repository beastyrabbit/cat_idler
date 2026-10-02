namespace IdleCatForest.Simulation
{
    /// <summary>
    /// A raid is a party of raiders sharing one health pool. Saves written before parties existed
    /// load with <see cref="Members"/> 0, which is the legacy single raider.
    /// </summary>
    public partial class Raid
    {
        /// <summary>Raider kind for presentation and text ("fox"); "" for a legacy raider.</summary>
        public string Kind = "";

        /// <summary>Raiders in the party; 0 is a legacy single raider.</summary>
        public int Members;

        /// <summary>The party's starting health; 0 when unknown (legacy raids use <see cref="Health"/>).</summary>
        public double MaxHealth;

        /// <summary>True for the scripted demo band.</summary>
        public bool Scripted;
    }

    // Raid-related state of other model types lives here so all raid fields stay in one model file.
    public partial class Cat
    {
        /// <summary>Raid id this cat was rallied against, "gate" while waiting for a warned raid, "" when not rallied.</summary>
        public string RalliedTo = "";
    }

    public partial class Village
    {
        /// <summary>Scripted demo raid progress: 0 not started, 1 warned, 2 spawned.</summary>
        public int ScriptedRaidStage;

        /// <summary>Game time from which the next rally may be called.</summary>
        public double RallyReadyAt;

        /// <summary>Raids this village has driven off.</summary>
        public int RaidsRepelled;
    }
}
