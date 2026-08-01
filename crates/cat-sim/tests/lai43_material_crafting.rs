//! LAI.43 acceptance contract for the single material-crafting authority.

use std::str::FromStr;

use cat_sim::{
    content_manifest::{
        AugmentationSlot, ContentId, ContentManifest, EquipmentSlot, FixtureSlot, ItemDefinitionId,
        MaterialId, MaterialInstanceId, PLAN1_RARE_MATERIAL_IDS, PhysicalLotId,
    },
    material_crafting::{
        CapabilitySet, DurabilityTarget, FixtureTarget, MATERIAL_CRAFTING_SCHEMA_VERSION,
        MAX_MATERIAL_RECEIPTS, MaterialCommand, MaterialCommandOperation, MaterialCommandResult,
        MaterialCraftingAuthority, MaterialCraftingCatalog, MaterialProcessingState,
        MaterialRecoveryReason, NamedMaterialInstance, ProductionContext, RecoveryDestination,
    },
    quality_lots::{
        BulkLotKey, FixtureInstance, ItemInstance, LotLocation, LotProvenance, PhysicalLot,
        QualityBand, QualityLotLedger,
    },
};

fn material_id(value: &str) -> MaterialId {
    MaterialId::from_str(value).unwrap()
}

fn instance_id(value: &str) -> MaterialInstanceId {
    MaterialInstanceId::from_str(value).unwrap()
}

fn item_definition(value: &str) -> ItemDefinitionId {
    ItemDefinitionId::from_str(value).unwrap()
}

fn content_id(value: &str) -> ContentId {
    ContentId::from_str(value).unwrap()
}

fn lot_id(value: &str) -> PhysicalLotId {
    PhysicalLotId::from_str(value).unwrap()
}

fn catalog() -> MaterialCraftingCatalog<'static> {
    MaterialCraftingCatalog::embedded()
}

fn all_capabilities() -> CapabilitySet {
    CapabilitySet::from_owned(
        ContentManifest::embedded()
            .capabilities
            .iter()
            .map(|capability| capability.id.clone()),
    )
}

fn raw_material(catalog: &MaterialCraftingCatalog<'_>, material: &str) -> NamedMaterialInstance {
    NamedMaterialInstance::raw_from_hunt(
        catalog,
        instance_id(&format!("drop_{material}")),
        material_id(material),
        QualityBand::Superior,
        LotProvenance {
            origin: format!("lair_{material}"),
            created_tick: 41,
        },
        LotLocation::Cache(format!("cache_{material}")),
        None,
    )
    .unwrap()
}

fn context(destination: LotLocation, completion_index: u64) -> ProductionContext {
    ProductionContext {
        world_seed: 0xA11C_E043,
        station_tier: 2,
        worker_skill: 70,
        tool_quality: Some(QualityBand::Fine),
        fixture_quality: Some(QualityBand::Common),
        completion_index,
        destination,
    }
}

fn empty_authority(
    catalog: &MaterialCraftingCatalog<'_>,
    materials: Vec<NamedMaterialInstance>,
    items: Vec<ItemInstance>,
    targets: Vec<FixtureTarget>,
) -> MaterialCraftingAuthority {
    MaterialCraftingAuthority::new(
        catalog,
        all_capabilities(),
        materials,
        QualityLotLedger::new(Vec::new(), items).unwrap(),
        targets,
    )
    .unwrap()
}

fn command(
    authority: &MaterialCraftingAuthority,
    id: &str,
    operation: MaterialCommandOperation,
) -> MaterialCommand {
    MaterialCommand {
        command_id: id.to_owned(),
        expected_version: authority.version(),
        operation,
    }
}

fn process(
    authority: &mut MaterialCraftingAuthority,
    catalog: &MaterialCraftingCatalog<'_>,
    material: &str,
    command_id: &str,
) {
    let id = instance_id(&format!("drop_{material}"));
    let command = command(
        authority,
        command_id,
        MaterialCommandOperation::Process {
            material_instance_id: id.clone(),
            station_id: item_definition("tannery"),
        },
    );
    authority.execute(catalog, command).unwrap();
    assert_eq!(
        authority.material(&id).unwrap().state,
        MaterialProcessingState::Processed
    );
}

