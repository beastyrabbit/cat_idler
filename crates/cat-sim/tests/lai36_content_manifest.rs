//! LAI.36 focused contract for the single production-content authority.

#[path = "../src/content_manifest.rs"]
mod content_manifest;

use std::{collections::BTreeSet, str::FromStr};

use cat_sim::{
    items::ItemKind, station_recipes::station_recipe_set, stockpiles::ResourceKind,
    types::BuildingType,
};
use content_manifest::{
    AccessibilityBinding, ArtAssetDescriptor, ArtKey, ArtLayer, AugmentationSlot, AuthorityDomain,
    COMPILED_HANDLER_REGISTRY, CURRENT_MILL_RECIPE_CUTOVER_TOTAL, CURRENT_MILL_RECIPE_TOTAL,
    CURRENT_RUNTIME_RECIPE_CUTOVER_TOTAL, CapabilityId, CapabilityRequirement, ContentId,
    ContentManifest, ContentOperation, CreatureId, CutoverCard, CutoverDisposition,
    EffectOperation, EquipmentSlot, FixtureSlot, FoodId, HOLE_AXIS_COUNT, ItemClass,
    ItemDefinitionId, MaterialId, MaterialInstanceId, PERSISTED_COMBINED_MILL_RECIPE_ALIAS,
    PLAN1_BREW_RECIPE_IDS, PLAN1_COOKHOUSE_RECIPE_IDS, PLAN1_CREATURE_IDS, PLAN1_RARE_MATERIAL_IDS,
    PRE_CUTOVER_RUNTIME_RECIPE_IDS, PRE_CUTOVER_RUNTIME_RECIPE_TOTAL, PhysicalLotId,
    RECIPE_CUTOVER_RECEIPT_TOTAL, REQUIRED_FOUNDING_CAPABILITIES,
    RETAINED_PRE_CUTOVER_RECIPE_TOTAL, RecipeId, ResourceId, StationBehavior, TaskCategory,
    UNCHANGED_RECIPE_FLOW_ALLOWLIST, ValidationFailure, ValidationPhase, compiled_handler,
};

fn embedded() -> ContentManifest {
    ContentManifest::embedded().clone()
}

fn messages(manifest: &ContentManifest) -> Vec<String> {
    manifest
        .validate()
        .expect_err("mutated manifest must fail")
        .into_iter()
        .map(|error| error.to_string())
        .collect()
}

fn assert_failure(manifest: &ContentManifest, phase: ValidationPhase, failure: ValidationFailure) {
    let errors = manifest.validate().expect_err("mutated manifest must fail");
    assert!(
        errors
            .iter()
            .any(|error| error.phase == phase && error.failure == failure),
        "missing {phase:?}/{failure:?}; got {errors:#?}"
    );
}

fn resource_content_id(resource: ResourceKind) -> Option<&'static str> {
    match resource {
        ResourceKind::Herbs => Some("resource_herbs"),
        ResourceKind::Catnip => Some("food_catnip"),
        ResourceKind::Grain => Some("resource_grain"),
        ResourceKind::Flour => Some("resource_flour"),
        ResourceKind::Medicine => Some("resource_medicine"),
        ResourceKind::Brew => Some("food_brew"),
        ResourceKind::Materials => Some("resource_refined"),
        ResourceKind::Stone => Some("resource_stone"),
        ResourceKind::Refined => Some("resource_refined"),
        ResourceKind::Weapons => Some("item_weapon"),
        ResourceKind::Armor => Some("item_armor"),
        ResourceKind::Logs => Some("resource_logs"),
        ResourceKind::Lumber => Some("resource_lumber"),
        ResourceKind::Planks => Some("resource_planks"),
        ResourceKind::Blocks => Some("resource_blocks"),
        ResourceKind::Tools => Some("item_generic_tool"),
        ResourceKind::Fibre => Some("resource_fibre"),
        ResourceKind::Thread => Some("resource_thread"),
        ResourceKind::Hide => Some("resource_hide"),
        ResourceKind::Bone => Some("resource_bone"),
        ResourceKind::Cloth => Some("resource_cloth"),
        ResourceKind::Leather => Some("resource_leather"),
        ResourceKind::Ore => Some("resource_ore"),
        ResourceKind::Gem => Some("resource_gem"),
        ResourceKind::Clay => Some("resource_clay"),
        ResourceKind::Sand => Some("resource_sand"),
        ResourceKind::Metal => Some("resource_metal"),
        ResourceKind::Food
        | ResourceKind::Fish
        | ResourceKind::Water
        | ResourceKind::Preserves
        | ResourceKind::Blessings => None,
    }
}

