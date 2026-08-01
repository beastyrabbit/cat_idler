//! Focused LAI.44 progression contract.

use std::collections::{BTreeMap, BTreeSet};

use cat_sim::{
    content_manifest::{
        CapabilityId, CapabilityRequirement, ContentId, ContentManifest, ContentOperation, RecipeId,
    },
    planner_core::PlannerId,
    progression_research::{
        BOOST_RESEARCH_STAGE_COUNT, BOOST_UNLOCK_STUDY_COUNT, ColonyPartitionKey,
        CurrencyCommitOutcome, CurrencyEventId, HOLE_AXIS_STUDY_COUNT, HoleAxis,
        HoleVoidCreditPayload, LeaderDuplicateHook, LeaderTargetDecision, MAX_ACTIVE_SCHOLAR_WORK,
        MAX_DRAIN_BATCH, OOPSIE_PERCENT_BY_LEVEL, PLAYER_PREPARATION_DISCOUNT_BASIS_POINTS,
        PlayerPartitionKey, ProgressionAuthority, ProgressionCatalog, ProgressionError,
        RecipeCapabilityCheck, ResearchNotes, ResearchNotesLedger, ResearchNotesSpendRequest,
        ScholarAssignment, ScholarWorkStage, SpecializedBoost, StudyCurrency, StudyId, StudyKind,
        VoidInsight, WorkRecoveryReason,
    },
};

fn colony(name: &str) -> PlannerId {
    PlannerId::derive("lai44_colony", [name])
}

fn player(name: &str) -> PlannerId {
    PlannerId::derive("lai44_player", [name])
}

fn colony_key(name: &str) -> ColonyPartitionKey {
    ColonyPartitionKey {
        colony_id: colony(name),
    }
}

fn player_key(colony_name: &str, player_name: &str) -> PlayerPartitionKey {
    PlayerPartitionKey {
        colony_id: colony(colony_name),
        player_id: player(player_name),
    }
}

fn assignment(name: &str) -> ScholarAssignment {
    ScholarAssignment {
        scholar_id: PlannerId::derive("lai44_scholar", [name]),
        tool_id: PlannerId::derive("lai44_tool", [name]),
        station_id: PlannerId::derive("lai44_station", [name]),
        location_id: PlannerId::derive("lai44_location", [name]),
    }
}

fn study(name: &str) -> StudyId {
    StudyId::new(name).expect("valid test study")
}

fn content(name: &str) -> ContentId {
    ContentId::new(name).expect("valid content")
}

fn recipe(name: &str) -> RecipeId {
    RecipeId::new(name).expect("valid recipe")
}

fn complete_work(
    state: &mut ProgressionAuthority,
    id: &cat_sim::progression_research::ScholarWorkId,
    required: u64,
) {
    state
        .reserve_work(id, state.version)
        .expect("reserve physical work");
    state
        .start_work(id, state.version)
        .expect("start physical work");
    assert_eq!(
        state
            .progress_work(id, required, state.version)
            .expect("complete physical work"),
        ScholarWorkStage::Completed
    );
}

fn produce_notes(state: &mut ProgressionAuthority, minutes: u64) {
    let id = state
        .queue_notes_work(
            state.partition.clone(),
            assignment("notes"),
            minutes,
            state.version,
        )
        .expect("queue notes work");
    complete_work(state, &id, minutes);
    let credits = state
        .drain_terminal_work(MAX_DRAIN_BATCH)
        .expect("drain completed work");
    assert_eq!(credits.len(), 1);
    assert_eq!(credits[0].work_id, id);
}

fn produce_void(state: &mut ProgressionAuthority, whole: u64) {
    let amount = VoidInsight::from_whole(whole).expect("test amount fits");
    state
        .void
        .credit_hole_feed(HoleVoidCreditPayload {
            partition: state.partition.clone(),
            feed_sequence: state.void.credited_feed_through + 1,
            amount,
        })
        .expect("physical feed credit");
}

