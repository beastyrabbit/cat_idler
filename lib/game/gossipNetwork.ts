/**
 * Cat Gossip Network
 *
 * Pure functions for simulating gossip spread through the colony.
 * Cats generate rumors from events, gossip spreads based on chattiness,
 * and rumor reliability degrades with each retelling (telephone game effect).
 */

import { rollSeeded } from "@/lib/game/seededRng";

// --- Types ---

export type RumorType =
  | "food_sighting"
  | "predator_warning"
  | "romance"
  | "leadership"
  | "mystery"
  | "achievement";

export interface Rumor {
  type: RumorType;
  originalType: RumorType;
  sourceCatName: string;
  reliability: number; // 0-100
  retellings: number;
  message: string;
}

export interface GossipCat {
  name: string;
  chattiness: number; // 0-100
}

export interface GossipReport {
  totalRumors: number;
  reliableRumors: number; // reliability > 60
  distortedRumors: number; // originalType !== type
  mostGossipyCat: string | null;
  juiciestRumor: Rumor | null;
}

// --- Constants ---

const RUMOR_TYPES: RumorType[] = [
  "food_sighting",
  "predator_warning",
  "romance",
  "leadership",
  "mystery",
  "achievement",
];

const RUMOR_TEMPLATES: Record<RumorType, (name: string) => string> = {
  food_sighting: (name) =>
    `${name} claims to have spotted a huge fish near the river!`,
  predator_warning: (name) =>
    `${name} warns that a fox was seen lurking near the east wall.`,
  romance: (name) =>
    `${name} was seen sharing a mouse with someone by moonlight...`,
  leadership: (name) =>
    `${name} heard the leader has big plans for the colony.`,
  mystery: (name) => `${name} reports strange sounds coming from the old den.`,
  achievement: (name) => `${name} boasts about an incredible feat of skill!`,
};

const MIN_DEGRADE = 15;
const MAX_DEGRADE = 25;
const DISTORTION_THRESHOLD = 60;

// --- Functions ---

/**
 * Create an initial rumor from a colony event.
 */
export function generateRumor(
  eventType: RumorType,
  sourceCatName: string,
  _seed: number,
): Rumor {
  return {
    type: eventType,
    originalType: eventType,
    sourceCatName,
    reliability: 100,
    retellings: 0,
    message: RUMOR_TEMPLATES[eventType](sourceCatName),
  };
}

/**
 * Calculate chance (0-100) a cat passes along a rumor.
 * Chattier cats spread more; unreliable rumors spread less.
 */
export function calculateSpreadChance(
  chattiness: number,
  reliability: number,
): number {
  if (chattiness === 0 || reliability === 0) return 0;
  const raw = (chattiness / 100) * (reliability / 100) * 100;
  return Math.min(100, Math.max(0, Math.round(raw)));
}

/**
 * Reduce reliability per retelling by 15-25% (seeded RNG).
 */
export function degradeReliability(reliability: number, seed: number): number {
  const roll = rollSeeded(seed);
  const degradeAmount = MIN_DEGRADE + roll.value * (MAX_DEGRADE - MIN_DEGRADE);
  return Math.max(0, Math.round(reliability - degradeAmount));
}

/**
 * Possibly alter a rumor's content at low reliability.
 * Returns a new Rumor (does not mutate input).
 */
export function distortRumor(
  rumor: Rumor,
  reliability: number,
  seed: number,
): Rumor {
  const result: Rumor = {
    ...rumor,
    retellings: rumor.retellings + 1,
  };

  // High reliability: no distortion
  if (reliability > DISTORTION_THRESHOLD) {
    return result;
  }

  // Low reliability: chance of type change proportional to unreliability
  const roll = rollSeeded(seed);
  const distortChance =
    (DISTORTION_THRESHOLD - reliability) / DISTORTION_THRESHOLD;

  if (roll.value < distortChance) {
    // Pick a different type using next roll
    const typeRoll = rollSeeded(roll.nextSeed);
    const otherTypes = RUMOR_TYPES.filter((t) => t !== rumor.type);
    const newType = otherTypes[Math.floor(typeRoll.value * otherTypes.length)];
    result.type = newType;
    result.message = RUMOR_TEMPLATES[newType](rumor.sourceCatName);
  }

  return result;
}

/**
 * Evaluate gossip network summary statistics.
 */
export function evaluateGossipNetwork(
  cats: GossipCat[],
  rumors: Rumor[],
): GossipReport {
  const totalRumors = rumors.length;
  const reliableRumors = rumors.filter((r) => r.reliability > 60).length;
  const distortedRumors = rumors.filter(
    (r) => r.type !== r.originalType,
  ).length;

  // Most gossipy cat: highest chattiness
  let mostGossipyCat: string | null = null;
  if (cats.length > 0) {
    const sorted = [...cats].sort((a, b) => b.chattiness - a.chattiness);
    mostGossipyCat = sorted[0].name;
  }

  // Juiciest rumor: lowest reliability among distorted rumors
  let juiciestRumor: Rumor | null = null;
  const distorted = rumors.filter((r) => r.type !== r.originalType);
  if (distorted.length > 0) {
    const sorted = [...distorted].sort((a, b) => a.reliability - b.reliability);
    juiciestRumor = sorted[0];
  }

  return {
    totalRumors,
    reliableRumors,
    distortedRumors,
    mostGossipyCat,
    juiciestRumor,
  };
}

/**
 * Generate newspaper "Whiskers & Whispers" gossip column.
 */
export function generateGossipColumn(
  report: GossipReport,
  colonyName: string,
): string {
  const lines: string[] = [];
  lines.push(`=== WHISKERS & WHISPERS — ${colonyName} Colony Gossip ===`);
  lines.push("");

  if (report.totalRumors === 0) {
    lines.push(
      "An unusually quiet day in the colony. Not a whisper to be heard.",
    );
    lines.push(
      "Even the most talkative cats have nothing to report. Perhaps tomorrow will bring more excitement.",
    );
    return lines.join("\n");
  }

  lines.push(
    `Our correspondents have collected ${report.totalRumors} rumor${report.totalRumors === 1 ? "" : "s"} circulating through ${colonyName} today.`,
  );

  if (report.reliableRumors > 0) {
    lines.push(
      `Of these, ${report.reliableRumors} appear${report.reliableRumors === 1 ? "s" : ""} to be reliable.`,
    );
  }

  if (report.distortedRumors > 0) {
    lines.push(
      `CAUTION: ${report.distortedRumors} unconfirmed rumor${report.distortedRumors === 1 ? "" : "s"} may have been distorted through retelling.`,
    );
  }

  if (report.mostGossipyCat) {
    lines.push("");
    lines.push(
      `Today's most prolific gossip: ${report.mostGossipyCat}, who never misses a chance to share the latest whispers.`,
    );
  }

  if (report.juiciestRumor) {
    lines.push("");
    lines.push("--- JUICIEST RUMOR ---");
    lines.push(
      `"${report.juiciestRumor.message}" (retold ${report.juiciestRumor.retellings} times, reliability: ${report.juiciestRumor.reliability}%)`,
    );
  }

  return lines.join("\n");
}
