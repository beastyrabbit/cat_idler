use cat_sim::{
    favor::{Favor, FavorCommitOutcome, FavorLedger, MICRO_FAVOR_PER_FAVOR},
    leader_planner::EffectiveLevel,
    planner_core::PlannerId,
    shrine_offerings::{
        OfferingBeliefEstimate, OfferingCargoDisposition, OfferingError, OfferingPackage,
        OfferingStage, ShrineOfferingReview, ShrineOfferingReviewBlock,
        ShrineOfferingReviewContext, ShrineOfferingState,
    },
};

fn estimate(
    package: OfferingPackage,
    believed_available_lower: u64,
    replacement_minutes: u64,
    evidence_id: &str,
) -> OfferingBeliefEstimate {
    OfferingBeliefEstimate {
        package,
        believed_available_lower,
        replacement_minutes,
        labor_minutes: 60,
        reserve_risk_basis_points: 0,
        committed_use_penalty_basis_points: 0,
        confidence_basis_points: 8_000,
        evidence_ids: vec![evidence_id.to_owned()],
    }
}

fn context(level: u8, review_bucket: u64) -> ShrineOfferingReviewContext {
    ShrineOfferingReviewContext {
        world_seed: 42_424_242,
        colony_id: PlannerId::derive("test_colony", ["one"]),
        leader_id: PlannerId::derive("test_leader", ["mothstar"]),
        review_bucket,
        effective_level: EffectiveLevel::try_from(level).unwrap(),
        covered_by_officer_request: false,
        survival_or_active_defense: false,
    }
}

fn estimates_with_only(package: OfferingPackage) -> Vec<OfferingBeliefEstimate> {
    OfferingPackage::ALL
        .into_iter()
        .map(|candidate| {
            let available = if candidate == package {
                candidate.quantity()
            } else {
                candidate.quantity() - 1
            };
            estimate(
                candidate,
                available,
                30,
                &format!("belief-{}", candidate.stable_id()),
            )
        })
        .collect()
}

fn complete_current(
    state: &mut ShrineOfferingState,
    ledger: &mut FavorLedger,
    expected_version: u64,
    start_tick: u64,
) -> FavorCommitOutcome {
    let pipeline = state.current.as_mut().unwrap();
    pipeline
        .resources_reserved(format!("task-{}", pipeline.occurrence), start_tick + 1)
        .unwrap();
    pipeline.depart(start_tick + 2).unwrap();
    pipeline.deposit(start_tick + 3).unwrap();
    pipeline.begin_ritual(start_tick + 4).unwrap();
    pipeline
        .consume_and_credit(true, ledger, expected_version, start_tick + 5)
        .unwrap()
}

#[test]
fn endless_four_package_physical_loop_credits_exactly_one_favor_each_time() {
    let mut state = ShrineOfferingState::new("shrine-alpha");
    let mut ledger = FavorLedger::new();

    for (index, package) in OfferingPackage::ALL.into_iter().enumerate() {
        let review = state
            .consider_endless_offering(
                &context(5, index as u64),
                &estimates_with_only(package),
                index as u64 * 10,
            )
            .unwrap();
        let ShrineOfferingReview::Started { choice, .. } = review else {
            panic!("level five non-emergency review should start the offering");
        };
        assert_eq!(choice.package, package);
        assert_eq!(package.base_favor(), Favor::ONE);

        let version = ledger.version;
        assert_eq!(
            complete_current(&mut state, &mut ledger, version, index as u64 * 10),
            FavorCommitOutcome::Committed
        );
        let pipeline = state.current.as_mut().unwrap();
        assert_eq!(pipeline.stage, OfferingStage::Completed);
        assert_eq!(
            pipeline
                .consume_and_credit(true, &mut ledger, version, index as u64 * 10 + 9)
                .unwrap(),
            FavorCommitOutcome::AlreadyCommitted
        );
        assert_eq!(ledger.event_count(), index + 1);
        assert_eq!(
            ledger.balance.micro_favor(),
            (index as u64 + 1) * MICRO_FAVOR_PER_FAVOR
        );

        let restarted: ShrineOfferingState =
            serde_json::from_value(serde_json::to_value(&state).unwrap()).unwrap();
        assert_eq!(restarted, state);
        state = restarted;
    }

    assert_eq!(state.next_occurrence, 4);
    assert_eq!(ledger.balance, Favor::from_whole(4).unwrap());
}

