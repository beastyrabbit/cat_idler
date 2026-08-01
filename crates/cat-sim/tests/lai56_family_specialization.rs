#[path = "../src/skill_catalog.rs"]
mod skill_catalog;

#[path = "../src/family_specialization.rs"]
mod family_specialization;

#[path = "../src/family_housing.rs"]
mod family_housing;

use std::collections::{BTreeMap, BTreeSet};

use family_housing::{
    CareActivityChoice, DEN_BEDS, ELDER_LODGE_BEDS, ElderLodgeEffects, FAMILY_HOME_DEPENDENT_BEDS,
    FAMILY_HOME_PARTNERED_ADULT_BEDS, FamilyHousingError, HouseholdProfile, HousingKind,
    HousingMoveReason, LifeStage, NURSERY_PERMANENT_BEDS, PartnershipAuthority,
    PartnershipCandidate, TEACHING_OBLIGATION_REAL_TASKS, TeachingObligation, TeachingSite,
    VersionedFamilyHousingState, choose_care_activity, close_kin_excluded, complete_teaching,
    defer_for_emergency, elder_lodge_effects, housing_capacity, housing_move_recommendation,
    partnership_score, record_parent_real_task,
};
use family_specialization::{
    APPRENTICE_LEARNING_BASIS_POINTS, AptitudeInheritance, BIRTH_BLEND_BASIS_POINTS,
    BIRTH_BOTH_BASIS_POINTS, BIRTH_FIRST_PARENT_BASIS_POINTS, BIRTH_NONE_BASIS_POINTS,
    BIRTH_SECOND_PARENT_BASIS_POINTS, BIRTH_SKILL_CAP_XP_CENTI, BLEND_PARENT_TRANSFER_BASIS_POINTS,
    BirthSeedOutcome, EnterpriseGoodsOwnership, FamilyBranchRule, FamilyEnterprise,
    FamilySpecializationError, GenerationProfessionRecord, LearningContext,
    MATURE_TRADITION_JOINT_SUCCESSFUL_UNITS, MATURE_TRADITION_MIN_LEVEL,
    MATURE_TRADITION_MIN_LINKED_GENERATIONS, ParentProfessionSeed, PersonalityAxis,
    SINGLE_PARENT_TRANSFER_BASIS_POINTS, TRADITION_LEARNING_BASIS_POINTS,
    VersionedFamilySpecializationState, apply_learning_context, birth_seed_band_sum_basis_points,
    birth_seed_grant, formal_teaching_xp, keyed_birth_seed_outcome, occupational_surname_key,
    tradition_maturity,
};
use skill_catalog::{LEVEL_100_XP_CENTI, SkillProgress, XP_CENTI_PER_PRIMARY_HOUR, XpGrantSource};

#[test]
fn lai56_birth_seed_bands_transfers_cap_and_inheritance_contract_are_exact() {
    assert_eq!(birth_seed_band_sum_basis_points(), 10_000);
    assert_eq!(BIRTH_FIRST_PARENT_BASIS_POINTS, 3_000);
    assert_eq!(BIRTH_SECOND_PARENT_BASIS_POINTS, 3_000);
    assert_eq!(BIRTH_BLEND_BASIS_POINTS, 1_250);
    assert_eq!(BIRTH_BOTH_BASIS_POINTS, 1_250);
    assert_eq!(BIRTH_NONE_BASIS_POINTS, 1_500);
    assert_eq!(SINGLE_PARENT_TRANSFER_BASIS_POINTS, 500);
    assert_eq!(BLEND_PARENT_TRANSFER_BASIS_POINTS, 250);
    assert_eq!(BIRTH_SKILL_CAP_XP_CENTI, 625 * XP_CENTI_PER_PRIMARY_HOUR);

    let first = ParentProfessionSeed::new(
        "parent_a",
        "lineage_a",
        "tradition_hunter",
        BTreeMap::from([
            ("hunting".to_owned(), 1_000_000),
            ("teaching".to_owned(), 200_000),
        ]),
    );
    let second = ParentProfessionSeed::new(
        "parent_b",
        "lineage_b",
        "tradition_fisher",
        BTreeMap::from([
            ("fishing".to_owned(), 800_000),
            ("hunting".to_owned(), 100_000),
        ]),
    );

    let single = birth_seed_grant(BirthSeedOutcome::FirstParent, &first, &second);
    assert_eq!(single.inherited_skill_xp_centi["hunting"], 50_000);
    assert_eq!(single.inherited_skill_xp_centi["teaching"], 10_000);
    assert_eq!(single.source_lineage_ids, vec!["lineage_a"]);

    let second_single = birth_seed_grant(BirthSeedOutcome::SecondParent, &first, &second);
    assert_eq!(second_single.inherited_skill_xp_centi["fishing"], 40_000);

    let blend = birth_seed_grant(BirthSeedOutcome::Blend, &first, &second);
    assert_eq!(blend.inherited_skill_xp_centi["hunting"], 27_500);
    assert_eq!(blend.inherited_skill_xp_centi["fishing"], 20_000);

    let none = birth_seed_grant(BirthSeedOutcome::None, &first, &second);
    assert!(none.inherited_skill_xp_centi.is_empty());

    let capped = birth_seed_grant(BirthSeedOutcome::Both, &first, &second);
    assert_eq!(
        capped.inherited_skill_xp_centi["hunting"],
        BIRTH_SKILL_CAP_XP_CENTI
    );
    assert_eq!(
        capped.inherited_personality_axes,
        vec![PersonalityAxis::RelationalAnalytical]
    );
    assert_eq!(
        capped.aptitude_inheritance,
        AptitudeInheritance::AuthoritativeAttributeSystem
    );
    assert!(capped.inherited_acquired_trait_ids.is_empty());

    assert_eq!(
        keyed_birth_seed_outcome("kitten_01", "parent_a", "parent_b"),
        keyed_birth_seed_outcome("kitten_01", "parent_a", "parent_b")
    );
}

