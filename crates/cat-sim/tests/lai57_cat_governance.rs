//! Focused LAI.57 contract tests. Kept unexecuted until the serialized feature
//! verification lane.

use cat_sim::cat_governance::{
    BackingEligibility, BallotSignals, CAT_GOVERNANCE_SCHEMA_VERSION, CatElectionState, CatVoter,
    CivicCandidate, CivicMeritMetrics, ElectionTrigger, ExpulsionPlan, ExpulsionRequest,
    ExpulsionResident, ExpulsionScope, ExpulsionStage, GovernanceLifeStage, PLAYER_BACKING_VOTES,
    RelationalAnalyticalAxis, VoterCandidateView, cast_cat_ballot, select_civic_slate,
};

fn merit(governance: u16, leadership: u16) -> CivicMeritMetrics {
    CivicMeritMetrics {
        governance,
        inherited_leadership: leadership,
        effective_charisma: 5_000,
        intelligence: 5_000,
        office_breadth: 5_000,
        leadership_service_record: 5_000,
        relevant_traits: 5_000,
    }
}

#[test]
fn lai57_slate_uses_exact_merit_and_top_five_stable_ties() {
    let candidates = (0..7)
        .map(|index| CivicCandidate {
            cat_id: format!("cat-{index}"),
            life_stage: if index == 6 {
                GovernanceLifeStage::Young
            } else {
                GovernanceLifeStage::Adult
            },
            resident: true,
            barred: false,
            merit: merit((index * 1_000) as u16, ((6 - index) * 1_000) as u16),
        })
        .collect::<Vec<_>>();
    let slate = select_civic_slate(&candidates).expect("valid candidates");
    assert_eq!(slate.len(), 5);
    assert!(!slate.iter().any(|entry| entry.cat_id == "cat-6"));
    assert!(slate.windows(2).all(|pair| {
        pair[0].civic_merit > pair[1].civic_merit
            || (pair[0].civic_merit == pair[1].civic_merit
                && (pair[0].governance > pair[1].governance
                    || (pair[0].governance == pair[1].governance
                        && pair[0].cat_id < pair[1].cat_id)))
    }));
}

#[test]
fn lai57_every_adult_votes_with_fixed_point_axis_and_keyed_variation() {
    let views = vec![
        VoterCandidateView {
            candidate_id: "social".to_owned(),
            signals: BallotSignals {
                charisma: 10_000,
                care: 10_000,
                trust: 10_000,
                social_conduct: 10_000,
                personality_compatibility: 10_000,
                governance: 0,
                intelligence: 0,
                office_experience: 0,
                skill: 0,
                results: 0,
            },
            civic_merit: 5_000,
            governance: 0,
        },
        VoterCandidateView {
            candidate_id: "analyst".to_owned(),
            signals: BallotSignals {
                charisma: 0,
                care: 0,
                trust: 0,
                social_conduct: 0,
                personality_compatibility: 0,
                governance: 10_000,
                intelligence: 10_000,
                office_experience: 10_000,
                skill: 10_000,
                results: 10_000,
            },
            civic_merit: 5_000,
            governance: 10_000,
        },
    ];
    let relational = cast_cat_ballot(
        "election-1",
        &CatVoter {
            cat_id: "voter-r".to_owned(),
            life_stage: GovernanceLifeStage::Adult,
            resident: true,
            axis: RelationalAnalyticalAxis::new(-10_000).unwrap(),
        },
        &views,
    )
    .unwrap()
    .unwrap();
    assert_eq!(relational.candidate_cat_id, "social");
    let analytical = cast_cat_ballot(
        "election-1",
        &CatVoter {
            cat_id: "voter-a".to_owned(),
            life_stage: GovernanceLifeStage::Elder,
            resident: true,
            axis: RelationalAnalyticalAxis::new(10_000).unwrap(),
        },
        &views,
    )
    .unwrap()
    .unwrap();
    assert_eq!(analytical.candidate_cat_id, "analyst");
}