#[test]
fn founding_and_every_nonfree_content_have_one_manifest_capability_study() {
    let manifest = ContentManifest::embedded();
    let catalog = ProgressionCatalog::from_embedded().expect("canonical progression catalog");
    assert_eq!(
        catalog.founding_capabilities(),
        &BTreeSet::from([
            CapabilityId::new("apple_gathering").unwrap(),
            CapabilityId::new("basic_food_handling").unwrap(),
            CapabilityId::new("hand_fishing").unwrap(),
            CapabilityId::new("water_collection").unwrap(),
        ])
    );
    for raw in ["logs", "stone"] {
        let resource = manifest
            .resources
            .iter()
            .find(|resource| resource.id.as_str() == raw)
            .unwrap();
        assert!(resource.acquisition.founding_available);
        assert_eq!(resource.canonical_capability, CapabilityRequirement::Free);
    }
    assert_eq!(
        catalog.derived_capability_study_total() + catalog.founding_capabilities().len(),
        manifest.derived_capability_total()
    );

    let mut grants = BTreeMap::<&ContentId, Vec<&CapabilityId>>::new();
    for capability in &manifest.capabilities {
        for content_id in &capability.canonical_for {
            grants.entry(content_id).or_default().push(&capability.id);
        }
    }
    let required_content = manifest
        .resources
        .iter()
        .map(|entry| (&entry.content_id, &entry.canonical_capability))
        .chain(
            manifest
                .foods
                .iter()
                .map(|entry| (&entry.content_id, &entry.canonical_capability)),
        )
        .chain(
            manifest
                .item_definitions
                .iter()
                .map(|entry| (&entry.content_id, &entry.canonical_capability)),
        )
        .chain(
            manifest
                .materials
                .iter()
                .map(|entry| (&entry.content_id, &entry.canonical_capability)),
        )
        .chain(
            manifest
                .creatures
                .iter()
                .map(|entry| (&entry.content_id, &entry.canonical_capability)),
        )
        .chain(
            manifest
                .stations
                .iter()
                .map(|entry| (&entry.content_id, &entry.canonical_capability)),
        )
        .chain(
            manifest
                .recipes
                .iter()
                .map(|entry| (&entry.content_id, &entry.canonical_capability)),
        )
        .chain(
            manifest
                .augmentations
                .iter()
                .map(|entry| (&entry.content_id, &entry.canonical_capability)),
        )
        .chain(
            manifest
                .fixtures
                .iter()
                .map(|entry| (&entry.content_id, &entry.canonical_capability)),
        );
    for (content_id, requirement) in required_content {
        match requirement {
            CapabilityRequirement::Free => assert!(!grants.contains_key(content_id)),
            CapabilityRequirement::Required(capability_id) => {
                assert_eq!(grants.get(content_id).unwrap(), &[capability_id]);
                let study = catalog.study(&StudyId::from_capability(capability_id));
                if catalog.founding_capabilities().contains(capability_id) {
                    assert!(
                        study.is_none(),
                        "founding capability {capability_id:?} unexpectedly has a study"
                    );
                } else {
                    assert!(
                        study.is_some(),
                        "content {content_id:?} requires capability {capability_id:?}, but the progression catalog has no matching study"
                    );
                }
            }
        }
    }
    assert_eq!(
        catalog
            .studies()
            .keys()
            .filter(|id| id.as_str() == "plank_processing")
            .count(),
        1
    );
    assert!(
        catalog
            .studies()
            .keys()
            .all(|id| !id.as_str().starts_with("recipe_"))
    );
}

