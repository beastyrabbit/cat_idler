use cat_client::{
    LeaderAiConnectionState, LeaderAiFeedback, LeaderAiLiveState, LeaderAiWireMessage,
    apply_leader_ai_frame, decode_leader_ai_frame, queue_authenticated_leader_ai_action,
};
use cat_protocol::{
    ActionAcceptedResult, ActionConflict, ActionIdempotencyId, ActionProtocolVersion,
    ActionReplayResult, BoundedActionId, BoundedBasisPointNudge, BoundedEntityId, BoundedPlayerId,
    CurrentStateHint, CurrentVersionHint, ExpectedStateVersions, LeaderAiActionEnvelope,
    LeaderAiActionPayload, LeaderAiActionResponse, LeaderAiActionResult, ReportSafeString,
    SelectedColonyId,
};

fn action() -> LeaderAiActionEnvelope {
    LeaderAiActionEnvelope {
        protocol_version: ActionProtocolVersion::current(),
        idempotency_id: ActionIdempotencyId::new("action:live:1").expect("id"),
        colony_id: SelectedColonyId::new("colony:one").expect("colony"),
        player_id: BoundedPlayerId::new("player:one").expect("player"),
        expected_versions: ExpectedStateVersions {
            expected_planner_version: 1,
            expected_domain_version: 1,
            expected_resource_version: 1,
            expected_spatial_version: Some(1),
            expected_reservation_version: Some(1),
            expected_research_version: Some(1),
            expected_scholar_version: Some(1),
            expected_boost_version: Some(1),
            expected_diplomacy_version: Some(1),
            expected_trade_version: Some(1),
            expected_prosthetic_version: Some(1),
            expected_care_version: Some(1),
            expected_officer_version: Some(1),
            expected_standing_order_version: Some(1),
        },
        payload: LeaderAiActionPayload::NudgePlan {
            plan_id: BoundedEntityId::new("plan:one").expect("plan"),
            nudge: BoundedBasisPointNudge::new(1_500).expect("bounded nudge"),
            reason_key: None,
        },
    }
}

#[test]
fn incompatible_header_is_update_required_before_nested_decode() {
    let frame = r#"{"protocolVersion":99,"schemaVersion":999,"colonies":["not decoded"]}"#;
    assert_eq!(
        decode_leader_ai_frame(frame).expect("update required"),
        LeaderAiWireMessage::UpdateRequired {
            received_version: Some(99)
        }
    );
}

#[test]
fn action_queue_requires_authentication_and_selected_colony() {
    let mut state = LeaderAiLiveState {
        connection: LeaderAiConnectionState::Connected,
        selected_colony_id: Some("colony:one".to_owned()),
        ..Default::default()
    };
    let queued_action = action();
    assert_eq!(
        queue_authenticated_leader_ai_action(&mut state, queued_action.clone()),
        Err("leader-AI action requires an authenticated player")
    );
    state.authenticated_player_id = Some("player:one".to_owned());
    let mut foreign = queued_action;
    foreign.colony_id = SelectedColonyId::new("colony:other").expect("colony");
    assert_eq!(
        queue_authenticated_leader_ai_action(&mut state, foreign),
        Err("action colony is not selected")
    );
    assert!(queue_authenticated_leader_ai_action(&mut state, action()).is_ok());
    assert_eq!(state.outbound.len(), 1);
}

#[test]
fn malformed_current_leader_ai_frame_is_not_fallback_world_data() {
    let frame = r#"{"protocolVersion":2,"schemaVersion":1,"colonies":[]}"#;
    assert!(decode_leader_ai_frame(frame).is_err());
}

#[test]
fn action_id_type_remains_bounded_and_stable() {
    let id = BoundedActionId::new("action:live:1").expect("bounded id");
    assert_eq!(id.as_str(), "action:live:1");
}

fn response(result: LeaderAiActionResult) -> String {
    serde_json::to_string(&LeaderAiActionResponse {
        protocol_version: ActionProtocolVersion::current(),
        idempotency_id: ActionIdempotencyId::new("action:live:response").expect("id"),
        colony_id: SelectedColonyId::new("colony:one").expect("colony"),
        result,
        refresh: None,
    })
    .expect("response json")
}

fn versions() -> CurrentVersionHint {
    CurrentVersionHint {
        planner_version: Some(2),
        domain_version: Some(2),
        resource_version: Some(2),
        spatial_version: None,
        reservation_version: None,
        research_version: None,
        scholar_version: None,
        boost_version: None,
        diplomacy_version: None,
        trade_version: None,
        prosthetic_version: None,
        care_version: None,
        officer_version: None,
        standing_order_version: None,
    }
}

fn state_hint() -> CurrentStateHint {
    CurrentStateHint {
        state_code: ReportSafeString::new("accepted").expect("state"),
        visible_entity_id: None,
        visible_stage: None,
    }
}

#[test]
fn action_replies_become_typed_feedback_without_client_simulation() {
    let accepted = response(LeaderAiActionResult::Accepted {
        accepted: ActionAcceptedResult {
            result_code: ReportSafeString::new("nudge_committed").expect("code"),
            changed_ids: vec![BoundedEntityId::new("plan:one").expect("plan")],
            committed_versions: versions(),
            current_state_hint: Some(state_hint()),
        },
    });
    let rejected = response(LeaderAiActionResult::Rejected {
        conflict: ActionConflict::VersionMismatch {
            current_version_hint: versions(),
            current_state_hint: state_hint(),
        },
    });
    let duplicate = response(LeaderAiActionResult::DuplicateReplay {
        replay: ActionReplayResult {
            original_accepted: true,
            result_code: ReportSafeString::new("nudge_committed").expect("code"),
            committed_versions: Some(versions()),
            current_state_hint: Some(state_hint()),
        },
    });
    let mut state = LeaderAiLiveState::default();
    apply_leader_ai_frame(&mut state, &accepted);
    apply_leader_ai_frame(&mut state, &rejected);
    apply_leader_ai_frame(&mut state, &duplicate);
    assert!(matches!(
        state.feedback.pop_front(),
        Some(LeaderAiFeedback::Accepted { .. })
    ));
    assert!(matches!(
        state.feedback.pop_front(),
        Some(LeaderAiFeedback::Rejected { .. })
    ));
    assert!(matches!(
        state.feedback.pop_front(),
        Some(LeaderAiFeedback::Duplicate {
            original_accepted: true,
            ..
        })
    ));
}
