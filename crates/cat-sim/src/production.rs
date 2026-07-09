//! Production chains ported from `lib/game/production.ts`.

use crate::types::{BuildingType, CatSpecialization};

pub const WORKSHOP_MATERIALS_PER_CYCLE: f64 = 5.0;
pub const WORKSHOP_REFINED_PER_CYCLE: f64 = 1.0;
pub const WORKSHOP_CYCLE_SEC: f64 = 600.0;
/// Architects run workshops at double speed.
pub const ARCHITECT_SPEED: f64 = 2.0;

pub const FIELD_FOOD_PER_HOUR: f64 = 2.0;

pub const WORKSHOP_UNLOCK_LEVEL: u32 = 2;
pub const FIELD_UNLOCK_LEVEL: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkshopOptions {
    pub has_worker: bool,
    pub worker_is_architect: bool,
    pub materials_available: f64,
}

impl WorkshopOptions {
    #[must_use]
    pub const fn new(
        has_worker: bool,
        worker_is_architect: bool,
        materials_available: f64,
    ) -> Self {
        Self {
            has_worker,
            worker_is_architect,
            materials_available,
        }
    }

    #[must_use]
    pub const fn from_worker(
        worker_specialization: Option<CatSpecialization>,
        materials_available: f64,
    ) -> Self {
        Self {
            has_worker: worker_specialization.is_some(),
            worker_is_architect: worker_is_architect(worker_specialization),
            materials_available,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkshopStep {
    /// Carry-over cycle time in seconds after this tick.
    pub next_progress: f64,
    /// Materials consumed this tick.
    pub materials_used: f64,
    /// Refined goods produced this tick.
    pub refined_produced: f64,
}

#[must_use]
pub const fn workshop_unlocked(village_level: u32) -> bool {
    village_level >= WORKSHOP_UNLOCK_LEVEL
}

#[must_use]
pub const fn field_unlocked(village_level: u32) -> bool {
    village_level >= FIELD_UNLOCK_LEVEL
}

#[must_use]
pub const fn building_unlocked(building_type: BuildingType, village_level: u32) -> bool {
    match building_type {
        BuildingType::Workshop => workshop_unlocked(village_level),
        BuildingType::Field => field_unlocked(village_level),
        _ => true,
    }
}

#[must_use]
pub const fn worker_is_architect(worker_specialization: Option<CatSpecialization>) -> bool {
    matches!(worker_specialization, Some(CatSpecialization::Architect))
}

#[must_use]
pub fn advance_workshop(
    progress_sec: f64,
    elapsed_sec: f64,
    options: WorkshopOptions,
) -> WorkshopStep {
    if !options.has_worker || elapsed_sec <= 0.0 {
        return WorkshopStep {
            next_progress: progress_sec,
            materials_used: 0.0,
            refined_produced: 0.0,
        };
    }

    let speed = if options.worker_is_architect {
        ARCHITECT_SPEED
    } else {
        1.0
    };
    let mut progress = progress_sec + elapsed_sec * speed;

    let cycles_by_time = (progress / WORKSHOP_CYCLE_SEC).floor();
    let cycles_by_materials = (options.materials_available / WORKSHOP_MATERIALS_PER_CYCLE).floor();
    let cycles = js_max(0.0, js_min(cycles_by_time, cycles_by_materials));

    progress -= cycles * WORKSHOP_CYCLE_SEC;
    progress = js_min(progress, WORKSHOP_CYCLE_SEC);

    WorkshopStep {
        next_progress: progress,
        materials_used: cycles * WORKSHOP_MATERIALS_PER_CYCLE,
        refined_produced: cycles * WORKSHOP_REFINED_PER_CYCLE,
    }
}

#[must_use]
pub fn field_yield(elapsed_sec: f64) -> f64 {
    js_max(0.0, elapsed_sec / 3600.0 * FIELD_FOOD_PER_HOUR)
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
    use super::{
        FIELD_FOOD_PER_HOUR, FIELD_UNLOCK_LEVEL, WORKSHOP_CYCLE_SEC, WORKSHOP_MATERIALS_PER_CYCLE,
        WORKSHOP_REFINED_PER_CYCLE, WORKSHOP_UNLOCK_LEVEL, WorkshopOptions, WorkshopStep,
        advance_workshop, building_unlocked, field_unlocked, field_yield, worker_is_architect,
        workshop_unlocked,
    };
    use crate::types::{BuildingType, CatSpecialization};

    fn options(
        has_worker: bool,
        worker_is_architect: bool,
        materials_available: f64,
    ) -> WorkshopOptions {
        WorkshopOptions {
            has_worker,
            worker_is_architect,
            materials_available,
        }
    }

    fn assert_f64_bits(actual: f64, expected: f64, label: &str) {
        assert_eq!(actual.to_bits(), expected.to_bits(), "{label}");
    }

    fn assert_step_bits(actual: WorkshopStep, expected: WorkshopStep, label: &str) {
        assert_f64_bits(
            actual.next_progress,
            expected.next_progress,
            &format!("{label} next_progress"),
        );
        assert_f64_bits(
            actual.materials_used,
            expected.materials_used,
            &format!("{label} materials_used"),
        );
        assert_f64_bits(
            actual.refined_produced,
            expected.refined_produced,
            &format!("{label} refined_produced"),
        );
    }

    #[test]
    fn constants_match_typescript_exports() {
        assert_f64_bits(WORKSHOP_MATERIALS_PER_CYCLE, 5.0, "materials per cycle");
        assert_f64_bits(WORKSHOP_REFINED_PER_CYCLE, 1.0, "refined per cycle");
        assert_f64_bits(WORKSHOP_CYCLE_SEC, 600.0, "workshop cycle seconds");
        assert_f64_bits(FIELD_FOOD_PER_HOUR, 2.0, "field food per hour");
        assert_eq!(WORKSHOP_UNLOCK_LEVEL, 2);
        assert_eq!(FIELD_UNLOCK_LEVEL, 4);
    }

    #[test]
    fn unlocks_use_inclusive_village_level_thresholds() {
        assert!(!workshop_unlocked(0));
        assert!(!workshop_unlocked(1));
        assert!(workshop_unlocked(2));
        assert!(workshop_unlocked(3));

        assert!(!field_unlocked(3));
        assert!(field_unlocked(4));
        assert!(field_unlocked(5));
    }

    #[test]
    fn typed_unlock_helper_gates_only_production_buildings() {
        assert!(!building_unlocked(BuildingType::Workshop, 1));
        assert!(building_unlocked(BuildingType::Workshop, 2));
        assert!(!building_unlocked(BuildingType::Field, 3));
        assert!(building_unlocked(BuildingType::Field, 4));
        assert!(building_unlocked(BuildingType::Den, 0));
    }

    #[test]
    fn typed_worker_helper_identifies_architects() {
        assert!(!worker_is_architect(None));
        assert!(!worker_is_architect(Some(CatSpecialization::Hunter)));
        assert!(worker_is_architect(Some(CatSpecialization::Architect)));

        assert_eq!(
            WorkshopOptions::from_worker(Some(CatSpecialization::Architect), 25.0),
            options(true, true, 25.0)
        );
        assert_eq!(
            WorkshopOptions::from_worker(None, 25.0),
            options(false, false, 25.0)
        );
    }

    #[test]
    fn workshop_does_not_progress_without_worker_or_positive_elapsed_time() {
        assert_step_bits(
            advance_workshop(123.0, 600.0, options(false, false, 50.0)),
            WorkshopStep {
                next_progress: 123.0,
                materials_used: 0.0,
                refined_produced: 0.0,
            },
            "without worker",
        );

        assert_step_bits(
            advance_workshop(123.0, 0.0, options(true, true, 50.0)),
            WorkshopStep {
                next_progress: 123.0,
                materials_used: 0.0,
                refined_produced: 0.0,
            },
            "zero elapsed",
        );

        assert_step_bits(
            advance_workshop(123.0, -1.0, options(true, true, 50.0)),
            WorkshopStep {
                next_progress: 123.0,
                materials_used: 0.0,
                refined_produced: 0.0,
            },
            "negative elapsed",
        );
    }

    #[test]
    fn workshop_accumulates_short_ticks_and_converts_complete_cycles() {
        assert_step_bits(
            advance_workshop(590.0, 30.0, options(true, false, 10.0)),
            WorkshopStep {
                next_progress: 20.0,
                materials_used: 5.0,
                refined_produced: 1.0,
            },
            "one completed cycle",
        );

        assert_step_bits(
            advance_workshop(0.0, 1_800.0, options(true, false, 15.0)),
            WorkshopStep {
                next_progress: 0.0,
                materials_used: 15.0,
                refined_produced: 3.0,
            },
            "three completed cycles",
        );
    }

    #[test]
    fn architect_worker_advances_workshop_at_double_speed() {
        assert_step_bits(
            advance_workshop(0.0, 300.0, options(true, true, 5.0)),
            WorkshopStep {
                next_progress: 0.0,
                materials_used: 5.0,
                refined_produced: 1.0,
            },
            "architect cycle",
        );

        assert_step_bits(
            advance_workshop(100.0, 125.0, options(true, true, 0.0)),
            WorkshopStep {
                next_progress: 350.0,
                materials_used: 0.0,
                refined_produced: 0.0,
            },
            "architect partial progress",
        );
    }

    #[test]
    fn workshop_cycles_are_limited_by_available_materials() {
        assert_step_bits(
            advance_workshop(590.0, 30.0, options(true, false, 0.0)),
            WorkshopStep {
                next_progress: 600.0,
                materials_used: 0.0,
                refined_produced: 0.0,
            },
            "stalls at one full cycle without materials",
        );

        assert_step_bits(
            advance_workshop(0.0, 1_800.0, options(true, false, 5.0)),
            WorkshopStep {
                next_progress: 600.0,
                materials_used: 5.0,
                refined_produced: 1.0,
            },
            "insufficient materials cap banked progress",
        );

        assert_step_bits(
            advance_workshop(0.0, 1_800.0, options(true, false, 14.9)),
            WorkshopStep {
                next_progress: 600.0,
                materials_used: 10.0,
                refined_produced: 2.0,
            },
            "fractional materials are floored to complete cycles",
        );
    }

    #[test]
    fn workshop_preserves_typescript_edge_cases_for_negative_progress_and_materials() {
        assert_step_bits(
            advance_workshop(-700.0, 100.0, options(true, false, 50.0)),
            WorkshopStep {
                next_progress: -600.0,
                materials_used: 0.0,
                refined_produced: 0.0,
            },
            "negative progress is not clamped upward",
        );

        assert_step_bits(
            advance_workshop(700.0, 0.1, options(true, false, -1.0)),
            WorkshopStep {
                next_progress: 600.0,
                materials_used: 0.0,
                refined_produced: 0.0,
            },
            "negative materials cannot create negative cycles",
        );
    }

    #[test]
    fn field_yield_is_passive_food_per_elapsed_window() {
        assert_f64_bits(field_yield(0.0), 0.0, "zero elapsed");
        assert_f64_bits(field_yield(-60.0), 0.0, "negative elapsed");
        assert_f64_bits(field_yield(1_800.0), 1.0, "half hour");
        assert_f64_bits(field_yield(3_600.0), 2.0, "one hour");
        assert_f64_bits(field_yield(5_400.0), 3.0, "one and a half hours");
    }

    #[test]
    fn nan_inputs_match_math_max_min_propagation() {
        let workshop_step = advance_workshop(f64::NAN, 1.0, options(true, false, 5.0));
        assert!(workshop_step.next_progress.is_nan());
        assert!(workshop_step.materials_used.is_nan());
        assert!(workshop_step.refined_produced.is_nan());

        assert!(field_yield(f64::NAN).is_nan());
    }
}
