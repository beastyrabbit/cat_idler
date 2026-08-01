use cat_sim::{
    content_manifest::{ContentId, ContentManifest, CreatureId, CreatureTier, MaterialInstanceId},
    hunting_lair::{
        AttemptAuthority, AttemptGate, EquipmentLocation, EquippedItem, EquippedItemKind,
        GAME_MINUTES_PER_HOUR, GatheringQualityRequest, HuntAttemptRequest, HuntResolution,
        HuntingCatalog, HuntingError, HuntingLairState, HuntingSiteKind, SOLO_PARTY_CAP,
        StoragePlacement, TileCoord, attempt_gate, generate_roster, named_drop_percent,
        named_drop_roll_percent, party_cap, predicted_success_percent, rare_quality_range,
        recover_outputs, release_equipment_reservations, resolve_attempt, respawn_hours,
    },
    quality_lots::{LotLocation, QualityBand},
};

fn catalog() -> HuntingCatalog<'static> {
    HuntingCatalog::from_manifest(ContentManifest::embedded()).unwrap()
}

fn tile() -> TileCoord {
    TileCoord { x: 7, y: -3 }
}

fn lair_at(level: u8, seed: u32) -> HuntingLairState {
    HuntingLairState::new_enemy_lair(&catalog(), seed, "enemy_lair_alpha", tile(), level).unwrap()
}

fn item_id(value: &str) -> MaterialInstanceId {
    MaterialInstanceId::new(value).unwrap()
}

fn weapon(owner: &str, effect: u16, durability: u32) -> EquippedItem {
    EquippedItem {
        item_instance_id: item_id(&format!("{}_weapon", owner)),
        kind: EquippedItemKind::Weapon,
        resolved_effect: effect,
        durability,
        reserved: false,
        location: EquipmentLocation::Equipped(owner.to_owned()),
        usable: true,
    }
}

fn armor(owner: &str, effect: u16, durability: u32) -> EquippedItem {
    EquippedItem {
        item_instance_id: item_id(&format!("{}_armor", owner)),
        kind: EquippedItemKind::Armor,
        resolved_effect: effect,
        durability,
        reserved: false,
        location: EquipmentLocation::Equipped(owner.to_owned()),
        usable: true,
    }
}

fn hunter(id: &str, combat_power: u16, health_percent: u8) -> cat_sim::hunting_lair::HunterInput {
    cat_sim::hunting_lair::HunterInput {
        cat_id: id.to_owned(),
        combat_power,
        health_percent,
        weapon: None,
        armor: None,
    }
}

fn request(
    world_seed: u32,
    party: Vec<cat_sim::hunting_lair::HunterInput>,
    has_hunting_bulk: bool,
    capacity_units: u32,
) -> HuntAttemptRequest {
    HuntAttemptRequest {
        world_seed,
        now_game_minute: 1_000,
        has_hunting_bulk,
        party,
        quality: GatheringQualityRequest {
            source_quality: QualityBand::Common,
            lead_skill: 80,
            tool_quality: Some(QualityBand::Fine),
            fixture_quality: None,
        },
        storage: StoragePlacement {
            stockpile_id: "main_stockpile".to_owned(),
            capacity_units,
        },
    }
}

fn resolve_with_outcome(
    catalog: &HuntingCatalog<'_>,
    lair: &HuntingLairState,
    mut request: HuntAttemptRequest,
    cleared: bool,
) -> HuntResolution {
    for seed in 1..=10_000 {
        request.world_seed = seed;
        if let Ok(result) = resolve_attempt(catalog, lair, request.clone()) {
            if result.cleared == cleared {
                return result;
            }
        }
    }
    panic!("no deterministic hunt outcome found for cleared={cleared}");
}

