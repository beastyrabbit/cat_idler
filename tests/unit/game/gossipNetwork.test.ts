/**
 * Tests for Cat Gossip Network
 *
 * Rumor propagation through social connections with reliability degradation.
 */

import { describe, it, expect } from "vitest";
import {
  generateRumor,
  calculateSpreadChance,
  degradeReliability,
  distortRumor,
  evaluateGossipNetwork,
  generateGossipColumn,
} from "@/lib/game/gossipNetwork";
import type {
  Rumor,
  GossipCat,
  GossipReport,
  RumorType,
} from "@/lib/game/gossipNetwork";

describe("generateRumor", () => {
  it("creates rumor with correct type and 100% reliability", () => {
    const rumor = generateRumor("food_sighting", "Whiskers", 42);
    expect(rumor.type).toBe("food_sighting");
    expect(rumor.reliability).toBe(100);
  });

  it("includes source cat name in message", () => {
    const rumor = generateRumor("romance", "Mittens", 99);
    expect(rumor.message).toContain("Mittens");
    expect(rumor.sourceCatName).toBe("Mittens");
  });

  it("sets retellings to 0", () => {
    const rumor = generateRumor("predator_warning", "Shadow", 7);
    expect(rumor.retellings).toBe(0);
  });

  it("sets originalType to match type", () => {
    const rumor = generateRumor("mystery", "Luna", 123);
    expect(rumor.originalType).toBe("mystery");
    expect(rumor.originalType).toBe(rumor.type);
  });

  it("generates different messages for different rumor types", () => {
    const food = generateRumor("food_sighting", "Cat", 1);
    const predator = generateRumor("predator_warning", "Cat", 1);
    expect(food.message).not.toBe(predator.message);
  });
});

describe("calculateSpreadChance", () => {
  it("returns higher chance for chattier cats", () => {
    const low = calculateSpreadChance(20, 100);
    const high = calculateSpreadChance(80, 100);
    expect(high).toBeGreaterThan(low);
  });

  it("returns lower chance for unreliable rumors", () => {
    const reliable = calculateSpreadChance(50, 100);
    const unreliable = calculateSpreadChance(50, 20);
    expect(reliable).toBeGreaterThan(unreliable);
  });

  it("clamps between 0 and 100", () => {
    const result = calculateSpreadChance(100, 100);
    expect(result).toBeLessThanOrEqual(100);
    expect(result).toBeGreaterThanOrEqual(0);
  });

  it("returns 0 for 0 chattiness", () => {
    const result = calculateSpreadChance(0, 100);
    expect(result).toBe(0);
  });

  it("returns 0 for 0 reliability", () => {
    const result = calculateSpreadChance(50, 0);
    expect(result).toBe(0);
  });
});

describe("degradeReliability", () => {
  it("reduces reliability by 15-25%", () => {
    const result = degradeReliability(100, 42);
    expect(result).toBeGreaterThanOrEqual(75);
    expect(result).toBeLessThanOrEqual(85);
  });

  it("clamps to minimum 0", () => {
    const result = degradeReliability(5, 42);
    expect(result).toBeGreaterThanOrEqual(0);
  });

  it("is deterministic with same seed", () => {
    const a = degradeReliability(100, 42);
    const b = degradeReliability(100, 42);
    expect(a).toBe(b);
  });

  it("produces different results with different seeds", () => {
    const a = degradeReliability(100, 42);
    const b = degradeReliability(100, 999);
    // Different seeds should give different degradation amounts
    // (statistically, not guaranteed, but with these seeds they differ)
    expect(a).not.toBe(b);
  });
});

describe("distortRumor", () => {
  it("keeps original at high reliability (>60)", () => {
    const rumor: Rumor = {
      type: "food_sighting",
      originalType: "food_sighting",
      sourceCatName: "Whiskers",
      reliability: 80,
      retellings: 1,
      message: "Whiskers saw a huge fish!",
    };
    const result = distortRumor(rumor, 80, 42);
    expect(result.type).toBe("food_sighting");
  });

  it("may change type at low reliability (<=60)", () => {
    const rumor: Rumor = {
      type: "food_sighting",
      originalType: "food_sighting",
      sourceCatName: "Whiskers",
      reliability: 20,
      retellings: 3,
      message: "Whiskers saw a huge fish!",
    };
    // Try multiple seeds to find one that causes distortion
    let distorted = false;
    for (let seed = 1; seed <= 100; seed++) {
      const result = distortRumor(rumor, 20, seed);
      if (result.type !== "food_sighting") {
        distorted = true;
        break;
      }
    }
    expect(distorted).toBe(true);
  });

  it("preserves originalType even when type changes", () => {
    const rumor: Rumor = {
      type: "food_sighting",
      originalType: "food_sighting",
      sourceCatName: "Whiskers",
      reliability: 10,
      retellings: 5,
      message: "Whiskers saw a huge fish!",
    };
    // Find a seed that causes distortion
    for (let seed = 1; seed <= 100; seed++) {
      const result = distortRumor(rumor, 10, seed);
      if (result.type !== "food_sighting") {
        expect(result.originalType).toBe("food_sighting");
        return;
      }
    }
    // At reliability 10, at least one seed in 100 should distort
    expect.fail("Expected at least one distortion in 100 seeds");
  });

  it("increments retellings", () => {
    const rumor: Rumor = {
      type: "mystery",
      originalType: "mystery",
      sourceCatName: "Luna",
      reliability: 90,
      retellings: 2,
      message: "Luna heard strange sounds!",
    };
    const result = distortRumor(rumor, 90, 42);
    expect(result.retellings).toBe(3);
  });

  it("does not mutate original rumor", () => {
    const rumor: Rumor = {
      type: "achievement",
      originalType: "achievement",
      sourceCatName: "Tiger",
      reliability: 50,
      retellings: 1,
      message: "Tiger caught three mice!",
    };
    distortRumor(rumor, 50, 42);
    expect(rumor.retellings).toBe(1);
  });
});