#[test]
fn lai56_tradition_apprenticeship_and_formal_teaching_are_bounded_and_non_debiting() {
    assert_eq!(TRADITION_LEARNING_BASIS_POINTS, 11_000);
    assert_eq!(APPRENTICE_LEARNING_BASIS_POINTS, 12_500);
    assert_eq!(
        apply_learning_context(100, LearningContext::FamilyTradition),
        110
    );
    assert_eq!(
        apply_learning_context(100, LearningContext::ParentApprentice),
        125
    );
    assert_eq!(
        apply_learning_context(100, LearningContext::MentorApprentice),
        125
    );
    assert_eq!(apply_learning_context(100, LearningContext::Ordinary), 100);
    assert_eq!(
        apply_learning_context(100, LearningContext::TraditionMentored),
        137
    );

    let ordinary = formal_teaching_xp("fishing", SkillProgress::new(80 * 80 * 100), 60);
    assert_eq!(ordinary.learner_skill_id, "fishing");
    assert_eq!(ordinary.learner_xp_centi, 80);
    assert_eq!(ordinary.teacher_xp_debited, 0);
    assert!(ordinary.teacher_grants.iter().any(|grant| {
        grant.skill_id == "teaching" && grant.xp_centi > 0 && grant.source == XpGrantSource::Primary
    }));

    let bounded_master = formal_teaching_xp(
        "fishing",
        SkillProgress::new(LEVEL_100_XP_CENTI + 10_000_000),
        60,
    );
    assert_eq!(bounded_master.learner_xp_centi, 125);
    assert_eq!(bounded_master.teacher_xp_debited, 0);
}

#[test]
fn lai56_mature_tradition_requires_linked_generations_units_and_station_continuity() {
    assert_eq!(MATURE_TRADITION_MIN_LINKED_GENERATIONS, 2);
    let records = vec![
        GenerationProfessionRecord {
            cat_id: "elder".to_owned(),
            generation_index: 1,
            parent_cat_ids: vec![],
            profession_id: "metalworking".to_owned(),
            skill_level: MATURE_TRADITION_MIN_LEVEL,
            successful_units: 80,
            physical_enterprise_id: Some("smithy_one".to_owned()),
        },
        GenerationProfessionRecord {
            cat_id: "child".to_owned(),
            generation_index: 2,
            parent_cat_ids: vec!["elder".to_owned()],
            profession_id: "metalworking".to_owned(),
            skill_level: MATURE_TRADITION_MIN_LEVEL,
            successful_units: MATURE_TRADITION_JOINT_SUCCESSFUL_UNITS - 80,
            physical_enterprise_id: Some("smithy_one".to_owned()),
        },
    ];
    let mature = tradition_maturity("metalworking", &records, true);
    assert!(mature.mature);
    assert!(mature.station_enterprise_continuity);
    assert_eq!(
        mature.joint_successful_units,
        MATURE_TRADITION_JOINT_SUCCESSFUL_UNITS
    );

    let mut split_site = records.clone();
    split_site[1].physical_enterprise_id = Some("smithy_two".to_owned());
    assert!(!tradition_maturity("metalworking", &split_site, true).mature);

    let mut under_level = records.clone();
    under_level[1].skill_level = MATURE_TRADITION_MIN_LEVEL - 1;
    assert!(!tradition_maturity("metalworking", &under_level, true).mature);
}

