namespace IdleCatForest.Simulation
{
    /// <summary>
    /// Needs, sleep, idle life and the colony upkeep that runs inside the village tick.
    /// Initialisers are the classic values. Knobs marked "pre-declared" are not read yet.
    /// </summary>
    public sealed partial class Pacing
    {
        // Need decay, per game hour, before resilience and building services.
        public double HungerDecayPerHour = 20.0;
        public double ThirstDecayPerHour = 30.0;
        public double RestDecayPerHour = 12.0;
        // The resilience upgrade slows hunger and thirst by this much per level, down to the floor.
        public double ResiliencePerLevel = 0.08;
        public double ResilienceFloor = 0.45;

        // Health: regeneration while fed and watered, damage at an empty need.
        public double HealthRegenPerSecond = 0.002;
        public double HealthRegenMinNeed = 50;
        public double StarvationDamagePerMinute = 2.0;
        public double DehydrationDamagePerMinute = 3.0;
        /// <summary>Pre-declared: whether empty needs can kill a cat.</summary>
        public bool NeedsCanKill = true;
        /// <summary>
        /// Pre-declared: the critical health that need damage stops at when <see cref="NeedsCanKill"/> is false.
        /// Unused while needs can kill.
        /// </summary>
        public double NeedsHealthFloor = 0;

        // Moved from Wave 2 PACING-HANDOFF constants by W2-INT-a (classic values are the old literals).
        /// <summary>Per-cat seeded first-tick needs, metabolism and threshold jitter (W2-03).</summary>
        public bool NeedStagger = false;
        /// <summary>Range of the seeded first-tick Hunger, Thirst and Rest.</summary>
        public double NeedSeedMin = 55;
        public double NeedSeedMax = 100;
        /// <summary>A critical need waits for a delivery leg shorter than this many tiles.</summary>
        public int DeliveryLegFinishTiles = 15;
        /// <summary>The need retry delay doubles at most this often (3 s to 48 s).</summary>
        public int NeedBackoffMaxDoublings = 4;
        /// <summary>Reachable-route checks per need resource before trying the next one.</summary>
        public int NeedSourceSearchLimit = 4;
        /// <summary>An idle cat that has not reached its spot after this long picks again.</summary>
        public double IdleWalkLimitSeconds = 45;
        /// <summary>Idle cats that may stand on one tile.</summary>
        public int IdleTileShare = 2;
        /// <summary>Logs and stone the Leader keeps gathered while no Forester holds office (W2-02).</summary>
        public double LeaderStockTarget = 60;
        /// <summary>Concurrent Leader gather jobs per resource.</summary>
        public int LeaderGathersPerResource = 2;
        /// <summary>At most one Emergency pre-emption per job kind in this window.</summary>
        public double EmergencyCooldownSeconds = 60;
        /// <summary>Build, expand and production work past this share is never pre-empted.</summary>
        public double EmergencyProtectedProgress = 0.5;

        // A need trip starts below these levels.
        public double DrinkThreshold = 35;
        public double EatThreshold = 35;
        public double HealThreshold = 40;
        public double SleepThreshold = 20;
        // A cat keeps carrying work cargo while every need stays at or above these levels.
        public double CargoKeepThirst = 15;
        public double CargoKeepHunger = 15;
        public double CargoKeepHealth = 20;
        /// <summary>Delay before a failed need search or a blocked need route is retried.</summary>
        public double NeedRetrySeconds = 1;

        // Servings taken from a store and what one serving restores.
        public double NeedServing = 1;
        public double BrewServing = 0.25;
        public double WaterServingAfterBrew = 0.75;
        public double DrinkRestorePerServing = 50;
        public double EatRestorePerServing = 50;
        public double HealRestorePerServing = 25;

        // Sleep in a den.
        public double SleepRecoveryPerSecond = 0.15;
        public double SleepWakeRest = 95;

