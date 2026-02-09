/**
 * Predator Threat Level System
 *
 * Pure functions for assessing colony threat from enemy types,
 * surrounding tile danger, and colony defensive capabilities.
 * Produces a threat score (0-100), threat level, and newspaper-ready reports.
 */

import type { EnemyType } from "@/types/game";
import { ENEMY_STATS } from "@/types/game";

// =============================================================================
// Types
// =============================================================================

export type ThreatLevel = "minimal" | "low" | "moderate" | "high" | "critical";

export interface ThreatFactor {
  source: string;
  contribution: number;
}

export interface ThreatAssessment {
  score: number;
  level: ThreatLevel;
  topFactors: ThreatFactor[];
  report: string;
}

// =============================================================================
// Constants
// =============================================================================

const THREAT_THRESHOLDS = [
  { min: 80, level: "critical" as const },
  { min: 60, level: "high" as const },
  { min: 40, level: "moderate" as const },
  { min: 20, level: "low" as const },
] as const;

const MAX_TILE_DANGER = 30;
const MAX_ENEMY_THREAT = 40;
const MAX_DEFENSE = 40;
const MAX_GUARD_CONTRIBUTION = 15;
const MAX_WALL_CONTRIBUTION = 15;
const MAX_STAT_CONTRIBUTION = 10;
const MAX_WALL_LEVEL = 5;
const MAX_ENEMY_TYPES = 5; // fox, hawk, badger, bear, rival_cat

// =============================================================================
// Core Functions
// =============================================================================

/**
 * Map a threat score (0-100) to a ThreatLevel.
 * Clamps out-of-range values.
 */
export function getThreatLevel(score: number): ThreatLevel {
  const clamped = Math.max(0, Math.min(100, score));
  for (const { min, level } of THREAT_THRESHOLDS) {
    if (clamped >= min) return level;
  }
  return "minimal";
}

/**
 * Compute overall threat score from danger and defense components.
 * Result is clamped to 0-100.
 */
export function calculateThreatScore(
  dangerTotal: number,
  defenseTotal: number,
): number {
  return Math.max(0, Math.min(100, dangerTotal - defenseTotal));
}

/**
 * Assess danger contribution from surrounding tile danger levels.
 * Takes an array of danger values (0-100 each), returns 0-30 scaled score.
 */
export function assessTileDanger(tileDangerLevels: number[]): number {
  if (tileDangerLevels.length === 0) return 0;
  const avg =
    tileDangerLevels.reduce((sum, d) => sum + d, 0) / tileDangerLevels.length;
  return Math.round((avg / 100) * MAX_TILE_DANGER);
}

/**
 * Assess threat contribution from nearby enemy types.
 * Considers both frequency (count) and strength (from ENEMY_STATS).
 * Returns 0-40 scaled score.
 */
export function assessEnemyThreat(enemyTypes: EnemyType[]): number {
  if (enemyTypes.length === 0) return 0;

  // Frequency component: how many enemies (scaled by max types)
  const uniqueCount = new Set(enemyTypes).size;
  const frequencyScore =
    (Math.min(uniqueCount, MAX_ENEMY_TYPES) / MAX_ENEMY_TYPES) * 20;

  // Strength component: average strength of all enemy types present
  const maxStrength = Math.max(
    ...Object.values(ENEMY_STATS).map(
      (s) => s.baseClicks + (s.damage[0] + s.damage[1]) / 2,
    ),
  );

  const totalStrength = enemyTypes.reduce((sum, type) => {
    const stats = ENEMY_STATS[type];
    return sum + stats.baseClicks + (stats.damage[0] + stats.damage[1]) / 2;
  }, 0);
  const avgStrength = totalStrength / enemyTypes.length;
  const strengthScore = (avgStrength / maxStrength) * 20;

  return Math.min(MAX_ENEMY_THREAT, Math.round(frequencyScore + strengthScore));
}

/**
 * Assess colony defense strength from guards, walls, and cat stats.
 * Returns 0-40 scaled score (higher = more defended).
 */
export function assessDefenseStrength(
  guards: number,
  wallLevel: number,
  avgDefenseStat: number,
  colonySize: number,
): number {
  if (colonySize === 0) return 0;

  // Guard contribution: ratio of guards to colony size, scaled to 0-15
  const guardRatio = Math.min(1, guards / colonySize);
  const guardScore = guardRatio * MAX_GUARD_CONTRIBUTION;

  // Wall contribution: wall level scaled to 0-15
  const wallScore =
    (Math.min(wallLevel, MAX_WALL_LEVEL) / MAX_WALL_LEVEL) *
    MAX_WALL_CONTRIBUTION;

  // Avg defense stat contribution: scaled to 0-10
  const statScore =
    (Math.min(avgDefenseStat, 100) / 100) * MAX_STAT_CONTRIBUTION;

  return Math.min(MAX_DEFENSE, Math.round(guardScore + wallScore + statScore));
}

/**
 * Generate a newspaper-ready security bulletin from threat score and factors.
 */
export function generateThreatReport(
  score: number,
  topFactors: ThreatFactor[],
): string {
  const level = getThreatLevel(score);

  const levelDescriptions: Record<ThreatLevel, string> = {
    minimal:
      "The colony rests easy tonight. Patrols report no significant threats in the surrounding territory.",
    low: "A few distant dangers stir beyond the borders, but the colony's defenses hold firm.",
    moderate:
      "Scouts report growing activity in nearby territories. Vigilance is advised for all colony members.",
    high: "Warning: Colony defenses are strained. Multiple threats detected in surrounding areas. Reinforce walls and assign more guards immediately.",
    critical:
      "EMERGENCY: Overwhelming threat detected! The colony faces imminent danger from all sides. All cats must prepare for defense.",
  };

  let report = `SECURITY BULLETIN — Threat Level: ${level.toUpperCase()}\n\n`;
  report += levelDescriptions[level];

  if (topFactors.length > 0) {
    report += "\n\nKey concerns:";
    for (const factor of topFactors) {
      report += `\n- ${factor.source} (threat contribution: ${factor.contribution})`;
    }
  }

  return report;
}
