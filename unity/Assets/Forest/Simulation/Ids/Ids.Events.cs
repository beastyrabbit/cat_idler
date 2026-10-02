namespace IdleCatForest.Simulation
{
    /// <summary>
    /// Event kinds passed to <c>Note</c>. Values are saved in event logs and used as UI-text keys:
    /// never rename or reuse one.
    /// </summary>
    public static class EventKind
    {
        // In use (World/ and Actions/).
        public const string Research = "research";
        public const string Tithe = "tithe";

        // Pre-declared for Wave 2.
        public const string Milestone = "milestone";
        public const string RaidWarning = "raid_warning";
        public const string RaidRepelled = "raid_repelled";
        public const string BuildingCompleted = "building_completed";
        public const string Discovery = "discovery";
        public const string StorageFull = "storage_full";
        public const string StorageEmpty = "storage_empty";
        public const string RankUp = "rank_up";
        public const string Birth = "birth";
        public const string Arrival = "arrival";
        public const string Death = "death";
        public const string ExpeditionLoot = "expedition_loot";
        public const string Error = "error";
    }
}
