use bevy::prelude::*;
use cat_client::leader_ai_ui::{
    AccessibleLabel, ColorToken, ControlKind, EntityKind, FeedbackState, FocusKey, FocusMemory,
    FocusRetention, ForbiddenPattern, GeometryScale, InputBlockerState, LeaderAiUiFoundation,
    LeaderAiUiFoundationPlugin, LeaderAiUiTheme, OverlayBand, OverlayLayer, ResponsiveClass,
    ResponsivePolicy, RoleColor, StateStyle, TestIdBuilder, UiSection, ViewportSize,
    validate_product_normal_tokens,
};

#[test]
fn default_theme_uses_grounded_tokens_and_lai_scales() {
    let theme = LeaderAiUiTheme::default();

    assert!(validate_product_normal_tokens(&theme).is_ok());
    assert_eq!(theme.spacing.steps(), [4, 8, 12, 16, 24, 32]);
    assert!(theme.geometry.uses_restrained_geometry());
    assert!(theme.motion.all_within_lai_range());
    assert!(theme.color_for(RoleColor::Paper).is_some());
    assert!(theme.color_for(RoleColor::Wood).is_some());
    assert!(theme.color_for(RoleColor::Stone).is_some());
    assert!(theme.color_for(RoleColor::Olive).is_some());
    assert!(theme.color_for(RoleColor::Rust).is_some());
}

#[test]
fn forbidden_product_patterns_are_rejected() {
    let mut theme = LeaderAiUiTheme::default();
    theme.colors.push(ColorToken::new(
        "glass-glow-hero-kpi-chart-token",
        RoleColor::Paper,
        Color::WHITE,
    ));

    let err = validate_product_normal_tokens(&theme).unwrap_err();
    assert!(matches!(
        err,
        cat_client::leader_ai_ui::StyleValidationError::ForbiddenToken {
            pattern: ForbiddenPattern::Glass,
            ..
        }
    ));
}

#[test]
fn geometry_motion_and_state_roles_are_bounded() {
    let theme = LeaderAiUiTheme::default();

    assert_eq!(theme.geometry.panel_radius_px, 10);
    assert_eq!(theme.geometry.button_radius_px, 8);
    assert_eq!(theme.motion.fast_ms, 100);
    assert_eq!(theme.motion.standard_ms, 150);
    assert_eq!(theme.motion.slow_ms, 200);

    for state in [
        FeedbackState::Loading,
        FeedbackState::Empty,
        FeedbackState::Stale,
        FeedbackState::UpdateRequired,
        FeedbackState::Error,
    ] {
        assert!(theme.state_style(state).is_some(), "{state:?} missing");
    }
    assert!(
        theme
            .state_style(FeedbackState::UpdateRequired)
            .unwrap()
            .blocks_mutation
    );

    let mut invalid = theme.clone();
    invalid.geometry = GeometryScale {
        panel_radius_px: 18,
        ..invalid.geometry
    };
    assert!(validate_product_normal_tokens(&invalid).is_err());

    let mut missing = theme.clone();
    missing
        .states
        .retain(|style| style.state != FeedbackState::Stale);
    missing.states.push(StateStyle {
        state: FeedbackState::Empty,
        ..theme.state_style(FeedbackState::Empty).unwrap()
    });
    assert!(validate_product_normal_tokens(&missing).is_err());
}

#[test]
fn responsive_policy_keeps_world_primary_across_native_and_wasm() {
    let policy = ResponsivePolicy::default();

    let wide = policy.decide(ViewportSize {
        width_px: 1440,
        height_px: 900,
        is_wasm: false,
    });
    assert_eq!(wide.class, ResponsiveClass::Wide);
    assert_eq!(wide.council_width_px, 820);
    assert_eq!(wide.right_inspector_width_px, 320);
    assert!(wide.keeps_world_primary);

    let compact = policy.decide(ViewportSize {
        width_px: 960,
        height_px: 720,
        is_wasm: true,
    });
    assert_eq!(compact.class, ResponsiveClass::Compact);
    assert_eq!(compact.council_width_px, 928);
    assert_eq!(compact.right_inspector_width_px, 0);
    assert!(compact.keeps_world_primary);
}

