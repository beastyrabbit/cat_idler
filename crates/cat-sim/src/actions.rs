//! Pure action application and snapshot building ported from
//! `app/api/game/actions/route.ts` and `server/game.ts:getGlobalDashboard`.

use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashSet},
};

use cat_protocol as proto;

use crate::{
    entities::{self, Cat, CatActivity, MapType, Position},
    housing::{self, HousingBuilding},
    idle_engine, idle_rules,
    life_sim::{can_work, get_life_stage},
    production,
    skills::Labor,
    storage::{self, StorageBuilding},
    threat,
    types::{self, BuildingType, CatSpecialization, JobKind, JobStatus, TaskType, UpgradeKey},
    upgrade_tree,
    village_area::{self, gate_placement_default},
    village_layout::{GridPos, VILLAGE_ANCHOR, village_ring_radius},
    world_tick::{
        ColonyRuntime, ConstructionPhase, ElectionKind, ElectionRuntime, EventKind, EventLog,
        JobMetadata, JobRequester, JobRuntime, RaiderRuntime, TilePos, VoteRuntime, WorldState,
        ZoneRuntime, found_colony, world_tick,
    },
    zones,
};

const DEFEND_CLICK_DAMAGE: f64 = 6.0;
const EVENT_KEEP_SNAPSHOT: usize = 30;
const KICK_THRESHOLD: u32 = 5;
const MAX_ADVANCE_SECONDS: u64 = 86_400;

const UPGRADE_DEFAULTS: [(UpgradeKey, u32, u32); 6] = [
    (UpgradeKey::ClickPower, 20, 2),
    (UpgradeKey::SupplySpeed, 10, 3),
    (UpgradeKey::HuntMastery, 10, 5),
    (UpgradeKey::BuildMastery, 10, 5),
    (UpgradeKey::RitualMastery, 10, 6),
    (UpgradeKey::Resilience, 10, 7),
];

/// Pure action context supplied by the server shell.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActionCtx {
    pub session_id: String,
    pub player_id: String,
    pub now_ms: i64,
}

/// Apply a client action to the deterministic world state.
#[must_use]
pub fn apply_action(
    world: &mut WorldState,
    action: &proto::ClientAction,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    match action {
        proto::ClientAction::Ensure => {
            ensure_colony(world, ctx.now_ms);
            ok()
        }
        proto::ClientAction::Presence { .. } => ok(),
        proto::ClientAction::RequestJob { kind, .. } => {
            let kind = proto_to_sim_job_kind(*kind);
            with_colony(world, ctx, |colony| request_job(colony, kind, ctx))
        }
        proto::ClientAction::Boost { job_id, .. } => {
            with_colony(world, ctx, |colony| boost_job(colony, job_id, ctx))
        }
        proto::ClientAction::PurchaseUpgrade { key, .. } => {
            let key = proto_to_sim_upgrade_key(*key);
            with_colony(world, ctx, |colony| purchase_upgrade(colony, key, ctx))
        }
        proto::ClientAction::CastVote {
            election_id,
            cat_id,
            ..
        } => with_colony(world, ctx, |colony| {
            cast_vote(colony, election_id, cat_id, ctx)
        }),
        proto::ClientAction::RequestVoteKick { .. } => {
            with_colony(world, ctx, |colony| request_vote_kick(colony, ctx))
        }
        proto::ClientAction::CreateZone {
            kind,
            a,
            b,
            duration_ms,
            ..
        } => with_colony(world, ctx, |colony| {
            create_zone(colony, *kind, *a, *b, *duration_ms, ctx)
        }),
        proto::ClientAction::RemoveZone { zone_id, .. } => {
            with_colony(world, ctx, |colony| remove_zone(colony, zone_id, ctx))
        }
        proto::ClientAction::PlanBuilding { building_type, .. } => {
            with_colony(world, ctx, |colony| {
                plan_building(colony, *building_type, ctx)
            })
        }
        proto::ClientAction::UnlockNode { node_id, .. } => {
            with_colony(world, ctx, |colony| unlock_node(colony, node_id, ctx))
        }
        proto::ClientAction::AssignWorker {
            cat_id,
            building_id,
            ..
        } => with_colony(world, ctx, |colony| {
            assign_worker(colony, cat_id, building_id.as_deref(), ctx)
        }),
        proto::ClientAction::TrainWarrior { cat_id, .. } => with_colony(world, ctx, |colony| {
            train_warrior(colony, cat_id.as_deref(), ctx)
        }),
        proto::ClientAction::DefendRaid { .. } => {
            with_colony(world, ctx, |colony| defend_raid(colony, ctx))
        }
        proto::ClientAction::BuildRoad { a, b, .. } => {
            with_colony(world, ctx, |colony| build_road(colony, *a, *b, ctx))
        }
        proto::ClientAction::SetTestAcceleration { preset } => {
            set_test_acceleration(world, *preset);
            ok()
        }
        proto::ClientAction::AdvanceTime { seconds } => {
            if *seconds < 1 || *seconds > MAX_ADVANCE_SECONDS {
                return fail(format!(
                    "Invalid seconds (must be 1..{MAX_ADVANCE_SECONDS})."
                ));
            }
            let _ = world_tick(world, ctx.now_ms + (*seconds as i64 * 1000));
            ok()
        }
        proto::ClientAction::SetTestRngSeed { seed } => {
            for colony in &mut world.colonies {
                colony.test_rng_seed = *seed;
            }
            ok()
        }
        proto::ClientAction::FoundVillage { name, session_id } => {
            found_village(world, name, session_id, ctx)
        }
        proto::ClientAction::JoinVillage { colony_id, .. } => {
            if world.colonies.iter().any(|colony| colony.id == *colony_id) {
                ok()
            } else {
                fail("Village not found.")
            }
        }
    }
}

/// Build the wire snapshot consumed by clients.
#[must_use]
pub fn build_snapshot(world: &WorldState, now_ms: i64, online_count: u32) -> proto::WorldSnapshot {
    proto::WorldSnapshot {
        now: now_ms,
        world_seed: i64::from(world.world_seed),
        online_count,
        colonies: world
            .colonies
            .iter()
            .map(|colony| colony_snapshot(colony, now_ms))
            .collect(),
    }
}

fn with_colony(
    world: &mut WorldState,
    ctx: &ActionCtx,
    f: impl FnOnce(&mut ColonyRuntime) -> proto::ActionResult,
) -> proto::ActionResult {
    ensure_colony(world, ctx.now_ms);
    f(&mut world.colonies[0])
}

fn ensure_colony(world: &mut WorldState, now_ms: i64) {
    if world.colonies.is_empty() {
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", now_ms, 1));
    }
}

