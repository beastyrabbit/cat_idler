//! Focused lifecycle contracts for the canonical LAI.57 governance authority.
//! These are intentionally not executed by this card; root integration wires the
//! new leaf into `cat_sim` before the serialized verification lane.

use cat_sim::{
    cat_governance::{
        ElectionTrigger, ExpulsionScope, GovernanceLifeStage, RelationalAnalyticalAxis,
    },
    governance_authority::{
        BackingActor, BackingCommand, BackingEligibilityWire, BackingOutcome, CandidateBallotFacts,
        CleanupKind, GovernanceAuthorityError, GovernanceAuthorityState, GovernanceResidentFact,
        MAX_GOVERNANCE_RECEIPTS, ReportSafeAppointmentCandidate,
    },
    officer_expertise::{ExpertiseLevel, InstitutionError},
    officers::OfficerRole,
};

fn resident(
    id: &str,
    household: &str,
    governance: u16,
    relational_axis: i16,
) -> GovernanceResidentFact {
    GovernanceResidentFact {
        cat_id: id.to_owned(),
        household_id: household.to_owned(),
        life_stage: GovernanceLifeStage::Adult,
        resident: true,
        alive: true,
        barred: false,
        guardian_id: None,
        axis: RelationalAnalyticalAxis::new(relational_axis).expect("bounded axis"),
        merit: cat_sim::cat_governance::CivicMeritMetrics {
            governance,
            inherited_leadership: governance,
            effective_charisma: governance,
            intelligence: governance,
            office_breadth: governance,
            leadership_service_record: governance,
            relevant_traits: governance,
        },
        ballot_facts: CandidateBallotFacts {
            charisma: governance,
            care: governance,
            trust: governance,
            social_conduct: governance,
            personality_compatibility: governance,
            governance,
            intelligence: governance,
            office_experience: governance,
            skill: governance,
            results: governance,
        },
        job_id: Some(format!("job-{id}")),
        office_id: None,
        residence_id: Some(format!("home-{household}")),
        enterprise_id: Some(format!("enterprise-{household}")),
        partnership_id: Some(format!("partnership-{household}")),
        carried_cargo_ids: vec![format!("cargo-{id}")],
        reservation_ids: vec![format!("reservation-{id}")],
        owned_item_ids: vec![format!("owned-{id}")],
        equipped_item_ids: vec![format!("equipped-{id}")],
    }
}

fn authority_with_six_residents() -> GovernanceAuthorityState {
    let mut authority = GovernanceAuthorityState::new("colony-alpha").expect("authority");
    for (id, merit) in [
        ("alpha", 9_000),
        ("bravo", 8_000),
        ("charlie", 7_000),
        ("delta", 6_000),
        ("echo", 5_000),
        ("foxtrot", 4_000),
    ] {
        authority
            .register_resident(resident(id, "house-a", merit, 0))
            .expect("resident");
    }
    authority
}

fn encoded(authority: &GovernanceAuthorityState) -> String {
    serde_json::to_string(authority).expect("authority JSON")
}

#[test]
fn lai57_authority_persists_top_five_all_votes_and_tie_order_once() {
    let mut authority = authority_with_six_residents();
    let opened = authority
        .open_scheduled_election("season-1", 100, 10)
        .expect("opens election");
    assert!(opened.created);
    let executor = authority
        .executor_election(&opened.election_id)
        .expect("executor view");
    assert_eq!(executor.election.slate.len(), 5);
    assert_eq!(executor.eligible_voter_ids.len(), 6);
    assert_eq!(executor.election.cat_ballots.len(), 6);
    let again = authority
        .open_scheduled_election("season-1", 101, 10)
        .expect("same scheduled occurrence");
    assert_eq!(
        again,
        cat_sim::governance_authority::ElectionOpenOutcome {
            election_id: opened.election_id.clone(),
            created: false,
        }
    );
    let result = authority
        .resolve_election(&opened.election_id, 110)
        .expect("resolves once");
    assert_eq!(result.tie_order.len(), 5);
    assert_eq!(result.winner_cat_id.as_deref(), Some("alpha"));
    assert_eq!(
        authority
            .resolve_election(&opened.election_id, 111)
            .expect("replay resolution"),
        result
    );
    let report = authority.report();
    assert_eq!(report.leader_cat_id.as_deref(), Some("alpha"));
    assert!(report.elections[0].total_votes.values().sum::<u32>() >= 6);
}

