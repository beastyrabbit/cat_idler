/**
 * Tests for Cat Obituary System
 *
 * Generates newspaper-style obituaries for deceased cats.
 */

import { describe, it, expect } from "vitest";
import {
  getCauseOfDeathLabel,
  getLifeSummary,
  getNotableTrait,
  getMemorialQuote,
  formatObituaryColumn,
  getServiceYears,
} from "@/lib/game/obituaries";
import type { CatStats, CatSpecialization, LifeStage } from "@/types/game";

// Helper: build default stats
function makeStats(overrides: Partial<CatStats> = {}): CatStats {
  return {
    attack: 10,
    defense: 10,
    hunting: 10,
    medicine: 10,
    cleaning: 10,
    building: 10,
    leadership: 10,
    vision: 10,
    ...overrides,
  };
}

// ─── getCauseOfDeathLabel ───────────────────────────────────────────

describe("getCauseOfDeathLabel", () => {
  it("returns label for starvation", () => {
    expect(getCauseOfDeathLabel("starvation")).toBe("Starvation");
  });

  it("returns label for dehydration", () => {
    expect(getCauseOfDeathLabel("dehydration")).toBe("Dehydration");
  });

  it("returns label for old_age", () => {
    expect(getCauseOfDeathLabel("old_age")).toBe("Old Age");
  });

  it("returns label for combat", () => {
    expect(getCauseOfDeathLabel("combat")).toBe("Combat");
  });

  it("returns label for unknown", () => {
    expect(getCauseOfDeathLabel("unknown")).toBe("Unknown Causes");
  });
});

// ─── getServiceYears ────────────────────────────────────────────────

describe("getServiceYears", () => {
  it('returns "less than an hour" for 0 hours', () => {
    expect(getServiceYears(0)).toBe("less than an hour");
  });

  it('returns "1 hour" for exactly 1 hour', () => {
    expect(getServiceYears(1)).toBe("1 hour");
  });

  it("returns hours for less than 24 hours", () => {
    expect(getServiceYears(5)).toBe("5 hours");
  });

  it('returns "1 day" for exactly 24 hours', () => {
    expect(getServiceYears(24)).toBe("1 day");
  });

  it("returns days for 48+ hours", () => {
    expect(getServiceYears(72)).toBe("3 days");
  });

  it("returns days and hours for non-exact multiples", () => {
    expect(getServiceYears(30)).toBe("1 day, 6 hours");
  });

  it("handles fractional hours by flooring", () => {
    expect(getServiceYears(0.5)).toBe("less than an hour");
  });

  it("returns days with remainder hours", () => {
    expect(getServiceYears(49)).toBe("2 days, 1 hour");
  });
});

// ─── getNotableTrait ────────────────────────────────────────────────

describe("getNotableTrait", () => {
  it("identifies the highest stat", () => {
    const stats = makeStats({ hunting: 50 });
    const result = getNotableTrait(stats);
    expect(result.trait).toBe("hunting");
  });

  it("returns a descriptive label for the trait", () => {
    const stats = makeStats({ hunting: 50 });
    const result = getNotableTrait(stats);
    expect(result.label).toContain("hunting");
  });

  it("picks first stat in order when all are equal", () => {
    const stats = makeStats(); // all 10
    const result = getNotableTrait(stats);
    // Should pick 'attack' (first in CatStats key order)
    expect(result.trait).toBe("attack");
  });

  it("handles high leadership", () => {
    const stats = makeStats({ leadership: 90 });
    const result = getNotableTrait(stats);
    expect(result.trait).toBe("leadership");
    expect(result.label).toContain("leadership");
  });

  it("handles high defense", () => {
    const stats = makeStats({ defense: 80 });
    const result = getNotableTrait(stats);
    expect(result.trait).toBe("defense");
  });

  it("handles high medicine", () => {
    const stats = makeStats({ medicine: 75 });
    const result = getNotableTrait(stats);
    expect(result.trait).toBe("medicine");
  });

  it("handles high building", () => {
    const stats = makeStats({ building: 60 });
    const result = getNotableTrait(stats);
    expect(result.trait).toBe("building");
  });

  it("handles high vision", () => {
    const stats = makeStats({ vision: 99 });
    const result = getNotableTrait(stats);
    expect(result.trait).toBe("vision");
  });
});

// ─── getLifeSummary ─────────────────────────────────────────────────

describe("getLifeSummary", () => {
  it("mentions the cat name", () => {
    const summary = getLifeSummary({
      name: "Whiskers",
      ageHours: 30,
      lifeStage: "adult",
      specialization: null,
      stats: makeStats(),
      causeOfDeath: "old_age",
    });
    expect(summary).toContain("Whiskers");
  });

  it("mentions the life stage", () => {
    const summary = getLifeSummary({
      name: "Mittens",
      ageHours: 50,
      lifeStage: "elder",
      specialization: null,
      stats: makeStats(),
      causeOfDeath: "old_age",
    });
    expect(summary).toContain("elder");
  });

  it("mentions specialization when present", () => {
    const summary = getLifeSummary({
      name: "Shadow",
      ageHours: 30,
      lifeStage: "adult",
      specialization: "hunter",
      stats: makeStats({ hunting: 50 }),
      causeOfDeath: "combat",
    });
    expect(summary).toContain("hunter");
  });

  it("works without specialization", () => {
    const summary = getLifeSummary({
      name: "Patches",
      ageHours: 10,
      lifeStage: "young",
      specialization: null,
      stats: makeStats(),
      causeOfDeath: "starvation",
    });
    expect(summary).toContain("Patches");
    expect(summary.length).toBeGreaterThan(20);
  });

  it("handles kitten death", () => {
    const summary = getLifeSummary({
      name: "Tiny",
      ageHours: 2,
      lifeStage: "kitten",
      specialization: null,
      stats: makeStats(),
      causeOfDeath: "dehydration",
    });
    expect(summary).toContain("Tiny");
    expect(summary).toContain("kitten");
  });

  it("mentions cause of death", () => {
    const summary = getLifeSummary({
      name: "Brave",
      ageHours: 30,
      lifeStage: "adult",
      specialization: null,
      stats: makeStats(),
      causeOfDeath: "combat",
    });
    expect(summary.toLowerCase()).toContain("combat");
  });

  it("handles zero-age kitten", () => {
    const summary = getLifeSummary({
      name: "Newborn",
      ageHours: 0,
      lifeStage: "kitten",
      specialization: null,
      stats: makeStats(),
      causeOfDeath: "unknown",
    });
    expect(summary).toContain("Newborn");
  });
});

