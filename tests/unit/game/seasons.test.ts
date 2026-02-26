/**
 * Tests for Seasonal Cycle System
 *
 * Seasons cycle: spring → summer → autumn → winter → spring → ...
 * Default season length: 6 game-hours per season (24h full cycle)
 */

import { describe, it, expect } from "vitest";
import {
  getSeason,
  getSeasonModifiers,
  getSeasonProgress,
  getSeasonTransition,
  type Season,
  type SeasonModifiers,
} from "@/lib/game/seasons";

// ─── getSeason ──────────────────────────────────────────────────────────────

describe("getSeason", () => {
  it("returns spring at age 0", () => {
    expect(getSeason(0)).toBe("spring");
  });

  it("returns spring during first season window (0-5.99h)", () => {
    expect(getSeason(3)).toBe("spring");
    expect(getSeason(5.99)).toBe("spring");
  });

  it("returns summer at 6h", () => {
    expect(getSeason(6)).toBe("summer");
  });

  it("returns summer during second window (6-11.99h)", () => {
    expect(getSeason(9)).toBe("summer");
  });

  it("returns autumn at 12h", () => {
    expect(getSeason(12)).toBe("autumn");
  });

  it("returns winter at 18h", () => {
    expect(getSeason(18)).toBe("winter");
  });

  it("cycles back to spring at 24h", () => {
    expect(getSeason(24)).toBe("spring");
  });

  it("handles multiple full cycles", () => {
    // 48h = 2 full cycles, should be spring again
    expect(getSeason(48)).toBe("spring");
    // 54h = 2 cycles + 6h = summer
    expect(getSeason(54)).toBe("summer");
  });

  it("supports custom season length", () => {
    // 12h per season: spring 0-11, summer 12-23, autumn 24-35, winter 36-47
    expect(getSeason(0, 12)).toBe("spring");
    expect(getSeason(12, 12)).toBe("summer");
    expect(getSeason(24, 12)).toBe("autumn");
    expect(getSeason(36, 12)).toBe("winter");
    expect(getSeason(48, 12)).toBe("spring");
  });

  it("handles very large colony ages", () => {
    // 1000h = 41 full cycles + 16h remainder → winter (18h boundary not reached)
    // 1000 / 6 = 166.67 seasons, 166 % 4 = 2 → autumn
    expect(getSeason(1000)).toBe("autumn");
  });

  it("handles negative age by treating as 0", () => {
    expect(getSeason(-5)).toBe("spring");
  });
});

// ─── getSeasonModifiers ─────────────────────────────────────────────────────

describe("getSeasonModifiers", () => {
  it("returns spring modifiers with breeding and herb boosts", () => {
    const mods = getSeasonModifiers("spring");
    expect(mods.food).toBeCloseTo(1.1);
    expect(mods.water).toBeCloseTo(1.0);
    expect(mods.herbs).toBeCloseTo(1.2);
    expect(mods.materials).toBeCloseTo(1.0);
    expect(mods.breeding).toBeCloseTo(1.2);
    expect(mods.encounters).toBeCloseTo(1.0);
    expect(mods.restDecay).toBeCloseTo(1.0);
  });

  it("returns summer modifiers with food boost and water penalty", () => {
    const mods = getSeasonModifiers("summer");
    expect(mods.food).toBeCloseTo(1.2);
    expect(mods.water).toBeCloseTo(0.9);
    expect(mods.herbs).toBeCloseTo(1.0);
    expect(mods.materials).toBeCloseTo(1.0);
    expect(mods.breeding).toBeCloseTo(1.0);
    expect(mods.encounters).toBeCloseTo(1.15);
    expect(mods.restDecay).toBeCloseTo(1.0);
  });

  it("returns autumn modifiers with food/materials boost and herb penalty", () => {
    const mods = getSeasonModifiers("autumn");
    expect(mods.food).toBeCloseTo(1.3);
    expect(mods.water).toBeCloseTo(1.0);
    expect(mods.herbs).toBeCloseTo(0.9);
    expect(mods.materials).toBeCloseTo(1.2);
    expect(mods.breeding).toBeCloseTo(1.0);
    expect(mods.encounters).toBeCloseTo(1.0);
    expect(mods.restDecay).toBeCloseTo(1.0);
  });

  it("returns winter modifiers with harsh penalties", () => {
    const mods = getSeasonModifiers("winter");
    expect(mods.food).toBeCloseTo(0.7);
    expect(mods.water).toBeCloseTo(0.9);
    expect(mods.herbs).toBeCloseTo(0.8);
    expect(mods.materials).toBeCloseTo(1.0);
    expect(mods.breeding).toBeCloseTo(1.0);
    expect(mods.encounters).toBeCloseTo(0.7);
    expect(mods.restDecay).toBeCloseTo(1.2);
  });

  it("returns all modifier keys for every season", () => {
    const seasons: Season[] = ["spring", "summer", "autumn", "winter"];
    const expectedKeys: (keyof SeasonModifiers)[] = [
      "food",
      "water",
      "herbs",
      "materials",
      "breeding",
      "encounters",
      "restDecay",
    ];
    for (const season of seasons) {
      const mods = getSeasonModifiers(season);
      for (const key of expectedKeys) {
        expect(mods[key]).toBeTypeOf("number");
        expect(mods[key]).toBeGreaterThan(0);
      }
    }
  });
});

