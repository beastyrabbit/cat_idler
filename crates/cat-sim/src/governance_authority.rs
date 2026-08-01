//! Canonical, versioned LAI.57 governance lifecycle authority.
//!
//! This leaf composes [`crate::cat_governance`] for the exact ballot and
//! expulsion rules and [`crate::officer_expertise`] for report-safe imperfect
//! appointments and succession. It deliberately emits cleanup intents instead
//! of mutating jobs, cargo, residence, equipment, or partnership authorities.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    cat_governance::{
        BackingEligibility, BallotSignals, CatElectionState, CatVoter, CivicCandidate,
        CivicMeritMetrics, ElectionResult, ElectionTrigger, ExpulsionPlan, ExpulsionRequest,
        ExpulsionResident, ExpulsionScope, GovernanceError, GovernanceLifeStage,
        RelationalAnalyticalAxis, VoterCandidateView, cast_cat_ballot, select_civic_slate,
    },
    officer_expertise::{
        AppointmentCandidate, AppointmentSelection, ExpertiseLevel, InstitutionError,
        OfficerInstitutionState,
    },
    officers::OfficerRole,
    planner_core::PlannerId,
};

pub const GOVERNANCE_AUTHORITY_SCHEMA_VERSION: u32 = 1;
pub const MAX_GOVERNANCE_RESIDENTS: usize = 4_096;
pub const MAX_GOVERNANCE_ELECTIONS: usize = 256;
pub const MAX_GOVERNANCE_EXPULSIONS: usize = 256;
pub const MAX_GOVERNANCE_RECEIPTS: usize = 2_048;
pub const MAX_GOVERNANCE_BACKING_PLAYERS: usize = 2_048;
pub const MAX_GOVERNANCE_ID_BYTES: usize = 160;

/// Information a candidate has made available to the election executor. It is
/// never returned by [`GovernanceReport`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CandidateBallotFacts {
    pub charisma: u16,
    pub care: u16,
    pub trust: u16,
    pub social_conduct: u16,
    pub personality_compatibility: u16,
    pub governance: u16,
    pub intelligence: u16,
    pub office_experience: u16,
    pub skill: u16,
    pub results: u16,
}

impl CandidateBallotFacts {
    fn signals(self) -> BallotSignals {
        BallotSignals {
            charisma: self.charisma,
            care: self.care,
            trust: self.trust,
            social_conduct: self.social_conduct,
            personality_compatibility: self.personality_compatibility,
            governance: self.governance,
            intelligence: self.intelligence,
            office_experience: self.office_experience,
            skill: self.skill,
            results: self.results,
        }
    }

    fn validate(self) -> Result<(), GovernanceAuthorityError> {
        if [
            self.charisma,
            self.care,
            self.trust,
            self.social_conduct,
            self.personality_compatibility,
            self.governance,
            self.intelligence,
            self.office_experience,
            self.skill,
            self.results,
        ]
        .into_iter()
        .any(|value| value > crate::cat_governance::SCORE_SCALE)
        {
            return Err(GovernanceAuthorityError::MalformedState);
        }
        Ok(())
    }
}

/// Stable resident facts owned by this lifecycle authority. `cat_id` and
/// `household_id` are real world IDs, not planner-derived replacements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GovernanceResidentFact {
    pub cat_id: String,
    pub household_id: String,
    pub life_stage: GovernanceLifeStage,
    pub resident: bool,
    pub alive: bool,
    pub barred: bool,
    pub guardian_id: Option<String>,
    pub axis: RelationalAnalyticalAxis,
    pub merit: CivicMeritMetrics,
    pub ballot_facts: CandidateBallotFacts,
    pub job_id: Option<String>,
    pub office_id: Option<String>,
    pub residence_id: Option<String>,
    pub enterprise_id: Option<String>,
    pub partnership_id: Option<String>,
    pub carried_cargo_ids: Vec<String>,
    pub reservation_ids: Vec<String>,
    pub owned_item_ids: Vec<String>,
    pub equipped_item_ids: Vec<String>,
}

impl GovernanceResidentFact {
    fn election_eligible(&self) -> bool {
        self.alive && self.resident && self.life_stage.election_eligible()
    }

    fn candidate(&self) -> CivicCandidate {
        CivicCandidate {
            cat_id: self.cat_id.clone(),
            life_stage: self.life_stage,
            resident: self.resident && self.alive,
            barred: self.barred,
            merit: self.merit,
        }
    }

    fn expulsion_resident(&self, is_leader: bool) -> ExpulsionResident {
        ExpulsionResident {
            cat_id: self.cat_id.clone(),
            household_id: self.household_id.clone(),
            life_stage: self.life_stage,
            guardian_id: self.guardian_id.clone(),
            is_leader,
            job_id: self.job_id.clone(),
            office_id: self.office_id.clone(),
            residence_id: self.residence_id.clone(),
            enterprise_id: self.enterprise_id.clone(),
            carried_cargo_ids: self.carried_cargo_ids.clone(),
            reservation_ids: self.reservation_ids.clone(),
            owned_item_ids: self.owned_item_ids.clone(),
            equipped_item_ids: self.equipped_item_ids.clone(),
        }
    }

