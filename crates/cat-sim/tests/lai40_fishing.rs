use std::collections::BTreeSet;

use cat_sim::{
    content_manifest::{ContentManifest, ItemDefinitionId, MaterialId, MaterialInstanceId},
    fishing::{
        DockOrientation, FISHING_SCHEMA_VERSION, FishingAttempt, FishingAuthority, FishingError,
        FishingMode, MAX_FISHING_RECEIPTS, fishing_hut_footprint, fishing_profile,
        validate_hut_placement,
    },
    food_ecology::{
        EcologyReport, FishHabitat, FishTask, FoodEcology, FoundingFoodSites, HandFishingRequest,
        ReportAudience, ReportLevel, Tile, WaterSource,
    },
    quality_lots::{ItemInstance, LotLocation, QualityBand, QualityLotLedger, RecoveryReason},
    spatial_tasks::TilePoint,
};

fn ecology() -> FoodEcology {
    let revealed = (-1..=3)
        .flat_map(|x| (-1..=1).map(move |y| Tile { x, y }))
        .collect();
    FoodEcology::new(
        ContentManifest::embedded(),
        FoundingFoodSites {
            revealed_reachable_tiles: revealed,
            water: WaterSource {
                source_tile: Tile { x: 0, y: 0 },
                valid_bank_tile: Tile { x: 1, y: 0 },
            },
            apple_tree_tile: Tile { x: 2, y: 0 },
            fish_habitat: FishHabitat {
                water_tile: Tile { x: 0, y: 0 },
                shoreline_task_tile: Tile { x: 1, y: 0 },
                stock: 24,
                capacity: 24,
                next_replenish_tick: 120,
            },
        },
        0,
    )
    .unwrap()
}
fn rod(quality: QualityBand) -> ItemInstance {
    ItemInstance {
        id: MaterialInstanceId::new("rod_a").unwrap(),
        definition_id: ItemDefinitionId::new("fishing_rod").unwrap(),
        material_id: MaterialId::new("wood").unwrap(),
        quality,
        durability: 2,
        location: LotLocation::Stockpile("store_a".into()),
        reservation: None,
        equipment_slot: None,
        augmentation_slot: None,
        augmentation: None,
    }
}
fn attempt(id: &str, hut: bool) -> FishingAttempt {
    FishingAttempt {
        command_id: id.into(),
        habitat_id: "habitat_a".into(),
        attempt_index: 0,
        world_seed: 7,
        now_game_minute: 1,
        source_quality: QualityBand::Common,
        worker_skill: 40,
        staffed_hut: hut,
        cargo_id: "cargo_a".into(),
    }
}

#[test]
fn exact_profiles_and_rod_quality_reliability_are_public() {
    assert_eq!(fishing_profile(None, false).catch_units, 12);
    assert_eq!(fishing_profile(None, false).cycle_game_minutes, 45);
    assert_eq!(
        fishing_profile(Some(QualityBand::Common), false).reliability_percent,
        90
    );
    assert_eq!(fishing_profile(None, true).catch_units, 18);
    assert_eq!(fishing_profile(None, true).cycle_game_minutes, 30);
    assert_eq!(
        fishing_profile(Some(QualityBand::Common), true).catch_units,
        24
    );
    assert_eq!(
        fishing_profile(Some(QualityBand::Common), true).cycle_game_minutes,
        24
    );
    assert_eq!(
        fishing_profile(Some(QualityBand::Common), true).reliability_percent,
        100
    );
    assert!(
        fishing_profile(Some(QualityBand::Masterwork), false).rod_reliability_contribution_percent
            > fishing_profile(Some(QualityBand::Crude), false).rod_reliability_contribution_percent
    );
    let exact = [
        (QualityBand::Crude, 87, 12),
        (QualityBand::Common, 90, 15),
        (QualityBand::Fine, 92, 17),
        (QualityBand::Superior, 95, 20),
        (QualityBand::Masterwork, 99, 24),
    ];
    for (quality, reliability, contribution) in exact {
        let profile = fishing_profile(Some(quality), false);
        assert_eq!(profile.reliability_percent, reliability);
        assert_eq!(profile.rod_reliability_contribution_percent, contribution);
    }
}

