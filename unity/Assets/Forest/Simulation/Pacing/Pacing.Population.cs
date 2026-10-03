namespace IdleCatForest.Simulation
{
    /// <summary>Aging, breeding, migration, probation and housing. Initialisers are the classic values.</summary>
    public sealed partial class Pacing
    {
        // Aging, in game hours of a cat's age.
        public double KittenUntilHours = 24;
        public double ElderAgeHours = 240;
        /// <summary>Elder age for the leader and the loremaster.</summary>
        public double HonouredElderAgeHours = 288;
        /// <summary>Chance per upkeep tick that an elder dies of old age.</summary>
        public double OldAgeDeathChance = 0.0002;

        // Breeding: checked once per interval after the founding grace, with stock per living cat.
        public int BreedingCheckSeconds = 3600;
        public double BreedingAfterFoundingSeconds = 36 * 3600;
        public double BreedingFoodPerCat = 2.5;
        public double BreedingWaterPerCat = 2.5;
        public double ParentMinAgeHours = 24;
        public double ParentMaxAgeHours = 240;
        public double BreedChanceBase = 0.001;
        public double BreedBlessingStep = 0.02;
        public double BreedBlessingCap = 0.5;
        public double BreedBlessingWeight = 0.0007;
        public double GestationSeconds = 18 * 3600;
        /// <summary>A kitten's stats are the parents' mean plus up to half this spread either way.</summary>
        public double KittenStatSpread = 16;

        // Migration: the first wanderer after the founding grace, then one per interval while stock holds.
        public double MigrationFirstAtSeconds = 30 * 3600;
        public double MigrationIntervalSeconds = 12 * 3600;
        public double MigrationFoodPerCat = 4;
        public double MigrationWaterPerCat = 5;
        public int MigrationPopulationCap = 200;
        /// <summary>
        /// Whether a wanderer only comes while a bed is free. When true, a missing bed postpones the visit
        /// until one frees up instead of spending the interval.
        /// </summary>
        public bool MigrationRequiresFreeBed = false;
        /// <summary>How long an arrived wanderer may stay without a bed.</summary>
        public double ProbationSeconds = 36 * 3600;

        // Leader housing plans: after the grace, when spare beds run out and stock per cat holds.
        public double HousingPlanAfterSeconds = 6 * 3600;
        public int HousingSpareBeds = 1;
        public double HousingFoodPerCat = 3;
        public double HousingWaterPerCat = 4;
        /// <summary>A failed housing siting scan is repeated at the latest after this long on an unchanged layout (W2-02).</summary>
        public double HousingSitingRetrySeconds = 600;

        /// <summary>
        /// Demo (D6): the first pregnancy around minute 10 to 12 and a kitten about 8 minutes later, the first
        /// wanderer at minute 8 and another every 6 minutes while beds and food hold. Aging keeps the classic
        /// hours, so a demo kitten stays a kitten for the whole session.
        /// </summary>
        partial void DemoPopulation()
        {
            BreedingCheckSeconds = 60;
            BreedingAfterFoundingSeconds = 5 * 60;
            BreedChanceBase = 0.2;
            GestationSeconds = 8 * 60;
            MigrationFirstAtSeconds = 8 * 60;
            MigrationIntervalSeconds = 6 * 60;
            MigrationRequiresFreeBed = true;
            ProbationSeconds = 2 * 3600;
            // The player is asked to place the first den around minute 3 to 7; after minute 10 the leader
            // keeps housing one step ahead of growth.
            HousingPlanAfterSeconds = 10 * 60;
        }
    }
}
