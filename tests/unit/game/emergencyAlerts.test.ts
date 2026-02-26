import { describe, expect, it } from "vitest";
import {
  calculateSeverity,
  classifySeverityLevel,
  detectEmergencies,
  generateEmergencyBulletin,
  prioritizeAlerts,
} from "@/lib/game/emergencyAlerts";
import type { ColonyMetrics, EmergencyAlert } from "@/lib/game/emergencyAlerts";

const healthyMetrics: ColonyMetrics = {
  foodLevel: 80,
  waterLevel: 70,
  avgHealth: 85,
  sickCatCount: 1,
  totalCats: 10,
  shelterCapacity: 15,
  predatorThreat: 20,
};

describe("calculateSeverity", () => {
  it("returns 0 for healthy food level", () => {
    expect(calculateSeverity("famine", healthyMetrics)).toBe(0);
  });

  it("returns higher values for worse food conditions", () => {
    const low: ColonyMetrics = { ...healthyMetrics, foodLevel: 5 };
    const mid: ColonyMetrics = { ...healthyMetrics, foodLevel: 15 };
    const severeLow = calculateSeverity("famine", low);
    const severeMid = calculateSeverity("famine", mid);
    expect(severeLow).toBeGreaterThan(severeMid);
    expect(severeMid).toBeGreaterThan(0);
  });

  it("caps severity at 100", () => {
    const extreme: ColonyMetrics = { ...healthyMetrics, foodLevel: 0 };
    expect(calculateSeverity("famine", extreme)).toBeLessThanOrEqual(100);
  });

  it("calculates drought severity from water level", () => {
    const dry: ColonyMetrics = { ...healthyMetrics, waterLevel: 5 };
    expect(calculateSeverity("drought", dry)).toBeGreaterThan(50);
  });

  it("calculates epidemic severity from sick cat ratio", () => {
    const sick: ColonyMetrics = {
      ...healthyMetrics,
      sickCatCount: 8,
      totalCats: 10,
      avgHealth: 30,
    };
    expect(calculateSeverity("epidemic", sick)).toBeGreaterThan(50);
  });

  it("calculates overcrowding severity from population vs capacity", () => {
    const crowded: ColonyMetrics = {
      ...healthyMetrics,
      totalCats: 25,
      shelterCapacity: 15,
    };
    expect(calculateSeverity("overcrowding", crowded)).toBeGreaterThan(50);
  });

  it("calculates predator siege severity from threat level", () => {
    const besieged: ColonyMetrics = { ...healthyMetrics, predatorThreat: 90 };
    expect(calculateSeverity("predator_siege", besieged)).toBeGreaterThan(50);
  });
});

describe("classifySeverityLevel", () => {
  it("returns advisory for 0-25", () => {
    expect(classifySeverityLevel(0)).toBe("advisory");
    expect(classifySeverityLevel(10)).toBe("advisory");
    expect(classifySeverityLevel(25)).toBe("advisory");
  });

  it("returns warning for 26-50", () => {
    expect(classifySeverityLevel(26)).toBe("warning");
    expect(classifySeverityLevel(38)).toBe("warning");
    expect(classifySeverityLevel(50)).toBe("warning");
  });

  it("returns critical for 51-75", () => {
    expect(classifySeverityLevel(51)).toBe("critical");
    expect(classifySeverityLevel(63)).toBe("critical");
    expect(classifySeverityLevel(75)).toBe("critical");
  });

  it("returns catastrophic for 76-100", () => {
    expect(classifySeverityLevel(76)).toBe("catastrophic");
    expect(classifySeverityLevel(88)).toBe("catastrophic");
    expect(classifySeverityLevel(100)).toBe("catastrophic");
  });
});

