/**
 * Colony Reputation System
 *
 * Pure functions for calculating colony fame/reputation from
 * encounters, buildings, age, and population. Provides gameplay
 * modifiers: recruitment bonus and trade price modifier.
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type ReputationLevel =
  | "unknown"
  | "local"
  | "regional"
  | "renowned"
  | "legendary";

export interface ColonyFameFactors {
  encountersWon: number;
  encountersLost: number;
  buildingsCompleted: number;
  colonyAgeHours: number;
  catsAlive: number;
  isCritical: boolean;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const REPUTATION_THRESHOLDS: readonly {
  min: number;
  level: ReputationLevel;
}[] = [
  { min: 80, level: "legendary" },
  { min: 55, level: "renowned" },
  { min: 35, level: "regional" },
  { min: 15, level: "local" },
] as const;

const RECRUITMENT_BONUSES: Record<ReputationLevel, number> = {
  unknown: 0,
  local: 10,
  regional: 20,
  renowned: 35,
  legendary: 50,
};

const TRADE_MODIFIERS: Record<ReputationLevel, number> = {
  unknown: 1.0,
  local: 0.95,
  regional: 0.9,
  renowned: 0.8,
  legendary: 0.7,
};

const DESCRIPTIONS: Record<ReputationLevel, string> = {
  unknown:
    "The colony remains unheard of beyond its immediate borders, a quiet settlement known only to its own residents.",
  local:
    "Word of the colony has spread among nearby territories — local cats speak of a small but determined settlement.",
  regional:
    "The colony's name carries across the region, recognized for its growing strength and resilience by neighbouring communities.",
  renowned:
    "Fame of the colony has reached distant lands — travellers bring tales of its prosperity, and ambitious cats seek it out.",
  legendary:
    "The colony has achieved mythical status — its name echoes across every territory, drawing cats from the farthest reaches of the known world.",
};

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

/**
 * Map a reputation score (0-100) to a reputation level.
 * Scores outside 0-100 are clamped.
 */
export function getReputationLevel(score: number): ReputationLevel {
  const clamped = Math.max(0, Math.min(100, score));
  for (const { min, level } of REPUTATION_THRESHOLDS) {
    if (clamped >= min) return level;
  }
  return "unknown";
}

/**
 * Calculate net fame from combat record.
 * Wins use logarithmic scaling (0-25), losses subtract linearly (-2 each, max -15).
 * Result is clamped to minimum 0.
 */
export function calculateFameFromEncounters(won: number, lost: number): number {
  const winFame = won > 0 ? Math.min(25, Math.log(won + 1) * 6.5) : 0;
  const lossPenalty = Math.min(15, lost * 2);
  return Math.max(0, Math.round((winFame - lossPenalty) * 100) / 100);
}

/**
 * Compute overall reputation score from colony fame factors.
 * Combines encounter fame, building fame, age fame, and population fame,
 * minus critical-state penalty. Clamped 0-100.
 */
export function calculateReputationScore(factors: ColonyFameFactors): number {
  const encounterFame = calculateFameFromEncounters(
    factors.encountersWon,
    factors.encountersLost,
  );

  // Buildings: count × 4, capped at 20
  const buildingFame = Math.min(20, factors.buildingsCompleted * 4);

  // Colony age: logarithmic, capped at 15
  const ageFame =
    factors.colonyAgeHours > 0
      ? Math.min(15, Math.log(factors.colonyAgeHours + 1) * 3.5)
      : 0;

  // Population: pop / 2, capped at 15
  const populationFame = Math.min(15, factors.catsAlive / 2);

  // Critical penalty
  const criticalPenalty = factors.isCritical ? 10 : 0;

  const raw =
    encounterFame + buildingFame + ageFame + populationFame - criticalPenalty;
  return Math.max(0, Math.min(100, Math.round(raw * 100) / 100));
}

/**
 * Get recruitment bonus percentage for a reputation level.
 * Higher reputation attracts more wandering cats (0-50%).
 */
export function getRecruitmentBonus(level: ReputationLevel): number {
  return RECRUITMENT_BONUSES[level];
}

/**
 * Get trade price multiplier for a reputation level.
 * Higher reputation means better deals (1.0 = normal, < 1.0 = discount).
 */
export function getTradeModifier(level: ReputationLevel): number {
  return TRADE_MODIFIERS[level];
}

/**
 * Get a newspaper-ready description of the colony's standing.
 */
export function describeReputation(level: ReputationLevel): string {
  return DESCRIPTIONS[level];
}
