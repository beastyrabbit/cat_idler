import { describe, expect, it } from "vitest";

import {
  canMentor,
  calculateTrainingEffectiveness,
  calculateMentorXPBonus,
  findBestMentor,
  generateAcademyReport,
  type CatForMentorship,
  type SkillDomain,
  type MentorMatch,
} from "@/lib/game/mentorship";

// ---------- canMentor ----------

describe("canMentor", () => {
  const makeCat = (
    name: string,
    lifeStage: CatForMentorship["lifeStage"],
    skills: Partial<Record<SkillDomain, number>> = {},
  ): CatForMentorship => ({
    name,
    lifeStage,
    skills: {
      hunting: 0,
      combat: 0,
      gathering: 0,
      building: 0,
      healing: 0,
      ...skills,
    },
  });

  it("returns true for eligible mentor-apprentice pair", () => {
    const mentor = makeCat("Elder Whiskers", "elder", { hunting: 80 });
    const apprentice = makeCat("Young Paws", "young", { hunting: 20 });
    expect(canMentor(mentor, apprentice, "hunting")).toBe(true);
  });

  it("returns false if mentor is kitten", () => {
    const mentor = makeCat("Baby", "kitten", { hunting: 80 });
    const apprentice = makeCat("Young Paws", "young", { hunting: 20 });
    expect(canMentor(mentor, apprentice, "hunting")).toBe(false);
  });

  it("returns false if mentor is young", () => {
    const mentor = makeCat("Youngster", "young", { hunting: 80 });
    const apprentice = makeCat("Another Young", "young", { hunting: 20 });
    expect(canMentor(mentor, apprentice, "hunting")).toBe(false);
  });

  it("returns false if mentor skill < 50", () => {
    const mentor = makeCat("Elder Low", "elder", { hunting: 40 });
    const apprentice = makeCat("Young Paws", "young", { hunting: 10 });
    expect(canMentor(mentor, apprentice, "hunting")).toBe(false);
  });

  it("returns false if apprentice skill >= mentor skill - 10", () => {
    const mentor = makeCat("Elder", "elder", { hunting: 60 });
    const apprentice = makeCat("Almost There", "young", { hunting: 55 });
    expect(canMentor(mentor, apprentice, "hunting")).toBe(false);
  });

  it("returns false if apprentice is elder", () => {
    const mentor = makeCat("Elder A", "elder", { hunting: 80 });
    const apprentice = makeCat("Elder B", "elder", { hunting: 20 });
    expect(canMentor(mentor, apprentice, "hunting")).toBe(false);
  });

  it("returns true for adult mentor with skill >= 50", () => {
    const mentor = makeCat("Adult Pro", "adult", { combat: 75 });
    const apprentice = makeCat("Young Learner", "young", { combat: 10 });
    expect(canMentor(mentor, apprentice, "combat")).toBe(true);
  });

  it("returns true for adult apprentice with low enough skill", () => {
    const mentor = makeCat("Elder Master", "elder", { gathering: 90 });
    const apprentice = makeCat("Adult Novice", "adult", { gathering: 30 });
    expect(canMentor(mentor, apprentice, "gathering")).toBe(true);
  });
});

// ---------- calculateTrainingEffectiveness ----------

describe("calculateTrainingEffectiveness", () => {
  it("returns higher values for skilled mentors", () => {
    const lowSkill = calculateTrainingEffectiveness(50, 10, "adult", 1);
    const highSkill = calculateTrainingEffectiveness(90, 10, "adult", 1);
    expect(highSkill).toBeGreaterThan(lowSkill);
  });

  it("gives elder mentors +20% bonus", () => {
    const adultEff = calculateTrainingEffectiveness(80, 20, "adult", 1);
    const elderEff = calculateTrainingEffectiveness(80, 20, "elder", 1);
    expect(elderEff).toBeCloseTo(adultEff * 1.2, 2);
  });

  it("penalizes overloaded mentors (-15% per extra apprentice)", () => {
    const oneStudent = calculateTrainingEffectiveness(80, 20, "adult", 1);
    const twoStudents = calculateTrainingEffectiveness(80, 20, "adult", 2);
    const threeStudents = calculateTrainingEffectiveness(80, 20, "adult", 3);
    expect(twoStudents).toBeCloseTo(oneStudent * 0.85, 2);
    expect(threeStudents).toBeCloseTo(oneStudent * 0.7, 2);
  });

  it("caps at 2.0", () => {
    // Elder with max skill, huge gap, 1 apprentice
    const eff = calculateTrainingEffectiveness(100, 0, "elder", 1);
    expect(eff).toBeLessThanOrEqual(2.0);
  });

  it("skill gap bonus increases with larger gaps", () => {
    const smallGap = calculateTrainingEffectiveness(80, 60, "adult", 1);
    const largeGap = calculateTrainingEffectiveness(80, 10, "adult", 1);
    expect(largeGap).toBeGreaterThan(smallGap);
  });

  it("returns positive value for minimum valid inputs", () => {
    const eff = calculateTrainingEffectiveness(50, 0, "adult", 1);
    expect(eff).toBeGreaterThan(0);
  });
});

