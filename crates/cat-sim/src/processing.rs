//! Staffed crop and timber processing chains for farms/forestry.
//!
//! These recipes are intentionally separate from the legacy P12.4 wood-cutter chain:
//! `materials -> planks` remains save-compatible, while the new sawmill exclusively
//! performs `logs -> lumber`. No recipe copies an old resource into a new one.

use crate::production::ARCHITECT_SPEED;

pub const MILL_CYCLE_SEC: f64 = 600.0;
pub const MILL_GRAIN_PER_CYCLE: f64 = 4.0;
pub const MILL_FLOUR_FROM_GRAIN: f64 = 2.0;
pub const MILL_FLOUR_PER_FOOD_CYCLE: f64 = 2.0;
pub const MILL_FOOD_PER_CYCLE: f64 = 4.0;

pub const SAWMILL_CYCLE_SEC: f64 = 600.0;
pub const SAWMILL_LOGS_PER_CYCLE: f64 = 5.0;
pub const SAWMILL_LUMBER_PER_CYCLE: f64 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MillOptions {
    pub has_worker: bool,
    pub worker_is_architect: bool,
    pub grain_available: f64,
    pub flour_available: f64,
    pub flour_headroom: f64,
    pub food_headroom: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MillStep {
    pub next_progress: f64,
    pub grain_used: f64,
    pub flour_produced: f64,
    pub flour_used: f64,
    pub food_produced: f64,
}

/// Advance a mill's single worker timer.
///
/// Each complete cycle first bakes held flour into food. When that recipe cannot run,
/// it grinds grain into flour. This stable priority lets a long accelerated tick grind
/// and then bake without inventing inputs, while capacity can stall either output.
#[must_use]
pub fn advance_mill(progress_sec: f64, elapsed_sec: f64, options: MillOptions) -> MillStep {
    let old_progress = non_negative(progress_sec).min(MILL_CYCLE_SEC);
    if !options.has_worker || elapsed_sec <= 0.0 {
        return MillStep {
            next_progress: old_progress,
            grain_used: 0.0,
            flour_produced: 0.0,
            flour_used: 0.0,
            food_produced: 0.0,
        };
    }

    let speed = if options.worker_is_architect {
        ARCHITECT_SPEED
    } else {
        1.0
    };
    let mut progress = old_progress + non_negative(elapsed_sec) * speed;
    let mut grain = non_negative(options.grain_available);
    let mut flour = non_negative(options.flour_available);
    let mut flour_headroom = non_negative(options.flour_headroom);
    let mut food_headroom = non_negative(options.food_headroom);
    let mut step = MillStep {
        next_progress: 0.0,
        grain_used: 0.0,
        flour_produced: 0.0,
        flour_used: 0.0,
        food_produced: 0.0,
    };

    while progress >= MILL_CYCLE_SEC {
        if flour >= MILL_FLOUR_PER_FOOD_CYCLE && food_headroom >= MILL_FOOD_PER_CYCLE {
            flour -= MILL_FLOUR_PER_FOOD_CYCLE;
            // Consuming flour releases capacity that a later grind in this same
            // accelerated tick may use.
            flour_headroom += MILL_FLOUR_PER_FOOD_CYCLE;
            food_headroom -= MILL_FOOD_PER_CYCLE;
            step.flour_used += MILL_FLOUR_PER_FOOD_CYCLE;
            step.food_produced += MILL_FOOD_PER_CYCLE;
        } else if grain >= MILL_GRAIN_PER_CYCLE && flour_headroom >= MILL_FLOUR_FROM_GRAIN {
            grain -= MILL_GRAIN_PER_CYCLE;
            flour += MILL_FLOUR_FROM_GRAIN;
            flour_headroom -= MILL_FLOUR_FROM_GRAIN;
            step.grain_used += MILL_GRAIN_PER_CYCLE;
            step.flour_produced += MILL_FLOUR_FROM_GRAIN;
        } else {
            // A fully-worked blocked cycle remains banked; production resumes as soon
            // as input or output headroom becomes available.
            progress = MILL_CYCLE_SEC;
            break;
        }
        progress -= MILL_CYCLE_SEC;
    }
    step.next_progress = progress.min(MILL_CYCLE_SEC);
    step
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SawmillOptions {
    pub has_worker: bool,
    pub worker_is_architect: bool,
    pub logs_available: f64,
    pub lumber_headroom: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SawmillStep {
    pub next_progress: f64,
    pub logs_used: f64,
    pub lumber_produced: f64,
}

/// Advance the new timber chain (`logs -> lumber`) without touching legacy
/// `materials` or `planks`.
#[must_use]
pub fn advance_sawmill(
    progress_sec: f64,
    elapsed_sec: f64,
    options: SawmillOptions,
) -> SawmillStep {
    let old_progress = non_negative(progress_sec).min(SAWMILL_CYCLE_SEC);
    if !options.has_worker || elapsed_sec <= 0.0 {
        return SawmillStep {
            next_progress: old_progress,
            logs_used: 0.0,
            lumber_produced: 0.0,
        };
    }
    let speed = if options.worker_is_architect {
        ARCHITECT_SPEED
    } else {
        1.0
    };
    let mut progress = old_progress + non_negative(elapsed_sec) * speed;
    let cycles_by_time = (progress / SAWMILL_CYCLE_SEC).floor();
    let cycles_by_logs = (non_negative(options.logs_available) / SAWMILL_LOGS_PER_CYCLE).floor();
    let cycles_by_capacity =
        (non_negative(options.lumber_headroom) / SAWMILL_LUMBER_PER_CYCLE).floor();
    let cycles = cycles_by_time
        .min(cycles_by_logs)
        .min(cycles_by_capacity)
        .max(0.0);
    progress = (progress - cycles * SAWMILL_CYCLE_SEC).min(SAWMILL_CYCLE_SEC);

    SawmillStep {
        next_progress: progress,
        logs_used: cycles * SAWMILL_LOGS_PER_CYCLE,
        lumber_produced: cycles * SAWMILL_LUMBER_PER_CYCLE,
    }
}

/// Construction draws from new lumber first, then legacy planks. This is a spend
/// allocation only: it never converts one stock into the other.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimberSpend {
    pub lumber_used: f64,
    pub legacy_planks_used: f64,
    pub covered: bool,
}

#[must_use]
pub fn allocate_construction_timber(required: f64, lumber: f64, legacy_planks: f64) -> TimberSpend {
    let required = non_negative(required);
    let lumber_used = non_negative(lumber).min(required);
    let remaining = required - lumber_used;
    let legacy_planks_used = non_negative(legacy_planks).min(remaining);
    TimberSpend {
        lumber_used,
        legacy_planks_used,
        covered: lumber_used + legacy_planks_used >= required,
    }
}

fn non_negative(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mill(grain: f64, flour: f64, flour_room: f64, food_room: f64) -> MillOptions {
        MillOptions {
            has_worker: true,
            worker_is_architect: false,
            grain_available: grain,
            flour_available: flour,
            flour_headroom: flour_room,
            food_headroom: food_room,
        }
    }

    fn sawmill(logs: f64, room: f64) -> SawmillOptions {
        SawmillOptions {
            has_worker: true,
            worker_is_architect: false,
            logs_available: logs,
            lumber_headroom: room,
        }
    }

    #[test]
    fn mill_needs_a_worker_and_an_exact_completed_cycle() {
        let mut options = mill(4.0, 0.0, 100.0, 100.0);
        let partial = advance_mill(0.0, MILL_CYCLE_SEC - 1.0, options);
        assert_eq!(partial.grain_used, 0.0);
        assert_eq!(partial.next_progress, MILL_CYCLE_SEC - 1.0);

        options.has_worker = false;
        let idle = advance_mill(MILL_CYCLE_SEC - 1.0, 100.0, options);
        assert_eq!(idle.next_progress, MILL_CYCLE_SEC - 1.0);
        assert_eq!(idle.grain_used, 0.0);
    }

    #[test]
    fn mill_grinds_then_bakes_deterministically_on_accelerated_ticks() {
        let step = advance_mill(0.0, 2.0 * MILL_CYCLE_SEC, mill(4.0, 0.0, 100.0, 100.0));
        assert_eq!(step.next_progress, 0.0);
        assert_eq!(step.grain_used, 4.0);
        assert_eq!(step.flour_produced, 2.0);
        assert_eq!(step.flour_used, 2.0);
        assert_eq!(step.food_produced, 4.0);
    }

    #[test]
    fn mill_prefers_held_flour_and_obeys_both_output_caps() {
        let baked = advance_mill(0.0, MILL_CYCLE_SEC, mill(100.0, 2.0, 100.0, 4.0));
        assert_eq!(baked.food_produced, 4.0);
        assert_eq!(baked.grain_used, 0.0);

        let blocked = advance_mill(0.0, MILL_CYCLE_SEC, mill(4.0, 2.0, 1.99, 3.99));
        assert_eq!(blocked.food_produced, 0.0);
        assert_eq!(blocked.flour_produced, 0.0);
        assert_eq!(blocked.next_progress, MILL_CYCLE_SEC);
    }

    #[test]
    fn architect_speed_applies_without_changing_recipe_amounts() {
        let mut options = mill(4.0, 0.0, 100.0, 100.0);
        options.worker_is_architect = true;
        let step = advance_mill(0.0, MILL_CYCLE_SEC / 2.0, options);
        assert_eq!(step.grain_used, 4.0);
        assert_eq!(step.flour_produced, 2.0);
    }

    #[test]
    fn sawmill_requires_staff_input_time_and_capacity() {
        let exact = advance_sawmill(0.0, SAWMILL_CYCLE_SEC, sawmill(5.0, 2.0));
        assert_eq!(exact.logs_used, 5.0);
        assert_eq!(exact.lumber_produced, 2.0);
        assert_eq!(exact.next_progress, 0.0);

        let no_logs = advance_sawmill(0.0, SAWMILL_CYCLE_SEC, sawmill(4.99, 100.0));
        assert_eq!(no_logs.logs_used, 0.0);
        assert_eq!(no_logs.next_progress, SAWMILL_CYCLE_SEC);

        let full = advance_sawmill(0.0, SAWMILL_CYCLE_SEC, sawmill(5.0, 1.99));
        assert_eq!(full.lumber_produced, 0.0);
        assert_eq!(full.next_progress, SAWMILL_CYCLE_SEC);
    }

    #[test]
    fn accelerated_sawmill_cycles_are_exact_and_deterministic() {
        let options = sawmill(25.0, 10.0);
        let first = advance_sawmill(300.0, 2_700.0, options);
        let second = advance_sawmill(300.0, 2_700.0, options);
        assert_eq!(first, second);
        assert_eq!(first.logs_used, 25.0);
        assert_eq!(first.lumber_produced, 10.0);
        assert_eq!(first.next_progress, 0.0);
    }

    #[test]
    fn construction_uses_new_lumber_then_legacy_planks_without_conversion() {
        assert_eq!(
            allocate_construction_timber(5.0, 3.0, 4.0),
            TimberSpend {
                lumber_used: 3.0,
                legacy_planks_used: 2.0,
                covered: true,
            }
        );
        assert_eq!(
            allocate_construction_timber(5.0, 1.0, 2.0),
            TimberSpend {
                lumber_used: 1.0,
                legacy_planks_used: 2.0,
                covered: false,
            }
        );
    }
}
