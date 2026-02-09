import type { LifeStage } from "@/types/game";
import { rollSeeded } from "./seededRng";

// ~30 nature-themed prefixes inspired by warrior cats
const PREFIXES = [
  "Shadow",
  "Thorn",
  "Storm",
  "Ember",
  "Frost",
  "Bramble",
  "Ivy",
  "Ash",
  "Fern",
  "Willow",
  "Birch",
  "Hawk",
  "Raven",
  "Cedar",
  "Moss",
  "Flame",
  "Breeze",
  "Dusk",
  "Dawn",
  "Flint",
  "Stone",
  "Cloud",
  "Sage",
  "Briar",
  "Alder",
  "Maple",
  "Pine",
  "Otter",
  "Fox",
  "Wren",
] as const;

// Warrior suffixes for adults and elders
const WARRIOR_SUFFIXES = [
  "claw",
  "whisker",
  "heart",
  "fur",
  "stripe",
  "tail",
  "fang",
  "pelt",
  "thorn",
  "leaf",
  "brook",
  "blaze",
  "shade",
  "pool",
  "flight",
] as const;

/** Pick a deterministic prefix from a seed. */
export function getNamePrefix(seed: number): string {
  const { value } = rollSeeded(seed);
  return PREFIXES[Math.floor(value * PREFIXES.length)];
}

/** Pick the appropriate suffix for a life stage. Kittens get "kit", young get "paw", adults/elders get a warrior suffix. */
export function getNameSuffix(lifeStage: LifeStage, seed: number): string {
  if (lifeStage === "kitten") return "kit";
  if (lifeStage === "young") return "paw";

  // Double-roll to spread sequential seeds across the suffix pool
  const first = rollSeeded(seed);
  const { value } = rollSeeded(first.nextSeed);
  return WARRIOR_SUFFIXES[Math.floor(value * WARRIOR_SUFFIXES.length)];
}

/** Generate a full cat name from a seed and optional life stage (defaults to adult). */
export function generateName(
  seed: number,
  lifeStage: LifeStage = "adult",
): string {
  const prefix = getNamePrefix(seed);

  // Advance the seed for suffix selection so prefix and suffix are independent
  const { nextSeed } = rollSeeded(seed);
  const suffix = getNameSuffix(lifeStage, nextSeed);

  return prefix + suffix;
}

/** Replace the suffix of a cat name with "star" for leader titles. */
export function formatLeaderTitle(name: string): string {
  // Find where the prefix ends by matching against known prefixes (longest match first)
  const sorted = [...PREFIXES].sort((a, b) => b.length - a.length);
  for (const prefix of sorted) {
    if (name.startsWith(prefix) && name.length > prefix.length) {
      return prefix + "star";
    }
  }

  // Fallback: if no known prefix found, use first character as prefix
  if (name.length <= 1) return name + "star";
  return name[0].toUpperCase() + "star";
}

/** Generate unique names for a litter of cats. Defaults to kitten life stage. */
export function generateLitterNames(
  seed: number,
  count: number,
  lifeStage: LifeStage = "kitten",
): string[] {
  if (count <= 0) return [];

  const names: string[] = [];
  const usedNames = new Set<string>();
  let currentSeed = seed;

  for (let i = 0; i < count; i++) {
    let attempts = 0;
    let name: string;

    do {
      // Advance seed for each attempt
      const roll = rollSeeded(currentSeed);
      currentSeed = roll.nextSeed;
      name = generateName(currentSeed, lifeStage);
      const advanceRoll = rollSeeded(currentSeed);
      currentSeed = advanceRoll.nextSeed;
      attempts++;
    } while (usedNames.has(name) && attempts < 100);

    usedNames.add(name);
    names.push(name);
  }

  return names;
}
