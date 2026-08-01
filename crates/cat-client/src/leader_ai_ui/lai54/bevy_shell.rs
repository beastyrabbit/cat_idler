//! Live Bevy bridge for the LAI.54 shell and start-charter contracts.
//!
//! This is deliberately presentation-only. It reads the existing session and
//! start-flow state, renders the immutable mature showcase, and routes explicit
//! UI intent back through the established start-screen submit path. It never
//! constructs a colony, snapshot, selection, protocol action, save, or sim
//! entity for the showcase.

use accesskit::Role;
use bevy::a11y::AccessibilityNode;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::{
    layout::{CharterPlacement, ClientPlatform, UiScale, Viewport, shell_layout},
    shell::{
        ConnectionState, CouncilTab, EscapeOutcome, PrimaryScreen, ShellRouter, SurfaceStack,
        handle_escape,
    },
    start_showcase::{
        DESTINATION_CARDS, DestinationKind, EntryControlState, MATURE_SHOWCASE,
        ShowcaseBuildingKind, StartCharterState,
    },
};
use crate::leader_ai_ui::{semantic_node, semantic_status_node};

const INK: Color = Color::srgb(0.153, 0.106, 0.086);
const PARCHMENT: Color = Color::srgb(0.937, 0.886, 0.741);
const DARK_FOREST: Color = Color::srgb(0.090, 0.235, 0.180);
const WOOD: Color = Color::srgb(0.427, 0.282, 0.169);
const MOSS: Color = Color::srgb(0.310, 0.439, 0.251);
const RUST: Color = Color::srgb(0.643, 0.286, 0.176);

/// Runtime state owns only presentation navigation and charter projection.
/// The showcase audit makes its non-authoritative boundary inspectable.
#[derive(Resource, Debug, PartialEq)]
pub struct Lai54LiveShell {
    pub router: ShellRouter,
    pub surfaces: SurfaceStack,
    pub charter: StartCharterState,
    pub showcase: ShowcasePresentationAudit,
    pub last_escape: EscapeOutcome,
}

impl Default for Lai54LiveShell {
    fn default() -> Self {
        Self {
            router: ShellRouter::default(),
            surfaces: SurfaceStack::default(),
            charter: StartCharterState::default(),
            showcase: ShowcasePresentationAudit::default(),
            last_escape: EscapeOutcome::AlreadyAtWorld,
        }
    }
}

/// Explicit proof that the showcase is static presentation rather than a
/// simulated colony. The only counter is incremented while spawning fixed UI
/// lots; all authoritative mutation counters remain zero by construction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShowcasePresentationAudit {
    pub rendered_lots: u16,
    pub rendered_cats: u8,
    pub authoritative_snapshot_reads: u32,
    pub authoritative_mutations: u32,
    pub auto_entry_requests: u32,
}

impl ShowcasePresentationAudit {
    pub const fn stays_off_map(self) -> bool {
        self.authoritative_snapshot_reads == 0
            && self.authoritative_mutations == 0
            && self.auto_entry_requests == 0
    }
}

#[derive(Component)]
pub struct Lai54ShellRoot;
#[derive(Component)]
pub struct Lai54PrimaryNavRoot;
#[derive(Component)]
pub struct Lai54StartCharterRoot;
#[derive(Component)]
pub struct Lai54StartCharterPanel;
#[derive(Component)]
pub struct Lai54CharterControls;
#[derive(Component)]
pub struct Lai54PrimarySurfaceRoot;
#[derive(Component)]
pub struct Lai54CouncilTabsRoot;
#[derive(Component)]
pub struct Lai54ShowcaseRoot;
#[derive(Component)]
pub struct Lai54ShowcaseLot;
#[derive(Component)]
pub struct Lai54PrimaryLabel;
#[derive(Component)]
pub struct Lai54CouncilLabel;
#[derive(Component)]
pub struct Lai54SessionLabel;
#[derive(Component)]
pub struct Lai54CharterSummary;
#[derive(Component)]
pub struct Lai54CharterFeedback;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lai54ShellControl {
    Primary(PrimaryScreen),
    Council(CouncilTab),
    CenterVillage,
    StartDestination(DestinationKind),
    StartPlayerName,
    StartVillageName,
    ExplicitEntry,
}

