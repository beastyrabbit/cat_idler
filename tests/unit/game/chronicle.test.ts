import { describe, it, expect } from "vitest";
import {
  classifyEra,
  getEventSignificance,
  findHistoricalFirsts,
  getAnniversaries,
  formatChronicleEntry,
  formatChronicleColumn,
} from "@/lib/game/chronicle";
import type {
  ColonyEra,
  SignificanceLevel,
  EventSignificance,
  HistoricalFirst,
  Anniversary,
  ChronicleColumn,
  ChronicleEvent,
} from "@/lib/game/chronicle";

// ---------------------------------------------------------------------------
// classifyEra
// ---------------------------------------------------------------------------
describe("classifyEra", () => {
  it("returns 'founding' for colony age < 6 hours", () => {
    const era = classifyEra(3, 5, 1, "thriving");
    expect(era).toBe("founding");
  });

  it("returns 'growth' for colony age 6-24h with expanding population", () => {
    const era = classifyEra(12, 8, 3, "thriving");
    expect(era).toBe("growth");
  });

  it("returns 'golden_age' for colony age 24-48h with thriving status", () => {
    const era = classifyEra(30, 15, 6, "thriving");
    expect(era).toBe("golden_age");
  });

  it("returns 'decline' when colony status is 'critical'", () => {
    const era = classifyEra(30, 15, 6, "critical");
    expect(era).toBe("decline");
  });

  it("returns 'decline' when colony status is 'struggling'", () => {
    const era = classifyEra(12, 5, 2, "struggling");
    expect(era).toBe("decline");
  });

  it("returns 'legacy' for colony age > 48 hours", () => {
    const era = classifyEra(72, 20, 10, "thriving");
    expect(era).toBe("legacy");
  });

  it("returns 'founding' for 0 hours (brand new colony)", () => {
    const era = classifyEra(0, 1, 0, "thriving");
    expect(era).toBe("founding");
  });

  it("decline overrides age-based era classification", () => {
    // Even at legacy age, critical status means decline
    const era = classifyEra(72, 20, 10, "critical");
    expect(era).toBe("decline");
  });
});

// ---------------------------------------------------------------------------
// getEventSignificance
// ---------------------------------------------------------------------------
describe("getEventSignificance", () => {
  it("rates first-of-type event as 'landmark'", () => {
    const event: ChronicleEvent = {
      type: "birth",
      timestamp: 1000,
      message: "A kitten was born",
    };
    const priorEvents: ChronicleEvent[] = [];
    const sig = getEventSignificance(event, priorEvents);
    expect(sig.level).toBe("landmark");
    expect(sig.reason).toContain("first");
  });

  it("rates second event of same type as 'routine'", () => {
    const event: ChronicleEvent = {
      type: "birth",
      timestamp: 2000,
      message: "Another kitten",
    };
    const priorEvents: ChronicleEvent[] = [
      { type: "birth", timestamp: 1000, message: "First kitten" },
    ];
    const sig = getEventSignificance(event, priorEvents);
    expect(sig.level).toBe("routine");
  });

  it("rates event of a new type as 'landmark' even with other events existing", () => {
    const event: ChronicleEvent = {
      type: "death",
      timestamp: 3000,
      message: "A cat passed",
    };
    const priorEvents: ChronicleEvent[] = [
      { type: "birth", timestamp: 1000, message: "First kitten" },
      { type: "birth", timestamp: 2000, message: "Second kitten" },
    ];
    const sig = getEventSignificance(event, priorEvents);
    expect(sig.level).toBe("landmark");
  });

  it("rates notable event types with 'notable' significance", () => {
    const event: ChronicleEvent = {
      type: "leader_change",
      timestamp: 5000,
      message: "New leader",
    };
    const priorEvents: ChronicleEvent[] = [
      { type: "leader_change", timestamp: 2000, message: "Previous leader" },
    ];
    const sig = getEventSignificance(event, priorEvents);
    expect(sig.level).toBe("notable");
  });

  it("returns a reason string for all significance levels", () => {
    const event: ChronicleEvent = {
      type: "birth",
      timestamp: 1000,
      message: "Kitten",
    };
    const sig = getEventSignificance(event, []);
    expect(typeof sig.reason).toBe("string");
    expect(sig.reason.length).toBeGreaterThan(0);
  });

  it("handles event types with underscores correctly", () => {
    const event: ChronicleEvent = {
      type: "predator_attack",
      timestamp: 1000,
      message: "Attack!",
    };
    const sig = getEventSignificance(event, []);
    expect(sig.level).toBe("landmark");
  });

  it("treats building_complete as notable when not first", () => {
    const event: ChronicleEvent = {
      type: "building_complete",
      timestamp: 3000,
      message: "House built",
    };
    const priorEvents: ChronicleEvent[] = [
      { type: "building_complete", timestamp: 1000, message: "First building" },
    ];
    const sig = getEventSignificance(event, priorEvents);
    expect(sig.level).toBe("notable");
  });
});

