//! Focused LAI.45 pure planner/runtime contracts.

use std::collections::BTreeSet;

use cat_sim::{
    authority::{AuthorityActor, AuthorityContext, AuthorityDomain},
    beliefs::{BeliefKey, BeliefKind, Confidence, EvidenceId, ReportId},
    content_manifest::{ContentId, PhysicalLotId},
    leader_planner::{
        EffectiveLevel, LeaderPosture,
        content_planner::{
            CandidateKind, CargoIntent, CargoStage, ContentDomain, ContentPlannerError,
            ContentPlannerState, ExecutionFeedback, GoalRequirement, KeepStockOrder,
            LocatedFoodRecoveryKind, OfficerCoverage, OfficerPlanRequest, PlannerCommandStage,
            PlannerCompetence, PlannerPhase, PlannerReviewRequest, RecoveryReason,
            ReportSafePlanningInput, ReportedCandidate, ReportedCargo, ReportedFoodKind,
            ReportedSiteKind, ReportedSiteRef, TypedOfficerRequest, god_report_bytes,
            keep_stock_orders, officer_plan_requests, planner_report_bytes, review, specialist_for,
        },
    },
    officer_expertise::{
        ExpertiseLevel, OfficeExpertiseSupport, effective_level as officer_effective_level,
    },
    officer_requests::{
        OfficerRequestBook, OfficerRequestDraft, OfficerRequestPayload, RequestKind,
        RequestedSpaceKind, TypedOfficerRequestDraft, structured_request_budget,
    },
    officers::OfficerRole,
    planner_core::{BasisPoints, PlannerId},
    quality_lots::QualityBand,
};

const TICKS_PER_GAME_HOUR: u64 = 60;

fn id(namespace: &str, value: &str) -> PlannerId {
    PlannerId::derive(namespace, [value])
}

fn site(kind: ReportedSiteKind, value: &str) -> ReportedSiteRef {
    ReportedSiteRef {
        site_id: id("site", value),
        kind,
        x: value.len() as i32,
        y: -(value.len() as i32),
        report_id: id("report", value),
    }
}

fn cargo(
    content: &str,
    food_kind: ReportedFoodKind,
    units: u32,
    replacement_cost: u64,
) -> ReportedCargo {
    ReportedCargo {
        content_id: ContentId::new(content).unwrap(),
        food_kind: Some(food_kind),
        quality: QualityBand::Common,
        believed_units: units,
        believed_replacement_cost_milli: replacement_cost,
        lot_id: Some(PhysicalLotId::new(format!("{content}_lot")).unwrap()),
    }
}

