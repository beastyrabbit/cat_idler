/**
 * Leader utility director — the colony's IAUS-style brain.
 *
 * The old leader was a hand-ordered list of `if (ratio < threshold)` rules,
 * each axis decided in isolation, so it could never say "water is a crisis,
 * defer the storehouse and pull two hunters onto water." This module scores
 * every goal on ONE [0,1] scale from response curves over a colony snapshot,
 * then hands a shared employment budget to the highest-scoring goals first —
 * so scarce labour flows to the most urgent axis automatically, with no
 * priority list to maintain.
 *
 * Pure and deterministic (no DB, no RNG, no Date.now): the same snapshot always
 * yields the same plan, so 1x and 10000x agree and the seeded policy roll —
 * which still lives at the executor's call site — is the only source of leader
 * fallibility. See docs/LEADER_AI_DESIGN.md.
 */

import type { CatSpecialization } from "./idleEngine";
import type { LeaderDecision, LeaderSnapshot } from "./leaderAI";

// --- Tunables ---------------------------------------------------------------

/** Fraction of the stage-weighted workforce the leader commits to core jobs. */
export const EMPLOYMENT_TARGET_RATIO = 0.7;
/**
 * Once the core budget is spent, the leader keeps filling low-priority work
 * (extra hunts, scouting) until at least this fraction of able cats are busy —
 * the "near-zero idle" floor that makes the village read as intentional.
 */
export const IDLE_EMPLOYMENT_FLOOR = 0.8;
/**
 * Projection horizon in ticks. A store that empties within this many ticks
 * scores maximum urgency regardless of how full it looks — essential at
 * 10000x, where a full tank can drain between two ticks.
 */
export const PROJECTION_HORIZON_TICKS = 6;
/** Above this food/capacity ratio the leader calls hunts already out back home. */
export const HUNT_CANCEL_RATIO = 1.1;
/** Above this food/capacity ratio the leader commissions another storehouse. */
export const STORAGE_RATIO = 0.9;
/** Housing pressure (pop / (capacity + committed)) that commissions a den. */
export const DEN_PRESSURE_THRESHOLD = 0.8;
/** Food/water ratio both stores must clear before a mouth is spared for study. */
export const RESEARCH_COMFORT_RATIO = 0.5;
/** Stores must exceed this fraction of capacity before food is tithed. */
export const TITHE_FOOD_RATIO = 0.6;
/** Food spent per blessing when tithing surplus. */
export const TITHE_FOOD_AMOUNT = 20;
/** Refined goods spent per blessing when tithing. */
export const TITHE_REFINED_AMOUNT = 5;
/** A standing hunt yields diminishing value; cap concurrent hunts per this. */
export const HUNT_MAX_SLOTS_RATIO = 0.7;
/** Fishing is steady supplemental food, capped so it complements hunts. */
export const FISH_MAX_SLOTS = 1;
/** Fetchers the leader will run at once when the reservoir is dry. */
export const WATER_MAX_SLOTS = 4;
/** Quarry expeditions the leader will run at once when materials are low. */
export const QUARRY_MAX_SLOTS = 2;
/** Explore jobs the leader keeps out while a frontier remains. */
export const SCOUT_MAX_SLOTS = 2;
/**
 * Baseline urgency of scouting the frontier — deliberately modest so genuine
 * survival crises (food/water near empty) always outrank exploration for
 * labour, but a calm colony still keeps mapping.
 */
export const SCOUT_BASE_SCORE = 0.3;
/** Baseline urgency of staffing a standing workshop/smithy/research hut. */
export const STAFF_BASE_SCORE = 0.45;
/** Baseline urgency of training toward the threat-scaled guard. */
export const WARRIOR_BASE_SCORE = 0.5;
/** Standing warriors the leader aims for, by threat band. */
export const WARRIOR_TARGET_BY_BAND: Record<
	"calm" | "rising" | "imminent",
	number
> = { calm: 2, rising: 4, imminent: 7 };
/** Never train more than this fraction of the workforce into warriors. */
export const WARRIOR_MAX_RATIO = 0.4;

const EPS = 1e-9;

