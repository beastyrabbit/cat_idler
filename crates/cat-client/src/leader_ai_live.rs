//! Live protocol-v2 bridge for the leader-AI client surfaces.
//!
//! The bridge deliberately owns only authenticated wire state. Rendering and
//! projections consume the decoded report-safe snapshot; no client simulation
//! or coordinate inference belongs here.

use std::collections::VecDeque;

use bevy::prelude::*;
use cat_protocol::{
    ActionConflict, LeaderAiActionEnvelope, LeaderAiActionResponse, LeaderAiSnapshotEnvelope,
    PROTOCOL_VERSION, SnapshotDecodeError,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeaderAiWireMessage {
    Snapshot(LeaderAiSnapshotEnvelope),
    Action(Box<LeaderAiActionResponse>),
    UpdateRequired { received_version: Option<u32> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeaderAiWireError {
    MalformedFrame,
    Snapshot(String),
    Action(String),
}

impl std::fmt::Display for LeaderAiWireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedFrame => f.write_str("malformed leader-AI frame"),
            Self::Snapshot(error) => write!(f, "snapshot frame: {error}"),
            Self::Action(error) => write!(f, "action frame: {error}"),
        }
    }
}

impl std::error::Error for LeaderAiWireError {}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct LeaderAiLiveState {
    pub snapshot: Option<LeaderAiSnapshotEnvelope>,
    pub selected_colony_id: Option<String>,
    pub selected_colony_version: Option<u64>,
    pub connection: LeaderAiConnectionState,
    pub feedback: VecDeque<LeaderAiFeedback>,
    pub outbound: VecDeque<LeaderAiActionEnvelope>,
    pub authenticated_player_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LeaderAiConnectionState {
    #[default]
    Disconnected,
    Connected,
    UpdateRequired,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeaderAiFeedback {
    Accepted {
        idempotency_id: String,
        result_code: String,
    },
    Rejected {
        idempotency_id: String,
        conflict: Box<ActionConflict>,
    },
    Duplicate {
        idempotency_id: String,
        original_accepted: bool,
    },
    UpdateRequired,
    Reconnecting,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct LeaderAiSelectedColonyResource {
    pub colony_id: Option<String>,
    pub state_version: Option<u64>,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct LeaderAiVersionResource {
    pub state_version: Option<u64>,
    pub planner_version: Option<u64>,
    pub resource_version: Option<u64>,
    pub spatial_version: Option<u64>,
    pub research_version: Option<u64>,
    pub boost_version: Option<u64>,
    pub diplomacy_version: Option<u64>,
    pub trade_version: Option<u64>,
}

/// Decode the protocol header before nested snapshot/action deserialization.
/// Unknown protocol versions become an explicit UPDATE_REQUIRED state.
pub fn decode_leader_ai_frame(text: &str) -> Result<LeaderAiWireMessage, LeaderAiWireError> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|_| LeaderAiWireError::MalformedFrame)?;
    let object = value.as_object().ok_or(LeaderAiWireError::MalformedFrame)?;
    let protocol = object
        .get("protocolVersion")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    if protocol != Some(PROTOCOL_VERSION) {
        return Ok(LeaderAiWireMessage::UpdateRequired {
            received_version: protocol,
        });
    }
    if object.contains_key("colonies") {
        return LeaderAiSnapshotEnvelope::decode_json(text)
            .map(LeaderAiWireMessage::Snapshot)
            .map_err(|error| LeaderAiWireError::Snapshot(error.to_string()));
    }
    if object.contains_key("result") && object.contains_key("idempotencyId") {
        return serde_json::from_value(value)
            .map(|response| LeaderAiWireMessage::Action(Box::new(response)))
            .map_err(|error| LeaderAiWireError::Action(error.to_string()));
    }
    Err(LeaderAiWireError::MalformedFrame)
}

pub fn apply_leader_ai_frame(state: &mut LeaderAiLiveState, text: &str) {
    match decode_leader_ai_frame(text) {
        Ok(LeaderAiWireMessage::Snapshot(snapshot)) => {
            state.selected_colony_id = Some(snapshot.selected_colony_id.as_str().to_owned());
            state.selected_colony_version = snapshot
                .colonies
                .iter()
                .find(|colony| colony.colony_id == snapshot.selected_colony_id)
                .map(|colony| colony.state_version);
            state.snapshot = Some(snapshot);
            state.connection = LeaderAiConnectionState::Connected;
        }
        Ok(LeaderAiWireMessage::Action(response)) => {
            let idempotency_id = response.idempotency_id.as_str().to_owned();
            let feedback = match response.result {
                cat_protocol::LeaderAiActionResult::Accepted { accepted } => {
                    LeaderAiFeedback::Accepted {
                        idempotency_id,
                        result_code: accepted.result_code.as_str().to_owned(),
                    }
                }
                cat_protocol::LeaderAiActionResult::Rejected { conflict } => {
                    LeaderAiFeedback::Rejected {
                        idempotency_id,
                        conflict: Box::new(conflict),
                    }
                }
                cat_protocol::LeaderAiActionResult::DuplicateReplay { replay } => {
                    LeaderAiFeedback::Duplicate {
                        idempotency_id,
                        original_accepted: replay.original_accepted,
                    }
                }
            };
            state.feedback.push_back(feedback);
            state.feedback.truncate(32);
        }
        Ok(LeaderAiWireMessage::UpdateRequired { .. }) => {
            state.connection = LeaderAiConnectionState::UpdateRequired;
            state.feedback.push_back(LeaderAiFeedback::UpdateRequired);
        }
        Err(_error) => state.feedback.push_back(LeaderAiFeedback::Rejected {
            idempotency_id: "frame".to_owned(),
            conflict: Box::new(ActionConflict::MalformedPayload),
        }),
    }
    state.feedback.truncate(32);
}

pub fn queue_authenticated_leader_ai_action(
    state: &mut LeaderAiLiveState,
    action: LeaderAiActionEnvelope,
) -> Result<(), &'static str> {
    if state.authenticated_player_id.is_none() {
        return Err("leader-AI action requires an authenticated player");
    }
    if state.connection != LeaderAiConnectionState::Connected {
        return Err("leader-AI transport is not connected");
    }
    if state.selected_colony_id.as_deref() != Some(action.colony_id.as_str()) {
        return Err("action colony is not selected");
    }
    state.outbound.push_back(action);
    Ok(())
}

#[must_use]
pub fn looks_like_leader_ai_frame(text: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|value| value.as_object().cloned())
        .is_some_and(|object| {
            object.contains_key("schemaVersion") || object.contains_key("idempotencyId")
        })
}

pub fn mark_leader_ai_reconnecting(state: &mut LeaderAiLiveState) {
    state.connection = LeaderAiConnectionState::Disconnected;
    state.feedback.push_back(LeaderAiFeedback::Reconnecting);
    state.feedback.truncate(32);
}

pub fn leader_ai_snapshot_decode_error(error: SnapshotDecodeError) -> LeaderAiWireMessage {
    match error {
        SnapshotDecodeError::UnsupportedProtocolVersion(version) => {
            LeaderAiWireMessage::UpdateRequired {
                received_version: Some(version),
            }
        }
        _ => LeaderAiWireMessage::UpdateRequired {
            received_version: None,
        },
    }
}