fn candidate(
    name: &str,
    domain: ContentDomain,
    kind: CandidateKind,
    reported_site: Option<ReportedSiteRef>,
) -> ReportedCandidate {
    ReportedCandidate {
        id: id("candidate", name),
        domain,
        kind,
        target_id: id("target", name),
        site: reported_site,
        cargo: None,
        urgency_basis_points: 7_500,
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

fn hole_candidate(
    name: &str,
    food_kind: ReportedFoodKind,
    units: u32,
    replacement_cost: u64,
) -> ReportedCandidate {
    let mut value = candidate(
        name,
        ContentDomain::Hole,
        CandidateKind::FeedHole,
        Some(site(ReportedSiteKind::HoleWorkArea, "hole")),
    );
    value.target_id = id("target", "next_hole_feed");
    value.cargo = Some(cargo(name, food_kind, units, replacement_cost));
    value
}

fn input(mut candidates: Vec<ReportedCandidate>) -> ReportSafePlanningInput {
    candidates.sort_by(|left, right| left.id.cmp(&right.id));
    ReportSafePlanningInput {
        schema_version: 1,
        colony_id: id("colony", "one"),
        report_version: 7,
        observed_tick: 0,
        posture: LeaderPosture::Stabilize,
        candidates,
    }
}

fn request(
    state: &ContentPlannerState,
    request_name: &str,
    world_seed: u32,
    review_tick: u64,
    competence: PlannerCompetence,
    report: ReportSafePlanningInput,
) -> PlannerReviewRequest {
    PlannerReviewRequest {
        request_id: id("review", request_name),
        expected_state_version: state.version,
        world_seed,
        review_tick,
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
        .expect("feed reservation command")
}

#[test]
fn exact_phase_machine_emits_commands_without_world_mutation() {
    let mut state = ContentPlannerState::new(id("colony", "one"));
    let recovery = candidate(
        "apple_recovery",
        ContentDomain::Food,
        CandidateKind::RecoverFood(LocatedFoodRecoveryKind::AppleTree),
        Some(site(ReportedSiteKind::AppleTree, "apple_tree_1")),
    );
    let review_request = request(
        &state,
        "phase-machine",
        1,
        10,
        PlannerCompetence::Strong,
        input(vec![recovery]),
    );
    let outcome = review(&mut state, review_request).unwrap();
    assert_eq!(outcome.phases.as_slice(), PlannerPhase::ORDER.as_slice());
    assert_eq!(
        outcome
            .commands
            .iter()
            .map(|command| command.stage)
            .collect::<Vec<_>>(),
        [
            PlannerCommandStage::ResolveReportedSite,
            PlannerCommandStage::RequestReservation,
            PlannerCommandStage::RequestAssignment,
            PlannerCommandStage::Execute,
            PlannerCommandStage::Observe,
        ]
    );
    let encoded = serde_json::to_string(&outcome).unwrap();
    for forbidden in [
        "authoritative",
        "regeneration",
        "worldMutation",
        "hiddenStock",
    ] {
        assert!(!encoded.contains(forbidden));
    }
}

#[test]
fn exact_domain_ownership_and_founding_vacancy_fallback_are_visible() {
    assert_eq!(
        specialist_for(ContentDomain::Hole),
        Some(OfficerRole::Loremaster)
    );
    assert_eq!(
        specialist_for(ContentDomain::ResearchNotes),
        Some(OfficerRole::Loremaster)
    );
    assert_eq!(
        specialist_for(ContentDomain::Research),
        Some(OfficerRole::Loremaster)
    );
    assert_eq!(
        specialist_for(ContentDomain::VoidResearch),
        Some(OfficerRole::Loremaster)
    );
    assert_eq!(
        specialist_for(ContentDomain::Hunting),
        Some(OfficerRole::Captain)
    );
    assert_eq!(
        specialist_for(ContentDomain::Defense),
        Some(OfficerRole::Captain)
    );
    assert_eq!(
        specialist_for(ContentDomain::Danger),
        Some(OfficerRole::Captain)
    );
    for domain in [
        ContentDomain::Food,
        ContentDomain::Apples,
        ContentDomain::Fishing,
        ContentDomain::FoodDays,
        ContentDomain::Cookhouse,
    ] {
        assert_eq!(specialist_for(domain), Some(OfficerRole::Farmer));
    }
    for domain in [
        ContentDomain::Processing,
        ContentDomain::Tools,
        ContentDomain::Fixtures,
        ContentDomain::Augmentations,
    ] {
        assert_eq!(specialist_for(domain), Some(OfficerRole::ClothLeader));
    }

    let mut state = ContentPlannerState::new(id("colony", "one"));
    let review_request = request(
        &state,
        "vacancy",
        42,
        1,
        PlannerCompetence::Strong,
        input(vec![hole_candidate(
            "apples",
            ReportedFoodKind::Apples,
            20,
            10,
        )]),
    );
    let outcome = review(&mut state, review_request).unwrap();
    let goal_id = outcome.commands[0].goal_id.clone();
    let goal = state.live_goals.get(&goal_id).unwrap();
    assert!(matches!(
        &goal.owner,
        cat_sim::leader_planner::content_planner::PlannerOwner::FoundingLeaderVacancy(
            OfficerRole::Loremaster
        )
    ));
    assert_eq!(goal.confidence_basis_points, 6_000);
}

#[test]
fn strong_leader_uses_low_replacement_cost_and_weak_stale_leader_can_waste_scarce_food() {
    let mut apples = hole_candidate("apples", ReportedFoodKind::Apples, 2, 900);
    let mut fish = hole_candidate("fish", ReportedFoodKind::Fish, 50, 100);
    apples.ordered_fallbacks = vec![fish.id.clone()];
    fish.ordered_fallbacks = vec![apples.id.clone()];

    let mut strong = ContentPlannerState::new(id("colony", "one"));
    let strong_request = request(
        &strong,
        "strong",
        5,
        6 * 60,
        PlannerCompetence::Strong,
        input(vec![apples.clone(), fish.clone()]),
    );
    let strong_outcome = review(&mut strong, strong_request).unwrap();
    assert_eq!(
        selected_feed(&strong_outcome),
        ContentId::new("fish").unwrap()
    );
    assert_eq!(
        strong_outcome
            .commands
            .iter()
            .find(|command| command.stage == PlannerCommandStage::RequestReservation)
            .unwrap()
            .ordered_fallbacks,
        vec![id("candidate", "apples")]
    );

    let mut weak = ContentPlannerState::new(id("colony", "one"));
    let weak_request = request(
        &weak,
        "weak",
        5,
        6 * 60,
        PlannerCompetence::Weak,
        input(vec![fish, apples]),
    );
    let weak_outcome = review(&mut weak, weak_request).unwrap();
    assert_eq!(
        selected_feed(&weak_outcome),
        ContentId::new("apples").unwrap()
    );
}

#[test]
fn omission_is_keyed_per_review_and_officer_request_advances_exactly_one_band() {
    let candidate = hole_candidate("apples", ReportedFoodKind::Apples, 20, 10);
    let mut base = ContentPlannerState::new(id("colony", "one"));
    let seed = (1..100_000)
        .find(|seed| {
            let mut probe = base.clone();
            let mut probe_request = request(
                &probe,
                "one-band",
                *seed,
                1,
                PlannerCompetence::Ordinary,
                input(vec![candidate.clone()]),
            );
            probe_request.leader_level = EffectiveLevel::try_from(1).unwrap();
            review(&mut probe, probe_request).is_ok_and(|outcome| {
                outcome.omitted.len() == 1 && outcome.omitted[0].roll_basis_points >= 1_200
            })
        })
        .unwrap();
    let mut without = request(
        &base,
        "one-band",
        seed,
        1,
        PlannerCompetence::Ordinary,
        input(vec![candidate.clone()]),
    );
    without.leader_level = EffectiveLevel::try_from(1).unwrap();
    let omitted = review(&mut base, without).unwrap();
    assert_eq!(omitted.omitted.len(), 1);

    let mut covered_state = ContentPlannerState::new(id("colony", "one"));
    let mut covered = request(
        &covered_state,
        "one-band",
        seed,
        1,
        PlannerCompetence::Ordinary,
        input(vec![candidate.clone()]),
    );
    covered.leader_level = EffectiveLevel::try_from(1).unwrap();
    covered.officers = vec![OfficerCoverage {
        role: OfficerRole::Loremaster,
        officer_id: id("cat", "loremaster"),
        effective_level: EffectiveLevel::try_from(1).unwrap(),
    }];
    covered.officer_requests = vec![OfficerPlanRequest {
        request_id: id("request", "hole"),
        officer_role: OfficerRole::Loremaster,
        report_id: id("report", "hole"),
        expires_tick: 100,
        request: TypedOfficerRequest::Space {
            candidate_id: candidate.id,
            site_kind: ReportedSiteKind::HoleWorkArea,
            required_cells: 9,
        },
    }];
    let included = review(&mut covered_state, covered).unwrap();
    assert!(included.omitted.is_empty());
    assert!(!included.commands.is_empty());
}

#[test]
fn hole_is_endlessly_eligible_after_delivery_and_omission_never_completes_it() {
    let hole = hole_candidate("fish", ReportedFoodKind::Fish, 20, 10);
    let mut state = ContentPlannerState::new(id("colony", "one"));
    let first_request = request(
        &state,
        "hole-first",
        1,
        1,
        PlannerCompetence::Strong,
        input(vec![hole.clone()]),
    );
    let first = review(&mut state, first_request).unwrap();
    let first_goal = first.commands[0].goal_id.clone();

    let mut second_request = request(
        &state,
        "hole-second",
        2,
        2,
        PlannerCompetence::Strong,
        input(vec![hole]),
    );
    second_request.execution_feedback = vec![ExecutionFeedback {
        goal_id: first_goal.clone(),
        cargo_stage: CargoStage::Delivered,
        delivery_endpoint: None,
        salvage_endpoint: None,
        reported_delivery_route_viable: true,
        failure: None,
        report_id: id("report", "delivered"),
    }];
    let second = review(&mut state, second_request).unwrap();
    assert!(state.terminal_goals.contains_key(&first_goal));
    assert!(
        second
            .commands
            .iter()
            .any(|command| command.goal_id != first_goal)
    );
    assert_eq!(
        state.drain_terminal_goals(65),
        Err(ContentPlannerError::DrainTooLarge)
    );
    assert_eq!(state.drain_terminal_goals(1).unwrap().len(), 1);
}

#[test]
fn every_food_recovery_kind_requires_its_reported_physical_location() {
    let cases = [
        (
            LocatedFoodRecoveryKind::AppleTree,
            ReportedSiteKind::AppleTree,
        ),
        (
            LocatedFoodRecoveryKind::FishShore,
            ReportedSiteKind::FishShore,
        ),
        (
            LocatedFoodRecoveryKind::HuntingLair,
            ReportedSiteKind::HuntingLair,
        ),
        (
            LocatedFoodRecoveryKind::FarmPlot,
            ReportedSiteKind::FarmPlot,
        ),
        (
            LocatedFoodRecoveryKind::Cookhouse,
            ReportedSiteKind::Cookhouse,
        ),
    ];
    for (index, (recovery, site_kind)) in cases.into_iter().enumerate() {
        let mut state = ContentPlannerState::new(id("colony", &format!("c{index}")));
        let mut report = input(vec![candidate(
            &format!("recovery{index}"),
            ContentDomain::Food,
            CandidateKind::RecoverFood(recovery),
            Some(site(site_kind, &format!("site{index}"))),
        )]);
        report.colony_id = state.colony_id.clone();
        let review_request = request(
            &state,
            &format!("recovery{index}"),
            1,
            1,
            PlannerCompetence::Strong,
            report,
        );
        let outcome = review(&mut state, review_request).unwrap();
        assert!(outcome.commands.iter().any(|command| {
            command
                .site
                .as_ref()
                .is_some_and(|site| site.kind == site_kind)
        }));
    }
}

#[test]
fn defense_preempts_before_pickup_and_picked_cargo_has_delivery_or_salvage_intent() {
    let mut state = ContentPlannerState::new(id("colony", "one"));
    let hole = hole_candidate("meat", ReportedFoodKind::Meat, 20, 10);
    let first_request = request(
        &state,
        "cargo-first",
        1,
        1,
        PlannerCompetence::Strong,
        input(vec![hole.clone()]),
    );
    let first = review(&mut state, first_request).unwrap();
    let goal_id = first.commands[0].goal_id.clone();

    let defense = candidate(
        "attack",
        ContentDomain::Defense,
        CandidateKind::Defend,
        Some(site(ReportedSiteKind::DefenseSite, "wall")),
    );
    let mut before = request(
        &state,
        "before-pickup",
        2,
        2,
        PlannerCompetence::Strong,
        input(vec![defense.clone(), hole.clone()]),
    );
    before.execution_feedback = vec![ExecutionFeedback {
        goal_id: goal_id.clone(),
        cargo_stage: CargoStage::BeforePickup,
        delivery_endpoint: None,
        salvage_endpoint: None,
        reported_delivery_route_viable: false,
        failure: None,
        report_id: id("report", "before"),
    }];
    let preempted = review(&mut state, before).unwrap();
    assert_eq!(
        preempted.commands[0].stage,
        PlannerCommandStage::PreemptBeforePickup
    );

    let mut delivery_state = ContentPlannerState::new(id("colony", "one"));
    let delivery_first_request = request(
        &delivery_state,
        "delivery-first",
        1,
        1,
        PlannerCompetence::Strong,
        input(vec![hole.clone()]),
    );
    let delivery_first = review(&mut delivery_state, delivery_first_request).unwrap();
    let delivery_goal = delivery_first.commands[0].goal_id.clone();
    let mut picked = request(
        &delivery_state,
        "picked",
        2,
        2,
        PlannerCompetence::Strong,
        input(vec![defense, hole]),
    );
    picked.execution_feedback = vec![ExecutionFeedback {
        goal_id: delivery_goal,
        cargo_stage: CargoStage::PickedUp,
        delivery_endpoint: Some(site(ReportedSiteKind::HoleWorkArea, "hole")),
        salvage_endpoint: Some(site(ReportedSiteKind::Stockpile, "safe_store")),
        reported_delivery_route_viable: false,
        failure: None,
        report_id: id("report", "picked"),
    }];
    let salvaged = review(&mut delivery_state, picked).unwrap();
    assert!(matches!(
        &salvaged.commands[0].cargo_intent,
        CargoIntent::SalvagePicked { .. }
    ));
}

#[test]
fn god_and_planner_consume_byte_equivalent_report_safe_inputs() {
    let report = input(vec![hole_candidate(
        "meal",
        ReportedFoodKind::Meal,
        10,
        100,
    )]);
    assert_eq!(
        planner_report_bytes(&report).unwrap(),
        god_report_bytes(&report).unwrap()
    );
    let bytes = String::from_utf8(planner_report_bytes(&report).unwrap()).unwrap();
    for forbidden in ["hidden", "authoritative", "regeneration", "respawn"] {
        assert!(!bytes.contains(forbidden));
    }
}

#[test]
fn office_room_and_tools_raise_only_effective_not_personal_expertise() {
    let personal = ExpertiseLevel::Two;
    let support = OfficeExpertiseSupport {
        office_id: id("office", "loremaster"),
        room_operational: true,
        required_tool_id: Some(id("tool", "microscope")),
        required_tool_operational: true,
    };
    assert_eq!(
        officer_effective_level(personal, support.effective_bonuses()),
        ExpertiseLevel::Four
    );
    assert_eq!(personal, ExpertiseLevel::Two);
}

#[test]
fn specialist_keep_stock_orders_are_bounded_and_emit_only_when_reported_below_minimum() {
    let loremaster = OfficerCoverage {
        role: OfficerRole::Loremaster,
        officer_id: id("cat", "loremaster"),
        effective_level: EffectiveLevel::try_from(5).unwrap(),
    };
    let mut state = ContentPlannerState::new(id("colony", "one"));
    state
        .install_standing_order(
            KeepStockOrder {
                id: id("standing", "notes_inputs"),
                officer_role: OfficerRole::Loremaster,
                content_id: ContentId::new("wood").unwrap(),
                minimum_units: 10,
                target_units: 20,
                created_tick: 0,
            },
            std::slice::from_ref(&loremaster),
        )
        .unwrap();
    for index in 1..6 {
        state
            .install_standing_order(
                KeepStockOrder {
                    id: id("standing", &format!("bounded_{index}")),
                    officer_role: OfficerRole::Loremaster,
                    content_id: ContentId::new("wood").unwrap(),
                    minimum_units: 10,
                    target_units: 20,
                    created_tick: index,
                },
                std::slice::from_ref(&loremaster),
            )
            .unwrap();
    }
    assert_eq!(
        state.install_standing_order(
            KeepStockOrder {
                id: id("standing", "overflow"),
                officer_role: OfficerRole::Loremaster,
                content_id: ContentId::new("wood").unwrap(),
                minimum_units: 10,
                target_units: 20,
                created_tick: 7,
            },
            std::slice::from_ref(&loremaster),
        ),
        Err(ContentPlannerError::StandingOrderCapacityReached)
    );
    let mut keep = candidate(
        "keep_wood",
        ContentDomain::ResearchNotes,
        CandidateKind::KeepStock,
        Some(site(ReportedSiteKind::Workshop, "workshop")),
    );
    keep.cargo = Some(cargo("wood", ReportedFoodKind::Other, 5, 10));
    let mut review_request = request(
        &state,
        "keep",
        1,
        1,
        PlannerCompetence::Strong,
        input(vec![keep]),
    );
    review_request.officers = vec![loremaster];
    assert!(
        !review(&mut state, review_request)
            .unwrap()
            .commands
            .is_empty()
    );

    let mut vacancy = ContentPlannerState::new(id("colony", "one"));
    assert_eq!(
        vacancy.install_standing_order(
            KeepStockOrder {
                id: id("standing", "bad"),
                officer_role: OfficerRole::Loremaster,
                content_id: ContentId::new("wood").unwrap(),
                minimum_units: 1,
                target_units: 2,
                created_tick: 0,
            },
            &[],
        ),
        Err(ContentPlannerError::StandingOrderRequiresSpecialist)
    );
}

#[test]
fn typed_dependency_space_and_workshop_requests_persist_and_bridge_to_plans() {
    let officer = id("cat", "farmer");
    let actor = AuthorityActor::Officer {
        cat_id: officer.clone(),
        role: OfficerRole::Farmer,
    };
    let belief_key = BeliefKey::new(
        id("domain", "food"),
        id("subject", "apples"),
        BeliefKind::Stock,
    );
    let evidence = EvidenceId::derive("colony-one", &belief_key, 0, &officer, 0);
    let report = ReportId::derive(&evidence, &officer);
    let base = OfficerRequestDraft {
        source_domain: AuthorityDomain::Farming,
        target_domain: AuthorityDomain::Building,
        kind: RequestKind::Building,
        target_id: id("candidate", "apple_recovery"),
        quantity: 1,
        base_urgency: BasisPoints::new(5_000),
        rationale_id: id("rationale", "apple_space"),
        evidence_ids: BTreeSet::from([evidence]),
        report_ids: BTreeSet::from([report]),
        confidence: Confidence::new(8_000).unwrap(),
        estimated_resource_cost: 10,
        estimated_labor_ticks: 10,
    };
    let mut book = OfficerRequestBook::new();
    book.propose_typed(
        &actor,
        AuthorityContext {
            leader_present: true,
            player_authorized: false,
        },
        id("colony", "one"),
        officer.clone(),
        OfficerRole::Farmer,
        TypedOfficerRequestDraft {
            request: base.clone(),
            payload: OfficerRequestPayload::Space {
                kind: RequestedSpaceKind::AppleTree,
                required_cells: 1,
            },
        },
        structured_request_budget(ExpertiseLevel::One),
        0,
        TICKS_PER_GAME_HOUR,
    )
    .unwrap();
    let mut keep_stock = OfficerRequestDraft {
        source_domain: AuthorityDomain::Farming,
        target_domain: AuthorityDomain::Farming,
        kind: RequestKind::Operational,
        target_id: id("candidate", "keep_apples"),
        quantity: 20,
        base_urgency: BasisPoints::new(4_000),
        rationale_id: id("rationale", "keep_apples"),
        evidence_ids: BTreeSet::new(),
        report_ids: BTreeSet::new(),
        confidence: Confidence::new(8_000).unwrap(),
        estimated_resource_cost: 10,
        estimated_labor_ticks: 10,
    };
    keep_stock.evidence_ids.insert(EvidenceId::derive(
        "colony-one",
        &BeliefKey::new(
            id("domain", "food"),
            id("subject", "keep_apples"),
            BeliefKind::Stock,
        ),
        0,
        &id("cat", "farmer"),
        0,
    ));
    keep_stock.report_ids = base.report_ids.clone();
    book.propose_typed(
        &actor,
        AuthorityContext {
            leader_present: true,
            player_authorized: false,
        },
        id("colony", "one"),
        id("cat", "farmer"),
        OfficerRole::Farmer,
        TypedOfficerRequestDraft {
            request: keep_stock,
            payload: OfficerRequestPayload::KeepStock {
                content_id: ContentId::new("apples").unwrap(),
                minimum_units: 10,
                target_units: 20,
            },
        },
        structured_request_budget(ExpertiseLevel::One),
        0,
        TICKS_PER_GAME_HOUR,
    )
    .unwrap();
    let mut dependency = base.clone();
    dependency.target_id = id("candidate", "cookhouse_supply");
    book.propose_typed(
        &actor,
        AuthorityContext {
            leader_present: true,
            player_authorized: false,
        },
        id("colony", "one"),
        officer.clone(),
        OfficerRole::Farmer,
        TypedOfficerRequestDraft {
            request: dependency,
            payload: OfficerRequestPayload::Dependency {
                dependency_target_id: id("candidate", "apple_recovery"),
            },
        },
        structured_request_budget(ExpertiseLevel::One),
        0,
        TICKS_PER_GAME_HOUR,
    )
    .unwrap();
    let mut workshop = base;
    workshop.target_id = id("candidate", "cookhouse_workshop");
    book.propose_typed(
        &actor,
        AuthorityContext {
            leader_present: true,
            player_authorized: false,
        },
        id("colony", "one"),
        officer,
        OfficerRole::Farmer,
        TypedOfficerRequestDraft {
            request: workshop,
            payload: OfficerRequestPayload::Workshop {
                station_id: id("station", "cookhouse"),
                operation_id: id("operation", "supply"),
            },
        },
        structured_request_budget(ExpertiseLevel::One),
        0,
        TICKS_PER_GAME_HOUR,
    )
    .unwrap();
    let restored: OfficerRequestBook =
        serde_json::from_str(&serde_json::to_string(&book).unwrap()).unwrap();
    let bridged = officer_plan_requests(&restored, 1).unwrap();
    assert_eq!(bridged.len(), 3);
    assert_eq!(keep_stock_orders(&restored, 1).unwrap().len(), 1);
    assert!(bridged.iter().any(|request| matches!(
        &request.request,
        TypedOfficerRequest::Space {
            site_kind: ReportedSiteKind::AppleTree,
            required_cells: 1,
            ..
        }
    )));
    assert!(
        bridged
            .iter()
            .any(|request| matches!(&request.request, TypedOfficerRequest::Dependency { .. }))
    );
    assert!(
        bridged
            .iter()
            .any(|request| matches!(&request.request, TypedOfficerRequest::Workshop { .. }))
    );

    let apple = candidate(
        "apple_recovery",
        ContentDomain::Food,
        CandidateKind::RecoverFood(LocatedFoodRecoveryKind::AppleTree),
        Some(site(ReportedSiteKind::AppleTree, "apple_tree")),
    );
    let supply = candidate(
        "cookhouse_supply",
        ContentDomain::Cookhouse,
        CandidateKind::SupplyCookhouse,
        Some(site(ReportedSiteKind::Cookhouse, "cookhouse")),
    );
    let workshop_goal = candidate(
        "cookhouse_workshop",
        ContentDomain::Cookhouse,
        CandidateKind::SupplyCookhouse,
        Some(site(ReportedSiteKind::Cookhouse, "cookhouse")),
    );
    let mut planner = ContentPlannerState::new(id("colony", "one"));
    let mut planner_request = request(
        &planner,
        "typed-flow",
        1,
        1,
        PlannerCompetence::Strong,
        input(vec![workshop_goal, supply, apple]),
    );
    planner_request.officers = vec![OfficerCoverage {
        role: OfficerRole::Farmer,
        officer_id: id("cat", "farmer"),
        effective_level: EffectiveLevel::try_from(5).unwrap(),
    }];
    planner_request.officer_requests = bridged;
    review(&mut planner, planner_request).unwrap();
    let apple_goal = planner
        .live_goals
        .values()
        .find(|goal| goal.candidate_id == id("candidate", "apple_recovery"))
        .unwrap()
        .id
        .clone();
    let supply_goal = planner
        .live_goals
        .values()
        .find(|goal| goal.candidate_id == id("candidate", "cookhouse_supply"))
        .unwrap();
    assert!(supply_goal.dependencies.contains(&apple_goal));
    let workshop_goal = planner
        .live_goals
        .values()
        .find(|goal| goal.candidate_id == id("candidate", "cookhouse_workshop"))
        .unwrap();
    assert!(
        workshop_goal
            .requirements
            .iter()
            .any(|requirement| matches!(requirement, GoalRequirement::Workshop { .. }))
    );
}

#[test]
fn restart_order_partition_replay_and_conflict_twins_are_atomic() {
    let hole = hole_candidate("fish", ReportedFoodKind::Fish, 20, 10);
    let defense = candidate(
        "defense",
        ContentDomain::Defense,
        CandidateKind::Defend,
        Some(site(ReportedSiteKind::DefenseSite, "wall")),
    );
    let report = input(vec![hole.clone(), defense]);
    let mut reversed_report = report.clone();
    reversed_report.candidates.reverse();
    let mut forward = ContentPlannerState::new(id("colony", "one"));
    let first = request(&forward, "r1", 11, 10, PlannerCompetence::Strong, report);
    let mut reversed_state = ContentPlannerState::new(id("colony", "one"));
    let reversed_request = request(
        &reversed_state,
        "r1",
        11,
        10,
        PlannerCompetence::Strong,
        reversed_report,
    );
    review(&mut reversed_state, reversed_request).unwrap();
    let first_outcome = review(&mut forward, first.clone()).unwrap();
    assert_eq!(
        serde_json::to_vec(&forward).unwrap(),
        serde_json::to_vec(&reversed_state).unwrap()
    );
    let before_replay = serde_json::to_vec(&forward).unwrap();
    assert_eq!(review(&mut forward, first.clone()).unwrap(), first_outcome);
    assert_eq!(serde_json::to_vec(&forward).unwrap(), before_replay);

    let mut conflict = first;
    conflict.world_seed += 1;
    assert_eq!(
        review(&mut forward, conflict),
        Err(ContentPlannerError::ReplayConflict)
    );
    assert_eq!(serde_json::to_vec(&forward).unwrap(), before_replay);

    let mut restarted: ContentPlannerState =
        serde_json::from_slice(&serde_json::to_vec(&forward).unwrap()).unwrap();
    let second = request(
        &restarted,
        "r2",
        12,
        20,
        PlannerCompetence::Strong,
        input(vec![hole.clone()]),
    );
    review(&mut restarted, second).unwrap();

    let mut partitioned = forward;
    let second_partition = request(
        &partitioned,
        "r2",
        12,
        20,
        PlannerCompetence::Strong,
        input(vec![hole]),
    );
    review(&mut partitioned, second_partition).unwrap();
    assert_eq!(
        serde_json::to_vec(&restarted).unwrap(),
        serde_json::to_vec(&partitioned).unwrap()
    );
}

#[test]
fn strict_version_unknown_field_and_invalid_fallback_decode_fail_closed() {
    let state = ContentPlannerState::new(id("colony", "one"));
    let mut value = serde_json::to_value(&state).unwrap();
    value["schemaVersion"] = serde_json::json!(99);
    assert!(serde_json::from_value::<ContentPlannerState>(value).is_err());
    let mut unknown = serde_json::to_value(&state).unwrap();
    unknown["hiddenTruth"] = serde_json::json!(1);
    assert!(serde_json::from_value::<ContentPlannerState>(unknown).is_err());
    let mut report_version = serde_json::to_value(input(Vec::new())).unwrap();
    report_version["schemaVersion"] = serde_json::json!(2);
    assert!(serde_json::from_value::<ReportSafePlanningInput>(report_version).is_err());

    let mut bad = hole_candidate("fish", ReportedFoodKind::Fish, 20, 10);
    bad.ordered_fallbacks = vec![id("candidate", "missing")];
    let mut target = ContentPlannerState::new(id("colony", "one"));
    let bad_request = request(
        &target,
        "bad-fallback",
        1,
        1,
        PlannerCompetence::Strong,
        input(vec![bad]),
    );
    assert_eq!(
        review(&mut target, bad_request),
        Err(ContentPlannerError::UnknownFallback)
    );
    assert_eq!(target.version, 0);
}

#[test]
fn picked_cargo_feedback_requires_both_delivery_and_salvage_endpoints() {
    let feedback = ExecutionFeedback {
        goal_id: id("goal", "one"),
        cargo_stage: CargoStage::PickedUp,
        delivery_endpoint: Some(site(ReportedSiteKind::HoleWorkArea, "hole")),
        salvage_endpoint: None,
        reported_delivery_route_viable: true,
        failure: Some(RecoveryReason::ReportedRouteLoss),
        report_id: id("report", "route"),
    };
    let mut state = ContentPlannerState::new(id("colony", "one"));
    let mut review_request = request(
        &state,
        "missing-salvage",
        1,
        1,
        PlannerCompetence::Strong,
        input(Vec::new()),
    );
    review_request.execution_feedback = vec![feedback];
    assert_eq!(
        review(&mut state, review_request),
        Err(ContentPlannerError::MissingCargoDisposition)
    );
}
