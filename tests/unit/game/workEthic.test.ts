import { describe, it, expect } from "vitest";
import {
  calculateWorkEthic,
  getEfficiencyMultiplier,
  detectSlacker,
  detectOverworked,
  evaluateColonyProductivity,
  generateWorkplaceColumn,
} from "@/lib/game/workEthic";
import type {
  ProductivityTier,
  WorkerCat,
  ProductivityReport,
} from "@/lib/game/workEthic";

describe("calculateWorkEthic", () => {
  it("combines skills, rest bonus, satisfaction, and age modifier for a rested adult", () => {
    // skills avg = (80 + 60) / 2 = 70, rest bonus = +20 (rest > 70), no penalty, adult 1.0x
    // score = (70 + 20) * 1.0 = 90
    const score = calculateWorkEthic(
      { hunting: 80, combat: 60 },
      90,
      10,
      10,
      "adult",
    );
    expect(score).toBe(90);
  });

  it("clamps result to 0-100", () => {
    // Max skills (100+100)/2=100, rest bonus +20, no penalty, adult 1.0x → 120 → clamped to 100
    const high = calculateWorkEthic(
      { hunting: 100, combat: 100 },
      100,
      0,
      0,
      "adult",
    );
    expect(high).toBe(100);

    // 0 skills, no rest bonus, penalty -20, kitten 0.5x → (0 + 0 - 20) * 0.5 = -10 → clamped to 0
    const low = calculateWorkEthic(
      { hunting: 0, combat: 0 },
      10,
      80,
      10,
      "kitten",
    );
    expect(low).toBe(0);
  });

  it("returns higher score for rested, skilled cats", () => {
    const rested = calculateWorkEthic(
      { hunting: 80, combat: 80 },
      90,
      10,
      10,
      "adult",
    );
    const tired = calculateWorkEthic(
      { hunting: 80, combat: 80 },
      20,
      10,
      10,
      "adult",
    );
    expect(rested).toBeGreaterThan(tired);
  });

  it("returns lower score for hungry/thirsty cats", () => {
    const satisfied = calculateWorkEthic(
      { hunting: 60, combat: 60 },
      50,
      20,
      20,
      "adult",
    );
    const hungry = calculateWorkEthic(
      { hunting: 60, combat: 60 },
      50,
      80,
      20,
      "adult",
    );
    expect(hungry).toBeLessThan(satisfied);
  });

  it("applies correct age modifier per life stage", () => {
    const base = { hunting: 60, combat: 60 };
    const kitten = calculateWorkEthic(base, 50, 10, 10, "kitten");
    const young = calculateWorkEthic(base, 50, 10, 10, "young");
    const adult = calculateWorkEthic(base, 50, 10, 10, "adult");
    const elder = calculateWorkEthic(base, 50, 10, 10, "elder");

    // kitten < elder < young < adult (0.5x < 0.7x < 0.8x < 1.0x)
    expect(kitten).toBeLessThan(elder);
    expect(elder).toBeLessThan(young);
    expect(young).toBeLessThan(adult);
  });
});

describe("getEfficiencyMultiplier", () => {
  it("returns 0.5 for workEthic 0-20", () => {
    expect(getEfficiencyMultiplier(0)).toBe(0.5);
    expect(getEfficiencyMultiplier(10)).toBe(0.5);
    expect(getEfficiencyMultiplier(20)).toBe(0.5);
  });

  it("returns 0.75 for workEthic 21-40", () => {
    expect(getEfficiencyMultiplier(21)).toBe(0.75);
    expect(getEfficiencyMultiplier(30)).toBe(0.75);
    expect(getEfficiencyMultiplier(40)).toBe(0.75);
  });

  it("returns 1.0 for workEthic 41-60", () => {
    expect(getEfficiencyMultiplier(41)).toBe(1.0);
    expect(getEfficiencyMultiplier(50)).toBe(1.0);
    expect(getEfficiencyMultiplier(60)).toBe(1.0);
  });

  it("returns 1.25 for workEthic 61-80", () => {
    expect(getEfficiencyMultiplier(61)).toBe(1.25);
    expect(getEfficiencyMultiplier(70)).toBe(1.25);
    expect(getEfficiencyMultiplier(80)).toBe(1.25);
  });

  it("returns 1.5 for workEthic 81-100", () => {
    expect(getEfficiencyMultiplier(81)).toBe(1.5);
    expect(getEfficiencyMultiplier(90)).toBe(1.5);
    expect(getEfficiencyMultiplier(100)).toBe(1.5);
  });

  it("clamps out-of-range inputs", () => {
    expect(getEfficiencyMultiplier(-10)).toBe(0.5);
    expect(getEfficiencyMultiplier(150)).toBe(1.5);
  });
});

describe("detectSlacker", () => {
  it("returns true when work ethic < 30 and rest > 70", () => {
    expect(detectSlacker(20, 80)).toBe(true);
    expect(detectSlacker(10, 90)).toBe(true);
  });

  it("returns false when work ethic >= 30", () => {
    expect(detectSlacker(30, 80)).toBe(false);
    expect(detectSlacker(50, 90)).toBe(false);
  });

  it("returns false when rest <= 70 (tired, not slacking)", () => {
    expect(detectSlacker(20, 70)).toBe(false);
    expect(detectSlacker(10, 50)).toBe(false);
  });
});

