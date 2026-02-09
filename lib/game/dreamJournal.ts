/**
 * Cat Dream Journal
 *
 * Pure functions for generating cat dreams from recent colony events.
 * Dreams are categorized by type and vividness based on rest level.
 */

import { rollSeeded } from "@/lib/game/seededRng";

// ── Types ──────────────────────────────────────────────────────────────────

export type DreamType =
  | "adventure"
  | "nightmare"
  | "prophetic"
  | "nostalgic"
  | "absurd";

export type VividnessLevel = "fragmented" | "hazy" | "clear" | "vivid";

export interface Dream {
  catName: string;
  dreamType: DreamType;
  vividness: VividnessLevel;
  eventSource: string;
  message: string;
  restLevel: number;
}

export interface DreamReport {
  totalDreams: number;
  vividDreams: number;
  nightmareCount: number;
  propheticCount: number;
  mostActiveDreamer: string | null;
  dominantDreamType: DreamType | null;
}

// ── Constants ──────────────────────────────────────────────────────────────

const NIGHTMARE_EVENTS = new Set(["predator", "injury"]);
const ADVENTURE_EVENTS = new Set(["hunt", "explore"]);
const NOSTALGIC_EVENTS = new Set(["birth", "breeding"]);

const PROPHETIC_REST_THRESHOLD = 80;
const PROPHETIC_CHANCE = 0.3;
const ABSURD_CHANCE = 0.35;

const DREAM_MESSAGES: Record<DreamType, readonly string[]> = {
  adventure: [
    "{cat} chased a giant mouse across moonlit rooftops",
    "{cat} led a daring expedition through the shadow forest",
    "{cat} discovered a hidden valley full of catnip",
    "{cat} outran the wind itself in an endless meadow",
  ],
  nightmare: [
    "{cat} was pursued by shadows with gleaming eyes",
    "{cat} watched helplessly as the colony walls crumbled",
    "{cat} fell into an endless dark pit with no bottom",
    "{cat} heard terrible howling from beyond the fence",
  ],
  prophetic: [
    "{cat} saw shimmering visions of abundance ahead",
    "{cat} glimpsed a stranger arriving at the colony gates",
    "{cat} watched the seasons change in the blink of an eye",
    "{cat} heard whispers foretelling a great challenge",
  ],
  nostalgic: [
    "{cat} remembered warm milk and gentle paws from kittenhood",
    "{cat} relived playing with siblings under the old oak tree",
    "{cat} dreamt of a friend long gone, purring softly nearby",
    "{cat} wandered through memories of the colony's early days",
  ],
  absurd: [
    "{cat} rode a flying fish over a sea of yarn",
    "{cat} had a conversation with a philosophical mouse",
    "{cat} discovered that trees could walk and talk",
    "{cat} played chess with an upside-down squirrel",
  ],
};

const INTERPRETATIONS: Record<DreamType, readonly string[]> = {
  adventure: [
    "The spirits of the hunt stir — bold endeavors lie ahead",
    "Restless paws foretell a journey beyond familiar paths",
    "The dream-wind carries the scent of undiscovered prey",
  ],
  nightmare: [
    "Dark omens swirl — vigilance is the price of safety",
    "The shadow-dreams warn of unseen dangers lurking near",
    "Troubled sleep signals a time to strengthen defenses",
  ],
  prophetic: [
    "The veil between worlds thins — heed the vision's wisdom",
    "Ancient dream-spirits have chosen to share their sight",
    "A prophetic gift bestowed upon a worthy dreamer",
  ],
  nostalgic: [
    "The past reaches forward with gentle paws of comfort",
    "Old bonds echo through time, reminding us of roots",
    "Memory-dreams bring peace and renew the spirit",
  ],
  absurd: [
    "The dream-realm dances to its own mysterious tune",
    "Chaos-visions bring unexpected insights to the wise",
    "When dreams make no sense, the universe is playful",
  ],
};

// ── Functions ──────────────────────────────────────────────────────────────

export function calculateVividness(restLevel: number): VividnessLevel {
  const clamped = Math.max(0, Math.min(100, restLevel));
  if (clamped <= 30) return "fragmented";
  if (clamped <= 60) return "hazy";
  if (clamped <= 80) return "clear";
  return "vivid";
}

