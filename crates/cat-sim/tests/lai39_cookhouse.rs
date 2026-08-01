//! LAI.39 red contract for the pure manifest-driven Cookhouse transaction leaf.

use std::{collections::BTreeSet, str::FromStr};

use cat_sim::{
    content_manifest::{
        CapabilityId, ContentId, ContentManifest, ItemDefinitionId, MaterialId, MaterialInstanceId,
        PLAN1_BREW_RECIPE_IDS, PLAN1_COOKHOUSE_RECIPE_IDS, PhysicalLotId, RecipeDescriptor,
        RecipeId,
    },
    cookhouse::{
        BatchStage, CookhouseBatch, CookhouseBatchRequest, CookhouseError, CookhouseFixture,
        CookhouseQueue, CookhouseQueueEntry, CookhouseReadiness, CookhouseRecoveryReason,
        IngredientState, MAX_COOKHOUSE_QUEUE_ENTRIES, WORK_UNITS_PER_COMPLEXITY,
        cookhouse_task_footprint, prepare_batch, prepare_batch_from_ledger,
    },
    quality_lots::{
        BulkLotKey, ItemInstance, LotLocation, LotProvenance, PhysicalLot, ProductionComplexity,
        ProductionQualityInput, QualityBand, QualityLotLedger, QualityVariationKey,
        keyed_variation, production_quality_score, quality_from_score,
    },
    spatial_tasks::TilePoint,
};

fn content(value: &str) -> ContentId {
    ContentId::from_str(value).unwrap()
}

fn recipe(value: &str) -> RecipeId {
    RecipeId::from_str(value).unwrap()
}

fn capability(value: &str) -> CapabilityId {
    CapabilityId::from_str(value).unwrap()
}

fn item(value: &str) -> ItemDefinitionId {
    ItemDefinitionId::from_str(value).unwrap()
}

fn instance(value: &str) -> MaterialInstanceId {
    MaterialInstanceId::from_str(value).unwrap()
}

fn material(value: &str) -> MaterialId {
    MaterialId::from_str(value).unwrap()
}

fn lot_id(value: &str) -> PhysicalLotId {
    PhysicalLotId::from_str(value).unwrap()
}

fn lot_at(
    id: &str,
    content_id: &str,
    quality: QualityBand,
    quantity: u32,
    location: LotLocation,
) -> PhysicalLot {
    PhysicalLot {
        id: lot_id(id),
        key: BulkLotKey::new(content(content_id), quality),
        provenance: LotProvenance {
            origin: format!("source_{id}"),
            created_tick: 10,
        },
        quantity,
        location,
        reservation: None,
    }
}

fn lot(id: &str, content_id: &str, quality: QualityBand, quantity: u32) -> PhysicalLot {
    lot_at(
        id,
        content_id,
        quality,
        quantity,
        LotLocation::Stockpile("stockpile_1".to_owned()),
    )
}

fn all_capabilities(manifest: &ContentManifest) -> BTreeSet<CapabilityId> {
    manifest
        .capabilities
        .iter()
        .map(|entry| entry.id.clone())
        .collect()
}

fn readiness(manifest: &ContentManifest) -> CookhouseReadiness {
    CookhouseReadiness {
        station_id: "cookhouse_1".to_owned(),
        station_tier: 5,
        worker_id: "worker_1".to_owned(),
        worker_skill: 20,
        capabilities: all_capabilities(manifest),
        tools: Vec::new(),
        fixtures: Vec::new(),
        output_free_units: 100,
    }
}

fn request(recipe_id: &str, suffix: usize) -> CookhouseBatchRequest {
    CookhouseBatchRequest {
        batch_id: format!("batch_{suffix}"),
        recipe_id: recipe(recipe_id),
        world_seed: 7,
        completion_index: suffix as u64,
    }
}

fn recipe_lots(recipe: &RecipeDescriptor, suffix: usize) -> Vec<PhysicalLot> {
    let mut result = recipe
        .ingredients
        .iter()
        .enumerate()
        .map(|(index, ingredient)| {
            lot(
                &format!("lot_{suffix}_{index}"),
                ingredient.content_id.as_str(),
                QualityBand::Common,
                ingredient.units,
            )
        })
        .collect::<Vec<_>>();
    if recipe.requires_fuel {
        result.push(lot(
            &format!("lot_{suffix}_fuel"),
            "resource_fuel",
            QualityBand::Common,
            1,
        ));
    }
    result
}

fn baked_lots() -> Vec<PhysicalLot> {
    vec![
        lot("lot_apples", "food_apple", QualityBand::Common, 2),
        lot("lot_fuel", "resource_fuel", QualityBand::Common, 1),
    ]
}

