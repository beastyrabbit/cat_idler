import { describe, expect, it } from "vitest";

import {
	type LeaderDecision,
	type LeaderSnapshot,
	planLeaderActions,
	targetWarriors,
	WARRIOR_MAX_RATIO,
	WARRIOR_TARGET_BY_BAND,
} from "@/lib/game/leaderAI";

const CAP = 200;

/** A calm mid-sized colony; override fields per scenario. */
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

function kinds(decisions: LeaderDecision[]): string[] {
	return decisions.map((d) => d.kind);
}

function count(decisions: LeaderDecision[], kind: string): number {
	const d = decisions.find((x) => x.kind === kind);
	return d && "count" in d ? d.count : 0;
}

describe("planLeaderActions (thin delegation to the director)", () => {
	it("returns no labour decisions for an empty colony", () => {
		expect(
			planLeaderActions(
				snap({
					population: 0,
					idleCats: 0,
					workforce: 0,
					resources: { food: 100, refined: 0 },
				}),
			),
		).toEqual([]);
	});

	it("dispatches hunts when the larder is low", () => {
		expect(
			count(
				planLeaderActions(snap({ resources: { food: CAP * 0.3, refined: 0 } })),
				"hunt",
			),
		).toBeGreaterThan(0);
	});

	it("fetches water when the reservoir runs low and a site is known", () => {
		const fetch = count(
			planLeaderActions(snap({ water: CAP * 0.2, hasWaterSite: true })),
			"fetch_water",
		);
		expect(fetch).toBeGreaterThan(0);
	});

	it("never fetches water without a known site (veto gate)", () => {
		expect(
			kinds(planLeaderActions(snap({ water: 0, hasWaterSite: false }))),
		).not.toContain("fetch_water");
	});

	it("orders cancellations before labour before capital projects", () => {
		// Overflowing stores with crowding: cancel_hunts first, then any labour,
		// then the build/tithe decisions.
		const decisions = planLeaderActions(
			snap({
				activeHunts: 4,
				idleCats: 4,
				resources: { food: CAP * 1.2, refined: 10 },
				housing: { capacity: 10, committed: 0 },
			}),
		);
		expect(decisions[0].kind).toBe("cancel_hunts");
		const order = kinds(decisions);
		expect(order).toContain("build_den");
		expect(order).toContain("tithe");
		// A capital/offering decision never precedes the cancellation.
		expect(order.indexOf("cancel_hunts")).toBeLessThan(order.indexOf("tithe"));
	});

	it("commissions a den under crowding pressure", () => {
		expect(
			kinds(
				planLeaderActions(snap({ housing: { capacity: 25, committed: 0 } })),
			),
		).toContain("build_den");
	});
});

describe("targetWarriors", () => {
	const calm = (overrides: Partial<LeaderSnapshot> = {}) =>
		snap({ resources: { food: CAP, refined: 0 }, ...overrides });

	it("is zero without a barracks", () => {
		expect(targetWarriors(calm({ hasBarracks: false }))).toBe(0);
	});

	it("scales with the threat band", () => {
		const rising = targetWarriors(
			calm({ hasBarracks: true, threatBand: "rising", workforce: 40 }),
		);
		const imminent = targetWarriors(
			calm({ hasBarracks: true, threatBand: "imminent", workforce: 40 }),
		);
		expect(rising).toBe(WARRIOR_TARGET_BY_BAND.rising);
		expect(imminent).toBe(WARRIOR_TARGET_BY_BAND.imminent);
		expect(imminent).toBeGreaterThan(rising);
	});

	it("caps the guard at the workforce fraction", () => {
		const capped = targetWarriors(
			calm({ hasBarracks: true, threatBand: "imminent", workforce: 4 }),
		);
		expect(capped).toBe(Math.floor(4 * WARRIOR_MAX_RATIO));
	});
});
