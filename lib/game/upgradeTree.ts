/**
 * God / Cat upgrade tree (pure data model + rules).
 *
 * One tree, two ways to advance (roadmap section 3, "Research & god upgrade
 * tree"):
 *   - Gods spend blessings to buy a node instantly (`godPurchase`).
 *   - Cats research nodes slowly: a dedicated researcher accrues ~10 points
 *     per week (`pointsPerTickFor` / `accrueResearch`) and the colony unlocks
 *     the cheapest affordable node on its own (`catAutoUnlock`).
 *
 * This module is pure: no DB imports, no side effects, JSON-friendly state.
 * Integration into the tick and the `colonies` schema comes in a later pass —
 * `UpgradeTreeState` is shaped to drop straight into a colony column.
 */

// =============================================================================
// Effect registry
// =============================================================================

/**
 * Every modifier a node can grant. `mult` effects stack additively and are
 * resolved as `1 + sum` (a value of 0.1 == +10%). `add` effects are plain
 * additive bonuses resolved as `sum` (default 0).
 */
export type EffectKey =
	| "huntYieldMult"
	| "gatherYieldMult"
	| "materialYieldMult"
	| "farmYieldMult"
	| "moveSpeedMult"
	| "combatPowerMult"
	| "defenseMult"
	| "researchRateMult"
	| "storagePerLevelMult"
	| "housingPerDen"
	| "waterCarryCapacity";

export type EffectKind = "mult" | "add";

export const EFFECT_KINDS: Record<EffectKey, EffectKind> = {
	huntYieldMult: "mult",
	gatherYieldMult: "mult",
	materialYieldMult: "mult",
	farmYieldMult: "mult",
	moveSpeedMult: "mult",
	combatPowerMult: "mult",
	defenseMult: "mult",
	researchRateMult: "mult",
	storagePerLevelMult: "mult",
	housingPerDen: "add",
	waterCarryCapacity: "add",
};

export const EFFECT_KEYS = Object.keys(EFFECT_KINDS) as EffectKey[];

export interface NodeEffect {
	key: EffectKey;
	value: number;
}

/** What a node makes available once owned. All fields optional. */
export interface UpgradeUnlocks {
	buildings?: string[];
	jobs?: string[];
	effects?: NodeEffect[];
}

// =============================================================================
// Node model
// =============================================================================

export type UpgradeEra = 1 | 2 | 3;

export interface UpgradeNode {
	id: string;
	name: string;
	description: string;
	era: UpgradeEra;
	/** Cost in research points (cats) or blessings (gods). Always 5-25. */
	cost: number;
	prerequisites: string[];
	unlocks: UpgradeUnlocks;
}

/**
 * The starter tree — ~18 nodes across three eras of Age-of-Empires style
 * progression. Building/job ids reuse existing enums where they exist
 * (`workshop`, `field`, `den`, `food_storage`, `hunt_expedition`, ...) and
 * introduce forward-looking ids (`research_hut`, `sawmill`, `smithy`,
 * `barracks`, `school`) as plain strings for later systems to adopt.
 */
