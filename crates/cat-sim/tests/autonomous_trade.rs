use std::collections::{BTreeMap, BTreeSet};

use cat_sim::{
    authority::AuthorityActor,
    autonomous_trade::{
        DeliveryValidation, RecoveryDisposition, TradeAction, TradeActionId, TradeActionKind,
        TradeAuthorization, TradeBlockReason, TradeCargoLeg, TradeColonyKind, TradeContractId,
        TradeError, TradeLedger, TradeParty, TradeProposal, TradeRecoveryState, TradeStage,
    },
    beliefs::{
        BeliefKey, BeliefKind, BeliefProjection, Confidence, EstimateRange, EvidenceId,
        EvidenceSource, ProjectedBeliefValue, ReportLevel,
    },
    diplomacy::{DiplomacyColonyId, DiplomacyPair, DiplomacyRelationship},
    planner_core::{IntentId, PlannerId},
    spatial_resolver::{
        ResolvedSpatialTask, SpatialResolutionCandidate, SpatialResolutionOutcome,
        SpatialResolutionRequest, SpatialTaskCategory, resolve_spatial_task,
    },
    spatial_tasks::{
        Rect, ResourceSourceKind, SiteMetadata, SiteRef, TaskFootprint, TilePoint, WorkSlot,
    },
    task_runtime::CargoLocation,
    trade_valuation::{
        ALLIED_STRATEGIC_DISADVANTAGE_BASIS_POINTS, FRIENDLY_VALUE_BOUND_BASIS_POINTS,
        TradePersonality, TradePurpose, TradeValuation, TradeValuationError,
    },
    world_reservations::{
        CapacityReservation, WorldReservationLedger, WorldReservationTransaction,
    },
};

fn id(namespace: &str, value: &str) -> PlannerId {
    PlannerId::derive(namespace, [value])
}

fn projection(
    label: &str,
    value: i64,
    confidence: u16,
    expires_tick: Option<u64>,
) -> BeliefProjection {
    let key = BeliefKey::new(
        id("trade-domain", label),
        id("trade-subject", label),
        BeliefKind::Stock,
    );
    let reporter = id("cat", &format!("reporter-{label}"));
    let evidence = EvidenceId::derive(label, &key, 5, &reporter, 0);
    BeliefProjection {
        key,
        value: ProjectedBeliefValue::StockRange(EstimateRange::new(value, value, value).unwrap()),
        confidence: Confidence::new(confidence).unwrap(),
        observed_tick: 5,
        expires_tick,
        source: EvidenceSource::AuthorizedOfficerReport,
        reporter_id: reporter,
        evidence_ids: BTreeSet::from([evidence]),
        report_level: ReportLevel::Three,
    }
}

fn valuation(
    relationship: DiplomacyRelationship,
    purpose: TradePurpose,
    personality: TradePersonality,
    offered: i64,
    requested: i64,
) -> TradeValuation {
    TradeValuation::evaluate(
        relationship,
        purpose,
        personality,
        &projection("offered", offered, 8_000, Some(100)),
        &projection("requested", requested, 8_000, Some(100)),
        10,
    )
    .unwrap()
}

fn footprint(anchor: TilePoint) -> TaskFootprint {
    TaskFootprint::rectangular(Rect::new(anchor, 1, 1).unwrap())
}

fn route(stable_id: &str, x: i32) -> SiteRef {
    SiteRef::OrderedRoute {
        metadata: SiteMetadata::revealed(stable_id),
        route: vec![TilePoint { x, y: 1 }, TilePoint { x: x + 1, y: 1 }],
    }
}

fn resolved_spatial(source_id: &str, endpoint_id: &str, x: i32) -> ResolvedSpatialTask {
    let source = SiteRef::ResourceSource {
        metadata: SiteMetadata::revealed(source_id),
        source_id: source_id.to_owned(),
        resource_kind: ResourceSourceKind::FishHabitat,
        footprint: footprint(TilePoint { x, y: 0 }),
    };
    let bank = SiteRef::Tile {
        metadata: SiteMetadata::revealed(format!("bank-{source_id}")),
        tile: TilePoint { x, y: 1 },
    };
    let endpoint = SiteRef::Stockpile {
        metadata: SiteMetadata::revealed(endpoint_id),
        stockpile_id: endpoint_id.to_owned(),
        footprint: footprint(TilePoint { x: x + 5, y: 1 }),
    };
    let request = SpatialResolutionRequest {
        category: SpatialTaskCategory::Fish,
        pinned_objective_id: Some(source_id.to_owned()),
        pinned_delivery_endpoint: endpoint,
        delivery_endpoint_exists: true,
        requested_source_units: 1,
        requested_delivery_units: 1,
        delivery_capacity: 1,
        candidates: vec![SpatialResolutionCandidate {
            objective: source,
            work_slot: WorkSlot::exclusive(format!("slot-{source_id}"), bank),
            source_to_work_route: route(&format!("pickup-{source_id}"), x),
            work_to_delivery_route: route(&format!("delivery-{source_id}"), x + 2),
            objective_exists: true,
            work_position_available: true,
            source_available_units: 1,
            source_capacity: 1,
            source_to_work_route_capacity: 1,
            work_to_delivery_route_capacity: 1,
        }],
    };
    match resolve_spatial_task(request) {
        SpatialResolutionOutcome::Resolved(resolved) => *resolved,
        SpatialResolutionOutcome::Blocked(blocked) => {
            panic!("unexpected spatial block: {blocked:?}")
        }
    }
}

