import { describe, expect, it } from "vitest";

import {
	FOREST_TYPES,
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
