import { describe, it, expect } from "vitest";
import {
  getSkillTier,
  getSkillTitle,
  rankCatsBySkill,
  findColonyChampions,
  generateRivalryReport,
  getSkillSummary,
  formatRankingsColumn,
  type SkillDomain,
  type SkillTier as SkillTierType,
} from "@/lib/game/skillRankings";
import type { CatStats } from "@/types/game";

function makeStats(overrides: Partial<CatStats> = {}): CatStats {
  return {
    attack: 50,
    defense: 50,
    hunting: 50,
    medicine: 50,
    cleaning: 50,
    building: 50,
    leadership: 50,
    vision: 50,
    ...overrides,
  };
}

interface SimpleCat {
  name: string;
  stats: CatStats;
}

function makeCat(name: string, overrides: Partial<CatStats> = {}): SimpleCat {
  return { name, stats: makeStats(overrides) };
}

// ─── getSkillTier ───────────────────────────────────────────────

describe("getSkillTier", () => {
  it("returns null for stat below 20", () => {
    expect(getSkillTier(19)).toBeNull();
    expect(getSkillTier(0)).toBeNull();
  });

  it("returns novice for stat 20-39", () => {
    expect(getSkillTier(20)).toBe("novice");
    expect(getSkillTier(39)).toBe("novice");
  });

  it("returns apprentice for stat 40-59", () => {
    expect(getSkillTier(40)).toBe("apprentice");
    expect(getSkillTier(59)).toBe("apprentice");
  });

  it("returns skilled for stat 60-79", () => {
    expect(getSkillTier(60)).toBe("skilled");
    expect(getSkillTier(79)).toBe("skilled");
  });

  it("returns master for stat 80+", () => {
    expect(getSkillTier(80)).toBe("master");
    expect(getSkillTier(100)).toBe("master");
  });

  it("handles exact boundary values", () => {
    expect(getSkillTier(19)).toBeNull();
    expect(getSkillTier(20)).toBe("novice");
    expect(getSkillTier(39)).toBe("novice");
    expect(getSkillTier(40)).toBe("apprentice");
    expect(getSkillTier(59)).toBe("apprentice");
    expect(getSkillTier(60)).toBe("skilled");
    expect(getSkillTier(79)).toBe("skilled");
    expect(getSkillTier(80)).toBe("master");
  });
});

// ─── getSkillTitle ──────────────────────────────────────────────

describe("getSkillTitle", () => {
  it("returns null for stat below 20", () => {
    expect(getSkillTitle("hunting", 10)).toBeNull();
  });

  it("returns distinct titles for each domain at master tier", () => {
    const domains: SkillDomain[] = [
      "hunting",
      "combat",
      "medicine",
      "building",
      "leadership",
      "scouting",
    ];
    const titles = domains.map((d) => getSkillTitle(d, 90));
    // All non-null
    titles.forEach((t) => expect(t).not.toBeNull());
    // All distinct
    expect(new Set(titles).size).toBe(domains.length);
  });

  it("returns distinct titles for each tier within a domain", () => {
    const tiers = [25, 45, 65, 85]; // novice, apprentice, skilled, master
    const titles = tiers.map((v) => getSkillTitle("hunting", v));
    titles.forEach((t) => expect(t).not.toBeNull());
    expect(new Set(titles).size).toBe(tiers.length);
  });

  it("returns a string title for valid domain and tier", () => {
    const title = getSkillTitle("combat", 80);
    expect(typeof title).toBe("string");
    expect((title as string).length).toBeGreaterThan(0);
  });
});

// ─── rankCatsBySkill ────────────────────────────────────────────

