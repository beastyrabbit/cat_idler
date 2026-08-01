//! Report-safe founding-Leader planning foundations specified by
//! `docs/leader-ai-overhaul/planner-and-beliefs.md`.

use std::{collections::BTreeSet, fmt};

use serde::{Deserialize, Serialize};

use crate::{
    authority::AuthorityDomain,
    beliefs::{EvidenceId, ReportId},
    cat_traits::{CatPersonality, PersonalityAxis, PersonalityPole},
    officer_requests::OfficerRequestBook,
    officers::OfficerRole,
    planner_core::{
        BASIS_POINTS_SCALE, BasisPoints, IntentScoreInputs, PlannerId, PlannerRngStream,
        PlannerScore, planner_roll, score_intent,
    },
};

#[path = "leader_content_planner.rs"]
pub mod content_planner;

pub const LEADER_PLANNER_SCHEMA_VERSION: u32 = 2;
pub const MAX_POSTURE_EVIDENCE_IDS: usize = 64;
pub const MAX_GOALS_PER_REVIEW: usize = 16;
pub const MAX_RATIONALE_KEYS_PER_GOAL: usize = 8;
pub const GAME_MINUTES_PER_DAY: u32 = 24 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaderPosture {
    Defend,
    Crisis,
    Recover,
    Establish,
    Stabilize,
    Grow,
    Prosper,
}

