/**
 * Path System Calculations
 *
 * Pure functions for path wear and travel effects.
 * See TASKS.md: LOGIC-009
 */

/**
 * Add wear to a path from cat travel.
 *
 * @param currentWear - Current path wear (0-100)
 * @param amount - Amount of wear to add
 * @returns New wear value (capped at 100)
 */
export function addPathWear(currentWear: number, amount: number): number {
  return Math.min(100, currentWear + amount)
}

/**
 * Decay path wear over time (unused paths decay).
 *
 * @param currentWear - Current path wear (0-100)
 * @returns New wear value (minimum 0)
 */
export function decayPathWear(currentWear: number): number {
  return Math.max(0, currentWear - 1)
}

/**
 * Get speed bonus from path wear.
 *
 * Rules:
 * - Wear 0-29: No bonus
 * - Wear 30-59: +10% speed
 * - Wear 60-89: +25% speed
 * - Wear 90-100: +40% speed
 *
 * @param pathWear - Path wear level (0-100)
 * @returns Speed multiplier (0.0 to 0.4)
 */
export function getPathSpeedBonus(pathWear: number): number {
  if (pathWear < 30) {
    return 0
  } else if (pathWear < 60) {
    return 0.1
  } else if (pathWear < 90) {
    return 0.25
  } else {
    return 0.4
  }
}

/**
 * Get danger reduction from path wear.
 *
 * Rules:
 * - Wear 0-29: No reduction
 * - Wear 30-59: -25% danger
 * - Wear 60-89: -60% danger
 * - Wear 90-100: -100% danger (no encounters)
 *
 * @param pathWear - Path wear level (0-100)
 * @returns Danger reduction multiplier (0.0 to 1.0)
 */
export function getPathDangerReduction(pathWear: number): number {
  if (pathWear < 30) {
    return 0
  } else if (pathWear < 60) {
    return 0.25
  } else if (pathWear < 90) {
    return 0.6
  } else {
    return 1.0
  }
}



