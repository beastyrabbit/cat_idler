//! LAI.38 red contract for typed food, founding ecology, and Apple regrowth.
//!
//! The future pure `food_ecology` authority consumes the validated food catalog
//! and LAI.37 physical lots. It owns neither generic inventory aliases, Leader
//! choice, station performance, protocol projection, nor world-tick placement.

use std::{collections::BTreeSet, str::FromStr};

mod content_manifest {
    pub use cat_sim::content_manifest::*;
}

mod quality_lots {
    pub use cat_sim::quality_lots::*;
}

#[path = "../src/food_ecology.rs"]
mod food_ecology;

use cat_sim::{
    content_manifest::{CapabilityId, ContentId, ContentManifest, FoodId, PhysicalLotId},
    quality_lots::{
        BulkLotKey, LotLocation, LotProvenance, PhysicalLot, ProductionComplexity,
        ProductionQualityInput, QualityBand, QualityLotLedger, QualityVariationKey, RecoveryReason,
        gathering_quality_score, keyed_variation, quality_from_score,
    },
};
use food_ecology::{
    AppleHarvestRequest, AppleState, AppleTask, ConsumptionRequest, EcologyReport, FishHabitat,
    FishTask, FoodEcology, FoodNeed, FoodPermission, FoodPolicy, FoodUse, FoundingFoodSites,
    HandFishingRequest, ReportAudience, ReportLevel, Tile, WaterSource,
};

fn content(value: &str) -> ContentId {
    ContentId::from_str(value).unwrap()
}

fn food(value: &str) -> FoodId {
    FoodId::from_str(value).unwrap()
}

fn capability(value: &str) -> CapabilityId {
    CapabilityId::from_str(value).unwrap()
}

fn lot_id(value: &str) -> PhysicalLotId {
    PhysicalLotId::from_str(value).unwrap()
}

fn founding_capabilities() -> BTreeSet<CapabilityId> {
    [
        capability("water_collection"),
        capability("apple_gathering"),
        capability("hand_fishing"),
        capability("basic_food_handling"),
    ]
    .into_iter()
    .collect()
}

fn founding_sites() -> FoundingFoodSites {
    let water = Tile { x: 1, y: 1 };
    let bank = Tile { x: 1, y: 2 };
    let apple_tree = Tile { x: 4, y: 3 };
    let fish_water = Tile { x: 7, y: 2 };
    let shoreline = Tile { x: 6, y: 2 };
    FoundingFoodSites {
        revealed_reachable_tiles: [water, bank, apple_tree, fish_water, shoreline]
            .into_iter()
            .collect(),
        water: WaterSource {
            source_tile: water,
            valid_bank_tile: bank,
        },
        apple_tree_tile: apple_tree,
        fish_habitat: FishHabitat {
            water_tile: fish_water,
            shoreline_task_tile: shoreline,
            stock: 24,
            capacity: 24,
            next_replenish_tick: 130,
        },
    }
}

fn ecology() -> FoodEcology {
    FoodEcology::new(ContentManifest::embedded(), founding_sites(), 10).unwrap()
}

fn lot(
    id: &str,
    content_id: &str,
    quality: QualityBand,
    quantity: u32,
    location: LotLocation,
    created_tick: u64,
) -> PhysicalLot {
    PhysicalLot {
        id: lot_id(id),
        key: BulkLotKey::new(content(content_id), quality),
        provenance: LotProvenance {
            origin: "founding_food_source".to_owned(),
            created_tick,
        },
        quantity,
        location,
        reservation: None,
    }
}

fn apple_harvest(tree: Tile, harvest_index: u64) -> AppleHarvestRequest {
    AppleHarvestRequest {
        task: AppleTask {
            tree_tile: tree,
            task_tile: tree,
        },
        source_quality: QualityBand::Common,
        worker_skill: 50,
        tool_quality: Some(QualityBand::Fine),
        fixture_quality: None,
        world_seed: 0x5eed_cafe,
        harvest_index,
        now_tick: 11,
    }
}

