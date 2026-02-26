/**
 * Tests for Colony Supply & Demand Market Report System
 */

import { describe, it, expect } from "vitest";
import {
  getSupplyDemandStatus,
  calculateFoodBalance,
  calculateWaterBalance,
  calculateHerbBalance,
  calculateMaterialBalance,
  getMarketHeadline,
  getResourceOutlook,
  formatMarketReport,
  type SupplyDemandStatus,
  type ResourceBalance,
  type MarketColonyData,
  type MarketReport,
} from "@/lib/game/supplyDemand";

// ---------------------------------------------------------------------------
// getSupplyDemandStatus
// ---------------------------------------------------------------------------
describe("getSupplyDemandStatus", () => {
  it("returns critical_shortage for ratio < 0.25", () => {
    expect(getSupplyDemandStatus(0)).toBe("critical_shortage");
    expect(getSupplyDemandStatus(0.1)).toBe("critical_shortage");
    expect(getSupplyDemandStatus(0.24)).toBe("critical_shortage");
  });

  it("returns shortage for ratio 0.25-0.75 (inclusive lower)", () => {
    expect(getSupplyDemandStatus(0.25)).toBe("shortage");
    expect(getSupplyDemandStatus(0.5)).toBe("shortage");
    expect(getSupplyDemandStatus(0.74)).toBe("shortage");
  });

  it("returns balanced for ratio 0.75-1.25 (inclusive lower)", () => {
    expect(getSupplyDemandStatus(0.75)).toBe("balanced");
    expect(getSupplyDemandStatus(1.0)).toBe("balanced");
    expect(getSupplyDemandStatus(1.24)).toBe("balanced");
  });

  it("returns surplus for ratio 1.25-2.0 (inclusive lower)", () => {
    expect(getSupplyDemandStatus(1.25)).toBe("surplus");
    expect(getSupplyDemandStatus(1.5)).toBe("surplus");
    expect(getSupplyDemandStatus(1.99)).toBe("surplus");
  });

  it("returns abundant for ratio > 2.0", () => {
    expect(getSupplyDemandStatus(2.01)).toBe("abundant");
    expect(getSupplyDemandStatus(5.0)).toBe("abundant");
    expect(getSupplyDemandStatus(100)).toBe("abundant");
  });

  it("handles boundary at 2.0 as surplus", () => {
    expect(getSupplyDemandStatus(2.0)).toBe("surplus");
  });
});

// ---------------------------------------------------------------------------
// calculateFoodBalance
// ---------------------------------------------------------------------------
describe("calculateFoodBalance", () => {
  it("returns resource name as food", () => {
    const result = calculateFoodBalance(10, 2, false, 0);
    expect(result.resource).toBe("food");
  });

  it("calculates production from hunter count", () => {
    // Each hunter produces some food
    const noHunters = calculateFoodBalance(10, 0, false, 0);
    const withHunters = calculateFoodBalance(10, 5, false, 0);
    expect(withHunters.production).toBeGreaterThan(noHunters.production);
  });

  it("mouse farm boosts food production", () => {
    const noFarm = calculateFoodBalance(10, 2, false, 0);
    const withFarm = calculateFoodBalance(10, 2, true, 0);
    expect(withFarm.production).toBeGreaterThan(noFarm.production);
  });

  it("food storage level boosts production", () => {
    const noStorage = calculateFoodBalance(10, 2, false, 0);
    const withStorage = calculateFoodBalance(10, 2, false, 3);
    expect(withStorage.production).toBeGreaterThan(noStorage.production);
  });

  it("consumption scales with cat count", () => {
    const fewCats = calculateFoodBalance(5, 2, false, 0);
    const manyCats = calculateFoodBalance(20, 2, false, 0);
    expect(manyCats.consumption).toBeGreaterThan(fewCats.consumption);
  });

  it("ratio is production / consumption", () => {
    const result = calculateFoodBalance(10, 3, false, 0);
    expect(result.ratio).toBeCloseTo(result.production / result.consumption, 2);
  });

  it("status matches ratio", () => {
    const result = calculateFoodBalance(10, 3, false, 0);
    expect(result.status).toBe(getSupplyDemandStatus(result.ratio));
  });

  it("handles zero cats gracefully", () => {
    const result = calculateFoodBalance(0, 2, false, 0);
    expect(result.ratio).toBeGreaterThan(0);
    expect(result.status).toBe("abundant");
  });
});

