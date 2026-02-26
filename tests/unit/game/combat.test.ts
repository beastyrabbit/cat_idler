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
import type { Building } from "@/types/game";

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
  function makeWall(
    level: number,
    constructionProgress: number,
  ): Pick<Building, "type" | "level" | "constructionProgress"> {
    return { type: "walls", level, constructionProgress };
  }

  it("should return 0 when no walls exist", () => {
    expect(calculateColonyDefense([])).toBe(0);
  });

  it("should return 20 for a level-1 wall at 100% construction", () => {
    expect(calculateColonyDefense([makeWall(1, 100)])).toBe(20);
  });

  it("should return 40 for a level-2 wall at 100% construction", () => {
    expect(calculateColonyDefense([makeWall(2, 100)])).toBe(40);
  });

  it("should return 100 for a level-5 wall at 100% construction", () => {
    expect(calculateColonyDefense([makeWall(5, 100)])).toBe(100);
  });

  it("should scale proportionally with construction progress", () => {
    // level 3 * 20 = 60, at 50% = 30
    expect(calculateColonyDefense([makeWall(3, 50)])).toBe(30);
  });

  it("should return 0 for a wall at 0% construction", () => {
    expect(calculateColonyDefense([makeWall(2, 0)])).toBe(0);
  });

  it("should sum defense from multiple walls", () => {
    // level 1 (20) + level 2 (40) = 60
    expect(calculateColonyDefense([makeWall(1, 100), makeWall(2, 100)])).toBe(
      60,
    );
  });

  it("should cap total defense at 100", () => {
    // level 3 (60) + level 3 (60) = 120 → capped at 100
    expect(calculateColonyDefense([makeWall(3, 100), makeWall(3, 100)])).toBe(
      100,
    );
  });

  it("should handle partial construction across multiple walls", () => {
    // level 2 at 50% = 20, level 1 at 100% = 20 → total 40
    expect(calculateColonyDefense([makeWall(2, 50), makeWall(1, 100)])).toBe(
      40,
    );
  });
});