fn item_content_id(item: ItemKind) -> &'static str {
    match item {
        ItemKind::Mug => "item_mug",
        ItemKind::Bowl => "item_bowl",
        ItemKind::Furniture => "item_furniture",
        ItemKind::Tool => "item_generic_tool",
        ItemKind::Weapon => "item_weapon",
        ItemKind::Armor => "item_armor",
        ItemKind::Clothing => "item_treated_pelt_clothing",
        ItemKind::Trinket => "item_trinket",
        ItemKind::Toy => "item_toy",
        ItemKind::Brick => "item_brick",
    }
}

fn station_id(building: BuildingType) -> Option<&'static str> {
    match building {
        BuildingType::Mill => Some("cookhouse"),
        BuildingType::Sawmill => Some("sawmill"),
        BuildingType::Workshop => Some("workshop"),
        BuildingType::Smelter => Some("smelter"),
        BuildingType::WoodCutter => Some("wood_cutter"),
        BuildingType::StonePrep => Some("stone_prep"),
        BuildingType::Woodworking => Some("woodworking"),
        BuildingType::Clothier => Some("clothier"),
        BuildingType::Tannery => Some("tannery"),
        BuildingType::Smithy => Some("smithy"),
        _ => None,
    }
}

macro_rules! assert_stable_id {
    ($kind:ty) => {{
        assert_eq!(<$kind>::from_str("a_0").unwrap().as_str(), "a_0");
        assert!(<$kind>::from_str(&"a".repeat(64)).is_ok());
        let too_long = "a".repeat(65);
        for malformed in [
            "",
            "A",
            "0start",
            "_start",
            "has-dash",
            "has space",
            "é",
            &too_long,
        ] {
            assert!(<$kind>::from_str(malformed).is_err(), "{malformed:?}");
            let encoded = serde_json::to_string(malformed).unwrap();
            assert!(
                serde_json::from_str::<$kind>(&encoded).is_err(),
                "{malformed:?}"
            );
        }
    }};
}

#[test]
fn all_eleven_stable_id_types_share_the_exact_ascii_wire_grammar() {
    assert_stable_id!(ContentId);
    assert_stable_id!(ResourceId);
    assert_stable_id!(FoodId);
    assert_stable_id!(ItemDefinitionId);
    assert_stable_id!(MaterialId);
    assert_stable_id!(CreatureId);
    assert_stable_id!(RecipeId);
    assert_stable_id!(CapabilityId);
    assert_stable_id!(ArtKey);
    assert_stable_id!(PhysicalLotId);
    assert_stable_id!(MaterialInstanceId);
}

#[test]
fn behavior_boundaries_are_closed_serde_enums() {
    assert_eq!(
        serde_json::from_str::<EquipmentSlot>("\"main_hand\"").unwrap(),
        EquipmentSlot::MainHand
    );
    for unknown in [
        "\"tail\"",
        "\"generic_dashboard\"",
        "\"shrine\"",
        "\"freeform\"",
    ] {
        assert!(serde_json::from_str::<EquipmentSlot>(unknown).is_err());
        assert!(serde_json::from_str::<ItemClass>(unknown).is_err());
        assert!(serde_json::from_str::<TaskCategory>(unknown).is_err());
        assert!(serde_json::from_str::<StationBehavior>(unknown).is_err());
        assert!(serde_json::from_str::<AuthorityDomain>(unknown).is_err());
        assert!(serde_json::from_str::<EffectOperation>(unknown).is_err());
        assert!(serde_json::from_str::<AugmentationSlot>(unknown).is_err());
        assert!(serde_json::from_str::<FixtureSlot>(unknown).is_err());
    }
    assert_eq!(
        serde_json::from_str::<CutoverCard>("\"LAI.39\"").unwrap(),
        CutoverCard::Lai39
    );
    assert!(serde_json::from_str::<CutoverCard>("\"LAI.36\"").is_err());
}

