/**
 * Resource Spoilage System
 *
 * Pure functions for calculating food/herb decay over time
 * based on storage building levels, temperature, and capacity.
 */

// --- Types ---

export type SpoilableResource = "food" | "herbs";

export interface StorageConditions {
  resource: SpoilableResource;
  currentAmount: number;
  maxCapacity: number;
  storageLevel: number; // 0-3 building upgrade level
  temperature: number; // degrees Celsius
}

export interface ResourceSpoilageResult {
  resource: SpoilableResource;
  originalAmount: number;
  spoiledAmount: number;
  remainingAmount: number;
  spoilageRate: number;
  efficiency: number; // 0-100 storage efficiency percentage
}

export interface StorageReport {
  results: ResourceSpoilageResult[];
  overallEfficiency: number; // average across all resources
  worstResource: SpoilableResource | null;
}

// --- Constants ---

const BASE_DECAY_RATES: Record<SpoilableResource, number> = {
  food: 2,
  herbs: 1,
};

const STORAGE_REDUCTION_PER_LEVEL: Record<SpoilableResource, number> = {
  food: 0.5, // 50% reduction per level
  herbs: 0.4, // 40% reduction per level
};

const HOT_THRESHOLD = 30;
const COLD_THRESHOLD = 5;
const HOT_MULTIPLIER = 2;
const COLD_MULTIPLIER = 0.5;

const CAPACITY_PENALTY_THRESHOLD = 0.8;
const CAPACITY_PENALTY_MULTIPLIER = 1.25;

// --- Functions ---

/**
 * Calculate the spoilage rate for a resource given storage conditions.
 *
 * @param resource - Type of spoilable resource
 * @param storageLevel - Building upgrade level (0-3)
 * @param temperature - Degrees Celsius
 * @param capacityRatio - Current fill ratio (0-1)
 * @returns Spoilage rate as percentage per tick
 */
export function calculateSpoilageRate(
  resource: SpoilableResource,
  storageLevel: number,
  temperature: number,
  capacityRatio: number,
): number {
  const baseRate = BASE_DECAY_RATES[resource];
  const reductionPerLevel = STORAGE_REDUCTION_PER_LEVEL[resource];

  // Storage reduces spoilage
  const storageMultiplier = Math.max(0, 1 - reductionPerLevel * storageLevel);
  let rate = baseRate * storageMultiplier;

  // Temperature modifier
  if (temperature > HOT_THRESHOLD) {
    rate *= HOT_MULTIPLIER;
  } else if (temperature < COLD_THRESHOLD) {
    rate *= COLD_MULTIPLIER;
  }

  // Capacity penalty
  if (capacityRatio > CAPACITY_PENALTY_THRESHOLD) {
    rate *= CAPACITY_PENALTY_MULTIPLIER;
  }

  return Math.max(0, rate);
}

/**
 * Apply spoilage to a resource amount.
 *
 * @param currentAmount - Current resource amount
 * @param spoilageRate - Spoilage rate as percentage
 * @returns Amount remaining after spoilage
 */
export function applySpoilage(
  currentAmount: number,
  spoilageRate: number,
): number {
  if (currentAmount <= 0) return 0;
  const remaining = currentAmount * (1 - spoilageRate / 100);
  return Math.max(0, remaining);
}

/**
 * Estimate resource amount after multiple ticks of spoilage.
 *
 * @param currentAmount - Starting amount
 * @param spoilageRate - Spoilage rate as percentage per tick
 * @param ticks - Number of ticks to simulate
 * @returns Projected amount after N ticks
 */
export function estimateSpoilageOverTime(
  currentAmount: number,
  spoilageRate: number,
  ticks: number,
): number {
  if (ticks <= 0 || spoilageRate <= 0) return currentAmount;
  const retentionRate = 1 - spoilageRate / 100;
  return currentAmount * Math.pow(retentionRate, ticks);
}

/**
 * Evaluate storage efficiency across multiple resources.
 *
 * @param resources - Array of storage conditions to evaluate
 * @returns Storage report with per-resource results and overall efficiency
 */
export function evaluateStorageEfficiency(
  resources: StorageConditions[],
): StorageReport {
  if (resources.length === 0) {
    return { results: [], overallEfficiency: 100, worstResource: null };
  }

  const results: ResourceSpoilageResult[] = resources.map((cond) => {
    const capacityRatio =
      cond.maxCapacity > 0 ? cond.currentAmount / cond.maxCapacity : 0;
    const spoilageRate = calculateSpoilageRate(
      cond.resource,
      cond.storageLevel,
      cond.temperature,
      capacityRatio,
    );
    const remainingAmount = applySpoilage(cond.currentAmount, spoilageRate);
    const spoiledAmount = cond.currentAmount - remainingAmount;
    const efficiency = (1 - spoilageRate / 100) * 100;

    return {
      resource: cond.resource,
      originalAmount: cond.currentAmount,
      spoiledAmount: Math.round(spoiledAmount * 100) / 100,
      remainingAmount: Math.round(remainingAmount * 100) / 100,
      spoilageRate,
      efficiency,
    };
  });

  const overallEfficiency =
    results.reduce((sum, r) => sum + r.efficiency, 0) / results.length;

  let worstResource: SpoilableResource | null = null;
  let worstEfficiency = Infinity;
  for (const r of results) {
    if (r.efficiency < worstEfficiency) {
      worstEfficiency = r.efficiency;
      worstResource = r.resource;
    }
  }

  return { results, overallEfficiency, worstResource };
}

/**
 * Generate a newspaper "Storage & Supplies" column from a storage report.
 *
 * @param report - Storage efficiency report
 * @param colonyName - Name of the colony
 * @returns Formatted newspaper column text
 */
export function generateSpoilageReport(
  report: StorageReport,
  colonyName: string,
): string {
  const lines: string[] = [];
  lines.push(`STORAGE & SUPPLIES — ${colonyName}`);
  lines.push("");

  if (report.results.length === 0) {
    lines.push("Nothing currently stored in the colony warehouses.");
    return lines.join("\n");
  }

  // Overall assessment
  if (report.overallEfficiency > 90) {
    lines.push(
      "Storage conditions are excellent across the colony. Minimal waste reported.",
    );
  } else if (report.overallEfficiency >= 50) {
    lines.push(
      "Storage conditions are adequate, though some improvements could reduce waste.",
    );
  } else {
    lines.push(
      "Alarming waste levels reported! Poor storage conditions are causing crisis-level spoilage.",
    );
  }
  lines.push("");

  // Per-resource details
  for (const r of report.results) {
    const label = r.resource.charAt(0).toUpperCase() + r.resource.slice(1);
    lines.push(
      `${label}: ${r.remainingAmount} units remaining (${r.spoiledAmount} lost to spoilage, ${r.spoilageRate.toFixed(1)}% rate)`,
    );
  }

  if (report.worstResource && report.results.length > 1) {
    lines.push("");
    lines.push(
      `Worst performing: ${report.worstResource} storage needs urgent attention.`,
    );
  }

  return lines.join("\n");
}
