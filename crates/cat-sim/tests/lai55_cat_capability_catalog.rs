#[path = "../src/skill_catalog.rs"]
mod skill_catalog;

#[path = "../src/cat_capabilities.rs"]
mod cat_capabilities;

use std::collections::{BTreeMap, BTreeSet};

use cat_capabilities::{
    CapabilityAttributes, CapabilityBodyPart, CapabilityProfile, EffectiveAnatomy,
    InheritedAttribute, LaborAffinity, LaborAffinityProfile, PartFunction, SkillJudgmentUse,
    anatomy_eligibility, assignment_candidate_key,
};
use skill_catalog::{
    AMBIENT_CLEANING_INTERVAL_MINUTES, AMBIENT_CLEANING_XP_CENTI, AMBIENT_DISCOVERY_XP_CENTI,
    ActivityCompletion, ActivityXpDeclaration, AmbientSkillCandidate, HAUL_LEG_XP_CENTI,
    LEVEL_100_XP_CENTI, LeaderDutyDomain, OfficeKind, SkillProgress, XpGrantSource,
    activity_xp_grants, ambient_cleaning_xp_grants, floor_level_from_xp_centi,
    leader_duty_xp_grants, office_duty_profile, office_duty_xp_grants, office_proficiency_skill_id,
    productive_xp_centi_for_minutes, skill_definition, skills_in_category,
    supervised_officer_xp_grants,
};

#[test]
fn lai55_attributes_preserve_existing_eight_and_add_charisma_intelligence() {
    assert_eq!(
        InheritedAttribute::ALL,
        [
            InheritedAttribute::Attack,
            InheritedAttribute::Defense,
            InheritedAttribute::Hunting,
            InheritedAttribute::Medicine,
            InheritedAttribute::Cleaning,
            InheritedAttribute::Building,
            InheritedAttribute::Leadership,
            InheritedAttribute::Vision,
            InheritedAttribute::Charisma,
            InheritedAttribute::Intelligence,
        ]
    );

    assert!(CapabilityAttributes::new(1, 2, 3, 4, 5, 6, 7, 8, 9, 20).is_ok());
    assert!(CapabilityAttributes::new(0, 2, 3, 4, 5, 6, 7, 8, 9, 20).is_err());
    assert!(CapabilityAttributes::new(1, 2, 3, 4, 5, 6, 7, 8, 9, 21).is_err());

    let profile = CapabilityProfile {
        attributes: CapabilityAttributes::new(10, 10, 10, 10, 10, 10, 10, 10, 12, 16).unwrap(),
        influence_xp_centi: 2_500,
    };
    assert!(profile.effective_charisma_basis_points() > 12_000);
    assert!(
        profile.intelligence_modifier_basis_points(SkillJudgmentUse::Planning)
            > profile.intelligence_modifier_basis_points(SkillJudgmentUse::Learning)
    );
}

#[test]
fn lai55_skill_registry_is_data_owned_and_complete() {
    assert_eq!(
        skills_in_category("gathering").as_slice(),
        &[
            "hunting",
            "fishing",
            "foraging",
            "farming",
            "waterwork",
            "woodcutting",
            "quarrying",
            "scouting",
        ]
    );
    assert_eq!(
        skills_in_category("construction_logistics").as_slice(),
        &["construction", "roadwork", "hauling"]
    );
    assert_eq!(
        skills_in_category("food").as_slice(),
        &["milling", "cooking", "preservation", "brewing"]
    );
    assert_eq!(
        skills_in_category("industry").as_slice(),
        &[
            "woodworking",
            "crafting",
            "textiles",
            "tanning",
            "metalworking",
            "gemwork",
        ]
    );
    assert_eq!(
        skills_in_category("care_service").as_slice(),
        &["medicine", "cleaning", "teaching", "influence"]
    );
    assert_eq!(
        skills_in_category("martial_spiritual").as_slice(),
        &["fighting", "training", "ritual", "command"]
    );
    assert_eq!(
        skills_in_category("civic").as_slice(),
        &[
            "research",
            "trade",
            "diplomacy",
            "governance",
            "administration",
        ]
    );

    let office_ids = OfficeKind::ALL
        .iter()
        .map(|office| office_proficiency_skill_id(*office))
        .collect::<Vec<_>>();
    assert_eq!(
        office_ids.as_slice(),
        &[
            "office_steward",
            "office_accountant",
            "office_forester",
            "office_farmer",
            "office_captain",
            "office_loremaster",
            "office_cloth_leader",
        ]
    );
    for skill_id in office_ids {
        assert_eq!(skill_definition(skill_id).unwrap().category_id, "office");
    }
}