/**
 * Standing warriors the leader wants, given the threat band and colony size. A
 * bigger colony can afford a bigger guard, always capped at
 * {@link WARRIOR_MAX_RATIO} of the workforce so the economy isn't hollowed out.
 */
export function targetWarriors(snapshot: LeaderSnapshot): number {
	if (!snapshot.hasBarracks) {
		return 0;
	}
	const band = snapshot.threatBand ?? "calm";
	const base = WARRIOR_TARGET_BY_BAND[band];
	const workforce = snapshot.workforce ?? snapshot.population;
	const cap = Math.floor(workforce * WARRIOR_MAX_RATIO);
	return Math.min(base, Math.max(1, cap));
}

// --- Response curves --------------------------------------------------------

export function clamp01(x: number): number {
	return x < 0 ? 0 : x > 1 ? 1 : x;
}

/**
 * Deficit urgency for a fillable store. Inverse-quadratic in the fill ratio, so
 * urgency ramps hard as the store nears empty and flattens near full. 0 at or
 * above full, 1 at empty.
 */
export function deficitCurve(ratio: number): number {
	const r = clamp01(ratio);
	return (1 - r) * (1 - r);
}

/**
 * Projected-scarcity urgency: how close a store is to running dry given its net
 * drain per tick. A store not draining scores 0; one that empties within the
 * horizon scores 1. This is what lets a still-full but fast-draining reservoir
 * score high at accelerated time scales.
 */
export function projectionCurve(
	amount: number,
	drainPerTick: number,
	horizonTicks: number = PROJECTION_HORIZON_TICKS,
): number {
	if (drainPerTick <= 0 || horizonTicks <= 0) {
		return 0;
	}
	const ticksToEmpty = Math.max(0, amount) / Math.max(EPS, drainPerTick);
	return clamp01(1 - ticksToEmpty / horizonTicks);
}

/**
 * Logistic pressure around a centre — a decision flips on decisively rather
 * than ramping linearly. Used for housing (commission a den once crowding
 * crosses the threshold).
 */
export function pressureCurve(
	pressure: number,
	center: number = DEN_PRESSURE_THRESHOLD,
	steepness = 10,
): number {
	return clamp01(1 / (1 + Math.exp(-steepness * (pressure - center))));
}

/**
 * Surplus urgency: 0 up to `threshold`, then ramps to 1 at full. The inverse of
 * the deficit curve — used to decide when a store is comfortable enough to give
 * away (tithe) or worth expanding (storehouse).
 */
export function surplusCurve(ratio: number, threshold: number): number {
	if (ratio <= threshold) {
		return 0;
	}
	return clamp01((ratio - threshold) / (1 - threshold));
}

/**
 * Probabilistic-OR combine: `1 - (1-a)(1-b)`. Keeps the result in [0,1] and
 * lets either consideration alone push a goal high — a store that is either
 * low *or* draining fast is urgent.
 */
export function combineOr(a: number, b: number): number {
	return clamp01(1 - (1 - clamp01(a)) * (1 - clamp01(b)));
}

/** At or above this fill ratio the projection lookahead is fully suppressed. */
export const PROJECTION_GATE_RATIO = 0.9;

/**
 * How much the projection lookahead is allowed to count, given the store's fill
 * ratio: fully suppressed at/above {@link PROJECTION_GATE_RATIO} (a brimming
 * store is never a crisis, however fast it drains, because it is being
 * replenished) and ramping to full weight as the store empties. This keeps the
 * director from panic-hunting on a full larder while still reacting a tick early
 * when a *low* store is draining fast at accelerated time scales.
 */
export function projectionGate(fillRatio: number): number {
	return clamp01((PROJECTION_GATE_RATIO - fillRatio) / PROJECTION_GATE_RATIO);
}

/**
 * Combined urgency for a survival store: its standing deficit, OR-ed with a
 * projected-scarcity term that only counts once the store is drawn down enough
 * for the drain to matter.
 */
export function survivalScore(
	fillRatio: number,
	amount: number,
	drainPerTick: number,
): number {
	return combineOr(
		deficitCurve(fillRatio),
		projectionCurve(amount, drainPerTick) * projectionGate(fillRatio),
	);
}

// --- Snapshot helpers -------------------------------------------------------

