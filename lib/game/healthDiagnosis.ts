/**
 * Cat Health Diagnosis System
 *
 * Pure functions for veterinary health diagnosis.
 * Examines cat needs to detect medical conditions, assign severity,
 * recommend treatment, and generate colony health reports.
 */

// --- Types ---

export type ConditionType =
  | "malnutrition"
  | "dehydration"
  | "exhaustion"
  | "injury";

export type Severity = "mild" | "severe" | "critical";

export interface Diagnosis {
  condition: ConditionType;
  severity: Severity;
  value: number;
}

export interface CatDiagnosisEntry {
  catName: string;
  diagnoses: Diagnosis[];
  isHealthy: boolean;
}

export interface ColonyHealthReport {
  totalCats: number;
  healthyCats: number;
  healthyPercent: number;
  conditionCounts: Record<ConditionType, number>;
  severeCases: number;
  criticalCases: number;
  healthGrade: string;
  mostCommonCondition: ConditionType | null;
}

// --- Thresholds ---

const MALNUTRITION_MILD = 30;
const MALNUTRITION_SEVERE = 15;

const DEHYDRATION_MILD = 30;
const DEHYDRATION_SEVERE = 15;

const EXHAUSTION_MILD = 25;
const EXHAUSTION_SEVERE = 10;

const INJURY_MILD = 50;
const INJURY_SEVERE = 25;

// --- Functions ---

/**
 * Classify severity from a stat value against thresholds.
 * Returns null if value is at or above the mild threshold.
 */
export function getConditionSeverity(
  value: number,
  mildThreshold: number,
  severeThreshold: number,
): Severity | null {
  if (value >= mildThreshold) return null;
  if (value < severeThreshold) return "severe";
  return "mild";
}

interface CatNeeds {
  hunger: number;
  thirst: number;
  rest: number;
  health: number;
}

/**
 * Detect all active medical conditions from cat needs.
 */
export function diagnoseCat(needs: CatNeeds): Diagnosis[] {
  const diagnoses: Diagnosis[] = [];

  const malnutrition = getConditionSeverity(
    needs.hunger,
    MALNUTRITION_MILD,
    MALNUTRITION_SEVERE,
  );
  if (malnutrition) {
    diagnoses.push({
      condition: "malnutrition",
      severity: malnutrition,
      value: needs.hunger,
    });
  }

  const dehydration = getConditionSeverity(
    needs.thirst,
    DEHYDRATION_MILD,
    DEHYDRATION_SEVERE,
  );
  if (dehydration) {
    diagnoses.push({
      condition: "dehydration",
      severity: dehydration,
      value: needs.thirst,
    });
  }

  const exhaustion = getConditionSeverity(
    needs.rest,
    EXHAUSTION_MILD,
    EXHAUSTION_SEVERE,
  );
  if (exhaustion) {
    diagnoses.push({
      condition: "exhaustion",
      severity: exhaustion,
      value: needs.rest,
    });
  }

  const injury = getConditionSeverity(needs.health, INJURY_MILD, INJURY_SEVERE);
  if (injury) {
    diagnoses.push({
      condition: "injury",
      severity: injury === "severe" ? "critical" : injury,
      value: needs.health,
    });
  }

  return diagnoses;
}

/**
 * Treatment recommendation for a diagnosed condition.
 */
export function recommendTreatment(diagnosis: Diagnosis): string {
  const treatments: Record<ConditionType, Record<Severity, string>> = {
    malnutrition: {
      mild: "Increase food rations and monitor weight",
      severe: "Immediate emergency food supply required — urgent intervention",
      critical:
        "Emergency feeding protocol — immediate life-saving food supply",
    },
    dehydration: {
      mild: "Ensure fresh water access and monitor intake",
      severe: "Immediate water supply required — urgent hydration needed",
      critical: "Emergency hydration protocol — immediate water supply",
    },
    exhaustion: {
      mild: "Schedule rest periods and reduce workload",
      severe: "Immediate rest required — urgent removal from all duties",
      critical: "Emergency rest protocol — immediate bed rest required",
    },
    injury: {
      mild: "Treat wounds and monitor recovery",
      severe: "Immediate wound treatment required — urgent medical care",
      critical: "Emergency surgery — immediate life-saving treatment needed",
    },
  };

  return treatments[diagnosis.condition][diagnosis.severity];
}

