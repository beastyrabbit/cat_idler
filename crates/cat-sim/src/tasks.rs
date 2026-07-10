//! Task assignment helpers ported from `lib/game/tasks.ts`.
//!
//! The TypeScript `getAssignedCat` uses raw `Math.random()` for leader
//! misassignment. This Rust port accepts injected rolls so the behaviour remains
//! deterministic and testable in `cat-sim`.

use crate::{
    cost_constants::task_skill,
    entities::{Cat, CatStats},
    needs_constants::LEADER_QUALITY,
    types::{LifeStage, TaskType},
};

const HOUR_MS: f64 = 1000.0 * 60.0 * 60.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssignmentRolls {
    pub wrong_assignment: f64,
    pub wrong_cat: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssignedCat<'a> {
    pub cat: Option<&'a Cat>,
    pub is_optimal: bool,
}

#[must_use]
pub fn get_optimal_cat_for_task(
    cats: &[Cat],
    task_type: TaskType,
    current_time_ms: i64,
) -> Option<&Cat> {
    if cats.is_empty() {
        return None;
    }

    let is_dangerous_task = matches!(
        task_type,
        TaskType::Hunt | TaskType::Patrol | TaskType::Guard
    );
    let requires_outside = matches!(
        task_type,
        TaskType::Hunt
            | TaskType::GatherHerbs
            | TaskType::FetchWater
            | TaskType::Explore
            | TaskType::Patrol
    );
    let relevant_skill = task_skill(task_type);

    let mut eligible_cats = cats.iter().filter(|cat| {
        let age = get_age_in_hours(cat.birth_time, current_time_ms);
        let life_stage = get_life_stage(age);
        can_perform_task(life_stage, requires_outside, is_dangerous_task)
    });

    let first_cat = eligible_cats.next()?;
    let mut best_cat = first_cat;
    let mut best_skill = stat_value(&first_cat.stats, relevant_skill);

    for cat in eligible_cats {
        let skill = stat_value(&cat.stats, relevant_skill);
        if skill > best_skill {
            best_skill = skill;
            best_cat = cat;
        }
    }

    Some(best_cat)
}

#[must_use]
pub fn get_assignment_time(leadership_stat: f64) -> u32 {
    if leadership_stat <= f64::from(LEADER_QUALITY.bad.max) {
        LEADER_QUALITY.bad.time
    } else if leadership_stat <= f64::from(LEADER_QUALITY.okay.max) {
        LEADER_QUALITY.okay.time
    } else if leadership_stat <= f64::from(LEADER_QUALITY.good.max) {
        LEADER_QUALITY.good.time
    } else {
        LEADER_QUALITY.great.time
    }
}

#[must_use]
pub fn get_assigned_cat(
    cats: &[Cat],
    task_type: TaskType,
    leadership_stat: f64,
    current_time_ms: i64,
    rolls: AssignmentRolls,
) -> AssignedCat<'_> {
    assert_roll(rolls.wrong_assignment);
    assert_roll(rolls.wrong_cat);

    let Some(optimal_cat) = get_optimal_cat_for_task(cats, task_type, current_time_ms) else {
        return AssignedCat {
            cat: None,
            is_optimal: false,
        };
    };

    let should_assign_wrong = rolls.wrong_assignment < wrong_chance(leadership_stat);
    if !should_assign_wrong {
        return AssignedCat {
            cat: Some(optimal_cat),
            is_optimal: true,
        };
    }

    let other_cats: Vec<_> = cats.iter().filter(|cat| cat.id != optimal_cat.id).collect();
    if other_cats.is_empty() {
        return AssignedCat {
            cat: Some(optimal_cat),
            is_optimal: true,
        };
    }

    let random_index = (rolls.wrong_cat * other_cats.len() as f64).floor() as usize;

    AssignedCat {
        cat: Some(other_cats[random_index]),
        is_optimal: false,
    }
}

fn wrong_chance(leadership_stat: f64) -> f64 {
    if leadership_stat <= f64::from(LEADER_QUALITY.bad.max) {
        LEADER_QUALITY.bad.wrong_chance
    } else if leadership_stat <= f64::from(LEADER_QUALITY.okay.max) {
        LEADER_QUALITY.okay.wrong_chance
    } else if leadership_stat <= f64::from(LEADER_QUALITY.good.max) {
        LEADER_QUALITY.good.wrong_chance
    } else {
        LEADER_QUALITY.great.wrong_chance
    }
}

fn get_age_in_hours(birth_time_ms: i64, current_time_ms: i64) -> f64 {
    (current_time_ms - birth_time_ms) as f64 / HOUR_MS
}

fn get_life_stage(age_in_hours: f64) -> LifeStage {
    if age_in_hours < 6.0 {
        LifeStage::Kitten
    } else if age_in_hours < 24.0 {
        LifeStage::Young
    } else if age_in_hours < 48.0 {
        LifeStage::Adult
    } else {
        LifeStage::Elder
    }
}

