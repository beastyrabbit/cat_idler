//! Smithy production chain ported from `lib/game/smithy.ts`.

use crate::types::CatSpecialization;

/// Refined goods one smithy cycle consumes.
pub const SMITHY_REFINED_PER_CYCLE: f64 = 2.0;
/// Raw materials one smithy cycle consumes.
pub const SMITHY_MATERIALS_PER_CYCLE: f64 = 3.0;
/// Weapons one smithy cycle forges.
pub const SMITHY_WEAPONS_PER_CYCLE: f64 = 1.0;
/// Armor one smithy cycle forges.
pub const SMITHY_ARMOR_PER_CYCLE: f64 = 1.0;
/// Seconds of work one full smithy cycle takes.
pub const SMITHY_CYCLE_SEC: f64 = 900.0;
/// Architects work the forge at double speed.
pub const SMITH_FAST_SPEED: f64 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmithyOptions {
    pub has_worker: bool,
    pub worker_is_fast: bool,
    pub refined_available: f64,
    pub materials_available: f64,
}

impl SmithyOptions {
    #[must_use]
    pub const fn new(
        has_worker: bool,
        worker_is_fast: bool,
        refined_available: f64,
        materials_available: f64,
    ) -> Self {
        Self {
            has_worker,
            worker_is_fast,
            refined_available,
            materials_available,
        }
    }

