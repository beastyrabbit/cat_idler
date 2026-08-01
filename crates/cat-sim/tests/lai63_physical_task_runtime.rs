//! Focused LAI.63 physical-task runtime contracts.

use std::collections::BTreeMap;

use cat_sim::{
    authority::{AuthorityActor, AuthorityDomain},
    cat_capability_authority::{ProductiveOutcome, WorkActivity},
    entities::{Cat, CatActivity, CatNeeds, CatStats, MapType, Position, RoleXp},
    intent_graph::{Intent, IntentInsert},
    leader_ai_runtime::{
        LeaderAiRuntimeState, PhysicalTaskBlockReason, PhysicalTaskCargoBinding,
        PhysicalTaskExecutionOutcome, PhysicalTaskExecutionRequest, PhysicalTaskInterruption,
        PhysicalTaskWorkReceipt, PhysicalTaskWorkerReport,
    },
    physical_storage::StorageCompatibility,
    planner_core::{IntentId, PlannerId},
    quality_lots::{BulkLotKey, LotLocation, LotProvenance, PhysicalLot, QualityBand},
    spatial_resolver::{ResolvedSpatialTask, SpatialTaskCategory},
    spatial_tasks::{
        Rect, ResourceSourceKind, SiteMetadata, SiteRef, SpatialObjective, TaskFootprint,
        TilePoint, WorkSlot,
    },
    storage_authority::{
        StorageAddress, StorageCommand, StorageCommandEnvelope, StorageIdentity, StorageZone,
        StorageZoneKind,
    },
    task_runtime::{CargoLocation, TaskCategory, TaskId, TaskStage, VisibleTaskRuntime},
    types::BuildingType,
};

fn id(namespace: &str, value: &str) -> PlannerId {
    PlannerId::derive(namespace, [value])
}

fn base_cat() -> Cat {
    Cat {
        id: "hunter".to_owned(),
        colony_id: "colony_one".to_owned(),
        name: "Moss".to_owned(),
        parent_ids: vec![None, None],
        birth_time: 0,
        death_time: None,
        stats: CatStats {
            attack: 10.0,
            defense: 10.0,
            hunting: 10.0,
            medicine: 10.0,
            cleaning: 10.0,
            building: 10.0,
            leadership: 10.0,
            vision: 10.0,
        },
        needs: CatNeeds {
            hunger: 100.0,
            thirst: 100.0,
            rest: 100.0,
            health: 100.0,
        },
        current_task: None,
        position: Position {
            map: MapType::World,
            x: 0.0,
            y: 0.0,
        },
        destination: None,
        carrying: None,
        activity: CatActivity::Idle,
        is_pregnant: false,
        pregnancy_due_time: None,
        age_hours: 30.0,
        pregnancy_due_age_hours: None,
        pregnancy_mate_id: None,
        sprite_params: Some(BTreeMap::new()),
        specialization: None,
        role_xp: RoleXp::default(),
        skills: Default::default(),
        boosted: false,
        preferred_labors: Default::default(),
    }
}

fn footprint(x: i32, y: i32) -> TaskFootprint {
    TaskFootprint::rectangular(Rect::try_new(TilePoint { x, y }, 1, 1).unwrap())
}

fn route(name: &str, first: TilePoint, second: TilePoint) -> SiteRef {
    SiteRef::OrderedRoute {
        metadata: SiteMetadata::revealed(name),
        route: vec![first, second],
    }
}

fn resolved_hunt() -> ResolvedSpatialTask {
    let objective = SiteRef::ResourceSource {
        metadata: SiteMetadata::revealed("cave_lair"),
        source_id: "cave_lair".to_owned(),
        resource_kind: ResourceSourceKind::Hunting,
        footprint: footprint(4, 4),
    };
    let work = WorkSlot::exclusive(
        "cave_lair_bank",
        SiteRef::Tile {
            metadata: SiteMetadata::revealed("cave_lair_bank"),
            tile: TilePoint { x: 4, y: 5 },
        },
    );
    let endpoint = SiteRef::Stockpile {
        metadata: SiteMetadata::revealed("home_stockpile"),
        stockpile_id: "home_stockpile".to_owned(),
        footprint: footprint(10, 0),
    };
    ResolvedSpatialTask {
        category: SpatialTaskCategory::Hunt,
        spatial: SpatialObjective::resolved(objective, vec![work], Some(endpoint)),
        source_to_work_route: route(
            "route_lair_to_bank",
            TilePoint { x: 3, y: 4 },
            TilePoint { x: 3, y: 5 },
        ),
        work_to_delivery_route: route(
            "route_bank_to_home",
            TilePoint { x: 3, y: 5 },
            TilePoint { x: 3, y: 6 },
        ),
        source_units: 3,
        source_capacity: 3,
        delivery_units: 3,
        delivery_capacity: 3,
        source_to_work_route_capacity: 1,
        work_to_delivery_route_capacity: 1,
    }
}

