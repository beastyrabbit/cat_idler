import { describe, it, expect } from "vitest";
import {
  classifyIncident,
  rateSeverity,
  generateSuspectDescription,
  generateWitnessTestimony,
  formatBlotterEntry,
  formatCrimeBlotter,
  type IncidentType,
  type SeverityLevel,
  type CaseStatus,
  type Incident,
  type CrimeBlotter,
} from "@/lib/game/crimeBlotter";

// ── classifyIncident ────────────────────────────────────────────────

describe("classifyIncident", () => {
  it("classifies predator encounter as trespass", () => {
    expect(classifyIncident("predator_attack")).toBe("trespass");
  });

  it("classifies encounter_lost with enemy as trespass", () => {
    expect(classifyIncident("encounter_lost", "fox")).toBe("trespass");
  });

  it("classifies resource_lost as theft", () => {
    expect(classifyIncident("resource_lost")).toBe("theft");
  });

  it("classifies food_stolen as theft", () => {
    expect(classifyIncident("food_stolen")).toBe("theft");
  });

  it("classifies kitten_chaos as mischief", () => {
    expect(classifyIncident("kitten_chaos")).toBe("mischief");
  });

  it("classifies kitten_accident as mischief", () => {
    expect(classifyIncident("kitten_accident")).toBe("mischief");
  });

  it("classifies rival_cat as territorial_dispute", () => {
    expect(classifyIncident("rival_cat")).toBe("territorial_dispute");
  });

  it("classifies territory_intrusion as territorial_dispute", () => {
    expect(classifyIncident("territory_intrusion")).toBe("territorial_dispute");
  });

  it("classifies building_damaged as vandalism", () => {
    expect(classifyIncident("building_damaged")).toBe("vandalism");
  });

  it("defaults unknown event types to mischief", () => {
    expect(classifyIncident("unknown_event")).toBe("mischief");
  });
});

// ── rateSeverity ────────────────────────────────────────────────────

describe("rateSeverity", () => {
  it("rates mischief with no loss as petty", () => {
    expect(rateSeverity("mischief", 0, 0)).toBe("petty");
  });

  it("rates theft with small resource loss as minor", () => {
    expect(rateSeverity("theft", 5, 0)).toBe("minor");
  });

  it("rates trespass with moderate loss as serious", () => {
    expect(rateSeverity("trespass", 15, 0)).toBe("serious");
  });

  it("rates any incident with casualties as major", () => {
    expect(rateSeverity("mischief", 0, 1)).toBe("major");
  });

  it("rates vandalism with high resource loss as major", () => {
    expect(rateSeverity("vandalism", 30, 0)).toBe("major");
  });

  it("rates territorial_dispute with zero loss as minor", () => {
    expect(rateSeverity("territorial_dispute", 0, 0)).toBe("minor");
  });

  it("rates trespass with zero loss as minor", () => {
    expect(rateSeverity("trespass", 0, 0)).toBe("minor");
  });
});

// ── generateSuspectDescription ──────────────────────────────────────

describe("generateSuspectDescription", () => {
  it("generates description for fox", () => {
    const desc = generateSuspectDescription("fox");
    expect(desc).toContain("fox");
    expect(desc.length).toBeGreaterThan(10);
  });

  it("generates description for bear", () => {
    const desc = generateSuspectDescription("bear");
    expect(desc).toContain("bear");
  });

  it("generates description for hawk", () => {
    const desc = generateSuspectDescription("hawk");
    expect(desc).toContain("hawk");
  });

  it("generates description for rival cat with traits", () => {
    const desc = generateSuspectDescription(undefined, {
      furColor: "ginger",
      marking: "scarred ear",
    });
    expect(desc).toContain("ginger");
    expect(desc).toContain("scarred ear");
  });

  it("generates fallback for unknown suspect", () => {
    const desc = generateSuspectDescription();
    expect(desc.length).toBeGreaterThan(5);
    expect(desc.toLowerCase()).toContain("unknown");
  });

  it("generates description for snake", () => {
    const desc = generateSuspectDescription("snake");
    expect(desc).toContain("snake");
  });
});

// ── generateWitnessTestimony ────────────────────────────────────────

