import { describe, it, expect } from "vitest";
import {
  calculateLegacyScore,
  getLegacyTier,
  composeLegacyTitle,
  rankLegacies,
  evaluateHallOfFame,
  generateHallOfFameColumn,
} from "@/lib/game/legacyScore";
import type {
  LegacyTier,
  LegacyEntry,
  HallOfFameReport,
} from "@/lib/game/legacyScore";

describe("calculateLegacyScore", () => {
  it("sums weighted contributions correctly", () => {
    // 10 days (10) + 2 offspring (20) + 5 tasks (10) + 3 encounters (15) + 2 leader days (6) + 1 building (8)
    expect(calculateLegacyScore(10, 2, 5, 3, 2, 1)).toBe(69);
  });

  it("returns 0 for all-zero inputs", () => {
    expect(calculateLegacyScore(0, 0, 0, 0, 0, 0)).toBe(0);
  });

  it("handles negative inputs by clamping to 0", () => {
    expect(calculateLegacyScore(-5, -1, -2, -3, -1, -1)).toBe(0);
  });

  it("gives expected score for mixed inputs", () => {
    // 50 days (50) + 5 offspring (50) + 20 tasks (40) + 10 encounters (50) + 10 leader days (30) + 3 buildings (24)
    expect(calculateLegacyScore(50, 5, 20, 10, 10, 3)).toBe(244);
  });
});

describe("getLegacyTier", () => {
  it('returns "forgotten" for score 0-19', () => {
    expect(getLegacyTier(0)).toBe("forgotten");
    expect(getLegacyTier(19)).toBe("forgotten");
  });

  it('returns "remembered" for score 20-49', () => {
    expect(getLegacyTier(20)).toBe("remembered");
    expect(getLegacyTier(49)).toBe("remembered");
  });

  it('returns "honored" for score 50-99', () => {
    expect(getLegacyTier(50)).toBe("honored");
    expect(getLegacyTier(99)).toBe("honored");
  });

  it('returns "legendary" for score 100+', () => {
    expect(getLegacyTier(100)).toBe("legendary");
    expect(getLegacyTier(500)).toBe("legendary");
  });

  it('handles negative score (returns "forgotten")', () => {
    expect(getLegacyTier(-10)).toBe("forgotten");
  });
});

describe("composeLegacyTitle", () => {
  it("includes cat name and tier", () => {
    const title = composeLegacyTitle("Whiskers", "legendary", "survival");
    expect(title).toContain("Whiskers");
    expect(title).toContain("Legendary");
  });

  it("includes top contribution type", () => {
    const title = composeLegacyTitle("Shadow", "honored", "tasks");
    expect(title).toContain("Shadow");
    expect(title).toContain("Worker");
  });

  it("handles each tier differently", () => {
    const forgotten = composeLegacyTitle("A", "forgotten", "survival");
    const remembered = composeLegacyTitle("A", "remembered", "survival");
    const honored = composeLegacyTitle("A", "honored", "survival");
    const legendary = composeLegacyTitle("A", "legendary", "survival");
    // Each tier should produce a different title format
    const titles = new Set([forgotten, remembered, honored, legendary]);
    expect(titles.size).toBe(4);
  });
});

describe("rankLegacies", () => {
  it("sorts by score descending", () => {
    const entries: LegacyEntry[] = [
      {
        catName: "A",
        score: 50,
        tier: "honored",
        topContribution: "survival",
        rank: 0,
      },
      {
        catName: "B",
        score: 150,
        tier: "legendary",
        topContribution: "tasks",
        rank: 0,
      },
      {
        catName: "C",
        score: 10,
        tier: "forgotten",
        topContribution: "offspring",
        rank: 0,
      },
    ];
    const ranked = rankLegacies(entries);
    expect(ranked[0].catName).toBe("B");
    expect(ranked[1].catName).toBe("A");
    expect(ranked[2].catName).toBe("C");
  });

  it("assigns rank starting from 1", () => {
    const entries: LegacyEntry[] = [
      {
        catName: "A",
        score: 100,
        tier: "legendary",
        topContribution: "survival",
        rank: 0,
      },
      {
        catName: "B",
        score: 50,
        tier: "honored",
        topContribution: "tasks",
        rank: 0,
      },
    ];
    const ranked = rankLegacies(entries);
    expect(ranked[0].rank).toBe(1);
    expect(ranked[1].rank).toBe(2);
  });

  it("handles empty input", () => {
    expect(rankLegacies([])).toEqual([]);
  });

  it("handles ties (same rank)", () => {
    const entries: LegacyEntry[] = [
      {
        catName: "A",
        score: 100,
        tier: "legendary",
        topContribution: "survival",
        rank: 0,
      },
      {
        catName: "B",
        score: 100,
        tier: "legendary",
        topContribution: "tasks",
        rank: 0,
      },
      {
        catName: "C",
        score: 50,
        tier: "honored",
        topContribution: "offspring",
        rank: 0,
      },
    ];
    const ranked = rankLegacies(entries);
    expect(ranked[0].rank).toBe(1);
    expect(ranked[1].rank).toBe(1);
    expect(ranked[2].rank).toBe(3);
  });
});

