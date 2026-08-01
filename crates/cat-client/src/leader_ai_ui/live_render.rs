//! Bevy entity bridge for live report-safe leader-AI UI and world footprints.

use std::collections::HashMap;

use accesskit::Role;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_input_focus::{FocusCause, InputFocus, tab_navigation::TabIndex};
use cat_protocol::{
    DiplomacyRelationshipTarget, DismissalReason, SiteRefActionTarget, SiteRefSnapshot,
    TradeRejectionReason,
};

use super::{
    AuthenticatedPlayerIdentity, CatCareAction, CatCarePanelInput, DivineBoostKind,
    ExpectedCatCareVersion, ExpectedCatCareVersionBundle, ExpectedDomainVersion,
    ExpectedPlannerVersion, ExpectedProstheticVersion, ExpectedReservationVersion,
    ExpectedResourceVersion, ExpectedVersionBundle, LeaderAiActionButton, LeaderAiLocalAction,
    LeaderAiLocalButton, LeaderAiPlanNudgeAction, LeaderAiSelectionButton, LeaderAiSelectionKind,
    LeaderAiSemanticNode, LeaderAiStandingOrderAction, PlansPanelInput, ProgressionAction,
    ProgressionExpectedBoostVersion, ProgressionExpectedDiplomacyVersion,
    ProgressionExpectedPlannerVersion, ProgressionExpectedResearchVersion,
    ProgressionExpectedResourceVersion, ProgressionExpectedScholarVersion,
    ProgressionExpectedTradeVersion, ProgressionExpectedVersionBundle, ProgressionPanelInput,
    ProgressionStableIdempotencyId, StableIdempotencyId, StandingOrderDraft,
    VisibleTaskMarkerInput, VisibleTaskSnapshotMarkerSource, build_cat_care_action_envelope,
    build_progression_action_envelope, build_standing_order_action_envelope,
    project_visible_task_footprints, semantic_node, semantic_status_node,
    send_expected_version_action,
};
use crate::{LeaderAiConnectionState, LeaderAiFeedback, LeaderAiLiveState};

#[derive(Default)]
pub struct LeaderAiLiveRenderPlugin;

impl Plugin for LeaderAiLiveRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LeaderAiLiveState>()
            .init_resource::<super::super::leader_ai_live::LeaderAiSelectedColonyResource>()
            .init_resource::<super::super::leader_ai_live::LeaderAiVersionResource>()
            .add_systems(
                Update,
                (
                    sync_live_projection_inputs,
                    sync_live_entities,
                    style_live_text,
                    sync_leader_ai_accessibility,
                    restore_leader_ai_focus,
                )
                    .chain(),
            );
    }
}

type NewLeaderAiTextQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut TextFont,
        Option<&'static LeaderAiActionButton>,
        Option<&'static LeaderAiSelectionButton>,
        Option<&'static LeaderAiLocalButton>,
    ),
    (Added<LeaderAiLiveSurfaceEntity>, With<Text>),
>;

