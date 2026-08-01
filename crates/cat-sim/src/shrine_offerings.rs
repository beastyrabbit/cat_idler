//! Endless belief-driven Shrine offering pipeline specified by
//! `docs/leader-ai-overhaul/shrine-favor-research.md`.

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    authority::AuthorityDomain,
    favor::{Favor, FavorCommitOutcome, FavorError, FavorEventId, FavorEventKind, FavorLedger},
    leader_planner::{EffectiveLevel, omission_roll_basis_points, optional_omission_basis_points},
    planner_core::{BASIS_POINTS_SCALE, PlannerId},
};

pub const SHRINE_OFFERING_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfferingPackage {
    Food,
    Herbs,
    Materials,
    RefinedResources,
}

impl OfferingPackage {
    pub const ALL: [Self; 4] = [
        Self::Food,
        Self::Herbs,
        Self::Materials,
        Self::RefinedResources,
    ];

    #[must_use]
    pub const fn quantity(self) -> u64 {
        match self {
            Self::Food => 20,
            Self::Herbs => 5,
            Self::Materials => 10,
            Self::RefinedResources => 5,
        }
    }

    #[must_use]
    pub const fn base_favor(self) -> Favor {
        Favor::ONE
    }

    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Food => "food_20",
            Self::Herbs => "herbs_5",
            Self::Materials => "materials_10",
            Self::RefinedResources => "refined_resources_5",
        }
    }
}

/// Report-safe estimates available to the planner. No authoritative stock or
/// regeneration value is accepted by this API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OfferingBeliefEstimate {
    pub package: OfferingPackage,
    pub believed_available_lower: u64,
    pub replacement_minutes: u64,
    pub labor_minutes: u64,
    pub reserve_risk_basis_points: i64,
    pub committed_use_penalty_basis_points: i64,
    pub confidence_basis_points: u16,
    pub evidence_ids: Vec<String>,
}

