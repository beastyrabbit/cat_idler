//! Full-window research ledger UI backed by `cat_sim::research_catalog`.

use super::*;
use bevy::{
    input::{ButtonState, keyboard::KeyboardInput, mouse::MouseScrollUnit},
    math::Rot2,
    ui::Val2,
};
use cat_protocol::ResearchSnapshot;
use cat_sim::{
    research_catalog::{
        RESEARCH_NODE_COUNT, ResearchCategory, ResearchNode, ResearchPayload, research_catalog,
    },
    upgrade_tree::UPGRADE_NODES,
};
use std::collections::{HashMap, HashSet};

const HEADER_HEIGHT: f32 = 104.0;
const NODE_WIDTH: f32 = 184.0;
const NODE_HEIGHT: f32 = 96.0;
const MAP_PADDING_X: f32 = 88.0;
const MAP_PADDING_Y: f32 = 118.0;
const MAP_STEP_X: f32 = 216.0;
const MAP_STEP_Y: f32 = 114.0;
const MIN_SCALE: f32 = 0.42;
const MAX_SCALE: f32 = 1.35;

// A cartographer's worktable: the dependency map lives on a dark forest desk,
// while individual studies remain high-contrast paper records. Prerequisites,
// not catalog categories, own the layout; category is only a branch accent.
const RESEARCH_DESK: Color = Color::srgb(0.12, 0.15, 0.11);
const LEDGER_PAPER_DARK: Color = Color::srgb(0.58, 0.46, 0.30);
const LEDGER_INK: Color = Color::srgb(0.13, 0.095, 0.055);
const LEDGER_MUTED: Color = Color::srgb(0.39, 0.33, 0.25);
const BUILDING_INK: Color = Color::srgb(0.30, 0.55, 0.40);
const RECIPE_INK: Color = Color::srgb(0.72, 0.43, 0.20);
const UPGRADE_INK: Color = Color::srgb(0.40, 0.48, 0.70);
const OWNED_INK: Color = Color::srgb(0.20, 0.42, 0.22);
const READY_INK: Color = Color::srgb(0.70, 0.39, 0.08);
const LOCKED_INK: Color = Color::srgb(0.35, 0.32, 0.27);
const OWNED_PAPER: Color = Color::srgb(0.70, 0.79, 0.60);
const READY_PAPER: Color = Color::srgb(0.92, 0.76, 0.45);
const LOCKED_PAPER: Color = Color::srgb(0.76, 0.73, 0.64);
const RESEARCH_DRAG_GAIN: f32 = 1.65;

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
    ResearchReady,
    ResearchUnaffordable,
    LegacyReady,
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

#[derive(Debug)]
struct UnifiedTreeLayout {
    positions: Vec<Vec2>,
    depths: Vec<usize>,
    size: Vec2,
    layer_count: usize,
}

fn prerequisite_depth(
    index: usize,
    nodes: &[ResearchNode],
    by_id: &HashMap<String, usize>,
    memo: &mut [Option<usize>],
) -> usize {
    if let Some(depth) = memo[index] {
        return depth;
    }
    let depth = nodes[index]
        .prerequisites
        .iter()
        .map(|id| prerequisite_depth(by_id[id], nodes, by_id, memo) + 1)
        .max()
        .unwrap_or(0);
    memo[index] = Some(depth);
    depth
}

/// Turn the catalog DAG into one compact, left-to-right tree. Each layer is
/// one prerequisite step farther from the root. Ordering children around the
/// average position of their parents keeps related branches together without
/// restoring the old category strips.
fn build_unified_tree_layout(
    nodes: &[ResearchNode],
    by_id: &HashMap<String, usize>,
) -> UnifiedTreeLayout {
    let mut memo = vec![None; nodes.len()];
    let depths = (0..nodes.len())
        .map(|index| prerequisite_depth(index, nodes, by_id, &mut memo))
        .collect::<Vec<_>>();
    let layer_count = depths.iter().copied().max().unwrap_or(0) + 1;
    let mut layers = vec![Vec::new(); layer_count];
    for (index, depth) in depths.iter().copied().enumerate() {
        layers[depth].push(index);
    }
    let widest_layer = layers.iter().map(Vec::len).max().unwrap_or(1);
    let mut row_positions = vec![0.0_f32; nodes.len()];
    let mut positions = vec![Vec2::ZERO; nodes.len()];
    for (depth, layer) in layers.iter_mut().enumerate() {
        layer.sort_by(|left, right| {
            let parent_row = |index: usize| {
                let prerequisites = &nodes[index].prerequisites;
                if prerequisites.is_empty() {
                    widest_layer as f32 / 2.0
                } else {
                    prerequisites
                        .iter()
                        .map(|id| row_positions[by_id[id]])
                        .sum::<f32>()
                        / prerequisites.len() as f32
                }
            };
            parent_row(*left)
                .total_cmp(&parent_row(*right))
                .then_with(|| nodes[*left].layout.x.cmp(&nodes[*right].layout.x))
                .then_with(|| nodes[*left].layout.y.cmp(&nodes[*right].layout.y))
                .then_with(|| nodes[*left].id.cmp(&nodes[*right].id))
        });
        let row_offset = (widest_layer - layer.len()) as f32 / 2.0;
        for (row, index) in layer.iter().copied().enumerate() {
            let logical_row = row_offset + row as f32;
            row_positions[index] = logical_row;
            positions[index] = Vec2::new(
                MAP_PADDING_X + depth as f32 * MAP_STEP_X,
                MAP_PADDING_Y + logical_row * MAP_STEP_Y,
            );
        }
    }
    UnifiedTreeLayout {
        positions,
        depths,
        size: Vec2::new(
            MAP_PADDING_X * 2.0 + (layer_count - 1) as f32 * MAP_STEP_X + NODE_WIDTH,
            MAP_PADDING_Y * 2.0 + (widest_layer - 1) as f32 * MAP_STEP_Y + NODE_HEIGHT,
        ),
        layer_count,
    }
}

