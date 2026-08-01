//! LAI.65 canonical server-boundary contracts.
//!
//! This module deliberately has no Axum, SQLite, or simulation dependency.
//! The binary supplies HMAC/session verification and atomic database writes;
//! this boundary then binds the already-trusted session to the canonical v3
//! envelope, admits only broad God actions, and produces a typed dispatch for
//! the authoritative `cat-sim` adapters.

use std::collections::{BTreeMap, BTreeSet};

use cat_protocol::lai64::{
    ActionErrorSnapshot, ActionOutcome, ActionReceipt, CANONICAL_ACTION_SCHEMA_VERSION,
    CanonicalActionEnvelope, CanonicalColonySnapshot, CanonicalGodAction,
    CanonicalSnapshotEnvelope, CanonicalWireError, MAX_CANONICAL_ACTION_WIRE_BYTES,
    MAX_CANONICAL_ITEMS, NudgeDomain, PersonalStance, ReportText, StableId, VersionExpectation,
    VersionLane,
};
use serde::{Deserialize, Serialize};

/// Deliberately small, bounded state retained for one player/target Hole
/// bucket. The DTO accepts a 100ms request of up to 64 clicks, while the
/// authoritative rate limiter admits at most 20 physical clicks each second.
pub const HOLE_CLICK_LIMIT_PER_SECOND: usize = 20;
pub const HOLE_CLICK_WINDOW_MS: i64 = 1_000;
pub const MAX_CANONICAL_REPLAY_ROWS: usize = 2_048;
pub const MAX_TEST_RESET_CHALLENGES: usize = 64;
pub const TEST_RESET_CHALLENGE_TTL_MS: i64 = 60_000;
pub const CANONICAL_PERSISTENCE_ROW_SCHEMA_VERSION: u32 = 1;

/// An identity produced by the server's verified session layer. Do not build
/// this from the action JSON; the connection/session adapter is the only
/// legitimate constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedCanonicalSession {
    session_id: StableId,
    authenticated_player_id: StableId,
    expires_at_ms: i64,
}

impl TrustedCanonicalSession {
    pub fn new(
        session_id: StableId,
        authenticated_player_id: StableId,
        expires_at_ms: i64,
    ) -> Result<Self, CanonicalBoundaryError> {
        if expires_at_ms < 0 {
            return Err(CanonicalBoundaryError::InvalidTrustedSession);
        }
        Ok(Self {
            session_id,
            authenticated_player_id,
            expires_at_ms,
        })
    }

    #[must_use]
    pub const fn session_id(&self) -> &StableId {
        &self.session_id
    }

    #[must_use]
    pub const fn authenticated_player_id(&self) -> &StableId {
        &self.authenticated_player_id
    }

    #[must_use]
    pub const fn expires_at_ms(&self) -> i64 {
        self.expires_at_ms
    }

    fn validate_at(&self, now_ms: i64) -> Result<(), CanonicalBoundaryError> {
        if now_ms < 0 || now_ms > self.expires_at_ms {
            Err(CanonicalBoundaryError::Unauthenticated)
        } else {
            Ok(())
        }
    }
}

/// A selected colony is either the common world village or one personal
/// village. The boundary intentionally returns one opaque denial for a
/// missing and a foreign personal colony, so it never reveals ownership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalColonyAccess {
    GlobalVillage,
    PersonalVillage { owner_player_id: StableId },
}

pub trait CanonicalColonyDirectory {
    fn selected_colony_access(&self, colony_id: &StableId) -> Option<CanonicalColonyAccess>;
}

pub trait CanonicalVersionSource {
    fn current_version(&self, colony_id: &StableId, lane: VersionLane) -> Option<u64>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalServerBuild {
    Production,
    TestBuild,
}

/// A production server must never implement this trait with a permissive
/// verifier. In test builds it models a first signed challenge followed by a
/// separate, consuming confirmation step.
pub trait SignedTestResetGate {
    fn consume_second_confirmation(
        &mut self,
        session: &TrustedCanonicalSession,
        nonce: &StableId,
        signature: &ReportText,
        confirmation: &ReportText,
        now_ms: i64,
    ) -> Result<(), CanonicalBoundaryError>;

    /// The canonical confirmation always carries a selected colony.  Existing
    /// test gates which only model one unscoped challenge retain the original
    /// method, while the durable server gate overrides this to bind stage one
    /// and stage two to the same authorized colony.
    fn consume_second_confirmation_for_colony(
        &mut self,
        session: &TrustedCanonicalSession,
        selected_colony_id: &StableId,
        nonce: &StableId,
        signature: &ReportText,
        confirmation: &ReportText,
        now_ms: i64,
    ) -> Result<(), CanonicalBoundaryError> {
        let _ = selected_colony_id;
        self.consume_second_confirmation(session, nonce, signature, confirmation, now_ms)
    }
}

/// Signature verification belongs beside the server's test fixture signing
/// key. It is separated from the gate so no secret is copied into the action,
/// replay record, or SQLite row.
pub trait TestResetSignatureVerifier {
    fn verify_first_step(
        &self,
        session: &TrustedCanonicalSession,
        nonce: &StableId,
        signature: &ReportText,
    ) -> bool;
}

/// Public, secret-free payload for the test fixture signer. The server checks
/// the resulting HMAC with its session secret; the selected colony is bound in
/// the staged record and must match at confirmation time.
#[must_use]
pub fn test_reset_signature_message(session: &TrustedCanonicalSession, nonce: &StableId) -> String {
    format!(
        "cat-server:test-reset:v1:{}:{}:{}",
        session.session_id.as_str(),
        session.authenticated_player_id.as_str(),
        nonce.as_str()
    )
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ResetChallengeKey {
    session_id: StableId,
    nonce: StableId,
}

/// The durable identity of a staged reset challenge. It is deliberately only
/// a session/nonce pair; player, selected-colony, signature, and expiry remain
/// validated fields of the persisted challenge record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalResetChallengeKey {
    pub session_id: StableId,
    pub nonce: StableId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestResetStage {
    Staged,
    Replayed,
}

/// In-memory challenge ledger used by test-only fixture routes. A production
/// service may persist the same key with the later SQLite transaction, but it
/// must preserve the same one-use/session-scoped semantics.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TwoStepSignedTestResetGate {
    challenges: BTreeMap<ResetChallengeKey, StagedResetChallenge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StagedResetChallenge {
    authenticated_player_id: StableId,
    selected_colony_id: Option<StableId>,
    stage_idempotency_id: Option<StableId>,
    signature: ReportText,
    expires_at_ms: i64,
}

/// The persistence-safe contents of one verified, staged challenge. This has
/// no HMAC secret or bearer session signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedTestResetChallenge {
    pub session_id: StableId,
    pub authenticated_player_id: StableId,
    pub selected_colony_id: Option<StableId>,
    pub stage_idempotency_id: Option<StableId>,
    pub nonce: StableId,
    pub signature: ReportText,
    pub expires_at_ms: i64,
}

impl TwoStepSignedTestResetGate {
    pub fn stage_first_step(
        &mut self,
        session: &TrustedCanonicalSession,
        nonce: StableId,
        signature: ReportText,
        now_ms: i64,
        verifier: &impl TestResetSignatureVerifier,
    ) -> Result<(), CanonicalBoundaryError> {
        self.stage_first_step_inner(session, None, None, nonce, signature, now_ms, verifier)
            .map(|_| ())
    }

