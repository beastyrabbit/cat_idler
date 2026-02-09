/**
 * Colony Emergency Alert System
 *
 * Pure functions for detecting colony emergencies from metrics,
 * classifying severity levels, and generating newspaper BREAKING NEWS bulletins.
 */

export type EmergencyType =
  | "famine"
  | "drought"
  | "epidemic"
  | "overcrowding"
  | "predator_siege";

export type SeverityLevel =
  | "advisory"
  | "warning"
  | "critical"
  | "catastrophic";

export interface ColonyMetrics {
  foodLevel: number;
  waterLevel: number;
  avgHealth: number;
  sickCatCount: number;
  totalCats: number;
  shelterCapacity: number;
  predatorThreat: number;
}

export interface EmergencyAlert {
  type: EmergencyType;
  severity: number;
  level: SeverityLevel;
  message: string;
}

const FAMINE_THRESHOLD = 20;
const DROUGHT_THRESHOLD = 20;
const EPIDEMIC_SICK_RATIO = 0.3;
const PREDATOR_SIEGE_THRESHOLD = 60;

const EMERGENCY_MESSAGES: Record<EmergencyType, (severity: number) => string> =
  {
    famine: (s) =>
      s >= 76
        ? "Colony faces starvation — food reserves completely depleted"
        : s >= 51
          ? "Food dangerously low — rationing in effect"
          : s >= 26
            ? "Food supplies declining — conservation advised"
            : "Food reserves dropping — situation monitored",
    drought: (s) =>
      s >= 76
        ? "Water crisis — sources nearly dry, dehydration imminent"
        : s >= 51
          ? "Water reserves critically low — strict rationing ordered"
          : s >= 26
            ? "Water supplies declining — conservation measures needed"
            : "Water levels dropping — monitoring continues",
    epidemic: (s) =>
      s >= 76
        ? "Plague sweeps colony — mass sickness threatens survival"
        : s >= 51
          ? "Epidemic worsening — sick cats overwhelm healers"
          : s >= 26
            ? "Illness spreading — quarantine measures recommended"
            : "Increased sickness reported — healers on alert",
    overcrowding: (s) =>
      s >= 76
        ? "Severe overcrowding — cats fighting for space, morale collapsing"
        : s >= 51
          ? "Overcrowding critical — shelters packed beyond capacity"
          : s >= 26
            ? "Population straining shelter limits — expansion needed"
            : "Colony growing beyond comfortable capacity",
    predator_siege: (s) =>
      s >= 76
        ? "Predator siege — colony surrounded, casualties mounting"
        : s >= 51
          ? "Sustained predator assault — defenses under heavy pressure"
          : s >= 26
            ? "Predator activity intensifying — patrols on high alert"
            : "Predator presence increasing — watchers deployed",
  };

/**
 * Calculate severity score (0-100) for a specific emergency type.
 * Returns 0 when metrics are within safe thresholds.
 */
export function calculateSeverity(
  emergencyType: EmergencyType,
  metrics: ColonyMetrics,
): number {
  let raw: number;

  switch (emergencyType) {
    case "famine": {
      if (metrics.foodLevel >= FAMINE_THRESHOLD) return 0;
      raw = ((FAMINE_THRESHOLD - metrics.foodLevel) / FAMINE_THRESHOLD) * 100;
      break;
    }
    case "drought": {
      if (metrics.waterLevel >= DROUGHT_THRESHOLD) return 0;
      raw =
        ((DROUGHT_THRESHOLD - metrics.waterLevel) / DROUGHT_THRESHOLD) * 100;
      break;
    }
    case "epidemic": {
      const sickRatio =
        metrics.totalCats > 0 ? metrics.sickCatCount / metrics.totalCats : 0;
      if (sickRatio <= EPIDEMIC_SICK_RATIO) return 0;
      const ratioSeverity =
        ((sickRatio - EPIDEMIC_SICK_RATIO) / (1 - EPIDEMIC_SICK_RATIO)) * 50;
      const healthSeverity = ((100 - metrics.avgHealth) / 100) * 50;
      raw = ratioSeverity + healthSeverity;
      break;
    }
    case "overcrowding": {
      if (metrics.totalCats <= metrics.shelterCapacity) return 0;
      const overflowRatio =
        (metrics.totalCats - metrics.shelterCapacity) /
        Math.max(1, metrics.shelterCapacity);
      raw = Math.min(overflowRatio * 100, 100);
      break;
    }
    case "predator_siege": {
      if (metrics.predatorThreat <= PREDATOR_SIEGE_THRESHOLD) return 0;
      raw =
        ((metrics.predatorThreat - PREDATOR_SIEGE_THRESHOLD) /
          (100 - PREDATOR_SIEGE_THRESHOLD)) *
        100;
      break;
    }
  }

  return Math.min(100, Math.max(0, Math.round(raw)));
}

/**
 * Map a severity score (0-100) to a named severity level.
 */
export function classifySeverityLevel(score: number): SeverityLevel {
  if (score <= 25) return "advisory";
  if (score <= 50) return "warning";
  if (score <= 75) return "critical";
  return "catastrophic";
}

/**
 * Scan colony metrics and return alerts for all active emergencies.
 */
export function detectEmergencies(metrics: ColonyMetrics): EmergencyAlert[] {
  const emergencyTypes: EmergencyType[] = [
    "famine",
    "drought",
    "epidemic",
    "overcrowding",
    "predator_siege",
  ];

  const alerts: EmergencyAlert[] = [];

  for (const type of emergencyTypes) {
    const severity = calculateSeverity(type, metrics);
    if (severity > 0) {
      const level = classifySeverityLevel(severity);
      const message = EMERGENCY_MESSAGES[type](severity);
      alerts.push({ type, severity, level, message });
    }
  }

  return alerts;
}

/**
 * Sort alerts by severity descending (most urgent first).
 */
export function prioritizeAlerts(alerts: EmergencyAlert[]): EmergencyAlert[] {
  return [...alerts].sort((a, b) => b.severity - a.severity);
}

/**
 * Generate a newspaper "BREAKING NEWS" emergency bulletin.
 */
export function generateEmergencyBulletin(
  alerts: EmergencyAlert[],
  colonyName: string,
): string {
  if (alerts.length === 0) {
    return `EMERGENCY BULLETIN — ${colonyName}\n\nNo emergencies reported. All clear in ${colonyName} today.`;
  }

  const hasCatastrophic = alerts.some((a) => a.level === "catastrophic");
  const header = hasCatastrophic
    ? `!!! BREAKING NEWS — CATASTROPHIC EMERGENCY IN ${colonyName.toUpperCase()} !!!`
    : `BREAKING NEWS — Emergency Alert for ${colonyName}`;

  const lines = [header, ""];

  if (alerts.length === 1) {
    const alert = alerts[0];
    const levelTag = alert.level.toUpperCase();
    lines.push(`[${levelTag}] ${alert.message}`);
  } else {
    lines.push(
      `${alerts.length} active emergencies reported in ${colonyName}:`,
    );
    lines.push("");
    for (const alert of alerts) {
      const levelTag = alert.level.toUpperCase();
      lines.push(`  [${levelTag}] ${alert.message}`);
    }
  }

  return lines.join("\n");
}