#[test]
fn exactly_thirty_hole_axis_studies_and_boost_research_use_only_void() {
    let catalog = ProgressionCatalog::from_embedded().unwrap();
    assert_eq!(catalog.hole_axis_studies().len(), HOLE_AXIS_STUDY_COUNT);
    for axis in [HoleAxis::Width, HoleAxis::Depth, HoleAxis::Darkness] {
        assert_eq!(
            catalog
                .hole_axis_studies()
                .into_iter()
                .filter_map(|entry| match entry.kind {
                    StudyKind::HoleAxis {
                        axis: candidate,
                        level,
                    } if candidate == axis => Some(level),
                    _ => None,
                })
                .collect::<BTreeSet<_>>(),
            (1..=10).collect()
        );
    }
    assert_eq!(
        catalog.boost_unlock_studies().len(),
        BOOST_UNLOCK_STUDY_COUNT
    );
    assert_eq!(SpecializedBoost::ALL.len(), BOOST_UNLOCK_STUDY_COUNT);
    assert_eq!(
        catalog
            .studies()
            .values()
            .filter(|entry| matches!(entry.kind, StudyKind::BoostDuration { .. }))
            .count(),
        usize::from(BOOST_RESEARCH_STAGE_COUNT)
    );
    assert_eq!(
        catalog
            .studies()
            .values()
            .filter(|entry| matches!(entry.kind, StudyKind::BoostEconomy { .. }))
            .count(),
        usize::from(BOOST_RESEARCH_STAGE_COUNT)
    );
    for entry in catalog.studies().values() {
        match entry.kind {
            StudyKind::OrdinaryCapability { .. } => {
                assert_eq!(entry.currency(), StudyCurrency::Notes)
            }
            StudyKind::HoleAxis { .. }
            | StudyKind::BoostUnlock { .. }
            | StudyKind::BoostDuration { .. }
            | StudyKind::BoostEconomy { .. } => {
                assert_eq!(entry.currency(), StudyCurrency::Void)
            }
        }
    }
}

#[test]
fn locked_content_is_visible_and_movable_but_guarded_for_every_mutating_operation() {
    let catalog = ProgressionCatalog::from_embedded().unwrap();
    let mut state = ProgressionAuthority::new(colony("gates")).unwrap();
    let planks = content("resource_planks");
    for operation in [
        ContentOperation::Discover,
        ContentOperation::Store,
        ContentOperation::Trade,
    ] {
        state
            .guard_operation(&catalog, &planks, operation)
            .expect("non-use operation remains available");
    }
    for operation in [
        ContentOperation::Process,
        ContentOperation::Craft,
        ContentOperation::FeedHole,
    ] {
        assert_eq!(
            state.guard_operation(&catalog, &planks, operation),
            Err(ProgressionError::CapabilityLocked)
        );
    }
    state.owned_studies.insert(study("plank_processing"));
    for operation in [
        ContentOperation::Process,
        ContentOperation::Craft,
        ContentOperation::FeedHole,
    ] {
        state
            .guard_operation(&catalog, &planks, operation)
            .expect("canonical capability unlocks every use guard");
    }
    assert_eq!(
        state.guard_operation(&catalog, &planks, ContentOperation::InstallFixture),
        Err(ProgressionError::InvalidOperationClass)
    );
    assert_eq!(
        state.guard_operation(&catalog, &planks, ContentOperation::Augment),
        Err(ProgressionError::InvalidOperationClass)
    );
    assert_eq!(
        state.guard_operation(
            &catalog,
            &content("fixture_workshop"),
            ContentOperation::InstallFixture
        ),
        Err(ProgressionError::CapabilityLocked)
    );
    assert_eq!(
        state.guard_operation(
            &catalog,
            &content("augmentation_weapon_augmentation"),
            ContentOperation::Augment
        ),
        Err(ProgressionError::CapabilityLocked)
    );
    assert_eq!(
        state.guard_operation(
            &catalog,
            &content("not_manifest_content"),
            ContentOperation::Store
        ),
        Err(ProgressionError::UnknownContent)
    );
}