#[test]
fn catalog_rejects_non_exact_hunting_manifest_authority() {
    let manifest = ContentManifest::embedded();
    assert_eq!(
        HuntingCatalog::from_manifest(manifest)
            .unwrap()
            .creatures()
            .len(),
        20
    );

    let mut missing = (*manifest).clone();
    missing.creatures.pop();
    assert_eq!(
        HuntingCatalog::from_manifest(&missing).unwrap_err(),
        HuntingError::CreatureCatalogMismatch
    );

    let mut bad_band = (*manifest).clone();
    bad_band.lair_bands[3].mystic_required_from_level = Some(60);
    assert_eq!(
        HuntingCatalog::from_manifest(&bad_band).unwrap_err(),
        HuntingError::LairBandMismatch
    );
}

#[test]
fn all_twenty_manifest_rows_have_exact_plan_common_and_named_loot_dominance() {
    let catalog = catalog();
    let expected = [
        ("cave_bat", 1, 0, 1, "bat_wing"),
        ("red_fox", 12, 2, 1, "fox_pelt"),
        ("badger", 18, 3, 2, "badger_pelt"),
        ("wild_boar", 24, 3, 4, "boar_tusk"),
        ("gray_wolf", 22, 3, 3, "wolf_pelt"),
        ("lynx", 20, 3, 3, "lynx_pelt"),
        ("great_stag", 35, 4, 5, "stag_antler"),
        ("giant_serpent", 18, 4, 2, "serpent_scale"),
        ("brown_bear", 30, 6, 4, "bear_pelt"),
        ("great_eagle", 16, 3, 1, "eagle_feather"),
        ("moon_stag", 40, 5, 5, "moon_antler"),
        ("warg", 35, 5, 5, "warg_fang"),
        ("cockatrice", 24, 5, 2, "cockatrice_eye"),
        ("forest_troll", 50, 10, 8, "troll_hide"),
        ("griffin", 45, 7, 6, "griffin_plume"),
        ("basilisk", 35, 8, 5, "basilisk_scale"),
        ("manticore", 55, 9, 8, "manticore_barb"),
        ("chimera", 70, 12, 10, "beast_core"),
        ("wyvern", 80, 14, 12, "wyvern_membrane"),
        ("elder_dragon", 120, 30, 20, "dragon_heart"),
    ];

    for (index, (id, meat, hide, bone, material)) in expected.iter().enumerate() {
        let creature = &catalog.creatures()[index];
        assert_eq!(creature.id.as_str(), *id);
        assert_eq!(creature.primary_material.as_str(), *material);
        let quantity = |content_id: &str| {
            creature
                .common_loot
                .iter()
                .find(|loot| loot.content_id.as_str() == content_id)
                .map_or(0, |loot| loot.units)
        };
        assert_eq!(quantity("food_raw_meat"), *meat);
        assert_eq!(quantity("resource_hide"), *hide);
        assert_eq!(quantity("resource_bone"), *bone);
        assert_eq!(quantity("food"), 0);
    }

    assert!(
        catalog.creatures()[19].common_loot[0].units > catalog.creatures()[0].common_loot[0].units
    );
}

#[test]
fn boundary_levels_apply_party_tier_and_respawn_rules() {
    let catalog = catalog();
    for (level, min_size, max_size, respawn) in [
        (1, 1, 1, 6),
        (19, 1, 1, 6),
        (20, 1, 2, 8),
        (39, 1, 2, 8),
        (40, 2, 2, 12),
        (59, 2, 2, 12),
        (60, 2, 3, 14),
        (61, 2, 3, 14),
        (79, 2, 3, 14),
        (80, 3, 3, 18),
        (94, 3, 3, 18),
        (95, 3, 3, 24),
        (100, 3, 3, 24),
    ] {
        let roster = generate_roster(&catalog, 42, "enemy_lair_alpha", 3, level).unwrap();
        assert!(roster.len() >= min_size);
        assert!(roster.len() <= max_size);
        assert_eq!(respawn_hours(level), respawn);
        if level <= 39 {
            assert!(roster.iter().all(|entry| {
                catalog.creature(&entry.creature_id).unwrap().tier == CreatureTier::Normal
            }));
        }
        if level >= 61 && level < 95 {
            assert!(roster.iter().any(|entry| {
                catalog.creature(&entry.creature_id).unwrap().tier == CreatureTier::Mystic
            }));
        }
        if level >= 95 {
            assert_eq!(roster[0].creature_id.as_str(), "elder_dragon");
            assert_eq!(
                roster
                    .iter()
                    .filter(|entry| entry.creature_id.as_str() == "elder_dragon")
                    .count(),
                1
            );
            assert_eq!(roster.len(), 3);
        }
    }

    let level_60 = generate_roster(&catalog, 3, "enemy_lair_alpha", 1, 60).unwrap();
    assert!(!level_60.is_empty());
}

