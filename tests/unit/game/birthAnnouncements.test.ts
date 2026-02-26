/**
 * Tests for Cat Birth Announcements System
 *
 * Newspaper-style birth announcements with lineage, inherited traits,
 * omens based on colony status, and litter context.
 */

import { describe, it, expect } from "vitest";
import {
  getBirthOmen,
  getInheritedTraits,
  formatBirthAnnouncement,
  formatLitterAnnouncement,
  getBirthHeadline,
  formatBirthsColumn,
} from "@/lib/game/birthAnnouncements";
import type { CatStats } from "@/types/game";

// ---------------------------------------------------------------------------
// getBirthOmen
// ---------------------------------------------------------------------------

describe("getBirthOmen", () => {
  it('returns "Star of Plenty" for thriving colony', () => {
    const omen = getBirthOmen("thriving", 42);
    expect(omen.name).toBe("Star of Plenty");
    expect(omen.symbol).toBe("★");
    expect(omen.prediction).toBeTruthy();
    expect(omen.prediction.length).toBeGreaterThan(10);
  });

  it('returns "Moon of Resilience" for struggling colony', () => {
    const omen = getBirthOmen("struggling", 42);
    expect(omen.name).toBe("Moon of Resilience");
    expect(omen.symbol).toBe("☽");
    expect(omen.prediction).toBeTruthy();
    expect(omen.prediction.length).toBeGreaterThan(10);
  });

  it('returns "Dawn of Promise" for starting colony', () => {
    const omen = getBirthOmen("starting", 42);
    expect(omen.name).toBe("Dawn of Promise");
    expect(omen.symbol).toBe("☀");
    expect(omen.prediction).toBeTruthy();
    expect(omen.prediction.length).toBeGreaterThan(10);
  });

  it("is deterministic — same seed produces same result", () => {
    const a = getBirthOmen("thriving", 12345);
    const b = getBirthOmen("thriving", 12345);
    expect(a).toEqual(b);
  });

  it("different seeds can produce different predictions for same status", () => {
    const results = new Set<string>();
    for (let seed = 1; seed <= 100; seed++) {
      results.add(getBirthOmen("thriving", seed).prediction);
    }
    expect(results.size).toBeGreaterThan(1);
  });

  it("returns a fallback omen for dead colony", () => {
    const omen = getBirthOmen("dead", 42);
    expect(omen.name).toBeTruthy();
    expect(omen.symbol).toBeTruthy();
    expect(omen.prediction).toBeTruthy();
  });
});

// ---------------------------------------------------------------------------
// getInheritedTraits
// ---------------------------------------------------------------------------

describe("getInheritedTraits", () => {
  const baseStats: CatStats = {
    attack: 30,
    defense: 30,
    hunting: 30,
    medicine: 30,
    cleaning: 30,
    building: 30,
    leadership: 30,
    vision: 30,
  };

  it("returns empty list when no parent stats are notable", () => {
    const traits = getInheritedTraits(baseStats);
    expect(traits).toEqual([]);
  });

  it("detects hunting ≥ 70", () => {
    const traits = getInheritedTraits({ ...baseStats, hunting: 70 });
    expect(traits).toContainEqual(expect.stringContaining("hunter"));
  });

  it("detects medicine ≥ 70", () => {
    const traits = getInheritedTraits({ ...baseStats, medicine: 75 });
    expect(traits).toContainEqual(expect.stringContaining("ealer"));
  });

  it("detects leadership ≥ 70", () => {
    const traits = getInheritedTraits({ ...baseStats, leadership: 80 });
    expect(traits).toContainEqual(expect.stringContaining("leader"));
  });

  it("detects building ≥ 70", () => {
    const traits = getInheritedTraits({ ...baseStats, building: 90 });
    expect(traits).toContainEqual(expect.stringContaining("builder"));
  });

  it("detects attack ≥ 70", () => {
    const traits = getInheritedTraits({ ...baseStats, attack: 71 });
    expect(traits).toContainEqual(expect.stringContaining("ighter"));
  });

  it("detects defense ≥ 70", () => {
    const traits = getInheritedTraits({ ...baseStats, defense: 70 });
    expect(traits).toContainEqual(expect.stringContaining("uardian"));
  });

  it("detects vision ≥ 70", () => {
    const traits = getInheritedTraits({ ...baseStats, vision: 85 });
    expect(traits).toContainEqual(expect.stringContaining("cout"));
  });

  it("returns multiple traits when multiple stats are notable", () => {
    const traits = getInheritedTraits({
      ...baseStats,
      hunting: 80,
      medicine: 75,
      attack: 90,
    });
    expect(traits.length).toBe(3);
  });

  it("stat exactly at 69 does not qualify", () => {
    const traits = getInheritedTraits({ ...baseStats, hunting: 69 });
    expect(traits).toEqual([]);
  });

  it("all stats at max returns 7 traits", () => {
    const maxStats: CatStats = {
      attack: 100,
      defense: 100,
      hunting: 100,
      medicine: 100,
      cleaning: 30, // cleaning has no trait mapping
      building: 100,
      leadership: 100,
      vision: 100,
    };
    expect(getInheritedTraits(maxStats).length).toBe(7);
  });
});

