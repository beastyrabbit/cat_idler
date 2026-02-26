import { rollSeeded } from "./seededRng";
import type { CatStats } from "@/types/game";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type AthleticEvent =
  | "mousing_sprint"
  | "high_fence_jump"
  | "tail_chase"
  | "night_stalk"
  | "fish_swipe";

export interface EventStanding {
  catName: string;
  score: number;
  rank: number;
}

export interface MVPAward {
  catName: string;
  totalScore: number;
  bestEvent: AthleticEvent;
  bestEventScore: number;
}

export interface SportsPage {
  sectionTitle: string;
  entries: string[];
  mvp: MVPAward;
  totalEvents: number;
  editionNumber: number;
}

// ---------------------------------------------------------------------------
// Event stat weights: [primaryStat, primaryWeight, secondaryStat, secondaryWeight]
// ---------------------------------------------------------------------------

const EVENT_WEIGHTS: Record<
  AthleticEvent,
  {
    primary: keyof CatStats;
    primaryW: number;
    secondary: keyof CatStats;
    secondaryW: number;
  }
> = {
  mousing_sprint: {
    primary: "hunting",
    primaryW: 0.7,
    secondary: "vision",
    secondaryW: 0.3,
  },
  high_fence_jump: {
    primary: "attack",
    primaryW: 0.5,
    secondary: "defense",
    secondaryW: 0.5,
  },
  tail_chase: {
    primary: "hunting",
    primaryW: 0.5,
    secondary: "vision",
    secondaryW: 0.5,
  },
  night_stalk: {
    primary: "vision",
    primaryW: 0.7,
    secondary: "hunting",
    secondaryW: 0.3,
  },
  fish_swipe: {
    primary: "hunting",
    primaryW: 0.5,
    secondary: "attack",
    secondaryW: 0.5,
  },
};

// ---------------------------------------------------------------------------
// Commentary templates: Record<placement, Record<eventType, string[]>>
// ---------------------------------------------------------------------------

type Placement = "gold" | "silver" | "bronze" | "other";

const COMMENTARY: Record<Placement, Record<AthleticEvent, string[]>> = {
  gold: {
    mousing_sprint: [
      "{name} blazed through the field like a ginger lightning bolt!",
      "{name} left all rivals eating dust in a stunning sprint victory!",
      "{name} claimed gold with paws barely touching the ground!",
    ],
    high_fence_jump: [
      "{name} soared over the fence with breathtaking grace!",
      "{name} cleared the highest bar the colony has ever seen!",
      "{name} defied gravity in a jaw-dropping leap to victory!",
    ],
    tail_chase: [
      "{name} outlasted every competitor in an epic endurance showdown!",
      "{name} kept going when all others collapsed — a true champion!",
      "{name} proved unstoppable in the grueling tail chase!",
    ],
    night_stalk: [
      "{name} moved like a shadow — nobody saw them coming!",
      "{name} was practically invisible in a masterclass of stealth!",
      "{name} emerged from the darkness as the undisputed champion!",
    ],
    fish_swipe: [
      "{name} snagged the biggest catch with lightning reflexes!",
      "{name} plucked fish from the stream like a seasoned angler!",
      "{name} dominated the riverbank with unmatched precision!",
    ],
  },
  silver: {
    mousing_sprint: [
      "{name} gave the winner a real scare — a close second!",
      "{name} pushed hard but fell just short of the finish line.",
      "{name} showed impressive speed, narrowly missing gold.",
    ],
    high_fence_jump: [
      "{name} cleared an impressive height for a strong silver.",
      "{name} nearly matched the champion's leap — a fine effort!",
      "{name} showed excellent form in a tight second-place finish.",
    ],
    tail_chase: [
      "{name} held on valiantly but faded in the final stretch.",
      "{name} put up a tremendous fight for second place.",
      "{name} showed great stamina but couldn't quite keep pace.",
    ],
    night_stalk: [
      "{name} was nearly undetectable — a worthy silver medalist.",
      "{name} crept through the shadows with impressive skill.",
      "{name} proved a formidable stalker, just behind the champion.",
    ],
    fish_swipe: [
      "{name} landed some impressive catches for a solid silver.",
      "{name} showed quick paws but couldn't quite match the winner.",
      "{name} put on a fine fishing display, earning second place.",
    ],
  },
  bronze: {
    mousing_sprint: [
      "{name} rounds out the podium with a respectable third.",
      "{name} showed grit to hold onto the bronze position.",
      "{name} earned bronze with a determined late surge.",
    ],
    high_fence_jump: [
      "{name} cleared a decent height to claim the bronze.",
      "{name} showed solid jumping ability for third place.",
      "{name} earned a podium spot with a consistent performance.",
    ],
    tail_chase: [
      "{name} hung in there for a gritty bronze finish.",
      "{name} outlasted most of the field for a solid third.",
      "{name} showed real heart to complete the podium.",
    ],
    night_stalk: [
      "{name} managed to stay hidden long enough for bronze.",
      "{name} showed promising stealth skills — one to watch.",
      "{name} crept into third place with a decent showing.",
    ],
    fish_swipe: [
      "{name} snagged enough fish to earn the bronze.",
      "{name} showed decent technique for a third-place finish.",
      "{name} rounded out the podium with some nice catches.",
    ],
  },
  other: {
    mousing_sprint: [
      "{name} competed bravely but finished out of the medals.",
      "{name} gained valuable experience for next time.",
      "{name} showed spirit despite a tough field.",
    ],
    high_fence_jump: [
      "{name} didn't make the podium but showed heart.",
      "{name} needs more practice on the high bars.",
      "{name} participated with enthusiasm if not height.",
    ],
    tail_chase: [
      "{name} gave it their all but ran out of steam.",
      "{name} needs to work on endurance training.",
      "{name} finished the course — and that's what counts.",
    ],
    night_stalk: [
      "{name} was spotted too early — better luck next time.",
      "{name} needs to work on those sneaking skills.",
      "{name} tried their best in the shadows.",
    ],
    fish_swipe: [
      "{name} came away empty-pawed this time.",
      "{name} needs to work on their fishing reflexes.",
      "{name} splashed around but didn't land the big ones.",
    ],
  },
};

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

