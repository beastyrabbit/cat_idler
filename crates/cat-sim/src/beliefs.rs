//! Deterministic belief and report boundary specified by
//! `docs/leader-ai-overhaul/planner-and-beliefs.md`.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Serialize};

use crate::planner_core::PlannerId;

pub const BELIEF_STORE_SCHEMA_VERSION: u32 = 1;
pub const MAX_CONFIDENCE_BASIS_POINTS: u16 = 10_000;
pub const CONFIDENCE_DECAY_PER_INTERVAL: u16 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub enum ReportLevel {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
}

impl ReportLevel {
    #[must_use]
    pub const fn stock_error_basis_points(self) -> u16 {
        match self {
            Self::One => 4_000,
            Self::Two => 2_500,
            Self::Three => 1_200,
            Self::Four => 500,
            Self::Five => 200,
        }
    }

    #[must_use]
    pub const fn regeneration_visible(self) -> bool {
        matches!(self, Self::Four | Self::Five)
    }

    #[must_use]
    pub const fn regeneration_error_basis_points(self) -> Option<u16> {
        match self {
            Self::Four => Some(2_500),
            Self::Five => Some(1_000),
            _ => None,
        }
    }
}

impl TryFrom<u8> for ReportLevel {
    type Error = BeliefValidationError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            3 => Ok(Self::Three),
            4 => Ok(Self::Four),
            5 => Ok(Self::Five),
            _ => Err(BeliefValidationError::InvalidReportLevel(value)),
        }
    }
}

