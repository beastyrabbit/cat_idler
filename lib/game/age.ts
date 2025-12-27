/**
 * Cat Age System
 *
 * Pure functions for calculating cat age, life stage, and death chance.
 * See TASKS.md: LOGIC-004
 *
 * TODO: Implement these functions using TDD
 * 1. Write tests first in tests/unit/game/age.test.ts
 * 2. Implement functions to make tests pass
 */

import type { LifeStage } from '@/types/game'
import { LIFE_STAGE_HOURS } from '@/types/game'

/**
 * Calculate cat age in hours.
 *
 * @param birthTime - Timestamp when cat was born
 * @param currentTime - Current timestamp
 * @returns Age in hours (can be fractional)
 */
export function getAgeInHours(birthTime: number, currentTime: number): number {
  const ageMs = currentTime - birthTime
  return ageMs / (1000 * 60 * 60) // Convert milliseconds to hours
}

/**
 * Determine cat's life stage based on age.
 *
 * Life stages:
 * - Kitten: 0-6 hours
 * - Young: 6-24 hours
 * - Adult: 24-48 hours
 * - Elder: 48+ hours
 *
 * @param ageInHours - Cat's age in hours
 * @returns Life stage
 */
export function getLifeStage(ageInHours: number): LifeStage {
  if (ageInHours < 6) {
    return 'kitten'
  } else if (ageInHours < 24) {
    return 'young'
  } else if (ageInHours < 48) {
    return 'adult'
  } else {
    return 'elder'
  }
}

/**
 * Calculate death chance based on age.
 *
 * Rules:
 * - 0% before 48 hours
 * - 1% at exactly 48 hours
 * - +0.5% per hour after 48
 * - Leaders/healers: threshold is 57.6 hours (48 * 1.2)
 *
 * @param ageInHours - Cat's age in hours
 * @param isLeaderOrHealer - Whether cat is leader or healer
 * @returns Death probability (0.0 to 1.0)
 */
export function getDeathChance(ageInHours: number, isLeaderOrHealer: boolean): number {
  const threshold = isLeaderOrHealer ? 57.6 : 48
  
  if (ageInHours < threshold) {
    return 0
  }
  
  const hoursPastThreshold = ageInHours - threshold
  const baseChance = 0.01 // 1% at threshold
  const additionalChance = hoursPastThreshold * 0.005 // +0.5% per hour
  
  return baseChance + additionalChance
}

/**
 * Check if cat should die this tick (random roll).
 *
 * @param ageInHours - Cat's age in hours
 * @param isLeaderOrHealer - Whether cat is leader or healer
 * @returns True if cat dies this tick
 */
export function shouldDieOfOldAge(ageInHours: number, isLeaderOrHealer: boolean): boolean {
  const deathChance = getDeathChance(ageInHours, isLeaderOrHealer)
  return Math.random() < deathChance
}

/**
 * Get age modifier for skill learning.
 *
 * Modifiers:
 * - Young: 1.5x (learn faster)
 * - Adult: 1.0x (normal)
 * - Elder: 0.5x (learn slower)
 * - Kitten: 0x (can't learn)
 *
 * @param lifeStage - Cat's life stage
 * @returns Multiplier for skill gain
 */
export function getAgeSkillModifier(lifeStage: LifeStage): number {
  switch (lifeStage) {
    case 'kitten':
      return 0
    case 'young':
      return 1.5
    case 'adult':
      return 1.0
    case 'elder':
      return 0.5
    default:
      return 1.0
  }
}

/**
 * Check if cat can perform a specific task based on age.
 *
 * @param lifeStage - Cat's life stage
 * @param taskRequiresOutside - Whether task is outside colony
 * @param isDangerousTask - Whether task is dangerous (hunting, patrol)
 * @returns True if cat can perform task
 */
export function canPerformTask(
  lifeStage: LifeStage,
  taskRequiresOutside: boolean,
  isDangerousTask: boolean
): boolean {
  // Kittens cannot go outside or do dangerous tasks
  if (lifeStage === 'kitten') {
    return !taskRequiresOutside && !isDangerousTask
  }
  
  // Young cats cannot do dangerous tasks alone
  if (lifeStage === 'young') {
    return !isDangerousTask
  }
  
  // Adults and elders can do all tasks
  return true
}


