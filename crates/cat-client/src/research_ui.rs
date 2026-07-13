//! Full-window research ledger UI backed by `cat_sim::research_catalog`.

use super::*;
use bevy::{
    input::{ButtonState, keyboard::KeyboardInput, mouse::MouseScrollUnit},
    math::Rot2,
    ui::Val2,
};
use cat_protocol::ResearchSnapshot;
use cat_sim::{
    research_catalog::{ResearchCategory, ResearchPayload, research_catalog},
    upgrade_tree::UPGRADE_NODES,
};
use std::collections::{HashMap, HashSet};

const HEADER_HEIGHT: f32 = 96.0;
const NODE_WIDTH: f32 = 154.0;
const NODE_HEIGHT: f32 = 64.0;
const MAP_PADDING_X: f32 = 96.0;
const MAP_PADDING_Y: f32 = 92.0;
const MAP_STEP_X: f32 = 180.0;
const MAP_STEP_Y: f32 = 88.0;
const MAP_WIDTH: f32 = MAP_PADDING_X * 2.0 + MAP_STEP_X * 94.0;
const MAP_HEIGHT: f32 = MAP_PADDING_Y * 2.0 + MAP_STEP_Y * 12.0;
const MIN_SCALE: f32 = 0.42;
const MAX_SCALE: f32 = 1.35;

