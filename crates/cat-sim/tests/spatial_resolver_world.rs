use cat_sim::{
    planner_core::{IntentId, PlannerId},
    reservation_transaction::ClaimMode,
    spatial_resolver::{
        ResolvedSpatialTask, SpatialResolutionCandidate, SpatialResolutionOutcome,
        SpatialResolutionRequest, SpatialTaskCategory, resolve_spatial_task,
    },
    spatial_tasks::{
        Rect, ResourceSourceKind, SiteLifecycleStage, SiteMetadata, SiteRef, SiteVisibility,
        SpatialBlockReason, TaskFootprint, TilePoint, WorkSlot, footprint_for,
    },
    types::BuildingType,
    world_reservations::{
        CapacityReservation, WorldClaimKind, WorldCommitOutcome, WorldReleaseOutcome,
        WorldReservationError, WorldReservationLedger, WorldReservationTransaction,
        WorldReservationValidation, WorldRevalidationOutcome,
    },
};

fn id(namespace: &str, value: &str) -> PlannerId {
    PlannerId::derive(namespace, [value])
}

fn footprint(anchor: TilePoint, width: i32, height: i32) -> TaskFootprint {
    TaskFootprint::rectangular(Rect::new(anchor, width, height).unwrap())
}

fn resource(
    stable_id: &str,
    kind: ResourceSourceKind,
    anchor: TilePoint,
    width: i32,
    height: i32,
) -> SiteRef {
    SiteRef::ResourceSource {
        metadata: SiteMetadata::revealed(stable_id),
        source_id: stable_id.to_owned(),
        resource_kind: kind,
        footprint: footprint(anchor, width, height),
    }
}

fn tile(stable_id: &str, point: TilePoint) -> SiteRef {
    SiteRef::Tile {
        metadata: SiteMetadata::revealed(stable_id),
        tile: point,
    }
}

fn route(stable_id: &str, start: TilePoint) -> SiteRef {
    SiteRef::OrderedRoute {
        metadata: SiteMetadata::revealed(stable_id),
        route: vec![
            start,
            TilePoint {
                x: start.x + 1,
                y: start.y,
            },
        ],
    }
}

fn endpoint(stable_id: &str) -> SiteRef {
    SiteRef::Stockpile {
        metadata: SiteMetadata::revealed(stable_id),
        stockpile_id: stable_id.to_owned(),
        footprint: footprint(TilePoint { x: 100, y: 100 }, 1, 1),
    }
}

fn candidate(
    objective: SiteRef,
    slot_id: &str,
    work_point: TilePoint,
    source_capacity: u32,
) -> SpatialResolutionCandidate {
    SpatialResolutionCandidate {
        objective,
        work_slot: WorkSlot::exclusive(slot_id, tile(&format!("{slot_id}-site"), work_point)),
        source_to_work_route: route(&format!("{slot_id}-source-route"), work_point),
        work_to_delivery_route: route(&format!("{slot_id}-delivery-route"), work_point),
        objective_exists: true,
        work_position_available: true,
        source_available_units: source_capacity,
        source_capacity,
        source_to_work_route_capacity: 16,
        work_to_delivery_route_capacity: 16,
    }
}

fn request(
    category: SpatialTaskCategory,
    candidates: Vec<SpatialResolutionCandidate>,
) -> SpatialResolutionRequest {
    SpatialResolutionRequest {
        category,
        pinned_objective_id: None,
        pinned_delivery_endpoint: endpoint("stockpile-pinned"),
        delivery_endpoint_exists: true,
        requested_source_units: 1,
        requested_delivery_units: 1,
        delivery_capacity: 32,
        candidates,
    }
}

fn resolved(request: SpatialResolutionRequest) -> ResolvedSpatialTask {
    match resolve_spatial_task(request) {
        SpatialResolutionOutcome::Resolved(resolved) => *resolved,
        SpatialResolutionOutcome::Blocked(blocked) => {
            panic!("expected resolved spatial task, got {blocked:?}")
        }
    }
}

