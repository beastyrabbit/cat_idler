/**
 * Lunar Calendar System
 *
 * Pure functions for tracking 8-phase moon cycle (30-day period)
 * and calculating stat modifiers for hunting, stealth, breeding, mood, and rest.
 */

// ── Types ──────────────────────────────────────────────────────

export type MoonPhase =
  | "new_moon"
  | "waxing_crescent"
  | "first_quarter"
  | "waxing_gibbous"
  | "full_moon"
  | "waning_gibbous"
  | "last_quarter"
  | "waning_crescent";

export interface MoonModifiers {
  huntingBonus: number;
  stealthBonus: number;
  breedingBonus: number;
  moodBonus: number;
  restBonus: number;
}

export interface LunarReport {
  currentPhase: MoonPhase;
  phaseName: string;
  dayInCycle: number;
  daysUntilFullMoon: number;
  modifiers: MoonModifiers;
  affectedCats: number;
  dominantEffect: string;
}

// ── Constants ──────────────────────────────────────────────────

const LUNAR_CYCLE_DAYS = 30;
const FULL_MOON_START = 15;

const ZERO_MODIFIERS: MoonModifiers = {
  huntingBonus: 0,
  stealthBonus: 0,
  breedingBonus: 0,
  moodBonus: 0,
  restBonus: 0,
};

const PHASE_MODIFIERS: Record<MoonPhase, MoonModifiers> = {
  new_moon: {
    huntingBonus: -10,
    stealthBonus: 30,
    breedingBonus: 0,
    moodBonus: 0,
    restBonus: 0,
  },
  waxing_crescent: {
    huntingBonus: 0,
    stealthBonus: 0,
    breedingBonus: 0,
    moodBonus: 10,
    restBonus: 0,
  },
  first_quarter: { ...ZERO_MODIFIERS },
  waxing_gibbous: {
    huntingBonus: 15,
    stealthBonus: 0,
    breedingBonus: 0,
    moodBonus: 0,
    restBonus: 0,
  },
  full_moon: {
    huntingBonus: 30,
    stealthBonus: 0,
    breedingBonus: 20,
    moodBonus: -10,
    restBonus: 0,
  },
  waning_gibbous: {
    huntingBonus: 0,
    stealthBonus: 0,
    breedingBonus: 0,
    moodBonus: 0,
    restBonus: 20,
  },
  last_quarter: { ...ZERO_MODIFIERS },
  waning_crescent: {
    huntingBonus: 0,
    stealthBonus: 15,
    breedingBonus: 0,
    moodBonus: -5,
    restBonus: 0,
  },
};

const PHASE_NAMES: Record<MoonPhase, string> = {
  new_moon: "New Moon",
  waxing_crescent: "Waxing Crescent",
  first_quarter: "First Quarter",
  waxing_gibbous: "Waxing Gibbous",
  full_moon: "Full Moon",
  waning_gibbous: "Waning Gibbous",
  last_quarter: "Last Quarter",
  waning_crescent: "Waning Crescent",
};

// ── Functions ──────────────────────────────────────────────────

export function calculateMoonDay(dayNumber: number): number {
  const safe = Math.max(0, dayNumber);
  return safe % LUNAR_CYCLE_DAYS;
}

export function getMoonPhase(dayInCycle: number): MoonPhase {
  if (dayInCycle <= 3) return "new_moon";
  if (dayInCycle <= 7) return "waxing_crescent";
  if (dayInCycle <= 10) return "first_quarter";
  if (dayInCycle <= 14) return "waxing_gibbous";
  if (dayInCycle <= 18) return "full_moon";
  if (dayInCycle <= 22) return "waning_gibbous";
  if (dayInCycle <= 25) return "last_quarter";
  return "waning_crescent";
}

export function getMoonPhaseModifiers(phase: MoonPhase): MoonModifiers {
  return { ...PHASE_MODIFIERS[phase] };
}

export function getMoonPhaseName(phase: MoonPhase): string {
  return PHASE_NAMES[phase];
}

function findDominantEffect(modifiers: MoonModifiers): string {
  const entries: [string, number][] = [
    ["hunting", modifiers.huntingBonus],
    ["stealth", modifiers.stealthBonus],
    ["breeding", modifiers.breedingBonus],
    ["mood", modifiers.moodBonus],
    ["rest", modifiers.restBonus],
  ];

  let best = entries[0];
  for (const entry of entries) {
    if (Math.abs(entry[1]) > Math.abs(best[1])) {
      best = entry;
    }
  }

  if (best[1] === 0) return "none";
  const sign = best[1] > 0 ? "+" : "";
  return `${best[0]} ${sign}${best[1]}%`;
}

export function evaluateLunarInfluence(
  dayNumber: number,
  catCount: number,
): LunarReport {
  const dayInCycle = calculateMoonDay(dayNumber);
  const phase = getMoonPhase(dayInCycle);
  const modifiers = getMoonPhaseModifiers(phase);

  let daysUntilFullMoon: number;
  if (dayInCycle >= FULL_MOON_START && dayInCycle <= 18) {
    daysUntilFullMoon = 0;
  } else if (dayInCycle < FULL_MOON_START) {
    daysUntilFullMoon = FULL_MOON_START - dayInCycle;
  } else {
    daysUntilFullMoon = LUNAR_CYCLE_DAYS - dayInCycle + FULL_MOON_START;
  }

  return {
    currentPhase: phase,
    phaseName: getMoonPhaseName(phase),
    dayInCycle,
    daysUntilFullMoon,
    modifiers,
    affectedCats: catCount,
    dominantEffect: findDominantEffect(modifiers),
  };
}

export function generateLunarAlmanac(
  report: LunarReport,
  colonyName: string,
): string {
  const lines: string[] = [];

  lines.push(`LUNAR ALMANAC — ${colonyName}`);
  lines.push("");

  if (report.currentPhase === "full_moon") {
    lines.push(
      "THE FULL MOON RISES! All cats feel the pull of the lunar tide. " +
        "Hunters prowl with fierce determination, but restless energy fills the colony.",
    );
  } else if (report.currentPhase === "new_moon") {
    lines.push(
      "A New Moon cloaks the colony in darkness. " +
        "Shadows deepen and stealth comes naturally to every cat.",
    );
  } else {
    lines.push(`The ${report.phaseName} hangs over ${colonyName} tonight.`);
  }

  lines.push("");
  lines.push(
    `Phase: ${report.phaseName} (Day ${report.dayInCycle + 1} of ${LUNAR_CYCLE_DAYS})`,
  );
  lines.push(`Cats affected: ${report.affectedCats}`);

  if (report.dominantEffect !== "none") {
    lines.push(`Dominant influence: ${report.dominantEffect}`);
  }

  if (report.daysUntilFullMoon === 0) {
    lines.push("The full moon is upon us!");
  } else {
    lines.push(`Days until next full moon: ${report.daysUntilFullMoon}`);
  }

  return lines.join("\n");
}
