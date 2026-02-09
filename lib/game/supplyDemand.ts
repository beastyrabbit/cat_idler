/**
 * Colony Supply & Demand Market Report System
 *
 * Pure functions for calculating resource production vs consumption balance,
 * identifying shortages/surpluses, and generating newspaper Market Report
 * headlines and outlook summaries for The Catford Examiner.
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type SupplyDemandStatus =
  | "critical_shortage"
  | "shortage"
  | "balanced"
  | "surplus"
  | "abundant";

export interface ResourceBalance {
  resource: string;
  production: number;
  consumption: number;
  ratio: number;
  status: SupplyDemandStatus;
}

export interface MarketColonyData {
  catCount: number;
  hunterCount: number;
  waterFetcherCount: number;
  gathererCount: number;
  explorerCount: number;
  healerCount: number;
  builderCount: number;
  pendingBuilds: number;
  hasMouseFarm: boolean;
  foodStorageLevel: number;
  waterBowlLevel: number;
  herbGardenLevel: number;
}

export interface MarketReport {
  headline: string;
  resourceBalances: ResourceBalance[];
  outlook: string[];
  overallHealth: SupplyDemandStatus;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Production rate per hunter per tick */
const FOOD_PER_HUNTER = 3;
/** Bonus food from mouse farm */
const MOUSE_FARM_BONUS = 5;
/** Bonus food per food storage level */
const FOOD_STORAGE_BONUS = 2;
/** Food consumed per cat per tick */
const FOOD_PER_CAT = 1;

/** Production rate per water fetcher per tick */
const WATER_PER_FETCHER = 4;
/** Bonus water per water bowl level */
const WATER_BOWL_BONUS = 3;
/** Water consumed per cat per tick */
const WATER_PER_CAT = 1;

/** Production rate per gatherer per tick */
const HERBS_PER_GATHERER = 2;
/** Bonus herbs per herb garden level */
const HERB_GARDEN_BONUS = 2;
/** Herbs consumed per healer per tick */
const HERBS_PER_HEALER = 3;

/** Production rate per explorer per tick */
const MATERIALS_PER_EXPLORER = 2;
/** Materials consumed per builder per tick */
const MATERIALS_PER_BUILDER = 3;
/** Extra material demand per pending build */
const MATERIALS_PER_PENDING_BUILD = 1;

/** Status priority for finding the worst status (lower = worse) */
const STATUS_PRIORITY: Record<SupplyDemandStatus, number> = {
  critical_shortage: 0,
  shortage: 1,
  balanced: 2,
  surplus: 3,
  abundant: 4,
};

/** Display names for resources */
const RESOURCE_DISPLAY_NAMES: Record<string, string> = {
  food: "Food",
  water: "Water",
  herbs: "Herbs",
  materials: "Materials",
};

// ---------------------------------------------------------------------------
// Core Functions
// ---------------------------------------------------------------------------

/**
 * Map a production/consumption ratio to a supply-demand status label.
 */
export function getSupplyDemandStatus(ratio: number): SupplyDemandStatus {
  if (ratio < 0.25) return "critical_shortage";
  if (ratio < 0.75) return "shortage";
  if (ratio < 1.25) return "balanced";
  if (ratio <= 2.0) return "surplus";
  return "abundant";
}

/**
 * Build a ResourceBalance from production and consumption values.
 * Handles zero-consumption gracefully (ratio becomes Infinity → abundant).
 */
function buildBalance(
  resource: string,
  production: number,
  consumption: number,
): ResourceBalance {
  const safeConsumption = Math.max(consumption, 0.01);
  const ratio = production / safeConsumption;
  return {
    resource,
    production,
    consumption,
    ratio,
    status: getSupplyDemandStatus(ratio),
  };
}

// ---------------------------------------------------------------------------
// Resource Balance Calculators
// ---------------------------------------------------------------------------

/**
 * Calculate food production vs consumption balance.
 */
export function calculateFoodBalance(
  catCount: number,
  hunterCount: number,
  hasMouseFarm: boolean,
  foodStorageLevel: number,
): ResourceBalance {
  const production =
    hunterCount * FOOD_PER_HUNTER +
    (hasMouseFarm ? MOUSE_FARM_BONUS : 0) +
    foodStorageLevel * FOOD_STORAGE_BONUS;
  const consumption = catCount * FOOD_PER_CAT;
  return buildBalance("food", production, consumption);
}

/**
 * Calculate water production vs consumption balance.
 */
