import { describe, it, expect } from "vitest";
import {
  getThreatLevel,
  calculateThreatScore,
  assessTileDanger,
  assessEnemyThreat,
  assessDefenseStrength,
  generateThreatReport,
} from "@/lib/game/threats";

// =============================================================================
// getThreatLevel
// =============================================================================

describe("getThreatLevel", () => {
  it("returns 'minimal' for score 0", () => {
    expect(getThreatLevel(0)).toBe("minimal");
  });

  it("returns 'minimal' for score 19", () => {
    expect(getThreatLevel(19)).toBe("minimal");
  });

  it("returns 'low' for score 20", () => {
    expect(getThreatLevel(20)).toBe("low");
  });

  it("returns 'low' for score 39", () => {
    expect(getThreatLevel(39)).toBe("low");
  });

  it("returns 'moderate' for score 40", () => {
    expect(getThreatLevel(40)).toBe("moderate");
  });

  it("returns 'moderate' for score 59", () => {
    expect(getThreatLevel(59)).toBe("moderate");
  });

  it("returns 'high' for score 60", () => {
    expect(getThreatLevel(60)).toBe("high");
  });

  it("returns 'high' for score 79", () => {
    expect(getThreatLevel(79)).toBe("high");
  });

  it("returns 'critical' for score 80", () => {
    expect(getThreatLevel(80)).toBe("critical");
  });

  it("returns 'critical' for score 100", () => {
    expect(getThreatLevel(100)).toBe("critical");
  });

  it("clamps negative score to 'minimal'", () => {
    expect(getThreatLevel(-10)).toBe("minimal");
  });

  it("clamps score above 100 to 'critical'", () => {
    expect(getThreatLevel(150)).toBe("critical");
  });
});

// =============================================================================
// assessTileDanger
// =============================================================================

describe("assessTileDanger", () => {
  it("returns 0 for empty tile array", () => {
    expect(assessTileDanger([])).toBe(0);
  });

  it("returns 0 for all-zero danger levels", () => {
    expect(assessTileDanger([0, 0, 0])).toBe(0);
  });

  it("computes average and scales to 0-30 range", () => {
    // Average of [50, 50] = 50, scaled: (50/100)*30 = 15
    expect(assessTileDanger([50, 50])).toBe(15);
  });

  it("returns 30 for max danger levels", () => {
    expect(assessTileDanger([100, 100, 100])).toBe(30);
  });

  it("handles single tile", () => {
    // Average of [80] = 80, scaled: (80/100)*30 = 24
    expect(assessTileDanger([80])).toBe(24);
  });

  it("handles mixed danger levels", () => {
    // Average of [0, 50, 100] = 50, scaled: (50/100)*30 = 15
    expect(assessTileDanger([0, 50, 100])).toBe(15);
  });
});

// =============================================================================
// assessEnemyThreat
// =============================================================================

describe("assessEnemyThreat", () => {
  it("returns 0 for empty enemy array", () => {
    expect(assessEnemyThreat([])).toBe(0);
  });

  it("calculates threat for single weak enemy (hawk)", () => {
    // hawk: baseClicks=15, damage avg=30 → strength = (15 + 30) = 45
    // frequency: 1 enemy → (1/5)*20 = 4
    // strength: 45 / maxStrength(bear: 75+60=135) → (45/135)*20 ≈ 6.67 → 7
    // total: 4 + 7 = 11
    const result = assessEnemyThreat(["hawk"]);
    expect(result).toBeGreaterThan(0);
    expect(result).toBeLessThanOrEqual(40);
  });

  it("calculates threat for single strong enemy (bear)", () => {
    const bearThreat = assessEnemyThreat(["bear"]);
    const hawkThreat = assessEnemyThreat(["hawk"]);
    expect(bearThreat).toBeGreaterThan(hawkThreat);
  });

  it("returns higher threat for more enemy types", () => {
    const oneThreat = assessEnemyThreat(["fox"]);
    const threeThreat = assessEnemyThreat(["fox", "hawk", "badger"]);
    expect(threeThreat).toBeGreaterThan(oneThreat);
  });

  it("caps at 40 for all enemy types", () => {
    const maxThreat = assessEnemyThreat([
      "fox",
      "hawk",
      "badger",
      "bear",
      "rival_cat",
    ]);
    expect(maxThreat).toBeLessThanOrEqual(40);
    expect(maxThreat).toBeGreaterThan(30); // all 5 types should be significant
  });

  it("handles duplicate enemy types (counts unique)", () => {
    const singleFox = assessEnemyThreat(["fox"]);
    const doubleFox = assessEnemyThreat(["fox", "fox"]);
    // Duplicates should count as frequency but not add new types
    expect(doubleFox).toBeGreaterThanOrEqual(singleFox);
  });
});

