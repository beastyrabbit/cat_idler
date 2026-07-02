import { describe, expect, it } from "vitest";

import {
	DEN_PRESSURE_THRESHOLD,
	EMPLOYMENT_TARGET_RATIO,
	HUNT_CANCEL_RATIO,
	HUNT_HOLD_RATIO,
	type LeaderDecision,
	type LeaderSnapshot,
	planLeaderActions,
	QUARRY_HOLD_RATIO,
	QUARRY_LOW_RATIO,
	SCOUT_TARGET,
	STORAGE_RATIO,
	TITHE_FOOD_AMOUNT,
	TITHE_FOOD_RATIO,
	TITHE_REFINED_AMOUNT,
	targetHuntSlots,
	targetWarriors,
	WARRIOR_MAX_RATIO,
	WARRIOR_TARGET_BY_BAND,
} from "@/lib/game/leaderAI";

const CAP = 200;

/** A calm mid-sized colony; override fields per scenario. */
function snap(overrides: Partial<LeaderSnapshot> = {}): LeaderSnapshot {
	return {
		population: 20,
		idleCats: 10,
		employedCats: 0,
		resources: { food: 100, refined: 0 },
		foodCapacity: CAP,
		materials: 100,
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

function huntCount(decisions: LeaderDecision[]): number {
	const hunt = decisions.find((d) => d.kind === "hunt");
	return hunt && hunt.kind === "hunt" ? hunt.count : 0;
}

function quarryCount(decisions: LeaderDecision[]): number {
	const quarry = decisions.find((d) => d.kind === "quarry");
	return quarry && quarry.kind === "quarry" ? quarry.count : 0;
}

function scoutCount(decisions: LeaderDecision[]): number {
	const scout = decisions.find((d) => d.kind === "scout");
	return scout && scout.kind === "scout" ? scout.count : 0;
}

describe("leaderAI", () => {
	describe("targetHuntSlots", () => {
		it("returns zero at or above the hold ratio", () => {
			expect(
				targetHuntSlots(
					snap({ resources: { food: CAP * HUNT_HOLD_RATIO, refined: 0 } }),
				),
			).toBe(0);
			expect(
				targetHuntSlots(snap({ resources: { food: CAP, refined: 0 } })),
			).toBe(0);
			expect(
				targetHuntSlots(snap({ resources: { food: CAP * 1.5, refined: 0 } })),
			).toBe(0);
		});

		it("caps at half the colony when stores are empty", () => {
			const slots = targetHuntSlots(
				snap({ resources: { food: 0, refined: 0 } }),
			);
			expect(slots).toBe(Math.floor(20 * EMPLOYMENT_TARGET_RATIO));
			expect(slots).toBe(10);
		});

		it("scales with the food deficit", () => {
			// 45% of cap -> deficit 0.5 of the 0..0.9 band -> half of max slots.
			expect(
				targetHuntSlots(snap({ resources: { food: CAP * 0.45, refined: 0 } })),
			).toBe(5);
		});

		it("yields zero slots for empty and single-cat colonies", () => {
			expect(
				targetHuntSlots(
					snap({ population: 0, resources: { food: 0, refined: 0 } }),
				),
			).toBe(0);
			expect(
				targetHuntSlots(
					snap({ population: 1, resources: { food: 0, refined: 0 } }),
				),
			).toBe(0);
		});

		it("dispatches half of a twenty-cat colony at zero food", () => {
			expect(
				targetHuntSlots(
					snap({ population: 20, resources: { food: 0, refined: 0 } }),
				),
			).toBe(10);
		});
	});

	describe("hunt dispatch", () => {
		it("plans hunts to reach the target from zero", () => {
			expect(huntCount(planLeaderActions(snap()))).toBe(5);
		});

		it("plans only the gap up to the target", () => {
			expect(huntCount(planLeaderActions(snap({ activeHunts: 3 })))).toBe(2);
		});

		it("plans nothing once the target is met", () => {
			expect(kinds(planLeaderActions(snap({ activeHunts: 5 })))).not.toContain(
				"hunt",
			);
		});

		it("never sends more than the idle cats available", () => {
			expect(huntCount(planLeaderActions(snap({ idleCats: 1 })))).toBe(1);
		});

		it("never pushes total employment past half the colony", () => {
			// 8 already employed -> only room for 2 more before hitting 10.
			expect(huntCount(planLeaderActions(snap({ employedCats: 8 })))).toBe(2);
			expect(
				kinds(planLeaderActions(snap({ employedCats: 10 }))),
			).not.toContain("hunt");
		});
	});

	describe("hunt hysteresis", () => {
		it("holds without dispatching or cancelling inside the 90-110% band", () => {
			const holding = planLeaderActions(
				snap({ activeHunts: 5, resources: { food: CAP * 0.95, refined: 0 } }),
			);
			expect(kinds(holding)).not.toContain("hunt");
			expect(kinds(holding)).not.toContain("cancel_hunts");

			const stillHolding = planLeaderActions(
				snap({ activeHunts: 5, resources: { food: CAP * 1.05, refined: 0 } }),
			);
			expect(kinds(stillHolding)).not.toContain("hunt");
			expect(kinds(stillHolding)).not.toContain("cancel_hunts");
		});

		it("only cancels once stores climb above the cancel ratio", () => {
			// At exactly the cancel ratio it still holds.
			expect(
				kinds(
					planLeaderActions(
						snap({
							activeHunts: 5,
							resources: { food: CAP * HUNT_CANCEL_RATIO, refined: 0 },
						}),
					),
				),
			).not.toContain("cancel_hunts");

			expect(
				kinds(
					planLeaderActions(
						snap({
							activeHunts: 5,
							resources: { food: CAP * 1.2, refined: 0 },
						}),
					),
				),
			).toContain("cancel_hunts");
		});

		it("re-dispatches once food falls back below the hold ratio", () => {
			expect(
				huntCount(
					planLeaderActions(
						snap({
							activeHunts: 0,
							resources: { food: CAP * 0.5, refined: 0 },
						}),
					),
				),
			).toBeGreaterThan(0);
		});

		it("does not cancel when no hunts are out", () => {
			expect(
				kinds(
					planLeaderActions(
						snap({
							activeHunts: 0,
							resources: { food: CAP * 1.5, refined: 0 },
						}),
					),
				),
			).not.toContain("cancel_hunts");
		});
	});

	describe("storehouse commissioning", () => {
		it("commissions storage above 90% of capacity", () => {
			expect(
				kinds(
					planLeaderActions(
						snap({ resources: { food: CAP * 0.91, refined: 0 } }),
					),
				),
			).toContain("build_storage");
		});

		it("does not commission storage at exactly 90%", () => {
			expect(
				kinds(
					planLeaderActions(
						snap({ resources: { food: CAP * STORAGE_RATIO, refined: 0 } }),
					),
				),
			).not.toContain("build_storage");
		});

		it("never queues a second storehouse while one is pending", () => {
			expect(
				kinds(
					planLeaderActions(
						snap({
							storagePlansInFlight: 1,
							resources: { food: CAP * 0.95, refined: 0 },
						}),
					),
				),
			).not.toContain("build_storage");
		});
	});

	describe("den planning", () => {
		it("plans a den at or above the pressure threshold", () => {
			// 20 cats into 25 shelter -> pressure exactly 0.8.
			expect(
				kinds(
					planLeaderActions(snap({ housing: { capacity: 25, committed: 0 } })),
				),
			).toContain("build_den");
			expect(DEN_PRESSURE_THRESHOLD).toBe(0.8);
		});

		it("counts committed shelter against the pressure", () => {
			// 20 cats into 20 + 10 committed -> pressure 0.667, no den.
			expect(
				kinds(
					planLeaderActions(snap({ housing: { capacity: 20, committed: 10 } })),
				),
			).not.toContain("build_den");
		});

		it("never plans a second den while one is in flight", () => {
			expect(
				kinds(
					planLeaderActions(
						snap({
							denPlansInFlight: 1,
							housing: { capacity: 10, committed: 0 },
						}),
					),
				),
			).not.toContain("build_den");
		});

		it("plans a den when there is no shelter at all", () => {
			expect(
				kinds(
					planLeaderActions(snap({ housing: { capacity: 0, committed: 0 } })),
				),
			).toContain("build_den");
		});
	});

	describe("workshop staffing", () => {
		it("staffs idle cats left after hunts are dispatched", () => {
			// Base food dispatches 5 hunts from 10 idle, leaving 5 for workshops.
			const decisions = planLeaderActions(snap({ workshopsNeedingWorkers: 2 }));
			const assign = decisions.find((d) => d.kind === "assign_workshop");
			expect(assign).toEqual({ kind: "assign_workshop", count: 2 });
		});

		it("skips staffing when hunts consume all the idle cats", () => {
			expect(
				kinds(
					planLeaderActions(
						snap({
							idleCats: 5,
							workshopsNeedingWorkers: 3,
							resources: { food: 0, refined: 0 },
						}),
					),
				),
			).not.toContain("assign_workshop");
		});

		it("staffs up to the number of workshops needing workers", () => {
			const decisions = planLeaderActions(
				snap({
					idleCats: 4,
					workshopsNeedingWorkers: 2,
					resources: { food: CAP * 0.95, refined: 0 },
				}),
			);
			const assign = decisions.find((d) => d.kind === "assign_workshop");
			expect(assign).toEqual({ kind: "assign_workshop", count: 2 });
		});
	});

	describe("research staffing", () => {
		it("staffs a researcher when a hut needs one and stores are comfortable", () => {
			const decisions = planLeaderActions(
				snap({
					researchHutsNeedingWorkers: 1,
					idleCats: 6,
					// Full stores: no hunts dispatched, food/water comfortable.
					resources: { food: CAP, refined: 0 },
					water: CAP,
				}),
			);
			const research = decisions.find((d) => d.kind === "assign_research");
			expect(research).toEqual({ kind: "assign_research", count: 1 });
		});

		it("leaves the hut empty when food is below the comfort ratio", () => {
			const decisions = planLeaderActions(
				snap({
					researchHutsNeedingWorkers: 1,
					idleCats: 6,
					resources: { food: CAP * 0.2, refined: 0 },
					water: CAP,
				}),
			);
			expect(kinds(decisions)).not.toContain("assign_research");
		});

		it("leaves the hut empty when water is below the comfort ratio", () => {
			const decisions = planLeaderActions(
				snap({
					researchHutsNeedingWorkers: 1,
					idleCats: 6,
					resources: { food: CAP, refined: 0 },
					water: CAP * 0.2,
				}),
			);
			expect(kinds(decisions)).not.toContain("assign_research");
		});
	});

	describe("tithing", () => {
		it("tithes food only above 60% of capacity plus the tithe amount", () => {
			const threshold = CAP * TITHE_FOOD_RATIO + TITHE_FOOD_AMOUNT;
			expect(
				kinds(
					planLeaderActions(
						snap({ resources: { food: threshold, refined: 0 } }),
					),
				),
			).not.toContain("tithe");

			const above = planLeaderActions(
				snap({ resources: { food: threshold + 1, refined: 0 } }),
			).find((d) => d.kind === "tithe");
			expect(above).toEqual({
				kind: "tithe",
				food: TITHE_FOOD_AMOUNT,
				refined: 0,
				blessings: 1,
			});
		});

		it("tithes refined goods once five are in store", () => {
			expect(
				planLeaderActions(snap({ resources: { food: 100, refined: 4 } })).find(
					(d) => d.kind === "tithe",
				),
			).toBeUndefined();

			expect(
				planLeaderActions(snap({ resources: { food: 100, refined: 5 } })).find(
					(d) => d.kind === "tithe",
				),
			).toEqual({
				kind: "tithe",
				food: 0,
				refined: TITHE_REFINED_AMOUNT,
				blessings: 1,
			});
		});

		it("stacks food and refined tithes into two blessings", () => {
			const tithe = planLeaderActions(
				snap({ resources: { food: CAP, refined: 10 } }),
			).find((d) => d.kind === "tithe");
			expect(tithe).toEqual({
				kind: "tithe",
				food: TITHE_FOOD_AMOUNT,
				refined: TITHE_REFINED_AMOUNT,
				blessings: 2,
			});
		});
	});

	describe("quarry expeditions", () => {
		// Materials low enough to want stone, food high enough that hunts
		// don't consume the idle cats first.
		function quarrySnap(overrides: Partial<LeaderSnapshot> = {}) {
			return snap({
				resources: { food: CAP, refined: 0 },
				materials: CAP * 0.3,
				hasQuarrySite: true,
				...overrides,
			});
		}

		it("opens a quarry below the low ratio when a site is explored", () => {
			expect(quarryCount(planLeaderActions(quarrySnap()))).toBe(1);
		});

		it("opens nothing without an explored quarry site", () => {
			expect(
				kinds(planLeaderActions(quarrySnap({ hasQuarrySite: false }))),
			).not.toContain("quarry");
		});

		it("stops opening quarries above the hold ratio", () => {
			expect(
				quarryCount(
					planLeaderActions(
						quarrySnap({ materials: CAP * (QUARRY_HOLD_RATIO + 0.05) }),
					),
				),
			).toBe(0);
		});

		it("does not open a quarry at exactly the low ratio", () => {
			expect(
				quarryCount(
					planLeaderActions(quarrySnap({ materials: CAP * QUARRY_LOW_RATIO })),
				),
			).toBe(0);
		});

		it("holds inside the 40-60% band without opening a second", () => {
			// One quarry already out, materials mid-band: hold, don't add.
			expect(
				quarryCount(
					planLeaderActions(
						quarrySnap({ materials: CAP * 0.5, activeQuarries: 1 }),
					),
				),
			).toBe(0);
			// No quarry out yet but inside the band: hysteresis keeps it shut.
			expect(
				quarryCount(
					planLeaderActions(
						quarrySnap({ materials: CAP * 0.5, activeQuarries: 0 }),
					),
				),
			).toBe(0);
		});

		it("never opens a second quarry while one already runs", () => {
			expect(
				quarryCount(planLeaderActions(quarrySnap({ activeQuarries: 1 }))),
			).toBe(0);
		});
	});

	describe("scouting the frontier", () => {
		function scoutSnap(overrides: Partial<LeaderSnapshot> = {}) {
			return snap({
				resources: { food: CAP, refined: 0 },
				hasFrontier: true,
				...overrides,
			});
		}

		it("keeps up to the target number of scouts out while a frontier remains", () => {
			expect(scoutCount(planLeaderActions(scoutSnap()))).toBe(SCOUT_TARGET);
		});

		it("plans no scouts when there is no frontier", () => {
			expect(
				kinds(planLeaderActions(scoutSnap({ hasFrontier: false }))),
			).not.toContain("scout");
		});

		it("only tops the frontier up to the target", () => {
			expect(
				scoutCount(planLeaderActions(scoutSnap({ activeScouts: 1 }))),
			).toBe(SCOUT_TARGET - 1);
		});

		it("plans no scouts once the target is already out", () => {
			expect(
				kinds(planLeaderActions(scoutSnap({ activeScouts: SCOUT_TARGET }))),
			).not.toContain("scout");
		});

		it("never sends more scouts than the idle cats available", () => {
			expect(scoutCount(planLeaderActions(scoutSnap({ idleCats: 1 })))).toBe(1);
		});
	});

	describe("decision priority and boundaries", () => {
		it("orders decisions hunt/cancel, storage, den, workshop, tithe", () => {
			// Overflowing stores with crowding, an empty workshop, and refined
			// surplus: cancel + storage + den + workshop + tithe.
			const decisions = planLeaderActions(
				snap({
					activeHunts: 4,
					idleCats: 4,
					resources: { food: CAP * 1.2, refined: 10 },
					housing: { capacity: 10, committed: 0 },
					workshopsNeedingWorkers: 1,
				}),
			);
			expect(kinds(decisions)).toEqual([
				"cancel_hunts",
				"build_storage",
				"build_den",
				"assign_workshop",
				"tithe",
			]);
		});

		it("returns no decisions for an empty colony", () => {
			expect(
				planLeaderActions(
					snap({
						population: 0,
						idleCats: 0,
						resources: { food: 100, refined: 0 },
					}),
				),
			).toEqual([]);
		});

		it("does not dispatch hunts in a single-cat colony", () => {
			expect(
				kinds(
					planLeaderActions(
						snap({
							population: 1,
							idleCats: 1,
							resources: { food: 0, refined: 0 },
						}),
					),
				),
			).not.toContain("hunt");
		});
	});

	describe("storehouse cap (spam regression)", () => {
		// Food brushing the cap normally triggers a storehouse.
		const overflowing = { food: CAP * 0.95, refined: 0 };

		it("builds a storehouse when under the cap", () => {
			expect(
				kinds(
					planLeaderActions(
						snap({
							resources: overflowing,
							storehouseCount: 1,
							storehouseCap: 3,
						}),
					),
				),
			).toContain("build_storage");
		});

		it("stops building once the storehouse cap is reached", () => {
			// The old bug: every finished granary left food >90%, so the leader
			// re-triggered forever. With the cap, standing storehouses at the
			// limit block any new build.
			expect(
				kinds(
					planLeaderActions(
						snap({
							resources: overflowing,
							storehouseCount: 3,
							storehouseCap: 3,
						}),
					),
				),
			).not.toContain("build_storage");
		});
	});

	describe("water economy", () => {
		it("fetches water when the reservoir runs low", () => {
			const decisions = planLeaderActions(
				snap({ water: CAP * 0.2, hasWaterSite: true }),
			);
			const fetch = decisions.find((d) => d.kind === "fetch_water");
			expect(fetch?.kind).toBe("fetch_water");
			expect(fetch && fetch.kind === "fetch_water" ? fetch.count : 0).toBe(2);
		});

		it("holds through the mid band without re-dispatching", () => {
			expect(
				kinds(
					planLeaderActions(
						snap({
							water: CAP * 0.7,
							hasWaterSite: true,
							activeWaterFetchers: 1,
						}),
					),
				),
			).not.toContain("fetch_water");
		});

		it("stops fetching once the reservoir is nearly full", () => {
			expect(
				kinds(
					planLeaderActions(snap({ water: CAP * 0.95, hasWaterSite: true })),
				),
			).not.toContain("fetch_water");
		});

		it("never fetches without a known water tile", () => {
			expect(
				kinds(planLeaderActions(snap({ water: 0, hasWaterSite: false }))),
			).not.toContain("fetch_water");
		});
	});

	// Military scenarios keep the larder full so no hunts eat the idle budget.
	const calm = (overrides: Partial<LeaderSnapshot> = {}) =>
		snap({ resources: { food: CAP, refined: 0 }, ...overrides });

	function trainCount(decisions: LeaderDecision[]): number {
		const d = decisions.find((x) => x.kind === "train_warrior");
		return d && d.kind === "train_warrior" ? d.count : 0;
	}

	describe("targetWarriors", () => {
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
			// A tiny workforce can't field the imminent-band target.
			const capped = targetWarriors(
				calm({ hasBarracks: true, threatBand: "imminent", workforce: 4 }),
			);
			expect(capped).toBe(Math.floor(4 * WARRIOR_MAX_RATIO));
		});
	});

	describe("train_warrior decisions", () => {
		it("trains recruits toward the threat-scaled target", () => {
			const decisions = planLeaderActions(
				calm({
					hasBarracks: true,
					threatBand: "rising",
					warriorCount: 0,
					trainingInFlight: 0,
					workforce: 40,
					idleCats: 10,
				}),
			);
			expect(trainCount(decisions)).toBe(WARRIOR_TARGET_BY_BAND.rising);
		});

		it("counts existing warriors and in-flight training toward the target", () => {
			const decisions = planLeaderActions(
				calm({
					hasBarracks: true,
					threatBand: "rising",
					warriorCount: 2,
					trainingInFlight: 1,
					workforce: 40,
					idleCats: 10,
				}),
			);
			expect(trainCount(decisions)).toBe(WARRIOR_TARGET_BY_BAND.rising - 3);
		});

		it("trains nobody without a barracks", () => {
			expect(
				kinds(
					planLeaderActions(
						calm({ hasBarracks: false, threatBand: "imminent" }),
					),
				),
			).not.toContain("train_warrior");
		});

		it("is bounded by idle cats", () => {
			const decisions = planLeaderActions(
				calm({
					hasBarracks: true,
					threatBand: "imminent",
					warriorCount: 0,
					workforce: 40,
					idleCats: 1,
					employedCats: 0,
				}),
			);
			expect(trainCount(decisions)).toBeLessThanOrEqual(1);
		});
	});

	describe("starving military stand-down", () => {
		it("cancels training and staffs no smithy when starving", () => {
			const decisions = planLeaderActions(
				snap({
					resources: { food: 10, refined: 0 },
					starving: true,
					hasBarracks: true,
					threatBand: "imminent",
					trainingInFlight: 2,
					smithiesNeedingWorkers: 1,
					idleCats: 10,
				}),
			);
			const k = kinds(decisions);
			expect(k).toContain("cancel_training");
			expect(k).not.toContain("train_warrior");
			expect(k).not.toContain("assign_smithy");
		});
	});

	describe("assign_smithy decisions", () => {
		it("staffs an idle smith when comfortable", () => {
			const decisions = planLeaderActions(
				calm({
					smithiesNeedingWorkers: 1,
					idleCats: 10,
					starving: false,
				}),
			);
			expect(kinds(decisions)).toContain("assign_smithy");
		});
	});
});