describe("evaluateGossipNetwork", () => {
  it("counts total rumors", () => {
    const cats: GossipCat[] = [{ name: "Whiskers", chattiness: 80 }];
    const rumors: Rumor[] = [
      {
        type: "food_sighting",
        originalType: "food_sighting",
        sourceCatName: "Whiskers",
        reliability: 90,
        retellings: 0,
        message: "Fish!",
      },
      {
        type: "mystery",
        originalType: "mystery",
        sourceCatName: "Whiskers",
        reliability: 40,
        retellings: 3,
        message: "Sounds!",
      },
    ];
    const report = evaluateGossipNetwork(cats, rumors);
    expect(report.totalRumors).toBe(2);
  });

  it("identifies reliable vs distorted rumors", () => {
    const cats: GossipCat[] = [{ name: "A", chattiness: 50 }];
    const rumors: Rumor[] = [
      {
        type: "food_sighting",
        originalType: "food_sighting",
        sourceCatName: "A",
        reliability: 80,
        retellings: 1,
        message: "msg",
      },
      {
        type: "mystery",
        originalType: "food_sighting",
        sourceCatName: "A",
        reliability: 30,
        retellings: 4,
        message: "msg",
      },
      {
        type: "romance",
        originalType: "romance",
        sourceCatName: "A",
        reliability: 70,
        retellings: 1,
        message: "msg",
      },
    ];
    const report = evaluateGossipNetwork(cats, rumors);
    expect(report.reliableRumors).toBe(2); // reliability > 60
    expect(report.distortedRumors).toBe(1); // type !== originalType
  });

  it("finds most gossipy cat (highest chattiness)", () => {
    const cats: GossipCat[] = [
      { name: "Quiet", chattiness: 10 },
      { name: "Chatty", chattiness: 95 },
      { name: "Medium", chattiness: 50 },
    ];
    const report = evaluateGossipNetwork(cats, []);
    expect(report.mostGossipyCat).toBe("Chatty");
  });

  it("finds juiciest rumor (lowest reliability with type change)", () => {
    const cats: GossipCat[] = [{ name: "A", chattiness: 50 }];
    const rumors: Rumor[] = [
      {
        type: "food_sighting",
        originalType: "food_sighting",
        sourceCatName: "A",
        reliability: 90,
        retellings: 0,
        message: "msg",
      },
      {
        type: "mystery",
        originalType: "predator_warning",
        sourceCatName: "A",
        reliability: 20,
        retellings: 5,
        message: "msg",
      },
      {
        type: "romance",
        originalType: "food_sighting",
        sourceCatName: "A",
        reliability: 40,
        retellings: 3,
        message: "msg",
      },
    ];
    const report = evaluateGossipNetwork(cats, rumors);
    expect(report.juiciestRumor).not.toBeNull();
    expect(report.juiciestRumor!.reliability).toBe(20);
  });

  it("handles empty inputs", () => {
    const report = evaluateGossipNetwork([], []);
    expect(report.totalRumors).toBe(0);
    expect(report.reliableRumors).toBe(0);
    expect(report.distortedRumors).toBe(0);
    expect(report.mostGossipyCat).toBeNull();
    expect(report.juiciestRumor).toBeNull();
  });
});

describe("generateGossipColumn", () => {
  it("includes colony name", () => {
    const report: GossipReport = {
      totalRumors: 1,
      reliableRumors: 1,
      distortedRumors: 0,
      mostGossipyCat: "Whiskers",
      juiciestRumor: null,
    };
    const column = generateGossipColumn(report, "Catford");
    expect(column).toContain("Catford");
  });

  it("handles 0 rumors (quiet day)", () => {
    const report: GossipReport = {
      totalRumors: 0,
      reliableRumors: 0,
      distortedRumors: 0,
      mostGossipyCat: null,
      juiciestRumor: null,
    };
    const column = generateGossipColumn(report, "Catford");
    expect(column.toLowerCase()).toContain("quiet");
  });

  it("handles multiple rumors with summary", () => {
    const report: GossipReport = {
      totalRumors: 5,
      reliableRumors: 3,
      distortedRumors: 2,
      mostGossipyCat: "Chatty",
      juiciestRumor: {
        type: "mystery",
        originalType: "food_sighting",
        sourceCatName: "Luna",
        reliability: 15,
        retellings: 6,
        message: "Strange noises from the den!",
      },
    };
    const column = generateGossipColumn(report, "Catford");
    expect(column).toContain("5");
    expect(column).toContain("Chatty");
  });

  it('highlights distorted rumors as "unconfirmed"', () => {
    const report: GossipReport = {
      totalRumors: 3,
      reliableRumors: 1,
      distortedRumors: 2,
      mostGossipyCat: "Gabby",
      juiciestRumor: {
        type: "romance",
        originalType: "food_sighting",
        sourceCatName: "Whiskers",
        reliability: 10,
        retellings: 7,
        message: "Whiskers found love!",
      },
    };
    const column = generateGossipColumn(report, "Catford");
    expect(column.toLowerCase()).toContain("unconfirmed");
  });
});
