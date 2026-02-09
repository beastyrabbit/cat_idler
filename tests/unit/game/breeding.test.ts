/**
 * Tests for Fertility Blessing Modifier
 *
 * Pure functions: calculateFertilityBonus, calculateBreedingChance
 */

import { describe, it, expect } from "vitest";
import {
  calculateFertilityBonus,
  calculateBreedingChance,
} from "@/lib/game/breeding";

describe("calculateFertilityBonus", () => {
  it("should return 0 when blessings is 0", () => {
    expect(calculateFertilityBonus(0)).toBe(0);
  });

  it("should return 0.02 per blessing point", () => {
    expect(calculateFertilityBonus(1)).toBe(0.02);
    expect(calculateFertilityBonus(5)).toBe(0.1);
    expect(calculateFertilityBonus(10)).toBe(0.2);
  });

  it("should cap bonus at 0.5", () => {
    expect(calculateFertilityBonus(25)).toBe(0.5);
    expect(calculateFertilityBonus(30)).toBe(0.5);
    expect(calculateFertilityBonus(100)).toBe(0.5);
  });

  it("should return 0 for negative blessings", () => {
    expect(calculateFertilityBonus(-1)).toBe(0);
    expect(calculateFertilityBonus(-100)).toBe(0);
  });

  it("should handle fractional blessings", () => {
    expect(calculateFertilityBonus(0.5)).toBeCloseTo(0.01);
  });
});

describe("calculateBreedingChance", () => {
  it("should return base chance when no blessings", () => {
    expect(calculateBreedingChance(0.3, 0)).toBe(0.3);
  });

  it("should add blessing bonus to base chance", () => {
    // 0.3 base + 5 blessings * 0.02 = 0.3 + 0.1 = 0.4
    expect(calculateBreedingChance(0.3, 5)).toBeCloseTo(0.4);
  });

  it("should cap at 0.8 maximum", () => {
    // 0.3 base + 100 blessings = 0.3 + 0.5 (capped bonus) = 0.8
    expect(calculateBreedingChance(0.3, 100)).toBe(0.8);
  });

  it("should cap at 0.8 even with high base", () => {
    // 0.7 base + 10 blessings * 0.02 = 0.7 + 0.2 = 0.9 → capped to 0.8
    expect(calculateBreedingChance(0.7, 10)).toBe(0.8);
  });

  it("should handle zero base chance", () => {
    // 0 base + 5 blessings * 0.02 = 0.1
    expect(calculateBreedingChance(0, 5)).toBeCloseTo(0.1);
  });

  it("should handle negative base chance", () => {
    // Negative base treated as 0 effectively
    expect(calculateBreedingChance(-0.5, 0)).toBe(0);
    expect(calculateBreedingChance(-0.5, 10)).toBeCloseTo(0.2);
  });

  it("should handle huge blessings count", () => {
    // Bonus capped at 0.5, total capped at 0.8
    expect(calculateBreedingChance(0.3, 9999)).toBe(0.8);
  });

  it("should work with typical gameplay values", () => {
    // Base 30%, 3 blessings = 30% + 6% = 36%
    expect(calculateBreedingChance(0.3, 3)).toBeCloseTo(0.36);
    // Base 30%, 15 blessings = 30% + 30% = 60%
    expect(calculateBreedingChance(0.3, 15)).toBeCloseTo(0.6);
    // Base 30%, 25 blessings = 30% + 50% = 80%
    expect(calculateBreedingChance(0.3, 25)).toBe(0.8);
  });

  it("should handle negative blessings", () => {
    // Negative blessings = 0 bonus
    expect(calculateBreedingChance(0.3, -5)).toBe(0.3);
  });
});
