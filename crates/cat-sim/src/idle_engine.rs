//! Idle-engine formulas ported from `lib/game/idleEngine.ts`.

use crate::types::{CatSpecialization, JobKind};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct UpgradeLevels {
    pub click_power: f64,
    pub supply_speed: f64,
    pub hunt_mastery: f64,
    pub build_mastery: f64,
    pub ritual_mastery: f64,
    pub resilience: f64,
}

pub const BASE_JOB_SECONDS: [(JobKind, f64); 22] = [
    (JobKind::SupplyFood, 20.0),
    (JobKind::SupplyWater, 15.0),
    (JobKind::LeaderPlanHunt, 30.0 * 60.0),
    (JobKind::HuntExpedition, 8.0 * 60.0 * 60.0),
    // Renewable orchard/bush gathering is a short farming shift.
    (JobKind::GatherFood, 40.0 * 60.0),
    // P16/P17 fishing: a bounded shoreline shift. Physical travel and three
    // conserved cargo trips make the wall clock longer than this work timer.
    (JobKind::Fish, 45.0 * 60.0),
    (JobKind::LeaderPlanHouse, 20.0 * 60.0 * 60.0),
    (JobKind::BuildHouse, 8.0 * 60.0 * 60.0),
    // Physical road projects time each tile independently after its material
    // arrives; this nominal one-tile duration exists for job/tool projections.
    (JobKind::BuildRoad, 60.0),
    (JobKind::Ritual, 6.0 * 60.0 * 60.0),
    (JobKind::Quarry, 2.0 * 60.0 * 60.0),
    (JobKind::GatherLogs, 2.0 * 60.0 * 60.0),
    // A forester must physically reach and tend the stump. The following growth
    // remains ecological game-time state, rather than occupying this worker.
    (JobKind::ReplantTree, 30.0 * 60.0),
    (JobKind::ForageFibre, 2.0 * 60.0 * 60.0),
    (JobKind::Explore, 30.0 * 60.0),
    (JobKind::FetchWater, 45.0 * 60.0),
    (JobKind::TrainWarrior, 3.0 * 60.0 * 60.0),
    (JobKind::ExpandVillage, 10.0 * 60.0),
    // P12.6: CarryOffering is travel-controlled and completes on physical shrine
    // delivery; this nominal value is only a bounded compatibility fallback.
    (JobKind::CarryOffering, 5.0 * 60.0),
    // The delivered goods then receive the former 40-minute offering ceremony.
    (JobKind::PerformOffering, 40.0 * 60.0),
    // P16 (Rust-only, no TS predecessor): a gather-spot mover's "duration" is really
    // just a nominal travel buffer — the job completes as soon as the assigned cat
    // reaches the gather spot and picks up its cargo (see `world_tick`'s gather-spot
    // pickup phase), not on this timer.
    (JobKind::HaulGatherSpot, 5.0 * 60.0),
    // A short renewable shift. It keeps spare adults occupied without outranking
    // concrete resource, construction, staffing, or shrine work.
    (JobKind::VillageMaintenance, 30.0 * 60.0),
];

#[must_use]
pub fn base_job_seconds(kind: JobKind) -> f64 {
    BASE_JOB_SECONDS
        .iter()
        .find_map(|(candidate, seconds)| (*candidate == kind).then_some(*seconds))
        .expect("BASE_JOB_SECONDS covers every JobKind variant")
}

#[must_use]
pub fn normalize_time_scale(time_scale: Option<f64>) -> f64 {
    let Some(time_scale) = time_scale else {
        return 1.0;
    };

    if time_scale == 0.0 || !time_scale.is_finite() {
        return 1.0;
    }

    js_max(1.0, time_scale)
}

#[must_use]
pub fn apply_click_boost_seconds(clicks_in_current_minute: f64, click_power_level: f64) -> f64 {
    let base_boost = 10.0 + click_power_level * 2.0;

    if clicks_in_current_minute <= 30.0 {
        return base_boost;
    }

    let excess = clicks_in_current_minute - 30.0;
    let decay_factor = 0.95_f64.powf(excess / 10.0);
    js_max(1.0, (base_boost * decay_factor).floor())
}

