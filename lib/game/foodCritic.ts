import { rollSeeded } from "./seededRng";

// ── Types ───────────────────────────────────────────────────────────

export type CuisineCategory =
  | "fresh_catch"
  | "foraged_finds"
  | "stored_provisions"
  | "mouse_farm_fare"
  | "water_service";

export type StarRating = 0 | 1 | 2 | 3 | 4;

export interface DishOfDay {
  name: string;
  category: CuisineCategory;
  description: string;
}

export interface FoodCriticColumn {
  sectionTitle: string;
  entries: string[];
  dishOfDay: DishOfDay;
  totalReviews: number;
  editionNumber: number;
}

// ── Cuisine classification ──────────────────────────────────────────

const SOURCE_TO_CUISINE: Record<string, CuisineCategory> = {
  hunt: "fresh_catch",
  mice: "fresh_catch",
  birds: "fresh_catch",
  fish: "fresh_catch",
  herbs: "foraged_finds",
  forage: "foraged_finds",
  food_storage: "stored_provisions",
  mouse_farm: "mouse_farm_fare",
  water: "water_service",
};

export function classifyCuisine(foodSource: string): CuisineCategory {
  return SOURCE_TO_CUISINE[foodSource] ?? "foraged_finds";
}

// ── Star rating ─────────────────────────────────────────────────────

export function rateStars(
  _cuisineCategory: CuisineCategory,
  resourceLevel: number,
  buildingLevel?: number,
): StarRating {
  if (resourceLevel <= 0) return 0;

  let base: number;
  if (resourceLevel < 20) base = 1;
  else if (resourceLevel < 50) base = 2;
  else if (resourceLevel < 80) base = 3;
  else base = 4;

  if (buildingLevel && buildingLevel > 0) {
    base = Math.min(4, base + 1);
  }

  return base as StarRating;
}

// ── Critique text generation ────────────────────────────────────────

