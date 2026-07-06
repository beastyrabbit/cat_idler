import { describe, expect, it } from "vitest";

import {
	advanceWorkshop,
	FIELD_UNLOCK_LEVEL,
	fieldUnlocked,
	fieldYield,
	WORKSHOP_CYCLE_SEC,
	WORKSHOP_UNLOCK_LEVEL,
	workshopUnlocked,
} from "@/lib/game/production";

describe("production", () => {
	describe("advanceWorkshop", () => {
		it("produces nothing without a worker", () => {
			const step = advanceWorkshop(0, 10_000, {
				hasWorker: false,
				materialsAvailable: 100,
			});
			expect(step).toEqual({
				nextProgress: 0,
				materialsUsed: 0,
				refinedProduced: 0,
			});
		});

		it("accumulates short ticks into a full cycle", () => {
			let progress = 0;
			let refined = 0;
			for (let i = 0; i < WORKSHOP_CYCLE_SEC; i++) {
				const step = advanceWorkshop(progress, 1, {
					hasWorker: true,
					materialsAvailable: 100,
				});
				progress = step.nextProgress;
				refined += step.refinedProduced;
			}
			expect(refined).toBe(1);
			expect(progress).toBe(0);
		});

		it("completes exactly at the cycle boundary", () => {
			const under = advanceWorkshop(WORKSHOP_CYCLE_SEC - 1, 0.5, {
				hasWorker: true,
				materialsAvailable: 100,
			});
			expect(under.refinedProduced).toBe(0);

			const at = advanceWorkshop(WORKSHOP_CYCLE_SEC - 1, 1, {
				hasWorker: true,
				materialsAvailable: 100,
			});
			expect(at.refinedProduced).toBe(1);
			expect(at.materialsUsed).toBe(5);
		});

		it("is limited by available materials and stalls progress", () => {
			const step = advanceWorkshop(0, WORKSHOP_CYCLE_SEC * 3, {
				hasWorker: true,
				materialsAvailable: 7, // only one cycle's worth
			});
			expect(step.refinedProduced).toBe(1);
			expect(step.materialsUsed).toBe(5);
			// Progress can't bank more than one full cycle while starved.
			expect(step.nextProgress).toBeLessThanOrEqual(WORKSHOP_CYCLE_SEC);
		});

		it("architects work twice as fast", () => {
			const step = advanceWorkshop(0, WORKSHOP_CYCLE_SEC / 2, {
				hasWorker: true,
				workerIsArchitect: true,
				materialsAvailable: 100,
			});
			expect(step.refinedProduced).toBe(1);
		});
	});

	describe("fieldYield", () => {
		it("prorates by elapsed time", () => {
			expect(fieldYield(3600)).toBe(2);
			expect(fieldYield(1800)).toBe(1);
			expect(fieldYield(0)).toBe(0);
			expect(fieldYield(-5)).toBe(0);
		});
	});

	describe("unlocks", () => {
		it("gates workshops and fields by village level", () => {
			expect(workshopUnlocked(WORKSHOP_UNLOCK_LEVEL - 1)).toBe(false);
			expect(workshopUnlocked(WORKSHOP_UNLOCK_LEVEL)).toBe(true);
			expect(fieldUnlocked(FIELD_UNLOCK_LEVEL - 1)).toBe(false);
			expect(fieldUnlocked(FIELD_UNLOCK_LEVEL)).toBe(true);
		});
	});
});
