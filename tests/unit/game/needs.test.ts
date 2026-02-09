/**
 * Tests for Cat Needs System
 *
 * TASK: LOGIC-001, LOGIC-002, LOGIC-003
 *
 * Instructions for developers:
 * 1. Read each test carefully
 * 2. Run tests: npm test -- tests/unit/game/needs.test.ts
 * 3. See them fail (RED)
 * 4. Implement the functions in lib/game/needs.ts
 * 5. Run tests again until they pass (GREEN)
 * 6. Refactor if needed
 */

import { describe, it, expect } from 'vitest'
import {
  decayNeeds,
  restoreHunger,
  restoreThirst,
  restoreRest,
  restoreHealth,
  applyNeedsDamage,
  applyNeedsDamageOverTime,
  hasNeedsCritical,
  isDead,
} from '@/lib/game/needs'
import type { CatNeeds } from '@/types/game'

// Helper to create fresh needs object
function createNeeds(overrides: Partial<CatNeeds> = {}): CatNeeds {
  return {
    hunger: 100,
    thirst: 100,
    rest: 100,
    health: 100,
    ...overrides,
  }
}

// =============================================================================
// LOGIC-001: decayNeeds
// =============================================================================

describe('decayNeeds', () => {
  it('should decay hunger by 5 per tick', () => {
    const needs = createNeeds()
    const result = decayNeeds(needs, 1)
    expect(result.hunger).toBe(95)
  })

  it('should decay thirst by 3 per tick', () => {
    const needs = createNeeds()
    const result = decayNeeds(needs, 1)
    expect(result.thirst).toBe(97)
  })

  it('should decay rest by 2 per tick', () => {
    const needs = createNeeds()
    const result = decayNeeds(needs, 1)
    expect(result.rest).toBe(98)
  })

  it('should NOT decay health', () => {
    const needs = createNeeds({ health: 50 })
    const result = decayNeeds(needs, 1)
    expect(result.health).toBe(50)
  })

  it('should not go below 0 for hunger', () => {
    const needs = createNeeds({ hunger: 3 })
    const result = decayNeeds(needs, 1)
    expect(result.hunger).toBe(0)
  })

  it('should not go below 0 for thirst', () => {
    const needs = createNeeds({ thirst: 1 })
    const result = decayNeeds(needs, 1)
    expect(result.thirst).toBe(0)
  })

  it('should not go below 0 for rest', () => {
    const needs = createNeeds({ rest: 1 })
    const result = decayNeeds(needs, 1)
    expect(result.rest).toBe(0)
  })

  it('should handle multiple ticks', () => {
    const needs = createNeeds()
    const result = decayNeeds(needs, 3)
    expect(result.hunger).toBe(85) // 100 - (5 * 3)
    expect(result.thirst).toBe(91) // 100 - (3 * 3)
    expect(result.rest).toBe(94) // 100 - (2 * 3)
  })

  it('should handle 0 ticks (no change)', () => {
    const needs = createNeeds()
    const result = decayNeeds(needs, 0)
    expect(result).toEqual(needs)
  })

  it('should not mutate the original object', () => {
    const needs = createNeeds()
    const original = { ...needs }
    decayNeeds(needs, 1)
    expect(needs).toEqual(original)
  })
})

// =============================================================================
// LOGIC-002: restoreHunger
// =============================================================================

describe('restoreHunger', () => {
  it('should add 30 hunger by default', () => {
    const needs = createNeeds({ hunger: 50 })
    const result = restoreHunger(needs)
    expect(result.hunger).toBe(80)
  })

  it('should add custom amount', () => {
    const needs = createNeeds({ hunger: 50 })
    const result = restoreHunger(needs, 20)
    expect(result.hunger).toBe(70)
  })

  it('should cap at 100', () => {
    const needs = createNeeds({ hunger: 90 })
    const result = restoreHunger(needs, 30)
    expect(result.hunger).toBe(100)
  })

  it('should not affect other needs', () => {
    const needs = createNeeds({ hunger: 50, thirst: 60, rest: 70, health: 80 })
    const result = restoreHunger(needs, 30)
    expect(result.thirst).toBe(60)
    expect(result.rest).toBe(70)
    expect(result.health).toBe(80)
  })

  it('should not mutate the original object', () => {
    const needs = createNeeds({ hunger: 50 })
    const original = { ...needs }
    restoreHunger(needs, 30)
    expect(needs).toEqual(original)
  })
})

// =============================================================================
// LOGIC-002: restoreThirst
// =============================================================================

describe('restoreThirst', () => {
  it('should add 40 thirst by default', () => {
    const needs = createNeeds({ thirst: 50 })
    const result = restoreThirst(needs)
    expect(result.thirst).toBe(90)
  })

  it('should cap at 100', () => {
    const needs = createNeeds({ thirst: 80 })
    const result = restoreThirst(needs, 40)
    expect(result.thirst).toBe(100)
  })
})

// =============================================================================
// LOGIC-002: restoreRest
// =============================================================================

describe('restoreRest', () => {
  it('should add 20 rest without beds', () => {
    const needs = createNeeds({ rest: 50 })
    const result = restoreRest(needs, 20, false)
    expect(result.rest).toBe(70)
  })

  it('should add 30 rest with beds', () => {
    const needs = createNeeds({ rest: 50 })
    const result = restoreRest(needs, 30, true)
    expect(result.rest).toBe(80)
  })

  it('should cap at 100', () => {
    const needs = createNeeds({ rest: 90 })
    const result = restoreRest(needs, 30, true)
    expect(result.rest).toBe(100)
  })
})