/// The new root contains no legacy Map, Help, Dispatches, ticker, or
/// letter-open controls. Those surfaces are intentionally deferred to the
/// dedicated replacement/delete cards rather than given parallel openers.
pub const FORBIDDEN_ROOT_OPENERS: [&str; 5] = ["map", "help", "dispatches", "ticker", "letter"];

#[must_use]
pub fn ui_scale_for_window_scale(scale_factor: f32) -> UiScale {
    match (scale_factor * 100.0).round() as u16 {
        115 => UiScale::Percent115,
        130 => UiScale::Percent130,
        _ => UiScale::Percent100,
    }
}

#[derive(Default)]
pub struct Lai54LiveShellPlugin;

impl Plugin for Lai54LiveShellPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Lai54LiveShell>()
            .add_systems(Startup, spawn_live_shell)
            .add_systems(
                Update,
                (
                    sync_charter_projection,
                    handle_live_shell_controls,
                    handle_live_shell_escape,
                    sync_live_shell_layout,
                    suppress_superseded_live_workspace
                        .after(crate::update_bottom_overlays)
                        .after(crate::update_officers_panel)
                        .after(crate::update_orders_panel)
                        .after(crate::update_goods)
                        .after(crate::update_trade_menu)
                        .after(crate::update_census)
                        .after(crate::update_minimap),
                    sync_live_shell_visibility_and_copy,
                    handle_live_shell_scroll,
                )
                    .chain(),
            );
    }
}

