//! Runtime world tick skeleton ported from `server/game.ts:workerTick`.
//!
//! This P7.1 module owns the in-memory runtime shapes and phase ordering. Later
//! P7 cards fill in the no-op phase bodies with the pure module calls.

use std::collections::BTreeMap;

use crate::{
    biomes::MaxResources,
    entities::{Cat, ColonyStatus, Position, Resources},
    types::{BuildingType, JobKind, JobStatus, PolicyTier, TileType, UpgradeKey},
    upgrade_tree::{UpgradeTreeState, create_upgrade_tree_state},
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
}

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
        phase_4_leader_bootstrap_and_policy(colony, gate);
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
        phase_17_legacy_emergency_hunt(colony, gate);
        phase_18_leader_snapshot_assembly(colony, gate);
        phase_19_leader_cancellations(colony, gate);
        phase_20_leader_labor_assignments_and_staffing(colony, gate);
        phase_21_leader_capital_decisions_and_tithe(colony, gate);
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
    })
}

/// Phase 2: load runtime config, legacy upgrade levels, upgrade tree state, and
/// resolved effects for this tick.
fn phase_2_runtime_upgrades_and_effects(_: &mut ColonyRuntime, gate: TickGate) {
    let _ = (gate.elapsed_sec, gate.processed_through);
}

/// Phase 3: initialize the base seeded RNG and derive movement, life, and raid
/// fork roots without persisting fork state.
fn phase_3_base_rng_and_fork_roots(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 4: choose or repair the leader, log leader changes, roll policy tier,
/// and compute policy action reliability.
fn phase_4_leader_bootstrap_and_policy(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 5: snapshot alive cats/buildings and compute initial storage caps.
fn phase_5_initial_roster_buildings_and_caps(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 6: age cats, process old-age deaths, leadership tenure, milestones,
/// births, conceptions, and death-related job cancellation.
fn phase_6_life_simulation(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 7: consume food/water, apply spoilage and resource caps, prepare
/// `nextResources`, and compute minute cadence.
fn phase_7_consumption_spoilage_resource_pre_patch_minute_cadence(
    _: &mut ColonyRuntime,
    _: TickGate,
) {
}

/// Phase 8: append the water crisis edge event when water crosses the low
/// threshold.
fn phase_8_water_low_crisis_edge(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 9: resolve due elections/vote-kicks and open scheduled or snap
/// elections.
fn phase_9_elections_lifecycle(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 10: expire zones and prune the event log to the newest retained events
/// on the minute cadence.
fn phase_10_zones_and_event_pruning(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 11: decay path wear while preserving built roads and explored trail
/// thresholds.
fn phase_11_path_wear_decay(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 12: regrow depleted non-forest food resources once per minute.
fn phase_12_resource_regrowth(_: &mut ColonyRuntime, _: TickGate) {}

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
fn phase_17_legacy_emergency_hunt(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 18: assemble the leader snapshot: workforce, housing/storage pressure,
/// jobs, staffing gaps, warriors, threat, and starvation flags.
fn phase_18_leader_snapshot_assembly(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 19: execute leader cancellation decisions before spending labor.
fn phase_19_leader_cancellations(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 20: match idle cats to leader labor slots and staff production,
/// research, smithy, expedition, and training work.
fn phase_20_leader_labor_assignments_and_staffing(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 21: execute leader capital decisions and minute-cadence tithe deposits.
fn phase_21_leader_capital_decisions_and_tithe(_: &mut ColonyRuntime, _: TickGate) {}

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
fn phase_37_final_clamp_critical_collapse_status_persist(_: &mut ColonyRuntime, _: TickGate) {
    let _ = PolicyTier::Normal;
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
