//! LAI.58 canonical two-lane research authority.
//!
//! This leaf deliberately has no world-tick, protocol, UI, persistence-adapter,
//! or legacy-purchase dependency.  It is the versioned mutation boundary which
//! later integration uses with the canonical LAI.44 [`ProgressionCatalog`],
//! [`ResearchNotes`], and [`VoidInsight`] types.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    planner_core::PlannerId,
    progression_research::{
        ColonyPartitionKey, CurrencyEventId, ProgressionCatalog, ResearchNotes,
        ResearchNotesLedger, ResearchNotesSpendRequest, StudyCurrency, StudyId, StudyKind,
        VoidDebitPurpose, VoidInsight, VoidInsightLedger, VoidSpendRequest,
    },
};

pub const RESEARCH_AUTHORITY_SCHEMA_VERSION: u32 = 1;
pub const ROLLING_SEVEN_GAME_DAYS_MINUTES: u64 = 7 * 24 * 60;
pub const MAX_GOD_QUEUE_ENTRIES: usize = 64;
pub const MAX_RESEARCH_RECEIPTS: usize = 512;
pub const MAX_LEADER_COMMITS: usize = 32;
/// A persistence-valid preparation can only target the bounded God path.
/// Keep report output bounded even when inspecting a not-yet-validated value.
pub const MAX_RESEARCH_REPORT_PREPARATIONS: usize = MAX_GOD_QUEUE_ENTRIES;
pub const PREPARATION_BASIS_POINTS: u16 = 2_500;
pub const OOPSIE_PERCENT_BY_EFFECTIVE_LEVEL: [u8; 5] = [25, 12, 5, 1, 0];

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResearchCommandId(PlannerId);

