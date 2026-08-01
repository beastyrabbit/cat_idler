//! LAI.37 red contract for universal quality and physical quality lots.
//!
//! This future pure authority consumes the LAI.36 content-manifest IDs. It does
//! not own ID parsing, protocol projection, world simulation, or rendering.

use std::str::FromStr;

use cat_sim::{
    content_manifest::{
        AugmentationSlot, ContentId, EquipmentSlot, FixtureSlot, ItemDefinitionId, MaterialId,
        MaterialInstanceId, PhysicalLotId,
    },
    quality_lots::{
        BulkLotKey, ExactItemPayload, FixtureInstance, ItemAugmentation, ItemInstance, LotLocation,
        LotProvenance, PhysicalLot, ProductionComplexity, ProductionQualityInput,
        QUALITY_LOTS_SCHEMA_VERSION, QualityBand, QualityLotLedger, QualityVariationKey,
        RecoveryReason, StationFixture, gathering_quality_score, keyed_variation,
        production_quality_score, quality_from_score,
    },
};

fn content(value: &str) -> ContentId {
    ContentId::from_str(value).unwrap()
}

fn lot_id(value: &str) -> PhysicalLotId {
    PhysicalLotId::from_str(value).unwrap()
}

fn material_instance(value: &str) -> MaterialInstanceId {
    MaterialInstanceId::from_str(value).unwrap()
}

fn item_definition(value: &str) -> ItemDefinitionId {
    ItemDefinitionId::from_str(value).unwrap()
}

fn material(value: &str) -> MaterialId {
    MaterialId::from_str(value).unwrap()
}

fn item(
    id: &str,
    definition_id: &str,
    material_id: &str,
    quality: QualityBand,
    location: LotLocation,
) -> ItemInstance {
    ItemInstance {
        id: material_instance(id),
        definition_id: item_definition(definition_id),
        material_id: material(material_id),
        quality,
        durability: 80,
        location,
        reservation: None,
        equipment_slot: None,
        augmentation_slot: Some(AugmentationSlot::Tool),
        augmentation: None,
    }
}

fn payload(id: &str, definition_id: &str, slot: Option<AugmentationSlot>) -> ExactItemPayload {
    ExactItemPayload {
        id: material_instance(id),
        definition_id: item_definition(definition_id),
        material_id: material("iron"),
        quality: QualityBand::Fine,
        durability: 80,
        location: LotLocation::Stockpile("main".to_owned()),
        reservation: None,
        equipment_slot: None,
        augmentation_slot: slot,
    }
}

fn provenance(value: &str) -> LotProvenance {
    LotProvenance {
        origin: value.to_owned(),
        created_tick: 42,
    }
}

fn lot(
    id: &str,
    content_id: &str,
    quality: QualityBand,
    quantity: u32,
    location: LotLocation,
    provenance_id: &str,
) -> PhysicalLot {
    PhysicalLot {
        id: lot_id(id),
        key: BulkLotKey::new(content(content_id), quality),
        provenance: provenance(provenance_id),
        quantity,
        location,
        reservation: None,
    }
}

#[test]
fn lai37_quality_bands_have_exact_ordinals_and_multiplier_tables() {
    let expected = [
        (QualityBand::Crude, 0, 80, 75, 80),
        (QualityBand::Common, 1, 100, 100, 100),
        (QualityBand::Fine, 2, 120, 130, 115),
        (QualityBand::Superior, 3, 145, 170, 135),
        (QualityBand::Masterwork, 4, 175, 225, 160),
    ];
    assert_eq!(QualityBand::ALL.len(), 5);
    for (band, ordinal, food, trade_hole, item) in expected {
        assert_eq!(band.ordinal(), ordinal);
        assert_eq!(QualityBand::from_ordinal(ordinal), Ok(band));
        assert_eq!(band.input_quality_milli(), i32::from(ordinal) * 1_000);
        assert_eq!(band.food_nutrition_percent(), food);
        assert_eq!(band.trade_hole_value_percent(), trade_hole);
        assert_eq!(band.item_effect_durability_percent(), item);
    }
    assert!(QualityBand::from_ordinal(5).is_err());
}

