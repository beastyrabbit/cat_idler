/**
 * Per-resource storage capacity (pure).
 *
 * The colony stores each resource against its own cap instead of a single
 * shared "food" number. Dry goods (food, herbs, materials, refined) live in
 * the granary storehouses (`food_storage`); water is held in water bowls.
 * Caps are derived from the finished buildings so the server and the HUD
 * always agree on how full each store is.
 */

export type ResourceKey = "food" | "water" | "herbs" | "materials" | "refined";

export interface StorageCapacities {
	food: number;
	water: number;
	herbs: number;
	materials: number;
	refined: number;
}

/** Minimal building shape the capacity math needs. */
export interface StorageBuilding {
	type: string;
	level?: number;
	constructionProgress: number;
}

/** Base capacity every settlement starts with, before any storehouses. */
export const BASE_CAPACITY: StorageCapacities = {
	food: 200,
	water: 200,
	herbs: 100,
	materials: 100,
	refined: 100,
};

/** Dry-goods a single finished granary (`food_storage`) adds per level. */
export const GRANARY_BONUS = {
	food: 400,
	herbs: 100,
	materials: 100,
	refined: 50,
} as const;

/** Extra water a single finished water bowl holds per level. */
export const WATER_BOWL_BONUS = 200;

function isFinished(building: StorageBuilding): boolean {
	return building.constructionProgress >= 100;
}

function levelOf(building: StorageBuilding): number {
	return Math.max(1, building.level ?? 1);
}

/** Per-resource storage caps for a colony's current buildings. */
export function storageCapacities(
	buildings: readonly StorageBuilding[],
): StorageCapacities {
	const caps: StorageCapacities = { ...BASE_CAPACITY };
	for (const building of buildings) {
		if (!isFinished(building)) {
			continue;
		}
		const level = levelOf(building);
		if (building.type === "food_storage") {
			caps.food += GRANARY_BONUS.food * level;
			caps.herbs += GRANARY_BONUS.herbs * level;
			caps.materials += GRANARY_BONUS.materials * level;
			caps.refined += GRANARY_BONUS.refined * level;
		} else if (building.type === "water_bowl") {
			caps.water += WATER_BOWL_BONUS * level;
		}
	}
	return caps;
}

/**
 * How many granary storehouses the leader is allowed to raise. Scales with
 * population so a small colony never carpets its clearing in hay bales, and
 * a growing one still keeps enough dry storage.
 */
export function storehouseCap(population: number): number {
	return Math.max(1, Math.floor(population / 6));
}

/** Finished granaries currently standing. */
export function countStorehouses(
	buildings: readonly StorageBuilding[],
): number {
	return buildings.filter((b) => b.type === "food_storage" && isFinished(b))
		.length;
}
