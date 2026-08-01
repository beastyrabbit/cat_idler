//! Persistent acquired-trait progress and capability modifier ordering from
//! `docs/leader-ai-overhaul/cats-and-care.md`.

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize};

use crate::planner_core::{BASIS_POINTS_SCALE, BasisPoints};

pub const BATTLE_HARDENED_DEPLOYMENTS: u32 = 5;
pub const DEVOTED_RITUALS: u32 = 20;
pub const SEASONED_SCHOLAR_INSIGHT: u64 = 200;
pub const CAREGIVER_TREATMENT_MINUTES: u64 = 100 * 60;
pub const BURNOUT_ONSET_MINUTES: u64 = 24 * 60;
pub const BURNOUT_RECOVERY_MINUTES: u64 = 72 * 60;
pub const PROSTHETIC_ADAPTED_MINUTES: u64 = 72 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcquiredTrait {
    Traumatized,
    BattleHardened,
    Devoted,
    SeasonedScholar,
    Caregiver,
    BurnedOut,
    ProstheticAdapted,
}

impl AcquiredTrait {
    pub const ALL: [Self; 7] = [
        Self::Traumatized,
        Self::BattleHardened,
        Self::Devoted,
        Self::SeasonedScholar,
        Self::Caregiver,
        Self::BurnedOut,
        Self::ProstheticAdapted,
    ];

    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Traumatized => "traumatized",
            Self::BattleHardened => "battle_hardened",
            Self::Devoted => "devoted",
            Self::SeasonedScholar => "seasoned_scholar",
            Self::Caregiver => "caregiver",
            Self::BurnedOut => "burned_out",
            Self::ProstheticAdapted => "prosthetic_adapted",
        }
    }

    #[must_use]
    pub const fn incompatible_with(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Traumatized, Self::BattleHardened) | (Self::BattleHardened, Self::Traumatized)
        )
    }
}

/// A stable ordered trait registry that enforces opposed-trait replacement.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct AcquiredTraitSet(BTreeSet<AcquiredTrait>);

impl AcquiredTraitSet {
    #[must_use]
    pub fn contains(&self, acquired: AcquiredTrait) -> bool {
        self.0.contains(&acquired)
    }

    pub fn insert(&mut self, acquired: AcquiredTrait) -> bool {
        self.0
            .retain(|existing| !acquired.incompatible_with(*existing));
        self.0.insert(acquired)
    }

    pub fn remove(&mut self, acquired: AcquiredTrait) -> bool {
        self.0.remove(&acquired)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = AcquiredTrait> + '_ {
        self.0.iter().copied()
    }
}

impl<'de> Deserialize<'de> for AcquiredTraitSet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let traits = BTreeSet::<AcquiredTrait>::deserialize(deserializer)?;
        for acquired in &traits {
            if traits
                .iter()
                .any(|other| acquired != other && acquired.incompatible_with(*other))
            {
                return Err(serde::de::Error::custom(
                    "acquired trait set contains incompatible traits",
                ));
            }
        }
        Ok(Self(traits))
    }
}

/// Persistent counters are integers in game minutes or completed units.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcquiredTraitState {
    #[serde(default)]
    pub traits: AcquiredTraitSet,
    #[serde(default)]
    combat_deployments_without_severe: u32,
    #[serde(default)]
    completed_shrine_rituals: u32,
    #[serde(default)]
    insight_produced: u64,
    #[serde(default)]
    completed_treatment_minutes: u64,
    #[serde(default)]
    high_stress_minutes: u64,
    #[serde(default)]
    below_forty_recovery_minutes: u64,
    #[serde(default)]
    productive_prosthetic_minutes: u64,
}

impl AcquiredTraitState {
    pub fn record_severe_or_missing_injury(&mut self) {
        self.combat_deployments_without_severe = 0;
        self.traits.insert(AcquiredTrait::Traumatized);
    }

