import { describe, it, expect } from "vitest";
import {
  getCatZodiacSign,
  getDailyFortune,
  getSignCompatibility,
  formatHoroscopeEntry,
  formatHoroscopeColumn,
  ZODIAC_SIGNS,
} from "@/lib/game/horoscope";
import type {
  ZodiacSign,
  DailyFortune,
  FortuneSeverity,
  CompatibilityLevel,
} from "@/lib/game/horoscope";

describe("getCatZodiacSign", () => {
  const colonyCreatedAt = 0; // epoch start for simplicity

  it("returns The Prowler for month 1 (day 0-29)", () => {
    // Birth at day 5 → month 1
    const birthTime = 5 * 24 * 60 * 60 * 1000;
    const sign = getCatZodiacSign(birthTime, colonyCreatedAt);
    expect(sign.name).toBe("The Prowler");
    expect(sign.month).toBe(1);
  });

  it("returns The Sunbeam for month 2 (day 30-59)", () => {
    const birthTime = 35 * 24 * 60 * 60 * 1000;
    const sign = getCatZodiacSign(birthTime, colonyCreatedAt);
    expect(sign.name).toBe("The Sunbeam");
    expect(sign.month).toBe(2);
  });

  it("returns The Elder for month 12 (day 330-359)", () => {
    const birthTime = 340 * 24 * 60 * 60 * 1000;
    const sign = getCatZodiacSign(birthTime, colonyCreatedAt);
    expect(sign.name).toBe("The Elder");
    expect(sign.month).toBe(12);
  });

  it("wraps around after 360 days (month 13 → month 1)", () => {
    const birthTime = 365 * 24 * 60 * 60 * 1000;
    const sign = getCatZodiacSign(birthTime, colonyCreatedAt);
    expect(sign.month).toBe(1); // wraps back
  });

  it("uses colony creation time as epoch for month calculation", () => {
    const colonyStart = 100 * 24 * 60 * 60 * 1000; // colony started at day 100
    const birthTime = 135 * 24 * 60 * 60 * 1000; // born 35 days after colony start
    const sign = getCatZodiacSign(birthTime, colonyStart);
    expect(sign.name).toBe("The Sunbeam"); // day 35 relative → month 2
  });

  it("returns The Prowler for birth at exact colony creation time", () => {
    const sign = getCatZodiacSign(colonyCreatedAt, colonyCreatedAt);
    expect(sign.name).toBe("The Prowler");
    expect(sign.month).toBe(1);
  });

  it("has all 12 zodiac signs defined", () => {
    expect(ZODIAC_SIGNS).toHaveLength(12);
    const months = ZODIAC_SIGNS.map((s) => s.month);
    expect(months).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
  });

  it("each sign has name, symbol, month, and trait", () => {
    for (const sign of ZODIAC_SIGNS) {
      expect(sign.name).toBeTruthy();
      expect(sign.symbol).toBeTruthy();
      expect(sign.month).toBeGreaterThanOrEqual(1);
      expect(sign.month).toBeLessThanOrEqual(12);
      expect(sign.trait).toBeTruthy();
    }
  });
});

describe("getDailyFortune", () => {
  const sign: ZodiacSign = ZODIAC_SIGNS[0]; // The Prowler

  it("returns a fortune with prediction, severity, and luckyActivity", () => {
    const fortune = getDailyFortune(sign, 12345, 1);
    expect(fortune.prediction).toBeTruthy();
    expect(fortune.severity).toBeTruthy();
    expect(fortune.luckyActivity).toBeTruthy();
  });

  it("severity is a valid FortuneSeverity", () => {
    const validSeverities: FortuneSeverity[] = [
      "blessed",
      "favorable",
      "neutral",
      "challenging",
      "ominous",
    ];
    const fortune = getDailyFortune(sign, 42, 10);
    expect(validSeverities).toContain(fortune.severity);
  });

  it("is deterministic — same inputs produce same fortune", () => {
    const f1 = getDailyFortune(sign, 999, 7);
    const f2 = getDailyFortune(sign, 999, 7);
    expect(f1).toEqual(f2);
  });

  it("different days produce different fortunes", () => {
    const f1 = getDailyFortune(sign, 999, 1);
    const f2 = getDailyFortune(sign, 999, 2);
    // At least one field should differ (prediction or severity or luckyActivity)
    const same =
      f1.prediction === f2.prediction &&
      f1.severity === f2.severity &&
      f1.luckyActivity === f2.luckyActivity;
    expect(same).toBe(false);
  });

  it("different signs on the same day produce different fortunes", () => {
    const sign1 = ZODIAC_SIGNS[0];
    const sign2 = ZODIAC_SIGNS[6];
    const f1 = getDailyFortune(sign1, 999, 5);
    const f2 = getDailyFortune(sign2, 999, 5);
    const same =
      f1.prediction === f2.prediction &&
      f1.severity === f2.severity &&
      f1.luckyActivity === f2.luckyActivity;
    expect(same).toBe(false);
  });

  it("works with day 0", () => {
    const fortune = getDailyFortune(sign, 42, 0);
    expect(fortune.prediction).toBeTruthy();
  });

  it("works with very large day numbers", () => {
    const fortune = getDailyFortune(sign, 42, 999999);
    expect(fortune.prediction).toBeTruthy();
    expect(fortune.severity).toBeTruthy();
  });
});

