/**
 * Tests for Territory Influence System
 *
 * Pure functions for calculating colony territorial control over world map tiles.
 */

import { describe, it, expect } from "vitest";
import {
  getInfluenceDecay,
  calculateTileInfluence,
  classifyTerritory,
  calculateTerritorySummary,
  type TerritoryStatus,
  type TileInfluenceInput,
  type TerritorySummary,
} from "@/lib/game/territory";

// =============================================================================
// getInfluenceDecay
// =============================================================================

describe("getInfluenceDecay", () => {
  it("returns 100 at distance 0 (colony center)", () => {
    expect(getInfluenceDecay(0)).toBe(100);
  });

  it("returns 50 at distance 1", () => {
    expect(getInfluenceDecay(1)).toBe(50);
  });

  it("returns ~33.33 at distance 2", () => {
    expect(getInfluenceDecay(2)).toBeCloseTo(33.33, 1);
  });

  it("returns 25 at distance 3", () => {
    expect(getInfluenceDecay(3)).toBe(25);
  });

  it("decays monotonically with increasing distance", () => {
    for (let d = 0; d < 10; d++) {
      expect(getInfluenceDecay(d)).toBeGreaterThan(getInfluenceDecay(d + 1));
    }
  });

  it("returns 0 for negative distances", () => {
    expect(getInfluenceDecay(-1)).toBe(0);
    expect(getInfluenceDecay(-100)).toBe(0);
  });

  it("approaches 0 at large distances", () => {
    expect(getInfluenceDecay(99)).toBeLessThan(2);
  });
});

// =============================================================================
// calculateTileInfluence
// =============================================================================

describe("calculateTileInfluence", () => {
  it("returns base influence with no cats/guards on neutral tile", () => {
    expect(calculateTileInfluence(0, 0, 0, "field")).toBe(100);
    expect(calculateTileInfluence(1, 0, 0, "field")).toBe(50);
  });

  it("adds patrol bonus of +15 per cat", () => {
    // distance 1 → base 50, +15 for one cat = 65
    expect(calculateTileInfluence(1, 1, 0, "field")).toBe(65);
    // distance 1 → base 50, +30 for two cats = 80
    expect(calculateTileInfluence(1, 2, 0, "field")).toBe(80);
  });

  it("adds guard bonus of +10 per guard", () => {
    // distance 1 → base 50, +10 for one guard = 60
    expect(calculateTileInfluence(1, 0, 1, "field")).toBe(60);
    // distance 1 → base 50, +20 for two guards = 70
    expect(calculateTileInfluence(1, 0, 2, "field")).toBe(70);
  });

  it("combines patrol and guard bonuses", () => {
    // distance 1 → base 50, +15 cat + 10 guard = 75
    expect(calculateTileInfluence(1, 1, 1, "field")).toBe(75);
  });

  it("subtracts enemy penalty for enemy_territory (-20)", () => {
    // distance 0 → base 100, -20 enemy = 80
    expect(calculateTileInfluence(0, 0, 0, "enemy_territory")).toBe(80);
  });

  it("subtracts enemy penalty for enemy_lair (-30)", () => {
    // distance 0 → base 100, -30 lair = 70
    expect(calculateTileInfluence(0, 0, 0, "enemy_lair")).toBe(70);
  });

  it("clamps result to minimum 0", () => {
    // distance 9 → base 10, -30 lair = negative → clamped to 0
    expect(calculateTileInfluence(9, 0, 0, "enemy_lair")).toBe(0);
  });

  it("clamps result to maximum 100", () => {
    // distance 0 → base 100, +30 from 2 cats = 130 → clamped to 100
    expect(calculateTileInfluence(0, 2, 0, "field")).toBe(100);
  });

  it("handles negative distance gracefully (base 0)", () => {
    expect(calculateTileInfluence(-1, 2, 2, "field")).toBe(50);
    // 0 base + 30 cats + 20 guards = 50
  });

  it("handles zero cats and guards on enemy lair at distance", () => {
    // distance 3 → base 25, -30 lair = negative → 0
    expect(calculateTileInfluence(3, 0, 0, "enemy_lair")).toBe(0);
  });
});

