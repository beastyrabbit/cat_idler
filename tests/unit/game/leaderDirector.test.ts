import { describe, expect, it } from "vitest";

import type { LeaderDecision, LeaderSnapshot } from "@/lib/game/leaderAI";
import {
	assignmentFit,
	type CatBrief,
	clamp01,
	combineOr,
	deficitCurve,
	directColony,
	matchCatsToSlots,
	type OpenSlots,
	pressureCurve,
	projectionCurve,
	projectionGate,
	surplusCurve,
	survivalScore,
} from "@/lib/game/leaderDirector";

const CAP = 200;

/** A calm mid-sized colony with full stores; override per scenario. */
function snap(overrides: Partial<LeaderSnapshot> = {}): LeaderSnapshot {
	return {
		population: 20,
		workforce: 20,
		idleCats: 10,
		employedCats: 0,
		resources: { food: CAP, refined: 0 },
		foodCapacity: CAP,
		materials: CAP,
		materialsCapacity: CAP,
		water: CAP,
		waterCapacity: CAP,
		housing: { capacity: 40, committed: 0 },
		activeHunts: 0,
		activeQuarries: 0,
		activeScouts: 0,
		activeWaterFetchers: 0,
		hasQuarrySite: false,
		hasWaterSite: false,
		hasFrontier: false,
		denPlansInFlight: 0,
		storagePlansInFlight: 0,
		storehouseCount: 0,
		storehouseCap: 3,
		workshopsNeedingWorkers: 0,
		...overrides,
	};
}

function slotFor(slots: OpenSlots[], goal: string): number {
	return slots.find((s) => s.goal === goal)?.count ?? 0;
}

function decisionKinds(decisions: LeaderDecision[]): string[] {
	return decisions.map((d) => d.kind);
}

describe("response curves", () => {
	it("clamps to [0,1]", () => {
		expect(clamp01(-2)).toBe(0);
		expect(clamp01(0.5)).toBe(0.5);
		expect(clamp01(2)).toBe(1);
	});

	it("deficitCurve is 0 at/above full, 1 at empty, convex between", () => {
		expect(deficitCurve(1)).toBe(0);
		expect(deficitCurve(1.5)).toBe(0);
		expect(deficitCurve(0)).toBe(1);
		expect(deficitCurve(0.5)).toBeCloseTo(0.25, 6); // (1-0.5)^2
		// Convex: urgency ramps harder near empty than a straight line.
		expect(deficitCurve(0.25)).toBeGreaterThan(0.5);
	});

	it("projectionCurve rises as a store nears empty within the horizon", () => {
		expect(projectionCurve(100, 0)).toBe(0); // not draining
		expect(projectionCurve(100, -5)).toBe(0); // negative drain = filling
		// Empties in 1 tick (horizon 6) → high urgency.
		expect(projectionCurve(10, 10)).toBeCloseTo(1 - 1 / 6, 6);
		// Empties in >horizon ticks → no urgency yet.
		expect(projectionCurve(600, 10)).toBe(0);
	});

	it("projectionGate suppresses the lookahead on a brimming store", () => {
		expect(projectionGate(1)).toBe(0); // full → never a crisis
		expect(projectionGate(0.9)).toBe(0);
		expect(projectionGate(0)).toBe(1); // empty → full weight
		expect(projectionGate(0.45)).toBeCloseTo(0.5, 6);
	});

	it("survivalScore ignores fast drain while full but reacts once low", () => {
		// Full and draining hard: gate suppresses the panic.
		expect(survivalScore(1, CAP, 9999)).toBe(0);
		// Low and draining hard: urgent.
		expect(survivalScore(0.3, 60, 40)).toBeGreaterThan(0.5);
	});

	it("pressureCurve flips decisively around its centre", () => {
		expect(pressureCurve(0.8)).toBeCloseTo(0.5, 6);
		expect(pressureCurve(0.4)).toBeLessThan(0.05);
		expect(pressureCurve(1.2)).toBeGreaterThan(0.95);
	});

	it("surplusCurve is 0 up to the threshold then ramps to 1", () => {
		expect(surplusCurve(0.5, 0.6)).toBe(0);
		expect(surplusCurve(0.6, 0.6)).toBe(0);
		expect(surplusCurve(1, 0.6)).toBe(1);
		expect(surplusCurve(0.8, 0.6)).toBeCloseTo(0.5, 6);
	});

	it("combineOr keeps [0,1] and lets either term drive it high", () => {
		expect(combineOr(0, 0)).toBe(0);
		expect(combineOr(1, 0)).toBe(1);
		expect(combineOr(0, 1)).toBe(1);
		expect(combineOr(0.5, 0.5)).toBeCloseTo(0.75, 6);
	});
});

