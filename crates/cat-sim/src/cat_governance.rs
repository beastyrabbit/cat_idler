//! Fixed-point cat elections, God backing, succession triggers, and physical
//! expulsion contracts for LAI.57.
//!
//! This is the replacement leaf for the legacy player-ballot election helper.
//! Runtime integration later connects its winner to [`crate::officer_expertise`]
//! and its cleanup plan to the single reservation/cargo authority.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const CAT_GOVERNANCE_SCHEMA_VERSION: u32 = 1;
pub const CIVIC_CANDIDATE_COUNT: usize = 5;
pub const SCORE_SCALE: u16 = 10_000;
pub const PLAYER_BACKING_VOTES: u32 = 10;
pub const KEYED_BALLOT_VARIATION_MAX: i64 = 250;
pub const CIVIC_MERIT_WEIGHTS_PERCENT: [u16; 7] = [25, 20, 15, 15, 10, 10, 5];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceLifeStage {
    Kitten,
    Young,
    Adult,
    Elder,
}

impl GovernanceLifeStage {
    #[must_use]
    pub const fn election_eligible(self) -> bool {
        matches!(self, Self::Adult | Self::Elder)
    }
}

/// `-10_000` is fully Relational; `+10_000` is fully Analytical.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RelationalAnalyticalAxis(i16);

impl RelationalAnalyticalAxis {
    pub fn new(value: i16) -> Result<Self, GovernanceError> {
        if (-10_000..=10_000).contains(&value) {
            Ok(Self(value))
        } else {
            Err(GovernanceError::AxisOutOfRange)
        }
    }

    #[must_use]
    pub const fn value(self) -> i16 {
        self.0
    }

    #[must_use]
    pub fn relational_weight(self) -> u16 {
        ((10_000_i32 - i32::from(self.0)) / 2) as u16
    }