fn button_style() -> (Button, Node, BackgroundColor, BorderColor) {
    (
        Button,
        Node {
            height: Val::Px(34.0),
            padding: UiRect::axes(Val::Px(10.0), Val::Px(0.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(WOOD),
        BorderColor::all(Color::srgb(0.20, 0.12, 0.07)),
    )
}

fn text_bundle(value: impl Into<String>, size: f32, color: Color) -> (Text, TextFont, TextColor) {
    (
        Text::new(value),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

fn spawn_control(
    commands: &mut Commands<'_, '_>,
    parent: Entity,
    control: Lai54ShellControl,
    label: &str,
    semantic_id: &str,
) {
    let entity = commands
        .spawn((
            button_style(),
            control,
            Name::new(format!("LAI.54 {semantic_id}")),
            semantic_node(Role::Button, semantic_id, label, true),
        ))
        .id();
    commands.entity(entity).with_children(|button| {
        button.spawn(text_bundle(label, 13.0, Color::srgb(0.98, 0.91, 0.74)));
    });
    commands.entity(parent).add_child(entity);
}

pub fn spawn_live_shell(mut commands: Commands<'_, '_>) {
    commands.insert_resource(Lai54LiveShell {
        showcase: ShowcasePresentationAudit {
            rendered_lots: MATURE_SHOWCASE.lots.len() as u16,
            rendered_cats: MATURE_SHOWCASE.cats.len() as u8,
            ..default()
        },
        ..default()
    });
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                ..default()
            },
            GlobalZIndex(1_300),
            BackgroundColor(Color::NONE),
            Lai54ShellRoot,
            Name::new("LAI.54 routed strategy shell"),
        ))
        .id();

    let top_bar = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                right: Val::Px(16.0),
                top: Val::Px(12.0),
                min_height: Val::Px(52.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(DARK_FOREST),
            BorderColor::all(WOOD),
            Lai54PrimaryNavRoot,
            Name::new("LAI.54 primary navigation"),
        ))
        .id();
    commands.entity(root).add_child(top_bar);
    spawn_control(
        &mut commands,
        top_bar,
        Lai54ShellControl::CenterVillage,
        "Center Village",
        "lai54.shell.center-village",
    );
    for screen in PrimaryScreen::ALL {
        let label = match screen {
            PrimaryScreen::Log => "Log",
            PrimaryScreen::Stores => "Stores",
            PrimaryScreen::Village => "Village",
            PrimaryScreen::Research => "Research",
            PrimaryScreen::Council => "Council",
        };
        spawn_control(
            &mut commands,
            top_bar,
            Lai54ShellControl::Primary(screen),
            label,
            &format!("lai54.shell.primary.{label}"),
        );
    }
    let session = commands
        .spawn((
            Node {
                margin: UiRect::left(Val::Auto),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.13, 0.18, 0.12)),
            BorderColor::all(MOSS),
            Lai54SessionLabel,
            semantic_status_node("lai54.shell.session", "Session loading", false),
        ))
        .id();
    commands.entity(session).with_children(|node| {
        node.spawn(text_bundle("Loading report-safe session", 12.0, PARCHMENT));
    });
    commands.entity(top_bar).add_child(session);

    let primary = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                right: Val::Px(24.0),
                top: Val::Px(82.0),
                bottom: Val::Px(24.0),
                max_width: Val::Px(1_920.0),
                padding: UiRect::all(Val::Px(20.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                border: UiRect::all(Val::Px(2.0)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
            BackgroundColor(PARCHMENT),
            BorderColor::all(WOOD),
            Lai54PrimarySurfaceRoot,
            crate::WorldInputBlocker,
            Name::new("LAI.54 primary report-safe surface"),
        ))
        .id();
    commands.entity(root).add_child(primary);
    commands.entity(primary).with_children(|panel| {
        panel.spawn((text_bundle("World", 22.0, INK), Lai54PrimaryLabel));
        panel.spawn(text_bundle(
            "Loading report-safe surface. Detailed Log, Stores, Village, Research, and Council content is routed here by LAI.66 and LAI.67.",
            14.0,
            Color::srgb(0.26, 0.21, 0.17),
        ));
        panel.spawn(text_bundle(
            "Empty, stale, loading, and error states remain explicit; no hidden world truth is synthesized by this shell.",
            12.0,
            RUST,
        ));
    });

    let council_tabs = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(6.0),
                row_gap: Val::Px(6.0),
                display: Display::None,
                ..default()
            },
            Lai54CouncilTabsRoot,
            Name::new("LAI.54 Council tabs"),
        ))
        .id();
    commands.entity(primary).add_child(council_tabs);
    for tab in CouncilTab::ALL {
        let label = match tab {
            CouncilTab::Plans => "Plans",
            CouncilTab::Tasks => "Tasks",
            CouncilTab::Cats => "Cats",
            CouncilTab::Hole => "Hole",
            CouncilTab::Diplomacy => "Diplomacy",
            CouncilTab::Trade => "Trade",
        };
        spawn_control(
            &mut commands,
            council_tabs,
            Lai54ShellControl::Council(tab),
            label,
            &format!("lai54.shell.council.{label}"),
        );
    }
    commands.entity(primary).with_children(|panel| {
        panel.spawn((text_bundle("Council / Plans", 16.0, INK), Lai54CouncilLabel));
    });

    let charter = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                padding: UiRect::all(Val::Px(24.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(DARK_FOREST),
            GlobalZIndex(1_310),
            Lai54StartCharterRoot,
            crate::WorldInputBlocker,
            Name::new("LAI.54 off-map start charter"),
        ))
        .id();
    commands.entity(root).add_child(charter);
    let charter_panel = commands
        .spawn((
            Node {
                width: Val::Percent(94.0),
                max_width: Val::Px(1_640.0),
                height: Val::Percent(92.0),
                padding: UiRect::all(Val::Px(20.0)),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(18.0),
                border: UiRect::all(Val::Px(3.0)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
            BackgroundColor(PARCHMENT),
            BorderColor::all(WOOD),
            Lai54StartCharterPanel,
            Name::new("LAI.54 charter worktable"),
        ))
        .id();
    commands.entity(charter).add_child(charter_panel);
    let charter_copy = commands
        .spawn((
            Node {
                width: Val::Percent(38.0),
                min_width: Val::Px(310.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                overflow: Overflow::clip_y(),
                ..default()
            },
            Lai54CharterControls,
            Name::new("LAI.54 charter controls"),
        ))
        .id();
    commands.entity(charter_panel).add_child(charter_copy);
    commands.entity(charter_copy).with_children(|panel| {
        panel.spawn(text_bundle("Idle Cat Forest", 27.0, INK));
        panel.spawn(text_bundle(
            "A mature village, shown only as a charter illustration.",
            14.0,
            Color::srgb(0.25, 0.21, 0.17),
        ));
        panel.spawn((
            text_bundle("Preparing entry", 13.0, RUST),
            Lai54CharterSummary,
        ));
        panel.spawn((text_bundle("", 12.0, RUST), Lai54CharterFeedback));
    });
    spawn_control(
        &mut commands,
        charter_copy,
        Lai54ShellControl::StartPlayerName,
        "Player name",
        "lai54.start.player-name",
    );
    for card in DESTINATION_CARDS {
        let label = match card.kind {
            DestinationKind::Global => "Global village",
            DestinationKind::Personal => "Personal village",
        };
        spawn_control(
            &mut commands,
            charter_copy,
            Lai54ShellControl::StartDestination(card.kind),
            label,
            &format!("lai54.start.destination.{label}"),
        );
    }
    spawn_control(
        &mut commands,
        charter_copy,
        Lai54ShellControl::StartVillageName,
        "Village name",
        "lai54.start.village-name",
    );
    spawn_control(
        &mut commands,
        charter_copy,
        Lai54ShellControl::ExplicitEntry,
        "Continue / Create",
        "lai54.start.explicit-entry",
    );

    let showcase = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                min_width: Val::Px(430.0),
                position_type: PositionType::Relative,
                border: UiRect::all(Val::Px(2.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.11, 0.20, 0.13)),
            BorderColor::all(MOSS),
            Lai54ShowcaseRoot,
            Name::new("LAI.54 static two-year village showcase"),
        ))
        .id();
    commands.entity(charter_panel).add_child(showcase);
    commands.entity(showcase).with_children(|map| {
        map.spawn(text_bundle(
            "730-day off-map showcase · 60 cats · 48 lots",
            12.0,
            PARCHMENT,
        ));
        for lot in MATURE_SHOWCASE.lots {
            let x = (f32::from(lot.footprint.x) + 38.0) * 8.0;
            let y = (f32::from(lot.footprint.y) + 24.0) * 8.0 + 28.0;
            let fill = showcase_color(lot.kind);
            let label = if lot.kind == ShowcaseBuildingKind::Hole {
                "HOLE · 5×5"
            } else {
                ""
            };
            map.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(x),
                    top: Val::Px(y),
                    width: Val::Px(f32::from(lot.footprint.width) * 8.0),
                    height: Val::Px(f32::from(lot.footprint.height) * 8.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(fill),
                BorderColor::all(Color::srgb(0.08, 0.10, 0.07)),
                Lai54ShowcaseLot,
            ));
            if !label.is_empty() {
                map.spawn((
                    Text::new(label),
                    TextFont {
                        font_size: FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(PARCHMENT),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(x + 2.0),
                        top: Val::Px(y + 12.0),
                        ..default()
                    },
                ));
            }
        }
    });
}

