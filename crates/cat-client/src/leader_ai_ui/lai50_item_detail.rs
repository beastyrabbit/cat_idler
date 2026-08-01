//! LAI.50 report-safe physical item, bulk-lot, and rare-material inspector.
//!
//! This leaf consumes only the canonical protocol-v3/schema-v2 selected-colony
//! report. It never reads simulation authority, invents an item silhouette,
//! substitutes generic art, derives hidden crafting value, or treats a missing
//! manifest entry as a known definition. Quality and augmentation are text and
//! effect details in this first implementation; they are deliberately not icon
//! frames or overlays.

use accesskit::{Action, Role};
use bevy::a11y::{AccessibilityNode, ActionRequest as AccessibilityActionRequest};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use cat_protocol::{
    PROTOCOL_VERSION,
    lai64::{
        AugmentationSnapshotV2, CANONICAL_SNAPSHOT_SCHEMA_VERSION, CanonicalColonySnapshot,
        CanonicalSnapshotEnvelope, ContentManifestEntrySnapshot, ExactItemSnapshotV2,
        QualityBandSnapshot, QualityLotSnapshotV2, RareMaterialSnapshotV2,
    },
};

use super::{
    art_assets::{Lai68ArtCategory, resolve_lai68_art_key},
    semantic_node, semantic_status_node,
};

const INK: Color = Color::srgb(0.153, 0.106, 0.086);
const MUTED_INK: Color = Color::srgb(0.34, 0.27, 0.22);
const PARCHMENT: Color = Color::srgb(0.937, 0.886, 0.741);
const PAPER_SHADE: Color = Color::srgb(0.866, 0.792, 0.635);
const DARK_FOREST: Color = Color::srgb(0.090, 0.235, 0.180);
const WOOD: Color = Color::srgb(0.427, 0.282, 0.169);
const STONE: Color = Color::srgb(0.48, 0.46, 0.39);
const MOSS: Color = Color::srgb(0.310, 0.439, 0.251);

pub const LAI50_ITEM_DETAIL_ROOT_ID: &str = "lai50:item-detail:panel";
pub const LAI50_ITEM_DETAIL_BODY_ID: &str = "lai50:item-detail:body";
pub const LAI50_REFRESH_CONTROL_ID: &str = "lai50:item-detail:refresh";
pub const LAI50_CLOSE_CONTROL_ID: &str = "lai50:item-detail:close";

/// The inspector is a product-normal ledger page: parchment content, wood
/// rules, and a dark-forest outer worktable without glass, glow, or KPI tiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lai50VisualDirection {
    pub parchment_content: bool,
    pub wood_rules: bool,
    pub dark_forest_worktable: bool,
    pub uses_glass: bool,
    pub uses_glow: bool,
    pub uses_quality_frames: bool,
    pub uses_augmentation_overlays: bool,
}