fn can_perform_task(
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

fn stat_value(stats: &CatStats, skill: &str) -> f64 {
    match skill {
        "attack" => stats.attack,
        "defense" => stats.defense,
        "hunting" => stats.hunting,
        "medicine" => stats.medicine,
        "cleaning" => stats.cleaning,
        "building" => stats.building,
        "leadership" => stats.leadership,
        "vision" => stats.vision,
        _ => unreachable!("TASK_TO_SKILL only references known CatStats fields"),
    }
}

fn assert_roll(roll: f64) {
    assert!(
        (0.0..1.0).contains(&roll),
        "injected assignment rolls must be finite f64 values in [0, 1)"
    );
}

#[cfg(test)]
mod tests {
    use crate::{
        entities::{Cat, CatNeeds, CatStats, MapType, Position, RoleXp},
        tasks::{AssignmentRolls, get_assigned_cat, get_assignment_time, get_optimal_cat_for_task},
        types::TaskType,
    };

    const NOW: i64 = 1_700_000_000_000;
    const HOUR_MS: i64 = 60 * 60 * 1000;

    fn cat(id: &str, birth_time: i64, stats: CatStats) -> Cat {
        Cat {
            id: id.to_owned(),
            colony_id: "colony-1".to_owned(),
            name: id.to_owned(),
            parent_ids: vec![None, None],
            birth_time,
            death_time: None,
            stats,
            needs: CatNeeds {
                hunger: 0.0,
                thirst: 0.0,
                rest: 0.0,
                health: 100.0,
            },
            current_task: None,
            position: Position {
                map: MapType::Colony,
                x: 0.0,
                y: 0.0,
            },
            destination: None,
            carrying: None,
            activity: crate::entities::CatActivity::Idle,
            is_pregnant: false,
            pregnancy_due_time: None,
            age_hours: 0.0,
            pregnancy_due_age_hours: None,
            pregnancy_mate_id: None,
            sprite_params: None,
            specialization: None,
            role_xp: RoleXp::default(),
            skills: Default::default(),
        }
    }

    fn adult(id: &str, stats: CatStats) -> Cat {
        cat(id, NOW - (30 * HOUR_MS), stats)
    }

    fn stats_with(skill: &str, value: f64) -> CatStats {
        let mut stats = CatStats::default();
        match skill {
            "attack" => stats.attack = value,
            "defense" => stats.defense = value,
            "hunting" => stats.hunting = value,
            "medicine" => stats.medicine = value,
            "cleaning" => stats.cleaning = value,
            "building" => stats.building = value,
            "leadership" => stats.leadership = value,
            "vision" => stats.vision = value,
            other => panic!("unknown test skill {other}"),
        }
        stats
    }

    #[test]
    fn optimal_cat_uses_task_to_skill_and_keeps_first_tie() {
        let low_hunter = adult("low", stats_with("hunting", 4.0));
        let best_hunter = adult("best", stats_with("hunting", 12.0));
        let tied_hunter = adult("tied", stats_with("hunting", 12.0));
        let cats = vec![low_hunter, best_hunter, tied_hunter];

        let result = get_optimal_cat_for_task(&cats, TaskType::Hunt, NOW);

        assert_eq!(result.map(|cat| cat.id.as_str()), Some("best"));
    }

    #[test]
    fn optimal_cat_uses_medicine_for_heal() {
        let hunter = adult("hunter", stats_with("hunting", 100.0));
        let healer = adult("healer", stats_with("medicine", 8.0));
        let cats = vec![hunter, healer];

        let result = get_optimal_cat_for_task(&cats, TaskType::Heal, NOW);

        assert_eq!(result.map(|cat| cat.id.as_str()), Some("healer"));
    }

    #[test]
    fn age_gating_matches_typescript_task_rules() {
        let kitten = cat("kitten", NOW - (5 * HOUR_MS), stats_with("hunting", 100.0));
        let young = cat("young", NOW - (12 * HOUR_MS), stats_with("hunting", 90.0));
        let adult_cat = adult("adult", stats_with("hunting", 1.0));
        let cats = vec![kitten, young, adult_cat];

        assert_eq!(
            get_optimal_cat_for_task(&cats, TaskType::Hunt, NOW).map(|cat| cat.id.as_str()),
            Some("adult")
        );

        assert_eq!(
            get_optimal_cat_for_task(&cats, TaskType::GatherHerbs, NOW).map(|cat| cat.id.as_str()),
            Some("young")
        );

        assert_eq!(
            get_optimal_cat_for_task(&cats, TaskType::Clean, NOW).map(|cat| cat.id.as_str()),
            Some("kitten")
        );
    }

    #[test]
    fn assignment_time_uses_leader_quality_boundaries() {
        assert_eq!(get_assignment_time(5.0), 30);
        assert_eq!(get_assignment_time(10.0), 30);
        assert_eq!(get_assignment_time(15.0), 20);
        assert_eq!(get_assignment_time(20.0), 20);
        assert_eq!(get_assignment_time(25.0), 10);
        assert_eq!(get_assignment_time(30.0), 10);
        assert_eq!(get_assignment_time(35.0), 5);
        assert_eq!(get_assignment_time(100.0), 5);
    }

    #[test]
    fn assigned_cat_uses_injected_roll_at_wrong_chance_boundary() {
        let optimal = adult("optimal", stats_with("hunting", 10.0));
        let other = adult("other", stats_with("hunting", 1.0));
        let cats = vec![optimal, other];

        let exact_boundary = get_assigned_cat(
            &cats,
            TaskType::Hunt,
            5.0,
            NOW,
            AssignmentRolls {
                wrong_assignment: 0.4,
                wrong_cat: 0.0,
            },
        );
        assert_eq!(
            exact_boundary.cat.map(|cat| cat.id.as_str()),
            Some("optimal")
        );
        assert!(exact_boundary.is_optimal);

        let below_boundary = get_assigned_cat(
            &cats,
            TaskType::Hunt,
            5.0,
            NOW,
            AssignmentRolls {
                wrong_assignment: 0.399_999,
                wrong_cat: 0.0,
            },
        );
        assert_eq!(below_boundary.cat.map(|cat| cat.id.as_str()), Some("other"));
        assert!(!below_boundary.is_optimal);
    }
}