#[test]
fn lai55_xp_uses_exact_productive_secondary_supervised_haul_and_mastery_math() {
    assert_eq!(productive_xp_centi_for_minutes(60), 100);
    let grants = activity_xp_grants(
        ActivityXpDeclaration {
            primary_skill_id: "woodcutting",
            secondary_skill_ids: &["foraging"],
            office_skill_id: Some("office_forester"),
            supervised_skill_ids: &["teaching"],
            haul_legs: 2,
        },
        60,
        ActivityCompletion::Productive,
    );
    assert!(grants.iter().any(|grant| {
        grant.skill_id == "woodcutting"
            && grant.xp_centi == 100
            && grant.source == XpGrantSource::Primary
    }));
    assert!(grants.iter().any(|grant| {
        grant.skill_id == "foraging"
            && grant.xp_centi == 25
            && grant.source == XpGrantSource::Secondary
    }));
    assert!(grants.iter().any(|grant| {
        grant.skill_id == "teaching"
            && grant.xp_centi == 10
            && grant.source == XpGrantSource::Supervised
    }));
    assert!(
        grants.iter().any(|grant| {
            grant.skill_id == "hauling" && grant.xp_centi == 2 * HAUL_LEG_XP_CENTI
        })
    );
    assert!(
        activity_xp_grants(
            ActivityXpDeclaration {
                primary_skill_id: "woodcutting",
                secondary_skill_ids: &["foraging"],
                office_skill_id: None,
                supervised_skill_ids: &[],
                haul_legs: 1,
            },
            60,
            ActivityCompletion::InvalidRoute,
        )
        .is_empty()
    );

    assert_eq!(floor_level_from_xp_centi(9_999 * 100), 99);
    assert_eq!(floor_level_from_xp_centi(10_000 * 100), 100);
    let progress = SkillProgress::new(LEVEL_100_XP_CENTI + 12_345);
    assert_eq!(progress.output_effect_level(), 100);
    assert_eq!(progress.mastery_xp_centi(), 12_345);
}

#[test]
fn lai55_office_learning_separates_report_clearance_from_cross_training() {
    let steward = office_duty_profile(OfficeKind::Steward);
    assert_eq!(
        steward.cross_training_skill_ids,
        &["construction", "roadwork", "hauling"]
    );
    assert_eq!(
        office_duty_profile(OfficeKind::Accountant).cross_training_skill_ids,
        &["trade", "administration"]
    );
    assert_eq!(
        office_duty_profile(OfficeKind::Captain).supervised_skill_ids,
        &["command"]
    );

    let officer = office_duty_xp_grants(OfficeKind::Loremaster, 60);
    assert!(officer.iter().any(|grant| {
        grant.skill_id == "office_loremaster"
            && grant.xp_centi == 100
            && grant.report_clearance_office == Some(OfficeKind::Loremaster)
    }));
    assert!(officer.iter().any(|grant| {
        grant.skill_id == "governance"
            && grant.xp_centi == 25
            && grant.report_clearance_office.is_none()
    }));
    assert!(
        officer
            .iter()
            .any(|grant| grant.skill_id == "research" && grant.xp_centi == 25)
    );

    let supervised = supervised_officer_xp_grants(OfficeKind::Captain, 60);
    assert!(
        supervised
            .iter()
            .all(|grant| grant.report_clearance_office.is_none())
    );
    assert!(
        supervised
            .iter()
            .any(|grant| grant.skill_id == "command" && grant.xp_centi == 10)
    );

    let leader = leader_duty_xp_grants(LeaderDutyDomain::Trade, 60);
    assert!(
        leader
            .iter()
            .any(|grant| grant.skill_id == "governance" && grant.xp_centi == 100)
    );
    assert!(
        leader
            .iter()
            .any(|grant| grant.skill_id == "trade" && grant.xp_centi == 25)
    );
}