#[test]
fn hut_is_full_land_footprint_with_oriented_dock_and_reserved_water() {
    let footprint =
        fishing_hut_footprint(TilePoint { x: 10, y: 10 }, DockOrientation::East).unwrap();
    assert_eq!(footprint.land.tiles.as_slice().len(), 9);
    assert_eq!(footprint.dock_land, TilePoint { x: 12, y: 11 });
    assert_eq!(footprint.reserved_water, TilePoint { x: 13, y: 11 });
    let land = footprint
        .land
        .tiles
        .as_slice()
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let water = BTreeSet::from([footprint.reserved_water]);
    let reachable = land.clone();
    validate_hut_placement(&footprint, &land, &water, &reachable, &BTreeSet::new()).unwrap();
    assert!(
        validate_hut_placement(
            &footprint,
            &land,
            &BTreeSet::new(),
            &reachable,
            &BTreeSet::new()
        )
        .is_err()
    );
    let mut unreachable = reachable.clone();
    unreachable.remove(&TilePoint { x: 10, y: 10 });
    assert!(matches!(
        validate_hut_placement(&footprint, &land, &water, &unreachable, &BTreeSet::new()),
        Err(FishingError::UnreachableHut)
    ));
    let mut forged_orientation = footprint.clone();
    forged_orientation.orientation = DockOrientation::North;
    assert!(matches!(
        validate_hut_placement(
            &forged_orientation,
            &land,
            &water,
            &reachable,
            &BTreeSet::new()
        ),
        Err(FishingError::InvalidOrientation)
    ));
    for orientation in [
        DockOrientation::North,
        DockOrientation::East,
        DockOrientation::South,
        DockOrientation::West,
    ] {
        let oriented = fishing_hut_footprint(TilePoint { x: 10, y: 10 }, orientation).unwrap();
        assert_eq!(oriented.land.tiles.as_slice().len(), 9);
        assert!(
            !oriented
                .land
                .tiles
                .as_slice()
                .contains(&oriented.reserved_water)
        );
        let land = oriented.land.tiles.as_slice().iter().copied().collect();
        let water = BTreeSet::from([oriented.reserved_water]);
        assert!(
            validate_hut_placement(
                &oriented,
                &land,
                &water,
                &land,
                &BTreeSet::from([oriented.dock_land])
            )
            .is_err()
        );
    }
}

#[test]
fn catch_is_finite_quality_lotted_cargo_at_real_shore_and_wears_exact_rod() {
    let mut ecology = ecology();
    let mut ledger = QualityLotLedger::new(Vec::new(), Vec::new()).unwrap();
    let mut authority = FishingAuthority::default();
    let mut exact_rod = rod(QualityBand::Common);
    let outcome = authority
        .fish(
            &mut ecology,
            &mut ledger,
            Some(&mut exact_rod),
            attempt("fish_a", false),
        )
        .unwrap();
    assert_eq!(outcome.mode, FishingMode::RodOnly);
    assert_eq!(outcome.shoreline_task.anchor, TilePoint { x: 1, y: 0 });
    assert_eq!(exact_rod.durability, 1);
    if outcome.succeeded {
        let lot = ledger.lot(outcome.caught_lot_id.as_ref().unwrap()).unwrap();
        assert_eq!(lot.key.content_id.as_str(), "food_raw_fish");
        assert_eq!(lot.location, LotLocation::Cargo("cargo_a".into()));
        assert_eq!(lot.provenance.origin, "founding_fish_source");
        assert_eq!(outcome.caught_units, 15);
    }
    assert!(ecology.fish_habitat().stock <= 24);
}

#[test]
fn failed_accepted_attempt_wears_rod_advances_index_and_debits_no_fish() {
    let (outcome, ecology, exact_rod, authority) = (0..10_000)
        .find_map(|seed| {
            let mut ecology = ecology();
            let mut ledger = QualityLotLedger::new(Vec::new(), Vec::new()).unwrap();
            let mut authority = FishingAuthority::default();
            let mut exact_rod = rod(QualityBand::Crude);
            let mut request = attempt("failed_fishing", false);
            request.world_seed = seed;
            let outcome = authority
                .fish(&mut ecology, &mut ledger, Some(&mut exact_rod), request)
                .ok()?;
            (!outcome.succeeded).then_some((outcome, ecology, exact_rod, authority))
        })
        .expect("the 87 percent Rod profile has deterministic failing seeds");

    assert_eq!(outcome.caught_units, 0);
    assert!(outcome.caught_lot_id.is_none());
    assert_eq!(ecology.fish_habitat().stock, 24);
    assert_eq!(exact_rod.durability, 1);
    assert_eq!(authority.next_attempt_index(), 1);
}

#[test]
fn attempt_index_replay_and_restart_are_idempotent() {
    let mut ecology = ecology();
    let mut ledger = QualityLotLedger::new(Vec::new(), Vec::new()).unwrap();
    let mut authority = FishingAuthority::default();
    let mut exact_rod = rod(QualityBand::Common);
    let request = attempt("replay_fishing", false);
    let first = authority
        .fish(
            &mut ecology,
            &mut ledger,
            Some(&mut exact_rod),
            request.clone(),
        )
        .unwrap();
    let durability_after_first = exact_rod.durability;
    let stock_after_first = ecology.fish_habitat().stock;
    let ledger_after_first = ledger.clone();

    let replay = authority
        .fish(&mut ecology, &mut ledger, Some(&mut exact_rod), request)
        .unwrap();
    assert_eq!(replay, first);
    assert_eq!(exact_rod.durability, durability_after_first);
    assert_eq!(ecology.fish_habitat().stock, stock_after_first);
    assert_eq!(ledger, ledger_after_first);

    let mut stale = attempt("stale_index", false);
    stale.attempt_index = 0;
    assert!(matches!(
        authority.fish(&mut ecology, &mut ledger, Some(&mut exact_rod), stale),
        Err(FishingError::AttemptIndex {
            expected: 1,
            actual: 0
        })
    ));

    let bytes = serde_json::to_vec(&authority).unwrap();
    let restored: FishingAuthority = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(restored.next_attempt_index(), 1);
    assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
}