    fn validate(&self) -> Result<(), GovernanceAuthorityError> {
        validate_id(&self.cat_id)?;
        validate_id(&self.household_id)?;
        if let Some(guardian_id) = &self.guardian_id {
            validate_id(guardian_id)?;
        }
        self.merit
            .validate()
            .map_err(GovernanceAuthorityError::Governance)?;
        self.ballot_facts.validate()?;
        validate_ids(&self.carried_cargo_ids)?;
        validate_ids(&self.reservation_ids)?;
        validate_ids(&self.owned_item_ids)?;
        validate_ids(&self.equipped_item_ids)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElectionLifecycle {
    Open,
    Resolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolvedElection {
    pub winner_cat_id: Option<String>,
    pub total_votes: BTreeMap<String, u32>,
    /// Persisted deterministic rank: total votes, civic merit, Governance, ID.
    pub tie_order: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ElectionRecord {
    election: CatElectionState,
    eligible_voter_ids: BTreeSet<String>,
    lifecycle: ElectionLifecycle,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    result: Option<ResolvedElection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorElectionView {
    pub election: CatElectionState,
    pub eligible_voter_ids: BTreeSet<String>,
    pub lifecycle: ElectionLifecycle,
    pub result: Option<ResolvedElection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupKind {
    Job,
    Office,
    Election,
    Residence,
    Enterprise,
    Cargo,
    Reservation,
    Equipment,
    Partnership,
    Departure,
}

impl CleanupKind {
    const ALL: [Self; 10] = [
        Self::Job,
        Self::Office,
        Self::Election,
        Self::Residence,
        Self::Enterprise,
        Self::Cargo,
        Self::Reservation,
        Self::Equipment,
        Self::Partnership,
        Self::Departure,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CleanupIntent {
    pub intent_id: String,
    pub cat_id: String,
    pub kind: CleanupKind,
    /// Frozen stable IDs give the owning authority exact work without granting
    /// this authority the right to perform it.
    pub referenced_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExpulsionLifecycleRecord {
    pub plan: ExpulsionPlan,
    pub intents: BTreeMap<String, CleanupIntent>,
    pub acknowledged_intent_ids: BTreeSet<String>,
    pub departure_reachable: bool,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpulsionPreview {
    pub expulsion_id: String,
    pub plan: ExpulsionPlan,
    pub intents: Vec<CleanupIntent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackingActor {
    pub eligibility: BackingEligibilityWire,
    /// A global-village request has no personal-owner exception. It must have a
    /// globally eligible actor, preventing a local owner from backing there.
    pub global_village: bool,
}

/// Serializable counterpart of the existing execution-only eligibility type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackingEligibilityWire {
    pub authenticated: bool,
    pub eligible_global_player: bool,
    pub personal_village_owner: bool,
}

impl From<BackingEligibilityWire> for BackingEligibility {
    fn from(value: BackingEligibilityWire) -> Self {
        Self {
            authenticated: value.authenticated,
            eligible_global_player: value.eligible_global_player,
            personal_village_owner: value.personal_village_owner,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackingCommand {
    pub idempotency_id: String,
    pub expected_version: u64,
    pub election_id: String,
    pub player_id: String,
    pub candidate_cat_id: String,
    pub actor: BackingActor,
    pub submitted_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackingReceipt {
    pub idempotency_id: String,
    pub command_fingerprint: u64,
    pub accepted_version: u64,
    pub election_id: String,
    pub player_id: String,
    pub candidate_cat_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackingOutcome {
    Applied(BackingReceipt),
    Replayed(BackingReceipt),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElectionOpenOutcome {
    pub election_id: String,
    pub created: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportSafeAppointmentCandidate {
    pub cat_id: String,
    pub believed_merit: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficerAppointmentOutcome {
    pub selected_cat_id: String,
    pub sampled_cat_ids: Vec<String>,
}

/// Projection-safe summary. It intentionally omits raw merit, signals, ballot
/// scores, guardian links, and cleanup references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GovernanceReport {
    pub version: u64,
    pub colony_id: String,
    pub leader_cat_id: Option<String>,
    pub elections: Vec<PublicElectionSummary>,
    pub pending_expulsion_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicElectionSummary {
    pub election_id: String,
    pub trigger: ElectionTrigger,
    pub lifecycle: ElectionLifecycle,
    pub candidate_cat_ids: Vec<String>,
    pub winner_cat_id: Option<String>,
    pub total_votes: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceAuthorityState {
    schema_version: u32,
    colony_id: String,
    version: u64,
    residents: BTreeMap<String, GovernanceResidentFact>,
    elections: BTreeMap<String, ElectionRecord>,
    scheduled_elections: BTreeMap<String, String>,
    snap_elections: BTreeMap<String, String>,
    expulsions: BTreeMap<String, ExpulsionLifecycleRecord>,
    receipts: BTreeMap<String, BackingReceipt>,
    leader_cat_id: Option<String>,
    officer_institution: OfficerInstitutionState,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedGovernanceAuthorityState {
    schema_version: u32,
    colony_id: String,
    version: u64,
    residents: BTreeMap<String, GovernanceResidentFact>,
    elections: BTreeMap<String, ElectionRecord>,
    scheduled_elections: BTreeMap<String, String>,
    snap_elections: BTreeMap<String, String>,
    expulsions: BTreeMap<String, ExpulsionLifecycleRecord>,
    receipts: BTreeMap<String, BackingReceipt>,
    leader_cat_id: Option<String>,
    officer_institution: OfficerInstitutionState,
}

impl<'de> Deserialize<'de> for GovernanceAuthorityState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = UncheckedGovernanceAuthorityState::deserialize(deserializer)?;
        let state = Self {
            schema_version: wire.schema_version,
            colony_id: wire.colony_id,
            version: wire.version,
            residents: wire.residents,
            elections: wire.elections,
            scheduled_elections: wire.scheduled_elections,
            snap_elections: wire.snap_elections,
            expulsions: wire.expulsions,
            receipts: wire.receipts,
            leader_cat_id: wire.leader_cat_id,
            officer_institution: wire.officer_institution,
        };
        state.validate().map_err(serde::de::Error::custom)?;
        Ok(state)
    }
}

impl GovernanceAuthorityState {
    pub fn new(colony_id: impl Into<String>) -> Result<Self, GovernanceAuthorityError> {
        let colony_id = colony_id.into();
        validate_id(&colony_id)?;
        Ok(Self {
            schema_version: GOVERNANCE_AUTHORITY_SCHEMA_VERSION,
            officer_institution: OfficerInstitutionState::new(colony_id.clone())?,
            colony_id,
            version: 0,
            residents: BTreeMap::new(),
            elections: BTreeMap::new(),
            scheduled_elections: BTreeMap::new(),
            snap_elections: BTreeMap::new(),
            expulsions: BTreeMap::new(),
            receipts: BTreeMap::new(),
            leader_cat_id: None,
        })
    }

    #[must_use]
    pub const fn version(&self) -> u64 {
        self.version
    }

    #[must_use]
    pub fn colony_id(&self) -> &str {
        &self.colony_id
    }

    #[must_use]
    pub fn resident(&self, cat_id: &str) -> Option<&GovernanceResidentFact> {
        self.residents.get(cat_id)
    }

    #[must_use]
    pub fn officer_institution(&self) -> &OfficerInstitutionState {
        &self.officer_institution
    }

    #[must_use]
    pub fn executor_election(&self, election_id: &str) -> Option<ExecutorElectionView> {
        self.elections
            .get(election_id)
            .map(|record| ExecutorElectionView {
                election: record.election.clone(),
                eligible_voter_ids: record.eligible_voter_ids.clone(),
                lifecycle: record.lifecycle,
                result: record.result.clone(),
            })
    }

    pub fn register_resident(
        &mut self,
        fact: GovernanceResidentFact,
    ) -> Result<(), GovernanceAuthorityError> {
        self.transact(|staged| staged.register_resident_inner(fact))
    }

    pub fn remove_resident(
        &mut self,
        cat_id: &str,
    ) -> Result<GovernanceResidentFact, GovernanceAuthorityError> {
        self.transact(|staged| staged.remove_resident_inner(cat_id))
    }

    pub fn open_scheduled_election(
        &mut self,
        schedule_key: &str,
        now_tick: u64,
        ticks_per_game_hour: u64,
    ) -> Result<ElectionOpenOutcome, GovernanceAuthorityError> {
        self.transact(|staged| {
            staged.open_scheduled_election_inner(schedule_key, now_tick, ticks_per_game_hour)
        })
    }

    pub fn open_snap_election(
        &mut self,
        trigger: ElectionTrigger,
        snap_key: &str,
        now_tick: u64,
    ) -> Result<ElectionOpenOutcome, GovernanceAuthorityError> {
        self.transact(|staged| staged.open_snap_election_inner(trigger, snap_key, now_tick))
    }

    pub fn submit_backing(
        &mut self,
        command: BackingCommand,
    ) -> Result<BackingOutcome, GovernanceAuthorityError> {
        self.transact(|staged| staged.submit_backing_inner(command))
    }

    pub fn resolve_election(
        &mut self,
        election_id: &str,
        now_tick: u64,
    ) -> Result<ResolvedElection, GovernanceAuthorityError> {
        self.transact(|staged| staged.resolve_election_inner(election_id, now_tick))
    }

    pub fn record_death(
        &mut self,
        cat_id: &str,
        now_tick: u64,
        ticks_per_game_hour: u64,
    ) -> Result<Option<ElectionOpenOutcome>, GovernanceAuthorityError> {
        self.transact(|staged| staged.record_death_inner(cat_id, now_tick, ticks_per_game_hour))
    }

    pub fn preview_expulsion(
        &mut self,
        expulsion_id: &str,
        target_cat_id: &str,
        scope: ExpulsionScope,
    ) -> Result<ExpulsionPreview, GovernanceAuthorityError> {
        self.transact(|staged| staged.preview_expulsion_inner(expulsion_id, target_cat_id, scope))
    }

    pub fn acknowledge_cleanup(
        &mut self,
        expulsion_id: &str,
        intent_id: &str,
        departure_reachable: bool,
    ) -> Result<(), GovernanceAuthorityError> {
        self.transact(|staged| {
            staged.acknowledge_cleanup_inner(expulsion_id, intent_id, departure_reachable)
        })
    }

    pub fn commit_expulsion(
        &mut self,
        expulsion_id: &str,
        now_tick: u64,
        ticks_per_game_hour: u64,
    ) -> Result<Option<ElectionOpenOutcome>, GovernanceAuthorityError> {
        self.transact(|staged| {
            staged.commit_expulsion_inner(expulsion_id, now_tick, ticks_per_game_hour)
        })
    }

    pub fn appoint_officer_from_reports(
        &mut self,
        role: OfficerRole,
        world_seed: u32,
        leader_effective_level: ExpertiseLevel,
        candidates: Vec<ReportSafeAppointmentCandidate>,
        now_tick: u64,
    ) -> Result<Option<OfficerAppointmentOutcome>, GovernanceAuthorityError> {
        self.transact(|staged| {
            staged.appoint_officer_from_reports_inner(
                role,
                world_seed,
                leader_effective_level,
                candidates,
                now_tick,
            )
        })
    }

    fn register_resident_inner(
        &mut self,
        fact: GovernanceResidentFact,
    ) -> Result<(), GovernanceAuthorityError> {
        fact.validate()?;
        if self.residents.contains_key(&fact.cat_id) {
            return Err(GovernanceAuthorityError::DuplicateResident);
        }
        if self.residents.len() >= MAX_GOVERNANCE_RESIDENTS {
            return Err(GovernanceAuthorityError::CapacityExceeded);
        }
        self.residents.insert(fact.cat_id.clone(), fact);
        self.bump_version()?;
        Ok(())
    }

    fn remove_resident_inner(
        &mut self,
        cat_id: &str,
    ) -> Result<GovernanceResidentFact, GovernanceAuthorityError> {
        validate_id(cat_id)?;
        if self.leader_cat_id.as_deref() == Some(cat_id) {
            return Err(GovernanceAuthorityError::LeaderVacancyRequired);
        }
        let resident = self
            .residents
            .remove(cat_id)
            .ok_or(GovernanceAuthorityError::UnknownResident)?;
        self.bump_version()?;
        Ok(resident)
    }

    /// A schedule key is an external, stable occurrence ID. Repeating the same
    /// key returns its already-persisted election and never opens another one.
    fn open_scheduled_election_inner(
        &mut self,
        schedule_key: &str,
        now_tick: u64,
        ticks_per_game_hour: u64,
    ) -> Result<ElectionOpenOutcome, GovernanceAuthorityError> {
        validate_id(schedule_key)?;
        if let Some(election_id) = self.scheduled_elections.get(schedule_key) {
            return Ok(ElectionOpenOutcome {
                election_id: election_id.clone(),
                created: false,
            });
        }
        if self.eligible_candidate_count() == 0 {
            return Err(GovernanceAuthorityError::NoEligibleCandidate);
        }
        self.open_leader_vacancy_if_needed(now_tick, ticks_per_game_hour)?;
        let election_id = self.election_id("scheduled", schedule_key);
        self.create_election(election_id.clone(), ElectionTrigger::Scheduled, now_tick)?;
        self.scheduled_elections
            .insert(schedule_key.to_owned(), election_id.clone());
        Ok(ElectionOpenOutcome {
            election_id,
            created: true,
        })
    }

    /// A snap key is likewise a durable cause occurrence. Death/expulsion paths
    /// call this after opening the existing officer-institution vacancy.
    fn open_snap_election_inner(
        &mut self,
        trigger: ElectionTrigger,
        snap_key: &str,
        now_tick: u64,
    ) -> Result<ElectionOpenOutcome, GovernanceAuthorityError> {
        if trigger == ElectionTrigger::Scheduled {
            return Err(GovernanceAuthorityError::InvalidElectionTrigger);
        }
        validate_id(snap_key)?;
        if let Some(election_id) = self.snap_elections.get(snap_key) {
            return Ok(ElectionOpenOutcome {
                election_id: election_id.clone(),
                created: false,
            });
        }
        if self.eligible_candidate_count() == 0 {
            return Err(GovernanceAuthorityError::NoEligibleCandidate);
        }
        let election_id = self.election_id("snap", snap_key);
        self.create_election(election_id.clone(), trigger, now_tick)?;
        self.snap_elections
            .insert(snap_key.to_owned(), election_id.clone());
        Ok(ElectionOpenOutcome {
            election_id,
            created: true,
        })
    }

    fn submit_backing_inner(
        &mut self,
        command: BackingCommand,
    ) -> Result<BackingOutcome, GovernanceAuthorityError> {
        command.validate()?;
        let fingerprint = command.fingerprint();
        if let Some(receipt) = self.receipts.get(&command.idempotency_id) {
            if receipt.command_fingerprint != fingerprint {
                return Err(GovernanceAuthorityError::IdempotencyConflict);
            }
            return Ok(BackingOutcome::Replayed(receipt.clone()));
        }
        if command.expected_version != self.version {
            return Err(GovernanceAuthorityError::VersionMismatch {
                expected: command.expected_version,
                actual: self.version,
            });
        }
        if command.actor.global_village && !command.actor.eligibility.eligible_global_player {
            return Err(GovernanceAuthorityError::GlobalVillageBackingDenied);
        }
        let record = self
            .elections
            .get_mut(&command.election_id)
            .ok_or(GovernanceAuthorityError::UnknownElection)?;
        if record.lifecycle != ElectionLifecycle::Open {
            return Err(GovernanceAuthorityError::ElectionClosed);
        }
        if !record
            .election
            .player_backing
            .contains_key(&command.player_id)
            && record.election.player_backing.len() >= MAX_GOVERNANCE_BACKING_PLAYERS
        {
            return Err(GovernanceAuthorityError::CapacityExceeded);
        }
        record.election.set_player_backing(
            command.player_id.clone(),
            command.candidate_cat_id.clone(),
            command.actor.eligibility.into(),
            command.submitted_tick,
        )?;
        if self.receipts.len() >= MAX_GOVERNANCE_RECEIPTS {
            return Err(GovernanceAuthorityError::CapacityExceeded);
        }
        self.bump_version()?;
        let receipt = BackingReceipt {
            idempotency_id: command.idempotency_id.clone(),
            command_fingerprint: fingerprint,
            accepted_version: self.version,
            election_id: command.election_id,
            player_id: command.player_id,
            candidate_cat_id: command.candidate_cat_id,
        };
        self.receipts
            .insert(receipt.idempotency_id.clone(), receipt.clone());
        Ok(BackingOutcome::Applied(receipt))
    }

    /// Resolves each persisted election once. A resolution is replay-safe and
    /// hands the chosen real cat to the existing succession institution.
    fn resolve_election_inner(
        &mut self,
        election_id: &str,
        now_tick: u64,
    ) -> Result<ResolvedElection, GovernanceAuthorityError> {
        let record = self
            .elections
            .get(election_id)
            .ok_or(GovernanceAuthorityError::UnknownElection)?;
        if let Some(result) = &record.result {
            return Ok(result.clone());
        }
        let result = record.election.resolve()?;
        let resolved = self.persisted_result(record, result)?;
        let winner = resolved
            .winner_cat_id
            .clone()
            .ok_or(GovernanceAuthorityError::NoEligibleCandidate)?;
        self.handoff_elected_leader(&winner, now_tick)?;
        let record = self
            .elections
            .get_mut(election_id)
            .expect("record was read from this map");
        record.lifecycle = ElectionLifecycle::Resolved;
        record.result = Some(resolved.clone());
        self.bump_version()?;
        Ok(resolved)
    }

    /// Death opens the existing deterministic succession record first, then
    /// creates the one snap-election occurrence for this real cat ID.
    fn record_death_inner(
        &mut self,
        cat_id: &str,
        now_tick: u64,
        ticks_per_game_hour: u64,
    ) -> Result<Option<ElectionOpenOutcome>, GovernanceAuthorityError> {
        validate_id(cat_id)?;
        let was_leader = self.leader_cat_id.as_deref() == Some(cat_id);
        let planner_id = institution_cat_id(cat_id);
        self.officer_institution
            .officer_died(&planner_id, now_tick)?;
        let resident = self
            .residents
            .get_mut(cat_id)
            .ok_or(GovernanceAuthorityError::UnknownResident)?;
        resident.alive = false;
        resident.resident = false;
        if !was_leader {
            self.bump_version()?;
            return Ok(None);
        }
        self.officer_institution
            .leader_died(&planner_id, now_tick, ticks_per_game_hour)?;
        self.leader_cat_id = None;
        let snap_key = format!("death:{cat_id}");
        let outcome =
            self.open_snap_election_inner(ElectionTrigger::LeaderDeath, &snap_key, now_tick)?;
        Ok(Some(outcome))
    }

    fn preview_expulsion_inner(
        &mut self,
        expulsion_id: &str,
        target_cat_id: &str,
        scope: ExpulsionScope,
    ) -> Result<ExpulsionPreview, GovernanceAuthorityError> {
        validate_id(expulsion_id)?;
        validate_id(target_cat_id)?;
        if let Some(existing) = self.expulsions.get(expulsion_id) {
            return Ok(ExpulsionPreview {
                expulsion_id: expulsion_id.to_owned(),
                plan: existing.plan.clone(),
                intents: existing.intents.values().cloned().collect(),
            });
        }
        if self.expulsions.len() >= MAX_GOVERNANCE_EXPULSIONS {
            return Err(GovernanceAuthorityError::CapacityExceeded);
        }
        let residents = self
            .residents
            .values()
            .filter(|resident| resident.alive && resident.resident)
            .map(|resident| {
                resident.expulsion_resident(self.leader_cat_id.as_deref() == Some(&resident.cat_id))
            })
            .collect::<Vec<_>>();
        let plan = ExpulsionPlan::build(
            ExpulsionRequest {
                expulsion_id: expulsion_id.to_owned(),
                colony_id: self.colony_id.clone(),
                target_cat_id: target_cat_id.to_owned(),
                scope,
            },
            &residents,
        )?;
        let intents = cleanup_intents(expulsion_id, &plan, &self.residents)?;
        let record = ExpulsionLifecycleRecord {
            plan: plan.clone(),
            intents: intents
                .iter()
                .cloned()
                .map(|intent| (intent.intent_id.clone(), intent))
                .collect(),
            acknowledged_intent_ids: BTreeSet::new(),
            departure_reachable: false,
            completed: false,
        };
        self.expulsions.insert(expulsion_id.to_owned(), record);
        self.bump_version()?;
        Ok(ExpulsionPreview {
            expulsion_id: expulsion_id.to_owned(),
            plan,
            intents,
        })
    }

    /// Acknowledgement is the only expulsion mutation here. The referenced
    /// authority remains responsible for performing and proving its cleanup.
    fn acknowledge_cleanup_inner(
        &mut self,
        expulsion_id: &str,
        intent_id: &str,
        departure_reachable: bool,
    ) -> Result<(), GovernanceAuthorityError> {
        let record = self
            .expulsions
            .get_mut(expulsion_id)
            .ok_or(GovernanceAuthorityError::UnknownExpulsion)?;
        if record.completed {
            return Err(GovernanceAuthorityError::ExpulsionCompleted);
        }
        let intent = record
            .intents
            .get(intent_id)
            .ok_or(GovernanceAuthorityError::UnknownCleanupIntent)?;
        if intent.kind == CleanupKind::Departure && !departure_reachable {
            return Err(GovernanceAuthorityError::DepartureUnreachable);
        }
        if intent.kind == CleanupKind::Departure {
            record.departure_reachable = true;
        }
        record.acknowledged_intent_ids.insert(intent_id.to_owned());
        self.bump_version()?;
        Ok(())
    }

    /// Only after every frozen intent has an acknowledgement and a reachable
    /// departure has been proved can governance mark its residents departed.
    fn commit_expulsion_inner(
        &mut self,
        expulsion_id: &str,
        now_tick: u64,
        ticks_per_game_hour: u64,
    ) -> Result<Option<ElectionOpenOutcome>, GovernanceAuthorityError> {
        let (member_ids, opens_snap) = {
            let record = self
                .expulsions
                .get(expulsion_id)
                .ok_or(GovernanceAuthorityError::UnknownExpulsion)?;
            if record.completed {
                return Ok(None);
            }
            if !record.departure_reachable
                || record.acknowledged_intent_ids.len() != record.intents.len()
            {
                return Err(GovernanceAuthorityError::CleanupIncomplete);
            }
            (
                record
                    .plan
                    .members
                    .iter()
                    .map(|member| member.cat_id.clone())
                    .collect::<Vec<_>>(),
                record.plan.opens_snap_election,
            )
        };
        let leader_id = member_ids
            .iter()
            .find(|cat_id| self.leader_cat_id.as_deref() == Some(cat_id.as_str()))
            .cloned();
        for cat_id in &member_ids {
            let resident = self
                .residents
                .get_mut(cat_id)
                .ok_or(GovernanceAuthorityError::UnknownResident)?;
            resident.resident = false;
        }
        if let Some(leader_id) = leader_id {
            self.officer_institution.leader_died(
                &institution_cat_id(&leader_id),
                now_tick,
                ticks_per_game_hour,
            )?;
            self.leader_cat_id = None;
        }
        let record = self
            .expulsions
            .get_mut(expulsion_id)
            .expect("expulsion exists until completion");
        for stage in [
            crate::cat_governance::ExpulsionStage::CargoReturned,
            crate::cat_governance::ExpulsionStage::ReservationsReleased,
            crate::cat_governance::ExpulsionStage::RolesCleared,
            crate::cat_governance::ExpulsionStage::ResidenceVacated,
            crate::cat_governance::ExpulsionStage::ItemsResolved,
            crate::cat_governance::ExpulsionStage::PhysicalDeparture,
            crate::cat_governance::ExpulsionStage::Completed,
        ] {
            record.plan.advance(stage)?;
        }
        record.completed = true;
        let outcome = if opens_snap {
            Some(self.open_snap_election_inner(
                ElectionTrigger::LeaderExpulsion,
                &format!("expulsion:{expulsion_id}"),
                now_tick,
            )?)
        } else {
            self.bump_version()?;
            None
        };
        Ok(outcome)
    }

    /// Uses report-safe believed merit only. The raw civic merit and ballot
    /// facts stay inside this authority and never become appointment input.
    fn appoint_officer_from_reports_inner(
        &mut self,
        role: OfficerRole,
        world_seed: u32,
        leader_effective_level: ExpertiseLevel,
        candidates: Vec<ReportSafeAppointmentCandidate>,
        now_tick: u64,
    ) -> Result<Option<OfficerAppointmentOutcome>, GovernanceAuthorityError> {
        let mut canonical = BTreeMap::new();
        for candidate in candidates {
            validate_id(&candidate.cat_id)?;
            if canonical
                .insert(candidate.cat_id.clone(), candidate)
                .is_some()
            {
                return Err(GovernanceAuthorityError::DuplicateCandidate);
            }
        }
        self.officer_institution.open_office(role, now_tick)?;
        let appointment_candidates = canonical
            .values()
            .map(|candidate| AppointmentCandidate {
                cat_id: institution_cat_id(&candidate.cat_id),
                believed_merit: candidate.believed_merit,
                eligible: self
                    .residents
                    .get(&candidate.cat_id)
                    .is_some_and(GovernanceResidentFact::election_eligible),
            })
            .collect::<Vec<_>>();
        let selection = self.officer_institution.select_for_open_office(
            world_seed,
            role,
            leader_effective_level,
            appointment_candidates,
        )?;
        let Some(selection) = selection else {
            self.bump_version()?;
            return Ok(None);
        };
        let selected_cat_id = real_cat_id_from_institution(&selection, &canonical)?;
        self.officer_institution.appoint_officer(
            role,
            institution_cat_id(&selected_cat_id),
            now_tick,
        )?;
        self.bump_version()?;
        Ok(Some(OfficerAppointmentOutcome {
            selected_cat_id,
            sampled_cat_ids: real_sampled_ids(&selection, &canonical)?,
        }))
    }

    #[must_use]
    pub fn report(&self) -> GovernanceReport {
        GovernanceReport {
            version: self.version,
            colony_id: self.colony_id.clone(),
            leader_cat_id: self.leader_cat_id.clone(),
            elections: self
                .elections
                .iter()
                .map(|(election_id, record)| PublicElectionSummary {
                    election_id: election_id.clone(),
                    trigger: record.election.trigger,
                    lifecycle: record.lifecycle,
                    candidate_cat_ids: record
                        .election
                        .slate
                        .iter()
                        .map(|entry| entry.cat_id.clone())
                        .collect(),
                    winner_cat_id: record
                        .result
                        .as_ref()
                        .and_then(|result| result.winner_cat_id.clone()),
                    total_votes: record
                        .result
                        .as_ref()
                        .map_or_else(BTreeMap::new, |result| result.total_votes.clone()),
                })
                .collect(),
            pending_expulsion_ids: self
                .expulsions
                .iter()
                .filter_map(|(id, record)| (!record.completed).then_some(id.clone()))
                .collect(),
        }
    }

    pub fn validate(&self) -> Result<(), GovernanceAuthorityError> {
        if self.schema_version != GOVERNANCE_AUTHORITY_SCHEMA_VERSION {
            return Err(GovernanceAuthorityError::UnsupportedSchemaVersion);
        }
        validate_id(&self.colony_id)?;
        if self.residents.len() > MAX_GOVERNANCE_RESIDENTS
            || self.elections.len() > MAX_GOVERNANCE_ELECTIONS
            || self.expulsions.len() > MAX_GOVERNANCE_EXPULSIONS
            || self.receipts.len() > MAX_GOVERNANCE_RECEIPTS
        {
            return Err(GovernanceAuthorityError::CapacityExceeded);
        }
        for (cat_id, resident) in &self.residents {
            if cat_id != &resident.cat_id {
                return Err(GovernanceAuthorityError::MalformedState);
            }
            resident.validate()?;
        }
        for (election_id, record) in &self.elections {
            if election_id != &record.election.election_id {
                return Err(GovernanceAuthorityError::MalformedState);
            }
            record.election.validate()?;
            if record.election.colony_id != self.colony_id
                || record.eligible_voter_ids.len() > MAX_GOVERNANCE_RESIDENTS
                || record.election.player_backing.len() > MAX_GOVERNANCE_BACKING_PLAYERS
                || record.election.cat_ballots.len() != record.eligible_voter_ids.len()
                || record
                    .election
                    .cat_ballots
                    .keys()
                    .any(|voter_id| !record.eligible_voter_ids.contains(voter_id))
            {
                return Err(GovernanceAuthorityError::MalformedState);
            }
            validate_election_ids(&record.election)?;
            if (record.lifecycle == ElectionLifecycle::Resolved) != record.result.is_some() {
                return Err(GovernanceAuthorityError::MalformedState);
            }
            if let Some(result) = &record.result {
                validate_resolved_result(&record.election, result)?;
            }
        }
        for election_id in self
            .scheduled_elections
            .values()
            .chain(self.snap_elections.values())
        {
            if !self.elections.contains_key(election_id) {
                return Err(GovernanceAuthorityError::MalformedState);
            }
        }
        for (id, receipt) in &self.receipts {
            if id != &receipt.idempotency_id || !self.elections.contains_key(&receipt.election_id) {
                return Err(GovernanceAuthorityError::MalformedState);
            }
        }
        for (id, record) in &self.expulsions {
            if id != &record.plan.expulsion_id || record.plan.colony_id != self.colony_id {
                return Err(GovernanceAuthorityError::MalformedState);
            }
            validate_expulsion_record(record)?;
        }
        if let Some(leader_cat_id) = &self.leader_cat_id {
            if !self.residents.contains_key(leader_cat_id)
                || self.officer_institution.leader() != Some(&institution_cat_id(leader_cat_id))
            {
                return Err(GovernanceAuthorityError::MalformedState);
            }
        }
        Ok(())
    }

    fn create_election(
        &mut self,
        election_id: String,
        trigger: ElectionTrigger,
        now_tick: u64,
    ) -> Result<(), GovernanceAuthorityError> {
        if self.elections.len() >= MAX_GOVERNANCE_ELECTIONS {
            return Err(GovernanceAuthorityError::CapacityExceeded);
        }
        let candidates = self
            .residents
            .values()
            .filter(|resident| resident.alive)
            .map(GovernanceResidentFact::candidate)
            .collect::<Vec<_>>();
        let slate = select_civic_slate(&candidates)?;
        if slate.is_empty() {
            return Err(GovernanceAuthorityError::NoEligibleCandidate);
        }
        let mut election = CatElectionState::new(
            election_id.clone(),
            self.colony_id.clone(),
            trigger,
            now_tick,
            slate,
        )?;
        let eligible_voter_ids = self
            .residents
            .values()
            .filter(|resident| resident.election_eligible())
            .map(|resident| resident.cat_id.clone())
            .collect::<BTreeSet<_>>();
        let views = self.views_for_slate(&election)?;
        for voter_id in &eligible_voter_ids {
            let voter = self
                .residents
                .get(voter_id)
                .expect("voter IDs were collected from residents");
            let ballot = cast_cat_ballot(
                &election_id,
                &CatVoter {
                    cat_id: voter.cat_id.clone(),
                    life_stage: voter.life_stage,
                    resident: voter.resident && voter.alive,
                    axis: voter.axis,
                },
                &views,
            )?
            .ok_or(GovernanceAuthorityError::MissingEligibleBallot)?;
            election.record_cat_ballot(ballot)?;
        }
        self.elections.insert(
            election_id,
            ElectionRecord {
                election,
                eligible_voter_ids,
                lifecycle: ElectionLifecycle::Open,
                result: None,
            },
        );
        self.bump_version()?;
        Ok(())
    }

    fn views_for_slate(
        &self,
        election: &CatElectionState,
    ) -> Result<Vec<VoterCandidateView>, GovernanceAuthorityError> {
        election
            .slate
            .iter()
            .map(|entry| {
                let resident = self
                    .residents
                    .get(&entry.cat_id)
                    .ok_or(GovernanceAuthorityError::MalformedState)?;
                Ok(VoterCandidateView {
                    candidate_id: entry.cat_id.clone(),
                    signals: resident.ballot_facts.signals(),
                    civic_merit: entry.civic_merit,
                    governance: entry.governance,
                })
            })
            .collect()
    }

    fn persisted_result(
        &self,
        record: &ElectionRecord,
        result: ElectionResult,
    ) -> Result<ResolvedElection, GovernanceAuthorityError> {
        let mut ranked = record
            .election
            .slate
            .iter()
            .map(|entry| {
                Ok((
                    *result
                        .total_votes
                        .get(&entry.cat_id)
                        .ok_or(GovernanceAuthorityError::MalformedState)?,
                    entry.civic_merit,
                    entry.governance,
                    entry.cat_id.clone(),
                ))
            })
            .collect::<Result<Vec<_>, GovernanceAuthorityError>>()?;
        ranked.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| right.1.cmp(&left.1))
                .then_with(|| right.2.cmp(&left.2))
                .then_with(|| left.3.cmp(&right.3))
        });
        Ok(ResolvedElection {
            winner_cat_id: result.winner_cat_id,
            total_votes: result.total_votes,
            tie_order: ranked.into_iter().map(|entry| entry.3).collect(),
        })
    }

    fn handoff_elected_leader(
        &mut self,
        winner_cat_id: &str,
        now_tick: u64,
    ) -> Result<(), GovernanceAuthorityError> {
        let winner = self
            .residents
            .get(winner_cat_id)
            .ok_or(GovernanceAuthorityError::UnknownResident)?;
        if !winner.election_eligible() || winner.barred {
            return Err(GovernanceAuthorityError::NoEligibleCandidate);
        }
        let planner_id = institution_cat_id(winner_cat_id);
        if self.officer_institution.leader().is_none() {
            match self
                .officer_institution
                .appoint_leader(planner_id.clone(), now_tick)
            {
                Ok(_) => {}
                Err(InstitutionError::NoActiveSuccession) => {
                    self.officer_institution
                        .set_founding_leader(planner_id, now_tick)?;
                }
                Err(error) => return Err(error.into()),
            }
        } else if self.officer_institution.leader() != Some(&planner_id) {
            return Err(GovernanceAuthorityError::LeaderVacancyRequired);
        }
        self.leader_cat_id = Some(winner_cat_id.to_owned());
        Ok(())
    }

    fn open_leader_vacancy_if_needed(
        &mut self,
        now_tick: u64,
        ticks_per_game_hour: u64,
    ) -> Result<(), GovernanceAuthorityError> {
        let Some(leader_cat_id) = self.leader_cat_id.clone() else {
            return Ok(());
        };
        self.officer_institution.leader_died(
            &institution_cat_id(&leader_cat_id),
            now_tick,
            ticks_per_game_hour,
        )?;
        self.leader_cat_id = None;
        Ok(())
    }

    fn election_id(&self, kind: &str, occurrence: &str) -> String {
        let hash = stable_hash([self.colony_id.as_str(), kind, occurrence]);
        format!("governance:{kind}:{hash:016x}")
    }

    fn eligible_candidate_count(&self) -> usize {
        self.residents
            .values()
            .filter(|resident| resident.alive && resident.resident && !resident.barred)
            .filter(|resident| resident.life_stage.election_eligible())
            .count()
    }

    fn bump_version(&mut self) -> Result<(), GovernanceAuthorityError> {
        self.version = self
            .version
            .checked_add(1)
            .ok_or(GovernanceAuthorityError::VersionOverflow)?;
        Ok(())
    }

    fn transact<T>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> Result<T, GovernanceAuthorityError>,
    ) -> Result<T, GovernanceAuthorityError> {
        let mut staged = self.clone();
        let result = operation(&mut staged)?;
        staged.validate()?;
        *self = staged;
        Ok(result)
    }
}

impl BackingCommand {
    fn validate(&self) -> Result<(), GovernanceAuthorityError> {
        validate_id(&self.idempotency_id)?;
        validate_id(&self.election_id)?;
        validate_id(&self.player_id)?;
        validate_id(&self.candidate_cat_id)
    }

    fn fingerprint(&self) -> u64 {
        let submitted_tick = self.submitted_tick.to_string();
        stable_hash([
            self.election_id.as_str(),
            self.player_id.as_str(),
            self.candidate_cat_id.as_str(),
            self.idempotency_id.as_str(),
            if self.actor.eligibility.authenticated {
                "1"
            } else {
                "0"
            },
            if self.actor.eligibility.eligible_global_player {
                "1"
            } else {
                "0"
            },
            if self.actor.eligibility.personal_village_owner {
                "1"
            } else {
                "0"
            },
            if self.actor.global_village { "1" } else { "0" },
            submitted_tick.as_str(),
        ])
    }
}

fn cleanup_intents(
    expulsion_id: &str,
    plan: &ExpulsionPlan,
    residents: &BTreeMap<String, GovernanceResidentFact>,
) -> Result<Vec<CleanupIntent>, GovernanceAuthorityError> {
    let mut intents = Vec::new();
    for member in &plan.members {
        let resident = residents
            .get(&member.cat_id)
            .ok_or(GovernanceAuthorityError::UnknownResident)?;
        for kind in CleanupKind::ALL {
            let referenced_ids = match kind {
                CleanupKind::Job => member.clear_job_id.iter().cloned().collect(),
                CleanupKind::Office => member.clear_office_id.iter().cloned().collect(),
                CleanupKind::Election => vec![member.cat_id.clone()],
                CleanupKind::Residence => member.vacate_residence_id.iter().cloned().collect(),
                CleanupKind::Enterprise => member.clear_enterprise_id.iter().cloned().collect(),
                CleanupKind::Cargo => member.return_cargo_ids.clone(),
                CleanupKind::Reservation => member.release_reservation_ids.clone(),
                CleanupKind::Equipment => {
                    let mut ids = member.resolve_owned_item_ids.clone();
                    ids.extend(member.resolve_equipped_item_ids.clone());
                    sorted_unique(ids)
                }
                CleanupKind::Partnership => resident.partnership_id.iter().cloned().collect(),
                CleanupKind::Departure => Vec::new(),
            };
            let intent_id = format!(
                "cleanup:{:016x}",
                stable_hash([expulsion_id, &member.cat_id, cleanup_kind_id(kind)])
            );
            intents.push(CleanupIntent {
                intent_id,
                cat_id: member.cat_id.clone(),
                kind,
                referenced_ids,
            });
        }
    }
    intents.sort_by(|left, right| left.intent_id.cmp(&right.intent_id));
    Ok(intents)
}

fn validate_expulsion_record(
    record: &ExpulsionLifecycleRecord,
) -> Result<(), GovernanceAuthorityError> {
    if record.intents.len() != record.plan.members.len() * CleanupKind::ALL.len()
        || record.intents.len() > MAX_GOVERNANCE_RESIDENTS * CleanupKind::ALL.len()
        || record.intents.iter().any(|(id, intent)| {
            id != &intent.intent_id
                || validate_id(id).is_err()
                || validate_id(&intent.cat_id).is_err()
                || validate_ids(&intent.referenced_ids).is_err()
        })
        || record
            .acknowledged_intent_ids
            .iter()
            .any(|id| !record.intents.contains_key(id))
        || (record.completed
            && (!record.departure_reachable
                || record.acknowledged_intent_ids.len() != record.intents.len()
                || record.plan.stage != crate::cat_governance::ExpulsionStage::Completed))
        || (!record.completed
            && record.plan.stage != crate::cat_governance::ExpulsionStage::Planned)
    {
        return Err(GovernanceAuthorityError::MalformedState);
    }
    for member in &record.plan.members {
        for kind in CleanupKind::ALL {
            let expected_id = format!(
                "cleanup:{:016x}",
                stable_hash([
                    record.plan.expulsion_id.as_str(),
                    member.cat_id.as_str(),
                    cleanup_kind_id(kind),
                ])
            );
            if record
                .intents
                .get(&expected_id)
                .is_none_or(|intent| intent.cat_id != member.cat_id || intent.kind != kind)
            {
                return Err(GovernanceAuthorityError::MalformedState);
            }
        }
    }
    Ok(())
}

fn validate_resolved_result(
    election: &CatElectionState,
    result: &ResolvedElection,
) -> Result<(), GovernanceAuthorityError> {
    if result.tie_order.len() != election.slate.len()
        || result.total_votes.len() != election.slate.len()
        || result
            .tie_order
            .iter()
            .any(|id| !result.total_votes.contains_key(id))
    {
        return Err(GovernanceAuthorityError::MalformedState);
    }
    let mut expected = election
        .slate
        .iter()
        .map(|entry| {
            Ok((
                *result
                    .total_votes
                    .get(&entry.cat_id)
                    .ok_or(GovernanceAuthorityError::MalformedState)?,
                entry.civic_merit,
                entry.governance,
                entry.cat_id.clone(),
            ))
        })
        .collect::<Result<Vec<_>, GovernanceAuthorityError>>()?;
    expected.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| right.2.cmp(&left.2))
            .then_with(|| left.3.cmp(&right.3))
    });
    let expected_tie_order = expected
        .into_iter()
        .map(|entry| entry.3)
        .collect::<Vec<_>>();
    if result.tie_order != expected_tie_order
        || result.winner_cat_id != result.tie_order.first().cloned()
    {
        return Err(GovernanceAuthorityError::MalformedState);
    }
    Ok(())
}

fn validate_election_ids(election: &CatElectionState) -> Result<(), GovernanceAuthorityError> {
    validate_id(&election.election_id)?;
    validate_id(&election.colony_id)?;
    for entry in &election.slate {
        validate_id(&entry.cat_id)?;
    }
    for (voter_id, ballot) in &election.cat_ballots {
        validate_id(voter_id)?;
        validate_id(&ballot.voter_cat_id)?;
        validate_id(&ballot.candidate_cat_id)?;
    }
    for (player_id, backing) in &election.player_backing {
        validate_id(player_id)?;
        validate_id(&backing.player_id)?;
        validate_id(&backing.candidate_cat_id)?;
    }
    Ok(())
}

fn validate_id(value: &str) -> Result<(), GovernanceAuthorityError> {
    if value.trim().is_empty() || value.len() > MAX_GOVERNANCE_ID_BYTES {
        return Err(GovernanceAuthorityError::InvalidId);
    }
    Ok(())
}

fn validate_ids(values: &[String]) -> Result<(), GovernanceAuthorityError> {
    if values.iter().any(|value| validate_id(value).is_err())
        || values.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(GovernanceAuthorityError::InvalidId);
    }
    Ok(())
}

fn institution_cat_id(real_cat_id: &str) -> PlannerId {
    PlannerId::derive("governance_real_cat", [real_cat_id])
}

fn real_cat_id_from_institution(
    selection: &AppointmentSelection,
    candidates: &BTreeMap<String, ReportSafeAppointmentCandidate>,
) -> Result<String, GovernanceAuthorityError> {
    candidates
        .keys()
        .find(|real_id| institution_cat_id(real_id) == selection.selected_cat_id)
        .cloned()
        .ok_or(GovernanceAuthorityError::MalformedState)
}

fn real_sampled_ids(
    selection: &AppointmentSelection,
    candidates: &BTreeMap<String, ReportSafeAppointmentCandidate>,
) -> Result<Vec<String>, GovernanceAuthorityError> {
    let mut ids = selection
        .sampled_cat_ids
        .iter()
        .map(|sampled_id| {
            candidates
                .keys()
                .find(|real_id| institution_cat_id(real_id) == *sampled_id)
                .cloned()
                .ok_or(GovernanceAuthorityError::MalformedState)
        })
        .collect::<Result<Vec<_>, _>>()?;
    ids.sort();
    Ok(ids)
}

fn sorted_unique(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

const fn cleanup_kind_id(kind: CleanupKind) -> &'static str {
    match kind {
        CleanupKind::Job => "job",
        CleanupKind::Office => "office",
        CleanupKind::Election => "election",
        CleanupKind::Residence => "residence",
        CleanupKind::Enterprise => "enterprise",
        CleanupKind::Cargo => "cargo",
        CleanupKind::Reservation => "reservation",
        CleanupKind::Equipment => "equipment",
        CleanupKind::Partnership => "partnership",
        CleanupKind::Departure => "departure",
    }
}

fn stable_hash<const N: usize>(parts: [&str; N]) -> u64 {
    let mut hash = 14_695_981_039_346_656_037_u64;
    for part in parts {
        for byte in part.bytes().chain([b'|']) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(1_099_511_628_211);
        }
    }
    hash
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovernanceAuthorityError {
    Governance(GovernanceError),
    Institution(InstitutionError),
    InvalidId,
    UnsupportedSchemaVersion,
    CapacityExceeded,
    DuplicateResident,
    DuplicateCandidate,
    UnknownResident,
    UnknownElection,
    UnknownExpulsion,
    UnknownCleanupIntent,
    MissingEligibleBallot,
    NoEligibleCandidate,
    ElectionClosed,
    InvalidElectionTrigger,
    VersionMismatch { expected: u64, actual: u64 },
    VersionOverflow,
    IdempotencyConflict,
    GlobalVillageBackingDenied,
    DepartureUnreachable,
    CleanupIncomplete,
    ExpulsionCompleted,
    LeaderVacancyRequired,
    MalformedState,
}

impl From<GovernanceError> for GovernanceAuthorityError {
    fn from(value: GovernanceError) -> Self {
        Self::Governance(value)
    }
}

impl From<InstitutionError> for GovernanceAuthorityError {
    fn from(value: InstitutionError) -> Self {
        Self::Institution(value)
    }
}

impl std::fmt::Display for GovernanceAuthorityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "governance authority error: {self:?}")
    }
}

impl std::error::Error for GovernanceAuthorityError {}
