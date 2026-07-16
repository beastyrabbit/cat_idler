//! Deterministic unattended and signed-player campaigns for P19.C1 source cargo.

use cat_protocol as proto;
use cat_sim::{
    actions::{ActionCtx, apply_action},
    entities::CarryingKind,
    items::{ItemKind, ItemLocation, Material},
    officers::OfficerRole,
    station_recipes::{
        HIDE_TO_LEATHER_RECIPE_ID, SMELTER_RECIPE_ID, SMITHY_TOOL_RECIPE_ID,
        SMITHY_WEAPON_RECIPE_ID, STONE_TO_BLOCKS_RECIPE_ID,
    },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ClothRouteObservations {
    forage_fibre_in_paws: bool,
    delivered_fibre: bool,
    station_fibre_in_paws: bool,
    local_fibre: bool,
    local_thread: bool,
    station_thread_out_paws: bool,
    delivered_thread: bool,
    station_thread_in_paws: bool,
    local_cloth: bool,
    station_cloth_in_paws: bool,
    delivered_cloth: bool,
}

fn run_signed_forage_to_cloth(seed: u32) -> (WorldState, ClothRouteObservations) {
    let mut world = new_world(seed);
    world
        .colonies
        .push(found_colony(seed, "colony-1", START, seed));
    let colony = &mut world.colonies[0];
    colony.resources.food = 100.0;
    colony.resources.water = 100.0;
    colony.recipe_entitlement_rules_version =
        cat_sim::world_tick::CURRENT_RECIPE_ENTITLEMENT_RULES_VERSION;
    colony
        .upgrade_tree
        .owned_node_ids
        .push("textiles".to_owned());
    let anchor = colony.anchor;
    let clothier_id = "guided-clothier".to_owned();
    colony.buildings.push(BuildingRuntime {
        id: clothier_id.clone(),
        building_type: BuildingType::Clothier,
        position: TilePos {
            x: anchor.x + 6,
            y: anchor.y + 6,
        },
        is_complete: true,
        construction_progress: 100,
        production_queue: default_production_queue(BuildingType::Clothier),
        ..BuildingRuntime::default()
    });
    reconcile_colony_stockpiles(colony);
    let accelerated = apply_action(
        &mut world,
        &proto::ClientAction::SetTestAcceleration {
            preset: proto::AccelerationPreset::Hyper,
        },
        &ctx(START),
    );
    assert!(accelerated.ok);
    let worker_id = world.colonies[0].cats.last().unwrap().id.clone();
    for edit in [
        proto::ProductionQueueEdit::Remove { index: 0 },
        proto::ProductionQueueEdit::Remove { index: 0 },
        proto::ProductionQueueEdit::Add {
            recipe_id: cat_sim::station_recipes::FIBRE_TO_THREAD_RECIPE_ID.to_owned(),
            repeat: false,
        },
        proto::ProductionQueueEdit::Add {
            recipe_id: cat_sim::station_recipes::FIBRE_TO_CLOTH_RECIPE_ID.to_owned(),
            repeat: false,
        },
        proto::ProductionQueueEdit::SetPaused { paused: true },
    ] {
        let result = apply_action(
            &mut world,
            &proto::ClientAction::EditProductionQueue {
                session_id: "source-cargo-session".to_owned(),
                nickname: "Weaver".to_owned(),
                sig: "pure-sim".to_owned(),
                building_id: clothier_id.clone(),
                edit,
            },
            &ctx(START + 1),
        );
        assert!(result.ok, "signed Clothier queue: {result:?}");
    }
    let assigned = apply_action(
        &mut world,
        &proto::ClientAction::AssignWorker {
            session_id: "source-cargo-session".to_owned(),
            nickname: "Weaver".to_owned(),
            sig: "pure-sim".to_owned(),
            cat_id: worker_id,
            building_id: Some(clothier_id.clone()),
        },
        &ctx(START + 1),
    );
    assert!(assigned.ok);
    let mut now = START + 2;
    let mut seen = ClothRouteObservations::default();
    for request in 0..5 {
        let foraged = apply_action(
            &mut world,
            &proto::ClientAction::RequestJob {
                session_id: "source-cargo-session".to_owned(),
                nickname: "Weaver".to_owned(),
                sig: "pure-sim".to_owned(),
                kind: proto::JobKind::ForageFibre,
            },
            &ctx(now),
        );
        assert!(foraged.ok, "signed Fibre forage {request}: {foraged:?}");
        let expected = f64::from(request + 1);
        for _ in 0..600 {
            now += 1_000;
            let reports = world_tick(&mut world, now);
            assert_eq!(reports[0].reset_reason, None);
            let colony = &world.colonies[0];
            seen.forage_fibre_in_paws |= colony.cats.iter().any(|cat| {
                cat.carrying.as_ref().is_some_and(|cargo| {
                    cargo.kind == CarryingKind::Fibre && cargo.source_gather_spot.is_none()
                })
            });
            seen.delivered_fibre |= colony.resources.fibre > 0.0;
            if colony.resources.fibre >= expected {
                break;
            }
        }
        assert!(
            world.colonies[0].resources.fibre >= expected,
            "signed Fibre forage {request} did not return to storage"
        );
    }
    let resumed = apply_action(
        &mut world,
        &proto::ClientAction::EditProductionQueue {
            session_id: "source-cargo-session".to_owned(),
            nickname: "Weaver".to_owned(),
            sig: "pure-sim".to_owned(),
            building_id: clothier_id.clone(),
            edit: proto::ProductionQueueEdit::SetPaused { paused: false },
        },
        &ctx(now + 1),
    );
    assert!(resumed.ok, "signed Clothier resume: {resumed:?}");
    let input_id = station_input_id(&clothier_id);
    let output_id = station_output_id(&clothier_id);
    for _ in 0..1_800 {
        now += 1_000;
        let reports = world_tick(&mut world, now);
        assert_eq!(reports[0].reset_reason, None);
        let colony = &world.colonies[0];
        seen.forage_fibre_in_paws |= colony.cats.iter().any(|cat| {
            cat.carrying.as_ref().is_some_and(|cargo| {
                cargo.kind == CarryingKind::Fibre && cargo.source_gather_spot.is_none()
            })
        });
        seen.delivered_fibre |= colony.resources.fibre > 0.0;
        seen.station_fibre_in_paws |= colony.cats.iter().any(|cat| {
            cat.carrying.as_ref().is_some_and(|cargo| {
                cargo.kind == CarryingKind::Fibre
                    && cargo.source_gather_spot.as_deref().is_some_and(|marker| {
                        marker.starts_with(&format!("station-in|{clothier_id}|"))
                    })
            })
        });
        seen.local_fibre |= colony
            .stockpiles
            .iter()
            .find(|pile| pile.id == input_id)
            .is_some_and(|pile| pile.contents.fibre > 0.0);
        seen.local_thread |= colony.stockpiles.iter().any(|pile| {
            (pile.id == input_id || pile.id == output_id) && pile.contents.thread > 0.0
        });
        seen.station_thread_out_paws |= colony.cats.iter().any(|cat| {
            cat.carrying.as_ref().is_some_and(|cargo| {
                cargo.kind == CarryingKind::Thread
                    && cargo.source_gather_spot.as_deref().is_some_and(|marker| {
                        marker.starts_with(&format!("station-out|{clothier_id}|"))
                    })
            })
        });
        seen.delivered_thread |= colony.resources.thread > 0.0;
        seen.station_thread_in_paws |= colony.cats.iter().any(|cat| {
            cat.carrying.as_ref().is_some_and(|cargo| {
                cargo.kind == CarryingKind::Thread
                    && cargo.source_gather_spot.as_deref().is_some_and(|marker| {
                        marker.starts_with(&format!("station-in|{clothier_id}|"))
                    })
            })
        });
        seen.local_cloth |= colony
            .stockpiles
            .iter()
            .find(|pile| pile.id == output_id)
            .is_some_and(|pile| pile.contents.cloth > 0.0);
        seen.station_cloth_in_paws |= colony.cats.iter().any(|cat| {
            cat.carrying.as_ref().is_some_and(|cargo| {
                cargo.kind == CarryingKind::Cloth
                    && cargo.source_gather_spot.as_deref().is_some_and(|marker| {
                        marker.starts_with(&format!("station-out|{clothier_id}|"))
                    })
            })
        });
        seen.delivered_cloth |= colony.resources.cloth > 0.0;
        if seen
            == (ClothRouteObservations {
                forage_fibre_in_paws: true,
                delivered_fibre: true,
                station_fibre_in_paws: true,
                local_fibre: true,
                local_thread: true,
                station_thread_out_paws: true,
                delivered_thread: true,
                station_thread_in_paws: true,
                local_cloth: true,
                station_cloth_in_paws: true,
                delivered_cloth: true,
            })
        {
            break;
        }
    }
    (world, seen)
}

#[test]
fn signed_player_guides_real_fibre_forage_through_clothier_to_cloth() {
    let (left, left_seen) = run_signed_forage_to_cloth(0xC107_41E2);
    let (right, right_seen) = run_signed_forage_to_cloth(0xC107_41E2);
    assert_eq!(left, right);
    assert_eq!(left_seen, right_seen);
    assert_eq!(
        left_seen,
        ClothRouteObservations {
            forage_fibre_in_paws: true,
            delivered_fibre: true,
            station_fibre_in_paws: true,
            local_fibre: true,
            local_thread: true,
            station_thread_out_paws: true,
            delivered_thread: true,
            station_thread_in_paws: true,
            local_cloth: true,
            station_cloth_in_paws: true,
            delivered_cloth: true,
        }
    );
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SmithyRouteObservations {
    ore_inbound: bool,
    local_ore: bool,
    local_metal: bool,
    metal_outbound: bool,
    metal_inbound: bool,
    local_smithy_metal: bool,
    local_tool: bool,
    tool_outbound: bool,
    delivered_metal_tool: bool,
}

fn run_signed_ore_to_tool(seed: u32) -> (WorldState, SmithyRouteObservations) {
    let mut world = new_world(seed);
    world
        .colonies
        .push(found_colony(seed, "colony-1", START, seed));
    let colony = &mut world.colonies[0];
    colony.resources.food = 100.0;
    colony.resources.water = 100.0;
    colony.resources.ore = 10.0;
    colony
        .upgrade_tree
        .owned_node_ids
        .extend(["metallurgy_preparation", "toolmaking_staples"].map(str::to_owned));
    let anchor = colony.anchor;
    for (id, building_type, offset) in [
        ("guided-smelter", BuildingType::Smelter, 6),
        ("guided-smithy", BuildingType::Smithy, 10),
    ] {
        colony.buildings.push(BuildingRuntime {
            id: id.to_owned(),
            building_type,
            position: TilePos {
                x: anchor.x + offset,
                y: anchor.y + 6,
            },
            is_complete: true,
            construction_progress: 100,
            production_queue: default_production_queue(building_type),
            ..BuildingRuntime::default()
        });
    }
    reconcile_colony_stockpiles(colony);
    assert!(
        apply_action(
            &mut world,
            &proto::ClientAction::SetTestAcceleration {
                preset: proto::AccelerationPreset::Hyper,
            },
            &ctx(START),
        )
        .ok
    );
    for (building_id, recipe_id, default_entries) in [
        ("guided-smelter", SMELTER_RECIPE_ID, 1_usize),
        ("guided-smithy", SMITHY_TOOL_RECIPE_ID, 3_usize),
    ] {
        for _ in 0..default_entries {
            assert!(
                apply_action(
                    &mut world,
                    &proto::ClientAction::EditProductionQueue {
                        session_id: "source-cargo-session".to_owned(),
                        nickname: "Smith".to_owned(),
                        sig: "pure-sim".to_owned(),
                        building_id: building_id.to_owned(),
                        edit: proto::ProductionQueueEdit::Remove { index: 0 },
                    },
                    &ctx(START + 1),
                )
                .ok
            );
        }
        let added = apply_action(
            &mut world,
            &proto::ClientAction::EditProductionQueue {
                session_id: "source-cargo-session".to_owned(),
                nickname: "Smith".to_owned(),
                sig: "pure-sim".to_owned(),
                building_id: building_id.to_owned(),
                edit: proto::ProductionQueueEdit::Add {
                    recipe_id: recipe_id.to_owned(),
                    repeat: building_id == "guided-smelter",
                },
            },
            &ctx(START + 2),
        );
        assert!(added.ok, "signed queue add for {building_id}: {added:?}");
    }
    for (cat_index, building_id) in [(13, "guided-smelter"), (14, "guided-smithy")] {
        let cat_id = world.colonies[0].cats[cat_index].id.clone();
        assert!(
            apply_action(
                &mut world,
                &proto::ClientAction::AssignWorker {
                    session_id: "source-cargo-session".to_owned(),
                    nickname: "Smith".to_owned(),
                    sig: "pure-sim".to_owned(),
                    cat_id,
                    building_id: Some(building_id.to_owned()),
                },
                &ctx(START + 3),
            )
            .ok
        );
    }

    let mut seen = SmithyRouteObservations::default();
    let mut now = START + 3;
    for _ in 0..3_600 {
        now += 1_000;
        let reports = world_tick(&mut world, now);
        assert_eq!(reports[0].reset_reason, None);
        let colony = &world.colonies[0];
        let station_cargo = |kind, prefix: &str| {
            colony.cats.iter().any(|cat| {
                cat.carrying.as_ref().is_some_and(|cargo| {
                    cargo.kind == kind
                        && cargo
                            .source_gather_spot
                            .as_deref()
                            .is_some_and(|marker| marker.starts_with(prefix))
                })
            })
        };
        seen.ore_inbound |= station_cargo(CarryingKind::Ore, "station-in|guided-smelter|");
        seen.metal_outbound |= station_cargo(CarryingKind::Metal, "station-out|guided-smelter|");
        seen.metal_inbound |= station_cargo(CarryingKind::Metal, "station-in|guided-smithy|");
        seen.tool_outbound |= station_cargo(CarryingKind::Tools, "station-out|guided-smithy|");
        let amount = |id: &str, output: bool, field: fn(&cat_sim::entities::Resources) -> f64| {
            colony
                .stockpiles
                .iter()
                .find(|pile| {
                    pile.id
                        == if output {
                            station_output_id(id)
                        } else {
                            station_input_id(id)
                        }
                })
                .is_some_and(|pile| field(&pile.contents) > 0.0)
        };
        seen.local_ore |= amount("guided-smelter", false, |resources| resources.ore);
        seen.local_metal |= amount("guided-smelter", true, |resources| resources.metal);
        seen.local_smithy_metal |= amount("guided-smithy", false, |resources| resources.metal);
        seen.local_tool |= amount("guided-smithy", true, |resources| resources.tools);
        seen.delivered_metal_tool |= colony.items.instances().any(|instance| {
            instance.item.kind == ItemKind::Tool
                && instance.item.material == Material::Metal
                && instance.credited
        });
        if seen
            == (SmithyRouteObservations {
                ore_inbound: true,
                local_ore: true,
                local_metal: true,
                metal_outbound: true,
                metal_inbound: true,
                local_smithy_metal: true,
                local_tool: true,
                tool_outbound: true,
                delivered_metal_tool: true,
            })
        {
            break;
        }
    }
    (world, seen)
}

#[test]
fn signed_player_guides_ore_through_smelter_and_smithy_to_one_metal_tool() {
    let (left, left_seen) = run_signed_ore_to_tool(0x5A17_4EAF);
    let (right, right_seen) = run_signed_ore_to_tool(0x5A17_4EAF);
    assert_eq!(left, right);
    assert_eq!(left_seen, right_seen);
    assert_eq!(
        left_seen,
        SmithyRouteObservations {
            ore_inbound: true,
            local_ore: true,
            local_metal: true,
            metal_outbound: true,
            metal_inbound: true,
            local_smithy_metal: true,
            local_tool: true,
            tool_outbound: true,
            delivered_metal_tool: true,
        }
    );
    assert!(left.colonies[0].resources.tools >= 1.0);
    assert_eq!(left.colonies[0].resources.armor, 0.0);
    let tool = left.colonies[0]
        .items
        .instances()
        .find(|instance| {
            instance.item.kind == ItemKind::Tool && instance.item.material == Material::Metal
        })
        .expect("the delivered metal tool retains its exact identity");
    assert!(tool.credited);
    assert!(matches!(
        tool.location,
        ItemLocation::Stockpile { .. } | ItemLocation::Equipped { .. }
    ));
}

fn run_passive_established_smithy(
    seed: u32,
    cadence_minutes: i64,
    smithy_recipe_id: &str,
    smithy_study_id: &str,
) -> WorldState {
    let mut world = new_world(seed);
    world
        .colonies
        .push(found_colony(seed, "colony-1", START, seed));
    let colony = &mut world.colonies[0];
    colony.test_resource_decay_multiplier = 0.0;
    colony.resources.food = 500.0;
    colony.resources.water = 200.0;
    colony.resources.ore = 50.0;
    // Keep one complete Smithy batch immediately runnable so the Captain reserves
    // a paw before the director's established-colony employment fill; the Smelter
    // then replenishes this strictly physical starting buffer.
    colony.resources.metal = 2.0;
    colony
        .upgrade_tree
        .owned_node_ids
        .extend(["metallurgy_preparation", smithy_study_id, "barracks"].map(str::to_owned));
    let anchor = colony.anchor;
    for (id, building_type, offset) in [
        ("passive-smelter", BuildingType::Smelter, 6),
        ("passive-smithy", BuildingType::Smithy, 10),
    ] {
        colony.buildings.push(BuildingRuntime {
            id: id.to_owned(),
            building_type,
            position: TilePos {
                x: anchor.x + offset,
                y: anchor.y + 6,
            },
            is_complete: true,
            construction_progress: 100,
            assigned_cat: (building_type == BuildingType::Smelter)
                .then(|| colony.cats[13].id.clone()),
            production_queue: vec![cat_sim::world_tick::ProductionQueueEntry {
                recipe_id: if building_type == BuildingType::Smelter {
                    SMELTER_RECIPE_ID
                } else {
                    smithy_recipe_id
                }
                .to_owned(),
                repeat: true,
            }],
            ..BuildingRuntime::default()
        });
    }
    colony.buildings.push(BuildingRuntime {
        id: "passive-barracks".to_owned(),
        building_type: BuildingType::Barracks,
        position: TilePos {
            x: anchor.x + 14,
            y: anchor.y + 6,
        },
        is_complete: true,
        construction_progress: 100,
        ..BuildingRuntime::default()
    });
    let captain = colony.cats[0].id.clone();
    colony.officers.insert(OfficerRole::Captain, captain);
    reconcile_colony_stockpiles(colony);
    assert!(colony.resources.food >= 5.0 * 15.0);
    assert!(colony.resources.water >= 6.0 * 15.0);

    let mut now = START;
    for _ in 0..(4 * 60 / cadence_minutes) {
        now += cadence_minutes * 60_000;
        let reports = world_tick(&mut world, now);
        assert_eq!(reports[0].reset_reason, None);
    }
    world
}

#[test]
fn passive_captain_runs_smelter_and_smithy_at_one_and_five_minute_cadence() {
    for cadence in [1, 5] {
        let left = run_passive_established_smithy(
            0xCA97_A111,
            cadence,
            SMITHY_WEAPON_RECIPE_ID,
            "weaponsmithing",
        );
        let right = run_passive_established_smithy(
            0xCA97_A111,
            cadence,
            SMITHY_WEAPON_RECIPE_ID,
            "weaponsmithing",
        );
        assert_eq!(left, right, "cadence {cadence} deterministic twin");
        let colony = &left.colonies[0];
        assert!(colony.resources.metal > 0.0 || colony.resources.weapons > 0.0);
        let smithy = colony
            .buildings
            .iter()
            .find(|building| building.id == "passive-smithy")
            .unwrap();
        let local_metal = colony
            .stockpiles
            .iter()
            .find(|pile| pile.id == station_input_id("passive-smithy"))
            .map_or(0.0, |pile| pile.contents.metal);
        assert!(
            colony.resources.weapons > 0.0,
            "cadence {cadence} forged nothing: aggregate metal={}, local metal={local_metal}, assigned={:?}, automated={:?}, progress={}, paused={}, queue={:?}",
            colony.resources.metal,
            smithy.assigned_cat,
            smithy.automated_by,
            smithy.production_progress,
            smithy.production_paused,
            smithy.production_queue,
        );
        assert_eq!(
            colony.items.credited_count(ItemKind::Weapon),
            colony.resources.weapons as u32,
            "finite identities are the scalar authority"
        );
        assert!(colony.items.instances().any(|instance| {
            instance.item.kind == ItemKind::Weapon
                && matches!(
                    instance.location,
                    ItemLocation::Stockpile { .. }
                        | ItemLocation::Carrier { .. }
                        | ItemLocation::Equipped { .. }
                )
        }));
        assert_eq!(colony.metal_forge_progress, 0.0);
        assert!(colony.resources.food >= 5.0 * 15.0);
        assert!(colony.resources.water >= 6.0 * 15.0);
    }
}

#[test]
fn passive_captain_forges_exact_metal_tools_at_one_and_five_minute_cadence() {
    for cadence in [1, 5] {
        let left = run_passive_established_smithy(
            0x7001_CA75,
            cadence,
            SMITHY_TOOL_RECIPE_ID,
            "toolmaking_staples",
        );
        let right = run_passive_established_smithy(
            0x7001_CA75,
            cadence,
            SMITHY_TOOL_RECIPE_ID,
            "toolmaking_staples",
        );
        assert_eq!(left, right, "cadence {cadence} deterministic twin");
        assert!(
            left.colonies[0].items.instances().any(|instance| {
                instance.item.kind == ItemKind::Tool
                    && instance.item.material == Material::Metal
                    && instance.credited
            }),
            "cadence {cadence} never delivered a forged metal tool"
        );
    }
}

fn run_passive_established_textiles(
    seed: u32,
    cadence_minutes: i64,
    horizon_hours: i64,
) -> WorldState {
    let mut world = new_world(seed);
    world
        .colonies
        .push(found_colony(seed, "colony-1", START, seed));
    let colony = &mut world.colonies[0];
    colony
        .upgrade_tree
        .owned_node_ids
        .push("textiles".to_owned());
    colony
        .upgrade_tree
        .owned_node_ids
        .push("irrigation".to_owned());
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
    colony.buildings.extend([
        BuildingRuntime {
            id: "passive-established-granary".to_owned(),
            building_type: BuildingType::FoodStorage,
            level: 10,
            position: TilePos {
                x: anchor.x + 12,
                y: anchor.y,
            },
            is_complete: true,
            construction_progress: 100,
            ..BuildingRuntime::default()
        },
        BuildingRuntime {
            id: "passive-established-water-bowl".to_owned(),
            building_type: BuildingType::WaterBowl,
            level: 10,
            position: TilePos {
                x: anchor.x - 12,
                y: anchor.y,
            },
            is_complete: true,
            construction_progress: 100,
            ..BuildingRuntime::default()
        },
        BuildingRuntime {
            id: "passive-established-field".to_owned(),
            building_type: BuildingType::Field,
            position: TilePos {
                x: anchor.x,
                y: anchor.y + 12,
            },
            is_complete: true,
            construction_progress: 100,
            ..BuildingRuntime::default()
        },
    ]);
    colony.resources.food = 1_000.0;
    colony.resources.water = 1_000.0;
    let cloth_holder = colony.cats[0].id.clone();
    let farmer_holder = colony.cats[1].id.clone();
    colony
        .officers
        .insert(OfficerRole::ClothLeader, cloth_holder);
    colony.officers.insert(OfficerRole::Farmer, farmer_holder);
    reconcile_colony_stockpiles(colony);
    for minute in
        (cadence_minutes..=horizon_hours * 60).step_by(cadence_minutes.try_into().unwrap())
    {
        let reports = world_tick(&mut world, START + minute * 60_000);
        assert_eq!(reports[0].reset_reason, None, "passive minute {minute}");
        let colony = &world.colonies[0];
        let alive = colony
            .cats
            .iter()
            .filter(|cat| cat.death_time.is_none())
            .count() as f64;
        assert!(
            colony.resources.food + colony.resources.fish >= (alive * 5.0).max(20.0)
                && colony.resources.water >= (alive * 6.0).max(20.0),
            "textile automation crossed the comfort reserve at minute {minute}"
        );
        if world.colonies[0].resources.leather > 0.0 && world.colonies[0].resources.cloth > 0.0 {
            break;
        }
    }
    world
}

#[test]
fn established_farmer_and_cloth_leader_produce_leather_and_cloth_without_further_input() {
    let left = run_passive_established_textiles(7, 5, 18);
    let right = run_passive_established_textiles(7, 5, 18);
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
    assert!(
        colony.resources.cloth > 0.0,
        "unattended Farmer/ClothLeader never completed Fibre -> Cloth: fibre={}, thread={}, cloth={}, officer={:?}, assigned={:?}, automated={:?}, progress={}, queue={:?}, local_in={:?}, local_out={:?}",
        colony.resources.fibre,
        colony.resources.thread,
        colony.resources.cloth,
        colony.officers.get(&OfficerRole::ClothLeader),
        colony
            .buildings
            .iter()
            .find(|building| building.id == "passive-clothier-office")
            .and_then(|building| building.assigned_cat.as_deref()),
        colony
            .buildings
            .iter()
            .find(|building| building.id == "passive-clothier-office")
            .and_then(|building| building.automated_by),
        colony
            .buildings
            .iter()
            .find(|building| building.id == "passive-clothier-office")
            .map_or(0.0, |building| building.production_progress),
        colony
            .buildings
            .iter()
            .find(|building| building.id == "passive-clothier-office")
            .map(|building| &building.production_queue),
        colony
            .stockpiles
            .iter()
            .find(|pile| pile.id == station_input_id("passive-clothier-office"))
            .map(|pile| &pile.contents),
        colony
            .stockpiles
            .iter()
            .find(|pile| pile.id == station_output_id("passive-clothier-office"))
            .map(|pile| &pile.contents),
    );
    let minute_cadence = run_passive_established_textiles(7, 1, 18);
    assert!(
        minute_cadence.colonies[0].resources.leather > 0.0
            && minute_cadence.colonies[0].resources.cloth > 0.0,
        "one-minute unattended campaign never completed both physical textile routes"
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
