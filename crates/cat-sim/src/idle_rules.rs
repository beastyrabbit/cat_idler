//! Idle automation rules ported from `lib/game/idleRules.ts`.

use crate::{
    entities::{ColonyStatus, Resources},
    idle_engine::UpgradeLevels,
    types::JobKind,
};

pub const DEFAULT_CRITICAL_MS: i64 = 5 * 60 * 1000;
pub const DEFAULT_RITUAL_REQUEST_WINDOW_MS: i64 = 12 * 60 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ColonyResources {
    pub food: f64,
    pub water: f64,
    pub herbs: f64,
    pub materials: f64,
    pub blessings: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimalJob {
    pub kind: JobKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TickConsumption {
    pub food_use: f64,
    pub water_use: f64,
}

pub trait FoodWater {
    fn food(&self) -> f64;
    fn water(&self) -> f64;
}

pub trait FoodWaterHerbs: FoodWater {
    fn herbs(&self) -> f64;
}

impl FoodWater for ColonyResources {
    fn food(&self) -> f64 {
        self.food
    }

    fn water(&self) -> f64 {
        self.water
    }
}

impl FoodWaterHerbs for ColonyResources {
    fn herbs(&self) -> f64 {
        self.herbs
    }
}

impl FoodWater for Resources {
    fn food(&self) -> f64 {
        self.food
    }

    fn water(&self) -> f64 {
        self.water
    }
}

impl FoodWaterHerbs for Resources {
    fn herbs(&self) -> f64 {
        self.herbs
    }
}

impl<T: FoodWater> FoodWater for &T {
    fn food(&self) -> f64 {
        (*self).food()
    }

    fn water(&self) -> f64 {
        (*self).water()
    }
}

impl<T: FoodWaterHerbs> FoodWaterHerbs for &T {
    fn herbs(&self) -> f64 {
        (*self).herbs()
    }
}

impl From<JobKind> for MinimalJob {
    fn from(kind: JobKind) -> Self {
        Self { kind }
    }
}

#[must_use]
pub fn has_conflicting_strategic_job(kind: JobKind, jobs: &[MinimalJob]) -> bool {
    match kind {
        JobKind::LeaderPlanHunt => jobs
            .iter()
            .any(|job| matches!(job.kind, JobKind::LeaderPlanHunt | JobKind::HuntExpedition)),
        JobKind::LeaderPlanHouse => jobs
            .iter()
            .any(|job| matches!(job.kind, JobKind::LeaderPlanHouse | JobKind::BuildHouse)),
        JobKind::Ritual => jobs.iter().any(|job| job.kind == JobKind::Ritual),
        _ => false,
    }
}

#[must_use]
pub fn should_auto_queue_hunt(food: f64, jobs: &[MinimalJob]) -> bool {
    if food >= 12.0 {
        return false;
    }

    !has_conflicting_strategic_job(JobKind::LeaderPlanHunt, jobs)
}

#[must_use]
pub fn should_auto_queue_build(materials: f64, jobs: &[MinimalJob]) -> bool {
    if materials >= 8.0 {
        return false;
    }

    !has_conflicting_strategic_job(JobKind::LeaderPlanHouse, jobs)
}

#[must_use]
pub fn should_start_ritual(
    ritual_requested_at: Option<i64>,
    resources: impl FoodWater,
    jobs: &[MinimalJob],
) -> bool {
    if !js_truthy_timestamp(ritual_requested_at) {
        return false;
    }

    if resources.food() < 16.0 || resources.water() < 16.0 {
        return false;
    }

    !has_conflicting_strategic_job(JobKind::Ritual, jobs)
}

#[must_use]
pub fn consumption_for_tick(
    cat_count: f64,
    elapsed_sec: f64,
    upgrades: UpgradeLevels,
) -> TickConsumption {
    // Resilience max level is 10 -> minimum scale 0.20, clamped to 0.45 floor.
    let resilience_scale = js_max(0.45, 1.0 - upgrades.resilience * 0.08);

    TickConsumption {
        food_use: ((cat_count * elapsed_sec) / 3600.0) * resilience_scale,
        water_use: ((cat_count * elapsed_sec) / 3000.0) * resilience_scale,
    }
}

#[must_use]
pub fn next_colony_status(resources: impl FoodWaterHerbs) -> ColonyStatus {
    let total_supply = resources.food() + resources.water() + resources.herbs();

    if total_supply < 20.0 {
        return ColonyStatus::Struggling;
    }

    if total_supply > 70.0 {
        return ColonyStatus::Thriving;
    }

    ColonyStatus::Starting
}

#[must_use]
pub fn should_track_critical(
    resources: impl FoodWater,
    unattended_hours: f64,
    resilience_hours: f64,
) -> bool {
    let critically_low = resources.food() <= 0.0 || resources.water() <= 0.0;
    critically_low && unattended_hours >= resilience_hours
}

#[must_use]
pub fn should_reset_from_critical(critical_since: Option<i64>, now: i64) -> bool {
    should_reset_from_critical_after(critical_since, now, DEFAULT_CRITICAL_MS)
}

#[must_use]
pub fn should_reset_from_critical_after(
    critical_since: Option<i64>,
    now: i64,
    critical_ms: i64,
) -> bool {
    let Some(critical_since) = critical_since else {
        return false;
    };

    if critical_since == 0 {
        return false;
    }

    now - critical_since >= critical_ms
}

#[must_use]
pub fn ritual_request_is_fresh(ritual_requested_at: Option<i64>, now: i64) -> bool {
    ritual_request_is_fresh_with_window(ritual_requested_at, now, DEFAULT_RITUAL_REQUEST_WINDOW_MS)
}

#[must_use]
pub fn ritual_request_is_fresh_with_window(
    ritual_requested_at: Option<i64>,
    now: i64,
    window_ms: i64,
) -> bool {
    let Some(ritual_requested_at) = ritual_requested_at else {
        return false;
    };

    ritual_requested_at != 0 && now - ritual_requested_at < window_ms
}

fn js_truthy_timestamp(value: Option<i64>) -> bool {
    matches!(value, Some(value) if value != 0)
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

#[cfg(test)]
mod tests {
    use super::{
        ColonyResources, DEFAULT_CRITICAL_MS, DEFAULT_RITUAL_REQUEST_WINDOW_MS, MinimalJob,
        consumption_for_tick, has_conflicting_strategic_job, next_colony_status,
        ritual_request_is_fresh, ritual_request_is_fresh_with_window, should_auto_queue_build,
        should_auto_queue_hunt, should_reset_from_critical, should_reset_from_critical_after,
        should_start_ritual, should_track_critical,
    };
    use crate::{entities::ColonyStatus, idle_engine::UpgradeLevels, types::JobKind};

    fn resources(food: f64, water: f64, herbs: f64) -> ColonyResources {
        ColonyResources {
            food,
            water,
            herbs,
            materials: 0.0,
            blessings: 0.0,
        }
    }

    fn assert_f64_bits(actual: f64, expected: f64, label: &str) {
        assert_eq!(actual.to_bits(), expected.to_bits(), "{label}");
    }

    #[test]
    fn strategic_conflicts_match_idle_rules_ts() {
        assert!(has_conflicting_strategic_job(
            JobKind::LeaderPlanHunt,
            &[MinimalJob {
                kind: JobKind::HuntExpedition,
            }],
        ));
        assert!(has_conflicting_strategic_job(
            JobKind::LeaderPlanHunt,
            &[MinimalJob {
                kind: JobKind::LeaderPlanHunt,
            }],
        ));
        assert!(has_conflicting_strategic_job(
            JobKind::LeaderPlanHouse,
            &[MinimalJob {
                kind: JobKind::BuildHouse,
            }],
        ));
        assert!(has_conflicting_strategic_job(
            JobKind::LeaderPlanHouse,
            &[MinimalJob {
                kind: JobKind::LeaderPlanHouse,
            }],
        ));
        assert!(has_conflicting_strategic_job(
            JobKind::Ritual,
            &[MinimalJob {
                kind: JobKind::Ritual,
            }],
        ));
        assert!(!has_conflicting_strategic_job(
            JobKind::LeaderPlanHunt,
            &[MinimalJob {
                kind: JobKind::SupplyFood,
            }],
        ));
        assert!(!has_conflicting_strategic_job(
            JobKind::SupplyFood,
            &[MinimalJob {
                kind: JobKind::Ritual,
            }],
        ));
    }

    #[test]
    fn auto_queues_hunt_and_build_only_below_thresholds_without_conflicts() {
        assert!(should_auto_queue_hunt(11.999, &[]));
        assert!(should_auto_queue_hunt(11.0, &[]));
        assert!(!should_auto_queue_hunt(12.0, &[]));
        assert!(!should_auto_queue_hunt(
            3.0,
            &[MinimalJob {
                kind: JobKind::LeaderPlanHunt,
            }],
        ));
        assert!(!should_auto_queue_hunt(
            3.0,
            &[MinimalJob {
                kind: JobKind::HuntExpedition,
            }],
        ));

        assert!(should_auto_queue_build(7.999, &[]));
        assert!(should_auto_queue_build(7.0, &[]));
        assert!(!should_auto_queue_build(8.0, &[]));
        assert!(!should_auto_queue_build(
            2.0,
            &[MinimalJob {
                kind: JobKind::BuildHouse,
            }],
        ));
        assert!(!should_auto_queue_build(
            2.0,
            &[MinimalJob {
                kind: JobKind::LeaderPlanHouse,
            }],
        ));
    }

    #[test]
    fn ritual_starts_only_when_requested_supplied_and_conflict_free() {
        assert!(!should_start_ritual(None, resources(20.0, 20.0, 0.0), &[]));
        assert!(!should_start_ritual(
            Some(0),
            resources(20.0, 20.0, 0.0),
            &[]
        ));
        assert!(!should_start_ritual(
            Some(1_000),
            resources(15.999, 20.0, 0.0),
            &[],
        ));
        assert!(!should_start_ritual(
            Some(1_000),
            resources(20.0, 15.999, 0.0),
            &[],
        ));
        assert!(!should_start_ritual(
            Some(1_000),
            resources(20.0, 20.0, 0.0),
            &[MinimalJob {
                kind: JobKind::Ritual,
            }],
        ));
        assert!(should_start_ritual(
            Some(1_000),
            resources(16.0, 16.0, 0.0),
            &[],
        ));
    }

    #[test]
    fn consumption_for_tick_uses_resilience_scale_and_floor() {
        let base = consumption_for_tick(
            5.0,
            600.0,
            UpgradeLevels {
                resilience: 0.0,
                ..UpgradeLevels::default()
            },
        );
        assert_f64_bits(base.food_use, 0.8333333333333334, "base food");
        assert_f64_bits(base.water_use, 1.0, "base water");

        let level_four = consumption_for_tick(
            5.0,
            600.0,
            UpgradeLevels {
                resilience: 4.0,
                ..UpgradeLevels::default()
            },
        );
        assert_f64_bits(
            level_four.food_use,
            0.5666666666666667,
            "resilience level 4 food",
        );
        assert_f64_bits(
            level_four.water_use,
            0.6799999999999999,
            "resilience level 4 water",
        );

        let level_ten = consumption_for_tick(
            9.0,
            1800.0,
            UpgradeLevels {
                resilience: 10.0,
                ..UpgradeLevels::default()
            },
        );
        assert_f64_bits(level_ten.food_use, 2.025, "resilience floor food");
        assert_f64_bits(level_ten.water_use, 2.43, "resilience floor water");
    }

    #[test]
    fn colony_status_uses_strict_supply_bands() {
        assert_eq!(
            next_colony_status(resources(1.0, 2.0, 3.0)),
            ColonyStatus::Struggling,
        );
        assert_eq!(
            next_colony_status(resources(19.0, 0.5, 0.499)),
            ColonyStatus::Struggling,
        );
        assert_eq!(
            next_colony_status(resources(20.0, 0.0, 0.0)),
            ColonyStatus::Starting,
        );
        assert_eq!(
            next_colony_status(resources(10.0, 10.0, 10.0)),
            ColonyStatus::Starting,
        );
        assert_eq!(
            next_colony_status(resources(70.0, 0.0, 0.0)),
            ColonyStatus::Starting,
        );
        assert_eq!(
            next_colony_status(resources(30.0, 30.0, 20.0)),
            ColonyStatus::Thriving,
        );
    }

    #[test]
    fn critical_tracking_and_reset_thresholds_match_idle_rules_ts() {
        assert!(should_track_critical(resources(0.0, 10.0, 0.0), 5.0, 4.0));
        assert!(should_track_critical(
            resources(10.0, -0.001, 0.0),
            4.0,
            4.0,
        ));
        assert!(!should_track_critical(resources(0.0, 10.0, 0.0), 3.0, 4.0,));
        assert!(!should_track_critical(
            resources(0.001, 10.0, 0.0),
            5.0,
            4.0,
        ));

        let now = 1_700_000_000_000;
        assert_eq!(DEFAULT_CRITICAL_MS, 300_000);
        assert!(!should_reset_from_critical(None, now));
        assert!(!should_reset_from_critical(Some(0), now));
        assert!(should_reset_from_critical(Some(now - 5 * 60 * 1000), now,));
        assert!(!should_reset_from_critical(Some(now - 4 * 60 * 1000), now,));
        assert!(should_reset_from_critical_after(
            Some(now - 15_000),
            now,
            10_000,
        ));
    }

    #[test]
    fn ritual_request_freshness_uses_twelve_hour_strict_window() {
        let now = 1_700_000_000_000;
        assert_eq!(DEFAULT_RITUAL_REQUEST_WINDOW_MS, 43_200_000);
        assert!(!ritual_request_is_fresh(None, now));
        assert!(!ritual_request_is_fresh(Some(0), now));
        assert!(ritual_request_is_fresh(Some(now - 1_000), now));
        assert!(ritual_request_is_fresh(
            Some(now - DEFAULT_RITUAL_REQUEST_WINDOW_MS + 1),
            now,
        ));
        assert!(!ritual_request_is_fresh(
            Some(now - DEFAULT_RITUAL_REQUEST_WINDOW_MS),
            now,
        ));
        assert!(!ritual_request_is_fresh(
            Some(now - 13 * 60 * 60 * 1000),
            now,
        ));
        assert!(ritual_request_is_fresh_with_window(
            Some(now - 9_999),
            now,
            10_000,
        ));
        assert!(!ritual_request_is_fresh_with_window(
            Some(now - 10_000),
            now,
            10_000,
        ));
    }
}
