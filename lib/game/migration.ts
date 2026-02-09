/**
 * Cat Migration System
 *
 * Pure functions for calculating whether wandering cats would
 * choose to immigrate to the colony based on attractiveness scoring,
 * probability thresholds, and seeded RNG evaluation.
 */

import { rollSeeded } from "./seededRng";

// --- Types ---

export interface ColonyAttractivenessInput {
  food: number;
  water: number;
  buildingCount: number;
  currentPop: number;
  maxPop: number;
  dangerLevel: number;
}

export interface MigrationResult {
  migrants: number;
  rejected: number;
  attractiveness: number;
  probability: number;
}

// --- Constants ---

const FOOD_MAX = 25;
const FOOD_DIVISOR = 50;
const WATER_MAX = 20;
const WATER_DIVISOR = 30;
const SHELTER_PER_BUILDING = 5;
const SHELTER_MAX = 25;
const HEADROOM_MAX = 20;
const SAFETY_MAX = 10;

const PROB_HIGH = 0.8; // attractiveness >= 70
const PROB_MEDIUM = 0.4; // attractiveness 50-69
const PROB_LOW = 0.15; // attractiveness 30-49
const PROB_MINIMAL = 0.02; // attractiveness < 30

// --- Functions ---

export function calculateColonyAttractiveness(
  input: ColonyAttractivenessInput,
): number {
  const food = Math.min(input.food / FOOD_DIVISOR, 1) * FOOD_MAX;
  const water = Math.min(input.water / WATER_DIVISOR, 1) * WATER_MAX;
  const shelter = Math.min(
    input.buildingCount * SHELTER_PER_BUILDING,
    SHELTER_MAX,
  );
  const headroom =
    input.maxPop > 0
      ? Math.max(0, (input.maxPop - input.currentPop) / input.maxPop) *
        HEADROOM_MAX
      : 0;
  const safety = ((100 - Math.min(input.dangerLevel, 100)) / 100) * SAFETY_MAX;

  return Math.round(food + water + shelter + headroom + safety);
}

export function getMigrationProbability(attractiveness: number): number {
  if (attractiveness >= 70) return PROB_HIGH;
  if (attractiveness >= 50) return PROB_MEDIUM;
  if (attractiveness >= 30) return PROB_LOW;
  return PROB_MINIMAL;
}

export function evaluateMigrant(
  attractiveness: number,
  rngValue: number,
): boolean {
  const probability = getMigrationProbability(attractiveness);
  return rngValue < probability;
}

export function evaluateMigrationBatch(
  attractiveness: number,
  wandererCount: number,
  rngSeed: number,
): MigrationResult {
  const probability = getMigrationProbability(attractiveness);
  let migrants = 0;
  let seed = rngSeed;

  for (let i = 0; i < wandererCount; i++) {
    const roll = rollSeeded(seed);
    seed = roll.nextSeed;
    if (roll.value < probability) {
      migrants++;
    }
  }

  return {
    migrants,
    rejected: wandererCount - migrants,
    attractiveness,
    probability,
  };
}

export function generateArrivalReport(
  migrantCount: number,
  attractiveness: number,
  colonyName: string,
): string {
  if (migrantCount === 0) {
    return `No wandering cats chose to settle in ${colonyName} this period. The colony's attractiveness score stands at ${attractiveness}.`;
  }

  const catWord = migrantCount === 1 ? "cat" : "cats";
  const verb = migrantCount === 1 ? "has" : "have";

  return `${migrantCount} wandering ${catWord} ${verb} joined the colony of ${colonyName}! With an attractiveness score of ${attractiveness}, the colony continues to draw new residents.`;
}
