//! LAI.68 canonical-v3 world rendering.
//!
//! This slice intentionally consumes only
//! [`cat_protocol::lai64::CanonicalSnapshotEnvelope`].  The canonical report
//! carries exact Hole, lair, residence, construction, storage, fishing-hut,
//! task-objective-footprint, and route coordinates.  A task's `site_id` plus
//! `footprint` is the authoritative objective geometry projected by the server.
//! Its typed work-site and delivery-site footprints supply those distinct
//! roles. When any role is absent, the renderer records a
//! [`Lai68UnavailableField`] rather than deriving it from a task name, route
//! endpoint, cat location, or local terrain.
//!
//! No ecological regeneration values cross this leaf.  In particular, even a
//! report-level-four estimate is unrelated to world geometry and is not copied
//! into an art key, tooltip, semantic node, or render component.

use std::collections::{BTreeMap, BTreeSet};

use accesskit::Role;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use cat_protocol::lai64::{
    CanonicalColonySnapshot, CanonicalSnapshotEnvelope, ConstructionPhase, ExactItemSnapshotV2,
    Footprint, PhysicalCargoSnapshot, QualityBandSnapshot, StorageZoneSnapshotV2, TaskState, Tile,
};

use crate::layered_sprite::{
    CanvasSpec, LayerSlot, SpritePart, VariantSpec, VariantState, VisibilityPredicate, VisualOwner,
};

use super::{LeaderAiSemanticNode, art_assets::resolve_lai68_art_key, semantic_node};

/// One world tile occupies this many Bevy world units.  This matches the
/// client’s flat-grid projection without importing renderer-private state.
pub const LAI68_TILE_WORLD_UNITS: f32 = 10.0;
/// The full report-safe world view at normal orthographic scale.
pub const LAI68_DEFAULT_HALF_WIDTH_TILES: i32 = 96;
/// The full report-safe world view at normal orthographic scale.
pub const LAI68_DEFAULT_HALF_HEIGHT_TILES: i32 = 72;
/// Product language for this world layer.  It is intentionally a style
/// contract, not a server-provided theme or a request to reconstruct art.
pub const LAI68_WORLD_ART_DIRECTION: &str = "Parchment, wood, dark-forest, restrained pixel overlays; no glass, glow, gradients, or generic task markers.";

const PARCHMENT: Color = Color::srgb(0.937, 0.886, 0.741);
const WOOD: Color = Color::srgb(0.427, 0.282, 0.169);
const DARK_FOREST: Color = Color::srgb(0.090, 0.235, 0.180);
const INK: Color = Color::srgb(0.153, 0.106, 0.086);
const STONE: Color = Color::srgb(0.48, 0.46, 0.39);
const WATER: Color = Color::srgb(0.24, 0.42, 0.50);
const RUST: Color = Color::srgb(0.643, 0.286, 0.176);
const MOSS: Color = Color::srgb(0.310, 0.439, 0.251);

/// The feed state is intentionally separate from the report envelope.  It
/// lets the world clear stale entities when a canonical session is unavailable
/// without using an old local snapshot as authority.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Lai68FeedState {
    #[default]
    Loading,
    Ready,
    Stale {
        stale_since_ms: i64,
    },
    UpdateRequired,
    Error {
        message: String,
    },
}

/// The only input accepted by this world-rendering leaf.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai68SnapshotFeed {
    pub envelope: Option<CanonicalSnapshotEnvelope>,
    pub state: Lai68FeedState,
}

/// A typed report availability failure.  These are product-visible contracts:
/// callers may render them in an inspector, while the world view must leave the
/// corresponding geometry absent.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lai68UnavailableField {
    /// This task report omitted every typed worker slot/bank/work footprint.
    TaskWorkTile { task_id: String },
    /// A route’s final tile is not a delivery endpoint authority.
    TaskDeliveryEndpoint { task_id: String },
    /// A water task's source objective was reported, but its distinct dry-bank
    /// work footprint was omitted.
    WaterBankWorkTile { task_id: String },
    /// A task claiming to be Workshop work did not carry an exact 3×3 grid.
    WorkshopThreeByThreeFootprint { task_id: String },
    /// Storage’s slot/lot relationship did not identify a tile for this lot.
    StorageLotTile { lot_id: String },
    /// Storage’s slot/item relationship did not identify a tile for this item.
    StorageItemTile { item_id: String },
    /// Container DTOs carry fullness and kind but no image/art key.
    ContainerArtKey { container_id: String },
    /// Canonical-v3 has no dedicated crop type/stage/footprint DTO.
    CropWorldState,
    /// Family data names an enterprise but does not place it in the world.
    EnterpriseWorldLocation { enterprise_id: String },
    /// Residence DTOs carry a footprint but not an image/art key.
    ResidenceArtKey { residence_id: String },
}

/// A stable tile coordinate.  It does not invent a coordinate system beyond
/// the protocol’s `Tile` fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Lai68Tile {
    pub x: i32,
    pub y: i32,
}

impl From<&Tile> for Lai68Tile {
    fn from(tile: &Tile) -> Self {
        Self {
            x: tile.x,
            y: tile.y,
        }
    }
}

/// Palette slots are semantic and deliberately opaque.  A reported art key is
/// retained beside this style; unknown keys are never remapped to a different
/// content type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lai68PixelStyle {
    Parchment,
    Wood,
    DarkForest,
    Ink,
    Stone,
    Water,
    Rust,
    Moss,
    Quality(u8),
}

impl Lai68PixelStyle {
    fn color(self) -> Color {
        match self {
            Self::Parchment => PARCHMENT,
            Self::Wood => WOOD,
            Self::DarkForest => DARK_FOREST,
            Self::Ink => INK,
            Self::Stone => STONE,
            Self::Water => WATER,
            Self::Rust => RUST,
            Self::Moss => MOSS,
            Self::Quality(0) => STONE,
            Self::Quality(1) => PARCHMENT,
            Self::Quality(2) => MOSS,
            Self::Quality(3) => WOOD,
            Self::Quality(_) => RUST,
        }
    }

    fn pixel_extent(self) -> f32 {
        match self {
            Self::Ink | Self::Water => LAI68_TILE_WORLD_UNITS * 0.34,
            Self::Quality(_) => LAI68_TILE_WORLD_UNITS * 0.42,
            _ => LAI68_TILE_WORLD_UNITS * 0.78,
        }
    }
}

/// Construction’s stateful art is a protocol key plus the phase enum.  The
/// renderer does not fabricate a phase key if either changes in a future
/// protocol revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lai68ConstructionOverlay {
    pub phase: ConstructionPhase,
    pub art_state_id: String,
    pub phase_progress_basis_points: u16,
}