#[test]
fn lai38_uses_only_manifest_owned_concrete_foods_and_capability_rules() {
    let manifest = ContentManifest::embedded();
    let exact_raw = [
        ("water", "food_water", 0, 100, None, 1_000, 100, true),
        ("apple", "food_apple", 80, 10, Some(96), 1_000, 250, true),
        (
            "raw_fish",
            "food_raw_fish",
            140,
            0,
            Some(24),
            1_000,
            400,
            false,
        ),
        (
            "raw_meat",
            "food_raw_meat",
            180,
            0,
            Some(18),
            1_000,
            450,
            false,
        ),
    ];
    for (id, content_id, nutrition, hydration, spoilage, weight, value, raw_safe) in exact_raw {
        let descriptor = manifest
            .foods
            .iter()
            .find(|entry| entry.id == food(id))
            .unwrap();
        assert_eq!(descriptor.content_id, content(content_id));
        assert_eq!(descriptor.nutrition, nutrition);
        assert_eq!(descriptor.hydration, hydration);
        assert_eq!(descriptor.spoilage_hours, spoilage);
        assert_eq!(descriptor.weight_milli, weight);
        assert_eq!(descriptor.value_milli, value);
        assert_eq!(descriptor.raw_safe, raw_safe);
    }

    let prepared = [
        ("baked_apples", 125, 5, 72, 500),
        ("grilled_fish", 175, 0, 48, 650),
        ("roasted_meat", 220, 0, 48, 700),
        ("flatbread", 150, -5, 168, 550),
        ("apple_porridge", 180, 30, 72, 800),
        ("fish_stew", 240, 40, 48, 1_000),
        ("meat_stew", 275, 35, 48, 1_050),
        ("apple_preserves", 170, 10, 720, 1_000),
        ("smoked_fish", 220, -10, 480, 1_050),
        ("dried_meat", 260, -15, 480, 1_100),
        ("apple_tart", 300, 5, 120, 1_600),
        ("herb_crusted_fish", 330, 5, 72, 1_700),
        ("meat_pie", 390, 0, 96, 1_900),
        ("surf_and_turf", 430, 0, 72, 2_100),
        ("travel_rations", 420, -20, 960, 2_000),
        ("festival_cake", 520, 10, 120, 2_800),
        ("hunters_feast", 700, 10, 72, 3_500),
        ("grand_lair_feast", 980, 0, 72, 5_000),
    ];
    for (id, nutrition, hydration, spoilage_hours, value_milli) in prepared {
        let descriptor = manifest
            .foods
            .iter()
            .find(|entry| entry.id == food(id))
            .unwrap();
        assert_eq!(descriptor.nutrition, nutrition);
        assert_eq!(descriptor.hydration, hydration);
        assert_eq!(descriptor.spoilage_hours, Some(spoilage_hours));
        assert_eq!(descriptor.weight_milli, 1_000);
        assert_eq!(descriptor.value_milli, value_milli);
        assert!(!descriptor.raw_safe);
        assert_ne!(descriptor.behavior_handler, "generic_food");
    }
    let ids = manifest
        .foods
        .iter()
        .map(|descriptor| descriptor.id.as_str())
        .collect::<BTreeSet<_>>();
    assert!(!ids.contains("food") && !ids.contains("fish") && !ids.contains("preserves"));

    let ecology = ecology();
    let founding = founding_capabilities();
    for generic in ["food", "fish", "preserves"] {
        assert!(
            ecology
                .food_use_permitted(&food(generic), FoodUse::Trade, &founding)
                .is_err(),
            "generic compatibility food IDs are never a typed ecology path"
        );
    }
    assert!(
        ecology
            .food_use_permitted(&food("apple"), FoodUse::RawEat, &founding)
            .unwrap()
    );
    assert!(
        ecology
            .food_use_permitted(&food("apple"), FoodUse::CookhouseIngredient, &founding)
            .unwrap()
    );
    assert!(
        ecology
            .food_use_permitted(&food("raw_meat"), FoodUse::Trade, &BTreeSet::new())
            .unwrap(),
        "locked food may still be traded"
    );
    assert!(
        !ecology
            .food_use_permitted(&food("raw_meat"), FoodUse::HoleFeed, &BTreeSet::new())
            .unwrap(),
        "locked content cannot feed the Hole"
    );
    assert!(
        !ecology
            .food_use_permitted(&food("raw_fish"), FoodUse::RawEat, &founding)
            .unwrap(),
        "raw safety remains catalog-owned"
    );
}

