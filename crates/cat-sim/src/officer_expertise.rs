//! Pure officer expertise, appointment, and succession foundations specified by
//! `docs/leader-ai-overhaul/planner-and-beliefs.md`.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    authority::{AuthorityDomain, officer_owns_domain},
    beliefs::{
        BeliefKey, BeliefKind, BeliefValidationError, BeliefValue, Confidence, EstimateRange,
        EvidenceId, Observation, OfficerReport, ReportId, ReportLevel, Trend,
    },
    officer_requests::{OfficerRequestBook, OfficerRequestId},
    officers::OfficerRole,
    planner_core::{PlannerId, PlannerRngStream, planner_roll},
};

pub const OFFICER_INSTITUTION_SCHEMA_VERSION: u32 = 1;
pub const MAX_DUTY_RECORDS: usize = 4_096;
pub const MAX_APPOINTMENT_CANDIDATES: usize = 4_096;
pub const LEADER_SUCCESSION_GAME_HOURS: u64 = 6;
pub const PERSONAL_LEVEL_DUTY_HOURS: [u64; 5] = [0, 24, 96, 240, 480];

pub const AUTHORITY_DOMAIN_ORDER: [AuthorityDomain; 13] = [
    AuthorityDomain::Survival,
    AuthorityDomain::Evacuation,
    AuthorityDomain::Stewardship,
    AuthorityDomain::Building,
    AuthorityDomain::Accounting,
    AuthorityDomain::Forestry,
    AuthorityDomain::Farming,
    AuthorityDomain::Defense,
    AuthorityDomain::Research,
    AuthorityDomain::Textiles,
    AuthorityDomain::Diplomacy,
    AuthorityDomain::Trade,
    AuthorityDomain::ColonyWide,
];

pub fn domains_for(role: OfficerRole) -> impl Iterator<Item = AuthorityDomain> {
    AUTHORITY_DOMAIN_ORDER
        .into_iter()
        .filter(move |domain| officer_owns_domain(role, *domain))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub enum ExpertiseLevel {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
}

impl TryFrom<u8> for ExpertiseLevel {
    type Error = InstitutionError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            3 => Ok(Self::Three),
            4 => Ok(Self::Four),
            5 => Ok(Self::Five),
            _ => Err(InstitutionError::InvalidExpertiseLevel),
        }
    }
}