export function calculateWaterBalance(
  catCount: number,
  waterFetcherCount: number,
  waterBowlLevel: number,
): ResourceBalance {
  const production =
    waterFetcherCount * WATER_PER_FETCHER + waterBowlLevel * WATER_BOWL_BONUS;
  const consumption = catCount * WATER_PER_CAT;
  return buildBalance("water", production, consumption);
}

/**
 * Calculate herb production vs consumption balance.
 */
export function calculateHerbBalance(
  gathererCount: number,
  herbGardenLevel: number,
  healerCount: number,
): ResourceBalance {
  const production =
    gathererCount * HERBS_PER_GATHERER + herbGardenLevel * HERB_GARDEN_BONUS;
  const consumption = healerCount * HERBS_PER_HEALER;
  return buildBalance("herbs", production, consumption);
}

/**
 * Calculate material production vs consumption balance.
 */
export function calculateMaterialBalance(
  explorerCount: number,
  builderCount: number,
  pendingBuilds: number,
): ResourceBalance {
  const production = explorerCount * MATERIALS_PER_EXPLORER;
  const consumption =
    builderCount * MATERIALS_PER_BUILDER +
    pendingBuilds * MATERIALS_PER_PENDING_BUILD;
  return buildBalance("materials", production, consumption);
}

// ---------------------------------------------------------------------------
// Headline & Outlook
// ---------------------------------------------------------------------------

/**
 * Generate a newspaper headline summarizing the overall market situation.
 * Picks the most critical resource for the headline.
 */
export function getMarketHeadline(balances: ResourceBalance[]): string {
  if (balances.length === 0) {
    return "Colony Market Report: No Resources to Track";
  }

  // Find the worst-status resource
  const worst = balances.reduce((a, b) =>
    STATUS_PRIORITY[a.status] <= STATUS_PRIORITY[b.status] ? a : b,
  );

  const name = RESOURCE_DISPLAY_NAMES[worst.resource] ?? worst.resource;

  switch (worst.status) {
    case "critical_shortage":
      return `CRISIS: ${name} Supplies Near Collapse — Rationing Urged`;
    case "shortage":
      return `${name} Shortfall Worries Colony — Suppliers Stretched Thin`;
    case "balanced":
      return "Markets Steady — Colony Supplies Meet Demand";
    case "surplus":
      return "Surplus Fills Colony Stores — Prosperity on the Rise";
    case "abundant":
      return "Abundance Overflows — Colony Enjoys Record Stockpiles";
  }
}

/**
 * Generate a per-resource outlook sentence for the newspaper.
 */
export function getResourceOutlook(
  balance: ResourceBalance,
  resourceName: string,
): string {
  switch (balance.status) {
    case "critical_shortage":
      return `${resourceName} reserves are critically low — immediate action needed to prevent colony-wide shortages.`;
    case "shortage":
      return `${resourceName} supply is falling behind demand. Additional workers or infrastructure improvements are advised.`;
    case "balanced":
      return `${resourceName} production meets current demand. The colony's needs are adequately covered for now.`;
    case "surplus":
      return `${resourceName} stores are growing steadily. The colony has a comfortable buffer against future disruptions.`;
    case "abundant":
      return `${resourceName} is overflowing — the colony has far more than it needs. Consider redirecting workers elsewhere.`;
  }
}

// ---------------------------------------------------------------------------
// Full Report Assembly
// ---------------------------------------------------------------------------

/**
 * Assemble a complete Market Report from colony data.
 */
export function formatMarketReport(data: MarketColonyData): MarketReport {
  const resourceBalances: ResourceBalance[] = [
    calculateFoodBalance(
      data.catCount,
      data.hunterCount,
      data.hasMouseFarm,
      data.foodStorageLevel,
    ),
    calculateWaterBalance(
      data.catCount,
      data.waterFetcherCount,
      data.waterBowlLevel,
    ),
    calculateHerbBalance(
      data.gathererCount,
      data.herbGardenLevel,
      data.healerCount,
    ),
    calculateMaterialBalance(
      data.explorerCount,
      data.builderCount,
      data.pendingBuilds,
    ),
  ];

  const headline = getMarketHeadline(resourceBalances);

  const outlook = resourceBalances.map((balance) => {
    const displayName =
      RESOURCE_DISPLAY_NAMES[balance.resource] ?? balance.resource;
    return getResourceOutlook(balance, displayName);
  });

  // Overall health = worst status across all resources
  const overallHealth = resourceBalances.reduce((worst, balance) =>
    STATUS_PRIORITY[balance.status] < STATUS_PRIORITY[worst.status]
      ? balance
      : worst,
  ).status;

  return {
    headline,
    resourceBalances,
    outlook,
    overallHealth,
  };
}
