//! Focused pre-implementation LAI.16 tests for officer expertise and succession.

use cat_sim::{
    authority::{AuthorityDomain, officer_owns_domain},
    beliefs::ReportLevel,
    officer_expertise::{
        AppointmentCandidate, ExpertiseBonuses, ExpertiseLevel, InstitutionError, LeaderTransition,
        MAX_APPOINTMENT_CANDIDATES, OfficerInstitutionState, ReportFlowCapability,
        appointment_candidate_limit, domains_for, effective_level, officer_cadence_minutes,
        personal_level, report_capability, select_appointment_candidate,
    },
    officer_requests::OfficerRequestBook,
    officers::OfficerRole,
    planner_core::PlannerId,
};

fn cat(id: &str) -> PlannerId {
    PlannerId::derive("cat", [id])
}

fn candidates(count: usize) -> Vec<AppointmentCandidate> {
    (0..count)
        .map(|index| AppointmentCandidate {
            cat_id: cat(&format!("cat-{index:02}")),
            believed_merit: i64::try_from(index).unwrap(),
            eligible: true,
        })
        .collect()
}

#[test]
fn seven_roles_have_exact_authority_domains() {
    let expected = [
        (
            OfficerRole::Steward,
            &[
                AuthorityDomain::Survival,
                AuthorityDomain::Evacuation,
                AuthorityDomain::Stewardship,
                AuthorityDomain::Building,
            ][..],
        ),
        (OfficerRole::Accountant, &[AuthorityDomain::Accounting][..]),
        (OfficerRole::Forester, &[AuthorityDomain::Forestry][..]),
        (
            OfficerRole::Farmer,
            &[AuthorityDomain::Survival, AuthorityDomain::Farming][..],
        ),
        (OfficerRole::Captain, &[AuthorityDomain::Defense][..]),
        (OfficerRole::Loremaster, &[AuthorityDomain::Research][..]),
        (OfficerRole::ClothLeader, &[AuthorityDomain::Textiles][..]),
    ];
    assert_eq!(OfficerRole::ALL.len(), 7);
    for (role, domains) in expected {
        assert_eq!(domains_for(role).collect::<Vec<_>>(), domains);
        for domain in [
            AuthorityDomain::Survival,
            AuthorityDomain::Evacuation,
            AuthorityDomain::Stewardship,
            AuthorityDomain::Accounting,
            AuthorityDomain::Forestry,
            AuthorityDomain::Farming,
            AuthorityDomain::Defense,
            AuthorityDomain::Research,
            AuthorityDomain::Textiles,
            AuthorityDomain::Building,
            AuthorityDomain::Diplomacy,
            AuthorityDomain::Trade,
            AuthorityDomain::ColonyWide,
        ] {
            assert_eq!(domains.contains(&domain), officer_owns_domain(role, domain));
        }
    }
}

#[test]
fn duty_thresholds_bonuses_cadence_and_report_levels_are_exact() {
    let thresholds = [
        (0, ExpertiseLevel::One),
        (23 * 60 + 59, ExpertiseLevel::One),
        (24 * 60, ExpertiseLevel::Two),
        (96 * 60, ExpertiseLevel::Three),
        (240 * 60, ExpertiseLevel::Four),
        (480 * 60, ExpertiseLevel::Five),
        (u64::MAX, ExpertiseLevel::Five),
    ];
    for (minutes, expected) in thresholds {
        assert_eq!(personal_level(minutes), expected);
    }
    assert_eq!(
        effective_level(ExpertiseLevel::Two, ExpertiseBonuses::default()),
        ExpertiseLevel::Two
    );
    assert_eq!(
        effective_level(
            ExpertiseLevel::Two,
            ExpertiseBonuses {
                workflow_operational: true,
                reinforcement_operational: true,
            },
        ),
        ExpertiseLevel::Four
    );
    assert_eq!(
        effective_level(
            ExpertiseLevel::Five,
            ExpertiseBonuses {
                workflow_operational: true,
                reinforcement_operational: true,
            },
        ),
        ExpertiseLevel::Five
    );
    assert_eq!(
        [1, 2, 3, 4, 5]
            .map(|level| officer_cadence_minutes(ExpertiseLevel::try_from(level).unwrap())),
        [360, 180, 60, 30, 15]
    );

    let expected = [
        (ReportLevel::One, ReportFlowCapability::None, None),
        (ReportLevel::Two, ReportFlowCapability::Trend, None),
        (
            ReportLevel::Three,
            ReportFlowCapability::CoarseObservedRange,
            None,
        ),
        (
            ReportLevel::Four,
            ReportFlowCapability::NumericObservedRate,
            Some(2_500),
        ),
        (
            ReportLevel::Five,
            ReportFlowCapability::HighConfidenceNumericObservedRate,
            Some(1_000),
        ),
    ];
    for (raw, (level, flow, regeneration_error)) in (1..=5).zip(expected) {
        let capability = report_capability(ExpertiseLevel::try_from(raw).unwrap());
        assert_eq!(capability.level, level);
        assert_eq!(capability.flow, flow);
        assert_eq!(
            capability.regeneration_estimate_error_basis_points,
            regeneration_error
        );
    }
}

