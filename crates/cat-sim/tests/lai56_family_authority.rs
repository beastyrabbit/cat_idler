use cat_sim::{
    family_authority::{
        BirthRegistration, EnterpriseRequest, FamilyAuthorityError, FamilyAuthorityState,
        FamilyBuilding, FamilyCommand, FamilyCommandResult, FamilyOperation,
        ProfessionalCompletion, TeachingAssignment,
    },
    family_housing::{HousingKind, LifeStage, TeachingSite},
    family_specialization::{
        BIRTH_SKILL_CAP_XP_CENTI, BirthSeedOutcome, ParentProfessionSeed, birth_seed_grant,
        keyed_birth_seed_outcome,
    },
};
use std::collections::BTreeMap;

fn command(
    receipt_id: impl Into<String>,
    expected_revision: u64,
    operation: FamilyOperation,
) -> FamilyCommand {
    FamilyCommand {
        receipt_id: receipt_id.into(),
        expected_revision,
        operation,
    }
}

fn apply(
    state: &mut FamilyAuthorityState,
    receipt_id: impl Into<String>,
    operation: FamilyOperation,
) -> FamilyCommandResult {
    let version = state.revision;
    state
        .apply(command(receipt_id, version, operation))
        .expect("valid family command")
        .result
}

fn birth(
    state: &mut FamilyAuthorityState,
    cat_id: &str,
    stage: LifeStage,
    first: Option<&str>,
    second: Option<&str>,
) {
    apply(
        state,
        format!("receipt_{cat_id}"),
        FamilyOperation::RegisterBirth(BirthRegistration {
            newborn_cat_id: cat_id.to_owned(),
            life_stage: stage,
            first_parent_id: first.map(str::to_owned),
            second_parent_id: second.map(str::to_owned),
            attribute_authority_ref: format!("attributes_{cat_id}"),
            relational_analytical_authority_ref: format!("axis_{cat_id}"),
        }),
    );
}

fn building(
    state: &mut FamilyAuthorityState,
    id: &str,
    housing_kind: Option<HousingKind>,
    teaching_site: Option<TeachingSite>,
) {
    apply(
        state,
        format!("receipt_{id}"),
        FamilyOperation::RegisterBuilding(FamilyBuilding {
            building_id: id.to_owned(),
            housing_kind,
            teaching_site,
            completed: true,
            level: 1,
        }),
    );
}

#[test]
fn birth_bands_transfer_cap_and_non_genetic_exclusions_are_authoritative() {
    let first = ParentProfessionSeed::new(
        "first",
        "lineage_first",
        "tradition_first",
        BTreeMap::from([("fishing".to_owned(), 2_000_000)]),
    );
    let second = ParentProfessionSeed::new(
        "second",
        "lineage_second",
        "tradition_second",
        BTreeMap::from([("fishing".to_owned(), 2_000_000)]),
    );
    let cases = [
        BirthSeedOutcome::FirstParent,
        BirthSeedOutcome::SecondParent,
        BirthSeedOutcome::Blend,
        BirthSeedOutcome::Both,
        BirthSeedOutcome::None,
    ];
    for expected in cases {
        let child = (0..20_000)
            .map(|number| format!("child{number}"))
            .find(|child| keyed_birth_seed_outcome(child, "first", "second") == expected)
            .expect("every exact band has a keyed representative");
        let grant = birth_seed_grant(
            keyed_birth_seed_outcome(&child, "first", "second"),
            &first,
            &second,
        );
        assert!(
            grant
                .inherited_skill_xp_centi
                .values()
                .all(|xp| *xp <= BIRTH_SKILL_CAP_XP_CENTI)
        );
        assert!(grant.inherited_acquired_trait_ids.is_empty());
    }
    assert_eq!(
        birth_seed_grant(BirthSeedOutcome::FirstParent, &first, &second).inherited_skill_xp_centi["fishing"],
        BIRTH_SKILL_CAP_XP_CENTI
    );
    assert_eq!(
        birth_seed_grant(BirthSeedOutcome::Blend, &first, &second).inherited_skill_xp_centi["fishing"],
        BIRTH_SKILL_CAP_XP_CENTI
    );

    let mut state = FamilyAuthorityState::empty("colony", 7);
    birth(&mut state, "first", LifeStage::Adult, None, None);
    birth(&mut state, "second", LifeStage::Adult, None, None);
    birth(
        &mut state,
        "child",
        LifeStage::Kitten,
        Some("first"),
        Some("second"),
    );
    let child = &state.cats["child"];
    assert_eq!(
        child.identity_reference.inherited_parent_ids,
        vec!["first", "second"]
    );
    assert!(child.identity_reference.acquired_trait_ids.is_empty());
    assert!(!child.identity_reference.inherited_office_clearance);
}