// ---------------------------------------------------------------------------
// findHistoricalFirsts
// ---------------------------------------------------------------------------
describe("findHistoricalFirsts", () => {
  it("returns empty array for empty event list", () => {
    const firsts = findHistoricalFirsts([]);
    expect(firsts).toEqual([]);
  });

  it("finds the first event of each type", () => {
    const events: ChronicleEvent[] = [
      { type: "birth", timestamp: 1000, message: "First kitten" },
      { type: "birth", timestamp: 2000, message: "Second kitten" },
      { type: "death", timestamp: 3000, message: "First death" },
      { type: "building_complete", timestamp: 4000, message: "First house" },
    ];
    const firsts = findHistoricalFirsts(events);
    expect(firsts).toHaveLength(3);
    expect(firsts.map((f) => f.eventType).sort()).toEqual([
      "birth",
      "building_complete",
      "death",
    ]);
  });

  it("picks the earliest event per type by timestamp", () => {
    const events: ChronicleEvent[] = [
      { type: "birth", timestamp: 5000, message: "Later kitten" },
      { type: "birth", timestamp: 1000, message: "First kitten" },
      { type: "birth", timestamp: 3000, message: "Middle kitten" },
    ];
    const firsts = findHistoricalFirsts(events);
    expect(firsts).toHaveLength(1);
    expect(firsts[0].timestamp).toBe(1000);
    expect(firsts[0].description).toBe("First kitten");
  });

  it("returns correct HistoricalFirst structure", () => {
    const events: ChronicleEvent[] = [
      { type: "death", timestamp: 5000, message: "A brave cat fell" },
    ];
    const firsts = findHistoricalFirsts(events);
    expect(firsts[0]).toEqual({
      eventType: "death",
      timestamp: 5000,
      description: "A brave cat fell",
    });
  });

  it("handles single event", () => {
    const events: ChronicleEvent[] = [
      { type: "birth", timestamp: 100, message: "Colony founded" },
    ];
    const firsts = findHistoricalFirsts(events);
    expect(firsts).toHaveLength(1);
    expect(firsts[0].eventType).toBe("birth");
  });
});

// ---------------------------------------------------------------------------
// getAnniversaries
// ---------------------------------------------------------------------------
describe("getAnniversaries", () => {
  const HOUR = 3_600_000;

  it("returns empty array when no events match any interval", () => {
    const events: ChronicleEvent[] = [
      { type: "birth", timestamp: 1000, message: "Kitten" },
    ];
    const currentTime = 2000; // only 1 second later — no anniversary
    const anniversaries = getAnniversaries(events, currentTime, HOUR);
    expect(anniversaries).toEqual([]);
  });

  it("finds 1-hour anniversary", () => {
    const events: ChronicleEvent[] = [
      { type: "birth", timestamp: 0, message: "Colony started" },
    ];
    const currentTime = HOUR; // exactly 1 hour
    const anniversaries = getAnniversaries(events, currentTime, HOUR);
    expect(anniversaries).toHaveLength(1);
    expect(anniversaries[0].ageDescription).toContain("1");
    expect(anniversaries[0].originalEvent.type).toBe("birth");
  });

  it("finds multi-interval anniversaries (e.g., 6-hour)", () => {
    const events: ChronicleEvent[] = [
      { type: "death", timestamp: 0, message: "First loss" },
    ];
    const currentTime = 6 * HOUR;
    const anniversaries = getAnniversaries(events, currentTime, HOUR);
    expect(anniversaries.length).toBeGreaterThan(0);
    expect(anniversaries[0].originalEvent.type).toBe("death");
  });

  it("allows tolerance window for approximate matches", () => {
    const events: ChronicleEvent[] = [
      { type: "birth", timestamp: 0, message: "Colony started" },
    ];
    // Slightly past 1 hour — should still match within tolerance
    const currentTime = HOUR + 60_000; // 1h + 1min
    const anniversaries = getAnniversaries(events, currentTime, HOUR);
    expect(anniversaries).toHaveLength(1);
  });

  it("returns correct Anniversary structure", () => {
    const events: ChronicleEvent[] = [
      { type: "building_complete", timestamp: 0, message: "First house" },
    ];
    const currentTime = HOUR;
    const anniversaries = getAnniversaries(events, currentTime, HOUR);
    expect(anniversaries[0]).toMatchObject({
      originalEvent: {
        type: "building_complete",
        timestamp: 0,
        message: "First house",
      },
      intervalMs: HOUR,
    });
    expect(typeof anniversaries[0].ageDescription).toBe("string");
  });

  it("handles multiple events with different anniversary times", () => {
    const events: ChronicleEvent[] = [
      { type: "birth", timestamp: 0, message: "First birth" },
      { type: "death", timestamp: HOUR, message: "First death" },
    ];
    const currentTime = 2 * HOUR;
    const anniversaries = getAnniversaries(events, currentTime, HOUR);
    // birth is 2h old, death is 1h old — both should have anniversaries
    expect(anniversaries.length).toBeGreaterThanOrEqual(2);
  });
});

