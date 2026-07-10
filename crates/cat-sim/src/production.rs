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

// --- P12.4b raw-material refinement chains (P16 blueprint workshops) ---
//
// The wood-cutter and stone-prep shops share the refinement-workshop cadence
// (5 raw materials → 1 refined unit / 600s), so both reuse [`advance_workshop`]
// at the tick site — crediting planks and blocks respectively instead of the
// generic `refined` good. The woodworking shop is a two-input crafter
// (planks + blocks → tools) and has its own [`advance_woodworking`] cycle,
// mirroring the smithy's twin-input shape.

/// Raw materials one wood-cutter cycle consumes (aliased to the workshop rate).
pub const WOODCUTTER_MATERIALS_PER_CYCLE: f64 = WORKSHOP_MATERIALS_PER_CYCLE;
/// Planks one wood-cutter cycle produces.
pub const WOODCUTTER_PLANKS_PER_CYCLE: f64 = WORKSHOP_REFINED_PER_CYCLE;
/// Raw materials one stone-prep cycle consumes.
pub const STONEPREP_MATERIALS_PER_CYCLE: f64 = WORKSHOP_MATERIALS_PER_CYCLE;
/// Blocks one stone-prep cycle produces.
pub const STONEPREP_BLOCKS_PER_CYCLE: f64 = WORKSHOP_REFINED_PER_CYCLE;

/// Planks one woodworking cycle consumes.
pub const WOODWORKING_PLANKS_PER_CYCLE: f64 = 2.0;
/// Blocks one woodworking cycle consumes.
pub const WOODWORKING_BLOCKS_PER_CYCLE: f64 = 2.0;
/// Tools one woodworking cycle forges.
pub const WOODWORKING_TOOLS_PER_CYCLE: f64 = 1.0;
/// Seconds of work one full woodworking cycle takes.
pub const WOODWORKING_CYCLE_SEC: f64 = 600.0;

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

/// Inputs for one woodworking tick: whether a worker is present/fast and how many
/// planks and blocks are on hand.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WoodworkingOptions {
    pub has_worker: bool,
    pub worker_is_architect: bool,
    pub planks_available: f64,
    pub blocks_available: f64,
}

impl WoodworkingOptions {
    #[must_use]
    pub const fn new(
        has_worker: bool,
        worker_is_architect: bool,
        planks_available: f64,
        blocks_available: f64,
    ) -> Self {
        Self {
            has_worker,
            worker_is_architect,
            planks_available,
            blocks_available,
        }
    }