#[test]
fn species_unlock_by_minimum_and_actual_level_clamps_to_normative_range() {
    let catalog = catalog();
    for level in 1..=100 {
        let roster = generate_roster(&catalog, 99, "enemy_lair_alpha", 2, level).unwrap();
        for entry in roster {
            let creature = catalog.creature(&entry.creature_id).unwrap();
            assert!(creature.level_min <= level);
            assert_eq!(
                entry.actual_level,
                level.clamp(creature.level_min, creature.level_max)
            );
        }
    }
}

#[test]
fn roster_generation_is_keyed_by_seed_lair_generation_and_level() {
    let catalog = catalog();
    let first = generate_roster(&catalog, 42, "enemy_lair_alpha", 7, 80).unwrap();
    let replay = generate_roster(&catalog, 42, "enemy_lair_alpha", 7, 80).unwrap();
    let different_site = generate_roster(&catalog, 42, "enemy_lair_beta", 7, 80).unwrap();
    let different_generation = generate_roster(&catalog, 42, "enemy_lair_alpha", 8, 80).unwrap();
    let different_level = generate_roster(&catalog, 42, "enemy_lair_alpha", 7, 79).unwrap();

    assert_eq!(first, replay);
    assert_ne!(first, different_site);
    assert_ne!(first, different_generation);
    assert_ne!(first, different_level);
}

#[test]
fn authority_gates_and_party_cap_preserve_review_only_nudges() {
    let party = vec![hunter("a", 50, 80)];
    assert_eq!(
        attempt_gate(AttemptAuthority::AutonomousLeader, 70, &party).unwrap(),
        AttemptGate::CombatAuthorized
    );
    assert_eq!(
        attempt_gate(AttemptAuthority::AutonomousLeader, 69, &party).unwrap(),
        AttemptGate::Denied
    );
    assert_eq!(
        attempt_gate(AttemptAuthority::PlayerNudge, 45, &party).unwrap(),
        AttemptGate::ReviewAuthorized
    );
    assert_eq!(
        attempt_gate(AttemptAuthority::PlayerNudge, 44, &party).unwrap(),
        AttemptGate::Denied
    );
    assert_eq!(
        attempt_gate(AttemptAuthority::PlayerNudge, 45, &[hunter("a", 50, 79)]).unwrap(),
        AttemptGate::Denied
    );
    assert_eq!(party_cap(false), SOLO_PARTY_CAP);
    assert_eq!(party_cap(true), 3);
}

