//! Longitudinal player-guidance campaigns for the manual-to-officer handoff.
//!
//! These tests stay at the real `ClientAction`/`apply_action` boundary. They use a
//! one-second worker cadence for the opening, a bounded accelerated horizon for
//! longer consequences, and deterministic multi-seed twins.

use cat_protocol as proto;
use cat_sim::{
    actions::{ActionCtx, apply_action},
    entities::{CatActivity, MapType, Position},
    officers::OfficerRole as SimOfficerRole,
    types::{BuildingType, CatSpecialization, JobKind, JobStatus, TileType},
    upgrade_tree::{self, UPGRADE_NODES},
    world_tick::{
        BuildingRuntime, RaiderRuntime, TilePos, WorldState, footprint_for, footprint_tiles,
        found_colony, new_world, tile_is_occupied, world_tick,
    },
};

const START: i64 = 10_000;

fn ctx(now_ms: i64) -> ActionCtx {
    ActionCtx {
        session_id: "guided-session".to_owned(),
        player_id: "guided-player".to_owned(),
        colony_id: "colony-1".to_owned(),
        now_ms,
    }
}

fn signed_job(kind: proto::JobKind) -> proto::ClientAction {
    proto::ClientAction::RequestJob {
        session_id: "guided-session".to_owned(),
        nickname: "Guide".to_owned(),
        sig: "pure-sim".to_owned(),
        kind,
    }
}

fn apply_ok(world: &mut WorldState, action: proto::ClientAction, now_ms: i64) {
    let result = apply_action(world, &action, &ctx(now_ms));
    assert!(result.ok, "{action:?} failed: {:?}", result.message);
}

fn apply_if_possible(world: &mut WorldState, action: proto::ClientAction, now_ms: i64) -> bool {
    apply_action(world, &action, &ctx(now_ms)).ok
}

fn signed_officer(role: proto::OfficerRole, cat_id: String) -> proto::ClientAction {
    proto::ClientAction::AssignOfficer {
        session_id: "guided-session".to_owned(),
        nickname: "Guide".to_owned(),
        sig: "pure-sim".to_owned(),
        role,
        cat_id,
    }
}

const fn sim_role(role: proto::OfficerRole) -> SimOfficerRole {
    match role {
        proto::OfficerRole::Steward => SimOfficerRole::Steward,
        proto::OfficerRole::Accountant => SimOfficerRole::Accountant,
        proto::OfficerRole::Forester => SimOfficerRole::Forester,
        proto::OfficerRole::Farmer => SimOfficerRole::Farmer,
        proto::OfficerRole::Captain => SimOfficerRole::Captain,
        proto::OfficerRole::Loremaster => SimOfficerRole::Loremaster,
        proto::OfficerRole::ClothLeader => SimOfficerRole::ClothLeader,
    }
}

fn provision_mature_fixture_land(colony: &mut cat_sim::world_tick::ColonyRuntime) {
    let extension = colony
        .world_tiles
        .keys()
        .copied()
        .filter(|pos| (1..=22).contains(&pos.x) && (1..=22).contains(&pos.y))
        .collect::<Vec<_>>();
    for pos in &extension {
        if (pos.x >= 14 || pos.y >= 14)
            && let Some(tile) = colony.world_tiles.get_mut(pos)
            && tile.resources.water == 0
        {
            // This is claimed mature-settlement land, so provision it with the same
            // authoritative clearing marker as a completed expansion. Merely changing
            // the visible tile type leaves climate-generated 2x3 tree footprints live
            // under the exact spatial occupancy model.
            tile.tile_type = TileType::Meadow;
            tile.overlay_feature = None;
            tile.resources.food = 0;
            tile.resources.herbs = 0;
            tile.resources.water = 0;
            tile.max_resources.food = 0;
            tile.max_resources.herbs = 0;
            tile.danger_level = 0.0;
            tile.path_wear = 0;
            tile.last_depleted = 1;
        }
    }
    colony.claimed_tiles.extend(extension.iter().copied());
    colony.claimed_tiles.sort_by_key(|pos| (pos.y, pos.x));
    colony.claimed_tiles.dedup();
    colony.revealed_tiles.extend(extension);
}

