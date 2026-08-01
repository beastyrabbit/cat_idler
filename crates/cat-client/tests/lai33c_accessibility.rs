use accesskit::{Action, ActionRequest, NodeId, Role, TreeId};
use bevy::{
    a11y::ActionRequest as BevyActionRequest,
    input::{
        ButtonState,
        keyboard::{Key, KeyCode, KeyboardInput},
    },
    prelude::*,
};
use bevy_input_focus::InputFocus;
use cat_client::{
    LeaderAiConnectionState, LeaderAiLiveState,
    leader_ai_ui::{
        LeaderAiActionButton, LeaderAiInteractionPlugin, LeaderAiInteractionState,
        LeaderAiLocalAction, LeaderAiLocalButton, LeaderAiSelectionButton, LeaderAiSelectionKind,
        LeaderAiSemanticNode, TestIdBuilder, semantic_node, semantic_status_node,
    },
};
use cat_protocol::{
    ActionIdempotencyId, ActionProtocolVersion, BoundedBasisPointNudge, BoundedEntityId,
    BoundedPlayerId, ExpectedStateVersions, LeaderAiActionEnvelope, LeaderAiActionPayload,
    SelectedColonyId,
};

fn envelope() -> LeaderAiActionEnvelope {
    LeaderAiActionEnvelope {
        protocol_version: ActionProtocolVersion::current(),
        idempotency_id: ActionIdempotencyId::new("action:a11y:plan").expect("id"),
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

fn key(key_code: KeyCode) -> KeyboardInput {
    KeyboardInput {
        key_code,
        logical_key: Key::Character("".into()),
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    }
}

fn action_request(entity: Entity, action: Action) -> BevyActionRequest {
    BevyActionRequest(ActionRequest {
        action,
        target_tree: TreeId::ROOT,
        target_node: NodeId(entity.to_bits()),
        data: None,
    })
}

#[test]
fn accesskit_nodes_cover_panels_status_controls_and_report_safe_ids() {
    for panel in [
        "plans",
        "standing-orders",
        "officers",
        "tasks",
        "care",
        "shrine",
        "favor",
        "research",
        "scholars",
        "boosts",
        "diplomacy",
        "trade",
    ] {
        let id = TestIdBuilder::named_panel(panel);
        assert!(
            cat_client::leader_ai_ui::report_safe_semantic_id(id.as_str()),
            "{id:?}"
        );
        let node = semantic_node(Role::Pane, id.as_str(), format!("{panel} panel"), true);
        assert_eq!(node.role(), Role::Pane);
        assert_eq!(
            node.description(),
            Some(format!("test-id:{}", id.as_str()).as_str())
        );
        assert!(!node.supports_action(Action::Click));
    }

    let reconnect = semantic_status_node(
        "lai-connection:status",
        "Reconnecting. Showing the last report-safe snapshot.",
        true,
    );
    assert_eq!(reconnect.role(), Role::Alert);
    assert!(
        reconnect
            .label()
            .is_some_and(|label| !label.to_ascii_lowercase().contains("regeneration"))
    );

    let reload = semantic_node(
        Role::Button,
        "lai-ui:connection:control:reload",
        "Reload the client",
        true,
    );
    assert!(reload.supports_action(Action::Focus));
    assert!(reload.supports_action(Action::Click));
}

#[test]
fn tab_arrow_enter_and_space_route_selection_and_exact_action() {
    let mut app = app();
    let selection = app
        .world_mut()
        .spawn((
            semantic_node(
                Role::Button,
                "lai-ui:tasks:task:task-one",
                "Select visible task",
                true,
            ),
            LeaderAiSemanticNode {
                semantic_id: "lai-ui:tasks:task:task-one".to_owned(),
                focus_order: 10,
                enabled: true,
            },
            LeaderAiSelectionButton {
                kind: LeaderAiSelectionKind::Task,
                stable_id: "task:one".to_owned(),
                label: "Select task one".to_owned(),
                test_id: "lai-ui:tasks:task:task-one".to_owned(),
            },
        ))
        .id();
    let action = app
        .world_mut()
        .spawn((
            semantic_node(
                Role::Button,
                "lai-ui:plans:control:move-up:plan-one",
                "Move plan up",
                true,
            ),
            LeaderAiSemanticNode {
                semantic_id: "lai-ui:plans:control:move-up:plan-one".to_owned(),
                focus_order: 20,
                enabled: true,
            },
            LeaderAiActionButton {
                envelope: envelope(),
                label: "Move plan up".to_owned(),
                test_id: "lai-ui:plans:control:move-up:plan-one".to_owned(),
            },
        ))
        .id();

    app.world_mut().write_message(key(KeyCode::Tab));
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(selection));

    app.world_mut().write_message(key(KeyCode::Enter));
    app.update();
    assert_eq!(
        app.world()
            .resource::<LeaderAiInteractionState>()
            .selected_task_id
            .as_deref(),
        Some("task:one")
    );

    app.world_mut().write_message(key(KeyCode::ArrowDown));
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(action));

    app.world_mut().write_message(key(KeyCode::Space));
    app.update();
    assert_eq!(
        app.world().resource::<LeaderAiLiveState>().outbound.front(),
        Some(&envelope())
    );
}

