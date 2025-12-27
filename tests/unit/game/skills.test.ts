/**
 * Tests for Skill Learning System
 *
 * TASK: LOGIC-005
 */

import { describe, it, expect } from 'vitest'
import { calculateSkillGain } from '@/lib/game/skills'
import type { TaskType, LifeStage } from '@/types/game'

describe('calculateSkillGain', () => {
  it('should gain base XP for hunting task', () => {
    const result = calculateSkillGain(10, 'hunt', false, 'adult')
    expect(result).toBeGreaterThan(10)
    expect(result).toBeLessThanOrEqual(13) // base is 1-3
  })

  it('should gain 50% more on success', () => {
    const baseGain = calculateSkillGain(10, 'hunt', false, 'adult') - 10
    const successGain = calculateSkillGain(10, 'hunt', true, 'adult') - 10
    expect(successGain).toBeCloseTo(baseGain * 1.5, 1)
  })

  it('should gain 50% more as young cat', () => {
    const adultGain = calculateSkillGain(10, 'hunt', false, 'adult') - 10
    const youngGain = calculateSkillGain(10, 'hunt', false, 'young') - 10
    expect(youngGain).toBeCloseTo(adultGain * 1.5, 1)
  })

  it('should gain 50% less as elder', () => {
    const adultGain = calculateSkillGain(10, 'hunt', false, 'adult') - 10
    const elderGain = calculateSkillGain(10, 'hunt', false, 'elder') - 10
    expect(elderGain).toBeCloseTo(adultGain * 0.5, 1)
  })

  it('should return 0 for kittens (cannot learn)', () => {
    const result = calculateSkillGain(10, 'hunt', true, 'kitten')
    expect(result).toBe(10) // No gain for kittens
  })

  it('should cap at 100', () => {
    const result = calculateSkillGain(99, 'hunt', true, 'young')
    expect(result).toBe(100)
  })

  it('should not go below current skill', () => {
    const result = calculateSkillGain(50, 'hunt', false, 'adult')
    expect(result).toBeGreaterThanOrEqual(50)
  })

  it('should work for different task types', () => {
    const huntResult = calculateSkillGain(10, 'hunt', false, 'adult')
    const healResult = calculateSkillGain(10, 'heal', false, 'adult')
    const buildResult = calculateSkillGain(10, 'build', false, 'adult')
    
    // All should gain some XP
    expect(huntResult).toBeGreaterThan(10)
    expect(healResult).toBeGreaterThan(10)
    expect(buildResult).toBeGreaterThan(10)
  })
})



