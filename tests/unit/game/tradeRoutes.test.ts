/**
 * Tests for Colony Trade Routes System
 */

import { describe, it, expect } from "vitest";
import {
  calculateGoodsValue,
  scoreRouteProfitability,
  simulateTradeRun,
  evaluateTradeBatch,
  generateMarketReport,
} from "@/lib/game/tradeRoutes";
import type {
  TradeRoute,
  TradeRunResult,
  TradeBatchResult,
} from "@/lib/game/tradeRoutes";

// ── calculateGoodsValue ──────────────────────────────────────────────

describe("calculateGoodsValue", () => {
  it("returns base value for mid-capacity goods (food)", () => {
    // 30 of 100 capacity = 30%, between 20-50% → base value only
    // food base = 1, amount = 30 → value = 30
    const value = calculateGoodsValue("food", 30, 100);
    expect(value).toBe(30);
  });

  it("returns base value for mid-capacity goods (water)", () => {
    // water base = 1.5, amount = 40, capacity = 100 → 40% → base
    const value = calculateGoodsValue("water", 40, 100);
    expect(value).toBe(60); // 40 * 1.5
  });

  it("applies surplus bonus above 50% capacity", () => {
    // 60 of 100 = 60% → surplus modifier 1.2
    // food base = 1, value = 60 * 1 * 1.2 = 72
    const value = calculateGoodsValue("food", 60, 100);
    expect(value).toBe(72);
  });

  it("applies scarcity penalty below 20% capacity", () => {
    // 10 of 100 = 10% → scarcity modifier 1.5
    // food base = 1, value = 10 * 1 * 1.5 = 15
    const value = calculateGoodsValue("food", 10, 100);
    expect(value).toBe(15);
  });

  it("handles 0 amount (returns 0)", () => {
    const value = calculateGoodsValue("food", 0, 100);
    expect(value).toBe(0);
  });

  it("handles different resource base values", () => {
    // herbs base = 3, mid-capacity
    const herbs = calculateGoodsValue("herbs", 30, 100);
    expect(herbs).toBe(90); // 30 * 3

    // materials base = 2, mid-capacity
    const materials = calculateGoodsValue("materials", 30, 100);
    expect(materials).toBe(60); // 30 * 2
  });
});

// ── scoreRouteProfitability ──────────────────────────────────────────

describe("scoreRouteProfitability", () => {
  it("returns positive for profitable routes", () => {
    // goodsValue=100, distance=2, danger=0, demand=1.5
    // profit = (100 * 1.5) - 2 - 0 = 148
    const score = scoreRouteProfitability(100, 2, 0, 1.5);
    expect(score).toBeGreaterThan(0);
  });

  it("returns negative for unprofitable routes", () => {
    // goodsValue=10, distance=10, danger=90, demand=0.5
    // revenue = 10*0.5 = 5, cost = 10, expectedLoss = 10*0.5*0.9 = 4.5
    // profit = 5 - 10 - 4.5 = -9.5
    const score = scoreRouteProfitability(10, 10, 90, 0.5);
    expect(score).toBeLessThan(0);
  });

  it("accounts for danger-adjusted expected losses", () => {
    // Same goods, distance, demand — higher danger = lower profit
    const safe = scoreRouteProfitability(100, 3, 10, 1.0);
    const risky = scoreRouteProfitability(100, 3, 80, 1.0);
    expect(safe).toBeGreaterThan(risky);
  });

  it("scales with demand multiplier", () => {
    const lowDemand = scoreRouteProfitability(100, 3, 20, 0.5);
    const highDemand = scoreRouteProfitability(100, 3, 20, 2.0);
    expect(highDemand).toBeGreaterThan(lowDemand);
  });
});

// ── simulateTradeRun ─────────────────────────────────────────────────

describe("simulateTradeRun", () => {
  const baseRoute: TradeRoute = {
    destination: "Whiskerton",
    resource: "food",
    amount: 50,
    capacity: 100,
    distance: 3,
    dangerLevel: 30,
    demandMultiplier: 1.2,
  };

  it("uses seeded RNG for determinism", () => {
    const run1 = simulateTradeRun(baseRoute, 42);
    const run2 = simulateTradeRun(baseRoute, 42);
    expect(run1).toEqual(run2);
  });

  it("produces different results with different seeds", () => {
    const run1 = simulateTradeRun(baseRoute, 42);
    const run2 = simulateTradeRun(baseRoute, 999);
    // At least one field should differ
    const differs =
      run1.goodsArrived !== run2.goodsArrived ||
      run1.profit !== run2.profit ||
      run1.success !== run2.success;
    expect(differs).toBe(true);
  });

  it("delivers all goods on success (0 danger)", () => {
    const safeRoute: TradeRoute = { ...baseRoute, dangerLevel: 0 };
    const run = simulateTradeRun(safeRoute, 42);
    expect(run.goodsArrived).toBe(safeRoute.amount);
    expect(run.success).toBe(true);
  });

  it("calculates profit correctly on full delivery", () => {
    const safeRoute: TradeRoute = { ...baseRoute, dangerLevel: 0 };
    const run = simulateTradeRun(safeRoute, 42);
    // goodsValue for 50 food at 50% capacity = surplus: 50 * 1 * 1.2 = 60
    // profit = (60 * 1.2) - 3 = 69
    expect(run.profit).toBe(
      Math.round(
        calculateGoodsValue(
          safeRoute.resource,
          safeRoute.amount,
          safeRoute.capacity,
        ) *
          safeRoute.demandMultiplier -
          safeRoute.distance,
      ),
    );
  });

  it("loses goods proportionally to danger level on failure", () => {
    // Use high danger route
    const dangerRoute: TradeRoute = { ...baseRoute, dangerLevel: 100 };
    // Try seeds until we find a failure
    let failRun: TradeRunResult | null = null;
    for (let seed = 1; seed < 1000; seed++) {
      const run = simulateTradeRun(dangerRoute, seed);
      if (!run.success) {
        failRun = run;
        break;
      }
    }
    expect(failRun).not.toBeNull();
    expect(failRun!.goodsArrived).toBeLessThan(dangerRoute.amount);
    expect(failRun!.goodsSent).toBe(dangerRoute.amount);
  });
});

