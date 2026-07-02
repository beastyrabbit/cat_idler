/**
 * Leader "brain" — pure decision logic for the colony's autonomous leader.
 *
 * The worker tick feeds a snapshot of the colony (population, idle cats,
 * stores, housing, jobs in flight) and gets back a prioritized list of
 * decisions. This module has NO knowledge of the DB, RNG, or policy rolls:
 * the caller keeps the seeded policy-reliability gate at the point where it
 * executes each decision.
 *
 * Design goals — fixing the old scattered logic that oscillated (queued a
 * wave of hunts one tick, cancelled them the next, over-built houses, and
 * mostly issued a single job type):
 *  - Employ roughly half the colony across job types, not everyone on hunts.
 *  - Hysteresis on hunts: dispatch below 90% food, hold across the 90-110%
 *    band, and only cancel above 110%. This kills the "queue 6, cancel 6"
 *    loop — food has to swing a full band before a decision reverses.
 *  - At most one den plan and one storehouse in flight at a time.
 */

/** Fraction of the colony the leader tries to keep employed at once. */
export const EMPLOYMENT_TARGET_RATIO = 0.5;
/** At or above this food/capacity ratio, no new hunts are dispatched. */
export const HUNT_HOLD_RATIO = 0.9;
/** Above this food/capacity ratio, hunts already out are called off. */
export const HUNT_CANCEL_RATIO = 1.1;
/** Above this food/capacity ratio, the leader commissions a storehouse. */
export const STORAGE_RATIO = 0.9;
/** Housing pressure (pop / (capacity + committed)) that triggers a den. */
export const DEN_PRESSURE_THRESHOLD = 0.8;
/** Below this materials/capacity ratio, the leader opens a quarry. */
export const QUARRY_LOW_RATIO = 0.4;
/** At or above this materials/capacity ratio, no new quarry is opened. */
export const QUARRY_HOLD_RATIO = 0.6;
/** Quarry expeditions the leader keeps running while materials are low. */
export const QUARRY_TARGET = 1;
/** Below this water/capacity ratio, the leader sends cats to fetch water. */
export const WATER_LOW_RATIO = 0.5;
/** At or above this water/capacity ratio, no new water fetch is dispatched. */
export const WATER_HOLD_RATIO = 0.85;
/** Water-fetch expeditions kept running while the reservoir is low. */
export const WATER_FETCH_TARGET = 2;
/** Explore jobs the leader keeps running while a frontier remains. */
export const SCOUT_TARGET = 2;
/** Stores must exceed this fraction of capacity before food is tithed. */
export const TITHE_FOOD_RATIO = 0.6;
/** Food spent per blessing when tithing surplus. */
export const TITHE_FOOD_AMOUNT = 20;
/** Refined goods spent per blessing when tithing. */
export const TITHE_REFINED_AMOUNT = 5;

export interface LeaderSnapshot {
	/** Living cats in the colony (raw head count, incl. kittens). */
	population: number;
	/**
	 * Stage-weighted count of work-capable cats — kittens count for nothing,
	 * elders partially. The employment target is a fraction of this, not of the
	 * raw head count, so a nursery full of kittens doesn't inflate the budget.
	 * Falls back to {@link population} when omitted.
	 */
	workforce?: number;
	/** Cats free to take on a new job right now. */
	idleCats: number;
	/** Cats currently occupied by any job or workplace. */
	employedCats: number;
	resources: { food: number; refined: number };
	foodCapacity: number;
	/** Materials in store and the cap they're clamped to. */
	materials: number;
	materialsCapacity: number;
	/** Water in store and the cap it's clamped to. */
	water: number;
	waterCapacity: number;
	housing: { capacity: number; committed: number };
	/** hunt_expedition jobs in flight (active or queued). */
	activeHunts: number;
	/** quarry jobs in flight (active or queued). */
	activeQuarries: number;
	/** explore jobs in flight (active or queued). */
	activeScouts: number;
	/** fetch_water jobs in flight (active or queued). */
	activeWaterFetchers: number;
	/** An explored mountains/cave tile exists to quarry. */
	hasQuarrySite: boolean;
	/** An explored water tile the colony can draw from. */
	hasWaterSite: boolean;
	/** An unexplored tile still sits on the reachable frontier. */
	hasFrontier: boolean;
	/** Den plans in flight: leader_plan_house or a build_house den. */
	denPlansInFlight: number;
	/** Storehouse builds in flight: build_house with a food_storage target. */
	storagePlansInFlight: number;
	/** Finished granary storehouses currently standing. */
	storehouseCount: number;
	/** Cap on total storehouses (scales with population). */
	storehouseCap: number;
	/** Completed workshops that have no assigned worker. */
	workshopsNeedingWorkers: number;
}