const CRITIQUE_TEMPLATES: Record<CuisineCategory, Record<string, string[]>> = {
  fresh_catch: {
    high: [
      "The mouse was exquisitely presented — a triumph of the colony kitchen. {critic} awards top marks.",
      "A bird breast of such tenderness it practically melted on the tongue. {critic} is most impressed.",
      "The freshest catch this side of the river! {critic} doffs a paw in admiration.",
    ],
    mid: [
      "Adequate rodent fare, if somewhat lacking in seasoning. {critic} has seen better days.",
      "A competent fish fillet, though the presentation left something to be desired. {critic} is noncommittal.",
      "The hunt's bounty was serviceable. {critic} gives a measured nod of approval.",
    ],
    low: [
      "What was presented as 'fresh catch' appeared to be neither fresh nor recently caught. {critic} is unimpressed.",
      "A dried mouse carcass does not a feast make. {critic} demands better from this establishment.",
      "The fish had clearly seen better centuries. {critic} left the plate untouched.",
    ],
  },
  foraged_finds: {
    high: [
      "The herb selection was a revelation — aromatic, fresh, and expertly gathered. {critic} purrs with delight.",
      "Foraged greens of exceptional quality! The forest has been kind. {critic} recommends without reservation.",
      "A masterful display of the forager's art. {critic} is charmed beyond measure.",
    ],
    mid: [
      "The foraged offerings were acceptable, if unremarkable. {critic} has tasted finer weeds.",
      "Decent herb-work, but the colony needs more variety. {critic} encourages exploration.",
      "An honest spread of forest fare. {critic} appreciates the effort, if not the result.",
    ],
    low: [
      "Bitter roots and wilted leaves — the foragers must try harder. {critic} winces.",
      "One cannot review what barely qualifies as food. {critic} is most displeased.",
      "The foraged spread was an insult to discerning palates everywhere. {critic} stalks away.",
    ],
  },
  stored_provisions: {
    high: [
      "The pantry is a marvel of organisation and quality. {critic} commends the stores keeper.",
      "Well-preserved provisions of the highest order. {critic} rests easy knowing the colony eats well.",
      "Stored goods that rival fresh — a testament to expert preservation. {critic} is deeply satisfied.",
    ],
    mid: [
      "The provisions are adequate, though some items nearing their best-before. {critic} notes room for improvement.",
      "A functional larder, neither inspiring nor concerning. {critic} shrugs noncommittally.",
      "Stored fare that keeps bellies full, if not spirits high. {critic} marks it as passable.",
    ],
    low: [
      "The stores are dangerously depleted. {critic} sounds the alarm with utmost urgency.",
      "What provisions remain have seen far better days. {critic} recommends immediate restocking.",
      "An empty pantry tells its own grim story. {critic} has nothing to review.",
    ],
  },
  mouse_farm_fare: {
    high: [
      "The mouse farm produces specimens of outstanding plumpness. {critic} is most gratified.",
      "Farm-raised mice of exceptional quality — tender and well-fed. {critic} awards full marks.",
      "The mouse farm is the crown jewel of colony cuisine. {critic} visits daily.",
    ],
    mid: [
      "Farm output is steady but unremarkable. {critic} hopes for improvements next quarter.",
      "The mice are adequate — not the finest, but reliably available. {critic} appreciates consistency.",
      "A functional operation producing acceptable fare. {critic} sees potential here.",
    ],
    low: [
      "The mouse farm is producing scrawny, sad-looking specimens. {critic} is horrified.",
      "Farm output has dwindled to an embarrassing trickle. {critic} demands investment.",
      "One wonders if the mice are being fed at all. {critic} writes a strongly worded complaint.",
    ],
  },
  water_service: {
    high: [
      "Crystal clear water of exceptional freshness! {critic} has never been so well hydrated.",
      "The water supply is abundant and pure — a true luxury. {critic} drinks deeply and approvingly.",
      "Top-tier hydration services. {critic} rates this the finest water in all the colonies.",
    ],
    mid: [
      "The water supply is functional, though could be cleaner. {critic} is neither parched nor pleased.",
      "Adequate water service — it quenches thirst, and that is its duty. {critic} drinks without complaint.",
      "The water table holds, but just barely. {critic} monitors the situation.",
    ],
    low: [
      "Water rations are critically low. {critic} urges immediate action before tongues dry up entirely.",
      "What passes for water here would make a puddle weep. {critic} is gravely concerned.",
      "The colony faces a hydration crisis. {critic} has filed an emergency report.",
    ],
  },
};

function getCritiquePool(
  category: CuisineCategory,
  stars: StarRating,
): string[] {
  if (stars >= 3) return CRITIQUE_TEMPLATES[category].high;
  if (stars >= 1) return CRITIQUE_TEMPLATES[category].mid;
  return CRITIQUE_TEMPLATES[category].low;
}

export function generateCritiqueText(
  category: CuisineCategory,
  stars: StarRating,
  criticName: string,
  seed: number,
): string {
  const pool = getCritiquePool(category, stars);
  const categoryOrdinal: Record<CuisineCategory, number> = {
    fresh_catch: 1,
    foraged_finds: 2,
    stored_provisions: 3,
    mouse_farm_fare: 4,
    water_service: 5,
  };
  const combinedSeed =
    seed * 7919 + categoryOrdinal[category] * 131 + stars * 37;
  const roll = rollSeeded(combinedSeed);
  const index = Math.floor(roll.value * pool.length);
  return pool[index].replace(/\{critic\}/g, criticName);
}

// ── Dish of the day selection ───────────────────────────────────────

const DISH_NAMES: Record<CuisineCategory, string[]> = {
  fresh_catch: [
    "Grilled Field Mouse à la Paw",
    "Pan-Seared Sparrow Breast",
    "River Trout Tartare",
    "Mouse Tail Consommé",
    "Crispy Vole Fritters",
  ],
  foraged_finds: [
    "Wild Herb Medley",
    "Catnip-Infused Forest Salad",
    "Dandelion Root Surprise",
    "Mushroom Cap Delights",
    "Elderberry Compote",
  ],
  stored_provisions: [
    "Aged Colony Reserve",
    "Preserved Mouse Jerky",
    "Smoked Fish Selection",
    "Dried Berry Platter",
    "Cured Provisions Sampler",
  ],
  mouse_farm_fare: [
    "Farm-Fresh Mouse Filet",
    "Premium Grain-Fed Mouse",
    "Mouse Farm Special",
    "Free-Range Rodent Platter",
    "Artisanal Farm Mouse",
  ],
  water_service: [
    "Spring Water Tasting Flight",
    "Dew-Drop Refreshment",
    "Mountain Stream Draught",
    "Rain Barrel Reserve",
    "Crystal Spring Special",
  ],
};

