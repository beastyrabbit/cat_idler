//! Guided, deterministic farming/forestry campaign at the real client-action boundary.

use std::collections::BTreeSet;

use cat_protocol as proto;
use cat_sim::{
    actions::{ActionCtx, apply_action, build_snapshot},
    entities::{MapType, Position, Resources},
    stockpiles::{ResourceKind, Stockpile},
    terrain_gen::{
        DecorationRole, WORLD_TERRAIN_OPTIONS, derive_biome_decoration, generate_terrain_chunk,
    },
    types::{BuildingType, JobKind, JobStatus, TileType},
    upgrade_tree,
    world_gen::tile_to_chunk,
    world_tick::{
        BuildingRuntime, ColonyRuntime, JobMetadata, TilePos, WorldState,
        farm_designation_route_blocker, footprint_for, found_colony, found_global_colony,
        inside_village_interior,
    },
    zones::ZoneRect,
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
        automated_by: None,
        additional_work_slots: Vec::new(),
        production_queue: cat_sim::world_tick::default_production_queue(building_type),
        production_paused: false,
        construction_cargo: None,
    }
}

fn apply_ok(world: &mut WorldState, action: proto::ClientAction, now_ms: i64) {
    let result = apply_action(world, &action, &ctx(now_ms));
    assert!(result.ok, "{action:?} failed: {:?}", result.message);
}

fn try_action(world: &mut WorldState, action: proto::ClientAction, now_ms: i64) -> bool {
    apply_action(world, &action, &ctx(now_ms)).ok
}

/// Press the placement UI over deterministic claimed candidates before falling back to the
/// compatibility auto-site request. An explicit legal site reserves its scaffold atomically;
/// this is also how a role-vacant player supplies the otherwise-manual road-placement choice.
fn try_plan_at_claimed_site(
    world: &mut WorldState,
    building_type: proto::BuildingType,
    now_ms: i64,
) -> bool {
    let mut sites = world.colonies[0].claimed_tiles.to_vec();
    sites.sort_by_key(|tile| (tile.y, tile.x));
    for site in sites {
        let (session_id, nickname, sig) = signed();
        let action = proto::ClientAction::PlanBuilding {
            session_id,
            nickname,
            sig,
            building_type,
            site: Some(proto::TilePoint {
                x: site.x,
                y: site.y,
            }),
        };
        let result = apply_action(world, &action, &ctx(now_ms));
        if result.ok {
            return true;
        }
        if result.message.as_deref() == Some("That request is already in progress.") {
            return false;
        }
    }
    let (session_id, nickname, sig) = signed();
    try_action(
        world,
        proto::ClientAction::PlanBuilding {
            session_id,
            nickname,
            sig,
            building_type,
            site: None,
        },
        now_ms,
    )
}

/// Whether the player already has a live scaffold/reservation for this building type.
/// A real player waits on that visible construction instead of repainting every claimed
/// tile with the same plan button each decision step.
fn has_pending_building(colony: &ColonyRuntime, building_type: BuildingType) -> bool {
    colony
        .buildings
        .iter()
        .any(|building| building.building_type == building_type && !building.is_complete)
        || colony.jobs.iter().any(|job| {
            matches!(
                job.status,
                cat_sim::types::JobStatus::Queued | cat_sim::types::JobStatus::Active
            ) && matches!(
                job.metadata,
                JobMetadata::Construction {
                    building_type: candidate,
                    ..
                } if candidate == building_type
            )
        })
}

