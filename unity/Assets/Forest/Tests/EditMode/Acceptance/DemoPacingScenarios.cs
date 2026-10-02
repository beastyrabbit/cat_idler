using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Reflection;
using System.Threading;
using IdleCatForest.Simulation;
using static IdleCatForest.Acceptance.ScenarioKit;

namespace IdleCatForest.Acceptance
{
    /// <summary>
    /// The demo pacing profile. <c>demo_pacing.*</c> pins the profile table and always runs.
    /// <c>demo_timing.*</c> measures the D6 journey on a demo-profile world with a minimal scripted player and is
    /// opt-in (<c>--include-demo-timing</c>): it depends on colony automation, living cats, founding and the scripted
    /// raid, so W2-INT-a owns making it green.
    /// </summary>
    public static class DemoPacingScenarios
    {
        public static IEnumerable<Scenario> Cases()
        {
            yield return new Scenario("demo_pacing.classic_equals_wave1_values", ClassicEqualsWave1);
            yield return new Scenario("demo_pacing.demo_differs_only_in_set_fields", DemoDiffersOnlyInSetFields);
            yield return new Scenario("demo_pacing.goods_per_hour_match_classic", GoodsPerHourMatchClassic);
            yield return new Scenario("demo_timing.curve", PrintCurve);
            yield return new Scenario("demo_timing.research_by_90s", () => Within(Curve.FirstResearch, 0, 90, "first research point"));
            yield return new Scenario("demo_timing.first_study_affordable_by_3min", () => Within(Curve.FirstStudyAffordable, 0, 180, "first study affordable"));
            yield return new Scenario("demo_timing.migrant_by_12min", () => Within(Curve.FirstMigrant, 0, 720, "first migrant"));
            yield return new Scenario("demo_timing.kitten_by_24min", () => Within(Curve.FirstKitten, 0, 1440, "first kitten"));
            yield return new Scenario("demo_timing.raid_warning_12_to_14min", () => Within(Curve.RaidWarning, 720, 840, "raid warning"));
            yield return new Scenario("demo_timing.no_need_deaths_in_40min", () => Check(Curve.NeedDeaths == 0, "Cats died of needs in the demo profile: " + Curve.NeedDeaths + " (" + Curve.Line + ")"));
            yield return new Scenario("demo_timing.ten_meals_or_drinks_in_10min", () => Check(Curve.EatOrDrinkFirstTenMinutes >= 10, "Only " + Curve.EatOrDrinkFirstTenMinutes + " eat or drink trips in the first 10 minutes (" + Curve.Line + ")"));
        }

