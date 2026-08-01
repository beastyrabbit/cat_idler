//! Persistent cat stress lifecycle specified by
//! `docs/leader-ai-overhaul/cats-and-care.md`.

use serde::{Deserialize, Deserializer, Serialize};

use crate::cat_traits::CatPersonality;

pub const STRESS_MIN: u8 = 0;
pub const STRESS_MAX: u8 = 100;
pub const ROLLING_DAY_MINUTES: u32 = 24 * 60;
pub const BASE_WORK_MINUTES: u32 = 8 * 60;
pub const OVERWORK_STEP_MINUTES: u32 = 2 * 60;
pub const REST_STEP_MINUTES: u32 = 60;

/// Stress bands are exhaustive and ordered by increasing severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StressBand {
    Normal,
    Reduced,
    RefusalRisk,
    Critical,
}

/// A validated persisted stress value on the canonical 0–100 scale.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct StressLevel(u8);

impl StressLevel {
    #[must_use]
    pub const fn new_clamped(value: i32) -> Self {
        if value < STRESS_MIN as i32 {
            Self(STRESS_MIN)
        } else if value > STRESS_MAX as i32 {
            Self(STRESS_MAX)
        } else {
            Self(value as u8)
        }
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }

    #[must_use]
    pub const fn band(self) -> StressBand {
        match self.0 {
            0..=59 => StressBand::Normal,
            60..=79 => StressBand::Reduced,
            80..=94 => StressBand::RefusalRisk,
            _ => StressBand::Critical,
        }
    }

    fn add_clamped(&mut self, delta: i32) {
        *self = Self::new_clamped(i32::from(self.0).saturating_add(delta));
    }
}

impl<'de> Deserialize<'de> for StressLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u8::deserialize(deserializer)?;
        if value <= STRESS_MAX {
            Ok(Self(value))
        } else {
            Err(serde::de::Error::custom(format_args!(
                "stress must be in {STRESS_MIN}..={STRESS_MAX}, got {value}"
            )))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StressEvent {
    MinorInjury,
    SevereInjury,
    MissingPart,
    RaidDefeat,
}

impl StressEvent {
    #[must_use]
    pub const fn increase(self) -> u8 {
        match self {
            Self::MinorInjury => 10,
            Self::SevereInjury => 25,
            Self::MissingPart => 35,
            Self::RaidDefeat => 15,
        }
    }
}

/// Persisted leaf state. Minute remainders make recovery independent of tick
/// partitioning; the rolling work window itself remains owned by LAI.23.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StressState {
    #[serde(default)]
    pub level: StressLevel,
    #[serde(default)]
    safe_rest_remainder_minutes: u32,
    #[serde(default)]
    social_rest_remainder_minutes: u32,
}

impl StressState {
    #[must_use]
    pub const fn new(level: StressLevel) -> Self {
        Self {
            level,
            safe_rest_remainder_minutes: 0,
            social_rest_remainder_minutes: 0,
        }
    }

    pub fn apply_event(&mut self, event: StressEvent) {
        self.level.add_clamped(i32::from(event.increase()));
    }

    /// Apply only newly crossed two-hour buckets beyond eight hours in the
    /// externally maintained rolling game-day. A shrinking window never adds
    /// stress; LAI.23 supplies consecutive before/after totals.
    pub fn apply_rolling_work_change(
        &mut self,
        previous_work_minutes: u32,
        current_work_minutes: u32,
    ) {
        let bucket = |minutes: u32| {
            minutes
                .min(ROLLING_DAY_MINUTES)
                .saturating_sub(BASE_WORK_MINUTES)
                / OVERWORK_STEP_MINUTES
        };
        let crossed = bucket(current_work_minutes).saturating_sub(bucket(previous_work_minutes));
        self.level
            .add_clamped(i32::try_from(crossed).unwrap_or(i32::MAX));
    }

