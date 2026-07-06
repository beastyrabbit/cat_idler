/**
 * Leader "brain" — the public contract for the colony's autonomous leader.
 *
 * The reasoning now lives in {@link ./leaderDirector}: an IAUS-style utility
 * director that scores every goal on one [0,1] scale and hands a shared
 * employment budget to the highest-scoring goals first. This module keeps the
 * shared snapshot/decision types and a thin {@link planLeaderActions} that
 * flattens the director's plan into the legacy decision list, so callers and
 * tests that only care about "what did the leader decide" are unchanged.
 *
 * Nothing here touches the DB, RNG, or policy rolls: the caller keeps the
 * seeded policy-reliability gate at the point where it executes each decision.
 */

import { directColony } from "./leaderDirector";

export {
	EMPLOYMENT_TARGET_RATIO,
	targetWarriors,
	WARRIOR_MAX_RATIO,
	WARRIOR_TARGET_BY_BAND,
} from "./leaderDirector";

export interface LeaderSnapshot {
	/** Living cats in the colony (raw head count, incl. kittens). */
	population: number;
	/**
	 * Stage-weighted count of work-capable cats — kittens count for nothing,
	 * elders partially. The employment budget is a fraction of this, not of the
	 * raw head count. Falls back to {@link population} when omitted.
	 */
	workforce?: number;
	/** Cats free to take on a new job right now. */
	idleCats: number;
	/** Cats currently occupied by any job or workplace. */
	employedCats: number;
	resources: { food: number; refined: number };
	foodCapacity: number;
	/**
	 * Net food drained this tick (consumption + spoilage). Feeds the projection
	 * curve so a still-full but fast-draining larder scores urgent at high time
	 * scales. Optional; treated as 0 (no projected scarcity) when omitted.
	 */
	foodDrainPerTick?: number;
	/** Materials in store and the cap they're clamped to. */
	materials: number;
	materialsCapacity: number;
	/** Water in store and the cap it's clamped to. */
	water: number;
	waterCapacity: number;
	/** Net water drained this tick — feeds the projection curve like food. */
	waterDrainPerTick?: number;
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
	/** Completed research huts that have no assigned researcher. */
	researchHutsNeedingWorkers?: number;
	/** Completed smithies that have no assigned smith. */
	smithiesNeedingWorkers?: number;
	/** A finished barracks stands, so cats can be trained into warriors. */
	hasBarracks?: boolean;
	/** Trained warriors currently standing. */
	warriorCount?: number;
	/** train_warrior jobs already in flight. */
	trainingInFlight?: number;
	/** Current HUD threat band — scales the warrior target. */
	threatBand?: "calm" | "rising" | "imminent";
	/** A raid is already on the map, even if stored pressure has reset. */
	raidActive?: boolean;
	/** The larder is nearly empty; the leader stops staffing/training. */
	starving?: boolean;
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
	| { kind: "assign_research"; count: number }
	| { kind: "assign_smithy"; count: number }
	| { kind: "train_warrior"; count: number }
	| { kind: "cancel_training" }
	| { kind: "tithe"; food: number; refined: number; blessings: number };

/**
 * Flatten the director's plan into a single prioritized decision list:
 * cancellations first (they free labour), then the labour slots in descending
 * urgency, then the standalone build/tithe decisions. Callers that execute the
 * labour with the skill-fit assignment matcher should call `directColony`
 * directly for the `slots`; this exists for the simpler "list of decisions"
 * consumers and the golden-master tests.
 */
export function planLeaderActions(snapshot: LeaderSnapshot): LeaderDecision[] {
	const plan = directColony(snapshot);
	const cancels = plan.decisions.filter(
		(d) => d.kind === "cancel_hunts" || d.kind === "cancel_training",
	);
	const builds = plan.decisions.filter(
		(d) => d.kind !== "cancel_hunts" && d.kind !== "cancel_training",
	);
	const labour: LeaderDecision[] = plan.slots.map((slot) => ({
		kind: slot.goal,
		count: slot.count,
	}));
	return [...cancels, ...labour, ...builds];
}