fn transaction(
    resolved: ResolvedSpatialTask,
    colony: &str,
    task: &str,
    worker: &str,
) -> WorldReservationTransaction {
    let colony_id = id("colony", colony);
    let task_id = id("task", task);
    let intent_id = IntentId::derive(colony, 1, "spatial", task, 0);
    WorldReservationTransaction::new(
        colony_id,
        task_id,
        intent_id,
        resolved,
        id("cat", worker),
        Vec::new(),
        Vec::new(),
    )
    .unwrap()
}

fn metadata_mut(site: &mut SiteRef) -> &mut SiteMetadata {
    match site {
        SiteRef::Tile { metadata, .. }
        | SiteRef::Rect { metadata, .. }
        | SiteRef::OrderedTiles { metadata, .. }
        | SiteRef::Building { metadata, .. }
        | SiteRef::Stockpile { metadata, .. }
        | SiteRef::ResourceSource { metadata, .. }
        | SiteRef::OrderedRoute { metadata, .. }
        | SiteRef::Shrine { metadata, .. }
        | SiteRef::VillageTradeEndpoint { metadata, .. } => metadata,
    }
}

#[test]
fn every_required_category_maps_to_authoritative_roles() {
    let categories = vec![
        (
            SpatialTaskCategory::Hunt,
            resource(
                "cave-hunt",
                ResourceSourceKind::Hunting,
                TilePoint { x: 0, y: 0 },
                1,
                1,
            ),
        ),
        (
            SpatialTaskCategory::FetchWater,
            resource(
                "spring",
                ResourceSourceKind::Water,
                TilePoint { x: 0, y: 0 },
                1,
                1,
            ),
        ),
        (
            SpatialTaskCategory::Fish,
            resource(
                "fish-habitat",
                ResourceSourceKind::FishHabitat,
                TilePoint { x: 0, y: 0 },
                2,
                2,
            ),
        ),
        (
            SpatialTaskCategory::Quarry,
            resource(
                "quarry-face",
                ResourceSourceKind::Quarry,
                TilePoint { x: 0, y: 0 },
                2,
                2,
            ),
        ),
        (
            SpatialTaskCategory::Logging,
            resource(
                "tree",
                ResourceSourceKind::Tree,
                TilePoint { x: 0, y: 0 },
                2,
                3,
            ),
        ),
        (
            SpatialTaskCategory::Construction(BuildingType::Den),
            SiteRef::building("planned-den", BuildingType::Den, TilePoint { x: 0, y: 0 }),
        ),
        (
            SpatialTaskCategory::RoadConstruction,
            SiteRef::OrderedRoute {
                metadata: SiteMetadata {
                    stable_id: "road-route".into(),
                    lifecycle: SiteLifecycleStage::Planned,
                    visibility: SiteVisibility::Revealed,
                    blocked_reason: None,
                },
                route: vec![TilePoint { x: 0, y: 0 }, TilePoint { x: 1, y: 0 }],
            },
        ),
        (
            SpatialTaskCategory::StationWork(BuildingType::Smithy),
            SiteRef::building("smithy", BuildingType::Smithy, TilePoint { x: 0, y: 0 }),
        ),
        (
            SpatialTaskCategory::WorkshopWork,
            SiteRef::building("workshop", BuildingType::Workshop, TilePoint { x: 0, y: 0 }),
        ),
        (
            SpatialTaskCategory::FarmWork,
            SiteRef::building("field", BuildingType::Field, TilePoint { x: 0, y: 0 }),
        ),
    ];

    for (index, (category, objective)) in categories.into_iter().enumerate() {
        let slot = format!("slot-{index}");
        let result = resolved(request(
            category,
            vec![candidate(objective, &slot, TilePoint { x: 10, y: 10 }, 8)],
        ));
        assert_ne!(
            result.objective().stable_id(),
            result.work_slot().site.stable_id()
        );
        assert_ne!(
            result.work_slot().site.stable_id(),
            result.delivery_endpoint().stable_id()
        );
        assert_eq!(result.delivery_endpoint().stable_id(), "stockpile-pinned");
        result.validate().unwrap();
    }
}