export const UPGRADE_NODES: UpgradeNode[] = [
	// --- Era 1: Foundations -------------------------------------------------
	{
		id: "research_hut",
		name: "Research Hut",
		description:
			"Build the research hut and assign a scholar. The root of the whole tree — nothing is researched until a mouth is spared to study.",
		era: 1,
		cost: 5,
		prerequisites: [],
		unlocks: { buildings: ["research_hut"], jobs: ["research"] },
	},
	{
		id: "basic_tools",
		name: "Basic Tools",
		description: "Knapped claws and better snares. Hunters bring back more.",
		era: 1,
		cost: 5,
		prerequisites: ["research_hut"],
		unlocks: { effects: [{ key: "huntYieldMult", value: 0.1 }] },
	},
	{
		id: "water_carriers",
		name: "Water Carriers",
		description: "Woven gourds let a fetch-water trip haul far more per run.",
		era: 1,
		cost: 8,
		prerequisites: ["research_hut"],
		unlocks: {
			jobs: ["fetch_water"],
			effects: [{ key: "waterCarryCapacity", value: 1 }],
		},
	},
	{
		id: "den_insulation",
		name: "Den Insulation",
		description: "Moss-lined dens shelter another cat each without the chill.",
		era: 1,
		cost: 8,
		prerequisites: ["research_hut"],
		unlocks: { effects: [{ key: "housingPerDen", value: 1 }] },
	},
	{
		id: "foraging_lore",
		name: "Foraging Lore",
		description: "Elders teach which berries feed and which kill.",
		era: 1,
		cost: 6,
		prerequisites: ["basic_tools"],
		unlocks: { effects: [{ key: "gatherYieldMult", value: 0.15 }] },
	},

	// --- Era 2: Craft & Growth (gated buildings) ----------------------------
	{
		id: "sawmill",
		name: "Sawmill",
		description:
			"Raise the Sägewerk. Felled timber becomes usable materials far faster.",
		era: 2,
		cost: 12,
		prerequisites: ["foraging_lore"],
		unlocks: {
			buildings: ["sawmill"],
			jobs: ["quarry"],
			effects: [{ key: "materialYieldMult", value: 0.2 }],
		},
	},
	{
		id: "masonry",
		name: "Masonry",
		description: "Stacked stone stores. Every storehouse level holds more.",
		era: 2,
		cost: 12,
		prerequisites: ["sawmill"],
		unlocks: { effects: [{ key: "storagePerLevelMult", value: 0.25 }] },
	},
	{
		id: "smithy",
		name: "Smithy",
		description: "Build the smithy. Metal tools open the path to weapons.",
		era: 2,
		cost: 15,
		prerequisites: ["sawmill"],
		unlocks: { buildings: ["smithy"], jobs: ["forge_tools"] },
	},
	{
		id: "barracks",
		name: "Barracks",
		description: "Raise the barracks so cats can drill into real warriors.",
		era: 2,
		cost: 18,
		prerequisites: ["basic_tools"],
		unlocks: { buildings: ["barracks"], jobs: ["train_warrior"] },
	},
	{
		id: "school",
		name: "School",
		description:
			"Build the school. Kittens sit and learn, feeding the research effort while they grow.",
		era: 2,
		cost: 15,
		prerequisites: ["den_insulation"],
		unlocks: {
			buildings: ["school"],
			jobs: ["teach"],
			effects: [{ key: "researchRateMult", value: 0.5 }],
		},
	},
	{
		id: "irrigation",
		name: "Irrigation",
		description: "Dug channels feed the fields. Crops come in heavier.",
		era: 2,
		cost: 10,
		prerequisites: ["water_carriers"],
		unlocks: {
			buildings: ["field"],
			effects: [{ key: "farmYieldMult", value: 0.2 }],
		},
	},

	// --- Era 3: Advanced ----------------------------------------------------
	{
		id: "housing_tier_2",
		name: "Timbered Longdens",
		description: "Two-storey dens. Each den now shelters a small clan.",
		era: 3,
		cost: 20,
		prerequisites: ["masonry"],
		unlocks: { effects: [{ key: "housingPerDen", value: 2 }] },
	},
	{
		id: "weaponsmithing",
		name: "Weaponsmithing",
		description: "Forge claws of iron. Warriors strike far harder.",
		era: 3,
		cost: 22,
		prerequisites: ["smithy"],
		unlocks: {
			jobs: ["forge_weapon"],
			effects: [{ key: "combatPowerMult", value: 0.25 }],
		},
	},
	{
		id: "armorsmithing",
		name: "Armorsmithing",
		description:
			"Hammered plate. Defenders shrug off blows that once felled them.",
		era: 3,
		cost: 22,
		prerequisites: ["smithy"],
		unlocks: {
			jobs: ["forge_armor"],
			effects: [{ key: "defenseMult", value: 0.25 }],
		},
	},
	{
		id: "advanced_storage",
		name: "Advanced Storage",
		description:
			"Sealed cellars and lofts. Storehouses hold half again as much.",
		era: 3,
		cost: 18,
		prerequisites: ["masonry"],
		unlocks: { effects: [{ key: "storagePerLevelMult", value: 0.5 }] },
	},
	{
		id: "scholars_guild",
		name: "Scholars' Guild",
		description: "A true academy. Research races ahead.",
		era: 3,
		cost: 25,
		prerequisites: ["school"],
		unlocks: { effects: [{ key: "researchRateMult", value: 0.75 }] },
	},
	{
		id: "mounted_scouts",
		name: "Mounted Scouts",
		description: "Trained runners cover far more ground between waypoints.",
		era: 3,
		cost: 20,
		prerequisites: ["barracks"],
		unlocks: {
			jobs: ["explore"],
			effects: [{ key: "moveSpeedMult", value: 0.3 }],
		},
	},
	{
		id: "grand_housing",
		name: "Grand Housing",
		description: "Stone halls. A single den now houses a whole lineage.",
		era: 3,
		cost: 25,
		prerequisites: ["housing_tier_2"],
		unlocks: { effects: [{ key: "housingPerDen", value: 3 }] },
	},
];