fn baked_request() -> CookhouseBatchRequest {
    request("baked_apples", 1)
}

#[test]
fn lai39_catalog_is_exact_and_spatial_authority_projects_all_nine_cells() {
    let manifest = ContentManifest::embedded();
    let active = manifest
        .recipes
        .iter()
        .filter(|entry| entry.station.as_str() == "cookhouse")
        .map(|entry| entry.id.as_str())
        .collect::<BTreeSet<_>>();
    let expected = PLAN1_COOKHOUSE_RECIPE_IDS
        .iter()
        .chain(PLAN1_BREW_RECIPE_IDS.iter())
        .copied()
        .collect::<BTreeSet<_>>();
    assert_eq!(active, expected);
    assert_eq!(active.len(), 23);
    assert_eq!(
        manifest
            .recipes
            .iter()
            .filter(|entry| entry.station.as_str() == "mill")
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        vec!["mill_flour"]
    );

    let footprint = cookhouse_task_footprint(manifest, TilePoint { x: 10, y: 20 }).unwrap();
    assert_eq!(
        footprint.tiles.as_slice(),
        &[
            TilePoint { x: 10, y: 20 },
            TilePoint { x: 11, y: 20 },
            TilePoint { x: 12, y: 20 },
            TilePoint { x: 10, y: 21 },
            TilePoint { x: 11, y: 21 },
            TilePoint { x: 12, y: 21 },
            TilePoint { x: 10, y: 22 },
            TilePoint { x: 11, y: 22 },
            TilePoint { x: 12, y: 22 },
        ]
    );
}

