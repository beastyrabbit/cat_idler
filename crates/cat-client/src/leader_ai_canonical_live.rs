//! Canonical protocol-v3 client transport state.
//!
//! This is the only client-side bridge for the LAI.64 report-safe wire
//! contract.  It deliberately retains decoded reports, authenticated identity,
//! selected-colony scope, idempotency, and typed server feedback; it does not
//! inspect simulation state, reconstruct private facts, or translate legacy
//! LAI.24/25 messages.  The root WebSocket owner supplies bounded text frames
//! here and drains the already-validated canonical action queue.

use std::collections::VecDeque;

use bevy::prelude::*;
use cat_protocol::PROTOCOL_VERSION;
use cat_protocol::lai64::{
    ActionErrorSnapshot, ActionOutcome, ActionReceipt, CANONICAL_ACTION_SCHEMA_VERSION,
    CanonicalActionEnvelope, CanonicalGodAction, CanonicalSnapshotEnvelope, CanonicalWireError,
    MAX_CANONICAL_ACTION_WIRE_BYTES, MAX_CANONICAL_SNAPSHOT_WIRE_BYTES, StableId,
    VersionExpectation,
};

use crate::leader_ai_ui::{
    lai50_food::{
        Lai50ActionIntent as Lai50FoodActionIntent, Lai50RefreshState as Lai50FoodRefreshState,
        Lai50SnapshotFeed as Lai50FoodSnapshotFeed, Lai50ViewState as Lai50FoodViewState,
    },
    lai50_hole_hunting::{
        Lai50ActionIntent as Lai50HoleActionIntent, Lai50RefreshState as Lai50HoleRefreshState,
        Lai50SnapshotFeed as Lai50HoleSnapshotFeed, Lai50ViewState as Lai50HoleViewState,
    },
    lai50_item_detail::{
        Lai50RefreshState as Lai50ItemRefreshState, Lai50SnapshotFeed as Lai50ItemSnapshotFeed,
    },
    lai66::{Lai66RefreshState, Lai66SnapshotFeed},
    lai67::{Lai67ActionIntent, Lai67RefreshState, Lai67SnapshotFeed, Lai67ViewState},
    lai68::{Lai68FeedState, Lai68SnapshotFeed},
};

/// Bound the number of unsent mutations retained if a socket is briefly
/// unavailable.  A God must receive explicit feedback instead of silently
/// accumulating an unbounded local command log.
pub const MAX_CANONICAL_OUTBOUND_ACTIONS: usize = 128;
pub const MAX_CANONICAL_FEEDBACK: usize = 64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CanonicalLiveConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    UpdateRequired,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanonicalLiveRefreshState {
    Loading,
    Ready,
    Stale { stale_since_ms: i64 },
    UpdateRequired,
    Error { message: String },
}