        // Classic values field by field: the Wave 1 table frozen in docs/DEMO.md, plus the knobs W2-01 added.
        static readonly Dictionary<string, object> Wave1Classic = new Dictionary<string, object>
        {
            // Pacing.Jobs.cs (57)
            ["WorkerMinAgeHours"] = 12,
            ["AssignPreferenceScore"] = 100,
            ["AssignBoostScore"] = 20,
            ["HuntWork"] = 120,
            ["GatherWork"] = 60,
            ["TrainWork"] = 120,
            ["ScoutWork"] = 15,
            ["ExpansionWork"] = 10,
            ["OfferingWork"] = 30,
            ["OfferingHerbs"] = 5,
            ["AutoDenSearchRadius"] = 5,
            ["ScoutSurveyRingStep"] = 2,
            ["ScoutSurveyMaxRadius"] = 10,
            ["ScoutBaseRadius"] = 12,
            ["ScoutKnownTilesPerRadius"] = 100,
            ["ScoutRadiusJitter"] = 8,
            ["ScoutSightRadius"] = 2,
            ["ScoutMaxSeconds"] = 180,
            ["HaulBatchMax"] = 8,
            ["HaulSkillPerDelivery"] = 0.25,
            ["SpillPileCapacity"] = 1000000,
            ["ItemSpillPileCapacity"] = 10000,
            ["JobPlanningIntervalSeconds"] = 1,
            ["StorageRetrySeconds"] = 1,
            ["LaborSkillPerJob"] = 1,
            ["RoleExperiencePerJob"] = 1,
            ["SpecializationRoleExperience"] = 25,
            ["ToolWearPerJob"] = 0.1,
            ["WallSegmentWork"] = 10,
            ["PavingSegmentWork"] = 30,
            ["ScaffoldTimberBase"] = 4,
            ["ScaffoldTimberPerExisting"] = 2,
            ["ScaffoldBlocksBase"] = 2,
            ["ScaffoldBlocksPerExisting"] = 1,
            ["ScaffoldMinWork"] = 1,
            ["ItemBaseCondition"] = 100,
            ["PreservationDurabilityMult"] = 1.25,
            ["OfferingHerbsPerBlessing"] = 5,
            ["MouseFarmFoodPerCycle"] = 4,
            ["TrainFightSkill"] = 3,
            ["ReplantRegrowSeconds"] = 86400,
            ["HuntMasteryYieldPerLevel"] = 0.12,
            ["FishCatchMax"] = 8,
            ["HuntHidePerFood"] = 0.25,
            ["HuntBonePerFood"] = 0.125,
            ["HuntRegrowSeconds"] = 3600,
            ["WorkSkillDivisor"] = 300,
            ["WorkSkillBonusCap"] = 0.333,
            ["WorkStatBase"] = 0.75,
            ["WorkStatDivisor"] = 200,
            ["WorkCycleTimeFloor"] = 0.1,
            ["WorkToolBonus"] = 1.15,
            ["WorkRateMin"] = 0.1,
            ["WorkRateMax"] = 20,
            ["JobWatchdogSeconds"] = 25,
            ["SourceExclusionSeconds"] = 60,
            ["BuildersPerSite"] = 2,
            // Pacing.Life.cs (74)
            ["HungerDecayPerHour"] = 20.0,
            ["ThirstDecayPerHour"] = 30.0,
            ["RestDecayPerHour"] = 12.0,
            ["ResiliencePerLevel"] = 0.08,
            ["ResilienceFloor"] = 0.45,
            ["HealthRegenPerSecond"] = 0.002,
            ["HealthRegenMinNeed"] = 50,
            ["StarvationDamagePerMinute"] = 2.0,
            ["DehydrationDamagePerMinute"] = 3.0,
            ["NeedsCanKill"] = true,
            ["DrinkThreshold"] = 35,
            ["EatThreshold"] = 35,
            ["HealThreshold"] = 40,
            ["SleepThreshold"] = 20,
            ["CargoKeepThirst"] = 15,
            ["CargoKeepHunger"] = 15,
            ["CargoKeepHealth"] = 20,
            ["NeedRetrySeconds"] = 1,
            ["NeedServing"] = 1,
            ["BrewServing"] = 0.25,
            ["WaterServingAfterBrew"] = 0.75,
            ["DrinkRestorePerServing"] = 50,
            ["EatRestorePerServing"] = 50,
            ["HealRestorePerServing"] = 25,
            ["SleepRecoveryPerSecond"] = 0.15,
            ["SleepWakeRest"] = 95,
            ["NeedDwellMin"] = 4,
            ["NeedDwellMax"] = 6,
            ["MetabolismMin"] = 0.85,
            ["MetabolismMax"] = 1.15,
            ["NeedThresholdJitter"] = 10,
            ["NeedTripBackoffSeconds"] = 3,
            ["IdleDwellMin"] = 8,
            ["IdleDwellMax"] = 40,
            ["NightSleepBias"] = 15,
            ["IdleWanderIntervalSeconds"] = 20,
            ["IdleWanderRadius"] = 4,
            ["LeaderDirectSeconds"] = 10,
            ["UpkeepTickSeconds"] = 60,
            ["CompletedJobRetentionSeconds"] = 600,
            ["RegrowthCheckSeconds"] = 60,
            ["RegrownLogs"] = 40,
            ["RegrownForage"] = 30,
            ["CrewGroupSize"] = 15,
            ["HuntCrewPerGroup"] = 6.0,
            ["WaterCrewPerGroup"] = 2.0,
            ["FoodReservePerCat"] = 6,
            ["WaterReservePerCat"] = 8,
            ["EmergencyStockPerCat"] = 1,
            ["ScoutIntervalSeconds"] = 600,
            ["ForesterStockTarget"] = 40,
            ["ReplantIntervalSeconds"] = 600,
            ["FibreStockTarget"] = 10,
            ["LoremasterOfferingBlessingCap"] = 10,
            ["LoremasterOfferingMaterialsMin"] = 30,
            ["LoremasterOfferingMaterials"] = 5,
            ["LoremasterOfferingWork"] = 30,
            ["OfficerReserveMin"] = 2,
            ["OfficerReservePerGroup"] = 2.0,
            ["OfficerItemsPerCat"] = 0.5,
            ["OfficerFoodPerCat"] = 8,
            ["OfficerStockTarget"] = 32,
            ["MouseFarmGrain"] = 1,
            ["MouseFarmWorkSeconds"] = 600,
            ["ResearchSkillSecondsPerPoint"] = 3600,
            ["FarmHarvestCarry"] = 8,
            ["FarmHaulSkillGain"] = 0.25,
            ["FarmSkillSecondsPerPoint"] = 3600,
            ["AccountingWaitSeconds"] = 60,
            ["AccountingCountSeconds"] = 5,
            ["SpoilageRate"] = 0.0002,
            ["PreservesSpoilageRate"] = 0.00002,
            ["EcologyTickSeconds"] = 60,
            ["FishRegenPerHour"] = 0.5,
            // Pacing.Population.cs (26)
            ["KittenUntilHours"] = 24,
            ["ElderAgeHours"] = 240,
            ["HonouredElderAgeHours"] = 288,
            ["OldAgeDeathChance"] = 0.0002,
            ["BreedingCheckSeconds"] = 3600,
            ["BreedingAfterFoundingSeconds"] = 36 * 3600,
            ["BreedingFoodPerCat"] = 2.5,
            ["BreedingWaterPerCat"] = 2.5,
            ["ParentMinAgeHours"] = 24,
            ["ParentMaxAgeHours"] = 240,
            ["BreedChanceBase"] = 0.001,
            ["BreedBlessingStep"] = 0.02,
            ["BreedBlessingCap"] = 0.5,
            ["BreedBlessingWeight"] = 0.0007,
            ["GestationSeconds"] = 18 * 3600,
            ["KittenStatSpread"] = 16,
            ["MigrationFirstAtSeconds"] = 30 * 3600,
            ["MigrationIntervalSeconds"] = 12 * 3600,
            ["MigrationFoodPerCat"] = 4,
            ["MigrationWaterPerCat"] = 5,
            ["MigrationPopulationCap"] = 200,
            ["ProbationSeconds"] = 36 * 3600,
            ["HousingPlanAfterSeconds"] = 6 * 3600,
            ["HousingSpareBeds"] = 1,
            ["HousingFoodPerCat"] = 3,
            ["HousingWaterPerCat"] = 4,
            // Pacing.Research.cs (21)
            ["ResearchDivisor"] = 600,
            ["LeaderStudyIntervalSeconds"] = 86400,
            ["ElectionIntervalSeconds"] = 86400,
            ["ElectionDurationSeconds"] = 1800,
            ["FamilySourcesInputFactor"] = 0.9,
            ["FamilySourcesOutputFactor"] = 1.1,
            ["FamilyBulkDurationFactor"] = 0.9,
            ["UpgradeSupplySpeedStep"] = 0.1,
            ["UpgradeSupplySpeedFloor"] = 0.55,
            ["UpgradeHuntStep"] = 0.1,
            ["UpgradeHuntFloor"] = 0.45,
            ["UpgradeBuildStep"] = 0.1,
            ["UpgradeBuildFloor"] = 0.45,
            ["UpgradeRitualStep"] = 0.12,
            ["UpgradeRitualFloor"] = 0.4,
            ["BoostSeconds"] = 10,
            ["BoostSecondsPerClickPower"] = 2,
            ["BoostFreePerMinute"] = 30,
            ["BoostDecay"] = 0.95,
            ["BoostDecayClicks"] = 10.0,
            ["BoostMinSeconds"] = 1,
            // Pacing.Threats.cs (44)
            ["RaidCheckSeconds"] = 3600,
            ["RaidGraceSeconds"] = 8 * 3600,
            ["RaidMinCats"] = 20,
            ["RaidMinWealth"] = 250,
            ["RaidChance"] = 0.1,
            ["DemoRaidAtSeconds"] = 0,
            ["RaidWarningSeconds"] = 120,
            ["RaidCanKill"] = true,
            ["RaidSpeed"] = 0.5,
            ["DefenderStrikePerSecond"] = 0.1,
            ["RaiderStrikePerSecond"] = 0.03,
            ["DefenderWeaponWearPerSecond"] = 0.005,
            ["DefenderSkillSecondsPerPoint"] = 3600,
            ["RaidLootShare"] = 0.3,
            ["RaidLootMax"] = 20,
            ["RaidBreachDamage"] = 20,
            ["DungeonBaseThreat"] = 1.15,
            ["DungeonBandThreat"] = 0.65,
            ["DungeonFloorThreat"] = 0.85,
            ["GuardianBaseHealth"] = 15,
            ["GuardianFloorHealth"] = 9,
            ["GuardianBandHealth"] = 8,
            ["DungeonLootBase"] = 3,
            ["DungeonLootSecondary"] = 2,
            ["FloorRestockSeconds"] = 600,
            ["ReadinessFightSkillFactor"] = 0.25,
            ["ReadinessNeedNorm"] = 45,
            ["ExpeditionMinHealth"] = 65,
            ["ExpeditionMinNeed"] = 45,
            ["ExpeditionRetreatHealth"] = 50,
            ["ExpeditionRetreatNeed"] = 30,
            ["ExpeditionGiveUpHealth"] = 40,
            ["ExpeditionGiveUpNeed"] = 25,
            ["ExpeditionDescendCargoLimit"] = 8,
            ["DungeonRevealTiles"] = 4,
            ["DungeonReplanSeconds"] = 1,
            ["ExplorerStrikePerSecond"] = 1.8,
            ["ExplorerSkillSecondsPerPoint"] = 300,
            ["ExplorerWeaponWearPerSecond"] = 0.025,
            ["CreatureStrikePerSecond"] = 0.45,
            ["CreatureSpeed"] = 0.65,
            ["CreatureLeashTiles"] = 12,
            ["CreatureAggroTiles"] = 5,
            ["CreatureReplanSeconds"] = 1,
            // Pacing.Trade.cs (15)
            ["TraderFirstAtSeconds"] = 3600,
            ["TraderIntervalSeconds"] = 12 * 3600,
            ["TraderWindowSeconds"] = 900,
            ["TraderReplanSeconds"] = 10,
            ["TraderSpeed"] = 0.8,
            ["TraderPurse"] = 1000,
            ["TraderStockBase"] = 20,
            ["TraderStockSpread"] = 40,
            ["TraderBuyPrice"] = 2,
            ["TraderSellPricePerQuality"] = 4,
            ["CaravanSpeed"] = 0.8,
            ["VehicleSpeed"] = 1,
            ["TransportSkillSecondsPerPoint"] = 3600,
            ["LinearWorkPerTile"] = 30,
            ["InfrastructureWork"] = 120,
            // Pacing.World.cs (8)
            ["CatBaseSpeed"] = 1.6,
            ["RoadSpeedFactor"] = 1.75,
            ["DirtSpeedFactor"] = 1.05,
            ["MountainSpeedFactor"] = 0.4,
            ["ForestSpeedFactor"] = 0.85,
            ["OpenGroundSpeedFactor"] = 1,
            ["WadingSpeedFactor"] = .55,
            ["PathWearToDirt"] = 50,
            // Pacing.cs (2)
            ["Profile"] = Pacing.ClassicProfile,
            ["DayScale"] = 1,
            // Added by W2-01; classic leaves them inert.
            ["NeedsHealthFloor"] = 0,
            ["MigrationRequiresFreeBed"] = false,
        };