fn style_live_text(mut text: NewLeaderAiTextQuery<'_, '_>) {
    for (mut font, action, selection, local) in &mut text {
        if action.is_some() || selection.is_some() || local.is_some() {
            font.font_size = FontSize::Px(12.0);
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
struct LeaderAiLiveSurfaceEntity;

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct LeaderAiPanelEntity {
    pub domain: String,
    pub test_id: String,
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct LeaderAiRowEntity {
    pub domain: String,
    pub row_id: String,
    pub test_id: String,
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct LeaderAiWorldMarkerEntity {
    pub task_id: String,
    pub site_id: String,
    pub role: String,
    pub test_id: String,
    pub label: String,
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
struct LeaderAiStatusEntity {
    test_id: String,
    label: String,
    assertive: bool,
}

type UnsemanticSelection = (
    Without<LeaderAiSemanticNode>,
    Without<LeaderAiWorldMarkerEntity>,
);

#[derive(SystemParam)]
struct LeaderAiSemanticTargets<'w, 's> {
    panels: Query<'w, 's, (Entity, &'static LeaderAiPanelEntity), Without<LeaderAiSemanticNode>>,
    rows: Query<'w, 's, (Entity, &'static LeaderAiRowEntity), Without<LeaderAiSemanticNode>>,
    actions: Query<'w, 's, (Entity, &'static LeaderAiActionButton), Without<LeaderAiSemanticNode>>,
    selections: Query<'w, 's, (Entity, &'static LeaderAiSelectionButton), UnsemanticSelection>,
    markers:
        Query<'w, 's, (Entity, &'static LeaderAiWorldMarkerEntity), Without<LeaderAiSemanticNode>>,
    statuses: Query<'w, 's, (Entity, &'static LeaderAiStatusEntity), Without<LeaderAiSemanticNode>>,
    local_actions:
        Query<'w, 's, (Entity, &'static LeaderAiLocalButton), Without<LeaderAiSemanticNode>>,
}

fn selected_colony(state: &LeaderAiLiveState) -> Option<&cat_protocol::ColonyAiSnapshot> {
    let snapshot = state.snapshot.as_ref()?;
    snapshot
        .colonies
        .iter()
        .find(|colony| Some(colony.colony_id.as_str()) == state.selected_colony_id.as_deref())
}

fn sync_live_projection_inputs(
    state: Res<'_, LeaderAiLiveState>,
    mut plans: ResMut<'_, PlansPanelInput>,
    mut care: ResMut<'_, CatCarePanelInput>,
    mut progression: ResMut<'_, ProgressionPanelInput>,
    mut markers: ResMut<'_, VisibleTaskMarkerInput>,
    mut selected: ResMut<'_, super::super::leader_ai_live::LeaderAiSelectedColonyResource>,
    mut versions: ResMut<'_, super::super::leader_ai_live::LeaderAiVersionResource>,
) {
    let Some(colony) = selected_colony(&state).cloned() else {
        plans.colony = None;
        care.colony = None;
        progression.colony = None;
        markers.tasks.clear();
        selected.colony_id = None;
        selected.state_version = None;
        return;
    };
    selected.colony_id = Some(colony.colony_id.as_str().to_owned());
    selected.state_version = Some(colony.state_version);
    versions.state_version = Some(colony.state_version);
    versions.planner_version = colony.action_versions.planner_version;
    versions.research_version = colony.action_versions.research_version;
    versions.boost_version = colony.action_versions.boost_version;
    versions.diplomacy_version = colony.action_versions.diplomacy_version;
    versions.trade_version = colony.action_versions.trade_version;
    let colony_id = colony.colony_id.as_str().to_owned();
    plans.selected_colony_id = Some(colony_id.clone());
    plans.colony = Some(colony.clone());
    care.selected_colony_id = Some(colony_id.clone());
    care.colony = Some(colony.clone());
    progression.selected_colony_id = Some(colony_id.clone());
    progression.colony = Some(colony.clone());
    markers.selected_colony_id = Some(colony_id.clone());
    markers.colony_id = Some(colony_id);
    markers.tasks = colony.visible_tasks.clone();
}

fn sync_live_entities(
    mut commands: Commands<'_, '_>,
    state: Res<'_, LeaderAiLiveState>,
    surfaces: Query<'_, '_, Entity, (With<LeaderAiLiveSurfaceEntity>, Without<ChildOf>)>,
) {
    if !state.is_changed() {
        return;
    }
    for entity in &surfaces {
        commands.entity(entity).despawn();
    }
    spawn_connection_status(&mut commands, &state);
    let Some(colony) = selected_colony(&state) else {
        return;
    };

    // Protocol-v2 is a report-first council surface rather than the retired
    // legacy world HUD. Give it one opaque, intentional workspace so live
    // reports never become loose text over unrelated legacy controls.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.075, 0.066, 0.052)),
        GlobalZIndex(400),
        Name::new("Leader AI council workspace"),
        LeaderAiLiveSurfaceEntity,
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(10.0),
            min_width: Val::Px(720.0),
            min_height: Val::Px(32.0),
            ..default()
        },
        Text::new(format!(
            "Colony council  ·  {}  ·  report-safe command ledger",
            colony.colony_id.as_str()
        )),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::srgb(0.94, 0.86, 0.68)),
        GlobalZIndex(420),
        Name::new("Leader AI council heading"),
        LeaderAiLiveSurfaceEntity,
    ));

    let domains = [
        ("plans", "lai-ui:plans:panel"),
        ("standing-orders", "lai-ui:standing-orders:panel"),
        ("officers", "lai-ui:officers:panel"),
        ("tasks", "lai-ui:tasks:panel"),
        ("care", "lai-ui:care:panel"),
        ("shrine", "lai-ui:shrine:panel"),
        ("favor", "lai-ui:favor:panel"),
        ("research", "lai-ui:research:panel"),
        ("scholars", "lai-ui:scholars:panel"),
        ("boosts", "lai-ui:boosts:panel"),
        ("diplomacy", "lai-ui:diplomacy:panel"),
        ("trade", "lai-ui:trade:panel"),
    ];
    let mut panel_entities = HashMap::new();
    for (domain, test_id) in domains {
        let (left, top) = panel_position(domain);
        let (width, height) = panel_size(domain);
        let entity = commands
            .spawn((
                Node {
                    width: Val::Px(width),
                    height: Val::Px(height),
                    position_type: PositionType::Absolute,
                    left: Val::Px(left),
                    top: Val::Px(top),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(3.0),
                    padding: UiRect::all(Val::Px(7.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.14, 0.125, 0.10)),
                BorderColor::all(Color::srgb(0.38, 0.31, 0.22)),
                GlobalZIndex(410),
                LeaderAiPanelEntity {
                    domain: domain.to_owned(),
                    test_id: test_id.to_owned(),
                },
                Name::new(format!("Leader AI {domain} panel")),
                LeaderAiLiveSurfaceEntity,
            ))
            .with_child((
                Text::new(panel_label(domain)),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.91, 0.78, 0.54)),
                Name::new(panel_label(domain)),
            ))
            .id();
        panel_entities.insert(domain, entity);
    }
    for (domain, rows) in [
        (
            "plans",
            colony
                .plans
                .plans
                .iter()
                .map(|plan| plan.plan_id.as_str().to_owned())
                .collect::<Vec<_>>(),
        ),
        (
            "standing-orders",
            colony
                .standing_orders
                .iter()
                .map(|order| order.order_id.as_str().to_owned())
                .collect(),
        ),
        (
            "officers",
            colony
                .officer_requests
                .iter()
                .map(|request| request.request_id.as_str().to_owned())
                .collect(),
        ),
        (
            "tasks",
            colony
                .visible_tasks
                .iter()
                .map(|task| task.task_id.as_str().to_owned())
                .collect(),
        ),
        (
            "care",
            colony
                .cats
                .iter()
                .map(|cat| cat.cat_id.as_str().to_owned())
                .collect(),
        ),
        (
            "shrine",
            colony
                .shrine
                .pipeline
                .iter()
                .map(|offering| offering.offering_id.as_str().to_owned())
                .collect(),
        ),
        (
            "favor",
            colony
                .favor
                .favor_events
                .iter()
                .map(|event| event.event_id.as_str().to_owned())
                .collect(),
        ),
        (
            "research",
            colony
                .research
                .frontier
                .iter()
                .map(|study| study.study_id.as_str().to_owned())
                .collect(),
        ),
        (
            "scholars",
            colony
                .research
                .preparations
                .iter()
                .map(|preparation| preparation.preparation_id.as_str().to_owned())
                .collect(),
        ),
        (
            "boosts",
            colony
                .boosts
                .iter()
                .map(|boost| boost.boost_id.as_str().to_owned())
                .collect(),
        ),
        (
            "diplomacy",
            colony
                .diplomacy
                .relationships
                .iter()
                .map(|relationship| relationship.relationship_id.as_str().to_owned())
                .collect(),
        ),
        (
            "trade",
            colony
                .trade
                .iter()
                .map(|contract| contract.contract_id.as_str().to_owned())
                .collect(),
        ),
    ] {
        let Some(parent) = panel_entities.get(domain).copied() else {
            continue;
        };
        for row_id in rows.into_iter().take(visible_row_limit(domain)) {
            let test_id =
                super::TestIdBuilder::named_row(domain, row_entity_kind(domain), row_id.as_str())
                    .as_str()
                    .to_owned();
            commands.spawn((
                Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(32.0),
                    padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.20, 0.18, 0.145)),
                LeaderAiRowEntity {
                    domain: domain.to_owned(),
                    row_id: row_id.clone(),
                    test_id,
                },
                Text::new(visual_row_label(domain, row_id.as_str())),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::srgb(0.89, 0.85, 0.76)),
                Name::new(format!("Leader AI {domain} report row")),
                ChildOf(parent),
                LeaderAiLiveSurfaceEntity,
            ));
        }
    }

    // Every enabled button below carries a fully formed LAI.25 envelope. If
    // the session is not authenticated, no mutation button is shown; this
    // keeps the surface report-safe instead of rendering inert affordances.
    if state.authenticated_player_id.is_some()
        && state.connection == LeaderAiConnectionState::Connected
    {
        if let Some(identity) = live_identity(&state, colony)
            && let Some(expected_versions) = plan_expected_versions(colony)
            && let Ok(envelope) = build_standing_order_action_envelope(
                identity,
                StableIdempotencyId(stable_action_id(
                    "standing-order:create",
                    "new",
                    colony.state_version,
                )),
                expected_versions,
                LeaderAiStandingOrderAction::Create(StandingOrderDraft {
                    order_kind: "gather".to_owned(),
                    domain: "workforce".to_owned(),
                    target_id: None,
                    instruction: "Prioritize the selected report-safe objective".to_owned(),
                    priority_basis_points: 5_000,
                    expires_at_ms: None,
                }),
            )
        {
            let parent = panel_entities["standing-orders"];
            commands.spawn((
                Button,
                Interaction::None,
                control_node(128.0, 0.0, 32.0),
                BackgroundColor(Color::srgb(0.55, 0.34, 0.19)),
                LeaderAiActionButton {
                    envelope,
                    label: "Create standing order".to_owned(),
                    test_id: super::TestIdBuilder::named_control(
                        "standing-orders",
                        super::ControlKind::Create,
                        "new",
                    )
                    .as_str()
                    .to_owned(),
                },
                Text::new("Create standing order"),
                Name::new("Create standing order"),
                ChildOf(parent),
                LeaderAiLiveSurfaceEntity,
            ));
        }
        for order in colony.standing_orders.iter().take(1) {
            let order_id = order.order_id.as_str();
            let Some(versions) = plan_expected_versions(colony) else {
                continue;
            };
            for (label, action, suffix) in [
                (
                    "Edit",
                    LeaderAiStandingOrderAction::Update {
                        standing_order_id: order_id.to_owned(),
                        patch: super::StandingOrderDraftPatch {
                            priority_basis_points: Some(
                                if order.priority_basis_points.get() >= 9_500 {
                                    order.priority_basis_points.get().saturating_sub(500)
                                } else {
                                    order.priority_basis_points.get().saturating_add(500)
                                },
                            ),
                            ..default()
                        },
                    },
                    "edit",
                ),
                (
                    "Delete",
                    LeaderAiStandingOrderAction::Delete {
                        standing_order_id: order_id.to_owned(),
                    },
                    "delete",
                ),
            ] {
                let Some(identity) = live_identity(&state, colony) else {
                    continue;
                };
                let Ok(envelope) = build_standing_order_action_envelope(
                    identity,
                    StableIdempotencyId(stable_action_id(
                        if suffix == "edit" {
                            "standing-order:edit"
                        } else {
                            "standing-order:delete"
                        },
                        order_id,
                        colony.state_version,
                    )),
                    versions,
                    action,
                ) else {
                    continue;
                };
                commands.spawn((
                    Button,
                    Interaction::None,
                    control_node(96.0, if suffix == "edit" { 0.0 } else { 104.0 }, 68.0),
                    BackgroundColor(Color::srgb(0.55, 0.34, 0.19)),
                    LeaderAiActionButton {
                        envelope,
                        label: format!("{label} standing order {order_id}"),
                        test_id: super::TestIdBuilder::named_control(
                            "standing-orders",
                            if suffix == "edit" {
                                super::ControlKind::Edit
                            } else {
                                super::ControlKind::Delete
                            },
                            order_id,
                        )
                        .as_str()
                        .to_owned(),
                    },
                    Text::new(label),
                    Name::new(format!("{label} standing order")),
                    ChildOf(panel_entities["standing-orders"]),
                    LeaderAiLiveSurfaceEntity,
                ));
            }
        }
        for plan in colony.plans.plans.iter().take(1) {
            let plan_id = plan.plan_id.as_str();
            for (label, action, suffix) in [
                (
                    "Move up",
                    LeaderAiPlanNudgeAction::MoveUp {
                        plan_id: plan_id.to_owned(),
                        reason_key: None,
                    },
                    "up",
                ),
                (
                    "Move down",
                    LeaderAiPlanNudgeAction::MoveDown {
                        plan_id: plan_id.to_owned(),
                        reason_key: None,
                    },
                    "down",
                ),
                (
                    "Dismiss",
                    LeaderAiPlanNudgeAction::Dismiss {
                        intent_id: plan.intent_id.as_str().to_owned(),
                        planning_epoch: colony.plans.planning_epoch,
                        reason: DismissalReason::PlayerPriority,
                    },
                    "dismiss",
                ),
            ] {
                if let Some(envelope) = plan_envelope(&state, colony, plan_id, action) {
                    commands.spawn((
                        Button,
                        Interaction::None,
                        control_node(
                            96.0,
                            match suffix {
                                "up" => 0.0,
                                "down" => 104.0,
                                _ => 208.0,
                            },
                            52.0,
                        ),
                        BackgroundColor(Color::srgb(0.55, 0.34, 0.19)),
                        LeaderAiActionButton {
                            envelope,
                            label: format!("{label} {plan_id}"),
                            test_id: super::TestIdBuilder::control(
                                super::UiSection::Plans,
                                match suffix {
                                    "up" => super::ControlKind::MoveUp,
                                    "down" => super::ControlKind::MoveDown,
                                    _ => super::ControlKind::Dismiss,
                                },
                                plan_id,
                            )
                            .as_str()
                            .to_owned(),
                        },
                        Text::new(label),
                        Name::new(format!("{label} plan")),
                        ChildOf(panel_entities["plans"]),
                        LeaderAiLiveSurfaceEntity,
                    ));
                }
            }
        }

        for study in colony.research.frontier.iter().take(1) {
            if let Some(envelope) = progression_envelope(
                &state,
                colony,
                study.study_id.as_str(),
                ProgressionAction::PurchaseResearch {
                    study_id: study.study_id.as_str().to_owned(),
                    use_preparation: study.prepared_price_micro_favor.is_some(),
                    displayed_price_micro_favor: Some(
                        study
                            .prepared_price_micro_favor
                            .unwrap_or(study.price_micro_favor),
                    ),
                },
            ) {
                commands.spawn((
                    Button,
                    Interaction::None,
                    control_node(128.0, 0.0, 32.0),
                    BackgroundColor(Color::srgb(0.55, 0.34, 0.19)),
                    LeaderAiActionButton {
                        envelope,
                        label: format!("Purchase {}", study.display_name.as_str()),
                        test_id: super::TestIdBuilder::control(
                            super::UiSection::Progression,
                            super::ControlKind::Purchase,
                            study.study_id.as_str(),
                        )
                        .as_str()
                        .to_owned(),
                    },
                    Text::new("Purchase research"),
                    Name::new("Purchase research"),
                    ChildOf(panel_entities["research"]),
                    LeaderAiLiveSurfaceEntity,
                ));
            }
        }

        for boost_kind in DivineBoostKind::ALL
            .into_iter()
            .filter(|boost_kind| {
                !colony
                    .boosts
                    .iter()
                    .any(|boost| boost.boost_kind.as_str() == boost_kind.protocol_id())
            })
            .take(1)
        {
            if let Some(envelope) = progression_envelope(
                &state,
                colony,
                boost_kind.protocol_id(),
                ProgressionAction::ActivateDivineBoost {
                    boost_kind,
                    duration_hours: 1,
                    displayed_price_micro_favor: None,
                },
            ) {
                commands.spawn((
                    Button,
                    Interaction::None,
                    control_node(128.0, 0.0, 32.0),
                    BackgroundColor(Color::srgb(0.55, 0.34, 0.19)),
                    LeaderAiActionButton {
                        envelope,
                        label: format!("Activate {}", boost_kind.protocol_id()),
                        test_id: super::TestIdBuilder::control(
                            super::UiSection::Progression,
                            super::ControlKind::Activate,
                            boost_kind.protocol_id(),
                        )
                        .as_str()
                        .to_owned(),
                    },
                    Text::new("Activate boost"),
                    Name::new("Activate divine boost"),
                    ChildOf(panel_entities["boosts"]),
                    LeaderAiLiveSurfaceEntity,
                ));
            }
        }

        for relationship in colony.diplomacy.relationships.iter().take(1) {
            if let Some(envelope) = progression_envelope(
                &state,
                colony,
                relationship.relationship_id.as_str(),
                ProgressionAction::ChangeDiplomacy {
                    target_colony_id: relationship.other_colony_id.as_str().to_owned(),
                    relationship: DiplomacyRelationshipTarget::Friendly,
                },
            ) {
                commands.spawn((
                    Button,
                    Interaction::None,
                    control_node(128.0, 0.0, 32.0),
                    BackgroundColor(Color::srgb(0.55, 0.34, 0.19)),
                    LeaderAiActionButton {
                        envelope,
                        label: format!("Propose {}", relationship.other_colony_id.as_str()),
                        test_id: super::TestIdBuilder::named_control(
                            "diplomacy",
                            super::ControlKind::Propose,
                            relationship.relationship_id.as_str(),
                        )
                        .as_str()
                        .to_owned(),
                    },
                    Text::new("Propose relationship"),
                    Name::new("Propose diplomatic relationship"),
                    ChildOf(panel_entities["diplomacy"]),
                    LeaderAiLiveSurfaceEntity,
                ));
            }
        }

        for contract in colony
            .trade
            .iter()
            .filter(|contract| {
                matches!(
                    contract.stage,
                    cat_protocol::TradeStageSnapshot::Proposed
                        | cat_protocol::TradeStageSnapshot::AwaitingConsent
                )
            })
            .take(1)
        {
            for (label, control_kind, action, left) in [
                (
                    "Accept trade",
                    super::ControlKind::Accept,
                    ProgressionAction::AcceptTradeContract {
                        contract_id: contract.contract_id.as_str().to_owned(),
                    },
                    0.0,
                ),
                (
                    "Reject trade",
                    super::ControlKind::Reject,
                    ProgressionAction::RejectTradeContract {
                        contract_id: contract.contract_id.as_str().to_owned(),
                        reason: TradeRejectionReason::TermsDeclined,
                    },
                    136.0,
                ),
            ] {
                let Some(envelope) =
                    progression_envelope(&state, colony, contract.contract_id.as_str(), action)
                else {
                    continue;
                };
                commands.spawn((
                    Button,
                    Interaction::None,
                    control_node(128.0, left, 32.0),
                    BackgroundColor(Color::srgb(0.55, 0.34, 0.19)),
                    LeaderAiActionButton {
                        envelope,
                        label: format!("{label} {}", contract.contract_id.as_str()),
                        test_id: super::TestIdBuilder::named_control(
                            "trade",
                            control_kind,
                            contract.contract_id.as_str(),
                        )
                        .as_str()
                        .to_owned(),
                    },
                    Text::new(label),
                    Name::new("Respond to trade contract"),
                    ChildOf(panel_entities["trade"]),
                    LeaderAiLiveSurfaceEntity,
                ));
            }
        }

        for cat in &colony.cats {
            for body_part in &cat.anatomy.body_parts {
                if let Some(injury) = &body_part.injury
                    && injury.treatment.is_none()
                    && let Some(envelope) = care_envelope(
                        &state,
                        colony,
                        cat,
                        format!("treatment:{}", injury.injury_id.as_str()),
                        CatCareAction::RequestTreatment {
                            cat_id: cat.cat_id.as_str().to_owned(),
                            injury_id: injury.injury_id.as_str().to_owned(),
                            treatment_kind: "standard_care".to_owned(),
                        },
                    )
                {
                    commands.spawn((
                        Button,
                        Interaction::None,
                        control_node(128.0, 0.0, 32.0),
                        BackgroundColor(Color::srgb(0.55, 0.34, 0.19)),
                        LeaderAiActionButton {
                            envelope,
                            label: format!("Treat {}", cat.display_name.as_str()),
                            test_id: super::TestIdBuilder::control(
                                super::UiSection::Cats,
                                super::ControlKind::Treat,
                                injury.injury_id.as_str(),
                            )
                            .as_str()
                            .to_owned(),
                        },
                        Text::new("Request treatment"),
                        Name::new("Request treatment"),
                        ChildOf(panel_entities["care"]),
                        LeaderAiLiveSurfaceEntity,
                    ));
                }
            }
            if let Some(site) = cat.care.care_site.as_ref().and_then(action_site_target) {
                for prosthetic in &cat.prosthetics {
                    if prosthetic.fitting_task_id.is_none()
                        && let Some(envelope) = care_envelope(
                            &state,
                            colony,
                            cat,
                            format!("fit:{}", prosthetic.prosthetic_id.as_str()),
                            CatCareAction::FitProsthetic {
                                cat_id: cat.cat_id.as_str().to_owned(),
                                prosthetic_id: prosthetic.prosthetic_id.as_str().to_owned(),
                                body_part_id: prosthetic.body_part_id.as_str().to_owned(),
                                fitting_site: site.clone(),
                                fitter_cat_id: None,
                            },
                        )
                    {
                        commands.spawn((
                            Button,
                            Interaction::None,
                            control_node(128.0, 136.0, 32.0),
                            BackgroundColor(Color::srgb(0.55, 0.34, 0.19)),
                            LeaderAiActionButton {
                                envelope,
                                label: format!("Fit {}", prosthetic.prosthetic_kind.as_str()),
                                test_id: super::TestIdBuilder::control(
                                    super::UiSection::Cats,
                                    super::ControlKind::Fit,
                                    prosthetic.prosthetic_id.as_str(),
                                )
                                .as_str()
                                .to_owned(),
                            },
                            Text::new("Fit prosthetic"),
                            Name::new("Fit prosthetic"),
                            ChildOf(panel_entities["care"]),
                            LeaderAiLiveSurfaceEntity,
                        ));
                    }
                }
            }
        }
    }

    // Selection is always available from authoritative IDs and never queues a
    // mutation. These controls preserve the selected object across refreshes.
    for task in colony.visible_tasks.iter().take(1) {
        commands.spawn((
            Button,
            Interaction::None,
            control_node(128.0, 0.0, 32.0),
            BackgroundColor(Color::srgb(0.78, 0.72, 0.58)),
            LeaderAiSelectionButton {
                kind: LeaderAiSelectionKind::Task,
                stable_id: task.task_id.as_str().to_owned(),
                label: format!("Select task {}", task.task_id.as_str()),
                test_id: super::TestIdBuilder::named_control(
                    "tasks",
                    super::ControlKind::Select,
                    task.task_id.as_str(),
                )
                .as_str()
                .to_owned(),
            },
            Text::new("Select task"),
            Name::new("Select visible task"),
            ChildOf(panel_entities["tasks"]),
            LeaderAiLiveSurfaceEntity,
        ));
    }
    if let Ok(projections) = project_visible_task_footprints(VisibleTaskSnapshotMarkerSource {
        tasks: &colony.visible_tasks,
    }) {
        for projection in projections {
            for marker in projection.markers {
                let tile = marker.tile;
                commands.spawn((
                    Sprite::from_color(Color::srgb(0.62, 0.30, 0.16), Vec2::splat(8.0)),
                    Transform::from_xyz(tile.x as f32 * 10.0, -(tile.y as f32) * 10.0, 650.0),
                    LeaderAiWorldMarkerEntity {
                        task_id: marker.task_id.clone(),
                        site_id: marker.key.site_id.clone(),
                        role: marker.key.role.clone(),
                        test_id: marker.test_id.as_str().to_owned(),
                        label: marker.label.as_str().to_owned(),
                    },
                    LeaderAiSelectionButton {
                        kind: LeaderAiSelectionKind::Task,
                        stable_id: marker.task_id,
                        label: marker.label.as_str().to_owned(),
                        test_id: marker.test_id.as_str().to_owned(),
                    },
                    Name::new("Visible task world marker"),
                    LeaderAiLiveSurfaceEntity,
                ));
            }
        }
    }
}

/// Attach real platform accessibility nodes after authoritative entities spawn.
/// Semantic IDs are descriptions, not DOM-only test hooks or hidden state.
fn sync_leader_ai_accessibility(
    mut commands: Commands<'_, '_>,
    targets: LeaderAiSemanticTargets<'_, '_>,
) {
    for (entity, panel) in targets.panels.iter() {
        let order = panel_order(&panel.domain);
        commands.entity(entity).insert((
            semantic_node(
                Role::Pane,
                panel.test_id.clone(),
                format!("{} panel", panel.domain),
                true,
            ),
            LeaderAiSemanticNode {
                semantic_id: panel.test_id.clone(),
                focus_order: order,
                enabled: true,
            },
        ));
    }
    for (entity, row) in targets.rows.iter() {
        commands.entity(entity).insert((
            semantic_node(
                Role::ListItem,
                row.test_id.clone(),
                format!("{} report row {}", row.domain, row.row_id),
                true,
            ),
            LeaderAiSemanticNode {
                semantic_id: row.test_id.clone(),
                focus_order: 100,
                enabled: true,
            },
        ));
    }
    for (entity, action) in targets.actions.iter() {
        commands.entity(entity).insert((
            semantic_node(
                Role::Button,
                action.test_id.clone(),
                action.label.clone(),
                true,
            ),
            LeaderAiSemanticNode {
                semantic_id: action.test_id.clone(),
                focus_order: interactive_order(&action.test_id, 200),
                enabled: true,
            },
            TabIndex(i32::try_from(interactive_order(&action.test_id, 200)).unwrap_or(i32::MAX)),
        ));
    }
    for (entity, selection) in targets.selections.iter() {
        commands.entity(entity).insert((
            semantic_node(
                Role::Button,
                selection.test_id.clone(),
                selection.label.clone(),
                true,
            ),
            LeaderAiSemanticNode {
                semantic_id: selection.test_id.clone(),
                focus_order: interactive_order(&selection.test_id, 300),
                enabled: true,
            },
            TabIndex(i32::try_from(interactive_order(&selection.test_id, 300)).unwrap_or(i32::MAX)),
        ));
    }
    for (entity, marker) in targets.markers.iter() {
        commands.entity(entity).insert((
            semantic_node(
                Role::Button,
                marker.test_id.clone(),
                marker.label.clone(),
                true,
            ),
            LeaderAiSemanticNode {
                semantic_id: marker.test_id.clone(),
                focus_order: 400,
                enabled: true,
            },
            TabIndex(12_400),
        ));
    }
    for (entity, status) in targets.statuses.iter() {
        commands.entity(entity).insert((
            semantic_status_node(
                status.test_id.clone(),
                status.label.clone(),
                status.assertive,
            ),
            LeaderAiSemanticNode {
                semantic_id: status.test_id.clone(),
                focus_order: 0,
                enabled: true,
            },
        ));
    }
    for (entity, button) in targets.local_actions.iter() {
        let order = interactive_order(&button.test_id, 900);
        commands.entity(entity).insert((
            semantic_node(
                Role::Button,
                button.test_id.clone(),
                button.label.clone(),
                true,
            ),
            LeaderAiSemanticNode {
                semantic_id: button.test_id.clone(),
                focus_order: order,
                enabled: true,
            },
            TabIndex(i32::try_from(order).unwrap_or(i32::MAX)),
        ));
    }
}

fn panel_order(domain: &str) -> u32 {
    [
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
    ]
    .iter()
    .position(|candidate| *candidate == domain)
    .map_or(0, |index| index as u32)
}

fn interactive_order(test_id: &str, offset: u32) -> u32 {
    let domain = test_id
        .strip_prefix("lai-ui:")
        .and_then(|rest| rest.split(':').next())
        .unwrap_or("connection");
    panel_order(domain)
        .saturating_mul(1_000)
        .saturating_add(offset)
}

fn restore_leader_ai_focus(
    mut state: ResMut<'_, super::LeaderAiInteractionState>,
    semantics: Query<'_, '_, (Entity, &LeaderAiSemanticNode)>,
    mut focus: ResMut<'_, InputFocus>,
) {
    let Some(focused_test_id) = state.focused_test_id.as_deref() else {
        return;
    };
    if let Some((entity, _)) = semantics
        .iter()
        .find(|(_, semantic)| semantic.enabled && semantic.semantic_id == focused_test_id)
    {
        if focus.get() != Some(entity) {
            focus.set(entity, FocusCause::Navigated);
        }
        return;
    }
    let Some(panel_id) = parent_panel_id(focused_test_id) else {
        focus.clear();
        state.focused_test_id = None;
        return;
    };
    if let Some((entity, semantic)) = semantics
        .iter()
        .find(|(_, semantic)| semantic.semantic_id == panel_id)
    {
        focus.set(entity, FocusCause::Navigated);
        state.focused_test_id = Some(semantic.semantic_id.clone());
    } else {
        focus.clear();
        state.focused_test_id = None;
    }
}

fn parent_panel_id(test_id: &str) -> Option<String> {
    let rest = test_id.strip_prefix("lai-ui:")?;
    let domain = rest.split(':').next()?;
    Some(format!("lai-ui:{domain}:panel"))
}

fn spawn_connection_status(commands: &mut Commands<'_, '_>, state: &LeaderAiLiveState) {
    let (label, assertive) = match state.connection {
        LeaderAiConnectionState::Connected => {
            ("Connected to authoritative world".to_owned(), false)
        }
        LeaderAiConnectionState::Disconnected if state.snapshot.is_some() => (
            "Reconnecting. Showing the last report-safe snapshot; actions are unavailable."
                .to_owned(),
            true,
        ),
        LeaderAiConnectionState::Disconnected => {
            ("Waiting for an authoritative snapshot".to_owned(), false)
        }
        LeaderAiConnectionState::UpdateRequired => (
            "Update required. Reload the client to continue.".to_owned(),
            true,
        ),
    };
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(8.0),
            bottom: Val::Px(8.0),
            min_width: Val::Px(360.0),
            min_height: Val::Px(30.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.16, 0.14, 0.11)),
        GlobalZIndex(420),
        Text::new(label.clone()),
        Name::new("Leader AI connection status"),
        LeaderAiStatusEntity {
            test_id: "lai-connection:status".to_owned(),
            label,
            assertive,
        },
        LeaderAiLiveSurfaceEntity,
    ));
    if let Some(colony_id) = state.selected_colony_id.as_deref() {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                top: Val::Px(12.0),
                min_width: Val::Px(260.0),
                min_height: Val::Px(28.0),
                ..default()
            },
            Text::new(format!("Selected colony {colony_id}")),
            TextColor(Color::srgb(0.78, 0.72, 0.60)),
            GlobalZIndex(420),
            Name::new("Selected colony"),
            LeaderAiStatusEntity {
                test_id: "lai-colony:selected".to_owned(),
                label: format!("Selected colony {colony_id}"),
                assertive: false,
            },
            LeaderAiLiveSurfaceEntity,
        ));
    }
    if state.connection != LeaderAiConnectionState::Connected {
        commands.spawn((
            Button,
            Interaction::None,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(8.0),
                bottom: Val::Px(8.0),
                min_width: Val::Px(128.0),
                min_height: Val::Px(30.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.55, 0.34, 0.19)),
            Text::new("Reload"),
            GlobalZIndex(420),
            Name::new("Reload leader AI client"),
            LeaderAiLocalButton {
                action: LeaderAiLocalAction::Reload,
                label: "Reload the client".to_owned(),
                test_id: "lai-ui:connection:control:reload".to_owned(),
            },
            LeaderAiLiveSurfaceEntity,
        ));
    }
    if let Some(feedback) = state.feedback.back() {
        let (test_id, label, assertive) = match feedback {
            LeaderAiFeedback::Accepted { .. } => (
                "lai-feedback:action:accepted",
                "Action accepted by the authoritative server",
                false,
            ),
            LeaderAiFeedback::Rejected { .. } => (
                "lai-feedback:action:rejected",
                "Action rejected; the report will refresh",
                true,
            ),
            LeaderAiFeedback::Duplicate { .. } => (
                "lai-feedback:action:duplicate",
                "Duplicate action returned its original result",
                false,
            ),
            LeaderAiFeedback::UpdateRequired => (
                "lai-feedback:update-required",
                "Update required before further actions",
                true,
            ),
            LeaderAiFeedback::Reconnecting => (
                "lai-feedback:reconnecting",
                "Reconnecting to the authoritative server",
                true,
            ),
        };
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(400.0),
                bottom: Val::Px(8.0),
                min_width: Val::Px(360.0),
                min_height: Val::Px(30.0),
                ..default()
            },
            Text::new(label),
            GlobalZIndex(420),
            Name::new("Leader AI action feedback"),
            LeaderAiStatusEntity {
                test_id: test_id.to_owned(),
                label: label.to_owned(),
                assertive,
            },
            LeaderAiLiveSurfaceEntity,
        ));
    }
}