#[test]
fn recipe_bundle_gate_has_no_recipe_study_and_checks_all_formula_inputs() {
    let catalog = ProgressionCatalog::from_embedded().unwrap();
    let mut state = ProgressionAuthority::new(colony("recipe")).unwrap();
    state.owned_studies.insert(study("plank_processing"));
    let valid = RecipeCapabilityCheck {
        recipe_id: recipe("logs_to_planks"),
        station_exists: true,
        station_tier: 1,
        physical_ingredients_ready: true,
        tools_ready: true,
        capacity_ready: true,
        workers_ready: true,
    };
    state
        .guard_recipe(&catalog, &valid)
        .expect("resource-owned bundle is available");
    for invalid in [
        RecipeCapabilityCheck {
            station_exists: false,
            ..valid.clone()
        },
        RecipeCapabilityCheck {
            station_tier: 0,
            ..valid.clone()
        },
        RecipeCapabilityCheck {
            physical_ingredients_ready: false,
            ..valid.clone()
        },
        RecipeCapabilityCheck {
            tools_ready: false,
            ..valid.clone()
        },
        RecipeCapabilityCheck {
            capacity_ready: false,
            ..valid.clone()
        },
        RecipeCapabilityCheck {
            workers_ready: false,
            ..valid.clone()
        },
    ] {
        assert_eq!(
            state.guard_recipe(&catalog, &invalid),
            Err(ProgressionError::PhysicalPrerequisiteMissing)
        );
    }
}

#[test]
fn scholar_work_is_physical_idempotent_and_recovery_releases_exact_assignment() {
    let mut state = ProgressionAuthority::new(colony("scholars")).unwrap();
    let assigned = assignment("one");
    let id = state
        .queue_notes_work(state.partition.clone(), assigned.clone(), 20, state.version)
        .unwrap();
    let order = state.work_orders.get(&id.sequence).unwrap();
    assert_eq!(order.stage, ScholarWorkStage::Queued);
    assert_eq!(order.progress_work_minutes, 0);
    assert_eq!(order.required_work_minutes, 20);
    state.reserve_work(&id, state.version).unwrap();
    state.start_work(&id, state.version).unwrap();
    assert_eq!(
        state.progress_work(&id, 7, state.version).unwrap(),
        ScholarWorkStage::Working
    );
    let before_recovery = state.clone();
    assert_eq!(
        state.progress_work(&id, 0, state.version),
        Err(ProgressionError::MalformedRequest)
    );
    assert_eq!(state, before_recovery);
    let release = state
        .recover_work(&id, WorkRecoveryReason::ScholarDied, state.version)
        .unwrap();
    assert_eq!(release.assignment, assigned);
    assert!(release.released_reservation);
    assert_eq!(release.lost_progress_work_minutes, 7);
    assert_eq!(state.notes.balance, ResearchNotes::ZERO);
    assert!(state.drain_terminal_work(1).unwrap().is_empty());
    assert_eq!(state.notes.retired_work_through, 1);
    assert_eq!(
        state.reserve_work(&id, state.version),
        Err(ProgressionError::RetiredReplay)
    );
}

#[test]
fn completed_work_alone_emits_notes_and_restart_partition_twins_match() {
    let mut fine = ProgressionAuthority::new(colony("restart")).unwrap();
    let id = fine
        .queue_notes_work(
            fine.partition.clone(),
            assignment("partition"),
            12,
            fine.version,
        )
        .unwrap();
    fine.reserve_work(&id, fine.version).unwrap();
    fine.start_work(&id, fine.version).unwrap();
    fine.progress_work(&id, 5, fine.version).unwrap();
    let mut restarted: ProgressionAuthority =
        serde_json::from_value(serde_json::to_value(&fine).unwrap()).unwrap();
    fine.progress_work(&id, 7, fine.version).unwrap();
    restarted.progress_work(&id, 7, restarted.version).unwrap();
    assert_eq!(fine, restarted);
    assert_eq!(fine.notes.balance, ResearchNotes::ZERO);
    let fine_credit = fine.drain_terminal_work(1).unwrap();
    let restarted_credit = restarted.drain_terminal_work(1).unwrap();
    assert_eq!(fine_credit, restarted_credit);
    assert_eq!(fine, restarted);
    assert_eq!(fine.notes.balance, ResearchNotes::from_micro(12_000));

    let other = ProgressionAuthority::new(colony("other")).unwrap();
    assert_eq!(other.notes.balance, ResearchNotes::ZERO);
    assert_eq!(
        fine.queue_notes_work(colony_key("other"), assignment("wrong"), 1, fine.version),
        Err(ProgressionError::PartitionMismatch)
    );
}

