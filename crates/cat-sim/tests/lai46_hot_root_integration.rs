//! Static-focused LAI.46 spatial/reservation/runtime acceptance contract.

use cat_sim::{
    fishing::{DockOrientation, fishing_hut_footprint},
    planner_core::{IntentId, PlannerId},
    reservation_transaction::{
        ClaimMode, ClaimSpec, ReservationBundle, ReservationChecks, ReservationLedger,
    },
    spatial_resolver::{
        ResolvedSpatialTask, SpatialTaskCategory, validate_truthful_task_geometry,
    },
    spatial_tasks::{
        OrderedTiles, Rect, ResourceSourceKind, SiteMetadata, SiteRef, SpatialObjective,
        TaskFootprint, TilePoint, WorkSlot,
    },
    task_runtime::{
        CargoLocation, RuntimeBlockReason, TaskCategory, TaskStage, VisibleTaskRuntime,
    },
    types::BuildingType,
    world_reservations::{
        WorldReservationLedger, WorldReservationTransaction, WorldReservationValidation,
    },
    world_tick::{found_colony, new_world, world_tick},
};

fn id(namespace: &str, value: &str) -> PlannerId {
    PlannerId::derive(namespace, [value])
}

fn rect_site(name: &str, anchor: TilePoint, width: i32, height: i32) -> SiteRef {
    let rect = Rect::try_new(anchor, width, height).unwrap();
    SiteRef::Rect {
        metadata: SiteMetadata::revealed(name),
        rect,
        footprint: TaskFootprint::rectangular(rect),
    }
}

fn source(
    name: &str,
    kind: ResourceSourceKind,
    anchor: TilePoint,
    width: i32,
    height: i32,
) -> SiteRef {
    SiteRef::ResourceSource {
        metadata: SiteMetadata::revealed(name),
        source_id: name.to_owned(),
        resource_kind: kind,
        footprint: TaskFootprint::rectangular(
            Rect::try_new(anchor, width, height).unwrap(),
        ),
    }
}

fn tile(name: &str, point: TilePoint) -> SiteRef {
    SiteRef::Tile {
        metadata: SiteMetadata::revealed(name),
        tile: point,
    }
}

fn route(name: &str, points: Vec<TilePoint>) -> SiteRef {
    SiteRef::OrderedRoute {
        metadata: SiteMetadata::revealed(name),
        route: points,
    }
}

fn stockpile(name: &str, anchor: TilePoint) -> SiteRef {
    SiteRef::Stockpile {
        metadata: SiteMetadata::revealed(name),
        stockpile_id: name.to_owned(),
        footprint: TaskFootprint::rectangular(Rect::try_new(anchor, 2, 2).unwrap()),
    }
}

fn resolved(
    category: SpatialTaskCategory,
    objective: SiteRef,
    work: WorkSlot,
    endpoint: SiteRef,
    source_route: Vec<TilePoint>,
    delivery_route: Vec<TilePoint>,
) -> ResolvedSpatialTask {
    let source_route_id = format!(
        "exact-source-work-route:{}:{}:{}:{}",
        source_route.first().map_or(0, |tile| tile.x),
        source_route.first().map_or(0, |tile| tile.y),
        source_route.last().map_or(0, |tile| tile.x),
        source_route.last().map_or(0, |tile| tile.y),
    );
    let delivery_route_id = format!(
        "exact-work-delivery-route:{}:{}:{}:{}",
        delivery_route.first().map_or(0, |tile| tile.x),
        delivery_route.first().map_or(0, |tile| tile.y),
        delivery_route.last().map_or(0, |tile| tile.x),
        delivery_route.last().map_or(0, |tile| tile.y),
    );
    ResolvedSpatialTask {
        category,
        spatial: SpatialObjective::resolved(objective, vec![work], Some(endpoint)),
        source_to_work_route: route(&source_route_id, source_route),
        work_to_delivery_route: route(&delivery_route_id, delivery_route),
        source_units: 1,
        source_capacity: 1,
        delivery_units: 1,
        delivery_capacity: 1,
        source_to_work_route_capacity: 1,
        work_to_delivery_route_capacity: 1,
    }
}

