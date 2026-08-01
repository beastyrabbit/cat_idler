use cat_sim::{
    authority::AuthorityActor,
    diplomacy::{
        AuthorizedDiplomacyAction, DiplomacyAction, DiplomacyActionId, DiplomacyActionKind,
        DiplomacyAuthorization, DiplomacyColonyId, DiplomacyError, DiplomacyLedger,
        DiplomacyOutcome, DiplomacyPair, DiplomacyRelationship, ProposedRelationship,
    },
    planner_core::PlannerId,
};

fn colony(name: &str) -> DiplomacyColonyId {
    DiplomacyColonyId::derive(name)
}

#[test]
fn diplomacy_external_ids_and_update_ticks_survive_round_trip() {
    let first = colony("external-a");
    let second = colony("external-b");
    assert_eq!(first.external_id(), "external-a");
    let pair = DiplomacyPair::new(first.clone(), second).unwrap();
    let mut ledger = DiplomacyLedger::new();
    let player_id = player("timestamp");
    let receipt = ledger
        .apply_at(
            action(
                &pair,
                &first,
                "timestamp",
                0,
                DiplomacyActionKind::Propose(ProposedRelationship::Friendly),
            ),
            authorization(&first, &player_id),
            42,
        )
        .unwrap();
    assert_eq!(receipt.relationship_version, 1);
    let encoded = serde_json::to_string(&ledger).unwrap();
    let restored: DiplomacyLedger = serde_json::from_str(&encoded).unwrap();
    let record = restored.records().next().unwrap();
    assert_eq!(record.updated_at_tick, 42);
    assert_eq!(record.pair.first().external_id(), "external-a");
}

fn player(name: &str) -> PlannerId {
    PlannerId::derive("player", [name])
}

fn authorization(colony_id: &DiplomacyColonyId, player_id: &PlannerId) -> DiplomacyAuthorization {
    DiplomacyAuthorization {
        actor: AuthorityActor::God {
            player_id: player_id.clone(),
        },
        acting_colony_id: colony_id.clone(),
        owner_player_id: player_id.clone(),
        player_authorized: true,
    }
}

fn action(
    pair: &DiplomacyPair,
    acting: &DiplomacyColonyId,
    occurrence: &str,
    expected_version: u64,
    kind: DiplomacyActionKind,
) -> DiplomacyAction {
    DiplomacyAction {
        id: DiplomacyActionId::derive(pair.id(), acting, occurrence),
        pair: pair.clone(),
        acting_colony_id: acting.clone(),
        expected_version,
        kind,
    }
}

#[test]
fn pair_identity_is_unordered_stable_and_rejects_same_colony() {
    let a = colony("a");
    let b = colony("b");
    let forward = DiplomacyPair::new(a.clone(), b.clone()).unwrap();
    let reverse = DiplomacyPair::new(b, a.clone()).unwrap();
    assert_eq!(forward, reverse);
    assert!(matches!(
        DiplomacyPair::new(a.clone(), a),
        Err(DiplomacyError::SameColony)
    ));
}

