//! LAI.64 canonical, report-safe protocol family.
//!
//! This is deliberately a DTO-only boundary: server adapters construct these
//! projections from `cat-sim` authority and validate actions before dispatch.
//! It never exposes hidden stock or exact ecological regeneration.

use std::{collections::BTreeSet, fmt};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::PROTOCOL_VERSION;

pub const CANONICAL_SNAPSHOT_SCHEMA_VERSION: u32 = 2;
pub const CANONICAL_ACTION_SCHEMA_VERSION: u32 = 2;
pub const MAX_CANONICAL_COLONIES: usize = 256;
pub const MAX_CANONICAL_ITEMS: usize = 1_024;
pub const MAX_CANONICAL_TASKS: usize = 512;
pub const MAX_CANONICAL_ROUTE_TILES: usize = 4_096;
pub const MAX_CANONICAL_ACTION_BATCH: usize = 64;
pub const MAX_CANONICAL_STABLE_ID_BYTES: usize = 512;
pub const CANONICAL_CLICK_BATCH_WINDOW_MS: u64 = 100;
pub const MAX_CANONICAL_SNAPSHOT_WIRE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_CANONICAL_ACTION_WIRE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalWireError {
    UnsupportedProtocolVersion(u32),
    UnsupportedSchemaVersion(u32),
    MalformedHeader,
    MalformedPayload,
    InvalidBounds(&'static str),
    DuplicateOrUnordered(&'static str),
    WrongPartition,
    UnsupportedAction,
}

impl fmt::Display for CanonicalWireError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedProtocolVersion(value) => write!(out, "unsupported protocol {value}"),
            Self::UnsupportedSchemaVersion(value) => write!(out, "unsupported schema {value}"),
            Self::MalformedHeader => out.write_str("malformed action or snapshot header"),
            Self::MalformedPayload => out.write_str("malformed canonical payload"),
            Self::InvalidBounds(field) => write!(out, "invalid bounds: {field}"),
            Self::DuplicateOrUnordered(field) => write!(out, "duplicate or unordered: {field}"),
            Self::WrongPartition => out.write_str("wrong selected-colony partition"),
            Self::UnsupportedAction => out.write_str("unsupported canonical action"),
        }
    }
}

impl std::error::Error for CanonicalWireError {}

/// A catalog, entity, colony, receipt, or identity reference. Existing
/// authorities use both `colony:home` and length-prefixed
/// `planner:v1|<bytes>:<component>` identities, so the wire preserves those
/// values losslessly instead of inventing a second ID namespace.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct StableId(String);

