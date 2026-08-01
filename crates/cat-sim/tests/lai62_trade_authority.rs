//! Focused LAI.62-A acceptance harness.  The coordinator wires this module
//! through `cat_sim::trade_authority` and owns the serialized test gate.

use std::collections::{BTreeMap, BTreeSet};

use cat_sim::{
    authority::AuthorityActor,
    autonomous_trade::{
        DeliveryValidation, RecoveryDisposition, TradeAction, TradeActionId, TradeActionKind,
        TradeAuthorization, TradeBlockReason, TradeCargoLeg, TradeColonyKind, TradeContractId,
        TradeParty, TradeProposal, TradeStage,
    },
    beliefs::{
        BeliefKey, BeliefKind, BeliefProjection, Confidence, EstimateRange, EvidenceId,
        EvidenceSource, ProjectedBeliefValue, ReportLevel,
    },
    content_manifest::{ContentId, PhysicalLotId},
    diplomacy::DiplomacyColonyId,
    diplomacy::{DiplomacyPair, DiplomacyRelationship},
    moneyless_barter::{
        BarterContract, BarterOffer, ColonyId, DispatchRequest, PersonalStance,
        PhysicalLot as BarterLot, PhysicalLotKind, ReportMetric, StableId, TradePosture,
        TradeScoreInputs, pre_dispatch_gate,
    },
    physical_storage::StorageCompatibility,
    planner_core::{IntentId, PlannerId},
    quality_lots::{BulkLotKey, LotLocation, LotProvenance, PhysicalLot, QualityBand},
    spatial_resolver::{
        ResolvedSpatialTask, SpatialResolutionCandidate, SpatialResolutionOutcome,
        SpatialResolutionRequest, SpatialTaskCategory, resolve_spatial_task,
    },
    spatial_tasks::{
        Rect, ResourceSourceKind, SiteMetadata, SiteRef, TaskFootprint, TilePoint, WorkSlot,
    },
    storage_authority::{
        StorageAddress, StorageAuthority, StorageCommand, StorageCommandEnvelope, StorageIdentity,
        StorageZone, StorageZoneKind,
    },
    task_runtime::CargoLocation,
    trade_authority::{
        LAI63_LAI70_ADAPTERS_TO_DELETE, TradeAuthority, TradeAuthorityError, TradeContentLotBinding,
    },
    trade_valuation::{TradePersonality, TradePurpose, TradeValuation},
    world_reservations::{
        CapacityReservation, WorldReservationLedger, WorldReservationTransaction,
    },
};

fn village(id: &str) -> DiplomacyColonyId {
    DiplomacyColonyId::derive(id)
}

fn id(namespace: &str, value: &str) -> PlannerId {
    PlannerId::derive(namespace, [value])
}

fn projection(label: &str, value: i64) -> BeliefProjection {
    let key = BeliefKey::new(
        id("trade-domain", label),
        id("trade-subject", label),
        BeliefKind::Stock,
    );
    let reporter = id("cat", &format!("reporter-{label}"));
    BeliefProjection {
        key: key.clone(),
        value: ProjectedBeliefValue::StockRange(EstimateRange::new(value, value, value).unwrap()),
        confidence: Confidence::new(8_000).unwrap(),
        observed_tick: 5,
        expires_tick: Some(100),
        source: EvidenceSource::AuthorizedOfficerReport,
        reporter_id: reporter.clone(),
        evidence_ids: BTreeSet::from([EvidenceId::derive(label, &key, 5, &reporter, 0)]),
        report_level: ReportLevel::Three,
    }
}

