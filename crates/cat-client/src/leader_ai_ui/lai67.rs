//! LAI.67 report-safe Research and Council surfaces.
//!
//! The surfaces in this module consume only
//! [`cat_protocol::lai64::CanonicalSnapshotEnvelope`] and can emit only an
//! allowed [`cat_protocol::lai64::CanonicalGodAction`].  In particular, this
//! is not a second planner: it never guesses research prerequisites, exact
//! ecology, worker eligibility, trade consent, or any hidden world state.
//! When the canonical report does not carry a fact, the UI says so plainly.

use std::collections::{BTreeMap, BTreeSet};

use accesskit::{Action, Role};
use bevy::a11y::{AccessibilityNode, ActionRequest as AccessibilityActionRequest};
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use cat_protocol::lai64::{
    CanonicalColonySnapshot, CanonicalGodAction, CanonicalSnapshotEnvelope, EmergencySupply,
    PersonalStance, ReportConfidence, ResearchLane, TaskState, TradeStage,
};

use super::{
    lai54::{
        bevy_shell::{Lai54LiveShell, Lai54ShellRoot, ui_scale_for_window_scale},
        layout::{ClientPlatform, LayoutMode, UiScale, Viewport, shell_layout},
        shell::{CouncilTab, PrimaryScreen},
    },
    semantic_node, semantic_status_node,
};

pub const MAX_LAI67_RENDERED_ROWS: usize = 200;
pub const LAI67_STUDY_GRAPH_REGIONS: [&str; 3] = [
    "Foundations and provisions",
    "Village craft and institutions",
    "Hole, lore, and expeditions",
];

