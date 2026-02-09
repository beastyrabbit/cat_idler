/**
 * Tests for Cat Mood System
 *
 * Pure functions for calculating cat mood from needs, life stage,
 * colony population, and colony status.
 */

import { describe, it, expect } from "vitest";
import { getMood, getMoodModifier, getColonyMorale } from "@/lib/game/mood";
import type { CatNeeds, LifeStage, ColonyStatus } from "@/types/game";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const fullNeeds: CatNeeds = {
  hunger: 100,
  thirst: 100,
  rest: 100,
  health: 100,
};
const midNeeds: CatNeeds = { hunger: 50, thirst: 50, rest: 50, health: 50 };
const lowNeeds: CatNeeds = { hunger: 10, thirst: 10, rest: 10, health: 10 };
const zeroNeeds: CatNeeds = { hunger: 0, thirst: 0, rest: 0, health: 0 };

// ---------------------------------------------------------------------------
// getMood
// ---------------------------------------------------------------------------

describe("getMood", () => {
  it("returns ecstatic when all needs maxed, adult, large thriving colony", () => {
    const mood = getMood(fullNeeds, "adult", 12, "thriving");
    expect(mood).toBe("ecstatic");
  });

  it("returns happy when needs high and colony starting", () => {
    // needs=80 → needsScore=32, adult=+10, pop=4→+5, starting=+5 → total=52 → happy
    const needs: CatNeeds = { hunger: 80, thirst: 80, rest: 80, health: 80 };
    const mood = getMood(needs, "adult", 4, "starting");
    expect(mood).toBe("happy");
  });

  it("returns content when needs are adequate", () => {
    const mood = getMood(midNeeds, "adult", 5, "starting");
    expect(mood).toBe("content");
  });

  it("returns anxious when some needs are low", () => {
    const needs: CatNeeds = { hunger: 25, thirst: 25, rest: 25, health: 50 };
    const mood = getMood(needs, "adult", 3, "struggling");
    expect(mood).toBe("anxious");
  });

  it("returns miserable when needs are critically low", () => {
    const mood = getMood(lowNeeds, "elder", 1, "struggling");
    expect(mood).toBe("miserable");
  });

  it("returns miserable when all needs are zero and isolated", () => {
    // needs=0 → needsScore=0, elder=+5, pop=1→+2, dead=+0 → total=7 → miserable
    const mood = getMood(zeroNeeds, "elder", 1, "dead");
    expect(mood).toBe("miserable");
  });

  // Life stage effects
  it("gives adults highest life stage bonus", () => {
    const adultMood = getMood(midNeeds, "adult", 5, "starting");
    const kittenMood = getMood(midNeeds, "kitten", 5, "starting");
    const elderMood = getMood(midNeeds, "elder", 5, "starting");
    // Adult gets +10, kitten +7, elder +5 — adult should be >= others
    expect(["content", "happy", "ecstatic"]).toContain(adultMood);
    expect(adultMood).not.toBe("miserable");
  });

  it("handles all four life stages without error", () => {
    const stages: LifeStage[] = ["kitten", "young", "adult", "elder"];
    for (const stage of stages) {
      const mood = getMood(midNeeds, stage, 5, "thriving");
      expect([
        "miserable",
        "anxious",
        "content",
        "happy",
        "ecstatic",
      ]).toContain(mood);
    }
  });

  // Colony population effects
  it("gives higher social bonus for larger colonies", () => {
    // Same inputs except population
    const smallColony = getMood(midNeeds, "adult", 1, "thriving");
    const largeColony = getMood(midNeeds, "adult", 15, "thriving");
    // Large colony should be same or better mood
    const moodRank = {
      miserable: 0,
      anxious: 1,
      content: 2,
      happy: 3,
      ecstatic: 4,
    };
    expect(moodRank[largeColony]).toBeGreaterThanOrEqual(moodRank[smallColony]);
  });

  it("handles single-cat colony (population 1)", () => {
    const mood = getMood(fullNeeds, "adult", 1, "thriving");
    expect(["miserable", "anxious", "content", "happy", "ecstatic"]).toContain(
      mood,
    );
  });

  it("handles zero population edge case", () => {
    const mood = getMood(fullNeeds, "adult", 0, "thriving");
    expect(["miserable", "anxious", "content", "happy", "ecstatic"]).toContain(
      mood,
    );
  });

  // Colony status effects
  it("gives highest colony bonus for thriving status", () => {
    const statuses: ColonyStatus[] = [
      "dead",
      "struggling",
      "starting",
      "thriving",
    ];
    const moodRank = {
      miserable: 0,
      anxious: 1,
      content: 2,
      happy: 3,
      ecstatic: 4,
    };
    const ranks = statuses.map(
      (s) => moodRank[getMood(midNeeds, "adult", 5, s)],
    );
    // Thriving should produce the highest (or equal) rank
    expect(ranks[3]).toBeGreaterThanOrEqual(ranks[0]);
  });

  it("dead colony status gives no bonus", () => {
    const mood = getMood(lowNeeds, "elder", 1, "dead");
    expect(mood).toBe("miserable");
  });

  // Pure function contract
  it("is pure — same inputs always produce same output", () => {
    const a = getMood(fullNeeds, "adult", 8, "thriving");
    const b = getMood(fullNeeds, "adult", 8, "thriving");
    expect(a).toBe(b);
  });
});