/// Every role is specific to report geometry.  `TaskFootprint` is intentionally
/// named as a footprint (not objective/work/delivery) so it can never masquerade
/// as missing task semantics.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lai68RenderMarkerRole {
    HoleBoundary {
        cell_index: u8,
    },
    HoleWork {
        cell_index: u8,
    },
    HoleArt {
        width: u8,
        depth: u8,
        darkness: u8,
    },
    TaskObjectiveFootprint {
        task_id: String,
        cell_index: u16,
    },
    HuntObjectiveFootprint {
        task_id: String,
        cell_index: u16,
    },
    WaterObjectiveFootprint {
        task_id: String,
        cell_index: u16,
    },
    WorkshopFootprint {
        task_id: String,
        cell_index: u8,
    },
    TaskWorkSite {
        task_id: String,
        site_id: String,
        slot_id: Option<String>,
        cell_index: u16,
    },
    WaterBankWorkSite {
        task_id: String,
        site_id: String,
        slot_id: Option<String>,
        cell_index: u16,
    },
    TaskDeliverySite {
        task_id: String,
        site_id: String,
        slot_id: Option<String>,
        cell_index: u16,
    },
    TaskRoute {
        task_id: String,
        route_index: u16,
    },
    HuntLair {
        site_id: String,
    },
    ConstructionFootprint {
        project_id: String,
        cell_index: u16,
    },
    StorageContainer {
        container_id: String,
    },
    StorageLot {
        lot_id: String,
    },
    StorageItem {
        item_id: String,
    },
    ResidenceFootprint {
        residence_id: String,
        cell_index: u16,
    },
    FamilyResidence {
        household_id: String,
        cell_index: u16,
    },
    FishingHutFootprint {
        hut_id: String,
        cell_index: u8,
    },
    FishingDock {
        hut_id: String,
    },
    FishingWaterAttachment {
        hut_id: String,
    },
    ReportedVisualState {
        subject_id: String,
        cell_index: u16,
    },
}

impl Lai68RenderMarkerRole {
    fn layer(&self) -> Lai68RenderLayer {
        match self {
            Self::TaskRoute { .. } => Lai68RenderLayer::Route,
            Self::HoleBoundary { .. }
            | Self::TaskObjectiveFootprint { .. }
            | Self::HuntObjectiveFootprint { .. }
            | Self::WaterObjectiveFootprint { .. }
            | Self::WorkshopFootprint { .. }
            | Self::ConstructionFootprint { .. }
            | Self::ResidenceFootprint { .. }
            | Self::FishingHutFootprint { .. }
            | Self::ReportedVisualState { .. } => Lai68RenderLayer::Footprint,
            Self::HoleWork { .. }
            | Self::HoleArt { .. }
            | Self::HuntLair { .. }
            | Self::TaskWorkSite { .. }
            | Self::WaterBankWorkSite { .. }
            | Self::TaskDeliverySite { .. }
            | Self::StorageContainer { .. }
            | Self::StorageLot { .. }
            | Self::StorageItem { .. }
            | Self::FamilyResidence { .. }
            | Self::FishingDock { .. }
            | Self::FishingWaterAttachment { .. } => Lai68RenderLayer::Marker,
        }
    }

    fn style(&self, construction: Option<&Lai68ConstructionOverlay>) -> Lai68PixelStyle {
        match self {
            Self::HoleBoundary { .. } => Lai68PixelStyle::Stone,
            Self::HoleWork { .. } => Lai68PixelStyle::DarkForest,
            Self::HoleArt { .. } => Lai68PixelStyle::DarkForest,
            Self::TaskObjectiveFootprint { .. } => Lai68PixelStyle::Parchment,
            Self::HuntObjectiveFootprint { .. } => Lai68PixelStyle::Rust,
            Self::WaterObjectiveFootprint { .. } => Lai68PixelStyle::Water,
            Self::WorkshopFootprint { .. } => Lai68PixelStyle::Wood,
            Self::TaskWorkSite { .. } => Lai68PixelStyle::Moss,
            Self::WaterBankWorkSite { .. } => Lai68PixelStyle::Water,
            Self::TaskDeliverySite { .. } => Lai68PixelStyle::Wood,
            Self::TaskRoute { .. } => Lai68PixelStyle::Ink,
            Self::HuntLair { .. } => Lai68PixelStyle::Rust,
            Self::ConstructionFootprint { .. } => match construction.map(|overlay| overlay.phase) {
                Some(ConstructionPhase::Scaffold) => Lai68PixelStyle::Wood,
                Some(ConstructionPhase::Structure) => Lai68PixelStyle::Stone,
                Some(ConstructionPhase::FitOut) => Lai68PixelStyle::Moss,
                Some(ConstructionPhase::Operational) => Lai68PixelStyle::DarkForest,
                Some(ConstructionPhase::Blocked | ConstructionPhase::Cancelled) => {
                    Lai68PixelStyle::Rust
                }
                Some(ConstructionPhase::Reserve) | None => Lai68PixelStyle::Parchment,
            },
            Self::StorageContainer { .. } => Lai68PixelStyle::Wood,
            Self::StorageLot { .. } => Lai68PixelStyle::Quality(1),
            Self::StorageItem { .. } => Lai68PixelStyle::Quality(1),
            Self::ResidenceFootprint { .. } => Lai68PixelStyle::Wood,
            Self::FamilyResidence { .. } => Lai68PixelStyle::Parchment,
            Self::FishingHutFootprint { .. } => Lai68PixelStyle::Wood,
            Self::FishingDock { .. } => Lai68PixelStyle::Moss,
            Self::FishingWaterAttachment { .. } => Lai68PixelStyle::Water,
            Self::ReportedVisualState { .. } => Lai68PixelStyle::Ink,
        }
    }

