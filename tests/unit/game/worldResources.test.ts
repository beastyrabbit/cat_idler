/**
 * Tests for World Resource Management
 *
 * TASK: LOGIC-010
 */

import { describe, it, expect } from 'vitest'
import {
  harvestResources,
  regenerateResources,
  isTileDepleted,
} from '@/lib/game/worldResources'
import type { WorldTile } from '@/types/game'

const createForestTile = (overrides: Partial<WorldTile> = {}): WorldTile => ({
  _id: 'tile_1' as WorldTile['_id'],
  colonyId: 'colony_1' as WorldTile['colonyId'],
  x: 0,
  y: 0,
  type: 'forest',
  resources: { food: 50, herbs: 0, water: 0 },
  maxResources: { food: 100, herbs: 50 },
  dangerLevel: 25,
  pathWear: 0,
  lastDepleted: 0,
  ...overrides,
})

describe('harvestResources', () => {
  it('should harvest 1 food with 0 skill', () => {
    const tile = createForestTile()
    const result = harvestResources(tile, 'food', 0)
    expect(result.harvested).toBe(1)
    expect(result.newTile.resources.food).toBe(49)
  })

  it('should harvest 2 food with 50+ skill', () => {
    const tile = createForestTile()
    const result = harvestResources(tile, 'food', 50)
    expect(result.harvested).toBe(2)
    expect(result.newTile.resources.food).toBe(48)
  })

  it('should harvest 3 food with 100 skill', () => {
    const tile = createForestTile()
    const result = harvestResources(tile, 'food', 100)
    expect(result.harvested).toBe(3)
    expect(result.newTile.resources.food).toBe(47)
  })

  it('should not harvest more than available', () => {
    const tile = createForestTile({ resources: { food: 1, herbs: 0, water: 0 } })
    const result = harvestResources(tile, 'food', 100)
    expect(result.harvested).toBe(1)
    expect(result.newTile.resources.food).toBe(0)
  })

  it('should harvest herbs correctly', () => {
    const tile = createForestTile({ resources: { food: 0, herbs: 30, water: 0 } })
    const result = harvestResources(tile, 'herbs', 50)
    expect(result.harvested).toBe(2)
    expect(result.newTile.resources.herbs).toBe(28)
  })
})

describe('regenerateResources', () => {
  it('should regenerate 10% per hour', () => {
    const depletedTile = createForestTile({
      resources: { food: 0, herbs: 0, water: 0 },
    })
    const result = regenerateResources(depletedTile, 1)
    expect(result.resources.food).toBe(10) // 10% of 100
    expect(result.resources.herbs).toBe(5) // 10% of 50
  })

  it('should fully regenerate at 6 hours', () => {
    const depletedTile = createForestTile({
      resources: { food: 0, herbs: 0, water: 0 },
    })
    const result = regenerateResources(depletedTile, 6)
    expect(result.resources.food).toBe(100)
    expect(result.resources.herbs).toBe(50)
  })

  it('should not exceed max resources', () => {
    const tile = createForestTile({
      resources: { food: 90, herbs: 40, water: 0 },
    })
    const result = regenerateResources(tile, 2)
    expect(result.resources.food).toBe(100) // Capped
    expect(result.resources.herbs).toBe(50) // Capped
  })
})

describe('isTileDepleted', () => {
  it('should return false when resources available', () => {
    const tile = createForestTile()
    expect(isTileDepleted(tile)).toBe(false)
  })

  it('should return true when food depleted', () => {
    const tile = createForestTile({ resources: { food: 0, herbs: 0, water: 0 } })
    expect(isTileDepleted(tile)).toBe(true)
  })

  it('should return false for rivers (infinite water)', () => {
    const riverTile = createForestTile({
      type: 'river',
      resources: { food: 0, herbs: 0, water: 0 },
    })
    expect(isTileDepleted(riverTile)).toBe(false)
  })
})