fn install_completed_building(
    colony: &mut cat_sim::world_tick::ColonyRuntime,
    id: impl Into<String>,
    building_type: BuildingType,
) {
    provision_mature_fixture_land(colony);
    let (width, height) = footprint_for(building_type);
    let claimed = colony
        .claimed_tiles
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    let position = (2..=20)
        .flat_map(|y| (2..=20).map(move |x| TilePos { x, y }))
        .filter(|anchor| anchor.x >= 14 || anchor.y >= 14)
        .find(|anchor| {
            let tiles = footprint_tiles(*anchor, width, height);
            tiles.iter().all(|tile| claimed.contains(tile))
                && tiles.iter().all(|tile| {
                    [(0, -1), (1, 0), (0, 1), (-1, 0)]
                        .into_iter()
                        .all(|(dx, dy)| {
                            claimed.contains(&TilePos {
                                x: tile.x + dx,
                                y: tile.y + dy,
                            })
                        })
                })
                && tiles
                    .iter()
                    .all(|tile| !tile_is_occupied(colony, *tile, colony.test_rng_seed.unwrap_or(1)))
        })
        .expect("mature fixture must have a unique claimed buildable site");
    colony.buildings.push(BuildingRuntime {
        id: id.into(),
        building_type,
        level: 1,
        position,
        is_complete: true,
        construction_progress: 100,
        production_progress: 0.0,
        assigned_cat: None,
        automated_by: None,
    });
}

fn reset_work(world: &mut WorldState) {
    let colony = &mut world.colonies[0];
    colony.jobs.clear();
    for cat in &mut colony.cats {
        cat.current_task = None;
        cat.activity = CatActivity::Idle;
        cat.destination = None;
        cat.carrying = None;
    }
    for building in &mut colony.buildings {
        building.assigned_cat = None;
    }
}

fn job_in_flight(world: &WorldState, kind: JobKind) -> bool {
    jobs_in_flight(world, kind) > 0
}

fn jobs_in_flight(world: &WorldState, kind: JobKind) -> usize {
    world.colonies[0]
        .jobs
        .iter()
        .filter(|job| {
            job.kind == kind && matches!(job.status, JobStatus::Queued | JobStatus::Active)
        })
        .count()
}

fn guide_survival(world: &mut WorldState, now_ms: i64, include_expansion: bool) -> usize {
    let mut actions = 0;
    // A fifteen-cat settlement needs parallel expeditions; one eight-hour hunt cannot
    // feed the whole founding roster. A manual player fills the same bounded survival
    // slots the Farmer would own, one accepted click per cadence until each target is met.
    for (kind, sim_kind, target) in [
        (proto::JobKind::HuntExpedition, JobKind::HuntExpedition, 6),
        (proto::JobKind::FetchWater, JobKind::FetchWater, 3),
        (proto::JobKind::Quarry, JobKind::Quarry, 1),
    ] {
        if jobs_in_flight(world, sim_kind) < target
            && apply_if_possible(world, signed_job(kind), now_ms)
        {
            actions += 1;
        }
    }
    if include_expansion
        && !job_in_flight(world, JobKind::ExpandVillage)
        && apply_if_possible(world, signed_job(proto::JobKind::ExpandVillage), now_ms)
    {
        actions += 1;
    }
    actions
}

