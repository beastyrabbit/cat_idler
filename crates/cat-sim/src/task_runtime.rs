//! Persisted multi-stage visible tasks specified by
//! `docs/leader-ai-overhaul/spatial-task-contract.md`.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    planner_core::{IntentId, PlannerId},
    reservation_transaction::{ReservationId, ReservationLedger},
    spatial_tasks::{SiteRef, SpatialBlockReason, SpatialObjective},
};

pub const TASK_RUNTIME_SCHEMA_VERSION: u32 = 1;
pub const TASK_PROGRESS_MAX_BASIS_POINTS: u16 = 10_000;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TaskId(PlannerId);

impl TaskId {
    #[must_use]
    pub fn derive(colony_id: &str, intent_id: &IntentId, occurrence: u32) -> Self {
        let occurrence = occurrence.to_string();
        Self(PlannerId::derive(
            "visible_task",
            [colony_id, intent_id.as_str(), occurrence.as_str()],
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskCategory {
    Hunt,
    FetchWater,
    Fish,
    Quarry,
    Logging,
    Replant,
    BuildingConstruction,
    RoadConstruction,
    StationWork,
    WorkshopWork,
    FarmWork,
    HaulDelivery,
    StockpileTransfer,
    FibreForage,
    Scout,
    Expansion,
    OfferingRitual,
    Training,
    Accounting,
    Eat,
    Drink,
    Sleep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStage {
    Resolve,
    Reserve,
    TravelToSource,
    Pickup,
    TravelToWork,
    Work,
    TravelToEndpoint,
    Deposit,
    Complete,
    Blocked,
    Cancelled,
}

impl TaskStage {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Complete | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeBlockReason {
    Spatial(SpatialBlockReason),
    ReservationLost,
    RouteClosedBeforePickup,
    RouteClosedWithCargo,
    SourceRemoved,
    EndpointRemoved,
    WorkerRefused,
    WorkerDied,
    WorkerIncapacitated,
    CargoRecoveryRequired,
    InvalidLegacySite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartRevalidationOutcome {
    Unchanged,
    ActiveReservationValid,
    BlockedBeforePickup,
    BlockedCargoPreserved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CargoLocation {
    ReservedAtSource { source_id: String },
    Carried { cat_id: String },
    DepositedAtEndpoint { endpoint_id: String },
    SalvagedAtStockpile { stockpile_id: String },
    Stranded { site_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskCargo {
    pub cargo_id: String,
    pub resource_id: String,
    pub quantity: u64,
    pub location: CargoLocation,
}

impl TaskCargo {
    fn validate(&self) -> Result<(), TaskRuntimeError> {
        let location_id = match &self.location {
            CargoLocation::ReservedAtSource { source_id } => source_id,
            CargoLocation::Carried { cat_id } => cat_id,
            CargoLocation::DepositedAtEndpoint { endpoint_id } => endpoint_id,
            CargoLocation::SalvagedAtStockpile { stockpile_id } => stockpile_id,
            CargoLocation::Stranded { site_id } => site_id,
        };
        if self.cargo_id.is_empty()
            || self.resource_id.is_empty()
            || self.quantity == 0
            || location_id.is_empty()
        {
            return Err(TaskRuntimeError::MalformedState);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibleTaskRuntime {
    pub schema_version: u32,
    pub id: TaskId,
    pub occurrence: u32,
    pub colony_id: String,
    pub intent_id: IntentId,
    pub category: TaskCategory,
    pub stage: TaskStage,
    pub spatial: SpatialObjective,
    pub assigned_cat_ids: BTreeSet<String>,
    pub work_slot_by_cat: BTreeMap<String, String>,
    /// Ordered stable route/segment identities; sequence is authoritative.
    pub route_ids: Vec<String>,
    pub reservation_id: Option<ReservationId>,
    pub progress_basis_points: u16,
    pub cargo: Option<TaskCargo>,
    pub blocked_reason: Option<RuntimeBlockReason>,
    pub updated_tick: u64,
}

impl VisibleTaskRuntime {
    pub fn resolved(
        colony_id: impl Into<String>,
        intent_id: IntentId,
        occurrence: u32,
        category: TaskCategory,
        spatial: SpatialObjective,
        route_ids: Vec<String>,
        now_tick: u64,
    ) -> Result<Self, TaskRuntimeError> {
        let colony_id = colony_id.into();
        let id = TaskId::derive(&colony_id, &intent_id, occurrence);
        let mut task = Self {
            schema_version: TASK_RUNTIME_SCHEMA_VERSION,
            id,
            occurrence,
            colony_id,
            intent_id,
            category,
            stage: TaskStage::Resolve,
            spatial,
            assigned_cat_ids: BTreeSet::new(),
            work_slot_by_cat: BTreeMap::new(),
            route_ids,
            reservation_id: None,
            progress_basis_points: 0,
            cargo: None,
            blocked_reason: None,
            updated_tick: now_tick,
        };
        if let Some(reason) = task.spatial.blocked_reason {
            task.stage = TaskStage::Blocked;
            task.blocked_reason = Some(RuntimeBlockReason::Spatial(reason));
        }
        task.validate()?;
        Ok(task)
    }

    #[must_use]
    pub fn emits_world_marker(&self) -> bool {
        self.spatial.objective.is_some()
            && (self.route_ids.len() == 2
                || self.category == TaskCategory::BuildingConstruction)
            && !matches!(
                self.stage,
                TaskStage::Complete | TaskStage::Blocked | TaskStage::Cancelled
            )
    }

    #[must_use]
    pub fn is_worker_busy(&self, cat_id: &str, ledger: &ReservationLedger) -> bool {
        self.reservation_id
            .as_ref()
            .is_some_and(|reservation| ledger.contains(reservation))
            && self.assigned_cat_ids.contains(cat_id)
    }

    pub fn activate(
        &mut self,
        ledger: &ReservationLedger,
        reservation_id: ReservationId,
        assignments: impl IntoIterator<Item = (String, String)>,
        now_tick: u64,
    ) -> Result<(), TaskRuntimeError> {
        if !matches!(self.stage, TaskStage::Resolve | TaskStage::Reserve) {
            return Err(TaskRuntimeError::InvalidTransition);
        }
        if self.spatial.blocked_reason.is_some()
            || self.spatial.objective.is_none()
            || self.spatial.work_positions.is_empty()
            || self.spatial.delivery_endpoint.is_none()
        {
            return Err(TaskRuntimeError::IncompleteSpatialContract);
        }
        if !ledger.contains(&reservation_id) {
            return Err(TaskRuntimeError::ReservationNotCommitted);
        }
        let known_slots = self
            .spatial
            .work_positions
            .iter()
            .map(|slot| slot.stable_id.as_str())
            .collect::<BTreeSet<_>>();
        let raw_assignments = assignments.into_iter().collect::<Vec<_>>();
        let assignments = raw_assignments.iter().cloned().collect::<BTreeMap<_, _>>();
        if raw_assignments.len() != 1
            || assignments.len() != raw_assignments.len()
            || assignments
                .iter()
                .any(|(cat, slot)| cat.is_empty() || !known_slots.contains(slot.as_str()))
        {
            return Err(TaskRuntimeError::InvalidAssignment);
        }
        self.assigned_cat_ids = assignments.keys().cloned().collect();
        self.work_slot_by_cat = assignments;
        self.reservation_id = Some(reservation_id);
        self.stage = TaskStage::TravelToSource;
        self.blocked_reason = None;
        self.updated_tick = now_tick;
        Ok(())
    }

    pub fn begin_reservation(&mut self, now_tick: u64) -> Result<(), TaskRuntimeError> {
        if self.stage != TaskStage::Resolve {
            return Err(TaskRuntimeError::InvalidTransition);
        }
        if self.spatial.blocked_reason.is_some()
            || self.spatial.objective.is_none()
            || self.spatial.work_positions.is_empty()
            || self.spatial.delivery_endpoint.is_none()
        {
            return Err(TaskRuntimeError::IncompleteSpatialContract);
        }
        self.stage = TaskStage::Reserve;
        self.updated_tick = now_tick;
        Ok(())
    }

    pub fn advance(&mut self, next: TaskStage, now_tick: u64) -> Result<(), TaskRuntimeError> {
        let allowed = matches!(
            (self.stage, next),
            (TaskStage::TravelToSource, TaskStage::Pickup)
                | (TaskStage::Pickup, TaskStage::TravelToWork)
                | (TaskStage::Pickup, TaskStage::TravelToEndpoint)
                | (TaskStage::TravelToWork, TaskStage::Work)
                | (TaskStage::Work, TaskStage::TravelToEndpoint)
                | (TaskStage::TravelToEndpoint, TaskStage::Deposit)
        );
        if !allowed {
            return Err(TaskRuntimeError::InvalidTransition);
        }
        self.stage = next;
        self.updated_tick = now_tick;
        Ok(())
    }

    pub fn reserve_cargo_at_source(
        &mut self,
        cargo_id: impl Into<String>,
        resource_id: impl Into<String>,
        quantity: u64,
    ) -> Result<(), TaskRuntimeError> {
        if !matches!(
            self.stage,
            TaskStage::Resolve | TaskStage::Reserve | TaskStage::TravelToSource
        ) {
            return Err(TaskRuntimeError::InvalidTransition);
        }
        if self.cargo.is_some() {
            return Err(TaskRuntimeError::CargoAlreadyPresent);
        }
        let source_id = self
            .spatial
            .objective
            .as_ref()
            .ok_or(TaskRuntimeError::IncompleteSpatialContract)?
            .stable_id()
            .to_owned();
        let cargo = TaskCargo {
            cargo_id: cargo_id.into(),
            resource_id: resource_id.into(),
            quantity,
            location: CargoLocation::ReservedAtSource { source_id },
        };
        cargo.validate()?;
        self.cargo = Some(cargo);
        Ok(())
    }

    pub fn pickup(&mut self, cat_id: &str, now_tick: u64) -> Result<(), TaskRuntimeError> {
        if self.stage != TaskStage::Pickup || !self.assigned_cat_ids.contains(cat_id) {
            return Err(TaskRuntimeError::InvalidTransition);
        }
        let cargo = self.cargo.as_mut().ok_or(TaskRuntimeError::CargoMissing)?;
        if !matches!(cargo.location, CargoLocation::ReservedAtSource { .. }) {
            return Err(TaskRuntimeError::InvalidCargoLocation);
        }
        cargo.location = CargoLocation::Carried {
            cat_id: cat_id.to_owned(),
        };
        self.updated_tick = now_tick;
        Ok(())
    }

    pub fn deposit(&mut self, now_tick: u64) -> Result<(), TaskRuntimeError> {
        if self.stage != TaskStage::Deposit {
            return Err(TaskRuntimeError::InvalidTransition);
        }
        let endpoint_id = self
            .spatial
            .delivery_endpoint
            .as_ref()
            .ok_or(TaskRuntimeError::IncompleteSpatialContract)?
            .stable_id()
            .to_owned();
        let cargo = self.cargo.as_mut().ok_or(TaskRuntimeError::CargoMissing)?;
        if !matches!(cargo.location, CargoLocation::Carried { .. }) {
            return Err(TaskRuntimeError::InvalidCargoLocation);
        }
        cargo.location = CargoLocation::DepositedAtEndpoint { endpoint_id };
        self.updated_tick = now_tick;
        Ok(())
    }

    pub fn complete(
        &mut self,
        ledger: &mut ReservationLedger,
        now_tick: u64,
    ) -> Result<(), TaskRuntimeError> {
        if self.stage != TaskStage::Deposit
            || self.cargo.as_ref().is_some_and(|cargo| {
                !matches!(cargo.location, CargoLocation::DepositedAtEndpoint { .. })
            })
        {
            return Err(TaskRuntimeError::InvalidTransition);
        }
        self.release_reservation(ledger);
        self.stage = TaskStage::Complete;
        self.assigned_cat_ids.clear();
        self.work_slot_by_cat.clear();
        self.updated_tick = now_tick;
        Ok(())
    }

    pub fn block_before_pickup(
        &mut self,
        reason: RuntimeBlockReason,
        ledger: &mut ReservationLedger,
        now_tick: u64,
    ) -> Result<(), TaskRuntimeError> {
        if self.has_picked_up_cargo() {
            return Err(TaskRuntimeError::CargoRequiresRecovery);
        }
        if self
            .cargo
            .as_ref()
            .is_some_and(|cargo| matches!(cargo.location, CargoLocation::ReservedAtSource { .. }))
        {
            self.cargo = None;
        }
        self.release_reservation(ledger);
        self.assigned_cat_ids.clear();
        self.work_slot_by_cat.clear();
        self.stage = TaskStage::Blocked;
        self.blocked_reason = Some(reason);
        self.updated_tick = now_tick;
        Ok(())
    }

    /// Preserve exact cargo after route/worker failure. A validated safe owned
    /// stockpile may receive it; otherwise it remains physically stranded.
    pub fn recover_after_pickup(
        &mut self,
        reason: RuntimeBlockReason,
        safe_owned_stockpile: Option<&SiteRef>,
        last_site_id: &str,
        ledger: &mut ReservationLedger,
        now_tick: u64,
    ) -> Result<(), TaskRuntimeError> {
        let cargo = self.cargo.as_mut().ok_or(TaskRuntimeError::CargoMissing)?;
        if !matches!(cargo.location, CargoLocation::Carried { .. }) {
            return Err(TaskRuntimeError::InvalidCargoLocation);
        }
        cargo.location = if let Some(stockpile) = safe_owned_stockpile {
            if !matches!(stockpile, SiteRef::Stockpile { .. }) {
                return Err(TaskRuntimeError::MalformedSpatialContract);
            }
            stockpile
                .validate()
                .map_err(|_| TaskRuntimeError::MalformedSpatialContract)?;
            CargoLocation::SalvagedAtStockpile {
                stockpile_id: stockpile.stable_id().to_owned(),
            }
        } else if !last_site_id.is_empty() {
            CargoLocation::Stranded {
                site_id: last_site_id.to_owned(),
            }
        } else {
            return Err(TaskRuntimeError::MalformedState);
        };
        self.release_reservation(ledger);
        self.assigned_cat_ids.clear();
        self.work_slot_by_cat.clear();
        self.stage = TaskStage::Blocked;
        self.blocked_reason = Some(reason);
        self.updated_tick = now_tick;
        Ok(())
    }

    pub fn cancel(
        &mut self,
        ledger: &mut ReservationLedger,
        now_tick: u64,
    ) -> Result<(), TaskRuntimeError> {
        if self.has_picked_up_cargo() {
            return Err(TaskRuntimeError::CargoRequiresRecovery);
        }
        if self
            .cargo
            .as_ref()
            .is_some_and(|cargo| matches!(cargo.location, CargoLocation::ReservedAtSource { .. }))
        {
            self.cargo = None;
        }
        self.release_reservation(ledger);
        self.assigned_cat_ids.clear();
        self.work_slot_by_cat.clear();
        self.stage = TaskStage::Cancelled;
        self.updated_tick = now_tick;
        Ok(())
    }

    /// Reconcile persisted runtime state against the separately persisted
    /// reservation ledger. Missing claims never leave a cat busy; picked-up or
    /// deposited cargo remains exact and recoverable.
    pub fn revalidate_after_restart(
        &mut self,
        ledger: &mut ReservationLedger,
        safe_owned_stockpile: Option<&SiteRef>,
        last_site_id: &str,
        now_tick: u64,
    ) -> Result<RestartRevalidationOutcome, TaskRuntimeError> {
        self.validate()?;
        if matches!(
            self.stage,
            TaskStage::Resolve
                | TaskStage::Reserve
                | TaskStage::Blocked
                | TaskStage::Complete
                | TaskStage::Cancelled
        ) {
            return Ok(RestartRevalidationOutcome::Unchanged);
        }
        if self
            .reservation_id
            .as_ref()
            .is_some_and(|reservation| ledger.contains(reservation))
        {
            return Ok(RestartRevalidationOutcome::ActiveReservationValid);
        }
        if self
            .cargo
            .as_ref()
            .is_some_and(|cargo| matches!(cargo.location, CargoLocation::Carried { .. }))
        {
            self.recover_after_pickup(
                RuntimeBlockReason::ReservationLost,
                safe_owned_stockpile,
                last_site_id,
                ledger,
                now_tick,
            )?;
            return Ok(RestartRevalidationOutcome::BlockedCargoPreserved);
        }
        if self.has_picked_up_cargo() {
            self.release_reservation(ledger);
            self.assigned_cat_ids.clear();
            self.work_slot_by_cat.clear();
            self.stage = TaskStage::Blocked;
            self.blocked_reason = Some(RuntimeBlockReason::ReservationLost);
            self.updated_tick = now_tick;
            return Ok(RestartRevalidationOutcome::BlockedCargoPreserved);
        }
        self.block_before_pickup(RuntimeBlockReason::ReservationLost, ledger, now_tick)?;
        Ok(RestartRevalidationOutcome::BlockedBeforePickup)
    }

    fn has_picked_up_cargo(&self) -> bool {
        self.cargo.as_ref().is_some_and(|cargo| {
            matches!(
                cargo.location,
                CargoLocation::Carried { .. }
                    | CargoLocation::DepositedAtEndpoint { .. }
                    | CargoLocation::SalvagedAtStockpile { .. }
                    | CargoLocation::Stranded { .. }
            )
        })
    }

    fn release_reservation(&mut self, ledger: &mut ReservationLedger) {
        if let Some(reservation) = self.reservation_id.take() {
            ledger.rollback(&reservation);
        }
    }

    fn validate(&self) -> Result<(), TaskRuntimeError> {
        if self.schema_version != TASK_RUNTIME_SCHEMA_VERSION
            || self.colony_id.is_empty()
            || self.id != TaskId::derive(&self.colony_id, &self.intent_id, self.occurrence)
            || self.progress_basis_points > TASK_PROGRESS_MAX_BASIS_POINTS
            || self.route_ids.iter().any(String::is_empty)
            || self.assigned_cat_ids.iter().any(String::is_empty)
            || self.work_slot_by_cat.keys().collect::<BTreeSet<_>>()
                != self.assigned_cat_ids.iter().collect::<BTreeSet<_>>()
        {
            return Err(TaskRuntimeError::MalformedState);
        }
        self.spatial
            .validate()
            .map_err(|_| TaskRuntimeError::MalformedSpatialContract)?;
        if let Some(spatial_reason) = self.spatial.blocked_reason {
            if self.stage != TaskStage::Blocked
                || self.spatial.objective.is_some()
                || self.blocked_reason != Some(RuntimeBlockReason::Spatial(spatial_reason))
            {
                return Err(TaskRuntimeError::MalformedState);
            }
        } else if self.spatial.objective.is_none()
            || self.spatial.work_positions.is_empty()
            || self.spatial.delivery_endpoint.is_none()
        {
            return Err(TaskRuntimeError::IncompleteSpatialContract);
        }
        if let Some(cargo) = &self.cargo {
            cargo.validate()?;
        }
        if (self.stage == TaskStage::Blocked) != self.blocked_reason.is_some() {
            return Err(TaskRuntimeError::MalformedState);
        }
        let active = matches!(
            self.stage,
            TaskStage::TravelToSource
                | TaskStage::Pickup
                | TaskStage::TravelToWork
                | TaskStage::Work
                | TaskStage::TravelToEndpoint
                | TaskStage::Deposit
        );
        if active {
            if self.reservation_id.is_none()
                || self.assigned_cat_ids.len() != 1
                || self.work_slot_by_cat.len() != 1
            {
                return Err(TaskRuntimeError::MalformedState);
            }
        } else if self.reservation_id.is_some()
            || !self.assigned_cat_ids.is_empty()
            || !self.work_slot_by_cat.is_empty()
        {
            return Err(TaskRuntimeError::MalformedState);
        }
        if let Some(cargo) = &self.cargo {
            let coherent = match cargo.location {
                CargoLocation::ReservedAtSource { .. } => matches!(
                    self.stage,
                    TaskStage::Resolve
                        | TaskStage::Reserve
                        | TaskStage::TravelToSource
                        | TaskStage::Pickup
                ),
                CargoLocation::Carried { .. } => matches!(
                    self.stage,
                    TaskStage::Pickup
                        | TaskStage::TravelToWork
                        | TaskStage::Work
                        | TaskStage::TravelToEndpoint
                        | TaskStage::Deposit
                ),
                CargoLocation::DepositedAtEndpoint { .. } => {
                    matches!(self.stage, TaskStage::Deposit | TaskStage::Complete)
                }
                CargoLocation::SalvagedAtStockpile { .. } | CargoLocation::Stranded { .. } => {
                    self.stage == TaskStage::Blocked
                }
            };
            if !coherent {
                return Err(TaskRuntimeError::MalformedState);
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedVisibleTaskRuntime {
    schema_version: u32,
    id: TaskId,
    occurrence: u32,
    colony_id: String,
    intent_id: IntentId,
    category: TaskCategory,
    stage: TaskStage,
    spatial: SpatialObjective,
    #[serde(default)]
    assigned_cat_ids: BTreeSet<String>,
    #[serde(default)]
    work_slot_by_cat: BTreeMap<String, String>,
    #[serde(default)]
    route_ids: Vec<String>,
    reservation_id: Option<ReservationId>,
    #[serde(default)]
    progress_basis_points: u16,
    cargo: Option<TaskCargo>,
    blocked_reason: Option<RuntimeBlockReason>,
    updated_tick: u64,
}

impl<'de> Deserialize<'de> for VisibleTaskRuntime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = UncheckedVisibleTaskRuntime::deserialize(deserializer)?;
        let task = Self {
            schema_version: raw.schema_version,
            id: raw.id,
            occurrence: raw.occurrence,
            colony_id: raw.colony_id,
            intent_id: raw.intent_id,
            category: raw.category,
            stage: raw.stage,
            spatial: raw.spatial,
            assigned_cat_ids: raw.assigned_cat_ids,
            work_slot_by_cat: raw.work_slot_by_cat,
            route_ids: raw.route_ids,
            reservation_id: raw.reservation_id,
            progress_basis_points: raw.progress_basis_points,
            cargo: raw.cargo,
            blocked_reason: raw.blocked_reason,
            updated_tick: raw.updated_tick,
        };
        task.validate().map_err(serde::de::Error::custom)?;
        Ok(task)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskRuntimeError {
    InvalidTransition,
    IncompleteSpatialContract,
    MalformedSpatialContract,
    ReservationNotCommitted,
    InvalidAssignment,
    CargoAlreadyPresent,
    CargoMissing,
    InvalidCargoLocation,
    CargoRequiresRecovery,
    MalformedState,
}

impl std::fmt::Display for TaskRuntimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "task runtime error: {self:?}")
    }
}

impl std::error::Error for TaskRuntimeError {}
