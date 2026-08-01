//! Dependency-safe server guards for the LAI.25 action envelope.
//!
//! The reusable guards stop at authorization and define interfaces for the
//! remaining ordered checks. The server binary supplies the live world,
//! receipt, persistence, and snapshot adapters.

use std::collections::BTreeMap;

use cat_protocol::{
    ActionAuthorityClass, ActionConflict, ActionDecodeError, ActionProtocolVersion,
    ActionReplayResult, AuthenticatedPlayerId, AuthorityDenialReason, CurrentStateHint,
    CurrentVersionHint, ExpectedStateVersions, LeaderAiActionEnvelope, LeaderAiActionPayload,
    LeaderAiActionResponse, LeaderAiActionResult, LeaderAiSnapshotEnvelope, PROTOCOL_VERSION,
    PlayerOnlyAction, RegenerationReportSnapshot, ReportSafeString, SelectedColonyId,
    StaleClientRefresh, UpdateRequiredCode,
};
use cat_sim::authority::{AuthorityDomain, officer_owns_domain};
use cat_sim::officers::OfficerRole;
use serde::{Deserialize, Serialize};

use crate::identity::{SignedSession, player_id_for_session, verify_session_at};

pub const MAX_LEADER_AI_ACTION_FRAME_BYTES: usize = 64 * 1_024;
pub const MAX_SERVER_IDEMPOTENCY_RECEIPTS: usize = 2_048;
pub const UPDATE_REQUIRED: UpdateRequiredCode = UpdateRequiredCode::UpdateRequired;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRequiredResponse {
    pub code: UpdateRequiredCode,
    pub minimum_supported_version: u32,
    pub current_protocol_version: u32,
}

impl UpdateRequiredResponse {
    #[must_use]
    pub const fn current() -> Self {
        Self {
            code: UPDATE_REQUIRED,
            minimum_supported_version: minimum_supported_action_protocol_version(),
            current_protocol_version: current_action_protocol_version(),
        }
    }
}

#[must_use]
pub const fn minimum_supported_action_protocol_version() -> u32 {
    PROTOCOL_VERSION
}