describe("quota allocation", () => {
	it("pulls the whole idle pool onto hunts in a famine", () => {
		const slots = directColony(
			snap({ resources: { food: 0, refined: 0 }, idleCats: 10 }),
		).slots;
		// Hunt is capped at ceil(budget*0.7) = ceil(14*0.7)=10 slots.
		expect(slotFor(slots, "hunt")).toBeGreaterThanOrEqual(9);
	});

	it("serves a water crisis before a storehouse (cross-axis trade-off)", () => {
		const plan = directColony(
			snap({
				water: CAP * 0.15,
				hasWaterSite: true,
				resources: { food: CAP * 0.95, refined: 0 }, // stores brimming
				idleCats: 6,
			}),
		);
		// Water is urgent; it gets the top labour slot...
		expect(plan.slots[0].goal).toBe("fetch_water");
		expect(slotFor(plan.slots, "fetch_water")).toBeGreaterThan(0);
		// ...and the storehouse is still commissioned as a standalone capital job.
		expect(decisionKinds(plan.decisions)).toContain("build_storage");
	});

	it("keeps a single-slot workshop from being rounded away", () => {
		const slots = directColony(
			snap({ workshopsNeedingWorkers: 1, idleCats: 4 }),
		).slots;
		expect(slotFor(slots, "assign_workshop")).toBe(1);
	});

	it("vetoes goals with no site or frontier", () => {
		const slots = directColony(
			snap({
				water: 0,
				materials: 0,
				hasWaterSite: false,
				hasQuarrySite: false,
				hasFrontier: false,
			}),
		).slots;
		expect(slotFor(slots, "fetch_water")).toBe(0);
		expect(slotFor(slots, "quarry")).toBe(0);
		expect(slotFor(slots, "scout")).toBe(0);
	});

	it("tops water fetchers up to the gap, not past what's in flight", () => {
		const slots = directColony(
			snap({ water: CAP * 0.1, hasWaterSite: true, activeWaterFetchers: 3 }),
		).slots;
		// Cap is 4; 3 already out → at most 1 more.
		expect(slotFor(slots, "fetch_water")).toBeLessThanOrEqual(1);
	});
});

describe("near-zero idle", () => {
	it("fills leftover idle cats with low-priority work up to the floor", () => {
		// Full stores (no hunts), a frontier to scout: idle cats should be put
		// to work rather than left standing.
		const plan = directColony(
			snap({
				resources: { food: CAP, refined: 0 },
				idleCats: 12,
				employedCats: 0,
				hasFrontier: true,
			}),
		);
		const employed = plan.slots.reduce((sum, s) => sum + s.count, 0);
		// At least 80% of the 12 able cats get work.
		expect(employed).toBeGreaterThanOrEqual(Math.ceil(12 * 0.8));
	});

	it("does not invent work when none is available", () => {
		const plan = directColony(
			snap({
				resources: { food: CAP, refined: 0 },
				idleCats: 12,
				hasFrontier: false,
				hasQuarrySite: false,
			}),
		);
		// Nothing to do (full food, no frontier, no quarry) → no fill slots.
		expect(plan.slots.reduce((sum, s) => sum + s.count, 0)).toBe(0);
	});

	it("does not pad outbound work while threat is imminent", () => {
		const calm = directColony(
			snap({
				resources: { food: CAP * 0.9, refined: 0 },
				idleCats: 12,
				hasFrontier: true,
				hasQuarrySite: true,
				threatBand: "calm",
			}),
		).slots;
		const imminent = directColony(
			snap({
				resources: { food: CAP * 0.9, refined: 0 },
				idleCats: 12,
				hasFrontier: true,
				hasQuarrySite: true,
				threatBand: "imminent",
			}),
		).slots;
		const activeRaid = directColony(
			snap({
				resources: { food: CAP * 0.9, refined: 0 },
				idleCats: 12,
				hasFrontier: true,
				hasQuarrySite: true,
				threatBand: "calm",
				raidActive: true,
			}),
		).slots;

		const outbound = (slots: OpenSlots[]) =>
			slotFor(slots, "hunt") +
			slotFor(slots, "scout") +
			slotFor(slots, "quarry");
		expect(outbound(calm)).toBeGreaterThan(outbound(imminent));
		expect(outbound(imminent)).toBe(0);
		expect(outbound(activeRaid)).toBe(0);
	});
});

