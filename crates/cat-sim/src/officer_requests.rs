//! Persistent typed officer requests specified by
//! `docs/leader-ai-overhaul/planner-and-beliefs.md`.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Serialize};

use crate::{
    authority::{
        AuthorityActor, AuthorityContext, AuthorityDecision, AuthorityDenial, AuthorityDomain,
        AuthorityOperation, decide_authority, officer_owns_domain,
    },
    beliefs::{Confidence, EvidenceId, ReportId},
    content_manifest::ContentId,
    officers::OfficerRole,
    planner_core::{BasisPoints, PlannerId},
};

pub const OFFICER_REQUEST_SCHEMA_VERSION: u32 = 1;
pub const OFFICER_REQUEST_BOOK_SCHEMA_VERSION: u32 = 1;
pub const REQUEST_URGENCY_PER_FULL_GAME_HOUR: i64 = 100;
pub const MAX_REQUEST_URGENCY_AGE_BASIS_POINTS: i64 = 2_500;
pub const LIVE_OFFICER_REQUEST_CAPACITY: usize = 128;
pub const TERMINAL_OFFICER_REQUEST_CAPACITY: usize = 256;
pub const STRUCTURED_REQUEST_BUDGETS: [RequestBudget; 5] = [
    RequestBudget {
        resource_limit: 25,
        labor_tick_limit: 60,
    },
    RequestBudget {
        resource_limit: 50,
        labor_tick_limit: 180,
    },
    RequestBudget {
        resource_limit: 100,
        labor_tick_limit: 360,
    },
    RequestBudget {
        resource_limit: 200,
        labor_tick_limit: 720,
    },
    RequestBudget {
        resource_limit: 400,
        labor_tick_limit: 1_440,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestKind {
    Survival,
    ActiveDefense,
    Operational,
    Research,
    Building,
    Diplomacy,
    Trade,
}

impl RequestKind {
    #[must_use]
    pub const fn lifetime(self) -> RequestLifetime {
        match self {
            Self::Survival | Self::ActiveDefense => RequestLifetime::Survival,
            Self::Research | Self::Building | Self::Diplomacy | Self::Trade => {
                RequestLifetime::Strategic
            }
            Self::Operational => RequestLifetime::Standard,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestLifetime {
    Survival,
    Standard,
    Strategic,
}

impl RequestLifetime {
    #[must_use]
    pub const fn game_hours(self) -> u64 {
        match self {
            Self::Survival => 6,
            Self::Standard => 48,
            Self::Strategic => 7 * 24,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OfficerRequestId(PlannerId);

impl OfficerRequestId {
    #[must_use]
    pub fn derive(
        colony_id: &PlannerId,
        officer_id: &PlannerId,
        kind: RequestKind,
        target_id: &PlannerId,
        occurrence: u32,
    ) -> Self {
        Self(PlannerId::derive(
            "officer_request",
            [
                colony_id.as_str(),
                officer_id.as_str(),
                request_kind_id(kind),
                target_id.as_str(),
                occurrence.to_string().as_str(),
            ],
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

const fn request_kind_id(kind: RequestKind) -> &'static str {
    match kind {
        RequestKind::Survival => "survival",
        RequestKind::ActiveDefense => "active_defense",
        RequestKind::Operational => "operational",
        RequestKind::Research => "research",
        RequestKind::Building => "building",
        RequestKind::Diplomacy => "diplomacy",
        RequestKind::Trade => "trade",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfficerRequestState {
    Proposed,
    Accepted,
    Rejected,
    Fulfilled,
    Superseded,
    Expired,
}

impl OfficerRequestState {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Rejected | Self::Fulfilled | Self::Superseded | Self::Expired
        )
    }

    #[must_use]
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (
                Self::Proposed,
                Self::Accepted | Self::Rejected | Self::Superseded | Self::Expired
            ) | (Self::Accepted, Self::Fulfilled | Self::Superseded)
        ) || self == next
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestBudget {
    pub resource_limit: i64,
    pub labor_tick_limit: u64,
}

impl RequestBudget {
    #[must_use]
    pub const fn covers(self, resource_cost: i64, labor_ticks: u64) -> bool {
        resource_cost >= 0
            && resource_cost <= self.resource_limit
            && labor_ticks <= self.labor_tick_limit
    }
}

#[must_use]
pub const fn structured_request_budget(
    level: crate::officer_expertise::ExpertiseLevel,
) -> RequestBudget {
    STRUCTURED_REQUEST_BUDGETS[level as usize - 1]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficerRequestDraft {
    pub source_domain: AuthorityDomain,
    pub target_domain: AuthorityDomain,
    pub kind: RequestKind,
    pub target_id: PlannerId,
    pub quantity: u64,
    pub base_urgency: BasisPoints,
    pub rationale_id: PlannerId,
    pub evidence_ids: BTreeSet<EvidenceId>,
    pub report_ids: BTreeSet<ReportId>,
    pub confidence: Confidence,
    pub estimated_resource_cost: i64,
    pub estimated_labor_ticks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestedSpaceKind {
    HoleWorkArea,
    HuntingLair,
    AppleTree,
    FishShore,
    FarmPlot,
    Cookhouse,
    Workshop,
    Stockpile,
}

/// Cross-domain meaning that survives beyond a free-form target ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum OfficerRequestPayload {
    Target,
    Dependency {
        dependency_target_id: PlannerId,
    },
    Space {
        kind: RequestedSpaceKind,
        required_cells: u16,
    },
    Workshop {
        station_id: PlannerId,
        operation_id: PlannerId,
    },
    KeepStock {
        content_id: ContentId,
        minimum_units: u32,
        target_units: u32,
    },
}

impl OfficerRequestPayload {
    fn validate(&self) -> bool {
        match self {
            Self::Space { required_cells, .. } => *required_cells > 0,
            Self::KeepStock {
                minimum_units,
                target_units,
                ..
            } => *minimum_units > 0 && *target_units >= *minimum_units,
            Self::Target | Self::Dependency { .. } | Self::Workshop { .. } => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedOfficerRequestDraft {
    pub request: OfficerRequestDraft,
    pub payload: OfficerRequestPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RequestSemanticKey {
    colony_id: PlannerId,
    officer_role: OfficerRole,
    source_domain: AuthorityDomain,
    target_domain: AuthorityDomain,
    kind: RequestKind,
    target_id: PlannerId,
    quantity: u64,
    payload: OfficerRequestPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OfficerRequest {
    pub schema_version: u32,
    pub id: OfficerRequestId,
    pub colony_id: PlannerId,
    pub officer_id: PlannerId,
    pub officer_role: OfficerRole,
    pub adopted_by_officer_id: Option<PlannerId>,
    pub source_domain: AuthorityDomain,
    pub target_domain: AuthorityDomain,
    pub kind: RequestKind,
    pub payload: OfficerRequestPayload,
    pub target_id: PlannerId,
    pub quantity: u64,
    pub base_urgency: BasisPoints,
    pub rationale_id: PlannerId,
    pub evidence_ids: BTreeSet<EvidenceId>,
    pub report_ids: BTreeSet<ReportId>,
    pub confidence: Confidence,
    pub estimated_resource_cost: i64,
    pub estimated_labor_ticks: u64,
    pub resource_reservation_ids: BTreeSet<PlannerId>,
    pub labor_reservation_ids: BTreeSet<PlannerId>,
    pub dependencies: BTreeSet<OfficerRequestId>,
    pub creation_tick: u64,
    pub expiry_tick: u64,
    pub state: OfficerRequestState,
    pub terminal_tick: Option<u64>,
}

impl OfficerRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn proposed(
        id: OfficerRequestId,
        colony_id: PlannerId,
        officer_id: PlannerId,
        officer_role: OfficerRole,
        source_domain: AuthorityDomain,
        target_domain: AuthorityDomain,
        kind: RequestKind,
        target_id: PlannerId,
        quantity: u64,
        rationale_id: PlannerId,
        creation_tick: u64,
        ticks_per_game_hour: u64,
    ) -> Result<Self, OfficerRequestError> {
        if quantity == 0
            || ticks_per_game_hour == 0
            || !officer_owns_domain(officer_role, source_domain)
        {
            return Err(OfficerRequestError::MalformedPersistence);
        }
        let duration = kind
            .lifetime()
            .game_hours()
            .checked_mul(ticks_per_game_hour)
            .ok_or(OfficerRequestError::TickOverflow)?;
        let expiry_tick = creation_tick
            .checked_add(duration)
            .ok_or(OfficerRequestError::TickOverflow)?;
        Ok(Self {
            schema_version: OFFICER_REQUEST_SCHEMA_VERSION,
            id,
            colony_id,
            officer_id,
            officer_role,
            adopted_by_officer_id: None,
            source_domain,
            target_domain,
            kind,
            payload: OfficerRequestPayload::Target,
            target_id,
            quantity,
            base_urgency: BasisPoints::default(),
            rationale_id,
            evidence_ids: BTreeSet::new(),
            report_ids: BTreeSet::new(),
            confidence: Confidence::zero(),
            estimated_resource_cost: 0,
            estimated_labor_ticks: 0,
            resource_reservation_ids: BTreeSet::new(),
            labor_reservation_ids: BTreeSet::new(),
            dependencies: BTreeSet::new(),
            creation_tick,
            expiry_tick,
            state: OfficerRequestState::Proposed,
            terminal_tick: None,
        })
    }

    #[must_use]
    pub fn effective_urgency(&self, now_tick: u64, ticks_per_game_hour: u64) -> BasisPoints {
        if self.state != OfficerRequestState::Proposed || ticks_per_game_hour == 0 {
            return self.base_urgency;
        }
        let full_hours = now_tick.saturating_sub(self.creation_tick) / ticks_per_game_hour;
        let age = full_hours
            .saturating_mul(REQUEST_URGENCY_PER_FULL_GAME_HOUR as u64)
            .min(MAX_REQUEST_URGENCY_AGE_BASIS_POINTS as u64) as i64;
        BasisPoints::new(self.base_urgency.get().saturating_add(age))
    }

    fn transition(
        &mut self,
        next: OfficerRequestState,
        transition_tick: u64,
    ) -> Result<(), OfficerRequestError> {
        if self.state == next {
            return Ok(());
        }
        if !self.state.can_transition_to(next) {
            return Err(OfficerRequestError::InvalidTransition);
        }
        self.state = next;
        if next.is_terminal() {
            self.terminal_tick = Some(transition_tick);
            self.resource_reservation_ids.clear();
            self.labor_reservation_ids.clear();
        }
        Ok(())
    }

    fn semantic_key(&self) -> RequestSemanticKey {
        RequestSemanticKey {
            colony_id: self.colony_id.clone(),
            officer_role: self.officer_role,
            source_domain: self.source_domain,
            target_domain: self.target_domain,
            kind: self.kind,
            target_id: self.target_id.clone(),
            quantity: self.quantity,
            payload: self.payload.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficerRequestBook {
    pub schema_version: u32,
    pub version: u64,
    requests: BTreeMap<OfficerRequestId, OfficerRequest>,
}

impl OfficerRequestBook {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: OFFICER_REQUEST_BOOK_SCHEMA_VERSION,
            version: 0,
            requests: BTreeMap::new(),
        }
    }
    #[must_use]
    pub fn get(&self, id: &OfficerRequestId) -> Option<&OfficerRequest> {
        self.requests.get(id)
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&OfficerRequestId, &OfficerRequest)> {
        self.requests.iter()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn propose_structured(
        &mut self,
        actor: &AuthorityActor,
        context: AuthorityContext,
        colony_id: PlannerId,
        officer_id: PlannerId,
        officer_role: OfficerRole,
        draft: OfficerRequestDraft,
        budget: RequestBudget,
        creation_tick: u64,
        ticks_per_game_hour: u64,
    ) -> Result<RequestInsert, OfficerRequestError> {
        if actor
            != &(AuthorityActor::Officer {
                cat_id: officer_id.clone(),
                role: officer_role,
            })
        {
            return Err(OfficerRequestError::ActorMismatch);
        }
        if let AuthorityDecision::Denied(reason) = decide_authority(
            actor,
            AuthorityOperation::SubmitOfficerRequest,
            draft.source_domain,
            context,
        ) {
            return Err(OfficerRequestError::AuthorityDenied(reason));
        }
        if !budget.covers(draft.estimated_resource_cost, draft.estimated_labor_ticks) {
            return Err(OfficerRequestError::BudgetExceeded);
        }
        let occurrence =
            self.next_occurrence(&colony_id, &officer_id, draft.kind, &draft.target_id)?;
        let id = OfficerRequestId::derive(
            &colony_id,
            &officer_id,
            draft.kind,
            &draft.target_id,
            occurrence,
        );
        let mut request = OfficerRequest::proposed(
            id,
            colony_id,
            officer_id,
            officer_role,
            draft.source_domain,
            draft.target_domain,
            draft.kind,
            draft.target_id,
            draft.quantity,
            draft.rationale_id,
            creation_tick,
            ticks_per_game_hour,
        )?;
        request.base_urgency = draft.base_urgency;
        request.evidence_ids = draft.evidence_ids;
        request.report_ids = draft.report_ids;
        request.confidence = draft.confidence;
        request.estimated_resource_cost = draft.estimated_resource_cost;
        request.estimated_labor_ticks = draft.estimated_labor_ticks;
        self.insert_or_merge(request)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn propose_typed(
        &mut self,
        actor: &AuthorityActor,
        context: AuthorityContext,
        colony_id: PlannerId,
        officer_id: PlannerId,
        officer_role: OfficerRole,
        draft: TypedOfficerRequestDraft,
        budget: RequestBudget,
        creation_tick: u64,
        ticks_per_game_hour: u64,
    ) -> Result<RequestInsert, OfficerRequestError> {
        if !draft.payload.validate() {
            return Err(OfficerRequestError::MalformedPersistence);
        }
        let request_draft = draft.request;
        if actor
            != &(AuthorityActor::Officer {
                cat_id: officer_id.clone(),
                role: officer_role,
            })
        {
            return Err(OfficerRequestError::ActorMismatch);
        }
        if let AuthorityDecision::Denied(reason) = decide_authority(
            actor,
            AuthorityOperation::SubmitOfficerRequest,
            request_draft.source_domain,
            context,
        ) {
            return Err(OfficerRequestError::AuthorityDenied(reason));
        }
        if !budget.covers(
            request_draft.estimated_resource_cost,
            request_draft.estimated_labor_ticks,
        ) {
            return Err(OfficerRequestError::BudgetExceeded);
        }
        let occurrence = self.next_occurrence(
            &colony_id,
            &officer_id,
            request_draft.kind,
            &request_draft.target_id,
        )?;
        let id = OfficerRequestId::derive(
            &colony_id,
            &officer_id,
            request_draft.kind,
            &request_draft.target_id,
            occurrence,
        );
        let mut request = OfficerRequest::proposed(
            id,
            colony_id,
            officer_id,
            officer_role,
            request_draft.source_domain,
            request_draft.target_domain,
            request_draft.kind,
            request_draft.target_id,
            request_draft.quantity,
            request_draft.rationale_id,
            creation_tick,
            ticks_per_game_hour,
        )?;
        request.payload = draft.payload;
        request.base_urgency = request_draft.base_urgency;
        request.evidence_ids = request_draft.evidence_ids;
        request.report_ids = request_draft.report_ids;
        request.confidence = request_draft.confidence;
        request.estimated_resource_cost = request_draft.estimated_resource_cost;
        request.estimated_labor_ticks = request_draft.estimated_labor_ticks;
        self.insert_or_merge(request)
    }

    #[cfg(test)]
    pub(crate) fn insert_unchecked_for_test(&mut self, request: OfficerRequest) {
        self.requests.insert(request.id.clone(), request);
    }

    pub fn insert_or_merge(
        &mut self,
        mut request: OfficerRequest,
    ) -> Result<RequestInsert, OfficerRequestError> {
        validate_request(&request)?;
        if self.requests.contains_key(&request.id) {
            return Ok(RequestInsert::DuplicateId(request.id));
        }
        if let Some((id, existing)) = self.requests.iter_mut().find(|(_, existing)| {
            !existing.state.is_terminal() && existing.semantic_key() == request.semantic_key()
        }) {
            existing.evidence_ids.append(&mut request.evidence_ids);
            existing.report_ids.append(&mut request.report_ids);
            existing.base_urgency = existing.base_urgency.max(request.base_urgency);
            existing.confidence = existing.confidence.max(request.confidence);
            self.version = self.version.saturating_add(1);
            return Ok(RequestInsert::Merged(id.clone()));
        }
        if !request.state.is_terminal()
            && self
                .requests
                .values()
                .filter(|existing| !existing.state.is_terminal())
                .count()
                >= LIVE_OFFICER_REQUEST_CAPACITY
        {
            return Err(OfficerRequestError::LiveCapacityReached);
        }
        let id = request.id.clone();
        self.requests.insert(id.clone(), request);
        self.version = self.version.saturating_add(1);
        self.evict_terminal_history();
        Ok(RequestInsert::Inserted(id))
    }

    pub fn add_dependency(
        &mut self,
        request_id: &OfficerRequestId,
        dependency_id: &OfficerRequestId,
    ) -> Result<(), OfficerRequestError> {
        if request_id == dependency_id
            || !self.requests.contains_key(request_id)
            || !self.requests.contains_key(dependency_id)
        {
            return Err(if request_id == dependency_id {
                OfficerRequestError::DependencyCycle
            } else {
                OfficerRequestError::MissingRequest
            });
        }
        if self.reaches(dependency_id, request_id) {
            return Err(OfficerRequestError::DependencyCycle);
        }
        self.requests
            .get_mut(request_id)
            .expect("checked")
            .dependencies
            .insert(dependency_id.clone());
        self.version = self.version.saturating_add(1);
        Ok(())
    }

    pub fn accept(
        &mut self,
        id: &OfficerRequestId,
        actor: &AuthorityActor,
        context: AuthorityContext,
        budget: RequestBudget,
        now_tick: u64,
    ) -> Result<(), OfficerRequestError> {
        let request = self
            .requests
            .get_mut(id)
            .ok_or(OfficerRequestError::MissingRequest)?;
        if let AuthorityDecision::Denied(reason) = decide_authority(
            actor,
            AuthorityOperation::DecideOfficerRequest,
            request.target_domain,
            context,
        ) {
            return Err(OfficerRequestError::AuthorityDenied(reason));
        }
        if !budget.covers(
            request.estimated_resource_cost,
            request.estimated_labor_ticks,
        ) {
            return Err(OfficerRequestError::BudgetExceeded);
        }
        request.transition(OfficerRequestState::Accepted, now_tick)?;
        self.version = self.version.saturating_add(1);
        Ok(())
    }

    fn transition(
        &mut self,
        id: &OfficerRequestId,
        next: OfficerRequestState,
        now_tick: u64,
    ) -> Result<(), OfficerRequestError> {
        let request = self
            .requests
            .get_mut(id)
            .ok_or(OfficerRequestError::MissingRequest)?;
        request.transition(next, now_tick)?;
        self.version = self.version.saturating_add(1);
        self.evict_terminal_history();
        Ok(())
    }

    pub fn reject(
        &mut self,
        id: &OfficerRequestId,
        actor: &AuthorityActor,
        context: AuthorityContext,
        now_tick: u64,
    ) -> Result<(), OfficerRequestError> {
        self.authorized_transition(id, actor, context, OfficerRequestState::Rejected, now_tick)
    }

    pub fn supersede(
        &mut self,
        id: &OfficerRequestId,
        actor: &AuthorityActor,
        context: AuthorityContext,
        now_tick: u64,
    ) -> Result<(), OfficerRequestError> {
        self.authorized_transition(
            id,
            actor,
            context,
            OfficerRequestState::Superseded,
            now_tick,
        )
    }

    fn authorized_transition(
        &mut self,
        id: &OfficerRequestId,
        actor: &AuthorityActor,
        context: AuthorityContext,
        next: OfficerRequestState,
        now_tick: u64,
    ) -> Result<(), OfficerRequestError> {
        let request = self
            .requests
            .get(id)
            .ok_or(OfficerRequestError::MissingRequest)?;
        if let AuthorityDecision::Denied(reason) = decide_authority(
            actor,
            AuthorityOperation::DecideOfficerRequest,
            request.target_domain,
            context,
        ) {
            return Err(OfficerRequestError::AuthorityDenied(reason));
        }
        self.transition(id, next, now_tick)
    }

    pub fn fulfill(
        &mut self,
        id: &OfficerRequestId,
        actor: &AuthorityActor,
        now_tick: u64,
    ) -> Result<(), OfficerRequestError> {
        let request = self
            .requests
            .get(id)
            .ok_or(OfficerRequestError::MissingRequest)?;
        if let AuthorityDecision::Denied(reason) = decide_authority(
            actor,
            AuthorityOperation::ExecuteApprovedIntent,
            request.target_domain,
            AuthorityContext {
                leader_present: false,
                player_authorized: false,
            },
        ) {
            return Err(OfficerRequestError::AuthorityDenied(reason));
        }
        self.transition(id, OfficerRequestState::Fulfilled, now_tick)
    }

    pub fn expire_due(&mut self, now_tick: u64) -> Vec<OfficerRequestId> {
        let mut expired = Vec::new();
        for (id, request) in &mut self.requests {
            if request.state == OfficerRequestState::Proposed && now_tick >= request.expiry_tick {
                request.state = OfficerRequestState::Expired;
                request.terminal_tick = Some(now_tick);
                request.resource_reservation_ids.clear();
                request.labor_reservation_ids.clear();
                expired.push(id.clone());
            }
        }
        if !expired.is_empty() {
            self.version = self.version.saturating_add(1);
            self.evict_terminal_history();
        }
        expired
    }

    pub fn adopt_for_successor(
        &mut self,
        role: OfficerRole,
        successor_id: PlannerId,
        now_tick: u64,
    ) -> Vec<OfficerRequestId> {
        self.expire_due(now_tick);
        let mut adopted = Vec::new();
        for (id, request) in &mut self.requests {
            if request.officer_role == role && !request.state.is_terminal() {
                request.adopted_by_officer_id = Some(successor_id.clone());
                adopted.push(id.clone());
            }
        }
        if !adopted.is_empty() {
            self.version = self.version.saturating_add(1);
        }
        adopted
    }

    fn reaches(&self, start: &OfficerRequestId, target: &OfficerRequestId) -> bool {
        let mut pending = vec![start.clone()];
        let mut visited = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if &id == target {
                return true;
            }
            if visited.insert(id.clone())
                && let Some(request) = self.requests.get(&id)
            {
                pending.extend(request.dependencies.iter().rev().cloned());
            }
        }
        false
    }

    fn next_occurrence(
        &self,
        colony_id: &PlannerId,
        officer_id: &PlannerId,
        kind: RequestKind,
        target_id: &PlannerId,
    ) -> Result<u32, OfficerRequestError> {
        let count = self
            .requests
            .values()
            .filter(|request| {
                &request.colony_id == colony_id
                    && &request.officer_id == officer_id
                    && request.kind == kind
                    && &request.target_id == target_id
            })
            .count();
        u32::try_from(count).map_err(|_| OfficerRequestError::OccurrenceOverflow)
    }

    fn evict_terminal_history(&mut self) {
        while self
            .requests
            .values()
            .filter(|request| request.state.is_terminal())
            .count()
            > TERMINAL_OFFICER_REQUEST_CAPACITY
        {
            let Some(evicted_id) = self
                .requests
                .values()
                .filter_map(|request| request.terminal_tick.map(|tick| (tick, request.id.clone())))
                .min()
                .map(|(_, id)| id)
            else {
                break;
            };
            self.requests.remove(&evicted_id);
            for request in self.requests.values_mut() {
                request.dependencies.remove(&evicted_id);
            }
        }
    }

    fn validate(&self) -> Result<(), OfficerRequestError> {
        if self
            .requests
            .values()
            .filter(|request| !request.state.is_terminal())
            .count()
            > LIVE_OFFICER_REQUEST_CAPACITY
            || self
                .requests
                .values()
                .filter(|request| request.state.is_terminal())
                .count()
                > TERMINAL_OFFICER_REQUEST_CAPACITY
        {
            return Err(OfficerRequestError::MalformedPersistence);
        }
        let mut semantic = BTreeSet::new();
        for (id, request) in &self.requests {
            if id != &request.id {
                return Err(OfficerRequestError::MalformedPersistence);
            }
            validate_request(request)?;
            if !request.state.is_terminal() && !semantic.insert(request.semantic_key()) {
                return Err(OfficerRequestError::DuplicateSemanticRequest);
            }
            for dep in &request.dependencies {
                if !self.requests.contains_key(dep) {
                    return Err(OfficerRequestError::MissingRequest);
                }
            }
            for dependency in &request.dependencies {
                if self.reaches(dependency, id) {
                    return Err(OfficerRequestError::DependencyCycle);
                }
            }
        }
        Ok(())
    }
}

impl Default for OfficerRequestBook {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedRequestBook {
    schema_version: u32,
    version: u64,
    requests: BTreeMap<OfficerRequestId, OfficerRequest>,
}

impl<'de> Deserialize<'de> for OfficerRequestBook {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;
        let raw = UncheckedRequestBook::deserialize(deserializer)?;
        if raw.schema_version != OFFICER_REQUEST_BOOK_SCHEMA_VERSION {
            return Err(D::Error::custom(
                "unsupported officer-request-book schema version",
            ));
        }
        let book = Self {
            schema_version: raw.schema_version,
            version: raw.version,
            requests: raw.requests,
        };
        book.validate().map_err(D::Error::custom)?;
        Ok(book)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestInsert {
    Inserted(OfficerRequestId),
    Merged(OfficerRequestId),
    DuplicateId(OfficerRequestId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfficerRequestError {
    MissingRequest,
    DependencyCycle,
    DuplicateSemanticRequest,
    InvalidTransition,
    BudgetExceeded,
    ActorMismatch,
    TickOverflow,
    OccurrenceOverflow,
    MalformedPersistence,
    LiveCapacityReached,
    AuthorityDenied(AuthorityDenial),
}

impl fmt::Display for OfficerRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "officer request error: {self:?}")
    }
}
impl std::error::Error for OfficerRequestError {}

fn validate_request(request: &OfficerRequest) -> Result<(), OfficerRequestError> {
    if request.schema_version != OFFICER_REQUEST_SCHEMA_VERSION
        || request.quantity == 0
        || !request.payload.validate()
        || request.expiry_tick <= request.creation_tick
        || request.estimated_resource_cost < 0
        || request.dependencies.contains(&request.id)
        || !officer_owns_domain(request.officer_role, request.source_domain)
        || request.state.is_terminal() != request.terminal_tick.is_some()
        || (request.state.is_terminal()
            && (!request.resource_reservation_ids.is_empty()
                || !request.labor_reservation_ids.is_empty()))
    {
        return Err(OfficerRequestError::MalformedPersistence);
    }
    Ok(())
}
