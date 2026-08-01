//! Canonical LAI.46/LAI.63 Leader/officer runtime aggregate.
//!
//! The policy is specified by
//! `docs/leader-ai-overhaul/final-hole-hunting-content-plan.md` and
//! `docs/leader-ai-overhaul/final-integrated-overhaul-plan.md`. This is the
//! persisted composition root for the canonical planner, cat/family/governance
//! authorities, the two research lanes, staged construction, physical storage,
//! The Hole/divine actions, moneyless barter, exact tasks, and bounded
//! diagnostics. The former Shrine/Favor and purchase/scholar/coin aggregates
//! are intentionally not compatibility fields here.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    acquired_traits::AcquiredTraitState,
    anatomy::CatAnatomy,
    beliefs::BeliefStore,
    black_hole::{BlackHoleState, HoleAxes},
    cat_capabilities::{CapabilityAttributes, LaborAffinity, LaborAffinityProfile},
    cat_capability_authority::{
        CatCapabilityAuthority, CatCapabilityRegistration, ProductiveOutcome,
        ProductiveOutcomeReceipt,
    },
    cat_governance::{
        CivicMeritMetrics, GovernanceLifeStage, RelationalAnalyticalAxis, SCORE_SCALE,
    },
    cat_stress::StressState,
    cat_traits::{CatAttributes, CatTraits, LegacyCatAttributes, generate_personality},
    cat_willingness::{RefusalReason, TaskPriority, WillingnessDecision},
    construction_catalog::{BlueprintRequest, resolve_blueprint},
    construction_miracle_runtime::ConstructionMiracleRuntimeState,
    construction_runtime::ConstructionMaterializationRecord,
    construction_stages::{
        ConstructionBills, ConstructionProject, ConstructionStageBill, ConstructionTargetKind,
    },
    content_manifest::{
        ConstructionMiracleInputClass, ContentId, ContentManifest, ItemClass, MaterialInstanceId,
        PhysicalLotId,
    },
    divine_action_offers::ReportedResidentNeedsSummary,
    divine_boosts::DivineBoostState,
    divine_hole_authority::{
        DivineHoleAuthority, HoleAuthorityBinding, VoidActionEnvelope, VoidActionOutcome,
    },
    entities::Cat,
    family_authority::{
        BirthRegistration, FamilyAuthorityState, FamilyBuilding, FamilyCommand, FamilyOperation,
        ProfessionalCompletion,
    },
    family_housing::LifeStage as FamilyLifeStage,
    food_divine_policy::{BoundCargoPurpose, PurposeBoundCargo},
    governance_authority::{
        CandidateBallotFacts, GovernanceAuthorityState, GovernanceResidentFact,
    },
    intent_graph::IntentGraph,
    leader_ai_diagnostics::{Lai69ColonyId, Lai69DiagnosticsConfig, Lai69LeaderAiDiagnostics},
    leader_planner::content_planner::{
        ContentPlannerState, PlannerPhase, ReportSafePlanningInput, god_report_bytes,
        planner_report_bytes,
    },
    officer_requests::OfficerRequestBook,
    physical_storage::StorageCompatibility,
    planner_core::{PlannerId, PlannerRngStream, planner_roll},
    player_directives::PlayerDirectiveState,
    progression_research::{ResearchNotes, VoidInsight},
    prosthetics::ProstheticLedger,
    quality_lots::{
        BulkLotKey, ItemInstance, LotLocation, LotProvenance, PhysicalLot, QualityBand,
    },
    research_authority::ResearchAuthority,
    reservation_transaction::{
        ClaimMode, ClaimSpec, ReservationBundle, ReservationChecks, ReservationLedger,
    },
    scheduler::SchedulerState,
    skill_catalog::SkillProgress,
    skills::Labor,
    spatial_resolver::{ResolvedSpatialTask, SpatialTaskCategory},
    spatial_tasks::{SiteRef, TilePoint},
    storage_authority::{
        StorageAddress, StorageAuthority, StorageCommand, StorageCommandEnvelope, StorageIdentity,
    },
    task_runtime::{
        CargoLocation, RuntimeBlockReason, TaskCategory, TaskId, TaskStage, VisibleTaskRuntime,
    },
    trade_authority::TradeAuthority,
    types::LifeStage as LegacyLifeStage,
    workforce_matcher::{WorkforceEdge, WorkforceSlot, match_workforce},
    world_reservations::{
        CapacityReservation, WorldReservationId, WorldReservationLedger,
        WorldReservationTransaction, WorldReservationValidation,
    },
};

pub const LEADER_AI_RUNTIME_SCHEMA_VERSION: u32 = 2;
pub const MAX_VISIBLE_RUNTIME_TASKS: usize = 512;
pub const MAX_KNOWN_CARGO_SITES: usize = 2_048;
pub const MAX_CONSTRUCTION_PROJECTS: usize = 512;
pub const MAX_TASK_OUTCOME_BINDINGS: usize = 2_048;
pub const MAX_PHASE_RECEIPTS: usize = 512;
pub const MAX_CAT_PHYSICAL_STATES: usize = 4_096;
pub const DEFAULT_RUNTIME_COLONY_ID: &str = "leader-ai-runtime-default";
pub const DEFAULT_HOLE_ID: &str = "black_hole_main";
pub const MAX_PHYSICAL_TASK_WORKER_REPORTS: usize = 4_096;

/// Report-safe worker facts accepted by the physical-task executor. The
/// runtime intentionally receives no position, need, hidden inventory, or
/// unredacted capability truth here: callers must have already constructed
/// this bounded report projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalTaskWorkerReport {
    pub cat_id: String,
    pub alive: bool,
    pub capable: bool,
    pub willing: bool,
    /// Larger is better. Ties are resolved by the canonical matcher and stable
    /// cat ID, never incoming report order.
    pub suitability_score: i64,
}

/// One indivisible physical storage identity moved by a visible task. Partial
/// bulk-lot hauling and exact item hauling remain deliberately typed-blocked
/// until their own split/item transport authority is wired into the root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalTaskCargoBinding {
    pub identity: StorageIdentity,
    pub resource_id: String,
    pub quantity: u64,
    pub source: StorageAddress,
    pub endpoint: StorageAddress,
    /// An already-authorized physical recovery address. `None` means cargo
    /// stays at its exact route location as a stranded identity; it never
    /// moves to an invented fallback tile or inventory.
    pub recovery: Option<StorageAddress>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalTaskWorkReceipt {
    pub outcome: ProductiveOutcome,
    pub family_completion: Option<ProfessionalCompletion>,
}

/// External interruption facts. These are deliberately explicit so the task
/// executor neither reads hidden world truth nor conflates a cancelled task
/// with a carried-cargo loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalTaskInterruption {
    None,
    WorkerRefused,
    WorkerDied,
    WorkerIncapacitated,
    RouteLost,
    SourceLost,
    EndpointLost,
    Cancelled,
    SurvivalPreemption,
    DefensePreemption,
    VillagePreemption,
}

impl PhysicalTaskInterruption {
    const fn is_preemption(self) -> bool {
        matches!(
            self,
            Self::SurvivalPreemption | Self::DefensePreemption | Self::VillagePreemption
        )
    }

    const fn runtime_block_reason(self) -> RuntimeBlockReason {
        match self {
            Self::WorkerRefused => RuntimeBlockReason::WorkerRefused,
            Self::WorkerDied => RuntimeBlockReason::WorkerDied,
            Self::WorkerIncapacitated => RuntimeBlockReason::WorkerIncapacitated,
            Self::RouteLost => RuntimeBlockReason::RouteClosedWithCargo,
            Self::SourceLost => RuntimeBlockReason::SourceRemoved,
            Self::EndpointLost => RuntimeBlockReason::EndpointRemoved,
            Self::Cancelled
            | Self::SurvivalPreemption
            | Self::DefensePreemption
            | Self::VillagePreemption
            | Self::None => RuntimeBlockReason::RouteClosedBeforePickup,
        }
    }

    const fn pre_pickup_block_reason(self) -> RuntimeBlockReason {
        match self {
            Self::RouteLost => RuntimeBlockReason::RouteClosedBeforePickup,
            _ => self.runtime_block_reason(),
        }
    }
}

/// The exact physical facts the hot root must provide after it projects a
/// canonical planner task. `resolved` is the only source of source/work/route
/// geometry; this leaf will reject rather than repair a mismatching task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalTaskExecutionRequest {
    pub task_id: TaskId,
    pub resolved: ResolvedSpatialTask,
    pub cargo: PhysicalTaskCargoBinding,
    pub workers: Vec<PhysicalTaskWorkerReport>,
    pub priority: TaskPriority,
    pub interruption: PhysicalTaskInterruption,
    pub work: PhysicalTaskWorkReceipt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalTaskBlockReason {
    NoLivingCapableWorker,
    NoWillingWorker,
    ReservationUnavailable,
    CargoUnavailable,
    UnsupportedTaskStage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhysicalTaskExecutionOutcome {
    Activated { cat_id: String },
    Advanced { stage: TaskStage },
    Worked { cat_id: String },
    Completed,
    Recovered { stranded: bool },
    Cancelled,
    Preempted(PhysicalTaskInterruption),
    Blocked(PhysicalTaskBlockReason),
    Terminal(TaskStage),
}

/// The protected once-per-game-minute phase order. The ten internal planner
/// phases occur wholly inside `LeaderOfficerReview`; the surrounding runtime
/// phases may not interleave another planner or mutation path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtectedRuntimePhase {
    AuthorityAndNeeds,
    ReportAndBeliefObservation,
    LeaderOfficerReview,
    ExactSitesAndReservations,
    WorkforceMatching,
    VisibleTaskMovementCargo,
    HoleDivineAndPurposeCargo,
    UnifiedResearch,
    PersonalStanceAndPhysicalBarter,
    StressAndInjury,
    ProjectionAndDiagnostics,
}