describe("detectEmergencies", () => {
  it("returns empty array when all metrics are healthy", () => {
    expect(detectEmergencies(healthyMetrics)).toEqual([]);
  });

  it("detects famine when food < 20%", () => {
    const hungry: ColonyMetrics = { ...healthyMetrics, foodLevel: 10 };
    const alerts = detectEmergencies(hungry);
    expect(alerts).toHaveLength(1);
    expect(alerts[0].type).toBe("famine");
  });

  it("detects drought when water < 20%", () => {
    const dry: ColonyMetrics = { ...healthyMetrics, waterLevel: 10 };
    const alerts = detectEmergencies(dry);
    expect(alerts).toHaveLength(1);
    expect(alerts[0].type).toBe("drought");
  });

  it("detects epidemic when sick cats > 30% of population", () => {
    const sick: ColonyMetrics = {
      ...healthyMetrics,
      sickCatCount: 4,
      totalCats: 10,
      avgHealth: 40,
    };
    const alerts = detectEmergencies(sick);
    expect(alerts.some((a) => a.type === "epidemic")).toBe(true);
  });

  it("detects overcrowding when cats > shelterCapacity", () => {
    const crowded: ColonyMetrics = {
      ...healthyMetrics,
      totalCats: 20,
      shelterCapacity: 15,
    };
    const alerts = detectEmergencies(crowded);
    expect(alerts).toHaveLength(1);
    expect(alerts[0].type).toBe("overcrowding");
  });

  it("detects predator siege when threat > 60", () => {
    const besieged: ColonyMetrics = { ...healthyMetrics, predatorThreat: 75 };
    const alerts = detectEmergencies(besieged);
    expect(alerts).toHaveLength(1);
    expect(alerts[0].type).toBe("predator_siege");
  });

  it("can detect multiple simultaneous emergencies", () => {
    const crisis: ColonyMetrics = {
      foodLevel: 5,
      waterLevel: 5,
      avgHealth: 30,
      sickCatCount: 8,
      totalCats: 10,
      shelterCapacity: 5,
      predatorThreat: 90,
    };
    const alerts = detectEmergencies(crisis);
    expect(alerts.length).toBeGreaterThanOrEqual(3);
    const types = alerts.map((a) => a.type);
    expect(types).toContain("famine");
    expect(types).toContain("drought");
    expect(types).toContain("predator_siege");
  });
});

describe("prioritizeAlerts", () => {
  it("sorts by severity descending", () => {
    const alerts: EmergencyAlert[] = [
      { type: "famine", severity: 30, level: "warning", message: "Food low" },
      {
        type: "drought",
        severity: 80,
        level: "catastrophic",
        message: "Water critical",
      },
      {
        type: "epidemic",
        severity: 55,
        level: "critical",
        message: "Sickness spreading",
      },
    ];
    const sorted = prioritizeAlerts(alerts);
    expect(sorted[0].type).toBe("drought");
    expect(sorted[1].type).toBe("epidemic");
    expect(sorted[2].type).toBe("famine");
  });

  it("handles empty array", () => {
    expect(prioritizeAlerts([])).toEqual([]);
  });

  it("handles single alert", () => {
    const alerts: EmergencyAlert[] = [
      { type: "famine", severity: 50, level: "warning", message: "Food low" },
    ];
    const sorted = prioritizeAlerts(alerts);
    expect(sorted).toHaveLength(1);
    expect(sorted[0].type).toBe("famine");
  });
});

describe("generateEmergencyBulletin", () => {
  it("includes colony name", () => {
    const alerts: EmergencyAlert[] = [
      {
        type: "famine",
        severity: 60,
        level: "critical",
        message: "Food dangerously low",
      },
    ];
    const bulletin = generateEmergencyBulletin(alerts, "Whiskertown");
    expect(bulletin).toContain("Whiskertown");
  });

  it("handles 0 alerts (all clear)", () => {
    const bulletin = generateEmergencyBulletin([], "Whiskertown");
    expect(bulletin).toContain("Whiskertown");
    expect(bulletin.toLowerCase()).toMatch(/no.*emergenc|all clear|quiet/i);
  });

  it("handles 1 alert (singular)", () => {
    const alerts: EmergencyAlert[] = [
      {
        type: "drought",
        severity: 40,
        level: "warning",
        message: "Water supplies declining",
      },
    ];
    const bulletin = generateEmergencyBulletin(alerts, "Pawsville");
    expect(bulletin).toContain("Pawsville");
    expect(bulletin).toContain("Water supplies declining");
  });

  it("handles multiple alerts with summary", () => {
    const alerts: EmergencyAlert[] = [
      {
        type: "famine",
        severity: 80,
        level: "catastrophic",
        message: "Starvation imminent",
      },
      {
        type: "drought",
        severity: 60,
        level: "critical",
        message: "Water critical",
      },
      {
        type: "epidemic",
        severity: 40,
        level: "warning",
        message: "Sickness reported",
      },
    ];
    const bulletin = generateEmergencyBulletin(alerts, "Catford");
    expect(bulletin).toContain("Catford");
    expect(bulletin).toContain("Starvation imminent");
    expect(bulletin).toContain("Water critical");
    expect(bulletin).toContain("Sickness reported");
  });

  it("marks catastrophic alerts prominently", () => {
    const alerts: EmergencyAlert[] = [
      {
        type: "famine",
        severity: 90,
        level: "catastrophic",
        message: "Colony starving",
      },
    ];
    const bulletin = generateEmergencyBulletin(alerts, "Catford");
    expect(bulletin).toMatch(/CATASTROPHIC|!!!|BREAKING/i);
  });
});
