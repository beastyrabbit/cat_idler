import { describe, it, expect } from "vitest";
import {
  getExplorationRating,
  calculateExplorationScore,
  getTerrainDescription,
  getDangerAssessment,
  getResourceDiscovery,
  getScoutRecommendation,
  formatFieldDispatch,
  type ExplorationRating,
  type ExploreTileData,
  type FieldDispatch,
} from "@/lib/game/explorationReports";

// ============================================================================
// getExplorationRating
// ============================================================================

describe("getExplorationRating", () => {
  it("returns 'perilous' for score below 20", () => {
    expect(getExplorationRating(0)).toBe("perilous");
    expect(getExplorationRating(10)).toBe("perilous");
    expect(getExplorationRating(19)).toBe("perilous");
  });

  it("returns 'disappointing' for score 20-39", () => {
    expect(getExplorationRating(20)).toBe("disappointing");
    expect(getExplorationRating(30)).toBe("disappointing");
    expect(getExplorationRating(39)).toBe("disappointing");
  });

  it("returns 'routine' for score 40-59", () => {
    expect(getExplorationRating(40)).toBe("routine");
    expect(getExplorationRating(50)).toBe("routine");
    expect(getExplorationRating(59)).toBe("routine");
  });

  it("returns 'promising' for score 60-79", () => {
    expect(getExplorationRating(60)).toBe("promising");
    expect(getExplorationRating(70)).toBe("promising");
    expect(getExplorationRating(79)).toBe("promising");
  });

  it("returns 'remarkable' for score 80+", () => {
    expect(getExplorationRating(80)).toBe("remarkable");
    expect(getExplorationRating(90)).toBe("remarkable");
    expect(getExplorationRating(100)).toBe("remarkable");
  });
});

// ============================================================================
// calculateExplorationScore
// ============================================================================

describe("calculateExplorationScore", () => {
  const baseTile: ExploreTileData = {
    biomeType: "meadow",
    overlayFeature: null,
    dangerLevel: 10,
    resources: { food: 20, herbs: 4 },
  };

  it("returns higher score for resource-rich, low-danger tiles", () => {
    const tile: ExploreTileData = {
      biomeType: "jungle",
      overlayFeature: "ancient_road",
      dangerLevel: 0,
      resources: { food: 70, herbs: 30 },
    };
    const score = calculateExplorationScore(tile, 10);
    // Best-case scenario: high resources, no danger, good vision, road bonus
    expect(score).toBeGreaterThanOrEqual(45);
  });

  it("returns lower score for high-danger, low-resource tiles", () => {
    const tile: ExploreTileData = {
      biomeType: "enemy_lair",
      overlayFeature: null,
      dangerLevel: 80,
      resources: { food: 0, herbs: 0 },
    };
    const score = calculateExplorationScore(tile, 3);
    expect(score).toBeLessThan(20);
  });

  it("adds overlay bonus for ancient_road", () => {
    const withRoad: ExploreTileData = {
      ...baseTile,
      overlayFeature: "ancient_road",
    };
    const without = calculateExplorationScore(baseTile, 5);
    const withBonus = calculateExplorationScore(withRoad, 5);
    expect(withBonus).toBeGreaterThan(without);
  });

  it("adds overlay bonus for trade_route", () => {
    const withRoute: ExploreTileData = {
      ...baseTile,
      overlayFeature: "trade_route",
    };
    const without = calculateExplorationScore(baseTile, 5);
    const withBonus = calculateExplorationScore(withRoute, 5);
    expect(withBonus).toBeGreaterThan(without);
  });

  it("adds overlay bonus for game_trail", () => {
    const withTrail: ExploreTileData = {
      ...baseTile,
      overlayFeature: "game_trail",
    };
    const without = calculateExplorationScore(baseTile, 5);
    const withBonus = calculateExplorationScore(withTrail, 5);
    expect(withBonus).toBeGreaterThan(without);
  });

  it("higher scout vision increases score", () => {
    const lowVision = calculateExplorationScore(baseTile, 1);
    const highVision = calculateExplorationScore(baseTile, 10);
    expect(highVision).toBeGreaterThan(lowVision);
  });

  it("clamps score to 0-100", () => {
    // Zero resources, max danger
    const terrible: ExploreTileData = {
      biomeType: "enemy_lair",
      overlayFeature: null,
      dangerLevel: 100,
      resources: { food: 0, herbs: 0 },
    };
    expect(calculateExplorationScore(terrible, 0)).toBeGreaterThanOrEqual(0);

    // Max resources, zero danger
    const paradise: ExploreTileData = {
      biomeType: "jungle",
      overlayFeature: "ancient_road",
      dangerLevel: 0,
      resources: { food: 80, herbs: 35 },
    };
    expect(calculateExplorationScore(paradise, 10)).toBeLessThanOrEqual(100);
  });

  it("slow travel speed reduces score", () => {
    const fastBiome: ExploreTileData = {
      ...baseTile,
      biomeType: "meadow", // travelSpeed 1.2
    };
    const slowBiome: ExploreTileData = {
      ...baseTile,
      biomeType: "mountains", // travelSpeed 0.5
    };
    const fast = calculateExplorationScore(fastBiome, 5);
    const slow = calculateExplorationScore(slowBiome, 5);
    expect(fast).toBeGreaterThan(slow);
  });
});

