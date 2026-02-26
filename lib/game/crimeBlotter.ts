import { rollSeeded } from "./seededRng";

// ── Types ───────────────────────────────────────────────────────────

export type IncidentType =
  | "trespass"
  | "theft"
  | "mischief"
  | "territorial_dispute"
  | "vandalism";

export type SeverityLevel = "petty" | "minor" | "serious" | "major";

export type CaseStatus = "open" | "resolved" | "cold";

export interface Incident {
  type: IncidentType;
  severity: SeverityLevel;
  suspectDescription: string;
  witnessTestimony: string;
  caseStatus: CaseStatus;
  resourceLoss: number;
  casualties: number;
}

export interface CrimeBlotter {
  sectionTitle: string;
  entries: string[];
  totalIncidents: number;
  dayNumber: number;
}

// ── Event → Incident classification ─────────────────────────────────

const EVENT_TO_INCIDENT: Record<string, IncidentType> = {
  predator_attack: "trespass",
  encounter_lost: "trespass",
  encounter_won: "trespass",
  resource_lost: "theft",
  food_stolen: "theft",
  herb_stolen: "theft",
  kitten_chaos: "mischief",
  kitten_accident: "mischief",
  rival_cat: "territorial_dispute",
  territory_intrusion: "territorial_dispute",
  patrol_clash: "territorial_dispute",
  building_damaged: "vandalism",
  structure_destroyed: "vandalism",
};

export function classifyIncident(
  eventType: string,
  _enemyType?: string,
): IncidentType {
  return EVENT_TO_INCIDENT[eventType] ?? "mischief";
}

// ── Severity rating ─────────────────────────────────────────────────

export function rateSeverity(
  incidentType: IncidentType,
  resourceLoss: number,
  casualties: number,
): SeverityLevel {
  if (casualties > 0) return "major";
  if (resourceLoss >= 25) return "major";
  if (resourceLoss >= 10) return "serious";

  // Type-based floor: trespass and territorial disputes are at least minor
  if (incidentType === "trespass" || incidentType === "territorial_dispute") {
    if (resourceLoss > 0) return "minor";
    return "minor";
  }

  if (resourceLoss > 0) return "minor";

  // Mischief with no damage is petty
  if (incidentType === "mischief") return "petty";

  return "minor";
}

// ── Suspect descriptions ────────────────────────────────────────────

const ENEMY_DESCRIPTIONS: Record<string, string> = {
  fox: "A cunning fox with sharp eyes, last seen slinking along the colony perimeter",
  bear: "A large bear of considerable girth, reported lumbering near the food stores",
  hawk: "A shadow-casting hawk with razor talons, observed circling overhead",
  snake:
    "A silent snake, spotted weaving through the tall grass near the nursery",
  wolf: "A grey wolf with a scarred muzzle, tracked heading north from the colony",
  raccoon:
    "A masked raccoon with nimble paws, caught rummaging through supplies",
  owl: "A silent owl with piercing yellow eyes, spotted perched above the camp",
  coyote:
    "A lean coyote with a ragged coat, seen lurking at the edge of the territory",
};

export function generateSuspectDescription(
  enemyType?: string,
  catTraits?: { furColor?: string; marking?: string },
): string {
  if (enemyType && ENEMY_DESCRIPTIONS[enemyType]) {
    return ENEMY_DESCRIPTIONS[enemyType];
  }

  if (catTraits) {
    const parts: string[] = [];
    if (catTraits.furColor) parts.push(`${catTraits.furColor} feline`);
    if (catTraits.marking)
      parts.push(`distinguishing mark: ${catTraits.marking}`);
    return `A ${parts.join(", ")}` || "An unidentified cat of unknown origin";
  }

  if (enemyType) {
    return `An unidentified ${enemyType}, description pending further investigation`;
  }

  return "Suspect unknown — no witnesses have come forward with a description";
}

// ── Witness testimony ───────────────────────────────────────────────

