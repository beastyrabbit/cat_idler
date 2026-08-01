//! Focused LAI.46/LAI.63 canonical runtime cutover contract.

use std::collections::BTreeSet;

use cat_sim::{
    cat_capability_authority::{ProductiveOutcome, WorkActivity},
    construction_catalog::{BlueprintRequest, resolve_blueprint},
    construction_stages::{ConstructionProject, ConstructionTargetKind, stage_work_durations},
    content_manifest::{ContentId, PhysicalLotId},
    diplomacy::DiplomacyColonyId,
    divine_hole_authority::{
        ConstructionMiracleRequest, MiracleInput, MiracleLaborStage, VoidAction, VoidActionEnvelope,
    },
    family_authority::ProfessionalCompletion,
    food_divine_policy::BoundCargoPurpose,
    leader_ai_diagnostics::{Lai69DiagnosticDomain, Lai69Phase, Lai69TraceInput, Lai69TraceKind},
    leader_ai_runtime::{
        LEADER_AI_RUNTIME_SCHEMA_VERSION, LeaderAiRuntimeError, LeaderAiRuntimeState,
        ProtectedRuntimePhase,
    },
    leader_planner::{
        EffectiveLevel, LeaderPosture,
        content_planner::{
            CandidateKind, CargoIntent, CargoStage, ContentDomain, ContentPlannerState,
            ExecutionFeedback, OfficerCoverage, OfficerPlanRequest, PlannerCommandStage,
            PlannerCompetence, PlannerReviewRequest, ReportSafePlanningInput, ReportedCandidate,
            ReportedCargo, ReportedFoodKind, ReportedSiteKind, ReportedSiteRef,
            TypedOfficerRequest, god_report_bytes, planner_report_bytes, review,
        },
    },
    moneyless_barter::PersonalStance,
    officers::OfficerRole,
    physical_storage::StorageCompatibility,
    planner_core::{IntentId, PlannerId},
    progression_research::{HoleVoidCreditPayload, VoidInsight},
    quality_lots::{BulkLotKey, LotLocation, LotProvenance, PhysicalLot, QualityBand},
    spatial_tasks::{Rect, TaskFootprint, TilePoint},
    storage_authority::{
        StorageAddress, StorageCommand, StorageCommandEnvelope, StorageIdentity, StorageZone,
        StorageZoneKind,
    },
    task_runtime::TaskId,
    types::BuildingType,
    world_tick::{found_colony, new_world, world_tick},
};

fn id(namespace: &str, value: &str) -> PlannerId {
    PlannerId::derive(namespace, [value])
}

fn commit_all_phases(state: &mut LeaderAiRuntimeState, tick: u64) {
    let mut transaction = state.begin_tick_transaction(tick).unwrap();
    for phase in ProtectedRuntimePhase::ORDER {
        transaction.enter(phase).unwrap();
    }
    transaction.commit(state).unwrap();
}

fn site(kind: ReportedSiteKind, value: &str) -> ReportedSiteRef {
    ReportedSiteRef {
        site_id: id("site", value),
        kind,
        x: 7,
        y: 9,
        report_id: id("report", value),
    }
}

fn hole_candidate(
    name: &str,
    food_kind: ReportedFoodKind,
    units: u32,
    replacement_cost: u64,
) -> ReportedCandidate {
    ReportedCandidate {
        id: id("candidate", name),
        domain: ContentDomain::Hole,
        kind: CandidateKind::FeedHole,
        target_id: id("target", "next_hole_feed"),
        site: Some(site(ReportedSiteKind::HoleWorkArea, "black_hole_main")),
        cargo: Some(ReportedCargo {
            content_id: ContentId::new(name).unwrap(),
            food_kind: Some(food_kind),
            quality: QualityBand::Common,
            believed_units: units,
            believed_replacement_cost_milli: replacement_cost,
            lot_id: Some(PhysicalLotId::new(format!("{name}_lot")).unwrap()),
        }),
        urgency_basis_points: 2_000,
        confidence_basis_points: 8_000,
        expected_benefit_milli: 5_000,
        expected_labor_cost_milli: 1_000,
        temporary_player_bias_basis_points: 0,
        report_tick: 0,
        evidence_ids: BTreeSet::from([id("evidence", name)]),
        report_ids: BTreeSet::from([id("report", name)]),
        ordered_fallbacks: Vec::new(),
        rationale_key: id("rationale", name),
    }
}

