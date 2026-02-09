/**
 * Night Watch — shift-based guard scheduling tests
 */

import { describe, it, expect } from "vitest";
import {
  assignWatchShifts,
  calculateShiftCoverage,
  calculateWatchScore,
  identifyCoverageGaps,
  getWatchFatigueModifier,
} from "@/lib/game/nightWatch";
import type { WatchRoster, CoverageGap } from "@/lib/game/nightWatch";

// ---------- assignWatchShifts ----------

describe("assignWatchShifts", () => {
  it("returns all-zero roster for 0 guards", () => {
    const roster = assignWatchShifts(0);
    expect(roster).toEqual({ dusk: 0, midnight: 0, dawn: 0, totalGuards: 0 });
  });

  it("assigns 1 guard to midnight (highest priority)", () => {
    const roster = assignWatchShifts(1);
    expect(roster.midnight).toBe(1);
    expect(roster.dusk).toBe(0);
    expect(roster.dawn).toBe(0);
    expect(roster.totalGuards).toBe(1);
  });

  it("assigns 2 guards: midnight first, then dusk", () => {
    const roster = assignWatchShifts(2);
    expect(roster.midnight).toBe(1);
    expect(roster.dusk).toBe(1);
    expect(roster.dawn).toBe(0);
    expect(roster.totalGuards).toBe(2);
  });

  it("assigns 3 guards: one per shift", () => {
    const roster = assignWatchShifts(3);
    expect(roster.midnight).toBe(1);
    expect(roster.dusk).toBe(1);
    expect(roster.dawn).toBe(1);
    expect(roster.totalGuards).toBe(3);
  });

  it("assigns 6 guards evenly with midnight getting extras first", () => {
    const roster = assignWatchShifts(6);
    expect(roster.totalGuards).toBe(6);
    expect(roster.midnight).toBeGreaterThanOrEqual(roster.dusk);
    expect(roster.midnight).toBeGreaterThanOrEqual(roster.dawn);
    expect(roster.dusk + roster.midnight + roster.dawn).toBe(6);
  });

  it("assigns 10 guards with midnight priority", () => {
    const roster = assignWatchShifts(10);
    expect(roster.totalGuards).toBe(10);
    expect(roster.midnight).toBeGreaterThanOrEqual(roster.dusk);
    expect(roster.midnight).toBeGreaterThanOrEqual(roster.dawn);
    expect(roster.dusk + roster.midnight + roster.dawn).toBe(10);
  });

  it("handles large numbers with even distribution", () => {
    const roster = assignWatchShifts(99);
    expect(roster.totalGuards).toBe(99);
    expect(roster.dusk + roster.midnight + roster.dawn).toBe(99);
    // Midnight should have at most 1 more than others
    expect(roster.midnight - roster.dusk).toBeLessThanOrEqual(1);
    expect(roster.midnight - roster.dawn).toBeLessThanOrEqual(1);
  });
});

// ---------- calculateShiftCoverage ----------

describe("calculateShiftCoverage", () => {
  it("returns 0 for 0 guards on any shift", () => {
    expect(calculateShiftCoverage(0, "dusk")).toBe(0);
    expect(calculateShiftCoverage(0, "midnight")).toBe(0);
    expect(calculateShiftCoverage(0, "dawn")).toBe(0);
  });

  it("returns 100 for 3+ guards on dusk", () => {
    expect(calculateShiftCoverage(3, "dusk")).toBe(100);
  });

  it("returns 100 for 3+ guards on dawn", () => {
    expect(calculateShiftCoverage(3, "dawn")).toBe(100);
  });

  it("returns 100 for 3+ guards on midnight (sufficient)", () => {
    expect(calculateShiftCoverage(3, "midnight")).toBe(100);
  });

  it("gives midnight lower coverage per guard than other shifts", () => {
    const midnightScore = calculateShiftCoverage(1, "midnight");
    const duskScore = calculateShiftCoverage(1, "dusk");
    const dawnScore = calculateShiftCoverage(1, "dawn");
    expect(midnightScore).toBeLessThan(duskScore);
    expect(midnightScore).toBeLessThan(dawnScore);
  });

  it("scales linearly for partial coverage", () => {
    const one = calculateShiftCoverage(1, "dusk");
    const two = calculateShiftCoverage(2, "dusk");
    expect(two).toBeGreaterThan(one);
    expect(two).toBeLessThanOrEqual(100);
  });

  it("caps at 100 even with many guards", () => {
    expect(calculateShiftCoverage(10, "dusk")).toBe(100);
    expect(calculateShiftCoverage(10, "midnight")).toBe(100);
  });
});

// ---------- calculateWatchScore ----------

