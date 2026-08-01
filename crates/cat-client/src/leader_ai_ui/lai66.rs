//! LAI.66 report-safe Log, Stores, and Village primary surfaces.
//!
//! This leaf consumes only [`cat_protocol::lai64::CanonicalSnapshotEnvelope`].
//! It may sort, filter, page, group repeated reports, and format values that
//! already crossed that boundary. It never reads simulation authority, infers
//! hidden regeneration, predicts capacity, or manufactures family relations.
//! Missing protocol fields remain visibly unavailable until the server reports
//! them.

use std::collections::{BTreeMap, BTreeSet};

use accesskit::{Action, Role};
use bevy::a11y::{AccessibilityNode, ActionRequest as AccessibilityActionRequest};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use cat_protocol::lai64::{
    CanonicalColonySnapshot, CanonicalSnapshotEnvelope, FoodPermission, LifeStageSnapshot,
    PhysicalTaskSnapshot, QualityBandSnapshot, ReportConfidence, StorageZoneSnapshotV2, TaskState,
};

use super::{
    lai54::{
        bevy_shell::{Lai54LiveShell, Lai54ShellRoot, ui_scale_for_window_scale},
        layout::{CharterPlacement, ClientPlatform, LayoutMode, Viewport, shell_layout},
        shell::PrimaryScreen,
    },
    semantic_node, semantic_status_node,
};

pub const MAX_LOG_GROUPS_PER_PAGE: usize = 100;
pub const DEFAULT_LOG_GROUPS_PER_PAGE: usize = 40;
pub const REPEATED_EVENT_WINDOW_MS: i64 = 15 * 60 * 1_000;
pub const MAX_RENDERED_REPORT_ROWS: usize = 200;

