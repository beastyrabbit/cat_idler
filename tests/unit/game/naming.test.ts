/**
 * Tests for Cat Naming System
 *
 * Lore-friendly name generator with warrior cats conventions:
 * prefix (nature-themed) + suffix (life-stage-based)
 */

import { describe, it, expect } from "vitest";
import {
  generateName,
  getNamePrefix,
  getNameSuffix,
  formatLeaderTitle,
  generateLitterNames,
} from "@/lib/game/naming";

// ---------------------------------------------------------------------------
// getNamePrefix
// ---------------------------------------------------------------------------
describe("getNamePrefix", () => {
  it("returns a string for seed 1", () => {
    expect(typeof getNamePrefix(1)).toBe("string");
  });

  it("returns the same prefix for the same seed", () => {
    expect(getNamePrefix(42)).toBe(getNamePrefix(42));
  });

  it("returns different prefixes for different seeds", () => {
    const a = getNamePrefix(1);
    const b = getNamePrefix(999);
    // With 30 prefixes and different seeds, these should differ
    // (not guaranteed but extremely likely)
    expect(a).not.toBe(b);
  });

  it("handles seed 0 (normalizes to valid)", () => {
    const name = getNamePrefix(0);
    expect(name.length).toBeGreaterThan(0);
  });

  it("handles negative seed", () => {
    const name = getNamePrefix(-5);
    expect(name.length).toBeGreaterThan(0);
  });
});

// ---------------------------------------------------------------------------
// getNameSuffix
// ---------------------------------------------------------------------------
describe("getNameSuffix", () => {
  it('returns "kit" for kittens', () => {
    expect(getNameSuffix("kitten", 42)).toBe("kit");
  });

  it('returns "paw" for young cats', () => {
    expect(getNameSuffix("young", 42)).toBe("paw");
  });

  it("returns a warrior suffix for adults", () => {
    const suffix = getNameSuffix("adult", 42);
    expect(suffix.length).toBeGreaterThan(0);
    expect(suffix).not.toBe("kit");
    expect(suffix).not.toBe("paw");
  });

  it("returns a warrior suffix for elders (same pool as adults)", () => {
    const suffix = getNameSuffix("elder", 42);
    expect(suffix.length).toBeGreaterThan(0);
    expect(suffix).not.toBe("kit");
    expect(suffix).not.toBe("paw");
  });

  it("returns deterministic suffix for same seed and stage", () => {
    expect(getNameSuffix("adult", 100)).toBe(getNameSuffix("adult", 100));
  });

  it("can produce different adult suffixes with different seeds", () => {
    const suffixes = new Set<string>();
    for (let seed = 1; seed <= 50; seed++) {
      suffixes.add(getNameSuffix("adult", seed));
    }
    expect(suffixes.size).toBeGreaterThan(1);
  });
});

// ---------------------------------------------------------------------------
// generateName
// ---------------------------------------------------------------------------
describe("generateName", () => {
  it("returns a string combining prefix and suffix", () => {
    const name = generateName(42);
    expect(typeof name).toBe("string");
    expect(name.length).toBeGreaterThan(3);
  });

  it("defaults to adult life stage", () => {
    const name = generateName(42);
    // Should not end with "kit" or "paw" by default
    expect(name).not.toMatch(/kit$/);
    expect(name).not.toMatch(/paw$/);
  });

  it('appends "kit" for kitten life stage', () => {
    const name = generateName(42, "kitten");
    expect(name).toMatch(/kit$/);
  });

  it('appends "paw" for young life stage', () => {
    const name = generateName(42, "young");
    expect(name).toMatch(/paw$/);
  });

  it("produces warrior suffix for adult", () => {
    const name = generateName(42, "adult");
    expect(name).not.toMatch(/kit$/);
    expect(name).not.toMatch(/paw$/);
  });

  it("produces warrior suffix for elder", () => {
    const name = generateName(42, "elder");
    expect(name).not.toMatch(/kit$/);
    expect(name).not.toMatch(/paw$/);
  });

  it("is deterministic — same seed and stage produces same name", () => {
    expect(generateName(42, "adult")).toBe(generateName(42, "adult"));
    expect(generateName(100, "kitten")).toBe(generateName(100, "kitten"));
  });

  it("produces different names for different seeds", () => {
    const names = new Set<string>();
    for (let seed = 1; seed <= 20; seed++) {
      names.add(generateName(seed, "adult"));
    }
    expect(names.size).toBeGreaterThan(1);
  });

  it("handles seed 0", () => {
    const name = generateName(0, "kitten");
    expect(name.length).toBeGreaterThan(3);
    expect(name).toMatch(/kit$/);
  });

  it("handles negative seed", () => {
    const name = generateName(-10);
    expect(name.length).toBeGreaterThan(3);
  });
});

// ---------------------------------------------------------------------------
// formatLeaderTitle
// ---------------------------------------------------------------------------
describe("formatLeaderTitle", () => {
  it('replaces suffix with "star"', () => {
    // A name like "Thornwhisker" → "Thornstar"
    const result = formatLeaderTitle("Thornwhisker");
    expect(result).toBe("Thornstar");
  });

  it('replaces "kit" suffix with "star"', () => {
    expect(formatLeaderTitle("Shadowkit")).toBe("Shadowstar");
  });

  it('replaces "paw" suffix with "star"', () => {
    expect(formatLeaderTitle("Emberpaw")).toBe("Emberstar");
  });

  it("handles single-part names gracefully", () => {
    // If somehow a name has no recognizable suffix, append "star"
    const result = formatLeaderTitle("X");
    expect(result).toContain("star");
  });

  it("works with various warrior suffixes", () => {
    expect(formatLeaderTitle("Frostclaw")).toBe("Froststar");
    expect(formatLeaderTitle("Ivyheart")).toBe("Ivystar");
    expect(formatLeaderTitle("Ashfur")).toBe("Ashstar");
  });
});

// ---------------------------------------------------------------------------
// generateLitterNames
// ---------------------------------------------------------------------------
describe("generateLitterNames", () => {
  it("returns empty array for count 0", () => {
    expect(generateLitterNames(42, 0)).toEqual([]);
  });

  it("returns one name for count 1", () => {
    const names = generateLitterNames(42, 1, "kitten");
    expect(names).toHaveLength(1);
    expect(names[0]).toMatch(/kit$/);
  });

  it("returns unique names within a litter", () => {
    const names = generateLitterNames(42, 5, "kitten");
    expect(names).toHaveLength(5);
    const unique = new Set(names);
    expect(unique.size).toBe(5);
  });

  it("defaults to kitten life stage", () => {
    const names = generateLitterNames(42, 3);
    for (const name of names) {
      expect(name).toMatch(/kit$/);
    }
  });

  it("is deterministic — same seed and count produce same names", () => {
    const a = generateLitterNames(42, 3);
    const b = generateLitterNames(42, 3);
    expect(a).toEqual(b);
  });

  it("produces different litters for different seeds", () => {
    const a = generateLitterNames(1, 3);
    const b = generateLitterNames(999, 3);
    expect(a).not.toEqual(b);
  });
});