fn panel_position(domain: &str) -> (f32, f32) {
    match domain {
        "plans" => (8.0, 50.0),
        "standing-orders" => (8.0, 236.0),
        "officers" => (8.0, 422.0),
        "diplomacy" => (8.0, 526.0),
        "tasks" => (324.0, 50.0),
        "care" => (640.0, 50.0),
        "shrine" => (956.0, 50.0),
        "favor" => (956.0, 142.0),
        "research" => (956.0, 234.0),
        "scholars" => (956.0, 360.0),
        "boosts" => (956.0, 438.0),
        "trade" => (956.0, 526.0),
        _ => (8.0, 50.0 + panel_order(domain) as f32 * 48.0),
    }
}

fn panel_size(domain: &str) -> (f32, f32) {
    match domain {
        "plans" => (308.0, 178.0),
        "standing-orders" => (308.0, 178.0),
        "officers" => (308.0, 96.0),
        "diplomacy" => (308.0, 100.0),
        "tasks" | "care" => (308.0, 576.0),
        "shrine" | "favor" => (316.0, 84.0),
        "research" => (316.0, 118.0),
        "scholars" => (316.0, 70.0),
        "boosts" => (316.0, 80.0),
        "trade" => (316.0, 100.0),
        _ => (308.0, 120.0),
    }
}

