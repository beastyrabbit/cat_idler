/**
 * Cat Birth Announcements System
 *
 * Newspaper-style birth announcements for The Catford Examiner.
 * Generates celebratory announcements with lineage info, inherited traits,
 * litter context, and "born under" personality omens based on colony status.
 *
 * Pure functions — no side effects, no randomness except seeded.
 */

import type { CatStats, ColonyStatus } from "@/types/game";
import { rollSeeded } from "./seededRng";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface BirthOmen {
  name: string;
  prediction: string;
  symbol: string;
}

export interface LitterAnnouncement {
  headline: string;
  kittenAnnouncements: string[];
  litterSize: number;
  parentNames: [string, string];
  omen: BirthOmen;
}

export interface BirthsColumn {
  sectionTitle: string;
  announcements: LitterAnnouncement[];
  totalBirths: number;
}

export interface LitterInfo {
  kittenNames: string[];
  parentNames: [string, string];
  colonyStatus: ColonyStatus;
  parentStats: CatStats;
  seed: number;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const NOTABLE_THRESHOLD = 70;

const TRAIT_MESSAGES: Record<string, string> = {
  hunting: "Born with a hunter's eye",
  medicine: "Healer's touch in the bloodline",
  leadership: "Destined for the leader's den",
  building: "Paws of a master builder",
  attack: "Fighter's spirit inherited",
  defense: "Guardian's instinct in the blood",
  vision: "Scout's keen eye passed down",
};

const THRIVING_PREDICTIONS = [
  "destined to be bold and generous",
  "born to lead with a full belly",
  "blessed with the colony's abundance",
  "fated for great and joyful deeds",
];

const STRUGGLING_PREDICTIONS = [
  "destined to be tough and resourceful",
  "born with grit in their bones",
  "shaped by hardship into something fierce",
  "fated to endure where others falter",
];

const STARTING_PREDICTIONS = [
  "destined to be adventurous and curious",
  "born to explore uncharted territory",
  "blessed with the spark of new beginnings",
  "fated to write the colony's first legends",
];

const DEAD_PREDICTIONS = [
  "born against all odds",
  "a miracle in the silence",
  "destined to carry the memory forward",
];

const OMEN_MAP: Record<
  ColonyStatus,
  { name: string; symbol: string; predictions: string[] }
> = {
  thriving: {
    name: "Star of Plenty",
    symbol: "★",
    predictions: THRIVING_PREDICTIONS,
  },
  struggling: {
    name: "Moon of Resilience",
    symbol: "☽",
    predictions: STRUGGLING_PREDICTIONS,
  },
  starting: {
    name: "Dawn of Promise",
    symbol: "☀",
    predictions: STARTING_PREDICTIONS,
  },
  dead: {
    name: "Shadow of Memory",
    symbol: "✦",
    predictions: DEAD_PREDICTIONS,
  },
};

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

/**
 * Get a birth omen based on colony status at time of birth.
 * Uses seeded RNG to pick a prediction from the status-appropriate list.
 */
export function getBirthOmen(status: ColonyStatus, seed: number): BirthOmen {
  const omenDef = OMEN_MAP[status];
  // Double-roll with prime multiplier for better entropy from small seeds
  const roll1 = rollSeeded(seed);
  const roll2 = rollSeeded(roll1.nextSeed * 7919);
  const index = Math.floor(roll2.value * omenDef.predictions.length);

  return {
    name: omenDef.name,
    symbol: omenDef.symbol,
    prediction: omenDef.predictions[index],
  };
}

/**
 * Identify notable inherited traits from parent stats.
 * Returns flavor text for each stat at or above the notable threshold (70).
 */
export function getInheritedTraits(parentStats: CatStats): string[] {
  const traits: string[] = [];

  for (const [stat, message] of Object.entries(TRAIT_MESSAGES)) {
    const value = parentStats[stat as keyof CatStats];
    if (value >= NOTABLE_THRESHOLD) {
      traits.push(message);
    }
  }

  return traits;
}

/**
 * Format a single kitten's birth announcement text.
 */
export function formatBirthAnnouncement(
  kittenName: string,
  parentNames: [string, string],
  omen: BirthOmen,
  traits: string[],
  litterSize: number,
): string {
  const lines: string[] = [];

  lines.push(
    `${omen.symbol} ${kittenName}, born to ${parentNames[0]} & ${parentNames[1]}`,
  );
  lines.push(`Born under the ${omen.name} — ${omen.prediction}.`);

  if (traits.length > 0) {
    lines.push(`Lineage notes: ${traits.join(". ")}.`);
  }

  if (litterSize > 1) {
    lines.push(`One of ${litterSize} in this litter.`);
  }

  return lines.join("\n");
}

/**
 * Assemble a complete litter announcement with all kittens and shared omen.
 */
export function formatLitterAnnouncement(info: LitterInfo): LitterAnnouncement {
  const { kittenNames, parentNames, colonyStatus, parentStats, seed } = info;
  const omen = getBirthOmen(colonyStatus, seed);
  const traits = getInheritedTraits(parentStats);
  const litterSize = kittenNames.length;
  const headline = getBirthHeadline(litterSize, "the colony");

  const kittenAnnouncements = kittenNames.map((name) =>
    formatBirthAnnouncement(name, parentNames, omen, traits, litterSize),
  );

  return {
    headline,
    kittenAnnouncements,
    litterSize,
    parentNames,
    omen,
  };
}

/**
 * Generate a newspaper headline for the birth section based on litter size.
 */
export function getBirthHeadline(
  litterSize: number,
  colonyName: string,
): string {
  if (litterSize === 1) {
    return `New Kitten Welcomed in ${colonyName}!`;
  }
  if (litterSize === 2) {
    return `Twins Born in ${colonyName}!`;
  }
  if (litterSize === 3) {
    return `Triplets Arrive in ${colonyName}!`;
  }
  return `${litterSize} Kittens Born in ${colonyName}!`;
}

/**
 * Assemble multiple litter announcements into a complete newspaper births section.
 */
export function formatBirthsColumn(
  announcements: LitterAnnouncement[],
  colonyName: string,
): BirthsColumn {
  const totalBirths = announcements.reduce((sum, a) => sum + a.litterSize, 0);

  return {
    sectionTitle: `BIRTHS & ANNOUNCEMENTS — ${colonyName}`,
    announcements,
    totalBirths,
  };
}
