namespace IdleCatForest.Simulation
{
    /// <summary>
    /// Job kinds stored in <see cref="Job.Kind"/> and <see cref="Job.OriginalKind"/>, plus the
    /// request aliases <c>Request</c> normalises into them. Values are save and wire data:
    /// never change a value, only add new ones.
    /// </summary>
    public static class JobKind
    {
        // Gathering and survey jobs.
        public const string Hunt = "hunt";
        public const string Water = "water";
        public const string Logs = "logs";
        public const string Quarry = "quarry";
        public const string Fibre = "fibre";
        public const string Fish = "fish";
        public const string Scout = "scout";
        public const string Replant = "replant";

        // Village, production and logistics jobs.
        public const string Train = "train";
        public const string Expand = "expand";
        public const string Haul = "haul";
        public const string Build = "build";
        public const string Production = "production";
        public const string Offering = "offering";
        public const string MouseFarm = "mouse_farm";

        // Infrastructure jobs.
        public const string Road = "road";
        public const string Rail = "rail";
        public const string Bridge = "bridge";
        public const string Dock = "dock";
        public const string Wagon = "wagon";
        public const string Vessel = "vessel";

        // Request aliases normalised by Request; ExpandVillage is also the OriginalKind of restored legacy wall jobs.
        public const string SupplyFood = "supply_food";
        public const string LeaderPlanHunt = "leader_plan_hunt";
        public const string HuntExpedition = "hunt_expedition";
        public const string SupplyWater = "supply_water";
        public const string FetchWater = "fetch_water";
        public const string GatherLogs = "gather_logs";
        public const string ForageFibre = "forage_fibre";
        public const string ReplantTree = "replant_tree";
        public const string Explore = "explore";
        public const string TrainWarrior = "train_warrior";
        public const string ExpandVillage = "expand_village";
        public const string LeaderPlanHouse = "leader_plan_house";
        public const string BuildHouse = "build_house";
        public const string Ritual = "ritual";
        public const string CarryOffering = "carry_offering";
        public const string PerformOffering = "perform_offering";
    }

    /// <summary>Phases stored in <see cref="Job.Phase"/>. Values are save and wire data.</summary>
    public static class JobPhase
    {
        public const string Travel = "travel";
        public const string Fetch = "fetch";
        public const string ItemFetch = "item_fetch";
        public const string InputDelivery = "input_delivery";
        public const string Working = "working";
        public const string OutputDelivery = "output_delivery";
        public const string Return = "return";
    }
}