impl Default for CanonicalLiveRefreshState {
    fn default() -> Self {
        Self::Loading
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanonicalLiveFeedback {
    Accepted {
        idempotency_id: String,
    },
    Replayed {
        idempotency_id: String,
    },
    Rejected {
        idempotency_id: String,
        reason: String,
    },
    UpdateRequired {
        received_version: Option<u32>,
    },
    RateLimited {
        idempotency_id: String,
        retry_after_ms: Option<u64>,
        reason: String,
    },
    Reconnecting,
    TransportError {
        message: String,
    },
}

/// Authenticated and bounded canonical transport state.  The identity is set
/// only from a trusted client session establishment result, never from a UI
/// control or a received action frame.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct CanonicalLiveTransport {
    pub snapshot: Option<CanonicalSnapshotEnvelope>,
    pub selected_colony_id: Option<StableId>,
    pub authenticated_player_id: Option<StableId>,
    pub connection: CanonicalLiveConnectionState,
    pub refresh: CanonicalLiveRefreshState,
    pub feedback: VecDeque<CanonicalLiveFeedback>,
    pub outbound: VecDeque<CanonicalActionEnvelope>,
    in_flight: VecDeque<StableId>,
    next_idempotency_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanonicalLiveWireMessage {
    Snapshot(CanonicalSnapshotEnvelope),
    Receipt(ActionReceipt),
    Error(ActionErrorSnapshot),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanonicalLiveWireError {
    Snapshot(CanonicalWireError),
    ReceiptMalformed,
    ReceiptUnexpected,
    ReceiptPartitionMismatch,
    ReceiptVersionOrder,
    ActionErrorMalformed,
    ActionErrorUnexpected,
    OutboundSerialize,
    ActionUnavailable(&'static str),
    ActionInvalid(CanonicalWireError),
}

/// Cheap, allocation-free routing guard for the root socket owner. Validation
/// still happens only in [`CanonicalLiveTransport::receive_text`]; this merely
/// prevents legacy world/action frames from entering the canonical decoder.
#[must_use]
pub fn looks_like_canonical_frame(encoded: &str) -> bool {
    encoded.contains("\"snapshotSchemaVersion\"")
        || (encoded.contains("\"idempotencyId\"")
            && encoded.contains("\"selectedColonyId\"")
            && encoded.contains("\"outcome\"")
            && encoded.contains("\"committedVersions\""))
        || (encoded.contains("\"code\"")
            && encoded.contains("\"reason\"")
            && encoded.contains("\"refreshVersions\""))
}

impl std::fmt::Display for CanonicalLiveWireError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Snapshot(error) => write!(out, "canonical snapshot: {error}"),
            Self::ReceiptMalformed => out.write_str("malformed canonical action receipt"),
            Self::ReceiptUnexpected => out.write_str("receipt does not match a queued action"),
            Self::ReceiptPartitionMismatch => out.write_str("receipt selected a different colony"),
            Self::ReceiptVersionOrder => out.write_str("receipt versions are not strictly ordered"),
            Self::ActionErrorMalformed => out.write_str("malformed canonical action error"),
            Self::ActionErrorUnexpected => {
                out.write_str("canonical action error has no in-flight action")
            }
            Self::OutboundSerialize => out.write_str("canonical action could not be encoded"),
            Self::ActionUnavailable(reason) => out.write_str(reason),
            Self::ActionInvalid(error) => write!(out, "canonical action: {error}"),
        }
    }
}

impl std::error::Error for CanonicalLiveWireError {}

impl CanonicalLiveTransport {
    /// Trust only the identity returned by the client's authenticated session
    /// establishment path.  This intentionally does not create a socket.
    pub fn authenticate(&mut self, authenticated_player_id: StableId) {
        self.authenticated_player_id = Some(authenticated_player_id);
    }

    pub fn set_connecting(&mut self) {
        self.connection = CanonicalLiveConnectionState::Connecting;
    }

    pub fn mark_reconnecting(&mut self) {
        self.connection = CanonicalLiveConnectionState::Reconnecting;
        self.refresh =
            self.snapshot
                .as_ref()
                .map_or(CanonicalLiveRefreshState::Loading, |snapshot| {
                    CanonicalLiveRefreshState::Stale {
                        stale_since_ms: snapshot.now_ms,
                    }
                });
        self.push_feedback(CanonicalLiveFeedback::Reconnecting);
    }

    /// Decode a snapshot through the protocol's header-first, byte-bounded
    /// decoder.  No UI or local state gets an opportunity to inspect a payload
    /// before exact protocol/schema validation succeeds.
    pub fn receive_snapshot_json(&mut self, encoded: &str) -> Result<(), CanonicalLiveWireError> {
        let snapshot = CanonicalSnapshotEnvelope::decode_json(encoded)
            .map_err(CanonicalLiveWireError::Snapshot)?;
        self.apply_snapshot(snapshot);
        Ok(())
    }

