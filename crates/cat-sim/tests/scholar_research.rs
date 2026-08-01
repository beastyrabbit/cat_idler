//! LAI.19 scholar, Insight, preparation, purchase, and track-effect tests.

use std::collections::BTreeSet;

use cat_sim::{
    acquired_traits::{AcquiredTrait, AcquiredTraitState},
    divine_boosts::DivineBoostResearchStages,
    favor::{Favor, FavorEventId, FavorEventKind, FavorLedger},
    planner_core::{BasisPoints, PlannerId},
    research_purchase::{
        PlayerResearchPurchaseRequest, ResearchPurchaseError, ResearchPurchaseId,
        ResearchPurchaseOutcome, ResearchPurchaseState, StudyId, SyntheticResearchCatalog,
        SyntheticStudyDescriptor,
    },
    scholar_research::{
        GAME_MINUTES_PER_WEEK, INSIGHT_PER_COMPLETED_WEEK, Insight, PreparationId,
        PreparationOutcome, PrepareStudyRequest, ResearchTrackStages, ScholarId,
        ScholarPlayerPurchaseRequest, ScholarResearchError, ScholarResearchState,
        ScholarWorkAuthorization, ScholarWorkEventId, ScholarWorkModifiers, ScholarWorkOutcome,
        ScholarWorkRequest,
    },
};

fn colony() -> PlannerId {
    PlannerId::derive("test_colony", ["scholars"])
}

#[test]
fn scholar_week_metadata_is_explicitly_empty_before_work() {
    let state = ScholarResearchState::new();
    assert_eq!(state.insight_week_started_tick, None);
    assert_eq!(state.generated_this_week, Insight::ZERO);
}

fn scholar(id: &str) -> ScholarId {
    ScholarId::derive(id)
}

fn study(id: &str, price: u64, prerequisites: &[&str]) -> SyntheticStudyDescriptor {
    SyntheticStudyDescriptor {
        id: StudyId::derive(id),
        display_name: format!("Study {id}"),
        prerequisites: prerequisites.iter().map(|id| StudyId::derive(id)).collect(),
        undiscounted_price: Favor::from_whole(price).unwrap(),
        tags: BTreeSet::new(),
    }
}

