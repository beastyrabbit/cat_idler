/**
 * Cat Legacy Score System
 *
 * Pure functions for calculating a cat's lifetime contribution to the colony.
 * Legacy score determines how the cat is remembered in colony history.
 * Includes Hall of Fame ranking and newspaper column generation.
 */

// --- Types ---

export type LegacyTier = "forgotten" | "remembered" | "honored" | "legendary";

export type ContributionType =
  | "survival"
  | "offspring"
  | "tasks"
  | "encounters"
  | "leadership"
  | "buildings";

export interface LegacyEntry {
  catName: string;
  score: number;
  tier: LegacyTier;
  topContribution: ContributionType;
  rank: number;
}

export interface HallOfFameReport {
  totalEntries: number;
  legendaryCount: number;
  honoredCount: number;
  topCat: string | null;
  averageScore: number;
  dominantContribution: ContributionType | null;
}

// --- Constants ---

const POINTS_PER_SURVIVAL_DAY = 1;
const POINTS_PER_OFFSPRING = 10;
const POINTS_PER_TASK = 2;
const POINTS_PER_ENCOUNTER = 5;
const POINTS_PER_LEADERSHIP_DAY = 3;
const POINTS_PER_BUILDING = 8;

// --- Functions ---

/**
 * Calculate a cat's legacy score from lifetime contributions.
 * Each contribution category has a different point weight.
 * Negative inputs are clamped to 0. Result is clamped to 0+.
 */
export function calculateLegacyScore(
  survivalDays: number,
  offspring: number,
  tasksCompleted: number,
  encountersSurvived: number,
  leadershipDays: number,
  buildingsHelped: number,
): number {
  const safe = (n: number) => Math.max(0, n);
  const score =
    safe(survivalDays) * POINTS_PER_SURVIVAL_DAY +
    safe(offspring) * POINTS_PER_OFFSPRING +
    safe(tasksCompleted) * POINTS_PER_TASK +
    safe(encountersSurvived) * POINTS_PER_ENCOUNTER +
    safe(leadershipDays) * POINTS_PER_LEADERSHIP_DAY +
    safe(buildingsHelped) * POINTS_PER_BUILDING;
  return Math.max(0, score);
}

/**
 * Map a legacy score to a tier label.
 */
export function getLegacyTier(score: number): LegacyTier {
  if (score >= 100) return "legendary";
  if (score >= 50) return "honored";
  if (score >= 20) return "remembered";
  return "forgotten";
}

const TIER_CONTRIBUTION_LABELS: Record<ContributionType, string> = {
  survival: "Survivor",
  offspring: "Parent",
  tasks: "Worker",
  encounters: "Fighter",
  leadership: "Leader",
  buildings: "Builder",
};

/**
 * Compose a legacy title like "Whiskers the Legendary Survivor".
 * Each tier produces a different format.
 */
export function composeLegacyTitle(
  catName: string,
  tier: LegacyTier,
  topContribution: ContributionType,
): string {
  const label = TIER_CONTRIBUTION_LABELS[topContribution];
  switch (tier) {
    case "legendary":
      return `${catName} the Legendary ${label}`;
    case "honored":
      return `${catName}, Honored ${label}`;
    case "remembered":
      return `${catName}, Remembered ${label}`;
    case "forgotten":
      return `${catName}, a Forgotten ${label}`;
  }
}

/**
 * Sort legacy entries by score descending and assign ranks.
 * Tied scores receive the same rank (dense ranking).
 */
export function rankLegacies(legacies: LegacyEntry[]): LegacyEntry[] {
  if (legacies.length === 0) return [];

  const sorted = [...legacies].sort((a, b) => b.score - a.score);
  let currentRank = 1;

  return sorted.map((entry, i) => {
    if (i > 0 && entry.score < sorted[i - 1].score) {
      currentRank = i + 1;
    }
    return { ...entry, rank: currentRank };
  });
}

/**
 * Evaluate a collection of legacy entries into a Hall of Fame summary report.
 */
export function evaluateHallOfFame(legacies: LegacyEntry[]): HallOfFameReport {
  if (legacies.length === 0) {
    return {
      totalEntries: 0,
      legendaryCount: 0,
      honoredCount: 0,
      topCat: null,
      averageScore: 0,
      dominantContribution: null,
    };
  }

  const legendaryCount = legacies.filter((e) => e.tier === "legendary").length;
  const honoredCount = legacies.filter((e) => e.tier === "honored").length;

  const topEntry = legacies.reduce((best, e) =>
    e.score > best.score ? e : best,
  );

  const totalScore = legacies.reduce((sum, e) => sum + e.score, 0);
  const averageScore = totalScore / legacies.length;

  // Find dominant contribution type by frequency
  const counts = new Map<ContributionType, number>();
  for (const entry of legacies) {
    counts.set(
      entry.topContribution,
      (counts.get(entry.topContribution) ?? 0) + 1,
    );
  }
  let dominantContribution: ContributionType | null = null;
  let maxCount = 0;
  for (const [type, count] of counts) {
    if (count > maxCount) {
      maxCount = count;
      dominantContribution = type;
    }
  }

  return {
    totalEntries: legacies.length,
    legendaryCount,
    honoredCount,
    topCat: topEntry.catName,
    averageScore,
    dominantContribution,
  };
}

/**
 * Generate a newspaper "Hall of Fame" column from a report.
 */
export function generateHallOfFameColumn(
  report: HallOfFameReport,
  colonyName: string,
): string {
  const lines: string[] = [];
  lines.push(`HALL OF FAME — ${colonyName}`);
  lines.push("");

  if (report.totalEntries === 0) {
    lines.push(
      `The ${colonyName} colony has yet to inscribe any names in its Hall of Fame.`,
    );
    lines.push("No legends have been written — only time will tell.");
    return lines.join("\n");
  }

  lines.push(
    `${report.totalEntries} cats have earned their place in ${colonyName}'s history.`,
  );

  if (report.legendaryCount > 0) {
    lines.push(
      `Among them, ${report.legendaryCount} legendary ${report.legendaryCount === 1 ? "hero stands" : "heroes stand"} above all others.`,
    );
  }

  if (report.honoredCount > 0) {
    lines.push(
      `${report.honoredCount} honored ${report.honoredCount === 1 ? "cat is" : "cats are"} remembered for their service.`,
    );
  }

  if (report.topCat) {
    lines.push("");
    lines.push(
      `The greatest of all: ${report.topCat}, with a legacy score of ${Math.round(report.averageScore)}.`,
    );
  }

  if (report.dominantContribution) {
    lines.push(
      `Most cats are remembered for their ${report.dominantContribution}.`,
    );
  }

  return lines.join("\n");
}
