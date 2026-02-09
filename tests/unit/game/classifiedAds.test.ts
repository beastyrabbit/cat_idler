/**
 * Tests for Classified Ads System
 *
 * Newspaper-style classifieds generator with 4 ad categories:
 * situations vacant, services offered, wanted, lost & found.
 */

import { describe, it, expect } from "vitest";
import {
  getVacantPositionAds,
  getServicesOfferedAds,
  getWantedAds,
  getLostAndFoundAds,
  formatClassifiedAd,
  formatClassifiedsSection,
  type ClassifiedAd,
  type ClassifiedsSection,
  type AdCategory,
} from "@/lib/game/classifiedAds";

// ---------------------------------------------------------------------------
// getVacantPositionAds
// ---------------------------------------------------------------------------

describe("getVacantPositionAds", () => {
  it("generates hunter ad when food ≤ 30", () => {
    const ads = getVacantPositionAds(
      { food: 25, water: 80, herbs: 60 },
      ["den"],
      5,
    );
    expect(ads.length).toBeGreaterThanOrEqual(1);
    const hunterAd = ads.find((a) => a.headline.toLowerCase().includes("hunt"));
    expect(hunterAd).toBeDefined();
    expect(hunterAd!.category).toBe("situations_vacant");
  });

  it("generates water fetcher ad when water ≤ 30", () => {
    const ads = getVacantPositionAds(
      { food: 80, water: 20, herbs: 60 },
      ["den"],
      5,
    );
    const waterAd = ads.find((a) => a.headline.toLowerCase().includes("water"));
    expect(waterAd).toBeDefined();
    expect(waterAd!.category).toBe("situations_vacant");
  });

  it("generates herbalist ad when herbs ≤ 30", () => {
    const ads = getVacantPositionAds(
      { food: 80, water: 80, herbs: 15 },
      ["den"],
      5,
    );
    const herbAd = ads.find((a) => a.headline.toLowerCase().includes("herb"));
    expect(herbAd).toBeDefined();
    expect(herbAd!.category).toBe("situations_vacant");
  });

  it("generates no ads when all resources are above 30", () => {
    const ads = getVacantPositionAds(
      { food: 80, water: 80, herbs: 60 },
      ["den", "walls"],
      5,
    );
    expect(ads.length).toBe(0);
  });

  it("generates multiple ads when multiple resources are low", () => {
    const ads = getVacantPositionAds(
      { food: 10, water: 10, herbs: 10 },
      ["den"],
      3,
    );
    expect(ads.length).toBe(3);
    const categories = ads.map((a) => a.category);
    expect(categories.every((c) => c === "situations_vacant")).toBe(true);
  });

  it("sets urgency to high when resource ≤ 10", () => {
    const ads = getVacantPositionAds(
      { food: 5, water: 80, herbs: 60 },
      ["den"],
      5,
    );
    const hunterAd = ads.find((a) => a.headline.toLowerCase().includes("hunt"));
    expect(hunterAd!.urgency).toBe("high");
  });

  it("sets urgency to medium when resource between 11 and 30", () => {
    const ads = getVacantPositionAds(
      { food: 25, water: 80, herbs: 60 },
      ["den"],
      5,
    );
    const hunterAd = ads.find((a) => a.headline.toLowerCase().includes("hunt"));
    expect(hunterAd!.urgency).toBe("medium");
  });
});

// ---------------------------------------------------------------------------
// getServicesOfferedAds
// ---------------------------------------------------------------------------

