/**
 * Resource Trends Analysis
 *
 * Pure functions for computing moving averages, trend direction,
 * and percentage change across colony resource snapshots.
 */

export type TrendDirection = "rising" | "falling" | "stable";

export interface ResourceSnapshot {
  food: number;
  water: number;
  herbs: number;
  materials: number;
  timestamp: number;
}

export interface ResourceTrendEntry {
  trend: TrendDirection;
  average: number;
  percentChange: number;
}

export interface ResourceTrendReport {
  food: ResourceTrendEntry;
  water: ResourceTrendEntry;
  herbs: ResourceTrendEntry;
  materials: ResourceTrendEntry;
}

const TREND_THRESHOLD = 5; // ±5% to count as rising/falling
const DEFAULT_WINDOW = 3;

/**
 * Compute simple moving average of the last `windowSize` values.
 * Returns 0 for empty input.
 */
export function getMovingAverage(values: number[], windowSize: number): number {
  if (values.length === 0) return 0;
  const window = values.slice(-Math.min(windowSize, values.length));
  const sum = window.reduce((a, b) => a + b, 0);
  return sum / window.length;
}

/**
 * Percentage change from previous to current.
 * Handles zero denominator: 0→0 = 0%, 0→positive = +100%, positive→0 = -100%.
 */
export function getPercentChange(previous: number, current: number): number {
  if (previous === 0 && current === 0) return 0;
  if (previous === 0) return 100;
  return ((current - previous) / previous) * 100;
}

/**
 * Compare two adjacent windows of size `windowSize` to determine trend.
 * Needs at least 2*windowSize data points; returns "stable" otherwise.
 * Uses strict >5% / <-5% thresholds (exactly ±5% counts as stable).
 */
export function getTrend(values: number[], windowSize: number): TrendDirection {
  if (values.length < windowSize * 2) return "stable";

  const recentWindow = values.slice(-windowSize);
  const previousWindow = values.slice(-windowSize * 2, -windowSize);

  const recentAvg =
    recentWindow.reduce((a, b) => a + b, 0) / recentWindow.length;
  const previousAvg =
    previousWindow.reduce((a, b) => a + b, 0) / previousWindow.length;

  const change = getPercentChange(previousAvg, recentAvg);

  if (change > TREND_THRESHOLD) return "rising";
  if (change < -TREND_THRESHOLD) return "falling";
  return "stable";
}

/**
 * Full trend analysis for all 4 resource types.
 * Uses a default window of 3 snapshots.
 */
export function analyzeResourceTrends(
  history: ResourceSnapshot[],
  windowSize: number = DEFAULT_WINDOW,
): ResourceTrendReport {
  const resources = ["food", "water", "herbs", "materials"] as const;

  const report = {} as ResourceTrendReport;

  for (const key of resources) {
    const values = history.map((s) => s[key]);
    report[key] = {
      trend: getTrend(values, windowSize),
      average: getMovingAverage(values, windowSize),
      percentChange:
        values.length >= windowSize * 2
          ? getPercentChange(
              getMovingAverage(
                values.slice(0, values.length - windowSize),
                windowSize,
              ),
              getMovingAverage(values, windowSize),
            )
          : 0,
    };
  }

  return report;
}