#[must_use]
pub fn get_duration_seconds(
    kind: JobKind,
    specialization: Option<CatSpecialization>,
    upgrades: UpgradeLevels,
    skill: f64,
) -> f64 {
    let base = base_job_seconds(kind);
    // Continuous per-labor skill stacks on top of the discrete specialization cut.
    // At skill == 0 this is exactly 1.0, so behavior matches pre-P12.1 callers.
    let mut multiplier = crate::life_sim::trade_speed_multiplier(skill);

    if matches!(kind, JobKind::SupplyFood | JobKind::SupplyWater) {
        multiplier *= js_max(0.55, 1.0 - upgrades.supply_speed * 0.1);
    }

    if kind == JobKind::HuntExpedition {
        multiplier *= js_max(0.45, 1.0 - upgrades.hunt_mastery * 0.1);
        if specialization == Some(CatSpecialization::Hunter) {
            multiplier *= 0.5;
        }
    }

    if matches!(
        kind,
        JobKind::BuildHouse | JobKind::BuildRoad | JobKind::LeaderPlanHouse
    ) {
        multiplier *= js_max(0.45, 1.0 - upgrades.build_mastery * 0.1);
        if specialization == Some(CatSpecialization::Architect)
            && matches!(kind, JobKind::BuildHouse | JobKind::BuildRoad)
        {
            multiplier *= 0.5;
        }
    }

    // Only the ceremony shares Ritual's mastery/specialization curve. Physical
    // CarryOffering travel is governed by Haul skill and movement.
    if matches!(kind, JobKind::Ritual | JobKind::PerformOffering) {
        multiplier *= js_max(0.4, 1.0 - upgrades.ritual_mastery * 0.12);
        if specialization == Some(CatSpecialization::Ritualist) {
            multiplier *= 0.6;
        }
    }

    js_max(5.0, (base * multiplier).floor())
}

#[must_use]
pub fn get_scaled_duration_seconds(
    kind: JobKind,
    specialization: Option<CatSpecialization>,
    upgrades: UpgradeLevels,
    skill: f64,
    time_scale: Option<f64>,
) -> f64 {
    let base = get_duration_seconds(kind, specialization, upgrades, skill);
    let scale = normalize_time_scale(time_scale);

    js_max(1.0, (base / scale).floor())
}

#[must_use]
pub fn get_hunt_reward(
    hunt_skill: f64,
    specialization: Option<CatSpecialization>,
    hunter_xp: f64,
    upgrades: UpgradeLevels,
) -> f64 {
    let base = 24.0 + (hunt_skill / 15.0).floor();
    let upgrade_bonus = 1.0 + upgrades.hunt_mastery * 0.12;
    let specialist_bonus = if specialization == Some(CatSpecialization::Hunter) && hunter_xp >= 30.0
    {
        1.5
    } else {
        1.0
    };

    js_max(1.0, (base * upgrade_bonus * specialist_bonus).floor())
}

#[must_use]
pub fn get_resilience_hours(upgrades: UpgradeLevels, automation_tier: f64) -> f64 {
    let base = 2.0;
    let upgrade_bonus = upgrades.resilience * 6.0;
    let automation_bonus = (automation_tier * 6.0).floor();

    js_min(96.0, base + upgrade_bonus + automation_bonus)
}

#[must_use]
pub fn get_upgrade_cost(base_cost: f64, current_level: f64) -> f64 {
    base_cost * (current_level + 1.0)
}

#[must_use]
pub fn next_specialization(
    role: CatSpecialization,
    role_xp: f64,
    current: Option<CatSpecialization>,
) -> Option<CatSpecialization> {
    if current.is_some() {
        return current;
    }

    if role_xp >= 10.0 { Some(role) } else { None }
}

fn js_max(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else {
        left.max(right)
    }
}