export const UPGRADE_NODE_BY_ID: Record<string, UpgradeNode> =
	Object.fromEntries(UPGRADE_NODES.map((node) => [node.id, node]));

export function getNode(id: string): UpgradeNode | undefined {
	return UPGRADE_NODE_BY_ID[id];
}

// =============================================================================
// State
// =============================================================================

/** JSON-friendly progression state, ready to persist in a colony column. */
export interface UpgradeTreeState {
	ownedNodeIds: string[];
	researchPoints: number;
}

export function createUpgradeTreeState(): UpgradeTreeState {
	return { ownedNodeIds: [], researchPoints: 0 };
}

export function serializeUpgradeTreeState(state: UpgradeTreeState): {
	ownedNodeIds: string[];
	researchPoints: number;
} {
	return {
		ownedNodeIds: [...state.ownedNodeIds],
		researchPoints: state.researchPoints,
	};
}

/**
 * Rebuild state from arbitrary persisted JSON, filling defaults for missing
 * or malformed fields. Unknown/duplicate node ids are dropped so downstream
 * rules never see junk.
 */
export function deserializeUpgradeTreeState(raw: unknown): UpgradeTreeState {
	const base = createUpgradeTreeState();
	if (!raw || typeof raw !== "object") {
		return base;
	}

	const obj = raw as Record<string, unknown>;

	const ownedRaw = Array.isArray(obj.ownedNodeIds) ? obj.ownedNodeIds : [];
	const seen = new Set<string>();
	const ownedNodeIds: string[] = [];
	for (const id of ownedRaw) {
		if (typeof id === "string" && UPGRADE_NODE_BY_ID[id] && !seen.has(id)) {
			seen.add(id);
			ownedNodeIds.push(id);
		}
	}

	const points =
		typeof obj.researchPoints === "number" &&
		Number.isFinite(obj.researchPoints)
			? Math.max(0, obj.researchPoints)
			: 0;

	return { ownedNodeIds, researchPoints: points };
}

// =============================================================================
// Ownership + gating rules
// =============================================================================

export function isOwned(state: UpgradeTreeState, id: string): boolean {
	return state.ownedNodeIds.includes(id);
}

/** True when every prerequisite of `id` is owned. */
export function prerequisitesMet(state: UpgradeTreeState, id: string): boolean {
	const node = UPGRADE_NODE_BY_ID[id];
	if (!node) {
		return false;
	}
	return node.prerequisites.every((prereq) => isOwned(state, prereq));
}

/**
 * True when `id` is a real, not-yet-owned node whose prerequisites are all
 * satisfied. Points are NOT considered here — affordability is a separate
 * concern (see `catAutoUnlock`).
 */
export function canUnlock(state: UpgradeTreeState, id: string): boolean {
	const node = UPGRADE_NODE_BY_ID[id];
	if (!node || isOwned(state, id)) {
		return false;
	}
	return prerequisitesMet(state, id);
}

/** Every node that could be unlocked right now, in stable definition order. */
export function unlockableNodes(state: UpgradeTreeState): UpgradeNode[] {
	return UPGRADE_NODES.filter((node) => canUnlock(state, node.id));
}

function withOwned(state: UpgradeTreeState, id: string): string[] {
	return [...state.ownedNodeIds, id];
}

// =============================================================================
// God purchases (blessings, instant)
// =============================================================================

export interface PurchaseResult {
	ok: boolean;
	state: UpgradeTreeState;
	/** Blessings the god must spend (equal to node cost) when `ok`. */
	blessingsCost: number;
	reason?: "unknown-node" | "already-owned" | "prerequisites-unmet";
}

/**
 * Gods buy a node outright with blessings — instant, no research points spent.
 * Blessings themselves are held elsewhere (the colony ledger); this only
 * reports the cost and hands back the new state. On failure the input state is
 * returned unchanged.
 */
export function godPurchase(
	state: UpgradeTreeState,
	id: string,
): PurchaseResult {
	const node = UPGRADE_NODE_BY_ID[id];
	if (!node) {
		return { ok: false, state, blessingsCost: 0, reason: "unknown-node" };
	}
	if (isOwned(state, id)) {
		return { ok: false, state, blessingsCost: 0, reason: "already-owned" };
	}
	if (!prerequisitesMet(state, id)) {
		return {
			ok: false,
			state,
			blessingsCost: 0,
			reason: "prerequisites-unmet",
		};
	}

	return {
		ok: true,
		state: { ...state, ownedNodeIds: withOwned(state, id) },
		blessingsCost: node.cost,
	};
}