#[test]
fn hunt_uses_revealed_reachable_source_and_never_falls_back() {
    let source_b = resource(
        "hunt-b",
        ResourceSourceKind::Hunting,
        TilePoint { x: 0, y: 0 },
        1,
        1,
    );
    let source_a = resource(
        "hunt-a",
        ResourceSourceKind::Hunting,
        TilePoint { x: 2, y: 0 },
        1,
        1,
    );
    let initial = resolved(request(
        SpatialTaskCategory::Hunt,
        vec![
            candidate(source_b, "entrance-b", TilePoint { x: 1, y: 0 }, 4),
            candidate(source_a, "entrance-a", TilePoint { x: 3, y: 0 }, 4),
        ],
    ));
    assert_eq!(initial.objective().stable_id(), "hunt-a");

    let mut pinned_missing = request(
        SpatialTaskCategory::Hunt,
        vec![candidate(
            resource(
                "hunt-other",
                ResourceSourceKind::Hunting,
                TilePoint { x: 0, y: 0 },
                1,
                1,
            ),
            "entrance",
            TilePoint { x: 1, y: 0 },
            4,
        )],
    );
    pinned_missing.pinned_objective_id = Some("removed-hunt-a".into());
    let SpatialResolutionOutcome::Blocked(blocked) = resolve_spatial_task(pinned_missing) else {
        panic!("missing pinned Hunt source must block");
    };
    assert_eq!(
        blocked.blocked_reason,
        Some(SpatialBlockReason::SourceUnavailable)
    );
    assert!(blocked.objective.is_none());
    assert!(blocked.work_positions.is_empty());
    assert!(blocked.delivery_endpoint.is_none());

    let no_source = resolve_spatial_task(request(SpatialTaskCategory::Hunt, Vec::new()));
    assert_eq!(
        no_source.blocked_reason(),
        Some(SpatialBlockReason::SourceUnavailable)
    );
}

#[test]
fn hidden_source_missing_route_and_no_water_bank_block_without_markers() {
    let mut hidden = candidate(
        resource(
            "hidden-cave",
            ResourceSourceKind::Hunting,
            TilePoint { x: 0, y: 0 },
            1,
            1,
        ),
        "hidden-entrance",
        TilePoint { x: 1, y: 0 },
        4,
    );
    metadata_mut(&mut hidden.objective).visibility = SiteVisibility::Hidden;
    assert_eq!(
        resolve_spatial_task(request(SpatialTaskCategory::Hunt, vec![hidden])).blocked_reason(),
        Some(SpatialBlockReason::UnrevealedObjective)
    );

    let mut no_route = candidate(
        resource(
            "cave",
            ResourceSourceKind::Hunting,
            TilePoint { x: 0, y: 0 },
            1,
            1,
        ),
        "entrance",
        TilePoint { x: 1, y: 0 },
        4,
    );
    no_route.source_to_work_route = SiteRef::OrderedRoute {
        metadata: SiteMetadata::revealed("missing-route"),
        route: Vec::new(),
    };
    assert_eq!(
        resolve_spatial_task(request(SpatialTaskCategory::Hunt, vec![no_route])).blocked_reason(),
        Some(SpatialBlockReason::RouteUnavailable)
    );

    let water = resource(
        "water-source",
        ResourceSourceKind::Water,
        TilePoint { x: 0, y: 0 },
        1,
        1,
    );
    let no_bank = candidate(water, "water-bank", TilePoint { x: 0, y: 0 }, 8);
    let outcome = resolve_spatial_task(request(SpatialTaskCategory::FetchWater, vec![no_bank]));
    assert_eq!(
        outcome.blocked_reason(),
        Some(SpatialBlockReason::WorkPositionUnavailable)
    );
    let SpatialResolutionOutcome::Blocked(spatial) = outcome else {
        unreachable!()
    };
    assert!(spatial.objective.is_none());
}