#[test]
fn embedded_json_is_strict_validated_canonical_and_complete() {
    let manifest = ContentManifest::embedded();
    let summary = manifest.validate().unwrap();
    assert_eq!(
        summary.capability_total,
        manifest.derived_capability_total()
    );
    assert_eq!(summary.creature_total, 20);
    assert_eq!(summary.material_total, 20);
    assert!(summary.resource_total >= 25);
    assert!(summary.item_definition_total >= 20);
    assert_eq!(summary.recipe_total, manifest.recipes.len());
    assert_eq!(summary.recipe_cutover_total, RECIPE_CUTOVER_RECEIPT_TOTAL);
    assert!(summary.art_total >= 171);

    let canonical = manifest.to_canonical_json();
    assert_eq!(
        canonical,
        ContentManifest::decode_strict(&canonical)
            .unwrap()
            .to_canonical_json()
    );

    let mut value = serde_json::from_str::<serde_json::Value>(&canonical).unwrap();
    value["unknown_field"] = serde_json::Value::Bool(true);
    assert!(ContentManifest::decode_strict(&value.to_string()).is_err());
}

#[test]
fn plan1_tables_keep_exact_recipe_creature_material_and_hole_distinctions() {
    let manifest = ContentManifest::embedded();
    assert_eq!(
        manifest
            .creatures
            .iter()
            .map(|record| record.id.as_str())
            .collect::<Vec<_>>(),
        PLAN1_CREATURE_IDS
    );
    assert_eq!(
        manifest
            .materials
            .iter()
            .map(|record| record.id.as_str())
            .collect::<BTreeSet<_>>(),
        PLAN1_RARE_MATERIAL_IDS.into_iter().collect()
    );
    assert!(manifest.materials.iter().all(|material| {
        material
            .uses
            .iter()
            .any(|use_record| use_record.station.as_str() == "tannery")
    }));
    assert_eq!(
        manifest
            .recipes
            .iter()
            .filter(|recipe| recipe.station.as_str() == "cookhouse")
            .map(|recipe| recipe.id.as_str())
            .collect::<Vec<_>>(),
        PLAN1_COOKHOUSE_RECIPE_IDS
            .into_iter()
            .chain(PLAN1_BREW_RECIPE_IDS)
            .collect::<Vec<_>>()
    );
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
            .filter(|recipe| recipe.station.as_str() == "mill")
            .map(|recipe| recipe.id.as_str())
            .collect::<Vec<_>>(),
        vec!["mill_flour"]
    );
    assert_eq!(
        manifest
            .capabilities
            .iter()
            .filter(
                |capability| capability.id.as_str().starts_with("black_hole_")
                    && capability.id.as_str() != "black_hole_foundations"
            )
            .count(),
        HOLE_AXIS_COUNT
    );

    assert_eq!(
        manifest
            .lair_bands
            .iter()
            .map(|band| (band.band_min, band.band_max))
            .collect::<Vec<_>>(),
        vec![(1, 19), (20, 39), (40, 59), (60, 79), (80, 94), (95, 100)]
    );
    assert_eq!(
        manifest
            .lair_bands
            .iter()
            .map(|band| band.mystic_required_from_level)
            .collect::<Vec<_>>(),
        vec![None, None, None, Some(61), Some(80), Some(95)]
    );
    assert_eq!(
        manifest
            .public_lair_visual_bands()
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
            91..=100,
        ]
    );

    let hole = manifest
        .stations
        .iter()
        .find(|station| station.behavior == StationBehavior::Hole)
        .unwrap();
    assert_eq!(
        (
            hole.work_geometry.width,
            hole.work_geometry.height,
            hole.work_geometry.origin_x,
            hole.work_geometry.origin_y,
            hole.work_geometry.occupied_cells,
        ),
        (3, 3, 1, 1, 9)
    );
    let landmark = hole.landmark_geometry.as_ref().unwrap();
    assert_eq!(
        (
            landmark.width,
            landmark.height,
            landmark.origin_x,
            landmark.origin_y,
            landmark.occupied_cells,
        ),
        (5, 5, 0, 0, 25)
    );
}