#[test]
fn lai37_production_and_gathering_quality_use_exact_fixed_point_rules() {
    let base = ProductionQualityInput {
        weighted_input_quality_milli: 1_000,
        worker_skill: 50,
        tool_quality: Some(QualityBand::Fine),
        fixture_quality: Some(QualityBand::Common),
        station_tier: 2,
        complexity: ProductionComplexity::Prepared,
        keyed_variation: 0,
    };
    // 1000 + 250 + 300 + 200 + 125 - 250 + 0.
    assert_eq!(production_quality_score(base).unwrap(), 1_625);
    assert_eq!(
        production_quality_score(ProductionQualityInput {
            weighted_input_quality_milli: 0,
            worker_skill: 20,
            tool_quality: Some(QualityBand::Crude),
            fixture_quality: Some(QualityBand::Masterwork),
            station_tier: 4,
            complexity: ProductionComplexity::Raw,
            keyed_variation: 0,
        })
        .unwrap(),
        975,
        "tool, fixture, and station bonuses are (quality + 1) × 100 and (tier - 1) × 125"
    );
    for (complexity, penalty) in [
        (ProductionComplexity::Raw, 0),
        (ProductionComplexity::Simple, 0),
        (ProductionComplexity::Prepared, -250),
        (ProductionComplexity::Complex, -500),
        (ProductionComplexity::Feast, -750),
    ] {
        assert_eq!(
            production_quality_score(ProductionQualityInput {
                weighted_input_quality_milli: 0,
                worker_skill: 20,
                tool_quality: None,
                fixture_quality: None,
                station_tier: 1,
                complexity,
                keyed_variation: 0,
            })
            .unwrap(),
            penalty
        );
    }
    assert_eq!(quality_from_score(749), QualityBand::Crude);
    assert_eq!(quality_from_score(750), QualityBand::Common);
    assert_eq!(quality_from_score(1_749), QualityBand::Common);
    assert_eq!(quality_from_score(1_750), QualityBand::Fine);
    assert_eq!(quality_from_score(2_749), QualityBand::Fine);
    assert_eq!(quality_from_score(2_750), QualityBand::Superior);
    assert_eq!(quality_from_score(3_749), QualityBand::Superior);
    assert_eq!(quality_from_score(3_750), QualityBand::Masterwork);

    for (skill, bonus) in [
        (0, -500),
        (19, -500),
        (20, 0),
        (39, 0),
        (40, 250),
        (59, 250),
        (60, 500),
        (79, 500),
        (80, 750),
        (94, 750),
        (95, 1_000),
        (100, 1_000),
    ] {
        assert_eq!(
            production_quality_score(ProductionQualityInput {
                weighted_input_quality_milli: 0,
                worker_skill: skill,
                tool_quality: None,
                fixture_quality: None,
                station_tier: 1,
                complexity: ProductionComplexity::Raw,
                keyed_variation: 0,
            })
            .unwrap(),
            bonus
        );
    }
    assert!(
        production_quality_score(ProductionQualityInput {
            worker_skill: 101,
            ..base
        })
        .is_err()
    );
    assert!(
        production_quality_score(ProductionQualityInput {
            keyed_variation: 251,
            ..base
        })
        .is_err()
    );
    assert!(
        production_quality_score(ProductionQualityInput {
            keyed_variation: -251,
            ..base
        })
        .is_err()
    );
    assert!(
        production_quality_score(ProductionQualityInput {
            station_tier: 0,
            ..base
        })
        .is_err()
    );

    let gathering = gathering_quality_score(
        ProductionQualityInput {
            weighted_input_quality_milli: 9_999,
            worker_skill: 50,
            tool_quality: Some(QualityBand::Fine),
            fixture_quality: Some(QualityBand::Common),
            station_tier: 2,
            complexity: ProductionComplexity::Feast,
            keyed_variation: 0,
        },
        QualityBand::Common,
    )
    .unwrap();
    // Gathering substitutes source quality (1 × 1000) and omits complexity.
    assert_eq!(gathering, 1_875);
}

