import { describe, expect, it } from "vitest";
import {
	BASE_BREEDING_CHANCE_PER_HOUR,
	canWork,
	catBreedingChancePerHour,
	colonyCanBreed,
	conceptionProbability,
	getLifeStage,
	inheritStats,
	leadershipAfterTenure,
	oldAgeDeathProbability,
	SPECIALIST_BREEDING_BONUS,
	stageWorkEffectiveness,
	tradeLevel,
	tradeSpeedMultiplier,
	tradeYieldMultiplier,
	workforceWeight,
} from "@/lib/game/lifeSim";
import type { CatStats } from "@/types/game";

const flatStats = (value: number): CatStats => ({
	attack: value,
	defense: value,
	hunting: value,
	medicine: value,
	cleaning: value,
	building: value,
	leadership: value,
	vision: value,
});

describe("stage capability", () => {
	it("kittens cannot work and count for nothing in the workforce", () => {
		expect(stageWorkEffectiveness("kitten")).toBe(0);
		expect(canWork("kitten")).toBe(false);
		expect(workforceWeight("kitten")).toBe(0);
	});

	it("young and elders are partial, adults full", () => {
		expect(stageWorkEffectiveness("young")).toBeLessThan(1);
		expect(stageWorkEffectiveness("adult")).toBe(1);
		expect(stageWorkEffectiveness("elder")).toBeLessThan(1);
		expect(canWork("young")).toBe(true);
		expect(canWork("elder")).toBe(true);
	});
});

describe("oldAgeDeathProbability", () => {
	it("is zero before the elder death threshold", () => {
		expect(oldAgeDeathProbability(30, false, 1)).toBe(0);
	});

	it("scales with elapsed game-hours and clamps to a probability", () => {
		const perHour = oldAgeDeathProbability(48, false, 1);
		const perTwo = oldAgeDeathProbability(48, false, 2);
		expect(perHour).toBeCloseTo(0.01, 5);
		expect(perTwo).toBeCloseTo(0.02, 5);
		// A huge skip-time jump can't exceed certain death.
		expect(oldAgeDeathProbability(200, false, 10_000)).toBe(1);
	});

	it("returns zero for a non-positive elapsed window", () => {
		expect(oldAgeDeathProbability(60, false, 0)).toBe(0);
	});
});

describe("breeding gates", () => {
	it("needs food, water and housing headroom", () => {
		const healthy = {
			foodRatio: 0.6,
			waterRatio: 0.6,
			population: 10,
			housingCapacity: 14,
		};
		expect(colonyCanBreed(healthy)).toBe(true);
		expect(colonyCanBreed({ ...healthy, foodRatio: 0.2 })).toBe(false);
		expect(colonyCanBreed({ ...healthy, waterRatio: 0.1 })).toBe(false);
		// At or above capacity: no headroom, no breeding (soft cap).
		expect(colonyCanBreed({ ...healthy, population: 14 })).toBe(false);
	});

	it("also breeds on a per-capita food/water surplus below the ratio gate", () => {
		// Subsistence early colony: a large, mostly-empty granary keeps the ratio
		// far below 0.35, but the store still holds a real per-cat surplus. It must
		// be able to breed, otherwise the founders age out unreplaced (the unaided
		// early-collapse bug). 10 cats * 2.5 = 25 food needed.
		const subsistence = {
			foodRatio: 0.08,
			waterRatio: 0.08,
			population: 10,
			housingCapacity: 14,
			food: 40,
			water: 40,
		};
		expect(colonyCanBreed(subsistence)).toBe(true);
		// Below the per-capita floor AND below the ratio gate: still no breeding.
		expect(colonyCanBreed({ ...subsistence, food: 20 })).toBe(false);
		expect(colonyCanBreed({ ...subsistence, water: 10 })).toBe(false);
		// Housing headroom still governs even with a food surplus.
		expect(colonyCanBreed({ ...subsistence, population: 14 })).toBe(false);
	});

	it("specialists conceive more readily than plain adults", () => {
		const plain = catBreedingChancePerHour(null, 0);
		const specialist = catBreedingChancePerHour("hunter", 0);
		expect(plain).toBeCloseTo(BASE_BREEDING_CHANCE_PER_HOUR, 5);
		expect(specialist).toBeCloseTo(
			BASE_BREEDING_CHANCE_PER_HOUR + SPECIALIST_BREEDING_BONUS,
			5,
		);
		expect(specialist).toBeGreaterThan(plain);
	});

	it("scales conception chance by elapsed game-hours", () => {
		const perHour = conceptionProbability(null, 0, 1);
		const perTwo = conceptionProbability(null, 0, 2);
		expect(perTwo).toBeCloseTo(2 * perHour, 5);
		expect(conceptionProbability(null, 0, 0)).toBe(0);
	});
});

describe("inheritStats", () => {
	it("biases toward the stronger parent and stays in range", () => {
		const strong = { ...flatStats(20), hunting: 90 };
		const weak = { ...flatStats(20), hunting: 40 };
		// roll 0.5 -> zero mutation, so the result is the deterministic blend.
		const kitten = inheritStats(strong, weak, () => 0.5);
		// hunting blends 90*0.6 + 40*0.4 = 70, above the midpoint of 65.
		expect(kitten.hunting).toBe(70);
		for (const value of Object.values(kitten)) {
			expect(value).toBeGreaterThanOrEqual(1);
			expect(value).toBeLessThanOrEqual(100);
		}
	});

	it("falls back to the single parent when there is no mate", () => {
		const solo = flatStats(50);
		const kitten = inheritStats(solo, null, () => 0.5);
		expect(kitten.hunting).toBe(50);
	});

	it("keeps two born hunters' line strong at hunting", () => {
		const hunterA = { ...flatStats(30), hunting: 95 };
		const hunterB = { ...flatStats(30), hunting: 88 };
		const kitten = inheritStats(hunterA, hunterB, () => 0.5);
		expect(kitten.hunting).toBeGreaterThan(80);
	});
});

describe("trade depth", () => {
	it("ranks up on a diminishing curve", () => {
		expect(tradeLevel(0)).toBe(0);
		expect(tradeLevel(1)).toBe(1);
		expect(tradeLevel(9)).toBe(3);
		expect(tradeLevel(100)).toBe(10);
	});

	it("improves yield and speed with diminishing returns", () => {
		expect(tradeYieldMultiplier(0)).toBe(1);
		expect(tradeSpeedMultiplier(0)).toBe(1);

		const y30 = tradeYieldMultiplier(30);
		const y300 = tradeYieldMultiplier(300);
		expect(y30).toBeGreaterThan(1);
		expect(y300).toBeGreaterThan(y30);
		expect(y300).toBeLessThan(1.4);

		const s30 = tradeSpeedMultiplier(30);
		const s300 = tradeSpeedMultiplier(300);
		expect(s30).toBeLessThan(1);
		expect(s300).toBeLessThan(s30);
		expect(s300).toBeGreaterThan(0.75);
	});
});

describe("leadership tenure", () => {
	it("accrues leadership over time in office, capped at 100", () => {
		expect(leadershipAfterTenure(50, 0)).toBe(50);
		expect(leadershipAfterTenure(50, 10)).toBeGreaterThan(50);
		expect(leadershipAfterTenure(99, 10_000)).toBe(100);
	});
});

describe("getLifeStage re-export", () => {
	it("maps game-hours to stages", () => {
		expect(getLifeStage(0)).toBe("kitten");
		expect(getLifeStage(12)).toBe("young");
		expect(getLifeStage(30)).toBe("adult");
		expect(getLifeStage(60)).toBe("elder");
	});
});