// =============================================================================
// Cat research (slow, automatic)
// =============================================================================

/** Target accrual for one full-time researcher. */
export const RESEARCH_POINTS_PER_RESEARCHER_PER_WEEK = 10;

/** Seconds in a week (7 * 24 * 60 * 60). */
export const WEEK_SECONDS = 7 * 24 * 60 * 60;

/**
 * Per-second research rate for a single full-time researcher, before any
 * `researchRateMult` bonuses. Chosen so ~10 points accrue over one week:
 * 10 / 604800 ≈ 1.6534e-5 points/second.
 */
export const RESEARCH_POINTS_PER_SECOND =
	RESEARCH_POINTS_PER_RESEARCHER_PER_WEEK / WEEK_SECONDS;

/**
 * Points produced this tick. Linear in researcher count and elapsed time;
 * `rateMult` folds in `researchRateMult` from `resolveEffects` (default 1).
 */
export function pointsPerTickFor(
	researcherCount: number,
	elapsedSec: number,
	rateMult = 1,
): number {
	if (researcherCount <= 0 || elapsedSec <= 0) {
		return 0;
	}
	return (
		researcherCount *
		elapsedSec *
		RESEARCH_POINTS_PER_SECOND *
		Math.max(0, rateMult)
	);
}

/** Add accrued points to the pool (clamped at >= 0). */
export function accrueResearch(
	state: UpgradeTreeState,
	points: number,
): UpgradeTreeState {
	if (!Number.isFinite(points) || points === 0) {
		return state;
	}
	return {
		...state,
		researchPoints: Math.max(0, state.researchPoints + points),
	};
}

/**
 * Deterministic pick of the next node the cats would research if they could
 * afford it: cheapest unlockable node, ties broken by ascending id. Ignores
 * the current point balance. Returns null when nothing is unlockable.
 */
export function nextResearchTarget(
	state: UpgradeTreeState,
): UpgradeNode | null {
	let best: UpgradeNode | null = null;
	for (const node of unlockableNodes(state)) {
		if (
			best === null ||
			node.cost < best.cost ||
			(node.cost === best.cost && node.id < best.id)
		) {
			best = node;
		}
	}
	return best;
}

export interface AutoUnlockResult {
	ok: boolean;
	state: UpgradeTreeState;
	nodeId: string | null;
}

/**
 * The cats unlock one node on their own if they can afford it: the cheapest
 * affordable unlockable node (ties broken by ascending id), spending its cost
 * from the research pool. Deterministic. On failure the state is unchanged.
 */
export function catAutoUnlock(state: UpgradeTreeState): AutoUnlockResult {
	let best: UpgradeNode | null = null;
	for (const node of unlockableNodes(state)) {
		if (node.cost > state.researchPoints) {
			continue;
		}
		if (
			best === null ||
			node.cost < best.cost ||
			(node.cost === best.cost && node.id < best.id)
		) {
			best = node;
		}
	}

	if (best === null) {
		return { ok: false, state, nodeId: null };
	}

	return {
		ok: true,
		state: {
			ownedNodeIds: withOwned(state, best.id),
			researchPoints: state.researchPoints - best.cost,
		},
		nodeId: best.id,
	};
}

// =============================================================================
// Effect aggregation
// =============================================================================

export type ResolvedEffects = Record<EffectKey, number>;

/** Neutral modifiers: `mult` keys default to 1, `add` keys to 0. */
export function neutralEffects(): ResolvedEffects {
	const out = {} as ResolvedEffects;
	for (const key of EFFECT_KEYS) {
		out[key] = EFFECT_KINDS[key] === "mult" ? 1 : 0;
	}
	return out;
}

/**
 * Aggregate every effect granted by the owned nodes into a flat modifier map
 * the tick can consume. `mult` effects resolve to `1 + sum(values)`, `add`
 * effects to `sum(values)`. Unowned/unknown ids contribute nothing.
 */
export function resolveEffects(ownedNodeIds: string[]): ResolvedEffects {
	const sums = {} as Record<EffectKey, number>;
	for (const key of EFFECT_KEYS) {
		sums[key] = 0;
	}

	for (const id of ownedNodeIds) {
		const node = UPGRADE_NODE_BY_ID[id];
		if (!node?.unlocks.effects) {
			continue;
		}
		for (const effect of node.unlocks.effects) {
			sums[effect.key] += effect.value;
		}
	}

	const out = {} as ResolvedEffects;
	for (const key of EFFECT_KEYS) {
		out[key] = EFFECT_KINDS[key] === "mult" ? 1 + sums[key] : sums[key];
	}
	return out;
}
