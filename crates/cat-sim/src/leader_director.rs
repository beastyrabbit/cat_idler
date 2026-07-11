//! Utility leader director ported from `lib/game/leaderDirector.ts` and
//! `lib/game/leaderAI.ts`.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    leader_ai::{LeaderDecision, LeaderSnapshot, ThreatBand},
    officers::OfficerRole,
    types::CatSpecialization,
};

pub const EMPLOYMENT_TARGET_RATIO: f64 = 0.7;
/// Fraction of work-capable cats the director tops labour up to in `direct_colony`'s
/// fill pass (Hunt/Scout/Quarry). Job saturation tuning: raised 0.8 → 0.95 so a healthy
/// ~20-cat colony leaves at most ~1 idle cat instead of ~4. Standing hunt/scout/quarry
/// fill bypasses per-goal caps, so this floor is the binding employment lever for a
/// resource-comfortable colony; survival stays intact because water/food goals are still
/// ranked and granted first in the score pass, and the fill pass only adds food-producing
/// Hunt while food_r < 1. This deliberately diverges from the legacy TS floor (0.8); the
/// p3 director fixture was updated to the new deterministic counts to match.
pub const IDLE_EMPLOYMENT_FLOOR: f64 = 0.95;
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
/// Fill ratio below which a wild-findable resource (materials/food) counts as
/// "short" and starts pulling extra scouts out to discover new resource/hunt tiles.
/// At or above it the store is comfortable and contributes no scouting demand, so a
/// well-stocked colony fields fewer scouts than a resource-starved one.
pub const SCOUT_COMFORT_RATIO: f64 = 0.5;
/// How much a full wild-resource deficit lifts the scout goal's score above its
/// [`SCOUT_BASE_SCORE`] idle baseline. Capped (0.3 → at most 0.8) so a genuine
/// food/water survival crisis — whose survival curves reach 1.0 — always still
/// out-ranks scouting, keeping the food/water loop staffed first.
pub const SCOUT_DEFICIT_SCORE_WEIGHT: f64 = 0.5;
/// Extra scout slots a full wild-resource deficit unlocks on top of
/// [`SCOUT_MAX_SLOTS`], letting a short colony field more scouts than a stocked one.
pub const SCOUT_DEFICIT_EXTRA_SLOTS: u32 = 2;
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

/// The officer role that governs a labor goal (P12.2). Total over every
/// [`LaborGoalKind`] so a filled role covers a well-defined slice of the director's
/// goals; an unfilled role simply has no officer to act.
#[must_use]
pub fn officer_role_for(kind: LaborGoalKind) -> OfficerRole {
    match kind {
        LaborGoalKind::Hunt | LaborGoalKind::FetchWater => OfficerRole::Farmer,
        LaborGoalKind::Quarry => OfficerRole::Forester,
        LaborGoalKind::TrainWarrior | LaborGoalKind::AssignSmithy => OfficerRole::Captain,
        LaborGoalKind::AssignResearch | LaborGoalKind::Scout => OfficerRole::Loremaster,
        LaborGoalKind::AssignWorkshop => OfficerRole::Steward,
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
    /// Player-set priority flag (P15 "cat booster"), mirrors `entities::Cat::boosted`.
    /// Absent in older fixtures/callers → `false`, matching the field's own
    /// `#[serde(default)]` on the `Cat` entity.
    #[serde(default)]
    pub boosted: bool,
}

/// Multiplicative fit bonus for a boosted cat, applied alongside (and independent of)
/// the `SPECIALIZATION_FIT_MULTIPLIER`. Chosen slightly above the specialization bonus
/// so a boosted cat can win a marginally-better unboosted rival — including tipping a
/// close specialization matchup — while still being a *multiplicative* scale on the
/// cat's own base fit rather than a flat additive bump. That keeps the bonus bounded:
/// a boosted cat with near-zero base fit for a goal still scores near zero (1.6x of
/// ~0 is ~0), so it can never displace a genuinely strong specialist from a slot it's
/// useless for. Stacks with specialization (a boosted specialist gets both), which is
/// intentional — boosting a cat already suited for a role should make it an even
/// stronger pick.
const BOOST_FIT_MULTIPLIER: f64 = 1.6;
const SPECIALIZATION_FIT_MULTIPLIER: f64 = 1.5;

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

    let spec_multiplier = if spec_match {
        SPECIALIZATION_FIT_MULTIPLIER
    } else {
        1.0
    };
    let boost_multiplier = if cat.boosted {
        BOOST_FIT_MULTIPLIER
    } else {
        1.0
    };

    base * spec_multiplier * boost_multiplier
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
        if let Some(idx) = best_fit_index(&pool, goal, options) {
            assignments.push(Assignment {
                cat_id: pool[idx].id.clone(),
                goal,
            });
            pool.remove(idx);
        }
    }

    assignments
}

