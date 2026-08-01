//! LAI.58 focused contract tests for the independent God and Leader research lanes.

use std::collections::{BTreeMap, BTreeSet};

use cat_sim::{
    acquired_traits::AcquiredTraitState,
    favor::Favor,
    planner_core::{BasisPoints, PlannerId},
    research_manifest::{
        CURATED_CONVERGENCE_JUNCTION_IDS, GLOBAL_MODIFIER_FINITE_LEVELS,
        GLOBAL_MODIFIER_TERMINAL_LEVEL, GLOBAL_MODIFIER_TRACK_IDS,
        HISTORICAL_SOURCE_MANIFEST_STUDY_COUNT, MINIMUM_AND_JUNCTIONS, RESEARCH_GRAPH_ALLOWS_ZOOM,
        RESEARCH_GRAPH_DRAG_PANNING, RESEARCH_GRAPH_REGION_OWNS_SCROLL, research_manifest,
    },
    research_purchase::{
        GodResearchCurrency, GodResearchFundOutcome, GodResearchLaborOutcome,
        GodResearchStudyTerms, GodResearchTerms, GodResearchWorkAuthorization,
        LeaderDuplicateAuthorization, LeaderResearchCandidate, LeaderResearchDecisionInputs,
        LeaderResearchEventKind, LeaderResearchLaneState, LeaderResearchRequest, ResearchFunds,
        ResearchPurchaseId, ResearchPurchaseState, StudyId, SyntheticResearchCatalog,
        SyntheticStudyDescriptor,
    },
    scholar_research::{
        BeginPhysicalPreparationRequest, FundPreparedGodResearchRequest, PreparationId, ScholarId,
        ScholarResearchState, ScholarWorkAuthorization, ScholarWorkEventId, ScholarWorkModifiers,
        ScholarWorkRequest,
    },
};

fn colony() -> PlannerId {
    PlannerId::derive("lai58_colony", ["one"])
}

fn study(id: &str, prerequisites: &[&str]) -> SyntheticStudyDescriptor {
    // The synthetic descriptor keeps the legacy fixed-point wrapper for API
    // compatibility; every authoritative assertion below uses Notes/Void or
    // the free Leader lane.
    SyntheticStudyDescriptor {
        id: StudyId::derive(id),
        display_name: id.to_owned(),
        prerequisites: prerequisites.iter().map(|id| StudyId::derive(id)).collect(),
        undiscounted_price: Favor::from_whole(1).expect("one favor fits"),
        tags: BTreeSet::new(),
    }
}

fn catalog() -> SyntheticResearchCatalog {
    SyntheticResearchCatalog::new(vec![
        study("root", &[]),
        study("branch", &["root"]),
        study("target", &["branch"]),
        study("independent", &[]),
        study("repeatable", &[]),
    ])
    .with_repeatable_studies(BTreeSet::from([StudyId::derive("repeatable")]))
}

fn terms() -> GodResearchTerms {
    GodResearchTerms {
        by_study: BTreeMap::from([
            (
                StudyId::derive("root"),
                GodResearchStudyTerms {
                    currency: GodResearchCurrency::Notes,
                    price: 2,
                    duration_game_minutes: 8,
                },
            ),
            (
                StudyId::derive("branch"),
                GodResearchStudyTerms {
                    currency: GodResearchCurrency::VoidInsight,
                    price: 3,
                    duration_game_minutes: 12,
                },
            ),
            (
                StudyId::derive("target"),
                GodResearchStudyTerms {
                    currency: GodResearchCurrency::Notes,
                    price: 5,
                    duration_game_minutes: 20,
                },
            ),
            (
                StudyId::derive("independent"),
                GodResearchStudyTerms {
                    currency: GodResearchCurrency::Notes,
                    price: 7,
                    duration_game_minutes: 4,
                },
            ),
            (
                StudyId::derive("repeatable"),
                GodResearchStudyTerms {
                    currency: GodResearchCurrency::Notes,
                    price: 11,
                    duration_game_minutes: 4,
                },
            ),
        ]),
    }
}