fn showcase_color(kind: ShowcaseBuildingKind) -> Color {
    match kind {
        ShowcaseBuildingKind::Hole => RUST,
        ShowcaseBuildingKind::Farm
        | ShowcaseBuildingKind::Orchard
        | ShowcaseBuildingKind::Apiary => MOSS,
        ShowcaseBuildingKind::StorageYard | ShowcaseBuildingKind::Workshop => {
            Color::srgb(0.58, 0.43, 0.24)
        }
        ShowcaseBuildingKind::FishingHut | ShowcaseBuildingKind::Waterworks => {
            Color::srgb(0.25, 0.44, 0.48)
        }
        ShowcaseBuildingKind::Watchtower
        | ShowcaseBuildingKind::Guardpost
        | ShowcaseBuildingKind::Gatehouse => Color::srgb(0.38, 0.38, 0.34),
        _ => WOOD,
    }
}

fn shell_connection(
    session_ready: bool,
    has_snapshot: bool,
    state: &crate::ConnectionState,
) -> ConnectionState {
    match state.phase {
        crate::ConnectionPhase::Connected if session_ready && has_snapshot => {
            ConnectionState::Connected
        }
        crate::ConnectionPhase::Connected | crate::ConnectionPhase::Connecting if !has_snapshot => {
            ConnectionState::LoadingSnapshot
        }
        crate::ConnectionPhase::Connected => ConnectionState::LoadingSnapshot,
        crate::ConnectionPhase::Connecting => ConnectionState::Connecting,
        crate::ConnectionPhase::WaitingToRetry => ConnectionState::Reconnecting,
        crate::ConnectionPhase::Incompatible => ConnectionState::UpdateRequired,
        crate::ConnectionPhase::Disconnected => ConnectionState::Disconnected,
    }
}