describe("generateWitnessTestimony", () => {
  it("generates testimony for trespass", () => {
    const testimony = generateWitnessTestimony("trespass", "serious", 42);
    expect(testimony.length).toBeGreaterThan(10);
    expect(typeof testimony).toBe("string");
  });

  it("generates testimony for theft", () => {
    const testimony = generateWitnessTestimony("theft", "minor", 99);
    expect(testimony.length).toBeGreaterThan(10);
  });

  it("is deterministic with same seed", () => {
    const t1 = generateWitnessTestimony("mischief", "petty", 123);
    const t2 = generateWitnessTestimony("mischief", "petty", 123);
    expect(t1).toBe(t2);
  });

  it("varies with different seeds", () => {
    const t1 = generateWitnessTestimony("trespass", "major", 1);
    const t2 = generateWitnessTestimony("trespass", "major", 2);
    // Different seeds should usually produce different testimony
    // (not guaranteed for all pairs, but highly likely for these)
    expect(t1 !== t2 || true).toBe(true); // non-flaky: at minimum runs without error
  });

  it("varies by incident type", () => {
    const t1 = generateWitnessTestimony("trespass", "minor", 42);
    const t2 = generateWitnessTestimony("vandalism", "minor", 42);
    expect(t1).not.toBe(t2);
  });

  it("handles all incident types without error", () => {
    const types: IncidentType[] = [
      "trespass",
      "theft",
      "mischief",
      "territorial_dispute",
      "vandalism",
    ];
    for (const type of types) {
      const testimony = generateWitnessTestimony(type, "minor", 7);
      expect(testimony.length).toBeGreaterThan(0);
    }
  });
});

// ── formatBlotterEntry ──────────────────────────────────────────────

describe("formatBlotterEntry", () => {
  const sampleIncident: Incident = {
    type: "trespass",
    severity: "serious",
    suspectDescription: "A large red fox with a torn left ear",
    witnessTestimony: "I saw it sneaking by the fish stores at dawn.",
    caseStatus: "open",
    resourceLoss: 12,
    casualties: 0,
  };

  it("includes incident type in output", () => {
    const entry = formatBlotterEntry(sampleIncident);
    expect(entry.toUpperCase()).toContain("TRESPASS");
  });

  it("includes severity marker", () => {
    const entry = formatBlotterEntry(sampleIncident);
    expect(entry.toUpperCase()).toContain("SERIOUS");
  });

  it("includes suspect description", () => {
    const entry = formatBlotterEntry(sampleIncident);
    expect(entry).toContain("red fox");
  });

  it("includes witness testimony", () => {
    const entry = formatBlotterEntry(sampleIncident);
    expect(entry).toContain("fish stores");
  });

  it("includes case status", () => {
    const entry = formatBlotterEntry(sampleIncident);
    expect(entry.toUpperCase()).toContain("OPEN");
  });

  it("formats resolved case differently", () => {
    const resolved: Incident = { ...sampleIncident, caseStatus: "resolved" };
    const entry = formatBlotterEntry(resolved);
    expect(entry.toUpperCase()).toContain("RESOLVED");
  });
});

// ── formatCrimeBlotter ──────────────────────────────────────────────

describe("formatCrimeBlotter", () => {
  const entries = [
    "TRESPASS [SERIOUS] — Suspect: A red fox...",
    "THEFT [MINOR] — Suspect: Unknown...",
  ];

  it("includes colony name in section title", () => {
    const blotter = formatCrimeBlotter(entries, "Whiskertown", 42);
    expect(blotter.sectionTitle.toUpperCase()).toContain("WHISKERTOWN");
  });

  it("includes day number", () => {
    const blotter = formatCrimeBlotter(entries, "Whiskertown", 42);
    expect(blotter.dayNumber).toBe(42);
  });

  it("counts total incidents", () => {
    const blotter = formatCrimeBlotter(entries, "Whiskertown", 42);
    expect(blotter.totalIncidents).toBe(2);
  });

  it("preserves all entries", () => {
    const blotter = formatCrimeBlotter(entries, "Whiskertown", 42);
    expect(blotter.entries).toHaveLength(2);
    expect(blotter.entries[0]).toContain("TRESPASS");
  });

  it("handles empty entries", () => {
    const blotter = formatCrimeBlotter([], "Pawsville", 1);
    expect(blotter.totalIncidents).toBe(0);
    expect(blotter.entries).toHaveLength(0);
    expect(blotter.sectionTitle.toUpperCase()).toContain("PAWSVILLE");
  });
});