#[test]
fn friendly_requires_proposal_and_both_explicit_approvals() {
    let a = colony("a");
    let b = colony("b");
    let a_player = player("a");
    let b_player = player("b");
    let pair = DiplomacyPair::new(a.clone(), b.clone()).unwrap();
    let mut ledger = DiplomacyLedger::new();

    let proposed = ledger
        .apply(
            action(
                &pair,
                &a,
                "proposal",
                0,
                DiplomacyActionKind::Propose(ProposedRelationship::Friendly),
            ),
            authorization(&a, &a_player),
        )
        .unwrap();
    assert_eq!(proposed.outcome, DiplomacyOutcome::Proposed);
    assert_eq!(proposed.relationship_version, 1);
    let record = ledger.record(pair.id()).unwrap();
    assert_eq!(record.relationship, DiplomacyRelationship::Neutral);
    assert!(
        record
            .pending_consent
            .as_ref()
            .unwrap()
            .approvals
            .is_empty()
    );

    let first_approval = action(&pair, &a, "approve-a", 1, DiplomacyActionKind::Approve);
    let first_receipt = ledger
        .apply(first_approval.clone(), authorization(&a, &a_player))
        .unwrap();
    assert_eq!(first_receipt.outcome, DiplomacyOutcome::ApprovalRecorded);
    assert_eq!(ledger.record(pair.id()).unwrap().version, 2);
    assert_eq!(
        ledger
            .apply(first_approval, authorization(&a, &a_player))
            .unwrap(),
        first_receipt
    );
    assert_eq!(ledger.record(pair.id()).unwrap().version, 2);
    assert_eq!(
        ledger.record(pair.id()).unwrap().relationship,
        DiplomacyRelationship::Neutral
    );

    let redundant_action = action(
        &pair,
        &a,
        "approve-a-again",
        2,
        DiplomacyActionKind::Approve,
    );
    let redundant_approval = ledger
        .apply(redundant_action.clone(), authorization(&a, &a_player))
        .unwrap();
    assert_eq!(redundant_approval.outcome, DiplomacyOutcome::NoChange);
    assert_eq!(ledger.record(pair.id()).unwrap().version, 2);

    let activated = ledger
        .apply(
            action(&pair, &b, "approve-b", 2, DiplomacyActionKind::Approve),
            authorization(&b, &b_player),
        )
        .unwrap();
    assert_eq!(activated.outcome, DiplomacyOutcome::RelationshipActivated);
    let record = ledger.record(pair.id()).unwrap();
    assert_eq!(record.version, 3);
    assert_eq!(record.relationship, DiplomacyRelationship::Friendly);
    assert!(record.pending_consent.is_none());
    assert_eq!(
        ledger
            .apply(redundant_action, authorization(&a, &a_player))
            .unwrap(),
        redundant_approval
    );
}

#[test]
fn allied_also_requires_both_parties() {
    let a = colony("a");
    let b = colony("b");
    let pair = DiplomacyPair::new(a.clone(), b.clone()).unwrap();
    let a_player = player("a");
    let b_player = player("b");
    let mut ledger = DiplomacyLedger::new();
    ledger
        .apply(
            action(
                &pair,
                &b,
                "proposal",
                0,
                DiplomacyActionKind::Propose(ProposedRelationship::Allied),
            ),
            authorization(&b, &b_player),
        )
        .unwrap();
    ledger
        .apply(
            action(&pair, &b, "approve-b", 1, DiplomacyActionKind::Approve),
            authorization(&b, &b_player),
        )
        .unwrap();
    assert_eq!(
        ledger.record(pair.id()).unwrap().relationship,
        DiplomacyRelationship::Neutral
    );
    ledger
        .apply(
            action(&pair, &a, "approve-a", 2, DiplomacyActionKind::Approve),
            authorization(&a, &a_player),
        )
        .unwrap();
    assert_eq!(
        ledger.record(pair.id()).unwrap().relationship,
        DiplomacyRelationship::Allied
    );
}

#[test]
fn authorization_denials_and_forged_claims_never_mutate() {
    let a = colony("a");
    let b = colony("b");
    let outsider = colony("outsider");
    let pair = DiplomacyPair::new(a.clone(), b).unwrap();
    let owner = player("owner");
    let attacker = player("attacker");
    let proposal = action(
        &pair,
        &a,
        "proposal",
        0,
        DiplomacyActionKind::Propose(ProposedRelationship::Friendly),
    );
    let mut ledger = DiplomacyLedger::new();

    let forged = DiplomacyAuthorization {
        actor: AuthorityActor::God {
            player_id: attacker,
        },
        acting_colony_id: a.clone(),
        owner_player_id: owner.clone(),
        player_authorized: true,
    };
    assert!(matches!(
        ledger.apply(proposal.clone(), forged),
        Err(DiplomacyError::PlayerIdentityMismatch)
    ));
    let wrong_colony = DiplomacyAuthorization {
        acting_colony_id: outsider.clone(),
        ..authorization(&a, &owner)
    };
    assert!(matches!(
        ledger.apply(proposal.clone(), wrong_colony),
        Err(DiplomacyError::AuthorizationColonyMismatch)
    ));
    let outsider_action = DiplomacyAction {
        acting_colony_id: outsider.clone(),
        ..proposal.clone()
    };
    assert!(matches!(
        ledger.apply(outsider_action, authorization(&outsider, &owner)),
        Err(DiplomacyError::ActingColonyNotParty)
    ));
    let leader = DiplomacyAuthorization {
        actor: AuthorityActor::Leader {
            cat_id: PlannerId::derive("cat", ["leader"]),
        },
        acting_colony_id: a.clone(),
        owner_player_id: owner,
        player_authorized: true,
    };
    assert!(matches!(
        ledger.apply(proposal, leader),
        Err(DiplomacyError::AuthorityDenied(_))
    ));
    assert!(ledger.record(pair.id()).is_none());
}