fn weapon(id: &str, location: LotLocation) -> ItemInstance {
    ItemInstance {
        id: instance_id(id),
        definition_id: item_definition("weapon"),
        material_id: material_id("warg_fang"),
        quality: QualityBand::Fine,
        durability: 93,
        location,
        reservation: None,
        equipment_slot: None,
        augmentation_slot: Some(AugmentationSlot::Weapon),
        augmentation: None,
    }
}

fn fixture_item(id: &str, location: LotLocation) -> ItemInstance {
    ItemInstance {
        id: instance_id(id),
        definition_id: item_definition("cookhouse_fixture"),
        material_id: material_id("boar_tusk"),
        quality: QualityBand::Superior,
        durability: 77,
        location,
        reservation: None,
        equipment_slot: None,
        augmentation_slot: None,
        augmentation: None,
    }
}

fn cookhouse_target(id: &str) -> FixtureTarget {
    FixtureTarget {
        target_id: id.to_owned(),
        station_id: item_definition("cookhouse"),
        fixture: FixtureInstance {
            slot: FixtureSlot::Cookhouse,
            installed: None,
            reserved: false,
        },
    }
}

#[test]
fn lai43_catalog_preserves_all_twenty_manifest_rows_and_station_scope_exactly() {
    let manifest = ContentManifest::embedded();
    let catalog = catalog();
    assert_eq!(catalog.materials(), manifest.materials.as_slice());
    assert_eq!(catalog.materials().len(), 20);
    for (expected_id, actual) in PLAN1_RARE_MATERIAL_IDS.iter().zip(catalog.materials()) {
        assert_eq!(*expected_id, actual.id.as_str());
        assert_ne!(actual.raw_state, actual.processed_state);
        assert!(!actual.uses.is_empty());
        assert!(actual.canonical_capability.required_id().is_some());
        assert!((1..=10).contains(&actual.hole_darkness_gate));
        assert!(actual.hole_value_milli > 0);
    }
    assert_eq!(
        manifest
            .stations
            .iter()
            .filter(|station| station.id.as_str().contains("cloth"))
            .count(),
        1
    );
    assert_eq!(manifest.stations.len(), 15);
}

#[test]
fn lai43_all_twenty_raw_materials_process_once_without_identity_or_quality_laundering() {
    let catalog = catalog();
    let materials = PLAN1_RARE_MATERIAL_IDS
        .iter()
        .map(|id| raw_material(&catalog, id))
        .collect::<Vec<_>>();
    let originals = materials
        .iter()
        .map(|material| {
            (
                material.instance_id.clone(),
                material.quality,
                material.provenance.clone(),
            )
        })
        .collect::<Vec<_>>();
    let mut authority = empty_authority(&catalog, materials, Vec::new(), Vec::new());

    for (index, material) in PLAN1_RARE_MATERIAL_IDS.iter().enumerate() {
        process(
            &mut authority,
            &catalog,
            material,
            &format!("process_{index}"),
        );
    }
    for (id, quality, provenance) in originals {
        let processed = authority.material(&id).unwrap();
        assert_eq!(processed.instance_id, id);
        assert_eq!(processed.quality, quality);
        assert_eq!(processed.provenance, provenance);
        assert!(matches!(
            processed.location,
            LotLocation::StationOutput(ref station) if station == "tannery"
        ));
    }

    let before = serde_json::to_string(&authority).unwrap();
    let duplicate = command(
        &authority,
        "process_again",
        MaterialCommandOperation::Process {
            material_instance_id: instance_id("drop_fox_pelt"),
            station_id: item_definition("tannery"),
        },
    );
    assert!(authority.execute(&catalog, duplicate).is_err());
    assert_eq!(serde_json::to_string(&authority).unwrap(), before);
}

