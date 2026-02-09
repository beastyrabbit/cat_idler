/**
 * Tests for Colony Reputation System
 *
 * Colony fame/reputation scoring with gameplay modifiers.
 */

import { describe, it, expect } from "vitest";
import {
  getReputationLevel,
  calculateReputationScore,
  getRecruitmentBonus,
  getTradeModifier,
  describeReputation,
  calculateFameFromEncounters,
} from "@/lib/game/reputation";
import type { ReputationLevel, ColonyFameFactors } from "@/lib/game/reputation";

// ---------------------------------------------------------------------------
// getReputationLevel
// ---------------------------------------------------------------------------
describe("getReputationLevel", () => {
  it("returns 'unknown' for score 0", () => {
    expect(getReputationLevel(0)).toBe("unknown");
  });

  it("returns 'unknown' for score 14 (upper boundary)", () => {
    expect(getReputationLevel(14)).toBe("unknown");
  });

  it("returns 'local' for score 15 (lower boundary)", () => {
    expect(getReputationLevel(15)).toBe("local");
  });

  it("returns 'local' for score 34 (upper boundary)", () => {
    expect(getReputationLevel(34)).toBe("local");
  });

  it("returns 'regional' for score 35 (lower boundary)", () => {
    expect(getReputationLevel(35)).toBe("regional");
  });

  it("returns 'regional' for score 54 (upper boundary)", () => {
    expect(getReputationLevel(54)).toBe("regional");
  });

  it("returns 'renowned' for score 55 (lower boundary)", () => {
    expect(getReputationLevel(55)).toBe("renowned");
  });

  it("returns 'renowned' for score 79 (upper boundary)", () => {
    expect(getReputationLevel(79)).toBe("renowned");
  });

  it("returns 'legendary' for score 80 (lower boundary)", () => {
    expect(getReputationLevel(80)).toBe("legendary");
  });

  it("returns 'legendary' for score 100", () => {
    expect(getReputationLevel(100)).toBe("legendary");
  });

  it("clamps negative scores to 'unknown'", () => {
    expect(getReputationLevel(-10)).toBe("unknown");
  });

  it("clamps scores above 100 to 'legendary'", () => {
    expect(getReputationLevel(150)).toBe("legendary");
  });
});