impl ResearchCommandId {
    #[must_use]
    pub fn derive(colony_id: &PlannerId, action: &str) -> Self {
        Self(PlannerId::derive(
            "lai58-research",
            [colony_id.as_str(), action],
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateAuthorization {
    None,
    CriticalVillage,
    KeyedOopsie,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CriticalVillageAuthorization {
    pub report_marks_critical: bool,
    pub needed_before_tick: u64,
    pub estimated_god_completion_tick: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KeyedOopsieAuthorization {
    /// The effective combined expertise/Intelligence band.  Values above four
    /// are intentionally perfect rather than wrapping back to a poor band.
    pub effective_level: u8,
    pub keyed_roll_percent: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LeaderDuplicatePermit {
    None,
    CriticalVillage(CriticalVillageAuthorization),
    KeyedOopsie(KeyedOopsieAuthorization),
}

impl LeaderDuplicatePermit {
    #[must_use]
    pub const fn kind(&self) -> DuplicateAuthorization {
        match self {
            Self::None => DuplicateAuthorization::None,
            Self::CriticalVillage(_) => DuplicateAuthorization::CriticalVillage,
            Self::KeyedOopsie(_) => DuplicateAuthorization::KeyedOopsie,
        }
    }

    #[must_use]
    pub const fn oopsie_percent(effective_level: u8) -> u8 {
        match effective_level {
            0..=4 => OOPSIE_PERCENT_BY_EFFECTIVE_LEVEL[effective_level as usize],
            _ => 0,
        }
    }

    fn permits_queued_target(&self) -> bool {
        match self {
            Self::None => false,
            Self::CriticalVillage(value) => {
                value.report_marks_critical
                    && value.needed_before_tick < value.estimated_god_completion_tick
            }
            Self::KeyedOopsie(value) => {
                value.keyed_roll_percent < Self::oopsie_percent(value.effective_level)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchLane {
    Leader,
    God,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrozenGodTerms {
    pub currency: StudyCurrency,
    pub base_cost_micro: u64,
    pub payable_cost_micro: u64,
    pub duration_game_minutes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GodResearchEntry {
    pub sequence: u64,
    pub study_id: StudyId,
    #[serde(default)]
    pub frozen: Option<FrozenGodTerms>,
    pub staffed_labor_minutes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparationState {
    pub study_id: StudyId,
    pub required_labor_minutes: u64,
    pub completed_labor_minutes: u64,
}

impl PreparationState {
    #[must_use]
    pub const fn complete(&self) -> bool {
        self.completed_labor_minutes >= self.required_labor_minutes
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaderCommit {
    pub study_id: StudyId,
    pub committed_tick: u64,
    pub effective_loremaster_level: u8,
    pub duplicate_authorization: DuplicateAuthorization,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaderCandidate {
    pub study_id: StudyId,
    pub report_score: i64,
    pub need_score: i64,
    pub intelligence_score: i64,
    pub personality_score: i64,
    pub research_skill_score: i64,
}

impl LeaderCandidate {
    #[must_use]
    pub fn score(&self) -> i64 {
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaderExclusionReason {
    UnknownStudy,
    AlreadyOwned,
    PrerequisiteLocked,
    RepeatableWhileFiniteRemains,
    GodQueueDuplicateForbidden,
    RollingCadenceExhausted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaderSelection {
    pub selected: Option<StudyId>,
    pub selected_duplicate_authorization: DuplicateAuthorization,
    pub excluded: BTreeMap<StudyId, LeaderExclusionReason>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResearchCommandKind {
    QueueGodPath {
        target: StudyId,
    },
    FundGodFront {
        consume_preparation: bool,
    },
    /// Player intent only: open a zero-progress physical preparation record.
    /// No station, scholar, or labor claim is accepted from this command.
    RequestPreparation {
        study_id: StudyId,
    },
    /// Authoritative physical-work receipt. Runtime adapters must derive every
    /// witness and labor minute from world truth, never from a player payload.
    PerformPreparation {
        study_id: StudyId,
        staffed_research_station: bool,
        scholar_alive: bool,
        labor_minutes: u64,
    },
    PerformGodLabor {
        staffed_research_station: bool,
        scholar_alive: bool,
        labor_minutes: u64,
    },
    RemoveGodTarget {
        study_id: StudyId,
    },
    /// Move one queued God study immediately before another queued study.
    /// `None` means move to the back, matching the canonical protocol action.
    ReorderGodTarget {
        study_id: StudyId,
        before_study_id: Option<StudyId>,
    },
    CompleteLeader {
        study_id: StudyId,
        effective_loremaster_level: u8,
        now_tick: u64,
        duplicate_permit: LeaderDuplicatePermit,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResearchCommand {
    pub id: ResearchCommandId,
    pub expected_version: u64,
    pub kind: ResearchCommandKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchCommandOutcome {
    Queued {
        entries_added: u8,
    },
    Funded {
        payable_cost_micro: u64,
    },
    PreparationAdvanced {
        remaining_labor_minutes: u64,
    },
    PreparationRequested {
        study_id: StudyId,
        required_labor_minutes: u64,
    },
    GodLaborAdvanced {
        remaining_labor_minutes: u64,
    },
    GodStudyCompleted {
        study_id: StudyId,
    },
    GodTargetRemoved {
        removed_entries: u8,
        refunded_notes_micro: u64,
        refunded_void_micro: u64,
        lost_labor_minutes: u64,
        lost_preparation_labor_minutes: u64,
    },
    GodTargetReordered {
        study_id: StudyId,
        from_index: u8,
        to_index: u8,
    },
    LeaderCompleted {
        study_id: StudyId,
        duplicate_authorization: DuplicateAuthorization,
        refunded_notes_micro: u64,
        refunded_void_micro: u64,
        lost_god_labor_minutes: u64,
        lost_preparation_labor_minutes: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResearchReceipt {
    pub command: ResearchCommand,
    pub outcome: ResearchCommandOutcome,
    pub committed_version: u64,
}

/// Front funding is intentionally a typed report instead of a boolean.  The
/// frozen terms are borrowed from the canonical God-lane entry; no second
/// cost/balance ledger is created for a snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GodResearchFundingReport<'a> {
    AwaitingFunding,
    Frozen(&'a FrozenGodTerms),
}

/// One canonical God-lane queue entry in increasing persisted sequence order.
/// It deliberately has no worker identity, belief score, or private receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GodResearchQueueReport<'a> {
    pub sequence: u64,
    pub study_id: &'a StudyId,
    pub funding: GodResearchFundingReport<'a>,
    pub staffed_labor_minutes: u64,
}

/// A completed free-Leader research decision, sorted by the actual committed
/// tick and then its stable command ID rather than hash/map insertion order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeaderResearchDecisionReport<'a> {
    pub command_id: &'a ResearchCommandId,
    pub study_id: &'a StudyId,
    pub committed_tick: u64,
    pub effective_loremaster_level: u8,
    pub duplicate_authorization: DuplicateAuthorization,
}

/// A preparation work record.  The task/worker geometry is intentionally not
/// available here because it belongs to the later physical-task authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResearchPreparationStatusReport<'a> {
    pub study_id: &'a StudyId,
    pub required_labor_minutes: u64,
    pub completed_labor_minutes: u64,
}

/// The explicit state of a study at the God/Leader lane boundary.  It replaces
/// implicit duplicate inference from hidden planner beliefs or receipts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResearchStudyCollisionReport<'a> {
    Available,
    OwnedFinite,
    GodQueued {
        sequence: u64,
        funding: GodResearchFundingReport<'a>,
    },
    LeaderDecision(LeaderResearchDecisionReport<'a>),
}

/// Bounded report collection with an explicit truncation marker.  The marker
/// prevents callers from treating a malformed/unvalidated aggregate as an
/// unbounded snapshot source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchReportPage<T> {
    pub entries: Vec<T>,
    pub truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchAuthority {
    pub schema_version: u32,
    pub colony_id: PlannerId,
    pub version: u64,
    pub notes: ResearchNotesLedger,
    pub void: VoidInsightLedger,
    pub owned_finite: BTreeSet<StudyId>,
    pub repeatable_completions: BTreeMap<StudyId, u32>,
    pub next_queue_sequence: u64,
    pub god_queue: BTreeMap<u64, GodResearchEntry>,
    pub preparations: BTreeMap<StudyId, PreparationState>,
    pub leader_commits: BTreeMap<ResearchCommandId, LeaderCommit>,
    pub receipts: BTreeMap<ResearchCommandId, ResearchReceipt>,
}

impl ResearchAuthority {
    #[must_use]
    pub fn new(
        colony_id: PlannerId,
        notes_balance: ResearchNotes,
        void_balance: VoidInsight,
    ) -> Self {
        let mut notes = ResearchNotesLedger::new(colony_id.clone());
        notes.balance = notes_balance;
        let mut void = VoidInsightLedger::new(colony_id.clone());
        void.balance = void_balance;
        Self {
            schema_version: RESEARCH_AUTHORITY_SCHEMA_VERSION,
            colony_id,
            version: 0,
            notes,
            void,
            owned_finite: BTreeSet::new(),
            repeatable_completions: BTreeMap::new(),
            next_queue_sequence: 1,
            god_queue: BTreeMap::new(),
            preparations: BTreeMap::new(),
            leader_commits: BTreeMap::new(),
            receipts: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn leader_cadence_limit(effective_loremaster_level: u8) -> usize {
        match effective_loremaster_level {
            0 | 1 => 1,
            2 | 3 => 2,
            4 => 3,
            _ => 4,
        }
    }

    #[must_use]
    pub fn god_front(&self) -> Option<&GodResearchEntry> {
        self.god_queue.values().next()
    }

    #[must_use]
    pub fn leader_used_in_window(&self, now_tick: u64) -> usize {
        self.leader_commits
            .values()
            .filter(|commit| {
                commit
                    .committed_tick
                    .saturating_add(ROLLING_SEVEN_GAME_DAYS_MINUTES)
                    > now_tick
            })
            .count()
    }

    /// Returns God queue entries in persisted topological sequence order.
    /// Terms, study IDs, and labor are borrowed from the one canonical queue;
    /// no worker identity, ledger receipt, or duplicate balance is exposed.
    #[must_use]
    pub fn report_god_queue(&self) -> impl ExactSizeIterator<Item = GodResearchQueueReport<'_>> {
        self.god_queue.values().map(|entry| GodResearchQueueReport {
            sequence: entry.sequence,
            study_id: &entry.study_id,
            funding: match entry.frozen.as_ref() {
                Some(terms) => GodResearchFundingReport::Frozen(terms),
                None => GodResearchFundingReport::AwaitingFunding,
            },
            staffed_labor_minutes: entry.staffed_labor_minutes,
        })
    }

    /// Returns durable free-Leader decisions in deterministic decision order.
    /// The bounded vector is necessary because persistence keys are command
    /// hashes, while the UI/protocol order is committed time then stable ID.
    #[must_use]
    pub fn report_leader_decisions(&self) -> Vec<LeaderResearchDecisionReport<'_>> {
        let mut decisions = self
            .leader_commits
            .iter()
            .map(|(command_id, commit)| LeaderResearchDecisionReport {
                command_id,
                study_id: &commit.study_id,
                committed_tick: commit.committed_tick,
                effective_loremaster_level: commit.effective_loremaster_level,
                duplicate_authorization: commit.duplicate_authorization,
            })
            .collect::<Vec<_>>();
        decisions.sort_by(|left, right| {
            left.committed_tick
                .cmp(&right.committed_tick)
                .then_with(|| left.command_id.cmp(right.command_id))
        });
        decisions
    }

    /// Returns preparation state by stable study ID.  A normal authority has
    /// at most one preparation for each queued God study; an explicit page
    /// marker keeps malformed persisted state from producing an unbounded
    /// canonical report.
    #[must_use]
    pub fn report_preparations(&self) -> ResearchReportPage<ResearchPreparationStatusReport<'_>> {
        let truncated = self.preparations.len() > MAX_RESEARCH_REPORT_PREPARATIONS;
        let entries = self
            .preparations
            .values()
            .take(MAX_RESEARCH_REPORT_PREPARATIONS)
            .map(|state| ResearchPreparationStatusReport {
                study_id: &state.study_id,
                required_labor_minutes: state.required_labor_minutes,
                completed_labor_minutes: state.completed_labor_minutes,
            })
            .collect();
        ResearchReportPage { entries, truncated }
    }

    /// Reports the current God/Leader collision state for one authoritative
    /// study ID.  It does not consult report scores, personality, hidden
    /// regeneration, or the private receipt ledger.
    #[must_use]
    pub fn report_study_collision(&self, study_id: &StudyId) -> ResearchStudyCollisionReport<'_> {
        if self.owned_finite.contains(study_id) {
            return ResearchStudyCollisionReport::OwnedFinite;
        }
        if let Some(entry) = self
            .god_queue
            .values()
            .find(|entry| entry.study_id == *study_id)
        {
            return ResearchStudyCollisionReport::GodQueued {
                sequence: entry.sequence,
                funding: match entry.frozen.as_ref() {
                    Some(terms) => GodResearchFundingReport::Frozen(terms),
                    None => GodResearchFundingReport::AwaitingFunding,
                },
            };
        }
        if let Some((command_id, commit)) = self
            .leader_commits
            .iter()
            .find(|(_, commit)| commit.study_id == *study_id)
        {
            return ResearchStudyCollisionReport::LeaderDecision(LeaderResearchDecisionReport {
                command_id,
                study_id: &commit.study_id,
                committed_tick: commit.committed_tick,
                effective_loremaster_level: commit.effective_loremaster_level,
                duplicate_authorization: commit.duplicate_authorization,
            });
        }
        ResearchStudyCollisionReport::Available
    }

    /// Produce only information already lawful for the Leader's reports: stable
    /// study IDs, queue position, frozen state, and a typed reason.  It never
    /// exposes the hidden live worker identity or a mutable ledger receipt.
    #[must_use]
    pub fn report_safe_projection(&self) -> ResearchReportProjection {
        ResearchReportProjection {
            version: self.version,
            notes_balance: self.notes.balance,
            void_balance: self.void.balance,
            owned_finite: self.owned_finite.iter().cloned().collect(),
            god_queue: self
                .god_queue
                .values()
                .map(|entry| ResearchQueueReport {
                    study_id: entry.study_id.clone(),
                    frozen: entry.frozen.is_some(),
                    staffed_labor_minutes: entry.staffed_labor_minutes,
                })
                .collect(),
            preparations: self
                .preparations
                .values()
                .map(|state| PreparationReport {
                    study_id: state.study_id.clone(),
                    required_labor_minutes: state.required_labor_minutes,
                    completed_labor_minutes: state.completed_labor_minutes,
                })
                .collect(),
        }
    }

    pub fn select_leader_target(
        &self,
        catalog: &ProgressionCatalog,
        candidates: &[LeaderCandidate],
        effective_loremaster_level: u8,
        now_tick: u64,
        duplicate_permit: &LeaderDuplicatePermit,
    ) -> LeaderSelection {
        let mut excluded = BTreeMap::new();
        if self.leader_used_in_window(now_tick)
            >= Self::leader_cadence_limit(effective_loremaster_level)
        {
            for candidate in candidates {
                excluded.insert(
                    candidate.study_id.clone(),
                    LeaderExclusionReason::RollingCadenceExhausted,
                );
            }
            return LeaderSelection {
                selected: None,
                selected_duplicate_authorization: DuplicateAuthorization::None,
                excluded,
            };
        }
        let finite_remaining = self.has_unowned_finite(catalog);
        let mut legal = Vec::new();
        for candidate in candidates {
            let Some(definition) = catalog.study(&candidate.study_id) else {
                excluded.insert(
                    candidate.study_id.clone(),
                    LeaderExclusionReason::UnknownStudy,
                );
                continue;
            };
            if self.owned_finite.contains(&candidate.study_id) {
                excluded.insert(
                    candidate.study_id.clone(),
                    LeaderExclusionReason::AlreadyOwned,
                );
                continue;
            }
            if self.is_repeatable(&candidate.study_id) && finite_remaining {
                excluded.insert(
                    candidate.study_id.clone(),
                    LeaderExclusionReason::RepeatableWhileFiniteRemains,
                );
                continue;
            }
            if !definition
                .prerequisites
                .iter()
                .all(|id| self.owned_finite.contains(id))
            {
                excluded.insert(
                    candidate.study_id.clone(),
                    LeaderExclusionReason::PrerequisiteLocked,
                );
                continue;
            }
            let queued = self
                .god_queue
                .values()
                .any(|entry| entry.study_id == candidate.study_id);
            if queued && !duplicate_permit.permits_queued_target() {
                excluded.insert(
                    candidate.study_id.clone(),
                    LeaderExclusionReason::GodQueueDuplicateForbidden,
                );
                continue;
            }
            legal.push(candidate);
        }
        legal.sort_by(|left, right| {
            right
                .score()
                .cmp(&left.score())
                .then_with(|| left.study_id.cmp(&right.study_id))
        });
        let selected = legal.first().map(|candidate| candidate.study_id.clone());
        let selected_duplicate_authorization = selected
            .as_ref()
            .filter(|study_id| {
                self.god_queue
                    .values()
                    .any(|entry| &entry.study_id == *study_id)
            })
            .map_or(DuplicateAuthorization::None, |_| duplicate_permit.kind());
        LeaderSelection {
            selected,
            selected_duplicate_authorization,
            excluded,
        }
    }

    /// Atomically applies a versioned command.  Same-ID same-payload replays
    /// return their original receipt; a changed payload is a hard conflict.
    pub fn apply(
        &mut self,
        catalog: &ProgressionCatalog,
        command: ResearchCommand,
    ) -> Result<ResearchReceipt, ResearchAuthorityError> {
        self.validate_against(catalog)?;
        if let Some(receipt) = self.receipts.get(&command.id) {
            return if receipt.command == command {
                Ok(receipt.clone())
            } else {
                Err(ResearchAuthorityError::IdempotencyConflict)
            };
        }
        if command.expected_version != self.version {
            return Err(ResearchAuthorityError::StaleVersion);
        }
        if self.receipts.len() >= MAX_RESEARCH_RECEIPTS {
            return Err(ResearchAuthorityError::Backpressure);
        }
        let mut next = self.clone();
        let outcome = next.apply_new(catalog, &command)?;
        next.version = next
            .version
            .checked_add(1)
            .ok_or(ResearchAuthorityError::ArithmeticOverflow)?;
        let receipt = ResearchReceipt {
            command: command.clone(),
            outcome,
            committed_version: next.version,
        };
        next.receipts.insert(command.id.clone(), receipt.clone());
        next.validate_against(catalog)?;
        *self = next;
        Ok(receipt)
    }

    fn apply_new(
        &mut self,
        catalog: &ProgressionCatalog,
        command: &ResearchCommand,
    ) -> Result<ResearchCommandOutcome, ResearchAuthorityError> {
        match &command.kind {
            ResearchCommandKind::QueueGodPath { target } => {
                let added = self.queue_god_path(catalog, target)?;
                Ok(ResearchCommandOutcome::Queued {
                    entries_added: u8::try_from(added)
                        .map_err(|_| ResearchAuthorityError::Backpressure)?,
                })
            }
            ResearchCommandKind::FundGodFront {
                consume_preparation,
            } => self.fund_front(catalog, &command.id, *consume_preparation),
            ResearchCommandKind::RequestPreparation { study_id } => {
                self.request_preparation(catalog, study_id)
            }
            ResearchCommandKind::PerformPreparation {
                study_id,
                staffed_research_station,
                scholar_alive,
                labor_minutes,
            } => self.perform_preparation(
                catalog,
                study_id,
                *staffed_research_station,
                *scholar_alive,
                *labor_minutes,
            ),
            ResearchCommandKind::PerformGodLabor {
                staffed_research_station,
                scholar_alive,
                labor_minutes,
            } => self.perform_god_labor(
                catalog,
                *staffed_research_station,
                *scholar_alive,
                *labor_minutes,
            ),
            ResearchCommandKind::RemoveGodTarget { study_id } => {
                self.remove_god_target(catalog, study_id)
            }
            ResearchCommandKind::ReorderGodTarget {
                study_id,
                before_study_id,
            } => self.reorder_god_target(catalog, study_id, before_study_id.as_ref()),
            ResearchCommandKind::CompleteLeader {
                study_id,
                effective_loremaster_level,
                now_tick,
                duplicate_permit,
            } => self.complete_leader(
                catalog,
                command.id.clone(),
                study_id,
                *effective_loremaster_level,
                *now_tick,
                duplicate_permit,
            ),
        }
    }

    fn queue_god_path(
        &mut self,
        catalog: &ProgressionCatalog,
        target: &StudyId,
    ) -> Result<usize, ResearchAuthorityError> {
        let mut visiting = BTreeSet::new();
        let mut path = Vec::new();
        self.collect_missing_path(catalog, target, &mut visiting, &mut path)?;
        if self
            .god_queue
            .len()
            .checked_add(path.len())
            .ok_or(ResearchAuthorityError::ArithmeticOverflow)?
            > MAX_GOD_QUEUE_ENTRIES
        {
            return Err(ResearchAuthorityError::QueueFull);
        }
        for study_id in &path {
            let sequence = self.next_queue_sequence;
            self.next_queue_sequence = self
                .next_queue_sequence
                .checked_add(1)
                .ok_or(ResearchAuthorityError::ArithmeticOverflow)?;
            self.god_queue.insert(
                sequence,
                GodResearchEntry {
                    sequence,
                    study_id: study_id.clone(),
                    frozen: None,
                    staffed_labor_minutes: 0,
                },
            );
        }
        Ok(path.len())
    }

    fn collect_missing_path(
        &self,
        catalog: &ProgressionCatalog,
        study_id: &StudyId,
        visiting: &mut BTreeSet<StudyId>,
        path: &mut Vec<StudyId>,
    ) -> Result<(), ResearchAuthorityError> {
        if self.owned_finite.contains(study_id)
            || self
                .god_queue
                .values()
                .any(|entry| &entry.study_id == study_id)
        {
            return Ok(());
        }
        if !visiting.insert(study_id.clone()) {
            return Err(ResearchAuthorityError::Cycle);
        }
        let definition = catalog
            .study(study_id)
            .ok_or(ResearchAuthorityError::UnknownStudy)?;
        for prerequisite in &definition.prerequisites {
            self.collect_missing_path(catalog, prerequisite, visiting, path)?;
        }
        visiting.remove(study_id);
        path.push(study_id.clone());
        Ok(())
    }

    fn fund_front(
        &mut self,
        catalog: &ProgressionCatalog,
        command_id: &ResearchCommandId,
        consume_preparation: bool,
    ) -> Result<ResearchCommandOutcome, ResearchAuthorityError> {
        let Some(sequence) = self.god_queue.keys().next().copied() else {
            return Err(ResearchAuthorityError::NoGodFront);
        };
        let entry = self
            .god_queue
            .get(&sequence)
            .ok_or(ResearchAuthorityError::NoGodFront)?;
        if let Some(terms) = &entry.frozen {
            return Ok(ResearchCommandOutcome::Funded {
                payable_cost_micro: terms.payable_cost_micro,
            });
        }
        let study_id = entry.study_id.clone();
        let definition = catalog
            .study(&study_id)
            .ok_or(ResearchAuthorityError::UnknownStudy)?;
        let definition_currency = definition.currency();
        let base_cost_micro = definition.cost_micro;
        let duration_game_minutes = definition.required_work_minutes;
        let can_consume = consume_preparation
            && definition_currency == StudyCurrency::Notes
            && self
                .preparations
                .get(&study_id)
                .is_some_and(PreparationState::complete);
        if consume_preparation && !can_consume {
            return Err(ResearchAuthorityError::PreparationUnavailable);
        }
        let discount = if can_consume { base_cost_micro / 4 } else { 0 };
        let payable = base_cost_micro
            .checked_sub(discount)
            .ok_or(ResearchAuthorityError::ArithmeticOverflow)?;
        let event_id =
            CurrencyEventId::derive("lai58_god_front", &self.colony_id, command_id.as_str());
        let fingerprint = currency_fingerprint(command_id, &study_id, payable);
        match definition_currency {
            StudyCurrency::Notes => self
                .notes
                .debit(ResearchNotesSpendRequest {
                    id: event_id,
                    amount: ResearchNotes::from_micro(payable),
                    expected_version: self.notes.version,
                    fingerprint,
                })
                .map_err(map_notes_debit_error)?,
            StudyCurrency::Void => self
                .void
                .debit(VoidSpendRequest {
                    id: event_id,
                    amount: VoidInsight::from_micro(payable),
                    purpose: match &definition.kind {
                        StudyKind::HoleAxis { .. } => VoidDebitPurpose::HoleStudy,
                        StudyKind::BoostUnlock { .. }
                        | StudyKind::BoostDuration { .. }
                        | StudyKind::BoostEconomy { .. } => VoidDebitPurpose::BoostStudy,
                        StudyKind::OrdinaryCapability { .. } => {
                            return Err(ResearchAuthorityError::CanonicalLedgerRejected);
                        }
                    },
                    expected_version: self.void.version,
                    fingerprint,
                })
                .map_err(map_void_debit_error)?,
        };
        if can_consume {
            self.preparations.remove(&study_id);
        }
        let entry = self
            .god_queue
            .get_mut(&sequence)
            .ok_or(ResearchAuthorityError::NoGodFront)?;
        entry.frozen = Some(FrozenGodTerms {
            currency: definition_currency,
            base_cost_micro,
            payable_cost_micro: payable,
            duration_game_minutes,
        });
        Ok(ResearchCommandOutcome::Funded {
            payable_cost_micro: payable,
        })
    }

    fn request_preparation(
        &mut self,
        catalog: &ProgressionCatalog,
        study_id: &StudyId,
    ) -> Result<ResearchCommandOutcome, ResearchAuthorityError> {
        let front = self.god_front().ok_or(ResearchAuthorityError::NoGodFront)?;
        if front.study_id != *study_id || front.frozen.is_some() {
            return Err(ResearchAuthorityError::PreparationMustTargetOrdinaryUnfundedFront);
        }
        let definition = catalog
            .study(study_id)
            .ok_or(ResearchAuthorityError::UnknownStudy)?;
        if !definition.kind.is_ordinary() {
            return Err(ResearchAuthorityError::PreparationMustTargetOrdinaryUnfundedFront);
        }
        if self.preparations.contains_key(study_id) {
            return Err(ResearchAuthorityError::PreparationAlreadyExists);
        }
        let required_labor_minutes = definition.required_work_minutes.div_ceil(4);
        if required_labor_minutes == 0 {
            return Err(ResearchAuthorityError::MalformedPersistence);
        }
        self.preparations.insert(
            study_id.clone(),
            PreparationState {
                study_id: study_id.clone(),
                required_labor_minutes,
                completed_labor_minutes: 0,
            },
        );
        Ok(ResearchCommandOutcome::PreparationRequested {
            study_id: study_id.clone(),
            required_labor_minutes,
        })
    }

    fn perform_preparation(
        &mut self,
        catalog: &ProgressionCatalog,
        study_id: &StudyId,
        staffed_research_station: bool,
        scholar_alive: bool,
        labor_minutes: u64,
    ) -> Result<ResearchCommandOutcome, ResearchAuthorityError> {
        if !staffed_research_station || !scholar_alive {
            return Err(ResearchAuthorityError::ResearchInfrastructureUnavailable);
        }
        if labor_minutes == 0 {
            return Err(ResearchAuthorityError::MalformedCommand);
        }
        let front = self.god_front().ok_or(ResearchAuthorityError::NoGodFront)?;
        if front.study_id != *study_id || front.frozen.is_some() {
            return Err(ResearchAuthorityError::PreparationMustTargetOrdinaryUnfundedFront);
        }
        let definition = catalog
            .study(study_id)
            .ok_or(ResearchAuthorityError::UnknownStudy)?;
        if !definition.kind.is_ordinary() {
            return Err(ResearchAuthorityError::PreparationMustTargetOrdinaryUnfundedFront);
        }
        let required = definition.required_work_minutes.div_ceil(4);
        let state = self
            .preparations
            .get_mut(study_id)
            .ok_or(ResearchAuthorityError::PreparationNotRequested)?;
        if state.required_labor_minutes != required {
            return Err(ResearchAuthorityError::MalformedPersistence);
        }
        if state.complete() {
            return Err(ResearchAuthorityError::PreparationAlreadyExists);
        }
        state.completed_labor_minutes = state
            .completed_labor_minutes
            .checked_add(labor_minutes)
            .ok_or(ResearchAuthorityError::ArithmeticOverflow)?
            .min(required);
        Ok(ResearchCommandOutcome::PreparationAdvanced {
            remaining_labor_minutes: required - state.completed_labor_minutes,
        })
    }

    fn perform_god_labor(
        &mut self,
        catalog: &ProgressionCatalog,
        staffed_research_station: bool,
        scholar_alive: bool,
        labor_minutes: u64,
    ) -> Result<ResearchCommandOutcome, ResearchAuthorityError> {
        if !staffed_research_station || !scholar_alive {
            return Err(ResearchAuthorityError::ResearchInfrastructureUnavailable);
        }
        if labor_minutes == 0 {
            return Err(ResearchAuthorityError::MalformedCommand);
        }
        let sequence = self
            .god_queue
            .keys()
            .next()
            .copied()
            .ok_or(ResearchAuthorityError::NoGodFront)?;
        let entry = self
            .god_queue
            .get_mut(&sequence)
            .ok_or(ResearchAuthorityError::NoGodFront)?;
        let terms = entry
            .frozen
            .as_ref()
            .ok_or(ResearchAuthorityError::GodFrontUnfunded)?
            .clone();
        entry.staffed_labor_minutes = entry
            .staffed_labor_minutes
            .checked_add(labor_minutes)
            .ok_or(ResearchAuthorityError::ArithmeticOverflow)?
            .min(terms.duration_game_minutes);
        if entry.staffed_labor_minutes < terms.duration_game_minutes {
            return Ok(ResearchCommandOutcome::GodLaborAdvanced {
                remaining_labor_minutes: terms.duration_game_minutes - entry.staffed_labor_minutes,
            });
        }
        let study_id = entry.study_id.clone();
        self.god_queue.remove(&sequence);
        if self.is_repeatable(&study_id) {
            let completed = self
                .repeatable_completions
                .entry(study_id.clone())
                .or_default();
            *completed = completed
                .checked_add(1)
                .ok_or(ResearchAuthorityError::ArithmeticOverflow)?;
        } else {
            let definition = catalog
                .study(&study_id)
                .ok_or(ResearchAuthorityError::UnknownStudy)?;
            if !definition
                .prerequisites
                .iter()
                .all(|prerequisite| self.owned_finite.contains(prerequisite))
            {
                return Err(ResearchAuthorityError::PrerequisiteLocked);
            }
            self.owned_finite.insert(study_id.clone());
        }
        self.preparations.remove(&study_id);
        Ok(ResearchCommandOutcome::GodStudyCompleted { study_id })
    }

    fn reorder_god_target(
        &mut self,
        catalog: &ProgressionCatalog,
        study_id: &StudyId,
        before_study_id: Option<&StudyId>,
    ) -> Result<ResearchCommandOutcome, ResearchAuthorityError> {
        let mut ordered = self.god_queue.values().cloned().collect::<Vec<_>>();
        let from_index = ordered
            .iter()
            .position(|entry| &entry.study_id == study_id)
            .ok_or(ResearchAuthorityError::TargetNotQueued)?;
        if before_study_id == Some(study_id) {
            return Err(ResearchAuthorityError::NoOpReorder);
        }
        if let Some(before) = before_study_id
            && !ordered.iter().any(|entry| &entry.study_id == before)
        {
            return Err(ResearchAuthorityError::TargetNotQueued);
        }

        let original_order = ordered
            .iter()
            .map(|entry| entry.study_id.clone())
            .collect::<Vec<_>>();
        let funded_front = ordered
            .first()
            .filter(|entry| entry.frozen.is_some())
            .map(|entry| entry.study_id.clone());
        let moved = ordered.remove(from_index);
        let to_index = if let Some(before) = before_study_id {
            ordered
                .iter()
                .position(|entry| &entry.study_id == before)
                .ok_or(ResearchAuthorityError::TargetNotQueued)?
        } else {
            ordered.len()
        };
        ordered.insert(to_index, moved);
        if ordered
            .iter()
            .map(|entry| &entry.study_id)
            .eq(original_order.iter())
        {
            return Err(ResearchAuthorityError::NoOpReorder);
        }
        if funded_front
            .as_ref()
            .is_some_and(|front| ordered.first().is_none_or(|entry| &entry.study_id != front))
        {
            return Err(ResearchAuthorityError::GodFrontFrozen);
        }
        if !god_queue_order_is_topological(catalog, &self.owned_finite, &ordered) {
            return Err(ResearchAuthorityError::GodQueuePrerequisiteOrder);
        }

        let mut rebuilt = BTreeMap::new();
        for (index, mut entry) in ordered.into_iter().enumerate() {
            let sequence = u64::try_from(index)
                .ok()
                .and_then(|index| index.checked_add(1))
                .ok_or(ResearchAuthorityError::ArithmeticOverflow)?;
            entry.sequence = sequence;
            if rebuilt.insert(sequence, entry).is_some() {
                return Err(ResearchAuthorityError::ArithmeticOverflow);
            }
        }
        self.next_queue_sequence = u64::try_from(rebuilt.len())
            .ok()
            .and_then(|length| length.checked_add(1))
            .ok_or(ResearchAuthorityError::ArithmeticOverflow)?;
        self.god_queue = rebuilt;
        Ok(ResearchCommandOutcome::GodTargetReordered {
            study_id: study_id.clone(),
            from_index: u8::try_from(from_index)
                .map_err(|_| ResearchAuthorityError::Backpressure)?,
            to_index: u8::try_from(to_index).map_err(|_| ResearchAuthorityError::Backpressure)?,
        })
    }

    fn remove_god_target(
        &mut self,
        catalog: &ProgressionCatalog,
        target: &StudyId,
    ) -> Result<ResearchCommandOutcome, ResearchAuthorityError> {
        let keys = self.removal_keys(catalog, target)?;
        let removal = self.remove_keys(&keys)?;
        Ok(ResearchCommandOutcome::GodTargetRemoved {
            removed_entries: u8::try_from(keys.len())
                .map_err(|_| ResearchAuthorityError::Backpressure)?,
            refunded_notes_micro: removal.refunded_notes_micro,
            refunded_void_micro: removal.refunded_void_micro,
            lost_labor_minutes: removal.lost_labor_minutes,
            lost_preparation_labor_minutes: removal.lost_preparation_labor_minutes,
        })
    }

    fn complete_leader(
        &mut self,
        catalog: &ProgressionCatalog,
        command_id: ResearchCommandId,
        study_id: &StudyId,
        effective_loremaster_level: u8,
        now_tick: u64,
        duplicate_permit: &LeaderDuplicatePermit,
    ) -> Result<ResearchCommandOutcome, ResearchAuthorityError> {
        if self.leader_used_in_window(now_tick)
            >= Self::leader_cadence_limit(effective_loremaster_level)
        {
            return Err(ResearchAuthorityError::LeaderCadenceExhausted);
        }
        let definition = catalog
            .study(study_id)
            .ok_or(ResearchAuthorityError::UnknownStudy)?;
        if self.owned_finite.contains(study_id) {
            return Err(ResearchAuthorityError::AlreadyOwned);
        }
        if self.is_repeatable(study_id) && self.has_unowned_finite(catalog) {
            return Err(ResearchAuthorityError::FiniteResearchRemaining);
        }
        if !definition
            .prerequisites
            .iter()
            .all(|prerequisite| self.owned_finite.contains(prerequisite))
        {
            return Err(ResearchAuthorityError::PrerequisiteLocked);
        }
        let queued = self
            .god_queue
            .values()
            .any(|entry| entry.study_id == *study_id);
        if queued && !duplicate_permit.permits_queued_target() {
            return Err(ResearchAuthorityError::GodQueueDuplicateForbidden);
        }
        let removal = if queued {
            let keys = self.removal_keys(catalog, study_id)?;
            self.remove_keys(&keys)?
        } else {
            RemovalAccounting::default()
        };
        if self.is_repeatable(study_id) {
            let completed = self
                .repeatable_completions
                .entry(study_id.clone())
                .or_default();
            *completed = completed
                .checked_add(1)
                .ok_or(ResearchAuthorityError::ArithmeticOverflow)?;
        } else {
            self.owned_finite.insert(study_id.clone());
        }
        self.leader_commits.retain(|_, commit| {
            commit
                .committed_tick
                .saturating_add(ROLLING_SEVEN_GAME_DAYS_MINUTES)
                > now_tick
        });
        if self.leader_commits.len() >= MAX_LEADER_COMMITS {
            return Err(ResearchAuthorityError::Backpressure);
        }
        self.leader_commits.insert(
            command_id,
            LeaderCommit {
                study_id: study_id.clone(),
                committed_tick: now_tick,
                effective_loremaster_level,
                duplicate_authorization: if queued {
                    duplicate_permit.kind()
                } else {
                    DuplicateAuthorization::None
                },
            },
        );
        Ok(ResearchCommandOutcome::LeaderCompleted {
            study_id: study_id.clone(),
            duplicate_authorization: if queued {
                duplicate_permit.kind()
            } else {
                DuplicateAuthorization::None
            },
            refunded_notes_micro: removal.refunded_notes_micro,
            refunded_void_micro: removal.refunded_void_micro,
            lost_god_labor_minutes: removal.lost_labor_minutes,
            lost_preparation_labor_minutes: removal.lost_preparation_labor_minutes,
        })
    }

    fn removal_keys(
        &self,
        catalog: &ProgressionCatalog,
        target: &StudyId,
    ) -> Result<BTreeSet<u64>, ResearchAuthorityError> {
        let target_key = self
            .god_queue
            .iter()
            .find_map(|(key, entry)| (&entry.study_id == target).then_some(*key))
            .ok_or(ResearchAuthorityError::TargetNotQueued)?;
        let mut removed = BTreeSet::from([target_key]);
        loop {
            let mut changed = false;
            for (key, entry) in &self.god_queue {
                if removed.contains(key) {
                    continue;
                }
                let definition = catalog
                    .study(&entry.study_id)
                    .ok_or(ResearchAuthorityError::UnknownStudy)?;
                if definition.prerequisites.iter().any(|prerequisite| {
                    self.god_queue.iter().any(|(candidate_key, candidate)| {
                        removed.contains(candidate_key) && candidate.study_id == *prerequisite
                    })
                }) {
                    removed.insert(*key);
                    changed = true;
                }
            }
            if !changed {
                return Ok(removed);
            }
        }
    }

    fn remove_keys(
        &mut self,
        keys: &BTreeSet<u64>,
    ) -> Result<RemovalAccounting, ResearchAuthorityError> {
        let mut accounting = RemovalAccounting::default();
        for key in keys {
            let entry = self
                .god_queue
                .remove(key)
                .ok_or(ResearchAuthorityError::TargetNotQueued)?;
            accounting.lost_labor_minutes = accounting
                .lost_labor_minutes
                .checked_add(entry.staffed_labor_minutes)
                .ok_or(ResearchAuthorityError::ArithmeticOverflow)?;
            if let Some(terms) = entry.frozen {
                match terms.currency {
                    StudyCurrency::Notes => {
                        self.notes.balance = self
                            .notes
                            .balance
                            .checked_add(ResearchNotes::from_micro(terms.payable_cost_micro))
                            .map_err(|_| ResearchAuthorityError::ArithmeticOverflow)?;
                        self.notes.version = self
                            .notes
                            .version
                            .checked_add(1)
                            .ok_or(ResearchAuthorityError::ArithmeticOverflow)?;
                        accounting.refunded_notes_micro = accounting
                            .refunded_notes_micro
                            .checked_add(terms.payable_cost_micro)
                            .ok_or(ResearchAuthorityError::ArithmeticOverflow)?;
                    }
                    StudyCurrency::Void => {
                        self.void.balance = self
                            .void
                            .balance
                            .checked_add(VoidInsight::from_micro(terms.payable_cost_micro))
                            .map_err(|_| ResearchAuthorityError::ArithmeticOverflow)?;
                        self.void.version = self
                            .void
                            .version
                            .checked_add(1)
                            .ok_or(ResearchAuthorityError::ArithmeticOverflow)?;
                        accounting.refunded_void_micro = accounting
                            .refunded_void_micro
                            .checked_add(terms.payable_cost_micro)
                            .ok_or(ResearchAuthorityError::ArithmeticOverflow)?;
                    }
                }
            }
            if let Some(preparation) = self.preparations.remove(&entry.study_id) {
                accounting.lost_preparation_labor_minutes = accounting
                    .lost_preparation_labor_minutes
                    .checked_add(preparation.completed_labor_minutes)
                    .ok_or(ResearchAuthorityError::ArithmeticOverflow)?;
            }
        }
        Ok(accounting)
    }

    /// The authority does not invent an alternate graph.  LAI.44's canonical
    /// finite/infinite terminal distinction is represented by its explicit
    /// stage-eleven modifier study kinds; the richer presentation manifest is
    /// a projection of the same stable study IDs.
    fn is_repeatable(&self, study_id: &StudyId) -> bool {
        ProgressionCatalog::from_embedded()
            .ok()
            .and_then(|catalog| {
                catalog.study(study_id).map(|study| {
                    matches!(
                        study.kind,
                        StudyKind::BoostDuration { stage: 11 }
                            | StudyKind::BoostEconomy { stage: 11 }
                    )
                })
            })
            .unwrap_or(false)
    }

    fn has_unowned_finite(&self, catalog: &ProgressionCatalog) -> bool {
        catalog
            .studies()
            .keys()
            .any(|study_id| !self.is_repeatable(study_id) && !self.owned_finite.contains(study_id))
    }

    fn validate_against(&self, catalog: &ProgressionCatalog) -> Result<(), ResearchAuthorityError> {
        if self.schema_version != RESEARCH_AUTHORITY_SCHEMA_VERSION
            || self.next_queue_sequence == 0
            || self.god_queue.len() > MAX_GOD_QUEUE_ENTRIES
            || self.preparations.len() > MAX_RESEARCH_REPORT_PREPARATIONS
            || self.receipts.len() > MAX_RESEARCH_RECEIPTS
            || self.leader_commits.len() > MAX_LEADER_COMMITS
            || self.notes.partition
                != (ColonyPartitionKey {
                    colony_id: self.colony_id.clone(),
                })
            || self.void.partition
                != (ColonyPartitionKey {
                    colony_id: self.colony_id.clone(),
                })
        {
            return Err(ResearchAuthorityError::MalformedPersistence);
        }
        if self
            .god_queue
            .iter()
            .any(|(key, entry)| key != &entry.sequence || catalog.study(&entry.study_id).is_none())
            || self
                .god_queue
                .keys()
                .next_back()
                .is_some_and(|sequence| *sequence >= self.next_queue_sequence)
            || !god_queue_order_is_topological(
                catalog,
                &self.owned_finite,
                &self.god_queue.values().cloned().collect::<Vec<_>>(),
            )
        {
            return Err(ResearchAuthorityError::MalformedPersistence);
        }
        if self
            .god_queue
            .values()
            .filter(|entry| entry.frozen.is_some())
            .skip(1)
            .next()
            .is_some()
            || self
                .god_queue
                .values()
                .skip(1)
                .any(|entry| entry.frozen.is_some())
        {
            return Err(ResearchAuthorityError::MalformedPersistence);
        }
        if self.preparations.iter().any(|(study_id, state)| {
            let Some(definition) = catalog.study(study_id) else {
                return true;
            };
            study_id != &state.study_id
                || !definition.kind.is_ordinary()
                || !self
                    .god_queue
                    .values()
                    .any(|entry| &entry.study_id == study_id)
                || state.required_labor_minutes != definition.required_work_minutes.div_ceil(4)
                || state.required_labor_minutes == 0
                || state.completed_labor_minutes > state.required_labor_minutes
        }) {
            return Err(ResearchAuthorityError::MalformedPersistence);
        }
        if self.receipts.iter().any(|(id, receipt)| {
            id != &receipt.command.id
                || receipt.committed_version == 0
                || receipt.committed_version > self.version
        }) {
            return Err(ResearchAuthorityError::MalformedPersistence);
        }
        Ok(())
    }
}

fn god_queue_order_is_topological(
    catalog: &ProgressionCatalog,
    owned_finite: &BTreeSet<StudyId>,
    ordered: &[GodResearchEntry],
) -> bool {
    let mut available = owned_finite.clone();
    let mut queued = BTreeSet::new();
    for entry in ordered {
        let Some(definition) = catalog.study(&entry.study_id) else {
            return false;
        };
        if !queued.insert(entry.study_id.clone())
            || !definition
                .prerequisites
                .iter()
                .all(|prerequisite| available.contains(prerequisite))
        {
            return false;
        }
        available.insert(entry.study_id.clone());
    }
    true
}

#[derive(Clone, Copy, Debug, Default)]
struct RemovalAccounting {
    refunded_notes_micro: u64,
    refunded_void_micro: u64,
    lost_labor_minutes: u64,
    lost_preparation_labor_minutes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResearchQueueReport {
    pub study_id: StudyId,
    pub frozen: bool,
    pub staffed_labor_minutes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparationReport {
    pub study_id: StudyId,
    pub required_labor_minutes: u64,
    pub completed_labor_minutes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResearchReportProjection {
    pub version: u64,
    pub notes_balance: ResearchNotes,
    pub void_balance: VoidInsight,
    pub owned_finite: Vec<StudyId>,
    pub god_queue: Vec<ResearchQueueReport>,
    pub preparations: Vec<PreparationReport>,
}

fn currency_fingerprint(command_id: &ResearchCommandId, study_id: &StudyId, payable: u64) -> u64 {
    let mut value = 0xcbf2_9ce4_8422_2325_u64;
    for part in [command_id.as_str().as_bytes(), study_id.as_str().as_bytes()] {
        for byte in part {
            value ^= u64::from(*byte);
            value = value.wrapping_mul(0x0000_0100_0000_01b3);
        }
        value ^= 0xff;
        value = value.wrapping_mul(0x0000_0100_0000_01b3);
    }
    for byte in payable.to_le_bytes() {
        value ^= u64::from(byte);
        value = value.wrapping_mul(0x0000_0100_0000_01b3);
    }
    value
}

fn map_notes_debit_error(
    error: crate::progression_research::ProgressionError,
) -> ResearchAuthorityError {
    match error {
        crate::progression_research::ProgressionError::InsufficientCurrency => {
            ResearchAuthorityError::InsufficientNotes
        }
        crate::progression_research::ProgressionError::ArithmeticOverflow => {
            ResearchAuthorityError::ArithmeticOverflow
        }
        crate::progression_research::ProgressionError::StaleVersion => {
            ResearchAuthorityError::LedgerVersionConflict
        }
        _ => ResearchAuthorityError::CanonicalLedgerRejected,
    }
}

fn map_void_debit_error(
    error: crate::progression_research::ProgressionError,
) -> ResearchAuthorityError {
    match error {
        crate::progression_research::ProgressionError::InsufficientCurrency => {
            ResearchAuthorityError::InsufficientVoid
        }
        crate::progression_research::ProgressionError::ArithmeticOverflow => {
            ResearchAuthorityError::ArithmeticOverflow
        }
        crate::progression_research::ProgressionError::StaleVersion => {
            ResearchAuthorityError::LedgerVersionConflict
        }
        _ => ResearchAuthorityError::CanonicalLedgerRejected,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResearchAuthorityError {
    StaleVersion,
    IdempotencyConflict,
    Backpressure,
    QueueFull,
    UnknownStudy,
    Cycle,
    NoGodFront,
    GodFrontUnfunded,
    PreparationUnavailable,
    PreparationNotRequested,
    PreparationAlreadyExists,
    PreparationMustTargetOrdinaryUnfundedFront,
    ResearchInfrastructureUnavailable,
    MalformedCommand,
    PrerequisiteLocked,
    AlreadyOwned,
    FiniteResearchRemaining,
    GodQueueDuplicateForbidden,
    LeaderCadenceExhausted,
    TargetNotQueued,
    NoOpReorder,
    GodFrontFrozen,
    GodQueuePrerequisiteOrder,
    InsufficientNotes,
    InsufficientVoid,
    LedgerVersionConflict,
    CanonicalLedgerRejected,
    ArithmeticOverflow,
    MalformedPersistence,
}
impl fmt::Display for ResearchAuthorityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "research authority rejected request ({self:?})")
    }
}
impl std::error::Error for ResearchAuthorityError {}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedResearchAuthority {
    schema_version: u32,
    colony_id: PlannerId,
    version: u64,
    notes: ResearchNotesLedger,
    void: VoidInsightLedger,
    owned_finite: BTreeSet<StudyId>,
    repeatable_completions: BTreeMap<StudyId, u32>,
    next_queue_sequence: u64,
    god_queue: BTreeMap<u64, GodResearchEntry>,
    preparations: BTreeMap<StudyId, PreparationState>,
    leader_commits: BTreeMap<ResearchCommandId, LeaderCommit>,
    receipts: BTreeMap<ResearchCommandId, ResearchReceipt>,
}
impl<'de> Deserialize<'de> for ResearchAuthority {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = UncheckedResearchAuthority::deserialize(deserializer)?;
        let value = Self {
            schema_version: raw.schema_version,
            colony_id: raw.colony_id,
            version: raw.version,
            notes: raw.notes,
            void: raw.void,
            owned_finite: raw.owned_finite,
            repeatable_completions: raw.repeatable_completions,
            next_queue_sequence: raw.next_queue_sequence,
            god_queue: raw.god_queue,
            preparations: raw.preparations,
            leader_commits: raw.leader_commits,
            receipts: raw.receipts,
        };
        let catalog = ProgressionCatalog::from_embedded().map_err(serde::de::Error::custom)?;
        value
            .validate_against(&catalog)
            .map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> ProgressionCatalog {
        ProgressionCatalog::from_embedded().expect("embedded progression catalog")
    }

    fn colony() -> PlannerId {
        PlannerId::derive("research-reorder-test", ["colony"])
    }

    fn authority() -> ResearchAuthority {
        ResearchAuthority::new(
            colony(),
            ResearchNotes::from_micro(u64::MAX / 4),
            VoidInsight::from_micro(u64::MAX / 4),
        )
    }

    fn command(
        state: &ResearchAuthority,
        action: &str,
        kind: ResearchCommandKind,
    ) -> ResearchCommand {
        ResearchCommand {
            id: ResearchCommandId::derive(&colony(), action),
            expected_version: state.version,
            kind,
        }
    }

    fn independent_roots(catalog: &ProgressionCatalog) -> Vec<StudyId> {
        catalog
            .studies()
            .values()
            .filter(|study| study.prerequisites.is_empty())
            .map(|study| study.id.clone())
            .take(3)
            .collect()
    }

    #[test]
    fn god_reorder_is_atomic_idempotent_and_preserves_every_entry_fact() {
        let catalog = catalog();
        let roots = independent_roots(&catalog);
        assert_eq!(roots.len(), 3);
        let mut state = authority();
        for (index, target) in roots.iter().enumerate() {
            let request = command(
                &state,
                &format!("queue-{index}"),
                ResearchCommandKind::QueueGodPath {
                    target: target.clone(),
                },
            );
            state
                .apply(&catalog, request)
                .expect("queue independent root");
        }
        let request_preparation = command(
            &state,
            "request-front-preparation",
            ResearchCommandKind::RequestPreparation {
                study_id: roots[0].clone(),
            },
        );
        state
            .apply(&catalog, request_preparation)
            .expect("open zero-progress preparation");
        let fund = command(
            &state,
            "fund-front",
            ResearchCommandKind::FundGodFront {
                consume_preparation: false,
            },
        );
        state.apply(&catalog, fund).expect("fund front");
        let second = state
            .god_queue
            .values_mut()
            .find(|entry| entry.study_id == roots[1])
            .expect("second root queued");
        second.staffed_labor_minutes = 7;
        let frozen_before = state
            .god_queue
            .values()
            .map(|entry| {
                (
                    entry.study_id.clone(),
                    (entry.frozen.clone(), entry.staffed_labor_minutes),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let preparations_before = state.preparations.clone();
        let reorder = command(
            &state,
            "reorder-third-before-second",
            ResearchCommandKind::ReorderGodTarget {
                study_id: roots[2].clone(),
                before_study_id: Some(roots[1].clone()),
            },
        );
        let receipt = state
            .apply(&catalog, reorder.clone())
            .expect("reorder independent roots");
        assert!(matches!(
            receipt.outcome,
            ResearchCommandOutcome::GodTargetReordered {
                from_index: 2,
                to_index: 1,
                ..
            }
        ));
        assert_eq!(
            state
                .god_queue
                .values()
                .map(|entry| entry.study_id.clone())
                .collect::<Vec<_>>(),
            vec![roots[0].clone(), roots[2].clone(), roots[1].clone()]
        );
        assert_eq!(
            state.god_queue.keys().copied().collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(state.next_queue_sequence, 4);
        assert_eq!(
            state
                .god_queue
                .values()
                .map(|entry| {
                    (
                        entry.study_id.clone(),
                        (entry.frozen.clone(), entry.staffed_labor_minutes),
                    )
                })
                .collect::<BTreeMap<_, _>>(),
            frozen_before
        );
        assert_eq!(state.preparations, preparations_before);
        assert_eq!(
            state
                .apply(&catalog, reorder)
                .expect("exact replay returns receipt"),
            receipt
        );
        let encoded = serde_json::to_string(&state).expect("serialize");
        let restored: ResearchAuthority = serde_json::from_str(&encoded).expect("strict restart");
        assert_eq!(restored, state);
    }

    #[test]
    fn god_reorder_rejects_noop_unknown_prerequisite_and_frozen_front_moves() {
        let catalog = catalog();
        let roots = independent_roots(&catalog);
        assert_eq!(roots.len(), 3);
        let mut roots_state = authority();
        for (index, target) in roots.iter().enumerate() {
            let request = command(
                &roots_state,
                &format!("roots-{index}"),
                ResearchCommandKind::QueueGodPath {
                    target: target.clone(),
                },
            );
            roots_state
                .apply(&catalog, request)
                .expect("queue independent root");
        }
        let before = roots_state.clone();
        assert_eq!(
            roots_state
                .apply(
                    &catalog,
                    command(
                        &roots_state,
                        "noop",
                        ResearchCommandKind::ReorderGodTarget {
                            study_id: roots[1].clone(),
                            before_study_id: Some(roots[2].clone()),
                        },
                    ),
                )
                .expect_err("same relative position is a no-op"),
            ResearchAuthorityError::NoOpReorder
        );
        assert_eq!(roots_state, before);
        assert_eq!(
            roots_state
                .apply(
                    &catalog,
                    command(
                        &roots_state,
                        "unknown-before",
                        ResearchCommandKind::ReorderGodTarget {
                            study_id: roots[1].clone(),
                            before_study_id: Some(
                                StudyId::new("not_queued").expect("stable study id"),
                            ),
                        },
                    ),
                )
                .expect_err("unknown relative target"),
            ResearchAuthorityError::TargetNotQueued
        );

        let dependent = catalog
            .studies()
            .values()
            .find(|study| !study.prerequisites.is_empty())
            .expect("catalog has a dependent study")
            .id
            .clone();
        let mut dependency_state = authority();
        dependency_state
            .apply(
                &catalog,
                command(
                    &dependency_state,
                    "dependent-path",
                    ResearchCommandKind::QueueGodPath {
                        target: dependent.clone(),
                    },
                ),
            )
            .expect("queue dependent path");
        let prerequisite = dependency_state
            .god_queue
            .values()
            .find(|entry| entry.study_id != dependent)
            .expect("queued prerequisite")
            .study_id
            .clone();
        assert_eq!(
            dependency_state
                .apply(
                    &catalog,
                    command(
                        &dependency_state,
                        "invalid-topology",
                        ResearchCommandKind::ReorderGodTarget {
                            study_id: dependent.clone(),
                            before_study_id: Some(prerequisite.clone()),
                        },
                    ),
                )
                .expect_err("dependent cannot precede prerequisite"),
            ResearchAuthorityError::GodQueuePrerequisiteOrder
        );
        dependency_state
            .apply(
                &catalog,
                command(
                    &dependency_state,
                    "fund-prerequisite",
                    ResearchCommandKind::FundGodFront {
                        consume_preparation: false,
                    },
                ),
            )
            .expect("fund front");
        let funded_front = dependency_state
            .god_front()
            .expect("funded front remains queued")
            .study_id
            .clone();
        assert_eq!(
            dependency_state
                .apply(
                    &catalog,
                    command(
                        &dependency_state,
                        "move-before-frozen",
                        ResearchCommandKind::ReorderGodTarget {
                            study_id: dependent,
                            before_study_id: Some(funded_front),
                        },
                    ),
                )
                .expect_err("funded front cannot move"),
            ResearchAuthorityError::GodFrontFrozen
        );
    }
}
