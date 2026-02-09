/**
 * Classified Ads System
 *
 * Newspaper-style classifieds generator for The Catford Examiner.
 * Generates humorous classified advertisements based on colony state:
 * job postings, services offered, wanted notices, and lost & found items.
 */

import type { CatStats } from "@/types/game";
import { rollSeeded } from "./seededRng";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type AdCategory =
  | "situations_vacant"
  | "services_offered"
  | "wanted"
  | "lost_and_found";

export interface ClassifiedAd {
  category: AdCategory;
  headline: string;
  body: string;
  urgency: "low" | "medium" | "high";
}

export interface ClassifiedsSection {
  sectionTitle: string;
  ads: ClassifiedAd[];
  totalAds: number;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const VACANT_THRESHOLD = 30;
const VACANT_HIGH_URGENCY = 10;

const SERVICES_THRESHOLD = 60;

const WANTED_THRESHOLD = 20;
const WANTED_HIGH_URGENCY = 5;

const CATEGORY_HEADERS: Record<AdCategory, string> = {
  situations_vacant: "SITUATIONS VACANT",
  services_offered: "SERVICES OFFERED",
  wanted: "WANTED",
  lost_and_found: "LOST & FOUND",
};

const SKILL_LABELS: Record<string, string> = {
  attack: "Combat & Protection",
  defense: "Guard & Patrol",
  hunting: "Hunting & Tracking",
  medicine: "Healing & Herbalism",
  cleaning: "Colony Maintenance",
  building: "Construction & Repairs",
  leadership: "Leadership & Training",
  vision: "Scouting & Exploration",
};

const SERVICE_FLAVOR: Record<string, string[]> = {
  attack: [
    "Fearsome fighter available for colony defense",
    "Battle-tested warrior seeking protection contracts",
    "Experienced combatant offering training sessions",
  ],
  defense: [
    "Vigilant guardian available for night watches",
    "Seasoned patrol cat offering perimeter services",
    "Reliable defender for hire — no intruder too bold",
  ],
  hunting: [
    "Expert tracker with nose for prey",
    "Skilled hunter guarantees fresh catches",
    "Veteran mouser — no rodent escapes",
  ],
  medicine: [
    "Gentle healer with extensive herb knowledge",
    "Experienced medicine cat accepting patients",
    "Herbalist offering seasonal remedies",
  ],
  cleaning: [
    "Meticulous groomer and den organizer",
    "Tidying specialist for communal areas",
    "Professional cleaner — dens left spotless",
  ],
  building: [
    "Master craftspaw — walls, dens, anything",
    "Experienced builder taking new projects",
    "Skilled architect for colony expansion",
  ],
  leadership: [
    "Natural mentor offering guidance to young cats",
    "Experienced leader available for council duties",
    "Wise advisor for colony decision-making",
  ],
  vision: [
    "Sharp-eyed scout offering territory surveys",
    "Expert pathfinder for exploration expeditions",
    "Keen observer available for border reconnaissance",
  ],
};

const LOST_AND_FOUND_ITEMS = [
  {
    headline: "Found: Shiny Pebble",
    body: "Discovered near the eastern stream. Oddly warm to the touch. Claim at the colony den.",
  },
  {
    headline: "Found: Ball of Yarn",
    body: "Tangled but serviceable. Found behind the food storage. Owner please collect.",
  },
  {
    headline: "Found: Feathered Toy",
    body: "Slightly chewed. Located near the nursery. Kittens already eyeing it.",
  },
  {
    headline: "Found: Mysterious Bone",
    body: "Origin unknown. Found at the colony border. Possibly decorative.",
  },
  {
    headline: "Found: Half a Fish",
    body: "Still reasonably fresh. Found near the water bowl. First come, first served.",
  },
  {
    headline: "Found: Piece of String",
    body: "Excellent condition. Discovered on the patrol route. Many potential uses.",
  },
  {
    headline: "Found: Unusual Leaf",
    body: "Bright red and perfectly shaped. Found in the herb garden. Not edible.",
  },
  {
    headline: "Found: Old Bell",
    body: "Slightly rusty but still jingles. Found near the walls. Spooky provenance.",
  },
  {
    headline: "Found: Smooth Stone",
    body: "Perfectly round. Rolled out from under the elder corner. Lucky charm potential.",
  },
  {
    headline: "Found: Bird Feather",
    body: "Magnificent blue plumage. Found after the dawn patrol. Display quality.",
  },
  {
    headline: "Found: Acorn Cap",
    body: "Tiny but makes an excellent hat. Located near the oak tree.",
  },
  {
    headline: "Found: Twig Collection",
    body: "Neatly arranged in a pile. Someone was clearly working on something.",
  },
];

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

export function getVacantPositionAds(
  resources: { food: number; water: number; herbs: number },
  buildingTypes: string[],
  catCount: number,
): ClassifiedAd[] {
  const ads: ClassifiedAd[] = [];

  if (resources.food <= VACANT_THRESHOLD) {
    ads.push({
      category: "situations_vacant",
      headline: "Hunter Needed Urgently",
      body: `Colony of ${catCount} cats seeks experienced hunter. Food supplies running low. Apply at the leader's den.`,
      urgency: resources.food <= VACANT_HIGH_URGENCY ? "high" : "medium",
    });
  }

  if (resources.water <= VACANT_THRESHOLD) {
    ads.push({
      category: "situations_vacant",
      headline: "Water Fetcher Required",
      body: `Seeking reliable cat for water collection duties. Colony reserves dwindling. Strong legs preferred.`,
      urgency: resources.water <= VACANT_HIGH_URGENCY ? "high" : "medium",
    });
  }

  if (resources.herbs <= VACANT_THRESHOLD) {
    ads.push({
      category: "situations_vacant",
      headline: "Herbalist Position Available",
      body: `Colony medicine stocks critically low. Seeking cat with herb knowledge. Training provided for keen learners.`,
      urgency: resources.herbs <= VACANT_HIGH_URGENCY ? "high" : "medium",
    });
  }

  return ads;
}

export function getServicesOfferedAds(
  catStats: CatStats,
  catName: string,
  seed: number,
): ClassifiedAd[] {
  const ads: ClassifiedAd[] = [];
  const statKeys = Object.keys(SKILL_LABELS) as (keyof CatStats)[];
  let currentSeed = seed;

  for (const stat of statKeys) {
    if (catStats[stat] >= SERVICES_THRESHOLD) {
      const flavors = SERVICE_FLAVOR[stat];
      const roll = rollSeeded(currentSeed * 7919 + stat.length);
      currentSeed = roll.nextSeed;
      const flavorIndex = Math.floor(roll.value * flavors.length);

      ads.push({
        category: "services_offered",
        headline: `${SKILL_LABELS[stat]} — ${catName}`,
        body: `${flavors[flavorIndex]} Contact ${catName} at the colony.`,
        urgency: "low",
      });
    }
  }

  return ads;
}

export function getWantedAds(resources: {
  food: number;
  water: number;
  herbs: number;
}): ClassifiedAd[] {
  const ads: ClassifiedAd[] = [];

  if (resources.food <= WANTED_THRESHOLD) {
    ads.push({
      category: "wanted",
      headline: "WANTED: Food Supplies",
      body: "Any food donations gratefully received. Colony stores critically low. See the food storage manager.",
      urgency: resources.food <= WANTED_HIGH_URGENCY ? "high" : "medium",
    });
  }

  if (resources.water <= WANTED_THRESHOLD) {
    ads.push({
      category: "wanted",
      headline: "WANTED: Fresh Water",
      body: "Water reserves dangerously depleted. Any cat with knowledge of nearby water sources, please come forward.",
      urgency: resources.water <= WANTED_HIGH_URGENCY ? "high" : "medium",
    });
  }

  if (resources.herbs <= WANTED_THRESHOLD) {
    ads.push({
      category: "wanted",
      headline: "WANTED: Medicinal Herbs",
      body: "Herb stocks running low. Foragers and gatherers needed. The medicine cat will accept any contributions.",
      urgency: resources.herbs <= WANTED_HIGH_URGENCY ? "high" : "medium",
    });
  }

  return ads;
}

export function getLostAndFoundAds(
  colonySeed: number,
  dayNumber: number,
): ClassifiedAd[] {
  const combinedSeed = colonySeed * 7919 + dayNumber * 131;
  const countRoll = rollSeeded(combinedSeed);
  const count = 1 + Math.floor(countRoll.value * 3); // 1-3 items

  const ads: ClassifiedAd[] = [];
  let currentSeed = countRoll.nextSeed;

  for (let i = 0; i < count; i++) {
    const roll = rollSeeded(currentSeed);
    currentSeed = roll.nextSeed;
    const itemIndex = Math.floor(roll.value * LOST_AND_FOUND_ITEMS.length);
    const item = LOST_AND_FOUND_ITEMS[itemIndex];

    ads.push({
      category: "lost_and_found",
      headline: item.headline,
      body: item.body,
      urgency: "low",
    });
  }

  return ads;
}

export function formatClassifiedAd(ad: ClassifiedAd): string {
  const header = CATEGORY_HEADERS[ad.category];
  const urgentTag = ad.urgency === "high" ? " [URGENT]" : "";
  return `[${header}]${urgentTag}\n${ad.headline}\n${ad.body}`;
}

export function formatClassifiedsSection(
  ads: ClassifiedAd[],
  colonyName: string,
): ClassifiedsSection {
  return {
    sectionTitle: `Classifieds & Notices — ${colonyName}`,
    ads,
    totalAds: ads.length,
  };
}