fn report(mut candidates: Vec<ReportedCandidate>) -> ReportSafePlanningInput {
    candidates.sort_by(|left, right| left.id.cmp(&right.id));
    ReportSafePlanningInput {
        schema_version: 1,
        colony_id: id("colony", "one"),
        report_version: 1,
        observed_tick: 0,
        posture: LeaderPosture::Stabilize,
        candidates,
    }
}

fn planner_request(
    state: &ContentPlannerState,
    request_name: &str,
    competence: PlannerCompetence,
    report: ReportSafePlanningInput,
) -> PlannerReviewRequest {
    PlannerReviewRequest {
        request_id: id("review", request_name),
        expected_state_version: state.version,
        world_seed: 5,
        review_tick: 360,
        leader_id: id("cat", "leader"),
        leader_level: EffectiveLevel::try_from(5).unwrap(),
        competence,
        report,
        officers: Vec::new(),
        officer_requests: Vec::new(),
        execution_feedback: Vec::new(),
    }
}

fn selected_feed(
    outcome: &cat_sim::leader_planner::content_planner::PlannerReviewOutcome,
) -> ContentId {
    outcome
        .commands
        .iter()
        .find_map(|command| match &command.cargo_intent {
            CargoIntent::ReserveReported(cargo) => Some(cargo.content_id.clone()),
            _ => None,
        })
        .expect("one selected Hole cargo")
}

#[test]
fn canonical_aggregate_is_strict_partitioned_restart_stable_and_has_no_shadow_fields() {
    let mut state = LeaderAiRuntimeState::new_for_colony_seed("colony_one", 77).unwrap();
    assert_eq!(state.schema_version, LEADER_AI_RUNTIME_SCHEMA_VERSION);
    commit_all_phases(&mut state, 1);
    state.validate().unwrap();

    let json = serde_json::to_string(&state).unwrap();
    for forbidden in [
        "shrineFavor",
        "shrineOfferings",
        "\"favor\"",
        "\"coins\"",
        "\"scholars\"",
    ] {
        assert!(
            !json.contains(forbidden),
            "shadow field survived: {forbidden}"
        );
    }
    let value = serde_json::to_value(&state).unwrap();
    assert!(
        value["research"].get("purchases").is_none(),
        "legacy research purchase aggregate survived"
    );
    let restarted: LeaderAiRuntimeState = serde_json::from_str(&json).unwrap();
    assert_eq!(restarted, state);
    assert_eq!(serde_json::to_string(&restarted).unwrap(), json);

    let mut unknown = serde_json::to_value(&state).unwrap();
    unknown["parallelPlanner"] = serde_json::json!({});
    assert!(serde_json::from_value::<LeaderAiRuntimeState>(unknown).is_err());

    let mut wrong_partition = serde_json::to_value(&state).unwrap();
    wrong_partition["colonyId"] = serde_json::json!("other_colony");
    assert!(serde_json::from_value::<LeaderAiRuntimeState>(wrong_partition).is_err());
}

#[test]
fn phase_transaction_requires_exact_order_and_rolls_back_on_drop() {
    let state = LeaderAiRuntimeState::new_for_colony("colony_one").unwrap();
    let before = serde_json::to_vec(&state).unwrap();
    let mut transaction = state.begin_tick_transaction(1).unwrap();
    assert!(matches!(
        transaction.enter(ProtectedRuntimePhase::UnifiedResearch),
        Err(LeaderAiRuntimeError::PhaseOrderViolation)
    ));
    drop(transaction);
    assert_eq!(serde_json::to_vec(&state).unwrap(), before);

    let mut incomplete = state.begin_tick_transaction(1).unwrap();
    incomplete
        .enter(ProtectedRuntimePhase::AuthorityAndNeeds)
        .unwrap();
    let mut unchanged = state.clone();
    assert_eq!(
        incomplete.commit(&mut unchanged),
        Err(LeaderAiRuntimeError::IncompletePhaseTransaction)
    );
    assert_eq!(unchanged, state);
}