#[test]
fn plan1_brewing_is_canonical_cookhouse_work_and_never_mill_work() {
    let manifest = ContentManifest::embedded();
    assert_eq!(
        manifest
            .recipes
            .iter()
            .filter(|recipe| PLAN1_BREW_RECIPE_IDS.contains(&recipe.id.as_str()))
            .map(|recipe| (recipe.id.as_str(), recipe.station.as_str()))
            .collect::<Vec<_>>(),
        PLAN1_BREW_RECIPE_IDS
            .into_iter()
            .map(|recipe_id| (recipe_id, "cookhouse"))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        manifest
            .recipes
            .iter()
            .filter(|recipe| recipe.station.as_str() == "mill")
            .map(|recipe| recipe.id.as_str())
            .collect::<Vec<_>>(),
        vec!["mill_flour"]
    );
    assert!(
        manifest
            .recipe_cutover
            .iter()
            .all(|receipt| !PLAN1_BREW_RECIPE_IDS.contains(&receipt.legacy_id.as_str()))
    );
}

#[test]
fn recipe_bundles_are_resource_owned_and_recipes_have_no_research_nodes() {
    let manifest = ContentManifest::embedded();
    for recipe in &manifest.recipes {
        let owners = manifest
            .recipe_bundles
            .iter()
            .filter(|bundle| bundle.recipes.contains(&recipe.id))
            .collect::<Vec<_>>();
        assert_eq!(owners.len(), 1, "{}", recipe.id);
        assert_eq!(owners[0].capability, recipe.bundle_capability);
        assert!(manifest.resources.iter().any(|resource| {
            resource.content_id == owners[0].owner
                && resource.canonical_capability.required_id() == Some(&owners[0].capability)
        }));
        assert!(
            manifest
                .capabilities
                .iter()
                .all(|capability| capability.id.as_str() != recipe.id.as_str())
        );
    }
    let cookhouse_bundle_capabilities = manifest
        .recipes
        .iter()
        .filter(|recipe| recipe.station.as_str() == "cookhouse")
        .map(|recipe| recipe.bundle_capability.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        cookhouse_bundle_capabilities,
        BTreeSet::from([
            "apple_gathering",
            "grain_milling",
            "hand_fishing",
            "herb_gathering",
            "refined_processing",
        ])
    );
}

