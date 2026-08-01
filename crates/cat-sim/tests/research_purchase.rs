use std::collections::{BTreeMap, BTreeSet};

use cat_sim::{
    favor::{Favor, FavorEventId, FavorEventKind, FavorLedger},
    leader_planner::EffectiveLevel,
    planner_core::PlannerId,
    research_purchase::{
        AutomaticResearchCapability, AutomaticResearchPurchaseRequest, AutomaticStudyScoreInputs,
        PlayerResearchPurchaseRequest, ResearchPurchaseError, ResearchPurchaseId,
        ResearchPurchaseOutcome, ResearchPurchaseSource, ResearchPurchaseState, StudyId,
        SyntheticResearchCatalog, SyntheticStudyDescriptor,
    },
};

fn colony() -> PlannerId {
    PlannerId::derive("test_colony", ["one"])
}

fn study(id: &str, price: u64, prerequisites: &[&str]) -> SyntheticStudyDescriptor {
    SyntheticStudyDescriptor {
        id: StudyId::derive(id),
        display_name: format!("Study {id}"),
        prerequisites: prerequisites.iter().map(|id| StudyId::derive(id)).collect(),
        undiscounted_price: Favor::from_whole(price).unwrap(),
        tags: BTreeSet::from([PlannerId::derive("tag", ["synthetic"])]),
    }
}

fn catalog() -> SyntheticResearchCatalog {
    SyntheticResearchCatalog::new(vec![
        study("administration_1", 4, &[]),
        study("dependency", 2, &[]),
        study("duration_1", 3, &[]),
        study("duration_2", 5, &["duration_1"]),
        study("expensive", 20, &[]),
    ])
}

fn purchase_id(action: &str) -> ResearchPurchaseId {
    ResearchPurchaseId::derive("test", &colony(), action)
}

fn funded_ledger(amount: u64) -> FavorLedger {
    let mut ledger = FavorLedger::new();
    ledger
        .credit(
            FavorEventId::derive("test_funding", colony().as_str(), "grant"),
            FavorEventKind::LegacyMigrationCredit,
            Favor::from_whole(amount).unwrap(),
            0,
            0,
        )
        .unwrap();
    ledger
}

fn player_request(
    action: &str,
    study_id: &str,
    progress: &ResearchPurchaseState,
    ledger: &FavorLedger,
    discount_basis_points: u16,
) -> PlayerResearchPurchaseRequest {
    PlayerResearchPurchaseRequest {
        id: purchase_id(action),
        colony_id: colony(),
        study_id: StudyId::derive(study_id),
        expected_research_version: progress.version,
        expected_favor_version: ledger.version,
        discount_basis_points,
        consume_preparation: discount_basis_points > 0,
        now_tick: progress.version + 10,
    }
}

fn automatic_request(
    action: &str,
    progress: &ResearchPurchaseState,
    ledger: &FavorLedger,
    level: AutomaticResearchCapability,
    scores: BTreeMap<StudyId, AutomaticStudyScoreInputs>,
    now_tick: u64,
) -> AutomaticResearchPurchaseRequest {
    AutomaticResearchPurchaseRequest {
        id: purchase_id(action),
        colony_id: colony(),
        expected_research_version: progress.version,
        expected_favor_version: ledger.version,
        effective_loremaster: level,
        scores,
        now_tick,
    }
}

#[test]
fn player_purchase_filters_frontier_debits_discounted_favor_once_and_freezes_price() {
    let base_catalog = catalog();
    let mut progress = ResearchPurchaseState::new();
    let mut ledger = funded_ledger(10);
    let not_frontier = player_request("not-frontier", "duration_2", &progress, &ledger, 0);

    assert_eq!(
        progress.player_purchase(&mut ledger, &base_catalog, not_frontier),
        Err(ResearchPurchaseError::NotFrontier)
    );
    assert_eq!(ledger.balance, Favor::from_whole(10).unwrap());

    let request = player_request("prepared-duration", "duration_1", &progress, &ledger, 2_500);
    assert_eq!(
        progress
            .player_purchase(&mut ledger, &base_catalog, request.clone())
            .unwrap(),
        ResearchPurchaseOutcome::Committed
    );
    assert_eq!(ledger.balance, Favor::from_micro_favor(7_750_000));
    let event = progress.purchases.get(&request.id).unwrap();
    assert_eq!(event.undiscounted_price, Favor::from_whole(3).unwrap());
    assert_eq!(event.charged_price, Favor::from_micro_favor(2_250_000));
    assert_eq!(event.discount_basis_points, 2_500);
    assert!(event.consumed_preparation);

    let mut changed_catalog = catalog();
    changed_catalog.studies[2].undiscounted_price = Favor::from_whole(99).unwrap();
    let replay = PlayerResearchPurchaseRequest {
        expected_research_version: 0,
        expected_favor_version: 1,
        ..request
    };
    assert_eq!(
        progress
            .player_purchase(&mut ledger, &changed_catalog, replay)
            .unwrap(),
        ResearchPurchaseOutcome::AlreadyCommitted
    );
    assert_eq!(ledger.balance, Favor::from_micro_favor(7_750_000));
    assert_eq!(progress.version, 1);
}

