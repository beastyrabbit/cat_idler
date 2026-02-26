/**
 * Tests for Resource Spoilage System
 */

import { describe, it, expect } from "vitest";
import {
  calculateSpoilageRate,
  applySpoilage,
  estimateSpoilageOverTime,
  evaluateStorageEfficiency,
  generateSpoilageReport,
} from "@/lib/game/spoilage";
import type {
  SpoilableResource,
  StorageConditions,
  StorageReport,
} from "@/lib/game/spoilage";

describe("calculateSpoilageRate", () => {
  it("returns base rate for food (2%)", () => {
    const rate = calculateSpoilageRate("food", 0, 20, 0.5);
    expect(rate).toBe(2);
  });

  it("returns base rate for herbs (1%)", () => {
    const rate = calculateSpoilageRate("herbs", 0, 20, 0.5);
    expect(rate).toBe(1);
  });

  it("applies storage level reduction for food (50% per level)", () => {
    // Level 1: 2% * (1 - 0.5*1) = 2% * 0.5 = 1%
    expect(calculateSpoilageRate("food", 1, 20, 0.5)).toBe(1);
    // Level 2: 2% * (1 - 0.5*2) = 2% * 0 = 0%
    expect(calculateSpoilageRate("food", 2, 20, 0.5)).toBe(0);
    // Level 3: clamped to 0%
    expect(calculateSpoilageRate("food", 3, 20, 0.5)).toBe(0);
  });

  it("applies storage level reduction for herbs (40% per level)", () => {
    // Level 1: 1% * (1 - 0.4*1) = 1% * 0.6 = 0.6%
    expect(calculateSpoilageRate("herbs", 1, 20, 0.5)).toBeCloseTo(0.6);
    // Level 2: 1% * (1 - 0.4*2) = 1% * 0.2 = 0.2%
    expect(calculateSpoilageRate("herbs", 2, 20, 0.5)).toBeCloseTo(0.2);
    // Level 3: 1% * (1 - 0.4*3) = 1% * max(0, -0.2) = 0%
    expect(calculateSpoilageRate("herbs", 3, 20, 0.5)).toBe(0);
  });

  it("doubles rate for hot temperatures (>30C)", () => {
    // Base food 2%, no storage, hot = 2% * 2 = 4%
    expect(calculateSpoilageRate("food", 0, 35, 0.5)).toBe(4);
  });

  it("halves rate for cold temperatures (<5C)", () => {
    // Base food 2%, no storage, cold = 2% * 0.5 = 1%
    expect(calculateSpoilageRate("food", 0, 3, 0.5)).toBe(1);
  });

  it("applies capacity penalty above 80% ratio", () => {
    // Base food 2%, no storage, normal temp, 90% capacity = 2% * 1.25 = 2.5%
    expect(calculateSpoilageRate("food", 0, 20, 0.9)).toBe(2.5);
  });

  it("clamps to minimum 0 (no negative spoilage)", () => {
    // High storage level, cold, low capacity — should not go negative
    expect(calculateSpoilageRate("food", 3, 3, 0.1)).toBe(0);
  });

  it("handles 0 storage level (no building)", () => {
    expect(calculateSpoilageRate("food", 0, 20, 0.5)).toBe(2);
  });
});

describe("applySpoilage", () => {
  it("reduces amount by spoilage rate percentage", () => {
    // 100 food with 2% spoilage = 98 remaining
    expect(applySpoilage(100, 2)).toBe(98);
  });

  it("clamps to minimum 0", () => {
    expect(applySpoilage(1, 99)).toBeCloseTo(0.01);
    expect(applySpoilage(0.5, 100)).toBe(0);
  });

  it("handles 0 amount (returns 0)", () => {
    expect(applySpoilage(0, 5)).toBe(0);
  });
});

describe("estimateSpoilageOverTime", () => {
  it("compounds spoilage over multiple ticks", () => {
    // 100 food, 10% per tick, 3 ticks: 100 * 0.9^3 = 72.9
    expect(estimateSpoilageOverTime(100, 10, 3)).toBeCloseTo(72.9);
  });

  it("handles 0 ticks (returns original)", () => {
    expect(estimateSpoilageOverTime(100, 10, 0)).toBe(100);
  });

  it("handles 0 rate (returns original)", () => {
    expect(estimateSpoilageOverTime(100, 0, 5)).toBe(100);
  });
});

