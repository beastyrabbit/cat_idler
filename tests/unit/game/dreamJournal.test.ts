import { describe, it, expect } from "vitest";
import {
  generateDream,
  classifyDreamType,
  calculateVividness,
  interpretDream,
  evaluateDreamJournal,
  generateDreamColumn,
} from "@/lib/game/dreamJournal";
import type { Dream } from "@/lib/game/dreamJournal";

describe("generateDream", () => {
  it("creates dream with correct event source and cat name", () => {
    const dream = generateDream("hunt", "Whiskers", 75, 12345);
    expect(dream.catName).toBe("Whiskers");
    expect(dream.eventSource).toBe("hunt");
  });

  it("assigns vividness based on rest level", () => {
    const fragmented = generateDream("hunt", "Luna", 15, 42);
    expect(fragmented.vividness).toBe("fragmented");

    const hazy = generateDream("hunt", "Luna", 45, 42);
    expect(hazy.vividness).toBe("hazy");

    const clear = generateDream("hunt", "Luna", 70, 42);
    expect(clear.vividness).toBe("clear");

    const vivid = generateDream("hunt", "Luna", 90, 42);
    expect(vivid.vividness).toBe("vivid");
  });

  it("produces different messages per dream type", () => {
    const adventure = generateDream("hunt", "Luna", 50, 100);
    const nightmare = generateDream("predator", "Luna", 50, 100);
    expect(adventure.message).not.toBe(nightmare.message);
  });

  it("is deterministic with same seed", () => {
    const d1 = generateDream("hunt", "Luna", 50, 99999);
    const d2 = generateDream("hunt", "Luna", 50, 99999);
    expect(d1).toEqual(d2);
  });

  it("stores rest level on the dream", () => {
    const dream = generateDream("hunt", "Whiskers", 62, 12345);
    expect(dream.restLevel).toBe(62);
  });
});

describe("classifyDreamType", () => {
  it("maps predator events to nightmare", () => {
    expect(classifyDreamType("predator", 50, 42)).toBe("nightmare");
  });

  it("maps injury events to nightmare", () => {
    expect(classifyDreamType("injury", 50, 42)).toBe("nightmare");
  });

  it("maps hunt events to adventure", () => {
    expect(classifyDreamType("hunt", 50, 42)).toBe("adventure");
  });

  it("maps explore events to adventure", () => {
    expect(classifyDreamType("explore", 50, 42)).toBe("adventure");
  });

  it("allows prophetic dreams only at high rest (>80)", () => {
    // At low rest, should never be prophetic
    const results = new Set<string>();
    for (let seed = 1; seed <= 100; seed++) {
      results.add(classifyDreamType("mystery", 40, seed));
    }
    expect(results.has("prophetic")).toBe(false);

    // At high rest, prophetic is possible
    const highRestResults = new Set<string>();
    for (let seed = 1; seed <= 200; seed++) {
      highRestResults.add(classifyDreamType("mystery", 95, seed));
    }
    expect(highRestResults.has("prophetic")).toBe(true);
  });

  it("maps birth events to nostalgic", () => {
    expect(classifyDreamType("birth", 50, 42)).toBe("nostalgic");
  });

  it("maps breeding events to nostalgic", () => {
    expect(classifyDreamType("breeding", 50, 42)).toBe("nostalgic");
  });

  it("produces absurd dreams with seeded randomness", () => {
    const results = new Set<string>();
    for (let seed = 1; seed <= 200; seed++) {
      results.add(classifyDreamType("other", 50, seed));
    }
    expect(results.has("absurd")).toBe(true);
  });
});

describe("calculateVividness", () => {
  it('returns "fragmented" for rest 0-30', () => {
    expect(calculateVividness(0)).toBe("fragmented");
    expect(calculateVividness(15)).toBe("fragmented");
    expect(calculateVividness(30)).toBe("fragmented");
  });

  it('returns "hazy" for rest 31-60', () => {
    expect(calculateVividness(31)).toBe("hazy");
    expect(calculateVividness(45)).toBe("hazy");
    expect(calculateVividness(60)).toBe("hazy");
  });

  it('returns "clear" for rest 61-80', () => {
    expect(calculateVividness(61)).toBe("clear");
    expect(calculateVividness(70)).toBe("clear");
    expect(calculateVividness(80)).toBe("clear");
  });

  it('returns "vivid" for rest 81-100', () => {
    expect(calculateVividness(81)).toBe("vivid");
    expect(calculateVividness(90)).toBe("vivid");
    expect(calculateVividness(100)).toBe("vivid");
  });

  it("clamps out-of-range values", () => {
    expect(calculateVividness(-10)).toBe("fragmented");
    expect(calculateVividness(150)).toBe("vivid");
  });
});

