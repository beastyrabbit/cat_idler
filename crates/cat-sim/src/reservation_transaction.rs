//! Atomic scheduler reservation transactions specified by
//! `docs/leader-ai-overhaul/spatial-task-contract.md`.

use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};

use crate::{
    planner_core::{IntentId, PlannerId},
    spatial_tasks::{SpatialObjective, WorkSlotReservation},
};

pub const RESERVATION_LEDGER_SCHEMA_VERSION: u32 = 1;
pub const RESERVATION_BUNDLE_SCHEMA_VERSION: u32 = 1;
pub const MAX_COMMITTED_RESERVATIONS: usize = 4_096;
pub const MAX_OPTIONAL_CLAIMS_PER_KIND: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ReservationId(PlannerId);

impl ReservationId {
    #[must_use]
    pub fn derive(colony_id: &PlannerId, task_id: &PlannerId, intent_id: &IntentId) -> Self {
        Self(PlannerId::derive(
            "reservation_transaction",
            [colony_id.as_str(), task_id.as_str(), intent_id.as_str()],
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReservationCategory {
    Objective,
    WorkSlot,
    DeliveryCapacity,
    Route,
    Tool,
    CargoResource,
    Cat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClaimMode {
    Exclusive,
    Capacity { units: u32, capacity: u32 },
}

impl ClaimMode {
    fn validate(self) -> Result<(), ReservationError> {
        if let Self::Capacity { units, capacity } = self
            && (units == 0 || capacity == 0 || units > capacity)
        {
            return Err(ReservationError::MalformedBundle);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimSpec {
    pub stable_id: PlannerId,
    pub mode: ClaimMode,
}

impl ClaimSpec {
    #[must_use]
    pub const fn exclusive(stable_id: PlannerId) -> Self {
        Self {
            stable_id,
            mode: ClaimMode::Exclusive,
        }
    }

    #[must_use]
    pub const fn capacity(stable_id: PlannerId, units: u32, capacity: u32) -> Self {
        Self {
            stable_id,
            mode: ClaimMode::Capacity { units, capacity },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReservationKey {
    pub category: ReservationCategory,
    pub stable_id: PlannerId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReservationBundle {
    pub schema_version: u32,
    pub id: ReservationId,
    pub colony_id: PlannerId,
    pub task_id: PlannerId,
    pub intent_id: IntentId,
    pub objective: ClaimSpec,
    pub work_slot: ClaimSpec,
    pub delivery_capacity: ClaimSpec,
    pub route: ClaimSpec,
    pub tools: Vec<ClaimSpec>,
    pub cargo_resources: Vec<ClaimSpec>,
    pub cat: ClaimSpec,
}

impl ReservationBundle {
    #[allow(clippy::too_many_arguments)]
    pub fn from_spatial_objective(
        colony_id: PlannerId,
        task_id: PlannerId,
        intent_id: IntentId,
        spatial: &SpatialObjective,
        work_slot_index: usize,
        objective_mode: ClaimMode,
        delivery_mode: ClaimMode,
        route: ClaimSpec,
        tools: Vec<ClaimSpec>,
        cargo_resources: Vec<ClaimSpec>,
        cat_id: PlannerId,
    ) -> Result<Self, ReservationError> {
        spatial
            .validate()
            .map_err(|_| ReservationError::SpatialInvalid)?;
        if spatial.blocked_reason.is_some() {
            return Err(ReservationError::IncompleteSpatialObjective);
        }
        let objective = spatial
            .objective
            .as_ref()
            .ok_or(ReservationError::IncompleteSpatialObjective)?;
        let work_slot = spatial
            .work_positions
            .get(work_slot_index)
            .ok_or(ReservationError::MissingWorkSlot)?;
        let delivery = spatial
            .delivery_endpoint
            .as_ref()
            .ok_or(ReservationError::IncompleteSpatialObjective)?;
        let work_mode = match work_slot.reservation {
            WorkSlotReservation::Exclusive => ClaimMode::Exclusive,
            WorkSlotReservation::Capacity(capacity) => ClaimMode::Capacity {
                units: 1,
                capacity: capacity.get(),
            },
        };
        let reservation_id = ReservationId::derive(&colony_id, &task_id, &intent_id);
        let mut bundle = Self {
            schema_version: RESERVATION_BUNDLE_SCHEMA_VERSION,
            id: reservation_id,
            colony_id,
            task_id,
            intent_id,
            objective: ClaimSpec {
                stable_id: PlannerId::derive("spatial_objective", [objective.stable_id()]),
                mode: objective_mode,
            },
            work_slot: ClaimSpec {
                stable_id: PlannerId::derive("spatial_work_slot", [work_slot.stable_id.as_str()]),
                mode: work_mode,
            },
            delivery_capacity: ClaimSpec {
                stable_id: PlannerId::derive("spatial_delivery", [delivery.stable_id()]),
                mode: delivery_mode,
            },
            route,
            tools,
            cargo_resources,
            cat: ClaimSpec::exclusive(cat_id),
        };
        bundle.canonicalize();
        bundle.validate()?;
        Ok(bundle)
    }

    fn canonicalize(&mut self) {
        self.tools.sort();
        self.cargo_resources.sort();
    }

    fn claims(&self) -> Vec<(ReservationKey, ClaimMode)> {
        let mut claims = Vec::with_capacity(5 + self.tools.len() + self.cargo_resources.len());
        claims.push(claim(ReservationCategory::Objective, &self.objective));
        claims.push(claim(ReservationCategory::WorkSlot, &self.work_slot));
        claims.push(claim(
            ReservationCategory::DeliveryCapacity,
            &self.delivery_capacity,
        ));
        claims.push(claim(ReservationCategory::Route, &self.route));
        claims.extend(
            self.tools
                .iter()
                .map(|spec| claim(ReservationCategory::Tool, spec)),
        );
        claims.extend(
            self.cargo_resources
                .iter()
                .map(|spec| claim(ReservationCategory::CargoResource, spec)),
        );
        claims.push(claim(ReservationCategory::Cat, &self.cat));
        claims
    }

    fn validate(&self) -> Result<(), ReservationError> {
        if self.schema_version != RESERVATION_BUNDLE_SCHEMA_VERSION
            || self.id != ReservationId::derive(&self.colony_id, &self.task_id, &self.intent_id)
            || self.cat.mode != ClaimMode::Exclusive
            || self.tools.len() > MAX_OPTIONAL_CLAIMS_PER_KIND
            || self.cargo_resources.len() > MAX_OPTIONAL_CLAIMS_PER_KIND
            || !self.tools.windows(2).all(|pair| pair[0] < pair[1])
            || !self
                .tools
                .windows(2)
                .all(|pair| pair[0].stable_id != pair[1].stable_id)
            || !self
                .cargo_resources
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            || !self
                .cargo_resources
                .windows(2)
                .all(|pair| pair[0].stable_id != pair[1].stable_id)
        {
            return Err(ReservationError::MalformedBundle);
        }
        for (_, mode) in self.claims() {
            mode.validate()?;
        }
        Ok(())
    }
}

fn claim(category: ReservationCategory, spec: &ClaimSpec) -> (ReservationKey, ClaimMode) {
    (
        ReservationKey {
            category,
            stable_id: spec.stable_id.clone(),
        },
        spec.mode,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReservationChecks {
    pub objective_valid: bool,
    pub work_slot_valid: bool,
    pub delivery_capacity_valid: bool,
    pub route_valid: bool,
    pub tools_available: bool,
    pub cargo_resources_available: bool,
    pub cat_eligible: bool,
    pub cat_willing: bool,
}

impl ReservationChecks {
    #[must_use]
    pub const fn all_valid() -> Self {
        Self {
            objective_valid: true,
            work_slot_valid: true,
            delivery_capacity_valid: true,
            route_valid: true,
            tools_available: true,
            cargo_resources_available: true,
            cat_eligible: true,
            cat_willing: true,
        }
    }

    fn first_failure(self) -> Option<ReservationFailure> {
        if !self.objective_valid {
            Some(ReservationFailure::ObjectiveInvalid)
        } else if !self.work_slot_valid {
            Some(ReservationFailure::WorkSlotInvalid)
        } else if !self.delivery_capacity_valid {
            Some(ReservationFailure::DeliveryInvalid)
        } else if !self.route_valid {
            Some(ReservationFailure::RouteInvalid)
        } else if !self.tools_available {
            Some(ReservationFailure::ToolUnavailable)
        } else if !self.cargo_resources_available {
            Some(ReservationFailure::CargoResourceUnavailable)
        } else if !self.cat_eligible {
            Some(ReservationFailure::CatIneligible)
        } else if !self.cat_willing {
            Some(ReservationFailure::CatRefused)
        } else {
            None
        }
    }
}

impl Default for ReservationChecks {
    fn default() -> Self {
        Self::all_valid()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReservationFailure {
    ObjectiveInvalid,
    WorkSlotInvalid,
    DeliveryInvalid,
    RouteInvalid,
    ToolUnavailable,
    CargoResourceUnavailable,
    CatIneligible,
    CatRefused,
    Conflict(ReservationKey),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitOutcome {
    Committed,
    AlreadyCommitted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReservationLedger {
    pub schema_version: u32,
    pub version: u64,
    committed: BTreeMap<ReservationId, ReservationBundle>,
}

impl ReservationLedger {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: RESERVATION_LEDGER_SCHEMA_VERSION,
            version: 0,
            committed: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.committed.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.committed.is_empty()
    }

    #[must_use]
    pub fn contains(&self, id: &ReservationId) -> bool {
        self.committed.contains_key(id)
    }

    #[must_use]
    pub fn cat_is_busy(&self, cat_id: &PlannerId) -> bool {
        self.committed
            .values()
            .any(|bundle| &bundle.cat.stable_id == cat_id)
    }

    #[must_use]
    pub fn claim_count(&self) -> usize {
        self.committed
            .values()
            .map(|bundle| bundle.claims().len())
            .sum()
    }

    pub fn try_commit(
        &mut self,
        mut bundle: ReservationBundle,
        checks: ReservationChecks,
    ) -> Result<CommitOutcome, ReservationError> {
        if let Some(failure) = checks.first_failure() {
            return Err(ReservationError::Validation(failure));
        }
        bundle.canonicalize();
        bundle.validate()?;
        if let Some(existing) = self.committed.get(&bundle.id) {
            return if existing == &bundle {
                Ok(CommitOutcome::AlreadyCommitted)
            } else {
                Err(ReservationError::ReservationIdConflict)
            };
        }
        if self.committed.len() >= MAX_COMMITTED_RESERVATIONS {
            return Err(ReservationError::CapacityReached);
        }

        let mut candidate = self.clone();
        candidate.committed.insert(bundle.id.clone(), bundle);
        candidate.validate_conflicts()?;
        candidate.version = candidate.version.saturating_add(1);
        *self = candidate;
        Ok(CommitOutcome::Committed)
    }

    /// Resolve a scheduling wave in stable task/colony order. Conflicting
    /// transactions necessarily share the same site key, so this is the
    /// task/colony remainder of the specified site/task/colony ordering.
    pub fn commit_batch<I>(
        &mut self,
        transactions: I,
    ) -> BTreeMap<ReservationId, Result<CommitOutcome, ReservationError>>
    where
        I: IntoIterator<Item = (ReservationBundle, ReservationChecks)>,
    {
        let mut transactions = transactions.into_iter().collect::<Vec<_>>();
        transactions.sort_by(|(left, _), (right, _)| {
            left.task_id
                .cmp(&right.task_id)
                .then_with(|| left.colony_id.cmp(&right.colony_id))
                .then_with(|| left.id.cmp(&right.id))
        });
        transactions
            .into_iter()
            .map(|(bundle, checks)| {
                let id = bundle.id.clone();
                (id, self.try_commit(bundle, checks))
            })
            .collect()
    }

    pub fn rollback(&mut self, id: &ReservationId) -> bool {
        let removed = self.committed.remove(id).is_some();
        if removed {
            self.version = self.version.saturating_add(1);
        }
        removed
    }

    pub fn invalidate(&mut self, id: &ReservationId) -> bool {
        self.rollback(id)
    }

    fn validate_conflicts(&self) -> Result<(), ReservationError> {
        let mut occupied = BTreeMap::<ReservationKey, (bool, u64, u32)>::new();
        for bundle in self.committed.values() {
            for (key, mode) in bundle.claims() {
                let entry = occupied.entry(key.clone()).or_insert((false, 0, 0));
                match mode {
                    ClaimMode::Exclusive => {
                        if entry.0 || entry.1 > 0 {
                            return Err(ReservationError::Validation(
                                ReservationFailure::Conflict(key),
                            ));
                        }
                        entry.0 = true;
                    }
                    ClaimMode::Capacity { units, capacity } => {
                        if entry.0 || (entry.2 != 0 && entry.2 != capacity) {
                            return Err(ReservationError::Validation(
                                ReservationFailure::Conflict(key),
                            ));
                        }
                        entry.2 = capacity;
                        entry.1 = entry.1.saturating_add(u64::from(units));
                        if entry.1 > u64::from(capacity) {
                            return Err(ReservationError::Validation(
                                ReservationFailure::Conflict(key),
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), ReservationError> {
        if self.schema_version != RESERVATION_LEDGER_SCHEMA_VERSION
            || self.committed.len() > MAX_COMMITTED_RESERVATIONS
        {
            return Err(ReservationError::MalformedPersistence);
        }
        for (id, bundle) in &self.committed {
            if id != &bundle.id {
                return Err(ReservationError::MalformedPersistence);
            }
            bundle.validate()?;
        }
        self.validate_conflicts()
    }
}

impl Default for ReservationLedger {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UncheckedReservationLedger {
    schema_version: u32,
    #[serde(default)]
    version: u64,
    #[serde(default)]
    committed: BTreeMap<ReservationId, ReservationBundle>,
}

impl<'de> Deserialize<'de> for ReservationLedger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as _;

        let raw = UncheckedReservationLedger::deserialize(deserializer)?;
        let ledger = Self {
            schema_version: raw.schema_version,
            version: raw.version,
            committed: raw.committed,
        };
        ledger.validate().map_err(D::Error::custom)?;
        Ok(ledger)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReservationError {
    Validation(ReservationFailure),
    MalformedBundle,
    MalformedPersistence,
    ReservationIdConflict,
    CapacityReached,
    SpatialInvalid,
    IncompleteSpatialObjective,
    MissingWorkSlot,
}

impl fmt::Display for ReservationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "reservation error: {self:?}")
    }
}

impl std::error::Error for ReservationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial_tasks::{SiteMetadata, SiteRef, TilePoint, WorkSlot};

    fn id(namespace: &str, value: &str) -> PlannerId {
        PlannerId::derive(namespace, [value])
    }

    fn bundle(name: &str) -> ReservationBundle {
        let colony_id = id("colony", name);
        let task_id = id("task", name);
        let intent_id = IntentId::derive(name, 1, "build", name, 0);
        ReservationBundle {
            schema_version: RESERVATION_BUNDLE_SCHEMA_VERSION,
            id: ReservationId::derive(&colony_id, &task_id, &intent_id),
            colony_id,
            task_id,
            intent_id,
            objective: ClaimSpec::exclusive(id("objective", name)),
            work_slot: ClaimSpec::exclusive(id("work_slot", name)),
            delivery_capacity: ClaimSpec::capacity(id("delivery", name), 1, 4),
            route: ClaimSpec::capacity(id("route", name), 1, 8),
            tools: vec![
                ClaimSpec::exclusive(id("tool", &format!("{name}-b"))),
                ClaimSpec::exclusive(id("tool", &format!("{name}-a"))),
            ],
            cargo_resources: vec![
                ClaimSpec::capacity(id("resource", &format!("{name}-stone")), 2, 10),
                ClaimSpec::capacity(id("resource", &format!("{name}-wood")), 2, 10),
            ],
            cat: ClaimSpec::exclusive(id("cat", name)),
        }
    }

    fn tile_site(name: &str, x: i32) -> SiteRef {
        SiteRef::Tile {
            metadata: SiteMetadata::revealed(name),
            tile: TilePoint { x, y: 0 },
        }
    }

    #[test]
    fn typed_spatial_contract_derives_objective_slot_and_delivery_claims() {
        let spatial = SpatialObjective::resolved(
            tile_site("tree-1", 0),
            vec![WorkSlot::exclusive(
                "tree-1-slot",
                tile_site("tree-1-work", 1),
            )],
            Some(tile_site("stockpile-1", 2)),
        );
        let colony_id = id("colony", "spatial");
        let task_id = id("task", "spatial");
        let intent_id = IntentId::derive("spatial", 1, "logging", "tree-1", 0);
        let bundle = ReservationBundle::from_spatial_objective(
            colony_id,
            task_id,
            intent_id,
            &spatial,
            0,
            ClaimMode::Exclusive,
            ClaimMode::Capacity {
                units: 2,
                capacity: 10,
            },
            ClaimSpec::capacity(id("route", "tree-to-stockpile"), 1, 8),
            vec![ClaimSpec::exclusive(id("tool", "axe"))],
            vec![ClaimSpec::capacity(id("resource", "logs"), 2, 20)],
            id("cat", "logger"),
        )
        .unwrap();
        assert_eq!(
            bundle.objective.stable_id,
            PlannerId::derive("spatial_objective", ["tree-1"])
        );
        assert_eq!(
            bundle.work_slot.stable_id,
            PlannerId::derive("spatial_work_slot", ["tree-1-slot"])
        );
        assert_eq!(
            bundle.delivery_capacity.stable_id,
            PlannerId::derive("spatial_delivery", ["stockpile-1"])
        );
        let mut ledger = ReservationLedger::new();
        ledger
            .try_commit(bundle, ReservationChecks::all_valid())
            .unwrap();
        assert_eq!(ledger.claim_count(), 7);

        assert_eq!(
            ReservationBundle::from_spatial_objective(
                id("colony", "blocked"),
                id("task", "blocked"),
                IntentId::derive("blocked", 1, "logging", "tree", 0),
                &SpatialObjective::blocked(
                    crate::spatial_tasks::SpatialBlockReason::RouteUnavailable,
                ),
                0,
                ClaimMode::Exclusive,
                ClaimMode::Exclusive,
                ClaimSpec::exclusive(id("route", "missing")),
                Vec::new(),
                Vec::new(),
                id("cat", "none"),
            ),
            Err(ReservationError::IncompleteSpatialObjective)
        );
    }

    #[test]
    fn commit_is_all_claims_once_and_rollback_is_idempotent() {
        let mut ledger = ReservationLedger::new();
        let bundle = bundle("one");
        let reservation_id = bundle.id.clone();
        let cat_id = bundle.cat.stable_id.clone();
        assert_eq!(
            ledger
                .try_commit(bundle.clone(), ReservationChecks::all_valid())
                .unwrap(),
            CommitOutcome::Committed
        );
        assert_eq!(ledger.claim_count(), 9);
        assert!(ledger.cat_is_busy(&cat_id));
        let version = ledger.version;
        assert_eq!(
            ledger
                .try_commit(bundle, ReservationChecks::all_valid())
                .unwrap(),
            CommitOutcome::AlreadyCommitted
        );
        assert_eq!(ledger.version, version);
        assert!(ledger.rollback(&reservation_id));
        assert!(!ledger.cat_is_busy(&cat_id));
        let version = ledger.version;
        assert!(!ledger.rollback(&reservation_id));
        assert_eq!(ledger.version, version);
    }

    #[test]
    fn every_validation_failure_rolls_back_and_never_marks_cat_busy() {
        let cases = [
            (
                ReservationChecks {
                    objective_valid: false,
                    ..ReservationChecks::all_valid()
                },
                ReservationFailure::ObjectiveInvalid,
            ),
            (
                ReservationChecks {
                    work_slot_valid: false,
                    ..ReservationChecks::all_valid()
                },
                ReservationFailure::WorkSlotInvalid,
            ),
            (
                ReservationChecks {
                    delivery_capacity_valid: false,
                    ..ReservationChecks::all_valid()
                },
                ReservationFailure::DeliveryInvalid,
            ),
            (
                ReservationChecks {
                    route_valid: false,
                    ..ReservationChecks::all_valid()
                },
                ReservationFailure::RouteInvalid,
            ),
            (
                ReservationChecks {
                    tools_available: false,
                    ..ReservationChecks::all_valid()
                },
                ReservationFailure::ToolUnavailable,
            ),
            (
                ReservationChecks {
                    cargo_resources_available: false,
                    ..ReservationChecks::all_valid()
                },
                ReservationFailure::CargoResourceUnavailable,
            ),
            (
                ReservationChecks {
                    cat_eligible: false,
                    ..ReservationChecks::all_valid()
                },
                ReservationFailure::CatIneligible,
            ),
            (
                ReservationChecks {
                    cat_willing: false,
                    ..ReservationChecks::all_valid()
                },
                ReservationFailure::CatRefused,
            ),
        ];
        for (checks, expected) in cases {
            let mut ledger = ReservationLedger::new();
            let bundle = bundle("refusal");
            let cat_id = bundle.cat.stable_id.clone();
            assert_eq!(
                ledger.try_commit(bundle, checks),
                Err(ReservationError::Validation(expected))
            );
            assert!(ledger.is_empty());
            assert_eq!(ledger.version, 0);
            assert!(!ledger.cat_is_busy(&cat_id));
        }
    }

    #[test]
    fn exclusive_and_capacity_conflicts_leave_no_loser_claims() {
        let mut ledger = ReservationLedger::new();
        let winner = bundle("winner");
        let shared_objective = winner.objective.clone();
        ledger
            .try_commit(winner, ReservationChecks::all_valid())
            .unwrap();
        let mut loser = bundle("loser");
        loser.objective = shared_objective;
        let loser_id = loser.id.clone();
        let loser_cat = loser.cat.stable_id.clone();
        assert!(matches!(
            ledger.try_commit(loser, ReservationChecks::all_valid()),
            Err(ReservationError::Validation(ReservationFailure::Conflict(
                ReservationKey {
                    category: ReservationCategory::Objective,
                    ..
                }
            )))
        ));
        assert!(!ledger.contains(&loser_id));
        assert!(!ledger.cat_is_busy(&loser_cat));
        assert_eq!(ledger.len(), 1);

        let mut capacity_ledger = ReservationLedger::new();
        let mut first = bundle("first");
        first.delivery_capacity = ClaimSpec::capacity(id("delivery", "shared"), 2, 3);
        capacity_ledger
            .try_commit(first, ReservationChecks::all_valid())
            .unwrap();
        let mut second = bundle("second");
        second.delivery_capacity = ClaimSpec::capacity(id("delivery", "shared"), 2, 3);
        let second_cat = second.cat.stable_id.clone();
        assert!(
            capacity_ledger
                .try_commit(second, ReservationChecks::all_valid())
                .is_err()
        );
        assert_eq!(capacity_ledger.len(), 1);
        assert!(!capacity_ledger.cat_is_busy(&second_cat));

        let mut overflow_ledger = ReservationLedger::new();
        let mut maximum = bundle("maximum");
        maximum.delivery_capacity =
            ClaimSpec::capacity(id("delivery", "overflow"), u32::MAX, u32::MAX);
        overflow_ledger
            .try_commit(maximum, ReservationChecks::all_valid())
            .unwrap();
        let mut plus_one = bundle("plus-one");
        plus_one.delivery_capacity = ClaimSpec::capacity(id("delivery", "overflow"), 1, u32::MAX);
        assert!(
            overflow_ledger
                .try_commit(plus_one, ReservationChecks::all_valid())
                .is_err()
        );
        assert_eq!(overflow_ledger.len(), 1);
    }

    #[test]
    fn shuffled_optional_claims_commit_to_byte_equal_canonical_ledgers() {
        let mut forward = ReservationLedger::new();
        let mut reverse = ReservationLedger::new();
        let one = bundle("same");
        let mut two = one.clone();
        two.tools.reverse();
        two.cargo_resources.reverse();
        forward
            .try_commit(one, ReservationChecks::all_valid())
            .unwrap();
        reverse
            .try_commit(two, ReservationChecks::all_valid())
            .unwrap();
        assert_eq!(forward, reverse);
        assert_eq!(
            serde_json::to_string(&forward).unwrap(),
            serde_json::to_string(&reverse).unwrap()
        );
    }

    #[test]
    fn batch_conflict_winner_is_stable_across_input_order() {
        let first = bundle("a-task");
        let mut second = bundle("z-task");
        second.objective = first.objective.clone();
        let first_id = first.id.clone();
        let second_id = second.id.clone();

        let mut forward = ReservationLedger::new();
        let forward_results = forward.commit_batch([
            (first.clone(), ReservationChecks::all_valid()),
            (second.clone(), ReservationChecks::all_valid()),
        ]);
        let mut reverse = ReservationLedger::new();
        let reverse_results = reverse.commit_batch([
            (second, ReservationChecks::all_valid()),
            (first, ReservationChecks::all_valid()),
        ]);

        assert_eq!(forward, reverse);
        assert_eq!(forward_results, reverse_results);
        assert_eq!(
            forward_results.get(&first_id),
            Some(&Ok(CommitOutcome::Committed))
        );
        assert!(matches!(
            forward_results.get(&second_id),
            Some(Err(ReservationError::Validation(
                ReservationFailure::Conflict(_)
            )))
        ));
    }

    #[test]
    fn invalidation_releases_the_complete_transaction() {
        let mut ledger = ReservationLedger::new();
        let bundle = bundle("invalidated");
        let reservation_id = bundle.id.clone();
        ledger
            .try_commit(bundle, ReservationChecks::all_valid())
            .unwrap();
        assert!(ledger.invalidate(&reservation_id));
        assert!(ledger.is_empty());
        assert_eq!(ledger.claim_count(), 0);
    }

    #[test]
    fn persistence_defaults_empty_and_rejects_versions_and_noncanonical_claims() {
        let minimal = serde_json::json!({"schemaVersion": 1});
        assert_eq!(
            serde_json::from_value::<ReservationLedger>(minimal).unwrap(),
            ReservationLedger::new()
        );

        let mut ledger = ReservationLedger::new();
        ledger
            .try_commit(bundle("persisted"), ReservationChecks::all_valid())
            .unwrap();
        let json = serde_json::to_string(&ledger).unwrap();
        assert_eq!(
            serde_json::from_str::<ReservationLedger>(&json).unwrap(),
            ledger
        );

        let mut wrong_version = serde_json::to_value(&ledger).unwrap();
        wrong_version["schemaVersion"] = serde_json::json!(2);
        assert!(serde_json::from_value::<ReservationLedger>(wrong_version).is_err());

        let mut noncanonical = serde_json::to_value(&ledger).unwrap();
        let stored_bundle = noncanonical["committed"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap();
        stored_bundle["tools"].as_array_mut().unwrap().reverse();
        assert!(serde_json::from_value::<ReservationLedger>(noncanonical).is_err());

        let mut shared_cat = bundle("shared-cat");
        shared_cat.cat.mode = ClaimMode::Capacity {
            units: 1,
            capacity: 2,
        };
        let mut empty = ReservationLedger::new();
        assert_eq!(
            empty.try_commit(shared_cat, ReservationChecks::all_valid()),
            Err(ReservationError::MalformedBundle)
        );
        assert!(empty.is_empty());
    }
}