// ---------------------------------------------------------------------------
// calculateWaterBalance
// ---------------------------------------------------------------------------
describe("calculateWaterBalance", () => {
  it("returns resource name as water", () => {
    const result = calculateWaterBalance(10, 2, 0);
    expect(result.resource).toBe("water");
  });

  it("calculates production from water fetcher count", () => {
    const noFetchers = calculateWaterBalance(10, 0, 0);
    const withFetchers = calculateWaterBalance(10, 5, 0);
    expect(withFetchers.production).toBeGreaterThan(noFetchers.production);
  });

  it("water bowl level boosts production", () => {
    const noBowl = calculateWaterBalance(10, 2, 0);
    const withBowl = calculateWaterBalance(10, 2, 3);
    expect(withBowl.production).toBeGreaterThan(noBowl.production);
  });

  it("consumption scales with cat count", () => {
    const fewCats = calculateWaterBalance(5, 2, 0);
    const manyCats = calculateWaterBalance(20, 2, 0);
    expect(manyCats.consumption).toBeGreaterThan(fewCats.consumption);
  });

  it("handles zero cats gracefully", () => {
    const result = calculateWaterBalance(0, 2, 0);
    expect(result.status).toBe("abundant");
  });
});

// ---------------------------------------------------------------------------
// calculateHerbBalance
// ---------------------------------------------------------------------------
describe("calculateHerbBalance", () => {
  it("returns resource name as herbs", () => {
    const result = calculateHerbBalance(2, 0, 1);
    expect(result.resource).toBe("herbs");
  });

  it("calculates production from gatherer count", () => {
    const noGatherers = calculateHerbBalance(0, 0, 1);
    const withGatherers = calculateHerbBalance(3, 0, 1);
    expect(withGatherers.production).toBeGreaterThan(noGatherers.production);
  });

  it("herb garden level boosts production", () => {
    const noGarden = calculateHerbBalance(2, 0, 1);
    const withGarden = calculateHerbBalance(2, 3, 1);
    expect(withGarden.production).toBeGreaterThan(noGarden.production);
  });

  it("consumption scales with healer count", () => {
    const fewHealers = calculateHerbBalance(2, 0, 1);
    const manyHealers = calculateHerbBalance(2, 0, 5);
    expect(manyHealers.consumption).toBeGreaterThan(fewHealers.consumption);
  });

  it("handles zero healers gracefully", () => {
    const result = calculateHerbBalance(2, 0, 0);
    expect(result.status).toBe("abundant");
  });
});

// ---------------------------------------------------------------------------
// calculateMaterialBalance
// ---------------------------------------------------------------------------
describe("calculateMaterialBalance", () => {
  it("returns resource name as materials", () => {
    const result = calculateMaterialBalance(2, 1, 0);
    expect(result.resource).toBe("materials");
  });

  it("calculates production from explorer count", () => {
    const noExplorers = calculateMaterialBalance(0, 1, 0);
    const withExplorers = calculateMaterialBalance(5, 1, 0);
    expect(withExplorers.production).toBeGreaterThan(noExplorers.production);
  });

  it("consumption scales with builder count", () => {
    const fewBuilders = calculateMaterialBalance(2, 1, 0);
    const manyBuilders = calculateMaterialBalance(2, 5, 0);
    expect(manyBuilders.consumption).toBeGreaterThan(fewBuilders.consumption);
  });

  it("pending builds increase consumption", () => {
    const noBuilds = calculateMaterialBalance(2, 1, 0);
    const withBuilds = calculateMaterialBalance(2, 1, 5);
    expect(withBuilds.consumption).toBeGreaterThan(noBuilds.consumption);
  });

  it("handles zero builders and zero pending builds gracefully", () => {
    const result = calculateMaterialBalance(2, 0, 0);
    expect(result.status).toBe("abundant");
  });
});