impl StableId {
    pub fn new(value: impl Into<String>) -> Result<Self, CanonicalWireError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= MAX_CANONICAL_STABLE_ID_BYTES
            && !value.chars().any(char::is_control);
        if valid {
            Ok(Self(value))
        } else {
            Err(CanonicalWireError::InvalidBounds("stable_id"))
        }
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for StableId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ReportText(String);

impl ReportText {
    pub fn new(value: impl Into<String>) -> Result<Self, CanonicalWireError> {
        let value = value.into();
        if value.trim().is_empty() || value.len() > 512 || value.chars().any(char::is_control) {
            Err(CanonicalWireError::InvalidBounds("report_text"))
        } else {
            Ok(Self(value))
        }
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl<'de> Deserialize<'de> for ReportText {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

macro_rules! strict {
    ($item:item) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(deny_unknown_fields, rename_all = "camelCase")]
        $item
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportConfidence {
    Low,
    Moderate,
    High,
    OfficerVerified,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoodPermission {
    Allowed,
    Reserve,
    Forbidden,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalStance {
    Alliance,
    Neutral,
    Enemy,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructionPhase {
    Reserve,
    Scaffold,
    Structure,
    FitOut,
    Operational,
    Blocked,
    Cancelled,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchLane {
    God,
    Leader,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegenerationReport {
    Unavailable,
    OfficerReportedEstimate,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Proposed,
    Reserved,
    Assigned,
    InProgress,
    Blocked,
    Recovering,
    Complete,
    Refused,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeStage {
    Proposed,
    ConsentPending,
    Escrowed,
    EnRoute,
    Delivered,
    Recovering,
    Failed,
    Cancelled,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmergencySupply {
    DivineRation,
    DivineWater,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NudgeDomain {
    Survival,
    Defense,
    Hole,
    Hunting,
    Food,
    Housing,
    Construction,
    Storage,
    Research,
    Trade,
    Infrastructure,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityBandSnapshot {
    Crude,
    Common,
    Fine,
    Superior,
    Masterwork,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductionStageSnapshot {
    Reserved,
    InTransit,
    Ready,
    Working,
    OutputReady,
    Recovering,
    Complete,
    Cancelled,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifeStageSnapshot {
    Kitten,
    Adolescent,
    Adult,
    Elder,
}

strict! { pub struct Tile { pub x: i32, pub y: i32 } }
strict! { pub struct Footprint { pub ordered_tiles: Vec<Tile> } }
strict! { pub struct Route { pub ordered_tiles: Vec<Tile> } }
strict! { pub struct VersionExpectation { pub lane: VersionLane, pub expected_version: u64 } }
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionLane {
    Planner,
    Officers,
    Research,
    Construction,
    Storage,
    FoodPolicy,
    Hole,
    Divine,
    Governance,
    Diplomacy,
    Trade,
    Tasks,
    Reservations,
}
impl VersionLane {
    const COUNT: usize = 13;
}

strict! { pub struct PlanDependencySnapshot { pub plan_id: StableId, pub satisfied: bool } }
strict! { pub struct LeaderPlanSnapshot { pub plan_id: StableId, pub topic_id: StableId, pub phase: ReportText, pub priority_basis_points: u16, pub confidence: ReportConfidence, pub rationale: ReportText, #[serde(default)] pub dependencies: Vec<PlanDependencySnapshot>, #[serde(default)] pub responsible_officer_id: Option<StableId> } }
strict! { pub struct OfficerRequestSnapshotV2 { pub request_id: StableId, pub officer_id: StableId, pub request_kind: StableId, pub rationale: ReportText, pub confidence: ReportConfidence, #[serde(default)] pub capability_id: Option<StableId> } }
strict! { pub struct StandingOrderCapabilitySnapshot { pub capability_id: StableId, pub office_id: StableId, pub order_kind_id: StableId, pub enabled: bool, pub reason: ReportText } }
strict! { pub struct StandingOrderSnapshotV2 { pub order_id: StableId, pub capability_id: StableId, pub instruction: ReportText, pub expires_at_ms: Option<i64> } }

strict! { pub struct ReservationSnapshotV2 { pub reservation_id: StableId, pub cargo_id: StableId, pub state: ReportText, pub reason: Option<ReportText> } }
strict! { pub struct PhysicalCargoSnapshot { pub cargo_id: StableId, pub content_id: StableId, pub quantity: u64, pub quality_band: u8, pub provenance_id: StableId, pub created_at_ms: i64, pub reservation_id: Option<StableId>, pub container_id: Option<StableId>, pub location_site_id: Option<StableId>, pub location_tile: Option<Tile> } }
strict! { pub struct RefusalSnapshot { pub refusal_id: StableId, pub reason: ReportText, pub report_confidence: ReportConfidence } }
strict! { pub struct TaskBlockerSnapshot { pub blocker_id: StableId, pub reason: ReportText, pub recoverable: bool } }
strict! { pub struct TaskSiteSnapshotV2 { pub site_id: StableId, pub site_kind_id: StableId, pub slot_id: Option<StableId>, pub footprint: Footprint } }
strict! { pub struct PhysicalTaskSnapshot { pub task_id: StableId, pub task_kind_id: StableId, pub site_id: StableId, pub site_kind_id: StableId, pub objective: ReportText, pub state: TaskState, pub footprint: Footprint, #[serde(default)] pub work_sites: Vec<TaskSiteSnapshotV2>, #[serde(default)] pub delivery_site: Option<TaskSiteSnapshotV2>, pub route: Route, #[serde(default)] pub cargo: Vec<PhysicalCargoSnapshot>, #[serde(default)] pub reservations: Vec<ReservationSnapshotV2>, #[serde(default)] pub worker_cat_ids: Vec<StableId>, #[serde(default)] pub refusals: Vec<RefusalSnapshot>, #[serde(default)] pub anatomy_requirements: Vec<StableId>, #[serde(default)] pub blockers: Vec<TaskBlockerSnapshot> } }

strict! { pub struct SkillSnapshotV2 { pub skill_id: StableId, pub xp: u64, pub level: u16, pub mastery: u32 } }
strict! { pub struct AttributeSnapshot { pub attribute_id: StableId, pub inherited_value: u16, pub learned_value: u16, pub total_value: u16 } }
strict! { pub struct AffinitySnapshot { pub labor_id: StableId, pub disposition: ReportText, pub refusing: bool, pub refusal_reason: Option<ReportText> } }
strict! { pub struct FamilySnapshotV2 { pub household_id: Option<StableId>, pub partnership_id: Option<StableId>, #[serde(default)] pub parent_ids: Vec<StableId>, #[serde(default)] pub child_ids: Vec<StableId>, pub residence_id: Option<StableId>, pub mentor_id: Option<StableId>, pub tradition_id: Option<StableId>, pub surname: Option<ReportText>, pub enterprise_id: Option<StableId> } }
strict! { pub struct CatSnapshotV2 { pub cat_id: StableId, pub display_name: ReportText, pub life_stage: LifeStageSnapshot, pub job_id: Option<StableId>, #[serde(default)] pub attributes: Vec<AttributeSnapshot>, #[serde(default)] pub skills: Vec<SkillSnapshotV2>, #[serde(default)] pub affinities: Vec<AffinitySnapshot>, #[serde(default)] pub anatomy_eligibility: Vec<StableId>, pub family: FamilySnapshotV2, pub office_id: Option<StableId>, pub succession_eligible: bool } }

strict! { pub struct ElectionCandidateSnapshotV2 { pub cat_id: StableId, pub report_reason: ReportText, pub backing_blocks: u8, pub eligible: bool } }
strict! { pub struct GovernanceSnapshotV2 { pub election_id: Option<StableId>, #[serde(default)] pub candidates: Vec<ElectionCandidateSnapshotV2>, #[serde(default)] pub officers: Vec<OfficerSnapshotV2>, pub succession_summary: Option<ReportText> } }
strict! { pub struct OfficerSnapshotV2 { pub office_id: StableId, pub cat_id: Option<StableId>, pub report_expertise_level: u8, pub appointment_candidate_ids: Vec<StableId> } }

strict! { pub struct ResearchQueueEntrySnapshot { pub study_id: StableId, pub lane: ResearchLane, pub position: u8, pub funding_state: ReportText, pub progress_basis_points: u16, pub duplicate_reason: Option<ReportText>, pub refund_reason: Option<ReportText> } }
strict! { pub struct ResearchSnapshotV2 { pub notes_balance: u64, pub void_balance: u64, #[serde(default)] pub god_queue: Vec<ResearchQueueEntrySnapshot>, #[serde(default)] pub leader_decisions: Vec<ResearchQueueEntrySnapshot>, #[serde(default)] pub preparations: Vec<ResearchPreparationSnapshot> } }
strict! { pub struct ResearchPreparationSnapshot { pub preparation_id: StableId, pub study_id: StableId, pub physical_task_id: Option<StableId>, pub progress_basis_points: u16, pub player_discount_basis_points: u16 } }

strict! { pub struct ConstructionCargoSnapshot { pub phase: ConstructionPhase, pub work_share_basis_points: u16, #[serde(default)] pub delivered: Vec<PhysicalCargoSnapshot>, #[serde(default)] pub in_transit: Vec<PhysicalCargoSnapshot>, #[serde(default)] pub consumed: Vec<PhysicalCargoSnapshot> } }
strict! { pub struct ConstructionSnapshotV2 { pub project_id: StableId, pub building_id: StableId, pub phase: ConstructionPhase, pub footprint: Footprint, pub phase_progress_basis_points: u16, #[serde(default)] pub stage_cargo: Vec<ConstructionCargoSnapshot>, pub art_state_id: StableId } }

strict! { pub struct StorageSlotSnapshot { pub slot_id: StableId, pub lot_id: Option<StableId>, #[serde(default)] pub item_id: Option<StableId>, pub container_id: Option<StableId>, pub fullness_basis_points: u16 } }
strict! { pub struct StorageTileSnapshot { pub tile: Tile, #[serde(default)] pub slots: Vec<StorageSlotSnapshot> } }
strict! { pub struct ContainerSnapshotV2 { pub container_id: StableId, pub container_kind_id: StableId, pub capacity_slots: u8, pub contained_content_id: Option<StableId>, pub fullness_basis_points: u16 } }
strict! { pub struct StorageZoneSnapshotV2 { pub zone_id: StableId, pub linked_workshop_id: Option<StableId>, pub footprint: Footprint, #[serde(default)] pub tiles: Vec<StorageTileSnapshot>, #[serde(default)] pub containers: Vec<ContainerSnapshotV2>, #[serde(default)] pub lots: Vec<PhysicalCargoSnapshot> } }

strict! { pub struct FoodPermissionSnapshot { pub content_id: StableId, pub permission: FoodPermission, pub reason: ReportText, pub confidence: ReportConfidence } }
strict! { pub struct RegenerationEstimateSnapshot { pub lower_units_per_day: u64, pub upper_units_per_day: u64, pub observed_at_ms: i64, pub confidence: ReportConfidence } }
strict! { pub struct HoleSnapshotV2 { pub hole_id: StableId, pub width: u8, pub depth: u8, pub darkness: u8, pub footprint: Footprint, pub work_footprint: Footprint, pub food_permission_summary: ReportText, #[serde(default)] pub food_permissions: Vec<FoodPermissionSnapshot>, pub officer_report_level: u8, pub regeneration: RegenerationReport, pub officer_reported_regeneration: Option<RegenerationEstimateSnapshot>, #[serde(default)] pub contribution_receipts: Vec<StableId> } }
strict! { pub struct DivineBoostOfferSnapshotV2 { pub offer_id: StableId, pub boost_type_id: StableId, pub duration_game_hours: u32, pub exact_cost_micro_void: u64, pub effect_basis_points: i64 } }
strict! { pub struct EmergencyRescueOfferSnapshotV2 { pub witness_id: StableId, pub supply: EmergencySupply, pub quantity: u64, pub exact_cost_micro_void: u64 } }
strict! { pub struct ConstructionMiracleOfferSnapshotV2 { pub offer_id: StableId, pub project_id: StableId, pub building_id: StableId, pub phase: ConstructionPhase, pub footprint: Footprint, pub exact_cost_micro_void: u64, pub labor_reduction_basis_points: u16, pub input_value_multiplier_basis_points: u16 } }
strict! { pub struct DivineSnapshotV2 { pub inspiration_expires_at_ms: Option<i64>, #[serde(default)] pub active_boost_ids: Vec<StableId>, #[serde(default)] pub boost_offers: Vec<DivineBoostOfferSnapshotV2>, #[serde(default)] pub construction_miracle_offers: Vec<ConstructionMiracleOfferSnapshotV2>, #[serde(default)] pub rescue_offers: Vec<EmergencyRescueOfferSnapshotV2>, pub rescue_available: bool, pub rescue_reason: Option<ReportText> } }

strict! { pub struct TradeCargoSnapshotV2 { pub cargo_id: StableId, pub content_id: StableId, pub quantity: u64, pub quality_band: u8 } }
strict! { pub struct TradeContractSnapshotV2 { pub contract_id: StableId, pub partner_colony_id: StableId, pub stage: TradeStage, pub route: Route, #[serde(default)] pub escrow: Vec<TradeCargoSnapshotV2>, pub report_reason: Option<ReportText> } }
strict! { pub struct DiplomacySnapshotV2 { #[serde(default)] pub stances: Vec<PersonalStanceSnapshot>, #[serde(default)] pub contracts: Vec<TradeContractSnapshotV2> } }
strict! { pub struct PersonalStanceSnapshot { pub other_colony_id: StableId, pub stance: PersonalStance, pub consented: bool } }
strict! { pub struct DiagnosticSnapshot { pub diagnostic_id: StableId, pub domain: StableId, pub message: ReportText, pub occurred_at_ms: i64 } }

strict! { pub struct ContentManifestEntrySnapshot { pub content_id: StableId, pub content_kind_id: StableId, pub display_name: ReportText, pub art_key: StableId, pub accessibility_label: ReportText, #[serde(default)] pub capability_ids: Vec<StableId> } }
strict! { pub struct ContentManifestSnapshot { pub manifest_version: u32, pub checksum_id: StableId, #[serde(default)] pub entries: Vec<ContentManifestEntrySnapshot> } }
strict! { pub struct QualityLotSnapshotV2 { pub lot_id: StableId, pub content_id: StableId, pub quantity: u64, pub quality: QualityBandSnapshot, pub provenance_id: StableId, pub age_ms: u64, pub location_site_id: StableId, pub reservation_id: Option<StableId> } }
strict! { pub struct ExactItemSnapshotV2 { pub item_id: StableId, pub definition_id: StableId, pub material_id: StableId, pub quality: QualityBandSnapshot, pub durability_basis_points: u16, pub provenance_id: StableId, pub location_site_id: StableId, pub reservation_id: Option<StableId>, #[serde(default)] pub augmentation_ids: Vec<StableId> } }
strict! { pub struct FoodStockSnapshotV2 { pub content_id: StableId, pub lot_id: StableId, pub quantity: u64, pub quality: QualityBandSnapshot, pub nutrition_basis_points: u16, pub spoilage_basis_points: u16, pub permission: FoodPermission, pub location_site_id: StableId } }
strict! { pub struct HuntingCreatureSnapshot { pub creature_id: StableId, pub level_band: u8, pub health_basis_points: u16 } }
strict! { pub struct HuntingSiteSnapshotV2 { pub site_id: StableId, pub site_kind_id: StableId, pub tile: Tile, pub level_band: u8, #[serde(default)] pub creatures: Vec<HuntingCreatureSnapshot>, pub respawn_report: Option<ReportText>, pub report_confidence: ReportConfidence, #[serde(default)] pub cache_lot_ids: Vec<StableId>, #[serde(default)] pub cache_item_ids: Vec<StableId>, pub art_key: StableId } }
strict! { pub struct RareMaterialSnapshotV2 { pub material_instance_id: StableId, pub material_id: StableId, pub content_state_id: StableId, pub processed: bool, pub quality: QualityBandSnapshot, pub provenance_id: StableId, pub location_site_id: StableId, pub reservation_id: Option<StableId> } }
strict! { pub struct AugmentationSnapshotV2 { pub augmentation_instance_id: StableId, pub augmentation_id: StableId, pub target_item_id: StableId, pub material_instance_id: StableId, pub installed: bool, pub effect_summary: ReportText } }
strict! { pub struct FixtureSnapshotV2 { pub fixture_instance_id: StableId, pub fixture_id: StableId, pub station_id: StableId, pub installed: bool, pub quality: QualityBandSnapshot, pub effect_summary: ReportText } }
strict! { pub struct CookhouseBatchSnapshotV2 { pub batch_id: StableId, pub station_id: StableId, pub recipe_id: StableId, pub stage: ProductionStageSnapshot, pub progress_basis_points: u16, #[serde(default)] pub ingredient_lot_ids: Vec<StableId>, #[serde(default)] pub output_lot_ids: Vec<StableId>, pub worker_cat_id: Option<StableId>, pub blocker: Option<ReportText> } }
strict! { pub struct FishingHutSnapshotV2 { pub hut_id: StableId, pub footprint: Footprint, pub dock_land_tile: Tile, pub reserved_water_tile: Tile, pub orientation_id: StableId, pub mode_id: StableId, pub stage: ProductionStageSnapshot, pub progress_basis_points: u16, pub rod_item_id: Option<StableId>, pub worker_cat_id: Option<StableId>, pub habitat_report: ReportText, pub report_confidence: ReportConfidence, pub art_key: StableId } }
strict! { pub struct VisualStateSnapshotV2 { pub subject_id: StableId, pub art_key: StableId, pub state_id: StableId, pub accessibility_label: ReportText, pub footprint: Footprint } }
strict! { pub struct EventLogEntrySnapshot { pub event_id: StableId, pub domain_id: StableId, pub event_kind_id: StableId, pub message: ReportText, pub occurred_at_ms: i64, pub repeated_count: u16, pub confidence: ReportConfidence, #[serde(default)] pub source_ids: Vec<StableId> } }
strict! { pub struct ResidenceSnapshotV2 { pub residence_id: StableId, pub housing_kind_id: StableId, pub footprint: Footprint, pub capacity: u16, #[serde(default)] pub resident_cat_ids: Vec<StableId>, pub housing_pressure_basis_points: u16 } }
strict! { pub struct JobAssignmentSnapshotV2 { pub assignment_id: StableId, pub cat_id: StableId, pub job_kind_id: StableId, pub station_id: Option<StableId>, pub active: bool, pub report_reason: ReportText } }

strict! { pub struct CanonicalColonySnapshot { pub colony_id: StableId, pub state_version: u64, #[serde(default)] pub versions: Vec<VersionExpectation>, #[serde(default)] pub plans: Vec<LeaderPlanSnapshot>, #[serde(default)] pub officer_requests: Vec<OfficerRequestSnapshotV2>, #[serde(default)] pub standing_order_capabilities: Vec<StandingOrderCapabilitySnapshot>, #[serde(default)] pub standing_orders: Vec<StandingOrderSnapshotV2>, #[serde(default)] pub tasks: Vec<PhysicalTaskSnapshot>, #[serde(default)] pub cats: Vec<CatSnapshotV2>, #[serde(default)] pub job_assignments: Vec<JobAssignmentSnapshotV2>, #[serde(default)] pub residences: Vec<ResidenceSnapshotV2>, pub governance: GovernanceSnapshotV2, pub research: ResearchSnapshotV2, #[serde(default)] pub construction: Vec<ConstructionSnapshotV2>, #[serde(default)] pub storage_zones: Vec<StorageZoneSnapshotV2>, pub hole: HoleSnapshotV2, pub divine: DivineSnapshotV2, pub diplomacy: DiplomacySnapshotV2, #[serde(default)] pub content_manifest: Option<ContentManifestSnapshot>, #[serde(default)] pub quality_lots: Vec<QualityLotSnapshotV2>, #[serde(default)] pub exact_items: Vec<ExactItemSnapshotV2>, #[serde(default)] pub food_stocks: Vec<FoodStockSnapshotV2>, #[serde(default)] pub hunting_sites: Vec<HuntingSiteSnapshotV2>, #[serde(default)] pub rare_materials: Vec<RareMaterialSnapshotV2>, #[serde(default)] pub augmentations: Vec<AugmentationSnapshotV2>, #[serde(default)] pub fixtures: Vec<FixtureSnapshotV2>, #[serde(default)] pub cookhouse_batches: Vec<CookhouseBatchSnapshotV2>, #[serde(default)] pub fishing_huts: Vec<FishingHutSnapshotV2>, #[serde(default)] pub visual_states: Vec<VisualStateSnapshotV2>, #[serde(default)] pub event_log: Vec<EventLogEntrySnapshot>, #[serde(default)] pub diagnostics: Vec<DiagnosticSnapshot> } }
strict! { pub struct PublicColonySummaryV2 { pub colony_id: StableId, pub display_name: ReportText, pub can_view: bool, pub can_control: bool } }
strict! { pub struct CanonicalSnapshotEnvelope { pub protocol_version: u32, pub snapshot_schema_version: u32, pub now_ms: i64, pub selected_colony_id: StableId, #[serde(default)] pub public_colonies: Vec<PublicColonySummaryV2>, #[serde(default)] pub colonies: Vec<CanonicalColonySnapshot> } }

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotHeader {
    protocol_version: u32,
    snapshot_schema_version: u32,
}

impl CanonicalSnapshotEnvelope {
    /// Header validation intentionally happens on raw JSON before any nested
    /// payload (and thus before unknown tags or large allocations) is decoded.
    pub fn decode_json(encoded: &str) -> Result<Self, CanonicalWireError> {
        if encoded.len() > MAX_CANONICAL_SNAPSHOT_WIRE_BYTES {
            return Err(CanonicalWireError::InvalidBounds("snapshot_wire_bytes"));
        }
        let header: SnapshotHeader =
            serde_json::from_str(encoded).map_err(|_| CanonicalWireError::MalformedHeader)?;
        require_protocol(header.protocol_version)?;
        if header.snapshot_schema_version != CANONICAL_SNAPSHOT_SCHEMA_VERSION {
            return Err(CanonicalWireError::UnsupportedSchemaVersion(
                header.snapshot_schema_version,
            ));
        }
        let decoded: Self =
            serde_json::from_str(encoded).map_err(|_| CanonicalWireError::MalformedPayload)?;
        decoded.validate()?;
        Ok(decoded)
    }
    pub fn validate(&self) -> Result<(), CanonicalWireError> {
        require_protocol(self.protocol_version)?;
        if self.snapshot_schema_version != CANONICAL_SNAPSHOT_SCHEMA_VERSION {
            return Err(CanonicalWireError::UnsupportedSchemaVersion(
                self.snapshot_schema_version,
            ));
        }
        if self.colonies.len() != 1 || self.public_colonies.len() > MAX_CANONICAL_COLONIES {
            return Err(CanonicalWireError::InvalidBounds("colonies"));
        }
        if self.colonies[0].colony_id != self.selected_colony_id {
            return Err(CanonicalWireError::WrongPartition);
        }
        ordered_ids(
            self.public_colonies.iter().map(|x| &x.colony_id),
            "public_colony_ids",
        )?;
        for colony in &self.colonies {
            validate_colony(colony)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "action",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CanonicalGodAction {
    ResearchQueue {
        study_id: StableId,
    },
    ResearchReorder {
        study_id: StableId,
        before_study_id: Option<StableId>,
    },
    ResearchFund {
        study_id: StableId,
    },
    ResearchRemove {
        study_id: StableId,
    },
    ResearchPreparation {
        study_id: StableId,
    },
    FoodConservation {
        nudge_basis_points: i16,
    },
    HoleClickBatch {
        target_id: StableId,
        requested_clicks: u32,
        client_batch_window_ms: u64,
    },
    Inspiration,
    ActivateBoost {
        boost_id: StableId,
    },
    ConstructionMiracle {
        offer_id: StableId,
    },
    EmergencyRescue {
        witness_id: StableId,
    },
    CandidateBacking {
        election_id: StableId,
        candidate_id: StableId,
    },
    PersonalStance {
        other_colony_id: StableId,
        stance: PersonalStance,
    },
    Expel {
        subject_cat_id: StableId,
        household: bool,
    },
    BroadDomainNudge {
        domain: NudgeDomain,
        building_kind_id: Option<StableId>,
        basis_points: i16,
    },
    SignedTestReset {
        nonce: StableId,
        signature: ReportText,
        confirmation: ReportText,
    },
}

impl CanonicalGodAction {
    #[must_use]
    pub const fn required_lanes(&self) -> &'static [VersionLane] {
        match self {
            Self::ResearchQueue { .. }
            | Self::ResearchReorder { .. }
            | Self::ResearchFund { .. }
            | Self::ResearchRemove { .. }
            | Self::ResearchPreparation { .. } => &[VersionLane::Research],
            Self::FoodConservation { .. } => &[VersionLane::Planner, VersionLane::FoodPolicy],
            Self::HoleClickBatch { .. } => &[
                VersionLane::Hole,
                VersionLane::Divine,
                VersionLane::Reservations,
            ],
            Self::Inspiration => &[VersionLane::Divine],
            Self::ActivateBoost { .. } => &[VersionLane::Research, VersionLane::Divine],
            Self::ConstructionMiracle { .. } => &[
                VersionLane::Research,
                VersionLane::Construction,
                VersionLane::Storage,
                VersionLane::Divine,
                VersionLane::Reservations,
            ],
            Self::EmergencyRescue { .. } => &[
                VersionLane::Research,
                VersionLane::Storage,
                VersionLane::Divine,
                VersionLane::Reservations,
            ],
            Self::CandidateBacking { .. } => &[VersionLane::Governance],
            Self::PersonalStance { .. } => &[VersionLane::Diplomacy],
            Self::Expel { .. } => &[
                VersionLane::Governance,
                VersionLane::Tasks,
                VersionLane::Reservations,
            ],
            Self::BroadDomainNudge { .. } => &[VersionLane::Planner],
            Self::SignedTestReset { .. } => &[],
        }
    }
}

strict! { pub struct CanonicalActionEnvelope { pub protocol_version: u32, pub action_schema_version: u32, pub authenticated_player_id: StableId, pub selected_colony_id: StableId, pub idempotency_id: StableId, #[serde(default)] pub expected_versions: Vec<VersionExpectation>, pub payload: CanonicalGodAction } }

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActionHeader {
    protocol_version: u32,
    action_schema_version: u32,
}

impl CanonicalActionEnvelope {
    pub fn decode_json(encoded: &str) -> Result<Self, CanonicalWireError> {
        if encoded.len() > MAX_CANONICAL_ACTION_WIRE_BYTES {
            return Err(CanonicalWireError::InvalidBounds("action_wire_bytes"));
        }
        let header: ActionHeader =
            serde_json::from_str(encoded).map_err(|_| CanonicalWireError::MalformedHeader)?;
        require_protocol(header.protocol_version)?;
        if header.action_schema_version != CANONICAL_ACTION_SCHEMA_VERSION {
            return Err(CanonicalWireError::UnsupportedSchemaVersion(
                header.action_schema_version,
            ));
        }
        let decoded: Self =
            serde_json::from_str(encoded).map_err(|_| CanonicalWireError::UnsupportedAction)?;
        decoded.validate()?;
        Ok(decoded)
    }
    pub fn validate(&self) -> Result<(), CanonicalWireError> {
        require_protocol(self.protocol_version)?;
        if self.action_schema_version != CANONICAL_ACTION_SCHEMA_VERSION {
            return Err(CanonicalWireError::UnsupportedSchemaVersion(
                self.action_schema_version,
            ));
        }
        bounded_len(
            self.expected_versions.len(),
            VersionLane::COUNT,
            "expected_versions",
        )?;
        ordered_lanes(&self.expected_versions)?;
        let required_lanes = self.payload.required_lanes();
        if self.expected_versions.len() != required_lanes.len()
            || self
                .expected_versions
                .iter()
                .map(|version| version.lane)
                .ne(required_lanes.iter().copied())
        {
            return Err(CanonicalWireError::InvalidBounds("expected_versions"));
        }
        match &self.payload {
            CanonicalGodAction::HoleClickBatch {
                requested_clicks,
                client_batch_window_ms,
                ..
            } if !(1..=MAX_CANONICAL_ACTION_BATCH as u32).contains(requested_clicks)
                || *client_batch_window_ms != CANONICAL_CLICK_BATCH_WINDOW_MS =>
            {
                Err(CanonicalWireError::InvalidBounds("hole_click_batch"))
            }
            CanonicalGodAction::FoodConservation { nudge_basis_points }
                if !(-1_500..=1_500).contains(nudge_basis_points) || *nudge_basis_points == 0 =>
            {
                Err(CanonicalWireError::InvalidBounds("food_conservation"))
            }
            CanonicalGodAction::BroadDomainNudge {
                domain: _,
                building_kind_id: _,
                basis_points,
            } if !(-1_500..=1_500).contains(basis_points) || *basis_points == 0 => {
                Err(CanonicalWireError::InvalidBounds("broad_domain_nudge"))
            }
            CanonicalGodAction::BroadDomainNudge {
                domain,
                building_kind_id,
                ..
            } if building_kind_id.is_some() && !matches!(domain, NudgeDomain::Construction) => {
                Err(CanonicalWireError::InvalidBounds("building_nudge_domain"))
            }
            CanonicalGodAction::SignedTestReset { confirmation, .. }
                if confirmation.as_str() != "test_reset_confirmed" =>
            {
                Err(CanonicalWireError::InvalidBounds("test_reset_confirmation"))
            }
            _ => Ok(()),
        }
    }
}

strict! { pub struct ActionReceipt { pub idempotency_id: StableId, pub selected_colony_id: StableId, pub outcome: ActionOutcome, #[serde(default)] pub changed_ids: Vec<StableId>, pub reason: Option<ReportText>, #[serde(default)] pub committed_versions: Vec<VersionExpectation> } }
strict! { pub struct ActionErrorSnapshot { pub code: StableId, pub reason: ReportText, pub retry_after_ms: Option<u64>, #[serde(default)] pub refresh_versions: Vec<VersionExpectation> } }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionOutcome {
    Accepted,
    Replayed,
    Rejected,
    UpdateRequired,
    RateLimited,
}

fn require_protocol(version: u32) -> Result<(), CanonicalWireError> {
    if version == PROTOCOL_VERSION {
        Ok(())
    } else {
        Err(CanonicalWireError::UnsupportedProtocolVersion(version))
    }
}
fn ordered_ids<'a>(
    ids: impl IntoIterator<Item = &'a StableId>,
    label: &'static str,
) -> Result<(), CanonicalWireError> {
    let mut prior: Option<&StableId> = None;
    for id in ids {
        if prior.is_some_and(|previous| previous >= id) {
            return Err(CanonicalWireError::DuplicateOrUnordered(label));
        }
        prior = Some(id);
    }
    Ok(())
}
fn ordered_lanes(versions: &[VersionExpectation]) -> Result<(), CanonicalWireError> {
    let mut lanes = BTreeSet::new();
    for version in versions {
        if !lanes.insert(version.lane) {
            return Err(CanonicalWireError::DuplicateOrUnordered("version_lanes"));
        }
    }
    if versions.windows(2).any(|pair| pair[0].lane >= pair[1].lane) {
        Err(CanonicalWireError::DuplicateOrUnordered("version_lanes"))
    } else {
        Ok(())
    }
}
fn ordered_queue(
    entries: &[ResearchQueueEntrySnapshot],
    label: &'static str,
) -> Result<(), CanonicalWireError> {
    if entries
        .windows(2)
        .any(|pair| pair[0].position >= pair[1].position)
    {
        Err(CanonicalWireError::DuplicateOrUnordered(label))
    } else {
        Ok(())
    }
}
fn bounded_len(len: usize, maximum: usize, label: &'static str) -> Result<(), CanonicalWireError> {
    if len > maximum {
        Err(CanonicalWireError::InvalidBounds(label))
    } else {
        Ok(())
    }
}
fn validate_tiles(
    tiles: &[Tile],
    allow_empty: bool,
    label: &'static str,
) -> Result<(), CanonicalWireError> {
    if (!allow_empty && tiles.is_empty()) || tiles.len() > MAX_CANONICAL_ROUTE_TILES {
        return Err(CanonicalWireError::InvalidBounds(label));
    }
    let mut unique = BTreeSet::new();
    if tiles.iter().any(|tile| !unique.insert((tile.x, tile.y))) {
        return Err(CanonicalWireError::DuplicateOrUnordered(label));
    }
    Ok(())
}
fn validate_cargo(
    cargo: &[PhysicalCargoSnapshot],
    label: &'static str,
) -> Result<(), CanonicalWireError> {
    bounded_len(cargo.len(), MAX_CANONICAL_ITEMS, label)?;
    ordered_ids(cargo.iter().map(|entry| &entry.cargo_id), label)?;
    if cargo.iter().any(|entry| entry.quantity == 0) {
        return Err(CanonicalWireError::InvalidBounds(label));
    }
    Ok(())
}
fn is_exact_three_by_three(tiles: &[Tile]) -> bool {
    if tiles.len() != 9 {
        return false;
    }
    let unique = tiles
        .iter()
        .map(|tile| (tile.x, tile.y))
        .collect::<BTreeSet<_>>();
    let Some(min_x) = tiles.iter().map(|tile| tile.x).min() else {
        return false;
    };
    let Some(min_y) = tiles.iter().map(|tile| tile.y).min() else {
        return false;
    };
    unique.len() == 9
        && (0..3).all(|dy| (0..3).all(|dx| unique.contains(&(min_x + dx, min_y + dy))))
}
fn footprints_overlap(first: &Footprint, second: &Footprint) -> bool {
    let first_tiles = first
        .ordered_tiles
        .iter()
        .map(|tile| (tile.x, tile.y))
        .collect::<BTreeSet<_>>();
    second
        .ordered_tiles
        .iter()
        .any(|tile| first_tiles.contains(&(tile.x, tile.y)))
}
fn validate_exact_hole_footprint(tiles: &[Tile]) -> Result<(), CanonicalWireError> {
    if tiles.len() != 25 {
        return Err(CanonicalWireError::InvalidBounds("hole_footprint"));
    }
    validate_tiles(tiles, false, "hole_footprint")?;
    let min_x = tiles.iter().map(|tile| tile.x).min().unwrap_or_default();
    let max_x = tiles.iter().map(|tile| tile.x).max().unwrap_or_default();
    let min_y = tiles.iter().map(|tile| tile.y).min().unwrap_or_default();
    let max_y = tiles.iter().map(|tile| tile.y).max().unwrap_or_default();
    if i64::from(max_x) - i64::from(min_x) != 4 || i64::from(max_y) - i64::from(min_y) != 4 {
        return Err(CanonicalWireError::InvalidBounds("hole_footprint"));
    }
    Ok(())
}
fn validate_hole_work_footprint(
    hole_tiles: &[Tile],
    work_tiles: &[Tile],
) -> Result<(), CanonicalWireError> {
    validate_tiles(work_tiles, false, "hole_work_footprint")?;
    if work_tiles.len() != 9 {
        return Err(CanonicalWireError::InvalidBounds("hole_work_footprint"));
    }
    let min_x = hole_tiles
        .iter()
        .map(|tile| tile.x)
        .min()
        .unwrap_or_default();
    let min_y = hole_tiles
        .iter()
        .map(|tile| tile.y)
        .min()
        .unwrap_or_default();
    let expected: BTreeSet<_> = (1..=3)
        .flat_map(|y| (1..=3).map(move |x| (min_x + x, min_y + y)))
        .collect();
    let actual: BTreeSet<_> = work_tiles.iter().map(|tile| (tile.x, tile.y)).collect();
    if actual != expected {
        return Err(CanonicalWireError::InvalidBounds("hole_work_footprint"));
    }
    Ok(())
}
fn validate_colony(colony: &CanonicalColonySnapshot) -> Result<(), CanonicalWireError> {
    bounded_len(colony.plans.len(), MAX_CANONICAL_ITEMS, "plans")?;
    bounded_len(
        colony.officer_requests.len(),
        MAX_CANONICAL_ITEMS,
        "officer_requests",
    )?;
    bounded_len(
        colony.standing_order_capabilities.len(),
        MAX_CANONICAL_ITEMS,
        "standing_order_capabilities",
    )?;
    bounded_len(
        colony.standing_orders.len(),
        MAX_CANONICAL_ITEMS,
        "standing_orders",
    )?;
    bounded_len(colony.tasks.len(), MAX_CANONICAL_TASKS, "tasks")?;
    bounded_len(colony.cats.len(), MAX_CANONICAL_ITEMS, "cats")?;
    bounded_len(
        colony.job_assignments.len(),
        MAX_CANONICAL_ITEMS,
        "job_assignments",
    )?;
    bounded_len(colony.residences.len(), MAX_CANONICAL_ITEMS, "residences")?;
    bounded_len(
        colony.construction.len(),
        MAX_CANONICAL_ITEMS,
        "construction",
    )?;
    bounded_len(
        colony.storage_zones.len(),
        MAX_CANONICAL_ITEMS,
        "storage_zones",
    )?;
    bounded_len(colony.diagnostics.len(), MAX_CANONICAL_ITEMS, "diagnostics")?;
    bounded_len(colony.event_log.len(), MAX_CANONICAL_ITEMS, "event_log")?;
    bounded_len(
        colony.quality_lots.len(),
        MAX_CANONICAL_ITEMS,
        "quality_lots",
    )?;
    bounded_len(colony.exact_items.len(), MAX_CANONICAL_ITEMS, "exact_items")?;
    bounded_len(colony.food_stocks.len(), MAX_CANONICAL_ITEMS, "food_stocks")?;
    bounded_len(
        colony.hunting_sites.len(),
        MAX_CANONICAL_ITEMS,
        "hunting_sites",
    )?;
    bounded_len(
        colony.rare_materials.len(),
        MAX_CANONICAL_ITEMS,
        "rare_materials",
    )?;
    bounded_len(
        colony.augmentations.len(),
        MAX_CANONICAL_ITEMS,
        "augmentations",
    )?;
    bounded_len(colony.fixtures.len(), MAX_CANONICAL_ITEMS, "fixtures")?;
    bounded_len(
        colony.cookhouse_batches.len(),
        MAX_CANONICAL_ITEMS,
        "cookhouse_batches",
    )?;
    bounded_len(
        colony.fishing_huts.len(),
        MAX_CANONICAL_ITEMS,
        "fishing_huts",
    )?;
    bounded_len(
        colony.visual_states.len(),
        MAX_CANONICAL_ITEMS,
        "visual_states",
    )?;
    bounded_len(
        colony.divine.boost_offers.len(),
        MAX_CANONICAL_ITEMS,
        "divine_boost_offers",
    )?;
    bounded_len(
        colony.divine.construction_miracle_offers.len(),
        MAX_CANONICAL_ITEMS,
        "construction_miracle_offers",
    )?;
    bounded_len(
        colony.divine.rescue_offers.len(),
        2,
        "emergency_rescue_offers",
    )?;
    ordered_lanes(&colony.versions)?;
    ordered_ids(colony.plans.iter().map(|x| &x.plan_id), "plan_ids")?;
    ordered_ids(
        colony.officer_requests.iter().map(|x| &x.request_id),
        "officer_requests",
    )?;
    ordered_ids(
        colony
            .standing_order_capabilities
            .iter()
            .map(|x| &x.capability_id),
        "standing_order_capabilities",
    )?;
    ordered_ids(
        colony.standing_orders.iter().map(|x| &x.order_id),
        "standing_orders",
    )?;
    ordered_ids(colony.tasks.iter().map(|x| &x.task_id), "task_ids")?;
    ordered_ids(colony.cats.iter().map(|x| &x.cat_id), "cat_ids")?;
    ordered_ids(
        colony
            .job_assignments
            .iter()
            .map(|item| &item.assignment_id),
        "job_assignments",
    )?;
    ordered_ids(
        colony.residences.iter().map(|item| &item.residence_id),
        "residences",
    )?;
    ordered_ids(
        colony.construction.iter().map(|x| &x.project_id),
        "construction_projects",
    )?;
    ordered_ids(
        colony.storage_zones.iter().map(|x| &x.zone_id),
        "storage_zones",
    )?;
    ordered_ids(
        colony.diagnostics.iter().map(|x| &x.diagnostic_id),
        "diagnostics",
    )?;
    ordered_ids(
        colony.event_log.iter().map(|item| &item.event_id),
        "event_log",
    )?;
    ordered_ids(
        colony.quality_lots.iter().map(|item| &item.lot_id),
        "quality_lots",
    )?;
    ordered_ids(
        colony.exact_items.iter().map(|item| &item.item_id),
        "exact_items",
    )?;
    ordered_ids(
        colony.food_stocks.iter().map(|item| &item.lot_id),
        "food_stocks",
    )?;
    ordered_ids(
        colony.hunting_sites.iter().map(|item| &item.site_id),
        "hunting_sites",
    )?;
    ordered_ids(
        colony
            .rare_materials
            .iter()
            .map(|item| &item.material_instance_id),
        "rare_materials",
    )?;
    ordered_ids(
        colony
            .augmentations
            .iter()
            .map(|item| &item.augmentation_instance_id),
        "augmentations",
    )?;
    ordered_ids(
        colony.fixtures.iter().map(|item| &item.fixture_instance_id),
        "fixtures",
    )?;
    ordered_ids(
        colony.cookhouse_batches.iter().map(|item| &item.batch_id),
        "cookhouse_batches",
    )?;
    ordered_ids(
        colony.fishing_huts.iter().map(|item| &item.hut_id),
        "fishing_huts",
    )?;
    ordered_ids(
        colony.visual_states.iter().map(|item| &item.subject_id),
        "visual_states",
    )?;
    ordered_ids(
        colony.divine.active_boost_ids.iter(),
        "active_divine_boosts",
    )?;
    ordered_ids(
        colony
            .divine
            .boost_offers
            .iter()
            .map(|offer| &offer.offer_id),
        "divine_boost_offers",
    )?;
    ordered_ids(
        colony
            .divine
            .construction_miracle_offers
            .iter()
            .map(|offer| &offer.offer_id),
        "construction_miracle_offers",
    )?;
    ordered_ids(
        colony
            .divine
            .rescue_offers
            .iter()
            .map(|offer| &offer.witness_id),
        "emergency_rescue_offers",
    )?;
    if colony.divine.boost_offers.iter().any(|offer| {
        offer.duration_game_hours == 0
            || offer.exact_cost_micro_void == 0
            || offer.effect_basis_points <= 0
    }) || colony
        .divine
        .construction_miracle_offers
        .iter()
        .any(|offer| {
            offer.exact_cost_micro_void == 0
                || offer.labor_reduction_basis_points != 1_000
                || offer.input_value_multiplier_basis_points != 20_000
                || offer.footprint.ordered_tiles.is_empty()
        })
        || colony
            .divine
            .rescue_offers
            .iter()
            .any(|offer| offer.quantity == 0 || offer.exact_cost_micro_void == 0)
        || colony.divine.rescue_available != !colony.divine.rescue_offers.is_empty()
    {
        return Err(CanonicalWireError::InvalidBounds("divine_offers"));
    }
    ordered_queue(&colony.research.god_queue, "god_research_queue")?;
    ordered_queue(&colony.research.leader_decisions, "leader_research_queue")?;
    if colony
        .plans
        .iter()
        .any(|plan| plan.priority_basis_points > 10_000)
    {
        return Err(CanonicalWireError::InvalidBounds("plan_priority"));
    }
    for plan in &colony.plans {
        bounded_len(
            plan.dependencies.len(),
            MAX_CANONICAL_ITEMS,
            "plan_dependencies",
        )?;
        ordered_ids(
            plan.dependencies.iter().map(|item| &item.plan_id),
            "plan_dependencies",
        )?;
    }
    for officer in &colony.governance.officers {
        bounded_len(
            officer.appointment_candidate_ids.len(),
            MAX_CANONICAL_ITEMS,
            "appointment_candidates",
        )?;
        ordered_ids(
            officer.appointment_candidate_ids.iter(),
            "appointment_candidates",
        )?;
    }
    bounded_len(
        colony.governance.candidates.len(),
        MAX_CANONICAL_ITEMS,
        "election_candidates",
    )?;
    ordered_ids(
        colony.governance.candidates.iter().map(|item| &item.cat_id),
        "election_candidates",
    )?;
    ordered_ids(
        colony
            .governance
            .officers
            .iter()
            .map(|item| &item.office_id),
        "officers",
    )?;
    bounded_len(
        colony.research.god_queue.len(),
        MAX_CANONICAL_ITEMS,
        "god_research_queue",
    )?;
    bounded_len(
        colony.research.leader_decisions.len(),
        MAX_CANONICAL_ITEMS,
        "leader_research_queue",
    )?;
    bounded_len(
        colony.research.preparations.len(),
        MAX_CANONICAL_ITEMS,
        "research_preparations",
    )?;
    ordered_ids(
        colony
            .research
            .preparations
            .iter()
            .map(|item| &item.preparation_id),
        "research_preparations",
    )?;
    if colony
        .research
        .god_queue
        .iter()
        .chain(colony.research.leader_decisions.iter())
        .any(|entry| entry.progress_basis_points > 10_000)
        || colony.research.preparations.iter().any(|entry| {
            entry.progress_basis_points > 10_000 || entry.player_discount_basis_points > 10_000
        })
    {
        return Err(CanonicalWireError::InvalidBounds("research_basis_points"));
    }
    for task in &colony.tasks {
        validate_tiles(&task.footprint.ordered_tiles, false, "task_footprint")?;
        match task.task_kind_id.as_str() {
            "hunt" if task.site_kind_id.as_str() != "resource_source:hunting" => {
                return Err(CanonicalWireError::InvalidBounds("hunt_objective_kind"));
            }
            "fetch_water" if task.site_kind_id.as_str() != "resource_source:water" => {
                return Err(CanonicalWireError::InvalidBounds("water_objective_kind"));
            }
            "workshop_work"
                if task.site_kind_id.as_str() != "building:workshop"
                    || !is_exact_three_by_three(&task.footprint.ordered_tiles) =>
            {
                return Err(CanonicalWireError::InvalidBounds(
                    "workshop_objective_footprint",
                ));
            }
            _ => {}
        }
        bounded_len(
            task.work_sites.len(),
            MAX_CANONICAL_ITEMS,
            "task_work_sites",
        )?;
        if task.work_sites.windows(2).any(|pair| {
            (
                &pair[0].site_id,
                pair[0].slot_id.as_ref().map(StableId::as_str),
            ) >= (
                &pair[1].site_id,
                pair[1].slot_id.as_ref().map(StableId::as_str),
            )
        }) {
            return Err(CanonicalWireError::DuplicateOrUnordered("task_work_sites"));
        }
        for site in &task.work_sites {
            if site.slot_id.is_none() {
                return Err(CanonicalWireError::InvalidBounds("task_work_slot"));
            }
            validate_tiles(&site.footprint.ordered_tiles, false, "task_work_footprint")?;
        }
        if let Some(site) = &task.delivery_site {
            if site.slot_id.is_some() {
                return Err(CanonicalWireError::InvalidBounds("task_delivery_slot"));
            }
            validate_tiles(
                &site.footprint.ordered_tiles,
                false,
                "task_delivery_footprint",
            )?;
        }
        if task.task_kind_id.as_str() == "fetch_water"
            && (task.work_sites.is_empty()
                || task.work_sites.iter().any(|site| {
                    site.site_id == task.site_id
                        || footprints_overlap(&site.footprint, &task.footprint)
                }))
        {
            return Err(CanonicalWireError::InvalidBounds("water_bank_work_site"));
        }
        validate_tiles(&task.route.ordered_tiles, true, "task_route")?;
        validate_cargo(&task.cargo, "task_cargo")?;
        bounded_len(
            task.reservations.len(),
            MAX_CANONICAL_ITEMS,
            "task_reservations",
        )?;
        ordered_ids(
            task.reservations.iter().map(|x| &x.reservation_id),
            "task_reservations",
        )?;
        bounded_len(
            task.worker_cat_ids.len(),
            MAX_CANONICAL_ITEMS,
            "task_workers",
        )?;
        ordered_ids(task.worker_cat_ids.iter(), "task_workers")?;
        bounded_len(task.refusals.len(), MAX_CANONICAL_ITEMS, "task_refusals")?;
        ordered_ids(
            task.refusals.iter().map(|item| &item.refusal_id),
            "task_refusals",
        )?;
        bounded_len(
            task.anatomy_requirements.len(),
            MAX_CANONICAL_ITEMS,
            "task_anatomy_requirements",
        )?;
        ordered_ids(
            task.anatomy_requirements.iter(),
            "task_anatomy_requirements",
        )?;
        bounded_len(task.blockers.len(), MAX_CANONICAL_ITEMS, "task_blockers")?;
        ordered_ids(
            task.blockers.iter().map(|item| &item.blocker_id),
            "task_blockers",
        )?;
        if task.cargo.iter().any(|cargo| cargo.quality_band > 4) {
            return Err(CanonicalWireError::InvalidBounds("task_cargo_quality"));
        }
    }
    for cat in &colony.cats {
        bounded_len(cat.attributes.len(), MAX_CANONICAL_ITEMS, "cat_attributes")?;
        bounded_len(cat.skills.len(), MAX_CANONICAL_ITEMS, "cat_skills")?;
        bounded_len(cat.affinities.len(), MAX_CANONICAL_ITEMS, "cat_affinities")?;
        bounded_len(
            cat.anatomy_eligibility.len(),
            MAX_CANONICAL_ITEMS,
            "cat_anatomy_eligibility",
        )?;
        ordered_ids(
            cat.attributes.iter().map(|item| &item.attribute_id),
            "cat_attributes",
        )?;
        ordered_ids(cat.skills.iter().map(|item| &item.skill_id), "cat_skills")?;
        ordered_ids(
            cat.affinities.iter().map(|item| &item.labor_id),
            "cat_affinities",
        )?;
        ordered_ids(cat.anatomy_eligibility.iter(), "cat_anatomy_eligibility")?;
        ordered_ids(cat.family.parent_ids.iter(), "cat_parent_ids")?;
        ordered_ids(cat.family.child_ids.iter(), "cat_child_ids")?;
        if cat.attributes.iter().any(|attribute| {
            attribute
                .inherited_value
                .checked_add(attribute.learned_value)
                != Some(attribute.total_value)
        }) {
            return Err(CanonicalWireError::InvalidBounds("cat_attributes"));
        }
    }
    for residence in &colony.residences {
        validate_tiles(
            &residence.footprint.ordered_tiles,
            false,
            "residence_footprint",
        )?;
        bounded_len(
            residence.resident_cat_ids.len(),
            MAX_CANONICAL_ITEMS,
            "residence_residents",
        )?;
        ordered_ids(residence.resident_cat_ids.iter(), "residence_residents")?;
        if residence.capacity == 0
            || residence.resident_cat_ids.len() > usize::from(residence.capacity)
            || residence.housing_pressure_basis_points > 10_000
        {
            return Err(CanonicalWireError::InvalidBounds("residence"));
        }
    }
    for event in &colony.event_log {
        bounded_len(event.source_ids.len(), MAX_CANONICAL_ITEMS, "event_sources")?;
        ordered_ids(event.source_ids.iter(), "event_sources")?;
        if event.repeated_count == 0 {
            return Err(CanonicalWireError::InvalidBounds("event_log"));
        }
    }
    for project in &colony.construction {
        validate_tiles(
            &project.footprint.ordered_tiles,
            false,
            "construction_footprint",
        )?;
        if project.phase_progress_basis_points > 10_000 || project.stage_cargo.len() > 3 {
            return Err(CanonicalWireError::InvalidBounds("construction"));
        }
        for cargo in &project.stage_cargo {
            let expected_share = match cargo.phase {
                ConstructionPhase::Scaffold | ConstructionPhase::FitOut => 2_000,
                ConstructionPhase::Structure => 6_000,
                _ => return Err(CanonicalWireError::InvalidBounds("construction_phase")),
            };
            if cargo.work_share_basis_points != expected_share {
                return Err(CanonicalWireError::InvalidBounds("construction_work_share"));
            }
            validate_cargo(&cargo.delivered, "construction_delivered")?;
            validate_cargo(&cargo.in_transit, "construction_in_transit")?;
            validate_cargo(&cargo.consumed, "construction_consumed")?;
        }
    }
    let exact_item_ids = colony
        .exact_items
        .iter()
        .map(|item| item.item_id.as_str())
        .collect::<BTreeSet<_>>();
    for zone in &colony.storage_zones {
        if zone.tiles.len() > MAX_CANONICAL_ITEMS
            || zone.lots.len() > MAX_CANONICAL_ITEMS
            || zone.containers.len() > MAX_CANONICAL_ITEMS
            || zone.tiles.iter().any(|tile| tile.slots.len() > 4)
        {
            return Err(CanonicalWireError::InvalidBounds("storage_zone"));
        }
        validate_tiles(&zone.footprint.ordered_tiles, false, "storage_footprint")?;
        let footprint: BTreeSet<_> = zone
            .footprint
            .ordered_tiles
            .iter()
            .map(|tile| (tile.x, tile.y))
            .collect();
        let mut storage_tiles = BTreeSet::new();
        let mut storage_slot_ids = BTreeSet::new();
        let zone_lot_ids = zone
            .lots
            .iter()
            .map(|lot| lot.cargo_id.as_str())
            .collect::<BTreeSet<_>>();
        let zone_container_ids = zone
            .containers
            .iter()
            .map(|container| container.container_id.as_str())
            .collect::<BTreeSet<_>>();
        for tile in &zone.tiles {
            if !footprint.contains(&(tile.tile.x, tile.tile.y))
                || !storage_tiles.insert((tile.tile.x, tile.tile.y))
                || tile.slots.iter().any(|slot| {
                    slot.fullness_basis_points > 10_000
                        || usize::from(slot.lot_id.is_some())
                            + usize::from(slot.item_id.is_some())
                            + usize::from(slot.container_id.is_some())
                            > 1
                        || (slot.lot_id.is_none()
                            && slot.item_id.is_none()
                            && slot.container_id.is_none()
                            && slot.fullness_basis_points != 0)
                        || ((slot.lot_id.is_some() || slot.item_id.is_some())
                            && slot.fullness_basis_points != 10_000)
                })
            {
                return Err(CanonicalWireError::InvalidBounds("storage_tiles"));
            }
            ordered_ids(tile.slots.iter().map(|slot| &slot.slot_id), "storage_slots")?;
            for slot in &tile.slots {
                if !storage_slot_ids.insert(slot.slot_id.as_str())
                    || slot
                        .lot_id
                        .as_ref()
                        .is_some_and(|id| !zone_lot_ids.contains(id.as_str()))
                    || slot
                        .item_id
                        .as_ref()
                        .is_some_and(|id| !exact_item_ids.contains(id.as_str()))
                    || slot
                        .container_id
                        .as_ref()
                        .is_some_and(|id| !zone_container_ids.contains(id.as_str()))
                {
                    return Err(CanonicalWireError::InvalidBounds("storage_slot_identity"));
                }
            }
        }
        ordered_ids(
            zone.containers.iter().map(|item| &item.container_id),
            "storage_containers",
        )?;
        if zone
            .containers
            .iter()
            .any(|item| item.capacity_slots == 0 || item.fullness_basis_points > 10_000)
        {
            return Err(CanonicalWireError::InvalidBounds("storage_containers"));
        }
        validate_cargo(&zone.lots, "storage_lots")?;
    }
    if colony.hole.width > 10 || colony.hole.depth > 10 || colony.hole.darkness > 10 {
        return Err(CanonicalWireError::InvalidBounds("hole"));
    }
    validate_exact_hole_footprint(&colony.hole.footprint.ordered_tiles)?;
    validate_hole_work_footprint(
        &colony.hole.footprint.ordered_tiles,
        &colony.hole.work_footprint.ordered_tiles,
    )?;
    bounded_len(
        colony.hole.food_permissions.len(),
        MAX_CANONICAL_ITEMS,
        "food_permissions",
    )?;
    ordered_ids(
        colony
            .hole
            .food_permissions
            .iter()
            .map(|item| &item.content_id),
        "food_permissions",
    )?;
    bounded_len(
        colony.hole.contribution_receipts.len(),
        MAX_CANONICAL_ITEMS,
        "contribution_receipts",
    )?;
    ordered_ids(
        colony.hole.contribution_receipts.iter(),
        "contribution_receipts",
    )?;
    if !(1..=5).contains(&colony.hole.officer_report_level) {
        return Err(CanonicalWireError::InvalidBounds("officer_report_level"));
    }
    if colony.hole.officer_report_level < 4
        && (!matches!(colony.hole.regeneration, RegenerationReport::Unavailable)
            || colony.hole.officer_reported_regeneration.is_some())
    {
        return Err(CanonicalWireError::InvalidBounds("regeneration"));
    }
    if matches!(
        colony.hole.regeneration,
        RegenerationReport::OfficerReportedEstimate
    ) != colony.hole.officer_reported_regeneration.is_some()
    {
        return Err(CanonicalWireError::InvalidBounds("regeneration"));
    }
    if colony
        .hole
        .officer_reported_regeneration
        .as_ref()
        .is_some_and(|estimate| {
            estimate.lower_units_per_day >= estimate.upper_units_per_day
                || estimate.upper_units_per_day > 1_000_000_000
        })
    {
        return Err(CanonicalWireError::InvalidBounds("regeneration_estimate"));
    }
    bounded_len(
        colony.diplomacy.stances.len(),
        MAX_CANONICAL_ITEMS,
        "diplomacy_stances",
    )?;
    ordered_ids(
        colony
            .diplomacy
            .stances
            .iter()
            .map(|item| &item.other_colony_id),
        "diplomacy_stances",
    )?;
    bounded_len(
        colony.diplomacy.contracts.len(),
        MAX_CANONICAL_ITEMS,
        "trade_contracts",
    )?;
    ordered_ids(
        colony
            .diplomacy
            .contracts
            .iter()
            .map(|item| &item.contract_id),
        "trade_contracts",
    )?;
    for contract in &colony.diplomacy.contracts {
        validate_tiles(&contract.route.ordered_tiles, true, "trade_route")?;
        bounded_len(contract.escrow.len(), MAX_CANONICAL_ITEMS, "trade_escrow")?;
        ordered_ids(
            contract.escrow.iter().map(|item| &item.cargo_id),
            "trade_escrow",
        )?;
        if contract
            .escrow
            .iter()
            .any(|item| item.quantity == 0 || item.quality_band > 4)
        {
            return Err(CanonicalWireError::InvalidBounds("trade_escrow"));
        }
    }
    if let Some(manifest) = &colony.content_manifest {
        if manifest.manifest_version == 0 {
            return Err(CanonicalWireError::InvalidBounds("content_manifest"));
        }
        bounded_len(
            manifest.entries.len(),
            MAX_CANONICAL_ITEMS,
            "content_manifest",
        )?;
        ordered_ids(
            manifest.entries.iter().map(|item| &item.content_id),
            "content_manifest",
        )?;
        for entry in &manifest.entries {
            bounded_len(
                entry.capability_ids.len(),
                MAX_CANONICAL_ITEMS,
                "content_capabilities",
            )?;
            ordered_ids(entry.capability_ids.iter(), "content_capabilities")?;
        }
    }
    if colony.quality_lots.iter().any(|lot| lot.quantity == 0)
        || colony.exact_items.iter().any(|item| {
            item.durability_basis_points > 10_000
                || item.augmentation_ids.len() > MAX_CANONICAL_ITEMS
        })
        || colony.food_stocks.iter().any(|food| {
            food.quantity == 0
                || food.nutrition_basis_points > 20_000
                || food.spoilage_basis_points > 10_000
        })
    {
        return Err(CanonicalWireError::InvalidBounds("physical_content"));
    }
    for item in &colony.exact_items {
        ordered_ids(item.augmentation_ids.iter(), "item_augmentations")?;
    }
    for site in &colony.hunting_sites {
        bounded_len(
            site.creatures.len(),
            MAX_CANONICAL_ITEMS,
            "hunting_creatures",
        )?;
        bounded_len(
            site.cache_lot_ids.len(),
            MAX_CANONICAL_ITEMS,
            "hunting_cache_lots",
        )?;
        bounded_len(
            site.cache_item_ids.len(),
            MAX_CANONICAL_ITEMS,
            "hunting_cache_items",
        )?;
        ordered_ids(
            site.creatures.iter().map(|item| &item.creature_id),
            "hunting_creatures",
        )?;
        ordered_ids(site.cache_lot_ids.iter(), "hunting_cache_lots")?;
        ordered_ids(site.cache_item_ids.iter(), "hunting_cache_items")?;
        if site.level_band == 0
            || site.level_band > 10
            || site
                .creatures
                .iter()
                .any(|item| item.level_band == 0 || item.health_basis_points > 10_000)
        {
            return Err(CanonicalWireError::InvalidBounds("hunting_site"));
        }
    }
    for batch in &colony.cookhouse_batches {
        bounded_len(
            batch.ingredient_lot_ids.len(),
            MAX_CANONICAL_ITEMS,
            "cookhouse_ingredients",
        )?;
        bounded_len(
            batch.output_lot_ids.len(),
            MAX_CANONICAL_ITEMS,
            "cookhouse_outputs",
        )?;
        ordered_ids(batch.ingredient_lot_ids.iter(), "cookhouse_ingredients")?;
        ordered_ids(batch.output_lot_ids.iter(), "cookhouse_outputs")?;
        if batch.progress_basis_points > 10_000 {
            return Err(CanonicalWireError::InvalidBounds("cookhouse_progress"));
        }
    }
    for hut in &colony.fishing_huts {
        validate_tiles(&hut.footprint.ordered_tiles, false, "fishing_hut_footprint")?;
        if hut.footprint.ordered_tiles.len() != 9 || hut.progress_basis_points > 10_000 {
            return Err(CanonicalWireError::InvalidBounds("fishing_hut"));
        }
    }
    for visual in &colony.visual_states {
        validate_tiles(&visual.footprint.ordered_tiles, false, "visual_footprint")?;
    }
    Ok(())
}