fn leg(
    owner: &DiplomacyColonyId,
    recipient: &DiplomacyColonyId,
    reservation_colony_id: PlannerId,
    source_id: &str,
    occurrence: u32,
    x: i32,
) -> TradeCargoLeg {
    let spatial = resolved_spatial(source_id, &format!("endpoint-{source_id}"), x);
    let hauler_id = id("cat", &format!("hauler-{source_id}-{occurrence}"));
    let task_id = id("trade-task", &format!("{source_id}-{occurrence}"));
    let intent_id = IntentId::derive(owner.as_str(), u64::from(occurrence), "trade", source_id, 0);
    let resource_id = format!("goods-{source_id}");
    let escrow = WorldReservationTransaction::new(
        reservation_colony_id,
        task_id,
        intent_id,
        spatial.clone(),
        hauler_id.clone(),
        Vec::new(),
        vec![CapacityReservation {
            stable_id: PlannerId::derive("trade_resource", [&resource_id]),
            units: 1,
            capacity: 1,
        }],
    )
    .unwrap();
    TradeCargoLeg::new(
        owner.clone(),
        recipient.clone(),
        resource_id,
        1,
        spatial,
        escrow,
        hauler_id,
    )
    .unwrap()
}

#[derive(Clone)]
struct Fixture {
    proposal: TradeProposal,
    auth_a: TradeAuthorization,
    auth_b: TradeAuthorization,
}

fn fixture(
    occurrence: u32,
    source_a: &str,
    source_b: &str,
    relationship: DiplomacyRelationship,
    created_tick: u64,
) -> Fixture {
    fixture_for_colonies(
        occurrence,
        source_a,
        source_b,
        relationship,
        created_tick,
        "colony-a",
        "colony-b",
    )
}

#[allow(clippy::too_many_arguments)]
fn fixture_for_colonies(
    occurrence: u32,
    source_a: &str,
    source_b: &str,
    relationship: DiplomacyRelationship,
    created_tick: u64,
    colony_a_name: &str,
    colony_b_name: &str,
) -> Fixture {
    let colony_a = DiplomacyColonyId::derive(colony_a_name);
    let colony_b = DiplomacyColonyId::derive(colony_b_name);
    let pair = DiplomacyPair::new(colony_a.clone(), colony_b.clone()).unwrap();
    let reservation_a = id("colony", colony_a_name);
    let reservation_b = id("colony", colony_b_name);
    let parties = BTreeMap::from([
        (
            colony_a.clone(),
            TradeParty {
                diplomacy_id: colony_a.clone(),
                reservation_colony_id: reservation_a.clone(),
                kind: TradeColonyKind::PlayerFounded,
            },
        ),
        (
            colony_b.clone(),
            TradeParty {
                diplomacy_id: colony_b.clone(),
                reservation_colony_id: reservation_b.clone(),
                kind: TradeColonyKind::PlayerFounded,
            },
        ),
    ]);
    let purpose = TradePurpose::Ordinary;
    let valuations = BTreeMap::from([
        (
            colony_a.clone(),
            valuation(relationship, purpose, TradePersonality::Mercantile, 100, 95),
        ),
        (
            colony_b.clone(),
            valuation(
                relationship,
                purpose,
                TradePersonality::SelfSufficient,
                95,
                100,
            ),
        ),
    ]);
    let actor_a = AuthorityActor::Leader {
        cat_id: id("cat", &format!("leader-{colony_a_name}")),
    };
    let actor_b = AuthorityActor::Leader {
        cat_id: id("cat", &format!("leader-{colony_b_name}")),
    };
    let proposal = TradeProposal::new(
        pair,
        colony_a.clone(),
        occurrence,
        parties,
        relationship,
        purpose,
        valuations,
        vec![
            leg(&colony_a, &colony_b, reservation_a, source_a, occurrence, 0),
            leg(
                &colony_b,
                &colony_a,
                reservation_b,
                source_b,
                occurrence,
                20,
            ),
        ],
        created_tick,
        created_tick + 100,
        actor_a.clone(),
    )
    .unwrap();
    Fixture {
        proposal,
        auth_a: TradeAuthorization {
            actor: actor_a,
            acting_colony: colony_a,
            owner_player_id: None,
            authorized_for_colony: true,
        },
        auth_b: TradeAuthorization {
            actor: actor_b,
            acting_colony: colony_b,
            owner_player_id: None,
            authorized_for_colony: true,
        },
    }
}

