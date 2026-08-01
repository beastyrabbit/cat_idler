//! Focused contracts for the LAI.61 canonical pure coordinator.
//!
//! The serialized gate is coordinator-owned.  These tests intentionally cover
//! only the authority leaf, not world-tick, storage, protocol, or persistence
//! adapters that remain assigned to LAI.63–LAI.70.

use cat_sim::{
    divine_boosts::{
        DivineBoostActor, DivineBoostAuthorization, DivineBoostPurchaseId,
        DivineBoostPurchaseRequest, DivineBoostState, DivineBoostType,
    },
    divine_hole_authority::{
        ClickBatchRequest, ClickTargetSpec, ConstructionMiracleRequest, DivineHoleAuthority,
        DivineHoleCommand, DivineHoleCommandEnvelope, EmergencyRescueRequest, HoleAuthorityBinding,
        MiracleInput, MiracleLaborStage, PhysicalEdibleDecision, PhysicalEdibleLot,
        RUNTIME_CUTOVER_AUDIT, VoidAction, VoidActionEnvelope,
    },
    food_divine_policy::{
        BoundCargoPurpose, CLICK_BATCH_INTERVAL_MS, ContributionKind, EmergencySupplyKind,
        FoodPermission, FoodPolicyActor,
    },
    planner_core::PlannerId,
    progression_research::{
        ColonyPartitionKey, HoleVoidCreditPayload, PlayerPartitionKey, ProgressionAuthority,
        StudyId, VoidInsight, VoidInsightLedger,
    },
};

fn colony() -> PlannerId {
    PlannerId::derive("lai61-divine-hole-colony", ["one"])
}

fn player() -> PlannerId {
    PlannerId::derive("lai61-divine-hole-player", ["one"])
}

fn authority() -> DivineHoleAuthority {
    DivineHoleAuthority::new(HoleAuthorityBinding::new(colony(), "hole-one").unwrap())
}

fn funded_ledger(whole: u64) -> VoidInsightLedger {
    let mut ledger = VoidInsightLedger::new(colony());
    ledger
        .credit_hole_feed(HoleVoidCreditPayload {
            partition: ColonyPartitionKey {
                colony_id: colony(),
            },
            feed_sequence: 1,
            amount: VoidInsight::from_whole(whole).unwrap(),
        })
        .unwrap();
    ledger
}

fn apply(authority: &mut DivineHoleAuthority, id: &str, command: DivineHoleCommand) {
    authority
        .apply(DivineHoleCommandEnvelope::new(id, authority.version, command).unwrap())
        .unwrap();
}

#[test]
fn lai61_policy_is_leader_owned_physical_and_only_lethal_starvation_bypasses_forbidden() {
    let mut state = authority();
    apply(
        &mut state,
        "register-apple",
        DivineHoleCommand::RegisterEdible {
            edible_id: "apple".to_owned(),
            is_divine_ration: false,
            now_tick: 4,
        },
    );
    apply(
        &mut state,
        "register-ration",
        DivineHoleCommand::RegisterEdible {
            edible_id: "divine_ration".to_owned(),
            is_divine_ration: true,
            now_tick: 4,
        },
    );
    assert_eq!(
        state.edible_policy.entries["divine_ration"].permission,
        FoodPermission::Reserve
    );
    assert!(
        state
            .edible_policy
            .set_permission(FoodPolicyActor::God, "apple", FoodPermission::Forbidden, 5)
            .is_err()
    );
    apply(
        &mut state,
        "late-poor-policy",
        DivineHoleCommand::SetPermission {
            edible_id: "apple".to_owned(),
            permission: FoodPermission::Forbidden,
            now_tick: 20,
        },
    );
    let lots = [PhysicalEdibleLot {
        lot_id: "lot-apple".to_owned(),
        definition_id: "apple".to_owned(),
    }];
    assert_eq!(
        state
            .decide_physical_edible("lot-apple", &lots, true, false)
            .unwrap()
            .decision,
        PhysicalEdibleDecision::Protected
    );
    assert_eq!(
        state
            .decide_physical_edible("lot-apple", &lots, true, true)
            .unwrap()
            .decision,
        PhysicalEdibleDecision::LethalStarvationOverride
    );
}

