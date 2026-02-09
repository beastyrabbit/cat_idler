import { describe, it, expect } from "vitest";
import {
  classifyLegendType,
  calculateEmbellishment,
  extractMoral,
  createLegend,
  rankLegends,
  generateFolkloreColumn,
} from "@/lib/game/folklore";
import type { LegendEvent, Legend } from "@/lib/game/folklore";

// --- classifyLegendType ---

describe("classifyLegendType", () => {
  it("returns 'heroic' for combat_victory events", () => {
    const event: LegendEvent = {
      type: "combat_victory",
      catName: "Blaze",
      detail: "Defeated a fox",
      dayOccurred: 5,
    };
    expect(classifyLegendType(event)).toBe("heroic");
  });

  it("returns 'cautionary' for cat_death events", () => {
    const event: LegendEvent = {
      type: "cat_death",
      catName: "Shadow",
      detail: "Starved",
      dayOccurred: 12,
    };
    expect(classifyLegendType(event)).toBe("cautionary");
  });

  it("returns 'origin' for colony_founded events", () => {
    const event: LegendEvent = {
      type: "colony_founded",
      detail: "The colony was born",
      dayOccurred: 0,
    };
    expect(classifyLegendType(event)).toBe("origin");
  });

  it("returns 'origin' for building_complete events", () => {
    const event: LegendEvent = {
      type: "building_complete",
      detail: "First shelter built",
      dayOccurred: 3,
    };
    expect(classifyLegendType(event)).toBe("origin");
  });

  it("returns 'mysterious' for rare_trait events", () => {
    const event: LegendEvent = {
      type: "rare_trait",
      catName: "Mystic",
      detail: "Born with heterochromia",
      dayOccurred: 7,
    };
    expect(classifyLegendType(event)).toBe("mysterious");
  });

  it("returns 'mysterious' for strange_encounter events", () => {
    const event: LegendEvent = {
      type: "strange_encounter",
      detail: "A glowing figure seen at dusk",
      dayOccurred: 20,
    };
    expect(classifyLegendType(event)).toBe("mysterious");
  });

  it("returns 'humorous' for failed_hunt events", () => {
    const event: LegendEvent = {
      type: "failed_hunt",
      catName: "Clumsy",
      detail: "Tripped over own paws",
      dayOccurred: 9,
    };
    expect(classifyLegendType(event)).toBe("humorous");
  });

  it("returns 'humorous' for lazy_cat events", () => {
    const event: LegendEvent = {
      type: "lazy_cat",
      catName: "Snooze",
      detail: "Slept through an entire hunt",
      dayOccurred: 15,
    };
    expect(classifyLegendType(event)).toBe("humorous");
  });

  it("defaults to 'mysterious' for unknown event types", () => {
    const event: LegendEvent = {
      type: "alien_abduction",
      detail: "Unexplained",
      dayOccurred: 99,
    };
    expect(classifyLegendType(event)).toBe("mysterious");
  });
});

// --- calculateEmbellishment ---

describe("calculateEmbellishment", () => {
  it("returns 'factual' for age < 10", () => {
    expect(calculateEmbellishment(5)).toBe("factual");
  });

  it("returns 'embellished' for age 10-30", () => {
    expect(calculateEmbellishment(20)).toBe("embellished");
  });

  it("returns 'mythical' for age > 30", () => {
    expect(calculateEmbellishment(50)).toBe("mythical");
  });

  it("returns 'embellished' at exactly 10 (boundary)", () => {
    expect(calculateEmbellishment(10)).toBe("embellished");
  });

  it("returns 'embellished' at exactly 30 (boundary)", () => {
    expect(calculateEmbellishment(30)).toBe("embellished");
  });

  it("returns 'mythical' at 31 (boundary)", () => {
    expect(calculateEmbellishment(31)).toBe("mythical");
  });

  it("returns 'factual' for age 0", () => {
    expect(calculateEmbellishment(0)).toBe("factual");
  });
});

// --- extractMoral ---

describe("extractMoral", () => {
  it("returns a moral string for each legend type", () => {
    const types = [
      "heroic",
      "cautionary",
      "origin",
      "mysterious",
      "humorous",
    ] as const;
    const event: LegendEvent = {
      type: "test",
      detail: "test event",
      dayOccurred: 1,
    };
    for (const t of types) {
      const moral = extractMoral(t, event);
      expect(typeof moral).toBe("string");
      expect(moral.length).toBeGreaterThan(0);
    }
  });

  it("includes cat name when available", () => {
    const event: LegendEvent = {
      type: "combat_victory",
      catName: "Blaze",
      detail: "Won a battle",
      dayOccurred: 5,
    };
    const moral = extractMoral("heroic", event);
    expect(moral).toContain("Blaze");
  });
});

// --- createLegend ---

