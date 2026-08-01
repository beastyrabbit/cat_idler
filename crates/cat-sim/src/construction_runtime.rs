//! Canonical LAI.63 staged-construction runtime bridge.
//!
//! `construction_stages` owns the persisted project state machine and
//! `storage_authority` owns every physical cargo identity. This bridge commits
//! one construction action per project and canonical tick only when an
//! existing visible task proves the exact footprint, whole-site work position,
//! assigned worker, and persisted route identities. Direct requests with
//! missing authority are errors; the world-tick adapter leaves such projects
//! visibly waiting. Neither path mutates partial state.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    construction_catalog::{BlueprintRequest, resolve_blueprint},
    construction_stages::{ConstructionMutationError, ConstructionStage, ConstructionTargetKind},
    content_manifest::ContentManifest,
    food_divine_policy::BoundCargoPurpose,
    leader_ai_runtime::{LeaderAiRuntimeError, LeaderAiRuntimeState},
    spatial_tasks::{SiteRef, TaskFootprint},
    storage_authority::{
        StorageAddress, StorageAuthorityError, StorageCommand, StorageCommandEnvelope,
        StorageIdentity,
    },
    task_runtime::{CargoLocation, TaskCategory, TaskId, TaskStage},
    world_tick::{BuildingRuntime, TilePos, footprint_for, footprint_tiles},
};

pub const MAX_CONSTRUCTION_CARGO_IDENTITIES_PER_ACTION: usize = 128;

/// Persisted, fail-closed reason why an operational construction project could
/// not be projected into the legacy world-building list.
///
/// The project remains `Operational`; callers may repair the missing world fact
/// and retry on a later tick. In particular, this type prevents a new canonical
/// target such as a Cookhouse or Fishing Hut from being guessed into an
/// unrelated legacy [`crate::types::BuildingType`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructionMaterializationGapReason {
    UnsupportedCanonicalTarget,
    HoleUpgradeOwnedByBlackHole,
    ImmutableBlueprintMismatch,
    ExistingBuildingIdCollision,
    OccupiedFootprint,
    UpgradeTargetMissing,
    UpgradeTargetAmbiguous,
    UpgradeLevelConflict,
    ExistingTargetNotOperational,
    PersistedWorldProjectionMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConstructionMaterializationRecord {
    Materialized {
        project_id: String,
        building_id: String,
        materialized_tick: u64,
    },
    Gap {
        project_id: String,
        reason: ConstructionMaterializationGapReason,
        observed_tick: u64,
    },
}