#[test]
fn success_math_uses_exact_eligible_equipment_and_wear_intents_do_not_underflow() {
    let catalog = catalog();
    let lair = HuntingLairState::from_parts(
        "enemy_lair_alpha",
        HuntingSiteKind::EnemyLair,
        tile(),
        1,
        0,
        0,
        vec![cat_sim::hunting_lair::RosterEntry {
            slot: 0,
            creature_id: CreatureId::new("cave_bat").unwrap(),
            actual_level: 1,
            health: 2,
        }],
        false,
        None,
        vec![],
        vec![],
    )
    .unwrap();
    let mut equipped = hunter("a", 1, 100);
    equipped.weapon = Some(weapon("a", 20, 1));
    equipped.armor = Some(armor("a", 9, 1));
    assert_eq!(
        predicted_success_percent(&catalog, &lair, &[equipped.clone()]).unwrap(),
        73
    );

    let mut wrong_location = equipped.clone();
    wrong_location.weapon.as_mut().unwrap().location = EquipmentLocation::Stockpile("s".to_owned());
    wrong_location.armor.as_mut().unwrap().reserved = true;
    assert_eq!(
        predicted_success_percent(&catalog, &lair, &[wrong_location.clone()]).unwrap(),
        49
    );

    let result = resolve_attempt(&catalog, &lair, request(1, vec![equipped], false, 100)).unwrap();
    assert_eq!(result.wear_intents.len(), 2);
    assert!(
        result
            .wear_intents
            .iter()
            .all(|intent| intent.to_durability == 0)
    );

    let result = resolve_attempt(
        &catalog,
        &lair,
        request(1, vec![wrong_location], false, 100),
    )
    .unwrap();
    assert!(result.wear_intents.is_empty());
}

#[test]
fn failure_awards_failure_xp_damage_death_once_and_creates_no_loot() {
    let catalog = catalog();
    let lair = HuntingLairState::from_parts(
        "enemy_lair_alpha",
        HuntingSiteKind::EnemyLair,
        tile(),
        95,
        0,
        0,
        vec![cat_sim::hunting_lair::RosterEntry {
            slot: 0,
            creature_id: CreatureId::new("elder_dragon").unwrap(),
            actual_level: 95,
            health: 100,
        }],
        false,
        None,
        vec![],
        vec![],
    )
    .unwrap();
    let result = resolve_with_outcome(
        &catalog,
        &lair,
        request(1, vec![hunter("fragile", 0, 10)], false, 100),
        false,
    );

    assert!(!result.cleared);
    assert_eq!(result.lair, lair);
    assert!(result.outputs.common_lots.is_empty());
    assert!(result.outputs.named_drops.is_empty());
    assert_eq!(result.participants[0].hunting_xp, 3);
    assert_eq!(result.participants[0].fight_xp, 3);
    assert!(result.participants[0].damage >= 10);
    assert!(result.participants[0].died);
}

#[test]
fn failure_damage_rounds_only_after_the_fractional_party_average() {
    let catalog = catalog();
    let lair = HuntingLairState::from_parts(
        "enemy_lair_fractional_damage",
        HuntingSiteKind::EnemyLair,
        tile(),
        1,
        0,
        0,
        vec![cat_sim::hunting_lair::RosterEntry {
            slot: 0,
            creature_id: CreatureId::new("cave_bat").unwrap(),
            actual_level: 1,
            health: 2,
        }],
        false,
        None,
        vec![],
        vec![],
    )
    .unwrap();
    let result = resolve_with_outcome(
        &catalog,
        &lair,
        request(
            1,
            vec![hunter("power_zero", 0, 100), hunter("power_one", 1, 100)],
            true,
            100,
        ),
        false,
    );

    assert!(
        result
            .participants
            .iter()
            .all(|participant| participant.damage == 22)
    );
}