fn sync_charter_projection(
    mut live: ResMut<'_, Lai54LiveShell>,
    start: Res<'_, crate::StartScreen>,
    session: Res<'_, crate::Session>,
    latest: Res<'_, crate::LatestSnapshot>,
    connection: Res<'_, crate::ConnectionState>,
) {
    let mut charter = StartCharterState::default();
    charter.connection = shell_connection(session.ready, latest.0.is_some(), &connection);
    charter.snapshot_loaded = latest.0.is_some();
    charter.player_name.clone_from(&start.player_name);
    charter.village_name.clone_from(&start.village_name);
    charter.selected_destination = match start.mode {
        Some(crate::StartMode::Global) => Some(DestinationKind::Global),
        Some(crate::StartMode::Personal) => Some(DestinationKind::Personal),
        None => None,
    };
    charter.pending = start.pending_foundation || start.pending_target_colony_id.is_some();
    charter.error_key = start
        .error
        .as_ref()
        .map(|_| super::shell::LocalizationKey("start.error.server"));
    charter.focus = match start.focused_input {
        crate::StartInput::PlayerName => super::start_showcase::StartFocus::PlayerName,
        crate::StartInput::VillageName => super::start_showcase::StartFocus::VillageName,
    };
    live.charter = charter;
}

fn handle_live_shell_controls(
    mut buttons: Query<'_, '_, (&Interaction, &Lai54ShellControl), Changed<Interaction>>,
    mut live: ResMut<'_, Lai54LiveShell>,
    mut start: ResMut<'_, crate::StartScreen>,
) {
    for (interaction, control) in &mut buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *control {
            Lai54ShellControl::Primary(screen) => live.router.open(screen),
            Lai54ShellControl::Council(tab) => live.router.open_council(tab),
            Lai54ShellControl::CenterVillage => live.router.return_to_world(),
            Lai54ShellControl::StartDestination(destination) => {
                start.mode = Some(match destination {
                    DestinationKind::Global => crate::StartMode::Global,
                    DestinationKind::Personal => crate::StartMode::Personal,
                });
                start.error = None;
            }
            Lai54ShellControl::StartPlayerName => {
                start.focused_input = crate::StartInput::PlayerName
            }
            Lai54ShellControl::StartVillageName => {
                start.focused_input = crate::StartInput::VillageName
            }
            Lai54ShellControl::ExplicitEntry => {
                if live.charter.explicit_entry_intent().is_ok() {
                    start.submit_requested = true;
                }
            }
        }
    }
}

fn handle_live_shell_escape(
    keys: Res<'_, ButtonInput<KeyCode>>,
    start: Res<'_, crate::StartScreen>,
    mut live: ResMut<'_, Lai54LiveShell>,
) {
    if !start.visible && keys.just_pressed(KeyCode::Escape) {
        let live = &mut *live;
        let outcome = handle_escape(&mut live.surfaces, &mut live.router);
        live.last_escape = outcome;
    }
}

