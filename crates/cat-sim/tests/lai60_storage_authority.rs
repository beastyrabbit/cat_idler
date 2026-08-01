//! LAI.60-A canonical physical storage authority contracts.

use cat_sim::{
    content_manifest::{
        ContentId, ItemDefinitionId, MaterialId, MaterialInstanceId, PhysicalLotId,
    },
    physical_storage::{ContainerKind, StorageCompatibility, VISIBLE_SLOTS_PER_STORAGE_TILE},
    quality_lots::{
        BulkLotKey, ItemInstance, LotLocation, LotProvenance, PhysicalLot, QualityBand,
    },
    spatial_tasks::{Rect, TaskFootprint, TilePoint},
    storage_authority::{
        StorageAddress, StorageAuthority, StorageAuthorityError, StorageCommand,
        StorageCommandEnvelope, StorageContainer, StorageIdentity, StorageZone, StorageZoneKind,
        WorkshopStorageLink,
    },
};

fn lot(id: &str, content: &str, units: u32) -> PhysicalLot {
    PhysicalLot {
        id: PhysicalLotId::new(id).unwrap(),
        key: BulkLotKey::new(ContentId::new(content).unwrap(), QualityBand::Fine),
        provenance: LotProvenance {
            origin: "gathering:forest".to_owned(),
            created_tick: 9,
        },
        quantity: units,
        location: LotLocation::Source("source:forest".to_owned()),
        reservation: None,
    }
}

fn item(id: &str) -> ItemInstance {
    ItemInstance {
        id: MaterialInstanceId::new(id).unwrap(),
        definition_id: ItemDefinitionId::new("item_tool").unwrap(),
        material_id: MaterialId::new("material_iron").unwrap(),
        quality: QualityBand::Superior,
        durability: 73,
        location: LotLocation::Source("source:forge".to_owned()),
        reservation: None,
        equipment_slot: None,
        augmentation_slot: None,
        augmentation: None,
    }
}

fn zone(id: &str, kind: StorageZoneKind, x: i32, y: i32, width: i32, height: i32) -> StorageZone {
    StorageZone::new(
        id,
        kind,
        TaskFootprint::rectangular(Rect::try_new(TilePoint { x, y }, width, height).unwrap()),
    )
    .unwrap()
}

fn envelope(sequence: u64, command: StorageCommand) -> StorageCommandEnvelope {
    StorageCommandEnvelope {
        colony_id: "colony_one".to_owned(),
        command_id: format!("command_{sequence}"),
        fingerprint: format!("fingerprint_{sequence}"),
        sequence,
        command,
    }
}

fn register(authority: &mut StorageAuthority, sequence: u64, zone: StorageZone) {
    authority
        .execute(envelope(sequence, StorageCommand::RegisterZone { zone }))
        .unwrap();
}

#[test]
fn exact_visible_slots_container_capacities_and_compatibility_are_physical() {
    assert_eq!(VISIBLE_SLOTS_PER_STORAGE_TILE, 4);
    assert_eq!(ContainerKind::Basket.lot_capacity(), 4);
    assert_eq!(ContainerKind::Barrel.lot_capacity(), 8);
    assert_eq!(ContainerKind::Crate.lot_capacity(), 8);
    assert_eq!(ContainerKind::Chest.lot_capacity(), 16);
    assert_eq!(ContainerKind::Rack.lot_capacity(), 8);

    let mut authority = StorageAuthority::new("colony_one").unwrap();
    register(
        &mut authority,
        1,
        zone("zone_main", StorageZoneKind::Stockpile, 0, 0, 1, 1),
    );
    for slot in 0..4 {
        authority
            .execute(envelope(
                slot + 2,
                StorageCommand::DepositLot {
                    lot: lot(&format!("lot_{slot}"), "resource_logs", 1),
                    compatibility: StorageCompatibility::BulkMaterial,
                    destination: StorageAddress::Loose {
                        zone_id: "zone_main".to_owned(),
                        tile: TilePoint { x: 0, y: 0 },
                        slot: slot as u8,
                    },
                },
            ))
            .unwrap();
    }
    assert_eq!(
        authority
            .fullness("zone_main", TilePoint { x: 0, y: 0 })
            .unwrap(),
        (4, 4)
    );
    let overflow = authority.execute(envelope(
        6,
        StorageCommand::DepositLot {
            lot: lot("lot_overflow", "resource_logs", 1),
            compatibility: StorageCompatibility::BulkMaterial,
            destination: StorageAddress::Loose {
                zone_id: "zone_main".to_owned(),
                tile: TilePoint { x: 0, y: 0 },
                slot: 0,
            },
        },
    ));
    assert!(matches!(overflow, Err(StorageAuthorityError::SlotOccupied)));
}

