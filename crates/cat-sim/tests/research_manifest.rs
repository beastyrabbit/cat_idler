//! LAI.19A focused research-manifest foundation tests.

use std::collections::BTreeSet;

use cat_sim::{
    research_catalog::{RESEARCH_NODE_COUNT, research_catalog},
    research_manifest::{
        ADDITIVE_TRACK_STAGE_COUNT, ADDITIVE_TRACK_STUDY_COUNT,
        ADMINISTRATION_BASE_STANDING_ORDER_SLOTS, ADMINISTRATION_BASE_STRATEGIC_INTENT_SLOTS,
        ADMINISTRATION_STAGE_STANDING_ORDER_SLOTS, ADMINISTRATION_STAGE_STRATEGIC_INTENT_SLOTS,
        DEPRECATED_STUDY_IDS, DIVINE_DURATION_ALLOWED_GAME_HOURS,
        DIVINE_DURATION_STAGE_MAX_GAME_HOURS, DIVINE_ECONOMY_STAGE_DISCOUNT_BASIS_POINTS,
        LIVE_EFFECT_HANDLERS, ManifestEffect, ManifestStudySource, ManifestTrack,
        REHABILITATION_STAGE_RESTORATION_PERCENTAGE_POINTS, RESEARCH_MANIFEST_STUDY_COUNT,
        ResearchManifest, research_manifest,
    },
};

fn manifest_ids(manifest: &ResearchManifest) -> Vec<&str> {
    manifest
        .studies()
        .iter()
        .map(|study| study.stable_id.as_str())
        .collect()
}

