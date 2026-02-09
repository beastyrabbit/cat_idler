import { describe, expect, it } from 'vitest'

import { applySurvivalTick } from '@/lib/game/survival'

const baseNeeds = {
  hunger: 100,
  thirst: 100,
  rest: 100,
  health: 100,
}

describe('survival', () => {
  it('decays thirst faster without water and triggers dehydration start', () => {
    const result = applySurvivalTick(
      { ...baseNeeds, thirst: 1 },
      { food: 10, water: 0 },
      600,
      { needsDecayMultiplier: 1, needsDamageMultiplier: 1 },
    )

    expect(result.nextNeeds.thirst).toBe(0)
    expect(result.dehydratingStarted).toBe(true)
  })

  it('recovers thirst when water is available', () => {
    const result = applySurvivalTick(
      { ...baseNeeds, thirst: 0 },
      { food: 10, water: 10 },
      600,
      { needsDecayMultiplier: 1, needsDamageMultiplier: 1 },
    )

    expect(result.nextNeeds.thirst).toBeGreaterThan(0)
    expect(result.recoveredFromDehydration).toBe(true)
  })

  it('applies dehydration damage and can kill the cat', () => {
    const result = applySurvivalTick(
      { ...baseNeeds, thirst: 0, health: 2 },
      { food: 10, water: 0 },
      600,
      { needsDecayMultiplier: 1, needsDamageMultiplier: 1 },
    )

    expect(result.nextNeeds.health).toBe(0)
    expect(result.died).toBe(true)
  })

  it('applies multipliers to tune risk profile by policy', () => {
    const normal = applySurvivalTick(
      { ...baseNeeds, thirst: 100 },
      { food: 10, water: 0 },
      600,
      { needsDecayMultiplier: 1, needsDamageMultiplier: 1 },
    )

    const simple = applySurvivalTick(
      { ...baseNeeds, thirst: 100 },
      { food: 10, water: 0 },
      600,
      { needsDecayMultiplier: 1.25, needsDamageMultiplier: 1.3 },
    )

    expect(simple.nextNeeds.thirst).toBeLessThan(normal.nextNeeds.thirst)
  })

  it('recovers hunger when food is available', () => {
    const result = applySurvivalTick(
      { ...baseNeeds, hunger: 50 },
      { food: 10, water: 10 },
      600,
      { needsDecayMultiplier: 1, needsDamageMultiplier: 1 },
    )

    expect(result.nextNeeds.hunger).toBeGreaterThan(50)
  })

  it('applies starvation damage branch when hunger reaches zero', () => {
    const result = applySurvivalTick(
      { ...baseNeeds, hunger: 1, health: 5 },
      { food: 0, water: 10 },
      600,
      { needsDecayMultiplier: 1, needsDamageMultiplier: 1 },
    )

    expect(result.nextNeeds.hunger).toBe(0)
    expect(result.nextNeeds.health).toBeLessThan(5)
  })
})
