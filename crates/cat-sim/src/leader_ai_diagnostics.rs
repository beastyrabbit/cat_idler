//! LAI.69 pure, developer-only Leader-AI diagnostic trace data.
//!
//! This leaf owns no logging sink or player projection. Callers may opt in,
//! provide their own elapsed measurements, and persist the bounded trace.

use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

pub const LEADER_AI_DIAGNOSTICS_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_DIAGNOSTIC_CAPACITY: u16 = 256;
pub const MAX_DIAGNOSTIC_CAPACITY: u16 = 1_024;
pub const MAX_DIAGNOSTIC_MAP_ENTRIES: usize = 32;
pub const MAX_DIAGNOSTIC_TEXT_BYTES: usize = 256;
pub const HEARTBEAT_TICK_INTERVAL: u64 = 120;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticError {
    InvalidCapacity(u16),
    InvalidStableId,
    InvalidText,
    TooManyMapEntries,
    CountOverflow,
    SequenceOverflow,
    NonMonotonicTick { previous: u64, next: u64 },
    InvalidHeartbeatTick(u64),
    DuplicateHeartbeat(u64),
    InvalidEvent,
    InvalidSchemaVersion(u32),
    InvalidVisibility,
    MalformedState,
    InvalidPersistedState,
}

impl fmt::Display for DiagnosticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid Leader-AI diagnostic state: {self:?}")
    }
}

impl std::error::Error for DiagnosticError {}

/// Stable diagnostic identifier with the project-wide ASCII grammar.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiagnosticId(String);