/// Logical catalog state is allocated once. Applying a snapshot only rewrites
/// this fixed state vector; it can never append cards or dependency lines.
#[derive(Resource)]
pub(super) struct ResearchUiModel {
    entries: Vec<CatalogEntry>,
    connectors: Vec<CatalogConnector>,
    by_id: HashMap<String, usize>,
    states: Vec<CatalogNodeState>,
    layout: UnifiedTreeLayout,
    root_index: usize,
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
        let layout = build_unified_tree_layout(catalog.nodes(), &by_id);
        let root_index = by_id["research_hut"];
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
            layout,
            root_index,
        }
    }

    fn card_position(&self, index: usize) -> Vec2 {
        self.layout.positions[index]
    }

    fn selected_path(&self, selected: usize) -> HashSet<(usize, usize)> {
        let mut path = HashSet::new();
        let mut pending = vec![selected];
        while let Some(to) = pending.pop() {
            for prerequisite in &research_catalog().nodes()[to].prerequisites {
                let from = self.by_id[prerequisite];
                if path.insert((from, to)) {
                    pending.push(from);
                }
            }
        }
        path
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
        let has_legacy_blessing_purchase = UPGRADE_NODES.iter().any(|legacy| legacy.id == node.id);
        if snapshot.owned_node_ids.iter().any(|owned| owned == id) {
            return PurchaseState::Owned;
        }
        match self.state_of(id, snapshot) {
            CatalogNodeState::Owned => PurchaseState::Owned,
            CatalogNodeState::Locked => PurchaseState::Locked,
            CatalogNodeState::Available => {
                if can_afford(snapshot.research_points, node.cost) {
                    PurchaseState::ResearchReady
                } else if has_legacy_blessing_purchase && can_afford(snapshot.blessings, node.cost)
                {
                    PurchaseState::LegacyReady
                } else {
                    PurchaseState::ResearchUnaffordable
                }
            }
        }
    }

    fn dispatchable_legacy_node(&self, id: &str, snapshot: &ResearchSnapshot) -> bool {
        self.purchase_state(id, snapshot) == PurchaseState::LegacyReady
    }

    fn dispatchable_research_node(&self, id: &str, snapshot: &ResearchSnapshot) -> bool {
        self.purchase_state(id, snapshot) == PurchaseState::ResearchReady
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
            260.0
        } else if width <= 1500.0 {
            300.0
        } else {
            320.0
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
            zoom: 1.0,
            state_dirty: true,
            filter_dirty: true,
            transform_dirty: true,
            inspector_dirty: true,
        }
    }
}

impl UpgradeTreeUi {
    pub(super) fn captures_text_input(&self) -> bool {
        self.visible && self.search_active
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
pub(super) struct ResearchHeaderHint;
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
        ResearchCategory::Building => "Building",
        ResearchCategory::RecipeResource => "Recipe / resource",
        ResearchCategory::Upgrade => "Upgrade",
    }
}

fn research_drag_delta(pointer_delta: Vec2) -> Vec2 {
    pointer_delta * RESEARCH_DRAG_GAIN
}