// ============================================================================
// getTerrainDescription
// ============================================================================

describe("getTerrainDescription", () => {
  it("returns distinct descriptions for all 11 biome types", () => {
    const biomes = [
      "oak_forest",
      "pine_forest",
      "jungle",
      "dead_forest",
      "mountains",
      "swamp",
      "desert",
      "tundra",
      "meadow",
      "cave_entrance",
      "enemy_lair",
    ] as const;

    const descriptions = biomes.map((b) => getTerrainDescription(b, null));
    const unique = new Set(descriptions);
    expect(unique.size).toBe(11);
  });

  it("includes overlay feature context when present", () => {
    const withoutOverlay = getTerrainDescription("oak_forest", null);
    const withOverlay = getTerrainDescription("oak_forest", "ancient_road");
    expect(withOverlay).not.toBe(withoutOverlay);
    expect(withOverlay.length).toBeGreaterThan(withoutOverlay.length);
  });

  it("returns a string for each overlay feature type", () => {
    const overlays = [
      "river",
      "ancient_road",
      "game_trail",
      "trade_route",
    ] as const;
    for (const overlay of overlays) {
      const desc = getTerrainDescription("meadow", overlay);
      expect(typeof desc).toBe("string");
      expect(desc.length).toBeGreaterThan(0);
    }
  });

  it("returns non-empty string for all biomes without overlay", () => {
    const biomes = [
      "oak_forest",
      "pine_forest",
      "jungle",
      "dead_forest",
      "mountains",
      "swamp",
      "desert",
      "tundra",
      "meadow",
      "cave_entrance",
      "enemy_lair",
    ] as const;
    for (const biome of biomes) {
      const desc = getTerrainDescription(biome, null);
      expect(desc.length).toBeGreaterThan(0);
    }
  });
});

// ============================================================================
// getDangerAssessment
// ============================================================================

describe("getDangerAssessment", () => {
  it("returns low danger description for danger 0-24", () => {
    const low = getDangerAssessment(10);
    expect(typeof low).toBe("string");
    expect(low.length).toBeGreaterThan(0);
  });

  it("returns moderate danger description for danger 25-49", () => {
    const moderate = getDangerAssessment(35);
    expect(typeof moderate).toBe("string");
    expect(moderate.length).toBeGreaterThan(0);
  });

  it("returns high danger description for danger 50-74", () => {
    const high = getDangerAssessment(60);
    expect(typeof high).toBe("string");
    expect(high.length).toBeGreaterThan(0);
  });

  it("returns extreme danger description for danger 75+", () => {
    const extreme = getDangerAssessment(85);
    expect(typeof extreme).toBe("string");
    expect(extreme.length).toBeGreaterThan(0);
  });

  it("returns distinct descriptions for each danger range", () => {
    const descriptions = [
      getDangerAssessment(10),
      getDangerAssessment(35),
      getDangerAssessment(60),
      getDangerAssessment(85),
    ];
    const unique = new Set(descriptions);
    expect(unique.size).toBe(4);
  });
});

// ============================================================================
// getResourceDiscovery
// ============================================================================

describe("getResourceDiscovery", () => {
  it("mentions food quantity", () => {
    const result = getResourceDiscovery({ food: 30, herbs: 5 }, "oak_forest");
    expect(result).toContain("food");
  });

  it("mentions herbs quantity", () => {
    const result = getResourceDiscovery({ food: 10, herbs: 20 }, "swamp");
    expect(result).toContain("herb");
  });

  it("handles zero resources", () => {
    const result = getResourceDiscovery({ food: 0, herbs: 0 }, "cave_entrance");
    expect(typeof result).toBe("string");
    expect(result.length).toBeGreaterThan(0);
  });

  it("uses descriptors for different resource amounts", () => {
    const scarce = getResourceDiscovery({ food: 2, herbs: 1 }, "desert");
    const abundant = getResourceDiscovery({ food: 60, herbs: 30 }, "jungle");
    expect(scarce).not.toBe(abundant);
  });

  it("returns non-empty string for all biome types", () => {
    const biomes = [
      "oak_forest",
      "pine_forest",
      "jungle",
      "dead_forest",
      "mountains",
      "swamp",
      "desert",
      "tundra",
      "meadow",
      "cave_entrance",
      "enemy_lair",
    ] as const;
    for (const biome of biomes) {
      const result = getResourceDiscovery({ food: 10, herbs: 5 }, biome);
      expect(result.length).toBeGreaterThan(0);
    }
  });
});

