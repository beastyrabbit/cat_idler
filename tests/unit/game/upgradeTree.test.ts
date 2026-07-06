import { describe, expect, it } from "vitest";
import {
	accrueResearch,
	canUnlock,
	catAutoUnlock,
	createUpgradeTreeState,
	deserializeUpgradeTreeState,
	EFFECT_KINDS,
	getNode,
	godPurchase,
	neutralEffects,
	nextResearchTarget,
	pointsPerTickFor,
	RESEARCH_POINTS_PER_RESEARCHER_PER_WEEK,
	RESEARCH_POINTS_PER_SECOND,
	resolveEffects,
	serializeUpgradeTreeState,
	UPGRADE_NODE_BY_ID,
	UPGRADE_NODES,
	type UpgradeTreeState,
	unlockableNodes,
	WEEK_SECONDS,
} from "@/lib/game/upgradeTree";

function stateWith(
	ownedNodeIds: string[],
	researchPoints = 0,
): UpgradeTreeState {
	return { ownedNodeIds, researchPoints };
}

describe("tree structure", () => {
	it("has ~15-20 nodes across exactly three eras", () => {
		expect(UPGRADE_NODES.length).toBeGreaterThanOrEqual(15);
		expect(UPGRADE_NODES.length).toBeLessThanOrEqual(20);
		const eras = new Set(UPGRADE_NODES.map((n) => n.era));
		expect([...eras].sort()).toEqual([1, 2, 3]);
	});

	it("keeps every cost within the 5-25 band", () => {
		for (const node of UPGRADE_NODES) {
			expect(node.cost).toBeGreaterThanOrEqual(5);
			expect(node.cost).toBeLessThanOrEqual(25);
		}
	});

	it("has unique ids and a matching lookup map", () => {
		const ids = UPGRADE_NODES.map((n) => n.id);
		expect(new Set(ids).size).toBe(ids.length);
		for (const node of UPGRADE_NODES) {
			expect(UPGRADE_NODE_BY_ID[node.id]).toBe(node);
			expect(getNode(node.id)).toBe(node);
		}
		expect(getNode("does-not-exist")).toBeUndefined();
	});

	it("only references prerequisites that exist", () => {
		for (const node of UPGRADE_NODES) {
			for (const prereq of node.prerequisites) {
				expect(UPGRADE_NODE_BY_ID[prereq]).toBeDefined();
			}
		}
	});

	it("has exactly one era-1 root with no prerequisites", () => {
		const roots = UPGRADE_NODES.filter((n) => n.prerequisites.length === 0);
		expect(roots).toHaveLength(1);
		expect(roots[0].id).toBe("research_hut");
		expect(roots[0].era).toBe(1);
		expect(roots[0].unlocks.buildings).toContain("research_hut");
	});

	it("never orders a node before any of its prerequisites (era-monotonic)", () => {
		for (const node of UPGRADE_NODES) {
			for (const prereq of node.prerequisites) {
				const parent = UPGRADE_NODE_BY_ID[prereq];
				expect(parent.era).toBeLessThanOrEqual(node.era);
			}
		}
	});

	it("uses only registered effect keys", () => {
		for (const node of UPGRADE_NODES) {
			for (const effect of node.unlocks.effects ?? []) {
				expect(EFFECT_KINDS[effect.key]).toBeDefined();
			}
		}
	});
});

describe("canUnlock / prerequisite gating", () => {
	it("gates a node until all prerequisites are owned", () => {
		const fresh = createUpgradeTreeState();
		expect(canUnlock(fresh, "research_hut")).toBe(true);
		expect(canUnlock(fresh, "basic_tools")).toBe(false);

		const withRoot = stateWith(["research_hut"]);
		expect(canUnlock(withRoot, "basic_tools")).toBe(true);
	});

	it("requires every prerequisite, not just one", () => {
		// smithy <- sawmill <- foraging_lore <- basic_tools <- research_hut
		const partial = stateWith(["research_hut", "basic_tools", "foraging_lore"]);
		expect(canUnlock(partial, "sawmill")).toBe(true);
		expect(canUnlock(partial, "smithy")).toBe(false);
	});

	it("rejects unknown nodes and already-owned nodes", () => {
		expect(canUnlock(createUpgradeTreeState(), "nope")).toBe(false);
		expect(canUnlock(stateWith(["research_hut"]), "research_hut")).toBe(false);
	});

	it("lists unlockable nodes in stable definition order", () => {
		const s = stateWith(["research_hut"]);
		const ids = unlockableNodes(s).map((n) => n.id);
		expect(ids).toEqual(["basic_tools", "water_carriers", "den_insulation"]);
	});
});

