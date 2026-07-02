import { describe, expect, it } from "vitest";

import {
	HOUSE_PRESSURE_THRESHOLD,
	housingCapacity,
	housingPressure,
	shouldQueueHouse,
	villageLevel,
} from "@/lib/game/housing";

function building(
	type: string,
	level = 1,
	constructionProgress = 100,
): { type: string; level: number; constructionProgress: number } {
	return { type, level, constructionProgress };
}

describe("housing", () => {
	describe("housingCapacity", () => {
		it("counts the shrine as base capacity for 4", () => {
			expect(housingCapacity([building("shrine")])).toBe(4);
		});

		it("adds 2 per den level", () => {
			expect(
				housingCapacity([
					building("shrine"),
					building("den"),
					building("den", 2),
				]),
			).toBe(4 + 2 + 4);
		});

		it("ignores buildings still under construction", () => {
			expect(
				housingCapacity([building("shrine"), building("den", 1, 40)]),
			).toBe(4);
		});

		it("ignores non-housing buildings", () => {
			expect(
				housingCapacity([building("shrine"), building("food_storage")]),
			).toBe(4);
		});

		it("is zero with no buildings", () => {
			expect(housingCapacity([])).toBe(0);
		});
	});

	describe("housingPressure", () => {
		it("is population over capacity", () => {
			expect(housingPressure(10, 20)).toBe(0.5);
			expect(housingPressure(20, 14)).toBeCloseTo(20 / 14, 6);
		});

		it("treats zero capacity with cats as maximal pressure", () => {
			expect(housingPressure(5, 0)).toBe(Number.POSITIVE_INFINITY);
		});

		it("is zero for an empty colony regardless of capacity", () => {
			expect(housingPressure(0, 0)).toBe(0);
			expect(housingPressure(0, 10)).toBe(0);
		});
	});

	describe("shouldQueueHouse", () => {
		it("triggers exactly at the threshold", () => {
			expect(shouldQueueHouse(HOUSE_PRESSURE_THRESHOLD)).toBe(true);
			expect(shouldQueueHouse(HOUSE_PRESSURE_THRESHOLD - 0.001)).toBe(false);
			expect(shouldQueueHouse(HOUSE_PRESSURE_THRESHOLD + 0.5)).toBe(true);
		});
	});

	describe("villageLevel", () => {
		it("tiers up with completed non-shrine buildings", () => {
			const shrineOnly = [building("shrine")];
			expect(villageLevel(shrineOnly)).toBe(1);

			const starter = [
				building("shrine"),
				...Array.from({ length: 5 }, () => building("den")),
				building("food_storage"),
			];
			expect(villageLevel(starter)).toBe(2); // 6 completed buildings

			const grown = [
				building("shrine"),
				...Array.from({ length: 12 }, () => building("den")),
			];
			expect(villageLevel(grown)).toBe(3);

			const city = [
				building("shrine"),
				...Array.from({ length: 20 }, () => building("den")),
			];
			expect(villageLevel(city)).toBe(4);
		});

		it("does not count scaffolds toward the level", () => {
			const scaffolds = [
				building("shrine"),
				...Array.from({ length: 8 }, () => building("den", 1, 10)),
			];
			expect(villageLevel(scaffolds)).toBe(1);
		});

		it("boundary: exactly 6 and exactly 12 completed buildings", () => {
			const six = [
				building("shrine"),
				...Array.from({ length: 6 }, () => building("den")),
			];
			// shrine + 6 = 6 non-shrine completed
			expect(villageLevel(six)).toBe(2);

			const twelve = [
				building("shrine"),
				...Array.from({ length: 12 }, () => building("den")),
			];
			expect(villageLevel(twelve)).toBe(3);
		});
	});
});