fn resolved_water() -> ResolvedSpatialTask {
    let objective = SiteRef::ResourceSource {
        metadata: SiteMetadata::revealed("river_source"),
        source_id: "river_source".to_owned(),
        resource_kind: ResourceSourceKind::Water,
        footprint: footprint(20, 20),
    };
    let work = WorkSlot::exclusive(
        "river_bank",
        SiteRef::Tile {
            metadata: SiteMetadata::revealed("river_bank"),
            tile: TilePoint { x: 20, y: 21 },
        },
    );
    ResolvedSpatialTask {
        category: SpatialTaskCategory::FetchWater,
        spatial: SpatialObjective::resolved(
            objective,
            vec![work],
            Some(SiteRef::Stockpile {
                metadata: SiteMetadata::revealed("home_stockpile"),
                stockpile_id: "home_stockpile".to_owned(),
                footprint: footprint(10, 0),
            }),
        ),
        source_to_work_route: route(
            "route_river_to_bank",
            TilePoint { x: 19, y: 20 },
            TilePoint { x: 19, y: 21 },
        ),
        work_to_delivery_route: route(
            "route_bank_to_home_water",
            TilePoint { x: 19, y: 21 },
            TilePoint { x: 19, y: 22 },
        ),
        source_units: 3,
        source_capacity: 3,
        delivery_units: 3,
        delivery_capacity: 3,
        source_to_work_route_capacity: 1,
        work_to_delivery_route_capacity: 1,
    }
}

fn resolved_workshop() -> ResolvedSpatialTask {
    let objective = SiteRef::building(
        "workshop_one",
        BuildingType::Workshop,
        TilePoint { x: 30, y: 30 },
    );
    let work = WorkSlot::exclusive(
        "workshop_door",
        SiteRef::Tile {
            metadata: SiteMetadata::revealed("workshop_door"),
            tile: TilePoint { x: 33, y: 31 },
        },
    );
    ResolvedSpatialTask {
        category: SpatialTaskCategory::WorkshopWork,
        spatial: SpatialObjective::resolved(
            objective,
            vec![work],
            Some(SiteRef::Stockpile {
                metadata: SiteMetadata::revealed("home_stockpile"),
                stockpile_id: "home_stockpile".to_owned(),
                footprint: footprint(10, 0),
            }),
        ),
        source_to_work_route: route(
            "route_workshop_in",
            TilePoint { x: 29, y: 31 },
            TilePoint { x: 29, y: 32 },
        ),
        work_to_delivery_route: route(
            "route_workshop_out",
            TilePoint { x: 29, y: 32 },
            TilePoint { x: 29, y: 33 },
        ),
        source_units: 3,
        source_capacity: 3,
        delivery_units: 3,
        delivery_capacity: 3,
        source_to_work_route_capacity: 1,
        work_to_delivery_route_capacity: 1,
    }
}

fn execute_storage(
    state: &mut LeaderAiRuntimeState,
    sequence: u64,
    command_id: &str,
    command: StorageCommand,
) {
    state
        .storage
        .execute(StorageCommandEnvelope {
            colony_id: "colony_one".to_owned(),
            command_id: command_id.to_owned(),
            fingerprint: format!("{command_id}_v1"),
            sequence,
            command,
        })
        .unwrap();
}