#[test]
fn lai39_all_meal_rows_are_exact_including_fuel_and_container() {
    struct Expected<'a> {
        id: &'a str,
        complexity: u8,
        ingredients: &'a [(&'a str, u32)],
        output: (&'a str, u32),
        fuel: bool,
        container: bool,
    }
    let rows = [
        Expected {
            id: "baked_apples",
            complexity: 2,
            ingredients: &[("food_apple", 2)],
            output: ("food_baked_apples", 2),
            fuel: true,
            container: false,
        },
        Expected {
            id: "grilled_fish",
            complexity: 2,
            ingredients: &[("food_raw_fish", 1)],
            output: ("food_grilled_fish", 1),
            fuel: true,
            container: false,
        },
        Expected {
            id: "roasted_meat",
            complexity: 2,
            ingredients: &[("food_raw_meat", 1)],
            output: ("food_roasted_meat", 1),
            fuel: true,
            container: false,
        },
        Expected {
            id: "flatbread",
            complexity: 2,
            ingredients: &[("resource_flour", 2), ("food_water", 1)],
            output: ("food_flatbread", 2),
            fuel: true,
            container: false,
        },
        Expected {
            id: "apple_porridge",
            complexity: 3,
            ingredients: &[("food_apple", 2), ("resource_grain", 1), ("food_water", 1)],
            output: ("food_apple_porridge", 3),
            fuel: false,
            container: false,
        },
        Expected {
            id: "fish_stew",
            complexity: 3,
            ingredients: &[
                ("food_raw_fish", 2),
                ("food_water", 1),
                ("resource_herbs", 1),
            ],
            output: ("food_fish_stew", 3),
            fuel: false,
            container: false,
        },
        Expected {
            id: "meat_stew",
            complexity: 3,
            ingredients: &[
                ("food_raw_meat", 2),
                ("food_water", 1),
                ("resource_herbs", 1),
            ],
            output: ("food_meat_stew", 3),
            fuel: false,
            container: false,
        },
        Expected {
            id: "apple_preserves",
            complexity: 3,
            ingredients: &[("food_apple", 3), ("food_water", 1), ("resource_clay", 1)],
            output: ("food_apple_preserves", 3),
            fuel: false,
            container: true,
        },
        Expected {
            id: "smoked_fish",
            complexity: 3,
            ingredients: &[("food_raw_fish", 2), ("resource_herbs", 1)],
            output: ("food_smoked_fish", 2),
            fuel: true,
            container: false,
        },
        Expected {
            id: "dried_meat",
            complexity: 3,
            ingredients: &[("food_raw_meat", 2)],
            output: ("food_dried_meat", 2),
            fuel: true,
            container: false,
        },
        Expected {
            id: "apple_tart",
            complexity: 4,
            ingredients: &[("food_apple", 3), ("resource_flour", 2), ("food_water", 1)],
            output: ("food_apple_tart", 4),
            fuel: false,
            container: false,
        },
        Expected {
            id: "herb_crusted_fish",
            complexity: 4,
            ingredients: &[
                ("food_raw_fish", 2),
                ("resource_flour", 1),
                ("resource_herbs", 1),
                ("food_water", 1),
            ],
            output: ("food_herb_crusted_fish", 3),
            fuel: false,
            container: false,
        },
        Expected {
            id: "meat_pie",
            complexity: 4,
            ingredients: &[
                ("food_raw_meat", 2),
                ("resource_flour", 2),
                ("resource_herbs", 1),
                ("food_water", 1),
            ],
            output: ("food_meat_pie", 4),
            fuel: false,
            container: false,
        },
        Expected {
            id: "surf_and_turf",
            complexity: 4,
            ingredients: &[
                ("food_raw_fish", 2),
                ("food_raw_meat", 2),
                ("resource_herbs", 1),
                ("food_water", 1),
            ],
            output: ("food_surf_and_turf", 4),
            fuel: false,
            container: false,
        },
        Expected {
            id: "travel_rations",
            complexity: 4,
            ingredients: &[
                ("food_dried_meat", 1),
                ("food_smoked_fish", 1),
                ("food_flatbread", 1),
            ],
            output: ("food_travel_rations", 3),
            fuel: false,
            container: false,
        },
        Expected {
            id: "festival_cake",
            complexity: 5,
            ingredients: &[
                ("food_apple", 3),
                ("resource_flour", 3),
                ("food_water", 1),
                ("food_brew", 1),
                ("food_catnip", 1),
            ],
            output: ("food_festival_cake", 6),
            fuel: false,
            container: false,
        },
        Expected {
            id: "hunters_feast",
            complexity: 5,
            ingredients: &[
                ("food_raw_meat", 3),
                ("food_raw_fish", 2),
                ("food_apple", 2),
                ("resource_herbs", 2),
                ("food_water", 1),
            ],
            output: ("food_hunters_feast", 8),
            fuel: false,
            container: false,
        },
        Expected {
            id: "grand_lair_feast",
            complexity: 5,
            ingredients: &[
                ("food_raw_meat", 4),
                ("food_raw_fish", 4),
                ("food_apple", 3),
                ("resource_flour", 3),
                ("resource_herbs", 2),
                ("food_brew", 1),
            ],
            output: ("food_grand_lair_feast", 12),
            fuel: false,
            container: false,
        },
    ];
    let manifest = ContentManifest::embedded();
    assert_eq!(rows.len(), 18);
    for expected in rows {
        let actual = manifest
            .recipes
            .iter()
            .find(|entry| entry.id.as_str() == expected.id)
            .unwrap();
        assert_eq!(actual.complexity, expected.complexity, "{}", expected.id);
        let actual_ingredients = actual
            .ingredients
            .iter()
            .map(|entry| (entry.content_id.as_str(), entry.units))
            .collect::<Vec<_>>();
        assert_eq!(
            actual_ingredients.as_slice(),
            expected.ingredients,
            "{}",
            expected.id
        );
        assert_eq!(actual.outputs.len(), 1, "{}", expected.id);
        assert_eq!(
            (
                actual.outputs[0].content_id.as_str(),
                actual.outputs[0].units
            ),
            expected.output,
            "{}",
            expected.id
        );
        assert_eq!(actual.requires_fuel, expected.fuel, "{}", expected.id);
        assert_eq!(
            actual.requires_container, expected.container,
            "{}",
            expected.id
        );
    }
}

#[test]
fn lai39_all_five_brews_are_physical_cookhouse_food_and_feed_both_feasts() {
    let manifest = ContentManifest::embedded();
    let expected = [
        ("brew_grain_small", 1, "resource_grain"),
        ("brew_catnip_ale", 2, "food_catnip"),
        ("brew_herbal_tonic", 3, "resource_herbs"),
        ("brew_spiced_ale", 4, "food_catnip"),
        ("brew_masterwork", 5, "resource_herbs"),
    ];
    for (id, complexity, input) in expected {
        let descriptor = manifest
            .recipes
            .iter()
            .find(|entry| entry.id.as_str() == id)
            .unwrap();
        assert_eq!(descriptor.station.as_str(), "cookhouse");
        assert_eq!(descriptor.complexity, complexity);
        assert_eq!(descriptor.ingredients.len(), 1);
        assert_eq!(descriptor.ingredients[0].content_id.as_str(), input);
        assert_eq!(descriptor.ingredients[0].units, 1);
        assert_eq!(descriptor.outputs.len(), 1);
        assert_eq!(descriptor.outputs[0].content_id.as_str(), "food_brew");
        assert_eq!(descriptor.outputs[0].units, 1);
        assert!(!descriptor.requires_fuel);
        assert!(!descriptor.requires_container);
    }
    for feast in ["festival_cake", "grand_lair_feast"] {
        let descriptor = manifest
            .recipes
            .iter()
            .find(|entry| entry.id.as_str() == feast)
            .unwrap();
        assert!(descriptor.ingredients.iter().any(|ingredient| {
            ingredient.content_id.as_str() == "food_brew" && ingredient.units == 1
        }));
    }
}