describe("detectOverworked", () => {
  it("returns true when work ethic > 70 and rest < 30", () => {
    expect(detectOverworked(80, 20)).toBe(true);
    expect(detectOverworked(90, 10)).toBe(true);
  });

  it("returns false when rest >= 30", () => {
    expect(detectOverworked(80, 30)).toBe(false);
    expect(detectOverworked(90, 50)).toBe(false);
  });

  it("returns false when work ethic <= 70", () => {
    expect(detectOverworked(70, 20)).toBe(false);
    expect(detectOverworked(50, 10)).toBe(false);
  });
});

describe("evaluateColonyProductivity", () => {
  const makeCat = (
    name: string,
    workEthic: number,
    restLevel: number,
    efficiency: number,
  ): WorkerCat => ({
    name,
    workEthic,
    restLevel,
    efficiency,
  });

  it("counts total workers", () => {
    const cats = [
      makeCat("Whiskers", 50, 50, 1.0),
      makeCat("Mittens", 60, 60, 1.25),
      makeCat("Shadow", 30, 40, 0.75),
    ];
    const report = evaluateColonyProductivity(cats);
    expect(report.totalWorkers).toBe(3);
  });

  it("calculates average efficiency", () => {
    const cats = [
      makeCat("Whiskers", 50, 50, 1.0),
      makeCat("Mittens", 50, 50, 1.5),
    ];
    const report = evaluateColonyProductivity(cats);
    expect(report.averageEfficiency).toBe(1.25);
  });

  it("counts slackers and overworked", () => {
    const cats = [
      makeCat("Lazy", 20, 80, 0.5), // slacker: ethic < 30, rest > 70
      makeCat("Burnout", 80, 20, 1.25), // overworked: ethic > 70, rest < 30
      makeCat("Normal", 50, 50, 1.0),
    ];
    const report = evaluateColonyProductivity(cats);
    expect(report.slackerCount).toBe(1);
    expect(report.overworkedCount).toBe(1);
  });

  it("finds top worker (highest work ethic)", () => {
    const cats = [
      makeCat("Average", 50, 50, 1.0),
      makeCat("Star", 95, 60, 1.5),
      makeCat("Newbie", 30, 70, 0.75),
    ];
    const report = evaluateColonyProductivity(cats);
    expect(report.topWorker).toBe("Star");
  });

  it("finds dominant productivity tier", () => {
    const cats = [
      makeCat("A", 50, 50, 1.0), // average (41-60)
      makeCat("B", 55, 50, 1.0), // average (41-60)
      makeCat("C", 80, 50, 1.25), // diligent (61-80)
    ];
    const report = evaluateColonyProductivity(cats);
    expect(report.dominantTier).toBe("average");
  });

  it("handles empty input", () => {
    const report = evaluateColonyProductivity([]);
    expect(report.totalWorkers).toBe(0);
    expect(report.averageEfficiency).toBe(0);
    expect(report.slackerCount).toBe(0);
    expect(report.overworkedCount).toBe(0);
    expect(report.topWorker).toBeNull();
    expect(report.dominantTier).toBeNull();
  });
});

describe("generateWorkplaceColumn", () => {
  it("includes colony name", () => {
    const report: ProductivityReport = {
      totalWorkers: 5,
      averageEfficiency: 1.1,
      slackerCount: 1,
      overworkedCount: 1,
      topWorker: "Whiskers",
      dominantTier: "average",
    };
    const column = generateWorkplaceColumn(report, "Catford");
    expect(column).toContain("Catford");
  });

  it("handles 0 workers (ghost town)", () => {
    const report: ProductivityReport = {
      totalWorkers: 0,
      averageEfficiency: 0,
      slackerCount: 0,
      overworkedCount: 0,
      topWorker: null,
      dominantTier: null,
    };
    const column = generateWorkplaceColumn(report, "Catford");
    expect(column).toContain("Catford");
    expect(column.toLowerCase()).toMatch(/ghost|empty|no workers|quiet/);
  });

  it("handles multiple workers with summary", () => {
    const report: ProductivityReport = {
      totalWorkers: 10,
      averageEfficiency: 1.2,
      slackerCount: 2,
      overworkedCount: 1,
      topWorker: "Shadow",
      dominantTier: "diligent",
    };
    const column = generateWorkplaceColumn(report, "Catford");
    expect(column).toContain("10");
    expect(column).toContain("Shadow");
  });

  it("highlights slacker and overworked counts", () => {
    const report: ProductivityReport = {
      totalWorkers: 8,
      averageEfficiency: 1.0,
      slackerCount: 3,
      overworkedCount: 2,
      topWorker: "Mittens",
      dominantTier: "average",
    };
    const column = generateWorkplaceColumn(report, "Catford");
    expect(column).toContain("3");
    expect(column).toContain("2");
  });
});