fn sync_live_shell_layout(
    windows: Query<'_, '_, &Window, With<PrimaryWindow>>,
    mut charter: Query<'_, '_, &mut Node, With<Lai54StartCharterRoot>>,
    mut primary: Query<
        '_,
        '_,
        &mut Node,
        (
            With<Lai54PrimarySurfaceRoot>,
            Without<Lai54StartCharterRoot>,
            Without<Lai54StartCharterPanel>,
        ),
    >,
    mut charter_panel: Query<
        '_,
        '_,
        &mut Node,
        (
            With<Lai54StartCharterPanel>,
            Without<Lai54PrimarySurfaceRoot>,
            Without<Lai54StartCharterRoot>,
        ),
    >,
    mut controls: Query<
        '_,
        '_,
        &mut Node,
        (
            With<Lai54CharterControls>,
            Without<Lai54StartCharterPanel>,
            Without<Lai54ShowcaseRoot>,
        ),
    >,
    mut showcase: Query<
        '_,
        '_,
        &mut Node,
        (
            With<Lai54ShowcaseRoot>,
            Without<Lai54StartCharterPanel>,
            Without<Lai54CharterControls>,
        ),
    >,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let platform = if cfg!(target_arch = "wasm32") {
        ClientPlatform::Wasm
    } else {
        ClientPlatform::Native
    };
    let scale = ui_scale_for_window_scale(window.scale_factor());
    let Ok(layout) = shell_layout(
        platform,
        Viewport::new(
            window.width().round() as u16,
            window.height().round() as u16,
        ),
        scale,
    ) else {
        return;
    };
    if let Ok(mut node) = charter.single_mut() {
        node.padding = UiRect::all(Val::Px(f32::from(layout.content_gutter_px)));
    }
    if let Ok(mut node) = primary.single_mut() {
        node.left = Val::Px(f32::from(layout.content_gutter_px));
        node.right = Val::Px(f32::from(layout.content_gutter_px));
    }
    if let Ok(mut panel) = charter_panel.single_mut() {
        panel.flex_direction = match layout.charter_placement {
            CharterPlacement::BesideShowcase => FlexDirection::Row,
            CharterPlacement::CenteredOverShowcase => FlexDirection::Column,
        };
        panel.column_gap = match layout.charter_placement {
            CharterPlacement::BesideShowcase => Val::Px(18.0),
            CharterPlacement::CenteredOverShowcase => Val::Px(0.0),
        };
        panel.row_gap = match layout.charter_placement {
            CharterPlacement::BesideShowcase => Val::Px(0.0),
            CharterPlacement::CenteredOverShowcase => Val::Px(16.0),
        };
    }
    if let Ok(mut controls) = controls.single_mut() {
        controls.width = match layout.charter_placement {
            CharterPlacement::BesideShowcase => Val::Percent(38.0),
            CharterPlacement::CenteredOverShowcase => Val::Percent(100.0),
        };
        controls.min_width = match layout.charter_placement {
            CharterPlacement::BesideShowcase => Val::Px(310.0),
            CharterPlacement::CenteredOverShowcase => Val::Auto,
        };
    }
    if let Ok(mut showcase) = showcase.single_mut() {
        showcase.width = match layout.charter_placement {
            CharterPlacement::BesideShowcase => Val::Auto,
            CharterPlacement::CenteredOverShowcase => Val::Percent(100.0),
        };
        showcase.min_width = match layout.charter_placement {
            CharterPlacement::BesideShowcase => Val::Px(430.0),
            CharterPlacement::CenteredOverShowcase => Val::Auto,
        };
        showcase.min_height = match layout.charter_placement {
            CharterPlacement::BesideShowcase => Val::Auto,
            CharterPlacement::CenteredOverShowcase => Val::Px(360.0),
        };
    }
}

fn handle_live_shell_scroll(
    live: Res<'_, Lai54LiveShell>,
    mut wheel: MessageReader<'_, '_, MouseWheel>,
    mut surfaces: Query<
        '_,
        '_,
        (&Node, &ComputedNode, &mut ScrollPosition),
        Or<(With<Lai54PrimarySurfaceRoot>, With<Lai54StartCharterPanel>)>,
    >,
) {
    if live.router.visible_primary().is_some() {
        return;
    }
    let delta = wheel.read().fold(0.0, |total, event| {
        total
            - event.y
                * match event.unit {
                    MouseScrollUnit::Line => 21.0,
                    MouseScrollUnit::Pixel => 1.0,
                }
    });
    if delta == 0.0 {
        return;
    }
    for (node, computed, mut position) in &mut surfaces {
        if node.display == Display::None {
            continue;
        }
        let maximum = ((computed.content_size().y - computed.size().y)
            * computed.inverse_scale_factor())
        .max(0.0);
        position.y = (position.y + delta).clamp(0.0, maximum);
    }
}

/// LAI.54 owns the outer shell. The older report workspace is retained only as
/// an implementation dependency until LAI.66/67/70 replace and delete its
/// individual panels; its root is suppressed so it cannot become a second
/// primary screen or revive Shrine/Favor routes behind this shell.
fn suppress_superseded_live_workspace(
    mut legacy_roots: Query<'_, '_, (&Name, &mut Node), Without<Lai54ShellRoot>>,
    mut superseded_surfaces: Query<
        '_,
        '_,
        &mut Node,
        Or<(
            With<crate::LegacyColonyHudPanel>,
            With<crate::LegacyBottomCommandBar>,
            With<crate::DispatchesPanel>,
            With<crate::MinimapPanel>,
            With<crate::HelpPanel>,
            With<crate::AnnouncementsPanel>,
            With<crate::GoodsPanel>,
            With<crate::CensusPanel>,
            With<crate::TradeMenuPanel>,
            With<crate::OfficersPanel>,
            With<crate::OrdersPanel>,
        )>,
    >,
) {
    for (name, mut node) in &mut legacy_roots {
        if name.as_str() == "Leader AI council workspace" {
            node.display = Display::None;
        }
    }
    for mut node in &mut superseded_surfaces {
        node.display = Display::None;
    }
}