impl OfferingBeliefEstimate {
    fn utility_micro_favor(&self) -> Option<i128> {
        if self.believed_available_lower < self.package.quantity()
            || self.confidence_basis_points == 0
            || self.evidence_ids.iter().any(String::is_empty)
        {
            return None;
        }
        // 1 + replacement_hours + labor_hours/6, expressed in 1/360 hours.
        let denominator = 360_i128
            .saturating_add(i128::from(self.replacement_minutes).saturating_mul(6))
            .saturating_add(i128::from(self.labor_minutes));
        let expected =
            i128::from(self.package.base_favor().micro_favor()).saturating_mul(360) / denominator;
        let penalties = i128::from(
            self.reserve_risk_basis_points
                .saturating_add(self.committed_use_penalty_basis_points),
        )
        .saturating_mul(i128::from(Favor::ONE.micro_favor()))
            / i128::from(BASIS_POINTS_SCALE);
        Some(expected.saturating_sub(penalties))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OfferingChoice {
    pub package: OfferingPackage,
    pub utility_micro_favor: i128,
    pub evidence_ids: Vec<String>,
}

impl OfferingChoice {
    fn validate(&self) -> Result<(), OfferingError> {
        if self.evidence_ids.is_empty()
            || self.evidence_ids.iter().any(String::is_empty)
            || self.evidence_ids.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(OfferingError::MalformedState);
        }
        Ok(())
    }
}

/// Select solely from supplied beliefs. Equal utility resolves by stable
/// package order and input order cannot affect the result.
#[must_use]
pub fn select_offering(estimates: &[OfferingBeliefEstimate]) -> Option<OfferingChoice> {
    let mut candidates = estimates
        .iter()
        .filter_map(|estimate| {
            estimate
                .utility_micro_favor()
                .map(|utility| (estimate, utility))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|(left, left_utility), (right, right_utility)| {
        right_utility
            .cmp(left_utility)
            .then_with(|| left.package.cmp(&right.package))
    });
    candidates.first().map(|(estimate, utility)| {
        let mut evidence_ids = estimate.evidence_ids.clone();
        evidence_ids.sort();
        evidence_ids.dedup();
        OfferingChoice {
            package: estimate.package,
            utility_micro_favor: *utility,
            evidence_ids,
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShrineOfferingReviewContext {
    pub world_seed: u32,
    pub colony_id: PlannerId,
    pub leader_id: PlannerId,
    pub review_bucket: u64,
    pub effective_level: EffectiveLevel,
    pub covered_by_officer_request: bool,
    pub survival_or_active_defense: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShrineOfferingReviewBlock {
    CadenceNotDue,
    SurvivalOrActiveDefense,
    ActivePipeline,
    NoBelievedPackage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShrineOfferingReview {
    Deferred {
        reason: ShrineOfferingReviewBlock,
    },
    Omitted {
        roll_basis_points: u16,
        omission_basis_points: u16,
    },
    Started {
        pipeline_id: PlannerId,
        choice: OfferingChoice,
        roll_basis_points: u16,
        omission_basis_points: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfferingStage {
    Selected,
    ResourcesReserved,
    InTransit,
    Deposited,
    Ritual,
    Completed,
    Blocked,
    Cancelled,
}

impl OfferingStage {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Blocked | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OfferingCargoDisposition {
    ReleasedBeforePickup,
    DeliveredToShrine,
    SalvagedToStockpile { stockpile_id: String },
}

impl OfferingCargoDisposition {
    fn validate(&self) -> Result<(), OfferingError> {
        if let Self::SalvagedToStockpile { stockpile_id } = self
            && stockpile_id.is_empty()
        {
            return Err(OfferingError::MalformedState);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OfferingPipeline {
    pub schema_version: u32,
    pub id: PlannerId,
    pub shrine_id: String,
    pub occurrence: u64,
    pub choice: OfferingChoice,
    pub stage: OfferingStage,
    pub physical_task_id: Option<String>,
    pub cargo_disposition: Option<OfferingCargoDisposition>,
    pub blocked_reason: Option<String>,
    pub credited_event_id: Option<FavorEventId>,
    pub updated_tick: u64,
}

impl OfferingPipeline {
    fn validate(&self) -> Result<(), OfferingError> {
        self.choice.validate()?;
        if let Some(disposition) = &self.cargo_disposition {
            disposition.validate()?;
        }
        if self.schema_version != SHRINE_OFFERING_SCHEMA_VERSION || self.shrine_id.is_empty() {
            return Err(OfferingError::MalformedState);
        }
        match self.stage {
            OfferingStage::Selected => {
                if self.physical_task_id.is_some()
                    || self.cargo_disposition.is_some()
                    || self.blocked_reason.is_some()
                    || self.credited_event_id.is_some()
                {
                    return Err(OfferingError::MalformedState);
                }
            }
            OfferingStage::ResourcesReserved
            | OfferingStage::InTransit
            | OfferingStage::Deposited
            | OfferingStage::Ritual => {
                if self.physical_task_id.as_ref().is_none_or(String::is_empty)
                    || self.blocked_reason.is_some()
                    || self.credited_event_id.is_some()
                {
                    return Err(OfferingError::MalformedState);
                }
                if matches!(
                    self.stage,
                    OfferingStage::ResourcesReserved | OfferingStage::InTransit
                ) && self.cargo_disposition.is_some()
                {
                    return Err(OfferingError::MalformedState);
                }
                if matches!(self.stage, OfferingStage::Deposited | OfferingStage::Ritual)
                    && self.cargo_disposition != Some(OfferingCargoDisposition::DeliveredToShrine)
                {
                    return Err(OfferingError::MalformedState);
                }
            }
            OfferingStage::Completed => {
                if self.physical_task_id.as_ref().is_none_or(String::is_empty)
                    || self.blocked_reason.is_some()
                    || self.cargo_disposition != Some(OfferingCargoDisposition::DeliveredToShrine)
                    || self.credited_event_id.is_none()
                {
                    return Err(OfferingError::MalformedState);
                }
            }
            OfferingStage::Blocked => {
                if self.physical_task_id.as_ref().is_none_or(String::is_empty)
                    || self.blocked_reason.as_ref().is_none_or(String::is_empty)
                    || !matches!(
                        self.cargo_disposition,
                        Some(OfferingCargoDisposition::DeliveredToShrine)
                            | Some(OfferingCargoDisposition::SalvagedToStockpile { .. })
                    )
                    || self.credited_event_id.is_some()
                {
                    return Err(OfferingError::MalformedState);
                }
            }
            OfferingStage::Cancelled => {
                if self.blocked_reason.is_some()
                    || self.credited_event_id.is_some()
                    || self.cargo_disposition
                        != Some(OfferingCargoDisposition::ReleasedBeforePickup)
                {
                    return Err(OfferingError::MalformedState);
                }
            }
        }
        Ok(())
    }

    fn transition(&mut self, next: OfferingStage, now_tick: u64) -> Result<(), OfferingError> {
        let valid = matches!(
            (self.stage, next),
            (OfferingStage::Selected, OfferingStage::ResourcesReserved)
                | (OfferingStage::ResourcesReserved, OfferingStage::InTransit)
                | (OfferingStage::InTransit, OfferingStage::Deposited)
                | (OfferingStage::Deposited, OfferingStage::Ritual)
        );
        if !valid {
            return Err(OfferingError::InvalidTransition);
        }
        self.stage = next;
        self.updated_tick = now_tick;
        Ok(())
    }

    pub fn resources_reserved(
        &mut self,
        physical_task_id: impl Into<String>,
        now_tick: u64,
    ) -> Result<(), OfferingError> {
        let task_id = physical_task_id.into();
        if task_id.is_empty() {
            return Err(OfferingError::MalformedState);
        }
        self.transition(OfferingStage::ResourcesReserved, now_tick)?;
        self.physical_task_id = Some(task_id);
        Ok(())
    }

    pub fn depart(&mut self, now_tick: u64) -> Result<(), OfferingError> {
        self.transition(OfferingStage::InTransit, now_tick)
    }

    pub fn deposit(&mut self, now_tick: u64) -> Result<(), OfferingError> {
        self.transition(OfferingStage::Deposited, now_tick)?;
        self.cargo_disposition = Some(OfferingCargoDisposition::DeliveredToShrine);
        Ok(())
    }

    pub fn begin_ritual(&mut self, now_tick: u64) -> Result<(), OfferingError> {
        if self.cargo_disposition != Some(OfferingCargoDisposition::DeliveredToShrine) {
            return Err(OfferingError::PhysicalOfferingIncomplete);
        }
        self.transition(OfferingStage::Ritual, now_tick)
    }

    pub fn cancel_before_departure(&mut self, now_tick: u64) -> Result<(), OfferingError> {
        if !matches!(
            self.stage,
            OfferingStage::Selected | OfferingStage::ResourcesReserved
        ) {
            return Err(OfferingError::CargoRecoveryRequired);
        }
        self.stage = OfferingStage::Cancelled;
        self.cargo_disposition = Some(OfferingCargoDisposition::ReleasedBeforePickup);
        self.updated_tick = now_tick;
        Ok(())
    }

    pub fn block_after_cargo_salvage(
        &mut self,
        bounded_reason: impl Into<String>,
        stockpile_id: impl Into<String>,
        now_tick: u64,
    ) -> Result<(), OfferingError> {
        let stockpile_id = stockpile_id.into();
        let disposition = OfferingCargoDisposition::SalvagedToStockpile { stockpile_id };
        disposition.validate()?;
        self.block_after_recovered_cargo(bounded_reason, disposition, now_tick)
    }

    /// The physical task runtime must first salvage, return, or strand any
    /// picked-up cargo. This records only the bounded pipeline outcome.
    pub fn block_after_physical_recovery(
        &mut self,
        bounded_reason: impl Into<String>,
        cargo_recovery_complete: bool,
        now_tick: u64,
    ) -> Result<(), OfferingError> {
        if !cargo_recovery_complete {
            return Err(OfferingError::CargoRecoveryRequired);
        }
        self.block_after_recovered_cargo(
            bounded_reason,
            OfferingCargoDisposition::DeliveredToShrine,
            now_tick,
        )
    }

    fn block_after_recovered_cargo(
        &mut self,
        bounded_reason: impl Into<String>,
        disposition: OfferingCargoDisposition,
        now_tick: u64,
    ) -> Result<(), OfferingError> {
        let reason = bounded_reason.into();
        if reason.is_empty() || self.stage.is_terminal() {
            return Err(OfferingError::CargoRecoveryRequired);
        }
        disposition.validate()?;
        self.stage = OfferingStage::Blocked;
        self.blocked_reason = Some(reason);
        self.cargo_disposition = Some(disposition);
        self.updated_tick = now_tick;
        Ok(())
    }

    pub fn consume_and_credit(
        &mut self,
        resources_consumed: bool,
        ledger: &mut FavorLedger,
        expected_favor_version: u64,
        now_tick: u64,
    ) -> Result<FavorCommitOutcome, OfferingError> {
        if self.stage == OfferingStage::Completed {
            let Some(event_id) = &self.credited_event_id else {
                return Err(OfferingError::MalformedState);
            };
            if ledger.event(event_id).is_some() {
                return Ok(FavorCommitOutcome::AlreadyCommitted);
            }
            return Err(OfferingError::MalformedState);
        }
        if self.stage != OfferingStage::Ritual
            || !resources_consumed
            || self.cargo_disposition != Some(OfferingCargoDisposition::DeliveredToShrine)
        {
            return Err(OfferingError::PhysicalOfferingIncomplete);
        }
        let event_id = FavorEventId::derive("offering", &self.shrine_id, self.id.as_str());
        let outcome = ledger.credit(
            event_id.clone(),
            FavorEventKind::OfferingCredit,
            self.choice.package.base_favor(),
            expected_favor_version,
            now_tick,
        )?;
        self.credited_event_id = Some(event_id);
        self.stage = OfferingStage::Completed;
        self.updated_tick = now_tick;
        Ok(outcome)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedOfferingPipeline {
    schema_version: u32,
    id: PlannerId,
    shrine_id: String,
    occurrence: u64,
    choice: OfferingChoice,
    stage: OfferingStage,
    physical_task_id: Option<String>,
    #[serde(default)]
    cargo_disposition: Option<OfferingCargoDisposition>,
    blocked_reason: Option<String>,
    credited_event_id: Option<FavorEventId>,
    updated_tick: u64,
}

impl<'de> Deserialize<'de> for OfferingPipeline {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = UncheckedOfferingPipeline::deserialize(deserializer)?;
        let pipeline = Self {
            schema_version: raw.schema_version,
            id: raw.id,
            shrine_id: raw.shrine_id,
            occurrence: raw.occurrence,
            choice: raw.choice,
            stage: raw.stage,
            physical_task_id: raw.physical_task_id,
            cargo_disposition: raw.cargo_disposition,
            blocked_reason: raw.blocked_reason,
            credited_event_id: raw.credited_event_id,
            updated_tick: raw.updated_tick,
        };
        pipeline.validate().map_err(serde::de::Error::custom)?;
        Ok(pipeline)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShrineOfferingState {
    pub schema_version: u32,
    pub shrine_id: String,
    pub next_occurrence: u64,
    pub last_review_bucket: Option<u64>,
    pub current: Option<OfferingPipeline>,
}

impl ShrineOfferingState {
    #[must_use]
    pub fn new(shrine_id: impl Into<String>) -> Self {
        Self {
            schema_version: SHRINE_OFFERING_SCHEMA_VERSION,
            shrine_id: shrine_id.into(),
            next_occurrence: 0,
            last_review_bucket: None,
            current: None,
        }
    }

    pub fn start(
        &mut self,
        choice: OfferingChoice,
        now_tick: u64,
    ) -> Result<&mut OfferingPipeline, OfferingError> {
        choice.validate()?;
        if self.shrine_id.is_empty() {
            return Err(OfferingError::MalformedState);
        }
        if self
            .current
            .as_ref()
            .is_some_and(|pipeline| !pipeline.stage.is_terminal())
        {
            return Err(OfferingError::PipelineAlreadyActive);
        }
        let occurrence = self.next_occurrence;
        self.next_occurrence = self
            .next_occurrence
            .checked_add(1)
            .ok_or(OfferingError::Overflow)?;
        self.current = Some(OfferingPipeline {
            schema_version: SHRINE_OFFERING_SCHEMA_VERSION,
            id: PlannerId::derive(
                "shrine_offering",
                [&self.shrine_id, occurrence.to_string().as_str()],
            ),
            shrine_id: self.shrine_id.clone(),
            occurrence,
            choice,
            stage: OfferingStage::Selected,
            physical_task_id: None,
            cargo_disposition: None,
            blocked_reason: None,
            credited_event_id: None,
            updated_tick: now_tick,
        });
        Ok(self.current.as_mut().expect("pipeline inserted"))
    }

    pub fn consider_endless_offering(
        &mut self,
        context: &ShrineOfferingReviewContext,
        estimates: &[OfferingBeliefEstimate],
        now_tick: u64,
    ) -> Result<ShrineOfferingReview, OfferingError> {
        if self
            .last_review_bucket
            .is_some_and(|reviewed| reviewed >= context.review_bucket)
        {
            return Ok(ShrineOfferingReview::Deferred {
                reason: ShrineOfferingReviewBlock::CadenceNotDue,
            });
        }
        // Every due boundary is a real decision, including omission, emergency
        // deferral, or finding an already-active physical pipeline. Persist it
        // before branching so restart and repeated scheduler passes cannot turn
        // one Leader review into many offerings.
        self.last_review_bucket = Some(context.review_bucket);
        if context.survival_or_active_defense {
            return Ok(ShrineOfferingReview::Deferred {
                reason: ShrineOfferingReviewBlock::SurvivalOrActiveDefense,
            });
        }
        if self
            .current
            .as_ref()
            .is_some_and(|pipeline| !pipeline.stage.is_terminal())
        {
            return Ok(ShrineOfferingReview::Deferred {
                reason: ShrineOfferingReviewBlock::ActivePipeline,
            });
        }
        let Some(choice) = select_offering(estimates) else {
            return Ok(ShrineOfferingReview::Deferred {
                reason: ShrineOfferingReviewBlock::NoBelievedPackage,
            });
        };
        let roll_basis_points = omission_roll_basis_points(
            context.world_seed,
            &context.colony_id,
            &context.leader_id,
            AuthorityDomain::Research,
            context.review_bucket,
        );
        let omission_basis_points = optional_omission_basis_points(
            context.effective_level,
            context.covered_by_officer_request,
        );
        if roll_basis_points < omission_basis_points {
            return Ok(ShrineOfferingReview::Omitted {
                roll_basis_points,
                omission_basis_points,
            });
        }
        let pipeline = self.start(choice, now_tick)?;
        Ok(ShrineOfferingReview::Started {
            pipeline_id: pipeline.id.clone(),
            choice: pipeline.choice.clone(),
            roll_basis_points,
            omission_basis_points,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedShrineOfferingState {
    schema_version: u32,
    shrine_id: String,
    next_occurrence: u64,
    #[serde(default)]
    last_review_bucket: Option<u64>,
    current: Option<OfferingPipeline>,
}

impl ShrineOfferingState {
    fn validate(&self) -> Result<(), OfferingError> {
        if self.schema_version != SHRINE_OFFERING_SCHEMA_VERSION || self.shrine_id.is_empty() {
            return Err(OfferingError::MalformedState);
        }
        if let Some(pipeline) = &self.current {
            pipeline.validate()?;
            if pipeline.shrine_id != self.shrine_id || pipeline.occurrence >= self.next_occurrence {
                return Err(OfferingError::MalformedState);
            }
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for ShrineOfferingState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = UncheckedShrineOfferingState::deserialize(deserializer)?;
        let state = Self {
            schema_version: raw.schema_version,
            shrine_id: raw.shrine_id,
            next_occurrence: raw.next_occurrence,
            last_review_bucket: raw.last_review_bucket,
            current: raw.current,
        };
        state.validate().map_err(serde::de::Error::custom)?;
        Ok(state)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfferingError {
    NoBelievedPackage,
    PipelineAlreadyActive,
    InvalidTransition,
    PhysicalOfferingIncomplete,
    CargoRecoveryRequired,
    MalformedState,
    Overflow,
    Favor(FavorError),
}

impl From<FavorError> for OfferingError {
    fn from(value: FavorError) -> Self {
        Self::Favor(value)
    }
}

impl std::fmt::Display for OfferingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Shrine offering error: {self:?}")
    }
}

impl std::error::Error for OfferingError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn estimate(
        package: OfferingPackage,
        available: u64,
        replacement_minutes: u64,
    ) -> OfferingBeliefEstimate {
        OfferingBeliefEstimate {
            package,
            believed_available_lower: available,
            replacement_minutes,
            labor_minutes: 60,
            reserve_risk_basis_points: 0,
            committed_use_penalty_basis_points: 0,
            confidence_basis_points: 8_000,
            evidence_ids: vec![format!("report-{}", package.stable_id())],
        }
    }

    #[test]
    fn belief_utility_avoids_scarce_food_and_input_order_is_irrelevant() {
        let mut estimates = vec![
            estimate(OfferingPackage::Food, 19, 10),
            estimate(OfferingPackage::Herbs, 8, 30),
            estimate(OfferingPackage::Materials, 20, 300),
        ];
        let first = select_offering(&estimates).unwrap();
        estimates.reverse();
        assert_eq!(select_offering(&estimates).unwrap(), first);
        assert_eq!(first.package, OfferingPackage::Herbs);
    }

    #[test]
    fn poor_beliefs_can_choose_food_without_reading_authoritative_stock() {
        let estimates = [
            estimate(OfferingPackage::Food, 100, 5),
            estimate(OfferingPackage::Herbs, 5, 500),
        ];
        assert_eq!(
            select_offering(&estimates).unwrap().package,
            OfferingPackage::Food
        );
    }

    #[test]
    fn physical_consumption_precedes_single_idempotent_favor_credit() {
        let choice = select_offering(&[estimate(OfferingPackage::Herbs, 10, 10)]).unwrap();
        let mut state = ShrineOfferingState::new("shrine-1");
        let pipeline = state.start(choice, 1).unwrap();
        pipeline.resources_reserved("task-1", 2).unwrap();
        pipeline.depart(3).unwrap();
        pipeline.deposit(4).unwrap();
        pipeline.begin_ritual(5).unwrap();
        let mut ledger = FavorLedger::new();
        assert_eq!(
            pipeline.consume_and_credit(false, &mut ledger, 0, 6),
            Err(OfferingError::PhysicalOfferingIncomplete)
        );
        assert_eq!(ledger.balance, Favor::ZERO);
        assert_eq!(
            pipeline
                .consume_and_credit(true, &mut ledger, 0, 6)
                .unwrap(),
            FavorCommitOutcome::Committed
        );
        assert_eq!(ledger.balance, Favor::ONE);
        assert_eq!(ledger.event_count(), 1);
    }

    #[test]
    fn completed_pipeline_allows_another_immediately_without_cooldown() {
        let choice = select_offering(&[estimate(OfferingPackage::Materials, 20, 10)]).unwrap();
        let mut state = ShrineOfferingState::new("shrine-1");
        let first_id = {
            let pipeline = state.start(choice.clone(), 1).unwrap();
            pipeline.resources_reserved("task-1", 2).unwrap();
            pipeline.depart(3).unwrap();
            pipeline.deposit(4).unwrap();
            pipeline.begin_ritual(5).unwrap();
            let mut ledger = FavorLedger::new();
            pipeline
                .consume_and_credit(true, &mut ledger, 0, 6)
                .unwrap();
            pipeline.id.clone()
        };
        let second = state.start(choice, 6).unwrap();
        assert_ne!(second.id, first_id);
        assert_eq!(second.stage, OfferingStage::Selected);
    }
}
