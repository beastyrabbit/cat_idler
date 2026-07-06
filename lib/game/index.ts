/**
 * Game Logic Module
 *
 * This module exports all pure game logic functions.
 * These functions have NO side effects and NO database access.
 * They take inputs and return outputs - perfect for unit testing.
 */

// Colony achievement tracking
export * from "./achievements";
export * from "./adviceColumn";
// Age and life stage calculations
export * from "./age";
// Cat ancestry and lineage tracking
export * from "./ancestry";
export * from "./biomes";
// Birth announcements
export * from "./birthAnnouncements";
// Cat AI autonomous behavior
export * from "./catAI";
// Colony census & demographics
export * from "./census";
// Colony chronicle historical retrospective system
export * from "./chronicle";
// Newspaper classified ads
export * from "./classifiedAds";
// Combat resolution
export * from "./combat";
export * from "./crimeBlotter";
// Cat dream journal
export * from "./dreamJournal";
export type {
	ColonyMetrics,
	EmergencyAlert,
	EmergencyType,
} from "./emergencyAlerts";
// Colony emergency alerts (selective exports to avoid `SeverityLevel` naming conflicts)
export {
	calculateSeverity,
	classifySeverityLevel,
	detectEmergencies,
	generateEmergencyBulletin,
	prioritizeAlerts,
} from "./emergencyAlerts";
// Exploration field reports
export * from "./explorationReports";
// Cat folklore and legends
export * from "./folklore";
export * from "./foodCritic";
// Cat gossip network (rumor propagation)
export * from "./gossipNetwork";
// Cat health diagnosis system
export * from "./healthDiagnosis";
// Colony horoscope (zodiac signs, daily fortunes, compatibility)
// Selective exports to avoid `CompatibilityLevel` conflict with personality module
export {
	type Compatibility as HoroscopeCompatibility,
	type CompatibilityLevel as HoroscopeCompatibilityLevel,
	type DailyFortune,
	type FortuneSeverity,
	formatHoroscopeColumn,
	formatHoroscopeEntry,
	getCatZodiacSign,
	getDailyFortune,
	getSignCompatibility,
	type HoroscopeColumn,
	ZODIAC_SIGNS,
	type ZodiacSign,
} from "./horoscope";
// House build prerequisite planner
export * from "./housePlanner";
// Cat legacy score and Hall of Fame
export * from "./legacyScore";
export * from "./lifeMilestones";
// Lunar calendar and moon phase system
export * from "./lunarCalendar";
// Cat mentorship and apprenticeship system
// Selective exports to avoid `SkillDomain` conflict with skillRankings module
export {
	type CatForMentorship,
	calculateMentorXPBonus,
	calculateTrainingEffectiveness,
	canMentor,
	findBestMentor,
	generateAcademyReport,
	type MentorMatch,
	type SkillDomain as MentorshipSkillDomain,
} from "./mentorship";
export * from "./migration";
// Cat mood/happiness system
export * from "./mood";
// Cat naming system (warrior cats conventions)
export * from "./naming";
// Needs system (hunger, thirst, rest, health)
export * from "./needs";
export * from "./nightWatch";
export * from "./noise";
// Cat obituary generator
export * from "./obituaries";
// Path system
export * from "./paths";
// Cat personality profiles
export * from "./personality";
// Leader policy tier system
export * from "./policy";
// Cat popularity contest
export * from "./popularity";
// Colony proverbs & wisdom
export * from "./proverbs";
// Cat relationship / affinity tracking
export * from "./relationships";
// Colony reputation system
export * from "./reputation";
// Resource trend analysis (moving averages, direction, percent change)
export * from "./resourceTrends";
// Seasonal cycle system
export * from "./seasons";
// Deterministic seeded RNG
export * from "./seededRng";
// Cat skill rankings
export * from "./skillRankings";
// Skill learning system
export * from "./skills";
// Resource spoilage system
export * from "./spoilage";
export * from "./sportsPage";
// Colony supply & demand market report
export * from "./supplyDemand";
// Cat survival (needs decay, damage, death)
export * from "./survival";
// Task assignment
export * from "./tasks";
// Colony territory influence calculation
export * from "./territory";
// Predator threat level assessment
export * from "./threats";
// Colony trade route system
export * from "./tradeRoutes";
// Weather system (deterministic, seed-based)
export * from "./weather";
// Cat work ethic and productivity
export * from "./workEthic";
// World generation
export * from "./worldGen";
// World resource management
export * from "./worldResources";
