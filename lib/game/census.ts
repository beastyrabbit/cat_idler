/**
 * Colony Census & Demographics System
 *
 * Pure functions for analyzing population age distribution,
 * dependency ratio, median age, growth rate, workforce ratio,
 * and newspaper-style census headlines.
 */

import { getLifeStage } from "./age";

// =============================================================================
// Types
// =============================================================================

export interface AgeDistribution {
  kitten: number;
  young: number;
  adult: number;
  elder: number;
}

export interface CensusReport {
  totalPopulation: number;
  distribution: AgeDistribution;
  dependencyRatio: number;
  medianAge: number;
  workforceRatio: number;
}

// =============================================================================
// Functions
// =============================================================================

/**
 * Count cats per life stage from an array of ages (in hours).
 *
 * Uses the same life stage boundaries as getLifeStage:
 * - Kitten: 0-6h, Young: 6-24h, Adult: 24-48h, Elder: 48+h
 */
export function getAgeDistribution(catAges: number[]): AgeDistribution {
  const dist: AgeDistribution = { kitten: 0, young: 0, adult: 0, elder: 0 };
  for (const age of catAges) {
    const stage = getLifeStage(age);
    dist[stage]++;
  }
  return dist;
}

/**
 * Calculate dependency ratio: (kittens + elders) / (young + adults).
 *
 * Returns 0 when there are no dependents (or no population at all).
 * Returns Infinity when there are dependents but no workers.
 */
export function getDependencyRatio(distribution: AgeDistribution): number {
  const dependents = distribution.kitten + distribution.elder;
  const workers = distribution.young + distribution.adult;

  if (dependents === 0) return 0;
  if (workers === 0) return Infinity;

  return dependents / workers;
}

/**
 * Calculate median age from an array of cat ages (in hours).
 *
 * Returns 0 for empty array. Does not modify the input array.
 */
export function getMedianAge(catAges: number[]): number {
  if (catAges.length === 0) return 0;

  const sorted = [...catAges].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);

  if (sorted.length % 2 === 1) {
    return sorted[mid];
  }
  return (sorted[mid - 1] + sorted[mid]) / 2;
}

/**
 * Calculate population growth rate as a percentage.
 *
 * Formula: (births - deaths) / totalPopulation * 100
 * Returns 0 for zero population.
 */
export function getGrowthRate(
  births: number,
  deaths: number,
  totalPopulation: number,
): number {
  if (totalPopulation === 0) return 0;
  return ((births - deaths) / totalPopulation) * 100;
}

/**
 * Calculate the percentage of working-age cats (young + adults).
 *
 * Returns 0-100. Returns 0 for empty population.
 */
export function getWorkforceRatio(distribution: AgeDistribution): number {
  const total =
    distribution.kitten +
    distribution.young +
    distribution.adult +
    distribution.elder;

  if (total === 0) return 0;

  const workers = distribution.young + distribution.adult;
  return Math.round((workers / total) * 10000) / 100;
}

/**
 * Generate a newspaper-style census headline based on colony demographics.
 */
export function formatCensusHeadline(
  distribution: AgeDistribution,
  totalPopulation: number,
): string {
  if (totalPopulation === 0) {
    return "Ghost Colony: Census Finds No Living Cats";
  }

  const kittenRatio = distribution.kitten / totalPopulation;
  const elderRatio = distribution.elder / totalPopulation;
  const workerRatio =
    (distribution.young + distribution.adult) / totalPopulation;

  if (kittenRatio > 0.5) {
    return `Baby Boom! ${distribution.kitten} Kittens Overwhelm Colony Nurseries`;
  }

  if (elderRatio > 0.5) {
    return `Aging Population: Elders Now Outnumber Workers in Colony Census`;
  }

  if (workerRatio >= 0.7) {
    return `Strong Workforce Drives Colony Prosperity — ${totalPopulation} Cats Counted`;
  }

  if (totalPopulation <= 3) {
    return `Small but Mighty: Colony Census Records ${totalPopulation} Residents`;
  }

  return `Colony Census: ${totalPopulation} Cats Across All Generations`;
}