fn registered_scholar() -> (ScholarResearchState, ScholarId) {
    let scholar_id = ScholarId::derive("physical-scholar");
    let mut scholars = ScholarResearchState::new();
    scholars
        .record_completed_study_work(
            &mut AcquiredTraitState::default(),
            ScholarWorkRequest {
                id: ScholarWorkEventId::derive(&colony(), "register-physical-scholar"),
                scholar_id: scholar_id.clone(),
                completed_minutes: 1,
                expected_version: 0,
                authorization: ScholarWorkAuthorization {
                    scholars_guild_owned: true,
                    completed_research_station: true,
                    scholar_alive: true,
                },
                modifiers: ScholarWorkModifiers {
                    research_skill: BasisPoints::new(10_000),
                    scholarship: BasisPoints::new(10_000),
                },
                completed_tick: 0,
            },
        )
        .expect("register staffed scholar");
    (scholars, scholar_id)
}

#[test]
fn lai58_manifest_is_derived_clean_complete_and_has_explicit_terminals() {
    let manifest = research_manifest();
    assert!(!RESEARCH_GRAPH_ALLOWS_ZOOM);
    assert!(RESEARCH_GRAPH_DRAG_PANNING);
    assert!(RESEARCH_GRAPH_REGION_OWNS_SCROLL);
    assert_ne!(
        manifest.study_count(),
        HISTORICAL_SOURCE_MANIFEST_STUDY_COUNT
    );
    let totals = manifest
        .validate_lai58_graph()
        .expect("canonical LAI.58 graph");
    assert_eq!(totals.projected_node_count, manifest.study_count());
    assert!(totals.and_junction_count >= MINIMUM_AND_JUNCTIONS);
    assert_eq!(
        totals.curated_junction_count,
        CURATED_CONVERGENCE_JUNCTION_IDS.len()
    );
    for forbidden in ["shrine", "favor", "blessing", "coin", "food_storage"] {
        assert!(
            manifest
                .studies()
                .iter()
                .all(|study| !study.stable_id.contains(forbidden)),
            "{forbidden}"
        );
    }
    for required in [
        "typed_food_handling",
        "hunting_lairs",
        "universal_quality",
        "dragon_heart_processing",
        "family_homes",
        "elder_lodge",
        "nursery",
        "three_stage_construction",
        "rack_containers",
        "material_barter",
        "black_hole_darkness",
    ] {
        assert!(manifest.get(required).is_some(), "{required}");
    }
    assert_eq!(
        manifest.building_permit_ids("family_homes"),
        BTreeSet::from(["family_home"])
    );
    for track_id in GLOBAL_MODIFIER_TRACK_IDS {
        let studies = manifest.global_modifier_track_studies(track_id);
        assert_eq!(studies.len(), usize::from(GLOBAL_MODIFIER_TERMINAL_LEVEL));
        assert!(
            studies
                .iter()
                .take(usize::from(GLOBAL_MODIFIER_FINITE_LEVELS))
                .all(|study| !study.repeatable_terminal)
        );
        let terminal = studies.last().expect("terminal");
        assert!(terminal.repeatable_terminal);
        assert_eq!(
            terminal.cost_units,
            studies[usize::from(GLOBAL_MODIFIER_FINITE_LEVELS) - 1].cost_units * 2
        );
        assert_eq!(
            manifest
                .repeat_cost_units(&terminal.stable_id, 2)
                .expect("repeat cost"),
            terminal.cost_units * 4
        );
    }
}