impl From<ExpertiseLevel> for u8 {
    fn from(value: ExpertiseLevel) -> Self {
        value as Self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExpertiseBonuses {
    #[serde(default)]
    pub workflow_operational: bool,
    #[serde(default)]
    pub reinforcement_operational: bool,
}

/// Report-safe operational support for an office. A room and its required tool
/// can raise effective expertise, cadence, and report capability only; they
/// never alter the cat's persisted personal duty level or grant executor truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OfficeExpertiseSupport {
    pub office_id: PlannerId,
    pub room_operational: bool,
    pub required_tool_id: Option<PlannerId>,
    pub required_tool_operational: bool,
}

impl OfficeExpertiseSupport {
    #[must_use]
    pub const fn effective_bonuses(&self) -> ExpertiseBonuses {
        ExpertiseBonuses {
            workflow_operational: self.room_operational,
            reinforcement_operational: self.required_tool_id.is_some()
                && self.required_tool_operational,
        }
    }
}

#[must_use]
pub const fn personal_level(completed_duty_minutes: u64) -> ExpertiseLevel {
    personal_level_from_completed_duty_hours(completed_duty_minutes / 60)
}

#[must_use]
pub const fn personal_level_from_completed_duty_hours(completed_duty_hours: u64) -> ExpertiseLevel {
    match completed_duty_hours {
        0..=23 => ExpertiseLevel::One,
        24..=95 => ExpertiseLevel::Two,
        96..=239 => ExpertiseLevel::Three,
        240..=479 => ExpertiseLevel::Four,
        _ => ExpertiseLevel::Five,
    }
}

#[must_use]
pub const fn effective_level(
    personal: ExpertiseLevel,
    bonuses: ExpertiseBonuses,
) -> ExpertiseLevel {
    let value = personal as u8
        + bonuses.workflow_operational as u8
        + bonuses.reinforcement_operational as u8;
    match if value > 5 { 5 } else { value } {
        1 => ExpertiseLevel::One,
        2 => ExpertiseLevel::Two,
        3 => ExpertiseLevel::Three,
        4 => ExpertiseLevel::Four,
        _ => ExpertiseLevel::Five,
    }
}

#[must_use]
pub const fn officer_cadence_minutes(level: ExpertiseLevel) -> u32 {
    match level {
        ExpertiseLevel::One => 6 * 60,
        ExpertiseLevel::Two => 3 * 60,
        ExpertiseLevel::Three => 60,
        ExpertiseLevel::Four => 30,
        ExpertiseLevel::Five => 15,
    }
}

pub fn officer_cadence_ticks(
    level: ExpertiseLevel,
    ticks_per_game_hour: u64,
) -> Result<u64, InstitutionError> {
    if ticks_per_game_hour == 0 {
        return Err(InstitutionError::InvalidClock);
    }
    let cadence_minutes = u64::from(officer_cadence_minutes(level));
    let ticks = cadence_minutes
        .checked_mul(ticks_per_game_hour)
        .ok_or(InstitutionError::TickOverflow)?
        .div_ceil(60);
    Ok(ticks.max(1))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportFlowCapability {
    None,
    Trend,
    CoarseObservedRange,
    NumericObservedRate,
    HighConfidenceNumericObservedRate,
}

/// A report permission surface. It carries precision only and cannot carry executor truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReportCapability {
    pub level: ReportLevel,
    pub stock_error_basis_points: u16,
    pub flow: ReportFlowCapability,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regeneration_estimate_error_basis_points: Option<u16>,
}

#[must_use]
pub const fn report_capability(level: ExpertiseLevel) -> ReportCapability {
    let report_level = match level {
        ExpertiseLevel::One => ReportLevel::One,
        ExpertiseLevel::Two => ReportLevel::Two,
        ExpertiseLevel::Three => ReportLevel::Three,
        ExpertiseLevel::Four => ReportLevel::Four,
        ExpertiseLevel::Five => ReportLevel::Five,
    };
    let flow = match level {
        ExpertiseLevel::One => ReportFlowCapability::None,
        ExpertiseLevel::Two => ReportFlowCapability::Trend,
        ExpertiseLevel::Three => ReportFlowCapability::CoarseObservedRange,
        ExpertiseLevel::Four => ReportFlowCapability::NumericObservedRate,
        ExpertiseLevel::Five => ReportFlowCapability::HighConfidenceNumericObservedRate,
    };
    ReportCapability {
        level: report_level,
        stock_error_basis_points: report_level.stock_error_basis_points(),
        flow,
        regeneration_estimate_error_basis_points: report_level.regeneration_error_basis_points(),
    }
}

#[must_use]
pub const fn appointment_candidate_limit(level: ExpertiseLevel, eligible_count: usize) -> usize {
    let limit = match level {
        ExpertiseLevel::One => 3,
        ExpertiseLevel::Two => 5,
        ExpertiseLevel::Three => 8,
        ExpertiseLevel::Four => 12,
        ExpertiseLevel::Five => usize::MAX,
    };
    if eligible_count < limit {
        eligible_count
    } else {
        limit
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppointmentCandidate {
    pub cat_id: PlannerId,
    /// Suitability from the leader's bounded beliefs, never executor-only truth.
    pub believed_merit: i64,
    pub eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppointmentSelection {
    pub sampled_cat_ids: Vec<PlannerId>,
    pub selected_cat_id: PlannerId,
}

pub fn select_appointment_candidate(
    world_seed: u32,
    colony_id: &str,
    role: OfficerRole,
    vacancy_occurrence: u64,
    leader_effective_level: ExpertiseLevel,
    candidates: Vec<AppointmentCandidate>,
) -> Result<Option<AppointmentSelection>, InstitutionError> {
    if colony_id.is_empty() {
        return Err(InstitutionError::InvalidColonyId);
    }
    if candidates.len() > MAX_APPOINTMENT_CANDIDATES {
        return Err(InstitutionError::CandidateCapacityExceeded);
    }
    let mut canonical = BTreeMap::new();
    for candidate in candidates {
        if !planner_id_valid(&candidate.cat_id) {
            return Err(InstitutionError::InvalidCandidateId);
        }
        if canonical
            .insert(candidate.cat_id.clone(), candidate)
            .is_some()
        {
            return Err(InstitutionError::DuplicateCandidate);
        }
    }
    let eligible = canonical
        .into_values()
        .filter(|candidate| candidate.eligible)
        .collect::<Vec<_>>();
    if eligible.is_empty() {
        return Ok(None);
    }
    let occurrence = vacancy_occurrence.to_string();
    let role_id = role_stable_id(role);
    let mut ranked = eligible
        .into_iter()
        .map(|candidate| {
            let roll = planner_roll(
                world_seed,
                PlannerRngStream::Appointment,
                [
                    colony_id,
                    role_id,
                    occurrence.as_str(),
                    candidate.cat_id.as_str(),
                ],
            );
            (roll.next_seed, candidate)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|(left_roll, left), (right_roll, right)| {
        left_roll
            .cmp(right_roll)
            .then_with(|| left.cat_id.cmp(&right.cat_id))
    });
    ranked.truncate(appointment_candidate_limit(
        leader_effective_level,
        ranked.len(),
    ));
    let selected = ranked
        .iter()
        .map(|(_, candidate)| candidate)
        .min_by(|left, right| {
            right
                .believed_merit
                .cmp(&left.believed_merit)
                .then_with(|| left.cat_id.cmp(&right.cat_id))
        })
        .expect("non-empty eligible sample");
    let selected_cat_id = selected.cat_id.clone();
    let mut sampled_cat_ids = ranked
        .into_iter()
        .map(|(_, candidate)| candidate.cat_id)
        .collect::<Vec<_>>();
    sampled_cat_ids.sort();
    Ok(Some(AppointmentSelection {
        sampled_cat_ids,
        selected_cat_id,
    }))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AppointmentId(PlannerId);

impl AppointmentId {
    fn derive(
        colony_id: &str,
        role: OfficerRole,
        vacancy_occurrence: u64,
        cat_id: &PlannerId,
    ) -> Self {
        let occurrence = vacancy_occurrence.to_string();
        Self(PlannerId::derive(
            "officer_appointment",
            [
                colony_id,
                role_stable_id(role),
                occurrence.as_str(),
                cat_id.as_str(),
            ],
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VacancyId {
    stable_id: PlannerId,
    occurrence: u64,
}

impl VacancyId {
    fn derive(colony_id: &str, role: OfficerRole, occurrence: u64) -> Self {
        let occurrence_text = occurrence.to_string();
        Self {
            stable_id: PlannerId::derive(
                "officer_vacancy",
                [colony_id, role_stable_id(role), occurrence_text.as_str()],
            ),
            occurrence,
        }
    }

    #[must_use]
    pub const fn occurrence(&self) -> u64 {
        self.occurrence
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.stable_id.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OfficerAppointment {
    pub appointment_id: AppointmentId,
    pub cat_id: PlannerId,
    pub appointed_tick: u64,
    pub vacancy_occurrence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OfficeState {
    role: OfficerRole,
    vacancy_occurrence: u64,
    vacant_since_tick: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_runtime_review_tick: Option<u64>,
    appointment: Option<OfficerAppointment>,
}

impl OfficeState {
    fn closed(role: OfficerRole) -> Self {
        Self {
            role,
            vacancy_occurrence: 0,
            vacant_since_tick: None,
            last_runtime_review_tick: None,
            appointment: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DutyRecord {
    cat_id: PlannerId,
    role: OfficerRole,
    completed_duty_minutes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaderDutyRecord {
    cat_id: PlannerId,
    completed_duty_minutes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaderAppointment {
    appointment_id: PlannerId,
    cat_id: PlannerId,
    appointed_tick: u64,
    vacancy_occurrence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaderSuccession {
    vacancy_id: PlannerId,
    vacancy_occurrence: u64,
    pub opened_tick: u64,
    pub deadline_tick: u64,
    ticks_per_game_hour: u64,
    acting_steward_id: Option<PlannerId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LeaderOffice {
    vacancy_occurrence: u64,
    incumbent: Option<LeaderAppointment>,
    succession: Option<LeaderSuccession>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficerInstitutionState {
    colony_id: String,
    offices: BTreeMap<OfficerRole, OfficeState>,
    duty: BTreeMap<(PlannerId, OfficerRole), u64>,
    leader_duty: BTreeMap<PlannerId, u64>,
    leader: LeaderOffice,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InstitutionRef<'a> {
    schema_version: u32,
    colony_id: &'a str,
    offices: Vec<&'a OfficeState>,
    duty: Vec<DutyRecord>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    leader_duty: Vec<LeaderDutyRecord>,
    leader: &'a LeaderOffice,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InstitutionOwned {
    schema_version: u32,
    colony_id: String,
    offices: Vec<OfficeState>,
    #[serde(default)]
    duty: Vec<DutyRecord>,
    #[serde(default)]
    leader_duty: Vec<LeaderDutyRecord>,
    #[serde(default)]
    leader: LeaderOffice,
}

impl Serialize for OfficerInstitutionState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        InstitutionRef {
            schema_version: OFFICER_INSTITUTION_SCHEMA_VERSION,
            colony_id: &self.colony_id,
            offices: self.offices.values().collect(),
            duty: self
                .duty
                .iter()
                .map(|((cat_id, role), completed_duty_minutes)| DutyRecord {
                    cat_id: cat_id.clone(),
                    role: *role,
                    completed_duty_minutes: *completed_duty_minutes,
                })
                .collect(),
            leader_duty: self
                .leader_duty
                .iter()
                .map(|(cat_id, completed_duty_minutes)| LeaderDutyRecord {
                    cat_id: cat_id.clone(),
                    completed_duty_minutes: *completed_duty_minutes,
                })
                .collect(),
            leader: &self.leader,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OfficerInstitutionState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = InstitutionOwned::deserialize(deserializer)?;
        if wire.schema_version != OFFICER_INSTITUTION_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(
                "unsupported officer institution schema version",
            ));
        }
        if wire.offices.len() != OfficerRole::ALL.len() {
            return Err(serde::de::Error::custom(
                "persisted institution must contain exactly seven offices",
            ));
        }
        if wire.duty.len() > MAX_DUTY_RECORDS {
            return Err(serde::de::Error::custom(
                "officer duty record capacity exceeded",
            ));
        }
        if wire.leader_duty.len() > MAX_DUTY_RECORDS {
            return Err(serde::de::Error::custom(
                "leader duty record capacity exceeded",
            ));
        }
        let mut offices = BTreeMap::new();
        for office in wire.offices {
            if offices.insert(office.role, office).is_some() {
                return Err(serde::de::Error::custom("duplicate officer role"));
            }
        }
        let mut duty = BTreeMap::new();
        for record in wire.duty {
            if duty
                .insert((record.cat_id, record.role), record.completed_duty_minutes)
                .is_some()
            {
                return Err(serde::de::Error::custom("duplicate officer duty key"));
            }
        }
        let mut leader_duty = BTreeMap::new();
        for record in wire.leader_duty {
            if leader_duty
                .insert(record.cat_id, record.completed_duty_minutes)
                .is_some()
            {
                return Err(serde::de::Error::custom("duplicate leader duty key"));
            }
        }
        let state = Self {
            colony_id: wire.colony_id,
            offices,
            duty,
            leader_duty,
            leader: wire.leader,
        };
        state.validate().map_err(serde::de::Error::custom)?;
        Ok(state)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficerAppointmentTransition {
    pub role: OfficerRole,
    pub successor_id: PlannerId,
    pub appointed_tick: u64,
}

impl OfficerAppointmentTransition {
    pub fn adopt_requests(&self, requests: &mut OfficerRequestBook) -> Vec<OfficerRequestId> {
        requests.adopt_for_successor(self.role, self.successor_id.clone(), self.appointed_tick)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderTransition {
    pub successor_id: PlannerId,
    pub vacated_office: Option<OfficerRole>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficerRuntimeReview {
    pub role: OfficerRole,
    pub officer_id: PlannerId,
    pub effective_level: ExpertiseLevel,
    pub capability: ReportCapability,
    pub due_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfficerReportFact {
    StockEstimate { estimate: i64 },
    FlowObservation { trend: Trend, rate: i64 },
    RegenerationEstimate { estimate: i64 },
    Category { category: PlannerId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfficerRuntimeError {
    Institution(InstitutionError),
    Belief(BeliefValidationError),
    InvalidColonyId,
    InvalidReportFact,
}

impl From<InstitutionError> for OfficerRuntimeError {
    fn from(value: InstitutionError) -> Self {
        Self::Institution(value)
    }
}

impl From<BeliefValidationError> for OfficerRuntimeError {
    fn from(value: BeliefValidationError) -> Self {
        Self::Belief(value)
    }
}

impl fmt::Display for OfficerRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "officer runtime error: {self:?}")
    }
}

impl std::error::Error for OfficerRuntimeError {}

pub fn emit_officer_report(
    colony_id: &str,
    review: &OfficerRuntimeReview,
    key: BeliefKey,
    fact: OfficerReportFact,
    confidence: Confidence,
    ticks_per_game_hour: u64,
    occurrence: u32,
) -> Result<Option<OfficerReport>, OfficerRuntimeError> {
    if colony_id.is_empty() {
        return Err(OfficerRuntimeError::InvalidColonyId);
    }
    let report_level = review.capability.level;
    let Some(value) = reported_value(key.kind, report_level, fact)? else {
        return Ok(None);
    };
    let evidence_id = EvidenceId::derive(
        colony_id,
        &key,
        review.due_tick,
        &review.officer_id,
        occurrence,
    );
    let observation = Observation::new(
        evidence_id,
        key,
        value,
        confidence,
        review.due_tick,
        ticks_per_game_hour,
        review.officer_id.clone(),
        report_level,
    )?;
    Ok(Some(OfficerReport {
        report_id: ReportId::derive(&observation.evidence_id, &observation.reporter_id),
        observation,
        authorized: true,
    }))
}

fn reported_value(
    kind: BeliefKind,
    report_level: ReportLevel,
    fact: OfficerReportFact,
) -> Result<Option<BeliefValue>, OfficerRuntimeError> {
    let value = match (kind, fact) {
        (BeliefKind::Stock, OfficerReportFact::StockEstimate { estimate }) => {
            Some(BeliefValue::Estimate(EstimateRange::around(
                estimate,
                report_level.stock_error_basis_points(),
            )?))
        }
        (
            BeliefKind::Production | BeliefKind::Consumption,
            OfficerReportFact::FlowObservation { trend, rate },
        ) => match report_level {
            ReportLevel::One => None,
            ReportLevel::Two => Some(BeliefValue::Trend(trend)),
            ReportLevel::Three => Some(BeliefValue::Estimate(EstimateRange::around(
                rate,
                ReportLevel::Three.stock_error_basis_points(),
            )?)),
            ReportLevel::Four | ReportLevel::Five => {
                Some(BeliefValue::Estimate(EstimateRange::new(rate, rate, rate)?))
            }
        },
        (BeliefKind::Regeneration, OfficerReportFact::RegenerationEstimate { estimate }) => {
            let Some(error) = report_level.regeneration_error_basis_points() else {
                return Ok(None);
            };
            Some(BeliefValue::Estimate(EstimateRange::around(
                estimate, error,
            )?))
        }
        (
            BeliefKind::Route | BeliefKind::ActiveThreat | BeliefKind::StaticSite,
            OfficerReportFact::Category { category },
        ) => Some(BeliefValue::Category(category)),
        _ => return Err(OfficerRuntimeError::InvalidReportFact),
    };
    Ok(value)
}

impl OfficerInstitutionState {
    pub fn new(colony_id: impl Into<String>) -> Result<Self, InstitutionError> {
        let colony_id = colony_id.into();
        if colony_id.is_empty() {
            return Err(InstitutionError::InvalidColonyId);
        }
        Ok(Self {
            colony_id,
            offices: OfficerRole::ALL
                .iter()
                .copied()
                .map(|role| (role, OfficeState::closed(role)))
                .collect(),
            duty: BTreeMap::new(),
            leader_duty: BTreeMap::new(),
            leader: LeaderOffice::default(),
        })
    }

    #[must_use]
    pub fn appointment(&self, role: OfficerRole) -> Option<&OfficerAppointment> {
        self.offices.get(&role)?.appointment.as_ref()
    }

    #[must_use]
    pub fn vacancy(&self, role: OfficerRole) -> Option<(VacancyId, Option<u64>)> {
        let office = self.offices.get(&role)?;
        office.vacant_since_tick.map(|opened| {
            (
                VacancyId::derive(&self.colony_id, role, office.vacancy_occurrence),
                Some(opened),
            )
        })
    }

    /// Stable aggregate used by report projections without exposing mutable internals.
    #[must_use]
    pub fn version_hint(&self) -> u64 {
        self.offices.values().fold(0_u64, |version, office| {
            let appointment = office
                .appointment
                .as_ref()
                .map_or(0, |value| value.appointed_tick);
            version
                .rotate_left(5)
                .wrapping_add(office.vacancy_occurrence)
                .wrapping_add(appointment)
        })
    }

    pub fn open_office(
        &mut self,
        role: OfficerRole,
        now_tick: u64,
    ) -> Result<VacancyId, InstitutionError> {
        let office = self
            .offices
            .get_mut(&role)
            .expect("all seven roles exist by construction");
        if office.appointment.is_some() {
            return Err(InstitutionError::OfficeFilled);
        }
        if office.appointment.is_none() && office.vacant_since_tick.is_none() {
            office.vacant_since_tick = Some(now_tick);
        }
        Ok(VacancyId::derive(
            &self.colony_id,
            role,
            office.vacancy_occurrence,
        ))
    }

    pub fn select_for_open_office(
        &self,
        world_seed: u32,
        role: OfficerRole,
        leader_effective_level: ExpertiseLevel,
        candidates: Vec<AppointmentCandidate>,
    ) -> Result<Option<AppointmentSelection>, InstitutionError> {
        let office = self
            .offices
            .get(&role)
            .ok_or(InstitutionError::UnknownRole)?;
        if office.appointment.is_some() {
            return Err(InstitutionError::OfficeFilled);
        }
        if office.vacant_since_tick.is_none() {
            return Err(InstitutionError::OfficeUnavailable);
        }
        select_appointment_candidate(
            world_seed,
            &self.colony_id,
            role,
            office.vacancy_occurrence,
            leader_effective_level,
            candidates,
        )
    }

    pub fn appoint_officer(
        &mut self,
        role: OfficerRole,
        cat_id: PlannerId,
        now_tick: u64,
    ) -> Result<OfficerAppointmentTransition, InstitutionError> {
        if !planner_id_valid(&cat_id) {
            return Err(InstitutionError::InvalidCandidateId);
        }
        let office = self
            .offices
            .get(&role)
            .ok_or(InstitutionError::UnknownRole)?;
        if office.appointment.is_some() {
            return Err(InstitutionError::OfficeFilled);
        }
        if self.leader() == Some(&cat_id)
            || self.offices.values().any(|office| {
                office
                    .appointment
                    .as_ref()
                    .is_some_and(|a| a.cat_id == cat_id)
            })
        {
            return Err(InstitutionError::CandidateAlreadyAppointed);
        }
        let office = self
            .offices
            .get_mut(&role)
            .ok_or(InstitutionError::UnknownRole)?;
        if office.appointment.is_some() {
            return Err(InstitutionError::OfficeFilled);
        }
        if office.vacant_since_tick.is_none() {
            return Err(InstitutionError::OfficeUnavailable);
        }
        office.appointment = Some(OfficerAppointment {
            appointment_id: AppointmentId::derive(
                &self.colony_id,
                role,
                office.vacancy_occurrence,
                &cat_id,
            ),
            cat_id: cat_id.clone(),
            appointed_tick: now_tick,
            vacancy_occurrence: office.vacancy_occurrence,
        });
        office.vacant_since_tick = None;
        office.last_runtime_review_tick = None;
        if role == OfficerRole::Steward
            && let Some(succession) = &mut self.leader.succession
        {
            succession.acting_steward_id = Some(cat_id.clone());
        }
        Ok(OfficerAppointmentTransition {
            role,
            successor_id: cat_id,
            appointed_tick: now_tick,
        })
    }

    pub fn officer_died(
        &mut self,
        cat_id: &PlannerId,
        now_tick: u64,
    ) -> Result<Option<VacancyId>, InstitutionError> {
        let role = self.offices.iter().find_map(|(role, office)| {
            office
                .appointment
                .as_ref()
                .is_some_and(|appointment| &appointment.cat_id == cat_id)
                .then_some(*role)
        });
        let Some(role) = role else {
            return Ok(None);
        };
        let vacancy = self.vacate_office(role, now_tick)?;
        if self.acting_steward() == Some(cat_id)
            && let Some(succession) = &mut self.leader.succession
        {
            succession.acting_steward_id = None;
        }
        Ok(Some(vacancy))
    }

    pub fn vacate_office(
        &mut self,
        role: OfficerRole,
        now_tick: u64,
    ) -> Result<VacancyId, InstitutionError> {
        let office = self
            .offices
            .get_mut(&role)
            .ok_or(InstitutionError::UnknownRole)?;
        if office.appointment.is_none() {
            return Err(InstitutionError::OfficeVacant);
        }
        let next_occurrence = office
            .vacancy_occurrence
            .checked_add(1)
            .ok_or(InstitutionError::OccurrenceOverflow)?;
        office.appointment = None;
        office.vacancy_occurrence = next_occurrence;
        office.vacant_since_tick = Some(now_tick);
        office.last_runtime_review_tick = None;
        Ok(VacancyId::derive(
            &self.colony_id,
            role,
            office.vacancy_occurrence,
        ))
    }

    pub fn record_completed_duty_minutes(
        &mut self,
        cat_id: PlannerId,
        role: OfficerRole,
        completed_minutes: u64,
    ) -> Result<(), InstitutionError> {
        if !planner_id_valid(&cat_id) {
            return Err(InstitutionError::InvalidCandidateId);
        }
        let key = (cat_id, role);
        if !self.duty.contains_key(&key) && self.duty.len() >= MAX_DUTY_RECORDS {
            return Err(InstitutionError::DutyCapacityExceeded);
        }
        let duty = self.duty.entry(key).or_default();
        *duty = duty
            .checked_add(completed_minutes)
            .ok_or(InstitutionError::DutyOverflow)?;
        Ok(())
    }

    pub fn record_completed_duty_hours(
        &mut self,
        cat_id: PlannerId,
        role: OfficerRole,
        completed_hours: u64,
    ) -> Result<(), InstitutionError> {
        let completed_minutes = completed_hours
            .checked_mul(60)
            .ok_or(InstitutionError::DutyOverflow)?;
        self.record_completed_duty_minutes(cat_id, role, completed_minutes)
    }

    pub fn record_completed_leader_duty_minutes(
        &mut self,
        cat_id: PlannerId,
        completed_minutes: u64,
    ) -> Result<(), InstitutionError> {
        if !planner_id_valid(&cat_id) {
            return Err(InstitutionError::InvalidCandidateId);
        }
        if self.leader() != Some(&cat_id) {
            return Err(InstitutionError::LeaderMismatch);
        }
        if !self.leader_duty.contains_key(&cat_id) && self.leader_duty.len() >= MAX_DUTY_RECORDS {
            return Err(InstitutionError::DutyCapacityExceeded);
        }
        let duty = self.leader_duty.entry(cat_id).or_default();
        *duty = duty
            .checked_add(completed_minutes)
            .ok_or(InstitutionError::DutyOverflow)?;
        Ok(())
    }

    #[must_use]
    pub fn leader_completed_duty_minutes(&self, cat_id: &PlannerId) -> u64 {
        self.leader_duty.get(cat_id).copied().unwrap_or(0)
    }

    #[must_use]
    pub fn personal_level(&self, cat_id: &PlannerId, role: OfficerRole) -> ExpertiseLevel {
        personal_level(self.duty.get(&(cat_id.clone(), role)).copied().unwrap_or(0))
    }

    #[must_use]
    pub fn effective_level(
        &self,
        cat_id: &PlannerId,
        role: OfficerRole,
        bonuses: ExpertiseBonuses,
    ) -> ExpertiseLevel {
        effective_level(self.personal_level(cat_id, role), bonuses)
    }

    pub fn set_founding_leader(
        &mut self,
        cat_id: PlannerId,
        appointed_tick: u64,
    ) -> Result<(), InstitutionError> {
        if !planner_id_valid(&cat_id) {
            return Err(InstitutionError::InvalidCandidateId);
        }
        if self.leader.incumbent.is_some() || self.leader.succession.is_some() {
            return Err(InstitutionError::LeaderFilled);
        }
        if self.offices.values().any(|office| {
            office
                .appointment
                .as_ref()
                .is_some_and(|appointment| appointment.cat_id == cat_id)
        }) {
            return Err(InstitutionError::CandidateAlreadyAppointed);
        }
        self.leader.incumbent = Some(LeaderAppointment {
            appointment_id: leader_appointment_id(
                &self.colony_id,
                self.leader.vacancy_occurrence,
                &cat_id,
            ),
            cat_id,
            appointed_tick,
            vacancy_occurrence: self.leader.vacancy_occurrence,
        });
        Ok(())
    }

    pub fn leader_died(
        &mut self,
        cat_id: &PlannerId,
        now_tick: u64,
        ticks_per_game_hour: u64,
    ) -> Result<LeaderSuccession, InstitutionError> {
        if ticks_per_game_hour == 0 {
            return Err(InstitutionError::InvalidClock);
        }
        let incumbent = self
            .leader
            .incumbent
            .as_ref()
            .ok_or(InstitutionError::LeaderVacant)?;
        if &incumbent.cat_id != cat_id {
            return Err(InstitutionError::LeaderMismatch);
        }
        let deadline_tick = now_tick
            .checked_add(
                LEADER_SUCCESSION_GAME_HOURS
                    .checked_mul(ticks_per_game_hour)
                    .ok_or(InstitutionError::TickOverflow)?,
            )
            .ok_or(InstitutionError::TickOverflow)?;
        let next_occurrence = self
            .leader
            .vacancy_occurrence
            .checked_add(1)
            .ok_or(InstitutionError::OccurrenceOverflow)?;
        self.leader.incumbent = None;
        self.leader.vacancy_occurrence = next_occurrence;
        let acting_steward_id = self.officer(OfficerRole::Steward).cloned();
        let succession = LeaderSuccession {
            vacancy_id: leader_vacancy_id(&self.colony_id, self.leader.vacancy_occurrence),
            vacancy_occurrence: self.leader.vacancy_occurrence,
            opened_tick: now_tick,
            deadline_tick,
            ticks_per_game_hour,
            acting_steward_id,
        };
        self.leader.succession = Some(succession.clone());
        Ok(succession)
    }

    #[must_use]
    pub fn leader_succession_due(&self, now_tick: u64) -> bool {
        self.leader
            .succession
            .as_ref()
            .is_some_and(|succession| now_tick >= succession.deadline_tick)
    }

    pub fn appoint_leader(
        &mut self,
        successor_id: PlannerId,
        now_tick: u64,
    ) -> Result<LeaderTransition, InstitutionError> {
        if !planner_id_valid(&successor_id) {
            return Err(InstitutionError::InvalidCandidateId);
        }
        if self.leader.incumbent.is_some() {
            return Err(InstitutionError::LeaderFilled);
        }
        if self.leader.succession.is_none() {
            return Err(InstitutionError::NoActiveSuccession);
        }
        let vacated_office = self.offices.iter().find_map(|(role, office)| {
            office
                .appointment
                .as_ref()
                .is_some_and(|appointment| appointment.cat_id == successor_id)
                .then_some(*role)
        });
        if let Some(role) = vacated_office {
            self.vacate_office(role, now_tick)?;
        }
        self.leader.incumbent = Some(LeaderAppointment {
            appointment_id: leader_appointment_id(
                &self.colony_id,
                self.leader.vacancy_occurrence,
                &successor_id,
            ),
            cat_id: successor_id.clone(),
            appointed_tick: now_tick,
            vacancy_occurrence: self.leader.vacancy_occurrence,
        });
        self.leader.succession = None;
        Ok(LeaderTransition {
            successor_id,
            vacated_office,
        })
    }

    #[must_use]
    pub fn officer(&self, role: OfficerRole) -> Option<&PlannerId> {
        self.offices
            .get(&role)?
            .appointment
            .as_ref()
            .map(|appointment| &appointment.cat_id)
    }

    #[must_use]
    pub fn leader(&self) -> Option<&PlannerId> {
        self.leader
            .incumbent
            .as_ref()
            .map(|appointment| &appointment.cat_id)
    }

    #[must_use]
    pub fn acting_steward(&self) -> Option<&PlannerId> {
        self.leader.succession.as_ref()?.acting_steward_id.as_ref()
    }

    pub fn officer_runtime_due(
        &self,
        role: OfficerRole,
        now_tick: u64,
        ticks_per_game_hour: u64,
        bonuses: ExpertiseBonuses,
    ) -> Result<Option<OfficerRuntimeReview>, InstitutionError> {
        let office = self
            .offices
            .get(&role)
            .ok_or(InstitutionError::UnknownRole)?;
        let Some(appointment) = &office.appointment else {
            return Ok(None);
        };
        let effective_level = self.effective_level(&appointment.cat_id, role, bonuses);
        let cadence = officer_cadence_ticks(effective_level, ticks_per_game_hour)?;
        let anchor = office
            .last_runtime_review_tick
            .unwrap_or(appointment.appointed_tick);
        let due_tick = anchor
            .checked_add(cadence)
            .ok_or(InstitutionError::TickOverflow)?;
        if now_tick < due_tick {
            return Ok(None);
        }
        Ok(Some(OfficerRuntimeReview {
            role,
            officer_id: appointment.cat_id.clone(),
            effective_level,
            capability: report_capability(effective_level),
            due_tick,
        }))
    }

    pub fn complete_officer_runtime_review(
        &mut self,
        role: OfficerRole,
        now_tick: u64,
        ticks_per_game_hour: u64,
        bonuses: ExpertiseBonuses,
    ) -> Result<Option<OfficerRuntimeReview>, InstitutionError> {
        let review = self.officer_runtime_due(role, now_tick, ticks_per_game_hour, bonuses)?;
        if let Some(review) = &review {
            {
                let office = self
                    .offices
                    .get_mut(&role)
                    .ok_or(InstitutionError::UnknownRole)?;
                office.last_runtime_review_tick = Some(review.due_tick);
            }
            self.record_completed_duty_minutes(
                review.officer_id.clone(),
                role,
                u64::from(officer_cadence_minutes(review.effective_level)),
            )?;
        }
        Ok(review)
    }

    fn validate(&self) -> Result<(), InstitutionError> {
        if self.colony_id.is_empty()
            || self.offices.len() != OfficerRole::ALL.len()
            || self.duty.len() > MAX_DUTY_RECORDS
            || self.leader_duty.len() > MAX_DUTY_RECORDS
        {
            return Err(InstitutionError::MalformedPersistence);
        }
        let mut incumbents = BTreeSet::new();
        for role in OfficerRole::ALL {
            let office = self
                .offices
                .get(role)
                .ok_or(InstitutionError::MalformedPersistence)?;
            if office.role != *role
                || (office.appointment.is_some() && office.vacant_since_tick.is_some())
                || (office.appointment.is_none()
                    && office.vacant_since_tick.is_none()
                    && office.vacancy_occurrence != 0)
                || (office.appointment.is_none() && office.last_runtime_review_tick.is_some())
            {
                return Err(InstitutionError::MalformedPersistence);
            }
            if let Some(appointment) = &office.appointment
                && (appointment.vacancy_occurrence != office.vacancy_occurrence
                    || !planner_id_valid(&appointment.cat_id)
                    || appointment.appointment_id
                        != AppointmentId::derive(
                            &self.colony_id,
                            *role,
                            office.vacancy_occurrence,
                            &appointment.cat_id,
                        )
                    || office
                        .last_runtime_review_tick
                        .is_some_and(|tick| tick < appointment.appointed_tick)
                    || !incumbents.insert(appointment.cat_id.clone()))
            {
                return Err(InstitutionError::MalformedPersistence);
            }
        }
        if let Some(leader) = &self.leader.incumbent
            && (self.leader.succession.is_some()
                || !planner_id_valid(&leader.cat_id)
                || leader.vacancy_occurrence != self.leader.vacancy_occurrence
                || leader.appointment_id
                    != leader_appointment_id(
                        &self.colony_id,
                        self.leader.vacancy_occurrence,
                        &leader.cat_id,
                    )
                || incumbents.contains(&leader.cat_id))
        {
            return Err(InstitutionError::MalformedPersistence);
        }
        if let Some(succession) = &self.leader.succession {
            let duration = LEADER_SUCCESSION_GAME_HOURS
                .checked_mul(succession.ticks_per_game_hour)
                .and_then(|duration| succession.opened_tick.checked_add(duration));
            if self.leader.incumbent.is_some()
                || succession.ticks_per_game_hour == 0
                || succession.vacancy_occurrence != self.leader.vacancy_occurrence
                || succession.vacancy_id
                    != leader_vacancy_id(&self.colony_id, self.leader.vacancy_occurrence)
                || duration != Some(succession.deadline_tick)
                || succession.acting_steward_id.as_ref() != self.officer(OfficerRole::Steward)
            {
                return Err(InstitutionError::MalformedPersistence);
            }
        }
        if self.leader.incumbent.is_none()
            && self.leader.succession.is_none()
            && self.leader.vacancy_occurrence != 0
        {
            return Err(InstitutionError::MalformedPersistence);
        }
        if self
            .duty
            .keys()
            .any(|(cat_id, _)| !planner_id_valid(cat_id))
            || self
                .leader_duty
                .keys()
                .any(|cat_id| !planner_id_valid(cat_id))
        {
            return Err(InstitutionError::MalformedPersistence);
        }
        Ok(())
    }
}

fn role_stable_id(role: OfficerRole) -> &'static str {
    match role {
        OfficerRole::Steward => "steward",
        OfficerRole::Accountant => "accountant",
        OfficerRole::Forester => "forester",
        OfficerRole::Farmer => "farmer",
        OfficerRole::Captain => "captain",
        OfficerRole::Loremaster => "loremaster",
        OfficerRole::ClothLeader => "cloth_leader",
    }
}

fn leader_vacancy_id(colony_id: &str, occurrence: u64) -> PlannerId {
    let occurrence = occurrence.to_string();
    PlannerId::derive("leader_vacancy", [colony_id, occurrence.as_str()])
}

fn leader_appointment_id(colony_id: &str, occurrence: u64, cat_id: &PlannerId) -> PlannerId {
    let occurrence = occurrence.to_string();
    PlannerId::derive(
        "leader_appointment",
        [colony_id, occurrence.as_str(), cat_id.as_str()],
    )
}

fn planner_id_valid(id: &PlannerId) -> bool {
    id.as_str().starts_with("planner:v1|")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstitutionError {
    InvalidColonyId,
    InvalidExpertiseLevel,
    CandidateCapacityExceeded,
    DutyCapacityExceeded,
    DuplicateCandidate,
    InvalidCandidateId,
    UnknownRole,
    OfficeUnavailable,
    OfficeFilled,
    OfficeVacant,
    CandidateAlreadyAppointed,
    LeaderFilled,
    LeaderVacant,
    LeaderMismatch,
    NoActiveSuccession,
    InvalidClock,
    TickOverflow,
    OccurrenceOverflow,
    DutyOverflow,
    MalformedPersistence,
}

impl fmt::Display for InstitutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "officer institution error: {self:?}")
    }
}

impl std::error::Error for InstitutionError {}