#[test]
fn lai39_every_canonical_recipe_prepares_and_all_cutovers_and_aliases_fail() {
    let manifest = ContentManifest::embedded();
    for (index, recipe_id) in PLAN1_COOKHOUSE_RECIPE_IDS
        .iter()
        .chain(PLAN1_BREW_RECIPE_IDS.iter())
        .enumerate()
    {
        let descriptor = manifest
            .recipes
            .iter()
            .find(|entry| entry.id.as_str() == *recipe_id)
            .unwrap();
        let lots = recipe_lots(descriptor, index + 10);
        let batch = prepare_batch(
            manifest,
            &readiness(manifest),
            &lots,
            request(recipe_id, index + 10),
        )
        .unwrap();
        assert_eq!(
            batch.work_required(),
            u64::from(descriptor.complexity) * WORK_UNITS_PER_COMPLEXITY
        );
        assert_eq!(batch.output_plans().len(), descriptor.outputs.len());
    }

    assert_eq!(manifest.recipe_cutover.len(), 17);
    for (index, receipt) in manifest.recipe_cutover.iter().enumerate() {
        let error = prepare_batch(
            manifest,
            &readiness(manifest),
            &[],
            CookhouseBatchRequest {
                batch_id: format!("retired_{index}"),
                recipe_id: receipt.legacy_id.clone(),
                world_seed: 1,
                completion_index: 0,
            },
        )
        .unwrap_err();
        assert_eq!(
            error,
            CookhouseError::RetiredRecipe(receipt.legacy_id.clone())
        );
        for replacement in &receipt.replacement_ids {
            assert!(
                manifest
                    .recipes
                    .iter()
                    .any(|entry| &entry.id == replacement)
            );
        }
    }
    for (index, alias) in ["food", "fish", "preserves"].iter().enumerate() {
        assert_eq!(
            prepare_batch(
                manifest,
                &readiness(manifest),
                &[],
                request(alias, index + 50),
            ),
            Err(CookhouseError::UnknownRecipe(recipe(alias)))
        );
    }
    assert_eq!(
        prepare_batch(
            manifest,
            &readiness(manifest),
            &[],
            request("mill_flour", 60),
        ),
        Err(CookhouseError::NotCookhouseRecipe(recipe("mill_flour")))
    );
}

#[test]
fn lai39_readiness_gates_are_bounded_and_do_not_mutate_inputs() {
    let manifest = ContentManifest::embedded();
    let lots = baked_lots();
    let original = lots.clone();

    let mut no_worker = readiness(manifest);
    no_worker.worker_id.clear();
    assert_eq!(
        prepare_batch(manifest, &no_worker, &lots, baked_request()),
        Err(CookhouseError::NoWorkers)
    );
    let mut bad_worker = readiness(manifest);
    bad_worker.worker_id = "Bad Worker".to_owned();
    assert_eq!(
        prepare_batch(manifest, &bad_worker, &lots, baked_request()),
        Err(CookhouseError::InvalidWorkerId)
    );
    let mut bad_skill = readiness(manifest);
    bad_skill.worker_skill = 101;
    assert_eq!(
        prepare_batch(manifest, &bad_skill, &lots, baked_request()),
        Err(CookhouseError::InvalidWorkerSkill(101))
    );
    let mut low_tier = readiness(manifest);
    low_tier.station_tier = 0;
    assert_eq!(
        prepare_batch(manifest, &low_tier, &lots, baked_request()),
        Err(CookhouseError::InsufficientStationTier {
            required: 1,
            actual: 0,
        })
    );
    for missing in ["cookhouse", "apple_gathering", "refined_processing"] {
        let mut no_capability = readiness(manifest);
        no_capability.capabilities.remove(&capability(missing));
        assert_eq!(
            prepare_batch(manifest, &no_capability, &lots, baked_request()),
            Err(CookhouseError::MissingCapability(capability(missing))),
            "{missing}"
        );
    }
    let mut no_space = readiness(manifest);
    no_space.output_free_units = 1;
    assert_eq!(
        prepare_batch(manifest, &no_space, &lots, baked_request()),
        Err(CookhouseError::InsufficientOutputCapacity {
            required: 2,
            available: 1,
        })
    );
    let mut reserved = lots.clone();
    reserved[0].reservation = Some("other_batch".to_owned());
    assert_eq!(
        prepare_batch(manifest, &readiness(manifest), &reserved, baked_request()),
        Err(CookhouseError::ReservedInput(lot_id("lot_apples")))
    );
    let mut remote = lots.clone();
    remote[0].location = LotLocation::Cargo("cart_1".to_owned());
    assert_eq!(
        prepare_batch(manifest, &readiness(manifest), &remote, baked_request()),
        Err(CookhouseError::LotOutsideEligibleInput(lot_id(
            "lot_apples"
        )))
    );
    assert_eq!(
        prepare_batch(manifest, &readiness(manifest), &lots[..1], baked_request()),
        Err(CookhouseError::MissingIngredient {
            content_id: content("resource_fuel"),
            required: 1,
            selected: 0,
        })
    );
    assert_eq!(lots, original, "all preflight failures are atomic");
}