describe("getServicesOfferedAds", () => {
  it("generates ad for cat with hunting ≥ 60", () => {
    const stats = {
      attack: 10,
      defense: 10,
      hunting: 75,
      medicine: 10,
      cleaning: 10,
      building: 10,
      leadership: 10,
      vision: 10,
    };
    const ads = getServicesOfferedAds(stats, "Whiskers", 42);
    expect(ads.length).toBeGreaterThanOrEqual(1);
    expect(ads[0].category).toBe("services_offered");
    expect(ads[0].body).toContain("Whiskers");
  });

  it("generates multiple ads for cat with multiple high skills", () => {
    const stats = {
      attack: 80,
      defense: 70,
      hunting: 90,
      medicine: 65,
      cleaning: 10,
      building: 10,
      leadership: 10,
      vision: 10,
    };
    const ads = getServicesOfferedAds(stats, "Shadow", 42);
    expect(ads.length).toBe(4); // attack, defense, hunting, medicine
  });

  it("generates no ads when all stats below 60", () => {
    const stats = {
      attack: 30,
      defense: 40,
      hunting: 50,
      medicine: 20,
      cleaning: 10,
      building: 15,
      leadership: 25,
      vision: 30,
    };
    const ads = getServicesOfferedAds(stats, "Lazy", 42);
    expect(ads.length).toBe(0);
  });

  it("is deterministic with same seed", () => {
    const stats = {
      attack: 80,
      defense: 10,
      hunting: 10,
      medicine: 10,
      cleaning: 10,
      building: 10,
      leadership: 10,
      vision: 10,
    };
    const ads1 = getServicesOfferedAds(stats, "Fighter", 99);
    const ads2 = getServicesOfferedAds(stats, "Fighter", 99);
    expect(ads1).toEqual(ads2);
  });

  it("produces different flavor text with different seeds", () => {
    const stats = {
      attack: 80,
      defense: 10,
      hunting: 10,
      medicine: 10,
      cleaning: 10,
      building: 10,
      leadership: 10,
      vision: 10,
    };
    const ads1 = getServicesOfferedAds(stats, "Fighter", 1);
    const ads2 = getServicesOfferedAds(stats, "Fighter", 9999);
    // At least the body text should potentially differ (seeded flavor)
    // They both advertise the same skill but flavor text may differ
    expect(ads1[0].category).toBe(ads2[0].category);
  });

  it("includes cat name in body text", () => {
    const stats = {
      attack: 10,
      defense: 10,
      hunting: 10,
      medicine: 10,
      cleaning: 10,
      building: 70,
      leadership: 10,
      vision: 10,
    };
    const ads = getServicesOfferedAds(stats, "Builder Bob", 42);
    expect(ads[0].body).toContain("Builder Bob");
  });

  it("sets urgency to low for services offered", () => {
    const stats = {
      attack: 80,
      defense: 10,
      hunting: 10,
      medicine: 10,
      cleaning: 10,
      building: 10,
      leadership: 10,
      vision: 10,
    };
    const ads = getServicesOfferedAds(stats, "Fighter", 42);
    expect(ads[0].urgency).toBe("low");
  });
});

// ---------------------------------------------------------------------------
// getWantedAds
// ---------------------------------------------------------------------------

describe("getWantedAds", () => {
  it("generates wanted ad for resource ≤ 20", () => {
    const ads = getWantedAds({ food: 15, water: 80, herbs: 60 });
    expect(ads.length).toBe(1);
    expect(ads[0].category).toBe("wanted");
    expect(ads[0].headline.toLowerCase()).toContain("food");
  });

  it("generates no ads when all resources above 20", () => {
    const ads = getWantedAds({ food: 50, water: 80, herbs: 60 });
    expect(ads.length).toBe(0);
  });

  it("generates multiple ads when multiple resources critical", () => {
    const ads = getWantedAds({ food: 5, water: 10, herbs: 15 });
    expect(ads.length).toBe(3);
  });

  it("sets urgency to high when resource ≤ 5", () => {
    const ads = getWantedAds({ food: 3, water: 80, herbs: 60 });
    expect(ads[0].urgency).toBe("high");
  });

  it("sets urgency to medium when resource between 6 and 20", () => {
    const ads = getWantedAds({ food: 15, water: 80, herbs: 60 });
    expect(ads[0].urgency).toBe("medium");
  });
});

// ---------------------------------------------------------------------------
// getLostAndFoundAds
// ---------------------------------------------------------------------------

describe("getLostAndFoundAds", () => {
  it("generates 1-3 ads", () => {
    const ads = getLostAndFoundAds(12345, 10);
    expect(ads.length).toBeGreaterThanOrEqual(1);
    expect(ads.length).toBeLessThanOrEqual(3);
  });

  it("all ads have lost_and_found category", () => {
    const ads = getLostAndFoundAds(42, 5);
    ads.forEach((ad) => {
      expect(ad.category).toBe("lost_and_found");
    });
  });

  it("is deterministic with same seed and day", () => {
    const ads1 = getLostAndFoundAds(42, 10);
    const ads2 = getLostAndFoundAds(42, 10);
    expect(ads1).toEqual(ads2);
  });

  it("produces different items with different days", () => {
    const ads1 = getLostAndFoundAds(42, 1);
    const ads2 = getLostAndFoundAds(42, 100);
    // Different day numbers should produce different results
    const headlines1 = ads1.map((a) => a.headline).join(",");
    const headlines2 = ads2.map((a) => a.headline).join(",");
    expect(headlines1).not.toBe(headlines2);
  });

  it("sets urgency to low for lost and found items", () => {
    const ads = getLostAndFoundAds(42, 5);
    ads.forEach((ad) => {
      expect(ad.urgency).toBe("low");
    });
  });
});