/// Bevy UI transforms scale around the node centre. Compensate for that fixed
/// origin so `pan` remains the rendered top-left of this very wide canvas.
fn canvas_translation(pan: Vec2, zoom: f32, canvas_size: Vec2) -> Vec2 {
    pan - canvas_size * ((1.0 - zoom) / 2.0)
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

/// Spawn every catalog card and dependency connector exactly once.
pub(super) fn spawn_research_ui(commands: &mut Commands) {
    let model = ResearchUiModel::from_catalog();
    let catalog = research_catalog();

    commands
        .spawn((
            Node {
                display: Display::None,
                ..primary_screen_node()
            },
            ui_panel_frame(),
            GlobalZIndex(PRIMARY_SURFACE_Z),
            ResearchRoot,
            UiSurfaceRoot(UiSurfaceKind::PrimaryScreen),
            WorldInputBlocker,
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Auto,
                    min_height: Val::Px(HEADER_HEIGHT),
                    padding: UiRect::axes(Val::Px(14.0), Val::Px(10.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(7.0),
                    justify_content: JustifyContent::SpaceBetween,
                    border: UiRect::bottom(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(UI_HEADER),
                BorderColor::all(Color::NONE),
                ImageNode::default(),
                ZIndex(10),
                AdventurePanel::Dark,
            ))
            .with_children(|header| {
                header
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(14.0),
                        row_gap: Val::Px(7.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn(ui_text("Research tree", 22.0, UI_TITLE_INK));
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
                        row.spawn((ledger_button("Close"), LedgerAction::Close));
                    });
                header
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(7.0),
                        row_gap: Val::Px(7.0),
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
                            button
                                .spawn((ui_text("Search research", FS_SMALL, UI_INK), SearchText));
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
                            ui_text(
                                format!("{RESEARCH_NODE_COUNT} nodes"),
                                FS_SMALL,
                                UI_TITLE_INK,
                            ),
                            MatchCountText,
                        ));
                        row.spawn((
                            Node {
                                flex_grow: 1.0,
                                ..default()
                            },
                            ui_text(
                                "Drag the canvas to pan  |  Scroll to move  |  Ctrl+scroll to zoom",
                                FS_SMALL,
                                UI_TITLE_INK,
                            ),
                            ResearchHeaderHint,
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
                    BackgroundColor(RESEARCH_DESK),
                    ResearchViewport,
                ))
                .with_children(|viewport| {
                    viewport
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Px(model.layout.size.x),
                                height: Val::Px(model.layout.size.y),
                                ..default()
                            },
                            BackgroundColor(RESEARCH_DESK),
                            UiTransform::IDENTITY,
                            ResearchCanvas,
                        ))
                        .with_children(|canvas| {
                            let root_position = model.card_position(model.root_index);
                            canvas
                                .spawn(Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(root_position.x),
                                    top: Val::Px(root_position.y - 66.0),
                                    width: Val::Px(NODE_WIDTH),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(2.0),
                                    ..default()
                                })
                                .with_children(|caption| {
                                    caption.spawn(ui_text(
                                        "ONE TREE · 487 STUDIES",
                                        15.0,
                                        Color::srgb(0.88, 0.72, 0.39),
                                    ));
                                    caption.spawn(ui_text(
                                        format!(
                                            "{} dependency steps · begins here",
                                            model.layout.layer_count
                                        ),
                                        FS_SMALL,
                                        Color::srgb(0.76, 0.73, 0.64),
                                    ));
                                });

                            for (connector_index, connector) in
                                model.connectors.iter().copied().enumerate()
                            {
                                let from = model.card_position(connector.from)
                                    + Vec2::new(NODE_WIDTH, NODE_HEIGHT / 2.0);
                                let to = model.card_position(connector.to)
                                    + Vec2::new(0.0, NODE_HEIGHT / 2.0);
                                let delta = to - from;
                                let midpoint = (from + to) / 2.0;
                                let length = delta.length();
                                let depth_span = model.layout.depths[connector.to]
                                    - model.layout.depths[connector.from];
                                canvas.spawn((
                                    Node {
                                        display: Display::None,
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(midpoint.x - length / 2.0),
                                        top: Val::Px(midpoint.y - 1.5),
                                        width: Val::Px(length),
                                        height: Val::Px(3.0),
                                        ..default()
                                    },
                                    BackgroundColor(
                                        category_color(catalog.nodes()[connector.to].category)
                                            .with_alpha(if depth_span == 1 { 0.34 } else { 0.22 }),
                                    ),
                                    UiTransform::from_rotation(Rot2::radians(
                                        delta.y.atan2(delta.x),
                                    )),
                                    ResearchConnector(connector_index),
                                ));
                            }

                            for entry in &model.entries {
                                let index = entry.index;
                                let node = &catalog.nodes()[index];
                                let position = model.card_position(index);
                                canvas
                                    .spawn((
                                        Button,
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: Val::Px(position.x),
                                            top: Val::Px(position.y),
                                            width: Val::Px(NODE_WIDTH),
                                            height: Val::Px(NODE_HEIGHT),
                                            padding: UiRect::axes(Val::Px(11.0), Val::Px(8.0)),
                                            border: UiRect {
                                                left: Val::Px(4.0),
                                                right: Val::Px(1.0),
                                                top: Val::Px(1.0),
                                                bottom: Val::Px(1.0),
                                            },
                                            flex_direction: FlexDirection::Column,
                                            justify_content: JustifyContent::SpaceBetween,
                                            overflow: Overflow::clip(),
                                            ..default()
                                        },
                                        BackgroundColor(LOCKED_PAPER),
                                        BorderColor::all(category_color(node.category)),
                                        ResearchCard(index),
                                    ))
                                    .with_children(|card| {
                                        card.spawn(ui_text(
                                            if index == model.root_index {
                                                "Root · Building"
                                            } else {
                                                category_label(node.category)
                                            },
                                            10.5,
                                            category_color(node.category),
                                        ));
                                        card.spawn(ui_text_wrapped(
                                            node.name.clone(),
                                            14.0,
                                            LEDGER_INK,
                                        ));
                                        card.spawn((
                                            ui_text(
                                                format!(
                                                    "Locked | Era {} | {:.0} pts",
                                                    node.era, node.cost
                                                ),
                                                11.0,
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
                        width: Val::Px(300.0),
                        min_width: Val::Px(260.0),
                        height: Val::Percent(100.0),
                        min_height: Val::Px(0.0),
                        border: UiRect::left(Val::Px(3.0)),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.94, 0.89, 0.77)),
                    BorderColor::all(LEDGER_PAPER_DARK),
                    ZIndex(10),
                    ResearchInspector,
                ))
                .with_children(|inspector| {
                    spawn_vertical_scroll_area(inspector, 16.0, 10.0, |inspector| {
                        inspector.spawn(ui_text("Study details", FS_SECTION, LEDGER_MUTED));
                        inspector
                            .spawn((ui_text("Research Hut", 22.0, LEDGER_INK), InspectorTitle));
                        inspector.spawn((ui_text("", FS_SMALL, BUILDING_INK), InspectorMeta));
                        inspector.spawn((
                            ui_text_wrapped("", FS_BODY, LEDGER_INK),
                            InspectorDescription,
                        ));
                        inspector
                            .spawn(Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(1.0),
                                margin: UiRect::vertical(Val::Px(4.0)),
                                ..default()
                            })
                            .insert(BackgroundColor(LEDGER_PAPER_DARK));
                        inspector.spawn(ui_text("Requires", FS_SECTION, LEDGER_MUTED));
                        inspector.spawn((ui_text("", FS_BODY, LEDGER_INK), InspectorPrerequisites));
                        inspector.spawn(ui_text("Unlocks", FS_SECTION, LEDGER_MUTED));
                        inspector.spawn((ui_text("", FS_BODY, LEDGER_INK), InspectorPayloads));
                        inspector
                            .spawn((
                                Button,
                                Node {
                                    width: Val::Percent(100.0),
                                    min_height: Val::Px(36.0),
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
                    });
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

fn leader_priority_copy(research: &ResearchSnapshot) -> String {
    research.next_target.as_ref().map_or_else(
        || "No leader-priority study available".to_owned(),
        |target| format!("Leader priority: {} · {:.0} pts", target.name, target.cost),
    )
}

fn center_on(
    ui: &mut UpgradeTreeUi,
    model: &ResearchUiModel,
    index: usize,
    window: Option<&Window>,
) {
    let point = model.card_position(index) + Vec2::new(NODE_WIDTH / 2.0, NODE_HEIGHT / 2.0);
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

/// Open/close the full-page ledger from its explicit buttons.
pub(super) fn toggle_upgrade_tree(
    button: Query<&Interaction, (Changed<Interaction>, With<TreeButton>)>,
    close: Query<(&Interaction, &LedgerAction), Changed<Interaction>>,
    mut ui: ResMut<UpgradeTreeUi>,
    mut router: ResMut<UiRouter>,
    model: Res<ResearchUiModel>,
    windows: Query<&Window>,
) {
    let close_clicked = close.iter().any(|(interaction, action)| {
        *interaction == Interaction::Pressed && matches!(action, LedgerAction::Close)
    });
    let toggle_clicked = button
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);
    if toggle_clicked || close_clicked {
        let opening = !router.is_open(PrimaryScreen::Research);
        router.toggle(PrimaryScreen::Research);
        ui.search_active = false;
        if opening {
            let selected = ui.selected;
            center_on(&mut ui, &model, selected, windows.single().ok());
            ui.state_dirty = true;
            ui.filter_dirty = true;
            ui.transform_dirty = true;
            ui.inspector_dirty = true;
        }
    }
}

/// Apply full-window sizing and keep the inspector useful at all required
/// desktop widths. This does no catalog work.
#[allow(clippy::type_complexity)]
pub(super) fn update_research_shell(
    ui: Res<UpgradeTreeUi>,
    windows: Query<&Window>,
    ui_scale: Res<UiScale>,
    mut root: Query<
        &mut Node,
        (
            With<ResearchRoot>,
            Without<ResearchInspector>,
            Without<ResearchHeaderHint>,
        ),
    >,
    mut inspector: Query<
        &mut Node,
        (
            With<ResearchInspector>,
            Without<ResearchRoot>,
            Without<ResearchHeaderHint>,
        ),
    >,
    mut hint: Query<
        &mut Node,
        (
            With<ResearchHeaderHint>,
            Without<ResearchRoot>,
            Without<ResearchInspector>,
            Without<ResearchNext>,
        ),
    >,
    mut next: Query<
        &mut Node,
        (
            With<ResearchNext>,
            Without<ResearchRoot>,
            Without<ResearchInspector>,
            Without<ResearchHeaderHint>,
        ),
    >,
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
    let profile = UiLayoutProfile::new(window.width(), window.height(), ui_scale.0);
    let layout =
        ResearchResponsiveLayout::for_window(profile.effective_width, profile.effective_height);
    if let Ok(mut node) = inspector.single_mut() {
        node.width = Val::Px(layout.inspector_width);
    }
    if let Ok(mut node) = hint.single_mut() {
        node.display = if profile.compact {
            Display::None
        } else {
            Display::Flex
        };
    }
    if let Ok(mut node) = next.single_mut() {
        node.display = if profile.compact {
            Display::None
        } else {
            Display::Flex
        };
    }
}

/// Button interactions select nodes, switch catalog categories, focus search
/// and operate the fixed zoom controls.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn handle_research_controls(
    mut ui: ResMut<UpgradeTreeUi>,
    model: Res<ResearchUiModel>,
    windows: Query<&Window>,
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
                ui.selected = model.root_index;
                ui.zoom = 1.0;
                center_on(&mut ui, &model, model.root_index, windows.single().ok());
                ui.inspector_dirty = true;
                ui.filter_dirty = true;
            }
        }
    }
}

/// Text entry is deliberately small and local: clicking focuses the search
/// field, Backspace edits it, Escape clears focus and Enter jumps to the first
/// result.
pub(super) fn research_keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut keyboard: MessageReader<KeyboardInput>,
    windows: Query<&Window>,
    model: Res<ResearchUiModel>,
    mut ui: ResMut<UpgradeTreeUi>,
) {
    if !ui.visible {
        keyboard.clear();
        return;
    }
    let was_search_active = ui.search_active;
    if ui.search_active {
        let old_query = ui.query.clone();
        let mut accepts_text = was_search_active;
        for event in keyboard.read() {
            if event.state != ButtonState::Pressed {
                continue;
            }
            if !accepts_text {
                accepts_text = true;
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
                        center_on(&mut ui, &model, first, windows.single().ok());
                        ui.inspector_dirty = true;
                        ui.filter_dirty = true;
                    }
                    ui.search_active = false;
                    ui.filter_dirty = true;
                }
                _ => {
                    if let Some(text) = event.text.as_deref() {
                        for character in text.chars().filter(|c| !c.is_control()) {
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
        keyboard.clear();
    } else {
        keyboard.clear();
    }
}

/// Keyboard and wheel navigation alter only one `UiTransform`, even though the
/// transformed canvas contains the complete catalog.
#[derive(SystemParam)]
pub(super) struct ResearchPointerInput<'w, 's> {
    buttons: Res<'w, ButtonInput<MouseButton>>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    viewport:
        Query<'w, 's, (&'static ComputedNode, &'static UiGlobalTransform), With<ResearchViewport>>,
}

impl ResearchPointerInput<'_, '_> {
    fn over_canvas(&self) -> bool {
        let Some(cursor) = self
            .windows
            .single()
            .ok()
            .and_then(|window| window.cursor_position())
        else {
            return false;
        };
        self.viewport
            .single()
            .is_ok_and(|(computed, transform)| computed.contains_point(*transform, cursor))
    }
}

pub(super) fn navigate_research_canvas(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut motion: MessageReader<CursorMoved>,
    mut wheel: MessageReader<MouseWheel>,
    mut ui: ResMut<UpgradeTreeUi>,
    pointer: ResearchPointerInput,
) {
    if !ui.visible {
        motion.clear();
        wheel.clear();
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
        if pointer.over_canvas()
            && (pointer.buttons.pressed(MouseButton::Left)
                || pointer.buttons.pressed(MouseButton::Middle))
        {
            for event in motion.read() {
                if let Some(pointer_delta) = event.delta {
                    pan_delta += research_drag_delta(pointer_delta);
                }
            }
        } else {
            motion.clear();
        }
    } else {
        motion.clear();
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
    model: Res<ResearchUiModel>,
    mut canvas: Query<&mut UiTransform, With<ResearchCanvas>>,
) {
    if !ui.visible || !ui.transform_dirty {
        return;
    }
    if let Ok(mut transform) = canvas.single_mut() {
        let translation = canvas_translation(ui.pan, ui.zoom, model.layout.size);
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
    mut connectors: Query<
        (&ResearchConnector, &mut Node, &mut BackgroundColor),
        Without<ResearchCard>,
    >,
    mut filters: Query<(&FilterButton, &mut KitToggle)>,
    mut search: Query<&mut Text, (With<SearchText>, Without<MatchCountText>)>,
    mut count: Query<&mut Text, (With<MatchCountText>, Without<SearchText>)>,
) {
    if !ui.visible || !ui.filter_dirty {
        return;
    }
    let matches = model.filtered_indices(&ui.query, ui.filter);
    let selected_path = model.selected_path(ui.selected);
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
    for (line, mut node, mut background) in &mut connectors {
        let connector = model.connectors[line.0];
        node.display = if visible[connector.from]
            && visible[connector.to]
            && ((ui.selected == model.root_index && connector.from == model.root_index)
                || selected_path.contains(&(connector.from, connector.to)))
        {
            Display::Flex
        } else {
            Display::None
        };
        background.0 = if selected_path.contains(&(connector.from, connector.to)) {
            READY_INK.with_alpha(0.95)
        } else {
            let depth_span =
                model.layout.depths[connector.to] - model.layout.depths[connector.from];
            category_color(research_catalog().nodes()[connector.to].category)
                .with_alpha(if depth_span == 1 { 0.34 } else { 0.22 })
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
                "Search research".to_owned()
            }
        } else if ui.search_active {
            format!("Search: {}|", ui.query)
        } else {
            format!("Search: {}", ui.query)
        };
    }
    if let Ok(mut text) = count.single_mut() {
        text.0 = format!("{} / {RESEARCH_NODE_COUNT} nodes", matches.len());
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
                CatalogNodeState::Owned => OWNED_PAPER,
                CatalogNodeState::Available => READY_PAPER,
                CatalogNodeState::Locked => LOCKED_PAPER,
            };
        }
        for (marker, mut text, mut color) in &mut states {
            let node = &research_catalog().nodes()[marker.0];
            let (label, ink) = match model.states[marker.0] {
                CatalogNodeState::Owned => ("Owned", OWNED_INK),
                CatalogNodeState::Available => ("Available", READY_INK),
                CatalogNodeState::Locked => ("Locked", LOCKED_INK),
            };
            text.0 = format!("{label} | Era {} | {:.0} pts", node.era, node.cost);
            color.0 = ink;
        }
        if let Ok(mut text) = currency.single_mut() {
            text.0 = format!(
                "{:.0} blessings  |  {:.0} research  |  {} scholars",
                research.blessings, research.research_points, research.researcher_count
            );
        }
        if let Ok(mut text) = next.single_mut() {
            text.0 = leader_priority_copy(research);
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
        ResearchPayload::BuildingAvailableAtFounding { building_id } => format!(
            "Available from founding: {}",
            title_case_identifier(building_id)
        ),
        ResearchPayload::UnlockBuilding { building_id } => {
            format!("Unlock building: {}", title_case_identifier(building_id))
        }
        ResearchPayload::UnlockRecipe { recipe_id } => {
            format!("Unlock recipe: {}", recipe_display_name(recipe_id))
        }
        ResearchPayload::UnlockResource { resource_id } => format!("Resource: {resource_id}"),
        ResearchPayload::UnlockJob { job_id } if job_id == "gather_logs" => {
            "Unlocks logging".to_owned()
        }
        ResearchPayload::UnlockJob { job_id } => {
            format!("Unlock job: {}", title_case_identifier(job_id))
        }
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
        ResearchPayload::UnlockCapability { capability_id } => match capability_id.as_str() {
            "rail_logistics" => "Blueprints: Rail transport".to_owned(),
            "water_travel" => "Blueprints: Shipping".to_owned(),
            _ => format!("Capability: {}", title_case_identifier(capability_id)),
        },
    }
}

pub(super) fn recipe_display_name(recipe_id: &str) -> String {
    match recipe_id {
        "grain_to_flour" => "Grain grinding".to_owned(),
        "flour_to_food" => "Food baking".to_owned(),
        "logs_to_lumber" => "Lumber cutting".to_owned(),
        "materials_to_refined" => "Refined materials".to_owned(),
        "ore_to_metal" => "Metal smelting".to_owned(),
        "fibre_to_thread" => "Thread spinning".to_owned(),
        "fibre_to_cloth" => "Cloth weaving".to_owned(),
        "bone_mug" => "Bone mug carving".to_owned(),
        "stone_mug" => "Stone mug carving".to_owned(),
        "metal_mug" => "Metal mug forging".to_owned(),
        "hide_to_leather" => "Leather tanning".to_owned(),
        "smithy_weapon" => "Weapon forging".to_owned(),
        "smithy_tool" => "Tool forging".to_owned(),
        "smithy_armor" => "Armor forging".to_owned(),
        "bone_tool" => "Bone tool carving".to_owned(),
        "bone_trinket" => "Bone decoration carving".to_owned(),
        "bone_toy" => "Bone toy carving".to_owned(),
        "gem_jewelry" => "Gem jewelry and decoration".to_owned(),
        "clay_mug" => "Clay mug pottery".to_owned(),
        "clay_bowl" => "Clay bowl pottery".to_owned(),
        "clay_brick" => "Fired clay brick goods".to_owned(),
        "sand_glass_mug" => "Glass mug casting".to_owned(),
        "sand_glass_bowl" => "Glass bowl casting".to_owned(),
        "sand_glass_trinket" => "Glass decoration casting".to_owned(),
        _ => title_case_identifier(recipe_id),
    }
}

fn title_case_identifier(identifier: &str) -> String {
    identifier
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Repaint only the one-node inspector.
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
        "{} | Era {} | {:.0} research",
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
            PurchaseState::ResearchReady => format!("Research for {:.0} points", node.cost),
            PurchaseState::ResearchUnaffordable => {
                format!("Need {:.0} research points", node.cost)
            }
            PurchaseState::LegacyReady => format!("Commission for {:.0} blessings", node.cost),
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
    !session_ready
        || !matches!(
            purchase_state,
            Some(PurchaseState::ResearchReady | PurchaseState::LegacyReady)
        )
}

fn research_purchase_action(
    model: &ResearchUiModel,
    research: &ResearchSnapshot,
    node_id: &str,
    session: &Session,
) -> Option<ClientAction> {
    if model.dispatchable_research_node(node_id, research) {
        Some(ClientAction::ResearchNode {
            session_id: session.session_id.clone(),
            nickname: CLIENT_ACTOR_LABEL.to_owned(),
            sig: session.sig.clone(),
            node_id: node_id.to_owned(),
        })
    } else if model.dispatchable_legacy_node(node_id, research) {
        Some(ClientAction::UnlockNode {
            session_id: session.session_id.clone(),
            nickname: CLIENT_ACTOR_LABEL.to_owned(),
            sig: session.sig.clone(),
            node_id: node_id.to_owned(),
        })
    } else {
        None
    }
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
    let action = research_purchase_action(&model, research, &node.id, &session);
    if let Some(action) = action {
        outgoing.0.push(action);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ledger_captures_global_shortcuts_only_while_search_is_active() {
        let mut ui = UpgradeTreeUi::default();
        assert!(!ui.captures_text_input());
        ui.visible = true;
        assert!(!ui.captures_text_input());
        ui.search_active = true;
        assert!(ui.captures_text_input());
        ui.visible = false;
        assert!(!ui.captures_text_input());
    }

    #[test]
    fn transport_payload_copy_promises_blueprints_not_magical_travel() {
        assert_eq!(
            payload_line(&ResearchPayload::UnlockCapability {
                capability_id: "rail_logistics".to_owned(),
            }),
            "Blueprints: Rail transport"
        );
        assert_eq!(
            payload_line(&ResearchPayload::UnlockCapability {
                capability_id: "water_travel".to_owned(),
            }),
            "Blueprints: Shipping"
        );
    }
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

    fn session() -> Session {
        Session {
            session_id: "research-session".to_owned(),
            sig: "signed".to_owned(),
            presence_sent: true,
            ready: true,
            ..default()
        }
    }

    #[test]
    fn model_represents_every_catalog_card_and_dependency_connector() {
        let model = ResearchUiModel::from_catalog();
        assert_eq!(model.entries.len(), RESEARCH_NODE_COUNT);
        assert_eq!(model.entries.len(), RESEARCH_NODE_COUNT);
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
        assert_eq!(model.category_count(ResearchCategory::Building), 165);
        assert_eq!(model.category_count(ResearchCategory::RecipeResource), 167);
        assert_eq!(model.category_count(ResearchCategory::Upgrade), 155);
        assert!(model.category_count(ResearchCategory::Building) * 3 >= RESEARCH_NODE_COUNT);
        assert!(model.category_count(ResearchCategory::RecipeResource) * 3 >= RESEARCH_NODE_COUNT);
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
    fn leader_priority_copy_never_claims_that_the_hint_is_already_being_studied() {
        let mut research = snapshot(&[], 0.0);
        assert_eq!(
            leader_priority_copy(&research),
            "No leader-priority study available"
        );
        research.next_target = Some(cat_protocol::ResearchTarget {
            id: "research_hut".to_owned(),
            name: "Research Hut".to_owned(),
            cost: 5.0,
        });
        let copy = leader_priority_copy(&research);
        assert_eq!(copy, "Leader priority: Research Hut · 5 pts");
    }

    #[test]
    fn inspector_copy_distinguishes_founding_access_from_a_research_unlock() {
        let research_hut = research_catalog().get("research_hut").unwrap();
        let founding_line = research_hut
            .payloads
            .iter()
            .map(payload_line)
            .find(|line| line.starts_with("Available from founding:"))
            .expect("research hut founding placement copy");
        assert_eq!(founding_line, "Available from founding: Research Hut");

        for (node_id, expected_building, expected_modifier) in [
            ("wood_cutter_foundations", "Wood Cutter", "Output Add"),
            (
                "stone_prep_foundations",
                "Stone Prep",
                "Durability Add 0.15",
            ),
            (
                "woodworking_foundations",
                "Woodworking",
                "Durability Add 0.15",
            ),
        ] {
            let lines = research_catalog()
                .get(node_id)
                .unwrap()
                .payloads
                .iter()
                .map(payload_line)
                .collect::<Vec<_>>();
            assert!(
                lines.contains(&format!("Available from founding: {expected_building}")),
                "{node_id}: {lines:?}"
            );
            assert!(
                lines.iter().any(|line| line.contains(expected_modifier)),
                "the founding marker must not hide {node_id}'s purchased modifier"
            );
            assert!(
                lines
                    .iter()
                    .all(|line| !line.starts_with("Unlock building:"))
            );
        }

        let milling = research_catalog().get("milling").unwrap();
        assert!(
            milling
                .payloads
                .iter()
                .map(payload_line)
                .any(|line| line == "Unlock building: Mill")
        );
        assert!(
            research_catalog()
                .get("mill_foundations")
                .unwrap()
                .payloads
                .iter()
                .map(payload_line)
                .all(|line| !line.starts_with("Unlock building:"))
        );
    }

    #[test]
    fn inspector_names_the_sole_real_job_and_new_physical_recipes_for_players() {
        let sawmill = research_catalog().get("sawmill").unwrap();
        assert_eq!(
            sawmill
                .payloads
                .iter()
                .filter(|payload| matches!(payload, ResearchPayload::UnlockJob { .. }))
                .map(payload_line)
                .collect::<Vec<_>>(),
            ["Unlocks logging"]
        );

        for (node_id, expected) in [
            (
                "textiles",
                [
                    "Unlock recipe: Thread spinning",
                    "Unlock recipe: Cloth weaving",
                    "Unlock recipe: Leather tanning",
                ]
                .as_slice(),
            ),
            (
                "weaponsmithing",
                ["Unlock recipe: Weapon forging"].as_slice(),
            ),
            ("armorsmithing", ["Unlock recipe: Armor forging"].as_slice()),
        ] {
            let recipe_lines = research_catalog()
                .get(node_id)
                .unwrap()
                .payloads
                .iter()
                .filter(|payload| matches!(payload, ResearchPayload::UnlockRecipe { .. }))
                .map(payload_line)
                .collect::<Vec<_>>();
            assert_eq!(recipe_lines, expected, "{node_id}");
        }

        assert_eq!(
            research_catalog()
                .nodes()
                .iter()
                .flat_map(|node| node.payloads.iter())
                .filter(|payload| matches!(payload, ResearchPayload::UnlockJob { .. }))
                .count(),
            1
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
    fn every_catalog_node_uses_research_while_legacy_nodes_retain_blessing_fallback() {
        let model = ResearchUiModel::from_catalog();
        let mut research = snapshot(&["research_hut"], 99.0);
        assert_eq!(
            model.purchase_state("basic_tools", &research),
            PurchaseState::ResearchReady
        );
        assert!(model.dispatchable_research_node("basic_tools", &research));
        assert!(!model.dispatchable_legacy_node("basic_tools", &research));

        research.research_points = 0.0;
        assert_eq!(
            model.purchase_state("basic_tools", &research),
            PurchaseState::LegacyReady
        );
        research.research_points = 99.0;
        let generated = research_catalog()
            .nodes()
            .iter()
            .find(|node| node.id == "research_hut_foundations")
            .unwrap();
        assert_eq!(
            model.purchase_state(&generated.id, &research),
            PurchaseState::ResearchReady
        );
        assert!(!model.dispatchable_legacy_node("basic_tools", &research));
        assert!(model.dispatchable_research_node(&generated.id, &research));
        assert!(!model.dispatchable_legacy_node(&generated.id, &research));

        let generated_action =
            research_purchase_action(&model, &research, &generated.id, &session());
        assert!(matches!(
            generated_action,
            Some(ClientAction::ResearchNode { node_id, .. }) if node_id == generated.id
        ));

        research.research_points = 12.0;
        assert!(matches!(
            research_purchase_action(&model, &research, "basic_tools", &session()),
            Some(ClientAction::ResearchNode { node_id, .. }) if node_id == "basic_tools"
        ));
        research.research_points = 0.0;
        assert!(matches!(
            research_purchase_action(&model, &research, "basic_tools", &session()),
            Some(ClientAction::UnlockNode { node_id, .. }) if node_id == "basic_tools"
        ));
    }

    #[test]
    fn supported_generated_recipe_resource_study_dispatches_research() {
        let model = ResearchUiModel::from_catalog();
        let mut research = snapshot(&["research_hut", "basic_tools", "textiles"], 0.0);
        research.research_points = 999.0;
        assert_eq!(
            model.purchase_state("textile_work_sources", &research),
            PurchaseState::ResearchReady
        );
        assert!(model.dispatchable_research_node("textile_work_sources", &research));
        assert!(matches!(
            research_purchase_action(&model, &research, "textile_work_sources", &session()),
            Some(ClientAction::ResearchNode { node_id, .. }) if node_id == "textile_work_sources"
        ));
        assert!(!research_purchase_disabled(
            true,
            Some(PurchaseState::ResearchReady)
        ));
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
            Some(PurchaseState::ResearchUnaffordable)
        ));
        assert!(research_purchase_disabled(
            true,
            Some(PurchaseState::Locked)
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
        let viewport = world
            .query_filtered::<&Node, With<ResearchViewport>>()
            .single(&world)
            .expect("research ledger should have one viewport");
        assert_eq!(viewport.overflow, Overflow::clip());
        assert!(
            world
                .query_filtered::<&Node, With<ResearchCard>>()
                .iter(&world)
                .all(|node| node.overflow == Overflow::clip())
        );
    }

    #[test]
    fn canvas_scale_compensation_keeps_map_origin_at_pan() {
        let model = ResearchUiModel::from_catalog();
        let pan = Vec2::new(18.0, 16.0);
        let zoom = 0.82;
        let centre = model.layout.size / 2.0;
        for point in [
            Vec2::ZERO,
            model.card_position(model.root_index),
            model.layout.size,
        ] {
            let rendered =
                centre + (point - centre) * zoom + canvas_translation(pan, zoom, model.layout.size);
            let expected = pan + point * zoom;
            assert!(rendered.abs_diff_eq(expected, 0.01));
        }
    }

    #[test]
    fn pointer_drag_moves_research_canvas_faster_than_the_hand() {
        let pointer = Vec2::new(80.0, -40.0);
        let pan = research_drag_delta(pointer);
        assert!(pan.abs_diff_eq(pointer * RESEARCH_DRAG_GAIN, 0.001));
        assert!(pan.length() > pointer.length());
    }

    #[test]
    fn unified_layout_is_one_rooted_non_overlapping_dependency_tree() {
        let model = ResearchUiModel::from_catalog();
        let roots = research_catalog()
            .nodes()
            .iter()
            .enumerate()
            .filter(|(_, node)| node.prerequisites.is_empty())
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        assert_eq!(roots, [model.root_index]);
        assert!(model.layout.layer_count < 24);
        assert!(model.layout.size.x < 5_000.0);
        for connector in &model.connectors {
            assert!(model.layout.depths[connector.from] < model.layout.depths[connector.to]);
            assert!(model.card_position(connector.from).x < model.card_position(connector.to).x);
        }
        for (index, position) in model.layout.positions.iter().enumerate() {
            for other in model.layout.positions.iter().skip(index + 1) {
                assert!(
                    position.x != other.x || (position.y - other.y).abs() >= MAP_STEP_Y,
                    "two studies overlap in one prerequisite layer"
                );
            }
        }
    }

    #[test]
    fn selected_branch_highlights_every_ancestor_back_to_the_root() {
        let model = ResearchUiModel::from_catalog();
        let selected = model
            .layout
            .depths
            .iter()
            .enumerate()
            .max_by_key(|(_, depth)| **depth)
            .map(|(index, _)| index)
            .unwrap();
        let path = model.selected_path(selected);
        assert!(path.len() >= model.layout.depths[selected]);
        assert!(path.iter().any(|(from, _)| *from == model.root_index));
    }
}