function ratio(amount: number, capacity: number): number {
	if (capacity <= 0) {
		return amount > 0 ? 1 : 0;
	}
	return amount / capacity;
}

/** Stage-weighted workforce, falling back to raw population. */
function workforceOf(s: LeaderSnapshot): number {
	return s.workforce ?? s.population;
}

/** Cats able to take work (idle + already employed), for the idle floor. */
function ableCats(s: LeaderSnapshot): number {
	return s.idleCats + s.employedCats;
}

// --- Labour goals -----------------------------------------------------------

/** Job kinds the director staffs from the shared idle pool. */
export type LaborGoalKind =
	| "hunt"
	| "fish"
	| "fetch_water"
	| "quarry"
	| "scout"
	| "train_warrior"
	| "assign_workshop"
	| "assign_research"
	| "assign_smithy";

/** The cat stat a goal is matched on, and any specialization it prefers. */
export interface GoalSkill {
	skill:
		| "hunting"
		| "building"
		| "vision"
		| "medicine"
		| "attackDefense"
		| "leadership";
	preferSpecialization: CatSpecialization;
}

export const GOAL_SKILL: Record<LaborGoalKind, GoalSkill> = {
	hunt: { skill: "hunting", preferSpecialization: "hunter" },
	fish: { skill: "hunting", preferSpecialization: "hunter" },
	fetch_water: { skill: "hunting", preferSpecialization: null },
	quarry: { skill: "building", preferSpecialization: "architect" },
	scout: { skill: "vision", preferSpecialization: null },
	train_warrior: { skill: "attackDefense", preferSpecialization: null },
	assign_workshop: { skill: "building", preferSpecialization: null },
	assign_research: { skill: "medicine", preferSpecialization: null },
	assign_smithy: { skill: "building", preferSpecialization: "architect" },
};

interface LaborGoal {
	kind: LaborGoalKind;
	score: number;
	/** Total slots wanted, before subtracting in-flight and budget. */
	maxSlots: number;
	/** Slots already covered by active/queued jobs or assigned workers. */
	inFlight: number;
	/** Hard cap on slots regardless of score (e.g. buildings needing workers). */
	hardCap: number;
	/** True once staffing this goal is impossible (veto gate). */
	vetoed: boolean;
	/**
	 * How the score maps to a slot count. `scaled` goals send more cats the more
	 * urgent they are (hunt/water/quarry — a famine pulls the whole workforce);
	 * `fixed` goals want their full target whenever they are worth doing at all
	 * (a workshop needs exactly one cat, a frontier its couple of scouts), with
	 * the score deciding only their *priority* in the budget queue.
	 */
	mode: "scaled" | "fixed";
}