describe("godPurchase", () => {
	it("buys an unlockable node at its cost and marks it owned", () => {
		const result = godPurchase(createUpgradeTreeState(), "research_hut");
		expect(result.ok).toBe(true);
		expect(result.blessingsCost).toBe(5);
		expect(result.state.ownedNodeIds).toEqual(["research_hut"]);
		// God purchases never touch the research pool.
		expect(result.state.researchPoints).toBe(0);
	});

	it("rejects a double purchase and leaves state untouched", () => {
		const owned = stateWith(["research_hut"], 3);
		const result = godPurchase(owned, "research_hut");
		expect(result.ok).toBe(false);
		expect(result.reason).toBe("already-owned");
		expect(result.blessingsCost).toBe(0);
		expect(result.state).toBe(owned);
	});

	it("rejects a purchase with unmet prerequisites", () => {
		const result = godPurchase(createUpgradeTreeState(), "smithy");
		expect(result.ok).toBe(false);
		expect(result.reason).toBe("prerequisites-unmet");
	});

	it("rejects an unknown node", () => {
		const result = godPurchase(createUpgradeTreeState(), "ghost");
		expect(result.ok).toBe(false);
		expect(result.reason).toBe("unknown-node");
	});

	it("does not mutate the input state", () => {
		const input = createUpgradeTreeState();
		godPurchase(input, "research_hut");
		expect(input.ownedNodeIds).toEqual([]);
	});
});

describe("research accrual math", () => {
	it("derives the per-second rate from ~10 points/week", () => {
		expect(WEEK_SECONDS).toBe(604800);
		expect(RESEARCH_POINTS_PER_SECOND).toBeCloseTo(10 / 604800, 12);
	});

	it("accrues ~10 points over a week with one researcher", () => {
		const points = pointsPerTickFor(1, WEEK_SECONDS);
		expect(points).toBeCloseTo(RESEARCH_POINTS_PER_RESEARCHER_PER_WEEK, 9);
	});

	it("scales linearly with researcher count and elapsed time", () => {
		const one = pointsPerTickFor(1, 3600);
		expect(pointsPerTickFor(2, 3600)).toBeCloseTo(one * 2, 12);
		expect(pointsPerTickFor(1, 7200)).toBeCloseTo(one * 2, 12);
	});

	it("applies a research-rate multiplier", () => {
		const base = pointsPerTickFor(1, WEEK_SECONDS);
		expect(pointsPerTickFor(1, WEEK_SECONDS, 1.5)).toBeCloseTo(base * 1.5, 9);
	});

	it("produces nothing without researchers or elapsed time", () => {
		expect(pointsPerTickFor(0, WEEK_SECONDS)).toBe(0);
		expect(pointsPerTickFor(1, 0)).toBe(0);
		expect(pointsPerTickFor(-3, 10)).toBe(0);
	});

	it("accrueResearch adds to and never drives the pool below zero", () => {
		expect(accrueResearch(stateWith([], 4), 2.5).researchPoints).toBe(6.5);
		expect(accrueResearch(stateWith([], 1), -5).researchPoints).toBe(0);
		expect(accrueResearch(stateWith([], 4), 0).researchPoints).toBe(4);
	});
});

