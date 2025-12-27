/**
 * Cat AI Autonomous Behavior
 *
 * Pure functions for determining what cats should do autonomously.
 * See TASKS.md: LOGIC-008
 */

import type { Cat, Resources, BuildingType, AutonomousAction, Position } from '@/types/game'
import { hasNeedsCritical } from './needs'

/**
 * Determine what a cat should do autonomously based on needs.
 *
 * Rules:
 * 1. If hunger < 30 AND food available → eat
 * 2. If thirst < 40 AND water available → drink
 * 3. If rest < 20 → sleep (in beds if available)
 * 4. If any need < 15 AND on world map → return to colony
 * 5. Otherwise → null (continue current task)
 *
 * @param cat - The cat
 * @param colonyResources - Colony resources
 * @param colonyHasBuilding - Function to check if colony has a building type
 * @returns Autonomous action or null
 */
export function getAutonomousAction(
  cat: Cat,
  colonyResources: Resources,
  colonyHasBuilding: (type: BuildingType) => boolean
): AutonomousAction | null {
  // Priority 1: Critical needs while on world map - return to colony
  if (cat.position.map === 'world' && hasNeedsCritical(cat.needs, 15)) {
    return { type: 'return_to_colony' }
  }

  // Priority 2: Hunger (most critical)
  if (cat.needs.hunger < 30 && colonyResources.food > 0) {
    return { type: 'eat' }
  }

  // Priority 3: Thirst
  if (cat.needs.thirst < 40 && colonyResources.water > 0) {
    return { type: 'drink' }
  }

  // Priority 4: Rest
  if (cat.needs.rest < 20) {
    // Find a sleeping position (den or beds)
    const sleepPosition: Position = {
      map: 'colony',
      x: 1,
      y: 1, // Default to center of colony
    }
    return { type: 'sleep', position: sleepPosition }
  }

  // No urgent needs
  return null
}



