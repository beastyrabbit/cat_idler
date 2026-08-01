//! Animated, save-independent entry surface for Idle Cat Forest.

use super::*;

const CHARTER_WIDTH: f32 = 560.0;
const START_BANNER_HEIGHT: f32 = 66.0;
const START_BANNER_VERTICAL_PADDING: f32 = 8.0;
const START_BANNER_TITLE_SIZE: f32 = 23.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum StartMode {
    Global,
    Personal,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StartInput {
    PlayerName,
    VillageName,
}

/// Full-window entry flow over the save-independent mature showcase. This
/// resource contains only authoritative user choices and durable entry state.
#[derive(Resource, Debug)]
pub(super) struct StartScreen {
    pub(super) visible: bool,
    mode: Option<StartMode>,
    focused_input: StartInput,
    player_name: String,
    village_name: String,
    pub(super) pending_foundation: bool,
    pub(super) error: Option<String>,
    restored_destination_id: Option<String>,
    restored_mode_applied: bool,
    needs_village_name: bool,
}

impl Default for StartScreen {
    fn default() -> Self {
        Self {
            visible: true,
            mode: None,
            focused_input: StartInput::PlayerName,
            player_name: String::new(),
            village_name: String::new(),
            pending_foundation: false,
            error: None,
            restored_destination_id: None,
            restored_mode_applied: false,
            needs_village_name: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StartDestinationCopy {
    title: String,
    detail: String,
}

fn population_copy(population: usize) -> String {
    if population == 1 {
        "1 Katze".to_owned()
    } else {
        format!("{population} Katzen")
    }
}

fn destination_copy(village: Option<(&str, usize)>, global: bool) -> StartDestinationCopy {
    match (village, global) {
        (Some((name, population)), true) => StartDestinationCopy {
            title: name.to_owned(),
            detail: format!("Gemeinsame Welt · {}", population_copy(population)),
        },
        (Some((name, population)), false) => StartDestinationCopy {
            title: name.to_owned(),
            detail: format!("Deine Siedlung · {}", population_copy(population)),
        },
        (None, true) => StartDestinationCopy {
            title: "Grand Commons".to_owned(),
            detail: "Gemeinsame Welt".to_owned(),
        },
        (None, false) => StartDestinationCopy {
            title: "Neue Siedlung".to_owned(),
            detail: "Gründe deine eigene Zukunft".to_owned(),
        },
    }
}

fn restored_mode_for(
    selected_id: Option<&str>,
    colonies: &[(&str, VillageKind)],
) -> Option<StartMode> {
    let selected_id = selected_id?;
    colonies
        .iter()
        .find(|(id, _)| *id == selected_id)
        .map(|(_, kind)| match kind {
            VillageKind::Global => StartMode::Global,
            VillageKind::Personal => StartMode::Personal,
        })
}

#[derive(Clone, Copy)]
struct StartReadinessInput<'a> {
    session_ready: bool,
    has_snapshot: bool,
    player_name: &'a str,
    mode: Option<StartMode>,
    needs_village_name: bool,
    village_name: &'a str,
    pending_foundation: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StartSubmissionBlock {
    Loading,
    PlayerName,
    Destination,
    VillageName,
    Founding,
}

impl StartSubmissionBlock {
    fn message(self) -> &'static str {
        match self {
            Self::Loading => "Die Spielwelt wird noch geladen.",
            Self::PlayerName => "Gib deinem Spieler einen Namen mit mindestens 2 Zeichen.",
            Self::Destination => "Wähle eine Spielwelt.",
            Self::VillageName => "Gib deiner Siedlung einen Namen mit mindestens 2 Zeichen.",
            Self::Founding => "Deine Siedlung wird gerade gegründet.",
        }
    }
}

fn visible_name_is_ready(value: &str, max_chars: usize) -> bool {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    (2..=max_chars).contains(&normalized.chars().count())
}

fn start_submission_block(input: StartReadinessInput<'_>) -> Option<StartSubmissionBlock> {
    if input.pending_foundation {
        Some(StartSubmissionBlock::Founding)
    } else if !input.session_ready || !input.has_snapshot {
        Some(StartSubmissionBlock::Loading)
    } else if !visible_name_is_ready(input.player_name, PLAYER_NAME_MAX_CHARS) {
        Some(StartSubmissionBlock::PlayerName)
    } else if input.mode.is_none() {
        Some(StartSubmissionBlock::Destination)
    } else if input.needs_village_name
        && !visible_name_is_ready(input.village_name, VILLAGE_NAME_MAX_CHARS)
    {
        Some(StartSubmissionBlock::VillageName)
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct StartLayoutProfile {
    effective_width: f32,
    effective_height: f32,
    charter_width: f32,
    max_charter_height: f32,
    stack_destinations: bool,
    side_by_side_with_world: bool,
}

fn start_layout_profile(effective_width: f32, effective_height: f32) -> StartLayoutProfile {
    StartLayoutProfile {
        effective_width,
        effective_height,
        charter_width: CHARTER_WIDTH.min(effective_width * 0.92),
        max_charter_height: effective_height * 0.92,
        stack_destinations: effective_width < 860.0 || effective_height < 640.0,
        side_by_side_with_world: effective_width >= 1_280.0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct StartInputAppearance {
    border: Color,
    background: Color,
    label: Color,
    show_rail: bool,
}

fn start_input_appearance(focused: bool) -> StartInputAppearance {
    if focused {
        StartInputAppearance {
            border: UI_ACCENT,
            background: Color::srgb(1.0, 0.93, 0.72),
            label: UI_ACCENT,
            show_rail: true,
        }
    } else {
        StartInputAppearance {
            border: UI_DIVIDER,
            background: Color::srgb(0.89, 0.83, 0.68),
            label: UI_INK,
            show_rail: false,
        }
    }
}

#[derive(Component)]
pub(super) struct StartScreenRoot;
#[derive(Component)]
pub(super) struct StartCharter;
#[derive(Component)]
pub(super) struct StartDestinationRow;
#[derive(Component)]
struct StartPlayerInput;
#[derive(Component)]
struct StartVillageInput;
#[derive(Component)]
struct StartPlayerInputText;
#[derive(Component)]
struct StartVillageInputText;
#[derive(Component)]
struct StartPlayerLabel;
#[derive(Component)]
struct StartVillageLabel;
#[derive(Component)]
struct StartPlayerFocusRail;
#[derive(Component)]
struct StartVillageFocusRail;
#[derive(Component)]
struct StartGlobalButton;
#[derive(Component)]
struct StartPersonalButton;
#[derive(Component)]
struct StartGlobalTitle;
#[derive(Component)]
struct StartGlobalDetail;
#[derive(Component)]
struct StartPersonalTitle;
#[derive(Component)]
struct StartPersonalDetail;
#[derive(Component)]
struct StartGlobalSelected;
#[derive(Component)]
struct StartPersonalSelected;
#[derive(Component)]
struct StartSelectedSummary;
#[derive(Component)]
struct StartConnectionText;
#[derive(Component)]
struct StartHelperText;
#[derive(Component)]
struct StartContinueButton;
#[derive(Component)]
struct StartContinueText;
#[derive(Component)]
struct StartVillageField;
#[derive(Component)]
struct StartErrorText;
#[derive(Component)]
pub(super) struct StartScrollViewport;

type StartRootQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Node,
    (
        With<StartScreenRoot>,
        Without<StartCharter>,
        Without<StartDestinationRow>,
    ),
>;

#[derive(SystemParam)]
#[allow(clippy::type_complexity)]
pub(super) struct StartScreenUi<'w, 's> {
    player_input:
        Query<'w, 's, &'static Interaction, (Changed<Interaction>, With<StartPlayerInput>)>,
    village_input:
        Query<'w, 's, &'static Interaction, (Changed<Interaction>, With<StartVillageInput>)>,
    global_button:
        Query<'w, 's, &'static Interaction, (Changed<Interaction>, With<StartGlobalButton>)>,
    personal_button:
        Query<'w, 's, &'static Interaction, (Changed<Interaction>, With<StartPersonalButton>)>,
    continue_button:
        Query<'w, 's, &'static Interaction, (Changed<Interaction>, With<StartContinueButton>)>,
    nodes: ParamSet<
        'w,
        's,
        (
            Query<'w, 's, &'static mut Node, With<StartScreenRoot>>,
            Query<'w, 's, &'static mut Node, With<StartVillageField>>,
            Query<'w, 's, &'static mut Node, With<StartGlobalSelected>>,
            Query<'w, 's, &'static mut Node, With<StartPersonalSelected>>,
            Query<'w, 's, &'static mut Node, With<StartPlayerFocusRail>>,
            Query<'w, 's, &'static mut Node, With<StartVillageFocusRail>>,
        ),
    >,
    texts: ParamSet<
        'w,
        's,
        (
            Query<'w, 's, &'static mut Text, With<StartPlayerInputText>>,
            Query<'w, 's, &'static mut Text, With<StartVillageInputText>>,
            Query<'w, 's, &'static mut Text, With<StartGlobalTitle>>,
            Query<'w, 's, &'static mut Text, With<StartGlobalDetail>>,
            Query<'w, 's, &'static mut Text, With<StartPersonalTitle>>,
            Query<'w, 's, &'static mut Text, With<StartPersonalDetail>>,
            Query<'w, 's, &'static mut Text, With<StartSelectedSummary>>,
            Query<'w, 's, &'static mut Text, With<StartConnectionText>>,
        ),
    >,
    helper_text: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<StartHelperText>,
            Without<StartPlayerInputText>,
            Without<StartVillageInputText>,
            Without<StartGlobalTitle>,
            Without<StartGlobalDetail>,
            Without<StartPersonalTitle>,
            Without<StartPersonalDetail>,
            Without<StartSelectedSummary>,
            Without<StartConnectionText>,
        ),
    >,
    continue_text: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<StartContinueText>,
            Without<StartPlayerInputText>,
            Without<StartVillageInputText>,
            Without<StartGlobalTitle>,
            Without<StartGlobalDetail>,
            Without<StartPersonalTitle>,
            Without<StartPersonalDetail>,
            Without<StartSelectedSummary>,
            Without<StartConnectionText>,
            Without<StartHelperText>,
        ),
    >,
    error_text: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<StartErrorText>,
            Without<StartPlayerInputText>,
            Without<StartVillageInputText>,
            Without<StartGlobalTitle>,
            Without<StartGlobalDetail>,
            Without<StartPersonalTitle>,
            Without<StartPersonalDetail>,
            Without<StartSelectedSummary>,
            Without<StartConnectionText>,
            Without<StartHelperText>,
            Without<StartContinueText>,
        ),
    >,
    connection_color: Query<
        'w,
        's,
        &'static mut TextColor,
        (
            With<StartConnectionText>,
            Without<StartPlayerLabel>,
            Without<StartVillageLabel>,
        ),
    >,
    input_borders: ParamSet<
        'w,
        's,
        (
            Query<'w, 's, &'static mut BorderColor, With<StartPlayerInput>>,
            Query<'w, 's, &'static mut BorderColor, With<StartVillageInput>>,
        ),
    >,
    input_backgrounds: ParamSet<
        'w,
        's,
        (
            Query<'w, 's, &'static mut BackgroundColor, With<StartPlayerInput>>,
            Query<'w, 's, &'static mut BackgroundColor, With<StartVillageInput>>,
        ),
    >,
    label_colors: ParamSet<
        'w,
        's,
        (
            Query<'w, 's, &'static mut TextColor, With<StartPlayerLabel>>,
            Query<'w, 's, &'static mut TextColor, With<StartVillageLabel>>,
        ),
    >,
    card_toggles: ParamSet<
        'w,
        's,
        (
            Query<'w, 's, &'static mut KitToggle, With<StartGlobalButton>>,
            Query<'w, 's, &'static mut KitToggle, With<StartPersonalButton>>,
        ),
    >,
    continue_disabled: Query<'w, 's, &'static mut KitDisabled, With<StartContinueButton>>,
}

fn start_input_bundle() -> impl Bundle {
    (
        Button,
        Node {
            position_type: PositionType::Relative,
            width: Val::Percent(100.0),
            min_height: Val::Px(48.0),
            padding: UiRect::axes(Val::Px(18.0), Val::Px(0.0)),
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(start_input_appearance(false).background),
        BorderColor::all(start_input_appearance(false).border),
    )
}

fn destination_button_node() -> Node {
    Node {
        width: Val::Px(268.0),
        min_width: Val::Px(0.0),
        min_height: Val::Px(82.0),
        flex_grow: 1.0,
        padding: UiRect::all(Val::Px(14.0)),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(4.0),
        border: UiRect::all(Val::Px(3.0)),
        ..default()
    }
}

pub(super) fn spawn_start_screen(
    commands: &mut Commands,
    assets: &AssetServer,
    ui_art: &AdventureUiArt,
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            GlobalZIndex(START_SURFACE_Z),
            // The mature showcase remains visible and animated beneath the
            // blocking charter. This restrained scrim separates the form.
            BackgroundColor(Color::srgba(0.02, 0.03, 0.025, 0.28)),
            StartScreenRoot,
            UiSurfaceRoot(UiSurfaceKind::Modal),
            WorldInputBlocker,
        ))
        .with_children(|screen| {
            screen
                .spawn((
                    Node {
                        width: Val::Px(CHARTER_WIDTH),
                        max_width: Val::Percent(92.0),
                        height: Val::Px(470.0),
                        max_height: Val::Percent(92.0),
                        min_height: Val::Px(0.0),
                        flex_direction: FlexDirection::Column,
                        border: UiRect::all(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(UI_BG),
                    BorderColor::all(Color::NONE),
                    ImageNode {
                        image: assets.load("public/images/game/ui/panel-ornate.png"),
                        image_mode: NodeImageMode::Sliced(panel_slicer()),
                        visual_box: VisualBox::BorderBox,
                        ..default()
                    },
                    ZIndex(100),
                    StartCharter,
                ))
                .with_children(|charter| {
                    charter.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(START_BANNER_HEIGHT),
                            min_height: Val::Px(START_BANNER_HEIGHT),
                            flex_shrink: 0.0,
                            padding: UiRect::axes(
                                Val::Px(24.0),
                                Val::Px(START_BANNER_VERTICAL_PADDING),
                            ),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                        ImageNode {
                            image: ui_art.banner.clone(),
                            image_mode: NodeImageMode::Stretch,
                            ..default()
                        },
                        children![ui_text(
                            "IDLE CAT FOREST",
                            START_BANNER_TITLE_SIZE,
                            UI_TITLE_INK
                        )],
                    ));
                    let viewport = spawn_vertical_scroll_area(charter, 20.0, 8.0, |body| {
                        body.spawn((
                            ui_text(
                                "Verbindung zum Wald wird hergestellt …",
                                FS_SMALL,
                                UI_ACCENT,
                            ),
                            StartConnectionText,
                        ));
                        body.spawn((ui_text("Dein Name", FS_SECTION, UI_INK), StartPlayerLabel));
                        body.spawn((
                            start_input_bundle(),
                            StartPlayerInput,
                            children![
                                (
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(0.0),
                                        top: Val::Px(3.0),
                                        bottom: Val::Px(3.0),
                                        width: Val::Px(5.0),
                                        display: Display::None,
                                        ..default()
                                    },
                                    BackgroundColor(UI_ACCENT),
                                    StartPlayerFocusRail,
                                ),
                                (
                                    ui_text("Name eingeben …", 15.0, UI_MUTED),
                                    StartPlayerInputText,
                                ),
                            ],
                        ));
                        body.spawn(ui_text("Deine Spielwelt", FS_SECTION, UI_INK));
                        body.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(10.0),
                                row_gap: Val::Px(10.0),
                                ..default()
                            },
                            StartDestinationRow,
                        ))
                        .with_children(|row| {
                            row.spawn((
                                Button,
                                destination_button_node(),
                                BackgroundColor(UI_BUTTON_BROWN),
                                BorderColor::all(Color::NONE),
                                ImageNode::default(),
                                KitButton,
                                KitToggle::default(),
                                StartGlobalButton,
                            ))
                            .with_children(|card| {
                                card.spawn((
                                    ui_text("AUSGEWÄHLT", FS_SMALL, Color::srgb(1.0, 0.88, 0.55)),
                                    Node {
                                        display: Display::None,
                                        ..default()
                                    },
                                    StartGlobalSelected,
                                ));
                                card.spawn((
                                    ui_text("Grand Commons", 16.0, UI_INK),
                                    StartGlobalTitle,
                                ));
                                card.spawn((
                                    ui_text_wrapped("Gemeinsame Welt", FS_BODY, UI_MUTED),
                                    StartGlobalDetail,
                                ));
                            });
                            row.spawn((
                                Button,
                                destination_button_node(),
                                BackgroundColor(UI_BUTTON_BROWN),
                                BorderColor::all(Color::NONE),
                                ImageNode::default(),
                                KitButton,
                                KitToggle::default(),
                                StartPersonalButton,
                            ))
                            .with_children(|card| {
                                card.spawn((
                                    ui_text("AUSGEWÄHLT", FS_SMALL, Color::srgb(1.0, 0.88, 0.55)),
                                    Node {
                                        display: Display::None,
                                        ..default()
                                    },
                                    StartPersonalSelected,
                                ));
                                card.spawn((
                                    ui_text("Neue Siedlung", 16.0, UI_INK),
                                    StartPersonalTitle,
                                ));
                                card.spawn((
                                    ui_text_wrapped(
                                        "Gründe deine eigene Zukunft",
                                        FS_BODY,
                                        UI_MUTED,
                                    ),
                                    StartPersonalDetail,
                                ));
                            });
                        });
                        body.spawn((
                            ui_text_wrapped("Noch keine Spielwelt ausgewählt.", FS_BODY, UI_MUTED),
                            StartSelectedSummary,
                        ));
                        body.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(6.0),
                                display: Display::None,
                                ..default()
                            },
                            StartVillageField,
                        ))
                        .with_children(|field| {
                            field.spawn((
                                ui_text("Name deiner Siedlung", FS_SECTION, UI_INK),
                                StartVillageLabel,
                            ));
                            field.spawn((
                                start_input_bundle(),
                                StartVillageInput,
                                children![
                                    (
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: Val::Px(0.0),
                                            top: Val::Px(3.0),
                                            bottom: Val::Px(3.0),
                                            width: Val::Px(5.0),
                                            display: Display::None,
                                            ..default()
                                        },
                                        BackgroundColor(UI_ACCENT),
                                        StartVillageFocusRail,
                                    ),
                                    (
                                        ui_text("Siedlungsname eingeben …", 15.0, UI_MUTED),
                                        StartVillageInputText,
                                    ),
                                ],
                            ));
                        });
                    });
                    charter
                        .commands()
                        .entity(viewport)
                        .insert(StartScrollViewport);
                    charter
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            flex_shrink: 0.0,
                            padding: UiRect::axes(Val::Px(24.0), Val::Px(10.0)),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(6.0),
                            ..default()
                        })
                        .with_children(|footer| {
                            footer.spawn((
                                ui_text_wrapped(
                                    "Die Spielwelt wird noch geladen.",
                                    FS_SMALL,
                                    UI_MUTED,
                                ),
                                StartHelperText,
                            ));
                            footer.spawn((ui_text("", FS_BODY, UI_WARNING), StartErrorText));
                            footer.spawn((
                                ui_button(),
                                StartContinueButton,
                                KitDisabled { disabled: true },
                                children![(
                                    ui_text("Spielwelt wird geladen …", 15.0, UI_INK),
                                    StartContinueText,
                                )],
                            ));
                        });
                });
        });
}

