import { describe, expect, it } from "vitest";

import {
	BASE_CAPACITY,
	countStorehouses,
	GRANARY_BONUS,
	type StorageBuilding,
	storageCapacities,
	storehouseCap,
	WATER_BOWL_BONUS,
} from "@/lib/game/storage";

function building(
	type: string,
	constructionProgress = 100,
	level = 1,
): StorageBuilding {
	return { type, constructionProgress, level };
}

describe("storage", () => {
	describe("storageCapacities", () => {
		it("returns the base caps for an empty settlement", () => {
			expect(storageCapacities([])).toEqual(BASE_CAPACITY);
		});

		it("raises dry-goods caps per finished granary and per level", () => {
			const caps = storageCapacities([
				building("food_storage", 100, 1),
				building("food_storage", 100, 2),
			]);
			// One level-1 + one level-2 granary = 3 levels of bonus.
			expect(caps.food).toBe(BASE_CAPACITY.food + GRANARY_BONUS.food * 3);
			expect(caps.materials).toBe(
				BASE_CAPACITY.materials + GRANARY_BONUS.materials * 3,
			);
			// Granaries hold dry goods, not water.
			expect(caps.water).toBe(BASE_CAPACITY.water);
		});

		it("raises the water cap per finished water bowl", () => {
			const caps = storageCapacities([building("water_bowl", 100, 1)]);
			expect(caps.water).toBe(BASE_CAPACITY.water + WATER_BOWL_BONUS);
			expect(caps.food).toBe(BASE_CAPACITY.food);
		});

		it("ignores unfinished buildings", () => {
			const caps = storageCapacities([building("food_storage", 40)]);
			expect(caps).toEqual(BASE_CAPACITY);
		});
	});

	describe("storehouseCap", () => {
		it("allows at least one storehouse for a tiny colony", () => {
			expect(storehouseCap(0)).toBe(1);
			expect(storehouseCap(5)).toBe(1);
		});

		it("scales with population", () => {
			expect(storehouseCap(20)).toBe(3);
			expect(storehouseCap(60)).toBe(10);
		});
	});

	describe("countStorehouses", () => {
		it("counts only finished granaries", () => {
			const count = countStorehouses([
				building("food_storage", 100),
				building("food_storage", 60),
				building("water_bowl", 100),
				building("den", 100),
			]);
			expect(count).toBe(1);
		});
	});
});