export type LeaderDecision =
	| { kind: "hunt"; count: number }
	| { kind: "cancel_hunts" }
	| { kind: "fetch_water"; count: number }
	| { kind: "quarry"; count: number }
	| { kind: "scout"; count: number }
	| { kind: "build_den" }
	| { kind: "build_storage" }
	| { kind: "assign_workshop"; count: number }
	| { kind: "tithe"; food: number; refined: number; blessings: number };

/** Food as a fraction of storage capacity; unbounded when capacity is 0. */
function foodRatio(food: number, capacity: number): number {
	if (capacity <= 0) {
		return food > 0 ? Number.POSITIVE_INFINITY : 0;
	}
	return food / capacity;
}

/**
 * How many hunters the leader wants out in total, scaling with the food
 * deficit below the hold band and capped at half the colony. Zero once
 * stores reach 90% of capacity.
 */
export function targetHuntSlots(snapshot: LeaderSnapshot): number {
	const ratio = foodRatio(snapshot.resources.food, snapshot.foodCapacity);
	if (ratio >= HUNT_HOLD_RATIO) {
		return 0;
	}
	const deficit = (HUNT_HOLD_RATIO - ratio) / HUNT_HOLD_RATIO;
	const maxEmployed = Math.floor(
		(snapshot.workforce ?? snapshot.population) * EMPLOYMENT_TARGET_RATIO,
	);
	return Math.ceil(deficit * maxEmployed);
}

/**
 * Turn a colony snapshot into a prioritized list of leader decisions. The
 * order is deliberate — hunting/cancellation first, then storage, den,
 * workshop staffing, and finally tithing — so the caller consumes seeded
 * policy rolls in a stable sequence.
 */