    pub fn stage_first_step_for_colony(
        &mut self,
        session: &TrustedCanonicalSession,
        selected_colony_id: StableId,
        idempotency_id: StableId,
        nonce: StableId,
        signature: ReportText,
        now_ms: i64,
        verifier: &impl TestResetSignatureVerifier,
    ) -> Result<TestResetStage, CanonicalBoundaryError> {
        self.stage_first_step_inner(
            session,
            Some(selected_colony_id),
            Some(idempotency_id),
            nonce,
            signature,
            now_ms,
            verifier,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn stage_first_step_inner(
        &mut self,
        session: &TrustedCanonicalSession,
        selected_colony_id: Option<StableId>,
        idempotency_id: Option<StableId>,
        nonce: StableId,
        signature: ReportText,
        now_ms: i64,
        verifier: &impl TestResetSignatureVerifier,
    ) -> Result<TestResetStage, CanonicalBoundaryError> {
        session.validate_at(now_ms)?;
        if !verifier.verify_first_step(session, &nonce, &signature) {
            return Err(CanonicalBoundaryError::ResetSignatureRejected);
        }
        self.prune(now_ms);
        if let Some(idempotency_id) = &idempotency_id
            && let Some((key, challenge)) = self.challenges.iter().find(|(key, challenge)| {
                key.session_id == session.session_id
                    && challenge.stage_idempotency_id.as_ref() == Some(idempotency_id)
            })
        {
            if key.nonce == nonce
                && challenge.authenticated_player_id == session.authenticated_player_id
                && challenge.selected_colony_id == selected_colony_id
                && challenge.signature == signature
            {
                return Ok(TestResetStage::Replayed);
            }
            return Err(CanonicalBoundaryError::ReplayConflict);
        }
        let key = ResetChallengeKey {
            session_id: session.session_id.clone(),
            nonce,
        };
        if self.challenges.contains_key(&key) {
            return Err(CanonicalBoundaryError::ResetAlreadyStaged);
        }
        if self.challenges.len() >= MAX_TEST_RESET_CHALLENGES {
            return Err(CanonicalBoundaryError::ResetGateAtCapacity);
        }
        let expires_at_ms = now_ms
            .checked_add(TEST_RESET_CHALLENGE_TTL_MS)
            .ok_or(CanonicalBoundaryError::InvalidTrustedSession)?;
        self.challenges.insert(
            key,
            StagedResetChallenge {
                authenticated_player_id: session.authenticated_player_id.clone(),
                selected_colony_id,
                stage_idempotency_id: idempotency_id,
                signature,
                expires_at_ms,
            },
        );
        Ok(TestResetStage::Staged)
    }

    pub fn prune(&mut self, now_ms: i64) {
        self.challenges
            .retain(|_, challenge| challenge.expires_at_ms >= now_ms);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.challenges.len()
    }

    pub fn staged_challenge(
        &self,
        session_id: &StableId,
        nonce: &StableId,
    ) -> Option<StagedTestResetChallenge> {
        let key = ResetChallengeKey {
            session_id: session_id.clone(),
            nonce: nonce.clone(),
        };
        self.challenges
            .get(&key)
            .map(|challenge| StagedTestResetChallenge {
                session_id: key.session_id,
                authenticated_player_id: challenge.authenticated_player_id.clone(),
                selected_colony_id: challenge.selected_colony_id.clone(),
                stage_idempotency_id: challenge.stage_idempotency_id.clone(),
                nonce: key.nonce,
                signature: challenge.signature.clone(),
                expires_at_ms: challenge.expires_at_ms,
            })
    }

    pub fn restore_staged_challenge(
        &mut self,
        challenge: StagedTestResetChallenge,
    ) -> Result<(), CanonicalBoundaryError> {
        if challenge.expires_at_ms < 0 {
            return Err(CanonicalBoundaryError::PersistenceCodec);
        }
        let key = ResetChallengeKey {
            session_id: challenge.session_id,
            nonce: challenge.nonce,
        };
        if self.challenges.contains_key(&key) || self.challenges.len() >= MAX_TEST_RESET_CHALLENGES
        {
            return Err(CanonicalBoundaryError::PersistenceCodec);
        }
        self.challenges.insert(
            key,
            StagedResetChallenge {
                authenticated_player_id: challenge.authenticated_player_id,
                selected_colony_id: challenge.selected_colony_id,
                stage_idempotency_id: challenge.stage_idempotency_id,
                signature: challenge.signature,
                expires_at_ms: challenge.expires_at_ms,
            },
        );
        Ok(())
    }

    fn consume_second_confirmation_inner(
        &mut self,
        session: &TrustedCanonicalSession,
        selected_colony_id: Option<&StableId>,
        nonce: &StableId,
        signature: &ReportText,
        confirmation: &ReportText,
        now_ms: i64,
    ) -> Result<(), CanonicalBoundaryError> {
        session.validate_at(now_ms)?;
        if confirmation.as_str() != "test_reset_confirmed" {
            return Err(CanonicalBoundaryError::ResetConfirmationRejected);
        }
        let key = ResetChallengeKey {
            session_id: session.session_id.clone(),
            nonce: nonce.clone(),
        };
        let Some(challenge) = self.challenges.get(&key) else {
            return Err(CanonicalBoundaryError::ResetNotStaged);
        };
        if challenge.expires_at_ms < now_ms {
            self.challenges.remove(&key);
            return Err(CanonicalBoundaryError::ResetChallengeExpired);
        }
        if challenge.authenticated_player_id != session.authenticated_player_id
            || challenge.signature != *signature
        {
            return Err(CanonicalBoundaryError::ResetSignatureRejected);
        }
        if let (Some(expected), Some(actual)) = (&challenge.selected_colony_id, selected_colony_id)
            && expected != actual
        {
            return Err(CanonicalBoundaryError::SelectedColonyDenied);
        }
        self.challenges.remove(&key);
        self.prune(now_ms);
        Ok(())
    }
}

impl SignedTestResetGate for TwoStepSignedTestResetGate {
    fn consume_second_confirmation(
        &mut self,
        session: &TrustedCanonicalSession,
        nonce: &StableId,
        signature: &ReportText,
        confirmation: &ReportText,
        now_ms: i64,
    ) -> Result<(), CanonicalBoundaryError> {
        self.consume_second_confirmation_inner(
            session,
            None,
            nonce,
            signature,
            confirmation,
            now_ms,
        )
    }

    fn consume_second_confirmation_for_colony(
        &mut self,
        session: &TrustedCanonicalSession,
        selected_colony_id: &StableId,
        nonce: &StableId,
        signature: &ReportText,
        confirmation: &ReportText,
        now_ms: i64,
    ) -> Result<(), CanonicalBoundaryError> {
        self.consume_second_confirmation_inner(
            session,
            Some(selected_colony_id),
            nonce,
            signature,
            confirmation,
            now_ms,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalBoundaryError {
    InvalidTrustedSession,
    Unauthenticated,
    PayloadPlayerMismatch,
    SelectedColonyDenied,
    VersionMismatch,
    Wire(CanonicalWireError),
    ReplayConflict,
    ReplayReceiptInvalid,
    ReplayStoreAtCapacity,
    RateLimited { retry_after_ms: u64 },
    SignedTestResetDisabled,
    ResetSignatureRejected,
    ResetConfirmationRejected,
    ResetNotStaged,
    ResetChallengeExpired,
    ResetAlreadyStaged,
    ResetGateAtCapacity,
    PersistenceCodec,
    SnapshotPartitionRejected,
}

impl From<CanonicalWireError> for CanonicalBoundaryError {
    fn from(value: CanonicalWireError) -> Self {
        Self::Wire(value)
    }
}

impl CanonicalBoundaryError {
    #[must_use]
    pub fn action_error(&self) -> ActionErrorSnapshot {
        let (code, reason, retry_after_ms) = match self {
            Self::InvalidTrustedSession | Self::Unauthenticated => {
                ("action:unauthenticated", "Session is not valid.", None)
            }
            Self::PayloadPlayerMismatch => (
                "action:identity_mismatch",
                "Action identity does not match the authenticated session.",
                None,
            ),
            Self::SelectedColonyDenied => (
                "action:selected_colony_denied",
                "That village cannot be controlled by this session.",
                None,
            ),
            Self::VersionMismatch => (
                "action:version_mismatch",
                "The report changed. Refresh before trying again.",
                None,
            ),
            Self::Wire(_) => (
                "action:invalid_request",
                "The action request is not valid.",
                None,
            ),
            Self::ReplayConflict => (
                "action:idempotency_conflict",
                "That action ID was already used for a different request.",
                None,
            ),
            Self::ReplayReceiptInvalid => (
                "action:receipt_invalid",
                "The stored action receipt is invalid.",
                None,
            ),
            Self::ReplayStoreAtCapacity => (
                "action:receipt_capacity",
                "The action receipt store is temporarily full.",
                None,
            ),
            Self::RateLimited { retry_after_ms } => (
                "action:rate_limited",
                "Hole clicks are limited to protect the shared world.",
                Some(*retry_after_ms),
            ),
            Self::SignedTestResetDisabled => (
                "action:test_reset_disabled",
                "Test reset is unavailable in this server build.",
                None,
            ),
            Self::ResetSignatureRejected
            | Self::ResetConfirmationRejected
            | Self::ResetNotStaged
            | Self::ResetAlreadyStaged
            | Self::ResetGateAtCapacity => (
                "action:test_reset_denied",
                "The signed test-reset confirmation was not accepted.",
                None,
            ),
            Self::ResetChallengeExpired => (
                "action:test_reset_expired",
                "The signed test-reset challenge has expired; request a new confirmation.",
                None,
            ),
            Self::PersistenceCodec => (
                "action:persistence_invalid",
                "Stored action state is not valid.",
                None,
            ),
            Self::SnapshotPartitionRejected => (
                "action:snapshot_partition",
                "The selected village report cannot be sent.",
                None,
            ),
        };
        ActionErrorSnapshot {
            code: stable(code),
            reason: report(reason),
            retry_after_ms,
            refresh_versions: Vec::new(),
        }
    }
}

/// Canonical-only dispatch. The later server/runtime adapter owns actual
/// mutation; it receives no direct worker, tile, route, stock, or appointment
/// command through this enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalGodDispatch {
    ResearchQueue {
        study_id: StableId,
    },
    ResearchReorder {
        study_id: StableId,
        before_study_id: Option<StableId>,
    },
    ResearchFund {
        study_id: StableId,
    },
    ResearchRemove {
        study_id: StableId,
    },
    ResearchPreparation {
        study_id: StableId,
    },
    FoodConservation {
        nudge_basis_points: i16,
    },
    HoleClickBatch {
        target_id: StableId,
        requested_clicks: u32,
        accepted_clicks: u32,
    },
    Inspiration,
    ActivateBoost {
        boost_id: StableId,
    },
    ConstructionMiracle {
        offer_id: StableId,
    },
    EmergencyRescue {
        witness_id: StableId,
    },
    CandidateBacking {
        election_id: StableId,
        candidate_id: StableId,
    },
    PersonalStance {
        other_colony_id: StableId,
        stance: PersonalStance,
    },
    Expel {
        subject_cat_id: StableId,
        household: bool,
    },
    BroadDomainNudge {
        domain: NudgeDomain,
        building_kind_id: Option<StableId>,
        basis_points: i16,
    },
    SignedTestReset {
        nonce: StableId,
    },
}

impl CanonicalGodDispatch {
    /// All canonical action shapes have an authority-backed runtime adapter.
    /// Each adapter must still revalidate current witnesses before mutation.
    #[must_use]
    pub const fn authority_gap(&self) -> Option<()> {
        None
    }
}

impl From<&CanonicalGodAction> for CanonicalGodDispatch {
    fn from(action: &CanonicalGodAction) -> Self {
        match action {
            CanonicalGodAction::ResearchQueue { study_id } => Self::ResearchQueue {
                study_id: study_id.clone(),
            },
            CanonicalGodAction::ResearchReorder {
                study_id,
                before_study_id,
            } => Self::ResearchReorder {
                study_id: study_id.clone(),
                before_study_id: before_study_id.clone(),
            },
            CanonicalGodAction::ResearchFund { study_id } => Self::ResearchFund {
                study_id: study_id.clone(),
            },
            CanonicalGodAction::ResearchRemove { study_id } => Self::ResearchRemove {
                study_id: study_id.clone(),
            },
            CanonicalGodAction::ResearchPreparation { study_id } => Self::ResearchPreparation {
                study_id: study_id.clone(),
            },
            CanonicalGodAction::FoodConservation { nudge_basis_points } => Self::FoodConservation {
                nudge_basis_points: *nudge_basis_points,
            },
            CanonicalGodAction::HoleClickBatch {
                target_id,
                requested_clicks,
                ..
            } => Self::HoleClickBatch {
                target_id: target_id.clone(),
                requested_clicks: *requested_clicks,
                accepted_clicks: *requested_clicks,
            },
            CanonicalGodAction::Inspiration => Self::Inspiration,
            CanonicalGodAction::ActivateBoost { boost_id } => Self::ActivateBoost {
                boost_id: boost_id.clone(),
            },
            CanonicalGodAction::ConstructionMiracle { offer_id } => Self::ConstructionMiracle {
                offer_id: offer_id.clone(),
            },
            CanonicalGodAction::EmergencyRescue { witness_id } => Self::EmergencyRescue {
                witness_id: witness_id.clone(),
            },
            CanonicalGodAction::CandidateBacking {
                election_id,
                candidate_id,
            } => Self::CandidateBacking {
                election_id: election_id.clone(),
                candidate_id: candidate_id.clone(),
            },
            CanonicalGodAction::PersonalStance {
                other_colony_id,
                stance,
            } => Self::PersonalStance {
                other_colony_id: other_colony_id.clone(),
                stance: *stance,
            },
            CanonicalGodAction::Expel {
                subject_cat_id,
                household,
            } => Self::Expel {
                subject_cat_id: subject_cat_id.clone(),
                household: *household,
            },
            CanonicalGodAction::BroadDomainNudge {
                domain,
                building_kind_id,
                basis_points,
            } => Self::BroadDomainNudge {
                domain: *domain,
                building_kind_id: building_kind_id.clone(),
                basis_points: *basis_points,
            },
            CanonicalGodAction::SignedTestReset { nonce, .. } => Self::SignedTestReset {
                nonce: nonce.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedCanonicalAction {
    trusted_session: TrustedCanonicalSession,
    envelope: CanonicalActionEnvelope,
    dispatch: CanonicalGodDispatch,
    request_fingerprint: String,
}

impl AuthorizedCanonicalAction {
    #[must_use]
    pub const fn trusted_session(&self) -> &TrustedCanonicalSession {
        &self.trusted_session
    }

    #[must_use]
    pub const fn envelope(&self) -> &CanonicalActionEnvelope {
        &self.envelope
    }

    #[must_use]
    pub const fn dispatch(&self) -> &CanonicalGodDispatch {
        &self.dispatch
    }

    #[must_use]
    pub fn request_fingerprint(&self) -> &str {
        &self.request_fingerprint
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalIngress {
    Replay(ActionReceipt),
    Dispatch(AuthorizedCanonicalAction),
}

/// Test-only stage-one companion for the protocol's `signed_test_reset`
/// confirmation action. It has the same canonical v3 header, authenticated
/// player, selected-colony, idempotency, and version fields, but it is not a
/// world-mutating God action and therefore remains server-boundary-only.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CanonicalTestResetRequest {
    pub protocol_version: u32,
    pub action_schema_version: u32,
    pub authenticated_player_id: StableId,
    pub selected_colony_id: StableId,
    pub idempotency_id: StableId,
    #[serde(default)]
    pub expected_versions: Vec<VersionExpectation>,
    pub payload: CanonicalTestResetRequestPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "action",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CanonicalTestResetRequestPayload {
    SignedTestResetRequest {
        nonce: StableId,
        signature: ReportText,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestResetRequestHeader {
    protocol_version: u32,
    action_schema_version: u32,
}

#[derive(Deserialize)]
struct TestResetRequestDiscriminator {
    payload: TestResetRequestActionDiscriminator,
}

#[derive(Deserialize)]
struct TestResetRequestActionDiscriminator {
    action: String,
}

impl CanonicalTestResetRequest {
    pub fn decode_json(encoded: &str) -> Result<Self, CanonicalWireError> {
        if encoded.len() > MAX_CANONICAL_ACTION_WIRE_BYTES {
            return Err(CanonicalWireError::InvalidBounds("action_wire_bytes"));
        }
        let header: TestResetRequestHeader =
            serde_json::from_str(encoded).map_err(|_| CanonicalWireError::MalformedHeader)?;
        if header.protocol_version != cat_protocol::PROTOCOL_VERSION {
            return Err(CanonicalWireError::UnsupportedProtocolVersion(
                header.protocol_version,
            ));
        }
        if header.action_schema_version != CANONICAL_ACTION_SCHEMA_VERSION {
            return Err(CanonicalWireError::UnsupportedSchemaVersion(
                header.action_schema_version,
            ));
        }
        let request: Self =
            serde_json::from_str(encoded).map_err(|_| CanonicalWireError::MalformedPayload)?;
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> Result<(), CanonicalWireError> {
        if self.protocol_version != cat_protocol::PROTOCOL_VERSION {
            return Err(CanonicalWireError::UnsupportedProtocolVersion(
                self.protocol_version,
            ));
        }
        if self.action_schema_version != CANONICAL_ACTION_SCHEMA_VERSION {
            return Err(CanonicalWireError::UnsupportedSchemaVersion(
                self.action_schema_version,
            ));
        }
        if !self.expected_versions.is_empty() {
            return Err(CanonicalWireError::InvalidBounds(
                "test_reset_expected_versions",
            ));
        }
        Ok(())
    }
}

#[must_use]
pub fn is_canonical_test_reset_request(encoded: &str) -> bool {
    serde_json::from_str::<TestResetRequestDiscriminator>(encoded)
        .is_ok_and(|request| request.payload.action == "signed_test_reset_request")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedCanonicalTestResetRequest {
    trusted_session: TrustedCanonicalSession,
    request: CanonicalTestResetRequest,
}

impl AuthorizedCanonicalTestResetRequest {
    #[must_use]
    pub const fn trusted_session(&self) -> &TrustedCanonicalSession {
        &self.trusted_session
    }

    #[must_use]
    pub const fn request(&self) -> &CanonicalTestResetRequest {
        &self.request
    }
}

pub fn authorize_canonical_test_reset_request(
    trusted_session: TrustedCanonicalSession,
    request: CanonicalTestResetRequest,
    directory: &impl CanonicalColonyDirectory,
    now_ms: i64,
) -> Result<AuthorizedCanonicalTestResetRequest, CanonicalBoundaryError> {
    trusted_session.validate_at(now_ms)?;
    request.validate()?;
    if request.authenticated_player_id != trusted_session.authenticated_player_id {
        return Err(CanonicalBoundaryError::PayloadPlayerMismatch);
    }
    authorize_selected_colony(
        directory,
        &trusted_session.authenticated_player_id,
        &request.selected_colony_id,
    )?;
    Ok(AuthorizedCanonicalTestResetRequest {
        trusted_session,
        request,
    })
}

/// Validate and authorize an already decoded canonical DTO without touching
/// state. This keeps HMAC/session verification and live world access outside
/// the DTO and guarantees the claimed payload player is never trusted alone.
pub fn authorize_canonical_action(
    trusted_session: TrustedCanonicalSession,
    envelope: CanonicalActionEnvelope,
    directory: &impl CanonicalColonyDirectory,
    now_ms: i64,
) -> Result<AuthorizedCanonicalAction, CanonicalBoundaryError> {
    trusted_session.validate_at(now_ms)?;
    envelope.validate()?;
    if envelope.authenticated_player_id != trusted_session.authenticated_player_id {
        return Err(CanonicalBoundaryError::PayloadPlayerMismatch);
    }
    let access = authorize_selected_colony(
        directory,
        &trusted_session.authenticated_player_id,
        &envelope.selected_colony_id,
    )?;
    if matches!(
        &envelope.payload,
        CanonicalGodAction::CandidateBacking { .. }
            | CanonicalGodAction::PersonalStance { .. }
            | CanonicalGodAction::Expel { .. }
    ) && !matches!(access, CanonicalColonyAccess::PersonalVillage { .. })
    {
        return Err(CanonicalBoundaryError::SelectedColonyDenied);
    }
    let request_fingerprint = canonical_action_fingerprint(&envelope)?;
    Ok(AuthorizedCanonicalAction {
        dispatch: CanonicalGodDispatch::from(&envelope.payload),
        trusted_session,
        envelope,
        request_fingerprint,
    })
}

/// Ordered state-admission boundary. Replays occur before version/rate/reset
/// checks, so a lost response never consumes another Hole click or fails due
/// to a state version advanced by its original execution.
pub fn admit_canonical_action(
    mut authorized: AuthorizedCanonicalAction,
    versions: &impl CanonicalVersionSource,
    build: CanonicalServerBuild,
    reset_gate: &mut dyn SignedTestResetGate,
    replay_store: &CanonicalReplayStore,
    click_limiter: &mut HoleClickRateLimiter,
    now_ms: i64,
) -> Result<CanonicalIngress, CanonicalBoundaryError> {
    if let Some(receipt) = replay_store.replay(&authorized)? {
        return Ok(CanonicalIngress::Replay(receipt));
    }
    ensure_expected_versions(&authorized.envelope, versions)?;
    if let CanonicalGodAction::SignedTestReset {
        nonce,
        signature,
        confirmation,
    } = &authorized.envelope.payload
    {
        if matches!(build, CanonicalServerBuild::Production) {
            return Err(CanonicalBoundaryError::SignedTestResetDisabled);
        }
        reset_gate.consume_second_confirmation_for_colony(
            &authorized.trusted_session,
            &authorized.envelope.selected_colony_id,
            nonce,
            signature,
            confirmation,
            now_ms,
        )?;
    }
    if let CanonicalGodAction::HoleClickBatch {
        target_id,
        requested_clicks,
        ..
    } = &authorized.envelope.payload
    {
        let accepted_clicks = click_limiter.admit(
            &authorized.trusted_session.authenticated_player_id,
            target_id,
            *requested_clicks,
            now_ms,
        )?;
        if let CanonicalGodDispatch::HoleClickBatch {
            accepted_clicks: dispatch_accepted,
            ..
        } = &mut authorized.dispatch
        {
            *dispatch_accepted = accepted_clicks;
        }
    }
    Ok(CanonicalIngress::Dispatch(authorized))
}

fn authorize_selected_colony(
    directory: &impl CanonicalColonyDirectory,
    player_id: &StableId,
    selected_colony_id: &StableId,
) -> Result<CanonicalColonyAccess, CanonicalBoundaryError> {
    match directory.selected_colony_access(selected_colony_id) {
        Some(CanonicalColonyAccess::GlobalVillage) => Ok(CanonicalColonyAccess::GlobalVillage),
        Some(CanonicalColonyAccess::PersonalVillage { owner_player_id })
            if &owner_player_id == player_id =>
        {
            Ok(CanonicalColonyAccess::PersonalVillage { owner_player_id })
        }
        _ => Err(CanonicalBoundaryError::SelectedColonyDenied),
    }
}

fn ensure_expected_versions(
    envelope: &CanonicalActionEnvelope,
    versions: &impl CanonicalVersionSource,
) -> Result<(), CanonicalBoundaryError> {
    if envelope.expected_versions.iter().any(|expected| {
        versions.current_version(&envelope.selected_colony_id, expected.lane)
            != Some(expected.expected_version)
    }) {
        Err(CanonicalBoundaryError::VersionMismatch)
    } else {
        Ok(())
    }
}

fn canonical_action_fingerprint(
    envelope: &CanonicalActionEnvelope,
) -> Result<String, CanonicalBoundaryError> {
    let encoded = serde_json::to_string(envelope)
        .map_err(|_| CanonicalBoundaryError::Wire(CanonicalWireError::MalformedPayload))?;
    if encoded.len() > MAX_CANONICAL_ACTION_WIRE_BYTES {
        Err(CanonicalBoundaryError::Wire(
            CanonicalWireError::InvalidBounds("action_fingerprint"),
        ))
    } else {
        Ok(encoded)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ReplayKey {
    authenticated_player_id: StableId,
    selected_colony_id: StableId,
    idempotency_id: StableId,
}

impl ReplayKey {
    fn from_authorized(action: &AuthorizedCanonicalAction) -> Self {
        Self {
            authenticated_player_id: action.trusted_session.authenticated_player_id.clone(),
            selected_colony_id: action.envelope.selected_colony_id.clone(),
            idempotency_id: action.envelope.idempotency_id.clone(),
        }
    }
}

/// Bounded, deterministic replay store. Persist its public row projection in
/// the same SQLite transaction as the domain receipt; do not let an action
/// commit without its replay record.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CanonicalReplayStore {
    rows: BTreeMap<ReplayKey, CanonicalReplayReceiptRow>,
    next_sequence: u64,
}

impl CanonicalReplayStore {
    pub fn replay(
        &self,
        action: &AuthorizedCanonicalAction,
    ) -> Result<Option<ActionReceipt>, CanonicalBoundaryError> {
        let key = ReplayKey::from_authorized(action);
        let Some(row) = self.rows.get(&key) else {
            return Ok(None);
        };
        row.validate()?;
        if row.request_fingerprint != action.request_fingerprint {
            return Err(CanonicalBoundaryError::ReplayConflict);
        }
        Ok(Some(ActionReceipt {
            idempotency_id: row.idempotency_id.clone(),
            selected_colony_id: row.selected_colony_id.clone(),
            outcome: ActionOutcome::Replayed,
            changed_ids: row.receipt.changed_ids.clone(),
            reason: row.receipt.reason.clone(),
            committed_versions: row.receipt.committed_versions.clone(),
        }))
    }

    pub fn record(
        &mut self,
        action: &AuthorizedCanonicalAction,
        receipt: ActionReceipt,
        committed_at_ms: i64,
    ) -> Result<CanonicalReplayReceiptRow, CanonicalBoundaryError> {
        validate_receipt_for_action(&receipt, action)?;
        let key = ReplayKey::from_authorized(action);
        if let Some(existing) = self.rows.get(&key) {
            return if existing.request_fingerprint == action.request_fingerprint {
                Ok(existing.clone())
            } else {
                Err(CanonicalBoundaryError::ReplayConflict)
            };
        }
        if self.rows.len() >= MAX_CANONICAL_REPLAY_ROWS {
            return Err(CanonicalBoundaryError::ReplayStoreAtCapacity);
        }
        let sequence = self.next_sequence;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(CanonicalBoundaryError::ReplayStoreAtCapacity)?;
        let row = CanonicalReplayReceiptRow {
            row_schema_version: CANONICAL_PERSISTENCE_ROW_SCHEMA_VERSION,
            authenticated_player_id: action.trusted_session.authenticated_player_id.clone(),
            selected_colony_id: action.envelope.selected_colony_id.clone(),
            idempotency_id: action.envelope.idempotency_id.clone(),
            request_fingerprint: action.request_fingerprint.clone(),
            receipt,
            committed_at_ms,
            sequence,
        };
        row.validate()?;
        self.rows.insert(key, row.clone());
        Ok(row)
    }

    pub fn restore(
        &mut self,
        row: CanonicalReplayReceiptRow,
    ) -> Result<(), CanonicalBoundaryError> {
        row.validate()?;
        let key = ReplayKey {
            authenticated_player_id: row.authenticated_player_id.clone(),
            selected_colony_id: row.selected_colony_id.clone(),
            idempotency_id: row.idempotency_id.clone(),
        };
        if self.rows.contains_key(&key) {
            return Err(CanonicalBoundaryError::ReplayConflict);
        }
        if self.rows.len() >= MAX_CANONICAL_REPLAY_ROWS {
            return Err(CanonicalBoundaryError::ReplayStoreAtCapacity);
        }
        self.next_sequence = self.next_sequence.max(
            row.sequence
                .checked_add(1)
                .ok_or(CanonicalBoundaryError::ReplayStoreAtCapacity)?,
        );
        self.rows.insert(key, row);
        Ok(())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }
}

fn validate_receipt_for_action(
    receipt: &ActionReceipt,
    action: &AuthorizedCanonicalAction,
) -> Result<(), CanonicalBoundaryError> {
    if receipt.idempotency_id != action.envelope.idempotency_id
        || receipt.selected_colony_id != action.envelope.selected_colony_id
        || matches!(receipt.outcome, ActionOutcome::Replayed)
    {
        return Err(CanonicalBoundaryError::ReplayReceiptInvalid);
    }
    validate_ordered_ids(&receipt.changed_ids)?;
    validate_versions(&receipt.committed_versions)?;
    validate_exact_version_lanes(
        &receipt.committed_versions,
        action.envelope.payload.required_lanes(),
    )?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct HoleClickRateKey {
    authenticated_player_id: StableId,
    target_id: StableId,
}

/// Click timestamps are retained per player and Hole target. They are not a
/// session limiter: reconnecting cannot bypass the player-target quota.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HoleClickRateLimiter {
    hits: BTreeMap<HoleClickRateKey, Vec<i64>>,
}

impl HoleClickRateLimiter {
    pub fn admit(
        &mut self,
        authenticated_player_id: &StableId,
        target_id: &StableId,
        requested_clicks: u32,
        now_ms: i64,
    ) -> Result<u32, CanonicalBoundaryError> {
        let requested =
            usize::try_from(requested_clicks).map_err(|_| CanonicalBoundaryError::RateLimited {
                retry_after_ms: 1_000,
            })?;
        if requested == 0 || now_ms < 0 {
            return Err(CanonicalBoundaryError::RateLimited {
                retry_after_ms: u64::try_from(HOLE_CLICK_WINDOW_MS).unwrap_or(1_000),
            });
        }
        let key = HoleClickRateKey {
            authenticated_player_id: authenticated_player_id.clone(),
            target_id: target_id.clone(),
        };
        let hits = self.hits.entry(key).or_default();
        hits.retain(|hit_ms| now_ms.saturating_sub(*hit_ms) < HOLE_CLICK_WINDOW_MS);
        let available = HOLE_CLICK_LIMIT_PER_SECOND.saturating_sub(hits.len());
        if available == 0 {
            let oldest = hits.first().copied().unwrap_or(now_ms);
            let retry_after_ms = HOLE_CLICK_WINDOW_MS
                .saturating_sub(now_ms.saturating_sub(oldest))
                .max(1);
            return Err(CanonicalBoundaryError::RateLimited {
                retry_after_ms: u64::try_from(retry_after_ms).unwrap_or(1),
            });
        }
        let accepted = requested.min(available);
        for _ in 0..accepted {
            hits.push(now_ms);
        }
        u32::try_from(accepted)
            .map_err(|_| CanonicalBoundaryError::RateLimited { retry_after_ms: 1 })
    }

    pub fn restore(
        &mut self,
        row: CanonicalHoleClickRateRow,
    ) -> Result<(), CanonicalBoundaryError> {
        row.validate()?;
        let key = HoleClickRateKey {
            authenticated_player_id: row.authenticated_player_id,
            target_id: row.target_id,
        };
        if self.hits.contains_key(&key) {
            return Err(CanonicalBoundaryError::PersistenceCodec);
        }
        self.hits.insert(key, row.hit_timestamps_ms);
        Ok(())
    }

    #[must_use]
    pub fn rows(&self) -> Vec<CanonicalHoleClickRateRow> {
        self.hits
            .iter()
            .map(|(key, hit_timestamps_ms)| CanonicalHoleClickRateRow {
                row_schema_version: CANONICAL_PERSISTENCE_ROW_SCHEMA_VERSION,
                authenticated_player_id: key.authenticated_player_id.clone(),
                target_id: key.target_id.clone(),
                hit_timestamps_ms: hit_timestamps_ms.clone(),
            })
            .collect()
    }
}

/// Validate a report projection immediately before serialization/send. The
/// protocol enforces exactly one detailed colony and requires it to be the
/// selected colony; this extra guard documents that the server must not bypass
/// the invariant with a hand-built JSON response.
pub fn validate_snapshot_before_send<'a>(
    snapshot: &'a CanonicalSnapshotEnvelope,
    trusted_session: &TrustedCanonicalSession,
    directory: &impl CanonicalColonyDirectory,
    now_ms: i64,
) -> Result<&'a CanonicalColonySnapshot, CanonicalBoundaryError> {
    trusted_session.validate_at(now_ms)?;
    snapshot.validate()?;
    if snapshot.colonies.len() != 1
        || snapshot.colonies.first().map_or(true, |colony| {
            colony.colony_id != snapshot.selected_colony_id
        })
    {
        return Err(CanonicalBoundaryError::SnapshotPartitionRejected);
    }
    authorize_selected_colony(
        directory,
        &trusted_session.authenticated_player_id,
        &snapshot.selected_colony_id,
    )?;
    snapshot
        .colonies
        .first()
        .ok_or(CanonicalBoundaryError::SnapshotPartitionRejected)
}

/// A strict, JSON-ready replay row for future SQLite persistence. There are no
/// server secrets or hidden domain balances in this record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CanonicalReplayReceiptRow {
    pub row_schema_version: u32,
    pub authenticated_player_id: StableId,
    pub selected_colony_id: StableId,
    pub idempotency_id: StableId,
    pub request_fingerprint: String,
    pub receipt: ActionReceipt,
    pub committed_at_ms: i64,
    pub sequence: u64,
}

impl CanonicalReplayReceiptRow {
    pub fn validate(&self) -> Result<(), CanonicalBoundaryError> {
        if self.row_schema_version != CANONICAL_PERSISTENCE_ROW_SCHEMA_VERSION
            || self.request_fingerprint.is_empty()
            || self.request_fingerprint.len() > MAX_CANONICAL_ACTION_WIRE_BYTES
            || self.committed_at_ms < 0
            || self.receipt.idempotency_id != self.idempotency_id
            || self.receipt.selected_colony_id != self.selected_colony_id
            || matches!(self.receipt.outcome, ActionOutcome::Replayed)
        {
            return Err(CanonicalBoundaryError::PersistenceCodec);
        }
        let envelope = CanonicalActionEnvelope::decode_json(&self.request_fingerprint)
            .map_err(|_| CanonicalBoundaryError::PersistenceCodec)?;
        if envelope.authenticated_player_id != self.authenticated_player_id
            || envelope.selected_colony_id != self.selected_colony_id
            || envelope.idempotency_id != self.idempotency_id
        {
            return Err(CanonicalBoundaryError::PersistenceCodec);
        }
        validate_ordered_ids(&self.receipt.changed_ids)?;
        validate_versions(&self.receipt.committed_versions)?;
        validate_exact_version_lanes(
            &self.receipt.committed_versions,
            envelope.payload.required_lanes(),
        )
    }

    pub fn encode_json(&self) -> Result<String, CanonicalBoundaryError> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| CanonicalBoundaryError::PersistenceCodec)
    }

    pub fn decode_json(encoded: &str) -> Result<Self, CanonicalBoundaryError> {
        if encoded.len() > MAX_CANONICAL_ACTION_WIRE_BYTES {
            return Err(CanonicalBoundaryError::PersistenceCodec);
        }
        let row = serde_json::from_str::<Self>(encoded)
            .map_err(|_| CanonicalBoundaryError::PersistenceCodec)?;
        row.validate()?;
        Ok(row)
    }
}

/// Persisted rate state; a row contains at most the current 20 one-second
/// hits and remains suitable for an atomic upsert with its action receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CanonicalHoleClickRateRow {
    pub row_schema_version: u32,
    pub authenticated_player_id: StableId,
    pub target_id: StableId,
    #[serde(default)]
    pub hit_timestamps_ms: Vec<i64>,
}

impl CanonicalHoleClickRateRow {
    pub fn validate(&self) -> Result<(), CanonicalBoundaryError> {
        if self.row_schema_version != CANONICAL_PERSISTENCE_ROW_SCHEMA_VERSION
            || self.hit_timestamps_ms.len() > HOLE_CLICK_LIMIT_PER_SECOND
            || self
                .hit_timestamps_ms
                .iter()
                .any(|timestamp| *timestamp < 0)
            || self
                .hit_timestamps_ms
                .windows(2)
                .any(|pair| pair[0] > pair[1])
        {
            return Err(CanonicalBoundaryError::PersistenceCodec);
        }
        Ok(())
    }

    pub fn encode_json(&self) -> Result<String, CanonicalBoundaryError> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| CanonicalBoundaryError::PersistenceCodec)
    }

    pub fn decode_json(encoded: &str) -> Result<Self, CanonicalBoundaryError> {
        if encoded.len() > MAX_CANONICAL_ACTION_WIRE_BYTES {
            return Err(CanonicalBoundaryError::PersistenceCodec);
        }
        let row = serde_json::from_str::<Self>(encoded)
            .map_err(|_| CanonicalBoundaryError::PersistenceCodec)?;
        row.validate()?;
        Ok(row)
    }
}

/// Strict session-state row. It contains the already-derived player identity
/// and expiry, never a bearer signature or HMAC secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CanonicalSessionRow {
    pub row_schema_version: u32,
    pub session_id: StableId,
    pub authenticated_player_id: StableId,
    pub expires_at_ms: i64,
    pub revoked: bool,
}

impl CanonicalSessionRow {
    #[must_use]
    pub fn from_trusted(session: &TrustedCanonicalSession) -> Self {
        Self {
            row_schema_version: CANONICAL_PERSISTENCE_ROW_SCHEMA_VERSION,
            session_id: session.session_id.clone(),
            authenticated_player_id: session.authenticated_player_id.clone(),
            expires_at_ms: session.expires_at_ms,
            revoked: false,
        }
    }

    pub fn into_trusted(
        self,
        now_ms: i64,
    ) -> Result<TrustedCanonicalSession, CanonicalBoundaryError> {
        if self.row_schema_version != CANONICAL_PERSISTENCE_ROW_SCHEMA_VERSION || self.revoked {
            return Err(CanonicalBoundaryError::Unauthenticated);
        }
        let trusted = TrustedCanonicalSession::new(
            self.session_id,
            self.authenticated_player_id,
            self.expires_at_ms,
        )?;
        trusted.validate_at(now_ms)?;
        Ok(trusted)
    }

    pub fn encode_json(&self) -> Result<String, CanonicalBoundaryError> {
        if self.row_schema_version != CANONICAL_PERSISTENCE_ROW_SCHEMA_VERSION
            || self.expires_at_ms < 0
        {
            return Err(CanonicalBoundaryError::PersistenceCodec);
        }
        serde_json::to_string(self).map_err(|_| CanonicalBoundaryError::PersistenceCodec)
    }

    pub fn decode_json(encoded: &str) -> Result<Self, CanonicalBoundaryError> {
        if encoded.len() > MAX_CANONICAL_ACTION_WIRE_BYTES {
            return Err(CanonicalBoundaryError::PersistenceCodec);
        }
        let row = serde_json::from_str::<Self>(encoded)
            .map_err(|_| CanonicalBoundaryError::PersistenceCodec)?;
        if row.row_schema_version != CANONICAL_PERSISTENCE_ROW_SCHEMA_VERSION
            || row.expires_at_ms < 0
        {
            return Err(CanonicalBoundaryError::PersistenceCodec);
        }
        Ok(row)
    }
}

/// A future SQLite adapter must write these records atomically with the
/// authoritative domain aggregate. The shape is intentionally explicit so a
/// partial receipt/rate/session write cannot be mistaken for a committed action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalAtomicPersistenceBatch {
    pub replay_row: CanonicalReplayReceiptRow,
    pub rate_rows: Vec<CanonicalHoleClickRateRow>,
    pub session_row: CanonicalSessionRow,
    /// A successful reset confirmation consumes its durable challenge in the
    /// same transaction as the fresh selected-colony aggregate and replay
    /// receipt. Ordinary actions leave this empty.
    pub consumed_reset_challenge: Option<CanonicalResetChallengeKey>,
}

impl CanonicalAtomicPersistenceBatch {
    pub fn validate(&self) -> Result<(), CanonicalBoundaryError> {
        self.replay_row.validate()?;
        self.session_row.encode_json()?;
        if self.replay_row.authenticated_player_id != self.session_row.authenticated_player_id {
            return Err(CanonicalBoundaryError::PersistenceCodec);
        }
        if self.rate_rows.len() > MAX_CANONICAL_ITEMS {
            return Err(CanonicalBoundaryError::PersistenceCodec);
        }
        let mut keys = BTreeSet::new();
        for row in &self.rate_rows {
            row.validate()?;
            if !keys.insert((row.authenticated_player_id.clone(), row.target_id.clone())) {
                return Err(CanonicalBoundaryError::PersistenceCodec);
            }
        }
        Ok(())
    }
}

fn validate_ordered_ids(ids: &[StableId]) -> Result<(), CanonicalBoundaryError> {
    if ids.len() > MAX_CANONICAL_ITEMS || ids.windows(2).any(|pair| pair[0] >= pair[1]) {
        Err(CanonicalBoundaryError::ReplayReceiptInvalid)
    } else {
        Ok(())
    }
}

fn validate_versions(versions: &[VersionExpectation]) -> Result<(), CanonicalBoundaryError> {
    if versions.len() > 13 || versions.windows(2).any(|pair| pair[0].lane >= pair[1].lane) {
        Err(CanonicalBoundaryError::ReplayReceiptInvalid)
    } else {
        Ok(())
    }
}

fn validate_exact_version_lanes(
    versions: &[VersionExpectation],
    required_lanes: &[VersionLane],
) -> Result<(), CanonicalBoundaryError> {
    if versions.len() == required_lanes.len()
        && versions
            .iter()
            .map(|version| version.lane)
            .eq(required_lanes.iter().copied())
    {
        Ok(())
    } else {
        Err(CanonicalBoundaryError::ReplayReceiptInvalid)
    }
}

fn stable(value: &str) -> StableId {
    StableId::new(value).expect("static canonical boundary identifier")
}

fn report(value: &str) -> ReportText {
    ReportText::new(value).expect("static canonical boundary report")
}

#[allow(dead_code)]
const _: u32 = CANONICAL_ACTION_SCHEMA_VERSION;
