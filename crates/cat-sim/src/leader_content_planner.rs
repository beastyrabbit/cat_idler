//! Pure LAI.45 Leader/officer content planner.
//!
//! This leaf implements the report-driven policy described by
//! `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md` section 9. It
//! deliberately emits commands for LAI.46 to validate and execute; it never
//! reads or mutates authoritative world state.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Serialize};

use crate::{
    content_manifest::{ContentId, PhysicalLotId},
    leader_planner::{EffectiveLevel, LeaderPosture, optional_omission_basis_points},
    officer_requests::{OfficerRequestBook, OfficerRequestPayload, RequestedSpaceKind},
    officers::OfficerRole,
    planner_core::{PlannerId, PlannerRngStream, planner_roll},
    quality_lots::QualityBand,
};

pub const CONTENT_PLANNER_SCHEMA_VERSION: u32 = 1;
pub const LIVE_GOAL_CAPACITY: usize = 128;
pub const TERMINAL_GOAL_CAPACITY: usize = 256;
pub const DEPENDENCY_CAPACITY: usize = 512;
pub const STANDING_ORDER_CAPACITY: usize = 32;
pub const REVIEW_RECEIPT_CAPACITY: usize = 256;
pub const OMISSION_HISTORY_CAPACITY: usize = 256;
pub const DIAGNOSTIC_CAPACITY: usize = 256;
pub const MAX_REPORT_CANDIDATES: usize = 128;
pub const MAX_FALLBACKS: usize = 8;
pub const MAX_COMMANDS_PER_REVIEW: usize = 128;
pub const MAX_DRAIN_BATCH: usize = 64;
pub const STALE_REPORT_GAME_MINUTES: u64 = 6 * 60;
pub const FOUNDING_FALLBACK_CONFIDENCE_BASIS_POINTS: u16 = 7_500;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerPhase {
    Observe,
    Reports,
    Posture,
    Score,
    OmitOrExpand,
    Sites,
    Reserve,
    Assign,
    Execute,
    ObserveOrRecover,
}

