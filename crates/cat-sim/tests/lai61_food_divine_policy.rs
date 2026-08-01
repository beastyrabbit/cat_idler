//! Focused LAI.61 contracts. The coordinator runs this target only after the
//! bounded foundation wave is statically integrated.

use std::collections::BTreeSet;

use cat_sim::food_divine_policy::{
    BASIS_POINTS_SCALE, BoundCargoPurpose, CLICK_BATCH_INTERVAL_MS, ConservationNudge,
    ConstructionLaborStage, ConstructionMiracleInput, ConstructionMiracleRequest,
    ContributionBatch, ContributionClickState, ContributionKind, ContributionTarget,
    DivineActionState, EmergencyReportEvidence, EmergencySupplyKind, FOOD_DIVINE_SCHEMA_VERSION,
    FoodConsumptionDecision, FoodPermission, FoodPolicyActor, InspirationState, LeaderFoodPolicy,
    clicks_required_for_unit,
};

#[test]
fn lai61_food_permissions_are_leader_owned_and_starvation_never_dies_beside_food() {
    let mut policy = LeaderFoodPolicy::new();
    policy
        .register_edible("apple", false, 0)
        .expect("register apple");
    policy
        .register_edible("divine_ration", true, 0)
        .expect("register divine ration");
    assert_eq!(
        policy.entries["divine_ration"].permission,
        FoodPermission::Reserve
    );
    assert!(
        policy
            .set_permission(FoodPolicyActor::God, "apple", FoodPermission::Forbidden, 1,)
            .is_err()
    );
    policy
        .set_conservation_nudge(ConservationNudge::ProtectScarceFood)
        .expect("broad nudge");
    policy
        .set_permission(
            FoodPolicyActor::Leader,
            "apple",
            FoodPermission::Forbidden,
            2,
        )
        .expect("leader may make poor choice");
    let only_apple = BTreeSet::from(["apple".to_owned()]);
    assert_eq!(
        policy.consumption_decision("apple", &only_apple, true, false),
        FoodConsumptionDecision::Protected
    );
    assert_eq!(
        policy.consumption_decision("apple", &only_apple, true, true),
        FoodConsumptionDecision::LethalEmergencyOverride
    );
}

#[test]
fn lai61_clicks_use_value_formula_rate_limit_bound_cargo_and_no_overfill() {
    assert_eq!(clicks_required_for_unit(10, 10).unwrap(), 100);
    assert_eq!(clicks_required_for_unit(11, 10).unwrap(), 110);
    let mut state = ContributionClickState {
        schema_version: FOOD_DIVINE_SCHEMA_VERSION,
        version: 0,
        target: ContributionTarget {
            target_id: "home-1-stage-0".to_owned(),
            definition_id: "wood".to_owned(),
            contribution_kind: ContributionKind::Material,
            purpose: BoundCargoPurpose::Construction {
                project_id: "home-1".to_owned(),
                stage_index: 0,
            },
            required_units: 1,
            created_units: 0,
            clicks_toward_next_unit: 0,
            clicks_per_unit: 100,
            unit_value_micros: 10,
            active_labor_remaining_seconds: 200,
        },
        player_limiters: Default::default(),
        cargo: Default::default(),
        next_cargo_serial: 0,
    };
    let first = state
        .accept_batch(ContributionBatch {
            player_id: "god-a".to_owned(),
            requested_clicks: 40,
            client_batch_window_ms: CLICK_BATCH_INTERVAL_MS,
            now_real_ms: 0,
        })
        .expect("two-second burst");
    assert_eq!(first.accepted_clicks, 40);
    let throttled = state
        .accept_batch(ContributionBatch {
            player_id: "god-a".to_owned(),
            requested_clicks: 1,
            client_batch_window_ms: CLICK_BATCH_INTERVAL_MS,
            now_real_ms: 0,
        })
        .expect("same instant is throttled");
    assert_eq!(throttled.accepted_clicks, 0);
    let complete = state
        .accept_batch(ContributionBatch {
            player_id: "god-a".to_owned(),
            requested_clicks: 100,
            client_batch_window_ms: CLICK_BATCH_INTERVAL_MS,
            now_real_ms: 3_000,
        })
        .expect("refilled tokens complete target");
    assert_eq!(complete.accepted_clicks, 40);
    let finish = state
        .accept_batch(ContributionBatch {
            player_id: "god-a".to_owned(),
            requested_clicks: 20,
            client_batch_window_ms: CLICK_BATCH_INTERVAL_MS,
            now_real_ms: 4_000,
        })
        .expect("final clicks");
    assert_eq!(finish.accepted_clicks, 20);
    assert_eq!(state.target.created_units, 1);
    assert_eq!(state.target.active_labor_remaining_seconds, 100);
    let cargo = state.cargo.values().next().expect("physical cargo");
    assert!(!cargo.can_trade());
    assert!(!cargo.can_feed_hole());
    assert!(!cargo.can_return_to_general_stock());
    let full = state
        .accept_batch(ContributionBatch {
            player_id: "god-b".to_owned(),
            requested_clicks: 40,
            client_batch_window_ms: CLICK_BATCH_INTERVAL_MS,
            now_real_ms: 4_000,
        })
        .expect("complete target rejects overfill without error");
    assert_eq!(full.accepted_clicks, 0);
}