fn request(
    task_id: TaskId,
    resolved: ResolvedSpatialTask,
    identity: StorageIdentity,
    source: StorageAddress,
    endpoint: StorageAddress,
) -> PhysicalTaskExecutionRequest {
    PhysicalTaskExecutionRequest {
        task_id,
        resolved,
        cargo: PhysicalTaskCargoBinding {
            identity,
            resource_id: "food_raw_meat".to_owned(),
            quantity: 3,
            source,
            endpoint,
            recovery: None,
        },
        workers: vec![
            PhysicalTaskWorkerReport {
                cat_id: "refused".to_owned(),
                alive: true,
                capable: true,
                willing: false,
                suitability_score: 9_999,
            },
            PhysicalTaskWorkerReport {
                cat_id: "hunter".to_owned(),
                alive: true,
                capable: true,
                willing: true,
                suitability_score: 100,
            },
        ],
        priority: cat_sim::cat_willingness::TaskPriority::Required,
        interruption: PhysicalTaskInterruption::None,
        work: PhysicalTaskWorkReceipt {
            outcome: ProductiveOutcome::Productive {
                productive_minutes: 60,
                activity: Some(WorkActivity {
                    primary_skill_id: "hunting".to_owned(),
                    secondary_skill_ids: Vec::new(),
                    haul_legs: 0,
                }),
                office: None,
                supervised_by: None,
            },
            family_completion: None,
        },
    }
}

fn prepared_runtime() -> (
    LeaderAiRuntimeState,
    PhysicalTaskExecutionRequest,
    StorageIdentity,
    StorageAddress,
) {
    let mut state = LeaderAiRuntimeState::new_for_colony("colony_one").unwrap();
    state
        .reconcile_legacy_cats(19, "colony_one", &[base_cat()])
        .unwrap();

    let source = StorageAddress::LandCache {
        zone_id: "lair_cache".to_owned(),
        tile: TilePoint { x: 0, y: 0 },
        slot: 0,
    };
    let endpoint = StorageAddress::Loose {
        zone_id: "home_stockpile".to_owned(),
        tile: TilePoint { x: 10, y: 0 },
        slot: 0,
    };
    execute_storage(
        &mut state,
        1,
        "register_lair_cache",
        StorageCommand::RegisterZone {
            zone: StorageZone::new("lair_cache", StorageZoneKind::Cache, footprint(0, 0)).unwrap(),
        },
    );
    execute_storage(
        &mut state,
        2,
        "register_home_stockpile",
        StorageCommand::RegisterZone {
            zone: StorageZone::new(
                "home_stockpile",
                StorageZoneKind::Stockpile,
                footprint(10, 0),
            )
            .unwrap(),
        },
    );
    let lot_id = cat_sim::content_manifest::PhysicalLotId::new("lair_meat").unwrap();
    let identity = StorageIdentity::Lot(lot_id.clone());
    execute_storage(
        &mut state,
        3,
        "deposit_lair_meat",
        StorageCommand::DepositLot {
            lot: PhysicalLot {
                id: lot_id,
                key: BulkLotKey::new(
                    cat_sim::content_manifest::ContentId::new("food_raw_meat").unwrap(),
                    QualityBand::Common,
                ),
                provenance: LotProvenance {
                    origin: "hunt:cave_lair".to_owned(),
                    created_tick: 1,
                },
                quantity: 3,
                location: LotLocation::Cache("lair_cache".to_owned()),
                reservation: None,
            },
            compatibility: StorageCompatibility::Food,
            destination: source.clone(),
        },
    );

    let resolved = resolved_hunt();
    let intent_id = IntentId::derive("colony_one", 1, "hunt", "cave_lair", 0);
    let mut intent = Intent::proposed(
        intent_id.clone(),
        state.colony_partition.clone(),
        AuthorityActor::Scheduler,
        None,
        AuthorityDomain::Survival,
        id("kind", "hunt"),
        id("target", "cave_lair"),
        id("rationale", "food"),
        1,
    );
    intent.spatial_objective = Some(resolved.spatial.clone());
    assert!(matches!(
        state.intents.insert_or_merge(intent).unwrap(),
        IntentInsert::Inserted(_)
    ));
    let task = VisibleTaskRuntime::resolved(
        "colony_one",
        intent_id,
        0,
        TaskCategory::Hunt,
        resolved.spatial.clone(),
        Vec::new(),
        1,
    )
    .unwrap();
    let task_id = task.id.clone();
    state.scheduling.visible_tasks.insert(task_id.clone(), task);
    let request = request(
        task_id,
        resolved,
        identity.clone(),
        source,
        endpoint.clone(),
    );
    (state, request, identity, endpoint)
}

