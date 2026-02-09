/**
 * Cat Personality Profile System
 *
 * Determines personality archetypes from CatStats distribution,
 * generates descriptions, quirks, compatibility ratings, and
 * formatted newspaper profile columns.
 */

import type { CatStats, CatSpecialization, LifeStage } from "@/types/game";

// =============================================================================
// Types
// =============================================================================

export type PersonalityArchetype =
  | "adventurer"
  | "guardian"
  | "scholar"
  | "socialite"
  | "mystic"
  | "worker"
  | "generalist";

export type CompatibilityLevel = "perfect" | "good" | "neutral" | "poor";

export interface PersonalityProfile {
  archetype: PersonalityArchetype;
  title: string;
  description: string;
  quirk: string;
  dominantStats: { primary: string; secondary: string };
}

interface PersonalityCatData {
  name: string;
  stats: CatStats;
  lifeStage: LifeStage;
  specialization: CatSpecialization;
}

// =============================================================================
// Archetype Mapping
// =============================================================================

// Each archetype is defined by a primary+secondary stat pair
const ARCHETYPE_MAP: Array<{
  primary: keyof CatStats;
  secondary: keyof CatStats;
  archetype: PersonalityArchetype;
}> = [
  { primary: "hunting", secondary: "vision", archetype: "adventurer" },
  { primary: "defense", secondary: "attack", archetype: "guardian" },
  { primary: "attack", secondary: "defense", archetype: "guardian" },
  { primary: "medicine", secondary: "vision", archetype: "scholar" },
  { primary: "leadership", secondary: "cleaning", archetype: "socialite" },
  { primary: "cleaning", secondary: "leadership", archetype: "socialite" },
  { primary: "vision", secondary: "medicine", archetype: "mystic" },
  { primary: "building", secondary: "cleaning", archetype: "worker" },
  { primary: "cleaning", secondary: "building", archetype: "worker" },
];

// =============================================================================
// Archetype Titles
// =============================================================================

const ARCHETYPE_TITLES: Record<PersonalityArchetype, string> = {
  adventurer: "The Adventurer",
  guardian: "The Guardian",
  scholar: "The Scholar",
  socialite: "The Socialite",
  mystic: "The Mystic",
  worker: "The Worker",
  generalist: "The Generalist",
};

// =============================================================================
// Descriptions (standard = young/adult, with kitten/elder variants)
// =============================================================================

const DESCRIPTIONS: Record<
  PersonalityArchetype,
  { standard: string; kitten: string; elder: string }
> = {
  adventurer: {
    standard:
      "Always first to explore unknown territory, driven by an insatiable curiosity.",
    kitten:
      "Already tumbling toward every new sound and shadow, a born explorer in the making.",
    elder:
      "Years of wandering have mapped every trail, yet the horizon still calls.",
  },
  guardian: {
    standard:
      "Stands watch when others sleep, a tireless protector of the colony.",
    kitten:
      "Tiny but fierce, already standing guard over littermates with puffed-up fur.",
    elder:
      "A lifetime of vigilance has earned deep respect; the colony's living shield.",
  },
  scholar: {
    standard:
      "Studies herbs by moonlight, always seeking deeper knowledge of the natural world.",
    kitten:
      "Sniffs every leaf and pebble with intense focus, a budding naturalist.",
    elder:
      "The colony's living encyclopedia, consulted on every ailment and remedy.",
  },
  socialite: {
    standard:
      "Knows every cat's business and keeps the colony's social fabric strong.",
    kitten:
      "Already grooming everyone in sight and meowing for attention at all hours.",
    elder:
      "The grand connector — no alliance or feud escapes their watchful ear.",
  },
  mystic: {
    standard:
      "Stares at the moon for hours, sensing things others cannot perceive.",
    kitten:
      "Gazes at dust motes with an unsettling intensity that unnerves the elders.",
    elder:
      "Whispered to commune with spirits; the colony seeks their counsel on omens.",
  },
  worker: {
    standard:
      "Never seen without something to fix, the backbone of colony infrastructure.",
    kitten: "Drags sticks and leaves into elaborate piles before anyone asks.",
    elder:
      "Every wall and den bears their craftsmanship; retirement is not in their vocabulary.",
  },
  generalist: {
    standard:
      "Jack of all trades, master of none — adaptable and quietly dependable.",
    kitten: "Tries everything once, excels at nothing yet, but never gives up.",
    elder:
      "A lifetime of balanced contribution; the cat every team is glad to have.",
  },
};

// =============================================================================
// Quirks
// =============================================================================

const QUIRKS: Record<PersonalityArchetype, string> = {
  adventurer: "Brings back strange objects from solo expeditions.",
  guardian: "Hisses at leaves that blow too close to the den.",
  scholar: "Arranges herb piles in alphabetical order (by scent).",
  socialite: "Grooms other cats mid-conversation without warning.",
  mystic: "Sits perfectly still for so long that birds land on them.",
  worker: "Rearranges the bedding every single night.",
  generalist: "Can never decide which patrol to join first.",
};

const SPECIALIZED_QUIRKS: Record<string, string> = {
  "adventurer:hunter": "Returns from hunts with trophies no one asked for.",
  "worker:architect": "Critiques every structure and rebuilds it 'properly'.",
  "mystic:ritualist":
    "Chants softly during thunderstorms and claims to control the rain.",
};

