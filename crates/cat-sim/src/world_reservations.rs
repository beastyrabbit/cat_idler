//! World-scoped spatial reservation ledger specified by
//! `docs/leader-ai-overhaul/spatial-task-contract.md`.

use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    planner_core::{IntentId, PlannerId},
    reservation_transaction::{
        ClaimMode, MAX_COMMITTED_RESERVATIONS, MAX_OPTIONAL_CLAIMS_PER_KIND,
    },
    spatial_resolver::{ResolvedSpatialTask, SpatialTaskCategory},
    spatial_tasks::{SpatialBlockReason, WorkSlotReservation},
};

pub const WORLD_RESERVATION_SCHEMA_VERSION: u32 = 2;
pub const WORLD_RESERVATION_TRANSACTION_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorldReservationId(PlannerId);

impl WorldReservationId {
    #[must_use]
    pub fn derive(colony_id: &PlannerId, task_id: &PlannerId, intent_id: &IntentId) -> Self {
        Self(PlannerId::derive(
            "world_spatial_reservation",
            [colony_id.as_str(), task_id.as_str(), intent_id.as_str()],
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldClaimKind {
    Objective,
    ObjectiveTile,
    WorkSlot,
    DeliveryEndpoint,
    Route,
    Tool,
    CargoResource,
    Worker,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldClaimKey {
    pub kind: WorldClaimKind,
    pub stable_id: PlannerId,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldClaim {
    pub key: WorldClaimKey,
    pub mode: ClaimMode,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapacityReservation {
    pub stable_id: PlannerId,
    pub units: u32,
    pub capacity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorldReservationTransaction {
    pub schema_version: u32,
    pub id: WorldReservationId,
    pub colony_id: PlannerId,
    pub task_id: PlannerId,
    pub intent_id: IntentId,
    pub resolved: ResolvedSpatialTask,
    pub worker_id: PlannerId,
    pub tool_ids: Vec<PlannerId>,
    pub cargo_resources: Vec<CapacityReservation>,
    claims: Vec<WorldClaim>,
}

impl WorldReservationTransaction {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        colony_id: PlannerId,
        task_id: PlannerId,
        intent_id: IntentId,
        resolved: ResolvedSpatialTask,
        worker_id: PlannerId,
        mut tool_ids: Vec<PlannerId>,
        mut cargo_resources: Vec<CapacityReservation>,
    ) -> Result<Self, WorldReservationError> {
        tool_ids.sort();
        cargo_resources.sort();
        let id = WorldReservationId::derive(&colony_id, &task_id, &intent_id);
        let claims = build_claims(
            &colony_id,
            &resolved,
            &worker_id,
            &tool_ids,
            &cargo_resources,
        )?;
        let transaction = Self {
            schema_version: WORLD_RESERVATION_TRANSACTION_SCHEMA_VERSION,
            id,
            colony_id,
            task_id,
            intent_id,
            resolved,
            worker_id,
            tool_ids,
            cargo_resources,
            claims,
        };
        transaction.validate()?;
        Ok(transaction)
    }

    #[must_use]
    pub fn claims(&self) -> &[WorldClaim] {
        &self.claims
    }

    fn objective_site_id(&self) -> &str {
        self.resolved.objective().stable_id()
    }

    fn validate(&self) -> Result<(), WorldReservationError> {
        self.resolved
            .validate()
            .map_err(WorldReservationError::Blocked)?;
        if self.schema_version != WORLD_RESERVATION_TRANSACTION_SCHEMA_VERSION
            || self.id
                != WorldReservationId::derive(&self.colony_id, &self.task_id, &self.intent_id)
            || self.tool_ids.len() > MAX_OPTIONAL_CLAIMS_PER_KIND
            || self.cargo_resources.len() > MAX_OPTIONAL_CLAIMS_PER_KIND
            || !strictly_sorted(&self.tool_ids)
            || !strictly_sorted(&self.cargo_resources)
            || !strictly_sorted(&self.claims)
        {
            return Err(WorldReservationError::MalformedTransaction);
        }
        let expected = build_claims(
            &self.colony_id,
            &self.resolved,
            &self.worker_id,
            &self.tool_ids,
            &self.cargo_resources,
        )?;
        if self.claims != expected {
            return Err(WorldReservationError::MalformedTransaction);
        }
        Ok(())
    }
}

fn strictly_sorted<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn build_claims(
    colony_id: &PlannerId,
    resolved: &ResolvedSpatialTask,
    worker_id: &PlannerId,
    tool_ids: &[PlannerId],
    cargo_resources: &[CapacityReservation],
) -> Result<Vec<WorldClaim>, WorldReservationError> {
    resolved
        .validate()
        .map_err(WorldReservationError::Blocked)?;
    if tool_ids.len() > MAX_OPTIONAL_CLAIMS_PER_KIND
        || cargo_resources.len() > MAX_OPTIONAL_CLAIMS_PER_KIND
        || !strictly_sorted(tool_ids)
        || !strictly_sorted(cargo_resources)
    {
        return Err(WorldReservationError::MalformedTransaction);
    }
    let mut claims = Vec::new();
    claims.push(WorldClaim {
        key: site_key(
            WorldClaimKind::Objective,
            &format!(
                "{}:{}",
                colony_id.as_str(),
                resolved.objective().stable_id()
            ),
        ),
        mode: objective_claim_mode(resolved),
    });

    if let Some(tiles) = objective_tiles(resolved)? {
        for tile in tiles {
            claims.push(WorldClaim {
                key: WorldClaimKey {
                    kind: WorldClaimKind::ObjectiveTile,
                    stable_id: PlannerId::derive(
                        "world_tile",
                        [tile.x.to_string(), tile.y.to_string()],
                    ),
                },
                mode: objective_claim_mode(resolved),
            });
        }
    }

    let work_mode = match resolved.work_slot().reservation {
        WorkSlotReservation::Exclusive => ClaimMode::Exclusive,
        WorkSlotReservation::Capacity(capacity) => ClaimMode::Capacity {
            units: 1,
            capacity: capacity.get(),
        },
    };
    claims.push(WorldClaim {
        key: site_key(
            WorldClaimKind::WorkSlot,
            &format!(
                "{}:{}",
                colony_id.as_str(),
                resolved.work_slot().stable_id
            ),
        ),
        mode: work_mode,
    });
    claims.push(WorldClaim {
        key: site_key(
            WorldClaimKind::DeliveryEndpoint,
            &format!(
                "{}:{}",
                colony_id.as_str(),
                resolved.delivery_endpoint().stable_id()
            ),
        ),
        mode: ClaimMode::Capacity {
            units: resolved.delivery_units,
            capacity: resolved.delivery_capacity,
        },
    });
    claims.push(WorldClaim {
        key: site_key(
            WorldClaimKind::Route,
            resolved.source_to_work_route.stable_id(),
        ),
        mode: ClaimMode::Capacity {
            units: 1,
            capacity: resolved.source_to_work_route_capacity,
        },
    });
    claims.push(WorldClaim {
        key: site_key(
            WorldClaimKind::Route,
            resolved.work_to_delivery_route.stable_id(),
        ),
        mode: ClaimMode::Capacity {
            units: 1,
            capacity: resolved.work_to_delivery_route_capacity,
        },
    });
    claims.extend(tool_ids.iter().map(|stable_id| WorldClaim {
        key: WorldClaimKey {
            kind: WorldClaimKind::Tool,
            stable_id: PlannerId::derive(
                "world_colony_tool",
                [colony_id.as_str(), stable_id.as_str()],
            ),
        },
        mode: ClaimMode::Exclusive,
    }));
    claims.extend(cargo_resources.iter().map(|resource| WorldClaim {
        key: WorldClaimKey {
            kind: WorldClaimKind::CargoResource,
            stable_id: PlannerId::derive(
                "world_colony_cargo",
                [colony_id.as_str(), resource.stable_id.as_str()],
            ),
        },
        mode: ClaimMode::Capacity {
            units: resource.units,
            capacity: resource.capacity,
        },
    }));
    claims.push(WorldClaim {
        key: WorldClaimKey {
            kind: WorldClaimKind::Worker,
            stable_id: PlannerId::derive(
                "world_colony_worker",
                [colony_id.as_str(), worker_id.as_str()],
            ),
        },
        mode: ClaimMode::Exclusive,
    });
    claims.sort();
    if !strictly_sorted(&claims) || claims.iter().any(|claim| !valid_mode(claim.mode)) {
        return Err(WorldReservationError::MalformedTransaction);
    }
    Ok(claims)
}

fn objective_claim_mode(resolved: &ResolvedSpatialTask) -> ClaimMode {
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

fn objective_tiles(
    resolved: &ResolvedSpatialTask,
) -> Result<Option<Vec<crate::spatial_tasks::TilePoint>>, WorldReservationError> {
    match resolved.objective() {
        crate::spatial_tasks::SiteRef::OrderedRoute { route, .. } => Ok(Some(route.clone())),
        crate::spatial_tasks::SiteRef::OrderedTiles { tiles, .. } => {
            Ok(Some(tiles.as_slice().to_vec()))
        }
        crate::spatial_tasks::SiteRef::Tile { tile, .. } => Ok(Some(vec![*tile])),
        objective => objective
            .footprint()
            .map(|footprint| Some(footprint.tiles.as_slice().to_vec()))
            .ok_or(WorldReservationError::MalformedTransaction),
    }
}

fn site_key(kind: WorldClaimKind, stable_id: &str) -> WorldClaimKey {
    WorldClaimKey {
        kind,
        stable_id: PlannerId::derive("world_spatial_claim", [stable_id]),
    }
}

fn valid_mode(mode: ClaimMode) -> bool {
    match mode {
        ClaimMode::Exclusive => true,
        ClaimMode::Capacity { units, capacity } => units > 0 && capacity > 0 && units <= capacity,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldReservationValidation {
    pub objective_known_revealed: bool,
    pub objective_exists: bool,
    pub objective_occupancy_valid: bool,
    pub work_slot_available: bool,
    pub source_to_work_route_valid: bool,
    pub work_to_delivery_route_valid: bool,
    pub quantities_available: bool,
    pub cargo_capacity_available: bool,
    pub tools_available: bool,
    pub source_capacity_available: bool,
    pub endpoint_capacity_available: bool,
    pub worker_available: bool,
}

impl WorldReservationValidation {
    #[must_use]
    pub const fn all_valid() -> Self {
        Self {
            objective_known_revealed: true,
            objective_exists: true,
            objective_occupancy_valid: true,
            work_slot_available: true,
            source_to_work_route_valid: true,
            work_to_delivery_route_valid: true,
            quantities_available: true,
            cargo_capacity_available: true,
            tools_available: true,
            source_capacity_available: true,
            endpoint_capacity_available: true,
            worker_available: true,
        }
    }

    fn first_failure(self) -> Option<SpatialBlockReason> {
        if !self.objective_known_revealed {
            Some(SpatialBlockReason::UnrevealedObjective)
        } else if !self.objective_exists {
            Some(SpatialBlockReason::SourceUnavailable)
        } else if !self.objective_occupancy_valid {
            Some(SpatialBlockReason::ReservationConflict)
        } else if !self.work_slot_available {
            Some(SpatialBlockReason::WorkPositionUnavailable)
        } else if !self.source_to_work_route_valid || !self.work_to_delivery_route_valid {
            Some(SpatialBlockReason::RouteUnavailable)
        } else if !self.quantities_available || !self.source_capacity_available {
            Some(SpatialBlockReason::CapacityUnavailable)
        } else if !self.cargo_capacity_available || !self.endpoint_capacity_available {
            Some(SpatialBlockReason::DeliveryEndpointUnavailable)
        } else if !self.tools_available || !self.worker_available {
            Some(SpatialBlockReason::ReservationConflict)
        } else {
            None
        }
    }
}

impl Default for WorldReservationValidation {
    fn default() -> Self {
        Self::all_valid()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldCommitOutcome {
    Committed,
    AlreadyCommitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldReleaseOutcome {
    Released,
    NotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldRevalidationOutcome {
    Valid,
    Released(SpatialBlockReason),
    NotFound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBatchCommitResult {
    pub id: WorldReservationId,
    pub result: Result<WorldCommitOutcome, WorldReservationError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldReservationError {
    Blocked(SpatialBlockReason),
    Conflict(WorldClaimKey),
    MalformedTransaction,
    MalformedPersistence,
    ReservationIdConflict,
    CapacityReached,
    VersionExhausted,
}

impl fmt::Display for WorldReservationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "world reservation error: {self:?}")
    }
}

impl std::error::Error for WorldReservationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReservationLedger {
    version: u64,
    reservations: BTreeMap<WorldReservationId, WorldReservationTransaction>,
}

impl WorldReservationLedger {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            version: 0,
            reservations: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn version(&self) -> u64 {
        self.version
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.reservations.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.reservations.is_empty()
    }

    #[must_use]
    pub fn contains(&self, id: &WorldReservationId) -> bool {
        self.reservations.contains_key(id)
    }

    #[must_use]
    pub fn worker_is_reserved(&self, worker_id: &PlannerId) -> bool {
        self.reservations
            .values()
            .any(|transaction| &transaction.worker_id == worker_id)
    }

    #[must_use]
    pub fn claim_count(&self) -> usize {
        self.reservations
            .values()
            .map(|transaction| transaction.claims.len())
            .sum()
    }

    /// Deterministically reconcile the per-colony persisted mirrors into the
    /// one world claim book used by a tick. Duplicate byte-identical
    /// transactions collapse; conflicting transactions are resolved by the
    /// same stable site/task/colony/reservation order as a live batch commit.
    ///
    /// The returned loser IDs are used by `world_tick` to release the matching
    /// local reservation and recover any already-carried cargo before the
    /// colony may schedule again.
    #[must_use]
    pub fn reconcile_persisted_mirrors<'a>(
        ledgers: impl IntoIterator<Item = &'a Self>,
    ) -> (Self, Vec<WorldReservationId>) {
        let mut unique = BTreeMap::new();
        for ledger in ledgers {
            for transaction in ledger.reservations.values() {
                match unique.entry(transaction.id.clone()) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(transaction.clone());
                    }
                    std::collections::btree_map::Entry::Occupied(mut entry) => {
                        let incoming = serde_json::to_vec(transaction).unwrap_or_default();
                        let retained = serde_json::to_vec(entry.get()).unwrap_or_default();
                        if incoming < retained {
                            entry.insert(transaction.clone());
                        }
                    }
                }
            }
        }
        let mut reconciled = Self::new();
        let results = reconciled.commit_batch(
            unique
                .values()
                .cloned()
                .map(|transaction| {
                    (transaction, WorldReservationValidation::all_valid())
                })
                .collect(),
        );
        let losers = results
            .into_iter()
            .filter_map(|result| result.result.is_err().then_some(result.id))
            .collect();
        (reconciled, losers)
    }

    pub fn try_commit(
        &mut self,
        transaction: WorldReservationTransaction,
        validation: WorldReservationValidation,
    ) -> Result<WorldCommitOutcome, WorldReservationError> {
        if let Some(reason) = validation.first_failure() {
            return Err(WorldReservationError::Blocked(reason));
        }
        transaction.validate()?;
        if let Some(existing) = self.reservations.get(&transaction.id) {
            return if existing == &transaction {
                Ok(WorldCommitOutcome::AlreadyCommitted)
            } else {
                Err(WorldReservationError::ReservationIdConflict)
            };
        }
        if self.reservations.len() >= MAX_COMMITTED_RESERVATIONS {
            return Err(WorldReservationError::CapacityReached);
        }
        let mut candidate = self.clone();
        candidate
            .reservations
            .insert(transaction.id.clone(), transaction);
        candidate.validate_conflicts()?;
        candidate.version = candidate
            .version
            .checked_add(1)
            .ok_or(WorldReservationError::VersionExhausted)?;
        *self = candidate;
        Ok(WorldCommitOutcome::Committed)
    }

    /// Commit a wave in exact site ID, task ID, colony ID, reservation ID
    /// order, independent of collection iteration.
    pub fn commit_batch(
        &mut self,
        mut transactions: Vec<(WorldReservationTransaction, WorldReservationValidation)>,
    ) -> Vec<WorldBatchCommitResult> {
        transactions.sort_by(|(first, _), (second, _)| {
            first
                .objective_site_id()
                .cmp(second.objective_site_id())
                .then_with(|| first.task_id.cmp(&second.task_id))
                .then_with(|| first.colony_id.cmp(&second.colony_id))
                .then_with(|| first.id.cmp(&second.id))
        });
        transactions
            .into_iter()
            .map(|(transaction, validation)| {
                let id = transaction.id.clone();
                let result = self.try_commit(transaction, validation);
                WorldBatchCommitResult { id, result }
            })
            .collect()
    }

    pub fn release(
        &mut self,
        id: &WorldReservationId,
    ) -> Result<WorldReleaseOutcome, WorldReservationError> {
        if !self.reservations.contains_key(id) {
            return Ok(WorldReleaseOutcome::NotFound);
        }
        let next_version = self
            .version
            .checked_add(1)
            .ok_or(WorldReservationError::VersionExhausted)?;
        self.reservations.remove(id);
        self.version = next_version;
        Ok(WorldReleaseOutcome::Released)
    }

    pub fn revalidate(
        &mut self,
        id: &WorldReservationId,
        validation: WorldReservationValidation,
    ) -> Result<WorldRevalidationOutcome, WorldReservationError> {
        if !self.reservations.contains_key(id) {
            return Ok(WorldRevalidationOutcome::NotFound);
        }
        let Some(reason) = validation.first_failure() else {
            return Ok(WorldRevalidationOutcome::Valid);
        };
        self.release(id)?;
        Ok(WorldRevalidationOutcome::Released(reason))
    }

    fn validate_conflicts(&self) -> Result<(), WorldReservationError> {
        let mut occupied = BTreeMap::<WorldClaimKey, (bool, u64, u32)>::new();
        for transaction in self.reservations.values() {
            for claim in &transaction.claims {
                let entry = occupied.entry(claim.key.clone()).or_insert((false, 0, 0));
                match claim.mode {
                    ClaimMode::Exclusive => {
                        if entry.0 || entry.1 > 0 {
                            return Err(WorldReservationError::Conflict(claim.key.clone()));
                        }
                        entry.0 = true;
                    }
                    ClaimMode::Capacity { units, capacity } => {
                        if entry.0 || (entry.2 != 0 && entry.2 != capacity) {
                            return Err(WorldReservationError::Conflict(claim.key.clone()));
                        }
                        entry.2 = capacity;
                        entry.1 = entry
                            .1
                            .checked_add(u64::from(units))
                            .ok_or_else(|| WorldReservationError::Conflict(claim.key.clone()))?;
                        if entry.1 > u64::from(capacity) {
                            return Err(WorldReservationError::Conflict(claim.key.clone()));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), WorldReservationError> {
        if self.reservations.len() > MAX_COMMITTED_RESERVATIONS
            || (!self.reservations.is_empty() && self.version == 0)
        {
            return Err(WorldReservationError::MalformedPersistence);
        }
        for (id, transaction) in &self.reservations {
            if id != &transaction.id {
                return Err(WorldReservationError::MalformedPersistence);
            }
            transaction.validate()?;
        }
        self.validate_conflicts()
    }
}

impl Default for WorldReservationLedger {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PersistedWorldReservationLedger<'a> {
    schema_version: u32,
    version: u64,
    reservations: Vec<&'a WorldReservationTransaction>,
}

impl Serialize for WorldReservationLedger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        PersistedWorldReservationLedger {
            schema_version: WORLD_RESERVATION_SCHEMA_VERSION,
            version: self.version,
            reservations: self.reservations.values().collect(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for WorldReservationLedger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct PersistedWorldReservationLedger {
            schema_version: u32,
            #[serde(default)]
            version: u64,
            #[serde(default)]
            reservations: Vec<WorldReservationTransaction>,
        }

        let persisted = PersistedWorldReservationLedger::deserialize(deserializer)?;
        if persisted.schema_version != WORLD_RESERVATION_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(
                "unsupported world reservation schema version",
            ));
        }
        let mut reservations = BTreeMap::new();
        for transaction in persisted.reservations {
            if reservations
                .insert(transaction.id.clone(), transaction)
                .is_some()
            {
                return Err(serde::de::Error::custom("duplicate world reservation ID"));
            }
        }
        let ledger = Self {
            version: persisted.version,
            reservations,
        };
        ledger.validate().map_err(serde::de::Error::custom)?;
        Ok(ledger)
    }
}
