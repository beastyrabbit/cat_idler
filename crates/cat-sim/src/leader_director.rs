//! Utility leader director ported from `lib/game/leaderDirector.ts` and
//! `lib/game/leaderAI.ts`.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::{
    leader_ai::{LeaderDecision, LeaderSnapshot, ThreatBand},
    types::CatSpecialization,
};

pub const EMPLOYMENT_TARGET_RATIO: f64 = 0.7;
pub const IDLE_EMPLOYMENT_FLOOR: f64 = 0.8;
pub const PROJECTION_HORIZON_TICKS: f64 = 6.0;
pub const HUNT_CANCEL_RATIO: f64 = 1.1;
pub const STORAGE_RATIO: f64 = 0.9;
pub const DEN_PRESSURE_THRESHOLD: f64 = 0.8;
pub const RESEARCH_COMFORT_RATIO: f64 = 0.5;
pub const TITHE_FOOD_RATIO: f64 = 0.6;
pub const TITHE_FOOD_AMOUNT: u32 = 20;
pub const TITHE_REFINED_AMOUNT: u32 = 5;
pub const HUNT_MAX_SLOTS_RATIO: f64 = 0.7;
pub const WATER_MAX_SLOTS: u32 = 4;
pub const QUARRY_MAX_SLOTS: u32 = 2;
pub const SCOUT_MAX_SLOTS: u32 = 2;
pub const SCOUT_BASE_SCORE: f64 = 0.3;
pub const STAFF_BASE_SCORE: f64 = 0.45;
pub const WARRIOR_BASE_SCORE: f64 = 0.5;
pub const WARRIOR_MAX_RATIO: f64 = 0.4;
pub const PROJECTION_GATE_RATIO: f64 = 0.9;

const EPS: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WarriorTargetByBand {
    pub calm: u32,
    pub rising: u32,
    pub imminent: u32,
}

pub const WARRIOR_TARGET_BY_BAND: WarriorTargetByBand = WarriorTargetByBand {
    calm: 2,
    rising: 4,
    imminent: 7,
};

#[must_use]
pub fn target_warriors(snapshot: &LeaderSnapshot) -> u32 {
    if !snapshot.has_barracks.unwrap_or(false) {
        return 0;
    }

    let base = match snapshot.threat_band.unwrap_or(ThreatBand::Calm) {
        ThreatBand::Calm => WARRIOR_TARGET_BY_BAND.calm,
        ThreatBand::Rising => WARRIOR_TARGET_BY_BAND.rising,
        ThreatBand::Imminent => WARRIOR_TARGET_BY_BAND.imminent,
    };
    let workforce = workforce_of(snapshot);
    let cap = (workforce * WARRIOR_MAX_RATIO).floor() as u32;

    base.min(1.max(cap))
}

#[must_use]
pub fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

#[must_use]
pub fn deficit_curve(ratio: f64) -> f64 {
    let r = clamp01(ratio);
    (1.0 - r) * (1.0 - r)
}

#[must_use]
pub fn projection_curve(amount: f64, drain_per_tick: f64, horizon_ticks: f64) -> f64 {
    if drain_per_tick <= 0.0 || horizon_ticks <= 0.0 {
        return 0.0;
    }

    let ticks_to_empty = amount.max(0.0) / drain_per_tick.max(EPS);
    clamp01(1.0 - ticks_to_empty / horizon_ticks)
}

#[must_use]
pub fn pressure_curve(pressure: f64, center: f64, steepness: f64) -> f64 {
    clamp01(1.0 / (1.0 + (-steepness * (pressure - center)).exp()))
}

#[must_use]
pub fn surplus_curve(ratio: f64, threshold: f64) -> f64 {
    if ratio <= threshold {
        0.0
    } else {
        clamp01((ratio - threshold) / (1.0 - threshold))
    }
}

#[must_use]
pub fn combine_or(a: f64, b: f64) -> f64 {
    clamp01(1.0 - (1.0 - clamp01(a)) * (1.0 - clamp01(b)))
}

#[must_use]
pub fn projection_gate(fill_ratio: f64) -> f64 {
    clamp01((PROJECTION_GATE_RATIO - fill_ratio) / PROJECTION_GATE_RATIO)
}

