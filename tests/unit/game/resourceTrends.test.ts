/**
 * Resource Trends — Unit Tests
 *
 * Tests for moving averages, trend direction, percentage change,
 * and full resource trend analysis.
 */

import { describe, it, expect } from "vitest";
import {
  getMovingAverage,
  getTrend,
  getPercentChange,
  analyzeResourceTrends,
  type ResourceSnapshot,
} from "@/lib/game/resourceTrends";

// ---------------------------------------------------------------------------
// getMovingAverage
// ---------------------------------------------------------------------------

describe("getMovingAverage", () => {
  it("computes average of last N values", () => {
    const values = [10, 20, 30, 40, 50];
    expect(getMovingAverage(values, 3)).toBe(40); // (30+40+50)/3
  });

  it("uses all values when window equals length", () => {
    const values = [10, 20, 30];
    expect(getMovingAverage(values, 3)).toBe(20); // (10+20+30)/3
  });

  it("uses all values when window exceeds length", () => {
    const values = [10, 20];
    expect(getMovingAverage(values, 5)).toBe(15); // (10+20)/2
  });

  it("returns single value for window of 1", () => {
    const values = [10, 20, 30];
    expect(getMovingAverage(values, 1)).toBe(30); // last value
  });

  it("returns 0 for empty array", () => {
    expect(getMovingAverage([], 3)).toBe(0);
  });

  it("handles single element", () => {
    expect(getMovingAverage([42], 3)).toBe(42);
  });
});

// ---------------------------------------------------------------------------
// getPercentChange
// ---------------------------------------------------------------------------

describe("getPercentChange", () => {
  it("computes positive percent change", () => {
    expect(getPercentChange(100, 150)).toBe(50);
  });

  it("computes negative percent change", () => {
    expect(getPercentChange(200, 100)).toBe(-50);
  });

  it("returns 0 when values are equal", () => {
    expect(getPercentChange(50, 50)).toBe(0);
  });

  it("returns 0 when previous is 0 and current is 0", () => {
    expect(getPercentChange(0, 0)).toBe(0);
  });

  it("returns 100 when previous is 0 and current is positive", () => {
    // Convention: going from 0 to any positive = +100%
    expect(getPercentChange(0, 50)).toBe(100);
  });

  it("returns -100 when previous is positive and current is 0", () => {
    expect(getPercentChange(50, 0)).toBe(-100);
  });
});

// ---------------------------------------------------------------------------
// getTrend
// ---------------------------------------------------------------------------

describe("getTrend", () => {
  it('returns "rising" when recent average exceeds previous by >5%', () => {
    // Previous window: [100, 100, 100] avg=100
    // Recent window:   [110, 110, 110] avg=110 → +10% > 5%
    const values = [100, 100, 100, 110, 110, 110];
    expect(getTrend(values, 3)).toBe("rising");
  });

  it('returns "falling" when recent average is below previous by >5%', () => {
    // Previous window: [100, 100, 100] avg=100
    // Recent window:   [80, 80, 80] avg=80 → -20% < -5%
    const values = [100, 100, 100, 80, 80, 80];
    expect(getTrend(values, 3)).toBe("falling");
  });

  it('returns "stable" when change is within ±5%', () => {
    // Previous window: [100, 100, 100] avg=100
    // Recent window:   [102, 102, 102] avg=102 → +2% within ±5%
    const values = [100, 100, 100, 102, 102, 102];
    expect(getTrend(values, 3)).toBe("stable");
  });

  it('returns "stable" for not enough data (fewer than 2 windows)', () => {
    const values = [10, 20];
    expect(getTrend(values, 3)).toBe("stable");
  });

  it('returns "stable" for empty data', () => {
    expect(getTrend([], 3)).toBe("stable");
  });

  it("detects rising at exactly 5% boundary as stable", () => {
    // Previous avg=100, Recent avg=105 → exactly +5% is NOT > 5%
    const values = [100, 100, 100, 105, 105, 105];
    expect(getTrend(values, 3)).toBe("stable");
  });

  it("detects rising just above 5% boundary", () => {
    // Previous avg=100, Recent avg=106 → +6% > 5%
    const values = [100, 100, 100, 106, 106, 106];
    expect(getTrend(values, 3)).toBe("rising");
  });

  it("detects falling at exactly -5% boundary as stable", () => {
    // Previous avg=100, Recent avg=95 → exactly -5% is NOT < -5%
    const values = [100, 100, 100, 95, 95, 95];
    expect(getTrend(values, 3)).toBe("stable");
  });
});