#[test]
fn compatibility_inventory_partitions_retained_and_removed_runtime_recipe_ids() {
    let runtime_ids = BuildingType::ALL
        .iter()
        .copied()
        .filter_map(station_recipe_set)
        .flat_map(|set| set.recipes.iter().map(|recipe| recipe.id))
        .collect::<BTreeSet<_>>();
    assert_eq!(runtime_ids.len(), PRE_CUTOVER_RUNTIME_RECIPE_TOTAL);
    assert_eq!(
        runtime_ids,
        PRE_CUTOVER_RUNTIME_RECIPE_IDS.into_iter().collect()
    );

    let non_food_runtime_ids = BuildingType::ALL
        .iter()
        .copied()
        .filter(|building_type| *building_type != BuildingType::Mill)
        .filter_map(station_recipe_set)
        .flat_map(|set| set.recipes.iter().map(|recipe| recipe.id))
        .collect::<BTreeSet<_>>();
    let canonical_ids = ContentManifest::embedded()
        .recipes
        .iter()
        .map(|recipe| recipe.id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        non_food_runtime_ids
            .difference(&canonical_ids)
            .copied()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["materials_to_refined"]),
        "the impossible self-conversion is the only non-Mill runtime recipe removed"
    );

    let mill_runtime_ids = station_recipe_set(BuildingType::Mill)
        .unwrap()
        .recipes
        .iter()
        .map(|recipe| recipe.id)
        .collect::<BTreeSet<_>>();
    let cutover_ids = ContentManifest::embedded()
        .recipe_cutover
        .iter()
        .map(|recipe| recipe.legacy_id.as_str())
        .collect::<BTreeSet<_>>();
    let current_cutover_ids = cutover_ids
        .intersection(&runtime_ids)
        .copied()
        .collect::<BTreeSet<_>>();
    assert_eq!(mill_runtime_ids.len(), CURRENT_MILL_RECIPE_TOTAL);
    assert_eq!(
        current_cutover_ids.len(),
        CURRENT_RUNTIME_RECIPE_CUTOVER_TOTAL
    );
    assert_eq!(
        current_cutover_ids
            .difference(&mill_runtime_ids)
            .copied()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["materials_to_refined"])
    );
    assert_eq!(
        current_cutover_ids.intersection(&mill_runtime_ids).count(),
        CURRENT_MILL_RECIPE_CUTOVER_TOTAL
    );
    assert_eq!(
        canonical_ids
            .intersection(&mill_runtime_ids)
            .copied()
            .collect::<BTreeSet<_>>(),
        PLAN1_BREW_RECIPE_IDS.into_iter().collect()
    );
    assert!(cutover_ids.is_disjoint(&canonical_ids));
    let retained_current = canonical_ids
        .intersection(&runtime_ids)
        .copied()
        .collect::<BTreeSet<_>>();
    assert_eq!(retained_current.len(), RETAINED_PRE_CUTOVER_RECIPE_TOTAL);
    assert_eq!(
        retained_current
            .union(&current_cutover_ids)
            .copied()
            .collect::<BTreeSet<_>>(),
        runtime_ids
    );

    let alias_receipt = ContentManifest::embedded()
        .recipe_cutover
        .iter()
        .find(|entry| entry.legacy_id.as_str() == PERSISTED_COMBINED_MILL_RECIPE_ALIAS)
        .unwrap();
    assert_eq!(alias_receipt.disposition, CutoverDisposition::Remove);
    assert!(alias_receipt.replacement_ids.is_empty());
    assert_eq!(alias_receipt.owning_cutover_card, CutoverCard::Lai52);
    assert!(
        ContentManifest::embedded()
            .recipe_cutover
            .iter()
            .all(|entry| {
                match entry.disposition {
                    CutoverDisposition::SupersededBy => !entry.replacement_ids.is_empty(),
                    CutoverDisposition::Remove => entry.replacement_ids.is_empty(),
                }
            })
    );
}

