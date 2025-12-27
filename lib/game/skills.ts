/**
 * Skill Learning System
 *
 * Pure functions for calculating skill XP gain.
 * See TASKS.md: LOGIC-005
 */

import type { TaskType, LifeStage } from '@/types/game'
import { TASK_TO_SKILL } from '@/types/game'
import { getAgeSkillModifier } from './age'

/**
 * Calculate skill XP gain from completing a task.
 *
 * Rules:
 * - Base XP: 1-3 depending on task difficulty
 * - Success bonus: +50% if task succeeded well
 * - Age modifier: Young = 1.5x, Adult = 1.0x, Elder = 0.5x, Kitten = 0x
 * - Skill cap: 100
 * - Returns the NEW skill value (not the gain amount)
 *
 * @param currentSkill - Current skill level (0-100)
 * @param taskType - Type of task completed
 * @param taskSuccess - Whether task succeeded well
 * @param lifeStage - Cat's life stage
 * @returns New skill level (capped at 100)
 */
export function calculateSkillGain(
  currentSkill: number,
  taskType: TaskType,
  taskSuccess: boolean,
  lifeStage: LifeStage
): number {
  // Kittens cannot learn
  if (lifeStage === 'kitten') {
    return currentSkill
  }

  // Base XP: 1-3 depending on task
  const baseXP = getBaseXPForTask(taskType)
  
  // Success bonus: +50% if succeeded
  const successMultiplier = taskSuccess ? 1.5 : 1.0
  
  // Age modifier
  const ageModifier = getAgeSkillModifier(lifeStage)
  
  // Calculate gain
  const gain = baseXP * successMultiplier * ageModifier
  
  // Add to current skill and cap at 100
  return Math.min(100, currentSkill + gain)
}

/**
 * Get base XP for a task type.
 * 
 * @param taskType - Type of task
 * @returns Base XP (1-3)
 */
function getBaseXPForTask(taskType: TaskType): number {
  // More difficult/dangerous tasks give more XP
  switch (taskType) {
    case 'hunt':
    case 'patrol':
    case 'guard':
      return 3 // Dangerous tasks
    case 'build':
    case 'heal':
    case 'teach':
      return 2 // Moderate tasks
    case 'gather_herbs':
    case 'fetch_water':
    case 'explore':
      return 2 // Moderate tasks
    case 'clean':
    case 'kitsit':
    case 'rest':
      return 1 // Easy tasks
    default:
      return 1 // Fallback
  }
}



