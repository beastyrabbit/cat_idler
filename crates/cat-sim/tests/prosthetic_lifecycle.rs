//! Focused LAI.7 acceptance tests for prosthetic identity and lifecycle transitions.

use cat_sim::{
    acquired_traits::{AcquiredTrait, AcquiredTraitState, PROSTHETIC_ADAPTED_MINUTES},
    anatomy::{BodyPart, BodyPartCondition, CatAnatomy},
    prosthetics::{
        FitAuthorization, FitSiteKind, METAL_DURABILITY_MINUTES,
        PROSTHETIC_FUNCTION_CAP_BASIS_POINTS, ProstheticError, ProstheticId, ProstheticLedger,
        ProstheticLocation, ProstheticMaterial, RepairAuthorization, WOODEN_DURABILITY_MINUTES,
    },
};

fn missing(part: BodyPart) -> CatAnatomy {
    let mut anatomy = CatAnatomy::default();
    anatomy.injure(part, BodyPartCondition::Missing);
    anatomy
}

fn fit_auth(part: BodyPart) -> FitAuthorization<'static> {
    FitAuthorization {
        colony_id: "colony-1",
        cat_id: "cat-1",
        part,
        reservation_id: "fit-1",
        fitter_id: "medic-1",
        fitter_capable: true,
        patient_consents: true,
        site_id: "workshop-1",
        site_kind: FitSiteKind::Workshop,
        site_reachable: true,
    }
}

fn fitted_ledger(material: ProstheticMaterial, part: BodyPart) -> (ProstheticLedger, ProstheticId) {
    let mut ledger = ProstheticLedger::default();
    let id = ProstheticId::derive("colony-1", part, material, 1);
    ledger
        .register(id.clone(), material, part, "colony-1")
        .unwrap();
    ledger
        .begin_fitting(&id, &missing(part), fit_auth(part))
        .unwrap();
    ledger.complete_fitting(&id).unwrap();
    (ledger, id)
}

#[test]
fn restoration_material_rehab_adaptation_and_cap_are_exact() {
    assert_eq!(WOODEN_DURABILITY_MINUTES, 360 * 60);
    assert_eq!(METAL_DURABILITY_MINUTES, 1_080 * 60);
    assert_eq!(PROSTHETIC_FUNCTION_CAP_BASIS_POINTS, 9_000);

    let part = BodyPart::FrontLeftPaw;
    let anatomy = missing(part);
    let traits = AcquiredTraitState::default();
    let (mut wooden, wooden_id) = fitted_ledger(ProstheticMaterial::Wooden, part);
    let (metal, _) = fitted_ledger(ProstheticMaterial::Metal, part);
    assert_eq!(
        wooden.effective_part_function_basis_points(&anatomy, "cat-1", part, &traits),
        5_000
    );
    assert_eq!(
        metal.effective_part_function_basis_points(&anatomy, "cat-1", part, &traits),
        7_500
    );

    wooden.complete_rehabilitation_stage(&wooden_id).unwrap();
    assert_eq!(
        wooden.effective_part_function_basis_points(&anatomy, "cat-1", part, &traits),
        5_200
    );
    for _ in 0..30 {
        wooden.complete_rehabilitation_stage(&wooden_id).unwrap();
    }
    let mut adapted = AcquiredTraitState::default();
    adapted.record_productive_prosthetic_minutes(PROSTHETIC_ADAPTED_MINUTES);
    assert!(adapted.traits.contains(AcquiredTrait::ProstheticAdapted));
    assert_eq!(
        wooden.effective_part_function_basis_points(&anatomy, "cat-1", part, &adapted),
        9_000
    );

    assert_eq!(
        wooden.effective_part_function_basis_points(
            &CatAnatomy::default(),
            "cat-1",
            part,
            &adapted
        ),
        10_000,
        "the prosthetic layer must not replace natural anatomy calculation"
    );
}

