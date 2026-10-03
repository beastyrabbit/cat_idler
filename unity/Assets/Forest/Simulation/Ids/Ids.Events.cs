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

        // In use as literals in Jobs/, Systems/ and the domain files (added at W1-INT for the freeze;
        // call sites move to these constants in Wave 2). Birth, Death and Discovery below are in use too.
        public const string Completed = "completed";
        public const string Defense = "defense";
        public const string Dungeon = "dungeon";
        public const string Election = "election";
        public const string Emergency = "emergency";
        public const string Expansion = "expansion";
        public const string Migration = "migration";
        public const string Pregnancy = "pregnancy";
        public const string Raid = "raid";
        public const string RaidBreach = "raid_breach";
        public const string RaidLoss = "raid_loss";
        public const string Recovery = "recovery";
        public const string RecoveryBlocked = "recovery_blocked";
        public const string Trade = "trade";
        public const string Trader = "trader";
        public const string Upgrade = "upgrade";

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

        // Added in W2-08.
        /// <summary>A save-time or post-fault invariant repair (trimmed claims, unassigned double work, freed beds).</summary>
        public const string Repaired = "repaired";

        // Added at W2-INT-a: an office filled automatically once its study and building exist (W2-02).
        public const string Appointment = "appointment";
    }

    /// <summary>
    /// How loudly an event should be surfaced. Saved in event logs: never rename a value.
    /// Info events are routine (job completions, scout notes) and live in <c>Village.RoutineEvents</c>,
    /// so they never evict notable or alert events from <c>Village.Events</c>.
    /// </summary>
    public static class EventSeverity
    {
        public const string Info = "info";
        public const string Notable = "notable";
        public const string Alert = "alert";

        /// <summary>The default severity of a kind when its caller gives none (also used for logs saved before severities).</summary>
        public static string ForKind(string kind)
        {
            switch (kind)
            {
                case EventKind.Completed:
                case EventKind.Discovery:
                case EventKind.Repaired:
                    return Info;
                case EventKind.Raid:
                case EventKind.RaidWarning:
                case EventKind.RaidBreach:
                case EventKind.RaidLoss:
                case EventKind.Death:
                case EventKind.RecoveryBlocked:
                case EventKind.StorageEmpty:
                case EventKind.Error:
                    return Alert;
                default:
                    return Notable;
            }
        }
    }
}
