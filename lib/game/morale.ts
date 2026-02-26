import type { CatNeeds, Resources } from "@/types/game";

export interface MoraleInput {
  catNeedsArray: CatNeeds[];
  resources: Resources;
  aliveCount: number;
  deadCount: number;
}

export interface MoraleResult {
  score: number;
  label: string;
  description: string;
}

export type MoraleLabel =
  | "Euphoric"
  | "Content"
  | "Uneasy"
  | "Distressed"
  | "Despairing";

const MORALE_TIERS: { min: number; label: MoraleLabel; description: string }[] =
  [
    {
      min: 80,
      label: "Euphoric",
      description:
        "The colony thrives with joy and purpose. Spirits could not be higher.",
    },
    {
      min: 60,
      label: "Content",
      description:
        "A calm contentment pervades the colony. All is well enough.",
    },
    {
      min: 40,
      label: "Uneasy",
      description:
        "A quiet unease settles over the colony. Whiskers twitch with concern.",
    },
    {
      min: 20,
      label: "Distressed",
      description:
        "The colony is visibly struggling. Morale sinks with each passing hour.",
    },
    {
      min: 0,
      label: "Despairing",
      description:
        "Despair grips the colony. Survival itself hangs in the balance.",
    },
  ];

export function getMoraleLabel(score: number): MoraleLabel {
  for (const tier of MORALE_TIERS) {
    if (score >= tier.min) {
      return tier.label;
    }
  }
  return "Despairing";
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function avgNeedValue(needsArray: CatNeeds[]): number {
  if (needsArray.length === 0) return 0;
  let total = 0;
  for (const n of needsArray) {
    total += (n.hunger + n.thirst + n.rest + n.health) / 4;
  }
  return total / needsArray.length;
}

function resourceScore(resources: Resources): number {
  const foodWater =
    (clamp(resources.food, 0, 100) / 100) * 0.35 +
    (clamp(resources.water, 0, 100) / 100) * 0.35;
  const secondary =
    (clamp(resources.herbs, 0, 50) / 50) * 0.1 +
    (clamp(resources.materials, 0, 50) / 50) * 0.1 +
    (clamp(resources.blessings, 0, 20) / 20) * 0.1;
  return (foodWater + secondary) * 100;
}

function deathPenalty(deadCount: number, aliveCount: number): number {
  if (deadCount === 0) return 0;
  const ratio = deadCount / Math.max(1, aliveCount + deadCount);
  return clamp(ratio * 40, 0, 40);
}

export function computeColonyMorale(input: MoraleInput): MoraleResult {
  const { catNeedsArray, resources, aliveCount, deadCount } = input;

  if (aliveCount === 0 && catNeedsArray.length === 0) {
    return {
      score: 0,
      label: "Despairing",
      description: MORALE_TIERS[MORALE_TIERS.length - 1].description,
    };
  }

  // Weighted factors:
  // 50% avg cat wellbeing (hunger/thirst/rest/health)
  // 30% resource levels
  // 20% death penalty (subtracted)
  const needsComponent = avgNeedValue(catNeedsArray) * 0.5;
  const resourceComponent = resourceScore(resources) * 0.3;
  const deathComponent = deathPenalty(deadCount, aliveCount);

  const rawScore = needsComponent + resourceComponent - deathComponent;
  const score = Math.round(clamp(rawScore, 0, 100));

  const label = getMoraleLabel(score);
  const tier = MORALE_TIERS.find((t) => t.label === label)!;

  return {
    score,
    label,
    description: tier.description,
  };
}