#[test]
fn every_named_domain_has_complete_exact_geometry_and_no_marker_fallback() {
    let delivery = stockpile("stockpile-a", TilePoint { x: 20, y: 20 });
    let mut cases = Vec::new();

    let hole_landmark = rect_site("hole-landmark", TilePoint { x: 0, y: 0 }, 5, 5);
    let hole_work = rect_site("hole-work-area", TilePoint { x: 1, y: 1 }, 3, 3);
    cases.push(resolved(
        SpatialTaskCategory::HoleWork,
        hole_landmark,
        WorkSlot::exclusive("hole-work-slot", hole_work),
        tile("hole-delivery-edge", TilePoint { x: 1, y: 0 }),
        vec![TilePoint { x: 1, y: 1 }],
        vec![TilePoint { x: 1, y: 1 }, TilePoint { x: 1, y: 0 }],
    ));

    let workshop = SiteRef::building(
        "workshop-a",
        BuildingType::Workshop,
        TilePoint { x: 10, y: 10 },
    );
    let workshop_work = rect_site("workshop-work-area", TilePoint { x: 10, y: 10 }, 3, 3);
    cases.push(resolved(
        SpatialTaskCategory::WorkshopWork,
        workshop,
        WorkSlot::exclusive("workshop-work-slot", workshop_work),
        delivery.clone(),
        vec![TilePoint { x: 10, y: 10 }],
        vec![
            TilePoint { x: 10, y: 10 },
            TilePoint { x: 11, y: 10 },
            TilePoint { x: 12, y: 10 },
            TilePoint { x: 13, y: 10 },
            TilePoint { x: 14, y: 10 },
            TilePoint { x: 15, y: 10 },
            TilePoint { x: 16, y: 10 },
            TilePoint { x: 17, y: 10 },
            TilePoint { x: 18, y: 10 },
            TilePoint { x: 19, y: 10 },
            TilePoint { x: 20, y: 10 },
            TilePoint { x: 20, y: 11 },
            TilePoint { x: 20, y: 12 },
            TilePoint { x: 20, y: 13 },
            TilePoint { x: 20, y: 14 },
            TilePoint { x: 20, y: 15 },
            TilePoint { x: 20, y: 16 },
            TilePoint { x: 20, y: 17 },
            TilePoint { x: 20, y: 18 },
            TilePoint { x: 20, y: 19 },
            TilePoint { x: 20, y: 20 },
        ],
    ));

    let cookhouse = rect_site("cookhouse-a", TilePoint { x: 30, y: 30 }, 3, 3);
    let cookhouse_work = rect_site("cookhouse-work-area", TilePoint { x: 30, y: 30 }, 3, 3);
    cases.push(resolved(
        SpatialTaskCategory::CookhouseWork,
        cookhouse,
        WorkSlot::exclusive("cookhouse-work-slot", cookhouse_work),
        tile("cookhouse-output", TilePoint { x: 30, y: 30 }),
        vec![TilePoint { x: 30, y: 30 }],
        vec![TilePoint { x: 30, y: 30 }],
    ));

    let apple = source(
        "apple-tree-a",
        ResourceSourceKind::Tree,
        TilePoint { x: 39, y: 39 },
        3,
        3,
    );
    let apple_work = rect_site("apple-tree-work-area", TilePoint { x: 39, y: 39 }, 3, 3);
    cases.push(resolved(
        SpatialTaskCategory::AppleHarvest,
        apple,
        WorkSlot::exclusive("apple-tree-work-slot", apple_work),
        tile("apple-output", TilePoint { x: 39, y: 39 }),
        vec![TilePoint { x: 40, y: 40 }],
        vec![
            TilePoint { x: 40, y: 40 },
            TilePoint { x: 39, y: 40 },
            TilePoint { x: 39, y: 39 },
        ],
    ));

    for (category, kind, source_at, work_at) in [
        (
            SpatialTaskCategory::Hunt,
            ResourceSourceKind::Hunting,
            TilePoint { x: 50, y: 50 },
            TilePoint { x: 50, y: 51 },
        ),
        (
            SpatialTaskCategory::Quarry,
            ResourceSourceKind::Quarry,
            TilePoint { x: 60, y: 60 },
            TilePoint { x: 61, y: 60 },
        ),
        (
            SpatialTaskCategory::FetchWater,
            ResourceSourceKind::Water,
            TilePoint { x: 70, y: 70 },
            TilePoint { x: 70, y: 71 },
        ),
        (
            SpatialTaskCategory::Fish,
            ResourceSourceKind::FishHabitat,
            TilePoint { x: 80, y: 80 },
            TilePoint { x: 79, y: 80 },
        ),
    ] {
        cases.push(resolved(
            category,
            source(
                &format!("{kind:?}-source"),
                kind,
                source_at,
                1,
                1,
            ),
            WorkSlot::exclusive(
                format!("{kind:?}-work-slot"),
                tile(&format!("{kind:?}-bank"), work_at),
            ),
            tile(&format!("{kind:?}-delivery"), work_at),
            vec![source_at, work_at],
            vec![work_at],
        ));
    }

    let field = SiteRef::building(
        "farm-plot-a",
        BuildingType::Field,
        TilePoint { x: 90, y: 90 },
    );
    let field_work = rect_site("farm-plot-work-area", TilePoint { x: 90, y: 90 }, 2, 3);
    cases.push(resolved(
        SpatialTaskCategory::FarmWork,
        field,
        WorkSlot::exclusive("farm-plot-work-slot", field_work),
        tile("farm-output", TilePoint { x: 90, y: 90 }),
        vec![TilePoint { x: 90, y: 90 }],
        vec![TilePoint { x: 90, y: 90 }],
    ));

    let construction = SiteRef::building(
        "construction-workshop-a",
        BuildingType::Workshop,
        TilePoint { x: 100, y: 100 },
    );
    let construction_work =
        rect_site("construction-work-area", TilePoint { x: 100, y: 100 }, 3, 3);
    cases.push(resolved(
        SpatialTaskCategory::Construction(BuildingType::Workshop),
        construction,
        WorkSlot::exclusive("construction-work-slot", construction_work),
        tile("construction-input", TilePoint { x: 100, y: 100 }),
        vec![TilePoint { x: 100, y: 100 }],
        vec![TilePoint { x: 100, y: 100 }],
    ));

    for exact in cases {
        validate_truthful_task_geometry(&exact).unwrap();
        let encoded = serde_json::to_string(&exact).unwrap().to_ascii_lowercase();
        for forbidden in ["generic", "fallback", "center", "reported_work"] {
            assert!(!encoded.contains(forbidden));
        }
    }
}

