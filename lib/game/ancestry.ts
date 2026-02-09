/**
 * Cat Ancestry System
 *
 * Pure functions for tracing cat lineage — generation depth,
 * common ancestor detection, inbreeding coefficient, genetic
 * diversity assessment, and newspaper "Family Trees" column.
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type LineageDepthTier =
  | "founder"
  | "first_gen"
  | "established"
  | "deep_roots";

export interface CatLineageEntry {
  catId: string;
  catName: string;
  parentIds: string[];
  generation: number;
}

export interface GeneticDiversityReport {
  totalCats: number;
  averageGeneration: number;
  maxGeneration: number;
  founderCount: number;
  uniqueLineages: number;
  inbreedingRisk: "none" | "low" | "moderate" | "high";
  deepestLineageTier: LineageDepthTier;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const LINEAGE_THRESHOLDS = {
  FIRST_GEN: 1,
  ESTABLISHED_MIN: 2,
  ESTABLISHED_MAX: 4,
  DEEP_ROOTS: 5,
} as const;

// Inbreeding risk thresholds: ratio of founders to total cats
const INBREEDING_RISK_THRESHOLDS = {
  HIGH: 0.2, // fewer than 20% founders → high risk
  MODERATE: 0.3,
  LOW: 0.4,
} as const;

// ---------------------------------------------------------------------------
// getGenerationDepth
// ---------------------------------------------------------------------------

/**
 * Walk up the parent chain to count generation depth.
 * Founders (no parents) are generation 0. A cat's generation is
 * max(parent generations) + 1. Unknown cats default to 0.
 */
export function getGenerationDepth(
  catId: string,
  parentMap: Map<string, string[]>,
  visited: Set<string> = new Set(),
): number {
  if (visited.has(catId)) return 0; // cycle protection
  visited.add(catId);

  const parents = parentMap.get(catId);
  if (!parents || parents.length === 0) return 0;

  let maxParentDepth = 0;
  for (const parentId of parents) {
    const depth = getGenerationDepth(parentId, parentMap, visited);
    if (depth > maxParentDepth) maxParentDepth = depth;
  }

  return maxParentDepth + 1;
}

// ---------------------------------------------------------------------------
// classifyLineageDepth
// ---------------------------------------------------------------------------

/**
 * Classify a generation number into a lineage depth tier.
 */
export function classifyLineageDepth(generation: number): LineageDepthTier {
  if (generation === 0) return "founder";
  if (generation === LINEAGE_THRESHOLDS.FIRST_GEN) return "first_gen";
  if (
    generation >= LINEAGE_THRESHOLDS.ESTABLISHED_MIN &&
    generation <= LINEAGE_THRESHOLDS.ESTABLISHED_MAX
  )
    return "established";
  return "deep_roots";
}

// ---------------------------------------------------------------------------
// findCommonAncestors
// ---------------------------------------------------------------------------

/**
 * Collect all ancestors of a cat (including itself) by walking up the parent map.
 */
function collectAncestors(
  catId: string,
  parentMap: Map<string, string[]>,
  ancestors: Set<string> = new Set(),
): Set<string> {
  if (ancestors.has(catId)) return ancestors;
  ancestors.add(catId);

  const parents = parentMap.get(catId);
  if (parents) {
    for (const parentId of parents) {
      collectAncestors(parentId, parentMap, ancestors);
    }
  }
  return ancestors;
}

/**
 * Find shared ancestors between two cats.
 * Returns the IDs of all common ancestors (excluding the two cats themselves
 * unless one is the ancestor of the other).
 */
export function findCommonAncestors(
  catId1: string,
  catId2: string,
  parentMap: Map<string, string[]>,
): string[] {
  const ancestors1 = collectAncestors(catId1, parentMap);
  const ancestors2 = collectAncestors(catId2, parentMap);

  const common: string[] = [];
  for (const id of ancestors1) {
    if (ancestors2.has(id) && id !== catId1 && id !== catId2) {
      common.push(id);
    }
  }

  // Special case: if one cat IS an ancestor of the other
  if (ancestors1.has(catId2) && catId1 !== catId2) {
    // catId2 is an ancestor of catId1
    if (!common.includes(catId2)) common.push(catId2);
  }
  if (ancestors2.has(catId1) && catId1 !== catId2) {
    // catId1 is an ancestor of catId2
    if (!common.includes(catId1)) common.push(catId1);
  }

  return common;
}

// ---------------------------------------------------------------------------
// calculateInbreedingCoefficient
// ---------------------------------------------------------------------------

