use bevy::prelude::*;
use cat_client::leader_ai_ui::lai54::{
    bevy_shell::{
        FORBIDDEN_ROOT_OPENERS, Lai54LiveShell, Lai54PrimarySurfaceRoot, Lai54ShellControl,
        Lai54ShowcaseLot, Lai54ShowcaseRoot, Lai54StartCharterPanel, ShowcasePresentationAudit,
        spawn_live_shell, ui_scale_for_window_scale,
    },
    layout::{ClientPlatform, SUPPORTED_VIEWPORTS, UiScale, shell_layout},
    shell::{CouncilTab, PrimaryScreen},
    start_showcase::MATURE_SHOWCASE,
};

#[test]
fn live_shell_ecs_has_only_the_authoritative_navigation_and_static_showcase() {
    let mut app = App::new();
    app.add_systems(Startup, spawn_live_shell);
    app.update();

    let world = app.world_mut();
    let mut primary = 0;
    let mut council = 0;
    let mut controls = world.query::<&Lai54ShellControl>();
    for control in controls.iter(world) {
        match control {
            Lai54ShellControl::Primary(_) => primary += 1,
            Lai54ShellControl::Council(_) => council += 1,
            _ => {}
        }
    }
    assert_eq!(primary, PrimaryScreen::ALL.len());
    assert_eq!(council, CouncilTab::ALL.len());
    let mut lots = world.query::<&Lai54ShowcaseLot>();
    assert_eq!(lots.iter(world).count(), MATURE_SHOWCASE.lots.len());
    let mut showcase_roots = world.query::<&Lai54ShowcaseRoot>();
    assert_eq!(showcase_roots.iter(world).count(), 1);
    let mut scrollable_primary =
        world.query_filtered::<Entity, (With<Lai54PrimarySurfaceRoot>, With<ScrollPosition>)>();
    assert_eq!(scrollable_primary.iter(world).count(), 1);
    let mut scrollable_charter =
        world.query_filtered::<Entity, (With<Lai54StartCharterPanel>, With<ScrollPosition>)>();
    assert_eq!(scrollable_charter.iter(world).count(), 1);
    let live = world.resource::<Lai54LiveShell>();
    assert!(live.showcase.stays_off_map());
    assert_eq!(
        live.showcase.rendered_lots,
        MATURE_SHOWCASE.lots.len() as u16
    );
    assert_eq!(
        live.showcase.rendered_cats,
        MATURE_SHOWCASE.cats.len() as u8
    );
    assert_eq!(
        FORBIDDEN_ROOT_OPENERS,
        ["map", "help", "dispatches", "ticker", "letter"]
    );
}

#[test]
fn shell_contract_covers_all_desktop_scale_checkpoints_without_phone_fallback() {
    assert_eq!(ui_scale_for_window_scale(1.0), UiScale::Percent100);
    assert_eq!(ui_scale_for_window_scale(1.15), UiScale::Percent115);
    assert_eq!(ui_scale_for_window_scale(1.3), UiScale::Percent130);
    for platform in [ClientPlatform::Native, ClientPlatform::Wasm] {
        for viewport in SUPPORTED_VIEWPORTS {
            for scale in UiScale::ALL {
                let layout = shell_layout(platform, viewport, scale).expect("desktop contract");
                assert!(layout.keeps_top_bar_visible());
                assert!(layout.preserves_vertical_scroll());
            }
        }
    }
}

#[test]
fn showcase_render_audit_has_no_session_or_sim_mutation_path() {
    let audit = ShowcasePresentationAudit {
        rendered_lots: MATURE_SHOWCASE.lots.len() as u16,
        rendered_cats: MATURE_SHOWCASE.cats.len() as u8,
        ..default()
    };
    assert!(audit.stays_off_map());

    let shell = Lai54LiveShell {
        showcase: audit,
        ..default()
    };
    assert!(shell.showcase.stays_off_map());
    assert_eq!(shell.router.visible_primary(), None);
}