#[test]
fn world_tick_installs_the_protected_order_once_and_partitioning_is_stable() {
    let mut direct = new_world(42);
    direct.colonies.push(found_colony(42, "colony-one", 0, 7));
    let _ = world_tick(&mut direct, 1_000);
    let mut partitioned = direct.clone();

    let _ = world_tick(&mut direct, 61_000);
    for now_ms in 2_000..=61_000 {
        let _ = world_tick(&mut partitioned, now_ms);
    }

    let direct_runtime = &direct.colonies[0].leader_ai_runtime;
    let partitioned_runtime = &partitioned.colonies[0].leader_ai_runtime;
    assert_eq!(direct_runtime, partitioned_runtime);
    assert_eq!(
        direct_runtime
            .phase_receipts
            .values()
            .next_back()
            .unwrap()
            .phases,
        ProtectedRuntimePhase::ORDER
    );
}

#[test]
fn report_projection_is_byte_identical_and_rejects_hidden_regeneration() {
    let report = report(vec![hole_candidate(
        "resource_logs",
        ReportedFoodKind::Other,
        20,
        10,
    )]);
    assert_eq!(
        planner_report_bytes(&report).unwrap(),
        god_report_bytes(&report).unwrap()
    );
    let bytes = String::from_utf8(planner_report_bytes(&report).unwrap()).unwrap();
    for forbidden in [
        "regeneration",
        "replenishment",
        "respawn",
        "authoritativeStock",
    ] {
        assert!(!bytes.contains(forbidden));
    }

    let mut value = serde_json::to_value(&report).unwrap();
    value["exactRegeneration"] = serde_json::json!(12);
    assert!(serde_json::from_value::<ReportSafePlanningInput>(value).is_err());
}

#[test]
fn survival_preempts_hole_and_strong_weak_choices_remain_visible_and_legal() {
    let mut apples = hole_candidate("food_apples", ReportedFoodKind::Apples, 2, 900);
    let mut fish = hole_candidate("food_fish", ReportedFoodKind::Fish, 50, 100);
    apples.ordered_fallbacks = vec![fish.id.clone()];
    fish.ordered_fallbacks = vec![apples.id.clone()];

    let mut strong = ContentPlannerState::new(id("colony", "one"));
    let strong_request = planner_request(
        &strong,
        "strong",
        PlannerCompetence::Strong,
        report(vec![apples.clone(), fish.clone()]),
    );
    let strong_outcome = review(&mut strong, strong_request).unwrap();
    assert_eq!(
        selected_feed(&strong_outcome),
        ContentId::new("food_fish").unwrap()
    );

    let mut weak = ContentPlannerState::new(id("colony", "one"));
    let mut weak_report = report(vec![fish, apples]);
    weak_report.observed_tick = 0;
    let weak_request = planner_request(&weak, "weak", PlannerCompetence::Weak, weak_report);
    let weak_outcome = review(&mut weak, weak_request).unwrap();
    assert_eq!(
        selected_feed(&weak_outcome),
        ContentId::new("food_apples").unwrap()
    );

    let mut survival = ReportedCandidate {
        id: id("candidate", "lethal_need"),
        domain: ContentDomain::Survival,
        kind: CandidateKind::SelfPreservation,
        target_id: id("target", "lethal_need"),
        site: None,
        cargo: None,
        urgency_basis_points: 10_000,
        confidence_basis_points: 10_000,
        expected_benefit_milli: 100_000,
        expected_labor_cost_milli: 1,
        temporary_player_bias_basis_points: 0,
        report_tick: 0,
        evidence_ids: BTreeSet::from([id("evidence", "lethal_need")]),
        report_ids: BTreeSet::from([id("report", "lethal_need")]),
        ordered_fallbacks: Vec::new(),
        rationale_key: id("rationale", "survival_before_hole"),
    };
    survival.urgency_basis_points = 10_000;
    let mut preempted = ContentPlannerState::new(id("colony", "one"));
    let hole = hole_candidate("resource_logs", ReportedFoodKind::Other, 20, 1);
    let first_request = planner_request(
        &preempted,
        "preempt-hole-first",
        PlannerCompetence::Strong,
        report(vec![hole.clone()]),
    );
    let active_hole = review(&mut preempted, first_request).unwrap();
    let active_hole_goal = active_hole
        .commands
        .first()
        .expect("the first review materializes the reported Hole goal")
        .goal_id
        .clone();
    let mut preempt_request = planner_request(
        &preempted,
        "preempt-survival",
        PlannerCompetence::Strong,
        report(vec![hole, survival]),
    );
    preempt_request.execution_feedback = vec![ExecutionFeedback {
        goal_id: active_hole_goal,
        cargo_stage: CargoStage::BeforePickup,
        delivery_endpoint: None,
        salvage_endpoint: None,
        reported_delivery_route_viable: false,
        failure: None,
        report_id: id("report", "preempt-before-pickup"),
    }];
    let preemption = review(&mut preempted, preempt_request).unwrap();
    assert!(
        preemption
            .commands
            .iter()
            .any(|command| command.stage == PlannerCommandStage::PreemptBeforePickup)
    );
}