#[test]
fn fishing_hut_all_orientations_include_full_land_dock_shore_and_water() {
    for orientation in [
        DockOrientation::North,
        DockOrientation::East,
        DockOrientation::South,
        DockOrientation::West,
    ] {
        let footprint =
            fishing_hut_footprint(TilePoint { x: 5, y: 5 }, orientation).unwrap();
        let mut cells = footprint.land.tiles.as_slice().to_vec();
        cells.push(footprint.reserved_water);
        let objective = SiteRef::OrderedTiles {
            metadata: SiteMetadata::revealed(format!("fishing-hut-{orientation:?}")),
            tiles: OrderedTiles::canonical(cells),
        };
        let exact = resolved(
            SpatialTaskCategory::FishingHutWork,
            objective,
            WorkSlot::exclusive(
                format!("fishing-hut-dock-{orientation:?}"),
                tile("fishing-shore-work", footprint.dock_land),
            ),
            tile("fishing-water-attachment", footprint.reserved_water),
            vec![footprint.dock_land],
            vec![footprint.dock_land, footprint.reserved_water],
        );
        validate_truthful_task_geometry(&exact).unwrap();
    }
}

fn world_transaction(
    colony: &str,
    task: &str,
    objective_x: i32,
) -> WorldReservationTransaction {
    let objective_at = TilePoint {
        x: objective_x,
        y: 0,
    };
    let work_at = TilePoint {
        x: objective_x + 1,
        y: 0,
    };
    let exact = resolved(
        SpatialTaskCategory::Quarry,
        source(
            "shared-local-source-id",
            ResourceSourceKind::Quarry,
            objective_at,
            1,
            1,
        ),
        WorkSlot::exclusive(
            "shared-local-work-id",
            tile("shared-local-work-site", work_at),
        ),
        tile("shared-local-delivery-id", work_at),
        vec![objective_at, work_at],
        vec![work_at],
    );
    let colony_id = id("colony", colony);
    let intent_id = IntentId::derive(colony, 1, "quarry", task, 0);
    WorldReservationTransaction::new(
        colony_id,
        id("task", task),
        intent_id,
        exact,
        id("worker", "same-local-cat-id"),
        vec![id("tool", "same-local-tool-id")],
        Vec::new(),
    )
    .unwrap()
}