describe("calculateWatchScore", () => {
  it("returns 0 for empty roster", () => {
    const roster: WatchRoster = {
      dusk: 0,
      midnight: 0,
      dawn: 0,
      totalGuards: 0,
    };
    expect(calculateWatchScore(roster)).toBe(0);
  });

  it("weights midnight double in the average", () => {
    // Only midnight covered
    const midnightOnly: WatchRoster = {
      dusk: 0,
      midnight: 3,
      dawn: 0,
      totalGuards: 3,
    };
    // Only dusk covered
    const duskOnly: WatchRoster = {
      dusk: 3,
      midnight: 0,
      dawn: 0,
      totalGuards: 3,
    };

    const midScore = calculateWatchScore(midnightOnly);
    const duskScore = calculateWatchScore(duskOnly);
    // Midnight coverage contributes more to total score
    expect(midScore).toBeGreaterThan(duskScore);
  });

  it("returns 100 for fully covered roster", () => {
    const full: WatchRoster = { dusk: 3, midnight: 3, dawn: 3, totalGuards: 9 };
    expect(calculateWatchScore(full)).toBe(100);
  });

  it("returns a value between 0 and 100", () => {
    const partial: WatchRoster = {
      dusk: 1,
      midnight: 1,
      dawn: 1,
      totalGuards: 3,
    };
    const score = calculateWatchScore(partial);
    expect(score).toBeGreaterThan(0);
    expect(score).toBeLessThanOrEqual(100);
  });
});

// ---------- identifyCoverageGaps ----------

describe("identifyCoverageGaps", () => {
  it("returns empty array when all shifts adequately covered", () => {
    const roster: WatchRoster = {
      dusk: 3,
      midnight: 3,
      dawn: 3,
      totalGuards: 9,
    };
    expect(identifyCoverageGaps(roster)).toEqual([]);
  });

  it("identifies shift with 0 guards as critical", () => {
    const roster: WatchRoster = {
      dusk: 3,
      midnight: 0,
      dawn: 3,
      totalGuards: 6,
    };
    const gaps = identifyCoverageGaps(roster);
    const midnightGap = gaps.find((g) => g.shift === "midnight");
    expect(midnightGap).toBeDefined();
    expect(midnightGap!.severity).toBe("critical");
    expect(midnightGap!.guardsAssigned).toBe(0);
  });

  it("identifies shift with 1 guard as major", () => {
    const roster: WatchRoster = {
      dusk: 1,
      midnight: 3,
      dawn: 3,
      totalGuards: 7,
    };
    const gaps = identifyCoverageGaps(roster);
    const duskGap = gaps.find((g) => g.shift === "dusk");
    expect(duskGap).toBeDefined();
    expect(duskGap!.severity).toBe("major");
    expect(duskGap!.guardsAssigned).toBe(1);
  });

  it("identifies midnight gaps with critical severity even for 1 guard", () => {
    const roster: WatchRoster = {
      dusk: 3,
      midnight: 1,
      dawn: 3,
      totalGuards: 7,
    };
    const gaps = identifyCoverageGaps(roster);
    const midnightGap = gaps.find((g) => g.shift === "midnight");
    expect(midnightGap).toBeDefined();
    // Midnight with only 1 guard should be critical (higher severity than other shifts)
    expect(midnightGap!.severity).toBe("critical");
  });

  it("returns multiple gaps when multiple shifts are under-covered", () => {
    const roster: WatchRoster = {
      dusk: 0,
      midnight: 0,
      dawn: 0,
      totalGuards: 0,
    };
    const gaps = identifyCoverageGaps(roster);
    expect(gaps).toHaveLength(3);
    expect(gaps.every((g) => g.severity === "critical")).toBe(true);
  });

  it("includes guardsNeeded in gap info", () => {
    const roster: WatchRoster = {
      dusk: 1,
      midnight: 0,
      dawn: 3,
      totalGuards: 4,
    };
    const gaps = identifyCoverageGaps(roster);
    for (const gap of gaps) {
      expect(gap.guardsNeeded).toBeGreaterThan(gap.guardsAssigned);
    }
  });

  it("identifies shift with 2 guards as minor", () => {
    const roster: WatchRoster = {
      dusk: 2,
      midnight: 3,
      dawn: 3,
      totalGuards: 8,
    };
    const gaps = identifyCoverageGaps(roster);
    const duskGap = gaps.find((g) => g.shift === "dusk");
    expect(duskGap).toBeDefined();
    expect(duskGap!.severity).toBe("minor");
  });
});

// ---------- getWatchFatigueModifier ----------

describe("getWatchFatigueModifier", () => {
  it("returns 0 for 0 shifts (no work, no fatigue)", () => {
    expect(getWatchFatigueModifier(0)).toBe(0);
  });

  it("returns 1.0 for 1 shift (normal)", () => {
    expect(getWatchFatigueModifier(1)).toBe(1.0);
  });

  it("returns 1.3 for 2 shifts (tired)", () => {
    expect(getWatchFatigueModifier(2)).toBe(1.3);
  });

  it("returns 1.6 for 3 shifts (exhausted)", () => {
    expect(getWatchFatigueModifier(3)).toBe(1.6);
  });

  it("caps at 1.6 for more than 3 shifts", () => {
    expect(getWatchFatigueModifier(5)).toBe(1.6);
  });
});
