//! Pure action application and snapshot building ported from
//! `app/api/game/actions/route.ts` and `server/game.ts:getGlobalDashboard`.

use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet, HashSet},
};

use cat_protocol as proto;

use crate::{
    entities::{self, Cat, CatActivity, MapType, Position},
    farming::{self, FarmPlot, FarmStage},
    housing::{self, HousingBuilding},
    idle_engine, idle_rules,
    items::{Item, ItemKind, ItemStore, Material, item_weight_grams, item_workshop_id},
    leader_director::{
        OFFERING_MATERIALS_AMOUNT, OFFERING_MATERIALS_RESERVE, TITHE_FOOD_AMOUNT,
        TITHE_FOOD_RESERVE_FLOOR, TITHE_FOOD_RESERVE_PER_CAT, TITHE_REFINED_AMOUNT,
    },
    life_sim::{can_work, get_life_stage},
    officers::{OfficerRole, prerequisite_for},
    production,
    productivity::productive_duration_ms,
    skills::Labor,
    stockpiles,
    storage::{self, StorageBuilding},
    threat, trader,
    types::{BuildingType, CatSpecialization, JobKind, JobStatus, TaskType, TileType, UpgradeKey},
    upgrade_tree,
    village_area::{self, gate_placement_default},
    village_layout::{GridPos, village_ring_radius},
    village_sites::select_personal_village_site,
    world_tick::{
        ColonyRuntime, ConstructionPhase, ElectionKind, ElectionRuntime, EventKind, EventLog,
        JobMetadata, JobRequester, JobRuntime, RaiderRuntime, ScoutMission, ScoutResource, TilePos,
        TradeDirection, VillageKind, VillageScale, VillageTradeOffer, VoteRuntime, WorldState,
        ZoneRuntime, election_schedule_timing, ensure_farm_gather_spot_at,
        farm_designation_route_is_reachable, farm_gather_spot_id, farm_rect_touches_claim_boundary,
        farm_route_is_reachable, found_colony_at, found_global_colony, has_frontier,
        has_logging_site, has_quarry_site, has_water_site, inside_village_interior,
        is_farm_gather_spot_id, legal_farm_gather_spots, material_offering_metadata,
        migration_game_minute_at, occupied_farm_tiles, reconcile_colony_stockpiles,
        release_farm_worker, release_role_automation, village_exterior_is_road_connected,
        visible_offering_materials, world_tick,
    },
    zones,
};

