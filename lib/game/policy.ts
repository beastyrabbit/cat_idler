import type { PolicyTier } from "@/types/game";

export type LeaderPolicyBucket = "bad" | "normal" | "excellent";

export interface PolicyWeights {
  simple: number;
  normal: number;
  excellent: number;
}

export interface PolicyConfig {
  tier: PolicyTier;
  actionReliability: number;
  needsDecayMultiplier: number;
  needsDamageMultiplier: number;
  foodEmergencyThreshold: number;
  waterEmergencyThreshold: number;
  houseWaterRequired: number;
  houseMaterialsRequired: number;
}

const WEIGHTS_BY_BUCKET: Record<LeaderPolicyBucket, PolicyWeights> = {
  bad: { simple: 0.3, normal: 0.6, excellent: 0.1 },
  normal: { simple: 0.1, normal: 0.8, excellent: 0.1 },
  excellent: { simple: 0, normal: 0.7, excellent: 0.3 },
};

const CONFIG_BY_TIER: Record<PolicyTier, PolicyConfig> = {
  simple: {
    tier: "simple",
    actionReliability: 0.6,
    needsDecayMultiplier: 1.25,
    needsDamageMultiplier: 1.3,
    foodEmergencyThreshold: 8,
    waterEmergencyThreshold: 8,
    houseWaterRequired: 10,
    houseMaterialsRequired: 12,
  },
  normal: {
    tier: "normal",
    actionReliability: 0.9,
    needsDecayMultiplier: 1,
    needsDamageMultiplier: 1,
    foodEmergencyThreshold: 12,
    waterEmergencyThreshold: 12,
    houseWaterRequired: 8,
    houseMaterialsRequired: 10,
  },
  excellent: {
    tier: "excellent",
    actionReliability: 1,
    needsDecayMultiplier: 0.85,
    needsDamageMultiplier: 0.8,
    foodEmergencyThreshold: 16,
    waterEmergencyThreshold: 16,
    houseWaterRequired: 6,
    houseMaterialsRequired: 8,
  },
};

export function bucketFromLeadership(leadership: number): LeaderPolicyBucket {
  if (leadership < 35) {
    return "bad";
  }
  if (leadership < 70) {
    return "normal";
  }
  return "excellent";
}

export function weightsForLeadership(leadership: number): PolicyWeights {
  return WEIGHTS_BY_BUCKET[bucketFromLeadership(leadership)];
}

export function pickPolicyTier(leadership: number, roll: number): PolicyTier {
  const weights = weightsForLeadership(leadership);
  const clampedRoll = Math.max(0, Math.min(1, roll));

  if (clampedRoll < weights.simple) {
    return "simple";
  }

  if (clampedRoll < weights.simple + weights.normal) {
    return "normal";
  }

  return "excellent";
}

export function configForTier(tier: PolicyTier): PolicyConfig {
  return CONFIG_BY_TIER[tier];
}
