import { getLifeStage } from "./lifeSim";
import type { WorldPos } from "./movement";
import { MAX_RAID_CASUALTIES } from "./threat";
import {
  catCombatPower,
  combatStageFactor,
  type WarriorSpecialization,
} from "./warriors";

export const RAID_INTERCEPTION_RADIUS = 2;
export const RAID_INTERCEPTION_WOUND_DAMAGE = 35;

export type RaidInterceptionOutcome = "escape" | "flee" | "wounded" | "killed";

export interface InterceptableCat {
  id: string;
  position: WorldPos;
  trail?: WorldPos[];
  activity?: "idle" | "traveling" | "working" | "returning" | null;
  currentTask?: string | null;
  carrying?: unknown | null;
  deathTime?: number | null;
  stats: {
    attack: number;
    defense: number;
  };
  specialization?: WarriorSpecialization;
  ageHours?: number | null;
  roleXp?: { warrior?: number } | null;
}

export interface InterceptingRaider {
  id: string;
  position: WorldPos;
  trail?: WorldPos[];
  hp: number;
  strength: number;
  status?: "advancing" | "engaging" | "retreating" | "dead";
}

export interface RaidInterceptionPair {
  cat: InterceptableCat;
  raider: InterceptingRaider;
  distance: number;
}

export interface RaidInterceptionRoll {
  outcome: RaidInterceptionOutcome;
  catPower: number;
  raiderPower: number;
  margin: number;
  casualty: boolean;
}

function chebyshev(a: WorldPos, b: WorldPos): number {
  return Math.max(Math.abs(a.x - b.x), Math.abs(a.y - b.y));
}

function tileKey(pos: WorldPos): string {
  return `${Math.round(pos.x)},${Math.round(pos.y)}`;
}

function roundedTrail(position: WorldPos, trail: WorldPos[] | undefined) {
  const raw = trail && trail.length > 0 ? trail : [position];
  const points: WorldPos[] = [];
  const seen = new Set<string>();
  for (const pos of raw) {
    const rounded = { x: Math.round(pos.x), y: Math.round(pos.y) };
    const key = tileKey(rounded);
    if (!seen.has(key)) {
      seen.add(key);
      points.push(rounded);
    }
  }
  return points;
}

export function isCatRaidInterceptable(
  cat: Pick<
    InterceptableCat,
    "activity" | "carrying" | "currentTask" | "deathTime"
  >,
  isInsideVillage: (pos: WorldPos) => boolean,
  position: WorldPos,
): boolean {
  if (cat.deathTime != null || isInsideVillage(position)) {
    return false;
  }
  const activity = cat.activity ?? "idle";
  if (activity === "traveling" || activity === "working") {
    return true;
  }
  return (
    activity === "returning" && (cat.carrying != null || !!cat.currentTask)
  );
}

export function selectRaidInterceptions(
  raiders: InterceptingRaider[],
  cats: InterceptableCat[],
  isInsideVillage: (pos: WorldPos) => boolean,
  radius = RAID_INTERCEPTION_RADIUS,
): RaidInterceptionPair[] {
  const catBuckets = new Map<
    string,
    Array<{ cat: InterceptableCat; position: WorldPos }>
  >();
  for (const cat of cats) {
    for (const position of roundedTrail(cat.position, cat.trail)) {
      if (!isCatRaidInterceptable(cat, isInsideVillage, position)) {
        continue;
      }
      const key = tileKey(position);
      const bucket = catBuckets.get(key) ?? [];
      bucket.push({ cat, position });
      catBuckets.set(key, bucket);
    }
  }
  for (const bucket of catBuckets.values()) {
    bucket.sort((a, b) => a.cat.id.localeCompare(b.cat.id));
  }

  const selected: RaidInterceptionPair[] = [];
  const usedCats = new Set<string>();
  const advancing = [...raiders]
    .filter((r) => (r.status ?? "advancing") === "advancing" && r.hp > 0)
    .sort((a, b) => a.id.localeCompare(b.id));

  for (const raider of advancing) {
    const nearbyByCat = new Map<string, RaidInterceptionPair>();
    for (const raiderPos of roundedTrail(raider.position, raider.trail)) {
      const rx = Math.round(raiderPos.x);
      const ry = Math.round(raiderPos.y);
      for (let y = ry - radius; y <= ry + radius; y += 1) {
        for (let x = rx - radius; x <= rx + radius; x += 1) {
          const distance = Math.max(Math.abs(x - rx), Math.abs(y - ry));
          if (distance > radius) {
            continue;
          }
          for (const entry of catBuckets.get(`${x},${y}`) ?? []) {
            if (usedCats.has(entry.cat.id)) {
              continue;
            }
            const exactDistance = chebyshev(entry.position, raiderPos);
            if (exactDistance <= radius) {
              const current = nearbyByCat.get(entry.cat.id);
              if (!current || exactDistance < current.distance) {
                nearbyByCat.set(entry.cat.id, {
                  cat: entry.cat,
                  raider,
                  distance: exactDistance,
                });
              }
            }
          }
        }
      }
    }
    const nearby = [...nearbyByCat.values()];
    nearby.sort(
      (a, b) =>
        a.distance - b.distance ||
        a.cat.id.localeCompare(b.cat.id) ||
        a.raider.id.localeCompare(b.raider.id),
    );
    for (const pair of nearby) {
      usedCats.add(pair.cat.id);
      selected.push(pair);
    }
  }

  return selected;
}

export function resolveRaidInterception(
  cat: Pick<
    InterceptableCat,
    "ageHours" | "roleXp" | "specialization" | "stats"
  >,
  raider: Pick<InterceptingRaider, "hp" | "strength">,
  roll: number,
  casualtiesSoFar: number,
): RaidInterceptionRoll {
  const lifeStage = getLifeStage(cat.ageHours ?? 0);
  const catPower = catCombatPower({
    attack: cat.stats.attack,
    defense: cat.stats.defense,
    specialization: cat.specialization ?? null,
    warriorXp: cat.roleXp?.warrior ?? 0,
    stageFactor: combatStageFactor(lifeStage),
  });
  const raiderPower = Math.max(1, raider.hp || raider.strength);
  const swing = 0.75 + 0.5 * Math.min(0.999999, Math.max(0, roll));
  const margin = (catPower * swing) / raiderPower;

  if (margin >= 0.95) {
    return {
      outcome: "escape",
      catPower,
      raiderPower,
      margin,
      casualty: false,
    };
  }
  if (margin >= 0.6) {
    return { outcome: "flee", catPower, raiderPower, margin, casualty: false };
  }
  if (margin >= 0.35 || casualtiesSoFar >= MAX_RAID_CASUALTIES) {
    return {
      outcome: "wounded",
      catPower,
      raiderPower,
      margin,
      casualty: false,
    };
  }
  return { outcome: "killed", catPower, raiderPower, margin, casualty: true };
}