        // Every field the demo profile sets, with its demo value. Every other field must equal classic.
        static readonly Dictionary<string, object> DemoSets = new Dictionary<string, object>
        {
            ["Profile"] = Pacing.DemoProfile,
            ["DayScale"] = 60,
            ["HungerDecayPerHour"] = 160,
            ["ThirstDecayPerHour"] = 240,
            ["RestDecayPerHour"] = 96,
            ["NeedServing"] = 0.125,
            ["BrewServing"] = 0.03125,
            ["WaterServingAfterBrew"] = 0.09375,
            ["DrinkRestorePerServing"] = 400,
            ["EatRestorePerServing"] = 400,
            ["HealRestorePerServing"] = 200,
            ["NeedsCanKill"] = false,
            ["NeedsHealthFloor"] = 10,
            ["FishRegenPerHour"] = 12,
            ["BreedingCheckSeconds"] = 60,
            ["BreedingAfterFoundingSeconds"] = 300,
            ["BreedChanceBase"] = 0.2,
            ["GestationSeconds"] = 480,
            ["MigrationFirstAtSeconds"] = 480,
            ["MigrationIntervalSeconds"] = 360,
            ["MigrationRequiresFreeBed"] = true,
            ["ProbationSeconds"] = 7200,
            ["HousingPlanAfterSeconds"] = 600,
            ["ResearchDivisor"] = 60,
            ["LeaderStudyIntervalSeconds"] = 600,
            ["ElectionIntervalSeconds"] = 100 * 365 * 86400.0,
            ["DemoRaidAtSeconds"] = 900,
            ["RaidWarningSeconds"] = 120,
            ["RaidCanKill"] = false,
            ["FloorRestockSeconds"] = 600,
        };