#[test]
fn retained_runtime_recipes_match_their_exact_per_recipe_input_and_output_identities() {
    const HIDE_ONLY: &[ResourceKind] = &[ResourceKind::Hide];

    let manifest = ContentManifest::embedded();
    for building in BuildingType::ALL.iter().copied() {
        let Some(station) = station_id(building) else {
            continue;
        };
        let recipes = station_recipe_set(building).unwrap().recipes;
        for descriptor in recipes {
            if building == BuildingType::Mill && !PLAN1_BREW_RECIPE_IDS.contains(&descriptor.id) {
                assert!(
                    manifest
                        .recipes
                        .iter()
                        .all(|recipe| recipe.id.as_str() != descriptor.id)
                );
                assert!(
                    manifest
                        .recipe_cutover
                        .iter()
                        .any(|receipt| receipt.legacy_id.as_str() == descriptor.id)
                );
                continue;
            }
            if descriptor.id == "materials_to_refined" {
                assert!(
                    manifest
                        .recipes
                        .iter()
                        .all(|recipe| recipe.id.as_str() != descriptor.id)
                );
                let receipt = manifest
                    .recipe_cutover
                    .iter()
                    .find(|receipt| receipt.legacy_id.as_str() == descriptor.id)
                    .unwrap();
                assert_eq!(receipt.disposition, CutoverDisposition::Remove);
                assert!(receipt.replacement_ids.is_empty());
                assert_eq!(receipt.owning_cutover_card, CutoverCard::Lai52);
                continue;
            }

            let recipe = manifest
                .recipes
                .iter()
                .find(|recipe| recipe.id.as_str() == descriptor.id)
                .unwrap_or_else(|| panic!("missing retained recipe {}", descriptor.id));
            assert_eq!(recipe.station.as_str(), station);

            let intended_inputs = if descriptor.id == "hide_to_leather" {
                HIDE_ONLY
            } else {
                descriptor.input_resources
            };
            let expected_inputs = intended_inputs
                .iter()
                .map(|resource| {
                    resource_content_id(*resource).unwrap_or_else(|| {
                        panic!(
                            "retained recipe {} uses an unmapped input {resource:?}",
                            descriptor.id
                        )
                    })
                })
                .collect::<BTreeSet<_>>();
            let actual_inputs = recipe
                .ingredients
                .iter()
                .map(|ingredient| ingredient.content_id.as_str())
                .collect::<BTreeSet<_>>();
            assert_eq!(recipe.ingredients.len(), expected_inputs.len());
            assert_eq!(actual_inputs, expected_inputs, "{}", descriptor.id);

            let mut expected_outputs = descriptor
                .output_resources
                .iter()
                .map(|resource| {
                    resource_content_id(*resource).unwrap_or_else(|| {
                        panic!(
                            "retained recipe {} uses an unmapped output {resource:?}",
                            descriptor.id
                        )
                    })
                })
                .collect::<BTreeSet<_>>();
            if let Some(item) = descriptor.output_item {
                assert!(
                    descriptor.output_resources.is_empty(),
                    "finite item recipe {} must not inherit station-wide scalar outputs",
                    descriptor.id
                );
                expected_outputs.insert(item_content_id(item.kind));
            }
            let actual_outputs = recipe
                .outputs
                .iter()
                .map(|output| output.content_id.as_str())
                .collect::<BTreeSet<_>>();
            assert_eq!(recipe.outputs.len(), expected_outputs.len());
            assert_eq!(actual_outputs, expected_outputs, "{}", descriptor.id);
        }
    }

    let hide_to_leather = manifest
        .recipes
        .iter()
        .find(|recipe| recipe.id.as_str() == "hide_to_leather")
        .unwrap();
    assert_eq!(
        hide_to_leather
            .ingredients
            .iter()
            .map(|ingredient| ingredient.content_id.as_str())
            .collect::<Vec<_>>(),
        vec!["resource_hide"]
    );
    assert_eq!(
        hide_to_leather
            .outputs
            .iter()
            .map(|output| output.content_id.as_str())
            .collect::<Vec<_>>(),
        vec!["resource_leather"]
    );
}

#[test]
fn unchanged_recipe_flow_requires_an_explicit_allowlist_entry() {
    assert!(UNCHANGED_RECIPE_FLOW_ALLOWLIST.is_empty());
    assert!(ContentManifest::embedded().recipes.iter().all(|recipe| {
        recipe.ingredients.iter().all(|ingredient| {
            recipe
                .outputs
                .iter()
                .all(|output| output.content_id != ingredient.content_id)
        })
    }));

    let mut unchanged = embedded();
    unchanged.recipes[0].outputs[0].content_id =
        unchanged.recipes[0].ingredients[0].content_id.clone();
    assert_failure(
        &unchanged,
        ValidationPhase::References,
        ValidationFailure::WrongReferenceClass,
    );
    assert!(
        messages(&unchanged)
            .iter()
            .any(|message| message.contains("emits an ingredient unchanged"))
    );
}

#[test]
fn locked_content_is_referenceable_but_mutating_operations_are_capability_gated() {
    let manifest = ContentManifest::embedded();
    let grain = ContentId::new("resource_grain").unwrap();
    let empty = BTreeSet::new();
    for operation in [
        ContentOperation::Discover,
        ContentOperation::Store,
        ContentOperation::Trade,
    ] {
        assert!(
            manifest
                .is_operation_permitted(&grain, operation, &empty)
                .unwrap()
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
                .is_operation_permitted(&grain, operation, &empty)
                .unwrap()
        );
    }
    let owned = [CapabilityId::new("grain_milling").unwrap()]
        .into_iter()
        .collect();
    assert!(
        manifest
            .is_operation_permitted(&grain, ContentOperation::Process, &owned)
            .unwrap()
    );
    assert!(
        manifest
            .is_operation_permitted(
                &ContentId::new("unknown_content").unwrap(),
                ContentOperation::Store,
                &empty,
            )
            .is_err()
    );
}