    #[must_use]
    pub fn analytical_weight(self) -> u16 {
        ((10_000_i32 + i32::from(self.0)) / 2) as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CivicMeritMetrics {
    pub governance: u16,
    pub inherited_leadership: u16,
    pub effective_charisma: u16,
    pub intelligence: u16,
    pub office_breadth: u16,
    pub leadership_service_record: u16,
    pub relevant_traits: u16,
}

impl CivicMeritMetrics {
    pub fn validate(self) -> Result<(), GovernanceError> {
        if [
            self.governance,
            self.inherited_leadership,
            self.effective_charisma,
            self.intelligence,
            self.office_breadth,
            self.leadership_service_record,
            self.relevant_traits,
        ]
        .into_iter()
        .any(|score| score > SCORE_SCALE)
        {
            return Err(GovernanceError::ScoreOutOfRange);
        }
        Ok(())
    }

    pub fn weighted_merit(self) -> Result<u16, GovernanceError> {
        self.validate()?;
        let scores = [
            self.governance,
            self.inherited_leadership,
            self.effective_charisma,
            self.intelligence,
            self.office_breadth,
            self.leadership_service_record,
            self.relevant_traits,
        ];
        let weighted = scores
            .into_iter()
            .zip(CIVIC_MERIT_WEIGHTS_PERCENT)
            .map(|(score, weight)| u32::from(score) * u32::from(weight))
            .sum::<u32>()
            / 100;
        u16::try_from(weighted).map_err(|_| GovernanceError::Overflow)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CivicCandidate {
    pub cat_id: String,
    pub life_stage: GovernanceLifeStage,
    pub resident: bool,
    pub barred: bool,
    pub merit: CivicMeritMetrics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CivicSlateEntry {
    pub cat_id: String,
    pub civic_merit: u16,
    pub governance: u16,
}

pub fn select_civic_slate(
    candidates: &[CivicCandidate],
) -> Result<Vec<CivicSlateEntry>, GovernanceError> {
    let mut ids = BTreeSet::new();
    let mut eligible = Vec::new();
    for candidate in candidates {
        validate_id(&candidate.cat_id)?;
        if !ids.insert(candidate.cat_id.clone()) {
            return Err(GovernanceError::DuplicateId);
        }
        candidate.merit.validate()?;
        if candidate.resident && !candidate.barred && candidate.life_stage.election_eligible() {
            eligible.push(CivicSlateEntry {
                cat_id: candidate.cat_id.clone(),
                civic_merit: candidate.merit.weighted_merit()?,
                governance: candidate.merit.governance,
            });
        }
    }
    eligible.sort_by(|left, right| {
        right
            .civic_merit
            .cmp(&left.civic_merit)
            .then_with(|| right.governance.cmp(&left.governance))
            .then_with(|| left.cat_id.cmp(&right.cat_id))
    });
    eligible.truncate(CIVIC_CANDIDATE_COUNT);
    Ok(eligible)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BallotSignals {
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

impl BallotSignals {
    fn validate(self) -> Result<(), GovernanceError> {
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
        .any(|score| score > SCORE_SCALE)
        {
            return Err(GovernanceError::ScoreOutOfRange);
        }
        Ok(())
    }

    fn relational_score(self) -> i64 {
        [
            self.charisma,
            self.care,
            self.trust,
            self.social_conduct,
            self.personality_compatibility,
        ]
        .into_iter()
        .map(i64::from)
        .sum::<i64>()
            / 5
    }

    fn analytical_score(self) -> i64 {
        [
            self.governance,
            self.intelligence,
            self.office_experience,
            self.skill,
            self.results,
        ]
        .into_iter()
        .map(i64::from)
        .sum::<i64>()
            / 5
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoterCandidateView {
    pub candidate_id: String,
    pub signals: BallotSignals,
    pub civic_merit: u16,
    pub governance: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatVoter {
    pub cat_id: String,
    pub life_stage: GovernanceLifeStage,
    pub resident: bool,
    pub axis: RelationalAnalyticalAxis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatBallot {
    pub voter_cat_id: String,
    pub candidate_cat_id: String,
    pub score: i64,
}

pub fn cast_cat_ballot(
    election_id: &str,
    voter: &CatVoter,
    views: &[VoterCandidateView],
) -> Result<Option<CatBallot>, GovernanceError> {
    validate_id(election_id)?;
    validate_id(&voter.cat_id)?;
    if !voter.resident || !voter.life_stage.election_eligible() {
        return Ok(None);
    }
    let mut candidate_ids = BTreeSet::new();
    let mut ranked = Vec::new();
    for view in views {
        validate_id(&view.candidate_id)?;
        if !candidate_ids.insert(view.candidate_id.clone()) {
            return Err(GovernanceError::DuplicateId);
        }
        if view.civic_merit > SCORE_SCALE || view.governance > SCORE_SCALE {
            return Err(GovernanceError::ScoreOutOfRange);
        }
        view.signals.validate()?;
        let relational = i128::from(view.signals.relational_score())
            * i128::from(voter.axis.relational_weight());
        let analytical = i128::from(view.signals.analytical_score())
            * i128::from(voter.axis.analytical_weight());
        let interpolated = (relational + analytical) / i128::from(SCORE_SCALE);
        let variation = keyed_ballot_variation(election_id, &voter.cat_id, &view.candidate_id);
        let score = i64::try_from(interpolated)
            .map_err(|_| GovernanceError::Overflow)?
            .saturating_add(variation);
        ranked.push((
            score,
            view.civic_merit,
            view.governance,
            view.candidate_id.clone(),
        ));
    }
    ranked.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| right.2.cmp(&left.2))
            .then_with(|| left.3.cmp(&right.3))
    });
    Ok(ranked.first().map(|(score, _, _, candidate_id)| CatBallot {
        voter_cat_id: voter.cat_id.clone(),
        candidate_cat_id: candidate_id.clone(),
        score: *score,
    }))
}

fn keyed_ballot_variation(election_id: &str, voter_id: &str, candidate_id: &str) -> i64 {
    let hash = keyed_hash([election_id, voter_id, candidate_id]);
    let width =
        u64::try_from(KEYED_BALLOT_VARIATION_MAX * 2 + 1).expect("variation width is positive");
    i64::try_from(hash % width).expect("bounded variation fits") - KEYED_BALLOT_VARIATION_MAX
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElectionTrigger {
    Scheduled,
    LeaderDeath,
    LeaderExpulsion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlayerBacking {
    pub player_id: String,
    pub candidate_cat_id: String,
    pub submitted_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackingEligibility {
    pub authenticated: bool,
    pub eligible_global_player: bool,
    pub personal_village_owner: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatElectionState {
    pub schema_version: u32,
    pub election_id: String,
    pub colony_id: String,
    pub trigger: ElectionTrigger,
    pub opened_tick: u64,
    pub slate: Vec<CivicSlateEntry>,
    pub cat_ballots: BTreeMap<String, CatBallot>,
    pub player_backing: BTreeMap<String, PlayerBacking>,
}

impl CatElectionState {
    pub fn new(
        election_id: impl Into<String>,
        colony_id: impl Into<String>,
        trigger: ElectionTrigger,
        opened_tick: u64,
        slate: Vec<CivicSlateEntry>,
    ) -> Result<Self, GovernanceError> {
        let election_id = election_id.into();
        let colony_id = colony_id.into();
        validate_id(&election_id)?;
        validate_id(&colony_id)?;
        validate_slate(&slate)?;
        Ok(Self {
            schema_version: CAT_GOVERNANCE_SCHEMA_VERSION,
            election_id,
            colony_id,
            trigger,
            opened_tick,
            slate,
            cat_ballots: BTreeMap::new(),
            player_backing: BTreeMap::new(),
        })
    }

    pub fn record_cat_ballot(&mut self, ballot: CatBallot) -> Result<(), GovernanceError> {
        if !self
            .slate
            .iter()
            .any(|entry| entry.cat_id == ballot.candidate_cat_id)
        {
            return Err(GovernanceError::CandidateNotOnSlate);
        }
        validate_id(&ballot.voter_cat_id)?;
        self.cat_ballots.insert(ballot.voter_cat_id.clone(), ballot);
        Ok(())
    }

    pub fn set_player_backing(
        &mut self,
        player_id: impl Into<String>,
        candidate_cat_id: impl Into<String>,
        eligibility: BackingEligibility,
        submitted_tick: u64,
    ) -> Result<(), GovernanceError> {
        if !eligibility.authenticated
            || (!eligibility.eligible_global_player && !eligibility.personal_village_owner)
        {
            return Err(GovernanceError::PlayerBackingUnauthorized);
        }
        let player_id = player_id.into();
        let candidate_cat_id = candidate_cat_id.into();
        validate_id(&player_id)?;
        if !self
            .slate
            .iter()
            .any(|entry| entry.cat_id == candidate_cat_id)
        {
            return Err(GovernanceError::CandidateNotOnSlate);
        }
        self.player_backing.insert(
            player_id.clone(),
            PlayerBacking {
                player_id,
                candidate_cat_id,
                submitted_tick,
            },
        );
        Ok(())
    }

    pub fn resolve(&self) -> Result<ElectionResult, GovernanceError> {
        self.validate()?;
        let mut totals = self
            .slate
            .iter()
            .map(|entry| (entry.cat_id.clone(), 0_u32))
            .collect::<BTreeMap<_, _>>();
        for ballot in self.cat_ballots.values() {
            let votes = totals
                .get_mut(&ballot.candidate_cat_id)
                .ok_or(GovernanceError::CandidateNotOnSlate)?;
            *votes = votes.checked_add(1).ok_or(GovernanceError::Overflow)?;
        }
        for backing in self.player_backing.values() {
            let votes = totals
                .get_mut(&backing.candidate_cat_id)
                .ok_or(GovernanceError::CandidateNotOnSlate)?;
            *votes = votes
                .checked_add(PLAYER_BACKING_VOTES)
                .ok_or(GovernanceError::Overflow)?;
        }
        let mut ranked = self
            .slate
            .iter()
            .map(|entry| {
                (
                    totals[&entry.cat_id],
                    entry.civic_merit,
                    entry.governance,
                    entry.cat_id.clone(),
                )
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| right.1.cmp(&left.1))
                .then_with(|| right.2.cmp(&left.2))
                .then_with(|| left.3.cmp(&right.3))
        });
        Ok(ElectionResult {
            winner_cat_id: ranked.first().map(|entry| entry.3.clone()),
            total_votes: totals,
        })
    }

    pub fn validate(&self) -> Result<(), GovernanceError> {
        if self.schema_version != CAT_GOVERNANCE_SCHEMA_VERSION {
            return Err(GovernanceError::MalformedState);
        }
        validate_id(&self.election_id)?;
        validate_id(&self.colony_id)?;
        validate_slate(&self.slate)?;
        if self
            .cat_ballots
            .iter()
            .any(|(voter, ballot)| voter != &ballot.voter_cat_id)
            || self
                .player_backing
                .iter()
                .any(|(player, backing)| player != &backing.player_id)
        {
            return Err(GovernanceError::MalformedState);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElectionResult {
    pub winner_cat_id: Option<String>,
    pub total_votes: BTreeMap<String, u32>,
}

fn validate_slate(slate: &[CivicSlateEntry]) -> Result<(), GovernanceError> {
    if slate.len() > CIVIC_CANDIDATE_COUNT {
        return Err(GovernanceError::MalformedState);
    }
    let mut ids = BTreeSet::new();
    for entry in slate {
        validate_id(&entry.cat_id)?;
        if entry.civic_merit > SCORE_SCALE
            || entry.governance > SCORE_SCALE
            || !ids.insert(entry.cat_id.clone())
        {
            return Err(GovernanceError::MalformedState);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpulsionScope {
    SelectedAdult,
    WholeHousehold,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpulsionResident {
    pub cat_id: String,
    pub household_id: String,
    pub life_stage: GovernanceLifeStage,
    pub guardian_id: Option<String>,
    pub is_leader: bool,
    pub job_id: Option<String>,
    pub office_id: Option<String>,
    pub residence_id: Option<String>,
    pub enterprise_id: Option<String>,
    pub carried_cargo_ids: Vec<String>,
    pub reservation_ids: Vec<String>,
    pub owned_item_ids: Vec<String>,
    pub equipped_item_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpulsionRequest {
    pub expulsion_id: String,
    pub colony_id: String,
    pub target_cat_id: String,
    pub scope: ExpulsionScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExpulsionMemberCleanup {
    pub cat_id: String,
    pub return_cargo_ids: Vec<String>,
    pub release_reservation_ids: Vec<String>,
    pub clear_job_id: Option<String>,
    pub clear_office_id: Option<String>,
    pub vacate_residence_id: Option<String>,
    pub clear_enterprise_id: Option<String>,
    pub resolve_owned_item_ids: Vec<String>,
    pub resolve_equipped_item_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpulsionStage {
    Planned,
    CargoReturned,
    ReservationsReleased,
    RolesCleared,
    ResidenceVacated,
    ItemsResolved,
    PhysicalDeparture,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExpulsionPlan {
    pub schema_version: u32,
    pub expulsion_id: String,
    pub colony_id: String,
    pub scope: ExpulsionScope,
    pub members: Vec<ExpulsionMemberCleanup>,
    pub opens_snap_election: bool,
    pub stage: ExpulsionStage,
}

impl ExpulsionPlan {
    pub fn build(
        request: ExpulsionRequest,
        residents: &[ExpulsionResident],
    ) -> Result<Self, GovernanceError> {
        validate_id(&request.expulsion_id)?;
        validate_id(&request.colony_id)?;
        let target = residents
            .iter()
            .find(|resident| resident.cat_id == request.target_cat_id)
            .ok_or(GovernanceError::UnknownResident)?;
        if !target.life_stage.election_eligible() {
            return Err(GovernanceError::SelectedExpulsionRequiresAdult);
        }
        let selected = match request.scope {
            ExpulsionScope::SelectedAdult => vec![target],
            ExpulsionScope::WholeHousehold => residents
                .iter()
                .filter(|resident| resident.household_id == target.household_id)
                .collect::<Vec<_>>(),
        };
        let selected_ids = selected
            .iter()
            .map(|resident| resident.cat_id.as_str())
            .collect::<BTreeSet<_>>();
        for resident in &selected {
            if !resident.life_stage.election_eligible()
                && resident
                    .guardian_id
                    .as_deref()
                    .is_none_or(|guardian| !selected_ids.contains(guardian))
            {
                return Err(GovernanceError::DependentWithoutGuardian);
            }
        }
        let opens_snap_election = selected.iter().any(|resident| resident.is_leader);
        let mut members = selected
            .into_iter()
            .map(|resident| ExpulsionMemberCleanup {
                cat_id: resident.cat_id.clone(),
                return_cargo_ids: sorted_unique(&resident.carried_cargo_ids),
                release_reservation_ids: sorted_unique(&resident.reservation_ids),
                clear_job_id: resident.job_id.clone(),
                clear_office_id: resident.office_id.clone(),
                vacate_residence_id: resident.residence_id.clone(),
                clear_enterprise_id: resident.enterprise_id.clone(),
                resolve_owned_item_ids: sorted_unique(&resident.owned_item_ids),
                resolve_equipped_item_ids: sorted_unique(&resident.equipped_item_ids),
            })
            .collect::<Vec<_>>();
        members.sort_by(|left, right| left.cat_id.cmp(&right.cat_id));
        Ok(Self {
            schema_version: CAT_GOVERNANCE_SCHEMA_VERSION,
            expulsion_id: request.expulsion_id,
            colony_id: request.colony_id,
            scope: request.scope,
            members,
            opens_snap_election,
            stage: ExpulsionStage::Planned,
        })
    }

    pub fn advance(&mut self, next: ExpulsionStage) -> Result<(), GovernanceError> {
        let expected = match self.stage {
            ExpulsionStage::Planned => ExpulsionStage::CargoReturned,
            ExpulsionStage::CargoReturned => ExpulsionStage::ReservationsReleased,
            ExpulsionStage::ReservationsReleased => ExpulsionStage::RolesCleared,
            ExpulsionStage::RolesCleared => ExpulsionStage::ResidenceVacated,
            ExpulsionStage::ResidenceVacated => ExpulsionStage::ItemsResolved,
            ExpulsionStage::ItemsResolved => ExpulsionStage::PhysicalDeparture,
            ExpulsionStage::PhysicalDeparture => ExpulsionStage::Completed,
            ExpulsionStage::Completed => return Err(GovernanceError::InvalidTransition),
        };
        if next != expected {
            return Err(GovernanceError::InvalidTransition);
        }
        self.stage = next;
        Ok(())
    }

    #[must_use]
    pub const fn may_leave_colony(&self) -> bool {
        matches!(
            self.stage,
            ExpulsionStage::PhysicalDeparture | ExpulsionStage::Completed
        )
    }
}

fn sorted_unique(values: &[String]) -> Vec<String> {
    values
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernanceError {
    BlankId,
    DuplicateId,
    AxisOutOfRange,
    ScoreOutOfRange,
    CandidateNotOnSlate,
    PlayerBackingUnauthorized,
    UnknownResident,
    SelectedExpulsionRequiresAdult,
    DependentWithoutGuardian,
    InvalidTransition,
    MalformedState,
    Overflow,
}

impl std::fmt::Display for GovernanceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "cat governance error: {self:?}")
    }
}

impl std::error::Error for GovernanceError {}

fn validate_id(value: &str) -> Result<(), GovernanceError> {
    if value.trim().is_empty() {
        Err(GovernanceError::BlankId)
    } else {
        Ok(())
    }
}

fn keyed_hash<const N: usize>(parts: [&str; N]) -> u64 {
    let mut hash = 14_695_981_039_346_656_037_u64;
    for part in parts {
        for byte in part.bytes().chain([b'|']) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(1_099_511_628_211);
        }
    }
    hash
}