#[test]
fn stale_duplicate_unaffordable_and_rejected_player_operations_do_not_mutate() {
    let base_catalog = catalog();
    let mut progress = ResearchPurchaseState::new();
    let mut ledger = funded_ledger(1);
    let before_progress = progress.clone();
    let before_ledger = ledger.clone();
    let too_expensive = player_request("too-expensive", "dependency", &progress, &ledger, 0);

    assert_eq!(
        progress.player_purchase(&mut ledger, &base_catalog, too_expensive),
        Err(ResearchPurchaseError::Favor(
            cat_sim::favor::FavorError::InsufficientFavor
        ))
    );
    assert_eq!(progress, before_progress);
    assert_eq!(ledger, before_ledger);

    let mut funded = funded_ledger(10);
    let mut state = ResearchPurchaseState::new();
    let first = player_request("same-action", "dependency", &state, &funded, 0);
    assert_eq!(
        state
            .player_purchase(&mut funded, &base_catalog, first.clone())
            .unwrap(),
        ResearchPurchaseOutcome::Committed
    );
    let conflict = PlayerResearchPurchaseRequest {
        study_id: StudyId::derive("administration_1"),
        expected_research_version: 1,
        expected_favor_version: funded.version,
        ..first
    };
    assert_eq!(
        state.player_purchase(&mut funded, &base_catalog, conflict),
        Err(ResearchPurchaseError::PurchaseIdConflict)
    );
    assert_eq!(state.version, 1);
    assert_eq!(funded.event_count(), 2);

    let stale = PlayerResearchPurchaseRequest {
        id: purchase_id("stale"),
        colony_id: colony(),
        study_id: StudyId::derive("administration_1"),
        expected_research_version: 0,
        expected_favor_version: funded.version,
        discount_basis_points: 0,
        consume_preparation: false,
        now_tick: 99,
    };
    assert_eq!(
        state.player_purchase(&mut funded, &base_catalog, stale),
        Err(ResearchPurchaseError::StaleResearchVersion)
    );
    assert_eq!(state.version, 1);
    assert_eq!(funded.event_count(), 2);
}

#[test]
fn automatic_purchase_uses_affordable_frontier_scores_and_never_consumes_preparation() {
    let mut permuted = catalog();
    permuted.studies.reverse();
    let mut progress = ResearchPurchaseState::new();
    let mut ledger = funded_ledger(9);
    let scores = BTreeMap::from([
        (
            StudyId::derive("duration_1"),
            AutomaticStudyScoreInputs {
                belief_basis_points: 10,
                posture_basis_points: 20,
                personality_basis_points: 30,
                dependency_basis_points: 40,
                expected_value_basis_points: 50,
            },
        ),
        (
            StudyId::derive("administration_1"),
            AutomaticStudyScoreInputs {
                expected_value_basis_points: 1_000,
                ..AutomaticStudyScoreInputs::default()
            },
        ),
        (
            StudyId::derive("expensive"),
            AutomaticStudyScoreInputs {
                expected_value_basis_points: 9_000,
                ..AutomaticStudyScoreInputs::default()
            },
        ),
    ]);

    let auto_one = automatic_request(
        "auto-one",
        &progress,
        &ledger,
        AutomaticResearchCapability::EffectiveLoremaster(EffectiveLevel::try_from(5).unwrap()),
        scores,
        100,
    );
    let outcome = progress
        .automatic_purchase(&mut ledger, &permuted, auto_one)
        .unwrap();
    assert_eq!(outcome.outcome, ResearchPurchaseOutcome::Committed);
    assert_eq!(outcome.study_id, StudyId::derive("administration_1"));
    assert_eq!(outcome.score, Some(1_000));
    assert_eq!(ledger.balance, Favor::from_whole(5).unwrap());
    let event = progress.purchases.get(&purchase_id("auto-one")).unwrap();
    assert_eq!(event.source, ResearchPurchaseSource::Automatic);
    assert_eq!(event.charged_price, event.undiscounted_price);
    assert_eq!(event.discount_basis_points, 0);
    assert!(!event.consumed_preparation);
    assert!(
        !progress
            .owned_studies
            .contains(&StudyId::derive("expensive"))
    );
}