export function planLeaderActions(snapshot: LeaderSnapshot): LeaderDecision[] {
	const decisions: LeaderDecision[] = [];
	const ratio = foodRatio(snapshot.resources.food, snapshot.foodCapacity);

	const maxEmployed = Math.floor(
		(snapshot.workforce ?? snapshot.population) * EMPLOYMENT_TARGET_RATIO,
	);
	const employmentRoom = Math.max(0, maxEmployed - snapshot.employedCats);

	// --- Hunting, with hysteresis across the 90-110% food band ---------
	let huntsPlanned = 0;
	if (ratio > HUNT_CANCEL_RATIO && snapshot.activeHunts > 0) {
		decisions.push({ kind: "cancel_hunts" });
	} else {
		const gap = targetHuntSlots(snapshot) - snapshot.activeHunts;
		huntsPlanned = Math.max(
			0,
			Math.min(gap, snapshot.idleCats, employmentRoom),
		);
		if (huntsPlanned > 0) {
			decisions.push({ kind: "hunt", count: huntsPlanned });
		}
	}

	// --- Water: the colony draws its own water from the nearest known
	// water tile. Hysteresis mirrors the hunts — dispatch below 50% of the
	// reservoir cap, hold through the 50-85% band, and stop above 85%. Water
	// is life-or-death, so it's planned right after hunting and ahead of the
	// slower material/scout work.
	let waterFetchPlanned = 0;
	if (snapshot.hasWaterSite) {
		const waterRatio = foodRatio(snapshot.water, snapshot.waterCapacity);
		let wantFetchers = snapshot.activeWaterFetchers;
		if (waterRatio < WATER_LOW_RATIO) {
			wantFetchers = WATER_FETCH_TARGET;
		} else if (waterRatio >= WATER_HOLD_RATIO) {
			wantFetchers = 0;
		}
		const idleForWater = Math.max(0, snapshot.idleCats - huntsPlanned);
		const roomForWater = Math.max(0, employmentRoom - huntsPlanned);
		waterFetchPlanned = Math.max(
			0,
			Math.min(
				wantFetchers - snapshot.activeWaterFetchers,
				idleForWater,
				roomForWater,
			),
		);
		if (waterFetchPlanned > 0) {
			decisions.push({ kind: "fetch_water", count: waterFetchPlanned });
		}
	}

	// --- Quarry: keep one expedition running while materials run low.
	// Hysteresis mirrors the hunts — dispatch below 40% of the materials
	// cap, hold through the 40-60% band, and open nothing above 60%.
	let quarriesPlanned = 0;
	if (snapshot.hasQuarrySite) {
		const materialsRatio = foodRatio(
			snapshot.materials,
			snapshot.materialsCapacity,
		);
		let wantQuarries = snapshot.activeQuarries;
		if (materialsRatio < QUARRY_LOW_RATIO) {
			wantQuarries = QUARRY_TARGET;
		} else if (materialsRatio >= QUARRY_HOLD_RATIO) {
			wantQuarries = 0;
		}
		const idleForQuarry = Math.max(
			0,
			snapshot.idleCats - huntsPlanned - waterFetchPlanned,
		);
		const roomForQuarry = Math.max(
			0,
			employmentRoom - huntsPlanned - waterFetchPlanned,
		);
		quarriesPlanned = Math.max(
			0,
			Math.min(
				wantQuarries - snapshot.activeQuarries,
				idleForQuarry,
				roomForQuarry,
			),
		);
		if (quarriesPlanned > 0) {
			decisions.push({ kind: "quarry", count: quarriesPlanned });
		}
	}

	// --- Scout: keep up to SCOUT_TARGET explore jobs out while any
	// reachable frontier tile is still fogged.
	let scoutsPlanned = 0;
	if (snapshot.hasFrontier) {
		const idleForScout = Math.max(
			0,
			snapshot.idleCats - huntsPlanned - waterFetchPlanned - quarriesPlanned,
		);
		const roomForScout = Math.max(
			0,
			employmentRoom - huntsPlanned - waterFetchPlanned - quarriesPlanned,
		);
		scoutsPlanned = Math.max(
			0,
			Math.min(
				SCOUT_TARGET - snapshot.activeScouts,
				idleForScout,
				roomForScout,
			),
		);
		if (scoutsPlanned > 0) {
			decisions.push({ kind: "scout", count: scoutsPlanned });
		}
	}

	// --- Storehouse: stores brushing the cap, but only up to the
	// population-scaled storehouse cap (and one build at a time). Without the
	// cap the leader re-triggers every time a finished granary leaves food
	// still above 90%, carpeting the clearing in storehouses.
	const storehousesPlannedOrBuilt =
		snapshot.storehouseCount + snapshot.storagePlansInFlight;
	if (
		ratio > STORAGE_RATIO &&
		snapshot.storagePlansInFlight === 0 &&
		storehousesPlannedOrBuilt < snapshot.storehouseCap
	) {
		decisions.push({ kind: "build_storage" });
	}

	// --- Den: crowding pressure ----------------------------------------
	const shelter = snapshot.housing.capacity + snapshot.housing.committed;
	const pressure =
		shelter <= 0 ? Number.POSITIVE_INFINITY : snapshot.population / shelter;
	if (pressure >= DEN_PRESSURE_THRESHOLD && snapshot.denPlansInFlight === 0) {
		decisions.push({ kind: "build_den" });
	}

	// --- Workshops: staff idlers not already claimed by hunts/quarry/scout
	const idleAfterHunts = Math.max(
		0,
		snapshot.idleCats -
			huntsPlanned -
			waterFetchPlanned -
			quarriesPlanned -
			scoutsPlanned,
	);
	const staffing = Math.min(snapshot.workshopsNeedingWorkers, idleAfterHunts);
	if (staffing > 0) {
		decisions.push({ kind: "assign_workshop", count: staffing });
	}

	// --- Tithe: offer surplus stores to the shrine ---------------------
	const titheFood =
		snapshot.resources.food >
		snapshot.foodCapacity * TITHE_FOOD_RATIO + TITHE_FOOD_AMOUNT
			? TITHE_FOOD_AMOUNT
			: 0;
	const titheRefined =
		snapshot.resources.refined >= TITHE_REFINED_AMOUNT
			? TITHE_REFINED_AMOUNT
			: 0;
	const blessings = (titheFood > 0 ? 1 : 0) + (titheRefined > 0 ? 1 : 0);
	if (blessings > 0) {
		decisions.push({
			kind: "tithe",
			food: titheFood,
			refined: titheRefined,
			blessings,
		});
	}

	return decisions;
}