#[test]
fn victory_creates_quality_lots_named_instances_cache_and_respawn_deadline() {
    let catalog = catalog();
    let lair = HuntingLairState::from_parts(
        "enemy_lair_alpha",
        HuntingSiteKind::EnemyLair,
        tile(),
        95,
        0,
        0,
        vec![
            cat_sim::hunting_lair::RosterEntry {
                slot: 0,
                creature_id: CreatureId::new("elder_dragon").unwrap(),
                actual_level: 95,
                health: 100,
            },
            cat_sim::hunting_lair::RosterEntry {
                slot: 1,
                creature_id: CreatureId::new("wyvern").unwrap(),
                actual_level: 95,
                health: 92,
            },
        ],
        false,
        None,
        vec![],
        vec![],
    )
    .unwrap();
    let mut a = hunter("a", 500, 100);
    a.weapon = Some(weapon("a", 10, 3));
    let mut b = hunter("b", 500, 100);
    b.armor = Some(armor("b", 10, 3));
    let result = resolve_with_outcome(&catalog, &lair, request(123, vec![b, a], true, 2), true);

    assert!(result.cleared);
    assert!(result.lair.roster.is_empty());
    assert_eq!(result.lair.clear_index, 1);
    assert!(result.lair.first_clear_claimed);
    assert_eq!(
        result.lair.respawn_ready_game_minute,
        Some(1_000 + 24 * GAME_MINUTES_PER_HOUR)
    );
    assert_eq!(result.participants[0].cat_id, "a");
    assert!(
        result
            .participants
            .iter()
            .all(|entry| entry.hunting_xp == 13 && entry.fight_xp == 10)
    );
    assert_eq!(
        result
            .outputs
            .common_lots
            .iter()
            .filter(|lot| lot.key.content_id.as_str() == "food_raw_meat")
            .map(|lot| lot.quantity)
            .sum::<u32>(),
        200
    );
    assert!(
        result
            .outputs
            .common_lots
            .iter()
            .any(|lot| matches!(lot.location, LotLocation::Cache(_)))
    );
    assert!(!result.lair.cache_lot_ids.is_empty());
    assert!(
        result
            .outputs
            .named_drops
            .iter()
            .all(|drop| drop.quality == QualityBand::Masterwork)
    );
}

#[test]
fn drop_chances_quality_ranges_and_first_clear_floor_are_exact() {
    assert_eq!(named_drop_percent(1), 10);
    assert_eq!(named_drop_percent(24), 10);
    assert_eq!(named_drop_percent(25), 15);
    assert_eq!(named_drop_percent(49), 15);
    assert_eq!(named_drop_percent(50), 20);
    assert_eq!(named_drop_percent(69), 20);
    assert_eq!(named_drop_percent(70), 25);
    assert_eq!(named_drop_percent(84), 25);
    assert_eq!(named_drop_percent(85), 30);
    assert_eq!(named_drop_percent(94), 30);
    assert_eq!(named_drop_percent(95), 40);
    assert_eq!(named_drop_percent(100), 40);
    let cave_bat = CreatureId::new("cave_bat").unwrap();
    assert_eq!(
        named_drop_roll_percent(7, "enemy_lair_4_9", 3, &cave_bat, 2),
        66,
        "the exact roll key is replay-stable without time, level, slot, or nonce"
    );

    assert_eq!(
        rare_quality_range(1),
        (QualityBand::Crude, QualityBand::Crude)
    );
    assert_eq!(
        rare_quality_range(25),
        (QualityBand::Crude, QualityBand::Common)
    );
    assert_eq!(
        rare_quality_range(50),
        (QualityBand::Common, QualityBand::Fine)
    );
    assert_eq!(
        rare_quality_range(70),
        (QualityBand::Fine, QualityBand::Superior)
    );
    assert_eq!(
        rare_quality_range(85),
        (QualityBand::Superior, QualityBand::Masterwork)
    );
    assert_eq!(
        rare_quality_range(95),
        (QualityBand::Masterwork, QualityBand::Masterwork)
    );

    let catalog = catalog();
    let lair = HuntingLairState::from_parts(
        "enemy_lair_alpha",
        HuntingSiteKind::EnemyLair,
        tile(),
        1,
        0,
        0,
        vec![cat_sim::hunting_lair::RosterEntry {
            slot: 0,
            creature_id: CreatureId::new("cave_bat").unwrap(),
            actual_level: 1,
            health: 2,
        }],
        false,
        None,
        vec![],
        vec![],
    )
    .unwrap();
    let result = (1..=10_000)
        .find_map(|seed| {
            let result = resolve_attempt(
                &catalog,
                &lair,
                request(seed, vec![hunter("a", 500, 100)], false, 100),
            )
            .ok()?;
            (result.cleared
                && result.outputs.named_drops.len() == 1
                && result.outputs.named_drops[0].guaranteed_first_clear)
                .then_some(result)
        })
        .expect("first clear seed with guaranteed drop");
    assert_eq!(result.outputs.named_drops.len(), 1);
    assert!(result.outputs.named_drops[0].guaranteed_first_clear);
    assert_eq!(result.outputs.named_drops[0].quality, QualityBand::Crude);

    let mut later = lair.clone();
    later.first_clear_claimed = true;
    let later_result = (1..=10_000)
        .find_map(|seed| {
            let result = resolve_attempt(
                &catalog,
                &later,
                request(seed, vec![hunter("a", 500, 100)], false, 100),
            )
            .ok()?;
            (result.cleared && result.outputs.named_drops.is_empty()).then_some(result)
        })
        .expect("later clear seed without ordinary named drop");
    assert!(later_result.outputs.named_drops.is_empty());
}