// ── evaluateTradeBatch ───────────────────────────────────────────────

describe("evaluateTradeBatch", () => {
  const routes: TradeRoute[] = [
    {
      destination: "Whiskerton",
      resource: "food",
      amount: 50,
      capacity: 100,
      distance: 3,
      dangerLevel: 20,
      demandMultiplier: 1.2,
    },
    {
      destination: "Pawsville",
      resource: "water",
      amount: 30,
      capacity: 80,
      distance: 5,
      dangerLevel: 40,
      demandMultiplier: 1.5,
    },
    {
      destination: "Furton",
      resource: "herbs",
      amount: 10,
      capacity: 50,
      distance: 7,
      dangerLevel: 60,
      demandMultiplier: 2.0,
    },
  ];

  it("processes multiple routes", () => {
    const batch = evaluateTradeBatch(routes, 42);
    expect(batch.runs).toHaveLength(3);
  });

  it("handles empty routes array", () => {
    const batch = evaluateTradeBatch([], 42);
    expect(batch.runs).toHaveLength(0);
    expect(batch.totalProfit).toBe(0);
    expect(batch.successfulRuns).toBe(0);
    expect(batch.failedRuns).toBe(0);
  });

  it("tallies total profit and success/failure counts", () => {
    const batch = evaluateTradeBatch(routes, 42);
    const sumProfit = batch.runs.reduce((s, r) => s + r.profit, 0);
    const successes = batch.runs.filter((r) => r.success).length;
    const failures = batch.runs.filter((r) => !r.success).length;
    expect(batch.totalProfit).toBe(sumProfit);
    expect(batch.successfulRuns).toBe(successes);
    expect(batch.failedRuns).toBe(failures);
    expect(batch.successfulRuns + batch.failedRuns).toBe(routes.length);
  });

  it("uses seeded RNG for determinism across batch", () => {
    const batch1 = evaluateTradeBatch(routes, 42);
    const batch2 = evaluateTradeBatch(routes, 42);
    expect(batch1).toEqual(batch2);
  });
});

// ── generateMarketReport ─────────────────────────────────────────────

describe("generateMarketReport", () => {
  it("includes colony name", () => {
    const batch: TradeBatchResult = {
      runs: [],
      totalProfit: 0,
      successfulRuns: 0,
      failedRuns: 0,
    };
    const report = generateMarketReport(batch, "Catford");
    expect(report).toContain("Catford");
  });

  it("handles 0 runs (quiet market day)", () => {
    const batch: TradeBatchResult = {
      runs: [],
      totalProfit: 0,
      successfulRuns: 0,
      failedRuns: 0,
    };
    const report = generateMarketReport(batch, "Catford");
    expect(report.toLowerCase()).toContain("quiet");
  });

  it("handles 1 run (singular)", () => {
    const batch: TradeBatchResult = {
      runs: [
        {
          destination: "Whiskerton",
          resource: "food",
          goodsSent: 50,
          goodsArrived: 50,
          profit: 57,
          success: true,
        },
      ],
      totalProfit: 57,
      successfulRuns: 1,
      failedRuns: 0,
    };
    const report = generateMarketReport(batch, "Catford");
    expect(report).toContain("Whiskerton");
    expect(report).toContain("1 trade");
  });

  it("handles multiple runs with summary", () => {
    const batch: TradeBatchResult = {
      runs: [
        {
          destination: "Whiskerton",
          resource: "food",
          goodsSent: 50,
          goodsArrived: 50,
          profit: 57,
          success: true,
        },
        {
          destination: "Pawsville",
          resource: "water",
          goodsSent: 30,
          goodsArrived: 20,
          profit: -5,
          success: false,
        },
      ],
      totalProfit: 52,
      successfulRuns: 1,
      failedRuns: 1,
    };
    const report = generateMarketReport(batch, "Catford");
    expect(report).toContain("2 trade");
    expect(report).toContain("Whiskerton");
    expect(report).toContain("Pawsville");
  });
});
