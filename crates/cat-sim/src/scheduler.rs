//! Deterministic scheduler primitives specified by
//! `docs/leader-ai-overhaul/planner-and-beliefs.md`.

use std::{cmp::Ordering, collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};

use crate::{
    intent_graph::{Intent, IntentReason},
    planner_core::{
        BASIS_POINTS_SCALE, BasisPoints, FailureDisposition, IntentId, IntentScoreInputs,
        IntentState, IntentTieKey, PLANNER_SCHEMA_VERSION, PlannerCoreState, PlannerId,
        PlannerScore, compare_intent_priority, failure_disposition, score_intent,
        starvation_age_basis_points,
    },
};

pub const SCHEDULER_SCHEMA_VERSION: u32 = 1;
pub const HYSTERESIS_BASIS_POINTS: i64 = 1_500;
pub const PLAYER_EPOCH_BIAS_BASIS_POINTS: i64 = 1_500;
pub const MAX_CADENCE_GATES: usize = 64;
pub const MAX_PLAYER_INFLUENCES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerEpochAction {
    MoveUp,
    MoveDown,
    Dismiss,
}

impl PlayerEpochAction {
    #[must_use]
    pub const fn bias(self) -> BasisPoints {
        match self {
            Self::MoveUp => BasisPoints::new(PLAYER_EPOCH_BIAS_BASIS_POINTS),
            Self::MoveDown => BasisPoints::new(-PLAYER_EPOCH_BIAS_BASIS_POINTS),
            Self::Dismiss => BasisPoints::new(0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerEpochInfluence {
    pub planning_epoch: u64,
    pub action: PlayerEpochAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerState {
    pub schema_version: u32,
    pub core: PlannerCoreState,
    player_influences: BTreeMap<IntentId, PlayerEpochInfluence>,
    next_review_ticks: BTreeMap<PlannerId, u64>,
}

impl SchedulerState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: SCHEDULER_SCHEMA_VERSION,
            core: PlannerCoreState::new(),
            player_influences: BTreeMap::new(),
            next_review_ticks: BTreeMap::new(),
        }
    }

    pub fn set_player_influence(
        &mut self,
        intent_id: IntentId,
        action: PlayerEpochAction,
    ) -> InfluenceUpdate {
        let influence = PlayerEpochInfluence {
            planning_epoch: self.core.planning_epoch,
            action,
        };
        if self.player_influences.get(&intent_id) == Some(&influence) {
            return InfluenceUpdate::Unchanged;
        }
        if !self.player_influences.contains_key(&intent_id)
            && self.player_influences.len() >= MAX_PLAYER_INFLUENCES
        {
            return InfluenceUpdate::CapacityReached;
        }
        self.player_influences.insert(intent_id, influence);
        InfluenceUpdate::Replaced
    }

    #[must_use]
    pub fn influence(&self, intent_id: &IntentId) -> Option<PlayerEpochInfluence> {
        self.player_influences
            .get(intent_id)
            .copied()
            .filter(|influence| influence.planning_epoch == self.core.planning_epoch)
    }

    #[must_use]
    pub fn player_bias(&self, intent_id: &IntentId) -> BasisPoints {
        self.influence(intent_id)
            .map_or_else(BasisPoints::default, |influence| influence.action.bias())
    }

    #[must_use]
    pub fn is_dismissed(&self, intent_id: &IntentId) -> bool {
        self.influence(intent_id)
            .is_some_and(|influence| influence.action == PlayerEpochAction::Dismiss)
    }

    pub fn advance_epoch(&mut self, next_epoch: u64) -> Result<(), SchedulerError> {
        if next_epoch < self.core.planning_epoch {
            return Err(SchedulerError::EpochRewind);
        }
        if next_epoch == self.core.planning_epoch {
            return Ok(());
        }
        self.core.planning_epoch = next_epoch;
        self.player_influences.clear();
        Ok(())
    }

    #[must_use]
    pub fn review_due(&self, gate_id: &PlannerId, now_tick: u64, trigger: ReviewTrigger) -> bool {
        trigger.bypasses_cadence()
            || self
                .next_review_ticks
                .get(gate_id)
                .is_none_or(|next| now_tick >= *next)
    }

    pub fn record_review(
        &mut self,
        gate_id: PlannerId,
        now_tick: u64,
        cadence_ticks: u64,
    ) -> Result<u64, SchedulerError> {
        if cadence_ticks == 0 {
            return Err(SchedulerError::ZeroCadence);
        }
        if !self.next_review_ticks.contains_key(&gate_id)
            && self.next_review_ticks.len() >= MAX_CADENCE_GATES
        {
            return Err(SchedulerError::CadenceCapacityReached);
        }
        let next_tick = now_tick.saturating_add(cadence_ticks);
        self.next_review_ticks.insert(gate_id, next_tick);
        self.core.planning_clock = now_tick;
        Ok(next_tick)
    }

    fn validate(&self) -> Result<(), SchedulerError> {
        if self.schema_version != SCHEDULER_SCHEMA_VERSION
            || self.core.schema_version != PLANNER_SCHEMA_VERSION
            || self.next_review_ticks.len() > MAX_CADENCE_GATES
            || self.player_influences.len() > MAX_PLAYER_INFLUENCES
            || self
                .player_influences
                .values()
                .any(|influence| influence.planning_epoch != self.core.planning_epoch)
        {
            return Err(SchedulerError::MalformedPersistence);
        }
        Ok(())
    }
}

impl Default for SchedulerState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UncheckedSchedulerState {
    schema_version: u32,
    core: PlannerCoreState,
    #[serde(default)]
    player_influences: BTreeMap<IntentId, PlayerEpochInfluence>,
    #[serde(default)]
    next_review_ticks: BTreeMap<PlannerId, u64>,
}

impl<'de> Deserialize<'de> for SchedulerState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;

        let raw = UncheckedSchedulerState::deserialize(deserializer)?;
        let state = Self {
            schema_version: raw.schema_version,
            core: raw.core,
            player_influences: raw.player_influences,
            next_review_ticks: raw.next_review_ticks,
        };
        state.validate().map_err(D::Error::custom)?;
        Ok(state)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfluenceUpdate {
    Replaced,
    Unchanged,
    CapacityReached,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewTrigger {
    Scheduled,
    Emergency,
    Death,
    Refusal,
    InjuryOrRecovery,
    TaskTerminal,
    SiteLost,
    RouteChanged,
    PlayerInfluence,
}

impl ReviewTrigger {
    #[must_use]
    pub const fn bypasses_cadence(self) -> bool {
        !matches!(self, Self::Scheduled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerScoreContext {
    pub personality_weight: BasisPoints,
    pub opportunity_cost: BasisPoints,
    pub churn_penalty: BasisPoints,
    pub now_tick: u64,
    pub ticks_per_game_hour: u64,
}

#[must_use]
pub fn utility_score(intent: &Intent, context: SchedulerScoreContext) -> PlannerScore {
    utility_score_with_bias(intent, context, intent.temporary_player_bias)
}

fn utility_score_with_bias(
    intent: &Intent,
    context: SchedulerScoreContext,
    temporary_player_bias: BasisPoints,
) -> PlannerScore {
    let starvation_age = if matches!(
        intent.lifecycle.state,
        IntentState::Proposed
            | IntentState::Approved
            | IntentState::Reserving
            | IntentState::Blocked
            | IntentState::RetryWaiting
    ) {
        starvation_age_basis_points(
            context.now_tick.saturating_sub(intent.creation_tick),
            context.ticks_per_game_hour,
        )
    } else {
        BasisPoints::default()
    };
    score_intent(IntentScoreInputs {
        urgency: intent.urgency,
        strategic_weight: intent.strategic_weight,
        personality_weight: context.personality_weight,
        confidence: BasisPoints::new(i64::from(intent.confidence.get())),
        opportunity_cost: context.opportunity_cost,
        churn_penalty: context.churn_penalty,
        starvation_age,
        temporary_player_bias,
    })
}

/// Score with the current epoch's non-stacking player influence. A dismissed
/// intent is omitted rather than receiving an arbitrarily low sentinel score.
#[must_use]
pub fn utility_score_with_state(
    intent: &Intent,
    context: SchedulerScoreContext,
    scheduler: &SchedulerState,
) -> Option<PlannerScore> {
    (!scheduler.is_dismissed(&intent.id))
        .then(|| utility_score_with_bias(intent, context, scheduler.player_bias(&intent.id)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedIntent {
    pub intent_id: IntentId,
    pub score: PlannerScore,
    pub tie_key: IntentTieKey,
}

#[must_use]
pub fn rank_intents<'a, I>(intents: I, context: SchedulerScoreContext) -> Vec<RankedIntent>
where
    I: IntoIterator<Item = &'a Intent>,
{
    let mut ranked = intents
        .into_iter()
        .map(|intent| RankedIntent {
            intent_id: intent.id.clone(),
            score: utility_score(intent, context),
            tie_key: IntentTieKey {
                kind: intent.kind_id.as_str().to_owned(),
                creation_tick: intent.creation_tick,
                intent_id: intent.id.clone(),
                target_id: intent.target_id.clone(),
            },
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        compare_intent_priority(left.score, &left.tie_key, right.score, &right.tie_key)
    });
    ranked
}

#[must_use]
pub fn rank_intents_with_state<'a, I>(
    intents: I,
    context: SchedulerScoreContext,
    scheduler: &SchedulerState,
) -> Vec<RankedIntent>
where
    I: IntoIterator<Item = &'a Intent>,
{
    let mut ranked = intents
        .into_iter()
        .filter(|intent| !intent.lifecycle.state.is_terminal())
        .filter_map(|intent| {
            utility_score_with_state(intent, context, scheduler).map(|score| RankedIntent {
                intent_id: intent.id.clone(),
                score,
                tie_key: IntentTieKey {
                    kind: intent.kind_id.as_str().to_owned(),
                    creation_tick: intent.creation_tick,
                    intent_id: intent.id.clone(),
                    target_id: intent.target_id.clone(),
                },
            })
        })
        .collect::<Vec<_>>();
    ranked.sort_by(compare_ranked);
    ranked
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreemptionCause {
    Ordinary,
    Emergency,
    RouteInvalidated,
    WorkerIncapacitated,
}

#[must_use]
pub fn should_preempt(
    active_score: PlannerScore,
    replacement_score: PlannerScore,
    cause: PreemptionCause,
) -> bool {
    if !matches!(cause, PreemptionCause::Ordinary) {
        return true;
    }
    if replacement_score <= active_score {
        return false;
    }
    let active = i128::from(active_score.get());
    let improvement = i128::from(replacement_score.get()).saturating_sub(active);
    improvement.saturating_mul(i128::from(BASIS_POINTS_SCALE))
        >= active
            .abs()
            .saturating_mul(i128::from(HYSTERESIS_BASIS_POINTS))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryCause {
    Temporary,
    PermanentInvalidity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryOutcome {
    RetryWaiting { attempt: u8, retry_tick: u64 },
    TerminalFailure { attempts: u8 },
}

pub fn record_failure(
    intent: &mut Intent,
    now_tick: u64,
    ticks_per_game_minute: u64,
    cause: RetryCause,
) -> Result<RetryOutcome, SchedulerError> {
    if intent.lifecycle.state.is_terminal() {
        return Err(SchedulerError::TerminalIntent);
    }
    if cause == RetryCause::PermanentInvalidity {
        let mut lifecycle = intent.lifecycle;
        lifecycle.transition(IntentState::Failed, now_tick)?;
        intent.lifecycle = lifecycle;
        intent.blocked_reason = Some(IntentReason::PermanentInvalidity);
        release_intent_claims(intent);
        return Ok(RetryOutcome::TerminalFailure {
            attempts: intent.lifecycle.retry_count,
        });
    }
    if ticks_per_game_minute == 0 {
        return Err(SchedulerError::ZeroTickScale);
    }
    if intent.lifecycle.state == IntentState::RetryWaiting {
        return Err(SchedulerError::RetryAlreadyWaiting);
    }

    let attempt = intent.lifecycle.retry_count.saturating_add(1);
    let disposition = failure_disposition(attempt, now_tick, ticks_per_game_minute)
        .ok_or(SchedulerError::InvalidRetryAttempt)?;
    let mut lifecycle = intent.lifecycle;
    lifecycle.retry_count = attempt;
    let outcome = match disposition {
        FailureDisposition::RetryAt(retry_tick) => {
            lifecycle.transition(IntentState::RetryWaiting, now_tick)?;
            lifecycle.next_retry_tick = Some(retry_tick);
            RetryOutcome::RetryWaiting {
                attempt,
                retry_tick,
            }
        }
        FailureDisposition::TerminalFailure => {
            lifecycle.transition(IntentState::Failed, now_tick)?;
            RetryOutcome::TerminalFailure { attempts: attempt }
        }
    };
    intent.lifecycle = lifecycle;
    if matches!(outcome, RetryOutcome::TerminalFailure { .. }) {
        release_intent_claims(intent);
    }
    Ok(outcome)
}

pub fn activate_due_retry(intent: &mut Intent, now_tick: u64) -> Result<bool, SchedulerError> {
    if intent.lifecycle.state != IntentState::RetryWaiting
        || intent
            .lifecycle
            .next_retry_tick
            .is_none_or(|retry_tick| now_tick < retry_tick)
    {
        return Ok(false);
    }
    intent
        .lifecycle
        .transition(IntentState::Approved, now_tick)?;
    intent.lifecycle.next_retry_tick = None;
    Ok(true)
}

fn release_intent_claims(intent: &mut Intent) {
    intent.resource_reservation_ids.clear();
    intent.delivery_reservation_ids.clear();
    intent.assigned_cat_ids.clear();
    intent.task_ids.clear();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialChange {
    Belief,
    Route,
    Resource,
    Building,
    Dependency,
}

pub fn derive_materially_new_intent_id(
    prior_id: &IntentId,
    colony_id: &str,
    planning_epoch: u64,
    kind: &str,
    target_id: &str,
    occurrence_index: u32,
    _change: MaterialChange,
) -> Result<IntentId, SchedulerError> {
    let candidate = IntentId::derive(colony_id, planning_epoch, kind, target_id, occurrence_index);
    if &candidate == prior_id {
        return Err(SchedulerError::SameIntentIdentity);
    }
    Ok(candidate)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyDisposition {
    Ready,
    Waiting(Vec<IntentId>),
    Invalidated(Vec<IntentId>),
}

#[must_use]
pub fn dependency_disposition(
    intent: &Intent,
    states: &BTreeMap<IntentId, IntentState>,
) -> DependencyDisposition {
    let mut waiting = Vec::new();
    let mut invalid = Vec::new();
    for dependency in &intent.dependencies {
        match states.get(dependency) {
            Some(IntentState::Succeeded) => {}
            Some(IntentState::Cancelled | IntentState::Failed) | None => {
                invalid.push(dependency.clone());
            }
            Some(_) => waiting.push(dependency.clone()),
        }
    }
    if !invalid.is_empty() {
        DependencyDisposition::Invalidated(invalid)
    } else if !waiting.is_empty() {
        DependencyDisposition::Waiting(waiting)
    } else {
        DependencyDisposition::Ready
    }
}

pub fn apply_dependency_state(
    intent: &mut Intent,
    states: &BTreeMap<IntentId, IntentState>,
    now_tick: u64,
) -> Result<DependencyDisposition, SchedulerError> {
    let disposition = dependency_disposition(intent, states);
    match &disposition {
        DependencyDisposition::Ready => {}
        DependencyDisposition::Waiting(_) => {
            intent
                .lifecycle
                .transition(IntentState::Blocked, now_tick)?;
        }
        DependencyDisposition::Invalidated(_) => {
            intent.lifecycle.transition(IntentState::Failed, now_tick)?;
            intent.blocked_reason = Some(IntentReason::PermanentInvalidity);
            release_intent_claims(intent);
        }
    }
    Ok(disposition)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerError {
    MalformedPersistence,
    EpochRewind,
    ZeroCadence,
    CadenceCapacityReached,
    ZeroTickScale,
    InvalidRetryAttempt,
    RetryAlreadyWaiting,
    TerminalIntent,
    SameIntentIdentity,
    InvalidTransition(crate::planner_core::InvalidIntentTransition),
}

impl From<crate::planner_core::InvalidIntentTransition> for SchedulerError {
    fn from(value: crate::planner_core::InvalidIntentTransition) -> Self {
        Self::InvalidTransition(value)
    }
}

impl fmt::Display for SchedulerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "scheduler error: {self:?}")
    }
}

impl std::error::Error for SchedulerError {}

#[must_use]
pub fn compare_ranked(left: &RankedIntent, right: &RankedIntent) -> Ordering {
    compare_intent_priority(left.score, &left.tie_key, right.score, &right.tie_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        authority::{AuthorityActor, AuthorityDomain},
        beliefs::Confidence,
    };

    const HOUR: u64 = 60;

    fn id(namespace: &str, value: &str) -> PlannerId {
        PlannerId::derive(namespace, [value])
    }

    fn intent(occurrence: u32, kind: &str, target: &str, urgency: i64) -> Intent {
        let mut intent = Intent::proposed(
            IntentId::derive("colony", 1, kind, target, occurrence),
            id("colony", "one"),
            AuthorityActor::Leader {
                cat_id: id("cat", "leader"),
            },
            Some(id("cat", "leader")),
            AuthorityDomain::ColonyWide,
            id("kind", kind),
            id("target", target),
            id("rationale", "test"),
            10,
        );
        intent.urgency = BasisPoints::new(urgency);
        intent.strategic_weight = BasisPoints::new(10_000);
        intent.confidence = Confidence::new(10_000).unwrap();
        intent
    }

    fn score_context(now_tick: u64) -> SchedulerScoreContext {
        SchedulerScoreContext {
            personality_weight: BasisPoints::new(10_000),
            opportunity_cost: BasisPoints::new(0),
            churn_penalty: BasisPoints::new(0),
            now_tick,
            ticks_per_game_hour: HOUR,
        }
    }

    #[test]
    fn utility_scoring_is_fixed_point_saturating_and_order_independent() {
        let mut a = intent(0, "build", "a", 5_000);
        let b = intent(1, "water", "b", 6_000);
        a.temporary_player_bias = BasisPoints::new(1_500);
        assert_eq!(utility_score(&a, score_context(10)).get(), 6_500);

        let forward = rank_intents([&a, &b], score_context(10));
        let reverse = rank_intents([&b, &a], score_context(10));
        assert_eq!(forward, reverse);
        assert_eq!(forward[0].intent_id, a.id);

        a.urgency = BasisPoints::new(i64::MAX);
        a.strategic_weight = BasisPoints::new(i64::MAX);
        a.temporary_player_bias = BasisPoints::new(i64::MAX);
        assert_eq!(utility_score(&a, score_context(u64::MAX)).get(), i64::MAX);
    }

    #[test]
    fn hysteresis_edges_and_bypasses_are_exact_even_for_negative_scores() {
        assert!(!should_preempt(
            PlannerScore::new(1_000),
            PlannerScore::new(1_149),
            PreemptionCause::Ordinary,
        ));
        assert!(should_preempt(
            PlannerScore::new(1_000),
            PlannerScore::new(1_150),
            PreemptionCause::Ordinary,
        ));
        assert!(!should_preempt(
            PlannerScore::new(-1_000),
            PlannerScore::new(-851),
            PreemptionCause::Ordinary,
        ));
        assert!(should_preempt(
            PlannerScore::new(-1_000),
            PlannerScore::new(-850),
            PreemptionCause::Ordinary,
        ));
        for cause in [
            PreemptionCause::Emergency,
            PreemptionCause::RouteInvalidated,
            PreemptionCause::WorkerIncapacitated,
        ] {
            assert!(should_preempt(
                PlannerScore::new(10_000),
                PlannerScore::new(-10_000),
                cause,
            ));
        }
    }

    #[test]
    fn starvation_and_epoch_bias_use_exact_integer_boundaries() {
        let mut scheduler = SchedulerState::new();
        let intent = intent(0, "build", "den", 1_000);
        assert_eq!(utility_score(&intent, score_context(69)).get(), 1_000);
        assert_eq!(utility_score(&intent, score_context(70)).get(), 1_100);
        assert_eq!(utility_score(&intent, score_context(u64::MAX)).get(), 3_500);

        assert_eq!(
            scheduler.set_player_influence(intent.id.clone(), PlayerEpochAction::MoveUp),
            InfluenceUpdate::Replaced
        );
        assert_eq!(scheduler.player_bias(&intent.id), BasisPoints::new(1_500));
        assert_eq!(
            utility_score_with_state(&intent, score_context(10), &scheduler),
            Some(PlannerScore::new(2_500))
        );
        assert_eq!(
            scheduler.set_player_influence(intent.id.clone(), PlayerEpochAction::MoveUp),
            InfluenceUpdate::Unchanged
        );
        scheduler.set_player_influence(intent.id.clone(), PlayerEpochAction::MoveDown);
        assert_eq!(scheduler.player_bias(&intent.id), BasisPoints::new(-1_500));
        scheduler.set_player_influence(intent.id.clone(), PlayerEpochAction::Dismiss);
        assert!(scheduler.is_dismissed(&intent.id));
        assert_eq!(
            utility_score_with_state(&intent, score_context(10), &scheduler),
            None
        );
        assert!(rank_intents_with_state([&intent], score_context(10), &scheduler).is_empty());
        scheduler.advance_epoch(1).unwrap();
        assert_eq!(scheduler.player_bias(&intent.id), BasisPoints::new(0));
        assert!(!scheduler.is_dismissed(&intent.id));
    }

    #[test]
    fn cadence_gates_scheduled_reviews_but_all_invalidation_hooks_bypass() {
        let mut scheduler = SchedulerState::new();
        let gate = id("domain", "forestry");
        assert!(scheduler.review_due(&gate, 5, ReviewTrigger::Scheduled));
        assert_eq!(scheduler.record_review(gate.clone(), 5, 60).unwrap(), 65);
        assert!(!scheduler.review_due(&gate, 64, ReviewTrigger::Scheduled));
        assert!(scheduler.review_due(&gate, 65, ReviewTrigger::Scheduled));
        for trigger in [
            ReviewTrigger::Emergency,
            ReviewTrigger::Death,
            ReviewTrigger::Refusal,
            ReviewTrigger::InjuryOrRecovery,
            ReviewTrigger::TaskTerminal,
            ReviewTrigger::SiteLost,
            ReviewTrigger::RouteChanged,
            ReviewTrigger::PlayerInfluence,
        ] {
            assert!(scheduler.review_due(&gate, 6, trigger));
        }
        assert_eq!(
            scheduler.record_review(gate, 5, 0),
            Err(SchedulerError::ZeroCadence)
        );
    }

    #[test]
    fn retry_schedule_uses_four_waits_then_terminal_fifth_failure() {
        let mut intent = intent(0, "build", "den", 1_000);
        intent
            .lifecycle
            .transition(IntentState::Approved, 1)
            .unwrap();
        let expected = [15, 30, 60, 120];
        let mut now = 100;
        for (index, minutes) in expected.into_iter().enumerate() {
            let retry_tick = now + minutes;
            assert_eq!(
                record_failure(&mut intent, now, 1, RetryCause::Temporary).unwrap(),
                RetryOutcome::RetryWaiting {
                    attempt: index as u8 + 1,
                    retry_tick,
                }
            );
            assert!(!activate_due_retry(&mut intent, retry_tick - 1).unwrap());
            let waiting = intent.lifecycle;
            assert_eq!(
                record_failure(&mut intent, retry_tick - 1, 1, RetryCause::Temporary),
                Err(SchedulerError::RetryAlreadyWaiting)
            );
            assert_eq!(intent.lifecycle, waiting);
            assert!(activate_due_retry(&mut intent, retry_tick).unwrap());
            now = retry_tick;
        }
        intent
            .resource_reservation_ids
            .insert(id("resource", "wood"));
        intent.task_ids.insert(id("task", "build"));
        assert_eq!(
            record_failure(&mut intent, now, 1, RetryCause::Temporary).unwrap(),
            RetryOutcome::TerminalFailure {
                attempts: crate::planner_core::MAX_FAILED_ATTEMPTS,
            }
        );
        assert_eq!(intent.lifecycle.state, IntentState::Failed);
        assert!(intent.resource_reservation_ids.is_empty());
        assert!(intent.task_ids.is_empty());
        assert_eq!(crate::planner_core::RETRY_DELAYS_GAME_MINUTES[4], 240);
    }

    #[test]
    fn material_change_gets_new_identity_without_rewinding_prior_history() {
        let mut prior = intent(0, "build", "den", 1_000);
        prior.lifecycle.retry_count = 4;
        let new_id = derive_materially_new_intent_id(
            &prior.id,
            "colony",
            1,
            "build",
            "den",
            1,
            MaterialChange::Belief,
        )
        .unwrap();
        assert_ne!(new_id, prior.id);
        assert_eq!(prior.lifecycle.retry_count, 4);
        assert_eq!(
            derive_materially_new_intent_id(
                &prior.id,
                "colony",
                1,
                "build",
                "den",
                0,
                MaterialChange::Route,
            ),
            Err(SchedulerError::SameIntentIdentity)
        );
    }

    #[test]
    fn dependency_invalidation_is_ordered_terminal_and_releases_claims() {
        let dep_a = IntentId::derive("colony", 1, "source", "a", 0);
        let dep_b = IntentId::derive("colony", 1, "source", "b", 0);
        let mut intent = intent(0, "build", "den", 1_000);
        intent.dependencies.insert(dep_b.clone());
        intent.dependencies.insert(dep_a.clone());
        intent
            .resource_reservation_ids
            .insert(id("resource", "wood"));
        intent.assigned_cat_ids.insert(id("cat", "builder"));
        let states = BTreeMap::from([(dep_a.clone(), IntentState::Succeeded)]);
        assert_eq!(
            apply_dependency_state(&mut intent, &states, 50).unwrap(),
            DependencyDisposition::Invalidated(vec![dep_b])
        );
        assert_eq!(intent.lifecycle.state, IntentState::Failed);
        assert!(intent.resource_reservation_ids.is_empty());
        assert!(intent.assigned_cat_ids.is_empty());
    }

    #[test]
    fn scheduler_persistence_defaults_collections_and_rejects_stale_epoch() {
        let scheduler = SchedulerState::new();
        let minimal = serde_json::json!({
            "schemaVersion": 1,
            "core": {"schemaVersion": 1, "planningClock": 0, "planningEpoch": 0}
        });
        assert_eq!(
            serde_json::from_value::<SchedulerState>(minimal).unwrap(),
            scheduler
        );

        let mut active = SchedulerState::new();
        let intent_id = IntentId::derive("colony", 0, "build", "den", 0);
        active.set_player_influence(intent_id, PlayerEpochAction::MoveUp);
        let round_trip: SchedulerState =
            serde_json::from_str(&serde_json::to_string(&active).unwrap()).unwrap();
        assert_eq!(round_trip, active);

        let mut malformed = serde_json::to_value(&active).unwrap();
        malformed["core"]["planningEpoch"] = serde_json::json!(1);
        assert!(serde_json::from_value::<SchedulerState>(malformed).is_err());
    }
}
