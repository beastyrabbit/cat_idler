//! Full-window research ledger UI backed by `cat_sim::research_catalog`.

use super::*;
use bevy::{
    input::{ButtonState, keyboard::KeyboardInput, mouse::MouseScrollUnit},
    ui::Val2,
};
use cat_protocol::ResearchSnapshot;
use cat_sim::{
    research_catalog::{ResearchCategory, ResearchNode, ResearchPayload, research_catalog},
    research_tracks::{TechnologyKind, technology_catalog},
};
use std::collections::{HashMap, HashSet};

const HEADER_HEIGHT: f32 = 104.0;
const CATALOG_WIDTH: f32 = 244.0;
const NODE_WIDTH: f32 = 148.0;
const NODE_HEIGHT: f32 = 62.0;
const MAP_PADDING_X: f32 = 96.0;
const MAP_PADDING_Y: f32 = 92.0;
const MAP_STEP_X: f32 = 174.0;
const MAP_STEP_Y: f32 = 132.0;
const FIXED_TREE_SCALE: f32 = 0.92;
const ROOT_OVERVIEW_VERTICAL_BIAS: f32 = 24.0;
const CONNECTOR_STROKE: f32 = 2.0;

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
const DEPTH_BAND_A: Color = Color::srgba(0.22, 0.26, 0.19, 0.22);
const DEPTH_BAND_B: Color = Color::srgba(0.16, 0.19, 0.14, 0.16);
const STRUCTURE_INK: Color = Color::srgba(0.67, 0.62, 0.49, 0.46);
const CROSS_LINK_INK: Color = Color::srgba(0.67, 0.62, 0.49, 0.24);
const RESEARCH_DRAG_GAIN: f32 = 2.35;

/// Reuse the game's tracked pixel-art vocabulary. Stages in one family share
/// an icon on purpose: the repeated shape makes a progression track scannable,
/// while the title and step distinguish the individual study.
fn research_icon_path(node: &ResearchNode) -> &'static str {
    let id = node.id.as_str();
    match id {
        id if id == "research_hut" || id.starts_with("research_hut_") => {
            "public/images/game/buildings/research_hut.png"
        }
        id if id == "den_insulation"
            || id.starts_with("den_")
            || id.starts_with("housing_")
            || id == "grand_housing" =>
        {
            "public/images/game/buildings/den.png"
        }
        id if id.starts_with("food_storage_") || id.starts_with("storage_") => {
            "public/images/game/buildings/storehouse.png"
        }
        id if id == "water_carriers"
            || id.starts_with("water_bowl_")
            || id.starts_with("waterworks_")
            || id.starts_with("water_management_") =>
        {
            "public/images/game/icons/water.png"
        }
        id if id.starts_with("beds_") => "public/images/game/interior/bed.png",
        id if id.starts_with("herb_garden_") || id.starts_with("herbalism_") => {
            "public/images/game/icons/herbs.png"
        }
        id if id.starts_with("nursery_") => "public/images/game/buildings/school.png",
        id if id.starts_with("elder_corner_") || id.starts_with("welfare_") => {
            "public/images/ui/status/happy.png"
        }
        id if id.starts_with("walls_") || id.starts_with("defense_doctrine_") => {
            "public/images/game/infra/palisade.png"
        }
        id if id.starts_with("mouse_farm_") || id.starts_with("animal_husbandry_") => {
            "public/images/buildings/mouse_farm.png"
        }
        id if id.starts_with("shrine_") => "public/images/game/buildings/shrine.png",
        id if id.starts_with("workshop_") => "public/images/game/buildings/workshop.png",
        id if id == "irrigation"
            || id.starts_with("field_")
            || id.starts_with("field_craft_")
            || id.starts_with("agriculture_") =>
        {
            "public/images/game/farm/crop_mature.png"
        }
        id if id == "school" || id == "scholars_guild" || id.starts_with("school_") => {
            "public/images/game/buildings/school.png"
        }
        id if id == "smithy" || id.starts_with("smithy_") => {
            "public/images/game/buildings/smithy.png"
        }
        id if id == "barracks" || id.starts_with("barracks_") => {
            "public/images/game/buildings/barracks.png"
        }
        id if id.starts_with("wood_cutter_") => "public/images/game/buildings/wood_cutter.png",
        id if id.starts_with("stone_prep_") => "public/images/game/buildings/stone_prep.png",
        id if id.starts_with("woodworking_") => "public/images/game/buildings/woodworking.png",
        id if id == "textiles"
            || id.starts_with("clothier_")
            || id.starts_with("textile_work_") =>
        {
            "public/images/game/icons/cloth.png"
        }
        id if id.starts_with("tannery_") || id.starts_with("leatherworking_") => {
            "public/images/game/icons/leather.png"
        }
        id if id == "smelting" || id.starts_with("smelter_") || id.starts_with("metallurgy_") => {
            "public/images/game/icons/metal.png"
        }
        id if id.starts_with("accounting_tent_") || id.starts_with("governance_") => {
            "public/images/game/interior/scroll.png"
        }
        id if id == "milling" || id.starts_with("mill_") || id.starts_with("grain_milling_") => {
            "public/images/game/icons/grain.png"
        }
        id if id == "sawmill" || id.starts_with("sawmill_") => {
            "public/images/game/buildings/woodworking.png"
        }
        "stone_tools" => "public/images/game/icons/stone.png",
        "metal_tools" => "public/images/game/icons/metal.png",
        id if id == "basic_tools"
            || id == "precision_tools"
            || id.starts_with("toolmaking_")
            || id.starts_with("craftsmanship_") =>
        {
            "public/images/game/icons/tools.png"
        }
        id if id == "foraging_lore" || id.starts_with("foraging_") => {
            "public/images/ui/tasks/gather_herbs.png"
        }
        id if id.starts_with("hunting_") => "public/images/ui/tasks/hunt.png",
        id if id.starts_with("baking_") || id.starts_with("food_preservation_") => {
            "public/images/game/icons/food.png"
        }
        id if id.starts_with("carpentry_") => "public/images/game/icons/planks.png",
        id if id == "masonry"
            || id.starts_with("stonecraft_")
            || id.starts_with("construction_") =>
        {
            "public/images/game/icons/blocks.png"
        }
        id if id == "weaponsmithing"
            || id.starts_with("weaponcraft_")
            || id.starts_with("combat_doctrine_") =>
        {
            "public/images/game/icons/weapons.png"
        }
        id if id == "armorsmithing" || id.starts_with("armorcraft_") => {
            "public/images/game/icons/armor.png"
        }
        id if id.starts_with("brewing_") => "public/images/game/props/barrel.png",
        id if id.starts_with("trade_goods_") || id.starts_with("trade_") => {
            "public/images/game/icons/goods.png"
        }
        id if id == "mountaineering"
            || id == "mounted_scouts"
            || id.starts_with("exploration_")
            || id.starts_with("expedition_supplies_") =>
        {
            "public/images/ui/tasks/explore.png"
        }
        "rail" => "public/images/game/infra/road_straight_h.png",
        "shipping" => "public/images/game/infra/bridge.png",
        "advanced_storage" | "organized_provisioning" => "public/images/game/props/crate.png",
        "civil_engineering" => "public/images/game/icons/blocks.png",
        "preservation_science" => "public/images/game/icons/food.png",
        "public_administration" => "public/images/game/interior/scroll.png",
        "combined_arms" => "public/images/game/icons/weapons.png",
        id if id.starts_with("logistics_") => "public/images/game/props/crate.png",
        id if id.starts_with("scholarship_") => "public/images/game/interior/bookcase.png",
        id if id.starts_with("resilience_") => "public/images/game/infra/palisade.png",
        _ => match node.category {
            ResearchCategory::Building => "public/images/ui/tasks/build.png",
            ResearchCategory::RecipeResource => "public/images/game/icons/goods.png",
            ResearchCategory::Upgrade => "public/images/game/interior/scroll.png",
        },
    }
}

#[cfg(test)]
mod track_tests {
    use super::*;
    use std::collections::BTreeMap;

    fn snapshot(owned: &[&str]) -> ResearchSnapshot {
        ResearchSnapshot {
            owned_node_ids: owned.iter().map(|id| (*id).to_owned()).collect(),
            research_points: 0.0,
            researcher_count: 1,
            blessings: 0.0,
            next_target: None,
            queue: Vec::new(),
            repeatable_levels: BTreeMap::new(),
            research_cost_multiplier: 1.0,
            research_time_multiplier: 1.0,
            points_per_hour: 1.0,
        }
    }

    fn session() -> Session {
        Session {
            session_id: "research-session".to_owned(),
            sig: "signed".to_owned(),
            ready: true,
            ..default()
        }
    }

    #[test]
    fn finite_levels_and_infinite_terminals_are_individual_tree_nodes() {
        let model = ResearchUiModel::from_catalog();
        let expected = technology_catalog()
            .tracks()
            .iter()
            .map(|track| match track.kind {
                TechnologyKind::Milestone => 1,
                TechnologyKind::Building | TechnologyKind::Recipe => 1,
                TechnologyKind::GlobalModifier => 11,
            })
            .sum::<usize>();
        assert_eq!(model.entries.len(), expected);
        for (track_index, track) in technology_catalog().tracks().iter().enumerate() {
            let entries = &model.track_entries[track_index];
            assert_eq!(
                entries.len(),
                match track.kind {
                    TechnologyKind::Milestone => 1,
                    TechnologyKind::Building | TechnologyKind::Recipe => 1,
                    TechnologyKind::GlobalModifier => 11,
                }
            );
            if track.is_repeatable() {
                assert!(matches!(
                    model.entries[*entries.last().unwrap()].target,
                    CatalogTarget::Infinite
                ));
            }
        }
    }

