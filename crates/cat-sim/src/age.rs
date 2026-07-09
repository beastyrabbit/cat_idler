//! Cat age calculations ported from `lib/game/age.ts`.

use crate::{needs_constants::LIFE_STAGE_HOURS, types::LifeStage};

const MS_PER_HOUR: f64 = 1000.0 * 60.0 * 60.0;
const STANDARD_DEATH_THRESHOLD_HOURS: f64 = 48.0;
const LEADER_OR_HEALER_DEATH_THRESHOLD_HOURS: f64 = 57.6;
const BASE_DEATH_CHANCE: f64 = 0.01;
const DEATH_CHANCE_PER_HOUR: f64 = 0.005;

#[must_use]
pub fn get_age_in_hours(birth_time_ms: i64, current_time_ms: i64) -> f64 {
    (current_time_ms - birth_time_ms) as f64 / MS_PER_HOUR
}

#[must_use]
pub fn get_life_stage(age_in_hours: f64) -> LifeStage {
    for (stage, hours) in LIFE_STAGE_HOURS {
        if age_in_hours < hours.max {
            return stage;
        }
    }

    LifeStage::Elder
}

#[must_use]
pub fn get_death_chance(age_in_hours: f64, is_leader_or_healer: bool) -> f64 {
    let threshold = if is_leader_or_healer {
        LEADER_OR_HEALER_DEATH_THRESHOLD_HOURS
    } else {
        STANDARD_DEATH_THRESHOLD_HOURS
    };

    if age_in_hours < threshold {
        return 0.0;
    }

    let hours_past_threshold = age_in_hours - threshold;
    BASE_DEATH_CHANCE + hours_past_threshold * DEATH_CHANCE_PER_HOUR
}

/// Returns whether an old-age death roll succeeds.
///
/// The TypeScript source calls `Math.random()` internally. The Rust port takes
/// `roll` as an argument so callers can route randomness through the deterministic
/// simulation RNG chain.
#[must_use]
pub fn should_die_of_old_age(age_in_hours: f64, is_leader_or_healer: bool, roll: f64) -> bool {
    roll < get_death_chance(age_in_hours, is_leader_or_healer)
}

#[must_use]
pub fn get_age_skill_modifier(life_stage: LifeStage) -> f64 {
    match life_stage {
        LifeStage::Kitten => 0.0,
        LifeStage::Young => 1.5,
        LifeStage::Adult => 1.0,
        LifeStage::Elder => 0.5,
    }
}

#[must_use]
pub fn can_perform_task(
    life_stage: LifeStage,
    task_requires_outside: bool,
    is_dangerous_task: bool,
) -> bool {
    match life_stage {
        LifeStage::Kitten => !task_requires_outside && !is_dangerous_task,
        LifeStage::Young => !is_dangerous_task,
        LifeStage::Adult | LifeStage::Elder => true,
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::{
        can_perform_task, get_age_in_hours, get_age_skill_modifier, get_death_chance,
        get_life_stage, should_die_of_old_age,
    };
    use crate::types::LifeStage;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        source: String,
        note: String,
        age_in_hours: Vec<AgeInHoursCase>,
        life_stages: Vec<LifeStageCase>,
        death_chances: Vec<DeathChanceCase>,
        skill_modifiers: Vec<SkillModifierCase>,
        task_permissions: Vec<TaskPermissionCase>,
        old_age_rolls: Vec<OldAgeRollCase>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct AgeInHoursCase {
        birth_time: i64,
        current_time: i64,
        age: f64,
    }

    #[derive(Debug, Deserialize)]
    struct LifeStageCase {
        age: f64,
        stage: LifeStage,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DeathChanceCase {
        age: f64,
        is_leader_or_healer: bool,
        chance: f64,
    }

    #[derive(Debug, Deserialize)]
    struct SkillModifierCase {
        stage: LifeStage,
        modifier: f64,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct TaskPermissionCase {
        stage: LifeStage,
        task_requires_outside: bool,
        is_dangerous_task: bool,
        can_perform: bool,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct OldAgeRollCase {
        age: f64,
        is_leader_or_healer: bool,
        roll: f64,
        dies: bool,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!("../../../docs/migration/fixtures/p4/age.json"))
            .expect("age fixture parses")
    }

    fn assert_f64_exact(actual: f64, expected: f64) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }

    #[test]
    fn fixture_is_generated_from_age_ts() {
        let fixture = fixture();

        assert_eq!(fixture.source, "lib/game/age.ts");
        assert_eq!(
            fixture.note,
            "Rust should_die_of_old_age takes an injected roll param instead of calling Math.random()."
        );
    }

    #[test]
    fn age_in_hours_matches_ts_fixture() {
        for case in fixture().age_in_hours {
            assert_f64_exact(
                get_age_in_hours(case.birth_time, case.current_time),
                case.age,
            );
        }
    }

    #[test]
    fn life_stage_boundaries_match_ts_fixture() {
        for case in fixture().life_stages {
            assert_eq!(get_life_stage(case.age), case.stage);
        }
    }

    #[test]
    fn death_chance_boundaries_match_ts_fixture() {
        for case in fixture().death_chances {
            assert_f64_exact(
                get_death_chance(case.age, case.is_leader_or_healer),
                case.chance,
            );
        }
    }

    #[test]
    fn skill_modifiers_match_ts_fixture() {
        for case in fixture().skill_modifiers {
            assert_f64_exact(get_age_skill_modifier(case.stage), case.modifier);
        }
    }

    #[test]
    fn task_permissions_match_ts_fixture() {
        for case in fixture().task_permissions {
            assert_eq!(
                can_perform_task(
                    case.stage,
                    case.task_requires_outside,
                    case.is_dangerous_task
                ),
                case.can_perform
            );
        }
    }

    #[test]
    fn old_age_rolls_match_ts_math_random_cases() {
        for case in fixture().old_age_rolls {
            assert_eq!(
                should_die_of_old_age(case.age, case.is_leader_or_healer, case.roll),
                case.dies
            );
        }
    }

    #[test]
    fn non_finite_inputs_follow_ts_comparison_fallthrough() {
        assert_eq!(get_life_stage(f64::NAN), LifeStage::Elder);
        assert_eq!(get_life_stage(f64::INFINITY), LifeStage::Elder);
        assert!(get_death_chance(f64::NAN, false).is_nan());
        assert_eq!(get_death_chance(f64::NEG_INFINITY, false), 0.0);
        assert_eq!(get_death_chance(f64::INFINITY, false), f64::INFINITY);
        assert!(!should_die_of_old_age(f64::NAN, false, 0.0));
    }
}