    /// Decode a bounded, strict receipt.  Receipts have no snapshot header in
    /// the LAI.64 DTO, so their protocol lane is pinned by the already-open
    /// canonical-v3 socket and their identity/colony must match queued state.
    pub fn receive_receipt_json(&mut self, encoded: &str) -> Result<(), CanonicalLiveWireError> {
        if encoded.len() > MAX_CANONICAL_ACTION_WIRE_BYTES {
            return Err(CanonicalLiveWireError::ReceiptMalformed);
        }
        let receipt: ActionReceipt =
            serde_json::from_str(encoded).map_err(|_| CanonicalLiveWireError::ReceiptMalformed)?;
        self.apply_receipt(receipt)
    }

    pub fn receive_action_error_json(
        &mut self,
        encoded: &str,
    ) -> Result<ActionErrorSnapshot, CanonicalLiveWireError> {
        if encoded.len() > MAX_CANONICAL_ACTION_WIRE_BYTES {
            return Err(CanonicalLiveWireError::ActionErrorMalformed);
        }
        let error: ActionErrorSnapshot = serde_json::from_str(encoded)
            .map_err(|_| CanonicalLiveWireError::ActionErrorMalformed)?;
        self.apply_action_error(&error)?;
        Ok(error)
    }

    /// Socket integration convenience.  Snapshots are always tried through
    /// their header-first decoder first. A text frame that is not a snapshot
    /// can only be accepted as a strict action receipt or typed action error in
    /// the action byte lane.
    pub fn receive_text(
        &mut self,
        encoded: &str,
    ) -> Result<CanonicalLiveWireMessage, CanonicalLiveWireError> {
        if encoded.len() > MAX_CANONICAL_SNAPSHOT_WIRE_BYTES {
            return Err(CanonicalLiveWireError::Snapshot(
                CanonicalWireError::InvalidBounds("snapshot_wire_bytes"),
            ));
        }
        match CanonicalSnapshotEnvelope::decode_json(encoded) {
            Ok(snapshot) => {
                self.apply_snapshot(snapshot.clone());
                Ok(CanonicalLiveWireMessage::Snapshot(snapshot))
            }
            Err(CanonicalWireError::UnsupportedProtocolVersion(version)) => {
                self.mark_update_required(Some(version));
                Err(CanonicalLiveWireError::Snapshot(
                    CanonicalWireError::UnsupportedProtocolVersion(version),
                ))
            }
            Err(CanonicalWireError::UnsupportedSchemaVersion(version)) => {
                self.mark_update_required(None);
                Err(CanonicalLiveWireError::Snapshot(
                    CanonicalWireError::UnsupportedSchemaVersion(version),
                ))
            }
            Err(_) => {
                if let Ok(receipt) = serde_json::from_str::<ActionReceipt>(encoded) {
                    self.apply_receipt(receipt.clone())?;
                    Ok(CanonicalLiveWireMessage::Receipt(receipt))
                } else {
                    let error = self.receive_action_error_json(encoded)?;
                    Ok(CanonicalLiveWireMessage::Error(error))
                }
            }
        }
    }