// ---------------------------------------------------------------------------
// calculateFameFromEncounters
// ---------------------------------------------------------------------------
describe("calculateFameFromEncounters", () => {
  it("returns 0 for zero wins and zero losses", () => {
    expect(calculateFameFromEncounters(0, 0)).toBe(0);
  });

  it("returns positive fame for wins with no losses", () => {
    const fame = calculateFameFromEncounters(10, 0);
    expect(fame).toBeGreaterThan(0);
    expect(fame).toBeLessThanOrEqual(25);
  });

  it("applies logarithmic diminishing returns on wins", () => {
    const fame5 = calculateFameFromEncounters(5, 0);
    const fame50 = calculateFameFromEncounters(50, 0);
    // 10x more wins should NOT give 10x more fame
    expect(fame50 / fame5).toBeLessThan(5);
  });

  it("subtracts linear penalty for losses (capped at -15)", () => {
    const fameNoLoss = calculateFameFromEncounters(10, 0);
    const fameWithLoss = calculateFameFromEncounters(10, 3);
    expect(fameWithLoss).toBeLessThan(fameNoLoss);
    expect(fameNoLoss - fameWithLoss).toBe(6); // 3 losses × 2
  });

  it("caps loss penalty at -15", () => {
    const fameHeavyLoss = calculateFameFromEncounters(20, 20);
    const fameCappedLoss = calculateFameFromEncounters(20, 10);
    // Both should have -15 penalty (10×2=20 → capped at 15, 20×2=40 → capped at 15)
    expect(fameHeavyLoss).toBe(fameCappedLoss);
  });

  it("clamps result to minimum 0", () => {
    const fame = calculateFameFromEncounters(0, 10);
    expect(fame).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// calculateReputationScore
// ---------------------------------------------------------------------------
describe("calculateReputationScore", () => {
  it("returns 0 for all-zero stats", () => {
    const factors: ColonyFameFactors = {
      encountersWon: 0,
      encountersLost: 0,
      buildingsCompleted: 0,
      colonyAgeHours: 0,
      catsAlive: 0,
      isCritical: false,
    };
    expect(calculateReputationScore(factors)).toBe(0);
  });

  it("combines fame factors correctly", () => {
    const factors: ColonyFameFactors = {
      encountersWon: 10,
      encountersLost: 0,
      buildingsCompleted: 3,
      colonyAgeHours: 50,
      catsAlive: 10,
      isCritical: false,
    };
    const score = calculateReputationScore(factors);
    expect(score).toBeGreaterThan(0);
    expect(score).toBeLessThanOrEqual(100);
  });

  it("applies -10 penalty when isCritical is true", () => {
    const base: ColonyFameFactors = {
      encountersWon: 10,
      encountersLost: 0,
      buildingsCompleted: 3,
      colonyAgeHours: 50,
      catsAlive: 10,
      isCritical: false,
    };
    const critical: ColonyFameFactors = { ...base, isCritical: true };
    const baseScore = calculateReputationScore(base);
    const criticalScore = calculateReputationScore(critical);
    expect(baseScore - criticalScore).toBe(10);
  });

  it("caps at sum of individual maxima (25+20+15+15=75)", () => {
    const factors: ColonyFameFactors = {
      encountersWon: 1000,
      encountersLost: 0,
      buildingsCompleted: 100,
      colonyAgeHours: 10000,
      catsAlive: 100,
      isCritical: false,
    };
    const score = calculateReputationScore(factors);
    expect(score).toBe(75);
    expect(score).toBeLessThanOrEqual(100);
  });

  it("clamps result to minimum 0 even with heavy penalties", () => {
    const factors: ColonyFameFactors = {
      encountersWon: 0,
      encountersLost: 50,
      buildingsCompleted: 0,
      colonyAgeHours: 0,
      catsAlive: 0,
      isCritical: true,
    };
    expect(calculateReputationScore(factors)).toBe(0);
  });

  it("buildings contribution caps at 20", () => {
    const few: ColonyFameFactors = {
      encountersWon: 0,
      encountersLost: 0,
      buildingsCompleted: 5,
      colonyAgeHours: 0,
      catsAlive: 0,
      isCritical: false,
    };
    const many: ColonyFameFactors = { ...few, buildingsCompleted: 100 };
    expect(calculateReputationScore(many)).toBe(calculateReputationScore(few));
    expect(calculateReputationScore(few)).toBe(20);
  });

  it("population contribution caps at 15", () => {
    const base: ColonyFameFactors = {
      encountersWon: 0,
      encountersLost: 0,
      buildingsCompleted: 0,
      colonyAgeHours: 0,
      catsAlive: 30,
      isCritical: false,
    };
    const bigger: ColonyFameFactors = { ...base, catsAlive: 200 };
    expect(calculateReputationScore(base)).toBe(
      calculateReputationScore(bigger),
    );
    expect(calculateReputationScore(base)).toBe(15);
  });
});

// ---------------------------------------------------------------------------
// getRecruitmentBonus
// ---------------------------------------------------------------------------
describe("getRecruitmentBonus", () => {
  it("returns 0 for 'unknown'", () => {
    expect(getRecruitmentBonus("unknown")).toBe(0);
  });

  it("returns 10 for 'local'", () => {
    expect(getRecruitmentBonus("local")).toBe(10);
  });

  it("returns 20 for 'regional'", () => {
    expect(getRecruitmentBonus("regional")).toBe(20);
  });

  it("returns 35 for 'renowned'", () => {
    expect(getRecruitmentBonus("renowned")).toBe(35);
  });

  it("returns 50 for 'legendary'", () => {
    expect(getRecruitmentBonus("legendary")).toBe(50);
  });
});

// ---------------------------------------------------------------------------
// getTradeModifier
// ---------------------------------------------------------------------------
describe("getTradeModifier", () => {
  it("returns 1.0 for 'unknown'", () => {
    expect(getTradeModifier("unknown")).toBe(1.0);
  });

  it("returns 0.95 for 'local'", () => {
    expect(getTradeModifier("local")).toBe(0.95);
  });

  it("returns 0.9 for 'regional'", () => {
    expect(getTradeModifier("regional")).toBe(0.9);
  });

  it("returns 0.8 for 'renowned'", () => {
    expect(getTradeModifier("renowned")).toBe(0.8);
  });

  it("returns 0.7 for 'legendary'", () => {
    expect(getTradeModifier("legendary")).toBe(0.7);
  });
});

// ---------------------------------------------------------------------------
// describeReputation
// ---------------------------------------------------------------------------
describe("describeReputation", () => {
  it("returns a string for each level", () => {
    const levels: ReputationLevel[] = [
      "unknown",
      "local",
      "regional",
      "renowned",
      "legendary",
    ];
    for (const level of levels) {
      const desc = describeReputation(level);
      expect(typeof desc).toBe("string");
      expect(desc.length).toBeGreaterThan(0);
    }
  });

  it("returns distinct descriptions for each level", () => {
    const levels: ReputationLevel[] = [
      "unknown",
      "local",
      "regional",
      "renowned",
      "legendary",
    ];
    const descriptions = levels.map(describeReputation);
    const unique = new Set(descriptions);
    expect(unique.size).toBe(levels.length);
  });

  it("each description reads like newspaper prose", () => {
    const desc = describeReputation("legendary");
    // Should be a sentence, not just a word
    expect(desc.length).toBeGreaterThan(20);
  });
});