// A dry research ledger: dark walnut framing, sun-faded paper and restrained
// category inks. These are intentionally opaque so the world never competes
// with a screen containing five hundred pieces of information.
const LEDGER_PAPER: Color = Color::srgb(0.76, 0.70, 0.56);
const LEDGER_PAPER_DARK: Color = Color::srgb(0.58, 0.50, 0.37);
const LEDGER_INK: Color = Color::srgb(0.15, 0.105, 0.065);
const LEDGER_MUTED: Color = Color::srgb(0.36, 0.31, 0.24);
const BUILDING_INK: Color = Color::srgb(0.27, 0.39, 0.31);
const RECIPE_INK: Color = Color::srgb(0.48, 0.31, 0.18);
const UPGRADE_INK: Color = Color::srgb(0.27, 0.31, 0.47);
const OWNED_INK: Color = Color::srgb(0.25, 0.48, 0.28);
const READY_INK: Color = Color::srgb(0.68, 0.43, 0.10);
const LOCKED_INK: Color = Color::srgb(0.39, 0.36, 0.31);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CatalogNodeState {
    Owned,
    Available,
    Locked,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PurchaseState {
    Owned,
    Locked,
    LegacyReady,
    LegacyUnaffordable,
    IntegrationPending,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum ResearchFilter {
    #[default]
    All,
    Building,
    RecipeResource,
    Upgrade,
}

impl ResearchFilter {
    const ALL: [Self; 4] = [
        Self::All,
        Self::Building,
        Self::RecipeResource,
        Self::Upgrade,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Building => "Buildings",
            Self::RecipeResource => "Recipes & resources",
            Self::Upgrade => "Upgrades",
        }
    }

    fn includes(self, category: ResearchCategory) -> bool {
        matches!(self, Self::All)
            || matches!(
                (self, category),
                (Self::Building, ResearchCategory::Building)
                    | (Self::RecipeResource, ResearchCategory::RecipeResource)
                    | (Self::Upgrade, ResearchCategory::Upgrade)
            )
    }
}

#[derive(Clone, Copy, Debug)]
struct CatalogEntry {
    index: usize,
}

#[derive(Clone, Copy, Debug)]
struct CatalogConnector {
    from: usize,
    to: usize,
}

/// Logical catalog state is allocated once. Applying a snapshot only rewrites
/// this fixed state vector; it can never append cards or dependency lines.
#[derive(Resource)]
pub(super) struct ResearchUiModel {
    entries: Vec<CatalogEntry>,
    connectors: Vec<CatalogConnector>,
    by_id: HashMap<String, usize>,
    states: Vec<CatalogNodeState>,
}

impl ResearchUiModel {
    fn from_catalog() -> Self {
        let catalog = research_catalog();
        let by_id: HashMap<_, _> = catalog
            .nodes()
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.clone(), index))
            .collect();
        let entries = (0..catalog.nodes().len())
            .map(|index| CatalogEntry { index })
            .collect();
        let connectors = catalog
            .nodes()
            .iter()
            .enumerate()
            .flat_map(|(to, node)| {
                let by_id = &by_id;
                node.prerequisites.iter().map(move |id| CatalogConnector {
                    from: *by_id
                        .get(id)
                        .expect("validated research prerequisite must exist"),
                    to,
                })
            })
            .collect();
        let states = vec![CatalogNodeState::Locked; catalog.nodes().len()];
        Self {
            entries,
            connectors,
            by_id,
            states,
        }
    }

    #[cfg(test)]
    fn category_count(&self, category: ResearchCategory) -> usize {
        self.entries
            .iter()
            .filter(|entry| research_catalog().nodes()[entry.index].category == category)
            .count()
    }

    fn state_of(&self, id: &str, snapshot: &ResearchSnapshot) -> CatalogNodeState {
        let Some(index) = self.by_id.get(id).copied() else {
            return CatalogNodeState::Locked;
        };
        let node = &research_catalog().nodes()[index];
        let owned: HashSet<_> = snapshot.owned_node_ids.iter().map(String::as_str).collect();
        if owned.contains(node.id.as_str()) {
            CatalogNodeState::Owned
        } else if node
            .prerequisites
            .iter()
            .all(|prerequisite| owned.contains(prerequisite.as_str()))
        {
            CatalogNodeState::Available
        } else {
            CatalogNodeState::Locked
        }
    }

    fn apply_snapshot(&mut self, snapshot: &ResearchSnapshot) {
        let owned: HashSet<_> = snapshot.owned_node_ids.iter().map(String::as_str).collect();
        for (index, node) in research_catalog().nodes().iter().enumerate() {
            self.states[index] = if owned.contains(node.id.as_str()) {
                CatalogNodeState::Owned
            } else if node
                .prerequisites
                .iter()
                .all(|prerequisite| owned.contains(prerequisite.as_str()))
            {
                CatalogNodeState::Available
            } else {
                CatalogNodeState::Locked
            };
        }
    }

    fn filtered_indices(&self, query: &str, filter: ResearchFilter) -> Vec<usize> {
        let query = query.trim().to_lowercase();
        research_catalog()
            .nodes()
            .iter()
            .enumerate()
            .filter(|(_, node)| filter.includes(node.category))
            .filter(|(_, node)| {
                query.is_empty()
                    || node.id.to_lowercase().contains(&query)
                    || node.name.to_lowercase().contains(&query)
                    || node.description.to_lowercase().contains(&query)
            })
            .map(|(index, _)| index)
            .collect()
    }

    fn purchase_state(&self, id: &str, snapshot: &ResearchSnapshot) -> PurchaseState {
        let Some(index) = self.by_id.get(id).copied() else {
            return PurchaseState::Locked;
        };
        let node = &research_catalog().nodes()[index];
        if snapshot.owned_node_ids.iter().any(|owned| owned == id) {
            return PurchaseState::Owned;
        }
        if !UPGRADE_NODES.iter().any(|legacy| legacy.id == node.id) {
            return PurchaseState::IntegrationPending;
        }
        match self.state_of(id, snapshot) {
            CatalogNodeState::Owned => PurchaseState::Owned,
            CatalogNodeState::Locked => PurchaseState::Locked,
            CatalogNodeState::Available => {
                if can_afford(snapshot.blessings, node.cost) {
                    PurchaseState::LegacyReady
                } else {
                    PurchaseState::LegacyUnaffordable
                }
            }
        }
    }

    fn dispatchable_legacy_node(&self, id: &str, snapshot: &ResearchSnapshot) -> bool {
        self.purchase_state(id, snapshot) == PurchaseState::LegacyReady
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct ResearchResponsiveLayout {
    root_width: f32,
    root_height: f32,
    header_height: f32,
    canvas_width: f32,
    canvas_height: f32,
    inspector_width: f32,
}

impl ResearchResponsiveLayout {
    fn for_window(width: f32, height: f32) -> Self {
        let inspector_width = if width <= 1100.0 {
            248.0
        } else if width <= 1500.0 {
            280.0
        } else {
            312.0
        };
        Self {
            root_width: width,
            root_height: height,
            header_height: HEADER_HEIGHT,
            canvas_width: width - inspector_width,
            canvas_height: height - HEADER_HEIGHT,
            inspector_width,
        }
    }
}

/// The persistent interaction state for the full-page ledger. Parent HUD
/// systems only need `visible`; the rest remains private to this module.
#[derive(Resource)]
pub(super) struct UpgradeTreeUi {
    pub(super) visible: bool,
    filter: ResearchFilter,
    query: String,
    search_active: bool,
    selected: usize,
    pan: Vec2,
    zoom: f32,
    state_dirty: bool,
    filter_dirty: bool,
    transform_dirty: bool,
    inspector_dirty: bool,
}

impl Default for UpgradeTreeUi {
    fn default() -> Self {
        Self {
            visible: false,
            filter: ResearchFilter::All,
            query: String::new(),
            search_active: false,
            selected: 0,
            pan: Vec2::new(18.0, 16.0),
            zoom: 0.82,
            state_dirty: true,
            filter_dirty: true,
            transform_dirty: true,
            inspector_dirty: true,
        }
    }
}

#[derive(Component)]
pub(super) struct ResearchRoot;
#[derive(Component)]
pub(super) struct ResearchViewport;
#[derive(Component)]
pub(super) struct ResearchCanvas;
#[derive(Component, Clone, Copy)]
pub(super) struct ResearchCard(usize);
#[derive(Component, Clone, Copy)]
pub(super) struct ResearchConnector(usize);
#[derive(Component, Clone, Copy)]
pub(super) struct CardStateText(usize);
#[derive(Component)]
pub(super) struct ResearchInspector;
#[derive(Component)]
pub(super) struct InspectorTitle;
#[derive(Component)]
pub(super) struct InspectorMeta;
#[derive(Component)]
pub(super) struct InspectorDescription;
#[derive(Component)]
pub(super) struct InspectorPrerequisites;
#[derive(Component)]
pub(super) struct InspectorPayloads;
#[derive(Component)]
pub(super) struct PurchaseButton;
#[derive(Component)]
pub(super) struct PurchaseButtonText;
#[derive(Component)]
pub(super) struct ResearchCurrency;
#[derive(Component)]
pub(super) struct ResearchNext;
#[derive(Component)]
pub(super) struct SearchButton;
#[derive(Component)]
pub(super) struct SearchText;
#[derive(Component)]
pub(super) struct MatchCountText;
#[derive(Component, Clone, Copy)]
pub(super) struct FilterButton(ResearchFilter);
#[derive(Component, Clone, Copy)]
pub(super) enum LedgerAction {
    Close,
    ZoomIn,
    ZoomOut,
    Home,
}

fn category_color(category: ResearchCategory) -> Color {
    match category {
        ResearchCategory::Building => BUILDING_INK,
        ResearchCategory::RecipeResource => RECIPE_INK,
        ResearchCategory::Upgrade => UPGRADE_INK,
    }
}

fn category_label(category: ResearchCategory) -> &'static str {
    match category {
        ResearchCategory::Building => "BUILDING",
        ResearchCategory::RecipeResource => "RECIPE / RESOURCE",
        ResearchCategory::Upgrade => "UPGRADE",
    }
}

fn card_position(index: usize) -> Vec2 {
    let layout = research_catalog().nodes()[index].layout;
    Vec2::new(
        MAP_PADDING_X + layout.x as f32 * MAP_STEP_X,
        MAP_PADDING_Y + layout.y as f32 * MAP_STEP_Y,
    )
}

