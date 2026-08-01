//! Deterministic world-level diplomacy state specified by
//! `docs/leader-ai-overhaul/diplomacy-trade.md`.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    authority::{
        AuthorityActor, AuthorityContext, AuthorityDecision, AuthorityDenial, AuthorityDomain,
        AuthorityOperation, decide_authority,
    },
    planner_core::PlannerId,
};

pub const DIPLOMACY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiplomacyColonyId {
    stable_id: PlannerId,
    external_id: String,
}

impl Serialize for DiplomacyColonyId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Wire<'a> {
            stable_id: &'a PlannerId,
            external_id: &'a str,
        }
        Wire {
            stable_id: &self.stable_id,
            external_id: &self.external_id,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DiplomacyColonyId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Legacy(String),
            #[serde(rename_all = "camelCase")]
            Current {
                stable_id: PlannerId,
                #[serde(default)]
                external_id: String,
            },
        }
        match Wire::deserialize(deserializer)? {
            Wire::Legacy(stable) => Ok(Self {
                stable_id: PlannerId::derive("legacy_diplomacy_colony", [stable.as_str()]),
                external_id: stable,
            }),
            Wire::Current {
                stable_id,
                external_id,
            } => Ok(Self {
                stable_id,
                external_id,
            }),
        }
    }
}