/**
 * Letter grade (A-F) for colony health percentage.
 */
export function getHealthGrade(healthyPercent: number): string {
  if (healthyPercent >= 90) return "A";
  if (healthyPercent >= 75) return "B";
  if (healthyPercent >= 50) return "C";
  if (healthyPercent >= 25) return "D";
  return "F";
}

const ALL_CONDITIONS: ConditionType[] = [
  "malnutrition",
  "dehydration",
  "exhaustion",
  "injury",
];

/**
 * Aggregate colony-wide health statistics from individual diagnoses.
 */
export function assessColonyHealth(
  catDiagnoses: CatDiagnosisEntry[],
): ColonyHealthReport {
  const totalCats = catDiagnoses.length;
  const healthyCats = catDiagnoses.filter((e) => e.isHealthy).length;
  const healthyPercent =
    totalCats === 0 ? 100 : (healthyCats / totalCats) * 100;

  const conditionCounts: Record<ConditionType, number> = {
    malnutrition: 0,
    dehydration: 0,
    exhaustion: 0,
    injury: 0,
  };

  let severeCases = 0;
  let criticalCases = 0;

  for (const entry of catDiagnoses) {
    for (const d of entry.diagnoses) {
      conditionCounts[d.condition]++;
      if (d.severity === "severe") severeCases++;
      if (d.severity === "critical") criticalCases++;
    }
  }

  let mostCommonCondition: ConditionType | null = null;
  let maxCount = 0;
  for (const condition of ALL_CONDITIONS) {
    if (conditionCounts[condition] > maxCount) {
      maxCount = conditionCounts[condition];
      mostCommonCondition = condition;
    }
  }

  const healthGrade = getHealthGrade(healthyPercent);

  return {
    totalCats,
    healthyCats,
    healthyPercent,
    conditionCounts,
    severeCases,
    criticalCases,
    healthGrade,
    mostCommonCondition,
  };
}

/**
 * Generate a newspaper "Medical Bulletin" column.
 */
export function generateMedicalBulletin(
  report: ColonyHealthReport,
  colonyName: string,
): string {
  const lines: string[] = [];

  lines.push(`=== MEDICAL BULLETIN — ${colonyName} ===`);
  lines.push("");
  lines.push(`Colony Health Grade: ${report.healthGrade}`);

  if (report.totalCats === 0) {
    lines.push("No cats to examine. The clinic is empty.");
    return lines.join("\n");
  }

  lines.push(
    `Examined: ${report.totalCats} cats — ${report.healthyCats} healthy (${Math.round(report.healthyPercent)}%)`,
  );

  if (report.criticalCases > 0) {
    lines.push("");
    lines.push(
      `⚠ CRITICAL ALERT: ${report.criticalCases} cat(s) in critical condition requiring emergency care!`,
    );
  }

  if (report.severeCases > 0) {
    lines.push(
      `Severe cases: ${report.severeCases} cat(s) need urgent attention.`,
    );
  }

  if (report.healthyCats === report.totalCats) {
    lines.push("");
    lines.push(
      "Excellent health across the colony! All cats received a clean bill of health.",
    );
  } else if (report.mostCommonCondition) {
    const labels: Record<ConditionType, string> = {
      malnutrition: "Malnutrition",
      dehydration: "Dehydration",
      exhaustion: "Exhaustion",
      injury: "Injury",
    };
    lines.push("");
    lines.push(
      `Most common ailment: ${labels[report.mostCommonCondition]} (${report.conditionCounts[report.mostCommonCondition]} cases)`,
    );
  }

  return lines.join("\n");
}