// ---------------------------------------------------------------------------
// getMarketHeadline
// ---------------------------------------------------------------------------
describe("getMarketHeadline", () => {
  it("highlights critical shortage in headline", () => {
    const balances: ResourceBalance[] = [
      {
        resource: "food",
        production: 1,
        consumption: 10,
        ratio: 0.1,
        status: "critical_shortage",
      },
      {
        resource: "water",
        production: 5,
        consumption: 5,
        ratio: 1.0,
        status: "balanced",
      },
    ];
    const headline = getMarketHeadline(balances);
    expect(headline.toLowerCase()).toContain("food");
  });

  it("highlights surplus when no shortages", () => {
    const balances: ResourceBalance[] = [
      {
        resource: "food",
        production: 10,
        consumption: 5,
        ratio: 2.0,
        status: "surplus",
      },
      {
        resource: "water",
        production: 10,
        consumption: 5,
        ratio: 2.0,
        status: "surplus",
      },
    ];
    const headline = getMarketHeadline(balances);
    expect(headline.length).toBeGreaterThan(0);
  });

  it("returns balanced headline when all balanced", () => {
    const balances: ResourceBalance[] = [
      {
        resource: "food",
        production: 5,
        consumption: 5,
        ratio: 1.0,
        status: "balanced",
      },
      {
        resource: "water",
        production: 5,
        consumption: 5,
        ratio: 1.0,
        status: "balanced",
      },
    ];
    const headline = getMarketHeadline(balances);
    expect(headline.length).toBeGreaterThan(0);
  });

  it("picks the most critical resource for headline", () => {
    const balances: ResourceBalance[] = [
      {
        resource: "food",
        production: 3,
        consumption: 5,
        ratio: 0.6,
        status: "shortage",
      },
      {
        resource: "water",
        production: 1,
        consumption: 10,
        ratio: 0.1,
        status: "critical_shortage",
      },
      {
        resource: "herbs",
        production: 5,
        consumption: 5,
        ratio: 1.0,
        status: "balanced",
      },
    ];
    const headline = getMarketHeadline(balances);
    expect(headline.toLowerCase()).toContain("water");
  });

  it("handles empty balances array", () => {
    const headline = getMarketHeadline([]);
    expect(headline.length).toBeGreaterThan(0);
  });
});

// ---------------------------------------------------------------------------
// getResourceOutlook
// ---------------------------------------------------------------------------
describe("getResourceOutlook", () => {
  it("returns distinct outlook for critical_shortage", () => {
    const balance: ResourceBalance = {
      resource: "food",
      production: 1,
      consumption: 10,
      ratio: 0.1,
      status: "critical_shortage",
    };
    const outlook = getResourceOutlook(balance, "Food");
    expect(outlook).toContain("Food");
    expect(outlook.length).toBeGreaterThan(10);
  });

  it("returns distinct outlook for shortage", () => {
    const balance: ResourceBalance = {
      resource: "water",
      production: 3,
      consumption: 5,
      ratio: 0.6,
      status: "shortage",
    };
    const outlook = getResourceOutlook(balance, "Water");
    expect(outlook).toContain("Water");
  });

  it("returns distinct outlook for balanced", () => {
    const balance: ResourceBalance = {
      resource: "herbs",
      production: 5,
      consumption: 5,
      ratio: 1.0,
      status: "balanced",
    };
    const outlook = getResourceOutlook(balance, "Herbs");
    expect(outlook).toContain("Herbs");
  });

  it("returns distinct outlook for surplus", () => {
    const balance: ResourceBalance = {
      resource: "materials",
      production: 10,
      consumption: 5,
      ratio: 2.0,
      status: "surplus",
    };
    const outlook = getResourceOutlook(balance, "Materials");
    expect(outlook).toContain("Materials");
  });

  it("returns distinct outlook for abundant", () => {
    const balance: ResourceBalance = {
      resource: "food",
      production: 20,
      consumption: 5,
      ratio: 4.0,
      status: "abundant",
    };
    const outlook = getResourceOutlook(balance, "Food");
    expect(outlook).toContain("Food");
  });

  it("all five statuses produce different text", () => {
    const statuses: SupplyDemandStatus[] = [
      "critical_shortage",
      "shortage",
      "balanced",
      "surplus",
      "abundant",
    ];
    const outlooks = statuses.map((status, i) => {
      const balance: ResourceBalance = {
        resource: "food",
        production: i + 1,
        consumption: 5,
        ratio: (i + 1) / 5,
        status,
      };
      return getResourceOutlook(balance, "Food");
    });
    const uniqueOutlooks = new Set(outlooks);
    expect(uniqueOutlooks.size).toBe(5);
  });
});