// =============================================================================
// classifyTerritory
// =============================================================================

describe("classifyTerritory", () => {
  it('returns "controlled" for influence >= 60', () => {
    expect(classifyTerritory(60)).toBe("controlled");
    expect(classifyTerritory(80)).toBe("controlled");
    expect(classifyTerritory(100)).toBe("controlled");
  });

  it('returns "contested" for influence 30-59', () => {
    expect(classifyTerritory(30)).toBe("contested");
    expect(classifyTerritory(45)).toBe("contested");
    expect(classifyTerritory(59)).toBe("contested");
  });

  it('returns "wild" for influence < 30', () => {
    expect(classifyTerritory(0)).toBe("wild");
    expect(classifyTerritory(15)).toBe("wild");
    expect(classifyTerritory(29)).toBe("wild");
  });

  it("handles boundary values exactly", () => {
    expect(classifyTerritory(59.9)).toBe("contested");
    expect(classifyTerritory(29.9)).toBe("wild");
  });
});

// =============================================================================
// calculateTerritorySummary
// =============================================================================

describe("calculateTerritorySummary", () => {
  it("counts tiles per category correctly", () => {
    const tiles: TileInfluenceInput[] = [
      { distance: 0, catCount: 0, guardCount: 0, tileType: "field" }, // 100 → controlled
      { distance: 1, catCount: 0, guardCount: 0, tileType: "field" }, // 50 → contested
      { distance: 9, catCount: 0, guardCount: 0, tileType: "enemy_lair" }, // 0 → wild
    ];
    const summary = calculateTerritorySummary(tiles);
    expect(summary.controlled).toBe(1);
    expect(summary.contested).toBe(1);
    expect(summary.wild).toBe(1);
  });

  it("computes totalInfluence as sum of all tile influences", () => {
    const tiles: TileInfluenceInput[] = [
      { distance: 0, catCount: 0, guardCount: 0, tileType: "field" }, // 100
      { distance: 1, catCount: 0, guardCount: 0, tileType: "field" }, // 50
    ];
    const summary = calculateTerritorySummary(tiles);
    expect(summary.totalInfluence).toBe(150);
  });

  it("computes averageInfluence correctly", () => {
    const tiles: TileInfluenceInput[] = [
      { distance: 0, catCount: 0, guardCount: 0, tileType: "field" }, // 100
      { distance: 1, catCount: 0, guardCount: 0, tileType: "field" }, // 50
    ];
    const summary = calculateTerritorySummary(tiles);
    expect(summary.averageInfluence).toBe(75);
  });

  it("handles empty tile array", () => {
    const summary = calculateTerritorySummary([]);
    expect(summary.controlled).toBe(0);
    expect(summary.contested).toBe(0);
    expect(summary.wild).toBe(0);
    expect(summary.totalInfluence).toBe(0);
    expect(summary.averageInfluence).toBe(0);
  });

  it("counts multiple controlled tiles", () => {
    const tiles: TileInfluenceInput[] = [
      { distance: 0, catCount: 0, guardCount: 0, tileType: "field" }, // 100
      { distance: 0, catCount: 1, guardCount: 0, tileType: "field" }, // 100 (clamped)
      { distance: 1, catCount: 1, guardCount: 0, tileType: "field" }, // 65
    ];
    const summary = calculateTerritorySummary(tiles);
    expect(summary.controlled).toBe(3);
    expect(summary.contested).toBe(0);
    expect(summary.wild).toBe(0);
  });

  it("handles all-wild scenario at large distances", () => {
    const tiles: TileInfluenceInput[] = [
      { distance: 20, catCount: 0, guardCount: 0, tileType: "field" },
      { distance: 30, catCount: 0, guardCount: 0, tileType: "field" },
      { distance: 50, catCount: 0, guardCount: 0, tileType: "field" },
    ];
    const summary = calculateTerritorySummary(tiles);
    expect(summary.controlled).toBe(0);
    expect(summary.contested).toBe(0);
    expect(summary.wild).toBe(3);
  });
});