fn js_min(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else {
        left.min(right)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BASE_JOB_SECONDS, UpgradeLevels, apply_click_boost_seconds, base_job_seconds,
        get_duration_seconds, get_hunt_reward, get_resilience_hours, get_scaled_duration_seconds,
        get_upgrade_cost, next_specialization, normalize_time_scale,
    };
    use crate::types::{CatSpecialization, JobKind};

    fn upgrades() -> UpgradeLevels {
        UpgradeLevels::default()
    }

    fn assert_f64_exact(actual: f64, expected: f64) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }

    #[test]
    fn base_job_seconds_cover_every_job_kind_with_explicit_durations() {
        let expected = [
            (JobKind::SupplyFood, 20.0),
            (JobKind::SupplyWater, 15.0),
            (JobKind::LeaderPlanHunt, 1_800.0),
            (JobKind::HuntExpedition, 28_800.0),
            (JobKind::Fish, 2_700.0),
            (JobKind::LeaderPlanHouse, 72_000.0),
            (JobKind::BuildHouse, 28_800.0),
            (JobKind::BuildRoad, 60.0),
            (JobKind::Ritual, 21_600.0),
            (JobKind::Quarry, 7_200.0),
            (JobKind::GatherLogs, 7_200.0),
            (JobKind::ReplantTree, 1_800.0),
            (JobKind::ForageFibre, 7_200.0),
            (JobKind::Explore, 1_800.0),
            (JobKind::FetchWater, 2_700.0),
            (JobKind::TrainWarrior, 10_800.0),
            (JobKind::ExpandVillage, 600.0),
            (JobKind::CarryOffering, 300.0),
            (JobKind::PerformOffering, 2_400.0),
            (JobKind::HaulGatherSpot, 300.0),
            (JobKind::VillageMaintenance, 1_800.0),
        ];

        assert_eq!(BASE_JOB_SECONDS.len(), JobKind::ALL.len());
        assert_eq!(expected.len(), JobKind::ALL.len());

        for (kind, expected_seconds) in expected {
            let entries: Vec<_> = BASE_JOB_SECONDS
                .iter()
                .filter(|(candidate, _)| *candidate == kind)
                .collect();
            assert_eq!(entries.len(), 1, "{kind:?} must have exactly one entry");
            assert_f64_exact(base_job_seconds(kind), expected_seconds);
        }
    }

    #[test]
    fn normalize_time_scale_matches_typescript_truthiness_and_floor_inputs() {
        assert_f64_exact(normalize_time_scale(None), 1.0);
        assert_f64_exact(normalize_time_scale(Some(0.0)), 1.0);
        assert_f64_exact(normalize_time_scale(Some(f64::NAN)), 1.0);
        assert_f64_exact(normalize_time_scale(Some(f64::INFINITY)), 1.0);
        assert_f64_exact(normalize_time_scale(Some(-4.0)), 1.0);
        assert_f64_exact(normalize_time_scale(Some(20.0)), 20.0);
    }

    #[test]
    fn click_boost_uses_base_power_then_exponential_decay_after_thirty_clicks() {
        assert_f64_exact(apply_click_boost_seconds(1.0, 0.0), 10.0);
        assert_f64_exact(apply_click_boost_seconds(30.0, 0.0), 10.0);
        assert_f64_exact(apply_click_boost_seconds(31.0, 0.0), 9.0);
        assert_f64_exact(apply_click_boost_seconds(61.0, 0.0), 8.0);
        assert_f64_exact(apply_click_boost_seconds(200.0, 0.0), 4.0);
        assert_f64_exact(apply_click_boost_seconds(500.0, 0.0), 1.0);
        assert_f64_exact(apply_click_boost_seconds(1.0, 3.0), 16.0);
        assert_f64_exact(apply_click_boost_seconds(31.0, 3.0), 15.0);
    }

    #[test]
    fn durations_apply_upgrade_caps_and_specialization_multipliers_in_order() {
        // skill == 0.0 keeps every pre-P12.1 expectation intact.
        assert_f64_exact(
            get_duration_seconds(JobKind::SupplyWater, None, upgrades(), 0.0),
            15.0,
        );
        assert_f64_exact(
            get_duration_seconds(
                JobKind::SupplyWater,
                None,
                UpgradeLevels {
                    supply_speed: 4.0,
                    ..upgrades()
                },
                0.0,
            ),
            9.0,
        );
        assert_f64_exact(
            get_duration_seconds(
                JobKind::SupplyFood,
                None,
                UpgradeLevels {
                    supply_speed: 10.0,
                    ..upgrades()
                },
                0.0,
            ),
            11.0,
        );
        assert_f64_exact(
            get_duration_seconds(
                JobKind::HuntExpedition,
                Some(CatSpecialization::Hunter),
                UpgradeLevels {
                    hunt_mastery: 2.0,
                    ..upgrades()
                },
                0.0,
            ),
            11_520.0,
        );
        assert_f64_exact(
            get_duration_seconds(
                JobKind::HuntExpedition,
                Some(CatSpecialization::Hunter),
                UpgradeLevels {
                    hunt_mastery: 10.0,
                    ..upgrades()
                },
                0.0,
            ),
            6_480.0,
        );
        assert_f64_exact(
            get_duration_seconds(
                JobKind::BuildHouse,
                Some(CatSpecialization::Architect),
                UpgradeLevels {
                    build_mastery: 3.0,
                    ..upgrades()
                },
                0.0,
            ),
            10_080.0,
        );
        assert_f64_exact(
            get_duration_seconds(
                JobKind::LeaderPlanHouse,
                Some(CatSpecialization::Architect),
                UpgradeLevels {
                    build_mastery: 3.0,
                    ..upgrades()
                },
                0.0,
            ),
            50_400.0,
        );
        assert_f64_exact(
            get_duration_seconds(
                JobKind::Ritual,
                Some(CatSpecialization::Ritualist),
                UpgradeLevels {
                    ritual_mastery: 2.0,
                    ..upgrades()
                },
                0.0,
            ),
            9_849.0,
        );
        assert_f64_exact(
            get_duration_seconds(
                JobKind::Ritual,
                Some(CatSpecialization::Ritualist),
                UpgradeLevels {
                    ritual_mastery: 6.0,
                    ..upgrades()
                },
                0.0,
            ),
            5_184.0,
        );
        // P12.6: only the perform_offering ceremony shares Ritual's mastery and
        // specialization curve. The preceding carry is governed by physical travel.
        assert_f64_exact(
            get_duration_seconds(
                JobKind::PerformOffering,
                Some(CatSpecialization::Ritualist),
                UpgradeLevels {
                    ritual_mastery: 2.0,
                    ..upgrades()
                },
                0.0,
            ),
            1_094.0,
        );
        assert_f64_exact(
            get_duration_seconds(
                JobKind::PerformOffering,
                Some(CatSpecialization::Ritualist),
                UpgradeLevels {
                    ritual_mastery: 6.0,
                    ..upgrades()
                },
                0.0,
            ),
            576.0,
        );
    }

    #[test]
    fn skill_monotonically_shortens_bounded_duration() {
        let at = |skill: f64| get_duration_seconds(JobKind::Quarry, None, upgrades(), skill);
        let base = at(0.0);
        // skill 0 == today; more skill strictly shortens until the curve saturates.
        assert!(at(5.0) < base);
        assert!(at(30.0) < at(5.0));
        assert!(at(120.0) < at(30.0));
        // Bounded: the speed curve asymptotes to 0.75x, never below the 5s floor.
        assert!(at(1_000_000.0) >= 5.0);
        assert!(at(1_000_000.0) >= (base * 0.75).floor());
    }

    #[test]
    fn scaled_duration_divides_normalized_duration_and_floors_to_one_second() {
        assert_f64_exact(
            get_scaled_duration_seconds(JobKind::HuntExpedition, None, upgrades(), 0.0, Some(20.0)),
            1_440.0,
        );
        assert_f64_exact(
            get_scaled_duration_seconds(JobKind::SupplyWater, None, upgrades(), 0.0, Some(20.0)),
            1.0,
        );
        assert_f64_exact(
            get_scaled_duration_seconds(JobKind::SupplyWater, None, upgrades(), 0.0, Some(0.0)),
            15.0,
        );
    }

    #[test]
    fn hunt_reward_matches_skill_upgrade_and_hunter_xp_threshold_formula() {
        assert_f64_exact(get_hunt_reward(40.0, None, 0.0, upgrades()), 26.0);
        assert_f64_exact(
            get_hunt_reward(
                40.0,
                Some(CatSpecialization::Hunter),
                29.0,
                UpgradeLevels {
                    hunt_mastery: 2.0,
                    ..upgrades()
                },
            ),
            32.0,
        );
        assert_f64_exact(
            get_hunt_reward(
                40.0,
                Some(CatSpecialization::Hunter),
                30.0,
                UpgradeLevels {
                    hunt_mastery: 2.0,
                    ..upgrades()
                },
            ),
            48.0,
        );
    }

    #[test]
    fn resilience_hours_uses_upgrade_bonus_fractional_tier_floor_and_cap() {
        assert_f64_exact(get_resilience_hours(upgrades(), 0.0), 2.0);
        assert_f64_exact(
            get_resilience_hours(
                UpgradeLevels {
                    resilience: 3.0,
                    ..upgrades()
                },
                1.0,
            ),
            26.0,
        );
        assert_f64_exact(
            get_resilience_hours(
                UpgradeLevels {
                    resilience: 3.0,
                    ..upgrades()
                },
                1.5,
            ),
            29.0,
        );
        assert_f64_exact(
            get_resilience_hours(
                UpgradeLevels {
                    resilience: 20.0,
                    ..upgrades()
                },
                20.0,
            ),
            96.0,
        );
    }

    #[test]
    fn upgrade_cost_and_next_specialization_match_threshold_rules() {
        assert_f64_exact(get_upgrade_cost(5.0, 0.0), 5.0);
        assert_f64_exact(get_upgrade_cost(5.0, 2.0), 15.0);

        assert_eq!(
            next_specialization(CatSpecialization::Hunter, 9.0, None),
            None
        );
        assert_eq!(
            next_specialization(CatSpecialization::Hunter, 10.0, None),
            Some(CatSpecialization::Hunter)
        );
        assert_eq!(
            next_specialization(
                CatSpecialization::Hunter,
                40.0,
                Some(CatSpecialization::Architect),
            ),
            Some(CatSpecialization::Architect)
        );
    }
}
