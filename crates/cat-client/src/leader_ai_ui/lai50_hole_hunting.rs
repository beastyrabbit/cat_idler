//! LAI.50 report-safe Hole and Hunting Lair inspectors.
//!
//! This leaf consumes only the canonical protocol-v3 snapshot. It deliberately
//! leaves exact Lair level, success odds, party health/equipment, Hole feed
//! candidates, and other facts unavailable when the snapshot does not report
//! them. In particular, the client never derives ecology, regeneration,
//! respawn deadlines, combat odds, or hidden planner candidates.

use accesskit::{Action, Role};
use bevy::a11y::{AccessibilityNode, ActionRequest as AccessibilityActionRequest};
use bevy::prelude::*;
use cat_protocol::lai64::{
    CanonicalColonySnapshot, CanonicalGodAction, CanonicalSnapshotEnvelope,
    ContentManifestEntrySnapshot, NudgeDomain, PhysicalTaskSnapshot, QualityBandSnapshot,
    ReportConfidence, TaskState,
};

use super::{
    art_assets::{Lai68ArtCategory, resolve_lai68_art_key},
    semantic_node, semantic_status_node,
};

const INK: Color = Color::srgb(0.153, 0.106, 0.086);
const PARCHMENT: Color = Color::srgb(0.937, 0.886, 0.741);
const PAPER_SHADE: Color = Color::srgb(0.866, 0.792, 0.635);
const DARK_FOREST: Color = Color::srgb(0.090, 0.235, 0.180);
const WOOD: Color = Color::srgb(0.427, 0.282, 0.169);
const STONE: Color = Color::srgb(0.48, 0.46, 0.39);
const MOSS: Color = Color::srgb(0.310, 0.439, 0.251);
const RUST: Color = Color::srgb(0.643, 0.286, 0.176);

pub const LAI50_NUDGE_BASIS_POINTS: i16 = 1_500;
pub const MAX_LAI50_TASKS: usize = 32;
pub const MAX_LAI50_CREATURES: usize = 20;

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
    pub const fn blocks_remote_actions(&self) -> bool {
        !matches!(self, Self::Ready)
    }

    #[must_use]
    pub const fn keeps_last_report_visible(&self) -> bool {
        matches!(self, Self::Ready | Self::Stale { .. })
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
            reason: "This fact was not included in the latest report.".to_owned(),
        }
    }
}