fn action(
    contract_id: &TradeContractId,
    authorization: &TradeAuthorization,
    expected_version: u64,
    kind: TradeActionKind,
    occurrence: &str,
) -> TradeAction {
    TradeAction {
        id: TradeActionId::derive(contract_id, &authorization.acting_colony, occurrence, kind),
        contract_id: contract_id.clone(),
        acting_colony: authorization.acting_colony.clone(),
        expected_version,
        kind,
    }
}

fn accept_both(
    ledger: &mut TradeLedger,
    world: &mut WorldReservationLedger,
    fixture: &Fixture,
) -> TradeContractId {
    let contract_id = fixture.proposal.contract_id.clone();
    ledger
        .propose(fixture.proposal.clone(), &fixture.auth_a)
        .unwrap();
    ledger
        .apply_action(
            action(
                &contract_id,
                &fixture.auth_a,
                0,
                TradeActionKind::Accept,
                "a",
            ),
            &fixture.auth_a,
            fixture.proposal.relationship,
            fixture.proposal.created_tick + 1,
            world,
        )
        .unwrap();
    ledger
        .apply_action(
            action(
                &contract_id,
                &fixture.auth_b,
                1,
                TradeActionKind::Accept,
                "b",
            ),
            &fixture.auth_b,
            fixture.proposal.relationship,
            fixture.proposal.created_tick + 2,
            world,
        )
        .unwrap();
    contract_id
}

#[test]
fn friendly_and_allied_bounds_are_integer_exact() {
    assert_eq!(FRIENDLY_VALUE_BOUND_BASIS_POINTS, 1_000);
    assert_eq!(ALLIED_STRATEGIC_DISADVANTAGE_BASIS_POINTS, 2_000);
    assert_eq!(
        valuation(
            DiplomacyRelationship::Friendly,
            TradePurpose::Ordinary,
            TradePersonality::Balanced,
            100,
            90,
        )
        .disadvantage_basis_points,
        1_000
    );
    assert!(matches!(
        TradeValuation::evaluate(
            DiplomacyRelationship::Friendly,
            TradePurpose::Ordinary,
            TradePersonality::Balanced,
            &projection("offer", 100, 9_000, Some(100)),
            &projection("request", 89, 9_000, Some(100)),
            10,
        ),
        Err(TradeValuationError::OutsideRelationshipBound)
    ));
    assert!(matches!(
        TradeValuation::evaluate(
            DiplomacyRelationship::Friendly,
            TradePurpose::Ordinary,
            TradePersonality::Balanced,
            &projection("offer-fraction", 10_001, 9_000, Some(100)),
            &projection("request-fraction", 9_000, 9_000, Some(100)),
            10,
        ),
        Err(TradeValuationError::OutsideRelationshipBound)
    ));
    assert_eq!(
        valuation(
            DiplomacyRelationship::Allied,
            TradePurpose::Survival,
            TradePersonality::Balanced,
            100,
            80,
        )
        .disadvantage_basis_points,
        2_000
    );
    assert!(
        TradeValuation::evaluate(
            DiplomacyRelationship::Allied,
            TradePurpose::ActiveDefense,
            TradePersonality::Balanced,
            &projection("offer", 100, 9_000, Some(100)),
            &projection("request", 79, 9_000, Some(100)),
            10,
        )
        .is_err()
    );
    assert!(
        TradeValuation::evaluate(
            DiplomacyRelationship::Allied,
            TradePurpose::Ordinary,
            TradePersonality::Balanced,
            &projection("offer", 100, 9_000, Some(100)),
            &projection("request", 89, 9_000, Some(100)),
            10,
        )
        .is_err()
    );
}