#[test]
fn fitting_is_sided_atomic_and_refusal_or_bad_preconditions_consume_nothing() {
    let part = BodyPart::LeftEye;
    let id = ProstheticId::derive("colony-1", part, ProstheticMaterial::Metal, 7);
    let mut ledger = ProstheticLedger::default();
    ledger
        .register(id.clone(), ProstheticMaterial::Metal, part, "colony-1")
        .unwrap();
    let before = ledger.clone();

    let mut refused = fit_auth(part);
    refused.patient_consents = false;
    assert_eq!(
        ledger.begin_fitting(&id, &missing(part), refused),
        Err(ProstheticError::PatientRefused)
    );
    assert_eq!(ledger, before);

    let mut unreachable = fit_auth(part);
    unreachable.site_reachable = false;
    assert_eq!(
        ledger.begin_fitting(&id, &missing(part), unreachable),
        Err(ProstheticError::SiteUnreachable)
    );
    assert_eq!(ledger, before);

    assert_eq!(
        ledger.begin_fitting(
            &id,
            &missing(BodyPart::RightEye),
            fit_auth(BodyPart::RightEye)
        ),
        Err(ProstheticError::WrongPart)
    );
    assert_eq!(ledger, before);

    ledger
        .begin_fitting(&id, &missing(part), fit_auth(part))
        .unwrap();
    assert!(!ledger.trade_eligible(&id));
    ledger.cancel_reservation(&id).unwrap();
    assert_eq!(ledger, before);
    ledger
        .begin_fitting(&id, &missing(part), fit_auth(part))
        .unwrap();
    ledger.complete_fitting(&id).unwrap();
    assert_eq!(ledger.fitted_item("cat-1", part), Some(&id));
    assert_eq!(ledger.item_count(), 1);
}

#[test]
fn two_reserved_items_cannot_complete_into_one_anatomy_slot() {
    let part = BodyPart::FrontRightPaw;
    let anatomy = missing(part);
    let first = ProstheticId::derive("colony-1", part, ProstheticMaterial::Wooden, 1);
    let second = ProstheticId::derive("colony-1", part, ProstheticMaterial::Metal, 2);
    let mut ledger = ProstheticLedger::default();
    ledger
        .register(first.clone(), ProstheticMaterial::Wooden, part, "colony-1")
        .unwrap();
    ledger
        .register(second.clone(), ProstheticMaterial::Metal, part, "colony-1")
        .unwrap();
    ledger
        .begin_fitting(&first, &anatomy, fit_auth(part))
        .unwrap();
    let mut second_auth = fit_auth(part);
    second_auth.reservation_id = "fit-2";
    ledger
        .begin_fitting(&second, &anatomy, second_auth)
        .unwrap();
    ledger.complete_fitting(&first).unwrap();
    assert_eq!(
        ledger.complete_fitting(&second),
        Err(ProstheticError::SlotOccupied)
    );
    assert_eq!(ledger.fitted_item("cat-1", part), Some(&first));
    assert_eq!(ledger.item_count(), 2);
}

#[test]
fn wear_is_affected_work_only_batch_invariant_and_break_restores_missing_function() {
    let part = BodyPart::Tail;
    let anatomy = missing(part);
    let (mut batched, id) = fitted_ledger(ProstheticMaterial::Wooden, part);
    let mut partitioned = batched.clone();
    let mut batched_traits = AcquiredTraitState::default();
    let mut partitioned_traits = AcquiredTraitState::default();

    assert_eq!(
        batched.record_affected_work("cat-1", BodyPart::FrontLeftPaw, 99, &mut batched_traits),
        0
    );
    assert_eq!(
        batched.remaining_durability_minutes(&id),
        Some(WOODEN_DURABILITY_MINUTES)
    );
    assert_eq!(
        batched.record_affected_work(
            "cat-1",
            part,
            WOODEN_DURABILITY_MINUTES,
            &mut batched_traits
        ),
        WOODEN_DURABILITY_MINUTES
    );
    for _ in 0..360 {
        partitioned.record_affected_work("cat-1", part, 60, &mut partitioned_traits);
    }
    assert_eq!(batched, partitioned);
    assert_eq!(
        serde_json::to_value(&batched_traits).unwrap(),
        serde_json::to_value(&partitioned_traits).unwrap()
    );
    assert_eq!(
        batched.effective_part_function_basis_points(&anatomy, "cat-1", part, &batched_traits),
        0
    );
    assert_eq!(batched.remaining_durability_minutes(&id), Some(0));
}

