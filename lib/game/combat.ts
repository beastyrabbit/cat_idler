/**
 * Combat Resolution System
 *
 * Pure functions for combat calculations.
 * See TASKS.md: LOGIC-007
 */

import type { Building, CombatResult } from "@/types/game";

/**
 * Calculate combat outcome between cat and enemy.
 *
 * Rules:
 * - Cat roll: attack + defense + random(1-20)
 * - Enemy roll: strength + random(1-20)
 * - Cat wins if cat roll > enemy roll
 * - Damage on loss: 30-70 based on how much they lost by
 *
 * @param catAttack - Cat's attack stat
 * @param catDefense - Cat's defense stat
 * @param enemyStrength - Enemy's strength
 * @returns Combat result with win status and damage
 */
export function calculateCombatResult(
  catAttack: number,
  catDefense: number,
  enemyStrength: number,
): CombatResult {
  // Cat roll: attack + defense + random(1-20)
  const catRoll = catAttack + catDefense + Math.floor(Math.random() * 20) + 1;

  // Enemy roll: strength + random(1-20)
  const enemyRoll = enemyStrength + Math.floor(Math.random() * 20) + 1;

  const won = catRoll > enemyRoll;

  if (won) {
    return { won: true, damage: 0 };
  }

  // Calculate damage based on how much cat lost by
  const lossMargin = enemyRoll - catRoll;
  // Damage ranges from 30-70, scaled by loss margin (max margin is ~100)
  const damage = Math.min(70, Math.max(30, 30 + lossMargin * 0.4));

  return { won: false, damage: Math.round(damage) };
}

/**
 * Calculate clicks needed to help cat win encounter.
 *
 * Rules:
 * - Base clicks reduced by colony defense (up to 50%)
 * - Base clicks reduced by cat vision (up to 50%)
 * - Both modifiers stack multiplicatively
 *
 * @param baseClicks - Base clicks needed for this enemy
 * @param colonyDefense - Colony defense bonus (0-100)
 * @param catVision - Cat's vision stat (0-100)
 * @returns Number of clicks needed
 */
export function getClicksNeeded(
  baseClicks: number,
  colonyDefense: number,
  catVision: number,
): number {
  // Defense reduces clicks: 50% defense = 50% reduction, 100% defense = 50% reduction (capped)
  // Formula: multiplier = 1 - (defense / 100) * 0.5
  // At 50: 1 - 0.25 = 0.75... wait that's not right
  // Actually: multiplier = 0.5 + (defense / 100) * 0.5
  // At 0: 0.5 + 0 = 0.5 (50% of base)
  // At 50: 0.5 + 0.25 = 0.75 (75% of base)
  // At 100: 0.5 + 0.5 = 1.0 (100% of base)
  // That's backwards...

  // Let me think: 50 defense should give 50% reduction = 0.5 multiplier
  // So: multiplier = 1 - (defense / 100) * 0.5
  // At 0: 1 - 0 = 1.0 (100% of base)
  // At 50: 1 - 0.25 = 0.75 (75% of base) - WRONG, should be 0.5
  // At 100: 1 - 0.5 = 0.5 (50% of base) - correct

  // Actually: multiplier = 1 - (defense / 100) * 0.5
  // But test expects 50 defense = 0.5 multiplier
  // So: 0.5 = 1 - (50 / 100) * x
  // 0.5 = 1 - 0.5x
  // 0.5x = 0.5
  // x = 1
  // So: multiplier = 1 - (defense / 100) * 1
  // At 50: 1 - 0.5 = 0.5 ✓
  // At 100: 1 - 1 = 0 (but should be 0.5 max)

  // Actually the test says 50 defense = 50% reduction
  // So: multiplier = 1 - (defense / 100)
  // At 50: 0.5 ✓
  // At 100: 0 (but max should be 0.5)

  // Let me use: multiplier = Math.max(0.5, 1 - defense / 100)
  // At 0: 1.0 ✓
  // At 50: 0.5 ✓
  // At 100: 0.5 ✓

  const defenseMultiplier = Math.max(0.5, 1 - colonyDefense / 100);
  const visionMultiplier = Math.max(0.5, 1 - catVision / 100);

  // Stack both modifiers
  const totalMultiplier = defenseMultiplier * visionMultiplier;

  const clicks = baseClicks * totalMultiplier;

  // Minimum 1 click needed
  return Math.max(1, Math.round(clicks));
}

/**
 * Calculate colony defense score from wall buildings.
 *
 * Each wall level adds 20 defense points. Incomplete walls contribute
 * proportionally based on constructionProgress (0-100%).
 * Total defense is capped at 100.
 *
 * @param walls - Array of wall buildings (type, level, constructionProgress)
 * @returns Defense score (0-100)
 */
export function calculateColonyDefense(
  walls: Pick<Building, "type" | "level" | "constructionProgress">[],
): number {
  const raw = walls.reduce((sum, wall) => {
    return sum + wall.level * 20 * (wall.constructionProgress / 100);
  }, 0);
  return Math.min(100, Math.round(raw));
}
