//! Runtime world tick skeleton ported from `server/game.ts:workerTick`.
//!
//! This P7.1 module owns the in-memory runtime shapes and phase ordering. Later
//! P7 cards fill in the no-op phase bodies with the pure module calls.

use std::collections::{BTreeMap, HashSet};

use crate::{
    biomes::MaxResources,
    depletion::{CHOPPED_FOREST_FOOD_CAP, is_forest_type, regrowth_amount},
    elections::{
        BallotVote, ELECTION_WINDOW_MS, ElectionCandidate, KICK_WINDOW_MS, TERM_MS,
        candidates_for_unbarred, election_due, election_winner, should_trigger_kick, tally_votes,
    },
    entities::{
        Carrying, CarryingKind, Cat, CatActivity, CatNeeds, CatStats, ColonyStatus, MapType,
        Position, Resources, RoleXp,
    },
    genetics::{RollSource, SeededRollSource, inherit_traits, traits_to_sprite_params},
    idle_engine,
    idle_rules::consumption_for_tick,
    leader_ai::{LeaderDecision, LeaderHousing, LeaderResources, LeaderSnapshot},
    leader_director::{
        CatBrief, CatBriefStats, DirectorPlan, LaborGoalKind, MatchOptions, direct_colony,
        match_cats_to_slots_with_officers,
    },
    life_sim::{can_work, get_life_stage, leadership_after_tenure, old_age_death_probability},
    movement::{
        EXPLORE_SPEED_FACTOR, JobDestinationContext, MOVE_SPEED_TILES_PER_SEC, WorldPos,
        destination_for_job, pick_wander_target, walk_path,
    },
    officers::OfficerRole,
    pathfinding::{
        self, ColonyGridParams, FindPathOptions, GatePlacement as PathGatePlacement,
        TilePos as PathTilePos, WalkOverlayFeature, WalkTile, WalkTileResources, WalkTileType,
        build_colony_walk_grid, find_path,
    },
    policy::PolicyConfig,
    production::{WorkshopOptions, advance_workshop, field_yield},
    rng::{life_seed, movement_seed, raid_seed, roll_seeded},
    roads::{RoadCorridorOptions, RoadTile, select_road_corridor},
    shrine::should_deposit,
    skills::{HAUL_SKILL_GAIN, Labor, SKILL_GAIN_PER_JOB},
    smithy::{SmithyOptions, advance_smithy},
    spoilage::apply_food_spoilage_after_consumption,
    storage::{
        StorageBuilding, StorageCapacities, count_storehouses, storage_capacities, storehouse_cap,
    },
    threat::{
        ThreatSnapshot, accrue_threat, colony_wealth, plan_raid, resolve_raid, should_spawn_raid,
        threat_band,
    },
    trips::{HUNT_TRIP_COUNT, remaining_yield, split_yield, trip_due_at},
    types::{BuildingType, CatSpecialization, JobKind, JobStatus, TaskType, TileType, UpgradeKey},
    upgrade_tree::{
        UpgradeTreeState, accrue_research, cat_auto_unlock, create_upgrade_tree_state, get_node,
        points_per_tick_for, resolve_effects,
    },
    village_area::{
        ExpandOptions, GatePlacement as AreaGatePlacement, Side, expand_village, from_tiles,
        gate_placement_default, is_inside_village, should_expand, side_delta,
    },
    village_layout::{
        GridPos, SHRINE_LOCAL, VILLAGE_ANCHOR, colony_to_world, next_building_site_default,
        ring_cells, village_ring_radius, world_to_colony,
    },
    warriors::{
        CombatModifiers, DefenseStock, MusterCombatant, WARRIOR_XP_PER_RAID, can_fight,
        muster_defense,
    },
    world_gen::{TileResources, generate_world_chunk, get_colony_position},
    zones::{Zone, ZoneKind, ZonePos, ZoneRect, filter_targets_by_zones},
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
    /// Appointed officers (role → cat id). P12.2 additive layer; empty = no effect.
    pub officers: BTreeMap<OfficerRole, String>,
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
const QUARRY_TOTAL_YIELD: f64 = 15.0;
const ROAD_MATERIALS_RESERVE: f64 = 30.0;
const ROAD_MAX_PAVE_PER_BATCH: i32 = 6;
const WATER_TOTAL_YIELD: f64 = 40.0;
const WALK_WEAR: u32 = 8;
const RAIDER_SPEED_TILES_PER_SEC: f64 = 0.4;
const RAID_SPAWN_DISTANCE: f64 = 14.0;
const ENGAGE_RANGE: f64 = 1.5;
const DEFEND_CLICK_DAMAGE: f64 = 6.0;
const STARTER_CAT_COUNT: usize = 20;
const STARTER_AGE_MIN_HOURS: f64 = 6.0;
const STARTER_AGE_MAX_HOURS: f64 = 30.0;
const VILLAGE_START_RADIUS: i32 = 3;

#[derive(Debug, Clone)]
struct MovementPassContext {
    movement_seed: u32,
    movement_elapsed: f64,
    wander_chance: f64,
    ring_radius: i32,
    claimed_area: crate::village_area::VillageArea,
    area_gate: Option<AreaGatePlacement>,
    gate: TilePos,
    walk_tiles: Vec<WalkTile>,
    zones: Vec<Zone>,
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
            officers: BTreeMap::new(),
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
pub const fn new_world(world_seed: u32) -> WorldState {
    WorldState {
        world_seed,
        colonies: Vec::new(),
    }
}

#[must_use]
pub fn found_colony(
    world_seed: u32,
    colony_id: impl Into<ColonyId>,
    now_ms: i64,
    seed: u32,
) -> ColonyRuntime {
    let colony_id = colony_id.into();
    ColonyRuntime {
        id: colony_id.clone(),
        name: format!("Colony {colony_id}"),
        status: ColonyStatus::Starting,
        resources: starting_resources(),
        cats: create_starter_cats(&colony_id, now_ms, seed),
        buildings: starter_buildings(),
        world_tiles: starter_world_tiles(world_seed),
        claimed_tiles: founding_claimed_tiles(),
        run_number: 1,
        run_started_at: now_ms,
        created_at: now_ms,
        last_player_activity_at: Some(now_ms),
        last_tick: now_ms,
        test_rng_seed: Some(seed),
        ..ColonyRuntime::default()
    }
}

fn starting_resources() -> Resources {
    Resources {
        food: 150.0,
        water: 100.0,
        herbs: 16.0,
        materials: 24.0,
        refined: 0.0,
        weapons: 0.0,
        armor: 0.0,
        blessings: 0.0,
    }
}

fn create_starter_cats(colony_id: &str, now_ms: i64, seed: u32) -> Vec<Cat> {
    let names = starter_names();
    let mut rolls = SeededRollSource::new(seed);

    (0..STARTER_CAT_COUNT)
        .map(|index| {
            let spot = starter_cat_spot(index);
            Cat {
                id: format!("{colony_id}-cat-{}", index + 1),
                colony_id: colony_id.to_owned(),
                name: names[index].clone(),
                parent_ids: vec![None, None],
                birth_time: now_ms,
                death_time: None,
                stats: CatStats {
                    attack: starter_stat(&mut rolls, 30, 60),
                    defense: starter_stat(&mut rolls, 30, 60),
                    hunting: starter_stat(&mut rolls, 30, 60),
                    medicine: starter_stat(&mut rolls, 20, 50),
                    cleaning: starter_stat(&mut rolls, 25, 55),
                    building: starter_stat(&mut rolls, 20, 50),
                    leadership: starter_stat(&mut rolls, 20, 60),
                    vision: starter_stat(&mut rolls, 30, 60),
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
                    x: f64::from(spot.x),
                    y: f64::from(spot.y),
                },
                destination: None,
                carrying: None,
                activity: CatActivity::Idle,
                is_pregnant: false,
                pregnancy_due_time: None,
                age_hours: starter_age_hours(index),
                pregnancy_due_age_hours: None,
                pregnancy_mate_id: None,
                sprite_params: starter_sprite_params(&mut rolls),
                specialization: None,
                role_xp: RoleXp::default(),
                skills: Default::default(),
            }
        })
        .collect()
}

fn starter_names() -> Vec<String> {
    let mut names = ["Whiskers", "Shadow", "Luna", "Max", "Bella"]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut seed = 424_242;
    while names.len() < STARTER_CAT_COUNT {
        let name = generate_starter_name(seed);
        seed += 1;
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

fn generate_starter_name(seed: u32) -> String {
    const PREFIXES: &[&str] = &[
        "Shadow", "Thorn", "Storm", "Ember", "Frost", "Bramble", "Ivy", "Ash", "Fern", "Willow",
        "Birch", "Hawk", "Raven", "Cedar", "Moss", "Flame", "Breeze", "Dusk", "Dawn", "Flint",
        "Stone", "Cloud", "Sage", "Briar", "Alder", "Maple", "Pine", "Otter", "Fox", "Wren",
    ];
    const SUFFIXES: &[&str] = &[
        "claw", "whisker", "heart", "fur", "stripe", "tail", "fang", "pelt", "thorn", "leaf",
        "brook", "blaze", "shade", "pool", "flight",
    ];

    let prefix_roll = roll_seeded(f64::from(seed));
    let prefix = PREFIXES[(prefix_roll.value * PREFIXES.len() as f64).floor() as usize];
    let first_suffix_roll = roll_seeded(f64::from(prefix_roll.next_seed));
    let suffix_roll = roll_seeded(f64::from(first_suffix_roll.next_seed));
    let suffix = SUFFIXES[(suffix_roll.value * SUFFIXES.len() as f64).floor() as usize];

    format!("{prefix}{suffix}")
}

fn starter_cat_spot(index: usize) -> GridPos {
    let spots = ring_cells(2)
        .into_iter()
        .chain(ring_cells(3))
        .collect::<Vec<_>>();
    spots[index % spots.len()]
}

fn starter_age_hours(index: usize) -> f64 {
    let span = STARTER_AGE_MAX_HOURS - STARTER_AGE_MIN_HOURS;
    let denom = (STARTER_CAT_COUNT - 1).max(1) as f64;
    STARTER_AGE_MIN_HOURS + ((index as f64 / denom) * span).round()
}

fn starter_stat(rolls: &mut impl RollSource, min: u32, max: u32) -> f64 {
    let width = max - min + 1;
    f64::from(min + (rolls.roll() * f64::from(width)).floor() as u32)
}

fn starter_sprite_params(
    rolls: &mut impl RollSource,
) -> Option<BTreeMap<String, serde_json::Value>> {
    let traits = inherit_traits(None, None, rolls);
    let params = traits_to_sprite_params(&traits, None, rolls);
    let value = serde_json::to_value(params).ok()?;
    match value {
        serde_json::Value::Object(map) => Some(map.into_iter().collect()),
        _ => None,
    }
}

fn starter_buildings() -> Vec<BuildingRuntime> {
    let mut occupied = Vec::new();
    let mut buildings = vec![BuildingRuntime {
        id: "building-shrine".to_owned(),
        building_type: BuildingType::Shrine,
        level: 1,
        position: grid_to_tile(colony_to_world(SHRINE_LOCAL)),
        is_complete: true,
        construction_progress: 100,
        production_progress: 0.0,
        assigned_cat: None,
    }];
    let starter_specs = [
        (BuildingType::Den, 0.05, 2),
        (BuildingType::Den, 0.3, 2),
        (BuildingType::Den, 0.55, 2),
        (BuildingType::Den, 0.8, 2),
        (BuildingType::Den, 0.95, 1),
        (BuildingType::FoodStorage, 0.4, 1),
    ];

    for (index, (building_type, roll, level)) in starter_specs.into_iter().enumerate() {
        let Some(local_site) = next_building_site_default(&occupied, roll) else {
            break;
        };
        occupied.push(local_site);
        buildings.push(BuildingRuntime {
            id: format!("building-starter-{}", index + 1),
            building_type,
            level,
            position: grid_to_tile(colony_to_world(local_site)),
            is_complete: true,
            construction_progress: 100,
            production_progress: 0.0,
            assigned_cat: None,
        });
    }

    buildings
}

fn founding_claimed_tiles() -> Vec<TilePos> {
    let mut tiles = Vec::new();
    for dy in -VILLAGE_START_RADIUS..=VILLAGE_START_RADIUS {
        for dx in -VILLAGE_START_RADIUS..=VILLAGE_START_RADIUS {
            tiles.push(TilePos {
                x: VILLAGE_ANCHOR.x + dx,
                y: VILLAGE_ANCHOR.y + dy,
            });
        }
    }
    tiles
}

fn starter_world_tiles(world_seed: u32) -> BTreeMap<TilePos, WorldTileRuntime> {
    let colony_pos = get_colony_position();
    let mut tiles = BTreeMap::new();

    for chunk_y in -1..=1 {
        for chunk_x in -1..=1 {
            for tile in generate_world_chunk(
                chunk_x,
                chunk_y,
                i64::from(world_seed),
                colony_pos.x,
                colony_pos.y,
            ) {
                let pos = TilePos {
                    x: tile.x,
                    y: tile.y,
                };
                tiles.insert(
                    pos,
                    WorldTileRuntime {
                        pos,
                        tile_type: tile.tile_type,
                        resources: tile.resources,
                        max_resources: tile.max_resources,
                        danger_level: tile.danger_level,
                        path_wear: tile.path_wear,
                        last_depleted: tile.last_depleted,
                        overlay_feature: tile
                            .overlay_feature
                            .map(|feature| feature.as_str().to_owned()),
                    },
                );
            }
        }
    }

    tiles
}

fn grid_to_tile(pos: GridPos) -> TilePos {
    TilePos { x: pos.x, y: pos.y }
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
        if let Some(reset_reason) = phase_26_empty_colony_reset(colony, gate) {
            reports.push(TickReport {
                colony_id: colony.id.clone(),
                skipped: false,
                reset_reason: Some(reset_reason),
            });
            continue;
        }
        phase_27_due_job_prelude(colony, gate);
        phase_28_due_completion_supplies_and_planner_jobs(colony, gate);
        phase_29_due_completion_gathering_explore_expansion(colony, gate);
        phase_30_due_completion_build_ritual_training_return_mark_done(colony, gate);
        phase_31_mid_job_hauling(colony, gate);
        let mut movement =
            phase_32_movement_setup_and_village_expansion_queue(colony, gate, policy);
        phase_33_movement_deposits_and_no_destination_wander(colony, gate, &mut movement);
        phase_34_movement_travel_job_acceptance_reveal_path_wear(colony, gate, &movement);
        phase_35_deliberate_roads(colony, gate);
        if let Some(reset_reason) = phase_36_threat_and_raid_director(colony, gate) {
            reports.push(TickReport {
                colony_id: colony.id.clone(),
                skipped: false,
                reset_reason: Some(reset_reason),
            });
            continue;
        }
        let reset_reason = phase_37_final_clamp_critical_collapse_status_persist(colony, gate);

        reports.push(TickReport {
            colony_id: colony.id.clone(),
            skipped: false,
            reset_reason,
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
fn phase_14_promote_queued_jobs_and_break_ground(colony: &mut ColonyRuntime, gate: TickGate) {
    let queued_indices = colony
        .jobs
        .iter()
        .enumerate()
        .filter_map(|(index, job)| (job.status == JobStatus::Queued).then_some(index))
        .collect::<Vec<_>>();
    let mut movement_seed = movement_seed(colony.test_rng_seed.unwrap_or(1));

    for job_index in queued_indices {
        let mut next_metadata = colony.jobs[job_index].metadata.clone();

        if colony.jobs[job_index].kind == JobKind::BuildHouse
            && matches!(
                next_metadata,
                JobMetadata::Construction {
                    phase: ConstructionPhase::ConstructHouse,
                    ..
                }
            )
        {
            let roll = roll_seeded(f64::from(movement_seed));
            movement_seed = roll.next_seed;

            if let Some(site_local) = next_claimed_building_site(colony, roll.value) {
                let building_id = format!(
                    "building-{}-{}",
                    gate.processed_through,
                    colony.buildings.len() + 1
                );
                let scaffold_type = match next_metadata {
                    JobMetadata::Construction { building_type, .. } => {
                        scaffold_building_type(building_type)
                    }
                    _ => BuildingType::Den,
                };

                colony.buildings.push(BuildingRuntime {
                    id: building_id.clone(),
                    building_type: scaffold_type,
                    level: 1,
                    position: site_local,
                    is_complete: false,
                    construction_progress: 0,
                    production_progress: 0.0,
                    assigned_cat: None,
                });

                if let JobMetadata::Construction {
                    phase,
                    building_type: _,
                    ..
                } = next_metadata
                {
                    next_metadata = JobMetadata::Construction {
                        phase,
                        building_type: scaffold_type,
                        building_id: Some(building_id),
                        site: Some(site_local),
                    };
                }
            }
        }

        let job = &mut colony.jobs[job_index];
        job.status = JobStatus::Active;
        if job.started_at.is_none() {
            job.started_at = Some(gate.processed_through);
        }
        job.metadata = next_metadata;
    }
}

/// Phase 15: assign destinations for promoted jobs, including zoned hunt picks,
/// quarry/water/frontier targets, expansion targets, and shrine travel.
fn phase_15_assign_promoted_job_destinations(colony: &mut ColonyRuntime, _: TickGate) {
    let active_indices = colony
        .jobs
        .iter()
        .enumerate()
        .filter_map(|(index, job)| {
            (job.status == JobStatus::Active
                && job.assigned_cat.is_some()
                && !job_has_destination_metadata(job))
            .then_some(index)
        })
        .collect::<Vec<_>>();
    let mut movement_seed = movement_seed(colony.test_rng_seed.unwrap_or(1));
    let food_tiles = food_tiles_near_village(colony);
    let quarry_site = quarry_sites_near_village(colony).into_iter().next();
    let water_site = water_sites_near_village(colony).into_iter().next();
    let frontier_tiles = frontier_tiles_near_village(colony);
    let mut scout_promotions = 0usize;

    for job_index in active_indices {
        let job = colony.jobs[job_index].clone();
        let roll = roll_seeded(f64::from(movement_seed));
        movement_seed = roll.next_seed;

        let construction_site = match job.metadata {
            JobMetadata::Construction {
                site: Some(site), ..
            } => Some(tile_pos_to_world(site)),
            _ => None,
        };
        let expansion_site = match job.metadata {
            JobMetadata::Expansion { target, .. } => Some(tile_pos_to_world(target)),
            _ => None,
        };
        let explore_site = if job.kind == JobKind::Explore && !frontier_tiles.is_empty() {
            let site = frontier_tiles[scout_promotions % frontier_tiles.len()];
            scout_promotions += 1;
            Some(site)
        } else {
            None
        };
        let hunt_tiles = if job.kind == JobKind::HuntExpedition {
            food_tiles.as_slice()
        } else {
            &[]
        };
        let context = JobDestinationContext {
            anchor: village_anchor_world(),
            shrine: village_anchor_world(),
            food_tiles: hunt_tiles,
            roll: roll.value,
            site: construction_site,
            expansion_site,
            quarry_site,
            water_site,
            explore_site,
        };

        let Some(destination) = destination_for_job(job.kind.as_str(), &context) else {
            continue;
        };
        let site = world_pos_to_tile(destination);

        colony.jobs[job_index].metadata = match colony.jobs[job_index].kind {
            JobKind::HuntExpedition | JobKind::Quarry | JobKind::FetchWater => {
                JobMetadata::Hauling {
                    site: Some(site),
                    total_yield: None,
                    trips_done: 0,
                    next_trip_at: None,
                    accepted: false,
                }
            }
            JobKind::ExpandVillage => JobMetadata::Expansion {
                target: site,
                accepted: false,
            },
            JobKind::BuildHouse => match colony.jobs[job_index].metadata.clone() {
                JobMetadata::Construction {
                    phase,
                    building_type,
                    building_id,
                    ..
                } => JobMetadata::Construction {
                    phase,
                    building_type,
                    building_id,
                    site: Some(site),
                },
                _ => JobMetadata::Site {
                    site,
                    accepted: false,
                },
            },
            _ => JobMetadata::Site {
                site,
                accepted: false,
            },
        };

        if let Some(cat_id) = colony.jobs[job_index].assigned_cat.clone()
            && let Some(cat) = colony.cats.iter_mut().find(|cat| cat.id == cat_id)
        {
            cat.destination = Some(position_from_world(village_anchor_world()));
            cat.activity = CatActivity::Traveling;
            cat.current_task = task_for_job(job.kind);
        }
    }
}

/// Phase 16: update active scaffold progress from job timer progress.
fn phase_16_active_scaffold_progress(colony: &mut ColonyRuntime, gate: TickGate) {
    for job in &colony.jobs {
        if job.status != JobStatus::Active || job.kind != JobKind::BuildHouse {
            continue;
        }
        let JobMetadata::Construction {
            building_id: Some(building_id),
            ..
        } = &job.metadata
        else {
            continue;
        };
        let started_at = job.started_at.unwrap_or(gate.processed_through);
        let ends_at = job.ends_at.unwrap_or(started_at);
        let duration = ends_at.saturating_sub(started_at).max(1);
        let progress = (((gate.processed_through - started_at) as f64 / duration as f64) * 100.0)
            .round()
            .clamp(0.0, 99.0) as u8;

        if let Some(building) = colony
            .buildings
            .iter_mut()
            .find(|building| building.id == *building_id)
        {
            building.construction_progress = progress;
            building.is_complete = false;
        }
    }
}

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
        officers: colony.officers.clone(),
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

    let assignments = match_cats_to_slots_with_officers(
        &plan.slots,
        &available_idle,
        &colony.officers,
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
fn phase_23_production(colony: &mut ColonyRuntime, gate: TickGate) {
    auto_staff_idle_buildings(colony, BuildingType::Workshop, gate.processed_through);

    let production_elapsed = gate.elapsed_sec as f64 * normalize_time_scale(colony);
    let building_ids = colony
        .buildings
        .iter()
        .enumerate()
        .filter_map(|(index, building)| {
            (building.construction_progress >= 100).then_some((index, building.id.clone()))
        })
        .collect::<Vec<_>>();

    for (building_index, building_id) in building_ids {
        match colony.buildings[building_index].building_type {
            BuildingType::Field => {
                colony.resources.food += field_yield(production_elapsed);
            }
            BuildingType::Workshop => {
                let worker = assigned_worker(colony, &building_id);
                let step = advance_workshop(
                    colony.buildings[building_index].production_progress,
                    production_elapsed,
                    WorkshopOptions {
                        has_worker: worker.is_some(),
                        worker_is_architect: worker.is_some_and(|cat| {
                            cat.specialization == Some(CatSpecialization::Architect)
                        }),
                        materials_available: colony.resources.materials,
                    },
                );
                if step.refined_produced > 0.0 {
                    colony.resources.materials =
                        (colony.resources.materials - step.materials_used).max(0.0);
                    colony.resources.refined += step.refined_produced;
                    append_event(
                        colony,
                        gate.processed_through,
                        EventKind::Other("production".to_owned()),
                        format!(
                            "The workshop refined {} materials into {} refined good{}.",
                            step.materials_used,
                            step.refined_produced,
                            if step.refined_produced == 1.0 {
                                ""
                            } else {
                                "s"
                            }
                        ),
                    );
                }
                colony.buildings[building_index].production_progress = step.next_progress;
            }
            BuildingType::Smithy => {
                let worker = assigned_worker(colony, &building_id);
                let step = advance_smithy(
                    colony.buildings[building_index].production_progress,
                    production_elapsed,
                    SmithyOptions {
                        has_worker: worker.is_some(),
                        worker_is_fast: worker.is_some_and(|cat| {
                            cat.specialization == Some(CatSpecialization::Architect)
                        }),
                        refined_available: colony.resources.refined,
                        materials_available: colony.resources.materials,
                    },
                );
                if step.weapons_produced > 0.0 || step.armor_produced > 0.0 {
                    colony.resources.refined =
                        (colony.resources.refined - step.refined_used).max(0.0);
                    colony.resources.materials =
                        (colony.resources.materials - step.materials_used).max(0.0);
                    colony.resources.weapons += step.weapons_produced;
                    colony.resources.armor += step.armor_produced;
                    append_event(
                        colony,
                        gate.processed_through,
                        EventKind::Other("production".to_owned()),
                        format!(
                            "The smith forged {} weapon{} and {} armor at the smithy.",
                            step.weapons_produced,
                            if step.weapons_produced == 1.0 {
                                ""
                            } else {
                                "s"
                            },
                            step.armor_produced,
                        ),
                    );
                }
                colony.buildings[building_index].production_progress = step.next_progress;
            }
            _ => {}
        }
    }
}

/// Phase 24: accrue research from staffed research huts/schools and auto-unlock
/// affordable upgrade-tree nodes.
fn phase_24_research(colony: &mut ColonyRuntime, gate: TickGate) {
    let research_workforce = research_workforce(colony);
    let effects = resolve_effects(colony.upgrade_tree.owned_node_ids.iter());
    let gained = points_per_tick_for(
        research_workforce,
        gate.elapsed_sec as f64 * normalize_time_scale(colony),
        effects.research_rate_mult,
    );

    if gained > 0.0 {
        colony.upgrade_tree = accrue_research(&colony.upgrade_tree, gained);
    }

    loop {
        let result = cat_auto_unlock(&colony.upgrade_tree);
        if !result.ok {
            break;
        }
        let node_name = result
            .node_id
            .as_deref()
            .and_then(get_node)
            .map_or_else(
                || result.node_id.as_deref().unwrap_or("something"),
                |node| node.name,
            )
            .to_owned();
        colony.upgrade_tree = result.state;
        append_event(
            colony,
            gate.processed_through,
            EventKind::Other("research_unlocked".to_owned()),
            format!("The cats discovered {node_name}!"),
        );
    }
}

/// Phase 25: apply survival needs, deaths, carried-yield salvage, and
/// death-related job retirement.
fn phase_25_survival_deaths_and_carried_yield_salvage(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 26: reset empty colonies and short-circuit the remaining phases.
fn phase_26_empty_colony_reset(
    colony: &mut ColonyRuntime,
    gate: TickGate,
) -> Option<RunResetReason> {
    if colony.cats.is_empty() {
        return None;
    }
    if alive_cats(&colony.cats).next().is_some() {
        return None;
    }

    reset_run(colony, gate.processed_through, RunResetReason::AllCatsDead);
    Some(RunResetReason::AllCatsDead)
}

/// Phase 27: collect due active jobs and preserve the phase-14 queued snapshot
/// needed by completion parity.
fn phase_27_due_job_prelude(_: &mut ColonyRuntime, _: TickGate) {}

/// Phase 28: complete supply jobs and planner jobs, including hunt and house
/// queueing.
fn phase_28_due_completion_supplies_and_planner_jobs(colony: &mut ColonyRuntime, gate: TickGate) {
    let due_jobs = due_active_jobs(colony, gate);

    for job in due_jobs {
        match job.kind {
            JobKind::SupplyFood => colony.resources.food += 8.0,
            JobKind::SupplyWater => colony.resources.water += 8.0,
            JobKind::LeaderPlanHunt => {
                let hunter = select_best_cat(colony, Some(CatSpecialization::Hunter));
                queue_job(
                    colony,
                    gate.processed_through,
                    JobKind::HuntExpedition,
                    hunter,
                    JobMetadata::None,
                );
            }
            JobKind::LeaderPlanHouse => {
                let architect = select_best_cat(colony, Some(CatSpecialization::Architect));
                queue_job(
                    colony,
                    gate.processed_through,
                    JobKind::BuildHouse,
                    architect,
                    JobMetadata::Construction {
                        phase: ConstructionPhase::GatherMaterials,
                        building_type: BuildingType::Den,
                        building_id: None,
                        site: None,
                    },
                );
            }
            _ => {}
        }
    }
}

/// Phase 29: complete hunt/quarry/water/explore/expansion jobs, including tile
/// depletion and claimed-area mutation.
fn phase_29_due_completion_gathering_explore_expansion(colony: &mut ColonyRuntime, gate: TickGate) {
    let due_jobs = due_active_jobs(colony, gate);

    for job in due_jobs {
        match job.kind {
            JobKind::HuntExpedition => complete_hunt(colony, &job, gate),
            JobKind::Quarry => complete_fixed_yield_job(
                colony,
                &job,
                gate,
                QUARRY_TOTAL_YIELD,
                CarryingKind::Materials,
            ),
            JobKind::FetchWater => {
                complete_fixed_yield_job(colony, &job, gate, WATER_TOTAL_YIELD, CarryingKind::Water)
            }
            JobKind::Explore => {
                append_event(
                    colony,
                    gate.processed_through,
                    EventKind::Other("discovery".to_owned()),
                    "The scout mapped the lands around the village.",
                );
            }
            JobKind::ExpandVillage => complete_village_expansion(colony, &job, gate),
            _ => {}
        }
    }
}

/// Phase 30: complete build/ritual/training jobs, return workers, and mark jobs
/// completed.
fn phase_30_due_completion_build_ritual_training_return_mark_done(
    colony: &mut ColonyRuntime,
    gate: TickGate,
) {
    let due_jobs = due_active_jobs(colony, gate);

    for job in &due_jobs {
        match job.kind {
            JobKind::BuildHouse => complete_build(colony, job, gate),
            JobKind::Ritual => complete_ritual(colony, job, gate),
            JobKind::TrainWarrior => complete_warrior_training(colony, job, gate),
            _ => {}
        }

        if matches!(
            job.kind,
            JobKind::HuntExpedition
                | JobKind::BuildHouse
                | JobKind::Ritual
                | JobKind::Quarry
                | JobKind::Explore
                | JobKind::FetchWater
                | JobKind::ExpandVillage
        ) {
            return_assigned_cat(colony, job, gate);
        }
    }

    for job in due_jobs {
        if let Some(stored) = colony
            .jobs
            .iter_mut()
            .find(|candidate| candidate.id == job.id)
        {
            stored.status = JobStatus::Completed;
            stored.completed_at = Some(gate.processed_through);
        }
        append_event(
            colony,
            gate.processed_through,
            EventKind::JobCompleted,
            format!("Completed {}.", job.kind.as_str().replace('_', " ")),
        );
    }
}

/// Phase 31: run mid-job hauling trips for accepted active gathering and fetch
/// jobs.
fn phase_31_mid_job_hauling(colony: &mut ColonyRuntime, gate: TickGate) {
    let active_jobs = colony
        .jobs
        .iter()
        .filter(|job| {
            job.status == JobStatus::Active
                && matches!(
                    job.kind,
                    JobKind::HuntExpedition | JobKind::Quarry | JobKind::FetchWater
                )
                && job.assigned_cat.is_some()
        })
        .cloned()
        .collect::<Vec<_>>();

    for job in active_jobs {
        let JobMetadata::Hauling {
            site: Some(site),
            total_yield,
            trips_done,
            next_trip_at,
            accepted: true,
        } = job.metadata
        else {
            continue;
        };
        if trips_done >= (HUNT_TRIP_COUNT - 1) as u32 {
            continue;
        }
        let Some(started_at) = job.started_at else {
            continue;
        };
        let Some(ends_at) = job.ends_at else {
            continue;
        };
        let due_at = next_trip_at.unwrap_or_else(|| {
            trip_due_at(started_at as f64, ends_at as f64, trips_done as i32 + 1) as i64
        });
        if gate.processed_through < due_at || ends_at <= gate.processed_through {
            continue;
        }
        let Some(cat_id) = job.assigned_cat.as_deref() else {
            continue;
        };
        let Some(cat_index) = colony
            .cats
            .iter()
            .position(|cat| cat.id == cat_id && cat.death_time.is_none())
        else {
            continue;
        };
        if colony.cats[cat_index].activity != CatActivity::Working
            || colony.cats[cat_index].carrying.is_some()
        {
            continue;
        }

        let total = total_yield.unwrap_or_else(|| total_yield_for_job(colony, &job, cat_index));
        let share = split_yield(total, HUNT_TRIP_COUNT, trips_done as i32);
        if job.kind == JobKind::HuntExpedition {
            drain_hunt_site(colony, site, share, gate.processed_through);
        }

        if let Some(stored) = colony
            .jobs
            .iter_mut()
            .find(|candidate| candidate.id == job.id)
        {
            stored.metadata = JobMetadata::Hauling {
                site: Some(site),
                total_yield: Some(total),
                trips_done: trips_done + 1,
                next_trip_at: Some(trip_due_at(
                    started_at as f64,
                    ends_at as f64,
                    trips_done as i32 + 2,
                ) as i64),
                accepted: true,
            };
        }

        colony.cats[cat_index].gain_skill(Labor::Haul, HAUL_SKILL_GAIN);
        colony.cats[cat_index].carrying = Some(Carrying {
            kind: carrying_kind_for_job(job.kind),
            amount: share,
            job_ended_at: gate.processed_through,
        });
        colony.cats[cat_index].destination = Some(position_from_world(village_anchor_world()));
        colony.cats[cat_index].activity = CatActivity::Returning;
    }
}

/// Phase 32: prepare movement inputs and optionally queue village expansion.
fn phase_32_movement_setup_and_village_expansion_queue(
    colony: &mut ColonyRuntime,
    gate: TickGate,
    policy: TickPolicy,
) -> MovementPassContext {
    let mut movement_seed = movement_seed(colony.test_rng_seed.unwrap_or(1));
    let movement_elapsed = gate.elapsed_sec as f64 * normalize_time_scale(colony);
    let wander_chance = (0.02 * gate.elapsed_sec as f64).min(0.08);
    let ring_radius = village_ring_radius(colony.buildings.len() as i32);
    let claimed_area = claimed_area(colony);

    if !claimed_area.is_empty()
        && should_expand(
            alive_cats(&colony.cats).count() as i32,
            claimed_area.len() as i32,
            colony.buildings.len() as i32,
        )
        && !active_or_queued_jobs(colony)
            .iter()
            .any(|job| job.kind == JobKind::ExpandVillage)
        && can_take_policy_action(colony, policy)
    {
        let water_tiles = colony
            .world_tiles
            .values()
            .filter(|tile| tile_has_water(Some(tile)))
            .map(|tile| tile.pos)
            .collect::<HashSet<_>>();
        let roll = roll_seeded(f64::from(movement_seed));
        movement_seed = roll.next_seed;
        let mut next_roll = Some(roll.value);
        let mut rng = || next_roll.take().unwrap_or(0.0);
        let is_water = |pos: GridPos| water_tiles.contains(&TilePos { x: pos.x, y: pos.y });
        if let Some(target) = expand_village(
            &claimed_area,
            ExpandOptions {
                is_water: Some(&is_water),
                rng: Some(&mut rng),
            },
        ) {
            queue_job(
                colony,
                gate.processed_through,
                JobKind::ExpandVillage,
                select_best_cat(colony, Some(CatSpecialization::Architect)),
                JobMetadata::Expansion {
                    target: TilePos {
                        x: target.x,
                        y: target.y,
                    },
                    accepted: false,
                },
            );
        }
    }

    let area_gate = (!claimed_area.is_empty())
        .then(|| gate_placement_default(&claimed_area))
        .flatten();
    let gate_pos = movement_gate(area_gate, ring_radius);

    MovementPassContext {
        movement_seed,
        movement_elapsed,
        wander_chance,
        ring_radius,
        claimed_area,
        area_gate,
        gate: gate_pos,
        walk_tiles: colony
            .world_tiles
            .values()
            .map(walk_tile_from_runtime)
            .collect(),
        zones: colony
            .zones
            .iter()
            .map(|zone| Zone {
                rect: zone.rect,
                kind: zone.kind,
            })
            .collect(),
    }
}

/// Phase 33: deposit carried resources, clear missing destinations, and pick
/// idle wander targets.
fn phase_33_movement_deposits_and_no_destination_wander(
    colony: &mut ColonyRuntime,
    gate: TickGate,
    movement: &mut MovementPassContext,
) {
    let cat_ids = colony
        .cats
        .iter()
        .filter_map(|cat| {
            cat.death_time
                .is_none()
                .then_some((cat.id.clone(), cat.position, cat.carrying.clone()))
        })
        .collect::<Vec<_>>();

    for (cat_id, position, carrying) in cat_ids {
        if let Some(carrying) = carrying {
            let world_pos = position_to_world(position);
            if !should_deposit(
                &carrying,
                world_pos,
                village_anchor_world(),
                gate.processed_through,
            ) {
                continue;
            }

            credit_carrying(colony, &carrying);
            append_event(
                colony,
                gate.processed_through,
                EventKind::Other("shrine_deposit".to_owned()),
                deposit_message(&cat_id, &carrying),
            );

            let return_site = active_site_for_carrier(colony, &cat_id, gate.processed_through);
            if let Some(cat) = colony.cats.iter_mut().find(|cat| cat.id == cat_id) {
                cat.carrying = None;
                if let Some(site) = return_site {
                    cat.destination = Some(position_from_world(tile_pos_to_world(site)));
                    cat.activity = CatActivity::Traveling;
                    continue;
                }
            }
        }

        let Some(cat_index) = colony
            .cats
            .iter()
            .position(|cat| cat.id == cat_id && cat.death_time.is_none())
        else {
            continue;
        };
        if colony.cats[cat_index].destination.is_some() {
            continue;
        }

        match colony.cats[cat_index].activity {
            CatActivity::Traveling | CatActivity::Returning => {
                colony.cats[cat_index].activity = CatActivity::Idle;
            }
            CatActivity::Idle => {
                if next_movement_roll(movement) >= movement.wander_chance {
                    continue;
                }
                let world_pos = position_to_world(colony.cats[cat_index].position);
                let anchor = wander_anchor(colony, &cat_id);
                let target = pick_wander_target(
                    anchor,
                    next_movement_roll(movement),
                    next_movement_roll(movement),
                );
                let target_zone_pos = ZonePos {
                    x: target.x.round() as i32,
                    y: target.y.round() as i32,
                };
                if filter_targets_by_zones(&[target_zone_pos], &movement.zones, false).is_empty()
                    || target == world_pos
                {
                    continue;
                }
                colony.cats[cat_index].destination = Some(position_from_world(target));
            }
            CatActivity::Working => {}
        }
    }
}

/// Phase 34: move cats, accept jobs on shrine arrival, reveal tiles, and apply
/// path wear.
fn phase_34_movement_travel_job_acceptance_reveal_path_wear(
    colony: &mut ColonyRuntime,
    _: TickGate,
    movement: &MovementPassContext,
) {
    let area = pathfinding_area(&movement.claimed_area);
    let area_gate = movement.area_gate.map(pathfinding_gate);
    let walk_grid = build_colony_walk_grid(ColonyGridParams {
        tiles: &movement.walk_tiles,
        anchor: PathTilePos {
            x: VILLAGE_ANCHOR.x,
            y: VILLAGE_ANCHOR.y,
        },
        ring_radius: movement.ring_radius,
        gate: PathTilePos {
            x: movement.gate.x,
            y: movement.gate.y,
        },
        area: (!area.is_empty()).then_some(&area),
        area_gate,
        terrain: None,
    });
    let effects = resolve_effects(colony.upgrade_tree.owned_node_ids.iter());
    let cat_ids = colony
        .cats
        .iter()
        .filter_map(|cat| cat.death_time.is_none().then_some(cat.id.clone()))
        .collect::<Vec<_>>();

    for cat_id in cat_ids {
        let Some(cat_index) = colony
            .cats
            .iter()
            .position(|cat| cat.id == cat_id && cat.death_time.is_none())
        else {
            continue;
        };
        let Some(destination) = colony.cats[cat_index].destination else {
            continue;
        };

        let world_pos = position_to_world(colony.cats[cat_index].position);
        let destination = position_to_world(destination);
        let activity = colony.cats[cat_index].activity;
        let current_task = colony.cats[cat_index].current_task;
        let standing_tile = colony.world_tiles.get(&world_pos_to_tile(world_pos));
        let explore_slowdown =
            if current_task == Some(TaskType::Explore) && activity == CatActivity::Traveling {
                EXPLORE_SPEED_FACTOR
            } else {
                1.0
            };
        let speed = MOVE_SPEED_TILES_PER_SEC
            * (1.0 + movement_speed_bonus(standing_tile))
            * explore_slowdown
            * effects.move_speed_mult;

        let route = find_path(
            pathfinding_pos(world_pos),
            pathfinding_pos(destination),
            &walk_grid,
            FindPathOptions::default(),
        );
        let crosses_fence = is_inside_movement_village(world_pos_to_tile(world_pos), movement)
            != is_inside_movement_village(world_pos_to_tile(destination), movement);
        let at_gate = (world_pos.x - f64::from(movement.gate.x)).abs() < 1.0
            && (world_pos.y - f64::from(movement.gate.y)).abs() < 1.0;
        let waypoints = if let Some(route) = route.as_ref().filter(|route| route.len() > 2) {
            route[1..route.len() - 1]
                .iter()
                .copied()
                .map(movement_pos)
                .collect::<Vec<_>>()
        } else if crosses_fence && !at_gate {
            vec![tile_pos_to_world(movement.gate)]
        } else {
            Vec::new()
        };
        let walk = walk_path(
            world_pos,
            destination,
            movement.movement_elapsed * speed,
            &waypoints,
        );
        let arrived = walk.arrived;

        if arrived
            && activity == CatActivity::Traveling
            && let Some((job_index, site)) = unaccepted_active_job_site(colony, &cat_id)
        {
            accept_job(colony, job_index);
            let cat = &mut colony.cats[cat_index];
            cat.position = position_from_world(walk.position);
            cat.destination = Some(position_from_world(tile_pos_to_world(site)));
            continue;
        }

        let moved = walk.position != world_pos;
        if !moved && !arrived {
            continue;
        }

        {
            let cat = &mut colony.cats[cat_index];
            cat.position = position_from_world(walk.position);
            if arrived {
                cat.destination = None;
                cat.activity = if activity == CatActivity::Traveling {
                    CatActivity::Working
                } else {
                    CatActivity::Idle
                };
            }
        }

        if moved {
            reveal_and_wear_walked_tiles(colony, movement, &walk.tiles, current_task);
        }
    }
}

/// Phase 35: pave deliberate road corridors once per minute while preserving the
/// materials reserve.
fn phase_35_deliberate_roads(colony: &mut ColonyRuntime, gate: TickGate) {
    if !gate.minute_rolled || colony.resources.materials <= ROAD_MATERIALS_RESERVE {
        return;
    }

    let max_tiles = ROAD_MAX_PAVE_PER_BATCH
        .min((colony.resources.materials - ROAD_MATERIALS_RESERVE).floor() as i32);
    if max_tiles <= 0 {
        return;
    }

    let ring_radius = village_ring_radius(colony.buildings.len() as i32);
    let road_tiles = colony
        .world_tiles
        .values()
        .filter(|tile| tile.path_wear >= crate::roads::ROAD_PAVE_WEAR as u32)
        .map(|tile| RoadTile {
            x: tile.pos.x,
            y: tile.pos.y,
            path_wear: f64::from(tile.path_wear),
            is_paved: tile.overlay_feature.as_deref() == Some("road_built"),
        })
        .collect::<Vec<_>>();
    let corridor = select_road_corridor(
        &road_tiles,
        RoadCorridorOptions {
            anchor_x: VILLAGE_ANCHOR.x,
            anchor_y: VILLAGE_ANCHOR.y,
            ring_radius,
            max_tiles,
            wear_threshold: None,
        },
    );

    if corridor.is_empty() {
        return;
    }

    let mut paved = 0usize;
    for road in &corridor {
        if colony.resources.materials <= ROAD_MATERIALS_RESERVE {
            break;
        }
        if let Some(tile) = colony.world_tiles.get_mut(&TilePos {
            x: road.x,
            y: road.y,
        }) {
            tile.overlay_feature = Some("road_built".to_owned());
            tile.path_wear = 100;
            colony.resources.materials -= 1.0;
            paved += 1;
        }
    }

    if paved > 0 {
        append_event(
            colony,
            gate.processed_through,
            EventKind::Other("road_built".to_owned()),
            format!(
                "The leader had a well-worn trail paved into a road ({paved} tile{}).",
                if paved == 1 { "" } else { "s" }
            ),
        );
    }
}

/// Phase 36: run threat pressure, raid spawning/marching/combat, loot, and
/// raid-wipeout reset checks.
fn phase_36_threat_and_raid_director(
    colony: &mut ColonyRuntime,
    gate: TickGate,
) -> Option<RunResetReason> {
    let mut raid_rng_seed = raid_seed(colony.test_rng_seed.unwrap_or(1));
    let mut next_raid_roll = || {
        let roll = roll_seeded(f64::from(raid_rng_seed));
        raid_rng_seed = roll.next_seed;
        roll.value
    };

    let snapshot = threat_snapshot(colony, gate);

    if colony.active_raid.is_none() {
        let pressure = accrue_threat(
            colony.threat_pressure,
            snapshot,
            gate.elapsed_sec as f64 * normalize_time_scale(colony),
        );
        if should_spawn_raid(pressure) {
            spawn_raid(colony, gate, snapshot, &mut next_raid_roll);
        } else {
            colony.threat_pressure = pressure;
        }
        return None;
    }

    apply_banked_raid_clicks(colony, gate);
    let active_raid_id = colony.active_raid.clone()?;

    let live_units = live_raider_indices(colony, &active_raid_id);
    if live_units.is_empty() {
        end_raid(colony, &active_raid_id);
        return None;
    }

    let gate_pos = raid_gate_position(colony);
    let movement_budget =
        gate.elapsed_sec as f64 * normalize_time_scale(colony) * RAIDER_SPEED_TILES_PER_SEC;
    let mut any_at_gate = false;
    for index in live_units {
        let current = WorldPos {
            x: colony.raiders[index].position.x,
            y: colony.raiders[index].position.y,
        };
        let walk = walk_path(current, tile_pos_to_world(gate_pos), movement_budget, &[]);
        colony.raiders[index].position = position_from_world(walk.position);
        colony.raiders[index].destination = Some(position_from_world(tile_pos_to_world(gate_pos)));
        if cheb_distance_world(walk.position, tile_pos_to_world(gate_pos)) <= ENGAGE_RANGE {
            any_at_gate = true;
        }
    }

    if any_at_gate {
        resolve_active_raid(colony, gate, &active_raid_id, &mut next_raid_roll);
    }

    if alive_cats(&colony.cats).next().is_none() {
        reset_run(colony, gate.processed_through, RunResetReason::RaidWipeout);
        return Some(RunResetReason::RaidWipeout);
    }

    None
}

/// Phase 37: clamp resources, handle critical collapse, update status, persist
/// final state, and record `last_tick = processed_through`.
fn phase_37_final_clamp_critical_collapse_status_persist(
    colony: &mut ColonyRuntime,
    gate: TickGate,
) -> Option<RunResetReason> {
    let caps = storage_caps(colony);
    clamp_resources_to_caps(&mut colony.resources, caps);

    let unattended_hours = colony.last_player_activity_at.map_or(0.0, |last_activity| {
        (gate.processed_through - last_activity) as f64 / 3_600_000.0
    });
    let resilience_hours = colony.test_resilience_hours_override.unwrap_or_else(|| {
        idle_engine::get_resilience_hours(
            idle_engine_upgrade_levels(&colony.upgrade_levels),
            colony.automation_tier,
        )
    });
    let critical_ms = colony.test_critical_ms_override.max(1_000);
    if crate::idle_rules::should_track_critical(
        &colony.resources,
        unattended_hours,
        resilience_hours,
    ) {
        if colony.critical_since.is_none() {
            colony.critical_since = Some(gate.processed_through);
        }
        if crate::idle_rules::should_reset_from_critical_after(
            colony.critical_since,
            gate.processed_through,
            critical_ms,
        ) {
            reset_run(
                colony,
                gate.processed_through,
                RunResetReason::UnattendedCollapse,
            );
            return Some(RunResetReason::UnattendedCollapse);
        }
    } else {
        colony.critical_since = None;
    }

    let previous_water = f64::from_bits(gate.previous_water);
    if previous_water <= 3.0 && colony.resources.water > 6.0 {
        append_event(
            colony,
            gate.processed_through,
            EventKind::ResourceRecovered,
            "Water reserves restored to safe levels.",
        );
    }

    colony.status = crate::idle_rules::next_colony_status(&colony.resources);
    colony.last_tick = gate.processed_through;
    None
}

fn reset_run(colony: &mut ColonyRuntime, now_ms: i64, reason: RunResetReason) {
    let blessings = colony.resources.blessings;
    colony.jobs.clear();
    colony.raiders.clear();
    colony
        .buildings
        .retain(|building| building.construction_progress >= 100);
    for election in &mut colony.elections {
        if election.resolved_at.is_none() {
            election.resolved_at = Some(now_ms);
            election.winner_cat_id = None;
        }
    }
    for cat in colony
        .cats
        .iter_mut()
        .filter(|cat| cat.death_time.is_none())
    {
        cat.needs.hunger = 100.0;
        cat.needs.thirst = 100.0;
        cat.needs.rest = 100.0;
        cat.needs.health = 100.0;
        cat.current_task = None;
        cat.position = Position {
            map: MapType::Colony,
            x: 0.0,
            y: 0.0,
        };
        cat.destination = None;
        cat.carrying = None;
        cat.activity = CatActivity::Idle;
    }

    colony.resources = starting_resources_with_blessings(blessings);
    colony.status = ColonyStatus::Starting;
    colony.leader_id = choose_interim_leader_excluding(colony, None);
    colony.run_number = colony.run_number.saturating_add(1);
    colony.run_started_at = now_ms;
    colony.last_tick = now_ms;
    colony.critical_since = None;
    colony.ritual_requested_at = None;
    colony.threat_pressure = 0.0;
    colony.active_raid = None;
    colony.raid_clicks = 0.0;
    colony.last_raid_at = None;

    append_event(
        colony,
        now_ms,
        EventKind::Reset,
        format!(
            "The colony collapsed and started run {}.",
            colony.run_number
        ),
    );
    append_event(
        colony,
        now_ms,
        EventKind::Other("reset_reason".to_owned()),
        reset_reason_wire(reason),
    );
}

fn starting_resources_with_blessings(blessings: f64) -> Resources {
    Resources {
        food: 150.0,
        water: 100.0,
        herbs: 16.0,
        materials: 24.0,
        refined: 0.0,
        weapons: 0.0,
        armor: 0.0,
        blessings,
    }
}

fn reset_reason_wire(reason: RunResetReason) -> &'static str {
    match reason {
        RunResetReason::AllCatsDead => "all-cats-dead",
        RunResetReason::RaidWipeout => "raid-wipeout",
        RunResetReason::UnattendedCollapse => "unattended-collapse",
    }
}

fn threat_snapshot(colony: &ColonyRuntime, gate: TickGate) -> ThreatSnapshot {
    let alive = alive_cats(&colony.cats).collect::<Vec<_>>();
    ThreatSnapshot {
        wealth: colony_wealth(&colony.resources),
        population: alive.len() as f64,
        warriors: alive
            .iter()
            .filter(|cat| cat.specialization == Some(CatSpecialization::Warrior))
            .count() as f64,
        colony_age_sec: (gate.processed_through
            - if colony.run_started_at > 0 {
                colony.run_started_at
            } else {
                colony.created_at
            })
        .max(0) as f64
            / 1000.0
            * normalize_time_scale(colony),
    }
}

fn spawn_raid(
    colony: &mut ColonyRuntime,
    gate: TickGate,
    snapshot: ThreatSnapshot,
    next_raid_roll: &mut impl FnMut() -> f64,
) {
    let plan = plan_raid(snapshot);
    let gate_pos = raid_gate_position(colony);
    let angle = next_raid_roll() * std::f64::consts::TAU;
    let origin = TilePos {
        x: (f64::from(VILLAGE_ANCHOR.x) + angle.cos() * RAID_SPAWN_DISTANCE).round() as i32,
        y: (f64::from(VILLAGE_ANCHOR.y) + angle.sin() * RAID_SPAWN_DISTANCE).round() as i32,
    };
    let raid_id = format!(
        "raid-{}-{}",
        gate.processed_through,
        colony.raiders.len() + 1
    );
    let count = plan.count.max(0.0).floor() as usize;

    for index in 0..count {
        let jitter_x = match index % 3 {
            0 => 0,
            1 => 1,
            _ => -1,
        };
        let jitter_y = if (index / 3).is_multiple_of(2) { 0 } else { 1 };
        colony.raiders.push(RaiderRuntime {
            id: format!("{raid_id}-raider-{}", index + 1),
            raid_id: raid_id.clone(),
            position: Position {
                map: MapType::World,
                x: f64::from(origin.x + jitter_x),
                y: f64::from(origin.y + jitter_y),
            },
            destination: Some(position_from_world(tile_pos_to_world(gate_pos))),
            attack: plan.strength_each,
            defense: plan.strength_each,
            health: plan.strength_each,
        });
    }

    colony.active_raid = Some(raid_id.clone());
    colony.raid_clicks = 0.0;
    colony.last_raid_at = Some(gate.processed_through);
    colony.threat_pressure = 0.0;
    append_event(
        colony,
        gate.processed_through,
        EventKind::Raid,
        format!(
            "A warband of {} raider{} was spotted advancing on the village!",
            count,
            if count == 1 { "" } else { "s" }
        ),
    );
}

fn apply_banked_raid_clicks(colony: &mut ColonyRuntime, gate: TickGate) {
    let clicks = colony.raid_clicks.floor().max(0.0) as u32;
    if clicks == 0 {
        return;
    }
    let Some(active_raid_id) = colony.active_raid.clone() else {
        colony.raid_clicks = 0.0;
        return;
    };
    let gate_pos = tile_pos_to_world(raid_gate_position(colony));

    for _ in 0..clicks {
        let target = colony
            .raiders
            .iter()
            .enumerate()
            .filter(|(_, raider)| raider.raid_id == active_raid_id && raider.health > 0.0)
            .min_by(|(_, left), (_, right)| {
                cheb_distance_world(position_to_world(left.position), gate_pos)
                    .partial_cmp(&cheb_distance_world(
                        position_to_world(right.position),
                        gate_pos,
                    ))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(index, _)| index);
        let Some(target) = target else {
            break;
        };
        colony.raiders[target].health =
            (colony.raiders[target].health - DEFEND_CLICK_DAMAGE).max(0.0);
    }

    colony.raid_clicks = 0.0;
    if live_raider_indices(colony, &active_raid_id).is_empty() {
        end_raid(colony, &active_raid_id);
        append_event(
            colony,
            gate.processed_through,
            EventKind::Raid,
            "The defenders cut down the raiders before they reached the gate.",
        );
    }
}

fn resolve_active_raid(
    colony: &mut ColonyRuntime,
    gate: TickGate,
    raid_id: &str,
    next_raid_roll: &mut impl FnMut() -> f64,
) {
    let combatants = raid_combatants(colony);
    let effects = resolve_effects(colony.upgrade_tree.owned_node_ids.iter());
    let muster = muster_defense(
        &combatants,
        DefenseStock {
            weapons: colony.resources.weapons,
            armor: colony.resources.armor,
        },
        CombatModifiers {
            combat_power_mult: effects.combat_power_mult,
            defense_mult: effects.defense_mult,
        },
    );
    let raider_power = colony
        .raiders
        .iter()
        .filter(|raider| raider.raid_id == raid_id && raider.health > 0.0)
        .map(|raider| raider.health.max(0.0))
        .sum::<f64>();
    let outcome = resolve_raid(muster.total_power, raider_power, next_raid_roll());

    colony.resources.weapons = (colony.resources.weapons - f64::from(muster.weapons_used)).max(0.0);
    colony.resources.armor = (colony.resources.armor - f64::from(muster.armor_used)).max(0.0);

    if outcome.defenders_win {
        for mustered in &muster.per_cat {
            if let Some(cat) = colony.cats.iter_mut().find(|cat| {
                cat.id == mustered.id && cat.specialization == Some(CatSpecialization::Warrior)
            }) {
                cat.role_xp.warrior += WARRIOR_XP_PER_RAID;
            }
        }
        append_event(
            colony,
            gate.processed_through,
            EventKind::Raid,
            if muster.combatants > 0 {
                format!(
                    "The village guard drove the raiders off at the gate - {} defender{} held the line and the warband broke.",
                    muster.combatants,
                    if muster.combatants == 1 { "" } else { "s" },
                )
            } else {
                "The raiders battered at the fence but found nothing worth the fight and melted back into the wilds.".to_owned()
            },
        );
    } else {
        let stolen = loot_resources(&mut colony.resources, outcome.loot_fraction);
        if outcome.defender_casualties > 0 {
            let victim_id = weakest_mustered_victim(&muster.per_cat)
                .or_else(|| random_alive_cat(colony, next_raid_roll()));
            if let Some(victim_id) = victim_id {
                mark_cat_dead(colony, &victim_id, gate.processed_through);
                let victim_name = colony
                    .cats
                    .iter()
                    .find(|cat| cat.id == victim_id)
                    .map_or("A villager", |cat| cat.name.as_str())
                    .to_owned();
                append_event(
                    colony,
                    gate.processed_through,
                    EventKind::Raid,
                    format!("{victim_name} fell defending the gate as the raiders broke through."),
                );
            }
        }
        append_event(
            colony,
            gate.processed_through,
            EventKind::Raid,
            format!(
                "Raiders overran the fence and made off with {}. The village licks its wounds.",
                loot_line(&stolen)
            ),
        );
    }

    end_raid(colony, raid_id);
}

fn raid_combatants(colony: &ColonyRuntime) -> Vec<MusterCombatant> {
    alive_cats(&colony.cats)
        .filter_map(|cat| {
            let stage = get_life_stage(cat.age_hours);
            (can_work(stage) && can_fight(cat.specialization)).then(|| MusterCombatant {
                id: cat.id.clone(),
                attack: cat.stats.attack,
                defense: cat.stats.defense,
                specialization: cat.specialization,
                warrior_xp: cat.role_xp.warrior,
                life_stage: Some(stage),
            })
        })
        .collect()
}

fn weakest_mustered_victim(mustered: &[crate::warriors::MusteredCat]) -> Option<CatId> {
    mustered
        .iter()
        .min_by(|left, right| {
            left.power
                .partial_cmp(&right.power)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|cat| cat.id.clone())
}

fn random_alive_cat(colony: &ColonyRuntime, roll: f64) -> Option<CatId> {
    let alive = alive_cats(&colony.cats).collect::<Vec<_>>();
    if alive.is_empty() {
        return None;
    }
    let index =
        ((roll.clamp(0.0, 0.999_999) * alive.len() as f64).floor() as usize).min(alive.len() - 1);
    Some(alive[index].id.clone())
}

fn mark_cat_dead(colony: &mut ColonyRuntime, cat_id: &str, now_ms: i64) {
    for job in &mut colony.jobs {
        if job.assigned_cat.as_deref() == Some(cat_id)
            && matches!(job.status, JobStatus::Active | JobStatus::Queued)
        {
            job.status = JobStatus::Cancelled;
            job.completed_at = Some(now_ms);
        }
    }
    for building in &mut colony.buildings {
        if building.assigned_cat.as_deref() == Some(cat_id) {
            building.assigned_cat = None;
        }
    }
    if let Some(cat) = colony
        .cats
        .iter_mut()
        .find(|cat| cat.id == cat_id && cat.death_time.is_none())
    {
        cat.death_time = Some(now_ms);
        cat.current_task = None;
        cat.carrying = None;
        cat.destination = None;
        cat.activity = CatActivity::Idle;
    }
}

fn loot_resources(resources: &mut Resources, loot_fraction: f64) -> Vec<(&'static str, f64)> {
    let mut stolen = Vec::new();
    loot_one("food", &mut resources.food, loot_fraction, &mut stolen);
    loot_one("water", &mut resources.water, loot_fraction, &mut stolen);
    loot_one("herbs", &mut resources.herbs, loot_fraction, &mut stolen);
    loot_one(
        "materials",
        &mut resources.materials,
        loot_fraction,
        &mut stolen,
    );
    loot_one(
        "refined",
        &mut resources.refined,
        loot_fraction,
        &mut stolen,
    );
    stolen
}

fn loot_one(
    label: &'static str,
    store: &mut f64,
    loot_fraction: f64,
    stolen: &mut Vec<(&'static str, f64)>,
) {
    let take = (*store * loot_fraction).floor();
    if take > 0.0 {
        *store -= take;
        stolen.push((label, take));
    }
}

fn loot_line(stolen: &[(&'static str, f64)]) -> String {
    if stolen.is_empty() {
        return "little of value".to_owned();
    }
    stolen
        .iter()
        .map(|(label, amount)| format!("{amount} {label}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn live_raider_indices(colony: &ColonyRuntime, raid_id: &str) -> Vec<usize> {
    colony
        .raiders
        .iter()
        .enumerate()
        .filter_map(|(index, raider)| {
            (raider.raid_id == raid_id && raider.health > 0.0).then_some(index)
        })
        .collect()
}

fn end_raid(colony: &mut ColonyRuntime, raid_id: &str) {
    colony.raiders.retain(|raider| raider.raid_id != raid_id);
    if colony.active_raid.as_deref() == Some(raid_id) {
        colony.active_raid = None;
    }
    colony.raid_clicks = 0.0;
    colony.threat_pressure = 0.0;
}

fn raid_gate_position(colony: &ColonyRuntime) -> TilePos {
    TilePos {
        x: VILLAGE_ANCHOR.x,
        y: VILLAGE_ANCHOR.y + village_ring_radius(colony.buildings.len() as i32),
    }
}

fn cheb_distance_world(left: WorldPos, right: WorldPos) -> f64 {
    (left.x - right.x).abs().max((left.y - right.y).abs())
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
        idle_engine_upgrade_levels(&colony.upgrade_levels),
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

fn village_anchor_world() -> WorldPos {
    WorldPos {
        x: f64::from(VILLAGE_ANCHOR.x),
        y: f64::from(VILLAGE_ANCHOR.y),
    }
}

fn tile_pos_to_world(pos: TilePos) -> WorldPos {
    WorldPos {
        x: f64::from(pos.x),
        y: f64::from(pos.y),
    }
}

fn world_pos_to_tile(pos: WorldPos) -> TilePos {
    TilePos {
        x: pos.x.round() as i32,
        y: pos.y.round() as i32,
    }
}

fn position_from_world(pos: WorldPos) -> Position {
    Position {
        map: MapType::World,
        x: pos.x,
        y: pos.y,
    }
}

fn position_to_world(pos: Position) -> WorldPos {
    match pos.map {
        MapType::World => WorldPos { x: pos.x, y: pos.y },
        MapType::Colony => WorldPos {
            x: pos.x + f64::from(VILLAGE_ANCHOR.x),
            y: pos.y + f64::from(VILLAGE_ANCHOR.y),
        },
    }
}

fn pathfinding_pos(pos: WorldPos) -> pathfinding::WorldPos {
    pathfinding::WorldPos { x: pos.x, y: pos.y }
}

fn movement_pos(pos: pathfinding::WorldPos) -> WorldPos {
    WorldPos { x: pos.x, y: pos.y }
}

fn next_movement_roll(movement: &mut MovementPassContext) -> f64 {
    let roll = roll_seeded(f64::from(movement.movement_seed));
    movement.movement_seed = roll.next_seed;
    roll.value
}

fn claimed_area(colony: &ColonyRuntime) -> crate::village_area::VillageArea {
    let tiles = colony
        .claimed_tiles
        .iter()
        .map(|tile| GridPos {
            x: tile.x,
            y: tile.y,
        })
        .collect::<Vec<_>>();
    from_tiles(&tiles)
}

fn movement_gate(area_gate: Option<AreaGatePlacement>, ring_radius: i32) -> TilePos {
    if let Some(gate) = area_gate {
        let delta = side_delta(gate.side);
        return TilePos {
            x: gate.x + delta.x,
            y: gate.y + delta.y,
        };
    }

    TilePos {
        x: VILLAGE_ANCHOR.x,
        y: VILLAGE_ANCHOR.y + ring_radius,
    }
}

fn pathfinding_area(area: &crate::village_area::VillageArea) -> pathfinding::VillageArea {
    area.iter()
        .map(|key| {
            let pos = crate::village_area::pos_of(key);
            PathTilePos { x: pos.x, y: pos.y }
        })
        .collect()
}

fn pathfinding_gate(gate: AreaGatePlacement) -> PathGatePlacement {
    PathGatePlacement {
        x: gate.x,
        y: gate.y,
        side: match gate.side {
            Side::N => pathfinding::FenceSide::N,
            Side::E => pathfinding::FenceSide::E,
            Side::S => pathfinding::FenceSide::S,
            Side::W => pathfinding::FenceSide::W,
        },
    }
}

fn walk_tile_from_runtime(tile: &WorldTileRuntime) -> WalkTile {
    WalkTile {
        x: tile.pos.x,
        y: tile.pos.y,
        tile_type: walk_tile_type(tile.tile_type),
        overlay_feature: tile.overlay_feature.as_deref().map(walk_overlay_feature),
        resources: Some(WalkTileResources {
            water: tile.resources.water,
        }),
        path_wear: tile.path_wear,
    }
}

fn walk_tile_type(tile_type: TileType) -> WalkTileType {
    match tile_type {
        TileType::River => WalkTileType::River,
        TileType::DenseWoods => WalkTileType::DenseWoods,
        tile_type if is_forest_type(tile_type) => WalkTileType::Forest,
        _ => WalkTileType::Other,
    }
}

fn walk_overlay_feature(overlay: &str) -> WalkOverlayFeature {
    match overlay {
        "river" => WalkOverlayFeature::River,
        "road_built" => WalkOverlayFeature::RoadBuilt,
        "game_trail" => WalkOverlayFeature::GameTrail,
        "ancient_road" => WalkOverlayFeature::AncientRoad,
        "trade_route" => WalkOverlayFeature::TradeRoute,
        _ => WalkOverlayFeature::Other,
    }
}

fn movement_speed_bonus(tile: Option<&WorldTileRuntime>) -> f64 {
    let Some(tile) = tile else {
        return 0.0;
    };
    if tile.overlay_feature.as_deref() == Some("road_built") {
        0.6
    } else {
        get_path_speed_bonus(tile.path_wear)
    }
}

fn get_path_speed_bonus(path_wear: u32) -> f64 {
    if path_wear < 30 {
        0.0
    } else if path_wear < 60 {
        0.1
    } else if path_wear < 90 {
        0.25
    } else {
        0.4
    }
}

fn add_path_wear(current_wear: u32, amount: u32) -> u32 {
    current_wear.saturating_add(amount).min(100)
}

fn wander_anchor(colony: &ColonyRuntime, cat_id: &str) -> WorldPos {
    colony
        .buildings
        .iter()
        .find(|building| building.assigned_cat.as_deref() == Some(cat_id))
        .map_or_else(village_anchor_world, |building| {
            tile_pos_to_world(building.position)
        })
}

fn is_inside_movement_village(pos: TilePos, movement: &MovementPassContext) -> bool {
    if movement.claimed_area.is_empty() {
        return cheb_from_anchor(pos) < movement.ring_radius;
    }
    is_inside_village(GridPos { x: pos.x, y: pos.y }, &movement.claimed_area)
}

fn unaccepted_active_job_site(colony: &ColonyRuntime, cat_id: &str) -> Option<(usize, TilePos)> {
    colony.jobs.iter().enumerate().find_map(|(index, job)| {
        if job.status != JobStatus::Active || job.assigned_cat.as_deref() != Some(cat_id) {
            return None;
        }
        match job.metadata {
            JobMetadata::Hauling {
                site: Some(site),
                accepted: false,
                ..
            }
            | JobMetadata::Site {
                site,
                accepted: false,
            } => Some((index, site)),
            JobMetadata::Expansion {
                target,
                accepted: false,
            } => Some((index, target)),
            _ => None,
        }
    })
}

fn accept_job(colony: &mut ColonyRuntime, job_index: usize) {
    let metadata = colony.jobs[job_index].metadata.clone();
    colony.jobs[job_index].metadata = match metadata {
        JobMetadata::Hauling {
            site,
            total_yield,
            trips_done,
            next_trip_at,
            ..
        } => JobMetadata::Hauling {
            site,
            total_yield,
            trips_done,
            next_trip_at,
            accepted: true,
        },
        JobMetadata::Site { site, .. } => JobMetadata::Site {
            site,
            accepted: true,
        },
        JobMetadata::Expansion { target, .. } => JobMetadata::Expansion {
            target,
            accepted: true,
        },
        other => other,
    };
}

fn reveal_and_wear_walked_tiles(
    colony: &mut ColonyRuntime,
    movement: &MovementPassContext,
    walked: &[WorldPos],
    current_task: Option<TaskType>,
) {
    if walked.is_empty() {
        return;
    }

    let reveal_radius = if current_task == Some(TaskType::Explore) {
        2
    } else {
        1
    };
    let walked_tiles = walked
        .iter()
        .map(|pos| world_pos_to_tile(*pos))
        .collect::<Vec<_>>();
    let walked_keys = walked_tiles.iter().copied().collect::<HashSet<_>>();
    let min_x = walked_tiles.iter().map(|pos| pos.x).min().unwrap_or(0) - reveal_radius;
    let max_x = walked_tiles.iter().map(|pos| pos.x).max().unwrap_or(0) + reveal_radius;
    let min_y = walked_tiles.iter().map(|pos| pos.y).min().unwrap_or(0) - reveal_radius;
    let max_y = walked_tiles.iter().map(|pos| pos.y).max().unwrap_or(0) + reveal_radius;

    for x in min_x..=max_x {
        for y in min_y..=max_y {
            let pos = TilePos { x, y };
            if is_inside_movement_village(pos, movement) {
                continue;
            }
            let Some(tile) = colony.world_tiles.get_mut(&pos) else {
                continue;
            };
            if walked_keys.contains(&pos) {
                tile.path_wear = add_path_wear(tile.path_wear, WALK_WEAR).max(64);
            } else if walked_tiles.iter().any(|walked| {
                (walked.x - pos.x).abs().max((walked.y - pos.y).abs()) <= reveal_radius
            }) {
                tile.path_wear = tile.path_wear.max(63);
            }
        }
    }
}

fn scaffold_building_type(building_type: BuildingType) -> BuildingType {
    match building_type {
        BuildingType::Workshop
        | BuildingType::Field
        | BuildingType::FoodStorage
        | BuildingType::Smithy
        | BuildingType::Barracks => building_type,
        _ => BuildingType::Den,
    }
}

fn next_claimed_building_site(colony: &ColonyRuntime, roll: f64) -> Option<TilePos> {
    let occupied = colony
        .buildings
        .iter()
        .map(|building| building.position)
        .collect::<Vec<_>>();
    let mut free = colony
        .claimed_tiles
        .iter()
        .copied()
        .filter(|site| {
            *site
                != TilePos {
                    x: VILLAGE_ANCHOR.x,
                    y: VILLAGE_ANCHOR.y,
                }
                && !occupied.contains(site)
                && !tile_has_water(colony.world_tiles.get(site))
        })
        .collect::<Vec<_>>();
    free.sort_by_key(|site| (site.y, site.x));

    if free.is_empty() {
        let occupied_local = occupied
            .iter()
            .map(|site| {
                world_to_colony(GridPos {
                    x: site.x,
                    y: site.y,
                })
            })
            .collect::<Vec<_>>();
        return crate::village_layout::next_building_site_default(&occupied_local, roll)
            .map(colony_to_world)
            .map(|site| TilePos {
                x: site.x,
                y: site.y,
            })
            .filter(|site| !tile_has_water(colony.world_tiles.get(site)));
    }

    let clamped = roll.clamp(0.0, 0.999_999);
    Some(free[(clamped * free.len() as f64).floor() as usize])
}

fn tile_has_water(tile: Option<&WorldTileRuntime>) -> bool {
    tile.is_some_and(|tile| {
        tile.tile_type == TileType::River
            || tile.overlay_feature.as_deref() == Some("river")
            || tile.resources.water > 0
    })
}

fn job_has_destination_metadata(job: &JobRuntime) -> bool {
    match job.metadata {
        JobMetadata::Hauling {
            site: Some(_),
            accepted: _,
            ..
        }
        | JobMetadata::Site { .. }
        | JobMetadata::Expansion { accepted: _, .. } => true,
        JobMetadata::Construction { site: Some(_), .. } => job.kind == JobKind::BuildHouse,
        _ => false,
    }
}

fn food_tiles_near_village(colony: &ColonyRuntime) -> Vec<WorldPos> {
    colony
        .world_tiles
        .values()
        .filter(|tile| {
            tile.resources.food >= 25 && tile_is_explored(tile) && cheb_from_anchor(tile.pos) > 4
        })
        .map(|tile| tile_pos_to_world(tile.pos))
        .collect()
}

fn quarry_sites_near_village(colony: &ColonyRuntime) -> Vec<WorldPos> {
    let mut sites = colony
        .world_tiles
        .values()
        .filter(|tile| {
            matches!(tile.tile_type, TileType::Mountains | TileType::CaveEntrance)
                && tile_is_explored(tile)
        })
        .map(|tile| tile.pos)
        .collect::<Vec<_>>();
    sites.sort_by_key(|site| cheb_from_anchor(*site));
    sites.into_iter().map(tile_pos_to_world).collect()
}

fn water_sites_near_village(colony: &ColonyRuntime) -> Vec<WorldPos> {
    let mut sites = colony
        .world_tiles
        .values()
        .filter(|tile| tile_has_water(Some(tile)) && tile_is_explored(tile))
        .map(|tile| tile.pos)
        .collect::<Vec<_>>();
    sites.sort_by_key(|site| cheb_from_anchor(*site));
    sites.into_iter().map(tile_pos_to_world).collect()
}

fn frontier_tiles_near_village(colony: &ColonyRuntime) -> Vec<WorldPos> {
    let mut sites = colony
        .world_tiles
        .values()
        .filter(|tile| !tile_is_explored(tile))
        .map(|tile| tile.pos)
        .collect::<Vec<_>>();
    sites.sort_by_key(|site| cheb_from_anchor(*site));
    sites.into_iter().map(tile_pos_to_world).collect()
}

fn tile_is_explored(tile: &WorldTileRuntime) -> bool {
    tile.path_wear > 62 || cheb_from_anchor(tile.pos) <= 6
}

fn cheb_from_anchor(pos: TilePos) -> i32 {
    (pos.x - VILLAGE_ANCHOR.x)
        .abs()
        .max((pos.y - VILLAGE_ANCHOR.y).abs())
}

fn auto_staff_idle_buildings(colony: &mut ColonyRuntime, building_type: BuildingType, now_ms: i64) {
    let mut open_buildings = buildings_needing_workers(colony, building_type);
    if open_buildings.is_empty() {
        return;
    }
    let busy_ids = active_or_queued_jobs(colony)
        .iter()
        .filter_map(|job| job.assigned_cat.as_deref())
        .collect::<Vec<_>>();
    let assigned_building_ids = colony
        .buildings
        .iter()
        .filter_map(|building| building.assigned_cat.as_deref())
        .collect::<Vec<_>>();
    let cats = colony
        .cats
        .iter()
        .filter(|cat| {
            can_take_new_job_with_busy(cat, &busy_ids)
                && !assigned_building_ids.contains(&cat.id.as_str())
        })
        .map(|cat| cat.id.clone())
        .collect::<Vec<_>>();

    for cat_id in cats {
        let Some(building_id) = open_buildings.pop() else {
            break;
        };
        staff_building(colony, &building_id, &cat_id, now_ms);
    }
}

fn assigned_worker<'a>(colony: &'a ColonyRuntime, building_id: &str) -> Option<&'a Cat> {
    let assigned_cat = colony
        .buildings
        .iter()
        .find(|building| building.id == building_id)
        .and_then(|building| building.assigned_cat.as_deref())?;
    colony
        .cats
        .iter()
        .find(|cat| cat.id == assigned_cat && cat.death_time.is_none())
}

fn research_workforce(_: &ColonyRuntime) -> f64 {
    0.0
}

fn due_active_jobs(colony: &ColonyRuntime, gate: TickGate) -> Vec<JobRuntime> {
    colony
        .jobs
        .iter()
        .filter(|job| {
            job.status == JobStatus::Active
                && job
                    .ends_at
                    .is_some_and(|ends_at| ends_at <= gate.processed_through)
        })
        .cloned()
        .collect()
}

fn complete_hunt(colony: &mut ColonyRuntime, job: &JobRuntime, gate: TickGate) {
    let Some(cat_index) = assigned_alive_cat_index(colony, job) else {
        return;
    };
    let (site, total_yield, trips_done) = hauling_metadata(job);
    let total = total_yield.unwrap_or_else(|| hunt_yield_for(&colony.cats[cat_index], colony));
    let reward = remaining_yield(total, HUNT_TRIP_COUNT, trips_done as i32);
    if let Some(site) = site {
        drain_hunt_site(colony, site, reward, gate.processed_through);
    }

    let cat = &mut colony.cats[cat_index];
    cat.role_xp.hunter += 1.0;
    cat.gain_skill(Labor::Hunt, SKILL_GAIN_PER_JOB);
    cat.specialization = idle_engine::next_specialization(
        CatSpecialization::Hunter,
        cat.role_xp.hunter,
        cat.specialization,
    );
    cat.stats.hunting = (cat.stats.hunting + 0.4).min(100.0);
    cat.carrying = (reward > 0.0).then_some(Carrying {
        kind: CarryingKind::Food,
        amount: reward,
        job_ended_at: gate.processed_through,
    });
}

fn complete_fixed_yield_job(
    colony: &mut ColonyRuntime,
    job: &JobRuntime,
    gate: TickGate,
    total: f64,
    kind: CarryingKind,
) {
    let Some(cat_index) = assigned_alive_cat_index(colony, job) else {
        return;
    };
    let (_, total_yield, trips_done) = hauling_metadata(job);
    // Reuse the total cached by an earlier haul trip so skill scaling is applied once;
    // otherwise scale the base constant by this cat's labor skill here.
    let scaled_total =
        total_yield.unwrap_or_else(|| skill_scaled_yield(&colony.cats[cat_index], job.kind, total));
    let reward = remaining_yield(scaled_total, HUNT_TRIP_COUNT, trips_done as i32);
    let cat = &mut colony.cats[cat_index];
    if let Some(labor) = Labor::for_job_kind(job.kind) {
        cat.gain_skill(labor, SKILL_GAIN_PER_JOB);
    }
    cat.carrying = (reward > 0.0).then_some(Carrying {
        kind,
        amount: reward,
        job_ended_at: gate.processed_through,
    });
}

fn complete_village_expansion(colony: &mut ColonyRuntime, job: &JobRuntime, gate: TickGate) {
    let target = match job.metadata {
        JobMetadata::Expansion { target, .. } | JobMetadata::Site { site: target, .. } => target,
        _ => return,
    };
    if colony.claimed_tiles.contains(&target)
        || !is_adjacent_to_claimed(colony, target)
        || tile_has_water(colony.world_tiles.get(&target))
    {
        return;
    }
    colony.claimed_tiles.push(target);
    clear_claimed_forest_tile(colony, target, gate.processed_through);
    append_event(
        colony,
        gate.processed_through,
        EventKind::Other("village_expanded".to_owned()),
        format!(
            "The village claimed new ground at ({}, {}).",
            target.x, target.y
        ),
    );
}

fn complete_build(colony: &mut ColonyRuntime, job: &JobRuntime, gate: TickGate) {
    let Some(cat_index) = assigned_alive_cat_index(colony, job) else {
        return;
    };

    match job.metadata {
        JobMetadata::Construction {
            phase: ConstructionPhase::ConstructHouse,
            ref building_id,
            ..
        } => {
            if let Some(building_id) = building_id
                && let Some(building) = colony
                    .buildings
                    .iter_mut()
                    .find(|building| building.id == *building_id)
            {
                building.construction_progress = 100;
                building.is_complete = true;
            }
            colony.automation_tier =
                ((colony.automation_tier + 0.05).min(10.0) * 100.0).round() / 100.0;
        }
        _ => {
            colony.resources.materials += 12.0;
            chop_nearest_explored_forest(colony, gate.processed_through);
        }
    }

    let cat = &mut colony.cats[cat_index];
    cat.role_xp.architect += 1.0;
    cat.gain_skill(Labor::Build, SKILL_GAIN_PER_JOB);
    cat.specialization = idle_engine::next_specialization(
        CatSpecialization::Architect,
        cat.role_xp.architect,
        cat.specialization,
    );
    cat.stats.building = (cat.stats.building + 0.4).min(100.0);
}

fn complete_ritual(colony: &mut ColonyRuntime, job: &JobRuntime, gate: TickGate) {
    let Some(cat_index) = assigned_alive_cat_index(colony, job) else {
        return;
    };
    let blessings = 1.0 + f64::from(colony.upgrade_levels.ritual_mastery / 3);
    let cat = &mut colony.cats[cat_index];
    cat.role_xp.ritualist += 1.0;
    cat.gain_skill(Labor::Ritual, SKILL_GAIN_PER_JOB);
    cat.specialization = idle_engine::next_specialization(
        CatSpecialization::Ritualist,
        cat.role_xp.ritualist,
        cat.specialization,
    );
    cat.carrying = Some(Carrying {
        kind: CarryingKind::Blessings,
        amount: blessings,
        job_ended_at: gate.processed_through,
    });
}

fn complete_warrior_training(colony: &mut ColonyRuntime, job: &JobRuntime, _: TickGate) {
    let Some(cat_index) = assigned_alive_cat_index(colony, job) else {
        return;
    };
    let cat = &mut colony.cats[cat_index];
    cat.specialization = Some(CatSpecialization::Warrior);
    cat.role_xp.warrior += 1.0;
    cat.gain_skill(Labor::Fight, SKILL_GAIN_PER_JOB);
    cat.stats.attack = (cat.stats.attack + 3.0).min(100.0);
    cat.stats.defense = (cat.stats.defense + 3.0).min(100.0);
    cat.activity = CatActivity::Idle;
    cat.current_task = None;
}

fn return_assigned_cat(colony: &mut ColonyRuntime, job: &JobRuntime, gate: TickGate) {
    let Some(cat_index) = assigned_alive_cat_index(colony, job) else {
        return;
    };
    let destination = if job.kind == JobKind::BuildHouse {
        let first = roll_seeded(f64::from(movement_seed(colony.test_rng_seed.unwrap_or(1))));
        let second = roll_seeded(f64::from(first.next_seed));
        pick_wander_target(village_anchor_world(), first.value, second.value)
    } else {
        village_anchor_world()
    };
    let cat = &mut colony.cats[cat_index];
    cat.destination = Some(position_from_world(destination));
    cat.activity = CatActivity::Returning;
    cat.current_task = None;
    if job.kind == JobKind::TrainWarrior {
        append_event(
            colony,
            gate.processed_through,
            EventKind::Other("warrior_trained".to_owned()),
            "A recruit completed warrior training and joined the village guard.",
        );
    }
}

fn assigned_alive_cat_index(colony: &ColonyRuntime, job: &JobRuntime) -> Option<usize> {
    let cat_id = job.assigned_cat.as_deref()?;
    colony
        .cats
        .iter()
        .position(|cat| cat.id == cat_id && cat.death_time.is_none())
}

fn hauling_metadata(job: &JobRuntime) -> (Option<TilePos>, Option<f64>, u32) {
    match job.metadata {
        JobMetadata::Hauling {
            site,
            total_yield,
            trips_done,
            ..
        } => (site, total_yield, trips_done),
        JobMetadata::Site { site, .. } => (Some(site), None, 0),
        _ => (None, None, 0),
    }
}

fn hunt_yield_for(cat: &Cat, colony: &ColonyRuntime) -> f64 {
    let effects = resolve_effects(colony.upgrade_tree.owned_node_ids.iter());
    let base = idle_engine::get_hunt_reward(
        cat.stats.hunting,
        cat.specialization,
        cat.role_xp.hunter,
        idle_engine_upgrade_levels(&colony.upgrade_levels),
    );
    let stage_mult = crate::life_sim::stage_work_effectiveness(get_life_stage(cat.age_hours));
    // Hunt yield rides the continuous Hunt skill (P12.1). `role_xp.hunter` still gates
    // the specialist bonus inside `get_hunt_reward`; both increment +1 per hunt so this
    // is identical to the pre-P12.1 `trade_yield_multiplier(role_xp.hunter)` at parity.
    let yield_mult = crate::life_sim::trade_yield_multiplier(cat.skill(Labor::Hunt));
    (base * stage_mult * yield_mult * effects.hunt_yield_mult.max(0.0))
        .floor()
        .max(1.0)
}

/// Scale a job's base yield by the cat's continuous skill in that labor. Hunt yield is
/// handled by [`hunt_yield_for`] (which folds in life-stage + upgrade effects), so this
/// is only for the fixed-yield gathering jobs (quarry / fetch-water). At skill 0 the
/// multiplier is 1.0, so whole-number base yields are returned unchanged.
fn skill_scaled_yield(cat: &Cat, kind: JobKind, base: f64) -> f64 {
    match Labor::for_job_kind(kind) {
        Some(labor) => (base * crate::life_sim::trade_yield_multiplier(cat.skill(labor))).floor(),
        None => base,
    }
}

fn total_yield_for_job(colony: &ColonyRuntime, job: &JobRuntime, cat_index: usize) -> f64 {
    let cat = &colony.cats[cat_index];
    match job.kind {
        JobKind::FetchWater => skill_scaled_yield(cat, job.kind, WATER_TOTAL_YIELD),
        JobKind::Quarry => skill_scaled_yield(cat, job.kind, QUARRY_TOTAL_YIELD),
        JobKind::HuntExpedition => hunt_yield_for(cat, colony),
        _ => 0.0,
    }
}

fn carrying_kind_for_job(kind: JobKind) -> CarryingKind {
    match kind {
        JobKind::FetchWater => CarryingKind::Water,
        JobKind::Quarry => CarryingKind::Materials,
        _ => CarryingKind::Food,
    }
}

fn drain_hunt_site(colony: &mut ColonyRuntime, site: TilePos, amount: f64, now_ms: i64) {
    if amount <= 0.0 {
        return;
    }
    if let Some(tile) = colony.world_tiles.get_mut(&site) {
        tile.resources.food = tile.resources.food.saturating_sub(amount.floor() as u32);
        tile.last_depleted = now_ms;
    }
}

fn is_adjacent_to_claimed(colony: &ColonyRuntime, target: TilePos) -> bool {
    [
        TilePos {
            x: target.x + 1,
            y: target.y,
        },
        TilePos {
            x: target.x - 1,
            y: target.y,
        },
        TilePos {
            x: target.x,
            y: target.y + 1,
        },
        TilePos {
            x: target.x,
            y: target.y - 1,
        },
    ]
    .iter()
    .any(|neighbour| colony.claimed_tiles.contains(neighbour))
}

fn clear_claimed_forest_tile(colony: &mut ColonyRuntime, target: TilePos, now_ms: i64) {
    if let Some(tile) = colony.world_tiles.get_mut(&target)
        && is_forest_type(tile.tile_type)
    {
        tile.tile_type = TileType::Field;
        tile.resources.food = 0;
        tile.resources.herbs = 0;
        tile.max_resources.food = CHOPPED_FOREST_FOOD_CAP as u32;
        tile.last_depleted = now_ms;
    }
}

fn chop_nearest_explored_forest(colony: &mut ColonyRuntime, now_ms: i64) {
    let nearest = colony
        .world_tiles
        .values()
        .filter(|tile| is_forest_type(tile.tile_type) && tile_is_explored(tile))
        .map(|tile| tile.pos)
        .min_by_key(|site| cheb_from_anchor(*site));
    if let Some(site) = nearest {
        clear_claimed_forest_tile(colony, site, now_ms);
        append_event(
            colony,
            now_ms,
            EventKind::Other("forest_chopped".to_owned()),
            format!(
                "A forest at ({}, {}) was chopped for lumber.",
                site.x, site.y
            ),
        );
    }
}

fn credit_carrying(colony: &mut ColonyRuntime, carrying: &Carrying) {
    match carrying.kind {
        CarryingKind::Food => colony.resources.food += carrying.amount,
        CarryingKind::Materials => colony.resources.materials += carrying.amount,
        CarryingKind::Water => colony.resources.water += carrying.amount,
        CarryingKind::Blessings => colony.global_upgrade_points += carrying.amount,
    }
}

fn deposit_message(cat_id: &str, carrying: &Carrying) -> String {
    match carrying.kind {
        CarryingKind::Food => format!("{cat_id} delivered {} food to the shrine.", carrying.amount),
        CarryingKind::Materials => {
            format!(
                "{cat_id} hauled {} materials to the shrine.",
                carrying.amount
            )
        }
        CarryingKind::Water => {
            format!("{cat_id} carried {} water to the shrine.", carrying.amount)
        }
        CarryingKind::Blessings => {
            format!(
                "{cat_id}'s ritual beamed {} blessings up to the players.",
                carrying.amount
            )
        }
    }
}

fn active_site_for_carrier(colony: &ColonyRuntime, cat_id: &str, now_ms: i64) -> Option<TilePos> {
    colony.jobs.iter().find_map(|job| {
        if job.status != JobStatus::Active
            || job.assigned_cat.as_deref() != Some(cat_id)
            || job.ends_at.is_some_and(|ends_at| ends_at <= now_ms)
            || !matches!(
                job.kind,
                JobKind::HuntExpedition | JobKind::Quarry | JobKind::FetchWater
            )
        {
            return None;
        }
        match job.metadata {
            JobMetadata::Hauling {
                site: Some(site),
                accepted: true,
                ..
            } => Some(site),
            _ => None,
        }
    })
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

    #[test]
    fn queued_hunt_promotion_assigns_anchor_destination_and_food_site() {
        let mut world = WorldState {
            world_seed: 123,
            colonies: vec![ColonyRuntime {
                id: "colony-1".to_owned(),
                name: "MossClan".to_owned(),
                resources: plentiful_resources(),
                cats: vec![adult_idle_cat("hunter", "colony-1")],
                jobs: vec![JobRuntime {
                    id: "hunt-1".to_owned(),
                    kind: JobKind::HuntExpedition,
                    status: JobStatus::Queued,
                    assigned_cat: Some("hunter".to_owned()),
                    duration_ms: 60_000,
                    created_at: 0,
                    started_at: None,
                    ends_at: Some(600_000),
                    ..JobRuntime::default()
                }],
                world_tiles: BTreeMap::from([(
                    pos(12, 6),
                    WorldTileRuntime {
                        resources: TileResources {
                            food: 30,
                            herbs: 0,
                            water: 0,
                        },
                        path_wear: 63,
                        ..tile(12, 6, 63, None)
                    },
                )]),
                last_tick: 0,
                test_rng_seed: Some(12_345),
                ..ColonyRuntime::default()
            }],
        };

        let _ = world_tick(&mut world, 1_000);

        let colony = &world.colonies[0];
        let job = &colony.jobs[0];
        assert_eq!(job.status, JobStatus::Active);
        assert_eq!(job.started_at, Some(1_000));
        assert_eq!(
            job.metadata,
            JobMetadata::Hauling {
                site: Some(pos(12, 6)),
                total_yield: None,
                trips_done: 0,
                next_trip_at: None,
                accepted: true,
            }
        );
        assert_eq!(
            colony.cats[0].destination,
            Some(Position {
                map: MapType::World,
                x: 12.0,
                y: 6.0,
            })
        );
        assert_eq!(colony.cats[0].activity, CatActivity::Traveling);
        assert_eq!(colony.cats[0].current_task, Some(TaskType::Hunt));
    }

    #[test]
    fn mid_job_hunt_haul_splits_total_and_sets_carrying() {
        let mut cat = adult_idle_cat("hunter", "colony-1");
        cat.activity = CatActivity::Working;
        cat.current_task = Some(TaskType::Hunt);
        cat.position = Position {
            map: MapType::World,
            x: 12.0,
            y: 6.0,
        };

        let mut world = WorldState {
            world_seed: 123,
            colonies: vec![ColonyRuntime {
                id: "colony-1".to_owned(),
                name: "MossClan".to_owned(),
                resources: plentiful_resources(),
                cats: vec![cat],
                jobs: vec![JobRuntime {
                    id: "hunt-1".to_owned(),
                    kind: JobKind::HuntExpedition,
                    status: JobStatus::Active,
                    assigned_cat: Some("hunter".to_owned()),
                    duration_ms: 9_000,
                    created_at: 0,
                    started_at: Some(0),
                    ends_at: Some(9_000),
                    metadata: JobMetadata::Hauling {
                        site: Some(pos(12, 6)),
                        total_yield: Some(10.0),
                        trips_done: 0,
                        next_trip_at: Some(3_000),
                        accepted: true,
                    },
                    ..JobRuntime::default()
                }],
                world_tiles: BTreeMap::from([(
                    pos(12, 6),
                    WorldTileRuntime {
                        resources: TileResources {
                            food: 30,
                            herbs: 0,
                            water: 0,
                        },
                        path_wear: 63,
                        ..tile(12, 6, 63, None)
                    },
                )]),
                last_tick: 2_000,
                test_rng_seed: Some(12_345),
                ..ColonyRuntime::default()
            }],
        };

        let _ = world_tick(&mut world, 3_000);

        let colony = &world.colonies[0];
        assert_eq!(
            colony.jobs[0].metadata,
            JobMetadata::Hauling {
                site: Some(pos(12, 6)),
                total_yield: Some(10.0),
                trips_done: 1,
                next_trip_at: Some(6_000),
                accepted: true,
            }
        );
        assert_eq!(
            colony.cats[0].carrying,
            Some(Carrying {
                kind: CarryingKind::Food,
                amount: 4.0,
                job_ended_at: 3_000,
            })
        );
        assert_eq!(colony.cats[0].activity, CatActivity::Returning);
        assert_eq!(
            colony.cats[0].destination,
            Some(Position {
                map: MapType::World,
                x: 6.0,
                y: 6.0,
            })
        );
        assert_eq!(colony.world_tiles[&pos(12, 6)].resources.food, 26);
    }

    // ---- P12.1 skills ----

    #[test]
    fn skill_scaled_yield_is_monotonic_and_zero_matches_base() {
        let mut cat = adult_idle_cat("q", "colony-1");
        // At skill 0 the whole-number quarry base is returned unchanged.
        assert_eq!(
            skill_scaled_yield(&cat, JobKind::Quarry, QUARRY_TOTAL_YIELD),
            QUARRY_TOTAL_YIELD
        );

        cat.gain_skill(Labor::Quarry, 30.0);
        let mid = skill_scaled_yield(&cat, JobKind::Quarry, QUARRY_TOTAL_YIELD);
        assert!(mid > QUARRY_TOTAL_YIELD);

        cat.gain_skill(Labor::Quarry, 100_000.0);
        let high = skill_scaled_yield(&cat, JobKind::Quarry, QUARRY_TOTAL_YIELD);
        assert!(high >= mid);
        // Bounded by the 1.4x yield asymptote.
        assert!(high <= (QUARRY_TOTAL_YIELD * 1.4).ceil());

        // A labor-less job kind is never scaled.
        assert_eq!(
            skill_scaled_yield(&cat, JobKind::Explore, QUARRY_TOTAL_YIELD),
            QUARRY_TOTAL_YIELD
        );
    }

    #[test]
    fn hunt_yield_rides_hunt_skill_monotonically() {
        let colony = ColonyRuntime {
            cats: vec![adult_idle_cat("h", "colony-1")],
            ..ColonyRuntime::default()
        };
        let base = hunt_yield_for(&colony.cats[0], &colony);

        let mut skilled = colony.cats[0].clone();
        skilled.gain_skill(Labor::Hunt, 60.0);
        let boosted = hunt_yield_for(&skilled, &colony);
        assert!(boosted > base);

        let mut more = skilled.clone();
        more.gain_skill(Labor::Hunt, 100_000.0);
        assert!(hunt_yield_for(&more, &colony) >= boosted);
    }

    fn hauling_hunt_world(trips_done: u32, ends_at: i64) -> WorldState {
        let mut cat = adult_idle_cat("hunter", "colony-1");
        cat.activity = CatActivity::Working;
        cat.current_task = Some(TaskType::Hunt);
        cat.position = Position {
            map: MapType::World,
            x: 12.0,
            y: 6.0,
        };
        WorldState {
            world_seed: 123,
            colonies: vec![ColonyRuntime {
                id: "colony-1".to_owned(),
                name: "MossClan".to_owned(),
                resources: plentiful_resources(),
                cats: vec![cat],
                jobs: vec![JobRuntime {
                    id: "hunt-1".to_owned(),
                    kind: JobKind::HuntExpedition,
                    status: JobStatus::Active,
                    assigned_cat: Some("hunter".to_owned()),
                    duration_ms: 9_000,
                    created_at: 0,
                    started_at: Some(0),
                    ends_at: Some(ends_at),
                    metadata: JobMetadata::Hauling {
                        site: Some(pos(12, 6)),
                        total_yield: Some(10.0),
                        trips_done,
                        next_trip_at: Some(3_000),
                        accepted: true,
                    },
                    ..JobRuntime::default()
                }],
                world_tiles: BTreeMap::from([(
                    pos(12, 6),
                    WorldTileRuntime {
                        resources: TileResources {
                            food: 30,
                            herbs: 0,
                            water: 0,
                        },
                        path_wear: 63,
                        ..tile(12, 6, 63, None)
                    },
                )]),
                last_tick: 2_000,
                test_rng_seed: Some(12_345),
                ..ColonyRuntime::default()
            }],
        }
    }

    #[test]
    fn a_mid_job_haul_trip_grants_only_haul_skill() {
        // A trip runs (trips_done 0 < HUNT_TRIP_COUNT-1) but the job stays active.
        let mut world = hauling_hunt_world(0, 9_000);
        let _ = world_tick(&mut world, 3_000);

        let cat = &world.colonies[0].cats[0];
        assert_eq!(cat.skill(Labor::Haul), HAUL_SKILL_GAIN);
        assert_eq!(cat.skills.len(), 1);
        assert!(!cat.skills.contains_key(&Labor::Hunt));
    }

    #[test]
    fn completing_a_hunt_grants_hunt_skill_deterministically() {
        // trips_done at the last trip so only completion runs (grants Hunt, not Haul).
        let mut a = hauling_hunt_world(HUNT_TRIP_COUNT as u32 - 1, 1_000);
        let mut b = hauling_hunt_world(HUNT_TRIP_COUNT as u32 - 1, 1_000);
        let _ = world_tick(&mut a, 3_000);
        let _ = world_tick(&mut b, 3_000);

        let cat = &a.colonies[0].cats[0];
        assert_eq!(cat.skill(Labor::Hunt), SKILL_GAIN_PER_JOB);
        assert!(!cat.skills.contains_key(&Labor::Haul));
        // Same seed + inputs → identical cat state (skills included).
        assert_eq!(a.colonies[0].cats, b.colonies[0].cats);
    }

    #[test]
    fn skill_shortens_queued_job_duration() {
        // Queue a quarry through the tick path for a novice vs a skilled cat and
        // confirm the skilled cat's job ends sooner.
        fn quarry_duration(skill: f64) -> i64 {
            let mut cat = adult_idle_cat("miner", "colony-1");
            cat.gain_skill(Labor::Quarry, skill);
            let mut colony = ColonyRuntime {
                id: "colony-1".to_owned(),
                cats: vec![cat],
                last_tick: 0,
                ..ColonyRuntime::default()
            };
            queue_job(
                &mut colony,
                0,
                JobKind::Quarry,
                Some("miner".to_owned()),
                JobMetadata::None,
            );
            colony.jobs[0].duration_ms
        }

        let novice = quarry_duration(0.0);
        let expert = quarry_duration(200.0);
        assert!(expert < novice, "expert={expert} novice={novice}");
    }

    #[test]
    fn movement_advances_toward_destination_and_wears_traversed_tiles() {
        let mut cat = adult_idle_cat("walker", "colony-1");
        cat.position = Position {
            map: MapType::World,
            x: 10.0,
            y: 6.0,
        };
        cat.destination = Some(Position {
            map: MapType::World,
            x: 12.0,
            y: 6.0,
        });
        cat.activity = CatActivity::Traveling;

        let mut world_tiles = BTreeMap::new();
        for x in 9..=13 {
            for y in 5..=7 {
                world_tiles.insert(pos(x, y), tile(x, y, 0, None));
            }
        }

        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            resources: plentiful_resources(),
            cats: vec![cat],
            world_tiles,
            test_rng_seed: Some(123),
            ..ColonyRuntime::default()
        };
        let movement = MovementPassContext {
            movement_seed: movement_seed(123),
            movement_elapsed: 8.0,
            wander_chance: 0.0,
            ring_radius: 4,
            claimed_area: Default::default(),
            area_gate: None,
            gate: pos(6, 10),
            walk_tiles: colony
                .world_tiles
                .values()
                .map(walk_tile_from_runtime)
                .collect(),
            zones: Vec::new(),
        };

        phase_34_movement_travel_job_acceptance_reveal_path_wear(
            &mut colony,
            TickGate {
                elapsed_sec: 8,
                processed_through: 8_000,
                minute_rolled: false,
                previous_water: 0,
            },
            &movement,
        );

        let cat = &colony.cats[0];
        assert_eq!(
            cat.position,
            Position {
                map: MapType::World,
                x: 12.0,
                y: 6.0,
            }
        );
        assert_eq!(cat.destination, None);
        assert_eq!(cat.activity, CatActivity::Working);
        assert_eq!(colony.world_tiles[&pos(10, 6)].path_wear, 64);
        assert_eq!(colony.world_tiles[&pos(11, 6)].path_wear, 64);
        assert_eq!(colony.world_tiles[&pos(12, 6)].path_wear, 64);
        assert_eq!(colony.world_tiles[&pos(11, 5)].path_wear, 63);
        assert_eq!(colony.world_tiles[&pos(11, 7)].path_wear, 63);
    }

    #[test]
    fn deliberate_roads_pick_corridor_and_keep_material_reserve() {
        let mut world_tiles = BTreeMap::new();
        for x in 20..=27 {
            world_tiles.insert(pos(x, 6), tile(x, 6, 90, None));
        }

        let mut world = WorldState {
            world_seed: 123,
            colonies: vec![ColonyRuntime {
                id: "colony-1".to_owned(),
                resources: Resources {
                    materials: 38.0,
                    ..Resources::default()
                },
                world_tiles,
                last_tick: 0,
                test_rng_seed: Some(12_345),
                ..ColonyRuntime::default()
            }],
        };

        let reports = world_tick(&mut world, 60_000);

        assert_eq!(reports[0].reset_reason, None);
        let colony = &world.colonies[0];
        assert_eq!(colony.resources.materials, 32.0);
        for x in 20..=25 {
            let paved = &colony.world_tiles[&pos(x, 6)];
            assert_eq!(paved.overlay_feature.as_deref(), Some("road_built"));
            assert_eq!(paved.path_wear, 100);
        }
        assert_eq!(
            colony.world_tiles[&pos(26, 6)].overlay_feature.as_deref(),
            None
        );
        assert!(colony.resources.materials >= ROAD_MATERIALS_RESERVE);
    }

    #[test]
    fn raid_rolls_do_not_advance_base_test_rng_seed() {
        let mut defender = adult_idle_cat("warrior", "colony-1");
        defender.specialization = Some(CatSpecialization::Warrior);
        defender.stats.attack = 200.0;
        defender.stats.defense = 200.0;

        let base_colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            resources: plentiful_resources(),
            cats: vec![defender.clone()],
            run_started_at: 0,
            created_at: 0,
            last_tick: 0,
            test_rng_seed: Some(12_345),
            ..ColonyRuntime::default()
        };
        let mut active_raid_colony = base_colony.clone();
        active_raid_colony.active_raid = Some("raid-1".to_owned());
        active_raid_colony.raiders = vec![RaiderRuntime {
            id: "raider-1".to_owned(),
            raid_id: "raid-1".to_owned(),
            position: position_from_world(tile_pos_to_world(raid_gate_position(
                &active_raid_colony,
            ))),
            destination: None,
            attack: 1.0,
            defense: 1.0,
            health: 1.0,
        }];

        let mut peaceful = WorldState {
            world_seed: 123,
            colonies: vec![base_colony],
        };
        let mut raided = WorldState {
            world_seed: 123,
            colonies: vec![active_raid_colony],
        };

        let _ = world_tick(&mut peaceful, 1_000);
        let _ = world_tick(&mut raided, 1_000);

        assert_eq!(
            peaceful.colonies[0].test_rng_seed,
            raided.colonies[0].test_rng_seed
        );
        assert!(raided.colonies[0].active_raid.is_none());
        assert!(raided.colonies[0].raiders.is_empty());
    }

    #[test]
    fn all_dead_roster_resets_run_and_skips_later_phases() {
        let mut dead = adult_idle_cat("cat-1", "colony-1");
        dead.death_time = Some(1);

        let mut world = WorldState {
            world_seed: 123,
            colonies: vec![ColonyRuntime {
                id: "colony-1".to_owned(),
                resources: Resources {
                    food: 1.0,
                    water: 2.0,
                    herbs: 3.0,
                    materials: 99.0,
                    refined: 4.0,
                    weapons: 5.0,
                    armor: 6.0,
                    blessings: 7.0,
                },
                cats: vec![dead],
                jobs: vec![JobRuntime {
                    id: "job-1".to_owned(),
                    kind: JobKind::HuntExpedition,
                    status: JobStatus::Active,
                    assigned_cat: Some("cat-1".to_owned()),
                    ..JobRuntime::default()
                }],
                buildings: vec![
                    BuildingRuntime {
                        id: "complete-den".to_owned(),
                        construction_progress: 100,
                        is_complete: true,
                        ..BuildingRuntime::default()
                    },
                    BuildingRuntime {
                        id: "unfinished-den".to_owned(),
                        construction_progress: 50,
                        is_complete: false,
                        ..BuildingRuntime::default()
                    },
                ],
                world_tiles: BTreeMap::from([(pos(20, 6), tile(20, 6, 90, None))]),
                elections: vec![ElectionRuntime {
                    id: "election-1".to_owned(),
                    opened_at: 0,
                    closes_at: 120_000,
                    resolved_at: None,
                    winner_cat_id: Some("cat-1".to_owned()),
                    kind: ElectionKind::Scheduled,
                }],
                raiders: vec![RaiderRuntime {
                    id: "raider-1".to_owned(),
                    raid_id: "raid-1".to_owned(),
                    position: Position::default(),
                    destination: None,
                    attack: 1.0,
                    defense: 1.0,
                    health: 1.0,
                }],
                active_raid: Some("raid-1".to_owned()),
                threat_pressure: 90.0,
                raid_clicks: 3.0,
                run_number: 4,
                last_tick: 0,
                test_rng_seed: Some(12_345),
                ..ColonyRuntime::default()
            }],
        };

        let reports = world_tick(&mut world, 60_000);

        assert_eq!(
            reports,
            vec![TickReport {
                colony_id: "colony-1".to_owned(),
                skipped: false,
                reset_reason: Some(RunResetReason::AllCatsDead),
            }]
        );
        let colony = &world.colonies[0];
        assert_eq!(colony.resources, starting_resources_with_blessings(7.0));
        assert!(colony.jobs.is_empty());
        assert!(colony.raiders.is_empty());
        assert_eq!(colony.active_raid, None);
        assert_eq!(colony.threat_pressure, 0.0);
        assert_eq!(colony.raid_clicks, 0.0);
        assert_eq!(colony.run_number, 5);
        assert_eq!(colony.run_started_at, 60_000);
        assert_eq!(colony.last_tick, 60_000);
        assert_eq!(colony.buildings.len(), 1);
        assert_eq!(colony.buildings[0].id, "complete-den");
        assert!(colony.world_tiles.contains_key(&pos(20, 6)));
        assert_eq!(colony.elections[0].resolved_at, Some(60_000));
        assert_eq!(colony.elections[0].winner_cat_id, None);
    }

    #[test]
    fn founded_colony_world_tick_is_deterministic_for_same_seed() {
        let mut left = new_world(987_654);
        left.colonies
            .push(found_colony(left.world_seed, "colony-1", 1_000, 55_555));
        let mut right = new_world(987_654);
        right
            .colonies
            .push(found_colony(right.world_seed, "colony-1", 1_000, 55_555));

        for step in 1..=40 {
            let now = 1_000 + i64::from(step) * 60_000;
            assert_eq!(world_tick(&mut left, now), world_tick(&mut right, now));
        }

        assert_eq!(
            founded_snapshot(&left.colonies[0]),
            founded_snapshot(&right.colonies[0])
        );
    }

    #[test]
    fn founded_colony_survives_opening_ticks() {
        let mut world = new_world(1234);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 10_000, 1234));

        for step in 1..=40 {
            let now = 10_000 + i64::from(step) * 60_000;
            let reports = world_tick(&mut world, now);
            assert_eq!(reports[0].reset_reason, None);
        }

        let colony = &world.colonies[0];
        assert!(alive_cats(&colony.cats).count() > 0);
        assert_ne!(colony.status, ColonyStatus::Dead);
    }

    #[test]
    fn world_tick_processes_founded_colonies_in_stable_order_without_cross_mutation() {
        let start = 20_000;
        let mut world = new_world(777);
        let mut beta = found_colony(world.world_seed, "beta", start, 200);
        let mut alpha = found_colony(world.world_seed, "alpha", start, 100);
        beta.jobs.push(due_supply_job("beta-food", start, 1));
        alpha.jobs.push(due_supply_job("alpha-food", start, 2));
        world.colonies.push(beta);
        world.colonies.push(alpha);

        let reports = world_tick(&mut world, start + 60_000);

        assert_eq!(
            reports
                .iter()
                .map(|report| report.colony_id.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "beta"]
        );

        let alpha = world
            .colonies
            .iter()
            .find(|colony| colony.id == "alpha")
            .expect("alpha colony remains present");
        let beta = world
            .colonies
            .iter()
            .find(|colony| colony.id == "beta")
            .expect("beta colony remains present");

        assert_eq!(
            alpha
                .jobs
                .iter()
                .find(|job| job.id == "alpha-food")
                .map(|job| job.status),
            Some(JobStatus::Completed)
        );
        assert_eq!(
            beta.jobs
                .iter()
                .find(|job| job.id == "beta-food")
                .map(|job| job.status),
            Some(JobStatus::Completed)
        );
        assert!(alpha.cats.iter().all(|cat| cat.colony_id == "alpha"));
        assert!(beta.cats.iter().all(|cat| cat.colony_id == "beta"));
        assert!(alpha.jobs.iter().all(|job| !job.id.starts_with("beta")));
        assert!(beta.jobs.iter().all(|job| !job.id.starts_with("alpha")));
        assert_ne!(alpha.test_rng_seed, beta.test_rng_seed);
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
            skills: Default::default(),
        }
    }

    #[derive(Debug, PartialEq)]
    struct FoundedSnapshot {
        resources: Resources,
        population: usize,
        status: ColonyStatus,
        jobs: Vec<(JobId, JobKind, JobStatus, Option<CatId>, JobMetadata)>,
    }

    fn founded_snapshot(colony: &ColonyRuntime) -> FoundedSnapshot {
        FoundedSnapshot {
            resources: colony.resources.clone(),
            population: alive_cats(&colony.cats).count(),
            status: colony.status,
            jobs: colony
                .jobs
                .iter()
                .map(|job| {
                    (
                        job.id.clone(),
                        job.kind,
                        job.status,
                        job.assigned_cat.clone(),
                        job.metadata.clone(),
                    )
                })
                .collect(),
        }
    }

    fn due_supply_job(id: &str, start: i64, click_count: u32) -> JobRuntime {
        JobRuntime {
            id: id.to_owned(),
            kind: JobKind::SupplyFood,
            status: JobStatus::Active,
            requested_by: JobRequester::Player,
            assigned_cat: None,
            duration_ms: 60_000,
            speed: 1.0,
            yield_amount: 1.0,
            click_count,
            created_at: start,
            started_at: Some(start),
            ends_at: Some(start + 60_000),
            completed_at: None,
            metadata: JobMetadata::None,
        }
    }

    fn plentiful_resources() -> Resources {
        Resources {
            food: 100.0,
            water: 100.0,
            herbs: 16.0,
            materials: 100.0,
            refined: 20.0,
            weapons: 0.0,
            armor: 0.0,
            blessings: 0.0,
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