/**
 * Calculate a simplified inbreeding coefficient.
 * Based on the number of common ancestors and the generation distance.
 *
 * Formula: COI = min(1, commonAncestorCount × 0.5^((gen1 + gen2) / 2))
 * Returns 0–1 where 0 = unrelated, 1 = maximally inbred.
 */
export function calculateInbreedingCoefficient(
  commonAncestorCount: number,
  generation1: number,
  generation2: number,
): number {
  if (commonAncestorCount === 0) return 0;

  const avgGeneration = (generation1 + generation2) / 2;
  const coi = commonAncestorCount * Math.pow(0.5, avgGeneration);

  return Math.min(1, coi);
}

// ---------------------------------------------------------------------------
// assessGeneticDiversity
// ---------------------------------------------------------------------------

/**
 * Assess colony-wide genetic diversity from lineage entries.
 */
export function assessGeneticDiversity(
  catEntries: CatLineageEntry[],
): GeneticDiversityReport {
  if (catEntries.length === 0) {
    return {
      totalCats: 0,
      averageGeneration: 0,
      maxGeneration: 0,
      founderCount: 0,
      uniqueLineages: 0,
      inbreedingRisk: "none",
      deepestLineageTier: "founder",
    };
  }

  const totalCats = catEntries.length;
  const founderCount = catEntries.filter((e) => e.generation === 0).length;
  const generations = catEntries.map((e) => e.generation);
  const maxGeneration = Math.max(...generations);
  const averageGeneration =
    generations.reduce((sum, g) => sum + g, 0) / totalCats;

  // Unique lineages = number of distinct founders
  const uniqueLineages = founderCount > 0 ? founderCount : 0;

  // Inbreeding risk based on founder-to-total ratio
  const founderRatio = totalCats > 0 ? founderCount / totalCats : 1;
  let inbreedingRisk: "none" | "low" | "moderate" | "high";
  if (founderRatio >= INBREEDING_RISK_THRESHOLDS.LOW || maxGeneration === 0) {
    inbreedingRisk = "none";
  } else if (founderRatio >= INBREEDING_RISK_THRESHOLDS.MODERATE) {
    inbreedingRisk = "low";
  } else if (founderRatio >= INBREEDING_RISK_THRESHOLDS.HIGH) {
    inbreedingRisk = "moderate";
  } else {
    inbreedingRisk = "high";
  }

  const deepestLineageTier = classifyLineageDepth(maxGeneration);

  return {
    totalCats,
    averageGeneration,
    maxGeneration,
    founderCount,
    uniqueLineages,
    inbreedingRisk,
    deepestLineageTier,
  };
}

// ---------------------------------------------------------------------------
// generateFamilyTreeColumn
// ---------------------------------------------------------------------------

/**
 * Generate a newspaper "Family Trees" column from a genetic diversity report.
 */
export function generateFamilyTreeColumn(
  report: GeneticDiversityReport,
  colonyName: string,
): string {
  const lines: string[] = [];

  lines.push(`FAMILY TREES — ${colonyName}`);
  lines.push("═".repeat(40));
  lines.push("");

  if (report.totalCats === 0) {
    lines.push("No cats to report on.");
    return lines.join("\n");
  }

  // All-founder colony
  if (report.deepestLineageTier === "founder") {
    lines.push(
      `All ${report.totalCats} cats are founder stock. The colony's family trees have yet to branch!`,
    );
    lines.push("");
    lines.push(
      `With ${report.uniqueLineages} unique founder lineages, the genetic pool is fresh and diverse.`,
    );
    return lines.join("\n");
  }

  // General stats
  lines.push(
    `Population: ${report.totalCats} cats across ${report.maxGeneration + 1} generations`,
  );
  lines.push(
    `Founded by ${report.founderCount} original cats (${report.uniqueLineages} unique lineages)`,
  );
  lines.push(
    `Average generation depth: ${report.averageGeneration.toFixed(1)}`,
  );
  lines.push("");

  // Deep roots highlight
  if (report.deepestLineageTier === "deep_roots") {
    lines.push(
      `[DEEP ROOTS] Some family lines stretch back ${report.maxGeneration} generations — truly deep-rooted dynasties!`,
    );
    lines.push("");
  }

  // Inbreeding risk
  if (report.inbreedingRisk !== "none") {
    const riskLabels = {
      low: "Low inbreeding risk detected — genetic diversity is adequate but could improve.",
      moderate:
        "Moderate inbreeding risk — consider welcoming wandering cats to broaden the gene pool.",
      high: "HIGH INBREEDING RISK — the colony desperately needs fresh bloodlines! Many cats share common ancestors.",
    };
    lines.push(`[WARNING] ${riskLabels[report.inbreedingRisk]}`);
    lines.push("");
  }

  return lines.join("\n");
}