#[test]
fn one_world_ledger_conflicts_on_geometry_but_isolates_colony_local_ids() {
    let overlapping_a = world_transaction("a", "a", 0);
    let overlapping_b = world_transaction("b", "b", 0);
    let mut overlap = WorldReservationLedger::new();
    let results = overlap.commit_batch(vec![
        (
            overlapping_b.clone(),
            WorldReservationValidation::all_valid(),
        ),
        (
            overlapping_a.clone(),
            WorldReservationValidation::all_valid(),
        ),
    ]);
    assert_eq!(overlap.len(), 1);
    assert_eq!(results.len(), 2);

    let isolated_a = world_transaction("a", "isolated-a", 10);
    let isolated_b = world_transaction("b", "isolated-b", 20);
    let mut isolated = WorldReservationLedger::new();
    isolated
        .try_commit(isolated_a, WorldReservationValidation::all_valid())
        .unwrap();
    isolated
        .try_commit(isolated_b, WorldReservationValidation::all_valid())
        .unwrap();
    assert_eq!(isolated.len(), 2);
}

#[test]
fn shuffled_and_restart_world_reservation_twins_are_byte_equal() {
    let a = world_transaction("a", "a", 0);
    let b = world_transaction("b", "b", 0);
    let mut forward = WorldReservationLedger::new();
    forward.commit_batch(vec![
        (a.clone(), WorldReservationValidation::all_valid()),
        (b.clone(), WorldReservationValidation::all_valid()),
    ]);
    let mut reverse = WorldReservationLedger::new();
    reverse.commit_batch(vec![
        (b, WorldReservationValidation::all_valid()),
        (a, WorldReservationValidation::all_valid()),
    ]);
    assert_eq!(forward, reverse);

    let (restarted_forward, _) =
        WorldReservationLedger::reconcile_persisted_mirrors([&forward, &reverse]);
    let (restarted_reverse, _) =
        WorldReservationLedger::reconcile_persisted_mirrors([&reverse, &forward]);
    assert_eq!(restarted_forward, restarted_reverse);
    assert_eq!(
        serde_json::to_vec(&restarted_forward).unwrap(),
        serde_json::to_vec(&restarted_reverse).unwrap()
    );
}