const INK: Color = Color::srgb(0.153, 0.106, 0.086);
const PARCHMENT: Color = Color::srgb(0.937, 0.886, 0.741);
const PAPER_SHADE: Color = Color::srgb(0.866, 0.792, 0.635);
const DARK_FOREST: Color = Color::srgb(0.090, 0.235, 0.180);
const WOOD: Color = Color::srgb(0.427, 0.282, 0.169);
const STONE: Color = Color::srgb(0.48, 0.46, 0.39);
const MOSS: Color = Color::srgb(0.310, 0.439, 0.251);
const RUST: Color = Color::srgb(0.643, 0.286, 0.176);
const DANGER: Color = Color::srgb(0.58, 0.20, 0.18);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Lai66RefreshState {
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
pub struct Lai66SnapshotFeed {
    pub envelope: Option<CanonicalSnapshotEnvelope>,
    pub refresh: Lai66RefreshState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lai66SurfaceState {
    Loading,
    Ready,
    Empty,
    Stale { stale_since_ms: i64 },
    UpdateRequired,
    Error { message: String },
}

impl Default for Lai66SurfaceState {
    fn default() -> Self {
        Self::Loading
    }
}

impl Lai66SurfaceState {
    #[must_use]
    pub const fn keeps_last_report_visible(&self) -> bool {
        matches!(self, Self::Ready | Self::Empty | Self::Stale { .. })
    }

    #[must_use]
    pub const fn blocks_authoritative_mutation(&self) -> bool {
        true
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReportAvailability<T> {
    Reported(T),
    Unavailable { reason: String },
}

impl<T> Default for ReportAvailability<T> {
    fn default() -> Self {
        Self::Unavailable {
            reason: "This field has not been reported.".to_owned(),
        }
    }
}

impl<T> ReportAvailability<T> {
    #[must_use]
    pub const fn is_reported(&self) -> bool {
        matches!(self, Self::Reported(_))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogFilters {
    pub domain_id: Option<String>,
    pub query: String,
    pub from_ms: Option<i64>,
    pub page_offset: usize,
    pub page_size: usize,
}

impl Default for LogFilters {
    fn default() -> Self {
        Self {
            domain_id: None,
            query: String::new(),
            from_ms: None,
            page_offset: 0,
            page_size: DEFAULT_LOG_GROUPS_PER_PAGE,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoreFilters {
    pub zone_id: Option<String>,
    pub content_id: Option<String>,
    pub permission: Option<FoodPermission>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VillageFilters {
    pub household_id: Option<String>,
    pub residence_id: Option<String>,
    pub office_id: Option<String>,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai66ViewState {
    pub log_filters: LogFilters,
    pub store_filters: StoreFilters,
    pub village_filters: VillageFilters,
    pub selected_log_group_id: Option<String>,
    pub selected_zone_id: Option<String>,
    pub selected_household_id: Option<String>,
    pub focused_control_id: Option<String>,
    pub refresh_requests: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lai66FocusRefresh {
    Preserved,
    MovedToScreenRefresh,
    Empty,
}

pub fn retain_lai66_focus_after_refresh<'a>(
    focused_control_id: &mut Option<String>,
    visible_control_ids: impl IntoIterator<Item = &'a str>,
    screen_refresh_control_id: &str,
) -> Lai66FocusRefresh {
    let Some(focused) = focused_control_id.as_deref() else {
        return Lai66FocusRefresh::Empty;
    };
    if visible_control_ids
        .into_iter()
        .any(|candidate| candidate == focused)
    {
        Lai66FocusRefresh::Preserved
    } else {
        *focused_control_id = Some(screen_refresh_control_id.to_owned());
        Lai66FocusRefresh::MovedToScreenRefresh
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai66ReportsProjection {
    pub selected_colony_id: Option<String>,
    pub snapshot_now_ms: Option<i64>,
    pub state_version: Option<u64>,
    pub log: LogProjection,
    pub stores: StoresProjection,
    pub village: VillageProjection,
    pub reads_authoritative_world_truth: bool,
    pub recomputes_hidden_rules: bool,
    pub exposes_mutation_controls: bool,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai66ProjectionResource(pub Lai66ReportsProjection);

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
struct Lai66RenderState {
    dirty: bool,
}

impl Default for Lai66RenderState {
    fn default() -> Self {
        Self { dirty: true }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LogProjection {
    pub state: Lai66SurfaceState,
    pub authoritative_history_coverage: ReportAvailability<String>,
    pub available_domain_ids: Vec<String>,
    pub total_reported_events: usize,
    pub total_grouped_rows: usize,
    pub visible_groups: Vec<LogEventGroup>,
    pub page_offset: usize,
    pub has_previous_page: bool,
    pub has_next_page: bool,
    pub selected_group: Option<LogEventGroup>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEventGroup {
    pub group_id: String,
    pub domain_id: String,
    pub event_kind_id: String,
    pub summary: String,
    pub first_occurred_at_ms: i64,
    pub last_occurred_at_ms: i64,
    pub repeat_count: usize,
    pub ledger_event_ids: Vec<String>,
    pub source_event_ids: Vec<String>,
    pub confidence: ReportAvailability<ReportConfidence>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoresProjection {
    pub state: Lai66SurfaceState,
    pub zone_count: usize,
    pub visible_loose_slots: usize,
    pub occupied_loose_slots: usize,
    pub container_count: usize,
    pub reported_lot_count: usize,
    pub zones: Vec<StorageZoneRow>,
    pub food_permissions: Vec<FoodPermissionRow>,
    pub selected_zone: Option<StorageZoneRow>,
    pub explicit_workshop_zone_links: ReportAvailability<Vec<WorkshopZoneLinkRow>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageZoneRow {
    pub zone_id: String,
    pub semantic_id: String,
    pub linked_workshop_id: Option<String>,
    pub ordered_footprint: Vec<(i32, i32)>,
    pub tile_count: usize,
    pub visible_slot_capacity: usize,
    pub occupied_slots: usize,
    pub containers: Vec<ContainerRow>,
    pub lots: Vec<StoreLotRow>,
    pub unique_items: Vec<StoreItemRow>,
    pub rare_materials: Vec<StoreRareMaterialRow>,
    pub linked_hauling: Vec<StoreHaulRow>,
    pub blockers: Vec<StoreBlockerRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContainerRow {
    pub container_id: String,
    pub container_kind_id: String,
    pub capacity_slots: u8,
    pub fullness_basis_points: u16,
    pub contained_content_id: Option<String>,
    pub internal_lot_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreLotRow {
    pub lot_id: String,
    pub content_id: String,
    pub display_name: String,
    pub quantity: u64,
    pub quality_label: String,
    pub provenance_id: String,
    pub reported_age_ms: Option<u64>,
    pub reservation_id: Option<String>,
    pub container_id: Option<String>,
    pub location_site_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreItemRow {
    pub item_id: String,
    pub definition_id: String,
    pub material_id: String,
    pub quality_label: String,
    pub durability_basis_points: u16,
    pub provenance_id: String,
    pub reservation_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreRareMaterialRow {
    pub material_instance_id: String,
    pub material_id: String,
    pub content_state_id: String,
    pub processed: bool,
    pub quality_label: String,
    pub provenance_id: String,
    pub reservation_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreHaulRow {
    pub task_id: String,
    pub task_kind_id: String,
    pub state: TaskState,
    pub ordered_route: Vec<(i32, i32)>,
    pub worker_cat_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreBlockerRow {
    pub task_id: String,
    pub blocker_id: String,
    pub reason: String,
    pub recoverable: bool,
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
pub struct WorkshopZoneLinkRow {
    pub workshop_id: String,
    pub zone_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VillageProjection {
    pub state: Lai66SurfaceState,
    pub demographics: DemographicsProjection,
    pub employment: Vec<EmploymentRow>,
    pub durable_job_assignments: ReportAvailability<Vec<JobAssignmentRow>>,
    pub households: Vec<HouseholdRow>,
    pub housing: Vec<ResidenceRow>,
    pub partnerships: ReportAvailability<Vec<PartnershipRow>>,
    pub traditions: Vec<TraditionRow>,
    pub enterprises: Vec<EnterpriseRow>,
    pub election: ElectionProjection,
    pub officers: Vec<OfficerRow>,
    pub succession: ReportAvailability<String>,
    pub selected_household: Option<HouseholdRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DemographicsProjection {
    pub reported_resident_count: usize,
    pub succession_eligible_count: usize,
    pub assigned_office_count: usize,
    pub unassigned_household_count: usize,
    pub life_stage_counts: ReportAvailability<Vec<NamedCount>>,
}

impl Default for DemographicsProjection {
    fn default() -> Self {
        Self {
            reported_resident_count: 0,
            succession_eligible_count: 0,
            assigned_office_count: 0,
            unassigned_household_count: 0,
            life_stage_counts: ReportAvailability::Unavailable {
                reason: "Life stages are unavailable until a selected colony report is loaded."
                    .to_owned(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedCount {
    pub id: String,
    pub count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmploymentRow {
    pub cat_id: String,
    pub display_name: String,
    pub job_id: Option<String>,
    pub office_id: Option<String>,
    pub active_task_ids: Vec<String>,
    pub active_task_kind_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobAssignmentRow {
    pub assignment_id: String,
    pub cat_id: String,
    pub job_kind_id: String,
    pub station_id: Option<String>,
    pub active: bool,
    pub report_reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HouseholdRow {
    pub household_id: String,
    pub semantic_id: String,
    pub resident_cat_ids: Vec<String>,
    pub resident_names: Vec<String>,
    pub residence_ids: Vec<String>,
    pub parent_child_edges: Vec<(String, String)>,
    pub tradition_ids: Vec<String>,
    pub enterprise_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResidenceRow {
    pub residence_id: String,
    pub housing_kind_id: Option<String>,
    pub ordered_footprint: Vec<(i32, i32)>,
    pub resident_cat_ids: Vec<String>,
    pub reported_capacity: ReportAvailability<u16>,
    pub housing_pressure: ReportAvailability<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartnershipRow {
    pub partnership_id: String,
    pub cat_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraditionRow {
    pub tradition_id: String,
    pub cat_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnterpriseRow {
    pub enterprise_id: String,
    pub cat_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ElectionProjection {
    pub election_id: Option<String>,
    pub candidates: Vec<ElectionCandidateRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElectionCandidateRow {
    pub cat_id: String,
    pub display_name: String,
    pub report_reason: String,
    pub backing_blocks: u8,
    pub eligible: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfficerRow {
    pub office_id: String,
    pub cat_id: Option<String>,
    pub display_name: Option<String>,
    pub report_expertise_level: u8,
    pub appointment_candidate_ids: Vec<String>,
}

#[must_use]
pub fn project_lai66_reports(
    feed: &Lai66SnapshotFeed,
    view: &Lai66ViewState,
) -> Lai66ReportsProjection {
    let Some(envelope) = &feed.envelope else {
        return projection_without_snapshot(&feed.refresh);
    };
    if let Err(error) = envelope.validate() {
        return projection_error(format!("Invalid report snapshot: {error}"));
    }
    let Some(colony) = envelope.colonies.first() else {
        return projection_error("Selected colony report is missing.".to_owned());
    };

    let log = project_log(envelope, colony, &feed.refresh, view);
    let stores = project_stores(envelope, colony, &feed.refresh, view);
    let village = project_village(colony, &feed.refresh, view);
    Lai66ReportsProjection {
        selected_colony_id: Some(envelope.selected_colony_id.as_str().to_owned()),
        snapshot_now_ms: Some(envelope.now_ms),
        state_version: Some(colony.state_version),
        log,
        stores,
        village,
        reads_authoritative_world_truth: false,
        recomputes_hidden_rules: false,
        exposes_mutation_controls: false,
    }
}

fn projection_without_snapshot(refresh: &Lai66RefreshState) -> Lai66ReportsProjection {
    let state = state_for(refresh, true);
    Lai66ReportsProjection {
        log: LogProjection {
            state: state.clone(),
            ..default()
        },
        stores: StoresProjection {
            state: state.clone(),
            explicit_workshop_zone_links: unavailable_workshop_links(),
            ..default()
        },
        village: VillageProjection {
            state,
            partnerships: unavailable_partnerships(),
            ..default()
        },
        ..default()
    }
}

fn projection_error(message: String) -> Lai66ReportsProjection {
    projection_without_snapshot(&Lai66RefreshState::Error {
        message: bounded_copy(&message, 240),
    })
}

fn state_for(refresh: &Lai66RefreshState, empty: bool) -> Lai66SurfaceState {
    match refresh {
        Lai66RefreshState::Loading => Lai66SurfaceState::Loading,
        Lai66RefreshState::Ready if empty => Lai66SurfaceState::Empty,
        Lai66RefreshState::Ready => Lai66SurfaceState::Ready,
        Lai66RefreshState::Stale { stale_since_ms } => Lai66SurfaceState::Stale {
            stale_since_ms: *stale_since_ms,
        },
        Lai66RefreshState::UpdateRequired => Lai66SurfaceState::UpdateRequired,
        Lai66RefreshState::Error { message } => Lai66SurfaceState::Error {
            message: bounded_copy(message, 240),
        },
    }
}

fn project_log(
    _envelope: &CanonicalSnapshotEnvelope,
    colony: &CanonicalColonySnapshot,
    refresh: &Lai66RefreshState,
    view: &Lai66ViewState,
) -> LogProjection {
    let domain_ids = colony
        .event_log
        .iter()
        .map(|event| event.domain_id.as_str().to_owned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let query = view.log_filters.query.trim().to_ascii_lowercase();
    let mut events = colony
        .event_log
        .iter()
        .filter(|event| {
            view.log_filters
                .domain_id
                .as_ref()
                .is_none_or(|domain| event.domain_id.as_str() == domain)
                && view
                    .log_filters
                    .from_ms
                    .is_none_or(|from| event.occurred_at_ms >= from)
                && (query.is_empty()
                    || event.message.as_str().to_ascii_lowercase().contains(&query)
                    || event
                        .domain_id
                        .as_str()
                        .to_ascii_lowercase()
                        .contains(&query)
                    || event
                        .event_kind_id
                        .as_str()
                        .to_ascii_lowercase()
                        .contains(&query))
        })
        .collect::<Vec<_>>();
    events.sort_by(|left, right| {
        left.occurred_at_ms
            .cmp(&right.occurred_at_ms)
            .then_with(|| left.event_id.cmp(&right.event_id))
    });

    let mut groups = Vec::<LogEventGroup>::new();
    for event in &events {
        let merges = groups.last().is_some_and(|prior| {
            prior.domain_id == event.domain_id.as_str()
                && prior.event_kind_id == event.event_kind_id.as_str()
                && prior.summary == event.message.as_str()
                && event
                    .occurred_at_ms
                    .saturating_sub(prior.last_occurred_at_ms)
                    <= REPEATED_EVENT_WINDOW_MS
        });
        if merges {
            let prior = groups.last_mut().expect("checked above");
            prior.last_occurred_at_ms = event.occurred_at_ms;
            prior.repeat_count = prior
                .repeat_count
                .saturating_add(usize::from(event.repeated_count));
            prior
                .ledger_event_ids
                .push(event.event_id.as_str().to_owned());
            prior
                .source_event_ids
                .extend(event.source_ids.iter().map(|id| id.as_str().to_owned()));
        } else {
            groups.push(LogEventGroup {
                group_id: stable_semantic_id("log", event.event_id.as_str()),
                domain_id: event.domain_id.as_str().to_owned(),
                event_kind_id: event.event_kind_id.as_str().to_owned(),
                summary: event.message.as_str().to_owned(),
                first_occurred_at_ms: event.occurred_at_ms,
                last_occurred_at_ms: event.occurred_at_ms,
                repeat_count: usize::from(event.repeated_count),
                ledger_event_ids: vec![event.event_id.as_str().to_owned()],
                source_event_ids: event
                    .source_ids
                    .iter()
                    .map(|id| id.as_str().to_owned())
                    .collect(),
                confidence: ReportAvailability::Reported(event.confidence),
            });
        }
    }
    for group in &mut groups {
        group.source_event_ids.sort();
        group.source_event_ids.dedup();
    }
    groups.reverse();

    let page_size = view.log_filters.page_size.clamp(1, MAX_LOG_GROUPS_PER_PAGE);
    let page_offset = view.log_filters.page_offset.min(groups.len());
    let visible_groups = groups
        .iter()
        .skip(page_offset)
        .take(page_size)
        .cloned()
        .collect::<Vec<_>>();
    let selected_group = view
        .selected_log_group_id
        .as_ref()
        .and_then(|selected| groups.iter().find(|group| &group.group_id == selected))
        .cloned();

    LogProjection {
        state: state_for(refresh, groups.is_empty()),
        authoritative_history_coverage: if colony.event_log.is_empty() {
            ReportAvailability::Unavailable {
                reason: "The authoritative event-log collection is empty; there is no history entry to display.".to_owned(),
            }
        } else {
            ReportAvailability::Reported(
                "Complete authoritative event ledger reported for the selected colony.".to_owned(),
            )
        },
        available_domain_ids: domain_ids,
        total_reported_events: events
            .iter()
            .map(|event| usize::from(event.repeated_count))
            .sum(),
        total_grouped_rows: groups.len(),
        visible_groups,
        page_offset,
        has_previous_page: page_offset > 0,
        has_next_page: page_offset.saturating_add(page_size) < groups.len(),
        selected_group,
    }
}

fn project_stores(
    _envelope: &CanonicalSnapshotEnvelope,
    colony: &CanonicalColonySnapshot,
    refresh: &Lai66RefreshState,
    view: &Lai66ViewState,
) -> StoresProjection {
    let names = content_names(colony);
    let permissions = colony
        .hole
        .food_permissions
        .iter()
        .filter(|permission| {
            view.store_filters
                .content_id
                .as_ref()
                .is_none_or(|content| permission.content_id.as_str() == content)
                && view
                    .store_filters
                    .permission
                    .is_none_or(|state| permission.permission == state)
        })
        .map(|permission| FoodPermissionRow {
            content_id: permission.content_id.as_str().to_owned(),
            display_name: names
                .get(permission.content_id.as_str())
                .cloned()
                .unwrap_or_else(|| permission.content_id.as_str().to_owned()),
            permission: permission.permission,
            reason: permission.reason.as_str().to_owned(),
            confidence: permission.confidence,
        })
        .collect::<Vec<_>>();

    let mut zones = colony
        .storage_zones
        .iter()
        .filter(|zone| {
            view.store_filters
                .zone_id
                .as_ref()
                .is_none_or(|id| zone.zone_id.as_str() == id)
        })
        .map(|zone| project_zone(colony, zone, &names, &view.store_filters))
        .collect::<Vec<_>>();
    zones.sort_by(|left, right| left.zone_id.cmp(&right.zone_id));

    let selected_zone = view
        .selected_zone_id
        .as_ref()
        .and_then(|selected| zones.iter().find(|zone| &zone.zone_id == selected))
        .cloned();
    let visible_loose_slots = zones.iter().map(|zone| zone.visible_slot_capacity).sum();
    let occupied_loose_slots = zones.iter().map(|zone| zone.occupied_slots).sum();
    let container_count = zones.iter().map(|zone| zone.containers.len()).sum();
    let reported_lot_count = zones.iter().map(|zone| zone.lots.len()).sum();
    let workshop_links = zones
        .iter()
        .filter_map(|zone| {
            zone.linked_workshop_id
                .as_ref()
                .map(|workshop_id| WorkshopZoneLinkRow {
                    workshop_id: workshop_id.clone(),
                    zone_id: zone.zone_id.clone(),
                })
        })
        .collect::<Vec<_>>();

    StoresProjection {
        state: state_for(refresh, zones.is_empty() && permissions.is_empty()),
        zone_count: zones.len(),
        visible_loose_slots,
        occupied_loose_slots,
        container_count,
        reported_lot_count,
        zones,
        food_permissions: permissions,
        selected_zone,
        explicit_workshop_zone_links: if colony.storage_zones.is_empty() {
            unavailable_workshop_links()
        } else {
            ReportAvailability::Reported(workshop_links)
        },
    }
}

fn project_zone(
    colony: &CanonicalColonySnapshot,
    zone: &StorageZoneSnapshotV2,
    names: &BTreeMap<String, String>,
    filters: &StoreFilters,
) -> StorageZoneRow {
    let mut lots = zone
        .lots
        .iter()
        .filter(|lot| {
            filters
                .content_id
                .as_ref()
                .is_none_or(|content| lot.content_id.as_str() == content)
        })
        .map(|lot| StoreLotRow {
            lot_id: lot.cargo_id.as_str().to_owned(),
            content_id: lot.content_id.as_str().to_owned(),
            display_name: names
                .get(lot.content_id.as_str())
                .cloned()
                .unwrap_or_else(|| lot.content_id.as_str().to_owned()),
            quantity: lot.quantity,
            quality_label: quality_band_number_label(lot.quality_band),
            provenance_id: lot.provenance_id.as_str().to_owned(),
            reported_age_ms: None,
            reservation_id: lot.reservation_id.as_ref().map(|id| id.as_str().to_owned()),
            container_id: lot.container_id.as_ref().map(|id| id.as_str().to_owned()),
            location_site_id: lot.location_site_id.as_ref().map_or_else(
                || zone.zone_id.as_str().to_owned(),
                |id| id.as_str().to_owned(),
            ),
        })
        .collect::<Vec<_>>();
    let known_lot_ids = lots
        .iter()
        .map(|lot| lot.lot_id.clone())
        .collect::<BTreeSet<_>>();
    lots.extend(
        colony
            .quality_lots
            .iter()
            .filter(|lot| {
                lot.location_site_id == zone.zone_id
                    && !known_lot_ids.contains(lot.lot_id.as_str())
                    && filters
                        .content_id
                        .as_ref()
                        .is_none_or(|content| lot.content_id.as_str() == content)
            })
            .map(|lot| StoreLotRow {
                lot_id: lot.lot_id.as_str().to_owned(),
                content_id: lot.content_id.as_str().to_owned(),
                display_name: names
                    .get(lot.content_id.as_str())
                    .cloned()
                    .unwrap_or_else(|| lot.content_id.as_str().to_owned()),
                quantity: lot.quantity,
                quality_label: quality_label(lot.quality),
                provenance_id: lot.provenance_id.as_str().to_owned(),
                reported_age_ms: Some(lot.age_ms),
                reservation_id: lot.reservation_id.as_ref().map(|id| id.as_str().to_owned()),
                container_id: None,
                location_site_id: lot.location_site_id.as_str().to_owned(),
            }),
    );
    lots.sort_by(|left, right| left.lot_id.cmp(&right.lot_id));

    let mut containers = zone
        .containers
        .iter()
        .map(|container| ContainerRow {
            container_id: container.container_id.as_str().to_owned(),
            container_kind_id: container.container_kind_id.as_str().to_owned(),
            capacity_slots: container.capacity_slots,
            fullness_basis_points: container.fullness_basis_points,
            contained_content_id: container
                .contained_content_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            internal_lot_ids: lots
                .iter()
                .filter(|lot| lot.container_id.as_deref() == Some(container.container_id.as_str()))
                .map(|lot| lot.lot_id.clone())
                .collect(),
        })
        .collect::<Vec<_>>();
    containers.sort_by(|left, right| left.container_id.cmp(&right.container_id));

    let mut unique_items = colony
        .exact_items
        .iter()
        .filter(|item| {
            item.location_site_id == zone.zone_id
                && filters
                    .content_id
                    .as_ref()
                    .is_none_or(|content| item.definition_id.as_str() == content)
        })
        .map(|item| StoreItemRow {
            item_id: item.item_id.as_str().to_owned(),
            definition_id: item.definition_id.as_str().to_owned(),
            material_id: item.material_id.as_str().to_owned(),
            quality_label: quality_label(item.quality),
            durability_basis_points: item.durability_basis_points,
            provenance_id: item.provenance_id.as_str().to_owned(),
            reservation_id: item
                .reservation_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
        })
        .collect::<Vec<_>>();
    unique_items.sort_by(|left, right| left.item_id.cmp(&right.item_id));

    let mut rare_materials = colony
        .rare_materials
        .iter()
        .filter(|material| {
            material.location_site_id == zone.zone_id
                && filters.content_id.as_ref().is_none_or(|content| {
                    material.material_id.as_str() == content
                        || material.content_state_id.as_str() == content
                })
        })
        .map(|material| StoreRareMaterialRow {
            material_instance_id: material.material_instance_id.as_str().to_owned(),
            material_id: material.material_id.as_str().to_owned(),
            content_state_id: material.content_state_id.as_str().to_owned(),
            processed: material.processed,
            quality_label: quality_label(material.quality),
            provenance_id: material.provenance_id.as_str().to_owned(),
            reservation_id: material
                .reservation_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
        })
        .collect::<Vec<_>>();
    rare_materials
        .sort_by(|left, right| left.material_instance_id.cmp(&right.material_instance_id));

    let linked_tasks = colony
        .tasks
        .iter()
        .filter(|task| task_references_zone(task, zone.zone_id.as_str()))
        .collect::<Vec<_>>();
    let linked_hauling = linked_tasks
        .iter()
        .map(|task| StoreHaulRow {
            task_id: task.task_id.as_str().to_owned(),
            task_kind_id: task.task_kind_id.as_str().to_owned(),
            state: task.state,
            ordered_route: task
                .route
                .ordered_tiles
                .iter()
                .map(|tile| (tile.x, tile.y))
                .collect(),
            worker_cat_ids: task
                .worker_cat_ids
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
        })
        .collect::<Vec<_>>();
    let blockers = linked_tasks
        .iter()
        .flat_map(|task| {
            task.blockers.iter().map(|blocker| StoreBlockerRow {
                task_id: task.task_id.as_str().to_owned(),
                blocker_id: blocker.blocker_id.as_str().to_owned(),
                reason: blocker.reason.as_str().to_owned(),
                recoverable: blocker.recoverable,
            })
        })
        .collect::<Vec<_>>();
    let visible_slot_capacity = zone.tiles.iter().map(|tile| tile.slots.len()).sum();
    let occupied_slots = zone
        .tiles
        .iter()
        .flat_map(|tile| &tile.slots)
        .filter(|slot| slot.lot_id.is_some() || slot.container_id.is_some())
        .count();

    StorageZoneRow {
        zone_id: zone.zone_id.as_str().to_owned(),
        semantic_id: stable_semantic_id("stores-zone", zone.zone_id.as_str()),
        linked_workshop_id: zone
            .linked_workshop_id
            .as_ref()
            .map(|id| id.as_str().to_owned()),
        ordered_footprint: zone
            .footprint
            .ordered_tiles
            .iter()
            .map(|tile| (tile.x, tile.y))
            .collect(),
        tile_count: zone.tiles.len(),
        visible_slot_capacity,
        occupied_slots,
        containers,
        lots,
        unique_items,
        rare_materials,
        linked_hauling,
        blockers,
    }
}

fn task_references_zone(task: &PhysicalTaskSnapshot, zone_id: &str) -> bool {
    task.site_id.as_str() == zone_id
        || task.cargo.iter().any(|cargo| {
            cargo
                .location_site_id
                .as_ref()
                .is_some_and(|site| site.as_str() == zone_id)
        })
}

fn unavailable_workshop_links() -> ReportAvailability<Vec<WorkshopZoneLinkRow>> {
    ReportAvailability::Unavailable {
        reason: "No selected storage-zone collection is loaded, so Workshop links cannot be shown."
            .to_owned(),
    }
}

fn project_village(
    colony: &CanonicalColonySnapshot,
    refresh: &Lai66RefreshState,
    view: &Lai66ViewState,
) -> VillageProjection {
    let names = colony
        .cats
        .iter()
        .map(|cat| {
            (
                cat.cat_id.as_str().to_owned(),
                cat.display_name.as_str().to_owned(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let active_by_cat = active_tasks_by_cat(colony);

    let employment = colony
        .cats
        .iter()
        .filter(|cat| {
            view.village_filters
                .office_id
                .as_ref()
                .is_none_or(|office| {
                    cat.office_id
                        .as_ref()
                        .is_some_and(|id| id.as_str() == office)
                })
        })
        .map(|cat| {
            let tasks = active_by_cat
                .get(cat.cat_id.as_str())
                .cloned()
                .unwrap_or_default();
            EmploymentRow {
                cat_id: cat.cat_id.as_str().to_owned(),
                display_name: cat.display_name.as_str().to_owned(),
                job_id: cat.job_id.as_ref().map(|id| id.as_str().to_owned()),
                office_id: cat.office_id.as_ref().map(|id| id.as_str().to_owned()),
                active_task_ids: tasks
                    .iter()
                    .map(|task| task.task_id.as_str().to_owned())
                    .collect(),
                active_task_kind_ids: tasks
                    .iter()
                    .map(|task| task.task_kind_id.as_str().to_owned())
                    .collect(),
            }
        })
        .collect::<Vec<_>>();

    let mut household_members = BTreeMap::<String, Vec<_>>::new();
    for cat in &colony.cats {
        if let Some(household) = &cat.family.household_id {
            household_members
                .entry(household.as_str().to_owned())
                .or_default()
                .push(cat);
        }
    }
    let mut households = household_members
        .into_iter()
        .filter(|(household_id, _)| {
            view.village_filters
                .household_id
                .as_ref()
                .is_none_or(|selected| selected == household_id)
        })
        .map(|(household_id, mut members)| {
            members.sort_by(|left, right| left.cat_id.cmp(&right.cat_id));
            let member_ids = members
                .iter()
                .map(|cat| cat.cat_id.as_str().to_owned())
                .collect::<BTreeSet<_>>();
            let mut parent_child_edges = members
                .iter()
                .flat_map(|cat| {
                    cat.family.parent_ids.iter().filter_map(|parent| {
                        member_ids
                            .contains(parent.as_str())
                            .then(|| (parent.as_str().to_owned(), cat.cat_id.as_str().to_owned()))
                    })
                })
                .collect::<Vec<_>>();
            parent_child_edges.sort();
            parent_child_edges.dedup();
            HouseholdRow {
                semantic_id: stable_semantic_id("village-household", &household_id),
                household_id,
                resident_cat_ids: members
                    .iter()
                    .map(|cat| cat.cat_id.as_str().to_owned())
                    .collect(),
                resident_names: members
                    .iter()
                    .map(|cat| cat.display_name.as_str().to_owned())
                    .collect(),
                residence_ids: distinct_ids(
                    members
                        .iter()
                        .filter_map(|cat| cat.family.residence_id.as_ref()),
                ),
                parent_child_edges,
                tradition_ids: distinct_ids(
                    members
                        .iter()
                        .filter_map(|cat| cat.family.tradition_id.as_ref()),
                ),
                enterprise_ids: distinct_ids(
                    members
                        .iter()
                        .filter_map(|cat| cat.family.enterprise_id.as_ref()),
                ),
            }
        })
        .collect::<Vec<_>>();
    households.sort_by(|left, right| left.household_id.cmp(&right.household_id));

    let mut traditions = BTreeMap::<String, Vec<String>>::new();
    let mut enterprises = BTreeMap::<String, Vec<String>>::new();
    for cat in &colony.cats {
        if let Some(tradition) = &cat.family.tradition_id {
            traditions
                .entry(tradition.as_str().to_owned())
                .or_default()
                .push(cat.cat_id.as_str().to_owned());
        }
        if let Some(enterprise) = &cat.family.enterprise_id {
            enterprises
                .entry(enterprise.as_str().to_owned())
                .or_default()
                .push(cat.cat_id.as_str().to_owned());
        }
    }
    let reported_residences = colony
        .residences
        .iter()
        .map(|residence| residence.residence_id.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    let mut housing = colony
        .residences
        .iter()
        .filter(|residence| {
            view.village_filters
                .residence_id
                .as_ref()
                .is_none_or(|selected| selected == residence.residence_id.as_str())
        })
        .map(|residence| ResidenceRow {
            residence_id: residence.residence_id.as_str().to_owned(),
            housing_kind_id: Some(residence.housing_kind_id.as_str().to_owned()),
            ordered_footprint: residence
                .footprint
                .ordered_tiles
                .iter()
                .map(|tile| (tile.x, tile.y))
                .collect(),
            resident_cat_ids: residence
                .resident_cat_ids
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
            reported_capacity: ReportAvailability::Reported(residence.capacity),
            housing_pressure: ReportAvailability::Reported(format!(
                "{}/10000",
                residence.housing_pressure_basis_points
            )),
        })
        .collect::<Vec<_>>();
    let mut unreported_residence_members = BTreeMap::<String, Vec<String>>::new();
    for cat in &colony.cats {
        if let Some(residence) = &cat.family.residence_id
            && !reported_residences.contains(residence.as_str())
        {
            unreported_residence_members
                .entry(residence.as_str().to_owned())
                .or_default()
                .push(cat.cat_id.as_str().to_owned());
        }
    }
    housing.extend(
        unreported_residence_members
            .into_iter()
            .filter(|(residence_id, _)| {
                view.village_filters
                    .residence_id
                    .as_ref()
                    .is_none_or(|selected| selected == residence_id)
            })
            .map(|(residence_id, resident_cat_ids)| ResidenceRow {
                residence_id,
                housing_kind_id: None,
                ordered_footprint: Vec::new(),
                resident_cat_ids,
                reported_capacity: ReportAvailability::Unavailable {
                    reason: "The cat report references this residence, but its residence detail row is absent.".to_owned(),
                },
                housing_pressure: ReportAvailability::Unavailable {
                    reason: "The cat report references this residence, but no authoritative housing-pressure value was supplied.".to_owned(),
                },
            }),
    );
    housing.sort_by(|left, right| left.residence_id.cmp(&right.residence_id));
    let traditions = traditions
        .into_iter()
        .map(|(tradition_id, cat_ids)| TraditionRow {
            tradition_id,
            cat_ids,
        })
        .collect();
    let enterprises = enterprises
        .into_iter()
        .map(|(enterprise_id, cat_ids)| EnterpriseRow {
            enterprise_id,
            cat_ids,
        })
        .collect();
    let mut partnership_members = BTreeMap::<String, Vec<String>>::new();
    for cat in &colony.cats {
        if let Some(partnership) = &cat.family.partnership_id {
            partnership_members
                .entry(partnership.as_str().to_owned())
                .or_default()
                .push(cat.cat_id.as_str().to_owned());
        }
    }
    let partnerships = partnership_members
        .into_iter()
        .map(|(partnership_id, mut cat_ids)| {
            cat_ids.sort();
            PartnershipRow {
                partnership_id,
                cat_ids,
            }
        })
        .collect::<Vec<_>>();
    let job_assignments = colony
        .job_assignments
        .iter()
        .map(|assignment| JobAssignmentRow {
            assignment_id: assignment.assignment_id.as_str().to_owned(),
            cat_id: assignment.cat_id.as_str().to_owned(),
            job_kind_id: assignment.job_kind_id.as_str().to_owned(),
            station_id: assignment
                .station_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            active: assignment.active,
            report_reason: assignment.report_reason.as_str().to_owned(),
        })
        .collect::<Vec<_>>();
    let durable_job_assignments = if job_assignments.is_empty()
        && colony.cats.iter().any(|cat| cat.job_id.is_some())
    {
        ReportAvailability::Unavailable {
            reason: "Cats carry reported job IDs, but the durable assignment-detail collection is absent.".to_owned(),
        }
    } else {
        ReportAvailability::Reported(job_assignments)
    };

    let election = ElectionProjection {
        election_id: colony
            .governance
            .election_id
            .as_ref()
            .map(|id| id.as_str().to_owned()),
        candidates: colony
            .governance
            .candidates
            .iter()
            .map(|candidate| ElectionCandidateRow {
                cat_id: candidate.cat_id.as_str().to_owned(),
                display_name: names
                    .get(candidate.cat_id.as_str())
                    .cloned()
                    .unwrap_or_else(|| candidate.cat_id.as_str().to_owned()),
                report_reason: candidate.report_reason.as_str().to_owned(),
                backing_blocks: candidate.backing_blocks,
                eligible: candidate.eligible,
            })
            .collect(),
    };
    let officers = colony
        .governance
        .officers
        .iter()
        .map(|officer| OfficerRow {
            office_id: officer.office_id.as_str().to_owned(),
            cat_id: officer.cat_id.as_ref().map(|id| id.as_str().to_owned()),
            display_name: officer
                .cat_id
                .as_ref()
                .and_then(|id| names.get(id.as_str()).cloned()),
            report_expertise_level: officer.report_expertise_level,
            appointment_candidate_ids: officer
                .appointment_candidate_ids
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
        })
        .collect::<Vec<_>>();
    let selected_household = view
        .selected_household_id
        .as_ref()
        .and_then(|selected| {
            households
                .iter()
                .find(|household| &household.household_id == selected)
        })
        .cloned();

    VillageProjection {
        state: state_for(refresh, colony.cats.is_empty()),
        demographics: DemographicsProjection {
            reported_resident_count: colony.cats.len(),
            succession_eligible_count: colony
                .cats
                .iter()
                .filter(|cat| cat.succession_eligible)
                .count(),
            assigned_office_count: colony
                .cats
                .iter()
                .filter(|cat| cat.office_id.is_some())
                .count(),
            unassigned_household_count: colony
                .cats
                .iter()
                .filter(|cat| cat.family.household_id.is_none())
                .count(),
            life_stage_counts: ReportAvailability::Reported(life_stage_counts(colony)),
        },
        employment,
        durable_job_assignments,
        households,
        housing,
        partnerships: ReportAvailability::Reported(partnerships),
        traditions,
        enterprises,
        election,
        officers,
        succession: colony.governance.succession_summary.as_ref().map_or_else(
            || ReportAvailability::Unavailable {
                reason: "No succession report is currently available.".to_owned(),
            },
            |summary| ReportAvailability::Reported(summary.as_str().to_owned()),
        ),
        selected_household,
    }
}

fn active_tasks_by_cat<'a>(
    colony: &'a CanonicalColonySnapshot,
) -> BTreeMap<String, Vec<&'a PhysicalTaskSnapshot>> {
    let mut by_cat = BTreeMap::<String, Vec<_>>::new();
    for task in &colony.tasks {
        if matches!(task.state, TaskState::Complete | TaskState::Refused) {
            continue;
        }
        for cat_id in &task.worker_cat_ids {
            by_cat
                .entry(cat_id.as_str().to_owned())
                .or_default()
                .push(task);
        }
    }
    for tasks in by_cat.values_mut() {
        tasks.sort_by(|left, right| left.task_id.cmp(&right.task_id));
    }
    by_cat
}

fn life_stage_counts(colony: &CanonicalColonySnapshot) -> Vec<NamedCount> {
    [
        (LifeStageSnapshot::Kitten, "kitten"),
        (LifeStageSnapshot::Adolescent, "adolescent"),
        (LifeStageSnapshot::Adult, "adult"),
        (LifeStageSnapshot::Elder, "elder"),
    ]
    .into_iter()
    .map(|(stage, id)| NamedCount {
        id: id.to_owned(),
        count: colony
            .cats
            .iter()
            .filter(|cat| cat.life_stage == stage)
            .count(),
    })
    .collect()
}

fn unavailable_partnerships() -> ReportAvailability<Vec<PartnershipRow>> {
    ReportAvailability::Unavailable {
        reason: "Partnerships are unavailable until a selected colony report is loaded.".to_owned(),
    }
}

fn content_names(colony: &CanonicalColonySnapshot) -> BTreeMap<String, String> {
    colony
        .content_manifest
        .iter()
        .flat_map(|manifest| &manifest.entries)
        .map(|entry| {
            (
                entry.content_id.as_str().to_owned(),
                entry.display_name.as_str().to_owned(),
            )
        })
        .collect()
}

fn distinct_ids<'a>(
    ids: impl IntoIterator<Item = &'a cat_protocol::lai64::StableId>,
) -> Vec<String> {
    ids.into_iter()
        .map(|id| id.as_str().to_owned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn quality_label(quality: QualityBandSnapshot) -> String {
    match quality {
        QualityBandSnapshot::Crude => "Crude",
        QualityBandSnapshot::Common => "Common",
        QualityBandSnapshot::Fine => "Fine",
        QualityBandSnapshot::Superior => "Superior",
        QualityBandSnapshot::Masterwork => "Masterwork",
    }
    .to_owned()
}

fn quality_band_number_label(quality: u8) -> String {
    match quality {
        0 => "Crude".to_owned(),
        1 => "Common".to_owned(),
        2 => "Fine".to_owned(),
        3 => "Superior".to_owned(),
        4 => "Masterwork".to_owned(),
        other => format!("Reported quality {other}"),
    }
}

fn bounded_copy(value: &str, maximum: usize) -> String {
    value.chars().take(maximum).collect()
}

#[must_use]
pub fn stable_semantic_id(section: &str, authoritative_id: &str) -> String {
    let mut slug = String::new();
    for byte in authoritative_id.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'-' | b'_') {
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
    format!("lai66:{section}:{slug}")
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lai66LayoutContract {
    pub mode: LayoutMode,
    pub primary_width_percent: f32,
    pub detail_width_percent: f32,
    pub content_gutter_px: u16,
    pub minimum_pane_height_px: u16,
    pub row_minimum_height_px: u16,
    pub charter_placement: CharterPlacement,
}

#[must_use]
pub fn lai66_layout_contract(
    platform: ClientPlatform,
    viewport: Viewport,
    ui_scale: super::lai54::layout::UiScale,
) -> Option<Lai66LayoutContract> {
    let shell = shell_layout(platform, viewport, ui_scale).ok()?;
    Some(Lai66LayoutContract {
        mode: shell.mode,
        primary_width_percent: if shell.mode == LayoutMode::Wide {
            39.0
        } else {
            100.0
        },
        detail_width_percent: if shell.mode == LayoutMode::Wide {
            61.0
        } else {
            100.0
        },
        content_gutter_px: shell.content_gutter_px,
        minimum_pane_height_px: if shell.mode == LayoutMode::Wide {
            280
        } else {
            220
        },
        row_minimum_height_px: 34,
        charter_placement: shell.charter_placement,
    })
}

#[derive(Component)]
pub struct Lai66ReportsRoot;
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lai66ScreenRoot(pub PrimaryScreen);
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lai66Workspace(pub PrimaryScreen);
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lai66PrimaryPane(pub PrimaryScreen);
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lai66DetailPane(pub PrimaryScreen);
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lai66FeedbackLabel(pub PrimaryScreen);

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct Lai66Control {
    pub screen: PrimaryScreen,
    pub stable_id: String,
    pub focus_order: u32,
    pub action: Lai66ControlAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lai66StoreDetailTarget {
    ExactItem(String),
    BulkLot(String),
    RareMaterial(String),
}

/// Typed Stores-row selection forwarded by the LAI.50 bridge. It carries only
/// an identity that already crossed the canonical report boundary.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct Lai66StoreDetailSelection(pub Lai66StoreDetailTarget);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lai66ControlAction {
    Refresh,
    SetLogDomain(Option<String>),
    ClearLogFilters,
    LogPreviousPage,
    LogNextPage,
    SelectLogGroup(String),
    SelectZone(String),
    SelectHousehold(String),
    OpenStoreDetail(Lai66StoreDetailTarget),
    ClearSelection,
}

#[derive(Default)]
pub struct Lai66ReportsPlugin;

impl Plugin for Lai66ReportsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Lai66SnapshotFeed>()
            .init_resource::<Lai66ViewState>()
            .init_resource::<Lai66ProjectionResource>()
            .init_resource::<Lai66RenderState>()
            .add_message::<MouseWheel>()
            .add_message::<AccessibilityActionRequest>()
            .add_message::<Lai66StoreDetailSelection>()
            .add_systems(
                Update,
                (
                    attach_lai66_surfaces,
                    sync_lai66_projection,
                    sync_lai66_visibility,
                    render_lai66_projection,
                    reconcile_lai66_focus,
                    handle_lai66_pointer_controls,
                    handle_lai66_keyboard,
                    handle_lai66_accessibility_actions,
                    sync_lai66_control_focus_style,
                    sync_lai66_layout,
                    handle_lai66_scroll,
                )
                    .chain(),
            );
    }
}

fn attach_lai66_surfaces(
    mut commands: Commands<'_, '_>,
    shell: Query<'_, '_, Entity, With<Lai54ShellRoot>>,
    existing: Query<'_, '_, Entity, With<Lai66ReportsRoot>>,
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
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            GlobalZIndex(1_305),
            BackgroundColor(PARCHMENT),
            BorderColor::all(WOOD),
            Lai66ReportsRoot,
            crate::WorldInputBlocker,
            Name::new("LAI.66 report-safe primary screens"),
        ))
        .id();
    commands.entity(shell).add_child(root);
    for screen in [
        PrimaryScreen::Log,
        PrimaryScreen::Stores,
        PrimaryScreen::Village,
    ] {
        spawn_lai66_screen(&mut commands, root, screen);
    }
}

fn spawn_lai66_screen(commands: &mut Commands<'_, '_>, parent: Entity, screen: PrimaryScreen) {
    let label = screen_label(screen);
    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(20.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                display: Display::None,
                ..default()
            },
            Lai66ScreenRoot(screen),
            Name::new(format!("LAI.66 {label} screen")),
            semantic_node(
                Role::Pane,
                format!("lai66:{}:panel", label.to_ascii_lowercase()),
                format!("{label} report"),
                true,
            ),
        ))
        .id();
    commands.entity(parent).add_child(root);
    commands.entity(root).with_children(|screen_root| {
        screen_root.spawn(text_bundle(label, 24.0, INK));
        screen_root.spawn((
            text_bundle("Loading report-safe colony data", 13.0, RUST),
            Lai66FeedbackLabel(screen),
            semantic_status_node(
                format!("lai66:{}:status", label.to_ascii_lowercase()),
                format!("{label} is loading"),
                false,
            ),
        ));
    });
    let workspace = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                min_height: Val::Px(220.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.0),
                row_gap: Val::Px(12.0),
                overflow: Overflow::clip(),
                ..default()
            },
            Lai66Workspace(screen),
        ))
        .id();
    commands.entity(root).add_child(workspace);
    for detail in [false, true] {
        let pane = commands
            .spawn((
                Node {
                    width: Val::Percent(if detail { 61.0 } else { 39.0 }),
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
                BackgroundColor(if detail { PARCHMENT } else { PAPER_SHADE }),
                BorderColor::all(if detail { STONE } else { WOOD }),
                Name::new(format!(
                    "LAI.66 {label} {} pane",
                    if detail { "detail" } else { "index" }
                )),
            ))
            .id();
        if detail {
            commands.entity(pane).insert(Lai66DetailPane(screen));
        } else {
            commands.entity(pane).insert(Lai66PrimaryPane(screen));
        }
        commands.entity(workspace).add_child(pane);
    }
}

fn sync_lai66_projection(
    feed: Res<'_, Lai66SnapshotFeed>,
    view: Res<'_, Lai66ViewState>,
    mut projection: ResMut<'_, Lai66ProjectionResource>,
    mut render: ResMut<'_, Lai66RenderState>,
) {
    if feed.is_changed() || view.is_changed() {
        projection.0 = project_lai66_reports(&feed, &view);
        render.dirty = true;
    }
}

fn sync_lai66_visibility(
    live: Option<Res<'_, Lai54LiveShell>>,
    mut root: Query<'_, '_, &mut Node, With<Lai66ReportsRoot>>,
    mut screens: Query<'_, '_, (&Lai66ScreenRoot, &mut Node), Without<Lai66ReportsRoot>>,
) {
    let visible = live
        .as_ref()
        .and_then(|live| live.router.visible_primary())
        .filter(|screen| {
            matches!(
                screen,
                PrimaryScreen::Log | PrimaryScreen::Stores | PrimaryScreen::Village
            )
        });
    if let Ok(mut node) = root.single_mut() {
        node.display = if visible.is_some() {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (screen, mut node) in &mut screens {
        node.display = if visible == Some(screen.0) {
            Display::Flex
        } else {
            Display::None
        };
    }
}

#[allow(clippy::too_many_arguments)]
fn render_lai66_projection(
    mut commands: Commands<'_, '_>,
    projection: Res<'_, Lai66ProjectionResource>,
    mut render: ResMut<'_, Lai66RenderState>,
    primary_panes: Query<'_, '_, (Entity, &Lai66PrimaryPane)>,
    detail_panes: Query<'_, '_, (Entity, &Lai66DetailPane)>,
    mut feedback: Query<'_, '_, (&Lai66FeedbackLabel, &mut Text, &mut AccessibilityNode)>,
) {
    if !render.dirty || primary_panes.is_empty() || detail_panes.is_empty() {
        return;
    }
    for (marker, mut text, mut accessibility) in &mut feedback {
        let state = screen_state(&projection.0, marker.0);
        let copy = state_copy(state);
        text.0.clone_from(&copy);
        *accessibility = semantic_status_node(
            format!(
                "lai66:{}:status",
                screen_label(marker.0).to_ascii_lowercase()
            ),
            copy,
            matches!(
                state,
                Lai66SurfaceState::Error { .. } | Lai66SurfaceState::UpdateRequired
            ),
        );
    }
    for (pane, screen) in &primary_panes {
        commands.entity(pane).despawn_children();
        match screen.0 {
            PrimaryScreen::Log => render_log_index(&mut commands, pane, &projection.0.log),
            PrimaryScreen::Stores => render_store_index(&mut commands, pane, &projection.0.stores),
            PrimaryScreen::Village => {
                render_village_index(&mut commands, pane, &projection.0.village)
            }
            _ => {}
        }
    }
    for (pane, screen) in &detail_panes {
        commands.entity(pane).despawn_children();
        match screen.0 {
            PrimaryScreen::Log => render_log_detail(&mut commands, pane, &projection.0.log),
            PrimaryScreen::Stores => render_store_detail(&mut commands, pane, &projection.0.stores),
            PrimaryScreen::Village => {
                render_village_detail(&mut commands, pane, &projection.0.village)
            }
            _ => {}
        }
    }
    render.dirty = false;
}

fn render_log_index(commands: &mut Commands<'_, '_>, pane: Entity, log: &LogProjection) {
    spawn_section_text(
        commands,
        pane,
        "History",
        &format!(
            "{} authoritative events · {} quiet groups · filters: {}\nHistory coverage: {}",
            log.total_reported_events,
            log.total_grouped_rows,
            if log.available_domain_ids.is_empty() {
                "none available".to_owned()
            } else {
                log.available_domain_ids.join(", ")
            },
            availability_label(&log.authoritative_history_coverage)
        ),
    );
    spawn_local_control(
        commands,
        pane,
        PrimaryScreen::Log,
        "refresh",
        1,
        "Refresh report",
        Lai66ControlAction::Refresh,
    );
    spawn_local_control(
        commands,
        pane,
        PrimaryScreen::Log,
        "filter-domain-all",
        10,
        "Filter domain: all",
        Lai66ControlAction::SetLogDomain(None),
    );
    for (index, domain) in log.available_domain_ids.iter().enumerate() {
        spawn_local_control(
            commands,
            pane,
            PrimaryScreen::Log,
            &format!("filter-domain-{domain}"),
            11 + index as u32,
            &format!("Filter domain: {domain}"),
            Lai66ControlAction::SetLogDomain(Some(domain.clone())),
        );
    }
    spawn_local_control(
        commands,
        pane,
        PrimaryScreen::Log,
        "clear-filters",
        90,
        "Clear Log filters",
        Lai66ControlAction::ClearLogFilters,
    );
    for (index, group) in log
        .visible_groups
        .iter()
        .take(MAX_RENDERED_REPORT_ROWS)
        .enumerate()
    {
        let label = format!(
            "{} · {} · {}{} · confidence {}",
            group.domain_id,
            group.event_kind_id,
            group.summary,
            if group.repeat_count > 1 {
                format!(" · repeated {} times", group.repeat_count)
            } else {
                String::new()
            },
            availability_label(&group.confidence)
        );
        spawn_local_control(
            commands,
            pane,
            PrimaryScreen::Log,
            &group.group_id,
            100 + index as u32,
            &label,
            Lai66ControlAction::SelectLogGroup(group.group_id.clone()),
        );
    }
    if log.has_previous_page {
        spawn_local_control(
            commands,
            pane,
            PrimaryScreen::Log,
            "previous-page",
            10_001,
            "Previous history page",
            Lai66ControlAction::LogPreviousPage,
        );
    }
    if log.has_next_page {
        spawn_local_control(
            commands,
            pane,
            PrimaryScreen::Log,
            "next-page",
            10_002,
            "Next history page",
            Lai66ControlAction::LogNextPage,
        );
    }
}

fn render_log_detail(commands: &mut Commands<'_, '_>, pane: Entity, log: &LogProjection) {
    let Some(group) = &log.selected_group else {
        spawn_section_text(
            commands,
            pane,
            "Event detail",
            "Choose one grouped report. Repeated events remain individually traceable here.",
        );
        return;
    };
    spawn_section_text(
        commands,
        pane,
        "Event detail",
        &format!(
            "{}\nDomain: {}\nConfidence: {}\nFirst: {}\nLast: {}\nOccurrences: {}\n{}",
            group.summary,
            format!("{} / {}", group.domain_id, group.event_kind_id),
            availability_label(&group.confidence),
            group.first_occurred_at_ms,
            group.last_occurred_at_ms,
            group.repeat_count,
            format!(
                "Ledger entries:\n{}\nReported source IDs:\n{}",
                group.ledger_event_ids.join("\n"),
                fallback_join(&group.source_event_ids)
            )
        ),
    );
}

fn render_store_index(commands: &mut Commands<'_, '_>, pane: Entity, stores: &StoresProjection) {
    spawn_section_text(
        commands,
        pane,
        "Storage ledger",
        &format!(
            "{} zones · {}/{} visible loose slots · {} containers · {} reported lots",
            stores.zone_count,
            stores.occupied_loose_slots,
            stores.visible_loose_slots,
            stores.container_count,
            stores.reported_lot_count
        ),
    );
    spawn_local_control(
        commands,
        pane,
        PrimaryScreen::Stores,
        "refresh",
        1,
        "Refresh report",
        Lai66ControlAction::Refresh,
    );
    for (index, zone) in stores
        .zones
        .iter()
        .take(MAX_RENDERED_REPORT_ROWS)
        .enumerate()
    {
        let label = format!(
            "{} · {}/{} slots · {} containers · {} lots · workshop {}",
            zone.zone_id,
            zone.occupied_slots,
            zone.visible_slot_capacity,
            zone.containers.len(),
            zone.lots.len(),
            zone.linked_workshop_id.as_deref().unwrap_or("not linked")
        );
        spawn_local_control(
            commands,
            pane,
            PrimaryScreen::Stores,
            &zone.semantic_id,
            100 + index as u32,
            &label,
            Lai66ControlAction::SelectZone(zone.zone_id.clone()),
        );
    }
    spawn_section_text(
        commands,
        pane,
        "Leader food permissions",
        &stores
            .food_permissions
            .iter()
            .map(|row| {
                format!(
                    "{} · {:?} · {:?} confidence\n{}",
                    row.display_name, row.permission, row.confidence, row.reason
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

fn render_store_detail(commands: &mut Commands<'_, '_>, pane: Entity, stores: &StoresProjection) {
    let Some(zone) = &stores.selected_zone else {
        spawn_section_text(
            commands,
            pane,
            "Zone detail",
            "Choose a storage zone to inspect its exact footprint, containers, internal lots, hauling, and blockers.",
        );
        spawn_workshop_links(commands, pane, &stores.explicit_workshop_zone_links);
        return;
    };
    spawn_section_text(
        commands,
        pane,
        &zone.zone_id,
        &format!(
            "{} tiles · footprint {}\nVisible slots: {}/{} occupied\nLinked Workshop: {}",
            zone.tile_count,
            format_tiles(&zone.ordered_footprint),
            zone.occupied_slots,
            zone.visible_slot_capacity,
            zone.linked_workshop_id.as_deref().unwrap_or("none")
        ),
    );
    spawn_section_text(
        commands,
        pane,
        "Containers",
        &zone
            .containers
            .iter()
            .map(|row| {
                format!(
                    "{} · {} · {}/10000 full · {}/{} internal lots",
                    row.container_id,
                    row.container_kind_id,
                    row.fullness_basis_points,
                    row.internal_lot_ids.len(),
                    row.capacity_slots
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
    );
    spawn_section_text(
        commands,
        pane,
        "Lots and items",
        &zone
            .lots
            .iter()
            .map(|row| {
                format!(
                    "{} · {} ×{} · {} · source {}{}",
                    row.lot_id,
                    row.display_name,
                    row.quantity,
                    row.quality_label,
                    row.provenance_id,
                    row.reservation_id
                        .as_ref()
                        .map_or_else(String::new, |id| format!(" · reserved {id}"))
                )
            })
            .chain(zone.unique_items.iter().map(|row| {
                format!(
                    "{} · {} / {} · {} · durability {}/10000",
                    row.item_id,
                    row.definition_id,
                    row.material_id,
                    row.quality_label,
                    row.durability_basis_points
                )
            }))
            .chain(zone.rare_materials.iter().map(|row| {
                format!(
                    "{} · {} · state {} · {} · {} · source {}",
                    row.material_instance_id,
                    row.material_id,
                    row.content_state_id,
                    row.quality_label,
                    if row.processed { "processed" } else { "raw" },
                    row.provenance_id,
                )
            }))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    for (index, row) in zone.lots.iter().enumerate() {
        spawn_local_control(
            commands,
            pane,
            PrimaryScreen::Stores,
            &format!("lot:{}", row.lot_id),
            500 + index as u32,
            &format!("Inspect lot {} · {}", row.lot_id, row.display_name),
            Lai66ControlAction::OpenStoreDetail(Lai66StoreDetailTarget::BulkLot(
                row.lot_id.clone(),
            )),
        );
    }
    for (index, row) in zone.unique_items.iter().enumerate() {
        spawn_local_control(
            commands,
            pane,
            PrimaryScreen::Stores,
            &format!("item:{}", row.item_id),
            700 + index as u32,
            &format!("Inspect item {} · {}", row.item_id, row.definition_id),
            Lai66ControlAction::OpenStoreDetail(Lai66StoreDetailTarget::ExactItem(
                row.item_id.clone(),
            )),
        );
    }
    for (index, row) in zone.rare_materials.iter().enumerate() {
        spawn_local_control(
            commands,
            pane,
            PrimaryScreen::Stores,
            &format!("rare-material:{}", row.material_instance_id),
            900 + index as u32,
            &format!(
                "Inspect material {} · {}",
                row.material_instance_id, row.material_id
            ),
            Lai66ControlAction::OpenStoreDetail(Lai66StoreDetailTarget::RareMaterial(
                row.material_instance_id.clone(),
            )),
        );
    }
    spawn_section_text(
        commands,
        pane,
        "Linked hauling and blockers",
        &zone
            .linked_hauling
            .iter()
            .map(|row| {
                format!(
                    "{} · {} · {:?} · route {} · workers {}",
                    row.task_id,
                    row.task_kind_id,
                    row.state,
                    format_tiles(&row.ordered_route),
                    row.worker_cat_ids.join(", ")
                )
            })
            .chain(zone.blockers.iter().map(|row| {
                format!(
                    "{} / {} · {} · {}",
                    row.task_id,
                    row.blocker_id,
                    row.reason,
                    if row.recoverable {
                        "recoverable"
                    } else {
                        "not reported recoverable"
                    }
                )
            }))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    spawn_workshop_links(commands, pane, &stores.explicit_workshop_zone_links);
}

fn render_village_index(
    commands: &mut Commands<'_, '_>,
    pane: Entity,
    village: &VillageProjection,
) {
    spawn_section_text(
        commands,
        pane,
        "Village register",
        &format!(
            "{} residents · {} active offices · {} succession eligible · {} without a reported household\nLife stages: {}",
            village.demographics.reported_resident_count,
            village.demographics.assigned_office_count,
            village.demographics.succession_eligible_count,
            village.demographics.unassigned_household_count,
            match &village.demographics.life_stage_counts {
                ReportAvailability::Reported(counts) => counts
                    .iter()
                    .map(|row| format!("{} {}", row.count, row.id))
                    .collect::<Vec<_>>()
                    .join(", "),
                ReportAvailability::Unavailable { reason } => {
                    format!("not reported — {reason}")
                }
            }
        ),
    );
    spawn_local_control(
        commands,
        pane,
        PrimaryScreen::Village,
        "refresh",
        1,
        "Refresh report",
        Lai66ControlAction::Refresh,
    );
    for (index, household) in village
        .households
        .iter()
        .take(MAX_RENDERED_REPORT_ROWS)
        .enumerate()
    {
        let label = format!(
            "{} · {} residents · {}",
            household.household_id,
            household.resident_cat_ids.len(),
            household.resident_names.join(", ")
        );
        spawn_local_control(
            commands,
            pane,
            PrimaryScreen::Village,
            &household.semantic_id,
            100 + index as u32,
            &label,
            Lai66ControlAction::SelectHousehold(household.household_id.clone()),
        );
    }
    spawn_section_text(
        commands,
        pane,
        "Active work and offices",
        &village
            .employment
            .iter()
            .map(|row| {
                format!(
                    "{} · job {} · office {} · active work {}",
                    row.display_name,
                    row.job_id.as_deref().unwrap_or("none"),
                    row.office_id.as_deref().unwrap_or("none"),
                    if row.active_task_kind_ids.is_empty() {
                        "none reported".to_owned()
                    } else {
                        row.active_task_kind_ids.join(", ")
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let jobs = match &village.durable_job_assignments {
        ReportAvailability::Reported(rows) => rows
            .iter()
            .map(|row| {
                format!(
                    "{} · {} → {} · station {} · {} · {}",
                    row.assignment_id,
                    row.cat_id,
                    row.job_kind_id,
                    row.station_id.as_deref().unwrap_or("none"),
                    if row.active { "active" } else { "inactive" },
                    row.report_reason
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        ReportAvailability::Unavailable { reason } => format!("Not reported — {reason}"),
    };
    spawn_section_text(commands, pane, "Durable job assignments", &jobs);
}

fn render_village_detail(
    commands: &mut Commands<'_, '_>,
    pane: Entity,
    village: &VillageProjection,
) {
    if let Some(household) = &village.selected_household {
        spawn_section_text(
            commands,
            pane,
            &household.household_id,
            &format!(
                "Residents: {}\nResidence: {}\nTraditions: {}\nEnterprises: {}\nReported parent/child links: {}",
                household.resident_names.join(", "),
                fallback_join(&household.residence_ids),
                fallback_join(&household.tradition_ids),
                fallback_join(&household.enterprise_ids),
                household
                    .parent_child_edges
                    .iter()
                    .map(|(parent, child)| format!("{parent} → {child}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    } else {
        spawn_section_text(
            commands,
            pane,
            "Household detail",
            "Choose a reported household. Shared housing is not treated as proof of partnership.",
        );
    }
    spawn_section_text(
        commands,
        pane,
        "Housing",
        &village
            .housing
            .iter()
            .map(|row| {
                format!(
                    "{} · kind {} · residents {} · footprint {} · capacity {} · pressure {}",
                    row.residence_id,
                    row.housing_kind_id.as_deref().unwrap_or("not reported"),
                    row.resident_cat_ids.len(),
                    format_tiles(&row.ordered_footprint),
                    availability_label(&row.reported_capacity),
                    availability_label(&row.housing_pressure)
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let partnerships = match &village.partnerships {
        ReportAvailability::Reported(rows) => rows
            .iter()
            .map(|row| format!("{} · {}", row.partnership_id, row.cat_ids.join(" + ")))
            .collect::<Vec<_>>()
            .join("\n"),
        ReportAvailability::Unavailable { reason } => format!("Not reported — {reason}"),
    };
    spawn_section_text(commands, pane, "Partnerships", &partnerships);
    spawn_section_text(
        commands,
        pane,
        "Traditions and enterprises",
        &village
            .traditions
            .iter()
            .map(|row| {
                format!(
                    "Tradition {} · {} cats",
                    row.tradition_id,
                    row.cat_ids.len()
                )
            })
            .chain(village.enterprises.iter().map(|row| {
                format!(
                    "Enterprise {} · {} participating cats",
                    row.enterprise_id,
                    row.cat_ids.len()
                )
            }))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    spawn_section_text(
        commands,
        pane,
        "Election, officers, and succession",
        &village
            .election
            .candidates
            .iter()
            .map(|candidate| {
                format!(
                    "{} · {} · {} backing block(s) · {}",
                    candidate.display_name,
                    candidate.report_reason,
                    candidate.backing_blocks,
                    if candidate.eligible {
                        "eligible"
                    } else {
                        "ineligible"
                    }
                )
            })
            .chain(village.officers.iter().map(|officer| {
                format!(
                    "{} · {} · report level {}",
                    officer.office_id,
                    officer.display_name.as_deref().unwrap_or("vacant"),
                    officer.report_expertise_level
                )
            }))
            .chain(std::iter::once(format!(
                "Succession: {}",
                availability_label(&village.succession)
            )))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

fn spawn_section_text(commands: &mut Commands<'_, '_>, parent: Entity, heading: &str, body: &str) {
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
            semantic_node(
                Role::Pane,
                stable_semantic_id("section", heading),
                format!("{heading}. {}", bounded_copy(body, 512)),
                true,
            ),
            Name::new(format!("LAI.66 {heading} section")),
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

fn spawn_local_control(
    commands: &mut Commands<'_, '_>,
    parent: Entity,
    screen: PrimaryScreen,
    subject: &str,
    focus_order: u32,
    label: &str,
    action: Lai66ControlAction,
) {
    let semantic_id =
        stable_semantic_id(screen_label(screen).to_ascii_lowercase().as_str(), subject);
    let control = commands
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(34.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(STONE),
            semantic_node(Role::Button, semantic_id.clone(), label, true),
            Lai66Control {
                screen,
                stable_id: semantic_id,
                focus_order,
                action,
            },
            Name::new(format!("LAI.66 {label}")),
        ))
        .id();
    commands.entity(control).with_children(|button| {
        button.spawn(text_bundle(label, 12.0, INK));
    });
    commands.entity(parent).add_child(control);
}

fn spawn_workshop_links(
    commands: &mut Commands<'_, '_>,
    parent: Entity,
    links: &ReportAvailability<Vec<WorkshopZoneLinkRow>>,
) {
    let copy = match links {
        ReportAvailability::Reported(rows) if rows.is_empty() => {
            "No storage zone is currently linked to a Workshop.".to_owned()
        }
        ReportAvailability::Reported(rows) => rows
            .iter()
            .map(|row| format!("{} → {}", row.workshop_id, row.zone_id))
            .collect::<Vec<_>>()
            .join("\n"),
        ReportAvailability::Unavailable { reason } => format!("Not reported — {reason}"),
    };
    spawn_section_text(commands, parent, "Workshop input links", &copy);
}

fn availability_label<T: std::fmt::Debug>(availability: &ReportAvailability<T>) -> String {
    match availability {
        ReportAvailability::Reported(value) => format!("{value:?}"),
        ReportAvailability::Unavailable { reason } => format!("Not reported — {reason}"),
    }
}

fn handle_lai66_pointer_controls(
    mut interactions: Query<'_, '_, (&Interaction, &Lai66Control), Changed<Interaction>>,
    mut view: ResMut<'_, Lai66ViewState>,
    mut detail_selections: MessageWriter<'_, Lai66StoreDetailSelection>,
) {
    for (interaction, control) in &mut interactions {
        if *interaction == Interaction::Pressed {
            view.focused_control_id = Some(control.stable_id.clone());
            apply_lai66_action(&control.action, &mut view, &mut detail_selections);
        }
    }
}

fn reconcile_lai66_focus(
    live: Option<Res<'_, Lai54LiveShell>>,
    controls: Query<'_, '_, &Lai66Control>,
    mut view: ResMut<'_, Lai66ViewState>,
) {
    let Some(screen) = live.and_then(|live| live.router.visible_primary()) else {
        return;
    };
    if !matches!(
        screen,
        PrimaryScreen::Log | PrimaryScreen::Stores | PrimaryScreen::Village
    ) {
        return;
    }
    let visible = controls
        .iter()
        .filter(|control| control.screen == screen)
        .map(|control| control.stable_id.as_str())
        .collect::<Vec<_>>();
    if visible.is_empty() {
        return;
    }
    let Some(focused) = view.focused_control_id.as_deref() else {
        return;
    };
    if visible.contains(&focused) {
        return;
    }
    let refresh = stable_semantic_id(
        screen_label(screen).to_ascii_lowercase().as_str(),
        "refresh",
    );
    view.focused_control_id = Some(refresh);
}

fn handle_lai66_keyboard(
    keys: Option<Res<'_, ButtonInput<KeyCode>>>,
    live: Option<Res<'_, Lai54LiveShell>>,
    controls: Query<'_, '_, &Lai66Control>,
    mut view: ResMut<'_, Lai66ViewState>,
    mut detail_selections: MessageWriter<'_, Lai66StoreDetailSelection>,
) {
    let Some(keys) = keys else {
        return;
    };
    let Some(screen) = live.and_then(|live| live.router.visible_primary()) else {
        return;
    };
    if !matches!(
        screen,
        PrimaryScreen::Log | PrimaryScreen::Stores | PrimaryScreen::Village
    ) {
        return;
    }
    let mut available = controls
        .iter()
        .filter(|control| control.screen == screen)
        .cloned()
        .collect::<Vec<_>>();
    available.sort_by(|left, right| {
        left.focus_order
            .cmp(&right.focus_order)
            .then_with(|| left.stable_id.cmp(&right.stable_id))
    });
    if available.is_empty() {
        return;
    }
    let reverse = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let navigates = keys.just_pressed(KeyCode::Tab)
        || keys.just_pressed(KeyCode::ArrowDown)
        || keys.just_pressed(KeyCode::ArrowRight)
        || keys.just_pressed(KeyCode::ArrowUp)
        || keys.just_pressed(KeyCode::ArrowLeft);
    if navigates {
        let backward =
            reverse || keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::ArrowLeft);
        let current = view.focused_control_id.as_ref().and_then(|id| {
            available
                .iter()
                .position(|control| &control.stable_id == id)
        });
        let next = match (current, backward) {
            (None, false) => 0,
            (None, true) => available.len() - 1,
            (Some(0), true) => available.len() - 1,
            (Some(index), true) => index - 1,
            (Some(index), false) => (index + 1) % available.len(),
        };
        view.focused_control_id = Some(available[next].stable_id.clone());
    }
    if (keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space))
        && let Some(control) = view
            .focused_control_id
            .as_ref()
            .and_then(|id| available.iter().find(|control| &control.stable_id == id))
    {
        let action = control.action.clone();
        apply_lai66_action(&action, &mut view, &mut detail_selections);
    }
}

fn handle_lai66_accessibility_actions(
    mut requests: MessageReader<'_, '_, AccessibilityActionRequest>,
    controls: Query<'_, '_, &Lai66Control>,
    mut view: ResMut<'_, Lai66ViewState>,
    mut detail_selections: MessageWriter<'_, Lai66StoreDetailSelection>,
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
            apply_lai66_action(&control.action, &mut view, &mut detail_selections);
        }
    }
}

fn sync_lai66_control_focus_style(
    view: Res<'_, Lai66ViewState>,
    mut controls: Query<'_, '_, (&Lai66Control, &mut BackgroundColor, &mut BorderColor)>,
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

fn apply_lai66_action(
    action: &Lai66ControlAction,
    view: &mut Lai66ViewState,
    detail_selections: &mut MessageWriter<'_, Lai66StoreDetailSelection>,
) {
    match action {
        Lai66ControlAction::Refresh => {
            view.refresh_requests = view.refresh_requests.saturating_add(1);
        }
        Lai66ControlAction::SetLogDomain(domain_id) => {
            view.log_filters.domain_id.clone_from(domain_id);
            view.log_filters.page_offset = 0;
            view.selected_log_group_id = None;
        }
        Lai66ControlAction::ClearLogFilters => {
            view.log_filters = LogFilters::default();
            view.selected_log_group_id = None;
        }
        Lai66ControlAction::LogPreviousPage => {
            view.log_filters.page_offset = view
                .log_filters
                .page_offset
                .saturating_sub(view.log_filters.page_size.max(1));
        }
        Lai66ControlAction::LogNextPage => {
            view.log_filters.page_offset = view
                .log_filters
                .page_offset
                .saturating_add(view.log_filters.page_size.max(1));
        }
        Lai66ControlAction::SelectLogGroup(group_id) => {
            view.selected_log_group_id = Some(group_id.clone());
        }
        Lai66ControlAction::SelectZone(zone_id) => {
            view.selected_zone_id = Some(zone_id.clone());
        }
        Lai66ControlAction::SelectHousehold(household_id) => {
            view.selected_household_id = Some(household_id.clone());
        }
        Lai66ControlAction::OpenStoreDetail(target) => {
            detail_selections.write(Lai66StoreDetailSelection(target.clone()));
        }
        Lai66ControlAction::ClearSelection => {
            view.selected_log_group_id = None;
            view.selected_zone_id = None;
            view.selected_household_id = None;
        }
    }
}

fn sync_lai66_layout(
    windows: Query<'_, '_, &Window, With<PrimaryWindow>>,
    mut report_root: Query<'_, '_, &mut Node, With<Lai66ReportsRoot>>,
    mut workspaces: Query<
        '_,
        '_,
        &mut Node,
        (
            With<Lai66Workspace>,
            Without<Lai66ReportsRoot>,
            Without<Lai66PrimaryPane>,
            Without<Lai66DetailPane>,
        ),
    >,
    mut primary: Query<
        '_,
        '_,
        &mut Node,
        (
            With<Lai66PrimaryPane>,
            Without<Lai66DetailPane>,
            Without<Lai66ReportsRoot>,
            Without<Lai66Workspace>,
        ),
    >,
    mut detail: Query<
        '_,
        '_,
        &mut Node,
        (
            With<Lai66DetailPane>,
            Without<Lai66PrimaryPane>,
            Without<Lai66ReportsRoot>,
            Without<Lai66Workspace>,
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
    let Some(layout) = lai66_layout_contract(
        platform,
        Viewport::new(
            window.width().round() as u16,
            window.height().round() as u16,
        ),
        ui_scale_for_window_scale(window.scale_factor()),
    ) else {
        return;
    };
    if let Ok(mut root) = report_root.single_mut() {
        root.left = Val::Px(f32::from(layout.content_gutter_px));
        root.right = Val::Px(f32::from(layout.content_gutter_px));
        root.bottom = Val::Px(f32::from(layout.content_gutter_px));
    }
    for mut workspace in &mut workspaces {
        workspace.flex_direction = if layout.mode == LayoutMode::Wide {
            FlexDirection::Row
        } else {
            FlexDirection::Column
        };
    }
    for mut pane in &mut primary {
        pane.width = Val::Percent(layout.primary_width_percent);
        pane.height = if layout.mode == LayoutMode::Wide {
            Val::Percent(100.0)
        } else {
            Val::Percent(45.0)
        };
        pane.min_height = Val::Px(f32::from(layout.minimum_pane_height_px));
    }
    for mut pane in &mut detail {
        pane.width = Val::Percent(layout.detail_width_percent);
        pane.height = if layout.mode == LayoutMode::Wide {
            Val::Percent(100.0)
        } else {
            Val::Percent(55.0)
        };
        pane.min_height = Val::Px(f32::from(layout.minimum_pane_height_px));
    }
}

fn handle_lai66_scroll(
    mut wheel: MessageReader<'_, '_, MouseWheel>,
    mut panes: Query<
        '_,
        '_,
        (&Interaction, &Node, &ComputedNode, &mut ScrollPosition),
        Or<(With<Lai66PrimaryPane>, With<Lai66DetailPane>)>,
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

fn screen_state(projection: &Lai66ReportsProjection, screen: PrimaryScreen) -> &Lai66SurfaceState {
    match screen {
        PrimaryScreen::Log => &projection.log.state,
        PrimaryScreen::Stores => &projection.stores.state,
        PrimaryScreen::Village => &projection.village.state,
        _ => &projection.log.state,
    }
}

fn state_copy(state: &Lai66SurfaceState) -> String {
    match state {
        Lai66SurfaceState::Loading => "Loading the report-safe colony snapshot.".to_owned(),
        Lai66SurfaceState::Ready => "Current report loaded.".to_owned(),
        Lai66SurfaceState::Empty => {
            "No entries match this report and its current filters.".to_owned()
        }
        Lai66SurfaceState::Stale { stale_since_ms } => format!(
            "Report is stale since {stale_since_ms}; shown values remain the last received report."
        ),
        Lai66SurfaceState::UpdateRequired => {
            "Client update required before this report can refresh.".to_owned()
        }
        Lai66SurfaceState::Error { message } => format!("Report unavailable: {message}"),
    }
}

fn screen_label(screen: PrimaryScreen) -> &'static str {
    match screen {
        PrimaryScreen::Log => "Log",
        PrimaryScreen::Stores => "Stores",
        PrimaryScreen::Village => "Village",
        PrimaryScreen::Research => "Research",
        PrimaryScreen::Council => "Council",
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

fn format_tiles(tiles: &[(i32, i32)]) -> String {
    if tiles.is_empty() {
        return "none reported".to_owned();
    }
    tiles
        .iter()
        .take(16)
        .map(|(x, y)| format!("({x},{y})"))
        .chain((tiles.len() > 16).then(|| format!("… +{} tiles", tiles.len() - 16)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn fallback_join(values: &[String]) -> String {
    if values.is_empty() {
        "none reported".to_owned()
    } else {
        values.join(", ")
    }
}

/// Explicit art-direction receipt consumed by design and structural tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lai66VisualDirection {
    pub product_normal: bool,
    pub parchment_content: bool,
    pub wood_rules: bool,
    pub dark_forest_worktable: bool,
    pub uses_glass: bool,
    pub uses_glow: bool,
    pub uses_kpi_grid: bool,
    pub uses_excessive_pills: bool,
}

pub const LAI66_VISUAL_DIRECTION: Lai66VisualDirection = Lai66VisualDirection {
    product_normal: true,
    parchment_content: true,
    wood_rules: true,
    dark_forest_worktable: true,
    uses_glass: false,
    uses_glow: false,
    uses_kpi_grid: false,
    uses_excessive_pills: false,
};

/// Prevent unused palette drift while retaining an explicit dark-forest
/// worktable token for root integration.
pub const LAI66_DARK_FOREST_WORKTABLE: Color = DARK_FOREST;
pub const LAI66_SELECTION_COLOR: Color = MOSS;
pub const LAI66_ERROR_COLOR: Color = DANGER;
