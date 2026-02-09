import { describe, expect, it } from "vitest";

import {
  getConditionSeverity,
  diagnoseCat,
  recommendTreatment,
  assessColonyHealth,
  getHealthGrade,
  generateMedicalBulletin,
} from "@/lib/game/healthDiagnosis";

import type { Diagnosis, CatDiagnosisEntry } from "@/lib/game/healthDiagnosis";

describe("healthDiagnosis", () => {
  // --- getConditionSeverity ---

  describe("getConditionSeverity", () => {
    it("returns null when value is above mild threshold", () => {
      expect(getConditionSeverity(50, 30, 15)).toBeNull();
    });

    it('returns "mild" when value is below mild threshold', () => {
      expect(getConditionSeverity(25, 30, 15)).toBe("mild");
    });

    it('returns "severe" when value is below severe threshold', () => {
      expect(getConditionSeverity(10, 30, 15)).toBe("severe");
    });

    it("handles boundary values exactly at thresholds", () => {
      // At mild threshold — not below, so null
      expect(getConditionSeverity(30, 30, 15)).toBeNull();
      // Just below mild
      expect(getConditionSeverity(29, 30, 15)).toBe("mild");
      // At severe threshold — not below, so mild
      expect(getConditionSeverity(15, 30, 15)).toBe("mild");
      // Just below severe
      expect(getConditionSeverity(14, 30, 15)).toBe("severe");
    });
  });

  // --- diagnoseCat ---

  describe("diagnoseCat", () => {
    it("returns empty array for healthy cat (all needs high)", () => {
      const result = diagnoseCat({
        hunger: 80,
        thirst: 90,
        rest: 70,
        health: 100,
      });
      expect(result).toEqual([]);
    });

    it("detects malnutrition from low hunger", () => {
      const result = diagnoseCat({
        hunger: 20,
        thirst: 90,
        rest: 70,
        health: 100,
      });
      expect(result).toEqual([
        { condition: "malnutrition", severity: "mild", value: 20 },
      ]);
    });

    it("detects dehydration from low thirst", () => {
      const result = diagnoseCat({
        hunger: 80,
        thirst: 10,
        rest: 70,
        health: 100,
      });
      expect(result).toEqual([
        { condition: "dehydration", severity: "severe", value: 10 },
      ]);
    });

    it("detects exhaustion from low rest", () => {
      const result = diagnoseCat({
        hunger: 80,
        thirst: 90,
        rest: 20,
        health: 100,
      });
      expect(result).toEqual([
        { condition: "exhaustion", severity: "mild", value: 20 },
      ]);
    });

    it("detects injury from low health", () => {
      const result = diagnoseCat({
        hunger: 80,
        thirst: 90,
        rest: 70,
        health: 20,
      });
      expect(result).toEqual([
        { condition: "injury", severity: "critical", value: 20 },
      ]);
    });

    it("detects multiple conditions simultaneously", () => {
      const result = diagnoseCat({
        hunger: 10,
        thirst: 5,
        rest: 8,
        health: 15,
      });
      expect(result).toHaveLength(4);
      expect(result.map((d) => d.condition)).toEqual([
        "malnutrition",
        "dehydration",
        "exhaustion",
        "injury",
      ]);
    });

    it("assigns correct severity levels", () => {
      const result = diagnoseCat({
        hunger: 25,
        thirst: 10,
        rest: 5,
        health: 40,
      });
      const severities = Object.fromEntries(
        result.map((d) => [d.condition, d.severity]),
      );
      expect(severities).toEqual({
        malnutrition: "mild", // 25 < 30, >= 15
        dehydration: "severe", // 10 < 15
        exhaustion: "severe", // 5 < 10
        injury: "mild", // 40 < 50, >= 25
      });
    });
  });

  // --- recommendTreatment ---

  describe("recommendTreatment", () => {
    it("returns appropriate treatment for each condition type", () => {
      const malnutrition = recommendTreatment({
        condition: "malnutrition",
        severity: "mild",
        value: 25,
      });
      const dehydration = recommendTreatment({
        condition: "dehydration",
        severity: "mild",
        value: 25,
      });
      const exhaustion = recommendTreatment({
        condition: "exhaustion",
        severity: "mild",
        value: 20,
      });
      const injury = recommendTreatment({
        condition: "injury",
        severity: "mild",
        value: 40,
      });

      expect(malnutrition.toLowerCase()).toContain("food");
      expect(dehydration.toLowerCase()).toContain("water");
      expect(exhaustion.toLowerCase()).toContain("rest");
      expect(injury.toLowerCase()).toContain("treat");
    });

    it("varies recommendation by severity", () => {
      const mild = recommendTreatment({
        condition: "malnutrition",
        severity: "mild",
        value: 25,
      });
      const severe = recommendTreatment({
        condition: "malnutrition",
        severity: "severe",
        value: 5,
      });

      expect(mild).not.toBe(severe);
      expect(severe.toLowerCase()).toMatch(/immediate|urgent|emergency/);
    });
  });

  // --- assessColonyHealth ---

  describe("assessColonyHealth", () => {
    const healthyEntry: CatDiagnosisEntry = {
      catName: "Whiskers",
      diagnoses: [],
      isHealthy: true,
    };

    const sickEntry: CatDiagnosisEntry = {
      catName: "Shadow",
      diagnoses: [{ condition: "malnutrition", severity: "mild", value: 20 }],
      isHealthy: false,
    };

    const criticalEntry: CatDiagnosisEntry = {
      catName: "Patch",
      diagnoses: [{ condition: "injury", severity: "critical", value: 10 }],
      isHealthy: false,
    };

    it("counts healthy vs sick cats", () => {
      const report = assessColonyHealth([
        healthyEntry,
        sickEntry,
        healthyEntry,
      ]);
      expect(report.totalCats).toBe(3);
      expect(report.healthyCats).toBe(2);
    });

    it("calculates healthy percentage", () => {
      const report = assessColonyHealth([
        healthyEntry,
        sickEntry,
        healthyEntry,
        healthyEntry,
      ]);
      expect(report.healthyPercent).toBeCloseTo(75);
    });

    it("counts conditions by type", () => {
      const multiSick: CatDiagnosisEntry = {
        catName: "Luna",
        diagnoses: [
          { condition: "malnutrition", severity: "mild", value: 25 },
          { condition: "dehydration", severity: "severe", value: 10 },
        ],
        isHealthy: false,
      };
      const report = assessColonyHealth([sickEntry, multiSick]);
      expect(report.conditionCounts.malnutrition).toBe(2);
      expect(report.conditionCounts.dehydration).toBe(1);
      expect(report.conditionCounts.exhaustion).toBe(0);
      expect(report.conditionCounts.injury).toBe(0);
    });

    it("counts severe and critical cases", () => {
      const severeEntry: CatDiagnosisEntry = {
        catName: "Felix",
        diagnoses: [{ condition: "exhaustion", severity: "severe", value: 5 }],
        isHealthy: false,
      };
      const report = assessColonyHealth([
        healthyEntry,
        severeEntry,
        criticalEntry,
      ]);
      expect(report.severeCases).toBe(1);
      expect(report.criticalCases).toBe(1);
    });

    it("finds most common condition", () => {
      const dehydrated: CatDiagnosisEntry = {
        catName: "Mittens",
        diagnoses: [{ condition: "dehydration", severity: "mild", value: 25 }],
        isHealthy: false,
      };
      const report = assessColonyHealth([sickEntry, dehydrated, dehydrated]);
      expect(report.mostCommonCondition).toBe("dehydration");
    });

    it("handles empty input", () => {
      const report = assessColonyHealth([]);
      expect(report.totalCats).toBe(0);
      expect(report.healthyCats).toBe(0);
      expect(report.healthyPercent).toBe(100);
      expect(report.severeCases).toBe(0);
      expect(report.criticalCases).toBe(0);
      expect(report.mostCommonCondition).toBeNull();
      expect(report.healthGrade).toBe("A");
    });
  });

  // --- getHealthGrade ---

  describe("getHealthGrade", () => {
    it("returns A for 90%+, B for 75%+, C for 50%+, D for 25%+, F below", () => {
      expect(getHealthGrade(95)).toBe("A");
      expect(getHealthGrade(90)).toBe("A");
      expect(getHealthGrade(80)).toBe("B");
      expect(getHealthGrade(75)).toBe("B");
      expect(getHealthGrade(60)).toBe("C");
      expect(getHealthGrade(50)).toBe("C");
      expect(getHealthGrade(30)).toBe("D");
      expect(getHealthGrade(25)).toBe("D");
      expect(getHealthGrade(20)).toBe("F");
      expect(getHealthGrade(0)).toBe("F");
    });
  });

  // --- generateMedicalBulletin ---

  describe("generateMedicalBulletin", () => {
    it("includes colony name", () => {
      const report = assessColonyHealth([]);
      const bulletin = generateMedicalBulletin(report, "Whisker Haven");
      expect(bulletin).toContain("Whisker Haven");
    });

    it("shows health grade", () => {
      const report = assessColonyHealth([]);
      const bulletin = generateMedicalBulletin(report, "Whisker Haven");
      expect(bulletin).toContain("A");
    });

    it("handles all-healthy colony (positive message)", () => {
      const healthy1: CatDiagnosisEntry = {
        catName: "A",
        diagnoses: [],
        isHealthy: true,
      };
      const healthy2: CatDiagnosisEntry = {
        catName: "B",
        diagnoses: [],
        isHealthy: true,
      };
      const report = assessColonyHealth([healthy1, healthy2]);
      const bulletin = generateMedicalBulletin(report, "Purrville");
      expect(bulletin.toLowerCase()).toMatch(/health|clean|excellent|perfect/);
    });

    it("highlights critical cases", () => {
      const critical: CatDiagnosisEntry = {
        catName: "Danger",
        diagnoses: [{ condition: "injury", severity: "critical", value: 10 }],
        isHealthy: false,
      };
      const report = assessColonyHealth([critical]);
      const bulletin = generateMedicalBulletin(report, "Catford");
      expect(bulletin.toLowerCase()).toMatch(/critical|emergency|urgent/);
    });
  });
});