#[test]
fn lai57_backing_replaces_exactly_one_block_and_is_versioned_idempotent() {
    let mut authority = authority_with_six_residents();
    let election_id = authority
        .open_scheduled_election("season-1", 100, 10)
        .expect("opens")
        .election_id;
    let command = |idempotency_id: &str, candidate_cat_id: &str, expected_version| BackingCommand {
        idempotency_id: idempotency_id.to_owned(),
        expected_version,
        election_id: election_id.clone(),
        player_id: "player-one".to_owned(),
        candidate_cat_id: candidate_cat_id.to_owned(),
        actor: BackingActor {
            eligibility: BackingEligibilityWire {
                authenticated: true,
                eligible_global_player: true,
                personal_village_owner: false,
            },
            global_village: true,
        },
        submitted_tick: 101,
    };
    let first = authority
        .submit_backing(command("backing-a", "bravo", authority.version()))
        .expect("first backing");
    let BackingOutcome::Applied(first_receipt) = first else {
        panic!("first backing must apply");
    };
    assert!(matches!(
        authority.submit_backing(command("backing-a", "bravo", 0)),
        Ok(BackingOutcome::Replayed(receipt)) if receipt == first_receipt
    ));
    authority
        .submit_backing(command("backing-b", "charlie", authority.version()))
        .expect("replacement backing");
    let election = authority
        .executor_election(&election_id)
        .expect("executor election");
    assert_eq!(election.election.player_backing.len(), 1);
    assert_eq!(
        election.election.player_backing["player-one"].candidate_cat_id,
        "charlie"
    );
    let mut denied = command("denied", "alpha", authority.version());
    denied.actor.eligibility.eligible_global_player = false;
    denied.actor.eligibility.personal_village_owner = true;
    assert!(matches!(
        authority.submit_backing(denied),
        Err(GovernanceAuthorityError::GlobalVillageBackingDenied)
    ));
}

#[test]
fn lai57_death_opens_one_snap_and_uses_existing_succession_handoff() {
    let mut authority = authority_with_six_residents();
    let scheduled = authority
        .open_scheduled_election("season-1", 100, 10)
        .expect("scheduled");
    authority
        .resolve_election(&scheduled.election_id, 101)
        .expect("leader");
    let snap = authority
        .record_death("alpha", 200, 10)
        .expect("death")
        .expect("leader death opens snap");
    assert_eq!(
        authority
            .open_snap_election(ElectionTrigger::LeaderDeath, "death:alpha", 201)
            .expect("same death occurrence"),
        cat_sim::governance_authority::ElectionOpenOutcome {
            election_id: snap.election_id.clone(),
            created: false,
        }
    );
    let replacement = authority
        .resolve_election(&snap.election_id, 202)
        .expect("successor");
    assert_ne!(replacement.winner_cat_id.as_deref(), Some("alpha"));
    assert_eq!(authority.report().leader_cat_id, replacement.winner_cat_id);
}

#[test]
fn lai57_report_safe_officer_handoff_can_make_good_or_bad_report_based_choices() {
    let mut authority = authority_with_six_residents();
    let good = authority
        .appoint_officer_from_reports(
            OfficerRole::Steward,
            99,
            ExpertiseLevel::Five,
            vec![
                ReportSafeAppointmentCandidate {
                    cat_id: "alpha".into(),
                    believed_merit: 100,
                },
                ReportSafeAppointmentCandidate {
                    cat_id: "bravo".into(),
                    believed_merit: 10,
                },
            ],
            1,
        )
        .expect("good report appointment")
        .expect("eligible appointment");
    assert_eq!(good.selected_cat_id, "alpha");
    let bad = authority
        .appoint_officer_from_reports(
            OfficerRole::Accountant,
            100,
            ExpertiseLevel::Five,
            vec![
                ReportSafeAppointmentCandidate {
                    cat_id: "bravo".into(),
                    believed_merit: 1,
                },
                ReportSafeAppointmentCandidate {
                    cat_id: "charlie".into(),
                    believed_merit: 999,
                },
            ],
            2,
        )
        .expect("bad belief appointment")
        .expect("eligible appointment");
    assert_eq!(bad.selected_cat_id, "charlie");
    assert!(
        !serde_json::to_string(&authority.report())
            .expect("report JSON")
            .contains("9000")
    );
}