#[test]
fn lai61_inspiration_is_per_player_additive_timed_and_cooldown_bound() {
    let mut state = InspirationState::new();
    state.activate("god-a", 0).expect("first player");
    state.activate("god-b", 0).expect("second player");
    assert_eq!(
        state.effective_stat_basis_points(1),
        BASIS_POINTS_SCALE + 2_000
    );
    assert!(state.activate("god-a", 1).is_err());
    assert_eq!(
        state.effective_stat_basis_points(15 * 60 * 1_000),
        BASIS_POINTS_SCALE
    );
    assert!(state.activate("god-a", 15 * 60 * 1_000).is_err());
    state
        .activate("god-a", 60 * 60 * 1_000)
        .expect("cooldown elapsed");
}

#[test]
fn lai61_void_construction_miracle_is_exact_bound_and_earliest_stage_first() {
    let mut state = DivineActionState::new();
    let mut void_insight = 2;
    let mut stages = vec![
        ConstructionLaborStage {
            stage_index: 0,
            original_labor_seconds: 20,
            completed_labor_seconds: 15,
        },
        ConstructionLaborStage {
            stage_index: 1,
            original_labor_seconds: 60,
            completed_labor_seconds: 0,
        },
        ConstructionLaborStage {
            stage_index: 2,
            original_labor_seconds: 20,
            completed_labor_seconds: 0,
        },
    ];
    let event = state
        .apply_construction_miracle(
            &mut void_insight,
            &mut stages,
            ConstructionMiracleRequest {
                action_id: "miracle-1".to_owned(),
                player_id: "god-a".to_owned(),
                project_id: "workshop-1".to_owned(),
                hole_feed_value_per_void_micros: 50,
                inputs: vec![ConstructionMiracleInput {
                    stage_index: 1,
                    definition_id: "lumber".to_owned(),
                    quantity: 2,
                    unit_value_micros: 50,
                    missing_quantity_before: 4,
                }],
                now_real_ms: 5,
            },
        )
        .expect("exact one-void miracle");
    assert_eq!(void_insight, 1);
    assert_eq!(event.input_value_micros, 100);
    assert_eq!(event.labor_seconds_removed, 10);
    assert_eq!(stages[0].completed_labor_seconds, 20);
    assert_eq!(stages[1].completed_labor_seconds, 5);
    let cargo = state.cargo.values().next().expect("bound miracle cargo");
    assert_eq!(
        cargo.purpose,
        BoundCargoPurpose::Construction {
            project_id: "workshop-1".to_owned(),
            stage_index: 1
        }
    );
}

#[test]
fn lai61_population_rescue_requires_report_evidence_and_is_physical_uncapped_stock() {
    let mut state = DivineActionState::new();
    let mut void_insight = 2;
    let hidden = EmergencyReportEvidence {
        residents_dying_from_hunger: false,
        residents_dying_from_thirst: false,
    };
    assert!(
        state
            .create_void_rescue(
                &mut void_insight,
                "rescue-hidden",
                "god-a",
                EmergencySupplyKind::DivineRation,
                12,
                hidden,
                0,
            )
            .is_err()
    );
    let event = state
        .create_void_rescue(
            &mut void_insight,
            "rescue-visible",
            "god-a",
            EmergencySupplyKind::DivineRation,
            12,
            EmergencyReportEvidence {
                residents_dying_from_hunger: true,
                residents_dying_from_thirst: false,
            },
            1,
        )
        .expect("report-safe rescue");
    assert_eq!(event.quantity, 24);
    assert_eq!(void_insight, 1);
    let cargo = state.cargo.get(&event.cargo_id).expect("apron cargo");
    assert_eq!(cargo.quantity, 24);
    assert_eq!(cargo.site_id, "hole_delivery_apron");
    assert_eq!(
        EmergencySupplyKind::DivineRation.default_food_permission(),
        Some(FoodPermission::Reserve)
    );
    assert_eq!(
        EmergencySupplyKind::DivineWater.need_restored_basis_points(),
        BASIS_POINTS_SCALE as u16
    );
    assert!(!EmergencySupplyKind::DivineWater.expires());
}