#[test]
fn one_completed_preparation_freezes_floor_discount_and_is_consumed_once() {
    assert_eq!(PLAYER_PREPARATION_DISCOUNT_BASIS_POINTS, 2_500);
    let catalog = ProgressionCatalog::from_embedded().unwrap();
    let mut state = ProgressionAuthority::new(colony("prepare")).unwrap();
    produce_notes(&mut state, 2_000);
    let target = study("plank_processing");
    let definition = catalog.study(&target).unwrap();
    let base = definition.notes_cost().unwrap();
    let work = state
        .queue_preparation_work(
            &catalog,
            player_key("prepare", "owner"),
            assignment("prepare"),
            target.clone(),
            state.version,
        )
        .unwrap();
    complete_work(
        &mut state,
        &work,
        definition.required_work_minutes.div_ceil(4),
    );
    let prepared = state.prepared.get(&target).unwrap();
    assert_eq!(prepared.frozen_discount.micro(), base.micro() / 4);
    assert_eq!(
        prepared.frozen_payable.micro(),
        base.micro() - base.micro() / 4
    );
    state
        .claim_god_queue(
            &catalog,
            player_key("prepare", "owner"),
            target.clone(),
            state.version,
        )
        .unwrap();
    assert!(!state.prepared.contains_key(&target));
    assert!(state.god_queued[&target].prepared_terms.is_some());
    state
        .fund_god_study(
            &catalog,
            player_key("prepare", "owner"),
            target.clone(),
            CurrencyEventId::derive("study", &colony("prepare"), "planks"),
            state.version,
        )
        .unwrap();
    assert_eq!(
        state.funded[&target].paid_micro,
        base.micro() - base.micro() / 4
    );
    assert!(state.funded[&target].preparation_consumed);
    assert_eq!(
        state.queue_preparation_work(
            &catalog,
            player_key("prepare", "owner"),
            assignment("duplicate"),
            target,
            state.version,
        ),
        Err(ProgressionError::DuplicateLaneClaim)
    );

    let void_target = study("black_hole_width_01");
    assert_eq!(
        state.queue_preparation_work(
            &catalog,
            player_key("prepare", "owner"),
            assignment("void"),
            void_target,
            state.version,
        ),
        Err(ProgressionError::PreparationIneligible)
    );
}