describe("standalone decisions", () => {
	it("cancels hunts only when stores overflow past the cancel ratio", () => {
		expect(
			decisionKinds(
				directColony(
					snap({ activeHunts: 4, resources: { food: CAP * 1.05, refined: 0 } }),
				).decisions,
			),
		).not.toContain("cancel_hunts");
		expect(
			decisionKinds(
				directColony(
					snap({ activeHunts: 4, resources: { food: CAP * 1.2, refined: 0 } }),
				).decisions,
			),
		).toContain("cancel_hunts");
	});

	it("cancels training when the larder is bare", () => {
		expect(
			decisionKinds(
				directColony(snap({ starving: true, trainingInFlight: 2 })).decisions,
			),
		).toContain("cancel_training");
	});

	it("commissions a den at or above the pressure threshold", () => {
		// 20 cats into 25 shelter → pressure exactly 0.8.
		expect(
			decisionKinds(
				directColony(snap({ housing: { capacity: 25, committed: 0 } }))
					.decisions,
			),
		).toContain("build_den");
	});

	it("stops building storehouses at the cap", () => {
		const overflowing = { food: CAP * 0.95, refined: 0 };
		expect(
			decisionKinds(
				directColony(
					snap({
						resources: overflowing,
						storehouseCount: 3,
						storehouseCap: 3,
					}),
				).decisions,
			),
		).not.toContain("build_storage");
	});

	it("tithes food and refined surplus into blessings", () => {
		const tithe = directColony(
			snap({ resources: { food: CAP, refined: 10 } }),
		).decisions.find((d) => d.kind === "tithe");
		expect(tithe).toEqual({
			kind: "tithe",
			food: 20,
			refined: 5,
			blessings: 2,
		});
	});
});

describe("assignment matcher", () => {
	function cat(
		id: string,
		stats: Partial<CatBrief["stats"]>,
		specialization: CatBrief["specialization"] = null,
	): CatBrief {
		return {
			id,
			specialization,
			stats: {
				hunting: 30,
				building: 30,
				vision: 30,
				medicine: 30,
				attack: 30,
				defense: 30,
				leadership: 30,
				...stats,
			},
		};
	}

	it("sends the best hunter to a hunt slot", () => {
		const slots: OpenSlots[] = [{ goal: "hunt", count: 1, score: 1 }];
		const cats = [cat("a", { hunting: 40 }), cat("b", { hunting: 90 })];
		expect(matchCatsToSlots(slots, cats)).toEqual([
			{ catId: "b", goal: "hunt" },
		]);
	});

	it("weights a matching specialization above raw skill", () => {
		// The generalist has more raw hunting, but the hunter's 1.5x bonus wins.
		const slots: OpenSlots[] = [{ goal: "hunt", count: 1, score: 1 }];
		const cats = [
			cat("gen", { hunting: 70 }),
			cat("spec", { hunting: 50 }, "hunter"),
		];
		expect(assignmentFit(cats[1], "hunt")).toBeGreaterThan(
			assignmentFit(cats[0], "hunt"),
		);
		expect(matchCatsToSlots(slots, cats)[0].catId).toBe("spec");
	});

	it("fills higher-priority slots first with the best-fit cats", () => {
		// Water outranks scouting; the sharp-eyed cat should still be free for
		// the scout slot because the sturdy one takes water first.
		const slots: OpenSlots[] = [
			{ goal: "fetch_water", count: 1, score: 0.9 },
			{ goal: "scout", count: 1, score: 0.3 },
		];
		const cats = [
			cat("sturdy", { hunting: 90, vision: 10 }),
			cat("scout", { hunting: 20, vision: 90 }),
		];
		const result = matchCatsToSlots(slots, cats);
		expect(result).toEqual([
			{ catId: "sturdy", goal: "fetch_water" },
			{ catId: "scout", goal: "scout" },
		]);
	});

	it("breaks ties by stable id order, never randomly", () => {
		const slots: OpenSlots[] = [{ goal: "hunt", count: 1, score: 1 }];
		const cats = [cat("z", { hunting: 50 }), cat("a", { hunting: 50 })];
		// Equal fit → the first in the pool (z) wins, deterministically.
		expect(matchCatsToSlots(slots, cats)[0].catId).toBe("z");
	});

	it("never enrolls an existing warrior into training", () => {
		const slots: OpenSlots[] = [{ goal: "train_warrior", count: 1, score: 1 }];
		const cats = [
			cat("vet", { attack: 90, defense: 90 }, "warrior"),
			cat("rookie", { attack: 40, defense: 40 }),
		];
		expect(
			matchCatsToSlots(slots, cats, { excludeWarriorsFromTraining: true }),
		).toEqual([{ catId: "rookie", goal: "train_warrior" }]);
	});

	it("assigns each cat at most once across slots", () => {
		const slots: OpenSlots[] = [
			{ goal: "hunt", count: 2, score: 1 },
			{ goal: "scout", count: 2, score: 0.3 },
		];
		const cats = [cat("a", {}), cat("b", {}), cat("c", {})];
		const result = matchCatsToSlots(slots, cats);
		expect(result).toHaveLength(3); // only three cats to give
		expect(new Set(result.map((r) => r.catId)).size).toBe(3);
	});
});

describe("determinism", () => {
	it("returns an identical plan for an identical snapshot", () => {
		const s = snap({
			water: CAP * 0.3,
			hasWaterSite: true,
			hasFrontier: true,
			resources: { food: CAP * 0.4, refined: 0 },
			idleCats: 8,
		});
		expect(directColony(s)).toEqual(directColony(s));
	});
});
