import { describe, expect, it } from "vitest";

import {
  classifyProblem,
  getEmotionalTone,
  generateLetterBody,
  generateAdviceResponse,
  formatAdviceEntry,
  formatAdviceColumn,
  type ProblemCategory,
  type EmotionalTone,
  type AdviceColumn,
} from "@/lib/game/adviceColumn";

// ── classifyProblem ──────────────────────────────────────────────────

describe("classifyProblem", () => {
  it("returns 'hunger' when food need is critically low", () => {
    expect(
      classifyProblem({ hunger: 10, rest: 80, health: 80 }, "thriving"),
    ).toBe("hunger");
  });

  it("returns 'overwork' when rest is critically low", () => {
    expect(
      classifyProblem({ hunger: 80, rest: 10, health: 80 }, "thriving"),
    ).toBe("overwork");
  });

  it("returns 'health' when health is critically low", () => {
    expect(
      classifyProblem({ hunger: 80, rest: 80, health: 10 }, "thriving"),
    ).toBe("health");
  });

  it("returns 'loneliness' when colony status is 'struggling' and needs are fine", () => {
    expect(
      classifyProblem({ hunger: 80, rest: 80, health: 80 }, "struggling"),
    ).toBe("loneliness");
  });

  it("returns 'territory' when colony status is 'critical'", () => {
    expect(
      classifyProblem({ hunger: 80, rest: 80, health: 80 }, "critical"),
    ).toBe("territory");
  });

  it("returns 'leadership' when colony status is 'new' and needs are fine", () => {
    expect(classifyProblem({ hunger: 80, rest: 80, health: 80 }, "new")).toBe(
      "leadership",
    );
  });

  it("prioritizes worst need when multiple are low", () => {
    // hunger at 5 is worse than rest at 15
    expect(
      classifyProblem({ hunger: 5, rest: 15, health: 20 }, "thriving"),
    ).toBe("hunger");
  });

  it("falls back to 'hunger' for perfect colony (no real problem)", () => {
    expect(
      classifyProblem({ hunger: 100, rest: 100, health: 100 }, "thriving"),
    ).toBe("hunger");
  });

  it("returns 'health' when health is lowest among all low needs", () => {
    expect(
      classifyProblem({ hunger: 30, rest: 30, health: 5 }, "thriving"),
    ).toBe("health");
  });

  it("returns 'overwork' when rest is lowest among low needs", () => {
    expect(
      classifyProblem({ hunger: 30, rest: 5, health: 30 }, "thriving"),
    ).toBe("overwork");
  });
});

// ── getEmotionalTone ─────────────────────────────────────────────────

describe("getEmotionalTone", () => {
  it("returns 'desperate' for very high severity (>=80)", () => {
    expect(getEmotionalTone("hunger", 90)).toBe("desperate");
  });

  it("returns 'worried' for moderate severity (50-79)", () => {
    expect(getEmotionalTone("hunger", 60)).toBe("worried");
  });

  it("returns 'curious' for low severity (<30)", () => {
    expect(getEmotionalTone("hunger", 20)).toBe("curious");
  });

  it("returns 'grumpy' for medium severity (30-49) with overwork", () => {
    expect(getEmotionalTone("overwork", 40)).toBe("grumpy");
  });

  it("returns 'grumpy' for medium severity with territory", () => {
    expect(getEmotionalTone("territory", 35)).toBe("grumpy");
  });

  it("returns 'worried' for medium severity with non-grumpy categories", () => {
    expect(getEmotionalTone("loneliness", 40)).toBe("worried");
  });

  it("returns 'desperate' at severity boundary 80", () => {
    expect(getEmotionalTone("health", 80)).toBe("desperate");
  });

  it("returns 'curious' at severity 0", () => {
    expect(getEmotionalTone("leadership", 0)).toBe("curious");
  });
});

// ── generateLetterBody ──────────────────────────────────────────────

describe("generateLetterBody", () => {
  it("produces a non-empty string", () => {
    const body = generateLetterBody("hunger", "desperate", "Whiskers", 42);
    expect(body.length).toBeGreaterThan(0);
  });

  it("is deterministic for the same seed", () => {
    const a = generateLetterBody("hunger", "worried", "Mittens", 123);
    const b = generateLetterBody("hunger", "worried", "Mittens", 123);
    expect(a).toBe(b);
  });

  it("varies with different seeds", () => {
    const a = generateLetterBody("overwork", "grumpy", "Patch", 10);
    const b = generateLetterBody("overwork", "grumpy", "Patch", 999);
    expect(a).not.toBe(b);
  });

  it("includes the cat name", () => {
    const body = generateLetterBody("loneliness", "worried", "Shadow", 42);
    expect(body).toContain("Shadow");
  });

  it("produces different letters for different problem categories", () => {
    const hunger = generateLetterBody("hunger", "worried", "Tom", 42);
    const health = generateLetterBody("health", "worried", "Tom", 42);
    expect(hunger).not.toBe(health);
  });

  it("produces different letters for different tones", () => {
    const desperate = generateLetterBody("hunger", "desperate", "Tom", 42);
    const curious = generateLetterBody("hunger", "curious", "Tom", 42);
    expect(desperate).not.toBe(curious);
  });
});