// =============================================================================
// LOGIC-002: restoreHealth
// =============================================================================

describe('restoreHealth', () => {
  it('should add specified health', () => {
    const needs = createNeeds({ health: 50 })
    const result = restoreHealth(needs, 10)
    expect(result.health).toBe(60)
  })

  it('should cap at 100', () => {
    const needs = createNeeds({ health: 95 })
    const result = restoreHealth(needs, 20)
    expect(result.health).toBe(100)
  })
})

// =============================================================================
// LOGIC-003: applyNeedsDamage
// =============================================================================

describe('applyNeedsDamage', () => {
  it('should deal 5 damage when starving (hunger = 0)', () => {
    const needs = createNeeds({ hunger: 0 })
    const result = applyNeedsDamage(needs)
    expect(result.health).toBe(95)
  })

  it('should deal 3 damage when dehydrated (thirst = 0)', () => {
    const needs = createNeeds({ thirst: 0 })
    const result = applyNeedsDamage(needs)
    expect(result.health).toBe(97)
  })

  it('should deal 8 damage when both starving and dehydrated', () => {
    const needs = createNeeds({ hunger: 0, thirst: 0 })
    const result = applyNeedsDamage(needs)
    expect(result.health).toBe(92)
  })

  it('should NOT damage if hunger is above 0', () => {
    const needs = createNeeds({ hunger: 1 })
    const result = applyNeedsDamage(needs)
    expect(result.health).toBe(100)
  })

  it('should NOT damage if thirst is above 0', () => {
    const needs = createNeeds({ thirst: 1 })
    const result = applyNeedsDamage(needs)
    expect(result.health).toBe(100)
  })

  it('should not go below 0 health', () => {
    const needs = createNeeds({ hunger: 0, thirst: 0, health: 5 })
    const result = applyNeedsDamage(needs)
    expect(result.health).toBe(0)
  })

  it('should not affect hunger, thirst, or rest', () => {
    const needs = createNeeds({ hunger: 0, thirst: 0, rest: 50 })
    const result = applyNeedsDamage(needs)
    expect(result.hunger).toBe(0)
    expect(result.thirst).toBe(0)
    expect(result.rest).toBe(50)
  })
})

describe('applyNeedsDamageOverTime', () => {
  it('returns unchanged needs when tickCount is 0', () => {
    const needs = createNeeds({ hunger: 0, thirst: 0, health: 50 })
    const result = applyNeedsDamageOverTime(needs, 0)
    expect(result).toEqual(needs)
  })

  it('applies starvation and dehydration damage scaled by tickCount', () => {
    const needs = createNeeds({ hunger: 0, thirst: 0, health: 100 })
    const result = applyNeedsDamageOverTime(needs, 2)
    expect(result.health).toBe(84) // (5 + 3) * 2
  })

  it('clamps health to 0', () => {
    const needs = createNeeds({ hunger: 0, thirst: 0, health: 3 })
    const result = applyNeedsDamageOverTime(needs, 1)
    expect(result.health).toBe(0)
  })

  it('applies starvation-only damage branch', () => {
    const needs = createNeeds({ hunger: 0, thirst: 10, health: 100 })
    const result = applyNeedsDamageOverTime(needs, 2)
    expect(result.health).toBe(90)
  })

  it('applies dehydration-only damage branch', () => {
    const needs = createNeeds({ hunger: 10, thirst: 0, health: 100 })
    const result = applyNeedsDamageOverTime(needs, 2)
    expect(result.health).toBe(94)
  })
})

// =============================================================================
// hasNeedsCritical
// =============================================================================

describe('hasNeedsCritical', () => {
  it('should return false when all needs are high', () => {
    const needs = createNeeds()
    expect(hasNeedsCritical(needs)).toBe(false)
  })

  it('should return true when hunger is below threshold', () => {
    const needs = createNeeds({ hunger: 10 })
    expect(hasNeedsCritical(needs, 15)).toBe(true)
  })

  it('should return true when thirst is below threshold', () => {
    const needs = createNeeds({ thirst: 10 })
    expect(hasNeedsCritical(needs, 15)).toBe(true)
  })

  it('should return true when rest is below threshold', () => {
    const needs = createNeeds({ rest: 10 })
    expect(hasNeedsCritical(needs, 15)).toBe(true)
  })

  it('should return true when health is below threshold', () => {
    const needs = createNeeds({ health: 10 })
    expect(hasNeedsCritical(needs, 15)).toBe(true)
  })

  it('should use custom threshold', () => {
    const needs = createNeeds({ hunger: 25 })
    expect(hasNeedsCritical(needs, 30)).toBe(true)
    expect(hasNeedsCritical(needs, 20)).toBe(false)
  })
})

// =============================================================================
// isDead
// =============================================================================

describe('isDead', () => {
  it('should return false when health is above 0', () => {
    const needs = createNeeds({ health: 1 })
    expect(isDead(needs)).toBe(false)
  })

  it('should return true when health is 0', () => {
    const needs = createNeeds({ health: 0 })
    expect(isDead(needs)).toBe(true)
  })
})