#[test]
fn god_lane_funding_uses_the_typed_ledger_and_completion_still_requires_physical_work() {
    let catalog = ProgressionCatalog::from_embedded().unwrap();
    let mut ordinary = ProgressionAuthority::new(colony("god_notes")).unwrap();
    produce_notes(&mut ordinary, 2_000);
    produce_void(&mut ordinary, 20);
    let ordinary_target = study("plank_processing");
    ordinary
        .claim_god_queue(
            &catalog,
            player_key("god_notes", "owner"),
            ordinary_target.clone(),
            ordinary.version,
        )
        .unwrap();
    let void_before = ordinary.void.balance;
    ordinary
        .fund_god_study(
            &catalog,
            player_key("god_notes", "owner"),
            ordinary_target.clone(),
            CurrencyEventId::derive("god_study", &colony("god_notes"), "planks"),
            ordinary.version,
        )
        .unwrap();
    assert_eq!(
        ordinary.funded[&ordinary_target].currency,
        StudyCurrency::Notes
    );
    assert_eq!(ordinary.void.balance, void_before);
    assert!(!ordinary.owned_studies.contains(&ordinary_target));
    let required = catalog
        .study(&ordinary_target)
        .unwrap()
        .required_work_minutes;
    let work = ordinary
        .queue_funded_study_work(
            &catalog,
            player_key("god_notes", "owner"),
            assignment("god_notes"),
            ordinary_target.clone(),
            ordinary.version,
        )
        .unwrap();
    complete_work(&mut ordinary, &work, required);
    assert!(ordinary.owned_studies.contains(&ordinary_target));
    assert!(!ordinary.funded.contains_key(&ordinary_target));

    let mut hole = ProgressionAuthority::new(colony("god_void")).unwrap();
    produce_notes(&mut hole, 2_000);
    produce_void(&mut hole, 20);
    hole.owned_studies.insert(study("black_hole_foundations"));
    let hole_target = study("black_hole_width_01");
    hole.claim_god_queue(
        &catalog,
        player_key("god_void", "owner"),
        hole_target.clone(),
        hole.version,
    )
    .unwrap();
    let notes_before = hole.notes.balance;
    hole.fund_god_study(
        &catalog,
        player_key("god_void", "owner"),
        hole_target.clone(),
        CurrencyEventId::derive("god_study", &colony("god_void"), "width"),
        hole.version,
    )
    .unwrap();
    assert_eq!(hole.funded[&hole_target].currency, StudyCurrency::Void);
    assert_eq!(hole.notes.balance, notes_before);
}

#[test]
fn lane_exclusion_prefers_an_alternative_with_critical_and_keyed_hooks_explicit() {
    assert_eq!(OOPSIE_PERCENT_BY_LEVEL, [25, 12, 5, 1, 0]);
    let catalog = ProgressionCatalog::from_embedded().unwrap();
    let mut state = ProgressionAuthority::new(colony("lanes")).unwrap();
    let first = study("plank_processing");
    let second = study("black_hole_foundations");
    state
        .claim_god_queue(
            &catalog,
            player_key("lanes", "owner"),
            first.clone(),
            state.version,
        )
        .unwrap();
    assert_eq!(
        state
            .select_leader_target(&[first.clone(), second.clone()], &BTreeMap::new())
            .unwrap(),
        Some((second, LeaderTargetDecision::Allowed))
    );
    assert_eq!(
        state
            .leader_target_decision(
                &first,
                &LeaderDuplicateHook::VillageCritical {
                    report_marks_critical: true,
                    needed_before_tick: 10,
                    estimated_god_completion_tick: 20,
                }
            )
            .unwrap(),
        LeaderTargetDecision::VillageCriticalOverride
    );
    let key = PlannerId::derive("lai44_oopsie", ["stable"]);
    assert_eq!(
        state
            .leader_target_decision(
                &first,
                &LeaderDuplicateHook::KeyedOopsie {
                    decision_key: key,
                    effective_level: 3,
                    keyed_roll_percent: 0,
                }
            )
            .unwrap(),
        LeaderTargetDecision::KeyedOopsieOverride
    );
    assert_eq!(
        state
            .leader_target_decision(
                &first,
                &LeaderDuplicateHook::KeyedOopsie {
                    decision_key: PlannerId::derive("lai44_oopsie", ["miss"]),
                    effective_level: 3,
                    keyed_roll_percent: 1,
                }
            )
            .unwrap(),
        LeaderTargetDecision::ChooseAnother
    );
}