#[test]
fn lai57_player_backing_is_authenticated_exactly_ten_and_latest_replaces() {
    let slate = select_civic_slate(&[
        CivicCandidate {
            cat_id: "a".to_owned(),
            life_stage: GovernanceLifeStage::Adult,
            resident: true,
            barred: false,
            merit: merit(9_000, 9_000),
        },
        CivicCandidate {
            cat_id: "b".to_owned(),
            life_stage: GovernanceLifeStage::Adult,
            resident: true,
            barred: false,
            merit: merit(1_000, 1_000),
        },
    ])
    .unwrap();
    let mut election = CatElectionState::new(
        "election-1",
        "colony-1",
        ElectionTrigger::Scheduled,
        0,
        slate,
    )
    .unwrap();
    assert!(
        election
            .set_player_backing(
                "god-a",
                "a",
                BackingEligibility {
                    authenticated: false,
                    eligible_global_player: true,
                    personal_village_owner: false,
                },
                1,
            )
            .is_err()
    );
    let authorized = BackingEligibility {
        authenticated: true,
        eligible_global_player: true,
        personal_village_owner: false,
    };
    election
        .set_player_backing("god-a", "a", authorized, 1)
        .unwrap();
    election
        .set_player_backing("god-a", "b", authorized, 2)
        .unwrap();
    let result = election.resolve().unwrap();
    assert_eq!(result.total_votes["a"], 0);
    assert_eq!(result.total_votes["b"], PLAYER_BACKING_VOTES);
    assert_eq!(result.winner_cat_id.as_deref(), Some("b"));
    assert_eq!(election.schema_version, CAT_GOVERNANCE_SCHEMA_VERSION);
}

fn resident(
    id: &str,
    household: &str,
    life_stage: GovernanceLifeStage,
    guardian: Option<&str>,
    leader: bool,
) -> ExpulsionResident {
    ExpulsionResident {
        cat_id: id.to_owned(),
        household_id: household.to_owned(),
        life_stage,
        guardian_id: guardian.map(str::to_owned),
        is_leader: leader,
        job_id: Some(format!("job-{id}")),
        office_id: leader.then(|| "leader".to_owned()),
        residence_id: Some("home-1".to_owned()),
        enterprise_id: Some("mill-1".to_owned()),
        carried_cargo_ids: vec![format!("cargo-{id}")],
        reservation_ids: vec![format!("reservation-{id}")],
        owned_item_ids: vec![format!("owned-{id}")],
        equipped_item_ids: vec![format!("equipped-{id}")],
    }
}

#[test]
fn lai57_household_expulsion_keeps_dependents_with_guardian_and_cleans_before_departure() {
    let residents = vec![
        resident(
            "parent",
            "household-1",
            GovernanceLifeStage::Adult,
            None,
            true,
        ),
        resident(
            "kitten",
            "household-1",
            GovernanceLifeStage::Kitten,
            Some("parent"),
            false,
        ),
    ];
    let mut plan = ExpulsionPlan::build(
        ExpulsionRequest {
            expulsion_id: "expel-1".to_owned(),
            colony_id: "colony-1".to_owned(),
            target_cat_id: "parent".to_owned(),
            scope: ExpulsionScope::WholeHousehold,
        },
        &residents,
    )
    .expect("guardian travels with dependent");
    assert!(plan.opens_snap_election);
    assert_eq!(plan.members.len(), 2);
    assert!(!plan.may_leave_colony());
    for stage in [
        ExpulsionStage::CargoReturned,
        ExpulsionStage::ReservationsReleased,
        ExpulsionStage::RolesCleared,
        ExpulsionStage::ResidenceVacated,
        ExpulsionStage::ItemsResolved,
        ExpulsionStage::PhysicalDeparture,
        ExpulsionStage::Completed,
    ] {
        plan.advance(stage).expect("ordered cleanup transition");
    }
    assert!(plan.may_leave_colony());
}