// ---------------------------------------------------------------------------
// formatBirthAnnouncement
// ---------------------------------------------------------------------------

describe("formatBirthAnnouncement", () => {
  const omen = getBirthOmen("thriving", 1);

  it("includes kitten name", () => {
    const text = formatBirthAnnouncement(
      "Whiskerkit",
      ["Shadowpelt", "Moonwhisker"],
      omen,
      [],
      1,
    );
    expect(text).toContain("Whiskerkit");
  });

  it("includes parent names", () => {
    const text = formatBirthAnnouncement(
      "Whiskerkit",
      ["Shadowpelt", "Moonwhisker"],
      omen,
      [],
      1,
    );
    expect(text).toContain("Shadowpelt");
    expect(text).toContain("Moonwhisker");
  });

  it("includes omen info", () => {
    const text = formatBirthAnnouncement(
      "Whiskerkit",
      ["Shadowpelt", "Moonwhisker"],
      omen,
      [],
      1,
    );
    expect(text).toContain(omen.name);
  });

  it("includes inherited traits when present", () => {
    const traits = ["Born with a hunter's eye", "Fighter's spirit inherited"];
    const text = formatBirthAnnouncement(
      "Whiskerkit",
      ["Shadowpelt", "Moonwhisker"],
      omen,
      traits,
      1,
    );
    expect(text).toContain("hunter's eye");
    expect(text).toContain("Fighter's spirit");
  });

  it("does not mention traits when none present", () => {
    const text = formatBirthAnnouncement(
      "Whiskerkit",
      ["Shadowpelt", "Moonwhisker"],
      omen,
      [],
      1,
    );
    expect(text).not.toContain("hunter's eye");
  });
});

// ---------------------------------------------------------------------------
// formatLitterAnnouncement
// ---------------------------------------------------------------------------