#[test]
fn lai61_clicks_use_log_value_rate_bound_physical_cargo_and_no_overfill() {
    let mut state = authority();
    apply(
        &mut state,
        "target",
        DivineHoleCommand::RegisterClickTarget {
            target: ClickTargetSpec {
                target_id: "home-stage-zero".to_owned(),
                definition_id: "resource_logs".to_owned(),
                contribution_kind: ContributionKind::Material,
                purpose: BoundCargoPurpose::Construction {
                    project_id: "home-one".to_owned(),
                    stage_index: 0,
                },
                required_units: 1,
                unit_value_micros: 10,
                log_value_micros: 10,
                active_labor_remaining_seconds: 100,
            },
        },
    );
    let click =
        |state: &mut DivineHoleAuthority, id: &str, player_id: &str, clicks: u32, now: u64| {
            state
                .apply(
                    DivineHoleCommandEnvelope::new(
                        id,
                        state.version,
                        DivineHoleCommand::AcceptClickBatch {
                            batch: ClickBatchRequest {
                                target_id: "home-stage-zero".to_owned(),
                                player_id: player_id.to_owned(),
                                requested_clicks: clicks,
                                client_batch_window_ms: CLICK_BATCH_INTERVAL_MS,
                                now_real_ms: now,
                            },
                        },
                    )
                    .unwrap(),
                )
                .unwrap()
        };
    let first = click(&mut state, "click-one", "player-a", 40, 0);
    let cat_sim::divine_hole_authority::DivineHoleCommandOutcome::ClickBatch(first) = first else {
        panic!("click outcome")
    };
    assert_eq!(first.accepted_clicks, 40);
    let same_moment = click(&mut state, "click-two", "player-a", 1, 0);
    let cat_sim::divine_hole_authority::DivineHoleCommandOutcome::ClickBatch(same_moment) =
        same_moment
    else {
        panic!("click outcome")
    };
    assert_eq!(same_moment.accepted_clicks, 0);
    let second = click(&mut state, "click-three", "player-a", 40, 2_000);
    let cat_sim::divine_hole_authority::DivineHoleCommandOutcome::ClickBatch(second) = second
    else {
        panic!("click outcome")
    };
    assert_eq!(second.accepted_clicks, 40);
    let final_batch = click(&mut state, "click-four", "player-a", 20, 3_000);
    let cat_sim::divine_hole_authority::DivineHoleCommandOutcome::ClickBatch(final_batch) =
        final_batch
    else {
        panic!("click outcome")
    };
    assert_eq!(final_batch.accepted_clicks, 20);
    let [cargo] = final_batch.generated_cargo.as_slice() else {
        panic!("one physical grant")
    };
    assert_eq!(cargo.quantity, 1);
    assert!(!cargo.can_trade());
    assert!(!cargo.can_feed_hole());
    assert!(!cargo.can_return_to_general_stock());
    let overfill = click(&mut state, "click-five", "player-b", 40, 3_000);
    let cat_sim::divine_hole_authority::DivineHoleCommandOutcome::ClickBatch(overfill) = overfill
    else {
        panic!("click outcome")
    };
    assert_eq!(overfill.accepted_clicks, 0);
}

#[test]
fn lai61_inspiration_is_additive_per_player_and_cadence_bound() {
    let mut state = authority();
    apply(
        &mut state,
        "inspire-a",
        DivineHoleCommand::ActivateInspiration {
            player_id: "player-a".to_owned(),
            now_real_ms: 0,
        },
    );
    apply(
        &mut state,
        "inspire-b",
        DivineHoleCommand::ActivateInspiration {
            player_id: "player-b".to_owned(),
            now_real_ms: 0,
        },
    );
    assert_eq!(state.inspiration.additive_effect_basis_points(1), 2_000);
    assert!(
        state
            .apply(
                DivineHoleCommandEnvelope::new(
                    "inspire-a-again",
                    state.version,
                    DivineHoleCommand::ActivateInspiration {
                        player_id: "player-a".to_owned(),
                        now_real_ms: 1
                    }
                )
                .unwrap()
            )
            .is_err()
    );
    assert_eq!(
        state
            .inspiration
            .additive_effect_basis_points(15 * 60 * 1_000),
        0
    );
    assert!(
        state
            .apply(
                DivineHoleCommandEnvelope::new(
                    "inspire-a-too-early",
                    state.version,
                    DivineHoleCommand::ActivateInspiration {
                        player_id: "player-a".to_owned(),
                        now_real_ms: 15 * 60 * 1_000
                    }
                )
                .unwrap()
            )
            .is_err()
    );
    apply(
        &mut state,
        "inspire-a-later",
        DivineHoleCommand::ActivateInspiration {
            player_id: "player-a".to_owned(),
            now_real_ms: 60 * 60 * 1_000,
        },
    );
}

