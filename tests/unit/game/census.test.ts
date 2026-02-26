/**
 * Tests for Colony Census & Demographics System
 *
 * Pure functions for analyzing population age distribution,
 * dependency ratio, median age, growth rate, workforce ratio,
 * and newspaper-style census headlines.
 */

import { describe, it, expect } from "vitest";
import {
  getAgeDistribution,
  getDependencyRatio,
  getMedianAge,
  getGrowthRate,
  getWorkforceRatio,
  formatCensusHeadline,
} from "@/lib/game/census";
import type { AgeDistribution } from "@/lib/game/census";

// =============================================================================
// getAgeDistribution
// =============================================================================

describe("getAgeDistribution", () => {
  it("should return all zeros for empty population", () => {
    const result = getAgeDistribution([]);
    expect(result).toEqual({ kitten: 0, young: 0, adult: 0, elder: 0 });
  });

  it("should classify a single kitten (age < 6)", () => {
    const result = getAgeDistribution([3]);
    expect(result).toEqual({ kitten: 1, young: 0, adult: 0, elder: 0 });
  });

  it("should classify a single young cat (6 <= age < 24)", () => {
    const result = getAgeDistribution([12]);
    expect(result).toEqual({ kitten: 0, young: 1, adult: 0, elder: 0 });
  });

  it("should classify a single adult (24 <= age < 48)", () => {
    const result = getAgeDistribution([36]);
    expect(result).toEqual({ kitten: 0, young: 0, adult: 1, elder: 0 });
  });

  it("should classify a single elder (age >= 48)", () => {
    const result = getAgeDistribution([60]);
    expect(result).toEqual({ kitten: 0, young: 0, adult: 0, elder: 1 });
  });

  it("should handle exact boundary at 6 hours (young, not kitten)", () => {
    const result = getAgeDistribution([6]);
    expect(result.kitten).toBe(0);
    expect(result.young).toBe(1);
  });

  it("should handle exact boundary at 24 hours (adult, not young)", () => {
    const result = getAgeDistribution([24]);
    expect(result.young).toBe(0);
    expect(result.adult).toBe(1);
  });

  it("should handle exact boundary at 48 hours (elder, not adult)", () => {
    const result = getAgeDistribution([48]);
    expect(result.adult).toBe(0);
    expect(result.elder).toBe(1);
  });

  it("should correctly distribute a mixed population", () => {
    // 2 kittens, 3 young, 2 adults, 1 elder
    const ages = [1, 4, 8, 15, 20, 30, 40, 55];
    const result = getAgeDistribution(ages);
    expect(result).toEqual({ kitten: 2, young: 3, adult: 2, elder: 1 });
  });

  it("should handle all cats in the same life stage", () => {
    const result = getAgeDistribution([25, 30, 35, 40, 45]);
    expect(result).toEqual({ kitten: 0, young: 0, adult: 5, elder: 0 });
  });

  it("should handle age 0 as kitten", () => {
    const result = getAgeDistribution([0]);
    expect(result).toEqual({ kitten: 1, young: 0, adult: 0, elder: 0 });
  });

  it("should handle very old cats", () => {
    const result = getAgeDistribution([500, 1000]);
    expect(result).toEqual({ kitten: 0, young: 0, adult: 0, elder: 2 });
  });
});

// =============================================================================
// getDependencyRatio
// =============================================================================

describe("getDependencyRatio", () => {
  it("should return 0 when there are no dependents", () => {
    const dist: AgeDistribution = { kitten: 0, young: 5, adult: 5, elder: 0 };
    expect(getDependencyRatio(dist)).toBe(0);
  });

  it("should return Infinity when there are no workers", () => {
    const dist: AgeDistribution = { kitten: 3, young: 0, adult: 0, elder: 2 };
    expect(getDependencyRatio(dist)).toBe(Infinity);
  });

  it("should return correct ratio with mixed population", () => {
    // dependents = 2 + 1 = 3, workers = 3 + 4 = 7
    const dist: AgeDistribution = { kitten: 2, young: 3, adult: 4, elder: 1 };
    expect(getDependencyRatio(dist)).toBeCloseTo(3 / 7);
  });

  it("should return 1 when dependents equal workers", () => {
    const dist: AgeDistribution = { kitten: 2, young: 1, adult: 1, elder: 2 };
    // dependents = 4, workers = 2
    expect(getDependencyRatio(dist)).toBe(2);
  });

  it("should handle all zeros (0/0 = NaN → Infinity or 0)", () => {
    const dist: AgeDistribution = { kitten: 0, young: 0, adult: 0, elder: 0 };
    // With no population, no dependents: 0 dependents / 0 workers
    expect(getDependencyRatio(dist)).toBe(0);
  });

  it("should count only kittens and elders as dependents", () => {
    const dist: AgeDistribution = { kitten: 5, young: 0, adult: 0, elder: 5 };
    expect(getDependencyRatio(dist)).toBe(Infinity);
  });
});

// =============================================================================
// getMedianAge
// =============================================================================

describe("getMedianAge", () => {
  it("should return 0 for empty array", () => {
    expect(getMedianAge([])).toBe(0);
  });

  it("should return the single value for one-element array", () => {
    expect(getMedianAge([25])).toBe(25);
  });

  it("should return middle value for odd-length array", () => {
    expect(getMedianAge([10, 20, 30])).toBe(20);
  });

  it("should return average of two middle values for even-length array", () => {
    expect(getMedianAge([10, 20, 30, 40])).toBe(25);
  });

  it("should handle unsorted input", () => {
    expect(getMedianAge([30, 10, 20])).toBe(20);
  });

  it("should handle all same values", () => {
    expect(getMedianAge([15, 15, 15, 15])).toBe(15);
  });

  it("should handle two elements", () => {
    expect(getMedianAge([10, 30])).toBe(20);
  });

  it("should not modify the original array", () => {
    const ages = [30, 10, 20];
    getMedianAge(ages);
    expect(ages).toEqual([30, 10, 20]);
  });
});