pub(super) fn initialize_start_screen(
    session: Res<Session>,
    selection: Res<VillageSelection>,
    mut start: ResMut<StartScreen>,
) {
    start.player_name.clone_from(&session.nickname);
    if selection.join_required {
        start
            .restored_destination_id
            .clone_from(&selection.selected_id);
    }
}

pub(super) fn normalized_required_name(
    raw: &str,
    label: &str,
    max_chars: usize,
) -> Result<String, String> {
    let normalized = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let count = normalized.chars().count();
    if count < 2 {
        return Err(format!("{label} muss mindestens 2 Zeichen lang sein."));
    }
    if count > max_chars {
        return Err(format!(
            "{label} darf höchstens {max_chars} Zeichen lang sein."
        ));
    }
    if normalized.chars().any(char::is_control) {
        return Err(format!("{label} enthält ungültige Zeichen."));
    }
    Ok(normalized)
}

fn append_input_text(target: &mut String, text: &str, max_chars: usize) {
    for character in text.chars().filter(|character| !character.is_control()) {
        if target.chars().count() >= max_chars {
            break;
        }
        target.push(character);
    }
}

pub(super) fn start_input_label(value: &str, placeholder: &str, focused: bool) -> String {
    match (value.is_empty(), focused) {
        (true, true) => format!("| {placeholder}"),
        (true, false) => placeholder.to_owned(),
        (false, true) => format!("{value} |"),
        (false, false) => value.to_owned(),
    }
}

