//! Test-acceleration presets ported from `lib/game/testAcceleration.ts`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TestAccelerationPreset {
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "fast")]
    Fast,
    #[serde(rename = "turbo")]
    Turbo,
    #[serde(rename = "hyper")]
    Hyper,
    #[serde(rename = "ludicrous")]
    Ludicrous,
}

impl TestAccelerationPreset {
    pub const ALL: &'static [Self] = &[
        Self::Off,
        Self::Fast,
        Self::Turbo,
        Self::Hyper,
        Self::Ludicrous,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestAccelerationConfig {
    pub time_scale: f64,
    pub resource_decay_multiplier: f64,
    pub resilience_hours_override: Option<f64>,
    pub critical_ms_override: u32,
}

#[must_use]
pub const fn config_for_preset(preset: TestAccelerationPreset) -> TestAccelerationConfig {
    match preset {
        TestAccelerationPreset::Off => TestAccelerationConfig {
            time_scale: 1.0,
            resource_decay_multiplier: 1.0,
            resilience_hours_override: None,
            critical_ms_override: 5 * 60 * 1000,
        },
        TestAccelerationPreset::Fast => TestAccelerationConfig {
            time_scale: 20.0,
            resource_decay_multiplier: 20.0,
            resilience_hours_override: Some(0.05),
            critical_ms_override: 20_000,
        },
        TestAccelerationPreset::Turbo => TestAccelerationConfig {
            time_scale: 120.0,
            resource_decay_multiplier: 120.0,
            resilience_hours_override: Some(0.0),
            critical_ms_override: 5_000,
        },
        TestAccelerationPreset::Hyper => TestAccelerationConfig {
            time_scale: 100.0,
            resource_decay_multiplier: 20.0,
            resilience_hours_override: Some(0.0),
            critical_ms_override: 10_000,
        },
        TestAccelerationPreset::Ludicrous => TestAccelerationConfig {
            time_scale: 10_000.0,
            resource_decay_multiplier: 20.0,
            resilience_hours_override: Some(0.0),
            critical_ms_override: 10_000,
        },
    }
}

#[must_use]
pub fn preset_from_time_scale(time_scale: Option<f64>) -> TestAccelerationPreset {
    let scale = time_scale.unwrap_or(1.0);

    if scale >= 10_000.0 {
        return TestAccelerationPreset::Ludicrous;
    }
    if (120.0..10_000.0).contains(&scale) && scale.to_bits() != 100.0f64.to_bits() {
        return TestAccelerationPreset::Turbo;
    }
    if scale.to_bits() == 100.0f64.to_bits() {
        return TestAccelerationPreset::Hyper;
    }
    if scale >= 20.0 {
        return TestAccelerationPreset::Fast;
    }
    TestAccelerationPreset::Off
}

#[cfg(test)]
mod tests {
    use super::{
        TestAccelerationConfig, TestAccelerationPreset, config_for_preset, preset_from_time_scale,
    };

    fn assert_f64_literal(actual: f64, expected: f64) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }

    fn assert_config(actual: TestAccelerationConfig, expected: TestAccelerationConfig) {
        assert_f64_literal(actual.time_scale, expected.time_scale);
        assert_f64_literal(
            actual.resource_decay_multiplier,
            expected.resource_decay_multiplier,
        );
        assert_eq!(actual.critical_ms_override, expected.critical_ms_override);
        match (
            actual.resilience_hours_override,
            expected.resilience_hours_override,
        ) {
            (Some(actual), Some(expected)) => assert_f64_literal(actual, expected),
            (None, None) => {}
            other => panic!("resilience_hours_override mismatch: {other:?}"),
        }
    }

    #[test]
    fn config_for_preset_matches_typescript_for_every_preset() {
        let expected = [
            (
                TestAccelerationPreset::Off,
                TestAccelerationConfig {
                    time_scale: 1.0,
                    resource_decay_multiplier: 1.0,
                    resilience_hours_override: None,
                    critical_ms_override: 300_000,
                },
            ),
            (
                TestAccelerationPreset::Fast,
                TestAccelerationConfig {
                    time_scale: 20.0,
                    resource_decay_multiplier: 20.0,
                    resilience_hours_override: Some(0.05),
                    critical_ms_override: 20_000,
                },
            ),
            (
                TestAccelerationPreset::Turbo,
                TestAccelerationConfig {
                    time_scale: 120.0,
                    resource_decay_multiplier: 120.0,
                    resilience_hours_override: Some(0.0),
                    critical_ms_override: 5_000,
                },
            ),
            (
                TestAccelerationPreset::Hyper,
                TestAccelerationConfig {
                    time_scale: 100.0,
                    resource_decay_multiplier: 20.0,
                    resilience_hours_override: Some(0.0),
                    critical_ms_override: 10_000,
                },
            ),
            (
                TestAccelerationPreset::Ludicrous,
                TestAccelerationConfig {
                    time_scale: 10_000.0,
                    resource_decay_multiplier: 20.0,
                    resilience_hours_override: Some(0.0),
                    critical_ms_override: 10_000,
                },
            ),
        ];

        assert_eq!(TestAccelerationPreset::ALL.len(), expected.len());
        for (preset, config) in expected {
            assert_config(config_for_preset(preset), config);
        }
    }

    #[test]
    fn preset_from_time_scale_matches_typescript_branch_order() {
        let cases = [
            (None, TestAccelerationPreset::Off),
            (Some(1.0), TestAccelerationPreset::Off),
            (Some(19.999), TestAccelerationPreset::Off),
            (Some(20.0), TestAccelerationPreset::Fast),
            (Some(100.0), TestAccelerationPreset::Hyper),
            (Some(119.999), TestAccelerationPreset::Fast),
            (Some(120.0), TestAccelerationPreset::Turbo),
            (Some(9_999.999), TestAccelerationPreset::Turbo),
            (Some(10_000.0), TestAccelerationPreset::Ludicrous),
            (Some(f64::INFINITY), TestAccelerationPreset::Ludicrous),
            (Some(f64::NAN), TestAccelerationPreset::Off),
        ];

        for (time_scale, expected) in cases {
            assert_eq!(preset_from_time_scale(time_scale), expected);
        }
    }
}