/// Visible one-tile farm candidates a player would reasonably click. Field-linked
/// expansion explicitly paints prepared agricultural territory outside the palisade,
/// so those tiles come first; other exterior claim-edge tiles are a fallback.
/// Filtering obvious interior misses before sending signed actions keeps the harness
/// faithful to an informed map click instead of brute-forcing every settlement tile.
fn visible_exterior_farm_candidates(colony: &ColonyRuntime) -> Vec<TilePos> {
    let claimed = colony
        .claimed_tiles
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut candidates = claimed
        .iter()
        .copied()
        .filter(|tile| {
            let prepared = colony.agricultural_tiles.contains(tile);
            let claim_edge = [
                TilePos {
                    x: tile.x,
                    y: tile.y - 1,
                },
                TilePos {
                    x: tile.x + 1,
                    y: tile.y,
                },
                TilePos {
                    x: tile.x,
                    y: tile.y + 1,
                },
                TilePos {
                    x: tile.x - 1,
                    y: tile.y,
                },
            ]
            .into_iter()
            .any(|neighbor| !claimed.contains(&neighbor));
            colony.revealed_tiles.contains(tile)
                && !inside_village_interior(colony, *tile)
                && (prepared || claim_edge)
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|tile| {
        (
            !colony.agricultural_tiles.contains(tile),
            (tile.x - colony.anchor.x)
                .abs()
                .max((tile.y - colony.anchor.y).abs()),
            tile.y,
            tile.x,
        )
    });
    candidates
}

fn farm_click_geometry_signature(colony: &ColonyRuntime) -> (usize, usize, usize, usize) {
    (
        colony.claimed_tiles.len(),
        colony.buildings.len(),
        colony.stockpiles.len(),
        colony
            .world_tiles
            .values()
            .filter(|tile| tile.overlay_feature.as_deref() == Some("road_built"))
            .count(),
    )
}

/// Lay the manual access road to a queued site-less construction reservation using only
/// signed `BuildRoad` actions. This makes the strict vacant-Steward dependency explicit in
/// the campaign instead of mutating a road overlay behind the action boundary.
fn try_pave_reserved_build_access(
    world: &mut WorldState,
    building_type: BuildingType,
    now_ms: i64,
) -> bool {
    if world.colonies[0].jobs.iter().any(|job| {
        job.kind == JobKind::BuildRoad
            && matches!(job.status, JobStatus::Queued | JobStatus::Active)
    }) {
        return false;
    }
    let Some(site) = world.colonies[0]
        .jobs
        .iter()
        .find_map(|job| match job.metadata {
            JobMetadata::Construction {
                building_type: candidate,
                building_id: None,
                site: Some(site),
                ..
            } if candidate == building_type => Some(site),
            _ => None,
        })
    else {
        return false;
    };
    let (width, height) = footprint_for(building_type);
    let footprint = (site.y..site.y + height)
        .flat_map(|y| (site.x..site.x + width).map(move |x| TilePos { x, y }))
        .collect::<BTreeSet<_>>();
    let mut entrances = BTreeSet::new();
    for x in site.x..site.x + width {
        entrances.insert(TilePos { x, y: site.y - 1 });
        entrances.insert(TilePos {
            x,
            y: site.y + height,
        });
    }
    for y in site.y..site.y + height {
        entrances.insert(TilePos { x: site.x - 1, y });
        entrances.insert(TilePos {
            x: site.x + width,
            y,
        });
    }
    let roads = world.colonies[0]
        .world_tiles
        .values()
        .filter(|tile| tile.overlay_feature.as_deref() == Some("road_built"))
        .map(|tile| tile.pos)
        .collect::<Vec<_>>();
    let mut pairs = roads
        .into_iter()
        .flat_map(|road| {
            entrances
                .iter()
                .copied()
                .map(move |entrance| (road, entrance))
        })
        .filter(|(road, entrance)| {
            (road.x - entrance.x).abs() + (road.y - entrance.y).abs() <= 24
                && horizontal_then_vertical_path(*road, *entrance)
                    .iter()
                    .all(|tile| !footprint.contains(tile))
        })
        .collect::<Vec<_>>();
    pairs.sort_by_key(|(road, entrance)| {
        (
            (road.x - entrance.x).abs() + (road.y - entrance.y).abs(),
            entrance.y,
            entrance.x,
            road.y,
            road.x,
        )
    });
    for (road, entrance) in pairs {
        let (session_id, nickname, sig) = signed();
        if try_action(
            world,
            proto::ClientAction::BuildRoad {
                session_id,
                nickname,
                sig,
                a: proto::TilePoint {
                    x: road.x,
                    y: road.y,
                },
                b: proto::TilePoint {
                    x: entrance.x,
                    y: entrance.y,
                },
            },
            now_ms,
        ) {
            return !world.colonies[0].jobs.iter().any(|job| {
                job.kind == JobKind::BuildRoad
                    && matches!(job.status, JobStatus::Queued | JobStatus::Active)
            });
        }
    }
    false
}

fn horizontal_then_vertical_path(a: TilePos, b: TilePos) -> Vec<TilePos> {
    let mut path = Vec::new();
    let x_step = (b.x - a.x).signum();
    let mut x = a.x;
    while x != b.x {
        path.push(TilePos { x, y: a.y });
        x += x_step;
    }
    path.push(TilePos { x: b.x, y: a.y });
    let y_step = (b.y - a.y).signum();
    let mut y = a.y + y_step;
    while y_step != 0 && y != b.y + y_step {
        path.push(TilePos { x: b.x, y });
        y += y_step;
    }
    path
}

fn signed() -> (String, String, String) {
    (
        "guided-session".to_owned(),
        "Playtester".to_owned(),
        "server-verified".to_owned(),
    )
}

fn advance_at_player_cadence(world: &mut WorldState, now_ms: &mut i64, seconds: u64) {
    const STEP_SECONDS: u64 = 300;
    let whole_steps = seconds / STEP_SECONDS;
    for _ in 0..whole_steps {
        apply_ok(
            world,
            proto::ClientAction::AdvanceTime {
                seconds: STEP_SECONDS,
            },
            *now_ms,
        );
        *now_ms += i64::try_from(STEP_SECONDS * 1_000).expect("step time fits i64");
    }
    let remainder = seconds % STEP_SECONDS;
    if remainder > 0 {
        apply_ok(
            world,
            proto::ClientAction::AdvanceTime { seconds: remainder },
            *now_ms,
        );
        *now_ms += i64::try_from(remainder * 1_000).expect("remainder time fits i64");
    }
}

fn generated_sites(seed: u32, colony: &ColonyRuntime) -> (TilePos, TilePos, TilePos) {
    let max_x = colony
        .claimed_tiles
        .iter()
        .map(|tile| tile.x)
        .max()
        .expect("founding claim");
    let max_y = colony
        .claimed_tiles
        .iter()
        .map(|tile| tile.y)
        .max()
        .expect("founding claim");
    // A completed two-tile eastward claim expansion: the inner tile touches the old
    // settlement boundary and the parcel stays clear of the authored N/S gate road.
    let farm = Some((
        TilePos {
            x: max_x + 1,
            y: max_y,
        },
        TilePos {
            x: max_x + 2,
            y: max_y,
        },
    ));
    let mut forest = None;
    // Search outward from the founding region. Chunk-coordinate order used to begin at
    // (-12,-12), creating an artificial 140-tile guided expansion and making every
    // movement tick exercise a huge pathfinding envelope.
    for radius in 0_i32..=12 {
        for chunk_y in -radius..=radius {
            for chunk_x in -radius..=radius {
                if chunk_x.abs().max(chunk_y.abs()) != radius {
                    continue;
                }
                for tile in
                    generate_terrain_chunk(chunk_x, chunk_y, i64::from(seed), WORLD_TERRAIN_OPTIONS)
                {
                    if forest.is_none()
                        && tile.x.abs().max(tile.y.abs()) > 16
                        && matches!(
                            derive_biome_decoration(
                                tile.x,
                                tile.y,
                                i64::from(seed),
                                tile.climate_biome,
                            ),
                            Some(DecorationRole::Tree { .. })
                        )
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

fn grant_fixture_research_chain(state: &mut upgrade_tree::UpgradeTreeState, node_id: &str) {
    if upgrade_tree::is_owned(state, node_id) {
        return;
    }
    let node = cat_sim::research_catalog::research_catalog()
        .get(node_id)
        .expect("guided fixture references a canonical research node");
    for prerequisite in &node.prerequisites {
        grant_fixture_research_chain(state, prerequisite);
    }
    state.owned_node_ids.push(node_id.to_owned());
}

fn run_guided_campaign(seed: u32) -> WorldState {
    let mut world = WorldState {
        shared_spatial: Default::default(),
        world_seed: seed,
        colonies: vec![found_colony(
            seed,
            "colony-1",
            START_MS,
            seed.wrapping_add(17),
        )],
    };
    let colony = &mut world.colonies[0];

    // Exercise the real 15-cat founding roster. Twelve cats remain available for
    // survival while the farmer and two explicitly assigned processors work, and
    // the additional guided den leaves one five-bed migration cohort of headroom.
    for cat in &mut colony.cats {
        cat.age_hours = 8.0;
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
    for study in ["carpentry_preparation", "grain_milling_preparation"] {
        grant_fixture_research_chain(&mut colony.upgrade_tree, study);
    }
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
            "guided-field",
            BuildingType::Field,
            1,
            TilePos { x: 7, y: 1 },
        ),
        building(
            "guided-sawmill",
            BuildingType::Sawmill,
            1,
            TilePos { x: 7, y: 4 },
        ),
        building(
            "guided-workshop",
            BuildingType::Workshop,
            1,
            TilePos { x: 10, y: 1 },
        ),
    ]);

    let (farm_a, farm_b, forest_site) = generated_sites(seed, colony);

    // Model a player-expanded claim and an explored forest source.
    let mut farm_tile = colony
        .world_tiles
        .values()
        .next()
        .expect("founding world tile")
        .clone();
    farm_tile.pos = farm_a;
    farm_tile.tile_type = TileType::Field;
    farm_tile.resources.food = 0;
    farm_tile.resources.herbs = 0;
    farm_tile.resources.water = 0;
    farm_tile.max_resources.food = 0;
    farm_tile.max_resources.herbs = 0;
    farm_tile.last_depleted = START_MS;
    farm_tile.overlay_feature = None;
    colony.world_tiles.insert(farm_a, farm_tile.clone());
    farm_tile.pos = farm_b;
    colony.world_tiles.insert(farm_b, farm_tile.clone());
    let farm_handoff = TilePos {
        x: farm_a.x,
        y: farm_a.y - 1,
    };
    farm_tile.pos = farm_handoff;
    // The local basket handoff is already-cleared exterior ground. Its explicit stump
    // marker suppresses any deterministic tree/rock decoration generated underneath.
    farm_tile.overlay_feature = Some("stump".to_owned());
    colony.world_tiles.insert(farm_handoff, farm_tile);
    // Model the completed compact expansion that made this edge parcel claimable. Its
    // center row extends the authored east road to the relocated gate; the farm occupies
    // the outer row without covering that required road approach.
    let mut cleared_expansion_tile = colony.world_tiles[&farm_a].clone();
    cleared_expansion_tile.overlay_feature = Some("stump".to_owned());
    let road_y = colony
        .world_tiles
        .values()
        .find(|tile| {
            tile.pos.x == farm_a.x - 1 && tile.overlay_feature.as_deref() == Some("road_built")
        })
        .map(|tile| tile.pos.y)
        .expect("authored east road reaches the founding boundary");
    let claim_min_y = colony
        .claimed_tiles
        .iter()
        .map(|tile| tile.y)
        .min()
        .expect("founding claim");
    let outer_road_x = farm_b.x + 2;
    for x in farm_a.x..=outer_road_x {
        for y in claim_min_y..=farm_a.y {
            let site = TilePos { x, y };
            if !colony.claimed_tiles.contains(&site) {
                colony.claimed_tiles.push(site);
            }
            colony.revealed_tiles.insert(site);
            let mut tile = cleared_expansion_tile.clone();
            tile.pos = site;
            if y == road_y {
                tile.overlay_feature = Some("road_built".to_owned());
                tile.path_wear = 100;
            }
            colony.world_tiles.insert(site, tile);
        }
    }
    for farm_site in [farm_a, farm_b] {
        assert_eq!(colony.world_tiles[&farm_site].tile_type, TileType::Field);
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
    colony.revealed_tiles.insert(forest_site);
    // Keep this guided action deterministic: the explicitly revealed tree is the
    // only actionable logging decoration, while all unrelated founding trees are
    // already-felled background. The production predicate still comes from the real
    // generated decoration rather than a synthetic coarse forest tile.
    let unrelated_trees = colony
        .world_tiles
        .keys()
        .map(|site| tile_to_chunk(site.x, site.y))
        .map(|chunk| (chunk.chunk_x, chunk.chunk_y))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .flat_map(|(chunk_x, chunk_y)| {
            generate_terrain_chunk(chunk_x, chunk_y, i64::from(seed), WORLD_TERRAIN_OPTIONS)
        })
        .filter(|tile| matches!(tile.decoration, Some(DecorationRole::Tree { .. })))
        .map(|tile| TilePos {
            x: tile.x,
            y: tile.y,
        })
        .filter(|site| *site != forest_site && colony.world_tiles.contains_key(site))
        .collect::<Vec<_>>();
    for site in unrelated_trees {
        colony
            .world_tiles
            .get_mut(&site)
            .expect("tree came from the world tile map")
            .overlay_feature = Some("stump".to_owned());
    }

    let farmer_id = colony.cats[0].id.clone();
    let miller_id = colony.cats[1].id.clone();
    let sawyer_id = colony.cats[2].id.clone();
    let steward_id = colony.cats[3].id.clone();
    let field_worker_id = colony.cats[4].id.clone();
    colony.cats[4].position = Position {
        map: MapType::World,
        x: f64::from(farm_a.x),
        y: f64::from(farm_a.y),
    };
    let crop = choose_crop(&colony.resources);
    assert_eq!(
        crop,
        proto::CropKind::Grain,
        "the empty mill store drives crop choice"
    );

    // This synthetic campaign intentionally starts with a large survival runway.
    // Provision it through the same public stockpile action a player uses: the 4x4
    // warehouse yard contributes 640 units per accepted resource, which combines
    // with the 360-unit founding store without relying on the removed unbounded
    // shrine reservoir.
    let (session_id, nickname, sig) = signed();
    apply_ok(
        &mut world,
        proto::ClientAction::DesignateStockpile {
            session_id,
            nickname,
            sig,
            a: proto::TilePoint { x: 9, y: 9 },
            b: proto::TilePoint { x: 12, y: 12 },
            accepts: vec![proto::ResourceKind::Food, proto::ResourceKind::Water],
        },
        START_MS,
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
    let (session_id, nickname, sig) = signed();
    apply_ok(
        &mut world,
        proto::ClientAction::AssignOfficer {
            session_id,
            nickname,
            sig,
            role: proto::OfficerRole::Steward,
            cat_id: steward_id,
        },
        START_MS,
    );
    for (cat_id, building_id) in [
        (miller_id.clone(), "guided-mill"),
        (sawyer_id.clone(), "guided-sawmill"),
        (field_worker_id, "guided-field"),
    ] {
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
    let player_plot_id = world.colonies[0].farms[0].id.clone();

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

    // Drive time through repeated player actions rather than one giant offline jump.
    // Five-minute decisions let cats physically drink, walk, haul, and return between
    // the guided orders while still covering a full crop cycle quickly.
    let mut campaign_at = START_MS;
    advance_at_player_cadence(&mut world, &mut campaign_at, 8 * 3_600);
    advance_at_player_cadence(&mut world, &mut campaign_at, 16 * 3_600);
    advance_at_player_cadence(&mut world, &mut campaign_at, 600);
    let after_processing_ms = campaign_at;

    let colony = &world.colonies[0];
    assert!(
        colony
            .events
            .iter()
            .any(|event| event.message.contains("farmer harvested")),
        "the designated grain crop harvested: farms={:?}, fields={:?}, officers={:?}",
        colony.farms,
        colony
            .buildings
            .iter()
            .filter(|building| building.building_type == BuildingType::Field)
            .collect::<Vec<_>>(),
        colony.officers,
    );
    assert!(
        colony.events.iter().any(|event| {
            event.message.contains("The mill used") && event.message.contains("awaiting haulage")
        }),
        "the mill consumed delivered grain and left physical output for haulage"
    );
    assert!(
        colony.resources.lumber >= 2.0,
        "the logged timber reached the sawmill (logs={}, lumber={})",
        colony.resources.logs,
        colony.resources.lumber,
    );
    assert!(
        colony.world_tiles[&forest_site].last_depleted > START_MS,
        "the explicit logging source records its depletion"
    );
    assert!(
        matches!(
            colony.world_tiles[&forest_site].overlay_feature.as_deref(),
            Some("stump" | "road_built")
        ),
        "the explicit logging source stays marked as cleared terrain"
    );

    // Guide the processing cats off their benches through the same action a real
    // client uses. This freezes lumber production while the construction spend is
    // observed, without mutating jobs or assignments behind the action boundary.
    let action_at = after_processing_ms;
    for cat_id in [miller_id, sawyer_id] {
        let (session_id, nickname, sig) = signed();
        apply_ok(
            &mut world,
            proto::ClientAction::AssignWorker {
                session_id,
                nickname,
                sig,
                cat_id,
                building_id: None,
            },
            action_at,
        );
    }

    // Clearing is intentionally blocked while harvested produce is still on the plot,
    // in a farmer's basket, or in the local handoff. Keep advancing the signed guided
    // run until the Steward has physically drained a between-harvest window.
    let mut clear_at = action_at + 1_000;
    let mut cleared = false;
    for _ in 0..180 {
        let (session_id, nickname, sig) = signed();
        let result = apply_action(
            &mut world,
            &proto::ClientAction::ClearFarm {
                session_id,
                nickname,
                sig,
                plot_id: player_plot_id.clone(),
            },
            &ctx(clear_at),
        );
        if result.ok {
            cleared = true;
            break;
        }
        assert_eq!(
            result.message.as_deref(),
            Some("This farm still has produce awaiting delivery.")
        );
        apply_ok(
            &mut world,
            proto::ClientAction::AdvanceTime { seconds: 60 },
            clear_at,
        );
        clear_at += 60_000;
    }
    assert!(
        cleared,
        "the Steward eventually drains a clearable farm window"
    );
    assert!(
        world.colonies[0]
            .farms
            .iter()
            .all(|plot| plot.id != player_plot_id),
        "the signed clear action removes the player-selected plot"
    );
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

fn run_signed_player_farm_smoke_from_preearned_research(seed: u32) -> WorldState {
    const STEP_SECONDS: u64 = 15 * 60;
    const MAX_STEPS: usize = 2_400;
    const PRE_EARNED_RESEARCH_POINTS: f64 = 200.0;
    let mut world = WorldState {
        shared_spatial: Default::default(),
        world_seed: seed,
        colonies: vec![found_global_colony(seed, "colony-1", START_MS, seed)],
    };
    // This campaign's scope is player-guided construction and physical production.
    // Begin with a bounded bank earned before the observation window, but spend it
    // only through signed ResearchNode actions below; no ownership is injected.
    world.colonies[0].upgrade_tree.research_points = PRE_EARNED_RESEARCH_POINTS;
    assert!(world.colonies[0].upgrade_tree.owned_node_ids.is_empty());
    let founding_reveal = world.colonies[0].revealed_tiles.len();
    let mut now_ms = START_MS;
    let mut manual_access_road_built = false;
    let mut saw_delivered_flour = false;
    let mut saw_delivered_mill_food = false;
    let mut last_farm_click_geometry = None;

    for step in 0..MAX_STEPS {
        let colony = &world.colonies[0];
        let mill_produced_flour = colony.events.iter().any(|event| {
            event.message.contains("The mill used") && event.message.contains("leaving 2 flour")
        });
        let mill_produced_food = colony.events.iter().any(|event| {
            event.message.contains("The mill used") && event.message.contains("and 4 food")
        });
        saw_delivered_flour |= mill_produced_flour && colony.resources.flour >= 2.0;
        saw_delivered_mill_food |= mill_produced_food
            && colony
                .buildings
                .iter()
                .filter(|building| {
                    building.building_type == BuildingType::Mill && building.is_complete
                })
                .all(|building| {
                    cat_sim::world_tick::building_station_inventory(colony, building, true)
                        .into_iter()
                        .all(|(kind, amount)| {
                            kind != cat_sim::stockpiles::ResourceKind::Food
                                || amount <= f64::EPSILON
                        })
                        && cat_sim::world_tick::building_outbound_haul(colony, building)
                            <= f64::EPSILON
                });
        if colony
            .events
            .iter()
            .any(|event| event.message.contains("farmer harvested"))
            && saw_delivered_flour
            && saw_delivered_mill_food
        {
            assert!(colony.revealed_tiles.len() > founding_reveal);
            assert!(
                manual_access_road_built,
                "the player must visibly solve the vacant-Steward access-road dependency"
            );
            return world;
        }

        // Submit progression before survival errands so the player's architect is not
        // accidentally consumed by a fresh quarry/scout order on the same decision tick.
        // Every mutation below crosses the signed action boundary; state is only inspected
        // to decide which button a real player would press next.
        for node_id in [
            "research_hut",
            "basic_tools",
            "water_carriers",
            "irrigation",
            "milling",
            "foraging_lore",
            "sawmill",
            "masonry",
            "grain_milling_preparation",
            "grain_milling_staples",
        ] {
            let was_owned = upgrade_tree::is_owned(&world.colonies[0].upgrade_tree, node_id);
            let points_before = world.colonies[0].upgrade_tree.research_points;
            let (session_id, nickname, sig) = signed();
            let succeeded = try_action(
                &mut world,
                proto::ClientAction::ResearchNode {
                    session_id,
                    nickname,
                    sig,
                    node_id: node_id.to_owned(),
                },
                now_ms,
            );
            if !was_owned && succeeded {
                let cost = cat_sim::research_catalog::research_catalog()
                    .get(node_id)
                    .expect("signed path uses canonical studies")
                    .cost;
                let spent = points_before - world.colonies[0].upgrade_tree.research_points;
                assert!(
                    (spent - cost).abs() <= f64::EPSILON,
                    "signed purchase {node_id} must spend exactly {cost}, spent {spent}"
                );
            }
        }
        if upgrade_tree::is_owned(&world.colonies[0].upgrade_tree, "grain_milling_preparation") {
            assert!(
                cat_sim::world_tick::production_recipe_availability(
                    &world.colonies[0],
                    BuildingType::Mill,
                    cat_sim::world_tick::MILL_RECIPE_ID,
                )
                .is_some_and(|recipe| recipe.available),
                "the signed preparation purchase must flip the authoritative Mill entitlement"
            );
        }
        let basic_tools = upgrade_tree::is_owned(&world.colonies[0].upgrade_tree, "basic_tools");
        let workshop_complete = world.colonies[0].buildings.iter().any(|building| {
            building.building_type == BuildingType::Workshop && building.is_complete
        });
        if basic_tools && !workshop_complete {
            if !has_pending_building(&world.colonies[0], BuildingType::Workshop) {
                let _ = try_plan_at_claimed_site(&mut world, proto::BuildingType::Workshop, now_ms);
            }
            let paved_now =
                try_pave_reserved_build_access(&mut world, BuildingType::Workshop, now_ms);
            if paved_now {
                assert!(
                    world.colonies[0]
                        .events
                        .iter()
                        .any(|event| event.message.contains("A builder paved road tile")),
                    "the signed road action must emit its visible paved-road event"
                );
            }
            manual_access_road_built |= paved_now;
        }
        // The communal blueprint already owns its office Workshop. Preserve the
        // intended player-input proof by planning and paving a second reachable
        // Workshop while the Steward seat is still vacant, then appoint the Steward
        // to build that additive civic project and the rest of the portfolio.
        if basic_tools && workshop_complete && !manual_access_road_built {
            if !has_pending_building(&world.colonies[0], BuildingType::Workshop) {
                let _ = try_plan_at_claimed_site(&mut world, proto::BuildingType::Workshop, now_ms);
            }
            manual_access_road_built |=
                try_pave_reserved_build_access(&mut world, BuildingType::Workshop, now_ms);
        }
        if basic_tools && workshop_complete && manual_access_road_built {
            for cat_id in world.colonies[0]
                .cats
                .iter()
                .filter(|cat| cat.death_time.is_none())
                .map(|cat| cat.id.clone())
                .collect::<Vec<_>>()
            {
                let (session_id, nickname, sig) = signed();
                if try_action(
                    &mut world,
                    proto::ClientAction::AssignOfficer {
                        session_id,
                        nickname,
                        sig,
                        role: proto::OfficerRole::Steward,
                        cat_id,
                    },
                    now_ms,
                ) {
                    break;
                }
            }
        }

        // This smoke drives the maintained Grand Commons rather than pretending a
        // fresh personal village should rush twenty buildings. Its legitimate communal
        // blueprint starts at fifteen completed non-shrine buildings; Workshop, Mill,
        // water, school, and timber processing form a useful signed-action portfolio
        // that reaches the real level-4 boundary without eleven duplicate granaries.
        let complete_non_shrine = world.colonies[0]
            .buildings
            .iter()
            .filter(|building| {
                building.is_complete && building.building_type != BuildingType::Shrine
            })
            .count();
        if world.colonies[0]
            .officers
            .contains_key(&cat_sim::officers::OfficerRole::Steward)
            && complete_non_shrine < 20
        {
            for (sim_type, proto_type) in [
                (BuildingType::WaterBowl, proto::BuildingType::WaterBowl),
                (BuildingType::Sawmill, proto::BuildingType::Sawmill),
                (BuildingType::School, proto::BuildingType::School),
            ] {
                if !world.colonies[0]
                    .buildings
                    .iter()
                    .any(|building| building.building_type == sim_type && building.is_complete)
                    && !has_pending_building(&world.colonies[0], sim_type)
                {
                    let _ = try_plan_at_claimed_site(&mut world, proto_type, now_ms);
                }
                manual_access_road_built |=
                    try_pave_reserved_build_access(&mut world, sim_type, now_ms);
                if has_pending_building(&world.colonies[0], sim_type) {
                    break;
                }
            }
        }

        let field_complete = world.colonies[0]
            .buildings
            .iter()
            .any(|building| building.building_type == BuildingType::Field && building.is_complete);
        if upgrade_tree::is_owned(&world.colonies[0].upgrade_tree, "irrigation")
            && complete_non_shrine >= 20
            && !field_complete
        {
            if !has_pending_building(&world.colonies[0], BuildingType::Field) {
                let _ = try_plan_at_claimed_site(&mut world, proto::BuildingType::Field, now_ms);
            }
            manual_access_road_built |=
                try_pave_reserved_build_access(&mut world, BuildingType::Field, now_ms);
        }

        if upgrade_tree::is_owned(&world.colonies[0].upgrade_tree, "milling")
            && !world.colonies[0].buildings.iter().any(|building| {
                building.building_type == BuildingType::Mill && building.is_complete
            })
        {
            if !has_pending_building(&world.colonies[0], BuildingType::Mill) {
                let _ = try_plan_at_claimed_site(&mut world, proto::BuildingType::Mill, now_ms);
            }
            manual_access_road_built |=
                try_pave_reserved_build_access(&mut world, BuildingType::Mill, now_ms);
        }

        let completed_fields = world.colonies[0]
            .buildings
            .iter()
            .filter(|building| {
                building.building_type == BuildingType::Field && building.is_complete
            })
            .map(|building| (building.id.clone(), building.assigned_cat.is_none()))
            .collect::<Vec<_>>();
        for (building_id, needs_worker) in completed_fields {
            if needs_worker {
                let assigned = world.colonies[0]
                    .buildings
                    .iter()
                    .filter_map(|building| building.assigned_cat.clone())
                    .collect::<BTreeSet<_>>();
                for cat_id in world.colonies[0]
                    .cats
                    .iter()
                    .filter(|cat| cat.death_time.is_none() && !assigned.contains(&cat.id))
                    .map(|cat| cat.id.clone())
                    .collect::<Vec<_>>()
                {
                    let (session_id, nickname, sig) = signed();
                    if try_action(
                        &mut world,
                        proto::ClientAction::AssignWorker {
                            session_id,
                            nickname,
                            sig,
                            cat_id,
                            building_id: Some(building_id.clone()),
                        },
                        now_ms,
                    ) {
                        break;
                    }
                }
            }
        }

        for building_id in world.colonies[0]
            .buildings
            .iter()
            .filter(|building| {
                building.building_type == BuildingType::Mill
                    && building.is_complete
                    && building.assigned_cat.is_none()
            })
            .map(|building| building.id.clone())
            .collect::<Vec<_>>()
        {
            let assigned = world.colonies[0]
                .buildings
                .iter()
                .filter_map(|building| building.assigned_cat.clone())
                .collect::<BTreeSet<_>>();
            for cat_id in world.colonies[0]
                .cats
                .iter()
                .filter(|cat| cat.death_time.is_none() && !assigned.contains(&cat.id))
                .map(|cat| cat.id.clone())
                .collect::<Vec<_>>()
            {
                let (session_id, nickname, sig) = signed();
                if try_action(
                    &mut world,
                    proto::ClientAction::AssignWorker {
                        session_id,
                        nickname,
                        sig,
                        cat_id,
                        building_id: Some(building_id.clone()),
                    },
                    now_ms,
                ) {
                    break;
                }
            }
        }

        if !world.colonies[0].farms.is_empty() {
            // A plot already exists; the manually assigned Field worker will pick it up.
        } else if !world.colonies[0].buildings.iter().any(|building| {
            building.building_type == BuildingType::Field
                && building.is_complete
                && building.assigned_cat.is_some()
        }) {
            // Wait for the Field and its worker before painting agricultural ground.
        } else {
            let geometry = farm_click_geometry_signature(&world.colonies[0]);
            if last_farm_click_geometry != Some(geometry) {
                last_farm_click_geometry = Some(geometry);
                let candidates = visible_exterior_farm_candidates(&world.colonies[0]);
                for tile in candidates {
                    let (session_id, nickname, sig) = signed();
                    if try_action(
                        &mut world,
                        proto::ClientAction::DesignateFarm {
                            session_id,
                            nickname,
                            sig,
                            a: proto::TilePoint {
                                x: tile.x,
                                y: tile.y,
                            },
                            b: proto::TilePoint {
                                x: tile.x,
                                y: tile.y,
                            },
                            crop: proto::CropKind::Grain,
                        },
                        now_ms,
                    ) {
                        break;
                    }
                }
            }
        }

        // Keep the two founding refineries staffed through the same assignment UI. This
        // supplies the planks/blocks consumed by the deliberately long building campaign.
        for building_id in world.colonies[0]
            .buildings
            .iter()
            .filter(|building| {
                matches!(
                    building.building_type,
                    BuildingType::WoodCutter | BuildingType::StonePrep
                ) && building.assigned_cat.is_none()
            })
            .map(|building| building.id.clone())
            .collect::<Vec<_>>()
        {
            let assigned = world.colonies[0]
                .buildings
                .iter()
                .filter_map(|building| building.assigned_cat.clone())
                .collect::<BTreeSet<_>>();
            for cat_id in world.colonies[0]
                .cats
                .iter()
                .filter(|cat| cat.death_time.is_none() && !assigned.contains(&cat.id))
                .map(|cat| cat.id.clone())
                .collect::<Vec<_>>()
            {
                let (session_id, nickname, sig) = signed();
                if try_action(
                    &mut world,
                    proto::ClientAction::AssignWorker {
                        session_id,
                        nickname,
                        sig,
                        cat_id,
                        building_id: Some(building_id.clone()),
                    },
                    now_ms,
                ) {
                    break;
                }
            }
        }

        let mut requested_jobs = Vec::new();
        if world.colonies[0].resources.food < 80.0 {
            requested_jobs.push(proto::JobKind::HuntExpedition);
        }
        if world.colonies[0].resources.water < 90.0 {
            requested_jobs.push(proto::JobKind::FetchWater);
        }
        if world.colonies[0].resources.materials < 40.0 {
            requested_jobs.push(proto::JobKind::Quarry);
        }
        for kind in requested_jobs {
            let (session_id, nickname, sig) = signed();
            let _ = try_action(
                &mut world,
                proto::ClientAction::RequestJob {
                    session_id,
                    nickname,
                    sig,
                    kind,
                },
                now_ms,
            );
        }
        if step == 0 {
            let (session_id, nickname, sig) = signed();
            let _ = try_action(
                &mut world,
                proto::ClientAction::DispatchScout {
                    session_id,
                    nickname,
                    sig,
                    mission: proto::ScoutMission::Explore,
                },
                now_ms,
            );
        }

        apply_ok(
            &mut world,
            proto::ClientAction::AdvanceTime {
                seconds: STEP_SECONDS,
            },
            now_ms,
        );
        now_ms += i64::try_from(STEP_SECONDS * 1_000).expect("step fits i64");
    }

    let colony = &world.colonies[0];
    let agricultural_route_diagnostics = colony
        .agricultural_tiles
        .iter()
        .copied()
        .map(|tile| {
            (
                tile,
                farm_designation_route_blocker(
                    colony,
                    seed,
                    cat_sim::movement::WorldPos {
                        x: f64::from(colony.anchor.x),
                        y: f64::from(colony.anchor.y),
                    },
                    ZoneRect {
                        x1: tile.x,
                        y1: tile.y,
                        x2: tile.x,
                        y2: tile.y,
                    },
                ),
            )
        })
        .collect::<Vec<_>>();
    let visible_route_diagnostics = visible_exterior_farm_candidates(colony)
        .into_iter()
        .map(|tile| {
            (
                tile,
                farm_designation_route_blocker(
                    colony,
                    seed,
                    cat_sim::movement::WorldPos {
                        x: f64::from(colony.anchor.x),
                        y: f64::from(colony.anchor.y),
                    },
                    ZoneRect {
                        x1: tile.x,
                        y1: tile.y,
                        x2: tile.x,
                        y2: tile.y,
                    },
                ),
            )
        })
        .collect::<Vec<_>>();
    panic!(
        "signed pre-earned-research farm→Mill smoke timed out: alive={} food={} flour={} water={} blessings={} research={} delivered_flour={} delivered_food={} owned={:?} buildings={:?} farms={:?} agricultural_routes={agricultural_route_diagnostics:?} visible_routes={visible_route_diagnostics:?}",
        colony
            .cats
            .iter()
            .filter(|cat| cat.death_time.is_none())
            .count(),
        colony.resources.food,
        colony.resources.flour,
        colony.resources.water,
        colony.global_upgrade_points,
        colony.upgrade_tree.research_points,
        saw_delivered_flour,
        saw_delivered_mill_food,
        colony.upgrade_tree.owned_node_ids,
        colony
            .buildings
            .iter()
            .map(|building| (building.building_type, building.construction_progress))
            .collect::<Vec<_>>(),
        colony.farms,
    );
}

#[test]
fn signed_player_guidance_from_preearned_research_reaches_physical_farm_to_mill_deterministically()
{
    let first = run_signed_player_farm_smoke_from_preearned_research(42);
    let second = run_signed_player_farm_smoke_from_preearned_research(42);
    assert_eq!(first, second);
}

#[test]
fn signed_manual_field_assignment_works_while_farmer_office_is_vacant() {
    let seed = 42;
    let mut world = WorldState {
        shared_spatial: Default::default(),
        world_seed: seed,
        colonies: vec![found_colony(seed, "colony-1", START_MS, 59)],
    };
    let colony = &mut world.colonies[0];
    let (farm_site, farm_neighbor, _) = generated_sites(seed, colony);
    for cat in &mut colony.cats {
        cat.age_hours = 8.0;
    }
    colony.cats.truncate(1);
    colony.leader_id = Some(colony.cats[0].id.clone());
    colony.resources.food = 1_000.0;
    colony.resources.water = 1_000.0;
    let mut tile = colony
        .world_tiles
        .values()
        .next()
        .expect("founding tile")
        .clone();
    tile.pos = farm_site;
    tile.tile_type = TileType::Field;
    tile.resources.food = 0;
    tile.resources.herbs = 0;
    tile.resources.water = 0;
    tile.max_resources.food = 0;
    tile.max_resources.herbs = 0;
    tile.last_depleted = START_MS;
    tile.overlay_feature = None;
    colony.world_tiles.insert(farm_site, tile);
    let mut neighbor = colony.world_tiles[&farm_site].clone();
    neighbor.pos = farm_neighbor;
    colony.world_tiles.insert(farm_neighbor, neighbor);
    colony.claimed_tiles.extend([farm_site, farm_neighbor]);
    colony.revealed_tiles.extend([farm_site, farm_neighbor]);
    let mut cleared_path_tile = colony.world_tiles[&farm_site].clone();
    cleared_path_tile.overlay_feature = Some("stump".to_owned());
    let (min_x, max_x) = if colony.anchor.x <= farm_site.x {
        (colony.anchor.x, farm_site.x)
    } else {
        (farm_site.x, colony.anchor.x)
    };
    let (min_y, max_y) = if colony.anchor.y <= farm_site.y {
        (colony.anchor.y, farm_site.y)
    } else {
        (farm_site.y, colony.anchor.y)
    };
    let connected_claim = (min_x..=max_x)
        .map(|x| TilePos {
            x,
            y: colony.anchor.y,
        })
        .chain((min_y..=max_y).map(|y| TilePos { x: farm_site.x, y }));
    for site in connected_claim {
        if !colony.claimed_tiles.contains(&site) {
            colony.claimed_tiles.push(site);
        }
        colony.revealed_tiles.insert(site);
        let mut path_tile = cleared_path_tile.clone();
        path_tile.pos = site;
        colony.world_tiles.insert(site, path_tile);
    }
    let worker_id = colony.cats[0].id.clone();
    colony.cats[0].position = Position {
        map: MapType::World,
        x: f64::from(farm_site.x),
        y: f64::from(farm_site.y),
    };
    assert!(colony.officers.is_empty());
    colony.stockpiles.push(Stockpile {
        id: "manual-survival-runway".to_owned(),
        rect: ZoneRect {
            x1: 30,
            y1: 30,
            x2: 33,
            y2: 33,
        },
        accepts: [ResourceKind::Food, ResourceKind::Water]
            .into_iter()
            .collect(),
        contents: Resources::default(),
    });

    let (session_id, nickname, sig) = signed();
    apply_ok(
        &mut world,
        proto::ClientAction::DesignateFarm {
            session_id,
            nickname,
            sig,
            a: proto::TilePoint {
                x: farm_site.x,
                y: farm_site.y,
            },
            b: proto::TilePoint {
                x: farm_site.x,
                y: farm_site.y,
            },
            crop: proto::CropKind::Grain,
        },
        START_MS,
    );
    world.colonies[0].buildings.push(building(
        "manual-field",
        BuildingType::Field,
        1,
        TilePos {
            x: farm_site.x + 10,
            y: farm_site.y,
        },
    ));
    let (session_id, nickname, sig) = signed();
    apply_ok(
        &mut world,
        proto::ClientAction::AssignWorker {
            session_id,
            nickname,
            sig,
            cat_id: worker_id.clone(),
            building_id: Some("manual-field".to_owned()),
        },
        START_MS,
    );

    let mut now = START_MS;
    advance_at_player_cadence(&mut world, &mut now, 26 * 3_600);
    let colony = &world.colonies[0];
    assert!(
        colony.officers.is_empty(),
        "manual work never invents an officer"
    );
    assert!(
        colony.resources.grain > 0.0,
        "the manually assigned worker physically harvested and deposited grain; run={} assigned={:?} phase={:?} progress={} pending={} worker_alive={} events={:?}",
        colony.run_number,
        colony
            .buildings
            .iter()
            .find(|building| building.id == "manual-field")
            .and_then(|building| building.assigned_cat.as_deref()),
        colony.farms.first().map(|plot| plot.work_phase),
        colony.farms.first().map_or(0.0, |plot| plot.growth_hours),
        colony.farms.first().map_or(0.0, |plot| plot.pending_output),
        colony
            .cats
            .iter()
            .any(|cat| cat.id == worker_id && cat.death_time.is_none()),
        colony
            .events
            .iter()
            .rev()
            .take(8)
            .map(|event| event.message.as_str())
            .collect::<Vec<_>>(),
    );
    assert!(
        colony.cats[0]
            .skills
            .get(&cat_sim::skills::Labor::Farm)
            .copied()
            .unwrap_or(0.0)
            > 0.0,
        "actual plot work accrues Farm skill"
    );
}