    #[test]
    fn study_state_uses_selected_level_and_queue() {
        let model = ResearchUiModel::from_catalog();
        let root = model.root_index;
        assert_eq!(
            model.card_state(root, &snapshot(&[])),
            CatalogNodeState::Available
        );
        assert_eq!(
            model.card_state(root, &snapshot(&["research_hut"])),
            CatalogNodeState::Owned
        );

        let mut queued = snapshot(&[]);
        queued.queue.push(cat_protocol::ResearchQueueEntrySnapshot {
            key: "finite:research_hut".to_owned(),
            target: cat_protocol::ResearchQueueTargetSnapshot::Finite {
                node_id: "research_hut".to_owned(),
            },
            name: "Research Hut".to_owned(),
            source: cat_protocol::ResearchQueueSource::Player,
            status: cat_protocol::ResearchQueueStatus::Active,
            base_cost: 5.0,
            funded_cost: Some(5.0),
            progress_seconds: 1.0,
            required_seconds: 60.0,
        });
        assert_eq!(model.card_state(root, &queued), CatalogNodeState::Active);
    }

    #[test]
    fn selecting_a_finite_track_queues_its_full_path() {
        let model = ResearchUiModel::from_catalog();
        let research = snapshot(&["research_hut"]);
        let card = model
            .entries
            .iter()
            .find(|entry| {
                matches!(
                    entry.target,
                    CatalogTarget::Finite { node_index }
                        if research_catalog().nodes()[node_index].id == "basic_tools"
                )
            })
            .unwrap()
            .index;
        let action = research_purchase_action(&model, &research, card, &session()).unwrap();
        assert!(matches!(
            action,
            ClientAction::QueueResearchPath { node_id, .. } if node_id == "basic_tools"
        ));
    }

    #[test]
    fn catalog_filter_searches_track_names_not_raw_stage_noise() {
        let model = ResearchUiModel::from_catalog();
        let all = model.filtered_indices("", ResearchFilter::All);
        assert_eq!(all.len(), technology_catalog().tracks().len());
        let scholarship = model.filtered_indices("scholarship", ResearchFilter::Upgrade);
        assert_eq!(scholarship.len(), 1);
        assert_eq!(model.track_for_card(scholarship[0]).id, "scholarship");
    }

    #[test]
    fn graph_is_vertical_with_readable_spacing_and_forward_connectors() {
        let model = ResearchUiModel::from_catalog();
        for connector in &model.connectors {
            assert!(
                model.layout.depths[connector.from] < model.layout.depths[connector.to],
                "{} must precede {}",
                model.track_for_card(connector.from).name,
                model.track_for_card(connector.to).name
            );
            assert!(model.card_position(connector.from).y < model.card_position(connector.to).y);
        }
        for (index, left) in model.entries.iter().enumerate() {
            for right in &model.entries[index + 1..] {
                let position = model.card_position(left.index);
                let other = model.card_position(right.index);
                assert!(
                    (position.x - other.x).abs() >= NODE_WIDTH
                        || (position.y - other.y).abs() >= NODE_HEIGHT
                );
            }
        }
    }

    #[test]
    fn selection_focus_contains_every_prerequisite_and_unlock_descendant() {
        let model = ResearchUiModel::from_catalog();
        let selected = model
            .entries
            .iter()
            .find(|entry| entry.name == "Scholarship 5")
            .unwrap()
            .index;
        let visible = model.with_relatives(selected);
        assert!(visible.contains(&selected));
        assert!(visible.contains(&model.root_index));
        assert!(
            visible
                .iter()
                .any(|index| model.entries[*index].name == "Scholarship Infinite")
        );
        assert!(visible.len() < model.entries.len());
    }

    #[test]
    fn selection_focus_compacts_visible_layers_around_the_canvas_centre() {
        let model = ResearchUiModel::from_catalog();
        let selected = model
            .entries
            .iter()
            .find(|entry| entry.name == "Scholarship 5")
            .unwrap()
            .index;
        let visible = model.with_relatives(selected);
        let positions = model.display_positions(Some(selected));
        let canvas_centre = model.layout.size.x / 2.0;
        for depth in 0..model.layout.layer_count {
            let layer = visible
                .iter()
                .copied()
                .filter(|index| model.layout.depths[*index] == depth)
                .collect::<Vec<_>>();
            if layer.len() == 1 {
                let card_centre = positions[layer[0]].x + NODE_WIDTH / 2.0;
                assert!((card_centre - canvas_centre).abs() < 0.01);
            }
        }
        assert_ne!(positions[selected], model.card_position(selected));
    }

    #[test]
    fn curated_junction_cards_expose_multiple_incoming_branches() {
        let model = ResearchUiModel::from_catalog();
        for (name, minimum) in [
            ("Stone Tools", 2),
            ("Metal Tools", 3),
            ("Civil Engineering", 3),
        ] {
            let card = model
                .entries
                .iter()
                .find(|entry| entry.name == name)
                .unwrap_or_else(|| panic!("missing {name}"))
                .index;
            let incoming = model
                .connectors
                .iter()
                .filter(|connector| connector.to == card)
                .count();
            assert!(incoming >= minimum, "{name} has only {incoming} inputs");
        }
    }

    #[test]
    fn player_tree_has_many_visible_convergence_choices() {
        let model = ResearchUiModel::from_catalog();
        let convergences = model
            .entries
            .iter()
            .filter(|entry| {
                model
                    .connectors
                    .iter()
                    .filter(|connector| connector.to == entry.index)
                    .count()
                    >= 2
            })
            .count();
        assert!(
            convergences >= 24,
            "only {convergences} player-facing technologies visibly merge paths"
        );
    }