/** Score and size every labour goal from the snapshot. */
function laborGoals(s: LeaderSnapshot): LaborGoal[] {
	const budget = Math.floor(workforceOf(s) * EMPLOYMENT_TARGET_RATIO);
	const foodR = ratio(s.resources.food, s.foodCapacity);
	const waterR = ratio(s.water, s.waterCapacity);
	const materialsR = ratio(s.materials, s.materialsCapacity);

	const foodScore = survivalScore(
		foodR,
		s.resources.food,
		s.foodDrainPerTick ?? 0,
	);
	const waterScore = survivalScore(waterR, s.water, s.waterDrainPerTick ?? 0);
	const materialsScore = deficitCurve(materialsR);

	const comfortable =
		foodR >= RESEARCH_COMFORT_RATIO && waterR >= RESEARCH_COMFORT_RATIO;
	const warriorGap =
		targetWarriors(s) - (s.warriorCount ?? 0) - (s.trainingInFlight ?? 0);

	const goals: LaborGoal[] = [
		{
			kind: "hunt",
			score: foodScore,
			// Hunting can absorb most of the workforce in a famine.
			maxSlots: Math.ceil(budget * HUNT_MAX_SLOTS_RATIO),
			inFlight: s.activeHunts,
			hardCap: Math.ceil(budget * HUNT_MAX_SLOTS_RATIO),
			// Overflowing stores: hold, don't dispatch (cancellation handled apart).
			vetoed: foodR >= 1,
			mode: "scaled",
		},
		{
			kind: "fish",
			score: foodScore,
			maxSlots: FISH_MAX_SLOTS,
			inFlight: s.activeFishers ?? 0,
			hardCap: FISH_MAX_SLOTS,
			vetoed: !s.hasFishingSite || foodR >= 1,
			mode: "scaled",
		},
		{
			kind: "fetch_water",
			score: waterScore,
			maxSlots: WATER_MAX_SLOTS,
			inFlight: s.activeWaterFetchers,
			hardCap: WATER_MAX_SLOTS,
			vetoed: !s.hasWaterSite || waterR >= 1,
			mode: "scaled",
		},
		{
			kind: "quarry",
			score: materialsScore,
			maxSlots: QUARRY_MAX_SLOTS,
			inFlight: s.activeQuarries,
			hardCap: QUARRY_MAX_SLOTS,
			vetoed: !s.hasQuarrySite || materialsR >= 1,
			mode: "scaled",
		},
		{
			kind: "scout",
			score: SCOUT_BASE_SCORE,
			maxSlots: SCOUT_MAX_SLOTS,
			inFlight: s.activeScouts,
			hardCap: SCOUT_MAX_SLOTS,
			vetoed: !s.hasFrontier,
			mode: "fixed",
		},
		{
			kind: "assign_workshop",
			score: STAFF_BASE_SCORE,
			maxSlots: s.workshopsNeedingWorkers,
			inFlight: 0,
			hardCap: s.workshopsNeedingWorkers,
			vetoed: s.workshopsNeedingWorkers <= 0 || Boolean(s.starving),
			mode: "fixed",
		},
		{
			kind: "assign_research",
			score: STAFF_BASE_SCORE,
			maxSlots: s.researchHutsNeedingWorkers ?? 0,
			inFlight: 0,
			hardCap: s.researchHutsNeedingWorkers ?? 0,
			// A researcher gathers nothing — only staff when stores are comfortable.
			vetoed: (s.researchHutsNeedingWorkers ?? 0) <= 0 || !comfortable,
			mode: "fixed",
		},
		{
			kind: "assign_smithy",
			score: STAFF_BASE_SCORE,
			maxSlots: s.smithiesNeedingWorkers ?? 0,
			inFlight: 0,
			hardCap: s.smithiesNeedingWorkers ?? 0,
			vetoed: (s.smithiesNeedingWorkers ?? 0) <= 0 || Boolean(s.starving),
			mode: "fixed",
		},
		{
			kind: "train_warrior",
			score: WARRIOR_BASE_SCORE,
			maxSlots: Math.max(0, warriorGap),
			inFlight: 0,
			hardCap: Math.max(0, warriorGap),
			vetoed: warriorGap <= 0 || Boolean(s.starving),
			mode: "fixed",
		},
	];

	return goals;
}

/**
 * Open slots a goal wants staffed *now*, clamped to its hard cap. A `scaled`
 * goal sends more cats the more urgent it is; a `fixed` goal wants its full
 * target the moment it is worth doing at all (so a single-slot workshop isn't
 * rounded away by a modest priority score). A vetoed goal opens nothing.
 */
export function goalOpenSlots(goal: LaborGoal): number {
	if (goal.vetoed) {
		return 0;
	}
	const target =
		goal.mode === "fixed"
			? goal.maxSlots
			: Math.round(goal.score * goal.maxSlots);
	return Math.max(
		0,
		Math.min(target - goal.inFlight, goal.hardCap - goal.inFlight),
	);
}

/** A staffing request the executor turns into jobs/assignments. */
export interface OpenSlots {
	goal: LaborGoalKind;
	count: number;
	score: number;
}

// --- Assignment (greedy skill-fit matcher) ----------------------------------

/** Minimal cat view the assignment matcher needs. */
export interface CatBrief {
	id: string;
	specialization: CatSpecialization;
	stats: {
		hunting: number;
		building: number;
		vision: number;
		medicine: number;
		attack: number;
		defense: number;
		leadership: number;
	};
}

