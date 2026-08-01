//! LAI.50 report-safe Food, Cookhouse, and Fishing Hut surface.
//!
//! This module consumes only the canonical protocol-v3/schema-v2 selected-colony
//! snapshot. It does not read simulation authority, estimate ecology, turn
//! concrete foods into a generic scalar, or expose recipe/worker micromanagement.
//! Missing report fields stay visibly unavailable.

use std::collections::{BTreeMap, BTreeSet};

use accesskit::{Action, Role};
use bevy::a11y::{AccessibilityNode, ActionRequest as AccessibilityActionRequest};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use cat_protocol::{
    PROTOCOL_VERSION,
    lai64::{
        CANONICAL_SNAPSHOT_SCHEMA_VERSION, CanonicalColonySnapshot, CanonicalGodAction,
        CanonicalSnapshotEnvelope, FoodPermission, NudgeDomain, ProductionStageSnapshot,
        QualityBandSnapshot, ReportConfidence, StableId, TaskState,
    },
};

use super::{lai54::bevy_shell::Lai54ShellRoot, semantic_node, semantic_status_node};

const INK: Color = Color::srgb(0.153, 0.106, 0.086);
const PARCHMENT: Color = Color::srgb(0.937, 0.886, 0.741);
const PAPER_SHADE: Color = Color::srgb(0.866, 0.792, 0.635);
const DARK_FOREST: Color = Color::srgb(0.090, 0.235, 0.180);
const WOOD: Color = Color::srgb(0.427, 0.282, 0.169);
const STONE: Color = Color::srgb(0.48, 0.46, 0.39);
const MOSS: Color = Color::srgb(0.310, 0.439, 0.251);

pub const MAX_LAI50_RENDERED_ROWS: usize = 200;
pub const FOOD_CONSERVATION_STEP_BASIS_POINTS: i16 = 1_000;
pub const FOOD_PRIORITY_STEP_BASIS_POINTS: i16 = 1_000;

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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Lai50SurfaceState {
    #[default]
    Loading,
    Ready,
    Empty,
    Stale {
        stale_since_ms: i64,
    },
    UpdateRequired,
    Error {
        message: String,
    },
}