#[test]
fn handler_and_art_registries_bind_manifest_data_without_file_existence_authority() {
    let manifest = ContentManifest::embedded();
    assert!(!COMPILED_HANDLER_REGISTRY.is_empty());
    assert!(
        manifest
            .resources
            .iter()
            .all(|resource| compiled_handler(&resource.behavior_handler).is_some())
    );
    assert!(manifest.art_registry.iter().all(|asset| {
        !asset.planned_asset_path.is_empty()
            && !asset.logical_key.is_empty()
            && asset.native_width > 0
            && asset.native_height > 0
    }));
    assert!(manifest.creatures.iter().all(|creature| {
        manifest.art_asset(&creature.portrait).is_some_and(|asset| {
            asset.layer == ArtLayer::Portrait
                && asset.accessibility == AccessibilityBinding::CreatureName
        })
    }));
}

#[test]
fn validation_phases_cover_every_failure_class_deterministically() {
    let mut version = embedded();
    version.version += 1;
    assert_failure(
        &version,
        ValidationPhase::Version,
        ValidationFailure::UnsupportedVersion,
    );

    let mut duplicate = embedded();
    duplicate.resources.push(duplicate.resources[0].clone());
    assert_failure(
        &duplicate,
        ValidationPhase::IdentityAndOrder,
        ValidationFailure::DuplicateIdentity,
    );

    let mut order = embedded();
    order.resources[1].order = order.resources[0].order;
    assert_failure(
        &order,
        ValidationPhase::IdentityAndOrder,
        ValidationFailure::NonMonotonicOrder,
    );

    let mut vector = embedded();
    let capability = vector
        .capabilities
        .iter_mut()
        .find(|capability| !capability.prerequisites.is_empty())
        .unwrap();
    capability
        .prerequisites
        .push(capability.prerequisites[0].clone());
    assert_failure(
        &vector,
        ValidationPhase::IdentityAndOrder,
        ValidationFailure::DuplicateVectorMember,
    );

    let mut dangling = embedded();
    dangling.recipes[0].ingredients[0].content_id = ContentId::new("missing_content").unwrap();
    assert_failure(
        &dangling,
        ValidationPhase::References,
        ValidationFailure::DanglingReference,
    );

    let mut slot = embedded();
    slot.item_definitions[0].class = ItemClass::Augmentation;
    assert_failure(
        &slot,
        ValidationPhase::References,
        ValidationFailure::SlotMismatch,
    );

    let mut cycle = embedded();
    let self_id = cycle.capabilities[0].id.clone();
    cycle.capabilities[0].prerequisites.push(self_id);
    assert_failure(
        &cycle,
        ValidationPhase::Cycles,
        ValidationFailure::CapabilityCycle,
    );

    let mut numeric = embedded();
    numeric.creatures[0].level_max = 101;
    assert_failure(
        &numeric,
        ValidationPhase::NumericAndCardinality,
        ValidationFailure::NumericRange,
    );

    let mut cardinality = embedded();
    cardinality.lair_visual_bands.pop();
    assert_failure(
        &cardinality,
        ValidationPhase::NumericAndCardinality,
        ValidationFailure::Cardinality,
    );

    let mut handler = embedded();
    handler.resources[0].behavior_handler = "unregistered_handler".to_owned();
    assert_failure(
        &handler,
        ValidationPhase::HandlerRegistry,
        ValidationFailure::MissingHandler,
    );

    let mut art = embedded();
    let key = art.resources[0].art_key.clone();
    art.art_registry.retain(|asset| asset.key != key);
    assert_failure(
        &art,
        ValidationPhase::ArtRegistry,
        ValidationFailure::MissingArt,
    );

    let mut mismatched_art = embedded();
    let icon = mismatched_art
        .art_registry
        .iter_mut()
        .find(|asset| {
            asset.layer == ArtLayer::Icon
                && asset.accessibility == AccessibilityBinding::ContentName
        })
        .unwrap();
    icon.native_width = 64;
    icon.native_height = 64;
    assert_failure(
        &mismatched_art,
        ValidationPhase::ArtRegistry,
        ValidationFailure::InvalidArt,
    );

    let mut founding = embedded();
    founding.founding_capabilities.clear();
    assert_failure(
        &founding,
        ValidationPhase::FoundingBootstrap,
        ValidationFailure::FoundingBootstrap,
    );

    let mut canonical = embedded();
    let granted = canonical.capabilities[0].canonical_for.remove(0);
    assert!(messages(&canonical).iter().any(|message| {
        message.contains(granted.as_str()) && message.contains("exactly one canonical capability")
    }));

    let mut wrong_bundle_class = embedded();
    wrong_bundle_class.recipe_bundles[0].owner = ContentId::new("food_apple").unwrap();
    assert_failure(
        &wrong_bundle_class,
        ValidationPhase::CanonicalCapability,
        ValidationFailure::WrongReferenceClass,
    );

    let mut missing_bundle = embedded();
    missing_bundle.recipe_bundles[0].recipes.clear();
    assert_failure(
        &missing_bundle,
        ValidationPhase::CanonicalCapability,
        ValidationFailure::RecipeBundle,
    );
}