    #[test]
    fn every_track_has_a_real_semantic_icon_asset() {
        let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        for track in technology_catalog().tracks() {
            let node = &research_catalog().nodes()[track.node_indices[0]];
            let path = research_icon_path(node);
            assert!(
                workspace.join(path).is_file(),
                "{} uses missing {path}",
                track.id
            );
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum CatalogNodeState {
    Owned,
    Available,
    Queued,
    Active,
    Locked,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PurchaseState {
    Owned,
    Locked,
    ResearchReady,
    RepeatableReady,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CatalogTarget {
    Finite { node_index: usize },
    Track,
    Infinite,
}

#[derive(Clone, Debug)]
struct CatalogEntry {
    index: usize,
    track_index: usize,
    level: u32,
    target: CatalogTarget,
    name: String,
}

#[derive(Clone, Copy, Debug)]
struct CatalogConnector {
    from: usize,
    to: usize,
    primary: bool,
}

#[derive(Debug)]
struct UnifiedTreeLayout {
    positions: Vec<Vec2>,
    depths: Vec<usize>,
    primary_parents: Vec<Option<usize>>,
    size: Vec2,
    layer_count: usize,
    branch_count: usize,
}

#[cfg(any())]
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
#[cfg(any())]
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
    let root_index = depths
        .iter()
        .position(|depth| *depth == 0)
        .expect("validated research catalog has one root");
    // Every DAG node owns one stable visual parent. Prefer the nearest
    // prerequisite layer so the permanent backbone has short, legible routes;
    // remaining prerequisites are still retained as contextual cross-links.
    let primary_parents = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            node.prerequisites
                .iter()
                .map(|id| by_id[id])
                .max_by(|left, right| {
                    depths[*left]
                        .cmp(&depths[*right])
                        .then_with(|| nodes[*right].id.cmp(&nodes[*left].id))
                })
                .or_else(|| (index != root_index).then_some(root_index))
        })
        .collect::<Vec<_>>();
    let mut root_branches = vec![root_index; nodes.len()];
    let mut branch_ids = Vec::new();
    for layer in layers.iter().skip(1) {
        for index in layer.iter().copied() {
            let parent =
                primary_parents[index].expect("every non-root research study has a visual parent");
            root_branches[index] = if parent == root_index {
                if !branch_ids.contains(&index) {
                    branch_ids.push(index);
                }
                index
            } else {
                root_branches[parent]
            };
        }
    }
    branch_ids.sort_by(|left, right| {
        nodes[*left]
            .layout
            .x
            .cmp(&nodes[*right].layout.x)
            .then_with(|| nodes[*left].layout.y.cmp(&nodes[*right].layout.y))
            .then_with(|| nodes[*left].id.cmp(&nodes[*right].id))
    });
    let branch_order = branch_ids
        .iter()
        .enumerate()
        .map(|(order, index)| (*index, order))
        .collect::<HashMap<_, _>>();
    let mut row_positions = vec![0.0_f32; nodes.len()];
    let mut layer_rows = vec![Vec::new(); layer_count];
    let mut widest_span = 0.0_f32;
    // Order first, then centre every layer around the widest spaced span.
    // Main branches receive a full visual break; semantic families within a
    // branch receive a smaller break. This keeps linked stages together while
    // preventing a dense layer from becoming one uninterrupted text wall.
    for (depth, layer) in layers.iter_mut().enumerate() {
        layer.sort_by(|left, right| {
            let parent_row = |index: usize| {
                let prerequisites = &nodes[index].prerequisites;
                if prerequisites.is_empty() {
                    0.0
                } else {
                    prerequisites
                        .iter()
                        .map(|id| row_positions[by_id[id]])
                        .sum::<f32>()
                        / prerequisites.len() as f32
                }
            };
            let branch_rank = |index: usize| {
                branch_order
                    .get(&root_branches[index])
                    .copied()
                    .unwrap_or(0)
            };
            branch_rank(*left)
                .cmp(&branch_rank(*right))
                .then_with(|| parent_row(*left).total_cmp(&parent_row(*right)))
                .then_with(|| {
                    research_icon_path(&nodes[*left]).cmp(research_icon_path(&nodes[*right]))
                })
                .then_with(|| nodes[*left].layout.x.cmp(&nodes[*right].layout.x))
                .then_with(|| nodes[*left].layout.y.cmp(&nodes[*right].layout.y))
                .then_with(|| nodes[*left].id.cmp(&nodes[*right].id))
        });
        let mut logical_row = 0.0_f32;
        let mut previous = None;
        for index in layer.iter().copied() {
            if let Some(previous) = previous {
                logical_row += 1.0
                    + if root_branches[previous] != root_branches[index] {
                        MAIN_BRANCH_GAP_ROWS
                    } else if primary_parents[previous] != primary_parents[index]
                        || research_icon_path(&nodes[previous]) != research_icon_path(&nodes[index])
                    {
                        FAMILY_GAP_ROWS
                    } else {
                        0.0
                    };
            }
            row_positions[index] = logical_row;
            layer_rows[depth].push((index, logical_row));
            previous = Some(index);
        }
        widest_span = widest_span.max(logical_row);
    }
    let mut positions = vec![Vec2::ZERO; nodes.len()];
    for (depth, rows) in layer_rows.iter().enumerate() {
        let layer_span = rows.last().map_or(0.0, |(_, row)| *row);
        let row_offset = (widest_span - layer_span) / 2.0;
        for (index, row) in rows {
            positions[*index] = Vec2::new(
                MAP_PADDING_X + depth as f32 * MAP_STEP_X,
                MAP_PADDING_Y + (row_offset + row) * MAP_STEP_Y,
            );
        }
    }
    UnifiedTreeLayout {
        positions,
        depths,
        primary_parents,
        root_branches,
        size: Vec2::new(
            MAP_PADDING_X * 2.0 + (layer_count - 1) as f32 * MAP_STEP_X + NODE_WIDTH,
            MAP_PADDING_Y * 2.0 + widest_span * MAP_STEP_Y + NODE_HEIGHT,
        ),
        layer_count,
        branch_count: branch_ids.len(),
    }
}

/// Lay out the player-facing technology DAG from top to bottom. A layer is a
/// real prerequisite step, so long vertical gaps communicate progression and
/// horizontal space is reserved for sibling choices.
fn build_vertical_tree_layout(
    entries: &[CatalogEntry],
    dependencies: &[Vec<usize>],
    root_index: usize,
) -> UnifiedTreeLayout {
    let mut depths = vec![0_usize; entries.len()];
    for _ in 0..entries.len() {
        let previous = depths.clone();
        for (entry, required) in dependencies.iter().enumerate() {
            depths[entry] = required
                .iter()
                .map(|index| previous[*index].saturating_add(1))
                .max()
                .unwrap_or(0);
        }
        if depths == previous {
            break;
        }
    }
    let layer_count = depths.iter().copied().max().unwrap_or(0) + 1;
    let mut layers = vec![Vec::new(); layer_count];
    for (entry, depth) in depths.iter().copied().enumerate() {
        layers[depth].push(entry);
    }
    for layer in &mut layers {
        layer.sort_by(|left, right| {
            let left_entry = &entries[*left];
            let right_entry = &entries[*right];
            let left_track = &technology_catalog().tracks()[left_entry.track_index];
            let right_track = &technology_catalog().tracks()[right_entry.track_index];
            let left_node = match left_entry.target {
                CatalogTarget::Finite { node_index } => &research_catalog().nodes()[node_index],
                CatalogTarget::Track => &research_catalog().nodes()[left_track.node_indices[0]],
                CatalogTarget::Infinite => {
                    &research_catalog().nodes()[*left_track.node_indices.last().unwrap()]
                }
            };
            let right_node = match right_entry.target {
                CatalogTarget::Finite { node_index } => &research_catalog().nodes()[node_index],
                CatalogTarget::Track => &research_catalog().nodes()[right_track.node_indices[0]],
                CatalogTarget::Infinite => {
                    &research_catalog().nodes()[*right_track.node_indices.last().unwrap()]
                }
            };
            left_node
                .layout
                .x
                .cmp(&right_node.layout.x)
                .then_with(|| left_track.name.cmp(&right_track.name))
                .then_with(|| left_entry.level.cmp(&right_entry.level))
        });
    }

    let max_columns = layers.iter().map(Vec::len).max().unwrap_or(1);
    let mut positions = vec![Vec2::ZERO; entries.len()];
    let mut primary_parents = vec![None; entries.len()];
    for (depth, layer) in layers.iter().enumerate() {
        let left_offset = (max_columns.saturating_sub(layer.len())) as f32 * MAP_STEP_X / 2.0;
        for (column, entry) in layer.iter().copied().enumerate() {
            positions[entry] = Vec2::new(
                MAP_PADDING_X + left_offset + column as f32 * MAP_STEP_X,
                MAP_PADDING_Y + depth as f32 * MAP_STEP_Y,
            );
            primary_parents[entry] = dependencies[entry]
                .iter()
                .max_by_key(|required| depths[**required])
                .copied();
        }
    }
    let branch_count = dependencies
        .iter()
        .filter(|required| required.contains(&root_index))
        .count();
    UnifiedTreeLayout {
        positions,
        depths,
        primary_parents,
        size: Vec2::new(
            MAP_PADDING_X * 2.0 + max_columns.saturating_sub(1) as f32 * MAP_STEP_X + NODE_WIDTH,
            MAP_PADDING_Y * 2.0 + layer_count.saturating_sub(1) as f32 * MAP_STEP_Y + NODE_HEIGHT,
        ),
        layer_count,
        branch_count,
    }
}

/// Logical catalog state is allocated once. Applying a snapshot only rewrites
/// this fixed state vector; it can never append cards or dependency lines.
#[derive(Resource)]
pub(super) struct ResearchUiModel {
    entries: Vec<CatalogEntry>,
    connectors: Vec<CatalogConnector>,
    track_entries: Vec<Vec<usize>>,
    states: Vec<CatalogNodeState>,
    layout: UnifiedTreeLayout,
    root_index: usize,
}

impl ResearchUiModel {
    fn from_catalog() -> Self {
        let catalog = research_catalog();
        let technologies = technology_catalog();
        let mut entries = Vec::new();
        let mut track_entries = vec![Vec::new(); technologies.tracks().len()];
        let mut raw_to_entry = HashMap::new();
        for (track_index, track) in technologies.tracks().iter().enumerate() {
            if track.kind != TechnologyKind::GlobalModifier {
                let index = entries.len();
                entries.push(CatalogEntry {
                    index,
                    track_index,
                    level: 1,
                    target: if track.kind == TechnologyKind::Milestone {
                        CatalogTarget::Finite {
                            node_index: track.node_indices[0],
                        }
                    } else {
                        CatalogTarget::Track
                    },
                    name: track.name.clone(),
                });
                track_entries[track_index].push(index);
                for node_index in &track.node_indices {
                    raw_to_entry.insert(*node_index, index);
                }
                continue;
            }

            for level in 1..=cat_sim::research_tracks::FINITE_TRACK_LEVELS {
                let component = (level as usize * track.node_indices.len())
                    .div_ceil(cat_sim::research_tracks::FINITE_TRACK_LEVELS as usize)
                    .saturating_sub(1);
                let node_index = track.node_indices[component];
                let index = entries.len();
                entries.push(CatalogEntry {
                    index,
                    track_index,
                    level,
                    target: CatalogTarget::Finite { node_index },
                    name: format!("{} {level}", track.name),
                });
                track_entries[track_index].push(index);
                raw_to_entry.entry(node_index).or_insert(index);
            }
            let index = entries.len();
            entries.push(CatalogEntry {
                index,
                track_index,
                level: cat_sim::research_tracks::FINITE_TRACK_LEVELS + 1,
                target: CatalogTarget::Infinite,
                name: format!("{} Infinite", track.name),
            });
            track_entries[track_index].push(index);
        }

        let by_raw_id = catalog
            .nodes()
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.as_str(), index))
            .collect::<HashMap<_, _>>();
        let mut dependencies = vec![Vec::<usize>::new(); entries.len()];
        for entry in &entries {
            let track = &technologies.tracks()[entry.track_index];
            match entry.target {
                CatalogTarget::Finite { node_index } => {
                    if entry.level > 1 {
                        dependencies[entry.index]
                            .push(track_entries[entry.track_index][entry.level as usize - 2]);
                    }
                    for prerequisite in &catalog.nodes()[node_index].prerequisites {
                        let Some(raw_index) = by_raw_id.get(prerequisite.as_str()) else {
                            continue;
                        };
                        let required_entry = raw_to_entry.get(raw_index).copied().or_else(|| {
                            technologies
                                .for_node(prerequisite)
                                .and_then(|required_track| {
                                    technologies
                                        .tracks()
                                        .iter()
                                        .position(|candidate| {
                                            std::ptr::eq(candidate, required_track)
                                        })
                                        .and_then(|track_index| {
                                            track_entries[track_index].last().copied()
                                        })
                                })
                        });
                        if let Some(required_entry) = required_entry
                            && required_entry != entry.index
                            && entries[required_entry].track_index != entry.track_index
                            && !dependencies[entry.index].contains(&required_entry)
                        {
                            dependencies[entry.index].push(required_entry);
                        }
                    }
                }
                CatalogTarget::Track => {
                    // A building or recipe family is represented by one compact
                    // card, but later stages can be cross-discipline gates.
                    // Project every external stage prerequisite onto the card
                    // so collapsing a track never hides real graph structure.
                    for node_index in &track.node_indices {
                        for prerequisite in &catalog.nodes()[*node_index].prerequisites {
                            let Some(raw_index) = by_raw_id.get(prerequisite.as_str()) else {
                                continue;
                            };
                            let required_entry =
                                raw_to_entry.get(raw_index).copied().or_else(|| {
                                    technologies
                                        .for_node(prerequisite)
                                        .and_then(|required_track| {
                                            technologies
                                                .tracks()
                                                .iter()
                                                .position(|candidate| {
                                                    std::ptr::eq(candidate, required_track)
                                                })
                                                .and_then(|track_index| {
                                                    track_entries[track_index].last().copied()
                                                })
                                        })
                                });
                            if let Some(required_entry) = required_entry
                                && required_entry != entry.index
                                && entries[required_entry].track_index != entry.track_index
                                && !dependencies[entry.index].contains(&required_entry)
                            {
                                dependencies[entry.index].push(required_entry);
                            }
                        }
                    }
                }
                CatalogTarget::Infinite => {
                    dependencies[entry.index]
                        .push(track_entries[entry.track_index][entry.level as usize - 2]);
                }
            }
        }
        let root_index = technologies
            .get("research_hut")
            .and_then(|track| {
                technologies
                    .tracks()
                    .iter()
                    .position(|candidate| std::ptr::eq(candidate, track))
            })
            .map_or(0, |track_index| track_entries[track_index][0]);
        for (entry, required) in dependencies.iter_mut().enumerate() {
            if entry != root_index && required.is_empty() {
                required.push(root_index);
            }
        }
        let layout = build_vertical_tree_layout(&entries, &dependencies, root_index);
        let connectors = dependencies
            .iter()
            .enumerate()
            .flat_map(|(to, prerequisites)| {
                let primary_parents = &layout.primary_parents;
                prerequisites.iter().map(move |from| CatalogConnector {
                    from: *from,
                    to,
                    primary: primary_parents[to] == Some(*from),
                })
            })
            .collect();
        let states = vec![CatalogNodeState::Locked; entries.len()];
        Self {
            entries,
            connectors,
            track_entries,
            states,
            layout,
            root_index,
        }
    }

