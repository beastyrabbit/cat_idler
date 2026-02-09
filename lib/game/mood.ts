/**
 * Cat Mood System
 *
 * Pure functions for calculating cat mood from needs, life stage,
 * colony population, and colony status. Mood affects productivity.
 */

import type { CatNeeds, LifeStage, ColonyStatus } from "@/types/game";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type MoodLevel =
  | "miserable"
  | "anxious"
  | "content"
  | "happy"
  | "ecstatic";

export interface ColonyMorale {
  dominantMood: MoodLevel;
  averageModifier: number;
  catCount: number;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MOOD_MODIFIERS: Record<MoodLevel, number> = {
  miserable: 0.6,
  anxious: 0.8,
  content: 1.0,
  happy: 1.15,
  ecstatic: 1.3,
};

const LIFE_STAGE_BONUS: Record<LifeStage, number> = {
  kitten: 7,
  young: 8,
  adult: 10,
  elder: 5,
};

// ---------------------------------------------------------------------------
// getMood
// ---------------------------------------------------------------------------

/**
 * Calculate a cat's mood from their physical/social state.
 *
 * Scoring (0-70):
 *  - Needs score (0-40): average of hunger/thirst/rest/health scaled to 40
 *  - Life stage bonus (0-10): adult +10, young +8, kitten +7, elder +5
 *  - Social bonus (0-10): based on colony population
 *  - Colony status bonus (0-10): thriving +10, starting +5, struggling +2, dead +0
 *
 * Score → mood thresholds:
 *  - 0-14: miserable
 *  - 15-29: anxious
 *  - 30-44: content
 *  - 45-57: happy
 *  - 58-70: ecstatic
 */
export function getMood(
  needs: CatNeeds,
  lifeStage: LifeStage,
  colonyPopulation: number,
  colonyStatus: ColonyStatus,
): MoodLevel {
  const needsAvg =
    (needs.hunger + needs.thirst + needs.rest + needs.health) / 4;
  const needsScore = (needsAvg / 100) * 40;

  const lifeStageBonus = LIFE_STAGE_BONUS[lifeStage];

  let socialBonus: number;
  if (colonyPopulation >= 11) socialBonus = 10;
  else if (colonyPopulation >= 6) socialBonus = 8;
  else if (colonyPopulation >= 2) socialBonus = 5;
  else if (colonyPopulation === 1) socialBonus = 2;
  else socialBonus = 0;

  let statusBonus: number;
  if (colonyStatus === "thriving") statusBonus = 10;
  else if (colonyStatus === "starting") statusBonus = 5;
  else if (colonyStatus === "struggling") statusBonus = 2;
  else statusBonus = 0; // dead

  const total = needsScore + lifeStageBonus + socialBonus + statusBonus;

  if (total >= 58) return "ecstatic";
  if (total >= 45) return "happy";
  if (total >= 30) return "content";
  if (total >= 15) return "anxious";
  return "miserable";
}

// ---------------------------------------------------------------------------
// getMoodModifier
// ---------------------------------------------------------------------------

/**
 * Get the productivity multiplier for a given mood level.
 */
export function getMoodModifier(mood: MoodLevel): number {
  return MOOD_MODIFIERS[mood];
}

// ---------------------------------------------------------------------------
// getColonyMorale
// ---------------------------------------------------------------------------

/**
 * Compute aggregate colony morale from individual cat moods.
 */
export function getColonyMorale(moods: MoodLevel[]): ColonyMorale {
  if (moods.length === 0) {
    return { dominantMood: "content", averageModifier: 0, catCount: 0 };
  }

  const totalModifier = moods.reduce((sum, m) => sum + MOOD_MODIFIERS[m], 0);

  // Count occurrences to find dominant mood
  const counts: Record<string, number> = {};
  for (const m of moods) {
    counts[m] = (counts[m] ?? 0) + 1;
  }
  let dominantMood: MoodLevel = moods[0];
  let maxCount = 0;
  for (const [mood, count] of Object.entries(counts)) {
    if (count > maxCount) {
      maxCount = count;
      dominantMood = mood as MoodLevel;
    }
  }

  return {
    dominantMood,
    averageModifier: totalModifier / moods.length,
    catCount: moods.length,
  };
}
