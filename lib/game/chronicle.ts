// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type ColonyEra =
  | "founding"
  | "growth"
  | "golden_age"
  | "decline"
  | "legacy";

export type SignificanceLevel = "landmark" | "notable" | "routine";

export interface EventSignificance {
  level: SignificanceLevel;
  reason: string;
}

export interface ChronicleEvent {
  type: string;
  timestamp: number;
  message: string;
}

export interface HistoricalFirst {
  eventType: string;
  timestamp: number;
  description: string;
}

export interface Anniversary {
  originalEvent: { type: string; timestamp: number; message: string };
  ageDescription: string;
  intervalMs: number;
}

export interface ChronicleColumn {
  sectionTitle: string;
  era: ColonyEra;
  entries: string[];
  totalEntries: number;
}

// ---------------------------------------------------------------------------
// Notable event types — these get "notable" instead of "routine" on repeat
// ---------------------------------------------------------------------------

const NOTABLE_EVENT_TYPES = new Set([
  "leader_change",
  "building_complete",
  "predator_attack",
  "encounter_won",
  "encounter_lost",
]);

// ---------------------------------------------------------------------------
// Era display names
// ---------------------------------------------------------------------------

const ERA_DISPLAY: Record<ColonyEra, string> = {
  founding: "Founding Era",
  growth: "Growth Era",
  golden_age: "Golden Age",
  decline: "Decline",
  legacy: "Legacy Era",
};

// ---------------------------------------------------------------------------
// classifyEra
// ---------------------------------------------------------------------------

export function classifyEra(
  colonyAgeHours: number,
  catCount: number,
  buildingCount: number,
  status: string,
): ColonyEra {
  // Decline overrides everything — critical or struggling colonies
  if (status === "critical" || status === "struggling") {
    return "decline";
  }

  if (colonyAgeHours > 48) {
    return "legacy";
  }

  if (colonyAgeHours >= 24) {
    return "golden_age";
  }

  if (colonyAgeHours >= 6) {
    return "growth";
  }

  return "founding";
}

// ---------------------------------------------------------------------------
// getEventSignificance
// ---------------------------------------------------------------------------

export function getEventSignificance(
  event: ChronicleEvent,
  allPriorEvents: ChronicleEvent[],
): EventSignificance {
  const hasTypeOccurredBefore = allPriorEvents.some(
    (e) => e.type === event.type,
  );

  if (!hasTypeOccurredBefore) {
    return {
      level: "landmark",
      reason: `The first ${event.type.replace(/_/g, " ")} in colony history`,
    };
  }

  if (NOTABLE_EVENT_TYPES.has(event.type)) {
    return {
      level: "notable",
      reason: `A significant ${event.type.replace(/_/g, " ")} event`,
    };
  }

  return {
    level: "routine",
    reason: `A regular ${event.type.replace(/_/g, " ")} occurrence`,
  };
}

// ---------------------------------------------------------------------------
// findHistoricalFirsts
// ---------------------------------------------------------------------------

export function findHistoricalFirsts(
  events: ChronicleEvent[],
): HistoricalFirst[] {
  const firstByType = new Map<string, ChronicleEvent>();

  for (const event of events) {
    const existing = firstByType.get(event.type);
    if (!existing || event.timestamp < existing.timestamp) {
      firstByType.set(event.type, event);
    }
  }

  const firsts: HistoricalFirst[] = [];
  for (const [eventType, event] of firstByType) {
    firsts.push({
      eventType,
      timestamp: event.timestamp,
      description: event.message,
    });
  }

  return firsts;
}

// ---------------------------------------------------------------------------
// getAnniversaries
// ---------------------------------------------------------------------------

const ANNIVERSARY_TOLERANCE_RATIO = 0.05; // 5% tolerance window

export function getAnniversaries(
  events: ChronicleEvent[],
  currentTime: number,
  intervalMs: number,
): Anniversary[] {
  const anniversaries: Anniversary[] = [];
  const tolerance = intervalMs * ANNIVERSARY_TOLERANCE_RATIO;

  for (const event of events) {
    const age = currentTime - event.timestamp;
    if (age < intervalMs - tolerance) continue;

    // Check if the age is a multiple of the interval (within tolerance)
    const intervals = Math.round(age / intervalMs);
    if (intervals < 1) continue;

    const expectedAge = intervals * intervalMs;
    const diff = Math.abs(age - expectedAge);

    if (diff <= tolerance) {
      anniversaries.push({
        originalEvent: {
          type: event.type,
          timestamp: event.timestamp,
          message: event.message,
        },
        ageDescription: formatAge(expectedAge),
        intervalMs: expectedAge,
      });
    }
  }

  return anniversaries;
}

function formatAge(ms: number): string {
  const hours = Math.floor(ms / 3_600_000);
  if (hours < 1) {
    const minutes = Math.floor(ms / 60_000);
    return `${minutes} minute${minutes !== 1 ? "s" : ""} ago`;
  }
  if (hours < 24) {
    return `${hours} hour${hours !== 1 ? "s" : ""} ago`;
  }
  const days = Math.floor(hours / 24);
  return `${days} day${days !== 1 ? "s" : ""} ago`;
}

// ---------------------------------------------------------------------------
// formatChronicleEntry
// ---------------------------------------------------------------------------

export function formatChronicleEntry(
  event: ChronicleEvent,
  significance: EventSignificance,
  era: ColonyEra,
): string {
  const eraLabel = ERA_DISPLAY[era];
  const parts: string[] = [];

  if (significance.level === "landmark") {
    parts.push("[LANDMARK]");
  } else if (significance.level === "notable") {
    parts.push("[NOTABLE]");
  }

  parts.push(event.message);
  parts.push(`(${eraLabel})`);

  return parts.join(" ");
}

// ---------------------------------------------------------------------------
// formatChronicleColumn
// ---------------------------------------------------------------------------

export function formatChronicleColumn(
  entries: string[],
  colonyName: string,
  era: ColonyEra,
): ChronicleColumn {
  const eraLabel = ERA_DISPLAY[era].toLowerCase();
  return {
    sectionTitle: `Looking Back — ${colonyName} Colony Chronicle (${eraLabel})`,
    era,
    entries,
    totalEntries: entries.length,
  };
}
