import { describe, it, expect } from "vitest";
import {
  detectLifeStageTransition,
  generateMilestoneAnnouncement,
  getMilestoneTitle,
  getMilestoneDescription,
} from "@/lib/game/lifeMilestones";
import type { LifeStageTransition } from "@/lib/game/lifeMilestones";
import type { CatStats } from "@/types/game";

// =============================================================================
// detectLifeStageTransition
// =============================================================================

describe("detectLifeStageTransition", () => {
  it("returns null when no boundary crossed (both kitten)", () => {
    expect(detectLifeStageTransition(2, 5)).toBeNull();
  });

  it("returns null when no boundary crossed (both adult)", () => {
    expect(detectLifeStageTransition(30, 40)).toBeNull();
  });

  it("returns null when same age", () => {
    expect(detectLifeStageTransition(10, 10)).toBeNull();
  });

  it("detects kitten → young at 6h boundary", () => {
    const result = detectLifeStageTransition(5, 7);
    expect(result).not.toBeNull();
    expect(result!.from).toBe("kitten");
    expect(result!.to).toBe("young");
    expect(result!.label).toBe("First Steps");
    expect(result!.ageThresholdHours).toBe(6);
  });

  it("detects young → adult at 24h boundary", () => {
    const result = detectLifeStageTransition(20, 25);
    expect(result).not.toBeNull();
    expect(result!.from).toBe("young");
    expect(result!.to).toBe("adult");
    expect(result!.label).toBe("Coming of Age");
    expect(result!.ageThresholdHours).toBe(24);
  });

  it("detects adult → elder at 48h boundary", () => {
    const result = detectLifeStageTransition(45, 50);
    expect(result).not.toBeNull();
    expect(result!.from).toBe("adult");
    expect(result!.to).toBe("elder");
    expect(result!.label).toBe("Wisdom Years");
    expect(result!.ageThresholdHours).toBe(48);
  });

  it("detects exact boundary crossing (previous = 5.99, current = 6)", () => {
    const result = detectLifeStageTransition(5.99, 6);
    expect(result).not.toBeNull();
    expect(result!.from).toBe("kitten");
    expect(result!.to).toBe("young");
  });

  it("returns null for negative ages", () => {
    expect(detectLifeStageTransition(-5, -1)).toBeNull();
  });

  it("handles crossing multiple boundaries (returns earliest)", () => {
    // previousAge in kitten range, currentAge in adult range
    const result = detectLifeStageTransition(3, 30);
    expect(result).not.toBeNull();
    expect(result!.from).toBe("kitten");
    expect(result!.to).toBe("young");
    expect(result!.ageThresholdHours).toBe(6);
  });

  it("returns null when previousAge equals boundary exactly (already transitioned)", () => {
    // If previous was exactly 6, the cat was already young
    expect(detectLifeStageTransition(6, 10)).toBeNull();
  });
});

// =============================================================================
// getMilestoneTitle
// =============================================================================

describe("getMilestoneTitle", () => {
  it('returns "First Steps" for kitten → young', () => {
    const t: LifeStageTransition = {
      from: "kitten",
      to: "young",
      label: "First Steps",
      ageThresholdHours: 6,
    };
    expect(getMilestoneTitle(t)).toBe("First Steps");
  });

  it('returns "Coming of Age" for young → adult', () => {
    const t: LifeStageTransition = {
      from: "young",
      to: "adult",
      label: "Coming of Age",
      ageThresholdHours: 24,
    };
    expect(getMilestoneTitle(t)).toBe("Coming of Age");
  });

  it('returns "Wisdom Years" for adult → elder', () => {
    const t: LifeStageTransition = {
      from: "adult",
      to: "elder",
      label: "Wisdom Years",
      ageThresholdHours: 48,
    };
    expect(getMilestoneTitle(t)).toBe("Wisdom Years");
  });
});

// =============================================================================
// getMilestoneDescription
// =============================================================================