describe("evaluateHallOfFame", () => {
  const entries: LegacyEntry[] = [
    {
      catName: "Alpha",
      score: 150,
      tier: "legendary",
      topContribution: "tasks",
      rank: 1,
    },
    {
      catName: "Beta",
      score: 80,
      tier: "honored",
      topContribution: "survival",
      rank: 2,
    },
    {
      catName: "Gamma",
      score: 30,
      tier: "remembered",
      topContribution: "survival",
      rank: 3,
    },
    {
      catName: "Delta",
      score: 5,
      tier: "forgotten",
      topContribution: "offspring",
      rank: 4,
    },
  ];

  it("counts total entries", () => {
    const report = evaluateHallOfFame(entries);
    expect(report.totalEntries).toBe(4);
  });

  it("counts legendary and honored cats", () => {
    const report = evaluateHallOfFame(entries);
    expect(report.legendaryCount).toBe(1);
    expect(report.honoredCount).toBe(1);
  });

  it("finds top cat (highest score)", () => {
    const report = evaluateHallOfFame(entries);
    expect(report.topCat).toBe("Alpha");
  });

  it("calculates average score", () => {
    const report = evaluateHallOfFame(entries);
    // (150 + 80 + 30 + 5) / 4 = 66.25
    expect(report.averageScore).toBe(66.25);
  });

  it("finds dominant contribution type", () => {
    const report = evaluateHallOfFame(entries);
    // survival appears 2 times, tasks 1, offspring 1
    expect(report.dominantContribution).toBe("survival");
  });

  it("handles empty input", () => {
    const report = evaluateHallOfFame([]);
    expect(report.totalEntries).toBe(0);
    expect(report.legendaryCount).toBe(0);
    expect(report.honoredCount).toBe(0);
    expect(report.topCat).toBeNull();
    expect(report.averageScore).toBe(0);
    expect(report.dominantContribution).toBeNull();
  });
});

describe("generateHallOfFameColumn", () => {
  it("includes colony name", () => {
    const report: HallOfFameReport = {
      totalEntries: 1,
      legendaryCount: 1,
      honoredCount: 0,
      topCat: "Whiskers",
      averageScore: 120,
      dominantContribution: "survival",
    };
    const column = generateHallOfFameColumn(report, "Catford");
    expect(column).toContain("Catford");
  });

  it("handles 0 entries (no legends yet)", () => {
    const report: HallOfFameReport = {
      totalEntries: 0,
      legendaryCount: 0,
      honoredCount: 0,
      topCat: null,
      averageScore: 0,
      dominantContribution: null,
    };
    const column = generateHallOfFameColumn(report, "Catford");
    expect(column).toContain("Catford");
    expect(column.length).toBeGreaterThan(0);
  });

  it("handles multiple entries with ranking", () => {
    const report: HallOfFameReport = {
      totalEntries: 5,
      legendaryCount: 2,
      honoredCount: 1,
      topCat: "Alpha",
      averageScore: 85,
      dominantContribution: "tasks",
    };
    const column = generateHallOfFameColumn(report, "Catford");
    expect(column).toContain("Alpha");
    expect(column).toContain("5");
  });

  it("highlights legendary cats specially", () => {
    const report: HallOfFameReport = {
      totalEntries: 3,
      legendaryCount: 2,
      honoredCount: 1,
      topCat: "Hero",
      averageScore: 110,
      dominantContribution: "leadership",
    };
    const column = generateHallOfFameColumn(report, "Catford");
    expect(column.toLowerCase()).toContain("legendary");
  });
});
