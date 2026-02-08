import type { JobKind, UpgradeLevels } from "./idleEngine";

export interface ColonyResources {
  food: number;
  water: number;
  herbs: number;
  materials: number;
  blessings: number;
}

export interface MinimalJob {
  kind: JobKind;
}

export function hasConflictingStrategicJob(
  kind: JobKind,
  jobs: MinimalJob[],
): boolean {
  const kinds = new Set(jobs.map((job) => job.kind));

  if (kind === "leader_plan_hunt") {
    return kinds.has("leader_plan_hunt") || kinds.has("hunt_expedition");
  }

  if (kind === "leader_plan_house") {
    return kinds.has("leader_plan_house") || kinds.has("build_house");
  }

  if (kind === "ritual") {
    return kinds.has("ritual");
  }

  return false;
}

export function shouldAutoQueueHunt(food: number, jobs: MinimalJob[]): boolean {
  if (food >= 12) {
    return false;
  }

  return !hasConflictingStrategicJob("leader_plan_hunt", jobs);
}

export function shouldAutoQueueBuild(
  materials: number,
  jobs: MinimalJob[],
): boolean {
  if (materials >= 8) {
    return false;
  }

  return !hasConflictingStrategicJob("leader_plan_house", jobs);
}

export function shouldStartRitual(
  ritualRequestedAt: number | null | undefined,
  resources: Pick<ColonyResources, "food" | "water">,
  jobs: MinimalJob[],
): boolean {
  if (!ritualRequestedAt) {
    return false;
  }

  if (resources.food < 16 || resources.water < 16) {
    return false;
  }

  return !hasConflictingStrategicJob("ritual", jobs);
}

export function consumptionForTick(
  catCount: number,
  elapsedSec: number,
  upgrades: UpgradeLevels,
): { foodUse: number; waterUse: number } {
  // Resilience max level is 10 → minimum scale 0.20, clamped to 0.45 floor
  const resilienceScale = Math.max(0.45, 1 - upgrades.resilience * 0.08);

  return {
    foodUse: ((catCount * elapsedSec) / 3600) * resilienceScale,
    waterUse: ((catCount * elapsedSec) / 3000) * resilienceScale,
  };
}

export function nextColonyStatus(
  resources: Pick<ColonyResources, "food" | "water" | "herbs">,
): "starting" | "thriving" | "struggling" {
  const totalSupply = resources.food + resources.water + resources.herbs;

  if (totalSupply < 20) {
    return "struggling";
  }

  if (totalSupply > 70) {
    return "thriving";
  }

  return "starting";
}

export function shouldTrackCritical(
  resources: Pick<ColonyResources, "food" | "water">,
  unattendedHours: number,
  resilienceHours: number,
): boolean {
  const criticallyLow = resources.food <= 0 || resources.water <= 0;
  return criticallyLow && unattendedHours >= resilienceHours;
}

export function shouldResetFromCritical(
  criticalSince: number | null,
  now: number,
  criticalMs: number = 5 * 60 * 1000,
): boolean {
  if (!criticalSince) {
    return false;
  }

  return now - criticalSince >= criticalMs;
}

export function ritualRequestIsFresh(
  ritualRequestedAt: number | null | undefined,
  now: number,
  windowMs: number = 12 * 60 * 60 * 1000,
): boolean {
  return !!ritualRequestedAt && now - ritualRequestedAt < windowMs;
}