impl ProtectedRuntimePhase {
    pub const ORDER: [Self; 11] = [
        Self::AuthorityAndNeeds,
        Self::ReportAndBeliefObservation,
        Self::LeaderOfficerReview,
        Self::ExactSitesAndReservations,
        Self::WorkforceMatching,
        Self::VisibleTaskMovementCargo,
        Self::HoleDivineAndPurposeCargo,
        Self::UnifiedResearch,
        Self::PersonalStanceAndPhysicalBarter,
        Self::StressAndInjury,
        Self::ProjectionAndDiagnostics,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimePhaseReceipt {
    pub tick: u64,
    pub phases: Vec<ProtectedRuntimePhase>,
    pub planner_phases: Vec<PlannerPhase>,
    pub report_version: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskOutcomeBinding {
    pub task_id: TaskId,
    pub cat_id: String,
    pub outcome: ProductiveOutcome,
    pub capability_receipt_id: String,
    pub family_receipt_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatPhysicalState {
    pub cat_id: String,
    pub traits: CatTraits,
    pub acquired_traits: AcquiredTraitState,
    pub stress: StressState,
    pub anatomy: CatAnatomy,
    pub death_processed_tick: Option<u64>,
}

impl CatPhysicalState {
    fn from_legacy(world_seed: u32, colony_id: &str, cat: &Cat) -> Self {
        let legacy = LegacyCatAttributes {
            attack: rounded_legacy_stat(cat.stats.attack),
            defense: rounded_legacy_stat(cat.stats.defense),
            hunting: rounded_legacy_stat(cat.stats.hunting),
            medicine: rounded_legacy_stat(cat.stats.medicine),
            cleaning: rounded_legacy_stat(cat.stats.cleaning),
            building: rounded_legacy_stat(cat.stats.building),
            leadership: rounded_legacy_stat(cat.stats.leadership),
            vision: rounded_legacy_stat(cat.stats.vision),
        };
        Self {
            cat_id: cat.id.clone(),
            traits: CatTraits {
                attributes: CatAttributes::from_legacy_0_to_100(legacy),
                personality: generate_personality(world_seed, colony_id, &cat.id),
            },
            acquired_traits: AcquiredTraitState::default(),
            stress: StressState::default(),
            anatomy: CatAnatomy::default(),
            death_processed_tick: None,
        }
    }
}

/// Exact task/reservation adapter. It owns no strategy: every row is derived
/// from a canonical planner goal and points to one pinned world objective.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SchedulingRuntimeAggregate {
    pub scheduler: SchedulerState,
    pub reservations: ReservationLedger,
    pub world_reservations: WorldReservationLedger,
    pub visible_tasks: BTreeMap<TaskId, VisibleTaskRuntime>,
    pub resolved_spatial_tasks: BTreeMap<TaskId, ResolvedSpatialTask>,
    pub world_reservation_ids: BTreeMap<TaskId, WorldReservationId>,
    pub task_storage_identities: BTreeMap<TaskId, StorageIdentity>,
    /// The exact physical address selected for an activated task's delivery
    /// endpoint. Older snapshots may omit it; those tasks remain observable
    /// but are not advanced by the strict physical executor.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub task_storage_endpoints: BTreeMap<TaskId, StorageAddress>,
    pub known_cargo_site_ids: BTreeSet<String>,
}

impl SchedulingRuntimeAggregate {
    #[must_use]
    pub fn new() -> Self {
        Self {
            scheduler: SchedulerState::new(),
            reservations: ReservationLedger::new(),
            world_reservations: WorldReservationLedger::new(),
            visible_tasks: BTreeMap::new(),
            resolved_spatial_tasks: BTreeMap::new(),
            world_reservation_ids: BTreeMap::new(),
            task_storage_identities: BTreeMap::new(),
            task_storage_endpoints: BTreeMap::new(),
            known_cargo_site_ids: BTreeSet::new(),
        }
    }

    fn validate(
        &self,
        intents: &IntentGraph,
        storage: &StorageAuthority,
    ) -> Result<(), LeaderAiRuntimeError> {
        if self.visible_tasks.len() > MAX_VISIBLE_RUNTIME_TASKS
            || self.resolved_spatial_tasks.len() > MAX_VISIBLE_RUNTIME_TASKS
            || self.world_reservation_ids.len() > MAX_VISIBLE_RUNTIME_TASKS
            || self.task_storage_identities.len() > MAX_VISIBLE_RUNTIME_TASKS
            || self.task_storage_endpoints.len() > MAX_VISIBLE_RUNTIME_TASKS
            || self.known_cargo_site_ids.len() > MAX_KNOWN_CARGO_SITES
            || self.known_cargo_site_ids.iter().any(String::is_empty)
        {
            return Err(LeaderAiRuntimeError::BoundExceeded);
        }
        validate_leaf("scheduler", &self.scheduler)?;
        validate_leaf("reservations", &self.reservations)?;
        validate_leaf("world reservations", &self.world_reservations)?;
        for (task_id, task) in &self.visible_tasks {
            if task_id != &task.id {
                return Err(LeaderAiRuntimeError::TaskIdMismatch);
            }
            validate_leaf("visible task", task)?;
            if intents.get(&task.intent_id).is_none() {
                return Err(LeaderAiRuntimeError::DanglingTaskIntent);
            }
            if task
                .reservation_id
                .as_ref()
                .is_some_and(|id| !self.reservations.contains(id))
            {
                return Err(LeaderAiRuntimeError::DanglingTaskReservation);
            }
            validate_cargo_reference(task, &self.known_cargo_site_ids)?;
            if task.cargo.is_some() {
                let identity = self
                    .task_storage_identities
                    .get(task_id)
                    .ok_or(LeaderAiRuntimeError::DanglingStorageIdentity)?;
                if storage.location(identity).is_none() {
                    return Err(LeaderAiRuntimeError::DanglingStorageIdentity);
                }
                if self
                    .task_storage_endpoints
                    .get(task_id)
                    .is_some_and(|endpoint| matches!(endpoint, StorageAddress::RouteCargo { .. }))
                {
                    return Err(LeaderAiRuntimeError::DanglingStorageIdentity);
                }
                if self
                    .task_storage_endpoints
                    .get(task_id)
                    .is_some_and(|endpoint| !match (
                        task.spatial.delivery_endpoint.as_ref(),
                        endpoint,
                    ) {
                        (
                            Some(SiteRef::Stockpile { stockpile_id, .. }),
                            StorageAddress::Loose { zone_id, .. },
                        ) => stockpile_id == zone_id,
                        (Some(delivery), StorageAddress::PurposeCargo { site_id }) => {
                            delivery.stable_id() == site_id
                        }
                        _ => false,
                    })
                {
                    return Err(LeaderAiRuntimeError::DanglingStorageIdentity);
                }
            }
        }
        for task_id in self.task_storage_identities.keys() {
            if !self
                .visible_tasks
                .get(task_id)
                .is_some_and(|task| task.cargo.is_some())
            {
                return Err(LeaderAiRuntimeError::DanglingStorageIdentity);
            }
        }
        for task_id in self.task_storage_endpoints.keys() {
            if !self
                .visible_tasks
                .get(task_id)
                .is_some_and(|task| task.cargo.is_some())
            {
                return Err(LeaderAiRuntimeError::DanglingStorageIdentity);
            }
        }
        for (task_id, resolved) in &self.resolved_spatial_tasks {
            let task = self
                .visible_tasks
                .get(task_id)
                .ok_or(LeaderAiRuntimeError::DanglingResolvedTask)?;
            if resolved.spatial != task.spatial || resolved.validate().is_err() {
                return Err(LeaderAiRuntimeError::DanglingResolvedTask);
            }
        }
        for (task_id, reservation_id) in &self.world_reservation_ids {
            if !self.visible_tasks.contains_key(task_id)
                || !self.world_reservations.contains(reservation_id)
            {
                return Err(LeaderAiRuntimeError::DanglingWorldReservation);
            }
        }
        Ok(())
    }

    /// Public cutover preflight used by the protected world-tick phase. Full
    /// cross-authority validation still occurs at transaction commit.
    pub fn validate_for_world_cutover(&self) -> Result<(), LeaderAiRuntimeError> {
        if self.visible_tasks.len() > MAX_VISIBLE_RUNTIME_TASKS
            || self.resolved_spatial_tasks.len() > MAX_VISIBLE_RUNTIME_TASKS
            || self.world_reservation_ids.len() > MAX_VISIBLE_RUNTIME_TASKS
            || self.task_storage_identities.len() > MAX_VISIBLE_RUNTIME_TASKS
            || self.task_storage_endpoints.len() > MAX_VISIBLE_RUNTIME_TASKS
            || self.known_cargo_site_ids.len() > MAX_KNOWN_CARGO_SITES
        {
            return Err(LeaderAiRuntimeError::BoundExceeded);
        }
        for (task_id, resolved) in &self.resolved_spatial_tasks {
            if !self.visible_tasks.contains_key(task_id) || resolved.validate().is_err() {
                return Err(LeaderAiRuntimeError::DanglingResolvedTask);
            }
        }
        Ok(())
    }
}

impl Default for SchedulingRuntimeAggregate {
    fn default() -> Self {
        Self::new()
    }
}

/// The single persisted LAI.63 runtime state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderAiRuntimeState {
    pub schema_version: u32,
    pub colony_id: String,
    pub colony_partition: PlannerId,
    pub last_processed_tick: Option<u64>,
    pub planner: ContentPlannerState,
    pub beliefs: BeliefStore,
    pub last_report: Option<ReportSafePlanningInput>,
    /// Latest canonical resident-needs observation available to both the
    /// Leader and God projection. Rescue witnesses bind this report version;
    /// the server never derives emergency entitlement from raw cat state.
    pub resident_needs_report_version: Option<u64>,
    pub resident_needs_summary: Option<ReportedResidentNeedsSummary>,
    pub officer_requests: OfficerRequestBook,
    pub intents: IntentGraph,
    pub scheduling: SchedulingRuntimeAggregate,
    pub cat_capabilities: CatCapabilityAuthority,
    pub families: FamilyAuthorityState,
    pub governance: GovernanceAuthorityState,
    pub research: ResearchAuthority,
    pub construction_projects: BTreeMap<String, ConstructionProject>,
    pub construction_storage_identities: BTreeMap<String, BTreeSet<StorageIdentity>>,
    /// Last canonical tick which advanced each staged construction project.
    /// This is the persisted restart/idempotency guard for the construction
    /// runtime bridge; entries never create a project or cargo identity.
    pub construction_runtime_ticks: BTreeMap<String, u64>,
    /// Restart-idempotent world projection receipt, or a typed fail-closed gap,
    /// for every operational construction project that reached materialization.
    pub construction_materializations: BTreeMap<String, ConstructionMaterializationRecord>,
    /// Restart-safe miracle receipts and not-yet-opened stage labor credits.
    pub construction_miracles: ConstructionMiracleRuntimeState,
    pub storage: StorageAuthority,
    pub hole: BlackHoleState,
    pub divine_hole: DivineHoleAuthority,
    pub purpose_bound_storage: BTreeMap<StorageIdentity, BoundCargoPurpose>,
    pub boosts: DivineBoostState,
    pub trade: TradeAuthority,
    pub cat_physical: BTreeMap<String, CatPhysicalState>,
    pub prosthetics: ProstheticLedger,
    pub player_directives: PlayerDirectiveState,
    pub task_outcomes: BTreeMap<TaskId, TaskOutcomeBinding>,
    pub phase_receipts: BTreeMap<u64, RuntimePhaseReceipt>,
    pub diagnostics: Lai69LeaderAiDiagnostics,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedLeaderAiRuntimeState {
    schema_version: u32,
    colony_id: String,
    colony_partition: PlannerId,
    last_processed_tick: Option<u64>,
    planner: ContentPlannerState,
    beliefs: BeliefStore,
    last_report: Option<ReportSafePlanningInput>,
    #[serde(default)]
    resident_needs_report_version: Option<u64>,
    #[serde(default)]
    resident_needs_summary: Option<ReportedResidentNeedsSummary>,
    officer_requests: OfficerRequestBook,
    intents: IntentGraph,
    scheduling: SchedulingRuntimeAggregate,
    cat_capabilities: CatCapabilityAuthority,
    families: FamilyAuthorityState,
    governance: GovernanceAuthorityState,
    research: ResearchAuthority,
    construction_projects: BTreeMap<String, ConstructionProject>,
    construction_storage_identities: BTreeMap<String, BTreeSet<StorageIdentity>>,
    #[serde(default)]
    construction_runtime_ticks: BTreeMap<String, u64>,
    #[serde(default)]
    construction_materializations: BTreeMap<String, ConstructionMaterializationRecord>,
    #[serde(default)]
    construction_miracles: ConstructionMiracleRuntimeState,
    storage: StorageAuthority,
    hole: BlackHoleState,
    divine_hole: DivineHoleAuthority,
    purpose_bound_storage: BTreeMap<StorageIdentity, BoundCargoPurpose>,
    boosts: DivineBoostState,
    trade: TradeAuthority,
    cat_physical: BTreeMap<String, CatPhysicalState>,
    prosthetics: ProstheticLedger,
    player_directives: PlayerDirectiveState,
    task_outcomes: BTreeMap<TaskId, TaskOutcomeBinding>,
    phase_receipts: BTreeMap<u64, RuntimePhaseReceipt>,
    diagnostics: Lai69LeaderAiDiagnostics,
}

impl<'de> Deserialize<'de> for LeaderAiRuntimeState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = UncheckedLeaderAiRuntimeState::deserialize(deserializer)?;
        let state = Self {
            schema_version: raw.schema_version,
            colony_id: raw.colony_id,
            colony_partition: raw.colony_partition,
            last_processed_tick: raw.last_processed_tick,
            planner: raw.planner,
            beliefs: raw.beliefs,
            last_report: raw.last_report,
            resident_needs_report_version: raw.resident_needs_report_version,
            resident_needs_summary: raw.resident_needs_summary,
            officer_requests: raw.officer_requests,
            intents: raw.intents,
            scheduling: raw.scheduling,
            cat_capabilities: raw.cat_capabilities,
            families: raw.families,
            governance: raw.governance,
            research: raw.research,
            construction_projects: raw.construction_projects,
            construction_storage_identities: raw.construction_storage_identities,
            construction_runtime_ticks: raw.construction_runtime_ticks,
            construction_materializations: raw.construction_materializations,
            construction_miracles: raw.construction_miracles,
            storage: raw.storage,
            hole: raw.hole,
            divine_hole: raw.divine_hole,
            purpose_bound_storage: raw.purpose_bound_storage,
            boosts: raw.boosts,
            trade: raw.trade,
            cat_physical: raw.cat_physical,
            prosthetics: raw.prosthetics,
            player_directives: raw.player_directives,
            task_outcomes: raw.task_outcomes,
            phase_receipts: raw.phase_receipts,
            diagnostics: raw.diagnostics,
        };
        state.validate().map_err(serde::de::Error::custom)?;
        Ok(state)
    }
}

impl LeaderAiRuntimeState {
    #[must_use]
    pub fn new() -> Self {
        Self::new_for_colony(DEFAULT_RUNTIME_COLONY_ID).expect("default canonical runtime is valid")
    }

    pub fn new_for_colony(colony_id: &str) -> Result<Self, LeaderAiRuntimeError> {
        Self::new_for_colony_seed(colony_id, 0)
    }

