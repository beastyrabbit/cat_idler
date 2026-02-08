export type JobKind =
  | "supply_food"
  | "supply_water"
  | "leader_plan_hunt"
  | "hunt_expedition"
  | "leader_plan_house"
  | "build_house"
  | "ritual";

export type CatSpecialization = "hunter" | "architect" | "ritualist" | null;

export type UpgradeKey =
  | "click_power"
  | "supply_speed"
  | "hunt_mastery"
  | "build_mastery"
  | "ritual_mastery"
  | "resilience";

export interface UpgradeLevels {
  click_power: number;
  supply_speed: number;
  hunt_mastery: number;
  build_mastery: number;
  ritual_mastery: number;
  resilience: number;
}

export const BASE_JOB_SECONDS: Record<JobKind, number> = {
  supply_food: 20,
  supply_water: 15,
  leader_plan_hunt: 30 * 60,
  hunt_expedition: 8 * 60 * 60,
  leader_plan_house: 20 * 60 * 60,
  build_house: 8 * 60 * 60,
  ritual: 6 * 60 * 60,
};

export function normalizeTimeScale(
  timeScale: number | null | undefined,
): number {
  if (!timeScale || !Number.isFinite(timeScale)) {
    return 1;
  }
  return Math.max(1, timeScale);
}

export function applyClickBoostSeconds(
  clicksInCurrentMinute: number,
  clickPowerLevel: number,
): number {
  const baseBoost = 10 + clickPowerLevel * 2;

  if (clicksInCurrentMinute <= 30) {
    return baseBoost;
  }

  // Smooth exponential decay after 30 clicks
  const excess = clicksInCurrentMinute - 30;
  const decayFactor = Math.pow(0.95, excess / 10);
  return Math.max(1, Math.floor(baseBoost * decayFactor));
}

export function getDurationSeconds(
  kind: JobKind,
  specialization: CatSpecialization,
  upgrades: UpgradeLevels,
): number {
  const base = BASE_JOB_SECONDS[kind];
  let multiplier = 1;

  if (kind === "supply_food" || kind === "supply_water") {
    multiplier *= Math.max(0.55, 1 - upgrades.supply_speed * 0.1);
  }

  if (kind === "hunt_expedition") {
    multiplier *= Math.max(0.45, 1 - upgrades.hunt_mastery * 0.1);
    if (specialization === "hunter") {
      multiplier *= 0.5;
    }
  }

  if (kind === "build_house" || kind === "leader_plan_house") {
    multiplier *= Math.max(0.45, 1 - upgrades.build_mastery * 0.1);
    if (specialization === "architect" && kind === "build_house") {
      multiplier *= 0.5;
    }
  }

  if (kind === "ritual") {
    multiplier *= Math.max(0.4, 1 - upgrades.ritual_mastery * 0.12);
    if (specialization === "ritualist") {
      multiplier *= 0.6;
    }
  }

  return Math.max(5, Math.floor(base * multiplier));
}

export function getScaledDurationSeconds(
  kind: JobKind,
  specialization: CatSpecialization,
  upgrades: UpgradeLevels,
  timeScale: number | null | undefined,
): number {
  const base = getDurationSeconds(kind, specialization, upgrades);
  const scale = normalizeTimeScale(timeScale);
  return Math.max(1, Math.floor(base / scale));
}

export function getHuntReward(
  huntSkill: number,
  specialization: CatSpecialization,
  hunterXp: number,
  upgrades: UpgradeLevels,
): number {
  const base = 14 + Math.floor(huntSkill / 15);
  const upgradeBonus = 1 + upgrades.hunt_mastery * 0.12;
  const specialistBonus =
    specialization === "hunter" && hunterXp >= 30 ? 1.5 : 1;
  return Math.max(1, Math.floor(base * upgradeBonus * specialistBonus));
}

export function getResilienceHours(
  upgrades: UpgradeLevels,
  automationTier: number,
): number {
  const base = 2;
  const upgradeBonus = upgrades.resilience * 6;
  const automationBonus = Math.floor(automationTier * 6);
  return Math.min(96, base + upgradeBonus + automationBonus);
}

export function getUpgradeCost(baseCost: number, currentLevel: number): number {
  return baseCost * (currentLevel + 1);
}

export function nextSpecialization(
  role: Exclude<CatSpecialization, null>,
  roleXp: number,
  current: CatSpecialization,
): CatSpecialization {
  if (current) {
    return current;
  }

  if (roleXp >= 10) {
    return role;
  }

  return null;
}
