/**
 * Exploration Reports
 *
 * Pure functions for generating newspaper-style field dispatches
 * from world tile exploration data. Leverages BiomeType and
 * OverlayFeature from biomes.ts.
 */

import {
  type BiomeType,
  type OverlayFeature,
  BIOME_PROPERTIES,
} from "./biomes";

// =============================================================================
// Types
// =============================================================================

export type ExplorationRating =
  | "remarkable"
  | "promising"
  | "routine"
  | "disappointing"
  | "perilous";

export interface ExploreTileData {
  biomeType: BiomeType;
  overlayFeature: OverlayFeature;
  dangerLevel: number;
  resources: { food: number; herbs: number };
}

export interface FieldDispatch {
  headline: string;
  terrainDescription: string;
  dangerAssessment: string;
  resourceDiscovery: string;
  scoutRecommendation: string;
  rating: ExplorationRating;
  score: number;
}

// =============================================================================
// Constants
// =============================================================================

const TERRAIN_DESCRIPTIONS: Record<BiomeType, string> = {
  oak_forest:
    "Towering oaks form a dense canopy, their gnarled roots weaving across the forest floor.",
  pine_forest:
    "Tall pines stand in silent rows, their needles carpeting the ground in fragrant layers.",
  jungle:
    "A riot of green erupts in every direction — vines, ferns, and exotic blooms crowd for sunlight.",
  dead_forest:
    "Skeletal trees claw at a grey sky, their bark peeling in long, ghostly strips.",
  mountains:
    "Jagged peaks pierce the clouds, with treacherous scree slopes and howling winds.",
  swamp:
    "Murky waters pool between twisted roots, and the air hangs thick with moisture.",
  desert:
    "Endless dunes stretch to the horizon, shimmering in the relentless heat.",
  tundra:
    "A frozen expanse of permafrost and sparse lichen, where the wind never rests.",
  meadow:
    "Rolling grasslands dotted with wildflowers sway gently in the breeze.",
  cave_entrance:
    "A yawning dark opening in the rock face, cold air seeping from its depths.",
  enemy_lair:
    "The stench of predators hangs heavy here — claw marks score the earth and bones litter the ground.",
};

const OVERLAY_DESCRIPTIONS: Record<Exclude<OverlayFeature, null>, string> = {
  river:
    " A river cuts through the landscape, its waters running swift and clear.",
  ancient_road:
    " An ancient road, cracked but still passable, winds through the terrain.",
  game_trail: " A well-worn game trail reveals the passage of many creatures.",
  trade_route:
    " A trade route marked by old cairns suggests this land was once well-traveled.",
};

const DANGER_ASSESSMENTS: Record<string, string> = {
  low: "The area appears relatively safe. Scouts report minimal predator activity and no hostile territory markers.",
  moderate:
    "Caution is advised. Signs of predator activity are present, and the terrain offers limited defensive positions.",
  high: "Significant danger detected. Multiple predator trails converge here, and territorial markings are fresh.",
  extreme:
    "Extreme peril! This area is deep in hostile territory. Only the most experienced scouts should venture here.",
};

const RESOURCE_DESCRIPTORS: Record<string, string> = {
  none: "barren",
  scarce: "meagre",
  moderate: "decent",
  abundant: "plentiful",
  rich: "bountiful",
};

const SCOUT_RECOMMENDATIONS: Record<
  ExplorationRating,
  Record<string, string>
> = {
  remarkable: {
    low: "Dispatch a foraging party immediately — this area is a goldmine of resources with minimal risk.",
    high: "Rich resources detected, but send an armed escort. The rewards justify the risk.",
  },
  promising: {
    low: "A solid prospect for expansion. Recommend sending a small scouting team to establish a waypoint.",
    high: "Promising finds, but the danger gives pause. Consider sending a larger, well-armed party.",
  },
  routine: {
    low: "Standard territory. Worth periodic patrols but no urgent action needed.",
    high: "Unremarkable resources in a dangerous area. Best avoided unless strategically important.",
  },
  disappointing: {
    low: "Slim pickings here. Resources barely justify the travel time.",
    high: "Poor resources in hostile territory. Strongly recommend avoiding this area.",
  },
  perilous: {
    low: "Almost nothing of value and still dangerous. Mark on the map and move on.",
    high: "A death trap with nothing to show for it. Under no circumstances should cats be sent here.",
  },
};

// =============================================================================
// Functions
// =============================================================================

/**
 * Map a numeric score (0-100) to an exploration rating.
 */
export function getExplorationRating(score: number): ExplorationRating {
  if (score >= 80) return "remarkable";
  if (score >= 60) return "promising";
  if (score >= 40) return "routine";
  if (score >= 20) return "disappointing";
  return "perilous";
}

/**
 * Calculate an overall exploration score from tile data and scout vision.
 * Score is clamped to 0-100.
 *
 * Positive factors:
 *   - Resource richness: food + herbs relative to biome max (0-40)
 *   - Overlay feature bonus: ancient_road/trade_route +10, game_trail +5 (0-10)
 *   - Scout vision stat (0-10)
 *
 * Negative factors:
 *   - Danger level scaled 0-40 (inverse)
 *   - Slow travel speed reduces score (0-10)
 */