describe("formatLitterAnnouncement", () => {
  it("assembles complete litter announcement for single kitten", () => {
    const result = formatLitterAnnouncement({
      kittenNames: ["Dawnkit"],
      parentNames: ["Shadowpelt", "Moonwhisker"] as [string, string],
      colonyStatus: "thriving",
      parentStats: {
        attack: 30,
        defense: 30,
        hunting: 80,
        medicine: 30,
        cleaning: 30,
        building: 30,
        leadership: 30,
        vision: 30,
      },
      seed: 42,
    });
    expect(result.litterSize).toBe(1);
    expect(result.kittenAnnouncements.length).toBe(1);
    expect(result.parentNames).toEqual(["Shadowpelt", "Moonwhisker"]);
    expect(result.omen.name).toBe("Star of Plenty");
    expect(result.headline).toBeTruthy();
  });

  it("handles twins", () => {
    const result = formatLitterAnnouncement({
      kittenNames: ["Dawnkit", "Duskkit"],
      parentNames: ["Shadowpelt", "Moonwhisker"] as [string, string],
      colonyStatus: "starting",
      parentStats: {
        attack: 30,
        defense: 30,
        hunting: 30,
        medicine: 30,
        cleaning: 30,
        building: 30,
        leadership: 30,
        vision: 30,
      },
      seed: 99,
    });
    expect(result.litterSize).toBe(2);
    expect(result.kittenAnnouncements.length).toBe(2);
  });

  it("handles large litter (5 kittens)", () => {
    const result = formatLitterAnnouncement({
      kittenNames: ["Akit", "Bkit", "Ckit", "Dkit", "Ekit"],
      parentNames: ["Pa", "Ma"] as [string, string],
      colonyStatus: "struggling",
      parentStats: {
        attack: 30,
        defense: 30,
        hunting: 30,
        medicine: 30,
        cleaning: 30,
        building: 30,
        leadership: 30,
        vision: 30,
      },
      seed: 7,
    });
    expect(result.litterSize).toBe(5);
    expect(result.kittenAnnouncements.length).toBe(5);
  });

  it("includes inherited traits in kitten announcements", () => {
    const result = formatLitterAnnouncement({
      kittenNames: ["Fiercekit"],
      parentNames: ["Braveclaw", "Gentlewhisker"] as [string, string],
      colonyStatus: "thriving",
      parentStats: {
        attack: 90,
        defense: 80,
        hunting: 30,
        medicine: 30,
        cleaning: 30,
        building: 30,
        leadership: 30,
        vision: 30,
      },
      seed: 1,
    });
    expect(result.kittenAnnouncements[0]).toContain("ighter");
  });
});

// ---------------------------------------------------------------------------
// getBirthHeadline
// ---------------------------------------------------------------------------

describe("getBirthHeadline", () => {
  it('says "single kitten" for litter of 1', () => {
    const headline = getBirthHeadline(1, "Catford");
    expect(headline.toLowerCase()).toContain("kitten");
    expect(headline).toContain("Catford");
  });

  it('says "twins" for litter of 2', () => {
    const headline = getBirthHeadline(2, "Catford");
    expect(headline.toLowerCase()).toContain("twin");
  });

  it('says "triplets" for litter of 3', () => {
    const headline = getBirthHeadline(3, "Catford");
    expect(headline.toLowerCase()).toContain("triplet");
  });

  it("uses count for large litters (4+)", () => {
    const headline = getBirthHeadline(5, "Catford");
    expect(headline).toContain("5");
  });
});

// ---------------------------------------------------------------------------
// formatBirthsColumn
// ---------------------------------------------------------------------------

describe("formatBirthsColumn", () => {
  const makeLitter = (kittenNames: string[], colonyName: string) =>
    formatLitterAnnouncement({
      kittenNames,
      parentNames: ["Pa", "Ma"] as [string, string],
      colonyStatus: "thriving",
      parentStats: {
        attack: 30,
        defense: 30,
        hunting: 30,
        medicine: 30,
        cleaning: 30,
        building: 30,
        leadership: 30,
        vision: 30,
      },
      seed: 1,
    });

  it("assembles section title", () => {
    const litter = makeLitter(["Kit1"], "Catford");
    const column = formatBirthsColumn([litter], "Catford");
    expect(column.sectionTitle).toBeTruthy();
  });

  it("calculates total births across multiple litters", () => {
    const litter1 = makeLitter(["Kit1", "Kit2"], "Catford");
    const litter2 = makeLitter(["Kit3"], "Catford");
    const column = formatBirthsColumn([litter1, litter2], "Catford");
    expect(column.totalBirths).toBe(3);
  });

  it("includes all announcements", () => {
    const litter1 = makeLitter(["Kit1"], "Catford");
    const litter2 = makeLitter(["Kit2", "Kit3"], "Catford");
    const column = formatBirthsColumn([litter1, litter2], "Catford");
    expect(column.announcements.length).toBe(2);
  });

  it("handles empty announcements list", () => {
    const column = formatBirthsColumn([], "Catford");
    expect(column.totalBirths).toBe(0);
    expect(column.announcements).toEqual([]);
    expect(column.sectionTitle).toBeTruthy();
  });

  it("includes colony name in section title", () => {
    const litter = makeLitter(["Kit1"], "Whiskertown");
    const column = formatBirthsColumn([litter], "Whiskertown");
    expect(column.sectionTitle).toContain("Whiskertown");
  });
});