#[test]
fn valuation_accepts_only_fresh_report_safe_beliefs_and_personality_never_expands_bounds() {
    let expired = projection("expired", 100, 8_000, Some(10));
    let fresh = projection("fresh", 100, 8_000, Some(100));
    assert_eq!(
        TradeValuation::evaluate(
            DiplomacyRelationship::Friendly,
            TradePurpose::Ordinary,
            TradePersonality::Balanced,
            &expired,
            &fresh,
            10,
        ),
        Err(TradeValuationError::RecountRequired)
    );
    let zero = projection("zero", 100, 0, Some(100));
    assert!(matches!(
        TradeValuation::evaluate(
            DiplomacyRelationship::Friendly,
            TradePurpose::Ordinary,
            TradePersonality::Balanced,
            &zero,
            &fresh,
            10,
        ),
        Err(TradeValuationError::RecountRequired)
    ));
    let mercantile = valuation(
        DiplomacyRelationship::Friendly,
        TradePurpose::Ordinary,
        TradePersonality::Mercantile,
        100,
        105,
    );
    let self_sufficient = valuation(
        DiplomacyRelationship::Friendly,
        TradePurpose::Ordinary,
        TradePersonality::SelfSufficient,
        100,
        105,
    );
    assert_ne!(
        mercantile.personality_preference,
        self_sufficient.personality_preference
    );
    assert!(matches!(
        TradeValuation::evaluate(
            DiplomacyRelationship::Friendly,
            TradePurpose::Ordinary,
            TradePersonality::Mercantile,
            &projection("personality-offer", 100, 8_000, Some(100)),
            &projection("personality-request", 89, 8_000, Some(100)),
            10,
        ),
        Err(TradeValuationError::OutsideRelationshipBound)
    ));
    let json = serde_json::to_string(&mercantile).unwrap();
    for forbidden in ["hidden", "headroom", "regeneration", "danger"] {
        assert!(!json.contains(forbidden));
    }
}

#[test]
fn neutral_blocked_npc_and_forged_authority_cannot_initiate_or_accept() {
    for relationship in [
        DiplomacyRelationship::Neutral,
        DiplomacyRelationship::Blocked,
    ] {
        assert!(matches!(
            TradeValuation::evaluate(
                relationship,
                TradePurpose::Ordinary,
                TradePersonality::Balanced,
                &projection("offer", 100, 8_000, Some(100)),
                &projection("request", 100, 8_000, Some(100)),
                10,
            ),
            Err(TradeValuationError::RelationshipDenied)
        ));
    }
    let fixture = fixture(
        1,
        "source-a-1",
        "source-b-1",
        DiplomacyRelationship::Friendly,
        10,
    );
    let mut npc = fixture.proposal.clone();
    npc.parties.get_mut(npc.pair.second()).unwrap().kind = TradeColonyKind::Npc;
    let mut ledger = TradeLedger::new();
    assert_eq!(
        ledger.propose(npc, &fixture.auth_a),
        Err(TradeError::NpcLayerSeparated)
    );

    let mut forged = fixture.auth_a.clone();
    forged.acting_colony = DiplomacyColonyId::derive("forged");
    assert_eq!(
        ledger.propose(fixture.proposal.clone(), &forged),
        Err(TradeError::AuthorizationColonyMismatch)
    );

    let contract_id = ledger
        .propose(fixture.proposal.clone(), &fixture.auth_a)
        .unwrap();
    let mut world = WorldReservationLedger::new();
    assert_eq!(
        ledger.apply_action(
            action(
                &contract_id,
                &fixture.auth_a,
                0,
                TradeActionKind::Accept,
                "stale-relationship",
            ),
            &fixture.auth_a,
            DiplomacyRelationship::Allied,
            11,
            &mut world,
        ),
        Err(TradeError::RelationshipDenied)
    );
    assert_eq!(
        ledger.apply_action(
            action(
                &contract_id,
                &fixture.auth_a,
                0,
                TradeActionKind::Accept,
                "blocked"
            ),
            &fixture.auth_a,
            DiplomacyRelationship::Blocked,
            11,
            &mut world,
        ),
        Err(TradeError::RelationshipDenied)
    );
    assert!(world.is_empty());
    assert_eq!(ledger.contract(&contract_id).unwrap().version, 0);
}