#[test]
fn lai58_god_queue_is_topological_front_frozen_and_overtake_refunds_with_lost_labor() {
    let catalog = catalog();
    let mut state = ResearchPurchaseState::new();
    let mut funds = ResearchFunds {
        notes: 20,
        void_insight: 20,
    };

    assert_eq!(
        state
            .queue_god_target(&catalog, &terms(), StudyId::derive("target"))
            .expect("topological path"),
        vec![
            StudyId::derive("root"),
            StudyId::derive("branch"),
            StudyId::derive("target"),
        ]
    );
    assert_eq!(state.god_queue.entries().len(), 3);
    assert_eq!(
        state.fund_god_front(&mut funds).expect("fund root"),
        GodResearchFundOutcome::Funded
    );
    assert_eq!(funds.notes, 18);
    assert!(state.god_queue.entries()[0].frozen);
    assert!(!state.god_queue.entries()[1].frozen);
    assert_eq!(
        state
            .record_god_research_labor(
                &catalog,
                GodResearchWorkAuthorization {
                    completed_research_station: true,
                    staffed_scholar_alive: true,
                },
                3,
            )
            .expect("durable root labor"),
        GodResearchLaborOutcome::Advanced {
            remaining_labor_minutes: 5
        }
    );
    assert!(
        state
            .reorder_god_target(&catalog, &StudyId::derive("target"), 0)
            .is_err()
    );
    state
        .queue_god_target(&catalog, &terms(), StudyId::derive("independent"))
        .expect("append independent");
    state
        .reorder_god_target(&catalog, &StudyId::derive("independent"), 1)
        .expect("frozen front still permits safe tail reorder");

    let removal = state
        .remove_god_target(&catalog, &mut funds, &StudyId::derive("root"))
        .expect("root removal cascades");
    assert_eq!(removal.removed_studies.len(), 3);
    assert_eq!(removal.lost_labor_minutes, 3);
    assert_eq!(funds.notes, 20);
    assert_eq!(
        state.god_queue.entries()[0].study_id,
        StudyId::derive("independent")
    );
}

#[test]
fn lai58_leader_lane_is_free_finite_first_and_uses_exact_cadence_and_oopsie_bands() {
    assert_eq!(
        (0..=5)
            .map(LeaderResearchLaneState::quota_limit)
            .collect::<Vec<_>>(),
        vec![1, 1, 2, 2, 3, 4]
    );
    assert_eq!(
        (0..=4)
            .map(LeaderDuplicateAuthorization::oopsie_percent)
            .collect::<Vec<_>>(),
        vec![25, 12, 5, 1, 0]
    );

    let catalog = catalog();
    let mut state = ResearchPurchaseState::new();
    let mut funds = ResearchFunds {
        notes: 20,
        void_insight: 20,
    };
    let finite = StudyId::derive("independent");
    let selection = state
        .select_leader_target(
            &catalog,
            &[
                LeaderResearchCandidate {
                    study_id: StudyId::derive("repeatable"),
                    decision_inputs: LeaderResearchDecisionInputs {
                        need_score: 9_999,
                        ..LeaderResearchDecisionInputs::default()
                    },
                    repeatable: true,
                },
                LeaderResearchCandidate {
                    study_id: finite.clone(),
                    decision_inputs: LeaderResearchDecisionInputs {
                        report_score: 1,
                        ..LeaderResearchDecisionInputs::default()
                    },
                    repeatable: false,
                },
            ],
            LeaderDuplicateAuthorization::None,
        )
        .expect("selection");
    assert_eq!(selection, Some(finite.clone()));
    state
        .complete_leader_research(
            &catalog,
            &mut funds,
            LeaderResearchRequest {
                id: ResearchPurchaseId::derive("lai58", &colony(), "free"),
                study_id: finite,
                expected_research_version: state.version,
                effective_loremaster_level: 0,
                now_tick: 0,
                duplicate_authorization: LeaderDuplicateAuthorization::None,
            },
        )
        .expect("free instant leader research");
    assert!(
        state
            .complete_leader_research(
                &catalog,
                &mut funds,
                LeaderResearchRequest {
                    id: ResearchPurchaseId::derive("lai58", &colony(), "too-soon"),
                    study_id: StudyId::derive("root"),
                    expected_research_version: state.version,
                    effective_loremaster_level: 0,
                    now_tick: 7 * 24 * 60 - 1,
                    duplicate_authorization: LeaderDuplicateAuthorization::None,
                },
            )
            .is_err()
    );
    state
        .complete_leader_research(
            &catalog,
            &mut funds,
            LeaderResearchRequest {
                id: ResearchPurchaseId::derive("lai58", &colony(), "next-window"),
                study_id: StudyId::derive("root"),
                expected_research_version: state.version,
                effective_loremaster_level: 0,
                now_tick: 7 * 24 * 60,
                duplicate_authorization: LeaderDuplicateAuthorization::None,
            },
        )
        .expect("rolling seven-day window releases exactly at the boundary");
    assert_eq!(funds.notes, 20);
    assert_eq!(funds.void_insight, 20);
}

