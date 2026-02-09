/**
 * Colony Horoscope System
 *
 * Generates daily zodiac-style predictions for The Catford Examiner.
 * 12 cat-themed zodiac signs based on birth month (30-day months, relative to colony age).
 * Deterministic fortunes from colony seed + day number using seeded RNG.
 */

import { rollSeeded } from "./seededRng";

// =============================================================================
// Types
// =============================================================================

export interface ZodiacSign {
  name: string;
  symbol: string;
  month: number;
  trait: string;
}

export type FortuneSeverity =
  | "blessed"
  | "favorable"
  | "neutral"
  | "challenging"
  | "ominous";

export interface DailyFortune {
  prediction: string;
  severity: FortuneSeverity;
  luckyActivity: string;
}

export type CompatibilityLevel =
  | "soulmates"
  | "allies"
  | "neutral"
  | "rivals"
  | "nemeses";

export interface Compatibility {
  level: CompatibilityLevel;
  description: string;
}

export interface HoroscopeColumn {
  sectionTitle: string;
  entries: string[];
  dayNumber: number;
}

// =============================================================================
// Constants
// =============================================================================

const MS_PER_DAY = 24 * 60 * 60 * 1000;
const DAYS_PER_MONTH = 30;
const MONTHS_PER_YEAR = 12;

export const ZODIAC_SIGNS: ZodiacSign[] = [
  {
    name: "The Prowler",
    symbol: "\u2694",
    month: 1,
    trait: "adventurous, restless",
  },
  { name: "The Sunbeam", symbol: "\u2600", month: 2, trait: "warm, lazy" },
  {
    name: "The Shadow",
    symbol: "\u263D",
    month: 3,
    trait: "mysterious, independent",
  },
  { name: "The Whisker", symbol: "\u2042", month: 4, trait: "curious, social" },
  { name: "The Dreamer", symbol: "\u2601", month: 5, trait: "creative, aloof" },
  { name: "The Hunter", symbol: "\u2192", month: 6, trait: "focused, patient" },
  {
    name: "The Groomer",
    symbol: "\u2766",
    month: 7,
    trait: "meticulous, caring",
  },
  {
    name: "The Sentinel",
    symbol: "\u2720",
    month: 8,
    trait: "watchful, brave",
  },
  {
    name: "The Wanderer",
    symbol: "\u2737",
    month: 9,
    trait: "free-spirited, adaptable",
  },
  { name: "The Healer", symbol: "\u2695", month: 10, trait: "gentle, wise" },
  {
    name: "The Trickster",
    symbol: "\u2605",
    month: 11,
    trait: "playful, clever",
  },
  {
    name: "The Elder",
    symbol: "\u2234",
    month: 12,
    trait: "dignified, contemplative",
  },
];

const PREDICTIONS: Record<FortuneSeverity, string[]> = {
  blessed: [
    "The stars align in your favor — abundance flows freely today.",
    "A rare blessing descends upon you. All endeavors shall prosper.",
    "Fortune smiles upon the bold. Today is your day to shine.",
    "The cosmos grants you a gift. Accept it with open paws.",
  ],
  favorable: [
    "Good tidings are carried on the wind. Stay alert for opportunity.",
    "The path ahead is clear and warm. Stride forward with confidence.",
    "A friendly face brings unexpected joy before sundown.",
    "Your efforts will bear fruit today. Keep your whiskers keen.",
  ],
  neutral: [
    "The stars are quiet — a day for steady paws and calm hearts.",
    "Neither storm nor sunshine. Tend to your duties with care.",
    "Balance governs the day. What you give, you shall receive in kind.",
    "An uneventful day may be a blessing in disguise.",
  ],
  challenging: [
    "Clouds gather on the horizon. Conserve your strength for what lies ahead.",
    "A trial awaits, but strength is forged in adversity.",
    "Be wary of overconfidence. The clever cat checks twice.",
    "The day demands patience. What is delayed is not denied.",
  ],
  ominous: [
    "Dark omens stir in the shadows. Tread carefully today.",
    "The stars warn of misfortune. Stay close to trusted companions.",
    "An ill wind blows. Guard what you hold dear.",
    "The cosmos counsels retreat. Not every battle must be fought.",
  ],
};

const LUCKY_ACTIVITIES = [
  "hunting",
  "resting",
  "exploring",
  "guarding",
  "grooming",
  "building",
  "healing",
  "patrolling",
  "socializing",
  "foraging",
];

const SEVERITY_ORDER: FortuneSeverity[] = [
  "blessed",
  "favorable",
  "neutral",
  "challenging",
  "ominous",
];

// Compatibility matrix: difference in month positions → level
// Pairs with distance 6 (opposite) = soulmates, 1 = allies, 3/4 = rivals, 5 = nemeses, rest = neutral
const COMPATIBILITY_BY_DISTANCE: Record<number, CompatibilityLevel> = {
  0: "allies", // same sign
  1: "allies",
  2: "neutral",
  3: "rivals",
  4: "rivals",
  5: "nemeses",
  6: "soulmates",
};