#[test]
fn lai55_matching_is_lexicographic_and_refusal_is_never_overridden() {
    let anatomy = EffectiveAnatomy::healthy();
    let emergency_disliked = assignment_candidate_key(cat_capabilities::AssignmentInput {
        cat_stable_id: "cat-b",
        skill_id: "hauling",
        tier: cat_capabilities::AssignmentTier::Emergency,
        affinity: LaborAffinity::Disliked,
        family_enterprise: false,
        skill_level: 1,
        attribute_score: 1,
        continuity_minutes: 0,
        route_cost: 50,
        anatomy: &anatomy,
        self_preservation: false,
    })
    .unwrap();
    let leader_loved = assignment_candidate_key(cat_capabilities::AssignmentInput {
        cat_stable_id: "cat-a",
        skill_id: "hauling",
        tier: cat_capabilities::AssignmentTier::LeaderPriority(1),
        affinity: LaborAffinity::Loved,
        family_enterprise: true,
        skill_level: 100,
        attribute_score: 20,
        continuity_minutes: 100,
        route_cost: 1,
        anatomy: &anatomy,
        self_preservation: false,
    })
    .unwrap();
    assert!(emergency_disliked < leader_loved);

    let enterprise = assignment_candidate_key(cat_capabilities::AssignmentInput {
        cat_stable_id: "cat-c",
        skill_id: "cooking",
        tier: cat_capabilities::AssignmentTier::Background,
        affinity: LaborAffinity::Neutral,
        family_enterprise: true,
        skill_level: 1,
        attribute_score: 1,
        continuity_minutes: 0,
        route_cost: 99,
        anatomy: &anatomy,
        self_preservation: false,
    })
    .unwrap();
    let loved = assignment_candidate_key(cat_capabilities::AssignmentInput {
        cat_stable_id: "cat-d",
        skill_id: "cooking",
        tier: cat_capabilities::AssignmentTier::Background,
        affinity: LaborAffinity::Loved,
        family_enterprise: false,
        skill_level: 100,
        attribute_score: 20,
        continuity_minutes: 100,
        route_cost: 1,
        anatomy: &anatomy,
        self_preservation: false,
    })
    .unwrap();
    assert!(enterprise < loved);

    assert!(
        assignment_candidate_key(cat_capabilities::AssignmentInput {
            cat_stable_id: "cat-e",
            skill_id: "hauling",
            tier: cat_capabilities::AssignmentTier::Emergency,
            affinity: LaborAffinity::Refused,
            family_enterprise: true,
            skill_level: 100,
            attribute_score: 20,
            continuity_minutes: 100,
            route_cost: 1,
            anatomy: &anatomy,
            self_preservation: false,
        })
        .is_none()
    );
    assert!(
        assignment_candidate_key(cat_capabilities::AssignmentInput {
            cat_stable_id: "cat-f",
            skill_id: "hauling",
            tier: cat_capabilities::AssignmentTier::Emergency,
            affinity: LaborAffinity::Loved,
            family_enterprise: true,
            skill_level: 100,
            attribute_score: 20,
            continuity_minutes: 100,
            route_cost: 1,
            anatomy: &anatomy,
            self_preservation: true,
        })
        .is_none()
    );
}

