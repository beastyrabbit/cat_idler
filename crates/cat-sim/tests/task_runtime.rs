use cat_sim::{
    planner_core::{IntentId, PlannerId},
    reservation_transaction::{
        ClaimMode, ClaimSpec, ReservationBundle, ReservationChecks, ReservationId,
        ReservationLedger,
    },
    spatial_tasks::{
        Rect, ResourceSourceKind, SiteMetadata, SiteRef, SpatialBlockReason, SpatialObjective,
        TaskFootprint, TilePoint, WorkSlot,
    },
    task_runtime::{
        CargoLocation, RestartRevalidationOutcome, RuntimeBlockReason, TaskCategory,
        TaskRuntimeError, TaskStage, VisibleTaskRuntime,
    },
};

fn id(namespace: &str, value: &str) -> PlannerId {
    PlannerId::derive(namespace, [value])
}

fn footprint(x: i32, y: i32) -> TaskFootprint {
    TaskFootprint::rectangular(Rect::new(TilePoint { x, y }, 1, 1).unwrap())
}

fn source() -> SiteRef {
    SiteRef::ResourceSource {
        metadata: SiteMetadata::revealed("cave-7"),
        source_id: "cave-7".to_owned(),
        resource_kind: ResourceSourceKind::Hunting,
        footprint: footprint(4, 4),
    }
}

fn endpoint() -> SiteRef {
    SiteRef::Stockpile {
        metadata: SiteMetadata::revealed("food-pile"),
        stockpile_id: "food-pile".to_owned(),
        footprint: footprint(8, 8),
    }
}

fn spatial() -> SpatialObjective {
    SpatialObjective::resolved(
        source(),
        vec![WorkSlot::exclusive(
            "cave-entrance",
            SiteRef::Tile {
                metadata: SiteMetadata::revealed("cave-bank"),
                tile: TilePoint { x: 4, y: 5 },
            },
        )],
        Some(endpoint()),
    )
}

fn runtime() -> VisibleTaskRuntime {
    VisibleTaskRuntime::resolved(
        "colony-1",
        IntentId::derive("colony-1", 9, "hunt", "cave-7", 0),
        0,
        TaskCategory::Hunt,
        spatial(),
        vec!["route-source".to_owned(), "route-endpoint".to_owned()],
        100,
    )
    .unwrap()
}

fn committed_reservation(task: &VisibleTaskRuntime) -> (ReservationLedger, ReservationId) {
    let colony_id = id("colony", "colony-1");
    let task_id = id("task", task.id.as_str());
    let bundle = ReservationBundle::from_spatial_objective(
        colony_id,
        task_id,
        task.intent_id.clone(),
        &task.spatial,
        0,
        ClaimMode::Capacity {
            units: 1,
            capacity: 2,
        },
        ClaimMode::Capacity {
            units: 1,
            capacity: 50,
        },
        ClaimSpec::capacity(id("route", "hunt"), 1, 4),
        vec![ClaimSpec::exclusive(id("tool", "bow"))],
        vec![ClaimSpec::capacity(id("resource", "food"), 2, 20)],
        id("cat", "hunter"),
    )
    .unwrap();
    let reservation_id = bundle.id.clone();
    let mut ledger = ReservationLedger::new();
    ledger
        .try_commit(bundle, ReservationChecks::all_valid())
        .unwrap();
    (ledger, reservation_id)
}

fn activate(task: &mut VisibleTaskRuntime, ledger: &ReservationLedger, id: ReservationId) {
    task.activate(
        ledger,
        id,
        [("hunter".to_owned(), "cave-entrance".to_owned())],
        101,
    )
    .unwrap();
}

#[test]
fn unresolved_task_has_no_marker_assignment_or_busy_cat() {
    let task = VisibleTaskRuntime::resolved(
        "colony-1",
        IntentId::derive("colony-1", 1, "hunt", "missing", 0),
        0,
        TaskCategory::Hunt,
        SpatialObjective::blocked(SpatialBlockReason::SourceUnavailable),
        Vec::new(),
        1,
    )
    .unwrap();
    assert_eq!(task.stage, TaskStage::Blocked);
    assert!(!task.emits_world_marker());
    assert!(!task.is_worker_busy("hunter", &ReservationLedger::new()));
}

#[test]
fn cat_becomes_busy_only_after_complete_reservation_commits() {
    let mut task = runtime();
    let (ledger, reservation_id) = committed_reservation(&task);
    task.begin_reservation(100).unwrap();
    assert_eq!(task.stage, TaskStage::Reserve);
    assert_eq!(
        task.activate(
            &ReservationLedger::new(),
            reservation_id.clone(),
            [("hunter".to_owned(), "cave-entrance".to_owned())],
            101,
        ),
        Err(TaskRuntimeError::ReservationNotCommitted)
    );
    assert!(!task.is_worker_busy("hunter", &ledger));
    activate(&mut task, &ledger, reservation_id);
    assert!(task.is_worker_busy("hunter", &ledger));
}

#[test]
fn pre_pickup_route_loss_rolls_back_everything_and_releases_reserved_cargo() {
    let mut task = runtime();
    let (mut ledger, reservation_id) = committed_reservation(&task);
    activate(&mut task, &ledger, reservation_id.clone());
    task.reserve_cargo_at_source("cargo-1", "food", 2).unwrap();
    task.block_before_pickup(
        RuntimeBlockReason::RouteClosedBeforePickup,
        &mut ledger,
        102,
    )
    .unwrap();
    assert!(!ledger.contains(&reservation_id));
    assert!(task.assigned_cat_ids.is_empty());
    assert!(task.cargo.is_none());
    assert!(!task.is_worker_busy("hunter", &ledger));
}