    fn stable_name(&self) -> &'static str {
        match self {
            Self::HoleBoundary { .. } => "hole-boundary",
            Self::HoleWork { .. } => "hole-work",
            Self::HoleArt { .. } => "hole-art",
            Self::TaskObjectiveFootprint { .. } => "task-objective-footprint",
            Self::HuntObjectiveFootprint { .. } => "hunt-objective-footprint",
            Self::WaterObjectiveFootprint { .. } => "water-objective-footprint",
            Self::WorkshopFootprint { .. } => "workshop-footprint",
            Self::TaskWorkSite { .. } => "task-work-site",
            Self::WaterBankWorkSite { .. } => "water-bank-work-site",
            Self::TaskDeliverySite { .. } => "task-delivery-site",
            Self::TaskRoute { .. } => "task-route",
            Self::HuntLair { .. } => "hunt-lair",
            Self::ConstructionFootprint { .. } => "construction-footprint",
            Self::StorageContainer { .. } => "storage-container",
            Self::StorageLot { .. } => "storage-lot",
            Self::StorageItem { .. } => "storage-item",
            Self::ResidenceFootprint { .. } => "residence-footprint",
            Self::FamilyResidence { .. } => "family-residence",
            Self::FishingHutFootprint { .. } => "fishing-hut-footprint",
            Self::FishingDock { .. } => "fishing-dock",
            Self::FishingWaterAttachment { .. } => "fishing-water-attachment",
            Self::ReportedVisualState { .. } => "reported-visual-state",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lai68RenderLayer {
    Route,
    Footprint,
    Marker,
}

impl Lai68RenderLayer {
    fn z(self) -> f32 {
        match self {
            Self::Route => 610.0,
            Self::Footprint => 620.0,
            Self::Marker => 630.0,
        }
    }
}

/// Stable entity identity.  Coordinates participate in the key because a
/// changed reported cell must replace—not silently reuse—the old entity.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Lai68RenderKey {
    pub layer: Lai68RenderLayer,
    pub subject_id: String,
    pub role: Lai68RenderMarkerRole,
    pub tile: Lai68Tile,
}

/// Pure, report-safe marker model consumed by the Bevy reconciliation system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lai68WorldMarker {
    pub key: Lai68RenderKey,
    pub role: Lai68RenderMarkerRole,
    pub tile: Lai68Tile,
    pub semantic_id: String,
    pub tooltip: String,
    pub style: Lai68PixelStyle,
    pub reported_art_key: Option<String>,
    pub construction_overlay: Option<Lai68ConstructionOverlay>,
}

/// A projection never stores hidden world truth or attempts a local ecology
/// simulation.  `removed_keys` is set by the Bevy sync system from two adjacent
/// report projections, allowing deterministic restart/despawn assertions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai68WorldProjection {
    pub feed_state: Lai68FeedState,
    pub selected_colony_id: Option<String>,
    pub state_version: Option<u64>,
    pub protocol_valid: bool,
    pub markers: Vec<Lai68WorldMarker>,
    pub unavailable: BTreeSet<Lai68UnavailableField>,
    pub removed_keys: Vec<Lai68RenderKey>,
    pub protocol_error: Option<String>,
    pub reads_hidden_regeneration: bool,
    pub uses_generic_marker_fallback: bool,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai68WorldProjectionResource(pub Lai68WorldProjection);

/// Tile-space camera data.  `orthographic_scale_basis_points` grows when the
/// player zooms out, so culling exposes more authoritative tiles at larger
/// values.  It is intentionally a small adapter resource rather than a second
/// camera controller.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lai68Viewport {
    pub center_x: i32,
    pub center_y: i32,
    pub half_width_tiles: i32,
    pub half_height_tiles: i32,
    pub orthographic_scale_basis_points: u16,
}

impl Default for Lai68Viewport {
    fn default() -> Self {
        Self {
            center_x: 0,
            center_y: 0,
            half_width_tiles: LAI68_DEFAULT_HALF_WIDTH_TILES,
            half_height_tiles: LAI68_DEFAULT_HALF_HEIGHT_TILES,
            orthographic_scale_basis_points: 10_000,
        }
    }
}

impl Lai68Viewport {
    fn contains(self, tile: Lai68Tile) -> bool {
        let scale = i64::from(self.orthographic_scale_basis_points.max(1));
        let half_width = (i64::from(self.half_width_tiles.max(0)) * scale) / 10_000;
        let half_height = (i64::from(self.half_height_tiles.max(0)) * scale) / 10_000;
        i64::from(tile.x).abs_diff(i64::from(self.center_x)) <= half_width.unsigned_abs()
            && i64::from(tile.y).abs_diff(i64::from(self.center_y)) <= half_height.unsigned_abs()
    }
}

/// Marker data stored on a real Bevy sprite entity.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct Lai68RenderEntity {
    pub key: Lai68RenderKey,
    pub tile: Lai68Tile,
    pub role: Lai68RenderMarkerRole,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lai68WorldDetailTarget {
    ExactItem(String),
    BulkLot(String),
    RareMaterial(String),
}

/// Typed report-visible world selection. Only identities already present on a
/// canonical LAI.68 marker can be emitted.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct Lai68WorldDetailSelection(pub Lai68WorldDetailTarget);

/// Report-safe semantic node metadata mirrors AccessKit’s label and test ID so
/// native and WASM integrations can inspect the same stable identity.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct Lai68SemanticWorldNode {
    pub semantic_id: String,
    pub tooltip: String,
}

/// Adds actual Bevy projection, sprite reconciliation, semantic nodes, and
/// viewport culling.  It is additive and owns no protocol transport, input
/// action, simulation state, or camera movement.
#[derive(Default)]
pub struct Lai68WorldRenderPlugin;

impl Plugin for Lai68WorldRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Lai68SnapshotFeed>()
            .init_resource::<Lai68WorldProjectionResource>()
            .init_resource::<Lai68Viewport>()
            .add_message::<Lai68WorldDetailSelection>()
            .add_systems(
                Update,
                (
                    sync_lai68_projection,
                    reconcile_lai68_world_entities,
                    cull_lai68_world_entities,
                    emit_lai68_world_detail_selection,
                )
                    .chain(),
            );
    }
}

fn sync_lai68_projection(
    feed: Res<'_, Lai68SnapshotFeed>,
    mut projection: ResMut<'_, Lai68WorldProjectionResource>,
) {
    if !feed.is_changed() {
        return;
    }
    let previous_keys = projection
        .0
        .markers
        .iter()
        .map(|marker| marker.key.clone())
        .collect::<BTreeSet<_>>();
    let mut next = project_lai68_world(&feed);
    let next_keys = next
        .markers
        .iter()
        .map(|marker| marker.key.clone())
        .collect::<BTreeSet<_>>();
    next.removed_keys = previous_keys.difference(&next_keys).cloned().collect();
    projection.0 = next;
}

fn reconcile_lai68_world_entities(
    mut commands: Commands<'_, '_>,
    projection: Res<'_, Lai68WorldProjectionResource>,
    assets: Res<'_, AssetServer>,
    existing: Query<'_, '_, (Entity, &Lai68RenderEntity)>,
) {
    if !projection.is_changed() {
        return;
    }
    let current = existing
        .iter()
        .map(|(entity, marker)| (marker.key.clone(), (entity, marker.role.clone())))
        .collect::<BTreeMap<_, _>>();
    let desired = projection
        .0
        .markers
        .iter()
        .map(|marker| (marker.key.clone(), marker))
        .collect::<BTreeMap<_, _>>();

    for (key, (entity, _)) in &current {
        if !desired.contains_key(key) {
            commands.entity(*entity).despawn();
        }
    }
    for (key, marker) in desired {
        if let Some((entity, current_role)) = current.get(&key) {
            if matches!(&marker.role, Lai68RenderMarkerRole::HoleArt { .. }) {
                if current_role != &marker.role {
                    commands.entity(*entity).despawn();
                    spawn_render_entity(&mut commands, marker, &assets);
                }
            } else {
                commands
                    .entity(*entity)
                    .insert(render_bundle(marker, &assets));
            }
        } else {
            spawn_render_entity(&mut commands, marker, &assets);
        }
    }
}

