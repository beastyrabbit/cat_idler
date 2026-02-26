/**
 * Tests for Colony Achievement System
 */

import { describe, it, expect } from "vitest";
import {
  getAchievementDefinitions,
  checkAchievement,
  evaluateAchievements,
  getAchievementProgress,
  formatAchievementAnnouncement,
} from "@/lib/game/achievements";
import type {
  Achievement,
  ColonyStats,
  AchievementCategory,
} from "@/lib/game/achievements";

// ---------------------------------------------------------------------------
// Helper: zero-stat baseline
// ---------------------------------------------------------------------------
const EMPTY_STATS: ColonyStats = {
  totalCatsBorn: 0,
  catsAlive: 0,
  buildingsBuilt: 0,
  encountersSurvived: 0,
  encountersWon: 0,
  colonyAgeHours: 0,
  hasLeader: false,
  leaderAgeHours: 0,
  hasSurvivedCritical: false,
};

// ===========================================================================
// getAchievementDefinitions
// ===========================================================================
describe("getAchievementDefinitions", () => {
  it("returns at least 12 achievements", () => {
    const defs = getAchievementDefinitions();
    expect(defs.length).toBeGreaterThanOrEqual(12);
  });

  it("every achievement has required fields", () => {
    for (const a of getAchievementDefinitions()) {
      expect(a.id).toBeTruthy();
      expect(a.name).toBeTruthy();
      expect(a.description).toBeTruthy();
      expect(a.category).toBeTruthy();
      expect(typeof a.threshold).toBe("number");
      expect(a.statKey).toBeTruthy();
    }
  });

  it("ids are unique", () => {
    const ids = getAchievementDefinitions().map((a) => a.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("covers all five categories", () => {
    const cats = new Set(getAchievementDefinitions().map((a) => a.category));
    expect(cats).toContain("population");
    expect(cats).toContain("construction");
    expect(cats).toContain("survival");
    expect(cats).toContain("leadership");
    expect(cats).toContain("combat");
  });

  it("returns a new array each call (no shared mutation)", () => {
    const a = getAchievementDefinitions();
    const b = getAchievementDefinitions();
    expect(a).not.toBe(b);
    expect(a).toEqual(b);
  });
});

// ===========================================================================
// checkAchievement
// ===========================================================================
describe("checkAchievement", () => {
  const defs = getAchievementDefinitions();
  const find = (id: string) => defs.find((a) => a.id === id)!;

  // --- Population ---
  it("first_litter: unlocked at exactly 5 cats born", () => {
    const ach = find("first_litter");
    expect(checkAchievement(ach, { ...EMPTY_STATS, totalCatsBorn: 4 })).toBe(
      false,
    );
    expect(checkAchievement(ach, { ...EMPTY_STATS, totalCatsBorn: 5 })).toBe(
      true,
    );
  });

  it("growing_colony: unlocked at 10 cats alive", () => {
    const ach = find("growing_colony");
    expect(checkAchievement(ach, { ...EMPTY_STATS, catsAlive: 9 })).toBe(false);
    expect(checkAchievement(ach, { ...EMPTY_STATS, catsAlive: 10 })).toBe(true);
  });

  it("thriving_colony: unlocked at 20 cats alive", () => {
    const ach = find("thriving_colony");
    expect(checkAchievement(ach, { ...EMPTY_STATS, catsAlive: 19 })).toBe(
      false,
    );
    expect(checkAchievement(ach, { ...EMPTY_STATS, catsAlive: 20 })).toBe(true);
  });

  // --- Construction ---
  it("first_shelter: unlocked at 1 building", () => {
    const ach = find("first_shelter");
    expect(checkAchievement(ach, { ...EMPTY_STATS, buildingsBuilt: 0 })).toBe(
      false,
    );
    expect(checkAchievement(ach, { ...EMPTY_STATS, buildingsBuilt: 1 })).toBe(
      true,
    );
  });

  it("builder_colony: unlocked at 3 buildings", () => {
    const ach = find("builder_colony");
    expect(checkAchievement(ach, { ...EMPTY_STATS, buildingsBuilt: 2 })).toBe(
      false,
    );
    expect(checkAchievement(ach, { ...EMPTY_STATS, buildingsBuilt: 3 })).toBe(
      true,
    );
  });

  it("fortress: unlocked at 5 buildings", () => {
    const ach = find("fortress");
    expect(checkAchievement(ach, { ...EMPTY_STATS, buildingsBuilt: 4 })).toBe(
      false,
    );
    expect(checkAchievement(ach, { ...EMPTY_STATS, buildingsBuilt: 5 })).toBe(
      true,
    );
  });

  // --- Survival ---
  it("first_encounter_survived: unlocked at 1 encounter", () => {
    const ach = find("first_encounter_survived");
    expect(
      checkAchievement(ach, { ...EMPTY_STATS, encountersSurvived: 0 }),
    ).toBe(false);
    expect(
      checkAchievement(ach, { ...EMPTY_STATS, encountersSurvived: 1 }),
    ).toBe(true);
  });

  it("weathered_storm: unlocked when hasSurvivedCritical is true", () => {
    const ach = find("weathered_storm");
    expect(
      checkAchievement(ach, { ...EMPTY_STATS, hasSurvivedCritical: false }),
    ).toBe(false);
    expect(
      checkAchievement(ach, { ...EMPTY_STATS, hasSurvivedCritical: true }),
    ).toBe(true);
  });

  it("century_colony: unlocked at 100 colony age hours", () => {
    const ach = find("century_colony");
    expect(checkAchievement(ach, { ...EMPTY_STATS, colonyAgeHours: 99 })).toBe(
      false,
    );
    expect(checkAchievement(ach, { ...EMPTY_STATS, colonyAgeHours: 100 })).toBe(
      true,
    );
  });

  // --- Leadership ---
  it("first_leader: unlocked when hasLeader is true", () => {
    const ach = find("first_leader");
    expect(checkAchievement(ach, { ...EMPTY_STATS, hasLeader: false })).toBe(
      false,
    );
    expect(checkAchievement(ach, { ...EMPTY_STATS, hasLeader: true })).toBe(
      true,
    );
  });

  it("experienced_leader: unlocked at 24 leader age hours", () => {
    const ach = find("experienced_leader");
    expect(checkAchievement(ach, { ...EMPTY_STATS, leaderAgeHours: 23 })).toBe(
      false,
    );
    expect(checkAchievement(ach, { ...EMPTY_STATS, leaderAgeHours: 24 })).toBe(
      true,
    );
  });

  // --- Combat ---
  it("first_enemy_defeated: unlocked at 1 encounter won", () => {
    const ach = find("first_enemy_defeated");
    expect(checkAchievement(ach, { ...EMPTY_STATS, encountersWon: 0 })).toBe(
      false,
    );
    expect(checkAchievement(ach, { ...EMPTY_STATS, encountersWon: 1 })).toBe(
      true,
    );
  });

  it("veteran_defenders: unlocked at 10 encounters won", () => {
    const ach = find("veteran_defenders");
    expect(checkAchievement(ach, { ...EMPTY_STATS, encountersWon: 9 })).toBe(
      false,
    );
    expect(checkAchievement(ach, { ...EMPTY_STATS, encountersWon: 10 })).toBe(
      true,
    );
  });

  it("values above threshold also count as unlocked", () => {
    const ach = find("first_litter");
    expect(checkAchievement(ach, { ...EMPTY_STATS, totalCatsBorn: 100 })).toBe(
      true,
    );
  });
});

// ===========================================================================
// evaluateAchievements
// ===========================================================================
describe("evaluateAchievements", () => {
  it("returns empty array when no achievements are unlocked", () => {
    expect(evaluateAchievements(EMPTY_STATS)).toEqual([]);
  });

  it("returns only unlocked achievements", () => {
    const stats: ColonyStats = {
      ...EMPTY_STATS,
      totalCatsBorn: 5,
      buildingsBuilt: 1,
    };
    const unlocked = evaluateAchievements(stats);
    const ids = unlocked.map((a) => a.id);
    expect(ids).toContain("first_litter");
    expect(ids).toContain("first_shelter");
    expect(ids).not.toContain("growing_colony");
  });

  it("returns all achievements when all stats are maxed", () => {
    const maxStats: ColonyStats = {
      totalCatsBorn: 1000,
      catsAlive: 1000,
      buildingsBuilt: 1000,
      encountersSurvived: 1000,
      encountersWon: 1000,
      colonyAgeHours: 1000,
      hasLeader: true,
      leaderAgeHours: 1000,
      hasSurvivedCritical: true,
    };
    const unlocked = evaluateAchievements(maxStats);
    const defs = getAchievementDefinitions();
    expect(unlocked.length).toBe(defs.length);
  });

  it("each result has unlockedAt 'current'", () => {
    const stats: ColonyStats = { ...EMPTY_STATS, totalCatsBorn: 5 };
    const unlocked = evaluateAchievements(stats);
    for (const u of unlocked) {
      expect(u.unlockedAt).toBe("current");
    }
  });
});

// ===========================================================================
// getAchievementProgress
// ===========================================================================
describe("getAchievementProgress", () => {
  const defs = getAchievementDefinitions();
  const find = (id: string) => defs.find((a) => a.id === id)!;

  it("returns 0 for zero stats on numeric achievement", () => {
    expect(getAchievementProgress(find("first_litter"), EMPTY_STATS)).toBe(0);
  });

  it("returns 100 when threshold is met", () => {
    expect(
      getAchievementProgress(find("first_litter"), {
        ...EMPTY_STATS,
        totalCatsBorn: 5,
      }),
    ).toBe(100);
  });

  it("returns 100 when threshold is exceeded (capped)", () => {
    expect(
      getAchievementProgress(find("first_litter"), {
        ...EMPTY_STATS,
        totalCatsBorn: 50,
      }),
    ).toBe(100);
  });

  it("returns partial progress for numeric achievements", () => {
    // 3 out of 5 → 60%
    expect(
      getAchievementProgress(find("first_litter"), {
        ...EMPTY_STATS,
        totalCatsBorn: 3,
      }),
    ).toBe(60);
  });

  it("returns 0 or 100 for boolean achievements", () => {
    const ach = find("first_leader");
    expect(
      getAchievementProgress(ach, { ...EMPTY_STATS, hasLeader: false }),
    ).toBe(0);
    expect(
      getAchievementProgress(ach, { ...EMPTY_STATS, hasLeader: true }),
    ).toBe(100);
  });

  it("progress for weathered_storm (boolean)", () => {
    const ach = find("weathered_storm");
    expect(
      getAchievementProgress(ach, {
        ...EMPTY_STATS,
        hasSurvivedCritical: false,
      }),
    ).toBe(0);
    expect(
      getAchievementProgress(ach, {
        ...EMPTY_STATS,
        hasSurvivedCritical: true,
      }),
    ).toBe(100);
  });

  it("rounds progress to nearest integer", () => {
    // 1 out of 3 = 33.33...% → 33
    const ach = find("builder_colony");
    expect(
      getAchievementProgress(ach, { ...EMPTY_STATS, buildingsBuilt: 1 }),
    ).toBe(33);
  });
});

// ===========================================================================
// formatAchievementAnnouncement
// ===========================================================================
describe("formatAchievementAnnouncement", () => {
  const defs = getAchievementDefinitions();

  it("returns a non-empty string for every achievement", () => {
    for (const ach of defs) {
      const text = formatAchievementAnnouncement(ach);
      expect(text.length).toBeGreaterThan(0);
    }
  });

  it("includes the achievement name", () => {
    for (const ach of defs) {
      expect(formatAchievementAnnouncement(ach)).toContain(ach.name);
    }
  });

  it("includes the achievement description", () => {
    for (const ach of defs) {
      expect(formatAchievementAnnouncement(ach)).toContain(ach.description);
    }
  });

  it("produces unique text per achievement", () => {
    const texts = defs.map((a) => formatAchievementAnnouncement(a));
    expect(new Set(texts).size).toBe(texts.length);
  });

  it("contains a milestone marker", () => {
    for (const ach of defs) {
      const text = formatAchievementAnnouncement(ach);
      expect(text).toContain("MILESTONE");
    }
  });
});