impl<T> Lai50Availability<T> {
    #[must_use]
    pub const fn is_reported(&self) -> bool {
        matches!(self, Self::Reported(_))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lai50InspectorKind {
    #[default]
    Hole,
    HuntingLair,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai50ViewState {
    pub inspector: Lai50InspectorKind,
    pub selected_lair_id: Option<String>,
    pub selected_task_id: Option<String>,
    pub focused_control_id: Option<String>,
    pub refresh_requests: u64,
    pub local_feedback: Option<String>,
}

/// The shell or a world-selection bridge owns when this inspector is open.
/// Default-hidden prevents it from obscuring primary screens at startup.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Lai50PanelVisibility {
    pub visible: bool,
}

/// The transport owner wraps this allowed action in the authenticated,
/// versioned action envelope. This inspector never sends network traffic.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai50ActionIntent {
    pub sequence: u64,
    pub pending: Option<CanonicalGodAction>,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai50ProjectionResource(pub Lai50Projection);

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
struct Lai50RenderState {
    dirty: bool,
}

impl Default for Lai50RenderState {
    fn default() -> Self {
        Self { dirty: true }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai50Projection {
    pub selected_colony_id: Option<String>,
    pub now_ms: Option<i64>,
    pub state_version: Option<u64>,
    pub state: Lai50SurfaceState,
    pub hole: HoleInspectorProjection,
    pub lairs: Vec<LairIndexRow>,
    pub selected_lair: Option<HuntingLairInspectorProjection>,
    pub reads_authoritative_world_truth: bool,
    pub infers_hidden_exact_level: bool,
    pub infers_hidden_regeneration_or_respawn: bool,
    pub recomputes_combat_odds: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HoleInspectorProjection {
    pub hole_id: Lai50Availability<String>,
    pub axes: Lai50Availability<HoleAxesRow>,
    pub void_insight_milli: Lai50Availability<u64>,
    pub report_provenance: Lai50Availability<HoleReportProvenance>,
    pub believed_next_feed: Lai50Availability<HoleBelievedFeedRow>,
    pub believed_feed_rationale: Lai50Availability<String>,
    pub landmark_footprint: Vec<(i32, i32)>,
    pub work_footprint: Vec<(i32, i32)>,
    pub physical_tasks: Vec<PhysicalTaskRow>,
    pub approved_actions: Vec<Lai50ApprovedAction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoleAxesRow {
    pub width: u8,
    pub depth: u8,
    pub darkness: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoleReportProvenance {
    pub officer_report_level: u8,
    pub regeneration_observed_at_ms: Option<i64>,
    pub regeneration_confidence: Option<ReportConfidence>,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoleBelievedFeedRow {
    pub content_id: String,
    pub quantity: u64,
    pub quality_band: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalTaskRow {
    pub task_id: String,
    pub task_kind_id: String,
    pub objective: String,
    pub state: TaskState,
    pub worker_cat_ids: Vec<String>,
    pub footprint: Vec<(i32, i32)>,
    pub work_sites: Vec<TaskSiteRow>,
    pub delivery_site: Option<TaskSiteRow>,
    pub route: Vec<(i32, i32)>,
    pub cargo: Vec<PhysicalCargoRow>,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskSiteRow {
    pub site_id: String,
    pub site_kind_id: String,
    pub slot_id: Option<String>,
    pub footprint: Vec<(i32, i32)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalCargoRow {
    pub cargo_id: String,
    pub content_id: String,
    pub quantity: u64,
    pub quality_band: u8,
    pub provenance_id: String,
    pub reservation_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lai50ApprovedAction {
    RefreshReport,
    FocusPhysicalTask { task_id: String },
    FocusHuntingLair { site_id: String },
    NudgeHolePriority,
    NudgeHuntingPriority,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LairIndexRow {
    pub site_id: String,
    pub public_band: String,
    pub art_key: String,
    pub tile: (i32, i32),
    pub semantic_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HuntingLairInspectorProjection {
    pub site_id: String,
    pub public_band: String,
    pub public_art_key: String,
    pub tile: (i32, i32),
    pub report_confidence: Lai50Availability<ReportConfidence>,
    pub exact_level: Lai50Availability<u8>,
    pub predicted_success_basis_points: Lai50Availability<(u16, u16)>,
    pub creatures: Vec<CreaturePortraitRow>,
    pub party: HuntingPartyProjection,
    pub loot: HuntingLootProjection,
    pub respawn: Lai50Availability<String>,
    pub physical_tasks: Vec<PhysicalTaskRow>,
    pub approved_actions: Vec<Lai50ApprovedAction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreaturePortraitRow {
    pub creature_id: String,
    pub display_name: String,
    pub health_basis_points: u16,
    pub art_key: String,
    pub asset_path: String,
    pub accessibility_label: String,
    pub report_confidence: ReportConfidence,
    pub semantic_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HuntingPartyProjection {
    pub hunter_cat_ids: Vec<String>,
    pub health: Lai50Availability<Vec<(String, u16)>>,
    pub equipment: Lai50Availability<Vec<HunterEquipmentRow>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HunterEquipmentRow {
    pub cat_id: String,
    pub item_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HuntingLootProjection {
    pub cargo: Vec<PhysicalCargoRow>,
    pub cache_lot_ids: Vec<String>,
    pub cache_item_ids: Vec<String>,
    pub quality_lots: Vec<QualityLootRow>,
    pub exact_items: Vec<ExactLootRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualityLootRow {
    pub lot_id: String,
    pub content_id: String,
    pub quantity: u64,
    pub quality: QualityBandSnapshot,
    pub provenance_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactLootRow {
    pub item_id: String,
    pub definition_id: String,
    pub material_id: String,
    pub quality: QualityBandSnapshot,
    pub durability_basis_points: u16,
    pub provenance_id: String,
}

#[must_use]
pub fn project_lai50_hole_hunting(
    feed: &Lai50SnapshotFeed,
    view: &Lai50ViewState,
) -> Lai50Projection {
    let state = surface_state(feed);
    let Some(envelope) = feed.envelope.as_ref() else {
        return Lai50Projection { state, ..default() };
    };
    let Some(colony) = envelope
        .colonies
        .iter()
        .find(|colony| colony.colony_id == envelope.selected_colony_id)
    else {
        return Lai50Projection {
            selected_colony_id: Some(envelope.selected_colony_id.as_str().to_owned()),
            now_ms: Some(envelope.now_ms),
            state: Lai50SurfaceState::Error {
                message: "The selected colony is absent from this report.".to_owned(),
            },
            ..default()
        };
    };

    let lairs = project_lair_index(colony);
    let selected_id = view
        .selected_lair_id
        .clone()
        .or_else(|| lairs.first().map(|row| row.site_id.clone()));
    Lai50Projection {
        selected_colony_id: Some(colony.colony_id.as_str().to_owned()),
        now_ms: Some(envelope.now_ms),
        state_version: Some(colony.state_version),
        state,
        hole: project_hole(colony),
        lairs,
        selected_lair: selected_id.and_then(|site_id| project_lair(colony, &site_id)),
        reads_authoritative_world_truth: false,
        infers_hidden_exact_level: false,
        infers_hidden_regeneration_or_respawn: false,
        recomputes_combat_odds: false,
    }
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

fn project_hole(colony: &CanonicalColonySnapshot) -> HoleInspectorProjection {
    let hole = &colony.hole;
    let mut physical_tasks = colony
        .tasks
        .iter()
        .filter(|task| {
            task.site_id == hole.hole_id
                || task
                    .work_sites
                    .iter()
                    .any(|site| site.site_id == hole.hole_id)
                || task
                    .delivery_site
                    .as_ref()
                    .is_some_and(|site| site.site_id == hole.hole_id)
        })
        .map(physical_task_row)
        .collect::<Vec<_>>();
    physical_tasks.sort_by(|left, right| left.task_id.cmp(&right.task_id));
    physical_tasks.truncate(MAX_LAI50_TASKS);

    let rationale = colony
        .plans
        .iter()
        .filter(|plan| matches!(plan.topic_id.as_str(), "hole" | "black_hole" | "hole_feed"))
        .min_by(|left, right| left.plan_id.cmp(&right.plan_id))
        .map_or_else(
            || Lai50Availability::Unavailable {
                reason:
                    "No Hole-plan rationale is present in the canonical report; none is invented."
                        .to_owned(),
            },
            |plan| Lai50Availability::Reported(plan.rationale.as_str().to_owned()),
        );
    let provenance = hole.officer_reported_regeneration.as_ref().map_or(
        HoleReportProvenance {
            officer_report_level: hole.officer_report_level,
            regeneration_observed_at_ms: None,
            regeneration_confidence: None,
            note: "Officer report level is known, but this snapshot carries no feed-candidate report provenance.".to_owned(),
        },
        |estimate| HoleReportProvenance {
            officer_report_level: hole.officer_report_level,
            regeneration_observed_at_ms: Some(estimate.observed_at_ms),
            regeneration_confidence: Some(estimate.confidence),
            note: "The timestamp and confidence apply only to the reported regeneration estimate, not to a hidden feed choice.".to_owned(),
        },
    );
    let mut approved_actions = vec![
        Lai50ApprovedAction::RefreshReport,
        Lai50ApprovedAction::NudgeHolePriority,
    ];
    approved_actions.extend(physical_tasks.iter().map(|task| {
        Lai50ApprovedAction::FocusPhysicalTask {
            task_id: task.task_id.clone(),
        }
    }));

    HoleInspectorProjection {
        hole_id: Lai50Availability::Reported(hole.hole_id.as_str().to_owned()),
        axes: Lai50Availability::Reported(HoleAxesRow {
            width: hole.width,
            depth: hole.depth,
            darkness: hole.darkness,
        }),
        void_insight_milli: Lai50Availability::Reported(colony.research.void_balance),
        report_provenance: Lai50Availability::Reported(provenance),
        believed_next_feed: Lai50Availability::Unavailable {
            reason: "The canonical snapshot has no believed Hole feed candidate or fallback list. Physical cargo below is not relabeled as a belief.".to_owned(),
        },
        believed_feed_rationale: rationale,
        landmark_footprint: tiles(&hole.footprint.ordered_tiles),
        work_footprint: tiles(&hole.work_footprint.ordered_tiles),
        physical_tasks,
        approved_actions,
    }
}

fn project_lair_index(colony: &CanonicalColonySnapshot) -> Vec<LairIndexRow> {
    let mut rows = colony
        .hunting_sites
        .iter()
        .map(|site| LairIndexRow {
            site_id: site.site_id.as_str().to_owned(),
            public_band: public_lair_band(site.level_band),
            art_key: site.art_key.as_str().to_owned(),
            tile: (site.tile.x, site.tile.y),
            semantic_id: stable_semantic_id("lair", site.site_id.as_str()),
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.site_id.cmp(&right.site_id));
    rows
}

fn project_lair(
    colony: &CanonicalColonySnapshot,
    selected_site_id: &str,
) -> Option<HuntingLairInspectorProjection> {
    let site = colony
        .hunting_sites
        .iter()
        .find(|site| site.site_id.as_str() == selected_site_id)?;
    let manifest_entries = colony
        .content_manifest
        .as_ref()
        .map(|manifest| manifest.entries.as_slice())
        .unwrap_or_default();
    let mut creatures = site
        .creatures
        .iter()
        .filter_map(|creature| {
            creature_portrait_row(
                creature.creature_id.as_str(),
                creature.health_basis_points,
                site.report_confidence,
                manifest_entries,
            )
        })
        .collect::<Vec<_>>();
    creatures.sort_by(|left, right| left.creature_id.cmp(&right.creature_id));
    creatures.truncate(MAX_LAI50_CREATURES);

    let mut physical_tasks = colony
        .tasks
        .iter()
        .filter(|task| {
            task.site_id == site.site_id
                || task
                    .work_sites
                    .iter()
                    .any(|work_site| work_site.site_id == site.site_id)
        })
        .map(physical_task_row)
        .collect::<Vec<_>>();
    physical_tasks.sort_by(|left, right| left.task_id.cmp(&right.task_id));
    physical_tasks.truncate(MAX_LAI50_TASKS);

    let mut hunter_cat_ids = physical_tasks
        .iter()
        .flat_map(|task| task.worker_cat_ids.iter().cloned())
        .collect::<Vec<_>>();
    hunter_cat_ids.sort();
    hunter_cat_ids.dedup();
    let cargo = physical_tasks
        .iter()
        .flat_map(|task| task.cargo.iter().cloned())
        .collect::<Vec<_>>();
    let cache_lot_ids = site
        .cache_lot_ids
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect::<Vec<_>>();
    let cache_item_ids = site
        .cache_item_ids
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect::<Vec<_>>();
    let mut quality_lots = colony
        .quality_lots
        .iter()
        .filter(|lot| cache_lot_ids.iter().any(|id| id == lot.lot_id.as_str()))
        .map(|lot| QualityLootRow {
            lot_id: lot.lot_id.as_str().to_owned(),
            content_id: lot.content_id.as_str().to_owned(),
            quantity: lot.quantity,
            quality: lot.quality,
            provenance_id: lot.provenance_id.as_str().to_owned(),
        })
        .collect::<Vec<_>>();
    quality_lots.sort_by(|left, right| left.lot_id.cmp(&right.lot_id));
    let mut exact_items = colony
        .exact_items
        .iter()
        .filter(|item| cache_item_ids.iter().any(|id| id == item.item_id.as_str()))
        .map(|item| ExactLootRow {
            item_id: item.item_id.as_str().to_owned(),
            definition_id: item.definition_id.as_str().to_owned(),
            material_id: item.material_id.as_str().to_owned(),
            quality: item.quality,
            durability_basis_points: item.durability_basis_points,
            provenance_id: item.provenance_id.as_str().to_owned(),
        })
        .collect::<Vec<_>>();
    exact_items.sort_by(|left, right| left.item_id.cmp(&right.item_id));
    let mut approved_actions = vec![
        Lai50ApprovedAction::RefreshReport,
        Lai50ApprovedAction::FocusHuntingLair {
            site_id: selected_site_id.to_owned(),
        },
        Lai50ApprovedAction::NudgeHuntingPriority,
    ];
    approved_actions.extend(physical_tasks.iter().map(|task| {
        Lai50ApprovedAction::FocusPhysicalTask {
            task_id: task.task_id.clone(),
        }
    }));

    Some(HuntingLairInspectorProjection {
        site_id: site.site_id.as_str().to_owned(),
        public_band: public_lair_band(site.level_band),
        public_art_key: site.art_key.as_str().to_owned(),
        tile: (site.tile.x, site.tile.y),
        report_confidence: Lai50Availability::Reported(site.report_confidence),
        exact_level: Lai50Availability::Unavailable {
            reason: "Protocol v3 reports only the public ten-level band; no exact-level Captain report field exists.".to_owned(),
        },
        predicted_success_basis_points: Lai50Availability::Unavailable {
            reason: "No officer-reported success range crosses the canonical snapshot. The client does not recompute combat odds.".to_owned(),
        },
        creatures,
        party: HuntingPartyProjection {
            hunter_cat_ids,
            health: Lai50Availability::Unavailable {
                reason: "The canonical cat/task report contains no per-hunter health values.".to_owned(),
            },
            equipment: Lai50Availability::Unavailable {
                reason: "The canonical report does not bind equipped exact-item IDs to hunters."
                    .to_owned(),
            },
        },
        loot: HuntingLootProjection {
            cargo,
            cache_lot_ids,
            cache_item_ids,
            quality_lots,
            exact_items,
        },
        respawn: site.respawn_report.as_ref().map_or_else(
            || Lai50Availability::Unavailable {
                reason: "No respawn report is available; the client does not derive a deadline."
                    .to_owned(),
            },
            |report| Lai50Availability::Reported(report.as_str().to_owned()),
        ),
        physical_tasks,
        approved_actions,
    })
}

fn creature_portrait_row(
    creature_id: &str,
    health_basis_points: u16,
    report_confidence: ReportConfidence,
    manifest: &[ContentManifestEntrySnapshot],
) -> Option<CreaturePortraitRow> {
    let canonical_content_id = format!("creature_{creature_id}");
    let entry = manifest.iter().find(|entry| {
        entry.content_kind_id.as_str() == "creature"
            && (entry.content_id.as_str() == creature_id
                || entry.content_id.as_str() == canonical_content_id)
    })?;
    let asset = resolve_lai68_art_key(entry.art_key.as_str())?;
    if asset.category != Lai68ArtCategory::CreaturePortrait {
        return None;
    }
    Some(CreaturePortraitRow {
        creature_id: creature_id.to_owned(),
        display_name: entry.display_name.as_str().to_owned(),
        health_basis_points,
        art_key: entry.art_key.as_str().to_owned(),
        asset_path: asset.path.to_owned(),
        accessibility_label: entry.accessibility_label.as_str().to_owned(),
        report_confidence,
        semantic_id: stable_semantic_id("creature", creature_id),
    })
}

fn physical_task_row(task: &PhysicalTaskSnapshot) -> PhysicalTaskRow {
    PhysicalTaskRow {
        task_id: task.task_id.as_str().to_owned(),
        task_kind_id: task.task_kind_id.as_str().to_owned(),
        objective: task.objective.as_str().to_owned(),
        state: task.state,
        worker_cat_ids: task
            .worker_cat_ids
            .iter()
            .map(|id| id.as_str().to_owned())
            .collect(),
        footprint: tiles(&task.footprint.ordered_tiles),
        work_sites: task
            .work_sites
            .iter()
            .map(|site| TaskSiteRow {
                site_id: site.site_id.as_str().to_owned(),
                site_kind_id: site.site_kind_id.as_str().to_owned(),
                slot_id: site.slot_id.as_ref().map(|id| id.as_str().to_owned()),
                footprint: tiles(&site.footprint.ordered_tiles),
            })
            .collect(),
        delivery_site: task.delivery_site.as_ref().map(|site| TaskSiteRow {
            site_id: site.site_id.as_str().to_owned(),
            site_kind_id: site.site_kind_id.as_str().to_owned(),
            slot_id: site.slot_id.as_ref().map(|id| id.as_str().to_owned()),
            footprint: tiles(&site.footprint.ordered_tiles),
        }),
        route: tiles(&task.route.ordered_tiles),
        cargo: task
            .cargo
            .iter()
            .map(|cargo| PhysicalCargoRow {
                cargo_id: cargo.cargo_id.as_str().to_owned(),
                content_id: cargo.content_id.as_str().to_owned(),
                quantity: cargo.quantity,
                quality_band: cargo.quality_band,
                provenance_id: cargo.provenance_id.as_str().to_owned(),
                reservation_id: cargo
                    .reservation_id
                    .as_ref()
                    .map(|id| id.as_str().to_owned()),
            })
            .collect(),
        blockers: task
            .blockers
            .iter()
            .map(|blocker| blocker.reason.as_str().to_owned())
            .collect(),
    }
}

fn tiles(protocol_tiles: &[cat_protocol::lai64::Tile]) -> Vec<(i32, i32)> {
    protocol_tiles.iter().map(|tile| (tile.x, tile.y)).collect()
}

#[must_use]
pub fn public_lair_band(band: u8) -> String {
    if !(1..=10).contains(&band) {
        return "unavailable".to_owned();
    }
    let minimum = u16::from(band.saturating_sub(1)) * 10 + 1;
    let maximum = if band == 10 {
        100
    } else {
        u16::from(band) * 10
    };
    format!("{minimum}–{maximum}")
}

#[must_use]
pub fn stable_semantic_id(section: &str, authoritative_id: &str) -> String {
    let mut slug = authoritative_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, ':' | '_' | '-') {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
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
pub const fn is_lai50_allowed_remote_action(action: &CanonicalGodAction) -> bool {
    matches!(
        action,
        CanonicalGodAction::BroadDomainNudge {
            domain: NudgeDomain::Hole | NudgeDomain::Hunting,
            building_kind_id: None,
            ..
        }
    )
}

#[derive(Component)]
pub struct Lai50InspectorRoot;

#[derive(Component)]
pub struct Lai50InspectorBody;

#[derive(Component)]
pub struct Lai50StatusLabel;

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct Lai50Control {
    pub stable_id: String,
    pub focus_order: u32,
    pub action: Lai50ControlAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lai50ControlAction {
    Refresh,
    ShowHole,
    SelectLair(String),
    SelectTask(String),
    EmitCanonical(CanonicalGodAction),
}

/// Additive inspector plugin. The integration owner decides when this root is
/// visible and owns authenticated transport for [`Lai50ActionIntent`].
#[derive(Default)]
pub struct Lai50HoleHuntingPlugin;

impl Plugin for Lai50HoleHuntingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Lai50SnapshotFeed>()
            .init_resource::<Lai50ViewState>()
            .init_resource::<Lai50PanelVisibility>()
            .init_resource::<Lai50ActionIntent>()
            .init_resource::<Lai50ProjectionResource>()
            .init_resource::<Lai50RenderState>()
            .add_message::<AccessibilityActionRequest>()
            .add_systems(Startup, spawn_lai50_inspector)
            .add_systems(
                Update,
                (
                    sync_lai50_projection,
                    sync_lai50_visibility,
                    render_lai50_inspector,
                    handle_lai50_pointer_controls,
                    handle_lai50_keyboard,
                    handle_lai50_accessibility_actions,
                    sync_lai50_focus_style,
                )
                    .chain(),
            );
    }
}

fn spawn_lai50_inspector(mut commands: Commands<'_, '_>) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(24.0),
                top: Val::Px(82.0),
                width: Val::Px(560.0),
                max_width: Val::Percent(92.0),
                max_height: Val::Percent(84.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                border: UiRect::all(Val::Px(2.0)),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            GlobalZIndex(1_350),
            BackgroundColor(PARCHMENT),
            BorderColor::all(WOOD),
            Visibility::Hidden,
            Lai50InspectorRoot,
            crate::WorldInputBlocker,
            semantic_node(
                Role::Pane,
                "lai50:inspector:panel",
                "The Hole and Hunting Lair report inspector",
                true,
            ),
            Name::new("LAI.50 Hole and Hunting inspectors"),
        ))
        .id();
    commands.entity(root).with_children(|panel| {
        panel
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(7.0)),
                    border: UiRect::bottom(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(DARK_FOREST),
                BorderColor::all(WOOD),
                Name::new("LAI.50 dark-forest title band"),
            ))
            .with_child(text_bundle("The Hole and Hunting Lairs", 22.0, PARCHMENT));
        panel.spawn((
            text_bundle("Loading the latest officer report", 12.0, RUST),
            Lai50StatusLabel,
            semantic_status_node(
                "lai50:inspector:status",
                "Hole and Hunting report is loading",
                false,
            ),
        ));
        panel.spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            Lai50InspectorBody,
            Name::new("LAI.50 inspector body"),
        ));
    });
}

fn sync_lai50_visibility(
    panel: Res<'_, Lai50PanelVisibility>,
    mut root: Query<'_, '_, &mut Visibility, With<Lai50InspectorRoot>>,
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

fn sync_lai50_projection(
    feed: Res<'_, Lai50SnapshotFeed>,
    view: Res<'_, Lai50ViewState>,
    mut projection: ResMut<'_, Lai50ProjectionResource>,
    mut render: ResMut<'_, Lai50RenderState>,
) {
    if feed.is_changed() || view.is_changed() {
        projection.0 = project_lai50_hole_hunting(&feed, &view);
        render.dirty = true;
    }
}

#[allow(clippy::too_many_arguments)]
fn render_lai50_inspector(
    mut commands: Commands<'_, '_>,
    projection: Res<'_, Lai50ProjectionResource>,
    view: Res<'_, Lai50ViewState>,
    mut render: ResMut<'_, Lai50RenderState>,
    body: Query<'_, '_, Entity, With<Lai50InspectorBody>>,
    mut status: Query<'_, '_, (&mut Text, &mut AccessibilityNode), With<Lai50StatusLabel>>,
    children: Query<'_, '_, &Children>,
    asset_server: Option<Res<'_, AssetServer>>,
) {
    if !render.dirty {
        return;
    }
    render.dirty = false;
    let Ok(body) = body.single() else {
        return;
    };
    if let Ok(existing) = children.get(body) {
        for child in existing.iter() {
            commands.entity(child).despawn();
        }
    }
    if let Ok((mut status_text, mut status_accessibility)) = status.single_mut() {
        let copy = surface_status(&projection.0.state);
        status_text.0.clone_from(&copy);
        *status_accessibility = semantic_status_node(
            "lai50:inspector:status",
            copy,
            matches!(
                &projection.0.state,
                Lai50SurfaceState::Error { .. } | Lai50SurfaceState::UpdateRequired
            ),
        );
    }
    spawn_control(
        &mut commands,
        body,
        "show-hole",
        1,
        "The Hole",
        Lai50ControlAction::ShowHole,
    );
    for (index, lair) in projection.0.lairs.iter().enumerate() {
        spawn_control(
            &mut commands,
            body,
            &format!("lair-{}", lair.site_id),
            10 + index as u32,
            &format!(
                "Hunting Lair {} · band {} · ({},{})",
                lair.site_id, lair.public_band, lair.tile.0, lair.tile.1
            ),
            Lai50ControlAction::SelectLair(lair.site_id.clone()),
        );
    }
    match view.inspector {
        Lai50InspectorKind::Hole => render_hole(&mut commands, body, &projection.0),
        Lai50InspectorKind::HuntingLair => {
            render_lair(&mut commands, body, &projection.0, asset_server.as_deref())
        }
    }
}

fn render_hole(commands: &mut Commands<'_, '_>, body: Entity, projection: &Lai50Projection) {
    spawn_section(
        commands,
        body,
        "The Hole",
        "Fixed 5×5 landmark; work and delivery use the central 3×3.",
    );
    let hole = &projection.hole;
    spawn_availability(commands, body, "Axes", &hole.axes, |axes| {
        format!(
            "Width {} · Depth {} · Darkness {}",
            axes.width, axes.depth, axes.darkness
        )
    });
    spawn_availability(
        commands,
        body,
        "Void Insight",
        &hole.void_insight_milli,
        |milli| format!("{}.{:03}", milli / 1_000, milli % 1_000),
    );
    spawn_availability(
        commands,
        body,
        "Report provenance",
        &hole.report_provenance,
        |row| {
            format!(
                "Loremaster report level {} · observed {} · confidence {}\n{}",
                row.officer_report_level,
                row.regeneration_observed_at_ms
                    .map_or_else(|| "unavailable".to_owned(), |at| at.to_string()),
                row.regeneration_confidence
                    .map_or_else(|| "unavailable".to_owned(), |value| format!("{value:?}")),
                row.note
            )
        },
    );
    spawn_availability(
        commands,
        body,
        "Believed next feed",
        &hole.believed_next_feed,
        |feed| {
            format!(
                "{} ×{} · quality band {}",
                feed.content_id, feed.quantity, feed.quality_band
            )
        },
    );
    spawn_availability(
        commands,
        body,
        "Leader rationale",
        &hole.believed_feed_rationale,
        Clone::clone,
    );
    spawn_section(
        commands,
        body,
        "Footprints",
        &format!(
            "Landmark: {}\nWork/delivery objective: {}",
            format_tiles(&hole.landmark_footprint),
            format_tiles(&hole.work_footprint)
        ),
    );
    render_tasks(commands, body, &hole.physical_tasks, 200);
    spawn_control(
        commands,
        body,
        "nudge-hole",
        900,
        "Nudge Hole priority +15% (planner review still required)",
        Lai50ControlAction::EmitCanonical(CanonicalGodAction::BroadDomainNudge {
            domain: NudgeDomain::Hole,
            building_kind_id: None,
            basis_points: LAI50_NUDGE_BASIS_POINTS,
        }),
    );
    spawn_control(
        commands,
        body,
        "refresh",
        999,
        "Refresh report",
        Lai50ControlAction::Refresh,
    );
}

fn render_lair(
    commands: &mut Commands<'_, '_>,
    body: Entity,
    projection: &Lai50Projection,
    asset_server: Option<&AssetServer>,
) {
    let Some(lair) = projection.selected_lair.as_ref() else {
        spawn_section(
            commands,
            body,
            "Hunting Lair",
            "No selected Lair is present in this report.",
        );
        return;
    };
    spawn_section(
        commands,
        body,
        &format!("Hunting Lair — level band {}", lair.public_band),
        &format!(
            "{} at ({},{}) · public art {}",
            lair.site_id, lair.tile.0, lair.tile.1, lair.public_art_key
        ),
    );
    spawn_availability(
        commands,
        body,
        "Exact level",
        &lair.exact_level,
        u8::to_string,
    );
    spawn_availability(
        commands,
        body,
        "Predicted success",
        &lair.predicted_success_basis_points,
        |(low, high)| format!("{}–{}%", low / 100, high / 100),
    );
    spawn_availability(
        commands,
        body,
        "Captain report confidence",
        &lair.report_confidence,
        |confidence| format!("{confidence:?}"),
    );
    render_creatures(commands, body, &lair.creatures, asset_server);
    spawn_section(
        commands,
        body,
        "Party",
        &format!(
            "Assigned hunters: {}\nHealth: {}\nEquipment: {}",
            fallback_join(&lair.party.hunter_cat_ids),
            availability_text(&lair.party.health, |_| "reported".to_owned()),
            availability_text(&lair.party.equipment, |_| "reported".to_owned())
        ),
    );
    render_tasks(commands, body, &lair.physical_tasks, 300);
    spawn_section(
        commands,
        body,
        "Physical loot and quality",
        &format!(
            "Cargo: {}\nCache lots: {}\nCache items: {}\nReported quality lots: {}\nReported exact items: {}",
            format_cargo(&lair.loot.cargo),
            fallback_join(&lair.loot.cache_lot_ids),
            fallback_join(&lair.loot.cache_item_ids),
            lair.loot
                .quality_lots
                .iter()
                .map(|lot| format!(
                    "{} {}×{} {:?} · provenance {}",
                    lot.lot_id, lot.content_id, lot.quantity, lot.quality, lot.provenance_id
                ))
                .collect::<Vec<_>>()
                .join(", "),
            lair.loot
                .exact_items
                .iter()
                .map(|item| format!(
                    "{} {} / {} {:?} · {}% durability · provenance {}",
                    item.item_id,
                    item.definition_id,
                    item.material_id,
                    item.quality,
                    item.durability_basis_points / 100,
                    item.provenance_id
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );
    spawn_availability(commands, body, "Respawn", &lair.respawn, Clone::clone);
    spawn_control(
        commands,
        body,
        "nudge-hunting",
        900,
        "Nudge Hunting priority +15% (cannot force combat)",
        Lai50ControlAction::EmitCanonical(CanonicalGodAction::BroadDomainNudge {
            domain: NudgeDomain::Hunting,
            building_kind_id: None,
            basis_points: LAI50_NUDGE_BASIS_POINTS,
        }),
    );
    spawn_control(
        commands,
        body,
        "refresh",
        999,
        "Refresh report",
        Lai50ControlAction::Refresh,
    );
}

fn render_creatures(
    commands: &mut Commands<'_, '_>,
    body: Entity,
    creatures: &[CreaturePortraitRow],
    asset_server: Option<&AssetServer>,
) {
    let section = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(7.0),
                padding: UiRect::vertical(Val::Px(8.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(STONE),
            Name::new("LAI.50 reported creatures"),
        ))
        .id();
    commands.entity(body).add_child(section);
    commands.entity(section).with_children(|rows| {
        rows.spawn(text_bundle("Reported encounter", 16.0, INK));
    });
    if creatures.is_empty() {
        commands.entity(section).with_children(|rows| {
            rows.spawn(text_bundle(
                "No report-gated creature portrait can be resolved from the canonical content manifest. No generic portrait is substituted.",
                12.0,
                RUST,
            ));
        });
        return;
    }
    for creature in creatures {
        let row = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(72.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(10.0),
                    padding: UiRect::all(Val::Px(6.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(PAPER_SHADE),
                BorderColor::all(WOOD),
                semantic_node(
                    Role::ListItem,
                    creature.semantic_id.clone(),
                    format!(
                        "{}; reported health {} percent; {:?} confidence",
                        creature.accessibility_label,
                        creature.health_basis_points / 100,
                        creature.report_confidence
                    ),
                    true,
                ),
                Name::new(format!("LAI.50 creature {}", creature.creature_id)),
            ))
            .id();
        commands.entity(section).add_child(row);
        commands.entity(row).with_children(|row| {
            if let Some(asset_server) = asset_server {
                row.spawn((
                    ImageNode::new(asset_server.load(creature.asset_path.clone())),
                    Node {
                        width: Val::Px(64.0),
                        height: Val::Px(64.0),
                        ..default()
                    },
                    Name::new(format!("{} portrait", creature.display_name)),
                ));
            }
            row.spawn(text_bundle(
                format!(
                    "{}\nReported health: {}%\nPortrait key: {}",
                    creature.display_name,
                    creature.health_basis_points / 100,
                    creature.art_key
                ),
                12.0,
                INK,
            ));
        });
    }
}

fn render_tasks(
    commands: &mut Commands<'_, '_>,
    body: Entity,
    tasks: &[PhysicalTaskRow],
    focus_base: u32,
) {
    spawn_section(
        commands,
        body,
        "Physical task and cargo",
        if tasks.is_empty() {
            "No matching physical task is reported."
        } else {
            "Every row below is a canonical task identity, stage, route, footprint, worker, and physical cargo report."
        },
    );
    for (index, task) in tasks.iter().enumerate() {
        spawn_section(
            commands,
            body,
            &format!("{} · {:?}", task.task_id, task.state),
            &format!(
                "{}\nKind: {}\nWorkers: {}\nFootprint: {}\nRoute: {}\nCargo: {}\nBlockers: {}",
                task.objective,
                task.task_kind_id,
                fallback_join(&task.worker_cat_ids),
                format_tiles(&task.footprint),
                format_tiles(&task.route),
                format_cargo(&task.cargo),
                fallback_join(&task.blockers)
            ),
        );
        spawn_section(
            commands,
            body,
            "Pinned task sites",
            &format!(
                "Work: {}\nDelivery: {}",
                format_task_sites(&task.work_sites),
                task.delivery_site.as_ref().map_or_else(
                    || "none reported".to_owned(),
                    |site| format_task_sites(std::slice::from_ref(site))
                )
            ),
        );
        spawn_control(
            commands,
            body,
            &format!("task-{}", task.task_id),
            focus_base + index as u32,
            &format!("Focus physical task {}", task.task_id),
            Lai50ControlAction::SelectTask(task.task_id.clone()),
        );
    }
}

fn spawn_availability<T>(
    commands: &mut Commands<'_, '_>,
    body: Entity,
    heading: &str,
    value: &Lai50Availability<T>,
    reported: impl Fn(&T) -> String,
) {
    let text = availability_text(value, reported);
    spawn_section(commands, body, heading, &text);
}

fn availability_text<T>(value: &Lai50Availability<T>, reported: impl Fn(&T) -> String) -> String {
    match value {
        Lai50Availability::Reported(value) => reported(value),
        Lai50Availability::Unavailable { reason } => format!("Unavailable — {reason}"),
    }
}

fn spawn_section(commands: &mut Commands<'_, '_>, parent: Entity, heading: &str, body: &str) {
    let section = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::vertical(Val::Px(7.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(STONE),
            Name::new(format!("LAI.50 {heading}")),
        ))
        .id();
    commands.entity(parent).add_child(section);
    commands.entity(section).with_children(|section| {
        section.spawn(text_bundle(heading, 16.0, INK));
        section.spawn(text_bundle(
            if body.trim().is_empty() {
                "Nothing reported."
            } else {
                body
            },
            12.0,
            Color::srgb(0.26, 0.21, 0.17),
        ));
    });
}

fn spawn_control(
    commands: &mut Commands<'_, '_>,
    parent: Entity,
    subject: &str,
    focus_order: u32,
    label: &str,
    action: Lai50ControlAction,
) {
    let stable_id = stable_semantic_id("control", subject);
    let control = commands
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                min_height: Val::Px(38.0),
                padding: UiRect::axes(Val::Px(9.0), Val::Px(6.0)),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(PAPER_SHADE),
            BorderColor::all(WOOD),
            semantic_node(Role::Button, stable_id.clone(), label, true),
            Lai50Control {
                stable_id,
                focus_order,
                action,
            },
            Name::new(format!("LAI.50 {label}")),
        ))
        .id();
    commands.entity(control).with_children(|button| {
        button.spawn(text_bundle(label, 12.0, INK));
    });
    commands.entity(parent).add_child(control);
}

fn handle_lai50_pointer_controls(
    mut interactions: Query<'_, '_, (&Interaction, &Lai50Control), Changed<Interaction>>,
    projection: Res<'_, Lai50ProjectionResource>,
    mut view: ResMut<'_, Lai50ViewState>,
    mut intent: ResMut<'_, Lai50ActionIntent>,
) {
    for (interaction, control) in &mut interactions {
        if *interaction == Interaction::Pressed {
            view.focused_control_id = Some(control.stable_id.clone());
            apply_control(
                &control.action,
                !projection.0.state.blocks_remote_actions(),
                &mut view,
                &mut intent,
            );
        }
    }
}

fn handle_lai50_keyboard(
    keys: Option<Res<'_, ButtonInput<KeyCode>>>,
    controls: Query<'_, '_, &Lai50Control>,
    projection: Res<'_, Lai50ProjectionResource>,
    mut view: ResMut<'_, Lai50ViewState>,
    mut intent: ResMut<'_, Lai50ActionIntent>,
) {
    let Some(keys) = keys else {
        return;
    };
    let mut controls = controls.iter().cloned().collect::<Vec<_>>();
    controls.sort_by(|left, right| {
        left.focus_order
            .cmp(&right.focus_order)
            .then_with(|| left.stable_id.cmp(&right.stable_id))
    });
    if controls.is_empty() {
        return;
    }
    if keys.just_pressed(KeyCode::Tab)
        || keys.just_pressed(KeyCode::ArrowDown)
        || keys.just_pressed(KeyCode::ArrowUp)
    {
        let backwards = keys.pressed(KeyCode::ShiftLeft)
            || keys.pressed(KeyCode::ShiftRight)
            || keys.just_pressed(KeyCode::ArrowUp);
        let current = view
            .focused_control_id
            .as_ref()
            .and_then(|id| controls.iter().position(|control| &control.stable_id == id));
        let next = match (current, backwards) {
            (None, false) => 0,
            (None, true) | (Some(0), true) => controls.len() - 1,
            (Some(index), true) => index - 1,
            (Some(index), false) => (index + 1) % controls.len(),
        };
        view.focused_control_id = Some(controls[next].stable_id.clone());
    }
    if (keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space))
        && let Some(control) = view
            .focused_control_id
            .as_ref()
            .and_then(|id| controls.iter().find(|control| &control.stable_id == id))
    {
        apply_control(
            &control.action,
            !projection.0.state.blocks_remote_actions(),
            &mut view,
            &mut intent,
        );
    }
}

fn handle_lai50_accessibility_actions(
    mut requests: MessageReader<'_, '_, AccessibilityActionRequest>,
    controls: Query<'_, '_, &Lai50Control>,
    projection: Res<'_, Lai50ProjectionResource>,
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
            apply_control(
                &control.action,
                !projection.0.state.blocks_remote_actions(),
                &mut view,
                &mut intent,
            );
        }
    }
}

fn apply_control(
    action: &Lai50ControlAction,
    remote_actions_allowed: bool,
    view: &mut Lai50ViewState,
    intent: &mut Lai50ActionIntent,
) {
    match action {
        Lai50ControlAction::Refresh => {
            view.refresh_requests = view.refresh_requests.saturating_add(1);
            view.local_feedback = Some("A report refresh was requested.".to_owned());
        }
        Lai50ControlAction::ShowHole => view.inspector = Lai50InspectorKind::Hole,
        Lai50ControlAction::SelectLair(site_id) => {
            view.inspector = Lai50InspectorKind::HuntingLair;
            view.selected_lair_id = Some(site_id.clone());
        }
        Lai50ControlAction::SelectTask(task_id) => {
            view.selected_task_id = Some(task_id.clone());
            view.local_feedback = Some(format!("Focused physical task {task_id}."));
        }
        Lai50ControlAction::EmitCanonical(_) if !remote_actions_allowed => {
            view.local_feedback =
                Some("Remote action blocked until a fresh report is ready.".to_owned());
        }
        Lai50ControlAction::EmitCanonical(action) if is_lai50_allowed_remote_action(action) => {
            intent.sequence = intent.sequence.saturating_add(1);
            intent.pending = Some(action.clone());
            view.local_feedback = Some(
                "A bounded planner nudge awaits authenticated transport and authoritative review."
                    .to_owned(),
            );
        }
        Lai50ControlAction::EmitCanonical(_) => {
            view.local_feedback =
                Some("This action is not approved for the LAI.50 inspectors.".to_owned());
        }
    }
}

fn sync_lai50_focus_style(
    view: Res<'_, Lai50ViewState>,
    mut controls: Query<'_, '_, (&Lai50Control, &Interaction, &mut BackgroundColor), With<Button>>,
) {
    for (control, interaction, mut background) in &mut controls {
        background.0 = if *interaction == Interaction::Pressed {
            MOSS
        } else if view.focused_control_id.as_deref() == Some(control.stable_id.as_str()) {
            Color::srgb(0.78, 0.70, 0.52)
        } else {
            PAPER_SHADE
        };
    }
}

fn surface_status(state: &Lai50SurfaceState) -> String {
    match state {
        Lai50SurfaceState::Loading => "Loading the latest officer report.".to_owned(),
        Lai50SurfaceState::Ready => "Report ready.".to_owned(),
        Lai50SurfaceState::Empty => "No report is available.".to_owned(),
        Lai50SurfaceState::Stale { stale_since_ms } => {
            format!("Stale report retained from {stale_since_ms}; remote actions are blocked.")
        }
        Lai50SurfaceState::UpdateRequired => {
            "Client update required; no hidden fallback is used.".to_owned()
        }
        Lai50SurfaceState::Error { message } => format!("Report error: {message}"),
    }
}

fn format_tiles(tiles: &[(i32, i32)]) -> String {
    if tiles.is_empty() {
        "none reported".to_owned()
    } else {
        tiles
            .iter()
            .map(|(x, y)| format!("({x},{y})"))
            .collect::<Vec<_>>()
            .join(" → ")
    }
}

fn format_cargo(cargo: &[PhysicalCargoRow]) -> String {
    if cargo.is_empty() {
        "none reported".to_owned()
    } else {
        cargo
            .iter()
            .map(|cargo| {
                format!(
                    "{} {}×{} Q{} · provenance {}",
                    cargo.cargo_id,
                    cargo.content_id,
                    cargo.quantity,
                    cargo.quality_band,
                    cargo.provenance_id
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn format_task_sites(sites: &[TaskSiteRow]) -> String {
    if sites.is_empty() {
        "none reported".to_owned()
    } else {
        sites
            .iter()
            .map(|site| {
                format!(
                    "{} [{}] slot {} · {}",
                    site.site_id,
                    site.site_kind_id,
                    site.slot_id.as_deref().unwrap_or("unavailable"),
                    format_tiles(&site.footprint)
                )
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

fn fallback_join(values: &[String]) -> String {
    if values.is_empty() {
        "none reported".to_owned()
    } else {
        values.join(", ")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lai50VisualDirection {
    pub parchment: bool,
    pub wood_rules: bool,
    pub dark_forest_context: bool,
    pub uses_glass: bool,
    pub uses_glow: bool,
    pub uses_kpi_cards: bool,
}

pub const LAI50_VISUAL_DIRECTION: Lai50VisualDirection = Lai50VisualDirection {
    parchment: true,
    wood_rules: true,
    dark_forest_context: true,
    uses_glass: false,
    uses_glow: false,
    uses_kpi_cards: false,
};

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
        StableId::new(value).expect("valid stable id")
    }

    fn text(value: &str) -> ReportText {
        ReportText::new(value).expect("valid report text")
    }

    #[test]
    fn public_band_never_reveals_an_exact_level() {
        assert_eq!(public_lair_band(1), "1–10");
        assert_eq!(public_lair_band(6), "51–60");
        assert_eq!(public_lair_band(10), "91–100");
        assert_eq!(public_lair_band(0), "unavailable");
    }

    #[test]
    fn portrait_requires_exact_creature_manifest_entry_and_delivered_category() {
        let manifest = vec![ContentManifestEntrySnapshot {
            content_id: id("creature_red_fox"),
            content_kind_id: id("creature"),
            display_name: text("Red Fox"),
            art_key: id("art_creature_red_fox"),
            accessibility_label: text("Red Fox portrait"),
            capability_ids: Vec::new(),
        }];
        let row = creature_portrait_row("red_fox", 8_500, ReportConfidence::Moderate, &manifest)
            .expect("exact portrait");
        assert_eq!(row.art_key, "art_creature_red_fox");
        assert!(row.asset_path.ends_with("art_creature_red_fox.png"));
        assert!(
            creature_portrait_row("badger", 10_000, ReportConfidence::High, &manifest).is_none()
        );
    }

    #[test]
    fn only_bounded_hole_and_hunting_nudges_are_remote_actions() {
        assert!(is_lai50_allowed_remote_action(
            &CanonicalGodAction::BroadDomainNudge {
                domain: NudgeDomain::Hunting,
                building_kind_id: None,
                basis_points: LAI50_NUDGE_BASIS_POINTS,
            }
        ));
        assert!(!is_lai50_allowed_remote_action(
            &CanonicalGodAction::Inspiration
        ));
    }

    #[test]
    fn visual_direction_forbids_dashboard_tropes() {
        assert!(LAI50_VISUAL_DIRECTION.parchment);
        assert!(LAI50_VISUAL_DIRECTION.wood_rules);
        assert!(LAI50_VISUAL_DIRECTION.dark_forest_context);
        assert!(!LAI50_VISUAL_DIRECTION.uses_glass);
        assert!(!LAI50_VISUAL_DIRECTION.uses_glow);
        assert!(!LAI50_VISUAL_DIRECTION.uses_kpi_cards);
    }
}
