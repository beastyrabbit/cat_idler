/**
 * Tests for Combat Resolution System
 *
 * TASK: LOGIC-007
 */

import { describe, it, expect, vi } from 'vitest'
import { calculateCombatResult, getClicksNeeded } from '@/lib/game/combat'

describe('calculateCombatResult', () => {
  it('should favor high stat cats', () => {
    let catWins = 0
    for (let i = 0; i < 100; i++) {
      const result = calculateCombatResult(80, 80, 20)
      if (result.won) catWins++
    }
    // High stat cat should win most of the time
    expect(catWins).toBeGreaterThan(70)
  })

  it('should return damage between 30-70 on loss', () => {
    // Force a loss scenario with very low stats
    let foundLoss = false
    for (let i = 0; i < 100; i++) {
      const result = calculateCombatResult(1, 1, 100)
      if (!result.won) {
        foundLoss = true
        expect(result.damage).toBeGreaterThanOrEqual(30)
        expect(result.damage).toBeLessThanOrEqual(70)
      }
    }
    expect(foundLoss).toBe(true)
  })

  it('should sometimes win and sometimes lose with balanced stats', () => {
    let wins = 0
    let losses = 0
    // Cat has 50+50=100 total, enemy has 100, so should be roughly equal
    for (let i = 0; i < 100; i++) {
      const result = calculateCombatResult(50, 50, 100)
      if (result.won) wins++
      else losses++
    }
    // Should have both wins and losses with balanced stats
    expect(wins).toBeGreaterThan(10)
    expect(losses).toBeGreaterThan(10)
  })

  it('should return won: true when cat wins', () => {
    // Mock random to ensure win
    vi.spyOn(Math, 'random')
      .mockReturnValueOnce(0.9) // Cat roll high
      .mockReturnValueOnce(0.1) // Enemy roll low
    
    const result = calculateCombatResult(80, 80, 20)
    expect(result.won).toBe(true)
    
    vi.restoreAllMocks()
  })
})

describe('getClicksNeeded', () => {
  it('should return base clicks with no modifiers', () => {
    const result = getClicksNeeded(50, 0, 0)
    expect(result).toBe(50)
  })

  it('should reduce clicks with colony defense', () => {
    const result = getClicksNeeded(50, 50, 0)
    expect(result).toBe(25) // 50 * 0.5
  })

  it('should reduce clicks with cat vision', () => {
    const result = getClicksNeeded(50, 0, 100)
    expect(result).toBe(25) // 50 * 0.5
  })

  it('should stack both modifiers', () => {
    const result = getClicksNeeded(100, 50, 100)
    expect(result).toBe(25) // 100 * 0.5 * 0.5
  })

  it('should not go below 1 click', () => {
    const result = getClicksNeeded(10, 90, 90)
    expect(result).toBeGreaterThanOrEqual(1)
  })
})



