/**
 * Cat Obituary System
 *
 * Pure functions for generating newspaper-style obituaries for deceased cats.
 * Feeds the "In Memoriam" section of The Catford Examiner.
 */

import type { CatStats, CatSpecialization, LifeStage } from "@/types/game";

// ─── Types ──────────────────────────────────────────────────────────

export type CauseOfDeath =
  | "starvation"
  | "dehydration"
  | "old_age"
  | "combat"
  | "unknown";

export interface ObituaryCatData {
  name: string;
  ageHours: number;
  lifeStage: LifeStage;
  specialization: CatSpecialization;
  stats: CatStats;
  causeOfDeath: CauseOfDeath;
}

export interface ObituaryColumn {
  title: string;
  lifeSummary: string;
  notableTrait: string;
  memorialQuote: string;
  serviceYears: string;
}

// ─── Cause of Death Labels ──────────────────────────────────────────

const CAUSE_LABELS: Record<CauseOfDeath, string> = {
  starvation: "Starvation",
  dehydration: "Dehydration",
  old_age: "Old Age",
  combat: "Combat",
  unknown: "Unknown Causes",
};

export function getCauseOfDeathLabel(cause: CauseOfDeath): string {
  return CAUSE_LABELS[cause];
}

// ─── Service Years ──────────────────────────────────────────────────

export function getServiceYears(ageHours: number): string {
  const totalHours = Math.floor(ageHours);

  if (totalHours < 1) return "less than an hour";

  const days = Math.floor(totalHours / 24);
  const hours = totalHours % 24;

  if (days === 0) {
    return hours === 1 ? "1 hour" : `${hours} hours`;
  }

  const dayStr = days === 1 ? "1 day" : `${days} days`;
  if (hours === 0) return dayStr;

  const hourStr = hours === 1 ? "1 hour" : `${hours} hours`;
  return `${dayStr}, ${hourStr}`;
}

// ─── Notable Trait ──────────────────────────────────────────────────

const STAT_DESCRIPTIONS: Record<keyof CatStats, string> = {
  attack: "Known for exceptional attack prowess",
  defense: "Renowned for steadfast defense",
  hunting: "A gifted hunting specialist",
  medicine: "Skilled in the healing arts of medicine",
  cleaning: "Dedicated to keeping the colony clean",
  building: "A talented building craftsman",
  leadership: "A natural-born leadership figure",
  vision: "Blessed with extraordinary vision",
};

export function getNotableTrait(stats: CatStats): {
  trait: string;
  label: string;
} {
  let bestTrait: keyof CatStats = "attack";
  let bestValue = -1;

  for (const key of Object.keys(stats) as (keyof CatStats)[]) {
    if (stats[key] > bestValue) {
      bestValue = stats[key];
      bestTrait = key;
    }
  }

  return {
    trait: bestTrait,
    label: STAT_DESCRIPTIONS[bestTrait],
  };
}

// ─── Life Summary ───────────────────────────────────────────────────

export function getLifeSummary(catData: ObituaryCatData): string {
  const { name, lifeStage, specialization, causeOfDeath } = catData;
  const causeLabel = getCauseOfDeathLabel(causeOfDeath).toLowerCase();
  const serviceStr = getServiceYears(catData.ageHours);

  const roleStr = specialization ? `, a dedicated ${specialization},` : "";

  if (lifeStage === "kitten") {
    return `${name}${roleStr} was a young kitten who served the colony for ${serviceStr}. Tragically taken by ${causeLabel}, this little one will be dearly missed.`;
  }

  return `${name}${roleStr} was a respected ${lifeStage} member of the colony who served faithfully for ${serviceStr}. Their passing due to ${causeLabel} leaves a void in our ranks.`;
}

// ─── Memorial Quote ─────────────────────────────────────────────────

const LIFE_STAGE_QUOTES: Record<LifeStage, string> = {
  kitten: "Gone too soon, but never forgotten.",
  young: "A light extinguished before its time.",
  adult: "They gave their best years to the colony.",
  elder: "A life well-lived, a legacy enduring.",
};

const CAUSE_QUOTES: Record<CauseOfDeath, string> = {
  starvation: "May no other soul know such hunger.",
  dehydration: "The water ran dry, but our memories will not.",
  old_age: "Rest now, faithful servant of the colony.",
  combat: "Fallen in battle, remembered in glory.",
  unknown: "The mystery of their passing deepens our sorrow.",
};

export function getMemorialQuote(
  lifeStage: LifeStage,
  cause: CauseOfDeath,
): string {
  // Cause-specific quotes take priority for combat and old_age
  if (cause === "combat" || cause === "old_age") {
    return CAUSE_QUOTES[cause];
  }

  // Life stage quotes for everything else, with cause fallback for adult
  if (lifeStage === "adult" && cause !== "unknown") {
    return CAUSE_QUOTES[cause];
  }

  return LIFE_STAGE_QUOTES[lifeStage];
}

// ─── Format Obituary Column ─────────────────────────────────────────

export function formatObituaryColumn(catData: ObituaryCatData): ObituaryColumn {
  const { name, lifeStage, causeOfDeath, stats, ageHours } = catData;
  const notable = getNotableTrait(stats);

  return {
    title: `In Memoriam: ${name}`,
    lifeSummary: getLifeSummary(catData),
    notableTrait: notable.label,
    memorialQuote: getMemorialQuote(lifeStage, causeOfDeath),
    serviceYears: getServiceYears(ageHours),
  };
}