#[test]
fn lai43_one_material_can_mint_exactly_one_item_and_receipts_replay_or_conflict() {
    let catalog = catalog();
    let raw = raw_material(&catalog, "fox_pelt");
    let source_id = raw.instance_id.clone();
    let mut authority = empty_authority(&catalog, vec![raw], Vec::new(), Vec::new());
    process(&mut authority, &catalog, "fox_pelt", "process_fox");

    let craft = command(
        &authority,
        "craft_fox_clothing",
        MaterialCommandOperation::CraftItem {
            material_instance_id: source_id.clone(),
            station_id: item_definition("clothier"),
            output_content_id: content_id("item_treated_pelt_clothing"),
            context: context(LotLocation::Stockpile("tailor".to_owned()), 9),
        },
    );
    let receipt = authority.execute(&catalog, craft.clone()).unwrap();
    let produced_id = match &receipt.result {
        MaterialCommandResult::Crafted {
            consumed_material_id,
            produced_item_id,
        } => {
            assert_eq!(consumed_material_id, &source_id);
            produced_item_id.clone()
        }
        other => panic!("unexpected result: {other:?}"),
    };
    assert!(authority.material(&source_id).is_none());
    let produced = authority.ledger().item(&produced_id).unwrap();
    assert_eq!(
        produced.definition_id,
        item_definition("treated_pelt_clothing")
    );
    assert_eq!(produced.material_id, material_id("fox_pelt"));

    let replay = authority.execute(&catalog, craft).unwrap();
    assert_eq!(replay, receipt);
    assert_eq!(authority.ledger().items().count(), 1);

    let conflicting = MaterialCommand {
        command_id: "craft_fox_clothing".to_owned(),
        expected_version: authority.version(),
        operation: MaterialCommandOperation::RecoverItem {
            item_id: produced_id,
            reason: MaterialRecoveryReason::Cancelled,
            destination: RecoveryDestination::Stockpile("other".to_owned()),
        },
    };
    assert!(authority.execute(&catalog, conflicting).is_err());

    let second_mint = command(
        &authority,
        "craft_fox_again",
        MaterialCommandOperation::CraftItem {
            material_instance_id: source_id,
            station_id: item_definition("clothier"),
            output_content_id: content_id("item_treated_pelt_clothing"),
            context: context(LotLocation::Stockpile("tailor".to_owned()), 10),
        },
    );
    assert!(authority.execute(&catalog, second_mint).is_err());
    assert_eq!(authority.ledger().items().count(), 1);
}

#[test]
fn lai43_manifest_use_and_base_material_compatibility_are_both_mandatory() {
    let catalog = catalog();
    let raw = raw_material(&catalog, "badger_pelt");
    let mut authority = empty_authority(&catalog, vec![raw], Vec::new(), Vec::new());
    process(&mut authority, &catalog, "badger_pelt", "process_badger");
    let before = serde_json::to_string(&authority).unwrap();

    let base_material_mismatch = command(
        &authority,
        "badger_cannot_use_fox_base",
        MaterialCommandOperation::CraftItem {
            material_instance_id: instance_id("drop_badger_pelt"),
            station_id: item_definition("clothier"),
            output_content_id: content_id("item_treated_pelt_clothing"),
            context: context(LotLocation::Stockpile("tailor".to_owned()), 1),
        },
    );
    assert!(authority.execute(&catalog, base_material_mismatch).is_err());
    assert_eq!(serde_json::to_string(&authority).unwrap(), before);

    let wrong_manifest_output = command(
        &authority,
        "badger_wrong_output",
        MaterialCommandOperation::CraftItem {
            material_instance_id: instance_id("drop_badger_pelt"),
            station_id: item_definition("clothier"),
            output_content_id: content_id("item_membrane_clothing"),
            context: context(LotLocation::Stockpile("tailor".to_owned()), 1),
        },
    );
    assert!(authority.execute(&catalog, wrong_manifest_output).is_err());
    assert_eq!(serde_json::to_string(&authority).unwrap(), before);
}

