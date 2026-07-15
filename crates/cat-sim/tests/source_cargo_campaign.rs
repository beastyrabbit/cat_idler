//! Deterministic unattended and signed-player campaigns for P19.C1 source cargo.

use cat_protocol as proto;
use cat_sim::{
    actions::{ActionCtx, apply_action},
    entities::CarryingKind,
    officers::OfficerRole,
    station_recipes::{HIDE_TO_LEATHER_RECIPE_ID, STONE_TO_BLOCKS_RECIPE_ID},
    stockpiles::{station_input_id, station_output_id},
    storage::BASE_CAPACITY,
    types::{BuildingType, JobKind, JobStatus},
    world_tick::{
        BuildingRuntime, TilePos, WorldState, default_production_queue, found_colony, new_world,
        reconcile_colony_stockpiles, world_tick,
    },
};

const START: i64 = 10_000;

fn ctx(now_ms: i64) -> ActionCtx {
    ActionCtx {
        session_id: "source-cargo-session".to_owned(),
        player_id: "source-cargo-player".to_owned(),
        colony_id: "colony-1".to_owned(),
        now_ms,
    }
}

fn run_passive_hunts(seed: u32) -> (WorldState, bool, bool) {
    let mut world = new_world(seed);
    world
        .colonies
        .push(found_colony(seed, "colony-1", START, seed));
    let mut saw_hide_in_paws = false;
    let mut saw_bone_in_paws = false;
    for minute in 1..=24 * 60i64 {
        let now = START + minute * 60_000;
        let reports = world_tick(&mut world, now);
        assert_eq!(reports[0].reset_reason, None, "passive minute {minute}");
        saw_hide_in_paws |= world.colonies[0].cats.iter().any(|cat| {
            cat.carrying
                .as_ref()
                .is_some_and(|cargo| cargo.kind == CarryingKind::Hide)
        });
        saw_bone_in_paws |= world.colonies[0].cats.iter().any(|cat| {
            cat.carrying
                .as_ref()
                .is_some_and(|cargo| cargo.kind == CarryingKind::Bone)
        });
    }
    (world, saw_hide_in_paws, saw_bone_in_paws)
}