#[test]
fn terminal_tasks_hide_geometry_and_recovery_preserves_exact_cargo_identity() {
    let objective_at = TilePoint { x: 0, y: 0 };
    let work_at = TilePoint { x: 1, y: 0 };
    let exact = resolved(
        SpatialTaskCategory::Quarry,
        source(
            "quarry-source-a",
            ResourceSourceKind::Quarry,
            objective_at,
            1,
            1,
        ),
        WorkSlot::exclusive("quarry-work-slot", tile("quarry-work-site", work_at)),
        stockpile("recovery-stockpile", work_at),
        vec![objective_at, work_at],
        vec![work_at],
    );
    let colony = "colony-a";
    let intent = IntentId::derive(colony, 1, "quarry", "quarry-source-a", 0);
    let mut task = VisibleTaskRuntime::resolved(
        colony,
        intent.clone(),
        0,
        TaskCategory::Quarry,
        exact.spatial.clone(),
        vec![
            exact.source_to_work_route.stable_id().to_owned(),
            exact.work_to_delivery_route.stable_id().to_owned(),
        ],
        1,
    )
    .unwrap();
    let mut reservations = ReservationLedger::new();
    let local = ReservationBundle::from_spatial_objective(
        id("colony", colony),
        id("task", task.id.as_str()),
        intent,
        &exact.spatial,
        0,
        ClaimMode::Capacity {
            units: 1,
            capacity: 1,
        },
        ClaimMode::Capacity {
            units: 1,
            capacity: 1,
        },
        ClaimSpec::capacity(id("route", "quarry"), 1, 1),
        Vec::new(),
        Vec::new(),
        id("cat", "worker-a"),
    )
    .unwrap();
    let local_id = local.id.clone();
    reservations
        .try_commit(local, ReservationChecks::all_valid())
        .unwrap();
    task.begin_reservation(2).unwrap();
    task.reserve_cargo_at_source("lot:stone-a", "resource_stone", 1)
        .unwrap();
    task.activate(
        &reservations,
        local_id,
        [("worker-a".to_owned(), "quarry-work-slot".to_owned())],
        2,
    )
    .unwrap();
    task.advance(TaskStage::Pickup, 3).unwrap();
    task.pickup("worker-a", 3).unwrap();
    let cargo_before = task.cargo.clone().unwrap();
    task.recover_after_pickup(
        RuntimeBlockReason::RouteClosedWithCargo,
        exact.spatial.delivery_endpoint.as_ref(),
        "quarry-work-site",
        &mut reservations,
        4,
    )
    .unwrap();
    let cargo_after = task.cargo.clone().unwrap();
    assert_eq!(cargo_after.cargo_id, cargo_before.cargo_id);
    assert_eq!(cargo_after.resource_id, cargo_before.resource_id);
    assert_eq!(cargo_after.quantity, cargo_before.quantity);
    assert!(matches!(
        cargo_after.location,
        CargoLocation::SalvagedAtStockpile { .. }
    ));
    assert!(!task.emits_world_marker());
    assert!(reservations.is_empty());
}

#[test]
fn canonical_world_runtime_executes_at_most_once_for_the_same_tick() {
    let mut world = new_world(7);
    world.colonies.push(found_colony(7, "colony-a", 0, 1));
    let _ = world_tick(&mut world, 1_000);
    let before = world.colonies[0]
        .leader_ai_runtime
        .phase_receipts
        .clone();
    let _ = world_tick(&mut world, 1_000);
    assert_eq!(
        world.colonies[0].leader_ai_runtime.phase_receipts,
        before
    );
}

#[test]
fn canonical_world_runtime_partition_and_restart_twins_match() {
    let mut direct = new_world(11);
    direct.colonies.push(found_colony(11, "colony-a", 0, 3));
    let _ = world_tick(&mut direct, 1_000);
    let mut partitioned = direct.clone();
    let _ = world_tick(&mut direct, 61_000);
    for now_ms in (2_000..=61_000).step_by(1_000) {
        let _ = world_tick(&mut partitioned, now_ms);
    }
    assert_eq!(
        direct.colonies[0].leader_ai_runtime,
        partitioned.colonies[0].leader_ai_runtime
    );

    let restarted = direct.clone();
    assert_eq!(
        serde_json::to_vec(&restarted.colonies[0].leader_ai_runtime).unwrap(),
        serde_json::to_vec(&direct.colonies[0].leader_ai_runtime).unwrap()
    );
}