#[test]
fn water_keeps_source_dry_bank_and_pinned_endpoint_distinct() {
    let water = resource(
        "spring-7",
        ResourceSourceKind::Water,
        TilePoint { x: 0, y: 0 },
        1,
        1,
    );
    let mut input = request(
        SpatialTaskCategory::FetchWater,
        vec![candidate(water, "dry-bank-7", TilePoint { x: 1, y: 0 }, 8)],
    );
    input.pinned_delivery_endpoint = endpoint("water-bowl-4");
    let result = resolved(input);
    assert_eq!(result.objective().stable_id(), "spring-7");
    assert_eq!(result.work_slot().stable_id, "dry-bank-7");
    assert_eq!(result.delivery_endpoint().stable_id(), "water-bowl-4");
}

#[test]
fn logging_and_workshop_use_complete_canonical_footprints() {
    let logging = resolved(request(
        SpatialTaskCategory::Logging,
        vec![candidate(
            resource(
                "tree-six",
                ResourceSourceKind::Tree,
                TilePoint { x: 4, y: 5 },
                2,
                3,
            ),
            "tree-perimeter",
            TilePoint { x: 3, y: 5 },
            1,
        )],
    ));
    let tree = logging.objective().footprint().unwrap();
    assert_eq!((tree.width, tree.height, tree.tiles.len()), (2, 3, 6));

    let workshop = resolved(request(
        SpatialTaskCategory::WorkshopWork,
        vec![candidate(
            SiteRef::building(
                "workshop-nine",
                BuildingType::Workshop,
                TilePoint { x: 10, y: 20 },
            ),
            "workshop-slot",
            TilePoint { x: 9, y: 20 },
            4,
        )],
    ));
    let workshop_footprint = workshop.objective().footprint().unwrap();
    let (width, height) = footprint_for(BuildingType::Workshop);
    assert_eq!(workshop_footprint.width, width);
    assert_eq!(workshop_footprint.height, height);
    assert_eq!(workshop_footprint.tiles.len(), (width * height) as usize);
    assert_eq!(
        workshop_footprint.tiles,
        workshop_footprint.rect().ordered_tiles()
    );
}

fn fish_resolved(slot: &str, work_x: i32, units: u32, capacity: u32) -> ResolvedSpatialTask {
    let mut input = request(
        SpatialTaskCategory::Fish,
        vec![candidate(
            resource(
                "habitat-shared",
                ResourceSourceKind::FishHabitat,
                TilePoint { x: 0, y: 0 },
                2,
                2,
            ),
            slot,
            TilePoint { x: work_x, y: 10 },
            capacity,
        )],
    );
    input.requested_source_units = units;
    input.delivery_capacity = u32::MAX;
    input.requested_delivery_units = units;
    resolved(input)
}

#[test]
fn fish_capacity_is_keyed_by_habitat_not_shore_and_sums_worldwide() {
    let first = transaction(fish_resolved("shore-a", 10, 1, 2), "a", "fish-a", "cat-a");
    let habitat_key = PlannerId::derive("world_spatial_claim", ["habitat-shared"]);
    let shore_key = PlannerId::derive("world_spatial_claim", ["shore-a-site"]);
    let objective_claim = first
        .claims()
        .iter()
        .find(|claim| claim.key.kind == WorldClaimKind::Objective)
        .unwrap();
    assert_eq!(objective_claim.key.stable_id, habitat_key);
    assert_ne!(objective_claim.key.stable_id, shore_key);

    let second = transaction(fish_resolved("shore-b", 20, 1, 2), "b", "fish-b", "cat-b");
    let third = transaction(fish_resolved("shore-c", 30, 1, 2), "c", "fish-c", "cat-c");
    let mut ledger = WorldReservationLedger::new();
    assert_eq!(
        ledger
            .try_commit(first, WorldReservationValidation::all_valid())
            .unwrap(),
        WorldCommitOutcome::Committed
    );
    ledger
        .try_commit(second, WorldReservationValidation::all_valid())
        .unwrap();
    let before = ledger.clone();
    assert!(matches!(
        ledger.try_commit(third, WorldReservationValidation::all_valid()),
        Err(WorldReservationError::Conflict(_))
    ));
    assert_eq!(ledger, before);
}

