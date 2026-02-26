/**
 * Tests for Colony Proverbs & Wisdom System
 *
 * Pure functions for generating cat-themed colony proverbs
 * based on colony status and resource levels.
 */

import { describe, it, expect } from "vitest";
import {
  getProverbCategory,
  getColonyProverb,
  getResourceProverb,
  getDailyWisdom,
  formatEditorialFooter,
  getProverbCount,
  type ProverbCategory,
  type DailyWisdom,
} from "@/lib/game/proverbs";

// ─── getProverbCategory ─────────────────────────────────────────────────────

describe("getProverbCategory", () => {
  it("maps thriving to celebration", () => {
    expect(getProverbCategory("thriving")).toBe("celebration");
  });

  it("maps struggling to survival", () => {
    expect(getProverbCategory("struggling")).toBe("survival");
  });

  it("maps starting to hope", () => {
    expect(getProverbCategory("starting")).toBe("hope");
  });

  it("maps dead to memorial", () => {
    expect(getProverbCategory("dead")).toBe("memorial");
  });
});

// ─── getProverbCount ────────────────────────────────────────────────────────

describe("getProverbCount", () => {
  it("returns positive count for celebration", () => {
    expect(getProverbCount("celebration")).toBeGreaterThanOrEqual(5);
  });

  it("returns positive count for survival", () => {
    expect(getProverbCount("survival")).toBeGreaterThanOrEqual(5);
  });

  it("returns positive count for hope", () => {
    expect(getProverbCount("hope")).toBeGreaterThanOrEqual(5);
  });

  it("returns positive count for memorial", () => {
    expect(getProverbCount("memorial")).toBeGreaterThanOrEqual(5);
  });
});

// ─── getColonyProverb ───────────────────────────────────────────────────────

describe("getColonyProverb", () => {
  it("returns a non-empty string for thriving status", () => {
    const proverb = getColonyProverb("thriving", 42);
    expect(proverb).toBeTruthy();
    expect(typeof proverb).toBe("string");
  });

  it("returns a non-empty string for struggling status", () => {
    const proverb = getColonyProverb("struggling", 42);
    expect(proverb).toBeTruthy();
  });

  it("returns a non-empty string for starting status", () => {
    const proverb = getColonyProverb("starting", 42);
    expect(proverb).toBeTruthy();
  });

  it("returns a non-empty string for dead status", () => {
    const proverb = getColonyProverb("dead", 42);
    expect(proverb).toBeTruthy();
  });

  it("is deterministic — same seed produces same proverb", () => {
    const p1 = getColonyProverb("thriving", 123);
    const p2 = getColonyProverb("thriving", 123);
    expect(p1).toBe(p2);
  });

  it("different seeds produce different proverbs (distribution)", () => {
    const results = new Set<string>();
    for (let seed = 1; seed <= 50; seed++) {
      results.add(getColonyProverb("thriving", seed));
    }
    // Should select at least 2 different proverbs across 50 seeds
    expect(results.size).toBeGreaterThanOrEqual(2);
  });
});

// ─── getResourceProverb ─────────────────────────────────────────────────────

describe("getResourceProverb", () => {
  it("returns null when all resources are fine", () => {
    const result = getResourceProverb({ food: 50, water: 50, herbs: 50 }, 42);
    expect(result).toBeNull();
  });

  it("returns a proverb when food is critically low (≤10)", () => {
    const result = getResourceProverb({ food: 5, water: 50, herbs: 50 }, 42);
    expect(result).toBeTruthy();
    expect(typeof result).toBe("string");
  });

  it("returns a proverb when water is critically low (≤10)", () => {
    const result = getResourceProverb({ food: 50, water: 3, herbs: 50 }, 42);
    expect(result).toBeTruthy();
  });

  it("returns a proverb when herbs are critically low (≤10)", () => {
    const result = getResourceProverb({ food: 50, water: 50, herbs: 0 }, 42);
    expect(result).toBeTruthy();
  });

  it("returns null at boundary (food = 11)", () => {
    const result = getResourceProverb({ food: 11, water: 50, herbs: 50 }, 42);
    expect(result).toBeNull();
  });

  it("returns proverb at boundary (food = 10)", () => {
    const result = getResourceProverb({ food: 10, water: 50, herbs: 50 }, 42);
    expect(result).toBeTruthy();
  });

  it("returns a proverb when all resources are critical", () => {
    const result = getResourceProverb({ food: 0, water: 0, herbs: 0 }, 42);
    expect(result).toBeTruthy();
  });

  it("is deterministic — same inputs produce same result", () => {
    const resources = { food: 5, water: 50, herbs: 50 };
    const r1 = getResourceProverb(resources, 99);
    const r2 = getResourceProverb(resources, 99);
    expect(r1).toBe(r2);
  });
});