impl From<ReportLevel> for u8 {
    fn from(value: ReportLevel) -> Self {
        value as Self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct Confidence(u16);

impl Confidence {
    pub fn new(value: u16) -> Result<Self, BeliefValidationError> {
        if value > MAX_CONFIDENCE_BASIS_POINTS {
            return Err(BeliefValidationError::InvalidConfidence(value));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for Confidence {
    type Error = BeliefValidationError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Confidence> for u16 {
    fn from(value: Confidence) -> Self {
        value.get()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeliefKind {
    Route,
    ActiveThreat,
    Stock,
    Production,
    Consumption,
    Regeneration,
    StaticSite,
}

impl BeliefKind {
    #[must_use]
    pub const fn expiry_game_hours(self) -> Option<u64> {
        match self {
            Self::Route | Self::ActiveThreat => Some(1),
            Self::Stock => Some(6),
            Self::Production | Self::Consumption => Some(12),
            Self::Regeneration => Some(24),
            Self::StaticSite => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Route => "route",
            Self::ActiveThreat => "active_threat",
            Self::Stock => "stock",
            Self::Production => "production",
            Self::Consumption => "consumption",
            Self::Regeneration => "regeneration",
            Self::StaticSite => "static_site",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trend {
    Rising,
    Stable,
    Falling,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimateRange {
    pub estimate: i64,
    pub lower_bound: i64,
    pub upper_bound: i64,
}

impl EstimateRange {
    pub fn new(
        estimate: i64,
        lower_bound: i64,
        upper_bound: i64,
    ) -> Result<Self, BeliefValidationError> {
        let range = Self {
            estimate,
            lower_bound,
            upper_bound,
        };
        range.validate()?;
        Ok(range)
    }

    pub fn around(estimate: i64, error_basis_points: u16) -> Result<Self, BeliefValidationError> {
        if estimate < 0 || error_basis_points > MAX_CONFIDENCE_BASIS_POINTS {
            return Err(BeliefValidationError::InvalidEstimateRange);
        }
        let estimate = i128::from(estimate);
        let scale = i128::from(MAX_CONFIDENCE_BASIS_POINTS);
        let error = i128::from(error_basis_points);
        let lower = estimate.saturating_mul(scale - error) / scale;
        let upper_numerator = estimate.saturating_mul(scale + error);
        let upper = upper_numerator.saturating_add(scale - 1) / scale;
        Self::new(
            estimate as i64,
            lower.clamp(0, i128::from(i64::MAX)) as i64,
            upper.clamp(0, i128::from(i64::MAX)) as i64,
        )
    }

    fn validate(&self) -> Result<(), BeliefValidationError> {
        if self.lower_bound < 0
            || self.lower_bound > self.estimate
            || self.estimate > self.upper_bound
        {
            return Err(BeliefValidationError::InvalidEstimateRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum BeliefValue {
    Estimate(EstimateRange),
    Trend(Trend),
    Category(PlannerId),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeliefKey {
    pub domain_id: PlannerId,
    pub subject_id: PlannerId,
    pub kind: BeliefKind,
}

impl BeliefKey {
    #[must_use]
    pub fn new(domain_id: PlannerId, subject_id: PlannerId, kind: BeliefKind) -> Self {
        Self {
            domain_id,
            subject_id,
            kind,
        }
    }

    #[must_use]
    pub fn stable_id(&self) -> PlannerId {
        PlannerId::derive(
            "belief",
            [
                self.domain_id.as_str(),
                self.subject_id.as_str(),
                self.kind.as_str(),
            ],
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EvidenceId(PlannerId);

impl EvidenceId {
    #[must_use]
    pub fn derive(
        colony_id: &str,
        key: &BeliefKey,
        observed_tick: u64,
        reporter_id: &PlannerId,
        occurrence: u32,
    ) -> Self {
        let tick = observed_tick.to_string();
        let occurrence = occurrence.to_string();
        let belief_id = key.stable_id();
        Self(PlannerId::derive(
            "evidence",
            [
                colony_id,
                belief_id.as_str(),
                tick.as_str(),
                reporter_id.as_str(),
                occurrence.as_str(),
            ],
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReportId(PlannerId);

impl ReportId {
    #[must_use]
    pub fn derive(evidence_id: &EvidenceId, reporter_id: &PlannerId) -> Self {
        Self(PlannerId::derive(
            "report",
            [evidence_id.0.as_str(), reporter_id.as_str()],
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    pub evidence_id: EvidenceId,
    pub key: BeliefKey,
    pub value: BeliefValue,
    pub confidence: Confidence,
    pub observed_tick: u64,
    pub expires_tick: Option<u64>,
    pub reporter_id: PlannerId,
    pub report_level: ReportLevel,
}

impl Observation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        evidence_id: EvidenceId,
        key: BeliefKey,
        value: BeliefValue,
        confidence: Confidence,
        observed_tick: u64,
        ticks_per_game_hour: u64,
        reporter_id: PlannerId,
        report_level: ReportLevel,
    ) -> Result<Self, BeliefValidationError> {
        let expires_tick = expiry_tick(key.kind, observed_tick, ticks_per_game_hour)?;
        validate_value(key.kind, report_level, &value)?;
        Ok(Self {
            evidence_id,
            key,
            value,
            confidence,
            observed_tick,
            expires_tick,
            reporter_id,
            report_level,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficerReport {
    pub report_id: ReportId,
    pub observation: Observation,
    pub authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    AuthorizedOfficerReport,
    DirectObservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionFeedback {
    SourceUnavailable,
    RouteBlocked,
    DestinationFull,
    NoWillingWorker,
    ReservationConflict,
    DependencyBlocked,
    SiteInvalidated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeliefRecord {
    pub key: BeliefKey,
    pub value: BeliefValue,
    pub confidence: Confidence,
    pub observed_tick: u64,
    pub expires_tick: Option<u64>,
    pub source: EvidenceSource,
    pub reporter_id: PlannerId,
    pub report_level: ReportLevel,
    pub primary_evidence_id: EvidenceId,
    pub evidence_ids: BTreeSet<EvidenceId>,
    pub superseded_evidence_ids: BTreeSet<EvidenceId>,
    pub contradiction_version: u64,
    pub invalidated: bool,
}

impl BeliefRecord {
    #[must_use]
    pub fn is_expired(&self, now_tick: u64) -> bool {
        self.expires_tick.is_some_and(|expiry| now_tick >= expiry)
    }

    #[must_use]
    pub fn effective_confidence(&self, now_tick: u64) -> Confidence {
        if self.invalidated {
            return Confidence::zero();
        }
        let Some(expires_tick) = self.expires_tick else {
            return self.confidence;
        };
        let interval = expires_tick.saturating_sub(self.observed_tick);
        if interval == 0 || now_tick <= expires_tick {
            return self.confidence;
        }
        let intervals = now_tick.saturating_sub(expires_tick) / interval;
        let decay = intervals.saturating_mul(u64::from(CONFIDENCE_DECAY_PER_INTERVAL));
        Confidence(
            self.confidence
                .get()
                .saturating_sub(decay.min(u64::from(u16::MAX)) as u16),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeliefStore {
    pub schema_version: u32,
    pub version: u64,
    beliefs: BTreeMap<PlannerId, BeliefRecord>,
}

impl BeliefStore {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: BELIEF_STORE_SCHEMA_VERSION,
            version: 0,
            beliefs: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn get(&self, key: &BeliefKey) -> Option<&BeliefRecord> {
        self.beliefs.get(&key.stable_id())
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&PlannerId, &BeliefRecord)> {
        self.beliefs.iter()
    }

    pub fn apply_observation(&mut self, observation: Observation) -> BeliefUpdate {
        self.apply_candidate(EvidenceCandidate::from_observation(observation))
    }

    pub fn apply_report(
        &mut self,
        report: OfficerReport,
    ) -> Result<BeliefUpdate, BeliefUpdateError> {
        if !report.authorized {
            return Err(BeliefUpdateError::UnauthorizedReport(report.report_id));
        }
        Ok(self.apply_candidate(EvidenceCandidate::from_report(report)))
    }

    pub fn invalidate(
        &mut self,
        key: &BeliefKey,
        evidence_id: EvidenceId,
        reporter_id: PlannerId,
        observed_tick: u64,
    ) -> bool {
        let Some(record) = self.beliefs.get_mut(&key.stable_id()) else {
            return false;
        };
        if observed_tick < record.observed_tick {
            return false;
        }
        if record.evidence_ids.contains(&evidence_id)
            || record.superseded_evidence_ids.contains(&evidence_id)
        {
            return false;
        }
        record
            .superseded_evidence_ids
            .append(&mut record.evidence_ids);
        record.primary_evidence_id = evidence_id.clone();
        record.evidence_ids.insert(evidence_id);
        record.confidence = Confidence::zero();
        record.observed_tick = observed_tick;
        record.expires_tick = None;
        record.source = EvidenceSource::DirectObservation;
        record.reporter_id = reporter_id;
        record.contradiction_version = record.contradiction_version.saturating_add(1);
        record.invalidated = true;
        self.version = self.version.saturating_add(1);
        true
    }

    #[must_use]
    pub fn project(&self, key: &BeliefKey, now_tick: u64) -> Option<BeliefProjection> {
        let record = self.beliefs.get(&key.stable_id())?;
        let value = if record.invalidated {
            ProjectedBeliefValue::Unavailable
        } else {
            match (key.kind, record.report_level, &record.value) {
                (BeliefKind::Stock, _, BeliefValue::Estimate(range)) => {
                    ProjectedBeliefValue::StockRange(range.clone())
                }
                (
                    BeliefKind::Production | BeliefKind::Consumption,
                    ReportLevel::Two,
                    BeliefValue::Trend(trend),
                ) => ProjectedBeliefValue::FlowTrend(*trend),
                (
                    BeliefKind::Production | BeliefKind::Consumption,
                    ReportLevel::Three,
                    BeliefValue::Estimate(range),
                ) => ProjectedBeliefValue::FlowRange(range.clone()),
                (
                    BeliefKind::Production | BeliefKind::Consumption,
                    ReportLevel::Four | ReportLevel::Five,
                    BeliefValue::Estimate(range),
                ) => ProjectedBeliefValue::FlowRate(range.estimate),
                (
                    BeliefKind::Regeneration,
                    ReportLevel::Four | ReportLevel::Five,
                    BeliefValue::Estimate(range),
                ) => ProjectedBeliefValue::RegenerationRange(range.clone()),
                (
                    BeliefKind::Route | BeliefKind::ActiveThreat | BeliefKind::StaticSite,
                    _,
                    BeliefValue::Category(category),
                ) => ProjectedBeliefValue::Category(category.clone()),
                _ => return None,
            }
        };
        Some(BeliefProjection {
            key: key.clone(),
            value,
            confidence: record.effective_confidence(now_tick),
            observed_tick: record.observed_tick,
            expires_tick: record.expires_tick,
            source: record.source,
            reporter_id: record.reporter_id.clone(),
            evidence_ids: record.evidence_ids.clone(),
            report_level: record.report_level,
        })
    }

    fn apply_candidate(&mut self, candidate: EvidenceCandidate) -> BeliefUpdate {
        let key = candidate.observation.key.clone();
        let belief_id = key.stable_id();
        let Some(existing) = self.beliefs.get_mut(&belief_id) else {
            self.beliefs.insert(belief_id, candidate.into_record());
            self.version = self.version.saturating_add(1);
            return BeliefUpdate::Inserted;
        };
        let evidence_id = &candidate.observation.evidence_id;
        if existing.evidence_ids.contains(evidence_id)
            || existing.superseded_evidence_ids.contains(evidence_id)
        {
            return BeliefUpdate::Duplicate;
        }

        let contradicts = existing.invalidated || existing.value != candidate.observation.value;
        let candidate_wins = candidate.precedes(existing);
        self.version = self.version.saturating_add(1);
        if !candidate_wins {
            if contradicts {
                existing
                    .superseded_evidence_ids
                    .insert(candidate.observation.evidence_id);
                existing.contradiction_version = existing.contradiction_version.saturating_add(1);
                return BeliefUpdate::RejectedLowerPrecedence;
            }
            existing
                .evidence_ids
                .insert(candidate.observation.evidence_id);
            return BeliefUpdate::Corroborated;
        }

        if contradicts {
            existing
                .superseded_evidence_ids
                .append(&mut existing.evidence_ids);
            existing.contradiction_version = existing.contradiction_version.saturating_add(1);
        }
        existing.primary_evidence_id = candidate.observation.evidence_id.clone();
        existing
            .evidence_ids
            .insert(candidate.observation.evidence_id);
        existing.value = candidate.observation.value;
        existing.confidence = candidate.observation.confidence;
        existing.observed_tick = candidate.observation.observed_tick;
        existing.expires_tick = candidate.observation.expires_tick;
        existing.source = candidate.source;
        existing.reporter_id = candidate.observation.reporter_id;
        existing.report_level = candidate.observation.report_level;
        existing.invalidated = false;
        if contradicts {
            BeliefUpdate::ReplacedContradiction
        } else {
            BeliefUpdate::ReplacedByPrecedence
        }
    }
}

impl Default for BeliefStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UncheckedBeliefStore {
    schema_version: u32,
    version: u64,
    beliefs: BTreeMap<PlannerId, BeliefRecord>,
}

impl<'de> Deserialize<'de> for BeliefStore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;

        let unchecked = UncheckedBeliefStore::deserialize(deserializer)?;
        if unchecked.schema_version != BELIEF_STORE_SCHEMA_VERSION {
            return Err(D::Error::custom("unsupported belief-store schema version"));
        }
        for (key, record) in &unchecked.beliefs {
            validate_record(key, record).map_err(D::Error::custom)?;
        }
        Ok(Self {
            schema_version: unchecked.schema_version,
            version: unchecked.version,
            beliefs: unchecked.beliefs,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeliefProjection {
    pub key: BeliefKey,
    pub value: ProjectedBeliefValue,
    pub confidence: Confidence,
    pub observed_tick: u64,
    pub expires_tick: Option<u64>,
    pub source: EvidenceSource,
    pub reporter_id: PlannerId,
    pub evidence_ids: BTreeSet<EvidenceId>,
    pub report_level: ReportLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ProjectedBeliefValue {
    StockRange(EstimateRange),
    FlowTrend(Trend),
    FlowRange(EstimateRange),
    FlowRate(i64),
    RegenerationRange(EstimateRange),
    Category(PlannerId),
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeliefUpdate {
    Inserted,
    Corroborated,
    ReplacedByPrecedence,
    ReplacedContradiction,
    RejectedLowerPrecedence,
    Duplicate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeliefUpdateError {
    UnauthorizedReport(ReportId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeliefValidationError {
    InvalidReportLevel(u8),
    InvalidConfidence(u16),
    InvalidEstimateRange,
    InvalidValueForLevel,
    InvalidExpiry,
    TickOverflow,
    MalformedRecord,
}

impl fmt::Display for BeliefValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid belief state: {self:?}")
    }
}

impl std::error::Error for BeliefValidationError {}

struct EvidenceCandidate {
    observation: Observation,
    source: EvidenceSource,
}

impl EvidenceCandidate {
    fn from_observation(observation: Observation) -> Self {
        Self {
            observation,
            source: EvidenceSource::DirectObservation,
        }
    }

    fn from_report(report: OfficerReport) -> Self {
        Self {
            observation: report.observation,
            source: EvidenceSource::AuthorizedOfficerReport,
        }
    }

    fn into_record(self) -> BeliefRecord {
        BeliefRecord {
            key: self.observation.key,
            value: self.observation.value,
            confidence: self.observation.confidence,
            observed_tick: self.observation.observed_tick,
            expires_tick: self.observation.expires_tick,
            source: self.source,
            reporter_id: self.observation.reporter_id,
            report_level: self.observation.report_level,
            primary_evidence_id: self.observation.evidence_id.clone(),
            evidence_ids: BTreeSet::from([self.observation.evidence_id]),
            superseded_evidence_ids: BTreeSet::new(),
            contradiction_version: 0,
            invalidated: false,
        }
    }

    fn precedes(&self, existing: &BeliefRecord) -> bool {
        match self.observation.observed_tick.cmp(&existing.observed_tick) {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => match self.source.cmp(&existing.source) {
                Ordering::Greater => true,
                Ordering::Less => false,
                Ordering::Equal => {
                    (&self.observation.reporter_id, &self.observation.evidence_id)
                        < (&existing.reporter_id, &existing.primary_evidence_id)
                }
            },
        }
    }
}

fn expiry_tick(
    kind: BeliefKind,
    observed_tick: u64,
    ticks_per_game_hour: u64,
) -> Result<Option<u64>, BeliefValidationError> {
    let Some(hours) = kind.expiry_game_hours() else {
        return Ok(None);
    };
    if ticks_per_game_hour == 0 {
        return Err(BeliefValidationError::InvalidExpiry);
    }
    let duration = hours
        .checked_mul(ticks_per_game_hour)
        .ok_or(BeliefValidationError::TickOverflow)?;
    observed_tick
        .checked_add(duration)
        .map(Some)
        .ok_or(BeliefValidationError::TickOverflow)
}

fn validate_value(
    kind: BeliefKind,
    level: ReportLevel,
    value: &BeliefValue,
) -> Result<(), BeliefValidationError> {
    match (kind, level, value) {
        (BeliefKind::Stock, level, BeliefValue::Estimate(range)) => {
            range.validate()?;
            if &EstimateRange::around(range.estimate, level.stock_error_basis_points())? == range {
                Ok(())
            } else {
                Err(BeliefValidationError::InvalidValueForLevel)
            }
        }
        (
            BeliefKind::Production | BeliefKind::Consumption,
            ReportLevel::Two,
            BeliefValue::Trend(_),
        ) => Ok(()),
        (
            BeliefKind::Production | BeliefKind::Consumption,
            ReportLevel::Three | ReportLevel::Four | ReportLevel::Five,
            BeliefValue::Estimate(range),
        ) => range.validate(),
        (
            BeliefKind::Regeneration,
            ReportLevel::Four | ReportLevel::Five,
            BeliefValue::Estimate(range),
        ) => {
            range.validate()?;
            let error = level
                .regeneration_error_basis_points()
                .ok_or(BeliefValidationError::InvalidValueForLevel)?;
            if &EstimateRange::around(range.estimate, error)? == range {
                Ok(())
            } else {
                Err(BeliefValidationError::InvalidValueForLevel)
            }
        }
        (
            BeliefKind::Route | BeliefKind::ActiveThreat | BeliefKind::StaticSite,
            _,
            BeliefValue::Category(_),
        ) => Ok(()),
        _ => Err(BeliefValidationError::InvalidValueForLevel),
    }
}

fn validate_record(key: &PlannerId, record: &BeliefRecord) -> Result<(), BeliefValidationError> {
    if key != &record.key.stable_id()
        || record.evidence_ids.is_empty()
        || !record.evidence_ids.contains(&record.primary_evidence_id)
        || !record
            .evidence_ids
            .is_disjoint(&record.superseded_evidence_ids)
    {
        return Err(BeliefValidationError::MalformedRecord);
    }
    if record.invalidated {
        if record.confidence != Confidence::zero() || record.expires_tick.is_some() {
            return Err(BeliefValidationError::MalformedRecord);
        }
        return Ok(());
    }
    match (record.key.kind.expiry_game_hours(), record.expires_tick) {
        (None, None) => {}
        (Some(_), Some(expiry)) if expiry > record.observed_tick => {}
        _ => return Err(BeliefValidationError::InvalidExpiry),
    }
    validate_value(record.key.kind, record.report_level, &record.value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TICKS_PER_HOUR: u64 = 60;

    fn planner_id(namespace: &str, value: &str) -> PlannerId {
        PlannerId::derive(namespace, [value])
    }

    fn key(kind: BeliefKind) -> BeliefKey {
        BeliefKey::new(
            planner_id("domain", "resources"),
            planner_id("subject", "food"),
            kind,
        )
    }

    fn stock_observation(
        estimate: i64,
        observed_tick: u64,
        reporter: &str,
        occurrence: u32,
    ) -> Observation {
        let key = key(BeliefKind::Stock);
        let reporter_id = planner_id("cat", reporter);
        Observation::new(
            EvidenceId::derive("colony-1", &key, observed_tick, &reporter_id, occurrence),
            key,
            BeliefValue::Estimate(
                EstimateRange::around(estimate, ReportLevel::Three.stock_error_basis_points())
                    .unwrap(),
            ),
            Confidence::new(8_000).unwrap(),
            observed_tick,
            TICKS_PER_HOUR,
            reporter_id,
            ReportLevel::Three,
        )
        .unwrap()
    }

    fn report(observation: Observation, authorized: bool) -> OfficerReport {
        OfficerReport {
            report_id: ReportId::derive(&observation.evidence_id, &observation.reporter_id),
            observation,
            authorized,
        }
    }

    #[test]
    fn report_levels_define_exact_visibility_and_error_bands() {
        assert_eq!(
            [
                ReportLevel::One.stock_error_basis_points(),
                ReportLevel::Two.stock_error_basis_points(),
                ReportLevel::Three.stock_error_basis_points(),
                ReportLevel::Four.stock_error_basis_points(),
                ReportLevel::Five.stock_error_basis_points(),
            ],
            [4_000, 2_500, 1_200, 500, 200]
        );
        assert_eq!(
            ReportLevel::Four.regeneration_error_basis_points(),
            Some(2_500)
        );
        assert_eq!(
            ReportLevel::Five.regeneration_error_basis_points(),
            Some(1_000)
        );
        assert!(!ReportLevel::Three.regeneration_visible());
        assert!(ReportLevel::Four.regeneration_visible());

        let regeneration_key = key(BeliefKind::Regeneration);
        let reporter = planner_id("cat", "forester");
        let low_level = Observation::new(
            EvidenceId::derive("colony-1", &regeneration_key, 0, &reporter, 0),
            regeneration_key,
            BeliefValue::Estimate(EstimateRange::around(10, 2_500).unwrap()),
            Confidence::new(8_000).unwrap(),
            0,
            TICKS_PER_HOUR,
            reporter.clone(),
            ReportLevel::Three,
        );
        assert_eq!(low_level, Err(BeliefValidationError::InvalidValueForLevel));

        let evidence_a = EvidenceId::derive("colony-1", &key(BeliefKind::Stock), 7, &reporter, 0);
        let evidence_b = EvidenceId::derive("colony-1", &key(BeliefKind::Stock), 7, &reporter, 0);
        assert_eq!(evidence_a, evidence_b);
        assert_eq!(
            ReportId::derive(&evidence_a, &reporter),
            ReportId::derive(&evidence_b, &reporter)
        );
    }

    #[test]
    fn expiry_classes_and_decay_are_integer_and_subject_specific() {
        assert_eq!(BeliefKind::Route.expiry_game_hours(), Some(1));
        assert_eq!(BeliefKind::Stock.expiry_game_hours(), Some(6));
        assert_eq!(BeliefKind::Production.expiry_game_hours(), Some(12));
        assert_eq!(BeliefKind::Regeneration.expiry_game_hours(), Some(24));
        assert_eq!(BeliefKind::StaticSite.expiry_game_hours(), None);

        let mut store = BeliefStore::new();
        store.apply_observation(stock_observation(100, 0, "accountant", 0));
        let record = store.get(&key(BeliefKind::Stock)).unwrap();
        assert_eq!(record.expires_tick, Some(6 * TICKS_PER_HOUR));
        assert_eq!(record.effective_confidence(6 * TICKS_PER_HOUR).get(), 8_000);
        assert_eq!(
            record.effective_confidence(12 * TICKS_PER_HOUR - 1).get(),
            8_000
        );
        assert_eq!(
            record.effective_confidence(12 * TICKS_PER_HOUR).get(),
            7_500
        );
        assert_eq!(
            record.effective_confidence(18 * TICKS_PER_HOUR).get(),
            7_000
        );
        assert_eq!(record.effective_confidence(u64::MAX).get(), 0);
    }

    #[test]
    fn flow_projection_changes_at_exact_level_boundaries() {
        let cases = [
            (
                ReportLevel::Two,
                BeliefValue::Trend(Trend::Falling),
                ProjectedBeliefValue::FlowTrend(Trend::Falling),
            ),
            (
                ReportLevel::Three,
                BeliefValue::Estimate(EstimateRange::new(10, 8, 12).unwrap()),
                ProjectedBeliefValue::FlowRange(EstimateRange::new(10, 8, 12).unwrap()),
            ),
            (
                ReportLevel::Four,
                BeliefValue::Estimate(EstimateRange::new(10, 9, 11).unwrap()),
                ProjectedBeliefValue::FlowRate(10),
            ),
        ];
        for (level, value, expected) in cases {
            let flow_key = key(BeliefKind::Consumption);
            let reporter = planner_id("cat", "accountant");
            let observation = Observation::new(
                EvidenceId::derive(
                    "colony-1",
                    &flow_key,
                    0,
                    &reporter,
                    u32::from(u8::from(level)),
                ),
                flow_key.clone(),
                value,
                Confidence::new(7_000).unwrap(),
                0,
                TICKS_PER_HOUR,
                reporter,
                level,
            )
            .unwrap();
            let mut store = BeliefStore::new();
            store.apply_observation(observation);
            assert_eq!(store.project(&flow_key, 0).unwrap().value, expected);
        }
    }

    #[test]
    fn precedence_and_contradiction_are_order_independent() {
        let older_direct = stock_observation(80, 10, "observer", 0);
        let newer_report = report(stock_observation(120, 20, "accountant", 1), true);

        let mut first = BeliefStore::new();
        first.apply_observation(older_direct.clone());
        assert_eq!(
            first.apply_report(newer_report.clone()).unwrap(),
            BeliefUpdate::ReplacedContradiction
        );
        let mut second = BeliefStore::new();
        second.apply_report(newer_report).unwrap();
        assert_eq!(
            second.apply_observation(older_direct),
            BeliefUpdate::RejectedLowerPrecedence
        );
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
        let record = first.get(&key(BeliefKind::Stock)).unwrap();
        assert_eq!(record.source, EvidenceSource::AuthorizedOfficerReport);
        assert_eq!(record.contradiction_version, 1);
        assert_eq!(record.superseded_evidence_ids.len(), 1);

        let direct_same_tick = stock_observation(140, 20, "observer", 2);
        assert_eq!(
            first.apply_observation(direct_same_tick),
            BeliefUpdate::ReplacedContradiction
        );
        assert_eq!(
            first.get(&key(BeliefKind::Stock)).unwrap().source,
            EvidenceSource::DirectObservation
        );
    }

    #[test]
    fn equal_report_ties_use_stable_reporter_then_evidence_id() {
        let alpha = report(stock_observation(90, 10, "alpha", 0), true);
        let beta = report(stock_observation(110, 10, "beta", 0), true);
        let mut first = BeliefStore::new();
        first.apply_report(beta.clone()).unwrap();
        first.apply_report(alpha.clone()).unwrap();
        let mut second = BeliefStore::new();
        second.apply_report(alpha).unwrap();
        second.apply_report(beta).unwrap();
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
        let expected_reporter = planner_id("cat", "alpha").min(planner_id("cat", "beta"));
        assert_eq!(
            first.get(&key(BeliefKind::Stock)).unwrap().reporter_id,
            expected_reporter
        );
    }

    #[test]
    fn direct_invalidation_rejects_stale_time_and_newer_evidence_recovers_with_provenance() {
        let belief_key = key(BeliefKind::Stock);
        let mut store = BeliefStore::new();
        store.apply_observation(stock_observation(100, 10, "accountant", 0));
        let invalidator = planner_id("cat", "inspector");
        let stale_id = EvidenceId::derive("colony-1", &belief_key, 9, &invalidator, 0);
        let before_stale = store.clone();
        assert!(!store.invalidate(&belief_key, stale_id, invalidator.clone(), 9));
        assert_eq!(store, before_stale);

        let invalidation_id = EvidenceId::derive("colony-1", &belief_key, 11, &invalidator, 1);
        assert!(store.invalidate(&belief_key, invalidation_id, invalidator, 11));
        let projection = store.project(&belief_key, 11).unwrap();
        assert_eq!(projection.confidence, Confidence::zero());
        assert_eq!(projection.value, ProjectedBeliefValue::Unavailable);
        assert_eq!(store.get(&belief_key).unwrap().contradiction_version, 1);

        assert_eq!(
            store.apply_observation(stock_observation(120, 12, "accountant", 2)),
            BeliefUpdate::ReplacedContradiction
        );
        let recovered = store.get(&belief_key).unwrap();
        assert!(!recovered.invalidated);
        assert_eq!(recovered.confidence, Confidence::new(8_000).unwrap());
        assert_eq!(recovered.contradiction_version, 2);
        assert_eq!(recovered.superseded_evidence_ids.len(), 2);
        assert!(matches!(
            store.project(&belief_key, 12).unwrap().value,
            ProjectedBeliefValue::StockRange(_)
        ));
    }

    #[test]
    fn hidden_truth_twins_have_identical_report_safe_inputs() {
        let stock_key = key(BeliefKind::Stock);
        let mut store = BeliefStore::new();
        store.apply_observation(stock_observation(100, 0, "accountant", 0));

        let project_with_hidden_truth = |hidden_regeneration: i64| {
            let _executor_only = hidden_regeneration;
            store.project(&stock_key, 0)
        };
        let low_hidden = project_with_hidden_truth(1);
        let high_hidden = project_with_hidden_truth(10_000);
        assert_eq!(low_hidden, high_hidden);
        let json = serde_json::to_string(&low_hidden).unwrap();
        assert!(!json.contains("regeneration_range"));
    }

    #[test]
    fn level_four_regeneration_projects_only_an_estimate_range() {
        let regeneration_key = key(BeliefKind::Regeneration);
        let reporter = planner_id("cat", "forester");
        let observation = Observation::new(
            EvidenceId::derive("colony-1", &regeneration_key, 0, &reporter, 0),
            regeneration_key.clone(),
            BeliefValue::Estimate(EstimateRange::around(20, 2_500).unwrap()),
            Confidence::new(7_000).unwrap(),
            0,
            TICKS_PER_HOUR,
            reporter,
            ReportLevel::Four,
        )
        .unwrap();
        let mut store = BeliefStore::new();
        store.apply_observation(observation);
        assert_eq!(
            store.project(&regeneration_key, 0).unwrap().value,
            ProjectedBeliefValue::RegenerationRange(EstimateRange::around(20, 2_500).unwrap())
        );
    }

    #[test]
    fn persisted_store_round_trips_and_rejects_malformed_state() {
        let mut store = BeliefStore::new();
        store.apply_observation(stock_observation(100, 0, "accountant", 0));
        let json = serde_json::to_string(&store).unwrap();
        let restored: BeliefStore = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, store);
        assert_eq!(serde_json::to_string(&restored).unwrap(), json);

        let mut wrong_version = serde_json::to_value(&store).unwrap();
        wrong_version["schemaVersion"] = serde_json::json!(2);
        assert!(
            serde_json::from_value::<BeliefStore>(wrong_version)
                .unwrap_err()
                .to_string()
                .contains("unsupported belief-store schema version")
        );

        let mut empty_evidence = serde_json::to_value(&store).unwrap();
        let record = empty_evidence["beliefs"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap();
        record["evidenceIds"] = serde_json::json!([]);
        assert!(
            serde_json::from_value::<BeliefStore>(empty_evidence)
                .unwrap_err()
                .to_string()
                .contains("MalformedRecord")
        );

        let regeneration_key = key(BeliefKind::Regeneration);
        let reporter = planner_id("cat", "forester");
        let regeneration = Observation::new(
            EvidenceId::derive("colony-1", &regeneration_key, 0, &reporter, 0),
            regeneration_key,
            BeliefValue::Estimate(EstimateRange::around(20, 2_500).unwrap()),
            Confidence::new(7_000).unwrap(),
            0,
            TICKS_PER_HOUR,
            reporter,
            ReportLevel::Four,
        )
        .unwrap();
        let mut regeneration_store = BeliefStore::new();
        regeneration_store.apply_observation(regeneration);
        let mut forbidden_low_level = serde_json::to_value(regeneration_store).unwrap();
        let record = forbidden_low_level["beliefs"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap();
        record["reportLevel"] = serde_json::json!(3);
        assert!(
            serde_json::from_value::<BeliefStore>(forbidden_low_level)
                .unwrap_err()
                .to_string()
                .contains("InvalidValueForLevel")
        );
    }

    #[test]
    fn unauthorized_reports_never_enter_the_store() {
        let mut store = BeliefStore::new();
        let initial_version = store.version;
        let unauthorized = report(stock_observation(100, 0, "stranger", 0), false);
        assert!(matches!(
            store.apply_report(unauthorized),
            Err(BeliefUpdateError::UnauthorizedReport(_))
        ));
        assert_eq!(store.iter().len(), 0);
        assert_eq!(store.version, initial_version);
    }

    #[test]
    fn execution_feedback_is_a_closed_redacted_category() {
        let json = serde_json::to_string(&ExecutionFeedback::RouteBlocked).unwrap();
        assert_eq!(json, "\"route_blocked\"");
        assert!(!json.contains("amount"));
        assert!(!json.contains("regeneration"));
    }
}