    pub fn new_for_colony_seed(
        colony_id: &str,
        colony_seed: u64,
    ) -> Result<Self, LeaderAiRuntimeError> {
        if colony_id.trim().is_empty() {
            return Err(LeaderAiRuntimeError::MalformedRuntimeState);
        }
        let partition = PlannerId::derive("colony", [colony_id]);
        let hole = BlackHoleState::new(
            DEFAULT_HOLE_ID,
            TilePoint { x: 0, y: 0 },
            HoleAxes::default(),
            crate::black_hole::OPENING_GAME_MINUTES,
        )
        .map_err(|_| LeaderAiRuntimeError::MalformedHole)?;
        let divine_hole = DivineHoleAuthority::new(
            HoleAuthorityBinding::new(partition.clone(), DEFAULT_HOLE_ID)
                .map_err(|_| LeaderAiRuntimeError::MalformedHole)?,
        );
        let diagnostics = Lai69LeaderAiDiagnostics::new(
            Lai69ColonyId::new(colony_id)
                .map_err(|_| LeaderAiRuntimeError::MalformedDiagnostics)?,
            false,
            Lai69DiagnosticsConfig::default(),
        )
        .map_err(|_| LeaderAiRuntimeError::MalformedDiagnostics)?;
        let state = Self {
            schema_version: LEADER_AI_RUNTIME_SCHEMA_VERSION,
            colony_id: colony_id.to_owned(),
            colony_partition: partition.clone(),
            last_processed_tick: None,
            planner: ContentPlannerState::new(partition.clone()),
            beliefs: BeliefStore::new(),
            last_report: None,
            resident_needs_report_version: None,
            resident_needs_summary: None,
            officer_requests: OfficerRequestBook::new(),
            intents: IntentGraph::new(),
            scheduling: SchedulingRuntimeAggregate::new(),
            cat_capabilities: CatCapabilityAuthority::new(),
            families: FamilyAuthorityState::empty(colony_id, colony_seed),
            governance: GovernanceAuthorityState::new(colony_id)
                .map_err(|_| LeaderAiRuntimeError::MalformedGovernance)?,
            research: ResearchAuthority::new(
                partition.clone(),
                ResearchNotes::ZERO,
                VoidInsight::ZERO,
            ),
            construction_projects: BTreeMap::new(),
            construction_storage_identities: BTreeMap::new(),
            construction_runtime_ticks: BTreeMap::new(),
            construction_materializations: BTreeMap::new(),
            construction_miracles: ConstructionMiracleRuntimeState::new(),
            storage: StorageAuthority::new(colony_id)
                .map_err(|_| LeaderAiRuntimeError::MalformedStorage)?,
            hole,
            divine_hole,
            purpose_bound_storage: BTreeMap::new(),
            boosts: DivineBoostState::new(partition),
            trade: TradeAuthority::new(),
            cat_physical: BTreeMap::new(),
            prosthetics: ProstheticLedger::default(),
            player_directives: PlayerDirectiveState::new(),
            task_outcomes: BTreeMap::new(),
            phase_receipts: BTreeMap::new(),
            diagnostics,
        };
        state.validate()?;
        Ok(state)
    }

    /// Rebind only the untouched compatibility default. Persisted state is
    /// never silently repartitioned.
    pub fn bind_pristine_to_colony(
        &mut self,
        colony_id: &str,
    ) -> Result<bool, LeaderAiRuntimeError> {
        if self != &Self::new() {
            return Ok(false);
        }
        *self = Self::new_for_colony(colony_id)?;
        Ok(true)
    }

    /// Set the physical Hole anchor only before any Hole/task/tick mutation.
    pub fn bind_pristine_hole_anchor(
        &mut self,
        anchor: TilePoint,
    ) -> Result<bool, LeaderAiRuntimeError> {
        if self.last_processed_tick.is_some()
            || self.hole.active_feed.is_some()
            || self.hole.active_upgrade.is_some()
            || !self.scheduling.visible_tasks.is_empty()
        {
            return Ok(false);
        }
        self.hole = BlackHoleState::new(
            DEFAULT_HOLE_ID,
            anchor,
            self.hole.axes,
            self.hole.next_opening_game_minute,
        )
        .map_err(|_| LeaderAiRuntimeError::MalformedHole)?;
        self.divine_hole
            .binding
            .validate_hole(&self.hole)
            .map_err(|_| LeaderAiRuntimeError::MalformedHole)?;
        Ok(true)
    }

    /// Backfill real cats into every canonical identity/lifecycle authority in
    /// stable birth-time/ID order. The whole reconciliation is staged.
    pub fn reconcile_legacy_cats(
        &mut self,
        world_seed: u32,
        colony_id: &str,
        cats: &[Cat],
    ) -> Result<(), LeaderAiRuntimeError> {
        if colony_id != self.colony_id {
            return Err(LeaderAiRuntimeError::WrongPartition);
        }
        let mut staged = self.clone();
        let mut ordered = cats.iter().collect::<Vec<_>>();
        ordered.sort_by(|left, right| {
            left.birth_time
                .cmp(&right.birth_time)
                .then_with(|| left.id.cmp(&right.id))
        });
        for cat in ordered {
            let physical = staged
                .cat_physical
                .entry(cat.id.clone())
                .or_insert_with(|| CatPhysicalState::from_legacy(world_seed, colony_id, cat))
                .clone();
            if staged.cat_capabilities.cat_report(&cat.id).is_none() {
                staged
                    .cat_capabilities
                    .register_cat(capability_registration(cat, &physical))
                    .map_err(|_| LeaderAiRuntimeError::MalformedCapabilities)?;
            }
            if !staged.families.cats.contains_key(&cat.id) {
                register_family_cat(&mut staged.families, cat)?;
            }
            if staged.governance.resident(&cat.id).is_none() {
                staged
                    .governance
                    .register_resident(governance_fact(cat, &physical)?)
                    .map_err(|_| LeaderAiRuntimeError::MalformedGovernance)?;
            }
        }
        let life_stages = cats
            .iter()
            .map(|cat| (cat.id.clone(), family_life_stage(cat.age_hours)))
            .collect::<BTreeMap<_, _>>();
        let life_stage_changed = life_stages.iter().any(|(cat_id, life_stage)| {
            staged
                .families
                .cats
                .get(cat_id)
                .is_some_and(|cat| cat.life_stage != *life_stage)
        });
        if life_stage_changed {
            staged
                .families
                .apply(FamilyCommand {
                    receipt_id: format!("runtime_life_stages_{}", staged.families.revision),
                    expected_revision: staged.families.revision,
                    operation: FamilyOperation::ReconcileLifeStages { life_stages },
                })
                .map_err(|_| LeaderAiRuntimeError::MalformedFamily)?;
        }
        staged.validate()?;
        *self = staged;
        Ok(())
    }

    /// Project each newly observed physical death into the family and
    /// governance authorities exactly once. The per-cat marker makes restart
    /// replay a no-op while the staged clone keeps succession/family updates
    /// atomic with the marker.
    pub fn reconcile_cat_deaths(
        &mut self,
        runtime_tick: u64,
        cats: &[Cat],
    ) -> Result<(), LeaderAiRuntimeError> {
        let mut staged = self.clone();
        for cat in cats.iter().filter(|cat| cat.death_time.is_some()) {
            let physical = staged
                .cat_physical
                .get_mut(&cat.id)
                .ok_or(LeaderAiRuntimeError::CatPartitionMismatch)?;
            if physical.death_processed_tick.is_some() {
                continue;
            }
            if staged
                .families
                .cats
                .get(&cat.id)
                .is_some_and(|family_cat| family_cat.alive)
            {
                staged
                    .families
                    .apply(FamilyCommand {
                        receipt_id: format!("runtime_death:{}", cat.id),
                        expected_revision: staged.families.revision,
                        operation: FamilyOperation::RecordDeath {
                            cat_id: cat.id.clone(),
                        },
                    })
                    .map_err(|_| LeaderAiRuntimeError::MalformedFamily)?;
            }
            if staged
                .governance
                .resident(&cat.id)
                .is_some_and(|resident| resident.alive)
            {
                let _ = staged
                    .governance
                    .record_death(&cat.id, runtime_tick, 60)
                    .map_err(|_| LeaderAiRuntimeError::MalformedGovernance)?;
            }
            physical.death_processed_tick = Some(runtime_tick);
        }
        staged.validate()?;
        *self = staged;
        Ok(())
    }

    /// Reconcile the exact completed family institutions and run the
    /// deterministic partnership/housing cadence against those physical sites.
    /// The caller resolves real building IDs and types; this aggregate never
    /// invents a residence or teaching location.
    pub fn reconcile_family_buildings_and_housing(
        &mut self,
        runtime_tick: u64,
        mut buildings: Vec<FamilyBuilding>,
        pressure_requires_den_return: bool,
    ) -> Result<(), LeaderAiRuntimeError> {
        buildings.sort_by(|left, right| left.building_id.cmp(&right.building_id));
        let current = self
            .families
            .buildings
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let buildings_changed = current != buildings;
        let housing_reconciliation_due = buildings
            .iter()
            .any(|building| building.housing_kind.is_some())
            && self
                .families
                .cats
                .values()
                .any(|cat| cat.alive && !self.families.residences.contains_key(&cat.cat_id));
        let cadence_due = runtime_tick % 60 == 0 || housing_reconciliation_due;
        if !buildings_changed && !cadence_due {
            return Ok(());
        }

        let mut staged = self.clone();
        if buildings_changed {
            staged
                .families
                .apply(FamilyCommand {
                    receipt_id: format!("runtime_family_buildings_{runtime_tick}"),
                    expected_revision: staged.families.revision,
                    operation: FamilyOperation::ReconcileBuildings { buildings },
                })
                .map_err(|_| LeaderAiRuntimeError::MalformedFamily)?;
        }
        staged
            .families
            .apply(FamilyCommand {
                receipt_id: format!("runtime_family_partnerships_{runtime_tick}"),
                expected_revision: staged.families.revision,
                operation: FamilyOperation::ReviewAutonomousPartnerships,
            })
            .map_err(|_| LeaderAiRuntimeError::MalformedFamily)?;
        staged
            .families
            .apply(FamilyCommand {
                receipt_id: format!("runtime_family_housing_{runtime_tick}"),
                expected_revision: staged.families.revision,
                operation: FamilyOperation::ReconcileHousing {
                    pressure_requires_den_return,
                },
            })
            .map_err(|_| LeaderAiRuntimeError::MalformedFamily)?;
        staged.validate()?;
        *self = staged;
        Ok(())
    }

    /// Bind one exact catalog blueprint and its already-reserved physical
    /// identities. The catalog bill is re-resolved during every validation.
    pub fn insert_construction_project(
        &mut self,
        project: ConstructionProject,
        identities: BTreeSet<StorageIdentity>,
    ) -> Result<(), LeaderAiRuntimeError> {
        if self.construction_projects.len() >= MAX_CONSTRUCTION_PROJECTS
            || self.construction_projects.contains_key(&project.project_id)
        {
            return Err(LeaderAiRuntimeError::BoundExceeded);
        }
        let mut staged = self.clone();
        staged
            .construction_storage_identities
            .insert(project.project_id.clone(), identities);
        staged
            .construction_projects
            .insert(project.project_id.clone(), project);
        staged.validate()?;
        *self = staged;
        Ok(())
    }

    /// One completed physical task may issue exactly one capability receipt and
    /// at most one family professional-completion receipt.
    pub fn apply_task_outcome_once(
        &mut self,
        task_id: TaskId,
        cat_id: String,
        outcome: ProductiveOutcome,
        family_completion: Option<ProfessionalCompletion>,
    ) -> Result<TaskOutcomeBinding, LeaderAiRuntimeError> {
        let capability_receipt_id = format!("task_outcome:{}", task_id.as_str());
        let family_receipt_id = family_completion
            .as_ref()
            .map(|_| format!("family_outcome:{}", task_id.as_str()));
        let binding = TaskOutcomeBinding {
            task_id: task_id.clone(),
            cat_id: cat_id.clone(),
            outcome: outcome.clone(),
            capability_receipt_id: capability_receipt_id.clone(),
            family_receipt_id: family_receipt_id.clone(),
        };
        if let Some(existing) = self.task_outcomes.get(&task_id) {
            return if existing == &binding {
                Ok(existing.clone())
            } else {
                Err(LeaderAiRuntimeError::OutcomeReplayConflict)
            };
        }
        if self.task_outcomes.len() >= MAX_TASK_OUTCOME_BINDINGS {
            return Err(LeaderAiRuntimeError::BoundExceeded);
        }
        let mut staged = self.clone();
        staged
            .cat_capabilities
            .apply_productive_outcome_receipt(ProductiveOutcomeReceipt {
                receipt_id: capability_receipt_id,
                cat_id,
                outcome,
            })
            .map_err(|_| LeaderAiRuntimeError::MalformedCapabilities)?;
        if let Some(completion) = family_completion {
            if completion.task_id != task_id.as_str() {
                return Err(LeaderAiRuntimeError::OutcomeReplayConflict);
            }
            staged
                .families
                .apply(FamilyCommand {
                    receipt_id: family_receipt_id
                        .clone()
                        .expect("family completion has a receipt"),
                    expected_revision: staged.families.revision,
                    operation: FamilyOperation::RecordProfessionalCompletion(completion),
                })
                .map_err(|_| LeaderAiRuntimeError::MalformedFamily)?;
        }
        staged.task_outcomes.insert(task_id, binding.clone());
        staged.validate()?;
        *self = staged;
        Ok(binding)
    }

