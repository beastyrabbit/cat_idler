//! LAI.58 persistent physical scholar-preparation state specified by
//! `docs/branch-plan-merge/bug-gui-design-BOARD.md`.
//!
//! The older `Insight` production/purchase surface remains solely for dirty
//! worktree compatibility. Authoritative LAI.58 work uses Notes/Void in
//! `research_purchase`; preparation is labor, never a third currency.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    acquired_traits::AcquiredTraitState,
    divine_boosts::DivineBoostResearchStages,
    favor::{Favor, FavorLedger},
    planner_core::{BASIS_POINTS_SCALE, BasisPoints, PlannerId},
    research_manifest::{
        ADMINISTRATION_BASE_STANDING_ORDER_SLOTS, ADMINISTRATION_BASE_STRATEGIC_INTENT_SLOTS,
        ADMINISTRATION_STAGE_STANDING_ORDER_SLOTS, ADMINISTRATION_STAGE_STRATEGIC_INTENT_SLOTS,
        DIVINE_DURATION_ALLOWED_GAME_HOURS, DIVINE_ECONOMY_STAGE_DISCOUNT_BASIS_POINTS,
        REHABILITATION_STAGE_RESTORATION_PERCENTAGE_POINTS,
    },
    research_purchase::{
        GodResearchCurrency, GodResearchFundOutcome, GodResearchQueueEntry,
        LeaderResearchCompletion, LeaderResearchRequest, PLAYER_PREPARATION_DISCOUNT_BASIS_POINTS,
        PlayerResearchPurchaseRequest, ResearchFunds, ResearchPurchaseError, ResearchPurchaseId,
        ResearchPurchaseOutcome, ResearchPurchaseState, StudyId, SyntheticResearchCatalog,
    },
};

pub const SCHOLAR_RESEARCH_SCHEMA_VERSION: u32 = 1;
pub const GAME_MINUTES_PER_WEEK: u64 = 7 * 24 * 60;
pub const INSIGHT_MICRO_PER_INSIGHT: u64 = 1_000_000;
pub const INSIGHT_PER_COMPLETED_WEEK: u64 = 20;
pub const MAX_SCHOLARS: usize = 256;
pub const MAX_PREPARED_STUDIES: usize = 2_048;
pub const MAX_SCHOLAR_WORK_EVENTS: usize = 2_048;
pub const MAX_PREPARATION_EVENTS: usize = 1_024;
pub const MAX_CONSUMED_PREPARATIONS: usize = 512;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Insight(u64);