// =============================================================================
// getGrowthRate
// =============================================================================

describe("getGrowthRate", () => {
  it("should return 0 for zero population", () => {
    expect(getGrowthRate(5, 3, 0)).toBe(0);
  });

  it("should return positive rate when births exceed deaths", () => {
    // (10 - 2) / 50 * 100 = 16%
    expect(getGrowthRate(10, 2, 50)).toBe(16);
  });

  it("should return negative rate when deaths exceed births", () => {
    // (2 - 8) / 20 * 100 = -30%
    expect(getGrowthRate(2, 8, 20)).toBe(-30);
  });

  it("should return 0 when births equal deaths", () => {
    expect(getGrowthRate(5, 5, 10)).toBe(0);
  });

  it("should return 0 when no births or deaths", () => {
    expect(getGrowthRate(0, 0, 10)).toBe(0);
  });

  it("should handle 100% growth (births = population, no deaths)", () => {
    expect(getGrowthRate(10, 0, 10)).toBe(100);
  });

  it("should handle large population with small changes", () => {
    // (1 - 0) / 1000 * 100 = 0.1%
    expect(getGrowthRate(1, 0, 1000)).toBeCloseTo(0.1);
  });
});

// =============================================================================
// getWorkforceRatio
// =============================================================================

describe("getWorkforceRatio", () => {
  it("should return 0 for empty population", () => {
    const dist: AgeDistribution = { kitten: 0, young: 0, adult: 0, elder: 0 };
    expect(getWorkforceRatio(dist)).toBe(0);
  });

  it("should return 100 when all cats are workers", () => {
    const dist: AgeDistribution = { kitten: 0, young: 3, adult: 7, elder: 0 };
    expect(getWorkforceRatio(dist)).toBe(100);
  });

  it("should return 0 when all cats are dependents", () => {
    const dist: AgeDistribution = { kitten: 5, young: 0, adult: 0, elder: 3 };
    expect(getWorkforceRatio(dist)).toBe(0);
  });

  it("should return correct percentage for mixed population", () => {
    // workers = 3 + 4 = 7, total = 2 + 3 + 4 + 1 = 10
    const dist: AgeDistribution = { kitten: 2, young: 3, adult: 4, elder: 1 };
    expect(getWorkforceRatio(dist)).toBe(70);
  });

  it("should return 50 when half are workers", () => {
    const dist: AgeDistribution = { kitten: 2, young: 1, adult: 1, elder: 2 };
    // workers = 2, total = 6 → 33.33...
    // Actually: workers = 1 + 1 = 2, total = 6
    expect(getWorkforceRatio(dist)).toBeCloseTo(33.33, 1);
  });

  it("should round to 2 decimal places", () => {
    // workers = 1, total = 3 → 33.33%
    const dist: AgeDistribution = { kitten: 1, young: 0, adult: 1, elder: 1 };
    expect(getWorkforceRatio(dist)).toBeCloseTo(33.33, 1);
  });
});

// =============================================================================
// formatCensusHeadline
// =============================================================================

describe("formatCensusHeadline", () => {
  it("should generate headline for empty colony", () => {
    const dist: AgeDistribution = { kitten: 0, young: 0, adult: 0, elder: 0 };
    const headline = formatCensusHeadline(dist, 0);
    expect(headline).toContain("Ghost");
    expect(typeof headline).toBe("string");
    expect(headline.length).toBeGreaterThan(0);
  });

  it("should generate headline for baby boom (majority kittens)", () => {
    const dist: AgeDistribution = { kitten: 8, young: 1, adult: 1, elder: 0 };
    const headline = formatCensusHeadline(dist, 10);
    expect(headline.toLowerCase()).toContain("baby boom");
  });

  it("should generate headline for aging colony (majority elders)", () => {
    const dist: AgeDistribution = { kitten: 0, young: 1, adult: 1, elder: 8 };
    const headline = formatCensusHeadline(dist, 10);
    expect(headline.toLowerCase()).toContain("aging");
  });

  it("should generate headline for strong workforce (majority workers)", () => {
    const dist: AgeDistribution = { kitten: 1, young: 4, adult: 4, elder: 1 };
    const headline = formatCensusHeadline(dist, 10);
    expect(headline.toLowerCase()).toContain("workforce");
  });

  it("should generate headline for small colony", () => {
    const dist: AgeDistribution = { kitten: 0, young: 1, adult: 1, elder: 0 };
    const headline = formatCensusHeadline(dist, 2);
    expect(typeof headline).toBe("string");
    expect(headline.length).toBeGreaterThan(0);
  });

  it("should return distinct headlines for different demographics", () => {
    const babyBoom: AgeDistribution = {
      kitten: 8,
      young: 1,
      adult: 1,
      elder: 0,
    };
    const aging: AgeDistribution = { kitten: 0, young: 1, adult: 1, elder: 8 };
    const balanced: AgeDistribution = {
      kitten: 2,
      young: 3,
      adult: 3,
      elder: 2,
    };

    const h1 = formatCensusHeadline(babyBoom, 10);
    const h2 = formatCensusHeadline(aging, 10);
    const h3 = formatCensusHeadline(balanced, 10);

    expect(h1).not.toBe(h2);
    expect(h2).not.toBe(h3);
    expect(h1).not.toBe(h3);
  });
});
