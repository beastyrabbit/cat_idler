//! Focused LAI.62 static contract tests.
//!
//! This test includes the additive leaf directly because LAI.62 does not own
//! `cat-sim/src/lib.rs`; production wiring belongs to LAI.63–LAI.70 owners.

#[path = "../src/moneyless_barter.rs"]
mod moneyless_barter;

use moneyless_barter::*;

fn colony(name: &str) -> ColonyId {
    ColonyId::derive(name)
}

fn lot(name: &str, kind: PhysicalLotKind, quantity: u64) -> PhysicalLot {
    PhysicalLot::new(
        StableId::derive("test-lot", &[name]),
        kind,
        quantity,
        900_000,
    )
    .unwrap()
}

#[test]
fn stable_ids_and_stances_are_versioned_and_canonical() {
    assert_eq!(MONEYLESS_BARTER_SCHEMA_VERSION, 1);
    assert_eq!(STABLE_ID_CONTRACT_VERSION, 1);
    let a = colony("a");
    let b = colony("b");
    let snapshot = StanceSnapshot::new(vec![
        StanceRecord::new(a.clone(), b.clone(), PersonalStance::Alliance).unwrap(),
        StanceRecord::new(b.clone(), a.clone(), PersonalStance::Neutral).unwrap(),
    ])
    .unwrap();
    assert_eq!(snapshot.stance(&a, &b), PersonalStance::Alliance);
    assert_eq!(
        PersonalStance::Alliance.trade_label(),
        "Alliance (trade-equivalent to Neutral)"
    );
    assert!(PersonalStance::Alliance.trade_allowed());
    assert!(PersonalStance::Neutral.trade_allowed());
    assert!(PersonalStance::Enemy.is_enemy());
    assert!(snapshot.records[0].source <= snapshot.records[1].source);
}

#[test]
fn global_village_is_locked_neutral() {
    let global = ColonyId::global();
    let village = colony("village");
    assert!(StanceRecord::new(global.clone(), village.clone(), PersonalStance::Alliance).is_err());
    let record = StanceRecord::new(global, village, PersonalStance::Neutral).unwrap();
    assert_eq!(record.stance, PersonalStance::Neutral);
}

#[test]
fn enemy_is_filtered_and_destination_enemy_rejects_before_dispatch() {
    let source = colony("source");
    let friendly = colony("friendly");
    let enemy = colony("enemy");
    let snapshot = StanceSnapshot::new(vec![
        StanceRecord::new(source.clone(), enemy.clone(), PersonalStance::Enemy).unwrap(),
        StanceRecord::new(enemy.clone(), source.clone(), PersonalStance::Enemy).unwrap(),
    ])
    .unwrap();
    let selected = outbound_candidates(&source, &snapshot, vec![enemy.clone(), friendly.clone()]);
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].destination, friendly);
    assert_eq!(
        pre_dispatch_gate(DispatchRequest {
            source: source.clone(),
            destination: enemy,
            source_stance: PersonalStance::Neutral,
            destination_stance: PersonalStance::Enemy,
        }),
        Err(DispatchRejection::DestinationMarksSenderEnemy)
    );
}

#[test]
fn offers_and_contracts_are_physical_and_rejection_has_no_contract_path() {
    let source = colony("source");
    let destination = colony("destination");
    let permit = pre_dispatch_gate(DispatchRequest {
        source: source.clone(),
        destination: destination.clone(),
        source_stance: PersonalStance::Alliance,
        destination_stance: PersonalStance::Neutral,
    })
    .unwrap();
    let offer = BarterOffer::new(
        &permit,
        StableId::derive("offer", &["one"]),
        vec![lot(
            "wood",
            PhysicalLotKind::Material {
                resource_id: "wood".into(),
            },
            4,
        )],
        vec![lot(
            "bread",
            PhysicalLotKind::TypedFood {
                food_id: "bread".into(),
            },
            2,
        )],
    )
    .unwrap();
    let contract =
        BarterContract::propose(&permit, offer, StableId::derive("contract", &["one"])).unwrap();
    assert_eq!(contract.stage, ContractStage::Proposed);
    assert!(contract.legs.is_empty());
    assert_eq!(contract.offer.offered[0].quantity, 4);
    assert_eq!(contract.offer.requested[0].quantity, 2);
    assert!(
        pre_dispatch_gate(DispatchRequest {
            source,
            destination,
            source_stance: PersonalStance::Enemy,
            destination_stance: PersonalStance::Neutral,
        })
        .is_err()
    );
}