#[test]
fn lai61_ordinary_rescue_clicks_make_one_physical_apron_unit_while_void_rescue_is_population_sized()
{
    let mut state = authority();
    apply(
        &mut state,
        "ordinary-water-target",
        DivineHoleCommand::RegisterClickTarget {
            target: ClickTargetSpec {
                target_id: "ordinary-water".to_owned(),
                definition_id: "divine_water".to_owned(),
                contribution_kind: ContributionKind::TypedFood,
                purpose: BoundCargoPurpose::Emergency {
                    supply: EmergencySupplyKind::DivineWater,
                },
                required_units: 1,
                unit_value_micros: 10,
                log_value_micros: 10,
                active_labor_remaining_seconds: 100,
            },
        },
    );
    let mut last = None;
    for (id, clicks, now_real_ms) in [
        ("ordinary-one", 40, 0),
        ("ordinary-two", 40, 2_000),
        ("ordinary-three", 20, 3_000),
    ] {
        last = Some(
            state
                .apply(
                    DivineHoleCommandEnvelope::new(
                        id,
                        state.version,
                        DivineHoleCommand::AcceptClickBatch {
                            batch: ClickBatchRequest {
                                target_id: "ordinary-water".to_owned(),
                                player_id: "player-a".to_owned(),
                                requested_clicks: clicks,
                                client_batch_window_ms: CLICK_BATCH_INTERVAL_MS,
                                now_real_ms,
                            },
                        },
                    )
                    .unwrap(),
                )
                .unwrap(),
        );
    }
    let outcome = match last {
        Some(cat_sim::divine_hole_authority::DivineHoleCommandOutcome::ClickBatch(outcome)) => {
            outcome
        }
        _ => panic!("click outcome"),
    };
    assert_eq!(outcome.generated_cargo[0].quantity, 1);
    assert_eq!(
        outcome.generated_cargo[0].purpose,
        BoundCargoPurpose::Emergency {
            supply: EmergencySupplyKind::DivineWater
        }
    );
}

