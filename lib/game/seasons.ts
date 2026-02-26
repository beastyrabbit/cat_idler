/**
 * Seasonal Cycle System
 *
 * Determines the current season based on colony age in hours.
 * Seasons cycle: spring → summer → autumn → winter → spring → ...
 * Each season applies multipliers to resource gathering, breeding,
 * encounters, and rest decay.
 *
 * Default season length: 6 game-hours (24h full cycle).
 */

export type Season = "spring" | "summer" | "autumn" | "winter";

export interface SeasonModifiers {
  food: number;
  water: number;
  herbs: number;
  materials: number;
  breeding: number;
  encounters: number;
  restDecay: number;
}

export interface SeasonTransition {
  current: Season;
  next: Season;
  hoursUntilNext: number;
}

const SEASON_ORDER: Season[] = ["spring", "summer", "autumn", "winter"];

const DEFAULT_SEASON_LENGTH_HOURS = 6;

const SEASON_MODIFIERS: Record<Season, SeasonModifiers> = {
  spring: {
    food: 1.1,
    water: 1.0,
    herbs: 1.2,
    materials: 1.0,
    breeding: 1.2,
    encounters: 1.0,
    restDecay: 1.0,
  },
  summer: {
    food: 1.2,
    water: 0.9,
    herbs: 1.0,
    materials: 1.0,
    breeding: 1.0,
    encounters: 1.15,
    restDecay: 1.0,
  },
  autumn: {
    food: 1.3,
    water: 1.0,
    herbs: 0.9,
    materials: 1.2,
    breeding: 1.0,
    encounters: 1.0,
    restDecay: 1.0,
  },
  winter: {
    food: 0.7,
    water: 0.9,
    herbs: 0.8,
    materials: 1.0,
    breeding: 1.0,
    encounters: 0.7,
    restDecay: 1.2,
  },
};

function clampAge(colonyAgeHours: number): number {
  return Math.max(0, colonyAgeHours);
}

export function getSeason(
  colonyAgeHours: number,
  seasonLengthHours: number = DEFAULT_SEASON_LENGTH_HOURS,
): Season {
  const age = clampAge(colonyAgeHours);
  const seasonIndex = Math.floor(age / seasonLengthHours) % 4;
  return SEASON_ORDER[seasonIndex];
}

export function getSeasonModifiers(season: Season): SeasonModifiers {
  return { ...SEASON_MODIFIERS[season] };
}

export function getSeasonProgress(
  colonyAgeHours: number,
  seasonLengthHours: number = DEFAULT_SEASON_LENGTH_HOURS,
): number {
  const age = clampAge(colonyAgeHours);
  return (age % seasonLengthHours) / seasonLengthHours;
}

export function getSeasonTransition(
  colonyAgeHours: number,
  seasonLengthHours: number = DEFAULT_SEASON_LENGTH_HOURS,
): SeasonTransition {
  const current = getSeason(colonyAgeHours, seasonLengthHours);
  const currentIndex = SEASON_ORDER.indexOf(current);
  const next = SEASON_ORDER[(currentIndex + 1) % 4];
  const progress = getSeasonProgress(colonyAgeHours, seasonLengthHours);
  const hoursUntilNext = seasonLengthHours * (1 - progress);

  return { current, next, hoursUntilNext };
}
