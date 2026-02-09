import type { LifeStage, CatStats } from "@/types/game";
import { getLifeStage } from "./age";

export interface LifeStageTransition {
  from: LifeStage;
  to: LifeStage;
  label: string;
  ageThresholdHours: number;
}

export interface MilestoneAnnouncement {
  headline: string;
  body: string;
  catName: string;
  transition: LifeStageTransition;
  statHighlight: string;
}

const TRANSITIONS: {
  from: LifeStage;
  to: LifeStage;
  label: string;
  threshold: number;
}[] = [
  { from: "kitten", to: "young", label: "First Steps", threshold: 6 },
  { from: "young", to: "adult", label: "Coming of Age", threshold: 24 },
  { from: "adult", to: "elder", label: "Wisdom Years", threshold: 48 },
];

/**
 * Detect if a life stage boundary was crossed between two age snapshots.
 * Returns the earliest transition crossed, or null if none.
 */
export function detectLifeStageTransition(
  previousAgeHours: number,
  currentAgeHours: number,
): LifeStageTransition | null {
  if (previousAgeHours < 0 || currentAgeHours < 0) return null;

  const prevStage = getLifeStage(previousAgeHours);
  const currStage = getLifeStage(currentAgeHours);

  if (prevStage === currStage) return null;

  // Find the first boundary crossed after previousAgeHours
  for (const t of TRANSITIONS) {
    if (previousAgeHours < t.threshold && currentAgeHours >= t.threshold) {
      return {
        from: t.from,
        to: t.to,
        label: t.label,
        ageThresholdHours: t.threshold,
      };
    }
  }

  return null;
}

/**
 * Get the milestone title from a transition.
 */
export function getMilestoneTitle(transition: LifeStageTransition): string {
  return transition.label;
}

/**
 * Get a milestone description including the cat's name.
 */
export function getMilestoneDescription(
  transition: LifeStageTransition,
  catName: string,
): string {
  switch (transition.to) {
    case "young":
      return `${catName} has left the nursery and taken their first brave steps into the colony. A new explorer emerges!`;
    case "adult":
      return `${catName} has come of age and is now a full member of the colony, ready to take on any duty.`;
    case "elder":
      return `${catName} enters the wisdom years, a respected sage whose experience guides the colony.`;
    default:
      return `${catName} has reached a new life stage.`;
  }
}

/**
 * Find the highest stat name from a CatStats object.
 * Ties broken alphabetically (Object.entries iterates in insertion order,
 * but we sort to guarantee alphabetical first).
 */
function getHighestStat(stats: CatStats): string {
  const entries = Object.entries(stats) as [keyof CatStats, number][];
  entries.sort((a, b) => {
    if (b[1] !== a[1]) return b[1] - a[1];
    return a[0].localeCompare(b[0]);
  });
  return entries[0][0];
}

/**
 * Generate a full newspaper-style milestone announcement.
 */
export function generateMilestoneAnnouncement(
  transition: LifeStageTransition,
  catName: string,
  stats: CatStats,
): MilestoneAnnouncement {
  const highestStat = getHighestStat(stats);

  return {
    headline: `${transition.label}: ${catName}`,
    body: getMilestoneDescription(transition, catName),
    catName,
    transition,
    statHighlight: `Strongest trait: ${highestStat} (${stats[highestStat as keyof CatStats]})`,
  };
}
