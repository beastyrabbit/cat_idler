use cat_sim::{
    authority::{AuthorityActor, AuthorityContext, AuthorityDomain},
    divine_boosts::{
        DivineBoostPurchaseId, DivineBoostPurchaseRequest, DivineBoostResearchStages,
        DivineBoostType,
    },
    favor::{Favor, FavorEventId, FavorEventKind},
    intent_graph::{Intent, IntentGraph},
    leader_ai_runtime::{
        LEADER_AI_RUNTIME_SCHEMA_VERSION, LeaderAiRuntimeError, LeaderAiRuntimeState,
        RuntimeIdempotencyReceipt, RuntimeMutationId,
    },
    planner_core::{IntentId, PlannerId},
    research_purchase::StudyId,
    reservation_transaction::{
        ClaimMode, ClaimSpec, ReservationBundle, ReservationChecks, ReservationId,
        ReservationLedger,
    },
    shrine_offerings::{OfferingChoice, OfferingPackage, ShrineOfferingState},
    spatial_tasks::{
        Rect, ResourceSourceKind, SiteMetadata, SiteRef, SpatialObjective, TaskFootprint,
        TilePoint, WorkSlot,
    },
    task_runtime::{CargoLocation, TaskCargo, TaskCategory, TaskStage, VisibleTaskRuntime},
};

fn id(namespace: &str, value: &str) -> PlannerId {
    PlannerId::derive(namespace, [value])
}

fn colony() -> PlannerId {
    id("colony", "colony-1")
}

fn player() -> PlannerId {
    id("player", "owner")
}

