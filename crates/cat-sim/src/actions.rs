//! Pure action application and snapshot building ported from
//! `app/api/game/actions/route.ts` and `server/game.ts:getGlobalDashboard`.

use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet, HashSet},
};

use cat_protocol as proto;

use crate::{
    entities::{self, Cat, CatActivity, MapType, Position},
    housing::{self, HousingBuilding},
    idle_engine, idle_rules,
    items::{Item, ItemKind, Material},
    life_sim::{can_work, get_life_stage},
    officers::OfficerRole,
    production,
    skills::Labor,
    stockpiles,
    storage::{self, StorageBuilding},
    threat, trader,
    types::{self, BuildingType, CatSpecialization, JobKind, JobStatus, TaskType, UpgradeKey},
    upgrade_tree,
    village_area::{self, gate_placement_default},
    village_layout::{GridPos, village_ring_radius},
    world_tick::{
        ColonyRuntime, ConstructionPhase, ElectionKind, ElectionRuntime, EventKind, EventLog,
        JobMetadata, JobRequester, JobRuntime, RaiderRuntime, TilePos, TradeDirection, VoteRuntime,
        WorldState, ZoneRuntime, found_colony, found_colony_at, reconcile_colony_stockpiles,
        select_founding_site, world_tick,
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
        proto::ClientAction::AssignOfficer { role, cat_id, .. } => {
            with_colony(world, ctx, |colony| {
                assign_officer(colony, proto_to_sim_officer_role(*role), cat_id, ctx)
            })
        }
        proto::ClientAction::UnassignOfficer { role, .. } => with_colony(world, ctx, |colony| {
            unassign_officer(colony, proto_to_sim_officer_role(*role), ctx)
        }),
        proto::ClientAction::DesignateStockpile { a, b, accepts, .. } => {
            with_colony(world, ctx, |colony| {
                designate_stockpile(colony, *a, *b, accepts, ctx)
            })
        }
        proto::ClientAction::RemoveStockpile { stockpile_id, .. } => {
            with_colony(world, ctx, |colony| {
                remove_stockpile(colony, stockpile_id, ctx)
            })
        }
        proto::ClientAction::DesignateGatherSpot { a, b, kind, .. } => {
            with_colony(world, ctx, |colony| {
                designate_gather_spot(colony, *a, *b, *kind, ctx)
            })
        }
        proto::ClientAction::RemoveGatherSpot { stockpile_id, .. } => {
            with_colony(world, ctx, |colony| {
                remove_gather_spot(colony, stockpile_id, ctx)
            })
        }
        proto::ClientAction::SellGoods {
            kind,
            material,
            quality,
            count,
            ..
        } => with_colony(world, ctx, |colony| {
            sell_goods(colony, kind, material, *quality, *count, ctx)
        }),
        proto::ClientAction::BuyResource {
            resource, amount, ..
        } => with_colony(world, ctx, |colony| {
            buy_resource(colony, *resource, *amount, ctx)
        }),
        proto::ClientAction::BoostCat {
            cat_id, boosted, ..
        } => with_colony(world, ctx, |colony| {
            boost_cat(colony, cat_id, *boosted, ctx)
        }),
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
            EventKind::RitualReady,
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
            | BuildingType::Smelter
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
    // P17/P19 ore→metal chain: the smelter's node id ("smelting") intentionally differs
    // from the building's own wire string ("smelter"), so it needs its own check rather
    // than reusing `building_type.as_str()` like the smithy/barracks gate above.
    if building_type == BuildingType::Smelter
        && !upgrade_tree::is_owned(&colony.upgrade_tree, upgrade_tree::SMELTING_NODE_ID)
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
    let node_name = upgrade_tree::get_node(node_id).map_or(node_id, |node| node.name);
    append_event(
        colony,
        ctx.now_ms,
        EventKind::NodeOwned,
        format!("The players blessed the village with {node_name}!"),
    );
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

/// Appoint `cat_id` to `role`. The cat must be alive and belong to this colony. A cat
/// holds at most one office, so this first vacates any office it already held; the role's
/// previous holder (if any) is replaced.
fn assign_officer(
    colony: &mut ColonyRuntime,
    role: OfficerRole,
    cat_id: &str,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    if !colony
        .cats
        .iter()
        .any(|cat| cat.id == cat_id && cat.death_time.is_none())
    {
        return fail("That cat is not available.");
    }

    colony
        .officers
        .retain(|_, holder| holder.as_str() != cat_id);
    colony.officers.insert(role, cat_id.to_owned());
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

/// Vacate `role`. A no-op (still `ok`) when the role is already empty.
fn unassign_officer(
    colony: &mut ColonyRuntime,
    role: OfficerRole,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    colony.officers.remove(&role);
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

/// Set/clear `cat_id`'s player priority flag (P15 "cat booster"). A no-op (still `ok`)
/// when the cat id is unknown or belongs to a dead cat — this is a player toggle on an
/// inspector panel, so an out-of-date/missing target should never surface as an error.
fn boost_cat(
    colony: &mut ColonyRuntime,
    cat_id: &str,
    boosted: bool,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    if let Some(cat) = colony
        .cats
        .iter_mut()
        .find(|cat| cat.id == cat_id && cat.death_time.is_none())
    {
        cat.boosted = boosted;
        colony.last_player_activity_at = Some(ctx.now_ms);
    }
    ok()
}

/// Designate a player stockpile over the rect `a..b` accepting `accepts`. Reuses the
/// zone edge cap; enforces a per-colony designated-pile limit and a non-empty accept set.
fn designate_stockpile(
    colony: &mut ColonyRuntime,
    a: proto::TilePoint,
    b: proto::TilePoint,
    accepts: &[proto::ResourceKind],
    ctx: &ActionCtx,
) -> proto::ActionResult {
    if accepts.is_empty() {
        return fail("A stockpile must accept at least one resource.");
    }
    let rect = zones::normalize_rect(
        f64::from(a.x),
        f64::from(a.y),
        f64::from(b.x),
        f64::from(b.y),
    );
    if rect.x2 - rect.x1 + 1 > stockpiles::STOCKPILE_MAX_EDGE
        || rect.y2 - rect.y1 + 1 > stockpiles::STOCKPILE_MAX_EDGE
    {
        return fail(format!(
            "Stockpiles are limited to {}x{} tiles.",
            stockpiles::STOCKPILE_MAX_EDGE,
            stockpiles::STOCKPILE_MAX_EDGE
        ));
    }
    let designated = colony
        .stockpiles
        .iter()
        .filter(|pile| !pile.is_shrine())
        .count();
    if designated >= stockpiles::MAX_DESIGNATED_STOCKPILES {
        return fail(format!(
            "You already have {} stockpiles.",
            stockpiles::MAX_DESIGNATED_STOCKPILES
        ));
    }

    let id = format!("stockpile-{}-{}", ctx.now_ms, colony.stockpiles.len() + 1);
    colony.stockpiles.push(stockpiles::Stockpile {
        id,
        rect,
        accepts: accepts
            .iter()
            .map(|kind| proto_to_sim_resource_kind(*kind))
            .collect(),
        contents: entities::Resources::default(),
    });
    reconcile_colony_stockpiles(colony);
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

/// Remove a designated stockpile by id. The shrine reservoir cannot be removed; an
/// unknown id is a no-op. Removed contents fold back into the reservoir via reconcile.
fn remove_stockpile(
    colony: &mut ColonyRuntime,
    stockpile_id: &str,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    if stockpile_id == stockpiles::SHRINE_STOCKPILE_ID {
        return fail("The shrine reservoir cannot be removed.");
    }
    colony
        .stockpiles
        .retain(|pile| pile.id != stockpile_id || pile.is_shrine());
    reconcile_colony_stockpiles(colony);
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

/// Designate a **gather spot** (P16): a temporary, single-resource pile, deliberately
/// smaller than a general stockpile (`GATHER_SPOT_MAX_EDGE`) and capped by its own
/// budget (`MAX_GATHER_SPOTS`, separate from the general designated-pile pool). May be
/// placed anywhere — including outside the claimed village, unlike a general
/// `DesignateStockpile`'s intent — since it reuses the same `Stockpile` machinery
/// unchanged: deposit routing/reconcile/capacity all apply exactly as for any other
/// pile. Only food/water/materials are accepted: the only resources a gatherer job can
/// currently carry (`entities::CarryingKind`).
fn designate_gather_spot(
    colony: &mut ColonyRuntime,
    a: proto::TilePoint,
    b: proto::TilePoint,
    kind: proto::ResourceKind,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    if !matches!(
        kind,
        proto::ResourceKind::Food | proto::ResourceKind::Water | proto::ResourceKind::Materials
    ) {
        return fail("Gather spots only collect food, water, or materials.");
    }
    let rect = zones::normalize_rect(
        f64::from(a.x),
        f64::from(a.y),
        f64::from(b.x),
        f64::from(b.y),
    );
    if rect.x2 - rect.x1 + 1 > stockpiles::GATHER_SPOT_MAX_EDGE
        || rect.y2 - rect.y1 + 1 > stockpiles::GATHER_SPOT_MAX_EDGE
    {
        return fail(format!(
            "Gather spots are limited to {}x{} tiles.",
            stockpiles::GATHER_SPOT_MAX_EDGE,
            stockpiles::GATHER_SPOT_MAX_EDGE
        ));
    }
    if colony.gather_spots.len() >= stockpiles::MAX_GATHER_SPOTS {
        return fail(format!(
            "You already have {} gather spots.",
            stockpiles::MAX_GATHER_SPOTS
        ));
    }

    let sim_kind = proto_to_sim_resource_kind(kind);
    let id = format!("gather-{}-{}", ctx.now_ms, colony.stockpiles.len() + 1);
    colony.stockpiles.push(stockpiles::Stockpile {
        id: id.clone(),
        rect,
        accepts: std::iter::once(sim_kind).collect(),
        contents: entities::Resources::default(),
    });
    colony.gather_spots.push(stockpiles::GatherSpot {
        stockpile_id: id,
        kind: sim_kind,
        expires_at_ms: ctx.now_ms + stockpiles::GATHER_SPOT_TTL_MS,
    });
    reconcile_colony_stockpiles(colony);
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

/// Remove a gather spot by its stockpile id before its TTL. Any in-flight
/// `haul_gather_spot` mover job targeting it is cancelled and its cat freed (see
/// `world_tick::cancel_gather_haul_jobs_for_spot`), mirroring the P16 TTL-expiry
/// cleanup, so it never dangles waiting on a site that no longer exists. Remaining
/// contents fold back into the shrine reservoir via reconcile, exactly like
/// `remove_stockpile`.
fn remove_gather_spot(
    colony: &mut ColonyRuntime,
    stockpile_id: &str,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    if !colony
        .gather_spots
        .iter()
        .any(|spot| spot.stockpile_id == stockpile_id)
    {
        return fail("Unknown gather spot.");
    }
    colony
        .gather_spots
        .retain(|spot| spot.stockpile_id != stockpile_id);
    colony.stockpiles.retain(|pile| pile.id != stockpile_id);
    crate::world_tick::cancel_gather_haul_jobs_for_spot(colony, stockpile_id, ctx.now_ms);
    reconcile_colony_stockpiles(colony);
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

/// Sell `count` of the crafted-item stack (`kind`/`material`/`quality`) to the visiting
/// trader for coin (P19 slice 3). Only valid while a trader is present and `Trading`;
/// fails cleanly (store/coin untouched) on an unknown kind/material, a zero count, or
/// insufficient stock.
fn sell_goods(
    colony: &mut ColonyRuntime,
    kind: &str,
    material: &str,
    quality: u8,
    count: u32,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    let Some(trader_unit) = colony.trader.as_ref() else {
        return fail("no_trader");
    };
    if trader_unit.state != trader::TraderState::Trading {
        return fail("no_trader");
    }
    if count == 0 {
        return fail("Invalid count.");
    }
    let Some(item_kind) = ItemKind::from_str_label(kind) else {
        return fail("Unknown item kind.");
    };
    let Some(item_material) = Material::from_str_label(material) else {
        return fail("Unknown item material.");
    };
    let item = Item::new(item_kind, item_material, quality);
    if colony.items.get(&item).copied().unwrap_or(0) < count {
        return fail("Not enough goods.");
    }

    let payout = trader::trader_buy_price(item, count);
    let removed = colony.remove_item(item, count);
    debug_assert!(removed, "checked availability above");
    colony.coin += payout;
    colony.last_player_activity_at = Some(ctx.now_ms);
    append_event(
        colony,
        ctx.now_ms,
        EventKind::Trade(TradeDirection::Sell),
        format!("The leader sold {count} {material} {kind} to the trader for {payout} coin."),
    );
    ok()
}

/// Buy `amount` of `resource` from the visiting trader with coin (P19 slice 3). Only
/// valid while a trader is present, `Trading`, and stocks that resource kind; fails
/// cleanly (coin/resources untouched) on an invalid amount, an unstocked resource kind,
/// or insufficient coin.
fn buy_resource(
    colony: &mut ColonyRuntime,
    kind: proto::ResourceKind,
    amount: f64,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    let Some(trader_unit) = colony.trader.as_ref() else {
        return fail("no_trader");
    };
    if trader_unit.state != trader::TraderState::Trading {
        return fail("no_trader");
    }
    if !amount.is_finite() || amount <= 0.0 {
        return fail("Invalid amount.");
    }

    let resource_kind = proto_to_sim_resource_kind(kind);
    let Some(cost) = trader::trader_sell_price(resource_kind, amount) else {
        return fail("The trader doesn't stock that.");
    };
    if colony.coin < cost {
        return fail("Not enough coin.");
    }

    colony.coin -= cost;
    stockpiles::add_resource(&mut colony.resources, resource_kind, amount);
    colony.last_player_activity_at = Some(ctx.now_ms);
    append_event(
        colony,
        ctx.now_ms,
        EventKind::Trade(TradeDirection::Buy),
        format!(
            "The leader bought {amount} {} from the trader for {cost} coin.",
            format!("{kind:?}").to_ascii_lowercase()
        ),
    );
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
        EventKind::RoadBuilt,
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
    // Place the new village at a distinct, valid site far from every existing colony so two
    // settlements never stack on the same anchor. Deterministic (RNG-free) site search.
    let existing_anchors: Vec<TilePos> =
        world.colonies.iter().map(|colony| colony.anchor).collect();
    let anchor = select_founding_site(world.world_seed, &existing_anchors);
    let mut colony = found_colony_at(world.world_seed, id, ctx.now_ms, seed, anchor);
    colony.name = name.to_owned();
    append_event(
        &mut colony,
        ctx.now_ms,
        EventKind::VillageFounded,
        format!("{name} was founded."),
    );
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
                planks: caps.planks,
                blocks: caps.blocks,
                tools: caps.tools,
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
        revealed_tiles: colony.revealed_tiles.iter().map(tile_point).collect(),
        // Merge every out scout's tentative tiles into one deterministic, deduped,
        // sorted set (matching `revealed_tiles`'s `BTreeSet` ordering) rather than
        // exposing per-scout grouping the client doesn't need.
        provisional_tiles: colony
            .provisional_tiles
            .values()
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>()
            .iter()
            .map(tile_point)
            .collect(),
        road_tiles: colony
            .world_tiles
            .iter()
            .filter(|(_, tile)| tile.overlay_feature.as_deref() == Some("road_built"))
            .map(|(pos, _)| tile_point(pos))
            .collect(),
        village_gate: village_gate_snapshot(colony),
        village_radius: village_ring_radius(colony.buildings.len() as i32) as u32,
        anchor: proto::TilePoint {
            x: colony.anchor.x,
            y: colony.anchor.y,
        },
        officers: colony
            .officers
            .iter()
            .map(|(role, cat_id)| (sim_to_proto_officer_role(*role), cat_id.clone()))
            .collect(),
        stockpiles: colony
            .stockpiles
            .iter()
            .map(|pile| stockpile_snapshot(pile, &colony.gather_spots))
            .collect(),
        stock_ledger: Some(stock_ledger_snapshot(colony)),
        items: items_snapshot(&colony.items),
        coin: colony.coin,
        trader: trader_snapshot(colony),
    }
}

/// Builds the visiting-trader snapshot (P19 slice 3), or `None` when no trader is
/// present. Offers are only populated while `Trading` (selling/buying is only valid
/// then) — `Arriving`/`Departing` traders report an empty offer list.
fn trader_snapshot(colony: &ColonyRuntime) -> Option<proto::TraderSnapshot> {
    let trader_unit = colony.trader.as_ref()?;
    let is_trading = trader_unit.state == trader::TraderState::Trading;

    let buy_offers = if is_trading {
        colony
            .items
            .iter()
            .map(|(item, &count)| proto::TraderBuyOffer {
                kind: item.kind.as_str().to_owned(),
                material: item.material.as_str().to_owned(),
                quality: item.quality,
                available: count,
                unit_price: trader::trader_buy_price(*item, 1),
            })
            .collect()
    } else {
        Vec::new()
    };

    let sell_offers = if is_trading {
        stockpiles::ResourceKind::ALL
            .iter()
            .filter_map(|&kind| {
                trader::trader_sell_price(kind, 1.0).map(|unit_price| proto::TraderSellOffer {
                    resource: sim_to_proto_resource_kind(kind),
                    unit_price,
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    Some(proto::TraderSnapshot {
        id: trader_unit.id.clone(),
        position: proto::TilePoint {
            x: trader_unit.position.x.round() as i32,
            y: trader_unit.position.y.round() as i32,
        },
        state: sim_to_proto_trader_state(trader_unit.state),
        buy_offers,
        sell_offers,
    })
}

fn sim_to_proto_trader_state(state: trader::TraderState) -> proto::TraderVisitState {
    match state {
        trader::TraderState::Arriving => proto::TraderVisitState::Arriving,
        trader::TraderState::Trading => proto::TraderVisitState::Trading,
        trader::TraderState::Departing => proto::TraderVisitState::Departing,
    }
}

/// Builds the item-stack snapshot list from the colony's item store, in the store's
/// own (deterministic `BTreeMap`) order.
fn items_snapshot(items: &BTreeMap<Item, u32>) -> Vec<proto::ItemStackSnapshot> {
    items
        .iter()
        .map(|(item, &count)| proto::ItemStackSnapshot {
            kind: item.kind.as_str().to_owned(),
            material: item.material.as_str().to_owned(),
            quality: item.quality,
            count,
            value: item.value(),
        })
        .collect()
}

fn stock_ledger_snapshot(colony: &ColonyRuntime) -> proto::StockLedgerSnapshot {
    proto::StockLedgerSnapshot {
        reported: resources_snapshot(&colony.stock_ledger.reported),
        last_counted: colony.stock_ledger.last_counted,
        accurate: colony.stock_ledger.is_accurate(&colony.resources),
    }
}

fn stockpile_snapshot(
    pile: &stockpiles::Stockpile,
    gather_spots: &[stockpiles::GatherSpot],
) -> proto::StockpileSnapshot {
    proto::StockpileSnapshot {
        id: pile.id.clone(),
        x1: pile.rect.x1,
        y1: pile.rect.y1,
        x2: pile.rect.x2,
        y2: pile.rect.y2,
        accepts: pile
            .accepts
            .iter()
            .map(|kind| sim_to_proto_resource_kind(*kind))
            .collect(),
        contents: resources_snapshot(&pile.contents),
        gather_spot: gather_spots
            .iter()
            .find(|spot| spot.stockpile_id == pile.id)
            .map(|spot| proto::GatherSpotSnapshot {
                kind: sim_to_proto_resource_kind(spot.kind),
                expires_at_ms: spot.expires_at_ms,
            }),
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
        parent_ids: cat.parent_ids.iter().flatten().cloned().collect(),
        parents: cat
            .parent_ids
            .iter()
            .flatten()
            .filter_map(|parent_id| {
                colony
                    .cats
                    .iter()
                    .find(|candidate| candidate.id == *parent_id)
                    .map(|parent| parent.name.clone())
            })
            .collect(),
        boosted: cat.boosted,
        pregnant: cat.is_pregnant,
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
            kind: event.kind.wire_kind(),
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
            let (width, height) = crate::world_tick::footprint_for(building.building_type);
            // A cat only counts as staffing this building while it's still alive —
            // mirrors `world_tick::assigned_worker`, which the production phase itself
            // uses to decide whether a bench/smithy has a live worker this tick.
            let has_live_worker = building.assigned_cat.as_deref().is_some_and(|cat_id| {
                colony
                    .cats
                    .iter()
                    .any(|cat| cat.id == cat_id && cat.death_time.is_none())
            });
            let staff_cap = production::building_staff_cap(building.building_type);
            let production_progress = production::building_cycle_sec(building.building_type)
                .map_or(0.0, |cycle_sec| {
                    (building.production_progress / cycle_sec).clamp(0.0, 1.0)
                });
            Some(proto::BuildingSnapshot {
                id: building.id.clone(),
                building_type,
                level: building.level,
                construction_progress: f64::from(building.construction_progress),
                world_position: tile_point(&building.position),
                position: tile_point(&building.position),
                footprint: proto::FootprintSize { width, height },
                staff_count: u32::from(has_live_worker),
                staff_cap,
                production_progress,
                production_output: production::building_output_label(building.building_type)
                    .map(str::to_owned),
                // Live sum of carried cargo whose haul target resolves to this building's
                // tile (see `world_tick::building_inbound_haul`). Only ever nonzero for the
                // shrine today: every other building type draws its inputs straight from
                // `colony.resources`, never from physically delivered cargo.
                inbound_haul: crate::world_tick::building_inbound_haul(colony, building),
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
        x: colony.anchor.x,
        y: colony.anchor.y + village_ring_radius(colony.buildings.len() as i32),
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
        planks: resources.planks,
        blocks: resources.blocks,
        tools: resources.tools,
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
        JobKind::Ritual | JobKind::CarryOffering => Some(TaskType::Guard),
        JobKind::Explore => Some(TaskType::Explore),
        JobKind::TrainWarrior => Some(TaskType::Guard),
        JobKind::ExpandVillage => Some(TaskType::Patrol),
        JobKind::HaulGatherSpot => Some(TaskType::Build),
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

fn proto_to_sim_officer_role(role: proto::OfficerRole) -> OfficerRole {
    match role {
        proto::OfficerRole::Steward => OfficerRole::Steward,
        proto::OfficerRole::Forester => OfficerRole::Forester,
        proto::OfficerRole::Farmer => OfficerRole::Farmer,
        proto::OfficerRole::Captain => OfficerRole::Captain,
        proto::OfficerRole::Loremaster => OfficerRole::Loremaster,
    }
}

fn sim_to_proto_officer_role(role: OfficerRole) -> proto::OfficerRole {
    match role {
        OfficerRole::Steward => proto::OfficerRole::Steward,
        OfficerRole::Forester => proto::OfficerRole::Forester,
        OfficerRole::Farmer => proto::OfficerRole::Farmer,
        OfficerRole::Captain => proto::OfficerRole::Captain,
        OfficerRole::Loremaster => proto::OfficerRole::Loremaster,
    }
}

fn proto_to_sim_resource_kind(kind: proto::ResourceKind) -> stockpiles::ResourceKind {
    use stockpiles::ResourceKind;
    match kind {
        proto::ResourceKind::Food => ResourceKind::Food,
        proto::ResourceKind::Water => ResourceKind::Water,
        proto::ResourceKind::Herbs => ResourceKind::Herbs,
        proto::ResourceKind::Materials => ResourceKind::Materials,
        proto::ResourceKind::Refined => ResourceKind::Refined,
        proto::ResourceKind::Weapons => ResourceKind::Weapons,
        proto::ResourceKind::Armor => ResourceKind::Armor,
        proto::ResourceKind::Blessings => ResourceKind::Blessings,
    }
}

fn sim_to_proto_resource_kind(kind: stockpiles::ResourceKind) -> proto::ResourceKind {
    use stockpiles::ResourceKind;
    match kind {
        ResourceKind::Food => proto::ResourceKind::Food,
        ResourceKind::Water => proto::ResourceKind::Water,
        ResourceKind::Herbs => proto::ResourceKind::Herbs,
        ResourceKind::Materials => proto::ResourceKind::Materials,
        ResourceKind::Refined => proto::ResourceKind::Refined,
        ResourceKind::Weapons => proto::ResourceKind::Weapons,
        ResourceKind::Armor => proto::ResourceKind::Armor,
        ResourceKind::Blessings => proto::ResourceKind::Blessings,
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
        proto::JobKind::CarryOffering => JobKind::CarryOffering,
        proto::JobKind::HaulGatherSpot => JobKind::HaulGatherSpot,
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
        JobKind::CarryOffering => proto::JobKind::CarryOffering,
        JobKind::HaulGatherSpot => proto::JobKind::HaulGatherSpot,
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
        proto::BuildingType::ResearchHut => Some(BuildingType::ResearchHut),
        // School is a later-tree research building not yet ported to the sim runtime.
        proto::BuildingType::School => None,
        proto::BuildingType::Smithy => Some(BuildingType::Smithy),
        proto::BuildingType::Barracks => Some(BuildingType::Barracks),
        proto::BuildingType::WoodCutter => Some(BuildingType::WoodCutter),
        proto::BuildingType::StonePrep => Some(BuildingType::StonePrep),
        proto::BuildingType::Woodworking => Some(BuildingType::Woodworking),
        proto::BuildingType::Clothier => Some(BuildingType::Clothier),
        proto::BuildingType::Tannery => Some(BuildingType::Tannery),
        proto::BuildingType::Smelter => Some(BuildingType::Smelter),
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
        BuildingType::WoodCutter => Some(proto::BuildingType::WoodCutter),
        BuildingType::StonePrep => Some(proto::BuildingType::StonePrep),
        BuildingType::Woodworking => Some(proto::BuildingType::Woodworking),
        // No protocol/client sprite yet — the Accounting Tent's effect surfaces via the
        // stock ledger, not a rendered building. Omitted from the buildings snapshot.
        BuildingType::AccountingTent => None,
        BuildingType::Clothier => Some(proto::BuildingType::Clothier),
        BuildingType::Tannery => Some(proto::BuildingType::Tannery),
        BuildingType::ResearchHut => Some(proto::BuildingType::ResearchHut),
        // NOTE: cat-client's `building_texture`/`building_label` (exhaustive matches over
        // `proto::BuildingType`) do not have a Smelter sprite arm yet — flagged for
        // catclient3, see `crates/cat-protocol/src/lib.rs`'s `BuildingType::Smelter` doc.
        BuildingType::Smelter => Some(proto::BuildingType::Smelter),
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
    use crate::village_layout::VILLAGE_ANCHOR;
    use crate::world_tick::TraderRuntime;

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
    fn snapshot_exposes_the_refinement_tier_resources_and_caps() {
        // P19 slice 1b: planks/blocks/tools and their caps must ride the wire so the
        // client HUD can show the refinement stockpile.
        let mut world = world_with_one_colony();
        world.colonies[0].resources.planks = 7.0;
        world.colonies[0].resources.blocks = 3.5;
        world.colonies[0].resources.tools = 2.0;

        let snapshot = build_snapshot(&world, 1_000_000, 1);
        let colony = &snapshot.colonies[0];
        assert_eq!(colony.resources.planks, 7.0);
        assert_eq!(colony.resources.blocks, 3.5);
        assert_eq!(colony.resources.tools, 2.0);
        assert!(colony.storage.capacities.planks > 0.0);
        assert!(colony.storage.capacities.blocks > 0.0);
        assert!(colony.storage.capacities.tools > 0.0);
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

    fn idle_worker_cat(id: &str, colony_id: &str, name: &str) -> Cat {
        Cat {
            id: id.to_string(),
            colony_id: colony_id.to_string(),
            name: name.to_string(),
            parent_ids: vec![None, None],
            birth_time: 0,
            death_time: None,
            stats: entities::CatStats::default(),
            needs: entities::CatNeeds::default(),
            current_task: None,
            position: Position {
                map: MapType::Colony,
                x: 0.0,
                y: 0.0,
            },
            destination: None,
            carrying: None,
            activity: CatActivity::Working,
            is_pregnant: false,
            pregnancy_due_time: None,
            age_hours: 24.0,
            pregnancy_due_age_hours: None,
            pregnancy_mate_id: None,
            sprite_params: None,
            specialization: None,
            role_xp: Default::default(),
            skills: Default::default(),
            boosted: false,
        }
    }

    #[test]
    fn colony_snapshot_reports_staffing_and_progress_for_a_mid_craft_bench() {
        // A staffed workshop halfway through its 600s refinement cycle should show up
        // on the wire as ~1/1 staffed and 50% through the current cycle, making
        // "refined" (see production::building_output_label).
        let mut world = world_with_one_colony();
        let colony = &mut world.colonies[0];
        let worker_id = "worker-1".to_string();
        colony
            .cats
            .push(idle_worker_cat(&worker_id, &colony.id, "Juniper"));
        colony.buildings.push(crate::world_tick::BuildingRuntime {
            id: "building-workshop-test".to_string(),
            building_type: BuildingType::Workshop,
            level: 2,
            position: TilePos { x: 20, y: 20 },
            is_complete: true,
            construction_progress: 100,
            production_progress: 300.0,
            assigned_cat: Some(worker_id),
        });

        let snapshot = build_snapshot(&world, 1_000_000, 1);
        let building = snapshot.colonies[0]
            .buildings
            .iter()
            .find(|building| building.id == "building-workshop-test")
            .expect("workshop building present in snapshot");

        assert_eq!(building.staff_count, 1);
        assert!(building.staff_cap >= 1);
        assert!((0.0..=1.0).contains(&building.production_progress));
        assert_eq!(building.production_progress, 0.5);
        assert_eq!(building.production_output.as_deref(), Some("refined"));
    }

    #[test]
    fn colony_snapshot_reports_zero_progress_for_a_no_cycle_building() {
        // The founding shrine has no worker slot and no timed production cycle, so it
        // should report an idle 0/0 staffing line and 0.0 progress with no output.
        let world = world_with_one_colony();
        let snapshot = build_snapshot(&world, 1_000_000, 1);
        let shrine = snapshot.colonies[0]
            .buildings
            .iter()
            .find(|building| building.building_type == proto::BuildingType::Shrine)
            .expect("founding shrine present in snapshot");

        assert_eq!(shrine.staff_count, 0);
        assert_eq!(shrine.staff_cap, 0);
        assert_eq!(shrine.production_progress, 0.0);
        assert_eq!(shrine.production_output, None);
    }

    #[test]
    fn cat_snapshot_resolves_known_parent_ids_and_names() {
        let mut world = world_with_one_colony();
        let colony = &mut world.colonies[0];
        let mother_id = "mother-1".to_string();
        colony
            .cats
            .push(idle_worker_cat(&mother_id, &colony.id, "Willow"));
        let kitten_id = "kitten-1".to_string();
        let mut kitten = idle_worker_cat(&kitten_id, &colony.id, "Fern");
        // One known parent (the mother), one unknown slot (`None`, e.g. an
        // unrecorded/founding father) — matches `Cat::parent_ids`'s shape.
        kitten.parent_ids = vec![Some(mother_id.clone()), None];
        colony.cats.push(kitten);

        let snapshot = build_snapshot(&world, 1_000_000, 1);
        let kitten_snap = snapshot.colonies[0]
            .cats
            .iter()
            .find(|cat| cat.id == kitten_id)
            .expect("kitten present in snapshot");

        assert_eq!(kitten_snap.parent_ids, vec![mother_id]);
        assert_eq!(kitten_snap.parents, vec!["Willow".to_string()]);
    }

    #[test]
    fn cat_snapshot_reports_empty_lineage_for_founding_cats() {
        // Founding cats are created with `parent_ids: vec![None, None]`.
        let world = world_with_one_colony();
        let snapshot = build_snapshot(&world, 1_000_000, 1);
        for cat in &snapshot.colonies[0].cats {
            assert!(cat.parent_ids.is_empty(), "cat {} has no founders", cat.id);
            assert!(cat.parents.is_empty(), "cat {} has no founders", cat.id);
        }
    }

    // ---- P12.2 officer actions ----

    fn assign_officer_action(role: proto::OfficerRole, cat_id: &str) -> proto::ClientAction {
        proto::ClientAction::AssignOfficer {
            session_id: "sess_1".to_string(),
            nickname: "Guest".to_string(),
            sig: "sig".to_string(),
            role,
            cat_id: cat_id.to_string(),
        }
    }

    fn unassign_officer_action(role: proto::OfficerRole) -> proto::ClientAction {
        proto::ClientAction::UnassignOfficer {
            session_id: "sess_1".to_string(),
            nickname: "Guest".to_string(),
            sig: "sig".to_string(),
            role,
        }
    }

    #[test]
    fn assign_officer_appoints_and_enforces_one_office_per_cat() {
        let mut world = world_with_one_colony();
        let cat_id = world.colonies[0].cats[0].id.clone();

        let res = apply_action(
            &mut world,
            &assign_officer_action(proto::OfficerRole::Farmer, &cat_id),
            &ctx(),
        );
        assert!(res.ok, "{res:?}");
        assert_eq!(
            world.colonies[0].officers.get(&OfficerRole::Farmer),
            Some(&cat_id)
        );

        // Re-appointing the same cat to a different role vacates the first.
        let res = apply_action(
            &mut world,
            &assign_officer_action(proto::OfficerRole::Captain, &cat_id),
            &ctx(),
        );
        assert!(res.ok, "{res:?}");
        assert!(
            !world.colonies[0]
                .officers
                .contains_key(&OfficerRole::Farmer)
        );
        assert_eq!(
            world.colonies[0].officers.get(&OfficerRole::Captain),
            Some(&cat_id)
        );
        assert_eq!(world.colonies[0].officers.len(), 1);
    }

    #[test]
    fn assign_officer_rejects_foreign_or_dead_cat() {
        let mut world = world_with_one_colony();

        let res = apply_action(
            &mut world,
            &assign_officer_action(proto::OfficerRole::Steward, "ghost"),
            &ctx(),
        );
        assert!(!res.ok, "foreign cat should be rejected");
        assert!(world.colonies[0].officers.is_empty());

        world.colonies[0].cats[0].death_time = Some(1_000);
        let dead_id = world.colonies[0].cats[0].id.clone();
        let res = apply_action(
            &mut world,
            &assign_officer_action(proto::OfficerRole::Steward, &dead_id),
            &ctx(),
        );
        assert!(!res.ok, "dead cat should be rejected");
        assert!(world.colonies[0].officers.is_empty());
    }

    #[test]
    fn unassign_officer_clears_role_and_empty_is_noop() {
        let mut world = world_with_one_colony();

        // Unassigning an empty role is a no-op that still succeeds.
        let res = apply_action(
            &mut world,
            &unassign_officer_action(proto::OfficerRole::Loremaster),
            &ctx(),
        );
        assert!(res.ok, "{res:?}");
        assert!(world.colonies[0].officers.is_empty());

        let cat_id = world.colonies[0].cats[0].id.clone();
        let _ = apply_action(
            &mut world,
            &assign_officer_action(proto::OfficerRole::Captain, &cat_id),
            &ctx(),
        );
        let res = apply_action(
            &mut world,
            &unassign_officer_action(proto::OfficerRole::Captain),
            &ctx(),
        );
        assert!(res.ok, "{res:?}");
        assert!(world.colonies[0].officers.is_empty());
    }

    // ---- P15 cat booster ----

    fn boost_cat_action(cat_id: &str, boosted: bool) -> proto::ClientAction {
        proto::ClientAction::BoostCat {
            session_id: "sess_1".to_string(),
            nickname: "Guest".to_string(),
            sig: "sig".to_string(),
            cat_id: cat_id.to_string(),
            boosted,
        }
    }

    #[test]
    fn boost_cat_sets_and_clears_the_flag() {
        let mut world = world_with_one_colony();
        let cat_id = world.colonies[0].cats[0].id.clone();
        assert!(!world.colonies[0].cats[0].boosted);

        let res = apply_action(&mut world, &boost_cat_action(&cat_id, true), &ctx());
        assert!(res.ok, "{res:?}");
        assert!(world.colonies[0].cats[0].boosted);

        let res = apply_action(&mut world, &boost_cat_action(&cat_id, false), &ctx());
        assert!(res.ok, "{res:?}");
        assert!(!world.colonies[0].cats[0].boosted);
    }

    #[test]
    fn boost_cat_is_a_clean_noop_for_an_unknown_or_dead_cat_id() {
        let mut world = world_with_one_colony();
        let before = world.colonies[0].clone();

        let res = apply_action(&mut world, &boost_cat_action("no-such-cat", true), &ctx());
        assert!(res.ok, "unknown cat id should still be ok: {res:?}");
        assert_eq!(world.colonies[0], before);

        let dead_cat_id = world.colonies[0].cats[0].id.clone();
        world.colonies[0].cats[0].death_time = Some(1);
        let before_dead = world.colonies[0].clone();
        let res = apply_action(&mut world, &boost_cat_action(&dead_cat_id, true), &ctx());
        assert!(res.ok, "dead cat id should still be ok: {res:?}");
        assert_eq!(world.colonies[0], before_dead);
    }

    #[test]
    fn cat_snapshot_round_trips_the_boosted_flag() {
        let mut world = world_with_one_colony();
        let cat_id = world.colonies[0].cats[0].id.clone();
        world.colonies[0].cats[0].boosted = true;

        let snapshot = build_snapshot(&world, 1_000_000, 1);
        let cat_snap = snapshot.colonies[0]
            .cats
            .iter()
            .find(|cat| cat.id == cat_id)
            .expect("boosted cat present in snapshot");
        assert!(cat_snap.boosted);

        let other = snapshot.colonies[0]
            .cats
            .iter()
            .find(|cat| cat.id != cat_id)
            .expect("second founding cat present");
        assert!(!other.boosted);
    }

    #[test]
    fn build_snapshot_exposes_officers_by_role() {
        let mut world = world_with_one_colony();
        let cat_id = world.colonies[0].cats[0].id.clone();
        world.colonies[0]
            .officers
            .insert(OfficerRole::Loremaster, cat_id.clone());

        let snap = build_snapshot(&world, 1_000_000, 1);
        assert_eq!(
            snap.colonies[0]
                .officers
                .get(&proto::OfficerRole::Loremaster),
            Some(&cat_id)
        );
    }

    // ---- P12.3 stockpile actions ----

    fn tp(x: i32, y: i32) -> proto::TilePoint {
        proto::TilePoint { x, y }
    }

    fn designate_action(
        a: proto::TilePoint,
        b: proto::TilePoint,
        accepts: Vec<proto::ResourceKind>,
    ) -> proto::ClientAction {
        proto::ClientAction::DesignateStockpile {
            session_id: "sess_1".to_string(),
            nickname: "Guest".to_string(),
            sig: "sig".to_string(),
            a,
            b,
            accepts,
        }
    }

    fn assert_stockpile_invariant(colony: &ColonyRuntime) {
        for &kind in stockpiles::ResourceKind::ALL {
            let sum: f64 = colony
                .stockpiles
                .iter()
                .map(|pile| stockpiles::resource_amount(&pile.contents, kind))
                .sum();
            let total = stockpiles::resource_amount(&colony.resources, kind);
            assert!(
                (sum - total).abs() <= 1e-6,
                "{kind:?}: pile sum {sum} != resources {total}"
            );
        }
    }

    #[test]
    fn designate_stockpile_adds_a_pile_and_keeps_the_invariant() {
        let mut world = world_with_one_colony();
        let before = world.colonies[0].stockpiles.len();
        let res = apply_action(
            &mut world,
            &designate_action(tp(8, 8), tp(9, 9), vec![proto::ResourceKind::Food]),
            &ctx(),
        );
        assert!(res.ok, "{res:?}");
        assert_eq!(world.colonies[0].stockpiles.len(), before + 1);
        assert_stockpile_invariant(&world.colonies[0]);
    }

    #[test]
    fn designate_stockpile_rejects_oversized_or_empty_accepts() {
        let mut world = world_with_one_colony();
        let before = world.colonies[0].stockpiles.len();

        let too_big = apply_action(
            &mut world,
            &designate_action(tp(0, 0), tp(20, 0), vec![proto::ResourceKind::Food]),
            &ctx(),
        );
        assert!(!too_big.ok, "oversized rect rejected");

        let empty = apply_action(
            &mut world,
            &designate_action(tp(8, 8), tp(8, 8), vec![]),
            &ctx(),
        );
        assert!(!empty.ok, "empty accept set rejected");
        assert_eq!(world.colonies[0].stockpiles.len(), before);
    }

    #[test]
    fn remove_stockpile_refuses_shrine_and_folds_designated_back() {
        let mut world = world_with_one_colony();
        let refuse = apply_action(
            &mut world,
            &proto::ClientAction::RemoveStockpile {
                session_id: "sess_1".to_string(),
                nickname: "Guest".to_string(),
                sig: "sig".to_string(),
                stockpile_id: stockpiles::SHRINE_STOCKPILE_ID.to_string(),
            },
            &ctx(),
        );
        assert!(!refuse.ok, "shrine reservoir cannot be removed");

        let _ = apply_action(
            &mut world,
            &designate_action(tp(8, 8), tp(8, 8), vec![proto::ResourceKind::Food]),
            &ctx(),
        );
        let pile_id = world.colonies[0]
            .stockpiles
            .iter()
            .find(|p| !p.is_shrine())
            .unwrap()
            .id
            .clone();
        world.colonies[0]
            .stockpiles
            .iter_mut()
            .find(|p| p.id == pile_id)
            .unwrap()
            .contents
            .food = 12.0;

        let removed = apply_action(
            &mut world,
            &proto::ClientAction::RemoveStockpile {
                session_id: "sess_1".to_string(),
                nickname: "Guest".to_string(),
                sig: "sig".to_string(),
                stockpile_id: pile_id.clone(),
            },
            &ctx(),
        );
        assert!(removed.ok, "{removed:?}");
        assert!(!world.colonies[0].stockpiles.iter().any(|p| p.id == pile_id));
        assert_stockpile_invariant(&world.colonies[0]);

        let noop = apply_action(
            &mut world,
            &proto::ClientAction::RemoveStockpile {
                session_id: "sess_1".to_string(),
                nickname: "Guest".to_string(),
                sig: "sig".to_string(),
                stockpile_id: "stockpile-nope".to_string(),
            },
            &ctx(),
        );
        assert!(noop.ok, "unknown id is a no-op");
    }

    fn designate_gather_action(
        a: proto::TilePoint,
        b: proto::TilePoint,
        kind: proto::ResourceKind,
    ) -> proto::ClientAction {
        proto::ClientAction::DesignateGatherSpot {
            session_id: "sess_1".to_string(),
            nickname: "Guest".to_string(),
            sig: "sig".to_string(),
            a,
            b,
            kind,
        }
    }

    #[test]
    fn designate_gather_spot_adds_a_pile_and_bookkeeping_record() {
        let mut world = world_with_one_colony();
        let res = apply_action(
            &mut world,
            &designate_gather_action(tp(30, 30), tp(30, 30), proto::ResourceKind::Food),
            &ctx(),
        );
        assert!(res.ok, "{res:?}");
        assert_eq!(world.colonies[0].gather_spots.len(), 1);
        let spot = &world.colonies[0].gather_spots[0];
        assert_eq!(spot.kind, stockpiles::ResourceKind::Food);
        let pile = world.colonies[0]
            .stockpiles
            .iter()
            .find(|pile| pile.id == spot.stockpile_id)
            .expect("underlying pile exists");
        assert_eq!(
            pile.accepts,
            [stockpiles::ResourceKind::Food].into_iter().collect()
        );
        assert_stockpile_invariant(&world.colonies[0]);
    }

    #[test]
    fn designate_gather_spot_rejects_unsupported_resources_and_oversized_rects() {
        let mut world = world_with_one_colony();

        let unsupported = apply_action(
            &mut world,
            &designate_gather_action(tp(30, 30), tp(30, 30), proto::ResourceKind::Blessings),
            &ctx(),
        );
        assert!(!unsupported.ok, "only food/water/materials are collectable");

        let too_big = apply_action(
            &mut world,
            &designate_gather_action(tp(0, 0), tp(10, 0), proto::ResourceKind::Food),
            &ctx(),
        );
        assert!(
            !too_big.ok,
            "gather spots are capped at GATHER_SPOT_MAX_EDGE"
        );
        assert!(world.colonies[0].gather_spots.is_empty());
    }

    #[test]
    fn designate_gather_spot_enforces_its_own_budget() {
        let mut world = world_with_one_colony();
        for i in 0..stockpiles::MAX_GATHER_SPOTS {
            let x = 30 + i as i32;
            let res = apply_action(
                &mut world,
                &designate_gather_action(tp(x, 30), tp(x, 30), proto::ResourceKind::Food),
                &ctx(),
            );
            assert!(res.ok, "spot {i}: {res:?}");
        }
        let over_budget = apply_action(
            &mut world,
            &designate_gather_action(tp(99, 99), tp(99, 99), proto::ResourceKind::Food),
            &ctx(),
        );
        assert!(
            !over_budget.ok,
            "budget enforced independent of MAX_DESIGNATED_STOCKPILES"
        );
        assert_eq!(
            world.colonies[0].gather_spots.len(),
            stockpiles::MAX_GATHER_SPOTS
        );
    }

    #[test]
    fn remove_gather_spot_folds_contents_back_and_cancels_its_mover() {
        let mut world = world_with_one_colony();
        let _ = apply_action(
            &mut world,
            &designate_gather_action(tp(30, 30), tp(30, 30), proto::ResourceKind::Food),
            &ctx(),
        );
        let spot_id = world.colonies[0].gather_spots[0].stockpile_id.clone();
        {
            let colony = &mut world.colonies[0];
            colony
                .stockpiles
                .iter_mut()
                .find(|pile| pile.id == spot_id)
                .unwrap()
                .contents
                .food = 9.0;
            colony.resources.food += 9.0;
            let cat_id = colony.cats[0].id.clone();
            colony.jobs.push(JobRuntime {
                id: "job-mover".to_owned(),
                kind: JobKind::HaulGatherSpot,
                status: JobStatus::Active,
                assigned_cat: Some(cat_id.clone()),
                metadata: JobMetadata::GatherHaul {
                    stockpile_id: spot_id.clone(),
                    site: Some(TilePos { x: 30, y: 30 }),
                    accepted: true,
                },
                ..JobRuntime::default()
            });
            if let Some(cat) = colony.cats.iter_mut().find(|cat| cat.id == cat_id) {
                cat.activity = CatActivity::Working;
            }
        }

        let removed = apply_action(
            &mut world,
            &proto::ClientAction::RemoveGatherSpot {
                session_id: "sess_1".to_string(),
                nickname: "Guest".to_string(),
                sig: "sig".to_string(),
                stockpile_id: spot_id.clone(),
            },
            &ctx(),
        );
        assert!(removed.ok, "{removed:?}");
        assert!(world.colonies[0].gather_spots.is_empty());
        assert!(!world.colonies[0].stockpiles.iter().any(|p| p.id == spot_id));
        assert_stockpile_invariant(&world.colonies[0]);

        let job = world.colonies[0]
            .jobs
            .iter()
            .find(|job| job.id == "job-mover")
            .unwrap();
        assert_eq!(job.status, JobStatus::Cancelled, "dangling mover cancelled");
        assert_eq!(
            world.colonies[0].cats[0].activity,
            CatActivity::Idle,
            "cat freed since it was not yet carrying anything"
        );

        let unknown = apply_action(
            &mut world,
            &proto::ClientAction::RemoveGatherSpot {
                session_id: "sess_1".to_string(),
                nickname: "Guest".to_string(),
                sig: "sig".to_string(),
                stockpile_id: "gather-nope".to_string(),
            },
            &ctx(),
        );
        assert!(!unknown.ok, "unknown gather spot id is rejected");
    }

    #[test]
    fn build_snapshot_exposes_stockpiles() {
        let mut world = world_with_one_colony();
        let _ = apply_action(
            &mut world,
            &designate_action(tp(8, 8), tp(9, 9), vec![proto::ResourceKind::Food]),
            &ctx(),
        );
        let snap = build_snapshot(&world, 1_000_000, 1);
        assert!(
            snap.colonies[0]
                .stockpiles
                .iter()
                .any(|pile| pile.id == stockpiles::SHRINE_STOCKPILE_ID),
            "shrine reservoir exposed"
        );
        assert!(
            snap.colonies[0].stockpiles.len() >= 2,
            "designated pile exposed"
        );
    }

    #[test]
    fn build_snapshot_flags_gather_spots_on_their_stockpile_snapshot() {
        let mut world = world_with_one_colony();
        let _ = apply_action(
            &mut world,
            &designate_gather_action(tp(30, 30), tp(30, 30), proto::ResourceKind::Water),
            &ctx(),
        );
        let spot_id = world.colonies[0].gather_spots[0].stockpile_id.clone();
        let snap = build_snapshot(&world, 1_000_000, 1);
        let pile = snap.colonies[0]
            .stockpiles
            .iter()
            .find(|pile| pile.id == spot_id)
            .expect("gather spot pile exposed");
        let gather_spot = pile.gather_spot.expect("flagged as a gather spot");
        assert_eq!(gather_spot.kind, proto::ResourceKind::Water);

        // The shrine and a general stockpile are never flagged as gather spots.
        let shrine = snap.colonies[0]
            .stockpiles
            .iter()
            .find(|pile| pile.id == stockpiles::SHRINE_STOCKPILE_ID)
            .expect("shrine exposed");
        assert!(shrine.gather_spot.is_none());
    }

    #[test]
    fn build_snapshot_omits_items_for_the_default_empty_store() {
        // P19 slice 1: every colony's item store starts empty (nothing produces items
        // yet), so the snapshot's `items` list should be empty too.
        let world = world_with_one_colony();
        let snap = build_snapshot(&world, 1_000_000, 1);
        assert!(snap.colonies[0].items.is_empty());
    }

    #[test]
    fn build_snapshot_exposes_a_seeded_item_store_in_deterministic_order_with_values() {
        use crate::items::{Item, ItemKind, Material, item_value};

        let mut world = world_with_one_colony();
        let colony = &mut world.colonies[0];
        // Insert out of eventual sort order to prove the snapshot reflects the store's
        // own BTreeMap order, not insertion order.
        colony.add_item(Item::new(ItemKind::Weapon, Material::Metal, 3), 2);
        colony.add_item(Item::new(ItemKind::Mug, Material::Wood, 1), 5);
        colony.add_item(Item::new(ItemKind::Mug, Material::Stone, 1), 1);

        let snap = build_snapshot(&world, 1_000_000, 1);
        let items = &snap.colonies[0].items;
        assert_eq!(items.len(), 3);

        // Mug < Weapon (kind order); within Mug, Wood < Stone (material order).
        assert_eq!(items[0].kind, "mug");
        assert_eq!(items[0].material, "wood");
        assert_eq!(items[0].count, 5);
        assert_eq!(items[0].value, item_value(ItemKind::Mug, Material::Wood, 1));

        assert_eq!(items[1].kind, "mug");
        assert_eq!(items[1].material, "stone");
        assert_eq!(items[1].count, 1);

        assert_eq!(items[2].kind, "weapon");
        assert_eq!(items[2].material, "metal");
        assert_eq!(items[2].quality, 3);
        assert_eq!(items[2].count, 2);
        assert_eq!(
            items[2].value,
            item_value(ItemKind::Weapon, Material::Metal, 3)
        );
    }

    // ---- P19 slice 3: visiting trader / caravan economy ----

    fn trading_trader() -> TraderRuntime {
        TraderRuntime {
            id: "trader-1".to_owned(),
            position: Position {
                map: MapType::World,
                x: 0.0,
                y: 0.0,
            },
            destination: None,
            state: trader::TraderState::Trading,
            arrived_at: Some(0),
        }
    }

    fn sell_goods_action(
        kind: &str,
        material: &str,
        quality: u8,
        count: u32,
    ) -> proto::ClientAction {
        proto::ClientAction::SellGoods {
            session_id: "sess_1".to_string(),
            nickname: "Guest".to_string(),
            sig: "sig".to_string(),
            kind: kind.to_string(),
            material: material.to_string(),
            quality,
            count,
        }
    }

    fn buy_resource_action(resource: proto::ResourceKind, amount: f64) -> proto::ClientAction {
        proto::ClientAction::BuyResource {
            session_id: "sess_1".to_string(),
            nickname: "Guest".to_string(),
            sig: "sig".to_string(),
            resource,
            amount,
        }
    }

    #[test]
    fn build_snapshot_defaults_coin_to_zero_and_omits_trader_when_none_visiting() {
        let world = world_with_one_colony();
        let snap = build_snapshot(&world, 1_000_000, 1);
        assert_eq!(snap.colonies[0].coin, 0.0);
        assert!(snap.colonies[0].trader.is_none());
    }

    #[test]
    fn build_snapshot_exposes_coin_and_a_trading_traders_buy_and_sell_offers() {
        let mut world = world_with_one_colony();
        let colony = &mut world.colonies[0];
        colony.coin = 42.0;
        colony.add_item(Item::new(ItemKind::Mug, Material::Wood, 1), 3);
        colony.trader = Some(trading_trader());

        let snap = build_snapshot(&world, 1_000_000, 1);
        let colony_snap = &snap.colonies[0];
        assert_eq!(colony_snap.coin, 42.0);
        let trader_snap = colony_snap.trader.as_ref().expect("trader present");
        assert_eq!(trader_snap.state, proto::TraderVisitState::Trading);
        assert_eq!(trader_snap.buy_offers.len(), 1);
        assert_eq!(trader_snap.buy_offers[0].kind, "mug");
        assert_eq!(trader_snap.buy_offers[0].material, "wood");
        assert_eq!(trader_snap.buy_offers[0].available, 3);
        assert!(
            !trader_snap.sell_offers.is_empty(),
            "a trading trader should list resource sell offers"
        );
    }

    #[test]
    fn build_snapshot_reports_no_offers_while_the_trader_is_still_arriving() {
        let mut world = world_with_one_colony();
        let colony = &mut world.colonies[0];
        colony.add_item(Item::new(ItemKind::Mug, Material::Wood, 1), 3);
        colony.trader = Some(TraderRuntime {
            state: trader::TraderState::Arriving,
            arrived_at: None,
            ..trading_trader()
        });

        let snap = build_snapshot(&world, 1_000_000, 1);
        let trader_snap = snap.colonies[0].trader.as_ref().expect("trader present");
        assert_eq!(trader_snap.state, proto::TraderVisitState::Arriving);
        assert!(trader_snap.buy_offers.is_empty());
        assert!(trader_snap.sell_offers.is_empty());
    }

    #[test]
    fn sell_goods_removes_items_and_credits_coin_at_the_trader_buy_price() {
        let mut world = world_with_one_colony();
        let item = Item::new(ItemKind::Mug, Material::Wood, 1);
        world.colonies[0].add_item(item, 5);
        world.colonies[0].trader = Some(trading_trader());

        let res = apply_action(&mut world, &sell_goods_action("mug", "wood", 1, 3), &ctx());
        assert!(res.ok, "{res:?}");
        assert_eq!(world.colonies[0].items.get(&item), Some(&2));
        assert_eq!(world.colonies[0].coin, trader::trader_buy_price(item, 3));
    }

    #[test]
    fn sell_goods_selling_more_than_owned_is_denied_and_leaves_the_store_and_coin_untouched() {
        let mut world = world_with_one_colony();
        let item = Item::new(ItemKind::Mug, Material::Wood, 1);
        world.colonies[0].add_item(item, 2);
        world.colonies[0].trader = Some(trading_trader());

        let res = apply_action(&mut world, &sell_goods_action("mug", "wood", 1, 3), &ctx());
        assert!(!res.ok);
        assert_eq!(
            world.colonies[0].items.get(&item),
            Some(&2),
            "store untouched"
        );
        assert_eq!(world.colonies[0].coin, 0.0, "coin untouched");
    }

    #[test]
    fn sell_goods_with_no_trader_present_is_denied() {
        let mut world = world_with_one_colony();
        let item = Item::new(ItemKind::Mug, Material::Wood, 1);
        world.colonies[0].add_item(item, 5);
        assert!(world.colonies[0].trader.is_none());

        let res = apply_action(&mut world, &sell_goods_action("mug", "wood", 1, 2), &ctx());
        assert!(!res.ok);
        assert_eq!(
            world.colonies[0].items.get(&item),
            Some(&5),
            "store untouched"
        );
        assert_eq!(world.colonies[0].coin, 0.0, "coin untouched");
    }

    #[test]
    fn sell_goods_while_trader_is_still_arriving_is_denied() {
        let mut world = world_with_one_colony();
        let item = Item::new(ItemKind::Mug, Material::Wood, 1);
        world.colonies[0].add_item(item, 5);
        world.colonies[0].trader = Some(TraderRuntime {
            state: trader::TraderState::Arriving,
            arrived_at: None,
            ..trading_trader()
        });

        let res = apply_action(&mut world, &sell_goods_action("mug", "wood", 1, 2), &ctx());
        assert!(!res.ok);
        assert_eq!(world.colonies[0].items.get(&item), Some(&5));
        assert_eq!(world.colonies[0].coin, 0.0);
    }

    #[test]
    fn sell_goods_rejects_an_unknown_kind_or_material() {
        let mut world = world_with_one_colony();
        world.colonies[0].trader = Some(trading_trader());

        let res = apply_action(
            &mut world,
            &sell_goods_action("not_a_kind", "wood", 1, 1),
            &ctx(),
        );
        assert!(!res.ok);
        assert_eq!(world.colonies[0].coin, 0.0);
    }

    #[test]
    fn buy_resource_spends_coin_and_credits_the_resource_at_the_trader_sell_price() {
        let mut world = world_with_one_colony();
        world.colonies[0].coin = 100.0;
        let starting_food = world.colonies[0].resources.food;
        world.colonies[0].trader = Some(trading_trader());

        let cost = trader::trader_sell_price(stockpiles::ResourceKind::Food, 20.0).unwrap();
        let res = apply_action(
            &mut world,
            &buy_resource_action(proto::ResourceKind::Food, 20.0),
            &ctx(),
        );
        assert!(res.ok, "{res:?}");
        assert_eq!(world.colonies[0].resources.food, starting_food + 20.0);
        assert_eq!(world.colonies[0].coin, 100.0 - cost);
    }

    #[test]
    fn buy_resource_with_insufficient_coin_is_denied_and_leaves_coin_and_resources_untouched() {
        let mut world = world_with_one_colony();
        world.colonies[0].coin = 1.0;
        let starting_food = world.colonies[0].resources.food;
        world.colonies[0].trader = Some(trading_trader());

        let res = apply_action(
            &mut world,
            &buy_resource_action(proto::ResourceKind::Food, 20.0),
            &ctx(),
        );
        assert!(!res.ok);
        assert_eq!(world.colonies[0].resources.food, starting_food);
        assert_eq!(world.colonies[0].coin, 1.0);
    }

    #[test]
    fn buy_resource_rejects_a_kind_the_trader_does_not_stock() {
        let mut world = world_with_one_colony();
        world.colonies[0].coin = 1_000.0;
        world.colonies[0].trader = Some(trading_trader());

        // Weapons/armor are functional smithy output, not a caravan trade good — see
        // `trader::resource_unit_price`.
        let res = apply_action(
            &mut world,
            &buy_resource_action(proto::ResourceKind::Weapons, 1.0),
            &ctx(),
        );
        assert!(!res.ok);
        assert_eq!(world.colonies[0].coin, 1_000.0);
    }

    #[test]
    fn buy_resource_with_no_trader_present_is_denied() {
        let mut world = world_with_one_colony();
        world.colonies[0].coin = 1_000.0;
        assert!(world.colonies[0].trader.is_none());

        let res = apply_action(
            &mut world,
            &buy_resource_action(proto::ResourceKind::Food, 5.0),
            &ctx(),
        );
        assert!(!res.ok);
        assert_eq!(world.colonies[0].coin, 1_000.0);
    }

    #[test]
    fn build_snapshot_exposes_the_revealed_fog_set() {
        let world = world_with_one_colony();
        let snap = build_snapshot(&world, 1_000_000, 1);
        let revealed = &snap.colonies[0].revealed_tiles;
        // The founding village reveal is present.
        assert!(!revealed.is_empty(), "founding reveal should be exposed");
        assert!(
            revealed.contains(&proto::TilePoint {
                x: VILLAGE_ANCHOR.x,
                y: VILLAGE_ANCHOR.y,
            }),
            "the village anchor tile is revealed"
        );
        assert!(
            revealed.len() < world.colonies[0].world_tiles.len(),
            "the wilds beyond the village start fogged"
        );
    }

    #[test]
    fn build_snapshot_exposes_a_scouts_provisional_tiles_dedeuped_and_sorted() {
        // P15: an out scout's tentative reveal must reach the client (dim/uncommitted)
        // distinctly from `revealed_tiles`, merged across every currently-out scout
        // into one deterministic, deduped, sorted set.
        let mut world = world_with_one_colony();
        world.colonies[0].provisional_tiles = BTreeMap::from([
            (
                "scout_a".to_owned(),
                BTreeSet::from([TilePos { x: 40, y: 40 }, TilePos { x: 41, y: 40 }]),
            ),
            (
                "scout_b".to_owned(),
                // Overlaps scout_a's tile — must not appear twice in the snapshot.
                BTreeSet::from([TilePos { x: 41, y: 40 }, TilePos { x: -5, y: 90 }]),
            ),
        ]);

        let snap = build_snapshot(&world, 1_000_000, 1);
        let provisional = &snap.colonies[0].provisional_tiles;

        assert_eq!(
            provisional,
            &vec![
                proto::TilePoint { x: -5, y: 90 },
                proto::TilePoint { x: 40, y: 40 },
                proto::TilePoint { x: 41, y: 40 },
            ],
            "provisional tiles from every out scout are merged, deduped, and sorted"
        );
        // Untouched by provisional state — the two tiers stay independent.
        assert!(
            !snap.colonies[0]
                .revealed_tiles
                .contains(&proto::TilePoint { x: 40, y: 40 }),
            "provisional tiles must not leak into the committed revealed set"
        );
    }

    #[test]
    fn build_snapshot_omits_provisional_tiles_when_no_scout_is_out() {
        let world = world_with_one_colony();
        assert!(world.colonies[0].provisional_tiles.is_empty());

        let snap = build_snapshot(&world, 1_000_000, 1);
        assert!(snap.colonies[0].provisional_tiles.is_empty());
    }

    #[test]
    fn revealed_fog_set_is_independent_of_the_world_tiles_map() {
        // Fog is tracked on a standalone tile set, so the snapshot still exposes the
        // founding reveal even when the live colony's `world_tiles` map is empty/sparse
        // (tiles are materialised lazily on the server) — this is what the client needs
        // to render an un-fogged village.
        let mut world = world_with_one_colony();
        world.colonies[0].world_tiles.clear();
        let snap = build_snapshot(&world, 1_000_000, 1);
        assert!(
            !snap.colonies[0].revealed_tiles.is_empty(),
            "revealed set must survive an empty world_tiles map"
        );
    }
}
