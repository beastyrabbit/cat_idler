import { describe, expect, it } from 'vitest'

import { planHousePipeline } from '@/lib/game/housePlanner'

describe('housePlanner', () => {
  it('queues prerequisite water and materials before construction', () => {
    const planned = planHousePipeline({
      resources: { water: 0, materials: 0 },
      activeOrQueuedJobs: [],
      waterRequired: 8,
      materialsRequired: 10,
    })

    expect(planned).toEqual([
      { kind: 'supply_water', metadata: { reason: 'house_prereq' } },
      { kind: 'build_house', metadata: { phase: 'gather_materials', reason: 'house_prereq' } },
    ])
  })

  it('queues construction when prerequisites are met', () => {
    const planned = planHousePipeline({
      resources: { water: 12, materials: 12 },
      activeOrQueuedJobs: [],
      waterRequired: 8,
      materialsRequired: 10,
    })

    expect(planned).toEqual([
      { kind: 'build_house', metadata: { phase: 'construct_house' } },
    ])
  })

  it('does not duplicate prerequisite jobs already queued', () => {
    const planned = planHousePipeline({
      resources: { water: 1, materials: 1 },
      activeOrQueuedJobs: [
        { kind: 'supply_water', metadata: { reason: 'house_prereq' } },
        { kind: 'build_house', metadata: { phase: 'gather_materials', reason: 'house_prereq' } },
      ],
      waterRequired: 8,
      materialsRequired: 10,
    })

    expect(planned).toEqual([])
  })
})