#[test]
fn accesskit_focus_click_routes_marker_selection_and_reload() {
    let mut app = app();
    let marker = app
        .world_mut()
        .spawn((
            semantic_node(
                Role::Button,
                "lai-ui:tasks:task:task-one:site:cave:objective",
                "Hunt objective, visible cave",
                true,
            ),
            LeaderAiSemanticNode {
                semantic_id: "lai-ui:tasks:task:task-one:site:cave:objective".to_owned(),
                focus_order: 10,
                enabled: true,
            },
            LeaderAiSelectionButton {
                kind: LeaderAiSelectionKind::Task,
                stable_id: "task:one".to_owned(),
                label: "Hunt objective, visible cave".to_owned(),
                test_id: "lai-ui:tasks:task:task-one:site:cave:objective".to_owned(),
            },
        ))
        .id();
    let reload = app
        .world_mut()
        .spawn((
            semantic_node(
                Role::Button,
                "lai-ui:connection:control:reload",
                "Reload the client",
                true,
            ),
            LeaderAiSemanticNode {
                semantic_id: "lai-ui:connection:control:reload".to_owned(),
                focus_order: 20,
                enabled: true,
            },
            LeaderAiLocalButton {
                action: LeaderAiLocalAction::Reload,
                label: "Reload the client".to_owned(),
                test_id: "lai-ui:connection:control:reload".to_owned(),
            },
        ))
        .id();

    app.world_mut()
        .write_message(action_request(marker, Action::Focus));
    app.world_mut()
        .write_message(action_request(marker, Action::Click));
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(marker));
    assert_eq!(
        app.world()
            .resource::<LeaderAiInteractionState>()
            .selected_task_id
            .as_deref(),
        Some("task:one")
    );

    app.world_mut()
        .write_message(action_request(reload, Action::Click));
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(reload));
    assert_eq!(
        app.world()
            .resource::<LeaderAiInteractionState>()
            .reload_requests,
        1
    );
}

#[test]
fn disabled_semantic_control_rejects_accesskit_focus_and_click() {
    let mut app = app();
    let disabled = app
        .world_mut()
        .spawn((
            semantic_node(
                Role::Button,
                "lai-ui:plans:control:move-up:stale-plan",
                "Move stale plan up",
                false,
            ),
            LeaderAiSemanticNode {
                semantic_id: "lai-ui:plans:control:move-up:stale-plan".to_owned(),
                focus_order: 10,
                enabled: false,
            },
            LeaderAiActionButton {
                envelope: envelope(),
                label: "Move stale plan up".to_owned(),
                test_id: "lai-ui:plans:control:move-up:stale-plan".to_owned(),
            },
        ))
        .id();
    app.world_mut()
        .write_message(action_request(disabled, Action::Focus));
    app.world_mut()
        .write_message(action_request(disabled, Action::Click));
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
    assert!(
        app.world()
            .resource::<LeaderAiLiveState>()
            .outbound
            .is_empty()
    );
}