fn request_job(colony: &mut ColonyRuntime, kind: JobKind, ctx: &ActionCtx) -> proto::ActionResult {
    if !matches!(
        kind,
        JobKind::SupplyFood
            | JobKind::SupplyWater
            | JobKind::LeaderPlanHunt
            | JobKind::LeaderPlanHouse
            | JobKind::Ritual
    ) {
        return fail("Unknown job kind.");
    }

    colony.last_player_activity_at = Some(ctx.now_ms);

    if !matches!(kind, JobKind::SupplyFood | JobKind::SupplyWater) {
        let jobs = active_or_queued_minimal_jobs(colony);
        if idle_rules::has_conflicting_strategic_job(kind, &jobs) {
            return fail("That request is already in progress.");
        }
        if kind == JobKind::Ritual
            && (idle_rules::ritual_request_is_fresh(colony.ritual_requested_at, ctx.now_ms)
                || jobs.iter().any(|job| job.kind == JobKind::Ritual))
        {
            return fail("Ritual request already pending or active.");
        }
    }

    if kind == JobKind::Ritual {
        colony.ritual_requested_at = Some(ctx.now_ms);
        append_event(
            colony,
            ctx.now_ms,
            EventKind::Other("ritual_ready".to_owned()),
            "A ritual was requested.",
        );
        return ok();
    }

    queue_job(
        colony,
        ctx.now_ms,
        kind,
        JobRequester::Player,
        None,
        JobMetadata::None,
    );
    ok()
}