#[test]
fn belief_only_selection_preserves_poor_stale_choices_and_has_no_hidden_fallback() {
    let mut state = ShrineOfferingState::new("shrine-alpha");
    let stale_poor_estimates = [
        estimate(
            OfferingPackage::Food,
            100,
            5,
            "stale-food-report-before-spoilage",
        ),
        estimate(
            OfferingPackage::Herbs,
            OfferingPackage::Herbs.quantity(),
            500,
            "fresh-herb-report",
        ),
    ];

    let review = state
        .consider_endless_offering(&context(5, 0), &stale_poor_estimates, 1)
        .unwrap();
    let ShrineOfferingReview::Started { choice, .. } = review else {
        panic!("believed available food should be eligible even if the report is stale");
    };
    assert_eq!(choice.package, OfferingPackage::Food);
    assert_eq!(
        choice.evidence_ids,
        vec!["stale-food-report-before-spoilage".to_owned()]
    );

    let mut no_fallback_state = ShrineOfferingState::new("shrine-beta");
    assert_eq!(
        no_fallback_state
            .consider_endless_offering(
                &context(5, 1),
                &[OfferingBeliefEstimate {
                    package: OfferingPackage::Materials,
                    believed_available_lower: OfferingPackage::Materials.quantity(),
                    replacement_minutes: 1,
                    labor_minutes: 1,
                    reserve_risk_basis_points: 0,
                    committed_use_penalty_basis_points: 0,
                    confidence_basis_points: 0,
                    evidence_ids: vec!["hidden-stock-is-not-a-belief".to_owned()],
                }],
                1,
            )
            .unwrap(),
        ShrineOfferingReview::Deferred {
            reason: ShrineOfferingReviewBlock::NoBelievedPackage
        }
    );
    assert!(no_fallback_state.current.is_none());
}

#[test]
fn leader_omission_can_forget_eligible_shrine_review_without_starting_fallback_work() {
    let omitted_bucket = (0..10_000)
        .find(|bucket| {
            matches!(
                ShrineOfferingState::new("probe")
                    .consider_endless_offering(
                        &context(1, *bucket),
                        &estimates_with_only(OfferingPackage::Herbs),
                        1,
                    )
                    .unwrap(),
                ShrineOfferingReview::Omitted { .. }
            )
        })
        .expect("bounded deterministic seed matrix should include a level-one omission");
    let mut state = ShrineOfferingState::new("shrine-alpha");
    let review = state
        .consider_endless_offering(
            &context(1, omitted_bucket),
            &estimates_with_only(OfferingPackage::Herbs),
            1,
        )
        .unwrap();

    let ShrineOfferingReview::Omitted {
        roll_basis_points,
        omission_basis_points,
    } = review
    else {
        panic!("selected bucket must omit");
    };
    assert!(roll_basis_points < omission_basis_points);
    assert!(state.current.is_none());

    let mut defended = ShrineOfferingState::new("shrine-alpha");
    let defended_context = ShrineOfferingReviewContext {
        survival_or_active_defense: true,
        ..context(5, 0)
    };
    assert_eq!(
        defended
            .consider_endless_offering(
                &defended_context,
                &estimates_with_only(OfferingPackage::Food),
                1,
            )
            .unwrap(),
        ShrineOfferingReview::Deferred {
            reason: ShrineOfferingReviewBlock::SurvivalOrActiveDefense
        }
    );
    assert!(defended.current.is_none());
}

#[test]
fn one_leader_cadence_bucket_cannot_start_repeated_offerings() {
    let mut state = ShrineOfferingState::new("shrine-alpha");
    let review = state
        .consider_endless_offering(
            &context(5, 7),
            &estimates_with_only(OfferingPackage::Herbs),
            1,
        )
        .unwrap();
    assert!(matches!(review, ShrineOfferingReview::Started { .. }));
    let mut ledger = FavorLedger::new();
    complete_current(&mut state, &mut ledger, 0, 1);

    assert_eq!(
        state
            .consider_endless_offering(
                &context(5, 7),
                &estimates_with_only(OfferingPackage::Materials),
                10,
            )
            .unwrap(),
        ShrineOfferingReview::Deferred {
            reason: ShrineOfferingReviewBlock::CadenceNotDue
        }
    );
    assert_eq!(state.next_occurrence, 1);

    assert!(matches!(
        state
            .consider_endless_offering(
                &context(5, 8),
                &estimates_with_only(OfferingPackage::Materials),
                11,
            )
            .unwrap(),
        ShrineOfferingReview::Started { .. }
    ));
    assert_eq!(state.next_occurrence, 2);
}