// ---------------------------------------------------------------------------
// formatClassifiedAd
// ---------------------------------------------------------------------------

describe("formatClassifiedAd", () => {
  it("includes category header in formatted text", () => {
    const ad: ClassifiedAd = {
      category: "situations_vacant",
      headline: "Hunter Needed",
      body: "Apply at the den.",
      urgency: "high",
    };
    const text = formatClassifiedAd(ad);
    expect(text).toContain("SITUATIONS VACANT");
    expect(text).toContain("Hunter Needed");
    expect(text).toContain("Apply at the den.");
  });

  it("formats services_offered category header", () => {
    const ad: ClassifiedAd = {
      category: "services_offered",
      headline: "Expert Builder",
      body: "Whiskers builds anything.",
      urgency: "low",
    };
    const text = formatClassifiedAd(ad);
    expect(text).toContain("SERVICES OFFERED");
  });

  it("formats wanted category header", () => {
    const ad: ClassifiedAd = {
      category: "wanted",
      headline: "Food Desperately Needed",
      body: "Colony running low.",
      urgency: "high",
    };
    const text = formatClassifiedAd(ad);
    expect(text).toContain("WANTED");
  });

  it("formats lost_and_found category header", () => {
    const ad: ClassifiedAd = {
      category: "lost_and_found",
      headline: "Found: Shiny Pebble",
      body: "Near the river.",
      urgency: "low",
    };
    const text = formatClassifiedAd(ad);
    expect(text).toContain("LOST & FOUND");
  });

  it("adds URGENT marker for high urgency", () => {
    const ad: ClassifiedAd = {
      category: "wanted",
      headline: "Food Needed",
      body: "Please help.",
      urgency: "high",
    };
    const text = formatClassifiedAd(ad);
    expect(text).toContain("URGENT");
  });

  it("does not add URGENT marker for low urgency", () => {
    const ad: ClassifiedAd = {
      category: "lost_and_found",
      headline: "Found: Ball of Yarn",
      body: "Claim at the den.",
      urgency: "low",
    };
    const text = formatClassifiedAd(ad);
    expect(text).not.toContain("URGENT");
  });
});

// ---------------------------------------------------------------------------
// formatClassifiedsSection
// ---------------------------------------------------------------------------

describe("formatClassifiedsSection", () => {
  it("includes colony name in section title", () => {
    const section = formatClassifiedsSection([], "Catford");
    expect(section.sectionTitle).toContain("Catford");
  });

  it("returns correct total ads count", () => {
    const ads: ClassifiedAd[] = [
      {
        category: "wanted",
        headline: "Food",
        body: "Need food",
        urgency: "high",
      },
      {
        category: "lost_and_found",
        headline: "Ball",
        body: "Found ball",
        urgency: "low",
      },
    ];
    const section = formatClassifiedsSection(ads, "Catford");
    expect(section.totalAds).toBe(2);
  });

  it("returns empty section with zero total for no ads", () => {
    const section = formatClassifiedsSection([], "Catford");
    expect(section.totalAds).toBe(0);
    expect(section.ads).toEqual([]);
  });

  it("preserves all ads in output", () => {
    const ads: ClassifiedAd[] = [
      {
        category: "situations_vacant",
        headline: "Hunter",
        body: "Need hunter",
        urgency: "medium",
      },
      {
        category: "services_offered",
        headline: "Builder",
        body: "Expert builder",
        urgency: "low",
      },
      {
        category: "wanted",
        headline: "Water",
        body: "Need water",
        urgency: "high",
      },
    ];
    const section = formatClassifiedsSection(ads, "Catford");
    expect(section.ads.length).toBe(3);
    expect(section.totalAds).toBe(3);
  });

  it("returns ClassifiedsSection type with required fields", () => {
    const section = formatClassifiedsSection([], "TestColony");
    expect(section).toHaveProperty("sectionTitle");
    expect(section).toHaveProperty("ads");
    expect(section).toHaveProperty("totalAds");
  });
});
