#[path = "../src/leader_ai_ui/lai54/mod.rs"]
mod lai54;

use lai54::{
    layout::{
        CharterPlacement, ClientPlatform, FocusRetention, FocusState, LayoutError,
        SUPPORTED_VIEWPORTS, ScrollState, UiScale, Viewport, shell_layout,
    },
    shell::{
        ConnectionState, CouncilTab, EscapeOutcome, IconTreatment, NavigationActivation,
        PRIMARY_NAVIGATION, PrimaryScreen, ShellRouter, SurfaceMaterial, SurfaceStack,
        TabTreatment, TopBarModel, VisualLanguage, handle_escape,
    },
    start_showcase::{
        DESTINATION_CARDS, DestinationKind, EntryControlState, EntryDisabledReason,
        MATURE_SHOWCASE, ShowcaseBinding, ShowcaseBuildingKind, StartCharterState, start_layout,
    },
};

#[test]
fn shell_has_exactly_five_routes_and_council_has_exactly_six_tabs() {
    assert_eq!(PrimaryScreen::ALL.len(), 5);
    assert_eq!(CouncilTab::ALL.len(), 6);

    let mut router = ShellRouter::default();
    for screen in PrimaryScreen::ALL {
        router.open(screen);
        assert_eq!(router.visible_primary(), Some(screen));
        assert_eq!(router.visible_primary_count(), 1);
    }
    for tab in CouncilTab::ALL {
        router.open_council(tab);
        assert_eq!(router.visible_primary(), Some(PrimaryScreen::Council));
        assert_eq!(router.council_tab(), tab);
    }

    assert_eq!(PRIMARY_NAVIGATION.len(), 5);
    for (index, item) in PRIMARY_NAVIGATION.into_iter().enumerate() {
        assert_eq!(item.screen, PrimaryScreen::ALL[index]);
        assert_eq!(item.activation, NavigationActivation::ExplicitControl);
        assert!(item.label_key.is_english_key());
    }
}

#[test]
fn top_bar_centers_village_and_exposes_every_connection_session_state() {
    for connection in ConnectionState::ALL {
        let top_bar = TopBarModel::new(connection);
        assert_eq!(top_bar.session.connection, connection);
        assert!(top_bar.center_village_label_key.is_english_key());
        assert!(connection.label_key().is_english_key());
        let _semantic_icon = connection.icon();
    }
}

#[test]
fn centralized_escape_unwinds_nested_surfaces_then_returns_to_world() {
    let mut router = ShellRouter::default();
    router.open_council(CouncilTab::Cats);
    let mut stack = SurfaceStack {
        transient_menu_open: true,
        modal_open: true,
        nested_detail_open: true,
    };

    assert_eq!(
        handle_escape(&mut stack, &mut router),
        EscapeOutcome::ClosedTransientMenu
    );
    assert_eq!(
        handle_escape(&mut stack, &mut router),
        EscapeOutcome::ClosedModal
    );
    assert_eq!(
        handle_escape(&mut stack, &mut router),
        EscapeOutcome::ClosedNestedDetail
    );
    assert_eq!(
        handle_escape(&mut stack, &mut router),
        EscapeOutcome::ClosedPrimaryToWorld
    );
    assert_eq!(
        handle_escape(&mut stack, &mut router),
        EscapeOutcome::AlreadyAtWorld
    );
    assert_eq!(router.visible_primary_count(), 0);
}

#[test]
fn native_and_wasm_share_all_viewport_and_scale_checkpoints() {
    for platform in [ClientPlatform::Native, ClientPlatform::Wasm] {
        for viewport in SUPPORTED_VIEWPORTS {
            for scale in UiScale::ALL {
                let layout = shell_layout(platform, viewport, scale).expect("supported checkpoint");
                assert!(layout.keeps_top_bar_visible());
                assert!(layout.preserves_vertical_scroll());
                assert!(layout.logical_width_px > 0.0);
                assert!(layout.logical_height_px > 0.0);

                let charter = start_layout(layout);
                if charter.placement == CharterPlacement::BesideShowcase {
                    assert_eq!(charter.charter_width_px, 560.0);
                } else {
                    assert!(charter.charter_width_px <= layout.logical_width_px * 0.92);
                }
                assert!(charter.charter_max_height_px <= layout.logical_height_px);
            }
        }
    }

    assert_eq!(
        shell_layout(
            ClientPlatform::Wasm,
            Viewport::new(390, 844),
            UiScale::Percent100
        ),
        Err(LayoutError::PhoneViewportOutOfScope)
    );
}

#[test]
fn showcase_is_mature_off_map_and_meets_the_full_catalog_contract() {
    MATURE_SHOWCASE.validate().expect("valid static showcase");
    assert_eq!(
        MATURE_SHOWCASE.binding,
        ShowcaseBinding::OffMapStaticPresentation
    );
    assert_eq!(MATURE_SHOWCASE.maturity_days, 730);
    assert!(MATURE_SHOWCASE.lots.len() >= 42);
    assert_eq!(MATURE_SHOWCASE.cats.len(), 60);
    assert!(MATURE_SHOWCASE.infrastructure.road_segments > 0);
    assert!(MATURE_SHOWCASE.infrastructure.wall_segments > 0);

    let count = |kind| {
        MATURE_SHOWCASE
            .lots
            .iter()
            .filter(|lot| lot.kind == kind)
            .count()
    };
    assert_eq!(count(ShowcaseBuildingKind::Hole), 1);
    assert_eq!(count(ShowcaseBuildingKind::Workshop), 1);
    assert!(count(ShowcaseBuildingKind::FamilyHome) > 0);
    assert!(count(ShowcaseBuildingKind::ElderLodge) > 0);
    assert!(count(ShowcaseBuildingKind::Cookhouse) > 0);
    assert!(count(ShowcaseBuildingKind::FishingHut) > 0);
    assert!(count(ShowcaseBuildingKind::Farm) > 0);
    assert!(count(ShowcaseBuildingKind::StorageYard) > 0);
    assert!(count(ShowcaseBuildingKind::Carpentry) > 0);
    assert!(count(ShowcaseBuildingKind::Watchtower) > 0);

    let hole = MATURE_SHOWCASE
        .lots
        .iter()
        .find(|lot| lot.kind == ShowcaseBuildingKind::Hole)
        .expect("one hole");
    assert!(hole.footprint.is_centered_five_by_five());
}