#[test]
fn lai43_augmentation_round_trip_retains_the_exact_physical_instance_and_wear() {
    let catalog = catalog();
    let raw = raw_material(&catalog, "warg_fang");
    let target = weapon("weapon_target", LotLocation::Stockpile("armory".to_owned()));
    let mut authority = empty_authority(&catalog, vec![raw], vec![target], Vec::new());
    process(&mut authority, &catalog, "warg_fang", "process_fang");

    let craft = command(
        &authority,
        "craft_weapon_augmentation",
        MaterialCommandOperation::CraftAugmentation {
            material_instance_id: instance_id("drop_warg_fang"),
            augmentation_id: item_definition("weapon_augmentation"),
            station_id: item_definition("smithy"),
            context: context(LotLocation::Stockpile("armory".to_owned()), 12),
        },
    );
    let crafted = authority.execute(&catalog, craft).unwrap();
    let augmentation_id = match crafted.result {
        MaterialCommandResult::Crafted {
            produced_item_id, ..
        } => produced_item_id,
        other => panic!("unexpected result: {other:?}"),
    };
    let original = authority.ledger().item(&augmentation_id).unwrap().clone();

    let install = command(
        &authority,
        "install_weapon_augmentation",
        MaterialCommandOperation::InstallAugmentation {
            target_item_id: instance_id("weapon_target"),
            augmentation_item_id: augmentation_id.clone(),
        },
    );
    authority.execute(&catalog, install).unwrap();
    assert!(authority.ledger().item(&augmentation_id).is_none());
    let installed = authority
        .ledger()
        .item(&instance_id("weapon_target"))
        .unwrap()
        .augmentation
        .as_ref()
        .unwrap();
    assert_eq!(installed.item.id, original.id);
    assert_eq!(installed.item.material_id, original.material_id);
    assert_eq!(installed.item.quality, original.quality);
    assert_eq!(installed.item.durability, original.durability);

    let wear = command(
        &authority,
        "wear_installed_augmentation",
        MaterialCommandOperation::Wear {
            target: DurabilityTarget::InstalledAugmentation(instance_id("weapon_target")),
            amount: 7,
        },
    );
    authority.execute(&catalog, wear).unwrap();
    let remove = command(
        &authority,
        "cancel_augmentation",
        MaterialCommandOperation::RemoveAugmentation {
            target_item_id: instance_id("weapon_target"),
            destination: RecoveryDestination::Origin,
        },
    );
    authority.execute(&catalog, remove).unwrap();
    let recovered = authority.ledger().item(&augmentation_id).unwrap();
    assert_eq!(recovered.id, original.id);
    assert_eq!(recovered.definition_id, original.definition_id);
    assert_eq!(recovered.material_id, original.material_id);
    assert_eq!(recovered.quality, original.quality);
    assert_eq!(recovered.durability, original.durability - 7);
    assert_eq!(recovered.location, original.location);
}

#[test]
fn lai43_fixture_round_trip_retains_exact_identity_quality_durability_and_target_slot() {
    let catalog = catalog();
    let fixture = fixture_item(
        "fixture_physical_one",
        LotLocation::Stockpile("fixtures".to_owned()),
    );
    let original = fixture.clone();
    let mut authority = empty_authority(
        &catalog,
        Vec::new(),
        vec![fixture],
        vec![cookhouse_target("cookhouse_7")],
    );

    let install = command(
        &authority,
        "install_fixture",
        MaterialCommandOperation::InstallFixture {
            target_id: "cookhouse_7".to_owned(),
            fixture_item_id: original.id.clone(),
        },
    );
    authority.execute(&catalog, install).unwrap();
    let installed = authority
        .fixture_target("cookhouse_7")
        .unwrap()
        .fixture
        .installed
        .as_ref()
        .unwrap();
    assert_eq!(installed.item.id, original.id);
    assert_eq!(installed.item.material_id, original.material_id);
    assert_eq!(installed.item.quality, original.quality);
    assert_eq!(installed.item.durability, original.durability);
    assert_eq!(installed.slot, FixtureSlot::Cookhouse);

    let wear = command(
        &authority,
        "wear_fixture",
        MaterialCommandOperation::Wear {
            target: DurabilityTarget::InstalledFixture("cookhouse_7".to_owned()),
            amount: 10,
        },
    );
    authority.execute(&catalog, wear).unwrap();
    let remove = command(
        &authority,
        "route_loss_fixture",
        MaterialCommandOperation::RemoveFixture {
            target_id: "cookhouse_7".to_owned(),
            destination: RecoveryDestination::Cache("last_land_tile_3_4".to_owned()),
        },
    );
    authority.execute(&catalog, remove).unwrap();
    let recovered = authority.ledger().item(&original.id).unwrap();
    assert_eq!(recovered.id, original.id);
    assert_eq!(recovered.material_id, original.material_id);
    assert_eq!(recovered.quality, original.quality);
    assert_eq!(recovered.durability, original.durability - 10);
    assert_eq!(
        recovered.location,
        LotLocation::Cache("last_land_tile_3_4".to_owned())
    );
}

