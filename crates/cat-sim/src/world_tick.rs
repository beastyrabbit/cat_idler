//! Runtime world tick skeleton ported from `server/game.ts:workerTick`.
//!
//! This P7.1 module owns the in-memory runtime shapes and phase ordering. Later
//! P7 cards fill in the no-op phase bodies with the pure module calls.

use std::collections::BTreeMap;

use crate::{
    biomes::MaxResources,
    depletion::{is_forest_type, regrowth_amount},
    elections::{
        BallotVote, ELECTION_WINDOW_MS, ElectionCandidate, KICK_WINDOW_MS, TERM_MS,
        candidates_for_unbarred, election_due, election_winner, should_trigger_kick, tally_votes,
    },
    entities::{Cat, CatActivity, ColonyStatus, Position, Resources},
    idle_engine,
    idle_rules::consumption_for_tick,
    leader_ai::{LeaderDecision, LeaderHousing, LeaderResources, LeaderSnapshot},
    leader_director::{
        CatBrief, CatBriefStats, DirectorPlan, LaborGoalKind, MatchOptions, direct_colony,
        match_cats_to_slots,
    },
    life_sim::{can_work, get_life_stage, leadership_after_tenure, old_age_death_probability},
    policy::PolicyConfig,
    rng::{life_seed, movement_seed, raid_seed, roll_seeded},
    spoilage::apply_food_spoilage_after_consumption,
    storage::{
        StorageBuilding, StorageCapacities, count_storehouses, storage_capacities, storehouse_cap,
    },
    threat::threat_band,
    types::{BuildingType, CatSpecialization, JobKind, JobStatus, TaskType, TileType, UpgradeKey},
    upgrade_tree::{UpgradeTreeState, create_upgrade_tree_state, resolve_effects},
    world_gen::TileResources,
    zones::{ZoneKind, ZoneRect},
};