#[test]
fn exact_lair_task_reserves_moves_deposits_and_credits_once() {
    let (mut state, request, identity, endpoint) = prepared_runtime();

    assert_eq!(
        state.advance_physical_task(request.clone(), 1).unwrap(),
        PhysicalTaskExecutionOutcome::Activated {
            cat_id: "hunter".to_owned(),
        }
    );
    let task = &state.scheduling.visible_tasks[&request.task_id];
    assert_eq!(task.stage, TaskStage::TravelToSource);
    assert!(matches!(
        task.spatial.objective.as_ref(),
        Some(SiteRef::ResourceSource {
            source_id,
            resource_kind: ResourceSourceKind::Hunting,
            ..
        }) if source_id == "cave_lair"
    ));
    assert_eq!(state.scheduling.reservations.len(), 1);
    assert_eq!(state.scheduling.world_reservations.len(), 1);
    let StorageIdentity::Lot(lot_id) = &identity else {
        unreachable!("the fixture uses one bulk lot")
    };
    assert!(
        state
            .storage
            .ledger()
            .lot(lot_id)
            .unwrap()
            .reservation
            .is_some()
    );

    assert!(matches!(
        state.advance_physical_task(request.clone(), 2).unwrap(),
        PhysicalTaskExecutionOutcome::Advanced {
            stage: TaskStage::Pickup
        }
    ));
    assert!(matches!(
        state.advance_physical_task(request.clone(), 3).unwrap(),
        PhysicalTaskExecutionOutcome::Advanced {
            stage: TaskStage::TravelToWork
        }
    ));
    assert_eq!(
        state.storage.location(&identity),
        Some(&StorageAddress::RouteCargo {
            route_id: request.task_id.as_str().to_owned(),
        })
    );
    assert!(matches!(
        state.scheduling.visible_tasks[&request.task_id]
            .cargo
            .as_ref()
            .unwrap()
            .location,
        CargoLocation::Carried { .. }
    ));
    assert!(matches!(
        state.advance_physical_task(request.clone(), 4).unwrap(),
        PhysicalTaskExecutionOutcome::Advanced {
            stage: TaskStage::Work
        }
    ));
    assert_eq!(
        state.advance_physical_task(request.clone(), 5).unwrap(),
        PhysicalTaskExecutionOutcome::Worked {
            cat_id: "hunter".to_owned(),
        }
    );
    assert_eq!(state.task_outcomes.len(), 1);
    assert!(matches!(
        state.advance_physical_task(request.clone(), 6).unwrap(),
        PhysicalTaskExecutionOutcome::Advanced {
            stage: TaskStage::Deposit
        }
    ));
    assert_eq!(
        state.advance_physical_task(request.clone(), 7).unwrap(),
        PhysicalTaskExecutionOutcome::Completed
    );
    assert_eq!(
        state.scheduling.visible_tasks[&request.task_id].stage,
        TaskStage::Complete
    );
    assert_eq!(state.storage.location(&identity), Some(&endpoint));
    assert_eq!(state.scheduling.reservations.len(), 0);
    assert_eq!(state.scheduling.world_reservations.len(), 0);
    assert_eq!(state.task_outcomes.len(), 1);
    assert!(matches!(
        state.advance_physical_task(request, 8).unwrap(),
        PhysicalTaskExecutionOutcome::Terminal(TaskStage::Complete)
    ));
}

