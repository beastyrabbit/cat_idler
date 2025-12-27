/**
 * Task Assignment System
 *
 * Pure functions for task assignment logic.
 * See TASKS.md: LOGIC-006
 */

import type { Cat, TaskType, LifeStage } from '@/types/game'
import { TASK_TO_SKILL, LEADER_QUALITY } from '@/types/game'
import { getLifeStage, getAgeInHours, canPerformTask } from './age'

/**
 * Find the optimal cat for a task based on skills and age.
 *
 * @param cats - Available cats
 * @param taskType - Type of task
 * @returns Optimal cat or null if none available
 */
export function getOptimalCatForTask(cats: Cat[], taskType: TaskType): Cat | null {
  if (cats.length === 0) {
    return null
  }

  const currentTime = Date.now()
  const isDangerousTask = taskType === 'hunt' || taskType === 'patrol' || taskType === 'guard'
  const requiresOutside = taskType === 'hunt' || taskType === 'gather_herbs' || taskType === 'fetch_water' || taskType === 'explore' || taskType === 'patrol'

  // Filter cats that can perform the task
  const eligibleCats = cats.filter(cat => {
    const age = getAgeInHours(cat.birthTime, currentTime)
    const lifeStage = getLifeStage(age)
    return canPerformTask(lifeStage, requiresOutside, isDangerousTask)
  })

  if (eligibleCats.length === 0) {
    return null
  }

  // Get the skill relevant to this task
  const relevantSkill = TASK_TO_SKILL[taskType]

  // Find cat with highest relevant skill
  let bestCat = eligibleCats[0]
  let bestSkill = eligibleCats[0].stats[relevantSkill]

  for (const cat of eligibleCats) {
    const skill = cat.stats[relevantSkill]
    if (skill > bestSkill) {
      bestSkill = skill
      bestCat = cat
    }
  }

  return bestCat
}

/**
 * Get assignment time based on leader's leadership stat.
 *
 * @param leadershipStat - Leader's leadership stat (0-100)
 * @returns Assignment time in seconds
 */
export function getAssignmentTime(leadershipStat: number): number {
  if (leadershipStat <= LEADER_QUALITY.bad.max) {
    return LEADER_QUALITY.bad.time
  } else if (leadershipStat <= LEADER_QUALITY.okay.max) {
    return LEADER_QUALITY.okay.time
  } else if (leadershipStat <= LEADER_QUALITY.good.max) {
    return LEADER_QUALITY.good.time
  } else {
    return LEADER_QUALITY.great.time
  }
}

/**
 * Get assigned cat considering leader quality (may not be optimal).
 *
 * @param cats - Available cats
 * @param taskType - Type of task
 * @param leadershipStat - Leader's leadership stat
 * @returns Assigned cat and whether it's optimal
 */
export function getAssignedCat(
  cats: Cat[],
  taskType: TaskType,
  leadershipStat: number
): { cat: Cat | null; isOptimal: boolean } {
  const optimalCat = getOptimalCatForTask(cats, taskType)
  
  if (!optimalCat) {
    return { cat: null, isOptimal: false }
  }

  // Determine wrong assignment chance based on leadership
  let wrongChance = 0
  if (leadershipStat <= LEADER_QUALITY.bad.max) {
    wrongChance = LEADER_QUALITY.bad.wrongChance
  } else if (leadershipStat <= LEADER_QUALITY.okay.max) {
    wrongChance = LEADER_QUALITY.okay.wrongChance
  } else if (leadershipStat <= LEADER_QUALITY.good.max) {
    wrongChance = LEADER_QUALITY.good.wrongChance
  } else {
    wrongChance = LEADER_QUALITY.great.wrongChance
  }

  // Check if leader makes wrong assignment
  const shouldAssignWrong = Math.random() < wrongChance

  if (!shouldAssignWrong) {
    return { cat: optimalCat, isOptimal: true }
  }

  // Pick a random wrong cat
  const otherCats = cats.filter(cat => cat._id !== optimalCat._id)
  if (otherCats.length === 0) {
    // If no other cats, use optimal
    return { cat: optimalCat, isOptimal: true }
  }

  const randomIndex = Math.floor(Math.random() * otherCats.length)
  return { cat: otherCats[randomIndex], isOptimal: false }
}