/// Bevy UI transforms scale around the node centre. Compensate for that fixed
/// origin so `pan` remains the rendered top-left of this very wide canvas.
fn canvas_translation(pan: Vec2, zoom: f32) -> Vec2 {
    pan - Vec2::new(MAP_WIDTH, MAP_HEIGHT) * ((1.0 - zoom) / 2.0)
}

/// Keep the overview legible: nearby dependencies are always inked, while a
/// cross-track dependency is revealed when its target is selected. The entity
/// still exists at all times, so selection/filter changes only toggle display.
fn connector_is_relevant(connector: CatalogConnector, selected: usize) -> bool {
    let length = card_position(connector.to).distance(card_position(connector.from));
    length <= MAP_STEP_X * 5.0 || connector.to == selected
}

fn ledger_button(label: impl Into<String>) -> impl Bundle {
    (
        Button,
        Node {
            height: Val::Px(28.0),
            padding: UiRect::horizontal(Val::Px(9.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(UI_BUTTON_BROWN),
        BorderColor::all(Color::NONE),
        ImageNode::default(),
        KitButton,
        children![ui_text(label, FS_SMALL, UI_INK)],
    )
}

/// Spawn all 500 catalog cards and every dependency connector exactly once.
pub(super) fn spawn_research_ui(commands: &mut Commands) {
    let model = ResearchUiModel::from_catalog();
    let catalog = research_catalog();

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::None,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::NONE),
            ImageNode::default(),
            AdventurePanel::Dark,
            GlobalZIndex(300),
            ResearchRoot,
            WorldInputBlocker,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(HEADER_HEIGHT),
                    min_height: Val::Px(HEADER_HEIGHT),
                    padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    border: UiRect::bottom(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(UI_HEADER),
                BorderColor::all(Color::NONE),
                ImageNode::default(),
                AdventurePanel::Dark,
            ))
            .with_children(|header| {
                header
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(14.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn(ui_text("Research Ledger", 20.0, UI_TITLE_INK));
                        row.spawn((ui_text("", FS_BODY, UI_TITLE_INK), ResearchCurrency));
                        row.spawn((
                            Node {
                                flex_grow: 1.0,
                                ..default()
                            },
                            ui_text("", FS_SMALL, UI_TITLE_INK),
                            ResearchNext,
                        ));
                        row.spawn((ledger_button("-"), LedgerAction::ZoomOut));
                        row.spawn((ledger_button("+"), LedgerAction::ZoomIn));
                        row.spawn((ledger_button("Home"), LedgerAction::Home));
                        row.spawn((ledger_button("Close [U]"), LedgerAction::Close));
                    });
                header
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(7.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Button,
                            Node {
                                height: Val::Px(28.0),
                                min_width: Val::Px(202.0),
                                padding: UiRect::horizontal(Val::Px(9.0)),
                                justify_content: JustifyContent::FlexStart,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(UI_BUTTON_BROWN),
                            BorderColor::all(Color::NONE),
                            ImageNode::default(),
                            KitButton,
                            SearchButton,
                        ))
                        .with_children(|button| {
                            button.spawn((
                                ui_text("Search nodes  [/]", FS_SMALL, UI_INK),
                                SearchText,
                            ));
                        });
                        for filter in ResearchFilter::ALL {
                            row.spawn((
                                ledger_button(filter.label()),
                                FilterButton(filter),
                                KitToggle {
                                    active: filter == ResearchFilter::All,
                                },
                            ));
                        }
                        row.spawn((
                            Node {
                                margin: UiRect::left(Val::Px(8.0)),
                                ..default()
                            },
                            ui_text("500 nodes", FS_SMALL, UI_TITLE_INK),
                            MatchCountText,
                        ));
                        row.spawn((
                            Node {
                                flex_grow: 1.0,
                                ..default()
                            },
                            ui_text(
                                "Pan: arrows / WASD / wheel    Zoom: + / − / Ctrl+wheel",
                                FS_SMALL,
                                UI_TITLE_INK,
                            ),
                        ));
                    });
            });

            root.spawn(Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                ..default()
            })
            .with_children(|body| {
                body.spawn((
                    Node {
                        position_type: PositionType::Relative,
                        flex_grow: 1.0,
                        min_width: Val::Px(0.0),
                        height: Val::Percent(100.0),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(LEDGER_PAPER),
                    ResearchViewport,
                ))
                .with_children(|viewport| {
                    viewport
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Px(MAP_WIDTH),
                                height: Val::Px(MAP_HEIGHT),
                                ..default()
                            },
                            UiTransform::IDENTITY,
                            ResearchCanvas,
                        ))
                        .with_children(|canvas| {
                            for (x, label, subtitle, color) in [
                                (0, "FOUNDING THREAD", "The working legacy tree", READY_INK),
                                (20, "BUILDING STUDIES", "Shelter, industry and civic works", BUILDING_INK),
                                (50, "RECIPES & RESOURCES", "Materials, crafts and stores", RECIPE_INK),
                                (80, "COLONY UPGRADES", "Knowledge applied to every paw", UPGRADE_INK),
                            ] {
                                canvas
                                    .spawn(Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(MAP_PADDING_X + x as f32 * MAP_STEP_X),
                                        top: Val::Px(28.0),
                                        width: Val::Px(440.0),
                                        flex_direction: FlexDirection::Column,
                                        ..default()
                                    })
                                    .with_children(|lane| {
                                        lane.spawn(ui_text(label, 15.0, color));
                                        lane.spawn(ui_text(subtitle, FS_SMALL, LEDGER_MUTED));
                                    });
                            }

                            for (connector_index, connector) in
                                model.connectors.iter().copied().enumerate()
                            {
                                let from = card_position(connector.from)
                                    + Vec2::new(NODE_WIDTH, NODE_HEIGHT / 2.0);
                                let to = card_position(connector.to)
                                    + Vec2::new(0.0, NODE_HEIGHT / 2.0);
                                let delta = to - from;
                                let midpoint = (from + to) / 2.0;
                                let length = delta.length();
                                canvas.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(midpoint.x - length / 2.0),
                                        top: Val::Px(midpoint.y - 1.0),
                                        width: Val::Px(length),
                                        height: Val::Px(2.0),
                                        ..default()
                                    },
                                    BackgroundColor(LEDGER_PAPER_DARK),
                                    UiTransform::from_rotation(Rot2::radians(delta.y.atan2(delta.x))),
                                    ResearchConnector(connector_index),
                                ));
                            }

                            for entry in &model.entries {
                                let index = entry.index;
                                let node = &catalog.nodes()[index];
                                let position = card_position(index);
                                canvas
                                    .spawn((
                                        Button,
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: Val::Px(position.x),
                                            top: Val::Px(position.y),
                                            width: Val::Px(NODE_WIDTH),
                                            height: Val::Px(NODE_HEIGHT),
                                            padding: UiRect::all(Val::Px(7.0)),
                                            border: UiRect::all(Val::Px(2.0)),
                                            flex_direction: FlexDirection::Column,
                                            justify_content: JustifyContent::SpaceBetween,
                                            overflow: Overflow::clip(),
                                            ..default()
                                        },
                                        BackgroundColor(LEDGER_PAPER),
                                        BorderColor::all(category_color(node.category)),
                                        ResearchCard(index),
                                    ))
                                    .with_children(|card| {
                                        card.spawn(ui_text(node.name.clone(), FS_BODY, LEDGER_INK));
                                        card.spawn((
                                            ui_text(
                                                format!("LOCKED · E{} · {:.0}b", node.era, node.cost),
                                                9.5,
                                                LOCKED_INK,
                                            ),
                                            CardStateText(index),
                                        ));
                                    });
                            }
                        });
                });

                body.spawn((
                    Node {
                        width: Val::Px(280.0),
                        min_width: Val::Px(248.0),
                        height: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(16.0)),
                        border: UiRect::left(Val::Px(2.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        ..default()
                    },
                    BackgroundColor(UI_BG),
                    BorderColor::all(Color::NONE),
                    ImageNode::default(),
                    AdventurePanel::Ornate,
                    ResearchInspector,
                ))
                .with_children(|inspector| {
                    inspector.spawn(ui_text("SELECTED STUDY", FS_SMALL, LEDGER_MUTED));
                    inspector.spawn((ui_text("Research Hut", 19.0, LEDGER_INK), InspectorTitle));
                    inspector.spawn((ui_text("", FS_SMALL, BUILDING_INK), InspectorMeta));
                    inspector.spawn((ui_text("", FS_BODY, LEDGER_INK), InspectorDescription));
                    inspector.spawn(Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(1.0),
                        margin: UiRect::vertical(Val::Px(4.0)),
                        ..default()
                    })
                    .insert(BackgroundColor(LEDGER_PAPER_DARK));
                    inspector.spawn(ui_text("REQUIRES", FS_SMALL, LEDGER_MUTED));
                    inspector.spawn((ui_text("", FS_BODY, LEDGER_INK), InspectorPrerequisites));
                    inspector.spawn(ui_text("UNLOCKS", FS_SMALL, LEDGER_MUTED));
                    inspector.spawn((ui_text("", FS_BODY, LEDGER_INK), InspectorPayloads));
                    inspector
                        .spawn((
                            Button,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(32.0),
                                padding: UiRect::horizontal(Val::Px(9.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(UI_BUTTON_GREY),
                            BorderColor::all(Color::NONE),
                            ImageNode::default(),
                            KitButton,
                            KitDisabled { disabled: true },
                            PurchaseButton,
                        ))
                        .with_children(|button| {
                            button.spawn((ui_text("", FS_SMALL, UI_INK), PurchaseButtonText));
                        });
                    inspector.spawn(ui_text(
                        "The original 23 studies can be commissioned with blessings. Expanded catalog studies remain visible for planning while their runtime effects are integrated.",
                        FS_SMALL,
                        LEDGER_MUTED,
                    ));
                });
            });
        });

    commands.insert_resource(model);
}