impl Insight {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn from_micro_insight(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn from_whole(value: u64) -> Option<Self> {
        match value.checked_mul(INSIGHT_MICRO_PER_INSIGHT) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    #[must_use]
    pub const fn micro_insight(self) -> u64 {
        self.0
    }

    fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    fn from_favor(value: Favor) -> Self {
        Self(value.micro_favor())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScholarId(PlannerId);

impl ScholarId {
    #[must_use]
    pub fn derive(cat_id: &str) -> Self {
        Self(PlannerId::derive("scholar", [cat_id]))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScholarWorkEventId(PlannerId);

impl ScholarWorkEventId {
    #[must_use]
    pub fn derive(colony_id: &PlannerId, action_id: &str) -> Self {
        Self(PlannerId::derive(
            "scholar_work_event",
            [colony_id.as_str(), action_id],
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PreparationId(PlannerId);

impl PreparationId {
    #[must_use]
    pub fn derive(colony_id: &PlannerId, action_id: &str) -> Self {
        Self(PlannerId::derive(
            "study_preparation",
            [colony_id.as_str(), action_id],
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScholarWorkOutcome {
    Recorded,
    AlreadyRecorded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparationOutcome {
    Prepared,
    AlreadyPrepared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScholarWorkAuthorization {
    pub scholars_guild_owned: bool,
    pub completed_research_station: bool,
    pub scholar_alive: bool,
}

impl ScholarWorkAuthorization {
    fn permits_work(self) -> bool {
        self.scholars_guild_owned && self.completed_research_station && self.scholar_alive
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScholarWorkModifiers {
    pub research_skill: BasisPoints,
    pub scholarship: BasisPoints,
}

impl ScholarWorkModifiers {
    fn validate(self) -> Result<(), ScholarResearchError> {
        if self.research_skill.get() < 0 || self.scholarship.get() < 0 {
            return Err(ScholarResearchError::InvalidModifier);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScholarWorkRequest {
    pub id: ScholarWorkEventId,
    pub scholar_id: ScholarId,
    pub completed_minutes: u64,
    pub expected_version: u64,
    pub authorization: ScholarWorkAuthorization,
    pub modifiers: ScholarWorkModifiers,
    pub completed_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScholarProgress {
    pub scholar_id: ScholarId,
    pub alive: bool,
    pub partial_week_minutes: u64,
    pub produced_insight: Insight,
}

impl ScholarProgress {
    fn validate(&self) -> Result<(), ScholarResearchError> {
        if self.partial_week_minutes >= GAME_MINUTES_PER_WEEK {
            return Err(ScholarResearchError::MalformedPersistence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScholarWorkEvent {
    pub id: ScholarWorkEventId,
    pub scholar_id: ScholarId,
    pub completed_minutes: u64,
    pub research_skill_basis_points: i64,
    pub scholarship_basis_points: i64,
    pub credited_insight: Insight,
    pub committed_version: u64,
    pub completed_tick: u64,
}

impl ScholarWorkEvent {
    fn matches(&self, request: &ScholarWorkRequest) -> bool {
        self.scholar_id == request.scholar_id
            && self.completed_minutes == request.completed_minutes
            && self.research_skill_basis_points == request.modifiers.research_skill.get()
            && self.scholarship_basis_points == request.modifiers.scholarship.get()
            && self.completed_tick == request.completed_tick
    }

    fn validate(&self) -> Result<(), ScholarResearchError> {
        if self.completed_minutes == 0
            || self.research_skill_basis_points < 0
            || self.scholarship_basis_points < 0
        {
            return Err(ScholarResearchError::MalformedPersistence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepareStudyRequest {
    pub id: PreparationId,
    pub study_id: StudyId,
    pub assigned_scholar: ScholarId,
    pub expected_version: u64,
    pub prepared_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparedStudy {
    pub preparation_id: PreparationId,
    pub study_id: StudyId,
    pub assigned_scholar: Option<ScholarId>,
    pub insight_cost: Insight,
    /// Zero for legacy pre-cutover preparations.  A positive value represents
    /// the physical, staffed 25%-of-frozen-duration preparation contract.
    #[serde(default)]
    pub required_labor_minutes: u64,
    #[serde(default)]
    pub completed_labor_minutes: u64,
    pub prepared_version: u64,
    pub prepared_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparationEvent {
    pub id: PreparationId,
    pub study_id: StudyId,
    pub assigned_scholar: ScholarId,
    pub insight_cost: Insight,
    #[serde(default)]
    pub required_labor_minutes: u64,
    pub committed_version: u64,
    pub prepared_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeginPhysicalPreparationRequest {
    pub id: PreparationId,
    pub study_id: StudyId,
    pub assigned_scholar: ScholarId,
    pub authorization: ScholarWorkAuthorization,
    pub expected_version: u64,
    pub prepared_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FundPreparedGodResearchRequest {
    pub id: ResearchPurchaseId,
    pub study_id: StudyId,
    pub expected_research_version: u64,
    pub expected_scholar_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderResearchWithPreparationLoss {
    pub completion: LeaderResearchCompletion,
    pub lost_preparation_labor_minutes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalPreparationLaborOutcome {
    NoWork,
    Advanced { remaining_labor_minutes: u64 },
    Prepared,
}

impl PreparationEvent {
    fn matches(&self, request: &PrepareStudyRequest) -> bool {
        self.study_id == request.study_id
            && self.assigned_scholar == request.assigned_scholar
            && self.prepared_tick == request.prepared_tick
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScholarPlayerPurchaseRequest {
    pub id: ResearchPurchaseId,
    pub colony_id: PlannerId,
    pub study_id: StudyId,
    pub expected_research_version: u64,
    pub expected_favor_version: u64,
    pub expected_scholar_version: u64,
    pub use_preparation: bool,
    pub now_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScholarResearchState {
    pub schema_version: u32,
    pub version: u64,
    pub insight_balance: Insight,
    /// Durable beginning of the currently credited insight week.
    #[serde(default)]
    pub insight_week_started_tick: Option<u64>,
    #[serde(default)]
    pub generated_this_week: Insight,
    scholars: BTreeMap<ScholarId, ScholarProgress>,
    preparations: BTreeMap<StudyId, PreparedStudy>,
    preparation_events: BTreeMap<PreparationId, PreparationEvent>,
    work_events: BTreeMap<ScholarWorkEventId, ScholarWorkEvent>,
    consumed_preparations: BTreeMap<ResearchPurchaseId, StudyId>,
}

impl ScholarResearchState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: SCHOLAR_RESEARCH_SCHEMA_VERSION,
            version: 0,
            insight_balance: Insight::ZERO,
            insight_week_started_tick: None,
            generated_this_week: Insight::ZERO,
            scholars: BTreeMap::new(),
            preparations: BTreeMap::new(),
            preparation_events: BTreeMap::new(),
            work_events: BTreeMap::new(),
            consumed_preparations: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn scholar(&self, scholar_id: &ScholarId) -> Option<&ScholarProgress> {
        self.scholars.get(scholar_id)
    }

    #[must_use]
    pub fn prepared_study(&self, study_id: &StudyId) -> Option<&PreparedStudy> {
        self.preparations.get(study_id)
    }

    /// Iterate living/deceased scholar progress in stable scholar-ID order.
    pub fn scholars(&self) -> impl ExactSizeIterator<Item = &ScholarProgress> {
        self.scholars.values()
    }

    /// Iterate durable prepared studies in stable study-ID order.
    pub fn preparations(&self) -> impl ExactSizeIterator<Item = &PreparedStudy> {
        self.preparations.values()
    }

    /// Legacy scholar-Insight production retained outside LAI.58 authority.
    pub fn record_completed_study_work(
        &mut self,
        acquired_traits: &mut AcquiredTraitState,
        request: ScholarWorkRequest,
    ) -> Result<ScholarWorkOutcome, ScholarResearchError> {
        if let Some(existing) = self.work_events.get(&request.id) {
            return if existing.matches(&request) {
                Ok(ScholarWorkOutcome::AlreadyRecorded)
            } else {
                Err(ScholarResearchError::EventIdConflict)
            };
        }
        self.validate()?;
        request.modifiers.validate()?;
        if request.expected_version != self.version {
            return Err(ScholarResearchError::StaleVersion);
        }
        if request.completed_minutes == 0 {
            return Err(ScholarResearchError::ZeroCompletedWork);
        }
        if !request.authorization.permits_work() {
            return Err(ScholarResearchError::ScholarWorkLocked);
        }
        if self.work_events.len() >= MAX_SCHOLAR_WORK_EVENTS {
            return Err(ScholarResearchError::CapacityExceeded);
        }

        let mut next = self.clone();
        let mut next_traits = acquired_traits.clone();
        let progress = next
            .scholars
            .entry(request.scholar_id.clone())
            .or_insert_with(|| ScholarProgress {
                scholar_id: request.scholar_id.clone(),
                alive: true,
                partial_week_minutes: 0,
                produced_insight: Insight::ZERO,
            });
        if !progress.alive {
            return Err(ScholarResearchError::ScholarDead);
        }
        let total_minutes = progress
            .partial_week_minutes
            .checked_add(request.completed_minutes)
            .ok_or(ScholarResearchError::Overflow)?;
        let completed_weeks = total_minutes / GAME_MINUTES_PER_WEEK;
        progress.partial_week_minutes = total_minutes % GAME_MINUTES_PER_WEEK;
        let mut credited = Insight::ZERO;
        for _ in 0..completed_weeks {
            let week_credit = insight_for_completed_week(request.modifiers, &next_traits)?;
            let previous_whole =
                progress.produced_insight.micro_insight() / INSIGHT_MICRO_PER_INSIGHT;
            progress.produced_insight = progress
                .produced_insight
                .checked_add(week_credit)
                .ok_or(ScholarResearchError::Overflow)?;
            let current_whole =
                progress.produced_insight.micro_insight() / INSIGHT_MICRO_PER_INSIGHT;
            next_traits.record_insight_produced(current_whole - previous_whole);
            credited = credited
                .checked_add(week_credit)
                .ok_or(ScholarResearchError::Overflow)?;
        }
        if credited != Insight::ZERO {
            next.insight_week_started_tick = Some(request.completed_tick);
            next.generated_this_week = credited;
        }
        next.insight_balance = next
            .insight_balance
            .checked_add(credited)
            .ok_or(ScholarResearchError::Overflow)?;
        let committed_version = next
            .version
            .checked_add(1)
            .ok_or(ScholarResearchError::Overflow)?;
        next.work_events.insert(
            request.id.clone(),
            ScholarWorkEvent {
                id: request.id,
                scholar_id: request.scholar_id,
                completed_minutes: request.completed_minutes,
                research_skill_basis_points: request.modifiers.research_skill.get(),
                scholarship_basis_points: request.modifiers.scholarship.get(),
                credited_insight: credited,
                committed_version,
                completed_tick: request.completed_tick,
            },
        );
        next.version = committed_version;
        next.validate()?;
        *self = next;
        *acquired_traits = next_traits;
        Ok(ScholarWorkOutcome::Recorded)
    }

    /// Legacy Insight-priced preparation retained for compatibility tests.
    /// New preparation must use [`Self::begin_physical_preparation`].
    pub fn prepare_study(
        &mut self,
        catalog: &SyntheticResearchCatalog,
        progress: &ResearchPurchaseState,
        request: PrepareStudyRequest,
    ) -> Result<PreparationOutcome, ScholarResearchError> {
        if let Some(existing) = self.preparation_events.get(&request.id) {
            return if existing.matches(&request) {
                Ok(PreparationOutcome::AlreadyPrepared)
            } else {
                Err(ScholarResearchError::EventIdConflict)
            };
        }
        self.validate()?;
        catalog.validate()?;
        if request.expected_version != self.version {
            return Err(ScholarResearchError::StaleVersion);
        }
        if self.preparations.contains_key(&request.study_id) {
            return Err(ScholarResearchError::AlreadyPrepared);
        }
        if progress.owned_studies.contains(&request.study_id) {
            return Err(ScholarResearchError::StudyAlreadyOwned);
        }
        let scholar = self
            .scholars
            .get(&request.assigned_scholar)
            .ok_or(ScholarResearchError::UnknownScholar)?;
        if !scholar.alive {
            return Err(ScholarResearchError::ScholarDead);
        }
        let study = catalog
            .study(&request.study_id)
            .ok_or(ScholarResearchError::UnknownStudy)?;
        let insight_cost = Insight::from_favor(study.undiscounted_price);
        let remaining = self
            .insight_balance
            .checked_sub(insight_cost)
            .ok_or(ScholarResearchError::InsufficientInsight)?;
        if self.preparations.len() >= MAX_PREPARED_STUDIES
            || self.preparation_events.len() >= MAX_PREPARATION_EVENTS
        {
            return Err(ScholarResearchError::CapacityExceeded);
        }
        let committed_version = self
            .version
            .checked_add(1)
            .ok_or(ScholarResearchError::Overflow)?;
        let prepared = PreparedStudy {
            preparation_id: request.id.clone(),
            study_id: request.study_id.clone(),
            assigned_scholar: Some(request.assigned_scholar.clone()),
            insight_cost,
            required_labor_minutes: 0,
            completed_labor_minutes: 0,
            prepared_version: committed_version,
            prepared_tick: request.prepared_tick,
        };
        let event = PreparationEvent {
            id: request.id.clone(),
            study_id: request.study_id.clone(),
            assigned_scholar: request.assigned_scholar,
            insight_cost,
            required_labor_minutes: 0,
            committed_version,
            prepared_tick: request.prepared_tick,
        };
        self.insight_balance = remaining;
        self.preparations.insert(request.study_id, prepared);
        self.preparation_events.insert(request.id, event);
        self.version = committed_version;
        self.validate()?;
        Ok(PreparationOutcome::Prepared)
    }

    /// Begin physical preparation using exactly 25% of the selected front's
    /// frozen duration terms. Currency remains unfunded until the completed
    /// preparation is atomically consumed; the preparation itself has no
    /// Insight/Notes/Void debit and never stacks.
    pub fn begin_physical_preparation(
        &mut self,
        catalog: &SyntheticResearchCatalog,
        progress: &ResearchPurchaseState,
        selected_front: &GodResearchQueueEntry,
        request: BeginPhysicalPreparationRequest,
    ) -> Result<PreparationOutcome, ScholarResearchError> {
        self.validate()?;
        catalog.validate()?;
        if request.expected_version != self.version {
            return Err(ScholarResearchError::StaleVersion);
        }
        if self.preparation_events.contains_key(&request.id) {
            return Err(ScholarResearchError::EventIdConflict);
        }
        if self.preparations.contains_key(&request.study_id) {
            return Err(ScholarResearchError::AlreadyPrepared);
        }
        if progress.owned_studies.contains(&request.study_id) {
            return Err(ScholarResearchError::StudyAlreadyOwned);
        }
        if catalog.study(&request.study_id).is_none() {
            return Err(ScholarResearchError::UnknownStudy);
        }
        if selected_front.study_id != request.study_id
            || selected_front.frozen
            || selected_front.currency != GodResearchCurrency::Notes
            || progress.god_queue.entries().first() != Some(selected_front)
        {
            return Err(ScholarResearchError::PreparationNotGodFront);
        }
        if !self
            .scholars
            .get(&request.assigned_scholar)
            .is_some_and(|scholar| scholar.alive)
        {
            return Err(ScholarResearchError::UnknownScholar);
        }
        if !request.authorization.permits_work() {
            return Err(ScholarResearchError::ScholarWorkLocked);
        }
        let required_labor_minutes = selected_front
            .frozen_duration_game_minutes
            .saturating_mul(u64::from(PLAYER_PREPARATION_DISCOUNT_BASIS_POINTS))
            .div_ceil(u64::try_from(BASIS_POINTS_SCALE).expect("positive basis-point scale"));
        if required_labor_minutes == 0 {
            return Err(ScholarResearchError::MalformedPersistence);
        }
        if self.preparations.len() >= MAX_PREPARED_STUDIES
            || self.preparation_events.len() >= MAX_PREPARATION_EVENTS
        {
            return Err(ScholarResearchError::CapacityExceeded);
        }
        let committed_version = self
            .version
            .checked_add(1)
            .ok_or(ScholarResearchError::Overflow)?;
        self.preparations.insert(
            request.study_id.clone(),
            PreparedStudy {
                preparation_id: request.id.clone(),
                study_id: request.study_id.clone(),
                assigned_scholar: Some(request.assigned_scholar.clone()),
                insight_cost: Insight::ZERO,
                required_labor_minutes,
                completed_labor_minutes: 0,
                prepared_version: committed_version,
                prepared_tick: request.prepared_tick,
            },
        );
        self.preparation_events.insert(
            request.id.clone(),
            PreparationEvent {
                id: request.id,
                study_id: request.study_id,
                assigned_scholar: request.assigned_scholar,
                insight_cost: Insight::ZERO,
                required_labor_minutes,
                committed_version,
                prepared_tick: request.prepared_tick,
            },
        );
        self.version = committed_version;
        self.validate()?;
        Ok(PreparationOutcome::Prepared)
    }

    pub fn record_physical_preparation_labor(
        &mut self,
        study_id: &StudyId,
        staffed_minutes: u64,
        expected_version: u64,
    ) -> Result<PhysicalPreparationLaborOutcome, ScholarResearchError> {
        self.validate()?;
        if expected_version != self.version {
            return Err(ScholarResearchError::StaleVersion);
        }
        if staffed_minutes == 0 {
            return Ok(PhysicalPreparationLaborOutcome::NoWork);
        }
        let assigned_scholar = self
            .preparations
            .get(study_id)
            .ok_or(ScholarResearchError::PreparationNotFound)?
            .assigned_scholar
            .clone();
        if self
            .preparations
            .get(study_id)
            .is_some_and(|prepared| prepared.required_labor_minutes == 0)
        {
            return Err(ScholarResearchError::PreparationNotPhysical);
        }
        if !assigned_scholar.as_ref().is_some_and(|scholar_id| {
            self.scholars
                .get(scholar_id)
                .is_some_and(|scholar| scholar.alive)
        }) {
            return Err(ScholarResearchError::ScholarDead);
        }
        let prepared = self
            .preparations
            .get_mut(study_id)
            .ok_or(ScholarResearchError::PreparationNotFound)?;
        prepared.completed_labor_minutes = prepared
            .completed_labor_minutes
            .saturating_add(staffed_minutes)
            .min(prepared.required_labor_minutes);
        let remaining = prepared
            .required_labor_minutes
            .saturating_sub(prepared.completed_labor_minutes);
        self.version = self
            .version
            .checked_add(1)
            .ok_or(ScholarResearchError::Overflow)?;
        Ok(if remaining == 0 {
            PhysicalPreparationLaborOutcome::Prepared
        } else {
            PhysicalPreparationLaborOutcome::Advanced {
                remaining_labor_minutes: remaining,
            }
        })
    }

    /// Atomically consume one completed physical preparation while funding the
    /// matching player-selected God front. The voucher discounts Notes by 25%;
    /// Void studies, Leader unlocks, incomplete work, and non-front targets
    /// cannot consume it.
    pub fn fund_prepared_god_front(
        &mut self,
        progress: &mut ResearchPurchaseState,
        funds: &mut ResearchFunds,
        request: FundPreparedGodResearchRequest,
    ) -> Result<GodResearchFundOutcome, ScholarResearchError> {
        self.validate()?;
        if let Some(consumed_study) = self.consumed_preparations.get(&request.id) {
            return if consumed_study == &request.study_id {
                Ok(GodResearchFundOutcome::AlreadyFunded)
            } else {
                Err(ScholarResearchError::EventIdConflict)
            };
        }
        if request.expected_scholar_version != self.version {
            return Err(ScholarResearchError::StaleVersion);
        }
        if request.expected_research_version != progress.version {
            return Err(ScholarResearchError::Purchase(
                ResearchPurchaseError::StaleResearchVersion,
            ));
        }
        let prepared = self
            .preparations
            .get(&request.study_id)
            .ok_or(ScholarResearchError::PreparationNotFound)?;
        if prepared.required_labor_minutes == 0 {
            return Err(ScholarResearchError::PreparationNotPhysical);
        }
        if prepared.completed_labor_minutes != prepared.required_labor_minutes {
            return Err(ScholarResearchError::PreparationIncomplete);
        }
        if !progress
            .god_queue
            .entries()
            .first()
            .is_some_and(|front| front.study_id == request.study_id && !front.frozen)
        {
            return Err(ScholarResearchError::PreparationNotGodFront);
        }
        if self.consumed_preparations.len() >= MAX_CONSUMED_PREPARATIONS {
            return Err(ScholarResearchError::CapacityExceeded);
        }

        let mut next_scholar = self.clone();
        let mut next_progress = progress.clone();
        let mut next_funds = *funds;
        let outcome = next_progress.fund_god_front_with_player_preparation(&mut next_funds)?;
        if outcome != GodResearchFundOutcome::Funded {
            return Err(ScholarResearchError::PreparationNotGodFront);
        }
        next_scholar
            .preparations
            .remove(&request.study_id)
            .ok_or(ScholarResearchError::PreparationNotFound)?;
        next_scholar
            .consumed_preparations
            .insert(request.id, request.study_id);
        next_scholar.version = next_scholar
            .version
            .checked_add(1)
            .ok_or(ScholarResearchError::Overflow)?;
        next_scholar.validate()?;
        *self = next_scholar;
        *progress = next_progress;
        *funds = next_funds;
        Ok(outcome)
    }

    /// Atomically apply a free Leader completion and discard any preparation
    /// attached to the overtaken God front. Currency refunds and research-labor
    /// loss come from the queue completion; this wrapper adds the separately
    /// persisted preparation-labor loss.
    pub fn complete_leader_research(
        &mut self,
        progress: &mut ResearchPurchaseState,
        catalog: &SyntheticResearchCatalog,
        funds: &mut ResearchFunds,
        request: LeaderResearchRequest,
    ) -> Result<LeaderResearchWithPreparationLoss, ScholarResearchError> {
        self.validate()?;
        let mut next_scholar = self.clone();
        let mut next_progress = progress.clone();
        let mut next_funds = *funds;
        let study_id = request.study_id.clone();
        let completion =
            next_progress.complete_leader_research(catalog, &mut next_funds, request)?;
        let removed_preparation = next_scholar.preparations.remove(&study_id);
        let lost_preparation_labor_minutes = removed_preparation
            .as_ref()
            .map_or(0, |prepared| prepared.completed_labor_minutes);
        if removed_preparation.is_some() {
            next_scholar.version = next_scholar
                .version
                .checked_add(1)
                .ok_or(ScholarResearchError::Overflow)?;
        }
        next_scholar.validate()?;
        *self = next_scholar;
        *progress = next_progress;
        *funds = next_funds;
        Ok(LeaderResearchWithPreparationLoss {
            completion,
            lost_preparation_labor_minutes,
        })
    }

    pub fn record_scholar_death(
        &mut self,
        scholar_id: &ScholarId,
    ) -> Result<usize, ScholarResearchError> {
        self.validate()?;
        let scholar = self
            .scholars
            .get_mut(scholar_id)
            .ok_or(ScholarResearchError::UnknownScholar)?;
        if !scholar.alive {
            return Ok(0);
        }
        scholar.alive = false;
        let mut released = 0;
        for prepared in self.preparations.values_mut() {
            if prepared.assigned_scholar.as_ref() == Some(scholar_id) {
                prepared.assigned_scholar = None;
                released += 1;
            }
        }
        self.version = self
            .version
            .checked_add(1)
            .ok_or(ScholarResearchError::Overflow)?;
        Ok(released)
    }

    pub fn reassign_preparation(
        &mut self,
        study_id: &StudyId,
        successor: ScholarId,
        expected_version: u64,
    ) -> Result<(), ScholarResearchError> {
        self.validate()?;
        if expected_version != self.version {
            return Err(ScholarResearchError::StaleVersion);
        }
        if !self
            .scholars
            .get(&successor)
            .is_some_and(|scholar| scholar.alive)
        {
            return Err(ScholarResearchError::UnknownScholar);
        }
        let prepared = self
            .preparations
            .get_mut(study_id)
            .ok_or(ScholarResearchError::PreparationNotFound)?;
        prepared.assigned_scholar = Some(successor);
        self.version = self
            .version
            .checked_add(1)
            .ok_or(ScholarResearchError::Overflow)?;
        Ok(())
    }

    pub fn select_preparation_target(
        &self,
        catalog: &SyntheticResearchCatalog,
        progress: &ResearchPurchaseState,
        approved_plan_dependencies: &BTreeSet<StudyId>,
    ) -> Result<StudyId, ScholarResearchError> {
        self.validate()?;
        catalog.validate()?;
        let mut candidates = catalog
            .studies
            .iter()
            .filter(|study| {
                !progress.owned_studies.contains(&study.id)
                    && !self.preparations.contains_key(&study.id)
            })
            .map(|study| {
                (
                    !approved_plan_dependencies.contains(&study.id),
                    study.id.clone(),
                )
            })
            .collect::<Vec<_>>();
        candidates.sort();
        candidates
            .first()
            .map(|(_, study_id)| study_id.clone())
            .ok_or(ScholarResearchError::NoPreparationCandidate)
    }

    /// Legacy Favor purchase retained outside both authoritative LAI.58 lanes.
    pub fn player_purchase(
        &mut self,
        progress: &mut ResearchPurchaseState,
        ledger: &mut FavorLedger,
        catalog: &SyntheticResearchCatalog,
        request: ScholarPlayerPurchaseRequest,
    ) -> Result<ResearchPurchaseOutcome, ScholarResearchError> {
        self.validate()?;
        let consumed = self.consumed_preparations.get(&request.id);
        let is_purchase_replay = progress.purchases.contains_key(&request.id);
        if request.expected_scholar_version != self.version && !is_purchase_replay {
            return Err(ScholarResearchError::StaleVersion);
        }
        if let Some(consumed_study) = consumed
            && consumed_study != &request.study_id
        {
            return Err(ScholarResearchError::EventIdConflict);
        }
        if request.use_preparation
            && consumed.is_none()
            && !self.preparations.contains_key(&request.study_id)
        {
            return Err(ScholarResearchError::PreparationNotFound);
        }
        if request.use_preparation
            && self
                .preparations
                .get(&request.study_id)
                .is_some_and(|prepared| {
                    prepared.required_labor_minutes > prepared.completed_labor_minutes
                })
        {
            return Err(ScholarResearchError::PreparationIncomplete);
        }

        let mut next_scholar = self.clone();
        let mut next_progress = progress.clone();
        let mut next_ledger = ledger.clone();
        let outcome = next_progress.player_purchase(
            &mut next_ledger,
            catalog,
            PlayerResearchPurchaseRequest {
                id: request.id.clone(),
                colony_id: request.colony_id,
                study_id: request.study_id.clone(),
                expected_research_version: request.expected_research_version,
                expected_favor_version: request.expected_favor_version,
                discount_basis_points: if request.use_preparation {
                    PLAYER_PREPARATION_DISCOUNT_BASIS_POINTS
                } else {
                    0
                },
                consume_preparation: request.use_preparation,
                now_tick: request.now_tick,
            },
        )?;
        if request.use_preparation && outcome == ResearchPurchaseOutcome::Committed {
            next_scholar
                .preparations
                .remove(&request.study_id)
                .ok_or(ScholarResearchError::PreparationNotFound)?;
            if next_scholar.consumed_preparations.len() >= MAX_CONSUMED_PREPARATIONS {
                return Err(ScholarResearchError::CapacityExceeded);
            }
            next_scholar
                .consumed_preparations
                .insert(request.id, request.study_id);
            next_scholar.version = next_scholar
                .version
                .checked_add(1)
                .ok_or(ScholarResearchError::Overflow)?;
        } else if request.use_preparation && consumed.is_none() {
            return Err(ScholarResearchError::PreparationNotFound);
        }
        next_scholar.validate()?;
        *self = next_scholar;
        *progress = next_progress;
        *ledger = next_ledger;
        Ok(outcome)
    }

    fn validate(&self) -> Result<(), ScholarResearchError> {
        if self.schema_version != SCHOLAR_RESEARCH_SCHEMA_VERSION
            || self.scholars.len() > MAX_SCHOLARS
            || self.preparations.len() > MAX_PREPARED_STUDIES
            || self.preparation_events.len() > MAX_PREPARATION_EVENTS
            || self.work_events.len() > MAX_SCHOLAR_WORK_EVENTS
            || self.consumed_preparations.len() > MAX_CONSUMED_PREPARATIONS
        {
            return Err(ScholarResearchError::MalformedPersistence);
        }
        for (id, scholar) in &self.scholars {
            if id != &scholar.scholar_id {
                return Err(ScholarResearchError::MalformedPersistence);
            }
            scholar.validate()?;
        }
        for (study_id, prepared) in &self.preparations {
            if study_id != &prepared.study_id
                || (prepared.insight_cost == Insight::ZERO
                    && (prepared.required_labor_minutes == 0
                        || prepared.completed_labor_minutes > prepared.required_labor_minutes))
                || (prepared.insight_cost != Insight::ZERO && prepared.required_labor_minutes != 0)
                || !self
                    .preparation_events
                    .contains_key(&prepared.preparation_id)
                || prepared
                    .assigned_scholar
                    .as_ref()
                    .is_some_and(|scholar_id| {
                        !self
                            .scholars
                            .get(scholar_id)
                            .is_some_and(|scholar| scholar.alive)
                    })
            {
                return Err(ScholarResearchError::MalformedPersistence);
            }
        }
        for (id, event) in &self.preparation_events {
            if id != &event.id
                || (event.insight_cost == Insight::ZERO && event.required_labor_minutes == 0)
                || (event.insight_cost != Insight::ZERO && event.required_labor_minutes != 0)
            {
                return Err(ScholarResearchError::MalformedPersistence);
            }
        }
        for (id, event) in &self.work_events {
            if id != &event.id {
                return Err(ScholarResearchError::MalformedPersistence);
            }
            event.validate()?;
        }
        let mut versions = self
            .work_events
            .values()
            .map(|event| event.committed_version)
            .chain(
                self.preparation_events
                    .values()
                    .map(|event| event.committed_version),
            )
            .collect::<Vec<_>>();
        versions.sort_unstable();
        if versions.windows(2).any(|pair| pair[0] == pair[1])
            || versions
                .last()
                .is_some_and(|version| *version > self.version)
        {
            return Err(ScholarResearchError::MalformedPersistence);
        }
        Ok(())
    }
}

impl Default for ScholarResearchState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedScholarResearchState {
    schema_version: u32,
    #[serde(default)]
    version: u64,
    #[serde(default)]
    insight_balance: Insight,
    #[serde(default)]
    insight_week_started_tick: Option<u64>,
    #[serde(default)]
    generated_this_week: Insight,
    #[serde(default)]
    scholars: BTreeMap<ScholarId, ScholarProgress>,
    #[serde(default)]
    preparations: BTreeMap<StudyId, PreparedStudy>,
    #[serde(default)]
    preparation_events: BTreeMap<PreparationId, PreparationEvent>,
    #[serde(default)]
    work_events: BTreeMap<ScholarWorkEventId, ScholarWorkEvent>,
    #[serde(default)]
    consumed_preparations: BTreeMap<ResearchPurchaseId, StudyId>,
}

impl<'de> Deserialize<'de> for ScholarResearchState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = UncheckedScholarResearchState::deserialize(deserializer)?;
        let state = Self {
            schema_version: raw.schema_version,
            version: raw.version,
            insight_balance: raw.insight_balance,
            insight_week_started_tick: raw.insight_week_started_tick,
            generated_this_week: raw.generated_this_week,
            scholars: raw.scholars,
            preparations: raw.preparations,
            preparation_events: raw.preparation_events,
            work_events: raw.work_events,
            consumed_preparations: raw.consumed_preparations,
        };
        state.validate().map_err(serde::de::Error::custom)?;
        Ok(state)
    }
}

fn insight_for_completed_week(
    modifiers: ScholarWorkModifiers,
    acquired_traits: &AcquiredTraitState,
) -> Result<Insight, ScholarResearchError> {
    let base = u128::from(INSIGHT_PER_COMPLETED_WEEK)
        .checked_mul(u128::from(INSIGHT_MICRO_PER_INSIGHT))
        .ok_or(ScholarResearchError::Overflow)?;
    let skill = u128::try_from(modifiers.research_skill.get())
        .map_err(|_| ScholarResearchError::InvalidModifier)?;
    let scholarship = u128::try_from(modifiers.scholarship.get())
        .map_err(|_| ScholarResearchError::InvalidModifier)?;
    let seasoned = u128::try_from(acquired_traits.insight_production_factor().get())
        .map_err(|_| ScholarResearchError::InvalidModifier)?;
    let scale = u128::try_from(BASIS_POINTS_SCALE).expect("positive basis-point scale");
    let denominator = scale.pow(3);
    let micro = base
        .checked_mul(skill)
        .and_then(|value| value.checked_mul(scholarship))
        .and_then(|value| value.checked_mul(seasoned))
        .ok_or(ScholarResearchError::Overflow)?
        / denominator;
    let micro = u64::try_from(micro).map_err(|_| ScholarResearchError::Overflow)?;
    Ok(Insight::from_micro_insight(micro))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResearchTrackStages {
    pub divine_duration: u8,
    pub divine_economy: u8,
    pub rehabilitation: u8,
    pub administration: u8,
}

impl ResearchTrackStages {
    pub fn from_progress(progress: &ResearchPurchaseState) -> Result<Self, ScholarResearchError> {
        Ok(Self {
            divine_duration: contiguous_stage(progress, "divine_duration")?,
            divine_economy: contiguous_stage(progress, "divine_economy")?,
            rehabilitation: contiguous_stage(progress, "rehabilitation")?,
            administration: contiguous_stage(progress, "administration")?,
        })
    }

    #[must_use]
    pub fn effects(self) -> ResearchRuntimeEffects {
        let duration_stage = self.divine_duration.min(11);
        let economy_stage = self.divine_economy.min(11);
        let rehabilitation_stage = self.rehabilitation.min(11);
        let administration_stage = self.administration.min(11);
        ResearchRuntimeEffects {
            divine_boost_stages: DivineBoostResearchStages {
                divine_duration_stage: duration_stage,
                divine_economy_stage: economy_stage,
            },
            max_divine_duration_game_hours: DIVINE_DURATION_ALLOWED_GAME_HOURS
                [usize::from(duration_stage)],
            divine_economy_discount_basis_points: stage_value(
                economy_stage,
                &DIVINE_ECONOMY_STAGE_DISCOUNT_BASIS_POINTS,
                0,
            ),
            rehabilitation_bonus_basis_points: u16::from(stage_value(
                rehabilitation_stage,
                &REHABILITATION_STAGE_RESTORATION_PERCENTAGE_POINTS,
                0,
            )) * 100,
            standing_order_slots: stage_value(
                administration_stage,
                &ADMINISTRATION_STAGE_STANDING_ORDER_SLOTS,
                ADMINISTRATION_BASE_STANDING_ORDER_SLOTS,
            ),
            strategic_intent_slots: stage_value(
                administration_stage,
                &ADMINISTRATION_STAGE_STRATEGIC_INTENT_SLOTS,
                ADMINISTRATION_BASE_STRATEGIC_INTENT_SLOTS,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResearchRuntimeEffects {
    pub divine_boost_stages: DivineBoostResearchStages,
    pub max_divine_duration_game_hours: u8,
    pub divine_economy_discount_basis_points: u16,
    pub rehabilitation_bonus_basis_points: u16,
    pub standing_order_slots: u8,
    pub strategic_intent_slots: u8,
}

fn stage_value<T: Copy, const N: usize>(stage: u8, values: &[T; N], base: T) -> T {
    if stage == 0 {
        base
    } else {
        values[usize::from(stage - 1)]
    }
}

fn contiguous_stage(
    progress: &ResearchPurchaseState,
    track_prefix: &str,
) -> Result<u8, ScholarResearchError> {
    let mut highest = 0;
    let mut gap = false;
    for stage in 1..=11 {
        let id = StudyId::derive(&format!("{track_prefix}_stage_{stage:02}"));
        if progress.owned_studies.contains(&id) {
            if gap {
                return Err(ScholarResearchError::NonContiguousResearchTrack);
            }
            highest = stage;
        } else {
            gap = true;
        }
    }
    Ok(highest)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScholarResearchError {
    ScholarWorkLocked,
    ScholarDead,
    UnknownScholar,
    UnknownStudy,
    StudyAlreadyOwned,
    AlreadyPrepared,
    PreparationNotFound,
    PreparationNotPhysical,
    PreparationNotGodFront,
    PreparationIncomplete,
    NoPreparationCandidate,
    InsufficientInsight,
    StaleVersion,
    EventIdConflict,
    ZeroCompletedWork,
    InvalidModifier,
    NonContiguousResearchTrack,
    CapacityExceeded,
    MalformedPersistence,
    Overflow,
    Purchase(ResearchPurchaseError),
}

impl From<ResearchPurchaseError> for ScholarResearchError {
    fn from(value: ResearchPurchaseError) -> Self {
        Self::Purchase(value)
    }
}

impl std::fmt::Display for ScholarResearchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Scholar research error: {self:?}")
    }
}

impl std::error::Error for ScholarResearchError {}
