/**
 * Cat Relationship / Affinity Tracking System
 *
 * Pure functions for tracking cat friendship from shared tasks,
 * encounters, meals, and proximity. Affinity score (0-100) maps
 * to relationship tiers with productivity bonuses.
 */

export type RelationshipTier =
  | "stranger"
  | "acquaintance"
  | "friend"
  | "bonded";

export type AffinityEvent =
  | "shared_task"
  | "shared_encounter"
  | "shared_meal"
  | "proximity";

export interface RelationshipSummary {
  cat1Name: string;
  cat2Name: string;
  affinity: number;
  tier: RelationshipTier;
  productivityBonus: number;
  description: string;
}

const AFFINITY_EVENT_VALUES: Record<AffinityEvent, number> = {
  shared_task: 5,
  shared_encounter: 10,
  shared_meal: 3,
  proximity: 1,
} as const;

const TIER_BONUSES: Record<RelationshipTier, number> = {
  stranger: 1.0,
  acquaintance: 1.05,
  friend: 1.15,
  bonded: 1.25,
} as const;

const DECAY_HOURS_PER_POINT = 6;

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

/**
 * Map an affinity score (0-100) to a relationship tier.
 * Values outside 0-100 are clamped.
 */
export function getRelationshipTier(affinity: number): RelationshipTier {
  const clamped = clamp(affinity, 0, 100);
  if (clamped >= 80) return "bonded";
  if (clamped >= 50) return "friend";
  if (clamped >= 20) return "acquaintance";
  return "stranger";
}

/**
 * Get the productivity multiplier for a relationship tier.
 */
export function getProductivityBonus(tier: RelationshipTier): number {
  return TIER_BONUSES[tier];
}

/**
 * Apply an affinity event to the current score.
 * Result is clamped to 0-100.
 */
export function updateAffinity(current: number, event: AffinityEvent): number {
  return clamp(current + AFFINITY_EVENT_VALUES[event], 0, 100);
}

/**
 * Decay affinity toward 0 based on hours since last interaction.
 * Loses 1 point per 6 hours (integer division). Negative hours treated as 0.
 */
export function decayAffinity(
  current: number,
  hoursSinceLastInteraction: number,
): number {
  const safeHours = Math.max(0, hoursSinceLastInteraction);
  const decay = Math.floor(safeHours / DECAY_HOURS_PER_POINT);
  return Math.max(0, current - decay);
}

/**
 * Generate a human-readable relationship description for the newspaper.
 */
export function describeRelationship(
  cat1Name: string,
  cat2Name: string,
  tier: RelationshipTier,
): string {
  switch (tier) {
    case "stranger":
      return `${cat1Name} and ${cat2Name} are strangers — they have yet to cross paths.`;
    case "acquaintance":
      return `${cat1Name} and ${cat2Name} are acquaintances — they nod in passing.`;
    case "friend":
      return `${cat1Name} and ${cat2Name} are friends — often seen sharing a sunbeam.`;
    case "bonded":
      return `${cat1Name} and ${cat2Name} are bonded — an inseparable pair.`;
  }
}