fn visible_row_limit(domain: &str) -> usize {
    match domain {
        "tasks" => 12,
        // Keep the two authored urgent-care cats and their real action
        // controls visible. The full list remains in the authoritative
        // snapshot; this fixed council surface is a prioritized ledger.
        "care" => 8,
        // One report row plus Move up/down/Dismiss fits the panel. Rendering
        // three rows first used to clip every plan control.
        "plans" => 1,
        "standing-orders" | "officers" | "shrine" | "favor" | "research" | "scholars"
        | "boosts" | "diplomacy" | "trade" => 1,
        _ => 3,
    }
}

fn control_node(width: f32, _left: f32, _top: f32) -> Node {
    Node {
        width: Val::Px(width),
        min_height: Val::Px(30.0),
        padding: UiRect::axes(Val::Px(7.0), Val::Px(4.0)),
        border: UiRect::all(Val::Px(1.0)),
        ..default()
    }
}

fn panel_label(domain: &str) -> &'static str {
    match domain {
        "plans" => "Plans",
        "standing-orders" => "Standing orders",
        "officers" => "Officer reports",
        "tasks" => "Visible tasks",
        "care" => "Cat care",
        "shrine" => "Shrine offering",
        "favor" => "Favor ledger",
        "research" => "Research frontier",
        "scholars" => "Scholar preparation",
        "boosts" => "Divine boosts",
        "diplomacy" => "Diplomacy",
        "trade" => "Trade contracts",
        _ => "Leader AI",
    }
}

