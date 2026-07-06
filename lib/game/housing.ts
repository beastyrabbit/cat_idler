/**
 * Housing & village growth rules (pure, tunable).
 *
 * Dens are the colony's houses: each completed den level shelters 2 cats,
 * with the shrine offering base shelter for 4. When the population
 * outgrows shelter, the leader plans more construction — this is the
 * village-growth feedback loop.
 */

export interface HousingBuilding {
	type: string;
	level: number;
	constructionProgress: number;
}

/** Cats sheltered by the shrine itself. */
const SHRINE_CAPACITY = 4;

/** Cats sheltered per den level. */
const DEN_CAPACITY_PER_LEVEL = 2;

/** Leader plans a new den when pressure reaches this. */
export const HOUSE_PRESSURE_THRESHOLD = 0.8;

/** Completed non-shrine buildings needed for each village level past 1. */
const VILLAGE_LEVEL_THRESHOLDS = [6, 12, 20, 30];

function isComplete(building: HousingBuilding): boolean {
	return building.constructionProgress >= 100;
}

export function housingCapacity(
	buildings: HousingBuilding[],
	/**
	 * Extra cats each completed den shelters, granted by upgrade-tree nodes
	 * (`housingPerDen`, e.g. Den Insulation). Flat per den, not per level.
	 */
	extraPerDen = 0,
): number {
	let capacity = 0;
	for (const building of buildings) {
		if (!isComplete(building)) {
			continue;
		}
		if (building.type === "shrine") {
			capacity += SHRINE_CAPACITY;
		} else if (building.type === "den") {
			capacity +=
				DEN_CAPACITY_PER_LEVEL * Math.max(1, building.level) +
				Math.max(0, extraPerDen);
		}
	}
	return capacity;
}

export function housingPressure(population: number, capacity: number): number {
	if (population <= 0) {
		return 0;
	}
	if (capacity <= 0) {
		return Number.POSITIVE_INFINITY;
	}
	return population / capacity;
}

export function shouldQueueHouse(pressure: number): boolean {
	return pressure >= HOUSE_PRESSURE_THRESHOLD;
}

/**
 * Village tier from completed non-shrine buildings. Later phases gate
 * unlocks (workshops, fields) on this.
 */
export function villageLevel(buildings: HousingBuilding[]): number {
	const completed = buildings.filter(
		(building) => building.type !== "shrine" && isComplete(building),
	).length;

	let level = 1;
	for (const threshold of VILLAGE_LEVEL_THRESHOLDS) {
		if (completed >= threshold) {
			level += 1;
		}
	}
	return level;
}