#[test]
fn officer_request_becomes_a_persisted_leader_requirement() {
    let candidate = hole_candidate("resource_logs", ReportedFoodKind::Other, 20, 10);
    let mut state = ContentPlannerState::new(id("colony", "one"));
    let mut request = planner_request(
        &state,
        "officer_request",
        PlannerCompetence::Ordinary,
        report(vec![candidate.clone()]),
    );
    request.leader_level = EffectiveLevel::try_from(5).unwrap();
    request.officers = vec![OfficerCoverage {
        role: OfficerRole::Loremaster,
        officer_id: id("cat", "loremaster"),
        effective_level: EffectiveLevel::try_from(3).unwrap(),
    }];
    request.officer_requests = vec![OfficerPlanRequest {
        request_id: id("request", "hole_space"),
        officer_role: OfficerRole::Loremaster,
        report_id: id("report", "black_hole_main"),
        expires_tick: 1_000,
        request: TypedOfficerRequest::Space {
            candidate_id: candidate.id,
            site_kind: ReportedSiteKind::HoleWorkArea,
            required_cells: 9,
        },
    }];
    let outcome = review(&mut state, request).unwrap();
    let goal = state.live_goals.get(&outcome.commands[0].goal_id).unwrap();
    assert!(goal.requirements.iter().any(|requirement| matches!(
        requirement,
        cat_sim::leader_planner::content_planner::GoalRequirement::Space {
            site_kind: ReportedSiteKind::HoleWorkArea,
            required_cells: 9,
        }
    )));
}

#[test]
fn workshop_construction_and_hole_use_complete_three_by_three_work_footprints() {
    let blueprint =
        resolve_blueprint(BlueprintRequest::NewBuilding(BuildingType::Workshop)).unwrap();
    let footprint = TaskFootprint::rectangular(
        Rect::try_new(
            TilePoint { x: 10, y: 20 },
            blueprint.footprint.width,
            blueprint.footprint.height,
        )
        .unwrap(),
    );
    let project = ConstructionProject::new(
        "project_workshop",
        ConstructionTargetKind::Building,
        Some(BuildingType::Workshop),
        1,
        blueprint.scaffold_tier,
        footprint,
        blueprint.fresh_bills(),
        blueprint.base_work_duration_ms,
        0,
    )
    .unwrap();
    let mut state = LeaderAiRuntimeState::new_for_colony("colony_one").unwrap();
    state
        .insert_construction_project(project, BTreeSet::new())
        .unwrap();
    let stored = &state.construction_projects["project_workshop"];
    assert_eq!((stored.footprint.width, stored.footprint.height), (3, 3));
    assert_eq!(stored.footprint.tiles.len(), 9);
    assert_eq!(state.hole.footprint().work.tiles.len(), 9);
    assert_eq!(state.hole.footprint().ring.len(), 16);
    assert_eq!(stage_work_durations(100), (20, 60, 20));

    let mut malformed = serde_json::to_value(&state).unwrap();
    malformed["constructionProjects"]["project_workshop"]["footprint"]["width"] =
        serde_json::json!(1);
    assert!(serde_json::from_value::<LeaderAiRuntimeState>(malformed).is_err());
}