const DISH_DESCRIPTIONS: Record<CuisineCategory, string> = {
  fresh_catch:
    "Freshly caught and lovingly prepared by the colony's finest hunters",
  foraged_finds:
    "Hand-picked from the colony's surrounding woodland and meadows",
  stored_provisions:
    "Carefully preserved for quality and flavour from the colony stores",
  mouse_farm_fare: "Sustainably raised on the colony's own mouse farm",
  water_service: "Sourced from the purest springs and streams near the colony",
};

interface Resources {
  food: number;
  water: number;
  herbs: number;
}

const RESOURCE_TO_CATEGORY: Record<string, CuisineCategory> = {
  food: "fresh_catch",
  water: "water_service",
  herbs: "foraged_finds",
};

export function selectDishOfDay(
  resources: Resources,
  buildings: string[],
  seed: number,
): DishOfDay {
  // Buildings get priority when resources are tied
  if (buildings.includes("mouse_farm")) {
    const maxResource = Math.max(
      resources.food,
      resources.water,
      resources.herbs,
    );
    if (maxResource <= resources.food || maxResource <= resources.herbs) {
      return pickDish("mouse_farm_fare", seed);
    }
  }
  if (buildings.includes("food_storage")) {
    const maxResource = Math.max(
      resources.food,
      resources.water,
      resources.herbs,
    );
    if (maxResource <= resources.food) {
      return pickDish("stored_provisions", seed);
    }
  }

  // Find highest resource
  let bestCategory: CuisineCategory = "fresh_catch";
  let bestValue = -1;

  for (const [resource, value] of Object.entries(resources)) {
    if (value > bestValue) {
      bestValue = value;
      bestCategory = RESOURCE_TO_CATEGORY[resource] ?? "fresh_catch";
    }
  }

  return pickDish(bestCategory, seed);
}

function pickDish(category: CuisineCategory, seed: number): DishOfDay {
  const names = DISH_NAMES[category];
  const roll = rollSeeded(seed * 6131 + 17);
  const index = Math.floor(roll.value * names.length);
  return {
    name: names[index],
    category,
    description: DISH_DESCRIPTIONS[category],
  };
}

// ── Format single review entry ──────────────────────────────────────

const CATEGORY_DISPLAY: Record<CuisineCategory, string> = {
  fresh_catch: "FRESH CATCH",
  foraged_finds: "FORAGED FINDS",
  stored_provisions: "STORED PROVISIONS",
  mouse_farm_fare: "MOUSE FARM FARE",
  water_service: "WATER SERVICE",
};

function starDisplay(stars: StarRating): string {
  if (stars === 0) return "☆☆☆☆";
  return "★".repeat(stars) + "☆".repeat(4 - stars);
}

export function formatReviewEntry(
  category: CuisineCategory,
  stars: StarRating,
  critique: string,
  dishName: string,
): string {
  const lines: string[] = [];
  lines.push(`${CATEGORY_DISPLAY[category]} — ${starDisplay(stars)}`);
  lines.push(`Featured: ${dishName}`);
  lines.push(critique);
  return lines.join("\n");
}

// ── Format complete food critic column ──────────────────────────────

export function formatFoodCriticColumn(
  entries: string[],
  criticName: string,
  dishOfDay: DishOfDay,
  editionNumber: number,
): FoodCriticColumn {
  return {
    sectionTitle: `DINING & CUISINE — Reviews by ${criticName} — Edition ${editionNumber}`,
    entries,
    dishOfDay,
    totalReviews: entries.length,
    editionNumber,
  };
}