// =============================================================================
// assessDefenseStrength
// =============================================================================

describe("assessDefenseStrength", () => {
  it("returns 0 for no defenses", () => {
    expect(assessDefenseStrength(0, 0, 0, 10)).toBe(0);
  });

  it("returns 0 for zero colony size", () => {
    expect(assessDefenseStrength(5, 3, 50, 0)).toBe(0);
  });

  it("increases with more guards", () => {
    const fewGuards = assessDefenseStrength(1, 0, 0, 10);
    const manyGuards = assessDefenseStrength(5, 0, 0, 10);
    expect(manyGuards).toBeGreaterThan(fewGuards);
  });

  it("increases with higher wall level", () => {
    const noWalls = assessDefenseStrength(0, 0, 0, 10);
    const maxWalls = assessDefenseStrength(0, 5, 0, 10);
    expect(maxWalls).toBeGreaterThan(noWalls);
  });

  it("increases with higher avg defense stat", () => {
    const lowDef = assessDefenseStrength(0, 0, 10, 10);
    const highDef = assessDefenseStrength(0, 0, 80, 10);
    expect(highDef).toBeGreaterThan(lowDef);
  });

  it("caps at 40", () => {
    const maxDefense = assessDefenseStrength(10, 5, 100, 10);
    expect(maxDefense).toBeLessThanOrEqual(40);
  });

  it("guard ratio matters relative to colony size", () => {
    // 2 guards in colony of 4 > 2 guards in colony of 20
    const smallColony = assessDefenseStrength(2, 0, 0, 4);
    const largeColony = assessDefenseStrength(2, 0, 0, 20);
    expect(smallColony).toBeGreaterThan(largeColony);
  });
});

// =============================================================================
// calculateThreatScore
// =============================================================================

describe("calculateThreatScore", () => {
  it("returns 0 when danger is 0 and defense is 0", () => {
    expect(calculateThreatScore(0, 0)).toBe(0);
  });

  it("returns danger minus defense", () => {
    expect(calculateThreatScore(60, 20)).toBe(40);
  });

  it("clamps to 0 when defense exceeds danger", () => {
    expect(calculateThreatScore(10, 30)).toBe(0);
  });

  it("clamps to 100 when danger is extreme", () => {
    expect(calculateThreatScore(120, 0)).toBe(100);
  });

  it("returns exact score for balanced inputs", () => {
    expect(calculateThreatScore(50, 0)).toBe(50);
  });
});

// =============================================================================
// generateThreatReport
// =============================================================================

describe("generateThreatReport", () => {
  it("generates report for minimal threat", () => {
    const report = generateThreatReport(5, []);
    expect(report.toLowerCase()).toContain("minimal");
    expect(typeof report).toBe("string");
    expect(report.length).toBeGreaterThan(0);
  });

  it("generates report for critical threat", () => {
    const report = generateThreatReport(90, [
      { source: "bears", contribution: 25 },
      { source: "low walls", contribution: 15 },
    ]);
    expect(report.toLowerCase()).toContain("critical");
  });

  it("includes top threat factors in report", () => {
    const report = generateThreatReport(65, [
      { source: "nearby foxes", contribution: 20 },
    ]);
    expect(report).toContain("nearby foxes");
  });

  it("generates distinct reports for different levels", () => {
    const minimal = generateThreatReport(5, []);
    const critical = generateThreatReport(90, []);
    expect(minimal).not.toBe(critical);
  });

  it("handles empty factors gracefully", () => {
    const report = generateThreatReport(50, []);
    expect(report.length).toBeGreaterThan(0);
  });
});
