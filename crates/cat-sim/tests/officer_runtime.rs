//! LAI.16 officer runtime leaf coverage: cadence, report emission, budgets,
//! request adoption, and deterministic persistence.

use std::collections::BTreeSet;

use cat_sim::{
    authority::{AuthorityActor, AuthorityContext, AuthorityDomain},
    beliefs::{
        BeliefKey, BeliefKind, BeliefStore, BeliefValue, Confidence, EvidenceId,
        ProjectedBeliefValue, ReportId,
    },
    officer_expertise::{
        ExpertiseBonuses, ExpertiseLevel, OfficerInstitutionState, OfficerReportFact,
        emit_officer_report, officer_cadence_ticks,
    },
    officer_requests::{
        LIVE_OFFICER_REQUEST_CAPACITY, OfficerRequestBook, OfficerRequestDraft,
        OfficerRequestError, RequestInsert, RequestKind, structured_request_budget,
    },
    officers::OfficerRole,
    planner_core::{BasisPoints, PlannerId},
};

const TICKS_PER_GAME_HOUR: u64 = 60;

fn planner_id(namespace: &str, value: &str) -> PlannerId {
    PlannerId::derive(namespace, [value])
}

fn cat(value: &str) -> PlannerId {
    planner_id("cat", value)
}

fn key(kind: BeliefKind, subject: &str) -> BeliefKey {
    BeliefKey::new(
        planner_id("domain", "officer-runtime"),
        planner_id("subject", subject),
        kind,
    )
}