export function calculateExplorationScore(
  tile: ExploreTileData,
  scoutVision: number,
): number {
  const biome = BIOME_PROPERTIES[tile.biomeType];

  // Resource richness (0-40)
  const maxFood = Math.max(biome.maxResources.food, 1);
  const maxHerbs = Math.max(biome.maxResources.herbs, 1);
  const foodRatio = Math.min(tile.resources.food / maxFood, 1);
  const herbRatio = Math.min(tile.resources.herbs / maxHerbs, 1);
  const resourceScore = ((foodRatio + herbRatio) / 2) * 40;

  // Danger penalty (0-40, inverted)
  const dangerPenalty = (Math.min(tile.dangerLevel, 100) / 100) * 40;

  // Overlay feature bonus (0-10)
  let overlayBonus = 0;
  if (
    tile.overlayFeature === "ancient_road" ||
    tile.overlayFeature === "trade_route"
  ) {
    overlayBonus = 10;
  } else if (tile.overlayFeature === "game_trail") {
    overlayBonus = 5;
  }

  // Scout vision bonus (0-10)
  const visionBonus = Math.min(scoutVision, 10);

  // Travel speed factor (0-10 penalty for slow travel)
  const travelPenalty = Math.max(0, (1 - biome.travelSpeed) * 10);

  const raw =
    resourceScore - dangerPenalty + overlayBonus + visionBonus - travelPenalty;
  return Math.max(0, Math.min(100, Math.round(raw)));
}

/**
 * Generate a flavorful terrain description for the dispatch.
 */
export function getTerrainDescription(
  biomeType: BiomeType,
  overlayFeature: OverlayFeature,
): string {
  const base = TERRAIN_DESCRIPTIONS[biomeType];
  if (overlayFeature) {
    return base + OVERLAY_DESCRIPTIONS[overlayFeature];
  }
  return base;
}

/**
 * Generate a danger assessment for the newspaper.
 */
export function getDangerAssessment(dangerLevel: number): string {
  if (dangerLevel >= 75) return DANGER_ASSESSMENTS.extreme;
  if (dangerLevel >= 50) return DANGER_ASSESSMENTS.high;
  if (dangerLevel >= 25) return DANGER_ASSESSMENTS.moderate;
  return DANGER_ASSESSMENTS.low;
}

/**
 * Generate a resource discovery summary.
 */
export function getResourceDiscovery(
  resources: { food: number; herbs: number },
  biomeType: BiomeType,
): string {
  const biome = BIOME_PROPERTIES[biomeType];
  const biomeName = biome.name;

  const foodDesc = getResourceDescriptor(resources.food);
  const herbDesc = getResourceDescriptor(resources.herbs);

  if (resources.food === 0 && resources.herbs === 0) {
    return `The ${biomeName} yielded no usable resources. The land appears ${RESOURCE_DESCRIPTORS.none}.`;
  }

  const parts: string[] = [];
  if (resources.food > 0) {
    parts.push(
      `${RESOURCE_DESCRIPTORS[foodDesc]} food supplies (${resources.food} units)`,
    );
  }
  if (resources.herbs > 0) {
    parts.push(
      `${RESOURCE_DESCRIPTORS[herbDesc]} herb deposits (${resources.herbs} units)`,
    );
  }

  return `Scouts discovered ${parts.join(" and ")} in the ${biomeName}.`;
}

/**
 * Generate a scout recommendation based on rating and danger.
 */
export function getScoutRecommendation(
  rating: ExplorationRating,
  dangerLevel: number,
): string {
  const dangerKey = dangerLevel >= 50 ? "high" : "low";
  return SCOUT_RECOMMENDATIONS[rating][dangerKey];
}

/**
 * Assemble a complete field dispatch for the newspaper.
 */
export function formatFieldDispatch(
  tileData: ExploreTileData,
  scoutName: string,
  scoutVision: number,
): FieldDispatch {
  const biomeName = BIOME_PROPERTIES[tileData.biomeType].name;
  const score = calculateExplorationScore(tileData, scoutVision);
  const rating = getExplorationRating(score);

  return {
    headline: `${scoutName} Reports from the ${biomeName}: Expedition Rated "${rating.charAt(0).toUpperCase() + rating.slice(1)}"`,
    terrainDescription: getTerrainDescription(
      tileData.biomeType,
      tileData.overlayFeature,
    ),
    dangerAssessment: getDangerAssessment(tileData.dangerLevel),
    resourceDiscovery: getResourceDiscovery(
      tileData.resources,
      tileData.biomeType,
    ),
    scoutRecommendation: getScoutRecommendation(rating, tileData.dangerLevel),
    rating,
    score,
  };
}

// =============================================================================
// Helpers (not exported)
// =============================================================================

function getResourceDescriptor(amount: number): string {
  if (amount === 0) return "none";
  if (amount <= 5) return "scarce";
  if (amount <= 15) return "moderate";
  if (amount <= 30) return "abundant";
  return "rich";
}
