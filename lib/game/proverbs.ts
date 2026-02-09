import type { ColonyStatus } from "@/types/game";
import { rollSeeded } from "./seededRng";

// ─── Types ──────────────────────────────────────────────────────────────────

export type ProverbCategory = "celebration" | "survival" | "hope" | "memorial";

export interface DailyWisdom {
  proverb: string;
  category: ProverbCategory;
  resourceWarning: string | null;
  attribution: string;
}

export interface ResourceLevels {
  food: number;
  water: number;
  herbs: number;
}

// ─── Constants ──────────────────────────────────────────────────────────────

const CRITICAL_THRESHOLD = 10;

const PROVERBS: Record<ProverbCategory, string[]> = {
  celebration: [
    "A full belly makes a bold heart",
    "Many paws make light work",
    "The colony that feasts together stays together",
    "When the hunting is good, share the bounty",
    "A warm den and a full store — what more could a cat ask?",
    "Prosperity is the reward of the vigilant",
    "The mice run slow when the colony runs strong",
  ],
  survival: [
    "Even in lean times, a sharp claw finds prey",
    "The colony that endures the storm grows stronger roots",
    "Hunger sharpens the senses, but dulls the spirit",
    "A thin belly teaches patience to the hunter",
    "When the river runs dry, the wise cat digs deeper",
    "Hardship is the forge that tempers the colony",
    "The darkest night reveals the brightest stars",
  ],
  hope: [
    "Every great colony began with two cats and a dream",
    "The first den is always the most precious",
    "Small beginnings hold the seeds of greatness",
    "A single pawprint marks the start of a great journey",
    "New roots grow deepest when planted with care",
    "The youngest colony carries the oldest hopes",
    "From one spark, a thousand fires may kindle",
  ],
  memorial: [
    "They lived as one, and the wind carries their song",
    "A colony's true measure is the love it leaves behind",
    "Gone from the den, but never from the heart",
    "The bravest cats live longest in memory",
    "Even fallen colonies leave pawprints in the earth",
    "What was built with love cannot truly be lost",
    "The stars remember every colony that dared to dream",
  ],
};

const FOOD_PROVERBS = [
  "An empty belly knows no loyalty",
  "The larder grows thin — hunt while you can",
  "When the mice vanish, the colony must adapt",
  "A hungry colony is a desperate colony",
  "The hunt must not rest while kittens cry",
];

const WATER_PROVERBS = [
  "A dry tongue speaks only of thirst",
  "Without water, even the strongest paws falter",
  "The stream cares not for the colony's needs",
  "Thirst is the cruelest teacher",
  "Find the water, or the water finds you gone",
];

const HERB_PROVERBS = [
  "A healer without herbs is a prayer without hope",
  "The garden lies bare — illness waits for no cat",
  "When the herbs run out, the wise cat rests carefully",
  "Medicine grows scarce; let no cat wander recklessly",
  "An empty herb store is a wound waiting to fester",
];

const ATTRIBUTIONS: Record<ProverbCategory, string> = {
  celebration: "— Elder wisdom",
  survival: "— Colony saying",
  hope: "— Founder's proverb",
  memorial: "— In memoriam",
};

// ─── Functions ──────────────────────────────────────────────────────────────

export function getProverbCategory(status: ColonyStatus): ProverbCategory {
  const mapping: Record<ColonyStatus, ProverbCategory> = {
    thriving: "celebration",
    struggling: "survival",
    starting: "hope",
    dead: "memorial",
  };
  return mapping[status];
}

export function getProverbCount(category: ProverbCategory): number {
  return PROVERBS[category].length;
}

export function getColonyProverb(status: ColonyStatus, seed: number): string {
  const category = getProverbCategory(status);
  const list = PROVERBS[category];
  // Chain two rolls with prime multiplier for better entropy from small seeds
  const first = rollSeeded(seed);
  const { value } = rollSeeded(first.nextSeed * 7919);
  const index = Math.floor(value * list.length);
  return list[index];
}

export function getResourceProverb(
  resources: ResourceLevels,
  seed: number,
): string | null {
  const criticalResources: string[][] = [];

  if (resources.food <= CRITICAL_THRESHOLD)
    criticalResources.push(FOOD_PROVERBS);
  if (resources.water <= CRITICAL_THRESHOLD)
    criticalResources.push(WATER_PROVERBS);
  if (resources.herbs <= CRITICAL_THRESHOLD)
    criticalResources.push(HERB_PROVERBS);

  if (criticalResources.length === 0) return null;

  // Pick from the first critical resource's proverbs using the seed
  const first = rollSeeded(seed);
  const { value } = rollSeeded(first.nextSeed * 7919);
  const list = criticalResources[0];
  const index = Math.floor(value * list.length);
  return list[index];
}

export function getDailyWisdom(
  status: ColonyStatus,
  resources: ResourceLevels,
  daySeed: number,
): DailyWisdom {
  const category = getProverbCategory(status);
  const proverb = getColonyProverb(status, daySeed);
  const resourceWarning = getResourceProverb(resources, daySeed);
  const attribution = ATTRIBUTIONS[category];

  return {
    proverb,
    category,
    resourceWarning,
    attribution,
  };
}

export function formatEditorialFooter(wisdom: DailyWisdom): string {
  const lines: string[] = [];
  lines.push(`"${wisdom.proverb}"`);
  lines.push(wisdom.attribution);
  if (wisdom.resourceWarning) {
    lines.push(`[${wisdom.resourceWarning}]`);
  }
  return lines.join("\n");
}