#[test]
fn mutual_acceptance_commits_both_escrows_atomically_and_prevents_double_spend() {
    let first = fixture(
        1,
        "shared-source-a",
        "source-b-1",
        DiplomacyRelationship::Friendly,
        10,
    );
    let mut ledger = TradeLedger::new();
    let mut world = WorldReservationLedger::new();
    let first_id = first.proposal.contract_id.clone();
    ledger
        .propose(first.proposal.clone(), &first.auth_a)
        .unwrap();
    ledger
        .apply_action(
            action(
                &first_id,
                &first.auth_a,
                0,
                TradeActionKind::Accept,
                "first-a",
            ),
            &first.auth_a,
            DiplomacyRelationship::Friendly,
            11,
            &mut world,
        )
        .unwrap();
    assert!(world.is_empty());
    assert_eq!(ledger.contract(&first_id).unwrap().acceptances.len(), 1);
    ledger
        .apply_action(
            action(
                &first_id,
                &first.auth_b,
                1,
                TradeActionKind::Accept,
                "first-b",
            ),
            &first.auth_b,
            DiplomacyRelationship::Friendly,
            12,
            &mut world,
        )
        .unwrap();
    assert_eq!(world.len(), 2);
    assert_eq!(
        ledger.contract(&first_id).unwrap().stage,
        TradeStage::Escrowed
    );

    let second = fixture(
        2,
        "shared-source-a",
        "source-b-2",
        DiplomacyRelationship::Friendly,
        20,
    );
    let second_id = second.proposal.contract_id.clone();
    ledger
        .propose(second.proposal.clone(), &second.auth_a)
        .unwrap();
    ledger
        .apply_action(
            action(
                &second_id,
                &second.auth_a,
                0,
                TradeActionKind::Accept,
                "second-a",
            ),
            &second.auth_a,
            DiplomacyRelationship::Friendly,
            21,
            &mut world,
        )
        .unwrap();
    let before_world = world.clone();
    let before_contract = ledger.contract(&second_id).unwrap().clone();
    assert!(matches!(
        ledger.apply_action(
            action(
                &second_id,
                &second.auth_b,
                1,
                TradeActionKind::Accept,
                "second-b"
            ),
            &second.auth_b,
            DiplomacyRelationship::Friendly,
            22,
            &mut world,
        ),
        Err(TradeError::Escrow(_))
    ));
    assert_eq!(world, before_world);
    assert_eq!(ledger.contract(&second_id).unwrap(), &before_contract);
}

#[test]
fn cargo_is_physical_until_atomic_counter_delivery_and_relationship_block_does_not_delete_it() {
    let fixture = fixture(
        3,
        "source-a-3",
        "source-b-3",
        DiplomacyRelationship::Friendly,
        10,
    );
    let mut ledger = TradeLedger::new();
    let mut world = WorldReservationLedger::new();
    let id = accept_both(&mut ledger, &mut world, &fixture);
    assert!(
        ledger
            .contract(&id)
            .unwrap()
            .proposal
            .legs
            .iter()
            .all(|leg| { matches!(leg.cargo.location, CargoLocation::ReservedAtSource { .. }) })
    );
    ledger.depart(&id, 2, 13, &world).unwrap();
    assert!(
        ledger
            .contract(&id)
            .unwrap()
            .proposal
            .legs
            .iter()
            .all(|leg| { matches!(leg.cargo.location, CargoLocation::Carried { .. }) })
    );
    ledger
        .attempt_delivery(&id, 3, DeliveryValidation::all_valid(), 14, &mut world)
        .unwrap();
    let contract = ledger.contract(&id).unwrap();
    assert_eq!(contract.stage, TradeStage::Complete);
    assert!(contract.proposal.legs.iter().all(|leg| {
        matches!(
            leg.cargo.location,
            CargoLocation::DepositedAtEndpoint { .. }
        )
    }));
    assert!(world.is_empty());
}

#[test]
fn cancellation_before_departure_releases_everything_but_after_departure_requires_recovery() {
    let before_departure = fixture(
        4,
        "source-a-4",
        "source-b-4",
        DiplomacyRelationship::Friendly,
        10,
    );
    let mut ledger = TradeLedger::new();
    let mut world = WorldReservationLedger::new();
    let id = accept_both(&mut ledger, &mut world, &before_departure);
    let cancel = action(
        &id,
        &before_departure.auth_a,
        2,
        TradeActionKind::Cancel,
        "cancel-before",
    );
    ledger
        .apply_action(
            cancel,
            &before_departure.auth_a,
            DiplomacyRelationship::Friendly,
            13,
            &mut world,
        )
        .unwrap();
    assert!(world.is_empty());
    assert_eq!(ledger.contract(&id).unwrap().stage, TradeStage::Cancelled);

    let fixture = fixture(
        5,
        "source-a-5",
        "source-b-5",
        DiplomacyRelationship::Friendly,
        20,
    );
    let id = accept_both(&mut ledger, &mut world, &fixture);
    ledger.depart(&id, 2, 23, &world).unwrap();
    let before = ledger.contract(&id).unwrap().clone();
    assert_eq!(
        ledger.apply_action(
            action(
                &id,
                &fixture.auth_a,
                3,
                TradeActionKind::Cancel,
                "cancel-after"
            ),
            &fixture.auth_a,
            DiplomacyRelationship::Blocked,
            24,
            &mut world,
        ),
        Err(TradeError::RecoveryRequired)
    );
    assert_eq!(ledger.contract(&id).unwrap(), &before);
}

