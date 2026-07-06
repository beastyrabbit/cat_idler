import { describe, expect, it } from "vitest";

import {
	CHOPPED_FOREST_FOOD_CAP,
	FOREST_TYPES,
	isChoppedStumpTile,
	isForestType,
	regrowthAmount,
} from "@/lib/game/depletion";

describe("isForestType", () => {
	it("treats every listed forest type as forest", () => {
		for (const type of FOREST_TYPES) {
			expect(isForestType(type)).toBe(true);
		}
	});

	it("treats worldgen forest tile types as forest", () => {
		expect(isForestType("forest")).toBe(true);
		expect(isForestType("dense_woods")).toBe(true);
	});

	it("treats non-forest tiles as not forest", () => {
		expect(isForestType("field")).toBe(false);
		expect(isForestType("meadow")).toBe(false);
		expect(isForestType("river")).toBe(false);
		expect(isForestType("enemy_territory")).toBe(false);
		expect(isForestType("")).toBe(false);
	});
});

describe("regrowthAmount", () => {
	it("regrows +1 food per hour", () => {
		expect(regrowthAmount(3600)).toBeCloseTo(1);
		expect(regrowthAmount(1800)).toBeCloseTo(0.5);
		expect(regrowthAmount(7200)).toBeCloseTo(2);
	});

	it("returns zero for zero or negative elapsed time", () => {
		expect(regrowthAmount(0)).toBe(0);
		expect(regrowthAmount(-100)).toBe(0);
	});

	it("scales linearly with elapsed time", () => {
		expect(regrowthAmount(120)).toBeCloseTo(120 / 3600);
	});
});

describe("isChoppedStumpTile", () => {
	const stump = {
		type: "field",
		maxResources: { food: CHOPPED_FOREST_FOOD_CAP },
		lastDepleted: 1_000,
	};

	it("detects a felled forest by its low field food cap + depletion stamp", () => {
		expect(isChoppedStumpTile(stump)).toBe(true);
	});

	it("ignores a natural field tile (higher food cap)", () => {
		expect(
			isChoppedStumpTile({
				type: "field",
				maxResources: { food: 40 },
				lastDepleted: 1_000,
			}),
		).toBe(false);
	});

	it("ignores a pristine field tile (never depleted)", () => {
		expect(isChoppedStumpTile({ ...stump, lastDepleted: 0 })).toBe(false);
	});

	it("ignores standing forest and water", () => {
		expect(
			isChoppedStumpTile({
				type: "forest",
				maxResources: { food: 0 },
				lastDepleted: 1_000,
			}),
		).toBe(false);
		expect(
			isChoppedStumpTile({
				type: "river",
				maxResources: { food: 0 },
				lastDepleted: 1_000,
			}),
		).toBe(false);
	});
});