/// Index of the best-fit cat in `pool` for `goal` (the greedy pick used by
/// [`match_cats_to_slots`]), or `None` if no eligible cat remains.
fn best_fit_index(pool: &[CatBrief], goal: LaborGoalKind, options: MatchOptions) -> Option<usize> {
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

    best_idx
}

/// Officer-aware matching (P12.2). Identical to [`match_cats_to_slots`] when
/// `officers` is empty (it delegates), so it is a strict superset with zero effect
/// on the empty case. When a slot's governing role ([`officer_role_for`]) is filled
/// and that officer cat is still in the pool and eligible, the officer takes the slot
/// FIRST — reliably working its domain — before the greedy best-fit fallback.
#[must_use]
pub fn match_cats_to_slots_with_officers(
    slots: &[OpenSlots],
    cats: &[CatBrief],
    officers: &BTreeMap<OfficerRole, String>,
    options: MatchOptions,
) -> Vec<Assignment> {
    if officers.is_empty() {
        return match_cats_to_slots(slots, cats, options);
    }

    let mut flat = Vec::new();
    for slot in slots {
        for _ in 0..slot.count {
            flat.push(slot.goal);
        }
    }

    let mut pool = cats.to_vec();
    let mut assignments = Vec::new();
    for goal in flat {
        let officer_idx = officers.get(&officer_role_for(goal)).and_then(|cat_id| {
            pool.iter().position(|cat| {
                &cat.id == cat_id
                    && !(goal == LaborGoalKind::TrainWarrior
                        && options.exclude_warriors_from_training
                        && cat.specialization == Some(CatSpecialization::Warrior))
            })
        });

        if let Some(idx) = officer_idx.or_else(|| best_fit_index(&pool, goal, options)) {
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

    // ADDITIVE officer effect: filled roles each get a small, capped slot bonus in
    // their category. No-op (byte-identical) when officers is empty.
    if !snapshot.officers.is_empty() {
        apply_officer_slot_bonus(snapshot, &goals, &mut slots);
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

/// Grant each filled officer role +1 slot in its category's highest-ranked already-open
/// slot, bounded by (a) idle cats not yet spent and (b) the goal's own hard-cap
/// headroom. Only boosts categories that already have an open slot — a filled role
/// never conjures work from nothing — keeping the effect small and no-op when empty.
fn apply_officer_slot_bonus(
    snapshot: &LeaderSnapshot,
    goals: &[LaborGoal],
    slots: &mut [OpenSlots],
) {
    let granted_total: u32 = slots.iter().map(|slot| slot.count).sum();
    let mut budget = snapshot.idle_cats.saturating_sub(granted_total);

    for role in OfficerRole::ALL {
        if budget == 0 {
            break;
        }
        if !snapshot.officers.contains_key(role) {
            continue;
        }
        // `slots` is in ranked order, so the first match is the role's top open goal.
        let Some(slot) = slots
            .iter_mut()
            .find(|slot| officer_role_for(slot.goal) == *role)
        else {
            continue;
        };
        let Some(goal) = goals.iter().find(|goal| goal.kind == slot.goal) else {
            continue;
        };
        let headroom = goal
            .hard_cap
            .saturating_sub(goal.in_flight.saturating_add(slot.count));
        if headroom == 0 {
            continue;
        }
        slot.count += 1;
        budget -= 1;
    }
}

/// Re-scale a store's shortfall within the comfort band and reuse the director's
/// quadratic [`deficit_curve`], so scouting shares the same response shape as the
/// hunt/water/quarry goals. Zero at/above [`SCOUT_COMFORT_RATIO`]; approaches 1 as
/// the store empties.
#[must_use]
fn deficit_below_comfort(fill_ratio: f64) -> f64 {
    if fill_ratio >= SCOUT_COMFORT_RATIO {
        0.0
    } else {
        deficit_curve(fill_ratio / SCOUT_COMFORT_RATIO)
    }
}

/// Deficit signal in [0,1] for the resources a scout can find in the wild —
/// materials (new quarry/forage tiles) and food (new hunt tiles). Zero when both
/// stores are comfortable or when there is no frontier left to map, so a stocked or
/// fully-explored colony gets no scouting boost; rises toward 1 as a wild-findable
/// store runs short, driving the leader to dispatch more scouts.
#[must_use]
pub fn scout_wild_deficit(snapshot: &LeaderSnapshot) -> f64 {
    if !snapshot.has_frontier {
        return 0.0;
    }
    let materials_r = ratio(snapshot.materials, snapshot.materials_capacity);
    let food_r = ratio(snapshot.resources.food, snapshot.food_capacity);
    deficit_below_comfort(materials_r).max(deficit_below_comfort(food_r))
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

    // Deficit-driven scouting: a wild-resource shortfall raises both the scout goal's
    // priority and how many scouts the leader will field. At zero deficit this is
    // exactly the historical (SCOUT_BASE_SCORE, SCOUT_MAX_SLOTS), so a comfortable or
    // fully-mapped colony is byte-identical to before.
    let scout_deficit = scout_wild_deficit(snapshot);
    let scout_score = clamp01(SCOUT_BASE_SCORE + SCOUT_DEFICIT_SCORE_WEIGHT * scout_deficit);
    let scout_slots =
        SCOUT_MAX_SLOTS + (f64::from(SCOUT_DEFICIT_EXTRA_SLOTS) * scout_deficit).round() as u32;

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
            score: scout_score,
            max_slots: scout_slots,
            in_flight: snapshot.active_scouts,
            hard_cap: scout_slots,
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
    use crate::leader_ai::{LeaderHousing, LeaderResources};

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

    // ---- P15 cat booster (leader director matcher bonus) ----
    //
    // `assignment_fits_match_ts_fixture` and `match_cats_to_slots_matches_ts_fixture`
    // (above) already double as the "unboosted matching is byte-identical to
    // pre-change" guardrail: every fixture `CatBrief` deserializes with
    // `boosted: false` (the field is `#[serde(default)]` and the TS-derived JSON
    // never carries it), and their `expected` values were computed under the
    // pre-boost formula — so those two tests only stay green if `boost_multiplier`
    // is an exact 1.0 (i.e. a true no-op) for every unboosted cat in the fixture.

    #[test]
    fn boosted_cat_wins_an_equal_fit_tie() {
        let slots = vec![OpenSlots {
            goal: LaborGoalKind::Hunt,
            count: 1,
            score: 0.5,
        }];
        let cats = vec![
            cat_brief("plain", 10.0, 10.0),
            boosted(cat_brief("boosted", 10.0, 10.0)),
        ];

        let assignments = match_cats_to_slots(&slots, &cats, MatchOptions::default());
        assert_eq!(
            assignments,
            vec![Assignment {
                cat_id: "boosted".to_owned(),
                goal: LaborGoalKind::Hunt,
            }]
        );
    }

    #[test]
    fn boosted_cat_wins_over_a_marginally_better_unboosted_cat() {
        // "better" unboosted cat has higher raw hunting (12 vs 10), but the boosted
        // cat's 1.6x multiplier (16.0) clears the unboosted cat's raw fit (12.0) —
        // within the bonus margin.
        let slots = vec![OpenSlots {
            goal: LaborGoalKind::Hunt,
            count: 1,
            score: 0.5,
        }];
        let cats = vec![
            cat_brief("better-unboosted", 10.0, 12.0),
            boosted(cat_brief("boosted", 10.0, 10.0)),
        ];

        let assignments = match_cats_to_slots(&slots, &cats, MatchOptions::default());
        assert_eq!(
            assignments,
            vec![Assignment {
                cat_id: "boosted".to_owned(),
                goal: LaborGoalKind::Hunt,
            }]
        );
    }

    #[test]
    fn boosted_hopelessly_unfit_cat_does_not_displace_a_strong_specialist() {
        // A Hunter specialist with strong hunting stats (fit = 20 * 1.5 = 30) must keep
        // the slot over a boosted cat with almost no hunting skill (fit = 1 * 1.6 =
        // 1.6) — the multiplicative bonus scales the boosted cat's own (near-zero)
        // base fit, so it never manufactures a competitive score out of nothing.
        let specialist = CatBrief {
            id: "specialist".to_owned(),
            specialization: Some(CatSpecialization::Hunter),
            stats: CatBriefStats {
                hunting: 20.0,
                building: 10.0,
                vision: 10.0,
                medicine: 10.0,
                attack: 10.0,
                defense: 10.0,
                leadership: 10.0,
            },
            boosted: false,
        };
        let boosted_but_useless = CatBrief {
            id: "boosted-unfit".to_owned(),
            specialization: None,
            stats: CatBriefStats {
                hunting: 1.0,
                building: 10.0,
                vision: 10.0,
                medicine: 10.0,
                attack: 10.0,
                defense: 10.0,
                leadership: 10.0,
            },
            boosted: true,
        };

        let slots = vec![OpenSlots {
            goal: LaborGoalKind::Hunt,
            count: 1,
            score: 0.5,
        }];
        let cats = vec![specialist, boosted_but_useless];

        let assignments = match_cats_to_slots(&slots, &cats, MatchOptions::default());
        assert_eq!(
            assignments,
            vec![Assignment {
                cat_id: "specialist".to_owned(),
                goal: LaborGoalKind::Hunt,
            }]
        );
    }

    #[test]
    fn assignment_fit_boost_multiplier_is_a_strict_no_op_when_unboosted() {
        // Direct arithmetic check, independent of the fixture: an unboosted cat's fit
        // is exactly base * spec_multiplier (boost contributes a literal 1.0 factor,
        // which is exact under IEEE754 multiplication — no drift).
        let cat = cat_brief("plain", 10.0, 12.0);
        assert!(!cat.boosted);
        assert_eq!(assignment_fit(&cat, LaborGoalKind::Hunt), 12.0);

        let boosted_cat = boosted(cat_brief("boosted", 10.0, 12.0));
        assert_eq!(
            assignment_fit(&boosted_cat, LaborGoalKind::Hunt),
            12.0 * BOOST_FIT_MULTIPLIER
        );
    }

    // ---- P12.2 officers (additive layer) ----

    fn cat_brief(id: &str, attack: f64, hunting: f64) -> CatBrief {
        CatBrief {
            id: id.to_owned(),
            specialization: None,
            stats: CatBriefStats {
                hunting,
                building: 10.0,
                vision: 10.0,
                medicine: 10.0,
                attack,
                defense: 10.0,
                leadership: 10.0,
            },
            boosted: false,
        }
    }

    fn boosted(mut cat: CatBrief) -> CatBrief {
        cat.boosted = true;
        cat
    }

    #[test]
    fn empty_officers_leave_direct_colony_byte_identical() {
        // Every TS-derived fixture carries no officers; clearing the (empty) map must
        // not shift the plan — the officer layer is inert until a role is filled.
        for case in fixture().direct_colony {
            let mut cleared = case.snapshot.clone();
            cleared.officers.clear();
            assert_plan_eq(&direct_colony(&cleared), &case.expected, &case.name);
        }
    }

    #[test]
    fn officer_matcher_with_empty_officers_equals_base_matcher() {
        for case in fixture().match_cats_to_slots {
            let officers = BTreeMap::new();
            let options: MatchOptions = case.options.into();
            assert_eq!(
                match_cats_to_slots_with_officers(&case.slots, &case.cats, &officers, options),
                match_cats_to_slots(&case.slots, &case.cats, options),
                "{}",
                case.name
            );
        }
    }

    #[test]
    fn officer_role_covers_every_labor_goal() {
        use LaborGoalKind::*;
        assert_eq!(officer_role_for(Hunt), OfficerRole::Farmer);
        assert_eq!(officer_role_for(FetchWater), OfficerRole::Farmer);
        assert_eq!(officer_role_for(Quarry), OfficerRole::Forester);
        assert_eq!(officer_role_for(Scout), OfficerRole::Loremaster);
        assert_eq!(officer_role_for(TrainWarrior), OfficerRole::Captain);
        assert_eq!(officer_role_for(AssignWorkshop), OfficerRole::Steward);
        assert_eq!(officer_role_for(AssignResearch), OfficerRole::Loremaster);
        assert_eq!(officer_role_for(AssignSmithy), OfficerRole::Captain);
    }

    #[test]
    fn filled_captain_takes_its_military_slot_before_a_better_fit() {
        let slots = vec![OpenSlots {
            goal: LaborGoalKind::TrainWarrior,
            count: 1,
            score: 0.5,
        }];
        let cats = vec![
            cat_brief("strong", 90.0, 10.0),
            cat_brief("cap", 10.0, 10.0),
        ];

        // Without officers the greedy best-fit (strong) wins.
        let base = match_cats_to_slots_with_officers(
            &slots,
            &cats,
            &BTreeMap::new(),
            MatchOptions::default(),
        );
        assert_eq!(base[0].cat_id, "strong");

        // Captain officer "cap" works its domain first despite the worse fit.
        let mut officers = BTreeMap::new();
        officers.insert(OfficerRole::Captain, "cap".to_owned());
        let out =
            match_cats_to_slots_with_officers(&slots, &cats, &officers, MatchOptions::default());
        assert_eq!(
            out,
            vec![Assignment {
                cat_id: "cap".to_owned(),
                goal: LaborGoalKind::TrainWarrior,
            }]
        );
    }

    #[test]
    fn filled_officer_adds_one_capped_slot_to_its_open_category() {
        let mut snapshot = fixture().direct_colony[0].snapshot.clone();
        snapshot.idle_cats = 5;
        snapshot.officers.clear();
        snapshot
            .officers
            .insert(OfficerRole::Captain, "cap".to_owned());

        let goals = vec![LaborGoal {
            kind: LaborGoalKind::TrainWarrior,
            score: 0.5,
            max_slots: 4,
            in_flight: 0,
            hard_cap: 4,
            vetoed: false,
            mode: LaborGoalMode::Fixed,
        }];
        let mut slots = vec![OpenSlots {
            goal: LaborGoalKind::TrainWarrior,
            count: 1,
            score: 0.5,
        }];
        apply_officer_slot_bonus(&snapshot, &goals, &mut slots);
        assert_eq!(slots[0].count, 2, "Captain grants +1 military slot");

        // Idle budget caps the bonus: with no spare idle cats, no boost.
        snapshot.idle_cats = 1;
        let mut tight = vec![OpenSlots {
            goal: LaborGoalKind::TrainWarrior,
            count: 1,
            score: 0.5,
        }];
        apply_officer_slot_bonus(&snapshot, &goals, &mut tight);
        assert_eq!(tight[0].count, 1, "no idle budget → no bonus");

        // Hard-cap headroom caps the bonus too.
        snapshot.idle_cats = 5;
        let capped_goals = vec![LaborGoal {
            hard_cap: 1,
            ..goals[0]
        }];
        let mut capped = vec![OpenSlots {
            goal: LaborGoalKind::TrainWarrior,
            count: 1,
            score: 0.5,
        }];
        apply_officer_slot_bonus(&snapshot, &capped_goals, &mut capped);
        assert_eq!(capped[0].count, 1, "no hard-cap headroom → no bonus");
    }

    // ---- Job saturation (IDLE_EMPLOYMENT_FLOOR raised 0.8 -> 0.95) ----

    /// A resource-comfortable snapshot with a frontier + quarry to absorb fill labour.
    fn healthy_snapshot(idle: u32, employed: u32) -> LeaderSnapshot {
        let mut snapshot = fixture()
            .direct_colony
            .into_iter()
            .find(|case| case.name == "round_robin_idle_floor")
            .expect("round_robin fixture present")
            .snapshot;
        snapshot.idle_cats = idle;
        snapshot.employed_cats = employed;
        snapshot.workforce = Some(f64::from(idle + employed));
        snapshot.population = idle + employed;
        snapshot
    }

    fn total_slots(plan: &DirectorPlan) -> u32 {
        plan.slots.iter().map(|slot| slot.count).sum()
    }

    #[test]
    fn idle_employment_floor_is_ninety_five_percent() {
        assert!((IDLE_EMPLOYMENT_FLOOR - 0.95).abs() < 1e-12);
    }

    #[test]
    fn healthy_twenty_cat_colony_leaves_at_most_one_idle() {
        // 20 work-capable cats, all idle, resources comfortable: the fill pass should
        // saturate labour so at most one cat stands idle (ceil(20 * 0.95) = 19 employed).
        let snapshot = healthy_snapshot(20, 0);
        let plan = direct_colony(&snapshot);
        let employed = total_slots(&plan);
        assert_eq!(employed, 19, "expected 19 of 20 employed, got {employed}");
        assert!(20 - employed <= 1, "idle count must be <= 1");
    }

    #[test]
    fn healthy_colony_saturation_is_deterministic() {
        let snapshot = healthy_snapshot(20, 0);
        assert_plan_eq(
            &direct_colony(&snapshot),
            &direct_colony(&snapshot),
            "healthy saturation",
        );
    }

    #[test]
    fn saturation_respects_already_employed_cats() {
        // With most cats already busy, the fill pass tops up only the remaining idle
        // cats toward the floor rather than over-committing beyond the workforce.
        let snapshot = healthy_snapshot(4, 16);
        let plan = direct_colony(&snapshot);
        let newly_employed = total_slots(&plan);
        assert!(
            newly_employed <= 4,
            "must not open more slots than idle cats ({newly_employed} > 4)"
        );
        // Floor target is ceil(20 * 0.95) = 19; 16 already busy -> up to 3 more opened.
        assert!(newly_employed >= 3, "should top up toward the floor");
    }

    #[test]
    fn starving_colony_pours_fill_labour_into_food_not_scouting() {
        // A starving colony must still prioritise food: the fill floor sends the extra
        // idle cats to Hunt (food) and never staffs workshop/research/smithy/training.
        let mut snapshot = healthy_snapshot(20, 0);
        snapshot.starving = Some(true);
        snapshot.resources.food = 0.0;
        snapshot.food_drain_per_tick = Some(10.0);
        snapshot.has_frontier = true;
        snapshot.has_quarry_site = true;
        snapshot.workshops_needing_workers = 3;
        snapshot.research_huts_needing_workers = Some(2);
        snapshot.smithies_needing_workers = Some(2);
        snapshot.has_barracks = Some(true);

        let plan = direct_colony(&snapshot);
        let hunt = plan
            .slots
            .iter()
            .find(|slot| slot.goal == LaborGoalKind::Hunt)
            .map_or(0, |slot| slot.count);
        let scout = plan
            .slots
            .iter()
            .find(|slot| slot.goal == LaborGoalKind::Scout)
            .map_or(0, |slot| slot.count);
        assert!(
            hunt >= scout,
            "food work must dominate: hunt {hunt} < scout {scout}"
        );
        for banned in [
            LaborGoalKind::AssignWorkshop,
            LaborGoalKind::AssignResearch,
            LaborGoalKind::AssignSmithy,
            LaborGoalKind::TrainWarrior,
        ] {
            assert!(
                plan.slots.iter().all(|slot| slot.goal != banned),
                "starving colony must not open {banned:?} slots"
            );
        }
    }

    /// Base for the deficit-driven scout scenarios: a frontier colony with a limited
    /// idle pool, full food/water (so hunt/water never compete) and a lone competing
    /// comfort goal (unstaffed workshops). Only the materials level is left to vary.
    fn scout_scenario(idle: u32) -> LeaderSnapshot {
        let mut snapshot = healthy_snapshot(idle, 0);
        snapshot.has_frontier = true;
        snapshot.has_water_site = false;
        snapshot.has_quarry_site = false;
        snapshot.workshops_needing_workers = 2;
        snapshot.research_huts_needing_workers = Some(0);
        snapshot.smithies_needing_workers = Some(0);
        snapshot.has_barracks = Some(false);
        snapshot.warrior_count = Some(0);
        snapshot.training_in_flight = Some(0);
        snapshot.water = snapshot.water_capacity;
        snapshot.resources.food = snapshot.food_capacity;
        snapshot
    }

    fn scout_slots(snapshot: &LeaderSnapshot) -> u32 {
        direct_colony(snapshot)
            .slots
            .iter()
            .find(|slot| slot.goal == LaborGoalKind::Scout)
            .map_or(0, |slot| slot.count)
    }

    fn scout_slot_score(snapshot: &LeaderSnapshot) -> f64 {
        direct_colony(snapshot)
            .slots
            .iter()
            .find(|slot| slot.goal == LaborGoalKind::Scout)
            .map_or(0.0, |slot| slot.score)
    }

    #[test]
    fn scout_demand_is_zero_when_stocked_or_fully_mapped() {
        let mut stocked = scout_scenario(6);
        stocked.materials = stocked.materials_capacity;
        assert_eq!(
            scout_wild_deficit(&stocked),
            0.0,
            "a comfortable colony must generate no scouting demand"
        );

        let mut mapped = scout_scenario(6);
        mapped.materials = 0.0;
        mapped.has_frontier = false;
        assert_eq!(
            scout_wild_deficit(&mapped),
            0.0,
            "a fully-mapped colony must generate no scouting demand"
        );
    }

    #[test]
    fn scout_demand_rises_as_a_wild_resource_runs_short() {
        let mut colony = scout_scenario(6);
        colony.materials = 0.0;
        let empty = scout_wild_deficit(&colony);
        assert!(
            empty > 0.9,
            "an empty materials store should drive strong scout demand, got {empty}"
        );

        colony.materials = colony.materials_capacity * SCOUT_COMFORT_RATIO * 0.5;
        let partial = scout_wild_deficit(&colony);
        assert!(
            partial > 0.0 && partial < empty,
            "a partial shortfall sits between comfort and empty, got {partial}"
        );
    }

    #[test]
    fn deficit_driven_short_colony_dispatches_more_scouts_than_a_stocked_one() {
        // Both colonies share the same frontier, idle pool and competing workshop; only
        // the materials store differs. The short colony's boosted scout score wins the
        // scarce labour, so it fields strictly more scouts at a strictly higher priority.
        let stocked = {
            let mut s = scout_scenario(4);
            s.materials = s.materials_capacity;
            s
        };
        let short = {
            let mut s = scout_scenario(4);
            s.materials = 0.0;
            s
        };

        assert!(
            scout_slot_score(&short) > scout_slot_score(&stocked),
            "short colony must value scouting more (short {}, stocked {})",
            scout_slot_score(&short),
            scout_slot_score(&stocked)
        );
        assert!(
            scout_slots(&short) > scout_slots(&stocked),
            "short colony must field more scouts (short {}, stocked {})",
            scout_slots(&short),
            scout_slots(&stocked)
        );
    }

    #[test]
    fn deficit_driven_scouting_is_deterministic() {
        let mut short = scout_scenario(4);
        short.materials = 0.0;
        assert_plan_eq(
            &direct_colony(&short),
            &direct_colony(&short),
            "deficit scouting",
        );
    }

    // ---- P16.x founding craft-bench staffing (AssignWorkshop construction demand) ----

    /// A fresh 5-cat founding colony mirroring `world_tick`'s `STARTER_BLUEPRINT`:
    /// comfortable food/water and three unstaffed P16 raw-material craft benches folded
    /// into `workshops_needing_workers` (per
    /// `world_tick::phase_18_leader_snapshot_assembly`).
    fn founding_five_cat_snapshot() -> LeaderSnapshot {
        LeaderSnapshot {
            population: 5,
            workforce: Some(5.0),
            idle_cats: 5,
            employed_cats: 0,
            resources: LeaderResources {
                food: 120.0,
                refined: 0.0,
            },
            food_capacity: 200.0,
            food_drain_per_tick: Some(0.5),
            materials: 40.0,
            materials_capacity: 100.0,
            water: 150.0,
            water_capacity: 200.0,
            water_drain_per_tick: Some(0.3),
            housing: LeaderHousing {
                capacity: 10,
                committed: 0,
            },
            active_hunts: 0,
            active_quarries: 0,
            active_scouts: 0,
            active_water_fetchers: 0,
            has_quarry_site: true,
            has_water_site: true,
            has_frontier: true,
            den_plans_in_flight: 0,
            storage_plans_in_flight: 0,
            storehouse_count: 0,
            storehouse_cap: 3,
            workshops_needing_workers: 3,
            research_huts_needing_workers: Some(0),
            smithies_needing_workers: Some(0),
            has_barracks: Some(false),
            warrior_count: Some(0),
            training_in_flight: Some(0),
            threat_band: Some(ThreatBand::Calm),
            starving: Some(false),
            officers: BTreeMap::new(),
        }
    }

    fn assign_workshop_slot_count(snapshot: &LeaderSnapshot) -> u32 {
        direct_colony(snapshot)
            .slots
            .iter()
            .find(|slot| slot.goal == LaborGoalKind::AssignWorkshop)
            .map_or(0, |slot| slot.count)
    }

    #[test]
    fn founding_colony_staffs_at_least_one_craft_bench() {
        // Regression test for the P16 founding stall: a fresh 5-cat colony with three
        // unstaffed craft benches (`workshops_needing_workers: 3`, mirroring what
        // `world_tick`'s snapshot builder now reports once it folds the raw-material
        // benches in) must claim at least one idle cat for AssignWorkshop instead of
        // pouring every idle cat into Hunt/Scout/Quarry and leaving the benches — and
        // therefore planks/blocks production — stalled forever. Before the fix this goal
        // was hard-*vetoed* at `workshops_needing_workers == 0` because the snapshot
        // builder only ever counted the (founding-absent) general Workshop building;
        // this proves the ranked loop reliably grants it a slot once that count is
        // fixed, at the flat historical score.
        let snapshot = founding_five_cat_snapshot();
        let count = assign_workshop_slot_count(&snapshot);
        assert!(
            count >= 1,
            "expected at least one AssignWorkshop slot on a fresh founding colony, got {count}"
        );
    }

    #[test]
    fn founding_colony_craft_bench_staffing_is_deterministic() {
        let snapshot = founding_five_cat_snapshot();
        assert_plan_eq(
            &direct_colony(&snapshot),
            &direct_colony(&snapshot),
            "founding craft bench staffing",
        );
    }

    #[test]
    fn craft_bench_demand_never_outranks_a_genuine_survival_crisis() {
        // Guardrail: construction demand may win idle labour when the colony is
        // comfortable (proven above), but a genuine food/water crisis must still claim
        // every idle cat first — AssignWorkshop must not open a single slot.

        // (a) The existing `starving` veto (food < 15% fill, world_tick's definition of
        // a survival crisis) still hard-vetoes AssignWorkshop outright.
        let mut starving = founding_five_cat_snapshot();
        starving.resources.food = 0.0;
        starving.food_drain_per_tick = Some(10.0);
        starving.starving = Some(true);
        assert_eq!(
            assign_workshop_slot_count(&starving),
            0,
            "a starving colony must not staff craft benches"
        );

        // (b) Even without the starving flag, a simultaneous food+water crisis outranks
        // and out-competes AssignWorkshop for the (scarce) idle labour on pure score +
        // capacity grounds — hunt/water win every idle cat before workshop gets a turn.
        let mut crisis = founding_five_cat_snapshot();
        crisis.resources.food = 0.0;
        crisis.food_drain_per_tick = Some(10.0);
        crisis.water = 0.0;
        crisis.water_drain_per_tick = Some(10.0);
        assert_eq!(
            assign_workshop_slot_count(&crisis),
            0,
            "a food+water crisis must claim all idle labour before AssignWorkshop"
        );
    }

    #[test]
    fn craft_bench_staffing_does_not_permanently_starve_scouting() {
        // Regression guard for a real bug caught while building this fix: an earlier
        // draft gave AssignWorkshop a deficit-driven score boost (rising toward 1.0 as
        // planks/blocks ran short). Because AssignWorkshop grants in `Fixed` mode (it
        // always wants all `workshops_needing_workers` benches at once, not a
        // score-scaled slice), that boost let it consistently outrank and out-compete
        // every other early-game goal on the very first tick, and because craft-bench
        // assignment is sticky (never re-contested once granted, unlike a job), it could
        // permanently claim 3 of a 5-cat colony's workers, leaving zero labour for
        // Scout ever again. `world_tick::tests::
        // scouts_random_walk_outward_and_reveal_new_fog_deterministically` caught this
        // over a 200-tick founding run. The fix here is the flat STAFF_BASE_SCORE: it
        // still reliably bootstraps craft-bench staffing (proven above) without
        // monopolizing every idle cat, so Scout and Hunt still get a fair share. This
        // test asserts Scout still opens at least one slot on a comfortable founding
        // colony with unstaffed craft benches, guarding against a future score-driven
        // regression reintroducing the starvation.
        let snapshot = founding_five_cat_snapshot();
        let scout_count = direct_colony(&snapshot)
            .slots
            .iter()
            .find(|slot| slot.goal == LaborGoalKind::Scout)
            .map_or(0, |slot| slot.count);
        assert!(
            scout_count >= 1,
            "AssignWorkshop must not monopolize every idle cat away from Scout, got scout \
             count {scout_count}"
        );
    }

    #[test]
    fn direct_colony_is_deterministic_with_officers_filled() {
        let mut snapshot = fixture().direct_colony[0].snapshot.clone();
        snapshot
            .officers
            .insert(OfficerRole::Farmer, "f".to_owned());
        snapshot
            .officers
            .insert(OfficerRole::Captain, "c".to_owned());
        assert_plan_eq(
            &direct_colony(&snapshot),
            &direct_colony(&snapshot),
            "officers filled",
        );
    }
}