#[test]
fn repair_requires_workshop_and_finite_inputs_and_never_mints_a_replacement() {
    let part = BodyPart::HindRightPaw;
    let (mut ledger, id) = fitted_ledger(ProstheticMaterial::Metal, part);
    ledger.record_affected_work(
        "cat-1",
        part,
        METAL_DURABILITY_MINUTES,
        &mut AcquiredTraitState::default(),
    );
    ledger.unfit(&id).unwrap();
    let before = ledger.clone();
    assert_eq!(
        ledger.begin_fitting(&id, &missing(part), fit_auth(part)),
        Err(ProstheticError::Broken)
    );
    assert_eq!(ledger, before);
    let mut auth = RepairAuthorization {
        colony_id: "colony-1",
        reservation_id: "repair-1",
        workshop_id: "workshop-1",
        workshop_reachable: true,
        finite_inputs_authorized: false,
    };
    assert_eq!(
        ledger.begin_repair(&id, auth),
        Err(ProstheticError::FiniteInputsNotAuthorized)
    );
    assert_eq!(ledger, before);
    auth.finite_inputs_authorized = true;
    ledger.begin_repair(&id, auth).unwrap();
    assert_eq!(ledger.item_count(), 1);
    assert!(!ledger.trade_eligible(&id));
    ledger.cancel_reservation(&id).unwrap();
    assert_eq!(ledger.remaining_durability_minutes(&id), Some(0));
    ledger.begin_repair(&id, auth).unwrap();
    ledger.complete_repair(&id).unwrap();
    assert_eq!(
        ledger.remaining_durability_minutes(&id),
        Some(METAL_DURABILITY_MINUTES)
    );
    assert_eq!(ledger.item_ids().collect::<Vec<_>>(), vec![&id]);
}

#[test]
fn restart_preserves_each_active_physical_transition_without_copying_identity() {
    fn restart(state: &ProstheticLedger) -> ProstheticLedger {
        serde_json::from_value(serde_json::to_value(state).unwrap()).unwrap()
    }

    let part = BodyPart::HindLeftPaw;
    let (mut ledger, id) = {
        let mut state = ProstheticLedger::default();
        let item_id = ProstheticId::derive("colony-1", part, ProstheticMaterial::Wooden, 91);
        state
            .register(
                item_id.clone(),
                ProstheticMaterial::Wooden,
                part,
                "colony-1",
            )
            .unwrap();
        (state, item_id)
    };
    ledger
        .begin_fitting(&id, &missing(part), fit_auth(part))
        .unwrap();
    assert_eq!(restart(&ledger), ledger);
    ledger.complete_fitting(&id).unwrap();
    assert_eq!(restart(&ledger), ledger);
    ledger.record_affected_work(
        "cat-1",
        part,
        WOODEN_DURABILITY_MINUTES,
        &mut AcquiredTraitState::default(),
    );
    ledger.unfit(&id).unwrap();
    let repair = RepairAuthorization {
        colony_id: "colony-1",
        reservation_id: "repair-91",
        workshop_id: "workshop-1",
        workshop_reachable: true,
        finite_inputs_authorized: true,
    };
    ledger.begin_repair(&id, repair).unwrap();
    assert_eq!(restart(&ledger), ledger);
    ledger.cancel_reservation(&id).unwrap();
    ledger.begin_trade(&id, "caravan-91").unwrap();
    assert_eq!(restart(&ledger), ledger);
    assert_eq!(ledger.item_count(), 1);
    assert_eq!(ledger.item_ids().next(), Some(&id));
}