#[test]
fn route_closure_returns_physically_or_strands_exact_stable_cargo_without_duplication() {
    let returning = fixture(
        6,
        "source-a-6",
        "source-b-6",
        DiplomacyRelationship::Friendly,
        10,
    );
    let mut ledger = TradeLedger::new();
    let mut world = WorldReservationLedger::new();
    let id = accept_both(&mut ledger, &mut world, &returning);
    ledger.depart(&id, 2, 13, &world).unwrap();
    let return_route = route("return-route", 5);
    ledger
        .close_route(
            &id,
            3,
            Some(&return_route),
            &BTreeMap::new(),
            14,
            &mut world,
        )
        .unwrap();
    assert_eq!(ledger.contract(&id).unwrap().stage, TradeStage::Returning);
    assert_eq!(
        ledger.contract(&id).unwrap().recovery,
        TradeRecoveryState::Returning
    );
    ledger.finish_return(&id, 4, &mut world).unwrap();
    assert!(world.is_empty());
    assert!(
        ledger
            .contract(&id)
            .unwrap()
            .proposal
            .legs
            .iter()
            .all(|leg| { matches!(leg.cargo.location, CargoLocation::ReservedAtSource { .. }) })
    );

    let stranded = fixture(
        7,
        "source-a-7",
        "source-b-7",
        DiplomacyRelationship::Friendly,
        20,
    );
    let id = accept_both(&mut ledger, &mut world, &stranded);
    ledger.depart(&id, 2, 23, &world).unwrap();
    let cargo_ids = ledger
        .contract(&id)
        .unwrap()
        .proposal
        .legs
        .iter()
        .map(|leg| leg.cargo.cargo_id.clone())
        .collect::<Vec<_>>();
    let sites = BTreeMap::from([
        (cargo_ids[0].clone(), "road-7-a".to_owned()),
        (cargo_ids[1].clone(), "road-7-b".to_owned()),
    ]);
    ledger
        .close_route(&id, 3, None, &sites, 24, &mut world)
        .unwrap();
    let contract = ledger.contract(&id).unwrap();
    assert_eq!(contract.stage, TradeStage::Stranded);
    assert_eq!(
        contract
            .proposal
            .legs
            .iter()
            .map(|leg| leg.cargo.quantity)
            .sum::<u64>(),
        2
    );
    assert!(
        contract
            .proposal
            .legs
            .iter()
            .all(|leg| { matches!(leg.cargo.location, CargoLocation::Stranded { .. }) })
    );
    assert!(world.is_empty());
}

#[test]
fn carrier_death_or_refusal_salvages_or_strands_without_completion() {
    let death_fixture = fixture(
        8,
        "source-a-8",
        "source-b-8",
        DiplomacyRelationship::Friendly,
        10,
    );
    let mut ledger = TradeLedger::new();
    let mut world = WorldReservationLedger::new();
    let id = accept_both(&mut ledger, &mut world, &death_fixture);
    ledger.depart(&id, 2, 13, &world).unwrap();
    let legs = ledger.contract(&id).unwrap().proposal.legs.clone();
    let safe = SiteRef::Stockpile {
        metadata: SiteMetadata::revealed("safe-owned"),
        stockpile_id: "safe-owned".into(),
        footprint: footprint(TilePoint { x: 50, y: 50 }),
    };
    let dispositions = legs
        .iter()
        .map(|leg| RecoveryDisposition {
            cargo_id: leg.cargo.cargo_id.clone(),
            safe_owned_stockpile: Some(safe.clone()),
            last_site_id: String::new(),
        })
        .collect::<Vec<_>>();
    ledger
        .carrier_failed(
            &id,
            3,
            TradeBlockReason::WorkerDied,
            &dispositions,
            14,
            &mut world,
        )
        .unwrap();
    let contract = ledger.contract(&id).unwrap();
    assert_eq!(contract.stage, TradeStage::Blocked);
    assert_eq!(contract.recovery, TradeRecoveryState::Salvaged);
    assert!(contract.proposal.legs.iter().all(|leg| {
        matches!(
            leg.cargo.location,
            CargoLocation::SalvagedAtStockpile { .. }
        )
    }));
    assert!(world.is_empty());

    let refused = fixture(
        80,
        "source-a-80",
        "source-b-80",
        DiplomacyRelationship::Friendly,
        20,
    );
    let refused_id = accept_both(&mut ledger, &mut world, &refused);
    ledger.depart(&refused_id, 2, 23, &world).unwrap();
    let dispositions = ledger
        .contract(&refused_id)
        .unwrap()
        .proposal
        .legs
        .iter()
        .map(|leg| RecoveryDisposition {
            cargo_id: leg.cargo.cargo_id.clone(),
            safe_owned_stockpile: Some(safe.clone()),
            last_site_id: String::new(),
        })
        .collect::<Vec<_>>();
    ledger
        .carrier_failed(
            &refused_id,
            3,
            TradeBlockReason::WorkerRefused,
            &dispositions,
            24,
            &mut world,
        )
        .unwrap();
    assert_eq!(
        ledger.contract(&refused_id).unwrap().blocked_reason,
        Some(TradeBlockReason::WorkerRefused)
    );
    assert!(world.is_empty());
}