    /// Build a single exact-lane envelope from the currently selected,
    /// report-safe canonical snapshot.  The UI supplies only an allowed enum;
    /// the transport supplies authenticated identity, selected colony, stable
    /// idempotency, and precisely the lanes required by that enum.
    pub fn queue_action(
        &mut self,
        payload: CanonicalGodAction,
    ) -> Result<(), CanonicalLiveWireError> {
        if self.connection != CanonicalLiveConnectionState::Connected {
            return Err(CanonicalLiveWireError::ActionUnavailable(
                "canonical transport is not connected",
            ));
        }
        let player = self.authenticated_player_id.clone().ok_or(
            CanonicalLiveWireError::ActionUnavailable(
                "canonical action requires an authenticated player",
            ),
        )?;
        let selected_colony_id =
            self.selected_colony_id
                .clone()
                .ok_or(CanonicalLiveWireError::ActionUnavailable(
                    "canonical action requires a selected colony report",
                ))?;
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or(CanonicalLiveWireError::ActionUnavailable(
                "canonical action requires a current selected-colony report",
            ))?;
        let colony = snapshot
            .colonies
            .first()
            .filter(|colony| colony.colony_id == selected_colony_id)
            .ok_or(CanonicalLiveWireError::ActionUnavailable(
                "canonical snapshot does not contain the selected colony",
            ))?;
        if self.outbound.len() >= MAX_CANONICAL_OUTBOUND_ACTIONS {
            return Err(CanonicalLiveWireError::ActionUnavailable(
                "canonical outbound queue is full",
            ));
        }

        self.next_idempotency_sequence = self.next_idempotency_sequence.saturating_add(1);
        let idempotency_id = StableId::new(format!(
            "canonical:v{PROTOCOL_VERSION}:{}:{}",
            player.as_str(),
            self.next_idempotency_sequence
        ))
        .map_err(CanonicalLiveWireError::ActionInvalid)?;
        let expected_versions = exact_required_versions(&payload, &colony.versions).ok_or(
            CanonicalLiveWireError::ActionUnavailable(
                "the selected report is missing a required version lane",
            ),
        )?;
        let envelope = CanonicalActionEnvelope {
            protocol_version: PROTOCOL_VERSION,
            action_schema_version: CANONICAL_ACTION_SCHEMA_VERSION,
            authenticated_player_id: player,
            selected_colony_id,
            idempotency_id,
            expected_versions,
            payload,
        };
        envelope
            .validate()
            .map_err(CanonicalLiveWireError::ActionInvalid)?;
        self.outbound.push_back(envelope);
        Ok(())
    }

    /// Serialize and remove exactly one queued action for the root WebSocket
    /// sender.  The root must leave the envelope queued when its actual socket
    /// send fails, using [`Self::next_outbound_json`] first if needed.
    pub fn next_outbound_json(&self) -> Result<Option<String>, CanonicalLiveWireError> {
        self.outbound
            .front()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|_| CanonicalLiveWireError::OutboundSerialize)
    }

    pub fn acknowledge_outbound_send(&mut self) {
        if let Some(action) = self.outbound.pop_front() {
            self.in_flight.push_back(action.idempotency_id);
            self.in_flight.truncate(MAX_CANONICAL_OUTBOUND_ACTIONS);
        }
    }

    pub fn mark_update_required(&mut self, received_version: Option<u32>) {
        self.connection = CanonicalLiveConnectionState::UpdateRequired;
        self.refresh = CanonicalLiveRefreshState::UpdateRequired;
        self.push_feedback(CanonicalLiveFeedback::UpdateRequired { received_version });
    }

    pub fn record_transport_error(&mut self, message: impl Into<String>) {
        if self.connection == CanonicalLiveConnectionState::UpdateRequired {
            return;
        }
        let message = message.into();
        self.refresh = CanonicalLiveRefreshState::Error {
            message: message.clone(),
        };
        self.push_feedback(CanonicalLiveFeedback::TransportError { message });
    }

    fn apply_snapshot(&mut self, snapshot: CanonicalSnapshotEnvelope) {
        self.selected_colony_id = Some(snapshot.selected_colony_id.clone());
        self.snapshot = Some(snapshot);
        self.connection = CanonicalLiveConnectionState::Connected;
        self.refresh = CanonicalLiveRefreshState::Ready;
    }