describe("interpretDream", () => {
  const baseDream: Dream = {
    catName: "Luna",
    dreamType: "adventure",
    vividness: "clear",
    eventSource: "hunt",
    message: "Luna chased a giant mouse through the meadow",
    restLevel: 70,
  };

  it("returns a mystical interpretation string", () => {
    const interp = interpretDream(baseDream, 42);
    expect(typeof interp).toBe("string");
    expect(interp.length).toBeGreaterThan(0);
  });

  it("varies interpretation by dream type", () => {
    const adventureInterp = interpretDream(
      { ...baseDream, dreamType: "adventure" },
      42,
    );
    const nightmareInterp = interpretDream(
      { ...baseDream, dreamType: "nightmare" },
      42,
    );
    expect(adventureInterp).not.toBe(nightmareInterp);
  });

  it("is deterministic with same seed", () => {
    const i1 = interpretDream(baseDream, 99999);
    const i2 = interpretDream(baseDream, 99999);
    expect(i1).toBe(i2);
  });
});

describe("evaluateDreamJournal", () => {
  const dreams: Dream[] = [
    {
      catName: "Luna",
      dreamType: "adventure",
      vividness: "vivid",
      eventSource: "hunt",
      message: "Luna hunted a great beast",
      restLevel: 90,
    },
    {
      catName: "Luna",
      dreamType: "nightmare",
      vividness: "hazy",
      eventSource: "predator",
      message: "Luna fled from a shadow",
      restLevel: 40,
    },
    {
      catName: "Whiskers",
      dreamType: "prophetic",
      vividness: "vivid",
      eventSource: "mystery",
      message: "Whiskers saw the future",
      restLevel: 95,
    },
    {
      catName: "Shadow",
      dreamType: "adventure",
      vividness: "clear",
      eventSource: "explore",
      message: "Shadow explored distant lands",
      restLevel: 70,
    },
  ];

  it("counts total dreams", () => {
    const report = evaluateDreamJournal(dreams);
    expect(report.totalDreams).toBe(4);
  });

  it("counts vivid and nightmare dreams", () => {
    const report = evaluateDreamJournal(dreams);
    expect(report.vividDreams).toBe(2);
    expect(report.nightmareCount).toBe(1);
  });

  it("counts prophetic dreams", () => {
    const report = evaluateDreamJournal(dreams);
    expect(report.propheticCount).toBe(1);
  });

  it("finds most active dreamer (most dreams)", () => {
    const report = evaluateDreamJournal(dreams);
    expect(report.mostActiveDreamer).toBe("Luna");
  });

  it("finds dominant dream type", () => {
    const report = evaluateDreamJournal(dreams);
    expect(report.dominantDreamType).toBe("adventure");
  });

  it("handles empty input", () => {
    const report = evaluateDreamJournal([]);
    expect(report.totalDreams).toBe(0);
    expect(report.vividDreams).toBe(0);
    expect(report.nightmareCount).toBe(0);
    expect(report.propheticCount).toBe(0);
    expect(report.mostActiveDreamer).toBeNull();
    expect(report.dominantDreamType).toBeNull();
  });
});

describe("generateDreamColumn", () => {
  it("includes colony name", () => {
    const report = evaluateDreamJournal([]);
    const column = generateDreamColumn(report, "Catford");
    expect(column).toContain("Catford");
  });

  it("handles 0 dreams (peaceful night)", () => {
    const report = evaluateDreamJournal([]);
    const column = generateDreamColumn(report, "Catford");
    expect(column.toLowerCase()).toContain("peaceful");
  });

  it("handles multiple dreams with summary", () => {
    const dreams: Dream[] = [
      {
        catName: "Luna",
        dreamType: "adventure",
        vividness: "vivid",
        eventSource: "hunt",
        message: "Luna dreamed of a great hunt",
        restLevel: 90,
      },
      {
        catName: "Shadow",
        dreamType: "nightmare",
        vividness: "hazy",
        eventSource: "predator",
        message: "Shadow had a terrible night",
        restLevel: 40,
      },
    ];
    const report = evaluateDreamJournal(dreams);
    const column = generateDreamColumn(report, "Catford");
    expect(column).toContain("2");
    expect(column).toContain("Catford");
  });

  it("highlights prophetic dreams specially", () => {
    const dreams: Dream[] = [
      {
        catName: "Oracle",
        dreamType: "prophetic",
        vividness: "vivid",
        eventSource: "mystery",
        message: "Oracle saw visions of tomorrow",
        restLevel: 95,
      },
    ];
    const report = evaluateDreamJournal(dreams);
    const column = generateDreamColumn(report, "Catford");
    expect(column.toLowerCase()).toContain("prophetic");
  });
});
