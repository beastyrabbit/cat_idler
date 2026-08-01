//! Deterministic planner foundations specified by
//! `docs/leader-ai-overhaul/planner-and-beliefs.md`.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Serialize};

use crate::rng::{SeededRoll, roll_seeded};

pub const PLANNER_SCHEMA_VERSION: u32 = 1;
pub const LIVE_INTENT_CAPACITY: usize = 128;
pub const TERMINAL_INTENT_CAPACITY: usize = 256;
pub const BASIS_POINTS_SCALE: i64 = 10_000;
pub const STARVATION_AGE_PER_GAME_HOUR: i64 = 100;
pub const MAX_STARVATION_AGE: i64 = 2_500;
pub const MAX_FAILED_ATTEMPTS: u8 = 5;
pub const RETRY_DELAYS_GAME_MINUTES: [u64; 5] = [15, 30, 60, 120, 240];

/// The versioned, clock-only portion of persisted planner state.
///
/// Later planner slices compose their own typed state with this leaf rather
/// than introducing independent clocks or schema versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannerCoreState {
    pub schema_version: u32,
    pub planning_clock: u64,
    pub planning_epoch: u64,
}

impl PlannerCoreState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: PLANNER_SCHEMA_VERSION,
            planning_clock: 0,
            planning_epoch: 0,
        }
    }
}

impl Default for PlannerCoreState {
    fn default() -> Self {
        Self::new()
    }
}

/// A stable, losslessly component-encoded identifier for planner-owned data.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlannerId(String);

