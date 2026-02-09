import { describe, it, expect } from "vitest";
import {
  getRelationshipTier,
  getProductivityBonus,
  updateAffinity,
  decayAffinity,
  describeRelationship,
} from "@/lib/game/relationships";

describe("getRelationshipTier", () => {
  it("returns stranger for affinity 0", () => {
    expect(getRelationshipTier(0)).toBe("stranger");
  });

  it("returns stranger for affinity 19", () => {
    expect(getRelationshipTier(19)).toBe("stranger");
  });

  it("returns acquaintance for affinity 20", () => {
    expect(getRelationshipTier(20)).toBe("acquaintance");
  });

  it("returns acquaintance for affinity 49", () => {
    expect(getRelationshipTier(49)).toBe("acquaintance");
  });

  it("returns friend for affinity 50", () => {
    expect(getRelationshipTier(50)).toBe("friend");
  });

  it("returns friend for affinity 79", () => {
    expect(getRelationshipTier(79)).toBe("friend");
  });

  it("returns bonded for affinity 80", () => {
    expect(getRelationshipTier(80)).toBe("bonded");
  });

  it("returns bonded for affinity 100", () => {
    expect(getRelationshipTier(100)).toBe("bonded");
  });

  it("clamps negative affinity to stranger", () => {
    expect(getRelationshipTier(-10)).toBe("stranger");
  });

  it("clamps affinity above 100 to bonded", () => {
    expect(getRelationshipTier(150)).toBe("bonded");
  });
});

describe("getProductivityBonus", () => {
  it("returns 1.0 for stranger", () => {
    expect(getProductivityBonus("stranger")).toBe(1.0);
  });

  it("returns 1.05 for acquaintance", () => {
    expect(getProductivityBonus("acquaintance")).toBe(1.05);
  });

  it("returns 1.15 for friend", () => {
    expect(getProductivityBonus("friend")).toBe(1.15);
  });

  it("returns 1.25 for bonded", () => {
    expect(getProductivityBonus("bonded")).toBe(1.25);
  });
});

describe("updateAffinity", () => {
  it("adds +5 for shared_task", () => {
    expect(updateAffinity(10, "shared_task")).toBe(15);
  });

  it("adds +10 for shared_encounter", () => {
    expect(updateAffinity(10, "shared_encounter")).toBe(20);
  });

  it("adds +3 for shared_meal", () => {
    expect(updateAffinity(10, "shared_meal")).toBe(13);
  });

  it("adds +1 for proximity", () => {
    expect(updateAffinity(10, "proximity")).toBe(11);
  });

  it("clamps result at 100", () => {
    expect(updateAffinity(98, "shared_encounter")).toBe(100);
  });

  it("clamps result at 0 for negative current", () => {
    expect(updateAffinity(-5, "proximity")).toBe(0);
  });

  it("works from 0 affinity", () => {
    expect(updateAffinity(0, "shared_task")).toBe(5);
  });

  it("does not exceed 100 even with large current", () => {
    expect(updateAffinity(100, "shared_encounter")).toBe(100);
  });
});

describe("decayAffinity", () => {
  it("decays 1 point per 6 hours", () => {
    expect(decayAffinity(30, 6)).toBe(29);
  });

  it("decays 2 points for 12 hours", () => {
    expect(decayAffinity(30, 12)).toBe(28);
  });

  it("does not decay for less than 6 hours", () => {
    expect(decayAffinity(30, 5)).toBe(30);
  });

  it("does not go below 0", () => {
    expect(decayAffinity(2, 600)).toBe(0);
  });

  it("handles 0 hours (no decay)", () => {
    expect(decayAffinity(50, 0)).toBe(50);
  });

  it("treats negative hours as 0", () => {
    expect(decayAffinity(50, -10)).toBe(50);
  });

  it("handles 0 affinity (stays 0)", () => {
    expect(decayAffinity(0, 100)).toBe(0);
  });

  it("handles very large hours", () => {
    expect(decayAffinity(100, 60000)).toBe(0);
  });
});

describe("describeRelationship", () => {
  it("describes strangers", () => {
    const desc = describeRelationship("Thornpaw", "Emberfur", "stranger");
    expect(desc).toContain("Thornpaw");
    expect(desc).toContain("Emberfur");
    expect(desc.toLowerCase()).toContain("stranger");
  });

  it("describes acquaintances", () => {
    const desc = describeRelationship("Thornpaw", "Emberfur", "acquaintance");
    expect(desc).toContain("Thornpaw");
    expect(desc).toContain("Emberfur");
  });

  it("describes friends", () => {
    const desc = describeRelationship("Thornpaw", "Emberfur", "friend");
    expect(desc).toContain("Thornpaw");
    expect(desc).toContain("Emberfur");
  });

  it("describes bonded pairs", () => {
    const desc = describeRelationship("Thornpaw", "Emberfur", "bonded");
    expect(desc).toContain("Thornpaw");
    expect(desc).toContain("Emberfur");
    expect(desc.toLowerCase()).toMatch(/bonded|inseparable/);
  });

  it("returns different descriptions for different tiers", () => {
    const s = describeRelationship("A", "B", "stranger");
    const a = describeRelationship("A", "B", "acquaintance");
    const f = describeRelationship("A", "B", "friend");
    const b = describeRelationship("A", "B", "bonded");
    const descriptions = new Set([s, a, f, b]);
    expect(descriptions.size).toBe(4);
  });
});