// ─── getMemorialQuote ───────────────────────────────────────────────

describe("getMemorialQuote", () => {
  it("returns a string for kitten + starvation", () => {
    const quote = getMemorialQuote("kitten", "starvation");
    expect(quote.length).toBeGreaterThan(5);
  });

  it("returns a string for elder + old_age", () => {
    const quote = getMemorialQuote("elder", "old_age");
    expect(quote.length).toBeGreaterThan(5);
  });

  it("returns a string for adult + combat", () => {
    const quote = getMemorialQuote("adult", "combat");
    expect(quote.length).toBeGreaterThan(5);
  });

  it("returns distinct quotes for different life stages", () => {
    const kittenQuote = getMemorialQuote("kitten", "unknown");
    const elderQuote = getMemorialQuote("elder", "unknown");
    expect(kittenQuote).not.toBe(elderQuote);
  });

  it("returns distinct quotes for different causes", () => {
    const combatQuote = getMemorialQuote("adult", "combat");
    const oldAgeQuote = getMemorialQuote("adult", "old_age");
    expect(combatQuote).not.toBe(oldAgeQuote);
  });

  it("handles young + dehydration", () => {
    const quote = getMemorialQuote("young", "dehydration");
    expect(quote.length).toBeGreaterThan(5);
  });

  it("handles all cause types", () => {
    const causes = [
      "starvation",
      "dehydration",
      "old_age",
      "combat",
      "unknown",
    ] as const;
    for (const cause of causes) {
      const quote = getMemorialQuote("adult", cause);
      expect(quote.length).toBeGreaterThan(5);
    }
  });
});

// ─── formatObituaryColumn ───────────────────────────────────────────

describe("formatObituaryColumn", () => {
  const baseCatData = {
    name: "Whiskers",
    ageHours: 50,
    lifeStage: "elder" as LifeStage,
    specialization: "hunter" as CatSpecialization,
    stats: makeStats({ hunting: 80 }),
    causeOfDeath: "old_age" as const,
  };

  it("returns a complete obituary column", () => {
    const column = formatObituaryColumn(baseCatData);
    expect(column).toHaveProperty("title");
    expect(column).toHaveProperty("lifeSummary");
    expect(column).toHaveProperty("notableTrait");
    expect(column).toHaveProperty("memorialQuote");
    expect(column).toHaveProperty("serviceYears");
  });

  it("title contains cat name", () => {
    const column = formatObituaryColumn(baseCatData);
    expect(column.title).toContain("Whiskers");
  });

  it("lifeSummary is a non-empty string", () => {
    const column = formatObituaryColumn(baseCatData);
    expect(column.lifeSummary.length).toBeGreaterThan(20);
  });

  it("serviceYears matches getServiceYears output", () => {
    const column = formatObituaryColumn(baseCatData);
    expect(column.serviceYears).toContain("2 days");
  });

  it("notableTrait mentions the top stat", () => {
    const column = formatObituaryColumn(baseCatData);
    expect(column.notableTrait).toContain("hunting");
  });

  it("memorialQuote is non-empty", () => {
    const column = formatObituaryColumn(baseCatData);
    expect(column.memorialQuote.length).toBeGreaterThan(5);
  });

  it("handles kitten with no specialization", () => {
    const column = formatObituaryColumn({
      name: "Tiny",
      ageHours: 1,
      lifeStage: "kitten",
      specialization: null,
      stats: makeStats(),
      causeOfDeath: "starvation",
    });
    expect(column.title).toContain("Tiny");
    expect(column.serviceYears).toBe("1 hour");
  });

  it("handles zero-age kitten", () => {
    const column = formatObituaryColumn({
      name: "Newborn",
      ageHours: 0,
      lifeStage: "kitten",
      specialization: null,
      stats: makeStats(),
      causeOfDeath: "unknown",
    });
    expect(column.title).toContain("Newborn");
    expect(column.serviceYears).toBe("less than an hour");
  });

  it("handles very old elder", () => {
    const column = formatObituaryColumn({
      name: "Ancient",
      ageHours: 200,
      lifeStage: "elder",
      specialization: "ritualist",
      stats: makeStats({ leadership: 95 }),
      causeOfDeath: "old_age",
    });
    expect(column.title).toContain("Ancient");
    expect(column.serviceYears).toContain("8 days");
  });

  it("handles combat death for adult", () => {
    const column = formatObituaryColumn({
      name: "Brave",
      ageHours: 30,
      lifeStage: "adult",
      specialization: null,
      stats: makeStats({ attack: 60, defense: 55 }),
      causeOfDeath: "combat",
    });
    expect(column.title).toContain("Brave");
  });
});