#[test]
fn unattended_founder_hunts_physically_return_hide_deterministically() {
    let (left, left_saw_cargo, left_saw_bone) = run_passive_hunts(7);
    let (right, right_saw_cargo, right_saw_bone) = run_passive_hunts(7);
    assert_eq!(left, right);
    assert_eq!(left_saw_cargo, right_saw_cargo);
    assert_eq!(left_saw_bone, right_saw_bone);
    assert!(
        left_saw_cargo,
        "the passive run never showed Hide in a cat's paws"
    );
    assert!(left.colonies[0].resources.hide > 0.0);
    assert!(
        left_saw_bone,
        "the passive run never showed Bone in a cat's paws"
    );
    assert!(left.colonies[0].resources.bone > 0.0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LeatherRouteObservations {
    hunt_hide_in_paws: bool,
    delivered_hide: bool,
    station_hide_in_paws: bool,
    local_hide: bool,
    local_leather: bool,
    station_leather_in_paws: bool,
    delivered_leather: bool,
}

fn run_signed_hunt_to_leather(seed: u32) -> (WorldState, LeatherRouteObservations) {
    let mut world = new_world(seed);
    world
        .colonies
        .push(found_colony(seed, "colony-1", START, seed));
    world.colonies[0]
        .upgrade_tree
        .owned_node_ids
        .push("textiles".to_owned());
    // Keep the leader's survival director from issuing an autonomous Hunt: every
    // Hide observation below must belong to the exact player-requested job.
    world.colonies[0].resources.food = 100.0;
    world.colonies[0].resources.water = 100.0;
    let anchor = world.colonies[0].anchor;
    // A new founding intentionally has no Tannery. This completed bench is an
    // established-colony fixture; its input still comes only from the real Hunt.
    world.colonies[0].buildings.push(BuildingRuntime {
        id: "guided-tannery".to_owned(),
        building_type: BuildingType::Tannery,
        position: TilePos {
            x: anchor.x + 6,
            y: anchor.y + 6,
        },
        is_complete: true,
        construction_progress: 100,
        production_queue: default_production_queue(BuildingType::Tannery),
        ..BuildingRuntime::default()
    });
    let accelerated = apply_action(
        &mut world,
        &proto::ClientAction::SetTestAcceleration {
            preset: proto::AccelerationPreset::Hyper,
        },
        &ctx(START),
    );
    assert!(accelerated.ok);
    let tannery_id = "guided-tannery".to_owned();
    let worker_id = world.colonies[0]
        .cats
        .last()
        .expect("founding worker")
        .id
        .clone();
    for edit in [
        proto::ProductionQueueEdit::Remove { index: 0 },
        proto::ProductionQueueEdit::Add {
            recipe_id: HIDE_TO_LEATHER_RECIPE_ID.to_owned(),
            repeat: true,
        },
    ] {
        let result = apply_action(
            &mut world,
            &proto::ClientAction::EditProductionQueue {
                session_id: "source-cargo-session".to_owned(),
                nickname: "Guide".to_owned(),
                sig: "pure-sim".to_owned(),
                building_id: tannery_id.clone(),
                edit,
            },
            &ctx(START + 1),
        );
        assert!(result.ok, "signed Tannery queue edit: {result:?}");
    }
    let assigned = apply_action(
        &mut world,
        &proto::ClientAction::AssignWorker {
            session_id: "source-cargo-session".to_owned(),
            nickname: "Guide".to_owned(),
            sig: "pure-sim".to_owned(),
            cat_id: worker_id,
            building_id: Some(tannery_id.clone()),
        },
        &ctx(START + 1),
    );
    assert!(assigned.ok, "signed Tannery assignment: {assigned:?}");
    let hunted = apply_action(
        &mut world,
        &proto::ClientAction::RequestJob {
            session_id: "source-cargo-session".to_owned(),
            nickname: "Guide".to_owned(),
            sig: "pure-sim".to_owned(),
            kind: proto::JobKind::HuntExpedition,
        },
        &ctx(START + 2),
    );
    assert!(hunted.ok, "signed Hunt expedition: {hunted:?}");
    let hunt_job_id = world.colonies[0]
        .jobs
        .iter()
        .find(|job| {
            job.kind == JobKind::HuntExpedition
                && job.requested_by == cat_sim::world_tick::JobRequester::Player
        })
        .expect("the signed action created a player-requested Hunt")
        .id
        .clone();

    let input_id = station_input_id(&tannery_id);
    let output_id = station_output_id(&tannery_id);
    let mut seen = LeatherRouteObservations {
        hunt_hide_in_paws: false,
        delivered_hide: false,
        station_hide_in_paws: false,
        local_hide: false,
        local_leather: false,
        station_leather_in_paws: false,
        delivered_leather: false,
    };
    for second in 1..=3_600i64 {
        let reports = world_tick(&mut world, START + second * 1_000);
        assert_eq!(reports[0].reset_reason, None, "guided second {second}");
        let colony = &world.colonies[0];
        let player_hunter = colony
            .jobs
            .iter()
            .find(|job| job.id == hunt_job_id)
            .and_then(|job| job.assigned_cat.as_deref());
        seen.hunt_hide_in_paws |= player_hunter.is_some_and(|cat_id| {
            colony.cats.iter().any(|cat| {
                cat.id == cat_id
                    && cat.carrying.as_ref().is_some_and(|cargo| {
                        cargo.kind == CarryingKind::Hide
                            && !cargo
                                .source_gather_spot
                                .as_deref()
                                .is_some_and(|marker| marker.starts_with("station-in|"))
                    })
            })
        });
        seen.delivered_hide |= colony.resources.hide > 0.0;
        seen.station_hide_in_paws |= colony.cats.iter().any(|cat| {
            cat.carrying.as_ref().is_some_and(|cargo| {
                cargo.kind == CarryingKind::Hide
                    && cargo.source_gather_spot.as_deref().is_some_and(|marker| {
                        marker.starts_with(&format!("station-in|{tannery_id}|"))
                    })
            })
        });
        seen.local_hide |= colony
            .stockpiles
            .iter()
            .find(|pile| pile.id == input_id)
            .is_some_and(|pile| pile.contents.hide > 0.0);
        seen.local_leather |= colony
            .stockpiles
            .iter()
            .find(|pile| pile.id == output_id)
            .is_some_and(|pile| pile.contents.leather > 0.0);
        seen.station_leather_in_paws |= colony.cats.iter().any(|cat| {
            cat.carrying.as_ref().is_some_and(|cargo| {
                cargo.kind == CarryingKind::Leather
                    && cargo.source_gather_spot.as_deref().is_some_and(|marker| {
                        marker.starts_with(&format!("station-out|{tannery_id}|"))
                    })
            })
        });
        seen.delivered_leather |= colony.resources.leather > 0.0;
        let player_hunt_completed = colony.jobs.iter().any(|job| {
            job.id == hunt_job_id
                && job.requested_by == cat_sim::world_tick::JobRequester::Player
                && job.status == JobStatus::Completed
        });
        if player_hunt_completed
            && seen
                == (LeatherRouteObservations {
                    hunt_hide_in_paws: true,
                    delivered_hide: true,
                    station_hide_in_paws: true,
                    local_hide: true,
                    local_leather: true,
                    station_leather_in_paws: true,
                    delivered_leather: true,
                })
        {
            break;
        }
    }
    assert!(world.colonies[0].jobs.iter().any(|job| {
        job.id == hunt_job_id
            && job.requested_by == cat_sim::world_tick::JobRequester::Player
            && job.status == JobStatus::Completed
    }));
    (world, seen)
}

#[test]
fn signed_player_guides_a_real_hunt_hide_through_tannery_to_leather() {
    let (left, left_seen) = run_signed_hunt_to_leather(0xCA7C_1EA7);
    let (right, right_seen) = run_signed_hunt_to_leather(0xCA7C_1EA7);
    assert_eq!(left, right);
    assert_eq!(left_seen, right_seen);
    assert_eq!(
        left_seen,
        LeatherRouteObservations {
            hunt_hide_in_paws: true,
            delivered_hide: true,
            station_hide_in_paws: true,
            local_hide: true,
            local_leather: true,
            station_leather_in_paws: true,
            delivered_leather: true,
        }
    );
}

fn run_passive_established_tannery(seed: u32) -> WorldState {
    let mut world = new_world(seed);
    world
        .colonies
        .push(found_colony(seed, "colony-1", START, seed));
    let colony = &mut world.colonies[0];
    colony
        .upgrade_tree
        .owned_node_ids
        .push("textiles".to_owned());
    let anchor = colony.anchor;
    colony.buildings.push(BuildingRuntime {
        id: "passive-tannery".to_owned(),
        building_type: BuildingType::Tannery,
        position: TilePos {
            x: anchor.x + 6,
            y: anchor.y + 6,
        },
        is_complete: true,
        construction_progress: 100,
        production_queue: default_production_queue(BuildingType::Tannery),
        ..BuildingRuntime::default()
    });
    colony.buildings.push(BuildingRuntime {
        id: "passive-clothier-office".to_owned(),
        building_type: BuildingType::Clothier,
        position: TilePos {
            x: anchor.x - 6,
            y: anchor.y + 6,
        },
        is_complete: true,
        construction_progress: 100,
        production_queue: default_production_queue(BuildingType::Clothier),
        ..BuildingRuntime::default()
    });
    let holder = colony.cats[0].id.clone();
    colony.officers.insert(OfficerRole::ClothLeader, holder);
    for minute in 1..=48 * 60i64 {
        let reports = world_tick(&mut world, START + minute * 60_000);
        assert_eq!(reports[0].reset_reason, None, "passive minute {minute}");
        if world.colonies[0].resources.leather > 0.0 {
            break;
        }
    }
    world
}

#[test]
fn established_cloth_leader_produces_leather_without_further_player_input() {
    let left = run_passive_established_tannery(7);
    let right = run_passive_established_tannery(7);
    assert_eq!(left, right);
    let colony = &left.colonies[0];
    let tannery = colony
        .buildings
        .iter()
        .find(|building| building.id == "passive-tannery")
        .unwrap();
    assert!(
        colony.resources.leather > 0.0,
        "hide={}, officer={:?}, alive={}, assigned={:?}, automated={:?}, progress={}, queue={:?}, local_in={:?}, local_out={:?}",
        colony.resources.hide,
        colony.officers.get(&OfficerRole::ClothLeader),
        colony
            .cats
            .iter()
            .filter(|cat| cat.death_time.is_none())
            .count(),
        tannery.assigned_cat,
        tannery.automated_by,
        tannery.production_progress,
        tannery.production_queue,
        colony
            .stockpiles
            .iter()
            .find(|pile| pile.id == station_input_id(&tannery.id))
            .map(|pile| pile.contents.hide),
        colony
            .stockpiles
            .iter()
            .find(|pile| pile.id == station_output_id(&tannery.id))
            .map(|pile| pile.contents.leather),
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StoneRouteObservations {
    quarry_stone_in_paws: bool,
    ordinary_stone_deposit: bool,
    station_in_stone_in_paws: bool,
    local_stone: bool,
    local_blocks: bool,
    station_out_blocks_in_paws: bool,
    delivered_blocks: bool,
}

fn run_signed_quarry(seed: u32) -> (WorldState, StoneRouteObservations) {
    let mut world = new_world(seed);
    world
        .colonies
        .push(found_colony(seed, "colony-1", START, seed));
    assert_eq!(world.colonies[0].resources.stone, 0.0);
    // Fixture-assisted visibility only: this grants no resource, ownership, office,
    // worker, or job. The signed action still passes the real quarry-site gate.
    let loaded = world.colonies[0]
        .world_tiles
        .keys()
        .copied()
        .collect::<Vec<_>>();
    world.colonies[0].revealed_tiles.extend(loaded);

    let accelerated = apply_action(
        &mut world,
        &proto::ClientAction::SetTestAcceleration {
            preset: proto::AccelerationPreset::Hyper,
        },
        &ctx(START),
    );
    assert!(accelerated.ok, "acceleration fixture failed");
    let stone_prep_id = world.colonies[0]
        .buildings
        .iter()
        .find(|building| building.building_type == BuildingType::StonePrep && building.is_complete)
        .expect("fresh village has a completed Stone Prep")
        .id
        .clone();
    let worker_id = world.colonies[0]
        .cats
        .iter()
        .find(|cat| cat.death_time.is_none() && cat.activity == Default::default())
        .expect("fresh village has an available player-directed worker")
        .id
        .clone();
    let blocks_at_founding = world.colonies[0].resources.blocks;
    for edit in [
        proto::ProductionQueueEdit::Remove { index: 0 },
        proto::ProductionQueueEdit::Add {
            recipe_id: STONE_TO_BLOCKS_RECIPE_ID.to_owned(),
            repeat: true,
        },
    ] {
        let queued = apply_action(
            &mut world,
            &proto::ClientAction::EditProductionQueue {
                session_id: "source-cargo-session".to_owned(),
                nickname: "Guide".to_owned(),
                sig: "pure-sim".to_owned(),
                building_id: stone_prep_id.clone(),
                edit,
            },
            &ctx(START + 1),
        );
        assert!(queued.ok, "signed Stone Prep queue edit failed: {queued:?}");
    }
    let assigned = apply_action(
        &mut world,
        &proto::ClientAction::AssignWorker {
            session_id: "source-cargo-session".to_owned(),
            nickname: "Guide".to_owned(),
            sig: "pure-sim".to_owned(),
            cat_id: worker_id,
            building_id: Some(stone_prep_id.clone()),
        },
        &ctx(START + 1),
    );
    assert!(
        assigned.ok,
        "signed Stone Prep assignment failed: {assigned:?}"
    );
    let ordered = apply_action(
        &mut world,
        &proto::ClientAction::RequestJob {
            session_id: "source-cargo-session".to_owned(),
            nickname: "Guide".to_owned(),
            sig: "pure-sim".to_owned(),
            kind: proto::JobKind::Quarry,
        },
        &ctx(START + 2),
    );
    assert!(ordered.ok, "signed quarry failed: {:?}", ordered.message);

    let input_id = station_input_id(&stone_prep_id);
    let output_id = station_output_id(&stone_prep_id);
    let mut observations = StoneRouteObservations {
        quarry_stone_in_paws: false,
        ordinary_stone_deposit: false,
        station_in_stone_in_paws: false,
        local_stone: false,
        local_blocks: false,
        station_out_blocks_in_paws: false,
        delivered_blocks: false,
    };
    for second in 1..=2_400i64 {
        let reports = world_tick(&mut world, START + second * 1_000);
        assert_eq!(reports[0].reset_reason, None, "guided second {second}");
        let colony = &world.colonies[0];
        observations.quarry_stone_in_paws |= colony.cats.iter().any(|cat| {
            cat.carrying.as_ref().is_some_and(|cargo| {
                cargo.kind == CarryingKind::Stone
                    && !cargo
                        .source_gather_spot
                        .as_deref()
                        .is_some_and(|marker| marker.starts_with("station-in|"))
            })
        });
        observations.ordinary_stone_deposit |= colony.resources.stone > 0.0;
        observations.station_in_stone_in_paws |= colony.cats.iter().any(|cat| {
            cat.carrying.as_ref().is_some_and(|cargo| {
                cargo.kind == CarryingKind::Stone
                    && cargo.source_gather_spot.as_deref().is_some_and(|marker| {
                        marker.starts_with(&format!("station-in|{stone_prep_id}|"))
                    })
            })
        });
        observations.local_stone |= colony
            .stockpiles
            .iter()
            .find(|pile| pile.id == input_id)
            .is_some_and(|pile| pile.contents.stone > 0.0);
        observations.local_blocks |= colony
            .stockpiles
            .iter()
            .find(|pile| pile.id == output_id)
            .is_some_and(|pile| pile.contents.blocks > 0.0);
        observations.station_out_blocks_in_paws |= colony.cats.iter().any(|cat| {
            cat.carrying.as_ref().is_some_and(|cargo| {
                cargo.kind == CarryingKind::Blocks
                    && cargo.source_gather_spot.as_deref().is_some_and(|marker| {
                        marker.starts_with(&format!("station-out|{stone_prep_id}|"))
                    })
            })
        });
        observations.delivered_blocks |= colony.resources.blocks > blocks_at_founding;
        let quarry_done = world.colonies[0]
            .jobs
            .iter()
            .any(|job| job.kind == JobKind::Quarry && job.status == JobStatus::Completed);
        if quarry_done
            && observations
                == (StoneRouteObservations {
                    quarry_stone_in_paws: true,
                    ordinary_stone_deposit: true,
                    station_in_stone_in_paws: true,
                    local_stone: true,
                    local_blocks: true,
                    station_out_blocks_in_paws: true,
                    delivered_blocks: true,
                })
        {
            break;
        }
    }
    (world, observations)
}

#[test]
fn signed_player_quarry_physically_returns_raw_stone_deterministically() {
    let (left, left_observations) = run_signed_quarry(0xCA7C_0100);
    let (right, right_observations) = run_signed_quarry(0xCA7C_0100);
    assert_eq!(left, right);
    assert_eq!(left_observations, right_observations);
    assert_eq!(
        left_observations,
        StoneRouteObservations {
            quarry_stone_in_paws: true,
            ordinary_stone_deposit: true,
            station_in_stone_in_paws: true,
            local_stone: true,
            local_blocks: true,
            station_out_blocks_in_paws: true,
            delivered_blocks: true,
        },
        "the signed Stone Prep route did not expose every physical stage: stone={}, blocks={}, prep={:?}",
        left.colonies[0].resources.stone,
        left.colonies[0].resources.blocks,
        left.colonies[0]
            .buildings
            .iter()
            .find(|building| building.building_type == BuildingType::StonePrep)
            .map(|building| (&building.assigned_cat, building.production_progress))
    );
}

fn run_passive_forester_stone_prep(seed: u32) -> (WorldState, bool, bool, bool) {
    let mut world = new_world(seed);
    world
        .colonies
        .push(found_colony(seed, "colony-1", START, seed));
    let forester = world.colonies[0].cats[0].id.clone();
    // Provision the office prerequisite and a one-time comfortable larder, then use
    // the real signed action. From the first simulation tick onward the Forester
    // receives no input; the larder merely establishes that survival work is solved
    // well enough for the leader to begin a non-survival refinement route.
    let population = world.colonies[0].cats.len() as f64;
    world.colonies[0].resources.food = population * 10.0;
    world.colonies[0].resources.water = population * 10.0;
    // Keep this campaign Stone-specific. A full compatibility Tool bank makes
    // physical Woodworking non-runnable, while an empty Blocks side gives the
    // physical Stone Prep bench truthful demand.
    world.colonies[0].resources.tools = BASE_CAPACITY.tools;
    world.colonies[0].resources.blocks = 0.0;
    reconcile_colony_stockpiles(&mut world.colonies[0]);
    world.colonies[0]
        .upgrade_tree
        .owned_node_ids
        .push("sawmill".to_owned());
    world.colonies[0].buildings.push(BuildingRuntime {
        id: "passive-forester-sawmill".to_owned(),
        building_type: BuildingType::Sawmill,
        position: TilePos { x: 40, y: 40 },
        is_complete: true,
        construction_progress: 100,
        production_queue: default_production_queue(BuildingType::Sawmill),
        ..BuildingRuntime::default()
    });
    let appointed = apply_action(
        &mut world,
        &proto::ClientAction::AssignOfficer {
            session_id: "source-cargo-session".to_owned(),
            nickname: "Guide".to_owned(),
            sig: "pure-sim".to_owned(),
            role: proto::OfficerRole::Forester,
            cat_id: forester,
        },
        &ctx(START),
    );
    assert!(appointed.ok, "Forester appointment failed: {appointed:?}");
    let stone_prep_id = world.colonies[0]
        .buildings
        .iter()
        .find(|building| building.building_type == BuildingType::StonePrep)
        .expect("founding village has Stone Prep")
        .id
        .clone();
    let input_id = station_input_id(&stone_prep_id);
    let output_id = station_output_id(&stone_prep_id);
    let mut saw_local_stone = false;
    let mut saw_local_blocks = false;
    let mut saw_banked_blocks = false;

    for minute in 1..=45i64 {
        let reports = world_tick(&mut world, START + minute * 60_000);
        assert_eq!(reports[0].reset_reason, None, "passive minute {minute}");
        let colony = &world.colonies[0];
        saw_local_stone |= colony
            .stockpiles
            .iter()
            .find(|pile| pile.id == input_id)
            .is_some_and(|pile| pile.contents.stone > 0.0);
        saw_local_blocks |= colony
            .stockpiles
            .iter()
            .find(|pile| pile.id == output_id)
            .is_some_and(|pile| pile.contents.blocks > 0.0);
        saw_banked_blocks |= colony.resources.blocks > 0.0;
    }
    (world, saw_local_stone, saw_local_blocks, saw_banked_blocks)
}

#[test]
fn appointed_forester_runs_physical_stone_prep_without_further_input_deterministically() {
    let left = run_passive_forester_stone_prep(4242);
    let right = run_passive_forester_stone_prep(4242);
    assert_eq!(left, right, "same passive seed must replay exactly");
    assert!(left.1, "passive Forester never admitted local Stone");
    assert!(left.2, "passive Forester never produced local Blocks");
    assert!(left.3, "passive Forester never banked finite-store Blocks");
}
