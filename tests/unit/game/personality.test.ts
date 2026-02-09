/**
 * Tests for Cat Personality Profile System
 */

import { describe, it, expect } from "vitest";
import {
  getDominantStats,
  getPersonalityArchetype,
  getPersonalityDescription,
  getPersonalityQuirk,
  getCompatibility,
  formatPersonalityProfile,
} from "@/lib/game/personality";
import type { CatStats, CatSpecialization, LifeStage } from "@/types/game";

// Helper to create stats with specific values
function makeStats(overrides: Partial<CatStats> = {}): CatStats {
  return {
    attack: 5,
    defense: 5,
    hunting: 5,
    medicine: 5,
    cleaning: 5,
    building: 5,
    leadership: 5,
    vision: 5,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// getDominantStats
// ---------------------------------------------------------------------------
describe("getDominantStats", () => {
  it("identifies the two highest stats", () => {
    const stats = makeStats({ hunting: 20, vision: 15 });
    const result = getDominantStats(stats);
    expect(result.primary).toBe("hunting");
    expect(result.secondary).toBe("vision");
  });

  it("handles ties by picking first in stat key order", () => {
    // All equal — first two keys in CatStats order are attack, defense
    const stats = makeStats();
    const result = getDominantStats(stats);
    expect(result.primary).toBe("attack");
    expect(result.secondary).toBe("defense");
  });

  it("picks correct pair when secondary ties with others", () => {
    const stats = makeStats({ leadership: 20 });
    // leadership is highest at 20, all others tie at 5 — first key "attack" is secondary
    const result = getDominantStats(stats);
    expect(result.primary).toBe("leadership");
    expect(result.secondary).toBe("attack");
  });

  it("handles zero stats", () => {
    const stats: CatStats = {
      attack: 0,
      defense: 0,
      hunting: 0,
      medicine: 0,
      cleaning: 0,
      building: 0,
      leadership: 0,
      vision: 0,
    };
    const result = getDominantStats(stats);
    expect(result.primary).toBe("attack");
    expect(result.secondary).toBe("defense");
  });

  it("handles maximum stats", () => {
    const stats = makeStats({ vision: 100, medicine: 99 });
    const result = getDominantStats(stats);
    expect(result.primary).toBe("vision");
    expect(result.secondary).toBe("medicine");
  });

  it("returns different stats for primary and secondary", () => {
    const stats = makeStats({ building: 50, cleaning: 40 });
    const result = getDominantStats(stats);
    expect(result.primary).not.toBe(result.secondary);
  });
});

// ---------------------------------------------------------------------------
// getPersonalityArchetype
// ---------------------------------------------------------------------------
describe("getPersonalityArchetype", () => {
  it("returns adventurer for high hunting + vision", () => {
    const stats = makeStats({ hunting: 20, vision: 15 });
    expect(getPersonalityArchetype(stats)).toBe("adventurer");
  });

  it("returns guardian for high defense + attack", () => {
    const stats = makeStats({ defense: 20, attack: 15 });
    expect(getPersonalityArchetype(stats)).toBe("guardian");
  });

  it("returns scholar for high medicine + vision", () => {
    const stats = makeStats({ medicine: 20, vision: 15 });
    expect(getPersonalityArchetype(stats)).toBe("scholar");
  });

  it("returns socialite for high leadership + cleaning", () => {
    const stats = makeStats({ leadership: 20, cleaning: 15 });
    expect(getPersonalityArchetype(stats)).toBe("socialite");
  });

  it("returns mystic for high vision + medicine", () => {
    // vision dominant, medicine secondary
    const stats = makeStats({ vision: 20, medicine: 15 });
    expect(getPersonalityArchetype(stats)).toBe("mystic");
  });

  it("returns worker for high building + cleaning", () => {
    const stats = makeStats({ building: 20, cleaning: 15 });
    expect(getPersonalityArchetype(stats)).toBe("worker");
  });

  it("returns generalist when all stats are equal", () => {
    const stats = makeStats();
    expect(getPersonalityArchetype(stats)).toBe("generalist");
  });

  it("returns generalist when no archetype pattern matches", () => {
    // attack + cleaning = no defined archetype pair
    const stats = makeStats({ attack: 20, cleaning: 15 });
    expect(getPersonalityArchetype(stats)).toBe("generalist");
  });
});

// ---------------------------------------------------------------------------
// getPersonalityDescription
// ---------------------------------------------------------------------------
describe("getPersonalityDescription", () => {
  it("returns a non-empty string for each archetype", () => {
    const archetypes = [
      "adventurer",
      "guardian",
      "scholar",
      "socialite",
      "mystic",
      "worker",
      "generalist",
    ] as const;
    for (const arch of archetypes) {
      const desc = getPersonalityDescription(arch, "adult");
      expect(desc).toBeTruthy();
      expect(typeof desc).toBe("string");
    }
  });

  it("returns different descriptions for different archetypes", () => {
    const d1 = getPersonalityDescription("adventurer", "adult");
    const d2 = getPersonalityDescription("guardian", "adult");
    expect(d1).not.toBe(d2);
  });

  it("adjusts description for kitten life stage", () => {
    const adultDesc = getPersonalityDescription("adventurer", "adult");
    const kittenDesc = getPersonalityDescription("adventurer", "kitten");
    expect(kittenDesc).not.toBe(adultDesc);
  });

  it("adjusts description for elder life stage", () => {
    const adultDesc = getPersonalityDescription("adventurer", "adult");
    const elderDesc = getPersonalityDescription("adventurer", "elder");
    expect(elderDesc).not.toBe(adultDesc);
  });

  it("treats young and adult similarly", () => {
    // young and adult both get the standard description
    const youngDesc = getPersonalityDescription("guardian", "young");
    const adultDesc = getPersonalityDescription("guardian", "adult");
    expect(youngDesc).toBe(adultDesc);
  });
});

// ---------------------------------------------------------------------------
// getPersonalityQuirk
// ---------------------------------------------------------------------------
describe("getPersonalityQuirk", () => {
  it("returns a non-empty string for each archetype", () => {
    const archetypes = [
      "adventurer",
      "guardian",
      "scholar",
      "socialite",
      "mystic",
      "worker",
      "generalist",
    ] as const;
    for (const arch of archetypes) {
      const quirk = getPersonalityQuirk(arch, null);
      expect(quirk).toBeTruthy();
    }
  });

  it("returns different quirks for different archetypes", () => {
    const q1 = getPersonalityQuirk("adventurer", null);
    const q2 = getPersonalityQuirk("guardian", null);
    expect(q1).not.toBe(q2);
  });

  it("returns specialized quirk for hunter adventurer", () => {
    const generic = getPersonalityQuirk("adventurer", null);
    const specialized = getPersonalityQuirk("adventurer", "hunter");
    expect(specialized).not.toBe(generic);
  });

  it("returns specialized quirk for architect worker", () => {
    const generic = getPersonalityQuirk("worker", null);
    const specialized = getPersonalityQuirk("worker", "architect");
    expect(specialized).not.toBe(generic);
  });

  it("returns specialized quirk for ritualist mystic", () => {
    const generic = getPersonalityQuirk("mystic", null);
    const specialized = getPersonalityQuirk("mystic", "ritualist");
    expect(specialized).not.toBe(generic);
  });

  it("returns generic quirk for non-matching specialization", () => {
    // architect with adventurer archetype — no special pairing
    const generic = getPersonalityQuirk("adventurer", null);
    const withSpec = getPersonalityQuirk("adventurer", "architect");
    expect(withSpec).toBe(generic);
  });

  it("handles null specialization", () => {
    const quirk = getPersonalityQuirk("scholar", null);
    expect(typeof quirk).toBe("string");
    expect(quirk.length).toBeGreaterThan(0);
  });
});

// ---------------------------------------------------------------------------
// getCompatibility
// ---------------------------------------------------------------------------
describe("getCompatibility", () => {
  it("returns perfect for same archetype", () => {
    expect(getCompatibility("adventurer", "adventurer")).toBe("perfect");
  });

  it("is symmetric", () => {
    expect(getCompatibility("adventurer", "guardian")).toBe(
      getCompatibility("guardian", "adventurer"),
    );
  });

  it("returns a valid compatibility level", () => {
    const levels = ["perfect", "good", "neutral", "poor"];
    const archetypes = [
      "adventurer",
      "guardian",
      "scholar",
      "socialite",
      "mystic",
      "worker",
      "generalist",
    ] as const;
    for (const a of archetypes) {
      for (const b of archetypes) {
        expect(levels).toContain(getCompatibility(a, b));
      }
    }
  });

  it("has at least one good pairing", () => {
    // adventurer + mystic should be good (exploration + vision)
    expect(getCompatibility("adventurer", "mystic")).toBe("good");
  });

  it("has at least one poor pairing", () => {
    // socialite + mystic — opposite temperaments
    expect(getCompatibility("socialite", "mystic")).toBe("poor");
  });

  it("generalist is neutral with everyone", () => {
    const archetypes = [
      "adventurer",
      "guardian",
      "scholar",
      "socialite",
      "mystic",
      "worker",
    ] as const;
    for (const a of archetypes) {
      expect(getCompatibility("generalist", a)).toBe("neutral");
    }
  });
});

// ---------------------------------------------------------------------------
// formatPersonalityProfile
// ---------------------------------------------------------------------------
describe("formatPersonalityProfile", () => {
  const baseCatData = {
    name: "Whiskers",
    stats: makeStats({ hunting: 20, vision: 15 }),
    lifeStage: "adult" as LifeStage,
    specialization: null as CatSpecialization,
  };

  it("returns a complete profile with all fields", () => {
    const profile = formatPersonalityProfile(baseCatData);
    expect(profile).toHaveProperty("archetype");
    expect(profile).toHaveProperty("title");
    expect(profile).toHaveProperty("description");
    expect(profile).toHaveProperty("quirk");
    expect(profile).toHaveProperty("dominantStats");
  });

  it("determines correct archetype from stats", () => {
    const profile = formatPersonalityProfile(baseCatData);
    expect(profile.archetype).toBe("adventurer");
  });

  it("includes cat name in title", () => {
    const profile = formatPersonalityProfile(baseCatData);
    expect(profile.title).toContain("Whiskers");
  });

  it("includes archetype label in title", () => {
    const profile = formatPersonalityProfile(baseCatData);
    expect(profile.title.toLowerCase()).toContain("adventurer");
  });

  it("provides dominant stats", () => {
    const profile = formatPersonalityProfile(baseCatData);
    expect(profile.dominantStats.primary).toBe("hunting");
    expect(profile.dominantStats.secondary).toBe("vision");
  });

  it("has non-empty description and quirk", () => {
    const profile = formatPersonalityProfile(baseCatData);
    expect(profile.description.length).toBeGreaterThan(0);
    expect(profile.quirk.length).toBeGreaterThan(0);
  });

  it("handles generalist cat", () => {
    const profile = formatPersonalityProfile({
      ...baseCatData,
      stats: makeStats(),
    });
    expect(profile.archetype).toBe("generalist");
  });

  it("handles kitten with specialization", () => {
    const profile = formatPersonalityProfile({
      ...baseCatData,
      lifeStage: "kitten",
      specialization: "hunter",
      stats: makeStats({ hunting: 20, vision: 15 }),
    });
    expect(profile.archetype).toBe("adventurer");
    expect(profile.description).toBeTruthy();
    expect(profile.quirk).toBeTruthy();
  });

  it("handles elder worker with architect specialization", () => {
    const profile = formatPersonalityProfile({
      name: "Oldpaws",
      stats: makeStats({ building: 20, cleaning: 15 }),
      lifeStage: "elder",
      specialization: "architect",
    });
    expect(profile.archetype).toBe("worker");
    expect(profile.title).toContain("Oldpaws");
  });

  it("profile quirk is specialized when matching", () => {
    const genericProfile = formatPersonalityProfile({
      ...baseCatData,
      specialization: null,
    });
    const specProfile = formatPersonalityProfile({
      ...baseCatData,
      specialization: "hunter",
    });
    expect(specProfile.quirk).not.toBe(genericProfile.quirk);
  });
});