#[test]
fn lai61_miracle_rescue_boosts_replay_conflicts_restart_partition_and_redaction() {
    let mut state = authority();
    let mut ledger = funded_ledger(4);
    let miracle = VoidAction::ConstructionMiracle(ConstructionMiracleRequest {
        project_id: "workshop-one".to_owned(),
        player_id: "player-a".to_owned(),
        hole_feed_value_per_void_micros: 50,
        original_total_work_ms: 100,
        labor_stages: vec![
            MiracleLaborStage {
                stage_index: 0,
                remaining_work_ms: 20,
            },
            MiracleLaborStage {
                stage_index: 1,
                remaining_work_ms: 80,
            },
        ],
        inputs: vec![MiracleInput {
            stage_index: 0,
            definition_id: "resource_lumber".to_owned(),
            quantity: 2,
            unit_value_micros: 50,
            missing_quantity_before: 3,
        }],
        now_real_ms: 10,
    });
    let envelope = VoidActionEnvelope::new(
        "miracle-one",
        state.version,
        ledger.version,
        miracle.clone(),
    )
    .unwrap();
    let outcome = state
        .apply_void_action(&mut ledger, envelope.clone())
        .unwrap();
    assert_eq!(outcome.void_debit_micro, VoidInsight::ONE.micro());
    assert_eq!(outcome.labor_work_removed_ms, 10);
    assert_eq!(outcome.labor_stages_after[0].remaining_work_ms, 10);
    assert_eq!(outcome.labor_stages_after[1].remaining_work_ms, 80);
    assert_eq!(
        outcome.generated_cargo[0].purpose,
        BoundCargoPurpose::Construction {
            project_id: "workshop-one".to_owned(),
            stage_index: 0
        }
    );
    assert_eq!(ledger.balance, VoidInsight::from_whole(3).unwrap());
    let replay = state.apply_void_action(&mut ledger, envelope).unwrap();
    assert_eq!(replay, outcome);
    assert_eq!(ledger.balance, VoidInsight::from_whole(3).unwrap());
    let conflict = VoidActionEnvelope::new(
        "miracle-one",
        state.version,
        ledger.version,
        VoidAction::EmergencyRescue(EmergencyRescueRequest {
            player_id: "player-a".to_owned(),
            supply: EmergencySupplyKind::DivineRation,
            living_resident_count: 4,
            residents_dying_from_hunger: true,
            residents_dying_from_thirst: false,
            now_real_ms: 11,
        }),
    )
    .unwrap();
    assert!(state.apply_void_action(&mut ledger, conflict).is_err());

    let rescue_void_version = ledger.version;
    let rescue = state
        .apply_void_action(
            &mut ledger,
            VoidActionEnvelope::new(
                "rescue-one",
                state.version,
                rescue_void_version,
                VoidAction::EmergencyRescue(EmergencyRescueRequest {
                    player_id: "player-a".to_owned(),
                    supply: EmergencySupplyKind::DivineRation,
                    living_resident_count: 4,
                    residents_dying_from_hunger: true,
                    residents_dying_from_thirst: false,
                    now_real_ms: 12,
                }),
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(rescue.generated_cargo[0].quantity, 8);
    assert_eq!(rescue.generated_cargo[0].site_id, "hole_delivery_apron");
    assert_eq!(ledger.balance, VoidInsight::from_whole(2).unwrap());

    let mut boosts = DivineBoostState::new(colony());
    let mut progression = ProgressionAuthority::new(colony()).unwrap();
    progression
        .owned_studies
        .insert(StudyId::new("divine_boost_fleet_paws").unwrap());
    let boost = DivineBoostPurchaseRequest {
        id: DivineBoostPurchaseId::derive(&colony(), &player(), 1),
        partition: PlayerPartitionKey {
            colony_id: colony(),
            player_id: player(),
        },
        player_sequence: 1,
        authorization: DivineBoostAuthorization {
            actor: DivineBoostActor::Player {
                player_id: player(),
            },
            authenticated_player_id: Some(player()),
            owns_colony: true,
        },
        boost_type: DivineBoostType::FleetPaws,
        duration_hours: 1,
        expected_boost_version: boosts.version,
        expected_void_version: ledger.version,
        activated_tick: 0,
        ticks_per_game_hour: 60,
    };
    state
        .purchase_specialized_boost(&mut boosts, &mut ledger, &progression, boost)
        .unwrap();
    assert_eq!(ledger.balance, VoidInsight::from_whole(1).unwrap());
    assert_eq!(DivineHoleAuthority::specialized_boosts().len(), 4);

    let persisted = state.canonical_json().unwrap();
    let restored = DivineHoleAuthority::decode_strict(&persisted).unwrap();
    assert_eq!(restored, state);
    assert!(DivineHoleAuthority::decode_strict(&format!("{persisted}x")).is_err());
    let report = state.report_safe_summary(13);
    let report_json = serde_json::to_string(&report).unwrap();
    assert!(!report_json.contains("regeneration"));
    assert!(!report_json.contains("microVoid"));
    assert!(
        RUNTIME_CUTOVER_AUDIT
            .iter()
            .any(|entry| entry.contains("LAI.63"))
    );
    let mut foreign = VoidInsightLedger::new(PlannerId::derive("other", ["colony"]));
    let foreign_void_version = foreign.version;
    assert!(
        state
            .apply_void_action(
                &mut foreign,
                VoidActionEnvelope::new(
                    "foreign",
                    state.version,
                    foreign_void_version,
                    VoidAction::EmergencyRescue(EmergencyRescueRequest {
                        player_id: "player-a".to_owned(),
                        supply: EmergencySupplyKind::DivineWater,
                        living_resident_count: 1,
                        residents_dying_from_hunger: false,
                        residents_dying_from_thirst: true,
                        now_real_ms: 20,
                    })
                )
                .unwrap()
            )
            .is_err()
    );
}