#[test]
fn entry_cards_are_distinct_localized_and_require_explicit_activation() {
    assert_eq!(DESTINATION_CARDS.len(), 2);
    assert_ne!(DESTINATION_CARDS[0].kind, DESTINATION_CARDS[1].kind);
    assert_ne!(
        DESTINATION_CARDS[0].title_key,
        DESTINATION_CARDS[1].title_key
    );
    for card in DESTINATION_CARDS {
        assert!(card.title_key.is_english_key());
        assert!(card.detail_key.is_english_key());
        assert!(card.action_key.is_english_key());
    }

    let mut charter = StartCharterState::default();
    assert_eq!(charter.selected_destination, None);
    assert_eq!(
        charter.explicit_entry_intent(),
        Err(EntryDisabledReason::Disconnected)
    );

    charter.connection = ConnectionState::Connected;
    charter.snapshot_loaded = true;
    charter.player_name = "Mara".to_owned();
    assert_eq!(
        charter.entry_control_state(),
        EntryControlState::Disabled(EntryDisabledReason::Destination)
    );
    charter.select_destination(DestinationKind::Global);
    assert_eq!(charter.entry_control_state(), EntryControlState::Enabled);
    assert_eq!(
        charter
            .explicit_entry_intent()
            .expect("explicit global entry")
            .destination,
        DestinationKind::Global
    );

    charter.select_destination(DestinationKind::Personal);
    assert_eq!(
        charter.entry_control_state(),
        EntryControlState::Disabled(EntryDisabledReason::VillageName)
    );
    charter.village_name = "Branchwood".to_owned();
    assert_eq!(charter.entry_control_state(), EntryControlState::Enabled);
}

#[test]
fn focus_scroll_disabled_loading_connection_and_error_states_are_explicit() {
    let mut focus = FocusState::default();
    focus.focus("council.cat.42");
    assert_eq!(
        focus.retain_or_parent(["council.cat.42"], "council.tabs"),
        FocusRetention::Preserved
    );
    assert_eq!(
        focus.retain_or_parent(["council.cat.7"], "council.tabs"),
        FocusRetention::MovedToParent
    );

    let mut scroll = ScrollState::default();
    scroll.set_extent(1_200.0, 400.0);
    scroll.scroll_by(1_000.0);
    assert_eq!(scroll.offset_px, 800.0);
    scroll.page_by(-1, 400.0);
    assert_eq!(scroll.offset_px, 440.0);

    let mut charter = StartCharterState {
        connection: ConnectionState::LoadingSnapshot,
        ..Default::default()
    };
    assert_eq!(
        charter.entry_control_state(),
        EntryControlState::Disabled(EntryDisabledReason::Loading)
    );
    charter.connection = ConnectionState::Error;
    charter.error_key = Some(lai54::shell::LocalizationKey("start.error.connection"));
    assert_eq!(
        charter.entry_control_state(),
        EntryControlState::Disabled(EntryDisabledReason::Error)
    );
    assert!(EntryDisabledReason::Error.label_key().is_english_key());

    let cases = [
        (
            StartCharterState {
                pending: true,
                ..Default::default()
            },
            EntryDisabledReason::Pending,
        ),
        (
            StartCharterState {
                connection: ConnectionState::Connected,
                snapshot_loaded: true,
                ..Default::default()
            },
            EntryDisabledReason::PlayerName,
        ),
        (
            StartCharterState {
                connection: ConnectionState::Connected,
                snapshot_loaded: true,
                player_name: "Mara".to_owned(),
                ..Default::default()
            },
            EntryDisabledReason::Destination,
        ),
        (
            StartCharterState {
                connection: ConnectionState::Connected,
                snapshot_loaded: true,
                player_name: "Mara".to_owned(),
                selected_destination: Some(DestinationKind::Personal),
                ..Default::default()
            },
            EntryDisabledReason::VillageName,
        ),
    ];
    for (state, expected) in cases {
        assert_eq!(
            state.entry_control_state(),
            EntryControlState::Disabled(expected)
        );
        assert!(expected.label_key().is_english_key());
    }
}

#[test]
fn visual_language_is_authored_solid_game_ui() {
    let visual = VisualLanguage::AUTHORED;
    assert_eq!(visual.worktable, SurfaceMaterial::DarkForestWorktable);
    assert_eq!(visual.content, SurfaceMaterial::Parchment);
    assert_eq!(visual.framing, SurfaceMaterial::Wood);
    assert_eq!(visual.feedback, SurfaceMaterial::SolidPanel);
    assert_eq!(visual.tabs, TabTreatment::Underline);
    assert_eq!(visual.icons, IconTreatment::SemanticPixel);

    for screen in PrimaryScreen::ALL {
        assert!(screen.label_key().is_english_key());
        let _semantic_icon = screen.icon();
    }
    for tab in CouncilTab::ALL {
        assert!(tab.label_key().is_english_key());
        let _semantic_icon = tab.icon();
    }
}