fn intent_id(kind: &str, target: &str) -> IntentId {
    IntentId::derive(colony().as_str(), 1, kind, target, 0)
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

fn intent(target: &str) -> Intent {
    Intent::proposed(
        intent_id("hunt", target),
        colony(),
        AuthorityActor::God {
            player_id: player(),
        },
        Some(id("cat", "leader")),
        AuthorityDomain::Survival,
        id("kind", "hunt"),
        id("target", target),
        id("rationale", target),
        10,
    )
}

fn graph_with_intent(target: &str) -> IntentGraph {
    let mut graph = IntentGraph::new();
    graph.insert_or_merge(intent(target)).unwrap();
    graph
}

fn runtime_task() -> VisibleTaskRuntime {
    VisibleTaskRuntime::resolved(
        "colony-1",
        intent_id("hunt", "cave-7"),
        0,
        TaskCategory::Hunt,
        spatial(),
        vec!["route-source".to_owned(), "route-endpoint".to_owned()],
        100,
    )
    .unwrap()
}

fn committed_reservation(task: &VisibleTaskRuntime) -> (ReservationLedger, ReservationId) {
    let bundle = ReservationBundle::from_spatial_objective(
        colony(),
        id("task", task.id.as_str()),
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

fn active_runtime_state() -> LeaderAiRuntimeState {
    let mut state = LeaderAiRuntimeState::new();
    state.intents = graph_with_intent("cave-7");
    let mut task = runtime_task();
    let (ledger, reservation_id) = committed_reservation(&task);
    task.activate(
        &ledger,
        reservation_id,
        [("hunter".to_owned(), "cave-entrance".to_owned())],
        101,
    )
    .unwrap();
    state.scheduling.reservations = ledger;
    state
        .scheduling
        .visible_tasks
        .insert(task.id.clone(), task.clone());
    state
        .scheduling
        .known_cargo_site_ids
        .extend(["cave-7".to_owned(), "food-pile".to_owned()]);
    state
}

#[test]
fn fresh_defaults_are_deterministic_and_reject_runtime_schema_drift() {
    let state = LeaderAiRuntimeState::new();
    assert_eq!(state.schema_version, LEADER_AI_RUNTIME_SCHEMA_VERSION);
    state.validate().unwrap();

    let json = serde_json::to_string(&state).unwrap();
    let restored: LeaderAiRuntimeState = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, state);
    assert_eq!(serde_json::to_string(&restored).unwrap(), json);

    let mut wrong_version = serde_json::to_value(&state).unwrap();
    wrong_version["schemaVersion"] = serde_json::json!(2);
    assert!(serde_json::from_value::<LeaderAiRuntimeState>(wrong_version).is_err());
}

#[test]
fn restart_round_trip_and_permutation_twins_keep_stable_task_and_receipt_order() {
    let mut first = active_runtime_state();
    let receipt_a = RuntimeIdempotencyReceipt {
        id: RuntimeMutationId::derive("test", &colony(), "a"),
        committed_tick: 10,
        expires_tick: 100,
        request_fingerprint: String::new(),
        response_json: String::new(),
    };
    let receipt_b = RuntimeIdempotencyReceipt {
        id: RuntimeMutationId::derive("test", &colony(), "b"),
        committed_tick: 11,
        expires_tick: 101,
        request_fingerprint: String::new(),
        response_json: String::new(),
    };
    first
        .idempotency_receipts
        .insert(receipt_b.id.clone(), receipt_b.clone());
    first
        .idempotency_receipts
        .insert(receipt_a.id.clone(), receipt_a.clone());

    let mut second = active_runtime_state();
    second
        .idempotency_receipts
        .insert(receipt_a.id.clone(), receipt_a);
    second
        .idempotency_receipts
        .insert(receipt_b.id.clone(), receipt_b);

    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
    let restarted: LeaderAiRuntimeState =
        serde_json::from_str(&serde_json::to_string(&first).unwrap()).unwrap();
    assert_eq!(restarted, first);
}

#[test]
fn aggregate_rejects_dangling_task_reservation_intent_and_cargo_references() {
    let mut missing_intent = active_runtime_state();
    missing_intent.intents = IntentGraph::new();
    assert_eq!(
        missing_intent.validate(),
        Err(LeaderAiRuntimeError::DanglingTaskIntent)
    );

    let mut missing_reservation = active_runtime_state();
    missing_reservation.scheduling.reservations = ReservationLedger::new();
    assert_eq!(
        missing_reservation.validate(),
        Err(LeaderAiRuntimeError::DanglingTaskReservation)
    );

    let mut missing_cargo_site = active_runtime_state();
    let task = missing_cargo_site
        .scheduling
        .visible_tasks
        .values_mut()
        .next()
        .unwrap();
    task.reserve_cargo_at_source("cargo-1", "food", 1).unwrap();
    task.advance(TaskStage::Pickup, 102).unwrap();
    task.pickup("hunter", 103).unwrap();
    task.cargo = Some(TaskCargo {
        cargo_id: "cargo-1".to_owned(),
        resource_id: "food".to_owned(),
        quantity: 1,
        location: CargoLocation::Carried {
            cat_id: "not-assigned".to_owned(),
        },
    });
    assert_eq!(
        missing_cargo_site.validate(),
        Err(LeaderAiRuntimeError::DanglingCargoReference)
    );
}

#[test]
fn projection_favor_and_idempotency_persistence_are_strict() {
    let state = LeaderAiRuntimeState::new();

    let mut hidden_regen = serde_json::to_value(&state).unwrap();
    hidden_regen["projectionSeal"]["hiddenRegeneration"] = serde_json::json!(42);
    assert!(serde_json::from_value::<LeaderAiRuntimeState>(hidden_regen).is_err());

    let mut negative_favor = serde_json::to_value(&state).unwrap();
    negative_favor["shrineFavor"]["favor"]["balance"] = serde_json::json!(-1);
    assert!(serde_json::from_value::<LeaderAiRuntimeState>(negative_favor).is_err());

    let mut bad_receipt = state;
    let id = RuntimeMutationId::derive("test", &colony(), "bad");
    bad_receipt.idempotency_receipts.insert(
        id.clone(),
        RuntimeIdempotencyReceipt {
            id,
            committed_tick: 20,
            expires_tick: 19,
            request_fingerprint: String::new(),
            response_json: String::new(),
        },
    );
    assert_eq!(
        bad_receipt.validate(),
        Err(LeaderAiRuntimeError::MalformedRuntimeState)
    );
}

#[test]
fn invalid_shrine_boost_and_research_stages_are_rejected() {
    let mut state = active_runtime_state();
    let mut shrine = ShrineOfferingState::new("shrine-a");
    shrine
        .start(
            OfferingChoice {
                package: OfferingPackage::Food,
                utility_micro_favor: 1_000_000,
                evidence_ids: vec!["belief-a".to_owned()],
            },
            5,
        )
        .unwrap();
    state
        .shrine_favor
        .shrine_offerings
        .insert("shrine-a".to_owned(), shrine);
    let mut invalid_shrine = serde_json::to_value(&state).unwrap();
    invalid_shrine["shrineFavor"]["shrineOfferings"]["shrine-a"]["current"]["stage"] =
        serde_json::json!("ritual");
    invalid_shrine["shrineFavor"]["shrineOfferings"]["shrine-a"]["current"]["physicalTaskId"] =
        serde_json::json!("missing-task");
    assert!(serde_json::from_value::<LeaderAiRuntimeState>(invalid_shrine).is_err());

    let mut boosted = active_runtime_state();
    boosted
        .shrine_favor
        .favor
        .credit(
            FavorEventId::derive("test_funding", colony().as_str(), "grant"),
            FavorEventKind::LegacyMigrationCredit,
            Favor::from_whole(10).unwrap(),
            0,
            0,
        )
        .unwrap();
    let purchase = DivineBoostPurchaseRequest {
        id: DivineBoostPurchaseId::derive("test", &colony(), "fleet"),
        colony_id: colony(),
        actor: AuthorityActor::God {
            player_id: player(),
        },
        authority_context: AuthorityContext {
            leader_present: true,
            player_authorized: true,
        },
        boost_type: DivineBoostType::FleetPaws,
        duration_hours: 1,
        committed_research_stages: DivineBoostResearchStages::default(),
        expected_boost_version: boosted.boosts.version,
        expected_favor_version: boosted.shrine_favor.favor.version,
        activated_tick: 10,
        ticks_per_game_hour: 60,
    };
    boosted
        .boosts
        .purchase(&mut boosted.shrine_favor.favor, purchase)
        .unwrap();
    let mut invalid_boost = serde_json::to_value(&boosted).unwrap();
    invalid_boost["boosts"]["active"]["fleet_paws"]["committedResearchStages"]["divineEconomyStage"] =
        serde_json::json!(12);
    invalid_boost["boosts"]["purchases"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .unwrap()["committedResearchStages"]["divineEconomyStage"] = serde_json::json!(12);
    assert!(serde_json::from_value::<LeaderAiRuntimeState>(invalid_boost).is_err());

    let mut invalid_research = LeaderAiRuntimeState::new();
    invalid_research
        .research
        .purchases
        .owned_studies
        .insert(StudyId::derive("divine_duration_stage_12"));
    assert_eq!(
        invalid_research.validate(),
        Err(LeaderAiRuntimeError::MalformedResearchStages)
    );
}

#[test]
fn completed_shrine_receipt_keeps_historical_task_id_after_bounded_task_pruning() {
    let mut state = LeaderAiRuntimeState::new();
    let mut shrine = ShrineOfferingState::new("shrine-a");
    let pipeline = shrine
        .start(
            OfferingChoice {
                package: OfferingPackage::Materials,
                utility_micro_favor: 1_000_000,
                evidence_ids: vec!["report-a".to_owned()],
            },
            5,
        )
        .unwrap();
    pipeline.resources_reserved("historical-task", 6).unwrap();
    pipeline.depart(7).unwrap();
    pipeline.deposit(8).unwrap();
    pipeline.begin_ritual(9).unwrap();
    pipeline
        .consume_and_credit(true, &mut state.shrine_favor.favor, 0, 10)
        .unwrap();
    state
        .shrine_favor
        .shrine_offerings
        .insert("shrine-a".to_owned(), shrine);

    state
        .validate()
        .expect("terminal audit references do not require retained task rows");
    let restarted: LeaderAiRuntimeState =
        serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
    assert_eq!(restarted, state);
}