#[test]
fn respawn_uses_absolute_deadline_and_is_restart_equivalent() {
    let catalog = catalog();
    let lair = lair_at(80, 11);
    let result = resolve_with_outcome(
        &catalog,
        &lair,
        request(
            99,
            vec![
                hunter("a", 500, 100),
                hunter("b", 500, 100),
                hunter("c", 500, 100),
            ],
            true,
            500,
        ),
        true,
    );
    let ready_at = 1_000 + 18 * GAME_MINUTES_PER_HOUR;

    assert_eq!(result.lair.respawn_ready_game_minute, Some(ready_at));
    assert_eq!(
        result
            .lair
            .respawn_if_ready(&catalog, 99, ready_at - 1)
            .unwrap(),
        None
    );
    let respawned = result
        .lair
        .respawn_if_ready(&catalog, 99, ready_at)
        .unwrap()
        .unwrap();
    let replay = result
        .lair
        .respawn_if_ready(&catalog, 99, ready_at)
        .unwrap()
        .unwrap();
    assert_eq!(respawned, replay);
    assert_eq!(respawned.generation, lair.generation + 1);
    assert!(!respawned.roster.is_empty());
}

#[test]
fn strict_serde_rejects_future_unknown_invalid_and_cave_entrance_sites() {
    let state = lair_at(20, 1);
    let json = serde_json::to_string(&state).unwrap();
    let decoded: HuntingLairState = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, state);

    let future = json.replace("\"schemaVersion\":1", "\"schemaVersion\":2");
    assert!(serde_json::from_str::<HuntingLairState>(&future).is_err());

    let unknown = json.replace("\"siteId\"", "\"extra\":1,\"siteId\"");
    assert!(serde_json::from_str::<HuntingLairState>(&unknown).is_err());

    let known_creature_state = HuntingLairState::from_parts(
        "enemy_lair_alpha",
        HuntingSiteKind::EnemyLair,
        tile(),
        1,
        0,
        0,
        vec![cat_sim::hunting_lair::RosterEntry {
            slot: 0,
            creature_id: CreatureId::new("cave_bat").unwrap(),
            actual_level: 1,
            health: 2,
        }],
        false,
        None,
        vec![],
        vec![],
    )
    .unwrap();
    let invalid_id_json = serde_json::to_string(&known_creature_state)
        .unwrap()
        .replace("cave_bat", "Bad Id!");
    assert!(serde_json::from_str::<HuntingLairState>(&invalid_id_json).is_err());
    let unknown_but_well_formed_id_json = serde_json::to_string(&known_creature_state)
        .unwrap()
        .replace("cave_bat", "unknown_creature");
    assert!(serde_json::from_str::<HuntingLairState>(&unknown_but_well_formed_id_json).is_err());

    assert_eq!(
        HuntingLairState::from_parts(
            "quarry_cave",
            HuntingSiteKind::CaveEntrance,
            tile(),
            20,
            0,
            0,
            vec![],
            false,
            None,
            vec![],
            vec![],
        )
        .unwrap_err(),
        HuntingError::InvalidSiteKind(HuntingSiteKind::CaveEntrance)
    );
}