    pub fn record_raid_defeat(&mut self) {
        self.combat_deployments_without_severe = 0;
        self.traits.insert(AcquiredTrait::Traumatized);
    }

    pub fn record_combat_deployment_without_severe(&mut self) {
        self.combat_deployments_without_severe =
            self.combat_deployments_without_severe.saturating_add(1);
        if self.combat_deployments_without_severe >= BATTLE_HARDENED_DEPLOYMENTS {
            self.traits.insert(AcquiredTrait::BattleHardened);
        }
    }

    pub fn record_completed_shrine_rituals(&mut self, count: u32) {
        self.completed_shrine_rituals = self.completed_shrine_rituals.saturating_add(count);
        if self.completed_shrine_rituals >= DEVOTED_RITUALS {
            self.traits.insert(AcquiredTrait::Devoted);
        }
    }

    pub fn record_insight_produced(&mut self, insight: u64) {
        self.insight_produced = self.insight_produced.saturating_add(insight);
        if self.insight_produced >= SEASONED_SCHOLAR_INSIGHT {
            self.traits.insert(AcquiredTrait::SeasonedScholar);
        }
    }

    pub fn record_completed_treatment_minutes(&mut self, minutes: u64) {
        self.completed_treatment_minutes = self.completed_treatment_minutes.saturating_add(minutes);
        if self.completed_treatment_minutes >= CAREGIVER_TREATMENT_MINUTES {
            self.traits.insert(AcquiredTrait::Caregiver);
        }
    }

    pub fn record_productive_prosthetic_minutes(&mut self, minutes: u64) {
        self.productive_prosthetic_minutes =
            self.productive_prosthetic_minutes.saturating_add(minutes);
        if self.productive_prosthetic_minutes >= PROSTHETIC_ADAPTED_MINUTES {
            self.traits.insert(AcquiredTrait::ProstheticAdapted);
        }
    }

    /// Advance continuous burnout onset/recovery clocks. Stress exactly 90
    /// counts toward onset; recovery requires strictly less than 40.
    pub fn advance_stress_time(&mut self, stress: u8, minutes: u64) {
        if stress >= 90 {
            self.high_stress_minutes = self.high_stress_minutes.saturating_add(minutes);
        } else {
            self.high_stress_minutes = 0;
        }
        if self.high_stress_minutes >= BURNOUT_ONSET_MINUTES {
            self.traits.insert(AcquiredTrait::BurnedOut);
        }

        if self.traits.contains(AcquiredTrait::BurnedOut) {
            if stress < 40 {
                self.below_forty_recovery_minutes =
                    self.below_forty_recovery_minutes.saturating_add(minutes);
                if self.below_forty_recovery_minutes >= BURNOUT_RECOVERY_MINUTES {
                    self.traits.remove(AcquiredTrait::BurnedOut);
                    self.high_stress_minutes = 0;
                    self.below_forty_recovery_minutes = 0;
                }
            } else {
                self.below_forty_recovery_minutes = 0;
            }
        } else {
            self.below_forty_recovery_minutes = 0;
        }
    }

    #[must_use]
    pub fn combat_risk_stress_factor(&self) -> BasisPoints {
        if self.traits.contains(AcquiredTrait::BattleHardened) {
            BasisPoints::new(7_500)
        } else if self.traits.contains(AcquiredTrait::Traumatized) {
            BasisPoints::new(12_500)
        } else {
            BasisPoints::new(BASIS_POINTS_SCALE)
        }
    }

    #[must_use]
    pub fn shrine_willingness_factor(&self) -> BasisPoints {
        self.factor_if(AcquiredTrait::Devoted, 12_000)
    }

    #[must_use]
    pub fn insight_production_factor(&self) -> BasisPoints {
        self.factor_if(AcquiredTrait::SeasonedScholar, 11_000)
    }

    #[must_use]
    pub fn medicine_effectiveness_factor(&self) -> BasisPoints {
        self.factor_if(AcquiredTrait::Caregiver, 11_000)
    }

