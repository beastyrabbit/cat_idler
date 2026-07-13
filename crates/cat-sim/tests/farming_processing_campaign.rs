//! Guided, deterministic farming/forestry campaign at the real client-action boundary.

use std::collections::BTreeSet;

use cat_protocol as proto;
use cat_sim::{
    actions::{ActionCtx, apply_action, build_snapshot},
    climate::ResourceHint,
    entities::{CatActivity, Resources},
    terrain_gen::{
        DecorationRole, WORLD_TERRAIN_OPTIONS, generate_terrain_chunk, tile_climate_biome,
    },
    types::{BuildingType, TileType},
    world_tick::{BuildingRuntime, TilePos, WorldState, found_colony, tile_is_occupied},
};

const START_MS: i64 = 1_000;

fn ctx(now_ms: i64) -> ActionCtx {
    ActionCtx {
        session_id: "guided-session".to_owned(),
        player_id: "guided-player".to_owned(),
        colony_id: "colony-1".to_owned(),
        now_ms,
    }
}

fn building(
    id: &str,
    building_type: BuildingType,
    level: u32,
    position: TilePos,
) -> BuildingRuntime {
    BuildingRuntime {
        id: id.to_owned(),
        building_type,
        level,
        position,
        is_complete: true,
        construction_progress: 100,
        production_progress: 0.0,
        assigned_cat: None,
    }
}

fn apply_ok(world: &mut WorldState, action: proto::ClientAction, now_ms: i64) {
    let result = apply_action(world, &action, &ctx(now_ms));
    assert!(result.ok, "{action:?} failed: {:?}", result.message);
}

fn signed() -> (String, String, String) {
    (
        "guided-session".to_owned(),
        "Playtester".to_owned(),
        "server-verified".to_owned(),
    )
}

fn generated_sites(seed: u32) -> (TilePos, TilePos, TilePos) {
    let mut farm = None;
    let mut forest = None;
    let mut fertile = BTreeSet::new();
    for chunk_y in -12..=12 {
        for chunk_x in -12..=12 {
            for tile in
                generate_terrain_chunk(chunk_x, chunk_y, i64::from(seed), WORLD_TERRAIN_OPTIONS)
            {
                if farm.is_none()
                    && tile.x.abs().max(tile.y.abs()) > 6
                    && tile.climate_biome.properties().fertility >= 1.0
                    && tile.river.is_none()
                    && !matches!(tile.decoration, Some(DecorationRole::Tree { .. }))
                {
                    let site = TilePos {
                        x: tile.x,
                        y: tile.y,
                    };
                    fertile.insert(site);
                    let left = TilePos {
                        x: site.x - 1,
                        y: site.y,
                    };
                    if fertile.contains(&left) {
                        farm = Some((left, site));
                    }
                }
                if forest.is_none()
                    && matches!(tile.decoration, Some(DecorationRole::Tree { .. }))
                    && tile.climate_biome.properties().resource == ResourceHint::Wood
                {
                    forest = Some(TilePos {
                        x: tile.x,
                        y: tile.y,
                    });
                }
                if let (Some((farm_a, farm_b)), Some(forest)) = (farm, forest) {
                    return (farm_a, farm_b, forest);
                }
            }
        }
    }
    panic!("bounded terrain scan must find fertile soil and forest for seed {seed}");
}

fn choose_crop(resources: &Resources) -> proto::CropKind {
    if resources.grain < 4.0 {
        proto::CropKind::Grain
    } else if resources.herbs < 2.0 {
        proto::CropKind::Herb
    } else {
        proto::CropKind::Catnip
    }
}

