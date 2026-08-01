use cat_protocol::ClientAction;
use cat_sim::{
    actions::{ActionCtx, apply_action, build_snapshot},
    biomes::MaxResources,
    black_hole::{BlackHoleAxes, BlackHoleRuntime},
    hunting_lair::{AttemptAuthority, HuntAdvice, HuntingLair, MonsterSpecies, SpeciesMaterial},
    hunting_runtime::{
        HuntingAttemptError, attempt_hunting_lair, captain_recommendation, reconcile_hunting_lairs,
        tick_hunting_lair_adapter,
    },
    items::{Item, ItemKind, ItemLocation, Material},
    types::TileType,
    world_gen::TileResources,
    world_tick::{TilePos, WorldTileRuntime, found_colony, new_world},
};

fn mapped_world() -> (cat_sim::world_tick::WorldState, TilePos, TilePos) {
    let mut world = new_world(42);
    world.colonies.push(found_colony(42, "colony-1", 0, 7));
    let lair = TilePos { x: 30, y: 12 };
    let quarry = TilePos { x: 31, y: 12 };
    for (position, tile_type, danger_level) in [
        (lair, TileType::EnemyLair, 95.0),
        (quarry, TileType::CaveEntrance, 98.0),
    ] {
        world.shared_spatial.tiles.insert(
            position,
            WorldTileRuntime {
                pos: position,
                tile_type,
                resources: TileResources {
                    food: 0,
                    herbs: 0,
                    water: 0,
                    gem: 0,
                    clay: 0,
                    sand: 0,
                },
                max_resources: MaxResources { food: 0, herbs: 0 },
                danger_level,
                path_wear: 0,
                last_depleted: 0,
                overlay_feature: None,
            },
        );
        world.colonies[0]
            .world_tiles
            .insert(position, world.shared_spatial.tiles[&position].clone());
        world.colonies[0].revealed_tiles.insert(position);
    }
    (world, lair, quarry)
}

fn equip_hunter(world: &mut cat_sim::world_tick::WorldState, cat_id: &str) -> (String, String) {
    let colony = &mut world.colonies[0];
    let weapon_id = colony
        .items
        .add_at(
            Item::new(ItemKind::Weapon, Material::Metal, 1),
            1,
            1.0,
            ItemLocation::Equipped {
                cat_id: cat_id.to_owned(),
            },
            true,
        )
        .pop()
        .expect("weapon id");
    let armor_id = colony
        .items
        .add_at(
            Item::new(ItemKind::Armor, Material::Metal, 1),
            1,
            1.0,
            ItemLocation::Equipped {
                cat_id: cat_id.to_owned(),
            },
            true,
        )
        .pop()
        .expect("armor id");
    (weapon_id, armor_id)
}

#[test]
fn enemy_lairs_get_persisted_rosters_but_quarry_caves_remain_distinct() {
    let (mut world, lair, quarry) = mapped_world();
    reconcile_hunting_lairs(&mut world.shared_spatial, world.world_seed, 0);

    assert_eq!(world.shared_spatial.hunting_lairs[&lair].monsters.len(), 3);
    assert!(!world.shared_spatial.hunting_lairs.contains_key(&quarry));
}

#[test]
fn captain_advises_while_leader_or_player_authority_controls_dispatch() {
    let (mut world, lair, _) = mapped_world();
    reconcile_hunting_lairs(&mut world.shared_spatial, world.world_seed, 0);
    let cat_id = world.colonies[0].cats[0].id.clone();
    let cat = &mut world.colonies[0].cats[0];
    cat.stats.attack = 100.0;
    cat.stats.defense = 100.0;
    cat.stats.hunting = 100.0;
    cat.needs.health = 100.0;
    equip_hunter(&mut world, &cat_id);

    let recommendation =
        captain_recommendation(&world, "colony-1", lair, std::slice::from_ref(&cat_id)).unwrap();
    assert!(matches!(
        recommendation.advice,
        HuntAdvice::Favored | HuntAdvice::Safe
    ));

    let report = attempt_hunting_lair(
        &mut world,
        "colony-1",
        lair,
        &[cat_id],
        AttemptAuthority::AutonomousLeader,
        0,
    )
    .expect("strong healthy hunter is safe for Leader dispatch");
    assert!(report.resolution.cleared);
    assert!(report.resolution.loot.food >= 12);
    assert!(world.shared_spatial.hunting_trophies.contains_key(&lair));
}