export function classifyDreamType(
  eventType: string,
  restLevel: number,
  seed: number,
): DreamType {
  if (NIGHTMARE_EVENTS.has(eventType)) return "nightmare";
  if (ADVENTURE_EVENTS.has(eventType)) return "adventure";
  if (NOSTALGIC_EVENTS.has(eventType)) return "nostalgic";

  const roll = rollSeeded(seed);

  if (restLevel > PROPHETIC_REST_THRESHOLD && roll.value < PROPHETIC_CHANCE) {
    return "prophetic";
  }

  const roll2 = rollSeeded(roll.nextSeed);
  if (roll2.value < ABSURD_CHANCE) return "absurd";

  return "nostalgic";
}

export function generateDream(
  eventType: string,
  catName: string,
  restLevel: number,
  seed: number,
): Dream {
  const dreamType = classifyDreamType(eventType, restLevel, seed);
  const vividness = calculateVividness(restLevel);

  const messages = DREAM_MESSAGES[dreamType];
  const roll = rollSeeded(seed);
  const msgIndex = Math.floor(roll.value * messages.length);
  const message = messages[msgIndex].replace("{cat}", catName);

  return {
    catName,
    dreamType,
    vividness,
    eventSource: eventType,
    message,
    restLevel,
  };
}

export function interpretDream(dream: Dream, seed: number): string {
  const interpretations = INTERPRETATIONS[dream.dreamType];
  const roll = rollSeeded(seed);
  const index = Math.floor(roll.value * interpretations.length);
  return interpretations[index];
}

export function evaluateDreamJournal(dreams: Dream[]): DreamReport {
  if (dreams.length === 0) {
    return {
      totalDreams: 0,
      vividDreams: 0,
      nightmareCount: 0,
      propheticCount: 0,
      mostActiveDreamer: null,
      dominantDreamType: null,
    };
  }

  const vividDreams = dreams.filter((d) => d.vividness === "vivid").length;
  const nightmareCount = dreams.filter(
    (d) => d.dreamType === "nightmare",
  ).length;
  const propheticCount = dreams.filter(
    (d) => d.dreamType === "prophetic",
  ).length;

  // Find most active dreamer
  const dreamerCounts = new Map<string, number>();
  for (const d of dreams) {
    dreamerCounts.set(d.catName, (dreamerCounts.get(d.catName) ?? 0) + 1);
  }
  let mostActiveDreamer: string | null = null;
  let maxDreams = 0;
  for (const [name, count] of dreamerCounts) {
    if (count > maxDreams) {
      maxDreams = count;
      mostActiveDreamer = name;
    }
  }

  // Find dominant dream type
  const typeCounts = new Map<DreamType, number>();
  for (const d of dreams) {
    typeCounts.set(d.dreamType, (typeCounts.get(d.dreamType) ?? 0) + 1);
  }
  let dominantDreamType: DreamType | null = null;
  let maxTypeCount = 0;
  for (const [type, count] of typeCounts) {
    if (count > maxTypeCount) {
      maxTypeCount = count;
      dominantDreamType = type;
    }
  }

  return {
    totalDreams: dreams.length,
    vividDreams,
    nightmareCount,
    propheticCount,
    mostActiveDreamer,
    dominantDreamType,
  };
}

export function generateDreamColumn(
  report: DreamReport,
  colonyName: string,
): string {
  const header = `DREAM DIARY — ${colonyName}`;

  if (report.totalDreams === 0) {
    return `${header}\n\nA peaceful night in ${colonyName} — no dreams were reported. The colony slumbers in tranquil silence.`;
  }

  const lines: string[] = [header, ""];

  lines.push(
    `Last night, ${report.totalDreams} dream${report.totalDreams === 1 ? " was" : "s were"} recorded across the colony of ${colonyName}.`,
  );

  if (report.vividDreams > 0) {
    lines.push(
      `${report.vividDreams} vivid dream${report.vividDreams === 1 ? "" : "s"} lit up the dreamscape.`,
    );
  }

  if (report.nightmareCount > 0) {
    lines.push(
      `${report.nightmareCount} nightmare${report.nightmareCount === 1 ? "" : "s"} troubled sleeping cats.`,
    );
  }

  if (report.propheticCount > 0) {
    lines.push(
      `${report.propheticCount} prophetic vision${report.propheticCount === 1 ? "" : "s"} whispered of things to come.`,
    );
  }

  if (report.mostActiveDreamer) {
    lines.push(`Most active dreamer: ${report.mostActiveDreamer}.`);
  }

  if (report.dominantDreamType) {
    lines.push(
      `The colony's dreams leaned toward the ${report.dominantDreamType}.`,
    );
  }

  return lines.join("\n");
}
