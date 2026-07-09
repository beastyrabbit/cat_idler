//! Policy tier selection ported from `lib/game/policy.ts`.

use serde::{Deserialize, Serialize};

use crate::types::PolicyTier;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LeaderPolicyBucket {
    Bad,
    Normal,
    Excellent,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PolicyWeights {
    pub simple: f64,
    pub normal: f64,
    pub excellent: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyConfig {
    pub tier: PolicyTier,
    pub action_reliability: f64,
    pub needs_decay_multiplier: f64,
    pub needs_damage_multiplier: f64,
    pub food_emergency_threshold: f64,
    pub water_emergency_threshold: f64,
    pub house_water_required: f64,
    pub house_materials_required: f64,
}

const BAD_WEIGHTS: PolicyWeights = PolicyWeights {
    simple: 0.3,
    normal: 0.6,
    excellent: 0.1,
};

const NORMAL_WEIGHTS: PolicyWeights = PolicyWeights {
    simple: 0.1,
    normal: 0.8,
    excellent: 0.1,
};

const EXCELLENT_WEIGHTS: PolicyWeights = PolicyWeights {
    simple: 0.0,
    normal: 0.7,
    excellent: 0.3,
};

const SIMPLE_CONFIG: PolicyConfig = PolicyConfig {
    tier: PolicyTier::Simple,
    action_reliability: 0.6,
    needs_decay_multiplier: 1.25,
    needs_damage_multiplier: 1.3,
    food_emergency_threshold: 8.0,
    water_emergency_threshold: 8.0,
    house_water_required: 10.0,
    house_materials_required: 12.0,
};

const NORMAL_CONFIG: PolicyConfig = PolicyConfig {
    tier: PolicyTier::Normal,
    action_reliability: 0.9,
    needs_decay_multiplier: 1.0,
    needs_damage_multiplier: 1.0,
    food_emergency_threshold: 12.0,
    water_emergency_threshold: 12.0,
    house_water_required: 8.0,
    house_materials_required: 10.0,
};

const EXCELLENT_CONFIG: PolicyConfig = PolicyConfig {
    tier: PolicyTier::Excellent,
    action_reliability: 1.0,
    needs_decay_multiplier: 0.85,
    needs_damage_multiplier: 0.8,
    food_emergency_threshold: 16.0,
    water_emergency_threshold: 16.0,
    house_water_required: 6.0,
    house_materials_required: 8.0,
};

#[must_use]
pub fn bucket_from_leadership(leadership: f64) -> LeaderPolicyBucket {
    if leadership < 35.0 {
        return LeaderPolicyBucket::Bad;
    }
    if leadership < 70.0 {
        return LeaderPolicyBucket::Normal;
    }
    LeaderPolicyBucket::Excellent
}

#[must_use]
pub fn weights_for_leadership(leadership: f64) -> PolicyWeights {
    match bucket_from_leadership(leadership) {
        LeaderPolicyBucket::Bad => BAD_WEIGHTS,
        LeaderPolicyBucket::Normal => NORMAL_WEIGHTS,
        LeaderPolicyBucket::Excellent => EXCELLENT_WEIGHTS,
    }
}

#[must_use]
pub fn pick_policy_tier(leadership: f64, roll: f64) -> PolicyTier {
    let weights = weights_for_leadership(leadership);
    let clamped_roll = clamp_roll_like_ts(roll);

    if clamped_roll < weights.simple {
        return PolicyTier::Simple;
    }
    if clamped_roll < weights.simple + weights.normal {
        return PolicyTier::Normal;
    }
    PolicyTier::Excellent
}

#[must_use]
pub fn config_for_tier(tier: PolicyTier) -> PolicyConfig {
    match tier {
        PolicyTier::Simple => SIMPLE_CONFIG,
        PolicyTier::Normal => NORMAL_CONFIG,
        PolicyTier::Excellent => EXCELLENT_CONFIG,
    }
}

fn clamp_roll_like_ts(roll: f64) -> f64 {
    roll.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde::Deserialize;

    use super::{
        LeaderPolicyBucket, PolicyConfig, PolicyWeights, bucket_from_leadership, config_for_tier,
        pick_policy_tier, weights_for_leadership,
    };
    use crate::{rng::roll_seeded, types::PolicyTier};

    #[derive(Debug, Deserialize)]
    struct Fixture {
        source: String,
        tiers: Vec<PolicyTier>,
        leadership: Vec<LeadershipCase>,
        configs: HashMap<PolicyTier, PolicyConfig>,
        picks: Vec<PickCase>,
        #[serde(rename = "seededPicks")]
        seeded_picks: Vec<SeededPickCase>,
    }

    #[derive(Debug, Deserialize)]
    struct LeadershipCase {
        leadership: f64,
        bucket: LeaderPolicyBucket,
        weights: PolicyWeights,
    }

    #[derive(Debug, Deserialize)]
    struct PickCase {
        leadership: f64,
        roll: f64,
        tier: PolicyTier,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SeededPickCase {
        leadership: f64,
        seed: f64,
        roll: f64,
        next_seed: u32,
        tier: PolicyTier,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../docs/migration/fixtures/p3/policy.json"
        ))
        .expect("policy fixture parses")
    }

    #[test]
    fn fixture_is_generated_from_policy_ts() {
        let fixture = fixture();

        assert_eq!(fixture.source, "lib/game/policy.ts");
        assert_eq!(
            fixture.tiers,
            [
                PolicyTier::Simple,
                PolicyTier::Normal,
                PolicyTier::Excellent
            ]
        );
    }

    #[test]
    fn leadership_buckets_and_weights_match_ts_fixture() {
        for case in fixture().leadership {
            assert_eq!(bucket_from_leadership(case.leadership), case.bucket);
            assert_eq!(weights_for_leadership(case.leadership), case.weights);
        }
    }

    #[test]
    fn configs_match_ts_fixture() {
        for (tier, expected) in fixture().configs {
            assert_eq!(config_for_tier(tier), expected);
        }
    }

    #[test]
    fn pick_policy_tier_matches_ts_roll_thresholds() {
        for case in fixture().picks {
            assert_eq!(pick_policy_tier(case.leadership, case.roll), case.tier);
        }
    }

    #[test]
    fn pick_policy_tier_matches_seeded_ts_rolls() {
        for case in fixture().seeded_picks {
            let roll = roll_seeded(case.seed);

            assert_eq!(roll.value, case.roll);
            assert_eq!(roll.next_seed, case.next_seed);
            assert_eq!(pick_policy_tier(case.leadership, roll.value), case.tier);
        }
    }

    #[test]
    fn nan_inputs_follow_ts_comparison_fallthrough() {
        assert_eq!(
            bucket_from_leadership(f64::NAN),
            LeaderPolicyBucket::Excellent
        );
        assert_eq!(pick_policy_tier(10.0, f64::NAN), PolicyTier::Excellent);
    }
}