fn current_research(latest: &LatestSnapshot) -> Option<&ResearchSnapshot> {
    latest
        .0
        .as_ref()
        .and_then(|world| world.colonies.first())
        .map(|colony| &colony.research)
}

fn center_on(ui: &mut UpgradeTreeUi, index: usize, window: Option<&Window>) {
    let point = card_position(index) + Vec2::new(NODE_WIDTH / 2.0, NODE_HEIGHT / 2.0);
    let (canvas_width, canvas_height) = window.map_or((760.0, 650.0), |window| {
        let layout = ResearchResponsiveLayout::for_window(window.width(), window.height());
        (layout.canvas_width, layout.canvas_height)
    });
    ui.pan = Vec2::new(
        canvas_width / 2.0 - point.x * ui.zoom,
        canvas_height / 2.0 - point.y * ui.zoom,
    );
    ui.transform_dirty = true;
}

/// Open/close the full-page ledger from the existing Tree command or `U`.
#[allow(clippy::too_many_arguments)]
pub(super) fn toggle_upgrade_tree(
    keys: Res<ButtonInput<KeyCode>>,
    button: Query<&Interaction, (Changed<Interaction>, With<TreeButton>)>,
    close: Query<(&Interaction, &LedgerAction), Changed<Interaction>>,
    mut ui: ResMut<UpgradeTreeUi>,
    mut goods: ResMut<GoodsUi>,
    mut announce: ResMut<AnnouncementsUi>,
    mut census: ResMut<CensusUi>,
) {
    let close_clicked = close.iter().any(|(interaction, action)| {
        *interaction == Interaction::Pressed && matches!(action, LedgerAction::Close)
    });
    let toggle_clicked = button
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);
    if keys.just_pressed(KeyCode::KeyU) || toggle_clicked || close_clicked {
        ui.visible = !ui.visible;
        ui.search_active = false;
        if ui.visible {
            goods.visible = false;
            announce.visible = false;
            census.visible = false;
            ui.state_dirty = true;
            ui.filter_dirty = true;
            ui.transform_dirty = true;
            ui.inspector_dirty = true;
        }
    }
}