    /// Advance one canonical task by one truthful physical stage. The caller
    /// supplies a report-safe workforce projection plus an already-resolved
    /// exact source/work/endpoint/route contract. This method never chooses a
    /// nearby site, endpoint, route, worker, or cargo substitute.
    ///
    /// A full clone is committed only after the local reservation, world
    /// reservation, storage identity movement, task stage, and once-only work
    /// outcome all validate together. Therefore an error leaves this aggregate
    /// byte-for-byte unchanged.
    pub fn advance_physical_task(
        &mut self,
        request: PhysicalTaskExecutionRequest,
        now_tick: u64,
    ) -> Result<PhysicalTaskExecutionOutcome, LeaderAiRuntimeError> {
        let mut staged = self.clone();
        let outcome = staged.advance_physical_task_staged(&request, now_tick)?;
        staged.validate()?;
        *self = staged;
        Ok(outcome)
    }

    fn advance_physical_task_staged(
        &mut self,
        request: &PhysicalTaskExecutionRequest,
        now_tick: u64,
    ) -> Result<PhysicalTaskExecutionOutcome, LeaderAiRuntimeError> {
        validate_physical_task_request(request)?;
        let (task_spatial, task_category, task_stage) = {
            let task = self
                .scheduling
                .visible_tasks
                .get(&request.task_id)
                .ok_or(LeaderAiRuntimeError::MissingPhysicalTask)?;
            (task.spatial.clone(), task.category, task.stage)
        };
        if task_spatial != request.resolved.spatial {
            return Err(LeaderAiRuntimeError::PhysicalTaskSpatialMismatch);
        }
        if self
            .scheduling
            .task_storage_endpoints
            .get(&request.task_id)
            .is_some_and(|endpoint| endpoint != &request.cargo.endpoint)
        {
            return Err(LeaderAiRuntimeError::PhysicalTaskCargoMismatch);
        }

        // Interruption handling is intentionally ahead of resolved-route
        // validation. An authority may have just invalidated the exact route;
        // carried cargo must then recover from the already persisted task
        // contract instead of being rejected before its recovery can run.
        if request.interruption != PhysicalTaskInterruption::None {
            return self.interrupt_physical_task(request, now_tick);
        }

        request
            .resolved
            .validate()
            .map_err(|_| LeaderAiRuntimeError::PhysicalTaskReservationInvalid)?;
        if !physical_task_category_matches(task_category, request.resolved.category) {
            if matches!(task_stage, TaskStage::Resolve | TaskStage::Reserve) {
                block_task_before_pickup(
                    &mut self.scheduling,
                    &request.task_id,
                    RuntimeBlockReason::InvalidLegacySite,
                    now_tick,
                )?;
                return Ok(PhysicalTaskExecutionOutcome::Blocked(
                    PhysicalTaskBlockReason::UnsupportedTaskStage,
                ));
            }
            return Err(LeaderAiRuntimeError::UnsupportedPhysicalTaskCategory);
        }

        let stage = self
            .scheduling
            .visible_tasks
            .get(&request.task_id)
            .expect("task was preflighted")
            .stage;
        match stage {
            TaskStage::Resolve | TaskStage::Reserve => {
                self.activate_physical_task(request, now_tick)
            }
            TaskStage::TravelToSource => {
                self.task_mut(&request.task_id)?
                    .advance(TaskStage::Pickup, now_tick)
                    .map_err(|_| LeaderAiRuntimeError::MalformedRuntimeState)?;
                Ok(PhysicalTaskExecutionOutcome::Advanced {
                    stage: TaskStage::Pickup,
                })
            }
            TaskStage::Pickup => self.pickup_physical_task(request, now_tick),
            TaskStage::TravelToWork => {
                self.task_mut(&request.task_id)?
                    .advance(TaskStage::Work, now_tick)
                    .map_err(|_| LeaderAiRuntimeError::MalformedRuntimeState)?;
                Ok(PhysicalTaskExecutionOutcome::Advanced {
                    stage: TaskStage::Work,
                })
            }
            TaskStage::Work => self.work_physical_task(request, now_tick),
            TaskStage::TravelToEndpoint => {
                self.task_mut(&request.task_id)?
                    .advance(TaskStage::Deposit, now_tick)
                    .map_err(|_| LeaderAiRuntimeError::MalformedRuntimeState)?;
                Ok(PhysicalTaskExecutionOutcome::Advanced {
                    stage: TaskStage::Deposit,
                })
            }
            TaskStage::Deposit => self.deposit_physical_task(request, now_tick),
            TaskStage::Complete | TaskStage::Blocked | TaskStage::Cancelled => {
                Ok(PhysicalTaskExecutionOutcome::Terminal(stage))
            }
        }
    }

    fn activate_physical_task(
        &mut self,
        request: &PhysicalTaskExecutionRequest,
        now_tick: u64,
    ) -> Result<PhysicalTaskExecutionOutcome, LeaderAiRuntimeError> {
        let task_id = request.task_id.clone();
        let existing_stage = self.task_mut(&task_id)?.stage;
        if existing_stage == TaskStage::Resolve {
            self.task_mut(&task_id)?
                .begin_reservation(now_tick)
                .map_err(|_| LeaderAiRuntimeError::MalformedRuntimeState)?;
        }

        let selected = select_physical_task_worker(&self.cat_capabilities, &task_id, request)?;
        let Some(worker) = selected else {
            let reason = physical_worker_block_reason(&request.workers);
            block_task_before_pickup(
                &mut self.scheduling,
                &task_id,
                match reason {
                    PhysicalTaskBlockReason::NoWillingWorker => RuntimeBlockReason::WorkerRefused,
                    PhysicalTaskBlockReason::NoLivingCapableWorker => {
                        RuntimeBlockReason::WorkerIncapacitated
                    }
                    PhysicalTaskBlockReason::ReservationUnavailable
                    | PhysicalTaskBlockReason::CargoUnavailable
                    | PhysicalTaskBlockReason::UnsupportedTaskStage => {
                        RuntimeBlockReason::ReservationLost
                    }
                },
                now_tick,
            )?;
            return Ok(PhysicalTaskExecutionOutcome::Blocked(reason));
        };

        self.verify_exact_cargo_at_source(&request.cargo)?;
        if !physical_storage_endpoint_matches(&request.cargo.endpoint, &request.resolved) {
            return Err(LeaderAiRuntimeError::PhysicalTaskCargoMismatch);
        }
        let (task_id_key, intent_id, spatial) = {
            let task = self
                .scheduling
                .visible_tasks
                .get(&task_id)
                .expect("task was preflighted");
            (
                PlannerId::derive("visible_task", [task.id.as_str()]),
                task.intent_id.clone(),
                task.spatial.clone(),
            )
        };
        let worker_id = PlannerId::derive("cat", [worker.as_str()]);
        let cargo_key = storage_identity_claim_key(&request.cargo.identity)?;
        let cargo_capacity = u32::try_from(request.cargo.quantity)
            .map_err(|_| LeaderAiRuntimeError::PhysicalTaskCargoMismatch)?;
        let route_id = physical_route_claim_key(&request.resolved)?;
        let local = ReservationBundle::from_spatial_objective(
            self.colony_partition.clone(),
            task_id_key.clone(),
            intent_id.clone(),
            &spatial,
            0,
            physical_objective_claim_mode(&request.resolved),
            ClaimMode::Capacity {
                units: request.resolved.delivery_units,
                capacity: request.resolved.delivery_capacity,
            },
            ClaimSpec::capacity(route_id, 1, 1),
            Vec::new(),
            vec![ClaimSpec::capacity(
                cargo_key.clone(),
                cargo_capacity,
                cargo_capacity,
            )],
            worker_id.clone(),
        )
        .map_err(|_| LeaderAiRuntimeError::PhysicalTaskReservationInvalid)?;
        let world = WorldReservationTransaction::new(
            self.colony_partition.clone(),
            task_id_key,
            intent_id,
            request.resolved.clone(),
            worker_id,
            Vec::new(),
            vec![CapacityReservation {
                stable_id: cargo_key,
                units: cargo_capacity,
                capacity: cargo_capacity,
            }],
        )
        .map_err(|_| LeaderAiRuntimeError::PhysicalTaskReservationInvalid)?;
        let local_id = local.id.clone();
        let world_id = world.id.clone();
        if self
            .scheduling
            .reservations
            .try_commit(local, ReservationChecks::all_valid())
            .is_err()
            || self
                .scheduling
                .world_reservations
                .try_commit(world, WorldReservationValidation::all_valid())
                .is_err()
        {
            self.scheduling.reservations.rollback(&local_id);
            let _ = self.scheduling.world_reservations.release(&world_id);
            block_task_before_pickup(
                &mut self.scheduling,
                &task_id,
                RuntimeBlockReason::ReservationLost,
                now_tick,
            )?;
            return Ok(PhysicalTaskExecutionOutcome::Blocked(
                PhysicalTaskBlockReason::ReservationUnavailable,
            ));
        }

        let reservation_owner = task_id.as_str().to_owned();
        if self
            .storage_command(
                format!("physical_reserve_{}", task_id.as_str()),
                format!("physical_reserve_v1_{}", task_id.as_str()),
                StorageCommand::Reserve {
                    identity: request.cargo.identity.clone(),
                    owner: reservation_owner,
                },
            )
            .is_err()
        {
            self.scheduling.reservations.rollback(&local_id);
            let _ = self.scheduling.world_reservations.release(&world_id);
            block_task_before_pickup(
                &mut self.scheduling,
                &task_id,
                RuntimeBlockReason::ReservationLost,
                now_tick,
            )?;
            return Ok(PhysicalTaskExecutionOutcome::Blocked(
                PhysicalTaskBlockReason::CargoUnavailable,
            ));
        }

        let route_ids = physical_route_ids(&request.resolved);
        {
            let SchedulingRuntimeAggregate {
                visible_tasks,
                reservations,
                ..
            } = &mut self.scheduling;
            let task = visible_tasks
                .get_mut(&task_id)
                .ok_or(LeaderAiRuntimeError::MissingPhysicalTask)?;
            if !task.route_ids.is_empty() && task.route_ids != route_ids {
                return Err(LeaderAiRuntimeError::PhysicalTaskSpatialMismatch);
            }
            task.route_ids = route_ids;
            task.reserve_cargo_at_source(
                task_id.as_str(),
                request.cargo.resource_id.clone(),
                request.cargo.quantity,
            )
            .map_err(|_| LeaderAiRuntimeError::MalformedRuntimeState)?;
            task.activate(
                reservations,
                local_id,
                [(
                    worker.clone(),
                    request.resolved.work_slot().stable_id.clone(),
                )],
                now_tick,
            )
            .map_err(|_| LeaderAiRuntimeError::MalformedRuntimeState)?;
        }
        self.scheduling
            .resolved_spatial_tasks
            .insert(task_id.clone(), request.resolved.clone());
        self.scheduling
            .world_reservation_ids
            .insert(task_id.clone(), world_id);
        self.scheduling
            .task_storage_identities
            .insert(task_id.clone(), request.cargo.identity.clone());
        self.scheduling
            .task_storage_endpoints
            .insert(task_id.clone(), request.cargo.endpoint.clone());
        self.remember_exact_cargo_sites(&request.resolved);
        Ok(PhysicalTaskExecutionOutcome::Activated { cat_id: worker })
    }

    fn pickup_physical_task(
        &mut self,
        request: &PhysicalTaskExecutionRequest,
        now_tick: u64,
    ) -> Result<PhysicalTaskExecutionOutcome, LeaderAiRuntimeError> {
        let task_id = request.task_id.clone();
        let cat_id = self
            .scheduling
            .visible_tasks
            .get(&task_id)
            .and_then(|task| task.assigned_cat_ids.iter().next().cloned())
            .ok_or(LeaderAiRuntimeError::MalformedRuntimeState)?;
        let owner = task_id.as_str().to_owned();
        self.storage_command(
            format!("physical_unreserve_{}", task_id.as_str()),
            format!("physical_unreserve_v1_{}", task_id.as_str()),
            StorageCommand::Unreserve {
                identity: request.cargo.identity.clone(),
                owner,
            },
        )?;
        self.storage_command(
            format!("physical_pickup_{}", task_id.as_str()),
            format!("physical_pickup_v1_{}", task_id.as_str()),
            StorageCommand::Move {
                identity: request.cargo.identity.clone(),
                destination: StorageAddress::RouteCargo {
                    route_id: task_id.as_str().to_owned(),
                },
            },
        )?;
        let task = self.task_mut(&task_id)?;
        task.pickup(&cat_id, now_tick)
            .and_then(|()| task.advance(TaskStage::TravelToWork, now_tick))
            .map_err(|_| LeaderAiRuntimeError::MalformedRuntimeState)?;
        Ok(PhysicalTaskExecutionOutcome::Advanced {
            stage: TaskStage::TravelToWork,
        })
    }