#[test]
fn officer_cadence_advances_one_boundary_at_a_time_and_survives_restart() {
    let mut late = OfficerInstitutionState::new("colony-1").unwrap();
    late.open_office(OfficerRole::Farmer, 0).unwrap();
    late.appoint_officer(OfficerRole::Farmer, cat("farmer"), 0)
        .unwrap();
    assert_eq!(
        officer_cadence_ticks(ExpertiseLevel::One, TICKS_PER_GAME_HOUR).unwrap(),
        360
    );
    assert_eq!(
        late.officer_runtime_due(
            OfficerRole::Farmer,
            359,
            TICKS_PER_GAME_HOUR,
            ExpertiseBonuses::default()
        )
        .unwrap(),
        None
    );

    let first_late = late
        .complete_officer_runtime_review(
            OfficerRole::Farmer,
            900,
            TICKS_PER_GAME_HOUR,
            ExpertiseBonuses::default(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(first_late.due_tick, 360);
    let restarted: OfficerInstitutionState =
        serde_json::from_str(&serde_json::to_string(&late).unwrap()).unwrap();
    assert_eq!(restarted, late);
    let second_late = late
        .complete_officer_runtime_review(
            OfficerRole::Farmer,
            900,
            TICKS_PER_GAME_HOUR,
            ExpertiseBonuses::default(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(second_late.due_tick, 720);

    let mut partitioned = OfficerInstitutionState::new("colony-1").unwrap();
    partitioned.open_office(OfficerRole::Farmer, 0).unwrap();
    partitioned
        .appoint_officer(OfficerRole::Farmer, cat("farmer"), 0)
        .unwrap();
    partitioned
        .complete_officer_runtime_review(
            OfficerRole::Farmer,
            360,
            TICKS_PER_GAME_HOUR,
            ExpertiseBonuses::default(),
        )
        .unwrap();
    partitioned
        .complete_officer_runtime_review(
            OfficerRole::Farmer,
            720,
            TICKS_PER_GAME_HOUR,
            ExpertiseBonuses::default(),
        )
        .unwrap();
    assert_eq!(
        serde_json::to_string(&late).unwrap(),
        serde_json::to_string(&partitioned).unwrap()
    );
}

#[test]
fn officer_reports_hide_regeneration_through_level_three_and_emit_only_ranges_later() {
    let mut state = OfficerInstitutionState::new("colony-1").unwrap();
    state.open_office(OfficerRole::Accountant, 0).unwrap();
    state
        .appoint_officer(OfficerRole::Accountant, cat("accountant"), 0)
        .unwrap();
    state
        .record_completed_duty_hours(cat("accountant"), OfficerRole::Accountant, 96)
        .unwrap();
    let level_three = state
        .complete_officer_runtime_review(
            OfficerRole::Accountant,
            60,
            TICKS_PER_GAME_HOUR,
            ExpertiseBonuses::default(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(level_three.effective_level, ExpertiseLevel::Three);
    let regen_key = key(BeliefKind::Regeneration, "grove");
    assert_eq!(
        emit_officer_report(
            "colony-1",
            &level_three,
            regen_key.clone(),
            OfficerReportFact::RegenerationEstimate { estimate: 10 },
            Confidence::new(8_000).unwrap(),
            TICKS_PER_GAME_HOUR,
            0,
        )
        .unwrap(),
        None
    );
    assert_eq!(
        emit_officer_report(
            "colony-1",
            &level_three,
            regen_key.clone(),
            OfficerReportFact::RegenerationEstimate { estimate: 9_999 },
            Confidence::new(8_000).unwrap(),
            TICKS_PER_GAME_HOUR,
            1,
        )
        .unwrap(),
        None
    );

    state
        .record_completed_duty_hours(cat("accountant"), OfficerRole::Accountant, 144)
        .unwrap();
    let level_four = state
        .complete_officer_runtime_review(
            OfficerRole::Accountant,
            90,
            TICKS_PER_GAME_HOUR,
            ExpertiseBonuses::default(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(level_four.effective_level, ExpertiseLevel::Four);
    let report = emit_officer_report(
        "colony-1",
        &level_four,
        regen_key.clone(),
        OfficerReportFact::RegenerationEstimate { estimate: 80 },
        Confidence::new(8_000).unwrap(),
        TICKS_PER_GAME_HOUR,
        0,
    )
    .unwrap()
    .unwrap();
    let BeliefValue::Estimate(range) = &report.observation.value else {
        panic!("regeneration report must be an estimate range");
    };
    assert!(range.lower_bound < range.estimate);
    assert!(range.estimate < range.upper_bound);

    let mut store = BeliefStore::new();
    store.apply_report(report).unwrap();
    assert!(matches!(
        store
            .project(&regen_key, level_four.due_tick)
            .unwrap()
            .value,
        ProjectedBeliefValue::RegenerationRange(_)
    ));
}

#[test]
fn structured_requests_enforce_budgets_bounds_and_order_stable_merge() {
    let officer = cat("farmer");
    let actor = AuthorityActor::Officer {
        cat_id: officer.clone(),
        role: OfficerRole::Farmer,
    };
    let context = AuthorityContext {
        leader_present: true,
        player_authorized: false,
    };
    let target = planner_id("building", "storehouse");
    let evidence = EvidenceId::derive("colony-1", &key(BeliefKind::Stock, "food"), 0, &officer, 0);
    let report = ReportId::derive(&evidence, &officer);
    let mut draft = OfficerRequestDraft {
        source_domain: AuthorityDomain::Farming,
        target_domain: AuthorityDomain::Building,
        kind: RequestKind::Building,
        target_id: target.clone(),
        quantity: 1,
        base_urgency: BasisPoints::new(4_000),
        rationale_id: planner_id("rationale", "food-storage"),
        evidence_ids: BTreeSet::from([evidence.clone()]),
        report_ids: BTreeSet::from([report.clone()]),
        confidence: Confidence::new(7_500).unwrap(),
        estimated_resource_cost: 26,
        estimated_labor_ticks: 60,
    };
    assert_eq!(
        structured_request_budget(ExpertiseLevel::One).resource_limit,
        25
    );
    let mut rejected = OfficerRequestBook::new();
    assert_eq!(
        rejected.propose_structured(
            &actor,
            context,
            planner_id("colony", "one"),
            officer.clone(),
            OfficerRole::Farmer,
            draft.clone(),
            structured_request_budget(ExpertiseLevel::One),
            0,
            TICKS_PER_GAME_HOUR,
        ),
        Err(OfficerRequestError::BudgetExceeded)
    );
    assert_eq!(rejected.iter().len(), 0);

    draft.estimated_resource_cost = 25;
    let mut forward = OfficerRequestBook::new();
    let inserted = forward
        .propose_structured(
            &actor,
            context,
            planner_id("colony", "one"),
            officer.clone(),
            OfficerRole::Farmer,
            draft.clone(),
            structured_request_budget(ExpertiseLevel::One),
            0,
            TICKS_PER_GAME_HOUR,
        )
        .unwrap();
    assert!(matches!(inserted, RequestInsert::Inserted(_)));

    let mut second = draft.clone();
    second.evidence_ids = BTreeSet::from([EvidenceId::derive(
        "colony-1",
        &key(BeliefKind::Stock, "food"),
        1,
        &officer,
        1,
    )]);
    second.report_ids = BTreeSet::new();
    assert!(matches!(
        forward
            .propose_structured(
                &actor,
                context,
                planner_id("colony", "one"),
                officer.clone(),
                OfficerRole::Farmer,
                second.clone(),
                structured_request_budget(ExpertiseLevel::One),
                1,
                TICKS_PER_GAME_HOUR,
            )
            .unwrap(),
        RequestInsert::Merged(_)
    ));

    let mut reverse = OfficerRequestBook::new();
    for item in [second, draft.clone()] {
        reverse
            .propose_structured(
                &actor,
                context,
                planner_id("colony", "one"),
                officer.clone(),
                OfficerRole::Farmer,
                item,
                structured_request_budget(ExpertiseLevel::One),
                0,
                TICKS_PER_GAME_HOUR,
            )
            .unwrap();
    }
    assert_eq!(
        serde_json::to_string(&forward).unwrap(),
        serde_json::to_string(&reverse).unwrap()
    );

    let mut full = OfficerRequestBook::new();
    for index in 0..LIVE_OFFICER_REQUEST_CAPACITY {
        let mut bounded = draft.clone();
        bounded.target_id = planner_id("bounded-target", &index.to_string());
        full.propose_structured(
            &actor,
            context,
            planner_id("colony", "one"),
            officer.clone(),
            OfficerRole::Farmer,
            bounded,
            structured_request_budget(ExpertiseLevel::Five),
            index as u64,
            TICKS_PER_GAME_HOUR,
        )
        .unwrap();
    }
    let mut overflow = draft;
    overflow.target_id = planner_id("bounded-target", "overflow");
    assert_eq!(
        full.propose_structured(
            &actor,
            context,
            planner_id("colony", "one"),
            officer,
            OfficerRole::Farmer,
            overflow,
            structured_request_budget(ExpertiseLevel::Five),
            200,
            TICKS_PER_GAME_HOUR,
        ),
        Err(OfficerRequestError::LiveCapacityReached)
    );
}

#[test]
fn vacancy_and_successor_adopt_live_requests_without_rekeying_identity() {
    let old_officer = cat("old-farmer");
    let new_officer = cat("new-farmer");
    let actor = AuthorityActor::Officer {
        cat_id: old_officer.clone(),
        role: OfficerRole::Farmer,
    };
    let context = AuthorityContext {
        leader_present: true,
        player_authorized: false,
    };
    let mut book = OfficerRequestBook::new();
    let insert = book
        .propose_structured(
            &actor,
            context,
            planner_id("colony", "one"),
            old_officer.clone(),
            OfficerRole::Farmer,
            OfficerRequestDraft {
                source_domain: AuthorityDomain::Farming,
                target_domain: AuthorityDomain::Building,
                kind: RequestKind::Building,
                target_id: planner_id("building", "granary"),
                quantity: 1,
                base_urgency: BasisPoints::new(5_000),
                rationale_id: planner_id("rationale", "granary"),
                evidence_ids: BTreeSet::new(),
                report_ids: BTreeSet::new(),
                confidence: Confidence::new(8_000).unwrap(),
                estimated_resource_cost: 10,
                estimated_labor_ticks: 30,
            },
            structured_request_budget(ExpertiseLevel::One),
            0,
            TICKS_PER_GAME_HOUR,
        )
        .unwrap();
    let RequestInsert::Inserted(request_id) = insert else {
        panic!("first request must insert");
    };

    let mut state = OfficerInstitutionState::new("colony-1").unwrap();
    state.open_office(OfficerRole::Farmer, 0).unwrap();
    state
        .appoint_officer(OfficerRole::Farmer, old_officer.clone(), 0)
        .unwrap();
    state.officer_died(&old_officer, 10).unwrap().unwrap();
    let transition = state
        .appoint_officer(OfficerRole::Farmer, new_officer.clone(), 20)
        .unwrap();
    assert_eq!(
        transition.adopt_requests(&mut book),
        vec![request_id.clone()]
    );
    let adopted = book.get(&request_id).unwrap();
    assert_eq!(adopted.officer_id, old_officer);
    assert_eq!(adopted.adopted_by_officer_id, Some(new_officer));
    assert_eq!(book.get(&request_id).unwrap().id, request_id);
}