/** How well a cat fits a goal: relevant skill × specialization bonus. */
export function assignmentFit(cat: CatBrief, goal: LaborGoalKind): number {
	const spec = GOAL_SKILL[goal];
	let base: number;
	switch (spec.skill) {
		case "hunting":
			base = cat.stats.hunting;
			break;
		case "building":
			base = cat.stats.building;
			break;
		case "vision":
			base = cat.stats.vision;
			break;
		case "medicine":
			base = cat.stats.medicine;
			break;
		case "leadership":
			base = cat.stats.leadership;
			break;
		case "attackDefense":
			base = cat.stats.attack + cat.stats.defense;
			break;
	}
	const specMatch =
		spec.preferSpecialization !== null &&
		cat.specialization === spec.preferSpecialization;
	return base * (specMatch ? 1.5 : 1);
}

/** One resolved cat→goal pairing. */
export interface Assignment {
	catId: string;
	goal: LaborGoalKind;
}

/**
 * Greedy best-cat-per-slot assignment: expand every open slot (in the
 * director's descending-score order), and for each pick the still-available cat
 * that fits it best. O(slots × cats) and deterministic — ties break by a stable
 * cat-id key, never by RNG — so the same snapshot always assigns the same cats.
 * This is the single global pass that replaces the executor's per-goal sorts, so
 * a great hunter is never burned on a scout slot while a scrub takes the hunt.
 */
export function matchCatsToSlots(
	slots: OpenSlots[],
	cats: CatBrief[],
	options: { excludeWarriorsFromTraining?: boolean } = {},
): Assignment[] {
	// Expand slots in priority order (already score-sorted by the director).
	const flat: LaborGoalKind[] = [];
	for (const slot of slots) {
		for (let i = 0; i < slot.count; i += 1) {
			flat.push(slot.goal);
		}
	}

	const pool = [...cats];
	const assignments: Assignment[] = [];
	for (const goal of flat) {
		let bestIdx = -1;
		let bestFit = -Infinity;
		for (let i = 0; i < pool.length; i += 1) {
			const cat = pool[i];
			if (
				goal === "train_warrior" &&
				options.excludeWarriorsFromTraining &&
				cat.specialization === "warrior"
			) {
				continue;
			}
			const fit = assignmentFit(cat, goal);
			// Strictly-greater keeps the first (stable id order) on ties.
			if (fit > bestFit) {
				bestFit = fit;
				bestIdx = i;
			}
		}
		if (bestIdx < 0) {
			continue;
		}
		assignments.push({ catId: pool[bestIdx].id, goal });
		pool.splice(bestIdx, 1);
	}
	return assignments;
}

// --- The director -----------------------------------------------------------

/** The director's full output: quotas for the executor's assignment pass. */
export interface DirectorPlan {
	/** Non-labour decisions (build/tithe/cancel) in a stable order. */
	decisions: LeaderDecision[];
	/** Labour slots to fill, highest priority first. */
	slots: OpenSlots[];
}

/**
 * Score every goal, allocate the shared employment budget in descending score,
 * then keep filling leftover able cats with low-priority work until the idle
 * floor is met. Returns the labour slots (priority-ordered) plus the standalone
 * build/tithe/cancel decisions.
 */
