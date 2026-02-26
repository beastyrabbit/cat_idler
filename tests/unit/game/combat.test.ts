/**
 * Tests for Combat Resolution System
 *
 * TASK: LOGIC-007
 */

import { describe, it, expect, vi } from "vitest";
import {
  calculateCombatResult,
  getClicksNeeded,
  calculateColonyDefense,
} from "@/lib/game/combat";
import type { BuildingForDefense } from "@/lib/game/combat";

describe("calculateCombatResult", () => {
  it("should favor high stat cats", () => {
    let catWins = 0;
    for (let i = 0; i < 100; i++) {
      const result = calculateCombatResult(80, 80, 20);
      if (result.won) catWins++;
    }
    // High stat cat should win most of the time
    expect(catWins).toBeGreaterThan(70);
  });

  it("should return damage between 30-70 on loss", () => {
    // Force a loss scenario with very low stats
    let foundLoss = false;
    for (let i = 0; i < 100; i++) {
      const result = calculateCombatResult(1, 1, 100);
      if (!result.won) {
        foundLoss = true;
        expect(result.damage).toBeGreaterThanOrEqual(30);
        expect(result.damage).toBeLessThanOrEqual(70);
      }
    }
    expect(foundLoss).toBe(true);
  });

  it("should sometimes win and sometimes lose with balanced stats", () => {
    let wins = 0;
    let losses = 0;
    // Cat has 50+50=100 total, enemy has 100, so should be roughly equal
    for (let i = 0; i < 100; i++) {
      const result = calculateCombatResult(50, 50, 100);
      if (result.won) wins++;
      else losses++;
    }
    // Should have both wins and losses with balanced stats
    expect(wins).toBeGreaterThan(10);
    expect(losses).toBeGreaterThan(10);
  });

  it("should return won: true when cat wins", () => {
    // Mock random to ensure win
    vi.spyOn(Math, "random")
      .mockReturnValueOnce(0.9) // Cat roll high
      .mockReturnValueOnce(0.1); // Enemy roll low

    const result = calculateCombatResult(80, 80, 20);
    expect(result.won).toBe(true);

    vi.restoreAllMocks();
  });
});

describe("getClicksNeeded", () => {
  it("should return base clicks with no modifiers", () => {
    const result = getClicksNeeded(50, 0, 0);
    expect(result).toBe(50);
  });

  it("should reduce clicks with colony defense", () => {
    const result = getClicksNeeded(50, 50, 0);
    expect(result).toBe(25); // 50 * 0.5
  });

  it("should reduce clicks with cat vision", () => {
    const result = getClicksNeeded(50, 0, 100);
    expect(result).toBe(25); // 50 * 0.5
  });

  it("should stack both modifiers", () => {
    const result = getClicksNeeded(100, 50, 100);
    expect(result).toBe(25); // 100 * 0.5 * 0.5
  });

  it("should not go below 1 click", () => {
    const result = getClicksNeeded(10, 90, 90);
    expect(result).toBeGreaterThanOrEqual(1);
  });
});

describe("calculateColonyDefense", () => {
  it("should return 0 when no buildings provided", () => {
    expect(calculateColonyDefense([])).toBe(0);
  });

  it("should return 0 when no wall buildings present", () => {
    const buildings: BuildingForDefense[] = [
      { type: "den", level: 3, constructionProgress: 100 },
      { type: "food_storage", level: 2, constructionProgress: 100 },
    ];
    expect(calculateColonyDefense(buildings)).toBe(0);
  });

  it("should ignore non-wall buildings", () => {
    const buildings: BuildingForDefense[] = [
      { type: "nursery", level: 5, constructionProgress: 100 },
      { type: "herb_garden", level: 3, constructionProgress: 100 },
      { type: "mouse_farm", level: 4, constructionProgress: 100 },
    ];
    expect(calculateColonyDefense(buildings)).toBe(0);
  });

  it("should return level × 10 for a single fully-constructed wall", () => {
    const buildings: BuildingForDefense[] = [
      { type: "walls", level: 1, constructionProgress: 100 },
    ];
    expect(calculateColonyDefense(buildings)).toBe(10);
  });

  it("should return level × 10 for level 3 wall", () => {
    const buildings: BuildingForDefense[] = [
      { type: "walls", level: 3, constructionProgress: 100 },
    ];
    expect(calculateColonyDefense(buildings)).toBe(30);
  });

  it("should scale by constructionProgress", () => {
    const buildings: BuildingForDefense[] = [
      { type: "walls", level: 2, constructionProgress: 50 },
    ];
    // level 2 = 20 base, 50% progress = 10
    expect(calculateColonyDefense(buildings)).toBe(10);
  });

  it("should handle 0% constructionProgress", () => {
    const buildings: BuildingForDefense[] = [
      { type: "walls", level: 5, constructionProgress: 0 },
    ];
    expect(calculateColonyDefense(buildings)).toBe(0);
  });

  it("should stack multiple wall buildings", () => {
    const buildings: BuildingForDefense[] = [
      { type: "walls", level: 2, constructionProgress: 100 },
      { type: "walls", level: 3, constructionProgress: 100 },
    ];
    // 20 + 30 = 50
    expect(calculateColonyDefense(buildings)).toBe(50);
  });

  it("should stack walls with partial construction", () => {
    const buildings: BuildingForDefense[] = [
      { type: "walls", level: 2, constructionProgress: 50 },
      { type: "walls", level: 4, constructionProgress: 75 },
    ];
    // (2*10*0.5) + (4*10*0.75) = 10 + 30 = 40
    expect(calculateColonyDefense(buildings)).toBe(40);
  });

  it("should cap at 100", () => {
    const buildings: BuildingForDefense[] = [
      { type: "walls", level: 5, constructionProgress: 100 },
      { type: "walls", level: 5, constructionProgress: 100 },
      { type: "walls", level: 5, constructionProgress: 100 },
    ];
    // 50 + 50 + 50 = 150, but capped at 100
    expect(calculateColonyDefense(buildings)).toBe(100);
  });

  it("should handle zero level", () => {
    const buildings: BuildingForDefense[] = [
      { type: "walls", level: 0, constructionProgress: 100 },
    ];
    expect(calculateColonyDefense(buildings)).toBe(0);
  });

  it("should mix walls with other buildings", () => {
    const buildings: BuildingForDefense[] = [
      { type: "den", level: 5, constructionProgress: 100 },
      { type: "walls", level: 2, constructionProgress: 100 },
      { type: "food_storage", level: 3, constructionProgress: 100 },
      { type: "walls", level: 1, constructionProgress: 100 },
    ];
    // Only walls: 20 + 10 = 30
    expect(calculateColonyDefense(buildings)).toBe(30);
  });

  it("should floor fractional defense values", () => {
    const buildings: BuildingForDefense[] = [
      { type: "walls", level: 1, constructionProgress: 33 },
    ];
    // 1*10*0.33 = 3.3, floored to 3
    expect(calculateColonyDefense(buildings)).toBe(3);
  });
});