export function getEventScore(
  stats: CatStats,
  eventType: AthleticEvent,
  seed: number,
): number {
  const w = EVENT_WEIGHTS[eventType];
  const baseStat =
    stats[w.primary] * w.primaryW + stats[w.secondary] * w.secondaryW;

  // Seeded RNG variance: ±15% of base stat (minimum 1 for zero-stat edge case)
  const mixedSeed = (seed * 31 + eventType.length * 7) >>> 0 || 1;
  const roll = rollSeeded(mixedSeed);
  const variance = 0.85 + roll.value * 0.3; // 0.85 to 1.15

  return baseStat * variance;
}

export function rankCompetitors(
  cats: { name: string; stats: CatStats }[],
  eventType: AthleticEvent,
  seed: number,
): EventStanding[] {
  if (cats.length === 0) return [];

  const scored = cats.map((cat, index) => {
    // Mix seed with cat index for per-cat variance (deterministic tie-breaking)
    const catSeed = (seed * 37 + index * 13 + 1) >>> 0 || 1;
    return {
      catName: cat.name,
      score: getEventScore(cat.stats, eventType, catSeed),
    };
  });

  scored.sort((a, b) => b.score - a.score);

  return scored.map((entry, index) => ({
    catName: entry.catName,
    score: entry.score,
    rank: index + 1,
  }));
}

function getPlacement(rank: number): Placement {
  if (rank === 1) return "gold";
  if (rank === 2) return "silver";
  if (rank === 3) return "bronze";
  return "other";
}

export function generateCommentary(
  standing: EventStanding,
  eventType: AthleticEvent,
  seed: number,
): string {
  const placement = getPlacement(standing.rank);
  const templates = COMMENTARY[placement][eventType];

  const mixedSeed =
    (seed * 41 + standing.rank * 17 + eventType.length * 3) >>> 0 || 1;
  const roll = rollSeeded(mixedSeed);
  const index = Math.floor(roll.value * templates.length);

  return templates[index].replace("{name}", standing.catName);
}

export function selectMVP(
  allStandings: Record<AthleticEvent, EventStanding[]>,
  seed: number,
): MVPAward {
  const events = Object.keys(allStandings) as AthleticEvent[];

  // Aggregate total scores and best event per cat
  const catTotals: Record<
    string,
    { total: number; bestEvent: AthleticEvent; bestScore: number }
  > = {};

  for (const event of events) {
    for (const standing of allStandings[event]) {
      if (!catTotals[standing.catName]) {
        catTotals[standing.catName] = {
          total: 0,
          bestEvent: event,
          bestScore: 0,
        };
      }
      catTotals[standing.catName].total += standing.score;
      if (standing.score > catTotals[standing.catName].bestScore) {
        catTotals[standing.catName].bestScore = standing.score;
        catTotals[standing.catName].bestEvent = event;
      }
    }
  }

  const entries = Object.entries(catTotals);

  // Sort by total descending; tie-break deterministically with seed
  entries.sort((a, b) => {
    if (b[1].total !== a[1].total) return b[1].total - a[1].total;
    // Tie-break: use seed-based hash of name
    const hashA = (seed * 53 + a[0].charCodeAt(0) * 7) >>> 0;
    const hashB = (seed * 53 + b[0].charCodeAt(0) * 7) >>> 0;
    return hashB - hashA;
  });

  const [topName, topData] = entries[0];

  return {
    catName: topName,
    totalScore: topData.total,
    bestEvent: topData.bestEvent,
    bestEventScore: topData.bestScore,
  };
}

export function formatEventResult(
  eventType: AthleticEvent,
  standings: EventStanding[],
  commentary: string,
): string {
  const title = eventType.replace(/_/g, " ").toUpperCase();
  const lines: string[] = [`--- ${title} ---`];

  for (const s of standings) {
    lines.push(`  ${s.rank}. ${s.catName} — ${s.score.toFixed(1)} pts`);
  }

  if (commentary) {
    lines.push("");
    lines.push(commentary);
  }

  return lines.join("\n");
}

export function formatSportsPage(
  entries: string[],
  mvp: MVPAward,
  editionNumber: number,
): SportsPage {
  return {
    sectionTitle: "SPORTS & ATHLETICS",
    entries,
    mvp,
    totalEvents: entries.length,
    editionNumber,
  };
}