    fn work_physical_task(
        &mut self,
        request: &PhysicalTaskExecutionRequest,
        now_tick: u64,
    ) -> Result<PhysicalTaskExecutionOutcome, LeaderAiRuntimeError> {
        let task_id = request.task_id.clone();
        let cat_id = self
            .scheduling
            .visible_tasks
            .get(&task_id)
            .and_then(|task| task.assigned_cat_ids.iter().next().cloned())
            .ok_or(LeaderAiRuntimeError::MalformedRuntimeState)?;
        {
            let task = self.task_mut(&task_id)?;
            task.progress_basis_points = crate::task_runtime::TASK_PROGRESS_MAX_BASIS_POINTS;
            task.advance(TaskStage::TravelToEndpoint, now_tick)
                .map_err(|_| LeaderAiRuntimeError::MalformedRuntimeState)?;
        }
        self.apply_task_outcome_once(
            task_id,
            cat_id.clone(),
            request.work.outcome.clone(),
            request.work.family_completion.clone(),
        )?;
        Ok(PhysicalTaskExecutionOutcome::Worked { cat_id })
    }

    fn deposit_physical_task(
        &mut self,
        request: &PhysicalTaskExecutionRequest,
        now_tick: u64,
    ) -> Result<PhysicalTaskExecutionOutcome, LeaderAiRuntimeError> {
        let task_id = request.task_id.clone();
        self.storage_command(
            format!("physical_deposit_{}", task_id.as_str()),
            format!("physical_deposit_v1_{}", task_id.as_str()),
            StorageCommand::Move {
                identity: request.cargo.identity.clone(),
                destination: request.cargo.endpoint.clone(),
            },
        )?;
        self.task_mut(&task_id)?
            .deposit(now_tick)
            .map_err(|_| LeaderAiRuntimeError::MalformedRuntimeState)?;
        self.complete_physical_task(&task_id, now_tick)?;
        Ok(PhysicalTaskExecutionOutcome::Completed)
    }

    fn interrupt_physical_task(
        &mut self,
        request: &PhysicalTaskExecutionRequest,
        now_tick: u64,
    ) -> Result<PhysicalTaskExecutionOutcome, LeaderAiRuntimeError> {
        let task_id = request.task_id.clone();
        let stage = self.task_mut(&task_id)?.stage;
        if matches!(
            stage,
            TaskStage::Complete | TaskStage::Blocked | TaskStage::Cancelled
        ) {
            return Ok(PhysicalTaskExecutionOutcome::Terminal(stage));
        }
        let cargo_location = self
            .scheduling
            .visible_tasks
            .get(&task_id)
            .and_then(|task| task.cargo.as_ref())
            .map(|cargo| cargo.location.clone());
        if matches!(cargo_location, Some(CargoLocation::Carried { .. })) {
            if let Some(recovery) = &request.cargo.recovery {
                self.storage_command(
                    format!("physical_recover_{}", task_id.as_str()),
                    format!("physical_recover_v1_{}", task_id.as_str()),
                    StorageCommand::Move {
                        identity: request.cargo.identity.clone(),
                        destination: recovery.clone(),
                    },
                )?;
            }
            let recovery_is_exact_endpoint = request
                .cargo
                .recovery
                .as_ref()
                .is_some_and(|address| address == &request.cargo.endpoint);
            let safe_endpoint = if recovery_is_exact_endpoint {
                self.task_mut(&task_id)?
                    .spatial
                    .delivery_endpoint
                    .clone()
                    .filter(|endpoint| matches!(endpoint, SiteRef::Stockpile { .. }))
            } else {
                None
            };
            let last_site_id = self
                .scheduling
                .resolved_spatial_tasks
                .get(&task_id)
                .map(|resolved| resolved.work_to_delivery_route.stable_id().to_owned())
                .unwrap_or_else(|| {
                    request
                        .resolved
                        .work_to_delivery_route
                        .stable_id()
                        .to_owned()
                });
            recover_task_after_pickup(
                &mut self.scheduling,
                &task_id,
                request.interruption.runtime_block_reason(),
                safe_endpoint.as_ref(),
                &last_site_id,
                now_tick,
            )?;
            self.release_world_reservation(&task_id)?;
            return Ok(PhysicalTaskExecutionOutcome::Recovered {
                stranded: safe_endpoint.is_none(),
            });
        }
        if matches!(
            cargo_location,
            Some(CargoLocation::DepositedAtEndpoint { .. })
        ) {
            self.complete_physical_task(&task_id, now_tick)?;
            return Ok(PhysicalTaskExecutionOutcome::Completed);
        }

        if cargo_location.is_some() {
            self.storage_command(
                format!("physical_release_{}", task_id.as_str()),
                format!("physical_release_v1_{}", task_id.as_str()),
                StorageCommand::Unreserve {
                    identity: request.cargo.identity.clone(),
                    owner: task_id.as_str().to_owned(),
                },
            )?;
        }
        if request.interruption == PhysicalTaskInterruption::Cancelled
            || request.interruption.is_preemption()
        {
            cancel_task_before_pickup(&mut self.scheduling, &task_id, now_tick)?;
            self.scheduling.task_storage_identities.remove(&task_id);
            self.scheduling.task_storage_endpoints.remove(&task_id);
            self.release_world_reservation(&task_id)?;
            return if request.interruption.is_preemption() {
                Ok(PhysicalTaskExecutionOutcome::Preempted(
                    request.interruption,
                ))
            } else {
                Ok(PhysicalTaskExecutionOutcome::Cancelled)
            };
        }
        block_task_before_pickup(
            &mut self.scheduling,
            &task_id,
            request.interruption.pre_pickup_block_reason(),
            now_tick,
        )?;
        self.scheduling.task_storage_identities.remove(&task_id);
        self.scheduling.task_storage_endpoints.remove(&task_id);
        self.release_world_reservation(&task_id)?;
        Ok(PhysicalTaskExecutionOutcome::Blocked(
            PhysicalTaskBlockReason::CargoUnavailable,
        ))
    }

    fn complete_physical_task(
        &mut self,
        task_id: &TaskId,
        now_tick: u64,
    ) -> Result<(), LeaderAiRuntimeError> {
        complete_task(&mut self.scheduling, task_id, now_tick)?;
        self.release_world_reservation(task_id)
    }

    fn release_world_reservation(&mut self, task_id: &TaskId) -> Result<(), LeaderAiRuntimeError> {
        if let Some(id) = self.scheduling.world_reservation_ids.remove(task_id) {
            self.scheduling
                .world_reservations
                .release(&id)
                .map_err(|_| LeaderAiRuntimeError::PhysicalTaskReservationInvalid)?;
        }
        Ok(())
    }

    fn verify_exact_cargo_at_source(
        &self,
        cargo: &PhysicalTaskCargoBinding,
    ) -> Result<(), LeaderAiRuntimeError> {
        if cargo.resource_id.is_empty()
            || cargo.quantity == 0
            || matches!(&cargo.endpoint, StorageAddress::RouteCargo { .. })
            || cargo
                .recovery
                .as_ref()
                .is_some_and(|address| matches!(address, StorageAddress::RouteCargo { .. }))
        {
            return Err(LeaderAiRuntimeError::PhysicalTaskCargoMismatch);
        }
        if self.storage.location(&cargo.identity) != Some(&cargo.source) {
            return Err(LeaderAiRuntimeError::PhysicalTaskCargoMismatch);
        }
        let StorageIdentity::Lot(lot_id) = &cargo.identity else {
            return Err(LeaderAiRuntimeError::UnsupportedPhysicalCargoIdentity);
        };
        let lot = self
            .storage
            .ledger()
            .lot(lot_id)
            .ok_or(LeaderAiRuntimeError::PhysicalTaskCargoMismatch)?;
        if lot.key.content_id.as_str() != cargo.resource_id.as_str()
            || u64::from(lot.quantity) != cargo.quantity
            || lot.reservation.is_some()
        {
            return Err(LeaderAiRuntimeError::PhysicalTaskCargoMismatch);
        }
        Ok(())
    }

    fn remember_exact_cargo_sites(&mut self, resolved: &ResolvedSpatialTask) {
        let mut sites = vec![
            resolved.objective().stable_id().to_owned(),
            resolved.work_slot().site.stable_id().to_owned(),
            resolved.delivery_endpoint().stable_id().to_owned(),
            resolved.source_to_work_route.stable_id().to_owned(),
            resolved.work_to_delivery_route.stable_id().to_owned(),
        ];
        sites.sort();
        sites.dedup();
        self.scheduling.known_cargo_site_ids.extend(sites);
    }

    fn task_mut(
        &mut self,
        task_id: &TaskId,
    ) -> Result<&mut VisibleTaskRuntime, LeaderAiRuntimeError> {
        self.scheduling
            .visible_tasks
            .get_mut(task_id)
            .ok_or(LeaderAiRuntimeError::MissingPhysicalTask)
    }

    fn storage_command(
        &mut self,
        command_id: String,
        fingerprint: String,
        command: StorageCommand,
    ) -> Result<(), LeaderAiRuntimeError> {
        let sequence = self
            .storage
            .version()
            .checked_add(1)
            .ok_or(LeaderAiRuntimeError::BoundExceeded)?;
        self.storage
            .execute(StorageCommandEnvelope {
                colony_id: self.colony_id.clone(),
                command_id,
                fingerprint,
                sequence,
                command,
            })
            .map(|_| ())
            .map_err(|_| LeaderAiRuntimeError::MalformedStorage)
    }

    /// Apply one Void action against `research.void` and materialize every
    /// generated unit into the same `StorageAuthority` transaction. The bound
    /// purpose is retained beside the exact storage identity so later Hole and
    /// barter adapters can reject it without reconstructing provenance.
    pub fn apply_void_action_and_materialize(
        &mut self,
        envelope: VoidActionEnvelope,
        destination: StorageAddress,
        compatibility: StorageCompatibility,
    ) -> Result<VoidActionOutcome, LeaderAiRuntimeError> {
        let mut staged = self.clone();
        let outcome = staged
            .divine_hole
            .apply_void_action(&mut staged.research.void, envelope)
            .map_err(|_| LeaderAiRuntimeError::MalformedHole)?;
        for cargo in &outcome.generated_cargo {
            staged.materialize_purpose_bound_cargo(cargo, destination.clone(), compatibility)?;
        }
        staged.validate()?;
        *self = staged;
        Ok(outcome)
    }

    pub(crate) fn materialize_purpose_bound_cargo(
        &mut self,
        cargo: &PurposeBoundCargo,
        destination: StorageAddress,
        compatibility: StorageCompatibility,
    ) -> Result<StorageIdentity, LeaderAiRuntimeError> {
        if let Ok(content_id) = ContentId::new(cargo.definition_id.clone())
            && ContentManifest::embedded()
                .construction_miracle_input(&content_id)
                .is_some_and(|descriptor| {
                    descriptor.physical_class != ConstructionMiracleInputClass::BulkLot
                })
        {
            return Err(LeaderAiRuntimeError::UnsupportedPhysicalCargoIdentity);
        }
        let identity_hash = fnv1a64(cargo.cargo_id.as_bytes());
        let lot_id = PhysicalLotId::new(format!("divine_{identity_hash:016x}"))
            .map_err(|_| LeaderAiRuntimeError::MalformedStorage)?;
        let identity = StorageIdentity::Lot(lot_id.clone());
        if let Some(existing) = self.purpose_bound_storage.get(&identity) {
            if existing == &cargo.purpose && self.storage.location(&identity).is_some() {
                return Ok(identity);
            }
            return Err(LeaderAiRuntimeError::OutcomeReplayConflict);
        }
        let quantity =
            u32::try_from(cargo.quantity).map_err(|_| LeaderAiRuntimeError::BoundExceeded)?;
        let sequence = self
            .storage
            .version()
            .checked_add(1)
            .ok_or(LeaderAiRuntimeError::BoundExceeded)?;
        self.storage
            .execute(StorageCommandEnvelope {
                colony_id: self.colony_id.clone(),
                command_id: format!("divine_deposit_{identity_hash:016x}"),
                fingerprint: format!("divine_deposit_v1_{identity_hash:016x}"),
                sequence,
                command: StorageCommand::DepositLot {
                    lot: PhysicalLot {
                        id: lot_id,
                        key: BulkLotKey::new(
                            ContentId::new(cargo.definition_id.clone())
                                .map_err(|_| LeaderAiRuntimeError::MalformedStorage)?,
                            QualityBand::Common,
                        ),
                        provenance: LotProvenance {
                            origin: format!("divine:{}", cargo.provenance_player_id),
                            created_tick: cargo.created_at_real_ms,
                        },
                        quantity,
                        location: LotLocation::Source(cargo.site_id.clone()),
                        reservation: None,
                    },
                    compatibility,
                    destination: destination.clone(),
                },
            })
            .map_err(|_| LeaderAiRuntimeError::MalformedStorage)?;
        let reservation_owner = match &destination {
            StorageAddress::ConstructionCargo { project_id } => project_id.clone(),
            _ => format!("purpose_{}", cargo.cargo_id.replace(':', "_")),
        };
        let reserve_sequence = self
            .storage
            .version()
            .checked_add(1)
            .ok_or(LeaderAiRuntimeError::BoundExceeded)?;
        self.storage
            .execute(StorageCommandEnvelope {
                colony_id: self.colony_id.clone(),
                command_id: format!("divine_reserve_{identity_hash:016x}"),
                fingerprint: format!("divine_reserve_v1_{identity_hash:016x}"),
                sequence: reserve_sequence,
                command: StorageCommand::Reserve {
                    identity: identity.clone(),
                    owner: reservation_owner,
                },
            })
            .map_err(|_| LeaderAiRuntimeError::MalformedStorage)?;
        self.purpose_bound_storage
            .insert(identity.clone(), cargo.purpose.clone());
        Ok(identity)
    }