const COMPATIBILITY_DESCRIPTIONS: Record<CompatibilityLevel, string[]> = {
  soulmates: [
    "A bond written in the stars — inseparable and unshakeable.",
    "Opposite forces that complete each other perfectly.",
    "Together they are greater than the sum of their parts.",
  ],
  allies: [
    "Natural companions who understand each other's rhythms.",
    "A warm friendship built on shared instincts.",
    "Side by side, they face the world with confidence.",
  ],
  neutral: [
    "Neither drawn together nor pushed apart — a polite coexistence.",
    "They pass like ships in the night, respectful but distant.",
    "A civil acquaintance with room to grow.",
  ],
  rivals: [
    "A friction that sparks competition — sometimes productive, sometimes not.",
    "They challenge each other, for better or for worse.",
    "Clashing temperaments make for uneasy alliances.",
  ],
  nemeses: [
    "The stars themselves conspire to set them at odds.",
    "A fundamental clash of nature that few can bridge.",
    "Best kept apart, lest sparks fly and fur is ruffled.",
  ],
};

// =============================================================================
// Functions
// =============================================================================

/**
 * Determine a cat's zodiac sign from their birth time relative to colony creation.
 * Each month is 30 days. 12 months cycle, wrapping after 360 days.
 */
export function getCatZodiacSign(
  birthTime: number,
  colonyCreatedAt: number,
): ZodiacSign {
  const daysSincColony = Math.floor(
    Math.max(0, birthTime - colonyCreatedAt) / MS_PER_DAY,
  );
  const dayInYear = daysSincColony % (DAYS_PER_MONTH * MONTHS_PER_YEAR);
  const monthIndex = Math.floor(dayInYear / DAYS_PER_MONTH);
  return ZODIAC_SIGNS[monthIndex];
}

/**
 * Generate a deterministic daily fortune for a zodiac sign.
 * Uses colony seed + day number + sign month as combined seed.
 */
export function getDailyFortune(
  sign: ZodiacSign,
  colonySeed: number,
  dayNumber: number,
): DailyFortune {
  // Combine seed with day and sign for unique per-sign-per-day results
  const combinedSeed = colonySeed * 7919 + dayNumber * 131 + sign.month * 37;

  // Roll for severity
  const severityRoll = rollSeeded(combinedSeed);
  const severityIndex = Math.floor(severityRoll.value * SEVERITY_ORDER.length);
  const severity = SEVERITY_ORDER[severityIndex];

  // Roll for prediction within severity category
  const predictionRoll = rollSeeded(severityRoll.nextSeed);
  const predictions = PREDICTIONS[severity];
  const predictionIndex = Math.floor(predictionRoll.value * predictions.length);
  const prediction = predictions[predictionIndex];

  // Roll for lucky activity
  const activityRoll = rollSeeded(predictionRoll.nextSeed);
  const activityIndex = Math.floor(
    activityRoll.value * LUCKY_ACTIVITIES.length,
  );
  const luckyActivity = LUCKY_ACTIVITIES[activityIndex];

  return { prediction, severity, luckyActivity };
}

/**
 * Calculate compatibility between two zodiac signs.
 * Based on the circular distance between their month positions.
 * Symmetric: sign1+sign2 = sign2+sign1.
 */
export function getSignCompatibility(
  sign1: ZodiacSign,
  sign2: ZodiacSign,
): Compatibility {
  const rawDistance = Math.abs(sign1.month - sign2.month);
  const circularDistance = Math.min(rawDistance, MONTHS_PER_YEAR - rawDistance);
  const level = COMPATIBILITY_BY_DISTANCE[circularDistance];

  // Deterministic description from combined months (order-independent)
  const descSeed =
    Math.min(sign1.month, sign2.month) * 100 +
    Math.max(sign1.month, sign2.month);
  const descriptions = COMPATIBILITY_DESCRIPTIONS[level];
  const descIndex = descSeed % descriptions.length;
  const description = descriptions[descIndex];

  return { level, description };
}

/**
 * Format a single horoscope entry for one zodiac sign.
 */
export function formatHoroscopeEntry(
  sign: ZodiacSign,
  fortune: DailyFortune,
): string {
  return `${sign.symbol} ${sign.name} (${sign.trait})\n${fortune.prediction}\nLucky activity: ${fortune.luckyActivity}`;
}

/**
 * Assemble all entries into a complete newspaper horoscope column.
 */
export function formatHoroscopeColumn(
  entries: string[],
  colonyName: string,
  dayNumber: number,
): HoroscopeColumn {
  return {
    sectionTitle: `${colonyName} Daily Stars & Horoscope`,
    entries,
    dayNumber,
  };
}