        static FieldInfo[] Knobs() => typeof(Pacing).GetFields(BindingFlags.Public | BindingFlags.Instance).OrderBy(f => f.Name, StringComparer.Ordinal).ToArray();

        static bool IsNumber(object x) => x is int || x is long || x is double || x is float;

        static bool Same(object a, object b) =>
            IsNumber(a) && IsNumber(b) ? Convert.ToDouble(a, CultureInfo.InvariantCulture) == Convert.ToDouble(b, CultureInfo.InvariantCulture) : Equals(a, b);

        static string Show(object value) => Convert.ToString(value, CultureInfo.InvariantCulture);

        static void ClassicEqualsWave1()
        {
            var knobs = Knobs();
            var names = new HashSet<string>(knobs.Select(f => f.Name));
            var missing = Wave1Classic.Keys.Where(k => !names.Contains(k)).ToArray();
            var unlisted = names.Where(n => !Wave1Classic.ContainsKey(n)).OrderBy(n => n, StringComparer.Ordinal).ToArray();
            Check(missing.Length == 0, "Classic table names knobs Pacing no longer has: " + string.Join(", ", missing));
            Check(unlisted.Length == 0, "Pacing knobs without a pinned classic value: " + string.Join(", ", unlisted));
            var changed = knobs.Where(f => !Same(f.GetValue(Pacing.Classic), Wave1Classic[f.Name]))
                .Select(f => f.Name + " " + Show(Wave1Classic[f.Name]) + " -> " + Show(f.GetValue(Pacing.Classic))).ToArray();
            Check(changed.Length == 0, "Classic pacing changed: " + string.Join("; ", changed));
            Check(Pacing.For(Pacing.ClassicProfile) == Pacing.Classic && new World().CurrentPacing() == Pacing.Classic, "New and classic worlds do not pace as classic");
        }