    /// Materialize one manifest-classified construction miracle output without
    /// collapsing exact items or fixtures into bulk lots.
    pub(crate) fn materialize_typed_construction_miracle_cargo(
        &mut self,
        cargo: &PurposeBoundCargo,
        destination: StorageAddress,
    ) -> Result<Vec<StorageIdentity>, LeaderAiRuntimeError> {
        let content_id = ContentId::new(cargo.definition_id.clone())
            .map_err(|_| LeaderAiRuntimeError::MalformedStorage)?;
        let manifest = ContentManifest::embedded();
        let descriptor = manifest
            .construction_miracle_input(&content_id)
            .ok_or(LeaderAiRuntimeError::MalformedStorage)?;
        match descriptor.physical_class {
            ConstructionMiracleInputClass::BulkLot => self
                .materialize_purpose_bound_cargo(
                    cargo,
                    destination,
                    StorageCompatibility::BulkMaterial,
                )
                .map(|identity| vec![identity]),
            ConstructionMiracleInputClass::ExactItem | ConstructionMiracleInputClass::Fixture => {
                let material_id = descriptor
                    .generated_material_id
                    .clone()
                    .ok_or(LeaderAiRuntimeError::MalformedStorage)?;
                let (definition_id, augmentation_slot, compatibility) =
                    match descriptor.physical_class {
                        ConstructionMiracleInputClass::ExactItem => {
                            let definition = manifest
                                .item_definitions
                                .iter()
                                .find(|definition| definition.content_id == content_id)
                                .ok_or(LeaderAiRuntimeError::MalformedStorage)?;
                            let compatibility = match definition.class {
                                ItemClass::Tool => StorageCompatibility::Tool,
                                ItemClass::Tableware => StorageCompatibility::SmallItem,
                                ItemClass::Furniture => StorageCompatibility::UniqueItem,
                                _ => StorageCompatibility::UniqueItem,
                            };
                            (
                                definition.id.clone(),
                                definition.augmentation_slot,
                                compatibility,
                            )
                        }
                        ConstructionMiracleInputClass::Fixture => {
                            let fixture = manifest
                                .fixtures
                                .iter()
                                .find(|fixture| fixture.content_id == content_id)
                                .ok_or(LeaderAiRuntimeError::MalformedStorage)?;
                            (fixture.id.clone(), None, StorageCompatibility::UniqueItem)
                        }
                        _ => unreachable!("bulk and ineligible classes handled above"),
                    };
                let mut identities = Vec::new();
                let reservation_owner = match &destination {
                    StorageAddress::ConstructionCargo { project_id } => project_id.clone(),
                    _ => format!("purpose_{}", cargo.cargo_id.replace(':', "_")),
                };
                for serial in 0..cargo.quantity {
                    let identity_hash = fnv1a64(format!("{}:{serial}", cargo.cargo_id).as_bytes());
                    let item_id =
                        MaterialInstanceId::new(format!("divine_item_{identity_hash:016x}"))
                            .map_err(|_| LeaderAiRuntimeError::MalformedStorage)?;
                    let identity = StorageIdentity::Item(item_id.clone());
                    if let Some(existing) = self.purpose_bound_storage.get(&identity) {
                        if existing == &cargo.purpose
                            && self.storage.location(&identity) == Some(&destination)
                            && self.storage.ledger().item(&item_id).is_some_and(|item| {
                                item.definition_id == definition_id
                                    && item.material_id == material_id
                                    && item.quality == QualityBand::Common
                                    && item.durability == 100
                                    && item.reservation.as_deref()
                                        == Some(reservation_owner.as_str())
                            })
                        {
                            identities.push(identity);
                            continue;
                        }
                        return Err(LeaderAiRuntimeError::OutcomeReplayConflict);
                    }
                    let sequence = self
                        .storage
                        .version()
                        .checked_add(1)
                        .ok_or(LeaderAiRuntimeError::BoundExceeded)?;
                    self.storage
                        .execute(StorageCommandEnvelope {
                            colony_id: self.colony_id.clone(),
                            command_id: format!("divine_item_deposit_{identity_hash:016x}"),
                            fingerprint: format!("divine_item_deposit_v1_{identity_hash:016x}"),
                            sequence,
                            command: StorageCommand::DepositItem {
                                item: ItemInstance {
                                    id: item_id,
                                    definition_id: definition_id.clone(),
                                    material_id: material_id.clone(),
                                    quality: QualityBand::Common,
                                    durability: 100,
                                    location: LotLocation::Source(cargo.site_id.clone()),
                                    reservation: None,
                                    equipment_slot: None,
                                    augmentation_slot,
                                    augmentation: None,
                                },
                                compatibility,
                                destination: destination.clone(),
                            },
                        })
                        .map_err(|_| LeaderAiRuntimeError::MalformedStorage)?;
                    let reserve_sequence = self
                        .storage
                        .version()
                        .checked_add(1)
                        .ok_or(LeaderAiRuntimeError::BoundExceeded)?;
                    self.storage
                        .execute(StorageCommandEnvelope {
                            colony_id: self.colony_id.clone(),
                            command_id: format!("divine_item_reserve_{identity_hash:016x}"),
                            fingerprint: format!("divine_item_reserve_v1_{identity_hash:016x}"),
                            sequence: reserve_sequence,
                            command: StorageCommand::Reserve {
                                identity: identity.clone(),
                                owner: reservation_owner.clone(),
                            },
                        })
                        .map_err(|_| LeaderAiRuntimeError::MalformedStorage)?;
                    self.purpose_bound_storage
                        .insert(identity.clone(), cargo.purpose.clone());
                    identities.push(identity);
                }
                Ok(identities)
            }
            ConstructionMiracleInputClass::Ineligible => {
                Err(LeaderAiRuntimeError::UnsupportedPhysicalCargoIdentity)
            }
        }
    }

    #[must_use]
    pub fn report_twin_bytes(&self) -> Option<(Vec<u8>, Vec<u8>)> {
        let report = self.last_report.as_ref()?;
        let planner = planner_report_bytes(report).ok()?;
        let god = god_report_bytes(report).ok()?;
        Some((planner, god))
    }

    #[must_use]
    pub fn storage_identity_can_trade(&self, identity: &StorageIdentity) -> bool {
        !self.purpose_bound_storage.contains_key(identity)
    }

    #[must_use]
    pub fn storage_identity_can_feed_hole(&self, identity: &StorageIdentity) -> bool {
        !self.purpose_bound_storage.contains_key(identity)
    }

    pub fn begin_tick_transaction(
        &self,
        tick: u64,
    ) -> Result<RuntimeTickTransaction, LeaderAiRuntimeError> {
        if self.last_processed_tick.is_some_and(|last| tick <= last) {
            return Err(LeaderAiRuntimeError::TickAlreadyProcessed);
        }
        Ok(RuntimeTickTransaction {
            tick,
            staged: self.clone(),
            completed_phases: Vec::new(),
        })
    }

    pub fn validate(&self) -> Result<(), LeaderAiRuntimeError> {
        if self.schema_version != LEADER_AI_RUNTIME_SCHEMA_VERSION
            || self.colony_id.trim().is_empty()
            || self.colony_partition != PlannerId::derive("colony", [&self.colony_id])
            || self.planner.colony_id != self.colony_partition
            || self.research.colony_id != self.colony_partition
            || self.boosts.colony_id != self.colony_partition
            || self.families.colony_id != self.colony_id
            || self.governance.colony_id() != self.colony_id
            || self.storage.colony_id() != self.colony_id
            || self.divine_hole.binding.colony_id != self.colony_partition
            || self.divine_hole.binding.hole_id != self.hole.hole_id
            || self.cat_physical.len() > MAX_CAT_PHYSICAL_STATES
            || self.construction_projects.len() > MAX_CONSTRUCTION_PROJECTS
            || self.task_outcomes.len() > MAX_TASK_OUTCOME_BINDINGS
            || self.phase_receipts.len() > MAX_PHASE_RECEIPTS
        {
            return Err(LeaderAiRuntimeError::MalformedRuntimeState);
        }
        validate_leaf("content planner", &self.planner)?;
        validate_leaf("beliefs", &self.beliefs)?;
        validate_leaf("officer requests", &self.officer_requests)?;
        if let Some(report) = &self.last_report {
            if report.colony_id != self.colony_partition
                || planner_report_bytes(report).ok() != god_report_bytes(report).ok()
            {
                return Err(LeaderAiRuntimeError::ReportProjectionMismatch);
            }
            validate_leaf("last report", report)?;
        }
        match (
            self.resident_needs_report_version,
            self.resident_needs_summary,
        ) {
            (None, None) => {}
            (Some(_report_version), Some(summary))
                if ReportedResidentNeedsSummary::new(
                    summary.living_resident_count,
                    summary.reported_dying_from_hunger,
                    summary.reported_dying_from_thirst,
                )
                .is_ok() =>
            {
                // Leaf transactions validate inside the protected tick before
                // `RuntimeTickTransaction::commit` advances
                // `last_processed_tick`. Offline catch-up may move by more than
                // one minute, so structural validation cannot compare these
                // versions; the transaction gateway writes both atomically.
            }
            _ => return Err(LeaderAiRuntimeError::MalformedRuntimeState),
        }
        validate_leaf("intents", &self.intents)?;
        self.cat_capabilities
            .validate()
            .map_err(|_| LeaderAiRuntimeError::MalformedCapabilities)?;
        self.families
            .validate()
            .map_err(|_| LeaderAiRuntimeError::MalformedFamily)?;
        self.governance
            .validate()
            .map_err(|_| LeaderAiRuntimeError::MalformedGovernance)?;
        validate_leaf("research authority", &self.research)?;
        self.storage
            .validate()
            .map_err(|_| LeaderAiRuntimeError::MalformedStorage)?;
        if self
            .purpose_bound_storage
            .keys()
            .any(|identity| self.storage.location(identity).is_none())
        {
            return Err(LeaderAiRuntimeError::DanglingStorageIdentity);
        }
        validate_leaf("black hole", &self.hole)?;
        self.divine_hole
            .validate()
            .map_err(|_| LeaderAiRuntimeError::MalformedHole)?;
        self.divine_hole
            .binding
            .validate_hole(&self.hole)
            .map_err(|_| LeaderAiRuntimeError::MalformedHole)?;
        if self.hole.micro_void_balance != 0 {
            return Err(LeaderAiRuntimeError::ShadowVoidBalance);
        }
        validate_leaf("divine boosts", &self.boosts)?;
        self.trade
            .validate()
            .map_err(|_| LeaderAiRuntimeError::MalformedTrade)?;
        validate_leaf("prosthetics", &self.prosthetics)?;
        self.player_directives
            .validate()
            .map_err(|_| LeaderAiRuntimeError::MalformedRuntimeState)?;
        self.diagnostics
            .validate()
            .map_err(|_| LeaderAiRuntimeError::MalformedDiagnostics)?;
        self.scheduling.validate(&self.intents, &self.storage)?;
        validate_construction(self)?;
        self.construction_miracles
            .validate(&self.construction_projects)
            .map_err(|_| LeaderAiRuntimeError::MalformedConstruction)?;
        validate_cat_partitions(self)?;
        validate_outcomes(self)?;
        for (tick, receipt) in &self.phase_receipts {
            if tick != &receipt.tick
                || receipt.phases.as_slice() != ProtectedRuntimePhase::ORDER.as_slice()
                || receipt.planner_phases.as_slice() != PlannerPhase::ORDER.as_slice()
            {
                return Err(LeaderAiRuntimeError::MalformedPhaseReceipt);
            }
        }
        if self.last_processed_tick != self.phase_receipts.keys().next_back().copied()
            && !(self.last_processed_tick.is_none() && self.phase_receipts.is_empty())
        {
            return Err(LeaderAiRuntimeError::MalformedPhaseReceipt);
        }
        Ok(())
    }
}

impl Default for LeaderAiRuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

/// Owns a full staged runtime clone. Dropping it rolls back every authority;
/// only `commit` can replace the live aggregate.
#[derive(Debug, Clone)]
pub struct RuntimeTickTransaction {
    tick: u64,
    staged: LeaderAiRuntimeState,
    completed_phases: Vec<ProtectedRuntimePhase>,
}

impl RuntimeTickTransaction {
    pub fn enter(
        &mut self,
        phase: ProtectedRuntimePhase,
    ) -> Result<&mut LeaderAiRuntimeState, LeaderAiRuntimeError> {
        let expected = ProtectedRuntimePhase::ORDER
            .get(self.completed_phases.len())
            .copied()
            .ok_or(LeaderAiRuntimeError::PhaseOrderViolation)?;
        if phase != expected {
            return Err(LeaderAiRuntimeError::PhaseOrderViolation);
        }
        self.completed_phases.push(phase);
        Ok(&mut self.staged)
    }

