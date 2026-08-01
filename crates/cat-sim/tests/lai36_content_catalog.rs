//! LAI.36 red contract for the unified, validated content manifest.
//!
//! This specifies the data authority from Plan 1 sections 3–4 and P1.12,
//! P1.13, P1.17, P1.18, and P1.29. Quality remains a downstream LAI.37
//! authority and is intentionally not defined or asserted by this contract.

use std::{collections::BTreeSet, str::FromStr};

use cat_sim::content_manifest::{
    ArtKey, CapabilityId, CapabilityRequirement, ContentId, ContentManifest, ContentOperation,
    CreatureId, FoodId, ItemDefinitionId, MaterialId, MaterialInstanceId, PLAN1_BREW_RECIPE_IDS,
    PLAN1_COOKHOUSE_RECIPE_IDS, PLAN1_CREATURE_IDS, PLAN1_RARE_MATERIAL_IDS, PhysicalLotId,
    RecipeId, ResourceId, StableId,
};
use serde_json::Value;

fn embedded() -> ContentManifest {
    ContentManifest::embedded().clone()
}

fn validation_messages(manifest: &ContentManifest) -> Vec<String> {
    manifest
        .validate()
        .expect_err("broken test manifest must fail validation")
        .into_iter()
        .map(|error| error.to_string())
        .collect()
}

