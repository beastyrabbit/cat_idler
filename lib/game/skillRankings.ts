/**
 * Cat Skill Rankings — "Who's Who" column for The Catford Examiner.
 *
 * Ranks cats per skill domain, bestows honorary titles on top-ranked cats,
 * generates head-to-head rivalry reports, and produces a formatted
 * "Classifieds & Rankings" newspaper section.
 *
 * Pure functions — no side effects, no randomness.
 */

import type { CatStats } from "@/types/game";

// ─── Types ──────────────────────────────────────────────────────

export type SkillDomain =
  | "hunting"
  | "combat"
  | "medicine"
  | "building"
  | "leadership"
  | "scouting";

export type SkillTier = "master" | "skilled" | "apprentice" | "novice";

export interface RankedCat {
  name: string;
  statValue: number;
  rank: number;
  title: string | null;
}

export interface ColonyChampions {
  champions: Record<
    SkillDomain,
    { name: string; statValue: number; title: string } | null
  >;
  mostVersatile: { name: string; averageStat: number } | null;
}

export interface SkillSummary {
  bestDomain: SkillDomain;
  bestValue: number;
  title: string | null;
  overallRating: number;
}

export interface RankingsColumn {
  headline: string;
  champions: { domain: string; name: string; title: string; value: number }[];
  mostVersatile: { name: string; rating: number } | null;
  rivalries: string[];
  catCount: number;
}

// ─── Constants ──────────────────────────────────────────────────

const SKILL_TITLES: Record<SkillDomain, Record<SkillTier, string>> = {
  hunting: {
    master: "Master Hunter",
    skilled: "Keen Stalker",
    apprentice: "Swift Pouncer",
    novice: "Mouse Chaser",
  },
  combat: {
    master: "Colony Champion",
    skilled: "Fierce Defender",
    apprentice: "Brave Brawler",
    novice: "Scrappy Fighter",
  },
  medicine: {
    master: "Chief Healer",
    skilled: "Herb Whisperer",
    apprentice: "Poultice Mixer",
    novice: "Leaf Sorter",
  },
  building: {
    master: "Master Builder",
    skilled: "Stone Paw",
    apprentice: "Den Shaper",
    novice: "Twig Hauler",
  },
  leadership: {
    master: "Born Leader",
    skilled: "Wise Counsel",
    apprentice: "Rally Caller",
    novice: "Kit Watcher",
  },
  scouting: {
    master: "Eagle Eye",
    skilled: "Sharp Scout",
    apprentice: "Trail Finder",
    novice: "Horizon Gazer",
  },
};

const ALL_DOMAINS: SkillDomain[] = [
  "hunting",
  "combat",
  "medicine",
  "building",
  "leadership",
  "scouting",
];

const DOMAIN_LABELS: Record<SkillDomain, string> = {
  hunting: "Hunting",
  combat: "Combat",
  medicine: "Medicine",
  building: "Building",
  leadership: "Leadership",
  scouting: "Scouting",
};

// ─── Helpers ────────────────────────────────────────────────────

interface CatWithStats {
  name: string;
  stats: CatStats;
}

function getStatForDomain(stats: CatStats, domain: SkillDomain): number {
  switch (domain) {
    case "hunting":
      return stats.hunting;
    case "combat":
      return Math.round((stats.attack + stats.defense) / 2);
    case "medicine":
      return stats.medicine;
    case "building":
      return stats.building;
    case "leadership":
      return stats.leadership;
    case "scouting":
      return stats.vision;
  }
}

function averageAllStats(stats: CatStats): number {
  const values = [
    stats.attack,
    stats.defense,
    stats.hunting,
    stats.medicine,
    stats.cleaning,
    stats.building,
    stats.leadership,
    stats.vision,
  ];
  return Math.round(values.reduce((a, b) => a + b, 0) / values.length);
}

// ─── Exported Functions ─────────────────────────────────────────

export function getSkillTier(statValue: number): SkillTier | null {
  if (statValue >= 80) return "master";
  if (statValue >= 60) return "skilled";
  if (statValue >= 40) return "apprentice";
  if (statValue >= 20) return "novice";
  return null;
}

export function getSkillTitle(
  domain: SkillDomain,
  statValue: number,
): string | null {
  const tier = getSkillTier(statValue);
  if (!tier) return null;
  return SKILL_TITLES[domain][tier];
}

export function rankCatsBySkill(
  cats: CatWithStats[],
  skill: SkillDomain,
): RankedCat[] {
  if (cats.length === 0) return [];

  const withValues = cats.map((cat) => ({
    name: cat.name,
    statValue: getStatForDomain(cat.stats, skill),
  }));

  withValues.sort((a, b) => b.statValue - a.statValue);

  const ranked: RankedCat[] = [];
  let currentRank = 1;

  for (let i = 0; i < withValues.length; i++) {
    if (i > 0 && withValues[i].statValue < withValues[i - 1].statValue) {
      currentRank = i + 1;
    }
    ranked.push({
      name: withValues[i].name,
      statValue: withValues[i].statValue,
      rank: currentRank,
      title: getSkillTitle(skill, withValues[i].statValue),
    });
  }

  return ranked;
}