#[test]
fn currency_receipts_replay_conflict_and_checked_arithmetic_are_atomic() {
    let mut notes = ResearchNotesLedger::new(colony("ledger"));
    notes.balance = ResearchNotes::from_micro(10);
    let request = ResearchNotesSpendRequest {
        id: CurrencyEventId::derive("test", &colony("ledger"), "one"),
        amount: ResearchNotes::from_micro(4),
        expected_version: 0,
        fingerprint: 44,
    };
    assert_eq!(
        notes.debit(request.clone()).unwrap(),
        CurrencyCommitOutcome::Committed
    );
    assert_eq!(
        notes.debit(request.clone()).unwrap(),
        CurrencyCommitOutcome::AlreadyCommitted
    );
    let before = notes.clone();
    assert_eq!(
        notes.debit(ResearchNotesSpendRequest {
            fingerprint: 45,
            ..request
        }),
        Err(ProgressionError::IdempotencyConflict)
    );
    assert_eq!(notes, before);
    assert_eq!(
        ResearchNotes::from_micro(u64::MAX).checked_add(ResearchNotes::ONE),
        Err(ProgressionError::ArithmeticOverflow)
    );
    assert_eq!(
        VoidInsight::from_micro(u64::MAX).checked_add(VoidInsight::ONE),
        Err(ProgressionError::ArithmeticOverflow)
    );
}

#[test]
fn bounded_terminal_drain_reopens_capacity_without_losing_conservation() {
    let mut state = ProgressionAuthority::new(colony("drain")).unwrap();
    let mut ids = Vec::new();
    for index in 0..MAX_ACTIVE_SCHOLAR_WORK {
        ids.push(
            state
                .queue_notes_work(
                    state.partition.clone(),
                    assignment(&format!("scholar_{index}")),
                    1,
                    state.version,
                )
                .unwrap(),
        );
    }
    assert_eq!(
        state.queue_notes_work(
            state.partition.clone(),
            assignment("full"),
            1,
            state.version,
        ),
        Err(ProgressionError::Backpressure)
    );
    for id in &ids {
        state
            .recover_work(id, WorkRecoveryReason::RouteLost, state.version)
            .unwrap();
    }
    while !state.work_orders.is_empty() {
        state.drain_terminal_work(MAX_DRAIN_BATCH).unwrap();
    }
    assert_eq!(
        state.retired_work_through,
        u64::try_from(MAX_ACTIVE_SCHOLAR_WORK).unwrap()
    );
    state
        .queue_notes_work(
            state.partition.clone(),
            assignment("after_drain"),
            1,
            state.version,
        )
        .expect("drained authority keeps operating");
}

#[test]
fn strict_future_unknown_malformed_and_bounds_fail_closed() {
    let state = ProgressionAuthority::new(colony("serde")).unwrap();
    let mut future = serde_json::to_value(&state).unwrap();
    future["schemaVersion"] = serde_json::json!(99);
    assert!(serde_json::from_value::<ProgressionAuthority>(future).is_err());

    let mut unknown = serde_json::to_value(&state).unwrap();
    unknown["surprise"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ProgressionAuthority>(unknown).is_err());

    let mut malformed = serde_json::to_value(&state).unwrap();
    malformed["nextWorkSequence"] = serde_json::json!(0);
    assert!(serde_json::from_value::<ProgressionAuthority>(malformed).is_err());

    let mut overflow = state;
    overflow.next_work_sequence = u64::MAX;
    assert_eq!(
        overflow.queue_notes_work(
            overflow.partition.clone(),
            assignment("overflow"),
            1,
            overflow.version,
        ),
        Err(ProgressionError::ArithmeticOverflow)
    );
}

#[test]
fn owned_sources_contain_no_retired_currency_vocabulary_or_historical_totals() {
    let sources = [
        include_str!("../src/progression_research.rs"),
        include_str!("../src/divine_boosts.rs"),
        include_str!("lai44_progression_research.rs"),
        include_str!("divine_boosts.rs"),
    ];
    let retired = [
        ["fa", "vor"].concat(),
        ["bless", "ings"].concat(),
        ["generic research ", "points"].concat(),
        ["scholar in", "sight"].concat(),
    ];
    for source in sources {
        let lower = source.to_ascii_lowercase();
        for word in &retired {
            assert!(!lower.contains(word), "{word}");
        }
        assert!(!lower.contains(&["531", "/556"].concat()));
        assert!(!lower.contains(&["531", "_study"].concat()));
    }
}