fn catalog() -> SyntheticResearchCatalog {
    SyntheticResearchCatalog::new(vec![
        study("root", 40, &[]),
        study("dependency", 80, &["root"]),
        study("speculative", 60, &[]),
    ])
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

fn work_request(
    action: &str,
    scholar_id: ScholarId,
    completed_minutes: u64,
    expected_version: u64,
    modifiers: ScholarWorkModifiers,
) -> ScholarWorkRequest {
    ScholarWorkRequest {
        id: ScholarWorkEventId::derive(&colony(), action),
        scholar_id,
        completed_minutes,
        expected_version,
        authorization: ScholarWorkAuthorization {
            scholars_guild_owned: true,
            completed_research_station: true,
            scholar_alive: true,
        },
        modifiers,
        completed_tick: 100,
    }
}

fn neutral_modifiers() -> ScholarWorkModifiers {
    ScholarWorkModifiers {
        research_skill: BasisPoints::new(10_000),
        scholarship: BasisPoints::new(10_000),
    }
}

#[test]
fn completed_physical_weeks_generate_exact_insight_and_partial_work_persists() {
    let mut state = ScholarResearchState::new();
    let mut traits = AcquiredTraitState::default();
    let scholar_id = scholar("marble");

    let partial = work_request(
        "partial",
        scholar_id.clone(),
        GAME_MINUTES_PER_WEEK - 1,
        0,
        neutral_modifiers(),
    );
    let outcome = state
        .record_completed_study_work(&mut traits, partial)
        .unwrap();
    assert_eq!(outcome, ScholarWorkOutcome::Recorded);
    assert_eq!(state.insight_balance, Insight::ZERO);
    assert_eq!(
        state.scholar(&scholar_id).unwrap().partial_week_minutes,
        GAME_MINUTES_PER_WEEK - 1
    );

    let completion = work_request("completion", scholar_id.clone(), 1, 1, neutral_modifiers());
    state
        .record_completed_study_work(&mut traits, completion.clone())
        .unwrap();
    assert_eq!(
        state.insight_balance,
        Insight::from_whole(INSIGHT_PER_COMPLETED_WEEK).unwrap()
    );
    assert_eq!(
        state
            .record_completed_study_work(&mut traits, completion)
            .unwrap(),
        ScholarWorkOutcome::AlreadyRecorded
    );
    assert_eq!(
        state.insight_balance,
        Insight::from_whole(INSIGHT_PER_COMPLETED_WEEK).unwrap()
    );
}

#[test]
fn skill_scholarship_and_seasoned_scholar_are_the_only_production_modifiers() {
    let mut state = ScholarResearchState::new();
    let mut traits = AcquiredTraitState::default();
    let scholar_id = scholar("juniper");
    let modifiers = ScholarWorkModifiers {
        research_skill: BasisPoints::new(12_500),
        scholarship: BasisPoints::new(20_000),
    };

    state
        .record_completed_study_work(
            &mut traits,
            work_request(
                "ten-weeks",
                scholar_id.clone(),
                10 * GAME_MINUTES_PER_WEEK,
                0,
                neutral_modifiers(),
            ),
        )
        .unwrap();
    assert_eq!(state.insight_balance, Insight::from_whole(200).unwrap());
    assert!(traits.traits.contains(AcquiredTrait::SeasonedScholar));

    state
        .record_completed_study_work(
            &mut traits,
            work_request(
                "seasoned-modified",
                scholar_id,
                GAME_MINUTES_PER_WEEK,
                1,
                modifiers,
            ),
        )
        .unwrap();
    assert_eq!(
        state.insight_balance,
        Insight::from_whole(255).unwrap(),
        "20 * 1.25 skill * 2.0 Scholarship * 1.10 Seasoned Scholar"
    );
}

#[test]
fn work_requires_the_scholar_unlock_a_physical_station_and_a_living_cat() {
    let denied = [
        ScholarWorkAuthorization {
            scholars_guild_owned: false,
            completed_research_station: true,
            scholar_alive: true,
        },
        ScholarWorkAuthorization {
            scholars_guild_owned: true,
            completed_research_station: false,
            scholar_alive: true,
        },
        ScholarWorkAuthorization {
            scholars_guild_owned: true,
            completed_research_station: true,
            scholar_alive: false,
        },
    ];
    for (index, authorization) in denied.into_iter().enumerate() {
        let mut state = ScholarResearchState::new();
        let mut traits = AcquiredTraitState::default();
        let mut request = work_request(
            &format!("denied-{index}"),
            scholar("locked"),
            GAME_MINUTES_PER_WEEK,
            0,
            neutral_modifiers(),
        );
        request.authorization = authorization;
        assert_eq!(
            state.record_completed_study_work(&mut traits, request),
            Err(ScholarResearchError::ScholarWorkLocked)
        );
        assert_eq!(state, ScholarResearchState::new());
    }
}

#[test]
fn preparation_costs_current_undiscounted_price_and_prioritizes_plan_dependencies() {
    let catalog = catalog();
    let mut state = ScholarResearchState::new();
    let mut traits = AcquiredTraitState::default();
    let scholar_id = scholar("maple");
    state
        .record_completed_study_work(
            &mut traits,
            work_request(
                "fund-preparation",
                scholar_id.clone(),
                5 * GAME_MINUTES_PER_WEEK,
                0,
                neutral_modifiers(),
            ),
        )
        .unwrap();
    let progress = ResearchPurchaseState::new();
    let selected = state
        .select_preparation_target(
            &catalog,
            &progress,
            &BTreeSet::from([StudyId::derive("dependency")]),
        )
        .unwrap();
    assert_eq!(selected, StudyId::derive("dependency"));

    let request = PrepareStudyRequest {
        id: PreparationId::derive(&colony(), "prepare-dependency"),
        study_id: selected.clone(),
        assigned_scholar: scholar_id,
        expected_version: state.version,
        prepared_tick: 200,
    };
    assert_eq!(
        state
            .prepare_study(&catalog, &progress, request.clone())
            .unwrap(),
        PreparationOutcome::Prepared
    );
    assert_eq!(state.insight_balance, Insight::from_whole(20).unwrap());
    assert_eq!(
        state.prepared_study(&selected).unwrap().insight_cost,
        Insight::from_whole(80).unwrap()
    );
    assert_eq!(
        state.prepare_study(&catalog, &progress, request).unwrap(),
        PreparationOutcome::AlreadyPrepared
    );

    let restart: ScholarResearchState =
        serde_json::from_str(&serde_json::to_string(&state).unwrap()).unwrap();
    assert_eq!(restart, state);
    assert!(restart.prepared_study(&selected).is_some());
}

#[test]
fn scholar_death_preserves_colony_insight_and_preparation_for_reassignment() {
    let catalog = catalog();
    let progress = ResearchPurchaseState::new();
    let mut state = ScholarResearchState::new();
    let mut old_traits = AcquiredTraitState::default();
    let old = scholar("old");
    state
        .record_completed_study_work(
            &mut old_traits,
            work_request(
                "old-work",
                old.clone(),
                4 * GAME_MINUTES_PER_WEEK,
                0,
                neutral_modifiers(),
            ),
        )
        .unwrap();
    state
        .prepare_study(
            &catalog,
            &progress,
            PrepareStudyRequest {
                id: PreparationId::derive(&colony(), "prepare-root"),
                study_id: StudyId::derive("root"),
                assigned_scholar: old.clone(),
                expected_version: 1,
                prepared_tick: 10,
            },
        )
        .unwrap();
    let balance_before_death = state.insight_balance;
    assert_eq!(state.record_scholar_death(&old).unwrap(), 1);
    assert_eq!(state.insight_balance, balance_before_death);
    assert_eq!(
        state
            .prepared_study(&StudyId::derive("root"))
            .unwrap()
            .assigned_scholar,
        None
    );

    let successor = scholar("successor");
    let mut successor_traits = AcquiredTraitState::default();
    state
        .record_completed_study_work(
            &mut successor_traits,
            work_request(
                "successor-work",
                successor.clone(),
                1,
                state.version,
                neutral_modifiers(),
            ),
        )
        .unwrap();
    state
        .reassign_preparation(&StudyId::derive("root"), successor.clone(), state.version)
        .unwrap();
    assert_eq!(
        state
            .prepared_study(&StudyId::derive("root"))
            .unwrap()
            .assigned_scholar,
        Some(successor)
    );
}

#[test]
fn prepared_player_purchase_is_atomic_discounted_and_consumed_exactly_once() {
    let catalog = catalog();
    let mut progress = ResearchPurchaseState::new();
    let mut ledger = funded_ledger(100);
    let mut state = ScholarResearchState::new();
    let mut traits = AcquiredTraitState::default();
    let scholar_id = scholar("cedar");
    state
        .record_completed_study_work(
            &mut traits,
            work_request(
                "prepare-funds",
                scholar_id.clone(),
                2 * GAME_MINUTES_PER_WEEK,
                0,
                neutral_modifiers(),
            ),
        )
        .unwrap();
    state
        .prepare_study(
            &catalog,
            &progress,
            PrepareStudyRequest {
                id: PreparationId::derive(&colony(), "prepare-root"),
                study_id: StudyId::derive("root"),
                assigned_scholar: scholar_id,
                expected_version: 1,
                prepared_tick: 20,
            },
        )
        .unwrap();
    let request = ScholarPlayerPurchaseRequest {
        id: ResearchPurchaseId::derive("test", &colony(), "buy-root"),
        colony_id: colony(),
        study_id: StudyId::derive("root"),
        expected_research_version: 0,
        expected_favor_version: ledger.version,
        expected_scholar_version: state.version,
        use_preparation: true,
        now_tick: 30,
    };
    assert_eq!(
        state
            .player_purchase(&mut progress, &mut ledger, &catalog, request.clone())
            .unwrap(),
        ResearchPurchaseOutcome::Committed
    );
    assert_eq!(ledger.balance, Favor::from_whole(70).unwrap());
    assert!(state.prepared_study(&StudyId::derive("root")).is_none());
    let event = progress.purchases.get(&request.id).unwrap();
    assert_eq!(event.discount_basis_points, 2_500);
    assert!(event.consumed_preparation);

    assert_eq!(
        state
            .player_purchase(&mut progress, &mut ledger, &catalog, request)
            .unwrap(),
        ResearchPurchaseOutcome::AlreadyCommitted
    );
    assert_eq!(ledger.balance, Favor::from_whole(70).unwrap());

    let before = (state.clone(), progress.clone(), ledger.clone());
    let missing = ScholarPlayerPurchaseRequest {
        id: ResearchPurchaseId::derive("test", &colony(), "missing-preparation"),
        colony_id: colony(),
        study_id: StudyId::derive("speculative"),
        expected_research_version: progress.version,
        expected_favor_version: ledger.version,
        expected_scholar_version: state.version,
        use_preparation: true,
        now_tick: 40,
    };
    assert_eq!(
        state.player_purchase(&mut progress, &mut ledger, &catalog, missing),
        Err(ScholarResearchError::PreparationNotFound)
    );
    assert_eq!((state, progress, ledger), before);
}

#[test]
fn player_purchase_rejects_undocumented_discount_terms() {
    let catalog = catalog();
    let mut progress = ResearchPurchaseState::new();
    let mut ledger = funded_ledger(100);
    let request = PlayerResearchPurchaseRequest {
        id: ResearchPurchaseId::derive("test", &colony(), "invalid-discount"),
        colony_id: colony(),
        study_id: StudyId::derive("root"),
        expected_research_version: 0,
        expected_favor_version: ledger.version,
        discount_basis_points: 5_000,
        consume_preparation: true,
        now_tick: 1,
    };
    let before = (progress.clone(), ledger.clone());
    assert_eq!(
        progress.player_purchase(&mut ledger, &catalog, request),
        Err(ResearchPurchaseError::MalformedRequest)
    );
    assert_eq!((progress, ledger), before);
}

#[test]
fn all_track_stages_project_exact_runtime_effects_and_reject_gaps() {
    let mut progress = ResearchPurchaseState::new();
    for track in [
        "divine_duration",
        "divine_economy",
        "rehabilitation",
        "administration",
    ] {
        for stage in 1..=11 {
            progress
                .owned_studies
                .insert(StudyId::derive(&format!("{track}_stage_{stage:02}")));
        }
    }
    let stages = ResearchTrackStages::from_progress(&progress).unwrap();
    let effects = stages.effects();
    assert_eq!(
        effects.divine_boost_stages,
        DivineBoostResearchStages {
            divine_duration_stage: 11,
            divine_economy_stage: 11,
        }
    );
    assert_eq!(effects.max_divine_duration_game_hours, 24);
    assert_eq!(effects.divine_economy_discount_basis_points, 3_300);
    assert_eq!(effects.rehabilitation_bonus_basis_points, 2_200);
    assert_eq!(effects.standing_order_slots, 14);
    assert_eq!(effects.strategic_intent_slots, 9);

    progress
        .owned_studies
        .remove(&StudyId::derive("administration_stage_05"));
    assert_eq!(
        ResearchTrackStages::from_progress(&progress),
        Err(ScholarResearchError::NonContiguousResearchTrack)
    );
}

#[test]
fn malformed_persisted_scholar_state_is_rejected() {
    let mut state = ScholarResearchState::new();
    state.insight_balance = Insight::from_whole(1).unwrap();
    let mut value = serde_json::to_value(state).unwrap();
    value["schemaVersion"] = serde_json::json!(99);
    assert!(serde_json::from_value::<ScholarResearchState>(value).is_err());

    let mut zero_minutes = serde_json::json!({
        "schemaVersion": 1,
        "version": 1,
        "insightBalance": 0,
        "scholars": {},
        "preparations": {},
        "preparationEvents": {},
        "workEvents": {
            "planner:v1|23:scholar_work_event_id|36:planner:v1|12:test_colony|8:scholars|3:bad": {
                "id": "planner:v1|23:scholar_work_event_id|36:planner:v1|12:test_colony|8:scholars|3:bad",
                "scholarId": "planner:v1|7:scholar|3:bad",
                "completedMinutes": 0,
                "researchSkillBasisPoints": 10000,
                "scholarshipBasisPoints": 10000,
                "creditedInsight": 0,
                "committedVersion": 1,
                "completedTick": 1
            }
        },
        "consumedPreparations": {}
    });
    assert!(serde_json::from_value::<ScholarResearchState>(zero_minutes.take()).is_err());
}