#[test]
fn score_uses_all_report_safe_fixed_point_terms_and_postures() {
    let metric = |estimate| ReportMetric::new(estimate).unwrap();
    let inputs = TradeScoreInputs {
        source_need: metric(900_000),
        destination_offerings: metric(800_000),
        quality: metric(700_000),
        utility: metric(700_000),
        exchange_value: metric(800_000),
        distance_premium: metric(100_000),
        travel_time: metric(100_000),
        route_risk: metric(100_000),
        carrying_cost: metric(100_000),
        carrying_capacity: metric(300_000),
        opportunity_cost: metric(100_000),
    };
    let decision = choose_posture(inputs).unwrap();
    assert_eq!(decision.posture, Some(TradePosture::PossibleNow));
    assert!(decision.score.net >= 0);
    let worse = TradeScoreInputs {
        distance_premium: metric(900_000),
        travel_time: metric(900_000),
        route_risk: metric(900_000),
        carrying_cost: metric(900_000),
        opportunity_cost: metric(900_000),
        ..inputs
    };
    assert_eq!(choose_posture(worse).unwrap().posture, None);
}

#[test]
fn physical_recovery_is_an_adapter_command_and_restart_conservation_is_explicit() {
    let source = colony("source");
    let destination = colony("destination");
    let leg = PhysicalTransferLeg {
        lot_id: StableId::derive("lot", &["one"]),
        owner: source,
        recipient: destination,
        source_endpoint_id: StableId::derive("source", &["one"]),
        destination_endpoint_id: StableId::derive("destination", &["one"]),
        reservation_id: StableId::derive("reservation", &["one"]),
        escrow_id: StableId::derive("escrow", &["one"]),
        hauler_id: Some(StableId::derive("hauler", &["one"])),
        route_id: Some(StableId::derive("route", &["one"])),
        location: PhysicalLocation::InTransit,
        quantity: 3,
    };
    let commands = recovery_commands(
        StableId::derive("contract", &["one"]),
        &leg,
        RecoveryReason::WorkerDied,
        None,
    );
    assert!(matches!(commands[0], PhysicalBarterCommand::Return { .. }));
    assert!(matches!(commands[1], PhysicalBarterCommand::Strand { .. }));
    assert_eq!(leg.quantity, 3);
    let before = RestartConservationSnapshot {
        contract_id: StableId::derive("contract", &["one"]),
        lots: vec![RestartLotSnapshot {
            lot_id: leg.lot_id.clone(),
            quantity: leg.quantity,
            location: PhysicalLocation::InTransit,
        }],
    };
    let after = RestartConservationSnapshot {
        lots: vec![RestartLotSnapshot {
            location: PhysicalLocation::Returning,
            ..before.lots[0].clone()
        }],
        ..before.clone()
    };
    assert!(validate_restart_conservation(&before, &after).is_ok());
}

#[test]
fn canonical_value_is_comparison_only_and_cutover_inventory_is_explicit() {
    let comparison = canonical_comparison_value(CanonicalValueInput {
        base_units: 10,
        quantity: 4,
        quality_bps: 900_000,
        utility_bps: 800_000,
    })
    .unwrap();
    assert_eq!(comparison.units(), 28);
    assert!(
        LAI70_LEGACY_CUTOVER_INVENTORY
            .iter()
            .any(|root| root.root_id == "coin")
    );
    assert!(
        LAI70_LEGACY_CUTOVER_INVENTORY
            .iter()
            .any(|root| root.root_id == "old-npc-trade-root")
    );
}
