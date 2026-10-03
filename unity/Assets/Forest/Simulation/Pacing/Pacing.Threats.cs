namespace IdleCatForest.Simulation
{
    /// <summary>Raids, defence and dungeons. Initialisers are the classic values.</summary>
    public sealed partial class Pacing
    {
        // Raid triggers: checked once per interval after the grace, for a large and wealthy colony.
        public int RaidCheckSeconds = 3600;
        public double RaidGraceSeconds = 8 * 3600;
        public int RaidMinCats = 20;
        /// <summary>Stored materials plus refined goods above which raiders come.</summary>
        public double RaidMinWealth = 250;
        public double RaidChance = 0.1;
        /// <summary>Pre-declared: game time of the scripted demo raid; 0 disables it.</summary>
        public double DemoRaidAtSeconds = 0;
        /// <summary>Pre-declared: warning lead time before a raid.</summary>
        public double RaidWarningSeconds = 120;
        /// <summary>Pre-declared: whether raids can kill a cat.</summary>
        public bool RaidCanKill = true;

        // Raid movement and combat at the village.
        public double RaidSpeed = 0.5;
        public double DefenderStrikePerSecond = 0.1;
        public double RaiderStrikePerSecond = 0.03;
        public double DefenderWeaponWearPerSecond = 0.005;
        public double DefenderSkillSecondsPerPoint = 3600;
        public double RaidLootShare = 0.3;
        public double RaidLootMax = 20;
        public double RaidBreachDamage = 20;

        // Dungeon threat: per site, per distance band and per floor below the first.
        public double DungeonBaseThreat = 1.15;
        public double DungeonBandThreat = 0.65;
        public double DungeonFloorThreat = 0.85;
        public double GuardianBaseHealth = 15;
        public double GuardianFloorHealth = 9;
        public double GuardianBandHealth = 8;
        public double DungeonLootBase = 3;
        public double DungeonLootSecondary = 2;
        /// <summary>Pre-declared: how long a cleared floor takes to restock.</summary>
        public double FloorRestockSeconds = 600;

        // Expedition readiness and safety, on the 0 to 100 need and health scales.
        public double ReadinessFightSkillFactor = 0.25;
        public double ReadinessNeedNorm = 45;
        public double ExpeditionMinHealth = 65;
        public double ExpeditionMinNeed = 45;
        public double ExpeditionRetreatHealth = 50;
        public double ExpeditionRetreatNeed = 30;
        public double ExpeditionGiveUpHealth = 40;
        public double ExpeditionGiveUpNeed = 25;
        /// <summary>A cat descends further only while carrying less than this.</summary>
        public double ExpeditionDescendCargoLimit = 8;
        public int DungeonRevealTiles = 4;
        public double DungeonReplanSeconds = 1;

        // Dungeon combat and creatures.
        public double ExplorerStrikePerSecond = 1.8;
        public double ExplorerSkillSecondsPerPoint = 300;
        public double ExplorerWeaponWearPerSecond = 0.025;
        public double CreatureStrikePerSecond = 0.45;
        public double CreatureSpeed = 0.65;
        public int CreatureLeashTiles = 12;
        public int CreatureAggroTiles = 5;
        public double CreatureReplanSeconds = 1;

        /// <summary>
        /// Demo (D15, D16): a scripted fox band attacks at minute 15 after a two-minute warning, and raids
        /// never kill a cat. A cleared dungeon floor restocks after the classic 10 minutes. The random raid
        /// trigger keeps its classic grace, so no unscripted raid falls inside a demo session.
        /// </summary>
        partial void DemoThreats()
        {
            DemoRaidAtSeconds = 15 * 60;
            RaidWarningSeconds = 120;
            RaidCanKill = false;
            FloorRestockSeconds = 600;
        }
    }
}