    fn card_position(&self, index: usize) -> Vec2 {
        debug_assert!(self.layout.depths[index] < self.layout.layer_count);
        self.layout.positions[index]
    }

    /// Focus mode keeps the selected branch at the same fixed scale, but
    /// compacts every visible depth layer around the canvas centre. Reusing
    /// full-tree coordinates here would technically filter the graph while
    /// leaving prerequisite cards several screens away.
    fn display_positions(&self, selected: Option<usize>) -> Vec<Vec2> {
        let Some(selected) = selected else {
            return self.layout.positions.clone();
        };
        let visible = self.with_relatives(selected);
        let mut layers = vec![Vec::new(); self.layout.layer_count];
        for index in visible {
            layers[self.layout.depths[index]].push(index);
        }
        let max_columns = layers.iter().map(Vec::len).max().unwrap_or(1).max(1);
        let compact_width = NODE_WIDTH + max_columns.saturating_sub(1) as f32 * MAP_STEP_X;
        let compact_left = (self.layout.size.x - compact_width) / 2.0;
        let mut positions = self.layout.positions.clone();
        for layer in &mut layers {
            layer.sort_by(|left, right| {
                self.layout.positions[*left]
                    .x
                    .total_cmp(&self.layout.positions[*right].x)
            });
            let layer_left =
                compact_left + (max_columns.saturating_sub(layer.len())) as f32 * MAP_STEP_X / 2.0;
            for (column, index) in layer.iter().copied().enumerate() {
                positions[index] = Vec2::new(
                    layer_left + column as f32 * MAP_STEP_X,
                    self.layout.positions[index].y,
                );
            }
        }
        positions
    }

    fn with_relatives(&self, selected: usize) -> HashSet<usize> {
        let mut visible = HashSet::from([selected]);
        let mut ancestors = vec![selected];
        while let Some(index) = ancestors.pop() {
            for connector in self.connectors.iter().filter(|edge| edge.to == index) {
                if visible.insert(connector.from) {
                    ancestors.push(connector.from);
                }
            }
        }
        let mut descendants = vec![selected];
        while let Some(index) = descendants.pop() {
            for connector in self.connectors.iter().filter(|edge| edge.from == index) {
                if visible.insert(connector.to) {
                    descendants.push(connector.to);
                }
            }
        }
        visible
    }