impl PlannerPhase {
    pub const ORDER: [Self; 10] = [
        Self::Observe,
        Self::Reports,
        Self::Posture,
        Self::Score,
        Self::OmitOrExpand,
        Self::Sites,
        Self::Reserve,
        Self::Assign,
        Self::Execute,
        Self::ObserveOrRecover,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentDomain {
    Defense,
    Survival,
    Hole,
    Hunting,
    Danger,
    Food,
    Apples,
    Fishing,
    FoodDays,
    Cookhouse,
    Research,
    ResearchNotes,
    VoidResearch,
    Processing,
    Tools,
    Fixtures,
    Augmentations,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocatedFoodRecoveryKind {
    AppleTree,
    FishShore,
    HuntingLair,
    FarmPlot,
    Cookhouse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportedSiteKind {
    HoleWorkArea,
    AppleTree,
    FishShore,
    HuntingLair,
    FarmPlot,
    Cookhouse,
    Workshop,
    ResearchStation,
    DefenseSite,
    Stockpile,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReportedSiteRef {
    pub site_id: PlannerId,
    pub kind: ReportedSiteKind,
    pub x: i32,
    pub y: i32,
    pub report_id: PlannerId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportedFoodKind {
    Apples,
    Fish,
    Meat,
    Meal,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReportedCargo {
    pub content_id: ContentId,
    pub food_kind: Option<ReportedFoodKind>,
    pub quality: QualityBand,
    pub believed_units: u32,
    pub believed_replacement_cost_milli: u64,
    pub lot_id: Option<PhysicalLotId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateKind {
    Defend,
    SelfPreservation,
    FeedHole,
    Hunt,
    RecoverFood(LocatedFoodRecoveryKind),
    SupplyCookhouse,
    StudyNotes,
    StudyVoid,
    ProcessMaterial,
    CraftTool,
    InstallFixture,
    InstallAugmentation,
    KeepStock,
}

impl CandidateKind {
    fn critical(self) -> bool {
        matches!(
            self,
            Self::Defend | Self::SelfPreservation | Self::RecoverFood(_)
        )
    }

    fn stable_name(self) -> &'static str {
        match self {
            Self::Defend => "defend",
            Self::SelfPreservation => "self_preservation",
            Self::FeedHole => "feed_hole",
            Self::Hunt => "hunt",
            Self::RecoverFood(LocatedFoodRecoveryKind::AppleTree) => "recover_apples",
            Self::RecoverFood(LocatedFoodRecoveryKind::FishShore) => "recover_fish",
            Self::RecoverFood(LocatedFoodRecoveryKind::HuntingLair) => "recover_hunt",
            Self::RecoverFood(LocatedFoodRecoveryKind::FarmPlot) => "recover_farm",
            Self::RecoverFood(LocatedFoodRecoveryKind::Cookhouse) => "recover_cookhouse",
            Self::SupplyCookhouse => "supply_cookhouse",
            Self::StudyNotes => "study_notes",
            Self::StudyVoid => "study_void",
            Self::ProcessMaterial => "process_material",
            Self::CraftTool => "craft_tool",
            Self::InstallFixture => "install_fixture",
            Self::InstallAugmentation => "install_augmentation",
            Self::KeepStock => "keep_stock",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReportedCandidate {
    pub id: PlannerId,
    pub domain: ContentDomain,
    pub kind: CandidateKind,
    pub target_id: PlannerId,
    pub site: Option<ReportedSiteRef>,
    pub cargo: Option<ReportedCargo>,
    pub urgency_basis_points: u16,
    pub confidence_basis_points: u16,
    pub expected_benefit_milli: u64,
    pub expected_labor_cost_milli: u64,
    /// Current-epoch God influence only. It is bounded to the scheduler's
    /// exact ±1,500 basis-point band and never identifies a worker or site.
    #[serde(default)]
    pub temporary_player_bias_basis_points: i16,
    pub report_tick: u64,
    pub evidence_ids: BTreeSet<PlannerId>,
    pub report_ids: BTreeSet<PlannerId>,
    pub ordered_fallbacks: Vec<PlannerId>,
    pub rationale_key: PlannerId,
}

impl ReportedCandidate {
    fn validate(&self) -> Result<(), ContentPlannerError> {
        if self.urgency_basis_points > 10_000
            || self.confidence_basis_points > 10_000
            || !(-1_500..=1_500).contains(&self.temporary_player_bias_basis_points)
            || self.evidence_ids.len() > MAX_FALLBACKS
            || self.report_ids.is_empty()
            || self.report_ids.len() > MAX_FALLBACKS
            || self.ordered_fallbacks.len() > MAX_FALLBACKS
            || self.ordered_fallbacks.iter().collect::<BTreeSet<_>>().len()
                != self.ordered_fallbacks.len()
            || self.ordered_fallbacks.contains(&self.id)
        {
            return Err(ContentPlannerError::MalformedReport);
        }
        if let CandidateKind::RecoverFood(kind) = self.kind {
            let expected = match kind {
                LocatedFoodRecoveryKind::AppleTree => ReportedSiteKind::AppleTree,
                LocatedFoodRecoveryKind::FishShore => ReportedSiteKind::FishShore,
                LocatedFoodRecoveryKind::HuntingLair => ReportedSiteKind::HuntingLair,
                LocatedFoodRecoveryKind::FarmPlot => ReportedSiteKind::FarmPlot,
                LocatedFoodRecoveryKind::Cookhouse => ReportedSiteKind::Cookhouse,
            };
            if self.site.as_ref().map(|site| site.kind) != Some(expected) {
                return Err(ContentPlannerError::MissingReportedSite);
            }
        }
        if self.kind == CandidateKind::FeedHole
            && (self.site.as_ref().map(|site| site.kind) != Some(ReportedSiteKind::HoleWorkArea)
                || self.cargo.is_none())
        {
            return Err(ContentPlannerError::MalformedReport);
        }
        Ok(())
    }
}

/// The only strategic input surface. There is intentionally no authoritative
/// stock, regeneration, route, reservation, or executor field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportSafePlanningInput {
    pub schema_version: u32,
    pub colony_id: PlannerId,
    pub report_version: u64,
    pub observed_tick: u64,
    pub posture: LeaderPosture,
    pub candidates: Vec<ReportedCandidate>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedReportSafePlanningInput {
    schema_version: u32,
    colony_id: PlannerId,
    report_version: u64,
    observed_tick: u64,
    posture: LeaderPosture,
    candidates: Vec<ReportedCandidate>,
}

impl<'de> Deserialize<'de> for ReportSafePlanningInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;
        let raw = UncheckedReportSafePlanningInput::deserialize(deserializer)?;
        let value = Self {
            schema_version: raw.schema_version,
            colony_id: raw.colony_id,
            report_version: raw.report_version,
            observed_tick: raw.observed_tick,
            posture: raw.posture,
            candidates: raw.candidates,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

impl ReportSafePlanningInput {
    pub fn canonicalize(&mut self) {
        self.candidates.sort_by(|left, right| {
            left.kind
                .cmp(&right.kind)
                .then_with(|| left.target_id.cmp(&right.target_id))
                .then_with(|| left.id.cmp(&right.id))
        });
    }

    fn validate(&self) -> Result<(), ContentPlannerError> {
        if self.schema_version != CONTENT_PLANNER_SCHEMA_VERSION
            || self.candidates.len() > MAX_REPORT_CANDIDATES
        {
            return Err(ContentPlannerError::MalformedReport);
        }
        let mut ids = BTreeSet::new();
        for candidate in &self.candidates {
            candidate.validate()?;
            if !ids.insert(candidate.id.clone()) {
                return Err(ContentPlannerError::DuplicateCandidate);
            }
        }
        for candidate in &self.candidates {
            for fallback in &candidate.ordered_fallbacks {
                let Some(target) = self
                    .candidates
                    .iter()
                    .find(|possible| &possible.id == fallback)
                else {
                    return Err(ContentPlannerError::UnknownFallback);
                };
                if target.domain != candidate.domain || target.kind != candidate.kind {
                    return Err(ContentPlannerError::InvalidFallback);
                }
            }
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ContentPlannerError> {
        let mut canonical = self.clone();
        canonical.canonicalize();
        canonical.validate()?;
        serde_json::to_vec(&canonical).map_err(|_| ContentPlannerError::MalformedReport)
    }
}

pub fn planner_report_bytes(
    input: &ReportSafePlanningInput,
) -> Result<Vec<u8>, ContentPlannerError> {
    input.canonical_bytes()
}

pub fn god_report_bytes(input: &ReportSafePlanningInput) -> Result<Vec<u8>, ContentPlannerError> {
    input.canonical_bytes()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerCompetence {
    Strong,
    Ordinary,
    Weak,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerOwner {
    Leader,
    Officer(OfficerRole),
    FoundingLeaderVacancy(OfficerRole),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OfficerCoverage {
    pub role: OfficerRole,
    pub officer_id: PlannerId,
    pub effective_level: EffectiveLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KeepStockOrder {
    pub id: PlannerId,
    pub officer_role: OfficerRole,
    pub content_id: ContentId,
    pub minimum_units: u32,
    pub target_units: u32,
    pub created_tick: u64,
}

impl KeepStockOrder {
    fn validate(&self) -> Result<(), ContentPlannerError> {
        if self.minimum_units == 0 || self.target_units < self.minimum_units {
            return Err(ContentPlannerError::MalformedStandingOrder);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum TypedOfficerRequest {
    Dependency {
        candidate_id: PlannerId,
        depends_on_candidate_id: PlannerId,
    },
    Space {
        candidate_id: PlannerId,
        site_kind: ReportedSiteKind,
        required_cells: u16,
    },
    Workshop {
        candidate_id: PlannerId,
        station_id: PlannerId,
        operation_id: PlannerId,
    },
}

impl TypedOfficerRequest {
    fn candidate_id(&self) -> &PlannerId {
        match self {
            Self::Dependency { candidate_id, .. }
            | Self::Space { candidate_id, .. }
            | Self::Workshop { candidate_id, .. } => candidate_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OfficerPlanRequest {
    pub request_id: PlannerId,
    pub officer_role: OfficerRole,
    pub report_id: PlannerId,
    pub expires_tick: u64,
    pub request: TypedOfficerRequest,
}

pub fn officer_plan_requests(
    book: &OfficerRequestBook,
    now_tick: u64,
) -> Result<Vec<OfficerPlanRequest>, ContentPlannerError> {
    let mut requests = Vec::new();
    for (_, request) in book.iter() {
        if request.state.is_terminal() || now_tick >= request.expiry_tick {
            continue;
        }
        let typed = match &request.payload {
            OfficerRequestPayload::Dependency {
                dependency_target_id,
            } => TypedOfficerRequest::Dependency {
                candidate_id: request.target_id.clone(),
                depends_on_candidate_id: dependency_target_id.clone(),
            },
            OfficerRequestPayload::Space {
                kind,
                required_cells,
            } => TypedOfficerRequest::Space {
                candidate_id: request.target_id.clone(),
                site_kind: request_space_kind(*kind),
                required_cells: *required_cells,
            },
            OfficerRequestPayload::Workshop {
                station_id,
                operation_id,
            } => TypedOfficerRequest::Workshop {
                candidate_id: request.target_id.clone(),
                station_id: station_id.clone(),
                operation_id: operation_id.clone(),
            },
            OfficerRequestPayload::Target | OfficerRequestPayload::KeepStock { .. } => continue,
        };
        let Some(report_id) = request.report_ids.iter().next() else {
            return Err(ContentPlannerError::MalformedOfficerRequest);
        };
        requests.push(OfficerPlanRequest {
            request_id: PlannerId::derive("officer_request_bridge", [request.id.as_str()]),
            officer_role: request.officer_role,
            report_id: PlannerId::derive("officer_report_bridge", [report_id.as_str()]),
            expires_tick: request.expiry_tick,
            request: typed,
        });
    }
    requests.sort_by(|left, right| left.request_id.cmp(&right.request_id));
    Ok(requests)
}

pub fn keep_stock_orders(
    book: &OfficerRequestBook,
    now_tick: u64,
) -> Result<Vec<KeepStockOrder>, ContentPlannerError> {
    let mut orders = Vec::new();
    for (_, request) in book.iter() {
        if request.state.is_terminal() || now_tick >= request.expiry_tick {
            continue;
        }
        let OfficerRequestPayload::KeepStock {
            content_id,
            minimum_units,
            target_units,
        } = &request.payload
        else {
            continue;
        };
        let order = KeepStockOrder {
            id: PlannerId::derive("keep_stock_order", [request.id.as_str()]),
            officer_role: request.officer_role,
            content_id: content_id.clone(),
            minimum_units: *minimum_units,
            target_units: *target_units,
            created_tick: request.creation_tick,
        };
        order.validate()?;
        orders.push(order);
    }
    orders.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(orders)
}

const fn request_space_kind(kind: RequestedSpaceKind) -> ReportedSiteKind {
    match kind {
        RequestedSpaceKind::HoleWorkArea => ReportedSiteKind::HoleWorkArea,
        RequestedSpaceKind::HuntingLair => ReportedSiteKind::HuntingLair,
        RequestedSpaceKind::AppleTree => ReportedSiteKind::AppleTree,
        RequestedSpaceKind::FishShore => ReportedSiteKind::FishShore,
        RequestedSpaceKind::FarmPlot => ReportedSiteKind::FarmPlot,
        RequestedSpaceKind::Cookhouse => ReportedSiteKind::Cookhouse,
        RequestedSpaceKind::Workshop => ReportedSiteKind::Workshop,
        RequestedSpaceKind::Stockpile => ReportedSiteKind::Stockpile,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CargoStage {
    BeforePickup,
    PickedUp,
    Delivered,
    Lost,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutionFeedback {
    pub goal_id: PlannerId,
    pub cargo_stage: CargoStage,
    pub delivery_endpoint: Option<ReportedSiteRef>,
    pub salvage_endpoint: Option<ReportedSiteRef>,
    pub reported_delivery_route_viable: bool,
    pub failure: Option<RecoveryReason>,
    pub report_id: PlannerId,
}

impl ExecutionFeedback {
    fn validate(&self) -> Result<(), ContentPlannerError> {
        if self.cargo_stage == CargoStage::PickedUp
            && (self.delivery_endpoint.is_none() || self.salvage_endpoint.is_none())
        {
            return Err(ContentPlannerError::MissingCargoDisposition);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryReason {
    ReportedShortage,
    ReportedRouteLoss,
    WorkerDeathOrIncapacity,
    ReportedDestinationFull,
    ReportedSourceUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalLifecycle {
    Proposed,
    Expanded,
    AwaitingSite,
    Reserving,
    Assigning,
    Executing,
    Observing,
    Recovering,
    Succeeded,
    Blocked,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GoalRequirement {
    Space {
        site_kind: ReportedSiteKind,
        required_cells: u16,
    },
    Workshop {
        station_id: PlannerId,
        operation_id: PlannerId,
    },
}

impl GoalLifecycle {
    fn terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PersistentGoal {
    pub id: PlannerId,
    pub occurrence: u32,
    pub candidate_id: PlannerId,
    pub domain: ContentDomain,
    pub kind: CandidateKind,
    pub target_id: PlannerId,
    pub owner: PlannerOwner,
    pub created_tick: u64,
    pub last_review_tick: u64,
    pub score: i64,
    pub confidence_basis_points: u16,
    pub lifecycle: GoalLifecycle,
    pub dependencies: BTreeSet<PlannerId>,
    pub requirements: BTreeSet<GoalRequirement>,
    pub ordered_fallbacks: Vec<PlannerId>,
    pub site: Option<ReportedSiteRef>,
    pub cargo: Option<ReportedCargo>,
    pub rationale_key: PlannerId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OmissionRecord {
    pub review_tick: u64,
    pub candidate_id: PlannerId,
    pub roll_basis_points: u16,
    pub threshold_basis_points: u16,
    pub owner: PlannerOwner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticKind {
    CandidateScored,
    CandidateOmitted,
    CandidateExpanded,
    FallbackSelected,
    EmergencyPreemption,
    CargoDelivery,
    CargoSalvage,
    RecoveryRequested,
    ReplayAccepted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReportSafeDiagnostic {
    pub tick: u64,
    pub kind: DiagnosticKind,
    pub subject_id: PlannerId,
    pub rationale_key: PlannerId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CargoIntent {
    None,
    ReserveReported(ReportedCargo),
    DeliverPicked { endpoint: ReportedSiteRef },
    SalvagePicked { endpoint: ReportedSiteRef },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerCommandStage {
    ResolveReportedSite,
    RequestReservation,
    RequestAssignment,
    Execute,
    Observe,
    Recover,
    PreemptBeforePickup,
    DeliverPickedCargo,
    SalvagePickedCargo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlannerCommand {
    pub id: PlannerId,
    pub goal_id: PlannerId,
    pub stage: PlannerCommandStage,
    pub site: Option<ReportedSiteRef>,
    pub cargo_intent: CargoIntent,
    pub ordered_fallbacks: Vec<PlannerId>,
    pub reason: PlannerId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlannerReviewOutcome {
    pub state_version: u64,
    pub planning_epoch: u64,
    pub phases: Vec<PlannerPhase>,
    pub posture: LeaderPosture,
    pub commands: Vec<PlannerCommand>,
    pub omitted: Vec<OmissionRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewReceipt {
    pub request_id: PlannerId,
    pub fingerprint: u64,
    pub outcome: PlannerReviewOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentPlannerState {
    pub schema_version: u32,
    pub colony_id: PlannerId,
    pub version: u64,
    pub planning_clock: u64,
    pub planning_epoch: u64,
    pub last_phase: PlannerPhase,
    pub posture: LeaderPosture,
    pub live_goals: BTreeMap<PlannerId, PersistentGoal>,
    pub terminal_goals: BTreeMap<PlannerId, PersistentGoal>,
    pub standing_orders: BTreeMap<PlannerId, KeepStockOrder>,
    pub omission_history: Vec<OmissionRecord>,
    pub receipts: BTreeMap<PlannerId, ReviewReceipt>,
    pub diagnostics: Vec<ReportSafeDiagnostic>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedContentPlannerState {
    schema_version: u32,
    colony_id: PlannerId,
    version: u64,
    planning_clock: u64,
    planning_epoch: u64,
    last_phase: PlannerPhase,
    posture: LeaderPosture,
    live_goals: BTreeMap<PlannerId, PersistentGoal>,
    terminal_goals: BTreeMap<PlannerId, PersistentGoal>,
    standing_orders: BTreeMap<PlannerId, KeepStockOrder>,
    omission_history: Vec<OmissionRecord>,
    receipts: BTreeMap<PlannerId, ReviewReceipt>,
    diagnostics: Vec<ReportSafeDiagnostic>,
}

impl<'de> Deserialize<'de> for ContentPlannerState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;
        let raw = UncheckedContentPlannerState::deserialize(deserializer)?;
        let state = Self {
            schema_version: raw.schema_version,
            colony_id: raw.colony_id,
            version: raw.version,
            planning_clock: raw.planning_clock,
            planning_epoch: raw.planning_epoch,
            last_phase: raw.last_phase,
            posture: raw.posture,
            live_goals: raw.live_goals,
            terminal_goals: raw.terminal_goals,
            standing_orders: raw.standing_orders,
            omission_history: raw.omission_history,
            receipts: raw.receipts,
            diagnostics: raw.diagnostics,
        };
        state.validate().map_err(D::Error::custom)?;
        Ok(state)
    }
}

impl ContentPlannerState {
    #[must_use]
    pub fn new(colony_id: PlannerId) -> Self {
        Self {
            schema_version: CONTENT_PLANNER_SCHEMA_VERSION,
            colony_id,
            version: 0,
            planning_clock: 0,
            planning_epoch: 0,
            last_phase: PlannerPhase::Observe,
            posture: LeaderPosture::Stabilize,
            live_goals: BTreeMap::new(),
            terminal_goals: BTreeMap::new(),
            standing_orders: BTreeMap::new(),
            omission_history: Vec::new(),
            receipts: BTreeMap::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn install_standing_order(
        &mut self,
        order: KeepStockOrder,
        coverage: &[OfficerCoverage],
    ) -> Result<(), ContentPlannerError> {
        order.validate()?;
        let Some(officer) = coverage
            .iter()
            .find(|coverage| coverage.role == order.officer_role)
        else {
            return Err(ContentPlannerError::StandingOrderRequiresSpecialist);
        };
        if officer.effective_level.get() < 3 {
            return Err(ContentPlannerError::StandingOrderRequiresExpertise);
        }
        let slot_limit = usize::from(officer.effective_level.get() - 2) * 2;
        let existing = self
            .standing_orders
            .values()
            .filter(|current| current.officer_role == order.officer_role)
            .count();
        if !self.standing_orders.contains_key(&order.id) && existing >= slot_limit {
            return Err(ContentPlannerError::StandingOrderCapacityReached);
        }
        if !self.standing_orders.contains_key(&order.id)
            && self.standing_orders.len() >= STANDING_ORDER_CAPACITY
        {
            return Err(ContentPlannerError::StandingOrderCapacityReached);
        }
        let next_version = self
            .version
            .checked_add(1)
            .ok_or(ContentPlannerError::ArithmeticOverflow)?;
        self.standing_orders.insert(order.id.clone(), order);
        self.version = next_version;
        Ok(())
    }

    pub fn drain_terminal_goals(
        &mut self,
        maximum: usize,
    ) -> Result<Vec<PersistentGoal>, ContentPlannerError> {
        if maximum > MAX_DRAIN_BATCH {
            return Err(ContentPlannerError::DrainTooLarge);
        }
        let ids = self
            .terminal_goals
            .keys()
            .take(maximum)
            .cloned()
            .collect::<Vec<_>>();
        let drained = ids
            .into_iter()
            .filter_map(|id| self.terminal_goals.remove(&id))
            .collect::<Vec<_>>();
        let drained_ids = drained
            .iter()
            .map(|goal| goal.id.clone())
            .collect::<BTreeSet<_>>();
        for goal in self.live_goals.values_mut() {
            goal.dependencies
                .retain(|dependency| !drained_ids.contains(dependency));
        }
        for goal in self.terminal_goals.values_mut() {
            goal.dependencies
                .retain(|dependency| !drained_ids.contains(dependency));
        }
        Ok(drained)
    }

    fn validate(&self) -> Result<(), ContentPlannerError> {
        if self.schema_version != CONTENT_PLANNER_SCHEMA_VERSION
            || self.live_goals.len() > LIVE_GOAL_CAPACITY
            || self.terminal_goals.len() > TERMINAL_GOAL_CAPACITY
            || self.standing_orders.len() > STANDING_ORDER_CAPACITY
            || self.omission_history.len() > OMISSION_HISTORY_CAPACITY
            || self.receipts.len() > REVIEW_RECEIPT_CAPACITY
            || self.diagnostics.len() > DIAGNOSTIC_CAPACITY
        {
            return Err(ContentPlannerError::MalformedPersistence);
        }
        let all_ids = self
            .live_goals
            .keys()
            .chain(self.terminal_goals.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        if all_ids.len() != self.live_goals.len() + self.terminal_goals.len() {
            return Err(ContentPlannerError::MalformedPersistence);
        }
        let dependency_count = self
            .live_goals
            .values()
            .chain(self.terminal_goals.values())
            .map(|goal| goal.dependencies.len() + goal.requirements.len())
            .sum::<usize>();
        if dependency_count > DEPENDENCY_CAPACITY {
            return Err(ContentPlannerError::MalformedPersistence);
        }
        for (id, goal) in &self.live_goals {
            if id != &goal.id
                || goal.lifecycle.terminal()
                || goal
                    .dependencies
                    .iter()
                    .any(|dependency| !all_ids.contains(dependency))
            {
                return Err(ContentPlannerError::MalformedPersistence);
            }
        }
        for (id, goal) in &self.terminal_goals {
            if id != &goal.id
                || !goal.lifecycle.terminal()
                || goal
                    .dependencies
                    .iter()
                    .any(|dependency| !all_ids.contains(dependency))
            {
                return Err(ContentPlannerError::MalformedPersistence);
            }
        }
        if has_dependency_cycle(&self.live_goals) {
            return Err(ContentPlannerError::DependencyCycle);
        }
        for order in self.standing_orders.values() {
            order.validate()?;
        }
        for (id, receipt) in &self.receipts {
            if id != &receipt.request_id
                || receipt.outcome.phases.as_slice() != PlannerPhase::ORDER.as_slice()
                || receipt.outcome.commands.len() > MAX_COMMANDS_PER_REVIEW
                || receipt.outcome.omitted.len() > MAX_REPORT_CANDIDATES
                || receipt.outcome.state_version > self.version
                || receipt.outcome.planning_epoch > self.planning_epoch
            {
                return Err(ContentPlannerError::MalformedPersistence);
            }
        }
        for goal in self.live_goals.values().chain(self.terminal_goals.values()) {
            if goal.ordered_fallbacks.len() > MAX_FALLBACKS
                || goal.confidence_basis_points > 10_000
                || goal.requirements.len() > MAX_FALLBACKS
            {
                return Err(ContentPlannerError::MalformedPersistence);
            }
        }
        if self.omission_history.iter().any(|record| {
            record.roll_basis_points >= 10_000 || record.threshold_basis_points > 10_000
        }) {
            return Err(ContentPlannerError::MalformedPersistence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlannerReviewRequest {
    pub request_id: PlannerId,
    pub expected_state_version: u64,
    pub world_seed: u32,
    pub review_tick: u64,
    pub leader_id: PlannerId,
    pub leader_level: EffectiveLevel,
    pub competence: PlannerCompetence,
    pub report: ReportSafePlanningInput,
    pub officers: Vec<OfficerCoverage>,
    pub officer_requests: Vec<OfficerPlanRequest>,
    pub execution_feedback: Vec<ExecutionFeedback>,
}

impl PlannerReviewRequest {
    fn canonicalize(&mut self) {
        self.report.canonicalize();
        self.officers.sort_by(|left, right| {
            left.role
                .cmp(&right.role)
                .then_with(|| left.officer_id.cmp(&right.officer_id))
        });
        self.officer_requests.sort_by(|left, right| {
            left.request_id
                .cmp(&right.request_id)
                .then_with(|| left.officer_role.cmp(&right.officer_role))
        });
        self.execution_feedback
            .sort_by(|left, right| left.goal_id.cmp(&right.goal_id));
    }

    fn validate(&self, state: &ContentPlannerState) -> Result<(), ContentPlannerError> {
        self.report.validate()?;
        if self.report.colony_id != state.colony_id
            || self.review_tick < state.planning_clock
            || self.officers.len() > OfficerRole::ALL.len()
            || self.officer_requests.len() > MAX_REPORT_CANDIDATES
            || self.execution_feedback.len() > MAX_REPORT_CANDIDATES
        {
            return Err(ContentPlannerError::MalformedReview);
        }
        let mut roles = BTreeSet::new();
        for officer in &self.officers {
            if !roles.insert(officer.role) {
                return Err(ContentPlannerError::MalformedReview);
            }
        }
        for request in &self.officer_requests {
            if request.expires_tick <= self.review_tick
                || !roles.contains(&request.officer_role)
                || !self
                    .report
                    .candidates
                    .iter()
                    .any(|candidate| &candidate.id == request.request.candidate_id())
            {
                return Err(ContentPlannerError::MalformedOfficerRequest);
            }
            match &request.request {
                TypedOfficerRequest::Dependency {
                    candidate_id,
                    depends_on_candidate_id,
                } => {
                    if candidate_id == depends_on_candidate_id
                        || !self
                            .report
                            .candidates
                            .iter()
                            .any(|candidate| &candidate.id == depends_on_candidate_id)
                    {
                        return Err(ContentPlannerError::MalformedOfficerRequest);
                    }
                }
                TypedOfficerRequest::Space {
                    candidate_id,
                    site_kind,
                    required_cells,
                } => {
                    let site_matches = self.report.candidates.iter().any(|candidate| {
                        &candidate.id == candidate_id
                            && candidate.site.as_ref().map(|site| site.kind) == Some(*site_kind)
                    });
                    if *required_cells == 0 || !site_matches {
                        return Err(ContentPlannerError::MalformedOfficerRequest);
                    }
                }
                TypedOfficerRequest::Workshop { .. } => {}
            }
        }
        for feedback in &self.execution_feedback {
            feedback.validate()?;
        }
        for candidate in &self.report.candidates {
            if candidate.kind == CandidateKind::KeepStock {
                let Some(cargo) = &candidate.cargo else {
                    return Err(ContentPlannerError::MalformedStandingOrder);
                };
                let Some(order) = state.standing_orders.values().find(|order| {
                    order.content_id == cargo.content_id
                        && cargo.believed_units < order.minimum_units
                }) else {
                    return Err(ContentPlannerError::MalformedStandingOrder);
                };
                if !roles.contains(&order.officer_role) {
                    return Err(ContentPlannerError::StandingOrderRequiresSpecialist);
                }
            }
        }
        Ok(())
    }

    fn fingerprint(&self) -> Result<u64, ContentPlannerError> {
        let bytes = serde_json::to_vec(self).map_err(|_| ContentPlannerError::MalformedReview)?;
        Ok(fnv1a64(&bytes))
    }
}

pub fn review(
    state: &mut ContentPlannerState,
    mut request: PlannerReviewRequest,
) -> Result<PlannerReviewOutcome, ContentPlannerError> {
    request.canonicalize();
    let fingerprint = request.fingerprint()?;
    if let Some(receipt) = state.receipts.get(&request.request_id) {
        return if receipt.fingerprint == fingerprint {
            Ok(receipt.outcome.clone())
        } else {
            Err(ContentPlannerError::ReplayConflict)
        };
    }
    if request.expected_state_version != state.version {
        return Err(ContentPlannerError::VersionConflict {
            expected: request.expected_state_version,
            actual: state.version,
        });
    }
    request.validate(state)?;
    let mut working = state.clone();
    let outcome = apply_review(&mut working, &request)?;
    working.receipts.insert(
        request.request_id.clone(),
        ReviewReceipt {
            request_id: request.request_id,
            fingerprint,
            outcome: outcome.clone(),
        },
    );
    trim_receipts(&mut working.receipts);
    working.validate()?;
    *state = working;
    Ok(outcome)
}

fn apply_review(
    state: &mut ContentPlannerState,
    request: &PlannerReviewRequest,
) -> Result<PlannerReviewOutcome, ContentPlannerError> {
    state.planning_clock = request.review_tick;
    state.planning_epoch = state
        .planning_epoch
        .checked_add(1)
        .ok_or(ContentPlannerError::ArithmeticOverflow)?;
    state.posture = request.report.posture;
    state.last_phase = PlannerPhase::ObserveOrRecover;

    let mut commands = feedback_commands(state, request)?;
    let emergency_present = request
        .report
        .candidates
        .iter()
        .any(|candidate| candidate.kind.critical());
    if emergency_present {
        commands.extend(preemption_commands(state, request)?);
    }

    let selected = select_candidates(request);
    let omission_roll = review_omission_roll(request, state.planning_epoch);
    let covered_candidate_ids = request
        .officer_requests
        .iter()
        .map(|request| request.request.candidate_id().clone())
        .collect::<BTreeSet<_>>();
    let mut omitted = Vec::new();
    for candidate in selected {
        push_diagnostic(
            state,
            request.review_tick,
            DiagnosticKind::CandidateScored,
            candidate.id.clone(),
            candidate.rationale_key.clone(),
        );
        if !candidate.ordered_fallbacks.is_empty() {
            push_diagnostic(
                state,
                request.review_tick,
                DiagnosticKind::FallbackSelected,
                candidate.id.clone(),
                candidate.rationale_key.clone(),
            );
        }
        let owner = owner_for(candidate.domain, &request.officers);
        let effective_level = owner_effective_level(&owner, request);
        let covered = covered_candidate_ids.contains(&candidate.id);
        // Keep the previously locked rule: a valid officer request advances
        // optional omission exactly one band. Vacancy fallback degradation is
        // applied to the base level before that one-band rule.
        let threshold = optional_omission_basis_points(effective_level, covered);
        if !candidate.kind.critical() && omission_roll < threshold {
            let record = OmissionRecord {
                review_tick: request.review_tick,
                candidate_id: candidate.id.clone(),
                roll_basis_points: omission_roll,
                threshold_basis_points: threshold,
                owner: owner.clone(),
            };
            push_bounded(
                &mut state.omission_history,
                record.clone(),
                OMISSION_HISTORY_CAPACITY,
            );
            push_diagnostic(
                state,
                request.review_tick,
                DiagnosticKind::CandidateOmitted,
                candidate.id.clone(),
                candidate.rationale_key.clone(),
            );
            omitted.push(record);
            continue;
        }
        let goal = expand_goal(state, request, candidate, owner)?;
        commands.extend(commands_for_goal(&goal, request.review_tick));
        state.live_goals.insert(goal.id.clone(), goal);
    }
    wire_reported_dependencies(state, &request.officer_requests)?;
    commands.sort_by(command_order);
    commands.dedup_by(|left, right| left.id == right.id);
    if commands.len() > MAX_COMMANDS_PER_REVIEW {
        return Err(ContentPlannerError::CommandCapacityReached);
    }
    state.version = state
        .version
        .checked_add(1)
        .ok_or(ContentPlannerError::ArithmeticOverflow)?;
    state.evict_histories();
    let outcome = PlannerReviewOutcome {
        state_version: state.version,
        planning_epoch: state.planning_epoch,
        phases: PlannerPhase::ORDER.to_vec(),
        posture: state.posture,
        commands,
        omitted,
    };
    Ok(outcome)
}

fn wire_reported_dependencies(
    state: &mut ContentPlannerState,
    requests: &[OfficerPlanRequest],
) -> Result<(), ContentPlannerError> {
    for request in requests {
        let TypedOfficerRequest::Dependency {
            candidate_id,
            depends_on_candidate_id,
        } = &request.request
        else {
            continue;
        };
        let goal_id = state
            .live_goals
            .values()
            .find(|goal| &goal.candidate_id == candidate_id)
            .map(|goal| goal.id.clone())
            .ok_or(ContentPlannerError::MalformedOfficerRequest)?;
        let dependency_id = state
            .live_goals
            .values()
            .find(|goal| &goal.candidate_id == depends_on_candidate_id)
            .map(|goal| goal.id.clone())
            .ok_or(ContentPlannerError::MalformedOfficerRequest)?;
        if goal_id == dependency_id {
            return Err(ContentPlannerError::DependencyCycle);
        }
        state
            .live_goals
            .get_mut(&goal_id)
            .expect("selected goal remains live")
            .dependencies
            .insert(dependency_id);
    }
    if state
        .live_goals
        .values()
        .map(|goal| goal.dependencies.len())
        .sum::<usize>()
        > DEPENDENCY_CAPACITY
        || has_dependency_cycle(&state.live_goals)
    {
        return Err(ContentPlannerError::DependencyCycle);
    }
    Ok(())
}

impl ContentPlannerState {
    fn evict_histories(&mut self) {
        while self.terminal_goals.len() > TERMINAL_GOAL_CAPACITY {
            if let Some(id) = self
                .terminal_goals
                .values()
                .min_by(|left, right| {
                    left.last_review_tick
                        .cmp(&right.last_review_tick)
                        .then_with(|| left.id.cmp(&right.id))
                })
                .map(|goal| goal.id.clone())
            {
                self.terminal_goals.remove(&id);
                for goal in self.live_goals.values_mut() {
                    goal.dependencies.remove(&id);
                }
                for goal in self.terminal_goals.values_mut() {
                    goal.dependencies.remove(&id);
                }
            }
        }
        if self.omission_history.len() > OMISSION_HISTORY_CAPACITY {
            self.omission_history
                .drain(..self.omission_history.len() - OMISSION_HISTORY_CAPACITY);
        }
        if self.diagnostics.len() > DIAGNOSTIC_CAPACITY {
            self.diagnostics
                .drain(..self.diagnostics.len() - DIAGNOSTIC_CAPACITY);
        }
    }
}

fn select_candidates(request: &PlannerReviewRequest) -> Vec<&ReportedCandidate> {
    let mut by_semantic = BTreeMap::<(CandidateKind, PlannerId), Vec<&ReportedCandidate>>::new();
    for candidate in &request.report.candidates {
        by_semantic
            .entry((candidate.kind, candidate.target_id.clone()))
            .or_default()
            .push(candidate);
    }
    let mut selected = Vec::new();
    for candidates in by_semantic.values_mut() {
        candidates.sort_by(|left, right| candidate_choice_order(left, right, request));
        if let Some(candidate) = candidates.first() {
            selected.push(*candidate);
        }
    }
    selected.sort_by(|left, right| {
        candidate_priority(left.kind)
            .cmp(&candidate_priority(right.kind))
            .then_with(|| left.target_id.cmp(&right.target_id))
            .then_with(|| left.id.cmp(&right.id))
    });
    selected
}

fn candidate_choice_order(
    left: &ReportedCandidate,
    right: &ReportedCandidate,
    request: &PlannerReviewRequest,
) -> std::cmp::Ordering {
    if left.kind == CandidateKind::FeedHole {
        let weak_stale = request.competence == PlannerCompetence::Weak
            && request
                .review_tick
                .saturating_sub(left.report_tick.min(right.report_tick))
                >= STALE_REPORT_GAME_MINUTES;
        if weak_stale {
            return left
                .cargo
                .as_ref()
                .map_or(u32::MAX, |cargo| cargo.believed_units)
                .cmp(
                    &right
                        .cargo
                        .as_ref()
                        .map_or(u32::MAX, |cargo| cargo.believed_units),
                )
                .then_with(|| left.id.cmp(&right.id));
        }
        return left
            .cargo
            .as_ref()
            .map_or(u64::MAX, |cargo| cargo.believed_replacement_cost_milli)
            .cmp(
                &right
                    .cargo
                    .as_ref()
                    .map_or(u64::MAX, |cargo| cargo.believed_replacement_cost_milli),
            )
            .then_with(|| left.id.cmp(&right.id));
    }
    right
        .urgency_basis_points
        .cmp(&left.urgency_basis_points)
        .then_with(|| left.id.cmp(&right.id))
}

fn expand_goal(
    state: &mut ContentPlannerState,
    request: &PlannerReviewRequest,
    candidate: &ReportedCandidate,
    owner: PlannerOwner,
) -> Result<PersistentGoal, ContentPlannerError> {
    if let Some(existing_id) = state
        .live_goals
        .values()
        .find(|goal| goal.candidate_id == candidate.id)
        .map(|goal| goal.id.clone())
    {
        let confidence = effective_confidence(candidate.confidence_basis_points, &owner);
        let score = checked_score(candidate, confidence)?;
        let existing = state
            .live_goals
            .get_mut(&existing_id)
            .expect("selected live goal exists");
        existing.last_review_tick = request.review_tick;
        existing.score = score;
        existing.confidence_basis_points = confidence;
        existing.owner = owner;
        existing.requirements = requirements_for(candidate, &request.officer_requests);
        existing.ordered_fallbacks = candidate.ordered_fallbacks.clone();
        existing.site = candidate.site.clone();
        existing.cargo = candidate.cargo.clone();
        existing.rationale_key = candidate.rationale_key.clone();
        return Ok(existing.clone());
    }
    let occurrence = state
        .live_goals
        .values()
        .chain(state.terminal_goals.values())
        .filter(|goal| goal.candidate_id == candidate.id)
        .count();
    let occurrence =
        u32::try_from(occurrence).map_err(|_| ContentPlannerError::ArithmeticOverflow)?;
    let epoch = state.planning_epoch.to_string();
    let occurrence_text = occurrence.to_string();
    let id = PlannerId::derive(
        "lai45_goal",
        [
            state.colony_id.as_str(),
            epoch.as_str(),
            candidate.kind.stable_name(),
            candidate.target_id.as_str(),
            occurrence_text.as_str(),
        ],
    );
    let confidence = effective_confidence(candidate.confidence_basis_points, &owner);
    let score = checked_score(candidate, confidence)?;
    let mut dependencies = BTreeSet::new();
    for officer_request in &request.officer_requests {
        if let TypedOfficerRequest::Dependency {
            candidate_id,
            depends_on_candidate_id,
        } = &officer_request.request
            && candidate_id == &candidate.id
            && let Some(dependency) = state
                .live_goals
                .values()
                .find(|goal| &goal.candidate_id == depends_on_candidate_id)
        {
            dependencies.insert(dependency.id.clone());
        }
    }
    if state.live_goals.len() >= LIVE_GOAL_CAPACITY {
        return Err(ContentPlannerError::LiveGoalCapacityReached);
    }
    let lifecycle = if candidate.site.is_some() {
        GoalLifecycle::Expanded
    } else {
        GoalLifecycle::AwaitingSite
    };
    push_diagnostic(
        state,
        request.review_tick,
        DiagnosticKind::CandidateExpanded,
        id.clone(),
        candidate.rationale_key.clone(),
    );
    Ok(PersistentGoal {
        id,
        occurrence,
        candidate_id: candidate.id.clone(),
        domain: candidate.domain,
        kind: candidate.kind,
        target_id: candidate.target_id.clone(),
        owner,
        created_tick: request.review_tick,
        last_review_tick: request.review_tick,
        score,
        confidence_basis_points: confidence,
        lifecycle,
        dependencies,
        requirements: requirements_for(candidate, &request.officer_requests),
        ordered_fallbacks: candidate.ordered_fallbacks.clone(),
        site: candidate.site.clone(),
        cargo: candidate.cargo.clone(),
        rationale_key: candidate.rationale_key.clone(),
    })
}

fn requirements_for(
    candidate: &ReportedCandidate,
    requests: &[OfficerPlanRequest],
) -> BTreeSet<GoalRequirement> {
    requests
        .iter()
        .filter(|request| request.request.candidate_id() == &candidate.id)
        .filter_map(|request| match &request.request {
            TypedOfficerRequest::Space {
                site_kind,
                required_cells,
                ..
            } => Some(GoalRequirement::Space {
                site_kind: *site_kind,
                required_cells: *required_cells,
            }),
            TypedOfficerRequest::Workshop {
                station_id,
                operation_id,
                ..
            } => Some(GoalRequirement::Workshop {
                station_id: station_id.clone(),
                operation_id: operation_id.clone(),
            }),
            TypedOfficerRequest::Dependency { .. } => None,
        })
        .collect()
}

fn commands_for_goal(goal: &PersistentGoal, review_tick: u64) -> Vec<PlannerCommand> {
    let cargo_intent = goal
        .cargo
        .clone()
        .map_or(CargoIntent::None, CargoIntent::ReserveReported);
    [
        PlannerCommandStage::ResolveReportedSite,
        PlannerCommandStage::RequestReservation,
        PlannerCommandStage::RequestAssignment,
        PlannerCommandStage::Execute,
        PlannerCommandStage::Observe,
    ]
    .into_iter()
    .map(|stage| PlannerCommand {
        id: command_id(&goal.id, stage, review_tick),
        goal_id: goal.id.clone(),
        stage,
        site: goal.site.clone(),
        cargo_intent: if stage == PlannerCommandStage::RequestReservation {
            cargo_intent.clone()
        } else {
            CargoIntent::None
        },
        ordered_fallbacks: goal.ordered_fallbacks.clone(),
        reason: goal.rationale_key.clone(),
    })
    .collect()
}

fn feedback_commands(
    state: &mut ContentPlannerState,
    request: &PlannerReviewRequest,
) -> Result<Vec<PlannerCommand>, ContentPlannerError> {
    let mut commands = Vec::new();
    for feedback in &request.execution_feedback {
        let Some(goal) = state.live_goals.get_mut(&feedback.goal_id) else {
            return Err(ContentPlannerError::UnknownGoal);
        };
        goal.last_review_tick = request.review_tick;
        if let Some(reason) = feedback.failure {
            goal.lifecycle = GoalLifecycle::Recovering;
            commands.push(PlannerCommand {
                id: command_id(&goal.id, PlannerCommandStage::Recover, request.review_tick),
                goal_id: goal.id.clone(),
                stage: PlannerCommandStage::Recover,
                site: goal.site.clone(),
                cargo_intent: cargo_disposition(feedback)?,
                ordered_fallbacks: goal.ordered_fallbacks.clone(),
                reason: recovery_rationale(reason),
            });
        } else if feedback.cargo_stage == CargoStage::Delivered {
            goal.lifecycle = GoalLifecycle::Succeeded;
        }
    }
    let terminal_ids = state
        .live_goals
        .values()
        .filter(|goal| goal.lifecycle.terminal())
        .map(|goal| goal.id.clone())
        .collect::<Vec<_>>();
    for id in terminal_ids {
        if let Some(goal) = state.live_goals.remove(&id) {
            state.terminal_goals.insert(id, goal);
        }
    }
    Ok(commands)
}

fn preemption_commands(
    state: &mut ContentPlannerState,
    request: &PlannerReviewRequest,
) -> Result<Vec<PlannerCommand>, ContentPlannerError> {
    let mut commands = Vec::new();
    for feedback in &request.execution_feedback {
        let Some(goal) = state.live_goals.get_mut(&feedback.goal_id) else {
            continue;
        };
        if goal.kind.critical() || goal.lifecycle.terminal() {
            continue;
        }
        let (stage, cargo_intent) = match feedback.cargo_stage {
            CargoStage::BeforePickup => {
                (PlannerCommandStage::PreemptBeforePickup, CargoIntent::None)
            }
            CargoStage::PickedUp if feedback.reported_delivery_route_viable => (
                PlannerCommandStage::DeliverPickedCargo,
                cargo_disposition(feedback)?,
            ),
            CargoStage::PickedUp => (
                PlannerCommandStage::SalvagePickedCargo,
                cargo_disposition(feedback)?,
            ),
            CargoStage::Delivered | CargoStage::Lost => continue,
        };
        goal.lifecycle = GoalLifecycle::Observing;
        commands.push(PlannerCommand {
            id: command_id(&goal.id, stage, request.review_tick),
            goal_id: goal.id.clone(),
            stage,
            site: goal.site.clone(),
            cargo_intent,
            ordered_fallbacks: goal.ordered_fallbacks.clone(),
            reason: PlannerId::derive("lai45_reason", ["defense_or_self_preservation_preemption"]),
        });
    }
    Ok(commands)
}

fn cargo_disposition(feedback: &ExecutionFeedback) -> Result<CargoIntent, ContentPlannerError> {
    if feedback.cargo_stage != CargoStage::PickedUp {
        return Ok(CargoIntent::None);
    }
    if feedback.reported_delivery_route_viable {
        Ok(CargoIntent::DeliverPicked {
            endpoint: feedback
                .delivery_endpoint
                .clone()
                .ok_or(ContentPlannerError::MissingCargoDisposition)?,
        })
    } else {
        Ok(CargoIntent::SalvagePicked {
            endpoint: feedback
                .salvage_endpoint
                .clone()
                .ok_or(ContentPlannerError::MissingCargoDisposition)?,
        })
    }
}

fn owner_for(domain: ContentDomain, officers: &[OfficerCoverage]) -> PlannerOwner {
    let specialist = specialist_for(domain);
    match specialist {
        Some(role) => officers
            .iter()
            .find(|coverage| coverage.role == role)
            .map_or(PlannerOwner::FoundingLeaderVacancy(role), |_| {
                PlannerOwner::Officer(role)
            }),
        None => PlannerOwner::Leader,
    }
}

#[must_use]
pub const fn specialist_for(domain: ContentDomain) -> Option<OfficerRole> {
    match domain {
        ContentDomain::Hole
        | ContentDomain::Research
        | ContentDomain::ResearchNotes
        | ContentDomain::VoidResearch => Some(OfficerRole::Loremaster),
        ContentDomain::Hunting | ContentDomain::Danger | ContentDomain::Defense => {
            Some(OfficerRole::Captain)
        }
        ContentDomain::Food
        | ContentDomain::Apples
        | ContentDomain::Fishing
        | ContentDomain::FoodDays
        | ContentDomain::Cookhouse
        | ContentDomain::Survival => Some(OfficerRole::Farmer),
        ContentDomain::Processing
        | ContentDomain::Tools
        | ContentDomain::Fixtures
        | ContentDomain::Augmentations => Some(OfficerRole::ClothLeader),
    }
}

fn owner_effective_level(owner: &PlannerOwner, request: &PlannerReviewRequest) -> EffectiveLevel {
    match owner {
        PlannerOwner::Officer(role) => request
            .officers
            .iter()
            .find(|coverage| &coverage.role == role)
            .map_or(request.leader_level, |coverage| coverage.effective_level),
        PlannerOwner::FoundingLeaderVacancy(_) => {
            EffectiveLevel::try_from(request.leader_level.get().saturating_sub(1).max(1))
                .expect("clamped effective level")
        }
        PlannerOwner::Leader => request.leader_level,
    }
}

fn effective_confidence(confidence: u16, owner: &PlannerOwner) -> u16 {
    if matches!(owner, PlannerOwner::FoundingLeaderVacancy(_)) {
        ((u32::from(confidence) * u32::from(FOUNDING_FALLBACK_CONFIDENCE_BASIS_POINTS)) / 10_000)
            as u16
    } else {
        confidence
    }
}

fn checked_score(
    candidate: &ReportedCandidate,
    confidence_basis_points: u16,
) -> Result<i64, ContentPlannerError> {
    let strategic = i128::from(strategic_weight(candidate.kind));
    let urgency = i128::from(candidate.urgency_basis_points);
    let confidence = i128::from(confidence_basis_points);
    let benefit = i128::from(candidate.expected_benefit_milli);
    let labor = i128::from(candidate.expected_labor_cost_milli);
    let weighted = urgency
        .checked_mul(strategic)
        .and_then(|value| value.checked_mul(confidence))
        .ok_or(ContentPlannerError::ArithmeticOverflow)?
        / 100_000_000_i128;
    let result = weighted
        .checked_add(benefit)
        .and_then(|value| value.checked_sub(labor))
        .and_then(|value| {
            value.checked_add(i128::from(candidate.temporary_player_bias_basis_points))
        })
        .ok_or(ContentPlannerError::ArithmeticOverflow)?;
    i64::try_from(result).map_err(|_| ContentPlannerError::ArithmeticOverflow)
}

fn strategic_weight(kind: CandidateKind) -> u32 {
    match kind {
        CandidateKind::Defend => 20_000,
        CandidateKind::SelfPreservation => 18_000,
        CandidateKind::RecoverFood(_) => 17_000,
        CandidateKind::FeedHole => 14_000,
        CandidateKind::Hunt | CandidateKind::SupplyCookhouse => 12_000,
        CandidateKind::StudyNotes | CandidateKind::StudyVoid => 10_000,
        CandidateKind::ProcessMaterial
        | CandidateKind::CraftTool
        | CandidateKind::InstallFixture
        | CandidateKind::InstallAugmentation
        | CandidateKind::KeepStock => 9_000,
    }
}

fn candidate_priority(kind: CandidateKind) -> u8 {
    match kind {
        CandidateKind::Defend => 0,
        CandidateKind::SelfPreservation => 1,
        CandidateKind::RecoverFood(_) => 2,
        CandidateKind::SupplyCookhouse => 3,
        CandidateKind::Hunt => 4,
        CandidateKind::FeedHole => 5,
        CandidateKind::KeepStock => 6,
        CandidateKind::StudyNotes | CandidateKind::StudyVoid => 7,
        CandidateKind::ProcessMaterial
        | CandidateKind::CraftTool
        | CandidateKind::InstallFixture
        | CandidateKind::InstallAugmentation => 8,
    }
}

fn review_omission_roll(request: &PlannerReviewRequest, epoch: u64) -> u16 {
    let epoch_text = epoch.to_string();
    let roll = planner_roll(
        request.world_seed,
        PlannerRngStream::Omission,
        [
            request.report.colony_id.as_str(),
            request.leader_id.as_str(),
            request.request_id.as_str(),
            epoch_text.as_str(),
        ],
    );
    ((u64::from(roll.next_seed) * 10_000) >> 32) as u16
}

fn command_id(goal_id: &PlannerId, stage: PlannerCommandStage, tick: u64) -> PlannerId {
    let tick_text = tick.to_string();
    PlannerId::derive(
        "lai45_command",
        [
            goal_id.as_str(),
            command_stage_name(stage),
            tick_text.as_str(),
        ],
    )
}

fn command_stage_name(stage: PlannerCommandStage) -> &'static str {
    match stage {
        PlannerCommandStage::ResolveReportedSite => "resolve_reported_site",
        PlannerCommandStage::RequestReservation => "request_reservation",
        PlannerCommandStage::RequestAssignment => "request_assignment",
        PlannerCommandStage::Execute => "execute",
        PlannerCommandStage::Observe => "observe",
        PlannerCommandStage::Recover => "recover",
        PlannerCommandStage::PreemptBeforePickup => "preempt_before_pickup",
        PlannerCommandStage::DeliverPickedCargo => "deliver_picked_cargo",
        PlannerCommandStage::SalvagePickedCargo => "salvage_picked_cargo",
    }
}

fn command_order(left: &PlannerCommand, right: &PlannerCommand) -> std::cmp::Ordering {
    command_priority(left.stage)
        .cmp(&command_priority(right.stage))
        .then_with(|| left.goal_id.cmp(&right.goal_id))
        .then_with(|| left.id.cmp(&right.id))
}

fn command_priority(stage: PlannerCommandStage) -> u8 {
    match stage {
        PlannerCommandStage::PreemptBeforePickup => 0,
        PlannerCommandStage::DeliverPickedCargo | PlannerCommandStage::SalvagePickedCargo => 1,
        PlannerCommandStage::Recover => 2,
        PlannerCommandStage::ResolveReportedSite => 3,
        PlannerCommandStage::RequestReservation => 4,
        PlannerCommandStage::RequestAssignment => 5,
        PlannerCommandStage::Execute => 6,
        PlannerCommandStage::Observe => 7,
    }
}

fn recovery_rationale(reason: RecoveryReason) -> PlannerId {
    let value = match reason {
        RecoveryReason::ReportedShortage => "reported_shortage",
        RecoveryReason::ReportedRouteLoss => "reported_route_loss",
        RecoveryReason::WorkerDeathOrIncapacity => "worker_death_or_incapacity",
        RecoveryReason::ReportedDestinationFull => "reported_destination_full",
        RecoveryReason::ReportedSourceUnavailable => "reported_source_unavailable",
    };
    PlannerId::derive("lai45_recovery", [value])
}

fn push_diagnostic(
    state: &mut ContentPlannerState,
    tick: u64,
    kind: DiagnosticKind,
    subject_id: PlannerId,
    rationale_key: PlannerId,
) {
    push_bounded(
        &mut state.diagnostics,
        ReportSafeDiagnostic {
            tick,
            kind,
            subject_id,
            rationale_key,
        },
        DIAGNOSTIC_CAPACITY,
    );
}

fn push_bounded<T>(values: &mut Vec<T>, value: T, capacity: usize) {
    if values.len() == capacity {
        values.remove(0);
    }
    values.push(value);
}

fn trim_receipts(receipts: &mut BTreeMap<PlannerId, ReviewReceipt>) {
    while receipts.len() > REVIEW_RECEIPT_CAPACITY {
        let Some(id) = receipts
            .values()
            .min_by(|left, right| {
                left.outcome
                    .planning_epoch
                    .cmp(&right.outcome.planning_epoch)
                    .then_with(|| left.request_id.cmp(&right.request_id))
            })
            .map(|receipt| receipt.request_id.clone())
        else {
            break;
        };
        receipts.remove(&id);
    }
}

fn has_dependency_cycle(goals: &BTreeMap<PlannerId, PersistentGoal>) -> bool {
    fn visit(
        id: &PlannerId,
        goals: &BTreeMap<PlannerId, PersistentGoal>,
        visiting: &mut BTreeSet<PlannerId>,
        complete: &mut BTreeSet<PlannerId>,
    ) -> bool {
        if complete.contains(id) {
            return false;
        }
        if !visiting.insert(id.clone()) {
            return true;
        }
        if let Some(goal) = goals.get(id) {
            for dependency in &goal.dependencies {
                if goals.contains_key(dependency) && visit(dependency, goals, visiting, complete) {
                    return true;
                }
            }
        }
        visiting.remove(id);
        complete.insert(id.clone());
        false
    }
    let mut visiting = BTreeSet::new();
    let mut complete = BTreeSet::new();
    goals
        .keys()
        .any(|id| visit(id, goals, &mut visiting, &mut complete))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentPlannerError {
    MalformedReport,
    MalformedReview,
    MalformedPersistence,
    MalformedStandingOrder,
    MalformedOfficerRequest,
    DuplicateCandidate,
    UnknownFallback,
    InvalidFallback,
    MissingReportedSite,
    MissingCargoDisposition,
    UnknownGoal,
    DependencyCycle,
    ArithmeticOverflow,
    CommandCapacityReached,
    LiveGoalCapacityReached,
    StandingOrderRequiresSpecialist,
    StandingOrderRequiresExpertise,
    StandingOrderCapacityReached,
    DrainTooLarge,
    ReplayConflict,
    VersionConflict { expected: u64, actual: u64 },
}

impl fmt::Display for ContentPlannerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "content planner error: {self:?}")
    }
}

impl std::error::Error for ContentPlannerError {}
