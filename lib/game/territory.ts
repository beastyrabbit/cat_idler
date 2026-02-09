/**
 * Territory Influence System
 *
 * Pure functions for calculating colony territorial control over world map tiles.
 * Distance-based decay from colony center, bonuses from cat presence/patrols,
 * and territory classification (controlled/contested/wild).
 */

export type TerritoryStatus = "controlled" | "contested" | "wild";

export interface TileInfluenceInput {
  distance: number;
  catCount: number;
  guardCount: number;
  tileType: string;
}

export interface TerritorySummary {
  controlled: number;
  contested: number;
  wild: number;
  totalInfluence: number;
  averageInfluence: number;
}

const PATROL_BONUS = 15;
const GUARD_BONUS = 10;

const ENEMY_PENALTIES: Record<string, number> = {
  enemy_territory: 20,
  enemy_lair: 30,
};

/**
 * Base influence from distance alone.
 * Formula: 100 / (1 + distance), returns 0 for negative distances.
 */
export function getInfluenceDecay(distance: number): number {
  if (distance < 0) return 0;
  return 100 / (1 + distance);
}

/**
 * Compute influence score for a single tile (0-100).
 */
export function calculateTileInfluence(
  distance: number,
  catCount: number,
  guardCount: number,
  tileType: string,
): number {
  const base = getInfluenceDecay(distance);
  const patrolBonus = catCount * PATROL_BONUS;
  const guardBonus = guardCount * GUARD_BONUS;
  const enemyPenalty = ENEMY_PENALTIES[tileType] ?? 0;

  const raw = base + patrolBonus + guardBonus - enemyPenalty;
  return Math.min(100, Math.max(0, Math.round(raw * 100) / 100));
}

/**
 * Classify tile as controlled/contested/wild based on influence score.
 */
export function classifyTerritory(influence: number): TerritoryStatus {
  if (influence >= 60) return "controlled";
  if (influence >= 30) return "contested";
  return "wild";
}

/**
 * Aggregate counts of controlled/contested/wild tiles with total and average influence.
 */
export function calculateTerritorySummary(
  tiles: TileInfluenceInput[],
): TerritorySummary {
  if (tiles.length === 0) {
    return {
      controlled: 0,
      contested: 0,
      wild: 0,
      totalInfluence: 0,
      averageInfluence: 0,
    };
  }

  let controlled = 0;
  let contested = 0;
  let wild = 0;
  let totalInfluence = 0;

  for (const tile of tiles) {
    const influence = calculateTileInfluence(
      tile.distance,
      tile.catCount,
      tile.guardCount,
      tile.tileType,
    );
    totalInfluence += influence;

    const status = classifyTerritory(influence);
    if (status === "controlled") controlled++;
    else if (status === "contested") contested++;
    else wild++;
  }

  return {
    controlled,
    contested,
    wild,
    totalInfluence,
    averageInfluence: totalInfluence / tiles.length,
  };
}