#[test]
fn additive_resource_data_uses_gapped_orders_without_renumbering_existing_content() {
    let mut manifest = embedded();
    let before = manifest.canonical_content_entries();
    let max_order = before.last().unwrap().order;

    let mut added = manifest.resources[0].clone();
    added.id = ResourceId::new("future_resin").unwrap();
    added.content_id = ContentId::new("resource_future_resin").unwrap();
    added.display_name = "Future Resin".to_owned();
    added.description = "Additive future Workshop resource.".to_owned();
    added.order = max_order + 10;
    added.art_key = ArtKey::new("art_resource_future_resin").unwrap();
    manifest.resources.push(added);
    manifest.art_registry.push(ArtAssetDescriptor {
        key: ArtKey::new("art_resource_future_resin").unwrap(),
        planned_asset_path: "assets/planned/content/art_resource_future_resin.png".to_owned(),
        logical_key: "art_resource_future_resin".to_owned(),
        native_width: 16,
        native_height: 16,
        layer: ArtLayer::Icon,
        accessibility: AccessibilityBinding::ContentName,
    });

    manifest.validate().unwrap();
    let after = manifest.canonical_content_entries();
    assert_eq!(&after[..before.len()], before.as_slice());
    assert_eq!(
        after.last().unwrap().content_id.as_str(),
        "resource_future_resin"
    );

    let appended = manifest.resources.pop().unwrap();
    manifest.resources.insert(0, appended);
    assert!(
        messages(&manifest)
            .iter()
            .any(|message| message.contains("non-monotonic stable order"))
    );
}

#[test]
fn founding_and_single_grant_authorities_are_not_shadowed() {
    let manifest = ContentManifest::embedded();
    let founding = manifest
        .founding_capabilities
        .iter()
        .map(CapabilityId::as_str)
        .collect::<BTreeSet<_>>();
    assert!(
        REQUIRED_FOUNDING_CAPABILITIES
            .iter()
            .all(|required| founding.contains(required))
    );

    let mut owners = BTreeSet::new();
    for capability in &manifest.capabilities {
        for content_id in &capability.canonical_for {
            assert!(
                owners.insert(content_id),
                "duplicate grant for {content_id}"
            );
        }
    }
    for resource in &manifest.resources {
        match &resource.canonical_capability {
            CapabilityRequirement::Free => assert!(!owners.contains(&resource.content_id)),
            CapabilityRequirement::Required(_) => assert!(owners.contains(&resource.content_id)),
        }
    }
}