macro_rules! assert_stable_id_contract {
    ($id:ty) => {{
        let valid = <$id>::from_str("a_0").expect("lowercase stable ID is valid");
        assert!(
            <$id>::from_str(&"a".repeat(64)).is_ok(),
            "64 bytes is valid"
        );
        assert!(
            <$id>::from_str("a__").is_ok(),
            "underscores remain valid after the first byte"
        );
        assert_eq!(valid.as_str(), "a_0");
        assert_eq!(serde_json::to_string(&valid).unwrap(), r#""a_0""#);
        assert_eq!(
            serde_json::from_str::<$id>(r#""a_0""#).unwrap(),
            valid,
            "stable IDs use a strict JSON string representation"
        );
        let too_long = "a".repeat(65);
        for invalid in ["", "A", "0start", "has-dash", "has space", "é", &too_long] {
            assert!(
                <$id>::from_str(invalid).is_err(),
                "{invalid:?} must be rejected"
            );
            assert!(serde_json::from_str::<$id>(&format!(r#""{invalid}""#)).is_err());
        }
    }};
}

#[test]
fn lai36_all_stable_id_newtypes_enforce_the_one_wire_grammar() {
    assert_stable_id_contract!(ContentId);
    assert_stable_id_contract!(ResourceId);
    assert_stable_id_contract!(FoodId);
    assert_stable_id_contract!(ItemDefinitionId);
    assert_stable_id_contract!(MaterialId);
    assert_stable_id_contract!(CreatureId);
    assert_stable_id_contract!(RecipeId);
    assert_stable_id_contract!(CapabilityId);
    assert_stable_id_contract!(ArtKey);
    assert_stable_id_contract!(PhysicalLotId);
    assert_stable_id_contract!(MaterialInstanceId);
}

#[test]
fn lai36_embedded_manifest_is_the_complete_single_content_authority() {
    let manifest = embedded();
    let summary = manifest.validate().expect("embedded manifest validates");

    assert!(summary.resource_total > 0);
    assert!(summary.food_total > 0);
    assert!(summary.item_definition_total > 0);
    assert!(summary.material_total > 0);
    assert_eq!(
        summary.creature_total, 20,
        "the full hunting roster is manifest-owned"
    );
    assert_eq!(
        manifest
            .creatures
            .iter()
            .map(|creature| creature.id.as_str())
            .collect::<Vec<_>>(),
        PLAN1_CREATURE_IDS.to_vec()
    );
    assert!(
        PLAN1_RARE_MATERIAL_IDS.iter().all(|id| manifest
            .materials
            .iter()
            .any(|material| material.id.as_str() == *id)),
        "every named creature drop has a manifest material record"
    );
    assert!(summary.station_total > 0);
    assert!(summary.recipe_total >= 18);
    assert!(
        manifest
            .recipes
            .iter()
            .any(|recipe| recipe.id.as_str() == "mill_flour")
    );
    assert_eq!(
        manifest
            .recipes
            .iter()
            .filter(|recipe| PLAN1_COOKHOUSE_RECIPE_IDS.contains(&recipe.id.as_str()))
            .count(),
        18,
        "the Cookhouse owns the approved eighteen-recipe meal table"
    );
    assert!(PLAN1_COOKHOUSE_RECIPE_IDS.iter().all(|id| {
        manifest
            .recipes
            .iter()
            .any(|recipe| recipe.id.as_str() == *id && recipe.station.as_str() == "cookhouse")
    }));
    assert!(PLAN1_BREW_RECIPE_IDS.iter().all(|id| {
        manifest
            .recipes
            .iter()
            .any(|recipe| recipe.id.as_str() == *id && recipe.station.as_str() == "cookhouse")
    }));
    assert_eq!(
        manifest
            .recipes
            .iter()
            .filter(|recipe| recipe.station.as_str() == "cookhouse")
            .count(),
        PLAN1_COOKHOUSE_RECIPE_IDS.len() + PLAN1_BREW_RECIPE_IDS.len(),
        "the Cookhouse owns the exact meal catalog plus the five moved brewing recipes"
    );
    assert!(summary.augmentation_total > 0);
    assert!(summary.fixture_total > 0);
    assert_eq!(
        summary.capability_total,
        manifest.derived_capability_total()
    );

    let entries = manifest.canonical_content_entries();
    assert!(entries.windows(2).all(|pair| {
        (pair[0].order, pair[0].content_id.as_str()) < (pair[1].order, pair[1].content_id.as_str())
    }));
    assert!(entries.iter().any(|entry| entry.class == "food"));
    assert!(entries.iter().any(|entry| entry.class == "item_definition"));
    assert!(entries.iter().any(|entry| entry.class == "material"));
    assert!(entries.iter().any(|entry| entry.class == "creature"));
    assert!(entries.iter().any(|entry| entry.class == "recipe"));
    assert!(entries.iter().any(|entry| entry.class == "augmentation"));
    assert!(entries.iter().any(|entry| entry.class == "fixture"));

    assert!(manifest.foods.iter().all(|food| {
        (food.nutrition > 0 || food.hydration > 0)
            && food.weight_milli > 0
            && food.value_milli > 0
            && !food.art_key.as_str().is_empty()
    }));
    assert!(
        manifest
            .item_definitions
            .iter()
            .all(|item| !item.art_key.as_str().is_empty())
    );
    assert!(manifest.item_definitions.iter().any(|item| {
        !item.functions.is_empty() || !item.base_materials.is_empty() || !item.layers.is_empty()
    }));
    assert!(manifest.creatures.iter().all(|creature| {
        creature.level_min >= 1
            && creature.level_max <= 100
            && !creature.common_loot.is_empty()
            && !creature.portrait.as_str().is_empty()
    }));
    assert!(manifest.materials.iter().all(|material| {
        material.hole_darkness_gate <= 10
            && material.hole_value_milli > 0
            && !material.uses.is_empty()
            && !material.art_key.as_str().is_empty()
    }));
    assert!(manifest.stations.iter().all(|station| {
        station.min_tier > 0 && station.footprint_cells > 0 && !station.art_key.as_str().is_empty()
    }));
    assert!(manifest.recipes.iter().all(|recipe| {
        recipe.station_tier > 0
            && !recipe.ingredients.is_empty()
            && !recipe.outputs.is_empty()
            && !recipe.art_key.as_str().is_empty()
    }));
    assert!(manifest.augmentations.iter().all(|augmentation| {
        !augmentation.consumed_materials.is_empty()
            && !augmentation.compatible_item_classes.is_empty()
    }));
    assert!(manifest.fixtures.iter().all(|fixture| {
        !fixture.consumed_materials.is_empty() && !fixture.compatible_stations.is_empty()
    }));
    assert!(
        manifest
            .capabilities
            .iter()
            .all(|capability| !capability.payload.effect_handler.is_empty())
    );

    for recipe in &manifest.recipes {
        let bundle = manifest
            .capabilities
            .iter()
            .find(|capability| capability.id == recipe.bundle_capability)
            .expect("each recipe bundle names an owned capability");
        assert!(
            bundle.canonical_for.iter().any(|content_id| {
                manifest
                    .resources
                    .iter()
                    .any(|resource| resource.content_id == *content_id)
                    || manifest
                        .materials
                        .iter()
                        .any(|material| material.content_id == *content_id)
            }),
            "recipe bundles are owned by a resource or material capability"
        );
        assert!(
            manifest
                .capabilities
                .iter()
                .all(|capability| capability.id.as_str() != recipe.id.as_str()),
            "a recipe may not grow its own research node"
        );
    }
}

#[test]
fn lai36_encounter_bands_are_not_the_ten_public_lair_visual_bands() {
    let manifest = embedded();
    assert_eq!(
        manifest
            .lair_bands
            .iter()
            .map(|band| (band.band_min, band.band_max))
            .collect::<Vec<_>>(),
        vec![(1, 19), (20, 39), (40, 59), (60, 79), (80, 94), (95, 100)],
        "six encounter bands own roster and mystic rules"
    );
    assert_eq!(
        manifest
            .lair_bands
            .iter()
            .map(|band| band.mystic_required_from_level)
            .collect::<Vec<_>>(),
        vec![None, None, None, Some(61), Some(80), Some(95)],
        "level 60 remains mixed; mandatory mystic encounters begin exactly at level 61"
    );
    assert_eq!(
        manifest
            .lair_bands
            .iter()
            .map(|band| band.public_art_key.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        6
    );

    let visual_bands = manifest.public_lair_visual_bands();
    assert_eq!(
        visual_bands.len(),
        10,
        "world art always reveals ten-level bands"
    );
    assert_eq!(
        visual_bands
            .iter()
            .map(|band| band.level_range())
            .collect::<Vec<_>>(),
        vec![
            1..=10,
            11..=20,
            21..=30,
            31..=40,
            41..=50,
            51..=60,
            61..=70,
            71..=80,
            81..=90,
            91..=100
        ]
    );
    assert_eq!(
        visual_bands
            .iter()
            .map(|band| band.art_key().as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        10,
        "each public ten-level band resolves a distinct deterministic art key"
    );
}

#[test]
fn lai36_strict_decode_and_validation_reject_unknown_duplicate_dangling_cycle_range_handler_and_art()
 {
    let encoded = embedded().to_canonical_json();
    let mut unknown: Value = serde_json::from_str(&encoded).unwrap();
    unknown["unknownField"] = Value::Bool(true);
    assert!(ContentManifest::decode_strict(&unknown.to_string()).is_err());

    let mut duplicate = embedded();
    duplicate.resources.push(duplicate.resources[0].clone());
    assert!(
        validation_messages(&duplicate)
            .iter()
            .any(|message| message.contains("duplicate"))
    );

    let mut dangling = embedded();
    dangling.recipes[0].ingredients[0].content_id = ContentId::from_str("missing_content").unwrap();
    assert!(
        validation_messages(&dangling)
            .iter()
            .any(|message| message.contains("dangling"))
    );

    let mut cycle = embedded();
    let capability_id = cycle.capabilities[0].id.clone();
    cycle.capabilities[0].prerequisites.push(capability_id);
    assert!(
        validation_messages(&cycle)
            .iter()
            .any(|message| message.contains("cycle"))
    );

    let mut range = embedded();
    range.creatures[0].level_max = 101;
    assert!(
        validation_messages(&range)
            .iter()
            .any(|message| message.contains("range"))
    );

    let mut handler = embedded();
    handler.resources[0].behavior_handler = "unregistered_handler".to_owned();
    assert!(
        validation_messages(&handler)
            .iter()
            .any(|message| message.contains("handler"))
    );

    let mut art = embedded();
    art.foods[0].art_key = art.resources[0].art_key.clone();
    assert!(
        validation_messages(&art)
            .iter()
            .any(|message| message.contains("ArtKey"))
    );
}

#[test]
fn lai36_locked_content_stays_referenceable_but_capability_gates_mutating_uses() {
    let manifest = embedded();
    let locked = manifest
        .resources
        .iter()
        .find(|resource| {
            matches!(
                &resource.canonical_capability,
                CapabilityRequirement::Required(_)
            )
        })
        .expect("embedded manifest has locked content")
        .content_id
        .clone();
    let capabilities = BTreeSet::new();

    for operation in [
        ContentOperation::Discover,
        ContentOperation::Store,
        ContentOperation::Trade,
    ] {
        assert!(
            manifest
                .is_operation_permitted(&locked, operation, &capabilities)
                .expect("known content remains referenceable")
        );
    }
    for operation in [
        ContentOperation::Process,
        ContentOperation::Craft,
        ContentOperation::InstallFixture,
        ContentOperation::Augment,
        ContentOperation::FeedHole,
    ] {
        assert!(
            !manifest
                .is_operation_permitted(&locked, operation, &capabilities)
                .expect("known locked content has a defined denial")
        );
    }
    assert!(
        manifest
            .is_operation_permitted(
                &ContentId::from_str("unknown_content").unwrap(),
                ContentOperation::Store,
                &capabilities,
            )
            .is_err()
    );

    let required = match manifest
        .resources
        .iter()
        .find(|resource| resource.content_id == locked)
        .unwrap()
        .canonical_capability
        .clone()
    {
        CapabilityRequirement::Required(required) => required,
        CapabilityRequirement::Free => panic!("selected content is locked"),
    };
    let capabilities = [required].into_iter().collect::<BTreeSet<_>>();
    assert!(
        manifest
            .is_operation_permitted(&locked, ContentOperation::Process, &capabilities)
            .unwrap()
    );
}

#[test]
fn lai36_additive_content_permutations_have_one_canonical_order() {
    let mut additive = embedded();
    let mut added = additive.resources[0].clone();
    assert!(matches!(
        &added.canonical_capability,
        CapabilityRequirement::Free
    ));
    let next_order = additive
        .canonical_content_entries()
        .last()
        .expect("embedded manifest has content")
        .order
        + 10;
    added.id = ResourceId::from_str("z_additive_logs").unwrap();
    added.content_id = ContentId::from_str("z_additive_logs").unwrap();
    added.display_name = "Additive Logs".to_owned();
    added.order = next_order;
    added.art_key = ArtKey::from_str("art_z_additive_logs").unwrap();
    additive.resources.push(added);
    additive
        .validate()
        .expect("a data-only free resource is an additive extension");

    let canonical = additive.canonical_content_entries();
    assert_eq!(
        canonical.last().unwrap().content_id.as_str(),
        "z_additive_logs"
    );
    let mut permuted = additive.clone();
    let appended = permuted.resources.pop().unwrap();
    permuted.resources.insert(0, appended);
    assert!(
        validation_messages(&permuted)
            .iter()
            .any(|message| message.contains("non-monotonic stable order"))
    );
    assert_eq!(
        additive.to_canonical_json(),
        ContentManifest::decode_strict(&additive.to_canonical_json())
            .unwrap()
            .to_canonical_json(),
        "strict decoding preserves the canonical additive manifest"
    );
}