describe("getMilestoneDescription", () => {
  it("includes cat name for kitten → young", () => {
    const t: LifeStageTransition = {
      from: "kitten",
      to: "young",
      label: "First Steps",
      ageThresholdHours: 6,
    };
    const desc = getMilestoneDescription(t, "Whiskers");
    expect(desc).toContain("Whiskers");
    expect(desc.length).toBeGreaterThan(10);
  });

  it("includes cat name for young → adult", () => {
    const t: LifeStageTransition = {
      from: "young",
      to: "adult",
      label: "Coming of Age",
      ageThresholdHours: 24,
    };
    const desc = getMilestoneDescription(t, "Shadow");
    expect(desc).toContain("Shadow");
  });

  it("includes cat name for adult → elder", () => {
    const t: LifeStageTransition = {
      from: "adult",
      to: "elder",
      label: "Wisdom Years",
      ageThresholdHours: 48,
    };
    const desc = getMilestoneDescription(t, "Mittens");
    expect(desc).toContain("Mittens");
  });
});

// =============================================================================
// generateMilestoneAnnouncement
// =============================================================================

describe("generateMilestoneAnnouncement", () => {
  const baseStats: CatStats = {
    attack: 5,
    defense: 3,
    hunting: 12,
    medicine: 2,
    cleaning: 1,
    building: 4,
    leadership: 6,
    vision: 8,
  };

  const kittenToYoung: LifeStageTransition = {
    from: "kitten",
    to: "young",
    label: "First Steps",
    ageThresholdHours: 6,
  };

  it("returns headline, body, catName, transition, and statHighlight", () => {
    const result = generateMilestoneAnnouncement(
      kittenToYoung,
      "Whiskers",
      baseStats,
    );
    expect(result).toHaveProperty("headline");
    expect(result).toHaveProperty("body");
    expect(result).toHaveProperty("catName");
    expect(result).toHaveProperty("transition");
    expect(result).toHaveProperty("statHighlight");
  });

  it("includes cat name in headline", () => {
    const result = generateMilestoneAnnouncement(
      kittenToYoung,
      "Whiskers",
      baseStats,
    );
    expect(result.headline).toContain("Whiskers");
  });

  it("includes transition label in headline", () => {
    const result = generateMilestoneAnnouncement(
      kittenToYoung,
      "Whiskers",
      baseStats,
    );
    expect(result.headline).toContain("First Steps");
  });

  it("preserves catName field", () => {
    const result = generateMilestoneAnnouncement(
      kittenToYoung,
      "Shadow",
      baseStats,
    );
    expect(result.catName).toBe("Shadow");
  });

  it("preserves transition field", () => {
    const result = generateMilestoneAnnouncement(
      kittenToYoung,
      "Shadow",
      baseStats,
    );
    expect(result.transition).toBe(kittenToYoung);
  });

  it("highlights the highest stat", () => {
    // hunting=12 is the highest stat
    const result = generateMilestoneAnnouncement(
      kittenToYoung,
      "Whiskers",
      baseStats,
    );
    expect(result.statHighlight).toContain("hunting");
  });

  it("handles tied stats (picks first alphabetically)", () => {
    const tiedStats: CatStats = {
      attack: 10,
      defense: 10,
      hunting: 5,
      medicine: 5,
      cleaning: 5,
      building: 5,
      leadership: 5,
      vision: 5,
    };
    const result = generateMilestoneAnnouncement(
      kittenToYoung,
      "Patches",
      tiedStats,
    );
    expect(result.statHighlight).toContain("attack");
  });

  it("generates body text including cat name", () => {
    const result = generateMilestoneAnnouncement(
      kittenToYoung,
      "Luna",
      baseStats,
    );
    expect(result.body).toContain("Luna");
    expect(result.body.length).toBeGreaterThan(20);
  });

  it("generates different body text per transition type", () => {
    const youngToAdult: LifeStageTransition = {
      from: "young",
      to: "adult",
      label: "Coming of Age",
      ageThresholdHours: 24,
    };
    const r1 = generateMilestoneAnnouncement(kittenToYoung, "Luna", baseStats);
    const r2 = generateMilestoneAnnouncement(youngToAdult, "Luna", baseStats);
    expect(r1.body).not.toBe(r2.body);
  });
});