    #[must_use]
    pub fn non_emergency_willingness_factor(&self) -> BasisPoints {
        self.factor_if(AcquiredTrait::BurnedOut, 7_500)
    }

    /// Additive restoration in basis points: +1,000 is ten percentage points.
    #[must_use]
    pub fn prosthetic_restoration_bonus(&self) -> BasisPoints {
        if self.traits.contains(AcquiredTrait::ProstheticAdapted) {
            BasisPoints::new(1_000)
        } else {
            BasisPoints::new(0)
        }
    }

    fn factor_if(&self, acquired: AcquiredTrait, active: i64) -> BasisPoints {
        if self.traits.contains(acquired) {
            BasisPoints::new(active)
        } else {
            BasisPoints::new(BASIS_POINTS_SCALE)
        }
    }
}

/// Fixed-point inputs in the authoritative modifier order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityModifierFactors {
    pub anatomy: BasisPoints,
    pub prosthetic_restoration: BasisPoints,
    pub innate_attribute: BasisPoints,
    pub personality: BasisPoints,
    pub acquired_trait: BasisPoints,
    pub stress: BasisPoints,
    pub divine_boost: BasisPoints,
}

/// Intermediate values make the exact order inspectable by protocol/UI later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityModifierBreakdown {
    pub base: i64,
    pub after_anatomy: i64,
    pub after_prosthetic_restoration: i64,
    pub after_innate_attribute: i64,
    pub after_personality: i64,
    pub after_acquired_trait: i64,
    pub after_stress: i64,
    pub after_divine_boost: i64,
}

impl CapabilityModifierFactors {
    #[must_use]
    pub fn apply(self, base: i64) -> CapabilityModifierBreakdown {
        let after_anatomy = apply_factor(base, self.anatomy);
        let after_prosthetic_restoration = apply_factor(after_anatomy, self.prosthetic_restoration);
        let after_innate_attribute =
            apply_factor(after_prosthetic_restoration, self.innate_attribute);
        let after_personality = apply_factor(after_innate_attribute, self.personality);
        let after_acquired_trait = apply_factor(after_personality, self.acquired_trait);
        let after_stress = apply_factor(after_acquired_trait, self.stress);
        let after_divine_boost = apply_factor(after_stress, self.divine_boost);
        CapabilityModifierBreakdown {
            base,
            after_anatomy,
            after_prosthetic_restoration,
            after_innate_attribute,
            after_personality,
            after_acquired_trait,
            after_stress,
            after_divine_boost,
        }
    }
}

