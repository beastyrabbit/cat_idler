namespace IdleCatForest.Simulation
{
    /// <summary>
    /// Job-side pacing: work durations, yields, regrowth, scouting, hauling, construction bills and
    /// work rates. Initialisers are the classic values; <c>DemoJobs</c> overrides them for the demo.
    /// </summary>
    public sealed partial class Pacing
    {
        // Assignment (JobAssign.cs).
        public double WorkerMinAgeHours = 12;
        public double AssignPreferenceScore = 100;
        public double AssignBoostScore = 20;

        // Work required per job kind, in work units (JobRequest.cs).
        public double HuntWork = 120;
        public double GatherWork = 60;
        public double TrainWork = 120;
        public double ScoutWork = 15;
        public double ExpansionWork = 10;
        public double OfferingWork = 30;
        public double OfferingHerbs = 5;
        public int AutoDenSearchRadius = 5;

        // Scouting (JobRequest.cs, JobPhases.Fetch.cs). Integer division by ScoutKnownTilesPerRadius is intended.
        public int ScoutSurveyRingStep = 2;
        public int ScoutSurveyMaxRadius = 10;
        public int ScoutBaseRadius = 12;
        public int ScoutKnownTilesPerRadius = 100;
        public int ScoutRadiusJitter = 8;
        public int ScoutSightRadius = 2;
        public double ScoutMaxSeconds = 180;

        // Hauling and spills (Haul.cs, JobCancel.cs, JobPhases.Delivery.cs).
        public double HaulBatchMax = 8;
        public double HaulSkillPerDelivery = 0.25;
        public double SpillPileCapacity = 1000000;
        public double ItemSpillPileCapacity = 10000;

        // Planning and storage retry cadence (JobTick.cs, JobPhases.Delivery.cs).
        public double JobPlanningIntervalSeconds = 1;
        public double StorageRetrySeconds = 1;

        // Job completion rewards and wear (JobCancel.cs).
        public double LaborSkillPerJob = 1;
        public double RoleExperiencePerJob = 1;
        public double SpecializationRoleExperience = 25;
        public double ToolWearPerJob = 0.1;

        // Segmented construction (JobPhases.Working.cs).
        public double WallSegmentWork = 10;
        public double PavingSegmentWork = 30;

        // Construction bill for an automatically sited scaffold (Construction.cs).
        public double ScaffoldTimberBase = 4;
        public double ScaffoldTimberPerExisting = 2;
        public double ScaffoldBlocksBase = 2;
        public double ScaffoldBlocksPerExisting = 1;
        public double ScaffoldMinWork = 1;

        // Completion yields and regrowth (JobCompletion.cs).
        public double ItemBaseCondition = 100;
        public double PreservationDurabilityMult = 1.25;
        public double OfferingHerbsPerBlessing = 5;
        public double MouseFarmFoodPerCycle = 4;
        public double TrainFightSkill = 3;
        public double ReplantRegrowSeconds = 86400;
        public double HuntMasteryYieldPerLevel = 0.12;
        public double FishCatchMax = 8;
        public double HuntHidePerFood = 0.25;
        public double HuntBonePerFood = 0.125;
        public double HuntRegrowSeconds = 3600;

        // Work rate (WorkRate.cs).
        public double WorkSkillDivisor = 300;
        public double WorkSkillBonusCap = 0.333;
        public double WorkStatBase = 0.75;
        public double WorkStatDivisor = 200;
        public double WorkCycleTimeFloor = 0.1;
        public double WorkToolBonus = 1.15;
        public double WorkRateMin = 0.1;
        public double WorkRateMax = 20;

        // Pre-declared for Wave 2; nothing reads these yet.
        public double JobWatchdogSeconds = 25;
        public double SourceExclusionSeconds = 60;
        public int BuildersPerSite = 2;

        partial void DemoJobs()
        {
            // Demo equals classic in Wave 1.
        }
    }
}
