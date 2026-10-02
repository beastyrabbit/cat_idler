namespace IdleCatForest.Simulation
{
    /// <summary>
    /// Stable action result codes carried in <see cref="ActionResult.Code"/>. Values are wire and
    /// UI-text keys: never rename or reuse one. The first block is the list frozen at W1-INT; the
    /// later blocks were added by W2-06 and need player text (W2-INT-b).
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

        // W2-06 successes.
        public const string Founded = "founded";
        public const string Joined = "joined";
        public const string Appointed = "appointed";
        public const string Dismissed = "dismissed";
        public const string Voted = "voted";
        public const string Signed = "signed";
        public const string Preferred = "preferred";
        public const string PreferenceSet = "preference_set";
        public const string QueueUpdated = "queue_updated";
        public const string Hurried = "hurried";
        public const string Upgraded = "upgraded";
        public const string Offered = "offered";
        public const string Equipped = "equipped";
        public const string Unequipped = "unequipped";
        public const string Repaired = "repaired";
        public const string Controlling = "controlling";
        public const string ControlLeft = "control_left";
        public const string ControlKept = "control_kept";
        public const string Moving = "moving";
        public const string Stopped = "stopped";
        public const string Collected = "collected";
        public const string Deposited = "deposited";
        public const string Taken = "taken";
        public const string Ate = "ate";
        public const string Drank = "drank";
        public const string Resting = "resting";

        // W2-06 router and identity failures.
        /// <summary>Fallback for a handler outside W2-06's files that has not set a code yet.</summary>
        public const string Rejected = "rejected";
        public const string InvalidAction = "invalid_action";
        public const string InvalidArguments = "invalid_arguments";
        public const string SurfaceOnly = "surface_only";
        public const string IdentityRequired = "identity_required";
        public const string VillageDenied = "village_denied";
        public const string UnknownAction = "unknown_action";
        public const string DeveloperOnly = "developer_only";

        // W2-06 cat and workplace failures.
        public const string CatRequired = "cat_required";
        public const string CatBusy = "cat_busy";
        public const string NoWorker = "no_worker";
        public const string Kitten = "kitten";
        public const string Migrating = "migrating";
        public const string CatControlled = "cat_controlled";
        public const string ExplorerAway = "explorer_away";
        public const string TransportReturning = "transport_returning";
        public const string AllSlotsStaffed = "all_slots_staffed";
        public const string ScaffoldHasBuilder = "scaffold_has_builder";
        public const string NotAssigned = "not_assigned";
        public const string UnknownLabor = "unknown_labor";
        public const string NoSourceGoods = "no_source_goods";

        // W2-06 production queue failures.
        public const string StationRequired = "station_required";
        public const string SlotRequired = "slot_required";
        public const string RecipeLocked = "recipe_locked";
        public const string QueueFull = "queue_full";
        public const string QueueIndex = "queue_index";
        public const string UnknownEdit = "unknown_edit";

        // W2-06 boost, research and blessing failures.
        public const string NoActiveJob = "no_active_job";
        public const string NoActiveResearch = "no_active_research";
        public const string CannotAfford = "cannot_afford";
        public const string UnknownStudy = "unknown_study";
        public const string AlreadyResearched = "already_researched";
        public const string MissingPrerequisite = "missing_prerequisite";
        public const string NotBlessable = "not_blessable";
        public const string ReserveProtected = "reserve_protected";
        public const string InvalidResource = "invalid_resource";
        public const string NothingToOffer = "nothing_to_offer";
        public const string UpgradeMaxed = "upgrade_maxed";

        // W2-06 equipment failures.
        public const string RepairStationRequired = "repair_station_required";
        public const string SlotOccupied = "slot_occupied";
        public const string NotEquipped = "not_equipped";
        public const string NoStorage = "no_storage";

        // W2-06 village, office and governance failures.
        public const string OnePersonalVillage = "one_personal_village";
        public const string InvalidName = "invalid_name";
        public const string NoFoundingSite = "no_founding_site";
        public const string UnknownRole = "unknown_role";
        public const string OfficeLocked = "office_locked";
        public const string AlreadyOfficer = "already_officer";
        public const string OfficeVacant = "office_vacant";
        public const string NoElection = "no_election";
        public const string LeaderRequired = "leader_required";
        public const string PetitionClosed = "petition_closed";

        // W2-06 direct-control failures.
        public const string ControlRequired = "control_required";
        public const string ControlledByOther = "controlled_by_other";
        public const string InvalidDirection = "invalid_direction";
        /// <summary>Same string as <see cref="Blocked.BlockedRoute"/>, so one text serves both lists.</summary>
        public const string BlockedRoute = "blocked_route";
        public const string OutOfReach = "out_of_reach";
        public const string NothingInReach = "nothing_in_reach";
        public const string NothingToCollect = "nothing_to_collect";
        public const string StorageFull = "storage_full";
        public const string NeedFoodInCargo = "need_food_in_cargo";
        public const string NeedWaterInCargo = "need_water_in_cargo";
        public const string GoToShrine = "go_to_shrine";
        public const string NoReservedBed = "no_reserved_bed";
    }

    /// <summary>Keys of <see cref="ActionResult.Args"/>; numbers are invariant-culture strings.</summary>
    public static class ResultArg
    {
        public const string Resource = "resource";
        public const string Need = "need";
        public const string Have = "have";
        public const string Target = "target";
        public const string Kind = "kind";
        public const string Building = "building";
        public const string Study = "study";
        public const string Role = "role";
        public const string Seconds = "seconds";
        public const string Points = "points";
        public const string Diminished = "diminished";
        public const string Interrupted = "interrupted";
        public const string Enabled = "enabled";
        public const string Blessings = "blessings";
        public const string Amount = "amount";
        public const string Direction = "direction";
    }
}