#[test]
fn manifest_preserves_the_487_catalog_and_appends_exactly_44_track_studies() {
    let manifest = research_manifest();
    let catalog = research_catalog();
    assert_eq!(catalog.nodes().len(), RESEARCH_NODE_COUNT);
    assert_eq!(manifest.studies().len(), RESEARCH_MANIFEST_STUDY_COUNT);
    assert_eq!(
        RESEARCH_MANIFEST_STUDY_COUNT,
        RESEARCH_NODE_COUNT + ADDITIVE_TRACK_STUDY_COUNT
    );

    let catalog_ids = catalog
        .nodes()
        .iter()
        .map(|node| node.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(&manifest_ids(manifest)[..RESEARCH_NODE_COUNT], catalog_ids);
    assert_eq!(
        manifest_ids(manifest)[RESEARCH_NODE_COUNT],
        "divine_duration_stage_01"
    );
    assert_eq!(
        manifest_ids(manifest).last(),
        Some(&"administration_stage_11")
    );

    for track in ManifestTrack::ALL {
        let studies = manifest.track_studies(track);
        assert_eq!(studies.len(), ADDITIVE_TRACK_STAGE_COUNT, "{track:?}");
        assert_eq!(studies[0].prerequisites, [track.root_prerequisite()]);
        for (index, study) in studies.iter().enumerate() {
            assert_eq!(study.source, ManifestStudySource::AdditiveTrack(track));
            assert_eq!(study.stage, Some(u8::try_from(index + 1).unwrap()));
            assert_eq!(
                manifest
                    .get(&study.stable_id)
                    .map(|found| found.order_index),
                Some(study.order_index)
            );
        }
    }
}

#[test]
fn manifest_has_unique_stable_ids_and_display_names_with_permutation_stable_order() {
    let manifest = research_manifest();
    let ids = manifest_ids(manifest);
    assert_eq!(ids.iter().collect::<BTreeSet<_>>().len(), ids.len());
    let display_names = manifest
        .studies()
        .iter()
        .map(|study| study.display_name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        display_names.iter().collect::<BTreeSet<_>>().len(),
        display_names.len()
    );

    let reversed = manifest.studies().iter().cloned().rev().collect::<Vec<_>>();
    let twin = ResearchManifest::from_studies(reversed).unwrap();
    assert_eq!(manifest.studies(), twin.studies());
    assert_eq!(manifest_ids(manifest), manifest_ids(&twin));
}

#[test]
fn manifest_topology_is_acyclic_reachable_and_has_no_deprecated_orphans() {
    let manifest = research_manifest();
    assert_eq!(manifest.starting_frontier_ids(), ["research_hut"]);
    let reachable = manifest.reachable_study_ids();
    assert_eq!(reachable.len(), RESEARCH_MANIFEST_STUDY_COUNT);
    for study in manifest.studies() {
        assert!(reachable.contains(study.stable_id.as_str()));
        assert!(!DEPRECATED_STUDY_IDS.contains(&study.stable_id.as_str()));
        for prerequisite in &study.prerequisites {
            assert!(manifest.get(prerequisite).is_some(), "{prerequisite}");
            assert!(!DEPRECATED_STUDY_IDS.contains(&prerequisite.as_str()));
        }
    }

    let mut cyclic = manifest.studies().to_vec();
    let root = cyclic
        .iter_mut()
        .find(|study| study.stable_id == "research_hut")
        .unwrap();
    root.prerequisites
        .push("administration_stage_11".to_owned());
    assert!(ResearchManifest::from_studies(cyclic).is_err());
}

#[test]
fn every_manifest_effect_references_a_live_handler() {
    let live = LIVE_EFFECT_HANDLERS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let manifest = research_manifest();
    for study in manifest.studies() {
        assert!(!study.effects.is_empty(), "{}", study.stable_id);
        for effect in &study.effects {
            assert!(
                live.contains(&effect.handler()),
                "{} used {:?}",
                study.stable_id,
                effect.handler()
            );
        }
    }
}

#[test]
fn additive_track_effect_tables_match_the_shrine_favor_research_spec() {
    assert_eq!(
        DIVINE_DURATION_ALLOWED_GAME_HOURS,
        [1, 2, 3, 4, 6, 8, 10, 12, 16, 18, 21, 24]
    );
    assert_eq!(
        DIVINE_DURATION_STAGE_MAX_GAME_HOURS,
        [2, 3, 4, 6, 8, 10, 12, 16, 18, 21, 24]
    );
    assert_eq!(
        DIVINE_ECONOMY_STAGE_DISCOUNT_BASIS_POINTS,
        [
            300, 600, 900, 1_200, 1_500, 1_800, 2_100, 2_400, 2_700, 3_000, 3_300
        ]
    );
    assert_eq!(
        REHABILITATION_STAGE_RESTORATION_PERCENTAGE_POINTS,
        [2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22]
    );
    assert_eq!(ADMINISTRATION_BASE_STANDING_ORDER_SLOTS, 3);
    assert_eq!(ADMINISTRATION_BASE_STRATEGIC_INTENT_SLOTS, 4);
    assert_eq!(
        ADMINISTRATION_STAGE_STANDING_ORDER_SLOTS,
        [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]
    );
    assert_eq!(
        ADMINISTRATION_STAGE_STRATEGIC_INTENT_SLOTS,
        [4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9]
    );

    let manifest = research_manifest();
    for (index, study) in manifest
        .track_studies(ManifestTrack::DivineDuration)
        .iter()
        .enumerate()
    {
        assert_eq!(
            study.effects,
            [ManifestEffect::DivineDuration {
                stage: u8::try_from(index + 1).unwrap(),
                max_duration_game_hours: DIVINE_DURATION_STAGE_MAX_GAME_HOURS[index],
            }]
        );
    }
    for (index, study) in manifest
        .track_studies(ManifestTrack::DivineEconomy)
        .iter()
        .enumerate()
    {
        assert_eq!(
            study.effects,
            [ManifestEffect::DivineEconomy {
                stage: u8::try_from(index + 1).unwrap(),
                discount_basis_points: DIVINE_ECONOMY_STAGE_DISCOUNT_BASIS_POINTS[index],
            }]
        );
    }
    for (index, study) in manifest
        .track_studies(ManifestTrack::Rehabilitation)
        .iter()
        .enumerate()
    {
        assert_eq!(
            study.effects,
            [ManifestEffect::Rehabilitation {
                stage: u8::try_from(index + 1).unwrap(),
                restoration_bonus_percentage_points:
                    REHABILITATION_STAGE_RESTORATION_PERCENTAGE_POINTS[index],
            }]
        );
    }
    for (index, study) in manifest
        .track_studies(ManifestTrack::Administration)
        .iter()
        .enumerate()
    {
        assert_eq!(
            study.effects,
            [ManifestEffect::Administration {
                stage: u8::try_from(index + 1).unwrap(),
                standing_order_slots: ADMINISTRATION_STAGE_STANDING_ORDER_SLOTS[index],
                strategic_intent_slots: ADMINISTRATION_STAGE_STRATEGIC_INTENT_SLOTS[index],
            }]
        );
    }
}