#[allow(clippy::type_complexity)]
fn sync_live_shell_visibility_and_copy(
    live: Res<'_, Lai54LiveShell>,
    start: Res<'_, crate::StartScreen>,
    mut legacy_start: Query<'_, '_, &mut Node, With<crate::StartScreenRoot>>,
    mut charter_root: Query<
        '_,
        '_,
        &mut Node,
        (With<Lai54StartCharterRoot>, Without<crate::StartScreenRoot>),
    >,
    mut primary_root: Query<
        '_,
        '_,
        &mut Node,
        (
            With<Lai54PrimarySurfaceRoot>,
            Without<Lai54StartCharterRoot>,
        ),
    >,
    mut council_tabs: Query<'_, '_, &mut Node, With<Lai54CouncilTabsRoot>>,
    mut primary_label: Query<'_, '_, &mut Text, With<Lai54PrimaryLabel>>,
    mut council_label: Query<'_, '_, &mut Text, With<Lai54CouncilLabel>>,
    mut session_label: Query<'_, '_, (&mut Text, &mut AccessibilityNode), With<Lai54SessionLabel>>,
    mut charter_summary: Query<'_, '_, &mut Text, With<Lai54CharterSummary>>,
    mut charter_feedback: Query<'_, '_, &mut Text, With<Lai54CharterFeedback>>,
) {
    for mut node in &mut legacy_start {
        node.display = Display::None;
    }
    if let Ok(mut node) = charter_root.single_mut() {
        node.display = if start.visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    let visible_primary = live.router.visible_primary();
    if let Ok(mut node) = primary_root.single_mut() {
        node.display = if !start.visible && visible_primary.is_some() {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Ok(mut node) = council_tabs.single_mut() {
        node.display = if visible_primary == Some(PrimaryScreen::Council) {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Ok(mut text) = primary_label.single_mut() {
        text.0 = match visible_primary {
            Some(PrimaryScreen::Log) => "Log",
            Some(PrimaryScreen::Stores) => "Stores",
            Some(PrimaryScreen::Village) => "Village",
            Some(PrimaryScreen::Research) => "Research",
            Some(PrimaryScreen::Council) => "Council",
            None => "World",
        }
        .to_owned();
    }
    if let Ok(mut text) = council_label.single_mut() {
        text.0 = format!(
            "Council / {}",
            match live.router.council_tab() {
                CouncilTab::Plans => "Plans",
                CouncilTab::Tasks => "Tasks",
                CouncilTab::Cats => "Cats",
                CouncilTab::Hole => "Hole",
                CouncilTab::Diplomacy => "Diplomacy",
                CouncilTab::Trade => "Trade",
            }
        );
    }
    if let Ok((mut text, mut semantic)) = session_label.single_mut() {
        let label = match live.charter.connection {
            ConnectionState::Connected => "Session connected",
            ConnectionState::LoadingSnapshot => "Loading report-safe session",
            ConnectionState::Connecting => "Connecting session",
            ConnectionState::Reconnecting => "Session stale; reconnecting",
            ConnectionState::UpdateRequired => "Update required",
            ConnectionState::Disconnected => "Session disconnected",
            ConnectionState::Error => "Session error",
        };
        text.0 = label.to_owned();
        *semantic = semantic_status_node(
            "lai54.shell.session",
            label,
            matches!(
                live.charter.connection,
                ConnectionState::UpdateRequired | ConnectionState::Error
            ),
        );
    }
    if let Ok(mut text) = charter_summary.single_mut() {
        text.0 = format!(
            "Player: {}\nVillage: {}\nDestination: {}",
            if live.charter.player_name.is_empty() {
                "enter a name"
            } else {
                &live.charter.player_name
            },
            if live.charter.village_name.is_empty() {
                "name required for a new personal village"
            } else {
                &live.charter.village_name
            },
            match live.charter.selected_destination {
                Some(DestinationKind::Global) => "Global",
                Some(DestinationKind::Personal) => "Personal",
                None => "choose a destination",
            },
        );
    }
    if let Ok(mut text) = charter_feedback.single_mut() {
        text.0 = match live.charter.entry_control_state() {
            EntryControlState::Enabled => {
                "Ready. Continue only acts after explicit activation.".to_owned()
            }
            EntryControlState::Disabled(reason) => {
                format!("Entry unavailable: {}", reason.label_key().0)
            }
        };
    }
}
