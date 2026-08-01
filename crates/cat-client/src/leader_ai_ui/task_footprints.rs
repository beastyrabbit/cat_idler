//! Snapshot-only render-model projection for LAI.29 visible task markers.
//!
//! This module consumes `cat-protocol` LAI.24 DTOs directly and deliberately
//! refuses to synthesize marker coordinates from cat movement, task names, or
//! local fallback geometry.

use std::collections::BTreeSet;

use bevy::prelude::*;
use cat_protocol::{
    NonEmptyStableId, ReportSafeString, SiteLifecycleStageSnapshot, SiteRefSnapshot, SiteSnapshot,
    SiteVisibilitySnapshot, SnapshotTilePoint, VisibleTaskSnapshot,
};

use super::{
    AccessibleLabel, RoleColor, StableUiId, TaskMarkerRole, TestIdBuilder,
    validate_product_normal_tokens,
};

pub const TASK_MARKER_OBJECTIVE_TEST_ID: &str = "task-marker:{task_id}:objective:{site_id}";
pub const TASK_MARKER_WORK_SLOT_TEST_ID: &str = "task-marker:{task_id}:work:{slot_id}";
pub const TASK_MARKER_ENDPOINT_TEST_ID: &str = "task-marker:{task_id}:endpoint:{site_id}";
pub const TASK_MARKER_CELL_TEST_ID: &str = "task-marker:{task_id}:cell:{index}:{site_id}";
pub const ACCESSIBLE_TASK_OBJECTIVE_LABEL: &str = "Task objective marker";
pub const ACCESSIBLE_TASK_WORK_SLOT_LABEL: &str = "Task work slot marker";
pub const ACCESSIBLE_TASK_ENDPOINT_LABEL: &str = "Task delivery endpoint marker";
pub const PLAYWRIGHT_TASK_MARKER_LOCATOR_MANIFEST: &str = "lai29-task-marker-locator-manifest";
pub const VISIBLE_BROWSER_CHECKPOINT_LAI29_WORKSHOP_FOOTPRINT: &str = "lai29-workshop-footprint";
pub const VISIBLE_BROWSER_CHECKPOINT_LAI29_HUNT_WATER: &str = "lai29-hunt-water";
pub const VISIBLE_BROWSER_CHECKPOINT_LAI29_DESPAWN_DEDUPE: &str = "lai29-despawn-dedupe";
pub const VISIBLE_BROWSER_CHECKPOINT_LAI29_REDACTION: &str = "lai29-redaction";

#[derive(Default)]
pub struct VisibleTaskMarkerPlugin;