        static void DemoDiffersOnlyInSetFields()
        {
            Check(Pacing.For(Pacing.DemoProfile) == Pacing.Demo && Pacing.Demo != Pacing.Classic, "Demo profile lookup is not its own table");
            var knobs = Knobs();
            var unknown = DemoSets.Keys.Where(k => knobs.All(f => f.Name != k)).ToArray();
            Check(unknown.Length == 0, "Demo table names knobs Pacing does not have: " + string.Join(", ", unknown));
            var wrong = new List<string>();
            foreach (var field in knobs)
            {
                object demo = field.GetValue(Pacing.Demo), classic = field.GetValue(Pacing.Classic);
                object expected = DemoSets.TryGetValue(field.Name, out var set) ? set : classic;
                if (!Same(demo, expected))
                    wrong.Add(field.Name + " expected " + Show(expected) + " got " + Show(demo));
            }
            Check(wrong.Count == 0, "Demo pacing differs from its table: " + string.Join("; ", wrong));
            var w = World.Create(41);
            w.PacingProfile = Pacing.DemoProfile;
            Check(w.CurrentPacing() == Pacing.Demo, "A demo-profile world does not pace as demo");
            w.Step(60);
            Near(DayClock.GameMinutes(w), DayClock.StartMinutes + 60, "Demo clock does not run one game minute per real second");
            Check(DayClock.Day(w) == 1, "Demo clock left Day 1 after one minute");
        }