fn run_guided_campaign(seed: u32) -> WorldState {
    let mut world = WorldState {
        world_seed: seed,
        colonies: vec![found_colony(
            seed,
            "colony-1",
            START_MS,
            seed.wrapping_add(17),
        )],
    };
    let (farm_a, farm_b, forest_site) = generated_sites(seed);
    let colony = &mut world.colonies[0];

    // Keep survival staffed while the explicitly assigned processors work.
    for cat in &mut colony.cats {
        cat.age_hours = 8.0;
    }
    let founders = colony.cats.clone();
    for copy in 1..=2 {
        for founder in &founders {
            let mut cat = founder.clone();
            cat.id = format!("{}-copy-{copy}", founder.id);
            cat.name = format!("{} {copy}", founder.name);
            cat.activity = CatActivity::Idle;
            cat.current_task = None;
            cat.destination = None;
            cat.carrying = None;
            colony.cats.push(cat);
        }
    }
    colony.resources.food = 1_000.0;
    colony.resources.water = 1_000.0;
    colony.resources.materials = 100.0;
    colony.resources.blocks = 20.0;
    colony.resources.planks = 0.0;
    colony.resources.grain = 0.0;
    colony.resources.catnip = 5.0;
    colony.resources.logs = 0.0;
    colony.resources.lumber = 0.0;
    colony.run_started_at = i64::MAX / 4;
    colony.created_at = i64::MAX / 4;
    colony
        .upgrade_tree
        .owned_node_ids
        .extend(["sawmill".to_owned(), "milling".to_owned()]);
    colony.buildings.retain(|building| {
        !matches!(
            building.building_type,
            BuildingType::WoodCutter | BuildingType::StonePrep | BuildingType::Woodworking
        )
    });
    colony.buildings.extend([
        building(
            "guided-granary",
            BuildingType::FoodStorage,
            10,
            TilePos { x: 1, y: 1 },
        ),
        building(
            "guided-water",
            BuildingType::WaterBowl,
            10,
            TilePos { x: 1, y: 4 },
        ),
        building("guided-den", BuildingType::Den, 10, TilePos { x: 4, y: 1 }),
        building("guided-mill", BuildingType::Mill, 1, TilePos { x: 4, y: 4 }),
        building(
            "guided-sawmill",
            BuildingType::Sawmill,
            1,
            TilePos { x: 7, y: 4 },
        ),
    ]);

    // Model a player-expanded claim and an explored forest source.
    let mut farm_tile = colony
        .world_tiles
        .values()
        .next()
        .expect("founding world tile")
        .clone();
    farm_tile.pos = farm_a;
    farm_tile.tile_type = TileType::Field;
    farm_tile.resources.water = 0;
    farm_tile.overlay_feature = None;
    colony.world_tiles.insert(farm_a, farm_tile.clone());
    farm_tile.pos = farm_b;
    colony.world_tiles.insert(farm_b, farm_tile);
    colony.claimed_tiles.extend([farm_a, farm_b]);
    for farm_site in [farm_a, farm_b] {
        assert!(!tile_is_occupied(colony, farm_site, seed));
        assert!(
            tile_climate_biome(seed, farm_site.x, farm_site.y)
                .properties()
                .fertility
                >= 1.0
        );
    }

    let mut forest_tile = colony
        .world_tiles
        .values()
        .next()
        .expect("founding world tile")
        .clone();
    forest_tile.pos = forest_site;
    forest_tile.tile_type = TileType::Forest;
    forest_tile.path_wear = 63;
    forest_tile.overlay_feature = None;
    colony.world_tiles.insert(forest_site, forest_tile);

    let farmer_id = colony.cats[0].id.clone();
    let miller_id = colony.cats[1].id.clone();
    let sawyer_id = colony.cats[2].id.clone();
    let crop = choose_crop(&colony.resources);
    assert_eq!(
        crop,
        proto::CropKind::Grain,
        "the empty mill store drives crop choice"
    );

    let (session_id, nickname, sig) = signed();
    apply_ok(
        &mut world,
        proto::ClientAction::AssignOfficer {
            session_id,
            nickname,
            sig,
            role: proto::OfficerRole::Farmer,
            cat_id: farmer_id,
        },
        START_MS,
    );
    for (cat_id, building_id) in [(miller_id, "guided-mill"), (sawyer_id, "guided-sawmill")] {
        let (session_id, nickname, sig) = signed();
        apply_ok(
            &mut world,
            proto::ClientAction::AssignWorker {
                session_id,
                nickname,
                sig,
                cat_id,
                building_id: Some(building_id.to_owned()),
            },
            START_MS,
        );
    }

    let (session_id, nickname, sig) = signed();
    apply_ok(
        &mut world,
        proto::ClientAction::DesignateFarm {
            session_id,
            nickname,
            sig,
            a: proto::TilePoint {
                x: farm_a.x,
                y: farm_a.y,
            },
            b: proto::TilePoint {
                x: farm_b.x,
                y: farm_b.y,
            },
            crop,
        },
        START_MS,
    );
    assert_eq!(
        build_snapshot(&world, START_MS, 1).colonies[0].farms.len(),
        1
    );

    let (session_id, nickname, sig) = signed();
    apply_ok(
        &mut world,
        proto::ClientAction::RequestJob {
            session_id,
            nickname,
            sig,
            kind: proto::JobKind::GatherLogs,
        },
        START_MS,
    );

    // Logging completes first; the second tick harvests and lets both processors
    // consume the newly returned inputs.
    apply_ok(
        &mut world,
        proto::ClientAction::AdvanceTime { seconds: 8 * 3_600 },
        START_MS,
    );
    let after_logging_ms = START_MS + 8 * 3_600 * 1_000;
    apply_ok(
        &mut world,
        proto::ClientAction::AdvanceTime {
            seconds: 16 * 3_600,
        },
        after_logging_ms,
    );
    let after_harvest_ms = after_logging_ms + 16 * 3_600 * 1_000;
    apply_ok(
        &mut world,
        proto::ClientAction::AdvanceTime { seconds: 600 },
        after_harvest_ms,
    );
    let after_processing_ms = after_harvest_ms + 600 * 1_000;

    let colony = &world.colonies[0];
    assert!(
        colony
            .events
            .iter()
            .any(|event| event.message.contains("farmers harvested")),
        "the designated grain crop harvested"
    );
    assert!(
        colony.events.iter().any(|event| {
            event.message.contains("The mill used") && event.message.contains("producing")
        }),
        "the mill ground grain and baked flour into food"
    );
    assert!(
        colony.resources.lumber >= 2.0,
        "the logged timber reached the sawmill (logs={}, lumber={})",
        colony.resources.logs,
        colony.resources.lumber,
    );
    assert_eq!(
        colony.world_tiles[&forest_site].overlay_feature.as_deref(),
        Some("stump"),
        "the explicit logging source was depleted exactly once"
    );

    // Isolate one real PlanBuilding request. Breaking ground must consume new lumber
    // first and leave the legacy plank stock untouched.
    let colony = &mut world.colonies[0];
    colony.jobs.clear();
    for cat in &mut colony.cats {
        cat.activity = CatActivity::Idle;
        cat.current_task = None;
        cat.destination = None;
        cat.carrying = None;
    }
    let lumber_before = colony.resources.lumber;
    let legacy_planks_before = colony.resources.planks;
    let (session_id, nickname, sig) = signed();
    apply_ok(
        &mut world,
        proto::ClientAction::PlanBuilding {
            session_id,
            nickname,
            sig,
            building_type: proto::BuildingType::Den,
        },
        after_processing_ms,
    );
    apply_ok(
        &mut world,
        proto::ClientAction::AdvanceTime { seconds: 1 },
        after_processing_ms,
    );
    let colony = &world.colonies[0];
    assert_eq!(colony.resources.lumber, lumber_before - 2.0);
    assert_eq!(colony.resources.planks, legacy_planks_before);
    assert!(
        colony
            .buildings
            .iter()
            .any(|building| !building.is_complete)
    );

    let plot_id = colony.farms[0].id.clone();
    let (session_id, nickname, sig) = signed();
    apply_ok(
        &mut world,
        proto::ClientAction::ClearFarm {
            session_id,
            nickname,
            sig,
            plot_id,
        },
        after_processing_ms + 1_000,
    );
    assert!(world.colonies[0].farms.is_empty());
    world
}

#[test]
fn guided_farming_forestry_processing_campaign_is_multi_seed_and_deterministic() {
    for seed in [7, 42, 99] {
        let first = run_guided_campaign(seed);
        let second = run_guided_campaign(seed);
        assert_eq!(first, second, "seed {seed} must replay bit-for-bit");
    }
}