#[test]
fn workshop_link_is_adjacent_to_the_entire_non_overlapping_three_by_three_footprint() {
    let mut authority = StorageAuthority::new("colony_one").unwrap();
    register(
        &mut authority,
        1,
        zone("zone_inputs", StorageZoneKind::WorkshopInput, 3, 0, 2, 3),
    );
    authority
        .execute(envelope(
            2,
            StorageCommand::LinkWorkshop {
                link: WorkshopStorageLink {
                    workshop_id: "building_workshop".to_owned(),
                    workshop_footprint: TaskFootprint::rectangular(
                        Rect::try_new(TilePoint { x: 0, y: 0 }, 3, 3).unwrap(),
                    ),
                    zone_id: "zone_inputs".to_owned(),
                },
            },
        ))
        .unwrap();
    authority.validate().unwrap();
}

#[test]
fn lots_keep_quality_provenance_age_and_identity_through_split_merge_and_restart() {
    let mut authority = StorageAuthority::new("colony_one").unwrap();
    register(
        &mut authority,
        1,
        zone("zone_main", StorageZoneKind::Stockpile, 0, 0, 2, 1),
    );
    authority
        .execute(envelope(
            2,
            StorageCommand::RegisterContainer {
                container: StorageContainer {
                    id: "container_crate".to_owned(),
                    kind: ContainerKind::Crate,
                    zone_id: "zone_main".to_owned(),
                    tile: TilePoint { x: 0, y: 0 },
                    slot: 0,
                    contents: Default::default(),
                },
            },
        ))
        .unwrap();
    authority
        .execute(envelope(
            3,
            StorageCommand::DepositLot {
                lot: lot("lot_logs", "resource_logs", 8),
                compatibility: StorageCompatibility::BulkMaterial,
                destination: StorageAddress::Container {
                    container_id: "container_crate".to_owned(),
                },
            },
        ))
        .unwrap();
    authority
        .execute(envelope(
            4,
            StorageCommand::SplitBulk {
                source: PhysicalLotId::new("lot_logs").unwrap(),
                split: PhysicalLotId::new("lot_logs_split").unwrap(),
                units: 3,
                destination: StorageAddress::Container {
                    container_id: "container_crate".to_owned(),
                },
            },
        ))
        .unwrap();
    assert_eq!(
        authority
            .ledger()
            .lot(&PhysicalLotId::new("lot_logs_split").unwrap())
            .unwrap()
            .key
            .quality,
        QualityBand::Fine
    );
    authority
        .execute(envelope(
            5,
            StorageCommand::MergeBulk {
                left: PhysicalLotId::new("lot_logs").unwrap(),
                right: PhysicalLotId::new("lot_logs_split").unwrap(),
            },
        ))
        .unwrap();
    assert_eq!(
        authority
            .ledger()
            .lot(&PhysicalLotId::new("lot_logs").unwrap())
            .unwrap()
            .quantity,
        8
    );
    let restored: StorageAuthority =
        serde_json::from_str(&authority.to_canonical_json().unwrap()).unwrap();
    assert_eq!(restored, authority);
}