#[test]
fn destination_failure_keeps_pinned_endpoint_and_in_flight_block_allows_recovery() {
    let full_fixture = fixture(
        9,
        "source-a-9",
        "source-b-9",
        DiplomacyRelationship::Friendly,
        10,
    );
    let mut ledger = TradeLedger::new();
    let mut world = WorldReservationLedger::new();
    let id = accept_both(&mut ledger, &mut world, &full_fixture);
    ledger.depart(&id, 2, 13, &world).unwrap();
    let endpoints = ledger
        .contract(&id)
        .unwrap()
        .proposal
        .legs
        .iter()
        .map(|leg| leg.spatial.delivery_endpoint().stable_id().to_owned())
        .collect::<Vec<_>>();
    ledger
        .attempt_delivery(
            &id,
            3,
            DeliveryValidation {
                destination_capacity_available: false,
                ..DeliveryValidation::all_valid()
            },
            14,
            &mut world,
        )
        .unwrap();
    let contract = ledger.contract(&id).unwrap();
    assert_eq!(contract.stage, TradeStage::Blocked);
    assert_eq!(
        contract.blocked_reason,
        Some(TradeBlockReason::DestinationFull)
    );
    assert_eq!(
        contract
            .proposal
            .legs
            .iter()
            .map(|leg| leg.spatial.delivery_endpoint().stable_id().to_owned())
            .collect::<Vec<_>>(),
        endpoints
    );
    assert_eq!(world.len(), 2);
    let return_route = route("blocked-return", 6);
    ledger
        .close_route(
            &id,
            4,
            Some(&return_route),
            &BTreeMap::new(),
            15,
            &mut world,
        )
        .unwrap();
    ledger.finish_return(&id, 5, &mut world).unwrap();
    assert!(world.is_empty());

    let removed = fixture(
        90,
        "source-a-90",
        "source-b-90",
        DiplomacyRelationship::Friendly,
        20,
    );
    let removed_id = accept_both(&mut ledger, &mut world, &removed);
    ledger.depart(&removed_id, 2, 23, &world).unwrap();
    ledger
        .attempt_delivery(
            &removed_id,
            3,
            DeliveryValidation {
                destination_exists: false,
                ..DeliveryValidation::all_valid()
            },
            24,
            &mut world,
        )
        .unwrap();
    assert_eq!(
        ledger.contract(&removed_id).unwrap().blocked_reason,
        Some(TradeBlockReason::DestinationRemoved)
    );
}

#[test]
fn due_order_is_next_tick_then_global_contract_id_and_input_order_independent() {
    let fixtures = vec![
        fixture(12, "a-12", "b-12", DiplomacyRelationship::Friendly, 8),
        fixture(10, "a-10", "b-10", DiplomacyRelationship::Friendly, 5),
        fixture(11, "a-11", "b-11", DiplomacyRelationship::Friendly, 5),
    ];
    let mut forward = TradeLedger::new();
    let mut reverse = TradeLedger::new();
    for fixture in &fixtures {
        forward
            .propose(fixture.proposal.clone(), &fixture.auth_a)
            .unwrap();
    }
    for fixture in fixtures.iter().rev() {
        reverse
            .propose(fixture.proposal.clone(), &fixture.auth_a)
            .unwrap();
    }
    assert_eq!(forward.due_contract_ids(8), reverse.due_contract_ids(8));
    let due = forward.due_contract_ids(8);
    let mut expected = fixtures
        .iter()
        .map(|fixture| {
            (
                fixture.proposal.created_tick,
                fixture.proposal.contract_id.clone(),
            )
        })
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(
        due,
        expected.into_iter().map(|(_, id)| id).collect::<Vec<_>>()
    );
    assert_eq!(
        serde_json::to_string(&forward).unwrap(),
        serde_json::to_string(&reverse).unwrap()
    );
    let mut unordered = serde_json::to_value(&forward).unwrap();
    unordered["contracts"].as_array_mut().unwrap().reverse();
    assert!(serde_json::from_value::<TradeLedger>(unordered).is_err());
}

