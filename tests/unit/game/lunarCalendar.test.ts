import { describe, it, expect } from "vitest";
import {
  calculateMoonDay,
  getMoonPhase,
  getMoonPhaseModifiers,
  getMoonPhaseName,
  evaluateLunarInfluence,
  generateLunarAlmanac,
} from "@/lib/game/lunarCalendar";

describe("calculateMoonDay", () => {
  it("returns 0 for day 0", () => {
    expect(calculateMoonDay(0)).toBe(0);
  });

  it("returns 0-29 for any positive day number", () => {
    for (const day of [1, 15, 29, 30, 59, 100, 999]) {
      const result = calculateMoonDay(day);
      expect(result).toBeGreaterThanOrEqual(0);
      expect(result).toBeLessThanOrEqual(29);
    }
  });

  it("wraps correctly (day 30 → 0, day 60 → 0)", () => {
    expect(calculateMoonDay(30)).toBe(0);
    expect(calculateMoonDay(60)).toBe(0);
  });

  it("handles negative input (clamps to 0)", () => {
    expect(calculateMoonDay(-5)).toBe(0);
    expect(calculateMoonDay(-100)).toBe(0);
  });
});

describe("getMoonPhase", () => {
  it('returns "new_moon" for days 0-3', () => {
    for (const day of [0, 1, 2, 3]) {
      expect(getMoonPhase(day)).toBe("new_moon");
    }
  });

  it('returns "waxing_crescent" for days 4-7', () => {
    for (const day of [4, 5, 6, 7]) {
      expect(getMoonPhase(day)).toBe("waxing_crescent");
    }
  });

  it('returns "first_quarter" for days 8-10', () => {
    for (const day of [8, 9, 10]) {
      expect(getMoonPhase(day)).toBe("first_quarter");
    }
  });

  it('returns "waxing_gibbous" for days 11-14', () => {
    for (const day of [11, 12, 13, 14]) {
      expect(getMoonPhase(day)).toBe("waxing_gibbous");
    }
  });

  it('returns "full_moon" for days 15-18', () => {
    for (const day of [15, 16, 17, 18]) {
      expect(getMoonPhase(day)).toBe("full_moon");
    }
  });

  it('returns "waning_gibbous" for days 19-22', () => {
    for (const day of [19, 20, 21, 22]) {
      expect(getMoonPhase(day)).toBe("waning_gibbous");
    }
  });

  it('returns "last_quarter" for days 23-25', () => {
    for (const day of [23, 24, 25]) {
      expect(getMoonPhase(day)).toBe("last_quarter");
    }
  });

  it('returns "waning_crescent" for days 26-29', () => {
    for (const day of [26, 27, 28, 29]) {
      expect(getMoonPhase(day)).toBe("waning_crescent");
    }
  });
});

describe("getMoonPhaseModifiers", () => {
  it("returns correct modifiers for each phase", () => {
    const newMoon = getMoonPhaseModifiers("new_moon");
    expect(newMoon.stealthBonus).toBe(30);
    expect(newMoon.huntingBonus).toBe(-10);

    const fullMoon = getMoonPhaseModifiers("full_moon");
    expect(fullMoon.huntingBonus).toBe(30);
    expect(fullMoon.breedingBonus).toBe(20);
    expect(fullMoon.moodBonus).toBe(-10);

    const waxingCrescent = getMoonPhaseModifiers("waxing_crescent");
    expect(waxingCrescent.moodBonus).toBe(10);

    const waxingGibbous = getMoonPhaseModifiers("waxing_gibbous");
    expect(waxingGibbous.huntingBonus).toBe(15);

    const waningGibbous = getMoonPhaseModifiers("waning_gibbous");
    expect(waningGibbous.restBonus).toBe(20);

    const waningCrescent = getMoonPhaseModifiers("waning_crescent");
    expect(waningCrescent.stealthBonus).toBe(15);
    expect(waningCrescent.moodBonus).toBe(-5);
  });

  it("returns all-zero modifiers for balanced phases", () => {
    for (const phase of ["first_quarter", "last_quarter"] as const) {
      const mods = getMoonPhaseModifiers(phase);
      expect(mods.huntingBonus).toBe(0);
      expect(mods.stealthBonus).toBe(0);
      expect(mods.breedingBonus).toBe(0);
      expect(mods.moodBonus).toBe(0);
      expect(mods.restBonus).toBe(0);
    }
  });
});

describe("getMoonPhaseName", () => {
  it("returns human-readable name for each phase", () => {
    expect(getMoonPhaseName("new_moon")).toBe("New Moon");
    expect(getMoonPhaseName("waxing_crescent")).toBe("Waxing Crescent");
    expect(getMoonPhaseName("first_quarter")).toBe("First Quarter");
    expect(getMoonPhaseName("waxing_gibbous")).toBe("Waxing Gibbous");
    expect(getMoonPhaseName("full_moon")).toBe("Full Moon");
    expect(getMoonPhaseName("waning_gibbous")).toBe("Waning Gibbous");
    expect(getMoonPhaseName("last_quarter")).toBe("Last Quarter");
    expect(getMoonPhaseName("waning_crescent")).toBe("Waning Crescent");
  });
});

describe("evaluateLunarInfluence", () => {
  it("calculates days until full moon", () => {
    // Day 0 = new moon, full moon starts at day 15
    const report = evaluateLunarInfluence(0, 10);
    expect(report.daysUntilFullMoon).toBe(15);
  });

  it("identifies dominant effect", () => {
    // Full moon: hunting +30 is the biggest modifier
    const fullMoonReport = evaluateLunarInfluence(15, 10);
    expect(fullMoonReport.dominantEffect).toContain("hunting");

    // New moon: stealth +30 is the biggest modifier
    const newMoonReport = evaluateLunarInfluence(0, 10);
    expect(newMoonReport.dominantEffect).toContain("stealth");
  });

  it("handles day 0 (new moon start)", () => {
    const report = evaluateLunarInfluence(0, 5);
    expect(report.currentPhase).toBe("new_moon");
    expect(report.phaseName).toBe("New Moon");
    expect(report.dayInCycle).toBe(0);
    expect(report.affectedCats).toBe(5);
  });

  it("handles full moon day", () => {
    const report = evaluateLunarInfluence(15, 8);
    expect(report.currentPhase).toBe("full_moon");
    expect(report.daysUntilFullMoon).toBe(0);
    expect(report.affectedCats).toBe(8);
  });
});

describe("generateLunarAlmanac", () => {
  it("includes colony name", () => {
    const report = evaluateLunarInfluence(5, 10);
    const column = generateLunarAlmanac(report, "Whiskertown");
    expect(column).toContain("Whiskertown");
  });

  it("handles full moon specially (dramatic text)", () => {
    const report = evaluateLunarInfluence(15, 10);
    const column = generateLunarAlmanac(report, "Catford");
    expect(column.toLowerCase()).toMatch(/full moon/);
    // Should have dramatic/special text for full moon
    expect(column.length).toBeGreaterThan(50);
  });

  it("handles new moon specially", () => {
    const report = evaluateLunarInfluence(0, 10);
    const column = generateLunarAlmanac(report, "Catford");
    expect(column.toLowerCase()).toMatch(/new moon/);
  });

  it("includes modifier summary and days until next full moon", () => {
    const report = evaluateLunarInfluence(5, 10);
    const column = generateLunarAlmanac(report, "Catford");
    // Should mention modifiers or effects
    expect(column.length).toBeGreaterThan(30);
    // Should show days until full moon
    expect(column).toMatch(/\d+/);
  });
});