        static void GoodsPerHourMatchClassic()
        {
            // Needs.cs restores RestorePerServing points per unit consumed and takes one serving per trip.
            Pacing demo = Pacing.Demo, classic = Pacing.Classic;
            Near(demo.HungerDecayPerHour / demo.EatRestorePerServing, classic.HungerDecayPerHour / classic.EatRestorePerServing, "Demo food per cat-hour differs from classic");
            Near(demo.ThirstDecayPerHour / demo.DrinkRestorePerServing, classic.ThirstDecayPerHour / classic.DrinkRestorePerServing, "Demo water per cat-hour differs from classic");
            Near(demo.EatRestorePerServing * demo.NeedServing, classic.EatRestorePerServing * classic.NeedServing, "One meal no longer restores the classic points");
            Near(demo.DrinkRestorePerServing * demo.NeedServing, classic.DrinkRestorePerServing * classic.NeedServing, "One drink no longer restores the classic points");
            Near(demo.DrinkRestorePerServing * demo.BrewServing, classic.DrinkRestorePerServing * classic.BrewServing, "One brew no longer restores the classic points");
            Near(demo.DrinkRestorePerServing * demo.WaterServingAfterBrew, classic.DrinkRestorePerServing * classic.WaterServingAfterBrew, "Water after a brew no longer restores the classic points");
            Near(demo.HealRestorePerServing * demo.NeedServing, classic.HealRestorePerServing * classic.NeedServing, "One herb trip no longer heals the classic amount");
            Check(demo.HungerDecayPerHour >= 6 * classic.HungerDecayPerHour && demo.ThirstDecayPerHour >= 6 * classic.ThirstDecayPerHour && demo.RestDecayPerHour >= 6 * classic.RestDecayPerHour, "Demo needs are not several times faster than classic");
        }

        // ---- The D6 measurement harness ------------------------------------------------------------------------

        const int MeasureSeconds = 40 * 60;

        /// <summary>Seconds into the run when each beat first happened; -1 when it never did.</summary>
        sealed class DemoCurve
        {
            public double FirstResearch = -1, FirstResearchPoint = -1, FirstStudyAffordable = -1, FirstMigrant = -1, FirstPregnancy = -1, FirstKitten = -1, RaidWarning = -1, Raid = -1;
            public int EatTrips, DrinkTrips, SleepTrips, HealTrips, EatOrDrinkFirstTenMinutes, Deaths, NeedDeaths, Studies, Cats;
            public string Line = "";
        }

        static readonly Lazy<DemoCurve> Measured = new Lazy<DemoCurve>(Measure, LazyThreadSafetyMode.ExecutionAndPublication);
        static DemoCurve Curve => Measured.Value;

        static void Within(double at, double from, double to, string beat) =>
            Check(at >= from && at <= to, beat + " at " + Seconds(at) + ", target " + Seconds(from) + " to " + Seconds(to) + " (" + Curve.Line + ")");

        static string Seconds(double at) => at < 0 ? "never" : Math.Round(at).ToString(CultureInfo.InvariantCulture) + "s";

        static void PrintCurve() => Console.WriteLine(Curve.Line);