#[test]
fn empty_habitat_never_fabricates_but_an_accepted_attempt_still_wears_the_rod() {
    let mut ecology = ecology();
    let catch_index = ecology.next_catch_index();
    ecology
        .catch_fish_units(
            HandFishingRequest {
                task: FishTask {
                    task_tile: Tile { x: 1, y: 0 },
                },
                source_quality: QualityBand::Common,
                worker_skill: 0,
                tool_quality: None,
                fixture_quality: None,
                world_seed: 7,
                catch_index,
                now_tick: 0,
            },
            24,
        )
        .unwrap();
    let mut ledger = QualityLotLedger::new(Vec::new(), Vec::new()).unwrap();
    let mut authority = FishingAuthority::default();
    let mut exact_rod = rod(QualityBand::Common);
    let outcome = authority
        .fish(
            &mut ecology,
            &mut ledger,
            Some(&mut exact_rod),
            attempt("empty_habitat", true),
        )
        .unwrap();

    assert_eq!(outcome.profile.reliability_percent, 100);
    assert!(!outcome.succeeded);
    assert_eq!(outcome.caught_units, 0);
    assert!(outcome.caught_lot_id.is_none());
    assert_eq!(ecology.fish_habitat().stock, 0);
    assert_eq!(ledger.total_bulk_quantity(), 0);
    assert_eq!(exact_rod.durability, 1);
    assert_eq!(authority.next_attempt_index(), 1);
}

#[test]
fn caught_cargo_recovers_by_identity_and_malformed_receipts_fail_closed() {
    let mut ecology = ecology();
    let mut ledger = QualityLotLedger::new(Vec::new(), Vec::new()).unwrap();
    let mut authority = FishingAuthority::default();
    let mut exact_rod = rod(QualityBand::Common);
    let outcome = authority
        .fish(
            &mut ecology,
            &mut ledger,
            Some(&mut exact_rod),
            attempt("recover_fishing", true),
        )
        .unwrap();
    assert!(outcome.succeeded);
    let lot_id = outcome.caught_lot_id.unwrap();
    authority
        .recover_cargo(
            &mut ledger,
            &lot_id,
            RecoveryReason::RouteLost,
            LotLocation::Cache("shore_cache".into()),
        )
        .unwrap();
    assert_eq!(
        ledger.lot(&lot_id).unwrap().location,
        LotLocation::Cache("shore_cache".into())
    );
    assert_eq!(ledger.lot(&lot_id).unwrap().reservation, None);

    let mut malformed = serde_json::to_value(&authority).unwrap();
    malformed["receipts"][0]["outcome"]["caughtUnits"] = serde_json::json!(0);
    assert!(serde_json::from_value::<FishingAuthority>(malformed).is_err());
}

#[test]
fn reports_are_shared_and_state_decode_is_strict() {
    let authority = FishingAuthority::default();
    let ecology = ecology();
    assert_eq!(
        authority.habitat_report(&ecology, ReportAudience::God, ReportLevel(3)),
        authority.habitat_report(&ecology, ReportAudience::Leader, ReportLevel(3))
    );
    assert_eq!(
        authority.habitat_report(&ecology, ReportAudience::God, ReportLevel(3)),
        EcologyReport::Hidden
    );
    let value = serde_json::to_value(&authority).unwrap();
    assert_eq!(value["schemaVersion"], FISHING_SCHEMA_VERSION);
    let mut future = value.clone();
    future["schemaVersion"] = serde_json::json!(FISHING_SCHEMA_VERSION + 1);
    assert!(serde_json::from_value::<FishingAuthority>(future).is_err());
    let mut unknown = value;
    unknown["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<FishingAuthority>(unknown).is_err());

    let oversized_receipts = serde_json::json!({
        "schemaVersion": FISHING_SCHEMA_VERSION,
        "nextAttemptIndex": MAX_FISHING_RECEIPTS + 1,
        "receipts": (0..=MAX_FISHING_RECEIPTS)
            .map(|_| serde_json::json!({}))
            .collect::<Vec<_>>()
    });
    assert!(serde_json::from_value::<FishingAuthority>(oversized_receipts).is_err());
}