#[must_use]
pub fn survival_score(fill_ratio: f64, amount: f64, drain_per_tick: f64) -> f64 {
    combine_or(
        deficit_curve(fill_ratio),
        projection_curve(amount, drain_per_tick, PROJECTION_HORIZON_TICKS)
            * projection_gate(fill_ratio),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaborGoalKind {
    Hunt,
    FetchWater,
    Quarry,
    Scout,
    TrainWarrior,
    AssignWorkshop,
    AssignResearch,
    AssignSmithy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalStat {
    Hunting,
    Building,
    Vision,
    Medicine,
    AttackDefense,
    Leadership,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoalSkill {
    pub skill: GoalStat,
    pub prefer_specialization: Option<CatSpecialization>,
}

#[must_use]
pub fn goal_skill(kind: LaborGoalKind) -> GoalSkill {
    match kind {
        LaborGoalKind::Hunt => GoalSkill {
            skill: GoalStat::Hunting,
            prefer_specialization: Some(CatSpecialization::Hunter),
        },
        LaborGoalKind::FetchWater => GoalSkill {
            skill: GoalStat::Hunting,
            prefer_specialization: None,
        },
        LaborGoalKind::Quarry => GoalSkill {
            skill: GoalStat::Building,
            prefer_specialization: Some(CatSpecialization::Architect),
        },
        LaborGoalKind::Scout => GoalSkill {
            skill: GoalStat::Vision,
            prefer_specialization: None,
        },
        LaborGoalKind::TrainWarrior => GoalSkill {
            skill: GoalStat::AttackDefense,
            prefer_specialization: None,
        },
        LaborGoalKind::AssignWorkshop => GoalSkill {
            skill: GoalStat::Building,
            prefer_specialization: None,
        },
        LaborGoalKind::AssignResearch => GoalSkill {
            skill: GoalStat::Medicine,
            prefer_specialization: None,
        },
        LaborGoalKind::AssignSmithy => GoalSkill {
            skill: GoalStat::Building,
            prefer_specialization: Some(CatSpecialization::Architect),
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaborGoalMode {
    Scaled,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaborGoal {
    pub kind: LaborGoalKind,
    pub score: f64,
    pub max_slots: u32,
    pub in_flight: u32,
    pub hard_cap: u32,
    pub vetoed: bool,
    pub mode: LaborGoalMode,
}

#[must_use]
pub fn goal_open_slots(goal: &LaborGoal) -> u32 {
    if goal.vetoed {
        return 0;
    }

    let target = match goal.mode {
        LaborGoalMode::Fixed => goal.max_slots,
        LaborGoalMode::Scaled => (goal.score * f64::from(goal.max_slots) + 0.5).floor() as u32,
    };

    target
        .saturating_sub(goal.in_flight)
        .min(goal.hard_cap.saturating_sub(goal.in_flight))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenSlots {
    pub goal: LaborGoalKind,
    pub count: u32,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatBriefStats {
    pub hunting: f64,
    pub building: f64,
    pub vision: f64,
    pub medicine: f64,
    pub attack: f64,
    pub defense: f64,
    pub leadership: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatBrief {
    pub id: String,
    pub specialization: Option<CatSpecialization>,
    pub stats: CatBriefStats,
}

#[must_use]
pub fn assignment_fit(cat: &CatBrief, goal: LaborGoalKind) -> f64 {
    let spec = goal_skill(goal);
    let base = match spec.skill {
        GoalStat::Hunting => cat.stats.hunting,
        GoalStat::Building => cat.stats.building,
        GoalStat::Vision => cat.stats.vision,
        GoalStat::Medicine => cat.stats.medicine,
        GoalStat::AttackDefense => cat.stats.attack + cat.stats.defense,
        GoalStat::Leadership => cat.stats.leadership,
    };
    let spec_match =
        spec.prefer_specialization.is_some() && cat.specialization == spec.prefer_specialization;

    base * if spec_match { 1.5 } else { 1.0 }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assignment {
    pub cat_id: String,
    pub goal: LaborGoalKind,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MatchOptions {
    pub exclude_warriors_from_training: bool,
}

#[must_use]
pub fn match_cats_to_slots(
    slots: &[OpenSlots],
    cats: &[CatBrief],
    options: MatchOptions,
) -> Vec<Assignment> {
    let mut flat = Vec::new();
    for slot in slots {
        for _ in 0..slot.count {
            flat.push(slot.goal);
        }
    }

    let mut pool = cats.to_vec();
    let mut assignments = Vec::new();
    for goal in flat {
        let mut best_idx = None;
        let mut best_fit = f64::NEG_INFINITY;
        for (idx, cat) in pool.iter().enumerate() {
            if goal == LaborGoalKind::TrainWarrior
                && options.exclude_warriors_from_training
                && cat.specialization == Some(CatSpecialization::Warrior)
            {
                continue;
            }

            let fit = assignment_fit(cat, goal);
            if fit > best_fit {
                best_fit = fit;
                best_idx = Some(idx);
            }
        }

        if let Some(idx) = best_idx {
            assignments.push(Assignment {
                cat_id: pool[idx].id.clone(),
                goal,
            });
            pool.remove(idx);
        }
    }

    assignments
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectorPlan {
    pub decisions: Vec<LeaderDecision>,
    pub slots: Vec<OpenSlots>,
}

#[must_use]
pub fn direct_colony(snapshot: &LeaderSnapshot) -> DirectorPlan {
    let mut decisions = Vec::new();
    let food_r = ratio(snapshot.resources.food, snapshot.food_capacity);

    if food_r > HUNT_CANCEL_RATIO && snapshot.active_hunts > 0 {
        decisions.push(LeaderDecision::CancelHunts);
    }
    if snapshot.starving.unwrap_or(false) && snapshot.training_in_flight.unwrap_or(0) > 0 {
        decisions.push(LeaderDecision::CancelTraining);
    }

    let mut labour_left = snapshot.idle_cats;
    let goals = labor_goals(snapshot);
    let mut ranked = goals.clone();
    ranked.sort_by(rank_goals);

    let mut granted: Vec<(LaborGoalKind, u32)> = Vec::new();
    for goal in &ranked {
        if labour_left == 0 {
            break;
        }

        let want = goal_open_slots(goal);
        let give = want.min(labour_left);
        if give > 0 {
            grant(&mut granted, goal.kind, give);
            labour_left -= give;
        }
    }

    let busy_so_far = snapshot.employed_cats + granted.iter().map(|(_, count)| *count).sum::<u32>();
    let employ_target = (f64::from(able_cats(snapshot)) * IDLE_EMPLOYMENT_FLOOR).ceil() as u32;
    let idle_left = snapshot
        .idle_cats
        .saturating_sub(busy_so_far.saturating_sub(snapshot.employed_cats));
    let mut fill_wanted = idle_left.min(employ_target.saturating_sub(busy_so_far));

    let fill_order = [
        (LaborGoalKind::Hunt, food_r < 1.0),
        (LaborGoalKind::Scout, snapshot.has_frontier),
        (LaborGoalKind::Quarry, snapshot.has_quarry_site),
    ];
    let mut progress = true;
    while fill_wanted > 0 && progress {
        progress = false;
        for (kind, open) in fill_order {
            if fill_wanted == 0 {
                break;
            }
            if !open {
                continue;
            }

            grant(&mut granted, kind, 1);
            fill_wanted -= 1;
            progress = true;
        }
    }

    let mut slots = Vec::new();
    for goal in &ranked {
        let count = granted_count(&granted, goal.kind);
        if count > 0 {
            let original_score = goals
                .iter()
                .find(|candidate| candidate.kind == goal.kind)
                .map_or(0.0, |candidate| candidate.score);
            slots.push(OpenSlots {
                goal: goal.kind,
                count,
                score: original_score,
            });
        }
    }

    let storehouses_in_play = snapshot.storehouse_count + snapshot.storage_plans_in_flight;
    if food_r > STORAGE_RATIO
        && snapshot.storage_plans_in_flight == 0
        && storehouses_in_play < snapshot.storehouse_cap
    {
        decisions.push(LeaderDecision::BuildStorage);
    }

    let shelter = snapshot.housing.capacity + snapshot.housing.committed;
    let pressure = if shelter == 0 {
        f64::INFINITY
    } else {
        f64::from(snapshot.population) / f64::from(shelter)
    };
    if pressure >= DEN_PRESSURE_THRESHOLD && snapshot.den_plans_in_flight == 0 {
        decisions.push(LeaderDecision::BuildDen);
    }

    let tithe_food = if snapshot.resources.food
        > snapshot.food_capacity * TITHE_FOOD_RATIO + f64::from(TITHE_FOOD_AMOUNT)
    {
        TITHE_FOOD_AMOUNT
    } else {
        0
    };
    let tithe_refined = if snapshot.resources.refined >= f64::from(TITHE_REFINED_AMOUNT) {
        TITHE_REFINED_AMOUNT
    } else {
        0
    };
    let blessings = u32::from(tithe_food > 0) + u32::from(tithe_refined > 0);
    if blessings > 0 {
        decisions.push(LeaderDecision::Tithe {
            food: tithe_food,
            refined: tithe_refined,
            blessings,
        });
    }

    DirectorPlan { decisions, slots }
}

#[must_use]
pub fn plan_leader_actions(snapshot: &LeaderSnapshot) -> Vec<LeaderDecision> {
    let plan = direct_colony(snapshot);
    let mut decisions = Vec::new();

    for decision in &plan.decisions {
        if matches!(
            decision,
            LeaderDecision::CancelHunts | LeaderDecision::CancelTraining
        ) {
            decisions.push(decision.clone());
        }
    }

    decisions.extend(plan.slots.iter().map(|slot| match slot.goal {
        LaborGoalKind::Hunt => LeaderDecision::Hunt { count: slot.count },
        LaborGoalKind::FetchWater => LeaderDecision::FetchWater { count: slot.count },
        LaborGoalKind::Quarry => LeaderDecision::Quarry { count: slot.count },
        LaborGoalKind::Scout => LeaderDecision::Scout { count: slot.count },
        LaborGoalKind::TrainWarrior => LeaderDecision::TrainWarrior { count: slot.count },
        LaborGoalKind::AssignWorkshop => LeaderDecision::AssignWorkshop { count: slot.count },
        LaborGoalKind::AssignResearch => LeaderDecision::AssignResearch { count: slot.count },
        LaborGoalKind::AssignSmithy => LeaderDecision::AssignSmithy { count: slot.count },
    }));

    for decision in plan.decisions {
        if !matches!(
            decision,
            LeaderDecision::CancelHunts | LeaderDecision::CancelTraining
        ) {
            decisions.push(decision);
        }
    }

    decisions
}

fn labor_goals(snapshot: &LeaderSnapshot) -> Vec<LaborGoal> {
    let budget = (workforce_of(snapshot) * EMPLOYMENT_TARGET_RATIO).floor();
    let food_r = ratio(snapshot.resources.food, snapshot.food_capacity);
    let water_r = ratio(snapshot.water, snapshot.water_capacity);
    let materials_r = ratio(snapshot.materials, snapshot.materials_capacity);

    let food_score = survival_score(
        food_r,
        snapshot.resources.food,
        snapshot.food_drain_per_tick.unwrap_or(0.0),
    );
    let water_score = survival_score(
        water_r,
        snapshot.water,
        snapshot.water_drain_per_tick.unwrap_or(0.0),
    );
    let materials_score = deficit_curve(materials_r);
    let comfortable = food_r >= RESEARCH_COMFORT_RATIO && water_r >= RESEARCH_COMFORT_RATIO;
    let warrior_gap = i64::from(target_warriors(snapshot))
        - i64::from(snapshot.warrior_count.unwrap_or(0))
        - i64::from(snapshot.training_in_flight.unwrap_or(0));
    let warrior_slots = warrior_gap.max(0) as u32;
    let hunt_slots = (budget * HUNT_MAX_SLOTS_RATIO).ceil() as u32;
    let research_huts = snapshot.research_huts_needing_workers.unwrap_or(0);
    let smithies = snapshot.smithies_needing_workers.unwrap_or(0);

    vec![
        LaborGoal {
            kind: LaborGoalKind::Hunt,
            score: food_score,
            max_slots: hunt_slots,
            in_flight: snapshot.active_hunts,
            hard_cap: hunt_slots,
            vetoed: food_r >= 1.0,
            mode: LaborGoalMode::Scaled,
        },
        LaborGoal {
            kind: LaborGoalKind::FetchWater,
            score: water_score,
            max_slots: WATER_MAX_SLOTS,
            in_flight: snapshot.active_water_fetchers,
            hard_cap: WATER_MAX_SLOTS,
            vetoed: !snapshot.has_water_site || water_r >= 1.0,
            mode: LaborGoalMode::Scaled,
        },
        LaborGoal {
            kind: LaborGoalKind::Quarry,
            score: materials_score,
            max_slots: QUARRY_MAX_SLOTS,
            in_flight: snapshot.active_quarries,
            hard_cap: QUARRY_MAX_SLOTS,
            vetoed: !snapshot.has_quarry_site || materials_r >= 1.0,
            mode: LaborGoalMode::Scaled,
        },
        LaborGoal {
            kind: LaborGoalKind::Scout,
            score: SCOUT_BASE_SCORE,
            max_slots: SCOUT_MAX_SLOTS,
            in_flight: snapshot.active_scouts,
            hard_cap: SCOUT_MAX_SLOTS,
            vetoed: !snapshot.has_frontier,
            mode: LaborGoalMode::Fixed,
        },
        LaborGoal {
            kind: LaborGoalKind::AssignWorkshop,
            score: STAFF_BASE_SCORE,
            max_slots: snapshot.workshops_needing_workers,
            in_flight: 0,
            hard_cap: snapshot.workshops_needing_workers,
            vetoed: snapshot.workshops_needing_workers == 0 || snapshot.starving.unwrap_or(false),
            mode: LaborGoalMode::Fixed,
        },
        LaborGoal {
            kind: LaborGoalKind::AssignResearch,
            score: STAFF_BASE_SCORE,
            max_slots: research_huts,
            in_flight: 0,
            hard_cap: research_huts,
            vetoed: research_huts == 0 || !comfortable,
            mode: LaborGoalMode::Fixed,
        },
        LaborGoal {
            kind: LaborGoalKind::AssignSmithy,
            score: STAFF_BASE_SCORE,
            max_slots: smithies,
            in_flight: 0,
            hard_cap: smithies,
            vetoed: smithies == 0 || snapshot.starving.unwrap_or(false),
            mode: LaborGoalMode::Fixed,
        },
        LaborGoal {
            kind: LaborGoalKind::TrainWarrior,
            score: WARRIOR_BASE_SCORE,
            max_slots: warrior_slots,
            in_flight: 0,
            hard_cap: warrior_slots,
            vetoed: warrior_gap <= 0 || snapshot.starving.unwrap_or(false),
            mode: LaborGoalMode::Fixed,
        },
    ]
}

fn ratio(amount: f64, capacity: f64) -> f64 {
    if capacity <= 0.0 {
        if amount > 0.0 { 1.0 } else { 0.0 }
    } else {
        amount / capacity
    }
}

fn workforce_of(snapshot: &LeaderSnapshot) -> f64 {
    snapshot.workforce.unwrap_or(f64::from(snapshot.population))
}

fn able_cats(snapshot: &LeaderSnapshot) -> u32 {
    snapshot.idle_cats + snapshot.employed_cats
}

fn goal_order(kind: LaborGoalKind) -> usize {
    match kind {
        LaborGoalKind::FetchWater => 0,
        LaborGoalKind::Hunt => 1,
        LaborGoalKind::Quarry => 2,
        LaborGoalKind::TrainWarrior => 3,
        LaborGoalKind::AssignSmithy => 4,
        LaborGoalKind::AssignWorkshop => 5,
        LaborGoalKind::AssignResearch => 6,
        LaborGoalKind::Scout => 7,
    }
}

fn rank_goals(a: &LaborGoal, b: &LaborGoal) -> Ordering {
    if b.score != a.score {
        return b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal);
    }

    goal_order(a.kind).cmp(&goal_order(b.kind))
}

fn grant(granted: &mut Vec<(LaborGoalKind, u32)>, kind: LaborGoalKind, count: u32) {
    if let Some((_, existing)) = granted.iter_mut().find(|(candidate, _)| *candidate == kind) {
        *existing += count;
    } else {
        granted.push((kind, count));
    }
}

fn granted_count(granted: &[(LaborGoalKind, u32)], kind: LaborGoalKind) -> u32 {
    granted
        .iter()
        .find_map(|(candidate, count)| (*candidate == kind).then_some(*count))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const EPSILON: f64 = 1e-12;

    #[derive(Debug, Deserialize)]
    struct Fixture {
        source: Vec<String>,
        #[serde(rename = "responseCurves")]
        response_curves: Vec<ResponseCurveCase>,
        #[serde(rename = "targetWarriors")]
        target_warriors: Vec<TargetWarriorCase>,
        #[serde(rename = "directColony")]
        direct_colony: Vec<DirectColonyCase>,
        #[serde(rename = "planLeaderActions")]
        plan_leader_actions: PlanLeaderActionsCase,
        #[serde(rename = "assignmentFits")]
        assignment_fits: Vec<AssignmentFitCase>,
        #[serde(rename = "matchCatsToSlots")]
        match_cats_to_slots: Vec<MatchCatsCase>,
        counts: Counts,
    }

    #[derive(Debug, Deserialize)]
    struct Counts {
        #[serde(rename = "responseCurves")]
        response_curves: usize,
        #[serde(rename = "targetWarriors")]
        target_warriors: usize,
        #[serde(rename = "directColony")]
        direct_colony: usize,
        #[serde(rename = "assignmentFits")]
        assignment_fits: usize,
        #[serde(rename = "matchCatsToSlots")]
        match_cats_to_slots: usize,
    }

    #[derive(Debug, Deserialize)]
    struct ResponseCurveCase {
        expression: String,
        value: f64,
    }

    #[derive(Debug, Deserialize)]
    struct TargetWarriorCase {
        name: String,
        snapshot: LeaderSnapshot,
        expected: u32,
    }

    #[derive(Debug, Deserialize)]
    struct DirectColonyCase {
        name: String,
        snapshot: LeaderSnapshot,
        expected: DirectorPlan,
    }

    #[derive(Debug, Deserialize)]
    struct PlanLeaderActionsCase {
        snapshot: LeaderSnapshot,
        direct: DirectorPlan,
        expected: Vec<LeaderDecision>,
    }

    #[derive(Debug, Deserialize)]
    struct AssignmentFitCase {
        name: String,
        cat: CatBrief,
        goal: LaborGoalKind,
        expected: f64,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct MatchCatsCase {
        name: String,
        slots: Vec<OpenSlots>,
        cats: Vec<CatBrief>,
        #[serde(default)]
        options: MatchCatsOptionsFixture,
        expected: Vec<Assignment>,
    }

    #[derive(Debug, Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct MatchCatsOptionsFixture {
        #[serde(default)]
        exclude_warriors_from_training: bool,
    }

    impl From<MatchCatsOptionsFixture> for MatchOptions {
        fn from(value: MatchCatsOptionsFixture) -> Self {
            Self {
                exclude_warriors_from_training: value.exclude_warriors_from_training,
            }
        }
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../docs/migration/fixtures/p3/leader_director.json"
        ))
        .expect("leader director fixture parses")
    }

    fn assert_float_eq(actual: f64, expected: f64, context: &str) {
        if actual.to_bits() == expected.to_bits() {
            return;
        }

        assert!(
            (actual - expected).abs() <= EPSILON,
            "{context}: actual {actual:?} expected {expected:?}"
        );
    }

    fn assert_plan_eq(actual: &DirectorPlan, expected: &DirectorPlan, context: &str) {
        assert_eq!(
            actual.decisions, expected.decisions,
            "{context}: decisions differ"
        );
        assert_eq!(
            actual.slots.len(),
            expected.slots.len(),
            "{context}: slots len"
        );
        for (idx, (actual_slot, expected_slot)) in
            actual.slots.iter().zip(expected.slots.iter()).enumerate()
        {
            assert_eq!(
                actual_slot.goal, expected_slot.goal,
                "{context}: slot {idx} goal"
            );
            assert_eq!(
                actual_slot.count, expected_slot.count,
                "{context}: slot {idx} count"
            );
            assert_float_eq(
                actual_slot.score,
                expected_slot.score,
                &format!("{context}: slot {idx} score"),
            );
        }
    }

    #[test]
    fn fixture_is_generated_from_leader_ts_sources() {
        let fixture = fixture();

        assert_eq!(
            fixture.source,
            ["lib/game/leaderDirector.ts", "lib/game/leaderAI.ts"]
        );
        assert_eq!(
            fixture.counts.response_curves,
            fixture.response_curves.len()
        );
        assert_eq!(
            fixture.counts.target_warriors,
            fixture.target_warriors.len()
        );
        assert_eq!(fixture.counts.direct_colony, fixture.direct_colony.len());
        assert_eq!(
            fixture.counts.assignment_fits,
            fixture.assignment_fits.len()
        );
        assert_eq!(
            fixture.counts.match_cats_to_slots,
            fixture.match_cats_to_slots.len()
        );
    }

    #[test]
    fn response_curves_match_ts_vectors() {
        for case in fixture().response_curves {
            let actual = match case.expression.as_str() {
                "clamp01(-2)" => clamp01(-2.0),
                "clamp01(0.5)" => clamp01(0.5),
                "clamp01(2)" => clamp01(2.0),
                "deficitCurve(1)" => deficit_curve(1.0),
                "deficitCurve(1.5)" => deficit_curve(1.5),
                "deficitCurve(0)" => deficit_curve(0.0),
                "deficitCurve(0.5)" => deficit_curve(0.5),
                "deficitCurve(0.25)" => deficit_curve(0.25),
                "projectionCurve(100, 0)" => projection_curve(100.0, 0.0, PROJECTION_HORIZON_TICKS),
                "projectionCurve(100, -5)" => {
                    projection_curve(100.0, -5.0, PROJECTION_HORIZON_TICKS)
                }
                "projectionCurve(10, 10)" => projection_curve(10.0, 10.0, PROJECTION_HORIZON_TICKS),
                "projectionCurve(600, 10)" => {
                    projection_curve(600.0, 10.0, PROJECTION_HORIZON_TICKS)
                }
                "projectionCurve(-5, 10)" => projection_curve(-5.0, 10.0, PROJECTION_HORIZON_TICKS),
                "projectionCurve(10, 10, 0)" => projection_curve(10.0, 10.0, 0.0),
                "projectionGate(1)" => projection_gate(1.0),
                "projectionGate(0.9)" => projection_gate(0.9),
                "projectionGate(0)" => projection_gate(0.0),
                "projectionGate(0.45)" => projection_gate(0.45),
                "survivalScore(1, 200, 9999)" => survival_score(1.0, 200.0, 9999.0),
                "survivalScore(0.3, 60, 40)" => survival_score(0.3, 60.0, 40.0),
                "pressureCurve(0.8)" => pressure_curve(0.8, DEN_PRESSURE_THRESHOLD, 10.0),
                "pressureCurve(0.4)" => pressure_curve(0.4, DEN_PRESSURE_THRESHOLD, 10.0),
                "pressureCurve(1.2)" => pressure_curve(1.2, DEN_PRESSURE_THRESHOLD, 10.0),
                "surplusCurve(0.5, 0.6)" => surplus_curve(0.5, 0.6),
                "surplusCurve(0.6, 0.6)" => surplus_curve(0.6, 0.6),
                "surplusCurve(1, 0.6)" => surplus_curve(1.0, 0.6),
                "surplusCurve(0.8, 0.6)" => surplus_curve(0.8, 0.6),
                "combineOr(0, 0)" => combine_or(0.0, 0.0),
                "combineOr(1, 0)" => combine_or(1.0, 0.0),
                "combineOr(0, 1)" => combine_or(0.0, 1.0),
                "combineOr(0.5, 0.5)" => combine_or(0.5, 0.5),
                other => panic!("unhandled response curve expression {other}"),
            };

            assert_float_eq(actual, case.value, &case.expression);
        }
    }

    #[test]
    fn target_warriors_matches_ts_fixture() {
        for case in fixture().target_warriors {
            assert_eq!(
                target_warriors(&case.snapshot),
                case.expected,
                "{}",
                case.name
            );
        }
    }

    #[test]
    fn direct_colony_matches_ts_fixture() {
        for case in fixture().direct_colony {
            let actual = direct_colony(&case.snapshot);
            assert_plan_eq(&actual, &case.expected, &case.name);
        }
    }

    #[test]
    fn direct_colony_is_deterministic() {
        for case in fixture().direct_colony {
            assert_plan_eq(
                &direct_colony(&case.snapshot),
                &direct_colony(&case.snapshot),
                &case.name,
            );
        }
    }

    #[test]
    fn plan_leader_actions_flattens_director_plan_like_ts() {
        let case = fixture().plan_leader_actions;

        assert_plan_eq(&direct_colony(&case.snapshot), &case.direct, "direct plan");
        assert_eq!(plan_leader_actions(&case.snapshot), case.expected);
    }

    #[test]
    fn assignment_fits_match_ts_fixture() {
        for case in fixture().assignment_fits {
            assert_float_eq(
                assignment_fit(&case.cat, case.goal),
                case.expected,
                &case.name,
            );
        }
    }

    #[test]
    fn match_cats_to_slots_matches_ts_fixture() {
        for case in fixture().match_cats_to_slots {
            assert_eq!(
                match_cats_to_slots(&case.slots, &case.cats, case.options.into()),
                case.expected,
                "{}",
                case.name
            );
        }
    }
}