// ============================================================================
// getScoutRecommendation
// ============================================================================

describe("getScoutRecommendation", () => {
  it("recommends sending more cats for remarkable rating", () => {
    const rec = getScoutRecommendation("remarkable", 10);
    expect(typeof rec).toBe("string");
    expect(rec.length).toBeGreaterThan(0);
  });

  it("varies recommendation by danger level for same rating", () => {
    const lowDanger = getScoutRecommendation("promising", 10);
    const highDanger = getScoutRecommendation("promising", 80);
    expect(lowDanger).not.toBe(highDanger);
  });

  it("recommends caution for perilous rating", () => {
    const rec = getScoutRecommendation("perilous", 90);
    expect(typeof rec).toBe("string");
    expect(rec.length).toBeGreaterThan(0);
  });

  it("returns distinct recommendations for each rating", () => {
    const ratings: ExplorationRating[] = [
      "remarkable",
      "promising",
      "routine",
      "disappointing",
      "perilous",
    ];
    const recs = ratings.map((r) => getScoutRecommendation(r, 30));
    const unique = new Set(recs);
    expect(unique.size).toBe(5);
  });
});

// ============================================================================
// formatFieldDispatch
// ============================================================================

describe("formatFieldDispatch", () => {
  const standardTile: ExploreTileData = {
    biomeType: "oak_forest",
    overlayFeature: null,
    dangerLevel: 20,
    resources: { food: 30, herbs: 5 },
  };

  it("returns all required fields", () => {
    const dispatch = formatFieldDispatch(standardTile, "Whiskers", 7);
    expect(dispatch).toHaveProperty("headline");
    expect(dispatch).toHaveProperty("terrainDescription");
    expect(dispatch).toHaveProperty("dangerAssessment");
    expect(dispatch).toHaveProperty("resourceDiscovery");
    expect(dispatch).toHaveProperty("scoutRecommendation");
    expect(dispatch).toHaveProperty("rating");
    expect(dispatch).toHaveProperty("score");
  });

  it("headline includes scout name", () => {
    const dispatch = formatFieldDispatch(standardTile, "Whiskers", 7);
    expect(dispatch.headline).toContain("Whiskers");
  });

  it("headline includes biome name", () => {
    const dispatch = formatFieldDispatch(standardTile, "Whiskers", 7);
    expect(dispatch.headline).toContain("Oak Forest");
  });

  it("rating matches score via getExplorationRating", () => {
    const dispatch = formatFieldDispatch(standardTile, "Scout", 5);
    expect(dispatch.rating).toBe(getExplorationRating(dispatch.score));
  });

  it("score is clamped 0-100", () => {
    const dispatch = formatFieldDispatch(standardTile, "Scout", 5);
    expect(dispatch.score).toBeGreaterThanOrEqual(0);
    expect(dispatch.score).toBeLessThanOrEqual(100);
  });

  it("terrainDescription matches getTerrainDescription output", () => {
    const dispatch = formatFieldDispatch(standardTile, "Scout", 5);
    expect(dispatch.terrainDescription).toBe(
      getTerrainDescription(
        standardTile.biomeType,
        standardTile.overlayFeature,
      ),
    );
  });

  it("dangerAssessment matches getDangerAssessment output", () => {
    const dispatch = formatFieldDispatch(standardTile, "Scout", 5);
    expect(dispatch.dangerAssessment).toBe(
      getDangerAssessment(standardTile.dangerLevel),
    );
  });

  it("works with overlay features", () => {
    const tileWithOverlay: ExploreTileData = {
      ...standardTile,
      overlayFeature: "trade_route",
    };
    const dispatch = formatFieldDispatch(tileWithOverlay, "Explorer", 8);
    expect(dispatch.terrainDescription).toBe(
      getTerrainDescription("oak_forest", "trade_route"),
    );
  });

  it("works with zero-resource tiles", () => {
    const barrenTile: ExploreTileData = {
      biomeType: "cave_entrance",
      overlayFeature: null,
      dangerLevel: 60,
      resources: { food: 0, herbs: 0 },
    };
    const dispatch = formatFieldDispatch(barrenTile, "Brave", 3);
    expect(dispatch.headline.length).toBeGreaterThan(0);
    expect(dispatch.rating).toBeDefined();
  });

  it("works with max-danger tiles", () => {
    const dangerTile: ExploreTileData = {
      biomeType: "enemy_lair",
      overlayFeature: null,
      dangerLevel: 100,
      resources: { food: 0, herbs: 0 },
    };
    const dispatch = formatFieldDispatch(dangerTile, "Daredevil", 1);
    expect(dispatch.score).toBeGreaterThanOrEqual(0);
    expect(dispatch.rating).toBe("perilous");
  });
});