#[test]
fn captain_uses_exact_equipped_gear_and_attempts_wear_the_same_items() {
    let (mut world, lair, _) = mapped_world();
    world.shared_spatial.hunting_lairs.insert(
        lair,
        HuntingLair::from_species(100, [MonsterSpecies::RivalBeast]),
    );
    let cat_id = world.colonies[0].cats[0].id.clone();
    let cat = &mut world.colonies[0].cats[0];
    cat.stats.attack = 50.0;
    cat.stats.defense = 50.0;
    cat.stats.hunting = 50.0;
    cat.needs.health = 100.0;

    let without_gear =
        captain_recommendation(&world, "colony-1", lair, std::slice::from_ref(&cat_id))
            .expect("recommendation without gear");
    world.colonies[0].resources.weapons = 99.0;
    world.colonies[0].resources.armor = 99.0;
    let with_fake_aggregate =
        captain_recommendation(&world, "colony-1", lair, std::slice::from_ref(&cat_id))
            .expect("aggregate resources do not become equipped gear");
    assert_eq!(
        with_fake_aggregate.predicted_success_percent,
        without_gear.predicted_success_percent
    );

    let weapon_id = world.colonies[0]
        .items
        .add_at(
            Item::new(ItemKind::Weapon, Material::Metal, 1),
            1,
            1.0,
            ItemLocation::Equipped {
                cat_id: cat_id.clone(),
            },
            true,
        )
        .pop()
        .expect("weapon id");
    let armor_id = world.colonies[0]
        .items
        .add_at(
            Item::new(ItemKind::Armor, Material::Leather, 1),
            1,
            1.0,
            ItemLocation::Equipped {
                cat_id: cat_id.clone(),
            },
            true,
        )
        .pop()
        .expect("armor id");
    let with_exact_gear =
        captain_recommendation(&world, "colony-1", lair, std::slice::from_ref(&cat_id))
            .expect("recommendation with exact gear");
    assert!(
        with_exact_gear.predicted_success_percent > with_fake_aggregate.predicted_success_percent
    );
    let weapon_before = world.colonies[0]
        .items
        .instance(&weapon_id)
        .unwrap()
        .durability;
    let armor_before = world.colonies[0]
        .items
        .instance(&armor_id)
        .unwrap()
        .durability;

    attempt_hunting_lair(
        &mut world,
        "colony-1",
        lair,
        &[cat_id],
        AttemptAuthority::PlayerNudge,
        12_000,
    )
    .expect("exact gear makes the healthy party eligible");

    assert!(
        world.colonies[0]
            .items
            .instance(&weapon_id)
            .unwrap()
            .durability
            < weapon_before
    );
    assert!(
        world.colonies[0]
            .items
            .instance(&armor_id)
            .unwrap()
            .durability
            < armor_before
    );
}

#[test]
fn successful_hunt_credits_finite_physical_loot_and_typed_species_materials() {
    let (mut world, lair, _) = mapped_world();
    reconcile_hunting_lairs(&mut world.shared_spatial, world.world_seed, 0);
    let cat_id = world.colonies[0].cats[0].id.clone();
    let cat = &mut world.colonies[0].cats[0];
    cat.stats.attack = 100.0;
    cat.stats.defense = 100.0;
    cat.stats.hunting = 100.0;
    cat.needs.health = 100.0;
    equip_hunter(&mut world, &cat_id);
    let before = (
        world.colonies[0].resources.food,
        world.colonies[0].resources.hide,
        world.colonies[0].resources.bone,
    );

    let report = attempt_hunting_lair(
        &mut world,
        "colony-1",
        lair,
        &[cat_id],
        AttemptAuthority::AutonomousLeader,
        24_000,
    )
    .expect("strong hunter clears the lair");
    assert!(report.resolution.cleared);
    let colony = &world.colonies[0];
    assert_eq!(
        colony.resources.food - before.0,
        f64::from(report.resolution.loot.food)
    );
    assert_eq!(
        colony.resources.hide - before.1,
        f64::from(report.resolution.loot.hide)
    );
    assert_eq!(
        colony.resources.bone - before.2,
        f64::from(report.resolution.loot.bone)
    );
    for (resource, expected) in [
        (
            cat_sim::stockpiles::ResourceKind::Food,
            colony.resources.food,
        ),
        (
            cat_sim::stockpiles::ResourceKind::Hide,
            colony.resources.hide,
        ),
        (
            cat_sim::stockpiles::ResourceKind::Bone,
            colony.resources.bone,
        ),
    ] {
        let physical = colony
            .stockpiles
            .iter()
            .map(|pile| cat_sim::stockpiles::resource_amount(&pile.contents, resource))
            .sum::<f64>();
        assert_eq!(
            physical, expected,
            "{resource:?} aggregate must remain physical"
        );
    }
    assert!(
        world.shared_spatial.hunting_materials["colony-1"]
            .values()
            .copied()
            .sum::<u32>()
            >= 1,
        "first clear guarantees one typed species material"
    );
}