#[test]
fn lai39_lot_selection_is_stable_reservation_safe_and_identity_conserving() {
    let manifest = ContentManifest::embedded();
    let lots = vec![
        lot("lot_apples_z", "food_apple", QualityBand::Crude, 8),
        lot("lot_fuel", "resource_fuel", QualityBand::Crude, 1),
        lot("lot_apples_a", "food_apple", QualityBand::Fine, 3),
    ];
    let mut reversed = lots.clone();
    reversed.reverse();
    let first = prepare_batch(manifest, &readiness(manifest), &lots, baked_request()).unwrap();
    let second = prepare_batch(manifest, &readiness(manifest), &reversed, baked_request()).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        first
            .reserved_inputs()
            .iter()
            .map(|input| input.lot_id.as_str())
            .collect::<Vec<_>>(),
        vec!["lot_apples_a", "lot_fuel"]
    );
    let apples = &first.reserved_inputs()[0];
    assert_eq!(apples.reserved_quantity, 3);
    assert_eq!(apples.consumed_quantity, 2);
    assert_eq!(first.weighted_input_quality_milli(), 1_333);

    let ledger = QualityLotLedger::new(lots, Vec::new()).unwrap();
    assert_eq!(
        prepare_batch_from_ledger(manifest, &readiness(manifest), &ledger, baked_request())
            .unwrap(),
        first
    );
    assert_eq!(ledger.total_bulk_quantity(), 12);
}

#[test]
fn lai39_quality_is_exact_keyed_and_cannot_launder_an_unconsumed_remainder() {
    let manifest = ContentManifest::embedded();
    let lots = vec![
        lot("lot_apples", "food_apple", QualityBand::Fine, 3),
        lot("lot_fuel", "resource_fuel", QualityBand::Crude, 1),
    ];
    let batch = prepare_batch(manifest, &readiness(manifest), &lots, baked_request()).unwrap();
    let plan = &batch.output_plans()[0];
    let variation = keyed_variation(&QualityVariationKey {
        world_seed: 7,
        content_id: content("food_baked_apples"),
        lot_id: plan.lot_id.clone(),
        completion_index: 1,
    });
    let score = production_quality_score(ProductionQualityInput {
        weighted_input_quality_milli: 1_333,
        worker_skill: 20,
        tool_quality: None,
        fixture_quality: None,
        station_tier: 5,
        complexity: ProductionComplexity::Simple,
        keyed_variation: variation,
    })
    .unwrap();
    assert_eq!(plan.quality, quality_from_score(score));

    let mut completed = batch;
    completed.deliver_inputs().unwrap();
    let result = completed
        .advance_work(completed.work_required(), 90)
        .unwrap()
        .unwrap();
    assert_eq!(result.remainders.len(), 1);
    assert_eq!(result.remainders[0].id, lot_id("lot_apples"));
    assert_eq!(result.remainders[0].quantity, 1);
    assert_eq!(result.remainders[0].key.quality, QualityBand::Fine);
    assert_eq!(
        result
            .consumed
            .iter()
            .map(|input| input.consumed_quantity)
            .sum::<u32>(),
        3
    );
}