// ---------------------------------------------------------------------------
// formatChronicleEntry
// ---------------------------------------------------------------------------
describe("formatChronicleEntry", () => {
  it("includes significance marker for landmark events", () => {
    const entry = formatChronicleEntry(
      { type: "birth", timestamp: 1000, message: "First kitten born" },
      { level: "landmark", reason: "First birth in colony history" },
      "founding",
    );
    expect(entry).toContain("LANDMARK");
    expect(entry).toContain("First kitten born");
  });

  it("includes era context in the entry", () => {
    const entry = formatChronicleEntry(
      { type: "death", timestamp: 5000, message: "A cat passed away" },
      { level: "routine", reason: "Regular event" },
      "golden_age",
    );
    expect(entry.toLowerCase()).toContain("golden age");
  });

  it("formats notable events differently from routine", () => {
    const notable = formatChronicleEntry(
      { type: "leader_change", timestamp: 3000, message: "New leader elected" },
      { level: "notable", reason: "Leadership change" },
      "growth",
    );
    const routine = formatChronicleEntry(
      { type: "birth", timestamp: 3000, message: "Another kitten" },
      { level: "routine", reason: "Regular birth" },
      "growth",
    );
    expect(notable).toContain("NOTABLE");
    expect(routine).not.toContain("LANDMARK");
    expect(routine).not.toContain("NOTABLE");
  });

  it("returns a non-empty string", () => {
    const entry = formatChronicleEntry(
      { type: "birth", timestamp: 0, message: "Kitten" },
      { level: "routine", reason: "Normal" },
      "founding",
    );
    expect(entry.length).toBeGreaterThan(0);
  });

  it("includes the event message", () => {
    const entry = formatChronicleEntry(
      {
        type: "predator_attack",
        timestamp: 9000,
        message: "Wolves at the gate!",
      },
      { level: "landmark", reason: "First predator attack" },
      "decline",
    );
    expect(entry).toContain("Wolves at the gate!");
  });
});

// ---------------------------------------------------------------------------
// formatChronicleColumn
// ---------------------------------------------------------------------------
describe("formatChronicleColumn", () => {
  it("includes colony name in section title", () => {
    const column = formatChronicleColumn(
      ["Entry 1", "Entry 2"],
      "Whiskerton",
      "golden_age",
    );
    expect(column.sectionTitle).toContain("Whiskerton");
  });

  it("returns correct ChronicleColumn structure", () => {
    const column = formatChronicleColumn(["Entry 1"], "Catford", "founding");
    expect(column).toMatchObject({
      era: "founding",
      entries: ["Entry 1"],
      totalEntries: 1,
    });
    expect(typeof column.sectionTitle).toBe("string");
  });

  it("handles empty entries array", () => {
    const column = formatChronicleColumn([], "EmptyTown", "legacy");
    expect(column.entries).toEqual([]);
    expect(column.totalEntries).toBe(0);
  });

  it("includes era in section title", () => {
    const column = formatChronicleColumn(["Entry"], "Catford", "decline");
    expect(column.sectionTitle.toLowerCase()).toContain("decline");
  });

  it("counts entries correctly", () => {
    const entries = ["A", "B", "C", "D"];
    const column = formatChronicleColumn(entries, "BigTown", "growth");
    expect(column.totalEntries).toBe(4);
    expect(column.entries).toHaveLength(4);
  });
});
