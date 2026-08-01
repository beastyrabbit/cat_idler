//! Focused LAI.6 acceptance tests for anatomy, incidents, and treatment.

use std::collections::BTreeMap;

use cat_sim::{
    anatomy::{
        BASIS_POINTS_FULL_FUNCTION, BodyPart, BodyPartCondition, CapabilityBlock, CatAnatomy,
        HazardousJob, MINOR_TREATMENT_MINUTES, SEVERE_TREATMENT_MINUTES, TreatmentTransition,
    },
    injuries::{
        HazardousWorkUnit, IncidentIdentity, IncidentResolution, InjuryOutcome, InjuryRolls,
        classify_outcome, incident_occurs, injury_rolls, resolve_incident_with_rolls,
    },
};

#[test]
fn anatomy_tracks_four_paws_two_eyes_and_tail_with_legacy_healthy_defaults() {
    assert_eq!(BodyPart::ALL.len(), 7);
    assert_eq!(
        BodyPart::ALL,
        [
            BodyPart::FrontLeftPaw,
            BodyPart::FrontRightPaw,
            BodyPart::HindLeftPaw,
            BodyPart::HindRightPaw,
            BodyPart::LeftEye,
            BodyPart::RightEye,
            BodyPart::Tail,
        ]
    );

    let anatomy: CatAnatomy = serde_json::from_str("{}").unwrap();
    for part in BodyPart::ALL {
        assert_eq!(anatomy.part(part).condition, BodyPartCondition::Healthy);
        assert_eq!(anatomy.part(part).treatment_minutes, 0);
    }
    assert_eq!(
        anatomy.paw_function_basis_points(),
        BASIS_POINTS_FULL_FUNCTION
    );

    let partial: CatAnatomy = serde_json::from_value(serde_json::json!({
        "leftEye": { "condition": "minor" }
    }))
    .unwrap();
    assert_eq!(
        partial.part(BodyPart::LeftEye).condition,
        BodyPartCondition::Minor
    );
    assert_eq!(
        partial.part(BodyPart::RightEye).condition,
        BodyPartCondition::Healthy
    );
    assert!(
        serde_json::from_value::<CatAnatomy>(serde_json::json!({
            "tail": { "condition": "missing", "treatmentMinutes": 1 }
        }))
        .is_err()
    );
}

#[test]
fn injury_identity_and_tick_are_persisted_without_fabricated_projection_values() {
    let mut anatomy = CatAnatomy::default();
    anatomy
        .part_mut(BodyPart::Tail)
        .record_incident("incident:tail:7", 7);
    let restored: CatAnatomy =
        serde_json::from_str(&serde_json::to_string(&anatomy).unwrap()).unwrap();
    assert_eq!(
        restored.part(BodyPart::Tail).injury_id.as_deref(),
        Some("incident:tail:7")
    );
    assert_eq!(restored.part(BodyPart::Tail).injured_at_tick, Some(7));
}

#[test]
fn part_and_group_function_use_exact_integer_percentages() {
    assert_eq!(BodyPartCondition::Healthy.function_basis_points(), 10_000);
    assert_eq!(BodyPartCondition::Minor.function_basis_points(), 8_500);
    assert_eq!(BodyPartCondition::Severe.function_basis_points(), 5_000);
    assert_eq!(BodyPartCondition::Missing.function_basis_points(), 0);

    let mut anatomy = CatAnatomy::default();
    anatomy.injure(BodyPart::FrontLeftPaw, BodyPartCondition::Missing);
    anatomy.injure(BodyPart::LeftEye, BodyPartCondition::Missing);
    anatomy.injure(BodyPart::Tail, BodyPartCondition::Minor);

    assert_eq!(anatomy.paw_function_basis_points(), 7_500);
    assert_eq!(anatomy.eye_function_basis_points(), 5_000);
    assert_eq!(anatomy.tail_function_basis_points(), 8_500);
    assert_eq!(anatomy.movement_function_basis_points(), 7_600);
    assert_eq!(anatomy.combat_function_basis_points(), 7_600);
    assert_eq!(anatomy.physical_labor_function_basis_points(), 7_500);
    assert_eq!(anatomy.vision_function_basis_points(), 5_000);
    assert_eq!(anatomy.scouting_function_basis_points(), 5_000);
    assert_eq!(anatomy.hunting_function_basis_points(), 5_000);
    assert_eq!(anatomy.ranged_combat_function_basis_points(), 5_350);
}

#[test]
fn hazardous_job_capability_uses_relevant_parts_and_blocks_severe_work() {
    let mut anatomy = CatAnatomy::default();
    anatomy.injure(BodyPart::LeftEye, BodyPartCondition::Severe);

    let scout = anatomy.job_capability(HazardousJob::Scout);
    assert_eq!(scout.task_function_basis_points, 7_500);
    assert_eq!(scout.blocked, Some(CapabilityBlock::Eye));

    let quarry = anatomy.job_capability(HazardousJob::Quarry);
    assert_eq!(quarry.task_function_basis_points, 10_000);
    assert_eq!(quarry.blocked, None);

    anatomy.injure(BodyPart::HindRightPaw, BodyPartCondition::Severe);
    assert_eq!(
        anatomy.job_capability(HazardousJob::Construction).blocked,
        Some(CapabilityBlock::Paw)
    );
    assert_eq!(
        anatomy.job_capability(HazardousJob::Raid).blocked,
        Some(CapabilityBlock::Paw)
    );
}