#[test]
fn lai43_reserved_carried_equipped_broken_occupied_and_wrong_compatibility_reject_atomically() {
    let catalog = catalog();
    for (index, mut target) in [
        weapon("target_0", LotLocation::Stockpile("armory".to_owned())),
        weapon("target_1", LotLocation::Cargo("carrier".to_owned())),
        weapon("target_2", LotLocation::Stockpile("armory".to_owned())),
        weapon("target_3", LotLocation::Stockpile("armory".to_owned())),
    ]
    .into_iter()
    .enumerate()
    {
        match index {
            0 => target.reservation = Some("reserved".to_owned()),
            2 => target.equipment_slot = Some(EquipmentSlot::MainHand),
            3 => target.durability = 0,
            _ => {}
        }
        let mut augmentation = weapon(
            &format!("augmentation_{index}"),
            LotLocation::Stockpile("armory".to_owned()),
        );
        augmentation.definition_id = item_definition("weapon_augmentation");
        let target_id = target.id.clone();
        let augmentation_id = augmentation.id.clone();
        let mut authority =
            empty_authority(&catalog, Vec::new(), vec![target, augmentation], Vec::new());
        let before = serde_json::to_string(&authority).unwrap();
        let install = command(
            &authority,
            &format!("ineligible_target_{index}"),
            MaterialCommandOperation::InstallAugmentation {
                target_item_id: target_id,
                augmentation_item_id: augmentation_id,
            },
        );
        assert!(authority.execute(&catalog, install).is_err());
        assert_eq!(serde_json::to_string(&authority).unwrap(), before);
    }

    let mut wrong_class = weapon(
        "wrong_class_target",
        LotLocation::Stockpile("armory".to_owned()),
    );
    wrong_class.definition_id = item_definition("generic_tool");
    wrong_class.augmentation_slot = Some(AugmentationSlot::Tool);
    let mut augmentation = weapon(
        "weapon_augmentation_physical",
        LotLocation::Stockpile("armory".to_owned()),
    );
    augmentation.definition_id = item_definition("weapon_augmentation");
    let mut wrong_class_authority = empty_authority(
        &catalog,
        Vec::new(),
        vec![wrong_class, augmentation],
        Vec::new(),
    );
    let before = serde_json::to_string(&wrong_class_authority).unwrap();
    let install = command(
        &wrong_class_authority,
        "wrong_class",
        MaterialCommandOperation::InstallAugmentation {
            target_item_id: instance_id("wrong_class_target"),
            augmentation_item_id: instance_id("weapon_augmentation_physical"),
        },
    );
    assert!(wrong_class_authority.execute(&catalog, install).is_err());
    assert_eq!(
        serde_json::to_string(&wrong_class_authority).unwrap(),
        before
    );

    let mut broken_fixture =
        fixture_item("broken_fixture", LotLocation::Cargo("carrier".to_owned()));
    broken_fixture.durability = 0;
    let mut occupied = cookhouse_target("cookhouse_occupied");
    occupied.fixture.reserved = true;
    let mut fixture_authority =
        empty_authority(&catalog, Vec::new(), vec![broken_fixture], vec![occupied]);
    let before = serde_json::to_string(&fixture_authority).unwrap();
    let install = command(
        &fixture_authority,
        "broken_fixture_install",
        MaterialCommandOperation::InstallFixture {
            target_id: "cookhouse_occupied".to_owned(),
            fixture_item_id: instance_id("broken_fixture"),
        },
    );
    assert!(fixture_authority.execute(&catalog, install).is_err());
    assert_eq!(serde_json::to_string(&fixture_authority).unwrap(), before);

    let wrong_station = FixtureTarget {
        target_id: "fishing_hut_wrong_fixture".to_owned(),
        station_id: item_definition("fishing_hut"),
        fixture: FixtureInstance {
            slot: FixtureSlot::FishingHut,
            installed: None,
            reserved: false,
        },
    };
    let fixture = fixture_item(
        "cookhouse_fixture_wrong_station",
        LotLocation::Stockpile("fixtures".to_owned()),
    );
    let mut wrong_station_authority =
        empty_authority(&catalog, Vec::new(), vec![fixture], vec![wrong_station]);
    let before = serde_json::to_string(&wrong_station_authority).unwrap();
    let install = command(
        &wrong_station_authority,
        "wrong_station",
        MaterialCommandOperation::InstallFixture {
            target_id: "fishing_hut_wrong_fixture".to_owned(),
            fixture_item_id: instance_id("cookhouse_fixture_wrong_station"),
        },
    );
    assert!(wrong_station_authority.execute(&catalog, install).is_err());
    assert_eq!(
        serde_json::to_string(&wrong_station_authority).unwrap(),
        before
    );
}