fn run_fully_manual(seed: u32) -> (WorldState, usize, usize) {
    let mut world = new_world(seed);
    world
        .colonies
        .push(found_colony(seed, "colony-1", START, seed));
    let initial_claim = world.colonies[0].claimed_tiles.len();
    let mut manual_actions = 0;

    // Thirty real worker-minutes at the production one-second cadence, with no
    // acceleration and no officer ever appointed.
    for step in 1..=1_800i64 {
        let now = START + step * 1_000;
        manual_actions += guide_survival(&mut world, now, step == 1);
        let reports = world_tick(&mut world, now);
        assert_eq!(
            reports[0].reset_reason, None,
            "seed {seed}, live step {step}"
        );
        assert!(world.colonies[0].officers.is_empty());
    }

    // Continue the same guided run through a bounded faster horizon so the queued
    // expedition/expansion work actually completes without making the test wait
    // eight wall-clock hours for a base hunt.
    apply_ok(
        &mut world,
        proto::ClientAction::SetTestAcceleration {
            preset: proto::AccelerationPreset::Hyper,
        },
        START + 1_800_000,
    );
    // Acceleration affects durations at dispatch time. Retire the deliberately
    // long live-cadence opening orders, then let the same manual guide reissue
    // them under the selected playtest preset so this bounded campaign reaches
    // their physical return/deposit outcomes.
    reset_work(&mut world);
    for step in 1..=600i64 {
        let now = START + 1_800_000 + step * 1_000;
        manual_actions += guide_survival(&mut world, now, step % 60 == 0);
        let reports = world_tick(&mut world, now);
        assert!(
            reports[0].reset_reason.is_none(),
            "seed {seed}, accelerated step {step}: {:?}",
            reports[0].reset_reason
        );
    }

    assert!(
        manual_actions >= 4,
        "the guide never issued the full work mix"
    );
    assert!(world.colonies[0].resources.food > 0.0);
    assert!(world.colonies[0].resources.water > 0.0);
    assert!(
        world.colonies[0].claimed_tiles.len() > initial_claim,
        "seed {seed}: expansion stayed inert; jobs={:?}, events={:?}",
        world.colonies[0]
            .jobs
            .iter()
            .map(|job| (job.kind, job.status, &job.metadata))
            .collect::<Vec<_>>(),
        world.colonies[0]
            .events
            .iter()
            .rev()
            .take(10)
            .collect::<Vec<_>>()
    );
    assert!(world.colonies[0].officers.is_empty());
    (world, manual_actions, initial_claim)
}

