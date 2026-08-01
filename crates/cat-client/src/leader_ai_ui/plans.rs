//! Pure render and action models for the LAI.28 Plans UI.
//!
//! The server snapshot remains authoritative. This leaf only formats visible
//! LAI.24 report-safe DTOs and constructs strict LAI.25 action envelopes.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use cat_protocol::{
    ActionConflict, ActionDecodeError, ActionIdempotencyId, ActionProtocolVersion,
    AuthenticatedPlayerId, BoundedBasisPointNudge, BoundedBasisPoints, BoundedEntityId,
    BoundedStandingOrderText, ColonyAiSnapshot, DismissalReason, ExpectedStateVersions,
    LeaderAiActionEnvelope, LeaderAiActionPayload, LeaderAiActionResponse, LeaderAiActionResult,
    NonEmptyStableId, OfficerRequestSnapshot, PlanSnapshot, ReportEstimateSnapshot,
    ReportSafeString, SelectedColonyId, SiteRefSnapshot, SnapshotDecodeError, SnapshotTilePoint,
    StaleClientRefresh, StandingOrderPatch, VisibleTaskSnapshot,
};

use super::{
    AccessibleLabel, ControlKind, EntityKind, FeedbackState, StableUiId, TestIdBuilder, UiSection,
};

pub const PLAN_NUDGE_UP_DELTA_BP_1500: i16 = 1_500;
pub const PLAN_NUDGE_DOWN_DELTA_BP_NEG_1500: i16 = -1_500;
pub const ACCESSIBLE_PLANS_PANEL_LABEL: &str = "Plans panel";
pub const ACCESSIBLE_STANDING_ORDERS_PANEL_LABEL: &str = "Standing orders panel";
pub const PLAN_ROW_TEST_ID_PREFIX: &str = "lai-ui:plans:plan:";
pub const STANDING_ORDER_ROW_TEST_ID_PREFIX: &str = "lai-ui:plans:standing-order:";
pub const PLAN_CONTROL_TEST_ID_PREFIX: &str = "lai-ui:plans:control:";
pub const OFFICER_REPORT_TEST_ID_PREFIX: &str = "lai-ui:plans:officer-report:";
pub const VISIBLE_BROWSER_CHECKPOINT_PLANS_TOP_EIGHT: &str = "lai28-plans-top-eight";
pub const PLAYWRIGHT_NO_DOM_STATE_INJECTION: &str = "no-dom-state-injection";

#[derive(Default)]
pub struct PlansPanelPlugin;

impl Plugin for PlansPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlansPanelState>()
            .init_resource::<PlansPanelInput>()
            .init_resource::<PlansPanelProjectionResource>()
            .add_systems(Update, update_plans_panel_projection);
    }
}

