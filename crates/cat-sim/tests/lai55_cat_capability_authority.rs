//! Focused LAI.55 authority acceptance tests.

use std::collections::{BTreeMap, BTreeSet};

use cat_sim::{
    acquired_traits::AcquiredTraitState,
    anatomy::{BodyPart, BodyPartCondition, CatAnatomy},
    cat_capabilities::{
        AssignmentTier, CapabilityAttributes, InheritedAttribute, LaborAffinity,
        LaborAffinityProfile,
    },
    cat_capability_authority::{
        AnatomyCapabilityContext, AptitudeReceipt, AssignmentEligibilityInput,
        CatCapabilityAuthority, CatCapabilityAuthorityError, CatCapabilityRegistration,
        ProductiveOutcome, ProductiveOutcomeReceipt, WorkActivity,
    },
    officer_expertise::{ExpertiseBonuses, ExpertiseLevel},
    prosthetics::ProstheticLedger,
    skill_catalog::{OfficeKind, SkillProgress},
};

fn registration(cat_id: &str, affinity: LaborAffinity) -> CatCapabilityRegistration {
    let mut affinities = BTreeMap::new();
    affinities.insert("woodcutting".to_owned(), affinity);
    CatCapabilityRegistration {
        cat_id: cat_id.to_owned(),
        attributes: CapabilityAttributes::new(11, 10, 10, 10, 10, 14, 10, 10, 12, 13).unwrap(),
        labor: LaborAffinityProfile {
            affinities,
            family_enterprise_skill_ids: BTreeSet::new(),
        },
        skills: BTreeMap::new(),
        office_duty_minutes: BTreeMap::new(),
    }
}

fn healthy_context<'a>(
    anatomy: &'a CatAnatomy,
    prosthetics: &'a ProstheticLedger,
    traits: &'a AcquiredTraitState,
) -> AnatomyCapabilityContext<'a> {
    AnatomyCapabilityContext {
        anatomy,
        prosthetics,
        acquired_traits: traits,
    }
}

#[test]
fn authority_registers_updates_removes_and_rejects_duplicate_or_unknown_real_ids() {
    let mut authority = CatCapabilityAuthority::new();
    authority
        .register_cat(registration("cat-2", LaborAffinity::Neutral))
        .unwrap();
    assert_eq!(
        authority.register_cat(registration("cat-2", LaborAffinity::Loved)),
        Err(CatCapabilityAuthorityError::DuplicateCatId)
    );
    assert_eq!(
        authority.update_cat("missing", registration("missing", LaborAffinity::Loved)),
        Err(CatCapabilityAuthorityError::UnknownCatId)
    );
    assert_eq!(
        authority.update_cat("cat-2", registration("other-cat", LaborAffinity::Loved)),
        Err(CatCapabilityAuthorityError::CatIdImmutable)
    );

    authority
        .update_cat("cat-2", registration("cat-2", LaborAffinity::Loved))
        .unwrap();
    assert_eq!(
        authority
            .cat_report("cat-2")
            .unwrap()
            .labor
            .affinity_for("woodcutting"),
        LaborAffinity::Loved
    );
    assert_eq!(authority.remove_cat("cat-2").unwrap().cat_id, "cat-2");
    assert_eq!(
        authority.remove_cat("cat-2"),
        Err(CatCapabilityAuthorityError::UnknownCatId)
    );
}