    #[must_use]
    pub const fn from_worker(
        worker_specialization: Option<CatSpecialization>,
        planks_available: f64,
        blocks_available: f64,
    ) -> Self {
        Self {
            has_worker: worker_specialization.is_some(),
            worker_is_architect: worker_is_architect(worker_specialization),
            planks_available,
            blocks_available,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WoodworkingStep {
    /// Carry-over cycle time in seconds after this tick.
    pub next_progress: f64,
    /// Planks consumed this tick.
    pub planks_used: f64,
    /// Blocks consumed this tick.
    pub blocks_used: f64,
    /// Tools produced this tick.
    pub tools_produced: f64,
}

/// Advance a staffed woodworking shop: planks + blocks → tools, floored by
/// elapsed time and the scarcer of the two inputs. Mirrors the smithy's
/// twin-input cadence; architects work the bench at double speed.
#[must_use]
pub fn advance_woodworking(
    progress_sec: f64,
    elapsed_sec: f64,
    options: WoodworkingOptions,
) -> WoodworkingStep {
    if !options.has_worker || elapsed_sec <= 0.0 {
        return WoodworkingStep {
            next_progress: progress_sec,
            planks_used: 0.0,
            blocks_used: 0.0,
            tools_produced: 0.0,
        };
    }

    let speed = if options.worker_is_architect {
        ARCHITECT_SPEED
    } else {
        1.0
    };
    let mut progress = progress_sec + elapsed_sec * speed;

    let cycles_by_time = (progress / WOODWORKING_CYCLE_SEC).floor();
    let cycles_by_planks = (options.planks_available / WOODWORKING_PLANKS_PER_CYCLE).floor();
    let cycles_by_blocks = (options.blocks_available / WOODWORKING_BLOCKS_PER_CYCLE).floor();
    let cycles = js_max(
        0.0,
        js_min(cycles_by_time, js_min(cycles_by_planks, cycles_by_blocks)),
    );

    progress -= cycles * WOODWORKING_CYCLE_SEC;
    progress = js_min(progress, WOODWORKING_CYCLE_SEC);

    WoodworkingStep {
        next_progress: progress,
        planks_used: cycles * WOODWORKING_PLANKS_PER_CYCLE,
        blocks_used: cycles * WOODWORKING_BLOCKS_PER_CYCLE,
        tools_produced: cycles * WOODWORKING_TOOLS_PER_CYCLE,
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

    fn wood_options(
        has_worker: bool,
        worker_is_architect: bool,
        planks_available: f64,
        blocks_available: f64,
    ) -> super::WoodworkingOptions {
        super::WoodworkingOptions {
            has_worker,
            worker_is_architect,
            planks_available,
            blocks_available,
        }
    }

    fn assert_wood_step_bits(
        actual: super::WoodworkingStep,
        expected: super::WoodworkingStep,
        label: &str,
    ) {
        assert_f64_bits(
            actual.next_progress,
            expected.next_progress,
            &format!("{label} next_progress"),
        );
        assert_f64_bits(
            actual.planks_used,
            expected.planks_used,
            &format!("{label} planks_used"),
        );
        assert_f64_bits(
            actual.blocks_used,
            expected.blocks_used,
            &format!("{label} blocks_used"),
        );
        assert_f64_bits(
            actual.tools_produced,
            expected.tools_produced,
            &format!("{label} tools_produced"),
        );
    }

    #[test]
    fn woodworking_chain_constants_match_the_two_plus_two_recipe() {
        assert_f64_bits(super::WOODWORKING_PLANKS_PER_CYCLE, 2.0, "planks per cycle");
        assert_f64_bits(super::WOODWORKING_BLOCKS_PER_CYCLE, 2.0, "blocks per cycle");
        assert_f64_bits(super::WOODWORKING_TOOLS_PER_CYCLE, 1.0, "tools per cycle");
        assert_f64_bits(super::WOODWORKING_CYCLE_SEC, 600.0, "cycle seconds");
        // The wood-cutter / stone-prep aliases inherit the refinement-workshop rate.
        assert_f64_bits(super::WOODCUTTER_MATERIALS_PER_CYCLE, 5.0, "woodcutter in");
        assert_f64_bits(super::WOODCUTTER_PLANKS_PER_CYCLE, 1.0, "woodcutter out");
        assert_f64_bits(super::STONEPREP_MATERIALS_PER_CYCLE, 5.0, "stoneprep in");
        assert_f64_bits(super::STONEPREP_BLOCKS_PER_CYCLE, 1.0, "stoneprep out");
    }

    #[test]
    fn woodworking_only_crafts_with_a_worker_and_positive_time() {
        assert_wood_step_bits(
            super::advance_woodworking(123.0, 600.0, wood_options(false, false, 50.0, 50.0)),
            super::WoodworkingStep {
                next_progress: 123.0,
                planks_used: 0.0,
                blocks_used: 0.0,
                tools_produced: 0.0,
            },
            "without worker",
        );
        assert_wood_step_bits(
            super::advance_woodworking(123.0, 0.0, wood_options(true, false, 50.0, 50.0)),
            super::WoodworkingStep {
                next_progress: 123.0,
                planks_used: 0.0,
                blocks_used: 0.0,
                tools_produced: 0.0,
            },
            "zero elapsed",
        );
    }

    #[test]
    fn woodworking_converts_complete_cycles_and_floors_by_scarcer_input() {
        assert_wood_step_bits(
            super::advance_woodworking(590.0, 30.0, wood_options(true, false, 4.0, 4.0)),
            super::WoodworkingStep {
                next_progress: 20.0,
                planks_used: 2.0,
                blocks_used: 2.0,
                tools_produced: 1.0,
            },
            "one completed cycle",
        );
        // Blocks are the bottleneck: only one cycle's worth on hand.
        assert_wood_step_bits(
            super::advance_woodworking(0.0, 1_800.0, wood_options(true, false, 20.0, 2.0)),
            super::WoodworkingStep {
                next_progress: 600.0,
                planks_used: 2.0,
                blocks_used: 2.0,
                tools_produced: 1.0,
            },
            "blocks cap banked progress",
        );
        // Architects run the bench at double speed.
        assert_wood_step_bits(
            super::advance_woodworking(0.0, 300.0, wood_options(true, true, 2.0, 2.0)),
            super::WoodworkingStep {
                next_progress: 0.0,
                planks_used: 2.0,
                blocks_used: 2.0,
                tools_produced: 1.0,
            },
            "architect cycle",
        );
    }

    #[test]
    fn woodworking_stalls_without_inputs() {
        assert_wood_step_bits(
            super::advance_woodworking(590.0, 30.0, wood_options(true, false, 0.0, 5.0)),
            super::WoodworkingStep {
                next_progress: 600.0,
                planks_used: 0.0,
                blocks_used: 0.0,
                tools_produced: 0.0,
            },
            "no planks",
        );
        assert_wood_step_bits(
            super::advance_woodworking(590.0, 30.0, wood_options(true, false, 5.0, 0.0)),
            super::WoodworkingStep {
                next_progress: 600.0,
                planks_used: 0.0,
                blocks_used: 0.0,
                tools_produced: 0.0,
            },
            "no blocks",
        );
    }
}