export function findColonyChampions(cats: CatWithStats[]): ColonyChampions {
  const champions = {} as Record<
    SkillDomain,
    { name: string; statValue: number; title: string } | null
  >;

  for (const domain of ALL_DOMAINS) {
    if (cats.length === 0) {
      champions[domain] = null;
      continue;
    }
    const ranked = rankCatsBySkill(cats, domain);
    const top = ranked[0];
    const title = getSkillTitle(domain, top.statValue);
    champions[domain] = {
      name: top.name,
      statValue: top.statValue,
      title: title ?? DOMAIN_LABELS[domain] + " Hopeful",
    };
  }

  let mostVersatile: { name: string; averageStat: number } | null = null;
  if (cats.length > 0) {
    let bestAvg = -1;
    for (const cat of cats) {
      const avg = averageAllStats(cat.stats);
      if (avg > bestAvg) {
        bestAvg = avg;
        mostVersatile = { name: cat.name, averageStat: avg };
      }
    }
  }

  return { champions, mostVersatile };
}

export function generateRivalryReport(
  cat1: CatWithStats,
  cat2: CatWithStats,
  skill: SkillDomain,
): string {
  const val1 = getStatForDomain(cat1.stats, skill);
  const val2 = getStatForDomain(cat2.stats, skill);
  const label = DOMAIN_LABELS[skill].toLowerCase();
  const diff = Math.abs(val1 - val2);

  if (diff === 0) {
    return `${cat1.name} and ${cat2.name} are locked in a dead heat for ${label} supremacy, both scoring ${val1} — a rivalry for the ages!`;
  }

  const leader = val1 > val2 ? cat1.name : cat2.name;
  const trailer = val1 > val2 ? cat2.name : cat1.name;
  const leaderVal = Math.max(val1, val2);
  const trailerVal = Math.min(val1, val2);

  if (diff <= 5) {
    return `${leader} edges out ${trailer} in ${label} by a whisker (${leaderVal} vs ${trailerVal}) — expect this rivalry to heat up!`;
  }

  if (diff <= 20) {
    return `${leader} leads ${trailer} in ${label} with a solid margin (${leaderVal} vs ${trailerVal}), but the gap is closing.`;
  }

  return `${leader} dominates ${trailer} in ${label} (${leaderVal} vs ${trailerVal}) — a commanding lead that may take moons to challenge.`;
}

export function getSkillSummary(cat: CatWithStats): SkillSummary {
  let bestDomain: SkillDomain = "hunting";
  let bestValue = -1;

  for (const domain of ALL_DOMAINS) {
    const val = getStatForDomain(cat.stats, domain);
    if (val > bestValue) {
      bestValue = val;
      bestDomain = domain;
    }
  }

  return {
    bestDomain,
    bestValue,
    title: getSkillTitle(bestDomain, bestValue),
    overallRating: averageAllStats(cat.stats),
  };
}

export function formatRankingsColumn(cats: CatWithStats[]): RankingsColumn {
  if (cats.length === 0) {
    return {
      headline: "No cats to rank — the colony awaits its first residents!",
      champions: [],
      mostVersatile: null,
      rivalries: [],
      catCount: 0,
    };
  }

  const colonyChampions = findColonyChampions(cats);

  const champList: {
    domain: string;
    name: string;
    title: string;
    value: number;
  }[] = [];
  for (const domain of ALL_DOMAINS) {
    const champ = colonyChampions.champions[domain];
    if (champ) {
      champList.push({
        domain: DOMAIN_LABELS[domain],
        name: champ.name,
        title: champ.title,
        value: champ.statValue,
      });
    }
  }

  const rivalries: string[] = [];
  if (cats.length >= 2) {
    for (const domain of ALL_DOMAINS) {
      const ranked = rankCatsBySkill(cats, domain);
      if (
        ranked.length >= 2 &&
        ranked[0].statValue > 0 &&
        ranked[1].statValue > 0
      ) {
        const diff = ranked[0].statValue - ranked[1].statValue;
        if (diff <= 20) {
          const cat1 = cats.find((c) => c.name === ranked[0].name)!;
          const cat2 = cats.find((c) => c.name === ranked[1].name)!;
          rivalries.push(generateRivalryReport(cat1, cat2, domain));
        }
      }
    }
  }

  const mostVersatile = colonyChampions.mostVersatile
    ? {
        name: colonyChampions.mostVersatile.name,
        rating: colonyChampions.mostVersatile.averageStat,
      }
    : null;

  const topChamp = champList.reduce((best, c) =>
    c.value > best.value ? c : best,
  );
  const headline = `${topChamp.name} crowned ${topChamp.title} as colony rankings shake up!`;

  return {
    headline,
    champions: champList,
    mostVersatile,
    rivalries,
    catCount: cats.length,
  };
}