impl PlannerId {
    /// Build an ID without delimiter ambiguity. Component lengths are UTF-8
    /// byte lengths, so arbitrary persisted IDs remain distinct.
    pub fn derive<I, S>(namespace: &str, components: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut encoded = String::from("planner:v1");
        push_id_component(&mut encoded, namespace);
        for component in components {
            push_id_component(&mut encoded, component.as_ref());
        }
        Self(encoded)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PlannerId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

fn push_id_component(encoded: &mut String, component: &str) {
    use std::fmt::Write as _;

    write!(encoded, "|{}:{component}", component.len()).expect("writing into a String cannot fail");
}

/// Stable identity for an intent occurrence in one planning epoch.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IntentId(PlannerId);

impl IntentId {
    #[must_use]
    pub fn derive(
        colony_id: &str,
        planning_epoch: u64,
        kind: &str,
        target_id: &str,
        occurrence_index: u32,
    ) -> Self {
        let epoch = planning_epoch.to_string();
        let occurrence = occurrence_index.to_string();
        Self(PlannerId::derive(
            "intent",
            [
                colony_id,
                epoch.as_str(),
                kind,
                target_id,
                occurrence.as_str(),
            ],
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for IntentId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Independent keyed planner streams. Fixed offsets keep these streams clear
/// of the movement, life, and raid forks in `rng.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerRngStream {
    Omission,
    PlanningError,
    Appointment,
    Personality,
    Injury,
    Refusal,
}

impl PlannerRngStream {
    #[must_use]
    pub const fn fork_offset(self) -> u32 {
        match self {
            Self::Omission => 4_000_003,
            Self::PlanningError => 5_000_003,
            Self::Appointment => 6_000_003,
            Self::Personality => 7_000_003,
            Self::Injury => 8_000_003,
            Self::Refusal => 9_000_003,
        }
    }
}

#[must_use]
pub const fn planner_fork_seed(world_seed: u32, stream: PlannerRngStream) -> u32 {
    world_seed.wrapping_add(stream.fork_offset())
}

/// Derive a stable seed from semantic inputs rather than collection/call order.
#[must_use]
pub fn keyed_planner_seed<I, S>(world_seed: u32, stream: PlannerRngStream, stable_inputs: I) -> u32
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    const FNV_PRIME: u32 = 16_777_619;

    let mut hash = planner_fork_seed(world_seed, stream) ^ 2_166_136_261;
    for input in stable_inputs {
        let bytes = input.as_ref().as_bytes();
        for byte in (bytes.len() as u64).to_le_bytes().iter().chain(bytes) {
            hash ^= u32::from(*byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    hash.max(1)
}

/// Make one project-LCG draw for a semantic key.
#[must_use]
pub fn planner_roll<I, S>(world_seed: u32, stream: PlannerRngStream, stable_inputs: I) -> SeededRoll
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    roll_seeded(keyed_planner_seed(world_seed, stream, stable_inputs).into())
}

/// Signed fixed-point percentage where 10,000 is 100%.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct BasisPoints(i64);

impl BasisPoints {
    #[must_use]
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

/// Comparable fixed-point planner score.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct PlannerScore(i64);

impl PlannerScore {
    #[must_use]
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentScoreInputs {
    pub urgency: BasisPoints,
    pub strategic_weight: BasisPoints,
    pub personality_weight: BasisPoints,
    pub confidence: BasisPoints,
    pub opportunity_cost: BasisPoints,
    pub churn_penalty: BasisPoints,
    pub starvation_age: BasisPoints,
    pub temporary_player_bias: BasisPoints,
}

/// Apply the planner score formula with an `i128` intermediate. Division is
/// performed only after the complete product, avoiding repeated truncation.
#[must_use]
pub fn score_intent(inputs: IntentScoreInputs) -> PlannerScore {
    let weighted = i128::from(inputs.urgency.get())
        .saturating_mul(i128::from(inputs.strategic_weight.get()))
        .saturating_mul(i128::from(inputs.personality_weight.get()))
        .saturating_mul(i128::from(inputs.confidence.get()))
        / i128::from(BASIS_POINTS_SCALE).pow(3);
    let score = weighted
        .saturating_sub(i128::from(inputs.opportunity_cost.get()))
        .saturating_sub(i128::from(inputs.churn_penalty.get()))
        .saturating_add(i128::from(inputs.starvation_age.get()))
        .saturating_add(i128::from(inputs.temporary_player_bias.get()));

    PlannerScore::new(score.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64)
}

#[must_use]
pub fn starvation_age_basis_points(elapsed_ticks: u64, ticks_per_game_hour: u64) -> BasisPoints {
    if ticks_per_game_hour == 0 {
        return BasisPoints::default();
    }
    let full_hours = elapsed_ticks / ticks_per_game_hour;
    let age = full_hours
        .saturating_mul(STARVATION_AGE_PER_GAME_HOUR as u64)
        .min(MAX_STARVATION_AGE as u64);
    BasisPoints::new(age as i64)
}

/// The exact stable tie key after score comparison.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentTieKey {
    pub kind: String,
    pub creation_tick: u64,
    pub intent_id: IntentId,
    pub target_id: PlannerId,
}

/// Compare intents in execution order: higher score first, then stable tie key.
#[must_use]
pub fn compare_intent_priority(
    left_score: PlannerScore,
    left_key: &IntentTieKey,
    right_score: PlannerScore,
    right_key: &IntentTieKey,
) -> Ordering {
    right_score
        .cmp(&left_score)
        .then_with(|| left_key.cmp(right_key))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentState {
    Proposed,
    Approved,
    Reserving,
    Active,
    Succeeded,
    Blocked,
    RetryWaiting,
    Cancelled,
    Failed,
}

impl IntentState {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Cancelled | Self::Failed)
    }

    #[must_use]
    pub fn can_transition_to(self, next: Self) -> bool {
        if self == next {
            return true;
        }
        if self.is_terminal() {
            return false;
        }
        if matches!(next, Self::Cancelled | Self::Failed) {
            return true;
        }
        matches!(
            (self, next),
            (Self::Proposed, Self::Approved | Self::Blocked)
                | (
                    Self::Approved,
                    Self::Reserving | Self::Blocked | Self::RetryWaiting
                )
                | (
                    Self::Reserving,
                    Self::Active | Self::Blocked | Self::RetryWaiting
                )
                | (
                    Self::Active,
                    Self::Succeeded | Self::Blocked | Self::RetryWaiting
                )
                | (
                    Self::Blocked,
                    Self::Approved | Self::Reserving | Self::RetryWaiting
                )
                | (
                    Self::RetryWaiting,
                    Self::Approved | Self::Reserving | Self::Blocked
                )
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentLifecycle {
    pub state: IntentState,
    pub retry_count: u8,
    pub next_retry_tick: Option<u64>,
    pub terminal_tick: Option<u64>,
}

impl IntentLifecycle {
    #[must_use]
    pub const fn proposed() -> Self {
        Self {
            state: IntentState::Proposed,
            retry_count: 0,
            next_retry_tick: None,
            terminal_tick: None,
        }
    }

    pub fn transition(
        &mut self,
        next: IntentState,
        transition_tick: u64,
    ) -> Result<(), InvalidIntentTransition> {
        if !self.state.can_transition_to(next) {
            return Err(InvalidIntentTransition {
                from: self.state,
                to: next,
            });
        }
        if self.state == next {
            return Ok(());
        }
        self.state = next;
        if next.is_terminal() {
            self.terminal_tick = Some(transition_tick);
            self.next_retry_tick = None;
        }
        Ok(())
    }
}

impl Default for IntentLifecycle {
    fn default() -> Self {
        Self::proposed()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidIntentTransition {
    pub from: IntentState,
    pub to: IntentState,
}

impl fmt::Display for InvalidIntentTransition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid intent transition from {:?} to {:?}",
            self.from, self.to
        )
    }
}

impl std::error::Error for InvalidIntentTransition {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureDisposition {
    RetryAt(u64),
    TerminalFailure,
}

#[must_use]
pub fn retry_delay_game_minutes(failed_attempt: u8) -> Option<u64> {
    let index = failed_attempt.checked_sub(1)? as usize;
    RETRY_DELAYS_GAME_MINUTES.get(index).copied()
}

/// Determine retry timing without mutating lifecycle state. Per the acceptance
/// contract, the fifth failure is terminal; the complete five-entry delay table
/// remains exposed for persisted schedule/version validation.
#[must_use]
pub fn failure_disposition(
    failed_attempt: u8,
    current_tick: u64,
    ticks_per_game_minute: u64,
) -> Option<FailureDisposition> {
    if failed_attempt == 0 {
        return None;
    }
    if failed_attempt >= MAX_FAILED_ATTEMPTS {
        return Some(FailureDisposition::TerminalFailure);
    }
    let delay = retry_delay_game_minutes(failed_attempt)
        .unwrap_or_default()
        .saturating_mul(ticks_per_game_minute);
    Some(FailureDisposition::RetryAt(
        current_tick.saturating_add(delay),
    ))
}

/// Minimal interface needed to enforce live/history bounds without owning the
/// richer intent schema introduced by later cards.
pub trait IntentCollectionRecord {
    fn intent_id(&self) -> &IntentId;
    fn terminal_tick(&self) -> Option<u64>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundedIntentCollections<T> {
    live_intents: BTreeMap<IntentId, T>,
    terminal_intents: Vec<T>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", bound(deserialize = "T: Deserialize<'de>"))]
struct UncheckedIntentCollections<T> {
    live_intents: BTreeMap<IntentId, T>,
    terminal_intents: Vec<T>,
}

impl<'de, T> Deserialize<'de> for BoundedIntentCollections<T>
where
    T: Deserialize<'de> + IntentCollectionRecord,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;

        let unchecked = UncheckedIntentCollections::<T>::deserialize(deserializer)?;
        if unchecked.live_intents.len() > LIVE_INTENT_CAPACITY {
            return Err(D::Error::custom(
                "live intent collection exceeds 128 entries",
            ));
        }
        if unchecked
            .live_intents
            .iter()
            .any(|(key, intent)| key != intent.intent_id())
        {
            return Err(D::Error::custom(
                "live intent collection key does not match record ID",
            ));
        }

        if unchecked
            .live_intents
            .values()
            .any(|intent| intent.terminal_tick().is_some())
        {
            return Err(D::Error::custom(
                "live intent collection contains a terminal record",
            ));
        }
        if unchecked.terminal_intents.len() > TERMINAL_INTENT_CAPACITY {
            return Err(D::Error::custom(
                "terminal intent history exceeds 256 entries",
            ));
        }
        if unchecked
            .terminal_intents
            .iter()
            .any(|intent| intent.terminal_tick().is_none())
        {
            return Err(D::Error::custom(
                "terminal intent history contains a nonterminal record",
            ));
        }
        let terminal_ids = unchecked
            .terminal_intents
            .iter()
            .map(IntentCollectionRecord::intent_id)
            .collect::<BTreeSet<_>>();
        if terminal_ids.len() != unchecked.terminal_intents.len() {
            return Err(D::Error::custom(
                "terminal intent history contains duplicate IDs",
            ));
        }
        if unchecked
            .live_intents
            .keys()
            .any(|id| terminal_ids.contains(id))
        {
            return Err(D::Error::custom(
                "intent ID appears in both live and terminal collections",
            ));
        }
        if !unchecked.terminal_intents.windows(2).all(|pair| {
            (pair[0].terminal_tick(), pair[0].intent_id())
                < (pair[1].terminal_tick(), pair[1].intent_id())
        }) {
            return Err(D::Error::custom(
                "terminal intent history is not in canonical completion order",
            ));
        }

        Ok(Self {
            live_intents: unchecked.live_intents,
            terminal_intents: unchecked.terminal_intents,
        })
    }
}

impl<T> BoundedIntentCollections<T>
where
    T: IntentCollectionRecord,
{
    #[must_use]
    pub const fn new() -> Self {
        Self {
            live_intents: BTreeMap::new(),
            terminal_intents: Vec::new(),
        }
    }

    #[must_use]
    pub fn live_len(&self) -> usize {
        self.live_intents.len()
    }

    #[must_use]
    pub fn terminal_len(&self) -> usize {
        self.terminal_intents.len()
    }

    pub fn live_intents(&self) -> impl ExactSizeIterator<Item = &T> {
        self.live_intents.values()
    }

    #[must_use]
    pub fn terminal_intents(&self) -> &[T] {
        &self.terminal_intents
    }

    /// Insert in stable-ID order. Replacing the same intent is allowed at cap;
    /// a distinct 129th live intent is returned untouched.
    pub fn insert_live(&mut self, intent: T) -> Result<Option<T>, T> {
        let id = intent.intent_id().clone();
        if !self.live_intents.contains_key(&id) && self.live_intents.len() >= LIVE_INTENT_CAPACITY {
            return Err(intent);
        }
        Ok(self.live_intents.insert(id, intent))
    }

    pub fn remove_live(&mut self, id: &IntentId) -> Option<T> {
        self.live_intents.remove(id)
    }

    /// Add one terminal record, deduplicate by stable ID, and evict by oldest
    /// completion tick then stable ID.
    pub fn push_terminal(
        &mut self,
        intent: T,
    ) -> Result<TerminalInsertOutcome<T>, NonTerminalHistoryError<T>> {
        if intent.terminal_tick().is_none() {
            return Err(NonTerminalHistoryError(intent));
        }

        let id = intent.intent_id().clone();
        self.live_intents.remove(&id);
        let replaced = self
            .terminal_intents
            .iter()
            .position(|existing| existing.intent_id() == &id)
            .map(|position| self.terminal_intents.remove(position));
        self.terminal_intents.push(intent);
        self.terminal_intents.sort_by(|left, right| {
            left.terminal_tick()
                .cmp(&right.terminal_tick())
                .then_with(|| left.intent_id().cmp(right.intent_id()))
        });
        let evicted = (self.terminal_intents.len() > TERMINAL_INTENT_CAPACITY)
            .then(|| self.terminal_intents.remove(0));

        Ok(TerminalInsertOutcome { replaced, evicted })
    }
}

impl<T> Default for BoundedIntentCollections<T>
where
    T: IntentCollectionRecord,
{
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalInsertOutcome<T> {
    pub replaced: Option<T>,
    pub evicted: Option<T>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonTerminalHistoryError<T>(pub T);

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct TestIntent {
        id: IntentId,
        terminal_tick: Option<u64>,
        payload: u32,
    }

    impl IntentCollectionRecord for TestIntent {
        fn intent_id(&self) -> &IntentId {
            &self.id
        }

        fn terminal_tick(&self) -> Option<u64> {
            self.terminal_tick
        }
    }

    fn test_intent(kind: &str, occurrence: u32, terminal_tick: Option<u64>) -> TestIntent {
        TestIntent {
            id: IntentId::derive("colony-1", 7, kind, "target", occurrence),
            terminal_tick,
            payload: occurrence,
        }
    }

    #[test]
    fn planner_contract_exposes_version_and_bounds() {
        assert_eq!(PLANNER_SCHEMA_VERSION, 1);
        assert_eq!(LIVE_INTENT_CAPACITY, 128);
        assert_eq!(TERMINAL_INTENT_CAPACITY, 256);
        assert_eq!(
            serde_json::to_value(PlannerCoreState::new()).unwrap(),
            serde_json::json!({
                "schemaVersion": 1,
                "planningClock": 0,
                "planningEpoch": 0,
            })
        );
    }

    #[test]
    fn stable_intent_ids_are_input_keyed_and_unambiguous() {
        let first = IntentId::derive("colony:1", 7, "build", "den:west", 0);
        let twin = IntentId::derive("colony:1", 7, "build", "den:west", 0);
        let delimiter_collision = IntentId::derive("colony", 7, "1:build", "den:west", 0);

        assert_eq!(first, twin);
        assert_ne!(first, delimiter_collision);
        assert_ne!(
            first,
            IntentId::derive("colony:1", 8, "build", "den:west", 0)
        );
        assert_ne!(
            first,
            IntentId::derive("colony:1", 7, "build", "den:west", 1)
        );
    }

    #[test]
    fn planner_rng_streams_are_keyed_twins_and_isolated() {
        let key = ["colony-1", "leader", "food", "review-7"];
        let omission = planner_roll(42, PlannerRngStream::Omission, key);
        let omission_twin = planner_roll(42, PlannerRngStream::Omission, key);
        let appointment = planner_roll(42, PlannerRngStream::Appointment, key);

        assert_eq!(omission, omission_twin);
        assert_ne!(omission.next_seed, appointment.next_seed);
        assert_eq!(
            omission,
            planner_roll(42, PlannerRngStream::Omission, key),
            "drawing another stream must not advance an omission chain"
        );
        assert_eq!(PlannerRngStream::Omission.fork_offset(), 4_000_003);
        assert_eq!(PlannerRngStream::Injury.fork_offset(), 8_000_003);
        assert_ne!(
            keyed_planner_seed(42, PlannerRngStream::Omission, ["ab", "c"]),
            keyed_planner_seed(42, PlannerRngStream::Omission, ["a", "bc"]),
            "length-prefixing must distinguish stable-key partitions"
        );
    }

    #[test]
    fn score_uses_basis_points_without_floating_point() {
        let score = score_intent(IntentScoreInputs {
            urgency: BasisPoints::new(8_000),
            strategic_weight: BasisPoints::new(12_500),
            personality_weight: BasisPoints::new(9_000),
            confidence: BasisPoints::new(7_500),
            opportunity_cost: BasisPoints::new(500),
            churn_penalty: BasisPoints::new(250),
            starvation_age: BasisPoints::new(1_000),
            temporary_player_bias: BasisPoints::new(1_500),
        });

        assert_eq!(score, PlannerScore::new(8_500));
        assert_eq!(serde_json::to_string(&score).unwrap(), "8500");

        let saturated = score_intent(IntentScoreInputs {
            urgency: BasisPoints::new(i64::MAX),
            strategic_weight: BasisPoints::new(i64::MAX),
            personality_weight: BasisPoints::new(i64::MAX),
            confidence: BasisPoints::new(i64::MAX),
            opportunity_cost: BasisPoints::new(0),
            churn_penalty: BasisPoints::new(0),
            starvation_age: BasisPoints::new(i64::MAX),
            temporary_player_bias: BasisPoints::new(i64::MAX),
        });
        assert_eq!(saturated, PlannerScore::new(i64::MAX));
    }

    #[test]
    fn starvation_aging_counts_only_full_hours_and_caps_at_twenty_five_points() {
        assert_eq!(starvation_age_basis_points(59, 60), BasisPoints::new(0));
        assert_eq!(starvation_age_basis_points(60, 60), BasisPoints::new(100));
        assert_eq!(
            starvation_age_basis_points(25 * 60, 60),
            BasisPoints::new(2_500)
        );
        assert_eq!(
            starvation_age_basis_points(u64::MAX, 60),
            BasisPoints::new(2_500)
        );
        assert_eq!(starvation_age_basis_points(100, 0), BasisPoints::new(0));
    }

    #[test]
    fn priority_ties_sort_by_kind_creation_intent_then_target() {
        let id_a = IntentId::derive("colony-1", 1, "build", "a", 0);
        let id_b = IntentId::derive("colony-1", 1, "build", "a", 1);
        let target_a = PlannerId::derive("target", ["a"]);
        let target_b = PlannerId::derive("target", ["b"]);
        let mut entries = [
            (
                PlannerScore::new(500),
                IntentTieKey {
                    kind: "water".to_owned(),
                    creation_tick: 1,
                    intent_id: id_a.clone(),
                    target_id: target_a.clone(),
                },
            ),
            (
                PlannerScore::new(500),
                IntentTieKey {
                    kind: "build".to_owned(),
                    creation_tick: 2,
                    intent_id: id_a.clone(),
                    target_id: target_a.clone(),
                },
            ),
            (
                PlannerScore::new(500),
                IntentTieKey {
                    kind: "build".to_owned(),
                    creation_tick: 1,
                    intent_id: id_b,
                    target_id: target_a,
                },
            ),
            (
                PlannerScore::new(600),
                IntentTieKey {
                    kind: "water".to_owned(),
                    creation_tick: 99,
                    intent_id: id_a,
                    target_id: target_b,
                },
            ),
        ];
        entries.reverse();
        entries.sort_by(|left, right| compare_intent_priority(left.0, &left.1, right.0, &right.1));

        assert_eq!(entries[0].0, PlannerScore::new(600));
        assert_eq!(entries[1].1.kind, "build");
        assert_eq!(entries[1].1.creation_tick, 1);
        assert_eq!(entries[2].1.kind, "build");
        assert_eq!(entries[2].1.creation_tick, 2);
        assert_eq!(entries[3].1.kind, "water");
    }

    #[test]
    fn lifecycle_allows_main_path_blocks_and_retry_but_never_resurrects_terminal_work() {
        let mut lifecycle = IntentLifecycle::proposed();
        lifecycle.transition(IntentState::Approved, 1).unwrap();
        lifecycle.transition(IntentState::Reserving, 2).unwrap();
        lifecycle.transition(IntentState::Blocked, 3).unwrap();
        lifecycle.transition(IntentState::RetryWaiting, 4).unwrap();
        lifecycle.transition(IntentState::Reserving, 5).unwrap();
        lifecycle.transition(IntentState::Active, 6).unwrap();
        lifecycle.transition(IntentState::Succeeded, 7).unwrap();

        assert_eq!(lifecycle.terminal_tick, Some(7));
        assert_eq!(lifecycle.next_retry_tick, None);
        assert_eq!(
            lifecycle.transition(IntentState::Approved, 8),
            Err(InvalidIntentTransition {
                from: IntentState::Succeeded,
                to: IntentState::Approved,
            })
        );
        assert_eq!(lifecycle.terminal_tick, Some(7));
    }

    #[test]
    fn live_collection_rejects_only_a_distinct_intent_past_the_cap() {
        let mut collections = BoundedIntentCollections::new();
        for occurrence in 0..LIVE_INTENT_CAPACITY as u32 {
            collections
                .insert_live(test_intent("live", occurrence, None))
                .unwrap();
        }

        let ids = collections
            .live_intents()
            .map(|intent| intent.id.clone())
            .collect::<Vec<_>>();
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
        let rejected = collections
            .insert_live(test_intent("overflow", 0, None))
            .unwrap_err();
        assert_eq!(rejected.payload, 0);
        assert_eq!(collections.live_len(), LIVE_INTENT_CAPACITY);

        let mut replacement = test_intent("live", 0, None);
        replacement.payload = 999;
        let replaced = collections.insert_live(replacement).unwrap().unwrap();
        assert_eq!(replaced.payload, 0);
        assert_eq!(collections.live_len(), LIVE_INTENT_CAPACITY);
    }

    #[test]
    fn bounded_collections_round_trip_and_reject_malformed_persisted_state() {
        let mut collections = BoundedIntentCollections::new();
        collections
            .insert_live(test_intent("live", 0, None))
            .unwrap();
        collections
            .push_terminal(test_intent("done", 0, Some(10)))
            .unwrap();
        let json = serde_json::to_string(&collections).unwrap();
        let restored: BoundedIntentCollections<TestIntent> = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, collections);
        assert_eq!(serde_json::to_string(&restored).unwrap(), json);

        let persisted_value = |live_intents: BTreeMap<IntentId, TestIntent>,
                               terminal_intents: Vec<TestIntent>| {
            serde_json::json!({
                "liveIntents": live_intents,
                "terminalIntents": terminal_intents,
            })
        };
        let assert_rejected = |value, expected: &str| {
            let error =
                serde_json::from_value::<BoundedIntentCollections<TestIntent>>(value).unwrap_err();
            assert!(
                error.to_string().contains(expected),
                "expected {expected:?}, got {error}"
            );
        };

        let oversized = (0..=LIVE_INTENT_CAPACITY as u32)
            .map(|occurrence| {
                let intent = test_intent("loaded", occurrence, None);
                (intent.id.clone(), intent)
            })
            .collect::<BTreeMap<_, _>>();
        assert_rejected(
            persisted_value(oversized, Vec::new()),
            "exceeds 128 entries",
        );

        let terminal_live = test_intent("terminal-live", 0, Some(1));
        assert_rejected(
            persisted_value(
                BTreeMap::from([(terminal_live.id.clone(), terminal_live)]),
                Vec::new(),
            ),
            "live intent collection contains a terminal record",
        );

        let oversized_history = (0..=TERMINAL_INTENT_CAPACITY as u32)
            .map(|occurrence| test_intent("history", occurrence, Some(occurrence as u64)))
            .collect();
        assert_rejected(
            persisted_value(BTreeMap::new(), oversized_history),
            "terminal intent history exceeds 256 entries",
        );
        assert_rejected(
            persisted_value(
                BTreeMap::new(),
                vec![test_intent("nonterminal-history", 0, None)],
            ),
            "terminal intent history contains a nonterminal record",
        );

        let duplicate = test_intent("duplicate", 0, Some(1));
        assert_rejected(
            persisted_value(BTreeMap::new(), vec![duplicate.clone(), duplicate.clone()]),
            "terminal intent history contains duplicate IDs",
        );

        let overlap_live = test_intent("overlap", 0, None);
        let overlap_terminal = test_intent("overlap", 0, Some(1));
        assert_rejected(
            persisted_value(
                BTreeMap::from([(overlap_live.id.clone(), overlap_live)]),
                vec![overlap_terminal],
            ),
            "intent ID appears in both live and terminal collections",
        );

        assert_rejected(
            persisted_value(
                BTreeMap::new(),
                vec![
                    test_intent("ordered", 1, Some(2)),
                    test_intent("ordered", 0, Some(1)),
                ],
            ),
            "terminal intent history is not in canonical completion order",
        );

        let keyed_as = test_intent("key", 0, None);
        let record_b = test_intent("record", 0, None);
        assert_rejected(
            persisted_value(BTreeMap::from([(keyed_as.id, record_b)]), Vec::new()),
            "live intent collection key does not match record ID",
        );
    }

    #[test]
    fn terminal_history_evicts_oldest_completion_then_stable_id() {
        let mut collections = BoundedIntentCollections::new();
        for occurrence in 0..254 {
            collections
                .push_terminal(test_intent(
                    "recent",
                    occurrence,
                    Some(100 + occurrence as u64),
                ))
                .unwrap();
        }
        let oldest_a = test_intent("oldest", 0, Some(10));
        let oldest_b = test_intent("oldest", 1, Some(10));
        let expected_evicted = oldest_a.id.clone().min(oldest_b.id.clone());
        collections.push_terminal(oldest_a).unwrap();
        collections.push_terminal(oldest_b).unwrap();

        let outcome = collections
            .push_terminal(test_intent("newest", 0, Some(10_000)))
            .unwrap();
        assert_eq!(collections.terminal_len(), TERMINAL_INTENT_CAPACITY);
        assert_eq!(outcome.evicted.unwrap().id, expected_evicted);
        assert!(collections.terminal_intents().windows(2).all(|pair| (
            pair[0].terminal_tick,
            &pair[0].id
        ) <= (
            pair[1].terminal_tick,
            &pair[1].id
        )));
    }

    #[test]
    fn terminal_history_rejects_live_records_and_deduplicates_stable_ids() {
        let mut collections = BoundedIntentCollections::new();
        let live = test_intent("one", 0, None);
        assert_eq!(
            collections.push_terminal(live.clone()),
            Err(NonTerminalHistoryError(live))
        );

        let first = test_intent("one", 0, Some(10));
        collections.push_terminal(first.clone()).unwrap();
        let mut replacement = first.clone();
        replacement.terminal_tick = Some(20);
        let outcome = collections.push_terminal(replacement).unwrap();
        assert_eq!(outcome.replaced, Some(first));
        assert_eq!(collections.terminal_len(), 1);
        assert_eq!(collections.terminal_intents()[0].terminal_tick, Some(20));
    }

    #[test]
    fn retry_schedule_and_terminal_failure_are_exact() {
        assert_eq!(RETRY_DELAYS_GAME_MINUTES, [15, 30, 60, 120, 240]);
        assert_eq!(retry_delay_game_minutes(1), Some(15));
        assert_eq!(retry_delay_game_minutes(5), Some(240));
        assert_eq!(retry_delay_game_minutes(6), None);
        assert_eq!(
            failure_disposition(1, 1_000, 60),
            Some(FailureDisposition::RetryAt(1_900))
        );
        assert_eq!(
            failure_disposition(5, 1_000, 60),
            Some(FailureDisposition::TerminalFailure)
        );
        assert_eq!(failure_disposition(0, 1_000, 60), None);
        assert_eq!(
            failure_disposition(4, u64::MAX - 1, u64::MAX),
            Some(FailureDisposition::RetryAt(u64::MAX))
        );
    }
}