    fn apply_receipt(&mut self, receipt: ActionReceipt) -> Result<(), CanonicalLiveWireError> {
        if self.selected_colony_id.as_ref() != Some(&receipt.selected_colony_id) {
            return Err(CanonicalLiveWireError::ReceiptPartitionMismatch);
        }
        if !strictly_ordered_versions(&receipt.committed_versions) {
            return Err(CanonicalLiveWireError::ReceiptVersionOrder);
        }
        let is_known = self
            .outbound
            .iter()
            .any(|action| action.idempotency_id == receipt.idempotency_id)
            || self
                .in_flight
                .iter()
                .any(|id| id == &receipt.idempotency_id);
        if !is_known {
            return Err(CanonicalLiveWireError::ReceiptUnexpected);
        }
        let idempotency_id = receipt.idempotency_id.as_str().to_owned();
        let reason = receipt.reason.as_ref().map_or_else(
            || "The server rejected this action.".to_owned(),
            |reason| reason.as_str().to_owned(),
        );
        match receipt.outcome {
            ActionOutcome::Accepted => {
                self.push_feedback(CanonicalLiveFeedback::Accepted { idempotency_id })
            }
            ActionOutcome::Replayed => {
                self.push_feedback(CanonicalLiveFeedback::Replayed { idempotency_id })
            }
            ActionOutcome::Rejected => self.push_feedback(CanonicalLiveFeedback::Rejected {
                idempotency_id,
                reason,
            }),
            ActionOutcome::UpdateRequired => {
                self.mark_update_required(Some(PROTOCOL_VERSION));
            }
            ActionOutcome::RateLimited => self.push_feedback(CanonicalLiveFeedback::RateLimited {
                idempotency_id,
                retry_after_ms: None,
                reason,
            }),
        }
        self.in_flight.retain(|id| id != &receipt.idempotency_id);
        Ok(())
    }

    fn apply_action_error(
        &mut self,
        error: &ActionErrorSnapshot,
    ) -> Result<(), CanonicalLiveWireError> {
        if !strictly_ordered_versions(&error.refresh_versions) {
            return Err(CanonicalLiveWireError::ReceiptVersionOrder);
        }
        let idempotency_id = self
            .in_flight
            .pop_front()
            .ok_or(CanonicalLiveWireError::ActionErrorUnexpected)?
            .as_str()
            .to_owned();
        let reason = error.reason.as_str().to_owned();
        match error.code.as_str() {
            "action:update_required" => self.mark_update_required(Some(PROTOCOL_VERSION)),
            "action:rate_limited" => {
                self.push_feedback(CanonicalLiveFeedback::RateLimited {
                    idempotency_id,
                    retry_after_ms: error.retry_after_ms,
                    reason,
                });
            }
            _ => self.push_feedback(CanonicalLiveFeedback::Rejected {
                idempotency_id,
                reason,
            }),
        }
        Ok(())
    }

    fn push_feedback(&mut self, feedback: CanonicalLiveFeedback) {
        self.feedback.push_back(feedback);
        self.feedback.truncate(MAX_CANONICAL_FEEDBACK);
    }
}

fn exact_required_versions(
    payload: &CanonicalGodAction,
    available: &[VersionExpectation],
) -> Option<Vec<VersionExpectation>> {
    payload
        .required_lanes()
        .iter()
        .map(|required| {
            available
                .iter()
                .find(|reported| reported.lane == *required)
                .cloned()
        })
        .collect()
}

fn strictly_ordered_versions(versions: &[VersionExpectation]) -> bool {
    versions.windows(2).all(|pair| pair[0].lane < pair[1].lane)
}

