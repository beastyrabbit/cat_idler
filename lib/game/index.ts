/**
 * Game Logic Module
 *
 * This module exports all pure game logic functions.
 * These functions have NO side effects and NO database access.
 * They take inputs and return outputs - perfect for unit testing.
 */

// Needs system (hunger, thirst, rest, health)
export * from "./needs";

// Age and life stage calculations
export * from "./age";

// Skill learning system
export * from "./skills";

// Combat resolution
export * from "./combat";

// Cat AI autonomous behavior
export * from "./catAI";

// Task assignment
export * from "./tasks";

// Path system
export * from "./paths";

// World resource management
export * from "./worldResources";

// World generation
export * from "./worldGen";
export * from "./biomes";
export * from "./noise";

// Deterministic seeded RNG
export * from "./seededRng";

// Leader policy tier system
export * from "./policy";

// Cat survival (needs decay, damage, death)
export * from "./survival";

// House build prerequisite planner
export * from "./housePlanner";
// Cat dream journal
export * from "./dreamJournal";
// Colony territory influence calculation
export * from "./territory";

// Predator threat level assessment
export * from "./threats";

// Colony trade route system
export * from "./tradeRoutes";

// Cat legacy score and Hall of Fame
export * from "./legacyScore";
// Lunar calendar and moon phase system
export * from "./lunarCalendar";
// Cat popularity contest
export * from "./popularity";
// Cat ancestry and lineage tracking
export * from "./ancestry";
// Cat folklore and legends
export * from "./folklore";

// Cat health diagnosis system
export * from "./healthDiagnosis";

// Cat gossip network (rumor propagation)
export * from "./gossipNetwork";
export * from "./migration";
export * from "./nightWatch";
export * from "./lifeMilestones";
export * from "./sportsPage";

export * from "./foodCritic";

export * from "./adviceColumn";

// Cat work ethic and productivity
export * from "./workEthic";

// Colony emergency alerts (selective exports to avoid `SeverityLevel` naming conflicts)
export {
  calculateSeverity,
  classifySeverityLevel,
  detectEmergencies,
  prioritizeAlerts,
  generateEmergencyBulletin,
} from "./emergencyAlerts";
export type {
  ColonyMetrics,
  EmergencyType,
  EmergencyAlert,
} from "./emergencyAlerts";

export * from "./crimeBlotter";

// Colony chronicle historical retrospective system
export * from "./chronicle";
// Resource trend analysis (moving averages, direction, percent change)
export * from "./resourceTrends";
// Weather system (deterministic, seed-based)
export * from "./weather";
// Resource spoilage system
export * from "./spoilage";
// Cat mood/happiness system
export * from "./mood";
// Seasonal cycle system
export * from "./seasons";
// Cat naming system (warrior cats conventions)
export * from "./naming";
// Cat relationship / affinity tracking
export * from "./relationships";
// Colony achievement tracking
export * from "./achievements";
// Colony reputation system
export * from "./reputation";
