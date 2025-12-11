/**
 * Cat Needs System
 *
 * Pure functions for managing cat needs (hunger, thirst, rest, health).
 * See TASKS.md: LOGIC-001, LOGIC-002, LOGIC-003
 *
 * TODO: Implement these functions using TDD
 * 1. Write tests first in tests/unit/game/needs.test.ts
 * 2. Implement functions to make tests pass
 */

import type { CatNeeds } from '@/types/game'
import { NEEDS_DECAY_RATES, NEEDS_RESTORE_AMOUNTS } from '@/types/game'

/**
 * Decay cat needs over time.
 *
 * Rules:
 * - Hunger: -5 per tick
 * - Thirst: -3 per tick
 * - Rest: -2 per tick
 * - Health: unchanged (only decreases from injury)
 * - Minimum value is 0
 *
 * @param currentNeeds - Current needs state
 * @param tickCount - Number of ticks to decay
 * @returns New needs state after decay
 */
export function decayNeeds(currentNeeds: CatNeeds, tickCount: number): CatNeeds {
  // TODO: Implement (LOGIC-001)
  throw new Error('Not implemented')
}

/**
 * Restore hunger when cat eats.
 *
 * Rules:
 * - Add specified amount to hunger
 * - Cap at 100
 *
 * @param needs - Current needs state
 * @param amount - Amount to restore (default: 30)
 * @returns New needs state
 */
export function restoreHunger(needs: CatNeeds, amount: number = NEEDS_RESTORE_AMOUNTS.eating): CatNeeds {
  // TODO: Implement (LOGIC-002)
  throw new Error('Not implemented')
}

/**
 * Restore thirst when cat drinks.
 *
 * Rules:
 * - Add specified amount to thirst
 * - Cap at 100
 *
 * @param needs - Current needs state
 * @param amount - Amount to restore (default: 40)
 * @returns New needs state
 */
export function restoreThirst(needs: CatNeeds, amount: number = NEEDS_RESTORE_AMOUNTS.drinking): CatNeeds {
  // TODO: Implement (LOGIC-002)
  throw new Error('Not implemented')
}

/**
 * Restore rest when cat sleeps.
 *
 * Rules:
 * - Add 20 rest normally, or 30 with beds
 * - Cap at 100
 *
 * @param needs - Current needs state
 * @param amount - Base amount to restore
 * @param hasBeds - Whether colony has beds building
 * @returns New needs state
 */
export function restoreRest(needs: CatNeeds, amount: number, hasBeds: boolean): CatNeeds {
  // TODO: Implement (LOGIC-002)
  throw new Error('Not implemented')
}

/**
 * Restore health when cat is healed.
 *
 * Rules:
 * - Add specified amount to health
 * - Cap at 100
 *
 * @param needs - Current needs state
 * @param amount - Amount to heal
 * @returns New needs state
 */
export function restoreHealth(needs: CatNeeds, amount: number): CatNeeds {
  // TODO: Implement (LOGIC-002)
  throw new Error('Not implemented')
}

/**
 * Apply damage from starvation/dehydration.
 *
 * Rules:
 * - If hunger = 0: -5 health
 * - If thirst = 0: -3 health
 * - Damage stacks if both are 0
 * - Health minimum is 0
 *
 * @param needs - Current needs state
 * @returns New needs state with damage applied
 */
export function applyNeedsDamage(needs: CatNeeds): CatNeeds {
  // TODO: Implement (LOGIC-003)
  throw new Error('Not implemented')
}

/**
 * Check if a cat's needs are critical (any below threshold).
 *
 * @param needs - Current needs state
 * @param threshold - Threshold to consider critical (default: 15)
 * @returns True if any need is below threshold
 */
export function hasNeedsCritical(needs: CatNeeds, threshold: number = 15): boolean {
  // TODO: Implement
  throw new Error('Not implemented')
}

/**
 * Check if cat is dead (health = 0).
 *
 * @param needs - Current needs state
 * @returns True if health is 0
 */
export function isDead(needs: CatNeeds): boolean {
  // TODO: Implement
  throw new Error('Not implemented')
}