#[test]
fn fully_manual_multi_seed_guidance_survives_and_progresses_at_live_cadence() {
    for seed in [7u32, 555, 2024] {
        let (mut world, actions, initial_claim) = run_fully_manual(seed);
        assert!(actions >= 4);
        assert!(world.colonies[0].claimed_tiles.len() > initial_claim);
        let mut now = 3_000_000;

        // Exercise manual study, staffing, shrine offerings, building, and defense
        // on the same longitudinal state. None of these actions requires an officer.
        reset_work(&mut world);
        world.colonies[0].upgrade_tree = upgrade_tree::create_upgrade_tree_state();
        world.colonies[0].upgrade_tree.research_points = 5.0;
        apply_ok(
            &mut world,
            proto::ClientAction::ResearchNode {
                session_id: "guided-session".to_owned(),
                nickname: "Guide".to_owned(),
                sig: "pure-sim".to_owned(),
                node_id: "research_hut".to_owned(),
            },
            now,
        );
        now += 1;
        world.colonies[0].upgrade_tree.research_points = 5.0;
        apply_ok(
            &mut world,
            proto::ClientAction::ResearchNode {
                session_id: "guided-session".to_owned(),
                nickname: "Guide".to_owned(),
                sig: "pure-sim".to_owned(),
                node_id: "basic_tools".to_owned(),
            },
            now,
        );

        install_completed_building(
            &mut world.colonies[0],
            "guided-research",
            BuildingType::ResearchHut,
        );
        let scholar = world.colonies[0].cats[0].id.clone();
        apply_ok(
            &mut world,
            proto::ClientAction::AssignWorker {
                session_id: "guided-session".to_owned(),
                nickname: "Guide".to_owned(),
                sig: "pure-sim".to_owned(),
                cat_id: scholar,
                building_id: Some("guided-research".to_owned()),
            },
            now + 1,
        );
        now += 1;
        world.colonies[0].resources.food = 200.0;
        world.colonies[0].resources.water = 200.0;
        let research_before = world.colonies[0].upgrade_tree.research_points;
        for _ in 0..5 {
            now += 1_000;
            let reports = world_tick(&mut world, now);
            assert_eq!(reports[0].reset_reason, None);
        }
        assert!(
            world.colonies[0].upgrade_tree.research_points > research_before,
            "seed {seed}: a player-staffed research hut accrued no research"
        );
        reset_work(&mut world);

        world.colonies[0].resources.food = 200.0;
        world.colonies[0].resources.refined = 10.0;
        let blessings_before_tithe = world.colonies[0].global_upgrade_points;
        now += 1;
        apply_ok(
            &mut world,
            proto::ClientAction::OfferTithe {
                session_id: "guided-session".to_owned(),
                nickname: "Guide".to_owned(),
                sig: "pure-sim".to_owned(),
            },
            now,
        );
        assert_eq!(
            world.colonies[0].global_upgrade_points,
            blessings_before_tithe + 2.0,
            "seed {seed}: food plus refined tithe did not credit two blessings"
        );
        world.colonies[0].resources.materials = 100.0;
        let blessings_before_offering = world.colonies[0].global_upgrade_points;
        now += 1;
        apply_ok(
            &mut world,
            proto::ClientAction::OfferMaterials {
                session_id: "guided-session".to_owned(),
                nickname: "Guide".to_owned(),
                sig: "pure-sim".to_owned(),
            },
            now,
        );
        for _ in 0..600 {
            now += 1_000;
            let reports = world_tick(&mut world, now);
            assert_eq!(reports[0].reset_reason, None);
            if world.colonies[0].global_upgrade_points > blessings_before_offering {
                break;
            }
        }
        assert!(
            world.colonies[0].global_upgrade_points > blessings_before_offering,
            "seed {seed}: the material offering never reached the shrine"
        );
        reset_work(&mut world);

        now += 1;
        apply_ok(
            &mut world,
            proto::ClientAction::PlanBuilding {
                session_id: "guided-session".to_owned(),
                nickname: "Guide".to_owned(),
                sig: "pure-sim".to_owned(),
                building_type: proto::BuildingType::AccountingTent,
            },
            now,
        );
        reset_work(&mut world);
        install_completed_building(
            &mut world.colonies[0],
            "guided-barracks",
            BuildingType::Barracks,
        );
        let recruit = world.colonies[0].cats[1].id.clone();
        now += 1;
        apply_ok(
            &mut world,
            proto::ClientAction::TrainWarrior {
                session_id: "guided-session".to_owned(),
                nickname: "Guide".to_owned(),
                sig: "pure-sim".to_owned(),
                cat_id: Some(recruit.clone()),
            },
            now,
        );
        for _ in 0..700 {
            now += 1_000;
            let reports = world_tick(&mut world, now);
            assert_eq!(reports[0].reset_reason, None);
            if world.colonies[0].cats.iter().any(|cat| {
                cat.id == recruit && cat.specialization == Some(CatSpecialization::Warrior)
            }) {
                break;
            }
        }
        assert!(
            world.colonies[0]
                .cats
                .iter()
                .any(|cat| cat.id == recruit
                    && cat.specialization == Some(CatSpecialization::Warrior)),
            "seed {seed}: player-requested warrior training never completed"
        );
        reset_work(&mut world);
        world.colonies[0].active_raid = Some("guided-raid".to_owned());
        world.colonies[0].raiders.clear();
        world.colonies[0].raiders.push(RaiderRuntime {
            id: "guided-raider".to_owned(),
            raid_id: "guided-raid".to_owned(),
            position: Position {
                map: MapType::World,
                x: 0.0,
                y: 0.0,
            },
            destination: None,
            attack: 1.0,
            defense: 1.0,
            health: 36.0,
        });
        let clicks_before = world.colonies[0].raid_clicks;
        let repelled_before = world.colonies[0]
            .events
            .iter()
            .filter(|event| event.kind.wire_kind() == "raid_repelled")
            .count();
        for click in 1..=6 {
            now += 1;
            apply_ok(
                &mut world,
                proto::ClientAction::DefendRaid {
                    session_id: "guided-session".to_owned(),
                    nickname: "Guide".to_owned(),
                    sig: "pure-sim".to_owned(),
                },
                now,
            );
            assert_eq!(
                world.colonies[0]
                    .raiders
                    .iter()
                    .find(|raider| raider.id == "guided-raider")
                    .expect("terminal cleanup belongs to the following world tick")
                    .health,
                36.0 - f64::from(click) * 6.0
            );
        }
        assert_eq!(world.colonies[0].raid_clicks, clicks_before + 6.0);
        now += 1_000;
        let reports = world_tick(&mut world, now);
        assert_eq!(reports[0].reset_reason, None);
        assert_eq!(world.colonies[0].active_raid, None);
        assert!(world.colonies[0].raiders.is_empty());
        assert_eq!(
            world.colonies[0]
                .events
                .iter()
                .filter(|event| event.kind.wire_kind() == "raid_repelled")
                .count(),
            repelled_before + 1,
            "seed {seed}: terminal defense did not produce exactly one cleanup event"
        );
        assert!(world.colonies[0].officers.is_empty());
    }
}