#[test]
fn action_replay_is_idempotent_and_stale_or_cross_colony_actions_do_not_mutate() {
    let fixture = fixture(
        13,
        "source-a-13",
        "source-b-13",
        DiplomacyRelationship::Friendly,
        10,
    );
    let mut ledger = TradeLedger::new();
    let mut world = WorldReservationLedger::new();
    let id = ledger
        .propose(fixture.proposal.clone(), &fixture.auth_a)
        .unwrap();
    let accept = action(&id, &fixture.auth_a, 0, TradeActionKind::Accept, "same");
    let first = ledger
        .apply_action(
            accept.clone(),
            &fixture.auth_a,
            DiplomacyRelationship::Friendly,
            11,
            &mut world,
        )
        .unwrap();
    let version = ledger.version();
    assert_eq!(
        ledger
            .apply_action(
                accept,
                &fixture.auth_a,
                DiplomacyRelationship::Friendly,
                11,
                &mut world,
            )
            .unwrap(),
        first
    );
    assert_eq!(ledger.version(), version);
    assert!(matches!(
        ledger.apply_action(
            action(&id, &fixture.auth_b, 0, TradeActionKind::Accept, "stale"),
            &fixture.auth_b,
            DiplomacyRelationship::Friendly,
            12,
            &mut world,
        ),
        Err(TradeError::StaleVersion { .. })
    ));
    assert_eq!(ledger.contract(&id).unwrap().acceptances.len(), 1);

    let other = fixture_for_colonies(
        1,
        "source-c",
        "source-d",
        DiplomacyRelationship::Friendly,
        20,
        "colony-c",
        "colony-d",
    );
    let other_id = ledger
        .propose(other.proposal.clone(), &other.auth_a)
        .unwrap();
    let other_before = ledger.contract(&other_id).unwrap().clone();
    assert_eq!(
        ledger.apply_action(
            action(&id, &other.auth_a, 1, TradeActionKind::Accept, "cross-pair"),
            &other.auth_a,
            DiplomacyRelationship::Friendly,
            21,
            &mut world,
        ),
        Err(TradeError::AuthorizationColonyMismatch)
    );
    assert_eq!(ledger.contract(&other_id).unwrap(), &other_before);
}

#[test]
fn strict_restart_preserves_in_transit_state_and_rejects_tampering() {
    let fixture = fixture(
        14,
        "source-a-14",
        "source-b-14",
        DiplomacyRelationship::Friendly,
        10,
    );
    let mut ledger = TradeLedger::new();
    let mut world = WorldReservationLedger::new();
    let id = accept_both(&mut ledger, &mut world, &fixture);
    ledger.depart(&id, 2, 13, &world).unwrap();
    let trade_json = serde_json::to_string(&ledger).unwrap();
    let world_json = serde_json::to_string(&world).unwrap();
    let trade_wire: serde_json::Value = serde_json::from_str(&trade_json).unwrap();
    assert!(
        trade_wire["contracts"][0]["proposal"]["parties"].is_array(),
        "structured diplomacy IDs must persist as canonical entries, never JSON object keys"
    );
    assert!(
        trade_wire["contracts"][0]["proposal"]["valuations"].is_array(),
        "report-safe valuations must persist with their structured colony IDs"
    );
    let mut duplicate_party = trade_wire.clone();
    let party = duplicate_party["contracts"][0]["proposal"]["parties"][0].clone();
    duplicate_party["contracts"][0]["proposal"]["parties"]
        .as_array_mut()
        .unwrap()
        .push(party);
    assert!(serde_json::from_value::<TradeLedger>(duplicate_party).is_err());
    let mut duplicate_valuation = trade_wire.clone();
    let valuation = duplicate_valuation["contracts"][0]["proposal"]["valuations"][0].clone();
    duplicate_valuation["contracts"][0]["proposal"]["valuations"]
        .as_array_mut()
        .unwrap()
        .push(valuation);
    assert!(serde_json::from_value::<TradeLedger>(duplicate_valuation).is_err());
    let restored_trade: TradeLedger = serde_json::from_str(&trade_json).unwrap();
    let restored_world: WorldReservationLedger = serde_json::from_str(&world_json).unwrap();
    assert_eq!(restored_trade, ledger);
    assert_eq!(restored_world, world);
    assert_eq!(
        restored_trade.contract(&id).unwrap().stage,
        TradeStage::InTransit
    );

    let empty: TradeLedger = serde_json::from_str(r#"{"schemaVersion":1}"#).unwrap();
    assert!(empty.is_empty());
    let mut unknown = serde_json::to_value(&ledger).unwrap();
    unknown["schemaVersion"] = 99.into();
    assert!(serde_json::from_value::<TradeLedger>(unknown).is_err());
    let mut duplicate = serde_json::to_value(&ledger).unwrap();
    let contract = duplicate["contracts"][0].clone();
    duplicate["contracts"]
        .as_array_mut()
        .unwrap()
        .push(contract);
    assert!(serde_json::from_value::<TradeLedger>(duplicate).is_err());
    let mut missing_cargo = serde_json::to_value(&ledger).unwrap();
    missing_cargo["contracts"][0]["proposal"]["legs"][0]["cargo"]["quantity"] = 0.into();
    assert!(serde_json::from_value::<TradeLedger>(missing_cargo).is_err());
}