#[test]
fn automatic_rolling_seven_day_quota_persists_across_restart_and_succession() {
    let catalog = catalog();
    let mut progress = ResearchPurchaseState::new();
    let mut ledger = funded_ledger(30);
    assert_eq!(
        [
            AutomaticResearchCapability::BeforeEffectiveLoremaster.quota_limit(),
            AutomaticResearchCapability::EffectiveLoremaster(EffectiveLevel::try_from(1).unwrap())
                .quota_limit(),
            AutomaticResearchCapability::EffectiveLoremaster(EffectiveLevel::try_from(2).unwrap())
                .quota_limit(),
            AutomaticResearchCapability::EffectiveLoremaster(EffectiveLevel::try_from(3).unwrap())
                .quota_limit(),
            AutomaticResearchCapability::EffectiveLoremaster(EffectiveLevel::try_from(4).unwrap())
                .quota_limit(),
            AutomaticResearchCapability::EffectiveLoremaster(EffectiveLevel::try_from(5).unwrap())
                .quota_limit(),
        ],
        [1, 1, 2, 2, 3, 4]
    );
    let level_two =
        AutomaticResearchCapability::EffectiveLoremaster(EffectiveLevel::try_from(2).unwrap());

    for (index, study_id) in ["dependency", "duration_1"].into_iter().enumerate() {
        let request = automatic_request(
            &format!("auto-{index}"),
            &progress,
            &ledger,
            level_two,
            BTreeMap::from([(
                StudyId::derive(study_id),
                AutomaticStudyScoreInputs {
                    expected_value_basis_points: 100,
                    ..AutomaticStudyScoreInputs::default()
                },
            )]),
            index as u64,
        );
        let outcome = progress
            .automatic_purchase(&mut ledger, &catalog, request)
            .unwrap();
        assert_eq!(outcome.quota_limit, 2);
        assert_eq!(outcome.quota_used_after, index + 1);
    }

    let restarted: ResearchPurchaseState =
        serde_json::from_value(serde_json::to_value(&progress).unwrap()).unwrap();
    progress = restarted;
    let blocked_by_succession = automatic_request(
        "succession-does-not-reset",
        &progress,
        &ledger,
        level_two,
        BTreeMap::new(),
        10,
    );
    assert_eq!(
        progress.automatic_purchase(&mut ledger, &catalog, blocked_by_succession),
        Err(ResearchPurchaseError::AutomaticQuotaExhausted)
    );
    assert_eq!(progress.version, 2);
    assert_eq!(ledger.event_count(), 3);

    let after_window = cat_sim::research_purchase::AUTOMATIC_RESEARCH_WINDOW_GAME_MINUTES + 1;
    let new_window = automatic_request(
        "new-window",
        &progress,
        &ledger,
        AutomaticResearchCapability::BeforeEffectiveLoremaster,
        BTreeMap::from([(
            StudyId::derive("duration_2"),
            AutomaticStudyScoreInputs {
                expected_value_basis_points: 100,
                ..AutomaticStudyScoreInputs::default()
            },
        )]),
        after_window,
    );
    let outcome = progress
        .automatic_purchase(&mut ledger, &catalog, new_window)
        .unwrap();
    assert_eq!(outcome.quota_limit, 1);
    assert_eq!(outcome.quota_used_after, 1);
    assert_eq!(outcome.study_id, StudyId::derive("duration_2"));
}

