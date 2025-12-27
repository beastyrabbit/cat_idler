/**
 * Tests for Cat AI Autonomous Behavior
 *
 * TASK: LOGIC-008
 */

import { describe, it, expect } from 'vitest'
import { getAutonomousAction } from '@/lib/game/catAI'
import { createMockCat, createMockHungryCat } from '@/tests/factories/cat'
import type { Resources, BuildingType } from '@/types/game'

describe('getAutonomousAction', () => {
  const baseResources: Resources = {
    food: 10,
    water: 10,
    herbs: 0,
    materials: 0,
    blessings: 0,
  }

  const hasNoBuildings = (type: BuildingType) => false
  const hasBeds = (type: BuildingType) => type === 'beds'

  it('should eat when hungry and food available', () => {
    const cat = createMockHungryCat()
    const result = getAutonomousAction(cat, baseResources, hasNoBuildings)
    expect(result?.type).toBe('eat')
  })

  it('should not eat when hungry but no food', () => {
    const cat = createMockHungryCat()
    const resources = { ...baseResources, food: 0 }
    const result = getAutonomousAction(cat, resources, hasNoBuildings)
    expect(result?.type).not.toBe('eat')
  })

  it('should drink when thirsty and water available', () => {
    const cat = createMockCat({
      needs: { hunger: 100, thirst: 30, rest: 100, health: 100 },
    })
    const result = getAutonomousAction(cat, baseResources, hasNoBuildings)
    expect(result?.type).toBe('drink')
  })

  it('should prioritize eating over drinking', () => {
    const cat = createMockCat({
      needs: { hunger: 20, thirst: 30, rest: 100, health: 100 },
    })
    const result = getAutonomousAction(cat, baseResources, hasNoBuildings)
    expect(result?.type).toBe('eat')
  })

  it('should sleep when rest is low', () => {
    const cat = createMockCat({
      needs: { hunger: 100, thirst: 100, rest: 15, health: 100 },
    })
    const result = getAutonomousAction(cat, baseResources, hasNoBuildings)
    expect(result?.type).toBe('sleep')
  })

  it('should return to colony when needs critical and on world map', () => {
    const cat = createMockCat({
      needs: { hunger: 10, thirst: 50, rest: 50, health: 100 },
      position: { map: 'world', x: 5, y: 5 },
    })
    const resources = { ...baseResources, food: 0, water: 0 }
    const result = getAutonomousAction(cat, resources, hasNoBuildings)
    expect(result?.type).toBe('return_to_colony')
  })

  it('should return null when all needs are fine', () => {
    const cat = createMockCat()
    const result = getAutonomousAction(cat, baseResources, hasNoBuildings)
    expect(result).toBeNull()
  })

  it('should return null when on world map but needs are okay', () => {
    const cat = createMockCat({
      position: { map: 'world', x: 5, y: 5 },
    })
    const result = getAutonomousAction(cat, baseResources, hasNoBuildings)
    expect(result).toBeNull()
  })
})



