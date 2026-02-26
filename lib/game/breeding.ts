/**
 * Fertility Blessing System
 *
 * Pure functions for calculating breeding chance bonuses from colony blessings.
 */

/**
 * Calculate fertility bonus from colony blessings.
 *
 * Each blessing adds +2% breeding chance, capped at 50% bonus.
 *
 * @param blessings - Number of colony blessings (from resources.blessings)
 * @returns Bonus breeding chance (0.0 to 0.5)
 */
export function calculateFertilityBonus(blessings: number): number {
  if (blessings <= 0) return 0;
  return Math.min(0.5, blessings * 0.02);
}

/**
 * Calculate final breeding chance with blessing bonus.
 *
 * Combines base chance with fertility bonus, capped at 80%.
 * Negative base chances are clamped to 0 before adding bonus.
 *
 * @param baseChance - Base breeding chance (e.g. 0.3 for 30%)
 * @param blessings - Number of colony blessings
 * @returns Final breeding chance (0.0 to 0.8)
 */
export function calculateBreedingChance(
  baseChance: number,
  blessings: number,
): number {
  const clampedBase = Math.max(0, baseChance);
  const bonus = calculateFertilityBonus(blessings);
  return Math.min(0.8, clampedBase + bonus);
}