#[test]
fn unused_automatic_quota_never_carries_into_later_windows() {
    let catalog = SyntheticResearchCatalog::new(
        (0..8)
            .map(|index| study(&format!("independent_{index}"), 1, &[]))
            .collect(),
    );
    let mut progress = ResearchPurchaseState::new();
    let mut ledger = funded_ledger(20);
    let level_four =
        AutomaticResearchCapability::EffectiveLoremaster(EffectiveLevel::try_from(4).unwrap());

    let first = automatic_request(
        "window-one-used-one",
        &progress,
        &ledger,
        level_four,
        BTreeMap::from([(
            StudyId::derive("independent_0"),
            AutomaticStudyScoreInputs {
                expected_value_basis_points: 100,
                ..AutomaticStudyScoreInputs::default()
            },
        )]),
        1,
    );
    progress
        .automatic_purchase(&mut ledger, &catalog, first)
        .unwrap();
    assert_eq!(progress.automatic_quota.used_in_window(1), 1);

    let second_window_tick = cat_sim::research_purchase::AUTOMATIC_RESEARCH_WINDOW_GAME_MINUTES + 2;
    for index in 1..=3 {
        let request = automatic_request(
            &format!("window-two-{index}"),
            &progress,
            &ledger,
            level_four,
            BTreeMap::from([(
                StudyId::derive(&format!("independent_{index}")),
                AutomaticStudyScoreInputs {
                    expected_value_basis_points: 100,
                    ..AutomaticStudyScoreInputs::default()
                },
            )]),
            second_window_tick,
        );
        let outcome = progress
            .automatic_purchase(&mut ledger, &catalog, request)
            .unwrap();
        assert_eq!(outcome.quota_limit, 3);
        assert_eq!(outcome.quota_used_after, index);
    }
    let rejected = automatic_request(
        "window-two-no-carryover-fourth",
        &progress,
        &ledger,
        level_four,
        BTreeMap::from([(
            StudyId::derive("independent_4"),
            AutomaticStudyScoreInputs {
                expected_value_basis_points: 100,
                ..AutomaticStudyScoreInputs::default()
            },
        )]),
        second_window_tick,
    );
    assert_eq!(
        progress.automatic_purchase(&mut ledger, &catalog, rejected),
        Err(ResearchPurchaseError::AutomaticQuotaExhausted)
    );
}

#[test]
fn automatic_unaffordable_duplicate_and_rejected_attempts_consume_no_quota_or_favor() {
    let catalog = catalog();
    let mut progress = ResearchPurchaseState::new();
    let mut ledger = funded_ledger(1);
    let unaffordable = automatic_request(
        "unaffordable",
        &progress,
        &ledger,
        AutomaticResearchCapability::BeforeEffectiveLoremaster,
        BTreeMap::new(),
        1,
    );

    assert_eq!(
        progress.automatic_purchase(&mut ledger, &catalog, unaffordable),
        Err(ResearchPurchaseError::NoAffordableFrontier)
    );
    assert_eq!(progress.automatic_quota.used_in_window(1), 0);
    assert_eq!(ledger.balance, Favor::from_whole(1).unwrap());

    let mut funded = funded_ledger(10);
    let mut state = ResearchPurchaseState::new();
    let request = automatic_request(
        "dupe-auto",
        &state,
        &funded,
        AutomaticResearchCapability::BeforeEffectiveLoremaster,
        BTreeMap::from([(
            StudyId::derive("dependency"),
            AutomaticStudyScoreInputs {
                expected_value_basis_points: 200,
                ..AutomaticStudyScoreInputs::default()
            },
        )]),
        5,
    );
    let first = state
        .automatic_purchase(&mut funded, &catalog, request.clone())
        .unwrap();
    assert_eq!(first.outcome, ResearchPurchaseOutcome::Committed);
    let replay = state
        .automatic_purchase(&mut funded, &catalog, request)
        .unwrap();
    assert_eq!(replay.outcome, ResearchPurchaseOutcome::AlreadyCommitted);
    assert_eq!(state.automatic_quota.used_in_window(5), 1);
    assert_eq!(funded.event_count(), 2);
}

#[test]
fn strict_catalog_state_and_restart_validation_reject_malformed_shapes() {
    let mut bad_catalog =
        SyntheticResearchCatalog::new(vec![study("a", 1, &["b"]), study("b", 1, &["a"])]);
    bad_catalog.studies[0].prerequisites.sort();
    bad_catalog.studies[1].prerequisites.sort();
    assert_eq!(
        bad_catalog.validate(),
        Err(ResearchPurchaseError::MalformedCatalog)
    );

    let catalog = catalog();
    let mut progress = ResearchPurchaseState::new();
    let mut ledger = funded_ledger(10);
    let valid = player_request("valid", "dependency", &progress, &ledger, 0);
    progress
        .player_purchase(&mut ledger, &catalog, valid)
        .unwrap();
    let mut value = serde_json::to_value(&progress).unwrap();
    value["purchases"][purchase_id("valid").as_str()]["chargedPrice"] = serde_json::json!(0);
    assert!(serde_json::from_value::<ResearchPurchaseState>(value).is_err());

    let mut mismatched_event_id = serde_json::to_value(&progress).unwrap();
    mismatched_event_id["purchases"][purchase_id("valid").as_str()]["id"] =
        serde_json::json!(purchase_id("different").as_str());
    assert!(serde_json::from_value::<ResearchPurchaseState>(mismatched_event_id).is_err());

    let mut quota = serde_json::to_value(&progress).unwrap();
    quota["automaticQuota"]["committedTicks"] = serde_json::json!([10, 5]);
    assert!(serde_json::from_value::<ResearchPurchaseState>(quota).is_err());
}
