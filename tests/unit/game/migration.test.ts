import { describe, it, expect } from "vitest";
import {
  calculateColonyAttractiveness,
  getMigrationProbability,
  evaluateMigrant,
  evaluateMigrationBatch,
  generateArrivalReport,
} from "@/lib/game/migration";
import type {
  ColonyAttractivenessInput,
  MigrationResult,
} from "@/lib/game/migration";

describe("calculateColonyAttractiveness", () => {
  const emptyColony: ColonyAttractivenessInput = {
    food: 0,
    water: 0,
    buildingCount: 0,
    currentPop: 10,
    maxPop: 10,
    dangerLevel: 100,
  };

  const richColony: ColonyAttractivenessInput = {
    food: 100,
    water: 100,
    buildingCount: 10,
    currentPop: 2,
    maxPop: 20,
    dangerLevel: 0,
  };

  it("returns 0 for empty/dangerous colony with no headroom", () => {
    expect(calculateColonyAttractiveness(emptyColony)).toBe(0);
  });

  it("returns ~100 for well-resourced, safe, spacious colony", () => {
    const score = calculateColonyAttractiveness(richColony);
    expect(score).toBeGreaterThanOrEqual(95);
    expect(score).toBeLessThanOrEqual(100);
  });

  it("food component caps at 25", () => {
    const input: ColonyAttractivenessInput = {
      ...emptyColony,
      food: 200,
      dangerLevel: 0,
      currentPop: 0,
      maxPop: 10,
    };
    // food=200 → min(200/50,1)*25 = 25
    // water=0 → 0, buildings=0 → 0, headroom=(10-0)/10*20=20, safety=(100-0)/100*10=10
    // total = 25 + 0 + 0 + 20 + 10 = 55
    const score = calculateColonyAttractiveness(input);
    // With even more food, score shouldn't change
    const score2 = calculateColonyAttractiveness({ ...input, food: 500 });
    expect(score).toBe(score2);
  });

  it("water component caps at 20", () => {
    const input: ColonyAttractivenessInput = {
      ...emptyColony,
      water: 100,
      dangerLevel: 0,
      currentPop: 0,
      maxPop: 10,
    };
    const score = calculateColonyAttractiveness(input);
    const score2 = calculateColonyAttractiveness({ ...input, water: 500 });
    expect(score).toBe(score2);
  });

  it("shelter component caps at 25", () => {
    const input: ColonyAttractivenessInput = {
      ...emptyColony,
      buildingCount: 5,
      dangerLevel: 0,
      currentPop: 0,
      maxPop: 10,
    };
    // buildingCount=5 → 5*5=25 (capped)
    const score = calculateColonyAttractiveness(input);
    const score2 = calculateColonyAttractiveness({
      ...input,
      buildingCount: 20,
    });
    expect(score).toBe(score2);
  });

  it("headroom component scales with available space", () => {
    const half: ColonyAttractivenessInput = {
      ...emptyColony,
      dangerLevel: 0,
      currentPop: 5,
      maxPop: 10,
    };
    // headroom = (10-5)/10 * 20 = 10
    const full: ColonyAttractivenessInput = {
      ...emptyColony,
      dangerLevel: 0,
      currentPop: 0,
      maxPop: 10,
    };
    // headroom = (10-0)/10 * 20 = 20
    const scoreHalf = calculateColonyAttractiveness(half);
    const scoreFull = calculateColonyAttractiveness(full);
    expect(scoreFull).toBeGreaterThan(scoreHalf);
  });

  it("safety component scales inversely with danger", () => {
    const safe: ColonyAttractivenessInput = {
      ...emptyColony,
      dangerLevel: 0,
      currentPop: 0,
      maxPop: 10,
    };
    const dangerous: ColonyAttractivenessInput = {
      ...emptyColony,
      dangerLevel: 80,
      currentPop: 0,
      maxPop: 10,
    };
    expect(calculateColonyAttractiveness(safe)).toBeGreaterThan(
      calculateColonyAttractiveness(dangerous),
    );
  });
});