#[test]
fn immediate_block_wins_race_clears_consent_and_unblocks_to_neutral_by_blocker_policy() {
    let a = colony("a");
    let b = colony("b");
    let pair = DiplomacyPair::new(a.clone(), b.clone()).unwrap();
    let a_player = player("a");
    let b_player = player("b");
    let mut ledger = DiplomacyLedger::new();
    ledger
        .apply(
            action(
                &pair,
                &a,
                "proposal",
                0,
                DiplomacyActionKind::Propose(ProposedRelationship::Allied),
            ),
            authorization(&a, &a_player),
        )
        .unwrap();
    ledger
        .apply(
            action(&pair, &a, "approve", 1, DiplomacyActionKind::Approve),
            authorization(&a, &a_player),
        )
        .unwrap();

    let blocked = ledger
        .apply(
            action(&pair, &b, "block-b", 1, DiplomacyActionKind::Block),
            authorization(&b, &b_player),
        )
        .unwrap();
    assert_eq!(blocked.outcome, DiplomacyOutcome::Blocked);
    let record = ledger.record(pair.id()).unwrap();
    assert_eq!(record.version, 3);
    assert_eq!(record.relationship, DiplomacyRelationship::Blocked);
    assert!(record.pending_consent.is_none());
    assert!(record.blocked_by.contains(&b));

    assert!(matches!(
        ledger.apply(
            action(&pair, &a, "late-approve", 3, DiplomacyActionKind::Approve),
            authorization(&a, &a_player),
        ),
        Err(DiplomacyError::RelationshipBlocked)
    ));
    assert!(matches!(
        ledger.apply(
            action(&pair, &a, "wrong-unblock", 3, DiplomacyActionKind::Unblock),
            authorization(&a, &a_player),
        ),
        Err(DiplomacyError::NotBlocker)
    ));

    ledger
        .apply(
            action(&pair, &a, "block-a", 0, DiplomacyActionKind::Block),
            authorization(&a, &a_player),
        )
        .unwrap();
    ledger
        .apply(
            action(&pair, &b, "unblock-b", 4, DiplomacyActionKind::Unblock),
            authorization(&b, &b_player),
        )
        .unwrap();
    assert_eq!(
        ledger.record(pair.id()).unwrap().relationship,
        DiplomacyRelationship::Blocked
    );
    ledger
        .apply(
            action(&pair, &a, "unblock-a", 5, DiplomacyActionKind::Unblock),
            authorization(&a, &a_player),
        )
        .unwrap();
    let record = ledger.record(pair.id()).unwrap();
    assert_eq!(record.relationship, DiplomacyRelationship::Neutral);
    assert!(record.blocked_by.is_empty());
}

#[test]
fn concurrent_consent_requires_refresh_and_never_loses_an_approval() {
    let a = colony("a");
    let b = colony("b");
    let pair = DiplomacyPair::new(a.clone(), b.clone()).unwrap();
    let a_player = player("a");
    let b_player = player("b");
    let mut ledger = DiplomacyLedger::new();
    ledger
        .apply(
            action(
                &pair,
                &a,
                "proposal",
                0,
                DiplomacyActionKind::Propose(ProposedRelationship::Friendly),
            ),
            authorization(&a, &a_player),
        )
        .unwrap();
    ledger
        .apply(
            action(&pair, &a, "approve-a", 1, DiplomacyActionKind::Approve),
            authorization(&a, &a_player),
        )
        .unwrap();
    assert!(matches!(
        ledger.apply(
            action(
                &pair,
                &b,
                "approve-b-stale",
                1,
                DiplomacyActionKind::Approve
            ),
            authorization(&b, &b_player),
        ),
        Err(DiplomacyError::StaleVersion {
            expected: 1,
            actual: 2
        })
    ));
    ledger
        .apply(
            action(&pair, &b, "approve-b", 2, DiplomacyActionKind::Approve),
            authorization(&b, &b_player),
        )
        .unwrap();
    assert_eq!(
        ledger.record(pair.id()).unwrap().relationship,
        DiplomacyRelationship::Friendly
    );
}