impl LeaderPosture {
    pub const PRECEDENCE: [Self; 7] = [
        Self::Defend,
        Self::Crisis,
        Self::Recover,
        Self::Establish,
        Self::Stabilize,
        Self::Grow,
        Self::Prosper,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreatBelief {
    Unknown,
    None,
    Credible,
    ActiveAttack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessBelief {
    Unknown,
    Accessible,
    Inaccessible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InjuryBelief {
    Unknown,
    None,
    Unresolved,
    DangerousUntreated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InfrastructureBelief {
    Unknown,
    Missing,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityConstraint {
    Population,
    Storage,
    Production,
    Territory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportSafePostureInputs {
    pub schema_version: u32,
    pub threat: ThreatBelief,
    pub essential_forecast_minutes: Option<u32>,
    pub food_access: AccessBelief,
    pub water_access: AccessBelief,
    pub injury: InjuryBelief,
    pub other_survival_failure: bool,
    pub emergency_recently_ended: bool,
    pub housing_damage_unresolved: bool,
    pub hole: InfrastructureBelief,
    pub food_infrastructure: InfrastructureBelief,
    pub water_infrastructure: InfrastructureBelief,
    pub storage: InfrastructureBelief,
    pub shelter: InfrastructureBelief,
    pub basic_production: InfrastructureBelief,
    pub bottleneck_present: bool,
    pub unstable_chain: bool,
    pub capacity_constraints: BTreeSet<CapacityConstraint>,
    pub forecast_stable: bool,
    pub evidence_ids: BTreeSet<EvidenceId>,
    pub report_ids: BTreeSet<ReportId>,
}

impl ReportSafePostureInputs {
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            schema_version: LEADER_PLANNER_SCHEMA_VERSION,
            threat: ThreatBelief::Unknown,
            essential_forecast_minutes: None,
            food_access: AccessBelief::Unknown,
            water_access: AccessBelief::Unknown,
            injury: InjuryBelief::Unknown,
            other_survival_failure: false,
            emergency_recently_ended: false,
            housing_damage_unresolved: false,
            hole: InfrastructureBelief::Unknown,
            food_infrastructure: InfrastructureBelief::Unknown,
            water_infrastructure: InfrastructureBelief::Unknown,
            storage: InfrastructureBelief::Unknown,
            shelter: InfrastructureBelief::Unknown,
            basic_production: InfrastructureBelief::Unknown,
            bottleneck_present: false,
            unstable_chain: false,
            capacity_constraints: BTreeSet::new(),
            forecast_stable: false,
            evidence_ids: BTreeSet::new(),
            report_ids: BTreeSet::new(),
        }
    }

    fn infrastructure(&self) -> [InfrastructureBelief; 6] {
        [
            self.hole,
            self.food_infrastructure,
            self.water_infrastructure,
            self.storage,
            self.shelter,
            self.basic_production,
        ]
    }

    fn validate(&self) -> Result<(), LeaderPlannerError> {
        if self.schema_version != LEADER_PLANNER_SCHEMA_VERSION
            || self.evidence_ids.len() > MAX_POSTURE_EVIDENCE_IDS
            || self.report_ids.len() > MAX_POSTURE_EVIDENCE_IDS
        {
            return Err(LeaderPlannerError::MalformedPersistence);
        }
        Ok(())
    }
}

impl Default for ReportSafePostureInputs {
    fn default() -> Self {
        Self::unknown()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedPostureInputs {
    schema_version: u32,
    threat: ThreatBelief,
    essential_forecast_minutes: Option<u32>,
    food_access: AccessBelief,
    water_access: AccessBelief,
    injury: InjuryBelief,
    other_survival_failure: bool,
    emergency_recently_ended: bool,
    housing_damage_unresolved: bool,
    hole: InfrastructureBelief,
    food_infrastructure: InfrastructureBelief,
    water_infrastructure: InfrastructureBelief,
    storage: InfrastructureBelief,
    shelter: InfrastructureBelief,
    basic_production: InfrastructureBelief,
    bottleneck_present: bool,
    unstable_chain: bool,
    #[serde(default)]
    capacity_constraints: BTreeSet<CapacityConstraint>,
    forecast_stable: bool,
    #[serde(default)]
    evidence_ids: BTreeSet<EvidenceId>,
    #[serde(default)]
    report_ids: BTreeSet<ReportId>,
}

impl<'de> Deserialize<'de> for ReportSafePostureInputs {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;

        let raw = UncheckedPostureInputs::deserialize(deserializer)?;
        let inputs = Self {
            schema_version: raw.schema_version,
            threat: raw.threat,
            essential_forecast_minutes: raw.essential_forecast_minutes,
            food_access: raw.food_access,
            water_access: raw.water_access,
            injury: raw.injury,
            other_survival_failure: raw.other_survival_failure,
            emergency_recently_ended: raw.emergency_recently_ended,
            housing_damage_unresolved: raw.housing_damage_unresolved,
            hole: raw.hole,
            food_infrastructure: raw.food_infrastructure,
            water_infrastructure: raw.water_infrastructure,
            storage: raw.storage,
            shelter: raw.shelter,
            basic_production: raw.basic_production,
            bottleneck_present: raw.bottleneck_present,
            unstable_chain: raw.unstable_chain,
            capacity_constraints: raw.capacity_constraints,
            forecast_stable: raw.forecast_stable,
            evidence_ids: raw.evidence_ids,
            report_ids: raw.report_ids,
        };
        inputs.validate().map_err(D::Error::custom)?;
        Ok(inputs)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostureSelection {
    pub posture: LeaderPosture,
    pub rationale_keys: BTreeSet<PlannerId>,
}

pub fn select_posture(
    inputs: &ReportSafePostureInputs,
) -> Result<PostureSelection, LeaderPlannerError> {
    inputs.validate()?;
    let forecast = inputs.essential_forecast_minutes;
    let infrastructure = inputs.infrastructure();
    let infrastructure_missing = infrastructure.contains(&InfrastructureBelief::Missing);
    let infrastructure_complete = infrastructure
        .iter()
        .all(|belief| *belief == InfrastructureBelief::Complete);

    let (posture, rationales): (LeaderPosture, &[&str]) = if matches!(
        inputs.threat,
        ThreatBelief::ActiveAttack | ThreatBelief::Credible
    ) {
        (LeaderPosture::Defend, &["credible_hostile_threat"])
    } else if report_safe_survival_failure(inputs) {
        (LeaderPosture::Crisis, &["survival_failure"])
    } else if inputs.emergency_recently_ended
        && (forecast.is_some_and(|minutes| minutes < 2 * GAME_MINUTES_PER_DAY)
            || inputs.injury == InjuryBelief::Unresolved
            || inputs.housing_damage_unresolved)
    {
        (LeaderPosture::Recover, &["emergency_recovery_incomplete"])
    } else if infrastructure_missing {
        (
            LeaderPosture::Establish,
            &["required_infrastructure_missing"],
        )
    } else if forecast.is_some_and(|minutes| {
        (2 * GAME_MINUTES_PER_DAY..=4 * GAME_MINUTES_PER_DAY).contains(&minutes)
    }) && (inputs.bottleneck_present || inputs.unstable_chain)
    {
        (LeaderPosture::Stabilize, &["essential_chain_unstable"])
    } else if forecast.is_some_and(|minutes| minutes > 4 * GAME_MINUTES_PER_DAY)
        && !inputs.capacity_constraints.is_empty()
    {
        (LeaderPosture::Grow, &["capacity_constrained"])
    } else if forecast.is_some_and(|minutes| minutes >= 7 * GAME_MINUTES_PER_DAY)
        && inputs.forecast_stable
        && infrastructure_complete
    {
        (LeaderPosture::Prosper, &["seven_stable_days"])
    } else {
        (LeaderPosture::Stabilize, &["monitor_reported_conditions"])
    };
    Ok(PostureSelection {
        posture,
        rationale_keys: rationales
            .iter()
            .map(|key| PlannerId::derive("leader_posture_rationale", [key]))
            .collect(),
    })
}

/// Report-visible survival failure, intentionally independent of selected posture.
/// A threat can make defense the leader's priority without making essential reserves
/// safe; callers must retain both goals in that case.
fn report_safe_survival_failure(inputs: &ReportSafePostureInputs) -> bool {
    inputs
        .essential_forecast_minutes
        .is_some_and(|minutes| minutes < GAME_MINUTES_PER_DAY)
        || inputs.food_access == AccessBelief::Inaccessible
        || inputs.water_access == AccessBelief::Inaccessible
        || inputs.injury == InjuryBelief::DangerousUntreated
        || inputs.other_survival_failure
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct EffectiveLevel(u8);

impl EffectiveLevel {
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }

    #[must_use]
    pub const fn leader_cadence_minutes(self) -> u32 {
        match self.0 {
            1 => 12 * 60,
            2 => 6 * 60,
            3 => 3 * 60,
            4 => 60,
            5 => 30,
            _ => unreachable!(),
        }
    }

    #[must_use]
    pub const fn forecast_horizon_hours(self) -> u32 {
        match self.0 {
            1 => 6,
            2 => 12,
            3 => 24,
            4 => 48,
            5 => 72,
            _ => unreachable!(),
        }
    }

    #[must_use]
    pub const fn omission_basis_points(self) -> u16 {
        match self.0 {
            1 => 2_500,
            2 => 1_200,
            3 => 500,
            4 => 100,
            5 => 0,
            _ => unreachable!(),
        }
    }
}

impl TryFrom<u8> for EffectiveLevel {
    type Error = LeaderPlannerError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if (1..=5).contains(&value) {
            Ok(Self(value))
        } else {
            Err(LeaderPlannerError::InvalidEffectiveLevel(value))
        }
    }
}

impl From<EffectiveLevel> for u8 {
    fn from(value: EffectiveLevel) -> Self {
        value.get()
    }
}

#[must_use]
pub const fn personal_level(completed_duty_hours: u64) -> EffectiveLevel {
    EffectiveLevel(if completed_duty_hours >= 480 {
        5
    } else if completed_duty_hours >= 240 {
        4
    } else if completed_duty_hours >= 96 {
        3
    } else if completed_duty_hours >= 24 {
        2
    } else {
        1
    })
}

#[must_use]
pub fn effective_level(
    completed_duty_hours: u64,
    workflow_researched: bool,
    reinforcement_researched: bool,
) -> EffectiveLevel {
    let personal = personal_level(completed_duty_hours).get();
    EffectiveLevel(
        personal
            .saturating_add(workflow_researched as u8)
            .saturating_add(reinforcement_researched as u8)
            .min(5),
    )
}

#[must_use]
pub fn omission_roll_basis_points(
    world_seed: u32,
    colony_id: &PlannerId,
    leader_id: &PlannerId,
    domain: AuthorityDomain,
    review_bucket: u64,
) -> u16 {
    let review = review_bucket.to_string();
    let roll = planner_roll(
        world_seed,
        PlannerRngStream::Omission,
        [
            colony_id.as_str(),
            leader_id.as_str(),
            authority_domain_id(domain),
            review.as_str(),
        ],
    );
    ((u64::from(roll.next_seed) * 10_000) >> 32) as u16
}

#[must_use]
pub const fn optional_omission_basis_points(
    level: EffectiveLevel,
    covered_by_officer_request: bool,
) -> u16 {
    if !covered_by_officer_request {
        return level.omission_basis_points();
    }
    match level.get() {
        1 => 1_200,
        2 => 500,
        3 => 100,
        4 | 5 => 0,
        _ => unreachable!(),
    }
}

const fn authority_domain_id(domain: AuthorityDomain) -> &'static str {
    match domain {
        AuthorityDomain::Survival => "survival",
        AuthorityDomain::Evacuation => "evacuation",
        AuthorityDomain::Stewardship => "stewardship",
        AuthorityDomain::Accounting => "accounting",
        AuthorityDomain::Forestry => "forestry",
        AuthorityDomain::Farming => "farming",
        AuthorityDomain::Defense => "defense",
        AuthorityDomain::Research => "research",
        AuthorityDomain::Textiles => "textiles",
        AuthorityDomain::Building => "building",
        AuthorityDomain::Diplomacy => "diplomacy",
        AuthorityDomain::Trade => "trade",
        AuthorityDomain::ColonyWide => "colony_wide",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OfficerGoalCoverage {
    pub domain: AuthorityDomain,
    pub target_id: PlannerId,
}

#[must_use]
pub fn active_officer_goal_coverage(
    requests: &OfficerRequestBook,
    now_tick: u64,
) -> BTreeSet<OfficerGoalCoverage> {
    requests
        .iter()
        .filter(|(_, request)| !request.state.is_terminal() && now_tick < request.expiry_tick)
        .map(|(_, request)| OfficerGoalCoverage {
            domain: request.target_domain,
            target_id: request.target_id.clone(),
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaderGoalKind {
    Defense,
    Survival,
    Hole,
    Growth,
}

impl LeaderGoalKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Defense => "defense",
            Self::Survival => "survival",
            Self::Hole => "hole",
            Self::Growth => "growth",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalCriticality {
    Emergency,
    SelfPreservation,
    Required,
    Optional,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderGoalCandidate {
    pub domain: AuthorityDomain,
    pub kind: LeaderGoalKind,
    pub target_id: PlannerId,
    pub rationale_keys: BTreeSet<PlannerId>,
    pub criticality: GoalCriticality,
}

pub fn foundational_goal_candidates(
    inputs: &ReportSafePostureInputs,
    posture: LeaderPosture,
) -> Result<Vec<LeaderGoalCandidate>, LeaderPlannerError> {
    inputs.validate()?;
    let mut candidates = Vec::new();
    if posture == LeaderPosture::Defend {
        candidates.push(LeaderGoalCandidate {
            domain: AuthorityDomain::Defense,
            kind: LeaderGoalKind::Defense,
            target_id: PlannerId::derive("leader_goal_target", ["colony_defense"]),
            rationale_keys: BTreeSet::from([PlannerId::derive(
                "leader_goal_rationale",
                ["reported_hostile_threat"],
            )]),
            criticality: GoalCriticality::Emergency,
        });
    }
    // Defense is the selected posture under a credible threat, but a reported
    // essential-reserve failure is an independent self-preservation obligation.
    // Keep both goals visible to the downstream intent/scheduler path: posture
    // priority must not make survival disappear when the colony cannot report
    // a day's food/water reserve.
    if matches!(posture, LeaderPosture::Crisis | LeaderPosture::Recover)
        || report_safe_survival_failure(inputs)
    {
        candidates.push(LeaderGoalCandidate {
            domain: AuthorityDomain::Survival,
            kind: LeaderGoalKind::Survival,
            target_id: PlannerId::derive("leader_goal_target", ["essential_reserves"]),
            rationale_keys: BTreeSet::from([PlannerId::derive(
                "leader_goal_rationale",
                ["reported_survival_deficiency"],
            )]),
            criticality: GoalCriticality::SelfPreservation,
        });
    }
    candidates.push(if inputs.hole == InfrastructureBelief::Missing {
        LeaderGoalCandidate {
            domain: AuthorityDomain::Building,
            kind: LeaderGoalKind::Hole,
            target_id: PlannerId::derive("leader_goal_target", ["required_hole"]),
            rationale_keys: BTreeSet::from([PlannerId::derive(
                "leader_goal_rationale",
                ["reported_hole_missing"],
            )]),
            criticality: GoalCriticality::Required,
        }
    } else {
        // A completed Hole begins an endless physical feed obligation; it
        // does not make the Hole goal disappear. The ordinary optional
        // omission band is intentional: inexperienced leaders may forget one
        // review, but every later review sees the demand again.
        LeaderGoalCandidate {
            domain: AuthorityDomain::Research,
            kind: LeaderGoalKind::Hole,
            target_id: PlannerId::derive("leader_goal_target", ["endless_hole_feed"]),
            rationale_keys: BTreeSet::from([PlannerId::derive(
                "leader_goal_rationale",
                ["hole_requests_next_feed"],
            )]),
            criticality: GoalCriticality::Optional,
        }
    });
    // A higher-precedence posture must not erase a report-safe capacity problem.
    // Full housing is a slow emergency: if defense or short-run survival keeps
    // winning posture selection until the founding generation becomes infertile,
    // the village can never recover demographically. Keep Growth as an optional
    // concurrent goal so low-skill leaders may still omit/delay it, but every
    // later review gets another deterministic chance to respond.
    if !inputs.capacity_constraints.is_empty() {
        candidates.push(LeaderGoalCandidate {
            domain: AuthorityDomain::ColonyWide,
            kind: LeaderGoalKind::Growth,
            target_id: PlannerId::derive("leader_goal_target", ["reported_capacity"]),
            rationale_keys: BTreeSet::from([PlannerId::derive(
                "leader_goal_rationale",
                ["reported_capacity_constraint"],
            )]),
            criticality: GoalCriticality::Optional,
        });
    }
    candidates.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.target_id.cmp(&right.target_id))
    });
    Ok(candidates)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LeaderGoalId(PlannerId);

impl LeaderGoalId {
    fn derive(
        colony_id: &PlannerId,
        planning_epoch: u64,
        kind: LeaderGoalKind,
        target_id: &PlannerId,
        occurrence_index: u32,
    ) -> Self {
        let epoch = planning_epoch.to_string();
        let occurrence = occurrence_index.to_string();
        Self(PlannerId::derive(
            "leader_goal",
            [
                colony_id.as_str(),
                epoch.as_str(),
                kind.as_str(),
                target_id.as_str(),
                occurrence.as_str(),
            ],
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderGoal {
    pub schema_version: u32,
    pub id: LeaderGoalId,
    pub planning_epoch: u64,
    pub occurrence_index: u32,
    pub posture: LeaderPosture,
    pub domain: AuthorityDomain,
    pub kind: LeaderGoalKind,
    pub target_id: PlannerId,
    pub rationale_keys: BTreeSet<PlannerId>,
    pub criticality: GoalCriticality,
    pub covered_by_officer_request: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalReviewContext {
    pub world_seed: u32,
    pub colony_id: PlannerId,
    pub leader_id: PlannerId,
    pub planning_epoch: u64,
    pub review_bucket: u64,
    pub posture: LeaderPosture,
    pub review_domain: AuthorityDomain,
    pub effective_level: EffectiveLevel,
    pub officer_coverage: BTreeSet<OfficerGoalCoverage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderGoalPlan {
    pub schema_version: u32,
    pub colony_id: PlannerId,
    pub planning_epoch: u64,
    pub posture: LeaderPosture,
    pub omission_roll_basis_points: u16,
    pub goals: Vec<LeaderGoal>,
}

pub fn generate_goal_plan(
    context: &GoalReviewContext,
    mut candidates: Vec<LeaderGoalCandidate>,
) -> Result<LeaderGoalPlan, LeaderPlannerError> {
    if candidates.len() > MAX_GOALS_PER_REVIEW {
        return Err(LeaderPlannerError::GoalCapacityReached);
    }
    for candidate in &candidates {
        validate_candidate(candidate)?;
    }
    candidates.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.target_id.cmp(&right.target_id))
            .then_with(|| left.rationale_keys.cmp(&right.rationale_keys))
    });
    if candidates
        .windows(2)
        .any(|pair| (pair[0].kind, &pair[0].target_id) == (pair[1].kind, &pair[1].target_id))
    {
        return Err(LeaderPlannerError::DuplicateGoalCandidate);
    }

    let omission_roll = omission_roll_basis_points(
        context.world_seed,
        &context.colony_id,
        &context.leader_id,
        context.review_domain,
        context.review_bucket,
    );
    let goals = candidates
        .into_iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            let coverage = OfficerGoalCoverage {
                domain: candidate.domain,
                target_id: candidate.target_id.clone(),
            };
            let covered = context.officer_coverage.contains(&coverage);
            let can_omit = candidate.kind != LeaderGoalKind::Defense
                && candidate.criticality == GoalCriticality::Optional;
            let omitted = can_omit
                && omission_roll < optional_omission_basis_points(context.effective_level, covered);
            (!omitted).then(|| LeaderGoal {
                schema_version: LEADER_PLANNER_SCHEMA_VERSION,
                id: LeaderGoalId::derive(
                    &context.colony_id,
                    context.planning_epoch,
                    candidate.kind,
                    &candidate.target_id,
                    index as u32,
                ),
                planning_epoch: context.planning_epoch,
                occurrence_index: index as u32,
                posture: context.posture,
                domain: candidate.domain,
                kind: candidate.kind,
                target_id: candidate.target_id,
                rationale_keys: candidate.rationale_keys,
                criticality: candidate.criticality,
                covered_by_officer_request: covered,
            })
        })
        .collect::<Vec<_>>();
    let plan = LeaderGoalPlan {
        schema_version: LEADER_PLANNER_SCHEMA_VERSION,
        colony_id: context.colony_id.clone(),
        planning_epoch: context.planning_epoch,
        posture: context.posture,
        omission_roll_basis_points: omission_roll,
        goals,
    };
    plan.validate()?;
    Ok(plan)
}

fn validate_candidate(candidate: &LeaderGoalCandidate) -> Result<(), LeaderPlannerError> {
    if candidate.rationale_keys.is_empty()
        || candidate.rationale_keys.len() > MAX_RATIONALE_KEYS_PER_GOAL
    {
        return Err(LeaderPlannerError::MalformedGoal);
    }
    Ok(())
}

impl LeaderGoalPlan {
    fn validate(&self) -> Result<(), LeaderPlannerError> {
        if self.schema_version != LEADER_PLANNER_SCHEMA_VERSION
            || self.omission_roll_basis_points >= 10_000
            || self.goals.len() > MAX_GOALS_PER_REVIEW
        {
            return Err(LeaderPlannerError::MalformedPersistence);
        }
        let mut previous = None;
        let mut semantic = BTreeSet::new();
        for goal in &self.goals {
            if goal.schema_version != LEADER_PLANNER_SCHEMA_VERSION
                || goal.planning_epoch != self.planning_epoch
                || goal.posture != self.posture
                || goal.rationale_keys.is_empty()
                || goal.rationale_keys.len() > MAX_RATIONALE_KEYS_PER_GOAL
                || goal.id
                    != LeaderGoalId::derive(
                        &self.colony_id,
                        goal.planning_epoch,
                        goal.kind,
                        &goal.target_id,
                        goal.occurrence_index,
                    )
                || !semantic.insert((goal.kind, goal.target_id.clone()))
            {
                return Err(LeaderPlannerError::MalformedPersistence);
            }
            let key = (goal.kind, goal.target_id.clone(), goal.id.clone());
            if previous.as_ref().is_some_and(|prior| prior >= &key) {
                return Err(LeaderPlannerError::MalformedPersistence);
            }
            previous = Some(key);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedLeaderGoalPlan {
    schema_version: u32,
    colony_id: PlannerId,
    planning_epoch: u64,
    posture: LeaderPosture,
    omission_roll_basis_points: u16,
    goals: Vec<LeaderGoal>,
}

impl<'de> Deserialize<'de> for LeaderGoalPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;

        let raw = UncheckedLeaderGoalPlan::deserialize(deserializer)?;
        let plan = Self {
            schema_version: raw.schema_version,
            colony_id: raw.colony_id,
            planning_epoch: raw.planning_epoch,
            posture: raw.posture,
            omission_roll_basis_points: raw.omission_roll_basis_points,
            goals: raw.goals,
        };
        plan.validate().map_err(D::Error::custom)?;
        Ok(plan)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainGoalSignal {
    pub domain: AuthorityDomain,
    pub kind: LeaderGoalKind,
    pub target_id: PlannerId,
    pub rationale_keys: BTreeSet<PlannerId>,
    pub criticality: GoalCriticality,
    pub urgency: BasisPoints,
    pub confidence: BasisPoints,
    pub opportunity_cost: BasisPoints,
    pub churn_penalty: BasisPoints,
    pub temporary_player_bias: BasisPoints,
    pub specialist_role: Option<OfficerRole>,
}

impl DomainGoalSignal {
    fn validate(&self) -> Result<(), LeaderPlannerError> {
        if self.rationale_keys.is_empty()
            || self.rationale_keys.len() > MAX_RATIONALE_KEYS_PER_GOAL
            || !(0..=BASIS_POINTS_SCALE).contains(&self.confidence.get())
            || self.urgency.get() < 0
            || self.opportunity_cost.get() < 0
            || self.churn_penalty.get() < 0
        {
            return Err(LeaderPlannerError::MalformedGoal);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaderDomainPlannerOwner {
    Leader,
    FoundingNoSpecialistFallback,
    Officer(OfficerRole),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoredLeaderGoal {
    pub goal: LeaderGoal,
    pub score: PlannerScore,
    pub strategic_weight: BasisPoints,
    pub personality_weight: BasisPoints,
    pub confidence: BasisPoints,
    pub planner_owner: LeaderDomainPlannerOwner,
    pub explanation_keys: BTreeSet<PlannerId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OmittedLeaderGoal {
    pub kind: LeaderGoalKind,
    pub target_id: PlannerId,
    pub omission_roll_basis_points: u16,
    pub omission_threshold_basis_points: u16,
    pub explanation_keys: BTreeSet<PlannerId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderDomainPlan {
    pub schema_version: u32,
    pub colony_id: PlannerId,
    pub planning_epoch: u64,
    pub posture: LeaderPosture,
    pub cadence_minutes: u32,
    pub forecast_horizon_hours: u32,
    pub omission_roll_basis_points: u16,
    pub goals: Vec<ScoredLeaderGoal>,
    pub omitted_goals: Vec<OmittedLeaderGoal>,
}

pub fn plan_founding_leader_domains(
    context: &GoalReviewContext,
    personality: CatPersonality,
    mut signals: Vec<DomainGoalSignal>,
    filled_specialists: &BTreeSet<OfficerRole>,
) -> Result<LeaderDomainPlan, LeaderPlannerError> {
    if signals.len() > MAX_GOALS_PER_REVIEW {
        return Err(LeaderPlannerError::GoalCapacityReached);
    }
    for signal in &signals {
        signal.validate()?;
    }
    signals.sort_by(|left, right| {
        criticality_rank(left.criticality)
            .cmp(&criticality_rank(right.criticality))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.target_id.cmp(&right.target_id))
            .then_with(|| left.rationale_keys.cmp(&right.rationale_keys))
    });
    if signals
        .windows(2)
        .any(|pair| (pair[0].kind, &pair[0].target_id) == (pair[1].kind, &pair[1].target_id))
    {
        return Err(LeaderPlannerError::DuplicateGoalCandidate);
    }

    let omission_roll = omission_roll_basis_points(
        context.world_seed,
        &context.colony_id,
        &context.leader_id,
        context.review_domain,
        context.review_bucket,
    );
    let mut included = Vec::new();
    let mut omitted = Vec::new();
    for (index, signal) in signals.into_iter().enumerate() {
        let coverage = OfficerGoalCoverage {
            domain: signal.domain,
            target_id: signal.target_id.clone(),
        };
        let covered = context.officer_coverage.contains(&coverage);
        let omission_threshold = optional_omission_basis_points(context.effective_level, covered);
        let can_omit = signal.criticality == GoalCriticality::Optional;
        let omitted_by_review = can_omit && omission_roll < omission_threshold;
        let owner = planner_owner(&signal, filled_specialists);
        if omitted_by_review {
            omitted.push(OmittedLeaderGoal {
                kind: signal.kind,
                target_id: signal.target_id,
                omission_roll_basis_points: omission_roll,
                omission_threshold_basis_points: omission_threshold,
                explanation_keys: explanation_keys(
                    &signal.rationale_keys,
                    signal.criticality,
                    owner,
                    true,
                    covered,
                ),
            });
            continue;
        }

        let strategic_weight =
            strategic_weight_basis_points(context.posture, signal.kind, signal.criticality);
        let personality_weight = personality_weight_for_goal(personality, signal.kind);
        let score = score_intent(IntentScoreInputs {
            urgency: signal.urgency,
            strategic_weight,
            personality_weight,
            confidence: signal.confidence,
            opportunity_cost: signal.opportunity_cost,
            churn_penalty: signal.churn_penalty,
            starvation_age: BasisPoints::new(0),
            temporary_player_bias: signal.temporary_player_bias,
        });
        included.push(ScoredLeaderGoal {
            goal: LeaderGoal {
                schema_version: LEADER_PLANNER_SCHEMA_VERSION,
                id: LeaderGoalId::derive(
                    &context.colony_id,
                    context.planning_epoch,
                    signal.kind,
                    &signal.target_id,
                    index as u32,
                ),
                planning_epoch: context.planning_epoch,
                occurrence_index: index as u32,
                posture: context.posture,
                domain: signal.domain,
                kind: signal.kind,
                target_id: signal.target_id,
                rationale_keys: signal.rationale_keys.clone(),
                criticality: signal.criticality,
                covered_by_officer_request: covered,
            },
            score,
            strategic_weight,
            personality_weight,
            confidence: signal.confidence,
            planner_owner: owner,
            explanation_keys: explanation_keys(
                &signal.rationale_keys,
                signal.criticality,
                owner,
                false,
                covered,
            ),
        });
    }

    included.sort_by(|left, right| {
        criticality_rank(left.goal.criticality)
            .cmp(&criticality_rank(right.goal.criticality))
            .then_with(|| right.score.cmp(&left.score))
            .then_with(|| left.goal.kind.cmp(&right.goal.kind))
            .then_with(|| left.goal.target_id.cmp(&right.goal.target_id))
            .then_with(|| left.goal.id.cmp(&right.goal.id))
    });
    let plan = LeaderDomainPlan {
        schema_version: LEADER_PLANNER_SCHEMA_VERSION,
        colony_id: context.colony_id.clone(),
        planning_epoch: context.planning_epoch,
        posture: context.posture,
        cadence_minutes: context.effective_level.leader_cadence_minutes(),
        forecast_horizon_hours: context.effective_level.forecast_horizon_hours(),
        omission_roll_basis_points: omission_roll,
        goals: included,
        omitted_goals: omitted,
    };
    plan.validate()?;
    Ok(plan)
}

impl LeaderDomainPlan {
    fn validate(&self) -> Result<(), LeaderPlannerError> {
        if self.schema_version != LEADER_PLANNER_SCHEMA_VERSION
            || self.omission_roll_basis_points >= 10_000
            || self.goals.len() + self.omitted_goals.len() > MAX_GOALS_PER_REVIEW
        {
            return Err(LeaderPlannerError::MalformedPersistence);
        }
        for scored in &self.goals {
            if scored.goal.schema_version != LEADER_PLANNER_SCHEMA_VERSION
                || scored.goal.planning_epoch != self.planning_epoch
                || scored.goal.posture != self.posture
                || scored.explanation_keys.is_empty()
                || scored.explanation_keys.len() > MAX_RATIONALE_KEYS_PER_GOAL
                || !(0..=BASIS_POINTS_SCALE).contains(&scored.confidence.get())
            {
                return Err(LeaderPlannerError::MalformedPersistence);
            }
        }
        for omitted in &self.omitted_goals {
            if omitted.explanation_keys.is_empty()
                || omitted.explanation_keys.len() > MAX_RATIONALE_KEYS_PER_GOAL
                || omitted.omission_roll_basis_points >= 10_000
                || omitted.omission_threshold_basis_points > 10_000
            {
                return Err(LeaderPlannerError::MalformedPersistence);
            }
        }
        Ok(())
    }
}

#[must_use]
pub const fn strategic_weight_basis_points(
    posture: LeaderPosture,
    kind: LeaderGoalKind,
    criticality: GoalCriticality,
) -> BasisPoints {
    let base = match kind {
        LeaderGoalKind::Defense => 20_000,
        LeaderGoalKind::Survival => 18_000,
        LeaderGoalKind::Hole => 14_000,
        LeaderGoalKind::Growth => 10_000,
    };
    let posture_bonus = match (posture, kind) {
        (LeaderPosture::Defend, LeaderGoalKind::Defense) => 5_000,
        (LeaderPosture::Crisis | LeaderPosture::Recover, LeaderGoalKind::Survival) => 4_000,
        (LeaderPosture::Establish, LeaderGoalKind::Hole) => 3_000,
        (LeaderPosture::Grow | LeaderPosture::Prosper, LeaderGoalKind::Growth) => 2_000,
        _ => 0,
    };
    let criticality_bonus = match criticality {
        GoalCriticality::Emergency => 5_000,
        GoalCriticality::SelfPreservation => 3_000,
        GoalCriticality::Required => 1_000,
        GoalCriticality::Optional => 0,
    };
    BasisPoints::new(base + posture_bonus + criticality_bonus)
}

#[must_use]
pub const fn personality_weight_for_goal(
    personality: CatPersonality,
    kind: LeaderGoalKind,
) -> BasisPoints {
    match kind {
        LeaderGoalKind::Defense | LeaderGoalKind::Survival => {
            personality.weight_factor(PersonalityAxis::CautiousBold, PersonalityPole::Negative)
        }
        LeaderGoalKind::Hole => {
            personality.weight_factor(PersonalityAxis::SkepticalDevout, PersonalityPole::Positive)
        }
        LeaderGoalKind::Growth => {
            personality.weight_factor(PersonalityAxis::ContentAmbitious, PersonalityPole::Positive)
        }
    }
}

const fn criticality_rank(criticality: GoalCriticality) -> u8 {
    match criticality {
        GoalCriticality::Emergency => 0,
        GoalCriticality::SelfPreservation => 1,
        GoalCriticality::Required => 2,
        GoalCriticality::Optional => 3,
    }
}

fn planner_owner(
    signal: &DomainGoalSignal,
    filled_specialists: &BTreeSet<OfficerRole>,
) -> LeaderDomainPlannerOwner {
    match signal.specialist_role {
        Some(role) if filled_specialists.contains(&role) => LeaderDomainPlannerOwner::Officer(role),
        Some(_) => LeaderDomainPlannerOwner::FoundingNoSpecialistFallback,
        None => LeaderDomainPlannerOwner::Leader,
    }
}

fn explanation_keys(
    rationale_keys: &BTreeSet<PlannerId>,
    criticality: GoalCriticality,
    owner: LeaderDomainPlannerOwner,
    omitted: bool,
    covered_by_officer_request: bool,
) -> BTreeSet<PlannerId> {
    let mut keys = rationale_keys.clone();
    if criticality == GoalCriticality::Emergency {
        insert_explanation(&mut keys, "emergency_injected");
    }
    if owner == LeaderDomainPlannerOwner::FoundingNoSpecialistFallback {
        insert_explanation(&mut keys, "founding_no_specialist_fallback");
    }
    if covered_by_officer_request {
        insert_explanation(&mut keys, "officer_request_reduced_omission");
    }
    if omitted {
        insert_explanation(&mut keys, "optional_goal_omitted");
    }
    keys
}

fn insert_explanation(keys: &mut BTreeSet<PlannerId>, value: &str) {
    if keys.len() < MAX_RATIONALE_KEYS_PER_GOAL {
        keys.insert(PlannerId::derive("leader_goal_explanation", [value]));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaderPlannerError {
    InvalidEffectiveLevel(u8),
    GoalCapacityReached,
    DuplicateGoalCandidate,
    MalformedGoal,
    MalformedPersistence,
}

impl fmt::Display for LeaderPlannerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "leader planner error: {self:?}")
    }
}

impl std::error::Error for LeaderPlannerError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        beliefs::{BeliefKey, BeliefKind},
        cat_traits::{
            CatPersonality, PersonalityAxis, PersonalityPole, PersonalityStrength, PersonalityValue,
        },
        officer_requests::{OfficerRequest, OfficerRequestId, RequestKind},
        officers::OfficerRole,
    };

    fn id(namespace: &str, value: &str) -> PlannerId {
        PlannerId::derive(namespace, [value])
    }

    fn complete_inputs(days: u32) -> ReportSafePostureInputs {
        ReportSafePostureInputs {
            essential_forecast_minutes: Some(days * GAME_MINUTES_PER_DAY),
            threat: ThreatBelief::None,
            food_access: AccessBelief::Accessible,
            water_access: AccessBelief::Accessible,
            injury: InjuryBelief::None,
            hole: InfrastructureBelief::Complete,
            food_infrastructure: InfrastructureBelief::Complete,
            water_infrastructure: InfrastructureBelief::Complete,
            storage: InfrastructureBelief::Complete,
            shelter: InfrastructureBelief::Complete,
            basic_production: InfrastructureBelief::Complete,
            ..ReportSafePostureInputs::unknown()
        }
    }

    fn rationale(value: &str) -> BTreeSet<PlannerId> {
        BTreeSet::from([id("rationale", value)])
    }

    fn candidate(
        kind: LeaderGoalKind,
        target: &str,
        criticality: GoalCriticality,
    ) -> LeaderGoalCandidate {
        LeaderGoalCandidate {
            domain: AuthorityDomain::ColonyWide,
            kind,
            target_id: id("target", target),
            rationale_keys: rationale(target),
            criticality,
        }
    }

    fn domain_signal(
        kind: LeaderGoalKind,
        target: &str,
        criticality: GoalCriticality,
        urgency: i64,
        confidence: i64,
    ) -> DomainGoalSignal {
        DomainGoalSignal {
            domain: match kind {
                LeaderGoalKind::Defense => AuthorityDomain::Defense,
                LeaderGoalKind::Survival => AuthorityDomain::Survival,
                LeaderGoalKind::Hole => AuthorityDomain::Research,
                LeaderGoalKind::Growth => AuthorityDomain::ColonyWide,
            },
            kind,
            target_id: id("target", target),
            rationale_keys: rationale(target),
            criticality,
            urgency: BasisPoints::new(urgency),
            confidence: BasisPoints::new(confidence),
            opportunity_cost: BasisPoints::new(0),
            churn_penalty: BasisPoints::new(0),
            temporary_player_bias: BasisPoints::new(0),
            specialist_role: None,
        }
    }

    fn context(level: u8) -> GoalReviewContext {
        GoalReviewContext {
            world_seed: 42,
            colony_id: id("colony", "one"),
            leader_id: id("cat", "leader"),
            planning_epoch: 7,
            review_bucket: 9,
            posture: LeaderPosture::Stabilize,
            review_domain: AuthorityDomain::ColonyWide,
            effective_level: EffectiveLevel::try_from(level).unwrap(),
            officer_coverage: BTreeSet::new(),
        }
    }

    #[test]
    fn posture_triggers_and_precedence_are_exact() {
        let mut inputs = complete_inputs(7);
        inputs.forecast_stable = true;
        assert_eq!(
            select_posture(&inputs).unwrap().posture,
            LeaderPosture::Prosper
        );

        inputs
            .capacity_constraints
            .insert(CapacityConstraint::Storage);
        assert_eq!(
            select_posture(&inputs).unwrap().posture,
            LeaderPosture::Grow
        );
        inputs.hole = InfrastructureBelief::Missing;
        assert_eq!(
            select_posture(&inputs).unwrap().posture,
            LeaderPosture::Establish
        );
        inputs.emergency_recently_ended = true;
        inputs.essential_forecast_minutes = Some(47 * 60 + 59);
        assert_eq!(
            select_posture(&inputs).unwrap().posture,
            LeaderPosture::Recover
        );
        inputs.essential_forecast_minutes = Some(23 * 60 + 59);
        assert_eq!(
            select_posture(&inputs).unwrap().posture,
            LeaderPosture::Crisis
        );
        inputs.threat = ThreatBelief::ActiveAttack;
        assert_eq!(
            select_posture(&inputs).unwrap().posture,
            LeaderPosture::Defend
        );

        let mut stabilize = complete_inputs(3);
        stabilize.bottleneck_present = true;
        assert_eq!(
            select_posture(&stabilize).unwrap().posture,
            LeaderPosture::Stabilize
        );
    }

    #[test]
    fn posture_forecast_boundaries_are_integer_exact() {
        let mut inputs = complete_inputs(0);
        inputs.essential_forecast_minutes = Some(GAME_MINUTES_PER_DAY - 1);
        assert_eq!(
            select_posture(&inputs).unwrap().posture,
            LeaderPosture::Crisis
        );
        inputs.essential_forecast_minutes = Some(GAME_MINUTES_PER_DAY);
        assert_ne!(
            select_posture(&inputs).unwrap().posture,
            LeaderPosture::Crisis
        );

        inputs.emergency_recently_ended = true;
        inputs.essential_forecast_minutes = Some(2 * GAME_MINUTES_PER_DAY - 1);
        assert_eq!(
            select_posture(&inputs).unwrap().posture,
            LeaderPosture::Recover
        );
        inputs.essential_forecast_minutes = Some(2 * GAME_MINUTES_PER_DAY);
        assert_ne!(
            select_posture(&inputs).unwrap().posture,
            LeaderPosture::Recover
        );

        inputs.emergency_recently_ended = false;
        inputs.bottleneck_present = true;
        inputs.essential_forecast_minutes = Some(4 * GAME_MINUTES_PER_DAY);
        assert_eq!(
            select_posture(&inputs).unwrap().posture,
            LeaderPosture::Stabilize
        );
        inputs.bottleneck_present = false;
        inputs
            .capacity_constraints
            .insert(CapacityConstraint::Production);
        inputs.essential_forecast_minutes = Some(4 * GAME_MINUTES_PER_DAY + 1);
        assert_eq!(
            select_posture(&inputs).unwrap().posture,
            LeaderPosture::Grow
        );

        inputs.capacity_constraints.clear();
        inputs.forecast_stable = true;
        inputs.essential_forecast_minutes = Some(7 * GAME_MINUTES_PER_DAY);
        assert_eq!(
            select_posture(&inputs).unwrap().posture,
            LeaderPosture::Prosper
        );
    }

    #[test]
    fn inaccessible_essentials_and_dangerous_injury_are_crisis_but_unknown_is_not_truth() {
        let unknown = ReportSafePostureInputs::unknown();
        assert_eq!(
            select_posture(&unknown).unwrap().posture,
            LeaderPosture::Stabilize
        );
        for crisis in [
            ReportSafePostureInputs {
                food_access: AccessBelief::Inaccessible,
                ..unknown.clone()
            },
            ReportSafePostureInputs {
                water_access: AccessBelief::Inaccessible,
                ..unknown.clone()
            },
            ReportSafePostureInputs {
                injury: InjuryBelief::DangerousUntreated,
                ..unknown.clone()
            },
        ] {
            assert_eq!(
                select_posture(&crisis).unwrap().posture,
                LeaderPosture::Crisis
            );
        }
        let json = serde_json::to_string(&unknown).unwrap();
        assert!(!json.contains("hidden"));
        assert!(!json.contains("regeneration"));
    }

    #[test]
    fn foundational_goals_cover_defense_survival_hole_and_growth_with_bounded_keys() {
        let mut combined = Vec::new();
        let mut defend = complete_inputs(7);
        defend.threat = ThreatBelief::Credible;
        combined.extend(foundational_goal_candidates(&defend, LeaderPosture::Defend).unwrap());

        let mut crisis = complete_inputs(0);
        crisis.essential_forecast_minutes = Some(GAME_MINUTES_PER_DAY - 1);
        combined.extend(foundational_goal_candidates(&crisis, LeaderPosture::Crisis).unwrap());

        let mut establish = complete_inputs(3);
        establish.hole = InfrastructureBelief::Missing;
        combined
            .extend(foundational_goal_candidates(&establish, LeaderPosture::Establish).unwrap());

        let mut grow = complete_inputs(5);
        grow.capacity_constraints
            .insert(CapacityConstraint::Storage);
        combined.extend(foundational_goal_candidates(&grow, LeaderPosture::Grow).unwrap());
        combined.sort_by_key(|candidate| candidate.kind);

        for kind in [
            LeaderGoalKind::Defense,
            LeaderGoalKind::Survival,
            LeaderGoalKind::Hole,
            LeaderGoalKind::Growth,
        ] {
            assert!(
                combined.iter().any(|goal| goal.kind == kind),
                "foundational planner omitted {kind:?}"
            );
        }
        assert!(
            combined.iter().any(|goal| {
                goal.kind == LeaderGoalKind::Hole
                    && goal.criticality == GoalCriticality::Optional
                    && goal.target_id
                        == PlannerId::derive("leader_goal_target", ["endless_hole_feed"])
            }),
            "a completed Hole must keep requesting an omittable next feed"
        );
        assert!(combined.iter().all(|goal| {
            !goal.rationale_keys.is_empty()
                && goal.rationale_keys.len() <= MAX_RATIONALE_KEYS_PER_GOAL
        }));
    }

    #[test]
    fn credible_threat_keeps_defense_priority_and_emits_report_safe_survival() {
        let mut inputs = complete_inputs(0);
        inputs.threat = ThreatBelief::Credible;
        inputs.essential_forecast_minutes = Some(0);

        let selection = select_posture(&inputs).unwrap();
        assert_eq!(selection.posture, LeaderPosture::Defend);
        let candidates = foundational_goal_candidates(&inputs, selection.posture).unwrap();
        assert_eq!(candidates[0].kind, LeaderGoalKind::Defense);
        assert_eq!(candidates[0].criticality, GoalCriticality::Emergency);
        let survival = candidates
            .iter()
            .find(|candidate| candidate.kind == LeaderGoalKind::Survival)
            .expect("report-safe reserve failure remains schedulable under defense");
        assert_eq!(survival.criticality, GoalCriticality::SelfPreservation);
    }

    #[test]
    fn full_housing_keeps_optional_growth_visible_under_higher_priority_postures() {
        for posture in [
            LeaderPosture::Defend,
            LeaderPosture::Crisis,
            LeaderPosture::Recover,
            LeaderPosture::Stabilize,
        ] {
            let mut inputs = complete_inputs(7);
            inputs
                .capacity_constraints
                .insert(CapacityConstraint::Population);
            let candidates = foundational_goal_candidates(&inputs, posture).unwrap();
            let growth = candidates
                .iter()
                .find(|candidate| candidate.kind == LeaderGoalKind::Growth)
                .expect("report-safe full housing remains a concurrent growth candidate");
            assert_eq!(growth.criticality, GoalCriticality::Optional);
        }
    }

    #[test]
    fn duty_thresholds_bonuses_cadence_horizon_and_omission_tables_are_exact() {
        for (hours, level) in [
            (0, 1),
            (23, 1),
            (24, 2),
            (95, 2),
            (96, 3),
            (239, 3),
            (240, 4),
            (479, 4),
            (480, 5),
        ] {
            assert_eq!(personal_level(hours).get(), level);
        }
        assert_eq!(effective_level(0, true, true).get(), 3);
        assert_eq!(effective_level(480, true, true).get(), 5);
        let levels = (1..=5)
            .map(|value| EffectiveLevel::try_from(value).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            levels
                .iter()
                .map(|level| level.leader_cadence_minutes())
                .collect::<Vec<_>>(),
            [720, 360, 180, 60, 30]
        );
        assert_eq!(
            levels
                .iter()
                .map(|level| level.forecast_horizon_hours())
                .collect::<Vec<_>>(),
            [6, 12, 24, 48, 72]
        );
        assert_eq!(
            levels
                .iter()
                .map(|level| level.omission_basis_points())
                .collect::<Vec<_>>(),
            [2_500, 1_200, 500, 100, 0]
        );
        assert_eq!(
            levels
                .iter()
                .map(|level| optional_omission_basis_points(*level, true))
                .collect::<Vec<_>>(),
            [1_200, 500, 100, 0, 0]
        );
    }

    #[test]
    fn omission_is_one_keyed_integer_roll_per_review_and_order_independent() {
        let context = context(1);
        let roll = omission_roll_basis_points(
            context.world_seed,
            &context.colony_id,
            &context.leader_id,
            context.review_domain,
            context.review_bucket,
        );
        assert_eq!(
            roll,
            omission_roll_basis_points(
                context.world_seed,
                &context.colony_id,
                &context.leader_id,
                context.review_domain,
                context.review_bucket,
            )
        );
        assert!(roll < 10_000);

        let candidates = vec![
            candidate(
                LeaderGoalKind::Growth,
                "territory",
                GoalCriticality::Optional,
            ),
            candidate(
                LeaderGoalKind::Defense,
                "perimeter",
                GoalCriticality::Optional,
            ),
            candidate(
                LeaderGoalKind::Survival,
                "water",
                GoalCriticality::SelfPreservation,
            ),
            candidate(LeaderGoalKind::Hole, "feed", GoalCriticality::Required),
        ];
        let mut reversed = candidates.clone();
        reversed.reverse();
        let forward = generate_goal_plan(&context, candidates).unwrap();
        let reverse = generate_goal_plan(&context, reversed).unwrap();
        assert_eq!(forward, reverse);
        assert_eq!(forward.omission_roll_basis_points, roll);
        assert!(
            forward
                .goals
                .iter()
                .any(|goal| goal.kind == LeaderGoalKind::Defense)
        );
        assert!(forward.goals.iter().any(|goal| {
            goal.kind == LeaderGoalKind::Survival
                && goal.criticality == GoalCriticality::SelfPreservation
        }));
    }

    #[test]
    fn officer_coverage_advances_one_band_without_forcing_optional_approval() {
        let mut context = context(1);
        context.world_seed = (1..100_000)
            .find(|seed| {
                let roll = omission_roll_basis_points(
                    *seed,
                    &context.colony_id,
                    &context.leader_id,
                    context.review_domain,
                    context.review_bucket,
                );
                (1_200..2_500).contains(&roll)
            })
            .expect("bounded seed matrix contains a roll between the first two omission bands");
        let optional = candidate(LeaderGoalKind::Growth, "storage", GoalCriticality::Optional);
        let defense = candidate(
            LeaderGoalKind::Defense,
            "perimeter",
            GoalCriticality::Optional,
        );
        let self_preservation = candidate(
            LeaderGoalKind::Survival,
            "water",
            GoalCriticality::SelfPreservation,
        );
        let without_request = generate_goal_plan(
            &context,
            vec![optional.clone(), defense.clone(), self_preservation.clone()],
        )
        .unwrap();
        assert_eq!(without_request.goals.len(), 2);
        assert!(
            without_request
                .goals
                .iter()
                .all(|goal| goal.kind != LeaderGoalKind::Growth)
        );

        context.officer_coverage.insert(OfficerGoalCoverage {
            domain: optional.domain,
            target_id: optional.target_id.clone(),
        });
        let with_request =
            generate_goal_plan(&context, vec![optional, defense, self_preservation]).unwrap();
        assert_eq!(with_request.goals.len(), 3);
        assert!(with_request.goals.iter().any(|goal| {
            goal.kind == LeaderGoalKind::Growth && goal.covered_by_officer_request
        }));
        assert_eq!(
            with_request.omission_roll_basis_points,
            without_request.omission_roll_basis_points
        );
    }

    #[test]
    fn valid_nonexpired_officer_request_covers_only_its_exact_goal() {
        let colony = id("colony", "one");
        let officer = id("cat", "steward");
        let target = id("target", "storage");
        let request = OfficerRequest::proposed(
            OfficerRequestId::derive(&colony, &officer, RequestKind::Building, &target, 0),
            colony,
            officer,
            OfficerRole::Steward,
            AuthorityDomain::Stewardship,
            AuthorityDomain::ColonyWide,
            RequestKind::Building,
            target.clone(),
            1,
            id("rationale", "capacity"),
            100,
            60,
        )
        .unwrap();
        let expiry = request.expiry_tick;
        let mut book = OfficerRequestBook::new();
        book.insert_or_merge(request).unwrap();
        assert_eq!(
            active_officer_goal_coverage(&book, expiry - 1),
            BTreeSet::from([OfficerGoalCoverage {
                domain: AuthorityDomain::ColonyWide,
                target_id: target,
            }])
        );
        assert!(active_officer_goal_coverage(&book, expiry).is_empty());
    }

    #[test]
    fn goal_order_ids_and_rationales_are_stable_bounded_and_strictly_persisted() {
        let context = context(5);
        let plan = generate_goal_plan(
            &context,
            vec![
                candidate(
                    LeaderGoalKind::Growth,
                    "territory",
                    GoalCriticality::Optional,
                ),
                candidate(LeaderGoalKind::Survival, "food", GoalCriticality::Required),
                candidate(LeaderGoalKind::Defense, "wall", GoalCriticality::Emergency),
                candidate(LeaderGoalKind::Hole, "feed", GoalCriticality::Required),
            ],
        )
        .unwrap();
        assert_eq!(
            plan.goals.iter().map(|goal| goal.kind).collect::<Vec<_>>(),
            [
                LeaderGoalKind::Defense,
                LeaderGoalKind::Survival,
                LeaderGoalKind::Hole,
                LeaderGoalKind::Growth,
            ]
        );

        let mut value = serde_json::to_value(&plan).unwrap();
        let restored: LeaderGoalPlan = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(restored, plan);
        value["schemaVersion"] = serde_json::json!(3);
        assert!(serde_json::from_value::<LeaderGoalPlan>(value).is_err());

        let mut too_many = candidate(
            LeaderGoalKind::Growth,
            "overexplained",
            GoalCriticality::Optional,
        );
        too_many.rationale_keys = (0..=MAX_RATIONALE_KEYS_PER_GOAL)
            .map(|index| id("rationale", &index.to_string()))
            .collect();
        assert_eq!(
            generate_goal_plan(&context, vec![too_many]),
            Err(LeaderPlannerError::MalformedGoal)
        );
    }

    #[test]
    fn report_input_serde_rejects_versions_and_oversized_provenance() {
        let inputs = complete_inputs(7);
        let json = serde_json::to_string(&inputs).unwrap();
        assert_eq!(
            serde_json::from_str::<ReportSafePostureInputs>(&json).unwrap(),
            inputs
        );
        let mut wrong = serde_json::to_value(&inputs).unwrap();
        wrong["schemaVersion"] = serde_json::json!(3);
        assert!(serde_json::from_value::<ReportSafePostureInputs>(wrong).is_err());

        let mut oversized = inputs;
        let key = BeliefKey::new(
            id("domain", "stock"),
            id("subject", "food"),
            BeliefKind::Stock,
        );
        let reporter = id("cat", "accountant");
        oversized.evidence_ids = (0..=MAX_POSTURE_EVIDENCE_IDS as u32)
            .map(|occurrence| EvidenceId::derive("colony", &key, 10, &reporter, occurrence))
            .collect();
        assert_eq!(
            select_posture(&oversized),
            Err(LeaderPlannerError::MalformedPersistence)
        );
        assert!(
            serde_json::from_value::<ReportSafePostureInputs>(
                serde_json::to_value(oversized).unwrap()
            )
            .is_err()
        );
    }

    #[test]
    fn domain_review_injects_emergencies_and_keeps_hole_ahead_of_growth() {
        let mut context = context(5);
        context.posture = LeaderPosture::Prosper;
        let personality = CatPersonality::default();
        let goals = vec![
            domain_signal(
                LeaderGoalKind::Growth,
                "territory",
                GoalCriticality::Optional,
                9_000,
                10_000,
            ),
            domain_signal(
                LeaderGoalKind::Hole,
                "feed",
                GoalCriticality::Required,
                9_000,
                10_000,
            ),
            domain_signal(
                LeaderGoalKind::Defense,
                "raid",
                GoalCriticality::Emergency,
                1_000,
                3_000,
            ),
        ];

        let plan =
            plan_founding_leader_domains(&context, personality, goals, &BTreeSet::new()).unwrap();

        assert_eq!(
            plan.goals
                .iter()
                .map(|goal| goal.goal.kind)
                .collect::<Vec<_>>(),
            [
                LeaderGoalKind::Defense,
                LeaderGoalKind::Hole,
                LeaderGoalKind::Growth,
            ]
        );
        assert!(
            plan.goals[0]
                .explanation_keys
                .contains(&id("leader_goal_explanation", "emergency_injected"))
        );
        assert_eq!(
            plan.goals[1].score,
            plan.goals[2].score.max(plan.goals[1].score)
        );
    }

    #[test]
    fn personality_and_report_confidence_weight_domain_scores_without_hidden_truth() {
        let context = context(5);
        let mut devout = CatPersonality::default();
        devout.set(
            PersonalityAxis::SkepticalDevout,
            PersonalityValue::new(PersonalityPole::Positive, PersonalityStrength::Extreme),
        );
        let mut skeptical = CatPersonality::default();
        skeptical.set(
            PersonalityAxis::SkepticalDevout,
            PersonalityValue::new(PersonalityPole::Negative, PersonalityStrength::Extreme),
        );
        let high_confidence = domain_signal(
            LeaderGoalKind::Hole,
            "feed",
            GoalCriticality::Required,
            10_000,
            10_000,
        );
        let low_confidence = domain_signal(
            LeaderGoalKind::Hole,
            "feed",
            GoalCriticality::Required,
            10_000,
            5_000,
        );

        let devout_plan = plan_founding_leader_domains(
            &context,
            devout,
            vec![high_confidence.clone()],
            &BTreeSet::new(),
        )
        .unwrap();
        let skeptical_plan = plan_founding_leader_domains(
            &context,
            skeptical,
            vec![high_confidence],
            &BTreeSet::new(),
        )
        .unwrap();
        let uncertain_plan =
            plan_founding_leader_domains(&context, devout, vec![low_confidence], &BTreeSet::new())
                .unwrap();

        assert!(devout_plan.goals[0].score > skeptical_plan.goals[0].score);
        assert!(uncertain_plan.goals[0].score < devout_plan.goals[0].score);
        let json = serde_json::to_string(&devout_plan).unwrap();
        assert!(!json.contains("hidden"));
        assert!(!json.contains("authoritative"));
    }

    #[test]
    fn absent_specialists_use_bounded_founding_leader_fallback_explanations() {
        let context = context(5);
        let mut farming = domain_signal(
            LeaderGoalKind::Survival,
            "food",
            GoalCriticality::SelfPreservation,
            9_000,
            8_000,
        );
        farming.specialist_role = Some(OfficerRole::Farmer);
        let fallback = plan_founding_leader_domains(
            &context,
            CatPersonality::default(),
            vec![farming.clone()],
            &BTreeSet::new(),
        )
        .unwrap();
        assert_eq!(
            fallback.goals[0].planner_owner,
            LeaderDomainPlannerOwner::FoundingNoSpecialistFallback
        );
        assert!(fallback.goals[0].explanation_keys.len() <= MAX_RATIONALE_KEYS_PER_GOAL);
        assert!(fallback.goals[0].explanation_keys.contains(&id(
            "leader_goal_explanation",
            "founding_no_specialist_fallback"
        )));

        let officer_owned = plan_founding_leader_domains(
            &context,
            CatPersonality::default(),
            vec![farming],
            &BTreeSet::from([OfficerRole::Farmer]),
        )
        .unwrap();
        assert_eq!(
            officer_owned.goals[0].planner_owner,
            LeaderDomainPlannerOwner::Officer(OfficerRole::Farmer)
        );
    }
}