impl Plugin for VisibleTaskMarkerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VisibleTaskMarkerInput>()
            .init_resource::<VisibleTaskMarkerWorld>()
            .add_systems(Update, update_visible_task_marker_world);
    }
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct VisibleTaskMarkerInput {
    pub selected_colony_id: Option<String>,
    pub colony_id: Option<String>,
    pub tasks: Vec<VisibleTaskSnapshot>,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct VisibleTaskMarkerWorld {
    pub projections: Vec<TaskFootprintProjection>,
    pub markers: Vec<TaskMarkerEntity>,
    pub retained_keys: BTreeSet<TaskSnapshotIdMarkerKey>,
    pub removed_keys: Vec<TaskSnapshotIdMarkerKey>,
    pub last_error: Option<TaskFootprintProjectionError>,
}

pub fn update_visible_task_marker_world(
    input: Res<'_, VisibleTaskMarkerInput>,
    mut world: ResMut<'_, VisibleTaskMarkerWorld>,
) {
    let previous_keys = world.retained_keys.clone();
    let selected_matches = input
        .selected_colony_id
        .as_deref()
        .zip(input.colony_id.as_deref())
        .is_some_and(|(selected, colony)| selected == colony);
    if !selected_matches {
        world.projections.clear();
        world.markers.clear();
        world.retained_keys.clear();
        world.removed_keys = previous_keys.into_iter().collect();
        world.last_error = None;
        return;
    }

    match project_visible_task_footprints(VisibleTaskSnapshotMarkerSource {
        tasks: &input.tasks,
    }) {
        Ok(projections) => {
            let markers = projections
                .iter()
                .flat_map(|projection| projection.markers.iter().cloned())
                .collect::<Vec<_>>();
            let retained_keys = markers
                .iter()
                .map(|marker| marker.key.clone())
                .collect::<BTreeSet<_>>();
            world.removed_keys = previous_keys
                .difference(&retained_keys)
                .cloned()
                .collect::<Vec<_>>();
            world.projections = projections;
            world.markers = markers;
            world.retained_keys = retained_keys;
            world.last_error = None;
        }
        Err(error) => {
            world.projections.clear();
            world.markers.clear();
            world.retained_keys.clear();
            world.removed_keys = previous_keys.into_iter().collect();
            world.last_error = Some(error);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoCatDestinationAuthorityForTaskMarkers;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DedupeVisibleTaskMarkerBySnapshotId;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpdateVisibleTaskMarkerFromSnapshotVersion;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DespawnRemovedVisibleTaskMarkers;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoStaleTaskMarkerReuse;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoDuplicateCoincidentTaskMarker;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticSiteStageDedupeKey;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisibleTaskRemovalEvent;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RedactedVisibleTaskNoMarker;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectiveLessBlockedTaskNoMapEntity;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissingSiteRefNoMarker;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockedSiteRefNoMarker;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForeignColonyVisibleTaskNoMarker;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectedColonyTaskMarkerFilter;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MultiColonyTaskMarkerIsolation;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReportSafeTaskMarkerVisibility;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteContactMarkerIsNotDeliveryEndpoint;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskMarkerSupportedZoomRange;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskMarkerViewportCullingKeepsAuthoritativeIds;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskMarkerScreenBoundsGuard;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskMarkerReportSafeTooltip;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskMarkerTooltipRedactionGuard;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoHiddenStockTooltipField;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoExactRegenerationBelowLevelFourTooltip;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoPrivateBeliefOrPlanTooltip;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoRadialTaskMarkerFallback;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoGenericTaskDestinationFallback;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoClientSideSiteGuessing;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HuntObjectiveCaveOrSourceMarker;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FetchWaterSourceMarker;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FetchWaterDryBankWorkMarker;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FetchWaterPinnedDeliveryEndpointMarker;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaterSourceIsNotWalkableWorkPosition;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockedOrUnreachableSiteSuppressesWorldMarker;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkshopObjectiveNineRowMajorCells;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkshopDistinctWorkSlotMarker;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkshopDistinctDeliveryEndpointMarker;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeObjectiveSixCanonicalCells;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoDuplicatedWorkshopSizeConstant;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CanonicalFootprintCellIndex(u8);

impl CanonicalFootprintCellIndex {
    pub fn new(value: usize) -> Result<Self, TaskFootprintProjectionError> {
        let value = u8::try_from(value)
            .map_err(|_| TaskFootprintProjectionError::InvalidCanonicalFootprint)?;
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskMarkerKind {
    Objective,
    WorkSlot,
    Endpoint,
    FootprintCell(CanonicalFootprintCellIndex),
}

impl TaskMarkerKind {
    const fn role(self) -> TaskMarkerRole {
        match self {
            Self::Objective => TaskMarkerRole::Objective,
            Self::WorkSlot => TaskMarkerRole::WorkSlot,
            Self::Endpoint => TaskMarkerRole::Endpoint,
            Self::FootprintCell(index) => TaskMarkerRole::Cell {
                row_major_index: index.get(),
            },
        }
    }

    const fn color(self, specialization: TaskMarkerSpecialization) -> RoleColor {
        match (self, specialization) {
            (Self::Objective, TaskMarkerSpecialization::FetchWaterSource) => RoleColor::Water,
            (Self::Objective, _) => RoleColor::Rust,
            (Self::WorkSlot, _) => RoleColor::Olive,
            (Self::Endpoint, _) => RoleColor::Wood,
            (Self::FootprintCell(_), _) => RoleColor::Stone,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskMarkerSpecialization {
    HuntObjectiveCaveOrSource,
    FetchWaterSource,
    FetchWaterDryBankWork,
    FetchWaterPinnedDeliveryEndpoint,
    WorkshopObjectiveCell,
    WorkshopDistinctWorkSlot,
    WorkshopDistinctDeliveryEndpoint,
    TreeObjectiveSixCanonicalCells,
    OrderedRoadRouteCell,
    OrderedRoadWorkSlot,
    OrderedRoadEndpoint,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskSnapshotIdMarkerKey {
    pub task_id: String,
    pub site_id: String,
    pub stage: String,
    pub role: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskMarkerEntity {
    pub key: TaskSnapshotIdMarkerKey,
    pub task_id: String,
    pub tile: SnapshotTilePoint,
    pub kind: TaskMarkerKind,
    pub specialization: TaskMarkerSpecialization,
    pub test_id: StableUiId,
    pub label: AccessibleLabel,
    pub role_color: RoleColor,
    pub assigned_cat_ids: Vec<String>,
    pub stage: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskFootprintProjection {
    pub task_id: String,
    pub intent_id: String,
    pub category: String,
    pub stage: String,
    pub assigned_cat_ids: Vec<String>,
    pub markers: Vec<TaskMarkerEntity>,
}

#[derive(Clone, Copy, Debug)]
pub struct VisibleTaskSnapshotMarkerSource<'a> {
    pub tasks: &'a [VisibleTaskSnapshot],
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct StrictSiteRefMarkerResolver;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskFootprintProjectionError {
    InvalidTheme,
    UnsupportedTaskCategory(String),
    UnsupportedSiteRef {
        category: String,
        expected: &'static str,
    },
    MissingEndpoint {
        task_id: String,
    },
    MissingDryBankWorkSlot {
        task_id: String,
    },
    InvalidWaterGeometry {
        task_id: String,
    },
    InvalidCanonicalFootprint,
    InvalidWorkshopFootprint,
    InvalidTreeFootprint,
    InvalidOrderedRoute,
    DuplicateMarkerKey(TaskSnapshotIdMarkerKey),
}

pub fn project_visible_task_footprints(
    source: VisibleTaskSnapshotMarkerSource<'_>,
) -> Result<Vec<TaskFootprintProjection>, TaskFootprintProjectionError> {
    let mut projections = Vec::new();
    for task in source.tasks {
        if let Some(projection) = project_visible_task_footprint(task)? {
            projections.push(projection);
        }
    }
    Ok(projections)
}

pub fn project_visible_task_footprint(
    task: &VisibleTaskSnapshot,
) -> Result<Option<TaskFootprintProjection>, TaskFootprintProjectionError> {
    validate_product_normal_tokens(&super::LeaderAiUiTheme::default())
        .map_err(|_| TaskFootprintProjectionError::InvalidTheme)?;

    if !StrictSiteRefMarkerResolver::is_renderable(task.objective.site()) {
        return Ok(None);
    }

    let mut projection = TaskFootprintProjection {
        task_id: task.task_id.as_str().to_string(),
        intent_id: task.intent_id.as_str().to_string(),
        category: task.category.as_str().to_string(),
        stage: task.stage.as_str().to_string(),
        assigned_cat_ids: task
            .assigned_cat_ids
            .iter()
            .map(|cat_id| cat_id.as_str().to_string())
            .collect(),
        markers: Vec::new(),
    };

    let category = normalized_category(&task.category);
    if category == TaskCategory::Unsupported && task.blocked_reason.is_some() {
        return Ok(None);
    }

    match category {
        TaskCategory::Hunt => {
            render_hunt_objective_from_revealed_hunting_source(task, &mut projection)?
        }
        TaskCategory::FetchWater => render_fetch_water_source_bank_endpoint(task, &mut projection)?,
        TaskCategory::Workshop => {
            render_workshop_three_by_three_objective_cells(task, &mut projection)?
        }
        TaskCategory::Tree => render_tree_six_canonical_footprint_cells(task, &mut projection)?,
        TaskCategory::Road => render_ordered_road_route_cells(task, &mut projection)?,
        TaskCategory::Unsupported => {
            return Err(TaskFootprintProjectionError::UnsupportedTaskCategory(
                task.category.as_str().to_string(),
            ));
        }
    }

    dedupe_by_snapshot_id(&projection.markers)?;
    Ok(Some(projection))
}

impl StrictSiteRefMarkerResolver {
    #[must_use]
    pub fn is_renderable(site: &SiteSnapshot) -> bool {
        site.visibility == SiteVisibilitySnapshot::Visible
            && site.lifecycle_stage != SiteLifecycleStageSnapshot::Blocked
            && site.blocked_reason.is_none()
    }

    fn delivery_endpoint_tile(
        site_ref: &SiteRefSnapshot,
    ) -> Result<Option<SnapshotTilePoint>, TaskFootprintProjectionError> {
        if !Self::is_renderable(site_ref.site()) {
            return Ok(None);
        }
        let tile = match site_ref {
            SiteRefSnapshot::Tile { tile, .. } => *tile,
            SiteRefSnapshot::Shrine { endpoint, .. }
            | SiteRefSnapshot::VillageEndpoint { endpoint, .. }
            | SiteRefSnapshot::TradeEndpoint { endpoint, .. } => *endpoint,
            _ => {
                return Err(TaskFootprintProjectionError::UnsupportedSiteRef {
                    category: "delivery_endpoint".to_string(),
                    expected: "Tile, Shrine, VillageEndpoint, or TradeEndpoint",
                });
            }
        };
        Ok(Some(tile))
    }
}

fn render_hunt_objective_from_revealed_hunting_source(
    task: &VisibleTaskSnapshot,
    projection: &mut TaskFootprintProjection,
) -> Result<(), TaskFootprintProjectionError> {
    let (site, cave_id, source_tile) = match &task.objective {
        SiteRefSnapshot::HuntSource {
            site,
            cave_id,
            source_tile,
        } => (site, cave_id, *source_tile),
        _ => {
            return Err(TaskFootprintProjectionError::UnsupportedSiteRef {
                category: task.category.as_str().to_string(),
                expected: "HuntSource",
            });
        }
    };
    push_marker(
        projection,
        site,
        Some(cave_id),
        source_tile,
        TaskMarkerKind::Objective,
        TaskMarkerSpecialization::HuntObjectiveCaveOrSource,
        "hunt source",
    )
}

fn render_fetch_water_source_bank_endpoint(
    task: &VisibleTaskSnapshot,
    projection: &mut TaskFootprintProjection,
) -> Result<(), TaskFootprintProjectionError> {
    let (site, source_tile, bank_tile) = match &task.objective {
        SiteRefSnapshot::WaterSourceAndBank {
            site,
            source_tile,
            bank_tile,
        } => (site, *source_tile, *bank_tile),
        _ => {
            return Err(TaskFootprintProjectionError::UnsupportedSiteRef {
                category: task.category.as_str().to_string(),
                expected: "WaterSourceAndBank",
            });
        }
    };
    if source_tile == bank_tile {
        return Err(TaskFootprintProjectionError::InvalidWaterGeometry {
            task_id: task.task_id.as_str().to_string(),
        });
    }
    if !task.work_slots.iter().any(|slot| slot.tile == bank_tile) {
        return Err(TaskFootprintProjectionError::MissingDryBankWorkSlot {
            task_id: task.task_id.as_str().to_string(),
        });
    }
    let endpoint =
        task.endpoint
            .as_ref()
            .ok_or_else(|| TaskFootprintProjectionError::MissingEndpoint {
                task_id: task.task_id.as_str().to_string(),
            })?;
    let Some(endpoint_tile) = StrictSiteRefMarkerResolver::delivery_endpoint_tile(endpoint)? else {
        return Ok(());
    };

    push_marker(
        projection,
        site,
        None,
        source_tile,
        TaskMarkerKind::Objective,
        TaskMarkerSpecialization::FetchWaterSource,
        "water source",
    )?;
    push_marker(
        projection,
        site,
        matching_slot_id(task, bank_tile),
        bank_tile,
        TaskMarkerKind::WorkSlot,
        TaskMarkerSpecialization::FetchWaterDryBankWork,
        "dry bank",
    )?;
    push_marker(
        projection,
        endpoint.site(),
        None,
        endpoint_tile,
        TaskMarkerKind::Endpoint,
        TaskMarkerSpecialization::FetchWaterPinnedDeliveryEndpoint,
        "delivery endpoint",
    )
}

fn render_workshop_three_by_three_objective_cells(
    task: &VisibleTaskSnapshot,
    projection: &mut TaskFootprintProjection,
) -> Result<(), TaskFootprintProjectionError> {
    let (site, ordered_tiles) = match &task.objective {
        SiteRefSnapshot::BuildingFootprint {
            site,
            building_kind,
            width,
            height,
            ordered_tiles,
            ..
        } if building_kind.as_str() == "workshop" && *width == 3 && *height == 3 => {
            (site, ordered_tiles)
        }
        _ => return Err(TaskFootprintProjectionError::InvalidWorkshopFootprint),
    };
    validate_nine_row_major_tiles(ordered_tiles)?;
    for (index, tile) in ordered_tiles.iter().copied().enumerate() {
        push_marker(
            projection,
            site,
            None,
            tile,
            TaskMarkerKind::FootprintCell(CanonicalFootprintCellIndex::new(index)?),
            TaskMarkerSpecialization::WorkshopObjectiveCell,
            "workshop footprint",
        )?;
    }
    for slot in &task.work_slots {
        push_marker(
            projection,
            site,
            Some(&slot.slot_id),
            slot.tile,
            TaskMarkerKind::WorkSlot,
            TaskMarkerSpecialization::WorkshopDistinctWorkSlot,
            "workshop work slot",
        )?;
    }
    if let Some(endpoint) = &task.endpoint
        && let Some(endpoint_tile) = StrictSiteRefMarkerResolver::delivery_endpoint_tile(endpoint)?
    {
        push_marker(
            projection,
            endpoint.site(),
            None,
            endpoint_tile,
            TaskMarkerKind::Endpoint,
            TaskMarkerSpecialization::WorkshopDistinctDeliveryEndpoint,
            "delivery endpoint",
        )?;
    }
    Ok(())
}

fn render_tree_six_canonical_footprint_cells(
    task: &VisibleTaskSnapshot,
    projection: &mut TaskFootprintProjection,
) -> Result<(), TaskFootprintProjectionError> {
    let (site, source_id, resource_kind, ordered_tiles) = match &task.objective {
        SiteRefSnapshot::ResourceSource {
            site,
            source_id,
            resource_kind,
            ordered_tiles,
        } if normalized_text(resource_kind.as_str()) == "tree" => {
            (site, source_id, resource_kind, ordered_tiles)
        }
        _ => return Err(TaskFootprintProjectionError::InvalidTreeFootprint),
    };
    if ordered_tiles.len() != 6 {
        return Err(TaskFootprintProjectionError::InvalidTreeFootprint);
    }
    for (index, tile) in ordered_tiles.iter().copied().enumerate() {
        push_marker(
            projection,
            site,
            Some(source_id),
            tile,
            TaskMarkerKind::FootprintCell(CanonicalFootprintCellIndex::new(index)?),
            TaskMarkerSpecialization::TreeObjectiveSixCanonicalCells,
            resource_kind.as_str(),
        )?;
    }
    for slot in &task.work_slots {
        push_marker(
            projection,
            site,
            Some(&slot.slot_id),
            slot.tile,
            TaskMarkerKind::WorkSlot,
            TaskMarkerSpecialization::TreeObjectiveSixCanonicalCells,
            "tree work slot",
        )?;
    }
    if let Some(endpoint) = &task.endpoint
        && let Some(endpoint_tile) = StrictSiteRefMarkerResolver::delivery_endpoint_tile(endpoint)?
    {
        push_marker(
            projection,
            endpoint.site(),
            None,
            endpoint_tile,
            TaskMarkerKind::Endpoint,
            TaskMarkerSpecialization::TreeObjectiveSixCanonicalCells,
            "delivery endpoint",
        )?;
    }
    Ok(())
}

fn render_ordered_road_route_cells(
    task: &VisibleTaskSnapshot,
    projection: &mut TaskFootprintProjection,
) -> Result<(), TaskFootprintProjectionError> {
    let (site, route_id, ordered_tiles) = match &task.objective {
        SiteRefSnapshot::OrderedRoute {
            site,
            route_id,
            ordered_tiles,
        } => (site, route_id, ordered_tiles),
        _ => return Err(TaskFootprintProjectionError::InvalidOrderedRoute),
    };
    if ordered_tiles.is_empty() {
        return Err(TaskFootprintProjectionError::InvalidOrderedRoute);
    }
    for (index, tile) in ordered_tiles.iter().copied().enumerate() {
        push_marker(
            projection,
            site,
            Some(route_id),
            tile,
            TaskMarkerKind::FootprintCell(CanonicalFootprintCellIndex::new(index)?),
            TaskMarkerSpecialization::OrderedRoadRouteCell,
            "ordered road",
        )?;
    }
    for slot in &task.work_slots {
        push_marker(
            projection,
            site,
            Some(&slot.slot_id),
            slot.tile,
            TaskMarkerKind::WorkSlot,
            TaskMarkerSpecialization::OrderedRoadWorkSlot,
            "road work slot",
        )?;
    }
    if let Some(endpoint) = &task.endpoint
        && let Some(endpoint_tile) = StrictSiteRefMarkerResolver::delivery_endpoint_tile(endpoint)?
    {
        push_marker(
            projection,
            endpoint.site(),
            None,
            endpoint_tile,
            TaskMarkerKind::Endpoint,
            TaskMarkerSpecialization::OrderedRoadEndpoint,
            "delivery endpoint",
        )?;
    }
    Ok(())
}

fn push_marker(
    projection: &mut TaskFootprintProjection,
    site: &SiteSnapshot,
    stable_site_override: Option<&NonEmptyStableId>,
    tile: SnapshotTilePoint,
    kind: TaskMarkerKind,
    specialization: TaskMarkerSpecialization,
    site_kind: &str,
) -> Result<(), TaskFootprintProjectionError> {
    if !StrictSiteRefMarkerResolver::is_renderable(site) {
        return Ok(());
    }
    let stable_site_id = stable_site_override.unwrap_or(&site.site_id).as_str();
    let role = kind.role();
    let test_id = TestIdBuilder::task_marker(&projection.task_id, stable_site_id, role);
    let label = AccessibleLabel::task_marker(&projection.category, role, site_kind);
    projection.markers.push(TaskMarkerEntity {
        key: TaskSnapshotIdMarkerKey {
            task_id: projection.task_id.clone(),
            site_id: stable_site_id.to_string(),
            stage: projection.stage.clone(),
            role: role.slug(),
        },
        task_id: projection.task_id.clone(),
        tile,
        kind,
        specialization,
        test_id,
        label,
        role_color: kind.color(specialization),
        assigned_cat_ids: projection.assigned_cat_ids.clone(),
        stage: projection.stage.clone(),
    });
    Ok(())
}

fn matching_slot_id(
    task: &VisibleTaskSnapshot,
    tile: SnapshotTilePoint,
) -> Option<&NonEmptyStableId> {
    task.work_slots
        .iter()
        .find(|slot| slot.tile == tile)
        .map(|slot| &slot.slot_id)
}

fn validate_nine_row_major_tiles(
    ordered_tiles: &[SnapshotTilePoint],
) -> Result<(), TaskFootprintProjectionError> {
    if ordered_tiles.len() != 9 {
        return Err(TaskFootprintProjectionError::InvalidWorkshopFootprint);
    }
    let anchor = ordered_tiles[0];
    let expected = (0..3)
        .flat_map(|dy| {
            (0..3).map(move |dx| SnapshotTilePoint {
                x: anchor.x + dx,
                y: anchor.y + dy,
            })
        })
        .collect::<Vec<_>>();
    if ordered_tiles == expected {
        Ok(())
    } else {
        Err(TaskFootprintProjectionError::InvalidWorkshopFootprint)
    }
}

fn dedupe_by_snapshot_id(markers: &[TaskMarkerEntity]) -> Result<(), TaskFootprintProjectionError> {
    let mut seen = BTreeSet::new();
    for marker in markers {
        if !seen.insert(marker.key.clone()) {
            return Err(TaskFootprintProjectionError::DuplicateMarkerKey(
                marker.key.clone(),
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TaskCategory {
    Hunt,
    FetchWater,
    Workshop,
    Tree,
    Road,
    Unsupported,
}

fn normalized_category(category: &ReportSafeString) -> TaskCategory {
    let normalized = normalized_text(category.as_str());
    match normalized.as_str() {
        "hunt" => TaskCategory::Hunt,
        "fetchwater" => TaskCategory::FetchWater,
        "workshop" | "buildworkshop" | "constructworkshop" => TaskCategory::Workshop,
        "tree" | "logging" | "felltree" | "choptree" | "cuttree" => TaskCategory::Tree,
        "road" | "buildroad" | "constructroad" | "roadconstruction" => TaskCategory::Road,
        _ => TaskCategory::Unsupported,
    }
}

fn normalized_text(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_ascii_whitespace() && *ch != '-' && *ch != '_')
        .flat_map(char::to_lowercase)
        .collect::<String>()
}

trait SiteRefSnapshotExt {
    fn site(&self) -> &SiteSnapshot;
}

impl SiteRefSnapshotExt for SiteRefSnapshot {
    fn site(&self) -> &SiteSnapshot {
        match self {
            Self::Tile { site, .. }
            | Self::AnchoredRect { site, .. }
            | Self::OrderedTileSet { site, .. }
            | Self::BuildingFootprint { site, .. }
            | Self::StockpileFootprint { site, .. }
            | Self::ResourceSource { site, .. }
            | Self::HuntSource { site, .. }
            | Self::WaterSourceAndBank { site, .. }
            | Self::OrderedRoute { site, .. }
            | Self::Shrine { site, .. }
            | Self::VillageEndpoint { site, .. }
            | Self::TradeEndpoint { site, .. } => site,
        }
    }
}