#[test]
fn lai39_tools_fixtures_and_multi_output_are_manifest_driven_physical_objects() {
    let manifest = ContentManifest::embedded();
    let mut custom = manifest.clone();
    let baked = custom
        .recipes
        .iter_mut()
        .find(|entry| entry.id.as_str() == "baked_apples")
        .unwrap();
    baked.tools = vec![item("fishing_rod")];
    baked.fixtures = vec![item("cookhouse")];
    baked
        .outputs
        .push(cat_sim::content_manifest::RecipeIngredient {
            content_id: content("food_brew"),
            units: 1,
        });
    let lots = baked_lots();
    assert_eq!(
        prepare_batch(&custom, &readiness(&custom), &lots, baked_request()),
        Err(CookhouseError::MissingTool(item("fishing_rod")))
    );

    let mut ready = readiness(&custom);
    ready.tools.push(ItemInstance {
        id: instance("tool_1"),
        definition_id: item("fishing_rod"),
        material_id: material("bat_wing"),
        quality: QualityBand::Fine,
        durability: 10,
        location: LotLocation::StationInput("cookhouse_1".to_owned()),
        reservation: None,
        equipment_slot: None,
        augmentation_slot: None,
        augmentation: None,
    });
    assert_eq!(
        prepare_batch(&custom, &ready, &lots, baked_request()),
        Err(CookhouseError::MissingFixture(item("cookhouse")))
    );
    ready.fixtures.push(CookhouseFixture {
        instance_id: instance("fixture_1"),
        definition_id: item("cookhouse"),
        quality: QualityBand::Superior,
        station_id: "cookhouse_1".to_owned(),
        reserved: false,
    });
    let batch = prepare_batch(&custom, &ready, &lots, baked_request()).unwrap();
    assert_eq!(batch.selected_tools()[0].instance_id, instance("tool_1"));
    assert_eq!(
        batch.selected_fixtures()[0].instance_id,
        instance("fixture_1")
    );
    assert_eq!(batch.output_plans().len(), 2);
    assert_ne!(
        batch.output_plans()[0].lot_id,
        batch.output_plans()[1].lot_id
    );
    assert_eq!(batch.output_plans()[0].quantity, 2);
    assert_eq!(batch.output_plans()[1].quantity, 1);

    ready.tools[0].reservation = Some("other_batch".to_owned());
    assert_eq!(
        prepare_batch(&custom, &ready, &lots, baked_request()),
        Err(CookhouseError::InvalidTool(instance("tool_1")))
    );
}

#[test]
fn lai39_physical_lifecycle_partition_restart_pickup_and_recovery_are_conserving() {
    let manifest = ContentManifest::embedded();
    let lots = baked_lots();
    let mut batch = prepare_batch(manifest, &readiness(manifest), &lots, baked_request()).unwrap();
    assert_eq!(batch.stage(), BatchStage::Reserved);
    assert!(
        batch
            .reserved_inputs()
            .iter()
            .all(|input| input.state == IngredientState::Reserved)
    );
    assert!(
        batch
            .physical_inputs()
            .iter()
            .all(|input| input.reservation.as_deref() == Some("batch_1"))
    );
    batch.mark_inputs_in_transit().unwrap();
    assert_eq!(batch.stage(), BatchStage::InTransit);
    assert!(
        batch
            .physical_inputs()
            .iter()
            .all(|input| matches!(input.location, LotLocation::Cargo(_)))
    );
    batch.deliver_inputs().unwrap();
    assert_eq!(batch.stage(), BatchStage::Ready);
    assert!(
        batch
            .physical_inputs()
            .iter()
            .all(|input| { input.location == LotLocation::StationInput("cookhouse_1".to_owned()) })
    );
    batch.set_paused(true).unwrap();
    assert_eq!(batch.advance_work(500, 20).unwrap(), None);
    assert_eq!(batch.work_completed(), 0);
    batch.set_paused(false).unwrap();

    let mut one_tick = batch.clone();
    let required = one_tick.work_required();
    let one_completion = one_tick.advance_work(required, 99).unwrap().unwrap();

    let mut partitioned = batch;
    partitioned.advance_work(37, 30).unwrap();
    let encoded = partitioned.to_canonical_json();
    let mut restarted = CookhouseBatch::decode_strict(&encoded).unwrap();
    restarted.advance_work(41, 60).unwrap();
    let partition_completion = restarted.advance_work(required - 78, 99).unwrap().unwrap();
    assert_eq!(restarted, one_tick);
    assert_eq!(partition_completion, one_completion);
    assert_eq!(restarted.stage(), BatchStage::OutputReady);
    assert!(
        partition_completion
            .consumed
            .iter()
            .all(|input| input.state == IngredientState::Consumed)
    );
    assert!(restarted.physical_inputs().is_empty());
    assert!(partition_completion.outputs.iter().all(|output| {
        output.location == LotLocation::StationOutput("cookhouse_1".to_owned())
            && output.provenance.created_tick == 99
    }));
    let input_units = partition_completion
        .consumed
        .iter()
        .map(|input| input.reserved_quantity)
        .sum::<u32>();
    let consumed_units = partition_completion
        .consumed
        .iter()
        .map(|input| input.consumed_quantity)
        .sum::<u32>();
    let remainder_units = partition_completion
        .remainders
        .iter()
        .map(|lot| lot.quantity)
        .sum::<u32>();
    assert_eq!(input_units, consumed_units + remainder_units);

    let picked_up = restarted.pickup_outputs().unwrap();
    assert_eq!(restarted.stage(), BatchStage::PickedUp);
    assert!(
        picked_up
            .iter()
            .all(|output| matches!(output.location, LotLocation::Cargo(_)))
    );
    assert_eq!(restarted.pickup_outputs().unwrap(), picked_up);

    let mut before = prepare_batch(
        manifest,
        &readiness(manifest),
        &lots,
        request("baked_apples", 2),
    )
    .unwrap();
    before.mark_inputs_in_transit().unwrap();
    let recovered = before
        .recover(CookhouseRecoveryReason::WorkerDeath)
        .unwrap();
    assert_eq!(recovered.inputs, lots);
    assert!(recovered.outputs.is_empty());
    assert_eq!(
        before.recover(CookhouseRecoveryReason::RouteLoss).unwrap(),
        recovered,
        "recovery is idempotent and preserves the first cause"
    );
    let restarted_before = CookhouseBatch::decode_strict(&before.to_canonical_json()).unwrap();
    assert_eq!(restarted_before, before);

    let mut route = prepare_batch(
        manifest,
        &readiness(manifest),
        &lots,
        request("baked_apples", 4),
    )
    .unwrap();
    route.mark_inputs_in_transit().unwrap();
    assert_eq!(
        route
            .recover(CookhouseRecoveryReason::RouteLoss)
            .unwrap()
            .inputs,
        lots
    );

    let mut cancelled_before = prepare_batch(
        manifest,
        &readiness(manifest),
        &lots,
        request("baked_apples", 5),
    )
    .unwrap();
    assert_eq!(
        cancelled_before
            .recover(CookhouseRecoveryReason::Cancelled)
            .unwrap()
            .inputs,
        lots
    );

    let mut after = prepare_batch(
        manifest,
        &readiness(manifest),
        &lots,
        request("baked_apples", 3),
    )
    .unwrap();
    after.deliver_inputs().unwrap();
    after.advance_work(after.work_required(), 101).unwrap();
    let salvage = after.recover(CookhouseRecoveryReason::Cancelled).unwrap();
    assert!(salvage.inputs.is_empty());
    assert_eq!(salvage.outputs.len(), 1);
    assert_eq!(salvage.outputs[0].quantity, 2);
    assert_eq!(
        salvage.outputs[0].location,
        LotLocation::StationOutput("cookhouse_1".to_owned())
    );
    assert_eq!(
        CookhouseBatch::decode_strict(&after.to_canonical_json()).unwrap(),
        after
    );
}