const DEFEND_CLICK_DAMAGE: f64 = 6.0;
const EVENT_KEEP_SNAPSHOT: usize = 30;
const KICK_THRESHOLD: u32 = 5;
const MAX_ADVANCE_SECONDS: u64 = 86_400;
const MAX_PERSONAL_VILLAGE_ID_COLLISIONS: u32 = 1_024;
const MAX_VILLAGE_NAME_CHARS: usize = 48;
const MAX_VILLAGE_TRADE_AMOUNT: f64 = 1_000_000.0;
const MAX_VILLAGE_TRADE_ID_COLLISIONS: u32 = 1_024;
const MAX_OPEN_VILLAGE_TRADE_OFFERS: usize = 32;

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
    /// Colony selected by the authenticated server connection. Mutating actions
    /// must never infer this from world order in a multi-colony world.
    pub colony_id: String,
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
            let world_seed = world.world_seed;
            with_colony(world, ctx, |colony| {
                request_job(colony, kind, world_seed, ctx)
            })
        }
        proto::ClientAction::DispatchScout { mission, .. } => {
            let mission = proto_to_sim_scout_mission(*mission);
            with_colony(world, ctx, |colony| dispatch_scout(colony, mission, ctx))
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
        proto::ClientAction::PlanBuilding {
            building_type,
            site,
            ..
        } => {
            let world_seed = world.world_seed;
            with_colony(world, ctx, |colony| {
                plan_building(colony, *building_type, *site, world_seed, ctx)
            })
        }
        proto::ClientAction::UnlockNode { node_id, .. } => {
            with_colony(world, ctx, |colony| unlock_node(colony, node_id, ctx))
        }
        proto::ClientAction::ResearchNode { node_id, .. } => {
            with_colony(world, ctx, |colony| research_node(colony, node_id, ctx))
        }
        proto::ClientAction::OfferTithe { .. } => {
            with_colony(world, ctx, |colony| offer_tithe(colony, ctx))
        }
        proto::ClientAction::OfferMaterials { .. } => {
            with_colony(world, ctx, |colony| offer_materials(colony, ctx))
        }
        proto::ClientAction::HaulGatherSpot {
            stockpile_id,
            cat_id,
            ..
        } => with_colony(world, ctx, |colony| {
            haul_gather_spot(colony, stockpile_id, cat_id.as_deref(), ctx)
        }),
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
            let world_seed = world.world_seed;
            with_colony(world, ctx, |colony| {
                build_road(colony, *a, *b, world_seed, ctx)
            })
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
        proto::ClientAction::FoundVillage {
            name, session_id, ..
        } => found_village(world, name, session_id, ctx),
        proto::ClientAction::JoinVillage { colony_id, .. } => {
            let Some(colony) = world.colonies.iter().find(|colony| colony.id == *colony_id) else {
                return fail("Village is not available.");
            };
            if !can_control_village(colony, &ctx.player_id) {
                return fail("Village is not available.");
            }
            ok_for_colony(&colony.id)
        }
        proto::ClientAction::OfferVillageTrade {
            target_colony_id,
            offered_kind,
            offered_amount,
            requested_kind,
            requested_amount,
            ..
        } => offer_village_trade(
            world,
            target_colony_id,
            *offered_kind,
            *offered_amount,
            *requested_kind,
            *requested_amount,
            ctx,
        ),
        proto::ClientAction::AcceptVillageTrade { offer_id, .. } => {
            accept_village_trade(world, offer_id, ctx)
        }
        proto::ClientAction::CancelVillageTrade { offer_id, .. } => {
            cancel_village_trade(world, offer_id, ctx)
        }
        proto::ClientAction::AssignOfficer { role, cat_id, .. } => {
            with_colony(world, ctx, |colony| {
                assign_officer(colony, proto_to_sim_officer_role(*role), cat_id, ctx)
            })
        }
        proto::ClientAction::UnassignOfficer { role, .. } => with_colony(world, ctx, |colony| {
            unassign_officer(colony, proto_to_sim_officer_role(*role), ctx)
        }),
        proto::ClientAction::DesignateFarm { a, b, crop, .. } => {
            let world_seed = world.world_seed;
            with_colony(world, ctx, |colony| {
                designate_farm(colony, world_seed, *a, *b, *crop, ctx)
            })
        }
        proto::ClientAction::ClearFarm { plot_id, .. } => {
            with_colony(world, ctx, |colony| clear_farm(colony, plot_id, ctx))
        }
        proto::ClientAction::DesignateStockpile { a, b, accepts, .. } => {
            let world_seed = world.world_seed;
            with_colony(world, ctx, |colony| {
                designate_stockpile(colony, *a, *b, accepts, world_seed, ctx)
            })
        }
        proto::ClientAction::RemoveStockpile { stockpile_id, .. } => {
            with_colony(world, ctx, |colony| {
                remove_stockpile(colony, stockpile_id, ctx)
            })
        }
        proto::ClientAction::DesignateGatherSpot { a, b, kind, .. } => {
            let world_seed = world.world_seed;
            with_colony(world, ctx, |colony| {
                designate_gather_spot(colony, *a, *b, *kind, world_seed, ctx)
            })
        }
        proto::ClientAction::DesignateFishingSpot { at, .. } => {
            let world_seed = world.world_seed;
            with_colony(world, ctx, |colony| {
                designate_fishing_spot(colony, *at, world_seed, ctx)
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
        proto::ClientAction::RepairItem { item_id, .. } => {
            with_colony(world, ctx, |colony| repair_item(colony, item_id, ctx))
        }
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
        proto::ClientAction::SetCatLaborPreference {
            cat_id,
            labor,
            enabled,
            ..
        } => with_colony(world, ctx, |colony| {
            set_cat_labor_preference(colony, cat_id, proto_to_sim_labor(*labor), *enabled, ctx)
        }),
        proto::ClientAction::EditProductionQueue {
            building_id, edit, ..
        } => with_colony(world, ctx, |colony| {
            edit_production_queue(colony, building_id, edit, ctx)
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
        selected_colony_id: None,
        known_villages: Vec::new(),
        village_trade_offers: world
            .colonies
            .iter()
            .flat_map(|colony| colony.village_trade_offers.values())
            .map(village_trade_offer_snapshot)
            .collect(),
    }
}

fn with_colony(
    world: &mut WorldState,
    ctx: &ActionCtx,
    f: impl FnOnce(&mut ColonyRuntime) -> proto::ActionResult,
) -> proto::ActionResult {
    let Some(colony) = world
        .colonies
        .iter_mut()
        .find(|colony| colony.id == ctx.colony_id)
    else {
        return fail("Village not found.");
    };
    if !can_control_village(colony, &ctx.player_id) {
        return fail("Village is not available.");
    }
    f(colony)
}

/// Defense-in-depth authorization used by every colony-scoped mutation. The
/// shared global village preserves the original communal play model; a personal
/// village is controllable only by its stable owner.
#[must_use]
pub fn can_control_village(colony: &ColonyRuntime, player_id: &str) -> bool {
    match colony.kind {
        VillageKind::Global => true,
        VillageKind::Personal => colony.owner_player_id.as_deref() == Some(player_id),
    }
}

fn ensure_colony(world: &mut WorldState, now_ms: i64) {
    if world.colonies.is_empty() {
        world
            .colonies
            .push(found_global_colony(world.world_seed, "colony-1", now_ms, 1));
    }
}

fn request_job(
    colony: &mut ColonyRuntime,
    kind: JobKind,
    world_seed: u32,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    if !matches!(
        kind,
        JobKind::SupplyFood
            | JobKind::SupplyWater
            | JobKind::LeaderPlanHunt
            | JobKind::LeaderPlanHouse
            | JobKind::GatherLogs
            | JobKind::Fish
            | JobKind::ForageFibre
            | JobKind::Ritual
            | JobKind::HuntExpedition
            | JobKind::Quarry
            | JobKind::Explore
            | JobKind::FetchWater
            | JobKind::TrainWarrior
            | JobKind::ExpandVillage
            | JobKind::CarryOffering
            | JobKind::PerformOffering
            | JobKind::HaulGatherSpot
            | JobKind::BuildHouse
    ) {
        return fail("Unknown job kind.");
    }

    if let upgrade_tree::JobResearchEntitlement::Requires { node_name, .. } =
        upgrade_tree::job_research_entitlement(&colony.upgrade_tree, kind.as_str())
    {
        return fail(format!("Research {node_name} before requesting logging."));
    }
    if kind == JobKind::GatherLogs && !has_logging_site(colony, world_seed) {
        return fail("No explored forest is available for logging.");
    }
    if kind == JobKind::Fish && !crate::world_tick::has_fishing_site(colony) {
        return fail("Designate a revealed shoreline fishing spot first.");
    }
    if kind == JobKind::Fish && !crate::world_tick::has_fishable_stock(colony) {
        return fail("The designated fish habitat is depleted and replenishing.");
    }
    if kind == JobKind::Quarry && !has_quarry_site(colony) {
        return fail("No explored quarry site is available.");
    }
    if kind == JobKind::FetchWater && !has_water_site(colony) {
        return fail("No explored water source is available.");
    }
    if kind == JobKind::Explore && !has_frontier(colony) {
        return fail("No unexplored frontier is available.");
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

    if kind == JobKind::TrainWarrior {
        return train_warrior(colony, None, ctx);
    }
    if kind == JobKind::CarryOffering {
        return offer_materials(colony, ctx);
    }
    if kind == JobKind::PerformOffering {
        return fail("An offering ritual begins only after physical shrine delivery.");
    }
    if kind == JobKind::HaulGatherSpot {
        return fail("Choose a gather spot to haul.");
    }
    if kind == JobKind::BuildHouse {
        return fail("Choose a building type to construct.");
    }
    if kind == JobKind::ExpandVillage
        && active_or_queued_jobs(colony)
            .iter()
            .any(|job| job.kind == JobKind::ExpandVillage)
    {
        return fail("That request is already in progress.");
    }

    let labor = Labor::for_job_kind(kind);
    let assigned_cat = match kind {
        JobKind::HuntExpedition => {
            select_best_cat_for_labor(colony, Some(CatSpecialization::Hunter), labor)
        }
        JobKind::Quarry => {
            select_best_cat_for_labor(colony, Some(CatSpecialization::Architect), labor)
        }
        JobKind::GatherLogs
        | JobKind::Fish
        | JobKind::ForageFibre
        | JobKind::Explore
        | JobKind::FetchWater
        | JobKind::ExpandVillage => select_best_cat_for_labor(colony, None, labor),
        _ => None,
    };
    if matches!(
        kind,
        JobKind::HuntExpedition
            | JobKind::Quarry
            | JobKind::GatherLogs
            | JobKind::Fish
            | JobKind::ForageFibre
            | JobKind::Explore
            | JobKind::FetchWater
            | JobKind::ExpandVillage
    ) && assigned_cat.is_none()
    {
        return fail("No available worker.");
    }

    let metadata = if kind == JobKind::ExpandVillage {
        let area = claimed_area(colony);
        let is_water = |position: GridPos| {
            let pos = TilePos {
                x: position.x,
                y: position.y,
            };
            if colony.claimed_tiles.contains(&pos) {
                return true;
            }
            colony.world_tiles.get(&pos).is_some_and(|tile| {
                tile.tile_type == TileType::River
                    || tile.overlay_feature.as_deref() == Some("river")
                    || tile.resources.water > 0
            })
        };
        let Some(target) = village_area::expand_village(
            &area,
            village_area::ExpandOptions {
                is_water: Some(&is_water),
                rng: None,
            },
        ) else {
            return fail("There is no adjacent land to claim.");
        };
        JobMetadata::Expansion {
            target: TilePos {
                x: target.x,
                y: target.y,
            },
            accepted: false,
            source_build_job_id: None,
            wall_work_ms: 0,
        }
    } else {
        JobMetadata::None
    };
    queue_job(
        colony,
        ctx.now_ms,
        kind,
        JobRequester::Player,
        assigned_cat,
        metadata,
    );
    ok()
}

fn dispatch_scout(
    colony: &mut ColonyRuntime,
    mission: ScoutMission,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    let Some(cat_id) = select_best_scout(colony) else {
        return fail("No available scout.");
    };
    queue_job(
        colony,
        ctx.now_ms,
        JobKind::Explore,
        JobRequester::Player,
        Some(cat_id),
        JobMetadata::Scout {
            mission,
            target: None,
            destination: None,
            accepted: false,
            found: false,
        },
    );
    colony.last_player_activity_at = Some(ctx.now_ms);
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
    let candidates = colony
        .cats
        .iter()
        .filter(|cat| cat.death_time.is_none())
        .map(|cat| crate::elections::ElectionCandidate {
            id: cat.id.clone(),
            leadership: cat.stats.leadership,
        })
        .collect::<Vec<_>>();
    if !crate::elections::candidates_for_unbarred(&candidates)
        .iter()
        .any(|candidate| candidate == cat_id)
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
    let voter = voter_id(ctx);
    if let Some((election_id, target_cat_id)) = colony
        .elections
        .iter()
        .find(|election| election.kind == ElectionKind::VoteKick && election.resolved_at.is_none())
        .map(|election| {
            (
                election.id.clone(),
                election.winner_cat_id.clone().unwrap_or_default(),
            )
        })
    {
        if colony
            .votes
            .iter()
            .any(|vote| vote.election_id == election_id && vote.voter_id == voter)
        {
            // Reconnects and double-clicks are idempotent: one stable player
            // identity contributes at most one signature to this petition.
            return ok();
        }
        colony.votes.push(VoteRuntime {
            id: format!("vote-{}-{}", ctx.now_ms, colony.votes.len() + 1),
            election_id,
            voter_id: voter,
            cat_id: target_cat_id,
            weight: 1.0,
        });
        colony.last_player_activity_at = Some(ctx.now_ms);
        return ok();
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
        voter_id: voter,
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
    site: Option<proto::TilePoint>,
    world_seed: u32,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    let Some(building_type) = proto_to_sim_building_type(building_type) else {
        return fail("That building is not supported by the simulation runtime yet.");
    };
    if !matches!(
        building_type,
        BuildingType::Den
            | BuildingType::WaterBowl
            | BuildingType::Beds
            | BuildingType::HerbGarden
            | BuildingType::Nursery
            | BuildingType::ElderCorner
            | BuildingType::Walls
            | BuildingType::MouseFarm
            | BuildingType::Workshop
            | BuildingType::Field
            | BuildingType::Smithy
            | BuildingType::Barracks
            | BuildingType::FoodStorage
            | BuildingType::AccountingTent
            | BuildingType::WoodCutter
            | BuildingType::StonePrep
            | BuildingType::Woodworking
            | BuildingType::Clothier
            | BuildingType::Tannery
            | BuildingType::ResearchHut
            | BuildingType::Smelter
            | BuildingType::Mill
            | BuildingType::Sawmill
            | BuildingType::School
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
    if let upgrade_tree::BuildingPlacementResearch::Requires { node_name, .. } =
        upgrade_tree::building_placement_research(&colony.upgrade_tree, building_type.as_str())
    {
        return fail(format!("Research {node_name} before construction."));
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
    let (building_id, site) = if let Some(site) = site {
        let site = TilePos {
            x: site.x,
            y: site.y,
        };
        match crate::world_tick::commit_player_scaffold(
            colony,
            site,
            building_type,
            world_seed,
            ctx.now_ms,
        ) {
            Ok(building_id) => (Some(building_id), Some(site)),
            Err(message) => return fail(message),
        }
    } else {
        // Compatibility path for old saved/reconnecting clients: the runtime keeps
        // choosing a deterministic site and pays at break-ground as before.
        (None, None)
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
            building_id,
            site,
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

fn research_node(
    colony: &mut ColonyRuntime,
    node_id: &str,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    let result = upgrade_tree::cat_purchase(&colony.upgrade_tree, node_id);
    if !result.ok {
        return fail("That technology is locked, owned, or lacks research points.");
    }
    colony.upgrade_tree = result.state;
    colony.last_player_activity_at = Some(ctx.now_ms);
    let node_name = upgrade_tree::get_node(node_id).map_or(node_id, |node| node.name);
    append_event(
        colony,
        ctx.now_ms,
        EventKind::ResearchUnlocked,
        format!("The scholars completed {node_name}!"),
    );
    ok()
}

fn offer_tithe(colony: &mut ColonyRuntime, ctx: &ActionCtx) -> proto::ActionResult {
    if !has_complete_building(colony, BuildingType::Shrine) {
        return fail("This village needs a completed shrine.");
    }
    let population = colony
        .cats
        .iter()
        .filter(|cat| cat.death_time.is_none())
        .count() as f64;
    let food_reserve = (population * TITHE_FOOD_RESERVE_PER_CAT).max(TITHE_FOOD_RESERVE_FLOOR);
    let food = if population > 0.0
        && colony.resources.food >= food_reserve + f64::from(TITHE_FOOD_AMOUNT)
    {
        TITHE_FOOD_AMOUNT
    } else {
        0
    };
    let refined = if colony.resources.refined >= f64::from(TITHE_REFINED_AMOUNT) {
        TITHE_REFINED_AMOUNT
    } else {
        0
    };
    let blessings = u32::from(food > 0) + u32::from(refined > 0);
    if blessings == 0 {
        return fail("No safe food or refined surplus is available.");
    }

    colony.resources.food -= f64::from(food);
    colony.resources.refined -= f64::from(refined);
    colony.global_upgrade_points += f64::from(blessings);
    colony.last_tithe_at = Some(ctx.now_ms);
    reconcile_colony_stockpiles(colony);
    colony.last_player_activity_at = Some(ctx.now_ms);
    append_event(
        colony,
        ctx.now_ms,
        EventKind::Tithe,
        format!("The players offered surplus stores (+{blessings} blessings)."),
    );
    ok()
}

fn offer_materials(colony: &mut ColonyRuntime, ctx: &ActionCtx) -> proto::ActionResult {
    if !has_complete_building(colony, BuildingType::Shrine) {
        return fail("This village needs a completed shrine.");
    }
    if colony.resources.materials
        < OFFERING_MATERIALS_RESERVE + f64::from(OFFERING_MATERIALS_AMOUNT)
        || visible_offering_materials(colony) + f64::EPSILON < f64::from(OFFERING_MATERIALS_AMOUNT)
    {
        return fail("Not enough physically stored surplus materials for an offering.");
    }
    if active_or_queued_jobs(colony)
        .iter()
        .any(|job| matches!(job.kind, JobKind::CarryOffering | JobKind::PerformOffering))
    {
        return fail("A material offering is already in progress.");
    }
    let Some(cat_id) = select_best_cat(colony, Some(CatSpecialization::Ritualist)) else {
        return fail("No available ritualist.");
    };
    let from = colony
        .cats
        .iter()
        .find(|cat| cat.id == cat_id)
        .map(|cat| crate::movement::WorldPos {
            x: cat.position.x + f64::from(colony.anchor.x),
            y: cat.position.y + f64::from(colony.anchor.y),
        })
        .unwrap_or_else(|| crate::movement::WorldPos {
            x: f64::from(colony.anchor.x),
            y: f64::from(colony.anchor.y),
        });
    let Some(metadata) = material_offering_metadata(colony, from, ctx.now_ms) else {
        return fail("No reachable physical material pile is available.");
    };
    queue_job(
        colony,
        ctx.now_ms,
        JobKind::CarryOffering,
        JobRequester::Player,
        Some(cat_id),
        metadata,
    );
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

fn haul_gather_spot(
    colony: &mut ColonyRuntime,
    stockpile_id: &str,
    cat_id: Option<&str>,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    let Some(spot) = colony
        .gather_spots
        .iter()
        .find(|spot| spot.stockpile_id == stockpile_id)
    else {
        return fail("Unknown gather spot.");
    };
    let has_contents = colony
        .stockpiles
        .iter()
        .find(|pile| pile.id == stockpile_id)
        .is_some_and(|pile| stockpiles::resource_amount(&pile.contents, spot.kind) > 0.0);
    if !has_contents {
        return fail("That gather spot is empty.");
    }
    if active_or_queued_jobs(colony).iter().any(|job| {
        job.kind == JobKind::HaulGatherSpot
            && matches!(
                &job.metadata,
                JobMetadata::GatherHaul { stockpile_id: target, .. } if target == stockpile_id
            )
    }) {
        return fail("That gather spot already has a mover.");
    }

    let carrier = if let Some(cat_id) = cat_id {
        let Some(index) = colony
            .cats
            .iter()
            .position(|cat| cat.id == cat_id && cat.death_time.is_none())
        else {
            return fail("That cat is not available.");
        };
        if !cat_can_take_assignment(colony, index) {
            return fail("That cat is busy.");
        }
        cat_id.to_owned()
    } else {
        let Some(cat_id) = select_best_cat(colony, None) else {
            return fail("No available carrier.");
        };
        cat_id
    };

    queue_job(
        colony,
        ctx.now_ms,
        JobKind::HaulGatherSpot,
        JobRequester::Player,
        Some(carrier),
        JobMetadata::GatherHaul {
            stockpile_id: stockpile_id.to_owned(),
            site: None,
            accepted: false,
        },
    );
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
        release_farm_worker(colony, cat_id);
        for building in &mut colony.buildings {
            if building.assigned_cat.as_deref() == Some(cat_id) {
                building.assigned_cat = None;
                building.automated_by = None;
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
            && production::building_staff_cap(building.building_type) > 0
    }) else {
        return fail("That building cannot take a worker.");
    };

    let displaced_farmer = (colony.buildings[building_index].building_type == BuildingType::Field)
        .then(|| colony.buildings[building_index].assigned_cat.clone())
        .flatten()
        .filter(|assigned| assigned != cat_id);
    if let Some(displaced) = displaced_farmer {
        release_farm_worker(colony, &displaced);
    }
    release_farm_worker(colony, cat_id);
    for building in &mut colony.buildings {
        if building.assigned_cat.as_deref() == Some(cat_id) {
            building.assigned_cat = None;
            building.automated_by = None;
        }
    }
    colony.buildings[building_index].assigned_cat = Some(cat_id.to_owned());
    colony.buildings[building_index].automated_by = None;
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
    if !colony.cats.iter().any(|cat| {
        cat.id == cat_id && cat.death_time.is_none() && can_work(get_life_stage(cat.age_hours))
    }) {
        return fail("That cat is not old enough and available to hold office.");
    }
    let prerequisite = prerequisite_for(role);
    if !has_complete_building(colony, prerequisite.building) {
        return fail(format!(
            "A completed {} is required for this office.",
            prerequisite.building.as_str().replace('_', " ")
        ));
    }
    if !upgrade_tree::is_owned(&colony.upgrade_tree, prerequisite.upgrade_node) {
        return fail(format!(
            "The {} technology is required for this office.",
            prerequisite.upgrade_node.replace('_', " ")
        ));
    }

    let vacated_roles = colony
        .officers
        .iter()
        .filter_map(|(filled_role, holder)| {
            (holder == cat_id && *filled_role != role).then_some(*filled_role)
        })
        .collect::<Vec<_>>();
    colony
        .officers
        .retain(|_, holder| holder.as_str() != cat_id);
    colony.officers.insert(role, cat_id.to_owned());
    for vacated in vacated_roles {
        release_role_automation(colony, vacated, ctx.now_ms);
    }
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
    release_role_automation(colony, role, ctx.now_ms);
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

fn set_cat_labor_preference(
    colony: &mut ColonyRuntime,
    cat_id: &str,
    labor: Labor,
    enabled: bool,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    let Some(cat) = colony
        .cats
        .iter_mut()
        .find(|cat| cat.id == cat_id && cat.death_time.is_none())
    else {
        return fail("That cat is not available.");
    };
    if enabled {
        cat.preferred_labors.insert(labor);
    } else {
        cat.preferred_labors.remove(&labor);
    }
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

fn edit_production_queue(
    colony: &mut ColonyRuntime,
    building_id: &str,
    edit: &proto::ProductionQueueEdit,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    let Some(building_index) = colony.buildings.iter().position(|building| {
        building.id == building_id
            && building.construction_progress >= 100
            && !crate::world_tick::available_production_recipes(building.building_type).is_empty()
    }) else {
        return fail("That building has no editable production queue.");
    };
    let building_type = colony.buildings[building_index].building_type;

    match edit {
        proto::ProductionQueueEdit::Add { recipe_id, repeat } => {
            if !crate::world_tick::production_recipe_availability(colony, building_type, recipe_id)
                .is_some_and(|recipe| recipe.available)
            {
                return fail("That recipe is not available at this station.");
            }
            let building = &mut colony.buildings[building_index];
            if building.production_queue.len() >= 32 {
                return fail("That production queue is full.");
            }
            building
                .production_queue
                .push(crate::world_tick::ProductionQueueEntry {
                    recipe_id: recipe_id.clone(),
                    repeat: *repeat,
                });
        }
        proto::ProductionQueueEdit::Remove { index } => {
            let building = &mut colony.buildings[building_index];
            if *index >= building.production_queue.len() {
                return fail("That queue entry no longer exists.");
            }
            building.production_queue.remove(*index);
            if *index == 0 {
                building.production_progress = 0.0;
            }
        }
        proto::ProductionQueueEdit::Move { index, direction } => {
            let building = &mut colony.buildings[building_index];
            let target = match direction {
                proto::QueueMoveDirection::Up => index.checked_sub(1),
                proto::QueueMoveDirection::Down => index.checked_add(1),
            };
            let Some(target) = target.filter(|target| *target < building.production_queue.len())
            else {
                return fail("That queue entry cannot move farther.");
            };
            building.production_queue.swap(*index, target);
            if *index == 0 || target == 0 {
                building.production_progress = 0.0;
            }
        }
        proto::ProductionQueueEdit::SetRepeat { index, repeat } => {
            let building = &mut colony.buildings[building_index];
            let Some(entry) = building.production_queue.get_mut(*index) else {
                return fail("That queue entry no longer exists.");
            };
            entry.repeat = *repeat;
        }
        proto::ProductionQueueEdit::SetPaused { paused } => {
            let building = &mut colony.buildings[building_index];
            building.production_paused = *paused;
        }
    }
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

fn designate_farm(
    colony: &mut ColonyRuntime,
    world_seed: u32,
    a: proto::TilePoint,
    b: proto::TilePoint,
    crop: proto::CropKind,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    let gate_was_connected = village_exterior_is_road_connected(colony, world_seed);
    let rect = zones::normalize_rect(
        f64::from(a.x),
        f64::from(a.y),
        f64::from(b.x),
        f64::from(b.y),
    );
    if farming::rect_tiles(rect).any(|tile| {
        !colony.revealed_tiles.contains(&TilePos {
            x: tile.x,
            y: tile.y,
        })
    }) {
        return fail("Farm plots must be revealed by a returning scout first.");
    }
    let occupied_tiles = occupied_farm_tiles(colony, rect, world_seed);
    let placement = farming::validate_placement(
        rect,
        &colony.farms,
        |tile| {
            colony.claimed_tiles.contains(&TilePos {
                x: tile.x,
                y: tile.y,
            })
        },
        |tile| {
            inside_village_interior(
                colony,
                TilePos {
                    x: tile.x,
                    y: tile.y,
                },
            )
        },
        |tile| {
            occupied_tiles.contains(&TilePos {
                x: tile.x,
                y: tile.y,
            })
        },
        |tile| {
            let tile = TilePos {
                x: tile.x,
                y: tile.y,
            };
            crate::world_tick::tile_farm_fertility(world_seed, tile, colony.world_tiles.get(&tile))
        },
    );
    let fertility = match placement {
        Ok(fertility) => fertility,
        Err(error) => {
            let message = match error {
                farming::FarmPlacementError::InvalidRect => "Invalid farm rectangle.",
                farming::FarmPlacementError::TooLarge => "Farm plots are limited to 8x8 tiles.",
                farming::FarmPlacementError::LimitReached => {
                    "This colony already has 16 farm plots."
                }
                farming::FarmPlacementError::OutsideClaim => {
                    "Farm plots must stay on claimed tiles."
                }
                farming::FarmPlacementError::VillageInterior => {
                    "Farm plots belong outside the walled village."
                }
                farming::FarmPlacementError::Occupied => "A farm tile is occupied.",
                farming::FarmPlacementError::Overlap => "Farm plots cannot overlap.",
                farming::FarmPlacementError::Barren => {
                    "Every farm tile must have positive fertility."
                }
            };
            return fail(message);
        }
    };
    if !farm_rect_touches_claim_boundary(colony, rect) {
        return fail("Farm plots must connect to the claimed exterior boundary.");
    }
    let plot_center = crate::movement::WorldPos {
        x: f64::from(rect.x1 + rect.x2) / 2.0,
        y: f64::from(rect.y1 + rect.y2) / 2.0,
    };
    if !farm_designation_route_is_reachable(
        colony,
        world_seed,
        crate::movement::WorldPos {
            x: f64::from(colony.anchor.x),
            y: f64::from(colony.anchor.y),
        },
        rect,
    ) {
        return fail("Farm plots need a reachable route from the village.");
    }
    let id = format!("farm-{}-{}", ctx.now_ms, colony.farms.len() + 1);
    colony.farms.push(FarmPlot {
        id,
        rect,
        crop: proto_to_sim_crop(crop),
        planted_at: ctx.now_ms,
        stage: FarmStage::Soil,
        growth_hours: 0.0,
        fertility,
        worker_id: None,
        work_phase: farming::FarmWorkPhase::WaitingForWorker,
        pending_output: 0.0,
    });
    for y in rect.y1..=rect.y2 {
        for x in rect.x1..=rect.x2 {
            colony.agricultural_tiles.insert(TilePos { x, y });
        }
    }
    let created = colony.farms.last().cloned().expect("farm was just pushed");
    let selected_handoff = legal_farm_gather_spots(colony, &created, world_seed)
        .into_iter()
        .find(|candidate| {
            let mut projected = colony.clone();
            ensure_farm_gather_spot_at(&mut projected, &created, world_seed, *candidate).is_some()
                && (!gate_was_connected
                    || village_exterior_is_road_connected(&projected, world_seed))
                && farm_route_is_reachable(
                    &projected,
                    world_seed,
                    crate::movement::WorldPos {
                        x: f64::from(projected.anchor.x),
                        y: f64::from(projected.anchor.y),
                    },
                    plot_center,
                )
        });
    let Some(gather_id) = selected_handoff
        .and_then(|spot| ensure_farm_gather_spot_at(colony, &created, world_seed, spot))
    else {
        colony.farms.pop();
        for y in rect.y1..=rect.y2 {
            for x in rect.x1..=rect.x2 {
                colony.agricultural_tiles.remove(&TilePos { x, y });
            }
        }
        return fail("Farm plots need a reachable adjacent crop gather spot.");
    };
    // Agricultural tiles sit outside the retained village wall. Re-check after both
    // plot and handoff have been excluded from the enclosure so validation matches the
    // exact topology the worker will actually traverse.
    if gate_was_connected && !village_exterior_is_road_connected(colony, world_seed)
        || !farm_route_is_reachable(
            colony,
            world_seed,
            crate::movement::WorldPos {
                x: f64::from(colony.anchor.x),
                y: f64::from(colony.anchor.y),
            },
            plot_center,
        )
    {
        if let Some(gather_rect) = colony
            .stockpiles
            .iter()
            .find(|pile| pile.id == gather_id)
            .map(|pile| pile.rect)
        {
            for y in gather_rect.y1..=gather_rect.y2 {
                for x in gather_rect.x1..=gather_rect.x2 {
                    colony.agricultural_tiles.remove(&TilePos { x, y });
                }
            }
        }
        colony
            .gather_spots
            .retain(|spot| spot.stockpile_id != gather_id);
        colony.stockpiles.retain(|pile| pile.id != gather_id);
        colony.farms.pop();
        for y in rect.y1..=rect.y2 {
            for x in rect.x1..=rect.x2 {
                colony.agricultural_tiles.remove(&TilePos { x, y });
            }
        }
        return fail("Farm plots must preserve the shrine-connected gate and worker route.");
    }
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

fn clear_farm(colony: &mut ColonyRuntime, plot_id: &str, ctx: &ActionCtx) -> proto::ActionResult {
    let Some(plot) = colony.farms.iter().find(|plot| plot.id == plot_id) else {
        return fail("Unknown farm plot.");
    };
    let plot_rect = plot.rect;
    let gather_id = farm_gather_spot_id(plot_id);
    let gather_rect = colony
        .stockpiles
        .iter()
        .find(|pile| pile.id == gather_id)
        .map(|pile| pile.rect);
    let gathered_output = colony
        .stockpiles
        .iter()
        .find(|pile| pile.id == gather_id)
        .map_or(0.0, |pile| {
            stockpiles::resource_amount(
                &pile.contents,
                match plot.crop {
                    farming::CropKind::Catnip => stockpiles::ResourceKind::Catnip,
                    farming::CropKind::Grain => stockpiles::ResourceKind::Grain,
                    farming::CropKind::Herb => stockpiles::ResourceKind::Herbs,
                },
            )
        });
    if plot.pending_output > 0.0
        || gathered_output > 0.0
        || colony.cats.iter().any(|cat| {
            cat.carrying
                .as_ref()
                .and_then(|cargo| cargo.source_gather_spot.as_deref())
                .is_some_and(|marker| {
                    marker.starts_with(&format!("farm-out|{plot_id}|"))
                        || marker == gather_id.as_str()
                })
        })
    {
        return fail("This farm still has produce awaiting delivery.");
    }
    let worker_id = plot.worker_id.clone();
    let before = colony.farms.len();
    colony.farms.retain(|plot| plot.id != plot_id);
    if colony.farms.len() == before {
        return fail("Unknown farm plot.");
    }
    if let Some(worker_id) = worker_id {
        release_farm_worker(colony, &worker_id);
    }
    for y in plot_rect.y1..=plot_rect.y2 {
        for x in plot_rect.x1..=plot_rect.x2 {
            colony.agricultural_tiles.remove(&TilePos { x, y });
        }
    }
    if let Some(rect) = gather_rect {
        for y in rect.y1..=rect.y2 {
            for x in rect.x1..=rect.x2 {
                colony.agricultural_tiles.remove(&TilePos { x, y });
            }
        }
    }
    crate::world_tick::cancel_gather_haul_jobs_for_spot(colony, &gather_id, ctx.now_ms);
    colony
        .gather_spots
        .retain(|spot| spot.stockpile_id != gather_id);
    colony.stockpiles.retain(|pile| pile.id != gather_id);
    reconcile_colony_stockpiles(colony);
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

/// Designate a player stockpile over the rect `a..b` accepting `accepts`. Reuses the
/// zone edge cap; enforces a per-colony designated-pile limit and a non-empty accept set.
fn designate_stockpile(
    colony: &mut ColonyRuntime,
    a: proto::TilePoint,
    b: proto::TilePoint,
    accepts: &[proto::ResourceKind],
    world_seed: u32,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    if accepts.is_empty() {
        return fail("A stockpile must accept at least one resource.");
    }
    if accepts
        .iter()
        .any(|kind| !kind.is_physical_stockpile_good())
    {
        return fail(
            "Stockpiles accept only physical goods; Blessings are divine favor and are never hauled or stored in piles.",
        );
    }
    let rect = zones::normalize_rect(
        f64::from(a.x),
        f64::from(a.y),
        f64::from(b.x),
        f64::from(b.y),
    );
    let Some((width, height)) = stockpiles::rect_dimensions(rect) else {
        return fail("The stockpile rectangle is invalid.");
    };
    if width > i64::from(stockpiles::STOCKPILE_MAX_EDGE)
        || height > i64::from(stockpiles::STOCKPILE_MAX_EDGE)
    {
        return fail(format!(
            "Stockpiles are limited to {}x{} tiles.",
            stockpiles::STOCKPILE_MAX_EDGE,
            stockpiles::STOCKPILE_MAX_EDGE
        ));
    }
    let gather_ids = colony
        .gather_spots
        .iter()
        .map(|spot| spot.stockpile_id.as_str())
        .collect::<HashSet<_>>();
    let designated = colony
        .stockpiles
        .iter()
        .filter(|pile| {
            !pile.is_shrine()
                && !pile.is_station_local()
                && !gather_ids.contains(pile.id.as_str())
                && !colony
                    .stock_ledger
                    .steward_managed_piles
                    .contains_key(&pile.id)
        })
        .count();
    if designated >= stockpiles::MAX_DESIGNATED_STOCKPILES {
        return fail(format!(
            "You already have {} stockpiles.",
            stockpiles::MAX_DESIGNATED_STOCKPILES
        ));
    }
    if let Some(error) =
        crate::world_tick::stockpile_placement_error(colony, rect, world_seed, true)
    {
        return fail(error.message());
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
    if matches!(
        stockpile_id,
        stockpiles::SHRINE_STOCKPILE_ID | stockpiles::GENERAL_STOREHOUSE_ID
    ) {
        return fail("The seeded village storehouse cannot be removed.");
    }
    if is_farm_gather_spot_id(stockpile_id) {
        return fail("A maintained farm handoff is removed by clearing its farm plot.");
    }
    if colony
        .gather_spots
        .iter()
        .any(|spot| spot.stockpile_id == stockpile_id)
    {
        return fail("Use the gather-spot removal control for this typed pile.");
    }
    if colony
        .stockpiles
        .iter()
        .any(|pile| pile.id == stockpile_id && pile.is_station_local())
    {
        return fail("Station-local storage cannot be removed directly.");
    }
    if let Some(managed) = colony.stock_ledger.steward_managed_piles.get(stockpile_id) {
        if managed.active {
            return fail(
                "This limited pile is actively managed by the Steward; vacate that office first.",
            );
        }
        let contains_goods = colony
            .stockpiles
            .iter()
            .find(|pile| pile.id == stockpile_id)
            .is_some_and(|pile| {
                stockpiles::ResourceKind::ALL
                    .iter()
                    .any(|kind| stockpiles::resource_amount(&pile.contents, *kind) > f64::EPSILON)
            });
        if contains_goods {
            return fail(
                "This dormant Steward pile still contains goods; move them before removing it.",
            );
        }
        colony
            .stock_ledger
            .steward_managed_piles
            .remove(stockpile_id);
    }
    crate::world_tick::cancel_stockpile_balance_jobs_for_pile(colony, stockpile_id, ctx.now_ms);
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
/// placed on mapped, revealed ground — including outside the claimed village once a
/// scout has revealed it, unlike a general `DesignateStockpile`'s intent — since it
/// reuses the same `Stockpile` machinery unchanged: deposit routing/reconcile/capacity
/// all apply exactly as for any other pile. Only resources with a maintained physical
/// carrier kind are accepted, including the three farm crops.
fn designate_gather_spot(
    colony: &mut ColonyRuntime,
    a: proto::TilePoint,
    b: proto::TilePoint,
    kind: proto::ResourceKind,
    world_seed: u32,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    if !matches!(
        kind,
        proto::ResourceKind::Food
            | proto::ResourceKind::Water
            | proto::ResourceKind::Materials
            | proto::ResourceKind::Logs
            | proto::ResourceKind::Catnip
            | proto::ResourceKind::Grain
            | proto::ResourceKind::Herbs
    ) {
        return fail(
            "Gather spots only collect food, water, materials, logs, catnip, grain, or herbs.",
        );
    }
    let rect = zones::normalize_rect(
        f64::from(a.x),
        f64::from(a.y),
        f64::from(b.x),
        f64::from(b.y),
    );
    let Some((width, height)) = stockpiles::rect_dimensions(rect) else {
        return fail("The gather-spot rectangle is invalid.");
    };
    if width > i64::from(stockpiles::GATHER_SPOT_MAX_EDGE)
        || height > i64::from(stockpiles::GATHER_SPOT_MAX_EDGE)
    {
        return fail(format!(
            "Gather spots are limited to {}x{} tiles.",
            stockpiles::GATHER_SPOT_MAX_EDGE,
            stockpiles::GATHER_SPOT_MAX_EDGE
        ));
    }
    let player_gather_spots = colony
        .gather_spots
        .iter()
        .filter(|spot| !is_farm_gather_spot_id(&spot.stockpile_id))
        .count();
    if player_gather_spots >= stockpiles::MAX_GATHER_SPOTS {
        return fail(format!(
            "You already have {} gather spots.",
            stockpiles::MAX_GATHER_SPOTS
        ));
    }
    if let Some(error) =
        crate::world_tick::stockpile_placement_error(colony, rect, world_seed, false)
    {
        return fail(error.message());
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
        purpose: stockpiles::GatherSpotPurpose::General,
    });
    reconcile_colony_stockpiles(colony);
    colony.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

/// Create a durable one-tile food gather spot on a physically walkable bank.
/// The player may click the bank itself or its adjacent water cell; water clicks
/// resolve in stable N/E/S/W order so signed replays remain deterministic.
fn designate_fishing_spot(
    colony: &mut ColonyRuntime,
    at: proto::TilePoint,
    world_seed: u32,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    if colony.gather_spots.len() >= stockpiles::MAX_GATHER_SPOTS {
        return fail(format!(
            "You already have {} gather spots.",
            stockpiles::MAX_GATHER_SPOTS
        ));
    }
    let clicked = TilePos { x: at.x, y: at.y };
    let mut candidates = vec![clicked];
    if crate::world_tick::tile_has_water(colony.world_tiles.get(&clicked)) {
        candidates = [
            TilePos {
                x: at.x,
                y: at.y - 1,
            },
            TilePos {
                x: at.x + 1,
                y: at.y,
            },
            TilePos {
                x: at.x,
                y: at.y + 1,
            },
            TilePos {
                x: at.x - 1,
                y: at.y,
            },
        ]
        .into_iter()
        .collect();
    }
    let site = candidates.into_iter().find(|site| {
        crate::world_tick::is_reachable_fishing_shore(colony, *site, world_seed)
            && crate::world_tick::stockpile_placement_error(
                colony,
                zones::normalize_rect(
                    f64::from(site.x),
                    f64::from(site.y),
                    f64::from(site.x),
                    f64::from(site.y),
                ),
                world_seed,
                false,
            )
            .is_none()
    });
    let Some(site) = site else {
        return fail("Choose a revealed, clear shoreline tile beside water.");
    };
    let rect = zones::normalize_rect(
        f64::from(site.x),
        f64::from(site.y),
        f64::from(site.x),
        f64::from(site.y),
    );
    let id = format!("gather-fish-{}-{}", ctx.now_ms, colony.stockpiles.len() + 1);
    let habitat = crate::world_tick::fishing_habitat_tile(colony, site)
        .expect("a validated fishing shore has adjacent water");
    colony
        .fish_habitats
        .entry(habitat)
        .or_insert(stockpiles::FishPopulation {
            stock: stockpiles::FISH_POPULATION_CAPACITY,
            capacity: stockpiles::FISH_POPULATION_CAPACITY,
            last_replenished_at_ms: ctx.now_ms,
        });
    colony.stockpiles.push(stockpiles::Stockpile {
        id: id.clone(),
        rect,
        accepts: std::iter::once(stockpiles::ResourceKind::Fish).collect(),
        contents: entities::Resources::default(),
    });
    colony.gather_spots.push(stockpiles::GatherSpot {
        stockpile_id: id,
        kind: stockpiles::ResourceKind::Fish,
        expires_at_ms: i64::MAX,
        purpose: stockpiles::GatherSpotPurpose::Fishing,
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
    if colony
        .farms
        .iter()
        .any(|plot| farm_gather_spot_id(&plot.id) == stockpile_id)
    {
        return fail("A maintained farm handoff is removed by clearing its farm plot.");
    }
    if !colony
        .gather_spots
        .iter()
        .any(|spot| spot.stockpile_id == stockpile_id)
    {
        return fail("Unknown gather spot.");
    }
    crate::world_tick::cancel_fishing_jobs_for_spot(colony, stockpile_id, ctx.now_ms);
    crate::world_tick::cancel_stockpile_balance_jobs_for_pile(colony, stockpile_id, ctx.now_ms);
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
    if colony.items.pristine_count(item) < count {
        return fail("Not enough goods.");
    }
    if count > trader::max_item_units_per_load(item) {
        return fail("That load is too heavy for one caravan transfer.");
    }
    let functional_kind = functional_resource_for_item(item);
    if let Some(kind) = functional_kind
        && visible_resource_amount(colony, kind) + f64::EPSILON < f64::from(count)
    {
        return fail("The identified equipment is not in player-visible storage.");
    }

    let effects = upgrade_tree::resolve_effects(colony.upgrade_tree.owned_node_ids.iter());
    let payout = trader::trader_buy_price(item, count) * effects.trade_value_mult;
    if let Some(kind) = functional_kind {
        let removed_physical = deduct_visible_resource(colony, kind, f64::from(count));
        debug_assert!(removed_physical, "checked physical availability above");
    }
    let removed = colony.items.remove_pristine(item, count);
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

fn functional_resource_for_item(item: Item) -> Option<stockpiles::ResourceKind> {
    match item.kind {
        ItemKind::Tool => Some(stockpiles::ResourceKind::Tools),
        ItemKind::Weapon => Some(stockpiles::ResourceKind::Weapons),
        ItemKind::Armor => Some(stockpiles::ResourceKind::Armor),
        _ => None,
    }
}

fn repair_recipe(item: Item) -> (BuildingType, stockpiles::ResourceKind) {
    let building = match item_workshop_id(item) {
        "woodworking" => BuildingType::Woodworking,
        "smithy" => BuildingType::Smithy,
        "clothier" => BuildingType::Clothier,
        "tannery" => BuildingType::Tannery,
        _ => BuildingType::StonePrep,
    };
    let resource = match item.material {
        Material::Wood => stockpiles::ResourceKind::Planks,
        Material::Stone | Material::Clay | Material::Gem | Material::Bone => {
            stockpiles::ResourceKind::Blocks
        }
        Material::Metal => stockpiles::ResourceKind::Metal,
        Material::Fibre => stockpiles::ResourceKind::Cloth,
        Material::Leather => stockpiles::ResourceKind::Leather,
    };
    (building, resource)
}

fn visible_resource_amount(colony: &ColonyRuntime, kind: stockpiles::ResourceKind) -> f64 {
    colony
        .stockpiles
        .iter()
        .filter(|pile| !pile.is_station_local())
        .map(|pile| stockpiles::resource_amount(&pile.contents, kind))
        .sum()
}

fn deduct_visible_resource(
    colony: &mut ColonyRuntime,
    kind: stockpiles::ResourceKind,
    amount: f64,
) -> bool {
    if amount <= 0.0 {
        return true;
    }
    if stockpiles::resource_amount(&colony.resources, kind) + f64::EPSILON < amount
        || visible_resource_amount(colony, kind) + f64::EPSILON < amount
    {
        return false;
    }
    let mut indices = colony
        .stockpiles
        .iter()
        .enumerate()
        .filter(|(_, pile)| !pile.is_station_local())
        .map(|(index, pile)| (pile.id.clone(), index))
        .collect::<Vec<_>>();
    indices.sort_by(|left, right| left.0.cmp(&right.0));
    let mut remaining = amount;
    for (_, index) in indices {
        let available = stockpiles::resource_amount(&colony.stockpiles[index].contents, kind);
        let taken = available.min(remaining);
        stockpiles::add_resource(&mut colony.stockpiles[index].contents, kind, -taken);
        remaining -= taken;
        if remaining <= f64::EPSILON {
            break;
        }
    }
    stockpiles::add_resource(&mut colony.resources, kind, -amount);
    true
}

/// Signed, finite repair at the item's existing production workshop. The station must
/// be complete and have a living assigned worker, whether assigned manually or by its
/// officer. One real material is removed from visible stock before condition changes.
fn repair_item(colony: &mut ColonyRuntime, item_id: &str, ctx: &ActionCtx) -> proto::ActionResult {
    let Some(instance) = colony.items.instance(item_id).cloned() else {
        return fail("Unknown item.");
    };
    if instance.is_pristine() {
        return fail("That item does not need repair.");
    }
    let (building_type, resource_kind) = repair_recipe(instance.item);
    let staffed = colony.buildings.iter().any(|building| {
        building.building_type == building_type
            && building.is_complete
            && building.construction_progress >= 100
            && building.assigned_cat.as_deref().is_some_and(|cat_id| {
                colony
                    .cats
                    .iter()
                    .any(|cat| cat.id == cat_id && cat.death_time.is_none())
            })
    });
    if !staffed {
        return fail("The appropriate workshop needs a living assigned worker.");
    }
    if !deduct_visible_resource(colony, resource_kind, 1.0) {
        return fail("The repair material must be in player-visible storage.");
    }
    let effects = upgrade_tree::resolve_effects(colony.upgrade_tree.owned_node_ids.iter());
    let durability_mult = effects
        .building(item_workshop_id(instance.item))
        .durability_mult;
    let repaired = colony.items.repair(item_id, durability_mult);
    debug_assert!(repaired, "validated damaged item above");
    colony.last_player_activity_at = Some(ctx.now_ms);
    append_event(
        colony,
        ctx.now_ms,
        EventKind::Production,
        format!("The workshop repaired {item_id} with one {resource_kind:?}."),
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
    // Keep the raid active through phase 36 when this was the killing click. That
    // phase owns the atomic terminal cleanup (corpse removal, telemetry/threat reset)
    // and the one Repelled event. Clearing only `active_raid` here stranded dead
    // RaiderRuntime records forever because phase 36 then had no raid id to finish.
    ok()
}

fn build_road(
    colony: &mut ColonyRuntime,
    a: proto::TilePoint,
    b: proto::TilePoint,
    world_seed: u32,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    if [a.x, a.y, b.x, b.y]
        .iter()
        .any(|&coord| i64::from(coord).abs() > 1_000)
    {
        return fail("Invalid road endpoints.");
    }
    let distance =
        (i64::from(b.x) - i64::from(a.x)).abs() + (i64::from(b.y) - i64::from(a.y)).abs();
    if distance > 24 {
        return fail("Roads are limited to 24 tiles per build.");
    }

    let path = road_path(a, b);
    if path.len() > 24 {
        return fail("Roads are limited to 24 tiles per build.");
    }
    if !crate::world_tick::road_path_attaches_to_shrine(colony, &path) {
        return fail("A new road must attach to the shrine-connected road network.");
    }
    for &pos in &path {
        if !colony.world_tiles.contains_key(&pos) {
            return fail("Roads can only be built on mapped terrain.");
        }
        if let Some(error) = crate::world_tick::road_placement_error(colony, pos, world_seed) {
            return fail(error.message());
        }
    }
    let new_tiles = path
        .iter()
        .filter(|&&pos| {
            colony
                .world_tiles
                .get(&pos)
                .is_some_and(|tile| tile.overlay_feature.as_deref() != Some("road_built"))
                && !crate::world_tick::tile_is_shrine_footprint(colony, pos)
        })
        .count();
    if colony.resources.materials < new_tiles as f64 {
        return fail(format!(
            "Not enough materials ({} needed, one per tile).",
            new_tiles
        ));
    }

    let mut paved = 0u32;
    for pos in path {
        if crate::world_tick::tile_is_shrine_footprint(colony, pos) {
            continue;
        }
        let tile = colony
            .world_tiles
            .get_mut(&pos)
            .expect("road path was prevalidated as mapped");
        if tile.overlay_feature.as_deref() == Some("road_built") {
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
    _session_id: &str,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    let name = name.trim();
    if name.is_empty() {
        return fail("Village name is required.");
    }
    if name.chars().count() > MAX_VILLAGE_NAME_CHARS || name.chars().any(char::is_control) {
        return fail("Village names must be 48 visible characters or fewer.");
    }
    if let Some(existing) = world.colonies.iter().find(|colony| {
        colony.kind == VillageKind::Personal
            && colony.owner_player_id.as_deref() == Some(ctx.player_id.as_str())
    }) {
        return ok_for_colony(&existing.id);
    }
    if !world
        .colonies
        .iter()
        .any(|colony| colony.kind == VillageKind::Global)
    {
        return fail("The global village is unavailable.");
    }

    let Some(id) = personal_village_id(world, &ctx.player_id) else {
        return fail("No unique personal-village identity is available.");
    };
    let seed = stable_seed(&[
        "idle-cat-forest/personal-village/v1",
        &world.world_seed.to_string(),
        &ctx.player_id,
    ]);
    // Place the new village at a distinct, valid site far from every existing colony so two
    // settlements never stack on the same anchor. Deterministic (RNG-free) site search.
    let existing_anchors: Vec<TilePos> =
        world.colonies.iter().map(|colony| colony.anchor).collect();
    let Some(site) =
        select_personal_village_site(world.world_seed, &ctx.player_id, &existing_anchors)
    else {
        return fail("No safe personal-village site is available.");
    };
    let anchor = site.anchor;
    let mut colony = found_colony_at(world.world_seed, id, ctx.now_ms, seed, anchor);
    colony.name = name.to_owned();
    colony.kind = VillageKind::Personal;
    colony.owner_player_id = Some(ctx.player_id.clone());
    append_event(
        &mut colony,
        ctx.now_ms,
        EventKind::VillageFounded,
        format!("{name} was founded."),
    );
    let colony_id = colony.id.clone();
    world.colonies.push(colony);
    ok_for_colony(&colony_id)
}

fn personal_village_id(world: &WorldState, player_id: &str) -> Option<String> {
    let base = stable_seed(&[
        "idle-cat-forest/personal-village-id/v1",
        &world.world_seed.to_string(),
        player_id,
    ]);
    for suffix in 0..=MAX_PERSONAL_VILLAGE_ID_COLLISIONS {
        let id = if suffix == 0 {
            format!("village-{base:08x}")
        } else {
            format!("village-{base:08x}-{suffix}")
        };
        if !world.colonies.iter().any(|colony| colony.id == id) {
            return Some(id);
        }
    }
    None
}

fn valid_trade_amount(amount: f64) -> bool {
    amount.is_finite() && amount > 0.0 && amount <= MAX_VILLAGE_TRADE_AMOUNT
}

fn offer_village_trade(
    world: &mut WorldState,
    target_colony_id: &str,
    offered_kind: proto::ResourceKind,
    offered_amount: f64,
    requested_kind: proto::ResourceKind,
    requested_amount: f64,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    if !valid_trade_amount(offered_amount) || !valid_trade_amount(requested_amount) {
        return fail("Trade amounts must be finite and positive.");
    }
    if offered_kind == requested_kind {
        return fail("A village trade must exchange different resources.");
    }
    let Some(source_index) = world
        .colonies
        .iter()
        .position(|colony| colony.id == ctx.colony_id)
    else {
        return fail("Village not found.");
    };
    let Some(target_index) = world
        .colonies
        .iter()
        .position(|colony| colony.id == target_colony_id)
    else {
        return fail("Village is not available.");
    };
    if source_index == target_index {
        return fail("Choose another village to trade with.");
    }
    let source = &world.colonies[source_index];
    if !can_control_village(source, &ctx.player_id) {
        return fail("Village is not available.");
    }
    let target = &world.colonies[target_index];
    if !source.known_village_ids.contains(&target.id)
        || !target.known_village_ids.contains(&source.id)
    {
        return fail("The villages have not discovered one another.");
    }
    if source.village_trade_offers.len() >= MAX_OPEN_VILLAGE_TRADE_OFFERS {
        return fail("This village already has too many open trade offers.");
    }
    let offered_kind = proto_to_sim_resource_kind(offered_kind);
    if stockpiles::resource_amount(&source.resources, offered_kind) + f64::EPSILON < offered_amount
    {
        return fail("The offering village lacks those resources.");
    }
    let requested_kind = proto_to_sim_resource_kind(requested_kind);
    let base = stable_seed(&[
        "idle-cat-forest/village-trade/v1",
        &source.id,
        &target.id,
        &ctx.now_ms.to_string(),
        &ctx.player_id,
    ]);
    let offer_id = (0..=MAX_VILLAGE_TRADE_ID_COLLISIONS).find_map(|suffix| {
        let candidate = if suffix == 0 {
            format!("trade-{base:08x}")
        } else {
            format!("trade-{base:08x}-{suffix}")
        };
        (!world
            .colonies
            .iter()
            .any(|colony| colony.village_trade_offers.contains_key(&candidate)))
        .then_some(candidate)
    });
    let Some(offer_id) = offer_id else {
        return fail("No unique trade identity is available.");
    };
    let source = &mut world.colonies[source_index];
    source.village_trade_offers.insert(
        offer_id.clone(),
        VillageTradeOffer {
            id: offer_id,
            from_colony_id: source.id.clone(),
            to_colony_id: target_colony_id.to_owned(),
            offered_kind,
            offered_amount,
            requested_kind,
            requested_amount,
            created_at: ctx.now_ms,
        },
    );
    source.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

fn accept_village_trade(
    world: &mut WorldState,
    offer_id: &str,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    let Some(offer) = world
        .colonies
        .iter()
        .find_map(|colony| colony.village_trade_offers.get(offer_id).cloned())
    else {
        return fail("Trade offer is not available.");
    };
    if offer.to_colony_id != ctx.colony_id {
        return fail("Trade offer is not available.");
    }
    let Some(source_index) = world
        .colonies
        .iter()
        .position(|colony| colony.id == offer.from_colony_id)
    else {
        return fail("Trade offer is not available.");
    };
    let Some(target_index) = world
        .colonies
        .iter()
        .position(|colony| colony.id == offer.to_colony_id)
    else {
        return fail("Trade offer is not available.");
    };
    let (source, target) = two_colonies_mut(&mut world.colonies, source_index, target_index);
    if !can_control_village(target, &ctx.player_id) {
        return fail("Trade offer is not available.");
    }
    if !source.known_village_ids.contains(&target.id)
        || !target.known_village_ids.contains(&source.id)
    {
        return fail("The villages have not discovered one another.");
    }
    if stockpiles::resource_amount(&source.resources, offer.offered_kind) + f64::EPSILON
        < offer.offered_amount
    {
        return fail("The offering village no longer has enough resources.");
    }
    if stockpiles::resource_amount(&target.resources, offer.requested_kind) + f64::EPSILON
        < offer.requested_amount
    {
        return fail("This village does not have enough requested resources.");
    }
    if trade_would_overflow(target, offer.offered_kind, offer.offered_amount)
        || trade_would_overflow(source, offer.requested_kind, offer.requested_amount)
    {
        return fail("A receiving village lacks storage for this trade.");
    }

    stockpiles::add_resource(
        &mut source.resources,
        offer.offered_kind,
        -offer.offered_amount,
    );
    stockpiles::add_resource(
        &mut target.resources,
        offer.requested_kind,
        -offer.requested_amount,
    );
    // Reconcile the outgoing halves first so their vacated physical slots are
    // available to receive the other village's goods.
    reconcile_colony_stockpiles(source);
    reconcile_colony_stockpiles(target);
    let Some(source_plan) =
        trade_deposit_plan(source, offer.requested_kind, offer.requested_amount)
    else {
        stockpiles::add_resource(
            &mut source.resources,
            offer.offered_kind,
            offer.offered_amount,
        );
        stockpiles::add_resource(
            &mut target.resources,
            offer.requested_kind,
            offer.requested_amount,
        );
        reconcile_colony_stockpiles(source);
        reconcile_colony_stockpiles(target);
        return fail("A receiving village lacks storage for this trade.");
    };
    let Some(target_plan) = trade_deposit_plan(target, offer.offered_kind, offer.offered_amount)
    else {
        stockpiles::add_resource(
            &mut source.resources,
            offer.offered_kind,
            offer.offered_amount,
        );
        stockpiles::add_resource(
            &mut target.resources,
            offer.requested_kind,
            offer.requested_amount,
        );
        reconcile_colony_stockpiles(source);
        reconcile_colony_stockpiles(target);
        return fail("A receiving village lacks storage for this trade.");
    };
    store_trade_incoming(
        source,
        offer.requested_kind,
        offer.requested_amount,
        &source_plan,
    );
    store_trade_incoming(
        target,
        offer.offered_kind,
        offer.offered_amount,
        &target_plan,
    );
    source.village_trade_offers.remove(offer_id);
    source.last_player_activity_at = Some(ctx.now_ms);
    target.last_player_activity_at = Some(ctx.now_ms);
    reconcile_colony_stockpiles(source);
    reconcile_colony_stockpiles(target);
    ok()
}

fn store_trade_incoming(
    colony: &mut ColonyRuntime,
    kind: stockpiles::ResourceKind,
    amount: f64,
    plan: &[(usize, f64)],
) {
    stockpiles::add_resource(&mut colony.resources, kind, amount);
    for &(index, stored) in plan {
        stockpiles::add_resource(&mut colony.stockpiles[index].contents, kind, stored);
    }
}

fn trade_deposit_plan(
    colony: &ColonyRuntime,
    kind: stockpiles::ResourceKind,
    amount: f64,
) -> Option<Vec<(usize, f64)>> {
    let capacities = storage::authoritative_storage_capacities_for_owned(
        &storage_buildings(colony),
        &colony.stockpiles,
        colony.upgrade_tree.owned_node_ids.iter(),
    );
    let mut indices = colony
        .stockpiles
        .iter()
        .enumerate()
        .filter(|(_, pile)| !pile.is_station_local() && pile.accepts.contains(&kind))
        .map(|(index, pile)| (pile.id.clone(), index))
        .collect::<Vec<_>>();
    indices.sort_by(|left, right| left.0.cmp(&right.0));
    let mut plan = Vec::new();
    let mut remaining = amount;
    for (_, index) in indices {
        let stored = remaining.min(stockpiles::headroom_for(
            &colony.stockpiles[index],
            kind,
            &capacities,
        ));
        if stored > 0.0 {
            plan.push((index, stored));
        }
        remaining -= stored;
        if remaining <= f64::EPSILON {
            break;
        }
    }
    (remaining <= f64::EPSILON).then_some(plan)
}

fn trade_would_overflow(
    colony: &ColonyRuntime,
    kind: stockpiles::ResourceKind,
    incoming: f64,
) -> bool {
    let capacities = storage::authoritative_storage_capacities_for_owned(
        &storage_buildings(colony),
        &colony.stockpiles,
        colony.upgrade_tree.owned_node_ids.iter(),
    );
    let capacity = match kind {
        stockpiles::ResourceKind::Food => Some(capacities.food),
        stockpiles::ResourceKind::Fish => Some(capacities.fish),
        stockpiles::ResourceKind::Water => Some(capacities.water),
        stockpiles::ResourceKind::Herbs => Some(capacities.herbs),
        stockpiles::ResourceKind::Catnip => Some(capacities.catnip),
        stockpiles::ResourceKind::Grain => Some(capacities.grain),
        stockpiles::ResourceKind::Flour => Some(capacities.flour),
        stockpiles::ResourceKind::Materials => Some(capacities.materials),
        stockpiles::ResourceKind::Refined => Some(capacities.refined),
        stockpiles::ResourceKind::Weapons => Some(capacities.weapons),
        stockpiles::ResourceKind::Armor => Some(capacities.armor),
        stockpiles::ResourceKind::Logs => Some(capacities.logs),
        stockpiles::ResourceKind::Lumber => Some(capacities.lumber),
        stockpiles::ResourceKind::Planks => Some(capacities.planks),
        stockpiles::ResourceKind::Blocks => Some(capacities.blocks),
        stockpiles::ResourceKind::Tools => Some(capacities.tools),
        stockpiles::ResourceKind::Fibre => Some(capacities.fibre),
        stockpiles::ResourceKind::Hide => Some(capacities.hide),
        stockpiles::ResourceKind::Cloth => Some(capacities.cloth),
        stockpiles::ResourceKind::Leather => Some(capacities.leather),
        stockpiles::ResourceKind::Ore => Some(capacities.ore),
        stockpiles::ResourceKind::Metal => Some(capacities.metal),
        stockpiles::ResourceKind::Blessings => None,
    };
    capacity.is_some_and(|capacity| {
        stockpiles::resource_amount(&colony.resources, kind) + incoming > capacity + f64::EPSILON
    }) || trade_deposit_plan(colony, kind, incoming).is_none()
}

fn cancel_village_trade(
    world: &mut WorldState,
    offer_id: &str,
    ctx: &ActionCtx,
) -> proto::ActionResult {
    let Some(source) = world.colonies.iter_mut().find(|colony| {
        colony.id == ctx.colony_id && colony.village_trade_offers.contains_key(offer_id)
    }) else {
        return fail("Trade offer is not available.");
    };
    if !can_control_village(source, &ctx.player_id) {
        return fail("Trade offer is not available.");
    }
    source.village_trade_offers.remove(offer_id);
    source.last_player_activity_at = Some(ctx.now_ms);
    ok()
}

fn two_colonies_mut(
    colonies: &mut [ColonyRuntime],
    left: usize,
    right: usize,
) -> (&mut ColonyRuntime, &mut ColonyRuntime) {
    debug_assert_ne!(left, right);
    if left < right {
        let (head, tail) = colonies.split_at_mut(right);
        (&mut head[left], &mut tail[0])
    } else {
        let (head, tail) = colonies.split_at_mut(left);
        (&mut tail[0], &mut head[right])
    }
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
    let caps =
        storage::authoritative_storage_capacities(&storage_buildings, &colony.stockpiles, &effects);
    let housing_buildings = housing_buildings(colony);
    let housing_capacity = housing::housing_capacity(&housing_buildings, effects.housing_per_den)
        * effects.housing_capacity_mult;
    let population = alive_cats.len() as u32;
    let current_migration_minute = migration_game_minute_at(colony, now_ms);
    let probation_deadlines = colony
        .migration_state
        .probationary_migrants
        .iter()
        .map(|migrant| (migrant.id.as_str(), migrant.housing_deadline_game_minute))
        .collect::<BTreeMap<_, _>>();
    let probationary = u32::try_from(
        alive_cats
            .iter()
            .filter(|cat| probation_deadlines.contains_key(cat.id.as_str()))
            .count(),
    )
    .unwrap_or(u32::MAX);
    let permanent_population = population.saturating_sub(probationary);
    let housing_capacity_u32 = housing_capacity.max(0.0).floor() as u32;
    let housed = permanent_population.min(housing_capacity_u32);
    let mut permanent_ids = alive_cats
        .iter()
        .filter(|cat| !probation_deadlines.contains_key(cat.id.as_str()))
        .map(|cat| cat.id.as_str())
        .collect::<Vec<_>>();
    permanent_ids.sort_unstable();
    let housed_ids = permanent_ids
        .into_iter()
        .take(housing_capacity_u32 as usize)
        .collect::<BTreeSet<_>>();
    let election_payload = election_snapshot(colony, &alive_cats);
    let election_schedule_payload = election_schedule_snapshot(colony, now_ms);
    let vote_kick_payload = vote_kick_snapshot(colony, &alive_cats);
    let warrior_count = alive_cats
        .iter()
        .filter(|cat| cat.specialization == Some(CatSpecialization::Warrior))
        .count() as u32;

    proto::ColonySnapshot {
        id: colony.id.clone(),
        name: colony.name.clone(),
        kind: match colony.kind {
            VillageKind::Global => proto::VillageKind::Global,
            VillageKind::Personal => proto::VillageKind::Personal,
        },
        scale: match colony.scale {
            VillageScale::Personal => proto::VillageScale::Personal,
            VillageScale::Communal => proto::VillageScale::Communal,
        },
        capabilities: proto::VillageCapabilities::default(),
        status: sim_to_proto_colony_status(colony.status),
        resources: colony_resources_snapshot(colony),
        storage: proto::StorageSnapshot {
            capacities: proto::ResourceCapacities {
                food: caps.food,
                fish: caps.fish,
                water: caps.water,
                herbs: caps.herbs,
                catnip: caps.catnip,
                grain: caps.grain,
                flour: caps.flour,
                materials: caps.materials,
                refined: caps.refined,
                weapons: caps.weapons,
                armor: caps.armor,
                planks: caps.planks,
                logs: caps.logs,
                lumber: caps.lumber,
                blocks: caps.blocks,
                tools: caps.tools,
                fibre: caps.fibre,
                hide: caps.hide,
                cloth: caps.cloth,
                leather: caps.leather,
                ore: caps.ore,
                metal: caps.metal,
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
            .map(|cat| {
                let deadline = probation_deadlines.get(cat.id.as_str()).copied();
                let status = if deadline.is_some() {
                    proto::CatHousingStatus::Probationary
                } else if housed_ids.contains(cat.id.as_str()) {
                    proto::CatHousingStatus::Housed
                } else {
                    proto::CatHousingStatus::Unhoused
                };
                cat_snapshot(
                    colony,
                    cat,
                    status,
                    deadline.map(|deadline| deadline.saturating_sub(current_migration_minute)),
                )
            })
            .collect(),
        jobs: jobs_snapshot(colony),
        upgrades: upgrades_snapshot(colony),
        events: events_snapshot(colony),
        housing: proto::HousingSnapshot {
            population,
            capacity: housing_capacity_u32,
            pressure: housing::housing_pressure(f64::from(population), housing_capacity),
            village_level: housing::village_level(&housing_buildings),
            housed,
            probationary,
            unhoused: population.saturating_sub(housed),
            departures: colony.migration_departures,
        },
        research: research_snapshot(colony),
        election: election_payload,
        election_schedule: election_schedule_payload,
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
        agricultural_tiles: colony.agricultural_tiles.iter().map(tile_point).collect(),
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
        dirt_road_tiles: colony
            .world_tiles
            .iter()
            .filter(|(_, tile)| crate::world_tick::tile_forms_dirt_road(tile))
            .map(|(pos, _)| tile_point(pos))
            .collect(),
        village_gate: village_gate_snapshot(colony),
        wall_segments: crate::world_tick::effective_wall_segments(colony)
            .into_iter()
            .map(|entry| proto::WallSegment {
                x: entry.segment.x,
                y: entry.segment.y,
                side: match entry.segment.side {
                    village_area::Side::N => proto::GateSide::N,
                    village_area::Side::E => proto::GateSide::E,
                    village_area::Side::S => proto::GateSide::S,
                    village_area::Side::W => proto::GateSide::W,
                },
                under_construction: entry.newly_built,
            })
            .collect(),
        village_radius: snapshot_village_radius(colony),
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
            .filter(|pile| !pile.is_station_local())
            .map(|pile| stockpile_snapshot(pile, colony))
            .collect(),
        active_stockpile_haul: active_stockpile_haul_snapshot(colony),
        farms: colony
            .farms
            .iter()
            .map(|plot| proto::FarmSnapshot {
                id: plot.id.clone(),
                x1: plot.rect.x1,
                y1: plot.rect.y1,
                x2: plot.rect.x2,
                y2: plot.rect.y2,
                crop: sim_to_proto_crop(plot.crop),
                planted_at: plot.planted_at,
                stage: sim_to_proto_farm_stage(plot.stage),
                growth_hours: plot.growth_hours,
                worker_id: plot.worker_id.clone(),
                work_phase: sim_to_proto_farm_work_phase(plot.work_phase),
                input_inventory: Vec::new(),
                output_inventory: (plot.pending_output > 0.0)
                    .then_some(proto::ResourceStackSnapshot {
                        kind: sim_to_proto_crop_resource(plot.crop),
                        amount: plot.pending_output,
                    })
                    .into_iter()
                    .collect(),
                worker_travel: farm_worker_travel(colony, plot),
                block_reason: farm_block_reason(colony, plot),
            })
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
            .keys()
            .filter_map(|item| {
                let available = colony
                    .items
                    .pristine_count(*item)
                    .min(trader::max_item_units_per_load(*item));
                (available > 0).then(|| proto::TraderBuyOffer {
                    kind: item.kind.as_str().to_owned(),
                    material: item.material.as_str().to_owned(),
                    quality: item.quality,
                    available,
                    unit_price: trader::trader_buy_price(*item, 1),
                    unit_weight_grams: item_weight_grams(*item),
                })
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
fn items_snapshot(items: &ItemStore) -> Vec<proto::ItemStackSnapshot> {
    items
        .iter()
        .map(|(item, &count)| proto::ItemStackSnapshot {
            kind: item.kind.as_str().to_owned(),
            material: item.material.as_str().to_owned(),
            quality: item.quality,
            count,
            value: item.value(),
            unit_weight_grams: item_weight_grams(*item),
            instances: items
                .instances()
                .filter(|instance| instance.item == *item)
                .map(|instance| proto::ItemInstanceSnapshot {
                    id: instance.id.clone(),
                    durability: instance.durability,
                    max_durability: instance.max_durability,
                    broken: instance.is_broken(),
                })
                .collect(),
        })
        .collect()
}

fn stock_ledger_snapshot(colony: &ColonyRuntime) -> proto::StockLedgerSnapshot {
    proto::StockLedgerSnapshot {
        reported: resources_snapshot(&colony.stock_ledger.reported),
        last_counted: colony.stock_ledger.last_counted,
        accurate: colony.stock_ledger.is_accurate(&colony.resources)
            && colony
                .stock_ledger
                .visible_piles_are_accurate(&colony.stockpiles),
        active_round: colony.stock_ledger.active_round.as_ref().map(|round| {
            proto::AccountingRoundSnapshot {
                worker_id: round.worker_id.clone(),
                tent_id: round.tent_id.clone(),
                phase: match round.phase {
                    crate::ledger::AccountingPhase::TravelingToTent => {
                        proto::AccountingPhase::TravelingToTent
                    }
                    crate::ledger::AccountingPhase::TravelingToPile => {
                        proto::AccountingPhase::TravelingToPile
                    }
                    crate::ledger::AccountingPhase::Counting => proto::AccountingPhase::Counting,
                    crate::ledger::AccountingPhase::ReturningToTent => {
                        proto::AccountingPhase::ReturningToTent
                    }
                    crate::ledger::AccountingPhase::WaitingAtTent => {
                        proto::AccountingPhase::WaitingAtTent
                    }
                },
                target_stockpile_id: round.target_stockpile_id.clone(),
                remaining_piles: round.pending_stockpile_ids.len()
                    + usize::from(round.target_stockpile_id.is_some()),
                unreachable_piles: round.unreachable_stockpile_ids.len(),
                dwell_elapsed_ms: round.dwell_elapsed_ms,
                dwell_required_ms: crate::ledger::PILE_COUNT_DWELL_MS,
            }
        }),
    }
}

fn stockpile_snapshot(
    pile: &stockpiles::Stockpile,
    colony: &ColonyRuntime,
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
        report: colony
            .stock_ledger
            .pile_reports
            .get(&pile.id)
            .map(|report| proto::StockpileReportSnapshot {
                reported: resources_snapshot(&report.reported),
                last_counted: report.last_counted,
                accurate: report.reported == pile.contents,
            }),
        gather_spot: colony
            .gather_spots
            .iter()
            .find(|spot| spot.stockpile_id == pile.id)
            .map(|spot| proto::GatherSpotSnapshot {
                kind: sim_to_proto_resource_kind(spot.kind),
                expires_at_ms: spot.expires_at_ms,
                purpose: match spot.purpose {
                    stockpiles::GatherSpotPurpose::General => proto::GatherSpotPurpose::General,
                    stockpiles::GatherSpotPurpose::Fishing => proto::GatherSpotPurpose::Fishing,
                },
                fish_population: (spot.purpose == stockpiles::GatherSpotPurpose::Fishing)
                    .then(|| {
                        let (x, y) = pile.center();
                        let shore = TilePos {
                            x: x.round() as i32,
                            y: y.round() as i32,
                        };
                        crate::world_tick::fishing_habitat_tile(colony, shore)
                            .and_then(|water| colony.fish_habitats.get(&water))
                            .map(|population| proto::FishPopulationSnapshot {
                                stock: population.stock,
                                capacity: population.capacity,
                                last_replenished_at_ms: population.last_replenished_at_ms,
                            })
                    })
                    .flatten(),
            }),
        steward_managed: colony.stock_ledger.steward_managed_piles.get(&pile.id).map(
            |provenance| proto::StewardManagedPileSnapshot {
                station_id: provenance.station_id.clone(),
                resource: sim_to_proto_resource_kind(provenance.resource),
                active: provenance.active,
            },
        ),
    }
}

fn active_stockpile_haul_snapshot(colony: &ColonyRuntime) -> Option<proto::StockpileHaulSnapshot> {
    colony.jobs.iter().find_map(|job| {
        let JobMetadata::StockpileHaul {
            source_stockpile_id,
            destination_stockpile_id,
            kind,
            amount_in_transit,
            ..
        } = &job.metadata
        else {
            return None;
        };
        let is_recovery = !matches!(job.status, JobStatus::Queued | JobStatus::Active);
        if is_recovery && *amount_in_transit <= f64::EPSILON {
            return None;
        }
        let worker_id = job
            .assigned_cat
            .clone()
            .unwrap_or_else(|| "unassigned".to_owned());
        let carrying = colony.cats.iter().any(|cat| {
            cat.id == worker_id
                && cat.carrying.as_ref().and_then(|cargo| {
                    crate::world_tick::steward_haul_job_id_for_snapshot(
                        cargo.source_gather_spot.as_deref(),
                    )
                }) == Some(job.id.as_str())
        });
        Some(proto::StockpileHaulSnapshot {
            job_id: job.id.clone(),
            worker_id,
            source_stockpile_id: source_stockpile_id.clone(),
            destination_stockpile_id: destination_stockpile_id.clone(),
            resource: sim_to_proto_resource_kind(*kind),
            amount: *amount_in_transit,
            phase: if is_recovery {
                proto::StockpileHaulPhase::RecoveryBlocked
            } else if carrying {
                proto::StockpileHaulPhase::CarryingToDestination
            } else {
                proto::StockpileHaulPhase::TravelingToSource
            },
        })
    })
}

fn cat_snapshot(
    colony: &ColonyRuntime,
    cat: &Cat,
    housing_status: proto::CatHousingStatus,
    probation_remaining_game_minutes: Option<u64>,
) -> proto::CatSnapshot {
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
        skills: cat
            .skills
            .iter()
            .map(|(labor, xp)| (sim_to_proto_labor(*labor), *xp))
            .collect(),
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
        preferred_labors: cat
            .preferred_labors
            .iter()
            .copied()
            .map(sim_to_proto_labor)
            .collect(),
        pregnant: cat.is_pregnant,
        housing_status,
        probation_remaining_game_minutes,
    }
}

fn sim_to_proto_labor(labor: Labor) -> proto::Labor {
    match labor {
        Labor::Hunt => proto::Labor::Hunt,
        Labor::Fishing => proto::Labor::Fishing,
        Labor::Build => proto::Labor::Build,
        Labor::Ritual => proto::Labor::Ritual,
        Labor::Fight => proto::Labor::Fight,
        Labor::Train => proto::Labor::Train,
        Labor::Quarry => proto::Labor::Quarry,
        Labor::Woodcut => proto::Labor::Woodcut,
        Labor::Forage => proto::Labor::Forage,
        Labor::FetchWater => proto::Labor::FetchWater,
        Labor::Mill => proto::Labor::Mill,
        Labor::Process => proto::Labor::Process,
        Labor::Craft => proto::Labor::Craft,
        Labor::Textile => proto::Labor::Textile,
        Labor::Metalwork => proto::Labor::Metalwork,
        Labor::Farm => proto::Labor::Farm,
        Labor::Haul => proto::Labor::Haul,
        Labor::Research => proto::Labor::Research,
        Labor::Scout => proto::Labor::Scout,
    }
}

fn proto_to_sim_labor(labor: proto::Labor) -> Labor {
    match labor {
        proto::Labor::Hunt => Labor::Hunt,
        proto::Labor::Fishing => Labor::Fishing,
        proto::Labor::Build => Labor::Build,
        proto::Labor::Ritual => Labor::Ritual,
        proto::Labor::Fight => Labor::Fight,
        proto::Labor::Train => Labor::Train,
        proto::Labor::Quarry => Labor::Quarry,
        proto::Labor::Woodcut => Labor::Woodcut,
        proto::Labor::Forage => Labor::Forage,
        proto::Labor::FetchWater => Labor::FetchWater,
        proto::Labor::Mill => Labor::Mill,
        proto::Labor::Process => Labor::Process,
        proto::Labor::Craft => Labor::Craft,
        proto::Labor::Textile => Labor::Textile,
        proto::Labor::Metalwork => Labor::Metalwork,
        proto::Labor::Farm => Labor::Farm,
        proto::Labor::Haul => Labor::Haul,
        proto::Labor::Research => Labor::Research,
        proto::Labor::Scout => Labor::Scout,
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

fn election_schedule_snapshot(
    colony: &ColonyRuntime,
    now_ms: i64,
) -> Option<proto::ElectionScheduleSnapshot> {
    election_schedule_timing(colony, now_ms).map(|schedule| proto::ElectionScheduleSnapshot {
        term_started_at: schedule.term_started_at,
        next_election_at: schedule.next_election_at,
        term_length_ms: schedule.term_length_ms,
        remaining_ms: schedule.next_election_at.saturating_sub(now_ms).max(0),
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
            let recipe_availability =
                crate::world_tick::available_production_recipes(building.building_type)
                    .iter()
                    .filter_map(|recipe_id| {
                        crate::world_tick::production_recipe_availability(
                            colony,
                            building.building_type,
                            recipe_id,
                        )
                    })
                    .collect::<Vec<_>>();
            let required_recipe_study = recipe_availability
                .iter()
                .find(|recipe| !recipe.available)
                .and_then(|recipe| recipe.required_study_id)
                .and_then(|id| crate::research_catalog::research_catalog().get(id))
                .map(|node| proto::ResearchTarget {
                    id: node.id.clone(),
                    name: node.name.clone(),
                    cost: node.cost,
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
                // tile (see `world_tick::building_inbound_haul`). Physical Mill/Sawmill
                // inputs target their station; ordinary cargo targets stockpiles.
                inbound_haul: crate::world_tick::building_inbound_haul(colony, building),
                outbound_haul: crate::world_tick::building_outbound_haul(colony, building),
                input_inventory: crate::world_tick::building_station_inventory(
                    colony, building, false,
                )
                .into_iter()
                .map(|(kind, amount)| proto::ResourceStackSnapshot {
                    kind: sim_to_proto_resource_kind(kind),
                    amount,
                })
                .collect(),
                output_inventory: crate::world_tick::building_station_inventory(
                    colony, building, true,
                )
                .into_iter()
                .map(|(kind, amount)| proto::ResourceStackSnapshot {
                    kind: sim_to_proto_resource_kind(kind),
                    amount,
                })
                .collect(),
                production_queue: crate::world_tick::building_production_queue(building)
                    .into_iter()
                    .map(|entry| proto::ProductionQueueEntrySnapshot {
                        recipe_id: entry.recipe_id,
                        repeat: entry.repeat,
                    })
                    .collect(),
                available_recipes: recipe_availability
                    .iter()
                    .filter(|recipe| recipe.available)
                    .map(|recipe| recipe.recipe_id.to_owned())
                    .collect(),
                required_recipe_study,
                production_paused: building.production_paused,
                production_block_reason:
                    crate::world_tick::building_production_block_reason_with_availability(
                        colony,
                        building,
                        &recipe_availability,
                    ),
                worker_travel: crate::world_tick::building_worker_travel(colony, building),
                inbound_cargo: crate::world_tick::building_station_cargo(colony, building, "in")
                    .into_iter()
                    .map(|(kind, amount)| proto::ResourceStackSnapshot {
                        kind: sim_to_proto_resource_kind(kind),
                        amount,
                    })
                    .collect(),
                outbound_cargo: crate::world_tick::building_station_cargo(colony, building, "out")
                    .into_iter()
                    .map(|(kind, amount)| proto::ResourceStackSnapshot {
                        kind: sim_to_proto_resource_kind(kind),
                        amount,
                    })
                    .collect(),
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
    let base_duration_ms = (duration_seconds * 1000.0) as i64;
    let duration_ms = if matches!(
        kind,
        JobKind::BuildHouse | JobKind::Quarry | JobKind::GatherLogs | JobKind::HaulGatherSpot
    ) {
        productive_duration_ms(
            base_duration_ms,
            crate::world_tick::usable_tool_stock(colony),
        )
    } else {
        base_duration_ms
    };

    if let Some(cat_id) = assigned_cat.as_deref() {
        for building in &mut colony.buildings {
            if building.assigned_cat.as_deref() == Some(cat_id)
                && production::building_staff_cap(building.building_type) > 0
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
    select_best_cat_for_labor(colony, specialization, None)
}

fn select_best_cat_for_labor(
    colony: &ColonyRuntime,
    specialization: Option<CatSpecialization>,
    labor: Option<Labor>,
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
            let a_preferred = labor.is_some_and(|labor| a.preferred_labors.contains(&labor));
            let b_preferred = labor.is_some_and(|labor| b.preferred_labors.contains(&labor));
            a_preferred
                .cmp(&b_preferred)
                .then_with(|| {
                    specialization_stat(a, specialization)
                        .total_cmp(&specialization_stat(b, specialization))
                })
                .then_with(|| b.id.cmp(&a.id))
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

fn select_best_scout(colony: &ColonyRuntime) -> Option<String> {
    let busy = busy_cat_ids(colony);
    let assigned = assigned_building_cat_ids(colony);
    colony
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
        .max_by(|a, b| {
            a.stats
                .vision
                .total_cmp(&b.stats.vision)
                .then_with(|| b.id.cmp(&a.id))
        })
        .map(|cat| cat.id.clone())
}

fn cat_can_take_assignment(colony: &ColonyRuntime, cat_index: usize) -> bool {
    let cat = &colony.cats[cat_index];
    let busy = busy_cat_ids(colony);
    cat.death_time.is_none()
        && can_work(get_life_stage(cat.age_hours))
        && cat.activity == CatActivity::Idle
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

fn snapshot_village_radius(colony: &ColonyRuntime) -> u32 {
    let dynamic_radius = village_ring_radius(colony.buildings.len() as i32) as u32;
    match colony.scale {
        // Keep the authored communal parcel framed from first connection. Ordinary
        // villages retain the established building-ring camera behavior exactly.
        VillageScale::Communal => dynamic_radius.max(9),
        VillageScale::Personal => dynamic_radius,
    }
}

fn claimed_area(colony: &ColonyRuntime) -> village_area::VillageArea {
    let tiles = colony
        .claimed_tiles
        .iter()
        .filter(|tile| !colony.agricultural_tiles.contains(tile))
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
        fish: resources.fish,
        water: resources.water,
        herbs: resources.herbs,
        catnip: resources.catnip,
        grain: resources.grain,
        flour: resources.flour,
        materials: resources.materials,
        refined: resources.refined,
        weapons: resources.weapons,
        armor: resources.armor,
        planks: resources.planks,
        logs: resources.logs,
        lumber: resources.lumber,
        blocks: resources.blocks,
        tools: resources.tools,
        fibre: resources.fibre,
        hide: resources.hide,
        cloth: resources.cloth,
        leather: resources.leather,
        ore: resources.ore,
        metal: resources.metal,
        blessings: resources.blessings,
    }
}

/// Player-facing colony resources project the canonical spendable blessing bank. Blessings
/// never occupy physical stockpiles, so per-pile and Accountant-ledger snapshots continue to
/// report their real (normally zero) `Resources::blessings` field without double-counting it.
fn colony_resources_snapshot(colony: &ColonyRuntime) -> proto::ResourceAmounts {
    let mut resources = resources_snapshot(&colony.resources);
    resources.blessings = colony.global_upgrade_points;
    resources
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
        JobKind::Fish => Some(TaskType::Fish),
        JobKind::SupplyWater | JobKind::FetchWater => Some(TaskType::FetchWater),
        JobKind::LeaderPlanHouse | JobKind::BuildHouse | JobKind::Quarry => Some(TaskType::Build),
        JobKind::GatherLogs => Some(TaskType::Build),
        JobKind::ForageFibre => Some(TaskType::Hunt),
        JobKind::Ritual | JobKind::PerformOffering => Some(TaskType::Guard),
        JobKind::CarryOffering => Some(TaskType::Build),
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
        colony_id: None,
    }
}

fn ok_for_colony(colony_id: &str) -> proto::ActionResult {
    proto::ActionResult {
        ok: true,
        message: None,
        colony_id: Some(colony_id.to_owned()),
    }
}

fn fail(message: impl Into<String>) -> proto::ActionResult {
    proto::ActionResult {
        ok: false,
        message: Some(message.into()),
        colony_id: None,
    }
}

fn proto_to_sim_officer_role(role: proto::OfficerRole) -> OfficerRole {
    match role {
        proto::OfficerRole::Steward => OfficerRole::Steward,
        proto::OfficerRole::Accountant => OfficerRole::Accountant,
        proto::OfficerRole::Forester => OfficerRole::Forester,
        proto::OfficerRole::Farmer => OfficerRole::Farmer,
        proto::OfficerRole::Captain => OfficerRole::Captain,
        proto::OfficerRole::Loremaster => OfficerRole::Loremaster,
        proto::OfficerRole::ClothLeader => OfficerRole::ClothLeader,
    }
}

fn proto_to_sim_scout_mission(mission: proto::ScoutMission) -> ScoutMission {
    match mission {
        proto::ScoutMission::Explore => ScoutMission::Explore,
        proto::ScoutMission::Resource(resource) => ScoutMission::Resource(match resource {
            proto::ScoutResource::Wood => ScoutResource::Wood,
            proto::ScoutResource::Food => ScoutResource::Food,
            proto::ScoutResource::Water => ScoutResource::Water,
            proto::ScoutResource::Stone => ScoutResource::Stone,
        }),
    }
}

fn sim_to_proto_officer_role(role: OfficerRole) -> proto::OfficerRole {
    match role {
        OfficerRole::Steward => proto::OfficerRole::Steward,
        OfficerRole::Accountant => proto::OfficerRole::Accountant,
        OfficerRole::Forester => proto::OfficerRole::Forester,
        OfficerRole::Farmer => proto::OfficerRole::Farmer,
        OfficerRole::Captain => proto::OfficerRole::Captain,
        OfficerRole::Loremaster => proto::OfficerRole::Loremaster,
        OfficerRole::ClothLeader => proto::OfficerRole::ClothLeader,
    }
}

fn proto_to_sim_crop(crop: proto::CropKind) -> farming::CropKind {
    match crop {
        proto::CropKind::Catnip => farming::CropKind::Catnip,
        proto::CropKind::Grain => farming::CropKind::Grain,
        proto::CropKind::Herb => farming::CropKind::Herb,
    }
}

fn sim_to_proto_crop(crop: farming::CropKind) -> proto::CropKind {
    match crop {
        farming::CropKind::Catnip => proto::CropKind::Catnip,
        farming::CropKind::Grain => proto::CropKind::Grain,
        farming::CropKind::Herb => proto::CropKind::Herb,
    }
}

fn sim_to_proto_farm_stage(stage: farming::FarmStage) -> proto::FarmStage {
    match stage {
        farming::FarmStage::Soil => proto::FarmStage::Soil,
        farming::FarmStage::Sprout => proto::FarmStage::Sprout,
        farming::FarmStage::Growing => proto::FarmStage::Growing,
        farming::FarmStage::Mature => proto::FarmStage::Mature,
        farming::FarmStage::Flowering => proto::FarmStage::Flowering,
    }
}

fn sim_to_proto_farm_work_phase(phase: farming::FarmWorkPhase) -> proto::FarmWorkPhase {
    match phase {
        farming::FarmWorkPhase::WaitingForWorker => proto::FarmWorkPhase::WaitingForWorker,
        farming::FarmWorkPhase::Traveling => proto::FarmWorkPhase::Traveling,
        farming::FarmWorkPhase::Planting => proto::FarmWorkPhase::Planting,
        farming::FarmWorkPhase::Tending => proto::FarmWorkPhase::Tending,
        farming::FarmWorkPhase::Harvesting => proto::FarmWorkPhase::Harvesting,
        farming::FarmWorkPhase::Hauling => proto::FarmWorkPhase::Hauling,
        farming::FarmWorkPhase::OutputBlocked => proto::FarmWorkPhase::OutputBlocked,
    }
}

fn sim_to_proto_crop_resource(crop: farming::CropKind) -> proto::ResourceKind {
    match crop {
        farming::CropKind::Catnip => proto::ResourceKind::Catnip,
        farming::CropKind::Grain => proto::ResourceKind::Grain,
        farming::CropKind::Herb => proto::ResourceKind::Herbs,
    }
}

fn farm_worker_travel(colony: &ColonyRuntime, plot: &FarmPlot) -> Option<String> {
    let worker = plot.worker_id.as_deref().and_then(|id| {
        colony
            .cats
            .iter()
            .find(|cat| cat.id == id && cat.death_time.is_none())
    })?;
    worker.destination.map(|destination| {
        format!(
            "traveling to {},{}",
            destination.x.round() as i32,
            destination.y.round() as i32
        )
    })
}

fn farm_block_reason(colony: &ColonyRuntime, plot: &FarmPlot) -> Option<String> {
    match plot.work_phase {
        farming::FarmWorkPhase::WaitingForWorker => Some("no_worker".to_owned()),
        farming::FarmWorkPhase::OutputBlocked => Some("output_storage_full".to_owned()),
        farming::FarmWorkPhase::Traveling => Some("worker_travel".to_owned()),
        farming::FarmWorkPhase::Hauling => Some("output_in_transit".to_owned()),
        _ if plot.worker_id.as_deref().is_some_and(|id| {
            !colony
                .cats
                .iter()
                .any(|cat| cat.id == id && cat.death_time.is_none())
        }) =>
        {
            Some("no_worker".to_owned())
        }
        _ => None,
    }
}

fn proto_to_sim_resource_kind(kind: proto::ResourceKind) -> stockpiles::ResourceKind {
    use stockpiles::ResourceKind;
    match kind {
        proto::ResourceKind::Food => ResourceKind::Food,
        proto::ResourceKind::Fish => ResourceKind::Fish,
        proto::ResourceKind::Water => ResourceKind::Water,
        proto::ResourceKind::Herbs => ResourceKind::Herbs,
        proto::ResourceKind::Catnip => ResourceKind::Catnip,
        proto::ResourceKind::Grain => ResourceKind::Grain,
        proto::ResourceKind::Flour => ResourceKind::Flour,
        proto::ResourceKind::Materials => ResourceKind::Materials,
        proto::ResourceKind::Refined => ResourceKind::Refined,
        proto::ResourceKind::Weapons => ResourceKind::Weapons,
        proto::ResourceKind::Armor => ResourceKind::Armor,
        proto::ResourceKind::Logs => ResourceKind::Logs,
        proto::ResourceKind::Lumber => ResourceKind::Lumber,
        proto::ResourceKind::Planks => ResourceKind::Planks,
        proto::ResourceKind::Blocks => ResourceKind::Blocks,
        proto::ResourceKind::Tools => ResourceKind::Tools,
        proto::ResourceKind::Fibre => ResourceKind::Fibre,
        proto::ResourceKind::Hide => ResourceKind::Hide,
        proto::ResourceKind::Cloth => ResourceKind::Cloth,
        proto::ResourceKind::Leather => ResourceKind::Leather,
        proto::ResourceKind::Ore => ResourceKind::Ore,
        proto::ResourceKind::Metal => ResourceKind::Metal,
        proto::ResourceKind::Blessings => ResourceKind::Blessings,
    }
}

fn sim_to_proto_resource_kind(kind: stockpiles::ResourceKind) -> proto::ResourceKind {
    use stockpiles::ResourceKind;
    match kind {
        ResourceKind::Food => proto::ResourceKind::Food,
        ResourceKind::Fish => proto::ResourceKind::Fish,
        ResourceKind::Water => proto::ResourceKind::Water,
        ResourceKind::Herbs => proto::ResourceKind::Herbs,
        ResourceKind::Catnip => proto::ResourceKind::Catnip,
        ResourceKind::Grain => proto::ResourceKind::Grain,
        ResourceKind::Flour => proto::ResourceKind::Flour,
        ResourceKind::Materials => proto::ResourceKind::Materials,
        ResourceKind::Refined => proto::ResourceKind::Refined,
        ResourceKind::Weapons => proto::ResourceKind::Weapons,
        ResourceKind::Armor => proto::ResourceKind::Armor,
        ResourceKind::Logs => proto::ResourceKind::Logs,
        ResourceKind::Lumber => proto::ResourceKind::Lumber,
        ResourceKind::Planks => proto::ResourceKind::Planks,
        ResourceKind::Blocks => proto::ResourceKind::Blocks,
        ResourceKind::Tools => proto::ResourceKind::Tools,
        ResourceKind::Fibre => proto::ResourceKind::Fibre,
        ResourceKind::Hide => proto::ResourceKind::Hide,
        ResourceKind::Cloth => proto::ResourceKind::Cloth,
        ResourceKind::Leather => proto::ResourceKind::Leather,
        ResourceKind::Ore => proto::ResourceKind::Ore,
        ResourceKind::Metal => proto::ResourceKind::Metal,
        ResourceKind::Blessings => proto::ResourceKind::Blessings,
    }
}

fn village_trade_offer_snapshot(offer: &VillageTradeOffer) -> proto::VillageTradeOfferSnapshot {
    proto::VillageTradeOfferSnapshot {
        id: offer.id.clone(),
        from_colony_id: offer.from_colony_id.clone(),
        to_colony_id: offer.to_colony_id.clone(),
        offered_kind: sim_to_proto_resource_kind(offer.offered_kind),
        offered_amount: offer.offered_amount,
        requested_kind: sim_to_proto_resource_kind(offer.requested_kind),
        requested_amount: offer.requested_amount,
        created_at: offer.created_at,
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
        proto::JobKind::GatherLogs => JobKind::GatherLogs,
        proto::JobKind::Fish => JobKind::Fish,
        proto::JobKind::ForageFibre => JobKind::ForageFibre,
        proto::JobKind::Explore => JobKind::Explore,
        proto::JobKind::FetchWater => JobKind::FetchWater,
        proto::JobKind::TrainWarrior => JobKind::TrainWarrior,
        proto::JobKind::ExpandVillage => JobKind::ExpandVillage,
        proto::JobKind::CarryOffering => JobKind::CarryOffering,
        proto::JobKind::PerformOffering => JobKind::PerformOffering,
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
        JobKind::GatherLogs => proto::JobKind::GatherLogs,
        JobKind::Fish => proto::JobKind::Fish,
        JobKind::ForageFibre => proto::JobKind::ForageFibre,
        JobKind::Explore => proto::JobKind::Explore,
        JobKind::FetchWater => proto::JobKind::FetchWater,
        JobKind::TrainWarrior => proto::JobKind::TrainWarrior,
        JobKind::ExpandVillage => proto::JobKind::ExpandVillage,
        JobKind::CarryOffering => proto::JobKind::CarryOffering,
        JobKind::PerformOffering => proto::JobKind::PerformOffering,
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
        proto::BuildingType::School => Some(BuildingType::School),
        proto::BuildingType::Smithy => Some(BuildingType::Smithy),
        proto::BuildingType::Barracks => Some(BuildingType::Barracks),
        proto::BuildingType::AccountingTent => Some(BuildingType::AccountingTent),
        proto::BuildingType::WoodCutter => Some(BuildingType::WoodCutter),
        proto::BuildingType::StonePrep => Some(BuildingType::StonePrep),
        proto::BuildingType::Woodworking => Some(BuildingType::Woodworking),
        proto::BuildingType::Clothier => Some(BuildingType::Clothier),
        proto::BuildingType::Tannery => Some(BuildingType::Tannery),
        proto::BuildingType::Smelter => Some(BuildingType::Smelter),
        proto::BuildingType::Mill => Some(BuildingType::Mill),
        proto::BuildingType::Sawmill => Some(BuildingType::Sawmill),
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
        BuildingType::AccountingTent => Some(proto::BuildingType::AccountingTent),
        BuildingType::Clothier => Some(proto::BuildingType::Clothier),
        BuildingType::Tannery => Some(proto::BuildingType::Tannery),
        BuildingType::ResearchHut => Some(proto::BuildingType::ResearchHut),
        BuildingType::School => Some(proto::BuildingType::School),
        // NOTE: cat-client's `building_texture`/`building_label` (exhaustive matches over
        // `proto::BuildingType`) do not have a Smelter sprite arm yet — flagged for
        // catclient3, see `crates/cat-protocol/src/lib.rs`'s `BuildingType::Smelter` doc.
        BuildingType::Smelter => Some(proto::BuildingType::Smelter),
        BuildingType::Mill => Some(proto::BuildingType::Mill),
        BuildingType::Sawmill => Some(proto::BuildingType::Sawmill),
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
        entities::CarryingKind::Fish => proto::CarryingKind::Fish,
        entities::CarryingKind::Blessings => proto::CarryingKind::Blessings,
        entities::CarryingKind::Materials => proto::CarryingKind::Materials,
        entities::CarryingKind::Refined => proto::CarryingKind::Refined,
        entities::CarryingKind::Logs => proto::CarryingKind::Logs,
        entities::CarryingKind::Lumber => proto::CarryingKind::Lumber,
        entities::CarryingKind::Planks => proto::CarryingKind::Planks,
        entities::CarryingKind::Blocks => proto::CarryingKind::Blocks,
        entities::CarryingKind::Tools => proto::CarryingKind::Tools,
        entities::CarryingKind::Water => proto::CarryingKind::Water,
        entities::CarryingKind::Catnip => proto::CarryingKind::Catnip,
        entities::CarryingKind::Grain => proto::CarryingKind::Grain,
        entities::CarryingKind::Flour => proto::CarryingKind::Flour,
        entities::CarryingKind::Herbs => proto::CarryingKind::Herbs,
        entities::CarryingKind::Ore => proto::CarryingKind::Ore,
        entities::CarryingKind::Metal => proto::CarryingKind::Metal,
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
    use crate::storage::GRANARY_BONUS;
    use crate::village_layout::VILLAGE_ANCHOR;
    use crate::world_tick::{
        BuildingRuntime, TraderRuntime, found_colony, found_global_colony, new_world,
        stockpile_placement_error,
    };

    fn ctx() -> ActionCtx {
        ActionCtx {
            session_id: "sess_1".to_string(),
            player_id: "player_1".to_string(),
            colony_id: "c1".to_string(),
            now_ms: 1_000_000,
        }
    }

    fn world_with_one_colony() -> WorldState {
        WorldState {
            world_seed: 20_240_703,
            colonies: vec![found_colony(20_240_703, "c1", 1_000_000, 1234)],
        }
    }

    fn completed_building(id: &str, building_type: BuildingType) -> BuildingRuntime {
        BuildingRuntime {
            id: id.to_owned(),
            building_type,
            level: 1,
            position: TilePos {
                x: VILLAGE_ANCHOR.x,
                y: VILLAGE_ANCHOR.y,
            },
            is_complete: true,
            construction_progress: 100,
            production_progress: 0.0,
            assigned_cat: None,
            automated_by: None,
            production_queue: crate::world_tick::default_production_queue(building_type),
            production_paused: false,
        }
    }

    fn signed_plan(building_type: proto::BuildingType) -> proto::ClientAction {
        proto::ClientAction::PlanBuilding {
            session_id: "sess_1".to_owned(),
            nickname: "Builder".to_owned(),
            sig: "server-verified".to_owned(),
            building_type,
            site: None,
        }
    }

    #[test]
    fn signed_building_plans_use_the_catalogs_single_bootstrap_and_mill_rules() {
        let mut bootstrap = world_with_one_colony();
        let mut bootstrap_twin = bootstrap.clone();
        let hut = signed_plan(proto::BuildingType::ResearchHut);
        let accepted = apply_action(&mut bootstrap, &hut, &ctx());
        let twin_accepted = apply_action(&mut bootstrap_twin, &hut, &ctx());
        assert!(accepted.ok, "founding hut was blocked: {accepted:?}");
        assert_eq!(accepted, twin_accepted);
        assert_eq!(
            bootstrap, bootstrap_twin,
            "signed bootstrap is deterministic"
        );
        assert_eq!(
            bootstrap.colonies[0].jobs.last().unwrap().kind,
            JobKind::BuildHouse
        );

        let mill = signed_plan(proto::BuildingType::Mill);
        let mut locked = world_with_one_colony();
        let denied = apply_action(&mut locked, &mill, &ctx());
        assert!(!denied.ok);
        assert_eq!(
            denied.message.as_deref(),
            Some("Research Milling before construction.")
        );

        locked.colonies[0]
            .upgrade_tree
            .owned_node_ids
            .push("mill_foundations".to_owned());
        let still_denied = apply_action(&mut locked, &mill, &ctx());
        assert!(!still_denied.ok);
        assert_eq!(still_denied.message, denied.message);

        locked.colonies[0]
            .upgrade_tree
            .owned_node_ids
            .push("milling".to_owned());
        let accepted = apply_action(&mut locked, &mill, &ctx());
        assert!(accepted.ok, "Milling did not unlock its mill: {accepted:?}");
    }

    #[test]
    fn personal_and_communal_founders_can_place_all_three_benches_without_basic_tools() {
        let benches = [
            (proto::BuildingType::WoodCutter, BuildingType::WoodCutter),
            (proto::BuildingType::StonePrep, BuildingType::StonePrep),
            (proto::BuildingType::Woodworking, BuildingType::Woodworking),
        ];

        for communal in [false, true] {
            for (protocol_type, sim_type) in benches {
                let mut without_study = new_world(20_240_703);
                let colony = if communal {
                    found_global_colony(20_240_703, "c1", 1_000_000, 1234)
                } else {
                    let mut colony = found_colony(20_240_703, "c1", 1_000_000, 1234);
                    colony.kind = VillageKind::Personal;
                    colony.owner_player_id = Some("player_1".to_owned());
                    colony
                };
                without_study.colonies.push(colony);
                let mut deterministic_twin = without_study.clone();
                let mut with_study = without_study.clone();
                with_study.colonies[0]
                    .upgrade_tree
                    .owned_node_ids
                    .push("basic_tools".to_owned());
                let action = signed_plan(protocol_type);

                let fresh_result = apply_action(&mut without_study, &action, &ctx());
                let twin_result = apply_action(&mut deterministic_twin, &action, &ctx());
                let studied_result = apply_action(&mut with_study, &action, &ctx());
                assert!(
                    fresh_result.ok,
                    "fresh {} {sim_type:?} placement was gated: {fresh_result:?}",
                    if communal { "communal" } else { "personal" }
                );
                assert_eq!(fresh_result, twin_result);
                assert_eq!(without_study, deterministic_twin);
                assert_eq!(fresh_result, studied_result);
                assert_eq!(without_study.colonies[0].jobs, with_study.colonies[0].jobs);
                assert_eq!(
                    without_study.colonies[0].resources,
                    with_study.colonies[0].resources
                );
                assert_eq!(
                    without_study.colonies[0].buildings,
                    with_study.colonies[0].buildings
                );
                assert!(without_study.colonies[0].jobs.iter().any(|job| {
                    job.kind == JobKind::BuildHouse && job_building_type(job) == Some(sim_type)
                }));
            }
        }
    }

    #[test]
    fn founding_bench_spatial_denial_is_mutation_free_without_basic_tools() {
        let action = proto::ClientAction::PlanBuilding {
            session_id: "sess_1".to_owned(),
            nickname: "Builder".to_owned(),
            sig: "server-verified".to_owned(),
            building_type: proto::BuildingType::Woodworking,
            site: Some(proto::TilePoint {
                x: VILLAGE_ANCHOR.x + 100_000,
                y: VILLAGE_ANCHOR.y + 100_000,
            }),
        };
        let mut world = world_with_one_colony();
        assert!(!upgrade_tree::is_owned(
            &world.colonies[0].upgrade_tree,
            "basic_tools"
        ));
        let before = world.clone();

        let denied = apply_action(&mut world, &action, &ctx());

        assert!(!denied.ok);
        assert_eq!(world, before);
    }

    #[test]
    fn founding_bench_exact_scaffold_keeps_atomic_cost_and_occupancy_rules() {
        let mut world = world_with_one_colony();
        let colony = &mut world.colonies[0];
        colony.resources.lumber = 40.0;
        colony.resources.planks = 40.0;
        colony.resources.blocks = 40.0;
        colony
            .officers
            .insert(OfficerRole::Steward, colony.cats[0].id.clone());
        let founding_index = colony
            .buildings
            .iter()
            .position(|building| building.building_type == BuildingType::WoodCutter)
            .expect("founding blueprint wood cutter");
        let site = colony.buildings.remove(founding_index).position;
        assert!(crate::world_tick::can_plan_building_at(
            colony,
            site,
            world.world_seed,
            BuildingType::WoodCutter,
        ));
        let exact_plan = |building_type| proto::ClientAction::PlanBuilding {
            session_id: "sess_1".to_owned(),
            nickname: "Builder".to_owned(),
            sig: "server-verified".to_owned(),
            building_type,
            site: Some(proto::TilePoint {
                x: site.x,
                y: site.y,
            }),
        };
        let before = world.colonies[0].clone();

        let placed = apply_action(
            &mut world,
            &exact_plan(proto::BuildingType::WoodCutter),
            &ctx(),
        );
        assert!(placed.ok, "exact founding bench denied: {placed:?}");
        let paid = world.colonies[0].clone();
        assert!(paid.resources.blocks < before.resources.blocks);
        assert!(
            paid.resources.lumber < before.resources.lumber
                || paid.resources.planks < before.resources.planks
        );
        assert!(paid.buildings.iter().any(|building| {
            building.building_type == BuildingType::WoodCutter
                && building.position == site
                && !building.is_complete
        }));

        let overlap = apply_action(
            &mut world,
            &exact_plan(proto::BuildingType::StonePrep),
            &ctx(),
        );
        assert!(!overlap.ok);
        assert_eq!(world.colonies[0], paid);

        let mut unfunded = before.clone();
        unfunded.resources.lumber = 0.0;
        unfunded.resources.planks = 0.0;
        unfunded.resources.blocks = 0.0;
        let mut unfunded_world = WorldState {
            world_seed: world.world_seed,
            colonies: vec![unfunded],
        };
        let unfunded_before = unfunded_world.clone();
        let denied = apply_action(
            &mut unfunded_world,
            &exact_plan(proto::BuildingType::WoodCutter),
            &ctx(),
        );
        assert!(!denied.ok);
        assert_eq!(unfunded_world, unfunded_before);
    }

    #[test]
    fn guided_player_can_bootstrap_research_then_purchase_and_place_a_mill() {
        let mut guided = world_with_one_colony();
        guided.colonies[0].upgrade_tree.research_points = 100.0;
        let mut twin = guided.clone();
        let actions = [
            signed_plan(proto::BuildingType::ResearchHut),
            proto::ClientAction::ResearchNode {
                session_id: "sess_1".to_owned(),
                nickname: "Scholar".to_owned(),
                sig: "server-verified".to_owned(),
                node_id: "research_hut".to_owned(),
            },
            proto::ClientAction::ResearchNode {
                session_id: "sess_1".to_owned(),
                nickname: "Scholar".to_owned(),
                sig: "server-verified".to_owned(),
                node_id: "water_carriers".to_owned(),
            },
            proto::ClientAction::ResearchNode {
                session_id: "sess_1".to_owned(),
                nickname: "Scholar".to_owned(),
                sig: "server-verified".to_owned(),
                node_id: "irrigation".to_owned(),
            },
            proto::ClientAction::ResearchNode {
                session_id: "sess_1".to_owned(),
                nickname: "Scholar".to_owned(),
                sig: "server-verified".to_owned(),
                node_id: "milling".to_owned(),
            },
            signed_plan(proto::BuildingType::Mill),
        ];
        for action in actions {
            let accepted = apply_action(&mut guided, &action, &ctx());
            let twin_accepted = apply_action(&mut twin, &action, &ctx());
            assert!(
                accepted.ok,
                "guided action failed: {action:?}: {accepted:?}"
            );
            assert_eq!(accepted, twin_accepted);
        }
        assert_eq!(guided, twin, "guided research/build campaign diverged");
        assert!(
            guided.colonies[0]
                .upgrade_tree
                .owned_node_ids
                .iter()
                .any(|id| id == "milling")
        );
        assert_eq!(
            guided.colonies[0].last_leader_research_choice_at, None,
            "manual purchases must not consume the Leader's daily choice"
        );
        assert!(guided.colonies[0].jobs.iter().any(|job| {
            job.kind == JobKind::BuildHouse
                && job_building_type(job) == Some(BuildingType::ResearchHut)
        }));
        assert!(guided.colonies[0].jobs.iter().any(|job| {
            job.kind == JobKind::BuildHouse && job_building_type(job) == Some(BuildingType::Mill)
        }));
    }

    #[test]
    fn fresh_personal_and_communal_capabilities_do_not_inherit_false_job_gates() {
        let signed_job = |kind| proto::ClientAction::RequestJob {
            session_id: "sess_1".to_owned(),
            nickname: "Founder".to_owned(),
            sig: "server-verified".to_owned(),
            kind,
        };

        for kind in [proto::JobKind::FetchWater, proto::JobKind::Explore] {
            let mut world = world_with_one_colony();
            world.colonies[0].kind = VillageKind::Personal;
            world.colonies[0].owner_player_id = Some("player_1".to_owned());
            let result = apply_action(&mut world, &signed_job(kind), &ctx());
            assert!(result.ok, "fresh personal {kind:?} was gated: {result:?}");
        }

        let mut personal = world_with_one_colony();
        personal.colonies[0].kind = VillageKind::Personal;
        personal.colonies[0].owner_player_id = Some("player_1".to_owned());
        personal.colonies[0].upgrade_tree.research_points = 5.0;
        let research = apply_action(
            &mut personal,
            &proto::ClientAction::ResearchNode {
                session_id: "sess_1".to_owned(),
                nickname: "Founder".to_owned(),
                sig: "server-verified".to_owned(),
                node_id: "research_hut".to_owned(),
            },
            &ctx(),
        );
        assert!(
            research.ok,
            "manual founding research was gated: {research:?}"
        );

        let mut communal = WorldState {
            world_seed: 20_240_703,
            colonies: vec![found_global_colony(20_240_703, "c1", 1_000_000, 1234)],
        };
        communal.colonies[0].buildings.push(completed_building(
            "communal-barracks",
            BuildingType::Barracks,
        ));
        let training = apply_action(
            &mut communal,
            &signed_job(proto::JobKind::TrainWarrior),
            &ctx(),
        );
        assert!(
            training.ok,
            "communal Barracks training was falsely research-gated: {training:?}"
        );
        assert!(
            communal.colonies[0]
                .jobs
                .iter()
                .any(|job| job.kind == JobKind::TrainWarrior)
        );
    }

    #[test]
    fn vote_kick_petition_accepts_five_distinct_players_and_is_idempotent_per_player() {
        let mut world = world_with_one_colony();
        let original_leader = world.colonies[0].cats[0].id.clone();
        world.colonies[0].leader_id = Some(original_leader.clone());
        let opened_at = 1_010_000;
        for player_index in 0..5 {
            let action_ctx = ActionCtx {
                session_id: format!("session-{player_index}"),
                player_id: format!("player-{player_index}"),
                colony_id: "c1".to_owned(),
                now_ms: opened_at + i64::from(player_index),
            };
            let action = proto::ClientAction::RequestVoteKick {
                session_id: action_ctx.session_id.clone(),
                nickname: format!("Voter {player_index}"),
                sig: "server-verified".to_owned(),
            };
            assert!(apply_action(&mut world, &action, &action_ctx).ok);
            if player_index == 0 {
                assert!(apply_action(&mut world, &action, &action_ctx).ok);
                assert_eq!(
                    world.colonies[0].votes.len(),
                    1,
                    "a reconnect/double-click cannot duplicate one signature"
                );
            }
        }
        let petition = world.colonies[0]
            .elections
            .iter()
            .find(|election| election.kind == ElectionKind::VoteKick)
            .expect("one petition");
        assert_eq!(
            petition.winner_cat_id.as_deref(),
            Some(original_leader.as_str())
        );
        assert_eq!(world.colonies[0].votes.len(), 5);
        let closes_at = petition.closes_at;

        let _ = world_tick(&mut world, closes_at);
        assert_ne!(
            world.colonies[0].leader_id.as_deref(),
            Some(original_leader.as_str()),
            "five stable signed identities remove the petition target"
        );
        assert!(world.colonies[0].elections.iter().any(|election| {
            election.kind == ElectionKind::VoteKick && election.resolved_at == Some(closes_at)
        }));
    }

    #[test]
    fn cast_vote_accepts_only_candidates_exposed_by_the_live_election_snapshot() {
        let mut world = world_with_one_colony();
        world.colonies[0].elections.push(ElectionRuntime {
            id: "election-live".to_owned(),
            opened_at: 999_000,
            closes_at: 2_000_000,
            resolved_at: None,
            winner_cat_id: None,
            kind: ElectionKind::Scheduled,
        });
        let snapshot = build_snapshot(&world, ctx().now_ms, 1);
        let election = snapshot.colonies[0]
            .election
            .as_ref()
            .expect("live election snapshot");
        let candidate = election.candidates[0].id.clone();
        let non_candidate = world.colonies[0]
            .cats
            .iter()
            .find(|cat| !election.candidates.iter().any(|listed| listed.id == cat.id))
            .expect("founding roster exceeds candidate cap")
            .id
            .clone();
        let vote = |cat_id| proto::ClientAction::CastVote {
            session_id: "sess_1".to_owned(),
            nickname: "Voter".to_owned(),
            sig: "server-verified".to_owned(),
            election_id: "election-live".to_owned(),
            cat_id,
        };
        let rejected = apply_action(&mut world, &vote(non_candidate), &ctx());
        assert!(!rejected.ok);
        assert!(world.colonies[0].votes.is_empty());
        let accepted = apply_action(&mut world, &vote(candidate.clone()), &ctx());
        assert!(accepted.ok, "{accepted:?}");
        assert_eq!(world.colonies[0].votes[0].cat_id, candidate);
    }

    #[test]
    fn typed_player_scout_action_dispatches_the_best_available_vision_cat() {
        let mut world = world_with_one_colony();
        let expected = world.colonies[0]
            .cats
            .iter()
            .max_by(|a, b| {
                a.stats
                    .vision
                    .total_cmp(&b.stats.vision)
                    .then_with(|| b.id.cmp(&a.id))
            })
            .expect("founding cat")
            .id
            .clone();
        let action = proto::ClientAction::DispatchScout {
            session_id: "sess_1".to_owned(),
            nickname: "Player".to_owned(),
            sig: "signed".to_owned(),
            mission: proto::ScoutMission::Resource(proto::ScoutResource::Wood),
        };

        let result = apply_action(&mut world, &action, &ctx());

        assert!(result.ok, "{result:?}");
        let job = world.colonies[0].jobs.last().expect("scout job");
        assert_eq!(job.kind, JobKind::Explore);
        assert_eq!(job.requested_by, JobRequester::Player);
        assert_eq!(job.assigned_cat.as_deref(), Some(expected.as_str()));
        assert!(matches!(
            job.metadata,
            JobMetadata::Scout {
                mission: ScoutMission::Resource(ScoutResource::Wood),
                target: None,
                destination: None,
                accepted: false,
                found: false,
            }
        ));
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
    fn farm_designation_requires_claimed_expansion_outside_the_wall_and_can_be_cleared() {
        let mut world = world_with_one_colony();
        let anchor = world.colonies[0].anchor;
        let inside = proto::ClientAction::DesignateFarm {
            session_id: "sess_1".to_owned(),
            nickname: "Tester".to_owned(),
            sig: "server-verified".to_owned(),
            a: proto::TilePoint {
                x: anchor.x + 6,
                y: anchor.y,
            },
            b: proto::TilePoint {
                x: anchor.x + 6,
                y: anchor.y,
            },
            crop: proto::CropKind::Grain,
        };
        let rejected = apply_action(&mut world, &inside, &ctx());
        assert!(!rejected.ok);
        assert_eq!(
            rejected.message.as_deref(),
            Some("Farm plots belong outside the walled village.")
        );

        let seed = world.world_seed;
        let expanded = world.colonies[0]
            .world_tiles
            .keys()
            .copied()
            .find(|tile| {
                (tile.x - anchor.x).abs().max((tile.y - anchor.y).abs()) > 6
                    && crate::terrain_gen::tile_climate_biome(seed, tile.x, tile.y)
                        .properties()
                        .fertility
                        > 0.0
                    && !crate::world_tick::tile_is_occupied(&world.colonies[0], *tile, seed)
            })
            .expect("starter chunks contain fertile expansion ground");
        for y in (expanded.y - 1)..=(expanded.y + 1) {
            for x in (expanded.x - 1)..=(expanded.x + 1) {
                let tile = TilePos { x, y };
                world.colonies[0].claimed_tiles.push(tile);
                world.colonies[0].revealed_tiles.insert(tile);
            }
        }
        let mut cleared = world.colonies[0].world_tiles[&expanded].clone();
        cleared.tile_type = TileType::Field;
        cleared.resources.water = 0;
        cleared.overlay_feature = Some("stump".to_owned());
        let (min_x, max_x) = if anchor.x <= expanded.x {
            (anchor.x, expanded.x)
        } else {
            (expanded.x, anchor.x)
        };
        let (min_y, max_y) = if anchor.y <= expanded.y {
            (anchor.y, expanded.y)
        } else {
            (expanded.y, anchor.y)
        };
        let route_claim = (min_x..=max_x)
            .map(|x| TilePos { x, y: anchor.y })
            .chain((min_y..=max_y).map(|y| TilePos { x: expanded.x, y }));
        for tile in route_claim {
            if !world.colonies[0].claimed_tiles.contains(&tile) {
                world.colonies[0].claimed_tiles.push(tile);
            }
            world.colonies[0].revealed_tiles.insert(tile);
            let mut runtime = cleared.clone();
            runtime.pos = tile;
            world.colonies[0].world_tiles.insert(tile, runtime);
        }
        let enclosed = proto::ClientAction::DesignateFarm {
            session_id: "sess_1".to_owned(),
            nickname: "Tester".to_owned(),
            sig: "server-verified".to_owned(),
            a: proto::TilePoint {
                x: expanded.x,
                y: expanded.y,
            },
            b: proto::TilePoint {
                x: expanded.x,
                y: expanded.y,
            },
            crop: proto::CropKind::Grain,
        };
        let rejected = apply_action(&mut world, &enclosed, &ctx());
        assert!(
            !rejected.ok,
            "enclosed agricultural hole accepted: {rejected:?}"
        );
        assert_eq!(
            rejected.message.as_deref(),
            Some("Farm plots must connect to the claimed exterior boundary.")
        );
        assert!(world.colonies[0].farms.is_empty());

        let reopened_exterior = TilePos {
            x: expanded.x + 1,
            y: expanded.y,
        };
        world.colonies[0]
            .claimed_tiles
            .retain(|tile| *tile != reopened_exterior);
        let designation = proto::ClientAction::DesignateFarm {
            session_id: "sess_1".to_owned(),
            nickname: "Tester".to_owned(),
            sig: "server-verified".to_owned(),
            a: proto::TilePoint {
                x: expanded.x,
                y: expanded.y,
            },
            b: proto::TilePoint {
                x: expanded.x,
                y: expanded.y,
            },
            crop: proto::CropKind::Grain,
        };
        let accepted = apply_action(&mut world, &designation, &ctx());
        assert!(accepted.ok, "{accepted:?}");
        assert_eq!(world.colonies[0].farms.len(), 1);
        assert!(world.colonies[0].agricultural_tiles.contains(&expanded));
        let expected_fertility = crate::world_tick::tile_farm_fertility(
            seed,
            expanded,
            world.colonies[0].world_tiles.get(&expanded),
        );
        assert_eq!(
            world.colonies[0].farms[0].fertility.to_bits(),
            expected_fertility.to_bits(),
            "signed designation uses the same prepared-ground fertility truth as automation"
        );

        let snapshot = build_snapshot(&world, ctx().now_ms, 1);
        assert_eq!(snapshot.colonies[0].farms.len(), 1);
        assert_eq!(snapshot.colonies[0].farms[0].crop, proto::CropKind::Grain);
        let plot_id = world.colonies[0].farms[0].id.clone();
        let gather_id = farm_gather_spot_id(&plot_id);
        let gather_tile = world.colonies[0]
            .stockpiles
            .iter()
            .find(|pile| pile.id == gather_id)
            .map(|pile| TilePos {
                x: pile.rect.x1,
                y: pile.rect.y1,
            })
            .unwrap();
        assert!(world.colonies[0].agricultural_tiles.contains(&gather_tile));
        assert!(
            world.colonies[0]
                .gather_spots
                .iter()
                .any(|spot| spot.stockpile_id == gather_id)
        );
        let cleared = apply_action(
            &mut world,
            &proto::ClientAction::ClearFarm {
                session_id: "sess_1".to_owned(),
                nickname: "Tester".to_owned(),
                sig: "server-verified".to_owned(),
                plot_id: plot_id.clone(),
            },
            &ctx(),
        );
        assert!(cleared.ok, "{cleared:?}");
        assert!(world.colonies[0].farms.is_empty());
        assert!(!world.colonies[0].agricultural_tiles.contains(&expanded));
        assert!(!world.colonies[0].agricultural_tiles.contains(&gather_tile));
        assert!(
            world.colonies[0]
                .stockpiles
                .iter()
                .all(|pile| pile.id != gather_id)
        );
        let recreated = apply_action(&mut world, &designation, &ctx());
        assert!(
            recreated.ok,
            "cleared farm tiles can be designated again: {recreated:?}"
        );
        assert_eq!(world.colonies[0].farms.len(), 1);
    }

    #[test]
    fn farm_designation_rejects_claimed_revealed_but_unreachable_land() {
        let mut world = world_with_one_colony();
        let world_seed = world.world_seed;
        let colony = &mut world.colonies[0];
        let anchor = colony.anchor;
        let barrier_x = anchor.x + 9;
        let min_y = colony.world_tiles.keys().map(|tile| tile.y).min().unwrap();
        let max_y = colony.world_tiles.keys().map(|tile| tile.y).max().unwrap();
        let target = colony
            .world_tiles
            .keys()
            .copied()
            .filter(|tile| tile.x > barrier_x)
            .filter(|tile| {
                colony.world_tiles.contains_key(&TilePos {
                    x: tile.x + 1,
                    y: tile.y,
                })
            })
            .filter(|tile| {
                crate::terrain_gen::tile_climate_biome(world_seed, tile.x, tile.y)
                    .properties()
                    .fertility
                    > 0.0
            })
            .min_by_key(|tile| ((tile.y - anchor.y).abs(), tile.x, tile.y))
            .expect("mapped fertile test tile beyond barrier");
        for y in min_y..=max_y {
            if let Some(tile) = colony.world_tiles.get_mut(&TilePos { x: barrier_x, y }) {
                tile.tile_type = TileType::River;
                tile.resources.water = 999;
                tile.overlay_feature = Some("river".to_owned());
            }
        }
        for tile in [
            target,
            TilePos {
                x: target.x + 1,
                y: target.y,
            },
        ] {
            let runtime = colony.world_tiles.get_mut(&tile).expect("mapped test tile");
            runtime.tile_type = TileType::Field;
            runtime.resources.water = 0;
            runtime.overlay_feature = Some("stump".to_owned());
            colony.claimed_tiles.push(tile);
            colony.revealed_tiles.insert(tile);
        }
        let result = apply_action(
            &mut world,
            &proto::ClientAction::DesignateFarm {
                session_id: "sess_1".to_owned(),
                nickname: "Tester".to_owned(),
                sig: "server-verified".to_owned(),
                a: tp(target.x, target.y),
                b: tp(target.x, target.y),
                crop: proto::CropKind::Grain,
            },
            &ctx(),
        );
        assert!(!result.ok, "river-sealed plot accepted: {result:?}");
        assert!(
            result
                .message
                .as_deref()
                .is_some_and(|message| message.contains("reachable"))
        );
        assert!(world.colonies[0].farms.is_empty());
    }

    #[test]
    fn gather_logs_requires_research_a_live_worker_and_an_explored_tree() {
        let mut world = world_with_one_colony();
        let action = proto::ClientAction::RequestJob {
            session_id: "sess_1".to_owned(),
            nickname: "Tester".to_owned(),
            sig: "server-verified".to_owned(),
            kind: proto::JobKind::GatherLogs,
        };
        let before_locked = world.clone();
        let locked = apply_action(&mut world, &action, &ctx());
        assert!(!locked.ok);
        assert_eq!(
            locked.message.as_deref(),
            Some("Research Sawmill before requesting logging.")
        );
        assert_eq!(world, before_locked, "denial must be mutation-free");

        world.colonies[0]
            .upgrade_tree
            .owned_node_ids
            .push("sawmill".to_owned());
        let seed = world.world_seed;
        let tree = (-12..=12)
            .flat_map(|chunk_y| (-12..=12).map(move |chunk_x| (chunk_x, chunk_y)))
            .find_map(|(chunk_x, chunk_y)| {
                crate::terrain_gen::generate_terrain_chunk(
                    chunk_x,
                    chunk_y,
                    i64::from(seed),
                    crate::terrain_gen::WORLD_TERRAIN_OPTIONS,
                )
                .into_iter()
                .find(|tile| {
                    matches!(
                        tile.decoration,
                        Some(crate::terrain_gen::DecorationRole::Tree { .. })
                    ) && tile.climate_biome.properties().resource
                        == crate::climate::ResourceHint::Wood
                })
                .map(|tile| TilePos {
                    x: tile.x,
                    y: tile.y,
                })
            })
            .expect("bounded climate scan contains a logging tree");
        let mut logging_tile = world.colonies[0]
            .world_tiles
            .values()
            .next()
            .expect("founding world tile")
            .clone();
        logging_tile.pos = tree;
        logging_tile.path_wear = 63;
        logging_tile.overlay_feature = None;
        world.colonies[0].world_tiles.insert(tree, logging_tile);

        let mut with_tools = world.clone();
        with_tools.colonies[0].resources.tools = 20.0;
        let accepted = apply_action(&mut world, &action, &ctx());
        assert!(accepted.ok, "{accepted:?}");
        let job = world.colonies[0].jobs.last().expect("logging job queued");
        assert_eq!(job.kind, JobKind::GatherLogs);
        assert!(job.assigned_cat.is_some());
        let baseline_duration = job.duration_ms;
        let accepted_with_tools = apply_action(&mut with_tools, &action, &ctx());
        assert!(accepted_with_tools.ok, "{accepted_with_tools:?}");
        let tool_job = with_tools.colonies[0]
            .jobs
            .last()
            .expect("tool-assisted logging job queued");
        assert!(tool_job.duration_ms < baseline_duration);
        assert_eq!(
            with_tools.colonies[0].resources.tools, 20.0,
            "the productivity reserve is reusable equipment, not consumed input"
        );

        let mut no_forest = world_with_one_colony();
        no_forest.colonies[0]
            .upgrade_tree
            .owned_node_ids
            .push("sawmill".to_owned());
        for tile in no_forest.colonies[0].world_tiles.values_mut() {
            tile.overlay_feature = Some("stump".to_owned());
        }
        let rejected = apply_action(&mut no_forest, &action, &ctx());
        assert!(!rejected.ok);
        assert_eq!(
            rejected.message.as_deref(),
            Some("No explored forest is available for logging.")
        );
    }

    #[test]
    fn banked_tools_accelerate_player_quarry_without_being_consumed() {
        let mut baseline = world_with_one_colony();
        baseline.colonies[0].jobs.clear();
        let mut equipped = baseline.clone();
        equipped.colonies[0].resources.tools = 20.0;
        let action = proto::ClientAction::RequestJob {
            session_id: "sess_1".to_owned(),
            nickname: "Tester".to_owned(),
            sig: "server-verified".to_owned(),
            kind: proto::JobKind::Quarry,
        };

        assert!(apply_action(&mut baseline, &action, &ctx()).ok);
        assert!(apply_action(&mut equipped, &action, &ctx()).ok);
        let baseline_ms = baseline.colonies[0].jobs.last().unwrap().duration_ms;
        let equipped_ms = equipped.colonies[0].jobs.last().unwrap().duration_ms;
        assert_eq!(
            equipped_ms,
            productive_duration_ms(baseline_ms, 20.0),
            "1.20x tool throughput shortens duration to five-sixths"
        );
        assert_eq!(equipped.colonies[0].resources.tools, 20.0);
    }

    #[test]
    fn found_village_adds_a_colony_with_starter_cats() {
        let mut world = world_with_one_colony();
        let res = apply_action(
            &mut world,
            &proto::ClientAction::FoundVillage {
                name: "Newford".to_string(),
                session_id: "sess_1".to_string(),
                sig: None,
            },
            &ctx(),
        );
        assert!(res.ok, "{res:?}");
        assert_eq!(world.colonies.len(), 2);
        let personal = &world.colonies[1];
        assert_eq!(personal.kind, VillageKind::Personal);
        assert_eq!(personal.owner_player_id.as_deref(), Some("player_1"));
        assert_ne!(personal.anchor, world.colonies[0].anchor);
        assert!(
            !personal.cats.is_empty(),
            "founded colony should have starter cats"
        );
        assert_eq!(res.colony_id.as_deref(), Some(personal.id.as_str()));
    }

    #[test]
    fn founding_is_idempotent_and_stable_for_one_player() {
        let mut world = world_with_one_colony();
        let action = proto::ClientAction::FoundVillage {
            name: "Newford".to_owned(),
            session_id: "sess_1".to_owned(),
            sig: None,
        };

        let first = apply_action(&mut world, &action, &ctx());
        let first_personal = world.colonies[1].clone();
        let second = apply_action(&mut world, &action, &ctx());

        assert!(first.ok && second.ok);
        assert_eq!(world.colonies.len(), 2);
        assert_eq!(world.colonies[1], first_personal);
        assert_eq!(first.colony_id, second.colony_id);
    }

    #[test]
    fn village_names_are_bounded_and_reject_control_characters() {
        let action = |name: String| proto::ClientAction::FoundVillage {
            name,
            session_id: "sess_1".to_owned(),
            sig: None,
        };
        for name in [
            "".to_owned(),
            "x".repeat(MAX_VILLAGE_NAME_CHARS + 1),
            "Moss\nHollow".to_owned(),
        ] {
            let mut world = world_with_one_colony();
            let before = world.clone();
            assert!(!apply_action(&mut world, &action(name), &ctx()).ok);
            assert_eq!(world, before);
        }

        let mut world = world_with_one_colony();
        let boundary = "x".repeat(MAX_VILLAGE_NAME_CHARS);
        assert!(apply_action(&mut world, &action(boundary.clone()), &ctx()).ok);
        assert_eq!(world.colonies[1].name, boundary);
    }

    #[test]
    fn personal_village_id_collision_search_is_bounded() {
        let mut world = world_with_one_colony();
        let player_id = "collision-player";
        let base_id = personal_village_id(&world, player_id).expect("initial id");
        for suffix in 0..=MAX_PERSONAL_VILLAGE_ID_COLLISIONS {
            let id = if suffix == 0 {
                base_id.clone()
            } else {
                format!("{base_id}-{suffix}")
            };
            world.colonies.push(ColonyRuntime {
                id,
                ..ColonyRuntime::default()
            });
        }

        assert_eq!(personal_village_id(&world, player_id), None);
    }

    #[test]
    fn action_context_routes_mutations_to_the_selected_colony() {
        let mut world = world_with_one_colony();
        let world_seed = world.world_seed;
        world
            .colonies
            .push(found_colony(world_seed, "c2", 1_000_000, 5678));
        world.colonies[1].kind = VillageKind::Personal;
        world.colonies[1].owner_player_id = Some("player_1".to_owned());
        let cat_id = world.colonies[1].cats[0].id.clone();
        grant_officer_prerequisite(&mut world.colonies[1], OfficerRole::Farmer);
        let mut selected_ctx = ctx();
        selected_ctx.colony_id = "c2".to_owned();
        let action = proto::ClientAction::AssignOfficer {
            session_id: selected_ctx.session_id.clone(),
            nickname: "Tester".to_owned(),
            sig: "server-verified".to_owned(),
            role: proto::OfficerRole::Farmer,
            cat_id: cat_id.clone(),
        };

        let result = apply_action(&mut world, &action, &selected_ctx);

        assert!(result.ok, "{result:?}");
        assert!(world.colonies[0].officers.is_empty());
        assert_eq!(
            world.colonies[1].officers.get(&OfficerRole::Farmer),
            Some(&cat_id)
        );
    }

    #[test]
    fn foreign_player_cannot_join_or_mutate_a_personal_village() {
        let mut world = world_with_one_colony();
        let world_seed = world.world_seed;
        let mut private = found_colony(world_seed, "private", 1_000_000, 5678);
        private.kind = VillageKind::Personal;
        private.owner_player_id = Some("owner".to_owned());
        let cat_id = private.cats[0].id.clone();
        world.colonies.push(private);
        let mut intruder = ctx();
        intruder.player_id = "intruder".to_owned();
        intruder.colony_id = "private".to_owned();

        let join = apply_action(
            &mut world,
            &proto::ClientAction::JoinVillage {
                colony_id: "private".to_owned(),
                session_id: intruder.session_id.clone(),
                sig: None,
            },
            &intruder,
        );
        let before = world.colonies[1].clone();
        let mutation = apply_action(
            &mut world,
            &proto::ClientAction::AssignOfficer {
                session_id: intruder.session_id.clone(),
                nickname: "Intruder".to_owned(),
                sig: "server-verified".to_owned(),
                role: proto::OfficerRole::Farmer,
                cat_id,
            },
            &intruder,
        );

        assert!(!join.ok);
        assert!(!mutation.ok);
        assert_eq!(world.colonies[1], before);
        assert_eq!(join.message, mutation.message);
    }

    #[test]
    fn only_shrine_delivered_scout_knowledge_creates_mutual_contact() {
        let mut world = world_with_one_colony();
        let mut personal = found_colony_at(
            world.world_seed,
            "personal",
            1_000_000,
            55,
            TilePos { x: 102, y: 6 },
        );
        personal.kind = VillageKind::Personal;
        personal.owner_player_id = Some("player_2".to_owned());
        let personal_shrine = TilePos {
            x: personal.anchor.x + 1,
            y: personal.anchor.y + 1,
        };
        world.colonies[0]
            .provisional_tiles
            .entry("scout".to_owned())
            .or_default()
            .insert(personal_shrine);
        world.colonies.push(personal);

        crate::world_tick::reconcile_village_discoveries(&mut world);
        assert!(world.colonies[0].known_village_ids.is_empty());
        assert!(world.colonies[1].known_village_ids.is_empty());

        // Expansion, recovery, and legacy saves can all permanently reveal a
        // tile. That generic map state must not impersonate a returned scout.
        world.colonies[0].revealed_tiles.insert(personal_shrine);
        crate::world_tick::reconcile_village_discoveries(&mut world);
        assert!(world.colonies[0].known_village_ids.is_empty());
        assert!(world.colonies[1].known_village_ids.is_empty());

        world.colonies[0]
            .pending_scout_delivery_tiles
            .insert(personal_shrine);
        crate::world_tick::reconcile_village_discoveries(&mut world);
        assert!(world.colonies[0].known_village_ids.contains("personal"));
        assert!(world.colonies[1].known_village_ids.contains("c1"));
        assert!(world.colonies[0].pending_scout_delivery_tiles.is_empty());
    }

    #[test]
    fn discovered_villages_exchange_resources_only_after_target_acceptance() {
        let mut world = world_with_one_colony();
        let mut personal = found_colony(world.world_seed, "personal", 1_000_000, 55);
        personal.kind = VillageKind::Personal;
        personal.owner_player_id = Some("player_2".to_owned());
        world.colonies[0]
            .known_village_ids
            .insert("personal".to_owned());
        personal.known_village_ids.insert("c1".to_owned());
        world.colonies.push(personal);
        world.colonies[0].resources.food = 100.0;
        world.colonies[1].resources.materials = 100.0;
        let before = (
            world.colonies[0].resources.clone(),
            world.colonies[1].resources.clone(),
        );

        let offered = apply_action(
            &mut world,
            &proto::ClientAction::OfferVillageTrade {
                session_id: "sess_1".to_owned(),
                nickname: "Global Cat".to_owned(),
                sig: "signed".to_owned(),
                target_colony_id: "personal".to_owned(),
                offered_kind: proto::ResourceKind::Food,
                offered_amount: 10.0,
                requested_kind: proto::ResourceKind::Materials,
                requested_amount: 5.0,
            },
            &ctx(),
        );
        assert!(offered.ok, "{offered:?}");
        assert_eq!(world.colonies[0].resources, before.0);
        assert_eq!(world.colonies[1].resources, before.1);
        let offer_id = world.colonies[0]
            .village_trade_offers
            .keys()
            .next()
            .expect("offer")
            .clone();
        let mut target_ctx = ctx();
        target_ctx.player_id = "player_2".to_owned();
        target_ctx.colony_id = "personal".to_owned();
        let accepted = apply_action(
            &mut world,
            &proto::ClientAction::AcceptVillageTrade {
                session_id: target_ctx.session_id.clone(),
                nickname: "Personal Cat".to_owned(),
                sig: "signed".to_owned(),
                offer_id,
            },
            &target_ctx,
        );

        assert!(accepted.ok, "{accepted:?}");
        assert_eq!(world.colonies[0].resources.food, 90.0);
        assert_eq!(
            world.colonies[0].resources.materials,
            before.0.materials + 5.0
        );
        assert_eq!(world.colonies[1].resources.food, before.1.food + 10.0);
        assert_eq!(world.colonies[1].resources.materials, 95.0);
        assert!(world.colonies[0].village_trade_offers.is_empty());
    }

    #[test]
    fn unknown_or_foreign_villages_cannot_create_or_accept_trade() {
        let mut world = world_with_one_colony();
        let mut personal = found_colony(world.world_seed, "personal", 1_000_000, 55);
        personal.kind = VillageKind::Personal;
        personal.owner_player_id = Some("owner".to_owned());
        world.colonies.push(personal);
        let action = proto::ClientAction::OfferVillageTrade {
            session_id: "sess_1".to_owned(),
            nickname: "Global Cat".to_owned(),
            sig: "signed".to_owned(),
            target_colony_id: "personal".to_owned(),
            offered_kind: proto::ResourceKind::Food,
            offered_amount: 1.0,
            requested_kind: proto::ResourceKind::Materials,
            requested_amount: 1.0,
        };
        assert!(!apply_action(&mut world, &action, &ctx()).ok);
        assert!(world.colonies[0].village_trade_offers.is_empty());
    }

    #[test]
    fn trade_offer_identity_collision_search_is_bounded() {
        let mut world = world_with_one_colony();
        let mut personal = found_colony(world.world_seed, "personal", 1_000_000, 55);
        personal.kind = VillageKind::Personal;
        personal.owner_player_id = Some("owner".to_owned());
        world.colonies[0]
            .known_village_ids
            .insert("personal".to_owned());
        personal.known_village_ids.insert("c1".to_owned());
        world.colonies.push(personal);
        let context = ctx();
        let base = stable_seed(&[
            "idle-cat-forest/village-trade/v1",
            "c1",
            "personal",
            &context.now_ms.to_string(),
            &context.player_id,
        ]);
        for suffix in 0..=MAX_VILLAGE_TRADE_ID_COLLISIONS {
            let id = if suffix == 0 {
                format!("trade-{base:08x}")
            } else {
                format!("trade-{base:08x}-{suffix}")
            };
            world.colonies[1].village_trade_offers.insert(
                id.clone(),
                VillageTradeOffer {
                    id,
                    from_colony_id: "personal".to_owned(),
                    to_colony_id: "c1".to_owned(),
                    offered_kind: stockpiles::ResourceKind::Food,
                    offered_amount: 1.0,
                    requested_kind: stockpiles::ResourceKind::Materials,
                    requested_amount: 1.0,
                    created_at: context.now_ms,
                },
            );
        }
        let count = world.colonies[1].village_trade_offers.len();

        let result = apply_action(
            &mut world,
            &proto::ClientAction::OfferVillageTrade {
                session_id: context.session_id.clone(),
                nickname: "Global Cat".to_owned(),
                sig: "signed".to_owned(),
                target_colony_id: "personal".to_owned(),
                offered_kind: proto::ResourceKind::Food,
                offered_amount: 1.0,
                requested_kind: proto::ResourceKind::Materials,
                requested_amount: 1.0,
            },
            &context,
        );

        assert!(!result.ok);
        assert_eq!(world.colonies[1].village_trade_offers.len(), count);
        assert!(world.colonies[0].village_trade_offers.is_empty());
    }

    #[test]
    fn a_village_cannot_accumulate_unbounded_open_trade_offers() {
        let mut world = world_with_one_colony();
        let mut personal = found_colony(world.world_seed, "personal", 1_000_000, 55);
        personal.kind = VillageKind::Personal;
        personal.owner_player_id = Some("owner".to_owned());
        world.colonies[0]
            .known_village_ids
            .insert("personal".to_owned());
        personal.known_village_ids.insert("c1".to_owned());
        world.colonies.push(personal);
        for index in 0..MAX_OPEN_VILLAGE_TRADE_OFFERS {
            let id = format!("existing-{index}");
            world.colonies[0].village_trade_offers.insert(
                id.clone(),
                VillageTradeOffer {
                    id,
                    from_colony_id: "c1".to_owned(),
                    to_colony_id: "personal".to_owned(),
                    offered_kind: stockpiles::ResourceKind::Food,
                    offered_amount: 1.0,
                    requested_kind: stockpiles::ResourceKind::Materials,
                    requested_amount: 1.0,
                    created_at: 1_000_000,
                },
            );
        }
        let before = world.clone();

        let result = apply_action(
            &mut world,
            &proto::ClientAction::OfferVillageTrade {
                session_id: "sess_1".to_owned(),
                nickname: "Global Cat".to_owned(),
                sig: "signed".to_owned(),
                target_colony_id: "personal".to_owned(),
                offered_kind: proto::ResourceKind::Food,
                offered_amount: 1.0,
                requested_kind: proto::ResourceKind::Materials,
                requested_amount: 1.0,
            },
            &ctx(),
        );

        assert!(!result.ok);
        assert_eq!(
            result.message.as_deref(),
            Some("This village already has too many open trade offers.")
        );
        assert_eq!(world, before);
    }

    #[test]
    fn trade_acceptance_is_atomic_when_recipient_storage_is_full() {
        let mut world = world_with_one_colony();
        let mut personal = found_colony(world.world_seed, "personal", 1_000_000, 55);
        personal.kind = VillageKind::Personal;
        personal.owner_player_id = Some("owner".to_owned());
        let effects = upgrade_tree::resolve_effects(personal.upgrade_tree.owned_node_ids.iter());
        let capacity = storage::storage_capacities(
            &storage_buildings(&personal),
            effects.storage_per_level_mult,
        )
        .food;
        personal.resources.food = capacity - 1.0;
        personal.known_village_ids.insert("c1".to_owned());
        world.colonies[0]
            .known_village_ids
            .insert("personal".to_owned());
        world.colonies.push(personal);
        assert!(
            apply_action(
                &mut world,
                &proto::ClientAction::OfferVillageTrade {
                    session_id: "sess_1".to_owned(),
                    nickname: "Global Cat".to_owned(),
                    sig: "signed".to_owned(),
                    target_colony_id: "personal".to_owned(),
                    offered_kind: proto::ResourceKind::Food,
                    offered_amount: 5.0,
                    requested_kind: proto::ResourceKind::Materials,
                    requested_amount: 1.0,
                },
                &ctx(),
            )
            .ok
        );
        let offer_id = world.colonies[0]
            .village_trade_offers
            .keys()
            .next()
            .unwrap()
            .clone();
        let before = world.clone();
        let mut owner = ctx();
        owner.player_id = "owner".to_owned();
        owner.colony_id = "personal".to_owned();

        let result = apply_action(
            &mut world,
            &proto::ClientAction::AcceptVillageTrade {
                session_id: owner.session_id.clone(),
                nickname: "Owner".to_owned(),
                sig: "signed".to_owned(),
                offer_id,
            },
            &owner,
        );

        assert!(!result.ok);
        assert_eq!(world, before);
    }

    #[test]
    fn trade_rejects_exact_physical_routing_gap_even_below_aggregate_capacity() {
        let mut colony = found_colony(42, "physical-gap", 1_000_000, 55);
        for pile in &mut colony.stockpiles {
            pile.accepts.remove(&stockpiles::ResourceKind::Food);
        }
        colony.resources.food = 0.0;

        assert!(
            trade_deposit_plan(&colony, stockpiles::ResourceKind::Food, 1.0).is_none(),
            "no accepting pile means there is no exact deposit plan"
        );
        assert!(
            trade_would_overflow(&colony, stockpiles::ResourceKind::Food, 1.0),
            "aggregate capacity alone must never authorize a release-build loss"
        );
    }

    #[test]
    fn signed_capacity_research_updates_snapshot_and_trade_from_one_authority() {
        let mut world = world_with_one_colony();
        let mut personal = found_colony(world.world_seed, "personal", 1_000_000, 55);
        personal.kind = VillageKind::Personal;
        personal.owner_player_id = Some("owner".to_owned());
        personal
            .buildings
            .retain(|building| building.building_type != BuildingType::FoodStorage);
        personal
            .buildings
            .push(completed_building("granary", BuildingType::FoodStorage));
        personal
            .upgrade_tree
            .owned_node_ids
            .extend(["masonry".to_owned(), "food_storage_foundations".to_owned()]);
        personal.upgrade_tree.research_points = 100.0;
        personal.known_village_ids.insert("c1".to_owned());
        world.colonies[0]
            .known_village_ids
            .insert("personal".to_owned());
        world.colonies.push(personal);

        let before = build_snapshot(&world, 1_000_000, 1).colonies[1]
            .storage
            .capacities;
        let mut owner = ctx();
        owner.player_id = "owner".to_owned();
        owner.colony_id = "personal".to_owned();
        let purchase = proto::ClientAction::ResearchNode {
            session_id: owner.session_id.clone(),
            nickname: "Owner".to_owned(),
            sig: "server-verified".to_owned(),
            node_id: "food_storage_stores".to_owned(),
        };
        let mut twin = world.clone();
        assert!(apply_action(&mut world, &purchase, &owner).ok);
        assert!(apply_action(&mut twin, &purchase, &owner).ok);
        assert_eq!(
            world, twin,
            "signed capacity purchase must be deterministic"
        );

        let after = build_snapshot(&world, 1_000_000, 1).colonies[1]
            .storage
            .capacities;
        let masonry_mult = 1.25;
        assert_eq!(before.food, 700.0);
        assert_eq!(
            after.food,
            before.food + GRANARY_BONUS.food * masonry_mult * 0.2
        );
        assert_eq!(
            after.fish,
            before.fish + GRANARY_BONUS.food * masonry_mult * 0.2
        );
        assert_eq!(
            after.herbs,
            before.herbs + GRANARY_BONUS.herbs * masonry_mult * 0.2
        );
        assert_eq!(
            after.materials,
            before.materials + GRANARY_BONUS.materials * masonry_mult * 0.2
        );
        assert_eq!(
            after.refined,
            before.refined + GRANARY_BONUS.refined * masonry_mult * 0.2
        );
        assert_eq!(after.water, before.water);
        assert_eq!(after.weapons, before.weapons);
        assert_eq!(after.armor, before.armor);

        world.colonies[1].resources.food = before.food - 1.0;
        world.colonies[1].resources.materials = 5.0;
        let storehouse = world.colonies[1]
            .stockpiles
            .iter_mut()
            .find(|pile| pile.is_general_storehouse())
            .expect("founding storehouse");
        storehouse.contents.food = before.food - 1.0;
        let offer = proto::ClientAction::OfferVillageTrade {
            session_id: "sess_1".to_owned(),
            nickname: "Global Cat".to_owned(),
            sig: "server-verified".to_owned(),
            target_colony_id: "personal".to_owned(),
            offered_kind: proto::ResourceKind::Food,
            offered_amount: 20.0,
            requested_kind: proto::ResourceKind::Materials,
            requested_amount: 1.0,
        };
        assert!(apply_action(&mut world, &offer, &ctx()).ok);
        let offer_id = world.colonies[0]
            .village_trade_offers
            .keys()
            .next()
            .expect("offer persisted")
            .clone();

        let mut legacy_capacity = world.clone();
        legacy_capacity.colonies[1]
            .upgrade_tree
            .owned_node_ids
            .retain(|id| id != "food_storage_stores");
        let rejected = apply_action(
            &mut legacy_capacity,
            &proto::ClientAction::AcceptVillageTrade {
                session_id: owner.session_id.clone(),
                nickname: "Owner".to_owned(),
                sig: "server-verified".to_owned(),
                offer_id: offer_id.clone(),
            },
            &owner,
        );
        assert!(!rejected.ok, "the control capacity cannot receive 20 food");

        let accepted = apply_action(
            &mut world,
            &proto::ClientAction::AcceptVillageTrade {
                session_id: owner.session_id.clone(),
                nickname: "Owner".to_owned(),
                sig: "server-verified".to_owned(),
                offer_id,
            },
            &owner,
        );
        assert!(
            accepted.ok,
            "researched local granary capacity must be honored"
        );
        assert_eq!(world.colonies[1].resources.food, before.food + 19.0);
        assert_eq!(
            world.colonies[1]
                .stockpiles
                .iter()
                .map(|pile| pile.contents.food)
                .sum::<f64>(),
            before.food + 19.0,
            "accepted goods must occupy real pile headroom"
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

    #[test]
    fn build_snapshot_exposes_authoritative_between_term_countdown() {
        let mut world = world_with_one_colony();
        let winner_id = world.colonies[0].cats[0].id.clone();
        world.colonies[0].elections.push(ElectionRuntime {
            id: "resolved-election".to_owned(),
            opened_at: 900_000,
            closes_at: 950_000,
            resolved_at: Some(951_000),
            winner_cat_id: Some(winner_id),
            kind: ElectionKind::Scheduled,
        });

        let snapshot = build_snapshot(&world, 1_000_000, 0);
        let schedule = snapshot.colonies[0]
            .election_schedule
            .as_ref()
            .expect("resolved term exposes its next boundary");
        assert_eq!(schedule.term_started_at, Some(950_000));
        assert_eq!(schedule.next_election_at, 87_350_000);
        assert_eq!(schedule.term_length_ms, 86_400_000);
        assert_eq!(schedule.remaining_ms, 86_350_000);

        let twin = build_snapshot(&world, 1_000_000, 0);
        assert_eq!(snapshot, twin, "schedule projection must be deterministic");
    }

    #[test]
    fn snapshot_exposes_communal_scale_radius_and_census_distinction() {
        let mut world = new_world(4_242);
        let global = found_global_colony(4_242, "colony-1", 1_000, 1);
        let mut personal = found_colony_at(4_242, "personal", 1_000, 2, TilePos { x: 102, y: 54 });
        personal.kind = VillageKind::Personal;
        personal.owner_player_id = Some("owner".to_owned());
        world.colonies = vec![global, personal];

        let snapshot = build_snapshot(&world, 1_000, 2);
        let global = &snapshot.colonies[0];
        let personal = &snapshot.colonies[1];
        assert_eq!(global.kind, proto::VillageKind::Global);
        assert_eq!(global.scale, proto::VillageScale::Communal);
        assert_eq!(global.housing.population, 30);
        assert_eq!(global.housing.capacity, 30);
        assert_eq!(global.village_radius, 9);
        assert_eq!(personal.kind, proto::VillageKind::Personal);
        assert_eq!(personal.scale, proto::VillageScale::Personal);
        assert_eq!(personal.housing.population, 15);
        assert_eq!(personal.housing.capacity, 15);
        assert_eq!(personal.village_radius, 4);
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
            preferred_labors: Default::default(),
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
            automated_by: None,
            production_queue: crate::world_tick::default_production_queue(BuildingType::Workshop),
            production_paused: false,
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
    fn signed_manual_assignment_can_run_a_sawmill_while_forester_office_is_vacant() {
        let mut world = world_with_one_colony();
        let colony = &mut world.colonies[0];
        assert!(!colony.officers.contains_key(&OfficerRole::Forester));
        let cat_id = colony.cats[0].id.clone();
        let building_id = "manual-sawmill".to_owned();
        colony.buildings.push(crate::world_tick::BuildingRuntime {
            id: building_id.clone(),
            building_type: BuildingType::Sawmill,
            level: 1,
            position: TilePos { x: 18, y: 18 },
            is_complete: true,
            construction_progress: 100,
            ..crate::world_tick::BuildingRuntime::default()
        });

        let result = apply_action(
            &mut world,
            &proto::ClientAction::AssignWorker {
                session_id: "sess_1".to_owned(),
                nickname: "Guest".to_owned(),
                sig: "signed".to_owned(),
                cat_id: cat_id.clone(),
                building_id: Some(building_id.clone()),
            },
            &ctx(),
        );

        assert!(result.ok, "manual staffing must not require a Forester");
        let building = world.colonies[0]
            .buildings
            .iter()
            .find(|building| building.id == building_id)
            .unwrap();
        assert_eq!(building.assigned_cat.as_deref(), Some(cat_id.as_str()));
        assert_eq!(building.automated_by, None);
    }

    #[test]
    fn signed_unassign_restores_farm_basket_and_frees_every_transient_field() {
        let mut world = world_with_one_colony();
        let colony = &mut world.colonies[0];
        let cat_id = colony.cats[0].id.clone();
        colony.buildings.push(crate::world_tick::BuildingRuntime {
            id: "manual-field".to_owned(),
            building_type: BuildingType::Field,
            is_complete: true,
            construction_progress: 100,
            assigned_cat: Some(cat_id.clone()),
            ..crate::world_tick::BuildingRuntime::default()
        });
        colony.farms.push(FarmPlot {
            id: "manual-farm".to_owned(),
            rect: zones::ZoneRect {
                x1: 12,
                y1: 12,
                x2: 12,
                y2: 12,
            },
            crop: farming::CropKind::Grain,
            planted_at: 1,
            stage: FarmStage::Growing,
            growth_hours: 7.0,
            fertility: 1.0,
            worker_id: Some(cat_id.clone()),
            work_phase: farming::FarmWorkPhase::Hauling,
            pending_output: 3.0,
        });
        let cat = &mut colony.cats[0];
        cat.current_task = Some(TaskType::Farm);
        cat.activity = CatActivity::Returning;
        cat.destination = Some(Position {
            map: MapType::World,
            x: 10.0,
            y: 10.0,
        });
        cat.carrying = Some(entities::Carrying {
            kind: entities::CarryingKind::Grain,
            amount: 2.0,
            job_ended_at: 1,
            source_gather_spot: Some("farm-out|manual-farm|farm-gather:manual-farm".to_owned()),
        });

        let result = apply_action(
            &mut world,
            &proto::ClientAction::AssignWorker {
                session_id: "sess_1".to_owned(),
                nickname: "Guest".to_owned(),
                sig: "signed".to_owned(),
                cat_id: cat_id.clone(),
                building_id: None,
            },
            &ctx(),
        );

        assert!(result.ok, "{result:?}");
        let colony = &world.colonies[0];
        let cat = colony.cats.iter().find(|cat| cat.id == cat_id).unwrap();
        assert_eq!(cat.current_task, None);
        assert_eq!(cat.activity, CatActivity::Idle);
        assert_eq!(cat.destination, None);
        assert_eq!(cat.carrying, None);
        assert_eq!(colony.farms[0].pending_output, 5.0);
        assert_eq!(colony.farms[0].worker_id, None);
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
    fn snapshot_exposes_visible_probationers_beds_unhoused_and_departures() {
        let mut world = world_with_one_colony();
        let colony = &mut world.colonies[0];
        colony.run_started_at = 0;
        let mut migrant = colony.cats[0].clone();
        migrant.id = "migrant-snapshot".to_owned();
        migrant.name = "Wayfarer Snapshot".to_owned();
        colony.cats.push(migrant);
        colony
            .migration_state
            .probationary_migrants
            .push(crate::migration::ProbationaryMigrant {
                id: "migrant-snapshot".to_owned(),
                arrived_game_minute: 1_800,
                housing_deadline_game_minute: 3_960,
            });
        colony.migration_departures = 2;

        let snapshot = build_snapshot(&world, 1_800 * 60_000, 1);
        let colony = &snapshot.colonies[0];

        assert_eq!(colony.housing.population, 16);
        assert_eq!(colony.housing.capacity, 15);
        assert_eq!(colony.housing.housed, 15);
        assert_eq!(colony.housing.probationary, 1);
        assert_eq!(colony.housing.unhoused, 1);
        assert_eq!(colony.housing.departures, 2);
        assert_eq!(
            colony
                .cats
                .iter()
                .find(|cat| cat.id == "migrant-snapshot")
                .unwrap()
                .housing_status,
            proto::CatHousingStatus::Probationary
        );
        assert_eq!(
            colony
                .cats
                .iter()
                .find(|cat| cat.id == "migrant-snapshot")
                .unwrap()
                .probation_remaining_game_minutes,
            Some(2_160)
        );

        let deadline = build_snapshot(&world, 3_960 * 60_000, 1);
        assert_eq!(
            deadline.colonies[0]
                .cats
                .iter()
                .find(|cat| cat.id == "migrant-snapshot")
                .unwrap()
                .probation_remaining_game_minutes,
            Some(0)
        );
    }

    #[test]
    fn populated_legacy_zero_den_snapshot_crosses_json_without_nonfinite_pressure() {
        let mut world = world_with_one_colony();
        world.colonies[0]
            .buildings
            .retain(|building| building.building_type != BuildingType::Den);

        let snapshot = build_snapshot(&world, 1_000_000, 1);
        let housing = snapshot.colonies[0].housing;
        assert_eq!(housing.population, 15);
        assert_eq!(housing.capacity, 0);
        assert_eq!(housing.pressure, 15.0);
        assert!(housing.pressure.is_finite());

        let websocket_text = serde_json::to_string(&snapshot).expect("serialize snapshot");
        let decoded: proto::WorldSnapshot =
            serde_json::from_str(&websocket_text).expect("deserialize snapshot");
        assert_eq!(decoded, snapshot);
    }

    #[test]
    fn large_census_housing_allocation_is_single_pass_shaped_and_order_stable() {
        let mut world = world_with_one_colony();
        let colony = &mut world.colonies[0];
        for index in 0..485 {
            let mut cat = colony.cats[0].clone();
            cat.id = format!("bulk-{index:04}");
            cat.name = format!("Bulk {index}");
            colony.cats.push(cat);
            if index < 100 {
                colony.migration_state.probationary_migrants.push(
                    crate::migration::ProbationaryMigrant {
                        id: format!("bulk-{index:04}"),
                        arrived_game_minute: 1_800,
                        housing_deadline_game_minute: 3_960,
                    },
                );
            }
        }
        let mut reversed = world.clone();
        reversed.colonies[0].cats.reverse();

        let summarize = |world: &WorldState| {
            let snapshot = build_snapshot(world, 1_800 * 60_000, 1);
            let colony = &snapshot.colonies[0];
            assert_eq!(colony.housing.population, 500);
            assert_eq!(colony.housing.housed, 15);
            assert_eq!(colony.housing.probationary, 100);
            assert_eq!(colony.housing.unhoused, 485);
            colony
                .cats
                .iter()
                .map(|cat| (cat.id.clone(), cat.housing_status))
                .collect::<BTreeMap<_, _>>()
        };

        assert_eq!(summarize(&world), summarize(&reversed));
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

    fn grant_officer_prerequisite(colony: &mut ColonyRuntime, role: OfficerRole) {
        let prerequisite = prerequisite_for(role);
        if !upgrade_tree::is_owned(&colony.upgrade_tree, prerequisite.upgrade_node) {
            colony
                .upgrade_tree
                .owned_node_ids
                .push(prerequisite.upgrade_node.to_owned());
        }
        if !has_complete_building(colony, prerequisite.building) {
            colony.buildings.push(crate::world_tick::BuildingRuntime {
                id: format!("test-role-{}", prerequisite.building.as_str()),
                building_type: prerequisite.building,
                level: 1,
                position: TilePos { x: 4, y: 4 },
                is_complete: true,
                construction_progress: 100,
                production_progress: 0.0,
                assigned_cat: None,
                automated_by: None,
                production_queue: crate::world_tick::default_production_queue(
                    prerequisite.building,
                ),
                production_paused: false,
            });
        }
    }

    #[test]
    fn assign_officer_appoints_and_enforces_one_office_per_cat() {
        let mut world = world_with_one_colony();
        let cat_id = world.colonies[0].cats[0].id.clone();
        grant_officer_prerequisite(&mut world.colonies[0], OfficerRole::Farmer);
        grant_officer_prerequisite(&mut world.colonies[0], OfficerRole::Captain);

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
    fn signed_officer_appointment_rejects_a_living_kitten() {
        let mut world = world_with_one_colony();
        grant_officer_prerequisite(&mut world.colonies[0], OfficerRole::Steward);
        world.colonies[0].cats[0].age_hours = 1.0;
        let kitten_id = world.colonies[0].cats[0].id.clone();

        let result = apply_action(
            &mut world,
            &assign_officer_action(proto::OfficerRole::Steward, &kitten_id),
            &ctx(),
        );

        assert!(!result.ok, "a kitten cannot hold an automation office");
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

    fn open_stockpile_points(
        world: &WorldState,
        width: i32,
        height: i32,
    ) -> (proto::TilePoint, proto::TilePoint) {
        let colony = &world.colonies[0];
        let mut anchors = colony.claimed_tiles.clone();
        anchors.sort_by_key(|tile| (tile.y, tile.x));
        let anchor = anchors
            .into_iter()
            .find(|anchor| {
                crate::world_tick::stockpile_placement_error(
                    colony,
                    zones::ZoneRect {
                        x1: anchor.x,
                        y1: anchor.y,
                        x2: anchor.x + width - 1,
                        y2: anchor.y + height - 1,
                    },
                    world.world_seed,
                    true,
                )
                .is_none()
            })
            .expect("founding claim has a valid stockpile rectangle");
        (
            tp(anchor.x, anchor.y),
            tp(anchor.x + width - 1, anchor.y + height - 1),
        )
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
        let (a, b) = open_stockpile_points(&world, 2, 2);
        let res = apply_action(
            &mut world,
            &designate_action(a, b, vec![proto::ResourceKind::Food]),
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
    fn designate_stockpile_accepts_all_physical_kinds_and_rejects_only_blessings() {
        const NONPHYSICAL_MESSAGE: &str = "Stockpiles accept only physical goods; Blessings are divine favor and are never hauled or stored in piles.";

        assert_eq!(proto::ResourceKind::ALL.len(), 23);
        for &kind in proto::ResourceKind::ALL {
            let mut world = world_with_one_colony();
            let before = world.colonies[0].stockpiles.len();
            let (a, b) = open_stockpile_points(&world, 1, 1);
            let result = apply_action(&mut world, &designate_action(a, b, vec![kind]), &ctx());

            assert_eq!(
                result.ok,
                kind.is_physical_stockpile_good(),
                "unexpected validation result for {kind:?}: {result:?}"
            );
            if kind.is_physical_stockpile_good() {
                assert_eq!(world.colonies[0].stockpiles.len(), before + 1);
                let pile = world.colonies[0]
                    .stockpiles
                    .last()
                    .expect("accepted designation adds a pile");
                assert_eq!(
                    pile.accepts.iter().copied().collect::<Vec<_>>(),
                    vec![proto_to_sim_resource_kind(kind)]
                );
            } else {
                assert_eq!(kind, proto::ResourceKind::Blessings);
                assert_eq!(result.message.as_deref(), Some(NONPHYSICAL_MESSAGE));
                assert_eq!(world.colonies[0].stockpiles.len(), before);
            }
        }

        let mut world = world_with_one_colony();
        let before = world.colonies[0].stockpiles.len();
        let (a, b) = open_stockpile_points(&world, 1, 1);
        let mixed = apply_action(
            &mut world,
            &designate_action(
                a,
                b,
                vec![proto::ResourceKind::Food, proto::ResourceKind::Blessings],
            ),
            &ctx(),
        );
        assert!(
            !mixed.ok,
            "one nonphysical kind rejects the whole accept set"
        );
        assert_eq!(mixed.message.as_deref(), Some(NONPHYSICAL_MESSAGE));
        assert_eq!(
            world.colonies[0].stockpiles.len(),
            before,
            "mixed invalid designation is atomic"
        );
    }

    #[test]
    fn designate_stockpile_rejects_spatial_collisions_atomically() {
        let mut world = world_with_one_colony();

        let cases = [
            (
                tp(30, 30),
                "claimed village land",
                "wild ground is outside the claim",
            ),
            (
                tp(6, 6),
                "building footprint",
                "the shrine occupies this tile",
            ),
            (
                tp(7, 1),
                "paved road",
                "the founding road occupies this tile",
            ),
        ];
        for (point, expected, reason) in cases {
            let before = world.colonies[0].clone();
            let result = apply_action(
                &mut world,
                &designate_action(point, point, vec![proto::ResourceKind::Food]),
                &ctx(),
            );
            assert!(!result.ok, "{reason}: {result:?}");
            assert!(
                result
                    .message
                    .as_deref()
                    .is_some_and(|message| message.contains(expected)),
                "{reason}: {result:?}"
            );
            assert_eq!(world.colonies[0], before, "{reason} mutated the colony");
        }

        let (a, b) = open_stockpile_points(&world, 1, 1);
        let placed = apply_action(
            &mut world,
            &designate_action(a, b, vec![proto::ResourceKind::Food]),
            &ctx(),
        );
        assert!(placed.ok, "{placed:?}");
        let before_overlap = world.colonies[0].clone();
        let overlap = apply_action(
            &mut world,
            &designate_action(a, b, vec![proto::ResourceKind::Water]),
            &ctx(),
        );
        assert!(!overlap.ok, "overlapping pile accepted: {overlap:?}");
        assert!(
            overlap
                .message
                .as_deref()
                .is_some_and(|message| message.contains("another stockpile"))
        );
        assert_eq!(world.colonies[0], before_overlap);

        let (water_a, _) = open_stockpile_points(&world, 1, 1);
        let water = TilePos {
            x: water_a.x,
            y: water_a.y,
        };
        let water_tile = world.colonies[0]
            .world_tiles
            .get_mut(&water)
            .expect("open stockpile point is mapped");
        water_tile.tile_type = crate::types::TileType::River;
        water_tile.resources.water = 999;
        let before_water = world.colonies[0].clone();
        let on_water = apply_action(
            &mut world,
            &designate_action(
                tp(water.x, water.y),
                tp(water.x, water.y),
                vec![proto::ResourceKind::Food],
            ),
            &ctx(),
        );
        assert!(!on_water.ok, "water pile accepted: {on_water:?}");
        assert!(
            on_water
                .message
                .as_deref()
                .is_some_and(|message| message.contains("water"))
        );
        assert_eq!(world.colonies[0], before_water);
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

        let (a, b) = open_stockpile_points(&world, 1, 1);
        let _ = apply_action(
            &mut world,
            &designate_action(a, b, vec![proto::ResourceKind::Food]),
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
        let recreated = apply_action(
            &mut world,
            &designate_action(a, b, vec![proto::ResourceKind::Water]),
            &ctx(),
        );
        assert!(
            recreated.ok,
            "removed stockpile tiles can be designated again: {recreated:?}"
        );

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

    #[test]
    fn remove_stockpile_cannot_bypass_typed_or_station_local_cleanup() {
        let mut world = world_with_one_colony();
        let (bank, _) = prepare_fishing_shore(&mut world);
        assert!(
            apply_action(
                &mut world,
                &proto::ClientAction::DesignateFishingSpot {
                    session_id: "sess_1".to_owned(),
                    nickname: "Angler".to_owned(),
                    sig: "signed".to_owned(),
                    at: proto::TilePoint {
                        x: bank.x,
                        y: bank.y,
                    },
                },
                &ctx(),
            )
            .ok
        );
        let fishing_id = world.colonies[0].gather_spots[0].stockpile_id.clone();
        let before = world.colonies[0].clone();
        let typed = apply_action(
            &mut world,
            &proto::ClientAction::RemoveStockpile {
                session_id: "sess_1".to_owned(),
                nickname: "Angler".to_owned(),
                sig: "signed".to_owned(),
                stockpile_id: fishing_id,
            },
            &ctx(),
        );
        assert!(!typed.ok);
        assert_eq!(world.colonies[0], before);

        let station_id = stockpiles::station_input_id("fixture-sawmill");
        world.colonies[0].stockpiles.push(stockpiles::Stockpile {
            id: station_id.clone(),
            rect: zones::normalize_rect(30.0, 30.0, 30.0, 30.0),
            accepts: std::iter::once(stockpiles::ResourceKind::Logs).collect(),
            contents: entities::Resources::default(),
        });
        let station = apply_action(
            &mut world,
            &proto::ClientAction::RemoveStockpile {
                session_id: "sess_1".to_owned(),
                nickname: "Angler".to_owned(),
                sig: "signed".to_owned(),
                stockpile_id: station_id.clone(),
            },
            &ctx(),
        );
        assert!(!station.ok);
        assert!(
            world.colonies[0]
                .stockpiles
                .iter()
                .any(|pile| pile.id == station_id)
        );
    }

    #[test]
    fn remove_stockpile_truthfully_handles_active_and_dormant_steward_piles() {
        let mut world = world_with_one_colony();
        let (a, b) = open_stockpile_points(&world, 1, 1);
        assert!(
            apply_action(
                &mut world,
                &designate_action(a, b, vec![proto::ResourceKind::Grain]),
                &ctx(),
            )
            .ok
        );
        let pile_id = world.colonies[0]
            .stockpiles
            .iter()
            .find(|pile| !pile.is_general_storehouse())
            .expect("designated pile")
            .id
            .clone();
        world.colonies[0].stock_ledger.steward_managed_piles.insert(
            pile_id.clone(),
            crate::ledger::StewardManagedPile {
                station_id: "mill-a".to_owned(),
                resource: stockpiles::ResourceKind::Grain,
                active: true,
            },
        );
        let remove = || proto::ClientAction::RemoveStockpile {
            session_id: "sess_1".to_owned(),
            nickname: "Guest".to_owned(),
            sig: "signed".to_owned(),
            stockpile_id: pile_id.clone(),
        };
        let active = apply_action(&mut world, &remove(), &ctx());
        assert!(!active.ok);
        assert!(
            active
                .message
                .as_deref()
                .unwrap()
                .contains("actively managed")
        );

        world.colonies[0]
            .stock_ledger
            .steward_managed_piles
            .get_mut(&pile_id)
            .unwrap()
            .active = false;
        world.colonies[0]
            .stockpiles
            .iter_mut()
            .find(|pile| pile.id == pile_id)
            .unwrap()
            .contents
            .grain = 1.0;
        let occupied = apply_action(&mut world, &remove(), &ctx());
        assert!(!occupied.ok);
        assert!(
            occupied
                .message
                .as_deref()
                .unwrap()
                .contains("still contains goods")
        );

        world.colonies[0]
            .stockpiles
            .iter_mut()
            .find(|pile| pile.id == pile_id)
            .unwrap()
            .contents
            .grain = 0.0;
        let empty = apply_action(&mut world, &remove(), &ctx());
        assert!(
            empty.ok,
            "empty dormant piles are player-removable: {empty:?}"
        );
        assert!(
            !world.colonies[0]
                .stock_ledger
                .steward_managed_piles
                .contains_key(&pile_id)
        );
        assert!(
            !world.colonies[0]
                .stockpiles
                .iter()
                .any(|pile| pile.id == pile_id)
        );
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

    fn open_gather_point(world: &WorldState) -> proto::TilePoint {
        let colony = &world.colonies[0];
        colony
            .world_tiles
            .keys()
            .copied()
            .find(|tile| {
                colony.revealed_tiles.contains(tile)
                    && crate::world_tick::stockpile_placement_error(
                        colony,
                        zones::ZoneRect {
                            x1: tile.x,
                            y1: tile.y,
                            x2: tile.x,
                            y2: tile.y,
                        },
                        world.world_seed,
                        false,
                    )
                    .is_none()
            })
            .map(|tile| tp(tile.x, tile.y))
            .expect("generated wilds have an open gather-spot tile")
    }

    #[test]
    fn designate_gather_spot_adds_a_pile_and_bookkeeping_record() {
        let mut world = world_with_one_colony();
        let point = open_gather_point(&world);
        let res = apply_action(
            &mut world,
            &designate_gather_action(point, point, proto::ResourceKind::Food),
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
    fn manual_gather_spots_accept_every_physically_carried_crop() {
        let mut world = world_with_one_colony();
        for (wire, sim) in [
            (proto::ResourceKind::Grain, stockpiles::ResourceKind::Grain),
            (
                proto::ResourceKind::Catnip,
                stockpiles::ResourceKind::Catnip,
            ),
            (proto::ResourceKind::Herbs, stockpiles::ResourceKind::Herbs),
        ] {
            let point = open_gather_point(&world);
            let result = apply_action(
                &mut world,
                &designate_gather_action(point, point, wire),
                &ctx(),
            );
            assert!(result.ok, "{wire:?}: {result:?}");
            assert_eq!(world.colonies[0].gather_spots.last().unwrap().kind, sim);
        }
    }

    #[test]
    fn designate_gather_spot_rejects_unsupported_resources_and_oversized_rects() {
        let mut world = world_with_one_colony();

        let unsupported = apply_action(
            &mut world,
            &designate_gather_action(tp(30, 30), tp(30, 30), proto::ResourceKind::Blessings),
            &ctx(),
        );
        assert!(
            !unsupported.ok,
            "only physically carried resources are collectable"
        );

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
    fn authored_spatial_actions_reject_extreme_or_unrevealed_coordinates_atomically() {
        let mut world = world_with_one_colony();
        for point in [tp(i32::MIN, i32::MIN), tp(i32::MAX, i32::MAX), tp(30, 30)] {
            let before = world.colonies[0].clone();
            let result = apply_action(
                &mut world,
                &designate_gather_action(point, point, proto::ResourceKind::Food),
                &ctx(),
            );
            assert!(!result.ok, "unsafe/unrevealed gather coordinate accepted");
            assert_eq!(
                world.colonies[0], before,
                "rejected gather spot mutated state"
            );
        }

        let before = world.colonies[0].clone();
        let road = proto::ClientAction::BuildRoad {
            session_id: "sess_1".to_owned(),
            nickname: "Guest".to_owned(),
            sig: "sig".to_owned(),
            a: tp(i32::MIN, 0),
            b: tp(i32::MAX, 0),
        };
        let result = apply_action(&mut world, &road, &ctx());
        assert!(!result.ok, "extreme road endpoints accepted");
        assert_eq!(world.colonies[0], before, "rejected road mutated state");
    }

    #[test]
    fn designate_gather_spot_enforces_its_own_budget() {
        let mut world = world_with_one_colony();
        for i in 0..stockpiles::MAX_GATHER_SPOTS {
            let point = open_gather_point(&world);
            let res = apply_action(
                &mut world,
                &designate_gather_action(point, point, proto::ResourceKind::Food),
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
        let point = open_gather_point(&world);
        let _ = apply_action(
            &mut world,
            &designate_gather_action(point, point, proto::ResourceKind::Food),
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
                    site: Some(TilePos {
                        x: point.x,
                        y: point.y,
                    }),
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

        let recreated = apply_action(
            &mut world,
            &designate_gather_action(point, point, proto::ResourceKind::Food),
            &ctx(),
        );
        assert!(
            recreated.ok,
            "removed gather-spot tiles can be designated again: {recreated:?}"
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
        let (a, b) = open_stockpile_points(&world, 2, 2);
        let _ = apply_action(
            &mut world,
            &designate_action(a, b, vec![proto::ResourceKind::Food]),
            &ctx(),
        );
        let snap = build_snapshot(&world, 1_000_000, 1);
        assert!(
            snap.colonies[0]
                .stockpiles
                .iter()
                .any(|pile| pile.id == stockpiles::GENERAL_STOREHOUSE_ID),
            "finite seeded storehouse exposed"
        );
        assert!(
            snap.colonies[0].stockpiles.len() >= 2,
            "designated pile exposed"
        );
    }

    #[test]
    fn build_snapshot_flags_gather_spots_on_their_stockpile_snapshot() {
        let mut world = world_with_one_colony();
        let point = open_gather_point(&world);
        let _ = apply_action(
            &mut world,
            &designate_gather_action(point, point, proto::ResourceKind::Water),
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

        // The seeded storehouse and a general stockpile are never flagged as gather spots.
        let shrine = snap.colonies[0]
            .stockpiles
            .iter()
            .find(|pile| pile.id == stockpiles::GENERAL_STOREHOUSE_ID)
            .expect("storehouse exposed");
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
    fn build_snapshot_distinguishes_traffic_dirt_from_authored_stone_roads() {
        let mut world = world_with_one_colony();
        let dirt = world.colonies[0]
            .world_tiles
            .values()
            .find(|tile| {
                tile.overlay_feature.is_none()
                    && !world.colonies[0].claimed_tiles.contains(&tile.pos)
                    && !matches!(
                        tile.tile_type,
                        crate::types::TileType::Mountains | crate::types::TileType::CaveEntrance
                    )
            })
            .expect("fixture has ordinary exterior ground")
            .pos;
        world.colonies[0]
            .world_tiles
            .get_mut(&dirt)
            .expect("dirt tile")
            .path_wear = crate::movement::WORN_ROAD_WEAR;

        let stone_ground = world.colonies[0]
            .world_tiles
            .values()
            .find(|tile| tile.pos != dirt && tile.overlay_feature.is_none())
            .expect("fixture has a second exterior tile")
            .pos;
        let stone = world.colonies[0]
            .world_tiles
            .get_mut(&stone_ground)
            .expect("stone ground tile");
        stone.tile_type = crate::types::TileType::Mountains;
        stone.path_wear = crate::movement::WORN_ROAD_WEAR;

        let snap = build_snapshot(&world, 1_000_000, 1);
        assert!(
            snap.colonies[0]
                .dirt_road_tiles
                .contains(&tile_point(&dirt))
        );
        assert!(
            !snap.colonies[0]
                .dirt_road_tiles
                .contains(&tile_point(&stone_ground)),
            "stone ground must not surface as a dirt road"
        );
        assert!(
            snap.colonies[0]
                .road_tiles
                .iter()
                .all(|road| !snap.colonies[0].dirt_road_tiles.contains(road)),
            "authored stone and traffic dirt surfaces are disjoint"
        );
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

    fn seed_visible_resource(
        colony: &mut ColonyRuntime,
        kind: stockpiles::ResourceKind,
        amount: f64,
    ) {
        stockpiles::add_resource(&mut colony.resources, kind, amount);
        let pile = colony
            .stockpiles
            .iter_mut()
            .find(|pile| !pile.is_station_local())
            .expect("founding colony has visible storage");
        stockpiles::add_resource(&mut pile.contents, kind, amount);
    }

    #[test]
    fn damaged_goods_stay_visible_but_traders_accept_only_pristine_load_bounded_units() {
        let mut world = world_with_one_colony();
        let mug = Item::new(ItemKind::Mug, Material::Wood, 1);
        world.colonies[0].add_item(mug, 2);
        world.colonies[0].items.wear(ItemKind::Mug, 1);
        world.colonies[0].trader = Some(trading_trader());

        let snapshot = build_snapshot(&world, 1_000_000, 1);
        let offer = &snapshot.colonies[0].trader.as_ref().unwrap().buy_offers[0];
        assert_eq!(
            offer.available, 1,
            "damaged item remains in Goods, not the offer"
        );
        assert_eq!(snapshot.colonies[0].items[0].count, 2);
        assert!(!apply_action(&mut world, &sell_goods_action("mug", "wood", 1, 2), &ctx()).ok);
        assert!(apply_action(&mut world, &sell_goods_action("mug", "wood", 1, 1), &ctx()).ok);
        assert_eq!(world.colonies[0].items.get(&mug), Some(&1));
        assert_eq!(world.colonies[0].items.pristine_count(mug), 0);

        let furniture = Item::new(ItemKind::Furniture, Material::Stone, 0);
        world.colonies[0].add_item(furniture, 2);
        let snapshot = build_snapshot(&world, 1_000_000, 1);
        let heavy_offer = snapshot.colonies[0]
            .trader
            .as_ref()
            .unwrap()
            .buy_offers
            .iter()
            .find(|offer| offer.kind == "furniture")
            .unwrap();
        assert_eq!(
            heavy_offer.available, 1,
            "20kg caravan load caps heavy units"
        );
        assert!(
            !apply_action(
                &mut world,
                &sell_goods_action("furniture", "stone", 0, 2),
                &ctx(),
            )
            .ok
        );
    }

    #[test]
    fn signed_repair_requires_a_living_worker_and_spends_one_visible_material() {
        let mut world = world_with_one_colony();
        let colony = &mut world.colonies[0];
        let worker_id = colony.cats[0].id.clone();
        colony.buildings.push(BuildingRuntime {
            id: "repair-bench".to_owned(),
            building_type: BuildingType::Woodworking,
            is_complete: true,
            construction_progress: 100,
            assigned_cat: Some(worker_id.clone()),
            ..BuildingRuntime::default()
        });
        let tool = Item::new(ItemKind::Tool, Material::Wood, 1);
        colony.add_crafted_item(tool, 1);
        let item_id = colony.items.instances().next().unwrap().id.clone();
        colony.items.wear(ItemKind::Tool, 1);
        let damaged = colony.items.instance(&item_id).unwrap().durability;
        let planks_before = colony.resources.planks;
        seed_visible_resource(colony, stockpiles::ResourceKind::Planks, 1.0);
        colony.cats[0].death_time = Some(ctx().now_ms - 1);

        let action = proto::ClientAction::RepairItem {
            session_id: "sess_1".to_owned(),
            nickname: "Guest".to_owned(),
            sig: "signed".to_owned(),
            item_id: item_id.clone(),
        };
        let denied = apply_action(&mut world, &action, &ctx());
        assert!(!denied.ok);
        assert_eq!(
            world.colonies[0]
                .items
                .instance(&item_id)
                .unwrap()
                .durability,
            damaged
        );
        assert_eq!(world.colonies[0].resources.planks, planks_before + 1.0);

        world.colonies[0].cats[0].death_time = None;
        let repaired = apply_action(&mut world, &action, &ctx());
        assert!(repaired.ok, "{:?}", repaired.message);
        assert!(
            world.colonies[0]
                .items
                .instance(&item_id)
                .unwrap()
                .is_pristine()
        );
        assert_eq!(world.colonies[0].resources.planks, planks_before);
    }

    #[test]
    fn selling_identified_equipment_removes_its_physical_stack_and_identity_together() {
        let mut world = world_with_one_colony();
        let colony = &mut world.colonies[0];
        let tool = Item::new(ItemKind::Tool, Material::Wood, 1);
        colony.add_crafted_item(tool, 1);
        let tools_before = colony.resources.tools;
        let visible_tools_before = visible_resource_amount(colony, stockpiles::ResourceKind::Tools);
        seed_visible_resource(colony, stockpiles::ResourceKind::Tools, 1.0);
        colony.trader = Some(trading_trader());

        let result = apply_action(&mut world, &sell_goods_action("tool", "wood", 1, 1), &ctx());
        assert!(result.ok, "{:?}", result.message);
        assert!(world.colonies[0].items.get(&tool).is_none());
        assert_eq!(world.colonies[0].resources.tools, tools_before);
        assert_eq!(
            visible_resource_amount(&world.colonies[0], stockpiles::ResourceKind::Tools),
            visible_tools_before
        );
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

    #[test]
    fn guided_labor_preference_selects_the_exact_cat_without_bypassing_liveness() {
        let mut world = world_with_one_colony();
        let cat_id = world.colonies[0].cats[0].id.clone();
        let action = proto::ClientAction::SetCatLaborPreference {
            session_id: "sess_1".to_owned(),
            nickname: "Player".to_owned(),
            sig: "signed".to_owned(),
            cat_id: cat_id.clone(),
            labor: proto::Labor::Scout,
            enabled: true,
        };
        assert!(apply_action(&mut world, &action, &ctx()).ok);
        assert!(
            world.colonies[0].cats[0]
                .preferred_labors
                .contains(&Labor::Scout)
        );

        let explore = proto::ClientAction::RequestJob {
            session_id: "sess_1".to_owned(),
            nickname: "Player".to_owned(),
            sig: "signed".to_owned(),
            kind: proto::JobKind::Explore,
        };
        assert!(apply_action(&mut world, &explore, &ctx()).ok);
        assert_eq!(
            world.colonies[0]
                .jobs
                .last()
                .unwrap()
                .assigned_cat
                .as_deref(),
            Some(cat_id.as_str())
        );

        world.colonies[0].cats[0].death_time = Some(ctx().now_ms);
        let clear = proto::ClientAction::SetCatLaborPreference {
            session_id: "sess_1".to_owned(),
            nickname: "Player".to_owned(),
            sig: "signed".to_owned(),
            cat_id,
            labor: proto::Labor::Scout,
            enabled: false,
        };
        assert!(!apply_action(&mut world, &clear, &ctx()).ok);
    }

    #[test]
    fn guided_station_queue_edits_are_ordered_recipe_scoped_and_exact() {
        let mut world = world_with_one_colony();
        world.colonies[0].recipe_entitlement_rules_version = 0;
        let colony = &mut world.colonies[0];
        colony.buildings.push(BuildingRuntime {
            id: "sawmill-player".to_owned(),
            building_type: BuildingType::Sawmill,
            is_complete: true,
            construction_progress: 100,
            production_queue: Vec::new(),
            ..BuildingRuntime::default()
        });
        let edit = |edit| proto::ClientAction::EditProductionQueue {
            session_id: "sess_1".to_owned(),
            nickname: "Player".to_owned(),
            sig: "signed".to_owned(),
            building_id: "sawmill-player".to_owned(),
            edit,
        };
        for repeat in [false, true] {
            assert!(
                apply_action(
                    &mut world,
                    &edit(proto::ProductionQueueEdit::Add {
                        recipe_id: crate::world_tick::SAWMILL_RECIPE_ID.to_owned(),
                        repeat,
                    }),
                    &ctx(),
                )
                .ok
            );
        }
        assert!(
            apply_action(
                &mut world,
                &edit(proto::ProductionQueueEdit::Move {
                    index: 1,
                    direction: proto::QueueMoveDirection::Up,
                }),
                &ctx(),
            )
            .ok
        );
        let queue = &world.colonies[0].buildings.last().unwrap().production_queue;
        assert_eq!(
            queue.iter().map(|entry| entry.repeat).collect::<Vec<_>>(),
            vec![true, false]
        );
        assert!(
            apply_action(
                &mut world,
                &edit(proto::ProductionQueueEdit::SetPaused { paused: true }),
                &ctx(),
            )
            .ok
        );
        assert!(
            world.colonies[0]
                .buildings
                .last()
                .unwrap()
                .production_paused
        );

        assert!(
            !apply_action(
                &mut world,
                &edit(proto::ProductionQueueEdit::Add {
                    recipe_id: "imaginary_recipe".to_owned(),
                    repeat: false,
                }),
                &ctx(),
            )
            .ok
        );

        world.colonies[0].buildings.push(BuildingRuntime {
            id: "mill-player".to_owned(),
            building_type: BuildingType::Mill,
            is_complete: true,
            construction_progress: 100,
            production_queue: Vec::new(),
            ..BuildingRuntime::default()
        });
        let queue_add =
            |building_id: &str, recipe_id: &str| proto::ClientAction::EditProductionQueue {
                session_id: "sess_1".to_owned(),
                nickname: "Player".to_owned(),
                sig: "signed".to_owned(),
                building_id: building_id.to_owned(),
                edit: proto::ProductionQueueEdit::Add {
                    recipe_id: recipe_id.to_owned(),
                    repeat: true,
                },
            };
        assert!(
            apply_action(
                &mut world,
                &queue_add("mill-player", crate::world_tick::MILL_RECIPE_ID),
                &ctx(),
            )
            .ok
        );
        assert!(
            !apply_action(
                &mut world,
                &queue_add("mill-player", crate::world_tick::SAWMILL_RECIPE_ID),
                &ctx(),
            )
            .ok,
            "Sawmill recipe cannot cross into a Mill"
        );
        assert!(
            !apply_action(
                &mut world,
                &queue_add("sawmill-player", crate::world_tick::MILL_RECIPE_ID),
                &ctx(),
            )
            .ok,
            "Mill recipe cannot cross into a Sawmill"
        );
        let snapshot = build_snapshot(&world, ctx().now_ms, 1);
        let buildings = &snapshot.colonies[0].buildings;
        assert_eq!(
            buildings
                .iter()
                .find(|building| building.id == "mill-player")
                .unwrap()
                .available_recipes,
            vec![crate::world_tick::MILL_RECIPE_ID.to_owned()]
        );
        assert_eq!(
            buildings
                .iter()
                .find(|building| building.id == "sawmill-player")
                .unwrap()
                .available_recipes,
            vec![crate::world_tick::SAWMILL_RECIPE_ID.to_owned()]
        );

        for (id, building_type, recipe_id) in [
            (
                "workshop-player",
                BuildingType::Workshop,
                crate::world_tick::WORKSHOP_RECIPE_ID,
            ),
            (
                "smelter-player",
                BuildingType::Smelter,
                crate::world_tick::SMELTER_RECIPE_ID,
            ),
        ] {
            world.colonies[0].buildings.push(BuildingRuntime {
                id: id.to_owned(),
                building_type,
                is_complete: true,
                construction_progress: 100,
                production_queue: Vec::new(),
                ..BuildingRuntime::default()
            });
            assert!(apply_action(&mut world, &queue_add(id, recipe_id), &ctx()).ok);
            assert_eq!(
                world.colonies[0]
                    .buildings
                    .iter()
                    .find(|building| building.id == id)
                    .unwrap()
                    .production_queue,
                vec![crate::world_tick::ProductionQueueEntry {
                    recipe_id: recipe_id.to_owned(),
                    repeat: true,
                }]
            );
        }
        assert!(
            !apply_action(
                &mut world,
                &queue_add("workshop-player", crate::world_tick::SMELTER_RECIPE_ID),
                &ctx(),
            )
            .ok,
            "Smelter recipe cannot cross into a Workshop"
        );
        assert!(
            !apply_action(
                &mut world,
                &queue_add("smelter-player", crate::world_tick::WORKSHOP_RECIPE_ID),
                &ctx(),
            )
            .ok,
            "Workshop recipe cannot cross into a Smelter"
        );
    }

    #[test]
    fn signed_queue_add_is_denied_until_signed_research_unlocks_the_recipe() {
        let mut world = world_with_one_colony();
        let colony = &mut world.colonies[0];
        colony.recipe_entitlement_rules_version =
            crate::world_tick::CURRENT_RECIPE_ENTITLEMENT_RULES_VERSION;
        colony.buildings.push(BuildingRuntime {
            id: "locked-sawmill".to_owned(),
            building_type: BuildingType::Sawmill,
            is_complete: true,
            construction_progress: 100,
            production_queue: Vec::new(),
            ..BuildingRuntime::default()
        });
        let add = proto::ClientAction::EditProductionQueue {
            session_id: "sess_1".to_owned(),
            nickname: "Player".to_owned(),
            sig: "signed".to_owned(),
            building_id: "locked-sawmill".to_owned(),
            edit: proto::ProductionQueueEdit::Add {
                recipe_id: crate::world_tick::SAWMILL_RECIPE_ID.to_owned(),
                repeat: true,
            },
        };

        let before = world.colonies[0].buildings.last().unwrap().clone();
        let denied = apply_action(&mut world, &add, &ctx());
        assert!(!denied.ok);
        assert_eq!(world.colonies[0].buildings.last().unwrap(), &before);
        let locked_snapshot = build_snapshot(&world, ctx().now_ms, 1);
        let locked = locked_snapshot.colonies[0]
            .buildings
            .iter()
            .find(|building| building.id == "locked-sawmill")
            .unwrap();
        assert!(locked.available_recipes.is_empty());
        assert_eq!(
            locked.production_block_reason.as_deref(),
            Some("research_locked")
        );
        assert_eq!(
            locked
                .required_recipe_study
                .as_ref()
                .map(|study| study.id.as_str()),
            Some("carpentry_preparation")
        );

        world.colonies[0].upgrade_tree.research_points = 100.0;
        for node_id in [
            "research_hut",
            "basic_tools",
            "foraging_lore",
            "sawmill",
            "carpentry_sources",
            "carpentry_preparation",
        ] {
            assert!(
                crate::upgrade_tree::can_unlock(&world.colonies[0].upgrade_tree, node_id),
                "{node_id} not unlockable from {:?}",
                world.colonies[0].upgrade_tree.owned_node_ids
            );
            let research = proto::ClientAction::ResearchNode {
                session_id: "sess_1".to_owned(),
                nickname: "Player".to_owned(),
                sig: "signed".to_owned(),
                node_id: node_id.to_owned(),
            };
            assert!(apply_action(&mut world, &research, &ctx()).ok, "{node_id}");
        }
        assert!(apply_action(&mut world, &add, &ctx()).ok);
        assert_eq!(
            world.colonies[0].buildings.last().unwrap().production_queue,
            [crate::world_tick::ProductionQueueEntry {
                recipe_id: crate::world_tick::SAWMILL_RECIPE_ID.to_owned(),
                repeat: true,
            }]
        );

        let snapshot = build_snapshot(&world, ctx().now_ms, 1);
        let sawmill = snapshot.colonies[0]
            .buildings
            .iter()
            .find(|building| building.id == "locked-sawmill")
            .unwrap();
        assert_eq!(
            sawmill.available_recipes,
            [crate::world_tick::SAWMILL_RECIPE_ID]
        );
        assert_eq!(sawmill.required_recipe_study, None);
    }

    #[test]
    fn foreign_personal_village_denies_labor_and_queue_mutations() {
        let mut world = world_with_one_colony();
        world.colonies[0].kind = VillageKind::Personal;
        world.colonies[0].owner_player_id = Some("another-player".to_owned());
        let cat_id = world.colonies[0].cats[0].id.clone();
        let result = apply_action(
            &mut world,
            &proto::ClientAction::SetCatLaborPreference {
                session_id: "sess_1".to_owned(),
                nickname: "Player".to_owned(),
                sig: "signed".to_owned(),
                cat_id,
                labor: proto::Labor::Haul,
                enabled: true,
            },
            &ctx(),
        );
        assert!(!result.ok);
    }

    fn prepare_fishing_shore(world: &mut WorldState) -> (TilePos, TilePos) {
        let colony = &world.colonies[0];
        let bank = colony
            .world_tiles
            .keys()
            .copied()
            .find(|bank| {
                colony.revealed_tiles.contains(bank)
                    && stockpile_placement_error(
                        colony,
                        zones::ZoneRect {
                            x1: bank.x,
                            y1: bank.y,
                            x2: bank.x,
                            y2: bank.y,
                        },
                        world.world_seed,
                        false,
                    )
                    .is_none()
                    && colony.world_tiles.contains_key(&TilePos {
                        x: bank.x,
                        y: bank.y - 1,
                    })
                    && {
                        let water = TilePos {
                            x: bank.x,
                            y: bank.y - 1,
                        };
                        let mut projected = colony.clone();
                        projected.revealed_tiles.insert(water);
                        let tile = projected.world_tiles.get_mut(&water).unwrap();
                        tile.tile_type = TileType::River;
                        tile.resources.water = 100;
                        crate::world_tick::is_reachable_fishing_shore(
                            &projected,
                            *bank,
                            world.world_seed,
                        )
                    }
            })
            .expect("founding reveal has a clear tile with a mapped neighbor");
        let water = TilePos {
            x: bank.x,
            y: bank.y - 1,
        };
        let colony = &mut world.colonies[0];
        colony.revealed_tiles.insert(water);
        let water_tile = colony.world_tiles.get_mut(&water).unwrap();
        water_tile.tile_type = TileType::River;
        water_tile.resources.water = 100;
        (bank, water)
    }

    #[test]
    fn fishing_designation_is_spatial_typed_durable_and_visible_in_snapshot() {
        let mut world = world_with_one_colony();
        let inland = open_gather_point(&world);
        let before = world.colonies[0].clone();
        let rejected = apply_action(
            &mut world,
            &proto::ClientAction::DesignateFishingSpot {
                session_id: "sess_1".to_owned(),
                nickname: "Angler".to_owned(),
                sig: "signed".to_owned(),
                at: inland,
            },
            &ctx(),
        );
        assert!(!rejected.ok, "inland fishing must be rejected");
        assert_eq!(world.colonies[0], before, "rejection is atomic");

        let (bank, water) = prepare_fishing_shore(&mut world);
        let designated = apply_action(
            &mut world,
            &proto::ClientAction::DesignateFishingSpot {
                session_id: "sess_1".to_owned(),
                nickname: "Angler".to_owned(),
                sig: "signed".to_owned(),
                at: proto::TilePoint {
                    x: water.x,
                    y: water.y,
                },
            },
            &ctx(),
        );
        assert!(
            designated.ok,
            "water click resolves to its clear bank: {designated:?}"
        );
        let colony = &world.colonies[0];
        let spot = colony.gather_spots.last().unwrap();
        assert_eq!(spot.purpose, stockpiles::GatherSpotPurpose::Fishing);
        assert_eq!(spot.kind, stockpiles::ResourceKind::Fish);
        assert_eq!(spot.expires_at_ms, i64::MAX);
        let pile = colony
            .stockpiles
            .iter()
            .find(|pile| pile.id == spot.stockpile_id)
            .unwrap();
        assert_eq!((pile.rect.x1, pile.rect.y1), (bank.x, bank.y));
        assert_eq!(pile.rect.x1, pile.rect.x2);
        assert_eq!(pile.rect.y1, pile.rect.y2);

        let snapshot = build_snapshot(&world, ctx().now_ms, 1);
        let visible = snapshot.colonies[0]
            .stockpiles
            .iter()
            .find(|pile| pile.id == spot.stockpile_id)
            .unwrap()
            .gather_spot
            .as_ref()
            .unwrap();
        assert_eq!(visible.purpose, proto::GatherSpotPurpose::Fishing);
        assert_eq!(visible.kind, proto::ResourceKind::Fish);
        assert_eq!(
            visible.fish_population.unwrap().stock,
            stockpiles::FISH_POPULATION_CAPACITY
        );
    }

    #[test]
    fn manual_fishing_requires_a_site_and_honors_exact_labor_preference() {
        let mut world = world_with_one_colony();
        let request = |world: &mut WorldState| {
            apply_action(
                world,
                &proto::ClientAction::RequestJob {
                    session_id: "sess_1".to_owned(),
                    nickname: "Angler".to_owned(),
                    sig: "signed".to_owned(),
                    kind: proto::JobKind::Fish,
                },
                &ctx(),
            )
        };
        assert!(!request(&mut world).ok);
        let (bank, _) = prepare_fishing_shore(&mut world);
        assert!(
            apply_action(
                &mut world,
                &proto::ClientAction::DesignateFishingSpot {
                    session_id: "sess_1".to_owned(),
                    nickname: "Angler".to_owned(),
                    sig: "signed".to_owned(),
                    at: proto::TilePoint {
                        x: bank.x,
                        y: bank.y
                    },
                },
                &ctx(),
            )
            .ok
        );
        let preferred_id = world.colonies[0].cats[3].id.clone();
        world.colonies[0].cats[3]
            .preferred_labors
            .insert(Labor::Fishing);
        assert!(request(&mut world).ok);
        let job = world.colonies[0]
            .jobs
            .iter()
            .find(|job| job.kind == JobKind::Fish)
            .unwrap();
        assert_eq!(job.assigned_cat.as_deref(), Some(preferred_id.as_str()));
    }

    #[test]
    fn manual_fishing_rejects_a_depleted_habitat_until_it_replenishes() {
        let mut world = world_with_one_colony();
        let (bank, water) = prepare_fishing_shore(&mut world);
        assert!(
            apply_action(
                &mut world,
                &proto::ClientAction::DesignateFishingSpot {
                    session_id: "sess_1".to_owned(),
                    nickname: "Angler".to_owned(),
                    sig: "signed".to_owned(),
                    at: proto::TilePoint {
                        x: bank.x,
                        y: bank.y,
                    },
                },
                &ctx(),
            )
            .ok
        );
        world.colonies[0]
            .fish_habitats
            .get_mut(&water)
            .expect("designation creates its canonical habitat")
            .stock = 0.0;

        let result = apply_action(
            &mut world,
            &proto::ClientAction::RequestJob {
                session_id: "sess_1".to_owned(),
                nickname: "Angler".to_owned(),
                sig: "signed".to_owned(),
                kind: proto::JobKind::Fish,
            },
            &ctx(),
        );

        assert!(!result.ok);
        assert_eq!(
            result.message.as_deref(),
            Some("The designated fish habitat is depleted and replenishing.")
        );
        assert!(
            world.colonies[0]
                .jobs
                .iter()
                .all(|job| job.kind != JobKind::Fish)
        );
    }

    #[test]
    fn fishing_designation_rejects_a_clear_but_unreachable_bank() {
        let mut world = world_with_one_colony();
        let bank = TilePos {
            x: world.colonies[0].anchor.x + 30,
            y: world.colonies[0].anchor.y + 30,
        };
        for dy in -1..=1 {
            for dx in -1..=1 {
                let pos = TilePos {
                    x: bank.x + dx,
                    y: bank.y + dy,
                };
                let mut tile = crate::world_tick::fresh_ground_tile(pos);
                if dx == 0 && dy == -1 {
                    tile.tile_type = TileType::River;
                    tile.resources.water = 100;
                } else if dx != 0 || dy != 0 {
                    tile.tile_type = TileType::Mountains;
                }
                world.colonies[0].world_tiles.insert(pos, tile);
                world.colonies[0].revealed_tiles.insert(pos);
            }
        }

        let result = apply_action(
            &mut world,
            &proto::ClientAction::DesignateFishingSpot {
                session_id: "sess_1".to_owned(),
                nickname: "Angler".to_owned(),
                sig: "signed".to_owned(),
                at: proto::TilePoint {
                    x: bank.x,
                    y: bank.y,
                },
            },
            &ctx(),
        );

        assert!(!result.ok);
        assert!(
            world.colonies[0]
                .gather_spots
                .iter()
                .all(|spot| spot.purpose != stockpiles::GatherSpotPurpose::Fishing)
        );
    }

    #[test]
    fn removing_fishing_spot_cancels_its_job_and_releases_worker() {
        let mut world = world_with_one_colony();
        let (bank, _) = prepare_fishing_shore(&mut world);
        assert!(
            apply_action(
                &mut world,
                &proto::ClientAction::DesignateFishingSpot {
                    session_id: "sess_1".to_owned(),
                    nickname: "Angler".to_owned(),
                    sig: "signed".to_owned(),
                    at: proto::TilePoint {
                        x: bank.x,
                        y: bank.y
                    },
                },
                &ctx(),
            )
            .ok
        );
        assert!(
            apply_action(
                &mut world,
                &proto::ClientAction::RequestJob {
                    session_id: "sess_1".to_owned(),
                    nickname: "Angler".to_owned(),
                    sig: "signed".to_owned(),
                    kind: proto::JobKind::Fish,
                },
                &ctx(),
            )
            .ok
        );
        let spot_id = world.colonies[0]
            .gather_spots
            .iter()
            .find(|spot| spot.purpose == stockpiles::GatherSpotPurpose::Fishing)
            .unwrap()
            .stockpile_id
            .clone();
        let worker_id = world.colonies[0]
            .jobs
            .iter()
            .find(|job| job.kind == JobKind::Fish)
            .unwrap()
            .assigned_cat
            .clone()
            .unwrap();

        let removed = apply_action(
            &mut world,
            &proto::ClientAction::RemoveGatherSpot {
                session_id: "sess_1".to_owned(),
                nickname: "Angler".to_owned(),
                sig: "signed".to_owned(),
                stockpile_id: spot_id,
            },
            &ctx(),
        );

        assert!(removed.ok);
        let job = world.colonies[0]
            .jobs
            .iter()
            .find(|job| job.kind == JobKind::Fish)
            .unwrap();
        assert_eq!(job.status, JobStatus::Cancelled);
        let worker = world.colonies[0]
            .cats
            .iter()
            .find(|cat| cat.id == worker_id)
            .unwrap();
        assert_eq!(worker.activity, CatActivity::Idle);
        assert_eq!(worker.destination, None);
        assert_eq!(worker.current_task, None);
    }

    #[test]
    fn removing_fishing_spot_preserves_and_retargets_earned_cargo() {
        let mut world = world_with_one_colony();
        let (bank, _) = prepare_fishing_shore(&mut world);
        assert!(
            apply_action(
                &mut world,
                &proto::ClientAction::DesignateFishingSpot {
                    session_id: "sess_1".to_owned(),
                    nickname: "Angler".to_owned(),
                    sig: "signed".to_owned(),
                    at: proto::TilePoint {
                        x: bank.x,
                        y: bank.y,
                    },
                },
                &ctx(),
            )
            .ok
        );
        assert!(
            apply_action(
                &mut world,
                &proto::ClientAction::RequestJob {
                    session_id: "sess_1".to_owned(),
                    nickname: "Angler".to_owned(),
                    sig: "signed".to_owned(),
                    kind: proto::JobKind::Fish,
                },
                &ctx(),
            )
            .ok
        );
        let spot_id = world.colonies[0]
            .gather_spots
            .iter()
            .find(|spot| spot.purpose == stockpiles::GatherSpotPurpose::Fishing)
            .unwrap()
            .stockpile_id
            .clone();
        let job_index = world.colonies[0]
            .jobs
            .iter()
            .position(|job| job.kind == JobKind::Fish)
            .unwrap();
        let worker_id = world.colonies[0].jobs[job_index]
            .assigned_cat
            .clone()
            .unwrap();
        // A final fishing trip marks the job complete before its carrier reaches
        // storage; removing the typed pile must still find and retarget it.
        world.colonies[0].jobs[job_index].status = JobStatus::Completed;
        world.colonies[0].jobs[job_index].metadata = JobMetadata::Hauling {
            site: Some(bank),
            total_yield: Some(12.0),
            trips_done: 1,
            next_trip_at: None,
            accepted: true,
        };
        let worker = world.colonies[0]
            .cats
            .iter_mut()
            .find(|cat| cat.id == worker_id)
            .unwrap();
        worker.activity = CatActivity::Returning;
        worker.current_task = Some(TaskType::Fish);
        worker.destination = Some(Position {
            map: MapType::World,
            x: f64::from(bank.x),
            y: f64::from(bank.y),
        });
        worker.carrying = Some(entities::Carrying {
            kind: entities::CarryingKind::Fish,
            amount: 4.0,
            job_ended_at: ctx().now_ms,
            source_gather_spot: None,
        });

        assert!(
            apply_action(
                &mut world,
                &proto::ClientAction::RemoveGatherSpot {
                    session_id: "sess_1".to_owned(),
                    nickname: "Angler".to_owned(),
                    sig: "signed".to_owned(),
                    stockpile_id: spot_id,
                },
                &ctx(),
            )
            .ok
        );

        let worker = world.colonies[0]
            .cats
            .iter()
            .find(|cat| cat.id == worker_id)
            .unwrap();
        assert_eq!(worker.carrying.as_ref().unwrap().amount, 4.0);
        assert_eq!(worker.activity, CatActivity::Returning);
        assert_eq!(worker.current_task, None);
        assert_ne!(
            worker.destination,
            Some(Position {
                map: MapType::World,
                x: f64::from(bank.x),
                y: f64::from(bank.y),
            })
        );
        assert_eq!(
            world.colonies[0].jobs[job_index].status,
            JobStatus::Cancelled
        );
    }
}