fn row_entity_kind(domain: &str) -> super::EntityKind {
    match domain {
        "plans" => super::EntityKind::Plan,
        "standing-orders" => super::EntityKind::StandingOrder,
        "officers" => super::EntityKind::OfficerReport,
        "tasks" => super::EntityKind::Task,
        "care" => super::EntityKind::Cat,
        "shrine" => super::EntityKind::ShrineOffering,
        "favor" => super::EntityKind::FavorEvent,
        "research" => super::EntityKind::Research,
        "scholars" => super::EntityKind::ScholarPreparation,
        "boosts" => super::EntityKind::Boost,
        "diplomacy" => super::EntityKind::DiplomacyRelationship,
        "trade" => super::EntityKind::TradeContract,
        _ => super::EntityKind::Feedback,
    }
}

fn visual_row_label(domain: &str, row_id: &str) -> String {
    const MAX_LABEL_CHARS: usize = 28;
    let meaningful = decode_planner_components(row_id)
        .and_then(|components| {
            components
                .into_iter()
                .rev()
                .find(|component| {
                    !component
                        .chars()
                        .all(|character| character.is_ascii_digit())
                })
                .map(str::to_owned)
        })
        .unwrap_or_else(|| row_id.to_owned());
    let readable = meaningful.replace(['_', '-'], " ");
    let compact = if readable.chars().count() <= MAX_LABEL_CHARS {
        readable
    } else {
        let prefix = readable
            .chars()
            .take(MAX_LABEL_CHARS - 1)
            .collect::<String>();
        format!("{prefix}…")
    };
    format!("{} · {compact}", panel_label(domain))
}