describe("evaluateStorageEfficiency", () => {
  it("processes multiple resources", () => {
    const conditions: StorageConditions[] = [
      {
        resource: "food",
        currentAmount: 80,
        maxCapacity: 100,
        storageLevel: 1,
        temperature: 20,
      },
      {
        resource: "herbs",
        currentAmount: 40,
        maxCapacity: 100,
        storageLevel: 0,
        temperature: 20,
      },
    ];
    const report = evaluateStorageEfficiency(conditions);
    expect(report.results).toHaveLength(2);
    expect(report.results[0].resource).toBe("food");
    expect(report.results[1].resource).toBe("herbs");
  });

  it("handles empty array", () => {
    const report = evaluateStorageEfficiency([]);
    expect(report.results).toHaveLength(0);
    expect(report.overallEfficiency).toBe(100);
    expect(report.worstResource).toBeNull();
  });

  it("calculates worst resource", () => {
    const conditions: StorageConditions[] = [
      {
        resource: "food",
        currentAmount: 80,
        maxCapacity: 100,
        storageLevel: 2,
        temperature: 20,
      },
      {
        resource: "herbs",
        currentAmount: 50,
        maxCapacity: 100,
        storageLevel: 0,
        temperature: 35,
      },
    ];
    const report = evaluateStorageEfficiency(conditions);
    // herbs with no storage and hot temp should be worst
    expect(report.worstResource).toBe("herbs");
  });

  it("calculates overall efficiency", () => {
    const conditions: StorageConditions[] = [
      {
        resource: "food",
        currentAmount: 50,
        maxCapacity: 100,
        storageLevel: 0,
        temperature: 20,
      },
    ];
    const report = evaluateStorageEfficiency(conditions);
    // Food: base 2%, no storage, normal temp, 50% capacity (no penalty)
    // Rate = 2%, efficiency = 100 - 2 = 98 (rate as % of loss)
    // Actually efficiency = (1 - spoilageRate/100) * 100
    expect(report.overallEfficiency).toBe(98);
  });
});

describe("generateSpoilageReport", () => {
  it("includes colony name", () => {
    const report: StorageReport = {
      results: [
        {
          resource: "food",
          originalAmount: 100,
          spoiledAmount: 2,
          remainingAmount: 98,
          spoilageRate: 2,
          efficiency: 98,
        },
      ],
      overallEfficiency: 98,
      worstResource: "food",
    };
    const text = generateSpoilageReport(report, "Whiskertown");
    expect(text).toContain("Whiskertown");
  });

  it("handles 0 resources (nothing stored)", () => {
    const report: StorageReport = {
      results: [],
      overallEfficiency: 100,
      worstResource: null,
    };
    const text = generateSpoilageReport(report, "Pawville");
    expect(text).toContain("Pawville");
    expect(text.toLowerCase()).toMatch(/no.*stor|nothing|empty/);
  });

  it("handles excellent efficiency (>90%)", () => {
    const report: StorageReport = {
      results: [
        {
          resource: "food",
          originalAmount: 100,
          spoiledAmount: 1,
          remainingAmount: 99,
          spoilageRate: 1,
          efficiency: 99,
        },
      ],
      overallEfficiency: 99,
      worstResource: "food",
    };
    const text = generateSpoilageReport(report, "Catford");
    expect(text.toLowerCase()).toMatch(/excellent|superb|outstanding|well/);
  });

  it("handles poor efficiency (<50%)", () => {
    const report: StorageReport = {
      results: [
        {
          resource: "food",
          originalAmount: 100,
          spoiledAmount: 60,
          remainingAmount: 40,
          spoilageRate: 60,
          efficiency: 40,
        },
      ],
      overallEfficiency: 40,
      worstResource: "food",
    };
    const text = generateSpoilageReport(report, "Ratville");
    expect(text.toLowerCase()).toMatch(/poor|dire|alarm|concern|crisis|waste/);
  });

  it("handles multiple resources with summary", () => {
    const report: StorageReport = {
      results: [
        {
          resource: "food",
          originalAmount: 100,
          spoiledAmount: 5,
          remainingAmount: 95,
          spoilageRate: 5,
          efficiency: 95,
        },
        {
          resource: "herbs",
          originalAmount: 50,
          spoiledAmount: 3,
          remainingAmount: 47,
          spoilageRate: 6,
          efficiency: 94,
        },
      ],
      overallEfficiency: 94.5,
      worstResource: "herbs",
    };
    const text = generateSpoilageReport(report, "Meowton");
    expect(text.toLowerCase()).toContain("food");
    expect(text.toLowerCase()).toContain("herbs");
  });
});