#[derive(Resource, Default, Clone, Debug, PartialEq, Eq)]
pub struct PlansPanelState {
    pub selected_plan_id: Option<String>,
    pub draft: Option<StandingOrderDraft>,
    pub refresh_state: PlansRefreshState,
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct PlansPanelRoot;

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct PlansPanelInput {
    pub selected_colony_id: Option<String>,
    pub colony: Option<ColonyAiSnapshot>,
    pub standing_orders: StandingOrdersPanel,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct PlansPanelProjectionResource {
    pub projection: Option<PlansPanelProjection>,
}

pub fn update_plans_panel_projection(
    input: Res<'_, PlansPanelInput>,
    state: Res<'_, PlansPanelState>,
    mut output: ResMut<'_, PlansPanelProjectionResource>,
) {
    let Some(colony) = input.colony.as_ref() else {
        output.projection = None;
        return;
    };
    let selected_colony_id = input
        .selected_colony_id
        .as_deref()
        .unwrap_or_else(|| colony.colony_id.as_str());
    if colony.colony_id.as_str() != selected_colony_id || !colony.capabilities.can_view {
        output.projection = None;
        return;
    }
    output.projection = Some(render_authoritative_top_eight_plans(
        colony,
        input.standing_orders.clone(),
        state.refresh_state,
    ));
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlansRefreshState {
    #[default]
    Current,
    Loading,
    Stale,
    UpdateRequired,
    Error,
}

impl PlansRefreshState {
    pub const fn feedback(self) -> FeedbackState {
        match self {
            Self::Current => FeedbackState::Empty,
            Self::Loading => FeedbackState::Loading,
            Self::Stale => FeedbackState::Stale,
            Self::UpdateRequired => FeedbackState::UpdateRequired,
            Self::Error => FeedbackState::Error,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlansPanelProjection {
    pub planner_version: u64,
    pub rows: Vec<PlanRowRenderModel>,
    pub officer_reports: Vec<OfficerReportPanel>,
    pub standing_orders: StandingOrdersPanel,
    pub refresh_state: PlansRefreshState,
    pub layout: PlansPanelLayoutSpec,
    pub chrome: PlansPanelChrome,
}

pub fn render_authoritative_top_eight_plans(
    colony: &ColonyAiSnapshot,
    standing_orders: StandingOrdersPanel,
    refresh_state: PlansRefreshState,
) -> PlansPanelProjection {
    let reports_by_id = colony
        .reports
        .iter()
        .map(|report| (report.report_id.as_str(), report))
        .collect::<BTreeMap<_, _>>();
    let tasks_by_intent = colony
        .visible_tasks
        .iter()
        .map(|task| (task.intent_id.as_str(), task))
        .collect::<BTreeMap<_, _>>();

    let rows = colony
        .plans
        .plans
        .iter()
        .take(8)
        .enumerate()
        .map(|(index, plan)| {
            PlanRowRenderModel::from_snapshot(
                index,
                plan,
                &reports_by_id,
                tasks_by_intent.get(plan.intent_id.as_str()).copied(),
            )
        })
        .collect();
    let officer_reports = colony
        .officer_requests
        .iter()
        .filter(|request| request.merged_into_request_id.is_none())
        .map(OfficerReportPanel::from_snapshot)
        .collect();

    PlansPanelProjection {
        planner_version: colony.plans.planner_version,
        rows,
        officer_reports,
        standing_orders,
        refresh_state,
        layout: PlansPanelLayoutSpec::default(),
        chrome: PlansPanelChrome::default(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanRowRenderModel {
    pub stable_id: PlanRowStableId,
    pub test_id: StableUiId,
    pub index: usize,
    pub lifecycle: PlanLifecycleStatusLabel,
    pub responsible_actor: PlanResponsibleActorLabel,
    pub dependencies: PlanDependencyList,
    pub rationale: PlanBoundedRationale,
    pub score_confidence_range: PlanScoreConfidenceRange,
    pub cost: PlanCostLabel,
    pub report_age: PlanReportAgeBadge,
    pub provenance: PlanReportProvenanceList,
    pub uncertainty: PlanUncertaintyCopy,
    pub urgency: PlanUrgency,
    pub objective: Option<PlanObjectiveSiteSummary>,
    pub assigned_cat_ids: Vec<String>,
    pub stage: Option<String>,
    pub progress_basis_points: Option<u16>,
    pub block_reason: Option<PlanBlockReason>,
    pub unavailable_states: Vec<ReportSafeUnavailableState>,
    pub controls: PlanControlsRenderModel,
}

impl PlanRowRenderModel {
    fn from_snapshot(
        index: usize,
        plan: &PlanSnapshot,
        reports_by_id: &BTreeMap<&str, &cat_protocol::BeliefReportSnapshot>,
        task: Option<&VisibleTaskSnapshot>,
    ) -> Self {
        let report_ids = plan
            .reasons
            .iter()
            .flat_map(|reason| reason.source_report_ids.iter())
            .collect::<BTreeSet<_>>();
        let source_reports = report_ids
            .iter()
            .filter_map(|report_id| reports_by_id.get(report_id.as_str()).copied())
            .collect::<Vec<_>>();
        let confidence_values = plan
            .reasons
            .iter()
            .map(|reason| reason.confidence_basis_points.get())
            .collect::<Vec<_>>();
        let objective = task.map(PlanObjectiveSiteSummary::from_task);
        let (assigned_cat_ids, stage, progress_basis_points) = task
            .map(|task| {
                (
                    task.assigned_cat_ids
                        .iter()
                        .map(|cat_id| cat_id.as_str().to_string())
                        .collect(),
                    Some(task.stage.as_str().to_string()),
                    Some(task.progress_basis_points.get()),
                )
            })
            .unwrap_or_else(|| (Vec::new(), None, None));
        Self {
            stable_id: PlanRowStableId(plan.plan_id.as_str().to_string()),
            test_id: TestIdBuilder::row(UiSection::Plans, EntityKind::Plan, plan.plan_id.as_str()),
            index,
            lifecycle: PlanLifecycleStatusLabel(plan.lifecycle_state.as_str().to_string()),
            responsible_actor: PlanResponsibleActorLabel {
                actor_id: plan.responsible_actor_id.as_str().to_string(),
                office: plan
                    .responsible_office
                    .as_ref()
                    .map(|office| office.as_str().to_string()),
            },
            dependencies: PlanDependencyList(
                plan.dependency_intent_ids
                    .iter()
                    .map(|dependency| dependency.as_str().to_string())
                    .collect(),
            ),
            rationale: PlanBoundedRationale(plan.rationale.as_str().to_string()),
            score_confidence_range: PlanScoreConfidenceRange {
                score_bucket: plan.score_bucket,
                confidence_min_basis_points: confidence_values.iter().min().copied(),
                confidence_max_basis_points: confidence_values.iter().max().copied(),
                expected_cost: EstimateRange::from_snapshot(&plan.expected_cost),
                expected_benefit: EstimateRange::from_snapshot(&plan.expected_benefit),
            },
            cost: PlanCostLabel(EstimateRange::from_snapshot(&plan.expected_cost)),
            report_age: PlanReportAgeBadge {
                oldest_age_ms: source_reports
                    .iter()
                    .map(|report| report.age_ms.get())
                    .max(),
                newest_age_ms: source_reports
                    .iter()
                    .map(|report| report.age_ms.get())
                    .min(),
            },
            provenance: PlanReportProvenanceList {
                source_report_ids: report_ids
                    .iter()
                    .map(|report_id| report_id.as_str().to_string())
                    .collect(),
            },
            uncertainty: PlanUncertaintyCopy::from_reports(&source_reports, &confidence_values),
            urgency: PlanUrgency::from_score(plan.score_bucket),
            objective,
            assigned_cat_ids,
            stage,
            progress_basis_points,
            block_reason: task
                .and_then(|task| task.blocked_reason.as_ref())
                .map(|reason| PlanBlockReason(reason.as_str().to_string())),
            unavailable_states: source_reports
                .iter()
                .map(|report| {
                    ReportSafeUnavailableState::from_report(
                        report.report_level,
                        &report.regeneration,
                    )
                })
                .collect(),
            controls: PlanControlsRenderModel::enabled_for(plan),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanRowStableId(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanLifecycleStatusLabel(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanResponsibleActorLabel {
    pub actor_id: String,
    pub office: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanDependencyList(pub Vec<String>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanBoundedRationale(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanScoreConfidenceRange {
    pub score_bucket: i16,
    pub confidence_min_basis_points: Option<u16>,
    pub confidence_max_basis_points: Option<u16>,
    pub expected_cost: EstimateRange,
    pub expected_benefit: EstimateRange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanCostLabel(pub EstimateRange);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanBlockReason(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanUncertaintyCopy(pub String);

impl PlanUncertaintyCopy {
    fn from_reports(
        reports: &[&cat_protocol::BeliefReportSnapshot],
        confidence_values: &[u16],
    ) -> Self {
        if reports.iter().any(|report| {
            matches!(
                report.regeneration,
                cat_protocol::RegenerationReportSnapshot::UnavailableBelowLevel4
            ) && report.report_level < 4
        }) {
            return Self("regeneration estimate unavailable until report level 4".to_string());
        }
        match (
            confidence_values.iter().min(),
            confidence_values.iter().max(),
        ) {
            (Some(min), Some(max)) => {
                Self(format!("reported confidence {}-{} basis points", min, max))
            }
            _ => Self("no source report confidence available".to_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EstimateRange {
    pub minimum: i64,
    pub maximum: i64,
    pub unit: String,
}

impl EstimateRange {
    fn from_snapshot(value: &ReportEstimateSnapshot) -> Self {
        Self {
            minimum: value.minimum,
            maximum: value.maximum,
            unit: value.unit.as_str().to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanReportAgeBadge {
    pub oldest_age_ms: Option<u64>,
    pub newest_age_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanReportProvenanceList {
    pub source_report_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanUrgency {
    Emergency,
    High,
    Normal,
    Low,
}

impl PlanUrgency {
    const fn from_score(score_bucket: i16) -> Self {
        match score_bucket {
            20..=i16::MAX => Self::Emergency,
            10..=19 => Self::High,
            0..=9 => Self::Normal,
            _ => Self::Low,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanObjectiveSiteSummary {
    pub task_id: String,
    pub category: String,
    pub stage: String,
    pub site_id: String,
    pub site_kind: String,
    pub representative_tile: Option<SnapshotTilePoint>,
}

impl PlanObjectiveSiteSummary {
    fn from_task(task: &VisibleTaskSnapshot) -> Self {
        Self {
            task_id: task.task_id.as_str().to_string(),
            category: task.category.as_str().to_string(),
            stage: task.stage.as_str().to_string(),
            site_id: site_ref_id(&task.objective),
            site_kind: site_ref_kind(&task.objective).to_string(),
            representative_tile: representative_tile(&task.objective),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReportSafeUnavailableState {
    RegenerationUnavailableBelowReportLevel4,
    RegenerationEstimated {
        minimum: i64,
        maximum: i64,
        unit: String,
        provenance_count: usize,
    },
    ExplicitlyUnavailable(String),
}

impl ReportSafeUnavailableState {
    fn from_report(level: u8, regeneration: &cat_protocol::RegenerationReportSnapshot) -> Self {
        match regeneration {
            cat_protocol::RegenerationReportSnapshot::UnavailableBelowLevel4 if level < 4 => {
                Self::RegenerationUnavailableBelowReportLevel4
            }
            cat_protocol::RegenerationReportSnapshot::UnavailableBelowLevel4 => {
                Self::ExplicitlyUnavailable("regeneration unavailable".to_string())
            }
            cat_protocol::RegenerationReportSnapshot::Estimated {
                estimate,
                provenance,
                ..
            } => Self::RegenerationEstimated {
                minimum: estimate.minimum,
                maximum: estimate.maximum,
                unit: estimate.unit.as_str().to_string(),
                provenance_count: provenance.source_report_ids.len(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanControlsRenderModel {
    pub move_up: MovePlanUpButton,
    pub move_down: MovePlanDownButton,
    pub dismiss: DismissPlanButton,
    pub domain_nudge: DomainNudgeControl,
}

impl PlanControlsRenderModel {
    fn enabled_for(plan: &PlanSnapshot) -> Self {
        let subject = plan.rationale.as_str();
        Self {
            move_up: MovePlanUpButton::new(plan.plan_id.as_str(), subject),
            move_down: MovePlanDownButton::new(plan.plan_id.as_str(), subject),
            dismiss: DismissPlanButton::new(plan.intent_id.as_str(), subject),
            domain_nudge: DomainNudgeControl {
                domain: plan
                    .responsible_office
                    .as_ref()
                    .map(|office| office.as_str().to_string())
                    .unwrap_or_else(|| "leader".to_string()),
                current_epoch_only: CurrentPlanningEpochOnly,
                disabled_reason: None,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MovePlanUpButton {
    pub test_id: StableUiId,
    pub label: AccessibleLabel,
    pub delta_basis_points: i16,
    pub disabled_reason: Option<PlanControlDisabledReason>,
}

impl MovePlanUpButton {
    fn new(plan_id: &str, subject: &str) -> Self {
        Self {
            test_id: TestIdBuilder::control(UiSection::Plans, ControlKind::MoveUp, plan_id),
            label: accessibility_label_move_plan_up(subject),
            delta_basis_points: PLAN_NUDGE_UP_DELTA_BP_1500,
            disabled_reason: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MovePlanDownButton {
    pub test_id: StableUiId,
    pub label: AccessibleLabel,
    pub delta_basis_points: i16,
    pub disabled_reason: Option<PlanControlDisabledReason>,
}

impl MovePlanDownButton {
    fn new(plan_id: &str, subject: &str) -> Self {
        Self {
            test_id: TestIdBuilder::control(UiSection::Plans, ControlKind::MoveDown, plan_id),
            label: accessibility_label_move_plan_down(subject),
            delta_basis_points: PLAN_NUDGE_DOWN_DELTA_BP_NEG_1500,
            disabled_reason: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DismissPlanButton {
    pub test_id: StableUiId,
    pub label: AccessibleLabel,
    pub disabled_reason: Option<PlanControlDisabledReason>,
}

impl DismissPlanButton {
    fn new(intent_id: &str, subject: &str) -> Self {
        Self {
            test_id: TestIdBuilder::control(UiSection::Plans, ControlKind::Dismiss, intent_id),
            label: accessibility_label_dismiss_plan(subject),
            disabled_reason: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainNudgeControl {
    pub domain: String,
    pub current_epoch_only: CurrentPlanningEpochOnly,
    pub disabled_reason: Option<PlanControlDisabledReason>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CurrentPlanningEpochOnly;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanControlDisabledReason {
    Stale,
    UpdateRequired,
    EmergencyLocked,
    RemovedPlan,
    Unauthorized,
    StandingOrderSlotLimitReached,
    MalformedInput,
}

pub fn accessibility_label_move_plan_up(subject: &str) -> AccessibleLabel {
    AccessibleLabel::control(ControlKind::MoveUp, subject)
}

pub fn accessibility_label_move_plan_down(subject: &str) -> AccessibleLabel {
    AccessibleLabel::control(ControlKind::MoveDown, subject)
}

pub fn accessibility_label_dismiss_plan(subject: &str) -> AccessibleLabel {
    AccessibleLabel::control(ControlKind::Dismiss, subject)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StandingOrdersPanel {
    pub slot_meter: AdministrationSlotMeter,
    pub draft: Option<StandingOrderDraft>,
    pub feedback: Option<StandingOrderBoundedFeedback>,
}

impl Default for StandingOrdersPanel {
    fn default() -> Self {
        Self::empty(3, 0)
    }
}

impl StandingOrdersPanel {
    pub const fn empty(slot_limit: u8, slot_used: u8) -> Self {
        Self {
            slot_meter: AdministrationSlotMeter::new(slot_limit, slot_used),
            draft: None,
            feedback: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdministrationSlotMeter {
    pub slot_limit: u8,
    pub slot_used: u8,
    pub vacant: u8,
    pub limit_reached: bool,
}

impl AdministrationSlotMeter {
    pub const fn new(slot_limit: u8, slot_used: u8) -> Self {
        let vacant = slot_limit.saturating_sub(slot_used);
        Self {
            slot_limit,
            slot_used,
            vacant,
            limit_reached: slot_used >= slot_limit,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdministrationSlotLimitReached;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StandingOrderDraft {
    pub order_kind: String,
    pub domain: String,
    pub target_id: Option<String>,
    pub instruction: String,
    pub priority_basis_points: u16,
    pub expires_at_ms: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StandingOrderCreateButton;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StandingOrderEditButton;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StandingOrderRemoveButton;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StandingOrderPolicyDomainPicker {
    pub selected_domain: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StandingOrderBoundedFeedback {
    pub state: FeedbackState,
    pub message: String,
    pub blocks_mutation: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StandingOrderDoesNotBypassKnowledgeOrPhysicalRules;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfficerReportPanel {
    pub request_id: String,
    pub test_id: OfficerReportTestId,
    pub office: String,
    pub domain: String,
    pub requested_action: String,
    pub budget: EstimateRange,
    pub priority_basis_points: u16,
    pub source_report_ids: Vec<String>,
    pub expires_at_ms: i64,
    pub blocked_reason: Option<String>,
    pub vacancy: OfficerVacancySlot,
    pub authority: OfficerAuthorityBadge,
    pub reasons: OfficerRequestReasonList,
}

impl OfficerReportPanel {
    fn from_snapshot(request: &OfficerRequestSnapshot) -> Self {
        Self {
            request_id: request.request_id.as_str().to_string(),
            test_id: OfficerReportTestId(format!(
                "{OFFICER_REPORT_TEST_ID_PREFIX}{}",
                request.request_id.as_str()
            )),
            office: request.office.as_str().to_string(),
            domain: request.domain.as_str().to_string(),
            requested_action: request.requested_action.as_str().to_string(),
            budget: EstimateRange::from_snapshot(&request.budget),
            priority_basis_points: request.priority_basis_points.get(),
            source_report_ids: request
                .source_report_ids
                .iter()
                .map(|source| source.as_str().to_string())
                .collect(),
            expires_at_ms: request.expires_at_ms,
            blocked_reason: request
                .blocked_reason
                .as_ref()
                .map(|reason| reason.as_str().to_string()),
            vacancy: OfficerVacancySlot {
                office: request.office.as_str().to_string(),
                vacant: request.blocked_reason.is_none(),
            },
            authority: OfficerAuthorityBadge {
                scope: request.domain.as_str().to_string(),
            },
            reasons: OfficerRequestReasonList(
                request
                    .source_report_ids
                    .iter()
                    .map(|source| source.as_str().to_string())
                    .collect(),
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfficerReportTestId(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfficerVacancySlot {
    pub office: String,
    pub vacant: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfficerAuthorityBadge {
    pub scope: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfficerRequestReasonList(pub Vec<String>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeaderResponsibleActorBadge;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectiveReportLevelGate;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegenerationUnavailableBelowReportLevel4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoClientRegenerationFallback;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlansNoHiddenTruthGuard;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlansPanelLayoutSpec {
    pub rows_limit: usize,
    pub row_height_px: u16,
    pub panel_radius_px: u16,
    pub compact_width_px: u16,
    pub world_first: bool,
}

impl Default for PlansPanelLayoutSpec {
    fn default() -> Self {
        Self {
            rows_limit: 8,
            row_height_px: 48,
            panel_radius_px: 10,
            compact_width_px: 820,
            world_first: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlansPanelChrome {
    pub paper_role: super::RoleColor,
    pub border_role: super::RoleColor,
    pub action_role: super::RoleColor,
    pub selected_role: super::RoleColor,
    pub danger_role: super::RoleColor,
}

impl Default for PlansPanelChrome {
    fn default() -> Self {
        Self {
            paper_role: super::RoleColor::Paper,
            border_role: super::RoleColor::Wood,
            action_role: super::RoleColor::Rust,
            selected_role: super::RoleColor::Olive,
            danger_role: super::RoleColor::Danger,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeterministicPlanRowOrder;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EqualNudgesDoNotStack;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OppositeNudgeReplacesPrior;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StablePlanTieBreakKey;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoStalePlanControlReuse;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DuplicateReplayUsesOriginalResult;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthenticatedPlayerIdentity {
    pub colony_id: String,
    pub player_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StableIdempotencyId(pub String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpectedPlannerVersion(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpectedDomainVersion(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpectedResourceVersion(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpectedReservationVersion(pub Option<u64>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpectedVersionBundle {
    pub planner: ExpectedPlannerVersion,
    pub domain: ExpectedDomainVersion,
    pub resource: ExpectedResourceVersion,
    pub reservation: ExpectedReservationVersion,
    pub standing_order: Option<u64>,
}

impl ExpectedVersionBundle {
    fn into_protocol(self) -> ExpectedStateVersions {
        ExpectedStateVersions {
            expected_planner_version: self.planner.0,
            expected_domain_version: self.domain.0,
            expected_resource_version: self.resource.0,
            expected_spatial_version: None,
            expected_reservation_version: self.reservation.0,
            expected_research_version: None,
            expected_scholar_version: None,
            expected_boost_version: None,
            expected_diplomacy_version: None,
            expected_trade_version: None,
            expected_prosthetic_version: None,
            expected_care_version: None,
            expected_officer_version: None,
            expected_standing_order_version: self.standing_order,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeaderAiPlanNudgeAction {
    MoveUp {
        plan_id: String,
        reason_key: Option<String>,
    },
    MoveDown {
        plan_id: String,
        reason_key: Option<String>,
    },
    Dismiss {
        intent_id: String,
        planning_epoch: u64,
        reason: DismissalReason,
    },
}

impl LeaderAiPlanNudgeAction {
    fn into_payload(self) -> Result<LeaderAiActionPayload, PlanActionBuildError> {
        match self {
            Self::MoveUp {
                plan_id,
                reason_key,
            } => Ok(LeaderAiActionPayload::NudgePlan {
                plan_id: entity_id(&plan_id)?,
                nudge: BoundedBasisPointNudge::new(PLAN_NUDGE_UP_DELTA_BP_1500)?,
                reason_key: optional_entity_id(reason_key)?,
            }),
            Self::MoveDown {
                plan_id,
                reason_key,
            } => Ok(LeaderAiActionPayload::NudgePlan {
                plan_id: entity_id(&plan_id)?,
                nudge: BoundedBasisPointNudge::new(PLAN_NUDGE_DOWN_DELTA_BP_NEG_1500)?,
                reason_key: optional_entity_id(reason_key)?,
            }),
            Self::Dismiss {
                intent_id,
                planning_epoch,
                reason,
            } => Ok(LeaderAiActionPayload::DismissIntent {
                intent_id: entity_id(&intent_id)?,
                planning_epoch,
                reason,
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeaderAiStandingOrderAction {
    Create(StandingOrderDraft),
    Update {
        standing_order_id: String,
        patch: StandingOrderDraftPatch,
    },
    Delete {
        standing_order_id: String,
    },
}

impl LeaderAiStandingOrderAction {
    fn into_payload(self) -> Result<LeaderAiActionPayload, PlanActionBuildError> {
        match self {
            Self::Create(draft) => Ok(LeaderAiActionPayload::CreateStandingOrder {
                order_kind: entity_id(&draft.order_kind)?,
                domain: entity_id(&draft.domain)?,
                target_id: optional_entity_id(draft.target_id)?,
                instruction: BoundedStandingOrderText::new(draft.instruction)?,
                priority_basis_points: BoundedBasisPoints::new(draft.priority_basis_points)?,
                expires_at_ms: draft.expires_at_ms,
            }),
            Self::Update {
                standing_order_id,
                patch,
            } => Ok(LeaderAiActionPayload::UpdateStandingOrder {
                standing_order_id: entity_id(&standing_order_id)?,
                patch: patch.into_protocol()?,
            }),
            Self::Delete { standing_order_id } => Ok(LeaderAiActionPayload::DeleteStandingOrder {
                standing_order_id: entity_id(&standing_order_id)?,
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StandingOrderDraftPatch {
    pub instruction: Option<String>,
    pub priority_basis_points: Option<u16>,
    pub target_id: Option<String>,
    pub clear_target: bool,
    pub expires_at_ms: Option<i64>,
    pub clear_expiry: bool,
}

impl StandingOrderDraftPatch {
    fn into_protocol(self) -> Result<StandingOrderPatch, PlanActionBuildError> {
        Ok(StandingOrderPatch {
            instruction: self
                .instruction
                .map(BoundedStandingOrderText::new)
                .transpose()?,
            priority_basis_points: self
                .priority_basis_points
                .map(BoundedBasisPoints::new)
                .transpose()?,
            target_id: optional_entity_id(self.target_id)?,
            clear_target: self.clear_target,
            expires_at_ms: self.expires_at_ms,
            clear_expiry: self.clear_expiry,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanActionBuildError {
    Action(ActionDecodeError),
    Snapshot(SnapshotDecodeError),
}

impl From<ActionDecodeError> for PlanActionBuildError {
    fn from(value: ActionDecodeError) -> Self {
        Self::Action(value)
    }
}

impl From<SnapshotDecodeError> for PlanActionBuildError {
    fn from(value: SnapshotDecodeError) -> Self {
        Self::Snapshot(value)
    }
}

pub fn send_expected_version_action(
    identity: AuthenticatedPlayerIdentity,
    idempotency: StableIdempotencyId,
    expected_versions: ExpectedVersionBundle,
    action: LeaderAiPlanNudgeAction,
) -> Result<LeaderAiActionEnvelope, PlanActionBuildError> {
    build_leader_ai_action_envelope(
        identity,
        idempotency,
        expected_versions,
        action.into_payload()?,
    )
}

pub fn build_leader_ai_action_envelope(
    identity: AuthenticatedPlayerIdentity,
    idempotency: StableIdempotencyId,
    expected_versions: ExpectedVersionBundle,
    payload: LeaderAiActionPayload,
) -> Result<LeaderAiActionEnvelope, PlanActionBuildError> {
    Ok(LeaderAiActionEnvelope {
        protocol_version: ActionProtocolVersion::current(),
        idempotency_id: ActionIdempotencyId::new(idempotency.0)?,
        colony_id: SelectedColonyId::new(identity.colony_id)?,
        player_id: AuthenticatedPlayerId::new(identity.player_id)?,
        expected_versions: expected_versions.into_protocol(),
        payload,
    })
}

pub fn build_standing_order_action_envelope(
    identity: AuthenticatedPlayerIdentity,
    idempotency: StableIdempotencyId,
    expected_versions: ExpectedVersionBundle,
    action: LeaderAiStandingOrderAction,
) -> Result<LeaderAiActionEnvelope, PlanActionBuildError> {
    build_leader_ai_action_envelope(
        identity,
        idempotency,
        expected_versions,
        action.into_payload()?,
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanActionConflictRefresh {
    pub refresh_state: PlansRefreshState,
    pub focus: PreservePlansPanelFocusAfterRefresh,
    pub draft: PreserveStandingOrderDraftAfterRefresh,
    pub feedback: BoundedPlanConflictToast,
}

pub struct VersionMismatchRefreshHandler;

impl VersionMismatchRefreshHandler {
    pub fn handle(
        response: &LeaderAiActionResponse,
        selected_plan_id: Option<&str>,
        draft: Option<StandingOrderDraft>,
        visible_plan_ids: &[String],
    ) -> Option<PlanActionConflictRefresh> {
        let conflict = match &response.result {
            LeaderAiActionResult::Rejected { conflict } => conflict,
            LeaderAiActionResult::DuplicateReplay { replay } => {
                return Some(PlanActionConflictRefresh {
                    refresh_state: PlansRefreshState::Stale,
                    focus: PreservePlansPanelFocusAfterRefresh::preserve(
                        selected_plan_id,
                        visible_plan_ids,
                    ),
                    draft: PreserveStandingOrderDraftAfterRefresh(draft),
                    feedback: BoundedPlanConflictToast::from_report_safe(
                        replay.result_code.as_str(),
                    ),
                });
            }
            LeaderAiActionResult::Accepted { .. } => return None,
        };
        let refresh_state = match conflict {
            ActionConflict::UpdateRequired { .. } => PlansRefreshState::UpdateRequired,
            ActionConflict::VersionMismatch { .. } => PlansRefreshState::Stale,
            _ => PlansRefreshState::Error,
        };
        Some(PlanActionConflictRefresh {
            refresh_state,
            focus: PreservePlansPanelFocusAfterRefresh::preserve(
                selected_plan_id,
                visible_plan_ids,
            ),
            draft: PreserveStandingOrderDraftAfterRefresh(draft),
            feedback: BoundedPlanConflictToast::from_conflict(conflict, response.refresh.as_ref()),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreservePlansPanelFocusAfterRefresh(pub Option<String>);

impl PreservePlansPanelFocusAfterRefresh {
    fn preserve(selected_plan_id: Option<&str>, visible_plan_ids: &[String]) -> Self {
        Self(selected_plan_id.and_then(|plan_id| {
            visible_plan_ids
                .iter()
                .any(|visible| visible == plan_id)
                .then(|| plan_id.to_string())
        }))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreserveStandingOrderDraftAfterRefresh(pub Option<StandingOrderDraft>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DespawnsUnknownPlanRows {
    pub removed_plan_ids: Vec<String>,
}

impl DespawnsUnknownPlanRows {
    pub fn between(previous_plan_ids: &[String], current_plan_ids: &[String]) -> Self {
        let current = current_plan_ids.iter().collect::<BTreeSet<_>>();
        Self {
            removed_plan_ids: previous_plan_ids
                .iter()
                .filter(|plan_id| !current.contains(plan_id))
                .cloned()
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemovedPlanControlsAreDisabled(pub Vec<String>);

impl From<DespawnsUnknownPlanRows> for RemovedPlanControlsAreDisabled {
    fn from(value: DespawnsUnknownPlanRows) -> Self {
        Self(value.removed_plan_ids)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundedPlanConflictToast {
    pub message: String,
}

impl BoundedPlanConflictToast {
    fn from_report_safe(message: &str) -> Self {
        Self {
            message: truncate_report_safe(message),
        }
    }

    fn from_conflict(conflict: &ActionConflict, refresh: Option<&StaleClientRefresh>) -> Self {
        let message = match conflict {
            ActionConflict::UpdateRequired { .. } => "UPDATE_REQUIRED".to_string(),
            ActionConflict::VersionMismatch {
                current_state_hint, ..
            } => current_state_hint.state_code.as_str().to_string(),
            ActionConflict::PreconditionFailed { reason } => reason.as_str().to_string(),
            ActionConflict::RateLimited { .. } => "rate limited".to_string(),
            ActionConflict::Unauthorized => "unauthorized".to_string(),
            ActionConflict::OwnershipDenied => "ownership denied".to_string(),
            ActionConflict::AuthorityDenied { .. } => "authority denied".to_string(),
            ActionConflict::DuplicateReplay { replay } => replay.result_code.as_str().to_string(),
            ActionConflict::InsufficientFavor { current_state_hint }
            | ActionConflict::ReservationConflict { current_state_hint } => {
                current_state_hint.state_code.as_str().to_string()
            }
            ActionConflict::MalformedActionId
            | ActionConflict::UnknownActionVariant
            | ActionConflict::MalformedPayload => "malformed action".to_string(),
            ActionConflict::LeaderCannotActivateBoost => "leader cannot activate boost".to_string(),
            ActionConflict::OfficerCannotActivateBoost => {
                "officer cannot activate boost".to_string()
            }
        };
        let message = refresh
            .map(|refresh| refresh.current_state_hint.state_code.as_str().to_string())
            .unwrap_or(message);
        Self::from_report_safe(&message)
    }
}

fn truncate_report_safe(value: &str) -> String {
    const MAX: usize = 120;
    if value.len() <= MAX {
        value.to_string()
    } else {
        value.chars().take(MAX).collect()
    }
}

fn entity_id(value: &str) -> Result<BoundedEntityId, ActionDecodeError> {
    BoundedEntityId::new(value)
}

fn optional_entity_id(value: Option<String>) -> Result<Option<BoundedEntityId>, ActionDecodeError> {
    value.map(BoundedEntityId::new).transpose()
}

fn site_ref_id(site_ref: &SiteRefSnapshot) -> String {
    match site_ref {
        SiteRefSnapshot::Tile { site, .. }
        | SiteRefSnapshot::AnchoredRect { site, .. }
        | SiteRefSnapshot::OrderedTileSet { site, .. }
        | SiteRefSnapshot::BuildingFootprint { site, .. }
        | SiteRefSnapshot::StockpileFootprint { site, .. }
        | SiteRefSnapshot::ResourceSource { site, .. }
        | SiteRefSnapshot::HuntSource { site, .. }
        | SiteRefSnapshot::WaterSourceAndBank { site, .. }
        | SiteRefSnapshot::OrderedRoute { site, .. }
        | SiteRefSnapshot::Shrine { site, .. }
        | SiteRefSnapshot::VillageEndpoint { site, .. }
        | SiteRefSnapshot::TradeEndpoint { site, .. } => site.site_id.as_str().to_string(),
    }
}

fn site_ref_kind(site_ref: &SiteRefSnapshot) -> &'static str {
    match site_ref {
        SiteRefSnapshot::Tile { .. } => "tile",
        SiteRefSnapshot::AnchoredRect { .. } => "anchored rect",
        SiteRefSnapshot::OrderedTileSet { .. } => "ordered tile set",
        SiteRefSnapshot::BuildingFootprint { .. } => "building footprint",
        SiteRefSnapshot::StockpileFootprint { .. } => "stockpile footprint",
        SiteRefSnapshot::ResourceSource { .. } => "resource source",
        SiteRefSnapshot::HuntSource { .. } => "hunt source",
        SiteRefSnapshot::WaterSourceAndBank { .. } => "water source and bank",
        SiteRefSnapshot::OrderedRoute { .. } => "ordered route",
        SiteRefSnapshot::Shrine { .. } => "shrine",
        SiteRefSnapshot::VillageEndpoint { .. } => "village endpoint",
        SiteRefSnapshot::TradeEndpoint { .. } => "trade endpoint",
    }
}

fn representative_tile(site_ref: &SiteRefSnapshot) -> Option<SnapshotTilePoint> {
    match site_ref {
        SiteRefSnapshot::Tile { tile, .. } => Some(*tile),
        SiteRefSnapshot::AnchoredRect { anchor, .. }
        | SiteRefSnapshot::BuildingFootprint { anchor, .. } => Some(*anchor),
        SiteRefSnapshot::OrderedTileSet { ordered_tiles, .. }
        | SiteRefSnapshot::StockpileFootprint { ordered_tiles, .. }
        | SiteRefSnapshot::ResourceSource { ordered_tiles, .. }
        | SiteRefSnapshot::OrderedRoute { ordered_tiles, .. } => ordered_tiles.first().copied(),
        SiteRefSnapshot::HuntSource { source_tile, .. } => Some(*source_tile),
        SiteRefSnapshot::WaterSourceAndBank { bank_tile, .. } => Some(*bank_tile),
        SiteRefSnapshot::Shrine { endpoint, .. }
        | SiteRefSnapshot::VillageEndpoint { endpoint, .. }
        | SiteRefSnapshot::TradeEndpoint { endpoint, .. } => Some(*endpoint),
    }
}

#[allow(dead_code)]
fn _ids_are_report_safe(_: &NonEmptyStableId, _: &ReportSafeString) {}