fn run_productive_expansion(seed: u32) -> Vec<TilePos> {
    let mut world = new_world(seed);
    world
        .colonies
        .push(found_colony(seed, "colony-1", START, seed));
    let initial = world.colonies[0].claimed_tiles.len();
    apply_ok(
        &mut world,
        proto::ClientAction::SetTestAcceleration {
            preset: proto::AccelerationPreset::Hyper,
        },
        START,
    );
    apply_ok(
        &mut world,
        signed_job(proto::JobKind::ExpandVillage),
        START + 1,
    );
    for step in 1..=120i64 {
        let _ = world_tick(&mut world, START + step * 1_000);
    }
    assert!(world.colonies[0].claimed_tiles.len() > initial);
    world.colonies[0].claimed_tiles.to_vec()
}

#[test]
fn manual_expansion_is_productive_and_deterministic_across_seeds() {
    for seed in [7u32, 555, 2024] {
        let first = run_productive_expansion(seed);
        assert_eq!(first, run_productive_expansion(seed));
    }
}

#[test]
fn dead_or_missing_officers_receive_a_living_deterministic_successor() {
    for missing in [false, true] {
        let mut world = new_world(77);
        world.colonies.push(found_colony(77, "colony-1", START, 77));
        world.colonies[0].jobs.clear();
        world.colonies[0].resources.food = 20.0;
        provision_role(&mut world.colonies[0], proto::OfficerRole::Farmer);
        let holder = if missing {
            "missing-cat".to_owned()
        } else {
            let id = world.colonies[0].cats[0].id.clone();
            world.colonies[0].cats[0].death_time = Some(START);
            id
        };
        world.colonies[0]
            .officers
            .insert(SimOfficerRole::Farmer, holder.clone());

        let _ = world_tick(&mut world, START + 60_000);

        let successor = world.colonies[0]
            .officers
            .get(&SimOfficerRole::Farmer)
            .expect("a dead or missing occupied office appoints a deterministic successor");
        assert_ne!(successor, &holder);
        assert!(
            world.colonies[0]
                .cats
                .iter()
                .any(|cat| cat.id == *successor && cat.death_time.is_none())
        );
    }
}

#[test]
fn vacant_farmer_leaves_emergency_hunt_orders_to_the_player() {
    let mut vacant = new_world(8080);
    vacant
        .colonies
        .push(found_colony(8080, "colony-1", START, 8080));
    vacant.colonies[0].jobs.clear();
    vacant.colonies[0].resources.food = 10.0;
    vacant.colonies[0].resources.water = 100.0;

    let mut filled = vacant.clone();
    provision_role(&mut filled.colonies[0], proto::OfficerRole::Farmer);
    let farmer = filled.colonies[0].cats[0].id.clone();
    filled.colonies[0]
        .officers
        .insert(SimOfficerRole::Farmer, farmer);

    let _ = world_tick(&mut vacant, START + 60_000);
    assert!(
        !job_in_flight(&vacant, JobKind::LeaderPlanHunt)
            && !job_in_flight(&vacant, JobKind::HuntExpedition),
        "a vacant Farmer office issued an unattended hunt",
    );

    for minute in 1..=30i64 {
        let _ = world_tick(&mut filled, START + minute * 60_000);
        if job_in_flight(&filled, JobKind::LeaderPlanHunt)
            || job_in_flight(&filled, JobKind::HuntExpedition)
        {
            break;
        }
    }
    assert!(
        job_in_flight(&filled, JobKind::LeaderPlanHunt)
            || job_in_flight(&filled, JobKind::HuntExpedition),
        "a filled Farmer office never took over emergency hunting",
    );
}