#[test]
fn restart_replay_cannot_consume_or_credit_the_same_physical_ritual_twice() {
    let mut state = ShrineOfferingState::new("shrine-alpha");
    assert!(matches!(
        state
            .consider_endless_offering(
                &context(5, 0),
                &estimates_with_only(OfferingPackage::RefinedResources),
                1,
            )
            .unwrap(),
        ShrineOfferingReview::Started { .. }
    ));
    let pipeline = state.current.as_mut().unwrap();
    pipeline.resources_reserved("task-offering", 2).unwrap();
    pipeline.depart(3).unwrap();
    pipeline.deposit(4).unwrap();
    pipeline.begin_ritual(5).unwrap();

    let ritual_state = serde_json::to_value(&state).unwrap();
    let serialized = serde_json::to_string(&ritual_state).unwrap();
    for removed_mechanic in ["cooldown", "tithe", "scalar", "blessing"] {
        assert!(!serialized.contains(removed_mechanic));
    }

    let mut ledger = FavorLedger::new();
    let mut first_restart: ShrineOfferingState =
        serde_json::from_value(ritual_state.clone()).unwrap();
    assert_eq!(
        first_restart
            .current
            .as_mut()
            .unwrap()
            .consume_and_credit(true, &mut ledger, 0, 6)
            .unwrap(),
        FavorCommitOutcome::Committed
    );
    assert_eq!(ledger.event_count(), 1);
    assert_eq!(ledger.balance, Favor::ONE);

    let mut second_restart: ShrineOfferingState = serde_json::from_value(ritual_state).unwrap();
    assert_eq!(
        second_restart
            .current
            .as_mut()
            .unwrap()
            .consume_and_credit(true, &mut ledger, 0, 6)
            .unwrap(),
        FavorCommitOutcome::AlreadyCommitted
    );
    assert_eq!(ledger.event_count(), 1);
    assert_eq!(ledger.balance, Favor::ONE);

    let mut corrupt_completed = serde_json::to_value(&second_restart).unwrap();
    corrupt_completed["current"]["creditedEventId"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<ShrineOfferingState>(corrupt_completed).is_err());
}

#[test]
fn cancellation_salvage_records_exact_cargo_disposition_without_favor_credit() {
    let mut state = ShrineOfferingState::new("shrine-alpha");
    assert!(matches!(
        state
            .consider_endless_offering(
                &context(5, 0),
                &estimates_with_only(OfferingPackage::Materials),
                10,
            )
            .unwrap(),
        ShrineOfferingReview::Started { .. }
    ));
    let mut ledger = FavorLedger::new();
    let pipeline = state.current.as_mut().unwrap();
    pipeline.resources_reserved("task-offering", 11).unwrap();
    pipeline.cancel_before_departure(12).unwrap();
    assert_eq!(pipeline.stage, OfferingStage::Cancelled);
    assert_eq!(
        pipeline.cargo_disposition,
        Some(OfferingCargoDisposition::ReleasedBeforePickup)
    );
    assert_eq!(
        pipeline.consume_and_credit(true, &mut ledger, 0, 13),
        Err(OfferingError::PhysicalOfferingIncomplete)
    );
    assert_eq!(ledger.balance, Favor::ZERO);

    assert!(matches!(
        state
            .consider_endless_offering(
                &context(5, 1),
                &estimates_with_only(OfferingPackage::Herbs),
                20,
            )
            .unwrap(),
        ShrineOfferingReview::Started { .. }
    ));
    let pipeline = state.current.as_mut().unwrap();
    pipeline.resources_reserved("task-offering-2", 21).unwrap();
    pipeline.depart(22).unwrap();
    pipeline
        .block_after_cargo_salvage("route_blocked", "safe-stockpile-1", 23)
        .unwrap();
    assert_eq!(pipeline.stage, OfferingStage::Blocked);
    assert_eq!(
        pipeline.cargo_disposition,
        Some(OfferingCargoDisposition::SalvagedToStockpile {
            stockpile_id: "safe-stockpile-1".to_owned()
        })
    );
    assert_eq!(
        pipeline.consume_and_credit(true, &mut ledger, 0, 24),
        Err(OfferingError::PhysicalOfferingIncomplete)
    );
    assert_eq!(ledger.event_count(), 0);

    let restarted: ShrineOfferingState =
        serde_json::from_value(serde_json::to_value(&state).unwrap()).unwrap();
    assert_eq!(restarted, state);
}