#[test]
fn picked_up_cargo_cannot_be_cancelled_and_is_salvaged_without_quantity_change() {
    let mut task = runtime();
    let original_spatial = task.spatial.clone();
    let (mut ledger, reservation_id) = committed_reservation(&task);
    activate(&mut task, &ledger, reservation_id.clone());
    task.reserve_cargo_at_source("cargo-1", "food", 2).unwrap();
    task.advance(TaskStage::Pickup, 102).unwrap();
    task.pickup("hunter", 103).unwrap();
    assert_eq!(
        task.cancel(&mut ledger, 104),
        Err(TaskRuntimeError::CargoRequiresRecovery)
    );
    task.recover_after_pickup(
        RuntimeBlockReason::RouteClosedWithCargo,
        Some(&endpoint()),
        "cave-bank",
        &mut ledger,
        105,
    )
    .unwrap();
    let cargo = task.cargo.as_ref().unwrap();
    assert_eq!((cargo.cargo_id.as_str(), cargo.quantity), ("cargo-1", 2));
    assert_eq!(
        cargo.location,
        CargoLocation::SalvagedAtStockpile {
            stockpile_id: "food-pile".to_owned()
        }
    );
    assert_eq!(task.spatial, original_spatial);
    assert!(!ledger.contains(&reservation_id));
}

#[test]
fn objective_endpoint_route_cargo_and_stage_survive_restart_exactly() {
    let mut task = runtime();
    let (ledger, reservation_id) = committed_reservation(&task);
    activate(&mut task, &ledger, reservation_id);
    task.reserve_cargo_at_source("cargo-1", "food", 2).unwrap();
    task.advance(TaskStage::Pickup, 102).unwrap();
    task.pickup("hunter", 103).unwrap();
    task.advance(TaskStage::TravelToEndpoint, 104).unwrap();
    let json = serde_json::to_string(&task).unwrap();
    let restored: VisibleTaskRuntime = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, task);
    assert_eq!(
        restored.spatial.objective.as_ref().unwrap().stable_id(),
        "cave-7"
    );
    assert_eq!(
        restored
            .spatial
            .delivery_endpoint
            .as_ref()
            .unwrap()
            .stable_id(),
        "food-pile"
    );
    assert_eq!(
        restored.route_ids,
        vec!["route-source".to_owned(), "route-endpoint".to_owned()]
    );
}

#[test]
fn restart_revalidation_drops_no_cargo_when_reservation_is_missing() {
    let mut task = runtime();
    let (ledger, reservation_id) = committed_reservation(&task);
    activate(&mut task, &ledger, reservation_id);
    task.reserve_cargo_at_source("cargo-restart", "food", 7)
        .unwrap();
    task.advance(TaskStage::Pickup, 102).unwrap();
    task.pickup("hunter", 103).unwrap();
    let mut restored: VisibleTaskRuntime =
        serde_json::from_str(&serde_json::to_string(&task).unwrap()).unwrap();
    let outcome = restored
        .revalidate_after_restart(
            &mut ReservationLedger::new(),
            Some(&endpoint()),
            "cave-bank",
            104,
        )
        .unwrap();
    assert_eq!(outcome, RestartRevalidationOutcome::BlockedCargoPreserved);
    assert_eq!(restored.cargo.as_ref().unwrap().quantity, 7);
    assert!(matches!(
        restored.cargo.as_ref().unwrap().location,
        CargoLocation::SalvagedAtStockpile { .. }
    ));
    assert!(restored.assigned_cat_ids.is_empty());
}

#[test]
fn deposit_and_complete_credit_no_second_cargo_and_release_reservation() {
    let mut task = runtime();
    let (mut ledger, reservation_id) = committed_reservation(&task);
    activate(&mut task, &ledger, reservation_id.clone());
    task.reserve_cargo_at_source("cargo-1", "food", 2).unwrap();
    task.advance(TaskStage::Pickup, 102).unwrap();
    task.pickup("hunter", 103).unwrap();
    task.advance(TaskStage::TravelToEndpoint, 104).unwrap();
    task.advance(TaskStage::Deposit, 105).unwrap();
    task.deposit(106).unwrap();
    task.complete(&mut ledger, 107).unwrap();
    assert_eq!(task.stage, TaskStage::Complete);
    assert!(!ledger.contains(&reservation_id));
    assert_eq!(task.cargo.as_ref().unwrap().quantity, 2);
    assert!(matches!(
        task.cargo.as_ref().unwrap().location,
        CargoLocation::DepositedAtEndpoint { .. }
    ));
}

#[test]
fn malformed_restart_and_illegal_stage_transition_fail_closed() {
    let task = runtime();
    let mut value = serde_json::to_value(&task).unwrap();
    value["progressBasisPoints"] = serde_json::json!(10_001);
    assert!(serde_json::from_value::<VisibleTaskRuntime>(value).is_err());
    assert_eq!(
        runtime().advance(TaskStage::Deposit, 2),
        Err(TaskRuntimeError::InvalidTransition)
    );
}