#[test]
fn lai38_founding_sources_are_revealed_reachable_and_not_starter_reserves() {
    let sites = founding_sites();
    let ecology = FoodEcology::new(ContentManifest::embedded(), sites.clone(), 10).unwrap();
    assert_eq!(ecology.founding_sites(), &sites);
    assert!(
        ecology
            .validate_founding_capabilities(&founding_capabilities())
            .is_ok()
    );
    assert!(
        ecology.initial_food_lots().is_empty(),
        "sources, never a starter reserve, found a colony"
    );

    let mut missing_bank = sites.clone();
    missing_bank.water.valid_bank_tile = Tile { x: 2, y: 2 };
    assert!(FoodEcology::new(ContentManifest::embedded(), missing_bank, 10).is_err());
    let mut unreachable_tree = sites.clone();
    unreachable_tree.apple_tree_tile = Tile { x: 99, y: 99 };
    assert!(FoodEcology::new(ContentManifest::embedded(), unreachable_tree, 10).is_err());
    let mut invalid_shoreline = sites;
    invalid_shoreline.fish_habitat.shoreline_task_tile = Tile { x: 99, y: 98 };
    assert!(FoodEcology::new(ContentManifest::embedded(), invalid_shoreline, 10).is_err());

    let tree = founding_sites().apple_tree_tile;
    assert!(
        ecology
            .apple_task(AppleTask {
                tree_tile: tree,
                task_tile: tree
            })
            .is_ok()
    );
    assert!(
        ecology
            .apple_task(AppleTask {
                tree_tile: tree,
                task_tile: Tile {
                    x: tree.x + 1,
                    y: tree.y
                },
            })
            .is_err(),
        "Apple work cannot move off the exact tree tile"
    );
}