#[test]
fn lai43_cancellation_death_and_route_recovery_return_same_identity_to_origin_stockpile_or_cache() {
    let catalog = catalog();
    let cases = [
        (
            MaterialRecoveryReason::Cancelled,
            RecoveryDestination::Origin,
            LotLocation::Cargo("carrier".to_owned()),
        ),
        (
            MaterialRecoveryReason::CarrierDeath,
            RecoveryDestination::Stockpile("nearest".to_owned()),
            LotLocation::Stockpile("nearest".to_owned()),
        ),
        (
            MaterialRecoveryReason::RouteLost,
            RecoveryDestination::Cache("tile_9_2".to_owned()),
            LotLocation::Cache("tile_9_2".to_owned()),
        ),
    ];
    for (index, (reason, destination, expected)) in cases.into_iter().enumerate() {
        let mut item = weapon(
            &format!("recovery_item_{index}"),
            LotLocation::Cargo("carrier".to_owned()),
        );
        item.reservation = Some(format!("route_{index}"));
        let id = item.id.clone();
        let mut authority = empty_authority(&catalog, Vec::new(), vec![item], Vec::new());
        let recovery = command(
            &authority,
            &format!("recover_{index}"),
            MaterialCommandOperation::RecoverItem {
                item_id: id.clone(),
                reason,
                destination,
            },
        );
        authority.execute(&catalog, recovery).unwrap();
        let recovered = authority.ledger().item(&id).unwrap();
        assert_eq!(recovered.id, id);
        assert_eq!(recovered.location, expected);
        assert_eq!(recovered.reservation, None);
    }
}

#[test]
fn lai43_logs_to_planks_consumes_exact_physical_logs_and_produces_quality_planks_globally() {
    let catalog = catalog();
    let logs = PhysicalLot {
        id: lot_id("logs_lot_exact"),
        key: BulkLotKey::new(content_id("resource_logs"), QualityBand::Fine),
        provenance: LotProvenance {
            origin: "oak_tree_11".to_owned(),
            created_tick: 3,
        },
        quantity: 2,
        location: LotLocation::StationInput("wood_cutter".to_owned()),
        reservation: None,
    };
    let mut authority = MaterialCraftingAuthority::new(
        &catalog,
        all_capabilities(),
        Vec::new(),
        QualityLotLedger::new(vec![logs], Vec::new()).unwrap(),
        Vec::new(),
    )
    .unwrap();
    let execute = command(
        &authority,
        "global_logs_to_planks",
        MaterialCommandOperation::LogsToPlanks {
            input_lot_id: lot_id("logs_lot_exact"),
            station_id: item_definition("wood_cutter"),
            context: context(LotLocation::Stockpile("timber".to_owned()), 88),
        },
    );
    let receipt = authority.execute(&catalog, execute.clone()).unwrap();
    let output_id = match receipt.result {
        MaterialCommandResult::PlanksProduced {
            consumed_lot_id,
            produced_lot_id,
        } => {
            assert_eq!(consumed_lot_id, lot_id("logs_lot_exact"));
            produced_lot_id
        }
        other => panic!("unexpected result: {other:?}"),
    };
    assert_eq!(
        authority
            .ledger()
            .lot(&lot_id("logs_lot_exact"))
            .unwrap()
            .quantity,
        1
    );
    let planks = authority.ledger().lot(&output_id).unwrap();
    assert_eq!(planks.key.content_id, content_id("resource_planks"));
    assert_eq!(planks.quantity, 1);
    assert!(planks.key.quality >= QualityBand::Common);

    let saved = serde_json::to_string(&authority).unwrap();
    let replay = authority.execute(&catalog, execute).unwrap();
    assert_eq!(replay.resulting_version, authority.version());
    assert_eq!(serde_json::to_string(&authority).unwrap(), saved);
}