#[test]
fn refusal_preemption_and_route_loss_never_destroy_or_retarget_cargo() {
    let (mut refused, mut refusal_request, identity, _) = prepared_runtime();
    refusal_request.workers[1].willing = false;
    assert_eq!(
        refused
            .advance_physical_task(refusal_request.clone(), 1)
            .unwrap(),
        PhysicalTaskExecutionOutcome::Blocked(PhysicalTaskBlockReason::NoWillingWorker)
    );
    assert_eq!(
        refused.scheduling.visible_tasks[&refusal_request.task_id].stage,
        TaskStage::Blocked
    );
    assert_eq!(
        refused.storage.location(&identity),
        Some(&refusal_request.cargo.source)
    );
    assert!(refused.scheduling.reservations.is_empty());
    assert!(refused.scheduling.world_reservations.is_empty());

    let (mut state, mut request, identity, _) = prepared_runtime();
    assert!(matches!(
        state.advance_physical_task(request.clone(), 1).unwrap(),
        PhysicalTaskExecutionOutcome::Activated { .. }
    ));
    request.interruption = PhysicalTaskInterruption::SurvivalPreemption;
    assert_eq!(
        state.advance_physical_task(request.clone(), 2).unwrap(),
        PhysicalTaskExecutionOutcome::Preempted(PhysicalTaskInterruption::SurvivalPreemption)
    );
    assert_eq!(
        state.scheduling.visible_tasks[&request.task_id].stage,
        TaskStage::Cancelled
    );
    assert_eq!(
        state.storage.location(&identity),
        Some(&request.cargo.source)
    );
    assert!(state.scheduling.reservations.is_empty());
    assert!(state.scheduling.world_reservations.is_empty());

    let (mut carried, mut carried_request, carried_identity, _) = prepared_runtime();
    for tick in 1..=3 {
        carried
            .advance_physical_task(carried_request.clone(), tick)
            .unwrap();
    }
    carried_request.interruption = PhysicalTaskInterruption::RouteLost;
    assert_eq!(
        carried
            .advance_physical_task(carried_request.clone(), 4)
            .unwrap(),
        PhysicalTaskExecutionOutcome::Recovered { stranded: true }
    );
    let task = &carried.scheduling.visible_tasks[&carried_request.task_id];
    assert!(matches!(
        task.cargo.as_ref().unwrap().location,
        CargoLocation::Stranded { .. }
    ));
    assert_eq!(
        carried.storage.location(&carried_identity),
        Some(&StorageAddress::RouteCargo {
            route_id: carried_request.task_id.as_str().to_owned(),
        })
    );
    assert!(carried.scheduling.reservations.is_empty());
    assert!(carried.scheduling.world_reservations.is_empty());
}

#[test]
fn mismatched_water_and_workshop_geometry_are_typed_blocked_without_fallback_sites() {
    let (mut state, mut request, _, _) = prepared_runtime();
    request.resolved = resolved_water();
    assert_eq!(
        state.advance_physical_task(request.clone(), 1),
        Err(cat_sim::leader_ai_runtime::LeaderAiRuntimeError::PhysicalTaskSpatialMismatch)
    );
    assert_eq!(
        state.scheduling.visible_tasks[&request.task_id].stage,
        TaskStage::Resolve
    );

    let (mut state, mut request, _, _) = prepared_runtime();
    request.resolved = resolved_workshop();
    assert_eq!(
        state.advance_physical_task(request.clone(), 1),
        Err(cat_sim::leader_ai_runtime::LeaderAiRuntimeError::PhysicalTaskSpatialMismatch)
    );
    assert_eq!(
        state.scheduling.visible_tasks[&request.task_id].stage,
        TaskStage::Resolve
    );
}

#[test]
fn construction_category_is_blocked_before_cargo_pickup() {
    let (mut state, mut request, identity, _) = prepared_runtime();
    let construction = ResolvedSpatialTask {
        category: SpatialTaskCategory::Construction(BuildingType::Workshop),
        ..resolved_workshop()
    };
    state
        .scheduling
        .visible_tasks
        .get_mut(&request.task_id)
        .unwrap()
        .category = TaskCategory::BuildingConstruction;
    state
        .scheduling
        .visible_tasks
        .get_mut(&request.task_id)
        .unwrap()
        .spatial = construction.spatial.clone();
    request.resolved = construction;

    assert_eq!(
        state.advance_physical_task(request.clone(), 1).unwrap(),
        PhysicalTaskExecutionOutcome::Blocked(PhysicalTaskBlockReason::UnsupportedTaskStage)
    );
    assert_eq!(
        state.scheduling.visible_tasks[&request.task_id].stage,
        TaskStage::Blocked
    );
    assert_eq!(
        state.storage.location(&identity),
        Some(&request.cargo.source)
    );
    assert!(state.scheduling.reservations.is_empty());
    assert!(state.scheduling.world_reservations.is_empty());
}