#[test]
fn capacity_math_rejects_large_sum_without_overflow_or_partial_commit() {
    let first = transaction(
        fish_resolved("max-shore-a", 10, u32::MAX, u32::MAX),
        "a",
        "max-a",
        "cat-a",
    );
    let second = transaction(
        fish_resolved("max-shore-b", 20, u32::MAX, u32::MAX),
        "b",
        "max-b",
        "cat-b",
    );
    let mut ledger = WorldReservationLedger::new();
    ledger
        .try_commit(first, WorldReservationValidation::all_valid())
        .unwrap();
    let before = ledger.clone();
    assert!(matches!(
        ledger.try_commit(second, WorldReservationValidation::all_valid()),
        Err(WorldReservationError::Conflict(_))
    ));
    assert_eq!(ledger, before);
}

#[test]
fn overlapping_exclusive_footprints_conflict_across_colonies() {
    let first_tree = resource(
        "tree-a",
        ResourceSourceKind::Tree,
        TilePoint { x: 5, y: 5 },
        2,
        3,
    );
    let second_tree = resource(
        "tree-b",
        ResourceSourceKind::Tree,
        TilePoint { x: 5, y: 5 },
        2,
        3,
    );
    let first = transaction(
        resolved(request(
            SpatialTaskCategory::Logging,
            vec![candidate(
                first_tree,
                "tree-slot-a",
                TilePoint { x: 4, y: 5 },
                1,
            )],
        )),
        "colony-a",
        "logging-a",
        "logger-a",
    );
    let second = transaction(
        resolved(request(
            SpatialTaskCategory::Logging,
            vec![candidate(
                second_tree,
                "tree-slot-b",
                TilePoint { x: 7, y: 5 },
                1,
            )],
        )),
        "colony-b",
        "logging-b",
        "logger-b",
    );
    let mut ledger = WorldReservationLedger::new();
    ledger
        .try_commit(first, WorldReservationValidation::all_valid())
        .unwrap();
    assert!(matches!(
        ledger.try_commit(second, WorldReservationValidation::all_valid()),
        Err(WorldReservationError::Conflict(key)) if key.kind == WorldClaimKind::ObjectiveTile
    ));
    assert_eq!(ledger.len(), 1);
}

#[test]
fn validation_commit_release_and_source_removal_are_atomic() {
    let tx = transaction(fish_resolved("shore", 10, 1, 3), "a", "fish", "worker");
    let reservation_id = tx.id.clone();
    let worker_id = tx.worker_id.clone();
    let mut ledger = WorldReservationLedger::new();
    let denied = WorldReservationValidation {
        work_to_delivery_route_valid: false,
        ..WorldReservationValidation::all_valid()
    };
    assert_eq!(
        ledger.try_commit(tx.clone(), denied),
        Err(WorldReservationError::Blocked(
            SpatialBlockReason::RouteUnavailable
        ))
    );
    assert!(ledger.is_empty());
    assert!(!ledger.worker_is_reserved(&worker_id));

    ledger
        .try_commit(tx, WorldReservationValidation::all_valid())
        .unwrap();
    let version = ledger.version();
    assert_eq!(
        ledger
            .revalidate(
                &reservation_id,
                WorldReservationValidation {
                    objective_exists: false,
                    ..WorldReservationValidation::all_valid()
                }
            )
            .unwrap(),
        WorldRevalidationOutcome::Released(SpatialBlockReason::SourceUnavailable)
    );
    assert_eq!(ledger.version(), version + 1);
    assert!(!ledger.worker_is_reserved(&worker_id));
    let version = ledger.version();
    assert_eq!(
        ledger.release(&reservation_id).unwrap(),
        WorldReleaseOutcome::NotFound
    );
    assert_eq!(ledger.version(), version);
}

