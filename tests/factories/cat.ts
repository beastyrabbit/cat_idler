/**
 * Test Factory: Cat
 *
 * Creates mock Cat objects for testing.
 * Use this to avoid repeating boilerplate in tests.
 *
 * Usage:
 * import { createMockCat } from '@/tests/factories/cat'
 *
 * const cat = createMockCat({ name: 'Whiskers' })
 * const hungryCat = createMockCat({ needs: { ...defaultNeeds, hunger: 10 } })
 */

import type { Cat, CatStats, CatNeeds, Position } from '@/types/game'

// Default values
const DEFAULT_STATS: CatStats = {
  attack: 50,
  defense: 50,
  hunting: 50,
  medicine: 50,
  cleaning: 50,
  building: 50,
  leadership: 50,
  vision: 50,
}

const DEFAULT_NEEDS: CatNeeds = {
  hunger: 100,
  thirst: 100,
  rest: 100,
  health: 100,
}

const DEFAULT_POSITION: Position = {
  map: 'colony',
  x: 0,
  y: 0,
}

let catIdCounter = 0

/**
 * Create a mock Cat for testing.
 *
 * @param overrides - Partial Cat object to override defaults
 * @returns Complete Cat object
 */
export function createMockCat(overrides: Partial<Cat> = {}): Cat {
  catIdCounter++

  return {
    _id: `cat_${catIdCounter}` as Cat['_id'],
    colonyId: 'colony_1' as Cat['colonyId'],
    name: `TestCat${catIdCounter}`,
    parentIds: [null, null],
    birthTime: Date.now(),
    deathTime: null,
    stats: { ...DEFAULT_STATS, ...overrides.stats },
    needs: { ...DEFAULT_NEEDS, ...overrides.needs },
    currentTask: null,
    position: { ...DEFAULT_POSITION, ...overrides.position },
    isPregnant: false,
    pregnancyDueTime: null,
    spriteParams: null,
    ...overrides,
  }
}

/**
 * Create a kitten (0-6 hours old).
 */
export function createMockKitten(overrides: Partial<Cat> = {}): Cat {
  return createMockCat({
    birthTime: Date.now() - 2 * 60 * 60 * 1000, // 2 hours ago
    stats: {
      ...DEFAULT_STATS,
      attack: 5,
      defense: 5,
      hunting: 5,
      medicine: 5,
      cleaning: 5,
      building: 5,
      leadership: 5,
      vision: 20,
    },
    ...overrides,
  })
}

/**
 * Create a young adult cat (6-24 hours old).
 */
export function createMockYoungCat(overrides: Partial<Cat> = {}): Cat {
  return createMockCat({
    birthTime: Date.now() - 12 * 60 * 60 * 1000, // 12 hours ago
    stats: {
      ...DEFAULT_STATS,
      attack: 30,
      defense: 30,
      hunting: 30,
      medicine: 30,
      cleaning: 30,
      building: 30,
      leadership: 20,
      vision: 40,
    },
    ...overrides,
  })
}

/**
 * Create an adult cat (24-48 hours old).
 */
export function createMockAdultCat(overrides: Partial<Cat> = {}): Cat {
  return createMockCat({
    birthTime: Date.now() - 36 * 60 * 60 * 1000, // 36 hours ago
    ...overrides,
  })
}

/**
 * Create an elder cat (48+ hours old).
 */
export function createMockElderCat(overrides: Partial<Cat> = {}): Cat {
  return createMockCat({
    birthTime: Date.now() - 60 * 60 * 60 * 1000, // 60 hours ago
    stats: {
      ...DEFAULT_STATS,
      attack: 40,
      defense: 60,
      hunting: 30, // Reduced hunting ability
      medicine: 80, // Good at healing
      cleaning: 70,
      building: 50,
      leadership: 80, // High leadership
      vision: 30, // Reduced vision
    },
    ...overrides,
  })
}

/**
 * Create a hungry cat (low hunger need).
 */
export function createMockHungryCat(overrides: Partial<Cat> = {}): Cat {
  return createMockCat({
    needs: {
      ...DEFAULT_NEEDS,
      hunger: 15,
      ...overrides.needs,
    },
    ...overrides,
  })
}

/**
 * Create an injured cat (low health).
 */
export function createMockInjuredCat(overrides: Partial<Cat> = {}): Cat {
  return createMockCat({
    needs: {
      ...DEFAULT_NEEDS,
      health: 30,
      ...overrides.needs,
    },
    ...overrides,
  })
}

/**
 * Create a cat specialized in hunting.
 */
export function createMockHunterCat(overrides: Partial<Cat> = {}): Cat {
  return createMockCat({
    stats: {
      ...DEFAULT_STATS,
      hunting: 90,
      vision: 80,
      attack: 70,
    },
    ...overrides,
  })
}

/**
 * Create a cat specialized in healing.
 */
export function createMockHealerCat(overrides: Partial<Cat> = {}): Cat {
  return createMockCat({
    stats: {
      ...DEFAULT_STATS,
      medicine: 90,
      vision: 60,
    },
    ...overrides,
  })
}

/**
 * Create a cat specialized in leadership.
 */
export function createMockLeaderCat(overrides: Partial<Cat> = {}): Cat {
  return createMockCat({
    stats: {
      ...DEFAULT_STATS,
      leadership: 90,
      attack: 70,
      defense: 70,
    },
    ...overrides,
  })
}

/**
 * Reset the cat ID counter (useful in beforeEach).
 */
export function resetCatIdCounter(): void {
  catIdCounter = 0
}

