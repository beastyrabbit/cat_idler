/**
 * Colony Achievement System
 *
 * Pure functions for evaluating colony milestones across five categories:
 * population, construction, survival, leadership, and combat.
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type AchievementCategory =
  | "population"
  | "construction"
  | "survival"
  | "leadership"
  | "combat";

export interface Achievement {
  id: string;
  name: string;
  description: string;
  category: AchievementCategory;
  threshold: number;
  statKey: string;
}

export interface ColonyStats {
  totalCatsBorn: number;
  catsAlive: number;
  buildingsBuilt: number;
  encountersSurvived: number;
  encountersWon: number;
  colonyAgeHours: number;
  hasLeader: boolean;
  leaderAgeHours: number;
  hasSurvivedCritical: boolean;
}

export interface UnlockedAchievement extends Achievement {
  unlockedAt: "current";
}

// ---------------------------------------------------------------------------
// Achievement definitions
// ---------------------------------------------------------------------------

const ACHIEVEMENTS: readonly Achievement[] = [
  // Population
  {
    id: "first_litter",
    name: "First Litter",
    description: "5 cats have been born in the colony",
    category: "population",
    threshold: 5,
    statKey: "totalCatsBorn",
  },
  {
    id: "growing_colony",
    name: "Growing Colony",
    description: "10 cats are alive at the same time",
    category: "population",
    threshold: 10,
    statKey: "catsAlive",
  },
  {
    id: "thriving_colony",
    name: "Thriving Colony",
    description: "20 cats are alive at the same time",
    category: "population",
    threshold: 20,
    statKey: "catsAlive",
  },

  // Construction
  {
    id: "first_shelter",
    name: "First Shelter",
    description: "The colony built its first structure",
    category: "construction",
    threshold: 1,
    statKey: "buildingsBuilt",
  },
  {
    id: "builder_colony",
    name: "Builder Colony",
    description: "3 buildings stand in the colony",
    category: "construction",
    threshold: 3,
    statKey: "buildingsBuilt",
  },
  {
    id: "fortress",
    name: "Fortress",
    description: "5 buildings form a true fortress",
    category: "construction",
    threshold: 5,
    statKey: "buildingsBuilt",
  },

  // Survival
  {
    id: "first_encounter_survived",
    name: "First Encounter Survived",
    description: "The colony survived its first dangerous encounter",
    category: "survival",
    threshold: 1,
    statKey: "encountersSurvived",
  },
  {
    id: "weathered_storm",
    name: "Weathered a Storm",
    description: "The colony survived a critical state",
    category: "survival",
    threshold: 1,
    statKey: "hasSurvivedCritical",
  },
  {
    id: "century_colony",
    name: "Century Colony",
    description: "The colony has endured for over 100 hours",
    category: "survival",
    threshold: 100,
    statKey: "colonyAgeHours",
  },

  // Leadership
  {
    id: "first_leader",
    name: "First Leader Elected",
    description: "A leader has risen to guide the colony",
    category: "leadership",
    threshold: 1,
    statKey: "hasLeader",
  },
  {
    id: "experienced_leader",
    name: "Experienced Leader",
    description: "The leader has served for 24 hours",
    category: "leadership",
    threshold: 24,
    statKey: "leaderAgeHours",
  },

  // Combat
  {
    id: "first_enemy_defeated",
    name: "First Enemy Defeated",
    description: "The colony won its first fight",
    category: "combat",
    threshold: 1,
    statKey: "encountersWon",
  },
  {
    id: "veteran_defenders",
    name: "Veteran Defenders",
    description: "10 encounters won through strength and courage",
    category: "combat",
    threshold: 10,
    statKey: "encountersWon",
  },
] as const;

// Boolean stat keys that resolve to 0 or 1 instead of a numeric value
const BOOLEAN_STAT_KEYS = new Set(["hasLeader", "hasSurvivedCritical"]);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function getStatValue(stats: ColonyStats, statKey: string): number {
  const raw = stats[statKey as keyof ColonyStats];
  if (typeof raw === "boolean") return raw ? 1 : 0;
  return typeof raw === "number" ? raw : 0;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/** Return all possible achievements with their thresholds. */
export function getAchievementDefinitions(): Achievement[] {
  return [...ACHIEVEMENTS];
}

/** Check if a single achievement is unlocked given colony stats. */
export function checkAchievement(
  achievement: Achievement,
  stats: ColonyStats,
): boolean {
  return getStatValue(stats, achievement.statKey) >= achievement.threshold;
}

/** Evaluate all achievements against colony stats, returning only unlocked ones. */
export function evaluateAchievements(
  stats: ColonyStats,
): UnlockedAchievement[] {
  return ACHIEVEMENTS.filter((a) => checkAchievement(a, stats)).map((a) => ({
    ...a,
    unlockedAt: "current" as const,
  }));
}

/** Progress percentage (0-100) toward unlocking an achievement. */
export function getAchievementProgress(
  achievement: Achievement,
  stats: ColonyStats,
): number {
  if (BOOLEAN_STAT_KEYS.has(achievement.statKey)) {
    return getStatValue(stats, achievement.statKey) >= achievement.threshold
      ? 100
      : 0;
  }
  const value = getStatValue(stats, achievement.statKey);
  const pct = (value / achievement.threshold) * 100;
  return Math.min(100, Math.round(pct));
}

/** Newspaper-ready announcement text for an achievement. */
export function formatAchievementAnnouncement(
  achievement: Achievement,
): string {
  return `MILESTONE ACHIEVED: ${achievement.name} — ${achievement.description}`;
}