pub type ColonyId = String;
pub type CatId = String;
pub type JobId = String;
pub type BuildingId = String;
pub type EventId = String;
pub type ElectionId = String;
pub type VoteId = String;
pub type RaiderId = String;
pub type RaidId = String;
pub type PlayerId = String;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct WorldState {
    pub world_seed: u32,
    pub colonies: Vec<ColonyRuntime>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColonyRuntime {
    pub id: ColonyId,
    pub name: String,
    pub leader_id: Option<CatId>,
    pub status: ColonyStatus,
    pub resources: Resources,
    pub cats: Vec<Cat>,
    pub jobs: Vec<JobRuntime>,
    pub buildings: Vec<BuildingRuntime>,
    pub events: Vec<EventLog>,
    pub world_tiles: BTreeMap<TilePos, WorldTileRuntime>,
    pub zones: Vec<ZoneRuntime>,
    pub elections: Vec<ElectionRuntime>,
    pub votes: Vec<VoteRuntime>,
    pub raiders: Vec<RaiderRuntime>,
    pub upgrade_levels: UpgradeLevels,
    pub upgrade_tree: UpgradeTreeState,
    pub automation_tier: f64,
    pub global_upgrade_points: f64,
    pub ritual_requested_at: Option<i64>,
    pub critical_since: Option<i64>,
    pub claimed_tiles: Vec<TilePos>,
    pub threat_pressure: f64,
    pub last_raid_at: Option<i64>,
    pub active_raid: Option<RaidId>,
    pub raid_clicks: f64,
    pub run_number: u32,
    pub run_started_at: i64,
    pub created_at: i64,
    pub last_player_activity_at: Option<i64>,
    pub last_tick: i64,
    pub test_time_scale: f64,
    pub test_resource_decay_multiplier: f64,
    pub test_resilience_hours_override: Option<f64>,
    pub test_critical_ms_override: i64,
    pub test_rng_seed: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JobRuntime {
    pub id: JobId,
    pub kind: JobKind,
    pub status: JobStatus,
    pub requested_by: JobRequester,
    pub assigned_cat: Option<CatId>,
    pub duration_ms: i64,
    pub speed: f64,
    pub yield_amount: f64,
    pub click_count: u32,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub ends_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub metadata: JobMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobRequester {
    Player,
    Leader,
    System,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JobMetadata {
    None,
    Construction {
        phase: ConstructionPhase,
        building_type: BuildingType,
        building_id: Option<BuildingId>,
        site: Option<TilePos>,
    },
    Expansion {
        target: TilePos,
        accepted: bool,
    },
    Hauling {
        site: Option<TilePos>,
        total_yield: Option<f64>,
        trips_done: u32,
        next_trip_at: Option<i64>,
        accepted: bool,
    },
    Site {
        site: TilePos,
        accepted: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstructionPhase {
    GatherMaterials,
    ConstructHouse,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuildingRuntime {
    pub id: BuildingId,
    pub building_type: BuildingType,
    pub level: u32,
    pub position: TilePos,
    pub is_complete: bool,
    pub construction_progress: u8,
    pub production_progress: f64,
    pub assigned_cat: Option<CatId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventLog {
    pub id: EventId,
    pub at_ms: i64,
    pub kind: EventKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    LeaderChange,
    JobQueued,
    JobCompleted,
    ResourceCrisis,
    ResourceRecovered,
    Election,
    Raid,
    Reset,
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldTileRuntime {
    pub pos: TilePos,
    pub tile_type: TileType,
    pub resources: TileResources,
    pub max_resources: MaxResources,
    pub danger_level: f64,
    pub path_wear: u32,
    pub last_depleted: i64,
    pub overlay_feature: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneRuntime {
    pub rect: ZoneRect,
    pub kind: ZoneKind,
    pub created_at: i64,
    pub expires_at: i64,
    pub player_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElectionRuntime {
    pub id: ElectionId,
    pub opened_at: i64,
    pub closes_at: i64,
    pub resolved_at: Option<i64>,
    pub winner_cat_id: Option<CatId>,
    pub kind: ElectionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElectionKind {
    Scheduled,
    Snap,
    VoteKick,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VoteRuntime {
    pub id: VoteId,
    pub election_id: ElectionId,
    pub voter_id: PlayerId,
    pub cat_id: CatId,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RaiderRuntime {
    pub id: RaiderId,
    pub raid_id: RaidId,
    pub position: Position,
    pub destination: Option<Position>,
    pub attack: f64,
    pub defense: f64,
    pub health: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UpgradeLevels {
    pub click_power: u32,
    pub supply_speed: u32,
    pub hunt_mastery: u32,
    pub build_mastery: u32,
    pub ritual_mastery: u32,
    pub resilience: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TickReport {
    pub colony_id: ColonyId,
    pub skipped: bool,
    pub reset_reason: Option<RunResetReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunResetReason {
    AllCatsDead,
    RaidWipeout,
    UnattendedCollapse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TickGate {
    elapsed_sec: i64,
    processed_through: i64,
    minute_rolled: bool,
    previous_water: u64,
}

#[derive(Debug, Clone, Copy)]
struct TickPolicy {
    config: PolicyConfig,
}

const EVENT_KEEP: usize = 2_000;
const MAX_PATH_DECAY_PER_TICK: u32 = 2;

impl Default for ColonyRuntime {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            leader_id: None,
            status: ColonyStatus::default(),
            resources: Resources::default(),
            cats: Vec::new(),
            jobs: Vec::new(),
            buildings: Vec::new(),
            events: Vec::new(),
            world_tiles: BTreeMap::new(),
            zones: Vec::new(),
            elections: Vec::new(),
            votes: Vec::new(),
            raiders: Vec::new(),
            upgrade_levels: UpgradeLevels::default(),
            upgrade_tree: create_upgrade_tree_state(),
            automation_tier: 0.0,
            global_upgrade_points: 0.0,
            ritual_requested_at: None,
            critical_since: None,
            claimed_tiles: Vec::new(),
            threat_pressure: 0.0,
            last_raid_at: None,
            active_raid: None,
            raid_clicks: 0.0,
            run_number: 0,
            run_started_at: 0,
            created_at: 0,
            last_player_activity_at: None,
            last_tick: 0,
            test_time_scale: 1.0,
            test_resource_decay_multiplier: 1.0,
            test_resilience_hours_override: None,
            test_critical_ms_override: 5 * 60 * 1000,
            test_rng_seed: None,
        }
    }
}

impl Default for JobRuntime {
    fn default() -> Self {
        Self {
            id: String::new(),
            kind: JobKind::SupplyFood,
            status: JobStatus::Queued,
            requested_by: JobRequester::System,
            assigned_cat: None,
            duration_ms: 0,
            speed: 0.0,
            yield_amount: 0.0,
            click_count: 0,
            created_at: 0,
            started_at: None,
            ends_at: None,
            completed_at: None,
            metadata: JobMetadata::None,
        }
    }
}

impl Default for BuildingRuntime {
    fn default() -> Self {
        Self {
            id: String::new(),
            building_type: BuildingType::Den,
            level: 1,
            position: TilePos { x: 0, y: 0 },
            is_complete: false,
            construction_progress: 0,
            production_progress: 0.0,
            assigned_cat: None,
        }
    }
}

impl UpgradeLevels {
    #[must_use]
    pub fn get(&self, key: UpgradeKey) -> u32 {
        match key {
            UpgradeKey::ClickPower => self.click_power,
            UpgradeKey::SupplySpeed => self.supply_speed,
            UpgradeKey::HuntMastery => self.hunt_mastery,
            UpgradeKey::BuildMastery => self.build_mastery,
            UpgradeKey::RitualMastery => self.ritual_mastery,
            UpgradeKey::Resilience => self.resilience,
        }
    }
}

#[must_use]
pub fn world_tick(state: &mut WorldState, now_ms: i64) -> Vec<TickReport> {
    let mut indices: Vec<usize> = (0..state.colonies.len()).collect();
    indices.sort_by(|left, right| state.colonies[*left].id.cmp(&state.colonies[*right].id));

    let mut reports = Vec::with_capacity(indices.len());
    for index in indices {
        let colony = &mut state.colonies[index];
        let Some(gate) = phase_1_colony_selection_and_elapsed_time_gate(colony, now_ms) else {
            reports.push(TickReport {
                colony_id: colony.id.clone(),
                skipped: true,
                reset_reason: None,
            });
            continue;
        };

        phase_2_runtime_upgrades_and_effects(colony, gate);
        phase_3_base_rng_and_fork_roots(colony, gate);
        let policy = phase_4_leader_bootstrap_and_policy(colony, gate);
        phase_5_initial_roster_buildings_and_caps(colony, gate);
        phase_6_life_simulation(colony, gate);
        phase_7_consumption_spoilage_resource_pre_patch_minute_cadence(colony, gate);
        phase_8_water_low_crisis_edge(colony, gate);
        phase_9_elections_lifecycle(colony, gate);
        phase_10_zones_and_event_pruning(colony, gate);
        phase_11_path_wear_decay(colony, gate);
        phase_12_resource_regrowth(colony, gate);
        phase_13_tick_local_target_caches(colony, gate);
        phase_14_promote_queued_jobs_and_break_ground(colony, gate);
        phase_15_assign_promoted_job_destinations(colony, gate);
        phase_16_active_scaffold_progress(colony, gate);
        phase_17_legacy_emergency_hunt(colony, gate, policy);
        let snapshot = phase_18_leader_snapshot_assembly(colony, gate);
        let plan = phase_19_leader_cancellations(colony, gate, &snapshot);
        phase_20_leader_labor_assignments_and_staffing(colony, gate, policy, &plan);
        phase_21_leader_capital_decisions_and_tithe(colony, gate, policy, &plan);
        phase_22_ritual_approval(colony, gate);
        phase_23_production(colony, gate);
        phase_24_research(colony, gate);
        phase_25_survival_deaths_and_carried_yield_salvage(colony, gate);
        phase_26_empty_colony_reset(colony, gate);
        phase_27_due_job_prelude(colony, gate);
        phase_28_due_completion_supplies_and_planner_jobs(colony, gate);
        phase_29_due_completion_gathering_explore_expansion(colony, gate);
        phase_30_due_completion_build_ritual_training_return_mark_done(colony, gate);
        phase_31_mid_job_hauling(colony, gate);
        phase_32_movement_setup_and_village_expansion_queue(colony, gate);
        phase_33_movement_deposits_and_no_destination_wander(colony, gate);
        phase_34_movement_travel_job_acceptance_reveal_path_wear(colony, gate);
        phase_35_deliberate_roads(colony, gate);
        phase_36_threat_and_raid_director(colony, gate);
        phase_37_final_clamp_critical_collapse_status_persist(colony, gate);

        reports.push(TickReport {
            colony_id: colony.id.clone(),
            skipped: false,
            reset_reason: None,
        });
    }

    reports
}

/// Phase 1: select the colony, compute elapsed seconds, and skip sub-second ticks
/// without mutating `last_tick`.
fn phase_1_colony_selection_and_elapsed_time_gate(
    colony: &ColonyRuntime,
    now_ms: i64,
) -> Option<TickGate> {
    let elapsed_sec = now_ms.saturating_sub(colony.last_tick).max(0) / 1000;
    if elapsed_sec == 0 {
        return None;
    }

    Some(TickGate {
        elapsed_sec,
        processed_through: colony
            .last_tick
            .saturating_add(elapsed_sec.saturating_mul(1000)),
        minute_rolled: elapsed_sec >= 60
            || now_ms.div_euclid(60_000) != colony.last_tick.div_euclid(60_000),
        previous_water: colony.resources.water.to_bits(),
    })
}

/// Phase 2: load runtime config, legacy upgrade levels, upgrade tree state, and
/// resolved effects for this tick.
fn phase_2_runtime_upgrades_and_effects(_: &mut ColonyRuntime, gate: TickGate) {
    let _ = (gate.elapsed_sec, gate.processed_through);
}

/// Phase 3: initialize the base seeded RNG and derive movement, life, and raid
/// fork roots without persisting fork state.
fn phase_3_base_rng_and_fork_roots(colony: &mut ColonyRuntime, _: TickGate) {
    let base_seed = colony.test_rng_seed.unwrap_or(1).max(1);
    let _ = (
        movement_seed(base_seed),
        life_seed(base_seed),
        raid_seed(base_seed),
    );
    colony.test_rng_seed = Some(base_seed);
}

/// Phase 4: choose or repair the leader, log leader changes, roll policy tier,
/// and compute policy action reliability.
fn phase_4_leader_bootstrap_and_policy(colony: &mut ColonyRuntime, gate: TickGate) -> TickPolicy {
    let leader_missing_or_dead = colony
        .leader_id
        .as_ref()
        .is_none_or(|leader_id| !alive_cats(&colony.cats).any(|cat| cat.id == *leader_id));

    if leader_missing_or_dead {
        let mut best_leader: Option<&Cat> = None;
        for candidate in alive_cats(&colony.cats) {
            if best_leader.is_none_or(|best| candidate.stats.leadership > best.stats.leadership) {
                best_leader = Some(candidate);
            }
        }
        if let Some(leader) = best_leader {
            colony.leader_id = Some(leader.id.clone());
            append_event(
                colony,
                gate.processed_through,
                EventKind::LeaderChange,
                format!("{} is now the interim leader.", leader.name),
            );
        }
    }

    let leadership = colony
        .leader_id
        .as_ref()
        .and_then(|leader_id| {
            alive_cats(&colony.cats)
                .find(|cat| cat.id == *leader_id)
                .map(|cat| cat.stats.leadership)
        })
        .unwrap_or(50.0);

    let policy_roll = next_base_roll(colony);
    let policy_tier = crate::policy::pick_policy_tier(leadership, policy_roll);
    let policy_config = crate::policy::config_for_tier(policy_tier);
    TickPolicy {
        config: policy_config,
    }
}

/// Phase 5: snapshot alive cats/buildings and compute initial storage caps.
fn phase_5_initial_roster_buildings_and_caps(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 6: age cats, process old-age deaths, leadership tenure, milestones,
/// births, conceptions, and death-related job cancellation.
fn phase_6_life_simulation(colony: &mut ColonyRuntime, gate: TickGate) {
    let elapsed_game_hours = elapsed_game_hours(colony, gate);
    if elapsed_game_hours <= 0.0 {
        return;
    }

    let mut life_rng_seed = life_seed(colony.test_rng_seed.unwrap_or(1));
    let leader_id = colony.leader_id.clone();

    for cat in &mut colony.cats {
        if cat.death_time.is_some() {
            continue;
        }

        cat.age_hours += elapsed_game_hours;

        let is_leader_or_healer =
            leader_id.as_ref() == Some(&cat.id) || cat.stats.medicine >= cat.stats.leadership;
        let death_probability =
            old_age_death_probability(cat.age_hours, is_leader_or_healer, elapsed_game_hours);
        if death_probability > 0.0 {
            let roll = roll_seeded(f64::from(life_rng_seed));
            life_rng_seed = roll.next_seed;
            if roll.value < death_probability {
                cat.death_time = Some(gate.processed_through);
                cat.activity = Default::default();
                cat.destination = None;
                cat.carrying = None;
                continue;
            }
        }

        if leader_id.as_ref() == Some(&cat.id) {
            cat.stats.leadership =
                leadership_after_tenure(cat.stats.leadership, elapsed_game_hours);
        }
    }
}

/// Phase 7: consume food/water, apply spoilage and resource caps, prepare
/// `nextResources`, and compute minute cadence.
fn phase_7_consumption_spoilage_resource_pre_patch_minute_cadence(
    colony: &mut ColonyRuntime,
    gate: TickGate,
) {
    let cat_count = alive_cats(&colony.cats).count() as f64;
    let elapsed_for_decay = gate.elapsed_sec as f64 * normalize_resource_decay_multiplier(colony);
    let consumption = consumption_for_tick(
        cat_count,
        elapsed_for_decay,
        idle_engine_upgrade_levels(&colony.upgrade_levels),
    );
    let caps = storage_caps(colony);

    colony.resources.food = apply_food_spoilage_after_consumption(
        colony.resources.food,
        consumption.food_use,
        caps.food,
        elapsed_for_decay,
    );
    colony.resources.water =
        clamp_resource(colony.resources.water - consumption.water_use, caps.water);
    colony.resources.herbs = clamp_resource(colony.resources.herbs, caps.herbs);
    colony.resources.materials = clamp_resource(colony.resources.materials, caps.materials);
    colony.resources.refined = clamp_resource(colony.resources.refined, caps.refined);
    colony.resources.weapons = clamp_resource(colony.resources.weapons, caps.weapons);
    colony.resources.armor = clamp_resource(colony.resources.armor, caps.armor);
}

/// Phase 8: append the water crisis edge event when water crosses the low
/// threshold.
fn phase_8_water_low_crisis_edge(colony: &mut ColonyRuntime, gate: TickGate) {
    let previous_water = f64::from_bits(gate.previous_water);
    if previous_water > 3.0 && colony.resources.water <= 3.0 {
        append_event(
            colony,
            gate.processed_through,
            EventKind::ResourceCrisis,
            "CRISIS: WATER RESERVES DANGEROUSLY LOW",
        );
    }
}

/// Phase 9: resolve due elections/vote-kicks and open scheduled or snap
/// elections.
fn phase_9_elections_lifecycle(colony: &mut ColonyRuntime, gate: TickGate) {
    let open_leadership = colony
        .elections
        .iter()
        .position(is_open_leadership_election);
    let open_kick = colony.elections.iter().position(|election| {
        election.kind == ElectionKind::VoteKick && election.resolved_at.is_none()
    });

    let mut open_leadership_after_resolution = open_leadership;
    if let Some(index) =
        open_leadership.filter(|index| colony.elections[*index].closes_at <= gate.processed_through)
    {
        let election_id = colony.elections[index].id.clone();
        let candidates = current_election_candidates(colony);
        let ballots = ballots_for(colony, &election_id);
        let tally = tally_votes(&ballots);
        let winner_id = election_winner(&candidates, &tally);

        colony.elections[index].resolved_at = Some(gate.processed_through);
        colony.elections[index].winner_cat_id = winner_id.clone();
        open_leadership_after_resolution = None;

        if let Some(winner_id) = winner_id {
            colony.leader_id = Some(winner_id.clone());
            let winner_name = alive_cats(&colony.cats)
                .find(|cat| cat.id == winner_id)
                .map_or("The winner", |cat| cat.name.as_str())
                .to_owned();
            append_event(
                colony,
                gate.processed_through,
                EventKind::Election,
                format!(
                    "{winner_name} won the leadership election with {} ballot{} cast.",
                    ballots.len(),
                    if ballots.len() == 1 { "" } else { "s" }
                ),
            );
        }
    }

    if let Some(index) =
        open_kick.filter(|index| colony.elections[*index].closes_at <= gate.processed_through)
    {
        let election_id = colony.elections[index].id.clone();
        let ballots = ballots_for(colony, &election_id);
        let target_id = vote_kick_target(&ballots);
        let kicked = colony.leader_id.is_some()
            && target_id.as_ref() == colony.leader_id.as_ref()
            && should_trigger_kick(&ballots);

        colony.elections[index].resolved_at = Some(gate.processed_through);
        colony.elections[index].winner_cat_id = kicked.then(|| target_id.clone()).flatten();

        if kicked {
            if let Some(target_id) = target_id {
                let target_name = alive_cats(&colony.cats)
                    .find(|cat| cat.id == target_id)
                    .map_or("The leader", |cat| cat.name.as_str())
                    .to_owned();
                append_event(
                    colony,
                    gate.processed_through,
                    EventKind::Election,
                    format!("{target_name} was voted out by the players!"),
                );
                colony.leader_id = choose_interim_leader_excluding(colony, Some(&target_id));
            }

            if open_leadership_after_resolution.is_none() {
                open_leadership_election(colony, gate.processed_through, ElectionKind::Snap);
                open_leadership_after_resolution = colony
                    .elections
                    .iter()
                    .position(is_open_leadership_election);
            }
        }
    }

    if open_leadership_after_resolution.is_none()
        && election_due(
            last_resolved_leadership_election_at(colony).map(|at| at as f64),
            gate.processed_through as f64,
            scaled_term_ms(colony),
        )
    {
        open_leadership_election(colony, gate.processed_through, ElectionKind::Scheduled);
    }
}

/// Phase 10: expire zones and prune the event log to the newest retained events
/// on the minute cadence.
fn phase_10_zones_and_event_pruning(colony: &mut ColonyRuntime, gate: TickGate) {
    colony
        .zones
        .retain(|zone| zone.expires_at > gate.processed_through);

    if gate.minute_rolled {
        prune_events_to_newest(colony, EVENT_KEEP);
    }
}

/// Phase 11: decay path wear while preserving built roads and explored trail
/// thresholds.
fn phase_11_path_wear_decay(colony: &mut ColonyRuntime, gate: TickGate) {
    let decay_amount = ((gate.elapsed_sec as f64 * normalize_time_scale(colony)) / 60.0)
        .floor()
        .clamp(0.0, f64::from(MAX_PATH_DECAY_PER_TICK)) as u32;
    if decay_amount == 0 {
        return;
    }

    for tile in colony.world_tiles.values_mut() {
        if tile.path_wear == 0 || tile.overlay_feature.as_deref() == Some("road_built") {
            continue;
        }

        if tile.path_wear >= 70 {
            tile.path_wear = tile.path_wear.saturating_sub(decay_amount).max(63);
        } else if tile.path_wear > 62 {
            continue;
        } else {
            tile.path_wear = tile.path_wear.saturating_sub(decay_amount).max(1);
        }
    }
}

/// Phase 12: regrow depleted non-forest food resources once per minute.
fn phase_12_resource_regrowth(colony: &mut ColonyRuntime, gate: TickGate) {
    if !gate.minute_rolled {
        return;
    }

    let amount = regrowth_amount(gate.elapsed_sec as f64 * normalize_time_scale(colony)).floor();
    if amount <= 0.0 {
        return;
    }
    let amount = amount as u32;

    for tile in colony.world_tiles.values_mut() {
        if tile.last_depleted <= 0 || is_forest_type(tile.tile_type) {
            continue;
        }
        tile.resources.food = tile
            .resources
            .food
            .saturating_add(amount)
            .min(tile.max_resources.food);
    }
}

/// Phase 13: build tick-local target caches and movement RNG helpers for food,
/// quarry, water, frontier, zones, and hunt-site draining.
fn phase_13_tick_local_target_caches(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 14: promote queued jobs, create scaffolds for construction jobs, and
/// stamp started timers.
fn phase_14_promote_queued_jobs_and_break_ground(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 15: assign destinations for promoted jobs, including zoned hunt picks,
/// quarry/water/frontier targets, expansion targets, and shrine travel.
fn phase_15_assign_promoted_job_destinations(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 16: update active scaffold progress from job timer progress.
fn phase_16_active_scaffold_progress(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 17: queue the legacy leader emergency hunt when food is below the
/// policy threshold and no conflicting strategic job exists.
fn phase_17_legacy_emergency_hunt(colony: &mut ColonyRuntime, gate: TickGate, policy: TickPolicy) {
    if colony.resources.food >= policy.config.food_emergency_threshold {
        return;
    }
    if has_conflicting_active_job(colony, JobKind::LeaderPlanHunt) {
        return;
    }
    if !can_take_policy_action(colony, policy) {
        return;
    }
    let Some(cat_id) = select_best_cat(colony, Some(CatSpecialization::Hunter)) else {
        return;
    };
    queue_job(
        colony,
        gate.processed_through,
        JobKind::LeaderPlanHunt,
        Some(cat_id),
        JobMetadata::None,
    );
}

/// Phase 18: assemble the leader snapshot: workforce, housing/storage pressure,
/// jobs, staffing gaps, warriors, threat, and starvation flags.
fn phase_18_leader_snapshot_assembly(colony: &mut ColonyRuntime, gate: TickGate) -> LeaderSnapshot {
    let caps = storage_caps(colony);
    let alive = alive_cats(&colony.cats).collect::<Vec<_>>();
    let active_jobs = active_or_queued_jobs(colony);
    let busy_ids = active_jobs
        .iter()
        .filter_map(|job| job.assigned_cat.as_deref())
        .collect::<Vec<_>>();
    let assigned_building_ids = colony
        .buildings
        .iter()
        .filter_map(|building| building.assigned_cat.as_deref())
        .collect::<Vec<_>>();

    let work_capable = alive
        .iter()
        .filter(|cat| can_work(get_life_stage(cat.age_hours)))
        .count() as u32;
    let idle_cats = alive
        .iter()
        .filter(|cat| {
            can_take_new_job_with_busy(cat, &busy_ids)
                && !assigned_building_ids.contains(&cat.id.as_str())
        })
        .count() as u32;
    let workforce = alive
        .iter()
        .map(|cat| crate::life_sim::workforce_weight(get_life_stage(cat.age_hours)))
        .sum::<f64>();

    let active_hunts = count_jobs(&active_jobs, JobKind::HuntExpedition);
    let active_quarries = count_jobs(&active_jobs, JobKind::Quarry);
    let active_scouts = count_jobs(&active_jobs, JobKind::Explore);
    let active_water_fetchers = count_jobs(&active_jobs, JobKind::FetchWater);
    let den_plans_in_flight = active_jobs
        .iter()
        .filter(|job| {
            job.kind == JobKind::LeaderPlanHouse
                || (job.kind == JobKind::BuildHouse
                    && job_building_type(job) == Some(BuildingType::Den))
        })
        .count() as u32;
    let storage_plans_in_flight = active_jobs
        .iter()
        .filter(|job| {
            job.kind == JobKind::BuildHouse
                && job_building_type(job) == Some(BuildingType::FoodStorage)
        })
        .count() as u32;

    let committed_capacity = colony
        .buildings
        .iter()
        .filter(|building| {
            building.building_type == BuildingType::Den && building.construction_progress < 100
        })
        .map(|building| 2 * building.level.max(1))
        .sum::<u32>()
        + 2 * active_jobs
            .iter()
            .filter(|job| {
                job.kind == JobKind::BuildHouse
                    && matches!(
                        job.metadata,
                        JobMetadata::Construction {
                            phase: ConstructionPhase::ConstructHouse,
                            building_type: BuildingType::Den,
                            ..
                        }
                    )
            })
            .count() as u32;

    let effects = resolve_effects(colony.upgrade_tree.owned_node_ids.iter());
    let housing_buildings = colony
        .buildings
        .iter()
        .map(|building| {
            crate::housing::HousingBuilding::new(
                building.building_type,
                f64::from(building.level),
                f64::from(building.construction_progress),
            )
        })
        .collect::<Vec<_>>();
    let storage_buildings = storage_buildings(colony);
    let population = alive.len() as u32;
    let food_drain = consumption_for_tick(
        population as f64,
        gate.elapsed_sec as f64 * normalize_resource_decay_multiplier(colony),
        idle_engine_upgrade_levels(&colony.upgrade_levels),
    );
    let current_threat_band = match threat_band(colony.threat_pressure) {
        crate::threat::ThreatBand::Calm => crate::leader_ai::ThreatBand::Calm,
        crate::threat::ThreatBand::Rising => crate::leader_ai::ThreatBand::Rising,
        crate::threat::ThreatBand::Imminent => crate::leader_ai::ThreatBand::Imminent,
    };
    let starving = caps.food > 0.0 && colony.resources.food / caps.food < 0.15;

    LeaderSnapshot {
        population,
        workforce: Some(workforce),
        idle_cats,
        employed_cats: work_capable.saturating_sub(idle_cats),
        resources: LeaderResources {
            food: colony.resources.food,
            refined: colony.resources.refined,
        },
        food_capacity: caps.food,
        food_drain_per_tick: Some(food_drain.food_use),
        materials: colony.resources.materials,
        materials_capacity: caps.materials,
        water: colony.resources.water,
        water_capacity: caps.water,
        water_drain_per_tick: Some(food_drain.water_use),
        housing: LeaderHousing {
            capacity: crate::housing::housing_capacity(&housing_buildings, effects.housing_per_den)
                as u32,
            committed: committed_capacity,
        },
        active_hunts,
        active_quarries,
        active_scouts,
        active_water_fetchers,
        has_quarry_site: has_quarry_site(colony),
        has_water_site: has_water_site(colony),
        has_frontier: has_frontier(colony),
        den_plans_in_flight,
        storage_plans_in_flight,
        storehouse_count: count_storehouses(&storage_buildings),
        storehouse_cap: storehouse_cap(population),
        workshops_needing_workers: buildings_needing_workers(colony, BuildingType::Workshop).len()
            as u32,
        research_huts_needing_workers: Some(0),
        smithies_needing_workers: Some(
            buildings_needing_workers(colony, BuildingType::Smithy).len() as u32,
        ),
        has_barracks: Some(has_complete_building(colony, BuildingType::Barracks)),
        warrior_count: Some(
            alive
                .iter()
                .filter(|cat| cat.specialization == Some(CatSpecialization::Warrior))
                .count() as u32,
        ),
        training_in_flight: Some(count_jobs(&active_jobs, JobKind::TrainWarrior)),
        threat_band: Some(current_threat_band),
        starving: Some(starving),
    }
}

/// Phase 19: execute leader cancellation decisions before spending labor.
fn phase_19_leader_cancellations(
    colony: &mut ColonyRuntime,
    gate: TickGate,
    snapshot: &LeaderSnapshot,
) -> DirectorPlan {
    if snapshot.population == 0 {
        return DirectorPlan {
            decisions: Vec::new(),
            slots: Vec::new(),
        };
    }

    let plan = direct_colony(snapshot);

    for decision in &plan.decisions {
        match decision {
            LeaderDecision::CancelHunts => {
                let cancelled = cancel_jobs(
                    colony,
                    gate.processed_through,
                    JobKind::HuntExpedition,
                    true,
                );
                if cancelled > 0 {
                    append_event(
                        colony,
                        gate.processed_through,
                        EventKind::Other("job_cancelled".to_owned()),
                        format!(
                            "The leader called off {cancelled} hunt{} - the stores are overflowing.",
                            if cancelled == 1 { "" } else { "s" }
                        ),
                    );
                }
            }
            LeaderDecision::CancelTraining => {
                let cancelled =
                    cancel_jobs(colony, gate.processed_through, JobKind::TrainWarrior, false);
                if cancelled > 0 {
                    append_event(
                        colony,
                        gate.processed_through,
                        EventKind::Other("job_cancelled".to_owned()),
                        format!(
                            "The leader called {cancelled} recruit{} back from the barracks - the larder is bare.",
                            if cancelled == 1 { "" } else { "s" }
                        ),
                    );
                }
            }
            _ => {}
        }
    }

    plan
}

/// Phase 20: match idle cats to leader labor slots and staff production,
/// research, smithy, expedition, and training work.
fn phase_20_leader_labor_assignments_and_staffing(
    colony: &mut ColonyRuntime,
    gate: TickGate,
    policy: TickPolicy,
    plan: &DirectorPlan,
) {
    let busy_ids = active_or_queued_jobs(colony)
        .iter()
        .filter_map(|job| job.assigned_cat.as_deref())
        .collect::<Vec<_>>();
    let assigned_building_ids = colony
        .buildings
        .iter()
        .filter_map(|building| building.assigned_cat.as_deref())
        .collect::<Vec<_>>();
    let available_idle = colony
        .cats
        .iter()
        .filter(|cat| {
            can_take_new_job_with_busy(cat, &busy_ids)
                && !assigned_building_ids.contains(&cat.id.as_str())
        })
        .map(cat_brief)
        .collect::<Vec<_>>();

    let assignments = match_cats_to_slots(
        &plan.slots,
        &available_idle,
        MatchOptions {
            exclude_warriors_from_training: true,
        },
    );
    let mut workshop_queue = buildings_needing_workers(colony, BuildingType::Workshop);
    let mut smithy_queue = buildings_needing_workers(colony, BuildingType::Smithy);

    for assignment in assignments {
        if !can_take_policy_action(colony, policy) {
            continue;
        }

        match assignment.goal {
            LaborGoalKind::Hunt => {
                queue_job(
                    colony,
                    gate.processed_through,
                    JobKind::HuntExpedition,
                    Some(assignment.cat_id),
                    JobMetadata::None,
                );
            }
            LaborGoalKind::FetchWater => {
                queue_job(
                    colony,
                    gate.processed_through,
                    JobKind::FetchWater,
                    Some(assignment.cat_id),
                    JobMetadata::None,
                );
            }
            LaborGoalKind::Quarry => {
                queue_job(
                    colony,
                    gate.processed_through,
                    JobKind::Quarry,
                    Some(assignment.cat_id),
                    JobMetadata::None,
                );
            }
            LaborGoalKind::Scout => {
                queue_job(
                    colony,
                    gate.processed_through,
                    JobKind::Explore,
                    Some(assignment.cat_id),
                    JobMetadata::None,
                );
            }
            LaborGoalKind::TrainWarrior => {
                queue_job(
                    colony,
                    gate.processed_through,
                    JobKind::TrainWarrior,
                    Some(assignment.cat_id),
                    JobMetadata::None,
                );
            }
            LaborGoalKind::AssignWorkshop => {
                if let Some(building_id) = workshop_queue.pop() {
                    staff_building(
                        colony,
                        &building_id,
                        &assignment.cat_id,
                        gate.processed_through,
                    );
                }
            }
            LaborGoalKind::AssignResearch => {}
            LaborGoalKind::AssignSmithy => {
                if let Some(building_id) = smithy_queue.pop() {
                    staff_building(
                        colony,
                        &building_id,
                        &assignment.cat_id,
                        gate.processed_through,
                    );
                }
            }
        }
    }
}

/// Phase 21: execute leader capital decisions and minute-cadence tithe deposits.
fn phase_21_leader_capital_decisions_and_tithe(
    colony: &mut ColonyRuntime,
    gate: TickGate,
    policy: TickPolicy,
    plan: &DirectorPlan,
) {
    for decision in &plan.decisions {
        match *decision {
            LeaderDecision::BuildStorage => {
                if can_take_policy_action(colony, policy) {
                    let architect = select_best_cat(colony, Some(CatSpecialization::Architect));
                    if let Some(cat_id) = architect {
                        queue_job(
                            colony,
                            gate.processed_through,
                            JobKind::BuildHouse,
                            Some(cat_id),
                            JobMetadata::Construction {
                                phase: ConstructionPhase::ConstructHouse,
                                building_type: BuildingType::FoodStorage,
                                building_id: None,
                                site: None,
                            },
                        );
                    }
                }
            }
            LeaderDecision::BuildDen => {
                if can_take_policy_action(colony, policy) {
                    let architect = select_best_cat(colony, Some(CatSpecialization::Architect));
                    queue_job(
                        colony,
                        gate.processed_through,
                        JobKind::LeaderPlanHouse,
                        architect,
                        JobMetadata::None,
                    );
                }
            }
            LeaderDecision::Tithe {
                food,
                refined,
                blessings,
            } => {
                if !gate.minute_rolled {
                    continue;
                }
                colony.resources.food -= f64::from(food);
                colony.resources.refined -= f64::from(refined);
                colony.global_upgrade_points += f64::from(blessings);
                append_event(
                    colony,
                    gate.processed_through,
                    EventKind::Other("shrine_deposit".to_owned()),
                    format!(
                        "The leader offered surplus stores to the gods (+{blessings} blessing{}).",
                        if blessings == 1 { "" } else { "s" }
                    ),
                );
            }
            _ => {}
        }
    }
}

/// Phase 22: approve requested rituals when resources and policy reliability
/// allow.
fn phase_22_ritual_approval(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 23: run fields, workshops, and smithies against patched resources and
/// building progress.
fn phase_23_production(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 24: accrue research from staffed research huts/schools and auto-unlock
/// affordable upgrade-tree nodes.
fn phase_24_research(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 25: apply survival needs, deaths, carried-yield salvage, and
/// death-related job retirement.
fn phase_25_survival_deaths_and_carried_yield_salvage(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 26: reset empty colonies and short-circuit the remaining phases.
fn phase_26_empty_colony_reset(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 27: collect due active jobs and preserve the phase-14 queued snapshot
/// needed by completion parity.
fn phase_27_due_job_prelude(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 28: complete supply jobs and planner jobs, including hunt and house
/// queueing.
fn phase_28_due_completion_supplies_and_planner_jobs(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 29: complete hunt/quarry/water/explore/expansion jobs, including tile
/// depletion and claimed-area mutation.
fn phase_29_due_completion_gathering_explore_expansion(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 30: complete build/ritual/training jobs, return workers, and mark jobs
/// completed.
fn phase_30_due_completion_build_ritual_training_return_mark_done(
    _: &mut ColonyRuntime,
    _: TickGate,
) {
}

/// Phase 31: run mid-job hauling trips for accepted active gathering and fetch
/// jobs.
fn phase_31_mid_job_hauling(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 32: prepare movement inputs and optionally queue village expansion.
fn phase_32_movement_setup_and_village_expansion_queue(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 33: deposit carried resources, clear missing destinations, and pick
/// idle wander targets.
fn phase_33_movement_deposits_and_no_destination_wander(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 34: move cats, accept jobs on shrine arrival, reveal tiles, and apply
/// path wear.
fn phase_34_movement_travel_job_acceptance_reveal_path_wear(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 35: pave deliberate road corridors once per minute while preserving the
/// materials reserve.
fn phase_35_deliberate_roads(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 36: run threat pressure, raid spawning/marching/combat, loot, and
/// raid-wipeout reset checks.
fn phase_36_threat_and_raid_director(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 37: clamp resources, handle critical collapse, update status, persist
/// final state, and record `last_tick = processed_through`.
fn phase_37_final_clamp_critical_collapse_status_persist(
    colony: &mut ColonyRuntime,
    gate: TickGate,
) {
    let caps = storage_caps(colony);
    clamp_resources_to_caps(&mut colony.resources, caps);
    colony.status = crate::idle_rules::next_colony_status(&colony.resources);
    colony.last_tick = gate.processed_through;
}

fn next_base_roll(colony: &mut ColonyRuntime) -> f64 {
    let seed = colony.test_rng_seed.unwrap_or(1);
    let roll = roll_seeded(f64::from(seed));
    colony.test_rng_seed = Some(roll.next_seed);
    roll.value
}

fn alive_cats(cats: &[Cat]) -> impl Iterator<Item = &Cat> {
    cats.iter().filter(|cat| cat.death_time.is_none())
}

fn elapsed_game_hours(colony: &ColonyRuntime, gate: TickGate) -> f64 {
    (gate.elapsed_sec as f64 * normalize_time_scale(colony)) / 3600.0
}

fn normalize_time_scale(colony: &ColonyRuntime) -> f64 {
    if colony.test_time_scale.is_finite() {
        colony.test_time_scale.max(1.0)
    } else {
        1.0
    }
}

fn normalize_resource_decay_multiplier(colony: &ColonyRuntime) -> f64 {
    if colony.test_resource_decay_multiplier.is_finite() {
        colony.test_resource_decay_multiplier.max(1.0)
    } else {
        1.0
    }
}

fn idle_engine_upgrade_levels(upgrades: &UpgradeLevels) -> idle_engine::UpgradeLevels {
    idle_engine::UpgradeLevels {
        click_power: f64::from(upgrades.click_power),
        supply_speed: f64::from(upgrades.supply_speed),
        hunt_mastery: f64::from(upgrades.hunt_mastery),
        build_mastery: f64::from(upgrades.build_mastery),
        ritual_mastery: f64::from(upgrades.ritual_mastery),
        resilience: f64::from(upgrades.resilience),
    }
}

fn storage_caps(colony: &ColonyRuntime) -> StorageCapacities {
    let buildings: Vec<StorageBuilding> = colony
        .buildings
        .iter()
        .map(|building| {
            StorageBuilding::new(
                building.building_type,
                f64::from(building.construction_progress),
                Some(f64::from(building.level)),
            )
        })
        .collect();
    let effects = resolve_effects(colony.upgrade_tree.owned_node_ids.iter());

    storage_capacities(&buildings, effects.storage_per_level_mult)
}

fn clamp_resources_to_caps(resources: &mut Resources, caps: StorageCapacities) {
    resources.food = clamp_resource(resources.food, caps.food);
    resources.water = clamp_resource(resources.water, caps.water);
    resources.herbs = clamp_resource(resources.herbs, caps.herbs);
    resources.materials = clamp_resource(resources.materials, caps.materials);
    resources.refined = clamp_resource(resources.refined, caps.refined);
    resources.weapons = clamp_resource(resources.weapons, caps.weapons);
    resources.armor = clamp_resource(resources.armor, caps.armor);
}

fn clamp_resource(value: f64, cap: f64) -> f64 {
    value.max(0.0).min(cap)
}

fn append_event(
    colony: &mut ColonyRuntime,
    at_ms: i64,
    kind: EventKind,
    message: impl Into<String>,
) {
    colony.events.push(EventLog {
        id: format!("event-{}-{}", at_ms, colony.events.len() + 1),
        at_ms,
        kind,
        message: message.into(),
    });
}

fn can_take_policy_action(colony: &mut ColonyRuntime, policy: TickPolicy) -> bool {
    next_base_roll(colony) <= policy.config.action_reliability
}

fn active_or_queued_jobs(colony: &ColonyRuntime) -> Vec<&JobRuntime> {
    colony
        .jobs
        .iter()
        .filter(|job| matches!(job.status, JobStatus::Active | JobStatus::Queued))
        .collect()
}

fn count_jobs(jobs: &[&JobRuntime], kind: JobKind) -> u32 {
    jobs.iter().filter(|job| job.kind == kind).count() as u32
}

fn has_conflicting_active_job(colony: &ColonyRuntime, kind: JobKind) -> bool {
    active_or_queued_jobs(colony).iter().any(|job| match kind {
        JobKind::LeaderPlanHunt => {
            matches!(job.kind, JobKind::LeaderPlanHunt | JobKind::HuntExpedition)
        }
        JobKind::LeaderPlanHouse => {
            matches!(job.kind, JobKind::LeaderPlanHouse | JobKind::BuildHouse)
        }
        _ => job.kind == kind,
    })
}

fn job_building_type(job: &JobRuntime) -> Option<BuildingType> {
    match job.metadata {
        JobMetadata::Construction { building_type, .. } => Some(building_type),
        _ => None,
    }
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

fn can_take_new_job_with_busy(cat: &Cat, busy_ids: &[&str]) -> bool {
    cat.death_time.is_none()
        && can_work(get_life_stage(cat.age_hours))
        && !busy_ids.contains(&cat.id.as_str())
        && cat.activity == CatActivity::Idle
        && cat.current_task.is_none()
        && cat.carrying.is_none()
        && cat.destination.is_none()
}

fn select_best_cat(
    colony: &ColonyRuntime,
    specialization: Option<CatSpecialization>,
) -> Option<CatId> {
    let busy_ids = active_or_queued_jobs(colony)
        .iter()
        .filter_map(|job| job.assigned_cat.as_deref())
        .collect::<Vec<_>>();
    let assigned_building_ids = colony
        .buildings
        .iter()
        .filter_map(|building| building.assigned_cat.as_deref())
        .collect::<Vec<_>>();
    let available = colony
        .cats
        .iter()
        .filter(|cat| {
            can_take_new_job_with_busy(cat, &busy_ids)
                && !assigned_building_ids.contains(&cat.id.as_str())
        })
        .collect::<Vec<_>>();

    let preferred = available
        .iter()
        .copied()
        .filter(|cat| cat.specialization == specialization && specialization.is_some())
        .collect::<Vec<_>>();
    let pool = if preferred.is_empty() {
        available
    } else {
        preferred
    };

    let mut best: Option<&Cat> = None;
    for cat in pool {
        if best.is_none_or(|current| {
            specialization_stat(cat, specialization) > specialization_stat(current, specialization)
        }) {
            best = Some(cat);
        }
    }
    best.map(|cat| cat.id.clone())
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

fn queue_job(
    colony: &mut ColonyRuntime,
    now_ms: i64,
    kind: JobKind,
    assigned_cat: Option<CatId>,
    metadata: JobMetadata,
) {
    let specialization = assigned_cat
        .as_ref()
        .and_then(|cat_id| colony.cats.iter().find(|cat| cat.id == *cat_id))
        .and_then(|cat| cat.specialization);
    let duration_seconds = idle_engine::get_scaled_duration_seconds(
        kind,
        specialization,
        idle_engine_upgrade_levels(&colony.upgrade_levels),
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
        requested_by: JobRequester::Leader,
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

fn task_for_job(kind: JobKind) -> Option<TaskType> {
    match kind {
        JobKind::HuntExpedition | JobKind::LeaderPlanHunt => Some(TaskType::Hunt),
        JobKind::FetchWater => Some(TaskType::FetchWater),
        JobKind::Quarry | JobKind::BuildHouse | JobKind::LeaderPlanHouse => Some(TaskType::Build),
        JobKind::Explore | JobKind::ExpandVillage => Some(TaskType::Explore),
        JobKind::TrainWarrior => Some(TaskType::Patrol),
        JobKind::Ritual => Some(TaskType::Rest),
        JobKind::SupplyFood | JobKind::SupplyWater => None,
    }
}

fn cat_brief(cat: &Cat) -> CatBrief {
    CatBrief {
        id: cat.id.clone(),
        specialization: cat.specialization,
        stats: CatBriefStats {
            hunting: cat.stats.hunting,
            building: cat.stats.building,
            vision: cat.stats.vision,
            medicine: cat.stats.medicine,
            attack: cat.stats.attack,
            defense: cat.stats.defense,
            leadership: cat.stats.leadership,
        },
    }
}

fn buildings_needing_workers(
    colony: &ColonyRuntime,
    building_type: BuildingType,
) -> Vec<BuildingId> {
    colony
        .buildings
        .iter()
        .filter(|building| {
            building.building_type == building_type
                && building.construction_progress >= 100
                && building.assigned_cat.is_none()
        })
        .map(|building| building.id.clone())
        .collect()
}

fn staff_building(colony: &mut ColonyRuntime, building_id: &str, cat_id: &str, now_ms: i64) {
    if let Some(building) = colony
        .buildings
        .iter_mut()
        .find(|building| building.id == building_id)
    {
        building.assigned_cat = Some(cat_id.to_owned());
    }
    append_event(
        colony,
        now_ms,
        EventKind::Other("worker_assigned".to_owned()),
        "The leader assigned a worker.",
    );
}

fn cancel_jobs(
    colony: &mut ColonyRuntime,
    now_ms: i64,
    kind: JobKind,
    return_to_shrine: bool,
) -> usize {
    let mut assigned = Vec::new();
    let mut cancelled = 0;
    for job in &mut colony.jobs {
        if job.kind == kind && job.status == JobStatus::Active {
            job.status = JobStatus::Cancelled;
            job.completed_at = Some(now_ms);
            if let Some(cat_id) = &job.assigned_cat {
                assigned.push(cat_id.clone());
            }
            cancelled += 1;
        }
    }

    for cat_id in assigned {
        if let Some(cat) = colony.cats.iter_mut().find(|cat| cat.id == cat_id) {
            cat.current_task = None;
            if return_to_shrine {
                cat.activity = CatActivity::Returning;
                cat.destination = Some(Position {
                    map: crate::entities::MapType::World,
                    x: 0.0,
                    y: 0.0,
                });
            } else {
                cat.activity = CatActivity::Idle;
            }
        }
    }

    cancelled
}

fn has_complete_building(colony: &ColonyRuntime, building_type: BuildingType) -> bool {
    colony.buildings.iter().any(|building| {
        building.building_type == building_type && building.construction_progress >= 100
    })
}

fn has_quarry_site(colony: &ColonyRuntime) -> bool {
    colony.world_tiles.values().any(|tile| {
        matches!(tile.tile_type, TileType::Mountains | TileType::CaveEntrance)
            && tile.path_wear > 62
    })
}

fn has_water_site(colony: &ColonyRuntime) -> bool {
    colony
        .world_tiles
        .values()
        .any(|tile| tile.resources.water > 0 && tile.path_wear > 62)
}

fn has_frontier(colony: &ColonyRuntime) -> bool {
    colony.world_tiles.values().any(|tile| tile.path_wear <= 62)
}

fn is_open_leadership_election(election: &ElectionRuntime) -> bool {
    matches!(election.kind, ElectionKind::Scheduled | ElectionKind::Snap)
        && election.resolved_at.is_none()
}

fn current_election_candidates(colony: &ColonyRuntime) -> Vec<ElectionCandidate> {
    let candidates = alive_cats(&colony.cats)
        .map(|cat| ElectionCandidate {
            id: cat.id.clone(),
            leadership: cat.stats.leadership,
        })
        .collect::<Vec<_>>();
    let candidate_ids = candidates_for_unbarred(&candidates);

    candidate_ids
        .iter()
        .filter_map(|id| candidates.iter().find(|candidate| candidate.id == *id))
        .cloned()
        .collect()
}

fn ballots_for(colony: &ColonyRuntime, election_id: &str) -> Vec<BallotVote> {
    colony
        .votes
        .iter()
        .filter(|vote| vote.election_id == election_id)
        .map(|vote| BallotVote {
            player_id: vote.voter_id.clone(),
            cat_id: vote.cat_id.clone(),
        })
        .collect()
}

fn vote_kick_target(ballots: &[BallotVote]) -> Option<CatId> {
    ballots.first().map(|ballot| ballot.cat_id.clone())
}

fn choose_interim_leader_excluding(
    colony: &ColonyRuntime,
    excluded_cat_id: Option<&str>,
) -> Option<CatId> {
    let mut best_leader: Option<&Cat> = None;
    for candidate in alive_cats(&colony.cats) {
        if excluded_cat_id == Some(candidate.id.as_str()) {
            continue;
        }
        if best_leader.is_none_or(|best| candidate.stats.leadership > best.stats.leadership) {
            best_leader = Some(candidate);
        }
    }
    best_leader.map(|cat| cat.id.clone())
}

fn last_resolved_leadership_election_at(colony: &ColonyRuntime) -> Option<i64> {
    colony
        .elections
        .iter()
        .filter(|election| {
            matches!(election.kind, ElectionKind::Scheduled | ElectionKind::Snap)
                && election.resolved_at.is_some()
        })
        .map(|election| election.closes_at)
        .max()
}

fn scaled_term_ms(colony: &ColonyRuntime) -> f64 {
    (TERM_MS / normalize_time_scale(colony)).max(10_000.0)
}

fn scaled_election_window_ms(colony: &ColonyRuntime, kind: ElectionKind) -> i64 {
    let base = match kind {
        ElectionKind::Scheduled | ElectionKind::Snap => ELECTION_WINDOW_MS,
        ElectionKind::VoteKick => KICK_WINDOW_MS,
    };
    (base / normalize_time_scale(colony)).max(5_000.0) as i64
}

fn open_leadership_election(colony: &mut ColonyRuntime, now_ms: i64, kind: ElectionKind) {
    if current_election_candidates(colony).is_empty() {
        return;
    }

    let id = format!(
        "{}-{}-{}",
        match kind {
            ElectionKind::Scheduled => "election",
            ElectionKind::Snap => "snap-election",
            ElectionKind::VoteKick => "vote-kick",
        },
        now_ms,
        colony.elections.len() + 1
    );
    colony.elections.push(ElectionRuntime {
        id,
        opened_at: now_ms,
        closes_at: now_ms + scaled_election_window_ms(colony, kind),
        resolved_at: None,
        winner_cat_id: None,
        kind,
    });
    append_event(
        colony,
        now_ms,
        EventKind::Election,
        "The colony is holding a leadership election - cast your vote!",
    );
}

fn prune_events_to_newest(colony: &mut ColonyRuntime, keep: usize) {
    if colony.events.len() <= keep {
        return;
    }

    let mut by_newest = colony
        .events
        .iter()
        .enumerate()
        .map(|(index, event)| (index, event.at_ms))
        .collect::<Vec<_>>();
    by_newest.sort_by(|(left_index, left_at), (right_index, right_at)| {
        right_at
            .cmp(left_at)
            .then_with(|| right_index.cmp(left_index))
    });

    let mut keep_indices = by_newest
        .into_iter()
        .take(keep)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    keep_indices.sort_unstable();

    let mut next_events = Vec::with_capacity(keep);
    for index in keep_indices {
        next_events.push(colony.events[index].clone());
    }
    colony.events = next_events;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        entities::{CatActivity, CatNeeds, CatStats, MapType},
        storage::BASE_CAPACITY,
    };

    #[test]
    fn empty_world_returns_empty_reports() {
        let mut world = WorldState {
            world_seed: 123,
            colonies: Vec::new(),
        };

        assert_eq!(world_tick(&mut world, 10_000), Vec::new());
    }

    #[test]
    fn sub_second_elapsed_skips_and_leaves_last_tick_unchanged() {
        let mut world = WorldState {
            world_seed: 123,
            colonies: vec![ColonyRuntime {
                id: "colony-1".to_owned(),
                last_tick: 9_001,
                ..ColonyRuntime::default()
            }],
        };

        let reports = world_tick(&mut world, 10_000);

        assert_eq!(
            reports,
            vec![TickReport {
                colony_id: "colony-1".to_owned(),
                skipped: true,
                reset_reason: None,
            }]
        );
        assert_eq!(world.colonies[0].last_tick, 9_001);
    }

    #[test]
    fn single_adult_idle_cat_consumes_spoils_and_persists_tick_and_seed() {
        let mut world = WorldState {
            world_seed: 123,
            colonies: vec![ColonyRuntime {
                id: "colony-1".to_owned(),
                name: "MossClan".to_owned(),
                resources: Resources {
                    food: 100.0,
                    water: 100.0,
                    herbs: 16.0,
                    materials: 0.0,
                    refined: 0.0,
                    weapons: 0.0,
                    armor: 0.0,
                    blessings: 0.0,
                },
                cats: vec![adult_idle_cat("cat-1", "colony-1")],
                last_tick: 1_000,
                test_rng_seed: Some(12_345),
                ..ColonyRuntime::default()
            }],
        };

        let reports = world_tick(&mut world, 61_000);

        assert_eq!(
            reports,
            vec![TickReport {
                colony_id: "colony-1".to_owned(),
                skipped: false,
                reset_reason: None,
            }]
        );

        let colony = &world.colonies[0];
        let consumption = consumption_for_tick(1.0, 60.0, idle_engine::UpgradeLevels::default());
        let expected_food = apply_food_spoilage_after_consumption(
            100.0,
            consumption.food_use,
            BASE_CAPACITY.food,
            60.0,
        );
        let expected_water = 100.0 - consumption.water_use;

        assert_eq!(colony.resources.food.to_bits(), expected_food.to_bits());
        assert_eq!(colony.resources.water.to_bits(), expected_water.to_bits());
        assert_eq!(colony.resources.herbs, 16.0);
        assert_eq!(colony.status, ColonyStatus::Thriving);
        assert_eq!(colony.last_tick, 61_000);
        assert_eq!(colony.test_rng_seed, Some(2_332_836_374));
        let expected_age: f64 = 24.0 + 60.0 / 3600.0;
        assert_eq!(colony.cats[0].age_hours.to_bits(), expected_age.to_bits());
        assert_eq!(colony.cats[0].death_time, None);
    }

    #[test]
    fn leader_assigns_water_fetchers_over_hunts_when_water_projection_is_worse() {
        let mut cats = vec![
            adult_idle_cat("leader", "colony-1"),
            adult_idle_cat("hunter", "colony-1"),
            adult_idle_cat("builder", "colony-1"),
        ];
        cats[0].stats.leadership = 80.0;
        cats[1].stats.hunting = 90.0;
        cats[2].stats.building = 80.0;

        let mut world = WorldState {
            world_seed: 123,
            colonies: vec![ColonyRuntime {
                id: "colony-1".to_owned(),
                name: "MossClan".to_owned(),
                leader_id: Some("leader".to_owned()),
                resources: Resources {
                    food: 30.0,
                    water: 1.0,
                    herbs: 16.0,
                    materials: 24.0,
                    refined: 0.0,
                    weapons: 0.0,
                    armor: 0.0,
                    blessings: 0.0,
                },
                cats,
                buildings: vec![BuildingRuntime {
                    id: "shrine".to_owned(),
                    building_type: BuildingType::Shrine,
                    level: 1,
                    position: pos(0, 0),
                    is_complete: true,
                    construction_progress: 100,
                    production_progress: 0.0,
                    assigned_cat: None,
                }],
                world_tiles: BTreeMap::from([(
                    pos(3, 0),
                    WorldTileRuntime {
                        pos: pos(3, 0),
                        tile_type: TileType::River,
                        resources: TileResources {
                            food: 0,
                            herbs: 0,
                            water: 100,
                        },
                        max_resources: MaxResources { food: 0, herbs: 0 },
                        danger_level: 0.0,
                        path_wear: 63,
                        last_depleted: 0,
                        overlay_feature: None,
                    },
                )]),
                last_tick: 1_000,
                test_rng_seed: Some(12_345),
                ..ColonyRuntime::default()
            }],
        };

        let _ = world_tick(&mut world, 61_000);

        let colony = &world.colonies[0];
        let water_jobs = colony
            .jobs
            .iter()
            .filter(|job| job.kind == JobKind::FetchWater)
            .collect::<Vec<_>>();
        assert_eq!(water_jobs.len(), 3);
        assert!(
            colony
                .jobs
                .iter()
                .all(|job| job.kind != JobKind::HuntExpedition)
        );
        assert_eq!(water_jobs[0].assigned_cat.as_deref(), Some("hunter"));
        assert_eq!(colony.cats[1].current_task, Some(TaskType::FetchWater));
    }

    #[test]
    fn tick_sweeps_expired_zones_by_processed_time() {
        let mut world = WorldState {
            world_seed: 123,
            colonies: vec![ColonyRuntime {
                id: "colony-1".to_owned(),
                zones: vec![
                    zone(1, 59_999),
                    zone(2, 60_000),
                    zone(3, 60_001),
                    zone(4, 120_000),
                ],
                last_tick: 0,
                ..ColonyRuntime::default()
            }],
        };

        let reports = world_tick(&mut world, 60_999);

        assert!(!reports[0].skipped);
        assert_eq!(
            world.colonies[0]
                .zones
                .iter()
                .map(|zone| zone.created_at)
                .collect::<Vec<_>>(),
            vec![3, 4]
        );
    }

    #[test]
    fn event_log_prune_keeps_newest_two_thousand_once_per_minute() {
        let events = (0..2_005)
            .map(|index| EventLog {
                id: format!("event-{index}"),
                at_ms: index,
                kind: EventKind::Other("test".to_owned()),
                message: format!("event {index}"),
            })
            .collect();
        let mut world = WorldState {
            world_seed: 123,
            colonies: vec![ColonyRuntime {
                id: "colony-1".to_owned(),
                events,
                last_tick: 59_000,
                ..ColonyRuntime::default()
            }],
        };

        let _ = world_tick(&mut world, 60_000);

        let events = &world.colonies[0].events;
        assert_eq!(events.len(), 2_000);
        assert_eq!(events.first().map(|event| event.at_ms), Some(5));
        assert_eq!(events.last().map(|event| event.at_ms), Some(2_004));
    }

    #[test]
    fn path_decay_is_clamped_and_preserves_roads_and_revealed_tiles() {
        let mut world = WorldState {
            world_seed: 123,
            colonies: vec![ColonyRuntime {
                id: "colony-1".to_owned(),
                world_tiles: BTreeMap::from([
                    (pos(0, 0), tile(0, 0, 80, None)),
                    (pos(1, 0), tile(1, 0, 80, Some("road_built"))),
                    (pos(2, 0), tile(2, 0, 64, None)),
                    (pos(3, 0), tile(3, 0, 2, None)),
                    (pos(4, 0), tile(4, 0, 0, None)),
                    (pos(5, 0), tile(5, 0, 70, None)),
                ]),
                last_tick: 0,
                ..ColonyRuntime::default()
            }],
        };

        let _ = world_tick(&mut world, 10 * 60 * 1000);

        let tiles = &world.colonies[0].world_tiles;
        assert_eq!(tiles[&pos(0, 0)].path_wear, 78);
        assert_eq!(tiles[&pos(1, 0)].path_wear, 80);
        assert_eq!(tiles[&pos(2, 0)].path_wear, 64);
        assert_eq!(tiles[&pos(3, 0)].path_wear, 1);
        assert_eq!(tiles[&pos(4, 0)].path_wear, 0);
        assert_eq!(tiles[&pos(5, 0)].path_wear, 68);
    }

    fn adult_idle_cat(id: &str, colony_id: &str) -> Cat {
        Cat {
            id: id.to_owned(),
            colony_id: colony_id.to_owned(),
            name: "Poppy".to_owned(),
            parent_ids: Vec::new(),
            birth_time: 0,
            death_time: None,
            stats: CatStats {
                attack: 10.0,
                defense: 10.0,
                hunting: 10.0,
                medicine: 10.0,
                cleaning: 10.0,
                building: 10.0,
                leadership: 50.0,
                vision: 10.0,
            },
            needs: CatNeeds {
                hunger: 100.0,
                thirst: 100.0,
                rest: 100.0,
                health: 100.0,
            },
            current_task: None,
            position: Position {
                map: MapType::Colony,
                x: 0.0,
                y: 0.0,
            },
            destination: None,
            carrying: None,
            activity: CatActivity::Idle,
            is_pregnant: false,
            pregnancy_due_time: None,
            age_hours: 24.0,
            pregnancy_due_age_hours: None,
            pregnancy_mate_id: None,
            sprite_params: None,
            specialization: None,
            role_xp: Default::default(),
        }
    }

    fn zone(created_at: i64, expires_at: i64) -> ZoneRuntime {
        ZoneRuntime {
            rect: ZoneRect {
                x1: 0,
                y1: 0,
                x2: 1,
                y2: 1,
            },
            kind: ZoneKind::Avoid,
            created_at,
            expires_at,
            player_id: None,
        }
    }

    fn pos(x: i32, y: i32) -> TilePos {
        TilePos { x, y }
    }

    fn tile(x: i32, y: i32, path_wear: u32, overlay_feature: Option<&str>) -> WorldTileRuntime {
        WorldTileRuntime {
            pos: pos(x, y),
            tile_type: TileType::Meadow,
            resources: TileResources {
                food: 0,
                herbs: 0,
                water: 0,
            },
            max_resources: MaxResources { food: 0, herbs: 0 },
            danger_level: 0.0,
            path_wear,
            last_depleted: 0,
            overlay_feature: overlay_feature.map(str::to_owned),
        }
    }
}