#[test]
fn lai43_restart_partition_order_and_strict_bounded_state_are_identical() {
    let catalog = catalog();
    let materials = vec![
        raw_material(&catalog, "dragon_heart"),
        raw_material(&catalog, "bat_wing"),
    ];
    let authority = empty_authority(&catalog, materials, Vec::new(), Vec::new());
    let encoded = serde_json::to_string(&authority).unwrap();
    assert!(encoded.find("drop_bat_wing").unwrap() < encoded.find("drop_dragon_heart").unwrap());
    let restarted: MaterialCraftingAuthority = serde_json::from_str(&encoded).unwrap();
    assert_eq!(restarted, authority);

    let mut left = restarted.clone();
    let mut right: MaterialCraftingAuthority = serde_json::from_str(&encoded).unwrap();
    let operation = MaterialCommandOperation::Process {
        material_instance_id: instance_id("drop_bat_wing"),
        station_id: item_definition("tannery"),
    };
    let command = command(&left, "partition_process", operation);
    assert_eq!(
        left.execute(&catalog, command.clone()).unwrap(),
        right.execute(&catalog, command).unwrap()
    );
    assert_eq!(
        serde_json::to_string(&left).unwrap(),
        serde_json::to_string(&right).unwrap()
    );

    let mut future = serde_json::to_value(&authority).unwrap();
    future["schemaVersion"] = serde_json::json!(MATERIAL_CRAFTING_SCHEMA_VERSION + 1);
    assert!(serde_json::from_value::<MaterialCraftingAuthority>(future).is_err());
    let mut unknown = serde_json::to_value(&authority).unwrap();
    unknown["futureField"] = serde_json::json!(true);
    assert!(serde_json::from_value::<MaterialCraftingAuthority>(unknown).is_err());
    let mut malformed = serde_json::to_value(&authority).unwrap();
    malformed["materials"][0]["contentState"] = serde_json::json!("processed_bat_wing");
    assert!(serde_json::from_value::<MaterialCraftingAuthority>(malformed).is_err());

    let mut bounded = serde_json::to_value(&left).unwrap();
    let receipt = bounded["receipts"][0].clone();
    bounded["receipts"] = serde_json::Value::Array(
        (0..=MAX_MATERIAL_RECEIPTS)
            .map(|_| receipt.clone())
            .collect(),
    );
    assert!(serde_json::from_value::<MaterialCraftingAuthority>(bounded).is_err());
}

#[test]
fn lai43_expected_version_and_every_rejection_leave_authority_byte_identical() {
    let catalog = catalog();
    let raw = raw_material(&catalog, "fox_pelt");
    let mut authority = empty_authority(&catalog, vec![raw], Vec::new(), Vec::new());
    let before = serde_json::to_string(&authority).unwrap();
    let stale = MaterialCommand {
        command_id: "stale".to_owned(),
        expected_version: 99,
        operation: MaterialCommandOperation::Process {
            material_instance_id: instance_id("drop_fox_pelt"),
            station_id: item_definition("tannery"),
        },
    };
    assert!(authority.execute(&catalog, stale).is_err());
    assert_eq!(serde_json::to_string(&authority).unwrap(), before);

    let raw_craft = command(
        &authority,
        "raw_craft",
        MaterialCommandOperation::CraftItem {
            material_instance_id: instance_id("drop_fox_pelt"),
            station_id: item_definition("clothier"),
            output_content_id: content_id("item_treated_pelt_clothing"),
            context: context(LotLocation::Stockpile("tailor".to_owned()), 1),
        },
    );
    assert!(authority.execute(&catalog, raw_craft).is_err());
    assert_eq!(serde_json::to_string(&authority).unwrap(), before);
}

#[test]
fn lai43_locked_capability_blocks_processing_and_plank_execution_without_partial_debit() {
    let catalog = catalog();
    let raw = raw_material(&catalog, "fox_pelt");
    let logs = PhysicalLot {
        id: lot_id("locked_logs"),
        key: BulkLotKey::new(content_id("resource_logs"), QualityBand::Common),
        provenance: LotProvenance {
            origin: "tree".to_owned(),
            created_tick: 1,
        },
        quantity: 1,
        location: LotLocation::Stockpile("main".to_owned()),
        reservation: None,
    };
    let mut authority = MaterialCraftingAuthority::new(
        &catalog,
        CapabilitySet::empty(),
        vec![raw],
        QualityLotLedger::new(vec![logs], Vec::new()).unwrap(),
        Vec::new(),
    )
    .unwrap();
    let before = serde_json::to_string(&authority).unwrap();
    let process = command(
        &authority,
        "locked_process",
        MaterialCommandOperation::Process {
            material_instance_id: instance_id("drop_fox_pelt"),
            station_id: item_definition("tannery"),
        },
    );
    assert!(authority.execute(&catalog, process).is_err());
    let planks = command(
        &authority,
        "locked_planks",
        MaterialCommandOperation::LogsToPlanks {
            input_lot_id: lot_id("locked_logs"),
            station_id: item_definition("wood_cutter"),
            context: context(LotLocation::Stockpile("main".to_owned()), 2),
        },
    );
    assert!(authority.execute(&catalog, planks).is_err());
    assert_eq!(serde_json::to_string(&authority).unwrap(), before);
}
