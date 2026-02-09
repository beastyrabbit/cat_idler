import type { CatNeeds } from "@/types/game";
import { NEEDS_DECAY_RATES } from "@/types/game";

export interface SurvivalResources {
  food: number;
  water: number;
}

export interface SurvivalConfig {
  needsDecayMultiplier: number;
  needsDamageMultiplier: number;
}

export interface SurvivalTickResult {
  nextNeeds: CatNeeds;
  dehydratingStarted: boolean;
  recoveredFromDehydration: boolean;
  died: boolean;
}

function clampNeed(value: number): number {
  return Math.max(0, Math.min(100, value));
}

function clampHealth(value: number): number {
  return Math.max(0, Math.min(100, value));
}

export function applySurvivalTick(
  needs: CatNeeds,
  resources: SurvivalResources,
  elapsedSec: number,
  config: SurvivalConfig,
): SurvivalTickResult {
  const tickUnits = Math.max(0, elapsedSec) / 600;
  const decayScale = Math.max(0.1, config.needsDecayMultiplier);
  const damageScale = Math.max(0.1, config.needsDamageMultiplier);

  const foodAvailable = resources.food > 0;
  const waterAvailable = resources.water > 0;

  const hungerDecayPerUnit = foodAvailable
    ? NEEDS_DECAY_RATES.hunger * 0.25
    : NEEDS_DECAY_RATES.hunger;
  const thirstDecayPerUnit = waterAvailable
    ? NEEDS_DECAY_RATES.thirst * 0.2
    : NEEDS_DECAY_RATES.thirst;

  let nextNeeds: CatNeeds = {
    hunger: clampNeed(needs.hunger - hungerDecayPerUnit * tickUnits * decayScale),
    thirst: clampNeed(needs.thirst - thirstDecayPerUnit * tickUnits * decayScale),
    rest: clampNeed(needs.rest - NEEDS_DECAY_RATES.rest * tickUnits),
    health: needs.health,
  };

  if (foodAvailable && nextNeeds.hunger < 90) {
    nextNeeds.hunger = clampNeed(nextNeeds.hunger + 5 * tickUnits);
  }

  if (waterAvailable && nextNeeds.thirst < 90) {
    nextNeeds.thirst = clampNeed(nextNeeds.thirst + 8 * tickUnits);
  }

  let damage = 0;
  if (nextNeeds.hunger === 0) {
    damage += 5 * tickUnits;
  }
  if (nextNeeds.thirst === 0) {
    damage += 3 * tickUnits;
  }

  if (damage > 0) {
    nextNeeds.health = clampHealth(nextNeeds.health - damage * damageScale);
  }

  return {
    nextNeeds,
    dehydratingStarted: needs.thirst > 0 && nextNeeds.thirst === 0,
    recoveredFromDehydration: needs.thirst === 0 && nextNeeds.thirst > 0,
    died: needs.health > 0 && nextNeeds.health === 0,
  };
}