#[test]
fn respawn_deadline_uses_the_attackers_game_clock_and_matches_the_snapshot() {
    let (mut world, lair, _) = mapped_world();
    world
        .shared_spatial
        .hunting_lairs
        .insert(lair, HuntingLair::from_species(90, [MonsterSpecies::Bear]));
    world.colonies[0].test_time_scale = 60.0;
    let cat_id = world.colonies[0].cats[0].id.clone();
    let cat = &mut world.colonies[0].cats[0];
    cat.stats.attack = 100.0;
    cat.stats.defense = 100.0;
    cat.stats.hunting = 100.0;
    cat.needs.health = 100.0;
    equip_hunter(&mut world, &cat_id);
    let now_ms = 1_000;

    let report = attempt_hunting_lair(
        &mut world,
        "colony-1",
        lair,
        &[cat_id],
        AttemptAuthority::AutonomousLeader,
        now_ms,
    )
    .expect("strong hunter clears the lair");
    let expected_deadline = now_ms + 12 * 60_000;
    assert_eq!(
        report.resolution.lair.respawn_ready_at_ms,
        Some(expected_deadline)
    );
    let snapshot = build_snapshot(&world, now_ms, 1);
    let public_deadline = snapshot.colonies[0]
        .hunting_lair
        .as_ref()
        .unwrap()
        .revealed_sites[0]
        .monsters[0]
        .respawn
        .as_ref()
        .unwrap()
        .respawns_at_ms;
    assert_eq!(public_deadline, expected_deadline);

    reconcile_hunting_lairs(
        &mut world.shared_spatial,
        world.world_seed,
        expected_deadline - 1,
    );
    assert_eq!(
        world.shared_spatial.hunting_lairs[&lair].current_danger(),
        0
    );
    reconcile_hunting_lairs(
        &mut world.shared_spatial,
        world.world_seed,
        expected_deadline,
    );
    assert!(world.shared_spatial.hunting_lairs[&lair].current_danger() > 0);
    assert_eq!(
        world.shared_spatial.hunting_lairs[&lair].respawn_ready_at_ms,
        None
    );
}

#[test]
fn hunting_parties_require_the_existing_hunting_bulk_research() {
    let (mut world, lair, _) = mapped_world();
    reconcile_hunting_lairs(&mut world.shared_spatial, world.world_seed, 0);
    let party = world.colonies[0]
        .cats
        .iter()
        .take(2)
        .map(|cat| cat.id.clone())
        .collect::<Vec<_>>();

    assert_eq!(
        attempt_hunting_lair(
            &mut world,
            "colony-1",
            lair,
            &party,
            AttemptAuthority::PlayerNudge,
            0,
        )
        .unwrap_err(),
        HuntingAttemptError::PartyTooLarge {
            supplied: 2,
            cap: 1
        }
    );

    world.colonies[0]
        .upgrade_tree
        .owned_node_ids
        .push("hunting_bulk".to_owned());
    let error = attempt_hunting_lair(
        &mut world,
        "colony-1",
        lair,
        &party,
        AttemptAuthority::PlayerNudge,
        0,
    )
    .unwrap_err();
    assert_ne!(
        error,
        HuntingAttemptError::PartyTooLarge {
            supplied: 2,
            cap: 1
        }
    );
}

#[test]
fn snapshot_and_player_nudge_expose_only_revealed_enemy_lairs() {
    let (mut world, lair, quarry) = mapped_world();
    reconcile_hunting_lairs(&mut world.shared_spatial, world.world_seed, 0);
    let action = ClientAction::NudgeHuntingSite {
        session_id: "session".to_owned(),
        nickname: "Cat".to_owned(),
        sig: "signed".to_owned(),
        site_id: Some(format!("enemy-lair:{}:{}", lair.x, lair.y)),
    };
    let result = apply_action(
        &mut world,
        &action,
        &ActionCtx {
            session_id: "session".to_owned(),
            player_id: "player".to_owned(),
            colony_id: "colony-1".to_owned(),
            now_ms: 1_000,
        },
    );
    assert!(result.ok, "{:?}", result.message);

    let snapshot = build_snapshot(&world, 1_000, 1);
    let colony = &snapshot.colonies[0];
    let hunting = colony.hunting_lair.as_ref().expect("revealed lair network");
    assert_eq!(hunting.revealed_sites.len(), 1);
    assert_eq!(
        hunting.nudged_site_id.as_deref(),
        Some(format!("enemy-lair:{}:{}", lair.x, lair.y).as_str())
    );
    assert_eq!(
        colony.revealed_quarry_sites,
        vec![cat_protocol::TilePoint {
            x: quarry.x,
            y: quarry.y,
        }]
    );
}

