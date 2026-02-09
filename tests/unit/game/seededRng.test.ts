import { describe, expect, it } from "vitest";

import { normalizeSeed, rollSeeded } from "@/lib/game/seededRng";

describe("seededRng", () => {
  it("normalizes invalid seeds to a stable fallback", () => {
    expect(normalizeSeed(Number.NaN)).toBe(1);
    expect(normalizeSeed(Number.POSITIVE_INFINITY)).toBe(1);
  });

  it("normalizes to unsigned integer space", () => {
    expect(normalizeSeed(-42.8)).toBe(42);
    expect(normalizeSeed(42.8)).toBe(42);
  });

  it("produces deterministic sequence for same seed", () => {
    const first = rollSeeded(123);
    const second = rollSeeded(123);

    expect(first.value).toBe(second.value);
    expect(first.nextSeed).toBe(second.nextSeed);
  });

  it("returns values in [0, 1)", () => {
    const { value } = rollSeeded(999);
    expect(value).toBeGreaterThanOrEqual(0);
    expect(value).toBeLessThan(1);
  });

  it("normalizes seed 0 to 1 (avoids degenerate LCG state)", () => {
    expect(normalizeSeed(0)).toBe(1);
  });

  it("produces divergent values when chaining", () => {
    const first = rollSeeded(123);
    const second = rollSeeded(first.nextSeed);
    expect(second.value).not.toBe(first.value);
  });
});
