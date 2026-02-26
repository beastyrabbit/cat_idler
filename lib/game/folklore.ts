/**
 * Cat Folklore & Legends System
 *
 * Pure functions for generating colony folklore stories from notable events.
 * Legends grow more embellished over time, developing morals and fame scores.
 */

// --- Types ---

export type LegendType =
  | "heroic"
  | "cautionary"
  | "origin"
  | "mysterious"
  | "humorous";

export type EmbellishmentLevel = "factual" | "embellished" | "mythical";

export interface LegendEvent {
  type: string;
  catName?: string;
  detail: string;
  dayOccurred: number;
}

export interface Legend {
  title: string;
  legendType: LegendType;
  embellishment: EmbellishmentLevel;
  moral: string;
  fameScore: number;
  event: LegendEvent;
  ageDays: number;
}

// --- Constants ---

const HEROIC_EVENTS = new Set(["combat_victory"]);
const CAUTIONARY_EVENTS = new Set(["cat_death"]);
const ORIGIN_EVENTS = new Set(["colony_founded", "building_complete"]);
const MYSTERIOUS_EVENTS = new Set(["rare_trait", "strange_encounter"]);
const HUMOROUS_EVENTS = new Set(["failed_hunt", "lazy_cat"]);

const TYPE_WEIGHTS: Record<LegendType, number> = {
  heroic: 5,
  cautionary: 4,
  origin: 3,
  mysterious: 3,
  humorous: 2,
};

const EMBELLISHMENT_MULTIPLIERS: Record<EmbellishmentLevel, number> = {
  factual: 1,
  embellished: 2,
  mythical: 4,
};

const EMBELLISHMENT_THRESHOLD_LOW = 10;
const EMBELLISHMENT_THRESHOLD_HIGH = 30;

// --- Functions ---

export function classifyLegendType(event: LegendEvent): LegendType {
  if (HEROIC_EVENTS.has(event.type)) return "heroic";
  if (CAUTIONARY_EVENTS.has(event.type)) return "cautionary";
  if (ORIGIN_EVENTS.has(event.type)) return "origin";
  if (HUMOROUS_EVENTS.has(event.type)) return "humorous";
  if (MYSTERIOUS_EVENTS.has(event.type)) return "mysterious";
  return "mysterious";
}

export function calculateEmbellishment(
  legendAgeDays: number,
): EmbellishmentLevel {
  if (legendAgeDays < EMBELLISHMENT_THRESHOLD_LOW) return "factual";
  if (legendAgeDays <= EMBELLISHMENT_THRESHOLD_HIGH) return "embellished";
  return "mythical";
}

export function extractMoral(
  legendType: LegendType,
  event: LegendEvent,
): string {
  const name = event.catName ?? "a brave cat";
  switch (legendType) {
    case "heroic":
      return `Courage like ${name}'s lights the darkest nights.`;
    case "cautionary":
      return `Let the fate of ${name} remind us: neglect invites ruin.`;
    case "origin":
      return `Every great colony begins with a single bold step.`;
    case "mysterious":
      return `Some things in ${name}'s world defy explanation.`;
    case "humorous":
      return `Even ${name}'s misadventures bring joy to the colony.`;
  }
}

function generateTitle(legendType: LegendType, event: LegendEvent): string {
  const name = event.catName ?? "The Colony";
  switch (legendType) {
    case "heroic":
      return `The Valor of ${name}`;
    case "cautionary":
      return `The Warning of ${name}`;
    case "origin":
      return `The Founding Tale`;
    case "mysterious":
      return `The Mystery of ${name}`;
    case "humorous":
      return `The Misadventure of ${name}`;
  }
}

export function createLegend(event: LegendEvent, currentDay: number): Legend {
  const ageDays = Math.max(0, currentDay - event.dayOccurred);
  const legendType = classifyLegendType(event);
  const embellishment = calculateEmbellishment(ageDays);
  const moral = extractMoral(legendType, event);
  const title = generateTitle(legendType, event);
  const fameScore =
    TYPE_WEIGHTS[legendType] * EMBELLISHMENT_MULTIPLIERS[embellishment];

  return {
    title,
    legendType,
    embellishment,
    moral,
    fameScore,
    event,
    ageDays,
  };
}

export function rankLegends(legends: Legend[]): Legend[] {
  return [...legends].sort((a, b) => b.fameScore - a.fameScore);
}

export function generateFolkloreColumn(
  legends: Legend[],
  colonyName: string,
): string {
  const lines: string[] = [];
  lines.push(`=== FOLKLORE & LEGENDS of ${colonyName} ===`);
  lines.push("");

  if (legends.length === 0) {
    lines.push(
      `The colony of ${colonyName} is young — no legends have formed yet.`,
    );
    lines.push("In time, great tales will be told.");
    return lines.join("\n");
  }

  const ranked = rankLegends(legends);
  const top = ranked.slice(0, 3);

  for (const legend of top) {
    const prefix = legend.embellishment === "mythical" ? "[MYTHICAL] " : "";
    const name = legend.event.catName ?? "Unknown";
    lines.push(`${prefix}${legend.title} (fame: ${legend.fameScore})`);
    lines.push(`  "${legend.event.detail}" — ${name}`);
    lines.push(`  Moral: ${legend.moral}`);
    lines.push(
      `  Status: ${legend.embellishment} (${legend.ageDays} days old)`,
    );
    lines.push("");
  }

  if (ranked.length > 3) {
    lines.push(
      `...and ${ranked.length - 3} more tale(s) whispered around the colony.`,
    );
  }

  return lines.join("\n");
}
