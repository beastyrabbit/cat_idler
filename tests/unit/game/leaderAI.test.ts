import { describe, expect, it } from "vitest";

import {
	DEN_PRESSURE_THRESHOLD,
	EMPLOYMENT_TARGET_RATIO,
	HUNT_CANCEL_RATIO,
	HUNT_HOLD_RATIO,
	type LeaderDecision,
	type LeaderSnapshot,
	planLeaderActions,
	STORAGE_RATIO,
	TITHE_FOOD_AMOUNT,
	TITHE_FOOD_RATIO,
	TITHE_REFINED_AMOUNT,
	targetHuntSlots,
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
		housing: { capacity: 40, committed: 0 },
		activeHunts: 0,
		denPlansInFlight: 0,
		storagePlansInFlight: 0,
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
});