#[test]
fn autonomous_partnership_rejects_kin_and_housing_obeys_real_capacity() {
    let mut state = FamilyAuthorityState::empty("colony", 2);
    building(&mut state, "den", Some(HousingKind::Den), None);
    building(
        &mut state,
        "home",
        Some(HousingKind::FamilyHome),
        Some(TeachingSite::FamilyHome),
    );
    building(&mut state, "lodge", Some(HousingKind::ElderLodge), None);
    building(
        &mut state,
        "nursery",
        Some(HousingKind::Nursery),
        Some(TeachingSite::Nursery),
    );
    birth(&mut state, "parenta", LifeStage::Adult, None, None);
    birth(&mut state, "parentb", LifeStage::Adult, None, None);
    apply(
        &mut state,
        "receipt_reviewparents",
        FamilyOperation::ReviewAutonomousPartnerships,
    );
    birth(
        &mut state,
        "sibone",
        LifeStage::Adult,
        Some("parenta"),
        Some("parentb"),
    );
    birth(
        &mut state,
        "sibtwo",
        LifeStage::Adult,
        Some("parenta"),
        Some("parentb"),
    );
    let result = apply(
        &mut state,
        "receipt_reviewsiblings",
        FamilyOperation::ReviewAutonomousPartnerships,
    );
    let FamilyCommandResult::PartnershipsReviewed {
        formed_partnership_ids,
    } = result
    else {
        panic!("expected partnership review")
    };
    assert!(
        !formed_partnership_ids
            .iter()
            .any(|id| id.contains("sibone") || id.contains("sibtwo"))
    );
    birth(
        &mut state,
        "kitten",
        LifeStage::Kitten,
        Some("parenta"),
        Some("parentb"),
    );
    birth(&mut state, "elder", LifeStage::Elder, None, None);
    apply(
        &mut state,
        "receipt_housing",
        FamilyOperation::ReconcileHousing {
            pressure_requires_den_return: true,
        },
    );
    assert_eq!(state.residences["elder"], "lodge");
    assert_eq!(state.residences["parenta"], "home");
    assert_eq!(state.residences["parentb"], "home");
    assert_eq!(state.residences["kitten"], "home");
    assert!(
        !state
            .residences
            .values()
            .any(|building_id| building_id == "nursery")
    );
}

#[test]
fn teaching_is_due_after_three_real_tasks_defers_and_survives_restart() {
    let mut state = FamilyAuthorityState::empty("colony", 3);
    building(
        &mut state,
        "home",
        Some(HousingKind::FamilyHome),
        Some(TeachingSite::FamilyHome),
    );
    birth(&mut state, "parent", LifeStage::Adult, None, None);
    birth(&mut state, "other", LifeStage::Adult, None, None);
    apply(
        &mut state,
        "receipt_review",
        FamilyOperation::ReviewAutonomousPartnerships,
    );
    birth(
        &mut state,
        "child",
        LifeStage::Kitten,
        Some("parent"),
        Some("other"),
    );
    for index in 0..3 {
        apply(
            &mut state,
            format!("receipt_work{index}"),
            FamilyOperation::RecordProfessionalCompletion(ProfessionalCompletion {
                task_id: format!("task{index}"),
                cat_id: "parent".to_owned(),
                profession_id: "fishing".to_owned(),
                skill_id: "fishing".to_owned(),
                skill_xp_centi: 100,
                enterprise_id: None,
            }),
        );
    }
    let obligation_id = state
        .teaching_obligations
        .keys()
        .next()
        .cloned()
        .expect("three tasks create obligation");
    assert!(state.teaching_obligations[&obligation_id].due);
    apply(
        &mut state,
        "receipt_defer",
        FamilyOperation::DeferTeachingForEmergency {
            parent_cat_id: "parent".to_owned(),
            dependent_cat_id: "child".to_owned(),
        },
    );
    let restarted =
        FamilyAuthorityState::decode_json(&serde_json::to_string(&state).expect("serialize"))
            .expect("strict restart");
    assert!(restarted.teaching_obligations[&obligation_id].deferred_by_emergency);
    let blocked = restarted.clone().apply(command(
        "receipt_blocked",
        restarted.revision,
        FamilyOperation::CompleteTeaching(TeachingAssignment {
            obligation_id: obligation_id.clone(),
            site_building_id: "home".to_owned(),
            learner_skill_id: "fishing".to_owned(),
            productive_minutes: 60,
        }),
    ));
    assert!(matches!(blocked, Err(FamilyAuthorityError::TeachingNotDue)));
    let mut state = restarted;
    apply(
        &mut state,
        "receipt_resume",
        FamilyOperation::ResumeDeferredTeaching {
            parent_cat_id: "parent".to_owned(),
            dependent_cat_id: "child".to_owned(),
        },
    );
    let before_rejected_teaching = state.clone();
    let rejected = state.apply(command(
        "receipt_badsite",
        state.revision,
        FamilyOperation::CompleteTeaching(TeachingAssignment {
            obligation_id: obligation_id.clone(),
            site_building_id: "missing".to_owned(),
            learner_skill_id: "fishing".to_owned(),
            productive_minutes: 60,
        }),
    ));
    assert!(matches!(
        rejected,
        Err(FamilyAuthorityError::UnknownBuilding(_))
    ));
    assert_eq!(
        state, before_rejected_teaching,
        "rejected commands are atomic"
    );
    apply(
        &mut state,
        "receipt_teach",
        FamilyOperation::CompleteTeaching(TeachingAssignment {
            obligation_id: obligation_id.clone(),
            site_building_id: "home".to_owned(),
            learner_skill_id: "fishing".to_owned(),
            productive_minutes: 60,
        }),
    );
    assert!(!state.teaching_obligations[&obligation_id].due);
    assert!(state.cats["parent"].profession_skill_xp_centi["teaching"] > 0);
}