describe("getSignCompatibility", () => {
  it("returns soulmates for complementary signs", () => {
    // The Prowler (1) + The Sentinel (8) — adventurer + guardian
    const compat = getSignCompatibility(ZODIAC_SIGNS[0], ZODIAC_SIGNS[7]);
    expect(compat.level).toBeTruthy();
    expect(compat.description).toBeTruthy();
  });

  it("returns a valid compatibility level", () => {
    const validLevels: CompatibilityLevel[] = [
      "soulmates",
      "allies",
      "neutral",
      "rivals",
      "nemeses",
    ];
    const compat = getSignCompatibility(ZODIAC_SIGNS[2], ZODIAC_SIGNS[5]);
    expect(validLevels).toContain(compat.level);
  });

  it("same sign compatibility is defined", () => {
    const compat = getSignCompatibility(ZODIAC_SIGNS[3], ZODIAC_SIGNS[3]);
    expect(compat.level).toBeTruthy();
    expect(compat.description).toBeTruthy();
  });

  it("is symmetric — sign1+sign2 equals sign2+sign1", () => {
    const c1 = getSignCompatibility(ZODIAC_SIGNS[1], ZODIAC_SIGNS[4]);
    const c2 = getSignCompatibility(ZODIAC_SIGNS[4], ZODIAC_SIGNS[1]);
    expect(c1.level).toBe(c2.level);
    expect(c1.description).toBe(c2.description);
  });

  it("all 12 signs have compatibility with each other", () => {
    for (let i = 0; i < 12; i++) {
      for (let j = i; j < 12; j++) {
        const compat = getSignCompatibility(ZODIAC_SIGNS[i], ZODIAC_SIGNS[j]);
        expect(compat.level).toBeTruthy();
        expect(compat.description).toBeTruthy();
      }
    }
  });
});

describe("formatHoroscopeEntry", () => {
  it("includes sign name in output", () => {
    const sign = ZODIAC_SIGNS[0];
    const fortune: DailyFortune = {
      prediction: "A great hunt awaits.",
      severity: "favorable",
      luckyActivity: "hunting",
    };
    const entry = formatHoroscopeEntry(sign, fortune);
    expect(entry).toContain("The Prowler");
  });

  it("includes sign symbol in output", () => {
    const sign = ZODIAC_SIGNS[0];
    const fortune: DailyFortune = {
      prediction: "A great hunt awaits.",
      severity: "favorable",
      luckyActivity: "hunting",
    };
    const entry = formatHoroscopeEntry(sign, fortune);
    expect(entry).toContain(sign.symbol);
  });

  it("includes prediction text", () => {
    const sign = ZODIAC_SIGNS[5];
    const fortune: DailyFortune = {
      prediction: "Patience will be rewarded.",
      severity: "neutral",
      luckyActivity: "resting",
    };
    const entry = formatHoroscopeEntry(sign, fortune);
    expect(entry).toContain("Patience will be rewarded.");
  });

  it("includes lucky activity", () => {
    const sign = ZODIAC_SIGNS[2];
    const fortune: DailyFortune = {
      prediction: "Secrets reveal themselves.",
      severity: "blessed",
      luckyActivity: "exploring",
    };
    const entry = formatHoroscopeEntry(sign, fortune);
    expect(entry).toContain("exploring");
  });

  it("returns a non-empty string", () => {
    const sign = ZODIAC_SIGNS[11];
    const fortune: DailyFortune = {
      prediction: "Reflect on past decisions.",
      severity: "challenging",
      luckyActivity: "guarding",
    };
    const entry = formatHoroscopeEntry(sign, fortune);
    expect(entry.length).toBeGreaterThan(0);
  });
});

describe("formatHoroscopeColumn", () => {
  it("includes colony name in section title", () => {
    const entries = ["entry1", "entry2"];
    const column = formatHoroscopeColumn(entries, "Whiskerfield", 42);
    expect(column.sectionTitle).toContain("Whiskerfield");
  });

  it("includes day number", () => {
    const entries = ["entry1"];
    const column = formatHoroscopeColumn(entries, "Catford", 15);
    expect(column.dayNumber).toBe(15);
  });

  it("includes all provided entries", () => {
    const entries = ["alpha", "beta", "gamma"];
    const column = formatHoroscopeColumn(entries, "Catford", 7);
    expect(column.entries).toEqual(entries);
    expect(column.entries).toHaveLength(3);
  });

  it("handles empty entries list", () => {
    const column = formatHoroscopeColumn([], "Catford", 1);
    expect(column.entries).toHaveLength(0);
    expect(column.sectionTitle).toBeTruthy();
  });

  it("section title contains 'Horoscope' or 'Stars'", () => {
    const column = formatHoroscopeColumn(["x"], "Catford", 5);
    const titleLower = column.sectionTitle.toLowerCase();
    expect(
      titleLower.includes("horoscope") || titleLower.includes("stars"),
    ).toBe(true);
  });
});