#[test]
fn outcomes_are_exactly_once_and_only_successful_work_grants_catalog_xp() {
    let mut authority = CatCapabilityAuthority::new();
    let mut cat = registration("cat-1", LaborAffinity::Neutral);
    cat.skills
        .insert("woodcutting".to_owned(), SkillProgress::new(1_000_000));
    authority.register_cat(cat).unwrap();
    let productive = ProductiveOutcomeReceipt {
        receipt_id: "work-1".to_owned(),
        cat_id: "cat-1".to_owned(),
        outcome: ProductiveOutcome::Productive {
            productive_minutes: 60,
            activity: Some(WorkActivity {
                primary_skill_id: "woodcutting".to_owned(),
                secondary_skill_ids: vec!["construction".to_owned()],
                haul_legs: 0,
            }),
            office: Some(OfficeKind::Forester),
            supervised_by: None,
        },
    };
    let first = authority
        .apply_productive_outcome_receipt(productive.clone())
        .unwrap();
    assert_eq!(
        authority
            .apply_productive_outcome_receipt(productive)
            .unwrap(),
        first,
        "same receipt must replay its original grants without applying twice"
    );
    let failed = authority
        .apply_productive_outcome_receipt(ProductiveOutcomeReceipt {
            receipt_id: "failed-1".to_owned(),
            cat_id: "cat-1".to_owned(),
            outcome: ProductiveOutcome::FailedProductive {
                productive_minutes: 60,
                activity: Some(WorkActivity {
                    primary_skill_id: "woodcutting".to_owned(),
                    secondary_skill_ids: Vec::new(),
                    haul_legs: 0,
                }),
                office: None,
                supervised_by: None,
            },
        })
        .unwrap();
    assert!(
        failed.is_empty(),
        "failed work must not grant partial learning"
    );
    authority
        .apply_productive_outcome_receipt(ProductiveOutcomeReceipt {
            receipt_id: "haul-1".to_owned(),
            cat_id: "cat-1".to_owned(),
            outcome: ProductiveOutcome::Hauling { haul_legs: 1 },
        })
        .unwrap();
    authority
        .apply_productive_outcome_receipt(ProductiveOutcomeReceipt {
            receipt_id: "refused-1".to_owned(),
            cat_id: "cat-1".to_owned(),
            outcome: ProductiveOutcome::Refused,
        })
        .unwrap();

    let report = authority.cat_report("cat-1").unwrap();
    let woodcutting = report
        .skills
        .iter()
        .find(|skill| skill.skill_id == "woodcutting")
        .unwrap();
    assert_eq!(woodcutting.progress.total_xp_centi, 1_000_125);
    assert_eq!(woodcutting.mastery_xp_centi, 125);
    assert_eq!(
        report
            .skills
            .iter()
            .find(|skill| skill.skill_id == "hauling")
            .unwrap()
            .progress
            .total_xp_centi,
        25
    );
    let forester = report
        .offices
        .iter()
        .find(|office| office.office == OfficeKind::Forester)
        .unwrap();
    assert_eq!(forester.completed_duty_minutes, 60);
    assert_eq!(forester.proficiency.total_xp_centi, 100);
    assert_eq!(
        authority.office_effective_level(
            "cat-1",
            OfficeKind::Forester,
            ExpertiseBonuses {
                workflow_operational: true,
                reinforcement_operational: true,
            },
        ),
        Ok(ExpertiseLevel::Three),
        "room/tool truth is supplied as existing operational context, not invented by the authority"
    );

    let mut conflict = ProductiveOutcomeReceipt {
        receipt_id: "work-1".to_owned(),
        cat_id: "cat-1".to_owned(),
        outcome: ProductiveOutcome::Unassigned,
    };
    assert_eq!(
        authority.apply_productive_outcome_receipt(conflict.clone()),
        Err(CatCapabilityAuthorityError::ReceiptConflict)
    );
    conflict.receipt_id = "unassigned-1".to_owned();
    assert!(
        authority
            .apply_productive_outcome_receipt(conflict)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn aptitude_replay_and_assignment_order_respect_refusal_and_authoritative_anatomy() {
    let mut authority = CatCapabilityAuthority::new();
    authority
        .register_cat(registration("cat-a", LaborAffinity::Refused))
        .unwrap();
    let aptitude = AptitudeReceipt {
        receipt_id: "ambient-1".to_owned(),
        cat_id: "cat-a".to_owned(),
        interval_index: 1,
        compatible_skill_ids: vec!["woodcutting".to_owned()],
    };
    let first = authority.apply_aptitude_receipt(aptitude.clone()).unwrap();
    assert_eq!(authority.apply_aptitude_receipt(aptitude).unwrap(), first);

    let anatomy = CatAnatomy::default();
    let prosthetics = ProstheticLedger::default();
    let traits = AcquiredTraitState::default();
    assert_eq!(
        authority
            .assignment_candidate_key(AssignmentEligibilityInput {
                cat_id: "cat-a",
                skill_id: "woodcutting",
                tier: AssignmentTier::Emergency,
                attribute: InheritedAttribute::Building,
                continuity_minutes: 0,
                route_cost: 0,
                self_preservation: false,
                anatomy: healthy_context(&anatomy, &prosthetics, &traits),
            })
            .unwrap(),
        None,
        "Refused labor is never forced, even for emergencies"
    );

    authority
        .register_cat(registration("cat-b", LaborAffinity::Loved))
        .unwrap();
    let key = authority
        .assignment_candidate_key(AssignmentEligibilityInput {
            cat_id: "cat-b",
            skill_id: "woodcutting",
            tier: AssignmentTier::LeaderPriority(3),
            attribute: InheritedAttribute::Building,
            continuity_minutes: 12,
            route_cost: 9,
            self_preservation: false,
            anatomy: healthy_context(&anatomy, &prosthetics, &traits),
        })
        .unwrap()
        .unwrap();
    assert_eq!(
        (key.urgency_rank, key.affinity_rank, key.stable_id),
        (3, 1, "cat-b".to_owned())
    );

    let mut missing_paw = CatAnatomy::default();
    missing_paw.injure(BodyPart::FrontLeftPaw, BodyPartCondition::Missing);
    assert_eq!(
        authority
            .assignment_candidate_key(AssignmentEligibilityInput {
                cat_id: "cat-b",
                skill_id: "woodcutting",
                tier: AssignmentTier::Emergency,
                attribute: InheritedAttribute::Building,
                continuity_minutes: 0,
                route_cost: 0,
                self_preservation: false,
                anatomy: healthy_context(&missing_paw, &prosthetics, &traits),
            })
            .unwrap(),
        None,
        "the authority reads CatAnatomy plus ProstheticLedger directly"
    );
}

#[test]
fn serde_is_strict_restart_safe_and_rejects_duplicate_ids() {
    let mut authority = CatCapabilityAuthority::new();
    let mut record = registration("cat-serde", LaborAffinity::Neutral);
    record
        .skills
        .insert("woodcutting".to_owned(), SkillProgress::new(1_000_001));
    authority.register_cat(record).unwrap();
    let encoded = serde_json::to_value(&authority).unwrap();
    let restored: CatCapabilityAuthority = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(restored.report(), authority.report());

    let mut duplicate = encoded.clone();
    let cats = duplicate["cats"].as_array_mut().unwrap();
    let first = cats.first().unwrap().clone();
    cats.push(first);
    assert!(serde_json::from_value::<CatCapabilityAuthority>(duplicate).is_err());

    let mut unknown = encoded;
    unknown["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<CatCapabilityAuthority>(unknown).is_err());
}
