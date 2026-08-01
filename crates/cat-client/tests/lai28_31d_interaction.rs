use bevy::prelude::*;
use cat_client::leader_ai_ui::{
    LeaderAiActionButton, LeaderAiInteractionPlugin, LeaderAiInteractionState,
    LeaderAiSelectionButton, LeaderAiSelectionKind,
};
use cat_client::{LeaderAiConnectionState, LeaderAiFeedback, LeaderAiLiveState};
use cat_protocol::{
    ActionIdempotencyId, ActionProtocolVersion, BoundedBasisPointNudge, BoundedEntityId,
    BoundedPlayerId, ExpectedStateVersions, LeaderAiActionEnvelope, LeaderAiActionPayload,
    SelectedColonyId,
};

fn envelope() -> LeaderAiActionEnvelope {
    LeaderAiActionEnvelope {
        protocol_version: ActionProtocolVersion::current(),
        idempotency_id: ActionIdempotencyId::new("action:ui:control").expect("id"),
        colony_id: SelectedColonyId::new("colony:one").expect("colony"),
        player_id: BoundedPlayerId::new("player:one").expect("player"),
        expected_versions: ExpectedStateVersions {
            expected_planner_version: 4,
            expected_domain_version: 7,
            expected_resource_version: 7,
            expected_spatial_version: None,
            expected_reservation_version: None,
            expected_research_version: None,
            expected_scholar_version: None,
            expected_boost_version: None,
            expected_diplomacy_version: None,
            expected_trade_version: None,
            expected_prosthetic_version: None,
            expected_care_version: None,
            expected_officer_version: None,
            expected_standing_order_version: None,
        },
        payload: LeaderAiActionPayload::NudgePlan {
            plan_id: BoundedEntityId::new("plan:one").expect("plan"),
            nudge: BoundedBasisPointNudge::new(1_500).expect("nudge"),
            reason_key: None,
        },
    }
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(LeaderAiInteractionPlugin)
        .insert_resource(LeaderAiLiveState {
            connection: LeaderAiConnectionState::Connected,
            selected_colony_id: Some("colony:one".to_owned()),
            authenticated_player_id: Some("player:one".to_owned()),
            ..Default::default()
        });
    app
}

#[test]
fn pressed_action_button_queues_exact_expected_version_envelope() {
    let mut app = app();
    app.world_mut().spawn((
        Button,
        Interaction::Pressed,
        LeaderAiActionButton {
            envelope: envelope(),
            label: "Move up plan:one".to_owned(),
            test_id: "lai-ui:plan:plan:one:up".to_owned(),
        },
    ));
    app.update();
    let live = app.world().resource::<LeaderAiLiveState>();
    assert_eq!(live.outbound.len(), 1);
    assert_eq!(
        live.outbound[0].expected_versions.expected_planner_version,
        4
    );
    assert!(matches!(
        live.outbound[0].payload,
        LeaderAiActionPayload::NudgePlan { .. }
    ));
    assert_eq!(
        app.world()
            .resource::<LeaderAiInteractionState>()
            .pending_idempotency_ids,
        ["action:ui:control"]
    );
}

#[test]
fn selection_button_changes_only_selected_report_safe_id() {
    let mut app = app();
    app.world_mut().spawn((
        Button,
        Interaction::Pressed,
        LeaderAiSelectionButton {
            kind: LeaderAiSelectionKind::Cat,
            stable_id: "cat:mallow".to_owned(),
            label: "Select cat Mallow".to_owned(),
            test_id: "lai-ui:care:cat:mallow:select".to_owned(),
        },
    ));
    app.update();
    let state = app.world().resource::<LeaderAiInteractionState>();
    assert_eq!(state.selected_cat_id.as_deref(), Some("cat:mallow"));
    assert!(
        app.world()
            .resource::<LeaderAiLiveState>()
            .outbound
            .is_empty()
    );
}

#[test]
fn typed_feedback_clears_pending_and_marks_update_required_stale() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<LeaderAiInteractionState>()
        .pending_idempotency_ids
        .push("action:ui:control".to_owned());
    app.world_mut()
        .resource_mut::<LeaderAiLiveState>()
        .feedback
        .push_back(LeaderAiFeedback::Accepted {
            idempotency_id: "action:ui:control".to_owned(),
            result_code: "accepted".to_owned(),
        });
    app.world_mut()
        .resource_mut::<LeaderAiLiveState>()
        .feedback
        .push_back(LeaderAiFeedback::UpdateRequired);
    app.update();
    let state = app.world().resource::<LeaderAiInteractionState>();
    assert!(state.pending_idempotency_ids.is_empty());
    assert!(state.stale_refresh_required);
}

#[test]
fn malformed_or_unauthenticated_button_stays_pending_free() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<LeaderAiLiveState>()
        .authenticated_player_id = None;
    app.world_mut().spawn((
        Button,
        Interaction::Pressed,
        LeaderAiActionButton {
            envelope: envelope(),
            label: "Move up".to_owned(),
            test_id: "lai-ui:plan:plan:one:up".to_owned(),
        },
    ));
    app.update();
    assert!(
        app.world()
            .resource::<LeaderAiLiveState>()
            .outbound
            .is_empty()
    );
    assert!(
        app.world()
            .resource::<LeaderAiInteractionState>()
            .pending_idempotency_ids
            .is_empty()
    );
}