fn role_prerequisite(role: proto::OfficerRole) -> (proto::BuildingType, BuildingType) {
    match role {
        proto::OfficerRole::Steward => (proto::BuildingType::Workshop, BuildingType::Workshop),
        proto::OfficerRole::Accountant => (
            proto::BuildingType::AccountingTent,
            BuildingType::AccountingTent,
        ),
        proto::OfficerRole::Forester => (proto::BuildingType::Sawmill, BuildingType::Sawmill),
        proto::OfficerRole::Farmer => (proto::BuildingType::Field, BuildingType::Field),
        proto::OfficerRole::Captain => (proto::BuildingType::Barracks, BuildingType::Barracks),
        proto::OfficerRole::Loremaster => {
            (proto::BuildingType::ResearchHut, BuildingType::ResearchHut)
        }
        proto::OfficerRole::ClothLeader => (proto::BuildingType::Clothier, BuildingType::Clothier),
    }
}

const fn role_upgrade(role: proto::OfficerRole) -> &'static str {
    match role {
        proto::OfficerRole::Steward => "basic_tools",
        proto::OfficerRole::Accountant => "basic_tools",
        proto::OfficerRole::Forester => "sawmill",
        proto::OfficerRole::Farmer => "irrigation",
        proto::OfficerRole::Captain => "barracks",
        proto::OfficerRole::Loremaster => "research_hut",
        proto::OfficerRole::ClothLeader => "textiles",
    }
}

fn role_building_id(role: proto::OfficerRole) -> String {
    format!("handoff-{role:?}")
}

fn provision_role(colony: &mut cat_sim::world_tick::ColonyRuntime, role: proto::OfficerRole) {
    let (_, building_type) = role_prerequisite(role);
    install_completed_building(colony, role_building_id(role), building_type);
    let upgrade = role_upgrade(role);
    if !colony
        .upgrade_tree
        .owned_node_ids
        .iter()
        .any(|owned| owned == upgrade)
    {
        colony.upgrade_tree.owned_node_ids.push(upgrade.to_owned());
    }
}

fn manual_actions_for_vacancies(world: &mut WorldState, now_ms: i64) -> usize {
    reset_work(world);
    let filled = world.colonies[0]
        .officers
        .keys()
        .copied()
        .collect::<Vec<_>>();
    let vacant = |role| !filled.contains(&sim_role(role));
    let mut actions = 0;

    for (index, role) in [
        proto::OfficerRole::Steward,
        proto::OfficerRole::Accountant,
        proto::OfficerRole::ClothLeader,
    ]
    .into_iter()
    .enumerate()
    {
        if vacant(role) {
            let cat_id = world.colonies[0].cats[index].id.clone();
            actions += usize::from(apply_if_possible(
                world,
                proto::ClientAction::AssignWorker {
                    session_id: "guided-session".to_owned(),
                    nickname: "Guide".to_owned(),
                    sig: "pure-sim".to_owned(),
                    cat_id,
                    building_id: Some(role_building_id(role)),
                },
                now_ms,
            ));
        }
    }
    if vacant(proto::OfficerRole::Captain) {
        let cat_id = world.colonies[0].cats[3].id.clone();
        actions += usize::from(apply_if_possible(
            world,
            proto::ClientAction::TrainWarrior {
                session_id: "guided-session".to_owned(),
                nickname: "Guide".to_owned(),
                sig: "pure-sim".to_owned(),
                cat_id: Some(cat_id),
            },
            now_ms,
        ));
    }
    for (role, kind) in [
        (proto::OfficerRole::Farmer, proto::JobKind::HuntExpedition),
        (proto::OfficerRole::Forester, proto::JobKind::Quarry),
        (proto::OfficerRole::Loremaster, proto::JobKind::Explore),
    ] {
        if vacant(role) {
            actions += usize::from(apply_if_possible(world, signed_job(kind), now_ms));
        }
    }
    actions
}

