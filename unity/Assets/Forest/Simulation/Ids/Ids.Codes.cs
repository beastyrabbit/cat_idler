namespace IdleCatForest.Simulation
{
    /// <summary>
    /// Stable action result codes carried in <see cref="ActionResult.Code"/>. Values are wire and
    /// UI-text keys: never rename or reuse one. Pre-declared for Wave 2; nothing returns them yet.
    /// </summary>
    public static class ResultCode
    {
        public const string Ok = "ok";
        public const string Queued = "queued";
        public const string Planned = "planned";
        public const string Assigned = "assigned";
        public const string Released = "released";
        public const string Studied = "studied";
        public const string Boosted = "boosted";
        public const string Cancelled = "cancelled";
        public const string Demolished = "demolished";
        public const string NotAWorkplace = "not_a_workplace";
        public const string UnknownTarget = "unknown_target";
        public const string RequiresPlayers = "requires_players";
        public const string FullGameOnly = "full_game_only";
        public const string MissingMaterial = "missing_material";
        public const string AllGroundsBusy = "all_grounds_busy";
        public const string NotImplemented = "not_implemented";
    }
}