// ── generateAdviceResponse ──────────────────────────────────────────

describe("generateAdviceResponse", () => {
  it("produces a non-empty string", () => {
    const advice = generateAdviceResponse("hunger", "desperate", 42);
    expect(advice.length).toBeGreaterThan(0);
  });

  it("is deterministic for the same seed", () => {
    const a = generateAdviceResponse("health", "worried", 555);
    const b = generateAdviceResponse("health", "worried", 555);
    expect(a).toBe(b);
  });

  it("varies with different seeds", () => {
    const a = generateAdviceResponse("territory", "grumpy", 10);
    const b = generateAdviceResponse("territory", "grumpy", 999);
    expect(a).not.toBe(b);
  });

  it("produces different advice for different problem categories", () => {
    const hunger = generateAdviceResponse("hunger", "worried", 42);
    const leadership = generateAdviceResponse("leadership", "worried", 42);
    expect(hunger).not.toBe(leadership);
  });

  it("produces different advice for different tones", () => {
    const desperate = generateAdviceResponse("hunger", "desperate", 42);
    const curious = generateAdviceResponse("hunger", "curious", 42);
    expect(desperate).not.toBe(curious);
  });

  it("contains 'Tabby' signature reference", () => {
    const advice = generateAdviceResponse("overwork", "worried", 42);
    expect(advice.toLowerCase()).toContain("tabby");
  });
});

// ── formatAdviceEntry ───────────────────────────────────────────────

describe("formatAdviceEntry", () => {
  it("includes 'Dear Tabby' header", () => {
    const entry = formatAdviceEntry(
      "I am so hungry...",
      "Eat more mice, dear.",
      "Whiskers",
      "hunger",
    );
    expect(entry).toContain("Dear Tabby");
  });

  it("includes the cat name", () => {
    const entry = formatAdviceEntry(
      "My paws hurt...",
      "Rest them well.",
      "Mittens",
      "health",
    );
    expect(entry).toContain("Mittens");
  });

  it("includes the letter body", () => {
    const letter = "I have been working too hard lately...";
    const entry = formatAdviceEntry(letter, "Take a nap.", "Patch", "overwork");
    expect(entry).toContain(letter);
  });

  it("includes the advice response", () => {
    const advice = "Find a sunny windowsill and relax.";
    const entry = formatAdviceEntry("Help me!", advice, "Shadow", "health");
    expect(entry).toContain(advice);
  });

  it("includes the problem category", () => {
    const entry = formatAdviceEntry(
      "Where is everyone?",
      "Reach out to neighbours.",
      "Luna",
      "loneliness",
    );
    expect(entry.toLowerCase()).toContain("loneliness");
  });

  it("produces multi-line output", () => {
    const entry = formatAdviceEntry(
      "letter text",
      "advice text",
      "Tom",
      "hunger",
    );
    expect(entry.split("\n").length).toBeGreaterThan(1);
  });
});

// ── formatAdviceColumn ──────────────────────────────────────────────

describe("formatAdviceColumn", () => {
  it("returns correct structure with section title", () => {
    const column = formatAdviceColumn(["entry1", "entry2"], 7);
    expect(column.sectionTitle).toContain("DEAR TABBY");
    expect(column.totalEntries).toBe(2);
    expect(column.editionNumber).toBe(7);
    expect(column.entries).toHaveLength(2);
  });

  it("handles empty entries", () => {
    const column = formatAdviceColumn([], 1);
    expect(column.totalEntries).toBe(0);
    expect(column.entries).toHaveLength(0);
  });

  it("includes edition number in section title", () => {
    const column = formatAdviceColumn(["e1"], 42);
    expect(column.sectionTitle).toContain("42");
  });

  it("preserves all entries in order", () => {
    const entries = ["first", "second", "third"];
    const column = formatAdviceColumn(entries, 5);
    expect(column.entries).toEqual(entries);
  });

  it("returns AdviceColumn type shape", () => {
    const column: AdviceColumn = formatAdviceColumn(["x"], 1);
    expect(column).toHaveProperty("sectionTitle");
    expect(column).toHaveProperty("entries");
    expect(column).toHaveProperty("totalEntries");
    expect(column).toHaveProperty("editionNumber");
  });
});