#[test]
fn multi_colony_pairs_are_isolated() {
    let a = colony("a");
    let b = colony("b");
    let c = colony("c");
    let ab = DiplomacyPair::new(a.clone(), b).unwrap();
    let ac = DiplomacyPair::new(a.clone(), c).unwrap();
    let owner = player("a");
    let mut ledger = DiplomacyLedger::new();
    ledger
        .apply(
            action(&ab, &a, "block", 0, DiplomacyActionKind::Block),
            authorization(&a, &owner),
        )
        .unwrap();
    assert_eq!(ledger.relationship(&ab), DiplomacyRelationship::Blocked);
    assert_eq!(ledger.relationship(&ac), DiplomacyRelationship::Neutral);
    assert!(ledger.record(ac.id()).is_none());
}

#[test]
fn batch_actions_use_stable_action_id_order_not_input_order() {
    let a = colony("a");
    let b = colony("b");
    let pair = DiplomacyPair::new(a.clone(), b.clone()).unwrap();
    let a_player = player("a");
    let b_player = player("b");
    let commands = vec![
        AuthorizedDiplomacyAction {
            action: action(
                &pair,
                &a,
                "01-propose",
                0,
                DiplomacyActionKind::Propose(ProposedRelationship::Friendly),
            ),
            authorization: authorization(&a, &a_player),
        },
        AuthorizedDiplomacyAction {
            action: action(&pair, &a, "02-approve-a", 1, DiplomacyActionKind::Approve),
            authorization: authorization(&a, &a_player),
        },
        AuthorizedDiplomacyAction {
            action: action(&pair, &b, "03-approve-b", 2, DiplomacyActionKind::Approve),
            authorization: authorization(&b, &b_player),
        },
    ];
    let mut forward = DiplomacyLedger::new();
    let mut reverse = DiplomacyLedger::new();
    let forward_results = forward.apply_batch(commands.clone());
    let reverse_results = reverse.apply_batch(commands.into_iter().rev().collect());
    assert_eq!(forward_results, reverse_results);
    assert!(forward_results.iter().all(|result| result.result.is_ok()));
    assert_eq!(forward.relationship(&pair), DiplomacyRelationship::Friendly);
    assert_eq!(
        serde_json::to_string(&forward).unwrap(),
        serde_json::to_string(&reverse).unwrap()
    );
}

#[test]
fn pending_consent_survives_restart_exactly() {
    let a = colony("a");
    let b = colony("b");
    let pair = DiplomacyPair::new(a.clone(), b.clone()).unwrap();
    let a_player = player("a");
    let b_player = player("b");
    let mut ledger = DiplomacyLedger::new();
    ledger
        .apply(
            action(
                &pair,
                &a,
                "proposal",
                0,
                DiplomacyActionKind::Propose(ProposedRelationship::Allied),
            ),
            authorization(&a, &a_player),
        )
        .unwrap();
    ledger
        .apply(
            action(&pair, &a, "approve-a", 1, DiplomacyActionKind::Approve),
            authorization(&a, &a_player),
        )
        .unwrap();
    let json = serde_json::to_string(&ledger).unwrap();
    let mut restored: DiplomacyLedger = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, ledger);
    restored
        .apply(
            action(&pair, &b, "approve-b", 2, DiplomacyActionKind::Approve),
            authorization(&b, &b_player),
        )
        .unwrap();
    assert_eq!(
        restored.record(pair.id()).unwrap().relationship,
        DiplomacyRelationship::Allied
    );
}

