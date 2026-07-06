import { describe, expect, it } from "vitest";

import {
	advanceSmithy,
	SMITH_FAST_SPEED,
	SMITHY_ARMOR_PER_CYCLE,
	SMITHY_CYCLE_SEC,
	SMITHY_MATERIALS_PER_CYCLE,
	SMITHY_REFINED_PER_CYCLE,
	SMITHY_WEAPONS_PER_CYCLE,
} from "@/lib/game/smithy";

describe("smithy", () => {
	describe("advanceSmithy", () => {
		it("produces nothing without a worker", () => {
			const step = advanceSmithy(0, 10_000, {
				hasWorker: false,
				refinedAvailable: 100,
				materialsAvailable: 100,
			});
			expect(step).toEqual({
				nextProgress: 0,
				refinedUsed: 0,
				materialsUsed: 0,
				weaponsProduced: 0,
				armorProduced: 0,
			});
		});

		it("forges a weapon and armor after one full cycle with inputs", () => {
			const step = advanceSmithy(0, SMITHY_CYCLE_SEC, {
				hasWorker: true,
				refinedAvailable: 100,
				materialsAvailable: 100,
			});
			expect(step.weaponsProduced).toBe(SMITHY_WEAPONS_PER_CYCLE);
			expect(step.armorProduced).toBe(SMITHY_ARMOR_PER_CYCLE);
			expect(step.refinedUsed).toBe(SMITHY_REFINED_PER_CYCLE);
			expect(step.materialsUsed).toBe(SMITHY_MATERIALS_PER_CYCLE);
			expect(step.nextProgress).toBe(0);
		});

		it("accumulates short ticks into a full cycle", () => {
			let progress = 0;
			let weapons = 0;
			for (let i = 0; i < SMITHY_CYCLE_SEC; i++) {
				const step = advanceSmithy(progress, 1, {
					hasWorker: true,
					refinedAvailable: 100,
					materialsAvailable: 100,
				});
				progress = step.nextProgress;
				weapons += step.weaponsProduced;
			}
			expect(weapons).toBe(1);
			expect(progress).toBe(0);
		});

		it("is limited by whichever input runs out first", () => {
			// Enough time for 3 cycles, refined for 3, but materials for only 1.
			const step = advanceSmithy(0, SMITHY_CYCLE_SEC * 3, {
				hasWorker: true,
				refinedAvailable: SMITHY_REFINED_PER_CYCLE * 3,
				materialsAvailable: SMITHY_MATERIALS_PER_CYCLE * 1,
			});
			expect(step.weaponsProduced).toBe(1);
			expect(step.materialsUsed).toBe(SMITHY_MATERIALS_PER_CYCLE);
			expect(step.refinedUsed).toBe(SMITHY_REFINED_PER_CYCLE);
		});

		it("stalls progress at one cycle when starved of inputs", () => {
			const step = advanceSmithy(0, SMITHY_CYCLE_SEC * 5, {
				hasWorker: true,
				refinedAvailable: 0,
				materialsAvailable: 0,
			});
			expect(step.weaponsProduced).toBe(0);
			expect(step.nextProgress).toBe(SMITHY_CYCLE_SEC);
		});

		it("a fast smith produces at double rate", () => {
			const step = advanceSmithy(0, SMITHY_CYCLE_SEC, {
				hasWorker: true,
				workerIsFast: true,
				refinedAvailable: 100,
				materialsAvailable: 100,
			});
			expect(step.weaponsProduced).toBe(SMITH_FAST_SPEED);
		});
	});
});