impl DiagnosticId {
    pub fn new(value: impl Into<String>) -> Result<Self, DiagnosticError> {
        let value = value.into();
        if !valid_stable_id(&value) {
            return Err(DiagnosticError::InvalidStableId);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DiagnosticId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl Serialize for DiagnosticId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for DiagnosticId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

/// Bounded diagnostic-only explanation. It is never a player log message.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiagnosticText(String);

impl DiagnosticText {
    pub fn new(value: impl Into<String>) -> Result<Self, DiagnosticError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_DIAGNOSTIC_TEXT_BYTES
            || value.chars().any(char::is_control)
        {
            return Err(DiagnosticError::InvalidText);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for DiagnosticText {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for DiagnosticText {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiagnosticCounts(BTreeMap<DiagnosticId, u64>);

impl DiagnosticCounts {
    pub fn try_from_map(
        values: BTreeMap<DiagnosticId, u64>,
    ) -> Result<Self, DiagnosticError> {
        if values.len() > MAX_DIAGNOSTIC_MAP_ENTRIES {
            return Err(DiagnosticError::TooManyMapEntries);
        }
        Ok(Self(values))
    }

    #[must_use]
    pub fn values(&self) -> &BTreeMap<DiagnosticId, u64> {
        &self.0
    }

    pub fn checked_increment(
        &mut self,
        key: DiagnosticId,
        amount: u64,
    ) -> Result<(), DiagnosticError> {
        if !self.0.contains_key(&key) && self.0.len() >= MAX_DIAGNOSTIC_MAP_ENTRIES {
            return Err(DiagnosticError::TooManyMapEntries);
        }
        let current = self.0.get(&key).copied().unwrap_or(0);
        let next = current
            .checked_add(amount)
            .ok_or(DiagnosticError::CountOverflow)?;
        self.0.insert(key, next);
        Ok(())
    }
}

impl Serialize for DiagnosticCounts {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DiagnosticCounts {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = BTreeMap::<DiagnosticId, u64>::deserialize(deserializer)?;
        Self::try_from_map(values).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiagnosticScores(BTreeMap<DiagnosticId, i64>);

impl DiagnosticScores {
    pub fn try_from_map(
        values: BTreeMap<DiagnosticId, i64>,
    ) -> Result<Self, DiagnosticError> {
        if values.len() > MAX_DIAGNOSTIC_MAP_ENTRIES {
            return Err(DiagnosticError::TooManyMapEntries);
        }
        Ok(Self(values))
    }

    #[must_use]
    pub fn values(&self) -> &BTreeMap<DiagnosticId, i64> {
        &self.0
    }
}

impl Serialize for DiagnosticScores {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DiagnosticScores {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = BTreeMap::<DiagnosticId, i64>::deserialize(deserializer)?;
        Self::try_from_map(values).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiagnosticReasons(BTreeMap<DiagnosticId, DiagnosticText>);

impl DiagnosticReasons {
    pub fn try_from_map(
        values: BTreeMap<DiagnosticId, DiagnosticText>,
    ) -> Result<Self, DiagnosticError> {
        if values.len() > MAX_DIAGNOSTIC_MAP_ENTRIES {
            return Err(DiagnosticError::TooManyMapEntries);
        }
        Ok(Self(values))
    }

    #[must_use]
    pub fn values(&self) -> &BTreeMap<DiagnosticId, DiagnosticText> {
        &self.0
    }
}

impl Serialize for DiagnosticReasons {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DiagnosticReasons {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = BTreeMap::<DiagnosticId, DiagnosticText>::deserialize(deserializer)?;
        Self::try_from_map(values).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticVisibility {
    DeveloperOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseBoundary {
    Enter,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillFamilyTransitionKind {
    Skill,
    Teaching,
    Family,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchLane {
    Leader,
    God,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradePosture {
    PossibleNow,
    BetterTrade,
    RejectedEnemy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceActionKind {
    Load,
    Save,
    Reset,
    Action,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionOutcome {
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DivineDiagnosticKind {
    ClickBatch,
    Contribution,
    RateRejection,
    Inspiration,
    Boost,
    Miracle,
    Rescue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalCause {
    Completed,
    Timeout,
    Stalled,
    SimulationFailure,
    Panic,
}

impl TerminalCause {
    #[must_use]
    pub const fn is_pass(self) -> bool {
        matches!(self, Self::Completed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "snake_case",
    tag = "state",
    content = "cause",
    deny_unknown_fields
)]
pub enum HeartbeatStatus {
    Running,
    Terminal(TerminalCause),
}

impl HeartbeatStatus {
    #[must_use]
    pub const fn is_pass(self) -> bool {
        matches!(self, Self::Terminal(TerminalCause::Completed))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransitionDiagnostic {
    pub domain: DiagnosticId,
    pub from: DiagnosticId,
    pub to: DiagnosticId,
    pub cause: DiagnosticText,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhaseDiagnostic {
    pub phase: DiagnosticId,
    pub boundary: PhaseBoundary,
    /// Supplied by the developer harness; this leaf never reads a clock.
    pub elapsed_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlannerDiagnostic {
    pub candidate_scores: DiagnosticScores,
    pub omissions: DiagnosticReasons,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatcherDiagnostic {
    pub priorities: DiagnosticScores,
    pub matches: DiagnosticCounts,
    pub rejections: DiagnosticReasons,
    pub task_count: u32,
    pub blockers: DiagnosticCounts,
    pub reservation_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillFamilyDiagnostic {
    pub kind: SkillFamilyTransitionKind,
    pub subject_id: DiagnosticId,
    pub related_id: Option<DiagnosticId>,
    pub delta: i64,
    pub detail: DiagnosticText,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ElectionDiagnostic {
    pub candidate_scores: DiagnosticScores,
    pub ballot_counts: DiagnosticCounts,
    pub selected_candidate: Option<DiagnosticId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResearchDiagnostic {
    pub lane: ResearchLane,
    pub selected: Option<DiagnosticId>,
    pub collision: bool,
    pub refunds: DiagnosticCounts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConstructionDiagnostic {
    pub project_id: DiagnosticId,
    pub stage: DiagnosticId,
    pub cargo: DiagnosticCounts,
    pub blockers: DiagnosticCounts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageDiagnostic {
    pub zone_id: DiagnosticId,
    pub used_slots: u32,
    pub capacity_slots: u32,
    pub pressure: DiagnosticCounts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HoleFeedDiagnostic {
    pub operation_id: DiagnosticId,
    pub stage: DiagnosticId,
    pub cargo: DiagnosticCounts,
    pub blockers: DiagnosticReasons,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TradeCaravanDiagnostic {
    pub contract_id: DiagnosticId,
    pub posture: TradePosture,
    pub caravan_stage: DiagnosticId,
    pub cargo: DiagnosticCounts,
    pub rejection: Option<DiagnosticText>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PersistenceActionDiagnostic {
    pub kind: PersistenceActionKind,
    pub action_id: DiagnosticId,
    pub outcome: ActionOutcome,
    pub counts: DiagnosticCounts,
    pub rejection: Option<DiagnosticText>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DivineDiagnostic {
    pub kind: DivineDiagnosticKind,
    pub action_id: DiagnosticId,
    pub counts: DiagnosticCounts,
    pub contribution_numerator: Option<u64>,
    pub contribution_denominator: Option<u64>,
    pub rejection: Option<DiagnosticText>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UiActionDiagnostic {
    pub envelope_id: DiagnosticId,
    pub action_id: DiagnosticId,
    pub outcome: ActionOutcome,
    pub rejection: Option<DiagnosticText>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HeartbeatDiagnostic {
    pub current_phase: DiagnosticId,
    pub task_count: u32,
    pub reservation_count: u32,
    pub last_transition: Option<TransitionDiagnostic>,
    pub status: HeartbeatStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticDomain {
    Phase,
    Planner,
    Matcher,
    SkillFamily,
    Election,
    Research,
    Construction,
    Storage,
    HoleFeed,
    TradeCaravan,
    PersistenceAction,
    Divine,
    UiAction,
    LastTransition,
    Heartbeat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "snake_case",
    tag = "kind",
    content = "payload",
    deny_unknown_fields
)]
pub enum DiagnosticEvent {
    Phase(PhaseDiagnostic),
    Planner(PlannerDiagnostic),
    Matcher(MatcherDiagnostic),
    SkillFamily(SkillFamilyDiagnostic),
    Election(ElectionDiagnostic),
    Research(ResearchDiagnostic),
    Construction(ConstructionDiagnostic),
    Storage(StorageDiagnostic),
    HoleFeed(HoleFeedDiagnostic),
    TradeCaravan(TradeCaravanDiagnostic),
    PersistenceAction(PersistenceActionDiagnostic),
    Divine(DivineDiagnostic),
    UiAction(UiActionDiagnostic),
    LastTransition(TransitionDiagnostic),
    Heartbeat(HeartbeatDiagnostic),
}

impl DiagnosticEvent {
    #[must_use]
    pub const fn domain(&self) -> DiagnosticDomain {
        match self {
            Self::Phase(_) => DiagnosticDomain::Phase,
            Self::Planner(_) => DiagnosticDomain::Planner,
            Self::Matcher(_) => DiagnosticDomain::Matcher,
            Self::SkillFamily(_) => DiagnosticDomain::SkillFamily,
            Self::Election(_) => DiagnosticDomain::Election,
            Self::Research(_) => DiagnosticDomain::Research,
            Self::Construction(_) => DiagnosticDomain::Construction,
            Self::Storage(_) => DiagnosticDomain::Storage,
            Self::HoleFeed(_) => DiagnosticDomain::HoleFeed,
            Self::TradeCaravan(_) => DiagnosticDomain::TradeCaravan,
            Self::PersistenceAction(_) => DiagnosticDomain::PersistenceAction,
            Self::Divine(_) => DiagnosticDomain::Divine,
            Self::UiAction(_) => DiagnosticDomain::UiAction,
            Self::LastTransition(_) => DiagnosticDomain::LastTransition,
            Self::Heartbeat(_) => DiagnosticDomain::Heartbeat,
        }
    }

    fn validate(&self, tick: u64) -> Result<(), DiagnosticError> {
        match self {
            Self::Storage(event)
                if event.capacity_slots == 0 || event.used_slots > event.capacity_slots =>
            {
                Err(DiagnosticError::InvalidEvent)
            }
            Self::TradeCaravan(event) => {
                match (event.posture, event.rejection.is_some()) {
                    (TradePosture::RejectedEnemy, true)
                    | (TradePosture::PossibleNow | TradePosture::BetterTrade, false) => Ok(()),
                    _ => Err(DiagnosticError::InvalidEvent),
                }
            }
            Self::PersistenceAction(event) => {
                validate_outcome_rejection(event.outcome, &event.rejection)
            }
            Self::UiAction(event) => {
                validate_outcome_rejection(event.outcome, &event.rejection)
            }
            Self::Divine(event) => {
                let ratio_valid = matches!(
                    (
                        event.contribution_numerator,
                        event.contribution_denominator
                    ),
                    (None, None) | (Some(_), Some(1..))
                );
                if !ratio_valid
                    || (event.kind == DivineDiagnosticKind::Contribution
                        && event.contribution_numerator.is_none())
                    || (event.kind == DivineDiagnosticKind::RateRejection
                        && event.rejection.is_none())
                {
                    Err(DiagnosticError::InvalidEvent)
                } else {
                    Ok(())
                }
            }
            Self::Heartbeat(_) if tick == 0 || !tick.is_multiple_of(HEARTBEAT_TICK_INTERVAL) => {
                Err(DiagnosticError::InvalidHeartbeatTick(tick))
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticRecord {
    pub sequence: u64,
    pub tick: u64,
    pub event: DiagnosticEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaderAiDiagnosticTrace {
    schema_version: u32,
    visibility: DiagnosticVisibility,
    enabled: bool,
    capacity: u16,
    next_sequence: u64,
    last_tick: Option<u64>,
    last_heartbeat_tick: Option<u64>,
    records: VecDeque<DiagnosticRecord>,
}

impl Default for LeaderAiDiagnosticTrace {
    fn default() -> Self {
        Self {
            schema_version: LEADER_AI_DIAGNOSTICS_SCHEMA_VERSION,
            visibility: DiagnosticVisibility::DeveloperOnly,
            enabled: false,
            capacity: DEFAULT_DIAGNOSTIC_CAPACITY,
            next_sequence: 0,
            last_tick: None,
            last_heartbeat_tick: None,
            records: VecDeque::new(),
        }
    }
}

impl LeaderAiDiagnosticTrace {
    pub fn enabled(capacity: u16) -> Result<Self, DiagnosticError> {
        validate_capacity(capacity)?;
        Ok(Self {
            enabled: true,
            capacity,
            ..Self::default()
        })
    }

    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    #[must_use]
    pub const fn visibility(&self) -> DiagnosticVisibility {
        self.visibility
    }

    #[must_use]
    pub const fn capacity(&self) -> u16 {
        self.capacity
    }

    #[must_use]
    pub fn records(&self) -> &VecDeque<DiagnosticRecord> {
        &self.records
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        *self = Self {
            capacity: self.capacity,
            ..Self::default()
        };
    }

    pub fn record(
        &mut self,
        tick: u64,
        event: DiagnosticEvent,
    ) -> Result<Option<u64>, DiagnosticError> {
        if !self.enabled {
            return Ok(None);
        }
        event.validate(tick)?;
        if let Some(previous) = self.last_tick
            && tick < previous
        {
            return Err(DiagnosticError::NonMonotonicTick {
                previous,
                next: tick,
            });
        }
        if matches!(&event, DiagnosticEvent::Heartbeat(_))
            && self
                .last_heartbeat_tick
                .is_some_and(|previous| previous >= tick)
        {
            return Err(DiagnosticError::DuplicateHeartbeat(tick));
        }
        let sequence = self.next_sequence;
        let next_sequence = sequence
            .checked_add(1)
            .ok_or(DiagnosticError::SequenceOverflow)?;
        self.records.push_back(DiagnosticRecord {
            sequence,
            tick,
            event,
        });
        if self.records.len() > usize::from(self.capacity) {
            self.records.pop_front();
        }
        if matches!(
            self.records.back().map(|record| &record.event),
            Some(DiagnosticEvent::Heartbeat(_))
        ) {
            self.last_heartbeat_tick = Some(tick);
        }
        self.next_sequence = next_sequence;
        self.last_tick = Some(tick);
        Ok(Some(sequence))
    }

    pub fn maybe_record_heartbeat(
        &mut self,
        tick: u64,
        current_phase: DiagnosticId,
        task_count: u32,
        reservation_count: u32,
        last_transition: Option<TransitionDiagnostic>,
        status: HeartbeatStatus,
    ) -> Result<Option<u64>, DiagnosticError> {
        if !self.enabled {
            return Ok(None);
        }
        if let Some(previous) = self.last_tick
            && tick < previous
        {
            return Err(DiagnosticError::NonMonotonicTick {
                previous,
                next: tick,
            });
        }
        if tick == 0 || !tick.is_multiple_of(HEARTBEAT_TICK_INTERVAL) {
            return Ok(None);
        }
        if self.last_heartbeat_tick == Some(tick) {
            return Ok(None);
        }
        self.record(
            tick,
            DiagnosticEvent::Heartbeat(HeartbeatDiagnostic {
                current_phase,
                task_count,
                reservation_count,
                last_transition,
                status,
            }),
        )
    }

    #[must_use]
    pub fn to_canonical_json(&self) -> String {
        serde_json::to_string(self).expect("diagnostic trace serialization is infallible")
    }

    pub fn decode_strict(json: &str) -> Result<Self, DiagnosticError> {
        let trace =
            serde_json::from_str::<Self>(json).map_err(|_| DiagnosticError::MalformedState)?;
        trace.validate_persisted()?;
        Ok(trace)
    }

    fn validate_persisted(&self) -> Result<(), DiagnosticError> {
        if self.schema_version != LEADER_AI_DIAGNOSTICS_SCHEMA_VERSION {
            return Err(DiagnosticError::InvalidSchemaVersion(self.schema_version));
        }
        if self.visibility != DiagnosticVisibility::DeveloperOnly {
            return Err(DiagnosticError::InvalidVisibility);
        }
        validate_capacity(self.capacity)?;
        if !self.enabled {
            if self.next_sequence != 0
                || self.last_tick.is_some()
                || self.last_heartbeat_tick.is_some()
                || !self.records.is_empty()
            {
                return Err(DiagnosticError::InvalidPersistedState);
            }
            return Ok(());
        }
        if self.records.len() > usize::from(self.capacity) {
            return Err(DiagnosticError::InvalidPersistedState);
        }
        if self.records.is_empty() {
            if self.next_sequence != 0
                || self.last_tick.is_some()
                || self.last_heartbeat_tick.is_some()
            {
                return Err(DiagnosticError::InvalidPersistedState);
            }
            return Ok(());
        }
        let retained = u64::try_from(self.records.len())
            .map_err(|_| DiagnosticError::InvalidPersistedState)?;
        let first_sequence = self
            .next_sequence
            .checked_sub(retained)
            .ok_or(DiagnosticError::InvalidPersistedState)?;
        let mut previous_tick = None;
        let mut observed_last_heartbeat = None;
        for (offset, record) in self.records.iter().enumerate() {
            let offset =
                u64::try_from(offset).map_err(|_| DiagnosticError::InvalidPersistedState)?;
            if record.sequence
                != first_sequence
                    .checked_add(offset)
                    .ok_or(DiagnosticError::InvalidPersistedState)?
                || previous_tick.is_some_and(|previous| record.tick < previous)
            {
                return Err(DiagnosticError::InvalidPersistedState);
            }
            record.event.validate(record.tick)?;
            if matches!(&record.event, DiagnosticEvent::Heartbeat(_)) {
                if observed_last_heartbeat == Some(record.tick) {
                    return Err(DiagnosticError::InvalidPersistedState);
                }
                observed_last_heartbeat = Some(record.tick);
            }
            previous_tick = Some(record.tick);
        }
        if self.last_tick != previous_tick {
            return Err(DiagnosticError::InvalidPersistedState);
        }
        if let Some(last_heartbeat_tick) = self.last_heartbeat_tick {
            let first_retained_tick = self.records.front().map_or(0, |record| record.tick);
            if last_heartbeat_tick == 0
                || !last_heartbeat_tick.is_multiple_of(HEARTBEAT_TICK_INTERVAL)
                || last_heartbeat_tick > self.last_tick.unwrap_or(0)
                || observed_last_heartbeat.is_some_and(|observed| observed > last_heartbeat_tick)
                || (last_heartbeat_tick >= first_retained_tick
                    && observed_last_heartbeat != Some(last_heartbeat_tick))
            {
                return Err(DiagnosticError::InvalidPersistedState);
            }
        } else if observed_last_heartbeat.is_some() {
            return Err(DiagnosticError::InvalidPersistedState);
        }
        Ok(())
    }
}

fn validate_capacity(capacity: u16) -> Result<(), DiagnosticError> {
    if capacity == 0 || capacity > MAX_DIAGNOSTIC_CAPACITY {
        Err(DiagnosticError::InvalidCapacity(capacity))
    } else {
        Ok(())
    }
}

fn validate_outcome_rejection(
    outcome: ActionOutcome,
    rejection: &Option<DiagnosticText>,
) -> Result<(), DiagnosticError> {
    match (outcome, rejection.is_some()) {
        (ActionOutcome::Accepted, false) | (ActionOutcome::Rejected, true) => Ok(()),
        _ => Err(DiagnosticError::InvalidEvent),
    }
}

fn valid_stable_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
}