const TESTIMONIES: Record<IncidentType, string[]> = {
  trespass: [
    "I heard rustling near the boundary stones just before dawn.",
    "There were strange tracks by the main gate this morning.",
    "Something large moved through the bushes — I could smell it.",
    "The kittens were spooked awake; that's when we knew something was out there.",
    "I was on night watch when I spotted a shadow crossing the clearing.",
  ],
  theft: [
    "The food pile was definitely bigger yesterday — someone's been at it.",
    "I caught a whiff of an outsider near the herb stores.",
    "Three fish, gone! Right under our whiskers!",
    "The morning count came up short. Again.",
    "Someone — or something — made off with our carefully dried herbs.",
  ],
  mischief: [
    "The little ones knocked over the water buckets. Again.",
    "There was yarn everywhere when I woke up.",
    "A kitten somehow got into the medicine stores. No harm done, thankfully.",
    "The nesting materials were scattered across camp like confetti.",
    "It was just kittens being kittens, but what a mess.",
  ],
  territorial_dispute: [
    "A strange cat was marking our territory — bold as you please.",
    "Two patrols nearly came to blows at the eastern boundary.",
    "The intruder hissed and refused to back down at first.",
    "There's been scent-marking along the north perimeter that isn't ours.",
    "A rival scout was spotted observing our camp from the ridge.",
  ],
  vandalism: [
    "The south wall had claw marks all over it this morning.",
    "Something tried to tear through the roof of the food store.",
    "The watchtower supports were gnawed — clearly deliberate.",
    "Our newest shelter was found partially collapsed.",
    "Scratch marks on every post — like something was testing our defences.",
  ],
};

export function generateWitnessTestimony(
  incidentType: IncidentType,
  _severity: SeverityLevel,
  seed: number,
): string {
  const pool = TESTIMONIES[incidentType];
  // Combine seed with incident type ordinal for per-type variation
  const typeOrdinal: Record<IncidentType, number> = {
    trespass: 1,
    theft: 2,
    mischief: 3,
    territorial_dispute: 4,
    vandalism: 5,
  };
  const combinedSeed = seed * 7919 + typeOrdinal[incidentType] * 131;
  const roll = rollSeeded(combinedSeed);
  const index = Math.floor(roll.value * pool.length);
  return pool[index];
}

// ── Format single blotter entry ─────────────────────────────────────

const INCIDENT_DISPLAY: Record<IncidentType, string> = {
  trespass: "TRESPASS",
  theft: "THEFT",
  mischief: "MISCHIEF",
  territorial_dispute: "TERRITORIAL DISPUTE",
  vandalism: "VANDALISM",
};

const SEVERITY_DISPLAY: Record<SeverityLevel, string> = {
  petty: "PETTY",
  minor: "MINOR",
  serious: "SERIOUS",
  major: "MAJOR",
};

const STATUS_DISPLAY: Record<CaseStatus, string> = {
  open: "OPEN",
  resolved: "RESOLVED",
  cold: "COLD CASE",
};

export function formatBlotterEntry(incident: Incident): string {
  const lines: string[] = [];
  lines.push(
    `${INCIDENT_DISPLAY[incident.type]} [${SEVERITY_DISPLAY[incident.severity]}]`,
  );
  lines.push(`Suspect: ${incident.suspectDescription}`);
  lines.push(`Witness: "${incident.witnessTestimony}"`);

  if (incident.resourceLoss > 0) {
    lines.push(`Estimated loss: ${incident.resourceLoss} units`);
  }
  if (incident.casualties > 0) {
    lines.push(
      `Casualties: ${incident.casualties} cat${incident.casualties > 1 ? "s" : ""}`,
    );
  }

  lines.push(`Case status: ${STATUS_DISPLAY[incident.caseStatus]}`);
  return lines.join("\n");
}

// ── Format complete crime blotter ───────────────────────────────────

export function formatCrimeBlotter(
  entries: string[],
  colonyName: string,
  dayNumber: number,
): CrimeBlotter {
  return {
    sectionTitle: `THE ${colonyName.toUpperCase()} POLICE GAZETTE — Day ${dayNumber}`,
    entries,
    totalIncidents: entries.length,
    dayNumber,
  };
}