#[test]
fn lai38_apple_harvest_regrowth_and_reports_are_deterministic_and_report_safe() {
    let tree = founding_sites().apple_tree_tile;
    let mut one_shot = ecology();
    let mut partitioned = ecology();
    assert_eq!(one_shot.apple_state(tree).unwrap(), AppleState::Full);

    let footprint = one_shot.apple_obstruction_footprint(tree).unwrap();
    let expected_footprint = (-1..=1)
        .flat_map(|dy| {
            (-1..=1).map(move |dx| Tile {
                x: tree.x + dx,
                y: tree.y + dy,
            })
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(footprint.cells, expected_footprint);
    assert_eq!(footprint.trunk_work_tile, tree);
    for cell in &footprint.cells {
        assert!(one_shot.validate_non_apple_placement(*cell).is_err());
    }

    let first_request = apple_harvest(tree, 0);
    let output = one_shot.harvest_apple(first_request.clone()).unwrap();
    let duplicate = ecology().harvest_apple(first_request.clone()).unwrap();
    assert_eq!(
        output.id, duplicate.id,
        "world seed, tree, and harvest index derive one stable physical lot"
    );
    assert_eq!(output.key.quality, duplicate.key.quality);
    assert_eq!(output.key.content_id, content("food_apple"));
    let expected_variation = keyed_variation(&QualityVariationKey {
        world_seed: first_request.world_seed,
        content_id: content("food_apple"),
        lot_id: output.id.clone(),
        completion_index: first_request.harvest_index,
    });
    let expected_quality = quality_from_score(
        gathering_quality_score(
            ProductionQualityInput {
                weighted_input_quality_milli: 0,
                worker_skill: first_request.worker_skill,
                tool_quality: first_request.tool_quality,
                fixture_quality: first_request.fixture_quality,
                station_tier: 1,
                complexity: ProductionComplexity::Raw,
                keyed_variation: expected_variation,
            },
            first_request.source_quality,
        )
        .unwrap(),
    );
    assert_eq!(
        output.key.quality, expected_quality,
        "quality is derived from source, skill, gear, and a keyed harvest roll"
    );
    assert_eq!(one_shot.apple_state(tree).unwrap(), AppleState::Medium);
    let mut partial_harvest = ecology();
    partial_harvest
        .harvest_apple(apple_harvest(tree, 0))
        .unwrap();
    let partial_deadline = partial_harvest.next_regrowth_tick(tree).unwrap();
    partial_harvest.advance_regrowth(partial_deadline).unwrap();
    assert_eq!(
        partial_harvest.apple_state(tree).unwrap(),
        AppleState::Full,
        "a partially harvested tree regrows without requiring total depletion"
    );
    assert!(
        one_shot
            .harvest_apple(AppleHarvestRequest {
                task: AppleTask {
                    tree_tile: tree,
                    task_tile: Tile { x: 0, y: 0 }
                },
                ..apple_harvest(tree, 1)
            })
            .is_err()
    );

    one_shot.harvest_apple(apple_harvest(tree, 1)).unwrap();
    assert_eq!(one_shot.apple_state(tree).unwrap(), AppleState::Low);
    one_shot.harvest_apple(apple_harvest(tree, 2)).unwrap();
    assert_eq!(one_shot.apple_state(tree).unwrap(), AppleState::Empty);
    let empty_snapshot = one_shot.clone();
    assert!(one_shot.harvest_apple(apple_harvest(tree, 3)).is_err());
    assert_eq!(one_shot, empty_snapshot, "an empty tree mints no Apple lot");

    for harvest_index in 0..3 {
        partitioned
            .harvest_apple(apple_harvest(tree, harvest_index))
            .unwrap();
    }
    let mut staged = one_shot.clone();
    let first_deadline = staged.next_regrowth_tick(tree).unwrap();
    staged.advance_regrowth(first_deadline).unwrap();
    assert_eq!(staged.apple_state(tree).unwrap(), AppleState::Low);
    let second_deadline = staged.next_regrowth_tick(tree).unwrap();
    staged.advance_regrowth(second_deadline).unwrap();
    assert_eq!(staged.apple_state(tree).unwrap(), AppleState::Medium);
    let third_deadline = staged.next_regrowth_tick(tree).unwrap();
    staged.advance_regrowth(third_deadline).unwrap();
    assert_eq!(staged.apple_state(tree).unwrap(), AppleState::Full);

    one_shot.advance_regrowth(third_deadline).unwrap();
    assert_eq!(one_shot.apple_state(tree).unwrap(), AppleState::Full);
    assert!(
        one_shot.advance_regrowth(third_deadline).is_err(),
        "each world tick is processed once"
    );

    partitioned.advance_regrowth(first_deadline).unwrap();
    partitioned.advance_regrowth(second_deadline).unwrap();
    partitioned.advance_regrowth(third_deadline).unwrap();
    assert_eq!(one_shot.apple_state(tree), partitioned.apple_state(tree));
    assert_eq!(
        one_shot.next_regrowth_tick(tree),
        partitioned.next_regrowth_tick(tree)
    );

    for level in [1, 2, 3] {
        assert_eq!(
            one_shot.ecology_report(ReportAudience::Leader, ReportLevel(level), tree),
            EcologyReport::Hidden
        );
        assert_eq!(
            one_shot.ecology_report(ReportAudience::God, ReportLevel(level), tree),
            EcologyReport::Hidden,
            "Gods receive the same hidden ecology report"
        );
    }
    for (level, uncertainty) in [(4, 25), (5, 10)] {
        let leader = one_shot.ecology_report(ReportAudience::Leader, ReportLevel(level), tree);
        let god = one_shot.ecology_report(ReportAudience::God, ReportLevel(level), tree);
        assert_eq!(leader, god);
        assert_eq!(leader.relative_error_percent(), Some(uncertainty));
        assert!(!leader.exposes_exact_regrowth_deadline());
    }

    let restarted: FoodEcology =
        serde_json::from_str(&serde_json::to_string(&one_shot).unwrap()).unwrap();
    assert_eq!(
        restarted, one_shot,
        "absolute regrowth timing survives restart"
    );

    let mut persisted = serde_json::to_value(&one_shot).unwrap();
    assert_eq!(persisted["schemaVersion"], 1);
    assert_eq!(
        persisted["manifestSchemaVersion"],
        ContentManifest::embedded().version
    );
    assert!(
        persisted.get("foods").is_none() && persisted.get("recipes").is_none(),
        "persist only compact ecology state, never a manifest copy"
    );
    persisted
        .as_object_mut()
        .unwrap()
        .insert("unknownField".to_owned(), serde_json::json!(true));
    assert!(
        serde_json::from_value::<FoodEcology>(persisted).is_err(),
        "persisted ecology rejects unknown fields"
    );
}

#[test]
fn lai38_consumption_spoilage_and_recovery_use_physical_quality_lots_without_duplication() {
    let mut apple_lot = lot(
        "lot_apple_food",
        "food_apple",
        QualityBand::Fine,
        3,
        LotLocation::Stockpile("main".to_owned()),
        1,
    );
    apple_lot.reservation = None;
    let water_lot = lot(
        "lot_water",
        "food_water",
        QualityBand::Crude,
        2,
        LotLocation::Stockpile("main".to_owned()),
        1,
    );
    let fish_lot = lot(
        "lot_fish_spoil",
        "food_raw_fish",
        QualityBand::Common,
        2,
        LotLocation::Cargo("route_a".to_owned()),
        1,
    );
    let mut reserved_fish_lot = lot(
        "lot_fish_reserved_spoil",
        "food_raw_fish",
        QualityBand::Common,
        1,
        LotLocation::Cargo("route_b".to_owned()),
        1,
    );
    reserved_fish_lot.reservation = Some("cookhouse_batch".to_owned());
    let mut ledger = QualityLotLedger::new(
        vec![apple_lot, water_lot, fish_lot, reserved_fish_lot],
        Vec::new(),
    )
    .unwrap();
    let ecology = ecology();
    let before = ledger.total_bulk_quantity();

    let consumed = ecology
        .consume(
            &mut ledger,
            ConsumptionRequest {
                lot_id: lot_id("lot_apple_food"),
                quantity: 1,
                permission: FoodPermission::Allowed,
                owned_capabilities: founding_capabilities(),
                now_tick: 11,
            },
        )
        .unwrap();
    assert_eq!(consumed.food_id, food("apple"));
    assert_eq!(consumed.quality, QualityBand::Fine);
    assert_eq!(
        consumed.nutrition, 96,
        "Fine applies the existing 120% quality multiplier"
    );
    assert_eq!(consumed.hydration, 12);
    assert_eq!(ledger.total_bulk_quantity(), before - 1);
    assert_eq!(ledger.lot(&lot_id("lot_apple_food")).unwrap().quantity, 2);

    let snapshot = ledger.clone();
    assert!(
        ecology
            .consume(
                &mut ledger,
                ConsumptionRequest {
                    lot_id: lot_id("lot_apple_food"),
                    quantity: 1,
                    permission: FoodPermission::Reserve,
                    owned_capabilities: founding_capabilities(),
                    now_tick: 11,
                },
            )
            .is_err()
    );
    assert_eq!(
        ledger, snapshot,
        "policy refusal cannot consume or relabel a lot"
    );
    assert!(
        ecology
            .consume(
                &mut ledger,
                ConsumptionRequest {
                    lot_id: lot_id("lot_water"),
                    quantity: 1,
                    permission: FoodPermission::Forbidden,
                    owned_capabilities: founding_capabilities(),
                    now_tick: 11,
                },
            )
            .is_err()
    );
    assert_eq!(
        ledger, snapshot,
        "explicit lot requests replace hidden AI food choice"
    );

    ledger
        .recover_lot(
            &lot_id("lot_fish_spoil"),
            RecoveryReason::CarrierDeath,
            LotLocation::Cache("salvage".to_owned()),
        )
        .unwrap();
    let conserved_after_recovery = ledger.total_bulk_quantity();
    assert_eq!(
        ledger.lot(&lot_id("lot_fish_spoil")).unwrap().reservation,
        None
    );
    let spoiled = ecology.apply_spoilage(&mut ledger, 1_441).unwrap();
    assert_eq!(
        spoiled.removed_quantity, 3,
        "expired raw Fish is removed once, never converted into a generic balance"
    );
    assert_eq!(
        spoiled.released_reservations,
        vec!["cookhouse_batch"],
        "physical spoilage invalidates dependent reserved work explicitly"
    );
    assert_eq!(ledger.total_bulk_quantity(), conserved_after_recovery - 3);
    assert!(ledger.lot(&lot_id("lot_fish_spoil")).is_none());
    assert!(ledger.lot(&lot_id("lot_fish_reserved_spoil")).is_none());

    let restarted: QualityLotLedger =
        serde_json::from_str(&serde_json::to_string(&ledger).unwrap()).unwrap();
    assert_eq!(
        restarted, ledger,
        "remaining Water and Apple lots retain exact identity on restart"
    );

    for reason in [
        RecoveryReason::Cancelled,
        RecoveryReason::CarrierDeath,
        RecoveryReason::RouteLost,
    ] {
        let mut cargo = lot(
            "lot_food_recovery",
            "food_apple",
            QualityBand::Common,
            2,
            LotLocation::Cargo("route_b".to_owned()),
            1,
        );
        cargo.reservation = Some("food_delivery".to_owned());
        let mut recovery = QualityLotLedger::new(vec![cargo], Vec::new()).unwrap();
        let total = recovery.total_bulk_quantity();
        recovery
            .recover_lot(
                &lot_id("lot_food_recovery"),
                reason,
                LotLocation::Stockpile("main".to_owned()),
            )
            .unwrap();
        assert_eq!(recovery.total_bulk_quantity(), total);
        assert_eq!(
            recovery
                .lot(&lot_id("lot_food_recovery"))
                .unwrap()
                .reservation,
            None
        );
    }
}

#[test]
fn lai38_finite_fish_is_shoreline_only_and_report_limited_without_hut_or_rod_inputs() {
    let mut ecology = ecology();
    let shoreline = founding_sites().fish_habitat.shoreline_task_tile;
    assert_eq!(ecology.fish_habitat().stock, 24);
    assert_eq!(ecology.fish_habitat().capacity, 24);
    assert_eq!(ecology.fish_habitat().next_replenish_tick, 130);

    let first_request = HandFishingRequest {
        task: FishTask {
            task_tile: shoreline,
        },
        source_quality: QualityBand::Common,
        worker_skill: 20,
        tool_quality: None,
        fixture_quality: None,
        world_seed: 99,
        catch_index: 0,
        now_tick: 11,
    };
    let catch = ecology.hand_fish(first_request.clone()).unwrap();
    assert_eq!(catch.key.content_id, content("food_raw_fish"));
    assert_eq!(catch.quantity, 1);
    let expected_fish_variation = keyed_variation(&QualityVariationKey {
        world_seed: 99,
        content_id: content("food_raw_fish"),
        lot_id: catch.id.clone(),
        completion_index: 0,
    });
    assert_eq!(
        catch.key.quality,
        quality_from_score(
            gathering_quality_score(
                ProductionQualityInput {
                    weighted_input_quality_milli: 0,
                    worker_skill: 20,
                    tool_quality: None,
                    fixture_quality: None,
                    station_tier: 1,
                    complexity: ProductionComplexity::Raw,
                    keyed_variation: expected_fish_variation,
                },
                QualityBand::Common,
            )
            .unwrap(),
        )
    );
    assert_eq!(
        ecology.fish_habitat().stock,
        23,
        "a hand catch consumes real habitat stock"
    );
    let repeat_snapshot = ecology.clone();
    assert!(
        ecology.hand_fish(first_request).is_err(),
        "the same unit catch cannot mint twice"
    );
    assert_eq!(
        ecology, repeat_snapshot,
        "repeat refusal preserves bounded habitat stock"
    );
    assert!(
        ecology
            .hand_fish(HandFishingRequest {
                task: FishTask {
                    task_tile: shoreline
                },
                source_quality: QualityBand::Common,
                worker_skill: 20,
                tool_quality: Some(QualityBand::Masterwork),
                fixture_quality: None,
                world_seed: 99,
                catch_index: 1,
                now_tick: 11,
            })
            .is_err(),
        "LAI.38 hand fishing cannot fabricate later Hut or Rod bonuses"
    );
    assert_eq!(
        ecology, repeat_snapshot,
        "unsupported equipment cannot mutate fish stock"
    );
    let invalid_task_snapshot = ecology.clone();
    assert!(
        ecology
            .hand_fish(HandFishingRequest {
                task: FishTask {
                    task_tile: Tile {
                        x: shoreline.x + 1,
                        y: shoreline.y
                    }
                },
                source_quality: QualityBand::Common,
                worker_skill: 20,
                tool_quality: None,
                fixture_quality: None,
                world_seed: 99,
                catch_index: 1,
                now_tick: 11,
            })
            .is_err(),
        "hand fishing cannot leave the valid shoreline task tile"
    );
    assert_eq!(
        ecology, invalid_task_snapshot,
        "an invalid shoreline task cannot mutate fish stock"
    );

    for index in 1..24 {
        ecology
            .hand_fish(HandFishingRequest {
                task: FishTask {
                    task_tile: shoreline,
                },
                source_quality: QualityBand::Common,
                worker_skill: 20,
                tool_quality: None,
                fixture_quality: None,
                world_seed: 99,
                catch_index: index,
                now_tick: 11,
            })
            .unwrap();
    }
    let empty_snapshot = ecology.clone();
    assert!(
        ecology
            .hand_fish(HandFishingRequest {
                task: FishTask {
                    task_tile: shoreline
                },
                source_quality: QualityBand::Common,
                worker_skill: 20,
                tool_quality: None,
                fixture_quality: None,
                world_seed: 99,
                catch_index: 24,
                now_tick: 11,
            })
            .is_err()
    );
    assert_eq!(
        ecology, empty_snapshot,
        "zero stock refusal does not mint Fish or advance ecology"
    );

    let depleted = ecology.clone();
    ecology.advance_fish_replenishment(130).unwrap();
    assert_eq!(
        ecology.fish_habitat().stock,
        1,
        "one unit replenishes every 120 game-minutes"
    );
    assert_eq!(
        ecology.fish_habitat().next_replenish_tick,
        250,
        "replenishment cursor is absolute and persisted"
    );

    let mut one_shot = depleted.clone();
    one_shot.advance_fish_replenishment(370).unwrap();
    let mut partitioned = depleted;
    partitioned.advance_fish_replenishment(130).unwrap();
    partitioned.advance_fish_replenishment(250).unwrap();
    partitioned.advance_fish_replenishment(370).unwrap();
    assert_eq!(
        one_shot, partitioned,
        "absolute one-unit replenishment is partition invariant"
    );
    let restarted: FoodEcology =
        serde_json::from_str(&serde_json::to_string(&one_shot).unwrap()).unwrap();
    assert_eq!(
        restarted, one_shot,
        "fish stock and its absolute replenishment cursor survive restart"
    );
    for level in [1, 2, 3] {
        assert_eq!(
            ecology.fish_report(ReportAudience::Leader, ReportLevel(level)),
            EcologyReport::Hidden
        );
        assert_eq!(
            ecology.fish_report(ReportAudience::God, ReportLevel(level)),
            EcologyReport::Hidden
        );
    }
    for (level, uncertainty) in [(4, 25), (5, 10)] {
        let leader = ecology.fish_report(ReportAudience::Leader, ReportLevel(level));
        assert_eq!(
            leader,
            ecology.fish_report(ReportAudience::God, ReportLevel(level))
        );
        assert_eq!(leader.relative_error_percent(), Some(uncertainty));
        assert!(!leader.exposes_exact_regrowth_deadline());
    }
}

#[test]
fn lai38_selects_eligible_physical_lots_deterministically_and_values_typed_food_exactly() {
    let mut reserved = lot(
        "lot_apple_reserved",
        "food_apple",
        QualityBand::Common,
        1,
        LotLocation::Stockpile("main".to_owned()),
        1,
    );
    reserved.reservation = Some("cookhouse_input".to_owned());
    let edible_a = lot(
        "lot_apple_a",
        "food_apple",
        QualityBand::Fine,
        1,
        LotLocation::Stockpile("main".to_owned()),
        1,
    );
    let edible_b = lot(
        "lot_apple_b",
        "food_apple",
        QualityBand::Fine,
        1,
        LotLocation::Stockpile("main".to_owned()),
        1,
    );
    let water = lot(
        "lot_water_hydration",
        "food_water",
        QualityBand::Common,
        1,
        LotLocation::Stockpile("main".to_owned()),
        1,
    );
    let spoiled = lot(
        "lot_fish_spoiled",
        "food_raw_fish",
        QualityBand::Common,
        1,
        LotLocation::Stockpile("main".to_owned()),
        1,
    );
    let non_food = lot(
        "lot_logs",
        "logs",
        QualityBand::Masterwork,
        1,
        LotLocation::Stockpile("main".to_owned()),
        1,
    );
    let legacy_generic = lot(
        "lot_legacy_generic",
        "food",
        QualityBand::Common,
        1,
        LotLocation::Stockpile("main".to_owned()),
        1,
    );
    let ledger = QualityLotLedger::new(
        vec![
            reserved,
            edible_b,
            water,
            spoiled,
            non_food,
            legacy_generic,
            edible_a,
        ],
        Vec::new(),
    )
    .unwrap();
    let ecology = ecology();
    let allowed = FoodPolicy::from_entries([(food("apple"), FoodPermission::Allowed)]).unwrap();
    let selected = ecology
        .select_eligible_lot(
            &ledger,
            FoodNeed::Hunger,
            &allowed,
            &founding_capabilities(),
            false,
            1_441,
        )
        .unwrap();
    assert_eq!(
        selected.lot_id,
        lot_id("lot_apple_a"),
        "content then stable lot ID breaks a physical-food tie"
    );
    assert_ne!(selected.lot_id, lot_id("lot_apple_reserved"));
    assert_ne!(selected.lot_id, lot_id("lot_fish_spoiled"));
    assert_ne!(selected.lot_id, lot_id("lot_logs"));
    assert_ne!(selected.lot_id, lot_id("lot_legacy_generic"));
    assert_eq!(
        ecology
            .select_eligible_lot(
                &ledger,
                FoodNeed::Hydration,
                &allowed,
                &founding_capabilities(),
                false,
                1_441
            )
            .unwrap()
            .lot_id,
        lot_id("lot_water_hydration"),
        "hydration uses the same deterministic physical-lot order and manifest properties"
    );

    let mut mutation_ledger = ledger.clone();
    let snapshot = mutation_ledger.clone();
    assert!(
        ecology
            .consume(
                &mut mutation_ledger,
                ConsumptionRequest {
                    lot_id: lot_id("lot_apple_reserved"),
                    quantity: 1,
                    permission: FoodPermission::Allowed,
                    owned_capabilities: founding_capabilities(),
                    now_tick: 25,
                },
            )
            .is_err()
    );
    assert_eq!(
        mutation_ledger, snapshot,
        "reserved lots cannot be consumed or relabeled on error"
    );

    let reserve = FoodPolicy::from_entries([(food("apple"), FoodPermission::Reserve)]).unwrap();
    assert!(
        ecology
            .select_eligible_lot(
                &ledger,
                FoodNeed::Hunger,
                &reserve,
                &founding_capabilities(),
                false,
                25
            )
            .is_err()
    );
    let forbidden = FoodPolicy::from_entries([(food("apple"), FoodPermission::Forbidden)]).unwrap();
    assert!(
        ecology
            .select_eligible_lot(
                &ledger,
                FoodNeed::Hunger,
                &forbidden,
                &founding_capabilities(),
                false,
                25
            )
            .is_err()
    );
    assert_eq!(
        ecology
            .select_eligible_lot(
                &ledger,
                FoodNeed::Hunger,
                &forbidden,
                &founding_capabilities(),
                true,
                25
            )
            .unwrap()
            .lot_id,
        lot_id("lot_apple_a"),
        "only an explicit lethal override can use Forbidden food"
    );

    assert_eq!(
        ecology
            .trade_value_milli(&food("apple"), QualityBand::Fine)
            .unwrap(),
        325
    );
    assert_eq!(
        ecology
            .hole_value_milli(&food("apple"), QualityBand::Fine, &founding_capabilities())
            .unwrap(),
        325
    );
    assert!(
        ecology
            .hole_value_milli(&food("raw_meat"), QualityBand::Common, &BTreeSet::new())
            .is_err()
    );
    assert_eq!(
        FoodEcology::clamp_hydration(3, -6),
        0,
        "negative hydration applies before the lower-bound clamp"
    );
}