impl DiplomacyColonyId {
    #[must_use]
    pub fn derive(external_id: &str) -> Self {
        Self {
            stable_id: PlannerId::derive("diplomacy_colony", [external_id]),
            external_id: external_id.to_owned(),
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.stable_id.as_str()
    }

    #[must_use]
    pub fn external_id(&self) -> &str {
        &self.external_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DiplomacyPairId(PlannerId);

impl DiplomacyPairId {
    #[must_use]
    fn derive(first: &DiplomacyColonyId, second: &DiplomacyColonyId) -> Self {
        Self(PlannerId::derive(
            "diplomacy_pair",
            [first.as_str(), second.as_str()],
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Canonical unordered colony pair. The lower stable colony ID is always
/// first and the pair ID is derived from that canonical order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiplomacyPair {
    id: DiplomacyPairId,
    first: DiplomacyColonyId,
    second: DiplomacyColonyId,
}

impl DiplomacyPair {
    pub fn new(
        first: DiplomacyColonyId,
        second: DiplomacyColonyId,
    ) -> Result<Self, DiplomacyError> {
        if first == second {
            return Err(DiplomacyError::SameColony);
        }
        let (first, second) = if first < second {
            (first, second)
        } else {
            (second, first)
        };
        Ok(Self {
            id: DiplomacyPairId::derive(&first, &second),
            first,
            second,
        })
    }

    #[must_use]
    pub const fn id(&self) -> &DiplomacyPairId {
        &self.id
    }

    #[must_use]
    pub const fn first(&self) -> &DiplomacyColonyId {
        &self.first
    }

    #[must_use]
    pub const fn second(&self) -> &DiplomacyColonyId {
        &self.second
    }

    #[must_use]
    pub fn contains(&self, colony_id: &DiplomacyColonyId) -> bool {
        colony_id == &self.first || colony_id == &self.second
    }
}

impl<'de> Deserialize<'de> for DiplomacyPair {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct PersistedPair {
            id: DiplomacyPairId,
            first: DiplomacyColonyId,
            second: DiplomacyColonyId,
        }

        let persisted = PersistedPair::deserialize(deserializer)?;
        if persisted.first >= persisted.second {
            return Err(serde::de::Error::custom(
                "diplomacy pair colonies must be distinct and canonically ordered",
            ));
        }
        let expected = DiplomacyPairId::derive(&persisted.first, &persisted.second);
        if persisted.id != expected {
            return Err(serde::de::Error::custom(
                "diplomacy pair ID does not match its colonies",
            ));
        }
        Ok(Self {
            id: persisted.id,
            first: persisted.first,
            second: persisted.second,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DiplomacyActionId(PlannerId);

impl DiplomacyActionId {
    #[must_use]
    pub fn derive(
        pair_id: &DiplomacyPairId,
        acting_colony_id: &DiplomacyColonyId,
        occurrence_id: &str,
    ) -> Self {
        Self(PlannerId::derive(
            "diplomacy_action",
            [pair_id.as_str(), acting_colony_id.as_str(), occurrence_id],
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiplomacyRelationship {
    Neutral,
    Friendly,
    Allied,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposedRelationship {
    Friendly,
    Allied,
}

impl ProposedRelationship {
    #[must_use]
    const fn relationship(self) -> DiplomacyRelationship {
        match self {
            Self::Friendly => DiplomacyRelationship::Friendly,
            Self::Allied => DiplomacyRelationship::Allied,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingDiplomacyConsent {
    pub proposal_action_id: DiplomacyActionId,
    pub target: ProposedRelationship,
    pub proposed_by: DiplomacyColonyId,
    pub approvals: BTreeSet<DiplomacyColonyId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiplomacyRecord {
    pub pair: DiplomacyPair,
    pub version: u64,
    pub relationship: DiplomacyRelationship,
    pub pending_consent: Option<PendingDiplomacyConsent>,
    pub blocked_by: BTreeSet<DiplomacyColonyId>,
    #[serde(default)]
    pub updated_at_tick: u64,
}

impl DiplomacyRecord {
    #[must_use]
    fn neutral(pair: DiplomacyPair) -> Self {
        Self {
            pair,
            version: 0,
            relationship: DiplomacyRelationship::Neutral,
            pending_consent: None,
            blocked_by: BTreeSet::new(),
            updated_at_tick: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiplomacyActionKind {
    Propose(ProposedRelationship),
    Approve,
    Block,
    Unblock,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiplomacyAction {
    pub id: DiplomacyActionId,
    pub pair: DiplomacyPair,
    pub acting_colony_id: DiplomacyColonyId,
    pub expected_version: u64,
    pub kind: DiplomacyActionKind,
}

/// Server-provided authorization facts. The simulation verifies that the God
/// actor, authenticated player, claimed colony, action, and pair all agree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiplomacyAuthorization {
    pub actor: AuthorityActor,
    pub acting_colony_id: DiplomacyColonyId,
    pub owner_player_id: PlannerId,
    pub player_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedDiplomacyAction {
    pub action: DiplomacyAction,
    pub authorization: DiplomacyAuthorization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiplomacyOutcome {
    Proposed,
    ApprovalRecorded,
    RelationshipActivated,
    Blocked,
    BlockerAdded,
    BlockerRemoved,
    UnblockedToNeutral,
    NoChange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiplomacyReceipt {
    pub action_id: DiplomacyActionId,
    pub pair_id: DiplomacyPairId,
    pub acting_colony_id: DiplomacyColonyId,
    pub actor_player_id: PlannerId,
    pub expected_version: u64,
    pub relationship_version: u64,
    pub relationship: DiplomacyRelationship,
    pub kind: DiplomacyActionKind,
    pub outcome: DiplomacyOutcome,
}

impl DiplomacyReceipt {
    fn matches_replay(&self, action: &DiplomacyAction, actor_player_id: &PlannerId) -> bool {
        self.action_id == action.id
            && self.pair_id == *action.pair.id()
            && self.acting_colony_id == action.acting_colony_id
            && self.actor_player_id == *actor_player_id
            && self.expected_version == action.expected_version
            && self.kind == action.kind
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiplomacyBatchResult {
    pub action_id: DiplomacyActionId,
    pub result: Result<DiplomacyReceipt, DiplomacyError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiplomacyError {
    SameColony,
    ActingColonyNotParty,
    AuthorizationColonyMismatch,
    PlayerIdentityMismatch,
    AuthorityDenied(AuthorityDenial),
    StaleVersion { expected: u64, actual: u64 },
    RelationshipBlocked,
    PendingProposalExists,
    NoPendingProposal,
    NotBlocker,
    ActionIdCollision,
    VersionExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiplomacyLedger {
    relationships: BTreeMap<DiplomacyPairId, DiplomacyRecord>,
    action_results: BTreeMap<DiplomacyActionId, DiplomacyReceipt>,
}

impl DiplomacyLedger {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            relationships: BTreeMap::new(),
            action_results: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn record(&self, pair_id: &DiplomacyPairId) -> Option<&DiplomacyRecord> {
        self.relationships.get(pair_id)
    }

    /// Iterate public relationship/consent records in stable pair-ID order.
    pub fn records(&self) -> impl ExactSizeIterator<Item = &DiplomacyRecord> {
        self.relationships.values()
    }

    #[must_use]
    pub fn relationship(&self, pair: &DiplomacyPair) -> DiplomacyRelationship {
        self.record(pair.id())
            .map_or(DiplomacyRelationship::Neutral, |record| record.relationship)
    }

    /// Apply one authorized action atomically. `Block` deliberately bypasses
    /// stale-version rejection so an authorized safety block wins a concurrent
    /// approval race; every other mutation requires an exact version.
    pub fn apply(
        &mut self,
        action: DiplomacyAction,
        authorization: DiplomacyAuthorization,
    ) -> Result<DiplomacyReceipt, DiplomacyError> {
        self.apply_at(action, authorization, 0)
    }

    pub fn apply_at(
        &mut self,
        action: DiplomacyAction,
        authorization: DiplomacyAuthorization,
        now_tick: u64,
    ) -> Result<DiplomacyReceipt, DiplomacyError> {
        let actor_player_id = validate_authorization(&action, &authorization)?;

        if let Some(receipt) = self.action_results.get(&action.id) {
            return if receipt.matches_replay(&action, actor_player_id) {
                Ok(receipt.clone())
            } else {
                Err(DiplomacyError::ActionIdCollision)
            };
        }

        let pair_id = action.pair.id().clone();
        let mut next = self
            .relationships
            .get(&pair_id)
            .cloned()
            .unwrap_or_else(|| DiplomacyRecord::neutral(action.pair.clone()));

        if action.kind != DiplomacyActionKind::Block && action.expected_version != next.version {
            return Err(DiplomacyError::StaleVersion {
                expected: action.expected_version,
                actual: next.version,
            });
        }

        let (outcome, mutated) = apply_to_record(&mut next, &action)?;
        if mutated {
            next.version = next
                .version
                .checked_add(1)
                .ok_or(DiplomacyError::VersionExhausted)?;
        }

        let receipt = DiplomacyReceipt {
            action_id: action.id.clone(),
            pair_id: pair_id.clone(),
            acting_colony_id: action.acting_colony_id,
            actor_player_id: actor_player_id.clone(),
            expected_version: action.expected_version,
            relationship_version: next.version,
            relationship: next.relationship,
            kind: action.kind,
            outcome,
        };

        if mutated {
            next.updated_at_tick = now_tick;
            self.relationships.insert(pair_id, next);
        }
        self.action_results
            .insert(receipt.action_id.clone(), receipt.clone());
        Ok(receipt)
    }

    /// Resolve a concurrently collected batch by stable action ID, independent
    /// of vector/map collection order.
    pub fn apply_batch(
        &mut self,
        mut actions: Vec<AuthorizedDiplomacyAction>,
    ) -> Vec<DiplomacyBatchResult> {
        actions.sort_by(|first, second| first.action.id.cmp(&second.action.id));
        actions
            .into_iter()
            .map(|authorized| {
                let action_id = authorized.action.id.clone();
                let result = self.apply(authorized.action, authorized.authorization);
                DiplomacyBatchResult { action_id, result }
            })
            .collect()
    }
}

impl Default for DiplomacyLedger {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_authorization<'a>(
    action: &DiplomacyAction,
    authorization: &'a DiplomacyAuthorization,
) -> Result<&'a PlannerId, DiplomacyError> {
    if !action.pair.contains(&action.acting_colony_id) {
        return Err(DiplomacyError::ActingColonyNotParty);
    }
    if authorization.acting_colony_id != action.acting_colony_id {
        return Err(DiplomacyError::AuthorizationColonyMismatch);
    }
    let decision = decide_authority(
        &authorization.actor,
        AuthorityOperation::ApproveDiplomacy,
        AuthorityDomain::Diplomacy,
        AuthorityContext {
            leader_present: false,
            player_authorized: authorization.player_authorized,
        },
    );
    if let AuthorityDecision::Denied(reason) = decision {
        return Err(DiplomacyError::AuthorityDenied(reason));
    }
    let AuthorityActor::God { player_id } = &authorization.actor else {
        return Err(DiplomacyError::AuthorityDenied(
            AuthorityDenial::StrategyForbidden,
        ));
    };
    if player_id != &authorization.owner_player_id {
        return Err(DiplomacyError::PlayerIdentityMismatch);
    }
    Ok(player_id)
}

fn apply_to_record(
    record: &mut DiplomacyRecord,
    action: &DiplomacyAction,
) -> Result<(DiplomacyOutcome, bool), DiplomacyError> {
    match action.kind {
        DiplomacyActionKind::Propose(target) => {
            if record.relationship == DiplomacyRelationship::Blocked {
                return Err(DiplomacyError::RelationshipBlocked);
            }
            if record.relationship == target.relationship() && record.pending_consent.is_none() {
                return Ok((DiplomacyOutcome::NoChange, false));
            }
            if let Some(pending) = &record.pending_consent {
                if pending.target == target && pending.proposed_by == action.acting_colony_id {
                    return Ok((DiplomacyOutcome::NoChange, false));
                }
                return Err(DiplomacyError::PendingProposalExists);
            }
            record.pending_consent = Some(PendingDiplomacyConsent {
                proposal_action_id: action.id.clone(),
                target,
                proposed_by: action.acting_colony_id.clone(),
                approvals: BTreeSet::new(),
            });
            Ok((DiplomacyOutcome::Proposed, true))
        }
        DiplomacyActionKind::Approve => {
            if record.relationship == DiplomacyRelationship::Blocked {
                return Err(DiplomacyError::RelationshipBlocked);
            }
            let Some(pending) = &mut record.pending_consent else {
                return Err(DiplomacyError::NoPendingProposal);
            };
            if !pending.approvals.insert(action.acting_colony_id.clone()) {
                return Ok((DiplomacyOutcome::NoChange, false));
            }
            if pending.approvals.contains(record.pair.first())
                && pending.approvals.contains(record.pair.second())
            {
                record.relationship = pending.target.relationship();
                record.pending_consent = None;
                Ok((DiplomacyOutcome::RelationshipActivated, true))
            } else {
                Ok((DiplomacyOutcome::ApprovalRecorded, true))
            }
        }
        DiplomacyActionKind::Block => {
            let was_blocked = record.relationship == DiplomacyRelationship::Blocked;
            let inserted = record.blocked_by.insert(action.acting_colony_id.clone());
            let cleared_pending = record.pending_consent.take().is_some();
            record.relationship = DiplomacyRelationship::Blocked;
            if was_blocked && !inserted && !cleared_pending {
                Ok((DiplomacyOutcome::NoChange, false))
            } else if was_blocked {
                Ok((DiplomacyOutcome::BlockerAdded, true))
            } else {
                Ok((DiplomacyOutcome::Blocked, true))
            }
        }
        DiplomacyActionKind::Unblock => {
            if record.relationship != DiplomacyRelationship::Blocked
                || !record.blocked_by.remove(&action.acting_colony_id)
            {
                return Err(DiplomacyError::NotBlocker);
            }
            if record.blocked_by.is_empty() {
                record.relationship = DiplomacyRelationship::Neutral;
                Ok((DiplomacyOutcome::UnblockedToNeutral, true))
            } else {
                Ok((DiplomacyOutcome::BlockerRemoved, true))
            }
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PersistedDiplomacyLedger<'a> {
    schema_version: u32,
    relationships: Vec<&'a DiplomacyRecord>,
    action_results: Vec<&'a DiplomacyReceipt>,
}

impl Serialize for DiplomacyLedger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        PersistedDiplomacyLedger {
            schema_version: DIPLOMACY_SCHEMA_VERSION,
            relationships: self.relationships.values().collect(),
            action_results: self.action_results.values().collect(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DiplomacyLedger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct PersistedDiplomacyLedger {
            schema_version: u32,
            relationships: Vec<DiplomacyRecord>,
            action_results: Vec<DiplomacyReceipt>,
        }

        let persisted = PersistedDiplomacyLedger::deserialize(deserializer)?;
        if persisted.schema_version != DIPLOMACY_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(format_args!(
                "unsupported diplomacy schema version {}",
                persisted.schema_version
            )));
        }

        let mut relationships = BTreeMap::new();
        for record in persisted.relationships {
            validate_record(&record).map_err(serde::de::Error::custom)?;
            let pair_id = record.pair.id().clone();
            if relationships.insert(pair_id, record).is_some() {
                return Err(serde::de::Error::custom("duplicate diplomacy pair"));
            }
        }

        let mut action_results = BTreeMap::new();
        for receipt in persisted.action_results {
            let Some(record) = relationships.get(&receipt.pair_id) else {
                return Err(serde::de::Error::custom(
                    "diplomacy action result references a missing pair",
                ));
            };
            if !record.pair.contains(&receipt.acting_colony_id)
                || receipt.relationship_version == 0
                || receipt.relationship_version > record.version
            {
                return Err(serde::de::Error::custom("invalid diplomacy action result"));
            }
            if action_results
                .insert(receipt.action_id.clone(), receipt)
                .is_some()
            {
                return Err(serde::de::Error::custom(
                    "duplicate diplomacy action result",
                ));
            }
        }

        Ok(Self {
            relationships,
            action_results,
        })
    }
}

fn validate_record(record: &DiplomacyRecord) -> Result<(), &'static str> {
    if record.version == 0 {
        return Err("persisted diplomacy records must have a positive version");
    }
    if record.relationship == DiplomacyRelationship::Blocked {
        if record.blocked_by.is_empty() || record.pending_consent.is_some() {
            return Err("blocked diplomacy record has invalid blocker/consent state");
        }
    } else if !record.blocked_by.is_empty() {
        return Err("non-blocked diplomacy record retains blockers");
    }
    if record
        .blocked_by
        .iter()
        .any(|colony_id| !record.pair.contains(colony_id))
    {
        return Err("diplomacy blocker is not a pair member");
    }
    if let Some(pending) = &record.pending_consent {
        if !record.pair.contains(&pending.proposed_by)
            || pending
                .approvals
                .iter()
                .any(|colony_id| !record.pair.contains(colony_id))
            || pending.target.relationship() == record.relationship
        {
            return Err("invalid pending diplomacy consent");
        }
        if pending.approvals.contains(record.pair.first())
            && pending.approvals.contains(record.pair.second())
        {
            return Err("fully approved diplomacy consent was not activated");
        }
    }
    Ok(())
}
