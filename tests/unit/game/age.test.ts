/**
 * Tests for Cat Age System
 *
 * TASK: LOGIC-004
 *
 * Instructions for developers:
 * 1. Read each test carefully
 * 2. Run tests: bun test tests/unit/game/age.test.ts
 * 3. See them fail (RED)
 * 4. Implement the functions in lib/game/age.ts
 * 5. Run tests again until they pass (GREEN)
 * 6. Refactor if needed
 */

import { describe, it, expect, vi } from 'vitest'
import {
  getAgeInHours,
  getLifeStage,
  getDeathChance,
  shouldDieOfOldAge,
  getAgeSkillModifier,
  canPerformTask,
} from '@/lib/game/age'

// =============================================================================
// LOGIC-004: getAgeInHours
// =============================================================================

describe('getAgeInHours', () => {
  it('should calculate hours from birth', () => {
    const birthTime = Date.now() - 3 * 60 * 60 * 1000 // 3 hours ago
    const currentTime = Date.now()
    const result = getAgeInHours(birthTime, currentTime)
    expect(result).toBeCloseTo(3, 0)
  })

  it('should handle fractional hours', () => {
    const birthTime = Date.now() - 90 * 60 * 1000 // 1.5 hours ago
    const currentTime = Date.now()
    const result = getAgeInHours(birthTime, currentTime)
    expect(result).toBeCloseTo(1.5, 1)
  })

  it('should return 0 for just born cat', () => {
    const birthTime = Date.now()
    const currentTime = Date.now()
    const result = getAgeInHours(birthTime, currentTime)
    expect(result).toBeCloseTo(0, 1)
  })
})

// =============================================================================
// LOGIC-004: getLifeStage
// =============================================================================

describe('getLifeStage', () => {
  it('should return kitten for 0-6 hours', () => {
    expect(getLifeStage(0)).toBe('kitten')
    expect(getLifeStage(5)).toBe('kitten')
    expect(getLifeStage(5.9)).toBe('kitten')
  })

  it('should return young for 6-24 hours', () => {
    expect(getLifeStage(6)).toBe('young')
    expect(getLifeStage(12)).toBe('young')
    expect(getLifeStage(23.9)).toBe('young')
  })

  it('should return adult for 24-48 hours', () => {
    expect(getLifeStage(24)).toBe('adult')
    expect(getLifeStage(36)).toBe('adult')
    expect(getLifeStage(47.9)).toBe('adult')
  })

  it('should return elder for 48+ hours', () => {
    expect(getLifeStage(48)).toBe('elder')
    expect(getLifeStage(60)).toBe('elder')
    expect(getLifeStage(100)).toBe('elder')
  })
})

// =============================================================================
// LOGIC-004: getDeathChance
// =============================================================================

describe('getDeathChance', () => {
  it('should return 0 for cats under 48 hours', () => {
    expect(getDeathChance(47, false)).toBe(0)
    expect(getDeathChance(0, false)).toBe(0)
  })

  it('should return 1% at exactly 48 hours', () => {
    expect(getDeathChance(48, false)).toBe(0.01)
  })

  it('should increase by 0.5% per hour after 48', () => {
    expect(getDeathChance(50, false)).toBeCloseTo(0.02, 2) // 1% + (2 * 0.5%)
    expect(getDeathChance(52, false)).toBeCloseTo(0.03, 2) // 1% + (4 * 0.5%)
    expect(getDeathChance(60, false)).toBeCloseTo(0.07, 2) // 1% + (12 * 0.5%)
  })

  it('should use 57.6h threshold for leaders/healers', () => {
    expect(getDeathChance(48, true)).toBe(0) // 48 < 57.6
    expect(getDeathChance(57.6, true)).toBeCloseTo(0.01, 2)
    expect(getDeathChance(60, true)).toBeCloseTo(0.022, 2) // 1% + (2.4 * 0.5%) = 1% + 1.2% = 2.2%
  })

  it('should cap at reasonable maximum', () => {
    // Very old cats should have high but not 100% death chance
    const chance = getDeathChance(200, false)
    expect(chance).toBeGreaterThan(0.5)
    expect(chance).toBeLessThan(1.0)
  })
})

// =============================================================================
// shouldDieOfOldAge
// =============================================================================

describe('shouldDieOfOldAge', () => {
  it('should return false for young cats', () => {
    expect(shouldDieOfOldAge(47, false)).toBe(false)
  })

  it('should return true sometimes for old cats', () => {
    // Mock Math.random to test probability
    const originalRandom = Math.random
    
    // Test with death (random < chance) - age 48 has 1% chance
    vi.spyOn(Math, 'random').mockReturnValue(0.005)
    const result1 = shouldDieOfOldAge(48, false)
    expect(result1).toBe(true) // 0.005 < 0.01, so should die
    
    // Test without death (random > chance) - age 48 has 1% chance
    vi.spyOn(Math, 'random').mockReturnValue(0.99)
    const result2 = shouldDieOfOldAge(48, false)
    expect(result2).toBe(false) // 0.99 > 0.01, so should not die
    
    // Test with very old cat (age 200, high death chance ~77%)
    vi.spyOn(Math, 'random').mockReturnValue(0.5)
    const result3 = shouldDieOfOldAge(200, false)
    expect(result3).toBe(true) // 0.5 < ~0.77, so should die
    
    // Test with very old cat but high random (should not die)
    vi.spyOn(Math, 'random').mockReturnValue(0.99)
    const result4 = shouldDieOfOldAge(200, false)
    expect(result4).toBe(false) // 0.99 > ~0.77, so should not die
    
    Math.random = originalRandom
  })
})

// =============================================================================
// getAgeSkillModifier
// =============================================================================

describe('getAgeSkillModifier', () => {
  it('should return 0 for kittens (cannot learn)', () => {
    expect(getAgeSkillModifier('kitten')).toBe(0)
  })

  it('should return 1.5 for young cats (learn faster)', () => {
    expect(getAgeSkillModifier('young')).toBe(1.5)
  })

  it('should return 1.0 for adult cats (normal)', () => {
    expect(getAgeSkillModifier('adult')).toBe(1.0)
  })

  it('should return 0.5 for elder cats (learn slower)', () => {
    expect(getAgeSkillModifier('elder')).toBe(0.5)
  })
})

// =============================================================================
// canPerformTask
// =============================================================================

describe('canPerformTask', () => {
  it('should allow kittens to do indoor tasks', () => {
    expect(canPerformTask('kitten', false, false)).toBe(true)
  })

  it('should NOT allow kittens to go outside', () => {
    expect(canPerformTask('kitten', true, false)).toBe(false)
  })

  it('should NOT allow kittens to do dangerous tasks', () => {
    expect(canPerformTask('kitten', false, true)).toBe(false)
  })

  it('should allow young cats to do safe outdoor tasks', () => {
    expect(canPerformTask('young', true, false)).toBe(true)
  })

  it('should NOT allow young cats to do dangerous tasks alone', () => {
    expect(canPerformTask('young', true, true)).toBe(false)
  })

  it('should allow adults to do all tasks', () => {
    expect(canPerformTask('adult', false, false)).toBe(true)
    expect(canPerformTask('adult', true, false)).toBe(true)
    expect(canPerformTask('adult', true, true)).toBe(true)
  })

  it('should allow elders to do all tasks (but with reduced efficiency)', () => {
    expect(canPerformTask('elder', false, false)).toBe(true)
    expect(canPerformTask('elder', true, false)).toBe(true)
    expect(canPerformTask('elder', true, true)).toBe(true)
  })
})