#[test]
fn lai57_household_expulsion_requires_guardian_all_cleanup_and_reachable_departure() {
    let mut authority = authority_with_six_residents();
    let mut parent = resident("parent", "family", 9_500, 0);
    parent.office_id = Some("office-parent".into());
    let mut kitten = resident("kitten", "family", 1_000, 0);
    kitten.life_stage = GovernanceLifeStage::Kitten;
    kitten.guardian_id = Some("parent".into());
    authority.register_resident(parent).expect("parent");
    authority.register_resident(kitten).expect("kitten");
    let preview = authority
        .preview_expulsion("expel-family", "parent", ExpulsionScope::WholeHousehold)
        .expect("guardian included");
    assert_eq!(preview.plan.members.len(), 2);
    assert_eq!(preview.intents.len(), 20);
    let departure = preview
        .intents
        .iter()
        .find(|intent| intent.kind == CleanupKind::Departure)
        .expect("departure intent");
    assert!(matches!(
        authority.acknowledge_cleanup("expel-family", &departure.intent_id, false),
        Err(GovernanceAuthorityError::DepartureUnreachable)
    ));
    for intent in &preview.intents {
        authority
            .acknowledge_cleanup(
                "expel-family",
                &intent.intent_id,
                intent.kind == CleanupKind::Departure,
            )
            .expect("each authority acknowledgement");
    }
    authority
        .commit_expulsion("expel-family", 100, 10)
        .expect("all cleanup gates completed");
    assert!(!authority.resident("parent").expect("parent fact").resident);
    assert!(!authority.resident("kitten").expect("kitten fact").resident);
    assert!(authority.report().pending_expulsion_ids.is_empty());
}

#[test]
fn lai57_selected_adult_expulsion_leaves_other_household_members_in_place() {
    let mut authority = GovernanceAuthorityState::new("colony-alpha").expect("authority");
    authority
        .register_resident(resident("adult", "family", 5_000, 0))
        .expect("adult");
    authority
        .register_resident(resident("partner", "family", 4_000, 0))
        .expect("partner");
    let preview = authority
        .preview_expulsion("expel-adult", "adult", ExpulsionScope::SelectedAdult)
        .expect("individual adult is valid");
    assert_eq!(preview.plan.members.len(), 1);
    for intent in &preview.intents {
        authority
            .acknowledge_cleanup(
                "expel-adult",
                &intent.intent_id,
                intent.kind == CleanupKind::Departure,
            )
            .expect("acknowledge");
    }
    authority
        .commit_expulsion("expel-adult", 10, 10)
        .expect("commit");
    assert!(!authority.resident("adult").expect("adult").resident);
    assert!(authority.resident("partner").expect("partner").resident);
}

#[test]
fn lai57_restart_decode_is_strict_and_btree_order_is_canonical() {
    let authority = authority_with_six_residents();
    let encoded = serde_json::to_string(&authority).expect("encode");
    let restored: GovernanceAuthorityState =
        serde_json::from_str(&encoded).expect("strict restore");
    assert_eq!(serde_json::to_string(&restored).expect("reencode"), encoded);
    let mut malformed: serde_json::Value = serde_json::from_str(&encoded).expect("json");
    malformed["unexpected"] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<GovernanceAuthorityState>(malformed).is_err());
    let mut unsupported: serde_json::Value = serde_json::from_str(&encoded).expect("json");
    unsupported["schemaVersion"] = serde_json::Value::from(99);
    assert!(serde_json::from_value::<GovernanceAuthorityState>(unsupported).is_err());
}

