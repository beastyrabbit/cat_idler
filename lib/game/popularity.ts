/**
 * Cat Popularity Contest System
 *
 * Pure functions for ranking cats by composite popularity score.
 * Combines social connections, heroic achievements, charm traits,
 * and fame events into a fun "Cat of the Month" ranking.
 */

// --- Types ---

export type PopularityTier =
  | "unknown"
  | "noticed"
  | "popular"
  | "beloved"
  | "legendary";

export interface PopularityFactors {
  socialScore: number;
  heroScore: number;
  charmScore: number;
  fameScore: number;
}

export interface PopularityEntry {
  catName: string;
  factors: PopularityFactors;
  totalScore: number;
  tier: PopularityTier;
}

// --- Constants ---

const MAX_FACTOR_SCORE = 25;

const CHARM_WEIGHTS: Record<string, number> = {
  friendly: 8,
  brave: 7,
  clever: 7,
  cautious: 3,
  curious: 4,
  playful: 5,
  gentle: 5,
  aggressive: 1,
  lazy: 1,
};

const DEFAULT_CHARM_WEIGHT = 3;

// --- Functions ---

/**
 * Calculate social score from relationship and mentor counts.
 * Each relationship = 2 pts, each mentorship = 3 pts, capped at 25.
 */
export function calculateSocialScore(
  relationshipCount: number,
  mentorCount: number,
): number {
  const raw = relationshipCount * 2 + mentorCount * 3;
  return Math.min(MAX_FACTOR_SCORE, Math.max(0, raw));
}

/**
 * Calculate hero score from combat victories and encounters survived.
 * Combat victories = 3 pts each, encounters = 1.5 pts each, capped at 25.
 */
export function calculateHeroScore(
  combatVictories: number,
  encountersSurvived: number,
): number {
  const raw = combatVictories * 3 + encountersSurvived * 1.5;
  return Math.min(MAX_FACTOR_SCORE, Math.max(0, raw));
}

/**
 * Calculate charm score from personality traits.
 * Friendly/brave/clever weigh more; aggressive/lazy weigh less.
 */
export function calculateCharmScore(traits: string[]): number {
  if (traits.length === 0) return 0;
  const raw = traits.reduce(
    (sum, t) => sum + (CHARM_WEIGHTS[t] ?? DEFAULT_CHARM_WEIGHT),
    0,
  );
  return Math.min(MAX_FACTOR_SCORE, Math.max(0, raw));
}

/**
 * Calculate fame score from event participation and leadership days.
 * Events = 1.5 pts each, leader days = 3 pts each, capped at 25.
 */
export function calculateFameScore(
  eventCount: number,
  leaderDays: number,
): number {
  const raw = eventCount * 1.5 + leaderDays * 3;
  return Math.min(MAX_FACTOR_SCORE, Math.max(0, raw));
}

/**
 * Classify total popularity score into a tier.
 */
export function getPopularityTier(score: number): PopularityTier {
  if (score >= 80) return "legendary";
  if (score >= 60) return "beloved";
  if (score >= 40) return "popular";
  if (score >= 20) return "noticed";
  return "unknown";
}

/**
 * Rank cats by total popularity score, descending. Stable sort for ties.
 * Returns a new array (does not mutate input).
 */
export function rankCatPopularity(
  entries: PopularityEntry[],
): PopularityEntry[] {
  return [...entries].sort((a, b) => b.totalScore - a.totalScore);
}

/**
 * Get the top-ranked cat (Cat of the Month).
 * Returns null if no entries.
 */
export function getCatOfTheMonth(
  entries: PopularityEntry[],
): PopularityEntry | null {
  if (entries.length === 0) return null;
  const ranked = rankCatPopularity(entries);
  return ranked[0];
}

/**
 * Generate a newspaper "Popularity Poll" column.
 * Shows top 5 cats with their tiers and highlights Cat of the Month.
 */
export function generatePopularityColumn(
  entries: PopularityEntry[],
  colonyName: string,
): string {
  const lines: string[] = [];
  lines.push(`=== ${colonyName} Popularity Poll ===`);
  lines.push("");

  if (entries.length === 0) {
    lines.push("No cats have entered the poll yet.");
    return lines.join("\n");
  }

  const ranked = rankCatPopularity(entries);
  const catOfMonth = ranked[0];

  lines.push(`Cat of the Month: ${catOfMonth.catName} (${catOfMonth.tier})`);
  lines.push("");
  lines.push("Top Rankings:");

  const top5 = ranked.slice(0, 5);
  top5.forEach((entry, i) => {
    lines.push(
      `  ${i + 1}. ${entry.catName} — ${entry.totalScore} pts [${entry.tier}]`,
    );
  });

  return lines.join("\n");
}