#[test]
fn incident_and_outcome_probability_matrices_are_exact() {
    let expected = [
        (HazardousWorkUnit::Scout, 150),
        (HazardousWorkUnit::Hunt, 100),
        (HazardousWorkUnit::Quarry, 80),
        (HazardousWorkUnit::Logging, 50),
        (HazardousWorkUnit::Construction, 30),
        (HazardousWorkUnit::RaidVictory, 500),
        (HazardousWorkUnit::RaidDefeat, 2_000),
    ];
    for (work, expected_count) in expected {
        let count = (0..10_000)
            .filter(|bucket| incident_occurs(work, *bucket))
            .count();
        assert_eq!(work.incident_basis_points(), expected_count);
        assert_eq!(count, usize::from(expected_count));
    }

    let mut counts = BTreeMap::new();
    for bucket in 0..10_000 {
        *counts.entry(classify_outcome(bucket)).or_insert(0usize) += 1;
    }
    assert_eq!(counts[&InjuryOutcome::Minor], 7_000);
    assert_eq!(counts[&InjuryOutcome::Severe], 2_000);
    assert_eq!(counts[&InjuryOutcome::Missing], 800);
    assert_eq!(counts[&InjuryOutcome::Fatal], 200);
}

#[test]
fn injury_rng_is_keyed_and_independent_of_batch_order() {
    assert_ne!(
        IncidentIdentity::new("ab", "c", 1, 2).stable_id(),
        IncidentIdentity::new("a", "bc", 1, 2).stable_id(),
        "length-prefixed incident IDs must not have delimiter ambiguity"
    );
    let identities = (0..128)
        .map(|index| IncidentIdentity::new("cat-7", "task-hunt-4", index, 90_000 + index))
        .collect::<Vec<_>>();
    let forward = identities
        .iter()
        .map(|identity| (identity.work_unit_index, injury_rolls(42, identity)))
        .collect::<BTreeMap<_, _>>();
    let reverse = identities
        .iter()
        .rev()
        .map(|identity| (identity.work_unit_index, injury_rolls(42, identity)))
        .collect::<BTreeMap<_, _>>();

    assert_eq!(forward, reverse);
    assert_eq!(
        injury_rolls(42, &identities[17]),
        injury_rolls(42, &identities[17])
    );
    assert_ne!(
        injury_rolls(42, &identities[17]),
        injury_rolls(42, &identities[18])
    );
}

#[test]
fn minor_and_severe_treatment_complete_at_exact_partition_invariant_hours() {
    let mut one_step = CatAnatomy::default();
    one_step.injure(BodyPart::FrontLeftPaw, BodyPartCondition::Minor);
    assert_eq!(
        one_step.treat(BodyPart::FrontLeftPaw, MINOR_TREATMENT_MINUTES - 1),
        TreatmentTransition::InProgress
    );
    assert_eq!(
        one_step.treat(BodyPart::FrontLeftPaw, 1),
        TreatmentTransition::Healed
    );

    let mut partitioned = CatAnatomy::default();
    partitioned.injure(BodyPart::RightEye, BodyPartCondition::Severe);
    for _ in 0..48 {
        partitioned.treat(BodyPart::RightEye, 60);
    }
    let mut batched = CatAnatomy::default();
    batched.injure(BodyPart::RightEye, BodyPartCondition::Severe);
    assert_eq!(
        batched.treat(BodyPart::RightEye, SEVERE_TREATMENT_MINUTES),
        TreatmentTransition::Healed
    );
    assert_eq!(partitioned, batched);
    assert_eq!(
        batched.part(BodyPart::RightEye).condition,
        BodyPartCondition::Healthy
    );

    batched.injure(BodyPart::Tail, BodyPartCondition::Missing);
    assert_eq!(
        batched.treat(BodyPart::Tail, u32::MAX),
        TreatmentTransition::NotTreatable
    );
    assert_eq!(
        batched.part(BodyPart::Tail).condition,
        BodyPartCondition::Missing
    );
}

#[test]
fn fatal_outcome_is_explicit_and_does_not_mutate_anatomy() {
    let identity = IncidentIdentity::new("cat-1", "raid-9", 3, 400);
    let mut anatomy = CatAnatomy::default();
    let before = anatomy.clone();
    let resolution = resolve_incident_with_rolls(
        &mut anatomy,
        HazardousWorkUnit::RaidDefeat,
        &identity,
        InjuryRolls {
            incident_bucket: 0,
            outcome_bucket: 9_999,
            part_selector: 5,
        },
    );

    assert_eq!(
        resolution,
        IncidentResolution::Fatal {
            incident_id: identity.stable_id()
        }
    );
    assert_eq!(anatomy, before);
}

#[test]
fn incident_resolution_observes_rate_boundary_and_applies_one_eligible_part() {
    let identity = IncidentIdentity::new("cat-2", "build-7", 1, 900);
    let mut anatomy = CatAnatomy::default();
    let no_incident = resolve_incident_with_rolls(
        &mut anatomy,
        HazardousWorkUnit::Construction,
        &identity,
        InjuryRolls {
            incident_bucket: 30,
            outcome_bucket: 0,
            part_selector: 6,
        },
    );
    assert_eq!(no_incident, IncidentResolution::NoIncident);

    let incident = resolve_incident_with_rolls(
        &mut anatomy,
        HazardousWorkUnit::Construction,
        &identity,
        InjuryRolls {
            incident_bucket: 29,
            outcome_bucket: 6_999,
            part_selector: 6,
        },
    );
    assert!(matches!(
        incident,
        IncidentResolution::Injured {
            part: BodyPart::Tail,
            outcome: InjuryOutcome::Minor,
            previous: BodyPartCondition::Healthy,
            current: BodyPartCondition::Minor,
            ..
        }
    ));
    assert_eq!(
        anatomy.part(BodyPart::Tail).condition,
        BodyPartCondition::Minor
    );
}
