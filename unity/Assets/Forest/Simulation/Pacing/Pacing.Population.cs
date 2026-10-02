namespace IdleCatForest.Simulation
{
    /// <summary>Aging, breeding, migration, probation and housing. Initialisers are the classic values.</summary>
    public sealed partial class Pacing
    {
        // Aging, in game hours of a cat's age.
        public double KittenUntilHours = 24;
        public double WorkingAgeHours = 12;
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
        /// <summary>How long an arrived wanderer may stay without a bed.</summary>
        public double ProbationSeconds = 36 * 3600;

        // Leader housing plans: after the grace, when spare beds run out and stock per cat holds.
        public double HousingPlanAfterSeconds = 6 * 3600;
        public int HousingSpareBeds = 1;
        public double HousingFoodPerCat = 3;
        public double HousingWaterPerCat = 4;

        partial void DemoPopulation()
        {
        }
    }
}