/// Decode the simulation's lossless `planner:v1|<bytes>:<component>` identity
/// for display only. Action payloads always keep the exact opaque ID.
fn decode_planner_components(encoded: &str) -> Option<Vec<&str>> {
    let mut rest = encoded.strip_prefix("planner:v1")?;
    let mut components = Vec::new();
    while !rest.is_empty() {
        rest = rest.strip_prefix('|')?;
        let colon = rest.find(':')?;
        let byte_len = rest[..colon].parse::<usize>().ok()?;
        rest = &rest[colon + 1..];
        let component = rest.get(..byte_len)?;
        if !component.is_char_boundary(component.len()) {
            return None;
        }
        components.push(component);
        rest = &rest[byte_len..];
    }
    Some(components)
}

fn live_identity(
    state: &LeaderAiLiveState,
    colony: &cat_protocol::ColonyAiSnapshot,
) -> Option<AuthenticatedPlayerIdentity> {
    state
        .authenticated_player_id
        .clone()
        .map(|player_id| AuthenticatedPlayerIdentity {
            colony_id: colony.colony_id.as_str().to_owned(),
            player_id,
        })
}

fn plan_expected_versions(
    colony: &cat_protocol::ColonyAiSnapshot,
) -> Option<ExpectedVersionBundle> {
    let versions = &colony.action_versions;
    Some(ExpectedVersionBundle {
        planner: ExpectedPlannerVersion(versions.planner_version?),
        domain: ExpectedDomainVersion(versions.domain_version?),
        resource: ExpectedResourceVersion(versions.resource_version?),
        reservation: ExpectedReservationVersion(versions.reservation_version),
        standing_order: versions.standing_order_version,
    })
}