#[test]
fn darkness_reveals_hunting_materials_to_the_black_hole_by_tier() {
    let (mut world, _, _) = mapped_world();
    let mut hole = BlackHoleRuntime::for_building("hole");
    hole.axes = BlackHoleAxes::new(0, 0, 3).unwrap();
    world.colonies[0]
        .black_holes
        .insert("hole".to_owned(), hole);
    world.shared_spatial.hunting_materials.insert(
        "colony-1".to_owned(),
        [
            (SpeciesMaterial::FoxPelt, 2),
            (SpeciesMaterial::BadgerPelt, 1),
            (SpeciesMaterial::BearPelt, 4),
            (SpeciesMaterial::BeastCore, 1),
        ]
        .into_iter()
        .collect(),
    );

    let snapshot = build_snapshot(&world, 0, 1);
    let items = &snapshot.colonies[0]
        .black_hole
        .as_ref()
        .expect("Black Hole snapshot")
        .accepted_items;
    assert!(items.iter().any(|item| item.kind_id == "fox_pelt"));
    assert!(items.iter().any(|item| item.kind_id == "badger_pelt"));
    assert!(!items.iter().any(|item| item.kind_id == "bear_pelt"));
    assert!(!items.iter().any(|item| item.kind_id == "beast_core"));
}

#[test]
fn player_nudge_creates_a_delayed_reserved_party_then_resolves_it() {
    let (mut world, lair, _) = mapped_world();
    reconcile_hunting_lairs(&mut world.shared_spatial, world.world_seed, 0);
    let cat_id = world.colonies[0].cats[0].id.clone();
    world.colonies[0].leader_id = Some(cat_id.clone());
    let cat = &mut world.colonies[0].cats[0];
    cat.stats.attack = 100.0;
    cat.stats.defense = 100.0;
    cat.stats.hunting = 100.0;
    cat.needs.health = 100.0;
    equip_hunter(&mut world, &cat_id);
    let action = ClientAction::NudgeHuntingSite {
        session_id: "session".to_owned(),
        nickname: "Cat".to_owned(),
        sig: "signed".to_owned(),
        site_id: Some(format!("enemy-lair:{}:{}", lair.x, lair.y)),
    };
    assert!(
        apply_action(
            &mut world,
            &action,
            &ActionCtx {
                session_id: "session".to_owned(),
                player_id: "player".to_owned(),
                colony_id: "colony-1".to_owned(),
                now_ms: 1_000,
            },
        )
        .ok
    );

    tick_hunting_lair_adapter(&mut world, 1_000);
    let party = world
        .shared_spatial
        .active_hunting_parties
        .values()
        .next()
        .expect("Leader accepts safe nudge")
        .clone();
    assert!(world.shared_spatial.recent_hunt_outcomes.is_empty());
    assert_eq!(
        world.colonies[0].cats[0].current_task,
        Some(cat_sim::types::TaskType::Hunt)
    );

    let snapshot = build_snapshot(&world, 1_000, 1);
    assert_eq!(
        snapshot.colonies[0]
            .hunting_lair
            .as_ref()
            .unwrap()
            .active_parties
            .len(),
        1
    );
    tick_hunting_lair_adapter(&mut world, party.resolves_at_ms);
    assert!(world.shared_spatial.active_hunting_parties.is_empty());
    assert_eq!(world.shared_spatial.recent_hunt_outcomes.len(), 1);
    assert_eq!(world.colonies[0].cats[0].current_task, None);
    assert_eq!(
        world.shared_spatial.hunting_attempt_nonces.get(&lair),
        Some(&1)
    );
}

#[test]
fn public_monster_ids_include_the_site_and_never_alias_across_lairs() {
    let (mut world, _, _) = mapped_world();
    let second = TilePos { x: 32, y: 12 };
    let mut tile = world
        .shared_spatial
        .tiles
        .values()
        .find(|tile| tile.tile_type == TileType::EnemyLair)
        .unwrap()
        .clone();
    tile.pos = second;
    world.shared_spatial.tiles.insert(second, tile.clone());
    world.colonies[0].world_tiles.insert(second, tile);
    world.colonies[0].revealed_tiles.insert(second);
    reconcile_hunting_lairs(&mut world.shared_spatial, world.world_seed, 0);

    let snapshot = build_snapshot(&world, 0, 1);
    let sites = &snapshot.colonies[0]
        .hunting_lair
        .as_ref()
        .unwrap()
        .revealed_sites;
    let ids = sites
        .iter()
        .flat_map(|site| site.monsters.iter().map(|monster| monster.id.as_str()))
        .collect::<std::collections::BTreeSet<_>>();
    let monster_count = sites.iter().map(|site| site.monsters.len()).sum::<usize>();
    assert_eq!(ids.len(), monster_count);
}