#[test]
fn lai56_surnames_branches_ancestry_and_enterprises_preserve_colony_goods() {
    assert_eq!(occupational_surname_key("milling"), Some("surname.miller"));
    assert_eq!(
        occupational_surname_key("milling_de"),
        Some("surname.muller")
    );
    assert_eq!(
        occupational_surname_key("metalworking"),
        Some("surname.smith")
    );

    let branch = FamilyBranchRule {
        lineage_id: "lineage_a".to_owned(),
        profession_id: "fishing".to_owned(),
        adult_surname_key: "surname.fisher".to_owned(),
        child_surname_key: "surname.hunter".to_owned(),
        follows_profession: false,
        ancestry_lineage_ids: vec!["lineage_a".to_owned(), "lineage_b".to_owned()],
    };
    assert_ne!(branch.adult_surname_key, branch.child_surname_key);
    assert!(branch.retains_family_membership_when_leaving_trade());

    let enterprise = FamilyEnterprise {
        enterprise_id: "smithy_one".to_owned(),
        tradition_id: "tradition_smith".to_owned(),
        profession_id: "metalworking".to_owned(),
        site_id: "site_smithy".to_owned(),
        signage_key: "enterprise.sign.smithy_one".to_owned(),
        worker_preference: true,
        mentoring_identity: true,
        history_identity: true,
        ui_identity: true,
        goods_ownership: EnterpriseGoodsOwnership::ColonyOwned,
    };
    assert_eq!(
        enterprise.goods_ownership,
        EnterpriseGoodsOwnership::ColonyOwned
    );
}

#[test]
fn lai56_partnerships_are_autonomous_deterministic_and_close_kin_excluded() {
    let first = PartnershipCandidate {
        cat_id: "cat_a".to_owned(),
        close_ancestor_or_descendant_ids: BTreeSet::new(),
        close_sibling_ids: BTreeSet::new(),
        inherited_attribute_score: 16,
        profession_skill_level: 45,
        personality_compatibility_basis_points: 8_500,
        relational_analytical: -10,
        tradition_ids: BTreeSet::from(["tradition_fisher".to_owned()]),
        housing_ready: true,
    };
    let mut second = PartnershipCandidate {
        cat_id: "cat_b".to_owned(),
        close_ancestor_or_descendant_ids: BTreeSet::new(),
        close_sibling_ids: BTreeSet::new(),
        inherited_attribute_score: 14,
        profession_skill_level: 35,
        personality_compatibility_basis_points: 8_000,
        relational_analytical: -5,
        tradition_ids: BTreeSet::from(["tradition_fisher".to_owned()]),
        housing_ready: true,
    };
    let score = partnership_score(&first, &second, 42, PartnershipAuthority::Autonomous).unwrap();
    assert_eq!(
        score,
        partnership_score(&second, &first, 42, PartnershipAuthority::Autonomous).unwrap()
    );
    assert!(partnership_score(&first, &second, 42, PartnershipAuthority::GodArranged).is_none());

    second.close_sibling_ids.insert("cat_a".to_owned());
    assert!(close_kin_excluded(&first, &second));
    assert!(partnership_score(&first, &second, 42, PartnershipAuthority::Autonomous).is_none());
}

