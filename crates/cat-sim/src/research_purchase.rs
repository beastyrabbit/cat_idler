//! LAI.58 research-lane leaf specified by
//! `docs/branch-plan-merge/bug-gui-design-BOARD.md`.
//!
//! The legacy Favor path is retained only to deserialize the shared dirty
//! worktree during clean-reset migration.  New authoritative orchestration is
//! the physical God queue plus free Leader lane defined in this module.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    favor::{Favor, FavorCommitOutcome, FavorError, FavorEventId, FavorEventKind, FavorLedger},
    leader_planner::EffectiveLevel,
    planner_core::{BASIS_POINTS_SCALE, PlannerId},
    research_manifest::research_manifest,
};

pub const RESEARCH_PURCHASE_SCHEMA_VERSION: u32 = 1;
pub const AUTOMATIC_RESEARCH_WINDOW_GAME_MINUTES: u64 = 7 * 24 * 60;
pub const MAX_SYNTHETIC_STUDIES: usize = 2_048;
pub const MAX_PREREQUISITES_PER_STUDY: usize = 16;
pub const MAX_PURCHASE_EVENTS: usize = 512;
pub const MAX_AUTOMATIC_QUOTA_TIMESTAMPS: usize = 32;
pub const PLAYER_PREPARATION_DISCOUNT_BASIS_POINTS: u16 = 2_500;
pub const MAX_GOD_RESEARCH_QUEUE: usize = 64;
pub const PREPARATION_LABOR_BASIS_POINTS: u16 = 2_500;
pub const LEADER_DUPLICATE_OOPSIE_PERCENT_BY_EXPERTISE: [u8; 5] = [25, 12, 5, 1, 0];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StudyId(PlannerId);