#[test]
fn recovery_and_release_helpers_preserve_identity_and_clear_reservations() {
    let catalog = catalog();
    let lair = HuntingLairState::from_parts(
        "enemy_lair_alpha",
        HuntingSiteKind::EnemyLair,
        tile(),
        1,
        0,
        0,
        vec![cat_sim::hunting_lair::RosterEntry {
            slot: 0,
            creature_id: CreatureId::new("cave_bat").unwrap(),
            actual_level: 1,
            health: 2,
        }],
        false,
        None,
        vec![],
        vec![],
    )
    .unwrap();
    let mut h = hunter("a", 500, 100);
    h.weapon = Some(EquippedItem {
        reserved: true,
        ..weapon("a", 1, 1)
    });
    let result = resolve_with_outcome(&catalog, &lair, request(1, vec![h.clone()], false, 0), true);
    let lot_ids = result
        .outputs
        .common_lots
        .iter()
        .map(|lot| lot.id.clone())
        .collect::<Vec<_>>();
    let recovered = recover_outputs(result.outputs, LotLocation::Stockpile("safe".to_owned()));
    assert_eq!(
        recovered
            .common_lots
            .iter()
            .map(|lot| lot.id.clone())
            .collect::<Vec<_>>(),
        lot_ids
    );
    assert!(
        recovered
            .common_lots
            .iter()
            .all(|lot| lot.reservation.is_none())
    );
    assert_eq!(
        release_equipment_reservations(&[h]),
        vec![item_id("a_weapon")]
    );
}

#[test]
fn party_validation_rejects_duplicates_and_over_cap_without_silent_drop() {
    let catalog = catalog();
    let lair = lair_at(20, 1);
    assert_eq!(
        resolve_attempt(&catalog, &lair, request(1, vec![], false, 10)).unwrap_err(),
        HuntingError::EmptyParty
    );
    assert_eq!(
        resolve_attempt(
            &catalog,
            &lair,
            request(
                1,
                vec![hunter("a", 100, 100), hunter("a", 100, 100)],
                true,
                10
            ),
        )
        .unwrap_err(),
        HuntingError::DuplicateCatId("a".to_owned())
    );
    assert_eq!(
        resolve_attempt(
            &catalog,
            &lair,
            request(
                1,
                vec![hunter("a", 100, 100), hunter("b", 100, 100)],
                false,
                10
            ),
        )
        .unwrap_err(),
        HuntingError::PartyTooLarge {
            supplied: 2,
            cap: 1
        }
    );
}

#[test]
fn common_loot_outputs_are_lai37_content_quality_lots_not_generic_food() {
    let catalog = catalog();
    let lair = HuntingLairState::from_parts(
        "enemy_lair_alpha",
        HuntingSiteKind::EnemyLair,
        tile(),
        1,
        0,
        0,
        vec![cat_sim::hunting_lair::RosterEntry {
            slot: 0,
            creature_id: CreatureId::new("cave_bat").unwrap(),
            actual_level: 1,
            health: 2,
        }],
        false,
        None,
        vec![],
        vec![],
    )
    .unwrap();
    let result = resolve_with_outcome(
        &catalog,
        &lair,
        request(1, vec![hunter("a", 500, 100)], false, 100),
        true,
    );
    let ids = result
        .outputs
        .common_lots
        .iter()
        .map(|lot| lot.key.content_id.clone())
        .collect::<Vec<ContentId>>();
    assert!(ids.iter().any(|id| id.as_str() == "food_raw_meat"));
    assert!(ids.iter().any(|id| id.as_str() == "resource_bone"));
    assert!(ids.iter().all(|id| id.as_str() != "food"));
    assert!(
        result
            .outputs
            .common_lots
            .iter()
            .all(|lot| QualityBand::ALL.contains(&lot.key.quality))
    );
}