    pub fn commit(
        mut self,
        target: &mut LeaderAiRuntimeState,
    ) -> Result<RuntimePhaseReceipt, LeaderAiRuntimeError> {
        if self.completed_phases.as_slice() != ProtectedRuntimePhase::ORDER.as_slice() {
            return Err(LeaderAiRuntimeError::IncompletePhaseTransaction);
        }
        let receipt = RuntimePhaseReceipt {
            tick: self.tick,
            phases: self.completed_phases,
            planner_phases: PlannerPhase::ORDER.to_vec(),
            report_version: self
                .staged
                .last_report
                .as_ref()
                .map(|report| report.report_version),
        };
        self.staged.last_processed_tick = Some(self.tick);
        self.staged
            .phase_receipts
            .insert(self.tick, receipt.clone());
        while self.staged.phase_receipts.len() > MAX_PHASE_RECEIPTS {
            let oldest = self
                .staged
                .phase_receipts
                .keys()
                .next()
                .copied()
                .expect("non-empty bounded phase receipts");
            self.staged.phase_receipts.remove(&oldest);
        }
        self.staged.validate()?;
        *target = self.staged;
        Ok(receipt)
    }
}

fn capability_registration(cat: &Cat, physical: &CatPhysicalState) -> CatCapabilityRegistration {
    let attributes = physical.traits.attributes;
    let mut affinities = BTreeMap::new();
    let mut skills = BTreeMap::new();
    for labor in Labor::ALL {
        let skill_id = legacy_labor_skill_id(*labor);
        if cat.preferred_labors.contains(labor) {
            affinities.insert(skill_id.to_owned(), LaborAffinity::Preferred);
        }
        let legacy_xp = cat.skill(*labor);
        if legacy_xp.is_finite() && legacy_xp > 0.0 {
            let centi = (legacy_xp * 100.0).round().clamp(0.0, u64::MAX as f64) as u64;
            skills.insert(skill_id.to_owned(), SkillProgress::new(centi));
        }
    }
    CatCapabilityRegistration {
        cat_id: cat.id.clone(),
        attributes: CapabilityAttributes::new(
            attributes.attack.get(),
            attributes.defense.get(),
            attributes.hunting.get(),
            attributes.medicine.get(),
            attributes.cleaning.get(),
            attributes.building.get(),
            attributes.leadership.get(),
            attributes.vision.get(),
            10,
            10,
        )
        .expect("legacy attributes and baselines are clamped to 1..=20"),
        labor: LaborAffinityProfile {
            affinities,
            family_enterprise_skill_ids: BTreeSet::new(),
        },
        skills,
        office_duty_minutes: BTreeMap::new(),
    }
}

fn register_family_cat(
    families: &mut FamilyAuthorityState,
    cat: &Cat,
) -> Result<(), LeaderAiRuntimeError> {
    let parents = cat
        .parent_ids
        .iter()
        .flatten()
        .filter(|parent| families.cats.contains_key(parent.as_str()))
        .cloned()
        .take(2)
        .collect::<Vec<_>>();
    let command = FamilyCommand {
        receipt_id: format!("runtime_birth:{}", cat.id),
        expected_revision: families.revision,
        operation: FamilyOperation::RegisterBirth(BirthRegistration {
            newborn_cat_id: cat.id.clone(),
            life_stage: family_life_stage(cat.age_hours),
            first_parent_id: parents.first().cloned(),
            second_parent_id: parents.get(1).cloned(),
            attribute_authority_ref: format!("cat_capabilities:{}", cat.id),
            relational_analytical_authority_ref: format!("governance_axis:{}", cat.id),
        }),
    };
    families
        .apply(command)
        .map_err(|_| LeaderAiRuntimeError::MalformedFamily)?;
    if cat.death_time.is_some() {
        families
            .apply(FamilyCommand {
                receipt_id: format!("runtime_death:{}", cat.id),
                expected_revision: families.revision,
                operation: FamilyOperation::RecordDeath {
                    cat_id: cat.id.clone(),
                },
            })
            .map_err(|_| LeaderAiRuntimeError::MalformedFamily)?;
    }
    Ok(())
}

fn governance_fact(
    cat: &Cat,
    physical: &CatPhysicalState,
) -> Result<GovernanceResidentFact, LeaderAiRuntimeError> {
    let attributes = physical.traits.attributes;
    let axis_roll = planner_roll(
        0,
        PlannerRngStream::Personality,
        [&cat.colony_id, &cat.id, "relational_analytical"],
    );
    let axis_value = i32::try_from(axis_roll.next_seed % 20_001)
        .unwrap_or(0)
        .saturating_sub(10_000) as i16;
    let axis = RelationalAnalyticalAxis::new(axis_value)
        .map_err(|_| LeaderAiRuntimeError::MalformedGovernance)?;
    let leadership = score_from_attribute(attributes.leadership.get());
    let charisma = score_from_attribute(10);
    let intelligence = score_from_attribute(10);
    let merit = CivicMeritMetrics {
        governance: 0,
        inherited_leadership: leadership,
        effective_charisma: charisma,
        intelligence,
        office_breadth: 0,
        leadership_service_record: 0,
        relevant_traits: 0,
    };
    Ok(GovernanceResidentFact {
        cat_id: cat.id.clone(),
        household_id: format!("household_{}", cat.id),
        life_stage: governance_life_stage(cat.age_hours),
        resident: true,
        alive: cat.death_time.is_none(),
        barred: false,
        guardian_id: None,
        axis,
        merit,
        ballot_facts: CandidateBallotFacts {
            charisma,
            care: score_from_attribute(attributes.medicine.get()),
            trust: leadership,
            social_conduct: SCORE_SCALE / 2,
            personality_compatibility: SCORE_SCALE / 2,
            governance: 0,
            intelligence,
            office_experience: 0,
            skill: score_from_attribute(attributes.leadership.get()),
            results: 0,
        },
        job_id: None,
        office_id: None,
        residence_id: None,
        enterprise_id: None,
        partnership_id: None,
        carried_cargo_ids: Vec::new(),
        reservation_ids: Vec::new(),
        owned_item_ids: Vec::new(),
        equipped_item_ids: Vec::new(),
    })
}

const fn score_from_attribute(attribute: u8) -> u16 {
    attribute as u16 * 500
}

fn family_life_stage(age_hours: f64) -> FamilyLifeStage {
    match crate::age::get_life_stage(age_hours) {
        LegacyLifeStage::Kitten => FamilyLifeStage::Kitten,
        LegacyLifeStage::Young => FamilyLifeStage::Young,
        LegacyLifeStage::Adult => FamilyLifeStage::Adult,
        LegacyLifeStage::Elder => FamilyLifeStage::Elder,
    }
}

fn governance_life_stage(age_hours: f64) -> GovernanceLifeStage {
    match crate::age::get_life_stage(age_hours) {
        LegacyLifeStage::Kitten => GovernanceLifeStage::Kitten,
        LegacyLifeStage::Young => GovernanceLifeStage::Young,
        LegacyLifeStage::Adult => GovernanceLifeStage::Adult,
        LegacyLifeStage::Elder => GovernanceLifeStage::Elder,
    }
}

const fn legacy_labor_skill_id(labor: Labor) -> &'static str {
    match labor {
        Labor::Hunt => "hunting",
        Labor::Fishing => "fishing",
        Labor::Build => "construction",
        Labor::Ritual => "ritual",
        Labor::Fight => "fighting",
        Labor::Train => "training",
        Labor::Quarry => "quarrying",
        Labor::Woodcut => "woodcutting",
        Labor::Forage => "foraging",
        Labor::FetchWater => "waterwork",
        Labor::Mill => "milling",
        Labor::Process | Labor::Craft => "crafting",
        Labor::Textile => "textiles",
        Labor::Metalwork => "metalworking",
        Labor::Farm => "farming",
        Labor::Haul => "hauling",
        Labor::Research => "research",
        Labor::Scout => "scouting",
    }
}

fn validate_physical_task_request(
    request: &PhysicalTaskExecutionRequest,
) -> Result<(), LeaderAiRuntimeError> {
    if request.workers.len() > MAX_PHYSICAL_TASK_WORKER_REPORTS
        || request.cargo.resource_id.is_empty()
        || request.cargo.quantity == 0
        || request
            .workers
            .iter()
            .any(|worker| worker.cat_id.trim().is_empty())
    {
        return Err(LeaderAiRuntimeError::MalformedPhysicalTaskRequest);
    }
    let mut worker_ids = request
        .workers
        .iter()
        .map(|worker| worker.cat_id.as_str())
        .collect::<Vec<_>>();
    worker_ids.sort_unstable();
    if worker_ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(LeaderAiRuntimeError::MalformedPhysicalTaskRequest);
    }
    Ok(())
}

fn select_physical_task_worker(
    capabilities: &CatCapabilityAuthority,
    task_id: &TaskId,
    request: &PhysicalTaskExecutionRequest,
) -> Result<Option<String>, LeaderAiRuntimeError> {
    let slot = WorkforceSlot {
        task_id: task_id.as_str().to_owned(),
        slot_id: request.resolved.work_slot().stable_id.clone(),
        priority: request.priority,
    };
    let edges = request
        .workers
        .iter()
        .map(|worker| WorkforceEdge {
            cat_id: worker.cat_id.clone(),
            task_id: task_id.as_str().to_owned(),
            slot_id: request.resolved.work_slot().stable_id.clone(),
            score: worker.suitability_score,
            eligible: worker.alive
                && worker.capable
                && capabilities.cat_report(&worker.cat_id).is_some(),
            willingness: if worker.willing {
                WillingnessDecision::Willing
            } else {
                WillingnessDecision::Refused(RefusalReason::CriticalStress)
            },
            current_assignment: false,
        })
        .collect::<Vec<_>>();
    let matched = match_workforce(&[slot], &edges)
        .map_err(|_| LeaderAiRuntimeError::MalformedPhysicalTaskRequest)?;
    Ok(matched
        .assignments
        .into_iter()
        .next()
        .map(|assignment| assignment.cat_id))
}

fn physical_worker_block_reason(workers: &[PhysicalTaskWorkerReport]) -> PhysicalTaskBlockReason {
    if workers.iter().any(|worker| worker.alive && worker.capable) {
        PhysicalTaskBlockReason::NoWillingWorker
    } else {
        PhysicalTaskBlockReason::NoLivingCapableWorker
    }
}

fn physical_task_category_matches(task: TaskCategory, spatial: SpatialTaskCategory) -> bool {
    // Construction, road, and Hole offering tasks are deliberately excluded
    // here. Their authorities own additional domain transitions (bills/work
    // stages and feed/upgrade receipts) which this single-identity transport
    // increment cannot truthfully synthesize. They are therefore
    // deterministically blocked before pickup by the caller above, rather
    // than gaining generic XP while silently skipping their domain commit.
    matches!(
        (task, spatial),
        (TaskCategory::Hunt, SpatialTaskCategory::Hunt)
            | (TaskCategory::FetchWater, SpatialTaskCategory::FetchWater)
            | (TaskCategory::Fish, SpatialTaskCategory::Fish)
            | (TaskCategory::Quarry, SpatialTaskCategory::Quarry)
            | (TaskCategory::Logging, SpatialTaskCategory::Logging)
            | (
                TaskCategory::StationWork,
                SpatialTaskCategory::StationWork(_)
            )
            | (
                TaskCategory::WorkshopWork,
                SpatialTaskCategory::WorkshopWork
            )
            | (TaskCategory::FarmWork, SpatialTaskCategory::FarmWork)
            | (
                TaskCategory::HaulDelivery,
                SpatialTaskCategory::EmergencySupply(_)
            )
    )
}

fn physical_objective_claim_mode(resolved: &ResolvedSpatialTask) -> ClaimMode {
    if matches!(
        resolved.category,
        SpatialTaskCategory::Logging
            | SpatialTaskCategory::Construction(_)
            | SpatialTaskCategory::RoadConstruction
    ) {
        ClaimMode::Exclusive
    } else {
        ClaimMode::Capacity {
            units: resolved.source_units,
            capacity: resolved.source_capacity,
        }
    }
}