impl StudyId {
    #[must_use]
    pub fn derive(stable_id: &str) -> Self {
        Self(PlannerId::derive("study", [stable_id]))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyntheticStudyDescriptor {
    pub id: StudyId,
    pub display_name: String,
    pub prerequisites: Vec<StudyId>,
    pub undiscounted_price: Favor,
    pub tags: BTreeSet<PlannerId>,
}

impl SyntheticStudyDescriptor {
    fn validate(&self) -> Result<(), ResearchPurchaseError> {
        if self.display_name.trim().is_empty()
            || self.undiscounted_price == Favor::ZERO
            || self.prerequisites.len() > MAX_PREREQUISITES_PER_STUDY
            || self.prerequisites.contains(&self.id)
            || self.prerequisites.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(ResearchPurchaseError::MalformedCatalog);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyntheticResearchCatalog {
    pub schema_version: u32,
    pub studies: Vec<SyntheticStudyDescriptor>,
    #[serde(default)]
    pub repeatable_studies: BTreeSet<StudyId>,
}

impl SyntheticResearchCatalog {
    #[must_use]
    pub fn new(studies: Vec<SyntheticStudyDescriptor>) -> Self {
        Self {
            schema_version: RESEARCH_PURCHASE_SCHEMA_VERSION,
            studies,
            repeatable_studies: BTreeSet::new(),
        }
    }

    #[must_use]
    pub fn with_repeatable_studies(mut self, repeatable_studies: BTreeSet<StudyId>) -> Self {
        self.repeatable_studies = repeatable_studies;
        self
    }

    pub fn validate(&self) -> Result<(), ResearchPurchaseError> {
        if self.schema_version != RESEARCH_PURCHASE_SCHEMA_VERSION
            || self.studies.is_empty()
            || self.studies.len() > MAX_SYNTHETIC_STUDIES
        {
            return Err(ResearchPurchaseError::MalformedCatalog);
        }
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        let mut by_id = BTreeMap::new();
        for study in &self.studies {
            study.validate()?;
            if !ids.insert(study.id.clone()) || !names.insert(study.display_name.clone()) {
                return Err(ResearchPurchaseError::MalformedCatalog);
            }
            by_id.insert(study.id.clone(), study);
        }
        if !self
            .repeatable_studies
            .iter()
            .all(|study_id| by_id.contains_key(study_id))
        {
            return Err(ResearchPurchaseError::MalformedCatalog);
        }
        for study in &self.studies {
            if study
                .prerequisites
                .iter()
                .any(|prerequisite| !by_id.contains_key(prerequisite))
            {
                return Err(ResearchPurchaseError::MalformedCatalog);
            }
        }
        for study in &self.studies {
            let mut visiting = BTreeSet::new();
            let mut visited = BTreeSet::new();
            validate_acyclic(&study.id, &by_id, &mut visiting, &mut visited)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn study(&self, study_id: &StudyId) -> Option<&SyntheticStudyDescriptor> {
        self.studies.iter().find(|study| &study.id == study_id)
    }

    #[must_use]
    pub fn is_repeatable(&self, study_id: &StudyId) -> bool {
        self.repeatable_studies.contains(study_id)
    }

    fn has_unowned_finite_frontier(&self, progress: &ResearchPurchaseState) -> bool {
        self.studies.iter().any(|study| {
            !self.is_repeatable(&study.id)
                && !progress.owned_studies.contains(&study.id)
                && study
                    .prerequisites
                    .iter()
                    .all(|prerequisite| progress.owned_studies.contains(prerequisite))
        })
    }

    pub fn frontier(
        &self,
        progress: &ResearchPurchaseState,
    ) -> Result<Vec<SyntheticStudyDescriptor>, ResearchPurchaseError> {
        self.validate()?;
        progress.validate()?;
        let mut frontier = self
            .studies
            .iter()
            .filter(|study| {
                !progress.owned_studies.contains(&study.id)
                    && study
                        .prerequisites
                        .iter()
                        .all(|prerequisite| progress.owned_studies.contains(prerequisite))
            })
            .cloned()
            .collect::<Vec<_>>();
        frontier.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(frontier)
    }
}

static CANONICAL_RESEARCH_CATALOG: OnceLock<SyntheticResearchCatalog> = OnceLock::new();

/// Legacy-compatible descriptor view of the canonical manifest. `Favor` is
/// only the old fixed-point storage type here; authoritative God funding below
/// uses Notes/Void and Leader completion is free.
#[must_use]
pub fn canonical_research_catalog() -> &'static SyntheticResearchCatalog {
    CANONICAL_RESEARCH_CATALOG.get_or_init(|| {
        let manifest = research_manifest();
        let studies = manifest
            .studies()
            .iter()
            .map(|study| {
                let whole_favor = study.cost_units;
                let mut prerequisites = study
                    .prerequisites
                    .iter()
                    .map(|id| StudyId::derive(id))
                    .collect::<Vec<_>>();
                prerequisites.sort();
                SyntheticStudyDescriptor {
                    id: StudyId::derive(&study.stable_id),
                    display_name: study.display_name.clone(),
                    prerequisites,
                    undiscounted_price: Favor::from_whole(whole_favor)
                        .expect("bounded embedded research prices fit Favor"),
                    tags: BTreeSet::new(),
                }
            })
            .collect();
        let repeatable_studies = manifest
            .studies()
            .iter()
            .filter(|study| study.repeatable_terminal)
            .map(|study| StudyId::derive(&study.stable_id))
            .collect();
        let catalog =
            SyntheticResearchCatalog::new(studies).with_repeatable_studies(repeatable_studies);
        catalog
            .validate()
            .expect("canonical manifest converts to a valid purchase catalog");
        catalog
    })
}

fn validate_acyclic(
    study_id: &StudyId,
    by_id: &BTreeMap<StudyId, &SyntheticStudyDescriptor>,
    visiting: &mut BTreeSet<StudyId>,
    visited: &mut BTreeSet<StudyId>,
) -> Result<(), ResearchPurchaseError> {
    if visited.contains(study_id) {
        return Ok(());
    }
    if !visiting.insert(study_id.clone()) {
        return Err(ResearchPurchaseError::MalformedCatalog);
    }
    let study = by_id
        .get(study_id)
        .ok_or(ResearchPurchaseError::MalformedCatalog)?;
    for prerequisite in &study.prerequisites {
        validate_acyclic(prerequisite, by_id, visiting, visited)?;
    }
    visiting.remove(study_id);
    visited.insert(study_id.clone());
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResearchPurchaseId(PlannerId);

impl ResearchPurchaseId {
    #[must_use]
    pub fn derive(namespace: &str, colony_id: &PlannerId, action_id: &str) -> Self {
        Self(PlannerId::derive(
            "research_purchase",
            [namespace, colony_id.as_str(), action_id],
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchPurchaseSource {
    Player,
    Automatic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResearchPurchaseEvent {
    pub id: ResearchPurchaseId,
    pub source: ResearchPurchaseSource,
    pub study_id: StudyId,
    pub undiscounted_price: Favor,
    pub charged_price: Favor,
    pub discount_basis_points: u16,
    pub consumed_preparation: bool,
    pub favor_event_id: FavorEventId,
    pub committed_research_version: u64,
    pub committed_favor_version: u64,
    pub committed_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResearchPurchaseOutcome {
    Committed,
    AlreadyCommitted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchPurchaseState {
    pub schema_version: u32,
    pub version: u64,
    pub owned_studies: BTreeSet<StudyId>,
    pub repeatable_completions: BTreeMap<StudyId, u32>,
    pub purchases: BTreeMap<ResearchPurchaseId, ResearchPurchaseEvent>,
    pub automatic_quota: AutomaticResearchQuotaState,
    pub god_queue: GodResearchQueueState,
    pub leader_lane: LeaderResearchLaneState,
}

impl ResearchPurchaseState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: RESEARCH_PURCHASE_SCHEMA_VERSION,
            version: 0,
            owned_studies: BTreeSet::new(),
            repeatable_completions: BTreeMap::new(),
            purchases: BTreeMap::new(),
            automatic_quota: AutomaticResearchQuotaState::new(),
            god_queue: GodResearchQueueState::new(),
            leader_lane: LeaderResearchLaneState::new(),
        }
    }

    /// Legacy Favor compatibility entry point retained for the pre-cutover
    /// tests/save reader. It is not part of either authoritative LAI.58 lane.
    pub fn player_purchase(
        &mut self,
        ledger: &mut FavorLedger,
        catalog: &SyntheticResearchCatalog,
        request: PlayerResearchPurchaseRequest,
    ) -> Result<ResearchPurchaseOutcome, ResearchPurchaseError> {
        self.commit_purchase(
            ledger,
            catalog,
            CommitResearchPurchaseRequest {
                id: request.id,
                colony_id: request.colony_id,
                source: ResearchPurchaseSource::Player,
                study_id: request.study_id,
                expected_research_version: request.expected_research_version,
                expected_favor_version: request.expected_favor_version,
                discount_basis_points: request.discount_basis_points,
                consume_preparation: request.consume_preparation,
                now_tick: request.now_tick,
            },
        )
        .map(|outcome| outcome.outcome)
    }

    /// Legacy Favor-backed automatic purchase retained outside the LAI.58
    /// Leader lane. New simulation integration must use
    /// [`Self::complete_leader_research`].
    pub fn automatic_purchase(
        &mut self,
        ledger: &mut FavorLedger,
        catalog: &SyntheticResearchCatalog,
        request: AutomaticResearchPurchaseRequest,
    ) -> Result<AutomaticResearchPurchaseOutcome, ResearchPurchaseError> {
        if let Some(existing) = self.purchases.get(&request.id) {
            return Ok(AutomaticResearchPurchaseOutcome {
                outcome: ResearchPurchaseOutcome::AlreadyCommitted,
                study_id: existing.study_id.clone(),
                score: None,
                quota_limit: request.effective_loremaster.quota_limit(),
                quota_used_after: self.automatic_quota.used_in_window(request.now_tick),
            });
        }
        self.validate()?;
        catalog.validate()?;
        let quota_limit = request.effective_loremaster.quota_limit();
        if self.automatic_quota.used_in_window(request.now_tick) >= quota_limit {
            return Err(ResearchPurchaseError::AutomaticQuotaExhausted);
        }
        let mut candidates = catalog
            .frontier(self)?
            .into_iter()
            .filter(|study| study.undiscounted_price <= ledger.balance)
            .map(|study| {
                let score = request
                    .scores
                    .get(&study.id)
                    .copied()
                    .unwrap_or_default()
                    .total_score();
                (study, score)
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return Err(ResearchPurchaseError::NoAffordableFrontier);
        }
        candidates.sort_by(|(left, left_score), (right, right_score)| {
            right_score
                .cmp(left_score)
                .then_with(|| left.id.cmp(&right.id))
        });
        let (selected, score) = candidates
            .first()
            .cloned()
            .expect("non-empty candidates have a best item");
        let committed = self.commit_purchase(
            ledger,
            catalog,
            CommitResearchPurchaseRequest {
                id: request.id,
                colony_id: request.colony_id,
                source: ResearchPurchaseSource::Automatic,
                study_id: selected.id,
                expected_research_version: request.expected_research_version,
                expected_favor_version: request.expected_favor_version,
                discount_basis_points: 0,
                consume_preparation: false,
                now_tick: request.now_tick,
            },
        )?;
        if committed.outcome == ResearchPurchaseOutcome::Committed {
            self.automatic_quota.record_commit(request.now_tick)?;
        }
        Ok(AutomaticResearchPurchaseOutcome {
            outcome: committed.outcome,
            study_id: committed.study_id,
            score: Some(score),
            quota_limit,
            quota_used_after: self.automatic_quota.used_in_window(request.now_tick),
        })
    }

    /// Queue a God target and every missing prerequisite in topological order.
    /// Nothing is spent here: only the physical front is eligible to freeze
    /// Notes or Void Insight.
    pub fn queue_god_target(
        &mut self,
        catalog: &SyntheticResearchCatalog,
        terms: &GodResearchTerms,
        study_id: StudyId,
    ) -> Result<Vec<StudyId>, ResearchPurchaseError> {
        self.validate()?;
        catalog.validate()?;
        let queued = self.god_queue.queue_path(
            catalog,
            terms,
            &self.owned_studies,
            &self.repeatable_completions,
            study_id,
        )?;
        if !queued.is_empty() {
            self.version = self
                .version
                .checked_add(1)
                .ok_or(ResearchPurchaseError::Overflow)?;
        }
        Ok(queued)
    }

    /// Freeze the front study's currency once.  Queue position, disconnects,
    /// and offline time cannot alter these terms.
    pub fn fund_god_front(
        &mut self,
        funds: &mut ResearchFunds,
    ) -> Result<GodResearchFundOutcome, ResearchPurchaseError> {
        self.validate()?;
        let outcome = self.god_queue.fund_front(funds, 0)?;
        if outcome == GodResearchFundOutcome::Funded {
            self.version = self
                .version
                .checked_add(1)
                .ok_or(ResearchPurchaseError::Overflow)?;
        }
        Ok(outcome)
    }

    pub(crate) fn fund_god_front_with_player_preparation(
        &mut self,
        funds: &mut ResearchFunds,
    ) -> Result<GodResearchFundOutcome, ResearchPurchaseError> {
        self.validate()?;
        let outcome = self
            .god_queue
            .fund_front(funds, PLAYER_PREPARATION_DISCOUNT_BASIS_POINTS)?;
        if outcome == GodResearchFundOutcome::Funded {
            self.version = self
                .version
                .checked_add(1)
                .ok_or(ResearchPurchaseError::Overflow)?;
        }
        Ok(outcome)
    }

    /// Credit staffed physical research labor to the funded front.  Completing
    /// a front makes it owned and leaves the next target unfunded.
    pub fn record_god_research_labor(
        &mut self,
        catalog: &SyntheticResearchCatalog,
        authorization: GodResearchWorkAuthorization,
        staffed_minutes: u64,
    ) -> Result<GodResearchLaborOutcome, ResearchPurchaseError> {
        self.validate()?;
        catalog.validate()?;
        if !authorization.permits_work() {
            return Err(ResearchPurchaseError::GodResearchInfrastructureUnavailable);
        }
        let outcome = self.god_queue.record_front_labor(staffed_minutes)?;
        if let GodResearchLaborOutcome::Completed(study_id) = &outcome {
            if catalog.is_repeatable(study_id) {
                let completions = self
                    .repeatable_completions
                    .entry(study_id.clone())
                    .or_default();
                *completions = completions
                    .checked_add(1)
                    .ok_or(ResearchPurchaseError::Overflow)?;
            } else {
                self.owned_studies.insert(study_id.clone());
            }
        }
        if !matches!(outcome, GodResearchLaborOutcome::NoWork) {
            self.version = self
                .version
                .checked_add(1)
                .ok_or(ResearchPurchaseError::Overflow)?;
        }
        Ok(outcome)
    }

    pub fn reorder_god_target(
        &mut self,
        catalog: &SyntheticResearchCatalog,
        study_id: &StudyId,
        to_index: usize,
    ) -> Result<(), ResearchPurchaseError> {
        self.validate()?;
        self.god_queue
            .reorder(catalog, &self.owned_studies, study_id, to_index)?;
        self.version = self
            .version
            .checked_add(1)
            .ok_or(ResearchPurchaseError::Overflow)?;
        Ok(())
    }

    pub fn remove_god_target(
        &mut self,
        catalog: &SyntheticResearchCatalog,
        funds: &mut ResearchFunds,
        study_id: &StudyId,
    ) -> Result<GodResearchRemoval, ResearchPurchaseError> {
        self.validate()?;
        let removal =
            self.god_queue
                .remove_with_dependents(catalog, &self.owned_studies, study_id, funds)?;
        self.version = self
            .version
            .checked_add(1)
            .ok_or(ResearchPurchaseError::Overflow)?;
        Ok(removal)
    }

    /// The free Leader lane deliberately has no ledger, queue slot, scholar,
    /// building, or timer dependency.  An overtake tears down the corresponding
    /// physical God queue entries, refunding frozen currency while preserving
    /// the visible lost-labor accounting.
    pub fn complete_leader_research(
        &mut self,
        catalog: &SyntheticResearchCatalog,
        funds: &mut ResearchFunds,
        request: LeaderResearchRequest,
    ) -> Result<LeaderResearchCompletion, ResearchPurchaseError> {
        self.validate()?;
        catalog.validate()?;
        if request.expected_research_version != self.version {
            return Err(ResearchPurchaseError::StaleResearchVersion);
        }
        if self.owned_studies.contains(&request.study_id) {
            return Err(ResearchPurchaseError::AlreadyOwned);
        }
        if catalog.is_repeatable(&request.study_id) && catalog.has_unowned_finite_frontier(self) {
            return Err(ResearchPurchaseError::FiniteResearchRemaining);
        }
        let study = catalog
            .study(&request.study_id)
            .ok_or(ResearchPurchaseError::UnknownStudy)?;
        if study
            .prerequisites
            .iter()
            .any(|prerequisite| !self.owned_studies.contains(prerequisite))
        {
            return Err(ResearchPurchaseError::NotFrontier);
        }
        let god_target = self.god_queue.entry(&request.study_id);
        request
            .duplicate_authorization
            .validate(god_target.is_some())?;
        self.leader_lane.record(&request)?;
        let overtake = if god_target.is_some() {
            Some(self.god_queue.remove_with_dependents(
                catalog,
                &self.owned_studies,
                &request.study_id,
                funds,
            )?)
        } else {
            None
        };
        if catalog.is_repeatable(&request.study_id) {
            let completions = self
                .repeatable_completions
                .entry(request.study_id.clone())
                .or_default();
            *completions = completions
                .checked_add(1)
                .ok_or(ResearchPurchaseError::Overflow)?;
        } else {
            self.owned_studies.insert(request.study_id.clone());
        }
        self.version = self
            .version
            .checked_add(1)
            .ok_or(ResearchPurchaseError::Overflow)?;
        Ok(LeaderResearchCompletion {
            study_id: request.study_id,
            duplicate_authorization: request.duplicate_authorization,
            event_kind: request.duplicate_authorization.into(),
            overtake,
        })
    }

    /// Select an eligible Leader target from report-derived scores.  Funded
    /// God work is excluded except for an explicit emergency/oopsie; queued
    /// work remains eligible but loses score proportional to estimated wait.
    pub fn select_leader_target(
        &self,
        catalog: &SyntheticResearchCatalog,
        candidates: &[LeaderResearchCandidate],
        duplicate_authorization: LeaderDuplicateAuthorization,
    ) -> Result<Option<StudyId>, ResearchPurchaseError> {
        self.validate()?;
        catalog.validate()?;
        if candidates
            .iter()
            .any(|candidate| candidate.repeatable != catalog.is_repeatable(&candidate.study_id))
        {
            return Err(ResearchPurchaseError::MalformedRequest);
        }
        let finite_available = catalog.has_unowned_finite_frontier(self);
        let mut ranked = candidates
            .iter()
            .filter(|candidate| !finite_available || !candidate.repeatable)
            .filter(|candidate| {
                catalog.study(&candidate.study_id).is_some_and(|study| {
                    !self.owned_studies.contains(&candidate.study_id)
                        && study
                            .prerequisites
                            .iter()
                            .all(|prerequisite| self.owned_studies.contains(prerequisite))
                })
            })
            .filter_map(|candidate| {
                let queue = self.god_queue.entry(&candidate.study_id);
                if queue.is_some_and(|entry| entry.frozen)
                    && !duplicate_authorization.permits_duplicate()
                {
                    return None;
                }
                let down_rank = self
                    .god_queue
                    .estimated_wait_minutes(&candidate.study_id)
                    .map_or(0, |minutes| i64::try_from(minutes).unwrap_or(i64::MAX));
                Some((
                    candidate
                        .decision_inputs
                        .total_score()
                        .saturating_sub(down_rank),
                    candidate.study_id.clone(),
                ))
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        Ok(ranked.first().map(|(_, study_id)| study_id.clone()))
    }

    fn commit_purchase(
        &mut self,
        ledger: &mut FavorLedger,
        catalog: &SyntheticResearchCatalog,
        request: CommitResearchPurchaseRequest,
    ) -> Result<CommittedResearchPurchase, ResearchPurchaseError> {
        self.validate()?;
        catalog.validate()?;
        if let Some(existing) = self.purchases.get(&request.id) {
            return if event_matches_request(existing, &request) {
                Ok(CommittedResearchPurchase {
                    outcome: ResearchPurchaseOutcome::AlreadyCommitted,
                    study_id: existing.study_id.clone(),
                })
            } else {
                Err(ResearchPurchaseError::PurchaseIdConflict)
            };
        }
        if request.expected_research_version != self.version {
            return Err(ResearchPurchaseError::StaleResearchVersion);
        }
        let valid_terms = match request.source {
            ResearchPurchaseSource::Player => matches!(
                (request.discount_basis_points, request.consume_preparation),
                (0, false) | (PLAYER_PREPARATION_DISCOUNT_BASIS_POINTS, true)
            ),
            ResearchPurchaseSource::Automatic => {
                request.discount_basis_points == 0 && !request.consume_preparation
            }
        };
        if !valid_terms {
            return Err(ResearchPurchaseError::MalformedRequest);
        }
        let study = catalog
            .study(&request.study_id)
            .ok_or(ResearchPurchaseError::UnknownStudy)?;
        if self.owned_studies.contains(&request.study_id) {
            return Err(ResearchPurchaseError::AlreadyOwned);
        }
        if study
            .prerequisites
            .iter()
            .any(|prerequisite| !self.owned_studies.contains(prerequisite))
        {
            return Err(ResearchPurchaseError::NotFrontier);
        }
        let charged_price =
            discounted_price(study.undiscounted_price, request.discount_basis_points)?;
        let committed_research_version = self
            .version
            .checked_add(1)
            .ok_or(ResearchPurchaseError::Overflow)?;
        let favor_event_id = FavorEventId::derive(
            "research_purchase",
            request.colony_id.as_str(),
            request.id.as_str(),
        );
        let favor_outcome = ledger.debit(
            favor_event_id.clone(),
            FavorEventKind::ResearchPurchase,
            charged_price,
            request.expected_favor_version,
            request.now_tick,
        )?;
        let favor_event = ledger
            .event(&favor_event_id)
            .ok_or(ResearchPurchaseError::MalformedPersistence)?;
        let event = ResearchPurchaseEvent {
            id: request.id.clone(),
            source: request.source,
            study_id: request.study_id.clone(),
            undiscounted_price: study.undiscounted_price,
            charged_price,
            discount_basis_points: request.discount_basis_points,
            consumed_preparation: request.consume_preparation,
            favor_event_id,
            committed_research_version,
            committed_favor_version: favor_event.committed_version,
            committed_tick: request.now_tick,
        };
        self.owned_studies.insert(request.study_id.clone());
        self.purchases.insert(request.id, event);
        self.version = committed_research_version;
        let outcome = match favor_outcome {
            FavorCommitOutcome::Committed => ResearchPurchaseOutcome::Committed,
            FavorCommitOutcome::AlreadyCommitted => ResearchPurchaseOutcome::AlreadyCommitted,
        };
        Ok(CommittedResearchPurchase {
            outcome,
            study_id: request.study_id,
        })
    }

    fn validate(&self) -> Result<(), ResearchPurchaseError> {
        if self.schema_version != RESEARCH_PURCHASE_SCHEMA_VERSION
            || self.purchases.len() > MAX_PURCHASE_EVENTS
        {
            return Err(ResearchPurchaseError::MalformedPersistence);
        }
        self.automatic_quota.validate()?;
        self.god_queue.validate()?;
        self.leader_lane.validate()?;
        if self
            .repeatable_completions
            .values()
            .any(|count| *count == 0)
            || self
                .repeatable_completions
                .keys()
                .any(|study_id| self.owned_studies.contains(study_id))
        {
            return Err(ResearchPurchaseError::MalformedPersistence);
        }
        let mut version = 0;
        let mut studies = BTreeSet::new();
        for (id, event) in &self.purchases {
            if id != &event.id {
                return Err(ResearchPurchaseError::MalformedPersistence);
            }
        }
        let mut events = self.purchases.values().collect::<Vec<_>>();
        events.sort_by_key(|event| event.committed_research_version);
        for event in events {
            if event.committed_research_version != version + 1
                || event.undiscounted_price == Favor::ZERO
                || event.charged_price == Favor::ZERO
                || match event.source {
                    ResearchPurchaseSource::Player => !matches!(
                        (event.discount_basis_points, event.consumed_preparation),
                        (0, false) | (PLAYER_PREPARATION_DISCOUNT_BASIS_POINTS, true)
                    ),
                    ResearchPurchaseSource::Automatic => {
                        event.discount_basis_points != 0 || event.consumed_preparation
                    }
                }
                || !studies.insert(event.study_id.clone())
            {
                return Err(ResearchPurchaseError::MalformedPersistence);
            }
            version = event.committed_research_version;
        }
        if version > self.version || !studies.is_subset(&self.owned_studies) {
            return Err(ResearchPurchaseError::MalformedPersistence);
        }
        Ok(())
    }
}

impl Default for ResearchPurchaseState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedResearchPurchaseState {
    schema_version: u32,
    #[serde(default)]
    version: u64,
    #[serde(default)]
    owned_studies: BTreeSet<StudyId>,
    #[serde(default)]
    repeatable_completions: BTreeMap<StudyId, u32>,
    #[serde(default)]
    purchases: BTreeMap<ResearchPurchaseId, ResearchPurchaseEvent>,
    #[serde(default)]
    automatic_quota: AutomaticResearchQuotaState,
    #[serde(default)]
    god_queue: GodResearchQueueState,
    #[serde(default)]
    leader_lane: LeaderResearchLaneState,
}

impl<'de> Deserialize<'de> for ResearchPurchaseState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = UncheckedResearchPurchaseState::deserialize(deserializer)?;
        let state = Self {
            schema_version: raw.schema_version,
            version: raw.version,
            owned_studies: raw.owned_studies,
            repeatable_completions: raw.repeatable_completions,
            purchases: raw.purchases,
            automatic_quota: raw.automatic_quota,
            god_queue: raw.god_queue,
            leader_lane: raw.leader_lane,
        };
        state.validate().map_err(serde::de::Error::custom)?;
        Ok(state)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerResearchPurchaseRequest {
    pub id: ResearchPurchaseId,
    pub colony_id: PlannerId,
    pub study_id: StudyId,
    pub expected_research_version: u64,
    pub expected_favor_version: u64,
    pub discount_basis_points: u16,
    pub consume_preparation: bool,
    pub now_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutomaticResearchPurchaseRequest {
    pub id: ResearchPurchaseId,
    pub colony_id: PlannerId,
    pub expected_research_version: u64,
    pub expected_favor_version: u64,
    pub effective_loremaster: AutomaticResearchCapability,
    pub scores: BTreeMap<StudyId, AutomaticStudyScoreInputs>,
    pub now_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomaticResearchCapability {
    BeforeEffectiveLoremaster,
    EffectiveLoremaster(EffectiveLevel),
}

impl AutomaticResearchCapability {
    #[must_use]
    pub const fn quota_limit(self) -> usize {
        match self {
            Self::BeforeEffectiveLoremaster => 1,
            Self::EffectiveLoremaster(level) => match level.get() {
                1 => 1,
                2 | 3 => 2,
                4 => 3,
                5 => 4,
                _ => unreachable!(),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutomaticStudyScoreInputs {
    pub belief_basis_points: i64,
    pub posture_basis_points: i64,
    pub personality_basis_points: i64,
    pub dependency_basis_points: i64,
    pub expected_value_basis_points: i64,
}

impl AutomaticStudyScoreInputs {
    #[must_use]
    pub fn total_score(self) -> i64 {
        i128::from(self.belief_basis_points)
            .saturating_add(i128::from(self.posture_basis_points))
            .saturating_add(i128::from(self.personality_basis_points))
            .saturating_add(i128::from(self.dependency_basis_points))
            .saturating_add(i128::from(self.expected_value_basis_points))
            .clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutomaticResearchPurchaseOutcome {
    pub outcome: ResearchPurchaseOutcome,
    pub study_id: StudyId,
    pub score: Option<i64>,
    pub quota_limit: usize,
    pub quota_used_after: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutomaticResearchQuotaState {
    pub schema_version: u32,
    pub committed_ticks: Vec<u64>,
    #[serde(default)]
    pub legacy_not_before_tick: Option<u64>,
}

impl AutomaticResearchQuotaState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: RESEARCH_PURCHASE_SCHEMA_VERSION,
            committed_ticks: Vec::new(),
            legacy_not_before_tick: None,
        }
    }

    #[must_use]
    pub fn used_in_window(&self, now_tick: u64) -> usize {
        self.committed_ticks
            .iter()
            .filter(|tick| tick.saturating_add(AUTOMATIC_RESEARCH_WINDOW_GAME_MINUTES) > now_tick)
            .count()
    }

    fn record_commit(&mut self, now_tick: u64) -> Result<(), ResearchPurchaseError> {
        self.committed_ticks
            .retain(|tick| tick.saturating_add(AUTOMATIC_RESEARCH_WINDOW_GAME_MINUTES) > now_tick);
        if self.committed_ticks.len() >= MAX_AUTOMATIC_QUOTA_TIMESTAMPS {
            return Err(ResearchPurchaseError::Overflow);
        }
        self.committed_ticks.push(now_tick);
        self.committed_ticks.sort_unstable();
        Ok(())
    }

    fn validate(&self) -> Result<(), ResearchPurchaseError> {
        if self.schema_version != RESEARCH_PURCHASE_SCHEMA_VERSION
            || self.committed_ticks.len() > MAX_AUTOMATIC_QUOTA_TIMESTAMPS
            || self
                .committed_ticks
                .windows(2)
                .any(|pair| pair[0] > pair[1])
        {
            return Err(ResearchPurchaseError::MalformedPersistence);
        }
        Ok(())
    }
}

impl Default for AutomaticResearchQuotaState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommitResearchPurchaseRequest {
    id: ResearchPurchaseId,
    colony_id: PlannerId,
    source: ResearchPurchaseSource,
    study_id: StudyId,
    expected_research_version: u64,
    expected_favor_version: u64,
    discount_basis_points: u16,
    consume_preparation: bool,
    now_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommittedResearchPurchase {
    outcome: ResearchPurchaseOutcome,
    study_id: StudyId,
}

fn event_matches_request(
    event: &ResearchPurchaseEvent,
    request: &CommitResearchPurchaseRequest,
) -> bool {
    event.source == request.source
        && event.study_id == request.study_id
        && event.discount_basis_points == request.discount_basis_points
        && event.consumed_preparation == request.consume_preparation
}

fn discounted_price(
    undiscounted: Favor,
    discount_basis_points: u16,
) -> Result<Favor, ResearchPurchaseError> {
    let multiplier = u128::from(BASIS_POINTS_SCALE as u16 - discount_basis_points);
    let numerator = u128::from(undiscounted.micro_favor()).saturating_mul(multiplier);
    let denominator = u128::from(BASIS_POINTS_SCALE as u16);
    let micro = numerator
        .checked_add(denominator - 1)
        .ok_or(ResearchPurchaseError::Overflow)?
        / denominator;
    let micro = u64::try_from(micro).map_err(|_| ResearchPurchaseError::Overflow)?;
    if micro == 0 {
        return Err(ResearchPurchaseError::MalformedRequest);
    }
    Ok(Favor::from_micro_favor(micro))
}

impl PartialOrd for AutomaticStudyScoreInputs {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AutomaticStudyScoreInputs {
    fn cmp(&self, other: &Self) -> Ordering {
        self.total_score().cmp(&other.total_score())
    }
}

/// The two currencies accepted by the God lane.  Notes fund ordinary studies;
/// Void Insight funds Hole-axis studies.  Neither name is a spendable village
/// currency and Leader research never touches either balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GodResearchCurrency {
    Notes,
    VoidInsight,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResearchFunds {
    pub notes: u64,
    pub void_insight: u64,
}

impl ResearchFunds {
    fn debit(
        &mut self,
        currency: GodResearchCurrency,
        amount: u64,
    ) -> Result<(), ResearchPurchaseError> {
        let balance = match currency {
            GodResearchCurrency::Notes => &mut self.notes,
            GodResearchCurrency::VoidInsight => &mut self.void_insight,
        };
        *balance = balance
            .checked_sub(amount)
            .ok_or(ResearchPurchaseError::InsufficientGodFunds)?;
        Ok(())
    }

    fn credit(
        &mut self,
        currency: GodResearchCurrency,
        amount: u64,
    ) -> Result<(), ResearchPurchaseError> {
        let balance = match currency {
            GodResearchCurrency::Notes => &mut self.notes,
            GodResearchCurrency::VoidInsight => &mut self.void_insight,
        };
        *balance = balance
            .checked_add(amount)
            .ok_or(ResearchPurchaseError::Overflow)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GodResearchStudyTerms {
    pub currency: GodResearchCurrency,
    pub price: u64,
    pub duration_game_minutes: u64,
}

impl GodResearchStudyTerms {
    pub fn validate(self) -> Result<(), ResearchPurchaseError> {
        if self.price == 0 || self.duration_game_minutes == 0 {
            return Err(ResearchPurchaseError::MalformedGodTerms);
        }
        Ok(())
    }

    #[must_use]
    pub fn preparation_labor_minutes(self) -> u64 {
        let numerator = self
            .duration_game_minutes
            .saturating_mul(u64::from(PREPARATION_LABOR_BASIS_POINTS));
        numerator.div_ceil(u64::from(BASIS_POINTS_SCALE as u16))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GodResearchTerms {
    pub by_study: BTreeMap<StudyId, GodResearchStudyTerms>,
}

impl GodResearchTerms {
    pub fn get(&self, study_id: &StudyId) -> Result<GodResearchStudyTerms, ResearchPurchaseError> {
        let terms = self
            .by_study
            .get(study_id)
            .copied()
            .ok_or(ResearchPurchaseError::MissingGodTerms)?;
        terms.validate()?;
        Ok(terms)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GodResearchQueueEntry {
    pub study_id: StudyId,
    pub currency: GodResearchCurrency,
    pub frozen_price: u64,
    pub frozen_duration_game_minutes: u64,
    pub completed_labor_minutes: u64,
    pub frozen: bool,
}

impl GodResearchQueueEntry {
    #[must_use]
    pub fn remaining_labor_minutes(&self) -> u64 {
        self.frozen_duration_game_minutes
            .saturating_sub(self.completed_labor_minutes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GodResearchQueueState {
    pub schema_version: u32,
    pub entries: Vec<GodResearchQueueEntry>,
}

impl GodResearchQueueState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: RESEARCH_PURCHASE_SCHEMA_VERSION,
            entries: Vec::new(),
        }
    }

    #[must_use]
    pub fn entries(&self) -> &[GodResearchQueueEntry] {
        &self.entries
    }

    #[must_use]
    pub fn entry(&self, study_id: &StudyId) -> Option<&GodResearchQueueEntry> {
        self.entries
            .iter()
            .find(|entry| &entry.study_id == study_id)
    }

    fn queue_path(
        &mut self,
        catalog: &SyntheticResearchCatalog,
        terms: &GodResearchTerms,
        owned: &BTreeSet<StudyId>,
        repeatable_completions: &BTreeMap<StudyId, u32>,
        target: StudyId,
    ) -> Result<Vec<StudyId>, ResearchPurchaseError> {
        let queued_ids = self
            .entries
            .iter()
            .map(|entry| entry.study_id.clone())
            .collect::<BTreeSet<_>>();
        let mut visiting = BTreeSet::new();
        let mut path = Vec::new();
        collect_missing_god_path(
            catalog,
            &target,
            owned,
            &queued_ids,
            &mut visiting,
            &mut path,
        )?;
        if self.entries.len().saturating_add(path.len()) > MAX_GOD_RESEARCH_QUEUE {
            return Err(ResearchPurchaseError::GodQueueCapacityExceeded);
        }
        for study_id in &path {
            let mut term = terms.get(study_id)?;
            if catalog.is_repeatable(study_id) {
                term.price = doubled_repeat_price(
                    term.price,
                    repeatable_completions.get(study_id).copied().unwrap_or(0),
                )?;
            }
            self.entries.push(GodResearchQueueEntry {
                study_id: study_id.clone(),
                currency: term.currency,
                frozen_price: term.price,
                frozen_duration_game_minutes: term.duration_game_minutes,
                completed_labor_minutes: 0,
                frozen: false,
            });
        }
        Ok(path)
    }

    fn fund_front(
        &mut self,
        funds: &mut ResearchFunds,
        discount_basis_points: u16,
    ) -> Result<GodResearchFundOutcome, ResearchPurchaseError> {
        let Some(front) = self.entries.first_mut() else {
            return Err(ResearchPurchaseError::GodQueueEmpty);
        };
        if front.frozen {
            return Ok(GodResearchFundOutcome::AlreadyFunded);
        }
        if !matches!(
            discount_basis_points,
            0 | PLAYER_PREPARATION_DISCOUNT_BASIS_POINTS
        ) || discount_basis_points != 0 && front.currency != GodResearchCurrency::Notes
        {
            return Err(ResearchPurchaseError::MalformedRequest);
        }
        let multiplier = u128::from(BASIS_POINTS_SCALE as u16 - discount_basis_points);
        let denominator = u128::from(BASIS_POINTS_SCALE as u16);
        let charged = u128::from(front.frozen_price)
            .saturating_mul(multiplier)
            .div_ceil(denominator);
        let charged = u64::try_from(charged).map_err(|_| ResearchPurchaseError::Overflow)?;
        funds.debit(front.currency, charged)?;
        front.frozen_price = charged;
        front.frozen = true;
        Ok(GodResearchFundOutcome::Funded)
    }

    fn estimated_wait_minutes(&self, study_id: &StudyId) -> Option<u64> {
        let index = self
            .entries
            .iter()
            .position(|entry| &entry.study_id == study_id)?;
        Some(
            self.entries
                .iter()
                .take(index + 1)
                .fold(0_u64, |total, entry| {
                    total.saturating_add(entry.remaining_labor_minutes())
                }),
        )
    }

    fn record_front_labor(
        &mut self,
        labor: u64,
    ) -> Result<GodResearchLaborOutcome, ResearchPurchaseError> {
        if labor == 0 {
            return Ok(GodResearchLaborOutcome::NoWork);
        }
        let Some(front) = self.entries.first_mut() else {
            return Err(ResearchPurchaseError::GodQueueEmpty);
        };
        if !front.frozen {
            return Err(ResearchPurchaseError::GodFrontNotFunded);
        }
        front.completed_labor_minutes = front
            .completed_labor_minutes
            .saturating_add(labor)
            .min(front.frozen_duration_game_minutes);
        if front.completed_labor_minutes < front.frozen_duration_game_minutes {
            return Ok(GodResearchLaborOutcome::Advanced {
                remaining_labor_minutes: front.remaining_labor_minutes(),
            });
        }
        let completed = self.entries.remove(0).study_id;
        Ok(GodResearchLaborOutcome::Completed(completed))
    }

    fn reorder(
        &mut self,
        catalog: &SyntheticResearchCatalog,
        owned: &BTreeSet<StudyId>,
        study_id: &StudyId,
        to_index: usize,
    ) -> Result<(), ResearchPurchaseError> {
        let from_index = self
            .entries
            .iter()
            .position(|entry| &entry.study_id == study_id)
            .ok_or(ResearchPurchaseError::GodQueueTargetNotFound)?;
        if to_index >= self.entries.len() {
            return Err(ResearchPurchaseError::GodQueueTargetNotFound);
        }
        if self.entries.first().is_some_and(|front| front.frozen)
            && (from_index == 0 || to_index == 0)
        {
            return Err(ResearchPurchaseError::GodQueueFrozenFront);
        }
        let entry = self.entries.remove(from_index);
        self.entries.insert(to_index, entry);
        if !god_queue_order_is_topological(catalog, owned, &self.entries) {
            let entry = self.entries.remove(to_index);
            self.entries.insert(from_index, entry);
            return Err(ResearchPurchaseError::GodQueuePrerequisiteOrder);
        }
        Ok(())
    }

    fn remove_with_dependents(
        &mut self,
        catalog: &SyntheticResearchCatalog,
        owned: &BTreeSet<StudyId>,
        study_id: &StudyId,
        funds: &mut ResearchFunds,
    ) -> Result<GodResearchRemoval, ResearchPurchaseError> {
        if self.entry(study_id).is_none() {
            return Err(ResearchPurchaseError::GodQueueTargetNotFound);
        }
        let mut removed = BTreeSet::from([study_id.clone()]);
        loop {
            let mut changed = false;
            for entry in &self.entries {
                let study = catalog
                    .study(&entry.study_id)
                    .ok_or(ResearchPurchaseError::MalformedCatalog)?;
                if study.prerequisites.iter().any(|id| removed.contains(id))
                    && removed.insert(entry.study_id.clone())
                {
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let mut result = GodResearchRemoval::default();
        let mut kept = Vec::with_capacity(self.entries.len());
        for entry in self.entries.drain(..) {
            if removed.contains(&entry.study_id) {
                if entry.frozen {
                    funds.credit(entry.currency, entry.frozen_price)?;
                    result.refunded.push((entry.currency, entry.frozen_price));
                }
                result.lost_labor_minutes = result
                    .lost_labor_minutes
                    .saturating_add(entry.completed_labor_minutes);
                result.removed_studies.push(entry.study_id);
            } else {
                kept.push(entry);
            }
        }
        self.entries = kept;
        if !god_queue_order_is_topological(catalog, owned, &self.entries) {
            return Err(ResearchPurchaseError::GodQueuePrerequisiteOrder);
        }
        Ok(result)
    }

    fn validate(&self) -> Result<(), ResearchPurchaseError> {
        if self.schema_version != RESEARCH_PURCHASE_SCHEMA_VERSION
            || self.entries.len() > MAX_GOD_RESEARCH_QUEUE
            || {
                let ids = self
                    .entries
                    .iter()
                    .map(|entry| &entry.study_id)
                    .collect::<BTreeSet<_>>();
                ids.len() != self.entries.len()
            }
            || self
                .entries
                .iter()
                .enumerate()
                .any(|(index, entry)| entry.frozen && index != 0)
            || self.entries.iter().any(|entry| {
                entry.frozen_price == 0
                    || entry.frozen_duration_game_minutes == 0
                    || entry.completed_labor_minutes > entry.frozen_duration_game_minutes
            })
        {
            return Err(ResearchPurchaseError::MalformedPersistence);
        }
        Ok(())
    }
}

impl Default for GodResearchQueueState {
    fn default() -> Self {
        Self::new()
    }
}

fn collect_missing_god_path(
    catalog: &SyntheticResearchCatalog,
    study_id: &StudyId,
    owned: &BTreeSet<StudyId>,
    queued: &BTreeSet<StudyId>,
    visiting: &mut BTreeSet<StudyId>,
    path: &mut Vec<StudyId>,
) -> Result<(), ResearchPurchaseError> {
    if owned.contains(study_id) || queued.contains(study_id) {
        return Ok(());
    }
    if !visiting.insert(study_id.clone()) {
        return Err(ResearchPurchaseError::MalformedCatalog);
    }
    let study = catalog
        .study(study_id)
        .ok_or(ResearchPurchaseError::UnknownStudy)?;
    for prerequisite in &study.prerequisites {
        collect_missing_god_path(catalog, prerequisite, owned, queued, visiting, path)?;
    }
    visiting.remove(study_id);
    path.push(study_id.clone());
    Ok(())
}

fn god_queue_order_is_topological(
    catalog: &SyntheticResearchCatalog,
    owned: &BTreeSet<StudyId>,
    queue: &[GodResearchQueueEntry],
) -> bool {
    let mut available = owned.clone();
    for entry in queue {
        let Some(study) = catalog.study(&entry.study_id) else {
            return false;
        };
        if study
            .prerequisites
            .iter()
            .any(|prerequisite| !available.contains(prerequisite))
        {
            return false;
        }
        available.insert(entry.study_id.clone());
    }
    true
}

fn doubled_repeat_price(
    base_terminal_price: u64,
    prior_completions: u32,
) -> Result<u64, ResearchPurchaseError> {
    let multiplier = 1_u64
        .checked_shl(prior_completions)
        .ok_or(ResearchPurchaseError::Overflow)?;
    base_terminal_price
        .checked_mul(multiplier)
        .ok_or(ResearchPurchaseError::Overflow)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GodResearchWorkAuthorization {
    pub completed_research_station: bool,
    pub staffed_scholar_alive: bool,
}

impl GodResearchWorkAuthorization {
    fn permits_work(self) -> bool {
        self.completed_research_station && self.staffed_scholar_alive
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GodResearchFundOutcome {
    Funded,
    AlreadyFunded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GodResearchLaborOutcome {
    NoWork,
    Advanced { remaining_labor_minutes: u64 },
    Completed(StudyId),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GodResearchRemoval {
    pub removed_studies: Vec<StudyId>,
    pub refunded: Vec<(GodResearchCurrency, u64)>,
    pub lost_labor_minutes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderDuplicateAuthorization {
    None,
    Emergency {
        report_indicates_urgent_need: bool,
        needed_before_tick: u64,
        estimated_god_completion_tick: u64,
    },
    Oopsie {
        effective_expertise_intelligence_level: u8,
        keyed_roll_percent: u8,
    },
}

impl LeaderDuplicateAuthorization {
    #[must_use]
    pub const fn oopsie_percent(effective_expertise_intelligence_level: u8) -> u8 {
        match effective_expertise_intelligence_level {
            0..=4 => {
                LEADER_DUPLICATE_OOPSIE_PERCENT_BY_EXPERTISE
                    [effective_expertise_intelligence_level as usize]
            }
            _ => 0,
        }
    }

    #[must_use]
    pub const fn permits_duplicate(self) -> bool {
        matches!(self, Self::Emergency { .. } | Self::Oopsie { .. })
    }

    fn validate(self, duplicate_target: bool) -> Result<(), ResearchPurchaseError> {
        match self {
            Self::None if duplicate_target => Err(ResearchPurchaseError::LeaderDuplicateForbidden),
            Self::Emergency { .. } | Self::Oopsie { .. } if !duplicate_target => {
                Err(ResearchPurchaseError::LeaderDuplicateForbidden)
            }
            Self::Emergency {
                report_indicates_urgent_need,
                needed_before_tick,
                estimated_god_completion_tick,
            } if !report_indicates_urgent_need
                || needed_before_tick >= estimated_god_completion_tick =>
            {
                Err(ResearchPurchaseError::LeaderDuplicateForbidden)
            }
            Self::Oopsie {
                effective_expertise_intelligence_level,
                keyed_roll_percent,
            } if keyed_roll_percent
                >= Self::oopsie_percent(effective_expertise_intelligence_level) =>
            {
                Err(ResearchPurchaseError::LeaderDuplicateForbidden)
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderResearchRequest {
    pub id: ResearchPurchaseId,
    pub study_id: StudyId,
    pub expected_research_version: u64,
    pub effective_loremaster_level: u8,
    pub now_tick: u64,
    pub duplicate_authorization: LeaderDuplicateAuthorization,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderResearchCandidate {
    pub study_id: StudyId,
    pub decision_inputs: LeaderResearchDecisionInputs,
    pub repeatable: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LeaderResearchDecisionInputs {
    pub report_score: i64,
    pub need_score: i64,
    pub intelligence_score: i64,
    pub personality_score: i64,
    pub research_skill_score: i64,
}

impl LeaderResearchDecisionInputs {
    #[must_use]
    pub fn total_score(self) -> i64 {
        [
            self.report_score,
            self.need_score,
            self.intelligence_score,
            self.personality_score,
            self.research_skill_score,
        ]
        .into_iter()
        .fold(0_i64, i64::saturating_add)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderResearchCompletion {
    pub study_id: StudyId,
    pub duplicate_authorization: LeaderDuplicateAuthorization,
    pub event_kind: LeaderResearchEventKind,
    pub overtake: Option<GodResearchRemoval>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaderResearchEvent {
    pub id: ResearchPurchaseId,
    pub study_id: StudyId,
    pub committed_tick: u64,
    pub effective_loremaster_level: u8,
    pub duplicate_authorization: LeaderDuplicateAuthorizationWire,
    #[serde(default)]
    pub event_kind: LeaderResearchEventKind,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaderResearchEventKind {
    #[default]
    StandardUnlock,
    IntentionalEmergencyOverride,
    AccidentalDuplicateOopsie,
}

impl From<LeaderDuplicateAuthorization> for LeaderResearchEventKind {
    fn from(value: LeaderDuplicateAuthorization) -> Self {
        match value {
            LeaderDuplicateAuthorization::None => Self::StandardUnlock,
            LeaderDuplicateAuthorization::Emergency { .. } => Self::IntentionalEmergencyOverride,
            LeaderDuplicateAuthorization::Oopsie { .. } => Self::AccidentalDuplicateOopsie,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaderDuplicateAuthorizationWire {
    None,
    Emergency,
    Oopsie,
}

impl From<LeaderDuplicateAuthorization> for LeaderDuplicateAuthorizationWire {
    fn from(value: LeaderDuplicateAuthorization) -> Self {
        match value {
            LeaderDuplicateAuthorization::None => Self::None,
            LeaderDuplicateAuthorization::Emergency { .. } => Self::Emergency,
            LeaderDuplicateAuthorization::Oopsie { .. } => Self::Oopsie,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaderResearchLaneState {
    pub schema_version: u32,
    pub committed_ticks: Vec<u64>,
    pub events: BTreeMap<ResearchPurchaseId, LeaderResearchEvent>,
}

impl LeaderResearchLaneState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: RESEARCH_PURCHASE_SCHEMA_VERSION,
            committed_ticks: Vec::new(),
            events: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn quota_limit(effective_loremaster_level: u8) -> usize {
        match effective_loremaster_level {
            0 | 1 => 1,
            2 | 3 => 2,
            4 => 3,
            _ => 4,
        }
    }

    #[must_use]
    pub fn used_in_window(&self, now_tick: u64) -> usize {
        self.committed_ticks
            .iter()
            .filter(|tick| tick.saturating_add(AUTOMATIC_RESEARCH_WINDOW_GAME_MINUTES) > now_tick)
            .count()
    }

    fn record(&mut self, request: &LeaderResearchRequest) -> Result<(), ResearchPurchaseError> {
        if let Some(event) = self.events.get(&request.id) {
            if event.study_id == request.study_id
                && event.committed_tick == request.now_tick
                && event.effective_loremaster_level == request.effective_loremaster_level
            {
                return Err(ResearchPurchaseError::LeaderResearchAlreadyCommitted);
            }
            return Err(ResearchPurchaseError::PurchaseIdConflict);
        }
        let limit = Self::quota_limit(request.effective_loremaster_level);
        if self.used_in_window(request.now_tick) >= limit {
            return Err(ResearchPurchaseError::AutomaticQuotaExhausted);
        }
        self.committed_ticks.retain(|tick| {
            tick.saturating_add(AUTOMATIC_RESEARCH_WINDOW_GAME_MINUTES) > request.now_tick
        });
        self.committed_ticks.push(request.now_tick);
        self.committed_ticks.sort_unstable();
        self.events.insert(
            request.id.clone(),
            LeaderResearchEvent {
                id: request.id.clone(),
                study_id: request.study_id.clone(),
                committed_tick: request.now_tick,
                effective_loremaster_level: request.effective_loremaster_level,
                duplicate_authorization: request.duplicate_authorization.into(),
                event_kind: request.duplicate_authorization.into(),
            },
        );
        Ok(())
    }

    fn validate(&self) -> Result<(), ResearchPurchaseError> {
        if self.schema_version != RESEARCH_PURCHASE_SCHEMA_VERSION
            || self.committed_ticks.len() > MAX_AUTOMATIC_QUOTA_TIMESTAMPS
            || self.events.len() > MAX_PURCHASE_EVENTS
            || self
                .committed_ticks
                .windows(2)
                .any(|pair| pair[0] > pair[1])
            || self.events.iter().any(|(id, event)| id != &event.id)
        {
            return Err(ResearchPurchaseError::MalformedPersistence);
        }
        Ok(())
    }
}

impl Default for LeaderResearchLaneState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResearchPurchaseError {
    UnknownStudy,
    AlreadyOwned,
    NotFrontier,
    NoAffordableFrontier,
    AutomaticQuotaExhausted,
    StaleResearchVersion,
    PurchaseIdConflict,
    MalformedCatalog,
    MalformedPersistence,
    MalformedRequest,
    MissingGodTerms,
    MalformedGodTerms,
    InsufficientGodFunds,
    GodQueueCapacityExceeded,
    GodQueueEmpty,
    GodQueueTargetNotFound,
    GodQueuePrerequisiteOrder,
    GodQueueFrozenFront,
    GodFrontNotFunded,
    GodResearchInfrastructureUnavailable,
    LeaderDuplicateForbidden,
    LeaderResearchAlreadyCommitted,
    FiniteResearchRemaining,
    Overflow,
    Favor(FavorError),
}

impl From<FavorError> for ResearchPurchaseError {
    fn from(value: FavorError) -> Self {
        Self::Favor(value)
    }
}

impl std::fmt::Display for ResearchPurchaseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Research purchase error: {self:?}")
    }
}

impl std::error::Error for ResearchPurchaseError {}