impl Lai50SurfaceState {
    #[must_use]
    pub const fn permits_remote_intent(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lai50Availability<T> {
    Reported(T),
    Unavailable { reason: String },
}

impl<T> Default for Lai50Availability<T> {
    fn default() -> Self {
        Self::Unavailable {
            reason: "This field has not been reported.".to_owned(),
        }
    }
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai50ViewState {
    pub selected_batch_id: Option<String>,
    pub selected_hut_id: Option<String>,
    pub selected_task_id: Option<String>,
    pub focused_control_id: Option<String>,
    pub refresh_requests: u64,
    pub local_feedback: Option<String>,
}

/// Integration owns when this additive panel is visible. Keeping the default
/// hidden avoids competing with the existing Stores surface during cutover.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lai50PanelVisibility {
    pub visible: bool,
}

/// The UI emits only an approved broad God intent. Authenticated transport owns
/// action-envelope versions, idempotency, and the authoritative response.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai50ActionIntent {
    pub sequence: u64,
    pub pending: Option<CanonicalGodAction>,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai50ProjectionResource(pub Lai50FoodProjection);

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
struct Lai50RenderState {
    dirty: bool,
    visible: bool,
}

impl Default for Lai50RenderState {
    fn default() -> Self {
        Self {
            dirty: true,
            visible: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai50FoodProjection {
    pub selected_colony_id: Option<String>,
    pub now_ms: Option<i64>,
    pub state_version: Option<u64>,
    pub state: Lai50SurfaceState,
    pub food_days: Lai50Availability<FoodDaysReport>,
    pub stocks: Vec<FoodStockRow>,
    pub permissions: Vec<FoodPermissionRow>,
    pub source_reports: Vec<FoodSourceReportRow>,
    pub cookhouse: CookhouseProjection,
    pub fishing_huts: Vec<FishingHutRow>,
    pub current_physical_tasks: Vec<FoodPhysicalTaskRow>,
    pub reads_authoritative_world_truth: bool,
    pub derives_hidden_ecology_or_regeneration: bool,
    pub exposes_recipe_or_worker_authority: bool,
    pub exposes_generic_food_scalar: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoodDaysReport {
    pub lower_milli_days: u64,
    pub upper_milli_days: u64,
    pub observed_at_ms: i64,
    pub confidence: ReportConfidence,
    pub explanation: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoodStockRow {
    pub content_id: String,
    pub display_name: String,
    pub quality: QualityBandSnapshot,
    pub total_quantity: u64,
    pub lot_ids: Vec<String>,
    pub nutrition_basis_points: Vec<u16>,
    pub spoilage_basis_points: Vec<u16>,
    pub permissions: Vec<FoodPermission>,
    pub location_site_ids: Vec<String>,
    pub provenance: Lai50Availability<Vec<String>>,
    pub semantic_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoodPermissionRow {
    pub content_id: String,
    pub display_name: String,
    pub permission: FoodPermission,
    pub reason: String,
    pub confidence: ReportConfidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoodSourceReportRow {
    pub report_id: String,
    pub domain_id: String,
    pub event_kind_id: String,
    pub message: String,
    pub occurred_at_ms: i64,
    pub confidence: ReportConfidence,
    pub source_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CookhouseProjection {
    pub batches: Vec<CookhouseBatchRow>,
    pub selected_batch: Option<CookhouseBatchRow>,
    pub queue_order: Lai50Availability<Vec<String>>,
    pub reported_modifiers: Lai50Availability<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CookhouseBatchRow {
    pub batch_id: String,
    pub station_id: String,
    pub recipe_id: String,
    pub recipe_name: String,
    pub stage: ProductionStageSnapshot,
    pub progress_basis_points: u16,
    pub ingredient_lot_ids: Vec<String>,
    pub output_lot_ids: Vec<String>,
    pub worker_cat_id: Option<String>,
    pub blocker: Option<String>,
    pub semantic_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FishingHutRow {
    pub hut_id: String,
    pub ordered_land_footprint: Vec<(i32, i32)>,
    pub dock_land_tile: (i32, i32),
    pub reserved_water_tile: (i32, i32),
    pub orientation_id: String,
    pub mode_id: String,
    pub stage: ProductionStageSnapshot,
    pub progress_basis_points: u16,
    pub worker_cat_id: Option<String>,
    pub habitat_report: String,
    pub report_confidence: ReportConfidence,
    pub hut_bonus: Lai50Availability<String>,
    pub rod: Lai50Availability<FishingRodRow>,
    pub art_key: String,
    pub semantic_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FishingRodRow {
    pub item_id: String,
    pub definition_id: String,
    pub material_id: String,
    pub quality: QualityBandSnapshot,
    pub remaining_durability_basis_points: u16,
    pub wear_basis_points: u16,
    pub augmentation_ids: Vec<String>,
    pub provenance_id: String,
    pub location_site_id: String,
    pub reservation_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoodPhysicalTaskRow {
    pub task_id: String,
    pub task_kind_id: String,
    pub objective: String,
    pub state: TaskState,
    pub site_id: String,
    pub site_kind_id: String,
    pub ordered_objective_footprint: Vec<(i32, i32)>,
    pub work_sites: Vec<FoodTaskSiteRow>,
    pub delivery_site: Option<FoodTaskSiteRow>,
    pub ordered_route: Vec<(i32, i32)>,
    pub worker_cat_ids: Vec<String>,
    pub cargo: Vec<FoodTaskCargoRow>,
    pub blockers: Vec<String>,
    pub semantic_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoodTaskSiteRow {
    pub site_id: String,
    pub site_kind_id: String,
    pub slot_id: Option<String>,
    pub ordered_footprint: Vec<(i32, i32)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoodTaskCargoRow {
    pub cargo_id: String,
    pub content_id: String,
    pub quantity: u64,
    pub quality_band: u8,
    pub provenance_id: String,
    pub reservation_id: Option<String>,
    pub container_id: Option<String>,
    pub location_site_id: Option<String>,
    pub location_tile: Option<(i32, i32)>,
}

/// Pure projection from the canonical report. Food-days and station modifiers
/// deliberately remain unavailable because canonical schema v2 does not carry
/// those reports and the client must not recompute them.
#[must_use]
pub fn project_lai50_food(feed: &Lai50SnapshotFeed, view: &Lai50ViewState) -> Lai50FoodProjection {
    let state = surface_state(feed);
    let Some(envelope) = feed.envelope.as_ref() else {
        return Lai50FoodProjection { state, ..default() };
    };
    if envelope.protocol_version != PROTOCOL_VERSION
        || envelope.snapshot_schema_version != CANONICAL_SNAPSHOT_SCHEMA_VERSION
    {
        return Lai50FoodProjection {
            selected_colony_id: Some(envelope.selected_colony_id.as_str().to_owned()),
            now_ms: Some(envelope.now_ms),
            state: Lai50SurfaceState::UpdateRequired,
            ..default()
        };
    }
    let Some(colony) = envelope
        .colonies
        .iter()
        .find(|colony| colony.colony_id == envelope.selected_colony_id)
    else {
        return Lai50FoodProjection {
            selected_colony_id: Some(envelope.selected_colony_id.as_str().to_owned()),
            now_ms: Some(envelope.now_ms),
            state: Lai50SurfaceState::Error {
                message: "The selected colony is absent from this report.".to_owned(),
            },
            ..default()
        };
    };
    project_colony(colony, envelope.now_ms, state, view)
}

fn project_colony(
    colony: &CanonicalColonySnapshot,
    now_ms: i64,
    state: Lai50SurfaceState,
    view: &Lai50ViewState,
) -> Lai50FoodProjection {
    let names = manifest_names(colony);
    let food_content_ids = colony
        .food_stocks
        .iter()
        .map(|stock| stock.content_id.as_str().to_owned())
        .chain(
            colony
                .hole
                .food_permissions
                .iter()
                .map(|entry| entry.content_id.as_str().to_owned()),
        )
        .collect::<BTreeSet<_>>();
    let stocks = project_stocks(colony, &names);
    let permissions = project_permissions(colony, &names);
    let source_reports = project_source_reports(colony);
    let cookhouse = project_cookhouse(colony, &names, view);
    let fishing_huts = project_fishing_huts(colony);
    let current_physical_tasks = colony
        .tasks
        .iter()
        .filter(|task| {
            !matches!(task.state, TaskState::Complete | TaskState::Refused)
                && is_food_physical_task(
                    task.task_kind_id.as_str(),
                    task.site_kind_id.as_str(),
                    task.cargo.iter().map(|cargo| cargo.content_id.as_str()),
                    &food_content_ids,
                )
        })
        .take(MAX_LAI50_RENDERED_ROWS)
        .map(project_task)
        .collect();

    Lai50FoodProjection {
        selected_colony_id: Some(colony.colony_id.as_str().to_owned()),
        now_ms: Some(now_ms),
        state_version: Some(colony.state_version),
        state,
        food_days: Lai50Availability::Unavailable {
            reason: "Canonical schema v2 does not carry the Farmer's believed food-days range, observation time, confidence, or explanation. The client will not derive it from stock, population, or hidden drain.".to_owned(),
        },
        stocks,
        permissions,
        source_reports,
        cookhouse,
        fishing_huts,
        current_physical_tasks,
        reads_authoritative_world_truth: false,
        derives_hidden_ecology_or_regeneration: false,
        exposes_recipe_or_worker_authority: false,
        exposes_generic_food_scalar: false,
    }
}

fn manifest_names(colony: &CanonicalColonySnapshot) -> BTreeMap<String, String> {
    colony
        .content_manifest
        .as_ref()
        .map(|manifest| {
            manifest
                .entries
                .iter()
                .map(|entry| {
                    (
                        entry.content_id.as_str().to_owned(),
                        entry.display_name.as_str().to_owned(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn display_name(names: &BTreeMap<String, String>, id: &str) -> String {
    names.get(id).cloned().unwrap_or_else(|| id.to_owned())
}

#[derive(Default)]
struct FoodStockAccumulator {
    display_name: String,
    quantity: u64,
    lot_ids: BTreeSet<String>,
    nutrition: BTreeSet<u16>,
    spoilage: BTreeSet<u16>,
    permissions: BTreeSet<u8>,
    locations: BTreeSet<String>,
    provenance: BTreeSet<String>,
}

fn project_stocks(
    colony: &CanonicalColonySnapshot,
    names: &BTreeMap<String, String>,
) -> Vec<FoodStockRow> {
    let provenance_by_lot = colony
        .quality_lots
        .iter()
        .map(|lot| (lot.lot_id.as_str(), lot.provenance_id.as_str().to_owned()))
        .collect::<BTreeMap<_, _>>();
    let mut grouped = BTreeMap::<(String, u8), FoodStockAccumulator>::new();
    for stock in &colony.food_stocks {
        let key = (
            stock.content_id.as_str().to_owned(),
            quality_ordinal(stock.quality),
        );
        let entry = grouped.entry(key).or_default();
        entry.display_name = display_name(names, stock.content_id.as_str());
        entry.quantity = entry.quantity.saturating_add(stock.quantity);
        entry.lot_ids.insert(stock.lot_id.as_str().to_owned());
        entry.nutrition.insert(stock.nutrition_basis_points);
        entry.spoilage.insert(stock.spoilage_basis_points);
        entry
            .permissions
            .insert(permission_ordinal(stock.permission));
        entry
            .locations
            .insert(stock.location_site_id.as_str().to_owned());
        if let Some(provenance) = provenance_by_lot.get(stock.lot_id.as_str()) {
            entry.provenance.insert(provenance.clone());
        }
    }
    let mut rows = grouped
        .into_iter()
        .map(|((content_id, quality), entry)| FoodStockRow {
            semantic_id: stable_semantic_id("stock", &format!("{content_id}:q{quality}")),
            content_id,
            display_name: entry.display_name,
            quality: quality_from_ordinal(quality),
            total_quantity: entry.quantity,
            lot_ids: entry.lot_ids.into_iter().collect(),
            nutrition_basis_points: entry.nutrition.into_iter().collect(),
            spoilage_basis_points: entry.spoilage.into_iter().collect(),
            permissions: entry
                .permissions
                .into_iter()
                .map(permission_from_ordinal)
                .collect(),
            location_site_ids: entry.locations.into_iter().collect(),
            provenance: if entry.provenance.is_empty() {
                Lai50Availability::Unavailable {
                    reason: "The food stock report has no matching quality-lot provenance record."
                        .to_owned(),
                }
            } else {
                Lai50Availability::Reported(entry.provenance.into_iter().collect())
            },
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .spoilage_basis_points
            .iter()
            .max()
            .cmp(&left.spoilage_basis_points.iter().max())
            .then_with(|| quality_ordinal(left.quality).cmp(&quality_ordinal(right.quality)))
            .then_with(|| left.content_id.cmp(&right.content_id))
    });
    rows
}

fn project_permissions(
    colony: &CanonicalColonySnapshot,
    names: &BTreeMap<String, String>,
) -> Vec<FoodPermissionRow> {
    let mut rows = colony
        .hole
        .food_permissions
        .iter()
        .map(|entry| FoodPermissionRow {
            content_id: entry.content_id.as_str().to_owned(),
            display_name: display_name(names, entry.content_id.as_str()),
            permission: entry.permission,
            reason: entry.reason.as_str().to_owned(),
            confidence: entry.confidence,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.content_id.cmp(&right.content_id));
    rows
}

fn project_source_reports(colony: &CanonicalColonySnapshot) -> Vec<FoodSourceReportRow> {
    let mut rows = colony
        .event_log
        .iter()
        .filter(|event| {
            let domain = event.domain_id.as_str().to_ascii_lowercase();
            let kind = event.event_kind_id.as_str().to_ascii_lowercase();
            [
                "food", "apple", "fish", "cook", "farm", "hunt", "water", "ecolog",
            ]
            .iter()
            .any(|needle| domain.contains(needle) || kind.contains(needle))
        })
        .take(MAX_LAI50_RENDERED_ROWS)
        .map(|event| FoodSourceReportRow {
            report_id: event.event_id.as_str().to_owned(),
            domain_id: event.domain_id.as_str().to_owned(),
            event_kind_id: event.event_kind_id.as_str().to_owned(),
            message: event.message.as_str().to_owned(),
            occurred_at_ms: event.occurred_at_ms,
            confidence: event.confidence,
            source_ids: event
                .source_ids
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .occurred_at_ms
            .cmp(&left.occurred_at_ms)
            .then_with(|| left.report_id.cmp(&right.report_id))
    });
    rows
}

fn project_cookhouse(
    colony: &CanonicalColonySnapshot,
    names: &BTreeMap<String, String>,
    view: &Lai50ViewState,
) -> CookhouseProjection {
    let mut batches = colony
        .cookhouse_batches
        .iter()
        .map(|batch| CookhouseBatchRow {
            batch_id: batch.batch_id.as_str().to_owned(),
            station_id: batch.station_id.as_str().to_owned(),
            recipe_id: batch.recipe_id.as_str().to_owned(),
            recipe_name: display_name(names, batch.recipe_id.as_str()),
            stage: batch.stage,
            progress_basis_points: batch.progress_basis_points,
            ingredient_lot_ids: batch
                .ingredient_lot_ids
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
            output_lot_ids: batch
                .output_lot_ids
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
            worker_cat_id: batch
                .worker_cat_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            blocker: batch
                .blocker
                .as_ref()
                .map(|blocker| blocker.as_str().to_owned()),
            semantic_id: stable_semantic_id("batch", batch.batch_id.as_str()),
        })
        .collect::<Vec<_>>();
    batches.sort_by(|left, right| left.batch_id.cmp(&right.batch_id));
    let selected_batch = view
        .selected_batch_id
        .as_deref()
        .and_then(|id| batches.iter().find(|batch| batch.batch_id == id))
        .cloned()
        .or_else(|| batches.first().cloned());
    CookhouseProjection {
        batches,
        selected_batch,
        queue_order: Lai50Availability::Unavailable {
            reason: "Canonical schema v2 reports batches but no authoritative Cookhouse queue position. Stable-ID display order is not presented as production priority.".to_owned(),
        },
        reported_modifiers: Lai50Availability::Unavailable {
            reason: "Station tier, Cook skill, tool, fixture, and complexity modifiers are not carried by the canonical Cookhouse batch report.".to_owned(),
        },
    }
}

fn project_fishing_huts(colony: &CanonicalColonySnapshot) -> Vec<FishingHutRow> {
    let exact_items = colony
        .exact_items
        .iter()
        .map(|item| (item.item_id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let mut rows = colony
        .fishing_huts
        .iter()
        .map(|hut| {
            let rod = hut.rod_item_id.as_ref().map_or_else(
                || Lai50Availability::Unavailable {
                    reason: "No Fishing Rod is reported for this Hut.".to_owned(),
                },
                |rod_id| {
                    exact_items.get(rod_id.as_str()).map_or_else(
                        || Lai50Availability::Unavailable {
                            reason: "The Hut references a Rod that is absent from the exact-item report."
                                .to_owned(),
                        },
                        |item| {
                            Lai50Availability::Reported(FishingRodRow {
                                item_id: item.item_id.as_str().to_owned(),
                                definition_id: item.definition_id.as_str().to_owned(),
                                material_id: item.material_id.as_str().to_owned(),
                                quality: item.quality,
                                remaining_durability_basis_points: item.durability_basis_points,
                                wear_basis_points: 10_000_u16
                                    .saturating_sub(item.durability_basis_points),
                                augmentation_ids: item
                                    .augmentation_ids
                                    .iter()
                                    .map(|id| id.as_str().to_owned())
                                    .collect(),
                                provenance_id: item.provenance_id.as_str().to_owned(),
                                location_site_id: item.location_site_id.as_str().to_owned(),
                                reservation_id: item
                                    .reservation_id
                                    .as_ref()
                                    .map(|id| id.as_str().to_owned()),
                            })
                        },
                    )
                },
            );
            FishingHutRow {
                hut_id: hut.hut_id.as_str().to_owned(),
                ordered_land_footprint: tile_pairs(&hut.footprint.ordered_tiles),
                dock_land_tile: (hut.dock_land_tile.x, hut.dock_land_tile.y),
                reserved_water_tile: (hut.reserved_water_tile.x, hut.reserved_water_tile.y),
                orientation_id: hut.orientation_id.as_str().to_owned(),
                mode_id: hut.mode_id.as_str().to_owned(),
                stage: hut.stage,
                progress_basis_points: hut.progress_basis_points,
                worker_cat_id: hut
                    .worker_cat_id
                    .as_ref()
                    .map(|id| id.as_str().to_owned()),
                habitat_report: hut.habitat_report.as_str().to_owned(),
                report_confidence: hut.report_confidence,
                hut_bonus: Lai50Availability::Unavailable {
                    reason: "The report carries Hut mode, stage, and staffing, but not an explicit active coordination/storage bonus. The client will not infer one.".to_owned(),
                },
                rod,
                art_key: hut.art_key.as_str().to_owned(),
                semantic_id: stable_semantic_id("fishing-hut", hut.hut_id.as_str()),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.hut_id.cmp(&right.hut_id));
    rows
}

fn project_task(task: &cat_protocol::lai64::PhysicalTaskSnapshot) -> FoodPhysicalTaskRow {
    FoodPhysicalTaskRow {
        task_id: task.task_id.as_str().to_owned(),
        task_kind_id: task.task_kind_id.as_str().to_owned(),
        objective: task.objective.as_str().to_owned(),
        state: task.state,
        site_id: task.site_id.as_str().to_owned(),
        site_kind_id: task.site_kind_id.as_str().to_owned(),
        ordered_objective_footprint: tile_pairs(&task.footprint.ordered_tiles),
        work_sites: task
            .work_sites
            .iter()
            .map(|site| FoodTaskSiteRow {
                site_id: site.site_id.as_str().to_owned(),
                site_kind_id: site.site_kind_id.as_str().to_owned(),
                slot_id: site.slot_id.as_ref().map(|id| id.as_str().to_owned()),
                ordered_footprint: tile_pairs(&site.footprint.ordered_tiles),
            })
            .collect(),
        delivery_site: task.delivery_site.as_ref().map(|site| FoodTaskSiteRow {
            site_id: site.site_id.as_str().to_owned(),
            site_kind_id: site.site_kind_id.as_str().to_owned(),
            slot_id: site.slot_id.as_ref().map(|id| id.as_str().to_owned()),
            ordered_footprint: tile_pairs(&site.footprint.ordered_tiles),
        }),
        ordered_route: tile_pairs(&task.route.ordered_tiles),
        worker_cat_ids: task
            .worker_cat_ids
            .iter()
            .map(|id| id.as_str().to_owned())
            .collect(),
        cargo: task
            .cargo
            .iter()
            .map(|cargo| FoodTaskCargoRow {
                cargo_id: cargo.cargo_id.as_str().to_owned(),
                content_id: cargo.content_id.as_str().to_owned(),
                quantity: cargo.quantity,
                quality_band: cargo.quality_band,
                provenance_id: cargo.provenance_id.as_str().to_owned(),
                reservation_id: cargo
                    .reservation_id
                    .as_ref()
                    .map(|id| id.as_str().to_owned()),
                container_id: cargo.container_id.as_ref().map(|id| id.as_str().to_owned()),
                location_site_id: cargo
                    .location_site_id
                    .as_ref()
                    .map(|id| id.as_str().to_owned()),
                location_tile: cargo.location_tile.as_ref().map(|tile| (tile.x, tile.y)),
            })
            .collect(),
        blockers: task
            .blockers
            .iter()
            .map(|blocker| blocker.reason.as_str().to_owned())
            .collect(),
        semantic_id: stable_semantic_id("task", task.task_id.as_str()),
    }
}

fn is_food_physical_task<'a>(
    task_kind_id: &str,
    site_kind_id: &str,
    mut cargo_content_ids: impl Iterator<Item = &'a str>,
    food_content_ids: &BTreeSet<String>,
) -> bool {
    let task = task_kind_id.to_ascii_lowercase();
    let site = site_kind_id.to_ascii_lowercase();
    let semantic_match = [
        "food", "cook", "fish", "apple", "farm", "hunt", "water", "meal", "forage",
    ]
    .iter()
    .any(|needle| task.contains(needle) || site.contains(needle));
    semantic_match || cargo_content_ids.any(|content_id| food_content_ids.contains(content_id))
}

fn tile_pairs(tiles: &[cat_protocol::lai64::Tile]) -> Vec<(i32, i32)> {
    tiles.iter().map(|tile| (tile.x, tile.y)).collect()
}

fn surface_state(feed: &Lai50SnapshotFeed) -> Lai50SurfaceState {
    match &feed.refresh {
        Lai50RefreshState::Loading => Lai50SurfaceState::Loading,
        Lai50RefreshState::Ready if feed.envelope.is_none() => Lai50SurfaceState::Empty,
        Lai50RefreshState::Ready => Lai50SurfaceState::Ready,
        Lai50RefreshState::Stale { stale_since_ms } => Lai50SurfaceState::Stale {
            stale_since_ms: *stale_since_ms,
        },
        Lai50RefreshState::UpdateRequired => Lai50SurfaceState::UpdateRequired,
        Lai50RefreshState::Error { message } => Lai50SurfaceState::Error {
            message: message.clone(),
        },
    }
}

const fn quality_ordinal(quality: QualityBandSnapshot) -> u8 {
    match quality {
        QualityBandSnapshot::Crude => 0,
        QualityBandSnapshot::Common => 1,
        QualityBandSnapshot::Fine => 2,
        QualityBandSnapshot::Superior => 3,
        QualityBandSnapshot::Masterwork => 4,
    }
}

const fn quality_from_ordinal(quality: u8) -> QualityBandSnapshot {
    match quality {
        0 => QualityBandSnapshot::Crude,
        1 => QualityBandSnapshot::Common,
        2 => QualityBandSnapshot::Fine,
        3 => QualityBandSnapshot::Superior,
        _ => QualityBandSnapshot::Masterwork,
    }
}

const fn permission_ordinal(permission: FoodPermission) -> u8 {
    match permission {
        FoodPermission::Allowed => 0,
        FoodPermission::Reserve => 1,
        FoodPermission::Forbidden => 2,
    }
}

const fn permission_from_ordinal(permission: u8) -> FoodPermission {
    match permission {
        0 => FoodPermission::Allowed,
        1 => FoodPermission::Reserve,
        _ => FoodPermission::Forbidden,
    }
}

#[must_use]
pub fn stable_semantic_id(section: &str, authoritative_id: &str) -> String {
    let mut slug = String::new();
    for byte in authoritative_id.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'-' | b'|') {
            slug.push(byte.to_ascii_lowercase() as char);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.len() > 72 {
        let hash = authoritative_id
            .bytes()
            .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
                hash.wrapping_mul(0x0000_0100_0000_01b3) ^ u64::from(byte)
            });
        slug.truncate(55);
        slug.push_str(&format!("-{hash:016x}"));
    }
    format!("lai50:{section}:{slug}")
}

#[must_use]
pub const fn is_lai50_allowed_action(action: &CanonicalGodAction) -> bool {
    matches!(
        action,
        CanonicalGodAction::FoodConservation { .. }
            | CanonicalGodAction::BroadDomainNudge {
                domain: NudgeDomain::Food,
                ..
            }
    )
}

#[derive(Component)]
pub struct Lai50Root;
#[derive(Component)]
pub struct Lai50Workspace;
#[derive(Component)]
pub struct Lai50StatusLabel;
#[derive(Component)]
pub struct Lai50ScrollablePane;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lai50PaneKind {
    FoodLedger,
    Operations,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lai50Pane(pub Lai50PaneKind);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lai50ControlAction {
    Refresh,
    SelectBatch(String),
    SelectHut(String),
    SelectTask(String),
    Emit(CanonicalGodAction),
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct Lai50Control {
    pub stable_id: String,
    pub focus_order: u32,
    pub enabled: bool,
    pub action: Lai50ControlAction,
}

#[derive(Default)]
pub struct Lai50FoodCookhousePlugin;

impl Plugin for Lai50FoodCookhousePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Lai50SnapshotFeed>()
            .init_resource::<Lai50ViewState>()
            .init_resource::<Lai50PanelVisibility>()
            .init_resource::<Lai50ActionIntent>()
            .init_resource::<Lai50ProjectionResource>()
            .init_resource::<Lai50RenderState>()
            .add_message::<MouseWheel>()
            .add_message::<AccessibilityActionRequest>()
            .add_systems(
                Update,
                (
                    attach_lai50_surface,
                    sync_lai50_projection,
                    sync_lai50_visibility,
                    render_lai50,
                    handle_pointer_controls,
                    handle_keyboard_controls,
                    handle_accessibility_controls,
                    sync_focus_style,
                    sync_layout,
                    handle_scroll,
                )
                    .chain(),
            );
    }
}

fn attach_lai50_surface(
    mut commands: Commands<'_, '_>,
    shell: Query<'_, '_, Entity, With<Lai54ShellRoot>>,
    existing: Query<'_, '_, Entity, With<Lai50Root>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Ok(shell) = shell.single() else {
        return;
    };
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                right: Val::Px(24.0),
                top: Val::Px(82.0),
                bottom: Val::Px(24.0),
                display: Display::None,
                padding: UiRect::all(Val::Px(18.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            GlobalZIndex(1_305),
            BackgroundColor(DARK_FOREST),
            BorderColor::all(WOOD),
            Lai50Root,
            crate::WorldInputBlocker,
            semantic_node(
                Role::Pane,
                "lai50:food-cookhouse:panel",
                "Food and Cookhouse report",
                true,
            ),
            Name::new("LAI.50 report-safe Food and Cookhouse"),
        ))
        .id();
    commands.entity(shell).add_child(root);
    commands.entity(root).with_children(|panel| {
        panel.spawn(text_bundle("Food and Cookhouse", 24.0, PARCHMENT));
        panel.spawn((
            text_bundle("Loading report-safe food data", 13.0, PAPER_SHADE),
            Lai50StatusLabel,
            semantic_status_node(
                "lai50:food-cookhouse:status",
                "Food and Cookhouse is loading",
                false,
            ),
        ));
    });
    let workspace = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(240.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.0),
                row_gap: Val::Px(12.0),
                overflow: Overflow::clip(),
                ..default()
            },
            Lai50Workspace,
            Name::new("LAI.50 Food and Cookhouse workspace"),
        ))
        .id();
    commands.entity(root).add_child(workspace);
    spawn_pane(
        &mut commands,
        workspace,
        Lai50PaneKind::FoodLedger,
        42.0,
        PARCHMENT,
    );
    spawn_pane(
        &mut commands,
        workspace,
        Lai50PaneKind::Operations,
        58.0,
        PAPER_SHADE,
    );
}

fn spawn_pane(
    commands: &mut Commands<'_, '_>,
    parent: Entity,
    kind: Lai50PaneKind,
    width_percent: f32,
    background: Color,
) {
    let pane = commands
        .spawn((
            Node {
                width: Val::Percent(width_percent),
                height: Val::Percent(100.0),
                min_height: Val::Px(220.0),
                padding: UiRect::all(Val::Px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                border: UiRect::all(Val::Px(1.0)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            Interaction::default(),
            ScrollPosition::default(),
            BackgroundColor(background),
            BorderColor::all(WOOD),
            Lai50Pane(kind),
            Lai50ScrollablePane,
            Name::new(format!("LAI.50 {kind:?} pane")),
        ))
        .id();
    commands.entity(parent).add_child(pane);
}

fn sync_lai50_projection(
    feed: Res<'_, Lai50SnapshotFeed>,
    view: Res<'_, Lai50ViewState>,
    mut projection: ResMut<'_, Lai50ProjectionResource>,
    mut render: ResMut<'_, Lai50RenderState>,
) {
    if feed.is_changed() || view.is_changed() {
        projection.0 = project_lai50_food(&feed, &view);
        render.dirty = true;
    }
}

fn sync_lai50_visibility(
    visibility: Res<'_, Lai50PanelVisibility>,
    mut root: Query<'_, '_, &mut Node, With<Lai50Root>>,
    mut render: ResMut<'_, Lai50RenderState>,
) {
    if render.visible != visibility.visible {
        render.visible = visibility.visible;
        render.dirty = true;
    }
    if let Ok(mut node) = root.single_mut() {
        node.display = if visibility.visible {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn render_lai50(
    mut commands: Commands<'_, '_>,
    projection: Res<'_, Lai50ProjectionResource>,
    mut render: ResMut<'_, Lai50RenderState>,
    panes: Query<'_, '_, (Entity, &Lai50Pane)>,
    mut status: Query<'_, '_, (&mut Text, &mut AccessibilityNode), With<Lai50StatusLabel>>,
) {
    if !render.dirty || panes.is_empty() {
        return;
    }
    let status_copy = state_copy(&projection.0.state);
    if let Ok((mut text, mut accessibility)) = status.single_mut() {
        text.0.clone_from(&status_copy);
        *accessibility = semantic_status_node(
            "lai50:food-cookhouse:status",
            status_copy,
            matches!(
                projection.0.state,
                Lai50SurfaceState::Error { .. } | Lai50SurfaceState::UpdateRequired
            ),
        );
    }
    for (pane, marker) in &panes {
        commands.entity(pane).despawn_children();
        match marker.0 {
            Lai50PaneKind::FoodLedger => render_food_ledger(&mut commands, pane, &projection.0),
            Lai50PaneKind::Operations => render_operations(&mut commands, pane, &projection.0),
        }
    }
    render.dirty = false;
}

fn render_food_ledger(
    commands: &mut Commands<'_, '_>,
    pane: Entity,
    projection: &Lai50FoodProjection,
) {
    spawn_section(
        commands,
        pane,
        "Food-days estimate",
        &availability_text(&projection.food_days),
    );
    spawn_control(
        commands,
        pane,
        "refresh",
        1,
        "Refresh report",
        true,
        Lai50ControlAction::Refresh,
    );
    spawn_section(
        commands,
        pane,
        "Typed food by spoilage and quality",
        &stocks_text(&projection.stocks),
    );
    spawn_section(
        commands,
        pane,
        "Allowed, Reserve, and Forbidden",
        &permissions_text(&projection.permissions),
    );
    spawn_section(
        commands,
        pane,
        "Food-source reports",
        &source_reports_text(&projection.source_reports),
    );
    let enabled = projection.state.permits_remote_intent();
    spawn_control(
        commands,
        pane,
        "conservation-more",
        10,
        "Conserve food more",
        enabled,
        Lai50ControlAction::Emit(CanonicalGodAction::FoodConservation {
            nudge_basis_points: FOOD_CONSERVATION_STEP_BASIS_POINTS,
        }),
    );
    spawn_control(
        commands,
        pane,
        "conservation-less",
        11,
        "Ease food conservation",
        enabled,
        Lai50ControlAction::Emit(CanonicalGodAction::FoodConservation {
            nudge_basis_points: -FOOD_CONSERVATION_STEP_BASIS_POINTS,
        }),
    );
    spawn_control(
        commands,
        pane,
        "nudge-food",
        12,
        "Nudge broad food priority",
        enabled,
        Lai50ControlAction::Emit(CanonicalGodAction::BroadDomainNudge {
            domain: NudgeDomain::Food,
            building_kind_id: None,
            basis_points: FOOD_PRIORITY_STEP_BASIS_POINTS,
        }),
    );
}

fn render_operations(
    commands: &mut Commands<'_, '_>,
    pane: Entity,
    projection: &Lai50FoodProjection,
) {
    spawn_section(
        commands,
        pane,
        "Cookhouse batches",
        &batches_text(&projection.cookhouse),
    );
    for (index, batch) in projection.cookhouse.batches.iter().enumerate() {
        spawn_control(
            commands,
            pane,
            &batch.semantic_id,
            100 + index as u32,
            &format!("Inspect {} at {}", batch.recipe_name, batch.station_id),
            true,
            Lai50ControlAction::SelectBatch(batch.batch_id.clone()),
        );
    }
    spawn_section(
        commands,
        pane,
        "Reported Cookhouse modifiers",
        &availability_text(&projection.cookhouse.reported_modifiers),
    );
    spawn_section(
        commands,
        pane,
        "Fishing Huts",
        &fishing_huts_text(&projection.fishing_huts),
    );
    for (index, hut) in projection.fishing_huts.iter().enumerate() {
        spawn_control(
            commands,
            pane,
            &hut.semantic_id,
            400 + index as u32,
            &format!("Inspect Fishing Hut {}", hut.hut_id),
            true,
            Lai50ControlAction::SelectHut(hut.hut_id.clone()),
        );
    }
    spawn_section(
        commands,
        pane,
        "Current physical food work and cargo",
        &tasks_text(&projection.current_physical_tasks),
    );
    for (index, task) in projection.current_physical_tasks.iter().enumerate() {
        spawn_control(
            commands,
            pane,
            &task.semantic_id,
            700 + index as u32,
            &format!("Inspect task {}", task.task_id),
            true,
            Lai50ControlAction::SelectTask(task.task_id.clone()),
        );
    }
    let enabled = projection.state.permits_remote_intent();
    spawn_control(
        commands,
        pane,
        "nudge-cookhouse",
        1_000,
        "Nudge Cookhouse priority",
        enabled,
        Lai50ControlAction::Emit(CanonicalGodAction::BroadDomainNudge {
            domain: NudgeDomain::Food,
            building_kind_id: Some(stable_id("station_cookhouse")),
            basis_points: FOOD_PRIORITY_STEP_BASIS_POINTS,
        }),
    );
    spawn_control(
        commands,
        pane,
        "nudge-fishing-hut",
        1_001,
        "Nudge Fishing Hut priority",
        enabled,
        Lai50ControlAction::Emit(CanonicalGodAction::BroadDomainNudge {
            domain: NudgeDomain::Food,
            building_kind_id: Some(stable_id("station_fishing_hut")),
            basis_points: FOOD_PRIORITY_STEP_BASIS_POINTS,
        }),
    );
}

fn spawn_section(commands: &mut Commands<'_, '_>, parent: Entity, heading: &str, body: &str) {
    let section = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::bottom(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(STONE),
            Name::new(format!("LAI.50 {heading} section")),
        ))
        .id();
    commands.entity(parent).add_child(section);
    commands.entity(section).with_children(|section| {
        section.spawn(text_bundle(heading, 16.0, INK));
        section.spawn(text_bundle(
            if body.trim().is_empty() {
                "Nothing reported in this section."
            } else {
                body
            },
            12.0,
            Color::srgb(0.26, 0.21, 0.17),
        ));
    });
}

#[allow(clippy::too_many_arguments)]
fn spawn_control(
    commands: &mut Commands<'_, '_>,
    parent: Entity,
    subject: &str,
    focus_order: u32,
    label: &str,
    enabled: bool,
    action: Lai50ControlAction,
) {
    let semantic_id = stable_semantic_id("control", subject);
    let control = commands
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(36.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(STONE),
            semantic_node(Role::Button, semantic_id.clone(), label, enabled),
            Lai50Control {
                stable_id: semantic_id,
                focus_order,
                enabled,
                action,
            },
            Name::new(format!("LAI.50 {label}")),
        ))
        .id();
    commands.entity(control).with_children(|button| {
        button.spawn(text_bundle(label, 12.0, if enabled { INK } else { STONE }));
    });
    commands.entity(parent).add_child(control);
}

fn handle_pointer_controls(
    mut interactions: Query<'_, '_, (&Interaction, &Lai50Control), Changed<Interaction>>,
    mut view: ResMut<'_, Lai50ViewState>,
    mut intent: ResMut<'_, Lai50ActionIntent>,
) {
    for (interaction, control) in &mut interactions {
        if *interaction == Interaction::Pressed {
            view.focused_control_id = Some(control.stable_id.clone());
            apply_control(control, &mut view, &mut intent);
        }
    }
}

fn handle_keyboard_controls(
    keys: Option<Res<'_, ButtonInput<KeyCode>>>,
    visibility: Res<'_, Lai50PanelVisibility>,
    controls: Query<'_, '_, &Lai50Control>,
    mut view: ResMut<'_, Lai50ViewState>,
    mut intent: ResMut<'_, Lai50ActionIntent>,
) {
    let Some(keys) = keys else {
        return;
    };
    if !visibility.visible {
        return;
    }
    let mut visible = controls.iter().cloned().collect::<Vec<_>>();
    visible.sort_by(|left, right| {
        left.focus_order
            .cmp(&right.focus_order)
            .then_with(|| left.stable_id.cmp(&right.stable_id))
    });
    if visible.is_empty() {
        return;
    }
    let reverse = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let navigating = keys.just_pressed(KeyCode::Tab)
        || keys.just_pressed(KeyCode::ArrowDown)
        || keys.just_pressed(KeyCode::ArrowRight)
        || keys.just_pressed(KeyCode::ArrowUp)
        || keys.just_pressed(KeyCode::ArrowLeft);
    if navigating {
        let backward =
            reverse || keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::ArrowLeft);
        let current = view
            .focused_control_id
            .as_ref()
            .and_then(|id| visible.iter().position(|control| &control.stable_id == id));
        let next = match (current, backward) {
            (None, false) => 0,
            (None, true) | (Some(0), true) => visible.len() - 1,
            (Some(index), true) => index - 1,
            (Some(index), false) => (index + 1) % visible.len(),
        };
        view.focused_control_id = Some(visible[next].stable_id.clone());
    }
    if (keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space))
        && let Some(control) = view
            .focused_control_id
            .as_ref()
            .and_then(|id| visible.iter().find(|control| &control.stable_id == id))
    {
        apply_control(control, &mut view, &mut intent);
    }
}

fn handle_accessibility_controls(
    mut requests: MessageReader<'_, '_, AccessibilityActionRequest>,
    controls: Query<'_, '_, &Lai50Control>,
    mut view: ResMut<'_, Lai50ViewState>,
    mut intent: ResMut<'_, Lai50ActionIntent>,
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
            apply_control(control, &mut view, &mut intent);
        }
    }
}

fn apply_control(
    control: &Lai50Control,
    view: &mut Lai50ViewState,
    intent: &mut Lai50ActionIntent,
) {
    if !control.enabled {
        view.local_feedback =
            Some("This control is unavailable until a current report is loaded.".to_owned());
        return;
    }
    match &control.action {
        Lai50ControlAction::Refresh => {
            view.refresh_requests = view.refresh_requests.saturating_add(1);
            view.local_feedback = Some("A report refresh was requested.".to_owned());
        }
        Lai50ControlAction::SelectBatch(batch_id) => {
            view.selected_batch_id = Some(batch_id.clone());
        }
        Lai50ControlAction::SelectHut(hut_id) => {
            view.selected_hut_id = Some(hut_id.clone());
        }
        Lai50ControlAction::SelectTask(task_id) => {
            view.selected_task_id = Some(task_id.clone());
        }
        Lai50ControlAction::Emit(action) if is_lai50_allowed_action(action) => {
            intent.sequence = intent.sequence.saturating_add(1);
            intent.pending = Some(action.clone());
            view.local_feedback = Some(
                "Broad food intent prepared; authenticated transport and the Leader remain authoritative."
                    .to_owned(),
            );
        }
        Lai50ControlAction::Emit(_) => {
            view.local_feedback =
                Some("This Food surface rejects direct or unrelated authority.".to_owned());
        }
    }
}

fn sync_focus_style(
    view: Res<'_, Lai50ViewState>,
    mut controls: Query<'_, '_, (&Lai50Control, &mut BackgroundColor, &mut BorderColor)>,
) {
    if !view.is_changed() {
        return;
    }
    for (control, mut background, mut border) in &mut controls {
        if view.focused_control_id.as_deref() == Some(control.stable_id.as_str()) {
            background.0 = PAPER_SHADE;
            border.set_all(MOSS);
        } else {
            background.0 = Color::NONE;
            border.set_all(STONE);
        }
    }
}

fn sync_layout(
    windows: Query<'_, '_, &Window, With<PrimaryWindow>>,
    mut roots: Query<'_, '_, &mut Node, With<Lai50Root>>,
    mut workspaces: Query<'_, '_, &mut Node, (With<Lai50Workspace>, Without<Lai50Root>)>,
    mut panes: Query<'_, '_, (&Lai50Pane, &mut Node), Without<Lai50Root>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let logical_width = window.width() / window.scale_factor();
    let wide = logical_width >= 1_180.0;
    if let Ok(mut root) = roots.single_mut() {
        let gutter = if wide { 24.0 } else { 12.0 };
        root.left = Val::Px(gutter);
        root.right = Val::Px(gutter);
        root.bottom = Val::Px(gutter);
    }
    for mut workspace in &mut workspaces {
        workspace.flex_direction = if wide {
            FlexDirection::Row
        } else {
            FlexDirection::Column
        };
    }
    for (marker, mut pane) in &mut panes {
        pane.width = Val::Percent(if wide {
            match marker.0 {
                Lai50PaneKind::FoodLedger => 42.0,
                Lai50PaneKind::Operations => 58.0,
            }
        } else {
            100.0
        });
        pane.height = if wide { Val::Percent(100.0) } else { Val::Auto };
        pane.min_height = Val::Px(if wide { 240.0 } else { 280.0 });
    }
}

fn handle_scroll(
    mut wheel: MessageReader<'_, '_, MouseWheel>,
    mut panes: Query<
        '_,
        '_,
        (&Interaction, &Node, &ComputedNode, &mut ScrollPosition),
        With<Lai50ScrollablePane>,
    >,
) {
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
    for (interaction, node, computed, mut position) in &mut panes {
        if *interaction != Interaction::Hovered || node.display == Display::None {
            continue;
        }
        let maximum = ((computed.content_size().y - computed.size().y)
            * computed.inverse_scale_factor())
        .max(0.0);
        position.y = (position.y + delta).clamp(0.0, maximum);
    }
}

fn state_copy(state: &Lai50SurfaceState) -> String {
    match state {
        Lai50SurfaceState::Loading => "Loading the report-safe food snapshot.".to_owned(),
        Lai50SurfaceState::Ready => "Current food report loaded.".to_owned(),
        Lai50SurfaceState::Empty => "No selected-colony food report is available.".to_owned(),
        Lai50SurfaceState::Stale { stale_since_ms } => format!(
            "Food report is stale since {stale_since_ms}; values remain the last received report and actions are disabled."
        ),
        Lai50SurfaceState::UpdateRequired => {
            "Client update required before this report can refresh.".to_owned()
        }
        Lai50SurfaceState::Error { message } => format!("Food report unavailable: {message}"),
    }
}

fn availability_text<T: std::fmt::Debug>(availability: &Lai50Availability<T>) -> String {
    match availability {
        Lai50Availability::Reported(value) => format!("{value:?}"),
        Lai50Availability::Unavailable { reason } => format!("Not reported — {reason}"),
    }
}

fn stocks_text(rows: &[FoodStockRow]) -> String {
    if rows.is_empty() {
        return "No concrete food lots are reported. No generic Food value is substituted."
            .to_owned();
    }
    rows.iter()
        .take(MAX_LAI50_RENDERED_ROWS)
        .map(|row| {
            format!(
                "{}  Q{} {} · nutrition {} · spoilage {} · {:?} · location {} · source {}",
                row.display_name,
                quality_ordinal(row.quality),
                row.total_quantity,
                join_u16(&row.nutrition_basis_points),
                join_u16(&row.spoilage_basis_points),
                row.permissions,
                row.location_site_ids.join(", "),
                availability_text(&row.provenance),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn permissions_text(rows: &[FoodPermissionRow]) -> String {
    if rows.is_empty() {
        return "No content-specific food permissions are reported.".to_owned();
    }
    rows.iter()
        .take(MAX_LAI50_RENDERED_ROWS)
        .map(|row| {
            format!(
                "{} · {:?} · {:?} confidence · {}",
                row.display_name, row.permission, row.confidence, row.reason
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn source_reports_text(rows: &[FoodSourceReportRow]) -> String {
    if rows.is_empty() {
        return "No food, Apple, Fish, farm, water, Hunting, or ecology event report is present. Exact regeneration and habitat stock remain hidden.".to_owned();
    }
    rows.iter()
        .take(MAX_LAI50_RENDERED_ROWS)
        .map(|row| {
            format!(
                "{} · {} · {:?} · {}",
                row.occurred_at_ms, row.domain_id, row.confidence, row.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn batches_text(cookhouse: &CookhouseProjection) -> String {
    if cookhouse.batches.is_empty() {
        return format!(
            "No Cookhouse batches reported.\nQueue: {}",
            availability_text(&cookhouse.queue_order)
        );
    }
    cookhouse
        .batches
        .iter()
        .take(MAX_LAI50_RENDERED_ROWS)
        .map(|batch| {
            format!(
                "{} · {:?} {}/10000 · worker {} · ingredients {} · outputs {} · {}",
                batch.recipe_name,
                batch.stage,
                batch.progress_basis_points,
                batch.worker_cat_id.as_deref().unwrap_or("not reported"),
                list_or_none(&batch.ingredient_lot_ids),
                list_or_none(&batch.output_lot_ids),
                batch
                    .blocker
                    .as_deref()
                    .map_or("no blocker reported", |blocker| blocker),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn fishing_huts_text(rows: &[FishingHutRow]) -> String {
    if rows.is_empty() {
        return "No Fishing Hut is reported. Hand-fishing difficulty or habitat stock is not inferred."
            .to_owned();
    }
    rows.iter()
        .take(MAX_LAI50_RENDERED_ROWS)
        .map(|hut| {
            let rod = match &hut.rod {
                Lai50Availability::Reported(rod) => format!(
                    "{:?} {} Rod · {}% wear · {}% durability",
                    rod.quality,
                    rod.material_id,
                    basis_points_percent(rod.wear_basis_points),
                    basis_points_percent(rod.remaining_durability_basis_points),
                ),
                Lai50Availability::Unavailable { reason } => format!("Rod not reported — {reason}"),
            };
            format!(
                "{} · mode {} · {:?} {}/10000 · land {:?} · dock {:?} → water {:?} · Hut bonus {} · {} · habitat {:?}: {}",
                hut.hut_id,
                hut.mode_id,
                hut.stage,
                hut.progress_basis_points,
                hut.ordered_land_footprint,
                hut.dock_land_tile,
                hut.reserved_water_tile,
                availability_text(&hut.hut_bonus),
                rod,
                hut.report_confidence,
                hut.habitat_report,
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn tasks_text(rows: &[FoodPhysicalTaskRow]) -> String {
    if rows.is_empty() {
        return "No current physical food task is reported.".to_owned();
    }
    rows.iter()
        .take(MAX_LAI50_RENDERED_ROWS)
        .map(|task| {
            let cargo = task
                .cargo
                .iter()
                .map(|cargo| {
                    format!(
                        "{} {} Q{} [{}]",
                        cargo.content_id, cargo.quantity, cargo.quality_band, cargo.cargo_id
                    )
                })
                .collect::<Vec<_>>();
            format!(
                "{} · {:?} · {} at {} ({}) · objective {:?} · work {} · route {:?} · cargo {} · blockers {}",
                task.task_kind_id,
                task.state,
                task.objective,
                task.site_id,
                task.site_kind_id,
                task.ordered_objective_footprint,
                task.work_sites.len(),
                task.ordered_route,
                list_or_none(&cargo),
                list_or_none(&task.blockers),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn join_u16(values: &[u16]) -> String {
    if values.is_empty() {
        "not reported".to_owned()
    } else {
        values
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join("/")
    }
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(", ")
    }
}

fn basis_points_percent(value: u16) -> String {
    format!("{}.{:02}", value / 100, value % 100)
}

fn stable_id(value: &str) -> StableId {
    StableId::new(value).expect("static LAI.50 canonical stable ID is valid")
}

fn text_bundle(value: impl Into<String>, font_size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font_size: FontSize::Px(font_size),
            ..default()
        },
        TextColor(color),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn food_task_filter_accepts_real_food_work_and_exact_food_cargo() {
        let foods = BTreeSet::from(["food_apple".to_owned()]);
        assert!(is_food_physical_task(
            "fetch_water",
            "resource_source:water",
            std::iter::empty(),
            &foods,
        ));
        assert!(is_food_physical_task(
            "haul",
            "storage:zone",
            ["food_apple"].into_iter(),
            &foods,
        ));
        assert!(!is_food_physical_task(
            "build_road",
            "construction:road",
            ["material_stone"].into_iter(),
            &foods,
        ));
    }

    #[test]
    fn food_surface_allow_list_rejects_direct_or_unrelated_authority() {
        assert!(is_lai50_allowed_action(
            &CanonicalGodAction::FoodConservation {
                nudge_basis_points: 1_000,
            }
        ));
        assert!(is_lai50_allowed_action(
            &CanonicalGodAction::BroadDomainNudge {
                domain: NudgeDomain::Food,
                building_kind_id: Some(stable_id("station_cookhouse")),
                basis_points: 1_000,
            }
        ));
        assert!(!is_lai50_allowed_action(
            &CanonicalGodAction::BroadDomainNudge {
                domain: NudgeDomain::Trade,
                building_kind_id: None,
                basis_points: 1_000,
            }
        ));
        assert!(!is_lai50_allowed_action(
            &CanonicalGodAction::ResearchQueue {
                study_id: stable_id("study_cooking"),
            }
        ));
    }

    #[test]
    fn rod_wear_is_only_the_complement_of_reported_durability() {
        let remaining = 6_100_u16;
        let wear = 10_000_u16.saturating_sub(remaining);
        assert_eq!(wear, 3_900);
        assert_eq!(basis_points_percent(remaining), "61.00");
        assert_eq!(basis_points_percent(wear), "39.00");
    }

    #[test]
    fn semantic_ids_are_bounded_and_stable() {
        let source = format!("hut:{}", "east-water-edge".repeat(20));
        let left = stable_semantic_id("fishing-hut", &source);
        let right = stable_semantic_id("fishing-hut", &source);
        assert_eq!(left, right);
        assert!(left.len() < 100);
        assert!(left.starts_with("lai50:fishing-hut:"));
    }
}