// ─── getSeasonProgress ──────────────────────────────────────────────────────

describe("getSeasonProgress", () => {
  it("returns 0.0 at the start of a season", () => {
    expect(getSeasonProgress(0)).toBeCloseTo(0.0);
    expect(getSeasonProgress(6)).toBeCloseTo(0.0);
    expect(getSeasonProgress(12)).toBeCloseTo(0.0);
    expect(getSeasonProgress(18)).toBeCloseTo(0.0);
  });

  it("returns 0.5 at the midpoint of a season", () => {
    expect(getSeasonProgress(3)).toBeCloseTo(0.5);
    expect(getSeasonProgress(9)).toBeCloseTo(0.5);
  });

  it("returns close to 1.0 near the end of a season", () => {
    expect(getSeasonProgress(5.9)).toBeCloseTo(5.9 / 6, 1);
  });

  it("resets at season boundaries across cycles", () => {
    // 24h = start of new cycle, spring start
    expect(getSeasonProgress(24)).toBeCloseTo(0.0);
    // 27h = 3h into spring of second cycle
    expect(getSeasonProgress(27)).toBeCloseTo(0.5);
  });

  it("supports custom season length", () => {
    // 12h seasons: progress at hour 6 within first season
    expect(getSeasonProgress(6, 12)).toBeCloseTo(0.5);
  });

  it("handles negative age", () => {
    expect(getSeasonProgress(-3)).toBeCloseTo(0.0);
  });
});

// ─── getSeasonTransition ────────────────────────────────────────────────────

describe("getSeasonTransition", () => {
  it("returns spring→summer at age 0 with 6h until next", () => {
    const t = getSeasonTransition(0);
    expect(t.current).toBe("spring");
    expect(t.next).toBe("summer");
    expect(t.hoursUntilNext).toBeCloseTo(6.0);
  });

  it("returns summer→autumn at age 6", () => {
    const t = getSeasonTransition(6);
    expect(t.current).toBe("summer");
    expect(t.next).toBe("autumn");
    expect(t.hoursUntilNext).toBeCloseTo(6.0);
  });

  it("returns autumn→winter at age 12", () => {
    const t = getSeasonTransition(12);
    expect(t.current).toBe("autumn");
    expect(t.next).toBe("winter");
  });

  it("returns winter→spring (wraps around)", () => {
    const t = getSeasonTransition(18);
    expect(t.current).toBe("winter");
    expect(t.next).toBe("spring");
  });

  it("calculates hours remaining correctly mid-season", () => {
    // 3h into spring → 3h remaining
    const t = getSeasonTransition(3);
    expect(t.current).toBe("spring");
    expect(t.hoursUntilNext).toBeCloseTo(3.0);
  });

  it("supports custom season length", () => {
    const t = getSeasonTransition(6, 12);
    expect(t.current).toBe("spring");
    expect(t.hoursUntilNext).toBeCloseTo(6.0);
  });

  it("handles negative age", () => {
    const t = getSeasonTransition(-10);
    expect(t.current).toBe("spring");
    expect(t.next).toBe("summer");
  });
});