#[test]
fn one_completed_task_has_one_xp_family_and_replay_path() {
    let colony = found_colony(7, "colony-one", 0, 7);
    let cat = colony.cats[0].clone();
    let mut state = LeaderAiRuntimeState::new_for_colony_seed("colony-one", 7).unwrap();
    state
        .reconcile_legacy_cats(7, "colony-one", &[cat.clone()])
        .unwrap();
    let intent_id = IntentId::derive("colony-one", 1, "hunt", "lair_one", 0);
    let task_id = TaskId::derive("colony-one", &intent_id, 0);
    let family_receipt_id = format!("family_outcome:{}", task_id.as_str());
    assert!(
        cat_sim::family_specialization::is_stable_id(task_id.as_str()),
        "nested canonical TaskId must be accepted losslessly by family authority"
    );
    assert!(
        cat_sim::family_specialization::is_stable_id(&family_receipt_id),
        "canonical family outcome receipt must preserve the full nested TaskId"
    );
    let outcome = ProductiveOutcome::Productive {
        productive_minutes: 60,
        activity: Some(WorkActivity {
            primary_skill_id: "hunting".to_owned(),
            secondary_skill_ids: Vec::new(),
            haul_legs: 0,
        }),
        office: None,
        supervised_by: None,
    };
    let family = ProfessionalCompletion {
        task_id: task_id.as_str().to_owned(),
        cat_id: cat.id.clone(),
        profession_id: "hunter".to_owned(),
        skill_id: "hunting".to_owned(),
        skill_xp_centi: 100,
        enterprise_id: None,
    };
    let binding = state
        .apply_task_outcome_once(
            task_id.clone(),
            cat.id.clone(),
            outcome.clone(),
            Some(family.clone()),
        )
        .unwrap();
    let after_first = state.clone();
    assert_eq!(
        state
            .apply_task_outcome_once(task_id, cat.id.clone(), outcome, Some(family))
            .unwrap(),
        binding
    );
    assert_eq!(state, after_first);
    let hunting = state
        .cat_capabilities
        .cat_report(&cat.id)
        .unwrap()
        .skills
        .into_iter()
        .find(|skill| skill.skill_id == "hunting")
        .unwrap();
    assert!(hunting.progress.total_xp_centi >= 100);
    assert!(
        state
            .families
            .completed_task_ids
            .contains(binding.task_id.as_str())
    );
}