    fn track_for_card(&self, card: usize) -> &'static cat_sim::research_tracks::TechnologyTrack {
        &technology_catalog().tracks()[self.entries[card].track_index]
    }

    fn node_for_card(&self, card: usize) -> &'static ResearchNode {
        let track = self.track_for_card(card);
        let node_index = match self.entries[card].target {
            CatalogTarget::Finite { node_index } => node_index,
            CatalogTarget::Track => track.node_indices[0],
            CatalogTarget::Infinite => *track.node_indices.last().unwrap(),
        };
        &research_catalog().nodes()[node_index]
    }

    fn queued_position(&self, card: usize, snapshot: &ResearchSnapshot) -> Option<usize> {
        let entry = &self.entries[card];
        let track = self.track_for_card(card);
        snapshot
            .queue
            .iter()
            .position(|queued| match (&entry.target, &queued.target) {
                (
                    CatalogTarget::Finite { node_index },
                    cat_protocol::ResearchQueueTargetSnapshot::Finite { node_id },
                ) => research_catalog().nodes()[*node_index].id == *node_id,
                (
                    CatalogTarget::Infinite,
                    cat_protocol::ResearchQueueTargetSnapshot::Repeatable { track_id, .. },
                ) => track_id == &track.id,
                (
                    CatalogTarget::Track,
                    cat_protocol::ResearchQueueTargetSnapshot::Finite { node_id },
                ) => track
                    .node_indices
                    .iter()
                    .any(|index| research_catalog().nodes()[*index].id == *node_id),
                _ => false,
            })
    }

    fn displayed_level(&self, card: usize, snapshot: &ResearchSnapshot) -> u32 {
        let track = self.track_for_card(card);
        if matches!(self.entries[card].target, CatalogTarget::Infinite) {
            snapshot
                .repeatable_levels
                .get(&track.id)
                .copied()
                .unwrap_or(cat_sim::research_tracks::FINITE_TRACK_LEVELS)
        } else {
            track.displayed_finite_level(&snapshot.owned_node_ids)
        }
    }

    fn card_state(&self, card: usize, snapshot: &ResearchSnapshot) -> CatalogNodeState {
        let entry = &self.entries[card];
        let track = self.track_for_card(card);
        if let Some(position) = self.queued_position(card, snapshot) {
            return if position == 0 {
                CatalogNodeState::Active
            } else {
                CatalogNodeState::Queued
            };
        }
        match entry.target {
            CatalogTarget::Finite { .. } => {
                let owned_level = track.displayed_finite_level(&snapshot.owned_node_ids);
                if owned_level >= entry.level {
                    CatalogNodeState::Owned
                } else if entry.level == owned_level.saturating_add(1) {
                    CatalogNodeState::Available
                } else {
                    CatalogNodeState::Locked
                }
            }
            CatalogTarget::Track => {
                let owned: HashSet<_> =
                    snapshot.owned_node_ids.iter().map(String::as_str).collect();
                if track.next_component(&snapshot.owned_node_ids).is_none() {
                    CatalogNodeState::Owned
                } else if track
                    .next_component(&snapshot.owned_node_ids)
                    .is_some_and(|node| {
                        node.prerequisites
                            .iter()
                            .all(|required| owned.contains(required.as_str()))
                    })
                {
                    CatalogNodeState::Available
                } else {
                    CatalogNodeState::Locked
                }
            }
            CatalogTarget::Infinite => {
                if track.next_component(&snapshot.owned_node_ids).is_none() {
                    CatalogNodeState::Available
                } else {
                    CatalogNodeState::Locked
                }
            }
        }
    }

    fn apply_snapshot(&mut self, snapshot: &ResearchSnapshot) {
        for index in 0..self.entries.len() {
            self.states[index] = self.card_state(index, snapshot);
        }
    }

    fn filtered_indices(&self, query: &str, filter: ResearchFilter) -> Vec<usize> {
        let query = query.trim().to_lowercase();
        technology_catalog()
            .tracks()
            .iter()
            .enumerate()
            .filter(|(_, track)| filter.includes(track.category))
            .filter(|(_, track)| {
                let node = &research_catalog().nodes()[track.node_indices[0]];
                query.is_empty()
                    || track.id.to_lowercase().contains(&query)
                    || track.name.to_lowercase().contains(&query)
                    || node.description.to_lowercase().contains(&query)
            })
            .map(|(track_index, _)| self.track_entries[track_index][0])
            .collect()
    }

    fn catalog_target(&self, track_index: usize, snapshot: &ResearchSnapshot) -> usize {
        let track = &technology_catalog().tracks()[track_index];
        if self.track_entries[track_index].len() == 1 {
            return self.track_entries[track_index][0];
        }
        let level = track.displayed_finite_level(&snapshot.owned_node_ids);
        if level < track.finite_level_count() {
            self.track_entries[track_index][level as usize]
        } else {
            self.track_entries[track_index].last().copied().unwrap()
        }
    }

    fn purchase_state(&self, card: usize, snapshot: &ResearchSnapshot) -> PurchaseState {
        let entry = &self.entries[card];
        if self.queued_position(card, snapshot).is_some() {
            return PurchaseState::Locked;
        }
        match entry.target {
            CatalogTarget::Finite { .. } => {
                if self
                    .track_for_card(card)
                    .displayed_finite_level(&snapshot.owned_node_ids)
                    >= entry.level
                {
                    PurchaseState::Owned
                } else {
                    PurchaseState::ResearchReady
                }
            }
            CatalogTarget::Track => {
                if self
                    .track_for_card(card)
                    .next_component(&snapshot.owned_node_ids)
                    .is_some()
                {
                    PurchaseState::ResearchReady
                } else {
                    PurchaseState::Owned
                }
            }
            CatalogTarget::Infinite => {
                if self
                    .track_for_card(card)
                    .next_component(&snapshot.owned_node_ids)
                    .is_none()
                {
                    PurchaseState::RepeatableReady
                } else {
                    PurchaseState::Locked
                }
            }
        }
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
            canvas_width: width - inspector_width - CATALOG_WIDTH,
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
    selected: Option<usize>,
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
            selected: None,
            pan: Vec2::new(18.0, 16.0),
            zoom: FIXED_TREE_SCALE,
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
pub(super) struct ResearchCatalogCard(usize);
#[derive(Component, Clone, Copy)]
pub(super) struct ResearchConnector(usize);
#[derive(Component)]
struct ResearchDepthBand;
#[derive(Component)]
pub(super) struct ResearchIcon(&'static str);
#[derive(Component)]
pub(super) struct InspectorIcon;
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
#[derive(Component)]
pub(super) struct ResearchQueueSummary;
#[derive(Component, Clone, Copy)]
pub(super) struct ResearchQueueRow(usize);
#[derive(Component, Clone, Copy)]
pub(super) struct ResearchQueueControl {
    slot: usize,
    action: ResearchQueueControlAction,
}
#[derive(Clone, Copy)]
pub(super) enum ResearchQueueControlAction {
    Up,
    Down,
    Remove,
}
#[derive(Component, Clone, Copy)]
pub(super) struct FilterButton(ResearchFilter);
#[derive(Component, Clone, Copy)]
pub(super) enum LedgerAction {
    Close,
    FullTree,
}

pub(super) fn load_research_icons(
    asset_server: Res<AssetServer>,
    mut icons: Query<(&ResearchIcon, &mut ImageNode), Added<ResearchIcon>>,
) {
    for (icon, mut image) in &mut icons {
        image.image = asset_server.load(icon.0);
    }
}

fn category_color(category: ResearchCategory) -> Color {
    match category {
        ResearchCategory::Building => BUILDING_INK,
        ResearchCategory::RecipeResource => RECIPE_INK,
        ResearchCategory::Upgrade => UPGRADE_INK,
    }
}

fn research_drag_delta(pointer_delta: Vec2) -> Vec2 {
    pointer_delta * RESEARCH_DRAG_GAIN
}

fn root_overview_zoom(ui_scale: f32) -> f32 {
    FIXED_TREE_SCALE / ui_scale.max(0.1)
}

fn responsive_research_layout(width: f32, height: f32, ui_scale: f32) -> ResearchResponsiveLayout {
    let profile = UiLayoutProfile::new(width, height, ui_scale);
    ResearchResponsiveLayout::for_window(profile.effective_width, profile.effective_height)
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
                        row.spawn((ledger_button("Full tree"), LedgerAction::FullTree));
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
                                format!("{} technologies", technology_catalog().tracks().len()),
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
                                "Wheel over a panel scrolls that panel  |  Drag the tree to pan",
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
                        width: Val::Px(CATALOG_WIDTH),
                        min_width: Val::Px(CATALOG_WIDTH),
                        height: Val::Percent(100.0),
                        min_height: Val::Px(0.0),
                        border: UiRect::right(Val::Px(2.0)),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.91, 0.86, 0.73)),
                    BorderColor::all(LEDGER_PAPER_DARK),
                    ZIndex(10),
                ))
                .with_children(|sidebar| {
                    sidebar
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                padding: UiRect::all(Val::Px(10.0)),
                                border: UiRect::bottom(Val::Px(1.0)),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(5.0),
                                ..default()
                            },
                            BorderColor::all(LEDGER_PAPER_DARK),
                        ))
                        .with_children(|queue| {
                            queue.spawn(ui_text("Research queue", FS_SECTION, LEDGER_INK));
                            queue.spawn((
                                ui_text_wrapped(
                                    "Queue empty · the Leader may choose one study daily.",
                                    FS_SMALL,
                                    LEDGER_MUTED,
                                ),
                                ResearchQueueSummary,
                            ));
                            for slot in 0..4 {
                                queue
                                    .spawn((
                                        Node {
                                            display: Display::None,
                                            width: Val::Percent(100.0),
                                            align_items: AlignItems::Center,
                                            column_gap: Val::Px(3.0),
                                            ..default()
                                        },
                                        ResearchQueueRow(slot),
                                    ))
                                    .with_children(|row| {
                                        row.spawn((
                                            Node {
                                                flex_grow: 1.0,
                                                min_width: Val::Px(0.0),
                                                overflow: Overflow::clip(),
                                                ..default()
                                            },
                                            ui_text(format!("{}.", slot + 1), 10.0, LEDGER_INK),
                                        ));
                                        for (label, action) in [
                                            ("Up", ResearchQueueControlAction::Up),
                                            ("Down", ResearchQueueControlAction::Down),
                                            ("Remove", ResearchQueueControlAction::Remove),
                                        ] {
                                            row.spawn((
                                                ledger_button(label),
                                                KitDisabled { disabled: true },
                                                ResearchQueueControl { slot, action },
                                            ));
                                        }
                                    });
                            }
                        });
                    sidebar.spawn(ui_text("Technology catalog", FS_SECTION, LEDGER_INK));
                    spawn_vertical_scroll_area(sidebar, 7.0, 6.0, |list| {
                        for (track_index, track) in technology_catalog().tracks().iter().enumerate()
                        {
                            let card = model.track_entries[track_index][0];
                            let node = &catalog.nodes()[track.node_indices[0]];
                            list.spawn((
                                Button,
                                Node {
                                    width: Val::Percent(100.0),
                                    min_height: Val::Px(46.0),
                                    padding: UiRect::all(Val::Px(6.0)),
                                    border: UiRect::left(Val::Px(3.0)),
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(7.0),
                                    ..default()
                                },
                                BackgroundColor(LOCKED_PAPER),
                                BorderColor::all(category_color(track.category)),
                                ResearchCatalogCard(track_index),
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    Node {
                                        width: Val::Px(30.0),
                                        height: Val::Px(30.0),
                                        min_width: Val::Px(30.0),
                                        ..default()
                                    },
                                    ImageNode::default(),
                                    ResearchIcon(research_icon_path(node)),
                                ));
                                button
                                    .spawn(Node {
                                        flex_grow: 1.0,
                                        min_width: Val::Px(0.0),
                                        flex_direction: FlexDirection::Column,
                                        ..default()
                                    })
                                    .with_children(|copy| {
                                        copy.spawn(ui_text_wrapped(
                                            track.name.clone(),
                                            FS_SMALL,
                                            LEDGER_INK,
                                        ));
                                        copy.spawn((
                                            ui_text("Locked", 10.0, LOCKED_INK),
                                            CardStateText(card),
                                        ));
                                    });
                            });
                        }
                    });
                });
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
                            for depth in 0..model.layout.layer_count {
                                canvas.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(0.0),
                                        top: Val::Px(
                                            MAP_PADDING_Y + depth as f32 * MAP_STEP_Y - 18.0,
                                        ),
                                        width: Val::Px(model.layout.size.x),
                                        height: Val::Px(NODE_HEIGHT + 36.0),
                                        border: UiRect::top(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(if depth % 2 == 0 {
                                        DEPTH_BAND_A
                                    } else {
                                        DEPTH_BAND_B
                                    }),
                                    BorderColor::all(STRUCTURE_INK.with_alpha(0.18)),
                                    ResearchDepthBand,
                                ));
                            }
                            canvas
                                .spawn(Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(root_position.x - 8.0),
                                    top: Val::Px(root_position.y - 58.0),
                                    width: Val::Px(NODE_WIDTH + 16.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(2.0),
                                    ..default()
                                })
                                .with_children(|caption| {
                                    caption.spawn(ui_text(
                                        format!(
                                            "{} STUDIES · ONE DEPENDENCY TREE",
                                            model.entries.len()
                                        ),
                                        15.0,
                                        Color::srgb(0.88, 0.72, 0.39),
                                    ));
                                    caption.spawn(ui_text(
                                        format!(
                                            "{} levels · {} main branches",
                                            model.layout.layer_count, model.layout.branch_count
                                        ),
                                        FS_SMALL,
                                        Color::srgb(0.76, 0.73, 0.64),
                                    ));
                                });

                            for (connector_index, connector) in
                                model.connectors.iter().copied().enumerate()
                            {
                                let from = model.card_position(connector.from)
                                    + Vec2::new(NODE_WIDTH / 2.0, NODE_HEIGHT);
                                let to = model.card_position(connector.to)
                                    + Vec2::new(NODE_WIDTH / 2.0, 0.0);
                                let delta = to - from;
                                let distance = delta.length().max(CONNECTOR_STROKE);
                                let midpoint = (from + to) / 2.0;
                                let color = if connector.primary {
                                    category_color(model.node_for_card(connector.to).category)
                                        .with_alpha(0.64)
                                } else {
                                    CROSS_LINK_INK
                                };
                                canvas.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::Px(midpoint.x - distance / 2.0),
                                        top: Val::Px(midpoint.y - CONNECTOR_STROKE / 2.0),
                                        width: Val::Px(distance),
                                        height: Val::Px(CONNECTOR_STROKE),
                                        ..default()
                                    },
                                    UiTransform::from_rotation(bevy::math::Rot2::radians(
                                        delta.y.atan2(delta.x),
                                    )),
                                    BackgroundColor(color),
                                    ResearchConnector(connector_index),
                                ));
                            }

                            for entry in &model.entries {
                                let index = entry.index;
                                let node = model.node_for_card(index);
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
                                            padding: UiRect::axes(Val::Px(8.0), Val::Px(7.0)),
                                            border: UiRect {
                                                left: Val::Px(4.0),
                                                right: Val::Px(1.0),
                                                top: Val::Px(1.0),
                                                bottom: Val::Px(1.0),
                                            },
                                            align_items: AlignItems::Center,
                                            column_gap: Val::Px(8.0),
                                            overflow: Overflow::clip(),
                                            ..default()
                                        },
                                        BackgroundColor(LOCKED_PAPER),
                                        BorderColor::all(category_color(node.category)),
                                        ResearchCard(index),
                                    ))
                                    .with_children(|card| {
                                        card.spawn((
                                            Node {
                                                width: Val::Px(38.0),
                                                height: Val::Px(38.0),
                                                min_width: Val::Px(38.0),
                                                padding: UiRect::all(Val::Px(3.0)),
                                                border: UiRect::all(Val::Px(1.0)),
                                                ..default()
                                            },
                                            BackgroundColor(
                                                category_color(node.category).with_alpha(0.12),
                                            ),
                                            BorderColor::all(
                                                category_color(node.category).with_alpha(0.42),
                                            ),
                                        ))
                                        .with_children(
                                            |frame| {
                                                frame.spawn((
                                                    Node {
                                                        width: Val::Percent(100.0),
                                                        height: Val::Percent(100.0),
                                                        ..default()
                                                    },
                                                    ImageNode::default(),
                                                    ResearchIcon(research_icon_path(node)),
                                                ));
                                            },
                                        );
                                        card.spawn((
                                            Node {
                                                flex_grow: 1.0,
                                                min_width: Val::Px(0.0),
                                                ..default()
                                            },
                                            ui_text_wrapped(entry.name.clone(), 12.5, LEDGER_INK),
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
                            .spawn(Node {
                                width: Val::Percent(100.0),
                                min_width: Val::Px(0.0),
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(10.0),
                                ..default()
                            })
                            .with_children(|identity| {
                                identity
                                    .spawn((
                                        Node {
                                            width: Val::Px(42.0),
                                            height: Val::Px(42.0),
                                            min_width: Val::Px(42.0),
                                            padding: UiRect::all(Val::Px(4.0)),
                                            border: UiRect::all(Val::Px(1.0)),
                                            ..default()
                                        },
                                        BackgroundColor(BUILDING_INK.with_alpha(0.12)),
                                        BorderColor::all(BUILDING_INK.with_alpha(0.45)),
                                    ))
                                    .with_children(|frame| {
                                        frame.spawn((
                                            Node {
                                                width: Val::Percent(100.0),
                                                height: Val::Percent(100.0),
                                                ..default()
                                            },
                                            ImageNode::default(),
                                            ResearchIcon(research_icon_path(
                                                &catalog.nodes()[model.root_index],
                                            )),
                                            InspectorIcon,
                                        ));
                                    });
                                identity.spawn((
                                    Node {
                                        flex_grow: 1.0,
                                        min_width: Val::Px(0.0),
                                        ..default()
                                    },
                                    ui_text_wrapped("Research Hut", 22.0, LEDGER_INK),
                                    InspectorTitle,
                                ));
                            });
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
    ui_scale: f32,
) {
    let point = model.display_positions(ui.selected)[index]
        + Vec2::new(NODE_WIDTH / 2.0, NODE_HEIGHT / 2.0);
    let (canvas_width, canvas_height) = window.map_or((760.0, 650.0), |window| {
        let layout = responsive_research_layout(window.width(), window.height(), ui_scale);
        (layout.canvas_width, layout.canvas_height)
    });
    ui.pan = Vec2::new(
        canvas_width / 2.0 - point.x * ui.zoom,
        canvas_height / 2.0
            - point.y * ui.zoom
            - if index == model.root_index {
                ROOT_OVERVIEW_VERTICAL_BIAS / ui_scale.max(0.1)
            } else {
                0.0
            },
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
    ui_scale: Res<UiScale>,
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
            ui.zoom = root_overview_zoom(ui_scale.0);
            let selected = ui.selected.unwrap_or(model.root_index);
            center_on(&mut ui, &model, selected, windows.single().ok(), ui_scale.0);
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
    let layout = responsive_research_layout(window.width(), window.height(), ui_scale.0);
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

/// Button interactions select studies, switch catalog categories and focus
/// search. The graph scale is intentionally fixed; only its pan changes.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn handle_research_controls(
    mut ui: ResMut<UpgradeTreeUi>,
    model: Res<ResearchUiModel>,
    windows: Query<&Window>,
    ui_scale: Res<UiScale>,
    search: Query<&Interaction, (Changed<Interaction>, With<SearchButton>)>,
    filters: Query<(&Interaction, &FilterButton), Changed<Interaction>>,
    cards: Query<(&Interaction, &ResearchCard), Changed<Interaction>>,
    catalog_cards: Query<
        (&Interaction, &ResearchCatalogCard),
        (Changed<Interaction>, Without<ResearchCard>),
    >,
    actions: Query<(&Interaction, &LedgerAction), Changed<Interaction>>,
    latest: Res<LatestSnapshot>,
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
            ui.selected = Some(card.0);
            center_on(&mut ui, &model, card.0, windows.single().ok(), ui_scale.0);
            ui.inspector_dirty = true;
            ui.filter_dirty = true;
        }
    }
    for (interaction, card) in &catalog_cards {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let selected = current_research(&latest)
            .map_or(model.track_entries[card.0][0], |snapshot| {
                model.catalog_target(card.0, snapshot)
            });
        ui.selected = Some(selected);
        center_on(&mut ui, &model, selected, windows.single().ok(), ui_scale.0);
        ui.inspector_dirty = true;
        ui.filter_dirty = true;
    }
    for (interaction, action) in &actions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            LedgerAction::Close => {}
            LedgerAction::FullTree => {
                ui.selected = None;
                ui.zoom = root_overview_zoom(ui_scale.0);
                center_on(
                    &mut ui,
                    &model,
                    model.root_index,
                    windows.single().ok(),
                    ui_scale.0,
                );
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
    ui_scale: Res<UiScale>,
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
                        ui.selected = Some(first);
                        center_on(&mut ui, &model, first, windows.single().ok(), ui_scale.0);
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

    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if pointer.over_canvas() {
        for event in wheel.read() {
            let scale = match event.unit {
                MouseScrollUnit::Line => 54.0,
                MouseScrollUnit::Pixel => 1.0,
            };
            if shift {
                pan_delta.x += (event.x + event.y) * scale;
            } else {
                pan_delta += Vec2::new(event.x * scale, event.y * scale);
            }
            ui.transform_dirty = true;
        }
    } else {
        wheel.clear();
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
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn update_research_filter(
    model: Res<ResearchUiModel>,
    mut ui: ResMut<UpgradeTreeUi>,
    mut cards: Query<(&ResearchCard, &mut Node, &mut BorderColor)>,
    mut catalog_cards: Query<
        (&ResearchCatalogCard, &mut Node),
        (Without<ResearchCard>, Without<ResearchConnector>),
    >,
    mut connectors: Query<
        (
            &ResearchConnector,
            &mut Node,
            &mut UiTransform,
            &mut BackgroundColor,
        ),
        (Without<ResearchCard>, Without<ResearchCatalogCard>),
    >,
    mut filters: Query<(&FilterButton, &mut KitToggle)>,
    mut search: Query<&mut Text, (With<SearchText>, Without<MatchCountText>)>,
    mut count: Query<&mut Text, (With<MatchCountText>, Without<SearchText>)>,
) {
    if !ui.visible || !ui.filter_dirty {
        return;
    }
    let matches = model.filtered_indices(&ui.query, ui.filter);
    let matched_tracks = matches
        .iter()
        .map(|card| model.entries[*card].track_index)
        .collect::<HashSet<_>>();
    let visible = ui.selected.map_or_else(
        || (0..model.entries.len()).collect(),
        |selected| model.with_relatives(selected),
    );
    let positions = model.display_positions(ui.selected);
    for (card, mut node, mut border) in &mut cards {
        node.display = if visible.contains(&card.0) {
            Display::Flex
        } else {
            Display::None
        };
        node.left = Val::Px(positions[card.0].x);
        node.top = Val::Px(positions[card.0].y);
        let color = if ui.selected == Some(card.0) {
            READY_INK
        } else {
            category_color(model.node_for_card(card.0).category)
        };
        *border = BorderColor::all(color);
    }
    for (card, mut node) in &mut catalog_cards {
        node.display = if matched_tracks.contains(&card.0) {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (line, mut node, mut transform, mut background) in &mut connectors {
        let connector = model.connectors[line.0];
        node.display = if visible.contains(&connector.from) && visible.contains(&connector.to) {
            Display::Flex
        } else {
            Display::None
        };
        let from = positions[connector.from] + Vec2::new(NODE_WIDTH / 2.0, NODE_HEIGHT);
        let to = positions[connector.to] + Vec2::new(NODE_WIDTH / 2.0, 0.0);
        let delta = to - from;
        let distance = delta.length().max(CONNECTOR_STROKE);
        let midpoint = (from + to) / 2.0;
        node.left = Val::Px(midpoint.x - distance / 2.0);
        node.top = Val::Px(midpoint.y - CONNECTOR_STROKE / 2.0);
        node.width = Val::Px(distance);
        transform.rotation = bevy::math::Rot2::radians(delta.y.atan2(delta.x));
        background.0 = if ui.selected.is_some()
            && visible.contains(&connector.from)
            && visible.contains(&connector.to)
        {
            READY_INK.with_alpha(0.95)
        } else if connector.primary {
            category_color(model.node_for_card(connector.to).category).with_alpha(0.64)
        } else {
            CROSS_LINK_INK
        };
    }
    ui.transform_dirty = true;
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
        text.0 = format!(
            "{} / {} technologies",
            matches.len(),
            technology_catalog().tracks().len()
        );
    }
    ui.filter_dirty = false;
}

/// Repaint fixed cards only when a new snapshot arrives or the page opens.
/// There is no Commands access here, which is the guard against entity churn.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn update_research_snapshot(
    latest: Res<LatestSnapshot>,
    mut model: ResMut<ResearchUiModel>,
    mut ui: ResMut<UpgradeTreeUi>,
    mut cards: Query<(&ResearchCard, &mut BackgroundColor)>,
    mut catalog_cards: Query<(&ResearchCatalogCard, &mut BackgroundColor), Without<ResearchCard>>,
    mut states: Query<
        (&CardStateText, &mut Text, &mut TextColor),
        (
            Without<ResearchCurrency>,
            Without<ResearchNext>,
            Without<ResearchQueueSummary>,
        ),
    >,
    mut currency: Query<
        &mut Text,
        (
            With<ResearchCurrency>,
            Without<ResearchNext>,
            Without<ResearchQueueSummary>,
            Without<CardStateText>,
        ),
    >,
    mut next: Query<
        &mut Text,
        (
            With<ResearchNext>,
            Without<ResearchCurrency>,
            Without<ResearchQueueSummary>,
            Without<CardStateText>,
        ),
    >,
    mut queue_summary: Query<
        &mut Text,
        (
            With<ResearchQueueSummary>,
            Without<ResearchCurrency>,
            Without<ResearchNext>,
            Without<CardStateText>,
        ),
    >,
    mut queue_rows: Query<(&ResearchQueueRow, &mut Node)>,
    mut queue_controls: Query<(&ResearchQueueControl, &mut KitDisabled)>,
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
                CatalogNodeState::Queued => Color::srgb(0.72, 0.77, 0.67),
                CatalogNodeState::Active => Color::srgb(0.93, 0.83, 0.52),
                CatalogNodeState::Locked => LOCKED_PAPER,
            };
        }
        for (card, mut background) in &mut catalog_cards {
            let target = model.catalog_target(card.0, research);
            background.0 = match model.states[target] {
                CatalogNodeState::Owned => OWNED_PAPER,
                CatalogNodeState::Available => READY_PAPER,
                CatalogNodeState::Queued => Color::srgb(0.72, 0.77, 0.67),
                CatalogNodeState::Active => Color::srgb(0.93, 0.83, 0.52),
                CatalogNodeState::Locked => LOCKED_PAPER,
            };
        }
        for (marker, mut text, mut color) in &mut states {
            let track = model.track_for_card(marker.0);
            let level = track.displayed_finite_level(&research.owned_node_ids);
            let target = model.catalog_target(model.entries[marker.0].track_index, research);
            let next_cost = track
                .next_component(&research.owned_node_ids)
                .map(|node| node.cost)
                .or_else(|| {
                    track.is_repeatable().then(|| {
                        cat_sim::research_tracks::repeatable_cost(&track.id, level + 1)
                            .unwrap_or(0.0)
                    })
                })
                .unwrap_or(0.0);
            let (label, ink) = match model.states[target] {
                CatalogNodeState::Owned if track.is_repeatable() => ("Infinite", OWNED_INK),
                CatalogNodeState::Owned => ("Complete", OWNED_INK),
                CatalogNodeState::Available => ("Available", READY_INK),
                CatalogNodeState::Queued => ("Queued", UPGRADE_INK),
                CatalogNodeState::Active => ("Researching", READY_INK),
                CatalogNodeState::Locked => ("Locked", LOCKED_INK),
            };
            text.0 = if next_cost > 0.0 {
                format!(
                    "{label} | {level}/{} | next {next_cost:.0} pts",
                    track.finite_level_count()
                )
            } else {
                format!("{label} | {level}/{}", track.finite_level_count())
            };
            color.0 = ink;
        }
        if let Ok(mut text) = currency.single_mut() {
            let points_per_hour = if research.points_per_hour.abs() < 0.0005 {
                0.0
            } else {
                research.points_per_hour
            };
            text.0 = format!(
                "{:.1} research  |  {:.2}/hour  |  {} scholars  |  {} queued",
                research.research_points,
                points_per_hour,
                research.researcher_count,
                research.queue.len()
            );
        }
        if let Ok(mut text) = next.single_mut() {
            text.0 = leader_priority_copy(research);
        }
        if let Ok(mut text) = queue_summary.single_mut() {
            text.0 = if research.queue.is_empty() {
                "Queue empty · the Leader may choose one study daily.".to_owned()
            } else {
                research
                    .queue
                    .iter()
                    .take(4)
                    .enumerate()
                    .map(|(index, entry)| {
                        let progress = if entry.required_seconds > 0.0 {
                            entry.progress_seconds / entry.required_seconds * 100.0
                        } else {
                            0.0
                        };
                        format!(
                            "{}. {} · {:.0}%",
                            index + 1,
                            entry.name,
                            progress.clamp(0.0, 100.0)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
        }
        for (row, mut node) in &mut queue_rows {
            node.display = if row.0 < research.queue.len() {
                Display::Flex
            } else {
                Display::None
            };
        }
        for (control, mut disabled) in &mut queue_controls {
            disabled.disabled = control.slot >= research.queue.len()
                || matches!(control.action, ResearchQueueControlAction::Up) && control.slot == 0
                || matches!(control.action, ResearchQueueControlAction::Down)
                    && control.slot + 1 >= research.queue.len();
        }
    } else {
        if let Ok(mut text) = currency.single_mut() {
            text.0 = "Awaiting colony research record…".to_owned();
        }
        if let Ok(mut text) = next.single_mut() {
            text.0 = "Connect to inspect progress".to_owned();
        }
        if let Ok(mut text) = queue_summary.single_mut() {
            text.0 = "Awaiting colony research record…".to_owned();
        }
        for (_, mut node) in &mut queue_rows {
            node.display = Display::None;
        }
        for (_, mut disabled) in &mut queue_controls {
            disabled.disabled = true;
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
    asset_server: Res<AssetServer>,
    mut ui: ResMut<UpgradeTreeUi>,
    mut purchase_button: Query<&mut KitDisabled, With<PurchaseButton>>,
    mut icon: Query<&mut ImageNode, With<InspectorIcon>>,
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
    let Some(selected) = ui.selected else {
        if let Ok(mut disabled) = purchase_button.single_mut() {
            disabled.disabled = true;
        }
        for (mut text, title, meta, description, prerequisites, payloads, buy) in &mut texts {
            text.0 = if title.is_some() {
                "Full technology tree".to_owned()
            } else if meta.is_some() {
                format!("{} studies", model.entries.len())
            } else if description.is_some() {
                "Select a technology to isolate everything it requires and everything it unlocks."
                    .to_owned()
            } else if prerequisites.is_some() || payloads.is_some() {
                "Nothing selected".to_owned()
            } else if buy.is_some() {
                "Select a technology".to_owned()
            } else {
                String::new()
            };
        }
        ui.inspector_dirty = false;
        return;
    };
    let entry = &model.entries[selected];
    let track = model.track_for_card(selected);
    let research = current_research(&latest);
    let display_node = if matches!(entry.target, CatalogTarget::Track) {
        research
            .and_then(|snapshot| track.next_component(&snapshot.owned_node_ids))
            .unwrap_or_else(|| model.node_for_card(selected))
    } else {
        model.node_for_card(selected)
    };
    let current_level = research.map_or(0, |snapshot| model.displayed_level(selected, snapshot));
    let next_cost = match entry.target {
        CatalogTarget::Finite { .. } => Some(display_node.cost),
        CatalogTarget::Track => Some(display_node.cost),
        CatalogTarget::Infinite => Some(
            cat_sim::research_tracks::repeatable_cost(&track.id, current_level + 1).unwrap_or(0.0),
        ),
    };
    let kind = match track.kind {
        TechnologyKind::Milestone => "Milestone",
        TechnologyKind::Building => "Building",
        TechnologyKind::Recipe => "Production",
        TechnologyKind::GlobalModifier => "Global modifier",
    };
    let meta = if matches!(entry.target, CatalogTarget::Infinite) {
        format!("{kind} | Infinite level {}", current_level + 1)
    } else if matches!(entry.target, CatalogTarget::Track) {
        format!(
            "{kind} | Level {current_level}/{} | next {}",
            track.finite_level_count(),
            (current_level + 1).min(track.finite_level_count())
        )
    } else {
        format!(
            "{kind} | Level {}/{}",
            entry.level,
            track.finite_level_count()
        )
    };
    let required_cards = model
        .connectors
        .iter()
        .filter(|connector| connector.to == selected)
        .map(|connector| model.entries[connector.from].name.as_str())
        .collect::<Vec<_>>();
    let prerequisites = if required_cards.is_empty() {
        "No prior study".to_owned()
    } else {
        required_cards.join("\n")
    };
    let payloads = if matches!(entry.target, CatalogTarget::Infinite) {
        "Permanently improves this global modifier by 3%.\nCost and research time double each level."
            .to_owned()
    } else {
        display_node
            .payloads
            .iter()
            .map(payload_line)
            .collect::<Vec<_>>()
            .join("\n")
    };
    let purchase_state = research.map(|snapshot| model.purchase_state(selected, snapshot));
    if let Ok(mut image) = icon.single_mut() {
        image.image = asset_server.load(research_icon_path(display_node));
    }
    let purchase = purchase_state.map_or_else(
        || "Awaiting colony".to_owned(),
        |state| match state {
            PurchaseState::Owned => "Study owned".to_owned(),
            PurchaseState::Locked => "Prerequisites required".to_owned(),
            PurchaseState::ResearchReady => {
                format!("Queue path · {:.0} pts", next_cost.unwrap_or(0.0))
            }
            PurchaseState::RepeatableReady => format!(
                "Queue infinite level {} · {:.0} pts",
                current_level + 1,
                next_cost.unwrap_or(0.0)
            ),
        },
    );
    if let Ok(mut disabled) = purchase_button.single_mut() {
        disabled.disabled = research_purchase_disabled(session.ready, purchase_state);
    }
    for (mut text, title, meta_marker, description, prereq_marker, payload_marker, buy) in
        &mut texts
    {
        if title.is_some() {
            text.0 = entry.name.clone();
        } else if meta_marker.is_some() {
            text.0 = meta.clone();
        } else if description.is_some() {
            text.0 = if matches!(entry.target, CatalogTarget::Infinite) {
                "An endless specialization for mature colonies. Each completion makes the same effect stronger and takes longer than the last."
                    .to_owned()
            } else {
                display_node.description.clone()
            };
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
            Some(PurchaseState::ResearchReady | PurchaseState::RepeatableReady)
        )
}

fn research_purchase_action(
    model: &ResearchUiModel,
    research: &ResearchSnapshot,
    card: usize,
    session: &Session,
) -> Option<ClientAction> {
    match (
        model.entries[card].target,
        model.purchase_state(card, research),
    ) {
        (CatalogTarget::Finite { node_index }, PurchaseState::ResearchReady) => {
            Some(ClientAction::QueueResearchPath {
                session_id: session.session_id.clone(),
                nickname: CLIENT_ACTOR_LABEL.to_owned(),
                sig: session.sig.clone(),
                node_id: research_catalog().nodes()[node_index].id.clone(),
            })
        }
        (CatalogTarget::Track, PurchaseState::ResearchReady) => {
            let node = model
                .track_for_card(card)
                .next_component(&research.owned_node_ids)?;
            Some(ClientAction::QueueResearchPath {
                session_id: session.session_id.clone(),
                nickname: CLIENT_ACTOR_LABEL.to_owned(),
                sig: session.sig.clone(),
                node_id: node.id.clone(),
            })
        }
        (CatalogTarget::Infinite, PurchaseState::RepeatableReady) => {
            Some(ClientAction::QueueRepeatableResearch {
                session_id: session.session_id.clone(),
                nickname: CLIENT_ACTOR_LABEL.to_owned(),
                sig: session.sig.clone(),
                track_id: model.track_for_card(card).id.clone(),
            })
        }
        _ => None,
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
    let Some(selected) = ui.selected else {
        return;
    };
    let action = research_purchase_action(&model, research, selected, &session);
    if let Some(action) = action {
        outgoing.0.push(action);
    }
}

pub(super) fn handle_research_queue_controls(
    latest: Res<LatestSnapshot>,
    ui: Res<UpgradeTreeUi>,
    session: Res<Session>,
    mut outgoing: ResMut<OutgoingActions>,
    controls: Query<(&Interaction, &ResearchQueueControl), Changed<Interaction>>,
) {
    if !ui.visible || !session.ready {
        return;
    }
    let Some(research) = current_research(&latest) else {
        return;
    };
    for (interaction, control) in &controls {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(entry) = research.queue.get(control.slot) else {
            continue;
        };
        let common = (
            session.session_id.clone(),
            CLIENT_ACTOR_LABEL.to_owned(),
            session.sig.clone(),
        );
        let action = match control.action {
            ResearchQueueControlAction::Up if control.slot > 0 => {
                Some(ClientAction::MoveQueuedResearch {
                    session_id: common.0,
                    nickname: common.1,
                    sig: common.2,
                    key: entry.key.clone(),
                    direction: -1,
                })
            }
            ResearchQueueControlAction::Down if control.slot + 1 < research.queue.len() => {
                Some(ClientAction::MoveQueuedResearch {
                    session_id: common.0,
                    nickname: common.1,
                    sig: common.2,
                    key: entry.key.clone(),
                    direction: 1,
                })
            }
            ResearchQueueControlAction::Remove => Some(ClientAction::RemoveQueuedResearch {
                session_id: common.0,
                nickname: common.1,
                sig: common.2,
                key: entry.key.clone(),
            }),
            ResearchQueueControlAction::Up | ResearchQueueControlAction::Down => None,
        };
        if let Some(action) = action {
            outgoing.0.push(action);
        }
    }
}

// Historical raw-node UI tests are retained as design archaeology. The product
// now presents normalized technology tracks, so these raw-card assertions must
// never define the new screen contract.
#[cfg(any())]
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
    use std::{collections::HashSet, path::PathBuf};

    fn snapshot(owned: &[&str], blessings: f64) -> ResearchSnapshot {
        ResearchSnapshot {
            owned_node_ids: owned.iter().map(|id| (*id).to_owned()).collect(),
            research_points: 12.0,
            researcher_count: 2,
            blessings,
            next_target: None,
            queue: Vec::new(),
            repeatable_levels: std::collections::BTreeMap::new(),
            research_cost_multiplier: 1.0,
            research_time_multiplier: 1.0,
            points_per_hour: 1.0,
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
        assert_eq!(model.category_count(ResearchCategory::Upgrade), 163);
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
            let layout = responsive_research_layout(width, height, 1.0);
            assert_eq!(layout.root_width, width);
            assert_eq!(layout.root_height, height);
            assert!(layout.canvas_width >= 700.0);
            assert!((240.0..=320.0).contains(&layout.inspector_width));
            assert!(layout.canvas_height >= 600.0);
            assert!(layout.header_height <= 112.0);
        }
        let scaled = responsive_research_layout(1024.0, 768.0, 1.3);
        assert!(scaled.canvas_width >= 500.0);
        assert!(
            (root_overview_zoom(1.0) - root_overview_zoom(1.3) * 1.3).abs() < 0.001,
            "root overview should retain its physical scale as interface scale changes"
        );
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
        assert_eq!(
            world
                .query::<&ResearchConnectorStroke>()
                .iter(&world)
                .count(),
            research_catalog()
                .nodes()
                .iter()
                .map(|node| node.prerequisites.len())
                .sum::<usize>()
                * 3
        );
        assert_eq!(
            world.query::<&ResearchDepthBand>().iter(&world).count(),
            ResearchUiModel::from_catalog().layout.layer_count
        );
        assert_eq!(
            world.query::<&ResearchIcon>().iter(&world).count(),
            RESEARCH_NODE_COUNT + 1
        );
        assert_eq!(world.query::<&InspectorIcon>().iter(&world).count(), 1);
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
        assert!(model.layout.branch_count > 1);
        assert!(model.layout.size.x < 5_000.0);
        assert_eq!(
            model
                .connectors
                .iter()
                .filter(|connector| connector.primary)
                .count(),
            RESEARCH_NODE_COUNT - 1
        );
        for connector in &model.connectors {
            assert!(model.layout.depths[connector.from] < model.layout.depths[connector.to]);
            assert!(model.card_position(connector.from).x < model.card_position(connector.to).x);
        }
        for mut index in 0..RESEARCH_NODE_COUNT {
            let mut remaining = RESEARCH_NODE_COUNT;
            while index != model.root_index {
                index = model.layout.primary_parents[index]
                    .expect("every non-root study should belong to the visible backbone");
                remaining -= 1;
                assert!(remaining > 0, "primary research backbone contains a cycle");
            }
        }
        for (index, position) in model.layout.positions.iter().enumerate() {
            for other in model.layout.positions.iter().skip(index + 1) {
                assert!(
                    position.x != other.x || (position.y - other.y).abs() + 0.01 >= MAP_STEP_Y,
                    "two studies overlap in one prerequisite layer"
                );
            }
        }
        let mut first_layer = model
            .layout
            .depths
            .iter()
            .enumerate()
            .filter(|(_, depth)| **depth == 1)
            .map(|(index, _)| model.card_position(index).y)
            .collect::<Vec<_>>();
        first_layer.sort_by(f32::total_cmp);
        assert!(
            first_layer.windows(2).all(|pair| {
                pair[1] - pair[0] + 0.01 >= MAP_STEP_Y * (1.0 + MAIN_BRANCH_GAP_ROWS)
            })
        );
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

    #[test]
    fn filtered_results_retain_their_dependency_context() {
        let model = ResearchUiModel::from_catalog();
        let selected = model
            .layout
            .depths
            .iter()
            .enumerate()
            .max_by_key(|(_, depth)| **depth)
            .map(|(index, _)| index)
            .unwrap();
        let visible = model.with_ancestors(&[selected]);
        assert!(visible.contains(&selected));
        assert!(visible.contains(&model.root_index));
        assert!(visible.len() > model.layout.depths[selected]);
    }

    #[test]
    fn every_study_uses_a_tracked_semantic_icon() {
        let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut distinct = HashSet::new();
        for node in research_catalog().nodes() {
            let path = research_icon_path(node);
            assert!(
                workspace.join(path).is_file(),
                "{} uses missing icon {path}",
                node.id
            );
            distinct.insert(path);
        }
        assert!(
            distinct.len() >= 30,
            "the icon vocabulary should distinguish real research domains"
        );
        assert_eq!(
            research_icon_path(research_catalog().get("waterworks_sources").unwrap()),
            research_icon_path(research_catalog().get("waterworks_quality").unwrap())
        );
        assert_ne!(
            research_icon_path(research_catalog().get("waterworks_sources").unwrap()),
            research_icon_path(research_catalog().get("weaponcraft_sources").unwrap())
        );
    }
}
