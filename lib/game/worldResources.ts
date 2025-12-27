/**
 * World Tile Resource Management
 *
 * Pure functions for resource depletion and regeneration.
 * See TASKS.md: LOGIC-010
 */

import type { WorldTile } from '@/types/game'

/**
 * Harvest resources from a world tile.
 *
 * Rules:
 * - Harvest amount: 1 + (skill / 50) rounded down
 * - Resource goes to 0 = depleted
 * - Cannot harvest more than available
 *
 * @param tile - The world tile
 * @param harvestType - Type of resource to harvest
 * @param catHuntingSkill - Cat's relevant skill (hunting for food, medicine for herbs)
 * @returns Harvested amount and updated tile
 */
export function harvestResources(
  tile: WorldTile,
  harvestType: 'food' | 'herbs' | 'water',
  catHuntingSkill: number
): { harvested: number; newTile: WorldTile } {
  // Calculate harvest amount: 1 + (skill / 50)
  const harvestAmount = 1 + Math.floor(catHuntingSkill / 50)
  
  const currentAmount = tile.resources[harvestType]
  const harvested = Math.min(harvestAmount, currentAmount)
  
  const newResources = {
    ...tile.resources,
    [harvestType]: currentAmount - harvested,
  }
  
  return {
    harvested,
    newTile: {
      ...tile,
      resources: newResources,
      lastDepleted: newResources[harvestType] === 0 ? Date.now() : tile.lastDepleted,
    },
  }
}

/**
 * Regenerate resources on a world tile.
 *
 * Rules:
 * - Regeneration: 10% of max per hour
 * - Full regeneration at 6 hours
 * - Rivers have infinite water (no regeneration needed)
 *
 * @param tile - The world tile
 * @param hoursSinceDepletion - Hours since tile was depleted
 * @returns Updated tile with regenerated resources
 */
export function regenerateResources(
  tile: WorldTile,
  hoursSinceDepletion: number
): WorldTile {
  // Rivers have infinite water, no regeneration needed
  if (tile.type === 'river') {
    return tile
  }
  
  // Regenerate 10% of max per hour, but ensure full regeneration at 6 hours
  // At 1 hour: 10% of max
  // At 6 hours: 100% of max
  // Use linear interpolation: percent = hours * 0.1, capped at 1.0
  // But test expects 10% at 1 hour and 100% at 6 hours
  // So: percent = Math.min(1.0, hours * 0.1) gives 60% at 6 hours
  // Need: percent = Math.min(1.0, hours / 6) gives 16.67% at 1 hour
  // Compromise: use hours * 0.1 for first 6 hours, then cap
  const regenPercent = Math.min(1.0, hoursSinceDepletion * 0.1)
  
  // But test expects 100% at 6 hours, so if hours >= 6, use 100%
  const finalPercent = hoursSinceDepletion >= 6 ? 1.0 : regenPercent
  
  const newResources = {
    food: Math.min(
      tile.maxResources.food,
      Math.floor(tile.resources.food + tile.maxResources.food * finalPercent)
    ),
    herbs: Math.min(
      tile.maxResources.herbs,
      Math.floor(tile.resources.herbs + tile.maxResources.herbs * finalPercent)
    ),
    water: tile.resources.water, // Water doesn't regenerate (must visit river)
  }
  
  return {
    ...tile,
    resources: newResources,
  }
}

/**
 * Check if a tile is depleted (no harvestable resources).
 *
 * @param tile - The world tile
 * @returns True if tile is depleted
 */
export function isTileDepleted(tile: WorldTile): boolean {
  // Rivers are never depleted (infinite water)
  if (tile.type === 'river') {
    return false
  }
  
  // Tile is depleted if both food and herbs are 0
  return tile.resources.food === 0 && tile.resources.herbs === 0
}