#[test]
fn exact_item_moves_and_reservation_conflicts_are_atomic() {
    let mut authority = StorageAuthority::new("colony_one").unwrap();
    register(
        &mut authority,
        1,
        zone("zone_main", StorageZoneKind::Stockpile, 0, 0, 1, 1),
    );
    authority
        .execute(envelope(
            2,
            StorageCommand::RegisterContainer {
                container: StorageContainer {
                    id: "container_rack".to_owned(),
                    kind: ContainerKind::Rack,
                    zone_id: "zone_main".to_owned(),
                    tile: TilePoint { x: 0, y: 0 },
                    slot: 0,
                    contents: Default::default(),
                },
            },
        ))
        .unwrap();
    authority
        .execute(envelope(
            3,
            StorageCommand::DepositItem {
                item: item("item_hammer"),
                compatibility: StorageCompatibility::Tool,
                destination: StorageAddress::Container {
                    container_id: "container_rack".to_owned(),
                },
            },
        ))
        .unwrap();
    let identity = StorageIdentity::Item(MaterialInstanceId::new("item_hammer").unwrap());
    authority
        .execute(envelope(
            4,
            StorageCommand::Reserve {
                identity: identity.clone(),
                owner: "project_house".to_owned(),
            },
        ))
        .unwrap();
    let rejected = authority.execute(envelope(
        5,
        StorageCommand::Move {
            identity: identity.clone(),
            destination: StorageAddress::RouteCargo {
                route_id: "route_home".to_owned(),
            },
        },
    ));
    assert!(matches!(rejected, Err(StorageAuthorityError::Reserved(_))));
    assert_eq!(
        authority.location(&identity),
        Some(&StorageAddress::Container {
            container_id: "container_rack".to_owned()
        })
    );
}

#[test]
fn construction_staging_consumption_and_recovery_choose_only_real_destinations() {
    let mut authority = StorageAuthority::new("colony_one").unwrap();
    register(
        &mut authority,
        1,
        zone("zone_origin", StorageZoneKind::Stockpile, 0, 0, 1, 1),
    );
    register(
        &mut authority,
        2,
        zone("zone_fallback", StorageZoneKind::Stockpile, 2, 0, 1, 1),
    );
    let identity = StorageIdentity::Lot(PhysicalLotId::new("lot_logs").unwrap());
    authority
        .execute(envelope(
            3,
            StorageCommand::DepositLot {
                lot: lot("lot_logs", "resource_logs", 3),
                compatibility: StorageCompatibility::BulkMaterial,
                destination: StorageAddress::Loose {
                    zone_id: "zone_origin".to_owned(),
                    tile: TilePoint { x: 0, y: 0 },
                    slot: 0,
                },
            },
        ))
        .unwrap();
    authority
        .execute(envelope(
            4,
            StorageCommand::StageConstruction {
                project_id: "project_home".to_owned(),
                identities: vec![identity.clone()],
            },
        ))
        .unwrap();
    assert!(matches!(
        authority.location(&identity),
        Some(StorageAddress::ConstructionCargo { .. })
    ));
    authority
        .execute(envelope(
            5,
            StorageCommand::Recover {
                identities: vec![identity.clone()],
                origin: Some(StorageAddress::Loose {
                    zone_id: "zone_origin".to_owned(),
                    tile: TilePoint { x: 0, y: 0 },
                    slot: 0,
                }),
                stockpile: Some(StorageAddress::Loose {
                    zone_id: "zone_fallback".to_owned(),
                    tile: TilePoint { x: 2, y: 0 },
                    slot: 0,
                }),
                cache: None,
            },
        ))
        .unwrap();
    authority
        .execute(envelope(
            6,
            StorageCommand::Consume {
                bulk: vec![(PhysicalLotId::new("lot_logs").unwrap(), 3)],
                items: Vec::new(),
            },
        ))
        .unwrap();
    assert!(
        authority
            .ledger()
            .lot(&PhysicalLotId::new("lot_logs").unwrap())
            .is_none()
    );
}