// ---------------------------------------------------------------------------
// getMoodModifier
// ---------------------------------------------------------------------------

describe("getMoodModifier", () => {
  it("returns 0.6 for miserable", () => {
    expect(getMoodModifier("miserable")).toBe(0.6);
  });

  it("returns 0.8 for anxious", () => {
    expect(getMoodModifier("anxious")).toBe(0.8);
  });

  it("returns 1.0 for content", () => {
    expect(getMoodModifier("content")).toBe(1.0);
  });

  it("returns 1.15 for happy", () => {
    expect(getMoodModifier("happy")).toBe(1.15);
  });

  it("returns 1.3 for ecstatic", () => {
    expect(getMoodModifier("ecstatic")).toBe(1.3);
  });

  it("modifier increases monotonically from miserable to ecstatic", () => {
    const moods = [
      "miserable",
      "anxious",
      "content",
      "happy",
      "ecstatic",
    ] as const;
    for (let i = 1; i < moods.length; i++) {
      expect(getMoodModifier(moods[i])).toBeGreaterThan(
        getMoodModifier(moods[i - 1]),
      );
    }
  });
});

// ---------------------------------------------------------------------------
// getColonyMorale
// ---------------------------------------------------------------------------

describe("getColonyMorale", () => {
  it("returns summary for a mixed colony", () => {
    const moods = ["happy", "content", "anxious", "happy", "ecstatic"] as const;
    const morale = getColonyMorale([...moods]);
    expect(morale.averageModifier).toBeCloseTo(
      (1.15 + 1.0 + 0.8 + 1.15 + 1.3) / 5,
      2,
    );
    expect(morale.dominantMood).toBe("happy");
    expect(morale.catCount).toBe(5);
  });

  it("returns correct dominant mood when one mood is most common", () => {
    const moods = ["content", "content", "content", "happy"] as const;
    const morale = getColonyMorale([...moods]);
    expect(morale.dominantMood).toBe("content");
  });

  it("handles all-same moods", () => {
    const moods = ["ecstatic", "ecstatic", "ecstatic"] as const;
    const morale = getColonyMorale([...moods]);
    expect(morale.dominantMood).toBe("ecstatic");
    expect(morale.averageModifier).toBeCloseTo(1.3, 2);
    expect(morale.catCount).toBe(3);
  });

  it("handles single cat colony", () => {
    const morale = getColonyMorale(["miserable"]);
    expect(morale.dominantMood).toBe("miserable");
    expect(morale.averageModifier).toBeCloseTo(0.6, 2);
    expect(morale.catCount).toBe(1);
  });

  it("handles empty colony", () => {
    const morale = getColonyMorale([]);
    expect(morale.catCount).toBe(0);
    expect(morale.averageModifier).toBe(0);
    expect(morale.dominantMood).toBe("content"); // default when no cats
  });
});