#[test]
fn death_and_trade_preserve_identity_and_reject_fitted_or_reserved_items() {
    let part = BodyPart::RightEye;
    let (mut ledger, id) = fitted_ledger(ProstheticMaterial::Metal, part);
    assert_eq!(
        ledger.begin_trade(&id, "caravan-1"),
        Err(ProstheticError::NotInInventory)
    );
    assert_eq!(ledger.recover_from_death("cat-1"), vec![id.clone()]);
    assert_eq!(
        ledger.location(&id),
        Some(&ProstheticLocation::Inventory {
            colony_id: "colony-1".into()
        })
    );
    ledger.begin_trade(&id, "caravan-1").unwrap();
    assert!(!ledger.trade_eligible(&id));
    ledger.cancel_trade(&id).unwrap();
    ledger.begin_trade(&id, "caravan-2").unwrap();
    ledger.complete_trade(&id, "colony-2").unwrap();
    assert_eq!(
        ledger.location(&id),
        Some(&ProstheticLocation::Inventory {
            colony_id: "colony-2".into()
        })
    );
    assert_eq!(ledger.item_count(), 1);
}

#[test]
fn restart_and_input_order_twins_conserve_one_id_and_strictly_validate_state() {
    let ids = [
        ProstheticId::derive(
            "colony-1",
            BodyPart::FrontLeftPaw,
            ProstheticMaterial::Wooden,
            2,
        ),
        ProstheticId::derive("colony-1", BodyPart::RightEye, ProstheticMaterial::Metal, 1),
    ];
    let mut forward = ProstheticLedger::default();
    let mut reverse = ProstheticLedger::default();
    for index in [0, 1] {
        let part = [BodyPart::FrontLeftPaw, BodyPart::RightEye][index];
        let material = [ProstheticMaterial::Wooden, ProstheticMaterial::Metal][index];
        forward
            .register(ids[index].clone(), material, part, "colony-1")
            .unwrap();
    }
    for index in [1, 0] {
        let part = [BodyPart::FrontLeftPaw, BodyPart::RightEye][index];
        let material = [ProstheticMaterial::Wooden, ProstheticMaterial::Metal][index];
        reverse
            .register(ids[index].clone(), material, part, "colony-1")
            .unwrap();
    }
    let encoded = serde_json::to_string(&forward).unwrap();
    assert_eq!(encoded, serde_json::to_string(&reverse).unwrap());
    let restarted: ProstheticLedger = serde_json::from_str(&encoded).unwrap();
    assert_eq!(restarted, forward);
    assert_eq!(restarted.item_count(), 2);
    assert_eq!(
        serde_json::from_str::<ProstheticLedger>("{}").unwrap(),
        ProstheticLedger::default()
    );

    let unknown = encoded.replacen("{", "{\"unknown\":1,", 1);
    assert!(serde_json::from_str::<ProstheticLedger>(&unknown).is_err());
    let duplicate = serde_json::json!({
        "schemaVersion": 1,
        "items": [
            {"id": ids[0], "material": "wooden", "part": "front_left_paw", "durabilityMinutes": WOODEN_DURABILITY_MINUTES, "rehabilitationStages": 0, "location": {"kind":"inventory", "colonyId":"colony-1"}},
            {"id": ids[0], "material": "wooden", "part": "front_left_paw", "durabilityMinutes": WOODEN_DURABILITY_MINUTES, "rehabilitationStages": 0, "location": {"kind":"inventory", "colonyId":"colony-1"}}
        ],
        "fittedSlots": []
    });
    assert!(serde_json::from_value::<ProstheticLedger>(duplicate).is_err());

    let mut wrong_version = serde_json::to_value(&forward).unwrap();
    wrong_version["schemaVersion"] = serde_json::json!(2);
    assert!(serde_json::from_value::<ProstheticLedger>(wrong_version).is_err());

    let mut impossible_durability = serde_json::to_value(&forward).unwrap();
    impossible_durability["items"][0]["durabilityMinutes"] =
        serde_json::json!(METAL_DURABILITY_MINUTES + 1);
    assert!(serde_json::from_value::<ProstheticLedger>(impossible_durability).is_err());

    let (fitted, _) = fitted_ledger(ProstheticMaterial::Wooden, BodyPart::Tail);
    let mut orphaned_fitted = serde_json::to_value(fitted).unwrap();
    orphaned_fitted["fittedSlots"] = serde_json::json!([]);
    assert!(serde_json::from_value::<ProstheticLedger>(orphaned_fitted).is_err());
}