#[test]
fn lai58_god_work_needs_physical_staff_and_duplicate_events_are_distinct() {
    let catalog = catalog();
    let mut state = ResearchPurchaseState::new();
    let mut funds = ResearchFunds {
        notes: 20,
        void_insight: 20,
    };
    state
        .queue_god_target(&catalog, &terms(), StudyId::derive("independent"))
        .expect("queue independent");
    state.fund_god_front(&mut funds).expect("fund independent");
    assert!(
        state
            .record_god_research_labor(
                &catalog,
                GodResearchWorkAuthorization {
                    completed_research_station: false,
                    staffed_scholar_alive: true,
                },
                1,
            )
            .is_err()
    );
    let completion = state
        .complete_leader_research(
            &catalog,
            &mut funds,
            LeaderResearchRequest {
                id: ResearchPurchaseId::derive("lai58", &colony(), "emergency-overtake"),
                study_id: StudyId::derive("independent"),
                expected_research_version: state.version,
                effective_loremaster_level: 5,
                now_tick: 0,
                duplicate_authorization: LeaderDuplicateAuthorization::Emergency {
                    report_indicates_urgent_need: true,
                    needed_before_tick: 5,
                    estimated_god_completion_tick: 10,
                },
            },
        )
        .expect("explicit emergency may overtake");
    assert_eq!(
        completion.event_kind,
        LeaderResearchEventKind::IntentionalEmergencyOverride
    );
    assert_eq!(
        LeaderResearchEventKind::from(LeaderDuplicateAuthorization::Oopsie {
            effective_expertise_intelligence_level: 3,
            keyed_roll_percent: 0,
        }),
        LeaderResearchEventKind::AccidentalDuplicateOopsie
    );
    assert_eq!(funds.notes, 20);
}

#[test]
fn lai58_preparation_is_staffed_nonstacking_labor_and_only_discounts_player_god_front() {
    let catalog = catalog();
    let mut progress = ResearchPurchaseState::new();
    let mut funds = ResearchFunds {
        notes: 20,
        void_insight: 20,
    };
    progress
        .queue_god_target(&catalog, &terms(), StudyId::derive("independent"))
        .expect("queue player target");

    let (mut scholars, scholar_id) = registered_scholar();
    let preparation_id = PreparationId::derive(&colony(), "prepare-independent");
    let front = progress.god_queue.entries()[0].clone();
    scholars
        .begin_physical_preparation(
            &catalog,
            &progress,
            &front,
            BeginPhysicalPreparationRequest {
                id: preparation_id,
                study_id: StudyId::derive("independent"),
                assigned_scholar: scholar_id,
                authorization: ScholarWorkAuthorization {
                    scholars_guild_owned: true,
                    completed_research_station: true,
                    scholar_alive: true,
                },
                expected_version: scholars.version,
                prepared_tick: 1,
            },
        )
        .expect("begin physical preparation");
    assert_eq!(
        scholars
            .prepared_study(&StudyId::derive("independent"))
            .expect("preparation")
            .required_labor_minutes,
        1
    );
    assert!(
        scholars
            .begin_physical_preparation(
                &catalog,
                &progress,
                &front,
                BeginPhysicalPreparationRequest {
                    id: PreparationId::derive(&colony(), "cannot-stack"),
                    study_id: StudyId::derive("independent"),
                    assigned_scholar: ScholarId::derive("physical-scholar"),
                    authorization: ScholarWorkAuthorization {
                        scholars_guild_owned: true,
                        completed_research_station: true,
                        scholar_alive: true,
                    },
                    expected_version: scholars.version,
                    prepared_tick: 2,
                },
            )
            .is_err()
    );
    scholars
        .record_physical_preparation_labor(&StudyId::derive("independent"), 1, scholars.version)
        .expect("complete physical preparation");
    let expected_research_version = progress.version;
    let expected_scholar_version = scholars.version;
    scholars
        .fund_prepared_god_front(
            &mut progress,
            &mut funds,
            FundPreparedGodResearchRequest {
                id: ResearchPurchaseId::derive("lai58", &colony(), "fund-prepared"),
                study_id: StudyId::derive("independent"),
                expected_research_version,
                expected_scholar_version,
            },
        )
        .expect("consume preparation on player God funding");
    assert_eq!(funds.notes, 14);
    assert!(
        scholars
            .prepared_study(&StudyId::derive("independent"))
            .is_none()
    );
}