fn plan_envelope(
    state: &LeaderAiLiveState,
    colony: &cat_protocol::ColonyAiSnapshot,
    plan_id: &str,
    action: LeaderAiPlanNudgeAction,
) -> Option<cat_protocol::LeaderAiActionEnvelope> {
    let identity = live_identity(state, colony)?;
    let expected_versions = plan_expected_versions(colony)?;
    let action_key = match &action {
        LeaderAiPlanNudgeAction::MoveUp { .. } => "move-up",
        LeaderAiPlanNudgeAction::MoveDown { .. } => "move-down",
        LeaderAiPlanNudgeAction::Dismiss { .. } => "dismiss",
    };
    let result = send_expected_version_action(
        identity,
        StableIdempotencyId(stable_action_id(
            &format!("plan:{action_key}"),
            plan_id,
            colony.state_version,
        )),
        expected_versions,
        action,
    );
    if let Err(error) = &result {
        debug!(
            target: "leader_ai_ui",
            domain = "plans",
            action = action_key,
            entity_id = plan_id,
            ?error,
            "suppressed invalid leader-AI control envelope"
        );
    }
    result.ok()
}

fn progression_envelope(
    state: &LeaderAiLiveState,
    colony: &cat_protocol::ColonyAiSnapshot,
    row_id: &str,
    action: ProgressionAction,
) -> Option<cat_protocol::LeaderAiActionEnvelope> {
    let identity = live_identity(state, colony)?;
    let action_key = match &action {
        ProgressionAction::PurchaseResearch { .. } => "purchase-research",
        ProgressionAction::PrepareScholarStudy { .. } => "prepare-scholar",
        ProgressionAction::ActivateDivineBoost { .. } => "activate-boost",
        ProgressionAction::ChangeDiplomacy { .. } => "change-diplomacy",
        ProgressionAction::ApproveAlliance { .. } => "approve-alliance",
        ProgressionAction::BlockColony { .. } => "block-colony",
        ProgressionAction::AcceptTradeContract { .. } => "accept-trade",
        ProgressionAction::RejectTradeContract { .. } => "reject-trade",
    };
    let action_versions = &colony.action_versions;
    let versions = ProgressionExpectedVersionBundle {
        planner: ProgressionExpectedPlannerVersion(action_versions.planner_version?),
        resource: ProgressionExpectedResourceVersion(action_versions.resource_version?),
        research: action_versions
            .research_version
            .map(ProgressionExpectedResearchVersion),
        scholar: action_versions
            .scholar_version
            .map(ProgressionExpectedScholarVersion),
        boost: action_versions
            .boost_version
            .map(ProgressionExpectedBoostVersion),
        diplomacy: action_versions
            .diplomacy_version
            .map(ProgressionExpectedDiplomacyVersion),
        trade: action_versions
            .trade_version
            .map(ProgressionExpectedTradeVersion),
        reservation: action_versions.reservation_version,
    };
    let result = build_progression_action_envelope(
        identity,
        ProgressionStableIdempotencyId(stable_action_id(
            &format!("progression:{action_key}"),
            row_id,
            colony.state_version,
        )),
        versions,
        action,
    );
    if let Err(error) = &result {
        debug!(
            target: "leader_ai_ui",
            domain = "progression",
            action = action_key,
            entity_id = row_id,
            ?error,
            "suppressed invalid leader-AI control envelope"
        );
    }
    result.ok()
}