fn storage_identity_claim_key(
    identity: &StorageIdentity,
) -> Result<PlannerId, LeaderAiRuntimeError> {
    let stable = serde_json::to_string(identity)
        .map_err(|_| LeaderAiRuntimeError::MalformedPhysicalTaskRequest)?;
    Ok(PlannerId::derive("storage_identity", [stable.as_str()]))
}

fn physical_route_claim_key(
    resolved: &ResolvedSpatialTask,
) -> Result<PlannerId, LeaderAiRuntimeError> {
    let source_route = resolved.source_to_work_route.stable_id();
    let delivery_route = resolved.work_to_delivery_route.stable_id();
    if source_route.is_empty() || delivery_route.is_empty() {
        return Err(LeaderAiRuntimeError::PhysicalTaskReservationInvalid);
    }
    Ok(PlannerId::derive(
        "physical_task_route",
        [source_route, delivery_route],
    ))
}

fn physical_route_ids(resolved: &ResolvedSpatialTask) -> Vec<String> {
    vec![
        resolved.source_to_work_route.stable_id().to_owned(),
        resolved.work_to_delivery_route.stable_id().to_owned(),
    ]
}

fn physical_storage_endpoint_matches(
    address: &StorageAddress,
    resolved: &ResolvedSpatialTask,
) -> bool {
    match (resolved.delivery_endpoint(), address) {
        (SiteRef::Stockpile { stockpile_id, .. }, StorageAddress::Loose { zone_id, .. }) => {
            stockpile_id == zone_id
        }
        (delivery, StorageAddress::PurposeCargo { site_id }) => delivery.stable_id() == site_id,
        _ => false,
    }
}

fn block_task_before_pickup(
    scheduling: &mut SchedulingRuntimeAggregate,
    task_id: &TaskId,
    reason: RuntimeBlockReason,
    now_tick: u64,
) -> Result<(), LeaderAiRuntimeError> {
    let SchedulingRuntimeAggregate {
        visible_tasks,
        reservations,
        ..
    } = scheduling;
    visible_tasks
        .get_mut(task_id)
        .ok_or(LeaderAiRuntimeError::MissingPhysicalTask)?
        .block_before_pickup(reason, reservations, now_tick)
        .map_err(|_| LeaderAiRuntimeError::MalformedRuntimeState)
}

fn cancel_task_before_pickup(
    scheduling: &mut SchedulingRuntimeAggregate,
    task_id: &TaskId,
    now_tick: u64,
) -> Result<(), LeaderAiRuntimeError> {
    let SchedulingRuntimeAggregate {
        visible_tasks,
        reservations,
        ..
    } = scheduling;
    visible_tasks
        .get_mut(task_id)
        .ok_or(LeaderAiRuntimeError::MissingPhysicalTask)?
        .cancel(reservations, now_tick)
        .map_err(|_| LeaderAiRuntimeError::MalformedRuntimeState)
}

fn recover_task_after_pickup(
    scheduling: &mut SchedulingRuntimeAggregate,
    task_id: &TaskId,
    reason: RuntimeBlockReason,
    safe_owned_stockpile: Option<&SiteRef>,
    last_site_id: &str,
    now_tick: u64,
) -> Result<(), LeaderAiRuntimeError> {
    let SchedulingRuntimeAggregate {
        visible_tasks,
        reservations,
        ..
    } = scheduling;
    visible_tasks
        .get_mut(task_id)
        .ok_or(LeaderAiRuntimeError::MissingPhysicalTask)?
        .recover_after_pickup(
            reason,
            safe_owned_stockpile,
            last_site_id,
            reservations,
            now_tick,
        )
        .map_err(|_| LeaderAiRuntimeError::MalformedRuntimeState)
}

fn complete_task(
    scheduling: &mut SchedulingRuntimeAggregate,
    task_id: &TaskId,
    now_tick: u64,
) -> Result<(), LeaderAiRuntimeError> {
    let SchedulingRuntimeAggregate {
        visible_tasks,
        reservations,
        ..
    } = scheduling;
    visible_tasks
        .get_mut(task_id)
        .ok_or(LeaderAiRuntimeError::MissingPhysicalTask)?
        .complete(reservations, now_tick)
        .map_err(|_| LeaderAiRuntimeError::MalformedRuntimeState)
}

fn validate_construction(state: &LeaderAiRuntimeState) -> Result<(), LeaderAiRuntimeError> {
    if state.construction_projects.len() != state.construction_storage_identities.len() {
        return Err(LeaderAiRuntimeError::MalformedConstruction);
    }
    if state.construction_runtime_ticks.len() > MAX_CONSTRUCTION_PROJECTS
        || state.construction_materializations.len() > MAX_CONSTRUCTION_PROJECTS
        || state
            .construction_runtime_ticks
            .keys()
            .any(|project_id| !state.construction_projects.contains_key(project_id))
        || state
            .construction_materializations
            .iter()
            .any(|(project_id, record)| {
                record.project_id() != project_id
                    || !state
                        .construction_projects
                        .get(project_id)
                        .is_some_and(|project| {
                            project.stage
                                == crate::construction_stages::ConstructionStage::Operational
                        })
            })
    {
        return Err(LeaderAiRuntimeError::MalformedConstruction);
    }
    for (project_id, project) in &state.construction_projects {
        if project_id != &project.project_id {
            return Err(LeaderAiRuntimeError::MalformedConstruction);
        }
        project
            .validate()
            .map_err(|_| LeaderAiRuntimeError::MalformedConstruction)?;
        let identities = state
            .construction_storage_identities
            .get(project_id)
            .ok_or(LeaderAiRuntimeError::MalformedConstruction)?;
        if identities
            .iter()
            .any(|identity| state.storage.location(identity).is_none())
        {
            return Err(LeaderAiRuntimeError::DanglingStorageIdentity);
        }
        if project.target_kind == ConstructionTargetKind::HoleUpgrade {
            if project.building_type.is_some() || project.footprint.tiles.len() != 9 {
                return Err(LeaderAiRuntimeError::MalformedConstruction);
            }
            continue;
        }
        let building_type = project
            .building_type
            .ok_or(LeaderAiRuntimeError::MalformedConstruction)?;
        let target_level = u8::try_from(project.target_level)
            .map_err(|_| LeaderAiRuntimeError::MalformedConstruction)?;
        let request = match project.target_kind {
            ConstructionTargetKind::Building => BlueprintRequest::NewBuilding(building_type),
            ConstructionTargetKind::BuildingUpgrade => BlueprintRequest::BuildingUpgrade {
                building_type,
                target_level,
            },
            ConstructionTargetKind::HoleUpgrade => unreachable!("handled above"),
        };
        let blueprint =
            resolve_blueprint(request).map_err(|_| LeaderAiRuntimeError::MalformedConstruction)?;
        if project.scaffold_tier != blueprint.scaffold_tier
            || project.target_level != u32::from(blueprint.target_level)
            || project.footprint.width != blueprint.footprint.width
            || project.footprint.height != blueprint.footprint.height
            || project.footprint.tiles.len()
                != usize::try_from(blueprint.footprint.width * blueprint.footprint.height)
                    .unwrap_or(0)
            || !construction_bills_match_recipe(&project.bills, &blueprint.fresh_bills())
            || project.original_total_work_ms != blueprint.base_work_duration_ms
        {
            return Err(LeaderAiRuntimeError::MalformedConstruction);
        }
    }
    Ok(())
}

fn construction_bills_match_recipe(
    current: &ConstructionBills,
    recipe: &ConstructionBills,
) -> bool {
    stage_bill_matches_recipe(&current.scaffold, &recipe.scaffold)
        && stage_bill_matches_recipe(&current.structure, &recipe.structure)
        && stage_bill_matches_recipe(&current.fit_out, &recipe.fit_out)
}

fn stage_bill_matches_recipe(
    current: &ConstructionStageBill,
    recipe: &ConstructionStageBill,
) -> bool {
    current.lines.len() == recipe.lines.len()
        && current
            .lines
            .iter()
            .zip(&recipe.lines)
            .all(|(left, right)| {
                left.content_id == right.content_id && left.required_units == right.required_units
            })
}

fn validate_cat_partitions(state: &LeaderAiRuntimeState) -> Result<(), LeaderAiRuntimeError> {
    let capability_ids = state
        .cat_capabilities
        .report()
        .cats
        .into_iter()
        .map(|cat| cat.cat_id)
        .collect::<BTreeSet<_>>();
    let physical_ids = state.cat_physical.keys().cloned().collect::<BTreeSet<_>>();
    let family_ids = state.families.cats.keys().cloned().collect::<BTreeSet<_>>();
    if capability_ids != physical_ids
        || capability_ids != family_ids
        || capability_ids
            .iter()
            .any(|cat_id| state.governance.resident(cat_id).is_none())
        || state.cat_physical.iter().any(|(id, cat)| id != &cat.cat_id)
    {
        return Err(LeaderAiRuntimeError::CatPartitionMismatch);
    }
    Ok(())
}

fn validate_outcomes(state: &LeaderAiRuntimeState) -> Result<(), LeaderAiRuntimeError> {
    for (task_id, binding) in &state.task_outcomes {
        if task_id != &binding.task_id
            || binding.cat_id.is_empty()
            || binding.capability_receipt_id.is_empty()
            || !state.cat_physical.contains_key(&binding.cat_id)
        {
            return Err(LeaderAiRuntimeError::MalformedOutcomeBinding);
        }
    }
    Ok(())
}

fn validate_cargo_reference(
    task: &VisibleTaskRuntime,
    known_cargo_site_ids: &BTreeSet<String>,
) -> Result<(), LeaderAiRuntimeError> {
    let Some(cargo) = &task.cargo else {
        return Ok(());
    };
    match &cargo.location {
        CargoLocation::ReservedAtSource { source_id } => {
            if !known_cargo_site_ids.contains(source_id) || task.reservation_id.is_none() {
                return Err(LeaderAiRuntimeError::DanglingCargoReference);
            }
        }
        CargoLocation::Carried { cat_id } => {
            if !task.assigned_cat_ids.contains(cat_id) {
                return Err(LeaderAiRuntimeError::DanglingCargoReference);
            }
        }
        CargoLocation::DepositedAtEndpoint { endpoint_id } => {
            if !known_cargo_site_ids.contains(endpoint_id) {
                return Err(LeaderAiRuntimeError::DanglingCargoReference);
            }
        }
        CargoLocation::SalvagedAtStockpile { stockpile_id } => {
            if !known_cargo_site_ids.contains(stockpile_id) {
                return Err(LeaderAiRuntimeError::DanglingCargoReference);
            }
        }
        CargoLocation::Stranded { site_id } => {
            if !known_cargo_site_ids.contains(site_id) {
                return Err(LeaderAiRuntimeError::DanglingCargoReference);
            }
        }
    }
    Ok(())
}

fn rounded_legacy_stat(value: f64) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    value
        .round()
        .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

const fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    let mut index = 0;
    while index < bytes.len() {
        hash ^= bytes[index] as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        index += 1;
    }
    hash
}

fn validate_leaf<T>(name: &'static str, value: &T) -> Result<(), LeaderAiRuntimeError>
where
    T: Serialize + DeserializeOwned,
{
    let value = serde_json::to_value(value).map_err(|_| LeaderAiRuntimeError::LeafInvalid(name))?;
    serde_json::from_value::<T>(value)
        .map(|_| ())
        .map_err(|_| LeaderAiRuntimeError::LeafInvalid(name))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderAiRuntimeError {
    MalformedRuntimeState,
    WrongPartition,
    BoundExceeded,
    LeafInvalid(&'static str),
    MalformedCapabilities,
    MalformedFamily,
    MalformedGovernance,
    MalformedConstruction,
    MalformedStorage,
    MalformedHole,
    MalformedTrade,
    MalformedDiagnostics,
    ShadowVoidBalance,
    ReportProjectionMismatch,
    CatPartitionMismatch,
    TaskIdMismatch,
    DanglingTaskIntent,
    DanglingTaskReservation,
    DanglingResolvedTask,
    DanglingWorldReservation,
    DanglingCargoReference,
    DanglingStorageIdentity,
    MalformedOutcomeBinding,
    OutcomeReplayConflict,
    MissingPhysicalTask,
    MalformedPhysicalTaskRequest,
    PhysicalTaskSpatialMismatch,
    UnsupportedPhysicalTaskCategory,
    UnsupportedPhysicalCargoIdentity,
    PhysicalTaskCargoMismatch,
    PhysicalTaskReservationInvalid,
    TickAlreadyProcessed,
    PhaseOrderViolation,
    IncompletePhaseTransaction,
    MalformedPhaseReceipt,
}

impl std::fmt::Display for LeaderAiRuntimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Leader AI runtime error: {self:?}")
    }
}

impl std::error::Error for LeaderAiRuntimeError {}