pub const LAI50_VISUAL_DIRECTION: Lai50VisualDirection = Lai50VisualDirection {
    parchment_content: true,
    wood_rules: true,
    dark_forest_worktable: true,
    uses_glass: false,
    uses_glow: false,
    uses_quality_frames: false,
    uses_augmentation_overlays: false,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Lai50RefreshState {
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

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai50SnapshotFeed {
    pub envelope: Option<CanonicalSnapshotEnvelope>,
    pub refresh: Lai50RefreshState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lai50DetailSelection {
    ExactItem(String),
    BulkLot(String),
    RareMaterial(String),
}

impl Lai50DetailSelection {
    fn semantic_suffix(&self) -> (&'static str, &str) {
        match self {
            Self::ExactItem(id) => ("item", id),
            Self::BulkLot(id) => ("lot", id),
            Self::RareMaterial(id) => ("rare-material", id),
        }
    }
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai50ViewState {
    pub selection: Option<Lai50DetailSelection>,
    pub focused_control_id: Option<String>,
    pub refresh_requests: u64,
}

/// The owning screen opens this inspector after selecting an exact item, lot,
/// or rare material. It is hidden until then.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lai50PanelVisibility {
    pub visible: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Lai50SurfaceState {
    #[default]
    Loading,
    Ready,
    Stale {
        stale_since_ms: i64,
    },
    Unavailable {
        reason: String,
    },
    UpdateRequired,
    Error {
        message: String,
    },
}

impl Lai50SurfaceState {
    #[must_use]
    pub const fn keeps_report_visible(&self) -> bool {
        matches!(self, Self::Ready | Self::Stale { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lai50Availability<T> {
    Reported(T),
    Unavailable { reason: String },
}

impl<T> Lai50Availability<T> {
    #[must_use]
    pub fn as_ref(&self) -> Lai50Availability<&T> {
        match self {
            Self::Reported(value) => Lai50Availability::Reported(value),
            Self::Unavailable { reason } => Lai50Availability::Unavailable {
                reason: reason.clone(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportedDefinition {
    pub definition_id: String,
    pub display_name: String,
    pub content_kind_id: String,
    pub art_key: String,
    pub accessibility_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactMaterialIcon {
    pub material_id: String,
    pub display_name: String,
    pub art_key: String,
    pub asset_path: &'static str,
    pub native_width_px: u16,
    pub native_height_px: u16,
    pub accessibility_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReportedReservation {
    Unreserved,
    Reserved { reservation_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportedAugmentation {
    pub augmentation_instance_id: String,
    pub augmentation_id: String,
    pub definition: Lai50Availability<ReportedDefinition>,
    pub material_instance_id: String,
    pub installed: bool,
    pub typed_effect: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactItemDetail {
    pub item_id: String,
    pub semantic_id: String,
    pub display_title: String,
    pub definition: Lai50Availability<ReportedDefinition>,
    pub material: Lai50Availability<ReportedDefinition>,
    pub material_icon: Lai50Availability<ExactMaterialIcon>,
    pub silhouette_art: Lai50Availability<String>,
    pub quality: QualityBandSnapshot,
    pub durability_basis_points: u16,
    pub augmentations: Lai50Availability<Vec<ReportedAugmentation>>,
    pub provenance_id: String,
    pub reservation: ReportedReservation,
    pub location_site_id: String,
    pub quality_is_text_only: bool,
    pub augmentation_is_detail_only: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BulkLotDetail {
    pub lot_id: String,
    pub semantic_id: String,
    pub content_id: String,
    pub definition: Lai50Availability<ReportedDefinition>,
    pub quantity: u64,
    pub quality: QualityBandSnapshot,
    pub age_ms: u64,
    pub provenance_id: String,
    pub reservation: ReportedReservation,
    pub location_site_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RareMaterialDetail {
    pub material_instance_id: String,
    pub semantic_id: String,
    pub material_id: String,
    pub definition: Lai50Availability<ReportedDefinition>,
    pub material_icon: Lai50Availability<ExactMaterialIcon>,
    pub content_state_id: String,
    pub processed: bool,
    pub quality: QualityBandSnapshot,
    pub provenance_id: String,
    pub reservation: ReportedReservation,
    pub location_site_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lai50Detail {
    ExactItem(ExactItemDetail),
    BulkLot(BulkLotDetail),
    RareMaterial(RareMaterialDetail),
}

impl Lai50Detail {
    #[must_use]
    pub fn semantic_id(&self) -> &str {
        match self {
            Self::ExactItem(detail) => &detail.semantic_id,
            Self::BulkLot(detail) => &detail.semantic_id,
            Self::RareMaterial(detail) => &detail.semantic_id,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai50ItemDetailProjection {
    pub state: Lai50SurfaceState,
    pub selected_colony_id: Option<String>,
    pub state_version: Option<u64>,
    pub snapshot_now_ms: Option<i64>,
    pub detail: Option<Lai50Detail>,
    pub reads_authoritative_world_truth: bool,
    pub invents_generic_art: bool,
    pub renders_quality_icon_frame: bool,
    pub renders_augmentation_icon_overlay: bool,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai50ProjectionResource(pub Lai50ItemDetailProjection);

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
struct Lai50RenderState {
    dirty: bool,
}

impl Default for Lai50RenderState {
    fn default() -> Self {
        Self { dirty: true }
    }
}

#[derive(Component)]
pub struct Lai50ItemDetailRoot;

#[derive(Component)]
pub struct Lai50ItemDetailBody;

#[derive(Component)]
pub struct Lai50ItemDetailStatus;

#[derive(Component)]
pub struct Lai50Scrollable;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lai50DetailBadge {
    Quality(QualityBandSnapshot),
    Augmentation,
    RawMaterial,
    ProcessedMaterial,
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct Lai50StableUiId(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lai50ControlAction {
    Refresh,
    Close,
}

/// Typed presentation event consumed by the LAI.50 route bridge. Clearing the
/// selection alone is not a route authority because an explicitly opened,
/// unselected inspector must still be able to render its unavailable state.
#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lai50ItemDetailCloseRequested;

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct Lai50Control {
    pub stable_id: String,
    pub action: Lai50ControlAction,
}

/// Projects the selected canonical report and rejects every non-v3/schema-v2,
/// malformed, foreign, missing, or unreported path visibly.
#[must_use]
pub fn project_lai50_item_detail(
    feed: &Lai50SnapshotFeed,
    view: &Lai50ViewState,
) -> Lai50ItemDetailProjection {
    let mut projection = Lai50ItemDetailProjection::default();
    let Some(envelope) = feed.envelope.as_ref() else {
        projection.state = refresh_without_report(&feed.refresh);
        return projection;
    };
    projection.selected_colony_id = Some(envelope.selected_colony_id.as_str().to_owned());
    projection.snapshot_now_ms = Some(envelope.now_ms);

    if envelope.protocol_version != PROTOCOL_VERSION
        || envelope.snapshot_schema_version != CANONICAL_SNAPSHOT_SCHEMA_VERSION
    {
        projection.state = Lai50SurfaceState::UpdateRequired;
        return projection;
    }
    if let Err(error) = envelope.validate() {
        projection.state = Lai50SurfaceState::Error {
            message: format!("Canonical item report rejected: {error}"),
        };
        return projection;
    }
    let Some(colony) = envelope
        .colonies
        .iter()
        .find(|colony| colony.colony_id == envelope.selected_colony_id)
    else {
        projection.state = Lai50SurfaceState::Error {
            message: "Selected colony report is unavailable.".to_owned(),
        };
        return projection;
    };
    projection.state_version = Some(colony.state_version);

    let Some(selection) = view.selection.as_ref() else {
        projection.state = Lai50SurfaceState::Unavailable {
            reason: "Select a reported item, lot, or named material.".to_owned(),
        };
        return projection;
    };
    projection.detail = project_reported_selection(colony, selection);
    if projection.detail.is_none() {
        let (kind, id) = selection.semantic_suffix();
        projection.state = Lai50SurfaceState::Unavailable {
            reason: format!("The selected {kind} “{id}” is not present in this report."),
        };
        return projection;
    }
    projection.state = match &feed.refresh {
        Lai50RefreshState::Loading | Lai50RefreshState::Ready => Lai50SurfaceState::Ready,
        Lai50RefreshState::Stale { stale_since_ms } => Lai50SurfaceState::Stale {
            stale_since_ms: *stale_since_ms,
        },
        Lai50RefreshState::UpdateRequired => Lai50SurfaceState::UpdateRequired,
        Lai50RefreshState::Error { message } => Lai50SurfaceState::Error {
            message: message.clone(),
        },
    };
    projection
}

fn refresh_without_report(refresh: &Lai50RefreshState) -> Lai50SurfaceState {
    match refresh {
        Lai50RefreshState::Loading | Lai50RefreshState::Ready => Lai50SurfaceState::Loading,
        Lai50RefreshState::Stale { stale_since_ms } => Lai50SurfaceState::Stale {
            stale_since_ms: *stale_since_ms,
        },
        Lai50RefreshState::UpdateRequired => Lai50SurfaceState::UpdateRequired,
        Lai50RefreshState::Error { message } => Lai50SurfaceState::Error {
            message: message.clone(),
        },
    }
}

#[must_use]
pub fn project_reported_selection(
    colony: &CanonicalColonySnapshot,
    selection: &Lai50DetailSelection,
) -> Option<Lai50Detail> {
    match selection {
        Lai50DetailSelection::ExactItem(item_id) => colony
            .exact_items
            .iter()
            .find(|item| item.item_id.as_str() == item_id)
            .map(|item| Lai50Detail::ExactItem(project_exact_item(colony, item))),
        Lai50DetailSelection::BulkLot(lot_id) => colony
            .quality_lots
            .iter()
            .find(|lot| lot.lot_id.as_str() == lot_id)
            .map(|lot| Lai50Detail::BulkLot(project_bulk_lot(colony, lot))),
        Lai50DetailSelection::RareMaterial(material_instance_id) => colony
            .rare_materials
            .iter()
            .find(|material| material.material_instance_id.as_str() == material_instance_id)
            .map(|material| Lai50Detail::RareMaterial(project_rare_material(colony, material))),
    }
}

fn project_exact_item(
    colony: &CanonicalColonySnapshot,
    item: &ExactItemSnapshotV2,
) -> ExactItemDetail {
    let definition = reported_definition(colony, item.definition_id.as_str());
    let material = reported_definition(colony, item.material_id.as_str());
    let material_icon = exact_material_icon(colony, item.material_id.as_str(), true);
    let silhouette_art = match definition.as_ref() {
        Lai50Availability::Reported(definition) => Lai50Availability::Unavailable {
            reason: format!(
                "Exact item silhouette asset “{}” has no delivered item-silhouette catalog entry.",
                definition.art_key
            ),
        },
        Lai50Availability::Unavailable { reason } => Lai50Availability::Unavailable {
            reason: reason.clone(),
        },
    };
    let mut projected_augmentations = Vec::new();
    let mut missing_augmentation_ids = Vec::new();
    for augmentation_id in &item.augmentation_ids {
        if let Some(augmentation) = colony.augmentations.iter().find(|candidate| {
            candidate.augmentation_instance_id == *augmentation_id
                && candidate.target_item_id == item.item_id
        }) {
            projected_augmentations.push(project_augmentation(colony, augmentation));
        } else {
            missing_augmentation_ids.push(augmentation_id.as_str());
        }
    }
    projected_augmentations.sort_by(|left, right| {
        left.augmentation_instance_id
            .cmp(&right.augmentation_instance_id)
    });
    let augmentations = if missing_augmentation_ids.is_empty() {
        Lai50Availability::Reported(projected_augmentations)
    } else {
        Lai50Availability::Unavailable {
            reason: format!(
                "Referenced augmentation reports are missing: {}",
                missing_augmentation_ids.join(", ")
            ),
        }
    };
    let display_title = format!(
        "{} {} {}",
        quality_label(item.quality),
        definition_name(&material, item.material_id.as_str()),
        definition_name(&definition, item.definition_id.as_str())
    );
    ExactItemDetail {
        item_id: item.item_id.as_str().to_owned(),
        semantic_id: format!("lai50:item:{}", item.item_id.as_str()),
        display_title,
        definition,
        material,
        material_icon,
        silhouette_art,
        quality: item.quality,
        durability_basis_points: item.durability_basis_points,
        augmentations,
        provenance_id: item.provenance_id.as_str().to_owned(),
        reservation: reported_reservation(item.reservation_id.as_ref().map(|id| id.as_str())),
        location_site_id: item.location_site_id.as_str().to_owned(),
        quality_is_text_only: true,
        augmentation_is_detail_only: true,
    }
}

fn project_augmentation(
    colony: &CanonicalColonySnapshot,
    augmentation: &AugmentationSnapshotV2,
) -> ReportedAugmentation {
    ReportedAugmentation {
        augmentation_instance_id: augmentation.augmentation_instance_id.as_str().to_owned(),
        augmentation_id: augmentation.augmentation_id.as_str().to_owned(),
        definition: reported_definition(colony, augmentation.augmentation_id.as_str()),
        material_instance_id: augmentation.material_instance_id.as_str().to_owned(),
        installed: augmentation.installed,
        typed_effect: augmentation.effect_summary.as_str().to_owned(),
    }
}

fn project_bulk_lot(colony: &CanonicalColonySnapshot, lot: &QualityLotSnapshotV2) -> BulkLotDetail {
    BulkLotDetail {
        lot_id: lot.lot_id.as_str().to_owned(),
        semantic_id: format!("lai50:lot:{}", lot.lot_id.as_str()),
        content_id: lot.content_id.as_str().to_owned(),
        definition: reported_definition(colony, lot.content_id.as_str()),
        quantity: lot.quantity,
        quality: lot.quality,
        age_ms: lot.age_ms,
        provenance_id: lot.provenance_id.as_str().to_owned(),
        reservation: reported_reservation(lot.reservation_id.as_ref().map(|id| id.as_str())),
        location_site_id: lot.location_site_id.as_str().to_owned(),
    }
}

fn project_rare_material(
    colony: &CanonicalColonySnapshot,
    material: &RareMaterialSnapshotV2,
) -> RareMaterialDetail {
    RareMaterialDetail {
        material_instance_id: material.material_instance_id.as_str().to_owned(),
        semantic_id: format!(
            "lai50:rare-material:{}",
            material.material_instance_id.as_str()
        ),
        material_id: material.material_id.as_str().to_owned(),
        definition: reported_definition(colony, material.material_id.as_str()),
        material_icon: exact_material_icon(colony, material.material_id.as_str(), false),
        content_state_id: material.content_state_id.as_str().to_owned(),
        processed: material.processed,
        quality: material.quality,
        provenance_id: material.provenance_id.as_str().to_owned(),
        reservation: reported_reservation(material.reservation_id.as_ref().map(|id| id.as_str())),
        location_site_id: material.location_site_id.as_str().to_owned(),
    }
}

fn reported_definition(
    colony: &CanonicalColonySnapshot,
    definition_id: &str,
) -> Lai50Availability<ReportedDefinition> {
    let Some(manifest) = colony.content_manifest.as_ref() else {
        return Lai50Availability::Unavailable {
            reason: "Content manifest was not included in this report.".to_owned(),
        };
    };
    let Some(entry) = manifest
        .entries
        .iter()
        .find(|entry| entry.content_id.as_str() == definition_id)
    else {
        return Lai50Availability::Unavailable {
            reason: format!("Definition “{definition_id}” is absent from the reported manifest."),
        };
    };
    Lai50Availability::Reported(definition_from_entry(entry))
}

fn definition_from_entry(entry: &ContentManifestEntrySnapshot) -> ReportedDefinition {
    ReportedDefinition {
        definition_id: entry.content_id.as_str().to_owned(),
        display_name: entry.display_name.as_str().to_owned(),
        content_kind_id: entry.content_kind_id.as_str().to_owned(),
        art_key: entry.art_key.as_str().to_owned(),
        accessibility_label: entry.accessibility_label.as_str().to_owned(),
    }
}

fn exact_material_icon(
    colony: &CanonicalColonySnapshot,
    material_id: &str,
    allow_resource_material_layer: bool,
) -> Lai50Availability<ExactMaterialIcon> {
    let definition = reported_definition(colony, material_id);
    let Lai50Availability::Reported(definition) = definition else {
        let Lai50Availability::Unavailable { reason } = definition else {
            unreachable!()
        };
        return Lai50Availability::Unavailable { reason };
    };
    let Some(asset) = resolve_lai68_art_key(&definition.art_key) else {
        return Lai50Availability::Unavailable {
            reason: format!(
                "Exact material art “{}” is not delivered; no generic icon was substituted.",
                definition.art_key
            ),
        };
    };
    if asset.category != Lai68ArtCategory::MaterialIcon
        && !(allow_resource_material_layer && asset.category == Lai68ArtCategory::ResourceIcon)
    {
        return Lai50Availability::Unavailable {
            reason: format!(
                "Reported art “{}” is not a material icon.",
                definition.art_key
            ),
        };
    }
    Lai50Availability::Reported(ExactMaterialIcon {
        material_id: material_id.to_owned(),
        display_name: definition.display_name,
        art_key: definition.art_key,
        asset_path: asset.path,
        native_width_px: asset.native_width_px,
        native_height_px: asset.native_height_px,
        accessibility_label: definition.accessibility_label,
    })
}

fn reported_reservation(reservation_id: Option<&str>) -> ReportedReservation {
    reservation_id.map_or(ReportedReservation::Unreserved, |reservation_id| {
        ReportedReservation::Reserved {
            reservation_id: reservation_id.to_owned(),
        }
    })
}

#[derive(Default)]
pub struct Lai50ItemDetailPlugin;

impl Plugin for Lai50ItemDetailPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Lai50SnapshotFeed>()
            .init_resource::<Lai50ViewState>()
            .init_resource::<Lai50PanelVisibility>()
            .init_resource::<Lai50ProjectionResource>()
            .init_resource::<Lai50RenderState>()
            .add_message::<MouseWheel>()
            .add_message::<AccessibilityActionRequest>()
            .add_message::<Lai50ItemDetailCloseRequested>()
            .add_systems(
                Update,
                (
                    attach_lai50_item_detail,
                    sync_lai50_projection,
                    sync_lai50_visibility,
                    render_lai50_projection,
                    handle_lai50_pointer,
                    handle_lai50_keyboard,
                    handle_lai50_accessibility,
                    sync_lai50_focus_style,
                    sync_lai50_responsive_layout,
                    handle_lai50_scroll,
                )
                    .chain(),
            );
    }
}

fn attach_lai50_item_detail(
    mut commands: Commands<'_, '_>,
    existing: Query<'_, '_, Entity, With<Lai50ItemDetailRoot>>,
) {
    if !existing.is_empty() {
        return;
    }
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(520.0),
                max_width: Val::Percent(96.0),
                height: Val::Percent(78.0),
                max_height: Val::Px(760.0),
                right: Val::Px(24.0),
                top: Val::Px(82.0),
                padding: UiRect::all(Val::Px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                border: UiRect::all(Val::Px(2.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            GlobalZIndex(1_350),
            BackgroundColor(DARK_FOREST),
            BorderColor::all(WOOD),
            Visibility::Hidden,
            Lai50ItemDetailRoot,
            Lai50StableUiId(LAI50_ITEM_DETAIL_ROOT_ID.to_owned()),
            crate::WorldInputBlocker,
            semantic_node(
                Role::Pane,
                LAI50_ITEM_DETAIL_ROOT_ID,
                "Physical item detail",
                true,
            ),
            Name::new("LAI.50 report-safe item detail"),
        ))
        .id();
    commands.entity(root).with_children(|panel| {
        panel
            .spawn(Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(42.0),
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|header| {
                header.spawn((
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                    text_bundle("Physical inventory detail", 22.0, PARCHMENT),
                ));
                spawn_control(
                    header,
                    LAI50_REFRESH_CONTROL_ID,
                    "Refresh",
                    Lai50ControlAction::Refresh,
                );
                spawn_control(
                    header,
                    LAI50_CLOSE_CONTROL_ID,
                    "Close",
                    Lai50ControlAction::Close,
                );
            });
        panel.spawn((
            text_bundle("Loading canonical item report", 13.0, PAPER_SHADE),
            Lai50ItemDetailStatus,
            semantic_status_node(
                "lai50:item-detail:status",
                "Loading canonical item report",
                false,
            ),
        ));
        panel.spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(180.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                border: UiRect::all(Val::Px(1.0)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            Interaction::default(),
            ScrollPosition::default(),
            BackgroundColor(PARCHMENT),
            BorderColor::all(WOOD),
            Lai50ItemDetailBody,
            Lai50Scrollable,
            Lai50StableUiId(LAI50_ITEM_DETAIL_BODY_ID.to_owned()),
            semantic_node(
                Role::GenericContainer,
                LAI50_ITEM_DETAIL_BODY_ID,
                "Scrollable physical inventory details",
                true,
            ),
        ));
    });
}

fn sync_lai50_visibility(
    panel: Res<'_, Lai50PanelVisibility>,
    mut root: Query<'_, '_, &mut Visibility, With<Lai50ItemDetailRoot>>,
) {
    if !panel.is_changed() {
        return;
    }
    for mut visibility in &mut root {
        *visibility = if panel.visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn spawn_control(
    parent: &mut ChildSpawnerCommands<'_>,
    stable_id: &str,
    label: &str,
    action: Lai50ControlAction,
) {
    parent
        .spawn((
            Button,
            Node {
                min_width: Val::Px(64.0),
                min_height: Val::Px(34.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(STONE),
            Lai50Control {
                stable_id: stable_id.to_owned(),
                action,
            },
            Lai50StableUiId(stable_id.to_owned()),
            semantic_node(Role::Button, stable_id, label, true),
            Name::new(format!("LAI.50 {label} control")),
        ))
        .with_child(text_bundle(label, 13.0, PARCHMENT));
}

fn sync_lai50_projection(
    feed: Res<'_, Lai50SnapshotFeed>,
    view: Res<'_, Lai50ViewState>,
    mut projection: ResMut<'_, Lai50ProjectionResource>,
    mut render: ResMut<'_, Lai50RenderState>,
) {
    if feed.is_changed() || view.is_changed() {
        projection.0 = project_lai50_item_detail(&feed, &view);
        render.dirty = true;
    }
}

fn render_lai50_projection(
    mut commands: Commands<'_, '_>,
    assets: Option<Res<'_, AssetServer>>,
    projection: Res<'_, Lai50ProjectionResource>,
    view: Res<'_, Lai50ViewState>,
    mut render: ResMut<'_, Lai50RenderState>,
    body: Query<'_, '_, Entity, With<Lai50ItemDetailBody>>,
    mut status: Query<'_, '_, (&mut Text, &mut AccessibilityNode), With<Lai50ItemDetailStatus>>,
    mut root: Query<'_, '_, &mut Node, With<Lai50ItemDetailRoot>>,
) {
    if !render.dirty {
        return;
    }
    let status_copy = surface_state_copy(&projection.0.state);
    if let Ok(mut root) = root.single_mut() {
        root.display = if view.selection.is_some() {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Ok((mut text, mut accessibility)) = status.single_mut() {
        text.0.clone_from(&status_copy);
        *accessibility = semantic_status_node(
            "lai50:item-detail:status",
            status_copy,
            matches!(
                projection.0.state,
                Lai50SurfaceState::Error { .. } | Lai50SurfaceState::UpdateRequired
            ),
        );
    }
    let Ok(body) = body.single() else {
        return;
    };
    commands.entity(body).despawn_children();
    if let Some(detail) = projection.0.detail.as_ref()
        && projection.0.state.keeps_report_visible()
    {
        render_detail(&mut commands, body, detail, assets.as_deref());
    } else {
        spawn_section(
            &mut commands,
            body,
            "Item report",
            &surface_state_copy(&projection.0.state),
        );
    }
    render.dirty = false;
}

fn render_detail(
    commands: &mut Commands<'_, '_>,
    parent: Entity,
    detail: &Lai50Detail,
    assets: Option<&AssetServer>,
) {
    match detail {
        Lai50Detail::ExactItem(item) => render_exact_item(commands, parent, item, assets),
        Lai50Detail::BulkLot(lot) => render_bulk_lot(commands, parent, lot),
        Lai50Detail::RareMaterial(material) => {
            render_rare_material(commands, parent, material, assets);
        }
    }
}

fn render_exact_item(
    commands: &mut Commands<'_, '_>,
    parent: Entity,
    item: &ExactItemDetail,
    assets: Option<&AssetServer>,
) {
    spawn_icon_and_identity(
        commands,
        parent,
        &item.material_icon,
        assets,
        item.display_title.clone(),
        &item.semantic_id,
    );
    spawn_fact(commands, parent, "Exact item ID", &item.item_id);
    spawn_fact(
        commands,
        parent,
        "Definition",
        &availability_definition(&item.definition),
    );
    spawn_fact(
        commands,
        parent,
        "Material icon layer",
        &availability_material_icon(&item.material_icon),
    );
    spawn_fact(
        commands,
        parent,
        "Item silhouette",
        &availability_text(&item.silhouette_art),
    );
    spawn_badge(
        commands,
        parent,
        "Quality",
        quality_label(item.quality),
        Lai50DetailBadge::Quality(item.quality),
    );
    spawn_fact(
        commands,
        parent,
        "Durability",
        &format_basis_points(item.durability_basis_points),
    );
    match &item.augmentations {
        Lai50Availability::Reported(augmentations) if augmentations.is_empty() => {
            spawn_fact(commands, parent, "Augmentation", "None reported");
        }
        Lai50Availability::Reported(augmentations) => {
            let copy = augmentations
                .iter()
                .map(|augmentation| {
                    format!(
                        "{} [{}] — {} — material {} — {}",
                        definition_name(&augmentation.definition, &augmentation.augmentation_id),
                        augmentation.augmentation_instance_id,
                        if augmentation.installed {
                            "installed"
                        } else {
                            "not installed"
                        },
                        augmentation.material_instance_id,
                        augmentation.typed_effect
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            spawn_badge(
                commands,
                parent,
                "Typed augmentation effect",
                &copy,
                Lai50DetailBadge::Augmentation,
            );
        }
        Lai50Availability::Unavailable { reason } => {
            spawn_fact(
                commands,
                parent,
                "Augmentation",
                &format!("Unavailable — {reason}"),
            );
        }
    }
    spawn_trace(
        commands,
        parent,
        &item.provenance_id,
        &item.location_site_id,
        &item.reservation,
    );
    spawn_section(
        commands,
        parent,
        "Visual contract",
        "Quality and augmentation are report text/effects only. No quality frame or augmentation overlay is rendered.",
    );
}

fn render_bulk_lot(commands: &mut Commands<'_, '_>, parent: Entity, lot: &BulkLotDetail) {
    spawn_section(
        commands,
        parent,
        &definition_name(&lot.definition, &lot.content_id),
        "Physical bulk lot",
    );
    spawn_fact(commands, parent, "Exact lot ID", &lot.lot_id);
    spawn_fact(commands, parent, "Content ID", &lot.content_id);
    spawn_fact(commands, parent, "Amount", &lot.quantity.to_string());
    spawn_badge(
        commands,
        parent,
        "Quality",
        quality_label(lot.quality),
        Lai50DetailBadge::Quality(lot.quality),
    );
    spawn_fact(commands, parent, "Reported age", &format_age(lot.age_ms));
    spawn_trace(
        commands,
        parent,
        &lot.provenance_id,
        &lot.location_site_id,
        &lot.reservation,
    );
}

fn render_rare_material(
    commands: &mut Commands<'_, '_>,
    parent: Entity,
    material: &RareMaterialDetail,
    assets: Option<&AssetServer>,
) {
    spawn_icon_and_identity(
        commands,
        parent,
        &material.material_icon,
        assets,
        definition_name(&material.definition, &material.material_id),
        &material.semantic_id,
    );
    spawn_fact(
        commands,
        parent,
        "Exact material instance ID",
        &material.material_instance_id,
    );
    spawn_fact(commands, parent, "Material ID", &material.material_id);
    spawn_fact(
        commands,
        parent,
        "Content state ID",
        &material.content_state_id,
    );
    spawn_badge(
        commands,
        parent,
        "Processing state",
        if material.processed {
            "Processed"
        } else {
            "Raw"
        },
        if material.processed {
            Lai50DetailBadge::ProcessedMaterial
        } else {
            Lai50DetailBadge::RawMaterial
        },
    );
    spawn_badge(
        commands,
        parent,
        "Quality",
        quality_label(material.quality),
        Lai50DetailBadge::Quality(material.quality),
    );
    spawn_fact(
        commands,
        parent,
        "Exact material icon",
        &availability_material_icon(&material.material_icon),
    );
    spawn_trace(
        commands,
        parent,
        &material.provenance_id,
        &material.location_site_id,
        &material.reservation,
    );
}

fn spawn_icon_and_identity(
    commands: &mut Commands<'_, '_>,
    parent: Entity,
    icon: &Lai50Availability<ExactMaterialIcon>,
    assets: Option<&AssetServer>,
    title: String,
    semantic_id: &str,
) {
    let row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(76.0),
                align_items: AlignItems::Center,
                column_gap: Val::Px(14.0),
                padding: UiRect::bottom(Val::Px(10.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(WOOD),
            semantic_node(Role::Group, semantic_id, title.clone(), true),
        ))
        .id();
    commands.entity(parent).add_child(row);
    if let (Lai50Availability::Reported(icon), Some(assets)) = (icon, assets) {
        let image = commands
            .spawn((
                Node {
                    width: Val::Px(64.0),
                    height: Val::Px(64.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                ImageNode::new(assets.load(icon.asset_path)),
                semantic_node(
                    Role::Image,
                    format!("{semantic_id}:material-icon"),
                    icon.accessibility_label.clone(),
                    true,
                ),
                Name::new(format!("LAI.50 exact material icon {}", icon.art_key)),
            ))
            .id();
        commands.entity(row).add_child(image);
    }
    let heading = commands
        .spawn((
            text_bundle(title, 22.0, INK),
            Node {
                flex_grow: 1.0,
                ..default()
            },
        ))
        .id();
    commands.entity(row).add_child(heading);
}

fn spawn_trace(
    commands: &mut Commands<'_, '_>,
    parent: Entity,
    provenance_id: &str,
    location_site_id: &str,
    reservation: &ReportedReservation,
) {
    spawn_section(
        commands,
        parent,
        "Physical trace",
        &format!(
            "Provenance: {provenance_id}\nLocation: {location_site_id}\nReservation: {}",
            reservation_label(reservation)
        ),
    );
}

fn spawn_fact(commands: &mut Commands<'_, '_>, parent: Entity, label: &str, value: &str) {
    let row = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            min_height: Val::Px(28.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(12.0),
            padding: UiRect::vertical(Val::Px(4.0)),
            border: UiRect::bottom(Val::Px(1.0)),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(row);
    let label = commands
        .spawn((
            Node {
                width: Val::Percent(34.0),
                ..default()
            },
            text_bundle(label, 13.0, MUTED_INK),
        ))
        .id();
    let value = commands
        .spawn((
            Node {
                width: Val::Percent(66.0),
                ..default()
            },
            text_bundle(value, 14.0, INK),
        ))
        .id();
    commands.entity(row).add_children(&[label, value]);
}

fn spawn_badge(
    commands: &mut Commands<'_, '_>,
    parent: Entity,
    label: &str,
    value: &str,
    badge: Lai50DetailBadge,
) {
    let row = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            min_height: Val::Px(32.0),
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            padding: UiRect::vertical(Val::Px(4.0)),
            border: UiRect::bottom(Val::Px(1.0)),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(row);
    let label = commands
        .spawn((
            Node {
                width: Val::Percent(34.0),
                ..default()
            },
            text_bundle(label, 13.0, MUTED_INK),
        ))
        .id();
    let badge = commands
        .spawn((
            Node {
                max_width: Val::Percent(66.0),
                padding: UiRect::axes(Val::Px(7.0), Val::Px(4.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(PAPER_SHADE),
            BorderColor::all(WOOD),
            badge,
            text_bundle(value, 13.0, INK),
        ))
        .id();
    commands.entity(row).add_children(&[label, badge]);
}

fn spawn_section(commands: &mut Commands<'_, '_>, parent: Entity, heading: &str, copy: &str) {
    let section = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(PAPER_SHADE),
            BorderColor::all(WOOD),
        ))
        .id();
    commands.entity(parent).add_child(section);
    let heading = commands.spawn(text_bundle(heading, 16.0, INK)).id();
    let copy = commands.spawn(text_bundle(copy, 13.0, MUTED_INK)).id();
    commands.entity(section).add_children(&[heading, copy]);
}

fn handle_lai50_pointer(
    controls: Query<'_, '_, (&Interaction, &Lai50Control), Changed<Interaction>>,
    mut view: ResMut<'_, Lai50ViewState>,
    mut close_requests: MessageWriter<'_, Lai50ItemDetailCloseRequested>,
) {
    for (interaction, control) in &controls {
        if *interaction == Interaction::Pressed {
            view.focused_control_id = Some(control.stable_id.clone());
            apply_lai50_action(&control.action, &mut view, &mut close_requests);
        }
    }
}

fn handle_lai50_keyboard(
    keys: Res<'_, ButtonInput<KeyCode>>,
    controls: Query<'_, '_, &Lai50Control>,
    mut view: ResMut<'_, Lai50ViewState>,
    mut close_requests: MessageWriter<'_, Lai50ItemDetailCloseRequested>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        view.selection = None;
        close_requests.write(Lai50ItemDetailCloseRequested);
        return;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        view.refresh_requests = view.refresh_requests.saturating_add(1);
    }
    let mut ordered = controls.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.stable_id.cmp(&right.stable_id));
    if ordered.is_empty() {
        return;
    }
    if keys.just_pressed(KeyCode::Tab)
        || keys.just_pressed(KeyCode::ArrowLeft)
        || keys.just_pressed(KeyCode::ArrowRight)
    {
        let backwards = keys.pressed(KeyCode::ShiftLeft)
            || keys.pressed(KeyCode::ShiftRight)
            || keys.just_pressed(KeyCode::ArrowLeft);
        let current = view
            .focused_control_id
            .as_ref()
            .and_then(|id| ordered.iter().position(|control| &control.stable_id == id));
        let next = match (current, backwards) {
            (None, false) => 0,
            (None, true) => ordered.len() - 1,
            (Some(0), true) => ordered.len() - 1,
            (Some(index), true) => index - 1,
            (Some(index), false) => (index + 1) % ordered.len(),
        };
        view.focused_control_id = Some(ordered[next].stable_id.clone());
    }
    if (keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space))
        && let Some(control) = view
            .focused_control_id
            .as_ref()
            .and_then(|id| ordered.iter().find(|control| &control.stable_id == id))
    {
        apply_lai50_action(&control.action, &mut view, &mut close_requests);
    }
}

fn handle_lai50_accessibility(
    mut requests: MessageReader<'_, '_, AccessibilityActionRequest>,
    controls: Query<'_, '_, &Lai50Control>,
    mut view: ResMut<'_, Lai50ViewState>,
    mut close_requests: MessageWriter<'_, Lai50ItemDetailCloseRequested>,
) {
    for request in requests.read() {
        let Some(entity) = Entity::try_from_bits(request.target_node.0) else {
            continue;
        };
        let Ok(control) = controls.get(entity) else {
            continue;
        };
        if matches!(request.action, Action::Focus | Action::Click) {
            view.focused_control_id = Some(control.stable_id.clone());
        }
        if request.action == Action::Click {
            apply_lai50_action(&control.action, &mut view, &mut close_requests);
        }
    }
}

fn apply_lai50_action(
    action: &Lai50ControlAction,
    view: &mut Lai50ViewState,
    close_requests: &mut MessageWriter<'_, Lai50ItemDetailCloseRequested>,
) {
    match action {
        Lai50ControlAction::Refresh => {
            view.refresh_requests = view.refresh_requests.saturating_add(1);
        }
        Lai50ControlAction::Close => {
            view.selection = None;
            close_requests.write(Lai50ItemDetailCloseRequested);
        }
    }
}

fn sync_lai50_focus_style(
    view: Res<'_, Lai50ViewState>,
    mut controls: Query<'_, '_, (&Lai50Control, &mut BackgroundColor, &mut BorderColor)>,
) {
    if !view.is_changed() {
        return;
    }
    for (control, mut background, mut border) in &mut controls {
        if view.focused_control_id.as_deref() == Some(control.stable_id.as_str()) {
            background.0 = WOOD;
            border.set_all(MOSS);
        } else {
            background.0 = Color::NONE;
            border.set_all(STONE);
        }
    }
}

fn sync_lai50_responsive_layout(
    windows: Query<'_, '_, &Window, With<PrimaryWindow>>,
    mut root: Query<'_, '_, &mut Node, With<Lai50ItemDetailRoot>>,
) {
    let (Ok(window), Ok(mut node)) = (windows.single(), root.single_mut()) else {
        return;
    };
    if window.resolution.width() < 720.0 {
        node.width = Val::Percent(100.0);
        node.max_width = Val::Percent(100.0);
        node.height = Val::Percent(100.0);
        node.max_height = Val::Percent(100.0);
        node.left = Val::Px(0.0);
        node.right = Val::Px(0.0);
        node.top = Val::Px(0.0);
        node.bottom = Val::Px(0.0);
        node.padding = UiRect::all(Val::Px(12.0));
    } else {
        node.width = Val::Px(520.0);
        node.max_width = Val::Percent(96.0);
        node.height = Val::Percent(78.0);
        node.max_height = Val::Px(760.0);
        node.left = Val::Auto;
        node.right = Val::Px(24.0);
        node.top = Val::Px(82.0);
        node.bottom = Val::Auto;
        node.padding = UiRect::all(Val::Px(14.0));
    }
}

fn handle_lai50_scroll(
    mut wheel: MessageReader<'_, '_, MouseWheel>,
    mut panes: Query<'_, '_, (&Interaction, &mut ScrollPosition), With<Lai50Scrollable>>,
) {
    let delta = wheel
        .read()
        .map(|event| match event.unit {
            MouseScrollUnit::Line => event.y * 28.0,
            MouseScrollUnit::Pixel => event.y,
        })
        .sum::<f32>();
    if delta == 0.0 {
        return;
    }
    for (interaction, mut scroll) in &mut panes {
        if *interaction == Interaction::Hovered {
            scroll.0.y = (scroll.0.y - delta).max(0.0);
        }
    }
}

fn surface_state_copy(state: &Lai50SurfaceState) -> String {
    match state {
        Lai50SurfaceState::Loading => "Loading canonical item report…".to_owned(),
        Lai50SurfaceState::Ready => "Current canonical report".to_owned(),
        Lai50SurfaceState::Stale { stale_since_ms } => {
            format!("Showing the last report; stale since {stale_since_ms} ms")
        }
        Lai50SurfaceState::Unavailable { reason } => format!("Unavailable: {reason}"),
        Lai50SurfaceState::UpdateRequired => {
            "Update required: this client accepts canonical protocol v3/schema v2 only.".to_owned()
        }
        Lai50SurfaceState::Error { message } => format!("Report error: {message}"),
    }
}

fn definition_name(
    definition: &Lai50Availability<ReportedDefinition>,
    fallback_id: &str,
) -> String {
    match definition {
        Lai50Availability::Reported(definition) => definition.display_name.clone(),
        Lai50Availability::Unavailable { .. } => fallback_id.to_owned(),
    }
}

fn availability_definition(definition: &Lai50Availability<ReportedDefinition>) -> String {
    match definition {
        Lai50Availability::Reported(definition) => {
            format!("{} ({})", definition.display_name, definition.definition_id)
        }
        Lai50Availability::Unavailable { reason } => format!("Unavailable — {reason}"),
    }
}

fn availability_material_icon(icon: &Lai50Availability<ExactMaterialIcon>) -> String {
    match icon {
        Lai50Availability::Reported(icon) => {
            format!("{} — {}", icon.display_name, icon.art_key)
        }
        Lai50Availability::Unavailable { reason } => format!("Unavailable — {reason}"),
    }
}

fn availability_text(value: &Lai50Availability<String>) -> String {
    match value {
        Lai50Availability::Reported(value) => value.clone(),
        Lai50Availability::Unavailable { reason } => format!("Unavailable — {reason}"),
    }
}

const fn quality_label(quality: QualityBandSnapshot) -> &'static str {
    match quality {
        QualityBandSnapshot::Crude => "Crude",
        QualityBandSnapshot::Common => "Common",
        QualityBandSnapshot::Fine => "Fine",
        QualityBandSnapshot::Superior => "Superior",
        QualityBandSnapshot::Masterwork => "Masterwork",
    }
}

fn format_basis_points(value: u16) -> String {
    format!("{}.{:02}%", value / 100, value % 100)
}

fn format_age(age_ms: u64) -> String {
    let seconds = age_ms / 1_000;
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;
    if days > 0 {
        format!("{days}d {}h", hours % 24)
    } else if hours > 0 {
        format!("{hours}h {}m", minutes % 60)
    } else if minutes > 0 {
        format!("{minutes}m {}s", seconds % 60)
    } else {
        format!("{seconds}s")
    }
}

fn reservation_label(reservation: &ReportedReservation) -> String {
    match reservation {
        ReportedReservation::Unreserved => "Unreserved".to_owned(),
        ReportedReservation::Reserved { reservation_id } => {
            format!("Reserved — {reservation_id}")
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use cat_protocol::lai64::{ReportText, StableId};

    fn id(value: &str) -> StableId {
        StableId::new(value).expect("test stable id")
    }

    fn report(value: &str) -> ReportText {
        ReportText::new(value).expect("test report text")
    }

    fn entry(id_value: &str, name: &str, art_key: &str) -> ContentManifestEntrySnapshot {
        ContentManifestEntrySnapshot {
            content_id: id(id_value),
            content_kind_id: id("material"),
            display_name: report(name),
            art_key: id(art_key),
            accessibility_label: report(name),
            capability_ids: Vec::new(),
        }
    }

    #[test]
    fn exact_material_art_accepts_only_delivered_material_category() {
        let material = entry("material_warg_fang", "Warg Fang", "art_material_warg_fang");
        let definition = definition_from_entry(&material);
        let asset = resolve_lai68_art_key(&definition.art_key).expect("delivered test icon");
        assert_eq!(asset.category, Lai68ArtCategory::MaterialIcon);
        assert_eq!(
            asset.path,
            "assets/planned/materials/art_material_warg_fang.png"
        );
        assert!(resolve_lai68_art_key("art_material_unknown_drop").is_none());
    }

    #[test]
    fn basis_points_and_age_are_exact_text_not_progress_simulation() {
        assert_eq!(format_basis_points(6_100), "61.00%");
        assert_eq!(format_age(90_061_000), "1d 1h");
        assert_eq!(quality_label(QualityBandSnapshot::Fine), "Fine");
    }

    #[test]
    fn reservation_and_visual_contract_do_not_invent_state() {
        assert_eq!(reported_reservation(None), ReportedReservation::Unreserved);
        assert_eq!(
            reported_reservation(Some("reservation:rod")),
            ReportedReservation::Reserved {
                reservation_id: "reservation:rod".to_owned()
            }
        );
        assert!(!LAI50_VISUAL_DIRECTION.uses_quality_frames);
        assert!(!LAI50_VISUAL_DIRECTION.uses_augmentation_overlays);
    }
}
