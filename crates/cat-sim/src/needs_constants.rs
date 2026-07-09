//! Needs, life-stage, and leader-quality constants ported from `types/game.ts`.

use crate::types::LifeStage;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NeedsDecayRates {
    pub hunger: f64,
    pub thirst: f64,
    pub rest: f64,
}

pub const NEEDS_DECAY_RATES: NeedsDecayRates = NeedsDecayRates {
    hunger: 5.0,
    thirst: 3.0,
    rest: 2.0,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NeedsRestoreAmounts {
    pub eating: f64,
    pub drinking: f64,
    pub sleeping: f64,
    pub sleeping_with_beds: f64,
}

pub const NEEDS_RESTORE_AMOUNTS: NeedsRestoreAmounts = NeedsRestoreAmounts {
    eating: 30.0,
    drinking: 40.0,
    sleeping: 20.0,
    sleeping_with_beds: 30.0,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LifeStageHours {
    pub min: f64,
    pub max: f64,
}

pub const LIFE_STAGE_HOURS: [(LifeStage, LifeStageHours); 4] = [
    (LifeStage::Kitten, LifeStageHours { min: 0.0, max: 6.0 }),
    (
        LifeStage::Young,
        LifeStageHours {
            min: 6.0,
            max: 24.0,
        },
    ),
    (
        LifeStage::Adult,
        LifeStageHours {
            min: 24.0,
            max: 48.0,
        },
    ),
    (
        LifeStage::Elder,
        LifeStageHours {
            min: 48.0,
            max: f64::INFINITY,
        },
    ),
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeaderQualityBand {
    pub min: u32,
    pub max: u32,
    pub time: u32,
    pub wrong_chance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeaderQuality {
    pub bad: LeaderQualityBand,
    pub okay: LeaderQualityBand,
    pub good: LeaderQualityBand,
    pub great: LeaderQualityBand,
}

pub const LEADER_QUALITY: LeaderQuality = LeaderQuality {
    bad: LeaderQualityBand {
        min: 0,
        max: 10,
        time: 30,
        wrong_chance: 0.4,
    },
    okay: LeaderQualityBand {
        min: 11,
        max: 20,
        time: 20,
        wrong_chance: 0.2,
    },
    good: LeaderQualityBand {
        min: 21,
        max: 30,
        time: 10,
        wrong_chance: 0.05,
    },
    great: LeaderQualityBand {
        min: 31,
        max: 100,
        time: 5,
        wrong_chance: 0.0,
    },
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::LifeStage;

    fn assert_f64_literal(actual: f64, expected: f64) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }

    #[test]
    fn needs_decay_rates_match_types_game_ts() {
        assert_f64_literal(NEEDS_DECAY_RATES.hunger, 5.0);
        assert_f64_literal(NEEDS_DECAY_RATES.thirst, 3.0);
        assert_f64_literal(NEEDS_DECAY_RATES.rest, 2.0);
    }

    #[test]
    fn needs_restore_amounts_match_types_game_ts() {
        assert_f64_literal(NEEDS_RESTORE_AMOUNTS.eating, 30.0);
        assert_f64_literal(NEEDS_RESTORE_AMOUNTS.drinking, 40.0);
        assert_f64_literal(NEEDS_RESTORE_AMOUNTS.sleeping, 20.0);
        assert_f64_literal(NEEDS_RESTORE_AMOUNTS.sleeping_with_beds, 30.0);
    }

    #[test]
    fn life_stage_hours_match_types_game_ts() {
        let expected = [
            (LifeStage::Kitten, LifeStageHours { min: 0.0, max: 6.0 }),
            (
                LifeStage::Young,
                LifeStageHours {
                    min: 6.0,
                    max: 24.0,
                },
            ),
            (
                LifeStage::Adult,
                LifeStageHours {
                    min: 24.0,
                    max: 48.0,
                },
            ),
            (
                LifeStage::Elder,
                LifeStageHours {
                    min: 48.0,
                    max: f64::INFINITY,
                },
            ),
        ];

        assert_eq!(LIFE_STAGE_HOURS.len(), expected.len());
        for ((actual_stage, actual_hours), (expected_stage, expected_hours)) in
            LIFE_STAGE_HOURS.iter().zip(expected)
        {
            assert_eq!(*actual_stage, expected_stage);
            assert_f64_literal(actual_hours.min, expected_hours.min);
            assert_f64_literal(actual_hours.max, expected_hours.max);
        }
    }

    #[test]
    fn leader_quality_matches_types_game_ts() {
        let expected = [
            (
                LEADER_QUALITY.bad,
                LeaderQualityBand {
                    min: 0,
                    max: 10,
                    time: 30,
                    wrong_chance: 0.4,
                },
            ),
            (
                LEADER_QUALITY.okay,
                LeaderQualityBand {
                    min: 11,
                    max: 20,
                    time: 20,
                    wrong_chance: 0.2,
                },
            ),
            (
                LEADER_QUALITY.good,
                LeaderQualityBand {
                    min: 21,
                    max: 30,
                    time: 10,
                    wrong_chance: 0.05,
                },
            ),
            (
                LEADER_QUALITY.great,
                LeaderQualityBand {
                    min: 31,
                    max: 100,
                    time: 5,
                    wrong_chance: 0.0,
                },
            ),
        ];

        for (actual, expected) in expected {
            assert_eq!(actual.min, expected.min);
            assert_eq!(actual.max, expected.max);
            assert_eq!(actual.time, expected.time);
            assert_f64_literal(actual.wrong_chance, expected.wrong_chance);
        }
    }
}