#[test]
fn lai55_affinities_serialize_strictly_and_expose_all_bands() {
    let mut affinities = BTreeMap::new();
    affinities.insert("hunting".to_owned(), LaborAffinity::Loved);
    affinities.insert("cleaning".to_owned(), LaborAffinity::Preferred);
    affinities.insert("research".to_owned(), LaborAffinity::Neutral);
    affinities.insert("hauling".to_owned(), LaborAffinity::Disliked);
    affinities.insert("fighting".to_owned(), LaborAffinity::Refused);
    let profile = LaborAffinityProfile {
        affinities,
        family_enterprise_skill_ids: BTreeSet::from(["hunting".to_owned()]),
    };

    assert_eq!(profile.affinity_for("hunting"), LaborAffinity::Loved);
    assert_eq!(profile.affinity_for("unknown"), LaborAffinity::Neutral);
    assert!(profile.is_family_enterprise("hunting"));
    assert!(!profile.eligible_for_village_labor("fighting"));

    let json = serde_json::to_string(&profile).unwrap();
    assert!(json.contains("familyEnterpriseSkillIds"));
    let denied = serde_json::from_str::<LaborAffinityProfile>(
        r#"{"affinities":{},"familyEnterpriseSkillIds":[],"extra":true}"#,
    );
    assert!(denied.is_err());
}

#[test]
fn lai55_anatomy_blocks_independently_and_prosthetics_can_restore_eligibility() {
    let missing_paw = EffectiveAnatomy::healthy().with_part(
        CapabilityBodyPart::FrontLeftPaw,
        PartFunction {
            natural_basis_points: 0,
            prosthetic_basis_points: None,
        },
    );
    assert!(anatomy_eligibility("woodcutting", &missing_paw, true).is_err());

    let fitted_paw = EffectiveAnatomy::healthy().with_part(
        CapabilityBodyPart::FrontLeftPaw,
        PartFunction {
            natural_basis_points: 0,
            prosthetic_basis_points: Some(7_500),
        },
    );
    assert!(anatomy_eligibility("woodcutting", &fitted_paw, true).is_ok());
    assert!(anatomy_eligibility("woodcutting", &fitted_paw, false).is_err());

    let missing_eye = EffectiveAnatomy::healthy().with_part(
        CapabilityBodyPart::LeftEye,
        PartFunction {
            natural_basis_points: 0,
            prosthetic_basis_points: None,
        },
    );
    assert!(anatomy_eligibility("hunting", &missing_eye, true).is_err());
    assert!(anatomy_eligibility("governance", &missing_eye, true).is_ok());
}

#[test]
fn lai55_ambient_cleaning_is_exact_and_keyed() {
    assert_eq!(AMBIENT_CLEANING_INTERVAL_MINUTES, 10);
    let mut interval = None;
    for candidate_interval in 0..200 {
        let grants = ambient_cleaning_xp_grants(
            "cat-ambient",
            candidate_interval,
            &[
                AmbientSkillCandidate {
                    skill_id: "governance",
                    compatible: true,
                    refused: false,
                },
                AmbientSkillCandidate {
                    skill_id: "fighting",
                    compatible: true,
                    refused: true,
                },
            ],
        );
        if grants.len() == 2 {
            interval = Some((candidate_interval, grants));
            break;
        }
    }
    let (_interval, grants) = interval.expect("5 percent keyed ambient gain appears in 200 tries");
    assert!(grants.iter().any(|grant| {
        grant.skill_id == "cleaning"
            && grant.xp_centi == AMBIENT_CLEANING_XP_CENTI
            && grant.source == XpGrantSource::AmbientCleaning
    }));
    assert!(grants.iter().any(|grant| {
        grant.skill_id == "governance"
            && grant.xp_centi == AMBIENT_DISCOVERY_XP_CENTI
            && grant.source == XpGrantSource::AmbientDiscovery
    }));
    assert!(!grants.iter().any(|grant| grant.skill_id == "fighting"));
}