#[test]
fn strict_persistence_rejects_unknown_reversed_mismatched_duplicate_and_invalid_state() {
    let a = colony("a");
    let b = colony("b");
    let pair = DiplomacyPair::new(a.clone(), b).unwrap();
    let owner = player("a");
    let mut ledger = DiplomacyLedger::new();
    ledger
        .apply(
            action(&pair, &a, "block", 0, DiplomacyActionKind::Block),
            authorization(&a, &owner),
        )
        .unwrap();
    let valid = serde_json::to_value(&ledger).unwrap();

    let mut unknown = valid.clone();
    unknown["schemaVersion"] = 99.into();
    assert!(serde_json::from_value::<DiplomacyLedger>(unknown).is_err());

    let mut reversed = valid.clone();
    let first = reversed["relationships"][0]["pair"]["first"].clone();
    let second = reversed["relationships"][0]["pair"]["second"].clone();
    reversed["relationships"][0]["pair"]["first"] = second;
    reversed["relationships"][0]["pair"]["second"] = first;
    assert!(serde_json::from_value::<DiplomacyLedger>(reversed).is_err());

    let mut mismatched = valid.clone();
    mismatched["relationships"][0]["pair"]["id"] = "forged-pair".into();
    assert!(serde_json::from_value::<DiplomacyLedger>(mismatched).is_err());

    let mut duplicate = valid.clone();
    let record = duplicate["relationships"][0].clone();
    duplicate["relationships"]
        .as_array_mut()
        .unwrap()
        .push(record);
    assert!(serde_json::from_value::<DiplomacyLedger>(duplicate).is_err());

    let mut invalid_blocked = valid;
    invalid_blocked["relationships"][0]["blockedBy"] = serde_json::json!([]);
    assert!(serde_json::from_value::<DiplomacyLedger>(invalid_blocked).is_err());

    let mut blocked_with_pending = serde_json::to_value(&ledger).unwrap();
    blocked_with_pending["relationships"][0]["pendingConsent"] = serde_json::json!({
        "proposalActionId": DiplomacyActionId::derive(pair.id(), &a, "forged-pending"),
        "target": "friendly",
        "proposedBy": a,
        "approvals": [],
    });
    assert!(serde_json::from_value::<DiplomacyLedger>(blocked_with_pending).is_err());
}

#[test]
fn action_id_collision_is_rejected_without_mutation() {
    let a = colony("a");
    let b = colony("b");
    let pair = DiplomacyPair::new(a.clone(), b).unwrap();
    let owner = player("a");
    let mut ledger = DiplomacyLedger::new();
    let proposal = action(
        &pair,
        &a,
        "same-id",
        0,
        DiplomacyActionKind::Propose(ProposedRelationship::Friendly),
    );
    ledger
        .apply(proposal.clone(), authorization(&a, &owner))
        .unwrap();
    let version = ledger.record(pair.id()).unwrap().version;
    let collision = DiplomacyAction {
        kind: DiplomacyActionKind::Block,
        ..proposal
    };
    assert!(matches!(
        ledger.apply(collision, authorization(&a, &owner)),
        Err(DiplomacyError::ActionIdCollision)
    ));
    assert_eq!(ledger.record(pair.id()).unwrap().version, version);
}

#[test]
fn persisted_public_state_contains_only_relationship_and_consent_facts() {
    let a = colony("a");
    let b = colony("b");
    let pair = DiplomacyPair::new(a.clone(), b).unwrap();
    let owner = player("a");
    let mut ledger = DiplomacyLedger::new();
    ledger
        .apply(
            action(
                &pair,
                &a,
                "proposal",
                0,
                DiplomacyActionKind::Propose(ProposedRelationship::Friendly),
            ),
            authorization(&a, &owner),
        )
        .unwrap();
    assert_eq!(ledger.records().len(), 1);
    let json = serde_json::to_string(&ledger).unwrap().to_ascii_lowercase();
    for forbidden_key in [
        "\"beliefs\":",
        "\"inventory\":",
        "\"plans\":",
        "\"hidden",
        "\"stockpiles\":",
    ] {
        assert!(!json.contains(forbidden_key));
    }
}