    /// Apply safe-rest recovery. Social recovery is cumulative only while the
    /// rest is compatible and the cat is Gregarious (positive signed axis).
    pub fn apply_safe_rest(
        &mut self,
        minutes: u32,
        compatible_social_rest: bool,
        personality: CatPersonality,
    ) {
        self.safe_rest_remainder_minutes = self.safe_rest_remainder_minutes.saturating_add(minutes);
        let safe_hours = self.safe_rest_remainder_minutes / REST_STEP_MINUTES;
        self.safe_rest_remainder_minutes %= REST_STEP_MINUTES;

        let gregarious = personality.solitary_gregarious.signed_level() > 0;
        let social_hours = if compatible_social_rest && gregarious {
            self.social_rest_remainder_minutes =
                self.social_rest_remainder_minutes.saturating_add(minutes);
            let hours = self.social_rest_remainder_minutes / REST_STEP_MINUTES;
            self.social_rest_remainder_minutes %= REST_STEP_MINUTES;
            hours
        } else {
            self.social_rest_remainder_minutes = 0;
            0
        };

        let recovery = safe_hours.saturating_mul(2).saturating_add(social_hours);
        self.level
            .add_clamped(-i32::try_from(recovery).unwrap_or(i32::MAX));
    }

    /// Unsafe activity breaks partial safe/social rest without granting
    /// recovery.
    pub fn interrupt_rest(&mut self) {
        self.safe_rest_remainder_minutes = 0;
        self.social_rest_remainder_minutes = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cat_traits::{PersonalityPole, PersonalityStrength, PersonalityValue};

    fn gregarious() -> CatPersonality {
        CatPersonality {
            solitary_gregarious: PersonalityValue::new(
                PersonalityPole::Positive,
                PersonalityStrength::Subtle,
            ),
            ..CatPersonality::default()
        }
    }

    #[test]
    fn stress_bands_cover_exact_boundaries() {
        for (value, band) in [
            (0, StressBand::Normal),
            (59, StressBand::Normal),
            (60, StressBand::Reduced),
            (79, StressBand::Reduced),
            (80, StressBand::RefusalRisk),
            (94, StressBand::RefusalRisk),
            (95, StressBand::Critical),
            (100, StressBand::Critical),
        ] {
            assert_eq!(StressLevel::new_clamped(value).band(), band);
        }
    }

    #[test]
    fn stress_events_use_exact_additions_and_clamp() {
        let mut state = StressState::default();
        state.apply_event(StressEvent::MinorInjury);
        state.apply_event(StressEvent::SevereInjury);
        state.apply_event(StressEvent::MissingPart);
        state.apply_event(StressEvent::RaidDefeat);
        assert_eq!(state.level.get(), 85);
        state.apply_event(StressEvent::SevereInjury);
        assert_eq!(state.level.get(), 100);
    }

    #[test]
    fn overwork_adds_once_per_new_full_two_hour_bucket() {
        let mut state = StressState::default();
        state.apply_rolling_work_change(479, 599);
        assert_eq!(state.level.get(), 0);
        state.apply_rolling_work_change(599, 600);
        state.apply_rolling_work_change(600, 719);
        state.apply_rolling_work_change(719, 720);
        assert_eq!(state.level.get(), 2);
        state.apply_rolling_work_change(720, 480);
        assert_eq!(state.level.get(), 2);
    }

    #[test]
    fn safe_and_compatible_gregarious_rest_is_partition_invariant() {
        let mut whole = StressState::new(StressLevel::new_clamped(20));
        let mut split = whole;
        whole.apply_safe_rest(180, true, gregarious());
        for minutes in [17, 43, 61, 59] {
            split.apply_safe_rest(minutes, true, gregarious());
        }
        assert_eq!(whole, split);
        assert_eq!(whole.level.get(), 11);
    }

    #[test]
    fn social_bonus_requires_compatible_gregarious_rest() {
        let mut solitary = StressState::new(StressLevel::new_clamped(10));
        solitary.apply_safe_rest(60, true, CatPersonality::default());
        assert_eq!(solitary.level.get(), 8);

        let mut incompatible = StressState::new(StressLevel::new_clamped(10));
        incompatible.apply_safe_rest(60, false, gregarious());
        assert_eq!(incompatible.level.get(), 8);
    }

    #[test]
    fn rest_interrupt_discards_partial_hour_and_recovery_clamps_at_zero() {
        let mut state = StressState::new(StressLevel::new_clamped(2));
        state.apply_safe_rest(59, true, gregarious());
        state.interrupt_rest();
        state.apply_safe_rest(60, true, gregarious());
        assert_eq!(state.level.get(), 0);
    }

    #[test]
    fn persisted_stress_rejects_out_of_range_values_and_defaults_remainders() {
        let state: StressState = serde_json::from_str(r#"{"level":42}"#).unwrap();
        assert_eq!(state.level.get(), 42);
        assert!(serde_json::from_str::<StressState>(r#"{"level":101}"#).is_err());
    }
}