/// Apply full-window sizing and keep the inspector useful at all required
/// desktop widths. This does no catalog work.
pub(super) fn update_research_shell(
    ui: Res<UpgradeTreeUi>,
    windows: Query<&Window>,
    mut root: Query<&mut Node, (With<ResearchRoot>, Without<ResearchInspector>)>,
    mut inspector: Query<&mut Node, (With<ResearchInspector>, Without<ResearchRoot>)>,
) {
    if let Ok(mut node) = root.single_mut() {
        node.display = if ui.visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let layout = ResearchResponsiveLayout::for_window(window.width(), window.height());
    if let Ok(mut node) = inspector.single_mut() {
        node.width = Val::Px(layout.inspector_width);
    }
}

/// Button interactions select nodes, switch catalog categories, focus search
/// and operate the fixed zoom controls.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn handle_research_controls(
    mut ui: ResMut<UpgradeTreeUi>,
    search: Query<&Interaction, (Changed<Interaction>, With<SearchButton>)>,
    filters: Query<(&Interaction, &FilterButton), Changed<Interaction>>,
    cards: Query<(&Interaction, &ResearchCard), Changed<Interaction>>,
    actions: Query<(&Interaction, &LedgerAction), Changed<Interaction>>,
) {
    if !ui.visible {
        return;
    }
    if search
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        ui.search_active = true;
        ui.filter_dirty = true;
    }
    for (interaction, filter) in &filters {
        if *interaction == Interaction::Pressed && ui.filter != filter.0 {
            ui.filter = filter.0;
            ui.filter_dirty = true;
        }
    }
    for (interaction, card) in &cards {
        if *interaction == Interaction::Pressed {
            ui.selected = card.0;
            ui.inspector_dirty = true;
            ui.filter_dirty = true;
        }
    }
    for (interaction, action) in &actions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            LedgerAction::Close => {}
            LedgerAction::ZoomIn => {
                ui.zoom = (ui.zoom * 1.15).clamp(MIN_SCALE, MAX_SCALE);
                ui.transform_dirty = true;
            }
            LedgerAction::ZoomOut => {
                ui.zoom = (ui.zoom / 1.15).clamp(MIN_SCALE, MAX_SCALE);
                ui.transform_dirty = true;
            }
            LedgerAction::Home => {
                ui.selected = 0;
                ui.zoom = 0.82;
                ui.pan = Vec2::new(18.0, 16.0);
                ui.transform_dirty = true;
                ui.inspector_dirty = true;
                ui.filter_dirty = true;
            }
        }
    }
}

/// Text entry is deliberately small and local: `/` focuses the search field,
/// Backspace edits it, Escape clears focus and Enter jumps to the first result.
pub(super) fn research_keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut keyboard: MessageReader<KeyboardInput>,
    windows: Query<&Window>,
    model: Res<ResearchUiModel>,
    mut ui: ResMut<UpgradeTreeUi>,
) {
    if !ui.visible {
        return;
    }
    if keys.just_pressed(KeyCode::Slash) {
        ui.search_active = true;
        ui.filter_dirty = true;
    }
    if ui.search_active {
        let old_query = ui.query.clone();
        for event in keyboard.read() {
            if event.state != ButtonState::Pressed {
                continue;
            }
            match event.key_code {
                KeyCode::Backspace => {
                    ui.query.pop();
                }
                KeyCode::Escape => {
                    ui.search_active = false;
                    ui.filter_dirty = true;
                }
                KeyCode::Enter => {
                    if let Some(first) = model
                        .filtered_indices(&ui.query, ui.filter)
                        .first()
                        .copied()
                    {
                        ui.selected = first;
                        center_on(&mut ui, first, windows.single().ok());
                        ui.inspector_dirty = true;
                        ui.filter_dirty = true;
                    }
                    ui.search_active = false;
                    ui.filter_dirty = true;
                }
                _ => {
                    if let Some(text) = event.text.as_deref() {
                        for character in text.chars().filter(|c| !c.is_control() && *c != '/') {
                            if ui.query.chars().count() < 48 {
                                ui.query.push(character);
                            }
                        }
                    }
                }
            }
        }
        if ui.query != old_query {
            ui.filter_dirty = true;
        }
    } else if keys.just_pressed(KeyCode::Escape) && !ui.query.is_empty() {
        ui.query.clear();
        ui.filter_dirty = true;
    }
}

/// Keyboard and wheel navigation alter only one `UiTransform`, even though the
/// transformed canvas contains the complete catalog.
pub(super) fn navigate_research_canvas(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: MessageReader<MouseWheel>,
    mut ui: ResMut<UpgradeTreeUi>,
) {
    if !ui.visible {
        return;
    }
    let mut pan_delta = Vec2::ZERO;
    if !ui.search_active {
        let speed = 540.0 * time.delta_secs();
        if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
            pan_delta.x += speed;
        }
        if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
            pan_delta.x -= speed;
        }
        if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) {
            pan_delta.y += speed;
        }
        if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS) {
            pan_delta.y -= speed;
        }
    }

    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    for event in wheel.read() {
        let scale = match event.unit {
            MouseScrollUnit::Line => 42.0,
            MouseScrollUnit::Pixel => 1.0,
        };
        if ctrl {
            let steps = match event.unit {
                MouseScrollUnit::Line => event.y,
                MouseScrollUnit::Pixel => event.y / 100.0,
            };
            ui.zoom = (ui.zoom * (1.0 + steps * 0.08)).clamp(MIN_SCALE, MAX_SCALE);
        } else if shift {
            pan_delta.x += (event.x + event.y) * scale;
        } else {
            pan_delta += Vec2::new(event.x * scale, event.y * scale);
        }
        ui.transform_dirty = true;
    }

    if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) {
        ui.zoom = (ui.zoom * 1.15).clamp(MIN_SCALE, MAX_SCALE);
        ui.transform_dirty = true;
    }
    if keys.just_pressed(KeyCode::Minus) {
        ui.zoom = (ui.zoom / 1.15).clamp(MIN_SCALE, MAX_SCALE);
        ui.transform_dirty = true;
    }
    if pan_delta != Vec2::ZERO {
        ui.pan += pan_delta;
        ui.transform_dirty = true;
    }
}