describe("getMigrationProbability", () => {
  it("returns 0.8 for attractiveness >= 70", () => {
    expect(getMigrationProbability(70)).toBe(0.8);
    expect(getMigrationProbability(100)).toBe(0.8);
    expect(getMigrationProbability(85)).toBe(0.8);
  });

  it("returns 0.4 for attractiveness 50-69", () => {
    expect(getMigrationProbability(50)).toBe(0.4);
    expect(getMigrationProbability(69)).toBe(0.4);
  });

  it("returns 0.15 for attractiveness 30-49", () => {
    expect(getMigrationProbability(30)).toBe(0.15);
    expect(getMigrationProbability(49)).toBe(0.15);
  });

  it("returns 0.02 for attractiveness < 30", () => {
    expect(getMigrationProbability(0)).toBe(0.02);
    expect(getMigrationProbability(29)).toBe(0.02);
  });
});

describe("evaluateMigrant", () => {
  it("returns true when rngValue < probability", () => {
    // attractiveness 80 → probability 0.8
    expect(evaluateMigrant(80, 0.5)).toBe(true);
    expect(evaluateMigrant(80, 0.79)).toBe(true);
  });

  it("returns false when rngValue >= probability", () => {
    expect(evaluateMigrant(80, 0.8)).toBe(false);
    expect(evaluateMigrant(80, 0.9)).toBe(false);
  });

  it("edge case: very low attractiveness, low rng still succeeds", () => {
    // attractiveness 10 → probability 0.02
    expect(evaluateMigrant(10, 0.01)).toBe(true);
    expect(evaluateMigrant(10, 0.02)).toBe(false);
  });
});

describe("evaluateMigrationBatch", () => {
  it("uses seeded RNG for determinism", () => {
    const result1 = evaluateMigrationBatch(80, 10, 42);
    const result2 = evaluateMigrationBatch(80, 10, 42);
    expect(result1.migrants).toBe(result2.migrants);
    expect(result1.rejected).toBe(result2.rejected);
  });

  it("returns correct migrant/rejected counts", () => {
    const result = evaluateMigrationBatch(80, 10, 42);
    expect(result.migrants + result.rejected).toBe(10);
    expect(result.attractiveness).toBe(80);
    expect(result.probability).toBe(0.8);
  });

  it("handles 0 wanderers", () => {
    const result = evaluateMigrationBatch(50, 0, 42);
    expect(result.migrants).toBe(0);
    expect(result.rejected).toBe(0);
  });

  it("different seeds produce different results", () => {
    const r1 = evaluateMigrationBatch(50, 100, 1);
    const r2 = evaluateMigrationBatch(50, 100, 9999);
    // With 100 wanderers and 40% chance, statistically different seeds should differ
    // (extremely unlikely to be identical)
    expect(r1.migrants).not.toBe(r2.migrants);
  });

  it("high attractiveness accepts most migrants", () => {
    const result = evaluateMigrationBatch(90, 100, 42);
    // 80% probability — most should be accepted
    expect(result.migrants).toBeGreaterThan(50);
  });

  it("low attractiveness rejects most migrants", () => {
    const result = evaluateMigrationBatch(10, 100, 42);
    // 2% probability — almost all rejected
    expect(result.migrants).toBeLessThan(20);
  });
});

describe("generateArrivalReport", () => {
  it("includes colony name and count", () => {
    const report = generateArrivalReport(5, 75, "Whiskerfield");
    expect(report).toContain("Whiskerfield");
    expect(report).toContain("5");
  });

  it("handles 0 migrants (no arrivals message)", () => {
    const report = generateArrivalReport(0, 30, "Catford");
    expect(report).toContain("Catford");
    expect(report).not.toMatch(
      /\d+ (cat|cats|wanderer|wanderers) (arrived|joined)/i,
    );
  });

  it("handles 1 migrant (singular)", () => {
    const report = generateArrivalReport(1, 60, "Catford");
    expect(report).toMatch(/1\s/);
    // Should not use plural
    expect(report).not.toMatch(/\bcats\b/);
  });

  it("handles multiple migrants (plural)", () => {
    const report = generateArrivalReport(3, 80, "Catford");
    expect(report).toMatch(/3\s/);
    expect(report).toMatch(/\bcats\b/);
  });
});