#[test]
fn mature_tradition_enterprise_never_owns_goods_and_death_cleans_live_relations() {
    let mut state = FamilyAuthorityState::empty("colony", 4);
    building(&mut state, "school", None, Some(TeachingSite::School));
    birth(&mut state, "grandparent", LifeStage::Adult, None, None);
    birth(
        &mut state,
        "parent",
        LifeStage::Adult,
        Some("grandparent"),
        None,
    );
    for (cat, prefix) in [("grandparent", "grand"), ("parent", "parent")] {
        for index in 0..100 {
            apply(
                &mut state,
                format!("receipt_{prefix}{index}"),
                FamilyOperation::RecordProfessionalCompletion(ProfessionalCompletion {
                    task_id: format!("task_{prefix}{index}"),
                    cat_id: cat.to_owned(),
                    profession_id: "fishing".to_owned(),
                    skill_id: "fishing".to_owned(),
                    skill_xp_centi: 2_500,
                    enterprise_id: None,
                }),
            );
        }
    }
    apply(
        &mut state,
        "receipt_enterprise",
        FamilyOperation::CreateMatureEnterprise(EnterpriseRequest {
            enterprise_id: "fishingenterprise".to_owned(),
            tradition_id: "tradition_fishing".to_owned(),
            profession_id: "fishing".to_owned(),
            site_id: "school".to_owned(),
            station_profession: false,
        }),
    );
    let enterprise = &state.enterprises["fishingenterprise"].enterprise;
    assert_eq!(
        enterprise.goods_ownership,
        cat_sim::family_specialization::EnterpriseGoodsOwnership::ColonyOwned
    );
    birth(&mut state, "partner", LifeStage::Adult, None, None);
    apply(
        &mut state,
        "receipt_review",
        FamilyOperation::ReviewAutonomousPartnerships,
    );
    apply(
        &mut state,
        "receipt_death",
        FamilyOperation::RecordDeath {
            cat_id: "parent".to_owned(),
        },
    );
    assert!(!state.cats["parent"].alive);
    assert!(
        !state
            .partnerships
            .values()
            .any(|partnership| partnership.first_cat_id == "parent"
                || partnership.second_cat_id == "parent")
    );
}

#[test]
fn receipts_are_idempotent_conflicts_are_rejected_and_decode_is_strict() {
    let mut state = FamilyAuthorityState::empty("colony", 9);
    let first_command = command(
        "receipt_once",
        0,
        FamilyOperation::RegisterBirth(BirthRegistration {
            newborn_cat_id: "founder".to_owned(),
            life_stage: LifeStage::Adult,
            first_parent_id: None,
            second_parent_id: None,
            attribute_authority_ref: "attributes_founder".to_owned(),
            relational_analytical_authority_ref: "axis_founder".to_owned(),
        }),
    );
    let first = state.apply(first_command.clone()).expect("initial apply");
    let replay = state.apply(first_command).expect("same receipt replays");
    assert_eq!(first, replay);
    let conflict = state.apply(command(
        "receipt_once",
        state.revision,
        FamilyOperation::RecordDeath {
            cat_id: "founder".to_owned(),
        },
    ));
    assert!(matches!(
        conflict,
        Err(FamilyAuthorityError::ReceiptConflict(_))
    ));
    assert!(FamilyAuthorityState::decode_json(r#"{"schemaVersion":1,"colonyId":"colony","colonySeed":1,"revision":0,"cats":{},"buildings":{},"partnerships":{},"households":{},"residences":{},"teachingObligations":{},"enterprises":{},"completedTaskIds":[],"receipts":{},"unknown":true}"#).is_err());
}