describe("rankCatsBySkill", () => {
  it("returns empty array for no cats", () => {
    expect(rankCatsBySkill([], "hunting")).toEqual([]);
  });

  it("ranks a single cat as rank 1", () => {
    const cats = [makeCat("Whiskers", { hunting: 75 })];
    const ranked = rankCatsBySkill(cats, "hunting");
    expect(ranked).toHaveLength(1);
    expect(ranked[0].rank).toBe(1);
    expect(ranked[0].name).toBe("Whiskers");
    expect(ranked[0].statValue).toBe(75);
  });

  it("ranks multiple cats in descending stat order", () => {
    const cats = [
      makeCat("Low", { hunting: 30 }),
      makeCat("High", { hunting: 90 }),
      makeCat("Mid", { hunting: 60 }),
    ];
    const ranked = rankCatsBySkill(cats, "hunting");
    expect(ranked[0].name).toBe("High");
    expect(ranked[0].rank).toBe(1);
    expect(ranked[1].name).toBe("Mid");
    expect(ranked[1].rank).toBe(2);
    expect(ranked[2].name).toBe("Low");
    expect(ranked[2].rank).toBe(3);
  });

  it("handles ties with same rank", () => {
    const cats = [
      makeCat("A", { hunting: 70 }),
      makeCat("B", { hunting: 70 }),
      makeCat("C", { hunting: 50 }),
    ];
    const ranked = rankCatsBySkill(cats, "hunting");
    expect(ranked[0].rank).toBe(1);
    expect(ranked[1].rank).toBe(1);
    expect(ranked[2].rank).toBe(3);
  });

  it("uses combat as average of attack and defense", () => {
    const cats = [
      makeCat("Fighter", { attack: 80, defense: 60 }), // combat = 70
      makeCat("Tank", { attack: 40, defense: 90 }), // combat = 65
    ];
    const ranked = rankCatsBySkill(cats, "combat");
    expect(ranked[0].name).toBe("Fighter");
    expect(ranked[0].statValue).toBe(70);
    expect(ranked[1].name).toBe("Tank");
    expect(ranked[1].statValue).toBe(65);
  });

  it("uses vision stat for scouting domain", () => {
    const cats = [
      makeCat("Eagle", { vision: 95 }),
      makeCat("Mole", { vision: 20 }),
    ];
    const ranked = rankCatsBySkill(cats, "scouting");
    expect(ranked[0].name).toBe("Eagle");
    expect(ranked[0].statValue).toBe(95);
  });

  it("includes title for each ranked cat", () => {
    const cats = [makeCat("Pro", { hunting: 85 })];
    const ranked = rankCatsBySkill(cats, "hunting");
    expect(ranked[0].title).not.toBeNull();
  });
});

// ─── findColonyChampions ────────────────────────────────────────

describe("findColonyChampions", () => {
  it("returns all null champions for empty cat list", () => {
    const result = findColonyChampions([]);
    const domains: SkillDomain[] = [
      "hunting",
      "combat",
      "medicine",
      "building",
      "leadership",
      "scouting",
    ];
    domains.forEach((d) => {
      expect(result.champions[d]).toBeNull();
    });
    expect(result.mostVersatile).toBeNull();
  });

  it("identifies top cat per domain", () => {
    const cats = [
      makeCat("Hunter", { hunting: 90, medicine: 20 }),
      makeCat("Healer", { hunting: 20, medicine: 90 }),
    ];
    const result = findColonyChampions(cats);
    expect(result.champions.hunting?.name).toBe("Hunter");
    expect(result.champions.medicine?.name).toBe("Healer");
  });

  it("finds most versatile cat (highest average stat)", () => {
    const cats = [
      makeCat("Specialist", {
        hunting: 100,
        medicine: 10,
        building: 10,
        leadership: 10,
        vision: 10,
        attack: 10,
        defense: 10,
      }),
      makeCat("AllRounder", {
        hunting: 60,
        medicine: 60,
        building: 60,
        leadership: 60,
        vision: 60,
        attack: 60,
        defense: 60,
      }),
    ];
    const result = findColonyChampions(cats);
    expect(result.mostVersatile?.name).toBe("AllRounder");
  });

  it("champions have titles for stat >= 20", () => {
    const cats = [makeCat("Champ", { hunting: 85 })];
    const result = findColonyChampions(cats);
    expect(result.champions.hunting?.title).toBeDefined();
    expect(typeof result.champions.hunting?.title).toBe("string");
  });
});

// ─── generateRivalryReport ──────────────────────────────────────

describe("generateRivalryReport", () => {
  it("produces a string comparing two cats", () => {
    const cat1 = makeCat("Shadow", { hunting: 85 });
    const cat2 = makeCat("Storm", { hunting: 70 });
    const report = generateRivalryReport(cat1, cat2, "hunting");
    expect(typeof report).toBe("string");
    expect(report).toContain("Shadow");
    expect(report).toContain("Storm");
  });

  it("mentions the skill domain", () => {
    const cat1 = makeCat("A", { building: 60 });
    const cat2 = makeCat("B", { building: 55 });
    const report = generateRivalryReport(cat1, cat2, "building");
    expect(report.toLowerCase()).toContain("building");
  });

  it("indicates the winner or close contest", () => {
    const cat1 = makeCat("Strong", { hunting: 90 });
    const cat2 = makeCat("Weak", { hunting: 30 });
    const report = generateRivalryReport(cat1, cat2, "hunting");
    expect(report.length).toBeGreaterThan(20);
  });

  it("handles identical stats gracefully", () => {
    const cat1 = makeCat("Alpha", { medicine: 50 });
    const cat2 = makeCat("Beta", { medicine: 50 });
    const report = generateRivalryReport(cat1, cat2, "medicine");
    expect(typeof report).toBe("string");
    expect(report.length).toBeGreaterThan(0);
  });
});