pub(super) fn update_research_transform(
    mut ui: ResMut<UpgradeTreeUi>,
    mut canvas: Query<&mut UiTransform, With<ResearchCanvas>>,
) {
    if !ui.visible || !ui.transform_dirty {
        return;
    }
    if let Ok(mut transform) = canvas.single_mut() {
        let translation = canvas_translation(ui.pan, ui.zoom);
        transform.translation = Val2::px(translation.x, translation.y);
        transform.scale = Vec2::splat(ui.zoom);
    }
    ui.transform_dirty = false;
}

/// Apply search/filter visibility without reallocating UI entities. Dependency
/// lines remain only when both endpoint cards are in the current result set.
#[allow(clippy::type_complexity)]
pub(super) fn update_research_filter(
    model: Res<ResearchUiModel>,
    mut ui: ResMut<UpgradeTreeUi>,
    mut cards: Query<(&ResearchCard, &mut Node, &mut BorderColor)>,
    mut connectors: Query<(&ResearchConnector, &mut Node), Without<ResearchCard>>,
    mut filters: Query<(&FilterButton, &mut KitToggle)>,
    mut search: Query<&mut Text, (With<SearchText>, Without<MatchCountText>)>,
    mut count: Query<&mut Text, (With<MatchCountText>, Without<SearchText>)>,
) {
    if !ui.visible || !ui.filter_dirty {
        return;
    }
    let matches = model.filtered_indices(&ui.query, ui.filter);
    let mut visible = vec![false; model.entries.len()];
    for index in &matches {
        visible[*index] = true;
    }
    for (card, mut node, mut border) in &mut cards {
        node.display = if visible[card.0] {
            Display::Flex
        } else {
            Display::None
        };
        let color = if card.0 == ui.selected {
            READY_INK
        } else {
            category_color(research_catalog().nodes()[card.0].category)
        };
        *border = BorderColor::all(color);
    }
    for (line, mut node) in &mut connectors {
        let connector = model.connectors[line.0];
        node.display = if visible[connector.from]
            && visible[connector.to]
            && connector_is_relevant(connector, ui.selected)
        {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (button, mut toggle) in &mut filters {
        toggle.active = button.0 == ui.filter;
    }
    if let Ok(mut text) = search.single_mut() {
        text.0 = if ui.query.is_empty() {
            if ui.search_active {
                "Search: |".to_owned()
            } else {
                "Search nodes  [/]".to_owned()
            }
        } else if ui.search_active {
            format!("Search: {}|", ui.query)
        } else {
            format!("Search: {}", ui.query)
        };
    }
    if let Ok(mut text) = count.single_mut() {
        text.0 = format!("{} / 500 nodes", matches.len());
    }
    ui.filter_dirty = false;
}

/// Repaint fixed cards only when a new snapshot arrives or the page opens.
/// There is no Commands access here, which is the guard against entity churn.
#[allow(clippy::type_complexity)]
pub(super) fn update_research_snapshot(
    latest: Res<LatestSnapshot>,
    mut model: ResMut<ResearchUiModel>,
    mut ui: ResMut<UpgradeTreeUi>,
    mut cards: Query<(&ResearchCard, &mut BackgroundColor)>,
    mut states: Query<
        (&CardStateText, &mut Text, &mut TextColor),
        (Without<ResearchCurrency>, Without<ResearchNext>),
    >,
    mut currency: Query<&mut Text, (With<ResearchCurrency>, Without<ResearchNext>)>,
    mut next: Query<&mut Text, (With<ResearchNext>, Without<ResearchCurrency>)>,
) {
    if !ui.visible || (!latest.is_changed() && !ui.state_dirty) {
        return;
    }
    if let Some(research) = current_research(&latest) {
        model.apply_snapshot(research);
        for (card, mut background) in &mut cards {
            background.0 = match model.states[card.0] {
                CatalogNodeState::Owned => Color::srgb(0.65, 0.72, 0.52),
                CatalogNodeState::Available => Color::srgb(0.82, 0.72, 0.49),
                CatalogNodeState::Locked => LEDGER_PAPER,
            };
        }
        for (marker, mut text, mut color) in &mut states {
            let node = &research_catalog().nodes()[marker.0];
            let (label, ink) = match model.states[marker.0] {
                CatalogNodeState::Owned => ("OWNED", OWNED_INK),
                CatalogNodeState::Available => ("AVAILABLE", READY_INK),
                CatalogNodeState::Locked => ("LOCKED", LOCKED_INK),
            };
            text.0 = format!("{label} · E{} · {:.0}b", node.era, node.cost);
            color.0 = ink;
        }
        if let Ok(mut text) = currency.single_mut() {
            text.0 = format!(
                "{:.0} blessings  ·  {:.0} research  ·  {} scholars",
                research.blessings, research.research_points, research.researcher_count
            );
        }
        if let Ok(mut text) = next.single_mut() {
            text.0 = research.next_target.as_ref().map_or_else(
                || "No automatic study queued".to_owned(),
                |target| format!("Studying next: {} · {:.0} pts", target.name, target.cost),
            );
        }
    } else {
        if let Ok(mut text) = currency.single_mut() {
            text.0 = "Awaiting colony research record…".to_owned();
        }
        if let Ok(mut text) = next.single_mut() {
            text.0 = "Connect to inspect progress".to_owned();
        }
    }
    ui.state_dirty = false;
    ui.inspector_dirty = true;
}

fn payload_line(payload: &ResearchPayload) -> String {
    match payload {
        ResearchPayload::UnlockBuilding { building_id } => format!("Build {building_id}"),
        ResearchPayload::UnlockRecipe { recipe_id } => format!("Recipe: {recipe_id}"),
        ResearchPayload::UnlockResource { resource_id } => format!("Resource: {resource_id}"),
        ResearchPayload::UnlockJob { job_id } => format!("Job: {job_id}"),
        ResearchPayload::ModifyBuilding {
            building_id,
            attribute,
            operation,
            value,
        } => format!("{building_id} {attribute:?} {operation:?} {value:.2}"),
        ResearchPayload::Modify {
            effect_id,
            operation,
            value,
        } => format!("{effect_id} {operation:?} {value:.2}"),
        ResearchPayload::UnlockCapability { capability_id } => {
            format!("Capability: {capability_id}")
        }
    }
}

/// Repaint only the one-node inspector. Generated catalog nodes are explicitly
/// non-actionable until the sim consumes their payloads.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn update_research_inspector(
    latest: Res<LatestSnapshot>,
    model: Res<ResearchUiModel>,
    session: Res<Session>,
    mut ui: ResMut<UpgradeTreeUi>,
    mut purchase_button: Query<&mut KitDisabled, With<PurchaseButton>>,
    mut texts: Query<
        (
            &mut Text,
            Option<&InspectorTitle>,
            Option<&InspectorMeta>,
            Option<&InspectorDescription>,
            Option<&InspectorPrerequisites>,
            Option<&InspectorPayloads>,
            Option<&PurchaseButtonText>,
        ),
        Or<(
            With<InspectorTitle>,
            With<InspectorMeta>,
            With<InspectorDescription>,
            With<InspectorPrerequisites>,
            With<InspectorPayloads>,
            With<PurchaseButtonText>,
        )>,
    >,
) {
    if !ui.visible || (!ui.inspector_dirty && !session.is_changed()) {
        return;
    }
    let node = &research_catalog().nodes()[ui.selected];
    let meta = format!(
        "{}  ·  ERA {}  ·  {:.0} BLESSINGS",
        category_label(node.category),
        node.era,
        node.cost
    );
    let prerequisites = if node.prerequisites.is_empty() {
        "No prior study".to_owned()
    } else {
        node.prerequisites
            .iter()
            .map(|id| {
                research_catalog()
                    .get(id)
                    .map_or(id.as_str(), |required| required.name.as_str())
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let payloads = node
        .payloads
        .iter()
        .map(payload_line)
        .collect::<Vec<_>>()
        .join("\n");
    let purchase_state =
        current_research(&latest).map(|research| model.purchase_state(&node.id, research));
    let purchase = purchase_state.map_or_else(
        || "Awaiting colony".to_owned(),
        |state| match state {
            PurchaseState::Owned => "Study owned".to_owned(),
            PurchaseState::Locked => "Prerequisites required".to_owned(),
            PurchaseState::LegacyReady => format!("Commission for {:.0} blessings", node.cost),
            PurchaseState::LegacyUnaffordable => format!("Need {:.0} blessings", node.cost),
            PurchaseState::IntegrationPending => "Runtime integration pending".to_owned(),
        },
    );
    if let Ok(mut disabled) = purchase_button.single_mut() {
        disabled.disabled = research_purchase_disabled(session.ready, purchase_state);
    }
    for (mut text, title, meta_marker, description, prereq_marker, payload_marker, buy) in
        &mut texts
    {
        if title.is_some() {
            text.0 = node.name.clone();
        } else if meta_marker.is_some() {
            text.0 = meta.clone();
        } else if description.is_some() {
            text.0 = node.description.clone();
        } else if prereq_marker.is_some() {
            text.0 = prerequisites.clone();
        } else if payload_marker.is_some() {
            text.0 = payloads.clone();
        } else if buy.is_some() {
            text.0 = purchase.clone();
        }
    }
    ui.inspector_dirty = false;
}

fn research_purchase_disabled(session_ready: bool, purchase_state: Option<PurchaseState>) -> bool {
    !session_ready || purchase_state != Some(PurchaseState::LegacyReady)
}

/// Dispatch is guarded by catalog support, prerequisite state and affordability
/// at the moment of the click—not merely by what the button happened to show.
pub(super) fn handle_research_purchase(
    latest: Res<LatestSnapshot>,
    model: Res<ResearchUiModel>,
    ui: Res<UpgradeTreeUi>,
    session: Res<Session>,
    mut outgoing: ResMut<OutgoingActions>,
    button: Query<&Interaction, (Changed<Interaction>, With<PurchaseButton>)>,
) {
    if !ui.visible
        || !session.ready
        || !button
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed)
    {
        return;
    }
    let Some(research) = current_research(&latest) else {
        return;
    };
    let node = &research_catalog().nodes()[ui.selected];
    if model.dispatchable_legacy_node(&node.id, research) {
        outgoing.0.push(ClientAction::UnlockNode {
            session_id: session.session_id.clone(),
            nickname: "Desktop Cat".to_owned(),
            sig: session.sig.clone(),
            node_id: node.id.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::world::CommandQueue;
    use cat_protocol::ResearchSnapshot;
    use cat_sim::research_catalog::{RESEARCH_NODE_COUNT, ResearchCategory, research_catalog};

    fn snapshot(owned: &[&str], blessings: f64) -> ResearchSnapshot {
        ResearchSnapshot {
            owned_node_ids: owned.iter().map(|id| (*id).to_owned()).collect(),
            research_points: 12.0,
            researcher_count: 2,
            blessings,
            next_target: None,
        }
    }

    #[test]
    fn model_represents_exactly_500_cards_and_every_dependency_connector() {
        let model = ResearchUiModel::from_catalog();
        assert_eq!(model.entries.len(), RESEARCH_NODE_COUNT);
        assert_eq!(model.entries.len(), 500);
        assert_eq!(
            model.connectors.len(),
            research_catalog()
                .nodes()
                .iter()
                .map(|node| node.prerequisites.len())
                .sum::<usize>()
        );
        assert_eq!(
            model.category_count(ResearchCategory::Building),
            research_catalog().category_count(ResearchCategory::Building)
        );
        assert_eq!(model.category_count(ResearchCategory::Building), 167);
        assert_eq!(model.category_count(ResearchCategory::RecipeResource), 167);
        assert_eq!(model.category_count(ResearchCategory::Upgrade), 166);
    }

    #[test]
    fn catalog_state_derivation_preserves_owned_available_and_locked() {
        let model = ResearchUiModel::from_catalog();
        let none = snapshot(&[], 99.0);
        assert_eq!(
            model.state_of("research_hut", &none),
            CatalogNodeState::Available
        );
        assert_eq!(
            model.state_of("basic_tools", &none),
            CatalogNodeState::Locked
        );
        let rooted = snapshot(&["research_hut"], 99.0);
        assert_eq!(
            model.state_of("research_hut", &rooted),
            CatalogNodeState::Owned
        );
        assert_eq!(
            model.state_of("basic_tools", &rooted),
            CatalogNodeState::Available
        );
    }

    #[test]
    fn search_and_category_filter_are_case_insensitive_and_composable() {
        let model = ResearchUiModel::from_catalog();
        assert_eq!(
            model.filtered_indices("", ResearchFilter::All).len(),
            RESEARCH_NODE_COUNT
        );
        let all = model.filtered_indices("SMITH", ResearchFilter::All);
        assert!(!all.is_empty());
        assert!(all.iter().all(|index| {
            let node = &research_catalog().nodes()[*index];
            node.name.to_lowercase().contains("smith")
                || node.id.to_lowercase().contains("smith")
                || node.description.to_lowercase().contains("smith")
        }));
        let buildings = model.filtered_indices("smith", ResearchFilter::Building);
        assert!(!buildings.is_empty());
        assert!(buildings.iter().all(|index| {
            research_catalog().nodes()[*index].category == ResearchCategory::Building
        }));
        assert!(
            model
                .filtered_indices("definitely-not-a-node", ResearchFilter::All)
                .is_empty()
        );
    }

    #[test]
    fn unsupported_catalog_nodes_never_offer_runtime_purchase_but_legacy_does() {
        let model = ResearchUiModel::from_catalog();
        let research = snapshot(&["research_hut"], 99.0);
        assert_eq!(
            model.purchase_state("basic_tools", &research),
            PurchaseState::LegacyReady
        );
        let generated = research_catalog()
            .nodes()
            .iter()
            .find(|node| node.id == "den_foundations")
            .unwrap();
        assert_eq!(
            model.purchase_state(&generated.id, &research),
            PurchaseState::IntegrationPending
        );
        assert!(model.dispatchable_legacy_node("basic_tools", &research));
        assert!(!model.dispatchable_legacy_node(&generated.id, &research));
    }

    #[test]
    fn purchase_button_stays_disabled_until_the_signed_session_is_ready() {
        assert!(research_purchase_disabled(
            false,
            Some(PurchaseState::LegacyReady)
        ));
        assert!(!research_purchase_disabled(
            true,
            Some(PurchaseState::LegacyReady)
        ));
        assert!(research_purchase_disabled(
            true,
            Some(PurchaseState::LegacyUnaffordable)
        ));
        assert!(research_purchase_disabled(true, None));
    }

    #[test]
    fn responsive_layout_keeps_canvas_and_inspector_visible_at_required_sizes() {
        for (width, height) in [(1024.0, 768.0), (1280.0, 800.0), (1920.0, 1080.0)] {
            let layout = ResearchResponsiveLayout::for_window(width, height);
            assert_eq!(layout.root_width, width);
            assert_eq!(layout.root_height, height);
            assert!(layout.canvas_width >= 700.0);
            assert!((240.0..=320.0).contains(&layout.inspector_width));
            assert!(layout.canvas_height >= 600.0);
            assert!(layout.header_height <= 112.0);
        }
    }

    #[test]
    fn repeated_snapshot_application_never_duplicates_logical_entities() {
        let mut model = ResearchUiModel::from_catalog();
        let original_entries = model.entries.len();
        let original_connectors = model.connectors.len();
        let research = snapshot(&["research_hut"], 8.0);
        model.apply_snapshot(&research);
        model.apply_snapshot(&research);
        assert_eq!(model.entries.len(), original_entries);
        assert_eq!(model.connectors.len(), original_connectors);
        assert_eq!(model.states.len(), RESEARCH_NODE_COUNT);
    }

    #[test]
    fn spawning_the_ledger_creates_one_fixed_card_set() {
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        spawn_research_ui(&mut Commands::new(&mut queue, &world));
        queue.apply(&mut world);

        assert_eq!(
            world.query::<&ResearchCard>().iter(&world).count(),
            RESEARCH_NODE_COUNT
        );
        assert_eq!(
            world.query::<&ResearchConnector>().iter(&world).count(),
            research_catalog()
                .nodes()
                .iter()
                .map(|node| node.prerequisites.len())
                .sum::<usize>()
        );
    }

    #[test]
    fn canvas_scale_compensation_keeps_map_origin_at_pan() {
        let pan = Vec2::new(18.0, 16.0);
        let zoom = 0.82;
        let centre = Vec2::new(MAP_WIDTH, MAP_HEIGHT) / 2.0;
        for point in [
            Vec2::ZERO,
            card_position(0),
            Vec2::new(MAP_WIDTH, MAP_HEIGHT),
        ] {
            let rendered = centre + (point - centre) * zoom + canvas_translation(pan, zoom);
            let expected = pan + point * zoom;
            assert!(rendered.abs_diff_eq(expected, 0.01));
        }
    }

    #[test]
    fn long_cross_track_connectors_appear_only_for_the_selected_target() {
        let model = ResearchUiModel::from_catalog();
        let connector = model
            .connectors
            .iter()
            .copied()
            .find(|edge| {
                card_position(edge.to).distance(card_position(edge.from)) > MAP_STEP_X * 5.0
            })
            .expect("catalog should include a cross-track dependency");
        assert!(!connector_is_relevant(connector, connector.from));
        assert!(connector_is_relevant(connector, connector.to));
    }
}