    #[must_use]
    pub const fn from_worker(
        worker_specialization: Option<CatSpecialization>,
        refined_available: f64,
        materials_available: f64,
    ) -> Self {
        Self {
            has_worker: worker_specialization.is_some(),
            worker_is_fast: smith_worker_is_fast(worker_specialization),
            refined_available,
            materials_available,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmithyStep {
    /// Carry-over cycle time in seconds after this tick.
    pub next_progress: f64,
    /// Refined goods consumed this tick.
    pub refined_used: f64,
    /// Raw materials consumed this tick.
    pub materials_used: f64,
    /// Weapons produced this tick.
    pub weapons_produced: f64,
    /// Armor produced this tick.
    pub armor_produced: f64,
}

#[must_use]
pub const fn smith_worker_is_fast(specialization: Option<CatSpecialization>) -> bool {
    matches!(specialization, Some(CatSpecialization::Architect))
}

#[must_use]
pub fn advance_smithy(progress_sec: f64, elapsed_sec: f64, options: SmithyOptions) -> SmithyStep {
    if !options.has_worker || elapsed_sec <= 0.0 {
        return SmithyStep {
            next_progress: progress_sec,
            refined_used: 0.0,
            materials_used: 0.0,
            weapons_produced: 0.0,
            armor_produced: 0.0,
        };
    }

    let speed = if options.worker_is_fast {
        SMITH_FAST_SPEED
    } else {
        1.0
    };
    let mut progress = progress_sec + elapsed_sec * speed;

    let cycles_by_time = (progress / SMITHY_CYCLE_SEC).floor();
    let cycles_by_refined = (options.refined_available / SMITHY_REFINED_PER_CYCLE).floor();
    let cycles_by_materials = (options.materials_available / SMITHY_MATERIALS_PER_CYCLE).floor();
    let cycles = js_max(
        0.0,
        js_min(
            cycles_by_time,
            js_min(cycles_by_refined, cycles_by_materials),
        ),
    );

    progress -= cycles * SMITHY_CYCLE_SEC;
    progress = js_min(progress, SMITHY_CYCLE_SEC);

    SmithyStep {
        next_progress: progress,
        refined_used: cycles * SMITHY_REFINED_PER_CYCLE,
        materials_used: cycles * SMITHY_MATERIALS_PER_CYCLE,
        weapons_produced: cycles * SMITHY_WEAPONS_PER_CYCLE,
        armor_produced: cycles * SMITHY_ARMOR_PER_CYCLE,
    }
}

fn js_max(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else if left >= right {
        left
    } else {
        right
    }
}

fn js_min(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else if left <= right {
        left
    } else {
        right
    }
}

#[cfg(test)]
mod tests {
    use crate::types::CatSpecialization;

    use super::{
        SMITH_FAST_SPEED, SMITHY_ARMOR_PER_CYCLE, SMITHY_CYCLE_SEC, SMITHY_MATERIALS_PER_CYCLE,
        SMITHY_REFINED_PER_CYCLE, SMITHY_WEAPONS_PER_CYCLE, SmithyOptions, SmithyStep,
        advance_smithy, smith_worker_is_fast,
    };

    fn options(
        has_worker: bool,
        worker_is_fast: bool,
        refined_available: f64,
        materials_available: f64,
    ) -> SmithyOptions {
        SmithyOptions {
            has_worker,
            worker_is_fast,
            refined_available,
            materials_available,
        }
    }

    fn assert_f64_bits(actual: f64, expected: f64, label: &str) {
        assert_eq!(actual.to_bits(), expected.to_bits(), "{label}");
    }

    fn assert_step_bits(actual: SmithyStep, expected: SmithyStep, label: &str) {
        assert_f64_bits(
            actual.next_progress,
            expected.next_progress,
            &format!("{label} next_progress"),
        );
        assert_f64_bits(
            actual.refined_used,
            expected.refined_used,
            &format!("{label} refined_used"),
        );
        assert_f64_bits(
            actual.materials_used,
            expected.materials_used,
            &format!("{label} materials_used"),
        );
        assert_f64_bits(
            actual.weapons_produced,
            expected.weapons_produced,
            &format!("{label} weapons_produced"),
        );
        assert_f64_bits(
            actual.armor_produced,
            expected.armor_produced,
            &format!("{label} armor_produced"),
        );
    }

    #[test]
    fn constants_match_typescript_exports() {
        assert_f64_bits(SMITHY_REFINED_PER_CYCLE, 2.0, "refined per cycle");
        assert_f64_bits(SMITHY_MATERIALS_PER_CYCLE, 3.0, "materials per cycle");
        assert_f64_bits(SMITHY_WEAPONS_PER_CYCLE, 1.0, "weapons per cycle");
        assert_f64_bits(SMITHY_ARMOR_PER_CYCLE, 1.0, "armor per cycle");
        assert_f64_bits(SMITHY_CYCLE_SEC, 900.0, "smithy cycle seconds");
        assert_f64_bits(SMITH_FAST_SPEED, 2.0, "fast smith speed");
    }

    #[test]
    fn architect_specialization_is_fast_smith() {
        assert!(smith_worker_is_fast(Some(CatSpecialization::Architect)));
        assert!(!smith_worker_is_fast(Some(CatSpecialization::Hunter)));
        assert!(!smith_worker_is_fast(Some(CatSpecialization::Ritualist)));
        assert!(!smith_worker_is_fast(Some(CatSpecialization::Warrior)));
        assert!(!smith_worker_is_fast(None));

        assert_eq!(
            SmithyOptions::from_worker(Some(CatSpecialization::Architect), 10.0, 15.0),
            options(true, true, 10.0, 15.0)
        );
        assert_eq!(
            SmithyOptions::from_worker(None, 10.0, 15.0),
            options(false, false, 10.0, 15.0)
        );
    }

    #[test]
    fn smithy_does_not_progress_without_worker_or_positive_elapsed_time() {
        assert_step_bits(
            advance_smithy(123.0, 900.0, options(false, false, 50.0, 50.0)),
            SmithyStep {
                next_progress: 123.0,
                refined_used: 0.0,
                materials_used: 0.0,
                weapons_produced: 0.0,
                armor_produced: 0.0,
            },
            "without worker",
        );

        assert_step_bits(
            advance_smithy(123.0, 0.0, options(true, true, 50.0, 50.0)),
            SmithyStep {
                next_progress: 123.0,
                refined_used: 0.0,
                materials_used: 0.0,
                weapons_produced: 0.0,
                armor_produced: 0.0,
            },
            "zero elapsed",
        );

        assert_step_bits(
            advance_smithy(123.0, -1.0, options(true, true, 50.0, 50.0)),
            SmithyStep {
                next_progress: 123.0,
                refined_used: 0.0,
                materials_used: 0.0,
                weapons_produced: 0.0,
                armor_produced: 0.0,
            },
            "negative elapsed",
        );
    }

    #[test]
    fn smithy_accumulates_short_ticks_and_converts_complete_cycles() {
        assert_step_bits(
            advance_smithy(890.0, 30.0, options(true, false, 4.0, 6.0)),
            SmithyStep {
                next_progress: 20.0,
                refined_used: 2.0,
                materials_used: 3.0,
                weapons_produced: 1.0,
                armor_produced: 1.0,
            },
            "one completed cycle",
        );

        assert_step_bits(
            advance_smithy(0.0, 2_700.0, options(true, false, 6.0, 9.0)),
            SmithyStep {
                next_progress: 0.0,
                refined_used: 6.0,
                materials_used: 9.0,
                weapons_produced: 3.0,
                armor_produced: 3.0,
            },
            "three completed cycles",
        );
    }

    #[test]
    fn fast_worker_advances_smithy_at_double_speed() {
        assert_step_bits(
            advance_smithy(0.0, 450.0, options(true, true, 2.0, 3.0)),
            SmithyStep {
                next_progress: 0.0,
                refined_used: 2.0,
                materials_used: 3.0,
                weapons_produced: 1.0,
                armor_produced: 1.0,
            },
            "fast worker cycle",
        );

        assert_step_bits(
            advance_smithy(100.0, 125.0, options(true, true, 0.0, 0.0)),
            SmithyStep {
                next_progress: 350.0,
                refined_used: 0.0,
                materials_used: 0.0,
                weapons_produced: 0.0,
                armor_produced: 0.0,
            },
            "fast worker partial progress",
        );
    }

    #[test]
    fn smithy_cycles_are_limited_by_available_inputs() {
        assert_step_bits(
            advance_smithy(890.0, 30.0, options(true, false, 0.0, 6.0)),
            SmithyStep {
                next_progress: 900.0,
                refined_used: 0.0,
                materials_used: 0.0,
                weapons_produced: 0.0,
                armor_produced: 0.0,
            },
            "stalls at one full cycle without refined goods",
        );

        assert_step_bits(
            advance_smithy(890.0, 30.0, options(true, false, 4.0, 0.0)),
            SmithyStep {
                next_progress: 900.0,
                refined_used: 0.0,
                materials_used: 0.0,
                weapons_produced: 0.0,
                armor_produced: 0.0,
            },
            "stalls at one full cycle without materials",
        );

        assert_step_bits(
            advance_smithy(0.0, 2_700.0, options(true, false, 2.0, 9.0)),
            SmithyStep {
                next_progress: 900.0,
                refined_used: 2.0,
                materials_used: 3.0,
                weapons_produced: 1.0,
                armor_produced: 1.0,
            },
            "refined goods cap banked progress",
        );

        assert_step_bits(
            advance_smithy(0.0, 2_700.0, options(true, false, 6.0, 5.9)),
            SmithyStep {
                next_progress: 900.0,
                refined_used: 2.0,
                materials_used: 3.0,
                weapons_produced: 1.0,
                armor_produced: 1.0,
            },
            "fractional materials are floored to complete cycles",
        );
    }

    #[test]
    fn smithy_preserves_typescript_edge_cases_for_negative_progress_and_inputs() {
        assert_step_bits(
            advance_smithy(-1_000.0, 100.0, options(true, false, 50.0, 50.0)),
            SmithyStep {
                next_progress: -900.0,
                refined_used: 0.0,
                materials_used: 0.0,
                weapons_produced: 0.0,
                armor_produced: 0.0,
            },
            "negative progress is not clamped upward",
        );

        assert_step_bits(
            advance_smithy(1_000.0, 0.1, options(true, false, -1.0, 50.0)),
            SmithyStep {
                next_progress: 900.0,
                refined_used: 0.0,
                materials_used: 0.0,
                weapons_produced: 0.0,
                armor_produced: 0.0,
            },
            "negative refined goods cannot create negative cycles",
        );

        assert_step_bits(
            advance_smithy(1_000.0, 0.1, options(true, false, 50.0, -1.0)),
            SmithyStep {
                next_progress: 900.0,
                refined_used: 0.0,
                materials_used: 0.0,
                weapons_produced: 0.0,
                armor_produced: 0.0,
            },
            "negative materials cannot create negative cycles",
        );
    }

    #[test]
    fn nan_inputs_match_math_max_min_propagation() {
        let smithy_step = advance_smithy(f64::NAN, 1.0, options(true, false, 2.0, 3.0));
        assert!(smithy_step.next_progress.is_nan());
        assert!(smithy_step.refined_used.is_nan());
        assert!(smithy_step.materials_used.is_nan());
        assert!(smithy_step.weapons_produced.is_nan());
        assert!(smithy_step.armor_produced.is_nan());
    }
}