        /// <summary>
        /// Seed 41 in the demo profile, stepped at 1 s for 40 sim minutes. The scripted player only keeps one scholar in
        /// the research hut; everything else is the colony on its own.
        /// </summary>
        static DemoCurve Measure()
        {
            var w = World.Create(41);
            w.PacingProfile = Pacing.DemoProfile;
            var curve = new DemoCurve();
            var v = w.Villages[0];
            var founders = new HashSet<string>(v.Cats.Select(c => c.Id));
            var foundingStudies = new HashSet<string>(v.Research);
            var goals = v.Cats.ToDictionary(c => c.Id, c => c.Goal);
            long seenSeq = v.Events.Select(e => e.Seq).DefaultIfEmpty(0).Max();
            for (int second = 1; second <= MeasureSeconds; second++)
            {
                StaffResearchHut(w, w.Villages[0]);
                w.Step(1);
                v = w.Villages[0];
                double t = w.TimeSeconds;
                double spent = v.Research.Where(id => !foundingStudies.Contains(id)).Sum(id => Catalog.Study(id)?.Cost ?? 0);
                if (curve.FirstResearch < 0 && v.ResearchPoints + spent > 0)
                    curve.FirstResearch = t;
                if (curve.FirstResearchPoint < 0 && v.ResearchPoints + spent >= 1)
                    curve.FirstResearchPoint = t;
                if (curve.FirstStudyAffordable < 0 && (spent > 0 || Catalog.Research.Any(n => !v.Research.Contains(n.Id) && n.Prerequisites.All(v.Research.Contains) && n.Cost <= v.ResearchPoints)))
                    curve.FirstStudyAffordable = t;
                foreach (var c in v.Cats.Where(c => !founders.Contains(c.Id)))
                {
                    if (c.ParentIds.Count > 0 && curve.FirstKitten < 0)
                        curve.FirstKitten = t;
                    if (c.ParentIds.Count == 0 && curve.FirstMigrant < 0)
                        curve.FirstMigrant = t;
                }
                if (curve.FirstPregnancy < 0 && v.Cats.Any(c => c.Alive && c.PregnantUntil > 0))
                    curve.FirstPregnancy = t;
                if (curve.Raid < 0 && v.Raids.Count > 0)
                    curve.Raid = t;
                foreach (var e in v.Events.Where(e => e.Seq > seenSeq))
                {
                    if (e.Kind == EventKind.RaidWarning && curve.RaidWarning < 0)
                        curve.RaidWarning = e.Time;
                    if (e.Kind == EventKind.Raid && curve.Raid < 0)
                        curve.Raid = e.Time;
                    if (e.Kind == EventKind.Death)
                    {
                        curve.Deaths++;
                        if (e.Text.IndexOf("needs", StringComparison.Ordinal) >= 0)
                            curve.NeedDeaths++;
                    }
                }
                seenSeq = Math.Max(seenSeq, v.Events.Select(e => e.Seq).DefaultIfEmpty(0).Max());
                foreach (var c in v.Cats)
                {
                    goals.TryGetValue(c.Id, out var before);
                    if (c.Goal != before && c.Goal.StartsWith(GoalKey.NeedPrefix, StringComparison.Ordinal))
                    {
                        if (c.Goal == GoalKey.NeedEat)
                            curve.EatTrips++;
                        else if (c.Goal == GoalKey.NeedDrink)
                            curve.DrinkTrips++;
                        else if (c.Goal == GoalKey.NeedSleep)
                            curve.SleepTrips++;
                        else if (c.Goal == GoalKey.NeedHeal)
                            curve.HealTrips++;
                        if ((c.Goal == GoalKey.NeedEat || c.Goal == GoalKey.NeedDrink) && t <= 600)
                            curve.EatOrDrinkFirstTenMinutes++;
                    }
                    goals[c.Id] = c.Goal;
                }
            }
            curve.Studies = v.Research.Count - foundingStudies.Count;
            curve.Cats = v.Cats.Count(c => c.Alive);
            curve.Line = "demo.curve seed=41 profile=demo minutes=40"
                + " research>0=" + Seconds(curve.FirstResearch)
                + " research>=1=" + Seconds(curve.FirstResearchPoint)
                + " study_affordable=" + Seconds(curve.FirstStudyAffordable)
                + " studies=" + curve.Studies
                + " migrant=" + Seconds(curve.FirstMigrant)
                + " pregnancy=" + Seconds(curve.FirstPregnancy)
                + " kitten=" + Seconds(curve.FirstKitten)
                + " raid_warning=" + Seconds(curve.RaidWarning)
                + " raid=" + Seconds(curve.Raid)
                + " trips eat=" + curve.EatTrips + " drink=" + curve.DrinkTrips + " sleep=" + curve.SleepTrips + " heal=" + curve.HealTrips
                + " eat_or_drink_10min=" + curve.EatOrDrinkFirstTenMinutes
                + " deaths=" + curve.Deaths + " need_deaths=" + curve.NeedDeaths
                + " cats=" + curve.Cats;
            return curve;
        }

        /// <summary>The minimal player: when a completed research hut has no living scholar, assign one idle adult.</summary>
        static void StaffResearchHut(World w, Village v)
        {
            foreach (var hut in v.Buildings.Where(b => b.Completed && b.Kind == "research_hut").ToArray())
            {
                if (hut.Slots.Any(s => s.CatId != "" && v.Cats.Any(c => c.Id == s.CatId && c.Alive)))
                    continue;
                var scholar = v.Cats.Where(c => c.Alive && c.Migration == "resident" && c.AgeHours >= 12 && c.ControlledBy == "" && c.JobId == "" && c.BuildingId == "" && c.OfficerRole == "" && c.Id != v.LeaderId && c.Position.Level == 0)
                    .OrderBy(c => c.Id, StringComparer.Ordinal).FirstOrDefault();
                if (scholar != null)
                    w.Apply(Context(v), new GameAction { Kind = "AssignWorker", CatId = scholar.Id, BuildingId = hut.Id });
            }
        }
    }
}