// ─── getDailyWisdom ─────────────────────────────────────────────────────────

describe("getDailyWisdom", () => {
  it("returns correct category for thriving colony", () => {
    const wisdom = getDailyWisdom(
      "thriving",
      { food: 50, water: 50, herbs: 50 },
      42,
    );
    expect(wisdom.category).toBe("celebration");
  });

  it("includes a non-empty proverb", () => {
    const wisdom = getDailyWisdom(
      "starting",
      { food: 50, water: 50, herbs: 50 },
      42,
    );
    expect(wisdom.proverb).toBeTruthy();
  });

  it("includes an attribution string", () => {
    const wisdom = getDailyWisdom(
      "thriving",
      { food: 50, water: 50, herbs: 50 },
      42,
    );
    expect(wisdom.attribution).toBeTruthy();
    expect(wisdom.attribution.startsWith("—")).toBe(true);
  });

  it("sets resourceWarning to null when resources are fine", () => {
    const wisdom = getDailyWisdom(
      "thriving",
      { food: 50, water: 50, herbs: 50 },
      42,
    );
    expect(wisdom.resourceWarning).toBeNull();
  });

  it("sets resourceWarning when food is critically low", () => {
    const wisdom = getDailyWisdom(
      "thriving",
      { food: 5, water: 50, herbs: 50 },
      42,
    );
    expect(wisdom.resourceWarning).toBeTruthy();
  });

  it("is deterministic — same inputs produce same wisdom", () => {
    const w1 = getDailyWisdom(
      "struggling",
      { food: 50, water: 50, herbs: 50 },
      77,
    );
    const w2 = getDailyWisdom(
      "struggling",
      { food: 50, water: 50, herbs: 50 },
      77,
    );
    expect(w1).toEqual(w2);
  });
});

// ─── formatEditorialFooter ──────────────────────────────────────────────────

describe("formatEditorialFooter", () => {
  it("includes the proverb text", () => {
    const wisdom: DailyWisdom = {
      proverb: "A full belly makes a bold heart",
      category: "celebration",
      resourceWarning: null,
      attribution: "— Elder wisdom",
    };
    const footer = formatEditorialFooter(wisdom);
    expect(footer).toContain("A full belly makes a bold heart");
  });

  it("includes the attribution", () => {
    const wisdom: DailyWisdom = {
      proverb: "Test proverb",
      category: "hope",
      resourceWarning: null,
      attribution: "— Colony saying",
    };
    const footer = formatEditorialFooter(wisdom);
    expect(footer).toContain("— Colony saying");
  });

  it("includes resource warning when present", () => {
    const wisdom: DailyWisdom = {
      proverb: "Test proverb",
      category: "survival",
      resourceWarning: "The larder grows thin",
      attribution: "— Elder wisdom",
    };
    const footer = formatEditorialFooter(wisdom);
    expect(footer).toContain("The larder grows thin");
  });

  it("does not include resource warning line when null", () => {
    const wisdom: DailyWisdom = {
      proverb: "Test proverb",
      category: "celebration",
      resourceWarning: null,
      attribution: "— Elder wisdom",
    };
    const footer = formatEditorialFooter(wisdom);
    // Should be relatively concise without warning
    const lines = footer.split("\n").filter((l) => l.trim());
    expect(lines.length).toBeLessThanOrEqual(3);
  });

  it("returns a non-empty string", () => {
    const wisdom: DailyWisdom = {
      proverb: "Many paws make light work",
      category: "celebration",
      resourceWarning: null,
      attribution: "— Elder wisdom",
    };
    expect(formatEditorialFooter(wisdom)).toBeTruthy();
  });
});