export function directColony(s: LeaderSnapshot): DirectorPlan {
	const decisions: LeaderDecision[] = [];
	const foodR = ratio(s.resources.food, s.foodCapacity);

	// --- Cancellations first: they free labour rather than spend it. --------
	if (foodR > HUNT_CANCEL_RATIO && s.activeHunts > 0) {
		decisions.push({ kind: "cancel_hunts" });
	}
	if (s.starving && (s.trainingInFlight ?? 0) > 0) {
		decisions.push({ kind: "cancel_training" });
	}

	// --- Labour allocation: descending score over the idle pool. Each goal's
	// own maxSlots is the reserve — hunting is capped at a fraction of the
	// workforce so a famine can't send literally everyone out, leaving cats for
	// water, staffing, and the low-priority fill below. Score decides only the
	// *order* cats flow to goals, so the most urgent axis is served first. -----
	let labourLeft = s.idleCats;

	const goals = laborGoals(s);
	// Stable sort: score desc, then a fixed goal order for ties.
	const order: LaborGoalKind[] = [
		"fetch_water",
		"fish",
		"hunt",
		"quarry",
		"train_warrior",
		"assign_smithy",
		"assign_workshop",
		"assign_research",
		"scout",
	];
	const ranked = [...goals].sort((a, b) => {
		if (b.score !== a.score) {
			return b.score - a.score;
		}
		return order.indexOf(a.kind) - order.indexOf(b.kind);
	});

	const granted = new Map<LaborGoalKind, number>();
	const grant = (kind: LaborGoalKind, n: number) => {
		granted.set(kind, (granted.get(kind) ?? 0) + n);
	};

	for (const goal of ranked) {
		if (labourLeft <= 0) {
			break;
		}
		const want = goalOpenSlots(goal);
		const give = Math.min(want, labourLeft);
		if (give > 0) {
			grant(goal.kind, give);
			labourLeft -= give;
		}
	}

	// --- Near-zero idle: spend remaining able cats (even beyond the core
	// budget) on useful low-priority work until the idle floor is met. Extra
	// hunts absorb the most cats, then scouting; both are capped by whether the
	// work exists (stores not overflowing, a frontier still fogged). ----------
	const busySoFar =
		s.employedCats + [...granted.values()].reduce((a, b) => a + b, 0);
	const employTarget = Math.ceil(ableCats(s) * IDLE_EMPLOYMENT_FLOOR);
	let idleLeft = Math.max(0, s.idleCats - (busySoFar - s.employedCats));
	let fillWanted = Math.max(0, Math.min(idleLeft, employTarget - busySoFar));

	const fillOrder: Array<{ kind: LaborGoalKind; open: () => boolean }> = [
		{ kind: "hunt", open: () => foodR < 1 },
		{
			kind: "fish",
			open: () =>
				foodR < 1 &&
				Boolean(s.hasFishingSite) &&
				(s.activeFishers ?? 0) + (granted.get("fish") ?? 0) < FISH_MAX_SLOTS,
		},
		{ kind: "scout", open: () => s.hasFrontier },
		{ kind: "quarry", open: () => s.hasQuarrySite },
	];
	// Round-robin so leftover cats spread across the open fill work.
	let progress = true;
	while (fillWanted > 0 && progress) {
		progress = false;
		for (const fill of fillOrder) {
			if (fillWanted <= 0) {
				break;
			}
			if (!fill.open()) {
				continue;
			}
			grant(fill.kind, 1);
			fillWanted -= 1;
			idleLeft -= 1;
			progress = true;
		}
	}
	void idleLeft;

	// --- Emit labour slots in the ranked (priority) order. ------------------
	const slots: OpenSlots[] = [];
	for (const goal of ranked) {
		const count = granted.get(goal.kind) ?? 0;
		if (count > 0) {
			const g = goals.find((x) => x.kind === goal.kind);
			slots.push({ goal: goal.kind, count, score: g?.score ?? 0 });
		}
	}

	// --- Capital projects: single, gated, standalone (use a chosen builder,
	// not the idle pool). ----------------------------------------------------
	const storehousesInPlay = s.storehouseCount + s.storagePlansInFlight;
	if (
		foodR > STORAGE_RATIO &&
		s.storagePlansInFlight === 0 &&
		storehousesInPlay < s.storehouseCap
	) {
		decisions.push({ kind: "build_storage" });
	}

	const shelter = s.housing.capacity + s.housing.committed;
	const pressure =
		shelter <= 0 ? Number.POSITIVE_INFINITY : s.population / shelter;
	if (pressure >= DEN_PRESSURE_THRESHOLD && s.denPlansInFlight === 0) {
		decisions.push({ kind: "build_den" });
	}

	// --- Tithe surplus to the shrine. ---------------------------------------
	const titheFood =
		s.resources.food > s.foodCapacity * TITHE_FOOD_RATIO + TITHE_FOOD_AMOUNT
			? TITHE_FOOD_AMOUNT
			: 0;
	const titheRefined =
		s.resources.refined >= TITHE_REFINED_AMOUNT ? TITHE_REFINED_AMOUNT : 0;
	const blessings = (titheFood > 0 ? 1 : 0) + (titheRefined > 0 ? 1 : 0);
	if (blessings > 0) {
		decisions.push({
			kind: "tithe",
			food: titheFood,
			refined: titheRefined,
			blessings,
		});
	}

	return { decisions, slots };
}