fn start_connection_copy(
    state: &ConnectionState,
    session_ready: bool,
    has_snapshot: bool,
) -> (String, Color) {
    if session_ready && has_snapshot {
        return ("SPIELWELT BEREIT".to_owned(), UI_POSITIVE);
    }
    match state.phase {
        ConnectionPhase::Connected => ("Die Spielwelt wird geladen …".to_owned(), UI_ACCENT),
        ConnectionPhase::WaitingToRetry => (
            format!(
                "Neuer Verbindungsversuch in {:.0} Sekunden …",
                state.retry_remaining_secs.ceil()
            ),
            UI_WARNING,
        ),
        ConnectionPhase::Incompatible => (
            "Diese Spielversion braucht ein Update.".to_owned(),
            UI_WARNING,
        ),
        ConnectionPhase::Disconnected => (
            "Der Wald ist gerade nicht erreichbar.".to_owned(),
            UI_WARNING,
        ),
        ConnectionPhase::Connecting => (
            "Verbindung zum Wald wird hergestellt …".to_owned(),
            UI_ACCENT,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_start_screen(
    mut keyboard: MessageReader<KeyboardInput>,
    mut start: ResMut<StartScreen>,
    mut session: ResMut<Session>,
    connection: Res<ConnectionState>,
    mut latest: ResMut<LatestSnapshot>,
    mut selection: ResMut<VillageSelection>,
    mut outgoing: ResMut<OutgoingActions>,
    mut ui: StartScreenUi,
) {
    {
        let mut roots = ui.nodes.p0();
        let Ok(mut root) = roots.single_mut() else {
            return;
        };
        root.display = if start.visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    if !start.visible {
        keyboard.clear();
        return;
    }

    if !start.restored_mode_applied
        && let Some(snapshot) = latest.0.as_ref()
    {
        let colonies = snapshot
            .colonies
            .iter()
            .map(|colony| (colony.id.as_str(), colony.kind))
            .collect::<Vec<_>>();
        start.mode = restored_mode_for(start.restored_destination_id.as_deref(), &colonies);
        start.restored_mode_applied = true;
    }

    if ui
        .player_input
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        start.focused_input = StartInput::PlayerName;
        start.error = None;
    }
    if ui
        .village_input
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        start.focused_input = StartInput::VillageName;
        start.error = None;
    }
    if ui
        .global_button
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        start.mode = Some(StartMode::Global);
        start.error = None;
    }
    if ui
        .personal_button
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        start.mode = Some(StartMode::Personal);
        start.focused_input = if start.player_name.trim().is_empty() {
            StartInput::PlayerName
        } else {
            StartInput::VillageName
        };
        start.error = None;
    }

    let mut submit = ui
        .continue_button
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);
    for event in keyboard.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        match event.key_code {
            KeyCode::Backspace => match start.focused_input {
                StartInput::PlayerName => {
                    start.player_name.pop();
                }
                StartInput::VillageName => {
                    start.village_name.pop();
                }
            },
            KeyCode::Tab => {
                start.focused_input = match start.focused_input {
                    StartInput::PlayerName if start.mode == Some(StartMode::Personal) => {
                        StartInput::VillageName
                    }
                    _ => StartInput::PlayerName,
                };
            }
            KeyCode::Enter | KeyCode::NumpadEnter => submit = true,
            _ => {
                if let Some(text) = event.text.as_deref() {
                    match start.focused_input {
                        StartInput::PlayerName => {
                            append_input_text(&mut start.player_name, text, PLAYER_NAME_MAX_CHARS)
                        }
                        StartInput::VillageName => {
                            append_input_text(&mut start.village_name, text, VILLAGE_NAME_MAX_CHARS)
                        }
                    }
                }
            }
        }
    }

    let global = latest.0.as_ref().and_then(|snapshot| {
        snapshot
            .colonies
            .iter()
            .find(|colony| colony.kind == VillageKind::Global)
            .map(|colony| (colony.name.clone(), colony.cats.len()))
    });
    let owned_personal = latest.0.as_ref().and_then(|snapshot| {
        snapshot
            .colonies
            .iter()
            .find(|colony| colony.kind == VillageKind::Personal && colony.capabilities.is_owner)
            .map(|colony| (colony.id.clone(), colony.name.clone(), colony.cats.len()))
    });
    start.needs_village_name = start.mode == Some(StartMode::Personal) && owned_personal.is_none();
    let readiness = StartReadinessInput {
        session_ready: session.ready,
        has_snapshot: latest.0.is_some(),
        player_name: &start.player_name,
        mode: start.mode,
        needs_village_name: start.needs_village_name,
        village_name: &start.village_name,
        pending_foundation: start.pending_foundation,
    };
    let blocked = start_submission_block(readiness);

    if submit && !start.pending_foundation {
        if let Some(reason) = blocked {
            start.error = Some(reason.message().to_owned());
        } else {
            let result = (|| -> Result<(), String> {
                let nickname = normalized_required_name(
                    &start.player_name,
                    "Dein Name",
                    PLAYER_NAME_MAX_CHARS,
                )?;
                let mode = start
                    .mode
                    .ok_or_else(|| "Bitte wähle eine Spielwelt.".to_owned())?;
                let snapshot = latest
                    .0
                    .as_mut()
                    .ok_or_else(|| "Die Spielwelt wird noch geladen …".to_owned())?;
                session.nickname = nickname;
                outgoing.0.push(presence_action(&session));
                match mode {
                    StartMode::Global => {
                        let global_id = snapshot
                            .colonies
                            .iter()
                            .find(|colony| colony.kind == VillageKind::Global)
                            .map(|colony| colony.id.clone())
                            .ok_or_else(|| {
                                "Die globale Siedlung ist nicht verfügbar.".to_owned()
                            })?;
                        if let Some(action) =
                            choose_village(&global_id, snapshot, &mut selection, &session)
                        {
                            outgoing.0.push(action);
                        }
                        start.visible = false;
                    }
                    StartMode::Personal => {
                        if let Some(personal_id) =
                            owned_personal.as_ref().map(|(id, _, _)| id.as_str())
                        {
                            if let Some(action) =
                                choose_village(personal_id, snapshot, &mut selection, &session)
                            {
                                outgoing.0.push(action);
                            }
                            start.visible = false;
                        } else {
                            let name = normalized_required_name(
                                &start.village_name,
                                "Der Siedlungsname",
                                VILLAGE_NAME_MAX_CHARS,
                            )?;
                            outgoing.0.push(ClientAction::FoundVillage {
                                name,
                                session_id: session.session_id.clone(),
                                sig: Some(session.sig.clone()),
                            });
                            start.pending_foundation = true;
                        }
                    }
                }
                persist_session(&session, &selection)
                    .map_err(|err| format!("Zugang konnte nicht gespeichert werden: {err}"))?;
                Ok(())
            })();
            start.error = result.err();
        }
    }

    if let Ok(mut node) = ui.nodes.p1().single_mut() {
        node.display = if start.needs_village_name {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Ok(mut text) = ui.texts.p0().single_mut() {
        text.0 = start_input_label(
            &start.player_name,
            "Name eingeben …",
            start.focused_input == StartInput::PlayerName,
        );
    }
    if let Ok(mut text) = ui.texts.p1().single_mut() {
        text.0 = start_input_label(
            &start.village_name,
            "Siedlungsname eingeben …",
            start.focused_input == StartInput::VillageName,
        );
    }

    let global_copy = destination_copy(
        global
            .as_ref()
            .map(|(name, population)| (name.as_str(), *population)),
        true,
    );
    let personal_copy = destination_copy(
        owned_personal
            .as_ref()
            .map(|(_, name, population)| (name.as_str(), *population)),
        false,
    );
    if let Ok(mut text) = ui.texts.p2().single_mut() {
        text.0 = global_copy.title.clone();
    }
    if let Ok(mut text) = ui.texts.p3().single_mut() {
        text.0 = global_copy.detail;
    }
    if let Ok(mut text) = ui.texts.p4().single_mut() {
        text.0 = personal_copy.title.clone();
    }
    if let Ok(mut text) = ui.texts.p5().single_mut() {
        text.0 = personal_copy.detail;
    }
    if let Ok(mut text) = ui.texts.p6().single_mut() {
        text.0 = match start.mode {
            Some(StartMode::Global) => format!("Ausgewählt: {}", global_copy.title),
            Some(StartMode::Personal) => format!("Ausgewählt: {}", personal_copy.title),
            None => "Noch keine Spielwelt ausgewählt.".to_owned(),
        };
    }
    let (connection_copy, connection_color) =
        start_connection_copy(&connection, session.ready, latest.0.is_some());
    if let Ok(mut text) = ui.texts.p7().single_mut() {
        let copy = connection_copy;
        text.0 = copy;
    }
    if let Ok(mut color) = ui.connection_color.single_mut() {
        color.0 = connection_color;
    }
    if let Ok(mut text) = ui.helper_text.single_mut() {
        text.0 = blocked.map_or_else(
            || "Alles bereit. Deine Spielwelt wartet.".to_owned(),
            |reason| reason.message().to_owned(),
        );
    }
    if let Ok(mut text) = ui.continue_text.single_mut() {
        text.0 = if blocked == Some(StartSubmissionBlock::Loading) {
            "Spielwelt wird geladen …"
        } else if blocked == Some(StartSubmissionBlock::Founding) {
            "Siedlung wird gegründet …"
        } else {
            match (start.mode, owned_personal.is_some()) {
                (Some(StartMode::Global), _) => "Grand Commons betreten",
                (Some(StartMode::Personal), true) => "Eigene Siedlung fortsetzen",
                (Some(StartMode::Personal), false) => "Eigene Siedlung gründen",
                (None, _) => "Spielwelt auswählen",
            }
        }
        .to_owned();
    }
    if let Ok(mut disabled) = ui.continue_disabled.single_mut() {
        disabled.disabled = blocked.is_some();
    }
    if let Ok(mut text) = ui.error_text.single_mut() {
        text.0 = start.error.clone().unwrap_or_default();
    }

    if let Ok(mut toggle) = ui.card_toggles.p0().single_mut() {
        toggle.active = start.mode == Some(StartMode::Global);
    }
    if let Ok(mut toggle) = ui.card_toggles.p1().single_mut() {
        toggle.active = start.mode == Some(StartMode::Personal);
    }
    if let Ok(mut node) = ui.nodes.p2().single_mut() {
        node.display = if start.mode == Some(StartMode::Global) {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Ok(mut node) = ui.nodes.p3().single_mut() {
        node.display = if start.mode == Some(StartMode::Personal) {
            Display::Flex
        } else {
            Display::None
        };
    }

    for (player, focused) in [
        (true, start.focused_input == StartInput::PlayerName),
        (false, start.focused_input == StartInput::VillageName),
    ] {
        let appearance = start_input_appearance(focused);
        if player {
            if let Ok(mut border) = ui.input_borders.p0().single_mut() {
                *border = BorderColor::all(appearance.border);
            }
            if let Ok(mut background) = ui.input_backgrounds.p0().single_mut() {
                background.0 = appearance.background;
            }
            if let Ok(mut color) = ui.label_colors.p0().single_mut() {
                color.0 = appearance.label;
            }
            if let Ok(mut rail) = ui.nodes.p4().single_mut() {
                rail.display = if appearance.show_rail {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        } else {
            if let Ok(mut border) = ui.input_borders.p1().single_mut() {
                *border = BorderColor::all(appearance.border);
            }
            if let Ok(mut background) = ui.input_backgrounds.p1().single_mut() {
                background.0 = appearance.background;
            }
            if let Ok(mut color) = ui.label_colors.p1().single_mut() {
                color.0 = appearance.label;
            }
            if let Ok(mut rail) = ui.nodes.p5().single_mut() {
                rail.display = if appearance.show_rail {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }
    }
}

pub(super) fn update_start_screen_layout(
    start: Res<StartScreen>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    mut root: StartRootQuery,
    mut charter: Query<&mut Node, With<StartCharter>>,
    mut destinations: Query<&mut Node, (With<StartDestinationRow>, Without<StartCharter>)>,
) {
    if !start.visible {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let profile = start_layout_profile(
        window.width() / ui_scale.0.max(0.01),
        window.height() / ui_scale.0.max(0.01),
    );
    if let Ok(mut node) = root.single_mut() {
        node.justify_content = if profile.side_by_side_with_world {
            JustifyContent::FlexStart
        } else {
            JustifyContent::Center
        };
        node.padding = if profile.side_by_side_with_world {
            UiRect::left(Val::Px(48.0))
        } else {
            UiRect::ZERO
        };
    }
    if let Ok(mut node) = charter.single_mut() {
        node.width = Val::Px(profile.charter_width);
        let desired_height: f32 = match (profile.stack_destinations, start.needs_village_name) {
            (false, false) => 470.0,
            (false, true) => 535.0,
            (true, false) => 550.0,
            (true, true) => 620.0,
        };
        node.height = Val::Px(desired_height.min(profile.max_charter_height));
    }
    if let Ok(mut node) = destinations.single_mut() {
        node.flex_direction = if profile.stack_destinations {
            FlexDirection::Column
        } else {
            FlexDirection::Row
        };
    }
}

fn next_start_scroll_offset(current: f32, wheel_delta: f32, maximum: f32) -> f32 {
    (current + wheel_delta).clamp(0.0, maximum.max(0.0))
}

fn gameplay_chrome_display(start_visible: bool) -> Display {
    if start_visible {
        Display::None
    } else {
        Display::Flex
    }
}

pub(super) fn sync_start_screen_gameplay_chrome(
    start: Res<StartScreen>,
    mut chrome: Query<&mut Node, With<GameplayChrome>>,
) {
    if !start.is_changed() {
        return;
    }
    let display = gameplay_chrome_display(start.visible);
    for mut node in &mut chrome {
        node.display = display;
    }
}

pub(super) fn scroll_start_screen(
    start: Res<StartScreen>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut wheel: MessageReader<MouseWheel>,
    mut viewport: Query<
        (&ComputedNode, &UiGlobalTransform, &mut ScrollPosition),
        With<StartScrollViewport>,
    >,
) {
    if !start.visible {
        wheel.clear();
        return;
    }
    let Some(cursor) = windows
        .single()
        .ok()
        .and_then(|window| window.cursor_position())
    else {
        wheel.clear();
        return;
    };
    let Ok((computed, transform, mut position)) = viewport.single_mut() else {
        wheel.clear();
        return;
    };
    if !computed.contains_point(*transform, cursor) {
        wheel.clear();
        return;
    }
    let viewport_height = computed.size().y * computed.inverse_scale_factor();
    let content_height = computed.content_size().y * computed.inverse_scale_factor();
    let maximum = (content_height - viewport_height).max(0.0);
    let delta = wheel
        .read()
        .map(|event| match event.unit {
            bevy::input::mouse::MouseScrollUnit::Line => -event.y * 38.0,
            bevy::input::mouse::MouseScrollUnit::Pixel => -event.y,
        })
        .sum();
    position.y = next_start_scroll_offset(position.y, delta, maximum);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returning_players_restore_the_persisted_destination_without_auto_entering() {
        let colonies = [
            ("commons", VillageKind::Global),
            ("branchwood", VillageKind::Personal),
        ];
        assert_eq!(
            restored_mode_for(Some("commons"), &colonies),
            Some(StartMode::Global)
        );
        assert_eq!(
            restored_mode_for(Some("branchwood"), &colonies),
            Some(StartMode::Personal)
        );
        assert_eq!(restored_mode_for(Some("missing"), &colonies), None);
        assert_eq!(restored_mode_for(None, &colonies), None);
    }

    #[test]
    fn destination_copy_names_the_real_village_and_population() {
        assert_eq!(
            destination_copy(Some(("Branchwood", 15)), false),
            StartDestinationCopy {
                title: "Branchwood".to_owned(),
                detail: "Deine Siedlung · 15 Katzen".to_owned(),
            }
        );
        assert_eq!(
            destination_copy(None, false),
            StartDestinationCopy {
                title: "Neue Siedlung".to_owned(),
                detail: "Gründe deine eigene Zukunft".to_owned(),
            }
        );
        assert_eq!(
            destination_copy(Some(("Grand Commons", 30)), true),
            StartDestinationCopy {
                title: "Grand Commons".to_owned(),
                detail: "Gemeinsame Welt · 30 Katzen".to_owned(),
            }
        );
    }

    #[test]
    fn submission_readiness_explains_every_blocked_state() {
        let ready = StartReadinessInput {
            session_ready: true,
            has_snapshot: true,
            player_name: "Mara",
            mode: Some(StartMode::Global),
            needs_village_name: false,
            village_name: "",
            pending_foundation: false,
        };
        assert_eq!(start_submission_block(ready), None);
        assert_eq!(
            start_submission_block(StartReadinessInput {
                session_ready: false,
                ..ready
            }),
            Some(StartSubmissionBlock::Loading)
        );
        assert_eq!(
            start_submission_block(StartReadinessInput {
                player_name: "x",
                ..ready
            }),
            Some(StartSubmissionBlock::PlayerName)
        );
        assert_eq!(
            start_submission_block(StartReadinessInput {
                mode: None,
                ..ready
            }),
            Some(StartSubmissionBlock::Destination)
        );
        assert_eq!(
            start_submission_block(StartReadinessInput {
                mode: Some(StartMode::Personal),
                needs_village_name: true,
                village_name: "",
                ..ready
            }),
            Some(StartSubmissionBlock::VillageName)
        );
        assert_eq!(
            start_submission_block(StartReadinessInput {
                pending_foundation: true,
                ..ready
            }),
            Some(StartSubmissionBlock::Founding)
        );
    }

    #[test]
    fn compact_layout_stacks_cards_without_hiding_the_charter() {
        let regular = start_layout_profile(1_024.0, 768.0);
        assert!(!regular.stack_destinations);
        assert_eq!(regular.charter_width, CHARTER_WIDTH);
        assert!(!regular.side_by_side_with_world);

        let scaled = start_layout_profile(1_024.0 / 1.3, 768.0 / 1.3);
        assert!(scaled.stack_destinations);
        assert!(scaled.charter_width <= scaled.effective_width * 0.92);
        assert!(scaled.max_charter_height <= scaled.effective_height * 0.92);

        let wide = start_layout_profile(1_920.0, 1_080.0);
        assert!(wide.side_by_side_with_world);
    }

    #[test]
    fn banner_reserves_a_safe_single_title_row() {
        let estimated_text_height = START_BANNER_TITLE_SIZE * 1.35;
        let required_height = estimated_text_height + START_BANNER_VERTICAL_PADDING * 2.0;
        assert!(START_BANNER_HEIGHT >= required_height);
    }

    #[test]
    fn focused_input_uses_border_background_and_focus_rail() {
        let resting = start_input_appearance(false);
        let focused = start_input_appearance(true);
        assert_ne!(resting.border, focused.border);
        assert_ne!(resting.background, focused.background);
        assert!(!resting.show_rail);
        assert!(focused.show_rail);
    }

    #[test]
    fn start_scroll_stays_bounded_while_reaching_the_full_form() {
        assert_eq!(next_start_scroll_offset(0.0, 120.0, 300.0), 120.0);
        assert_eq!(next_start_scroll_offset(120.0, 400.0, 300.0), 300.0);
        assert_eq!(next_start_scroll_offset(120.0, -400.0, 300.0), 0.0);
    }

    #[test]
    fn landing_hides_gameplay_chrome_while_showcase_remains_visible() {
        assert_eq!(gameplay_chrome_display(true), Display::None);
        assert_eq!(gameplay_chrome_display(false), Display::Flex);
    }
}