impl ConstructionMaterializationRecord {
    #[must_use]
    pub fn project_id(&self) -> &str {
        match self {
            Self::Materialized { project_id, .. } | Self::Gap { project_id, .. } => project_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstructionMaterializationOutcome {
    Materialized {
        project_id: String,
        building_id: String,
    },
    Gap {
        project_id: String,
        reason: ConstructionMaterializationGapReason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstructionRuntimeRequest {
    pub project_id: String,
    pub task_id: TaskId,
    pub expected_stage: ConstructionStage,
    pub worker_id: String,
    pub source_to_work_route_id: String,
    pub work_to_delivery_route_id: String,
    pub work_footprint: TaskFootprint,
    /// Whole physical lots which have already arrived at this construction
    /// site's authoritative `ConstructionCargo` address.
    pub delivered_identities: Vec<StorageIdentity>,
    /// Real elapsed work credited by the caller. Delivery and stage-opening
    /// actions require zero; labor actions require a nonzero duration.
    pub elapsed_work_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstructionRuntimeOutcome {
    SiteOpened,
    CargoAccepted {
        identities: usize,
        units: u64,
    },
    CargoPickedUp {
        identity: StorageIdentity,
        route_id: String,
    },
    CargoStaged {
        identity: StorageIdentity,
    },
    CargoAwaitingMore {
        staged_identities: usize,
    },
    LaborOpened {
        stage: ConstructionStage,
        consumed_identities: usize,
    },
    WorkAdvanced {
        applied_ms: u64,
        next_stage: ConstructionStage,
    },
    AlreadyAdvancedThisTick {
        stage: ConstructionStage,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstructionRuntimeError {
    EmptyAuthorityFact,
    MissingProject,
    MissingTask,
    WrongTaskCategory,
    TerminalTask,
    StageConflict,
    SpatialMismatch,
    WorkerMismatch,
    RouteMismatch,
    UnexpectedCargo,
    MissingCargo,
    CargoNotBound,
    CargoNotAtSite,
    CargoReservationMismatch,
    CargoContentMismatch,
    CargoQuantityMismatch,
    DuplicateCargoIdentity,
    TooManyCargoIdentities,
    WorkDurationMismatch,
    TickBeforeRuntime,
    Construction(ConstructionMutationError),
    Storage(StorageAuthorityError),
    Runtime(LeaderAiRuntimeError),
    ArithmeticOverflow,
}

impl std::fmt::Display for ConstructionRuntimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "canonical construction runtime error: {self:?}")
    }
}

impl std::error::Error for ConstructionRuntimeError {}

impl From<ConstructionMutationError> for ConstructionRuntimeError {
    fn from(value: ConstructionMutationError) -> Self {
        Self::Construction(value)
    }
}

impl From<StorageAuthorityError> for ConstructionRuntimeError {
    fn from(value: StorageAuthorityError) -> Self {
        Self::Storage(value)
    }
}

impl From<LeaderAiRuntimeError> for ConstructionRuntimeError {
    fn from(value: LeaderAiRuntimeError) -> Self {
        Self::Runtime(value)
    }
}

/// Apply one and only one canonical construction action.
///
/// The full aggregate is cloned first. Storage receipt writes, cargo
/// consumption, project progress, and the per-project tick guard therefore
/// commit atomically.
pub fn advance_construction(
    runtime: &mut LeaderAiRuntimeState,
    runtime_tick: u64,
    now_ms: i64,
    request: ConstructionRuntimeRequest,
) -> Result<ConstructionRuntimeOutcome, ConstructionRuntimeError> {
    if request.project_id.trim().is_empty() {
        return Err(ConstructionRuntimeError::EmptyAuthorityFact);
    }
    if runtime
        .last_processed_tick
        .is_some_and(|last| runtime_tick <= last)
    {
        return Err(ConstructionRuntimeError::TickBeforeRuntime);
    }
    let mut staged = runtime.clone();
    if staged
        .construction_runtime_ticks
        .get(&request.project_id)
        .is_some_and(|last| *last >= runtime_tick)
    {
        let stage = staged
            .construction_projects
            .get(&request.project_id)
            .ok_or(ConstructionRuntimeError::MissingProject)?
            .stage;
        return Ok(ConstructionRuntimeOutcome::AlreadyAdvancedThisTick { stage });
    }

    let project = staged
        .construction_projects
        .get(&request.project_id)
        .ok_or(ConstructionRuntimeError::MissingProject)?;
    if project.stage != request.expected_stage {
        return Err(ConstructionRuntimeError::StageConflict);
    }
    validate_task_authority(&staged, &request, project.stage)?;

    let outcome = match project.stage {
        ConstructionStage::SiteReserved => {
            require_no_cargo_or_work(&request)?;
            staged
                .construction_projects
                .get_mut(&request.project_id)
                .expect("project preflighted")
                .reserve_site(now_ms)?;
            ConstructionRuntimeOutcome::SiteOpened
        }
        stage if stage.is_delivery() => {
            if request.elapsed_work_ms != 0 {
                return Err(ConstructionRuntimeError::WorkDurationMismatch);
            }
            if request.delivered_identities.is_empty() {
                open_labor_stage(&mut staged, &request, now_ms)?
            } else {
                accept_delivered_cargo(&mut staged, &request, now_ms)?
            }
        }
        stage if stage.is_labor() => {
            if !request.delivered_identities.is_empty() {
                return Err(ConstructionRuntimeError::UnexpectedCargo);
            }
            if request.elapsed_work_ms == 0 {
                return Err(ConstructionRuntimeError::WorkDurationMismatch);
            }
            let advance = staged
                .construction_projects
                .get_mut(&request.project_id)
                .expect("project preflighted")
                .advance_work(request.elapsed_work_ms, now_ms)?;
            ConstructionRuntimeOutcome::WorkAdvanced {
                applied_ms: advance.applied_ms,
                next_stage: advance.next_stage,
            }
        }
        ConstructionStage::Operational | ConstructionStage::Cancelled => {
            return Err(ConstructionRuntimeError::StageConflict);
        }
        _ => unreachable!("all construction stages handled"),
    };

    staged
        .construction_runtime_ticks
        .insert(request.project_id, runtime_tick);
    staged.validate()?;
    *runtime = staged;
    Ok(outcome)
}

fn validate_task_authority(
    runtime: &LeaderAiRuntimeState,
    request: &ConstructionRuntimeRequest,
    stage: ConstructionStage,
) -> Result<(), ConstructionRuntimeError> {
    request
        .work_footprint
        .validate()
        .map_err(|_| ConstructionRuntimeError::SpatialMismatch)?;
    let project = runtime
        .construction_projects
        .get(&request.project_id)
        .ok_or(ConstructionRuntimeError::MissingProject)?;
    if project.footprint != request.work_footprint {
        return Err(ConstructionRuntimeError::SpatialMismatch);
    }
    let task = runtime
        .scheduling
        .visible_tasks
        .get(&request.task_id)
        .ok_or(ConstructionRuntimeError::MissingTask)?;
    if task.category != TaskCategory::BuildingConstruction {
        return Err(ConstructionRuntimeError::WrongTaskCategory);
    }
    if matches!(task.stage, TaskStage::Blocked | TaskStage::Cancelled)
        || (task.stage == TaskStage::Complete && !stage.is_delivery())
    {
        return Err(ConstructionRuntimeError::TerminalTask);
    }
    if site_footprint(task.spatial.objective.as_ref()) != Some(&project.footprint)
        || task.spatial.work_positions.len() != 1
        || site_footprint(task.spatial.work_positions.first().map(|slot| &slot.site))
            != Some(&project.footprint)
        || task.spatial.blocked_reason.is_some()
    {
        return Err(ConstructionRuntimeError::SpatialMismatch);
    }
    if stage != ConstructionStage::SiteReserved
        && !(stage.is_delivery() && task.stage == TaskStage::Complete)
    {
        if request.worker_id.trim().is_empty()
            || task.assigned_cat_ids.len() != 1
            || !task.assigned_cat_ids.contains(&request.worker_id)
            || runtime
                .cat_capabilities
                .cat_report(&request.worker_id)
                .is_none()
        {
            return Err(ConstructionRuntimeError::WorkerMismatch);
        }
    }
    if stage.is_delivery()
        && (!request.delivered_identities.is_empty()
            || runtime
                .construction_projects
                .get(&request.project_id)
                .is_some_and(|project| {
                    project
                        .active_delivery_bill()
                        .is_ok_and(|bill| bill.is_fully_delivered())
                }))
        && (request.source_to_work_route_id.trim().is_empty()
            || request.work_to_delivery_route_id.trim().is_empty()
            || task.route_ids.len() != 2
            || task.route_ids[0] != request.source_to_work_route_id
            || task.route_ids[1] != request.work_to_delivery_route_id)
    {
        return Err(ConstructionRuntimeError::RouteMismatch);
    }
    Ok(())
}

/// Advance every construction project which already has enough persisted
/// authority for its next action. Missing worker/route/cargo facts leave the
/// project visibly waiting; contradictory present facts return an error so the
/// surrounding runtime transaction can roll back.
pub fn advance_ready_construction(
    runtime: &mut LeaderAiRuntimeState,
    runtime_tick: u64,
    now_ms: i64,
) -> Result<Vec<(String, ConstructionRuntimeOutcome)>, ConstructionRuntimeError> {
    let project_ids = runtime
        .construction_projects
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let mut outcomes = Vec::new();
    for project_id in project_ids {
        let Some(project) = runtime.construction_projects.get(&project_id).cloned() else {
            continue;
        };
        if project.stage.is_terminal()
            || runtime
                .construction_runtime_ticks
                .get(&project_id)
                .is_some_and(|last| *last >= runtime_tick)
        {
            continue;
        }
        let expected_site_id = format!("construction_site:{project_id}");
        let Some((task_id, task)) = runtime
            .scheduling
            .visible_tasks
            .iter()
            .find(|(_, task)| {
                task.category == TaskCategory::BuildingConstruction
                    && task
                        .spatial
                        .objective
                        .as_ref()
                        .is_some_and(|site| site.metadata().stable_id == expected_site_id)
            })
            .map(|(task_id, task)| (task_id.clone(), task.clone()))
        else {
            continue;
        };
        if site_footprint(task.spatial.objective.as_ref()) != Some(&project.footprint)
            || task.spatial.work_positions.len() != 1
            || site_footprint(task.spatial.work_positions.first().map(|slot| &slot.site))
                != Some(&project.footprint)
        {
            continue;
        }
        let worker_id = task
            .assigned_cat_ids
            .iter()
            .next()
            .cloned()
            .unwrap_or_default();
        let (source_route, delivery_route) = if task.route_ids.len() == 2 {
            (task.route_ids[0].clone(), task.route_ids[1].clone())
        } else {
            (String::new(), String::new())
        };
        if project.stage.is_delivery()
            && let Some(outcome) = advance_visible_construction_cargo(
                runtime,
                &project,
                &task_id,
                &source_route,
                &delivery_route,
            )?
        {
            runtime
                .construction_runtime_ticks
                .insert(project_id.clone(), runtime_tick);
            runtime.validate()?;
            outcomes.push((project_id, outcome));
            continue;
        }
        let delivered_identities = if project.stage.is_delivery() {
            runtime
                .construction_storage_identities
                .get(&project_id)
                .into_iter()
                .flatten()
                .filter(|identity| {
                    runtime.storage.location(identity)
                        == Some(&StorageAddress::ConstructionCargo {
                            project_id: project_id.clone(),
                        })
                })
                .cloned()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let fully_delivered = project
            .active_delivery_bill()
            .is_ok_and(|bill| bill.is_fully_delivered());
        let site_cargo_complete = project.stage.is_delivery()
            && !fully_delivered
            && delivered_cargo_completes_missing_bill(runtime, &project, &delivered_identities)?;
        if project.stage.is_delivery()
            && !fully_delivered
            && !delivered_identities.is_empty()
            && !site_cargo_complete
        {
            if task.stage == TaskStage::Complete {
                retire_completed_construction_task(runtime, &task_id);
                runtime
                    .construction_runtime_ticks
                    .insert(project_id.clone(), runtime_tick);
                runtime.validate()?;
                outcomes.push((
                    project_id,
                    ConstructionRuntimeOutcome::CargoAwaitingMore {
                        staged_identities: delivered_identities.len(),
                    },
                ));
            }
            continue;
        }
        if project.stage != ConstructionStage::SiteReserved {
            let completed_delivery = project.stage.is_delivery()
                && task.stage == TaskStage::Complete
                && !delivered_identities.is_empty();
            if worker_id.is_empty() && !completed_delivery {
                continue;
            }
            if project.stage.is_delivery()
                && !fully_delivered
                && (delivered_identities.is_empty()
                    || source_route.is_empty()
                    || delivery_route.is_empty())
            {
                continue;
            }
            if project.stage.is_delivery()
                && fully_delivered
                && (source_route.is_empty() || delivery_route.is_empty())
            {
                continue;
            }
        }
        let elapsed_work_ms = if project.stage.is_labor() {
            runtime_tick
                .saturating_sub(runtime.last_processed_tick.unwrap_or(runtime_tick))
                .saturating_mul(60_000)
        } else {
            0
        };
        if project.stage.is_labor() && elapsed_work_ms == 0 {
            continue;
        }
        let request = ConstructionRuntimeRequest {
            project_id: project_id.clone(),
            task_id: task_id.clone(),
            expected_stage: project.stage,
            worker_id,
            source_to_work_route_id: source_route,
            work_to_delivery_route_id: delivery_route,
            work_footprint: project.footprint,
            delivered_identities: if fully_delivered {
                Vec::new()
            } else {
                delivered_identities
            },
            elapsed_work_ms,
        };
        let outcome = advance_construction(runtime, runtime_tick, now_ms, request)?;
        outcomes.push((project_id, outcome));
    }
    Ok(outcomes)
}

fn delivered_cargo_completes_missing_bill(
    runtime: &LeaderAiRuntimeState,
    project: &crate::construction_stages::ConstructionProject,
    identities: &[StorageIdentity],
) -> Result<bool, ConstructionRuntimeError> {
    let active = project.active_delivery_bill()?;
    let expected = active
        .lines
        .iter()
        .map(|line| (line.content_id.clone(), u64::from(line.missing_units())))
        .filter(|(_, units)| *units > 0)
        .collect::<BTreeMap<_, _>>();
    let mut actual = BTreeMap::<String, u64>::new();
    for identity in identities {
        let (content_id, quantity) = construction_identity_content_and_quantity(runtime, identity)?;
        let units = actual.entry(content_id).or_insert(0);
        *units = units
            .checked_add(u64::from(quantity))
            .ok_or(ConstructionRuntimeError::ArithmeticOverflow)?;
    }
    Ok(actual == expected)
}

/// Move exactly one already-bound construction lot according to the visible
/// task's persisted stage. This executor does not advance travel, choose a
/// worker, or synthesize routes. It only mirrors an observed pickup into the
/// exact first route and an observed deposit into `ConstructionCargo`.
fn advance_visible_construction_cargo(
    runtime: &mut LeaderAiRuntimeState,
    project: &crate::construction_stages::ConstructionProject,
    task_id: &TaskId,
    source_route_id: &str,
    delivery_route_id: &str,
) -> Result<Option<ConstructionRuntimeOutcome>, ConstructionRuntimeError> {
    if source_route_id.trim().is_empty() || delivery_route_id.trim().is_empty() {
        return Ok(None);
    }
    let task = runtime
        .scheduling
        .visible_tasks
        .get(task_id)
        .cloned()
        .ok_or(ConstructionRuntimeError::MissingTask)?;
    if task.category != TaskCategory::BuildingConstruction
        || task.route_ids.len() != 2
        || task.route_ids[0] != source_route_id
        || task.route_ids[1] != delivery_route_id
        || site_footprint(task.spatial.work_positions.first().map(|slot| &slot.site))
            != Some(&project.footprint)
        || site_footprint(task.spatial.delivery_endpoint.as_ref()) != Some(&project.footprint)
    {
        return Ok(None);
    }
    let Some(task_cargo) = task.cargo.as_ref() else {
        return Ok(None);
    };
    let exact_plan = exact_whole_lot_plan(runtime, project)?;
    let linked = runtime
        .scheduling
        .task_storage_identities
        .get(task_id)
        .cloned()
        .or_else(|| {
            exact_plan.iter().find_map(|identity| {
                let (content_id, quantity) =
                    construction_identity_content_and_quantity(runtime, identity).ok()?;
                (content_id == task_cargo.resource_id && u64::from(quantity) == task_cargo.quantity)
                    .then_some(identity.clone())
            })
        });
    let Some(identity) = linked else {
        return Ok(None);
    };
    if !exact_plan.contains(&identity)
        || !runtime
            .construction_storage_identities
            .get(&project.project_id)
            .is_some_and(|bound| bound.contains(&identity))
    {
        return Err(ConstructionRuntimeError::CargoNotBound);
    }
    runtime
        .scheduling
        .task_storage_identities
        .insert(task_id.clone(), identity.clone());

    let location = runtime
        .storage
        .location(&identity)
        .cloned()
        .ok_or(ConstructionRuntimeError::CargoNotBound)?;
    let carried = matches!(task_cargo.location, CargoLocation::Carried { .. });
    let deposited = matches!(
        task_cargo.location,
        CargoLocation::DepositedAtEndpoint { .. }
    );
    if carried
        && matches!(
            task.stage,
            TaskStage::TravelToWork
                | TaskStage::Work
                | TaskStage::TravelToEndpoint
                | TaskStage::Deposit
        )
        && !matches!(
            location,
            StorageAddress::RouteCargo { .. } | StorageAddress::ConstructionCargo { .. }
        )
    {
        unreserve_construction_identity(runtime, &identity, project, task_id)?;
        execute_storage(
            runtime,
            format!(
                "construction_pickup:{}:{}",
                project.project_id,
                storage_identity_suffix(&identity)
            ),
            format!(
                "construction_pickup_v1:{}:{}:{}",
                project.project_id,
                storage_identity_suffix(&identity),
                source_route_id
            ),
            StorageCommand::Move {
                identity: identity.clone(),
                destination: StorageAddress::RouteCargo {
                    route_id: source_route_id.to_owned(),
                },
            },
        )?;
        return Ok(Some(ConstructionRuntimeOutcome::CargoPickedUp {
            identity,
            route_id: source_route_id.to_owned(),
        }));
    }
    if deposited
        && matches!(task.stage, TaskStage::Deposit | TaskStage::Complete)
        && matches!(
            location,
            StorageAddress::RouteCargo { ref route_id }
                if route_id == source_route_id || route_id == delivery_route_id
        )
    {
        unreserve_construction_identity(runtime, &identity, project, task_id)?;
        execute_storage(
            runtime,
            format!(
                "construction_stage:{}:{}",
                project.project_id,
                storage_identity_suffix(&identity)
            ),
            format!(
                "construction_stage_v1:{}:{}:{}",
                project.project_id,
                storage_identity_suffix(&identity),
                delivery_route_id
            ),
            StorageCommand::StageConstruction {
                project_id: project.project_id.clone(),
                identities: vec![identity.clone()],
            },
        )?;
        return Ok(Some(ConstructionRuntimeOutcome::CargoStaged { identity }));
    }
    Ok(None)
}

fn unreserve_construction_identity(
    runtime: &mut LeaderAiRuntimeState,
    identity: &StorageIdentity,
    project: &crate::construction_stages::ConstructionProject,
    task_id: &TaskId,
) -> Result<(), ConstructionRuntimeError> {
    let owner = construction_identity_reservation(runtime, identity)?;
    let Some(owner) = owner else {
        return Ok(());
    };
    if owner != project.project_id && owner != task_id.as_str() {
        return Err(ConstructionRuntimeError::CargoReservationMismatch);
    }
    execute_storage(
        runtime,
        format!(
            "construction_haul_unreserve:{}:{}",
            project.project_id,
            storage_identity_suffix(identity)
        ),
        format!(
            "construction_haul_unreserve_v1:{}:{}:{}",
            project.project_id,
            storage_identity_suffix(identity),
            owner
        ),
        StorageCommand::Unreserve {
            identity: identity.clone(),
            owner,
        },
    )
}

/// Select only identities which can satisfy the complete active bill without
/// splitting a physical lot. Exact items and fixtures contribute one unit per
/// identity; existing site cargo remains part of the exact sum.
fn exact_whole_lot_plan(
    runtime: &LeaderAiRuntimeState,
    project: &crate::construction_stages::ConstructionProject,
) -> Result<Vec<StorageIdentity>, ConstructionRuntimeError> {
    let bill = project.active_delivery_bill()?;
    let bound = runtime
        .construction_storage_identities
        .get(&project.project_id)
        .ok_or(ConstructionRuntimeError::MissingProject)?;
    let mut plan = Vec::new();
    for line in &bill.lines {
        let missing = line.missing_units();
        if missing == 0 {
            continue;
        }
        let mut candidates = bound
            .iter()
            .filter_map(|identity| {
                let (content_id, quantity) =
                    construction_identity_content_and_quantity(runtime, identity).ok()?;
                (content_id == line.content_id
                    && runtime.storage.location(identity).is_some()
                    && !matches!(
                        runtime.storage.location(identity),
                        Some(StorageAddress::ConstructionCargo { project_id })
                            if project_id != &project.project_id
                    ))
                .then_some((identity.clone(), quantity))
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| left.0.cmp(&right.0));
        let selected = exact_lot_subset(&candidates, missing)
            .ok_or(ConstructionRuntimeError::CargoQuantityMismatch)?;
        plan.extend(selected);
    }
    plan.sort();
    plan.dedup();
    Ok(plan)
}

fn exact_lot_subset(
    candidates: &[(StorageIdentity, u32)],
    target: u32,
) -> Option<Vec<StorageIdentity>> {
    const MAX_SUBSET_STATES: usize = 8_192;
    let mut sums = BTreeMap::<u32, Vec<StorageIdentity>>::from([(0, Vec::new())]);
    for (identity, units) in candidates {
        let additions = sums
            .iter()
            .filter_map(|(sum, selected)| {
                let next = sum.checked_add(*units)?;
                (next <= target).then(|| {
                    let mut selected = selected.clone();
                    selected.push(identity.clone());
                    (next, selected)
                })
            })
            .collect::<Vec<_>>();
        for (sum, selected) in additions {
            sums.entry(sum).or_insert(selected);
        }
        if sums.len() > MAX_SUBSET_STATES {
            return None;
        }
    }
    sums.remove(&target)
}

fn storage_identity_suffix(identity: &StorageIdentity) -> String {
    match identity {
        StorageIdentity::Lot(id) => format!("lot:{id}"),
        StorageIdentity::Item(id) => format!("item:{id}"),
    }
}

fn construction_identity_content_and_quantity(
    runtime: &LeaderAiRuntimeState,
    identity: &StorageIdentity,
) -> Result<(String, u32), ConstructionRuntimeError> {
    match identity {
        StorageIdentity::Lot(lot_id) => {
            let lot = runtime
                .storage
                .ledger()
                .lot(lot_id)
                .ok_or(ConstructionRuntimeError::CargoNotBound)?;
            Ok((lot.key.content_id.as_str().to_owned(), lot.quantity))
        }
        StorageIdentity::Item(item_id) => {
            let item = runtime
                .storage
                .ledger()
                .item(item_id)
                .ok_or(ConstructionRuntimeError::CargoNotBound)?;
            let manifest = ContentManifest::embedded();
            let content_id = manifest
                .item_definitions
                .iter()
                .find(|definition| definition.id == item.definition_id)
                .map(|definition| definition.content_id.as_str())
                .or_else(|| {
                    manifest
                        .fixtures
                        .iter()
                        .find(|fixture| fixture.id == item.definition_id)
                        .map(|fixture| fixture.content_id.as_str())
                })
                .ok_or(ConstructionRuntimeError::CargoContentMismatch)?;
            Ok((content_id.to_owned(), 1))
        }
    }
}

fn construction_identity_reservation(
    runtime: &LeaderAiRuntimeState,
    identity: &StorageIdentity,
) -> Result<Option<String>, ConstructionRuntimeError> {
    match identity {
        StorageIdentity::Lot(lot_id) => runtime
            .storage
            .ledger()
            .lot(lot_id)
            .map(|lot| lot.reservation.clone())
            .ok_or(ConstructionRuntimeError::CargoNotBound),
        StorageIdentity::Item(item_id) => runtime
            .storage
            .ledger()
            .item(item_id)
            .map(|item| item.reservation.clone())
            .ok_or(ConstructionRuntimeError::CargoNotBound),
    }
}

fn validate_purpose_bound_construction_stage(
    runtime: &LeaderAiRuntimeState,
    identity: &StorageIdentity,
    project_id: &str,
    stage: ConstructionStage,
) -> Result<(), ConstructionRuntimeError> {
    let Some(purpose) = runtime.purpose_bound_storage.get(identity) else {
        return Ok(());
    };
    let stage_index = match stage {
        ConstructionStage::DeliverScaffold => 0,
        ConstructionStage::DeliverStructure => 1,
        ConstructionStage::DeliverFitOut => 2,
        _ => return Err(ConstructionRuntimeError::StageConflict),
    };
    if matches!(
        purpose,
        BoundCargoPurpose::Construction {
            project_id: bound_project,
            stage_index: bound_stage,
        } if bound_project == project_id && *bound_stage == stage_index
    ) {
        Ok(())
    } else {
        Err(ConstructionRuntimeError::CargoContentMismatch)
    }
}

/// Project every operational canonical project into the legacy world building
/// list in stable project-ID order.
///
/// Both aggregates are staged before commit. A successfully materialized row is
/// restart-idempotent, while an unrepresentable or contradictory target writes a
/// typed persisted gap and leaves the canonical project operational. The
/// immutable catalog is resolved again at the boundary; this function never
/// invents a generic recipe or building type.
pub fn materialize_operational_projects(
    runtime: &mut LeaderAiRuntimeState,
    buildings: &mut Vec<BuildingRuntime>,
    runtime_tick: u64,
) -> Result<Vec<ConstructionMaterializationOutcome>, ConstructionRuntimeError> {
    let mut staged_runtime = runtime.clone();
    let mut staged_buildings = buildings.clone();
    let project_ids = staged_runtime
        .construction_projects
        .iter()
        .filter_map(|(project_id, project)| {
            (project.stage == ConstructionStage::Operational).then_some(project_id.clone())
        })
        .collect::<Vec<_>>();
    let mut outcomes = Vec::with_capacity(project_ids.len());

    for project_id in project_ids {
        let project = staged_runtime
            .construction_projects
            .get(&project_id)
            .cloned()
            .ok_or(ConstructionRuntimeError::MissingProject)?;
        let record = match staged_runtime
            .construction_materializations
            .get(&project_id)
            .cloned()
        {
            Some(existing @ ConstructionMaterializationRecord::Materialized { .. }) => {
                if materialized_record_matches_world(&existing, &project, &staged_buildings) {
                    existing
                } else {
                    ConstructionMaterializationRecord::Gap {
                        project_id: project_id.clone(),
                        reason:
                            ConstructionMaterializationGapReason::PersistedWorldProjectionMismatch,
                        observed_tick: runtime_tick,
                    }
                }
            }
            Some(ConstructionMaterializationRecord::Gap { .. }) | None => {
                materialize_one_project(&project, &mut staged_buildings, runtime_tick)
            }
        };
        let outcome = match &record {
            ConstructionMaterializationRecord::Materialized {
                project_id,
                building_id,
                ..
            } => ConstructionMaterializationOutcome::Materialized {
                project_id: project_id.clone(),
                building_id: building_id.clone(),
            },
            ConstructionMaterializationRecord::Gap {
                project_id, reason, ..
            } => ConstructionMaterializationOutcome::Gap {
                project_id: project_id.clone(),
                reason: *reason,
            },
        };
        staged_runtime
            .construction_materializations
            .insert(project_id, record);
        outcomes.push(outcome);
    }

    staged_runtime.validate()?;
    *runtime = staged_runtime;
    *buildings = staged_buildings;
    Ok(outcomes)
}

fn materialized_record_matches_world(
    record: &ConstructionMaterializationRecord,
    project: &crate::construction_stages::ConstructionProject,
    buildings: &[BuildingRuntime],
) -> bool {
    let ConstructionMaterializationRecord::Materialized { building_id, .. } = record else {
        return false;
    };
    let Some(building_type) = project.building_type else {
        return false;
    };
    let position = TilePos {
        x: project.footprint.anchor.x,
        y: project.footprint.anchor.y,
    };
    buildings.iter().any(|building| {
        building.id == *building_id
            && exact_operational_building(building, building_type, project.target_level, position)
    })
}

fn materialize_one_project(
    project: &crate::construction_stages::ConstructionProject,
    buildings: &mut Vec<BuildingRuntime>,
    runtime_tick: u64,
) -> ConstructionMaterializationRecord {
    let gap = |reason| ConstructionMaterializationRecord::Gap {
        project_id: project.project_id.clone(),
        reason,
        observed_tick: runtime_tick,
    };

    if project.target_kind == ConstructionTargetKind::HoleUpgrade {
        return gap(ConstructionMaterializationGapReason::HoleUpgradeOwnedByBlackHole);
    }
    let Some(building_type) = project.building_type else {
        return gap(ConstructionMaterializationGapReason::UnsupportedCanonicalTarget);
    };
    let blueprint_request = match project.target_kind {
        ConstructionTargetKind::Building => BlueprintRequest::NewBuilding(building_type),
        ConstructionTargetKind::BuildingUpgrade => {
            let Ok(target_level) = u8::try_from(project.target_level) else {
                return gap(ConstructionMaterializationGapReason::ImmutableBlueprintMismatch);
            };
            BlueprintRequest::BuildingUpgrade {
                building_type,
                target_level,
            }
        }
        ConstructionTargetKind::HoleUpgrade => unreachable!("handled above"),
    };
    let Ok(blueprint) = resolve_blueprint(blueprint_request) else {
        return gap(ConstructionMaterializationGapReason::ImmutableBlueprintMismatch);
    };
    if blueprint.target_kind != project.target_kind
        || blueprint.building_type != building_type
        || u32::from(blueprint.target_level) != project.target_level
        || blueprint.footprint.width != project.footprint.width
        || blueprint.footprint.height != project.footprint.height
    {
        return gap(ConstructionMaterializationGapReason::ImmutableBlueprintMismatch);
    }

    let position = TilePos {
        x: project.footprint.anchor.x,
        y: project.footprint.anchor.y,
    };
    match project.target_kind {
        ConstructionTargetKind::Building => {
            let canonical_id = format!("construction_building:{}", project.project_id);
            if let Some(existing) = buildings
                .iter()
                .find(|building| building.id == canonical_id)
            {
                return if exact_operational_building(
                    existing,
                    building_type,
                    project.target_level,
                    position,
                ) {
                    ConstructionMaterializationRecord::Materialized {
                        project_id: project.project_id.clone(),
                        building_id: existing.id.clone(),
                        materialized_tick: runtime_tick,
                    }
                } else {
                    gap(ConstructionMaterializationGapReason::ExistingBuildingIdCollision)
                };
            }

            let exact_targets = buildings
                .iter()
                .filter(|building| {
                    building.building_type == building_type && building.position == position
                })
                .collect::<Vec<_>>();
            if exact_targets.len() > 1 {
                return gap(ConstructionMaterializationGapReason::UpgradeTargetAmbiguous);
            }
            if let Some(existing) = exact_targets.first() {
                return if exact_operational_building(
                    existing,
                    building_type,
                    project.target_level,
                    position,
                ) {
                    ConstructionMaterializationRecord::Materialized {
                        project_id: project.project_id.clone(),
                        building_id: existing.id.clone(),
                        materialized_tick: runtime_tick,
                    }
                } else {
                    gap(ConstructionMaterializationGapReason::ExistingTargetNotOperational)
                };
            }
            if buildings.iter().any(|building| {
                building_footprint(building)
                    .iter()
                    .any(|tile| project.footprint.tiles.as_slice().contains(tile))
            }) {
                return gap(ConstructionMaterializationGapReason::OccupiedFootprint);
            }

            buildings.push(BuildingRuntime {
                id: canonical_id.clone(),
                building_type,
                level: project.target_level,
                position,
                is_complete: true,
                construction_progress: 100,
                ..BuildingRuntime::default()
            });
            ConstructionMaterializationRecord::Materialized {
                project_id: project.project_id.clone(),
                building_id: canonical_id,
                materialized_tick: runtime_tick,
            }
        }
        ConstructionTargetKind::BuildingUpgrade => {
            let matching = buildings
                .iter()
                .enumerate()
                .filter(|(_, building)| {
                    building.building_type == building_type && building.position == position
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            let [index] = matching.as_slice() else {
                return gap(if matching.is_empty() {
                    ConstructionMaterializationGapReason::UpgradeTargetMissing
                } else {
                    ConstructionMaterializationGapReason::UpgradeTargetAmbiguous
                });
            };
            let building = &mut buildings[*index];
            if !building.is_complete {
                return gap(ConstructionMaterializationGapReason::ExistingTargetNotOperational);
            }
            if building.level > project.target_level {
                return gap(ConstructionMaterializationGapReason::UpgradeLevelConflict);
            }
            building.level = project.target_level;
            building.construction_progress = 100;
            ConstructionMaterializationRecord::Materialized {
                project_id: project.project_id.clone(),
                building_id: building.id.clone(),
                materialized_tick: runtime_tick,
            }
        }
        ConstructionTargetKind::HoleUpgrade => unreachable!("handled above"),
    }
}

fn exact_operational_building(
    building: &BuildingRuntime,
    building_type: crate::types::BuildingType,
    target_level: u32,
    position: TilePos,
) -> bool {
    building.building_type == building_type
        && building.level == target_level
        && building.position == position
        && building.is_complete
}

fn building_footprint(building: &BuildingRuntime) -> Vec<TilePos> {
    let (width, height) = footprint_for(building.building_type);
    footprint_tiles(building.position, width, height)
}

fn site_footprint(site: Option<&SiteRef>) -> Option<&TaskFootprint> {
    match site? {
        SiteRef::Building { footprint, .. }
        | SiteRef::ResourceSource { footprint, .. }
        | SiteRef::Stockpile { footprint, .. }
        | SiteRef::VillageTradeEndpoint { footprint, .. }
        | SiteRef::Rect { footprint, .. } => Some(footprint),
        SiteRef::Shrine { footprint, .. } => Some(footprint),
        SiteRef::Tile { .. } | SiteRef::OrderedTiles { .. } | SiteRef::OrderedRoute { .. } => None,
    }
}

fn require_no_cargo_or_work(
    request: &ConstructionRuntimeRequest,
) -> Result<(), ConstructionRuntimeError> {
    if !request.delivered_identities.is_empty() {
        return Err(ConstructionRuntimeError::UnexpectedCargo);
    }
    if request.elapsed_work_ms != 0 {
        return Err(ConstructionRuntimeError::WorkDurationMismatch);
    }
    Ok(())
}

fn accept_delivered_cargo(
    runtime: &mut LeaderAiRuntimeState,
    request: &ConstructionRuntimeRequest,
    now_ms: i64,
) -> Result<ConstructionRuntimeOutcome, ConstructionRuntimeError> {
    if request.delivered_identities.len() > MAX_CONSTRUCTION_CARGO_IDENTITIES_PER_ACTION {
        return Err(ConstructionRuntimeError::TooManyCargoIdentities);
    }
    let bound = runtime
        .construction_storage_identities
        .get(&request.project_id)
        .ok_or(ConstructionRuntimeError::MissingProject)?;
    let mut seen = BTreeSet::new();
    let mut deliveries = Vec::with_capacity(request.delivered_identities.len());
    for identity in &request.delivered_identities {
        if !seen.insert(identity.clone()) {
            return Err(ConstructionRuntimeError::DuplicateCargoIdentity);
        }
        if !bound.contains(identity) {
            return Err(ConstructionRuntimeError::CargoNotBound);
        }
        if runtime.storage.location(identity)
            != Some(&StorageAddress::ConstructionCargo {
                project_id: request.project_id.clone(),
            })
        {
            return Err(ConstructionRuntimeError::CargoNotAtSite);
        }
        validate_purpose_bound_construction_stage(
            runtime,
            identity,
            &request.project_id,
            request.expected_stage,
        )?;
        let (content_id, quantity) = construction_identity_content_and_quantity(runtime, identity)?;
        if construction_identity_reservation(runtime, identity)?.as_deref()
            != Some(request.project_id.as_str())
        {
            return Err(ConstructionRuntimeError::CargoReservationMismatch);
        }
        deliveries.push((content_id, quantity));
    }

    let project = runtime
        .construction_projects
        .get_mut(&request.project_id)
        .expect("project preflighted");
    let mut requested_by_content = BTreeMap::<String, u64>::new();
    for (content_id, units) in &deliveries {
        let total = requested_by_content.entry(content_id.clone()).or_insert(0);
        *total = total
            .checked_add(u64::from(*units))
            .ok_or(ConstructionRuntimeError::ArithmeticOverflow)?;
    }
    let active = project.active_delivery_bill()?;
    let missing_by_content = active
        .lines
        .iter()
        .map(|line| (line.content_id.clone(), u64::from(line.missing_units())))
        .filter(|(_, units)| *units > 0)
        .collect::<BTreeMap<_, _>>();
    for (content_id, units) in &requested_by_content {
        let line = active
            .lines
            .iter()
            .find(|line| &line.content_id == content_id)
            .ok_or(ConstructionRuntimeError::CargoContentMismatch)?;
        if *units > u64::from(line.missing_units()) {
            return Err(ConstructionRuntimeError::CargoQuantityMismatch);
        }
    }
    // A physical lot is indivisible in this leaf and accepted identities stay
    // at the site until stage opening consumes them. Accept the complete
    // outstanding bill atomically so a later tick cannot count the same
    // already-arrived lot twice.
    if requested_by_content != missing_by_content {
        return Err(ConstructionRuntimeError::CargoQuantityMismatch);
    }
    for (content_id, units) in deliveries {
        project.begin_transit(&content_id, units, now_ms)?;
        project.deliver_transit(&content_id, units, now_ms)?;
    }
    let units = requested_by_content
        .values()
        .try_fold(0_u64, |total, units| {
            total
                .checked_add(*units)
                .ok_or(ConstructionRuntimeError::ArithmeticOverflow)
        })?;
    Ok(ConstructionRuntimeOutcome::CargoAccepted {
        identities: request.delivered_identities.len(),
        units,
    })
}

fn open_labor_stage(
    runtime: &mut LeaderAiRuntimeState,
    request: &ConstructionRuntimeRequest,
    now_ms: i64,
) -> Result<ConstructionRuntimeOutcome, ConstructionRuntimeError> {
    if runtime
        .scheduling
        .visible_tasks
        .get(&request.task_id)
        .is_none_or(|task| task.stage != TaskStage::Complete)
    {
        return Err(ConstructionRuntimeError::TerminalTask);
    }
    let project = runtime
        .construction_projects
        .get(&request.project_id)
        .expect("project preflighted");
    let active = project.active_delivery_bill()?;
    if !active.is_fully_delivered() {
        return Err(ConstructionRuntimeError::MissingCargo);
    }
    let expected = active
        .lines
        .iter()
        .map(|line| (line.content_id.clone(), u64::from(line.required_units)))
        .collect::<BTreeMap<_, _>>();
    let bound = runtime
        .construction_storage_identities
        .get(&request.project_id)
        .ok_or(ConstructionRuntimeError::MissingProject)?;
    let mut consumed = Vec::new();
    let mut actual = BTreeMap::<String, u64>::new();
    for identity in bound {
        if runtime.storage.location(identity)
            != Some(&StorageAddress::ConstructionCargo {
                project_id: request.project_id.clone(),
            })
        {
            continue;
        }
        validate_purpose_bound_construction_stage(
            runtime,
            identity,
            &request.project_id,
            request.expected_stage,
        )?;
        let (content_id, quantity) = construction_identity_content_and_quantity(runtime, identity)?;
        if construction_identity_reservation(runtime, identity)?.as_deref()
            != Some(request.project_id.as_str())
        {
            return Err(ConstructionRuntimeError::CargoReservationMismatch);
        }
        let units = actual.entry(content_id).or_insert(0);
        *units = units
            .checked_add(u64::from(quantity))
            .ok_or(ConstructionRuntimeError::ArithmeticOverflow)?;
        consumed.push((identity.clone(), quantity));
    }
    if actual != expected {
        return Err(ConstructionRuntimeError::CargoQuantityMismatch);
    }

    retire_completed_construction_task(runtime, &request.task_id);
    for (index, (identity, _)) in consumed.iter().enumerate() {
        execute_storage(
            runtime,
            format!(
                "construction_unreserve:{}:{:?}:{index}",
                request.project_id, request.expected_stage
            ),
            format!(
                "construction_unreserve_v1:{}:{:?}:{index}",
                request.project_id, request.expected_stage
            ),
            StorageCommand::Unreserve {
                identity: identity.clone(),
                owner: request.project_id.clone(),
            },
        )?;
    }
    execute_storage(
        runtime,
        format!(
            "construction_consume:{}:{:?}",
            request.project_id, request.expected_stage
        ),
        format!(
            "construction_consume_v1:{}:{:?}",
            request.project_id, request.expected_stage
        ),
        StorageCommand::Consume {
            bulk: consumed
                .iter()
                .filter_map(|(identity, units)| match identity {
                    StorageIdentity::Lot(lot_id) => Some((lot_id.clone(), *units)),
                    StorageIdentity::Item(_) => None,
                })
                .collect(),
            items: consumed
                .iter()
                .filter_map(|(identity, _)| match identity {
                    StorageIdentity::Item(item_id) => Some(item_id.clone()),
                    StorageIdentity::Lot(_) => None,
                })
                .collect(),
        },
    )?;
    {
        let bindings = runtime
            .construction_storage_identities
            .get_mut(&request.project_id)
            .expect("project preflighted");
        for (identity, _) in &consumed {
            bindings.remove(identity);
        }
    }
    for (identity, _) in &consumed {
        runtime.purpose_bound_storage.remove(identity);
    }
    let labor_stage_index = match request.expected_stage {
        ConstructionStage::DeliverScaffold => 0,
        ConstructionStage::DeliverStructure => 1,
        ConstructionStage::DeliverFitOut => 2,
        _ => return Err(ConstructionRuntimeError::StageConflict),
    };
    let miracle_credit_ms = runtime
        .construction_miracles
        .take_pending_credit(&request.project_id, labor_stage_index);
    let project = runtime
        .construction_projects
        .get_mut(&request.project_id)
        .expect("project preflighted");
    project.begin_stage_work(now_ms)?;
    if miracle_credit_ms > 0 {
        project.advance_work(miracle_credit_ms, now_ms)?;
    }
    Ok(ConstructionRuntimeOutcome::LaborOpened {
        stage: project.stage,
        consumed_identities: consumed.len(),
    })
}

fn retire_completed_construction_task(runtime: &mut LeaderAiRuntimeState, task_id: &TaskId) {
    runtime.scheduling.visible_tasks.remove(task_id);
    runtime.scheduling.resolved_spatial_tasks.remove(task_id);
    runtime.scheduling.task_storage_identities.remove(task_id);
    runtime.scheduling.task_storage_endpoints.remove(task_id);
    if let Some(world_id) = runtime.scheduling.world_reservation_ids.remove(task_id) {
        let _ = runtime.scheduling.world_reservations.release(&world_id);
    }
}

fn execute_storage(
    runtime: &mut LeaderAiRuntimeState,
    command_id: String,
    fingerprint: String,
    command: StorageCommand,
) -> Result<(), ConstructionRuntimeError> {
    let sequence = runtime
        .storage
        .version()
        .checked_add(1)
        .ok_or(ConstructionRuntimeError::ArithmeticOverflow)?;
    runtime.storage.execute(StorageCommandEnvelope {
        colony_id: runtime.colony_id.clone(),
        command_id,
        fingerprint,
        sequence,
        command,
    })?;
    Ok(())
}