// ---------------------------------------------------------------------------
// analyzeResourceTrends
// ---------------------------------------------------------------------------

describe("analyzeResourceTrends", () => {
  const makeSnapshot = (
    food: number,
    water: number,
    herbs: number,
    materials: number,
    timestamp: number,
  ): ResourceSnapshot => ({ food, water, herbs, materials, timestamp });

  it("produces trend report for all 4 resource types", () => {
    const history: ResourceSnapshot[] = [
      makeSnapshot(100, 200, 50, 80, 1000),
      makeSnapshot(100, 200, 50, 80, 2000),
      makeSnapshot(100, 200, 50, 80, 3000),
      makeSnapshot(120, 180, 60, 80, 4000),
      makeSnapshot(120, 180, 60, 80, 5000),
      makeSnapshot(120, 180, 60, 80, 6000),
    ];
    const report = analyzeResourceTrends(history);

    expect(report.food.trend).toBe("rising"); // 100→120 = +20%
    expect(report.water.trend).toBe("falling"); // 200→180 = -10%
    expect(report.herbs.trend).toBe("rising"); // 50→60 = +20%
    expect(report.materials.trend).toBe("stable"); // 80→80 = 0%
  });

  it("includes moving averages in report", () => {
    const history: ResourceSnapshot[] = [
      makeSnapshot(10, 20, 30, 40, 1000),
      makeSnapshot(20, 30, 40, 50, 2000),
      makeSnapshot(30, 40, 50, 60, 3000),
    ];
    const report = analyzeResourceTrends(history);

    expect(report.food.average).toBe(20); // (10+20+30)/3
    expect(report.water.average).toBe(30); // (20+30+40)/3
    expect(report.herbs.average).toBe(40); // (30+40+50)/3
    expect(report.materials.average).toBe(50); // (40+50+60)/3
  });

  it("includes percent change in report", () => {
    const history: ResourceSnapshot[] = [
      makeSnapshot(100, 100, 100, 100, 1000),
      makeSnapshot(100, 100, 100, 100, 2000),
      makeSnapshot(100, 100, 100, 100, 3000),
      makeSnapshot(150, 50, 100, 200, 4000),
      makeSnapshot(150, 50, 100, 200, 5000),
      makeSnapshot(150, 50, 100, 200, 6000),
    ];
    const report = analyzeResourceTrends(history);

    expect(report.food.percentChange).toBe(50); // 100→150 = +50%
    expect(report.water.percentChange).toBe(-50); // 100→50 = -50%
    expect(report.herbs.percentChange).toBe(0); // 100→100 = 0%
    expect(report.materials.percentChange).toBe(100); // 100→200 = +100%
  });

  it("handles empty snapshot history", () => {
    const report = analyzeResourceTrends([]);

    expect(report.food.trend).toBe("stable");
    expect(report.food.average).toBe(0);
    expect(report.food.percentChange).toBe(0);
    expect(report.water.trend).toBe("stable");
    expect(report.herbs.trend).toBe("stable");
    expect(report.materials.trend).toBe("stable");
  });

  it("handles single snapshot", () => {
    const history: ResourceSnapshot[] = [makeSnapshot(100, 200, 50, 80, 1000)];
    const report = analyzeResourceTrends(history);

    expect(report.food.trend).toBe("stable");
    expect(report.food.average).toBe(100);
    expect(report.water.average).toBe(200);
  });

  it("uses default window size of 3", () => {
    // With 6 snapshots and window=3, two windows are compared
    const history: ResourceSnapshot[] = [
      makeSnapshot(50, 50, 50, 50, 1000),
      makeSnapshot(50, 50, 50, 50, 2000),
      makeSnapshot(50, 50, 50, 50, 3000),
      makeSnapshot(100, 50, 50, 50, 4000),
      makeSnapshot(100, 50, 50, 50, 5000),
      makeSnapshot(100, 50, 50, 50, 6000),
    ];
    const report = analyzeResourceTrends(history);

    expect(report.food.trend).toBe("rising"); // 50→100 = +100%
    expect(report.water.trend).toBe("stable"); // 50→50 = 0%
  });
});
