/**
 * Tests for Path System
 *
 * TASK: LOGIC-009
 */

import { describe, it, expect } from 'vitest'
import {
  addPathWear,
  decayPathWear,
  getPathSpeedBonus,
  getPathDangerReduction,
} from '@/lib/game/paths'

describe('getPathSpeedBonus', () => {
  it('should return 0 for wear < 30', () => {
    expect(getPathSpeedBonus(0)).toBe(0)
    expect(getPathSpeedBonus(29)).toBe(0)
  })

  it('should return 0.1 for wear 30-59', () => {
    expect(getPathSpeedBonus(30)).toBe(0.1)
    expect(getPathSpeedBonus(59)).toBe(0.1)
  })

  it('should return 0.25 for wear 60-89', () => {
    expect(getPathSpeedBonus(60)).toBe(0.25)
    expect(getPathSpeedBonus(89)).toBe(0.25)
  })

  it('should return 0.4 for wear 90+', () => {
    expect(getPathSpeedBonus(90)).toBe(0.4)
    expect(getPathSpeedBonus(100)).toBe(0.4)
  })
})

describe('getPathDangerReduction', () => {
  it('should return 0 for wear < 30', () => {
    expect(getPathDangerReduction(29)).toBe(0)
  })

  it('should return 0.25 for wear 30-59', () => {
    expect(getPathDangerReduction(30)).toBe(0.25)
    expect(getPathDangerReduction(59)).toBe(0.25)
  })

  it('should return 0.6 for wear 60-89', () => {
    expect(getPathDangerReduction(60)).toBe(0.6)
    expect(getPathDangerReduction(89)).toBe(0.6)
  })

  it('should return 1.0 for wear 90+ (no encounters)', () => {
    expect(getPathDangerReduction(90)).toBe(1.0)
    expect(getPathDangerReduction(100)).toBe(1.0)
  })
})

describe('addPathWear', () => {
  it('should add wear amount', () => {
    expect(addPathWear(0, 10)).toBe(10)
    expect(addPathWear(50, 20)).toBe(70)
  })

  it('should cap at 100', () => {
    expect(addPathWear(95, 10)).toBe(100)
    expect(addPathWear(100, 10)).toBe(100)
  })
})

describe('decayPathWear', () => {
  it('should reduce wear by 1 per hour', () => {
    expect(decayPathWear(50)).toBe(49)
    expect(decayPathWear(10)).toBe(9)
  })

  it('should not go below 0', () => {
    expect(decayPathWear(0)).toBe(0)
    expect(decayPathWear(1)).toBe(0)
  })
})



