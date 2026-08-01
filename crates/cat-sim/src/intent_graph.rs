//! Persistent intent graph, officer requests, and authority lifecycle specified by
//! `docs/leader-ai-overhaul/planner-and-beliefs.md`.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Serialize};

use crate::{
    authority::{
        AuthorityActor, AuthorityContext, AuthorityDecision, AuthorityDenial, AuthorityDomain,
        AuthorityOperation, decide_authority,
    },
    beliefs::{Confidence, EvidenceId, ReportId},
    planner_core::{
        BasisPoints, IntentCollectionRecord, IntentId, IntentLifecycle, IntentState,
        LIVE_INTENT_CAPACITY, PlannerId, TERMINAL_INTENT_CAPACITY,
    },
    spatial_tasks::SpatialObjective,
};

pub const INTENT_SCHEMA_VERSION: u32 = 1;
pub const INTENT_GRAPH_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentReason {
    EvidenceChanged,
    PlayerDismissed,
    LeaderCancelled,
    SuccessionReview,
    DeadlineExpired,
    DependencyCycle,
    AuthorityLost,
    PermanentInvalidity,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct IntentSemanticKey {
    colony_id: PlannerId,
    domain: AuthorityDomain,
    kind_id: PlannerId,
    target_id: PlannerId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Intent {
    pub schema_version: u32,
    pub id: IntentId,
    pub colony_id: PlannerId,
    pub proposer: AuthorityActor,
    pub leader_id: Option<PlannerId>,
    pub adopted_from_leader_id: Option<PlannerId>,
    pub authority_domain: AuthorityDomain,
    pub kind_id: PlannerId,
    pub target_id: PlannerId,
    pub rationale_id: PlannerId,
    pub evidence_ids: BTreeSet<EvidenceId>,
    pub report_ids: BTreeSet<ReportId>,
    pub belief_version: u64,
    pub creation_tick: u64,
    pub review_tick: u64,
    pub deadline_tick: Option<u64>,
    pub urgency: BasisPoints,
    pub strategic_weight: BasisPoints,
    pub confidence: Confidence,
    pub expected_benefit: BasisPoints,
    pub expected_cost: BasisPoints,
    pub dependencies: BTreeSet<IntentId>,
    pub dependents: BTreeSet<IntentId>,
    pub spatial_objective: Option<SpatialObjective>,
    pub resource_reservation_ids: BTreeSet<PlannerId>,
    pub delivery_reservation_ids: BTreeSet<PlannerId>,
    pub assigned_cat_ids: BTreeSet<PlannerId>,
    pub task_ids: BTreeSet<PlannerId>,
    pub lifecycle: IntentLifecycle,
    pub blocked_reason: Option<IntentReason>,
    pub temporary_player_bias: BasisPoints,
    pub standing_order_id: Option<PlannerId>,
}

impl Intent {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn proposed(
        id: IntentId,
        colony_id: PlannerId,
        proposer: AuthorityActor,
        leader_id: Option<PlannerId>,
        authority_domain: AuthorityDomain,
        kind_id: PlannerId,
        target_id: PlannerId,
        rationale_id: PlannerId,
        creation_tick: u64,
    ) -> Self {
        Self {
            schema_version: INTENT_SCHEMA_VERSION,
            id,
            colony_id,
            proposer,
            leader_id,
            adopted_from_leader_id: None,
            authority_domain,
            kind_id,
            target_id,
            rationale_id,
            evidence_ids: BTreeSet::new(),
            report_ids: BTreeSet::new(),
            belief_version: 0,
            creation_tick,
            review_tick: creation_tick,
            deadline_tick: None,
            urgency: BasisPoints::default(),
            strategic_weight: BasisPoints::new(10_000),
            confidence: Confidence::zero(),
            expected_benefit: BasisPoints::default(),
            expected_cost: BasisPoints::default(),
            dependencies: BTreeSet::new(),
            dependents: BTreeSet::new(),
            spatial_objective: None,
            resource_reservation_ids: BTreeSet::new(),
            delivery_reservation_ids: BTreeSet::new(),
            assigned_cat_ids: BTreeSet::new(),
            task_ids: BTreeSet::new(),
            lifecycle: IntentLifecycle::proposed(),
            blocked_reason: None,
            temporary_player_bias: BasisPoints::default(),
            standing_order_id: None,
        }
    }

    fn semantic_key(&self) -> IntentSemanticKey {
        IntentSemanticKey {
            colony_id: self.colony_id.clone(),
            domain: self.authority_domain,
            kind_id: self.kind_id.clone(),
            target_id: self.target_id.clone(),
        }
    }

    #[must_use]
    pub fn is_valid_at(&self, now_tick: u64) -> bool {
        !self.lifecycle.state.is_terminal()
            && self
                .deadline_tick
                .is_none_or(|deadline| now_tick < deadline)
    }
}

impl IntentCollectionRecord for Intent {
    fn intent_id(&self) -> &IntentId {
        &self.id
    }

    fn terminal_tick(&self) -> Option<u64> {
        self.lifecycle.terminal_tick
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentGraph {
    pub schema_version: u32,
    pub version: u64,
    intents: BTreeMap<IntentId, Intent>,
}

impl IntentGraph {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: INTENT_GRAPH_SCHEMA_VERSION,
            version: 0,
            intents: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn get(&self, id: &IntentId) -> Option<&Intent> {
        self.intents.get(id)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&IntentId, &Intent)> {
        self.intents.iter()
    }

    pub fn insert_or_merge(
        &mut self,
        mut intent: Intent,
    ) -> Result<IntentInsert, IntentGraphError> {
        validate_intent(&intent)?;
        if self.intents.contains_key(&intent.id) {
            return Ok(IntentInsert::DuplicateId(intent.id));
        }
        if let Some((existing_id, existing)) = self.intents.iter_mut().find(|(_, existing)| {
            !existing.lifecycle.state.is_terminal()
                && existing.semantic_key() == intent.semantic_key()
        }) {
            existing.evidence_ids.append(&mut intent.evidence_ids);
            existing.report_ids.append(&mut intent.report_ids);
            existing.urgency = existing.urgency.max(intent.urgency);
            existing.belief_version = existing.belief_version.max(intent.belief_version);
            existing.review_tick = existing.review_tick.max(intent.review_tick);
            self.version = self.version.saturating_add(1);
            return Ok(IntentInsert::Merged(existing_id.clone()));
        }
        if !intent.lifecycle.state.is_terminal()
            && self
                .intents
                .values()
                .filter(|existing| !existing.lifecycle.state.is_terminal())
                .count()
                >= LIVE_INTENT_CAPACITY
        {
            return Err(IntentGraphError::LiveCapacityReached);
        }
        let id = intent.id.clone();
        self.intents.insert(id.clone(), intent);
        self.version = self.version.saturating_add(1);
        self.evict_terminal_history();
        Ok(IntentInsert::Inserted(id))
    }

    pub fn add_dependency(
        &mut self,
        intent_id: &IntentId,
        dependency_id: &IntentId,
    ) -> Result<(), IntentGraphError> {
        if intent_id == dependency_id {
            return Err(IntentGraphError::DependencyCycle);
        }
        if !self.intents.contains_key(intent_id) || !self.intents.contains_key(dependency_id) {
            return Err(IntentGraphError::MissingIntent);
        }
        if self.reaches(dependency_id, intent_id) {
            return Err(IntentGraphError::DependencyCycle);
        }
        self.intents
            .get_mut(intent_id)
            .expect("checked")
            .dependencies
            .insert(dependency_id.clone());
        self.intents
            .get_mut(dependency_id)
            .expect("checked")
            .dependents
            .insert(intent_id.clone());
        self.version = self.version.saturating_add(1);
        Ok(())
    }

    pub fn cancel(
        &mut self,
        id: &IntentId,
        actor: &AuthorityActor,
        context: AuthorityContext,
        now_tick: u64,
        reason: IntentReason,
    ) -> Result<(), IntentGraphError> {
        let intent = self
            .intents
            .get_mut(id)
            .ok_or(IntentGraphError::MissingIntent)?;
        let decision = decide_authority(
            actor,
            AuthorityOperation::CancelIntent,
            intent.authority_domain,
            context,
        );
        if let AuthorityDecision::Denied(denial) = decision {
            return Err(IntentGraphError::AuthorityDenied(denial));
        }
        intent
            .lifecycle
            .transition(IntentState::Cancelled, now_tick)?;
        intent.blocked_reason = Some(reason);
        intent.resource_reservation_ids.clear();
        intent.delivery_reservation_ids.clear();
        intent.assigned_cat_ids.clear();
        intent.task_ids.clear();
        self.version = self.version.saturating_add(1);
        self.evict_terminal_history();
        Ok(())
    }

    pub fn succeed(&mut self, id: &IntentId, now_tick: u64) -> Result<(), IntentGraphError> {
        let intent = self
            .intents
            .get_mut(id)
            .ok_or(IntentGraphError::MissingIntent)?;
        intent
            .lifecycle
            .transition(IntentState::Succeeded, now_tick)?;
        intent.resource_reservation_ids.clear();
        intent.delivery_reservation_ids.clear();
        intent.assigned_cat_ids.clear();
        intent.task_ids.clear();
        self.version = self.version.saturating_add(1);
        self.evict_terminal_history();
        Ok(())
    }

    pub fn adopt_for_successor(
        &mut self,
        previous_leader_id: &PlannerId,
        successor_id: PlannerId,
        now_tick: u64,
    ) -> Vec<IntentId> {
        let mut adopted = Vec::new();
        for (id, intent) in &mut self.intents {
            if intent.leader_id.as_ref() == Some(previous_leader_id) && intent.is_valid_at(now_tick)
            {
                intent.adopted_from_leader_id = Some(previous_leader_id.clone());
                intent.leader_id = Some(successor_id.clone());
                intent.review_tick = now_tick;
                intent.blocked_reason = Some(IntentReason::SuccessionReview);
                adopted.push(id.clone());
            }
        }
        if !adopted.is_empty() {
            self.version = self.version.saturating_add(1);
        }
        adopted
    }

    pub fn expire_due(&mut self, now_tick: u64) -> Vec<IntentId> {
        let mut expired = Vec::new();
        for (id, intent) in &mut self.intents {
            if !intent.lifecycle.state.is_terminal()
                && intent
                    .deadline_tick
                    .is_some_and(|deadline| deadline <= now_tick)
            {
                intent
                    .lifecycle
                    .transition(IntentState::Failed, now_tick)
                    .expect("every live intent may fail at its deadline");
                intent.blocked_reason = Some(IntentReason::DeadlineExpired);
                intent.resource_reservation_ids.clear();
                intent.delivery_reservation_ids.clear();
                intent.assigned_cat_ids.clear();
                intent.task_ids.clear();
                expired.push(id.clone());
            }
        }
        if !expired.is_empty() {
            self.version = self.version.saturating_add(1);
            self.evict_terminal_history();
        }
        expired
    }

    fn reaches(&self, start: &IntentId, target: &IntentId) -> bool {
        let mut pending = vec![start.clone()];
        let mut visited = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if &id == target {
                return true;
            }
            if visited.insert(id.clone())
                && let Some(intent) = self.intents.get(&id)
            {
                pending.extend(intent.dependencies.iter().rev().cloned());
            }
        }
        false
    }

    fn evict_terminal_history(&mut self) {
        while self
            .intents
            .values()
            .filter(|intent| intent.lifecycle.state.is_terminal())
            .count()
            > TERMINAL_INTENT_CAPACITY
        {
            let Some(evicted_id) = self
                .intents
                .values()
                .filter_map(|intent| {
                    intent
                        .lifecycle
                        .terminal_tick
                        .map(|tick| (tick, intent.id.clone()))
                })
                .min()
                .map(|(_, id)| id)
            else {
                break;
            };
            self.intents.remove(&evicted_id);
            for intent in self.intents.values_mut() {
                intent.dependencies.remove(&evicted_id);
                intent.dependents.remove(&evicted_id);
            }
        }
    }

    fn validate(&self) -> Result<(), IntentGraphError> {
        if self
            .intents
            .values()
            .filter(|intent| !intent.lifecycle.state.is_terminal())
            .count()
            > LIVE_INTENT_CAPACITY
            || self
                .intents
                .values()
                .filter(|intent| intent.lifecycle.state.is_terminal())
                .count()
                > TERMINAL_INTENT_CAPACITY
        {
            return Err(IntentGraphError::MalformedPersistence);
        }
        let mut semantic_keys = BTreeSet::new();
        for (id, intent) in &self.intents {
            if id != &intent.id {
                return Err(IntentGraphError::MalformedPersistence);
            }
            validate_intent(intent)?;
            if !intent.lifecycle.state.is_terminal() && !semantic_keys.insert(intent.semantic_key())
            {
                return Err(IntentGraphError::DuplicateSemanticIntent);
            }
            for dependency in &intent.dependencies {
                let dependency_intent = self
                    .intents
                    .get(dependency)
                    .ok_or(IntentGraphError::MissingIntent)?;
                if !dependency_intent.dependents.contains(id) {
                    return Err(IntentGraphError::MalformedPersistence);
                }
            }
            for dependent in &intent.dependents {
                let dependent_intent = self
                    .intents
                    .get(dependent)
                    .ok_or(IntentGraphError::MissingIntent)?;
                if !dependent_intent.dependencies.contains(id) {
                    return Err(IntentGraphError::MalformedPersistence);
                }
            }
            for dependency in &intent.dependencies {
                if self.reaches(dependency, id) {
                    return Err(IntentGraphError::DependencyCycle);
                }
            }
        }
        Ok(())
    }
}

impl Default for IntentGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UncheckedIntentGraph {
    schema_version: u32,
    version: u64,
    intents: BTreeMap<IntentId, Intent>,
}

impl<'de> Deserialize<'de> for IntentGraph {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;
        let raw = UncheckedIntentGraph::deserialize(deserializer)?;
        if raw.schema_version != INTENT_GRAPH_SCHEMA_VERSION {
            return Err(D::Error::custom("unsupported intent-graph schema version"));
        }
        let graph = Self {
            schema_version: raw.schema_version,
            version: raw.version,
            intents: raw.intents,
        };
        graph.validate().map_err(D::Error::custom)?;
        Ok(graph)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentInsert {
    Inserted(IntentId),
    Merged(IntentId),
    DuplicateId(IntentId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentGraphError {
    InvalidSchema,
    MissingIntent,
    DependencyCycle,
    DuplicateSemanticIntent,
    MalformedPersistence,
    LiveCapacityReached,
    AuthorityDenied(AuthorityDenial),
    InvalidTransition(crate::planner_core::InvalidIntentTransition),
}

impl From<crate::planner_core::InvalidIntentTransition> for IntentGraphError {
    fn from(value: crate::planner_core::InvalidIntentTransition) -> Self {
        Self::InvalidTransition(value)
    }
}

impl fmt::Display for IntentGraphError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "intent graph error: {self:?}")
    }
}

impl std::error::Error for IntentGraphError {}

fn validate_intent(intent: &Intent) -> Result<(), IntentGraphError> {
    if intent.schema_version != INTENT_SCHEMA_VERSION
        || intent.dependencies.contains(&intent.id)
        || intent.dependents.contains(&intent.id)
        || intent.lifecycle.state.is_terminal() != intent.lifecycle.terminal_tick.is_some()
    {
        return Err(IntentGraphError::MalformedPersistence);
    }
    if let Some(objective) = &intent.spatial_objective {
        objective
            .validate()
            .map_err(|_| IntentGraphError::MalformedPersistence)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::beliefs::{BeliefKey, BeliefKind};
    use crate::{authority::*, officer_requests::*, officers::OfficerRole};

    const HOUR: u64 = 60;

    fn id(namespace: &str, value: &str) -> PlannerId {
        PlannerId::derive(namespace, [value])
    }

    fn leader(value: &str) -> AuthorityActor {
        AuthorityActor::Leader {
            cat_id: id("cat", value),
        }
    }

    fn evidence(occurrence: u32) -> EvidenceId {
        let key = BeliefKey::new(
            id("domain", "resources"),
            id("subject", "food"),
            BeliefKind::Stock,
        );
        let reporter = id("cat", "accountant");
        EvidenceId::derive("colony-1", &key, 10, &reporter, occurrence)
    }

    fn intent(occurrence: u32, target: &str) -> Intent {
        Intent::proposed(
            IntentId::derive("colony-1", 2, "build", target, occurrence),
            id("colony", "one"),
            leader("old-leader"),
            Some(id("cat", "old-leader")),
            AuthorityDomain::Building,
            id("intent_kind", "build"),
            id("target", target),
            id("rationale", "capacity"),
            10,
        )
    }

    fn request(occurrence: u32, target: &str, kind: RequestKind) -> OfficerRequest {
        let colony_id = id("colony", "one");
        let officer_id = id("cat", "steward");
        OfficerRequest::proposed(
            OfficerRequestId::derive(
                &colony_id,
                &officer_id,
                kind,
                &id("target", target),
                occurrence,
            ),
            colony_id,
            officer_id,
            OfficerRole::Steward,
            AuthorityDomain::Stewardship,
            AuthorityDomain::Building,
            kind,
            id("target", target),
            1,
            id("rationale", "needed"),
            100,
            HOUR,
        )
        .unwrap()
    }

    #[test]
    fn intent_and_request_schemas_are_versioned() {
        assert_eq!(INTENT_SCHEMA_VERSION, 1);
        assert_eq!(OFFICER_REQUEST_SCHEMA_VERSION, 1);
        assert_eq!(IntentGraph::new().schema_version, 1);
        assert_eq!(OfficerRequestBook::new().schema_version, 1);

        let value = serde_json::to_value(intent(0, "den")).unwrap();
        for field in [
            "schemaVersion",
            "id",
            "colonyId",
            "proposer",
            "leaderId",
            "adoptedFromLeaderId",
            "authorityDomain",
            "kindId",
            "targetId",
            "rationaleId",
            "evidenceIds",
            "reportIds",
            "beliefVersion",
            "creationTick",
            "reviewTick",
            "deadlineTick",
            "urgency",
            "strategicWeight",
            "confidence",
            "expectedBenefit",
            "expectedCost",
            "dependencies",
            "dependents",
            "spatialObjective",
            "resourceReservationIds",
            "deliveryReservationIds",
            "assignedCatIds",
            "taskIds",
            "lifecycle",
            "blockedReason",
            "temporaryPlayerBias",
            "standingOrderId",
        ] {
            assert!(
                value.get(field).is_some(),
                "missing persisted intent field {field}"
            );
        }

        let value = serde_json::to_value(request(0, "den", RequestKind::Building)).unwrap();
        for field in [
            "schemaVersion",
            "id",
            "colonyId",
            "officerId",
            "officerRole",
            "adoptedByOfficerId",
            "sourceDomain",
            "targetDomain",
            "kind",
            "targetId",
            "quantity",
            "baseUrgency",
            "rationaleId",
            "evidenceIds",
            "reportIds",
            "confidence",
            "estimatedResourceCost",
            "estimatedLaborTicks",
            "resourceReservationIds",
            "laborReservationIds",
            "dependencies",
            "creationTick",
            "expiryTick",
            "state",
            "terminalTick",
        ] {
            assert!(
                value.get(field).is_some(),
                "missing persisted officer-request field {field}"
            );
        }
    }

    #[test]
    fn request_lifetimes_and_aging_are_exact() {
        assert_eq!(RequestLifetime::Survival.game_hours(), 6);
        assert_eq!(RequestLifetime::Standard.game_hours(), 48);
        assert_eq!(RequestLifetime::Strategic.game_hours(), 7 * 24);
        assert_eq!(MAX_REQUEST_URGENCY_AGE_BASIS_POINTS, 2_500);

        for kind in [RequestKind::Survival, RequestKind::ActiveDefense] {
            assert_eq!(kind.lifetime(), RequestLifetime::Survival);
        }
        for kind in [
            RequestKind::Research,
            RequestKind::Building,
            RequestKind::Diplomacy,
            RequestKind::Trade,
        ] {
            assert_eq!(kind.lifetime(), RequestLifetime::Strategic);
        }
        let mut request = request(0, "den", RequestKind::Operational);
        request.base_urgency = BasisPoints::new(1_000);
        assert_eq!(request.expiry_tick, 100 + 48 * HOUR);
        assert_eq!(request.effective_urgency(100 + HOUR - 1, HOUR).get(), 1_000);
        assert_eq!(request.effective_urgency(100 + HOUR, HOUR).get(), 1_100);
        assert_eq!(
            request.effective_urgency(100 + 25 * HOUR, HOUR).get(),
            3_500
        );
        assert_eq!(request.effective_urgency(u64::MAX, HOUR).get(), 3_500);
    }

    #[test]
    fn authority_matrix_enforces_leader_officer_steward_scheduler_cat_and_god_boundaries() {
        let context = AuthorityContext {
            leader_present: true,
            player_authorized: true,
        };
        assert_eq!(
            decide_authority(
                &leader("leader"),
                AuthorityOperation::ApproveIntent,
                AuthorityDomain::ColonyWide,
                context,
            ),
            AuthorityDecision::Allowed
        );
        let forester = AuthorityActor::Officer {
            cat_id: id("cat", "forester"),
            role: OfficerRole::Forester,
        };
        assert_eq!(
            decide_authority(
                &forester,
                AuthorityOperation::ProposeIntent,
                AuthorityDomain::Forestry,
                context,
            ),
            AuthorityDecision::Allowed
        );
        assert_eq!(
            decide_authority(
                &forester,
                AuthorityOperation::SubmitOfficerRequest,
                AuthorityDomain::Farming,
                context,
            ),
            AuthorityDecision::Denied(AuthorityDenial::OutsideDomain)
        );
        assert_eq!(
            decide_authority(
                &forester,
                AuthorityOperation::ProposeIntent,
                AuthorityDomain::Farming,
                context,
            ),
            AuthorityDecision::Denied(AuthorityDenial::OutsideDomain)
        );
        let steward = AuthorityActor::ActingSteward {
            cat_id: id("cat", "steward"),
        };
        assert_eq!(
            decide_authority(
                &steward,
                AuthorityOperation::ApproveIntent,
                AuthorityDomain::Survival,
                context,
            ),
            AuthorityDecision::Denied(AuthorityDenial::LeaderStillPresent)
        );
        assert_eq!(
            decide_authority(
                &steward,
                AuthorityOperation::ApproveIntent,
                AuthorityDomain::Survival,
                AuthorityContext {
                    leader_present: false,
                    player_authorized: false,
                },
            ),
            AuthorityDecision::Allowed
        );
        assert_eq!(
            decide_authority(
                &steward,
                AuthorityOperation::ApproveIntent,
                AuthorityDomain::Research,
                AuthorityContext {
                    leader_present: false,
                    player_authorized: false,
                },
            ),
            AuthorityDecision::Denied(AuthorityDenial::OutsideDomain)
        );
        assert_eq!(
            decide_authority(
                &AuthorityActor::Scheduler,
                AuthorityOperation::ProposeIntent,
                AuthorityDomain::Survival,
                context,
            ),
            AuthorityDecision::Denied(AuthorityDenial::StrategyForbidden)
        );
        let cat = AuthorityActor::Cat {
            cat_id: id("cat", "worker"),
        };
        assert_eq!(
            decide_authority(
                &cat,
                AuthorityOperation::AcceptWork,
                AuthorityDomain::Building,
                context,
            ),
            AuthorityDecision::Allowed
        );
        assert_eq!(
            decide_authority(
                &cat,
                AuthorityOperation::RefuseWork,
                AuthorityDomain::Building,
                context,
            ),
            AuthorityDecision::Allowed
        );
        let god = AuthorityActor::God {
            player_id: id("player", "one"),
        };
        assert_eq!(
            decide_authority(
                &god,
                AuthorityOperation::PlayerNudge,
                AuthorityDomain::ColonyWide,
                context,
            ),
            AuthorityDecision::Allowed
        );
        assert_eq!(
            decide_authority(
                &god,
                AuthorityOperation::CancelIntent,
                AuthorityDomain::ColonyWide,
                context,
            ),
            AuthorityDecision::Denied(AuthorityDenial::LeaderRequired)
        );
    }

    #[test]
    fn intent_graph_deduplicates_merges_evidence_and_rejects_cycles() {
        let mut graph = IntentGraph::new();
        let mut first = intent(0, "den");
        first.evidence_ids.insert(evidence(0));
        first.urgency = BasisPoints::new(4_000);
        let first_id = first.id.clone();
        assert_eq!(
            graph.insert_or_merge(first).unwrap(),
            IntentInsert::Inserted(first_id.clone())
        );
        let mut equivalent = intent(1, "den");
        equivalent.evidence_ids.insert(evidence(1));
        equivalent.urgency = BasisPoints::new(7_000);
        assert_eq!(
            graph.insert_or_merge(equivalent).unwrap(),
            IntentInsert::Merged(first_id.clone())
        );
        assert_eq!(graph.iter().len(), 1);
        assert_eq!(graph.get(&first_id).unwrap().evidence_ids.len(), 2);
        assert_eq!(graph.get(&first_id).unwrap().urgency.get(), 7_000);

        let b = intent(2, "storage");
        let b_id = b.id.clone();
        let c = intent(3, "workshop");
        let c_id = c.id.clone();
        graph.insert_or_merge(b).unwrap();
        graph.insert_or_merge(c).unwrap();
        graph.add_dependency(&first_id, &b_id).unwrap();
        graph.add_dependency(&b_id, &c_id).unwrap();
        assert_eq!(
            graph.add_dependency(&c_id, &first_id),
            Err(IntentGraphError::DependencyCycle)
        );
    }

    #[test]
    fn cancellation_releases_claims_and_succession_preserves_identity_retry_and_attribution() {
        let mut graph = IntentGraph::new();
        let mut cancellable = intent(0, "den");
        cancellable
            .lifecycle
            .transition(IntentState::Approved, 11)
            .unwrap();
        cancellable
            .resource_reservation_ids
            .insert(id("reservation", "wood"));
        cancellable
            .delivery_reservation_ids
            .insert(id("reservation", "store"));
        cancellable.assigned_cat_ids.insert(id("cat", "builder"));
        cancellable.task_ids.insert(id("task", "build-den"));
        let cancel_id = cancellable.id.clone();
        graph.insert_or_merge(cancellable).unwrap();
        graph
            .cancel(
                &cancel_id,
                &leader("old-leader"),
                AuthorityContext {
                    leader_present: true,
                    player_authorized: false,
                },
                20,
                IntentReason::LeaderCancelled,
            )
            .unwrap();
        let cancelled = graph.get(&cancel_id).unwrap();
        assert_eq!(cancelled.lifecycle.state, IntentState::Cancelled);
        assert!(cancelled.resource_reservation_ids.is_empty());
        assert!(cancelled.delivery_reservation_ids.is_empty());
        assert!(cancelled.assigned_cat_ids.is_empty());
        assert!(cancelled.task_ids.is_empty());

        let mut continuing = intent(1, "storage");
        continuing
            .lifecycle
            .transition(IntentState::Approved, 11)
            .unwrap();
        continuing.lifecycle.retry_count = 3;
        let continuing_id = continuing.id.clone();
        let original_proposer = continuing.proposer.clone();
        graph.insert_or_merge(continuing).unwrap();
        let adopted =
            graph.adopt_for_successor(&id("cat", "old-leader"), id("cat", "new-leader"), 30);
        assert_eq!(adopted, vec![continuing_id.clone()]);
        let continuing = graph.get(&continuing_id).unwrap();
        assert_eq!(continuing.lifecycle.retry_count, 3);
        assert_eq!(continuing.proposer, original_proposer);
        assert_eq!(continuing.creation_tick, 10);
        assert_eq!(graph.iter().len(), 2);
    }

    #[test]
    fn live_intent_collection_stops_at_the_exact_bound() {
        let mut graph = IntentGraph::new();
        for occurrence in 0..LIVE_INTENT_CAPACITY as u32 {
            graph
                .insert_or_merge(intent(occurrence, &format!("target-{occurrence}")))
                .unwrap();
        }
        assert_eq!(graph.iter().len(), LIVE_INTENT_CAPACITY);
        assert_eq!(
            graph.insert_or_merge(intent(999, "overflow")),
            Err(IntentGraphError::LiveCapacityReached)
        );

        let mut terminal = IntentGraph::new();
        let first_id = intent(0, "terminal-0").id;
        for occurrence in 0..=TERMINAL_INTENT_CAPACITY as u32 {
            let mut completed = intent(occurrence, &format!("terminal-{occurrence}"));
            completed
                .lifecycle
                .transition(IntentState::Cancelled, occurrence as u64)
                .unwrap();
            terminal.insert_or_merge(completed).unwrap();
        }
        assert_eq!(terminal.iter().len(), TERMINAL_INTENT_CAPACITY);
        assert!(terminal.get(&first_id).is_none());
    }

    #[test]
    fn deadline_expiry_uses_normal_terminal_cleanup() {
        let mut graph = IntentGraph::new();
        let mut expiring = intent(0, "bridge");
        expiring.deadline_tick = Some(50);
        expiring
            .resource_reservation_ids
            .insert(id("reservation", "wood"));
        expiring.assigned_cat_ids.insert(id("cat", "builder"));
        expiring.task_ids.insert(id("task", "bridge"));
        let intent_id = expiring.id.clone();
        graph.insert_or_merge(expiring).unwrap();
        assert!(graph.expire_due(49).is_empty());
        assert_eq!(graph.expire_due(50), vec![intent_id.clone()]);
        let expired = graph.get(&intent_id).unwrap();
        assert_eq!(expired.lifecycle.state, IntentState::Failed);
        assert_eq!(expired.blocked_reason, Some(IntentReason::DeadlineExpired));
        assert!(expired.resource_reservation_ids.is_empty());
        assert!(expired.assigned_cat_ids.is_empty());
        assert!(expired.task_ids.is_empty());
    }

    #[test]
    fn officer_requests_dedupe_reject_cycles_enforce_budget_and_release_terminal_claims() {
        let mut book = OfficerRequestBook::new();
        let mut first = request(0, "den", RequestKind::Building);
        first.evidence_ids.insert(evidence(0));
        first.base_urgency = BasisPoints::new(5_000);
        first.estimated_resource_cost = 10;
        first.estimated_labor_ticks = 20;
        first
            .resource_reservation_ids
            .insert(id("reservation", "wood"));
        let first_id = first.id.clone();
        book.insert_or_merge(first).unwrap();
        let mut duplicate = request(1, "den", RequestKind::Building);
        duplicate.evidence_ids.insert(evidence(1));
        duplicate.base_urgency = BasisPoints::new(6_000);
        assert_eq!(
            book.insert_or_merge(duplicate).unwrap(),
            RequestInsert::Merged(first_id.clone())
        );
        assert_eq!(book.get(&first_id).unwrap().evidence_ids.len(), 2);

        let second = request(2, "storage", RequestKind::Operational);
        let second_id = second.id.clone();
        book.insert_or_merge(second).unwrap();
        book.add_dependency(&first_id, &second_id).unwrap();
        assert_eq!(
            book.add_dependency(&second_id, &first_id),
            Err(OfficerRequestError::DependencyCycle)
        );

        let version_before = book.version;
        assert_eq!(
            book.accept(
                &first_id,
                &leader("leader"),
                AuthorityContext {
                    leader_present: true,
                    player_authorized: false,
                },
                RequestBudget {
                    resource_limit: 9,
                    labor_tick_limit: 20,
                },
                110,
            ),
            Err(OfficerRequestError::BudgetExceeded)
        );
        assert_eq!(book.version, version_before);
        book.accept(
            &first_id,
            &leader("leader"),
            AuthorityContext {
                leader_present: true,
                player_authorized: false,
            },
            RequestBudget {
                resource_limit: 10,
                labor_tick_limit: 20,
            },
            110,
        )
        .unwrap();
        assert_eq!(
            book.get(&first_id)
                .unwrap()
                .effective_urgency(110 + 25 * HOUR, HOUR),
            book.get(&first_id).unwrap().base_urgency
        );
        book.fulfill(&first_id, &AuthorityActor::Scheduler, 120)
            .unwrap();
        assert!(
            book.get(&first_id)
                .unwrap()
                .resource_reservation_ids
                .is_empty()
        );
        assert_eq!(book.get(&first_id).unwrap().terminal_tick, Some(120));
    }

    #[test]
    fn request_expiry_and_succession_preserve_original_attribution() {
        let mut book = OfficerRequestBook::new();
        let survival = request(0, "water", RequestKind::Survival);
        assert_eq!(survival.expiry_tick, 100 + 6 * HOUR);
        let survival_id = survival.id.clone();
        book.insert_or_merge(survival).unwrap();
        assert!(book.expire_due(100 + 6 * HOUR - 1).is_empty());
        assert_eq!(book.expire_due(100 + 6 * HOUR), vec![survival_id]);

        let strategic = request(1, "shrine", RequestKind::Research);
        assert_eq!(strategic.expiry_tick, 100 + 7 * 24 * HOUR);
        let strategic_id = strategic.id.clone();
        let original_officer = strategic.officer_id.clone();
        book.insert_or_merge(strategic).unwrap();
        assert_eq!(
            book.adopt_for_successor(OfficerRole::Steward, id("cat", "new-steward"), 101),
            vec![strategic_id.clone()]
        );
        let adopted = book.get(&strategic_id).unwrap();
        assert_eq!(adopted.officer_id, original_officer);
        assert_eq!(
            adopted.adopted_by_officer_id,
            Some(id("cat", "new-steward"))
        );

        let overdue = request(2, "overdue", RequestKind::Operational);
        let overdue_id = overdue.id.clone();
        let overdue_expiry = overdue.expiry_tick;
        book.insert_or_merge(overdue).unwrap();
        assert!(
            book.adopt_for_successor(
                OfficerRole::Steward,
                id("cat", "third-steward"),
                overdue_expiry,
            )
            .iter()
            .all(|id| id != &overdue_id)
        );
        assert_eq!(
            book.get(&overdue_id).unwrap().state,
            OfficerRequestState::Expired
        );
    }

    #[test]
    fn request_states_and_collection_bounds_are_exact() {
        let context = AuthorityContext {
            leader_present: true,
            player_authorized: false,
        };
        let leader = leader("leader");
        let mut states = OfficerRequestBook::new();
        let mut invalid = request(0, "invalid", RequestKind::Operational);
        invalid
            .resource_reservation_ids
            .insert(id("reservation", "rejected-wood"));
        let invalid_id = invalid.id.clone();
        states.insert_or_merge(invalid).unwrap();
        let version_before = states.version;
        let unauthorized = AuthorityActor::Officer {
            cat_id: id("cat", "forester"),
            role: OfficerRole::Forester,
        };
        assert_eq!(
            states.reject(&invalid_id, &unauthorized, context, 101),
            Err(OfficerRequestError::AuthorityDenied(
                AuthorityDenial::OutsideDomain
            ))
        );
        assert_eq!(states.version, version_before);
        assert_eq!(
            states.fulfill(&invalid_id, &AuthorityActor::Scheduler, 101),
            Err(OfficerRequestError::InvalidTransition)
        );
        assert_eq!(states.version, version_before);
        states.reject(&invalid_id, &leader, context, 102).unwrap();
        assert_eq!(states.get(&invalid_id).unwrap().terminal_tick, Some(102));
        assert!(
            states
                .get(&invalid_id)
                .unwrap()
                .resource_reservation_ids
                .is_empty()
        );
        states.reject(&invalid_id, &leader, context, 999).unwrap();
        assert_eq!(states.get(&invalid_id).unwrap().terminal_tick, Some(102));

        let mut superseded = request(1, "superseded", RequestKind::Operational);
        superseded
            .labor_reservation_ids
            .insert(id("reservation", "builder"));
        let superseded_id = superseded.id.clone();
        states.insert_or_merge(superseded).unwrap();
        states
            .accept(
                &superseded_id,
                &leader,
                context,
                RequestBudget {
                    resource_limit: 0,
                    labor_tick_limit: 0,
                },
                103,
            )
            .unwrap();
        states
            .supersede(&superseded_id, &leader, context, 104)
            .unwrap();
        assert_eq!(states.get(&superseded_id).unwrap().terminal_tick, Some(104));
        assert!(
            states
                .get(&superseded_id)
                .unwrap()
                .labor_reservation_ids
                .is_empty()
        );

        let mut live = OfficerRequestBook::new();
        for occurrence in 0..LIVE_OFFICER_REQUEST_CAPACITY as u32 {
            live.insert_or_merge(request(
                occurrence,
                &format!("live-{occurrence}"),
                RequestKind::Operational,
            ))
            .unwrap();
        }
        assert_eq!(live.iter().len(), LIVE_OFFICER_REQUEST_CAPACITY);
        assert_eq!(
            live.insert_or_merge(request(999, "overflow", RequestKind::Operational)),
            Err(OfficerRequestError::LiveCapacityReached)
        );

        let mut terminal = OfficerRequestBook::new();
        let first_id = request(0, "terminal-0", RequestKind::Operational).id;
        for occurrence in 0..=TERMINAL_OFFICER_REQUEST_CAPACITY as u32 {
            let completed = request(
                occurrence,
                &format!("terminal-{occurrence}"),
                RequestKind::Operational,
            );
            let completed_id = completed.id.clone();
            terminal.insert_or_merge(completed).unwrap();
            terminal
                .reject(&completed_id, &leader, context, occurrence as u64)
                .unwrap();
        }
        assert_eq!(terminal.iter().len(), TERMINAL_OFFICER_REQUEST_CAPACITY);
        assert!(terminal.get(&first_id).is_none());
    }

    #[test]
    fn persisted_graphs_round_trip_and_reject_versions_cycles_and_semantic_duplicates() {
        let mut graph = IntentGraph::new();
        let a = intent(0, "a");
        let a_id = a.id.clone();
        let b = intent(1, "b");
        let b_id = b.id.clone();
        graph.insert_or_merge(a).unwrap();
        graph.insert_or_merge(b).unwrap();
        graph.add_dependency(&a_id, &b_id).unwrap();
        let json = serde_json::to_string(&graph).unwrap();
        let restored: IntentGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, graph);
        assert_eq!(serde_json::to_string(&restored).unwrap(), json);

        let mut wrong_version = serde_json::to_value(&graph).unwrap();
        wrong_version["schemaVersion"] = serde_json::json!(2);
        assert!(serde_json::from_value::<IntentGraph>(wrong_version).is_err());

        graph
            .intents
            .get_mut(&b_id)
            .unwrap()
            .dependencies
            .insert(a_id.clone());
        graph
            .intents
            .get_mut(&a_id)
            .unwrap()
            .dependents
            .insert(b_id);
        assert!(
            serde_json::from_str::<IntentGraph>(&serde_json::to_string(&graph).unwrap()).is_err()
        );

        let mut book = OfficerRequestBook::new();
        let first = request(0, "same", RequestKind::Building);
        let mut duplicate = request(1, "same", RequestKind::Building);
        book.insert_unchecked_for_test(first);
        duplicate.id = OfficerRequestId::derive(
            &duplicate.colony_id,
            &duplicate.officer_id,
            duplicate.kind,
            &duplicate.target_id,
            99,
        );
        book.insert_unchecked_for_test(duplicate);
        assert!(
            serde_json::from_str::<OfficerRequestBook>(&serde_json::to_string(&book).unwrap())
                .is_err()
        );

        let mut valid_book = OfficerRequestBook::new();
        valid_book
            .insert_or_merge(request(2, "valid", RequestKind::Operational))
            .unwrap();
        let mut wrong_book_version = serde_json::to_value(&valid_book).unwrap();
        wrong_book_version["schemaVersion"] = serde_json::json!(2);
        assert!(serde_json::from_value::<OfficerRequestBook>(wrong_book_version).is_err());

        let mut wrong_request_version = serde_json::to_value(&valid_book).unwrap();
        let stored_request = wrong_request_version["requests"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap();
        stored_request["schemaVersion"] = serde_json::json!(2);
        assert!(serde_json::from_value::<OfficerRequestBook>(wrong_request_version).is_err());

        let mut terminal_with_claim = OfficerRequestBook::new();
        let rejected = request(3, "rejected", RequestKind::Operational);
        let rejected_id = rejected.id.clone();
        terminal_with_claim.insert_or_merge(rejected).unwrap();
        terminal_with_claim
            .reject(
                &rejected_id,
                &leader("leader"),
                AuthorityContext {
                    leader_present: true,
                    player_authorized: false,
                },
                200,
            )
            .unwrap();
        let mut malformed_claim = serde_json::to_value(&terminal_with_claim).unwrap();
        let stored_request = malformed_claim["requests"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap();
        stored_request["resourceReservationIds"] = serde_json::json!([id("reservation", "orphan")]);
        assert!(serde_json::from_value::<OfficerRequestBook>(malformed_claim).is_err());
    }
}
