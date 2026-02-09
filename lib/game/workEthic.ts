export type ProductivityTier =
  | "slacker"
  | "lazy"
  | "average"
  | "diligent"
  | "workaholic";

export interface WorkerCat {
  name: string;
  workEthic: number;
  restLevel: number;
  efficiency: number;
}

export interface ProductivityReport {
  totalWorkers: number;
  averageEfficiency: number;
  slackerCount: number;
  overworkedCount: number;
  topWorker: string | null;
  dominantTier: ProductivityTier | null;
}

type LifeStage = "kitten" | "young" | "adult" | "elder";

const AGE_MODIFIERS: Record<LifeStage, number> = {
  kitten: 0.5,
  young: 0.8,
  adult: 1.0,
  elder: 0.7,
};

export function calculateWorkEthic(
  skills: { hunting: number; combat: number },
  restLevel: number,
  hunger: number,
  thirst: number,
  lifeStage: LifeStage,
): number {
  const skillAvg = (skills.hunting + skills.combat) / 2;

  let restBonus = 0;
  if (restLevel > 70) restBonus = 20;
  else if (restLevel >= 40) restBonus = 10;

  const satisfactionPenalty = hunger > 70 || thirst > 70 ? -20 : 0;

  const ageMod = AGE_MODIFIERS[lifeStage];

  const raw = (skillAvg + restBonus + satisfactionPenalty) * ageMod;
  return Math.round(Math.max(0, Math.min(100, raw)));
}

export function getEfficiencyMultiplier(workEthic: number): number {
  const clamped = Math.max(0, Math.min(100, workEthic));
  if (clamped <= 20) return 0.5;
  if (clamped <= 40) return 0.75;
  if (clamped <= 60) return 1.0;
  if (clamped <= 80) return 1.25;
  return 1.5;
}

export function getProductivityTier(workEthic: number): ProductivityTier {
  const clamped = Math.max(0, Math.min(100, workEthic));
  if (clamped <= 20) return "slacker";
  if (clamped <= 40) return "lazy";
  if (clamped <= 60) return "average";
  if (clamped <= 80) return "diligent";
  return "workaholic";
}

export function detectSlacker(workEthic: number, restLevel: number): boolean {
  return workEthic < 30 && restLevel > 70;
}

export function detectOverworked(
  workEthic: number,
  restLevel: number,
): boolean {
  return workEthic > 70 && restLevel < 30;
}

export function evaluateColonyProductivity(
  cats: readonly WorkerCat[],
): ProductivityReport {
  if (cats.length === 0) {
    return {
      totalWorkers: 0,
      averageEfficiency: 0,
      slackerCount: 0,
      overworkedCount: 0,
      topWorker: null,
      dominantTier: null,
    };
  }

  const totalWorkers = cats.length;
  const averageEfficiency =
    cats.reduce((sum, c) => sum + c.efficiency, 0) / totalWorkers;

  const slackerCount = cats.filter((c) =>
    detectSlacker(c.workEthic, c.restLevel),
  ).length;
  const overworkedCount = cats.filter((c) =>
    detectOverworked(c.workEthic, c.restLevel),
  ).length;

  let topWorker: string | null = null;
  let maxEthic = -1;
  for (const cat of cats) {
    if (cat.workEthic > maxEthic) {
      maxEthic = cat.workEthic;
      topWorker = cat.name;
    }
  }

  const tierCounts = new Map<ProductivityTier, number>();
  for (const cat of cats) {
    const tier = getProductivityTier(cat.workEthic);
    tierCounts.set(tier, (tierCounts.get(tier) ?? 0) + 1);
  }
  let dominantTier: ProductivityTier | null = null;
  let maxCount = 0;
  for (const [tier, count] of tierCounts) {
    if (count > maxCount) {
      maxCount = count;
      dominantTier = tier;
    }
  }

  return {
    totalWorkers,
    averageEfficiency: Math.round(averageEfficiency * 100) / 100,
    slackerCount,
    overworkedCount,
    topWorker,
    dominantTier,
  };
}

export function generateWorkplaceColumn(
  report: ProductivityReport,
  colonyName: string,
): string {
  if (report.totalWorkers === 0) {
    return `WORKPLACE REPORT — ${colonyName}\n\nA ghost town. No workers roam the colony grounds today. The workshops sit empty and quiet.`;
  }

  const tierLabel = report.dominantTier
    ? report.dominantTier.charAt(0).toUpperCase() + report.dominantTier.slice(1)
    : "Mixed";

  const lines: string[] = [
    `WORKPLACE REPORT — ${colonyName}`,
    "",
    `The colony's ${report.totalWorkers} workers report for duty with an average efficiency of ${report.averageEfficiency}x. The dominant work culture is "${tierLabel}."`,
  ];

  if (report.topWorker) {
    lines.push(
      `\nEmployee of the Day: ${report.topWorker} — leading by example with tireless dedication.`,
    );
  }

  if (report.slackerCount > 0) {
    lines.push(
      `\nManagement Concern: ${report.slackerCount} well-rested cat${report.slackerCount > 1 ? "s" : ""} spotted napping on the job despite having plenty of energy.`,
    );
  }

  if (report.overworkedCount > 0) {
    lines.push(
      `\nWelfare Alert: ${report.overworkedCount} dedicated worker${report.overworkedCount > 1 ? "s" : ""} running on fumes. Mandatory rest recommended.`,
    );
  }

  return lines.join("\n");
}