/// Projects the one canonical snapshot into the report-only primary UI feeds.
/// This is a clone of a validated DTO, not a model conversion or inference.
fn project_canonical_snapshot_feeds(
    transport: Res<'_, CanonicalLiveTransport>,
    mut lai50_hole: ResMut<'_, Lai50HoleSnapshotFeed>,
    mut lai50_food: ResMut<'_, Lai50FoodSnapshotFeed>,
    mut lai50_item: ResMut<'_, Lai50ItemSnapshotFeed>,
    mut lai66: ResMut<'_, Lai66SnapshotFeed>,
    mut lai67: ResMut<'_, Lai67SnapshotFeed>,
    mut lai68: ResMut<'_, Lai68SnapshotFeed>,
) {
    let envelope = transport.snapshot.clone();
    let lai50_hole_refresh = match &transport.refresh {
        CanonicalLiveRefreshState::Loading => Lai50HoleRefreshState::Loading,
        CanonicalLiveRefreshState::Ready => Lai50HoleRefreshState::Ready,
        CanonicalLiveRefreshState::Stale { stale_since_ms } => Lai50HoleRefreshState::Stale {
            stale_since_ms: *stale_since_ms,
        },
        CanonicalLiveRefreshState::UpdateRequired => Lai50HoleRefreshState::UpdateRequired,
        CanonicalLiveRefreshState::Error { message } => Lai50HoleRefreshState::Error {
            message: message.clone(),
        },
    };
    let lai50_food_refresh = match &transport.refresh {
        CanonicalLiveRefreshState::Loading => Lai50FoodRefreshState::Loading,
        CanonicalLiveRefreshState::Ready => Lai50FoodRefreshState::Ready,
        CanonicalLiveRefreshState::Stale { stale_since_ms } => Lai50FoodRefreshState::Stale {
            stale_since_ms: *stale_since_ms,
        },
        CanonicalLiveRefreshState::UpdateRequired => Lai50FoodRefreshState::UpdateRequired,
        CanonicalLiveRefreshState::Error { message } => Lai50FoodRefreshState::Error {
            message: message.clone(),
        },
    };
    let lai50_item_refresh = match &transport.refresh {
        CanonicalLiveRefreshState::Loading => Lai50ItemRefreshState::Loading,
        CanonicalLiveRefreshState::Ready => Lai50ItemRefreshState::Ready,
        CanonicalLiveRefreshState::Stale { stale_since_ms } => Lai50ItemRefreshState::Stale {
            stale_since_ms: *stale_since_ms,
        },
        CanonicalLiveRefreshState::UpdateRequired => Lai50ItemRefreshState::UpdateRequired,
        CanonicalLiveRefreshState::Error { message } => Lai50ItemRefreshState::Error {
            message: message.clone(),
        },
    };
    let (lai66_refresh, lai67_refresh, lai68_state) = match &transport.refresh {
        CanonicalLiveRefreshState::Loading => (
            Lai66RefreshState::Loading,
            Lai67RefreshState::Loading,
            Lai68FeedState::Loading,
        ),
        CanonicalLiveRefreshState::Ready => (
            Lai66RefreshState::Ready,
            Lai67RefreshState::Ready,
            Lai68FeedState::Ready,
        ),
        CanonicalLiveRefreshState::Stale { stale_since_ms } => (
            Lai66RefreshState::Stale {
                stale_since_ms: *stale_since_ms,
            },
            Lai67RefreshState::Stale {
                stale_since_ms: *stale_since_ms,
            },
            Lai68FeedState::Stale {
                stale_since_ms: *stale_since_ms,
            },
        ),
        CanonicalLiveRefreshState::UpdateRequired => (
            Lai66RefreshState::UpdateRequired,
            Lai67RefreshState::UpdateRequired,
            Lai68FeedState::UpdateRequired,
        ),
        CanonicalLiveRefreshState::Error { message } => (
            Lai66RefreshState::Error {
                message: message.clone(),
            },
            Lai67RefreshState::Error {
                message: message.clone(),
            },
            Lai68FeedState::Error {
                message: message.clone(),
            },
        ),
    };
    if lai50_hole.envelope != envelope || lai50_hole.refresh != lai50_hole_refresh {
        lai50_hole.envelope = envelope.clone();
        lai50_hole.refresh = lai50_hole_refresh;
    }
    if lai50_food.envelope != envelope || lai50_food.refresh != lai50_food_refresh {
        lai50_food.envelope = envelope.clone();
        lai50_food.refresh = lai50_food_refresh;
    }
    if lai50_item.envelope != envelope || lai50_item.refresh != lai50_item_refresh {
        lai50_item.envelope = envelope.clone();
        lai50_item.refresh = lai50_item_refresh;
    }
    if lai66.envelope != envelope || lai66.refresh != lai66_refresh {
        lai66.envelope = envelope.clone();
        lai66.refresh = lai66_refresh;
    }
    if lai67.envelope != envelope || lai67.refresh != lai67_refresh {
        lai67.envelope = envelope.clone();
        lai67.refresh = lai67_refresh;
    }
    if lai68.envelope != envelope || lai68.state != lai68_state {
        lai68.envelope = envelope;
        lai68.state = lai68_state;
    }
}