fn cull_lai68_world_entities(
    viewport: Res<'_, Lai68Viewport>,
    mut entities: Query<'_, '_, (&Lai68RenderEntity, &mut Visibility)>,
) {
    for (entity, mut visibility) in &mut entities {
        *visibility = if viewport.contains(entity.tile) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

#[allow(clippy::type_complexity)]
fn emit_lai68_world_detail_selection(
    buttons: Option<Res<'_, ButtonInput<MouseButton>>>,
    windows: Query<'_, '_, &Window, With<PrimaryWindow>>,
    camera: Query<'_, '_, (&Camera, &GlobalTransform), With<crate::WorldCamera>>,
    blockers: Query<
        '_,
        '_,
        (
            &ComputedNode,
            &UiGlobalTransform,
            &Node,
            Option<&InheritedVisibility>,
        ),
        With<crate::WorldInputBlocker>,
    >,
    markers: Query<
        '_,
        '_,
        (
            &Lai68RenderEntity,
            &GlobalTransform,
            Option<&InheritedVisibility>,
        ),
    >,
    mut selections: MessageWriter<'_, Lai68WorldDetailSelection>,
) {
    let Some(buttons) = buttons else {
        return;
    };
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if blockers.iter().any(|(computed, transform, node, visible)| {
        node.display != Display::None
            && visible.is_none_or(|visible| visible.get())
            && computed.contains_point(*transform, cursor)
    }) {
        return;
    }
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(camera_transform, cursor) else {
        return;
    };
    let maximum_distance_squared = (LAI68_TILE_WORLD_UNITS * 0.5).powi(2);
    let mut candidates = markers
        .iter()
        .filter(|(_, _, visible)| visible.is_none_or(|visible| visible.get()))
        .filter_map(|(marker, transform, _)| {
            let target = match &marker.role {
                Lai68RenderMarkerRole::StorageItem { item_id } => {
                    Lai68WorldDetailTarget::ExactItem(item_id.clone())
                }
                Lai68RenderMarkerRole::StorageLot { lot_id } => {
                    Lai68WorldDetailTarget::BulkLot(lot_id.clone())
                }
                _ => return None,
            };
            let distance_squared = transform.translation().truncate().distance_squared(world);
            (distance_squared <= maximum_distance_squared).then_some((
                distance_squared.to_bits(),
                marker.key.subject_id.clone(),
                target,
            ))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    if let Some((_, _, target)) = candidates.into_iter().next() {
        selections.write(Lai68WorldDetailSelection(target));
    }
}

fn spawn_render_entity(
    commands: &mut Commands<'_, '_>,
    marker: &Lai68WorldMarker,
    assets: &AssetServer,
) {
    if let Lai68RenderMarkerRole::HoleArt {
        width,
        depth,
        darkness,
    } = &marker.role
    {
        spawn_hole_art(commands, marker, assets, *width, *depth, *darkness);
    } else {
        commands.spawn(render_bundle(marker, assets));
    }
}

fn render_bundle(marker: &Lai68WorldMarker, assets: &AssetServer) -> impl Bundle {
    let size = marker.style.pixel_extent();
    let sprite = marker
        .reported_art_key
        .as_deref()
        .and_then(resolve_lai68_art_key)
        .map_or_else(
            || Sprite::from_color(marker.style.color(), Vec2::splat(size)),
            |art| Sprite {
                image: assets.load(art.path),
                custom_size: Some(Vec2::new(
                    f32::from(art.native_width_px) / 16.0 * LAI68_TILE_WORLD_UNITS,
                    f32::from(art.native_height_px) / 16.0 * LAI68_TILE_WORLD_UNITS,
                )),
                ..default()
            },
        );
    (
        sprite,
        Transform::from_xyz(
            marker.tile.x as f32 * LAI68_TILE_WORLD_UNITS,
            -(marker.tile.y as f32) * LAI68_TILE_WORLD_UNITS,
            marker.key.layer.z(),
        ),
        Visibility::Inherited,
        Lai68RenderEntity {
            key: marker.key.clone(),
            tile: marker.tile,
            role: marker.role.clone(),
        },
        Lai68SemanticWorldNode {
            semantic_id: marker.semantic_id.clone(),
            tooltip: marker.tooltip.clone(),
        },
        LeaderAiSemanticNode {
            semantic_id: marker.semantic_id.clone(),
            focus_order: 0,
            enabled: true,
        },
        semantic_node(
            Role::Image,
            marker.semantic_id.clone(),
            marker.tooltip.clone(),
            true,
        ),
        Name::new(format!("LAI.68 {}", marker.role.stable_name())),
    )
}

fn spawn_hole_art(
    commands: &mut Commands<'_, '_>,
    marker: &Lai68WorldMarker,
    assets: &AssetServer,
    width: u8,
    depth: u8,
    darkness: u8,
) {
    let owner = VisualOwner::new("hole", &marker.key.subject_id);
    let state = VariantState::new()
        .with_level("width", width)
        .with_level("depth", depth)
        .with_level("darkness", darkness);
    let resolved = hole_variant_spec().resolve(&state);
    let Some((base, overlays)) = resolved.parts().split_first() else {
        return;
    };
    let root = commands
        .spawn((
            base.sprite(assets, Vec2::splat(LAI68_TILE_WORLD_UNITS)),
            Transform::from_xyz(
                marker.tile.x as f32 * LAI68_TILE_WORLD_UNITS,
                -(marker.tile.y as f32) * LAI68_TILE_WORLD_UNITS,
                marker.key.layer.z(),
            ),
            Visibility::Inherited,
            Lai68RenderEntity {
                key: marker.key.clone(),
                tile: marker.tile,
                role: marker.role.clone(),
            },
            Lai68SemanticWorldNode {
                semantic_id: marker.semantic_id.clone(),
                tooltip: marker.tooltip.clone(),
            },
            LeaderAiSemanticNode {
                semantic_id: marker.semantic_id.clone(),
                focus_order: 0,
                enabled: true,
            },
            semantic_node(
                Role::Image,
                marker.semantic_id.clone(),
                marker.tooltip.clone(),
                true,
            ),
            Name::new("LAI.68 layered Hole art"),
            owner.clone(),
            base.slot.clone(),
            base.clone(),
            resolved.signature().clone(),
        ))
        .id();
    commands.entity(root).with_children(|parent| {
        for part in overlays {
            parent.spawn((
                part.sprite(assets, Vec2::splat(LAI68_TILE_WORLD_UNITS)),
                part.local_transform(Vec2::splat(LAI68_TILE_WORLD_UNITS), 0.01),
                Visibility::Inherited,
                owner.clone(),
                part.slot.clone(),
                part.clone(),
            ));
        }
    });
}

fn hole_variant_spec() -> VariantSpec {
    let canvas = CanvasSpec::new(UVec2::splat(16), UVec2::splat(80))
        .expect("the Hole canvas is exactly five world tiles");
    let mut parts = vec![SpritePart::new(
        LayerSlot::new(0, "base"),
        "public/images/game/buildings/black-hole/base.png",
    )];
    for (axis, order) in [("width", 10_i16), ("depth", 20), ("darkness", 30)] {
        for level in 1_i16..=10 {
            parts.push(
                SpritePart::new(
                    LayerSlot::new(order + level, format!("{axis}-{level:02}")),
                    format!("public/images/game/buildings/black-hole/{axis}-{level:02}.png"),
                )
                .visible_when(VisibilityPredicate::level_range(
                    axis,
                    u8::try_from(level).expect("Hole axis level is bounded"),
                    u8::try_from(level).expect("Hole axis level is bounded"),
                )),
            );
        }
    }
    VariantSpec::new(canvas, parts).expect("the Hole layer manifest is valid")
}

/// Produces the deterministic pure model used by the Bevy systems.  Invalid
/// canonical envelopes fail closed: no stale markers survive from a previous
/// selected colony.
#[must_use]
pub fn project_lai68_world(feed: &Lai68SnapshotFeed) -> Lai68WorldProjection {
    let Some(envelope) = &feed.envelope else {
        return Lai68WorldProjection {
            feed_state: feed.state.clone(),
            ..default()
        };
    };
    if let Err(error) = envelope.validate() {
        return Lai68WorldProjection {
            feed_state: feed.state.clone(),
            protocol_error: Some(format!("Invalid canonical world report: {error}")),
            ..default()
        };
    }
    let Some(colony) = envelope.colonies.first() else {
        return Lai68WorldProjection {
            feed_state: feed.state.clone(),
            protocol_error: Some("Selected canonical colony report is absent.".to_owned()),
            ..default()
        };
    };

    let mut builder = Lai68ProjectionBuilder::default();
    project_hole(colony, &mut builder);
    project_tasks(colony, &mut builder);
    project_construction(colony, &mut builder);
    project_storage(colony, &mut builder);
    project_residences_and_families(colony, &mut builder);
    project_hunting_sites(colony, &mut builder);
    project_fishing_huts(colony, &mut builder);
    project_reported_visual_states(colony, &mut builder);

    // Canonical-v3 only has an untyped visual-state collection; recognizing a
    // crop by an art key or English label would be client-side inference.
    builder
        .unavailable
        .insert(Lai68UnavailableField::CropWorldState);

    Lai68WorldProjection {
        feed_state: feed.state.clone(),
        selected_colony_id: Some(envelope.selected_colony_id.as_str().to_owned()),
        state_version: Some(colony.state_version),
        protocol_valid: true,
        markers: builder.markers.into_values().collect(),
        unavailable: builder.unavailable,
        removed_keys: Vec::new(),
        protocol_error: None,
        reads_hidden_regeneration: false,
        uses_generic_marker_fallback: false,
    }
}

#[derive(Default)]
struct Lai68ProjectionBuilder {
    markers: BTreeMap<Lai68RenderKey, Lai68WorldMarker>,
    unavailable: BTreeSet<Lai68UnavailableField>,
}

impl Lai68ProjectionBuilder {
    fn marker(
        &mut self,
        subject_id: &str,
        role: Lai68RenderMarkerRole,
        tile: Lai68Tile,
        tooltip: String,
        reported_art_key: Option<String>,
        construction_overlay: Option<Lai68ConstructionOverlay>,
    ) {
        let style = role.style(construction_overlay.as_ref());
        let key = Lai68RenderKey {
            layer: role.layer(),
            subject_id: subject_id.to_owned(),
            role: role.clone(),
            tile,
        };
        let semantic_id = semantic_id(&key);
        let marker = Lai68WorldMarker {
            key: key.clone(),
            role,
            tile,
            semantic_id,
            tooltip,
            style,
            reported_art_key,
            construction_overlay,
        };
        // A BTreeMap makes repeated snapshot entities deterministic.  The key
        // includes role, subject, and tile, so legitimate co-located roles do
        // not collapse into a generic marker.
        self.markers.entry(key).or_insert(marker);
    }
}

fn project_hole(colony: &CanonicalColonySnapshot, builder: &mut Lai68ProjectionBuilder) {
    let hole_id = colony.hole.hole_id.as_str();
    let boundary_tiles = sorted_tiles(&colony.hole.footprint);
    for (index, tile) in boundary_tiles.iter().copied().enumerate() {
        builder.marker(
            hole_id,
            Lai68RenderMarkerRole::HoleBoundary {
                cell_index: u8::try_from(index).expect("canonical Hole footprint is 25 cells"),
            },
            tile,
            format!("The Hole landmark boundary cell {}", index + 1),
            None,
            None,
        );
    }
    for (index, tile) in sorted_tiles(&colony.hole.work_footprint)
        .into_iter()
        .enumerate()
    {
        builder.marker(
            hole_id,
            Lai68RenderMarkerRole::HoleWork {
                cell_index: u8::try_from(index).expect("canonical Hole work footprint is 9 cells"),
            },
            tile,
            format!("The Hole reported work cell {}", index + 1),
            None,
            None,
        );
    }
    if let (Some(min_x), Some(max_x), Some(min_y), Some(max_y)) = (
        boundary_tiles.iter().map(|tile| tile.x).min(),
        boundary_tiles.iter().map(|tile| tile.x).max(),
        boundary_tiles.iter().map(|tile| tile.y).min(),
        boundary_tiles.iter().map(|tile| tile.y).max(),
    ) {
        builder.marker(
            hole_id,
            Lai68RenderMarkerRole::HoleArt {
                width: colony.hole.width,
                depth: colony.hole.depth,
                darkness: colony.hole.darkness,
            },
            Lai68Tile {
                x: min_x + (max_x - min_x) / 2,
                y: min_y + (max_y - min_y) / 2,
            },
            format!(
                "The Hole, reported axes width {}, depth {}, darkness {}",
                colony.hole.width, colony.hole.depth, colony.hole.darkness
            ),
            Some("art_station_black_hole".to_owned()),
            None,
        );
    }
}

fn project_tasks(colony: &CanonicalColonySnapshot, builder: &mut Lai68ProjectionBuilder) {
    for task in &colony.tasks {
        let task_id = task.task_id.as_str();
        let task_kind = task_kind(task.task_kind_id.as_str());
        let state = task_state_label(task.state);
        if matches!(task_kind, Lai68KnownTaskKind::Workshop)
            && is_exact_three_by_three(&task.footprint)
        {
            for (index, tile) in sorted_tiles(&task.footprint).into_iter().enumerate() {
                builder.marker(
                    task_id,
                    Lai68RenderMarkerRole::WorkshopFootprint {
                        task_id: task_id.to_owned(),
                        cell_index: u8::try_from(index).expect("three by three has nine cells"),
                    },
                    tile,
                    format!(
                        "Reported Workshop footprint cell {} for {} task",
                        index + 1,
                        state
                    ),
                    None,
                    None,
                );
            }
        } else {
            if matches!(task_kind, Lai68KnownTaskKind::Workshop) {
                builder
                    .unavailable
                    .insert(Lai68UnavailableField::WorkshopThreeByThreeFootprint {
                        task_id: task_id.to_owned(),
                    });
            }
            for (index, tile) in sorted_tiles(&task.footprint).into_iter().enumerate() {
                let cell_index = u16::try_from(index).expect("canonical task footprint bound");
                let role = match task_kind {
                    Lai68KnownTaskKind::Hunt => Lai68RenderMarkerRole::HuntObjectiveFootprint {
                        task_id: task_id.to_owned(),
                        cell_index,
                    },
                    Lai68KnownTaskKind::Water => Lai68RenderMarkerRole::WaterObjectiveFootprint {
                        task_id: task_id.to_owned(),
                        cell_index,
                    },
                    _ => Lai68RenderMarkerRole::TaskObjectiveFootprint {
                        task_id: task_id.to_owned(),
                        cell_index,
                    },
                };
                builder.marker(
                    task_id,
                    role,
                    tile,
                    format!(
                        "Reported task objective cell {} for {} task",
                        index + 1,
                        state
                    ),
                    None,
                    None,
                );
            }
        }
        for (index, tile) in task.route.ordered_tiles.iter().enumerate() {
            builder.marker(
                task_id,
                Lai68RenderMarkerRole::TaskRoute {
                    task_id: task_id.to_owned(),
                    route_index: u16::try_from(index).expect("canonical task route bound"),
                },
                Lai68Tile::from(tile),
                format!("Reported route step {} for {} task", index + 1, state),
                None,
                None,
            );
        }

        if task.work_sites.is_empty() {
            builder
                .unavailable
                .insert(Lai68UnavailableField::TaskWorkTile {
                    task_id: task_id.to_owned(),
                });
            if matches!(task_kind, Lai68KnownTaskKind::Water) {
                builder
                    .unavailable
                    .insert(Lai68UnavailableField::WaterBankWorkTile {
                        task_id: task_id.to_owned(),
                    });
            }
        } else {
            let mut work_sites = task.work_sites.iter().collect::<Vec<_>>();
            work_sites.sort_by(|left, right| {
                left.site_id
                    .cmp(&right.site_id)
                    .then_with(|| left.slot_id.cmp(&right.slot_id))
            });
            for work_site in work_sites {
                let site_id = work_site.site_id.as_str();
                let slot_id = work_site
                    .slot_id
                    .as_ref()
                    .map(|slot_id| slot_id.as_str().to_owned());
                for (index, tile) in sorted_tiles(&work_site.footprint).into_iter().enumerate() {
                    let cell_index =
                        u16::try_from(index).expect("canonical task work-site footprint bound");
                    let role = if matches!(task_kind, Lai68KnownTaskKind::Water) {
                        Lai68RenderMarkerRole::WaterBankWorkSite {
                            task_id: task_id.to_owned(),
                            site_id: site_id.to_owned(),
                            slot_id: slot_id.clone(),
                            cell_index,
                        }
                    } else {
                        Lai68RenderMarkerRole::TaskWorkSite {
                            task_id: task_id.to_owned(),
                            site_id: site_id.to_owned(),
                            slot_id: slot_id.clone(),
                            cell_index,
                        }
                    };
                    builder.marker(
                        task_id,
                        role,
                        tile,
                        format!("Reported work site {} for {} task", site_id, state),
                        None,
                        None,
                    );
                }
            }
        }
        if let Some(delivery_site) = &task.delivery_site {
            let site_id = delivery_site.site_id.as_str();
            let slot_id = delivery_site
                .slot_id
                .as_ref()
                .map(|slot_id| slot_id.as_str().to_owned());
            for (index, tile) in sorted_tiles(&delivery_site.footprint)
                .into_iter()
                .enumerate()
            {
                builder.marker(
                    task_id,
                    Lai68RenderMarkerRole::TaskDeliverySite {
                        task_id: task_id.to_owned(),
                        site_id: site_id.to_owned(),
                        slot_id: slot_id.clone(),
                        cell_index: u16::try_from(index)
                            .expect("canonical task delivery-site footprint bound"),
                    },
                    tile,
                    format!("Reported delivery site {} for {} task", site_id, state),
                    None,
                    None,
                );
            }
        } else {
            builder
                .unavailable
                .insert(Lai68UnavailableField::TaskDeliveryEndpoint {
                    task_id: task_id.to_owned(),
                });
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lai68KnownTaskKind {
    Workshop,
    Hunt,
    Water,
    Other,
}

fn task_kind(value: &str) -> Lai68KnownTaskKind {
    match value {
        "workshop_work" => Lai68KnownTaskKind::Workshop,
        "hunt" => Lai68KnownTaskKind::Hunt,
        "fetch_water" => Lai68KnownTaskKind::Water,
        _ => Lai68KnownTaskKind::Other,
    }
}

fn task_state_label(state: TaskState) -> &'static str {
    match state {
        TaskState::Proposed => "proposed",
        TaskState::Reserved => "reserved",
        TaskState::Assigned => "assigned",
        TaskState::InProgress => "in progress",
        TaskState::Blocked => "blocked",
        TaskState::Recovering => "recovering",
        TaskState::Complete => "complete",
        TaskState::Refused => "refused",
    }
}

fn project_construction(colony: &CanonicalColonySnapshot, builder: &mut Lai68ProjectionBuilder) {
    for project in &colony.construction {
        let project_id = project.project_id.as_str();
        let overlay = Lai68ConstructionOverlay {
            phase: project.phase,
            art_state_id: project.art_state_id.as_str().to_owned(),
            phase_progress_basis_points: project.phase_progress_basis_points,
        };
        for (index, tile) in sorted_tiles(&project.footprint).into_iter().enumerate() {
            builder.marker(
                project_id,
                Lai68RenderMarkerRole::ConstructionFootprint {
                    project_id: project_id.to_owned(),
                    cell_index: u16::try_from(index)
                        .expect("canonical construction footprint bound"),
                },
                tile,
                format!(
                    "Reported {} construction phase, {}% complete",
                    construction_phase_label(project.phase),
                    project.phase_progress_basis_points / 100
                ),
                Some(project.art_state_id.as_str().to_owned()),
                Some(overlay.clone()),
            );
        }
    }
}

fn construction_phase_label(phase: ConstructionPhase) -> &'static str {
    match phase {
        ConstructionPhase::Reserve => "reserved",
        ConstructionPhase::Scaffold => "scaffold",
        ConstructionPhase::Structure => "structure",
        ConstructionPhase::FitOut => "fit-out",
        ConstructionPhase::Operational => "operational",
        ConstructionPhase::Blocked => "blocked",
        ConstructionPhase::Cancelled => "cancelled",
    }
}

fn project_storage(colony: &CanonicalColonySnapshot, builder: &mut Lai68ProjectionBuilder) {
    for zone in &colony.storage_zones {
        project_storage_zone(zone, &colony.exact_items, builder);
    }
}

fn project_storage_zone(
    zone: &StorageZoneSnapshotV2,
    exact_items: &[ExactItemSnapshotV2],
    builder: &mut Lai68ProjectionBuilder,
) {
    let containers = zone
        .containers
        .iter()
        .map(|container| (container.container_id.as_str(), container))
        .collect::<BTreeMap<_, _>>();
    let lots = zone
        .lots
        .iter()
        .map(|lot| (lot.cargo_id.as_str(), lot))
        .collect::<BTreeMap<_, _>>();
    let items = exact_items
        .iter()
        .map(|item| (item.item_id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let mut placed_lots = BTreeSet::<String>::new();
    let mut placed_items = BTreeSet::<String>::new();
    let mut storage_tiles = zone.tiles.iter().collect::<Vec<_>>();
    storage_tiles.sort_by_key(|entry| (entry.tile.y, entry.tile.x));
    for storage_tile in storage_tiles {
        let tile = Lai68Tile::from(&storage_tile.tile);
        for slot in &storage_tile.slots {
            if let Some(container_id) = &slot.container_id
                && let Some(container) = containers.get(container_id.as_str())
            {
                let id = container.container_id.as_str();
                builder.marker(
                    id,
                    Lai68RenderMarkerRole::StorageContainer {
                        container_id: id.to_owned(),
                    },
                    tile,
                    format!(
                        "Reported {}: {}% full, {} internal slots",
                        container.container_kind_id.as_str(),
                        container.fullness_basis_points / 100,
                        container.capacity_slots
                    ),
                    None,
                    None,
                );
                builder
                    .unavailable
                    .insert(Lai68UnavailableField::ContainerArtKey {
                        container_id: id.to_owned(),
                    });
            }
            if let Some(lot_id) = &slot.lot_id
                && let Some(lot) = lots.get(lot_id.as_str())
            {
                project_storage_lot_marker(lot, tile, builder);
                placed_lots.insert(lot.cargo_id.as_str().to_owned());
            }
            if let Some(item_id) = &slot.item_id
                && let Some(item) = items.get(item_id.as_str())
            {
                let id = item.item_id.as_str();
                placed_items.insert(id.to_owned());
                let role = Lai68RenderMarkerRole::StorageItem {
                    item_id: id.to_owned(),
                };
                let mut marker = Lai68WorldMarker {
                    key: Lai68RenderKey {
                        layer: role.layer(),
                        subject_id: id.to_owned(),
                        role: role.clone(),
                        tile,
                    },
                    role,
                    tile,
                    semantic_id: String::new(),
                    tooltip: format!(
                        "Reported {} made from {}; {} quality; {}% durability",
                        item.definition_id.as_str(),
                        item.material_id.as_str(),
                        quality_label(item.quality),
                        item.durability_basis_points / 100
                    ),
                    style: Lai68PixelStyle::Quality(quality_ordinal(item.quality)),
                    reported_art_key: None,
                    construction_overlay: None,
                };
                marker.semantic_id = semantic_id(&marker.key);
                builder.markers.entry(marker.key.clone()).or_insert(marker);
            }
        }
    }
    for lot in &zone.lots {
        if placed_lots.contains(lot.cargo_id.as_str()) {
            continue;
        }
        if let Some(location_tile) = &lot.location_tile {
            project_storage_lot_marker(lot, Lai68Tile::from(location_tile), builder);
        } else {
            builder
                .unavailable
                .insert(Lai68UnavailableField::StorageLotTile {
                    lot_id: lot.cargo_id.as_str().to_owned(),
                });
        }
    }
    for item in exact_items
        .iter()
        .filter(|item| item.location_site_id == zone.zone_id)
    {
        if !placed_items.contains(item.item_id.as_str()) {
            builder
                .unavailable
                .insert(Lai68UnavailableField::StorageItemTile {
                    item_id: item.item_id.as_str().to_owned(),
                });
        }
    }
}

fn project_storage_lot_marker(
    lot: &PhysicalCargoSnapshot,
    tile: Lai68Tile,
    builder: &mut Lai68ProjectionBuilder,
) {
    let id = lot.cargo_id.as_str();
    let role = Lai68RenderMarkerRole::StorageLot {
        lot_id: id.to_owned(),
    };
    let mut marker = Lai68WorldMarker {
        key: Lai68RenderKey {
            layer: role.layer(),
            subject_id: id.to_owned(),
            role: role.clone(),
            tile,
        },
        role,
        tile,
        semantic_id: String::new(),
        tooltip: format!(
            "Reported {} ×{}; quality band {}; provenance {}",
            lot.content_id.as_str(),
            lot.quantity,
            lot.quality_band,
            lot.provenance_id.as_str()
        ),
        style: Lai68PixelStyle::Quality(lot.quality_band),
        reported_art_key: None,
        construction_overlay: None,
    };
    marker.semantic_id = semantic_id(&marker.key);
    builder.markers.entry(marker.key.clone()).or_insert(marker);
}

fn project_residences_and_families(
    colony: &CanonicalColonySnapshot,
    builder: &mut Lai68ProjectionBuilder,
) {
    let cats = colony
        .cats
        .iter()
        .map(|cat| (cat.cat_id.as_str(), cat))
        .collect::<BTreeMap<_, _>>();
    for residence in &colony.residences {
        let residence_id = residence.residence_id.as_str();
        let tiles = sorted_tiles(&residence.footprint);
        for (index, tile) in tiles.iter().copied().enumerate() {
            builder.marker(
                residence_id,
                Lai68RenderMarkerRole::ResidenceFootprint {
                    residence_id: residence_id.to_owned(),
                    cell_index: u16::try_from(index).expect("canonical residence footprint bound"),
                },
                tile,
                format!(
                    "Reported {} residence: {} of {} beds occupied",
                    residence.housing_kind_id.as_str(),
                    residence.resident_cat_ids.len(),
                    residence.capacity
                ),
                None,
                None,
            );
        }
        builder
            .unavailable
            .insert(Lai68UnavailableField::ResidenceArtKey {
                residence_id: residence_id.to_owned(),
            });

        let households = residence
            .resident_cat_ids
            .iter()
            .filter_map(|cat_id| cats.get(cat_id.as_str()))
            .filter_map(|cat| {
                (cat.family.residence_id.as_ref() == Some(&residence.residence_id))
                    .then(|| cat.family.household_id.as_ref().map(|id| id.as_str()))
                    .flatten()
            })
            .collect::<BTreeSet<_>>();
        for household_id in households {
            for (index, tile) in tiles.iter().copied().enumerate() {
                builder.marker(
                    household_id,
                    Lai68RenderMarkerRole::FamilyResidence {
                        household_id: household_id.to_owned(),
                        cell_index: u16::try_from(index)
                            .expect("canonical residence footprint bound"),
                    },
                    tile,
                    format!("Reported household {} at this residence", household_id),
                    None,
                    None,
                );
            }
        }
    }
    for cat in &colony.cats {
        if let Some(enterprise_id) = &cat.family.enterprise_id {
            builder
                .unavailable
                .insert(Lai68UnavailableField::EnterpriseWorldLocation {
                    enterprise_id: enterprise_id.as_str().to_owned(),
                });
        }
    }
}

fn project_hunting_sites(colony: &CanonicalColonySnapshot, builder: &mut Lai68ProjectionBuilder) {
    for site in &colony.hunting_sites {
        let site_id = site.site_id.as_str();
        builder.marker(
            site_id,
            Lai68RenderMarkerRole::HuntLair {
                site_id: site_id.to_owned(),
            },
            Lai68Tile::from(&site.tile),
            format!(
                "Reported {} lair, level band {}",
                site.site_kind_id.as_str(),
                site.level_band
            ),
            Some(site.art_key.as_str().to_owned()),
            None,
        );
    }
}

fn project_fishing_huts(colony: &CanonicalColonySnapshot, builder: &mut Lai68ProjectionBuilder) {
    for hut in &colony.fishing_huts {
        let hut_id = hut.hut_id.as_str();
        for (index, tile) in sorted_tiles(&hut.footprint).into_iter().enumerate() {
            builder.marker(
                hut_id,
                Lai68RenderMarkerRole::FishingHutFootprint {
                    hut_id: hut_id.to_owned(),
                    cell_index: u8::try_from(index)
                        .expect("canonical fishing hut footprint is nine cells"),
                },
                tile,
                format!("Reported Fishing Hut footprint cell {}", index + 1),
                Some(hut.art_key.as_str().to_owned()),
                None,
            );
        }
        builder.marker(
            hut_id,
            Lai68RenderMarkerRole::FishingDock {
                hut_id: hut_id.to_owned(),
            },
            Lai68Tile::from(&hut.dock_land_tile),
            "Reported Fishing Hut dock-facing land tile".to_owned(),
            Some(hut.art_key.as_str().to_owned()),
            None,
        );
        builder.marker(
            hut_id,
            Lai68RenderMarkerRole::FishingWaterAttachment {
                hut_id: hut_id.to_owned(),
            },
            Lai68Tile::from(&hut.reserved_water_tile),
            "Reported Fishing Hut reserved water attachment".to_owned(),
            Some(hut.art_key.as_str().to_owned()),
            None,
        );
    }
}

fn project_reported_visual_states(
    colony: &CanonicalColonySnapshot,
    builder: &mut Lai68ProjectionBuilder,
) {
    for state in &colony.visual_states {
        let subject_id = state.subject_id.as_str();
        for (index, tile) in sorted_tiles(&state.footprint).into_iter().enumerate() {
            builder.marker(
                subject_id,
                Lai68RenderMarkerRole::ReportedVisualState {
                    subject_id: subject_id.to_owned(),
                    cell_index: u16::try_from(index).expect("canonical visual footprint bound"),
                },
                tile,
                state.accessibility_label.as_str().to_owned(),
                Some(state.art_key.as_str().to_owned()),
                None,
            );
        }
    }
}

fn sorted_tiles(footprint: &Footprint) -> Vec<Lai68Tile> {
    let mut tiles = footprint
        .ordered_tiles
        .iter()
        .map(Lai68Tile::from)
        .collect::<Vec<_>>();
    tiles.sort_by_key(|tile| (tile.y, tile.x));
    tiles
}

fn is_exact_three_by_three(footprint: &Footprint) -> bool {
    if footprint.ordered_tiles.len() != 9 {
        return false;
    }
    let tiles = sorted_tiles(footprint);
    let min_x = tiles.first().map_or(0, |tile| tile.x);
    let min_y = tiles.iter().map(|tile| tile.y).min().unwrap_or_default();
    let expected = (0..3)
        .flat_map(|dy| {
            (0..3).map(move |dx| Lai68Tile {
                x: min_x + dx,
                y: min_y + dy,
            })
        })
        .collect::<Vec<_>>();
    tiles == expected
}

fn semantic_id(key: &Lai68RenderKey) -> String {
    let role = key.role.stable_name();
    let subject = stable_slug(&key.subject_id);
    format!(
        "lai68:world:{}:{}:{}:{}:{}",
        role, subject, key.tile.x, key.tile.y, key.layer as u8
    )
}

fn stable_slug(value: &str) -> String {
    let mut slug = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() {
            slug.push(byte.to_ascii_lowercase() as char);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    const MAX_BYTES: usize = 40;
    if slug.len() <= MAX_BYTES {
        return slug;
    }
    let hash = value.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        hash.wrapping_mul(0x0000_0100_0000_01b3) ^ u64::from(byte)
    });
    slug.truncate(MAX_BYTES - 17);
    format!("{slug}-{hash:016x}")
}

fn quality_ordinal(quality: QualityBandSnapshot) -> u8 {
    match quality {
        QualityBandSnapshot::Crude => 0,
        QualityBandSnapshot::Common => 1,
        QualityBandSnapshot::Fine => 2,
        QualityBandSnapshot::Superior => 3,
        QualityBandSnapshot::Masterwork => 4,
    }
}

fn quality_label(quality: QualityBandSnapshot) -> &'static str {
    match quality {
        QualityBandSnapshot::Crude => "crude",
        QualityBandSnapshot::Common => "common",
        QualityBandSnapshot::Fine => "fine",
        QualityBandSnapshot::Superior => "superior",
        QualityBandSnapshot::Masterwork => "masterwork",
    }
}