// =============================================================================
// Compatibility Matrix
// =============================================================================

// Stored as sorted key pairs → level. Same archetype = "perfect".
const COMPATIBILITY_PAIRS: Record<string, CompatibilityLevel> = {
  "adventurer:guardian": "good",
  "adventurer:mystic": "good",
  "adventurer:scholar": "neutral",
  "adventurer:socialite": "neutral",
  "adventurer:worker": "neutral",
  "guardian:scholar": "good",
  "guardian:socialite": "neutral",
  "guardian:mystic": "neutral",
  "guardian:worker": "good",
  "mystic:scholar": "good",
  "mystic:socialite": "poor",
  "mystic:worker": "poor",
  "scholar:socialite": "neutral",
  "scholar:worker": "neutral",
  "socialite:worker": "good",
};

// =============================================================================
// Functions
// =============================================================================

/**
 * Identify the two highest stats by value.
 * On ties, earlier keys in CatStats insertion order win.
 */
export function getDominantStats(stats: CatStats): {
  primary: string;
  secondary: string;
} {
  const entries = Object.entries(stats) as [keyof CatStats, number][];

  let primaryKey = entries[0][0];
  let primaryVal = entries[0][1];
  let secondaryKey = entries[1][0];
  let secondaryVal = entries[1][1];

  // Ensure primary >= secondary initially
  if (secondaryVal > primaryVal) {
    [primaryKey, primaryVal, secondaryKey, secondaryVal] = [
      secondaryKey,
      secondaryVal,
      primaryKey,
      primaryVal,
    ];
  }

  for (let i = 2; i < entries.length; i++) {
    const [key, val] = entries[i];
    if (val > primaryVal) {
      secondaryKey = primaryKey;
      secondaryVal = primaryVal;
      primaryKey = key;
      primaryVal = val;
    } else if (val > secondaryVal) {
      secondaryKey = key;
      secondaryVal = val;
    }
  }

  return { primary: primaryKey, secondary: secondaryKey };
}

/**
 * Determine personality archetype from stat distribution.
 * Checks primary+secondary against known archetype patterns.
 * Returns "generalist" if no clear dominant pair exists or no pattern matches.
 */
export function getPersonalityArchetype(stats: CatStats): PersonalityArchetype {
  const { primary, secondary } = getDominantStats(stats);

  // Check that the dominant pair is actually above the rest
  const values = Object.entries(stats) as [keyof CatStats, number][];
  const primaryVal = stats[primary as keyof CatStats];
  const thirdHighest = values
    .filter(([k]) => k !== primary && k !== secondary)
    .reduce((max, [, v]) => Math.max(max, v), 0);

  // If primary doesn't exceed the third-highest, no clear dominance
  if (primaryVal <= thirdHighest) return "generalist";

  for (const mapping of ARCHETYPE_MAP) {
    if (mapping.primary === primary && mapping.secondary === secondary) {
      return mapping.archetype;
    }
  }

  return "generalist";
}

/**
 * Get a life-stage-adjusted personality description.
 * Young and adult share the standard description.
 */
export function getPersonalityDescription(
  archetype: PersonalityArchetype,
  lifeStage: LifeStage,
): string {
  const desc = DESCRIPTIONS[archetype];
  if (lifeStage === "kitten") return desc.kitten;
  if (lifeStage === "elder") return desc.elder;
  return desc.standard;
}

/**
 * Get a humorous quirk for the archetype.
 * Returns a specialized quirk when archetype+specialization match.
 */
export function getPersonalityQuirk(
  archetype: PersonalityArchetype,
  specialization: CatSpecialization,
): string {
  if (specialization) {
    const key = `${archetype}:${specialization}`;
    if (key in SPECIALIZED_QUIRKS) {
      return SPECIALIZED_QUIRKS[key];
    }
  }
  return QUIRKS[archetype];
}

/**
 * Determine compatibility between two archetypes.
 * Same archetype = "perfect". Generalist with anyone = "neutral".
 */
export function getCompatibility(
  archetype1: PersonalityArchetype,
  archetype2: PersonalityArchetype,
): CompatibilityLevel {
  if (archetype1 === archetype2) return "perfect";
  if (archetype1 === "generalist" || archetype2 === "generalist")
    return "neutral";

  // Sort to create canonical key
  const sorted = [archetype1, archetype2].sort();
  const key = `${sorted[0]}:${sorted[1]}`;

  return COMPATIBILITY_PAIRS[key] ?? "neutral";
}

/**
 * Assemble a complete personality profile for newspaper display.
 */
export function formatPersonalityProfile(
  catData: PersonalityCatData,
): PersonalityProfile {
  const archetype = getPersonalityArchetype(catData.stats);
  const dominantStats = getDominantStats(catData.stats);
  const description = getPersonalityDescription(archetype, catData.lifeStage);
  const quirk = getPersonalityQuirk(archetype, catData.specialization);
  const title = `${catData.name} — ${ARCHETYPE_TITLES[archetype]}`;

  return {
    archetype,
    title,
    description,
    quirk,
    dominantStats,
  };
}