fn role_has_automated(world: &WorldState, role: proto::OfficerRole) -> bool {
    let colony = &world.colonies[0];
    match role {
        proto::OfficerRole::Steward => colony.buildings.iter().any(|building| {
            building.id == role_building_id(role) && building.assigned_cat.is_some()
        }),
        proto::OfficerRole::Accountant | proto::OfficerRole::ClothLeader => {
            colony.buildings.iter().any(|building| {
                building.id == role_building_id(role) && building.assigned_cat.is_some()
            })
        }
        proto::OfficerRole::Forester => job_in_flight(world, JobKind::Quarry),
        proto::OfficerRole::Farmer => {
            job_in_flight(world, JobKind::HuntExpedition)
                || job_in_flight(world, JobKind::FetchWater)
        }
        proto::OfficerRole::Captain => job_in_flight(world, JobKind::TrainWarrior),
        proto::OfficerRole::Loremaster => job_in_flight(world, JobKind::Explore),
    }
}

#[test]
fn staged_officer_handoff_reduces_manual_frequency_one_role_at_a_time() {
    let roles = [
        proto::OfficerRole::Steward,
        proto::OfficerRole::Accountant,
        proto::OfficerRole::Forester,
        proto::OfficerRole::Farmer,
        proto::OfficerRole::Captain,
        proto::OfficerRole::Loremaster,
        proto::OfficerRole::ClothLeader,
    ];
    let mut world = new_world(4242);
    world
        .colonies
        .push(found_colony(4242, "colony-1", START, 4242));
    world.colonies[0].buildings.retain(|building| {
        !matches!(
            building.building_type,
            BuildingType::Workshop
                | BuildingType::AccountingTent
                | BuildingType::Sawmill
                | BuildingType::Field
                | BuildingType::Barracks
                | BuildingType::ResearchHut
                | BuildingType::Clothier
        )
    });
    for index in 0..2 {
        let mut extra = world.colonies[0].cats[index].clone();
        extra.id = format!("handoff-extra-{index}");
        extra.name = format!("Handoff Extra {index}");
        world.colonies[0].cats.push(extra);
    }
    world.colonies[0].upgrade_tree.owned_node_ids = UPGRADE_NODES
        .iter()
        .map(|node| node.id.to_owned())
        .collect();
    // Planning a Field is gated by the real village-level progression in addition to
    // its research node. Build a legal level-four civic fixture (20 completed,
    // non-shrine buildings) so every signed PlanBuilding action below crosses the same
    // unlock path as live play instead of bypassing the action boundary.
    for index in 0..20 {
        install_completed_building(
            &mut world.colonies[0],
            format!("handoff-civic-{index}"),
            BuildingType::Beds,
        );
    }

    // Exercise signed planning separately, then install a legal mature completed fixture
    // and prove appointment still rejects each independently missing prerequisite.
    for (index, role) in roles.into_iter().enumerate() {
        reset_work(&mut world);
        let candidate = world.colonies[0].cats[index].id.clone();
        let denied = apply_action(
            &mut world,
            &signed_officer(role, candidate.clone()),
            &ctx(20_000 + index as i64),
        );
        assert!(!denied.ok, "{role:?} ignored its building prerequisite");

        let (proto_building, sim_building) = role_prerequisite(role);
        apply_ok(
            &mut world,
            proto::ClientAction::PlanBuilding {
                session_id: "guided-session".to_owned(),
                nickname: "Guide".to_owned(),
                sig: "pure-sim".to_owned(),
                building_type: proto_building,
            },
            21_000 + index as i64,
        );
        reset_work(&mut world);
        install_completed_building(&mut world.colonies[0], role_building_id(role), sim_building);
        let upgrade = role_upgrade(role);
        world.colonies[0]
            .upgrade_tree
            .owned_node_ids
            .retain(|owned| owned != upgrade);
        let denied = apply_action(
            &mut world,
            &signed_officer(role, candidate),
            &ctx(22_000 + index as i64),
        );
        assert!(!denied.ok, "{role:?} ignored its research prerequisite");
        world.colonies[0]
            .upgrade_tree
            .owned_node_ids
            .push(upgrade.to_owned());
    }

    let mut frequency = Vec::new();
    let mut baseline = world.clone();
    frequency.push(manual_actions_for_vacancies(&mut baseline, 30_000));
    assert_eq!(
        frequency[0], 7,
        "every vacant office needs one manual order"
    );

    for (index, role) in roles.into_iter().enumerate() {
        reset_work(&mut world);
        let candidate = world.colonies[0].cats[index].id.clone();
        apply_ok(
            &mut world,
            signed_officer(role, candidate),
            31_000 + index as i64,
        );

        let mut guided = world.clone();
        frequency.push(manual_actions_for_vacancies(
            &mut guided,
            32_000 + index as i64,
        ));
        assert_eq!(frequency[index + 1], 6 - index);

        // Isolate the newly-appointed role and prove it performs its category on
        // unattended ticks. The cumulative world above remains the handoff source.
        let mut automated = world.clone();
        automated.colonies[0]
            .officers
            .retain(|filled, _| *filled == sim_role(role));
        reset_work(&mut automated);
        automated.colonies[0].resources.food = 50.0;
        automated.colonies[0].resources.water = 50.0;
        automated.colonies[0].resources.materials = 0.0;
        for attempt in 1..=30i64 {
            let now = 100_000 + attempt * 60_000;
            let _ = world_tick(&mut automated, now);
            if role_has_automated(&automated, role) {
                break;
            }
        }
        assert!(
            role_has_automated(&automated, role),
            "{role:?} never took over its manual category"
        );
    }

    assert_eq!(frequency, vec![7, 6, 5, 4, 3, 2, 1, 0]);
}

