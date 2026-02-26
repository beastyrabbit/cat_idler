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
// Colony emergency alert system
export * from "./emergencyAlerts";
// Cat gossip network (rumor propagation)
export * from "./gossipNetwork";
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
// Cat work ethic and productivity
export * from "./workEthic";
// Resource spoilage system
export * from "./spoilage";