#[test]
fn lai57_receipt_capacity_failure_rolls_back_backing_replacement_byte_exactly() {
    let mut authority = authority_with_six_residents();
    let election_id = authority
        .open_scheduled_election("receipt-capacity", 1, 10)
        .expect("election")
        .election_id;
    for index in 0..MAX_GOVERNANCE_RECEIPTS {
        authority
            .submit_backing(BackingCommand {
                idempotency_id: format!("backing-{index:04}"),
                expected_version: authority.version(),
                election_id: election_id.clone(),
                player_id: "capacity-player".into(),
                candidate_cat_id: "alpha".into(),
                actor: BackingActor {
                    eligibility: BackingEligibilityWire {
                        authenticated: true,
                        eligible_global_player: true,
                        personal_village_owner: false,
                    },
                    global_village: true,
                },
                submitted_tick: index as u64,
            })
            .expect("fill receipt capacity");
    }
    let before = encoded(&authority);
    assert!(matches!(
        authority.submit_backing(BackingCommand {
            idempotency_id: "backing-over-capacity".into(),
            expected_version: authority.version(),
            election_id,
            player_id: "capacity-player".into(),
            candidate_cat_id: "bravo".into(),
            actor: BackingActor {
                eligibility: BackingEligibilityWire {
                    authenticated: true,
                    eligible_global_player: true,
                    personal_village_owner: false,
                },
                global_village: true,
            },
            submitted_tick: MAX_GOVERNANCE_RECEIPTS as u64,
        }),
        Err(GovernanceAuthorityError::CapacityExceeded)
    ));
    assert_eq!(encoded(&authority), before);
}

#[test]
fn lai57_leader_death_without_successor_rolls_back_every_partial_transition() {
    let mut authority = GovernanceAuthorityState::new("colony-alpha").expect("authority");
    authority
        .register_resident(resident("only-cat", "only-house", 9_000, 0))
        .expect("resident");
    let election = authority
        .open_scheduled_election("founding", 1, 10)
        .expect("election");
    authority
        .resolve_election(&election.election_id, 2)
        .expect("founding leader");
    let before = encoded(&authority);
    assert!(matches!(
        authority.record_death("only-cat", 3, 10),
        Err(GovernanceAuthorityError::NoEligibleCandidate)
    ));
    assert_eq!(encoded(&authority), before);
    assert!(authority.resident("only-cat").expect("resident").alive);
    assert_eq!(
        authority.report().leader_cat_id.as_deref(),
        Some("only-cat")
    );
}

#[test]
fn lai57_failed_leader_expulsion_commit_rolls_back_departure_and_succession() {
    let mut authority = GovernanceAuthorityState::new("colony-alpha").expect("authority");
    authority
        .register_resident(resident("only-cat", "only-house", 9_000, 0))
        .expect("resident");
    let election = authority
        .open_scheduled_election("founding", 1, 10)
        .expect("election");
    authority
        .resolve_election(&election.election_id, 2)
        .expect("founding leader");
    let preview = authority
        .preview_expulsion(
            "expel-only-leader",
            "only-cat",
            ExpulsionScope::SelectedAdult,
        )
        .expect("preview");
    for intent in preview.intents {
        authority
            .acknowledge_cleanup(
                "expel-only-leader",
                &intent.intent_id,
                intent.kind == CleanupKind::Departure,
            )
            .expect("acknowledgement");
    }
    let before = encoded(&authority);
    assert!(matches!(
        authority.commit_expulsion("expel-only-leader", 3, 10),
        Err(GovernanceAuthorityError::NoEligibleCandidate)
    ));
    assert_eq!(encoded(&authority), before);
    assert!(authority.resident("only-cat").expect("resident").resident);
    assert_eq!(
        authority.report().leader_cat_id.as_deref(),
        Some("only-cat")
    );
}

#[test]
fn lai57_failed_officer_appointment_rolls_back_opened_vacancy() {
    let mut authority = authority_with_six_residents();
    authority
        .appoint_officer_from_reports(
            OfficerRole::Steward,
            1,
            ExpertiseLevel::Five,
            vec![ReportSafeAppointmentCandidate {
                cat_id: "alpha".into(),
                believed_merit: 100,
            }],
            1,
        )
        .expect("first appointment")
        .expect("selected");
    let before = encoded(&authority);
    assert!(matches!(
        authority.appoint_officer_from_reports(
            OfficerRole::Accountant,
            2,
            ExpertiseLevel::Five,
            vec![ReportSafeAppointmentCandidate {
                cat_id: "alpha".into(),
                believed_merit: 100,
            }],
            2,
        ),
        Err(GovernanceAuthorityError::Institution(
            InstitutionError::CandidateAlreadyAppointed
        ))
    ));
    assert_eq!(encoded(&authority), before);
}
