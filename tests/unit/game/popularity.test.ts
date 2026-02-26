import { describe, expect, it } from "vitest";

import {
  calculateSocialScore,
  calculateHeroScore,
  calculateCharmScore,
  calculateFameScore,
  getPopularityTier,
  rankCatPopularity,
  getCatOfTheMonth,
  generatePopularityColumn,
} from "@/lib/game/popularity";

import type { PopularityEntry } from "@/lib/game/popularity";

describe("popularity", () => {
  // --- calculateSocialScore ---
  describe("calculateSocialScore", () => {
    it("returns 0 for no relationships", () => {
      expect(calculateSocialScore(0, 0)).toBe(0);
    });

    it("scales linearly with relationship count", () => {
      const score5 = calculateSocialScore(5, 0);
      const score10 = calculateSocialScore(10, 0);
      expect(score10).toBeGreaterThan(score5);
    });

    it("includes mentor count", () => {
      const withoutMentors = calculateSocialScore(5, 0);
      const withMentors = calculateSocialScore(5, 3);
      expect(withMentors).toBeGreaterThan(withoutMentors);
    });

    it("caps at 25", () => {
      expect(calculateSocialScore(100, 100)).toBe(25);
    });

    it("handles single relationship", () => {
      const score = calculateSocialScore(1, 0);
      expect(score).toBeGreaterThan(0);
      expect(score).toBeLessThanOrEqual(25);
    });
  });

  // --- calculateHeroScore ---
  describe("calculateHeroScore", () => {
    it("returns 0 for no victories", () => {
      expect(calculateHeroScore(0, 0)).toBe(0);
    });

    it("caps at 25", () => {
      expect(calculateHeroScore(100, 100)).toBe(25);
    });

    it("weights combat victories higher than encounters", () => {
      const combatOnly = calculateHeroScore(5, 0);
      const encountersOnly = calculateHeroScore(0, 5);
      expect(combatOnly).toBeGreaterThan(encountersOnly);
    });

    it("combines both factors", () => {
      const combined = calculateHeroScore(3, 3);
      const combatOnly = calculateHeroScore(3, 0);
      expect(combined).toBeGreaterThan(combatOnly);
    });
  });

  // --- calculateCharmScore ---
  describe("calculateCharmScore", () => {
    it("returns higher scores for friendly/brave/clever traits", () => {
      const highCharm = calculateCharmScore(["friendly", "brave", "clever"]);
      const lowCharm = calculateCharmScore(["aggressive", "lazy"]);
      expect(highCharm).toBeGreaterThan(lowCharm);
    });

    it("returns lower scores for aggressive/lazy traits", () => {
      const score = calculateCharmScore(["aggressive", "lazy"]);
      expect(score).toBeLessThan(calculateCharmScore(["friendly"]));
    });

    it("caps at 25", () => {
      const score = calculateCharmScore([
        "friendly",
        "brave",
        "clever",
        "friendly",
        "brave",
        "clever",
        "friendly",
        "brave",
        "clever",
        "friendly",
      ]);
      expect(score).toBe(25);
    });

    it("returns 0 for empty traits", () => {
      expect(calculateCharmScore([])).toBe(0);
    });

    it("handles neutral/unknown traits", () => {
      const score = calculateCharmScore(["cautious", "curious"]);
      expect(score).toBeGreaterThan(0);
      expect(score).toBeLessThanOrEqual(25);
    });
  });

  // --- calculateFameScore ---
  describe("calculateFameScore", () => {
    it("returns 0 for no events and no leadership", () => {
      expect(calculateFameScore(0, 0)).toBe(0);
    });

    it("caps at 25", () => {
      expect(calculateFameScore(100, 100)).toBe(25);
    });

    it("weights leadership days higher than events", () => {
      const leaderOnly = calculateFameScore(0, 5);
      const eventsOnly = calculateFameScore(5, 0);
      expect(leaderOnly).toBeGreaterThan(eventsOnly);
    });

    it("combines both factors", () => {
      const combined = calculateFameScore(3, 3);
      const eventsOnly = calculateFameScore(3, 0);
      expect(combined).toBeGreaterThan(eventsOnly);
    });
  });

  // --- getPopularityTier ---
  describe("getPopularityTier", () => {
    it("returns correct tier for each score range", () => {
      expect(getPopularityTier(0)).toBe("unknown");
      expect(getPopularityTier(10)).toBe("unknown");
      expect(getPopularityTier(25)).toBe("noticed");
      expect(getPopularityTier(45)).toBe("popular");
      expect(getPopularityTier(65)).toBe("beloved");
      expect(getPopularityTier(90)).toBe("legendary");
    });

    it("handles boundary values", () => {
      expect(getPopularityTier(0)).toBe("unknown");
      expect(getPopularityTier(19)).toBe("unknown");
      expect(getPopularityTier(20)).toBe("noticed");
      expect(getPopularityTier(39)).toBe("noticed");
      expect(getPopularityTier(40)).toBe("popular");
      expect(getPopularityTier(59)).toBe("popular");
      expect(getPopularityTier(60)).toBe("beloved");
      expect(getPopularityTier(79)).toBe("beloved");
      expect(getPopularityTier(80)).toBe("legendary");
      expect(getPopularityTier(100)).toBe("legendary");
    });
  });

  // --- rankCatPopularity ---
  describe("rankCatPopularity", () => {
    it("sorts descending by total score", () => {
      const entries: PopularityEntry[] = [
        makeEntry("Whiskers", 30),
        makeEntry("Mittens", 80),
        makeEntry("Shadow", 50),
      ];
      const ranked = rankCatPopularity(entries);
      expect(ranked[0].catName).toBe("Mittens");
      expect(ranked[1].catName).toBe("Shadow");
      expect(ranked[2].catName).toBe("Whiskers");
    });

    it("handles empty array", () => {
      expect(rankCatPopularity([])).toEqual([]);
    });

    it("handles ties with stable sort", () => {
      const entries: PopularityEntry[] = [
        makeEntry("Alpha", 50),
        makeEntry("Beta", 50),
        makeEntry("Gamma", 50),
      ];
      const ranked = rankCatPopularity(entries);
      expect(ranked[0].catName).toBe("Alpha");
      expect(ranked[1].catName).toBe("Beta");
      expect(ranked[2].catName).toBe("Gamma");
    });
  });

  // --- getCatOfTheMonth ---
  describe("getCatOfTheMonth", () => {
    it("returns top-ranked entry", () => {
      const entries: PopularityEntry[] = [
        makeEntry("Whiskers", 30),
        makeEntry("Mittens", 80),
      ];
      const winner = getCatOfTheMonth(entries);
      expect(winner).not.toBeNull();
      expect(winner!.catName).toBe("Mittens");
    });

    it("returns null for empty input", () => {
      expect(getCatOfTheMonth([])).toBeNull();
    });
  });

  // --- generatePopularityColumn ---
  describe("generatePopularityColumn", () => {
    it("includes colony name", () => {
      const entries: PopularityEntry[] = [makeEntry("Whiskers", 80)];
      const column = generatePopularityColumn(entries, "Catford");
      expect(column).toContain("Catford");
    });

    it("highlights Cat of the Month", () => {
      const entries: PopularityEntry[] = [
        makeEntry("Whiskers", 90),
        makeEntry("Shadow", 40),
      ];
      const column = generatePopularityColumn(entries, "Catford");
      expect(column).toContain("Whiskers");
      expect(column.toLowerCase()).toContain("cat of the month");
    });

    it("shows top 5 cats", () => {
      const entries: PopularityEntry[] = [
        makeEntry("Cat1", 90),
        makeEntry("Cat2", 80),
        makeEntry("Cat3", 70),
        makeEntry("Cat4", 60),
        makeEntry("Cat5", 50),
        makeEntry("Cat6", 40),
      ];
      const column = generatePopularityColumn(entries, "Catford");
      expect(column).toContain("Cat1");
      expect(column).toContain("Cat5");
      expect(column).not.toContain("Cat6");
    });

    it("handles empty entries", () => {
      const column = generatePopularityColumn([], "Catford");
      expect(column).toContain("Catford");
      expect(column.toLowerCase()).toMatch(/no (cats|poll|entries|data)/);
    });
  });

  // --- purity ---
  describe("purity", () => {
    it("all functions are pure (no side effects)", () => {
      const entries: PopularityEntry[] = [
        makeEntry("A", 50),
        makeEntry("B", 30),
      ];
      const copy = [...entries];
      rankCatPopularity(entries);
      expect(entries).toEqual(copy);
    });
  });
});

// --- helper ---
function makeEntry(catName: string, totalScore: number): PopularityEntry {
  return {
    catName,
    factors: {
      socialScore: totalScore * 0.25,
      heroScore: totalScore * 0.25,
      charmScore: totalScore * 0.25,
      fameScore: totalScore * 0.25,
    },
    totalScore,
    tier:
      totalScore >= 80
        ? "legendary"
        : totalScore >= 60
          ? "beloved"
          : totalScore >= 40
            ? "popular"
            : totalScore >= 20
              ? "noticed"
              : "unknown",
  };
}