fn boost_job(colony: &mut ColonyRuntime, job_id: &str, ctx: &ActionCtx) -> proto::ActionResult {
    let Some(job) = colony.jobs.iter_mut().find(|job| job.id == job_id) else {
        return fail("This job cannot be boosted.");
    };
    if !matches!(job.status, JobStatus::Active | JobStatus::Queued) {
        return fail("This job cannot be boosted.");
    }

    let reduce_seconds = idle_engine::apply_click_boost_seconds(
        f64::from(job.click_count + 1),
        f64::from(colony.upgrade_levels.click_power),
    );
    let Some(ends_at) = job.ends_at else {
        return fail("This job cannot be boosted.");
    };

    let min_end = ctx.now_ms + 5_000;
    job.ends_at = Some(min_end.max(ends_at - (reduce_seconds * 1000.0) as i64));
    job.click_count = job.click_count.saturating_add(1);
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

fn purchase_upgrade(
    colony: &mut ColonyRuntime,
    key: UpgradeKey,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    let Some((_, max_level, base_cost)) = UPGRADE_DEFAULTS
        .iter()
        .find(|(candidate, _, _)| *candidate == key)
    else {
        return fail("Upgrade not found.");
    };
    let level = colony.upgrade_levels.get(key);
    if level >= *max_level {
        return fail("Upgrade already maxed.");
    }

    let cost = idle_engine::get_upgrade_cost(f64::from(*base_cost), f64::from(level));
    if colony.global_upgrade_points < cost {
        return fail("Not enough ritual points.");
    }

    set_upgrade_level(&mut colony.upgrade_levels, key, level + 1);
    colony.global_upgrade_points -= cost;
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

fn cast_vote(
    colony: &mut ColonyRuntime,
    election_id: &str,
    cat_id: &str,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    let Some(election) = colony
        .elections
        .iter()
        .find(|election| election.id == election_id && election.resolved_at.is_none())
    else {
        return fail("Election not found.");
    };
    if ctx.now_ms >= election.closes_at {
        return fail("Election is closed.");
    }
    if !colony
        .cats
        .iter()
        .any(|cat| cat.id == cat_id && cat.death_time.is_none())
    {
        return fail("Candidate not found.");
    }

    let voter_id = voter_id(ctx);
    if let Some(vote) = colony
        .votes
        .iter_mut()
        .find(|vote| vote.election_id == election_id && vote.voter_id == voter_id)
    {
        vote.cat_id = cat_id.to_owned();
    } else {
        colony.votes.push(VoteRuntime {
            id: format!("vote-{}-{}", ctx.now_ms, colony.votes.len() + 1),
            election_id: election_id.to_owned(),
            voter_id,
            cat_id: cat_id.to_owned(),
            weight: 1.0,
        });
    }
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

fn request_vote_kick(colony: &mut ColonyRuntime, ctx: &ActionCtx) -> proto::ActionResult {
    if colony.leader_id.is_none() {
        return fail("No leader to remove.");
    }
    if colony
        .elections
        .iter()
        .any(|election| election.kind == ElectionKind::VoteKick && election.resolved_at.is_none())
    {
        return fail("Vote-kick already pending.");
    }

    let election_id = format!("kick-{}-{}", ctx.now_ms, colony.elections.len() + 1);
    colony.elections.push(ElectionRuntime {
        id: election_id.clone(),
        opened_at: ctx.now_ms,
        closes_at: ctx.now_ms + crate::elections::KICK_WINDOW_MS as i64,
        resolved_at: None,
        winner_cat_id: colony.leader_id.clone(),
        kind: ElectionKind::VoteKick,
    });
    colony.votes.push(VoteRuntime {
        id: format!("vote-{}-{}", ctx.now_ms, colony.votes.len() + 1),
        election_id,
        voter_id: voter_id(ctx),
        cat_id: colony.leader_id.clone().unwrap_or_default(),
        weight: 1.0,
    });
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

fn create_zone(
    colony: &mut ColonyRuntime,
    kind: proto::ZoneKind,
    a: proto::TilePoint,
    b: proto::TilePoint,
    duration_ms: u64,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    let duration_ms = i64::try_from(duration_ms).unwrap_or(i64::MAX);
    let rect = zones::normalize_rect(
        f64::from(a.x),
        f64::from(a.y),
        f64::from(b.x),
        f64::from(b.y),
    );
    let player_id = stable_player_u64(ctx);
    let active_player_zones = colony
        .zones
        .iter()
        .filter(|zone| zone.expires_at > ctx.now_ms && zone.player_id == Some(player_id))
        .count() as u32;

    if let Some(message) = zones::validate_zone(rect, duration_ms, active_player_zones) {
        return fail(message);
    }

    colony.zones.push(ZoneRuntime {
        rect,
        kind: proto_to_sim_zone_kind(kind),
        created_at: ctx.now_ms,
        expires_at: ctx.now_ms + duration_ms,
        player_id: Some(player_id),
    });
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

fn remove_zone(colony: &mut ColonyRuntime, zone_id: &str, ctx: &ActionCtx) -> proto::ActionResult {
    let Some(index) = parse_zone_index(zone_id) else {
        return fail("Zone not found.");
    };
    if index >= colony.zones.len() {
        return fail("Zone not found.");
    }
    colony.zones.remove(index);
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

fn plan_building(
    colony: &mut ColonyRuntime,
    building_type: proto::BuildingType,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    let Some(building_type) = proto_to_sim_building_type(building_type) else {
        return fail("That building is not supported by the simulation runtime yet.");
    };
    if !matches!(
        building_type,
        BuildingType::Workshop
            | BuildingType::Field
            | BuildingType::Smithy
            | BuildingType::Barracks
            | BuildingType::FoodStorage
            | BuildingType::Den
    ) {
        return fail("Unknown building type.");
    }

    let village_level = village_level(colony);
    if !production::building_unlocked(building_type, village_level) {
        return fail(match building_type {
            BuildingType::Workshop => "Workshops unlock at village level 2.",
            BuildingType::Field => "Fields unlock at village level 4.",
            _ => "Building is locked.",
        });
    }
    if matches!(building_type, BuildingType::Smithy | BuildingType::Barracks)
        && !upgrade_tree::is_owned(&colony.upgrade_tree, building_type.as_str())
    {
        return fail("That building must be researched or granted by the gods first.");
    }
    if active_or_queued_jobs(colony)
        .iter()
        .any(|job| job.kind == JobKind::BuildHouse && job_building_type(job) == Some(building_type))
    {
        return fail("That request is already in progress.");
    }

    let Some(architect) = select_best_cat(colony, Some(CatSpecialization::Architect)) else {
        return fail("No available worker.");
    };
    queue_job(
        colony,
        ctx.now_ms,
        JobKind::BuildHouse,
        JobRequester::Player,
        Some(architect),
        JobMetadata::Construction {
            phase: ConstructionPhase::ConstructHouse,
            building_type,
            building_id: None,
            site: None,
        },
    );
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

fn unlock_node(colony: &mut ColonyRuntime, node_id: &str, ctx: &ActionCtx) -> proto::ActionResult {
    let result = upgrade_tree::god_purchase(&colony.upgrade_tree, node_id);
    if !result.ok {
        return fail(format!("{:?}", result.reason));
    }
    if colony.global_upgrade_points < result.blessings_cost {
        return fail("insufficient-blessings");
    }
    colony.upgrade_tree = result.state;
    colony.global_upgrade_points -= result.blessings_cost;
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

fn assign_worker(
    colony: &mut ColonyRuntime,
    cat_id: &str,
    building_id: Option<&str>,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    let Some(cat_index) = colony
        .cats
        .iter()
        .position(|cat| cat.id == cat_id && cat.death_time.is_none())
    else {
        return fail("That cat is not available.");
    };

    if building_id.is_none() {
        for building in &mut colony.buildings {
            if building.assigned_cat.as_deref() == Some(cat_id) {
                building.assigned_cat = None;
            }
        }
        colony.last_player_activity_at = Some(ctx.now_ms);
        return ok();
    }

    if !cat_can_take_assignment(colony, cat_index) {
        return fail("That cat is busy.");
    }

    let building_id = building_id.unwrap_or_default();
    let Some(building_index) = colony.buildings.iter().position(|building| {
        building.id == building_id
            && building.construction_progress >= 100
            && matches!(
                building.building_type,
                BuildingType::Workshop | BuildingType::Smithy
            )
    }) else {
        return fail("That building cannot take a worker.");
    };

    for building in &mut colony.buildings {
        if building.assigned_cat.as_deref() == Some(cat_id) {
            building.assigned_cat = None;
        }
    }
    colony.buildings[building_index].assigned_cat = Some(cat_id.to_owned());
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

fn train_warrior(
    colony: &mut ColonyRuntime,
    cat_id: Option<&str>,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    if !has_complete_building(colony, BuildingType::Barracks) {
        return fail("no_barracks");
    }

    let busy = busy_cat_ids(colony);
    let eligible = |cat: &Cat| {
        cat.death_time.is_none()
            && can_work(get_life_stage(cat.age_hours))
            && cat.specialization != Some(CatSpecialization::Warrior)
            && !busy.contains(cat.id.as_str())
    };

    let recruit = if let Some(cat_id) = cat_id {
        let Some(cat) = colony.cats.iter().find(|cat| cat.id == cat_id) else {
            return fail("ineligible");
        };
        if !eligible(cat) {
            return fail("ineligible");
        }
        cat.id.clone()
    } else {
        let Some(cat) = colony
            .cats
            .iter()
            .filter(|cat| eligible(cat))
            .max_by(|a, b| {
                (a.stats.attack + a.stats.defense).total_cmp(&(b.stats.attack + b.stats.defense))
            })
        else {
            return fail("no_recruit");
        };
        cat.id.clone()
    };

    queue_job(
        colony,
        ctx.now_ms,
        JobKind::TrainWarrior,
        JobRequester::Player,
        Some(recruit),
        JobMetadata::None,
    );
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

fn defend_raid(colony: &mut ColonyRuntime, ctx: &ActionCtx) -> proto::ActionResult {
    let Some(active_raid) = colony.active_raid.clone() else {
        return fail("no_raid");
    };
    let gate = raid_gate_position(colony);
    let Some(target_index) = colony
        .raiders
        .iter()
        .enumerate()
        .filter(|(_, raider)| raider.raid_id == active_raid && raider.health > 0.0)
        .min_by(|(_, a), (_, b)| distance_to_gate(a, gate).total_cmp(&distance_to_gate(b, gate)))
        .map(|(index, _)| index)
    else {
        return fail("no_raid");
    };

    colony.raiders[target_index].health =
        (colony.raiders[target_index].health - DEFEND_CLICK_DAMAGE).max(0.0);
    colony.raid_clicks += 1.0;
    colony.last_player_activity_at = Some(ctx.now_ms);
    if !colony
        .raiders
        .iter()
        .any(|raider| raider.raid_id == active_raid && raider.health > 0.0)
    {
        colony.active_raid = None;
    }
    ok()
}

fn build_road(
    colony: &mut ColonyRuntime,
    a: proto::TilePoint,
    b: proto::TilePoint,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    if [a.x, a.y, b.x, b.y].iter().any(|coord| coord.abs() > 1_000) {
        return fail("Invalid road endpoints.");
    }
    let distance = (b.x - a.x).abs() + (b.y - a.y).abs();
    if distance > 24 {
        return fail("Roads are limited to 24 tiles per build.");
    }

    let path = road_path(a, b);
    if path.len() > 24 {
        return fail("Roads are limited to 24 tiles per build.");
    }
    if colony.resources.materials < path.len() as f64 {
        return fail(format!(
            "Not enough materials ({} needed, one per tile).",
            path.len()
        ));
    }

    let mut paved = 0u32;
    for pos in path {
        let Some(tile) = colony.world_tiles.get_mut(&pos) else {
            continue;
        };
        if tile.tile_type == types::TileType::River
            || tile.overlay_feature.as_deref() == Some("river")
        {
            continue;
        }
        tile.overlay_feature = Some("road_built".to_owned());
        tile.path_wear = 100;
        paved += 1;
    }
    colony.resources.materials -= f64::from(paved);
    colony.last_player_activity_at = Some(ctx.now_ms);
    append_event(
        colony,
        ctx.now_ms,
        EventKind::Other("road_built".to_owned()),
        format!("A paved road was laid ({paved} tiles)."),
    );
    ok()
}

fn found_village(
    world: &mut WorldState,
    name: &str,
    session_id: &str,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    let name = name.trim();
    if name.is_empty() {
        return fail("Village name is required.");
    }
    let id = next_colony_id(world);
    let seed = stable_seed(&[session_id, name, &id]);
    let mut colony = found_colony(world.world_seed, id, ctx.now_ms, seed);
    colony.name = name.to_owned();
    world.colonies.push(colony);
    ok()
}

fn set_test_acceleration(world: &mut WorldState, preset: proto::AccelerationPreset) {
    let preset = match preset {
        proto::AccelerationPreset::Off => crate::test_acceleration::TestAccelerationPreset::Off,
        proto::AccelerationPreset::Fast => crate::test_acceleration::TestAccelerationPreset::Fast,
        proto::AccelerationPreset::Turbo => crate::test_acceleration::TestAccelerationPreset::Turbo,
        proto::AccelerationPreset::Hyper => crate::test_acceleration::TestAccelerationPreset::Hyper,
        proto::AccelerationPreset::Ludicrous => {
            crate::test_acceleration::TestAccelerationPreset::Ludicrous
        }
    };
    let config = crate::test_acceleration::config_for_preset(preset);
    for colony in &mut world.colonies {
        colony.test_time_scale = config.time_scale;
        colony.test_resource_decay_multiplier = config.resource_decay_multiplier;
        colony.test_resilience_hours_override = config.resilience_hours_override;
        colony.test_critical_ms_override = i64::from(config.critical_ms_override);
    }
}

fn colony_snapshot(colony: &ColonyRuntime, now_ms: i64) -> proto::ColonySnapshot {
    let alive_cats = alive_cats_sorted(colony);
    let effects = upgrade_tree::resolve_effects(colony.upgrade_tree.owned_node_ids.iter());
    let storage_buildings = storage_buildings(colony);
    let caps = storage::storage_capacities(&storage_buildings, effects.storage_per_level_mult);
    let housing_buildings = housing_buildings(colony);
    let housing_capacity = housing::housing_capacity(&housing_buildings, effects.housing_per_den);
    let population = alive_cats.len() as u32;
    let election_payload = election_snapshot(colony, &alive_cats);
    let vote_kick_payload = vote_kick_snapshot(colony, &alive_cats);
    let warrior_count = alive_cats
        .iter()
        .filter(|cat| cat.specialization == Some(CatSpecialization::Warrior))
        .count() as u32;

    proto::ColonySnapshot {
        id: colony.id.clone(),
        name: colony.name.clone(),
        status: sim_to_proto_colony_status(colony.status),
        resources: resources_snapshot(&colony.resources),
        storage: proto::StorageSnapshot {
            capacities: proto::ResourceCapacities {
                food: caps.food,
                water: caps.water,
                herbs: caps.herbs,
                materials: caps.materials,
                refined: caps.refined,
                weapons: caps.weapons,
                armor: caps.armor,
            },
            food_capacity: Some(caps.food),
            tithe_rates: proto::TitheRates {
                food: 20.0,
                refined: 5.0,
            },
        },
        leader: leader_snapshot(colony, &alive_cats),
        cats: alive_cats
            .iter()
            .map(|cat| cat_snapshot(colony, cat))
            .collect(),
        jobs: jobs_snapshot(colony),
        upgrades: upgrades_snapshot(colony),
        events: events_snapshot(colony),
        housing: proto::HousingSnapshot {
            population,
            capacity: housing_capacity as u32,
            pressure: housing::housing_pressure(f64::from(population), housing_capacity),
            village_level: housing::village_level(&housing_buildings),
        },
        research: research_snapshot(colony),
        election: election_payload,
        vote_kick: vote_kick_payload,
        zones: zones_snapshot(colony, now_ms),
        threat: proto::ThreatSnapshot {
            pressure: colony.threat_pressure,
            band: sim_to_proto_threat_band(threat::threat_band(colony.threat_pressure)),
            raid_active: colony.active_raid.is_some(),
            warriors: warrior_count,
            weapons: colony.resources.weapons,
            armor: colony.resources.armor,
        },
        raiders: raiders_snapshot(colony),
        buildings: buildings_snapshot(colony),
        claimed_tiles: colony.claimed_tiles.iter().map(tile_point).collect(),
        village_gate: village_gate_snapshot(colony),
        village_radius: village_ring_radius(colony.buildings.len() as i32) as u32,
        anchor: proto::TilePoint {
            x: VILLAGE_ANCHOR.x,
            y: VILLAGE_ANCHOR.y,
        },
    }
}

fn cat_snapshot(colony: &ColonyRuntime, cat: &Cat) -> proto::CatSnapshot {
    proto::CatSnapshot {
        id: cat.id.clone(),
        name: cat.name.clone(),
        position: map_position(cat.position),
        activity: sim_to_proto_activity(cat.activity),
        destination: cat.destination.map(map_position),
        carrying: cat.carrying.as_ref().map(|carrying| proto::Carrying {
            kind: sim_to_proto_carrying_kind(carrying.kind),
            amount: carrying.amount,
            job_ended_at: carrying.job_ended_at,
        }),
        specialization: cat.specialization.map(sim_to_proto_specialization),
        age_hours: cat.age_hours,
        needs: proto::CatNeeds {
            hunger: cat.needs.hunger,
            thirst: cat.needs.thirst,
            rest: cat.needs.rest,
            health: cat.needs.health,
        },
        current_task: cat.current_task.map(|task| task.as_str().to_owned()),
        assigned_building_id: colony
            .buildings
            .iter()
            .find(|building| building.assigned_cat.as_deref() == Some(cat.id.as_str()))
            .map(|building| building.id.clone()),
        role_xp: proto::RoleXp {
            hunter: cat.role_xp.hunter,
            architect: cat.role_xp.architect,
            ritualist: cat.role_xp.ritualist,
            warrior: cat.role_xp.warrior,
        },
        stats: proto::CatStats {
            leadership: cat.stats.leadership,
        },
        death_time: cat.death_time,
    }
}

fn jobs_snapshot(colony: &ColonyRuntime) -> Vec<proto::JobSnapshot> {
    let mut jobs = colony
        .jobs
        .iter()
        .filter(|job| matches!(job.status, JobStatus::Active | JobStatus::Queued))
        .collect::<Vec<_>>();
    jobs.sort_by_key(|job| job.ends_at.unwrap_or(i64::MAX));
    jobs.into_iter()
        .map(|job| proto::JobSnapshot {
            id: job.id.clone(),
            kind: sim_to_proto_job_kind(job.kind),
            status: sim_to_proto_job_status(job.status),
            ends_at: job.ends_at.unwrap_or(job.created_at + job.duration_ms),
            started_at: job.started_at.unwrap_or(job.created_at),
            click_time_reduced_sec: f64::from(job.click_count),
            assigned_cat_name: job
                .assigned_cat
                .as_ref()
                .and_then(|cat_id| colony.cats.iter().find(|cat| cat.id == *cat_id))
                .map(|cat| cat.name.clone()),
        })
        .collect()
}

fn upgrades_snapshot(colony: &ColonyRuntime) -> Vec<proto::UpgradeSnapshot> {
    UPGRADE_DEFAULTS
        .iter()
        .map(|(key, max_level, base_cost)| proto::UpgradeSnapshot {
            key: sim_to_proto_upgrade_key(*key),
            level: colony.upgrade_levels.get(*key),
            max_level: *max_level,
            base_cost: *base_cost,
        })
        .collect()
}

fn events_snapshot(colony: &ColonyRuntime) -> Vec<proto::EventSnapshot> {
    let mut events = colony.events.iter().collect::<Vec<_>>();
    events.sort_by_key(|event| Reverse(event.at_ms));
    events
        .into_iter()
        .take(EVENT_KEEP_SNAPSHOT)
        .map(|event| proto::EventSnapshot {
            message: event.message.clone(),
            timestamp: event.at_ms,
        })
        .collect()
}

fn research_snapshot(colony: &ColonyRuntime) -> proto::ResearchSnapshot {
    let next_target = upgrade_tree::next_research_target(&colony.upgrade_tree);
    proto::ResearchSnapshot {
        owned_node_ids: colony.upgrade_tree.owned_node_ids.clone(),
        research_points: colony.upgrade_tree.research_points,
        researcher_count: 0,
        blessings: colony.global_upgrade_points,
        next_target: next_target.map(|node| proto::ResearchTarget {
            id: node.id.to_owned(),
            name: node.name.to_owned(),
            cost: node.cost,
        }),
    }
}

fn election_snapshot(
    colony: &ColonyRuntime,
    alive_cats: &[&Cat],
) -> Option<proto::ElectionSnapshot> {
    let election = colony.elections.iter().find(|election| {
        election.kind == ElectionKind::Scheduled && election.resolved_at.is_none()
    })?;
    let candidates = election_candidates(alive_cats);
    let candidate_ids = crate::elections::candidates_for_unbarred(
        &candidates
            .iter()
            .map(|candidate| crate::elections::ElectionCandidate {
                id: candidate.id.clone(),
                leadership: candidate.leadership,
            })
            .collect::<Vec<_>>(),
    );
    let candidate_set = candidate_ids.iter().collect::<HashSet<_>>();
    let votes = colony
        .votes
        .iter()
        .filter(|vote| election.id == vote.election_id && candidate_set.contains(&vote.cat_id))
        .map(|vote| crate::elections::BallotVote {
            player_id: vote.voter_id.clone(),
            cat_id: vote.cat_id.clone(),
        })
        .collect::<Vec<_>>();
    let tally = crate::elections::tally_votes(&votes)
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    Some(proto::ElectionSnapshot {
        id: election.id.clone(),
        ends_at: election.closes_at,
        total_ballots: votes
            .iter()
            .map(|vote| vote.player_id.as_str())
            .collect::<HashSet<_>>()
            .len() as u32,
        tally,
        candidates: candidate_ids
            .into_iter()
            .filter_map(|id| {
                candidates
                    .iter()
                    .find(|candidate| candidate.id == id)
                    .cloned()
            })
            .collect(),
    })
}

fn vote_kick_snapshot(
    colony: &ColonyRuntime,
    alive_cats: &[&Cat],
) -> Option<proto::VoteKickSnapshot> {
    let election = colony.elections.iter().find(|election| {
        election.kind == ElectionKind::VoteKick && election.resolved_at.is_none()
    })?;
    let target_cat_id = election
        .winner_cat_id
        .clone()
        .or_else(|| colony.leader_id.clone())?;
    let signatures = colony
        .votes
        .iter()
        .filter(|vote| vote.election_id == election.id)
        .map(|vote| vote.voter_id.as_str())
        .collect::<HashSet<_>>()
        .len() as u32;
    Some(proto::VoteKickSnapshot {
        id: election.id.clone(),
        ends_at: election.closes_at,
        target_name: alive_cats
            .iter()
            .find(|cat| cat.id == target_cat_id)
            .map_or_else(|| "the leader".to_owned(), |cat| cat.name.clone()),
        target_cat_id,
        signatures,
        needed: KICK_THRESHOLD,
    })
}

fn zones_snapshot(colony: &ColonyRuntime, now_ms: i64) -> Vec<proto::ZoneSnapshot> {
    colony
        .zones
        .iter()
        .enumerate()
        .filter(|(_, zone)| zone.expires_at > now_ms)
        .map(|(index, zone)| proto::ZoneSnapshot {
            id: zone_id(index),
            kind: sim_to_proto_zone_kind(zone.kind),
            x1: zone.rect.x1,
            y1: zone.rect.y1,
            x2: zone.rect.x2,
            y2: zone.rect.y2,
            expires_at: zone.expires_at,
        })
        .collect()
}

fn raiders_snapshot(colony: &ColonyRuntime) -> Vec<proto::RaiderSnapshot> {
    let Some(active_raid) = colony.active_raid.as_deref() else {
        return Vec::new();
    };
    colony
        .raiders
        .iter()
        .filter(|raider| raider.raid_id == active_raid && raider.health > 0.0)
        .map(|raider| proto::RaiderSnapshot {
            id: raider.id.clone(),
            position: proto::TilePoint {
                x: raider.position.x.round() as i32,
                y: raider.position.y.round() as i32,
            },
            hp: raider.health,
            strength: raider.attack.max(raider.defense),
            status: if raider.health <= 0.0 {
                proto::RaiderStatus::Dead
            } else {
                proto::RaiderStatus::Advancing
            },
        })
        .collect()
}

fn buildings_snapshot(colony: &ColonyRuntime) -> Vec<proto::BuildingSnapshot> {
    colony
        .buildings
        .iter()
        .filter_map(|building| {
            let building_type = sim_to_proto_building_type(building.building_type)?;
            Some(proto::BuildingSnapshot {
                id: building.id.clone(),
                building_type,
                level: building.level,
                construction_progress: f64::from(building.construction_progress),
                world_position: tile_point(&building.position),
                position: tile_point(&building.position),
            })
        })
        .collect()
}

fn leader_snapshot(colony: &ColonyRuntime, alive_cats: &[&Cat]) -> Option<proto::LeaderSnapshot> {
    let leader_id = colony.leader_id.as_ref()?;
    alive_cats
        .iter()
        .find(|cat| cat.id == *leader_id)
        .map(|cat| proto::LeaderSnapshot {
            id: cat.id.clone(),
            name: cat.name.clone(),
            leadership: cat.stats.leadership,
        })
}

fn village_gate_snapshot(colony: &ColonyRuntime) -> Option<proto::GatePlacement> {
    let area = claimed_area(colony);
    gate_placement_default(&area).map(|gate| proto::GatePlacement {
        x: gate.x,
        y: gate.y,
        side: match gate.side {
            village_area::Side::N => proto::GateSide::N,
            village_area::Side::E => proto::GateSide::E,
            village_area::Side::S => proto::GateSide::S,
            village_area::Side::W => proto::GateSide::W,
        },
    })
}

fn queue_job(
    colony: &mut ColonyRuntime,
    now_ms: i64,
    kind: JobKind,
    requested_by: JobRequester,
    assigned_cat: Option<String>,
    metadata: JobMetadata,
) {
    let (specialization, skill) = assigned_cat
        .as_ref()
        .and_then(|cat_id| colony.cats.iter().find(|cat| cat.id == *cat_id))
        .map_or((None, 0.0), |cat| {
            (
                cat.specialization,
                Labor::for_job_kind(kind).map_or(0.0, |labor| cat.skill(labor)),
            )
        });
    let duration_seconds = idle_engine::get_scaled_duration_seconds(
        kind,
        specialization,
        idle_upgrade_levels(&colony.upgrade_levels),
        skill,
        Some(colony.test_time_scale),
    );
    let duration_ms = (duration_seconds * 1000.0) as i64;

    if let Some(cat_id) = assigned_cat.as_deref() {
        for building in &mut colony.buildings {
            if building.assigned_cat.as_deref() == Some(cat_id)
                && matches!(
                    building.building_type,
                    BuildingType::Workshop | BuildingType::Smithy
                )
            {
                building.assigned_cat = None;
            }
        }
        if let Some(cat) = colony.cats.iter_mut().find(|cat| cat.id == cat_id) {
            cat.current_task = task_for_job(kind);
            cat.activity = CatActivity::Idle;
            cat.destination = None;
        }
    }

    colony.jobs.push(JobRuntime {
        id: format!("job-{}-{}", now_ms, colony.jobs.len() + 1),
        kind,
        status: JobStatus::Queued,
        requested_by,
        assigned_cat,
        duration_ms,
        speed: 1.0,
        yield_amount: 1.0,
        click_count: 0,
        created_at: now_ms,
        started_at: Some(now_ms),
        ends_at: Some(now_ms + duration_ms),
        completed_at: None,
        metadata,
    });
    append_event(
        colony,
        now_ms,
        EventKind::JobQueued,
        format!("Queued {}", kind.as_str().replace('_', " ")),
    );
}

fn append_event(
    colony: &mut ColonyRuntime,
    now_ms: i64,
    kind: EventKind,
    message: impl Into<String>,
) {
    colony.events.push(EventLog {
        id: format!("event-{}-{}", now_ms, colony.events.len() + 1),
        at_ms: now_ms,
        kind,
        message: message.into(),
    });
}

fn active_or_queued_jobs(colony: &ColonyRuntime) -> Vec<&JobRuntime> {
    colony
        .jobs
        .iter()
        .filter(|job| matches!(job.status, JobStatus::Active | JobStatus::Queued))
        .collect()
}

fn active_or_queued_minimal_jobs(colony: &ColonyRuntime) -> Vec<idle_rules::MinimalJob> {
    active_or_queued_jobs(colony)
        .into_iter()
        .map(|job| idle_rules::MinimalJob { kind: job.kind })
        .collect()
}

fn job_building_type(job: &JobRuntime) -> Option<BuildingType> {
    match job.metadata {
        JobMetadata::Construction { building_type, .. } => Some(building_type),
        _ => None,
    }
}

fn select_best_cat(
    colony: &ColonyRuntime,
    specialization: Option<CatSpecialization>,
) -> Option<String> {
    let busy = busy_cat_ids(colony);
    let assigned = assigned_building_cat_ids(colony);
    let mut pool = colony
        .cats
        .iter()
        .filter(|cat| {
            cat.death_time.is_none()
                && can_work(get_life_stage(cat.age_hours))
                && !busy.contains(cat.id.as_str())
                && !assigned.contains(cat.id.as_str())
                && cat.activity == CatActivity::Idle
                && cat.current_task.is_none()
                && cat.carrying.is_none()
                && cat.destination.is_none()
        })
        .collect::<Vec<_>>();
    let preferred = pool
        .iter()
        .copied()
        .filter(|cat| specialization.is_some() && cat.specialization == specialization)
        .collect::<Vec<_>>();
    if !preferred.is_empty() {
        pool = preferred;
    }

    pool.into_iter()
        .max_by(|a, b| {
            specialization_stat(a, specialization)
                .total_cmp(&specialization_stat(b, specialization))
        })
        .map(|cat| cat.id.clone())
}

fn specialization_stat(cat: &Cat, specialization: Option<CatSpecialization>) -> f64 {
    match specialization {
        Some(CatSpecialization::Hunter) => cat.stats.hunting,
        Some(CatSpecialization::Architect) => cat.stats.building,
        Some(CatSpecialization::Ritualist) => cat.stats.leadership,
        Some(CatSpecialization::Warrior) => cat.stats.attack,
        None => cat.stats.leadership,
    }
}

fn cat_can_take_assignment(colony: &ColonyRuntime, cat_index: usize) -> bool {
    let cat = &colony.cats[cat_index];
    let busy = busy_cat_ids(colony);
    cat.activity == CatActivity::Idle
        && cat.current_task.is_none()
        && cat.carrying.is_none()
        && cat.destination.is_none()
        && !busy.contains(cat.id.as_str())
}

fn busy_cat_ids(colony: &ColonyRuntime) -> HashSet<&str> {
    colony
        .jobs
        .iter()
        .filter(|job| matches!(job.status, JobStatus::Active | JobStatus::Queued))
        .filter_map(|job| job.assigned_cat.as_deref())
        .collect()
}

fn assigned_building_cat_ids(colony: &ColonyRuntime) -> HashSet<&str> {
    colony
        .buildings
        .iter()
        .filter_map(|building| building.assigned_cat.as_deref())
        .collect()
}

fn has_complete_building(colony: &ColonyRuntime, building_type: BuildingType) -> bool {
    colony.buildings.iter().any(|building| {
        building.building_type == building_type && building.construction_progress >= 100
    })
}

fn raid_gate_position(colony: &ColonyRuntime) -> TilePos {
    TilePos {
        x: VILLAGE_ANCHOR.x,
        y: VILLAGE_ANCHOR.y + village_ring_radius(colony.buildings.len() as i32),
    }
}

fn distance_to_gate(raider: &RaiderRuntime, gate: TilePos) -> f64 {
    (raider.position.x - f64::from(gate.x))
        .abs()
        .max((raider.position.y - f64::from(gate.y)).abs())
}

fn road_path(a: proto::TilePoint, b: proto::TilePoint) -> Vec<TilePos> {
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

fn storage_buildings(colony: &ColonyRuntime) -> Vec<StorageBuilding> {
    colony
        .buildings
        .iter()
        .map(|building| {
            StorageBuilding::new(
                building.building_type,
                f64::from(building.construction_progress),
                Some(f64::from(building.level)),
            )
        })
        .collect()
}

fn housing_buildings(colony: &ColonyRuntime) -> Vec<HousingBuilding> {
    colony
        .buildings
        .iter()
        .map(|building| {
            HousingBuilding::new(
                building.building_type,
                f64::from(building.level),
                f64::from(building.construction_progress),
            )
        })
        .collect()
}

fn village_level(colony: &ColonyRuntime) -> u32 {
    housing::village_level(&housing_buildings(colony))
}

fn claimed_area(colony: &ColonyRuntime) -> village_area::VillageArea {
    let tiles = colony
        .claimed_tiles
        .iter()
        .map(|tile| GridPos {
            x: tile.x,
            y: tile.y,
        })
        .collect::<Vec<_>>();
    village_area::from_tiles(&tiles)
}

fn alive_cats_sorted(colony: &ColonyRuntime) -> Vec<&Cat> {
    let mut cats = colony
        .cats
        .iter()
        .filter(|cat| cat.death_time.is_none())
        .collect::<Vec<_>>();
    cats.sort_by(|a, b| b.stats.leadership.total_cmp(&a.stats.leadership));
    cats
}

fn election_candidates(alive_cats: &[&Cat]) -> Vec<proto::ElectionCandidate> {
    alive_cats
        .iter()
        .map(|cat| proto::ElectionCandidate {
            id: cat.id.clone(),
            name: cat.name.clone(),
            leadership: cat.stats.leadership,
            specialization: cat.specialization.map(sim_to_proto_specialization),
        })
        .collect()
}

fn resources_snapshot(resources: &entities::Resources) -> proto::ResourceAmounts {
    proto::ResourceAmounts {
        food: resources.food,
        water: resources.water,
        herbs: resources.herbs,
        materials: resources.materials,
        refined: resources.refined,
        weapons: resources.weapons,
        armor: resources.armor,
        blessings: resources.blessings,
    }
}

fn idle_upgrade_levels(upgrades: &crate::world_tick::UpgradeLevels) -> idle_engine::UpgradeLevels {
    idle_engine::UpgradeLevels {
        click_power: f64::from(upgrades.click_power),
        supply_speed: f64::from(upgrades.supply_speed),
        hunt_mastery: f64::from(upgrades.hunt_mastery),
        build_mastery: f64::from(upgrades.build_mastery),
        ritual_mastery: f64::from(upgrades.ritual_mastery),
        resilience: f64::from(upgrades.resilience),
    }
}

fn task_for_job(kind: JobKind) -> Option<TaskType> {
    match kind {
        JobKind::SupplyFood | JobKind::LeaderPlanHunt | JobKind::HuntExpedition => {
            Some(TaskType::Hunt)
        }
        JobKind::SupplyWater | JobKind::FetchWater => Some(TaskType::FetchWater),
        JobKind::LeaderPlanHouse | JobKind::BuildHouse | JobKind::Quarry => Some(TaskType::Build),
        JobKind::Ritual => Some(TaskType::Guard),
        JobKind::Explore => Some(TaskType::Explore),
        JobKind::TrainWarrior => Some(TaskType::Guard),
        JobKind::ExpandVillage => Some(TaskType::Patrol),
    }
}

fn map_position(position: Position) -> proto::MapPosition {
    proto::MapPosition {
        map: match position.map {
            MapType::Colony => proto::MapName::Colony,
            MapType::World => proto::MapName::World,
        },
        x: position.x.round() as i32,
        y: position.y.round() as i32,
    }
}

fn tile_point(pos: &TilePos) -> proto::TilePoint {
    proto::TilePoint { x: pos.x, y: pos.y }
}

fn zone_id(index: usize) -> String {
    format!("zone-{index}")
}

fn parse_zone_index(zone_id: &str) -> Option<usize> {
    zone_id.strip_prefix("zone-")?.parse().ok()
}

fn voter_id(ctx: &ActionCtx) -> String {
    if ctx.player_id.is_empty() {
        ctx.session_id.clone()
    } else {
        ctx.player_id.clone()
    }
}

fn stable_player_u64(ctx: &ActionCtx) -> u64 {
    stable_hash(if ctx.player_id.is_empty() {
        ctx.session_id.as_bytes()
    } else {
        ctx.player_id.as_bytes()
    })
}

fn stable_seed(parts: &[&str]) -> u32 {
    let mut hash = 2_166_136_261_u64;
    for part in parts {
        hash ^= stable_hash(part.as_bytes());
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    (hash as u32).max(1)
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 14_695_981_039_346_656_037_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash
}

fn next_colony_id(world: &WorldState) -> String {
    let mut next = world.colonies.len() + 1;
    loop {
        let id = format!("colony-{next}");
        if !world.colonies.iter().any(|colony| colony.id == id) {
            return id;
        }
        next += 1;
    }
}

fn set_upgrade_level(upgrades: &mut crate::world_tick::UpgradeLevels, key: UpgradeKey, level: u32) {
    match key {
        UpgradeKey::ClickPower => upgrades.click_power = level,
        UpgradeKey::SupplySpeed => upgrades.supply_speed = level,
        UpgradeKey::HuntMastery => upgrades.hunt_mastery = level,
        UpgradeKey::BuildMastery => upgrades.build_mastery = level,
        UpgradeKey::RitualMastery => upgrades.ritual_mastery = level,
        UpgradeKey::Resilience => upgrades.resilience = level,
    }
}

fn ok() -> proto::ActionResult {
    proto::ActionResult {
        ok: true,
        message: None,
    }
}

fn fail(message: impl Into<String>) -> proto::ActionResult {
    proto::ActionResult {
        ok: false,
        message: Some(message.into()),
    }
}

fn proto_to_sim_job_kind(kind: proto::JobKind) -> JobKind {
    match kind {
        proto::JobKind::SupplyFood => JobKind::SupplyFood,
        proto::JobKind::SupplyWater => JobKind::SupplyWater,
        proto::JobKind::LeaderPlanHunt => JobKind::LeaderPlanHunt,
        proto::JobKind::HuntExpedition => JobKind::HuntExpedition,
        proto::JobKind::LeaderPlanHouse => JobKind::LeaderPlanHouse,
        proto::JobKind::BuildHouse => JobKind::BuildHouse,
        proto::JobKind::Ritual => JobKind::Ritual,
        proto::JobKind::Quarry => JobKind::Quarry,
        proto::JobKind::Explore => JobKind::Explore,
        proto::JobKind::FetchWater => JobKind::FetchWater,
        proto::JobKind::TrainWarrior => JobKind::TrainWarrior,
        proto::JobKind::ExpandVillage => JobKind::ExpandVillage,
    }
}

fn sim_to_proto_job_kind(kind: JobKind) -> proto::JobKind {
    match kind {
        JobKind::SupplyFood => proto::JobKind::SupplyFood,
        JobKind::SupplyWater => proto::JobKind::SupplyWater,
        JobKind::LeaderPlanHunt => proto::JobKind::LeaderPlanHunt,
        JobKind::HuntExpedition => proto::JobKind::HuntExpedition,
        JobKind::LeaderPlanHouse => proto::JobKind::LeaderPlanHouse,
        JobKind::BuildHouse => proto::JobKind::BuildHouse,
        JobKind::Ritual => proto::JobKind::Ritual,
        JobKind::Quarry => proto::JobKind::Quarry,
        JobKind::Explore => proto::JobKind::Explore,
        JobKind::FetchWater => proto::JobKind::FetchWater,
        JobKind::TrainWarrior => proto::JobKind::TrainWarrior,
        JobKind::ExpandVillage => proto::JobKind::ExpandVillage,
    }
}

fn sim_to_proto_job_status(status: JobStatus) -> proto::JobStatus {
    match status {
        JobStatus::Queued => proto::JobStatus::Queued,
        JobStatus::Active => proto::JobStatus::Active,
        JobStatus::Completed => proto::JobStatus::Completed,
        JobStatus::Failed => proto::JobStatus::Failed,
        JobStatus::Cancelled => proto::JobStatus::Cancelled,
    }
}

fn proto_to_sim_upgrade_key(key: proto::UpgradeKey) -> UpgradeKey {
    match key {
        proto::UpgradeKey::ClickPower => UpgradeKey::ClickPower,
        proto::UpgradeKey::SupplySpeed => UpgradeKey::SupplySpeed,
        proto::UpgradeKey::HuntMastery => UpgradeKey::HuntMastery,
        proto::UpgradeKey::BuildMastery => UpgradeKey::BuildMastery,
        proto::UpgradeKey::RitualMastery => UpgradeKey::RitualMastery,
        proto::UpgradeKey::Resilience => UpgradeKey::Resilience,
    }
}

fn sim_to_proto_upgrade_key(key: UpgradeKey) -> proto::UpgradeKey {
    match key {
        UpgradeKey::ClickPower => proto::UpgradeKey::ClickPower,
        UpgradeKey::SupplySpeed => proto::UpgradeKey::SupplySpeed,
        UpgradeKey::HuntMastery => proto::UpgradeKey::HuntMastery,
        UpgradeKey::BuildMastery => proto::UpgradeKey::BuildMastery,
        UpgradeKey::RitualMastery => proto::UpgradeKey::RitualMastery,
        UpgradeKey::Resilience => proto::UpgradeKey::Resilience,
    }
}

fn proto_to_sim_building_type(building_type: proto::BuildingType) -> Option<BuildingType> {
    match building_type {
        proto::BuildingType::Den => Some(BuildingType::Den),
        proto::BuildingType::FoodStorage => Some(BuildingType::FoodStorage),
        proto::BuildingType::WaterBowl => Some(BuildingType::WaterBowl),
        proto::BuildingType::Beds => Some(BuildingType::Beds),
        proto::BuildingType::HerbGarden => Some(BuildingType::HerbGarden),
        proto::BuildingType::Nursery => Some(BuildingType::Nursery),
        proto::BuildingType::ElderCorner => Some(BuildingType::ElderCorner),
        proto::BuildingType::Walls => Some(BuildingType::Walls),
        proto::BuildingType::MouseFarm => Some(BuildingType::MouseFarm),
        proto::BuildingType::Shrine => Some(BuildingType::Shrine),
        proto::BuildingType::Workshop => Some(BuildingType::Workshop),
        proto::BuildingType::Field => Some(BuildingType::Field),
        proto::BuildingType::ResearchHut | proto::BuildingType::School => None,
        proto::BuildingType::Smithy => Some(BuildingType::Smithy),
        proto::BuildingType::Barracks => Some(BuildingType::Barracks),
    }
}

fn sim_to_proto_building_type(building_type: BuildingType) -> Option<proto::BuildingType> {
    match building_type {
        BuildingType::Den => Some(proto::BuildingType::Den),
        BuildingType::FoodStorage => Some(proto::BuildingType::FoodStorage),
        BuildingType::WaterBowl => Some(proto::BuildingType::WaterBowl),
        BuildingType::Beds => Some(proto::BuildingType::Beds),
        BuildingType::HerbGarden => Some(proto::BuildingType::HerbGarden),
        BuildingType::Nursery => Some(proto::BuildingType::Nursery),
        BuildingType::ElderCorner => Some(proto::BuildingType::ElderCorner),
        BuildingType::Walls => Some(proto::BuildingType::Walls),
        BuildingType::MouseFarm => Some(proto::BuildingType::MouseFarm),
        BuildingType::Shrine => Some(proto::BuildingType::Shrine),
        BuildingType::Workshop => Some(proto::BuildingType::Workshop),
        BuildingType::Field => Some(proto::BuildingType::Field),
        BuildingType::Smithy => Some(proto::BuildingType::Smithy),
        BuildingType::Barracks => Some(proto::BuildingType::Barracks),
    }
}

fn proto_to_sim_zone_kind(kind: proto::ZoneKind) -> zones::ZoneKind {
    match kind {
        proto::ZoneKind::Avoid => zones::ZoneKind::Avoid,
        proto::ZoneKind::Gather => zones::ZoneKind::Gather,
    }
}

fn sim_to_proto_zone_kind(kind: zones::ZoneKind) -> proto::ZoneKind {
    match kind {
        zones::ZoneKind::Avoid => proto::ZoneKind::Avoid,
        zones::ZoneKind::Gather => proto::ZoneKind::Gather,
    }
}

fn sim_to_proto_colony_status(status: entities::ColonyStatus) -> proto::ColonyStatus {
    match status {
        entities::ColonyStatus::Starting => proto::ColonyStatus::Starting,
        entities::ColonyStatus::Thriving => proto::ColonyStatus::Thriving,
        entities::ColonyStatus::Struggling => proto::ColonyStatus::Struggling,
        entities::ColonyStatus::Dead => proto::ColonyStatus::Dead,
    }
}

fn sim_to_proto_activity(activity: CatActivity) -> proto::CatActivity {
    match activity {
        CatActivity::Idle => proto::CatActivity::Idle,
        CatActivity::Traveling => proto::CatActivity::Traveling,
        CatActivity::Working => proto::CatActivity::Working,
        CatActivity::Returning => proto::CatActivity::Returning,
    }
}

fn sim_to_proto_specialization(specialization: CatSpecialization) -> proto::Specialization {
    match specialization {
        CatSpecialization::Hunter => proto::Specialization::Hunter,
        CatSpecialization::Architect => proto::Specialization::Architect,
        CatSpecialization::Ritualist => proto::Specialization::Ritualist,
        CatSpecialization::Warrior => proto::Specialization::Warrior,
    }
}

fn sim_to_proto_carrying_kind(kind: entities::CarryingKind) -> proto::CarryingKind {
    match kind {
        entities::CarryingKind::Food => proto::CarryingKind::Food,
        entities::CarryingKind::Blessings => proto::CarryingKind::Blessings,
        entities::CarryingKind::Materials => proto::CarryingKind::Materials,
        entities::CarryingKind::Water => proto::CarryingKind::Water,
    }
}

fn sim_to_proto_threat_band(band: threat::ThreatBand) -> proto::ThreatBand {
    match band {
        threat::ThreatBand::Calm => proto::ThreatBand::Calm,
        threat::ThreatBand::Rising => proto::ThreatBand::Rising,
        threat::ThreatBand::Imminent => proto::ThreatBand::Imminent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ActionCtx {
        ActionCtx {
            session_id: "sess_1".to_string(),
            player_id: "player_1".to_string(),
            now_ms: 1_000_000,
        }
    }

    fn world_with_one_colony() -> WorldState {
        WorldState {
            world_seed: 20_240_703,
            colonies: vec![found_colony(20_240_703, "c1", 1_000_000, 1234)],
        }
    }

    #[test]
    fn found_village_adds_a_colony_with_starter_cats() {
        let mut world = WorldState {
            world_seed: 20_240_703,
            colonies: Vec::new(),
        };
        let res = apply_action(
            &mut world,
            &proto::ClientAction::FoundVillage {
                name: "Newford".to_string(),
                session_id: "sess_1".to_string(),
            },
            &ctx(),
        );
        assert!(res.ok, "{res:?}");
        assert_eq!(world.colonies.len(), 1);
        assert!(
            !world.colonies[0].cats.is_empty(),
            "founded colony should have starter cats"
        );
    }

    #[test]
    fn purchase_upgrade_without_points_soft_fails_and_mutates_nothing() {
        let mut world = world_with_one_colony();
        world.colonies[0].global_upgrade_points = 0.0;
        let before = world.colonies[0].clone();
        let res = apply_action(
            &mut world,
            &proto::ClientAction::PurchaseUpgrade {
                session_id: "sess_1".to_string(),
                nickname: "Guest".to_string(),
                sig: "sig".to_string(),
                key: proto::UpgradeKey::ClickPower,
            },
            &ctx(),
        );
        assert!(!res.ok, "insufficient points should soft-fail");
        assert_eq!(
            world.colonies[0].global_upgrade_points,
            before.global_upgrade_points
        );
    }

    #[test]
    fn build_snapshot_maps_colony_resources_and_cats() {
        let world = world_with_one_colony();
        let snap = build_snapshot(&world, 1_000_000, 3);
        assert_eq!(snap.colonies.len(), 1);
        assert_eq!(snap.online_count, 3);
        let colony = &snap.colonies[0];
        assert_eq!(colony.cats.len(), world.colonies[0].cats.len());
        assert_eq!(colony.resources.food, world.colonies[0].resources.food);
    }
}