#[test]
fn lai39_queue_preserves_authored_order_repeat_pause_progress_and_empty_state() {
    let manifest = ContentManifest::embedded();
    let mut queue = CookhouseQueue::new("cookhouse_1".to_owned()).unwrap();
    assert!(queue.entries().is_empty());
    for (entry_id, recipe_id, repeat) in [
        ("entry_a", "baked_apples", true),
        ("entry_b", "brew_grain_small", false),
        ("entry_c", "grand_lair_feast", false),
    ] {
        queue
            .enqueue(
                manifest,
                CookhouseQueueEntry {
                    entry_id: entry_id.to_owned(),
                    recipe_id: recipe(recipe_id),
                    repeat,
                    paused: false,
                    progress_work_units: 0,
                    completed_batches: 0,
                },
            )
            .unwrap();
    }
    assert_eq!(
        queue
            .entries()
            .iter()
            .map(|entry| entry.entry_id.as_str())
            .collect::<Vec<_>>(),
        vec!["entry_a", "entry_b", "entry_c"]
    );
    queue.set_paused("entry_a", true).unwrap();
    queue.add_progress(manifest, "entry_a", 50).unwrap();
    assert_eq!(queue.entries()[0].progress_work_units, 0);
    queue.set_paused("entry_a", false).unwrap();
    assert_eq!(
        queue.complete_front(manifest),
        Err(CookhouseError::InvalidStage)
    );
    queue.add_progress(manifest, "entry_a", 500).unwrap();
    assert_eq!(queue.entries()[0].progress_work_units, 200);
    queue.complete_front(manifest).unwrap();
    assert_eq!(
        queue
            .entries()
            .iter()
            .map(|entry| entry.entry_id.as_str())
            .collect::<Vec<_>>(),
        vec!["entry_b", "entry_c", "entry_a"]
    );
    assert_eq!(queue.entries()[2].progress_work_units, 0);
    assert_eq!(queue.entries()[2].completed_batches, 1);
    queue.add_progress(manifest, "entry_b", 100).unwrap();
    queue.complete_front(manifest).unwrap();
    assert_eq!(
        queue
            .entries()
            .iter()
            .map(|entry| entry.entry_id.as_str())
            .collect::<Vec<_>>(),
        vec!["entry_c", "entry_a"]
    );
    let encoded = queue.to_canonical_json();
    assert_eq!(CookhouseQueue::decode_strict(&encoded).unwrap(), queue);
}