fn apply_factor(value: i64, factor: BasisPoints) -> i64 {
    let adjusted =
        i128::from(value).saturating_mul(i128::from(factor.get())) / i128::from(BASIS_POINTS_SCALE);
    adjusted.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_catalog_and_opposed_replacement_are_exact() {
        let ids: Vec<_> = AcquiredTrait::ALL
            .into_iter()
            .map(AcquiredTrait::stable_id)
            .collect();
        assert_eq!(ids.len(), 7);
        let mut set = AcquiredTraitSet::default();
        set.insert(AcquiredTrait::Traumatized);
        set.insert(AcquiredTrait::BattleHardened);
        assert!(!set.contains(AcquiredTrait::Traumatized));
        assert!(set.contains(AcquiredTrait::BattleHardened));
        assert!(
            serde_json::from_str::<AcquiredTraitSet>(r#"["traumatized","battle_hardened"]"#)
                .is_err()
        );
    }

    #[test]
    fn trauma_and_battle_hardened_triggers_replace_each_other() {
        let mut state = AcquiredTraitState::default();
        state.record_severe_or_missing_injury();
        assert_eq!(state.combat_risk_stress_factor().get(), 12_500);
        for _ in 0..BATTLE_HARDENED_DEPLOYMENTS {
            state.record_combat_deployment_without_severe();
        }
        assert_eq!(state.combat_risk_stress_factor().get(), 7_500);
        state.record_raid_defeat();
        assert_eq!(state.combat_risk_stress_factor().get(), 12_500);
    }

    #[test]
    fn unit_and_minute_thresholds_do_not_trigger_early() {
        let mut state = AcquiredTraitState::default();
        state.record_completed_shrine_rituals(DEVOTED_RITUALS - 1);
        state.record_insight_produced(SEASONED_SCHOLAR_INSIGHT - 1);
        state.record_completed_treatment_minutes(CAREGIVER_TREATMENT_MINUTES - 1);
        state.record_productive_prosthetic_minutes(PROSTHETIC_ADAPTED_MINUTES - 1);
        assert_eq!(state.traits.iter().len(), 0);

        state.record_completed_shrine_rituals(1);
        state.record_insight_produced(1);
        state.record_completed_treatment_minutes(1);
        state.record_productive_prosthetic_minutes(1);
        assert!(state.traits.contains(AcquiredTrait::Devoted));
        assert!(state.traits.contains(AcquiredTrait::SeasonedScholar));
        assert!(state.traits.contains(AcquiredTrait::Caregiver));
        assert!(state.traits.contains(AcquiredTrait::ProstheticAdapted));
        assert_eq!(state.shrine_willingness_factor().get(), 12_000);
        assert_eq!(state.insight_production_factor().get(), 11_000);
        assert_eq!(state.medicine_effectiveness_factor().get(), 11_000);
        assert_eq!(state.prosthetic_restoration_bonus().get(), 1_000);
    }

    #[test]
    fn burnout_requires_continuous_onset_and_strict_recovery() {
        let mut state = AcquiredTraitState::default();
        state.advance_stress_time(90, BURNOUT_ONSET_MINUTES - 1);
        state.advance_stress_time(89, 1);
        state.advance_stress_time(100, BURNOUT_ONSET_MINUTES);
        assert!(state.traits.contains(AcquiredTrait::BurnedOut));
        assert_eq!(state.non_emergency_willingness_factor().get(), 7_500);

        state.advance_stress_time(39, BURNOUT_RECOVERY_MINUTES - 1);
        state.advance_stress_time(40, 1);
        state.advance_stress_time(39, BURNOUT_RECOVERY_MINUTES);
        assert!(!state.traits.contains(AcquiredTrait::BurnedOut));
    }

    #[test]
    fn burnout_progress_is_partition_invariant() {
        let mut whole = AcquiredTraitState::default();
        let mut split = AcquiredTraitState::default();
        whole.advance_stress_time(95, BURNOUT_ONSET_MINUTES);
        for minutes in [1, 59, 600, 780] {
            split.advance_stress_time(95, minutes);
        }
        assert_eq!(whole, split);
    }

    #[test]
    fn capability_pipeline_exposes_exact_modifier_order() {
        let factors = CapabilityModifierFactors {
            anatomy: BasisPoints::new(5_000),
            prosthetic_restoration: BasisPoints::new(15_000),
            innate_attribute: BasisPoints::new(20_000),
            personality: BasisPoints::new(5_000),
            acquired_trait: BasisPoints::new(20_000),
            stress: BasisPoints::new(5_000),
            divine_boost: BasisPoints::new(20_000),
        };
        assert_eq!(
            factors.apply(1_000),
            CapabilityModifierBreakdown {
                base: 1_000,
                after_anatomy: 500,
                after_prosthetic_restoration: 750,
                after_innate_attribute: 1_500,
                after_personality: 750,
                after_acquired_trait: 1_500,
                after_stress: 750,
                after_divine_boost: 1_500,
            }
        );
    }

    #[test]
    fn old_persisted_state_defaults_new_progress_fields() {
        let state: AcquiredTraitState = serde_json::from_str(r#"{"traits":[]}"#).unwrap();
        assert_eq!(state, AcquiredTraitState::default());
    }
}