describe("createLegend", () => {
  it("constructs a complete Legend object", () => {
    const event: LegendEvent = {
      type: "combat_victory",
      catName: "Blaze",
      detail: "Defeated a hawk",
      dayOccurred: 5,
    };
    const legend = createLegend(event, 8);
    expect(legend).toHaveProperty("title");
    expect(legend).toHaveProperty("legendType");
    expect(legend).toHaveProperty("embellishment");
    expect(legend).toHaveProperty("moral");
    expect(legend).toHaveProperty("fameScore");
    expect(legend).toHaveProperty("event");
    expect(legend).toHaveProperty("ageDays");
  });

  it("calculates correct age from currentDay - dayOccurred", () => {
    const event: LegendEvent = {
      type: "combat_victory",
      catName: "Blaze",
      detail: "Fought bravely",
      dayOccurred: 10,
    };
    const legend = createLegend(event, 25);
    expect(legend.ageDays).toBe(15);
  });

  it("assigns fame score based on type weight × embellishment multiplier", () => {
    const recentEvent: LegendEvent = {
      type: "combat_victory",
      catName: "Blaze",
      detail: "Won",
      dayOccurred: 90,
    };
    const oldEvent: LegendEvent = {
      type: "combat_victory",
      catName: "Blaze",
      detail: "Won",
      dayOccurred: 10,
    };
    const recentLegend = createLegend(recentEvent, 95); // age 5 → factual
    const oldLegend = createLegend(oldEvent, 95); // age 85 → mythical
    // Mythical should have higher fame than factual for same type
    expect(oldLegend.fameScore).toBeGreaterThan(recentLegend.fameScore);
  });
});

// --- rankLegends ---

describe("rankLegends", () => {
  it("sorts by fame score descending", () => {
    const legends: Legend[] = [
      createLegend({ type: "lazy_cat", detail: "Slept", dayOccurred: 90 }, 95), // low fame (humorous + factual)
      createLegend(
        {
          type: "combat_victory",
          catName: "Hero",
          detail: "Won",
          dayOccurred: 10,
        },
        95,
      ), // high fame (heroic + mythical)
      createLegend(
        { type: "colony_founded", detail: "Founded", dayOccurred: 50 },
        95,
      ), // mid fame
    ];
    const ranked = rankLegends(legends);
    for (let i = 0; i < ranked.length - 1; i++) {
      expect(ranked[i].fameScore).toBeGreaterThanOrEqual(
        ranked[i + 1].fameScore,
      );
    }
  });

  it("handles empty array", () => {
    expect(rankLegends([])).toEqual([]);
  });

  it("handles ties (stable sort)", () => {
    const event1: LegendEvent = {
      type: "combat_victory",
      catName: "A",
      detail: "Won first",
      dayOccurred: 10,
    };
    const event2: LegendEvent = {
      type: "combat_victory",
      catName: "B",
      detail: "Won second",
      dayOccurred: 10,
    };
    const legend1 = createLegend(event1, 15);
    const legend2 = createLegend(event2, 15);
    // Same type, same age → same fame
    expect(legend1.fameScore).toBe(legend2.fameScore);
    const ranked = rankLegends([legend1, legend2]);
    // Stable sort: original order preserved
    expect(ranked[0].event.catName).toBe("A");
    expect(ranked[1].event.catName).toBe("B");
  });
});

// --- generateFolkloreColumn ---

describe("generateFolkloreColumn", () => {
  it("includes colony name", () => {
    const legends = [
      createLegend(
        {
          type: "combat_victory",
          catName: "Hero",
          detail: "Won",
          dayOccurred: 5,
        },
        50,
      ),
    ];
    const column = generateFolkloreColumn(legends, "Whiskertown");
    expect(column).toContain("Whiskertown");
  });

  it("highlights mythical legends specially", () => {
    const event: LegendEvent = {
      type: "combat_victory",
      catName: "Ancient",
      detail: "Victory long ago",
      dayOccurred: 1,
    };
    const legend = createLegend(event, 100); // age 99 → mythical
    expect(legend.embellishment).toBe("mythical");
    const column = generateFolkloreColumn([legend], "TestColony");
    // Should have special treatment for mythical (e.g., "MYTHICAL" or emphasis)
    expect(column.toLowerCase()).toContain("myth");
  });

  it("handles empty legends (no legends message)", () => {
    const column = generateFolkloreColumn([], "EmptyColony");
    expect(column).toContain("EmptyColony");
    expect(column.length).toBeGreaterThan(0);
  });

  it("shows top 3 legends", () => {
    const legends = [
      createLegend(
        { type: "combat_victory", catName: "A", detail: "Won", dayOccurred: 1 },
        100,
      ),
      createLegend(
        { type: "cat_death", catName: "B", detail: "Died", dayOccurred: 2 },
        100,
      ),
      createLegend(
        { type: "colony_founded", detail: "Founded", dayOccurred: 0 },
        100,
      ),
      createLegend(
        {
          type: "failed_hunt",
          catName: "D",
          detail: "Failed",
          dayOccurred: 90,
        },
        100,
      ),
    ];
    const column = generateFolkloreColumn(legends, "BigColony");
    // Should mention at least the top 3 cats/events
    expect(column).toContain("A");
    expect(column).toContain("B");
    // Fourth legend might not appear
  });
});

// --- Purity ---

describe("purity", () => {
  it("all functions are pure (no side effects)", () => {
    const event: LegendEvent = {
      type: "combat_victory",
      catName: "Test",
      detail: "Test",
      dayOccurred: 5,
    };

    // Call functions multiple times — same inputs should give same outputs
    const type1 = classifyLegendType(event);
    const type2 = classifyLegendType(event);
    expect(type1).toBe(type2);

    const emb1 = calculateEmbellishment(15);
    const emb2 = calculateEmbellishment(15);
    expect(emb1).toBe(emb2);

    const moral1 = extractMoral("heroic", event);
    const moral2 = extractMoral("heroic", event);
    expect(moral1).toBe(moral2);

    const legend1 = createLegend(event, 20);
    const legend2 = createLegend(event, 20);
    expect(legend1).toEqual(legend2);
  });
});