// ---------------------------------------------------------------------------
// formatMarketReport
// ---------------------------------------------------------------------------
describe("formatMarketReport", () => {
  const baseColony: MarketColonyData = {
    catCount: 10,
    hunterCount: 3,
    waterFetcherCount: 2,
    gathererCount: 2,
    explorerCount: 2,
    healerCount: 1,
    builderCount: 1,
    pendingBuilds: 0,
    hasMouseFarm: false,
    foodStorageLevel: 0,
    waterBowlLevel: 0,
    herbGardenLevel: 0,
  };

  it("returns a complete MarketReport structure", () => {
    const report = formatMarketReport(baseColony);
    expect(report.headline).toBeDefined();
    expect(report.headline.length).toBeGreaterThan(0);
    expect(report.resourceBalances).toHaveLength(4);
    expect(report.outlook).toHaveLength(4);
    expect(report.overallHealth).toBeDefined();
  });

  it("includes all four resources in balances", () => {
    const report = formatMarketReport(baseColony);
    const names = report.resourceBalances.map((b) => b.resource);
    expect(names).toContain("food");
    expect(names).toContain("water");
    expect(names).toContain("herbs");
    expect(names).toContain("materials");
  });

  it("overall health reflects worst resource status", () => {
    const colony: MarketColonyData = {
      ...baseColony,
      hunterCount: 0,
      waterFetcherCount: 0,
      gathererCount: 0,
      explorerCount: 0,
    };
    const report = formatMarketReport(colony);
    // With no workers and many cats, should be critical
    expect(["critical_shortage", "shortage"]).toContain(report.overallHealth);
  });

  it("handles zero cats colony", () => {
    const colony: MarketColonyData = {
      ...baseColony,
      catCount: 0,
      hunterCount: 0,
      waterFetcherCount: 0,
      gathererCount: 0,
      explorerCount: 0,
      healerCount: 0,
      builderCount: 0,
    };
    const report = formatMarketReport(colony);
    expect(report.headline.length).toBeGreaterThan(0);
    expect(report.resourceBalances).toHaveLength(4);
  });

  it("handles fully stocked colony", () => {
    const colony: MarketColonyData = {
      ...baseColony,
      catCount: 5,
      hunterCount: 10,
      waterFetcherCount: 10,
      gathererCount: 10,
      explorerCount: 10,
      hasMouseFarm: true,
      foodStorageLevel: 5,
      waterBowlLevel: 5,
      herbGardenLevel: 5,
    };
    const report = formatMarketReport(colony);
    expect(["surplus", "abundant"]).toContain(report.overallHealth);
  });

  it("outlook has one entry per resource", () => {
    const report = formatMarketReport(baseColony);
    expect(report.outlook).toHaveLength(4);
    report.outlook.forEach((text) => {
      expect(text.length).toBeGreaterThan(0);
    });
  });

  it("resource balances have positive production and consumption values", () => {
    const report = formatMarketReport(baseColony);
    report.resourceBalances.forEach((balance) => {
      expect(balance.production).toBeGreaterThanOrEqual(0);
      expect(balance.consumption).toBeGreaterThanOrEqual(0);
    });
  });
});
