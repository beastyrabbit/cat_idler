import { describe, it, expect } from "vitest";
import {
  getGenerationDepth,
  classifyLineageDepth,
  findCommonAncestors,
  calculateInbreedingCoefficient,
  assessGeneticDiversity,
  generateFamilyTreeColumn,
} from "@/lib/game/ancestry";
import type {
  CatLineageEntry,
  LineageDepthTier,
  GeneticDiversityReport,
} from "@/lib/game/ancestry";

// ---------------------------------------------------------------------------
// Test data helpers
// ---------------------------------------------------------------------------

function makeParentMap(
  entries: Array<{ id: string; parents: string[] }>,
): Map<string, string[]> {
  const map = new Map<string, string[]>();
  for (const e of entries) {
    map.set(e.id, e.parents);
  }
  return map;
}

// ---------------------------------------------------------------------------
// getGenerationDepth
// ---------------------------------------------------------------------------

describe("getGenerationDepth", () => {
  it("returns 0 for a founder (no parents)", () => {
    const pm = makeParentMap([{ id: "A", parents: [] }]);
    expect(getGenerationDepth("A", pm)).toBe(0);
  });

  it("returns 1 for a cat with founder parents", () => {
    const pm = makeParentMap([
      { id: "A", parents: [] },
      { id: "B", parents: [] },
      { id: "C", parents: ["A", "B"] },
    ]);
    expect(getGenerationDepth("C", pm)).toBe(1);
  });

  it("walks multi-generation chains correctly", () => {
    const pm = makeParentMap([
      { id: "A", parents: [] },
      { id: "B", parents: [] },
      { id: "C", parents: ["A", "B"] },
      { id: "D", parents: ["C"] },
      { id: "E", parents: ["D"] },
    ]);
    expect(getGenerationDepth("E", pm)).toBe(3);
  });

  it("handles single-parent cats", () => {
    const pm = makeParentMap([
      { id: "A", parents: [] },
      { id: "B", parents: ["A"] },
    ]);
    expect(getGenerationDepth("B", pm)).toBe(1);
  });

  it("returns 0 for unknown cat (not in map)", () => {
    const pm = makeParentMap([]);
    expect(getGenerationDepth("unknown", pm)).toBe(0);
  });

  it("takes the max depth when parents are at different generations", () => {
    const pm = makeParentMap([
      { id: "A", parents: [] },
      { id: "B", parents: [] },
      { id: "C", parents: ["A", "B"] },
      // D has one founder parent and one gen-1 parent
      { id: "D", parents: ["A", "C"] },
    ]);
    expect(getGenerationDepth("D", pm)).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// classifyLineageDepth
// ---------------------------------------------------------------------------

describe("classifyLineageDepth", () => {
  it('returns "founder" for generation 0', () => {
    expect(classifyLineageDepth(0)).toBe("founder");
  });

  it('returns "first_gen" for generation 1', () => {
    expect(classifyLineageDepth(1)).toBe("first_gen");
  });

  it('returns "established" for generation 2', () => {
    expect(classifyLineageDepth(2)).toBe("established");
  });

  it('returns "established" for generation 4', () => {
    expect(classifyLineageDepth(4)).toBe("established");
  });

  it('returns "deep_roots" for generation 5', () => {
    expect(classifyLineageDepth(5)).toBe("deep_roots");
  });

  it('returns "deep_roots" for generation 10', () => {
    expect(classifyLineageDepth(10)).toBe("deep_roots");
  });
});

// ---------------------------------------------------------------------------
// findCommonAncestors
// ---------------------------------------------------------------------------

describe("findCommonAncestors", () => {
  it("returns empty array for unrelated cats", () => {
    const pm = makeParentMap([
      { id: "A", parents: [] },
      { id: "B", parents: [] },
    ]);
    expect(findCommonAncestors("A", "B", pm)).toEqual([]);
  });

  it("finds shared parents (siblings)", () => {
    const pm = makeParentMap([
      { id: "A", parents: [] },
      { id: "B", parents: [] },
      { id: "C", parents: ["A", "B"] },
      { id: "D", parents: ["A", "B"] },
    ]);
    const common = findCommonAncestors("C", "D", pm);
    expect(common.sort()).toEqual(["A", "B"]);
  });

  it("finds shared grandparents (cousins)", () => {
    const pm = makeParentMap([
      { id: "G1", parents: [] },
      { id: "G2", parents: [] },
      { id: "P1", parents: ["G1", "G2"] },
      { id: "P2", parents: ["G1", "G2"] },
      { id: "C1", parents: ["P1"] },
      { id: "C2", parents: ["P2"] },
    ]);
    const common = findCommonAncestors("C1", "C2", pm);
    expect(common.sort()).toEqual(["G1", "G2"]);
  });

  it("handles one cat being ancestor of the other", () => {
    const pm = makeParentMap([
      { id: "A", parents: [] },
      { id: "B", parents: ["A"] },
    ]);
    const common = findCommonAncestors("A", "B", pm);
    expect(common).toEqual(["A"]);
  });
});

// ---------------------------------------------------------------------------
// calculateInbreedingCoefficient
// ---------------------------------------------------------------------------

describe("calculateInbreedingCoefficient", () => {
  it("returns 0 for no common ancestors", () => {
    expect(calculateInbreedingCoefficient(0, 2, 2)).toBe(0);
  });

  it("returns positive value for common ancestors", () => {
    const coi = calculateInbreedingCoefficient(2, 1, 1);
    expect(coi).toBeGreaterThan(0);
    expect(coi).toBeLessThanOrEqual(1);
  });

  it("increases with more common ancestors", () => {
    const coi1 = calculateInbreedingCoefficient(1, 2, 2);
    const coi2 = calculateInbreedingCoefficient(3, 2, 2);
    expect(coi2).toBeGreaterThan(coi1);
  });

  it("caps at 1.0", () => {
    const coi = calculateInbreedingCoefficient(100, 1, 1);
    expect(coi).toBeLessThanOrEqual(1);
  });
});

// ---------------------------------------------------------------------------
// assessGeneticDiversity
// ---------------------------------------------------------------------------

describe("assessGeneticDiversity", () => {
  it("handles empty input", () => {
    const report = assessGeneticDiversity([]);
    expect(report.totalCats).toBe(0);
    expect(report.averageGeneration).toBe(0);
    expect(report.founderCount).toBe(0);
    expect(report.inbreedingRisk).toBe("none");
  });

  it("counts founders correctly", () => {
    const entries: CatLineageEntry[] = [
      { catId: "A", catName: "Whiskers", parentIds: [], generation: 0 },
      { catId: "B", catName: "Mittens", parentIds: [], generation: 0 },
      { catId: "C", catName: "Shadow", parentIds: ["A", "B"], generation: 1 },
    ];
    const report = assessGeneticDiversity(entries);
    expect(report.totalCats).toBe(3);
    expect(report.founderCount).toBe(2);
  });

  it("calculates average and max generation", () => {
    const entries: CatLineageEntry[] = [
      { catId: "A", catName: "Alpha", parentIds: [], generation: 0 },
      { catId: "B", catName: "Beta", parentIds: [], generation: 0 },
      { catId: "C", catName: "Gamma", parentIds: ["A", "B"], generation: 1 },
      { catId: "D", catName: "Delta", parentIds: ["C"], generation: 2 },
    ];
    const report = assessGeneticDiversity(entries);
    expect(report.maxGeneration).toBe(2);
    expect(report.averageGeneration).toBe(0.75); // (0+0+1+2)/4
  });

  it("counts unique lineages from founders", () => {
    const entries: CatLineageEntry[] = [
      { catId: "A", catName: "Alpha", parentIds: [], generation: 0 },
      { catId: "B", catName: "Beta", parentIds: [], generation: 0 },
      { catId: "C", catName: "Gamma", parentIds: [], generation: 0 },
    ];
    const report = assessGeneticDiversity(entries);
    expect(report.uniqueLineages).toBe(3);
  });

  it('classifies inbreeding risk as "high" for very few founders with many cats', () => {
    const entries: CatLineageEntry[] = [
      { catId: "A", catName: "Alpha", parentIds: [], generation: 0 },
      // Many descendants from a single founder
      { catId: "B", catName: "Beta", parentIds: ["A"], generation: 1 },
      { catId: "C", catName: "Gamma", parentIds: ["A"], generation: 1 },
      { catId: "D", catName: "Delta", parentIds: ["B", "C"], generation: 2 },
      { catId: "E", catName: "Epsilon", parentIds: ["B", "C"], generation: 2 },
      { catId: "F", catName: "Zeta", parentIds: ["D", "E"], generation: 3 },
    ];
    const report = assessGeneticDiversity(entries);
    expect(report.inbreedingRisk).toBe("high");
  });

  it("identifies deepest lineage tier", () => {
    const entries: CatLineageEntry[] = [
      { catId: "A", catName: "Alpha", parentIds: [], generation: 0 },
      { catId: "B", catName: "Beta", parentIds: ["A"], generation: 1 },
      { catId: "C", catName: "Gamma", parentIds: ["B"], generation: 2 },
      { catId: "D", catName: "Delta", parentIds: ["C"], generation: 3 },
      { catId: "E", catName: "Epsilon", parentIds: ["D"], generation: 4 },
      { catId: "F", catName: "Zeta", parentIds: ["E"], generation: 5 },
    ];
    const report = assessGeneticDiversity(entries);
    expect(report.deepestLineageTier).toBe("deep_roots");
  });
});

// ---------------------------------------------------------------------------
// generateFamilyTreeColumn
// ---------------------------------------------------------------------------

describe("generateFamilyTreeColumn", () => {
  it("includes colony name", () => {
    const report: GeneticDiversityReport = {
      totalCats: 5,
      averageGeneration: 1.2,
      maxGeneration: 3,
      founderCount: 2,
      uniqueLineages: 2,
      inbreedingRisk: "none",
      deepestLineageTier: "established",
    };
    const column = generateFamilyTreeColumn(report, "Whiskerton");
    expect(column).toContain("Whiskerton");
  });

  it("shows inbreeding risk when elevated", () => {
    const report: GeneticDiversityReport = {
      totalCats: 10,
      averageGeneration: 2.5,
      maxGeneration: 5,
      founderCount: 2,
      uniqueLineages: 2,
      inbreedingRisk: "high",
      deepestLineageTier: "deep_roots",
    };
    const column = generateFamilyTreeColumn(report, "TestColony");
    expect(column.toLowerCase()).toContain("inbreeding");
  });

  it("handles all-founder colony", () => {
    const report: GeneticDiversityReport = {
      totalCats: 4,
      averageGeneration: 0,
      maxGeneration: 0,
      founderCount: 4,
      uniqueLineages: 4,
      inbreedingRisk: "none",
      deepestLineageTier: "founder",
    };
    const column = generateFamilyTreeColumn(report, "NewColony");
    expect(column).toContain("founder");
  });

  it("highlights deep-rooted lineages", () => {
    const report: GeneticDiversityReport = {
      totalCats: 20,
      averageGeneration: 3.5,
      maxGeneration: 6,
      founderCount: 5,
      uniqueLineages: 5,
      inbreedingRisk: "low",
      deepestLineageTier: "deep_roots",
    };
    const column = generateFamilyTreeColumn(report, "AncientColony");
    expect(column).toContain("deep");
  });
});

// ---------------------------------------------------------------------------
// Purity check
// ---------------------------------------------------------------------------

describe("purity", () => {
  it("all functions are pure (no side effects)", () => {
    const pm = makeParentMap([
      { id: "A", parents: [] },
      { id: "B", parents: ["A"] },
    ]);

    // Call multiple times, same result
    const depth1 = getGenerationDepth("B", pm);
    const depth2 = getGenerationDepth("B", pm);
    expect(depth1).toBe(depth2);

    const tier1 = classifyLineageDepth(3);
    const tier2 = classifyLineageDepth(3);
    expect(tier1).toBe(tier2);

    const coi1 = calculateInbreedingCoefficient(2, 1, 1);
    const coi2 = calculateInbreedingCoefficient(2, 1, 1);
    expect(coi1).toBe(coi2);
  });
});