#[must_use]
pub const fn current_action_protocol_version() -> u32 {
    PROTOCOL_VERSION
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerActionConflict {
    UpdateRequired(UpdateRequiredResponse),
    Unauthenticated,
    Unauthorized,
    OwnershipDenied,
    AuthorityDenied(AuthorityDenialReason),
    VersionMismatch(Box<StaleClientRefresh>),
    DuplicateReplay(Box<ActionReplayResult>),
    PreconditionFailed(ReportSafeString),
    InsufficientFavor(Box<CurrentStateHint>),
    ReservationConflict(Box<CurrentStateHint>),
    RateLimited { retry_after_ms: u64 },
    MalformedActionId,
    UnknownActionVariant,
    MalformedPayload,
    OpaqueExistenceDenied,
    RejectLeaderBoostActivation,
    RejectOfficerBoostActivation,
    RejectOfficerOutOfDomainMutation,
}

impl ServerActionConflict {
    #[must_use]
    pub fn to_protocol_conflict(&self) -> ActionConflict {
        match self {
            Self::UpdateRequired(response) => ActionConflict::UpdateRequired {
                code: response.code,
                minimum_supported_version: response.minimum_supported_version,
                current_protocol_version: response.current_protocol_version,
            },
            Self::Unauthenticated | Self::Unauthorized => ActionConflict::Unauthorized,
            Self::OwnershipDenied | Self::OpaqueExistenceDenied => ActionConflict::OwnershipDenied,
            Self::AuthorityDenied(reason) => ActionConflict::AuthorityDenied {
                reason_class: *reason,
            },
            Self::VersionMismatch(refresh) => ActionConflict::VersionMismatch {
                current_version_hint: refresh.current_versions.clone(),
                current_state_hint: refresh.current_state_hint.clone(),
            },
            Self::DuplicateReplay(replay) => ActionConflict::DuplicateReplay {
                replay: replay.as_ref().clone(),
            },
            Self::PreconditionFailed(reason) => ActionConflict::PreconditionFailed {
                reason: reason.clone(),
            },
            Self::InsufficientFavor(current_state_hint) => ActionConflict::InsufficientFavor {
                current_state_hint: current_state_hint.as_ref().clone(),
            },
            Self::ReservationConflict(current_state_hint) => ActionConflict::ReservationConflict {
                current_state_hint: current_state_hint.as_ref().clone(),
            },
            Self::RateLimited { retry_after_ms } => ActionConflict::RateLimited {
                retry_after_ms: *retry_after_ms,
            },
            Self::MalformedActionId => ActionConflict::MalformedActionId,
            Self::UnknownActionVariant => ActionConflict::UnknownActionVariant,
            Self::MalformedPayload => ActionConflict::MalformedPayload,
            Self::RejectLeaderBoostActivation => ActionConflict::LeaderCannotActivateBoost,
            Self::RejectOfficerBoostActivation => ActionConflict::OfficerCannotActivateBoost,
            Self::RejectOfficerOutOfDomainMutation => ActionConflict::AuthorityDenied {
                reason_class: AuthorityDenialReason::OutsideOfficerDomain,
            },
        }
    }

    #[must_use]
    pub fn refresh_hint(&self) -> Option<StaleClientRefresh> {
        match self {
            Self::VersionMismatch(refresh) => Some(refresh.as_ref().clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerActionResult {
    UpdateRequired(UpdateRequiredResponse),
    ProtocolError(Box<ActionConflict>),
    Action(Box<LeaderAiActionResponse>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolCompatibleActionFrame {
    encoded: String,
}

impl ProtocolCompatibleActionFrame {
    #[must_use]
    pub fn encoded(&self) -> &str {
        &self.encoded
    }
}

pub fn check_protocol_compatibility(
    encoded: &str,
) -> Result<ProtocolCompatibleActionFrame, ServerActionConflict> {
    reject_before_action_decode(encoded)
}

pub fn reject_before_action_decode(
    encoded: &str,
) -> Result<ProtocolCompatibleActionFrame, ServerActionConflict> {
    if encoded.len() > MAX_LEADER_AI_ACTION_FRAME_BYTES {
        return Err(ServerActionConflict::MalformedPayload);
    }
    let value: serde_json::Value =
        serde_json::from_str(encoded).map_err(|_| ServerActionConflict::MalformedPayload)?;
    let version = value
        .as_object()
        .and_then(|object| object.get("protocolVersion"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .ok_or(ServerActionConflict::MalformedPayload)?;
    if version != PROTOCOL_VERSION {
        return Err(ServerActionConflict::UpdateRequired(
            UpdateRequiredResponse::current(),
        ));
    }
    Ok(ProtocolCompatibleActionFrame {
        encoded: encoded.to_owned(),
    })
}

pub fn decode_lai_action_envelope(
    frame: ProtocolCompatibleActionFrame,
) -> Result<LeaderAiActionEnvelope, ServerActionConflict> {
    LeaderAiActionEnvelope::decode_json(&frame.encoded).map_err(map_decode_error)
}

fn map_decode_error(error: ActionDecodeError) -> ServerActionConflict {
    match error {
        ActionDecodeError::UnsupportedProtocolVersion(_) => {
            ServerActionConflict::UpdateRequired(UpdateRequiredResponse::current())
        }
        ActionDecodeError::UnknownActionVariant => ServerActionConflict::UnknownActionVariant,
        ActionDecodeError::MalformedActionId => ServerActionConflict::MalformedActionId,
        ActionDecodeError::MalformedPayload | ActionDecodeError::InvalidBounds(_) => {
            ServerActionConflict::MalformedPayload
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedPlayerSession {
    player_id: AuthenticatedPlayerId,
    session_id: String,
}

impl VerifiedPlayerSession {
    #[must_use]
    pub fn player_id(&self) -> &AuthenticatedPlayerId {
        &self.player_id
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    #[must_use]
    pub fn rate_limit_key(&self) -> String {
        format!("s:{}", self.session_id)
    }
}

#[must_use]
pub fn constant_time_session_mac_check(session: &SignedSession, secret: &str, now_ms: i64) -> bool {
    verify_session_at(&session.session_id, Some(&session.sig), secret, now_ms)
}

pub fn check_hmac_session_authentication(
    session: &SignedSession,
    secret: &str,
    now_ms: i64,
    envelope: &LeaderAiActionEnvelope,
) -> Result<VerifiedPlayerSession, ServerActionConflict> {
    if !constant_time_session_mac_check(session, secret, now_ms)
        || player_id_for_session(&session.session_id) != session.player_id
        || envelope.player_id.as_str() != session.player_id
    {
        return Err(ServerActionConflict::Unauthenticated);
    }
    let player_id = AuthenticatedPlayerId::new(session.player_id.clone())
        .map_err(|_| ServerActionConflict::Unauthenticated)?;
    Ok(VerifiedPlayerSession {
        player_id,
        session_id: session.session_id.clone(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColonyControlPolicy {
    GlobalVillage,
    PlayerOwned { owner_player_id: String },
}

pub trait SelectedColonyOwnershipSource {
    fn control_policy(&self, colony_id: &str) -> Option<ColonyControlPolicy>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedColonyOwnershipDecision {
    OwnsSelectedColony,
    DenyForeignColonyMutation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedColonyOwnershipGuard {
    colony_id: SelectedColonyId,
}

impl SelectedColonyOwnershipGuard {
    #[must_use]
    pub fn colony_id(&self) -> &SelectedColonyId {
        &self.colony_id
    }
}

pub type OwnsSelectedColony = SelectedColonyOwnershipGuard;

pub fn check_selected_colony_ownership(
    source: &impl SelectedColonyOwnershipSource,
    session: &VerifiedPlayerSession,
    envelope: &LeaderAiActionEnvelope,
) -> Result<OwnsSelectedColony, ServerActionConflict> {
    let decision = match source.control_policy(envelope.colony_id.as_str()) {
        Some(ColonyControlPolicy::GlobalVillage) => {
            SelectedColonyOwnershipDecision::OwnsSelectedColony
        }
        Some(ColonyControlPolicy::PlayerOwned { owner_player_id })
            if owner_player_id == session.player_id.as_str() =>
        {
            SelectedColonyOwnershipDecision::OwnsSelectedColony
        }
        Some(ColonyControlPolicy::PlayerOwned { .. }) | None => {
            SelectedColonyOwnershipDecision::DenyForeignColonyMutation
        }
    };
    match decision {
        SelectedColonyOwnershipDecision::OwnsSelectedColony => Ok(SelectedColonyOwnershipGuard {
            colony_id: envelope.colony_id.clone(),
        }),
        SelectedColonyOwnershipDecision::DenyForeignColonyMutation => {
            Err(ServerActionConflict::OpaqueExistenceDenied)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerMutationActor<'a> {
    AuthenticatedPlayer(&'a VerifiedPlayerSession),
    Leader,
    Officer { role: OfficerRole },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorActionAuthorityClassification {
    AuthorizedPlayer,
    RejectMismatchedPlayerIdentity,
    RejectLeaderPlayerEnvelope,
    RejectLeaderBoostActivation,
    RejectOfficerPlayerEnvelope,
    RejectOfficerBoostActivation,
    RejectOfficerOutOfDomainMutation,
}

pub struct PlayerOnlyDivineBoostGuard;

impl PlayerOnlyDivineBoostGuard {
    #[must_use]
    pub const fn classify(
        actor: ServerMutationActor<'_>,
        payload: &LeaderAiActionPayload,
    ) -> Option<ActorActionAuthorityClassification> {
        if !matches!(
            payload.authority_class(),
            ActionAuthorityClass::PlayerOnly(PlayerOnlyAction::ActivateDivineBoost)
        ) {
            return None;
        }
        Some(match actor {
            ServerMutationActor::AuthenticatedPlayer(_) => {
                ActorActionAuthorityClassification::AuthorizedPlayer
            }
            ServerMutationActor::Leader => {
                ActorActionAuthorityClassification::RejectLeaderBoostActivation
            }
            ServerMutationActor::Officer { .. } => {
                ActorActionAuthorityClassification::RejectOfficerBoostActivation
            }
        })
    }
}

pub struct OfficerDomainAuthorityGuard;

impl OfficerDomainAuthorityGuard {
    #[must_use]
    pub fn owns(role: OfficerRole, domain: AuthorityDomain) -> bool {
        officer_owns_domain(role, domain)
    }
}

#[must_use]
pub fn classify_actor_action_authority(
    actor: ServerMutationActor<'_>,
    envelope: &LeaderAiActionEnvelope,
) -> ActorActionAuthorityClassification {
    if let Some(classification) = PlayerOnlyDivineBoostGuard::classify(actor, &envelope.payload) {
        if matches!(
            actor,
            ServerMutationActor::AuthenticatedPlayer(session)
                if session.player_id.as_str() != envelope.player_id.as_str()
        ) {
            return ActorActionAuthorityClassification::RejectMismatchedPlayerIdentity;
        }
        return classification;
    }
    match actor {
        ServerMutationActor::AuthenticatedPlayer(session)
            if session.player_id.as_str() == envelope.player_id.as_str() =>
        {
            ActorActionAuthorityClassification::AuthorizedPlayer
        }
        ServerMutationActor::AuthenticatedPlayer(_) => {
            ActorActionAuthorityClassification::RejectMismatchedPlayerIdentity
        }
        ServerMutationActor::Leader => {
            ActorActionAuthorityClassification::RejectLeaderPlayerEnvelope
        }
        ServerMutationActor::Officer { role } => {
            if OfficerDomainAuthorityGuard::owns(role, required_authority_domain(&envelope.payload))
            {
                ActorActionAuthorityClassification::RejectOfficerPlayerEnvelope
            } else {
                ActorActionAuthorityClassification::RejectOfficerOutOfDomainMutation
            }
        }
    }
}

pub fn check_actor_action_authority(
    actor: ServerMutationActor<'_>,
    envelope: &LeaderAiActionEnvelope,
) -> Result<(), ServerActionConflict> {
    match classify_actor_action_authority(actor, envelope) {
        ActorActionAuthorityClassification::AuthorizedPlayer => Ok(()),
        ActorActionAuthorityClassification::RejectLeaderBoostActivation => {
            Err(ServerActionConflict::RejectLeaderBoostActivation)
        }
        ActorActionAuthorityClassification::RejectOfficerBoostActivation => {
            Err(ServerActionConflict::RejectOfficerBoostActivation)
        }
        ActorActionAuthorityClassification::RejectOfficerOutOfDomainMutation => {
            Err(ServerActionConflict::RejectOfficerOutOfDomainMutation)
        }
        ActorActionAuthorityClassification::RejectLeaderPlayerEnvelope
        | ActorActionAuthorityClassification::RejectOfficerPlayerEnvelope
        | ActorActionAuthorityClassification::RejectMismatchedPlayerIdentity => {
            Err(ServerActionConflict::Unauthorized)
        }
    }
}

fn required_authority_domain(payload: &LeaderAiActionPayload) -> AuthorityDomain {
    match payload {
        LeaderAiActionPayload::PurchaseResearchWithFavor { .. }
        | LeaderAiActionPayload::PrepareScholarStudy { .. } => AuthorityDomain::Research,
        LeaderAiActionPayload::ChangeDiplomacy { .. }
        | LeaderAiActionPayload::ApproveAlliance { .. }
        | LeaderAiActionPayload::BlockColony { .. } => AuthorityDomain::Diplomacy,
        LeaderAiActionPayload::AcceptTradeContract { .. }
        | LeaderAiActionPayload::RejectTradeContract { .. } => AuthorityDomain::Trade,
        LeaderAiActionPayload::PhysicalPlacement { .. } => AuthorityDomain::Building,
        _ => AuthorityDomain::ColonyWide,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedMutation {
    envelope: LeaderAiActionEnvelope,
    verified_session: VerifiedPlayerSession,
    ownership: OwnsSelectedColony,
}

impl AuthorizedMutation {
    #[must_use]
    pub fn new(
        envelope: LeaderAiActionEnvelope,
        verified_session: VerifiedPlayerSession,
        ownership: OwnsSelectedColony,
    ) -> Self {
        Self {
            envelope,
            verified_session,
            ownership,
        }
    }

    #[must_use]
    pub fn envelope(&self) -> &LeaderAiActionEnvelope {
        &self.envelope
    }

    #[must_use]
    pub fn verified_session(&self) -> &VerifiedPlayerSession {
        &self.verified_session
    }

    #[must_use]
    pub fn ownership(&self) -> &OwnsSelectedColony {
        &self.ownership
    }
}

pub struct LeaderAiServerMutationPipeline;

impl LeaderAiServerMutationPipeline {
    pub fn validate_foundation(
        encoded: &str,
        session: &SignedSession,
        secret: &str,
        now_ms: i64,
        ownership_source: &impl SelectedColonyOwnershipSource,
    ) -> Result<AuthorizedMutation, ServerActionConflict> {
        let frame = check_protocol_compatibility(encoded)?;
        let envelope = decode_lai_action_envelope(frame)?;
        let verified_session =
            check_hmac_session_authentication(session, secret, now_ms, &envelope)?;
        let ownership =
            check_selected_colony_ownership(ownership_source, &verified_session, &envelope)?;
        check_actor_action_authority(
            ServerMutationActor::AuthenticatedPlayer(&verified_session),
            &envelope,
        )?;
        Ok(AuthorizedMutation {
            envelope,
            verified_session,
            ownership,
        })
    }

    pub fn execute_remaining(
        authorized: &AuthorizedMutation,
        executor: &mut impl OrderedMutationExecutor,
    ) -> Result<LeaderAiActionResponse, ServerActionConflict> {
        let expected_versions =
            ExpectedServerStateVersions::new(&authorized.envelope.expected_versions);
        executor.check_expected_state_versions(authorized, expected_versions)?;
        if let Some(replay) = executor.check_bounded_idempotent_replay(authorized)? {
            return Ok(replay);
        }
        executor.check_current_preconditions(authorized)?;
        executor.commit_atomic_favor_reservation_state(authorized)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ExpectedServerStateVersions<'a> {
    expected: &'a ExpectedStateVersions,
}

impl<'a> ExpectedServerStateVersions<'a> {
    #[must_use]
    pub const fn new(expected: &'a ExpectedStateVersions) -> Self {
        Self { expected }
    }

    #[must_use]
    pub const fn expected(&self) -> &'a ExpectedStateVersions {
        self.expected
    }
}

pub trait OrderedMutationExecutor {
    fn check_expected_state_versions(
        &mut self,
        authorized: &AuthorizedMutation,
        expected: ExpectedServerStateVersions<'_>,
    ) -> Result<(), ServerActionConflict>;

    fn check_bounded_idempotent_replay(
        &mut self,
        authorized: &AuthorizedMutation,
    ) -> Result<Option<LeaderAiActionResponse>, ServerActionConflict>;

    fn check_current_preconditions(
        &mut self,
        authorized: &AuthorizedMutation,
    ) -> Result<(), ServerActionConflict>;

    fn commit_atomic_favor_reservation_state(
        &mut self,
        authorized: &AuthorizedMutation,
    ) -> Result<LeaderAiActionResponse, ServerActionConflict>;
}

pub fn project_server_action_response(
    envelope: &LeaderAiActionEnvelope,
    conflict: &ServerActionConflict,
) -> ServerActionResult {
    if let ServerActionConflict::UpdateRequired(response) = conflict {
        return ServerActionResult::UpdateRequired(*response);
    }
    ServerActionResult::Action(Box::new(LeaderAiActionResponse {
        protocol_version: ActionProtocolVersion::current(),
        idempotency_id: envelope.idempotency_id.clone(),
        colony_id: envelope.colony_id.clone(),
        result: LeaderAiActionResult::Rejected {
            conflict: conflict.to_protocol_conflict(),
        },
        refresh: conflict.refresh_hint(),
    }))
}

pub type RefreshSnapshotHint = StaleClientRefresh;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ServerReceiptKey {
    colony_id: String,
    player_id: String,
    idempotency_id: String,
}

impl ServerReceiptKey {
    fn from_envelope(envelope: &LeaderAiActionEnvelope) -> Self {
        Self {
            colony_id: envelope.colony_id.as_str().to_owned(),
            player_id: envelope.player_id.as_str().to_owned(),
            idempotency_id: envelope.idempotency_id.as_str().to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredActionReceipt {
    request_fingerprint: String,
    response: LeaderAiActionResponse,
    sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyReplay {
    Missing,
    ReplayAcceptedPriorResult(LeaderAiActionResponse),
    ReplayRejectedPriorResult(LeaderAiActionResponse),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IdempotencyReceiptStore {
    receipts: BTreeMap<ServerReceiptKey, StoredActionReceipt>,
    next_sequence: u64,
}

impl IdempotencyReceiptStore {
    pub fn check_bounded_idempotent_replay(
        &self,
        envelope: &LeaderAiActionEnvelope,
    ) -> Result<IdempotencyReplay, ServerActionConflict> {
        let key = ServerReceiptKey::from_envelope(envelope);
        let Some(receipt) = self.receipts.get(&key) else {
            return Ok(IdempotencyReplay::Missing);
        };
        let fingerprint =
            serde_json::to_string(envelope).map_err(|_| ServerActionConflict::MalformedPayload)?;
        if receipt.request_fingerprint != fingerprint {
            return Err(ServerActionConflict::MalformedActionId);
        }
        let replay = replay_response(&receipt.response);
        if matches!(
            receipt.response.result,
            LeaderAiActionResult::Accepted { .. }
        ) {
            Ok(IdempotencyReplay::ReplayAcceptedPriorResult(replay))
        } else {
            Ok(IdempotencyReplay::ReplayRejectedPriorResult(replay))
        }
    }

    pub fn record(
        &mut self,
        envelope: &LeaderAiActionEnvelope,
        response: LeaderAiActionResponse,
    ) -> Result<(), ServerActionConflict> {
        let key = ServerReceiptKey::from_envelope(envelope);
        let request_fingerprint =
            serde_json::to_string(envelope).map_err(|_| ServerActionConflict::MalformedPayload)?;
        if let Some(existing) = self.receipts.get(&key) {
            return if existing.request_fingerprint == request_fingerprint {
                Ok(())
            } else {
                Err(ServerActionConflict::MalformedActionId)
            };
        }
        if self.receipts.len() == MAX_SERVER_IDEMPOTENCY_RECEIPTS
            && let Some(oldest) = self
                .receipts
                .iter()
                .min_by_key(|(_, receipt)| receipt.sequence)
                .map(|(key, _)| key.clone())
        {
            self.receipts.remove(&oldest);
        }
        let sequence = self.next_sequence;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(ServerActionConflict::MalformedPayload)?;
        self.receipts.insert(
            key,
            StoredActionReceipt {
                request_fingerprint,
                response,
                sequence,
            },
        );
        Ok(())
    }

    pub fn restore_serialized(
        &mut self,
        request_fingerprint: &str,
        response_json: &str,
    ) -> Result<(), ServerActionConflict> {
        let envelope =
            LeaderAiActionEnvelope::decode_json(request_fingerprint).map_err(map_decode_error)?;
        let response = serde_json::from_str::<LeaderAiActionResponse>(response_json)
            .map_err(|_| ServerActionConflict::MalformedPayload)?;
        if response.idempotency_id != envelope.idempotency_id
            || response.colony_id != envelope.colony_id
        {
            return Err(ServerActionConflict::MalformedPayload);
        }
        self.record(&envelope, response)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.receipts.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.receipts.is_empty()
    }
}

fn replay_response(response: &LeaderAiActionResponse) -> LeaderAiActionResponse {
    let replay = match &response.result {
        LeaderAiActionResult::Accepted { accepted } => ActionReplayResult {
            original_accepted: true,
            result_code: accepted.result_code.clone(),
            committed_versions: Some(accepted.committed_versions.clone()),
            current_state_hint: accepted.current_state_hint.clone(),
        },
        LeaderAiActionResult::Rejected { .. } | LeaderAiActionResult::DuplicateReplay { .. } => {
            ActionReplayResult {
                original_accepted: false,
                result_code: report_safe("action_rejected"),
                committed_versions: None,
                current_state_hint: None,
            }
        }
    };
    LeaderAiActionResponse {
        protocol_version: response.protocol_version,
        idempotency_id: response.idempotency_id.clone(),
        colony_id: response.colony_id.clone(),
        result: LeaderAiActionResult::DuplicateReplay { replay },
        refresh: response.refresh.clone(),
    }
}

pub fn check_expected_state_versions(
    expected: &ExpectedStateVersions,
    current: &CurrentVersionHint,
) -> Result<(), ServerActionConflict> {
    let required_match = current.planner_version == Some(expected.expected_planner_version)
        && current.domain_version == Some(expected.expected_domain_version)
        && current.resource_version == Some(expected.expected_resource_version);
    let optional_match =
        optional_version_matches(expected.expected_spatial_version, current.spatial_version)
            && optional_version_matches(
                expected.expected_reservation_version,
                current.reservation_version,
            )
            && optional_version_matches(
                expected.expected_research_version,
                current.research_version,
            )
            && optional_version_matches(expected.expected_scholar_version, current.scholar_version)
            && optional_version_matches(expected.expected_boost_version, current.boost_version)
            && optional_version_matches(
                expected.expected_diplomacy_version,
                current.diplomacy_version,
            )
            && optional_version_matches(expected.expected_trade_version, current.trade_version)
            && optional_version_matches(
                expected.expected_prosthetic_version,
                current.prosthetic_version,
            )
            && optional_version_matches(expected.expected_care_version, current.care_version)
            && optional_version_matches(expected.expected_officer_version, current.officer_version)
            && optional_version_matches(
                expected.expected_standing_order_version,
                current.standing_order_version,
            );
    if required_match && optional_match {
        Ok(())
    } else {
        Err(ServerActionConflict::VersionMismatch(Box::new(
            StaleClientRefresh {
                current_versions: current.clone(),
                current_state_hint: CurrentStateHint {
                    state_code: report_safe("stale_state"),
                    visible_entity_id: None,
                    visible_stage: None,
                },
            },
        )))
    }
}

fn optional_version_matches(expected: Option<u64>, current: Option<u64>) -> bool {
    match expected {
        Some(expected) => current == Some(expected),
        None => true,
    }
}

pub struct AtomicLeaderAiCommit<T> {
    candidate: T,
}

impl<T: Clone> AtomicLeaderAiCommit<T> {
    #[must_use]
    pub fn stage(current: &T) -> Self {
        Self {
            candidate: current.clone(),
        }
    }

    pub fn candidate_mut(&mut self) -> &mut T {
        &mut self.candidate
    }

    pub fn commit_favor_debit_once(self, current: &mut T) {
        self.commit_reservation_once(current);
    }

    fn commit_reservation_once(self, current: &mut T) {
        self.commit_runtime_state_once(current);
    }

    fn commit_runtime_state_once(self, current: &mut T) {
        *current = self.candidate;
    }
}

pub struct NoMutationBeforePreconditions;

pub struct ServerSideSnapshotRedactor;

impl ServerSideSnapshotRedactor {
    pub fn redact_snapshot_for_authenticated_colony(
        mut snapshot: LeaderAiSnapshotEnvelope,
        selected_colony_id: &str,
    ) -> Result<LeaderAiSnapshotEnvelope, ServerActionConflict> {
        redact_foreign_colony_private_beliefs(&mut snapshot, selected_colony_id);
        redact_private_plans(&mut snapshot, selected_colony_id);
        redact_hidden_stock(&mut snapshot);
        redact_regeneration_below_l4(&mut snapshot);
        redact_auth_material(&mut snapshot);
        snapshot
            .validate()
            .map_err(|_| ServerActionConflict::MalformedPayload)?;
        Ok(snapshot)
    }

    pub fn server_redaction_before_websocket_send(
        snapshot: LeaderAiSnapshotEnvelope,
        selected_colony_id: &str,
    ) -> Result<String, ServerActionConflict> {
        let redacted =
            Self::redact_snapshot_for_authenticated_colony(snapshot, selected_colony_id)?;
        serde_json::to_string(&redacted).map_err(|_| ServerActionConflict::MalformedPayload)
    }
}

fn redact_foreign_colony_private_beliefs(
    snapshot: &mut LeaderAiSnapshotEnvelope,
    selected_colony_id: &str,
) {
    snapshot
        .colonies
        .retain(|colony| colony.colony_id.as_str() == selected_colony_id);
}

fn redact_private_plans(snapshot: &mut LeaderAiSnapshotEnvelope, selected_colony_id: &str) {
    snapshot
        .colonies
        .retain(|colony| colony.colony_id.as_str() == selected_colony_id);
}

fn redact_hidden_stock(_snapshot: &mut LeaderAiSnapshotEnvelope) {
    // LAI.24 has no authoritative physical-stock field. Keeping this typed
    // boundary makes adding one a deliberate protocol review.
}

fn redact_regeneration_below_l4(snapshot: &mut LeaderAiSnapshotEnvelope) {
    for report in snapshot
        .colonies
        .iter_mut()
        .flat_map(|colony| &mut colony.reports)
    {
        if report.report_level < 4 {
            report.regeneration = RegenerationReportSnapshot::UnavailableBelowLevel4;
        }
    }
}

fn redact_auth_material(_snapshot: &mut LeaderAiSnapshotEnvelope) {
    // LAI.24 cannot represent a session, signature, peer address, or owner ID.
}

#[must_use]
pub const fn client_is_not_redaction_authority() -> bool {
    true
}

pub fn server_redaction_before_websocket_send(
    snapshot: LeaderAiSnapshotEnvelope,
    selected_colony_id: &str,
) -> Result<String, ServerActionConflict> {
    ServerSideSnapshotRedactor::server_redaction_before_websocket_send(snapshot, selected_colony_id)
}

fn report_safe(value: &str) -> ReportSafeString {
    ReportSafeString::new(value).expect("server report-safe literals are bounded and non-empty")
}