#[test]
fn lai39_queue_capacity_catalog_and_strict_persistence_reject_invalid_state() {
    let manifest = ContentManifest::embedded();
    let mut queue = CookhouseQueue::new("cookhouse_1".to_owned()).unwrap();
    for index in 0..MAX_COOKHOUSE_QUEUE_ENTRIES {
        queue
            .enqueue(
                manifest,
                CookhouseQueueEntry {
                    entry_id: format!("entry_{index}"),
                    recipe_id: recipe("baked_apples"),
                    repeat: false,
                    paused: false,
                    progress_work_units: index as u64,
                    completed_batches: 0,
                },
            )
            .unwrap();
    }
    assert_eq!(
        queue.enqueue(
            manifest,
            CookhouseQueueEntry {
                entry_id: "overflow".to_owned(),
                recipe_id: recipe("baked_apples"),
                repeat: false,
                paused: false,
                progress_work_units: 0,
                completed_batches: 0,
            },
        ),
        Err(CookhouseError::QueueFull)
    );

    let mut invalid = CookhouseQueue::new("cookhouse_1".to_owned()).unwrap();
    for receipt in &manifest.recipe_cutover {
        assert_eq!(
            invalid.enqueue(
                manifest,
                CookhouseQueueEntry {
                    entry_id: format!("retired_{}", receipt.order),
                    recipe_id: receipt.legacy_id.clone(),
                    repeat: false,
                    paused: false,
                    progress_work_units: 0,
                    completed_batches: 0,
                },
            ),
            Err(CookhouseError::RetiredRecipe(receipt.legacy_id.clone()))
        );
    }
    for (id, recipe_id) in [
        ("generic_food", "food"),
        ("generic_fish", "fish"),
        ("generic_preserves", "preserves"),
    ] {
        assert_eq!(
            invalid.enqueue(
                manifest,
                CookhouseQueueEntry {
                    entry_id: id.to_owned(),
                    recipe_id: recipe(recipe_id),
                    repeat: false,
                    paused: false,
                    progress_work_units: 0,
                    completed_batches: 0,
                },
            ),
            Err(CookhouseError::UnknownRecipe(recipe(recipe_id)))
        );
    }
    assert_eq!(
        invalid.enqueue(
            manifest,
            CookhouseQueueEntry {
                entry_id: "mill".to_owned(),
                recipe_id: recipe("mill_flour"),
                repeat: false,
                paused: false,
                progress_work_units: 0,
                completed_batches: 0,
            },
        ),
        Err(CookhouseError::NotCookhouseRecipe(recipe("mill_flour")))
    );

    let mut value: serde_json::Value = serde_json::from_str(&queue.to_canonical_json()).unwrap();
    value["unknown"] = serde_json::json!(true);
    assert!(CookhouseQueue::decode_strict(&value.to_string()).is_err());
    value.as_object_mut().unwrap().remove("unknown");
    value["schemaVersion"] = serde_json::json!(2);
    assert_eq!(
        CookhouseQueue::decode_strict(&value.to_string()),
        Err(CookhouseError::InvalidSchemaVersion(2))
    );

    let batch = prepare_batch(
        manifest,
        &readiness(manifest),
        &baked_lots(),
        baked_request(),
    )
    .unwrap();
    let mut batch_value: serde_json::Value =
        serde_json::from_str(&batch.to_canonical_json()).unwrap();
    batch_value["unknown"] = serde_json::json!(true);
    assert!(CookhouseBatch::decode_strict(&batch_value.to_string()).is_err());
    batch_value.as_object_mut().unwrap().remove("unknown");
    batch_value["schemaVersion"] = serde_json::json!(2);
    assert_eq!(
        CookhouseBatch::decode_strict(&batch_value.to_string()),
        Err(CookhouseError::InvalidSchemaVersion(2))
    );
    let mut forged_quality: serde_json::Value =
        serde_json::from_str(&batch.to_canonical_json()).unwrap();
    forged_quality["outputPlans"][0]["quality"] = serde_json::json!("masterwork");
    assert_eq!(
        CookhouseBatch::decode_strict(&forged_quality.to_string()),
        Err(CookhouseError::InvalidPersistedState)
    );
}