#[test]
fn batch_resolution_is_site_task_colony_order_stable() {
    let objective = resource(
        "one-tree",
        ResourceSourceKind::Tree,
        TilePoint { x: 5, y: 5 },
        2,
        3,
    );
    let first = transaction(
        resolved(request(
            SpatialTaskCategory::Logging,
            vec![candidate(
                objective.clone(),
                "slot-a",
                TilePoint { x: 4, y: 5 },
                1,
            )],
        )),
        "colony-z",
        "task-a",
        "cat-a",
    );
    let second = transaction(
        resolved(request(
            SpatialTaskCategory::Logging,
            vec![candidate(objective, "slot-b", TilePoint { x: 7, y: 5 }, 1)],
        )),
        "colony-a",
        "task-b",
        "cat-b",
    );
    let winner = first.id.clone();
    let wave = vec![
        (first, WorldReservationValidation::all_valid()),
        (second, WorldReservationValidation::all_valid()),
    ];
    let mut forward = WorldReservationLedger::new();
    let mut reversed = WorldReservationLedger::new();
    let forward_results = forward.commit_batch(wave.clone());
    let reverse_results = reversed.commit_batch(wave.into_iter().rev().collect());
    assert_eq!(forward_results, reverse_results);
    assert!(forward.contains(&winner));
    assert_eq!(
        serde_json::to_string(&forward).unwrap(),
        serde_json::to_string(&reversed).unwrap()
    );
}

#[test]
fn tools_resources_and_worker_commit_together_or_not_at_all() {
    let base = fish_resolved("shore-tools", 10, 1, 4);
    let colony_id = id("colony", "tool-colony");
    let task_id = id("task", "tool-task");
    let intent_id = IntentId::derive("tool-colony", 1, "fish", "tool-task", 0);
    let tx = WorldReservationTransaction::new(
        colony_id,
        task_id,
        intent_id,
        base,
        id("cat", "fisher"),
        vec![id("tool", "net")],
        vec![CapacityReservation {
            stable_id: id("cargo", "fish"),
            units: 1,
            capacity: 4,
        }],
    )
    .unwrap();
    assert!(
        tx.claims()
            .iter()
            .any(|claim| claim.key.kind == WorldClaimKind::Tool)
    );
    assert!(
        tx.claims()
            .iter()
            .any(|claim| claim.key.kind == WorldClaimKind::CargoResource)
    );
    let mut ledger = WorldReservationLedger::new();
    let denied = WorldReservationValidation {
        tools_available: false,
        ..WorldReservationValidation::all_valid()
    };
    assert!(ledger.try_commit(tx, denied).is_err());
    assert_eq!(ledger.claim_count(), 0);
}

#[test]
fn persistence_is_strict_ordered_and_restart_safe_with_empty_defaults() {
    let tx = transaction(
        fish_resolved("restart-shore", 10, 1, 4),
        "restart",
        "fish",
        "cat",
    );
    let mut ledger = WorldReservationLedger::new();
    ledger
        .try_commit(tx, WorldReservationValidation::all_valid())
        .unwrap();
    let json = serde_json::to_string(&ledger).unwrap();
    let restored: WorldReservationLedger = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, ledger);

    let empty: WorldReservationLedger = serde_json::from_str(r#"{"schemaVersion":1}"#).unwrap();
    assert_eq!(empty, WorldReservationLedger::new());

    let valid = serde_json::to_value(&ledger).unwrap();
    let mut unknown = valid.clone();
    unknown["schemaVersion"] = 99.into();
    assert!(serde_json::from_value::<WorldReservationLedger>(unknown).is_err());

    let mut duplicate = valid.clone();
    let reservation = duplicate["reservations"][0].clone();
    duplicate["reservations"]
        .as_array_mut()
        .unwrap()
        .push(reservation);
    assert!(serde_json::from_value::<WorldReservationLedger>(duplicate).is_err());

    let mut hidden = valid.clone();
    hidden["reservations"][0]["resolved"]["spatial"]["objective"]["metadata"]["visibility"] =
        "hidden".into();
    assert!(serde_json::from_value::<WorldReservationLedger>(hidden).is_err());

    let mut unordered = valid;
    unordered["reservations"][0]["claims"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    assert!(serde_json::from_value::<WorldReservationLedger>(unordered).is_err());
}

#[test]
fn claim_modes_remain_integer_and_explicit() {
    let tx = transaction(fish_resolved("shore-mode", 10, 1, 4), "mode", "fish", "cat");
    assert!(tx.claims().iter().any(|claim| {
        claim.key.kind == WorldClaimKind::Objective
            && claim.mode
                == ClaimMode::Capacity {
                    units: 1,
                    capacity: 4,
                }
    }));
}
