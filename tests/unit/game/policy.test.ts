import { describe, expect, it } from 'vitest'

import {
  bucketFromLeadership,
  configForTier,
  pickPolicyTier,
  weightsForLeadership,
} from '@/lib/game/policy'

describe('policy', () => {
  it('maps leadership into expected buckets', () => {
    expect(bucketFromLeadership(10)).toBe('bad')
    expect(bucketFromLeadership(50)).toBe('normal')
    expect(bucketFromLeadership(90)).toBe('excellent')
  })

  it('uses normalized probabilities for each leadership band', () => {
    expect(weightsForLeadership(10)).toEqual({ simple: 0.3, normal: 0.6, excellent: 0.1 })
    expect(weightsForLeadership(50)).toEqual({ simple: 0.1, normal: 0.8, excellent: 0.1 })
    expect(weightsForLeadership(90)).toEqual({ simple: 0, normal: 0.7, excellent: 0.3 })
  })

  it('selects simple/normal/excellent at decision boundaries', () => {
    expect(pickPolicyTier(10, 0.05)).toBe('simple')
    expect(pickPolicyTier(10, 0.5)).toBe('normal')
    expect(pickPolicyTier(10, 0.95)).toBe('excellent')
  })

  it('returns tier-specific config with expected tuning direction', () => {
    const simple = configForTier('simple')
    const normal = configForTier('normal')
    const excellent = configForTier('excellent')

    expect(simple.actionReliability).toBeLessThan(normal.actionReliability)
    expect(normal.actionReliability).toBeLessThanOrEqual(excellent.actionReliability)

    expect(simple.needsDecayMultiplier).toBeGreaterThan(normal.needsDecayMultiplier)
    expect(excellent.needsDecayMultiplier).toBeLessThan(normal.needsDecayMultiplier)

    expect(simple.houseMaterialsRequired).toBeGreaterThan(normal.houseMaterialsRequired)
    expect(excellent.houseMaterialsRequired).toBeLessThan(normal.houseMaterialsRequired)
  })
})