const INK: Color = Color::srgb(0.153, 0.106, 0.086);
const PARCHMENT: Color = Color::srgb(0.937, 0.886, 0.741);
const PAPER_SHADE: Color = Color::srgb(0.866, 0.792, 0.635);
const DARK_FOREST: Color = Color::srgb(0.090, 0.235, 0.180);
const WOOD: Color = Color::srgb(0.427, 0.282, 0.169);
const STONE: Color = Color::srgb(0.48, 0.46, 0.39);
const MOSS: Color = Color::srgb(0.310, 0.439, 0.251);
const RUST: Color = Color::srgb(0.643, 0.286, 0.176);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Lai67RefreshState {
    #[default]
    Loading,
    Ready,
    Stale {
        stale_since_ms: i64,
    },
    Conflict {
        reason: String,
    },
    UpdateRequired,
    Error {
        message: String,
    },
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai67SnapshotFeed {
    pub envelope: Option<CanonicalSnapshotEnvelope>,
    pub refresh: Lai67RefreshState,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Lai67SurfaceState {
    #[default]
    Loading,
    Ready,
    Empty,
    Stale {
        stale_since_ms: i64,
    },
    Conflict {
        reason: String,
    },
    UpdateRequired,
    Error {
        message: String,
    },
}

impl Lai67SurfaceState {
    #[must_use]
    pub const fn keeps_last_report_visible(&self) -> bool {
        matches!(
            self,
            Self::Ready | Self::Empty | Self::Stale { .. } | Self::Conflict { .. }
        )
    }

    #[must_use]
    pub const fn blocks_remote_actions(&self) -> bool {
        matches!(
            self,
            Self::Loading | Self::Conflict { .. } | Self::UpdateRequired | Self::Error { .. }
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lai67Availability<T> {
    Reported(T),
    Unavailable { reason: String },
}

impl<T> Default for Lai67Availability<T> {
    fn default() -> Self {
        Self::Unavailable {
            reason: "This field has not been reported.".to_owned(),
        }
    }
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai67ViewState {
    pub selected_study_id: Option<String>,
    pub selected_plan_id: Option<String>,
    pub selected_task_id: Option<String>,
    pub selected_cat_id: Option<String>,
    pub selected_trade_id: Option<String>,
    pub focused_control_id: Option<String>,
    pub refresh_requests: u64,
    pub last_local_feedback: Option<String>,
}

/// The UI writes an allowed action intent here; the authenticated client
/// transport owns the versioned `CanonicalActionEnvelope` and server result.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai67ActionIntent {
    pub sequence: u64,
    pub pending: Option<CanonicalGodAction>,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai67ProjectionResource(pub Lai67Projection);

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
struct Lai67RenderState {
    dirty: bool,
    route: Option<PrimaryScreen>,
    council_tab: CouncilTab,
}

impl Default for Lai67RenderState {
    fn default() -> Self {
        Self {
            dirty: true,
            route: None,
            council_tab: CouncilTab::Plans,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lai67Projection {
    pub selected_colony_id: Option<String>,
    pub now_ms: Option<i64>,
    pub state_version: Option<u64>,
    pub research: ResearchProjection,
    pub council: CouncilProjection,
    pub reads_authoritative_world_truth: bool,
    pub recomputes_hidden_rules: bool,
    pub emits_disallowed_controls: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResearchProjection {
    pub state: Lai67SurfaceState,
    pub notes_balance: Lai67Availability<u64>,
    pub void_balance: Lai67Availability<u64>,
    pub catalog: Vec<ResearchCatalogRow>,
    pub god_queue: Vec<ResearchQueueRow>,
    pub leader_lane: Vec<ResearchQueueRow>,
    pub preparations: Vec<ResearchPreparationRow>,
    pub graph_regions: [ResearchGraphRegion; 3],
    pub selected_study: Option<ResearchStudyInspector>,
    pub prerequisite_edges: Lai67Availability<Vec<ResearchGraphEdge>>,
    pub physical_scholar_work: Lai67Availability<Vec<ScholarWorkRow>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchCatalogRow {
    pub study_id: String,
    pub display_name: String,
    pub source_kind: String,
    pub semantic_id: String,
    pub capability_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchQueueRow {
    pub study_id: String,
    pub lane: ResearchLane,
    pub position: u8,
    pub funding_state: String,
    pub progress_basis_points: u16,
    pub duplicate_reason: Option<String>,
    pub refund_reason: Option<String>,
    pub semantic_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchPreparationRow {
    pub preparation_id: String,
    pub study_id: String,
    pub physical_task_id: Option<String>,
    pub progress_basis_points: u16,
    pub player_discount_basis_points: u16,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResearchGraphRegion {
    pub region_id: String,
    pub label: String,
    pub nodes: Vec<ResearchGraphNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchGraphNode {
    pub study_id: String,
    pub display_name: String,
    pub lane: Lai67Availability<ResearchLane>,
    pub progress_basis_points: Lai67Availability<u16>,
    pub status: Lai67Availability<String>,
    pub semantic_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchGraphEdge {
    pub from_study_id: String,
    pub to_study_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchStudyInspector {
    pub study_id: String,
    pub display_name: String,
    pub god_queue: Option<ResearchQueueRow>,
    pub leader_decision: Option<ResearchQueueRow>,
    pub preparation: Option<ResearchPreparationRow>,
    pub duplicate_or_overtake_explanation: Lai67Availability<String>,
    pub physical_scholar_work: Lai67Availability<ScholarWorkRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScholarWorkRow {
    pub task_id: String,
    pub task_kind_id: String,
    pub site_id: String,
    pub state: TaskState,
    pub ordered_footprint: Vec<(i32, i32)>,
    pub ordered_route: Vec<(i32, i32)>,
    pub worker_cat_ids: Vec<String>,
    pub blocker_reasons: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CouncilProjection {
    pub state: Lai67SurfaceState,
    pub plans: CouncilPlansProjection,
    pub tasks: CouncilTasksProjection,
    pub cats: CouncilCatsProjection,
    pub hole: CouncilHoleProjection,
    pub diplomacy: CouncilDiplomacyProjection,
    pub trade: CouncilTradeProjection,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CouncilPlansProjection {
    pub rows: Vec<CouncilPlanRow>,
    pub officer_requests: Vec<CouncilOfficerRequestRow>,
    pub standing_order_capabilities: Vec<CouncilStandingOrderCapabilityRow>,
    pub standing_orders: Vec<CouncilStandingOrderRow>,
    pub selected: Option<CouncilPlanRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CouncilPlanRow {
    pub plan_id: String,
    pub topic_id: String,
    pub phase: String,
    pub priority_basis_points: u16,
    pub confidence: ReportConfidence,
    pub rationale: String,
    pub dependencies: Vec<(String, bool)>,
    pub responsible_officer_id: Option<String>,
    pub semantic_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CouncilOfficerRequestRow {
    pub request_id: String,
    pub officer_id: String,
    pub request_kind: String,
    pub rationale: String,
    pub confidence: ReportConfidence,
    pub capability_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CouncilStandingOrderCapabilityRow {
    pub capability_id: String,
    pub office_id: String,
    pub order_kind_id: String,
    pub enabled: bool,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CouncilStandingOrderRow {
    pub order_id: String,
    pub capability_id: String,
    pub instruction: String,
    pub expires_at_ms: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CouncilTasksProjection {
    pub rows: Vec<CouncilTaskRow>,
    pub selected: Option<CouncilTaskRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CouncilTaskRow {
    pub task_id: String,
    pub task_kind_id: String,
    pub site_id: String,
    pub objective: String,
    pub state: TaskState,
    pub ordered_footprint: Vec<(i32, i32)>,
    pub ordered_route: Vec<(i32, i32)>,
    pub worker_cat_ids: Vec<String>,
    pub cargo_ids: Vec<String>,
    pub reservation_ids: Vec<String>,
    pub blockers: Vec<(String, bool)>,
    pub refusal_reasons: Vec<String>,
    pub anatomy_requirements: Vec<String>,
    pub semantic_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CouncilCatsProjection {
    pub rows: Vec<CouncilCatRow>,
    pub selected: Option<CouncilCatInspector>,
    pub election_id: Option<String>,
    pub candidates: Vec<CouncilCandidateRow>,
    pub officers: Vec<CouncilOfficerRow>,
    pub succession_summary: Lai67Availability<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CouncilCatRow {
    pub cat_id: String,
    pub display_name: String,
    pub office_id: Option<String>,
    pub succession_eligible: bool,
    pub semantic_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CouncilCatInspector {
    pub cat: CouncilCatRow,
    pub attributes: Vec<CouncilAttributeRow>,
    pub skills: Vec<CouncilSkillRow>,
    pub affinities: Vec<CouncilAffinityRow>,
    pub anatomy_eligibility: Vec<String>,
    pub household_id: Option<String>,
    pub parent_ids: Vec<String>,
    pub child_ids: Vec<String>,
    pub residence_id: Option<String>,
    pub mentor_id: Option<String>,
    pub tradition_id: Option<String>,
    pub surname: Option<String>,
    pub enterprise_id: Option<String>,
    pub active_task_ids: Vec<String>,
    pub equipment: Lai67Availability<Vec<String>>,
    pub stress: Lai67Availability<String>,
    pub office_history: Lai67Availability<Vec<String>>,
    pub personal_history: Lai67Availability<Vec<String>>,
    pub expulsion: Lai67Availability<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CouncilAttributeRow {
    pub attribute_id: String,
    pub inherited_value: u16,
    pub learned_value: u16,
    pub total_value: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CouncilSkillRow {
    pub skill_id: String,
    pub xp: u64,
    pub level: u16,
    pub mastery: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CouncilAffinityRow {
    pub labor_id: String,
    pub disposition: String,
    pub refusing: bool,
    pub refusal_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CouncilCandidateRow {
    pub cat_id: String,
    pub display_name: String,
    pub report_reason: String,
    pub backing_blocks: u8,
    pub eligible: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CouncilOfficerRow {
    pub office_id: String,
    pub cat_id: Option<String>,
    pub display_name: Option<String>,
    pub effective_expertise: u8,
    pub appointment_candidate_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CouncilHoleProjection {
    pub hole_id: Lai67Availability<String>,
    pub axes: Lai67Availability<HoleAxisReport>,
    pub landmark_footprint: Lai67Availability<Vec<(i32, i32)>>,
    pub work_footprint: Lai67Availability<Vec<(i32, i32)>>,
    pub food_permission_summary: Lai67Availability<String>,
    pub food_permissions: Vec<HoleFoodPermissionRow>,
    pub officer_report_level: Lai67Availability<u8>,
    pub regeneration: Lai67Availability<RegenerationReportRow>,
    pub contribution_receipt_ids: Vec<String>,
    pub notes_balance: Lai67Availability<u64>,
    pub void_balance: Lai67Availability<u64>,
    pub inspiration: Lai67Availability<InspirationRow>,
    pub boosts: Lai67Availability<Vec<String>>,
    pub boost_offers: Vec<DivineBoostOfferRow>,
    pub rescue: Lai67Availability<RescueRow>,
    pub construction_miracles: Vec<ConstructionMiracleRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoleAxisReport {
    pub width: u8,
    pub depth: u8,
    pub darkness: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoleFoodPermissionRow {
    pub content_id: String,
    pub permission: String,
    pub reason: String,
    pub confidence: ReportConfidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegenerationReportRow {
    pub lower_units_per_day: u64,
    pub upper_units_per_day: u64,
    pub observed_at_ms: i64,
    pub confidence: ReportConfidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspirationRow {
    pub expires_at_ms: Option<i64>,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RescueRow {
    pub available: bool,
    pub reason: Option<String>,
    pub offers: Vec<EmergencyRescueOfferRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DivineBoostOfferRow {
    pub offer_id: String,
    pub boost_type_id: String,
    pub duration_game_hours: u32,
    pub exact_cost_micro_void: u64,
    pub effect_basis_points: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmergencyRescueOfferRow {
    pub witness_id: String,
    pub supply: EmergencySupply,
    pub quantity: u64,
    pub exact_cost_micro_void: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstructionMiracleRow {
    pub offer_id: String,
    pub project_id: String,
    pub building_id: String,
    pub phase: String,
    pub exact_cost_micro_void: u64,
    pub labor_reduction_basis_points: u16,
    pub input_value_multiplier_basis_points: u16,
    pub ordered_footprint: Vec<(i32, i32)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CouncilDiplomacyProjection {
    pub rows: Vec<DiplomacyStanceRow>,
    pub explanation: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiplomacyStanceRow {
    pub other_colony_id: String,
    pub stance: PersonalStance,
    pub consented: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CouncilTradeProjection {
    pub rows: Vec<TradeContractRow>,
    pub direct_trade_controls: Lai67Availability<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeContractRow {
    pub contract_id: String,
    pub partner_colony_id: String,
    pub stage: TradeStage,
    pub ordered_route: Vec<(i32, i32)>,
    pub escrow: Vec<TradeCargoRow>,
    pub report_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeCargoRow {
    pub cargo_id: String,
    pub content_id: String,
    pub quantity: u64,
    pub quality_band: u8,
}

/// Project an immutable report into the complete LAI.67 presentation model.
/// All ordering is stable-ID based after the authoritative queue order.
#[must_use]
pub fn project_lai67_reports(feed: &Lai67SnapshotFeed, view: &Lai67ViewState) -> Lai67Projection {
    let state = state_from_feed(feed);
    let Some(envelope) = feed.envelope.as_ref() else {
        return Lai67Projection {
            research: ResearchProjection {
                state: state.clone(),
                ..default()
            },
            council: CouncilProjection { state, ..default() },
            ..default()
        };
    };
    let Some(colony) = selected_colony(envelope) else {
        let unavailable = Lai67SurfaceState::Error {
            message: "The selected colony is absent from this report.".to_owned(),
        };
        return Lai67Projection {
            selected_colony_id: Some(envelope.selected_colony_id.as_str().to_owned()),
            now_ms: Some(envelope.now_ms),
            research: ResearchProjection {
                state: unavailable.clone(),
                ..default()
            },
            council: CouncilProjection {
                state: unavailable,
                ..default()
            },
            ..default()
        };
    };

    Lai67Projection {
        selected_colony_id: Some(colony.colony_id.as_str().to_owned()),
        now_ms: Some(envelope.now_ms),
        state_version: Some(colony.state_version),
        research: project_research(colony, state.clone(), view),
        council: project_council(colony, state, view),
        reads_authoritative_world_truth: false,
        recomputes_hidden_rules: false,
        emits_disallowed_controls: false,
    }
}

fn selected_colony(envelope: &CanonicalSnapshotEnvelope) -> Option<&CanonicalColonySnapshot> {
    envelope
        .colonies
        .iter()
        .find(|colony| colony.colony_id == envelope.selected_colony_id)
}

fn state_from_feed(feed: &Lai67SnapshotFeed) -> Lai67SurfaceState {
    match &feed.refresh {
        Lai67RefreshState::Loading => Lai67SurfaceState::Loading,
        Lai67RefreshState::Ready if feed.envelope.is_none() => Lai67SurfaceState::Empty,
        Lai67RefreshState::Ready => Lai67SurfaceState::Ready,
        Lai67RefreshState::Stale { stale_since_ms } => Lai67SurfaceState::Stale {
            stale_since_ms: *stale_since_ms,
        },
        Lai67RefreshState::Conflict { reason } => Lai67SurfaceState::Conflict {
            reason: reason.clone(),
        },
        Lai67RefreshState::UpdateRequired => Lai67SurfaceState::UpdateRequired,
        Lai67RefreshState::Error { message } => Lai67SurfaceState::Error {
            message: message.clone(),
        },
    }
}

fn project_research(
    colony: &CanonicalColonySnapshot,
    state: Lai67SurfaceState,
    view: &Lai67ViewState,
) -> ResearchProjection {
    let mut catalog = research_catalog(colony);
    let mut god_queue = colony
        .research
        .god_queue
        .iter()
        .map(research_queue_row)
        .collect::<Vec<_>>();
    let mut leader_lane = colony
        .research
        .leader_decisions
        .iter()
        .map(research_queue_row)
        .collect::<Vec<_>>();
    let mut preparations = colony
        .research
        .preparations
        .iter()
        .map(|entry| ResearchPreparationRow {
            preparation_id: entry.preparation_id.as_str().to_owned(),
            study_id: entry.study_id.as_str().to_owned(),
            physical_task_id: entry
                .physical_task_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            progress_basis_points: entry.progress_basis_points,
            player_discount_basis_points: entry.player_discount_basis_points,
        })
        .collect::<Vec<_>>();
    catalog.sort_by(|left, right| left.study_id.cmp(&right.study_id));
    god_queue.sort_by(|left, right| {
        left.position
            .cmp(&right.position)
            .then_with(|| left.study_id.cmp(&right.study_id))
    });
    leader_lane.sort_by(|left, right| {
        left.position
            .cmp(&right.position)
            .then_with(|| left.study_id.cmp(&right.study_id))
    });
    preparations.sort_by(|left, right| left.preparation_id.cmp(&right.preparation_id));

    let graph_regions = graph_regions(&catalog, &god_queue, &leader_lane, &preparations);
    let selected_study_id = view.selected_study_id.clone().or_else(|| {
        god_queue
            .first()
            .map(|entry| entry.study_id.clone())
            .or_else(|| leader_lane.first().map(|entry| entry.study_id.clone()))
            .or_else(|| catalog.first().map(|entry| entry.study_id.clone()))
    });
    let selected_study = selected_study_id.as_deref().map(|study_id| {
        let god = god_queue
            .iter()
            .find(|entry| entry.study_id == study_id)
            .cloned();
        let leader = leader_lane
            .iter()
            .find(|entry| entry.study_id == study_id)
            .cloned();
        let preparation = preparations
            .iter()
            .find(|entry| entry.study_id == study_id)
            .cloned();
        let duplicate_or_overtake_explanation = duplicate_explanation(
            god.as_ref(),
            leader.as_ref(),
            preparation.as_ref(),
        );
        let scholar = preparation
            .as_ref()
            .and_then(|entry| entry.physical_task_id.as_deref())
            .and_then(|task_id| scholar_work_row(colony, task_id));
        ResearchStudyInspector {
            study_id: study_id.to_owned(),
            display_name: display_name_for_study(&catalog, study_id),
            god_queue: god,
            leader_decision: leader,
            preparation,
            duplicate_or_overtake_explanation,
            physical_scholar_work: scholar.map_or_else(
                || Lai67Availability::Unavailable {
                    reason: "No physical scholar task for this reported preparation is currently in the selected colony report.".to_owned(),
                },
                Lai67Availability::Reported,
            ),
        }
    });

    let physical_scholar_work = Lai67Availability::Reported(
        colony
            .tasks
            .iter()
            .filter(|task| is_scholar_task(task.task_kind_id.as_str()))
            .map(task_to_scholar_row)
            .collect(),
    );
    ResearchProjection {
        state,
        notes_balance: Lai67Availability::Reported(colony.research.notes_balance),
        void_balance: Lai67Availability::Reported(colony.research.void_balance),
        catalog,
        god_queue,
        leader_lane,
        preparations,
        graph_regions,
        selected_study,
        prerequisite_edges: Lai67Availability::Unavailable {
            reason: "Canonical schema v2 does not report prerequisite edges. The graph keeps its three regions but will not invent dependency lines.".to_owned(),
        },
        physical_scholar_work,
    }
}

fn research_catalog(colony: &CanonicalColonySnapshot) -> Vec<ResearchCatalogRow> {
    let mut rows = BTreeMap::<String, ResearchCatalogRow>::new();
    if let Some(manifest) = &colony.content_manifest {
        for entry in &manifest.entries {
            rows.insert(
                entry.content_id.as_str().to_owned(),
                ResearchCatalogRow {
                    study_id: entry.content_id.as_str().to_owned(),
                    display_name: entry.display_name.as_str().to_owned(),
                    source_kind: entry.content_kind_id.as_str().to_owned(),
                    semantic_id: stable_semantic_id("research-catalog", entry.content_id.as_str()),
                    capability_ids: entry
                        .capability_ids
                        .iter()
                        .map(|id| id.as_str().to_owned())
                        .collect(),
                },
            );
        }
    }
    for entry in colony
        .research
        .god_queue
        .iter()
        .chain(&colony.research.leader_decisions)
    {
        rows.entry(entry.study_id.as_str().to_owned())
            .or_insert_with(|| ResearchCatalogRow {
                study_id: entry.study_id.as_str().to_owned(),
                display_name: entry.study_id.as_str().to_owned(),
                source_kind: "reported study".to_owned(),
                semantic_id: stable_semantic_id("research-catalog", entry.study_id.as_str()),
                capability_ids: Vec::new(),
            });
    }
    for preparation in &colony.research.preparations {
        rows.entry(preparation.study_id.as_str().to_owned())
            .or_insert_with(|| ResearchCatalogRow {
                study_id: preparation.study_id.as_str().to_owned(),
                display_name: preparation.study_id.as_str().to_owned(),
                source_kind: "prepared study".to_owned(),
                semantic_id: stable_semantic_id("research-catalog", preparation.study_id.as_str()),
                capability_ids: Vec::new(),
            });
    }
    rows.into_values().collect()
}

fn research_queue_row(entry: &cat_protocol::lai64::ResearchQueueEntrySnapshot) -> ResearchQueueRow {
    ResearchQueueRow {
        study_id: entry.study_id.as_str().to_owned(),
        lane: entry.lane,
        position: entry.position,
        funding_state: entry.funding_state.as_str().to_owned(),
        progress_basis_points: entry.progress_basis_points,
        duplicate_reason: entry
            .duplicate_reason
            .as_ref()
            .map(|reason| reason.as_str().to_owned()),
        refund_reason: entry
            .refund_reason
            .as_ref()
            .map(|reason| reason.as_str().to_owned()),
        semantic_id: stable_semantic_id("research-queue", entry.study_id.as_str()),
    }
}

fn graph_regions(
    catalog: &[ResearchCatalogRow],
    god_queue: &[ResearchQueueRow],
    leader_lane: &[ResearchQueueRow],
    preparations: &[ResearchPreparationRow],
) -> [ResearchGraphRegion; 3] {
    let mut rows =
        BTreeMap::<String, (String, Option<ResearchQueueRow>, Option<ResearchQueueRow>)>::new();
    for entry in catalog {
        rows.insert(
            entry.study_id.clone(),
            (entry.display_name.clone(), None, None),
        );
    }
    for entry in god_queue {
        rows.entry(entry.study_id.clone())
            .or_insert_with(|| (entry.study_id.clone(), None, None))
            .1 = Some(entry.clone());
    }
    for entry in leader_lane {
        rows.entry(entry.study_id.clone())
            .or_insert_with(|| (entry.study_id.clone(), None, None))
            .2 = Some(entry.clone());
    }
    let prepared = preparations
        .iter()
        .map(|row| row.study_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut regions = std::array::from_fn(|index| ResearchGraphRegion {
        region_id: format!("region-{index}"),
        label: LAI67_STUDY_GRAPH_REGIONS[index].to_owned(),
        nodes: Vec::new(),
    });
    for (study_id, (display_name, god, leader)) in rows {
        let (lane, progress, status) = match (god, leader) {
            (Some(god), Some(leader)) => (
                Lai67Availability::Reported(ResearchLane::God),
                Lai67Availability::Reported(god.progress_basis_points),
                Lai67Availability::Reported(format!(
                    "God queue {} · Leader decision {}; {}",
                    god.funding_state,
                    leader.funding_state,
                    if prepared.contains(study_id.as_str()) {
                        "preparation reported"
                    } else {
                        "no preparation reported"
                    }
                )),
            ),
            (Some(god), None) => (
                Lai67Availability::Reported(ResearchLane::God),
                Lai67Availability::Reported(god.progress_basis_points),
                Lai67Availability::Reported(god.funding_state),
            ),
            (None, Some(leader)) => (
                Lai67Availability::Reported(ResearchLane::Leader),
                Lai67Availability::Reported(leader.progress_basis_points),
                Lai67Availability::Reported(leader.funding_state),
            ),
            (None, None) => (
                Lai67Availability::Unavailable {
                    reason: "No lane assignment is reported for this catalog entry.".to_owned(),
                },
                Lai67Availability::Unavailable {
                    reason: "No research progress is reported for this catalog entry.".to_owned(),
                },
                Lai67Availability::Unavailable {
                    reason: "No study state is reported for this catalog entry.".to_owned(),
                },
            ),
        };
        let index = research_region_for(&study_id);
        regions[index].nodes.push(ResearchGraphNode {
            display_name,
            semantic_id: stable_semantic_id("research-graph", &study_id),
            study_id,
            lane,
            progress_basis_points: progress,
            status,
        });
    }
    for region in &mut regions {
        region
            .nodes
            .sort_by(|left, right| left.study_id.cmp(&right.study_id));
    }
    regions
}

fn research_region_for(study_id: &str) -> usize {
    let normalized = study_id.to_ascii_lowercase();
    if ["hole", "void", "hunt", "lair", "ritual", "lore"]
        .iter()
        .any(|needle| normalized.contains(needle))
    {
        2
    } else if [
        "craft", "cook", "mill", "build", "home", "family", "storage", "road", "workshop", "metal",
        "cloth", "school",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
    {
        1
    } else {
        0
    }
}

fn duplicate_explanation(
    god: Option<&ResearchQueueRow>,
    leader: Option<&ResearchQueueRow>,
    preparation: Option<&ResearchPreparationRow>,
) -> Lai67Availability<String> {
    if let Some(reason) = god
        .and_then(|entry| entry.refund_reason.as_ref())
        .or_else(|| leader.and_then(|entry| entry.refund_reason.as_ref()))
    {
        return Lai67Availability::Reported(format!("Refund reported: {reason}"));
    }
    if let Some(reason) = god
        .and_then(|entry| entry.duplicate_reason.as_ref())
        .or_else(|| leader.and_then(|entry| entry.duplicate_reason.as_ref()))
    {
        return Lai67Availability::Reported(format!("Duplicate or overtake reported: {reason}"));
    }
    if let (Some(_), Some(_)) = (god, leader) {
        return Lai67Availability::Reported(
            "The study appears in both lanes, but the report supplies no duplicate/overtake reason."
                .to_owned(),
        );
    }
    if preparation.is_some() {
        return Lai67Availability::Reported(
            "Preparation is reported. It is physical scholar work and does not create a third currency."
                .to_owned(),
        );
    }
    Lai67Availability::Unavailable {
        reason:
            "No duplicate, overtake, refund, or preparation explanation is reported for this study."
                .to_owned(),
    }
}

fn display_name_for_study(catalog: &[ResearchCatalogRow], study_id: &str) -> String {
    catalog
        .iter()
        .find(|entry| entry.study_id == study_id)
        .map_or_else(|| study_id.to_owned(), |entry| entry.display_name.clone())
}

fn is_scholar_task(task_kind_id: &str) -> bool {
    let id = task_kind_id.to_ascii_lowercase();
    id.contains("research") || id.contains("scholar") || id.contains("prepare")
}

fn scholar_work_row(colony: &CanonicalColonySnapshot, task_id: &str) -> Option<ScholarWorkRow> {
    colony
        .tasks
        .iter()
        .find(|task| task.task_id.as_str() == task_id)
        .map(task_to_scholar_row)
}

fn task_to_scholar_row(task: &cat_protocol::lai64::PhysicalTaskSnapshot) -> ScholarWorkRow {
    ScholarWorkRow {
        task_id: task.task_id.as_str().to_owned(),
        task_kind_id: task.task_kind_id.as_str().to_owned(),
        site_id: task.site_id.as_str().to_owned(),
        state: task.state,
        ordered_footprint: tiles(&task.footprint.ordered_tiles),
        ordered_route: tiles(&task.route.ordered_tiles),
        worker_cat_ids: task
            .worker_cat_ids
            .iter()
            .map(|id| id.as_str().to_owned())
            .collect(),
        blocker_reasons: task
            .blockers
            .iter()
            .map(|blocker| blocker.reason.as_str().to_owned())
            .collect(),
    }
}

fn project_council(
    colony: &CanonicalColonySnapshot,
    state: Lai67SurfaceState,
    view: &Lai67ViewState,
) -> CouncilProjection {
    CouncilProjection {
        state,
        plans: project_plans(colony, view),
        tasks: project_tasks(colony, view),
        cats: project_cats(colony, view),
        hole: project_hole(colony),
        diplomacy: project_diplomacy(colony),
        trade: project_trade(colony),
    }
}

fn project_plans(
    colony: &CanonicalColonySnapshot,
    view: &Lai67ViewState,
) -> CouncilPlansProjection {
    let mut rows = colony
        .plans
        .iter()
        .map(|plan| CouncilPlanRow {
            plan_id: plan.plan_id.as_str().to_owned(),
            topic_id: plan.topic_id.as_str().to_owned(),
            phase: plan.phase.as_str().to_owned(),
            priority_basis_points: plan.priority_basis_points,
            confidence: plan.confidence,
            rationale: plan.rationale.as_str().to_owned(),
            dependencies: plan
                .dependencies
                .iter()
                .map(|dependency| (dependency.plan_id.as_str().to_owned(), dependency.satisfied))
                .collect(),
            responsible_officer_id: plan
                .responsible_officer_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            semantic_id: stable_semantic_id("council-plans", plan.plan_id.as_str()),
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .priority_basis_points
            .cmp(&left.priority_basis_points)
            .then_with(|| left.plan_id.cmp(&right.plan_id))
    });
    let selected = view
        .selected_plan_id
        .as_deref()
        .and_then(|id| rows.iter().find(|row| row.plan_id == id))
        .cloned()
        .or_else(|| rows.first().cloned());
    let mut officer_requests = colony
        .officer_requests
        .iter()
        .map(|request| CouncilOfficerRequestRow {
            request_id: request.request_id.as_str().to_owned(),
            officer_id: request.officer_id.as_str().to_owned(),
            request_kind: request.request_kind.as_str().to_owned(),
            rationale: request.rationale.as_str().to_owned(),
            confidence: request.confidence,
            capability_id: request
                .capability_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
        })
        .collect::<Vec<_>>();
    officer_requests.sort_by(|left, right| left.request_id.cmp(&right.request_id));
    let mut capabilities = colony
        .standing_order_capabilities
        .iter()
        .map(|entry| CouncilStandingOrderCapabilityRow {
            capability_id: entry.capability_id.as_str().to_owned(),
            office_id: entry.office_id.as_str().to_owned(),
            order_kind_id: entry.order_kind_id.as_str().to_owned(),
            enabled: entry.enabled,
            reason: entry.reason.as_str().to_owned(),
        })
        .collect::<Vec<_>>();
    capabilities.sort_by(|left, right| left.capability_id.cmp(&right.capability_id));
    let mut orders = colony
        .standing_orders
        .iter()
        .map(|entry| CouncilStandingOrderRow {
            order_id: entry.order_id.as_str().to_owned(),
            capability_id: entry.capability_id.as_str().to_owned(),
            instruction: entry.instruction.as_str().to_owned(),
            expires_at_ms: entry.expires_at_ms,
        })
        .collect::<Vec<_>>();
    orders.sort_by(|left, right| left.order_id.cmp(&right.order_id));
    CouncilPlansProjection {
        rows,
        officer_requests,
        standing_order_capabilities: capabilities,
        standing_orders: orders,
        selected,
    }
}

fn project_tasks(
    colony: &CanonicalColonySnapshot,
    view: &Lai67ViewState,
) -> CouncilTasksProjection {
    let mut rows = colony
        .tasks
        .iter()
        .filter(|task| !matches!(task.state, TaskState::Complete | TaskState::Refused))
        .map(|task| CouncilTaskRow {
            task_id: task.task_id.as_str().to_owned(),
            task_kind_id: task.task_kind_id.as_str().to_owned(),
            site_id: task.site_id.as_str().to_owned(),
            objective: task.objective.as_str().to_owned(),
            state: task.state,
            ordered_footprint: tiles(&task.footprint.ordered_tiles),
            ordered_route: tiles(&task.route.ordered_tiles),
            worker_cat_ids: task
                .worker_cat_ids
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
            cargo_ids: task
                .cargo
                .iter()
                .map(|cargo| cargo.cargo_id.as_str().to_owned())
                .collect(),
            reservation_ids: task
                .reservations
                .iter()
                .map(|reservation| reservation.reservation_id.as_str().to_owned())
                .collect(),
            blockers: task
                .blockers
                .iter()
                .map(|blocker| (blocker.reason.as_str().to_owned(), blocker.recoverable))
                .collect(),
            refusal_reasons: task
                .refusals
                .iter()
                .map(|refusal| refusal.reason.as_str().to_owned())
                .collect(),
            anatomy_requirements: task
                .anatomy_requirements
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
            semantic_id: stable_semantic_id("council-tasks", task.task_id.as_str()),
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.task_id.cmp(&right.task_id));
    let selected = view
        .selected_task_id
        .as_deref()
        .and_then(|id| rows.iter().find(|row| row.task_id == id))
        .cloned()
        .or_else(|| rows.first().cloned());
    CouncilTasksProjection { rows, selected }
}

fn project_cats(colony: &CanonicalColonySnapshot, view: &Lai67ViewState) -> CouncilCatsProjection {
    let names = colony
        .cats
        .iter()
        .map(|cat| (cat.cat_id.as_str(), cat.display_name.as_str().to_owned()))
        .collect::<BTreeMap<_, _>>();
    let mut rows = colony
        .cats
        .iter()
        .map(|cat| CouncilCatRow {
            cat_id: cat.cat_id.as_str().to_owned(),
            display_name: cat.display_name.as_str().to_owned(),
            office_id: cat.office_id.as_ref().map(|id| id.as_str().to_owned()),
            succession_eligible: cat.succession_eligible,
            semantic_id: stable_semantic_id("council-cats", cat.cat_id.as_str()),
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.cat_id.cmp(&right.cat_id));
    let selected_id = view
        .selected_cat_id
        .as_deref()
        .or_else(|| rows.first().map(|row| row.cat_id.as_str()));
    let selected = selected_id.and_then(|id| {
        let cat = colony.cats.iter().find(|cat| cat.cat_id.as_str() == id)?;
        let base = rows.iter().find(|row| row.cat_id == id)?.clone();
        let active_task_ids = colony
            .tasks
            .iter()
            .filter(|task| task.worker_cat_ids.iter().any(|worker| worker.as_str() == id))
            .map(|task| task.task_id.as_str().to_owned())
            .collect();
        Some(CouncilCatInspector {
            cat: base,
            attributes: cat
                .attributes
                .iter()
                .map(|attribute| CouncilAttributeRow {
                    attribute_id: attribute.attribute_id.as_str().to_owned(),
                    inherited_value: attribute.inherited_value,
                    learned_value: attribute.learned_value,
                    total_value: attribute.total_value,
                })
                .collect(),
            skills: cat
                .skills
                .iter()
                .map(|skill| CouncilSkillRow {
                    skill_id: skill.skill_id.as_str().to_owned(),
                    xp: skill.xp,
                    level: skill.level,
                    mastery: skill.mastery,
                })
                .collect(),
            affinities: cat
                .affinities
                .iter()
                .map(|affinity| CouncilAffinityRow {
                    labor_id: affinity.labor_id.as_str().to_owned(),
                    disposition: affinity.disposition.as_str().to_owned(),
                    refusing: affinity.refusing,
                    refusal_reason: affinity
                        .refusal_reason
                        .as_ref()
                        .map(|reason| reason.as_str().to_owned()),
                })
                .collect(),
            anatomy_eligibility: cat
                .anatomy_eligibility
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
            household_id: cat
                .family
                .household_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            parent_ids: cat
                .family
                .parent_ids
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
            child_ids: cat
                .family
                .child_ids
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
            residence_id: cat
                .family
                .residence_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            mentor_id: cat
                .family
                .mentor_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            tradition_id: cat
                .family
                .tradition_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            surname: cat.family.surname.as_ref().map(|name| name.as_str().to_owned()),
            enterprise_id: cat
                .family
                .enterprise_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            active_task_ids,
            equipment: Lai67Availability::Unavailable {
                reason: "Exact equipment is not linked to cats in canonical schema v2.".to_owned(),
            },
            stress: Lai67Availability::Unavailable {
                reason: "Stress is not reported by canonical schema v2.".to_owned(),
            },
            office_history: Lai67Availability::Unavailable {
                reason: "Historical office service is not reported by canonical schema v2.".to_owned(),
            },
            personal_history: Lai67Availability::Unavailable {
                reason: "Personal history is not reported by canonical schema v2.".to_owned(),
            },
            expulsion: Lai67Availability::Unavailable {
                reason: "The canonical action exists, but adult/guardian eligibility is not reported here. The UI will not guess eligibility or expose a misleading expulsion control.".to_owned(),
            },
        })
    });
    let mut candidates = colony
        .governance
        .candidates
        .iter()
        .map(|candidate| CouncilCandidateRow {
            cat_id: candidate.cat_id.as_str().to_owned(),
            display_name: names
                .get(candidate.cat_id.as_str())
                .cloned()
                .unwrap_or_else(|| candidate.cat_id.as_str().to_owned()),
            report_reason: candidate.report_reason.as_str().to_owned(),
            backing_blocks: candidate.backing_blocks,
            eligible: candidate.eligible,
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.cat_id.cmp(&right.cat_id));
    let mut officers = colony
        .governance
        .officers
        .iter()
        .map(|officer| CouncilOfficerRow {
            office_id: officer.office_id.as_str().to_owned(),
            cat_id: officer.cat_id.as_ref().map(|id| id.as_str().to_owned()),
            display_name: officer
                .cat_id
                .as_ref()
                .and_then(|id| names.get(id.as_str()))
                .cloned(),
            effective_expertise: officer.report_expertise_level,
            appointment_candidate_ids: officer
                .appointment_candidate_ids
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
        })
        .collect::<Vec<_>>();
    officers.sort_by(|left, right| left.office_id.cmp(&right.office_id));
    CouncilCatsProjection {
        rows,
        selected,
        election_id: colony
            .governance
            .election_id
            .as_ref()
            .map(|id| id.as_str().to_owned()),
        candidates,
        officers,
        succession_summary: colony.governance.succession_summary.as_ref().map_or_else(
            || Lai67Availability::Unavailable {
                reason: "No succession report is available.".to_owned(),
            },
            |summary| Lai67Availability::Reported(summary.as_str().to_owned()),
        ),
    }
}

fn project_hole(colony: &CanonicalColonySnapshot) -> CouncilHoleProjection {
    let hole = &colony.hole;
    let regeneration = if hole.officer_report_level < 4 {
        Lai67Availability::Unavailable {
            reason: "Regeneration estimates require effective officer report level 4. No server value is inferred below that gate.".to_owned(),
        }
    } else {
        hole.officer_reported_regeneration.as_ref().map_or_else(
            || Lai67Availability::Unavailable {
                reason: "An eligible officer level is reported, but no regeneration estimate has arrived.".to_owned(),
            },
            |estimate| {
                Lai67Availability::Reported(RegenerationReportRow {
                    lower_units_per_day: estimate.lower_units_per_day,
                    upper_units_per_day: estimate.upper_units_per_day,
                    observed_at_ms: estimate.observed_at_ms,
                    confidence: estimate.confidence,
                })
            },
        )
    };
    let mut food_permissions = hole
        .food_permissions
        .iter()
        .map(|permission| HoleFoodPermissionRow {
            content_id: permission.content_id.as_str().to_owned(),
            permission: format!("{:?}", permission.permission),
            reason: permission.reason.as_str().to_owned(),
            confidence: permission.confidence,
        })
        .collect::<Vec<_>>();
    food_permissions.sort_by(|left, right| left.content_id.cmp(&right.content_id));
    let mut construction_miracles = colony
        .divine
        .construction_miracle_offers
        .iter()
        .map(|offer| ConstructionMiracleRow {
            offer_id: offer.offer_id.as_str().to_owned(),
            project_id: offer.project_id.as_str().to_owned(),
            building_id: offer.building_id.as_str().to_owned(),
            phase: format!("{:?}", offer.phase),
            exact_cost_micro_void: offer.exact_cost_micro_void,
            labor_reduction_basis_points: offer.labor_reduction_basis_points,
            input_value_multiplier_basis_points: offer.input_value_multiplier_basis_points,
            ordered_footprint: tiles(&offer.footprint.ordered_tiles),
        })
        .collect::<Vec<_>>();
    construction_miracles.sort_by(|left, right| left.offer_id.cmp(&right.offer_id));
    CouncilHoleProjection {
        hole_id: Lai67Availability::Reported(hole.hole_id.as_str().to_owned()),
        axes: Lai67Availability::Reported(HoleAxisReport {
            width: hole.width,
            depth: hole.depth,
            darkness: hole.darkness,
        }),
        landmark_footprint: Lai67Availability::Reported(tiles(&hole.footprint.ordered_tiles)),
        work_footprint: Lai67Availability::Reported(tiles(&hole.work_footprint.ordered_tiles)),
        food_permission_summary: Lai67Availability::Reported(
            hole.food_permission_summary.as_str().to_owned(),
        ),
        food_permissions,
        officer_report_level: Lai67Availability::Reported(hole.officer_report_level),
        regeneration,
        contribution_receipt_ids: hole
            .contribution_receipts
            .iter()
            .map(|id| id.as_str().to_owned())
            .collect(),
        notes_balance: Lai67Availability::Reported(colony.research.notes_balance),
        void_balance: Lai67Availability::Reported(colony.research.void_balance),
        inspiration: Lai67Availability::Reported(InspirationRow {
            expires_at_ms: colony.divine.inspiration_expires_at_ms,
            active: colony.divine.inspiration_expires_at_ms.is_some(),
        }),
        boosts: Lai67Availability::Reported(
            colony
                .divine
                .active_boost_ids
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
        ),
        boost_offers: colony
            .divine
            .boost_offers
            .iter()
            .map(|offer| DivineBoostOfferRow {
                offer_id: offer.offer_id.as_str().to_owned(),
                boost_type_id: offer.boost_type_id.as_str().to_owned(),
                duration_game_hours: offer.duration_game_hours,
                exact_cost_micro_void: offer.exact_cost_micro_void,
                effect_basis_points: offer.effect_basis_points,
            })
            .collect(),
        rescue: Lai67Availability::Reported(RescueRow {
            available: colony.divine.rescue_available,
            reason: colony
                .divine
                .rescue_reason
                .as_ref()
                .map(|reason| reason.as_str().to_owned()),
            offers: colony
                .divine
                .rescue_offers
                .iter()
                .map(|offer| EmergencyRescueOfferRow {
                    witness_id: offer.witness_id.as_str().to_owned(),
                    supply: offer.supply,
                    quantity: offer.quantity,
                    exact_cost_micro_void: offer.exact_cost_micro_void,
                })
                .collect(),
        }),
        construction_miracles,
    }
}

fn project_diplomacy(colony: &CanonicalColonySnapshot) -> CouncilDiplomacyProjection {
    let mut rows = colony
        .diplomacy
        .stances
        .iter()
        .map(|stance| DiplomacyStanceRow {
            other_colony_id: stance.other_colony_id.as_str().to_owned(),
            stance: stance.stance,
            consented: stance.consented,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.other_colony_id.cmp(&right.other_colony_id));
    CouncilDiplomacyProjection {
        rows,
        explanation: "Alliance and Neutral are currently equivalent for trade. Enemy prevents eligible outbound barter and is checked before any escrow or caravan is created.".to_owned(),
    }
}

fn project_trade(colony: &CanonicalColonySnapshot) -> CouncilTradeProjection {
    let mut rows = colony
        .diplomacy
        .contracts
        .iter()
        .map(|contract| TradeContractRow {
            contract_id: contract.contract_id.as_str().to_owned(),
            partner_colony_id: contract.partner_colony_id.as_str().to_owned(),
            stage: contract.stage,
            ordered_route: tiles(&contract.route.ordered_tiles),
            escrow: contract
                .escrow
                .iter()
                .map(|cargo| TradeCargoRow {
                    cargo_id: cargo.cargo_id.as_str().to_owned(),
                    content_id: cargo.content_id.as_str().to_owned(),
                    quantity: cargo.quantity,
                    quality_band: cargo.quality_band,
                })
                .collect(),
            report_reason: contract
                .report_reason
                .as_ref()
                .map(|reason| reason.as_str().to_owned()),
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.contract_id.cmp(&right.contract_id));
    CouncilTradeProjection {
        rows,
        direct_trade_controls: Lai67Availability::Unavailable {
            reason: "Trade consent, route selection, cargo selection, and barter placement are Leader/officer work. This report is intentionally inspection-only for Gods.".to_owned(),
        },
    }
}

fn tiles(tiles: &[cat_protocol::lai64::Tile]) -> Vec<(i32, i32)> {
    tiles.iter().map(|tile| (tile.x, tile.y)).collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lai67LayoutContract {
    pub mode: LayoutMode,
    pub catalog_width_percent: f32,
    pub graph_width_percent: f32,
    pub inspector_width_percent: f32,
    pub minimum_pane_height_px: u16,
    pub row_minimum_height_px: u16,
}

#[must_use]
pub fn lai67_layout_contract(
    platform: ClientPlatform,
    viewport: Viewport,
    ui_scale: UiScale,
) -> Option<Lai67LayoutContract> {
    let shell = shell_layout(platform, viewport, ui_scale).ok()?;
    Some(match shell.mode {
        LayoutMode::Wide => Lai67LayoutContract {
            mode: LayoutMode::Wide,
            catalog_width_percent: 25.0,
            graph_width_percent: 50.0,
            inspector_width_percent: 25.0,
            minimum_pane_height_px: 280,
            row_minimum_height_px: 34,
        },
        LayoutMode::Compact => Lai67LayoutContract {
            mode: LayoutMode::Compact,
            catalog_width_percent: 100.0,
            graph_width_percent: 100.0,
            inspector_width_percent: 100.0,
            minimum_pane_height_px: 220,
            row_minimum_height_px: 34,
        },
    })
}

#[derive(Component)]
pub struct Lai67ReportsRoot;
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lai67ScreenRoot(pub PrimaryScreen);
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lai67Workspace(pub PrimaryScreen);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lai67PaneKind {
    ResearchCatalog,
    ResearchGraph,
    ResearchInspector,
    CouncilIndex,
    CouncilDetail,
}
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lai67Pane {
    pub screen: PrimaryScreen,
    pub kind: Lai67PaneKind,
}
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lai67FeedbackLabel(pub PrimaryScreen);
#[derive(Component)]
pub struct Lai67ScrollablePane;

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct Lai67Control {
    pub screen: PrimaryScreen,
    pub stable_id: String,
    pub focus_order: u32,
    pub action: Lai67ControlAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lai67ControlAction {
    Refresh,
    SelectStudy(String),
    SelectPlan(String),
    SelectTask(String),
    SelectCat(String),
    SelectTrade(String),
    EmitCanonical(CanonicalGodAction),
}

/// The public plugin is intentionally additive; the integration owner wires it
/// from `leader_ai_ui::mod` and the root app after the canonical protocol path
/// is the only active snapshot/action path.
#[derive(Default)]
pub struct Lai67ResearchCouncilPlugin;

impl Plugin for Lai67ResearchCouncilPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Lai67SnapshotFeed>()
            .init_resource::<Lai67ViewState>()
            .init_resource::<Lai67ActionIntent>()
            .init_resource::<Lai67ProjectionResource>()
            .init_resource::<Lai67RenderState>()
            .add_message::<MouseWheel>()
            .add_message::<AccessibilityActionRequest>()
            .add_systems(
                Update,
                (
                    attach_lai67_surfaces,
                    sync_lai67_projection,
                    sync_lai67_route_visibility,
                    render_lai67_projection,
                    handle_lai67_pointer_controls,
                    handle_lai67_keyboard,
                    handle_lai67_accessibility_actions,
                    sync_lai67_focus_style,
                    sync_lai67_layout,
                    handle_lai67_scroll,
                )
                    .chain(),
            );
    }
}

fn attach_lai67_surfaces(
    mut commands: Commands<'_, '_>,
    shell: Query<'_, '_, Entity, With<Lai54ShellRoot>>,
    existing: Query<'_, '_, Entity, With<Lai67ReportsRoot>>,
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
            GlobalZIndex(1_306),
            BackgroundColor(DARK_FOREST),
            BorderColor::all(WOOD),
            Lai67ReportsRoot,
            crate::WorldInputBlocker,
            Name::new("LAI.67 report-safe Research and Council"),
        ))
        .id();
    commands.entity(shell).add_child(root);
    spawn_lai67_screen(&mut commands, root, PrimaryScreen::Research);
    spawn_lai67_screen(&mut commands, root, PrimaryScreen::Council);
}

fn spawn_lai67_screen(commands: &mut Commands<'_, '_>, parent: Entity, screen: PrimaryScreen) {
    let heading = screen_label(screen);
    let screen_root = commands
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
            Lai67ScreenRoot(screen),
            semantic_node(
                Role::Pane,
                format!("lai67:{}:panel", heading.to_ascii_lowercase()),
                format!("{heading} report"),
                true,
            ),
            Name::new(format!("LAI.67 {heading} screen")),
        ))
        .id();
    commands.entity(parent).add_child(screen_root);
    commands.entity(screen_root).with_children(|root| {
        root.spawn(text_bundle(heading, 24.0, INK));
        root.spawn((
            text_bundle("Loading report-safe colony data", 13.0, RUST),
            Lai67FeedbackLabel(screen),
            semantic_status_node(
                format!("lai67:{}:status", heading.to_ascii_lowercase()),
                format!("{heading} is loading"),
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
            Lai67Workspace(screen),
            Name::new(format!("LAI.67 {heading} workspace")),
        ))
        .id();
    commands.entity(screen_root).add_child(workspace);
    match screen {
        PrimaryScreen::Research => {
            spawn_lai67_pane(
                commands,
                workspace,
                screen,
                Lai67PaneKind::ResearchCatalog,
                25.0,
                PAPER_SHADE,
                WOOD,
            );
            spawn_lai67_pane(
                commands,
                workspace,
                screen,
                Lai67PaneKind::ResearchGraph,
                50.0,
                PARCHMENT,
                STONE,
            );
            spawn_lai67_pane(
                commands,
                workspace,
                screen,
                Lai67PaneKind::ResearchInspector,
                25.0,
                PAPER_SHADE,
                WOOD,
            );
        }
        PrimaryScreen::Council => {
            spawn_lai67_pane(
                commands,
                workspace,
                screen,
                Lai67PaneKind::CouncilIndex,
                39.0,
                PAPER_SHADE,
                WOOD,
            );
            spawn_lai67_pane(
                commands,
                workspace,
                screen,
                Lai67PaneKind::CouncilDetail,
                61.0,
                PARCHMENT,
                STONE,
            );
        }
        _ => {}
    }
}

fn spawn_lai67_pane(
    commands: &mut Commands<'_, '_>,
    parent: Entity,
    screen: PrimaryScreen,
    kind: Lai67PaneKind,
    width: f32,
    background: Color,
    border: Color,
) {
    let pane = commands
        .spawn((
            Node {
                width: Val::Percent(width),
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
            BorderColor::all(border),
            Lai67Pane { screen, kind },
            Lai67ScrollablePane,
            Name::new(format!("LAI.67 {screen:?} {kind:?} pane")),
        ))
        .id();
    commands.entity(parent).add_child(pane);
}

fn sync_lai67_projection(
    feed: Res<'_, Lai67SnapshotFeed>,
    view: Res<'_, Lai67ViewState>,
    mut projection: ResMut<'_, Lai67ProjectionResource>,
    mut render: ResMut<'_, Lai67RenderState>,
) {
    if feed.is_changed() || view.is_changed() {
        projection.0 = project_lai67_reports(&feed, &view);
        render.dirty = true;
    }
}

fn sync_lai67_route_visibility(
    live: Option<Res<'_, Lai54LiveShell>>,
    mut root: Query<'_, '_, &mut Node, With<Lai67ReportsRoot>>,
    mut screens: Query<'_, '_, (&Lai67ScreenRoot, &mut Node), Without<Lai67ReportsRoot>>,
    mut render: ResMut<'_, Lai67RenderState>,
) {
    let route = live
        .as_ref()
        .and_then(|live| live.router.visible_primary())
        .filter(|screen| matches!(screen, PrimaryScreen::Research | PrimaryScreen::Council));
    let council_tab = live
        .as_ref()
        .map_or(CouncilTab::Plans, |live| live.router.council_tab());
    if render.route != route || render.council_tab != council_tab {
        render.route = route;
        render.council_tab = council_tab;
        render.dirty = true;
    }
    if let Ok(mut node) = root.single_mut() {
        node.display = if route.is_some() {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (screen, mut node) in &mut screens {
        node.display = if route == Some(screen.0) {
            Display::Flex
        } else {
            Display::None
        };
    }
}

#[allow(clippy::too_many_arguments)]
fn render_lai67_projection(
    mut commands: Commands<'_, '_>,
    projection: Res<'_, Lai67ProjectionResource>,
    mut render: ResMut<'_, Lai67RenderState>,
    panes: Query<'_, '_, (Entity, &Lai67Pane)>,
    mut feedback: Query<'_, '_, (&Lai67FeedbackLabel, &mut Text, &mut AccessibilityNode)>,
) {
    if !render.dirty || panes.is_empty() {
        return;
    }
    for (marker, mut text, mut accessibility) in &mut feedback {
        let state = match marker.0 {
            PrimaryScreen::Research => &projection.0.research.state,
            PrimaryScreen::Council => &projection.0.council.state,
            _ => continue,
        };
        let copy = surface_state_copy(state);
        text.0.clone_from(&copy);
        *accessibility = semantic_status_node(
            format!(
                "lai67:{}:status",
                screen_label(marker.0).to_ascii_lowercase()
            ),
            copy,
            matches!(
                state,
                Lai67SurfaceState::Conflict { .. }
                    | Lai67SurfaceState::Error { .. }
                    | Lai67SurfaceState::UpdateRequired
            ),
        );
    }
    for (pane, marker) in &panes {
        commands.entity(pane).despawn_children();
        match marker.kind {
            Lai67PaneKind::ResearchCatalog => {
                render_research_catalog(&mut commands, pane, &projection.0.research)
            }
            Lai67PaneKind::ResearchGraph => {
                render_research_graph(&mut commands, pane, &projection.0.research)
            }
            Lai67PaneKind::ResearchInspector => {
                render_research_inspector(&mut commands, pane, &projection.0.research)
            }
            Lai67PaneKind::CouncilIndex => render_council_index(
                &mut commands,
                pane,
                &projection.0.council,
                render.council_tab,
            ),
            Lai67PaneKind::CouncilDetail => render_council_detail(
                &mut commands,
                pane,
                &projection.0.council,
                render.council_tab,
            ),
        }
    }
    render.dirty = false;
}

fn render_research_catalog(
    commands: &mut Commands<'_, '_>,
    pane: Entity,
    research: &ResearchProjection,
) {
    spawn_section_text(
        commands,
        pane,
        "Research ledger",
        &format!(
            "Research Notes: {}\nVoid Insight: {}\nGod lane and free Leader lane are independent. The Leader's instant decisions do not spend player currency or consume preparation.",
            availability_label(&research.notes_balance),
            availability_label(&research.void_balance),
        ),
    );
    spawn_lai67_control(
        commands,
        pane,
        PrimaryScreen::Research,
        "refresh",
        1,
        "Refresh report",
        Lai67ControlAction::Refresh,
    );
    spawn_section_text(
        commands,
        pane,
        "God queue",
        &queue_body(&research.god_queue, "No God research is queued."),
    );
    for (index, entry) in research
        .god_queue
        .iter()
        .take(MAX_LAI67_RENDERED_ROWS)
        .enumerate()
    {
        spawn_lai67_control(
            commands,
            pane,
            PrimaryScreen::Research,
            &format!("study-{}", entry.study_id),
            100 + index as u32,
            &format!("Inspect God queue {}", entry.study_id),
            Lai67ControlAction::SelectStudy(entry.study_id.clone()),
        );
    }
    spawn_section_text(
        commands,
        pane,
        "Leader lane",
        &queue_body(
            &research.leader_lane,
            "No free Leader decision is reported in this review window.",
        ),
    );
    spawn_section_text(
        commands,
        pane,
        "Catalog",
        &format!(
            "{} report-visible entries. Content is listed by the manifest; prerequisite edges are not inferred.",
            research.catalog.len()
        ),
    );
    for (index, entry) in research
        .catalog
        .iter()
        .take(MAX_LAI67_RENDERED_ROWS)
        .enumerate()
    {
        spawn_lai67_control(
            commands,
            pane,
            PrimaryScreen::Research,
            &entry.semantic_id,
            1_000 + index as u32,
            &format!("Inspect {}", entry.display_name),
            Lai67ControlAction::SelectStudy(entry.study_id.clone()),
        );
    }
}

fn render_research_graph(
    commands: &mut Commands<'_, '_>,
    pane: Entity,
    research: &ResearchProjection,
) {
    spawn_section_text(
        commands,
        pane,
        "Three-region research graph",
        "Fixed-scale regional graph. Pan/scroll lives inside this surface; the client does not zoom or manufacture missing prerequisite lines.",
    );
    for region in &research.graph_regions {
        let body = region
            .nodes
            .iter()
            .map(|node| {
                format!(
                    "{}\n  lane: {}\n  progress: {}\n  state: {}",
                    node.display_name,
                    availability_label(&node.lane),
                    availability_label(&node.progress_basis_points),
                    availability_label(&node.status),
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        spawn_section_text(commands, pane, &region.label, &body);
        for (index, node) in region
            .nodes
            .iter()
            .take(MAX_LAI67_RENDERED_ROWS)
            .enumerate()
        {
            spawn_lai67_control(
                commands,
                pane,
                PrimaryScreen::Research,
                &node.semantic_id,
                2_000 + index as u32,
                &format!("Focus graph study {}", node.display_name),
                Lai67ControlAction::SelectStudy(node.study_id.clone()),
            );
        }
    }
    spawn_availability(
        commands,
        pane,
        "Reported dependency edges",
        &research.prerequisite_edges,
    );
}

fn render_research_inspector(
    commands: &mut Commands<'_, '_>,
    pane: Entity,
    research: &ResearchProjection,
) {
    let Some(study) = &research.selected_study else {
        spawn_section_text(
            commands,
            pane,
            "Study inspector",
            "Choose a report-visible catalog or queue entry. No hidden study information is shown.",
        );
        return;
    };
    spawn_section_text(
        commands,
        pane,
        &study.display_name,
        &format!(
            "Stable study ID: {}\nGod lane: {}\nLeader lane: {}\nPreparation: {}",
            study.study_id,
            study
                .god_queue
                .as_ref()
                .map_or_else(|| "not queued".to_owned(), queue_entry_body),
            study
                .leader_decision
                .as_ref()
                .map_or_else(|| "no decision reported".to_owned(), queue_entry_body),
            study.preparation.as_ref().map_or_else(
                || "no preparation reported".to_owned(),
                |preparation| format!(
                    "{} · {}/10000 · {}% player discount · physical task {}",
                    preparation.preparation_id,
                    preparation.progress_basis_points,
                    preparation.player_discount_basis_points / 100,
                    preparation
                        .physical_task_id
                        .as_deref()
                        .unwrap_or("not linked")
                )
            )
        ),
    );
    spawn_availability(
        commands,
        pane,
        "Duplicate, overtake, and refund",
        &study.duplicate_or_overtake_explanation,
    );
    spawn_availability(
        commands,
        pane,
        "Physical scholar work",
        &study.physical_scholar_work,
    );
    let action_is_reported_study =
        study.god_queue.is_some() || study.leader_decision.is_some() || study.preparation.is_some();
    if action_is_reported_study {
        spawn_lai67_control(
            commands,
            pane,
            PrimaryScreen::Research,
            "queue-study",
            5_000,
            "Queue study in God lane",
            Lai67ControlAction::EmitCanonical(CanonicalGodAction::ResearchQueue {
                study_id: stable_id(&study.study_id),
            }),
        );
        spawn_lai67_control(
            commands,
            pane,
            PrimaryScreen::Research,
            "fund-study",
            5_001,
            "Fund front study",
            Lai67ControlAction::EmitCanonical(CanonicalGodAction::ResearchFund {
                study_id: stable_id(&study.study_id),
            }),
        );
        spawn_lai67_control(
            commands,
            pane,
            PrimaryScreen::Research,
            "prepare-study",
            5_002,
            "Request scholar preparation",
            Lai67ControlAction::EmitCanonical(CanonicalGodAction::ResearchPreparation {
                study_id: stable_id(&study.study_id),
            }),
        );
        spawn_lai67_control(
            commands,
            pane,
            PrimaryScreen::Research,
            "remove-study",
            5_003,
            "Remove God queue study",
            Lai67ControlAction::EmitCanonical(CanonicalGodAction::ResearchRemove {
                study_id: stable_id(&study.study_id),
            }),
        );
    } else {
        spawn_section_text(
            commands,
            pane,
            "God action availability",
            "This manifest entry is not a reported study target. The client does not guess a study ID or expose a control that could target unrelated content.",
        );
    }
    spawn_section_text(
        commands,
        pane,
        "Queue reordering",
        "Reorder is available only within the God queue and cannot cross prerequisites. Choose a reported queue position in the transport-integrated surface; this leaf does not fabricate missing queue targets.",
    );
}

fn render_council_index(
    commands: &mut Commands<'_, '_>,
    pane: Entity,
    council: &CouncilProjection,
    tab: CouncilTab,
) {
    match tab {
        CouncilTab::Plans => render_plans_index(commands, pane, &council.plans),
        CouncilTab::Tasks => render_tasks_index(commands, pane, &council.tasks),
        CouncilTab::Cats => render_cats_index(commands, pane, &council.cats),
        CouncilTab::Hole => render_hole_index(commands, pane, &council.hole),
        CouncilTab::Diplomacy => render_diplomacy_index(commands, pane, &council.diplomacy),
        CouncilTab::Trade => render_trade_index(commands, pane, &council.trade),
    }
}

fn render_council_detail(
    commands: &mut Commands<'_, '_>,
    pane: Entity,
    council: &CouncilProjection,
    tab: CouncilTab,
) {
    match tab {
        CouncilTab::Plans => render_plans_detail(commands, pane, &council.plans),
        CouncilTab::Tasks => render_tasks_detail(commands, pane, &council.tasks),
        CouncilTab::Cats => render_cats_detail(commands, pane, &council.cats),
        CouncilTab::Hole => render_hole_detail(commands, pane, &council.hole),
        CouncilTab::Diplomacy => render_diplomacy_detail(commands, pane, &council.diplomacy),
        CouncilTab::Trade => render_trade_detail(commands, pane, &council.trade),
    }
}

fn render_plans_index(
    commands: &mut Commands<'_, '_>,
    pane: Entity,
    plans: &CouncilPlansProjection,
) {
    spawn_section_text(
        commands,
        pane,
        "Plans",
        "Priorities, dependencies, and rationale are reported from leadership beliefs. God nudges remain broad; this panel has no officer appointment or standing-order editing control.",
    );
    for (index, plan) in plans.rows.iter().take(MAX_LAI67_RENDERED_ROWS).enumerate() {
        spawn_lai67_control(
            commands,
            pane,
            PrimaryScreen::Council,
            &plan.semantic_id,
            100 + index as u32,
            &format!(
                "{} · priority {} · {:?}",
                plan.topic_id, plan.priority_basis_points, plan.confidence
            ),
            Lai67ControlAction::SelectPlan(plan.plan_id.clone()),
        );
    }
    spawn_section_text(
        commands,
        pane,
        "Officer requests",
        &plans
            .officer_requests
            .iter()
            .map(|request| {
                format!(
                    "{} · {} · {:?}\n{}",
                    request.officer_id, request.request_kind, request.confidence, request.rationale
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

fn render_plans_detail(
    commands: &mut Commands<'_, '_>,
    pane: Entity,
    plans: &CouncilPlansProjection,
) {
    if let Some(plan) = &plans.selected {
        spawn_section_text(
            commands,
            pane,
            &plan.topic_id,
            &format!(
                "Phase: {}\nPriority: {}\nConfidence: {:?}\nOfficer: {}\nRationale: {}\nDependencies: {}",
                plan.phase,
                plan.priority_basis_points,
                plan.confidence,
                plan.responsible_officer_id
                    .as_deref()
                    .unwrap_or("no officer reported"),
                plan.rationale,
                plan.dependencies
                    .iter()
                    .map(|(id, satisfied)| format!(
                        "{id}: {}",
                        if *satisfied { "met" } else { "waiting" }
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    } else {
        spawn_section_text(
            commands,
            pane,
            "Plan detail",
            "No leadership plan is reported.",
        );
    }
    spawn_section_text(
        commands,
        pane,
        "Officer capability and standing orders",
        &plans
            .standing_order_capabilities
            .iter()
            .map(|capability| {
                format!(
                    "{} · {} · {} · {}\n{}",
                    capability.office_id,
                    capability.capability_id,
                    capability.order_kind_id,
                    if capability.enabled {
                        "enabled"
                    } else {
                        "unavailable"
                    },
                    capability.reason
                )
            })
            .chain(plans.standing_orders.iter().map(|order| {
                format!(
                    "{} · {} · {} · expiry {}",
                    order.order_id,
                    order.capability_id,
                    order.instruction,
                    order
                        .expires_at_ms
                        .map_or_else(|| "not reported".to_owned(), |time| time.to_string())
                )
            }))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    spawn_lai67_control(
        commands,
        pane,
        PrimaryScreen::Council,
        "nudge-research",
        9_000,
        "Nudge broad research attention",
        Lai67ControlAction::EmitCanonical(CanonicalGodAction::BroadDomainNudge {
            domain: cat_protocol::lai64::NudgeDomain::Research,
            building_kind_id: None,
            basis_points: 500,
        }),
    );
}

fn render_tasks_index(
    commands: &mut Commands<'_, '_>,
    pane: Entity,
    tasks: &CouncilTasksProjection,
) {
    spawn_section_text(
        commands,
        pane,
        "Physical tasks",
        "Only open physical tasks are listed. Selecting a row focuses its exact reported site, complete footprint, route, cargo, and bounded blockers; it never invents a generic map marker.",
    );
    for (index, task) in tasks.rows.iter().take(MAX_LAI67_RENDERED_ROWS).enumerate() {
        spawn_lai67_control(
            commands,
            pane,
            PrimaryScreen::Council,
            &task.semantic_id,
            100 + index as u32,
            &format!(
                "{} · {} · {:?}",
                task.site_id, task.task_kind_id, task.state
            ),
            Lai67ControlAction::SelectTask(task.task_id.clone()),
        );
    }
}

fn render_tasks_detail(
    commands: &mut Commands<'_, '_>,
    pane: Entity,
    tasks: &CouncilTasksProjection,
) {
    let Some(task) = &tasks.selected else {
        spawn_section_text(
            commands,
            pane,
            "Task geometry",
            "Choose an open task to inspect its exact site and full geometry.",
        );
        return;
    };
    spawn_section_text(
        commands,
        pane,
        &task.task_kind_id,
        &format!(
            "Task: {}\nSite: {}\nObjective: {}\nState: {:?}\nFull ordered footprint: {}\nOrdered route: {}\nWorkers: {}\nCargo: {}\nReservations: {}\nAnatomy requirements: {}\nBlockers: {}\nRefusals: {}",
            task.task_id,
            task.site_id,
            task.objective,
            task.state,
            format_tiles(&task.ordered_footprint),
            format_tiles(&task.ordered_route),
            fallback_join(&task.worker_cat_ids),
            fallback_join(&task.cargo_ids),
            fallback_join(&task.reservation_ids),
            fallback_join(&task.anatomy_requirements),
            task.blockers
                .iter()
                .map(|(reason, recoverable)| format!(
                    "{} ({})",
                    reason,
                    if *recoverable {
                        "recoverable"
                    } else {
                        "reported blocking"
                    }
                ))
                .collect::<Vec<_>>()
                .join("; "),
            fallback_join(&task.refusal_reasons),
        ),
    );
    spawn_section_text(
        commands,
        pane,
        "World focus",
        "World-marker focus is a presentation selection only. Gods cannot reassign workers, choose routes, change the task site, or edit task cargo from this panel.",
    );
}

fn render_cats_index(commands: &mut Commands<'_, '_>, pane: Entity, cats: &CouncilCatsProjection) {
    spawn_section_text(
        commands,
        pane,
        "Cat register",
        "DF-style records expose only reported attributes, learned skills, family links, affinities, anatomy eligibility, civic data, and exact work references.",
    );
    for (index, cat) in cats.rows.iter().take(MAX_LAI67_RENDERED_ROWS).enumerate() {
        spawn_lai67_control(
            commands,
            pane,
            PrimaryScreen::Council,
            &cat.semantic_id,
            100 + index as u32,
            &format!(
                "{} · office {} · {}",
                cat.display_name,
                cat.office_id.as_deref().unwrap_or("none"),
                if cat.succession_eligible {
                    "succession eligible"
                } else {
                    "not reported eligible"
                }
            ),
            Lai67ControlAction::SelectCat(cat.cat_id.clone()),
        );
    }
    spawn_section_text(
        commands,
        pane,
        "Election and officers",
        &format!(
            "Election: {}\n{}\n{}",
            cats.election_id
                .as_deref()
                .unwrap_or("no election reported"),
            cats.candidates
                .iter()
                .map(|candidate| format!(
                    "{} · {} backing · {}",
                    candidate.display_name, candidate.backing_blocks, candidate.report_reason
                ))
                .collect::<Vec<_>>()
                .join("\n"),
            cats.officers
                .iter()
                .map(|officer| format!(
                    "{} · {} · effective expertise {}",
                    officer.office_id,
                    officer.display_name.as_deref().unwrap_or("vacant"),
                    officer.effective_expertise
                ))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    );
}

fn render_cats_detail(commands: &mut Commands<'_, '_>, pane: Entity, cats: &CouncilCatsProjection) {
    let Some(inspector) = &cats.selected else {
        spawn_section_text(commands, pane, "Cat inspector", "No cat is reported.");
        return;
    };
    spawn_section_text(
        commands,
        pane,
        &inspector.cat.display_name,
        &format!(
            "ID: {}\nOffice: {}\nActive tasks: {}\nAttributes: {}\nSkills: {}\nAffinities: {}\nAnatomy eligibility: {}",
            inspector.cat.cat_id,
            inspector.cat.office_id.as_deref().unwrap_or("none"),
            fallback_join(&inspector.active_task_ids),
            inspector
                .attributes
                .iter()
                .map(|attribute| format!(
                    "{} inherited {} + learned {} = {}",
                    attribute.attribute_id,
                    attribute.inherited_value,
                    attribute.learned_value,
                    attribute.total_value
                ))
                .collect::<Vec<_>>()
                .join("; "),
            inspector
                .skills
                .iter()
                .map(|skill| format!(
                    "{} L{} · XP {} · Mastery {}",
                    skill.skill_id, skill.level, skill.xp, skill.mastery
                ))
                .collect::<Vec<_>>()
                .join("; "),
            inspector
                .affinities
                .iter()
                .map(|affinity| format!(
                    "{}: {}{}",
                    affinity.labor_id,
                    affinity.disposition,
                    affinity
                        .refusal_reason
                        .as_ref()
                        .map_or_else(String::new, |reason| format!(" · {reason}"))
                ))
                .collect::<Vec<_>>()
                .join("; "),
            fallback_join(&inspector.anatomy_eligibility),
        ),
    );
    spawn_section_text(
        commands,
        pane,
        "Family, profession, and residence",
        &format!(
            "Household: {}\nParents: {}\nChildren: {}\nResidence: {}\nMentor: {}\nTradition: {}\nSurname: {}\nEnterprise: {}",
            inspector.household_id.as_deref().unwrap_or("not reported"),
            fallback_join(&inspector.parent_ids),
            fallback_join(&inspector.child_ids),
            inspector.residence_id.as_deref().unwrap_or("not reported"),
            inspector.mentor_id.as_deref().unwrap_or("not reported"),
            inspector.tradition_id.as_deref().unwrap_or("not reported"),
            inspector.surname.as_deref().unwrap_or("not reported"),
            inspector.enterprise_id.as_deref().unwrap_or("not reported"),
        ),
    );
    spawn_availability(commands, pane, "Equipment", &inspector.equipment);
    spawn_availability(commands, pane, "Stress", &inspector.stress);
    spawn_availability(commands, pane, "Office history", &inspector.office_history);
    spawn_availability(
        commands,
        pane,
        "Personal history",
        &inspector.personal_history,
    );
    spawn_availability(commands, pane, "Expulsion", &inspector.expulsion);
    if let Some(election_id) = &cats.election_id {
        for candidate in cats
            .candidates
            .iter()
            .filter(|candidate| candidate.eligible)
        {
            spawn_lai67_control(
                commands,
                pane,
                PrimaryScreen::Council,
                &format!("back-{}", candidate.cat_id),
                8_000,
                &format!("Give +10 backing to {}", candidate.display_name),
                Lai67ControlAction::EmitCanonical(CanonicalGodAction::CandidateBacking {
                    election_id: stable_id(election_id),
                    candidate_id: stable_id(&candidate.cat_id),
                }),
            );
        }
    }
    spawn_availability(commands, pane, "Succession", &cats.succession_summary);
}

fn render_hole_index(commands: &mut Commands<'_, '_>, pane: Entity, hole: &CouncilHoleProjection) {
    spawn_section_text(
        commands,
        pane,
        "The Hole",
        &format!(
            "Hole: {}\nAxes: {}\nVoid Insight: {}\nResearch Notes: {}\nFood policy: {}",
            availability_label(&hole.hole_id),
            availability_label(&hole.axes),
            availability_label(&hole.void_balance),
            availability_label(&hole.notes_balance),
            availability_label(&hole.food_permission_summary),
        ),
    );
    spawn_section_text(
        commands,
        pane,
        "Per-content food permissions",
        &hole
            .food_permissions
            .iter()
            .map(|permission| {
                format!(
                    "{} · {} · {:?}\n{}",
                    permission.content_id,
                    permission.permission,
                    permission.confidence,
                    permission.reason
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
    );
    spawn_section_text(
        commands,
        pane,
        "Report gate",
        &format!(
            "Effective officer report level: {}\nRegeneration: {}",
            availability_label(&hole.officer_report_level),
            availability_label(&hole.regeneration),
        ),
    );
}

fn render_hole_detail(commands: &mut Commands<'_, '_>, pane: Entity, hole: &CouncilHoleProjection) {
    spawn_section_text(
        commands,
        pane,
        "Landmark and feed work",
        &format!(
            "Full 5×5 landmark: {}\nCentral 3×3 work area: {}\nContribution receipts: {}",
            availability_label(&hole.landmark_footprint),
            availability_label(&hole.work_footprint),
            fallback_join(&hole.contribution_receipt_ids),
        ),
    );
    if let Lai67Availability::Reported(hole_id) = &hole.hole_id {
        spawn_lai67_control(
            commands,
            pane,
            PrimaryScreen::Council,
            "hole-click",
            9_000,
            "Contribute one ordinary divine click",
            Lai67ControlAction::EmitCanonical(CanonicalGodAction::HoleClickBatch {
                target_id: stable_id(hole_id),
                requested_clicks: 1,
                client_batch_window_ms: cat_protocol::lai64::CANONICAL_CLICK_BATCH_WINDOW_MS,
            }),
        );
    }
    spawn_lai67_control(
        commands,
        pane,
        PrimaryScreen::Council,
        "conserve-food",
        9_001,
        "Nudge broad food conservation",
        Lai67ControlAction::EmitCanonical(CanonicalGodAction::FoodConservation {
            nudge_basis_points: 500,
        }),
    );
    spawn_lai67_control(
        commands,
        pane,
        PrimaryScreen::Council,
        "inspiration",
        9_002,
        "Use personal Inspiration",
        Lai67ControlAction::EmitCanonical(CanonicalGodAction::Inspiration),
    );
    spawn_section_text(
        commands,
        pane,
        "Inspiration and specialized boosts",
        &format!(
            "Inspiration: {}\nActive specialized boosts: {}\nCurrent authenticated boost offers: {}",
            availability_label(&hole.inspiration),
            availability_label(&hole.boosts),
            hole.boost_offers.len(),
        ),
    );
    for (index, offer) in hole
        .boost_offers
        .iter()
        .take(MAX_LAI67_RENDERED_ROWS)
        .enumerate()
    {
        spawn_lai67_control(
            commands,
            pane,
            PrimaryScreen::Council,
            &format!("boost-{}", offer.offer_id),
            9_010 + index as u32,
            &format!(
                "{} for {} game hour(s) · {} µVoid · +{} bp",
                offer.boost_type_id,
                offer.duration_game_hours,
                offer.exact_cost_micro_void,
                offer.effect_basis_points,
            ),
            Lai67ControlAction::EmitCanonical(CanonicalGodAction::ActivateBoost {
                boost_id: stable_id(&offer.offer_id),
            }),
        );
    }
    spawn_section_text(
        commands,
        pane,
        "Miracles",
        &hole
            .construction_miracles
            .iter()
            .map(|project| {
                format!(
                    "{} · {} · {}\n{} µVoid · -{} bp original labor · {} bp Hole-feed-value cargo\nFootprint: {}",
                    project.project_id,
                    project.building_id,
                    project.phase,
                    project.exact_cost_micro_void,
                    project.labor_reduction_basis_points,
                    project.input_value_multiplier_basis_points,
                    format_tiles(&project.ordered_footprint)
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
    );
    for (index, project) in hole
        .construction_miracles
        .iter()
        .take(MAX_LAI67_RENDERED_ROWS)
        .enumerate()
    {
        spawn_lai67_control(
            commands,
            pane,
            PrimaryScreen::Council,
            &format!("miracle-{}", project.offer_id),
            9_100 + index as u32,
            &format!(
                "Miracle for {}: {} µVoid · 10% original labor · 2× Hole-feed-value cargo",
                project.building_id, project.exact_cost_micro_void,
            ),
            Lai67ControlAction::EmitCanonical(CanonicalGodAction::ConstructionMiracle {
                offer_id: stable_id(&project.offer_id),
            }),
        );
    }
    spawn_availability(commands, pane, "Emergency rescue", &hole.rescue);
    if let Lai67Availability::Reported(RescueRow {
        available: true,
        offers,
        ..
    }) = &hole.rescue
    {
        for (index, offer) in offers.iter().take(MAX_LAI67_RENDERED_ROWS).enumerate() {
            let label = match offer.supply {
                EmergencySupply::DivineRation => "Create emergency Divine Rations",
                EmergencySupply::DivineWater => "Create emergency Divine Water",
            };
            spawn_lai67_control(
                commands,
                pane,
                PrimaryScreen::Council,
                &format!("rescue-{}", offer.witness_id),
                9_200 + index as u32,
                &format!(
                    "{label}: {} units · {} µVoid",
                    offer.quantity, offer.exact_cost_micro_void,
                ),
                Lai67ControlAction::EmitCanonical(CanonicalGodAction::EmergencyRescue {
                    witness_id: stable_id(&offer.witness_id),
                }),
            );
        }
    }
}

fn render_diplomacy_index(
    commands: &mut Commands<'_, '_>,
    pane: Entity,
    diplomacy: &CouncilDiplomacyProjection,
) {
    spawn_section_text(commands, pane, "Diplomacy", &diplomacy.explanation);
    for (index, stance) in diplomacy
        .rows
        .iter()
        .take(MAX_LAI67_RENDERED_ROWS)
        .enumerate()
    {
        spawn_section_text(
            commands,
            pane,
            &stance.other_colony_id,
            &format!(
                "Personal stance: {:?}\nOther village consent: {}",
                stance.stance,
                if stance.consented {
                    "reported"
                } else {
                    "not reported"
                }
            ),
        );
        for (offset, stance_choice, label) in [
            (0_u32, PersonalStance::Alliance, "Choose Alliance"),
            (1_u32, PersonalStance::Neutral, "Choose Neutral"),
            (2_u32, PersonalStance::Enemy, "Choose Enemy"),
        ] {
            spawn_lai67_control(
                commands,
                pane,
                PrimaryScreen::Council,
                &format!("stance-{}-{offset}", stance.other_colony_id),
                100 + index as u32 * 10 + offset,
                label,
                Lai67ControlAction::EmitCanonical(CanonicalGodAction::PersonalStance {
                    other_colony_id: stable_id(&stance.other_colony_id),
                    stance: stance_choice,
                }),
            );
        }
    }
}

fn render_diplomacy_detail(
    commands: &mut Commands<'_, '_>,
    pane: Entity,
    diplomacy: &CouncilDiplomacyProjection,
) {
    spawn_section_text(
        commands,
        pane,
        "Honest stance rules",
        &format!(
            "{}\n\nNo monetary price, coin, or purse is represented. Current trade is physical barter only.",
            diplomacy.explanation
        ),
    );
}

fn render_trade_index(
    commands: &mut Commands<'_, '_>,
    pane: Entity,
    trade: &CouncilTradeProjection,
) {
    spawn_section_text(
        commands,
        pane,
        "Barter contracts",
        "Physical escrow, routes, stages, and recovery are reported below. No direct God trade-consent control is exposed.",
    );
    for (index, contract) in trade.rows.iter().take(MAX_LAI67_RENDERED_ROWS).enumerate() {
        spawn_lai67_control(
            commands,
            pane,
            PrimaryScreen::Council,
            &format!("trade-{}", contract.contract_id),
            100 + index as u32,
            &format!(
                "{} · {} · {:?}",
                contract.partner_colony_id, contract.contract_id, contract.stage
            ),
            Lai67ControlAction::SelectTrade(contract.contract_id.clone()),
        );
    }
}

fn render_trade_detail(
    commands: &mut Commands<'_, '_>,
    pane: Entity,
    trade: &CouncilTradeProjection,
) {
    spawn_availability(
        commands,
        pane,
        "Direct trade controls",
        &trade.direct_trade_controls,
    );
    spawn_section_text(
        commands,
        pane,
        "Physical barter ledger",
        &trade
            .rows
            .iter()
            .map(|contract| {
                format!(
                    "{} ↔ {} · {:?}\nRoute: {}\nEscrow: {}\n{}",
                    contract.contract_id,
                    contract.partner_colony_id,
                    contract.stage,
                    format_tiles(&contract.ordered_route),
                    contract
                        .escrow
                        .iter()
                        .map(|cargo| format!(
                            "{} {}×{} Q{}",
                            cargo.cargo_id, cargo.content_id, cargo.quantity, cargo.quality_band
                        ))
                        .collect::<Vec<_>>()
                        .join(", "),
                    contract
                        .report_reason
                        .as_deref()
                        .unwrap_or("No additional report reason.")
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

fn spawn_lai67_control(
    commands: &mut Commands<'_, '_>,
    parent: Entity,
    screen: PrimaryScreen,
    subject: &str,
    focus_order: u32,
    label: &str,
    action: Lai67ControlAction,
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
            Lai67Control {
                screen,
                stable_id: semantic_id,
                focus_order,
                action,
            },
            Name::new(format!("LAI.67 {label}")),
        ))
        .id();
    commands.entity(control).with_children(|button| {
        button.spawn(text_bundle(label, 12.0, INK));
    });
    commands.entity(parent).add_child(control);
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
            Name::new(format!("LAI.67 {heading} section")),
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

fn spawn_availability<T: std::fmt::Debug>(
    commands: &mut Commands<'_, '_>,
    parent: Entity,
    heading: &str,
    availability: &Lai67Availability<T>,
) {
    spawn_section_text(commands, parent, heading, &availability_label(availability));
}

fn availability_label<T: std::fmt::Debug>(availability: &Lai67Availability<T>) -> String {
    match availability {
        Lai67Availability::Reported(value) => format!("{value:?}"),
        Lai67Availability::Unavailable { reason } => format!("Not reported — {reason}"),
    }
}

fn queue_body(entries: &[ResearchQueueRow], empty: &str) -> String {
    if entries.is_empty() {
        return empty.to_owned();
    }
    entries
        .iter()
        .map(queue_entry_body)
        .collect::<Vec<_>>()
        .join("\n")
}

fn queue_entry_body(entry: &ResearchQueueRow) -> String {
    format!(
        "{} · position {} · {} · {}/10000{}{}",
        entry.study_id,
        entry.position,
        entry.funding_state,
        entry.progress_basis_points,
        entry
            .duplicate_reason
            .as_ref()
            .map_or_else(String::new, |reason| format!(" · duplicate: {reason}")),
        entry
            .refund_reason
            .as_ref()
            .map_or_else(String::new, |reason| format!(" · refund: {reason}")),
    )
}

fn handle_lai67_pointer_controls(
    mut interactions: Query<'_, '_, (&Interaction, &Lai67Control), Changed<Interaction>>,
    mut view: ResMut<'_, Lai67ViewState>,
    mut intent: ResMut<'_, Lai67ActionIntent>,
) {
    for (interaction, control) in &mut interactions {
        if *interaction == Interaction::Pressed {
            view.focused_control_id = Some(control.stable_id.clone());
            apply_lai67_action(&control.action, &mut view, &mut intent);
        }
    }
}

fn handle_lai67_keyboard(
    keys: Option<Res<'_, ButtonInput<KeyCode>>>,
    live: Option<Res<'_, Lai54LiveShell>>,
    controls: Query<'_, '_, &Lai67Control>,
    mut view: ResMut<'_, Lai67ViewState>,
    mut intent: ResMut<'_, Lai67ActionIntent>,
) {
    let Some(keys) = keys else {
        return;
    };
    let Some(screen) = live.and_then(|live| live.router.visible_primary()) else {
        return;
    };
    if !matches!(screen, PrimaryScreen::Research | PrimaryScreen::Council) {
        return;
    }
    let mut visible = controls
        .iter()
        .filter(|control| control.screen == screen)
        .cloned()
        .collect::<Vec<_>>();
    visible.sort_by(|left, right| {
        left.focus_order
            .cmp(&right.focus_order)
            .then_with(|| left.stable_id.cmp(&right.stable_id))
    });
    if visible.is_empty() {
        return;
    }
    let reverse = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let navigate = keys.just_pressed(KeyCode::Tab)
        || keys.just_pressed(KeyCode::ArrowDown)
        || keys.just_pressed(KeyCode::ArrowRight)
        || keys.just_pressed(KeyCode::ArrowUp)
        || keys.just_pressed(KeyCode::ArrowLeft);
    if navigate {
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
        apply_lai67_action(&control.action, &mut view, &mut intent);
    }
}

fn handle_lai67_accessibility_actions(
    mut requests: MessageReader<'_, '_, AccessibilityActionRequest>,
    controls: Query<'_, '_, &Lai67Control>,
    mut view: ResMut<'_, Lai67ViewState>,
    mut intent: ResMut<'_, Lai67ActionIntent>,
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
            apply_lai67_action(&control.action, &mut view, &mut intent);
        }
    }
}

fn apply_lai67_action(
    action: &Lai67ControlAction,
    view: &mut Lai67ViewState,
    intent: &mut Lai67ActionIntent,
) {
    match action {
        Lai67ControlAction::Refresh => {
            view.refresh_requests = view.refresh_requests.saturating_add(1);
            view.last_local_feedback = Some("A report refresh was requested.".to_owned());
        }
        Lai67ControlAction::SelectStudy(study_id) => {
            view.selected_study_id = Some(study_id.clone())
        }
        Lai67ControlAction::SelectPlan(plan_id) => view.selected_plan_id = Some(plan_id.clone()),
        Lai67ControlAction::SelectTask(task_id) => view.selected_task_id = Some(task_id.clone()),
        Lai67ControlAction::SelectCat(cat_id) => view.selected_cat_id = Some(cat_id.clone()),
        Lai67ControlAction::SelectTrade(contract_id) => {
            view.selected_trade_id = Some(contract_id.clone())
        }
        Lai67ControlAction::EmitCanonical(action) => {
            intent.sequence = intent.sequence.saturating_add(1);
            intent.pending = Some(action.clone());
            view.last_local_feedback = Some(
                "Canonical action prepared locally; it awaits authenticated transport and an authoritative result."
                    .to_owned(),
            );
        }
    }
}

fn sync_lai67_focus_style(
    view: Res<'_, Lai67ViewState>,
    mut controls: Query<'_, '_, (&Lai67Control, &mut BackgroundColor, &mut BorderColor)>,
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

fn sync_lai67_layout(
    windows: Query<'_, '_, &Window, With<PrimaryWindow>>,
    mut root: Query<'_, '_, &mut Node, With<Lai67ReportsRoot>>,
    mut workspaces: Query<'_, '_, (&Lai67Workspace, &mut Node)>,
    mut panes: Query<'_, '_, (&Lai67Pane, &mut Node), Without<Lai67ReportsRoot>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let platform = if cfg!(target_arch = "wasm32") {
        ClientPlatform::Wasm
    } else {
        ClientPlatform::Native
    };
    let Some(layout) = lai67_layout_contract(
        platform,
        Viewport::new(
            window.width().round() as u16,
            window.height().round() as u16,
        ),
        ui_scale_for_window_scale(window.scale_factor()),
    ) else {
        return;
    };
    if let Ok(mut node) = root.single_mut() {
        node.left = Val::Px(if layout.mode == LayoutMode::Wide {
            24.0
        } else {
            12.0
        });
        node.right = Val::Px(if layout.mode == LayoutMode::Wide {
            24.0
        } else {
            12.0
        });
        node.bottom = Val::Px(if layout.mode == LayoutMode::Wide {
            24.0
        } else {
            12.0
        });
    }
    for (_, mut workspace) in &mut workspaces {
        workspace.flex_direction = if layout.mode == LayoutMode::Wide {
            FlexDirection::Row
        } else {
            FlexDirection::Column
        };
    }
    for (marker, mut node) in &mut panes {
        let width = match marker.kind {
            Lai67PaneKind::ResearchCatalog => layout.catalog_width_percent,
            Lai67PaneKind::ResearchGraph => layout.graph_width_percent,
            Lai67PaneKind::ResearchInspector => layout.inspector_width_percent,
            Lai67PaneKind::CouncilIndex => {
                if layout.mode == LayoutMode::Wide {
                    39.0
                } else {
                    100.0
                }
            }
            Lai67PaneKind::CouncilDetail => {
                if layout.mode == LayoutMode::Wide {
                    61.0
                } else {
                    100.0
                }
            }
        };
        node.width = Val::Percent(width);
        node.height = if layout.mode == LayoutMode::Wide {
            Val::Percent(100.0)
        } else {
            Val::Auto
        };
        node.min_height = Val::Px(f32::from(layout.minimum_pane_height_px));
    }
}

fn handle_lai67_scroll(
    mut wheel: MessageReader<'_, '_, MouseWheel>,
    mut panes: Query<
        '_,
        '_,
        (&Interaction, &Node, &ComputedNode, &mut ScrollPosition),
        With<Lai67ScrollablePane>,
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

fn surface_state_copy(state: &Lai67SurfaceState) -> String {
    match state {
        Lai67SurfaceState::Loading => "Loading the report-safe colony snapshot.".to_owned(),
        Lai67SurfaceState::Ready => "Current report loaded.".to_owned(),
        Lai67SurfaceState::Empty => "No report-visible entries are available.".to_owned(),
        Lai67SurfaceState::Stale { stale_since_ms } => format!(
            "Report is stale since {stale_since_ms}; values remain the last received report."
        ),
        Lai67SurfaceState::Conflict { reason } => {
            format!(
                "Action conflict: {reason}. The visible report remains unchanged until refresh."
            )
        }
        Lai67SurfaceState::UpdateRequired => {
            "Client update required before this report can refresh.".to_owned()
        }
        Lai67SurfaceState::Error { message } => format!("Report unavailable: {message}"),
    }
}

fn screen_label(screen: PrimaryScreen) -> &'static str {
    match screen {
        PrimaryScreen::Research => "Research",
        PrimaryScreen::Council => "Council",
        PrimaryScreen::Log => "Log",
        PrimaryScreen::Stores => "Stores",
        PrimaryScreen::Village => "Village",
    }
}

fn stable_id(value: &str) -> cat_protocol::lai64::StableId {
    cat_protocol::lai64::StableId::new(value).expect("canonical report IDs were validated")
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
    format!("lai67:{section}:{slug}")
}

/// Explicit allow-list used by UI reviews and tests.  Adding a protocol action
/// does not automatically make it a LAI.67 control; it must be deliberately
/// added here with a report-safe affordance and a clear God authority reason.
#[must_use]
pub const fn is_lai67_allowed_action(action: &CanonicalGodAction) -> bool {
    matches!(
        action,
        CanonicalGodAction::ResearchQueue { .. }
            | CanonicalGodAction::ResearchReorder { .. }
            | CanonicalGodAction::ResearchFund { .. }
            | CanonicalGodAction::ResearchRemove { .. }
            | CanonicalGodAction::ResearchPreparation { .. }
            | CanonicalGodAction::FoodConservation { .. }
            | CanonicalGodAction::HoleClickBatch { .. }
            | CanonicalGodAction::Inspiration
            | CanonicalGodAction::ActivateBoost { .. }
            | CanonicalGodAction::ConstructionMiracle { .. }
            | CanonicalGodAction::EmergencyRescue { .. }
            | CanonicalGodAction::CandidateBacking { .. }
            | CanonicalGodAction::PersonalStance { .. }
            | CanonicalGodAction::Expel { .. }
            | CanonicalGodAction::BroadDomainNudge { .. }
            | CanonicalGodAction::SignedTestReset { .. }
    )
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

fn fallback_join(values: &[String]) -> String {
    if values.is_empty() {
        "none reported".to_owned()
    } else {
        values.join(", ")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lai67VisualDirection {
    pub product_normal: bool,
    pub parchment_content: bool,
    pub wood_rules: bool,
    pub dark_forest_worktable: bool,
    pub uses_glass: bool,
    pub uses_glow: bool,
    pub uses_kpi_grid: bool,
    pub uses_excessive_pills: bool,
}

pub const LAI67_VISUAL_DIRECTION: Lai67VisualDirection = Lai67VisualDirection {
    product_normal: true,
    parchment_content: true,
    wood_rules: true,
    dark_forest_worktable: true,
    uses_glass: false,
    uses_glow: false,
    uses_kpi_grid: false,
    uses_excessive_pills: false,
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