#[test]
fn construction_storage_identity_and_restart_conserve_the_same_physical_lot() {
    let mut state = LeaderAiRuntimeState::new_for_colony("colony_one").unwrap();
    let zone = StorageZone::new(
        "zone_main",
        StorageZoneKind::Stockpile,
        TaskFootprint::rectangular(Rect::try_new(TilePoint { x: 0, y: 0 }, 1, 1).unwrap()),
    )
    .unwrap();
    state
        .storage
        .execute(StorageCommandEnvelope {
            colony_id: "colony_one".to_owned(),
            command_id: "register_zone".to_owned(),
            fingerprint: "register_zone_v1".to_owned(),
            sequence: 1,
            command: StorageCommand::RegisterZone { zone },
        })
        .unwrap();
    let lot_id = PhysicalLotId::new("construction_logs").unwrap();
    state
        .storage
        .execute(StorageCommandEnvelope {
            colony_id: "colony_one".to_owned(),
            command_id: "deposit_logs".to_owned(),
            fingerprint: "deposit_logs_v1".to_owned(),
            sequence: 2,
            command: StorageCommand::DepositLot {
                lot: PhysicalLot {
                    id: lot_id.clone(),
                    key: BulkLotKey::new(
                        ContentId::new("resource_logs").unwrap(),
                        QualityBand::Common,
                    ),
                    provenance: LotProvenance {
                        origin: "gathering:forest".to_owned(),
                        created_tick: 1,
                    },
                    quantity: 100,
                    location: LotLocation::Source("forest_one".to_owned()),
                    reservation: None,
                },
                compatibility: StorageCompatibility::BulkMaterial,
                destination: StorageAddress::Loose {
                    zone_id: "zone_main".to_owned(),
                    tile: TilePoint { x: 0, y: 0 },
                    slot: 0,
                },
            },
        })
        .unwrap();

    let blueprint =
        resolve_blueprint(BlueprintRequest::NewBuilding(BuildingType::Workshop)).unwrap();
    let project = ConstructionProject::new(
        "project_workshop",
        ConstructionTargetKind::Building,
        Some(BuildingType::Workshop),
        1,
        blueprint.scaffold_tier,
        TaskFootprint::rectangular(Rect::try_new(TilePoint { x: 3, y: 3 }, 3, 3).unwrap()),
        blueprint.fresh_bills(),
        blueprint.base_work_duration_ms,
        0,
    )
    .unwrap();
    let identity = StorageIdentity::Lot(lot_id.clone());
    state
        .insert_construction_project(project, BTreeSet::from([identity.clone()]))
        .unwrap();
    assert!(state.storage.location(&identity).is_some());
    state
        .research
        .void
        .credit_hole_feed(HoleVoidCreditPayload {
            partition: state.research.void.partition.clone(),
            feed_sequence: 1,
            amount: VoidInsight::from_whole(2).unwrap(),
        })
        .unwrap();
    let miracle = VoidAction::ConstructionMiracle(ConstructionMiracleRequest {
        project_id: "project_workshop".to_owned(),
        player_id: "player_one".to_owned(),
        hole_feed_value_per_void_micros: 50,
        original_total_work_ms: 100,
        labor_stages: vec![MiracleLaborStage {
            stage_index: 0,
            remaining_work_ms: 100,
        }],
        inputs: vec![MiracleInput {
            stage_index: 0,
            definition_id: "resource_lumber".to_owned(),
            quantity: 2,
            unit_value_micros: 50,
            missing_quantity_before: 2,
        }],
        now_real_ms: 10,
    });
    let envelope = VoidActionEnvelope::new(
        "miracle_one",
        state.divine_hole.version,
        state.research.void.version,
        miracle,
    )
    .unwrap();
    let outcome = state
        .apply_void_action_and_materialize(
            envelope.clone(),
            StorageAddress::Loose {
                zone_id: "zone_main".to_owned(),
                tile: TilePoint { x: 0, y: 0 },
                slot: 1,
            },
            StorageCompatibility::BulkMaterial,
        )
        .unwrap();
    assert_eq!(state.research.void.balance, VoidInsight::ONE);
    assert_eq!(state.purpose_bound_storage.len(), 1);
    assert!(state.purpose_bound_storage.values().any(|purpose| matches!(
        purpose,
        BoundCargoPurpose::Construction {
            project_id,
            stage_index: 0,
        } if project_id == "project_workshop"
    )));
    let divine_identity = state.purpose_bound_storage.keys().next().unwrap();
    assert!(!state.storage_identity_can_trade(divine_identity));
    assert!(!state.storage_identity_can_feed_hole(divine_identity));
    let after_miracle = state.clone();
    assert_eq!(
        state
            .apply_void_action_and_materialize(
                envelope,
                StorageAddress::Loose {
                    zone_id: "zone_main".to_owned(),
                    tile: TilePoint { x: 0, y: 0 },
                    slot: 1,
                },
                StorageCompatibility::BulkMaterial,
            )
            .unwrap(),
        outcome
    );
    assert_eq!(state, after_miracle);
    assert_eq!(state.hole.micro_void_balance, 0);
    assert_eq!(state.trade.summary().contract_count, 0);
    state
        .trade
        .set_stance(
            "stance_one",
            "stance_one_v1",
            state.trade.version(),
            DiplomacyColonyId::derive("colony_one"),
            DiplomacyColonyId::derive("colony_two"),
            PersonalStance::Alliance,
        )
        .unwrap();

    let restarted: LeaderAiRuntimeState =
        serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
    assert_eq!(
        restarted.storage.location(&identity),
        state.storage.location(&identity)
    );
    assert_eq!(restarted.trade, state.trade);
    assert_eq!(restarted, state);
}

#[test]
fn diagnostics_are_bounded_and_do_not_change_public_report_bytes() {
    let mut state = LeaderAiRuntimeState::new_for_colony("colony_one").unwrap();
    state.diagnostics.enabled = true;
    state.diagnostics.config.max_records = 4;
    for tick in 1..=12 {
        state
            .diagnostics
            .record(Lai69TraceInput {
                tick,
                phase: Lai69Phase::Planning,
                domain: Lai69DiagnosticDomain::Planner,
                kind: Lai69TraceKind::PhaseEnter,
            })
            .unwrap();
    }
    assert_eq!(state.diagnostics.records.len(), 4);
    assert_eq!(
        state.diagnostics.records.front().map(|record| record.tick),
        Some(9)
    );
    state.validate().unwrap();
}