#[test]
fn lai56_housing_capacities_moves_and_elder_lodge_effects_are_bounded() {
    assert!(LifeStage::Kitten < LifeStage::Adult);
    assert_eq!(housing_capacity(HousingKind::Den).permanent_beds, DEN_BEDS);
    assert_eq!(
        housing_capacity(HousingKind::FamilyHome).partnered_adult_beds,
        FAMILY_HOME_PARTNERED_ADULT_BEDS
    );
    assert_eq!(
        housing_capacity(HousingKind::FamilyHome).dependent_beds,
        FAMILY_HOME_DEPENDENT_BEDS
    );
    assert_eq!(
        housing_capacity(HousingKind::ElderLodge).elder_beds,
        ELDER_LODGE_BEDS
    );
    assert_eq!(
        housing_capacity(HousingKind::Nursery).permanent_beds,
        NURSERY_PERMANENT_BEDS
    );

    let parenting = HouseholdProfile {
        household_id: "household_a".to_owned(),
        adult_cat_ids: vec!["adult_a".to_owned(), "adult_b".to_owned()],
        dependent_cat_ids: vec!["kitten_a".to_owned()],
        elder_cat_ids: vec![],
        pregnant_or_parenting: true,
        empty_nest: false,
        current_housing: HousingKind::Den,
    };
    assert_eq!(
        housing_move_recommendation(&parenting, true, false, false).reason,
        HousingMoveReason::ParentingFamilyHomePriority
    );

    let elder = HouseholdProfile {
        household_id: "household_b".to_owned(),
        adult_cat_ids: vec![],
        dependent_cat_ids: vec![],
        elder_cat_ids: vec!["elder_a".to_owned()],
        pregnant_or_parenting: false,
        empty_nest: true,
        current_housing: HousingKind::FamilyHome,
    };
    assert_eq!(
        housing_move_recommendation(&elder, false, true, true).reason,
        HousingMoveReason::ElderLodgeFreesFamilyHome
    );

    let effects = elder_lodge_effects(1_000, 10);
    assert_eq!(
        effects,
        ElderLodgeEffects {
            old_age_hazard_basis_points: 100,
            social_recovery_basis_points: 1_100,
            mentoring_basis_points: 1_150,
            immortality: false,
        }
    );
}

#[test]
fn lai56_teaching_obligations_persist_defer_and_precede_ambient_cleaning() {
    assert_eq!(TEACHING_OBLIGATION_REAL_TASKS, 3);
    let mut obligation = TeachingObligation::new("parent_a", "kitten_a");
    for _ in 0..TEACHING_OBLIGATION_REAL_TASKS {
        obligation = record_parent_real_task(obligation);
    }
    assert!(obligation.due);

    let deferred = defer_for_emergency(obligation.clone());
    assert!(deferred.due);
    assert!(deferred.deferred_by_emergency);
    assert_eq!(
        choose_care_activity(&deferred, Some("mentor_a"), TeachingSite::FamilyHome),
        CareActivityChoice::EmergencyWork
    );

    assert_eq!(
        choose_care_activity(&obligation, Some("mentor_a"), TeachingSite::FamilyHome),
        CareActivityChoice::PhysicalTeaching {
            site: TeachingSite::FamilyHome,
            mentor_cat_id: "mentor_a".to_owned(),
        }
    );
    assert_eq!(
        choose_care_activity(&obligation, Some("mentor_a"), TeachingSite::Den),
        CareActivityChoice::AmbientCleaning
    );

    let completed = complete_teaching(obligation);
    assert!(!completed.due);
    assert_eq!(completed.completed_real_tasks_since_teach, 0);
}

#[test]
fn lai56_versioned_leaf_states_are_strict_and_stable_without_hot_root_authority() {
    assert!(family_specialization::is_stable_id("cat:ash-paw"));
    assert!(family_specialization::is_stable_id("planner:v1|8:cat.home"));
    assert!(!family_specialization::is_stable_id("cat/ash"));

    let specialization = VersionedFamilySpecializationState::empty();
    specialization.validate().unwrap();
    let json = serde_json::to_string(&specialization).unwrap();
    assert!(json.contains("\"schemaVersion\":1"));
    assert!(
        serde_json::from_str::<VersionedFamilySpecializationState>(
            r#"{"schemaVersion":1,"parentSeeds":{},"enterprises":{},"extra":true}"#
        )
        .is_err()
    );

    let mut bad_specialization = VersionedFamilySpecializationState::empty();
    bad_specialization.parent_seeds.insert(
        "BadId".to_owned(),
        ParentProfessionSeed::new("parent", "lineage", "tradition", BTreeMap::new()),
    );
    assert_eq!(
        bad_specialization.validate(),
        Err(FamilySpecializationError::InvalidStableId(
            "BadId".to_owned()
        ))
    );

    let housing = VersionedFamilyHousingState::empty();
    housing.validate().unwrap();
    assert!(
        serde_json::from_str::<VersionedFamilyHousingState>(
            r#"{"schemaVersion":1,"households":{},"teachingObligations":{},"extra":true}"#
        )
        .is_err()
    );

    let mut bad_housing = VersionedFamilyHousingState::empty();
    bad_housing.teaching_obligations.insert(
        "bad/id".to_owned(),
        TeachingObligation::new("parent", "kitten"),
    );
    assert_eq!(
        bad_housing.validate(),
        Err(FamilyHousingError::InvalidStableId("bad/id".to_owned()))
    );
}
