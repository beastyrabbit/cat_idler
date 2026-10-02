namespace IdleCatForest.Simulation
{
    /// <summary>
    /// Blocked-reason codes written to <c>BlockedReason</c> on jobs, cats and slots. Values are
    /// save and wire data and the keys the player text is written against: never change a value.
    /// </summary>
    public static class Blocked
    {
        // Set by the job system today.
        public const string ForeignTerritory = "foreign_territory";
        public const string ExpansionFootprintBlocked = "expansion_footprint_blocked";
        public const string InfrastructureFootprintBlocked = "infrastructure_footprint_blocked";
        public const string BuildingEntranceDisconnected = "building_entrance_disconnected";
        public const string QueuePaused = "queue_paused";
        public const string MissingScaffoldInput = "missing_scaffold_input";
        public const string UnknownRecipe = "unknown_recipe";
        public const string MissingRecipeInput = "missing_recipe_input";
        public const string WallMaterialsMissing = "wall_materials_missing";
        public const string PavingInputMissing = "paving_input_missing";
        public const string OutputStorageFullOrUnreachable = "output_storage_full_or_unreachable";
        public const string WorkerRequired = "worker_required";
        public const string SourceChanged = "source_changed";
        public const string BuildingResearchRequired = "building_research_required";
        public const string SiteRequired = "site_required";

        // Set by movement when a route cannot be found; the job system reads it.
        public const string BlockedRoute = "blocked_route";

        // Pre-declared for Wave 2 (nothing sets these yet). RouteBlocked is distinct from BlockedRoute.
        public const string MissingMaterial = "missing_material";
        public const string RouteBlocked = "route_blocked";
        public const string SourceExcluded = "source_excluded";
    }
}