#[test]
fn lai58_leader_overtake_refunds_currency_and_loses_both_labor_kinds() {
    let catalog = catalog();
    let study_id = StudyId::derive("independent");
    let mut progress = ResearchPurchaseState::new();
    let mut funds = ResearchFunds {
        notes: 20,
        void_insight: 0,
    };
    progress
        .queue_god_target(&catalog, &terms(), study_id.clone())
        .expect("queue target");
    let (mut scholars, scholar_id) = registered_scholar();
    let front = progress.god_queue.entries()[0].clone();
    scholars
        .begin_physical_preparation(
            &catalog,
            &progress,
            &front,
            BeginPhysicalPreparationRequest {
                id: PreparationId::derive(&colony(), "overtaken-preparation"),
                study_id: study_id.clone(),
                assigned_scholar: scholar_id,
                authorization: ScholarWorkAuthorization {
                    scholars_guild_owned: true,
                    completed_research_station: true,
                    scholar_alive: true,
                },
                expected_version: scholars.version,
                prepared_tick: 1,
            },
        )
        .expect("begin preparation");
    scholars
        .record_physical_preparation_labor(&study_id, 1, scholars.version)
        .expect("preparation labor");
    progress.fund_god_front(&mut funds).expect("fund God front");
    progress
        .record_god_research_labor(
            &catalog,
            GodResearchWorkAuthorization {
                completed_research_station: true,
                staffed_scholar_alive: true,
            },
            1,
        )
        .expect("research labor");
    let expected_research_version = progress.version;
    let completion = scholars
        .complete_leader_research(
            &mut progress,
            &catalog,
            &mut funds,
            LeaderResearchRequest {
                id: ResearchPurchaseId::derive("lai58", &colony(), "overtake-with-preparation"),
                study_id,
                expected_research_version,
                effective_loremaster_level: 5,
                now_tick: 2,
                duplicate_authorization: LeaderDuplicateAuthorization::Emergency {
                    report_indicates_urgent_need: true,
                    needed_before_tick: 3,
                    estimated_god_completion_tick: 8,
                },
            },
        )
        .expect("Leader overtake");
    assert_eq!(completion.lost_preparation_labor_minutes, 1);
    assert_eq!(
        completion
            .completion
            .overtake
            .expect("God target removed")
            .lost_labor_minutes,
        1
    );
    assert_eq!(funds.notes, 20);
}

#[test]
fn lai58_infinite_terminal_remains_unowned_and_doubles_each_repeat_cost() {
    let catalog = catalog();
    let repeatable = StudyId::derive("repeatable");
    let mut state = ResearchPurchaseState::new();
    let mut funds = ResearchFunds {
        notes: 100,
        void_insight: 0,
    };
    let authorization = GodResearchWorkAuthorization {
        completed_research_station: true,
        staffed_scholar_alive: true,
    };
    state
        .queue_god_target(&catalog, &terms(), repeatable.clone())
        .expect("first repeat");
    state.fund_god_front(&mut funds).expect("fund first repeat");
    state
        .record_god_research_labor(&catalog, authorization, 4)
        .expect("complete first repeat");
    assert!(!state.owned_studies.contains(&repeatable));
    assert_eq!(state.repeatable_completions[&repeatable], 1);
    state
        .queue_god_target(&catalog, &terms(), repeatable.clone())
        .expect("second repeat");
    assert_eq!(state.god_queue.entries()[0].frozen_price, 22);
    state
        .fund_god_front(&mut funds)
        .expect("fund doubled repeat");
    assert_eq!(funds.notes, 67);
}