// ─── getSkillSummary ────────────────────────────────────────────

describe("getSkillSummary", () => {
  it("identifies best skill domain", () => {
    const cat = makeCat("Ace", { hunting: 90, building: 30 });
    const summary = getSkillSummary(cat);
    expect(summary.bestDomain).toBe("hunting");
    expect(summary.bestValue).toBe(90);
  });

  it("returns title for best domain", () => {
    const cat = makeCat("Master", { medicine: 85 });
    const summary = getSkillSummary(cat);
    expect(summary.title).not.toBeNull();
  });

  it("calculates overall rating as average of all stats", () => {
    const cat = makeCat("Even", {
      attack: 60,
      defense: 60,
      hunting: 60,
      medicine: 60,
      cleaning: 60,
      building: 60,
      leadership: 60,
      vision: 60,
    });
    const summary = getSkillSummary(cat);
    expect(summary.overallRating).toBe(60);
  });

  it("returns null title when best stat is below 20", () => {
    const cat = makeCat("Weak", {
      attack: 10,
      defense: 10,
      hunting: 10,
      medicine: 10,
      cleaning: 10,
      building: 10,
      leadership: 10,
      vision: 10,
    });
    const summary = getSkillSummary(cat);
    expect(summary.title).toBeNull();
  });

  it("handles combat domain correctly when attack+defense average is highest", () => {
    const cat = makeCat("Fighter", {
      attack: 90,
      defense: 90,
      hunting: 40,
      medicine: 40,
      cleaning: 40,
      building: 40,
      leadership: 40,
      vision: 40,
    });
    const summary = getSkillSummary(cat);
    expect(summary.bestDomain).toBe("combat");
    expect(summary.bestValue).toBe(90);
  });
});

// ─── formatRankingsColumn ───────────────────────────────────────

describe("formatRankingsColumn", () => {
  it("returns empty column for no cats", () => {
    const column = formatRankingsColumn([]);
    expect(column.catCount).toBe(0);
    expect(column.champions).toEqual([]);
    expect(column.mostVersatile).toBeNull();
    expect(column.rivalries).toEqual([]);
    expect(typeof column.headline).toBe("string");
  });

  it("populates champions from colony", () => {
    const cats = [
      makeCat("Hunter", { hunting: 85 }),
      makeCat("Builder", { building: 90 }),
    ];
    const column = formatRankingsColumn(cats);
    expect(column.champions.length).toBeGreaterThan(0);
    column.champions.forEach((c) => {
      expect(c.domain).toBeDefined();
      expect(c.name).toBeDefined();
      expect(c.title).toBeDefined();
      expect(c.value).toBeGreaterThan(0);
    });
  });

  it("generates rivalries when multiple cats exist", () => {
    const cats = [
      makeCat("Alpha", { hunting: 85, building: 30 }),
      makeCat("Beta", { hunting: 80, building: 90 }),
      makeCat("Gamma", { hunting: 40, building: 40 }),
    ];
    const column = formatRankingsColumn(cats);
    expect(column.rivalries.length).toBeGreaterThan(0);
    column.rivalries.forEach((r) => expect(typeof r).toBe("string"));
  });

  it("includes most versatile cat when cats exist", () => {
    const cats = [makeCat("Solo", { hunting: 60, building: 60, medicine: 60 })];
    const column = formatRankingsColumn(cats);
    expect(column.mostVersatile).not.toBeNull();
    expect(column.mostVersatile?.name).toBe("Solo");
  });

  it("includes catCount", () => {
    const cats = [makeCat("A"), makeCat("B"), makeCat("C")];
    const column = formatRankingsColumn(cats);
    expect(column.catCount).toBe(3);
  });

  it("generates a headline string", () => {
    const cats = [makeCat("Star", { hunting: 95 })];
    const column = formatRankingsColumn(cats);
    expect(typeof column.headline).toBe("string");
    expect(column.headline.length).toBeGreaterThan(0);
  });

  it("handles single cat colony", () => {
    const cats = [makeCat("Lonely")];
    const column = formatRankingsColumn(cats);
    expect(column.catCount).toBe(1);
    expect(column.champions.length).toBeGreaterThan(0);
  });
});
