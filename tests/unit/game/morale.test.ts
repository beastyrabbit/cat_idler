import { describe, it, expect } from "vitest";
import {
  computeColonyMorale,
  getMoraleLabel,
  type MoraleInput,
  type MoraleResult,
} from "@/lib/game/morale";
import type { CatNeeds, Resources } from "@/types/game";

/* ─── helpers ──────────────────────────────────── */

function makeNeeds(overrides: Partial<CatNeeds> = {}): CatNeeds {
  return { hunger: 80, thirst: 80, rest: 80, health: 100, ...overrides };
}

function makeResources(overrides: Partial<Resources> = {}): Resources {
  return {
    food: 50,
    water: 50,
    herbs: 10,
    materials: 10,
    blessings: 5,
    ...overrides,
  };
}

function makeInput(overrides: Partial<MoraleInput> = {}): MoraleInput {
  return {
    catNeedsArray: [makeNeeds(), makeNeeds(), makeNeeds()],
    resources: makeResources(),
    aliveCount: 3,
    deadCount: 0,
    ...overrides,
  };
}

/* ─── getMoraleLabel ───────────────────────────── */

describe("getMoraleLabel", () => {
  it("returns Euphoric for score 80-100", () => {
    expect(getMoraleLabel(80)).toBe("Euphoric");
    expect(getMoraleLabel(100)).toBe("Euphoric");
    expect(getMoraleLabel(95)).toBe("Euphoric");
  });

  it("returns Content for score 60-79", () => {
    expect(getMoraleLabel(60)).toBe("Content");
    expect(getMoraleLabel(79)).toBe("Content");
  });

  it("returns Uneasy for score 40-59", () => {
    expect(getMoraleLabel(40)).toBe("Uneasy");
    expect(getMoraleLabel(59)).toBe("Uneasy");
  });

  it("returns Distressed for score 20-39", () => {
    expect(getMoraleLabel(20)).toBe("Distressed");
    expect(getMoraleLabel(39)).toBe("Distressed");
  });

  it("returns Despairing for score 0-19", () => {
    expect(getMoraleLabel(0)).toBe("Despairing");
    expect(getMoraleLabel(19)).toBe("Despairing");
  });

  it("returns Despairing for negative score (fallback)", () => {
    expect(getMoraleLabel(-1)).toBe("Despairing");
    expect(getMoraleLabel(-100)).toBe("Despairing");
  });
});

/* ─── computeColonyMorale ──────────────────────── */

describe("computeColonyMorale", () => {
  it("returns a result with score, label, and description", () => {
    const result = computeColonyMorale(makeInput());
    expect(result).toHaveProperty("score");
    expect(result).toHaveProperty("label");
    expect(result).toHaveProperty("description");
    expect(typeof result.score).toBe("number");
    expect(typeof result.label).toBe("string");
    expect(typeof result.description).toBe("string");
  });

  it("clamps score between 0 and 100", () => {
    const perfect = computeColonyMorale(
      makeInput({
        catNeedsArray: [
          makeNeeds({ hunger: 100, thirst: 100, rest: 100, health: 100 }),
        ],
        resources: makeResources({
          food: 200,
          water: 200,
          herbs: 50,
          materials: 50,
          blessings: 20,
        }),
        aliveCount: 5,
        deadCount: 0,
      }),
    );
    expect(perfect.score).toBeLessThanOrEqual(100);
    expect(perfect.score).toBeGreaterThanOrEqual(0);

    const dire = computeColonyMorale(
      makeInput({
        catNeedsArray: [
          makeNeeds({ hunger: 0, thirst: 0, rest: 0, health: 5 }),
        ],
        resources: makeResources({
          food: 0,
          water: 0,
          herbs: 0,
          materials: 0,
          blessings: 0,
        }),
        aliveCount: 1,
        deadCount: 10,
      }),
    );
    expect(dire.score).toBeLessThanOrEqual(100);
    expect(dire.score).toBeGreaterThanOrEqual(0);
  });

  it("returns high morale for a healthy colony with good resources", () => {
    const result = computeColonyMorale(
      makeInput({
        catNeedsArray: [
          makeNeeds({ hunger: 90, thirst: 90, rest: 85, health: 100 }),
          makeNeeds({ hunger: 85, thirst: 95, rest: 90, health: 100 }),
          makeNeeds({ hunger: 95, thirst: 88, rest: 80, health: 100 }),
        ],
        resources: makeResources({
          food: 80,
          water: 80,
          herbs: 20,
          materials: 20,
          blessings: 10,
        }),
        aliveCount: 3,
        deadCount: 0,
      }),
    );
    expect(result.score).toBeGreaterThanOrEqual(60);
    expect(["Euphoric", "Content"]).toContain(result.label);
  });

  it("returns low morale when cats are starving and dehydrated", () => {
    const result = computeColonyMorale(
      makeInput({
        catNeedsArray: [
          makeNeeds({ hunger: 0, thirst: 0, rest: 10, health: 20 }),
          makeNeeds({ hunger: 5, thirst: 0, rest: 15, health: 30 }),
        ],
        resources: makeResources({
          food: 0,
          water: 0,
          herbs: 0,
          materials: 0,
          blessings: 0,
        }),
        aliveCount: 2,
        deadCount: 5,
      }),
    );
    expect(result.score).toBeLessThan(30);
    expect(["Distressed", "Despairing"]).toContain(result.label);
  });

  it("handles empty colony (no cats alive)", () => {
    const result = computeColonyMorale(
      makeInput({
        catNeedsArray: [],
        resources: makeResources({ food: 0, water: 0 }),
        aliveCount: 0,
        deadCount: 3,
      }),
    );
    expect(result.score).toBe(0);
    expect(result.label).toBe("Despairing");
  });

  it("factors in recent deaths negatively", () => {
    const withoutDeaths = computeColonyMorale(makeInput({ deadCount: 0 }));
    const withDeaths = computeColonyMorale(makeInput({ deadCount: 5 }));
    expect(withDeaths.score).toBeLessThan(withoutDeaths.score);
  });

  it("factors in resource abundance positively", () => {
    const lowResources = computeColonyMorale(
      makeInput({
        resources: makeResources({
          food: 5,
          water: 5,
          herbs: 0,
          materials: 0,
          blessings: 0,
        }),
      }),
    );
    const highResources = computeColonyMorale(
      makeInput({
        resources: makeResources({
          food: 100,
          water: 100,
          herbs: 30,
          materials: 30,
          blessings: 15,
        }),
      }),
    );
    expect(highResources.score).toBeGreaterThan(lowResources.score);
  });

  it("label matches the score tier", () => {
    const result = computeColonyMorale(makeInput());
    expect(result.label).toBe(getMoraleLabel(result.score));
  });

  it("description is non-empty", () => {
    const result = computeColonyMorale(makeInput());
    expect(result.description.length).toBeGreaterThan(0);
  });

  it("handles aliveCount > 0 but empty needs array gracefully", () => {
    const result = computeColonyMorale(
      makeInput({
        catNeedsArray: [],
        aliveCount: 2,
        deadCount: 0,
      }),
    );
    expect(result.score).toBeGreaterThanOrEqual(0);
    expect(result.score).toBeLessThanOrEqual(100);
  });

  it("single cat colony works correctly", () => {
    const result = computeColonyMorale(
      makeInput({
        catNeedsArray: [makeNeeds()],
        aliveCount: 1,
        deadCount: 0,
      }),
    );
    expect(result.score).toBeGreaterThanOrEqual(0);
    expect(result.score).toBeLessThanOrEqual(100);
  });
});
