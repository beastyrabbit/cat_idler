use cat_sim::{
    officers::OfficerRole,
    planner_core::PlannerId,
    player_directives::{
        AuthorityOverrideKey, PlayerDirectiveError, PlayerDirectiveId, PlayerDirectiveState,
        StandingOrder, StandingOrderPatch, TreatmentRequest,
    },
};

fn colony_id() -> PlannerId {
    PlannerId::derive("colony", ["directive-test"])
}

fn order(action: &str) -> StandingOrder {
    StandingOrder {
        id: PlayerDirectiveId::derive("standing_order", &colony_id(), action),
        order_kind: "prioritize".to_owned(),
        domain: "forestry".to_owned(),
        target_id: Some("site-a".to_owned()),
        instruction: "Keep lumber reserves above target.".to_owned(),
        priority_basis_points: 7_500,
        expires_tick: Some(200),
        created_tick: 100,
    }
}

#[test]
fn standing_orders_are_bounded_versioned_and_patch_atomically() {
    let mut state = PlayerDirectiveState::new();
    state.create_standing_order(order("one"), 1).unwrap();
    assert_eq!(state.version, 1);
    assert_eq!(
        state.create_standing_order(order("two"), 1),
        Err(PlayerDirectiveError::CapacityReached)
    );
    let before = state.clone();
    assert_eq!(
        state.update_standing_order(
            order("one").id.as_str(),
            StandingOrderPatch {
                instruction: None,
                priority_basis_points: None,
                target_id: None,
                clear_target: false,
                expires_tick: Some(50),
                clear_expiry: false,
            },
        ),
        Err(PlayerDirectiveError::InvalidDirective)
    );
    assert_eq!(state, before);

    state
        .update_standing_order(
            order("one").id.as_str(),
            StandingOrderPatch {
                instruction: Some("Prefer mature trees.".to_owned()),
                priority_basis_points: Some(9_000),
                target_id: None,
                clear_target: true,
                expires_tick: None,
                clear_expiry: true,
            },
        )
        .unwrap();
    assert_eq!(state.version, 2);
    state
        .delete_standing_order(order("one").id.as_str())
        .unwrap();
    assert_eq!(state.version, 3);
}

#[test]
fn authority_and_treatment_directives_are_durable_and_idempotent() {
    let mut state = PlayerDirectiveState::new();
    let key = AuthorityOverrideKey {
        role: OfficerRole::Forester,
        domain: "forestry".to_owned(),
        request_id: Some("request-a".to_owned()),
    };
    state.set_authority_override(key.clone(), true).unwrap();
    state.set_authority_override(key, true).unwrap();
    assert_eq!(state.version, 1);

    let request = TreatmentRequest {
        id: PlayerDirectiveId::derive("treatment", &colony_id(), "treat-one"),
        cat_id: "cat-a".to_owned(),
        injury_id: "injury-a".to_owned(),
        treatment_kind: "stabilize".to_owned(),
        requested_tick: 12,
    };
    state.request_treatment(request.clone()).unwrap();
    state.request_treatment(request).unwrap();
    assert_eq!(state.version, 2);

    let encoded = serde_json::to_string(&state).unwrap();
    let restored: PlayerDirectiveState = serde_json::from_str(&encoded).unwrap();
    assert_eq!(restored, state);
    restored.validate().unwrap();
}

#[test]
fn canonical_research_catalog_covers_the_complete_manifest() {
    let catalog = cat_sim::research_purchase::canonical_research_catalog();
    assert_eq!(catalog.studies.len(), 531);
    catalog.validate().unwrap();
    for study in cat_sim::research_manifest::research_manifest().studies() {
        assert!(
            catalog
                .study(&cat_sim::research_purchase::StudyId::derive(
                    &study.stable_id
                ))
                .is_some()
        );
    }
}