#[test]
fn low_level_report_projection_cannot_leak_hidden_regeneration() {
    for level in [
        ExpertiseLevel::One,
        ExpertiseLevel::Two,
        ExpertiseLevel::Three,
    ] {
        let project = |hidden_regeneration: i64| {
            let _executor_only = hidden_regeneration;
            serde_json::to_string(&report_capability(level)).unwrap()
        };
        assert_eq!(project(1), project(9_999_999));
        assert!(!project(1).contains("regeneration"));
    }
    let level_four = serde_json::to_string(&report_capability(ExpertiseLevel::Four)).unwrap();
    assert!(level_four.contains("regenerationEstimateErrorBasisPoints"));
    assert!(!level_four.contains("exact"));
}

#[test]
fn appointment_samples_exact_limits_without_replacement_and_is_order_stable() {
    let original = candidates(20);
    let mut reversed = original.clone();
    reversed.reverse();
    for (level, limit) in [(1, 3), (2, 5), (3, 8), (4, 12), (5, 20)] {
        let level = ExpertiseLevel::try_from(level).unwrap();
        assert_eq!(appointment_candidate_limit(level, 20), limit);
        let forward = select_appointment_candidate(
            42,
            "colony-1",
            OfficerRole::Forester,
            7,
            level,
            original.clone(),
        )
        .unwrap()
        .unwrap();
        let backward = select_appointment_candidate(
            42,
            "colony-1",
            OfficerRole::Forester,
            7,
            level,
            reversed.clone(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(forward, backward);
        assert_eq!(forward.sampled_cat_ids.len(), limit);
        let unique = forward
            .sampled_cat_ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(unique.len(), limit);
        assert!(forward.sampled_cat_ids.contains(&forward.selected_cat_id));
    }
}

#[test]
fn appointment_uses_believed_merit_stable_ties_and_keyed_occurrences() {
    let tied = vec![
        AppointmentCandidate {
            cat_id: cat("b"),
            believed_merit: 100,
            eligible: true,
        },
        AppointmentCandidate {
            cat_id: cat("a"),
            believed_merit: 100,
            eligible: true,
        },
        AppointmentCandidate {
            cat_id: cat("z"),
            believed_merit: 999,
            eligible: false,
        },
    ];
    let chosen = select_appointment_candidate(
        9,
        "colony",
        OfficerRole::Captain,
        0,
        ExpertiseLevel::Five,
        tied,
    )
    .unwrap()
    .unwrap();
    assert_eq!(chosen.selected_cat_id, cat("a"));
    assert!(!chosen.sampled_cat_ids.contains(&cat("z")));

    let pool = candidates(30);
    let first = select_appointment_candidate(
        9,
        "colony",
        OfficerRole::Captain,
        0,
        ExpertiseLevel::One,
        pool.clone(),
    )
    .unwrap()
    .unwrap();
    let next = select_appointment_candidate(
        9,
        "colony",
        OfficerRole::Captain,
        1,
        ExpertiseLevel::One,
        pool,
    )
    .unwrap()
    .unwrap();
    assert_ne!(first.sampled_cat_ids, next.sampled_cat_ids);
}

#[test]
fn filled_offices_survive_restart_and_only_vacancy_can_trigger_succession() {
    let mut state = OfficerInstitutionState::new("colony-1").unwrap();
    let vacancy = state.open_office(OfficerRole::Accountant, 100).unwrap();
    assert_eq!(vacancy.occurrence(), 0);
    let transition = state
        .appoint_officer(OfficerRole::Accountant, cat("first"), 110)
        .unwrap();
    assert_eq!(
        transition.adopt_requests(&mut OfficerRequestBook::new()),
        Vec::new()
    );
    assert_eq!(
        state.appoint_officer(OfficerRole::Accountant, cat("replacement"), 120),
        Err(InstitutionError::OfficeFilled)
    );

    let encoded = serde_json::to_string(&state).unwrap();
    assert!(encoded.contains("appointmentId"));
    assert!(encoded.contains("vacancyOccurrence"));
    let restarted: OfficerInstitutionState = serde_json::from_str(&encoded).unwrap();
    assert_eq!(restarted, state);
    assert_eq!(
        restarted.officer(OfficerRole::Accountant),
        Some(&cat("first"))
    );

    let second_vacancy = state.officer_died(&cat("first"), 200).unwrap().unwrap();
    assert_eq!(second_vacancy.occurrence(), 1);
    state
        .appoint_officer(OfficerRole::Accountant, cat("successor"), 210)
        .unwrap();
    assert_eq!(
        state.officer(OfficerRole::Accountant),
        Some(&cat("successor"))
    );
}

#[test]
fn duty_state_is_per_cat_per_role_persistent_and_canonical() {
    let mut forward = OfficerInstitutionState::new("colony-1").unwrap();
    let mut reverse = OfficerInstitutionState::new("colony-1").unwrap();
    let entries = [
        (cat("b"), OfficerRole::Captain, 24 * 60),
        (cat("a"), OfficerRole::Farmer, 96 * 60),
    ];
    for (cat_id, role, minutes) in entries.clone() {
        forward
            .record_completed_duty_minutes(cat_id, role, minutes)
            .unwrap();
    }
    for (cat_id, role, minutes) in entries.into_iter().rev() {
        reverse
            .record_completed_duty_minutes(cat_id, role, minutes)
            .unwrap();
    }
    assert_eq!(
        serde_json::to_string(&forward).unwrap(),
        serde_json::to_string(&reverse).unwrap()
    );
    assert_eq!(
        forward.personal_level(&cat("b"), OfficerRole::Captain),
        ExpertiseLevel::Two
    );
    assert_eq!(
        forward.personal_level(&cat("b"), OfficerRole::Farmer),
        ExpertiseLevel::One
    );
}

#[test]
fn leader_death_opens_exact_six_hour_succession_and_steward_acts_until_filled() {
    let mut state = OfficerInstitutionState::new("colony-1").unwrap();
    state.open_office(OfficerRole::Steward, 0).unwrap();
    state
        .appoint_officer(OfficerRole::Steward, cat("steward"), 1)
        .unwrap();
    state.set_founding_leader(cat("leader"), 1).unwrap();
    let succession = state.leader_died(&cat("leader"), 1_000, 10).unwrap();
    assert_eq!(succession.opened_tick, 1_000);
    assert_eq!(succession.deadline_tick, 1_060);
    let restarted: OfficerInstitutionState =
        serde_json::from_value(serde_json::to_value(&state).unwrap()).unwrap();
    assert_eq!(restarted, state);
    assert_eq!(state.acting_steward(), Some(&cat("steward")));
    assert!(!state.leader_succession_due(1_059));
    assert!(state.leader_succession_due(1_060));

    let transition = state.appoint_leader(cat("steward"), 1_050).unwrap();
    assert_eq!(
        transition,
        LeaderTransition {
            successor_id: cat("steward"),
            vacated_office: Some(OfficerRole::Steward)
        }
    );
    assert_eq!(state.leader(), Some(&cat("steward")));
    assert_eq!(state.acting_steward(), None);
    assert_eq!(state.officer(OfficerRole::Steward), None);
}

#[test]
fn steward_appointed_during_succession_becomes_acting_and_persists() {
    let mut state = OfficerInstitutionState::new("colony-1").unwrap();
    state.set_founding_leader(cat("leader"), 1).unwrap();
    state.leader_died(&cat("leader"), 1_000, 10).unwrap();
    assert_eq!(state.acting_steward(), None);

    state.open_office(OfficerRole::Steward, 1_001).unwrap();
    state
        .appoint_officer(OfficerRole::Steward, cat("late-steward"), 1_002)
        .unwrap();

    assert_eq!(state.acting_steward(), Some(&cat("late-steward")));
    let restarted: OfficerInstitutionState =
        serde_json::from_value(serde_json::to_value(&state).unwrap()).unwrap();
    assert_eq!(restarted, state);
}

#[test]
fn persistence_rejects_versions_missing_roles_duplicates_and_bounds() {
    let state = OfficerInstitutionState::new("colony-1").unwrap();
    let mut value = serde_json::to_value(&state).unwrap();
    value["schemaVersion"] = serde_json::json!(99);
    assert!(serde_json::from_value::<OfficerInstitutionState>(value).is_err());

    let mut unknown = serde_json::to_value(&state).unwrap();
    unknown["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<OfficerInstitutionState>(unknown).is_err());

    let mut missing_role = serde_json::to_value(&state).unwrap();
    missing_role["offices"].as_array_mut().unwrap().pop();
    assert!(serde_json::from_value::<OfficerInstitutionState>(missing_role).is_err());

    let mut duplicate_duty = serde_json::to_value(&state).unwrap();
    let duty = serde_json::json!({"catId": cat("a"), "role":"farmer", "completedDutyMinutes": 1});
    duplicate_duty["duty"] = serde_json::json!([duty.clone(), duty]);
    assert!(serde_json::from_value::<OfficerInstitutionState>(duplicate_duty).is_err());

    let too_many = candidates(MAX_APPOINTMENT_CANDIDATES + 1);
    assert_eq!(
        select_appointment_candidate(
            1,
            "colony",
            OfficerRole::Farmer,
            0,
            ExpertiseLevel::Five,
            too_many
        ),
        Err(InstitutionError::CandidateCapacityExceeded)
    );
    assert!(ExpertiseLevel::try_from(0).is_err());
    assert!(ExpertiseLevel::try_from(6).is_err());
}