fn drain_lai50_hole_action_intent(
    mut transport: ResMut<'_, CanonicalLiveTransport>,
    mut intent: ResMut<'_, Lai50HoleActionIntent>,
    mut view: ResMut<'_, Lai50HoleViewState>,
) {
    let Some(action) = intent.pending.clone() else {
        return;
    };
    match transport.queue_action(action) {
        Ok(()) => {
            intent.pending = None;
            view.local_feedback =
                Some("Action queued for the authenticated server receipt.".to_owned());
        }
        Err(error) => {
            view.local_feedback = Some(format!("Action remains pending: {error}"));
        }
    }
}

fn drain_lai50_food_action_intent(
    mut transport: ResMut<'_, CanonicalLiveTransport>,
    mut intent: ResMut<'_, Lai50FoodActionIntent>,
    mut view: ResMut<'_, Lai50FoodViewState>,
) {
    let Some(action) = intent.pending.clone() else {
        return;
    };
    match transport.queue_action(action) {
        Ok(()) => {
            intent.pending = None;
            view.local_feedback =
                Some("Action queued for the authenticated server receipt.".to_owned());
        }
        Err(error) => {
            view.local_feedback = Some(format!("Action remains pending: {error}"));
        }
    }
}

/// Drains only the LAI.67 allowed-action enum after that UI has run.  It does
/// not accept arbitrary JSON, legacy local actions, worker commands, or tile
/// commands.  On a transient unavailable transport the intent remains pending
/// so the UI does not silently lose a player decision.
fn drain_lai67_action_intent(
    mut transport: ResMut<'_, CanonicalLiveTransport>,
    mut intent: ResMut<'_, Lai67ActionIntent>,
    mut view: ResMut<'_, Lai67ViewState>,
) {
    let Some(action) = intent.pending.clone() else {
        return;
    };
    match transport.queue_action(action) {
        Ok(()) => {
            intent.pending = None;
            view.last_local_feedback = Some(
                "Canonical action queued for the authenticated server; awaiting its receipt."
                    .to_owned(),
            );
        }
        Err(error) => {
            view.last_local_feedback = Some(format!("Action remains pending: {error}"));
        }
    }
}

/// Canonical transport plugin.  Add this after the LAI.54/66/67/68 UI plugins.
/// Socket creation remains in the root client because it owns target-specific
/// ewebsock lifecycle; the root forwards received text to
/// [`CanonicalLiveTransport::receive_text`] and sends
/// [`CanonicalLiveTransport::next_outbound_json`] only after success.
#[derive(Default)]
pub struct CanonicalLiveTransportPlugin;

impl Plugin for CanonicalLiveTransportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CanonicalLiveTransport>()
            .init_resource::<Lai50HoleSnapshotFeed>()
            .init_resource::<Lai50FoodSnapshotFeed>()
            .init_resource::<Lai50ItemSnapshotFeed>()
            .init_resource::<Lai50HoleActionIntent>()
            .init_resource::<Lai50FoodActionIntent>()
            .init_resource::<Lai50HoleViewState>()
            .init_resource::<Lai50FoodViewState>()
            .init_resource::<Lai66SnapshotFeed>()
            .init_resource::<Lai67SnapshotFeed>()
            .init_resource::<Lai68SnapshotFeed>()
            .init_resource::<Lai67ActionIntent>()
            .init_resource::<Lai67ViewState>()
            .add_systems(PreUpdate, project_canonical_snapshot_feeds)
            .add_systems(
                PostUpdate,
                (
                    drain_lai50_hole_action_intent,
                    drain_lai50_food_action_intent,
                    drain_lai67_action_intent,
                )
                    .chain(),
            );
    }
}