#[test]
fn overlay_layers_have_world_first_ordering() {
    assert_eq!(OverlayLayer::WorldMarkers.band(), OverlayBand::World);
    assert_eq!(OverlayLayer::Council.band(), OverlayBand::Interface);
    assert_eq!(OverlayLayer::Modal.band(), OverlayBand::Blocking);
    assert!(OverlayLayer::WorldMarkers.z_index() < OverlayLayer::Council.z_index());
    assert!(OverlayLayer::Council.z_index() < OverlayLayer::Modal.z_index());
    assert!(OverlayLayer::Modal.z_index() < OverlayLayer::Toast.z_index());
}

#[test]
fn stable_ids_and_accessible_labels_are_report_safe() {
    let panel_id = TestIdBuilder::panel(UiSection::Plans);
    assert_eq!(panel_id.as_str(), "lai-ui:plans:panel");

    let row_id = TestIdBuilder::row(UiSection::Cats, EntityKind::Cat, "Cat 42 / Left Paw");
    assert_eq!(row_id.as_str(), "lai-ui:cats:cat:cat-42-left-paw");

    let control_id =
        TestIdBuilder::control(UiSection::Progression, ControlKind::Activate, "Boost A");
    assert_eq!(
        control_id.as_str(),
        "lai-ui:progression:control:activate:boost-a"
    );

    let marker_id = TestIdBuilder::task_marker(
        "task.water.7",
        "source:river bank",
        cat_client::leader_ai_ui::TaskMarkerRole::Endpoint,
    );
    assert_eq!(
        marker_id.as_str(),
        "lai-ui:tasks:task:task-water-7:site:source:river-bank:endpoint"
    );

    assert_eq!(
        AccessibleLabel::panel(UiSection::Cats).as_str(),
        "Cat care panel"
    );
    assert_eq!(
        AccessibleLabel::control(ControlKind::Treat, "Mallow front left paw").as_str(),
        "Treat Mallow front left paw"
    );
    assert_eq!(
        AccessibleLabel::task_marker(
            "Fetch Water",
            cat_client::leader_ai_ui::TaskMarkerRole::Objective,
            "river source"
        )
        .as_str(),
        "Fetch Water objective, river source"
    );
}

#[test]
fn focus_memory_preserves_visible_targets_and_blocks_world_input_only_when_needed() {
    let mut focus = FocusMemory::default();
    assert_eq!(
        focus.preserve_after_refresh(["a"]),
        FocusRetention::UnchangedEmpty
    );

    focus.remember(FocusKey::new("lai-ui:plans:plan:plan-1"));
    assert_eq!(
        focus.preserve_after_refresh(["lai-ui:plans:plan:plan-1", "other"]),
        FocusRetention::Preserved
    );
    assert_eq!(
        focus.preserve_after_refresh(["other"]),
        FocusRetention::Cleared
    );
    assert!(focus.active.is_none());

    let policy = cat_client::leader_ai_ui::WorldInputPolicy::default();
    assert!(policy.allows_world_input(InputBlockerState::default()));
    assert!(!policy.allows_world_input(InputBlockerState {
        text_input_active: true,
        ..InputBlockerState::default()
    }));
    assert!(!policy.allows_world_input(InputBlockerState {
        modal_open: true,
        ..InputBlockerState::default()
    }));
}

#[test]
fn foundation_plugin_registers_resource_without_runtime_dto_shims() {
    let mut app = App::new();
    app.add_plugins(LeaderAiUiFoundationPlugin);

    let foundation = app.world().resource::<LeaderAiUiFoundation>();
    assert!(validate_product_normal_tokens(&foundation.theme).is_ok());
    assert_eq!(foundation.responsive, ResponsivePolicy::default());
}
