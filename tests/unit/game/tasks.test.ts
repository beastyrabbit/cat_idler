/**
 * Tests for Task Assignment System
 *
 * TASK: LOGIC-006
 */

import { describe, it, expect } from 'vitest'
import { getOptimalCatForTask, getAssignedCat, getAssignmentTime } from '@/lib/game/tasks'
import { createMockCat, createMockHunterCat, createMockHealerCat, createMockAdultCat } from '@/tests/factories/cat'
import type { TaskType } from '@/types/game'

describe('getOptimalCatForTask', () => {
  it('should pick cat with highest hunting for hunt task', () => {
    const cats = [
      createMockAdultCat({ name: 'LowHunter', stats: { ...createMockAdultCat().stats, hunting: 30 } }),
      createMockHunterCat({ name: 'HighHunter', birthTime: Date.now() - 30 * 60 * 60 * 1000 }),
      createMockAdultCat({ name: 'Other', stats: { ...createMockAdultCat().stats, hunting: 20 } }),
    ]
    const result = getOptimalCatForTask(cats, 'hunt')
    expect(result?.name).toBe('HighHunter')
  })

  it('should pick cat with highest medicine for heal task', () => {
    const cats = [
      createMockAdultCat({ name: 'LowHealer', stats: { ...createMockAdultCat().stats, medicine: 30 } }),
      createMockHealerCat({ name: 'HighHealer', birthTime: Date.now() - 30 * 60 * 60 * 1000 }),
    ]
    const result = getOptimalCatForTask(cats, 'heal')
    expect(result?.name).toBe('HighHealer')
  })

  it('should return null if no cats available', () => {
    const result = getOptimalCatForTask([], 'hunt')
    expect(result).toBeNull()
  })

  it('should prefer adult cats over kittens for dangerous tasks', () => {
    const kitten = createMockCat({ 
      name: 'Kitten',
      birthTime: Date.now() - 2 * 60 * 60 * 1000, // 2 hours old
      stats: { ...createMockCat().stats, hunting: 80 }
    })
    const adult = createMockCat({
      name: 'Adult',
      birthTime: Date.now() - 30 * 60 * 60 * 1000, // 30 hours old
      stats: { ...createMockCat().stats, hunting: 70 }
    })
    const result = getOptimalCatForTask([kitten, adult], 'hunt')
    // Should prefer adult even with lower skill
    expect(result?.name).toBe('Adult')
  })
})

describe('getAssignmentTime', () => {
  it('should return 30s for bad leader (0-10)', () => {
    expect(getAssignmentTime(5)).toBe(30)
    expect(getAssignmentTime(10)).toBe(30)
  })

  it('should return 20s for okay leader (11-20)', () => {
    expect(getAssignmentTime(15)).toBe(20)
    expect(getAssignmentTime(20)).toBe(20)
  })

  it('should return 10s for good leader (21-30)', () => {
    expect(getAssignmentTime(25)).toBe(10)
    expect(getAssignmentTime(30)).toBe(10)
  })

  it('should return 5s for great leader (31+)', () => {
    expect(getAssignmentTime(35)).toBe(5)
    expect(getAssignmentTime(100)).toBe(5)
  })
})

describe('getAssignedCat', () => {
  const baseTime = Date.now() - 30 * 60 * 60 * 1000 // 30 hours ago
  const cats = [
    createMockHunterCat({ name: 'Hunter', birthTime: baseTime }),
    createMockHealerCat({ name: 'Healer', birthTime: baseTime }),
    createMockAdultCat({ name: 'Builder', stats: { ...createMockAdultCat().stats, building: 80 }, birthTime: baseTime }),
  ]

  it('should always assign optimal cat for great leader (31+)', () => {
    const result = getAssignedCat(cats, 'hunt', 35)
    expect(result.cat?.name).toBe('Hunter')
    expect(result.isOptimal).toBe(true)
  })

  it('should sometimes assign wrong cat for bad leader (0-10)', () => {
    let wrongAssignments = 0
    // Run multiple times to test probability
    for (let i = 0; i < 100; i++) {
      const result = getAssignedCat(cats, 'hunt', 5)
      if (!result.isOptimal) {
        wrongAssignments++
      }
    }
    // Bad leader should assign wrong cat ~40% of the time
    expect(wrongAssignments).toBeGreaterThan(20) // At least 20% wrong
    expect(wrongAssignments).toBeLessThan(60) // But not more than 60%
  })

  it('should return null if no cats available', () => {
    const result = getAssignedCat([], 'hunt', 50)
    expect(result.cat).toBeNull()
    expect(result.isOptimal).toBe(false)
  })
})