#[derive(Debug, Clone, PartialEq)]
struct GuidanceOutcome {
    resets: Vec<String>,
    food_bits: u64,
    water_bits: u64,
    population: usize,
    revealed: usize,
}

fn run_guidance(seed: u32, poor: bool) -> GuidanceOutcome {
    let mut world = new_world(seed);
    world
        .colonies
        .push(found_colony(seed, "colony-1", START, seed));
    apply_ok(
        &mut world,
        proto::ClientAction::SetTestAcceleration {
            preset: proto::AccelerationPreset::Hyper,
        },
        START,
    );
    let mut resets = Vec::new();
    for step in 1..=600i64 {
        let now = START + step * 1_000;
        if poor {
            for kind in [proto::JobKind::Explore, proto::JobKind::Quarry] {
                if !job_in_flight(
                    &world,
                    if kind == proto::JobKind::Explore {
                        JobKind::Explore
                    } else {
                        JobKind::Quarry
                    },
                ) {
                    let _ = apply_if_possible(&mut world, signed_job(kind), now);
                }
            }
        } else {
            let _ = guide_survival(&mut world, now, step % 60 == 0);
        }
        let report = world_tick(&mut world, now);
        if let Some(reason) = report[0].reset_reason {
            resets.push(format!("{reason:?}"));
        }
    }
    let colony = &world.colonies[0];
    GuidanceOutcome {
        resets,
        food_bits: colony.resources.food.to_bits(),
        water_bits: colony.resources.water.to_bits(),
        population: colony
            .cats
            .iter()
            .filter(|cat| cat.death_time.is_none())
            .count(),
        revealed: colony.revealed_tiles.len(),
    }
}

#[test]
fn poor_manual_guidance_has_bounded_visible_consequences_and_is_deterministic() {
    for seed in [7u32, 555, 2024] {
        let good = run_guidance(seed, false);
        let bad = run_guidance(seed, true);
        assert_eq!(good, run_guidance(seed, false));
        assert_eq!(bad, run_guidance(seed, true));
        assert!(
            good.resets.len() <= 1 && bad.resets.len() <= 1,
            "guidance caused repeated collapse: good={good:?}, bad={bad:?}"
        );
        let good_stores = f64::from_bits(good.food_bits) + f64::from_bits(good.water_bits);
        let bad_stores = f64::from_bits(bad.food_bits) + f64::from_bits(bad.water_bits);
        assert!(
            bad_stores < good_stores || bad.population < good.population,
            "ignoring survival had no visible cost: good={good:?}, bad={bad:?}"
        );
        assert!(
            bad.revealed >= good.revealed,
            "the poor guide should at least get the exploration it over-prioritized"
        );
    }
}