        // Pre-declared for living cats (W2-03).
        public double NeedDwellMin = 4;
        public double NeedDwellMax = 6;
        public double MetabolismMin = 0.85;
        public double MetabolismMax = 1.15;
        public double NeedThresholdJitter = 10;
        public double NeedTripBackoffSeconds = 3;
        public double IdleDwellMin = 8;
        public double IdleDwellMax = 40;
        /// <summary>Pre-declared: how far the sleep threshold rises at night.</summary>
        public double NightSleepBias = 15;

        // Idle wander around the village centre: one new spot per interval, within the radius.
        public int IdleWanderIntervalSeconds = 20;
        public int IdleWanderRadius = 4;

        // Village tick cadences.
        public int LeaderDirectSeconds = 10;
        public int UpkeepTickSeconds = 60;
        public double CompletedJobRetentionSeconds = 600;
        // Depleted forage and log tiles refill to these amounts when their regrowth time is due.
        public int RegrowthCheckSeconds = 60;
        public double RegrownLogs = 40;
        public double RegrownForage = 30;

        // Leader direction: crews per group of cats, and stock levels per living cat.
        public double CrewGroupSize = 15;
        public double HuntCrewPerGroup = 6.0;
        public double WaterCrewPerGroup = 2.0;
        public double FoodReservePerCat = 6;
        public double WaterReservePerCat = 8;
        public double EmergencyStockPerCat = 1;
        public int ScoutIntervalSeconds = 600;
        public double ForesterStockTarget = 40;
        public int ReplantIntervalSeconds = 600;
        public double FibreStockTarget = 10;
        public double LoremasterOfferingBlessingCap = 10;
        public double LoremasterOfferingMaterialsMin = 30;
        public double LoremasterOfferingMaterials = 5;
        public double LoremasterOfferingWork = 30;

        // Officer station staffing and recipe targets.
        public int OfficerReserveMin = 2;
        public double OfficerReservePerGroup = 2.0;
        public double OfficerItemsPerCat = 0.5;
        public double OfficerFoodPerCat = 8;
        public double OfficerStockTarget = 32;

        // Stations, farms and the accounting round.
        public double MouseFarmGrain = 1;
        public double MouseFarmWorkSeconds = 600;
        public double ResearchSkillSecondsPerPoint = 3600;
        public double FarmHarvestCarry = 8;
        public double FarmHaulSkillGain = 0.25;
        public double FarmSkillSecondsPerPoint = 3600;
        public double AccountingWaitSeconds = 60;
        public double AccountingCountSeconds = 5;

        // Spoilage per upkeep tick, as a fraction of unreserved stock.
        public double SpoilageRate = 0.0002;
        public double PreservesSpoilageRate = 0.00002;

        // Fish regrowth on water tiles, per game hour, applied every ecology tick.
        public int EcologyTickSeconds = 60;
        public double FishRegenPerHour = 0.5;

        /// <summary>
        /// Demo: needs move about 8x faster so eating, drinking and napping show up within minutes. A serving
        /// is an eighth of a unit and each unit restores 8x the points, so one trip restores what a classic
        /// trip does (cats go 8x as often) while food and water used per cat and hour stay at the classic
        /// rate and every stock target keeps its meaning.
        /// </summary>
        partial void DemoLife()
        {
            const double NeedSpeed = 8;
            HungerDecayPerHour *= NeedSpeed;
            ThirstDecayPerHour *= NeedSpeed;
            RestDecayPerHour *= NeedSpeed;
            NeedServing /= NeedSpeed;
            BrewServing /= NeedSpeed;
            WaterServingAfterBrew /= NeedSpeed;
            // The *RestorePerServing knobs are points per unit consumed (Needs.cs multiplies them by the dose).
            DrinkRestorePerServing *= NeedSpeed;
            EatRestorePerServing *= NeedSpeed;
            // Healing shares NeedServing, so one herb trip keeps healing 25 points.
            HealRestorePerServing *= NeedSpeed;
            // No starvation deaths in the demo (D16): need damage stops at critical health.
            NeedsCanKill = false;
            NeedsHealthFloor = 10;
            // Each cat gets its own first-tick needs and rhythm, so trips start staggered instead of together.
            NeedStagger = true;
            // A fished-out spot refills in about two hours instead of two days.
            FishRegenPerHour = 12;
        }
    }
}