#[test]
fn lai37_keyed_variation_is_bounded_and_partition_deterministic() {
    let key = QualityVariationKey {
        world_seed: 0x1234_5678,
        content_id: content("apples"),
        lot_id: lot_id("lot_apples"),
        completion_index: 7,
    };
    let one_shot = keyed_variation(&key);
    assert!((-250..=250).contains(&one_shot));
    assert_eq!(one_shot, keyed_variation(&key));

    let batch = (0..32)
        .map(|completion_index| {
            keyed_variation(&QualityVariationKey {
                completion_index,
                ..key.clone()
            })
        })
        .collect::<Vec<_>>();
    let partitioned = (0..11)
        .chain(11..32)
        .map(|completion_index| {
            keyed_variation(&QualityVariationKey {
                completion_index,
                ..key.clone()
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        batch, partitioned,
        "partitioning cannot change keyed quality variation"
    );
}

#[test]
fn lai37_bulk_lots_cover_every_physical_stock_class_without_laundering() {
    let physical_stock = [
        "water",
        "apples",
        "fish",
        "meat",
        "bone",
        "hide",
        "logs",
        "stone",
        "grain",
        "material_iron",
        "intermediate_planks",
        "meal_fish_stew",
        "tool_fishing_rod",
        "furniture_bed",
        "equipment_hunter_armor",
        "drop_bat_wing",
    ];
    let lots = physical_stock
        .iter()
        .enumerate()
        .map(|(index, content_id)| {
            lot(
                &format!("lot_{index}"),
                content_id,
                QualityBand::Fine,
                1,
                LotLocation::Stockpile("main".to_owned()),
                "founding_source",
            )
        })
        .collect::<Vec<_>>();
    let ledger = QualityLotLedger::new(lots, Vec::new()).unwrap();
    assert_eq!(ledger.total_bulk_quantity(), physical_stock.len() as u64);

    for location in [
        LotLocation::Source("tree_a".to_owned()),
        LotLocation::Stockpile("main".to_owned()),
        LotLocation::StationInput("cookhouse_a".to_owned()),
        LotLocation::StationOutput("cookhouse_a".to_owned()),
        LotLocation::Cargo("route_a".to_owned()),
        LotLocation::Cache("lair_a".to_owned()),
        LotLocation::Hole("black_hole_a".to_owned()),
    ] {
        QualityLotLedger::new(
            vec![lot(
                "lot_location",
                "apples",
                QualityBand::Common,
                1,
                location,
                "tree_a",
            )],
            Vec::new(),
        )
        .expect("every physical lot location is representable");
    }

    let apples_fine = BulkLotKey::new(content("apples"), QualityBand::Fine);
    assert_ne!(
        apples_fine,
        BulkLotKey::new(content("apples"), QualityBand::Common)
    );
    assert_ne!(
        apples_fine,
        BulkLotKey::new(content("fish"), QualityBand::Fine)
    );
}

#[test]
fn lai37_generic_iteration_and_batch_debit_are_stable_atomic_and_reservation_safe() {
    let first = lot(
        "lot_a",
        "food_apple",
        QualityBand::Common,
        3,
        LotLocation::Stockpile("main".to_owned()),
        "tree_a",
    );
    let mut reserved = lot(
        "lot_b",
        "food_apple",
        QualityBand::Fine,
        2,
        LotLocation::Stockpile("main".to_owned()),
        "tree_b",
    );
    reserved.reservation = Some("cookhouse".to_owned());
    let mut ledger = QualityLotLedger::new(vec![reserved, first], Vec::new()).unwrap();
    assert_eq!(
        ledger.lots().map(|lot| lot.id.as_str()).collect::<Vec<_>>(),
        vec!["lot_a", "lot_b"]
    );

    let snapshot = ledger.clone();
    assert!(
        ledger
            .debit_lots(&[(lot_id("lot_a"), 1), (lot_id("lot_b"), 1)])
            .is_err()
    );
    assert_eq!(
        ledger, snapshot,
        "a reserved member rejects the whole batch"
    );
    assert!(
        ledger
            .debit_lots(&[(lot_id("lot_a"), 1), (lot_id("lot_a"), 1)])
            .is_err()
    );
    assert_eq!(
        ledger, snapshot,
        "duplicate debit identity cannot partially mutate"
    );
    assert!(ledger.debit_lot(&lot_id("lot_a"), 4).is_err());
    assert_eq!(ledger, snapshot, "an oversized debit is atomic");

    ledger.debit_lot(&lot_id("lot_a"), 2).unwrap();
    assert_eq!(ledger.lot(&lot_id("lot_a")).unwrap().quantity, 1);
    ledger.debit_lot(&lot_id("lot_a"), 1).unwrap();
    assert!(ledger.lot(&lot_id("lot_a")).is_none());
    assert_eq!(ledger.total_bulk_quantity(), 2);

    let released = ledger.expire_lots(&[lot_id("lot_b")]).unwrap();
    assert_eq!(released, vec!["cookhouse"]);
    assert!(ledger.lot(&lot_id("lot_b")).is_none());
    assert_eq!(ledger.total_bulk_quantity(), 0);
}

#[test]
fn lai37_newly_produced_lot_insertion_is_validated_and_atomic() {
    let produced = lot(
        "lot_produced_a",
        "food_raw_fish",
        QualityBand::Common,
        12,
        LotLocation::Source("fish_habitat".to_owned()),
        "shoreline_attempt",
    );
    let mut ledger = QualityLotLedger::new(Vec::new(), Vec::new()).unwrap();
    ledger.insert_lot(produced.clone()).unwrap();
    assert_eq!(ledger.lot(&lot_id("lot_produced_a")), Some(&produced));

    let snapshot = ledger.clone();
    assert!(ledger.insert_lot(produced.clone()).is_err());
    assert_eq!(ledger, snapshot, "duplicate insertion cannot replace a lot");

    let second = lot(
        "lot_produced_b",
        "food_apple",
        QualityBand::Fine,
        2,
        LotLocation::Source("apple_tree".to_owned()),
        "tree_harvest",
    );
    assert!(ledger.insert_lots(vec![second.clone(), second]).is_err());
    assert_eq!(
        ledger, snapshot,
        "a duplicate inside a produced batch cannot partially insert"
    );
}

#[test]
fn lai37_lot_moves_splits_merges_and_recovery_preserve_identity_and_conservation() {
    let original = lot(
        "lot_logs",
        "logs",
        QualityBand::Superior,
        10,
        LotLocation::Source("forest_a".to_owned()),
        "tree_7",
    );
    let mut ledger = QualityLotLedger::new(vec![original], Vec::new()).unwrap();
    let total = ledger.total_bulk_quantity();
    ledger
        .move_lot(
            &lot_id("lot_logs"),
            LotLocation::Cargo("route_a".to_owned()),
        )
        .unwrap();
    ledger
        .split_lot(&lot_id("lot_logs"), lot_id("lot_logs_split"), 3)
        .unwrap();
    assert_eq!(ledger.total_bulk_quantity(), total);
    assert_eq!(
        ledger.lot(&lot_id("lot_logs_split")).unwrap().key.quality,
        QualityBand::Superior
    );
    assert_eq!(
        ledger
            .lot(&lot_id("lot_logs_split"))
            .unwrap()
            .provenance
            .origin,
        "tree_7"
    );

    ledger
        .merge_lots(&lot_id("lot_logs"), &lot_id("lot_logs_split"))
        .unwrap();
    assert_eq!(ledger.total_bulk_quantity(), total);

    for reason in [
        RecoveryReason::Cancelled,
        RecoveryReason::CarrierDeath,
        RecoveryReason::RouteLost,
    ] {
        ledger
            .recover_lot(
                &lot_id("lot_logs"),
                reason,
                LotLocation::Cache("recovery_cache".to_owned()),
            )
            .unwrap();
        assert_eq!(ledger.total_bulk_quantity(), total);
        assert_eq!(
            ledger.lot(&lot_id("lot_logs")).unwrap().location,
            LotLocation::Cache("recovery_cache".to_owned())
        );
    }

    for reason in [
        RecoveryReason::Cancelled,
        RecoveryReason::CarrierDeath,
        RecoveryReason::RouteLost,
    ] {
        let mut reserved_lot = lot(
            "lot_reserved_recovery",
            "logs",
            QualityBand::Common,
            4,
            LotLocation::Cargo("route_b".to_owned()),
            "tree_8",
        );
        reserved_lot.reservation = Some("delivery_b".to_owned());
        let mut reserved_ledger = QualityLotLedger::new(vec![reserved_lot], Vec::new()).unwrap();
        reserved_ledger
            .recover_lot(
                &lot_id("lot_reserved_recovery"),
                reason,
                LotLocation::Cache("recovery_cache".to_owned()),
            )
            .unwrap();
        let recovered = reserved_ledger
            .lot(&lot_id("lot_reserved_recovery"))
            .unwrap();
        assert_eq!(
            recovered.location,
            LotLocation::Cache("recovery_cache".to_owned())
        );
        assert_eq!(
            recovered.reservation, None,
            "recovery releases the original reservation"
        );
    }

    let restarted: QualityLotLedger =
        serde_json::from_str(&serde_json::to_string(&ledger).unwrap()).unwrap();
    assert_eq!(
        restarted, ledger,
        "restart round trips every lot identity and quantity"
    );
}

#[test]
fn lai37_merge_rejects_content_quality_provenance_and_reservation_laundering() {
    let shared_location = LotLocation::StationInput("cookhouse_a".to_owned());
    let left = lot(
        "lot_left",
        "apples",
        QualityBand::Fine,
        2,
        shared_location.clone(),
        "tree_a",
    );
    let different_quality = lot(
        "lot_quality",
        "apples",
        QualityBand::Common,
        2,
        shared_location.clone(),
        "tree_a",
    );
    let different_content = lot(
        "lot_content",
        "fish",
        QualityBand::Fine,
        2,
        shared_location.clone(),
        "tree_a",
    );
    let different_provenance = lot(
        "lot_provenance",
        "apples",
        QualityBand::Fine,
        2,
        shared_location.clone(),
        "tree_b",
    );
    let mut reserved = lot(
        "lot_reserved",
        "apples",
        QualityBand::Fine,
        2,
        shared_location,
        "tree_a",
    );
    reserved.reservation = Some("reservation_a".to_owned());

    for right in [
        different_quality,
        different_content,
        different_provenance,
        reserved,
    ] {
        let mut ledger =
            QualityLotLedger::new(vec![left.clone(), right.clone()], Vec::new()).unwrap();
        let total = ledger.total_bulk_quantity();
        assert!(ledger.merge_lots(&left.id, &right.id).is_err());
        assert_eq!(ledger.total_bulk_quantity(), total);
        assert_eq!(ledger.lot(&left.id).unwrap(), &left);
        assert_eq!(ledger.lot(&right.id).unwrap(), &right);
    }
}

#[test]
fn lai37_exact_item_instances_and_typed_slots_refuse_invalid_augmentation_or_fixture_work() {
    let base_item = ItemInstance {
        id: material_instance("item_rod_a"),
        definition_id: item_definition("fishing_rod"),
        material_id: material("iron"),
        quality: QualityBand::Fine,
        durability: 80,
        location: LotLocation::Stockpile("main".to_owned()),
        reservation: None,
        equipment_slot: None,
        augmentation_slot: Some(AugmentationSlot::Tool),
        augmentation: None,
    };
    let compatible = ItemAugmentation {
        item: payload(
            "augmentation_rod_grip",
            "rod_grip",
            Some(AugmentationSlot::Tool),
        ),
        slot: AugmentationSlot::Tool,
    };
    let incompatible = ItemAugmentation {
        item: payload(
            "augmentation_armor_lining",
            "armor_lining",
            Some(AugmentationSlot::Armor),
        ),
        slot: AugmentationSlot::Armor,
    };
    let mut eligible = base_item.clone();
    eligible.install_augmentation(compatible.clone()).unwrap();
    assert_eq!(eligible.augmentation, Some(compatible.clone()));
    assert!(
        eligible.install_augmentation(compatible.clone()).is_err(),
        "one typed slot only"
    );

    let mut reserved = base_item.clone();
    reserved.reservation = Some("task_a".to_owned());
    assert!(reserved.install_augmentation(compatible.clone()).is_err());
    let mut equipped = base_item.clone();
    equipped.equipment_slot = Some(EquipmentSlot::Tool);
    assert!(equipped.install_augmentation(compatible.clone()).is_err());
    let mut carried = base_item.clone();
    carried.location = LotLocation::Cargo("cat_a".to_owned());
    assert!(carried.install_augmentation(compatible.clone()).is_err());
    let mut broken = base_item.clone();
    broken.durability = 0;
    assert!(broken.install_augmentation(compatible.clone()).is_err());
    assert!(
        base_item
            .clone()
            .install_augmentation(incompatible)
            .is_err()
    );
    let mut not_augmentable = base_item.clone();
    not_augmentable.augmentation_slot = None;
    assert!(not_augmentable.install_augmentation(compatible).is_err());

    let fixture = StationFixture {
        item: payload("fixture_smoke_rack", "smoke_rack", None),
        slot: FixtureSlot::Cookhouse,
    };
    let mut cookhouse = FixtureInstance {
        slot: FixtureSlot::Cookhouse,
        installed: None,
        reserved: false,
    };
    cookhouse.install_fixture(fixture.clone()).unwrap();
    assert_eq!(cookhouse.installed, Some(fixture));
    assert!(
        cookhouse
            .install_fixture(StationFixture {
                item: payload("fixture_other_rack", "other_rack", None),
                slot: FixtureSlot::Cookhouse
            })
            .is_err()
    );
    let mut workshop = FixtureInstance {
        slot: FixtureSlot::Workshop,
        installed: None,
        reserved: false,
    };
    assert!(
        workshop
            .install_fixture(StationFixture {
                item: payload("fixture_wrong_rack", "wrong_rack", None),
                slot: FixtureSlot::Cookhouse
            })
            .is_err()
    );
    let mut reserved_station = FixtureInstance {
        reserved: true,
        ..cookhouse.clone()
    };
    reserved_station.installed = None;
    assert!(
        reserved_station
            .install_fixture(StationFixture {
                item: payload("fixture_blocked_rack", "blocked_rack", None),
                slot: FixtureSlot::Cookhouse
            })
            .is_err()
    );

    let item_ledger = QualityLotLedger::new(Vec::new(), vec![eligible]).unwrap();
    let restarted: QualityLotLedger =
        serde_json::from_str(&serde_json::to_string(&item_ledger).unwrap()).unwrap();
    assert_eq!(
        restarted, item_ledger,
        "restart preserves exact item-instance identity and augmentation"
    );
}

#[test]
fn lai37_wire_and_rejected_mutations_are_strict_and_atomic() {
    let valid = lot(
        "lot_strict",
        "logs",
        QualityBand::Common,
        4,
        LotLocation::Stockpile("main".to_owned()),
        "tree_strict",
    );

    let mut unknown_field = serde_json::to_value(&valid).unwrap();
    unknown_field["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<PhysicalLot>(unknown_field).is_err());

    let mut wrong_version = serde_json::to_value(&valid).unwrap();
    wrong_version["schemaVersion"] = serde_json::json!(QUALITY_LOTS_SCHEMA_VERSION + 1);
    let wrong_version_result = serde_json::from_value::<PhysicalLot>(wrong_version.clone());
    assert!(
        wrong_version_result.is_err(),
        "future PhysicalLot schema unexpectedly decoded: wire={wrong_version}, decoded={wrong_version_result:?}"
    );

    let mut zero_quantity = valid.clone();
    zero_quantity.quantity = 0;
    assert!(QualityLotLedger::new(vec![zero_quantity], Vec::new()).is_err());
    let mut empty_provenance = valid.clone();
    empty_provenance.provenance.origin.clear();
    assert!(QualityLotLedger::new(vec![empty_provenance], Vec::new()).is_err());
    let mut empty_location = valid.clone();
    empty_location.location = LotLocation::Stockpile(String::new());
    assert!(QualityLotLedger::new(vec![empty_location], Vec::new()).is_err());
    let mut empty_reservation = valid.clone();
    empty_reservation.reservation = Some(String::new());
    assert!(QualityLotLedger::new(vec![empty_reservation], Vec::new()).is_err());
    assert!(
        QualityLotLedger::new(vec![valid.clone(), valid.clone()], Vec::new()).is_err(),
        "duplicate physical identities are rejected"
    );

    let mut ledger = QualityLotLedger::new(vec![valid], Vec::new()).unwrap();
    let before = ledger.clone();
    assert!(
        ledger
            .split_lot(&lot_id("lot_strict"), lot_id("lot_split_zero"), 0)
            .is_err()
    );
    assert_eq!(ledger, before);
    assert!(
        ledger
            .move_lot(&lot_id("lot_strict"), LotLocation::Cargo(String::new()),)
            .is_err()
    );
    assert_eq!(ledger, before);
    assert!(
        ledger
            .recover_lot(
                &lot_id("lot_strict"),
                RecoveryReason::Cancelled,
                LotLocation::Cache(String::new()),
            )
            .is_err()
    );
    assert_eq!(
        ledger, before,
        "recovery validates its destination before changing a lot"
    );

    let left = lot(
        "lot_overflow_left",
        "stone",
        QualityBand::Fine,
        u32::MAX,
        LotLocation::Stockpile("main".to_owned()),
        "quarry_a",
    );
    let right = lot(
        "lot_overflow_right",
        "stone",
        QualityBand::Fine,
        1,
        LotLocation::Stockpile("main".to_owned()),
        "quarry_a",
    );
    let mut overflow = QualityLotLedger::new(vec![left, right], Vec::new()).unwrap();
    let overflow_before = overflow.clone();
    assert!(
        overflow
            .merge_lots(&lot_id("lot_overflow_left"), &lot_id("lot_overflow_right"),)
            .is_err()
    );
    assert_eq!(overflow, overflow_before);
}

#[test]
fn lai37_ledgers_reject_inventory_beyond_the_persisted_bound() {
    let lots = (0..=cat_sim::quality_lots::MAX_PHYSICAL_LOTS)
        .map(|index| {
            lot(
                &format!("bounded_lot_{index}"),
                "logs",
                QualityBand::Common,
                1,
                LotLocation::Stockpile("main".to_owned()),
                "bounded_source",
            )
        })
        .collect();
    assert!(QualityLotLedger::new(lots, Vec::new()).is_err());

    let bounded = QualityLotLedger::new(
        vec![lot(
            "wire_bound_lot",
            "logs",
            QualityBand::Common,
            1,
            LotLocation::Stockpile("main".to_owned()),
            "bounded_source",
        )],
        Vec::new(),
    )
    .unwrap();
    let mut wire = serde_json::to_value(bounded).unwrap();
    let encoded_lot = wire["lots"][0].clone();
    wire["lots"] = serde_json::Value::Array(
        (0..=cat_sim::quality_lots::MAX_PHYSICAL_LOTS)
            .map(|_| encoded_lot.clone())
            .collect(),
    );
    assert!(serde_json::from_value::<QualityLotLedger>(wire).is_err());
}

#[test]
fn lai37_item_identity_and_ledger_serialization_are_strict_and_canonical() {
    let item = ItemInstance {
        id: material_instance("item_canonical"),
        definition_id: item_definition("fishing_rod"),
        material_id: material("iron"),
        quality: QualityBand::Fine,
        durability: 5,
        location: LotLocation::Stockpile("main".to_owned()),
        reservation: None,
        equipment_slot: None,
        augmentation_slot: Some(AugmentationSlot::Tool),
        augmentation: None,
    };
    assert!(
        QualityLotLedger::new(Vec::new(), vec![item.clone(), item.clone()]).is_err(),
        "duplicate named-item identities are rejected"
    );
    let incompatible_wire = serde_json::to_value(&ItemInstance {
        augmentation: Some(ItemAugmentation {
            item: payload(
                "augmentation_bad_wire",
                "armor_lining",
                Some(AugmentationSlot::Armor),
            ),
            slot: AugmentationSlot::Armor,
        }),
        ..item.clone()
    })
    .unwrap();
    assert!(serde_json::from_value::<ItemInstance>(incompatible_wire).is_err());
    let mut unknown_item_wire = serde_json::to_value(&item).unwrap();
    unknown_item_wire["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ItemInstance>(unknown_item_wire).is_err());

    let first = lot(
        "lot_a",
        "logs",
        QualityBand::Common,
        1,
        LotLocation::Stockpile("main".to_owned()),
        "tree_a",
    );
    let second = lot(
        "lot_b",
        "stone",
        QualityBand::Fine,
        2,
        LotLocation::Cache("cache_a".to_owned()),
        "quarry_a",
    );
    let canonical =
        QualityLotLedger::new(vec![first.clone(), second.clone()], vec![item.clone()]).unwrap();
    let permuted = QualityLotLedger::new(vec![second, first], vec![item]).unwrap();
    assert_eq!(
        serde_json::to_string(&canonical).unwrap(),
        serde_json::to_string(&permuted).unwrap(),
        "BTree-backed restart wire order is independent of construction order"
    );
}

#[test]
fn lai37_item_ledger_mutations_are_stable_atomic_and_recovery_safe() {
    let item_b = item(
        "item_b",
        "generic_tool",
        "iron",
        QualityBand::Common,
        LotLocation::Cargo("route_a".to_owned()),
    );
    let mut item_a = item(
        "item_a",
        "fishing_rod",
        "iron",
        QualityBand::Fine,
        LotLocation::Stockpile("main".to_owned()),
    );
    item_a.reservation = Some("fish_task".to_owned());
    let mut ledger = QualityLotLedger::new(Vec::new(), vec![item_b.clone(), item_a.clone()])
        .expect("initial exact items are valid");
    assert_eq!(
        ledger
            .items()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        vec!["item_a", "item_b"],
        "item iteration is canonical by stable instance ID"
    );

    let snapshot = ledger.clone();
    assert!(ledger.insert_item(item_a.clone()).is_err());
    assert_eq!(ledger, snapshot, "duplicate insert cannot replace an item");
    assert!(
        ledger
            .insert_items(vec![
                item(
                    "item_c",
                    "generic_tool",
                    "iron",
                    QualityBand::Common,
                    LotLocation::Stockpile("main".to_owned()),
                ),
                item(
                    "item_c",
                    "generic_tool",
                    "iron",
                    QualityBand::Common,
                    LotLocation::Stockpile("main".to_owned()),
                ),
            ])
            .is_err()
    );
    assert_eq!(
        ledger, snapshot,
        "duplicate identity inside a batch cannot partially insert"
    );

    let mut replaced = item_a.clone();
    replaced.durability = 41;
    replaced.reservation = None;
    ledger.replace_item(replaced.clone()).unwrap();
    assert_eq!(ledger.item(&material_instance("item_a")), Some(&replaced));
    assert!(
        ledger
            .replace_item(item(
                "item_missing",
                "generic_tool",
                "iron",
                QualityBand::Common,
                LotLocation::Stockpile("main".to_owned()),
            ))
            .is_err()
    );

    ledger
        .recover_item(
            &material_instance("item_b"),
            RecoveryReason::CarrierDeath,
            LotLocation::Cache("recovery_cache".to_owned()),
        )
        .unwrap();
    let recovered = ledger.item(&material_instance("item_b")).unwrap();
    assert_eq!(
        recovered.location,
        LotLocation::Cache("recovery_cache".to_owned())
    );
    assert_eq!(
        recovered.reservation, None,
        "item recovery releases task reservations like physical lots"
    );

    let removed = ledger.remove_item(&material_instance("item_a")).unwrap();
    assert_eq!(removed.id, material_instance("item_a"));
    assert!(ledger.item(&material_instance("item_a")).is_none());
    assert!(ledger.remove_item(&material_instance("item_a")).is_err());
}