#[test]
fn zones_and_containers_cannot_be_removed_while_occupied_or_reserved() {
    let mut authority = StorageAuthority::new("colony_one").unwrap();
    register(
        &mut authority,
        1,
        zone("zone_main", StorageZoneKind::Stockpile, 0, 0, 1, 1),
    );
    authority
        .execute(envelope(
            2,
            StorageCommand::RegisterContainer {
                container: StorageContainer {
                    id: "container_crate".to_owned(),
                    kind: ContainerKind::Crate,
                    zone_id: "zone_main".to_owned(),
                    tile: TilePoint { x: 0, y: 0 },
                    slot: 0,
                    contents: Default::default(),
                },
            },
        ))
        .unwrap();
    authority
        .execute(envelope(
            3,
            StorageCommand::DepositLot {
                lot: lot("lot_logs", "resource_logs", 1),
                compatibility: StorageCompatibility::BulkMaterial,
                destination: StorageAddress::Container {
                    container_id: "container_crate".to_owned(),
                },
            },
        ))
        .unwrap();
    assert!(matches!(
        authority.execute(envelope(
            4,
            StorageCommand::RemoveContainer {
                container_id: "container_crate".to_owned()
            }
        )),
        Err(StorageAuthorityError::RemovalBlocked)
    ));
    assert!(matches!(
        authority.execute(envelope(
            5,
            StorageCommand::RemoveZone {
                zone_id: "zone_main".to_owned()
            }
        )),
        Err(StorageAuthorityError::RemovalBlocked)
    ));
}

#[test]
fn replay_conflicts_receipt_compaction_and_strict_decode_fail_closed() {
    let mut authority = StorageAuthority::new("colony_one").unwrap();
    let command = envelope(
        1,
        StorageCommand::RegisterZone {
            zone: zone("zone_main", StorageZoneKind::Stockpile, 0, 0, 1, 1),
        },
    );
    let first = authority.execute(command.clone()).unwrap();
    assert_eq!(authority.execute(command).unwrap(), first);
    let conflict = StorageCommandEnvelope {
        colony_id: "colony_one".to_owned(),
        command_id: "command_1".to_owned(),
        fingerprint: "different".to_owned(),
        sequence: 1,
        command: StorageCommand::RemoveZone {
            zone_id: "zone_main".to_owned(),
        },
    };
    assert!(matches!(
        authority.execute(conflict),
        Err(StorageAuthorityError::ReplayConflict)
    ));
    authority.drain_terminal_receipts_through(1);
    assert!(matches!(
        authority.execute(envelope(
            1,
            StorageCommand::RemoveZone {
                zone_id: "zone_main".to_owned()
            }
        )),
        Err(StorageAuthorityError::ReceiptDrained)
    ));
    let json = authority.to_canonical_json().unwrap();
    assert!(
        serde_json::from_str::<StorageAuthority>(
            &json.replace("\"schemaVersion\":1", "\"schemaVersion\":2")
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<StorageAuthority>(&format!(
            "{}{}",
            &json[..json.len() - 1],
            ",\"unknown\":true}"
        ))
        .is_err()
    );
}

#[test]
fn foreign_colony_and_legacy_scalar_currency_vocabulary_have_no_authority_path() {
    let mut authority = StorageAuthority::new("colony_one").unwrap();
    assert_eq!(authority.colony_id(), "colony_one");
    let foreign = StorageCommandEnvelope {
        colony_id: "colony_two".to_owned(),
        command_id: "command_foreign".to_owned(),
        fingerprint: "fingerprint_foreign".to_owned(),
        sequence: 1,
        command: StorageCommand::RegisterZone {
            zone: zone("zone_foreign", StorageZoneKind::Stockpile, 0, 0, 1, 1),
        },
    };
    assert!(matches!(
        authority.execute(foreign),
        Err(StorageAuthorityError::WrongColony)
    ));
    let source = include_str!("../src/storage_authority.rs");
    for forbidden in ["coin", "favor", "shrine", "insight", "scalar_inventory"] {
        assert!(
            !source.to_ascii_lowercase().contains(forbidden),
            "legacy authority leaked: {forbidden}"
        );
    }
}