fn valuation(offered: i64, requested: i64) -> TradeValuation {
    TradeValuation::evaluate(
        DiplomacyRelationship::Friendly,
        TradePurpose::Ordinary,
        TradePersonality::Balanced,
        &projection("offered", offered),
        &projection("requested", requested),
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
            work_slot: WorkSlot::exclusive(
                format!("slot-{source_id}"),
                SiteRef::Tile {
                    metadata: SiteMetadata::revealed(format!("bank-{source_id}")),
                    tile: TilePoint { x, y: 1 },
                },
            ),
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
    reservation_colony: PlannerId,
    source_id: &str,
    occurrence: u32,
    x: i32,
) -> TradeCargoLeg {
    let spatial = resolved_spatial(source_id, &format!("endpoint-{source_id}"), x);
    let hauler_id = id("cat", &format!("hauler-{source_id}-{occurrence}"));
    let resource_id = format!("goods_{source_id}");
    let escrow = WorldReservationTransaction::new(
        reservation_colony,
        id("trade-task", &format!("{source_id}-{occurrence}")),
        IntentId::derive(owner.as_str(), u64::from(occurrence), "trade", source_id, 0),
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

fn fixture(occurrence: u32) -> Fixture {
    let colony_a = village("colony-a");
    let colony_b = village("colony-b");
    let pair = DiplomacyPair::new(colony_a.clone(), colony_b.clone()).unwrap();
    let reservation_a = id("colony", "colony-a");
    let reservation_b = id("colony", "colony-b");
    let actor_a = AuthorityActor::Leader {
        cat_id: id("cat", "leader-a"),
    };
    let actor_b = AuthorityActor::Leader {
        cat_id: id("cat", "leader-b"),
    };
    let proposal = TradeProposal::new(
        pair,
        colony_a.clone(),
        occurrence,
        BTreeMap::from([
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
        ]),
        DiplomacyRelationship::Friendly,
        TradePurpose::Ordinary,
        BTreeMap::from([
            (colony_a.clone(), valuation(100, 95)),
            (colony_b.clone(), valuation(95, 100)),
        ]),
        vec![
            leg(
                &colony_a,
                &colony_b,
                reservation_a,
                "source-a",
                occurrence,
                0,
            ),
            leg(
                &colony_b,
                &colony_a,
                reservation_b,
                "source-b",
                occurrence,
                20,
            ),
        ],
        10,
        110,
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

fn content_contract() -> (BarterContract, Vec<TradeContentLotBinding>) {
    let source = ColonyId::derive("colony-a");
    let destination = ColonyId::derive("colony-b");
    let permit = pre_dispatch_gate(DispatchRequest {
        source: source.clone(),
        destination: destination.clone(),
        source_stance: PersonalStance::Neutral,
        destination_stance: PersonalStance::Alliance,
    })
    .unwrap();
    let offered_id = StableId::derive("trade-content", &["a"]);
    let requested_id = StableId::derive("trade-content", &["b"]);
    let offer = BarterOffer::new(
        &permit,
        StableId::derive("trade-offer", &["fixture"]),
        vec![
            BarterLot::new(
                offered_id.clone(),
                PhysicalLotKind::Material {
                    resource_id: "goods_source_a".to_owned(),
                },
                1,
                1_000_000,
            )
            .unwrap(),
        ],
        vec![
            BarterLot::new(
                requested_id.clone(),
                PhysicalLotKind::Material {
                    resource_id: "goods_source_b".to_owned(),
                },
                1,
                1_000_000,
            )
            .unwrap(),
        ],
    )
    .unwrap();
    let contract = BarterContract::propose(
        &permit,
        offer,
        StableId::derive("trade-content-contract", &["fixture"]),
    )
    .unwrap();
    let bindings = vec![
        TradeContentLotBinding {
            content_lot_id: offered_id,
            storage_identity: StorageIdentity::Lot(PhysicalLotId::new("lot_a").unwrap()),
        },
        TradeContentLotBinding {
            content_lot_id: requested_id,
            storage_identity: StorageIdentity::Lot(PhysicalLotId::new("lot_b").unwrap()),
        },
    ];
    (contract, bindings)
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

fn storage() -> StorageAuthority {
    let mut storage = StorageAuthority::new("colony_one").unwrap();
    let zone = StorageZone::new(
        "zone",
        StorageZoneKind::Stockpile,
        TaskFootprint::rectangular(Rect::new(TilePoint { x: 0, y: 0 }, 1, 1).unwrap()),
    )
    .unwrap();
    storage
        .execute(StorageCommandEnvelope {
            colony_id: "colony_one".to_owned(),
            command_id: "register".to_owned(),
            fingerprint: "register-v1".to_owned(),
            sequence: 1,
            command: StorageCommand::RegisterZone { zone },
        })
        .unwrap();
    for (sequence, id) in [(2, "lot_a"), (3, "lot_b")] {
        storage
            .execute(StorageCommandEnvelope {
                colony_id: "colony_one".to_owned(),
                command_id: format!("deposit-{id}"),
                fingerprint: format!("deposit-{id}-v1"),
                sequence,
                command: StorageCommand::DepositLot {
                    lot: PhysicalLot {
                        id: PhysicalLotId::new(id).unwrap(),
                        key: BulkLotKey::new(
                            ContentId::new("resource_logs").unwrap(),
                            QualityBand::Fine,
                        ),
                        provenance: LotProvenance {
                            origin: "gathering:forest".to_owned(),
                            created_tick: 1,
                        },
                        quantity: 1,
                        location: LotLocation::Source("forest".to_owned()),
                        reservation: None,
                    },
                    compatibility: StorageCompatibility::BulkMaterial,
                    destination: StorageAddress::Loose {
                        zone_id: "zone".to_owned(),
                        tile: TilePoint { x: 0, y: 0 },
                        slot: (sequence - 2) as u8,
                    },
                },
            })
            .unwrap();
    }
    storage
}

fn propose(authority: &mut TradeAuthority, fixture: &Fixture, command: &str) -> TradeContractId {
    let (content, bindings) = content_contract();
    authority
        .propose(
            command,
            format!("{command}-v1"),
            authority.version(),
            fixture.proposal.clone(),
            content,
            bindings,
            &fixture.auth_a,
        )
        .unwrap()
        .contract_id
        .unwrap()
}

#[test]
fn global_village_is_always_neutral_and_rejects_a_non_neutral_write() {
    let global = village("global-village");
    let local = village("local");
    let mut authority = TradeAuthority::new();
    assert_eq!(authority.stance(&global, &local), PersonalStance::Neutral);
    assert_eq!(
        authority.set_stance(
            "global-write",
            "global-write-v1",
            0,
            global.clone(),
            local.clone(),
            PersonalStance::Enemy
        ),
        Err(TradeAuthorityError::GlobalVillageLockedNeutral),
    );
    assert_eq!(authority.version(), 0);
}

#[test]
fn alliance_and_neutral_are_honest_trade_twins_at_the_stance_boundary() {
    let source = village("source");
    let destination = village("destination");
    let mut neutral = TradeAuthority::new();
    let neutral_receipt = neutral
        .set_stance(
            "n",
            "n-v1",
            0,
            source.clone(),
            destination.clone(),
            PersonalStance::Neutral,
        )
        .unwrap();
    let mut alliance = TradeAuthority::new();
    let alliance_receipt = alliance
        .set_stance(
            "a",
            "a-v1",
            0,
            source.clone(),
            destination.clone(),
            PersonalStance::Alliance,
        )
        .unwrap();
    assert_eq!(
        neutral_receipt.resulting_version,
        alliance_receipt.resulting_version
    );
    assert!(neutral.stance(&source, &destination).trade_allowed());
    assert!(alliance.stance(&source, &destination).trade_allowed());
    assert_eq!(
        PersonalStance::Alliance.trade_label(),
        "Alliance (trade-equivalent to Neutral)"
    );
}

#[test]
fn command_replay_conflict_and_restart_keep_the_one_directional_stance() {
    let source = village("source");
    let destination = village("destination");
    let mut authority = TradeAuthority::new();
    let first = authority
        .set_stance(
            "stance",
            "stance-v1",
            0,
            source.clone(),
            destination.clone(),
            PersonalStance::Alliance,
        )
        .unwrap();
    let replay = authority
        .set_stance(
            "stance",
            "stance-v1",
            0,
            source.clone(),
            destination.clone(),
            PersonalStance::Alliance,
        )
        .unwrap();
    assert_eq!(first, replay);
    assert_eq!(authority.version(), 1);
    assert_eq!(
        authority.set_stance(
            "stance",
            "different",
            1,
            source.clone(),
            destination.clone(),
            PersonalStance::Neutral
        ),
        Err(TradeAuthorityError::ReplayConflict),
    );
    let restored: TradeAuthority =
        serde_json::from_str(&serde_json::to_string(&authority).unwrap()).unwrap();
    assert_eq!(
        restored.stance(&source, &destination),
        PersonalStance::Alliance
    );
    assert_eq!(restored.version(), authority.version());
}

#[test]
fn directional_partitions_do_not_change_unrelated_villages() {
    let source = village("source");
    let destination = village("destination");
    let other = village("other");
    let mut authority = TradeAuthority::new();
    authority
        .set_stance(
            "only-one",
            "only-one-v1",
            0,
            source.clone(),
            destination.clone(),
            PersonalStance::Enemy,
        )
        .unwrap();
    assert_eq!(
        authority.stance(&source, &destination),
        PersonalStance::Enemy
    );
    assert_eq!(authority.stance(&source, &other), PersonalStance::Neutral);
    assert_eq!(authority.stance(&other, &source), PersonalStance::Neutral);
}

#[test]
fn physical_content_bindings_and_mutual_consent_create_one_contract() {
    let fixture = fixture(1);
    let mut authority = TradeAuthority::new();
    let contract_id = propose(&mut authority, &fixture, "propose");
    let storage = storage();
    let bindings = authority.content_lots(&contract_id).unwrap();
    assert_eq!(bindings.len(), 2);
    assert!(
        bindings
            .iter()
            .all(|binding| authority.verify_storage_binding(&storage, binding))
    );
    let mut world = WorldReservationLedger::new();
    authority
        .apply_action(
            action(
                &contract_id,
                &fixture.auth_a,
                0,
                TradeActionKind::Accept,
                "a",
            ),
            &fixture.auth_a,
            11,
            &mut world,
        )
        .unwrap();
    assert!(world.is_empty());
    authority
        .apply_action(
            action(
                &contract_id,
                &fixture.auth_b,
                1,
                TradeActionKind::Accept,
                "b",
            ),
            &fixture.auth_b,
            12,
            &mut world,
        )
        .unwrap();
    assert_eq!(
        authority.contract(&contract_id).unwrap().stage,
        TradeStage::Escrowed
    );
    assert_eq!(world.len(), 2);
}

#[test]
fn enemy_rejection_precedes_contract_content_receipt_and_world_mutation() {
    let fixture = fixture(2);
    let mut authority = TradeAuthority::new();
    authority
        .set_stance(
            "enemy",
            "enemy-v1",
            0,
            fixture.auth_a.acting_colony.clone(),
            fixture.auth_b.acting_colony.clone(),
            PersonalStance::Enemy,
        )
        .unwrap();
    let before = authority.clone();
    let world = WorldReservationLedger::new();
    let (content, bindings) = content_contract();
    assert_eq!(
        authority.propose(
            "blocked",
            "blocked-v1",
            authority.version(),
            fixture.proposal.clone(),
            content,
            bindings,
            &fixture.auth_a
        ),
        Err(TradeAuthorityError::EnemyRejected),
    );
    assert_eq!(authority, before);
    assert!(world.is_empty());
}

#[test]
fn authority_wrapper_dispatches_matched_haulers_and_delivers_exactly_once() {
    let fixture = fixture(3);
    let mut authority = TradeAuthority::new();
    let contract_id = propose(&mut authority, &fixture, "flow");
    let mut world = WorldReservationLedger::new();
    authority
        .apply_action(
            action(
                &contract_id,
                &fixture.auth_a,
                0,
                TradeActionKind::Accept,
                "a",
            ),
            &fixture.auth_a,
            11,
            &mut world,
        )
        .unwrap();
    authority
        .apply_action(
            action(
                &contract_id,
                &fixture.auth_b,
                1,
                TradeActionKind::Accept,
                "b",
            ),
            &fixture.auth_b,
            12,
            &mut world,
        )
        .unwrap();
    authority.depart(&contract_id, 2, 13, &world).unwrap();
    let departed = authority.contract(&contract_id).unwrap();
    assert_eq!(departed.stage, TradeStage::InTransit);
    assert_eq!(departed.proposal.legs.len(), 2);
    assert!(
        departed
            .proposal
            .legs
            .iter()
            .all(|leg| matches!(leg.cargo.location, CargoLocation::Carried { .. }))
    );
    let quantity_before = departed
        .proposal
        .legs
        .iter()
        .map(|leg| leg.cargo.quantity)
        .sum::<u64>();
    authority
        .attempt_delivery(
            &contract_id,
            3,
            DeliveryValidation::all_valid(),
            14,
            &mut world,
        )
        .unwrap();
    let complete = authority.contract(&contract_id).unwrap();
    assert_eq!(complete.stage, TradeStage::Complete);
    assert_eq!(
        complete
            .proposal
            .legs
            .iter()
            .map(|leg| leg.cargo.quantity)
            .sum::<u64>(),
        quantity_before
    );
    assert!(complete.proposal.legs.iter().all(|leg| matches!(
        leg.cargo.location,
        CargoLocation::DepositedAtEndpoint { .. }
    )));
    assert!(world.is_empty());
}

#[test]
fn carrier_failure_salvages_without_losing_bound_content_or_quantities() {
    let fixture = fixture(4);
    let mut authority = TradeAuthority::new();
    let contract_id = propose(&mut authority, &fixture, "recovery");
    let bindings_before = authority.content_lots(&contract_id).unwrap().to_vec();
    let mut world = WorldReservationLedger::new();
    authority
        .apply_action(
            action(
                &contract_id,
                &fixture.auth_a,
                0,
                TradeActionKind::Accept,
                "a",
            ),
            &fixture.auth_a,
            11,
            &mut world,
        )
        .unwrap();
    authority
        .apply_action(
            action(
                &contract_id,
                &fixture.auth_b,
                1,
                TradeActionKind::Accept,
                "b",
            ),
            &fixture.auth_b,
            12,
            &mut world,
        )
        .unwrap();
    authority.depart(&contract_id, 2, 13, &world).unwrap();
    let safe = SiteRef::Stockpile {
        metadata: SiteMetadata::revealed("safe"),
        stockpile_id: "safe".to_owned(),
        footprint: footprint(TilePoint { x: 50, y: 50 }),
    };
    let quantity_before = authority
        .contract(&contract_id)
        .unwrap()
        .proposal
        .legs
        .iter()
        .map(|leg| leg.cargo.quantity)
        .sum::<u64>();
    let dispositions = authority
        .contract(&contract_id)
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
    authority
        .carrier_failed(
            &contract_id,
            3,
            TradeBlockReason::WorkerDied,
            &dispositions,
            14,
            &mut world,
        )
        .unwrap();
    let recovered = authority.contract(&contract_id).unwrap();
    assert_eq!(
        recovered
            .proposal
            .legs
            .iter()
            .map(|leg| leg.cargo.quantity)
            .sum::<u64>(),
        quantity_before
    );
    assert!(recovered.proposal.legs.iter().all(|leg| matches!(
        leg.cargo.location,
        CargoLocation::SalvagedAtStockpile { .. }
    )));
    assert_eq!(
        authority.content_lots(&contract_id).unwrap(),
        bindings_before.as_slice()
    );
    assert!(world.is_empty());
}

#[test]
fn cancellation_replay_conflict_and_restart_preserve_contract_conservation() {
    let fixture = fixture(5);
    let mut authority = TradeAuthority::new();
    let contract_id = propose(&mut authority, &fixture, "cancel");
    let mut world = WorldReservationLedger::new();
    authority
        .apply_action(
            action(
                &contract_id,
                &fixture.auth_a,
                0,
                TradeActionKind::Accept,
                "a",
            ),
            &fixture.auth_a,
            11,
            &mut world,
        )
        .unwrap();
    authority
        .apply_action(
            action(
                &contract_id,
                &fixture.auth_b,
                1,
                TradeActionKind::Accept,
                "b",
            ),
            &fixture.auth_b,
            12,
            &mut world,
        )
        .unwrap();
    let cancel = action(
        &contract_id,
        &fixture.auth_a,
        2,
        TradeActionKind::Cancel,
        "cancel",
    );
    let first = authority
        .apply_action(cancel.clone(), &fixture.auth_a, 13, &mut world)
        .unwrap();
    let replay = authority
        .apply_action(cancel, &fixture.auth_a, 13, &mut world)
        .unwrap();
    assert_eq!(first, replay);
    assert_eq!(
        authority.contract(&contract_id).unwrap().stage,
        TradeStage::Cancelled
    );
    assert!(world.is_empty());
    let restored: TradeAuthority =
        serde_json::from_str(&serde_json::to_string(&authority).unwrap()).unwrap();
    assert_eq!(
        restored.contract(&contract_id),
        authority.contract(&contract_id)
    );
    assert_eq!(
        restored.content_lots(&contract_id),
        authority.content_lots(&contract_id)
    );
}

#[test]
fn set_stance_receipt_capacity_failure_rolls_back_the_overwrite() {
    let mut authority = TradeAuthority::new();
    let destination = village("destination");
    for index in 0..1_024_u64 {
        let source = village(&format!("source-{index}"));
        authority
            .set_stance(
                format!("command-{index}"),
                format!("fingerprint-{index}"),
                index,
                source,
                destination.clone(),
                PersonalStance::Neutral,
            )
            .unwrap();
    }
    let original = authority.stance(&village("source-0"), &destination);
    let before = authority.clone();
    assert_eq!(
        authority.set_stance(
            "overflow",
            "overflow-v1",
            authority.version(),
            village("source-0"),
            destination.clone(),
            PersonalStance::Alliance
        ),
        Err(TradeAuthorityError::TooManyReceipts),
    );
    assert_eq!(authority, before);
    assert_eq!(
        authority.stance(&village("source-0"), &destination),
        original
    );
}

#[test]
fn enemy_write_is_directional_and_has_no_contract_or_receipt_side_effect() {
    let source = village("source");
    let destination = village("destination");
    let mut authority = TradeAuthority::new();
    authority
        .set_stance(
            "enemy",
            "enemy-v1",
            0,
            source.clone(),
            destination.clone(),
            PersonalStance::Enemy,
        )
        .unwrap();
    assert_eq!(
        authority.authorize_dispatch(&source, &destination),
        Err(TradeAuthorityError::EnemyRejected)
    );
    assert_eq!(
        authority.stance(&source, &destination),
        PersonalStance::Enemy
    );
    assert_eq!(authority.summary().contract_count, 0);
    assert_eq!(authority.summary().active_contract_count, 0);
}

#[test]
fn report_only_scores_distinguish_close_possible_now_from_better_later() {
    let authority = TradeAuthority::new();
    let metric = |value| ReportMetric::new(value).unwrap();
    let close = authority
        .evaluate_posture(TradeScoreInputs {
            source_need: metric(700_000),
            destination_offerings: metric(700_000),
            quality: metric(200_000),
            utility: metric(200_000),
            exchange_value: metric(200_000),
            distance_premium: metric(10_000),
            travel_time: metric(10_000),
            route_risk: metric(10_000),
            carrying_cost: metric(10_000),
            carrying_capacity: metric(200_000),
            opportunity_cost: metric(10_000),
        })
        .unwrap();
    let distant = authority
        .evaluate_posture(TradeScoreInputs {
            source_need: metric(700_000),
            destination_offerings: metric(800_000),
            quality: metric(900_000),
            utility: metric(900_000),
            exchange_value: metric(900_000),
            distance_premium: metric(700_000),
            travel_time: metric(700_000),
            route_risk: metric(300_000),
            carrying_cost: metric(10_000),
            carrying_capacity: metric(200_000),
            opportunity_cost: metric(10_000),
        })
        .unwrap();
    assert_eq!(close.posture, Some(TradePosture::PossibleNow));
    assert_eq!(distant.posture, Some(TradePosture::BetterTrade));
}

#[test]
fn summary_is_report_safe_bounded_and_adapter_inventory_is_explicit() {
    let authority = TradeAuthority::new();
    assert_eq!(authority.summary().stance_count, 0);
    assert!(LAI63_LAI70_ADAPTERS_TO_DELETE.contains(&"crates/cat-sim/src/world_tick.rs"));
    assert!(LAI63_LAI70_ADAPTERS_TO_DELETE.contains(&"crates/cat-protocol/src/lai25_action.rs"));
}

// The following integration-shaped cases are intentionally kept as named
// acceptance targets for the coordinator's serialized gate.  Their fixtures
// are shared with autonomous_trade and exercise the authority's `propose`,
// `apply_action`, `depart`, `attempt_delivery`, and `carrier_failed` wrappers:
// * report-only possible-now/better-later scoring;
// * two-party consent, atomic escrow, matched haulers, and route stages;
// * route/death recovery with exact content identity conservation;
// * replay/conflict/restart and multi-colony partition isolation.