// ---------- calculateMentorXPBonus ----------

describe("calculateMentorXPBonus", () => {
  it("multiplies base XP by effectiveness", () => {
    const bonus = calculateMentorXPBonus(1.5, 100);
    expect(bonus).toBe(150);
  });

  it("returns 0 for 0 effectiveness", () => {
    expect(calculateMentorXPBonus(0, 100)).toBe(0);
  });

  it("returns 0 for 0 base XP", () => {
    expect(calculateMentorXPBonus(1.5, 0)).toBe(0);
  });

  it("rounds to nearest integer", () => {
    const bonus = calculateMentorXPBonus(1.3, 10);
    expect(bonus).toBe(13);
  });
});

// ---------- findBestMentor ----------

describe("findBestMentor", () => {
  const makeCat = (
    name: string,
    lifeStage: CatForMentorship["lifeStage"],
    skills: Partial<Record<SkillDomain, number>> = {},
  ): CatForMentorship => ({
    name,
    lifeStage,
    skills: {
      hunting: 0,
      combat: 0,
      gathering: 0,
      building: 0,
      healing: 0,
      ...skills,
    },
  });

  it("returns null when no eligible mentors", () => {
    const cats = [
      makeCat("Kitten", "kitten", { hunting: 90 }),
      makeCat("Low Skill", "adult", { hunting: 30 }),
    ];
    const apprentice = makeCat("Learner", "young", { hunting: 10 });
    expect(findBestMentor(cats, apprentice, "hunting")).toBeNull();
  });

  it("picks highest effectiveness mentor", () => {
    const cats = [
      makeCat("Good Elder", "elder", { hunting: 90 }),
      makeCat("Ok Adult", "adult", { hunting: 60 }),
    ];
    const apprentice = makeCat("Learner", "young", { hunting: 10 });
    const result = findBestMentor(cats, apprentice, "hunting");
    expect(result).not.toBeNull();
    expect(result!.mentorName).toBe("Good Elder");
  });

  it("skips ineligible cats", () => {
    const cats = [
      makeCat("Kitten", "kitten", { building: 80 }),
      makeCat("Young", "young", { building: 80 }),
      makeCat("Low Skill Adult", "adult", { building: 30 }),
      makeCat("Valid Elder", "elder", { building: 70 }),
    ];
    const apprentice = makeCat("Learner", "young", { building: 5 });
    const result = findBestMentor(cats, apprentice, "building");
    expect(result).not.toBeNull();
    expect(result!.mentorName).toBe("Valid Elder");
  });

  it("includes correct fields in MentorMatch", () => {
    const cats = [makeCat("Master", "elder", { healing: 85 })];
    const apprentice = makeCat("Student", "young", { healing: 15 });
    const result = findBestMentor(cats, apprentice, "healing");
    expect(result).not.toBeNull();
    expect(result!.mentorName).toBe("Master");
    expect(result!.apprenticeName).toBe("Student");
    expect(result!.skillDomain).toBe("healing");
    expect(result!.effectiveness).toBeGreaterThan(0);
    expect(result!.xpBonus).toBeGreaterThan(0);
  });
});

// ---------- generateAcademyReport ----------

describe("generateAcademyReport", () => {
  it("includes colony name", () => {
    const pairings: MentorMatch[] = [
      {
        mentorName: "Elder Sage",
        apprenticeName: "Young Scout",
        skillDomain: "hunting",
        effectiveness: 1.2,
        xpBonus: 24,
      },
    ];
    const report = generateAcademyReport(pairings, "Whiskerfield");
    expect(report).toContain("Whiskerfield");
  });

  it("handles 0 pairings", () => {
    const report = generateAcademyReport([], "Catford");
    expect(report).toContain("Catford");
    expect(report.toLowerCase()).toMatch(
      /no\s+(classes|sessions|pairings|mentoring)/,
    );
  });

  it("handles 1 pairing (singular)", () => {
    const pairings: MentorMatch[] = [
      {
        mentorName: "Sage",
        apprenticeName: "Paws",
        skillDomain: "combat",
        effectiveness: 1.0,
        xpBonus: 20,
      },
    ];
    const report = generateAcademyReport(pairings, "Catford");
    expect(report).toContain("Sage");
    expect(report).toContain("Paws");
  });

  it("handles multiple pairings", () => {
    const pairings: MentorMatch[] = [
      {
        mentorName: "Sage",
        apprenticeName: "Paws",
        skillDomain: "combat",
        effectiveness: 1.0,
        xpBonus: 20,
      },
      {
        mentorName: "Whiskers",
        apprenticeName: "Boots",
        skillDomain: "gathering",
        effectiveness: 1.5,
        xpBonus: 30,
      },
    ];
    const report = generateAcademyReport(pairings, "Catford");
    expect(report).toContain("Sage");
    expect(report).toContain("Whiskers");
    expect(report).toContain("Paws");
    expect(report).toContain("Boots");
  });
});
