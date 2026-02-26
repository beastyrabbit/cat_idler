/**
 * Night Watch — shift-based guard scheduling
 *
 * Pure functions for organizing colony night patrol shifts,
 * calculating coverage quality, identifying gaps, and fatigue.
 */

export type WatchShift = "dusk" | "midnight" | "dawn";

export interface WatchRoster {
  dusk: number;
  midnight: number;
  dawn: number;
  totalGuards: number;
}

export interface CoverageGap {
  shift: WatchShift;
  guardsAssigned: number;
  guardsNeeded: number;
  severity: "minor" | "major" | "critical";
}

/** Minimum guards for full coverage on any shift */
const FULL_COVERAGE_GUARDS = 3;

/** Coverage score per guard by shift type (midnight is harder) */
const COVERAGE_PER_GUARD: Record<WatchShift, number> = {
  dusk: 40,
  midnight: 25,
  dawn: 35,
};

/** Shift weight for overall score calculation (midnight counts double) */
const SHIFT_WEIGHT: Record<WatchShift, number> = {
  dusk: 1,
  midnight: 2,
  dawn: 1,
};

const SHIFTS: WatchShift[] = ["dusk", "midnight", "dawn"];

/**
 * Distribute N guards across 3 shifts, prioritizing midnight.
 *
 * Assignment order: midnight first, then dusk, then dawn.
 * After initial round-robin, remaining guards distributed evenly
 * with midnight getting extras first.
 */
export function assignWatchShifts(guardCount: number): WatchRoster {
  if (guardCount <= 0) {
    return { dusk: 0, midnight: 0, dawn: 0, totalGuards: 0 };
  }

  const roster: WatchRoster = {
    dusk: 0,
    midnight: 0,
    dawn: 0,
    totalGuards: guardCount,
  };

  // Priority order for assignment: midnight, dusk, dawn
  const priority: WatchShift[] = ["midnight", "dusk", "dawn"];
  let remaining = guardCount;

  // Round-robin with priority ordering
  while (remaining > 0) {
    for (const shift of priority) {
      if (remaining <= 0) break;
      roster[shift]++;
      remaining--;
    }
  }

  return roster;
}

/**
 * Score 0-100 for a single shift's coverage.
 *
 * Each guard adds COVERAGE_PER_GUARD points for that shift type.
 * Midnight guards contribute less per guard (harder shift).
 * Caps at 100 for >= FULL_COVERAGE_GUARDS guards.
 */
export function calculateShiftCoverage(
  guardsOnShift: number,
  shiftType: WatchShift,
): number {
  if (guardsOnShift <= 0) return 0;
  if (guardsOnShift >= FULL_COVERAGE_GUARDS) return 100;

  const perGuard = COVERAGE_PER_GUARD[shiftType];
  return Math.min(100, guardsOnShift * perGuard);
}

/**
 * Overall night watch quality (weighted average of shift scores).
 *
 * Midnight counts double: weights are dusk=1, midnight=2, dawn=1.
 * Total weight = 4, so score = (dusk*1 + midnight*2 + dawn*1) / 4.
 */
export function calculateWatchScore(roster: WatchRoster): number {
  const totalWeight = SHIFTS.reduce((sum, s) => sum + SHIFT_WEIGHT[s], 0);

  const weightedSum = SHIFTS.reduce((sum, shift) => {
    const coverage = calculateShiftCoverage(roster[shift], shift);
    return sum + coverage * SHIFT_WEIGHT[shift];
  }, 0);

  if (totalWeight === 0) return 0;

  return Math.round(weightedSum / totalWeight);
}

/**
 * List shifts with inadequate coverage (< FULL_COVERAGE_GUARDS).
 *
 * Severity rules:
 * - 0 guards on any shift → critical
 * - 1 guard on midnight → critical (most dangerous shift)
 * - 1 guard on other shifts → major
 * - 2 guards on any shift → minor
 */
export function identifyCoverageGaps(roster: WatchRoster): CoverageGap[] {
  const gaps: CoverageGap[] = [];

  for (const shift of SHIFTS) {
    const assigned = roster[shift];
    if (assigned >= FULL_COVERAGE_GUARDS) continue;

    let severity: CoverageGap["severity"];
    if (assigned === 0) {
      severity = "critical";
    } else if (assigned === 1) {
      severity = shift === "midnight" ? "critical" : "major";
    } else {
      severity = "minor";
    }

    gaps.push({
      shift,
      guardsAssigned: assigned,
      guardsNeeded: FULL_COVERAGE_GUARDS,
      severity,
    });
  }

  return gaps;
}

/**
 * Fatigue multiplier for cats working multiple shifts.
 *
 * - 0 shifts → 0 (no work done)
 * - 1 shift → 1.0 (normal)
 * - 2 shifts → 1.3 (tired)
 * - 3+ shifts → 1.6 (exhausted)
 */
export function getWatchFatigueModifier(shiftsWorked: number): number {
  if (shiftsWorked <= 0) return 0;
  if (shiftsWorked === 1) return 1.0;
  if (shiftsWorked === 2) return 1.3;
  return 1.6;
}
