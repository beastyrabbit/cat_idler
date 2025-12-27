/**
 * Biome Definitions
 *
 * Defines all biome types with their properties: danger, resources, travel speed, etc.
 */

export type BiomeType =
  | 'oak_forest'
  | 'pine_forest'
  | 'jungle'
  | 'dead_forest'
  | 'mountains'
  | 'swamp'
  | 'desert'
  | 'tundra'
  | 'meadow'
  | 'cave_entrance'
  | 'enemy_lair'

export type OverlayFeature = 'river' | 'ancient_road' | 'game_trail' | 'trade_route' | null

export interface BiomeProperties {
  type: BiomeType
  baseDanger: number
  baseResources: {
    food: { min: number; max: number }
    herbs: { min: number; max: number }
    water: number
  }
  maxResources: {
    food: number
    herbs: number
  }
  travelSpeed: number // Multiplier: 1.0 = normal, 0.5 = slow, 1.5 = fast
  name: string
}

export const BIOME_PROPERTIES: Record<BiomeType, BiomeProperties> = {
  oak_forest: {
    type: 'oak_forest',
    baseDanger: 20,
    baseResources: {
      food: { min: 15, max: 40 },
      herbs: { min: 0, max: 5 },
      water: 0,
    },
    maxResources: {
      food: 50,
      herbs: 8,
    },
    travelSpeed: 1.0,
    name: 'Oak Forest',
  },
  pine_forest: {
    type: 'pine_forest',
    baseDanger: 30,
    baseResources: {
      food: { min: 10, max: 30 },
      herbs: { min: 2, max: 10 },
      water: 0,
    },
    maxResources: {
      food: 40,
      herbs: 15,
    },
    travelSpeed: 0.9,
    name: 'Pine Forest',
  },
  jungle: {
    type: 'jungle',
    baseDanger: 45,
    baseResources: {
      food: { min: 30, max: 60 },
      herbs: { min: 10, max: 25 },
      water: 0,
    },
    maxResources: {
      food: 80,
      herbs: 35,
    },
    travelSpeed: 0.6,
    name: 'Jungle',
  },
  dead_forest: {
    type: 'dead_forest',
    baseDanger: 55,
    baseResources: {
      food: { min: 0, max: 15 },
      herbs: { min: 15, max: 35 },
      water: 0,
    },
    maxResources: {
      food: 20,
      herbs: 45,
    },
    travelSpeed: 0.8,
    name: 'Dead Forest',
  },
  mountains: {
    type: 'mountains',
    baseDanger: 50,
    baseResources: {
      food: { min: 0, max: 10 },
      herbs: { min: 0, max: 5 },
      water: 0,
    },
    maxResources: {
      food: 15,
      herbs: 8,
    },
    travelSpeed: 0.5,
    name: 'Mountains',
  },
  swamp: {
    type: 'swamp',
    baseDanger: 40,
    baseResources: {
      food: { min: 5, max: 25 },
      herbs: { min: 20, max: 40 },
      water: 0,
    },
    maxResources: {
      food: 35,
      herbs: 50,
    },
    travelSpeed: 0.7,
    name: 'Swamp',
  },
  desert: {
    type: 'desert',
    baseDanger: 35,
    baseResources: {
      food: { min: 0, max: 5 },
      herbs: { min: 0, max: 2 },
      water: 0,
    },
    maxResources: {
      food: 8,
      herbs: 4,
    },
    travelSpeed: 1.3,
    name: 'Desert',
  },
  tundra: {
    type: 'tundra',
    baseDanger: 45,
    baseResources: {
      food: { min: 0, max: 8 },
      herbs: { min: 0, max: 0 },
      water: 0,
    },
    maxResources: {
      food: 12,
      herbs: 0,
    },
    travelSpeed: 0.9,
    name: 'Tundra',
  },
  meadow: {
    type: 'meadow',
    baseDanger: 10,
    baseResources: {
      food: { min: 8, max: 25 },
      herbs: { min: 0, max: 4 },
      water: 0,
    },
    maxResources: {
      food: 30,
      herbs: 6,
    },
    travelSpeed: 1.2,
    name: 'Meadow',
  },
  cave_entrance: {
    type: 'cave_entrance',
    baseDanger: 60,
    baseResources: {
      food: { min: 0, max: 0 },
      herbs: { min: 0, max: 0 },
      water: 0,
    },
    maxResources: {
      food: 0,
      herbs: 0,
    },
    travelSpeed: 1.0,
    name: 'Cave Entrance',
  },
  enemy_lair: {
    type: 'enemy_lair',
    baseDanger: 80,
    baseResources: {
      food: { min: 0, max: 0 },
      herbs: { min: 0, max: 0 },
      water: 0,
    },
    maxResources: {
      food: 0,
      herbs: 0,
    },
    travelSpeed: 0.8,
    name: 'Enemy Lair',
  },
}

export interface OverlayFeatureProperties {
  dangerModifier: number
  speedModifier: number
  initialPathWear: number
  name: string
}

export const OVERLAY_FEATURE_PROPERTIES: Record<Exclude<OverlayFeature, null>, OverlayFeatureProperties> = {
  river: {
    dangerModifier: 5, // Sets danger to 5 (overrides biome)
    speedModifier: 0.8, // Slightly slower to cross (cats can walk through shallow rivers)
    initialPathWear: 0,
    name: 'River',
  },
  ancient_road: {
    dangerModifier: -15,
    speedModifier: 0.5, // +50% speed
    initialPathWear: 60,
    name: 'Ancient Road',
  },
  game_trail: {
    dangerModifier: -5,
    speedModifier: 0.2, // +20% speed
    initialPathWear: 45,
    name: 'Game Trail',
  },
  trade_route: {
    dangerModifier: -10,
    speedModifier: 0.3, // +30% speed
    initialPathWear: 50,
    name: 'Trade Route',
  },
}

/**
 * Calculate final danger level for a tile
 */
export function calculateDangerLevel(
  biomeType: BiomeType,
  overlayFeature: OverlayFeature,
  distanceFromColony: number
): number {
  const biome = BIOME_PROPERTIES[biomeType]
  let danger = biome.baseDanger

  // Apply overlay feature modifier
  if (overlayFeature && overlayFeature !== 'river') {
    const feature = OVERLAY_FEATURE_PROPERTIES[overlayFeature]
    danger = Math.max(0, danger + feature.dangerModifier)
  } else if (overlayFeature === 'river') {
    // Rivers always have danger 5
    return 5
  }

  // Add distance modifier: +2 per unit distance, capped at 95
  danger = Math.min(95, danger + distanceFromColony * 2)

  return Math.max(0, Math.min(100, danger))
}

/**
 * Calculate travel speed multiplier for a tile
 */
export function calculateTravelSpeed(
  biomeType: BiomeType,
  overlayFeature: OverlayFeature
): number {
  const biome = BIOME_PROPERTIES[biomeType]
  let speed = biome.travelSpeed

  // Apply overlay feature speed modifier
  if (overlayFeature && overlayFeature !== 'river') {
    const feature = OVERLAY_FEATURE_PROPERTIES[overlayFeature]
    speed = speed * (1 + feature.speedModifier)
  }

  return speed
}