fn care_envelope(
    state: &LeaderAiLiveState,
    colony: &cat_protocol::ColonyAiSnapshot,
    _cat: &cat_protocol::CatCareSnapshot,
    key: String,
    action: CatCareAction,
) -> Option<cat_protocol::LeaderAiActionEnvelope> {
    let identity = live_identity(state, colony)?;
    let action_versions = &colony.action_versions;
    let expected = ExpectedCatCareVersionBundle {
        planner_version: action_versions.planner_version?,
        domain_version: action_versions.domain_version?,
        resource_version: action_versions.resource_version?,
        care: ExpectedCatCareVersion(action_versions.care_version?),
        prosthetic: action_versions
            .prosthetic_version
            .map(ExpectedProstheticVersion),
        spatial_version: action_versions.spatial_version,
        reservation_version: action_versions.reservation_version,
    };
    let result = build_cat_care_action_envelope(
        identity,
        StableIdempotencyId(stable_action_id("care", &key, colony.state_version)),
        expected,
        action,
    );
    if let Err(error) = &result {
        debug!(
            target: "leader_ai_ui",
            domain = "care",
            action = key,
            ?error,
            "suppressed invalid leader-AI control envelope"
        );
    }
    result.ok()
}

fn stable_action_id(namespace: &str, subject: &str, version: u64) -> String {
    let candidate = format!("lai:{namespace}:{subject}:{version}");
    // Simulator entity IDs may contain the canonical planner `|length:value`
    // encoding, while action IDs deliberately use a narrower alphabet. Keep a
    // readable candidate only when the protocol type itself accepts it;
    // otherwise deterministically hash the whole lossless subject.
    if cat_protocol::ActionIdempotencyId::new(candidate.clone()).is_ok() {
        return candidate;
    }
    let hash = candidate
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            hash.wrapping_mul(0x0000_0100_0000_01b3) ^ u64::from(byte)
        });
    format!("lai:{namespace}:{hash:016x}:{version}")
}

fn action_site_target(site: &SiteRefSnapshot) -> Option<SiteRefActionTarget> {
    let tile = match site {
        SiteRefSnapshot::Tile { tile, .. }
        | SiteRefSnapshot::Shrine { endpoint: tile, .. }
        | SiteRefSnapshot::VillageEndpoint { endpoint: tile, .. }
        | SiteRefSnapshot::TradeEndpoint { endpoint: tile, .. } => *tile,
        _ => return None,
    };
    Some(SiteRefActionTarget::ExactTile { tile })
}

#[cfg(test)]
mod tests {
    use super::{decode_planner_components, stable_action_id, visual_row_label};

    #[test]
    fn action_ids_are_bounded_and_distinguish_actions_and_long_subjects() {
        let long_a = format!("entity:{}", "a".repeat(128));
        let long_b = format!("entity:{}", "b".repeat(128));
        let move_up = stable_action_id("plan:move-up", &long_a, 42);
        let move_down = stable_action_id("plan:move-down", &long_a, 42);
        let other_subject = stable_action_id("plan:move-up", &long_b, 42);

        assert!(move_up.len() <= 128);
        assert_ne!(move_up, move_down);
        assert_ne!(move_up, other_subject);
        assert_eq!(stable_action_id("plan:move-up", &long_a, 42), move_up);

        let planner_subject = "planner:v1|5:study|12:research_hut";
        let planner_action = stable_action_id("progression:purchase-research", planner_subject, 7);
        assert!(!planner_action.contains('|'));
        assert!(cat_protocol::ActionIdempotencyId::new(planner_action).is_ok());
    }

    #[test]
    fn planner_ids_are_readable_without_changing_the_opaque_action_target() {
        let encoded = "planner:v1|5:study|12:research_hut";
        assert_eq!(
            decode_planner_components(encoded),
            Some(vec!["study", "research_hut"])
        );
        assert_eq!(
            visual_row_label("research", encoded),
            "Research frontier · research hut"
        );
        assert_eq!(decode_planner_components("planner:v1|9:short"), None);
    }
}