describe("catAutoUnlock determinism", () => {
	it("unlocks the cheapest affordable node and deducts its cost", () => {
		// research_hut (5) is the only unlockable node; 6 points is enough.
		const result = catAutoUnlock(stateWith([], 6));
		expect(result.ok).toBe(true);
		expect(result.nodeId).toBe("research_hut");
		expect(result.state.ownedNodeIds).toEqual(["research_hut"]);
		expect(result.state.researchPoints).toBe(1);
	});

	it("does nothing when no unlockable node is affordable", () => {
		const poor = stateWith(["research_hut"], 4); // cheapest unlockable is 5
		const result = catAutoUnlock(poor);
		expect(result.ok).toBe(false);
		expect(result.nodeId).toBeNull();
		expect(result.state).toBe(poor);
	});

	it("breaks cost ties by ascending id", () => {
		// After research_hut: basic_tools(5) and water_carriers(8),
		// den_insulation(8). With 8 points the cheapest is basic_tools(5).
		const s = stateWith(["research_hut"], 8);
		const first = catAutoUnlock(s);
		expect(first.nodeId).toBe("basic_tools");
		// Now 3 points left, nothing affordable.
		expect(catAutoUnlock(first.state).ok).toBe(false);
	});

	it("picks the lexicographically smaller id among equal-cost candidates", () => {
		// With research_hut + basic_tools + foraging_lore owned, the cheapest
		// affordable unlockable nodes at 8 points are den_insulation(8) and
		// water_carriers(8); "den_insulation" < "water_carriers" wins the tie.
		const s = stateWith(["research_hut", "basic_tools", "foraging_lore"], 8);
		const result = catAutoUnlock(s);
		expect(result.nodeId).toBe("den_insulation");
	});

	it("nextResearchTarget reports the cheapest target ignoring the balance", () => {
		const broke = stateWith(["research_hut"], 0);
		expect(nextResearchTarget(broke)?.id).toBe("basic_tools");
		expect(
			nextResearchTarget(stateWith(UPGRADE_NODES.map((n) => n.id))),
		).toBeNull();
	});
});

describe("resolveEffects aggregation", () => {
	it("returns neutral modifiers for no owned nodes", () => {
		const resolved = resolveEffects([]);
		expect(resolved).toEqual(neutralEffects());
		expect(resolved.huntYieldMult).toBe(1);
		expect(resolved.housingPerDen).toBe(0);
	});

	it("resolves mult effects as 1 + sum and add effects as sum", () => {
		// basic_tools huntYieldMult 0.1; den_insulation + housing_tier_2 +
		// grand_housing housingPerDen 1+2+3.
		const resolved = resolveEffects([
			"basic_tools",
			"den_insulation",
			"housing_tier_2",
			"grand_housing",
		]);
		expect(resolved.huntYieldMult).toBeCloseTo(1.1, 12);
		expect(resolved.housingPerDen).toBe(6);
	});

	it("stacks multiple mult effects on the same key additively", () => {
		// school researchRateMult 0.5 + scholars_guild 0.75 => 1 + 1.25
		const resolved = resolveEffects(["school", "scholars_guild"]);
		expect(resolved.researchRateMult).toBeCloseTo(2.25, 12);
	});

	it("ignores unknown ids and nodes without effects", () => {
		const resolved = resolveEffects(["ghost", "research_hut", "smithy"]);
		expect(resolved).toEqual(neutralEffects());
	});
});

describe("serialization round-trip", () => {
	it("round-trips a populated state", () => {
		const state = stateWith(["research_hut", "basic_tools"], 12.5);
		const restored = deserializeUpgradeTreeState(
			serializeUpgradeTreeState(state),
		);
		expect(restored).toEqual(state);
	});

	it("serialize returns a detached copy", () => {
		const state = stateWith(["research_hut"], 3);
		const dto = serializeUpgradeTreeState(state);
		dto.ownedNodeIds.push("basic_tools");
		expect(state.ownedNodeIds).toEqual(["research_hut"]);
	});

	it("fills defaults for missing or malformed input", () => {
		expect(deserializeUpgradeTreeState(undefined)).toEqual(
			createUpgradeTreeState(),
		);
		expect(deserializeUpgradeTreeState(null)).toEqual(createUpgradeTreeState());
		expect(deserializeUpgradeTreeState("nope")).toEqual(
			createUpgradeTreeState(),
		);
		expect(deserializeUpgradeTreeState({})).toEqual(createUpgradeTreeState());
	});

	it("drops unknown ids, duplicates, and clamps a bad point value", () => {
		const restored = deserializeUpgradeTreeState({
			ownedNodeIds: ["research_hut", "research_hut", "ghost", 42],
			researchPoints: -5,
		});
		expect(restored.ownedNodeIds).toEqual(["research_hut"]);
		expect(restored.researchPoints).toBe(0);
	});

	it("defaults a non-numeric point value to zero", () => {
		const restored = deserializeUpgradeTreeState({
			ownedNodeIds: [],
			researchPoints: "lots",
		});
		expect(restored.researchPoints).toBe(0);
	});
});
