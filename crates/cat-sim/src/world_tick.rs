//! Runtime world tick skeleton ported from `server/game.ts:workerTick`.
//!
//! This P7.1 module owns the in-memory runtime shapes and phase ordering. Later
//! P7 cards fill in the no-op phase bodies with the pure module calls.

use std::collections::{BTreeMap, BTreeSet, HashSet};

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
    genetics::{
        CatSpriteParams, RollSource, SeededRollSource, extract_genetic_traits, inherit_traits,
        traits_to_sprite_params,
    },
    idle_engine,
    idle_rules::consumption_for_tick,
    leader_ai::{LeaderDecision, LeaderHousing, LeaderResources, LeaderSnapshot},
    leader_director::{
        CatBrief, CatBriefStats, DirectorPlan, LaborGoalKind, MatchOptions, direct_colony,
        match_cats_to_slots_with_officers,
    },
    ledger::{StockLedger, refresh_ledger},
    life_sim::{
        ColonyBreedingState, GESTATION_GAME_HOURS, can_work, colony_can_breed,
        conception_probability, get_life_stage, inherit_stats, leadership_after_tenure,
        old_age_death_probability,
    },
    movement::{
        EXPLORE_SPEED_FACTOR, JobDestinationContext, WorldPos, destination_for_job,
        effective_move_speed, pick_wander_target, road_surface_multiplier, scout_wander_target,
        walk_path,
    },
    officers::OfficerRole,
    pathfinding::{
        self, ColonyGridParams, FindPathOptions, GatePlacement as PathGatePlacement,
        TilePos as PathTilePos, WalkOverlayFeature, WalkTile, WalkTileResources, WalkTileType,
        build_colony_walk_grid, find_path,
    },
    policy::PolicyConfig,
    production::{
        WoodworkingOptions, WorkshopOptions, advance_woodworking, advance_workshop, field_yield,
    },
    rng::{life_seed, movement_seed, raid_seed, roll_seeded},
    roads::{RoadCorridorOptions, RoadTile, select_road_corridor},
    shrine::should_deposit,
    skills::{HAUL_SKILL_GAIN, Labor, SKILL_GAIN_PER_JOB},
    smithy::{SmithyOptions, advance_smithy},
    spoilage::apply_food_spoilage_after_consumption,
    stockpiles::{self, ResourceKind, Stockpile},
    storage::{
        StorageBuilding, StorageCapacities, count_storehouses, storage_capacities, storehouse_cap,
    },
    survival::{SurvivalResources, apply_survival_tick},
    threat::{
        ThreatSnapshot, accrue_threat, colony_wealth, plan_raid, resolve_raid, should_spawn_raid,
        threat_band,
    },
    trips::{HUNT_TRIP_COUNT, remaining_yield, split_yield, trip_due_at},
    types::{
        BuildingType, CatSpecialization, JobKind, JobStatus, LifeStage, TaskType, TileType,
        UpgradeKey,
    },
    upgrade_tree::{
        MOUNTAINEERING_NODE_ID, UpgradeTreeState, accrue_research, cat_auto_unlock,
        create_upgrade_tree_state, get_node, is_owned, points_per_tick_for, resolve_effects,
    },
    village_area::{
        ExpandOptions, GatePlacement as AreaGatePlacement, Side, expand_village, from_tiles,
        gate_placement_default, is_inside_village, should_expand, side_delta,
    },
    village_layout::{
        DEFAULT_MAX_RING, GridPos, VILLAGE_ANCHOR, colony_to_world,
        next_building_site_with_blocked, ring_cells, village_ring_radius,
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
    /// Fog-of-war: the set of world tiles the client is allowed to render un-fogged.
    /// Runtime-only and independent of `world_tiles` (which is lazily/​sparsely
    /// populated for the live colony) — the founding village reveal seeds it and cats
    /// walking near a tile add to it (`reveal_and_wear_walked_tiles`). Does not affect
    /// the sim; a `BTreeSet` keeps the snapshot order deterministic.
    pub revealed_tiles: BTreeSet<TilePos>,
    /// Appointed officers (role → cat id). P12.2 additive layer; empty = no effect.
    pub officers: BTreeMap<OfficerRole, String>,
    /// On-map stockpiles (P12.3). Always includes the shrine reservoir after a tick;
    /// their contents sum to `resources` per the balancing-reservoir invariant.
    pub stockpiles: Vec<Stockpile>,
    /// Reported stock ledger (P12.4a). A lagging *view* of `resources`; a staffed Accounting
    /// Tent keeps it exact each tick, otherwise it recounts on an interval. Never affects the
    /// true `resources`.
    pub stock_ledger: StockLedger,
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

/// Tile footprint `(width, height)` a building of `building_type` occupies.
///
/// The footprint is a pure function of the building type — it is *derived*, never
/// persisted, so a building still stores only `position + type`. `position` is the
/// footprint's **anchor = its minimum (north-west) corner**; the building covers the
/// half-open rectangle `[x, x + w) x [y, y + h)` (see [`footprint_tiles`]).
#[must_use]
pub const fn footprint_for(building_type: BuildingType) -> (i32, i32) {
    match building_type {
        // The shrine is the village hub, and the workshops/storehouse are broad
        // work-yards — all 3x3 (P16).
        BuildingType::Shrine
        | BuildingType::Workshop
        | BuildingType::Smithy
        | BuildingType::FoodStorage
        | BuildingType::WoodCutter
        | BuildingType::StonePrep
        | BuildingType::Woodworking => (3, 3),
        // Dwellings, gardens and the mid buildings take a 2x3 plot (P16).
        BuildingType::Den
        | BuildingType::Beds
        | BuildingType::Nursery
        | BuildingType::HerbGarden
        | BuildingType::ElderCorner
        | BuildingType::MouseFarm
        | BuildingType::Field
        | BuildingType::Barracks
        | BuildingType::AccountingTent => (2, 3),
        // Bowls and wall segments are single tiles.
        BuildingType::WaterBowl | BuildingType::Walls => (1, 1),
    }
}

/// The tiles covered by a `w x h` footprint anchored at its north-west corner
/// `position` — i.e. `[x, x + w) x [y, y + h)`, row-major. Empty if `w`/`h` <= 0.
#[must_use]
pub fn footprint_tiles(position: TilePos, w: i32, h: i32) -> Vec<TilePos> {
    let mut tiles = Vec::with_capacity((w.max(0) * h.max(0)) as usize);
    for dy in 0..h {
        for dx in 0..w {
            tiles.push(TilePos {
                x: position.x + dx,
                y: position.y + dy,
            });
        }
    }
    tiles
}

/// Tiles covered by an existing building's derived footprint.
fn building_footprint_tiles(building: &BuildingRuntime) -> Vec<TilePos> {
    let (w, h) = footprint_for(building.building_type);
    footprint_tiles(building.position, w, h)
}

/// Every tile currently covered by any building's footprint.
fn occupied_building_tiles(colony: &ColonyRuntime) -> HashSet<TilePos> {
    colony
        .buildings
        .iter()
        .flat_map(building_footprint_tiles)
        .collect()
}

/// Whether `tile` sits on the fence perimeter (the palisade wall ring): a tile that
/// is *not* claimed village ground but orthogonally borders it. Buildings must not
/// straddle the wall — since placement also requires the whole footprint to lie
/// inside `claimed_tiles`, perimeter tiles are excluded both ways.
fn tile_is_on_fence_perimeter(claimed: &HashSet<TilePos>, tile: TilePos) -> bool {
    if claimed.contains(&tile) {
        return false;
    }
    [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
        claimed.contains(&TilePos {
            x: tile.x + dx,
            y: tile.y + dy,
        })
    })
}

/// Whether `tile` can host (part of) a building footprint. True when the tile is
/// already covered by another building, holds water, holds a (client-rendered,
/// terrain-generated) tree, or lies on the fence perimeter (wall).
///
/// `world_seed` is required for the deterministic tree query — trees are otherwise
/// client-only, so the sim reconstructs them from the same terrain generator the
/// renderer uses (see [`crate::terrain_gen::tile_has_tree`]).
#[must_use]
pub fn tile_is_occupied(colony: &ColonyRuntime, tile: TilePos, world_seed: u32) -> bool {
    let claimed: HashSet<TilePos> = colony.claimed_tiles.iter().copied().collect();
    occupied_building_tiles(colony).contains(&tile)
        || tile_has_water(colony.world_tiles.get(&tile))
        || crate::terrain_gen::tile_has_tree(world_seed, tile.x, tile.y)
        || tile_is_on_fence_perimeter(&claimed, tile)
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
/// Small fixed start (P16): a founded colony begins with exactly this many adult cats.
const STARTER_CAT_COUNT: usize = 5;
/// Starter cats are spread across the adult band (kitten 0–6 / young 6–24 / adult
/// 24–48 / elder 48+ game-hours) so all five can work at full weight from day one and
/// none is near the elder mortality curve.
const STARTER_AGE_MIN_HOURS: f64 = 26.0;
const STARTER_AGE_MAX_HOURS: f64 = 42.0;
/// Half-width (from the shrine's centre tile) of the fixed founding village's claimed
/// square. Sized so the fixed blueprint — shrine + 3 dens + 3 workshops — fits with a
/// one-tile margin to the wall for the ring road out to each gate/edge.
const VILLAGE_START_RADIUS: i32 = 6;
/// Fog-of-war founding reveal: the whole claimed village starts revealed, plus a halo
/// of this Chebyshev radius around the anchor (covers the adjacent water source).
/// Everything beyond starts fogged and is uncovered by `reveal_and_wear_walked_tiles`.
const FOUNDING_REVEAL_RADIUS: i32 = 2;
/// Water level stamped on a founding pond tile (a practically-infinite source).
const FOUNDING_WATER: u32 = 999;

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
    /// Needed for the deterministic per-tile terrain surface speed factor.
    world_seed: u32,
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
            revealed_tiles: BTreeSet::new(),
            officers: BTreeMap::new(),
            stockpiles: Vec::new(),
            stock_ledger: StockLedger::default(),
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
    let mut colony = ColonyRuntime {
        id: colony_id.clone(),
        name: format!("Colony {colony_id}"),
        status: ColonyStatus::Starting,
        resources: starting_resources(),
        cats: create_starter_cats(&colony_id, now_ms, seed),
        buildings: starter_buildings(world_seed),
        world_tiles: starter_world_tiles(world_seed),
        claimed_tiles: founding_claimed_tiles(),
        run_number: 1,
        run_started_at: now_ms,
        created_at: now_ms,
        last_player_activity_at: Some(now_ms),
        last_tick: now_ms,
        test_rng_seed: Some(seed),
        ..ColonyRuntime::default()
    };
    // Pave the shrine-to-wall stone road cross and clear/guarantee the village's water
    // source so the fixed blueprint sits on solid, drinkable ground.
    stamp_founding_roads_and_water(&mut colony);
    // The world starts tiny: fog covers everything except the founding village reveal.
    // Cats uncover the rest as they walk.
    reveal_founding_area(&mut colony);
    // Seed the shrine reservoir so the stockpile invariant holds before the first tick.
    reconcile_colony_stockpiles(&mut colony);
    // The books are counted at founding, so the reported ledger starts exact.
    colony.stock_ledger = StockLedger::counted(&colony.resources, now_ms);
    colony
}

/// Lift the fog for the founding village reveal: the whole claimed village ground
/// starts revealed (players can see their own settlement), plus a `FOUNDING_REVEAL_RADIUS`
/// halo around the anchor so the immediately-adjacent water source is visible. Nothing
/// further is revealed at founding — that is the job of the reveal-on-walk pass. The
/// reveal set is independent of `world_tiles`, so it is correct even when the live
/// colony's tile map is sparse.
fn reveal_founding_area(colony: &mut ColonyRuntime) {
    let claimed = colony.claimed_tiles.clone();
    colony.revealed_tiles.extend(claimed);
    for dy in -FOUNDING_REVEAL_RADIUS..=FOUNDING_REVEAL_RADIUS {
        for dx in -FOUNDING_REVEAL_RADIUS..=FOUNDING_REVEAL_RADIUS {
            colony.revealed_tiles.insert(TilePos {
                x: VILLAGE_ANCHOR.x + dx,
                y: VILLAGE_ANCHOR.y + dy,
            });
        }
    }
}

fn starting_resources() -> Resources {
    // P16 pre-filled general stockpile. `materials` stands in for the "50 wood + 10
    // stone" of the blueprint until the P12.4b wood/stone chains land; food seeds the
    // colony while the first hunts and the nearby water source come online.
    Resources {
        food: 50.0,
        water: 100.0,
        herbs: 16.0,
        materials: 60.0,
        refined: 0.0,
        weapons: 0.0,
        armor: 0.0,
        // P12.4b refinement tier — empty at founding; the wood-cutter, stone-prep
        // and woodworking chains build planks/blocks/tools from raw materials.
        planks: 0.0,
        blocks: 0.0,
        tools: 0.0,
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
    cat_sprite_params_to_map(params)
}

/// Flatten sprite params into the `BTreeMap<String, Value>` shape `Cat::sprite_params`
/// stores (mirrors `spriteParams` JSON column semantics on the TS side).
fn cat_sprite_params_to_map(
    params: CatSpriteParams,
) -> Option<BTreeMap<String, serde_json::Value>> {
    let value = serde_json::to_value(params).ok()?;
    match value {
        serde_json::Value::Object(map) => Some(map.into_iter().collect()),
        _ => None,
    }
}

/// Reconstruct `CatSpriteParams` from a cat's stored sprite params map, for
/// extracting cosmetic genetic traits at breeding time. `None` for cats with no
/// sprite params yet (founders, in the rare case sprite generation failed) or a
/// shape that no longer round-trips.
fn cat_sprite_params_from_cat(cat: &Cat) -> Option<CatSpriteParams> {
    let map = cat.sprite_params.as_ref()?;
    let value = serde_json::Value::Object(map.clone().into_iter().collect());
    serde_json::from_value(value).ok()
}

/// The shrine's centre tile. The shrine's footprint anchor (its NW corner) is the
/// village anchor; a 3×3 footprint puts the centre one tile SE of it. Roads radiate
/// from this tile and the claimed square is centred on it.
const fn shrine_center_tile() -> TilePos {
    TilePos {
        x: VILLAGE_ANCHOR.x + 1,
        y: VILLAGE_ANCHOR.y + 1,
    }
}

/// The fixed founding blueprint (P16): the shrine dead-centre, three den houses and
/// the three raw-material workshops arranged in the four quadrants around it, leaving
/// the shrine's centre row/column clear for the road cross. Anchors are absolute NW
/// corners; every footprint stays inside `founding_claimed_tiles` and none overlap.
///
/// Layout (shrine centre at 7,7; claimed square 1..=13):
/// ```text
///   Woodworking(2,2)  Den(5,2)   ·road·   WoodCutter(9,2)
///                              [ Shrine 6..8 ]
///   Den(2,9)          Den(5,9)   ·road·   StonePrep(9,9)
/// ```
const STARTER_BLUEPRINT: [(BuildingType, i32, i32, u32); 7] = [
    (BuildingType::Shrine, 6, 6, 1),
    (BuildingType::Woodworking, 2, 2, 1),
    (BuildingType::Den, 5, 2, 2),
    (BuildingType::WoodCutter, 9, 2, 1),
    (BuildingType::Den, 2, 9, 2),
    (BuildingType::Den, 5, 9, 2),
    (BuildingType::StonePrep, 9, 9, 1),
];

fn starter_buildings(_world_seed: u32) -> Vec<BuildingRuntime> {
    STARTER_BLUEPRINT
        .into_iter()
        .enumerate()
        .map(|(index, (building_type, x, y, level))| {
            let id = if building_type == BuildingType::Shrine {
                "building-shrine".to_owned()
            } else {
                format!("building-starter-{}", index)
            };
            BuildingRuntime {
                id,
                building_type,
                level,
                position: TilePos { x, y },
                is_complete: true,
                construction_progress: 100,
                production_progress: 0.0,
                assigned_cat: None,
            }
        })
        .collect()
}

/// The stone-road tiles of the founding cross: the shrine's centre row/column extended
/// out to each wall (N/S/E/W), skipping the shrine's own footprint.
fn founding_road_tiles() -> Vec<TilePos> {
    let center = shrine_center_tile();
    let (shrine_w, shrine_h) = footprint_for(BuildingType::Shrine);
    let shrine_min_x = VILLAGE_ANCHOR.x;
    let shrine_max_x = VILLAGE_ANCHOR.x + shrine_w - 1;
    let shrine_min_y = VILLAGE_ANCHOR.y;
    let shrine_max_y = VILLAGE_ANCHOR.y + shrine_h - 1;
    let lo = -VILLAGE_START_RADIUS;
    let hi = VILLAGE_START_RADIUS;

    let mut tiles = Vec::new();
    for d in lo..=hi {
        // Vertical arm along the shrine's centre column, skipping the shrine rows.
        let y = center.y + d;
        if y < shrine_min_y || y > shrine_max_y {
            tiles.push(TilePos { x: center.x, y });
        }
        // Horizontal arm along the shrine's centre row, skipping the shrine columns.
        let x = center.x + d;
        if x < shrine_min_x || x > shrine_max_x {
            tiles.push(TilePos { x, y: center.y });
        }
    }
    tiles
}

/// Pave the founding road cross and guarantee the village sits on solid, drinkable
/// ground: clear any water that collides with a building or road footprint, then make
/// sure a reachable water source remains (carving a deterministic pond if not).
fn stamp_founding_roads_and_water(colony: &mut ColonyRuntime) {
    let roads = founding_road_tiles();
    let mut blocked: HashSet<TilePos> = colony
        .buildings
        .iter()
        .flat_map(building_footprint_tiles)
        .collect();
    blocked.extend(roads.iter().copied());

    // Buildings and roads take priority over terrain water — clear anything underneath.
    let colliding: Vec<TilePos> = colony
        .world_tiles
        .iter()
        .filter(|(pos, tile)| blocked.contains(pos) && tile_has_water(Some(tile)))
        .map(|(pos, _)| *pos)
        .collect();
    for pos in colliding {
        clear_water_tile(colony, pos);
    }

    // Lay the stone road overlay.
    for pos in &roads {
        if let Some(tile) = colony.world_tiles.get_mut(pos) {
            tile.overlay_feature = Some("road_built".to_owned());
        }
    }

    // Ensure the village still has a water source it can reach; otherwise carve one on
    // the nearest free in-band tile (deterministic — no RNG).
    let has_reachable_water = colony.world_tiles.values().any(|tile| {
        tile_has_water(Some(tile))
            && cheb_from_anchor(tile.pos) <= 6
            && !blocked.contains(&tile.pos)
    });
    if !has_reachable_water && let Some(pos) = founding_pond_site(colony, &blocked) {
        set_water_tile(colony, pos);
    }
}

/// Convert a world tile to plain walkable meadow ground (used to clear water from under
/// a building/road footprint).
fn clear_water_tile(colony: &mut ColonyRuntime, pos: TilePos) {
    if let Some(tile) = colony.world_tiles.get_mut(&pos) {
        tile.tile_type = TileType::Meadow;
        tile.overlay_feature = None;
        tile.resources.water = 0;
        tile.danger_level = 0.0;
    }
}

/// Stamp a world tile as an (essentially infinite) water source.
fn set_water_tile(colony: &mut ColonyRuntime, pos: TilePos) {
    if let Some(tile) = colony.world_tiles.get_mut(&pos) {
        tile.tile_type = TileType::River;
        tile.overlay_feature = Some("river".to_owned());
        tile.resources.water = FOUNDING_WATER;
        tile.danger_level = 5.0;
    }
}

/// Pick a deterministic free claimed tile (not a building/road, a few tiles out from the
/// shrine) to carve the founding pond on, scanning in `(y, x)` order.
fn founding_pond_site(colony: &ColonyRuntime, blocked: &HashSet<TilePos>) -> Option<TilePos> {
    let mut candidates: Vec<TilePos> = colony
        .claimed_tiles
        .iter()
        .copied()
        .filter(|pos| {
            !blocked.contains(pos)
                && (2..=6).contains(&cheb_from_anchor(*pos))
                && colony
                    .world_tiles
                    .get(pos)
                    .is_some_and(|tile| !tile_has_water(Some(tile)))
        })
        .collect();
    candidates.sort_by_key(|pos| (pos.y, pos.x));
    candidates.into_iter().next()
}

fn founding_claimed_tiles() -> Vec<TilePos> {
    let center = shrine_center_tile();
    let mut tiles = Vec::new();
    for dy in -VILLAGE_START_RADIUS..=VILLAGE_START_RADIUS {
        for dx in -VILLAGE_START_RADIUS..=VILLAGE_START_RADIUS {
            tiles.push(TilePos {
                x: center.x + dx,
                y: center.y + dy,
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

#[must_use]
pub fn world_tick(state: &mut WorldState, now_ms: i64) -> Vec<TickReport> {
    let world_seed = state.world_seed;
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
        phase_14_promote_queued_jobs_and_break_ground(colony, gate, world_seed);
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
        phase_25_survival_deaths_and_carried_yield_salvage(colony, gate, policy);
        if let Some(reset_reason) = phase_26_empty_colony_reset(colony, gate) {
            reconcile_colony_stockpiles(colony);
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
            phase_32_movement_setup_and_village_expansion_queue(colony, gate, policy, world_seed);
        phase_33_movement_deposits_and_no_destination_wander(colony, gate, &mut movement);
        phase_34_movement_travel_job_acceptance_reveal_path_wear(colony, gate, &movement);
        phase_35_deliberate_roads(colony, gate);
        if let Some(reset_reason) = phase_36_threat_and_raid_director(colony, gate) {
            reconcile_colony_stockpiles(colony);
            reports.push(TickReport {
                colony_id: colony.id.clone(),
                skipped: false,
                reset_reason: Some(reset_reason),
            });
            continue;
        }
        let reset_reason = phase_37_final_clamp_critical_collapse_status_persist(colony, gate);
        reconcile_colony_stockpiles(colony);

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
///
/// Ported from `server/game.ts:runLifeSimulation`. Three passes over the roster, all
/// on the life-sim's forked RNG chain (`life_seed`) so the policy/movement chains stay
/// byte-stable: (1) age + old-age mortality + leadership tenure — unchanged from
/// before; (2) deliver any kitten whose gestation is up; (3) pair off healthy adults
/// into new pregnancies while the village is fed, watered, and has spare housing.
fn phase_6_life_simulation(colony: &mut ColonyRuntime, gate: TickGate) {
    let elapsed_game_hours = elapsed_game_hours(colony, gate);
    if elapsed_game_hours <= 0.0 {
        return;
    }

    let mut life_rng_seed = life_seed(colony.test_rng_seed.unwrap_or(1));
    let leader_id = colony.leader_id.clone();

    // 1. Aging, old-age mortality, leadership tenure. Deaths are snapshotted here and
    // their salvage/job-cancel/event cleanup deferred until after the loop: those
    // steps need `&mut ColonyRuntime` as a whole, which conflicts with the
    // `&mut colony.cats` borrow this loop holds (same deferral pattern pass 2 below
    // uses for newborn insertion).
    let mut old_age_deaths: Vec<OldAgeDeath> = Vec::new();
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
                old_age_deaths.push(OldAgeDeath {
                    id: cat.id.clone(),
                    name: cat.name.clone(),
                    position: cat.position,
                    carrying: cat.carrying.clone(),
                });
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

    // Salvage each old-age death's carried yield, cancel its outstanding jobs, and
    // log a death event — mirrors phase 25's survival-death cleanup and TS
    // `retireCat`, which runs for every death including old age.
    for death in old_age_deaths {
        if let Some(carrying) = death.carrying {
            let deposit_at = position_to_world(death.position);
            credit_carrying(colony, &carrying, deposit_at);
        }
        cancel_cat_jobs(colony, &death.id, gate.processed_through);
        append_event(
            colony,
            gate.processed_through,
            EventKind::Other("death".to_owned()),
            format!("{} died peacefully of old age.", death.name),
        );
    }

    // 2. Births: any mother whose gestation (tracked against her own age) is up. Only
    // cats still alive after pass 1 are eligible — a mother who dies of old age this
    // tick loses the pregnancy along with everything else.
    let due_mother_indices: Vec<usize> = colony
        .cats
        .iter()
        .enumerate()
        .filter(|(_, cat)| {
            cat.death_time.is_none()
                && cat.is_pregnant
                && cat
                    .pregnancy_due_age_hours
                    .is_some_and(|due| cat.age_hours >= due)
        })
        .map(|(index, _)| index)
        .collect();

    let mut newborns: Vec<Cat> = Vec::with_capacity(due_mother_indices.len());
    for (birth_index, mother_index) in due_mother_indices.into_iter().enumerate() {
        let mother = colony.cats[mother_index].clone();
        let father = mother.pregnancy_mate_id.as_ref().and_then(|mate_id| {
            colony
                .cats
                .iter()
                .find(|cat| cat.id == *mate_id && cat.death_time.is_none())
                .cloned()
        });

        let mother_traits = extract_genetic_traits(cat_sprite_params_from_cat(&mother).as_ref());
        let father_traits = father
            .as_ref()
            .and_then(|father| extract_genetic_traits(cat_sprite_params_from_cat(father).as_ref()));

        let kitten_stats = inherit_stats(
            &mother.stats,
            father.as_ref().map(|father| &father.stats),
            || next_life_roll(&mut life_rng_seed),
        );

        // Cosmetic genetics run on the same forked chain, via the injected roll source
        // genetics.rs already exposes for deterministic testing.
        let mut genetics_rolls = SeededRollSource::new(life_rng_seed);
        let kitten_traits = inherit_traits(
            mother_traits.as_ref(),
            father_traits.as_ref(),
            &mut genetics_rolls,
        );
        let kitten_sprite_params = cat_sprite_params_to_map(traits_to_sprite_params(
            &kitten_traits,
            None,
            &mut genetics_rolls,
        ));
        life_rng_seed = genetics_rolls.seed();

        let name_roll = next_life_roll(&mut life_rng_seed);
        let kitten_name = generate_starter_name((name_roll * 1_000_000_000.0).floor() as u32);

        let kitten_id = format!("{}-kit-{}-{birth_index}", colony.id, gate.processed_through);

        newborns.push(Cat {
            id: kitten_id,
            colony_id: colony.id.clone(),
            name: kitten_name.clone(),
            parent_ids: vec![
                Some(mother.id.clone()),
                father.as_ref().map(|father| father.id.clone()),
            ],
            birth_time: gate.processed_through,
            death_time: None,
            stats: kitten_stats,
            needs: CatNeeds {
                hunger: 100.0,
                thirst: 100.0,
                rest: 100.0,
                health: 100.0,
            },
            current_task: None,
            position: mother.position,
            destination: None,
            carrying: None,
            activity: CatActivity::Idle,
            is_pregnant: false,
            pregnancy_due_time: None,
            age_hours: 0.0,
            pregnancy_due_age_hours: None,
            pregnancy_mate_id: None,
            sprite_params: kitten_sprite_params,
            specialization: None,
            role_xp: RoleXp::default(),
            skills: BTreeMap::new(),
        });

        let mother_cat = &mut colony.cats[mother_index];
        mother_cat.is_pregnant = false;
        mother_cat.pregnancy_due_time = None;
        mother_cat.pregnancy_due_age_hours = None;
        mother_cat.pregnancy_mate_id = None;

        let message = match father.as_ref() {
            Some(father) => {
                format!(
                    "{kitten_name} was born to {} and {}.",
                    mother.name, father.name
                )
            }
            None => format!("{kitten_name} was born to {}.", mother.name),
        };
        append_event(
            colony,
            gate.processed_through,
            EventKind::Other("birth".to_owned()),
            message,
        );
    }
    colony.cats.append(&mut newborns);

    // 3. Conceptions: adults pair off while the colony is healthy and has room. The
    // housing headroom check is the soft population cap — growth tracks the village's
    // shelter instead of running away. Caps/ratios are read from `colony.resources` as
    // they stand before phase 7's consumption, matching the TS ordering.
    let caps = storage_caps(colony);
    let food_ratio = if caps.food > 0.0 {
        colony.resources.food / caps.food
    } else {
        0.0
    };
    let water_ratio = if caps.water > 0.0 {
        colony.resources.water / caps.water
    } else {
        0.0
    };
    let housing_cap = colony_housing_capacity(colony);
    let blessings = colony.resources.blessings;
    let population = alive_cats(&colony.cats).count() as f64;
    let mut pregnant_count = alive_cats(&colony.cats)
        .filter(|cat| cat.is_pregnant)
        .count() as f64;

    let adults: Vec<BreedingCandidate> = colony
        .cats
        .iter()
        .enumerate()
        .filter(|(_, cat)| {
            cat.death_time.is_none()
                && !cat.is_pregnant
                && get_life_stage(cat.age_hours) == LifeStage::Adult
        })
        .map(|(index, cat)| BreedingCandidate {
            cat_index: index,
            id: cat.id.clone(),
            stats: cat.stats.clone(),
            specialization: cat.specialization,
        })
        .collect();

    for candidate in &adults {
        let breeding_state = ColonyBreedingState {
            food_ratio,
            water_ratio,
            population: population + pregnant_count,
            housing_capacity: housing_cap,
            food: Some(colony.resources.food),
            water: Some(colony.resources.water),
        };
        if !colony_can_breed(&breeding_state) {
            break;
        }

        let chance =
            conception_probability(candidate.specialization, blessings, elapsed_game_hours);
        let roll = next_life_roll(&mut life_rng_seed);
        if roll >= chance {
            continue;
        }

        let mate_id = pick_mate(&adults, &candidate.id, candidate.specialization);

        let cat_name;
        {
            let cat = &mut colony.cats[candidate.cat_index];
            cat.is_pregnant = true;
            cat.pregnancy_due_age_hours = Some(cat.age_hours + GESTATION_GAME_HOURS);
            cat.pregnancy_due_time =
                Some(gate.processed_through + (GESTATION_GAME_HOURS * 3_600_000.0) as i64);
            cat.pregnancy_mate_id = mate_id.clone();
            cat_name = cat.name.clone();
        }
        pregnant_count += 1.0;

        let mate_name = mate_id
            .as_ref()
            .and_then(|mate_id| colony.cats.iter().find(|cat| cat.id == *mate_id))
            .map(|cat| cat.name.clone());
        let message = match mate_name {
            Some(mate_name) => format!("{cat_name} and {mate_name} are expecting a litter."),
            None => format!("{cat_name} is expecting a litter."),
        };
        append_event(
            colony,
            gate.processed_through,
            EventKind::Other("breeding".to_owned()),
            message,
        );
    }
}

/// A cat that died of old age in phase 6 pass 1, snapshotted before the pass's
/// mutable borrow of `colony.cats` ends. Processed afterward to salvage carried
/// yield, cancel jobs, and log a death event against `&mut ColonyRuntime`.
struct OldAgeDeath {
    id: CatId,
    name: String,
    position: Position,
    carrying: Option<Carrying>,
}

/// A conception-eligible adult snapshotted before phase 6's conception pass mutates
/// anything — used both to iterate candidates in a stable order and as `pick_mate`'s
/// pairing pool (mirrors `adults` in `server/game.ts:runLifeSimulation`).
struct BreedingCandidate {
    cat_index: usize,
    id: CatId,
    stats: CatStats,
    specialization: Option<CatSpecialization>,
}

/// Pick a co-parent for a conceiving cat: another eligible adult, preferring one with
/// the same specialization so lineages of a trade concentrate, then the strongest
/// available (leadership + hunting + building, ties keep the earliest candidate).
/// Deterministic — no RNG. `None` if no partner exists (ported from
/// `server/game.ts:pickMate`).
fn pick_mate(
    candidates: &[BreedingCandidate],
    cat_id: &str,
    specialization: Option<CatSpecialization>,
) -> Option<CatId> {
    let others: Vec<&BreedingCandidate> = candidates
        .iter()
        .filter(|candidate| candidate.id != cat_id)
        .collect();
    if others.is_empty() {
        return None;
    }

    let same_trade: Vec<&BreedingCandidate> = specialization
        .map(|specialization| {
            others
                .iter()
                .copied()
                .filter(|candidate| candidate.specialization == Some(specialization))
                .collect()
        })
        .unwrap_or_default();
    let pool = if same_trade.is_empty() {
        others
    } else {
        same_trade
    };

    let score = |candidate: &BreedingCandidate| {
        candidate.stats.leadership + candidate.stats.hunting + candidate.stats.building
    };
    let mut best: Option<&BreedingCandidate> = None;
    for candidate in pool {
        if best.is_none_or(|current| score(candidate) > score(current)) {
            best = Some(candidate);
        }
    }
    best.map(|candidate| candidate.id.clone())
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
    colony.resources.planks = clamp_resource(colony.resources.planks, caps.planks);
    colony.resources.blocks = clamp_resource(colony.resources.blocks, caps.blocks);
    colony.resources.tools = clamp_resource(colony.resources.tools, caps.tools);
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
/// Planks a construction scaffold consumes at break-ground (P19 slice 1b). Kept
/// deliberately small: the 5-cat start banks only a trickle of refined materials, so a
/// den must stay affordable within a handful of wood-cutter cycles or the colony can
/// never grow. Gates growth without freezing it.
const SCAFFOLD_PLANK_COST: f64 = 2.0;
/// Dressed-stone blocks a construction scaffold consumes at break-ground.
const SCAFFOLD_BLOCK_COST: f64 = 2.0;

fn phase_14_promote_queued_jobs_and_break_ground(
    colony: &mut ColonyRuntime,
    gate: TickGate,
    world_seed: u32,
) {
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
            // P19 slice 1b build cost: breaking ground draws refined build materials
            // (planks + blocks) from the stores. If the wood-cutter/stone-prep benches
            // have not banked enough yet, the job stays Queued and retries on a later
            // tick — construction is gated on refined-material supply, not free. The cost
            // keys only off pile-invariant resource totals, so this stays deterministic.
            if colony.resources.planks < SCAFFOLD_PLANK_COST
                || colony.resources.blocks < SCAFFOLD_BLOCK_COST
            {
                continue;
            }

            let roll = roll_seeded(f64::from(movement_seed));
            movement_seed = roll.next_seed;

            // The footprint depends on the scaffold's type, so resolve it before
            // searching for a free site.
            let scaffold_type = match next_metadata {
                JobMetadata::Construction { building_type, .. } => {
                    scaffold_building_type(building_type)
                }
                _ => BuildingType::Den,
            };

            if let Some(site_local) =
                next_claimed_building_site(colony, roll.value, world_seed, scaffold_type)
            {
                // Spend the build materials only once a real site is committed.
                colony.resources.planks = (colony.resources.planks - SCAFFOLD_PLANK_COST).max(0.0);
                colony.resources.blocks = (colony.resources.blocks - SCAFFOLD_BLOCK_COST).max(0.0);
                let building_id = format!(
                    "building-{}-{}",
                    gate.processed_through,
                    colony.buildings.len() + 1
                );

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
        // Scouts random-walk rather than beeline a fixed frontier tile: the first leg
        // heads outward from wherever the scout is forming up (the anchor at promotion),
        // and phase 33 re-picks a fresh outward leg each time it arrives. Two extra draws
        // off the seeded movement chain keep the meander deterministic.
        let explore_site = if job.kind == JobKind::Explore {
            let from = job
                .assigned_cat
                .as_deref()
                .and_then(|cat_id| colony.cats.iter().find(|cat| cat.id == cat_id))
                .map_or_else(village_anchor_world, |cat| position_to_world(cat.position));
            let dir = roll_seeded(f64::from(movement_seed));
            let len = roll_seeded(f64::from(dir.next_seed));
            movement_seed = len.next_seed;
            Some(scout_wander_target(
                from,
                village_anchor_world(),
                dir.value,
                len.value,
            ))
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

        if let Some(cat_id) = colony.jobs[job_index].assigned_cat.clone() {
            // Gathering cats head toward the pile they will ultimately haul to (nearest
            // designated pile accepting their yield); every other job still forms up at the
            // village anchor. With no designated piles both resolve to the anchor, so this is
            // byte-identical to pre-haul-fill.
            let dest = if matches!(
                job.kind,
                JobKind::HuntExpedition | JobKind::Quarry | JobKind::FetchWater
            ) && let Some(cat_pos) = colony
                .cats
                .iter()
                .find(|cat| cat.id == cat_id)
                .map(|cat| position_to_world(cat.position))
            {
                haul_destination(colony, carrying_kind_for_job(job.kind), cat_pos)
            } else {
                village_anchor_world()
            };
            if let Some(cat) = colony.cats.iter_mut().find(|cat| cat.id == cat_id) {
                cat.destination = Some(position_from_world(dest));
                cat.activity = CatActivity::Traveling;
                cat.current_task = task_for_job(job.kind);
            }
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
    // The P16 raw-material craft benches (wood-cutter/stone-prep/woodworking) have no TS
    // equivalent and no labour goal of their own; fold their staffing need into the
    // ported `AssignWorkshop` goal (P16.x) so a founding colony's first idle cats can
    // actually claim one instead of the fill pass exhausting every idle cat on
    // Hunt/Scout/Quarry before phase 23's non-sticky bench mop-up ever sees a candidate.
    let craft_benches_needing_workers: u32 = RAW_MATERIAL_WORKSHOPS
        .iter()
        .map(|&bench_type| buildings_needing_workers(colony, bench_type).len() as u32)
        .sum();

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
            as u32
            + craft_benches_needing_workers,
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
    // Free the raw-material benches before drafting labour so critical hunt/water work
    // always outbids a refinement task for the scarce founding cats. Phase 23 re-fills
    // whichever benches still have a genuinely idle cat left over.
    release_raw_material_workshop_workers(colony);

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
    // Include the raw-material benches in the same queue AssignWorkshop draws from: the
    // release call just above frees them, so — for a bench with no prior worker — this
    // is what finally lets the founding colony's leader bind a cat to one instead of
    // relying solely on phase 23's leftover-idle mop-up, which never sees a candidate
    // while the idle-employment-floor fill pass (below `direct_colony`) keeps claiming
    // every idle cat for Hunt/Scout/Quarry first.
    let mut workshop_queue = buildings_needing_workers(colony, BuildingType::Workshop);
    for bench_type in RAW_MATERIAL_WORKSHOPS {
        workshop_queue.extend(buildings_needing_workers(colony, bench_type));
    }
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
                        true,
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
                        true,
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
    // Idle mop-up (P12.4a/b). The generic workshop, the accounting tent, and the P16
    // raw-material chains (wood-cutter / stone-prep / woodworking) are all filled here
    // from cats that phase 20 left genuinely idle. The raw chains were released back to
    // the labour pool at the top of phase 20 (`release_raw_material_workshop_workers`),
    // so critical hunt/water work always wins the cats first — the mop-up only claims a
    // true surplus and never sticky-binds a cat while food or water go unworked. Their
    // staffing is announced quietly (no per-tick event spam) because the release/re-staff
    // cadence re-touches the same benches every tick.
    auto_staff_idle_buildings(colony, BuildingType::Workshop, gate.processed_through, true);
    auto_staff_idle_buildings(
        colony,
        BuildingType::AccountingTent,
        gate.processed_through,
        true,
    );
    // Fund construction first: staff the scarcer of the two build-material benches
    // (wood-cutter → planks, stone-prep → blocks) so a single spare cat keeps planks and
    // blocks balanced enough to break ground. The woodworking (tools) bench is a luxury
    // tier that only draws a cat once both build materials are already stocked — leaving
    // it earlier would let it drain the planks/blocks the colony needs to grow. The order
    // keys only off pile-invariant resource totals, so it stays deterministic.
    let (first_bench, second_bench) = if colony.resources.planks <= colony.resources.blocks {
        (BuildingType::WoodCutter, BuildingType::StonePrep)
    } else {
        (BuildingType::StonePrep, BuildingType::WoodCutter)
    };
    auto_staff_idle_buildings(colony, first_bench, gate.processed_through, false);
    auto_staff_idle_buildings(colony, second_bench, gate.processed_through, false);
    auto_staff_idle_buildings(
        colony,
        BuildingType::Woodworking,
        gate.processed_through,
        false,
    );

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
                    // Pile the fresh refined goods at the nearest accepting stockpile to this
                    // workshop (P12.4a). Only touches pile contents, never `resources`; with
                    // no designated piles this lands in the shrine reservoir exactly as before.
                    let site = colony.buildings[building_index].position;
                    route_output_to_nearest_pile(
                        colony,
                        ResourceKind::Refined,
                        step.refined_produced,
                        site,
                    );
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
                    // Route the forged gear to the nearest accepting stockpile to the smithy
                    // (P12.4a) — pile-only, `resources` unchanged, shrine fallback with no piles.
                    let site = colony.buildings[building_index].position;
                    route_output_to_nearest_pile(
                        colony,
                        ResourceKind::Weapons,
                        step.weapons_produced,
                        site,
                    );
                    route_output_to_nearest_pile(
                        colony,
                        ResourceKind::Armor,
                        step.armor_produced,
                        site,
                    );
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
            BuildingType::WoodCutter => {
                // P12.4b: raw materials → planks, on the refinement-workshop cadence.
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
                    colony.resources.planks += step.refined_produced;
                    append_event(
                        colony,
                        gate.processed_through,
                        EventKind::Other("production".to_owned()),
                        format!(
                            "The wood-cutter split {} materials into {} plank{}.",
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
            BuildingType::StonePrep => {
                // P12.4b: raw materials → dressed stone blocks.
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
                    colony.resources.blocks += step.refined_produced;
                    append_event(
                        colony,
                        gate.processed_through,
                        EventKind::Other("production".to_owned()),
                        format!(
                            "The stone-prep shop dressed {} materials into {} block{}.",
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
            BuildingType::Woodworking => {
                // P12.4b: planks + blocks → tools (twin-input crafter).
                let worker = assigned_worker(colony, &building_id);
                let step = advance_woodworking(
                    colony.buildings[building_index].production_progress,
                    production_elapsed,
                    WoodworkingOptions {
                        has_worker: worker.is_some(),
                        worker_is_architect: worker.is_some_and(|cat| {
                            cat.specialization == Some(CatSpecialization::Architect)
                        }),
                        planks_available: colony.resources.planks,
                        blocks_available: colony.resources.blocks,
                    },
                );
                if step.tools_produced > 0.0 {
                    colony.resources.planks = (colony.resources.planks - step.planks_used).max(0.0);
                    colony.resources.blocks = (colony.resources.blocks - step.blocks_used).max(0.0);
                    colony.resources.tools += step.tools_produced;
                    append_event(
                        colony,
                        gate.processed_through,
                        EventKind::Other("production".to_owned()),
                        format!(
                            "The woodworkers crafted {} tool{} from planks and blocks.",
                            step.tools_produced,
                            if step.tools_produced == 1.0 { "" } else { "s" }
                        ),
                    );
                }
                colony.buildings[building_index].production_progress = step.next_progress;
            }
            _ => {}
        }
    }

    // Refresh the reported stock ledger (P12.4a). A staffed Accounting Tent recounts it to the
    // exact current stock every tick; otherwise it lags and recounts on an interval. This only
    // touches `stock_ledger`, never the true `resources`.
    let staffed = has_staffed_accounting_tent(colony);
    refresh_ledger(
        &mut colony.stock_ledger,
        &colony.resources,
        staffed,
        gate.processed_through,
    );
}

/// Deposit a produced `amount` of `kind` into the nearest accepting stockpile to `at`
/// (P12.4a inter-workshop routing). Pile-contents only — the caller has already credited the
/// authoritative `resources`, so `resources` is never touched here and stays byte-identical.
/// With no designated player piles this resolves to the shrine reservoir, matching pre-P12.4a.
fn route_output_to_nearest_pile(
    colony: &mut ColonyRuntime,
    kind: ResourceKind,
    amount: f64,
    at: TilePos,
) {
    if amount <= 0.0 {
        return;
    }
    if let Some(idx) =
        stockpiles::deposit_index(&colony.stockpiles, kind, f64::from(at.x), f64::from(at.y))
    {
        stockpiles::add_resource(&mut colony.stockpiles[idx].contents, kind, amount);
    }
}

/// Whether a completed Accounting Tent is staffed by a living cat (its bookkeeper keeps the
/// stock ledger exact each tick).
fn has_staffed_accounting_tent(colony: &ColonyRuntime) -> bool {
    colony.buildings.iter().any(|building| {
        building.building_type == BuildingType::AccountingTent
            && building.construction_progress >= 100
            && assigned_worker(colony, &building.id).is_some()
    })
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
///
/// Ported from `server/game.ts:workerTick`'s per-cat `applySurvivalTick` loop
/// (`lib/game/survival.ts`), which is the sole mutator of `cat.needs` on the
/// map-first god-sim path (there is no per-cat "eating" event — hunger/thirst
/// simply decay slower and regenerate toward 90 while the colony's shared food/
/// water store holds any amount above zero, and decay at the full rate with no
/// regen once a store is empty). The threshold model itself is deterministic —
/// no RNG rolls, so none are drawn on any forked chain here.
///
/// Iterates alive cats in stable cats-vector order. On death: the carrier's
/// carried yield (if any) is salvaged via the same `credit_carrying` deposit
/// path shrine hauling uses (nearest accepting pile, else the shrine anchor),
/// the cat's own active/queued jobs are cancelled so none are left assigned to
/// a cat that will never return (`server/game.ts:retireCat`), and activity/
/// destination/carrying are cleared the same way phase 6's old-age death
/// clears them. Old-age death (phase 6) does not currently emit any event in
/// this port, so there is no existing event pattern to match for cause; the
/// death event here uses `EventKind::Other("death")` with TS's exact cause
/// string, and reuses `EventKind::ResourceCrisis`/`ResourceRecovered` for the
/// dehydration start/recovery edges — the same two variants phase 8's colony
/// water-crisis event uses, matching TS's shared `"crisis"`/`"recovery"` event
/// type strings.
fn phase_25_survival_deaths_and_carried_yield_salvage(
    colony: &mut ColonyRuntime,
    gate: TickGate,
    policy: TickPolicy,
) {
    let elapsed_sec = gate.elapsed_sec as f64 * normalize_resource_decay_multiplier(colony);
    let resources = SurvivalResources {
        food: colony.resources.food,
        water: colony.resources.water,
    };

    let cat_ids: Vec<CatId> = alive_cats(&colony.cats).map(|cat| cat.id.clone()).collect();

    for cat_id in cat_ids {
        let Some(index) = colony
            .cats
            .iter()
            .position(|cat| cat.id == cat_id && cat.death_time.is_none())
        else {
            continue;
        };

        let result = apply_survival_tick(
            &colony.cats[index].needs,
            resources,
            elapsed_sec,
            policy.config,
        );
        colony.cats[index].needs = result.next_needs.clone();
        let cat_name = colony.cats[index].name.clone();

        if result.dehydrating_started {
            append_event(
                colony,
                gate.processed_through,
                EventKind::ResourceCrisis,
                format!("{cat_name} started dehydrating."),
            );
        }

        if result.recovered_from_dehydration {
            append_event(
                colony,
                gate.processed_through,
                EventKind::ResourceRecovered,
                format!("{cat_name} recovered from dehydration."),
            );
        }

        if !result.died {
            continue;
        }

        // A dying carrier's yield is salvaged rather than lost, before the
        // cleanup below clears `carrying`.
        if let Some(carrying) = colony.cats[index].carrying.clone() {
            let deposit_at = position_to_world(colony.cats[index].position);
            credit_carrying(colony, &carrying, deposit_at);
        }

        // Cancel the dying cat's own active/queued jobs (mirrors `retireCat`)
        // so none are left stuck waiting on an assigned cat that is now dead.
        cancel_cat_jobs(colony, &cat_id, gate.processed_through);

        let died_of_thirst = result.next_needs.thirst == 0.0;
        let died_of_hunger = result.next_needs.hunger == 0.0;
        let cause = match (died_of_thirst, died_of_hunger) {
            (true, true) => "starvation and dehydration",
            (true, false) => "dehydration",
            _ => "starvation",
        };

        // Mirror phase 6's old-age death cleanup exactly.
        let cat = &mut colony.cats[index];
        cat.death_time = Some(gate.processed_through);
        cat.activity = CatActivity::default();
        cat.destination = None;
        cat.carrying = None;

        append_event(
            colony,
            gate.processed_through,
            EventKind::Other("death".to_owned()),
            format!("{cat_name} died from {cause}."),
        );
    }
}

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
        let carrying_kind = carrying_kind_for_job(job.kind);
        let haul_from = position_to_world(colony.cats[cat_index].position);
        let haul_to = haul_destination(colony, carrying_kind, haul_from);
        colony.cats[cat_index].carrying = Some(Carrying {
            kind: carrying_kind,
            amount: share,
            job_ended_at: gate.processed_through,
        });
        colony.cats[cat_index].destination = Some(position_from_world(haul_to));
        colony.cats[cat_index].activity = CatActivity::Returning;
    }
}

/// Phase 32: prepare movement inputs and optionally queue village expansion.
fn phase_32_movement_setup_and_village_expansion_queue(
    colony: &mut ColonyRuntime,
    gate: TickGate,
    policy: TickPolicy,
    world_seed: u32,
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
        world_seed,
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
            // Deposit once the carrier reaches its haul destination — the pile it walked to,
            // or the shrine anchor when no designated pile accepts the resource. With no
            // designated piles this is exactly the shrine anchor, matching pre-haul-fill.
            let deposit_target = haul_destination(colony, carrying.kind, world_pos);
            if !should_deposit(&carrying, world_pos, deposit_target, gate.processed_through) {
                continue;
            }

            credit_carrying(colony, &carrying, world_pos);
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

        // Random-walk scouting: a cat still on an explore job that has reached its last
        // wander target picks a fresh outward leg (it may be Idle or Working after an
        // arrival). This meanders the scout across new fog until its explore job
        // completes, at which point phase 30 turns it around toward the shrine.
        if colony.cats[cat_index].current_task == Some(TaskType::Explore)
            && let Some(target) = next_scout_leg(colony, &cat_id, movement)
        {
            colony.cats[cat_index].destination = Some(position_from_world(target));
            colony.cats[cat_index].activity = CatActivity::Traveling;
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
        mountains_unlocked: is_owned(&colony.upgrade_tree, MOUNTAINEERING_NODE_ID),
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
        let standing_tile_pos = world_pos_to_tile(world_pos);
        let standing_tile = colony.world_tiles.get(&standing_tile_pos);
        let explore_slowdown =
            if current_task == Some(TaskType::Explore) && activity == CatActivity::Traveling {
                EXPLORE_SPEED_FACTOR
            } else {
                1.0
            };
        // Per-cat effective rate: base × terrain surface (the tile the cat is on)
        // × per-cat gait × life-stage gait. This desyncs the herd — cats on slow
        // ground or with a slow gait fall behind instead of stepping in unison.
        let standing_biome = crate::terrain_gen::tile_biome(
            movement.world_seed,
            standing_tile_pos.x,
            standing_tile_pos.y,
        );
        let stage = get_life_stage(colony.cats[cat_index].age_hours);
        // Paved stone roads (×1.75) and worn dirt roads (×1.05) carry cats faster
        // than open ground; the road network is the fast lane for the haul loop.
        let road_mult = standing_tile.map_or(1.0, |tile| {
            road_surface_multiplier(
                tile.overlay_feature.as_deref() == Some("road_built"),
                tile.path_wear,
            )
        });
        let speed = effective_move_speed(standing_biome, &cat_id, stage)
            * road_mult
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
    // Drop player piles; the end-of-tick reconcile reseeds the shrine reservoir.
    colony.stockpiles.clear();
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
        planks: 0.0,
        blocks: 0.0,
        tools: 0.0,
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

/// Draw the next roll from the forked life-sim chain, advancing `seed` in place.
fn next_life_roll(seed: &mut u32) -> f64 {
    let roll = roll_seeded(f64::from(*seed));
    *seed = roll.next_seed;
    roll.value
}

/// Total cats the village can currently shelter (shrine + completed dens), for the
/// breeding gate's housing headroom check. Computed independently of phase 18's
/// `LeaderSnapshot` since phase 6 (life sim) runs earlier in the tick.
fn colony_housing_capacity(colony: &ColonyRuntime) -> f64 {
    let housing_buildings: Vec<crate::housing::HousingBuilding> = colony
        .buildings
        .iter()
        .map(|building| {
            crate::housing::HousingBuilding::new(
                building.building_type,
                f64::from(building.level),
                f64::from(building.construction_progress),
            )
        })
        .collect();
    let effects = resolve_effects(colony.upgrade_tree.owned_node_ids.iter());
    crate::housing::housing_capacity(&housing_buildings, effects.housing_per_den)
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
    resources.planks = clamp_resource(resources.planks, caps.planks);
    resources.blocks = clamp_resource(resources.blocks, caps.blocks);
    resources.tools = clamp_resource(resources.tools, caps.tools);
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
                    BuildingType::Workshop
                        | BuildingType::Smithy
                        | BuildingType::WoodCutter
                        | BuildingType::StonePrep
                        | BuildingType::Woodworking
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

fn staff_building(
    colony: &mut ColonyRuntime,
    building_id: &str,
    cat_id: &str,
    now_ms: i64,
    announce: bool,
) {
    if let Some(building) = colony
        .buildings
        .iter_mut()
        .find(|building| building.id == building_id)
    {
        building.assigned_cat = Some(cat_id.to_owned());
    }
    // The raw-material benches are released and re-staffed every tick, so they pass
    // `announce = false` to avoid flooding the log with a per-tick "worker assigned".
    if announce {
        append_event(
            colony,
            now_ms,
            EventKind::Other("worker_assigned".to_owned()),
            "The leader assigned a worker.",
        );
    }
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

/// Next outward wander leg for a scout whose explore job is still running, or `None`
/// once the job is done (so the completing job's return-to-shrine takes over). Two
/// draws off the shared seeded movement chain keep the meander deterministic.
fn next_scout_leg(
    colony: &ColonyRuntime,
    cat_id: &str,
    movement: &mut MovementPassContext,
) -> Option<WorldPos> {
    let has_active_explore = colony.jobs.iter().any(|job| {
        job.kind == JobKind::Explore
            && job.status == JobStatus::Active
            && job.assigned_cat.as_deref() == Some(cat_id)
    });
    if !has_active_explore {
        return None;
    }
    let from = colony
        .cats
        .iter()
        .find(|cat| cat.id == cat_id)
        .map(|cat| position_to_world(cat.position))?;
    let dir = next_movement_roll(movement);
    let len = next_movement_roll(movement);
    Some(scout_wander_target(from, village_anchor_world(), dir, len))
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
        TileType::Mountains => WalkTileType::Mountain,
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
            let walked_on = walked_keys.contains(&pos);
            let in_halo = !walked_on
                && walked_tiles.iter().any(|walked| {
                    (walked.x - pos.x).abs().max((walked.y - pos.y).abs()) <= reveal_radius
                });
            if !walked_on && !in_halo {
                continue;
            }
            // Reveal regardless of whether the tile is materialised in `world_tiles`
            // (the live colony's map is sparse); only bump wear on tiles that exist.
            colony.revealed_tiles.insert(pos);
            if let Some(tile) = colony.world_tiles.get_mut(&pos) {
                if walked_on {
                    tile.path_wear = add_path_wear(tile.path_wear, WALK_WEAR).max(64);
                } else {
                    tile.path_wear = tile.path_wear.max(63);
                }
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
        | BuildingType::Barracks
        | BuildingType::AccountingTent => building_type,
        _ => BuildingType::Den,
    }
}

/// Pick a free anchor for a `building_type` footprint. Deterministic: same colony
/// state + roll + seed → same site. A site fits when its whole `w x h` footprint lies
/// inside `claimed_tiles` and every covered tile is free (no building, water, or tree;
/// footprint-within-claimed also keeps it off the fence perimeter). `roll` indexes
/// among all fitting anchors, scanned in `(y, x)` order, matching the legacy roll
/// semantics. When nothing fits inside the village, spiral outward past the fence
/// (still footprint-aware) so a build can defer gracefully rather than crash.
fn next_claimed_building_site(
    colony: &ColonyRuntime,
    roll: f64,
    world_seed: u32,
    building_type: BuildingType,
) -> Option<TilePos> {
    let (w, h) = footprint_for(building_type);
    let claimed: HashSet<TilePos> = colony.claimed_tiles.iter().copied().collect();
    let occupied = occupied_building_tiles(colony);
    // Fields/farms only take on fertile ground (grass/meadow/marsh). Rock, sand,
    // tundra, forest, and water are barren, so a field site must be farmable.
    let require_farmable = building_type == BuildingType::Field;

    let footprint_free_at = |anchor: TilePos, require_claimed: bool| -> bool {
        footprint_tiles(anchor, w, h).into_iter().all(|tile| {
            (!require_claimed || claimed.contains(&tile))
                && !occupied.contains(&tile)
                && !tile_has_water(colony.world_tiles.get(&tile))
                && !crate::terrain_gen::tile_has_tree(world_seed, tile.x, tile.y)
                && (!require_farmable || tile_is_farmable(colony.world_tiles.get(&tile)))
        })
    };

    let mut free = colony
        .claimed_tiles
        .iter()
        .copied()
        .filter(|anchor| footprint_free_at(*anchor, true))
        .collect::<Vec<_>>();
    free.sort_by_key(|site| (site.y, site.x));
    free.dedup();

    if !free.is_empty() {
        let clamped = roll.clamp(0.0, 0.999_999);
        return Some(free[(clamped * free.len() as f64).floor() as usize]);
    }

    // No room left inside the fence: spiral outward, still refusing occupied ground.
    next_building_site_with_blocked(&[], roll, DEFAULT_MAX_RING, |local| {
        let anchor = colony_to_world(local);
        !footprint_free_at(
            TilePos {
                x: anchor.x,
                y: anchor.y,
            },
            false,
        )
    })
    .map(colony_to_world)
    .map(|site| TilePos {
        x: site.x,
        y: site.y,
    })
}

fn tile_has_water(tile: Option<&WorldTileRuntime>) -> bool {
    tile.is_some_and(|tile| {
        tile.tile_type == TileType::River
            || tile.overlay_feature.as_deref() == Some("river")
            || tile.resources.water > 0
    })
}

/// Whether a field/farm may be sown on this ground. Mirrors the climate biome
/// fertility table (`climate::BiomeClimate::farmable`): grass, meadow, and marsh
/// (swamp) are fertile; rock (mountains), sand (desert), tundra, forest, cave,
/// enemy, and water tiles are barren. An unrevealed/absent tile is not farmable.
fn tile_is_farmable(tile: Option<&WorldTileRuntime>) -> bool {
    tile.is_some_and(|tile| {
        matches!(
            tile.tile_type,
            TileType::Field | TileType::Meadow | TileType::Swamp
        ) && tile.overlay_feature.as_deref() != Some("river")
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

fn tile_is_explored(tile: &WorldTileRuntime) -> bool {
    tile.path_wear > 62 || cheb_from_anchor(tile.pos) <= 6
}

fn cheb_from_anchor(pos: TilePos) -> i32 {
    (pos.x - VILLAGE_ANCHOR.x)
        .abs()
        .max((pos.y - VILLAGE_ANCHOR.y).abs())
}

/// The P12.4b raw-material refinement benches. The leader staffs these as a mop-up
/// (non-sticky) set: every tick they are released back to the labour pool at the top
/// of phase 20 and re-filled from the leftover idle surplus in phase 23, so food and
/// water work always draws the cats first. First-time staffing (P16.x) instead goes
/// through phase 20's `AssignWorkshop` goal — see the `craft_benches_needing_workers`
/// local in `phase_18_leader_snapshot_assembly`, folded into the snapshot's
/// `workshops_needing_workers` — which is what actually gets a founding colony's cats
/// onto an empty bench on the very first tick. Once a bench has a worker, phase 23's
/// mop-up keeps it staffed tick over tick.
const RAW_MATERIAL_WORKSHOPS: [BuildingType; 3] = [
    BuildingType::WoodCutter,
    BuildingType::StonePrep,
    BuildingType::Woodworking,
];

/// Release every cat bound to a raw-material refinement bench so the leader's labour
/// pass can re-draft them for hunting/water first. Whatever remains genuinely idle is
/// re-staffed by phase 23's mop-up (or claimed directly by `AssignWorkshop` in phase 20
/// on the tick a bench first needs a worker). Keeps the benches non-sticky (survival
/// guardrail).
fn release_raw_material_workshop_workers(colony: &mut ColonyRuntime) {
    for building in &mut colony.buildings {
        if RAW_MATERIAL_WORKSHOPS.contains(&building.building_type) {
            building.assigned_cat = None;
        }
    }
}

fn auto_staff_idle_buildings(
    colony: &mut ColonyRuntime,
    building_type: BuildingType,
    now_ms: i64,
    announce: bool,
) {
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
        staff_building(colony, &building_id, &cat_id, now_ms, announce);
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
    } else if matches!(
        job.kind,
        JobKind::HuntExpedition | JobKind::Quarry | JobKind::FetchWater
    ) {
        // The completing gatherer carries its final trip's yield home; route it to the pile
        // that yield belongs in (nearest designated pile accepting it), falling back to the
        // shrine anchor when none does — byte-identical with no designated piles.
        let from = position_to_world(colony.cats[cat_index].position);
        haul_destination(colony, carrying_kind_for_job(job.kind), from)
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

/// The shrine reservoir's footprint at this world's village anchor.
fn shrine_stockpile_rect() -> ZoneRect {
    stockpiles::shrine_rect(VILLAGE_ANCHOR.x, VILLAGE_ANCHOR.y)
}

/// Restore the stockpile balancing-reservoir invariant for a colony (seeds the shrine
/// reservoir if absent). Runs at the end of every tick and after stockpile actions.
pub fn reconcile_colony_stockpiles(colony: &mut ColonyRuntime) {
    stockpiles::reconcile(
        &mut colony.stockpiles,
        &colony.resources,
        shrine_stockpile_rect(),
    );
}

/// Map a carried resource to its stockpile [`ResourceKind`], if it can be stockpiled.
/// Blessings fund the global upgrade pool and never enter piles (see [`credit_carrying`]),
/// so they have no kind — carriers of Blessings always fall back to the shrine anchor.
fn carrying_resource_kind(kind: CarryingKind) -> Option<ResourceKind> {
    match kind {
        CarryingKind::Food => Some(ResourceKind::Food),
        CarryingKind::Materials => Some(ResourceKind::Materials),
        CarryingKind::Water => Some(ResourceKind::Water),
        CarryingKind::Blessings => None,
    }
}

/// Where a cat carrying `carrying_kind` (picked up at `from_pos`) should haul to: the nearest
/// *designated* stockpile that accepts the resource (Euclidean distance from `from_pos`,
/// tie-broken by stockpile id for determinism), or the shrine anchor when none accepts it.
///
/// The shrine reservoir is only the fallback, never a preferred target, so designated piles
/// win when they accept the resource. Selection uses no RNG. **With no designated piles (only
/// the shrine reservoir) this always returns the shrine anchor**, so hauling stays
/// byte-identical to pre-haul-fill.
fn haul_destination(
    colony: &ColonyRuntime,
    carrying_kind: CarryingKind,
    from_pos: WorldPos,
) -> WorldPos {
    let Some(kind) = carrying_resource_kind(carrying_kind) else {
        return village_anchor_world();
    };
    let mut best: Option<(&Stockpile, f64)> = None;
    for pile in &colony.stockpiles {
        if pile.is_shrine() || !pile.accepts.contains(&kind) {
            continue;
        }
        let (cx, cy) = pile.center();
        let dist = (cx - from_pos.x).powi(2) + (cy - from_pos.y).powi(2);
        let better = match best {
            None => true,
            Some((best_pile, best_dist)) => {
                dist < best_dist || (dist == best_dist && pile.id < best_pile.id)
            }
        };
        if better {
            best = Some((pile, dist));
        }
    }
    match best {
        Some((pile, _)) => {
            let (cx, cy) = pile.center();
            WorldPos { x: cx, y: cy }
        }
        None => village_anchor_world(),
    }
}

/// Cancel a dead cat's active/queued jobs so none are left stuck waiting on an
/// assigned cat that no longer exists (mirrors TS `retireCat`). Shared by every
/// death path — old-age (phase 6 pass 1) and survival (phase 25).
fn cancel_cat_jobs(colony: &mut ColonyRuntime, cat_id: &str, now_ms: i64) {
    for job in &mut colony.jobs {
        if job.assigned_cat.as_deref() == Some(cat_id)
            && matches!(job.status, JobStatus::Active | JobStatus::Queued)
        {
            job.status = JobStatus::Cancelled;
            job.completed_at = Some(now_ms);
        }
    }
}

fn credit_carrying(colony: &mut ColonyRuntime, carrying: &Carrying, deposit_at: WorldPos) {
    // Blessings never enter `resources` (they fund the global upgrade pool), so they are
    // not placed in a pile — keeping `sum(piles) == resources` intact.
    let kind = match carrying.kind {
        CarryingKind::Food => ResourceKind::Food,
        CarryingKind::Materials => ResourceKind::Materials,
        CarryingKind::Water => ResourceKind::Water,
        CarryingKind::Blessings => {
            colony.global_upgrade_points += carrying.amount;
            return;
        }
    };

    stockpiles::add_resource(&mut colony.resources, kind, carrying.amount);
    if let Some(idx) =
        stockpiles::deposit_index(&colony.stockpiles, kind, deposit_at.x, deposit_at.y)
    {
        stockpiles::add_resource(&mut colony.stockpiles[idx].contents, kind, carrying.amount);
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
                    planks: 0.0,
                    blocks: 0.0,
                    tools: 0.0,
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
                    planks: 0.0,
                    blocks: 0.0,
                    tools: 0.0,
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
            world_seed: 123,
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
                    planks: 0.0,
                    blocks: 0.0,
                    tools: 0.0,
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
    fn founded_colony_elects_a_leader_over_time_without_player_input() {
        let mut world = new_world(2024);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 1_000, 2024));

        // Tick across more than one full election window (~30 game-min) with no
        // ballots cast and no player actions of any kind.
        let mut now = 1_000;
        for _ in 0..40 {
            now += 60_000;
            let _ = world_tick(&mut world, now);
        }

        let colony = &world.colonies[0];
        // A scheduled leadership election opened and resolved entirely on its own.
        assert!(
            colony.elections.iter().any(|election| {
                matches!(election.kind, ElectionKind::Scheduled)
                    && election.resolved_at.is_some()
                    && election.winner_cat_id.is_some()
            }),
            "a term election auto-resolved with a winner: {:?}",
            colony.elections
        );
        // The colony holds a seated leader, and it is a live cat.
        let leader = colony
            .leader_id
            .clone()
            .expect("colony has a seated leader");
        assert!(
            alive_cats(&colony.cats).any(|cat| cat.id == leader),
            "the elected leader is a living cat"
        );
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

    // ---- P12.3 spatial stockpiles ----

    fn designated_pile(id: &str, rect: ZoneRect, accepts: &[ResourceKind]) -> Stockpile {
        Stockpile {
            id: id.to_owned(),
            rect,
            accepts: accepts.iter().copied().collect(),
            contents: Resources::default(),
        }
    }

    #[test]
    fn stockpile_contents_sum_to_resources_every_tick() {
        let mut world = new_world(424_242);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 1_000, 9_090));
        world.colonies[0].stockpiles.push(designated_pile(
            "stockpile-a",
            ZoneRect {
                x1: 7,
                y1: 7,
                x2: 8,
                y2: 8,
            },
            &[ResourceKind::Food],
        ));

        for step in 1..=40 {
            let now = 1_000 + i64::from(step) * 60_000;
            let _ = world_tick(&mut world, now);
            let colony = &world.colonies[0];
            assert!(
                colony.stockpiles.iter().any(Stockpile::is_shrine),
                "shrine reservoir present at step {step}"
            );
            for &kind in ResourceKind::ALL {
                let sum: f64 = colony
                    .stockpiles
                    .iter()
                    .map(|pile| stockpiles::resource_amount(&pile.contents, kind))
                    .sum();
                let total = stockpiles::resource_amount(&colony.resources, kind);
                assert!(
                    (sum - total).abs() <= 1e-6,
                    "{kind:?}: pile sum {sum} != resources {total} at step {step}"
                );
            }
        }
    }

    #[test]
    fn a_designated_pile_does_not_change_the_resource_trajectory() {
        // #1 regression: stockpiles are a view, never the economy. A designated pile
        // (which reroutes deposits) must leave `resources` bit-identical to the
        // shrine-only baseline every tick.
        let mut plain = new_world(31_337);
        plain
            .colonies
            .push(found_colony(plain.world_seed, "colony-1", 1_000, 4_242));
        let mut with_pile = new_world(31_337);
        with_pile
            .colonies
            .push(found_colony(with_pile.world_seed, "colony-1", 1_000, 4_242));
        with_pile.colonies[0].stockpiles.push(designated_pile(
            "stockpile-a",
            ZoneRect {
                x1: 6,
                y1: 6,
                x2: 7,
                y2: 7,
            },
            &[
                ResourceKind::Food,
                ResourceKind::Water,
                ResourceKind::Materials,
            ],
        ));

        for step in 1..=40 {
            let now = 1_000 + i64::from(step) * 60_000;
            let _ = world_tick(&mut plain, now);
            let _ = world_tick(&mut with_pile, now);
            let baseline = &plain.colonies[0].resources;
            let observed = &with_pile.colonies[0].resources;
            for &kind in ResourceKind::ALL {
                assert_eq!(
                    stockpiles::resource_amount(baseline, kind).to_bits(),
                    stockpiles::resource_amount(observed, kind).to_bits(),
                    "{kind:?} diverged at step {step}"
                );
            }
        }
    }

    // ---- Haul-fill: carrying cats deliver to designated stockpiles ----

    fn tile_rect(x: i32, y: i32) -> ZoneRect {
        ZoneRect {
            x1: x,
            y1: y,
            x2: x,
            y2: y,
        }
    }

    fn haul_movement_ctx() -> MovementPassContext {
        MovementPassContext {
            movement_seed: movement_seed(1),
            movement_elapsed: 0.0,
            // No wandering, so a deposited carrier keeps a stable (None) destination.
            wander_chance: 0.0,
            ring_radius: 4,
            claimed_area: Default::default(),
            area_gate: None,
            gate: pos(6, 10),
            walk_tiles: Vec::new(),
            zones: Vec::new(),
            world_seed: 123,
        }
    }

    fn carrying_cat_at(id: &str, kind: CarryingKind, amount: f64, at: WorldPos) -> Cat {
        let mut cat = adult_idle_cat(id, "colony-1");
        cat.position = position_from_world(at);
        cat.activity = CatActivity::Returning;
        cat.carrying = Some(Carrying {
            kind,
            amount,
            job_ended_at: 0,
        });
        cat
    }

    #[test]
    fn haul_destination_falls_back_to_the_shrine_anchor_with_no_designated_piles() {
        // #1 regression: with only the shrine reservoir, hauling targets the anchor exactly —
        // byte-identical to pre-haul-fill regardless of where the carrier stands.
        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            ..ColonyRuntime::default()
        };
        reconcile_colony_stockpiles(&mut colony); // seeds only the shrine reservoir
        for from in [WorldPos { x: 6.0, y: 6.0 }, WorldPos { x: 40.0, y: 3.0 }] {
            assert_eq!(
                haul_destination(&colony, CarryingKind::Food, from),
                village_anchor_world()
            );
        }
    }

    #[test]
    fn haul_destination_picks_the_nearest_accepting_pile_then_ties_by_id() {
        let colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            stockpiles: vec![
                designated_pile("stockpile-b", tile_rect(10, 6), &[ResourceKind::Food]),
                designated_pile("stockpile-a", tile_rect(2, 6), &[ResourceKind::Food]),
                designated_pile("stockpile-c", tile_rect(20, 6), &[ResourceKind::Materials]),
            ],
            ..ColonyRuntime::default()
        };

        // Nearest accepting food pile to (9,6) is the one at (10,6).
        assert_eq!(
            haul_destination(&colony, CarryingKind::Food, WorldPos { x: 9.0, y: 6.0 }),
            WorldPos { x: 10.0, y: 6.0 }
        );
        // Equidistant from the anchor (6,6): (2,6) and (10,6) both at dist 16 → lower id wins.
        assert_eq!(
            haul_destination(&colony, CarryingKind::Food, WorldPos { x: 6.0, y: 6.0 }),
            WorldPos { x: 2.0, y: 6.0 }
        );
        // Materials only pile at (20,6) serves a materials carrier.
        assert_eq!(
            haul_destination(
                &colony,
                CarryingKind::Materials,
                WorldPos { x: 18.0, y: 6.0 }
            ),
            WorldPos { x: 20.0, y: 6.0 }
        );
    }

    #[test]
    fn haul_destination_skips_piles_that_reject_the_kind_and_blessings_never_pile() {
        let colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            stockpiles: vec![designated_pile(
                "stockpile-mat",
                tile_rect(10, 6),
                &[ResourceKind::Materials],
            )],
            ..ColonyRuntime::default()
        };

        // A food carrier ignores a materials-only pile and heads for the shrine anchor.
        assert_eq!(
            haul_destination(&colony, CarryingKind::Food, WorldPos { x: 9.0, y: 6.0 }),
            village_anchor_world()
        );
        // Blessings fund the global pool (never piled), so they always fall back to the anchor.
        assert_eq!(
            haul_destination(
                &colony,
                CarryingKind::Blessings,
                WorldPos { x: 9.0, y: 6.0 }
            ),
            village_anchor_world()
        );
    }

    #[test]
    fn carrying_cat_fills_the_designated_pile_it_reaches_not_the_shrine() {
        // A food carrier standing on its food pile deposits there this tick — the player pile
        // fills, the shrine reservoir does not.
        let pile_at = WorldPos { x: 10.0, y: 6.0 };
        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            cats: vec![carrying_cat_at("hauler", CarryingKind::Food, 8.0, pile_at)],
            ..ColonyRuntime::default()
        };
        reconcile_colony_stockpiles(&mut colony);
        colony.stockpiles.push(designated_pile(
            "stockpile-food",
            tile_rect(10, 6),
            &[ResourceKind::Food],
        ));

        // `now` well inside the grace window, so only reaching the pile (not force-deposit)
        // can trigger the deposit.
        let gate = production_gate(1, 1_000);
        phase_33_movement_deposits_and_no_destination_wander(
            &mut colony,
            gate,
            &mut haul_movement_ctx(),
        );

        let pile = colony
            .stockpiles
            .iter()
            .find(|p| p.id == "stockpile-food")
            .expect("food pile present");
        assert_eq!(pile.contents.food, 8.0, "player pile filled on arrival");
        let shrine = colony
            .stockpiles
            .iter()
            .find(|p| p.is_shrine())
            .expect("shrine present");
        assert_eq!(
            shrine.contents.food, 0.0,
            "goods went to the player pile, not the shrine reservoir"
        );
        assert_eq!(
            colony.resources.food, 8.0,
            "resources credited exactly once"
        );
        assert!(colony.cats[0].carrying.is_none(), "carrier unloaded");
    }

    #[test]
    fn carrying_cat_at_a_rejecting_pile_delivers_to_the_shrine() {
        // The only nearby pile rejects Food. haul_destination is the shrine anchor, so the
        // carrier only deposits once the grace window forces it — and the goods land in the
        // shrine reservoir, never the rejecting pile.
        let mat_pile_at = WorldPos { x: 10.0, y: 6.0 };
        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            cats: vec![carrying_cat_at(
                "hauler",
                CarryingKind::Food,
                5.0,
                mat_pile_at,
            )],
            ..ColonyRuntime::default()
        };
        reconcile_colony_stockpiles(&mut colony);
        colony.stockpiles.push(designated_pile(
            "stockpile-mat",
            tile_rect(10, 6),
            &[ResourceKind::Materials],
        ));

        // Past the grace window → force-deposit at the carrier's position.
        let gate = production_gate(1, crate::shrine::DEPOSIT_GRACE_MS + 1);
        phase_33_movement_deposits_and_no_destination_wander(
            &mut colony,
            gate,
            &mut haul_movement_ctx(),
        );

        let mat_pile = colony
            .stockpiles
            .iter()
            .find(|p| p.id == "stockpile-mat")
            .expect("materials pile present");
        assert_eq!(mat_pile.contents.food, 0.0, "rejecting pile stayed empty");
        let shrine = colony
            .stockpiles
            .iter()
            .find(|p| p.is_shrine())
            .expect("shrine present");
        assert_eq!(shrine.contents.food, 5.0, "food fell back to the shrine");
    }

    #[test]
    fn mid_job_haul_routes_the_carrier_toward_a_designated_food_pile() {
        // Same setup as `mid_job_hunt_haul_splits_total_and_sets_carrying`, but a designated
        // food pile exists: the carrier now heads for the pile (14,6) instead of the anchor.
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
                stockpiles: vec![designated_pile(
                    "stockpile-food",
                    tile_rect(14, 6),
                    &[ResourceKind::Food],
                )],
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

        let cat = &world.colonies[0].cats[0];
        assert_eq!(cat.activity, CatActivity::Returning);
        assert_eq!(
            cat.destination,
            Some(Position {
                map: MapType::World,
                x: 14.0,
                y: 6.0,
            }),
            "carrier routed to the food pile, not the anchor"
        );
    }

    #[test]
    fn no_designated_piles_hauling_trajectory_stays_in_the_shrine_bit_for_bit() {
        // #1 regression over a live founded colony (life sim + hunting/hauling active): with no
        // player piles, every resource sits in the shrine reservoir bit-for-bit each tick, so
        // the haul-fill routing is a pure no-op on the economy.
        let mut world = new_world(31_337);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 1_000, 4_242));

        for step in 1..=60 {
            let now = 1_000 + i64::from(step) * 60_000;
            let _ = world_tick(&mut world, now);
            let colony = &world.colonies[0];

            let player_piles = colony.stockpiles.iter().filter(|p| !p.is_shrine()).count();
            assert_eq!(player_piles, 0, "no player piles appear at step {step}");

            let shrine = colony
                .stockpiles
                .iter()
                .find(|p| p.is_shrine())
                .expect("shrine present");
            for &kind in ResourceKind::ALL {
                assert_eq!(
                    stockpiles::resource_amount(&shrine.contents, kind).to_bits(),
                    stockpiles::resource_amount(&colony.resources, kind).to_bits(),
                    "{kind:?}: shrine != resources at step {step}"
                );
            }
        }
    }

    // ---- P12.4a accountant + inter-workshop routing ----

    fn workshop_colony(with_refined_pile: bool) -> ColonyRuntime {
        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            resources: Resources {
                materials: 50.0,
                ..Resources::default()
            },
            cats: vec![adult_idle_cat("smith", "colony-1")],
            buildings: vec![BuildingRuntime {
                id: "workshop-1".to_owned(),
                building_type: BuildingType::Workshop,
                level: 1,
                position: TilePos { x: 12, y: 12 },
                is_complete: true,
                construction_progress: 100,
                production_progress: 590.0,
                assigned_cat: Some("smith".to_owned()),
            }],
            last_tick: 0,
            test_rng_seed: Some(1),
            ..ColonyRuntime::default()
        };
        // Seed the shrine reservoir at the anchor (6, 6), far from the workshop at (12, 12).
        reconcile_colony_stockpiles(&mut colony);
        if with_refined_pile {
            colony.stockpiles.push(designated_pile(
                "stockpile-refined",
                ZoneRect {
                    x1: 12,
                    y1: 12,
                    x2: 12,
                    y2: 12,
                },
                &[ResourceKind::Refined],
            ));
        }
        colony
    }

    fn production_gate(elapsed_sec: i64, processed_through: i64) -> TickGate {
        TickGate {
            elapsed_sec,
            processed_through,
            minute_rolled: false,
            previous_water: 0,
        }
    }

    #[test]
    fn workshop_output_routes_to_the_nearest_accepting_stockpile() {
        let mut colony = workshop_colony(true);

        // 30s completes one workshop cycle (590 + 30 ≥ 600): 5 materials → 1 refined.
        phase_23_production(&mut colony, production_gate(30, 30_000));

        assert_eq!(colony.resources.refined, 1.0);
        assert_eq!(colony.resources.materials, 45.0);
        let pile = colony
            .stockpiles
            .iter()
            .find(|pile| pile.id == "stockpile-refined")
            .expect("designated pile present");
        assert_eq!(
            pile.contents.refined, 1.0,
            "refined piled at the nearest stockpile to the workshop"
        );
        let shrine = colony
            .stockpiles
            .iter()
            .find(|pile| pile.is_shrine())
            .expect("shrine present");
        assert_eq!(
            shrine.contents.refined, 0.0,
            "output did not default to the shrine when a nearer pile accepts it"
        );
    }

    fn chain_colony(
        building_type: BuildingType,
        resources: Resources,
        staffed: bool,
    ) -> ColonyRuntime {
        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            resources,
            cats: vec![adult_idle_cat("crafter", "colony-1")],
            buildings: vec![BuildingRuntime {
                id: "chain-1".to_owned(),
                building_type,
                level: 1,
                position: TilePos { x: 12, y: 12 },
                is_complete: true,
                construction_progress: 100,
                production_progress: 590.0,
                assigned_cat: staffed.then(|| "crafter".to_owned()),
            }],
            last_tick: 0,
            test_rng_seed: Some(1),
            ..ColonyRuntime::default()
        };
        reconcile_colony_stockpiles(&mut colony);
        colony
    }

    #[test]
    fn wood_cutter_refines_materials_into_planks_when_it_has_a_worker() {
        // Staffed: 590 + 30 ≥ 600 completes one cycle → 5 materials become 1 plank.
        let mut staffed = chain_colony(
            BuildingType::WoodCutter,
            Resources {
                materials: 50.0,
                ..Resources::default()
            },
            true,
        );
        phase_23_production(&mut staffed, production_gate(30, 30_000));
        assert_eq!(staffed.resources.planks, 1.0);
        assert_eq!(staffed.resources.materials, 45.0);

        // FIXTURE UPDATE (P19 slice 1b): phase 23 now AUTO-STAFFS the raw-material benches
        // from any genuinely idle cat, so an "unstaffed" bench that has a free cat on hand
        // is mopped up and refines this very tick — the leader no longer leaves the shop
        // cold. (Pre-1b this branch expected planks == 0 because auto-staffing was deferred.)
        let mut auto_staffed = chain_colony(
            BuildingType::WoodCutter,
            Resources {
                materials: 50.0,
                ..Resources::default()
            },
            false,
        );
        phase_23_production(&mut auto_staffed, production_gate(30, 30_000));
        assert_eq!(auto_staffed.resources.planks, 1.0);
        assert_eq!(auto_staffed.resources.materials, 45.0);

        // With NO worker available at all (no cats to mop up), the bench still makes nothing.
        let mut no_worker = chain_colony(
            BuildingType::WoodCutter,
            Resources {
                materials: 50.0,
                ..Resources::default()
            },
            false,
        );
        no_worker.cats.clear();
        phase_23_production(&mut no_worker, production_gate(30, 30_000));
        assert_eq!(no_worker.resources.planks, 0.0);
        assert_eq!(no_worker.resources.materials, 50.0);
    }

    #[test]
    fn stone_prep_dresses_materials_into_blocks_when_staffed() {
        let mut colony = chain_colony(
            BuildingType::StonePrep,
            Resources {
                materials: 50.0,
                ..Resources::default()
            },
            true,
        );
        phase_23_production(&mut colony, production_gate(30, 30_000));
        assert_eq!(colony.resources.blocks, 1.0);
        assert_eq!(colony.resources.materials, 45.0);
    }

    #[test]
    fn woodworking_crafts_tools_from_planks_and_blocks_only_when_both_present() {
        // Both inputs present → one cycle consumes 2 planks + 2 blocks → 1 tool.
        let mut colony = chain_colony(
            BuildingType::Woodworking,
            Resources {
                planks: 10.0,
                blocks: 10.0,
                ..Resources::default()
            },
            true,
        );
        phase_23_production(&mut colony, production_gate(30, 30_000));
        assert_eq!(colony.resources.tools, 1.0);
        assert_eq!(colony.resources.planks, 8.0);
        assert_eq!(colony.resources.blocks, 8.0);

        // Missing blocks → the bench stalls, tools stay at zero.
        let mut starved = chain_colony(
            BuildingType::Woodworking,
            Resources {
                planks: 10.0,
                blocks: 0.0,
                ..Resources::default()
            },
            true,
        );
        phase_23_production(&mut starved, production_gate(30, 30_000));
        assert_eq!(starved.resources.tools, 0.0);
        assert_eq!(starved.resources.planks, 10.0);
    }

    #[test]
    fn staffed_wood_cutter_accumulates_planks_over_many_ticks() {
        let mut colony = chain_colony(
            BuildingType::WoodCutter,
            Resources {
                materials: 100.0,
                ..Resources::default()
            },
            true,
        );
        colony.buildings[0].production_progress = 0.0;
        // Ten 600s ticks = ten cycles = 50 materials → 10 planks.
        for step in 1..=10 {
            phase_23_production(&mut colony, production_gate(600, i64::from(step) * 600_000));
        }
        assert_eq!(colony.resources.planks, 10.0);
        assert_eq!(colony.resources.materials, 50.0);
    }

    #[test]
    fn breaking_ground_on_a_scaffold_consumes_planks_and_blocks() {
        // P19 slice 1b build cost: committing a scaffold site draws SCAFFOLD_PLANK_COST
        // planks + SCAFFOLD_BLOCK_COST blocks from the stores and places one new scaffold.
        let mut colony = found_colony(4242, "colony-1", 10_000, 4242);
        colony.resources.planks = 10.0;
        colony.resources.blocks = 10.0;
        let cat_id = colony.cats[0].id.clone();
        let scaffolds_before = colony
            .buildings
            .iter()
            .filter(|building| !building.is_complete)
            .count();
        queue_job(
            &mut colony,
            10_000,
            JobKind::BuildHouse,
            Some(cat_id),
            JobMetadata::Construction {
                phase: ConstructionPhase::ConstructHouse,
                building_type: BuildingType::Den,
                building_id: None,
                site: None,
            },
        );

        phase_14_promote_queued_jobs_and_break_ground(
            &mut colony,
            production_gate(60, 70_000),
            4242,
        );

        assert_eq!(colony.resources.planks, 8.0);
        assert_eq!(colony.resources.blocks, 8.0);
        let scaffolds_after = colony
            .buildings
            .iter()
            .filter(|building| !building.is_complete)
            .count();
        assert_eq!(
            scaffolds_after,
            scaffolds_before + 1,
            "exactly one scaffold should have broken ground"
        );
    }

    #[test]
    fn breaking_ground_defers_when_build_materials_are_short() {
        // With planks below the scaffold cost, the build job stays Queued to retry once
        // the benches have banked enough — nothing is spent and no scaffold appears.
        let mut colony = found_colony(4242, "colony-1", 10_000, 4242);
        colony.resources.planks = 1.0;
        colony.resources.blocks = 10.0;
        let cat_id = colony.cats[0].id.clone();
        let buildings_before = colony.buildings.len();
        queue_job(
            &mut colony,
            10_000,
            JobKind::BuildHouse,
            Some(cat_id),
            JobMetadata::Construction {
                phase: ConstructionPhase::ConstructHouse,
                building_type: BuildingType::Den,
                building_id: None,
                site: None,
            },
        );
        let job_id = colony.jobs.last().expect("queued build job").id.clone();

        phase_14_promote_queued_jobs_and_break_ground(
            &mut colony,
            production_gate(60, 70_000),
            4242,
        );

        assert_eq!(colony.resources.planks, 1.0);
        assert_eq!(colony.resources.blocks, 10.0);
        assert_eq!(colony.buildings.len(), buildings_before);
        let job = colony
            .jobs
            .iter()
            .find(|job| job.id == job_id)
            .expect("build job retained");
        assert_eq!(job.status, JobStatus::Queued);
    }

    #[test]
    fn production_routing_leaves_the_resource_aggregate_identical() {
        // A designated pile reroutes where the output *piles*, but the authoritative
        // `resources` aggregate is byte-identical to the shrine-only case.
        let mut with_pile = workshop_colony(true);
        let mut no_pile = workshop_colony(false);

        phase_23_production(&mut with_pile, production_gate(30, 30_000));
        phase_23_production(&mut no_pile, production_gate(30, 30_000));

        for &kind in ResourceKind::ALL {
            assert_eq!(
                stockpiles::resource_amount(&with_pile.resources, kind).to_bits(),
                stockpiles::resource_amount(&no_pile.resources, kind).to_bits(),
                "{kind:?} resources diverged between piled and shrine-only"
            );
        }
        // With no designated pile the output funnels to the shrine, exactly as pre-P12.4a.
        let shrine = no_pile
            .stockpiles
            .iter()
            .find(|pile| pile.is_shrine())
            .expect("shrine present");
        assert_eq!(shrine.contents.refined, 1.0);
    }

    #[test]
    fn no_designated_piles_keeps_all_production_stock_in_the_shrine() {
        // #1 regression: with production active and no player piles, every resource sits in
        // the shrine reservoir bit-for-bit — the routing change is a no-op on the economy.
        let mut colony = workshop_colony(false);
        let start_refined = colony.resources.refined;

        for step in 1..=10 {
            // 600s per iteration completes exactly one cycle while materials last.
            phase_23_production(&mut colony, production_gate(600, i64::from(step) * 600_000));
            // Mirror the end-of-tick reconcile that folds net change into the reservoir.
            reconcile_colony_stockpiles(&mut colony);

            for &kind in ResourceKind::ALL {
                let player_sum: f64 = colony
                    .stockpiles
                    .iter()
                    .filter(|pile| !pile.is_shrine())
                    .map(|pile| stockpiles::resource_amount(&pile.contents, kind))
                    .sum();
                assert_eq!(
                    player_sum, 0.0,
                    "no player piles hold {kind:?} at step {step}"
                );
                let shrine = colony
                    .stockpiles
                    .iter()
                    .find(|pile| pile.is_shrine())
                    .expect("shrine present");
                assert_eq!(
                    stockpiles::resource_amount(&shrine.contents, kind).to_bits(),
                    stockpiles::resource_amount(&colony.resources, kind).to_bits(),
                    "{kind:?} shrine != resources at step {step}"
                );
            }
        }
        assert!(
            colony.resources.refined > start_refined,
            "production was active over the run"
        );
    }

    fn accounting_colony(reported_food: f64, staffed: bool, last_counted: i64) -> ColonyRuntime {
        // Staffed: a completed tent worked by a living cat. Unstaffed: no bookkeeping tent at
        // all (a built-but-idle tent would just be auto-staffed by an idle cat each tick).
        let (buildings, cats) = if staffed {
            (
                vec![BuildingRuntime {
                    id: "tent-1".to_owned(),
                    building_type: BuildingType::AccountingTent,
                    level: 1,
                    position: TilePos { x: 6, y: 6 },
                    is_complete: true,
                    construction_progress: 100,
                    production_progress: 0.0,
                    assigned_cat: Some("book".to_owned()),
                }],
                vec![adult_idle_cat("book", "colony-1")],
            )
        } else {
            (Vec::new(), vec![adult_idle_cat("idle", "colony-1")])
        };
        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            resources: Resources {
                food: 200.0,
                water: 42.0,
                ..Resources::default()
            },
            cats,
            buildings,
            stock_ledger: StockLedger {
                reported: Resources {
                    food: reported_food,
                    ..Resources::default()
                },
                last_counted,
            },
            last_tick: 0,
            test_rng_seed: Some(1),
            ..ColonyRuntime::default()
        };
        reconcile_colony_stockpiles(&mut colony);
        colony
    }

    #[test]
    fn staffed_accounting_tent_recounts_the_ledger_to_exact_stock_each_tick() {
        let mut colony = accounting_colony(5.0, true, 1_000);
        let truth = colony.resources.clone();

        phase_23_production(&mut colony, production_gate(1, 5_000));

        assert_eq!(
            colony.stock_ledger.reported, truth,
            "staffed tent recounts to exact stock"
        );
        assert_eq!(colony.stock_ledger.last_counted, 5_000);
        assert!(colony.stock_ledger.is_accurate(&colony.resources));
        assert_eq!(
            colony.resources, truth,
            "ledger refresh never mutates resources"
        );
    }

    #[test]
    fn unstaffed_ledger_lags_within_interval_then_recounts() {
        let mut colony = accounting_colony(50.0, false, 1_000);

        // Within the recount interval: reported stays stale, resources untouched.
        phase_23_production(&mut colony, production_gate(1, 1_000 + 5_000));
        assert_eq!(colony.stock_ledger.reported.food, 50.0, "still lagging");
        assert_eq!(colony.stock_ledger.last_counted, 1_000);
        assert_eq!(colony.resources.food, 200.0, "resources untouched");

        // Past the interval: recount to the exact current stock.
        phase_23_production(
            &mut colony,
            production_gate(1, 1_000 + crate::ledger::UNSTAFFED_RECOUNT_INTERVAL_MS),
        );
        assert_eq!(colony.stock_ledger.reported.food, 200.0, "recounted");
        assert_eq!(
            colony.stock_ledger.last_counted,
            1_000 + crate::ledger::UNSTAFFED_RECOUNT_INTERVAL_MS
        );
    }

    #[test]
    fn found_colony_starts_with_an_exact_ledger() {
        let colony = found_colony(42, "colony-1", 7_000, 99);
        assert!(colony.stock_ledger.is_accurate(&colony.resources));
        assert_eq!(colony.stock_ledger.last_counted, 7_000);
    }

    #[test]
    fn found_colony_reveals_the_village_but_not_the_wilds() {
        let colony = found_colony(42, "colony-1", 7_000, 99);

        // The reveal set is populated at founding, independent of world_tiles.
        assert!(
            !colony.revealed_tiles.is_empty(),
            "a freshly founded colony must have a non-empty revealed set"
        );

        // The village anchor and every claimed village tile start revealed.
        let anchor = TilePos {
            x: VILLAGE_ANCHOR.x,
            y: VILLAGE_ANCHOR.y,
        };
        assert!(colony.revealed_tiles.contains(&anchor));
        for tile in &colony.claimed_tiles {
            assert!(
                colony.revealed_tiles.contains(tile),
                "claimed village tile {tile:?} must start revealed"
            );
        }

        // A far tile (well outside the village) is fogged.
        let far = TilePos {
            x: VILLAGE_ANCHOR.x + 40,
            y: VILLAGE_ANCHOR.y + 40,
        };
        assert!(!colony.revealed_tiles.contains(&far));
    }

    #[test]
    fn cats_walking_reveals_more_tiles_over_time() {
        let mut world = new_world(1234);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 10_000, 1234));

        let founding_revealed = world.colonies[0].revealed_tiles.len();

        for step in 1..=60 {
            let now = 10_000 + i64::from(step) * 60_000;
            let _ = world_tick(&mut world, now);
        }

        let after_revealed = world.colonies[0].revealed_tiles.len();
        assert!(
            after_revealed > founding_revealed,
            "cats walking should uncover more tiles ({after_revealed} vs {founding_revealed})"
        );
    }

    #[test]
    fn scouts_random_walk_outward_and_reveal_new_fog_deterministically() {
        // Drive two identical founded colonies. Confirm (a) the leader dispatches
        // scouts (explore jobs fire), (b) their outward random walk reveals fog well
        // beyond the founding ring, and (c) the whole thing is bit-for-bit
        // deterministic across the twin runs.
        let run = || {
            let mut world = new_world(4242);
            world
                .colonies
                .push(found_colony(world.world_seed, "colony-1", 10_000, 4242));
            let mut saw_explore = false;
            let mut max_reveal_cheb = 0;
            for step in 1..=200 {
                let now = 10_000 + i64::from(step) * 60_000;
                let _ = world_tick(&mut world, now);
                let colony = &world.colonies[0];
                if colony.jobs.iter().any(|job| job.kind == JobKind::Explore) {
                    saw_explore = true;
                }
                for tile in &colony.revealed_tiles {
                    max_reveal_cheb = max_reveal_cheb.max(cheb_from_anchor(*tile));
                }
            }
            let revealed = world.colonies[0]
                .revealed_tiles
                .iter()
                .copied()
                .collect::<Vec<_>>();
            (saw_explore, max_reveal_cheb, revealed)
        };
        let (saw_explore, max_cheb, left) = run();
        let (_, _, right) = run();

        assert!(saw_explore, "the leader should dispatch at least one scout");
        assert!(
            max_cheb > 8,
            "outward scouting should reveal fog past the founding ring, reached cheb {max_cheb}"
        );
        assert_eq!(
            left, right,
            "scouting + reveal must be deterministic across identical runs"
        );
    }

    #[test]
    fn revealed_tiles_are_deterministic_across_identical_runs() {
        let mut left = new_world(555);
        left.colonies
            .push(found_colony(left.world_seed, "colony-1", 1_000, 42));
        let mut right = new_world(555);
        right
            .colonies
            .push(found_colony(right.world_seed, "colony-1", 1_000, 42));

        for step in 1..=30 {
            let now = 1_000 + i64::from(step) * 60_000;
            let _ = world_tick(&mut left, now);
            let _ = world_tick(&mut right, now);
        }

        let revealed = |world: &WorldState| -> Vec<TilePos> {
            world.colonies[0].revealed_tiles.iter().copied().collect()
        };
        assert_eq!(revealed(&left), revealed(&right));
    }

    #[test]
    fn stock_ledger_is_deterministic_across_identical_runs() {
        let mut left = new_world(555);
        left.colonies
            .push(found_colony(left.world_seed, "colony-1", 1_000, 42));
        let mut right = new_world(555);
        right
            .colonies
            .push(found_colony(right.world_seed, "colony-1", 1_000, 42));

        for step in 1..=20 {
            let now = 1_000 + i64::from(step) * 60_000;
            let _ = world_tick(&mut left, now);
            let _ = world_tick(&mut right, now);
        }

        assert_eq!(
            left.colonies[0].stock_ledger,
            right.colonies[0].stock_ledger
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
    fn founded_colony_thrives_over_a_long_horizon_with_the_small_start() {
        // Survival guardrail (P16): the fixed 5-cat small start, seeded with the
        // pre-filled stockpile (food 50 / materials 60 / water 100) and its nearby
        // water source, must self-sustain over a long horizon — cats work, food and
        // water hold, and the run never collapses. Runs long enough (800 ticks) that
        // the founding food would deplete without a working hunt/fetch economy.
        for seed in [1234u32, 42, 7, 99, 555] {
            let mut world = new_world(seed);
            world
                .colonies
                .push(found_colony(world.world_seed, "colony-1", 10_000, seed));

            let mut min_population = usize::MAX;
            for step in 1..=500 {
                let now = 10_000 + i64::from(step) * 60_000;
                let reports = world_tick(&mut world, now);
                assert_eq!(
                    reports[0].reset_reason, None,
                    "seed {seed} tick {step} reset the run"
                );
                min_population = min_population.min(alive_cats(&world.colonies[0].cats).count());
            }

            let colony = &world.colonies[0];
            assert_ne!(colony.status, ColonyStatus::Dead, "seed {seed} colony died");
            assert!(
                min_population >= STARTER_CAT_COUNT,
                "seed {seed} dipped to {min_population} cats — the small start starved"
            );
            assert!(
                colony.resources.food > 0.0 && colony.resources.water > 0.0,
                "seed {seed} ran a resource dry (food {:.1}, water {:.1})",
                colony.resources.food,
                colony.resources.water,
            );
        }
    }

    // ---- Phase 25: survival needs, dehydration/starvation death, carried-yield salvage ----

    /// A minimal single-cat colony for phase-25 unit tests — no buildings/stockpiles
    /// needed since `credit_carrying` degrades gracefully to crediting `resources`
    /// directly when no designated pile exists (mirrors the no-pile hauling tests).
    fn survival_colony(cat: Cat, food: f64, water: f64) -> ColonyRuntime {
        ColonyRuntime {
            id: "colony-1".to_owned(),
            resources: Resources {
                food,
                water,
                ..Resources::default()
            },
            cats: vec![cat],
            ..ColonyRuntime::default()
        }
    }

    fn survival_cat(needs: CatNeeds) -> Cat {
        Cat {
            needs,
            ..adult_idle_cat("cat-1", "colony-1")
        }
    }

    fn normal_policy() -> TickPolicy {
        TickPolicy {
            config: crate::policy::config_for_tier(crate::types::PolicyTier::Normal),
        }
    }

    #[test]
    fn water_depletion_crisis_starts_before_death() {
        // Water fully depleted, food plentiful: thirst decays at the full (water-
        // unavailable) rate and crosses zero this tick, but the single tick's damage
        // isn't enough to kill — the cat is in crisis, not dead.
        let cat = survival_cat(CatNeeds {
            hunger: 100.0,
            thirst: 1.0,
            rest: 100.0,
            health: 100.0,
        });
        let mut colony = survival_colony(cat, 100.0, 0.0);

        phase_25_survival_deaths_and_carried_yield_salvage(
            &mut colony,
            production_gate(600, 600_000),
            normal_policy(),
        );

        assert_eq!(colony.cats[0].needs.thirst, 0.0);
        assert!(colony.cats[0].needs.health > 0.0);
        assert_eq!(colony.cats[0].death_time, None);
        assert!(
            colony.events.iter().any(|event| {
                event.kind == EventKind::ResourceCrisis
                    && event.message == "Poppy started dehydrating."
            }),
            "expected a dehydration crisis event, got {:?}",
            colony.events
        );
    }

    #[test]
    fn dehydration_death_past_the_threshold_logs_a_death_event() {
        // Already dehydrating (thirst pinned at 0, water still out) with low health —
        // this tick's damage finishes the job.
        let cat = survival_cat(CatNeeds {
            hunger: 100.0,
            thirst: 0.0,
            rest: 100.0,
            health: 20.0,
        });
        let mut colony = survival_colony(cat, 100.0, 0.0);

        phase_25_survival_deaths_and_carried_yield_salvage(
            &mut colony,
            production_gate(4_200, 4_200_000),
            normal_policy(),
        );

        assert_eq!(colony.cats[0].needs.health, 0.0);
        assert_eq!(colony.cats[0].death_time, Some(4_200_000));
        assert_eq!(colony.cats[0].activity, CatActivity::Idle);
        assert_eq!(colony.cats[0].destination, None);
        assert_eq!(colony.cats[0].carrying, None);
        assert!(
            colony.events.iter().any(
                |event| matches!(&event.kind, EventKind::Other(kind) if kind == "death")
                    && event.message == "Poppy died from dehydration."
            ),
            "expected a dehydration death event, got {:?}",
            colony.events
        );
    }

    #[test]
    fn water_restoration_recovers_before_the_death_threshold() {
        // Thirst is pinned at 0 (already dehydrating) but water is restored this tick —
        // the counter should reset upward and the cat survives.
        let cat = survival_cat(CatNeeds {
            hunger: 100.0,
            thirst: 0.0,
            rest: 100.0,
            health: 50.0,
        });
        let mut colony = survival_colony(cat, 100.0, 10.0);

        phase_25_survival_deaths_and_carried_yield_salvage(
            &mut colony,
            production_gate(600, 600_000),
            normal_policy(),
        );

        assert!(colony.cats[0].needs.thirst > 0.0);
        assert_eq!(colony.cats[0].needs.health, 50.0);
        assert_eq!(colony.cats[0].death_time, None);
        assert!(
            colony.events.iter().any(|event| {
                event.kind == EventKind::ResourceRecovered
                    && event.message == "Poppy recovered from dehydration."
            }),
            "expected a dehydration recovery event, got {:?}",
            colony.events
        );
    }

    #[test]
    fn starvation_death_past_the_threshold_logs_a_starvation_event() {
        // Food fully depleted (water fine), hunger already pinned at 0 with low
        // health — this tick's damage kills, and the cause reads "starvation", not
        // "dehydration" (mirrors `server/game.ts`'s cause-string branching).
        let cat = survival_cat(CatNeeds {
            hunger: 0.0,
            thirst: 100.0,
            rest: 100.0,
            health: 10.0,
        });
        let mut colony = survival_colony(cat, 0.0, 100.0);

        phase_25_survival_deaths_and_carried_yield_salvage(
            &mut colony,
            production_gate(1_200, 1_200_000),
            normal_policy(),
        );

        assert_eq!(colony.cats[0].needs.health, 0.0);
        assert_eq!(colony.cats[0].death_time, Some(1_200_000));
        assert!(
            colony.events.iter().any(
                |event| matches!(&event.kind, EventKind::Other(kind) if kind == "death")
                    && event.message == "Poppy died from starvation."
            ),
            "expected a starvation death event, got {:?}",
            colony.events
        );
    }

    #[test]
    fn dying_carriers_yield_is_salvaged_into_the_store_and_carrying_is_cleared() {
        let mut cat = survival_cat(CatNeeds {
            hunger: 100.0,
            thirst: 0.0,
            rest: 100.0,
            health: 5.0,
        });
        cat.carrying = Some(Carrying {
            kind: CarryingKind::Materials,
            amount: 12.0,
            job_ended_at: 0,
        });
        let mut colony = survival_colony(cat, 100.0, 0.0);
        colony.resources.materials = 5.0;

        phase_25_survival_deaths_and_carried_yield_salvage(
            &mut colony,
            production_gate(1_200, 1_200_000),
            normal_policy(),
        );

        assert_eq!(colony.cats[0].death_time, Some(1_200_000));
        assert_eq!(colony.cats[0].carrying, None);
        assert_eq!(colony.resources.materials, 17.0);
    }

    #[test]
    fn dying_cats_active_and_queued_jobs_are_cancelled() {
        let cat = survival_cat(CatNeeds {
            hunger: 100.0,
            thirst: 0.0,
            rest: 100.0,
            health: 5.0,
        });
        let mut colony = survival_colony(cat, 100.0, 0.0);
        colony.jobs = vec![
            JobRuntime {
                id: "job-active".to_owned(),
                kind: JobKind::HuntExpedition,
                status: JobStatus::Active,
                requested_by: JobRequester::Leader,
                assigned_cat: Some("cat-1".to_owned()),
                duration_ms: 1_000,
                speed: 1.0,
                yield_amount: 0.0,
                click_count: 0,
                created_at: 0,
                started_at: Some(0),
                ends_at: Some(999_999_999),
                completed_at: None,
                metadata: JobMetadata::None,
            },
            JobRuntime {
                id: "job-queued".to_owned(),
                kind: JobKind::Quarry,
                status: JobStatus::Queued,
                requested_by: JobRequester::Leader,
                assigned_cat: Some("cat-1".to_owned()),
                duration_ms: 1_000,
                speed: 1.0,
                yield_amount: 0.0,
                click_count: 0,
                created_at: 0,
                started_at: None,
                ends_at: None,
                completed_at: None,
                metadata: JobMetadata::None,
            },
        ];

        phase_25_survival_deaths_and_carried_yield_salvage(
            &mut colony,
            production_gate(1_200, 1_200_000),
            normal_policy(),
        );

        assert_eq!(colony.cats[0].death_time, Some(1_200_000));
        assert!(
            colony
                .jobs
                .iter()
                .all(|job| job.status == JobStatus::Cancelled),
            "expected both jobs cancelled, got {:?}",
            colony.jobs
        );
    }

    #[test]
    fn survival_tick_is_deterministic_across_identical_runs_through_a_depletion() {
        // Two independently-founded worlds on the same seed, driven through the same
        // forced water depletion (a direct, non-RNG mutation applied identically to
        // both), must end up byte-identical — including the eventual dehydration
        // death. Proves phase 25's deterministic-threshold model (no RNG chain to
        // desync) stays byte-identical under a real depletion + death event.
        let mut left = new_world(4242);
        left.colonies
            .push(found_colony(left.world_seed, "colony-1", 10_000, 4242));
        let mut right = new_world(4242);
        right
            .colonies
            .push(found_colony(right.world_seed, "colony-1", 10_000, 4242));

        // Force one cat toward the dehydration-death edge identically on both worlds
        // — a plain field write, not RNG, so it cannot desync the twin.
        let starved_needs = CatNeeds {
            hunger: 100.0,
            thirst: 1.0,
            rest: 100.0,
            health: 5.0,
        };
        left.colonies[0].cats[0].needs = starved_needs.clone();
        right.colonies[0].cats[0].needs = starved_needs;

        for step in 1..=40 {
            let now = 10_000 + i64::from(step) * 60_000;
            // Keep water at zero throughout so the depletion is sustained regardless
            // of any leader fetch-water response, identically on both worlds.
            left.colonies[0].resources.water = 0.0;
            right.colonies[0].resources.water = 0.0;
            assert_eq!(world_tick(&mut left, now), world_tick(&mut right, now));
        }

        assert_eq!(left.colonies[0].cats, right.colonies[0].cats);
        assert!(
            left.colonies[0].cats[0].death_time.is_some(),
            "the forced depletion never actually killed the cat — twin comparison is vacuous"
        );
        assert!(
            left.colonies[0]
                .events
                .iter()
                .any(|event| event.message.contains("died from")),
            "expected a death event on both worlds, got {:?}",
            left.colonies[0].events
        );
    }

    // ---- Phase 6 pass 1: old-age death parity with phase 25 survival deaths ----

    /// A cat old enough that, after phase 6's per-tick age increment, the
    /// `hoursPastThreshold` term alone drives `old_age_death_probability` to its 1.0
    /// clamp for any positive elapsed window — deterministic old-age death regardless
    /// of the RNG roll (reached here via extreme age rather than extreme elapsed
    /// hours, unlike `life_sim::old_age_death_probability_scales_and_clamps`'s
    /// "skip-time cap" case, but the same clamp).
    fn ancient_cat(id: &str, colony_id: &str) -> Cat {
        Cat {
            age_hours: 100_000.0,
            ..adult_idle_cat(id, colony_id)
        }
    }

    fn old_age_colony(cat: Cat, materials: f64) -> ColonyRuntime {
        ColonyRuntime {
            id: "colony-1".to_owned(),
            resources: Resources {
                materials,
                ..Resources::default()
            },
            cats: vec![cat],
            test_rng_seed: Some(777),
            ..ColonyRuntime::default()
        }
    }

    #[test]
    fn old_age_death_salvages_the_dying_cats_carried_yield() {
        let mut cat = ancient_cat("elder", "colony-1");
        cat.carrying = Some(Carrying {
            kind: CarryingKind::Materials,
            amount: 12.0,
            job_ended_at: 0,
        });
        let mut colony = old_age_colony(cat, 5.0);

        phase_6_life_simulation(&mut colony, production_gate(3_600, 3_600_000));

        assert_eq!(colony.cats[0].death_time, Some(3_600_000));
        assert_eq!(colony.cats[0].carrying, None);
        assert_eq!(colony.resources.materials, 17.0);
    }

    #[test]
    fn old_age_death_cancels_the_dying_cats_active_and_queued_jobs() {
        let cat = ancient_cat("elder", "colony-1");
        let mut colony = old_age_colony(cat, 0.0);
        colony.jobs = vec![
            JobRuntime {
                id: "job-active".to_owned(),
                kind: JobKind::HuntExpedition,
                status: JobStatus::Active,
                requested_by: JobRequester::Leader,
                assigned_cat: Some("elder".to_owned()),
                duration_ms: 1_000,
                speed: 1.0,
                yield_amount: 0.0,
                click_count: 0,
                created_at: 0,
                started_at: Some(0),
                ends_at: Some(999_999_999),
                completed_at: None,
                metadata: JobMetadata::None,
            },
            JobRuntime {
                id: "job-queued".to_owned(),
                kind: JobKind::Quarry,
                status: JobStatus::Queued,
                requested_by: JobRequester::Leader,
                assigned_cat: Some("elder".to_owned()),
                duration_ms: 1_000,
                speed: 1.0,
                yield_amount: 0.0,
                click_count: 0,
                created_at: 0,
                started_at: None,
                ends_at: None,
                completed_at: None,
                metadata: JobMetadata::None,
            },
        ];

        phase_6_life_simulation(&mut colony, production_gate(3_600, 3_600_000));

        assert_eq!(colony.cats[0].death_time, Some(3_600_000));
        assert!(
            colony
                .jobs
                .iter()
                .all(|job| job.status == JobStatus::Cancelled),
            "expected both jobs cancelled, got {:?}",
            colony.jobs
        );
    }

    #[test]
    fn old_age_death_logs_a_died_peacefully_event() {
        let cat = ancient_cat("elder", "colony-1");
        let mut colony = old_age_colony(cat, 0.0);

        phase_6_life_simulation(&mut colony, production_gate(3_600, 3_600_000));

        assert_eq!(colony.cats[0].death_time, Some(3_600_000));
        assert!(
            colony.events.iter().any(
                |event| matches!(&event.kind, EventKind::Other(kind) if kind == "death")
                    && event.message == "Poppy died peacefully of old age."
            ),
            "expected an old-age death event, got {:?}",
            colony.events
        );
    }

    #[test]
    fn old_age_death_is_deterministic_across_identical_runs() {
        // Two independently-founded worlds on the same seed, driven through the same
        // forced old age (a direct, non-RNG mutation applied identically to both),
        // must end up byte-identical — including the eventual old-age death, its
        // salvage/job-cancel cleanup, and its event. Proves phase 6 pass 1's deferred
        // post-loop processing (collected during the loop, applied after) stays
        // byte-identical and doesn't perturb the life-chain RNG draw sequence.
        let mut left = new_world(9001);
        left.colonies
            .push(found_colony(left.world_seed, "colony-1", 10_000, 9001));
        let mut right = new_world(9001);
        right
            .colonies
            .push(found_colony(right.world_seed, "colony-1", 10_000, 9001));

        // Force one cat toward a guaranteed old-age death identically on both worlds —
        // a plain field write, not RNG, so it cannot desync the twin.
        left.colonies[0].cats[0].age_hours = 100_000.0;
        right.colonies[0].cats[0].age_hours = 100_000.0;

        for step in 1..=10 {
            let now = 10_000 + i64::from(step) * 60_000;
            assert_eq!(world_tick(&mut left, now), world_tick(&mut right, now));
        }

        assert_eq!(left.colonies[0].cats, right.colonies[0].cats);
        assert_eq!(left.colonies[0].events, right.colonies[0].events);
        assert!(
            left.colonies[0].cats[0].death_time.is_some(),
            "the forced old age never actually killed the cat — twin comparison is vacuous"
        );
        assert!(
            left.colonies[0]
                .events
                .iter()
                .any(|event| event.message.contains("died peacefully of old age")),
            "expected an old-age death event on both worlds, got {:?}",
            left.colonies[0].events
        );
    }

    // ---- Breeding: conception, gestation, birth (life-sim population loop) ----

    /// A minimal hand-built colony for phase-6 breeding tests: a shrine + one den give
    /// housing headroom (capacity 4 + 2 = 6) for `cats`, with `food`/`water` set
    /// directly so the conception gate is exercised precisely.
    fn breeding_colony(cats: Vec<Cat>, food: f64, water: f64) -> ColonyRuntime {
        ColonyRuntime {
            id: "colony-1".to_owned(),
            resources: Resources {
                food,
                water,
                ..Resources::default()
            },
            cats,
            buildings: vec![
                BuildingRuntime {
                    id: "shrine-1".to_owned(),
                    building_type: BuildingType::Shrine,
                    level: 1,
                    position: TilePos { x: 10, y: 10 },
                    is_complete: true,
                    construction_progress: 100,
                    production_progress: 0.0,
                    assigned_cat: None,
                },
                BuildingRuntime {
                    id: "den-1".to_owned(),
                    building_type: BuildingType::Den,
                    level: 1,
                    position: TilePos { x: 14, y: 10 },
                    is_complete: true,
                    construction_progress: 100,
                    production_progress: 0.0,
                    assigned_cat: None,
                },
            ],
            test_rng_seed: Some(777),
            test_time_scale: 20.0,
            ..ColonyRuntime::default()
        }
    }

    #[test]
    fn conception_fires_for_a_healthy_pair_with_housing_headroom() {
        // Housing cap 6 (shrine 4 + den 2), population 2 — plenty of headroom. 20
        // elapsed game-hours pushes conceptionProbability (0.06/hr) to its 1.0 cap for
        // both cats, so this is a deterministic "conception happens" test, not a
        // probabilistic one.
        let mut colony = breeding_colony(
            vec![
                adult_idle_cat("mother", "colony-1"),
                adult_idle_cat("father", "colony-1"),
            ],
            150.0,
            150.0,
        );
        assert!(!colony.cats[0].is_pregnant && !colony.cats[1].is_pregnant);

        phase_6_life_simulation(&mut colony, production_gate(3_600, 3_600_000));

        // 24h start age + 20h elapsed = 44h, still adult (< 48h elder threshold).
        assert!(colony.cats[0].is_pregnant, "mother should have conceived");
        assert!(colony.cats[1].is_pregnant, "father should have conceived");
        assert_eq!(colony.cats[0].pregnancy_due_age_hours, Some(50.0));
        assert_eq!(colony.cats[1].pregnancy_due_age_hours, Some(50.0));
        assert_eq!(colony.cats[0].pregnancy_mate_id.as_deref(), Some("father"));
        assert_eq!(colony.cats[1].pregnancy_mate_id.as_deref(), Some("mother"));
    }

    #[test]
    fn conception_is_gated_by_food_below_the_ratio_and_per_capita_floor() {
        // food=3 fails both the >0.35-of-200-cap ratio gate AND the per-capita
        // fallback (population 2 * 2.5/cat = 5) — colonyCanBreed must be false, so the
        // loop breaks before any roll and nobody conceives even though water and
        // housing are fine.
        let mut colony = breeding_colony(
            vec![
                adult_idle_cat("mother", "colony-1"),
                adult_idle_cat("father", "colony-1"),
            ],
            3.0,
            150.0,
        );

        phase_6_life_simulation(&mut colony, production_gate(3_600, 3_600_000));

        assert!(!colony.cats[0].is_pregnant);
        assert!(!colony.cats[1].is_pregnant);
        assert_eq!(colony.cats[0].pregnancy_due_age_hours, None);
        assert_eq!(colony.cats[1].pregnancy_due_age_hours, None);
    }

    #[test]
    fn conception_is_gated_by_population_at_the_housing_cap() {
        // Only a single den (capacity 2) and no shrine: housing cap == population, so
        // `population < housingCapacity` is false and nobody conceives despite ample
        // food and water.
        let mut colony = breeding_colony(
            vec![
                adult_idle_cat("mother", "colony-1"),
                adult_idle_cat("father", "colony-1"),
            ],
            150.0,
            150.0,
        );
        colony.buildings = vec![BuildingRuntime {
            id: "den-1".to_owned(),
            building_type: BuildingType::Den,
            level: 1,
            position: TilePos { x: 14, y: 10 },
            is_complete: true,
            construction_progress: 100,
            production_progress: 0.0,
            assigned_cat: None,
        }];

        phase_6_life_simulation(&mut colony, production_gate(3_600, 3_600_000));

        assert!(!colony.cats[0].is_pregnant);
        assert!(!colony.cats[1].is_pregnant);
    }

    #[test]
    fn gestation_completes_into_a_kitten_and_clears_the_mothers_pregnancy() {
        let mut mother = adult_idle_cat("mother", "colony-1");
        mother.is_pregnant = true;
        // Due just past the 24h starting age, so a tiny elapsed window both crosses
        // the gestation threshold AND keeps this tick's fresh conception chance for
        // the now-unpregnant mother/father pair negligible (0.06/hr * 0.02h ~= 0.1%) —
        // this test is about birth wiring, not the conception roll re-firing the same
        // tick.
        mother.pregnancy_due_age_hours = Some(24.01);
        mother.pregnancy_mate_id = Some("father".to_owned());
        let mut father = adult_idle_cat("father", "colony-1");
        father.stats.attack = 90.0;

        let mut colony = breeding_colony(vec![mother, father], 150.0, 150.0);
        colony.test_time_scale = 1.0;

        // 72s at the default 1.0 time scale is 0.02 game-hours: 24h start + 0.02h =
        // 24.02h >= the 24.01h due age.
        phase_6_life_simulation(&mut colony, production_gate(72, 72_000));

        assert_eq!(colony.cats.len(), 3, "a kitten should have been born");
        let mother = colony
            .cats
            .iter()
            .find(|cat| cat.id == "mother")
            .expect("mother still present");
        assert!(!mother.is_pregnant);
        assert_eq!(mother.pregnancy_due_age_hours, None);
        assert_eq!(mother.pregnancy_mate_id, None);
        assert_eq!(mother.pregnancy_due_time, None);

        let kitten = colony
            .cats
            .iter()
            .find(|cat| cat.id != "mother" && cat.id != "father")
            .expect("newborn kitten present");
        assert_eq!(
            kitten.parent_ids,
            vec![Some("mother".to_owned()), Some("father".to_owned())]
        );
        assert_eq!(kitten.age_hours, 0.0);
        assert!(!can_work(get_life_stage(kitten.age_hours)));
        assert_eq!(kitten.colony_id, "colony-1");
        // Blended (60/40 toward the stronger parent) + up to +/-8 mutation, clamped to
        // [1, 100] — exercise the wiring without re-deriving inheritStats' own math
        // (covered by life_sim.rs's unit tests).
        assert!(kitten.stats.attack >= 1.0 && kitten.stats.attack <= 100.0);

        let birth_event = colony
            .events
            .iter()
            .find(|event| matches!(&event.kind, EventKind::Other(kind) if kind == "birth"));
        assert!(birth_event.is_some(), "a birth event should be logged");
        assert!(birth_event.unwrap().message.contains(&kitten.name));
    }

    #[test]
    fn population_grows_over_a_long_horizon_without_tripping_a_reset() {
        // Same founding shape and horizon as the survival guardrail above, proving the
        // population loop actually closes: cats born during the run outlive it. Seed
        // 1234 conceives and delivers at least one kitten inside 800 ticks.
        let mut world = new_world(1234);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 10_000, 1234));

        for step in 1..=800 {
            let now = 10_000 + i64::from(step) * 60_000;
            let reports = world_tick(&mut world, now);
            assert_eq!(reports[0].reset_reason, None, "tick {step} reset the run");
        }

        let colony = &world.colonies[0];
        let final_population = alive_cats(&colony.cats).count();
        assert!(
            final_population > STARTER_CAT_COUNT,
            "population never grew past the founding {STARTER_CAT_COUNT} (ended at {final_population})"
        );
        assert!(
            colony
                .events
                .iter()
                .any(|event| matches!(&event.kind, EventKind::Other(kind) if kind == "birth")),
            "at least one birth event should have been logged"
        );
    }

    #[test]
    fn breeding_is_deterministic_for_identical_seeds() {
        // Seed 42 conceives and delivers kittens (genetics + naming rolls exercised)
        // within 500 ticks — two independently-founded worlds on the same seed must
        // end up with byte-identical cat rosters, including newborn ids, stats, and
        // sprite params.
        let mut left = new_world(42);
        left.colonies
            .push(found_colony(left.world_seed, "colony-1", 10_000, 42));
        let mut right = new_world(42);
        right
            .colonies
            .push(found_colony(right.world_seed, "colony-1", 10_000, 42));

        for step in 1..=500 {
            let now = 10_000 + i64::from(step) * 60_000;
            assert_eq!(world_tick(&mut left, now), world_tick(&mut right, now));
        }

        assert_eq!(left.colonies[0].cats, right.colonies[0].cats);
        assert!(
            left.colonies[0].cats.len() > STARTER_CAT_COUNT,
            "seed 42 should have produced at least one kitten by tick 500"
        );
    }

    #[test]
    fn founded_colony_leader_staffs_the_wood_cutter_and_banks_build_materials() {
        // P19 slice 1b: with the leader auto-staffing the P16 raw-material benches from
        // its idle surplus, a real founded colony (not a hand-staffed unit) refines its
        // raw materials into planks AND blocks over a long horizon — the balanced mop-up
        // keeps both build materials flowing so construction can be funded.
        let mut world = new_world(1234);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 10_000, 1234));
        for step in 1..=400 {
            let now = 10_000 + i64::from(step) * 60_000;
            let reports = world_tick(&mut world, now);
            assert_eq!(reports[0].reset_reason, None, "tick {step} reset the run");
        }
        let colony = &world.colonies[0];
        assert!(
            colony.resources.planks > 0.0,
            "leader never banked planks (got {})",
            colony.resources.planks
        );
        assert!(
            colony.resources.blocks > 0.0,
            "leader never banked blocks (got {})",
            colony.resources.blocks
        );
    }

    #[test]
    fn founded_colony_keeps_nearly_every_cat_employed() {
        // Job-saturation tuning: a healthy founded colony should leave at most a couple
        // of work-capable cats idle once the leader has spent its labour.
        let mut world = new_world(1234);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 10_000, 1234));

        // Steady-state idleness: measure every tick and require idle to be small on the
        // overwhelming majority. A rare 1-tick blip is allowed — when a synchronized
        // founding cohort of jobs finishes together the freed cats are re-employed on the
        // very next tick — but "cats standing idle" must be the exception, not the norm.
        let mut idle_over_two = 0usize;
        for step in 1..=60 {
            let now = 10_000 + i64::from(step) * 60_000;
            let reports = world_tick(&mut world, now);
            assert_eq!(reports[0].reset_reason, None);

            let colony = &world.colonies[0];
            let busy_ids = active_or_queued_jobs(colony)
                .iter()
                .filter_map(|job| job.assigned_cat.as_deref())
                .collect::<Vec<_>>();
            let assigned_building_ids = colony
                .buildings
                .iter()
                .filter_map(|building| building.assigned_cat.as_deref())
                .collect::<Vec<_>>();
            let idle = alive_cats(&colony.cats)
                .filter(|cat| {
                    can_work(get_life_stage(cat.age_hours))
                        && can_take_new_job_with_busy(cat, &busy_ids)
                        && !assigned_building_ids.contains(&cat.id.as_str())
                })
                .count();
            if idle > 2 {
                idle_over_two += 1;
            }
        }

        assert!(
            idle_over_two <= 2,
            "healthy founded colony had {idle_over_two} ticks with >2 idle cats (expected <= 2)"
        );
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
            planks: 0.0,
            blocks: 0.0,
            tools: 0.0,
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

    // --- P14.1: building footprints, tile occupancy & collision ---------------

    #[test]
    fn footprint_for_matches_expected_sizes() {
        // P16: shrine + all workshops/storehouse are 3x3.
        for building_type in [
            BuildingType::Shrine,
            BuildingType::Workshop,
            BuildingType::Smithy,
            BuildingType::FoodStorage,
            BuildingType::WoodCutter,
            BuildingType::StonePrep,
            BuildingType::Woodworking,
        ] {
            assert_eq!(footprint_for(building_type), (3, 3));
        }
        // Dwellings and mid buildings take a 2x3 plot.
        for building_type in [
            BuildingType::Den,
            BuildingType::Beds,
            BuildingType::Nursery,
            BuildingType::HerbGarden,
            BuildingType::ElderCorner,
            BuildingType::MouseFarm,
            BuildingType::Field,
            BuildingType::Barracks,
            BuildingType::AccountingTent,
        ] {
            assert_eq!(footprint_for(building_type), (2, 3));
        }
        for building_type in [BuildingType::WaterBowl, BuildingType::Walls] {
            assert_eq!(footprint_for(building_type), (1, 1));
        }
    }

    #[test]
    fn footprint_tiles_covers_the_anchored_rectangle() {
        // The anchor is the NW corner; the footprint is [x, x+w) x [y, y+h).
        let tiles = footprint_tiles(TilePos { x: 6, y: 6 }, 3, 3);
        assert_eq!(tiles.len(), 9);
        assert!(tiles.contains(&TilePos { x: 6, y: 6 }));
        assert!(tiles.contains(&TilePos { x: 8, y: 8 }));
        assert!(!tiles.contains(&TilePos { x: 9, y: 6 }));
        assert!(!tiles.contains(&TilePos { x: 5, y: 6 }));

        let shed = footprint_tiles(TilePos { x: 0, y: 0 }, 2, 3);
        assert_eq!(shed.len(), 6);
        assert!(shed.contains(&TilePos { x: 1, y: 2 }));
        assert!(!shed.contains(&TilePos { x: 2, y: 0 }));

        // Degenerate footprints cover nothing.
        assert!(footprint_tiles(TilePos { x: 0, y: 0 }, 0, 5).is_empty());
    }

    #[test]
    fn tile_is_occupied_flags_buildings_water_trees_and_perimeter() {
        let seed = 42;
        let mut colony = found_colony(seed, "colony-1", 1_000, 42);
        assert_eq!(colony.buildings[0].building_type, BuildingType::Shrine);

        // Building footprint: the shrine's 3x3 at (6,6) covers (7,7).
        assert!(tile_is_occupied(&colony, TilePos { x: 7, y: 7 }, seed));

        // Perimeter: a tile just outside the claimed area but bordering it (the wall).
        assert!(tile_is_occupied(&colony, TilePos { x: 0, y: 7 }, seed));

        // A terrain-generated tree tile inside the founding area is occupied.
        let tree = (3..=9)
            .flat_map(|y| (3..=9).map(move |x| TilePos { x, y }))
            .find(|tile| crate::terrain_gen::tile_has_tree(seed, tile.x, tile.y))
            .expect("founding area has a tree for seed 42");
        assert!(tile_is_occupied(&colony, tree, seed));

        // Open claimed ground (no building, tree, water, or perimeter) is free.
        let open = colony
            .claimed_tiles
            .iter()
            .copied()
            .find(|tile| !tile_is_occupied(&colony, *tile, seed))
            .expect("founding village has open ground");
        assert!(!tile_is_occupied(&colony, open, seed));

        // Putting water on that tile makes it occupied.
        colony
            .world_tiles
            .get_mut(&open)
            .expect("open claimed tile has a world tile")
            .resources
            .water = 5;
        assert!(tile_is_occupied(&colony, open, seed));
    }

    #[test]
    fn next_claimed_building_site_rejects_occupied_footprints_and_is_deterministic() {
        let seed = 42;
        let full = found_colony(seed, "colony-1", 1_000, 42);
        let claimed: HashSet<TilePos> = full.claimed_tiles.iter().copied().collect();

        // Clear everything but the shrine so the founding village has open interior room
        // to exercise the primary (within-fence) placement path.
        let mut colony = full.clone();
        colony
            .buildings
            .retain(|building| building.building_type == BuildingType::Shrine);
        let shrine_tiles: HashSet<TilePos> = building_footprint_tiles(&colony.buildings[0])
            .into_iter()
            .collect();

        // A 2x2 den lands on a fully free, claimed footprint that never overlaps the
        // shrine, a tree, or water.
        let den = next_claimed_building_site(&colony, 0.0, seed, BuildingType::Den)
            .expect("a free 2x2 den site exists inside the fence");
        for tile in footprint_tiles(den, 2, 2) {
            assert!(claimed.contains(&tile), "den footprint {tile:?} is claimed");
            assert!(
                !shrine_tiles.contains(&tile),
                "den avoids the shrine at {tile:?}"
            );
            assert!(!crate::terrain_gen::tile_has_tree(seed, tile.x, tile.y));
            assert!(
                !tile_is_occupied(&colony, tile, seed),
                "den tile {tile:?} is free"
            );
        }

        // A wider 2x3 shed also fits inside the fence on free claimed ground.
        let shed = next_claimed_building_site(&colony, 0.5, seed, BuildingType::Workshop)
            .expect("a free 2x3 workshop site exists inside the fence");
        for tile in footprint_tiles(shed, 2, 3) {
            assert!(
                claimed.contains(&tile),
                "shed footprint {tile:?} is claimed"
            );
            assert!(!tile_is_occupied(&colony, tile, seed));
        }

        // Deterministic: same colony + roll + seed → same site, on both the roomy and the
        // fully-built (fallback) colony.
        assert_eq!(
            den,
            next_claimed_building_site(&colony, 0.0, seed, BuildingType::Den).unwrap()
        );
        assert_eq!(
            next_claimed_building_site(&full, 0.7, seed, BuildingType::Den),
            next_claimed_building_site(&full, 0.7, seed, BuildingType::Den)
        );
    }

    fn typed_tile(x: i32, y: i32, tile_type: TileType) -> WorldTileRuntime {
        WorldTileRuntime {
            tile_type,
            ..tile(x, y, 0, None)
        }
    }

    /// Fill `claimed_tiles` + `world_tiles` for a `w x h` block of one tile type.
    fn typed_block(
        colony: &mut ColonyRuntime,
        origin: TilePos,
        w: i32,
        h: i32,
        tile_type: TileType,
    ) {
        for dy in 0..h {
            for dx in 0..w {
                let p = pos(origin.x + dx, origin.y + dy);
                colony.claimed_tiles.push(p);
                colony
                    .world_tiles
                    .insert(p, typed_tile(p.x, p.y, tile_type));
            }
        }
    }

    #[test]
    fn tile_is_farmable_follows_the_climate_fertility_table() {
        // Grass/meadow/marsh are fertile.
        assert!(tile_is_farmable(Some(&typed_tile(0, 0, TileType::Field))));
        assert!(tile_is_farmable(Some(&typed_tile(0, 0, TileType::Meadow))));
        assert!(tile_is_farmable(Some(&typed_tile(0, 0, TileType::Swamp))));
        // Rock, sand, tundra, forest, and water are barren.
        for tile_type in [
            TileType::Mountains,
            TileType::Desert,
            TileType::Tundra,
            TileType::Forest,
            TileType::River,
            TileType::CaveEntrance,
        ] {
            assert!(
                !tile_is_farmable(Some(&typed_tile(0, 0, tile_type))),
                "{tile_type:?} is not farmable"
            );
        }
        // A meadow flooded by a river overlay is not farmable, and an unrevealed
        // (absent) tile is never farmable.
        let flooded = WorldTileRuntime {
            overlay_feature: Some("river".to_owned()),
            ..typed_tile(0, 0, TileType::Meadow)
        };
        assert!(!tile_is_farmable(Some(&flooded)));
        assert!(!tile_is_farmable(None));
    }

    #[test]
    fn field_places_on_grass_and_is_rejected_on_barren_ground() {
        let seed = 42;

        // Grass colony: a field finds fertile, tree-free ground inside the claim.
        let mut grass = ColonyRuntime {
            id: "grass".to_owned(),
            ..ColonyRuntime::default()
        };
        typed_block(&mut grass, pos(40, 40), 6, 7, TileType::Meadow);
        let field = next_claimed_building_site(&grass, 0.0, seed, BuildingType::Field)
            .expect("a field fits on the grass claim");
        for tile in footprint_tiles(field, 2, 3) {
            assert!(
                tile_is_farmable(grass.world_tiles.get(&tile)),
                "field footprint {tile:?} is fertile"
            );
        }

        // Rock colony: no fertile ground anywhere, so a field is rejected — but a
        // den (no farmable requirement) still places on the rock.
        let mut rock = ColonyRuntime {
            id: "rock".to_owned(),
            ..ColonyRuntime::default()
        };
        typed_block(&mut rock, pos(80, 80), 6, 7, TileType::Mountains);
        assert_eq!(
            next_claimed_building_site(&rock, 0.0, seed, BuildingType::Field),
            None,
            "a field cannot be sown on barren rock"
        );
        assert!(
            next_claimed_building_site(&rock, 0.0, seed, BuildingType::Den).is_some(),
            "a den still fits on the rock claim (farmability is field-only)"
        );
    }

    #[test]
    fn found_colony_places_the_shrine_with_a_three_by_three_footprint() {
        let colony = found_colony(4242, "colony-1", 1_000, 4242);
        let shrine = colony
            .buildings
            .iter()
            .find(|building| building.building_type == BuildingType::Shrine)
            .expect("found_colony places the shrine");
        assert_eq!(
            shrine.position,
            TilePos {
                x: VILLAGE_ANCHOR.x,
                y: VILLAGE_ANCHOR.y
            }
        );
        assert_eq!(footprint_for(shrine.building_type), (3, 3));

        // No starter building overlaps the shrine footprint.
        let shrine_tiles: HashSet<TilePos> = building_footprint_tiles(shrine).into_iter().collect();
        for building in colony.buildings.iter().filter(|b| b.id != shrine.id) {
            for tile in building_footprint_tiles(building) {
                assert!(
                    !shrine_tiles.contains(&tile),
                    "building {} overlaps the shrine at {tile:?}",
                    building.id
                );
            }
        }
    }

    #[test]
    fn founded_colony_has_no_overlapping_building_footprints() {
        let mut world = new_world(4242);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 10_000, 4242));
        for step in 1..=40 {
            let now = 10_000 + i64::from(step) * 60_000;
            let _ = world_tick(&mut world, now);
        }

        let colony = &world.colonies[0];
        let mut seen: HashSet<TilePos> = HashSet::new();
        for building in &colony.buildings {
            for tile in building_footprint_tiles(building) {
                assert!(
                    seen.insert(tile),
                    "building {} shares tile {tile:?} with another",
                    building.id
                );
            }
        }
        assert!(
            colony.buildings.len() >= 2,
            "colony builds beyond the shrine ({} buildings)",
            colony.buildings.len()
        );
    }

    #[test]
    fn founded_colony_building_placements_are_identical_for_same_seed() {
        let run = || {
            let mut world = new_world(4242);
            world
                .colonies
                .push(found_colony(world.world_seed, "colony-1", 10_000, 4242));
            for step in 1..=40 {
                let now = 10_000 + i64::from(step) * 60_000;
                let _ = world_tick(&mut world, now);
            }
            world.colonies[0]
                .buildings
                .iter()
                .map(|building| {
                    (
                        building.id.clone(),
                        building.building_type,
                        building.position,
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn founded_colony_keeps_building_and_surviving_with_footprints() {
        let mut world = new_world(4242);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 10_000, 4242));
        for step in 1..=60 {
            let now = 10_000 + i64::from(step) * 60_000;
            let reports = world_tick(&mut world, now);
            assert_eq!(reports[0].reset_reason, None);
        }
        let colony = &world.colonies[0];
        assert!(alive_cats(&colony.cats).count() > 0);
        assert_ne!(colony.status, ColonyStatus::Dead);
        // The village keeps at least the shrine plus a home.
        assert!(colony.buildings.len() >= 2);
    }

    // --- P16: fixed founding village blueprint --------------------------------

    #[test]
    fn founding_starts_with_five_adult_cats() {
        let colony = found_colony(4242, "colony-1", 1_000, 4242);
        assert_eq!(alive_cats(&colony.cats).count(), 5);
        assert_eq!(STARTER_CAT_COUNT, 5);
        for cat in &colony.cats {
            assert_eq!(
                get_life_stage(cat.age_hours),
                crate::types::LifeStage::Adult,
                "starter cat {} should found as an adult (age {})",
                cat.id,
                cat.age_hours,
            );
        }
    }

    #[test]
    fn founding_pre_fills_the_stockpile_with_food_and_materials() {
        let colony = found_colony(4242, "colony-1", 1_000, 4242);
        assert_eq!(colony.resources.food, 50.0);
        assert_eq!(colony.resources.materials, 60.0);
        // The shrine reservoir invariant (P12.3): stockpile contents sum to resources.
        assert!(colony.stockpiles.iter().any(Stockpile::is_shrine));
        for &kind in ResourceKind::ALL {
            let sum: f64 = colony
                .stockpiles
                .iter()
                .map(|pile| stockpiles::resource_amount(&pile.contents, kind))
                .sum();
            let total = stockpiles::resource_amount(&colony.resources, kind);
            assert!((sum - total).abs() <= 1e-6, "{kind:?} reservoir invariant");
        }
    }

    #[test]
    fn founding_places_the_fixed_blueprint_shrine_dens_and_workshops() {
        let colony = found_colony(4242, "colony-1", 1_000, 4242);

        // Shrine dead-centre at the anchor.
        let shrine = colony
            .buildings
            .iter()
            .find(|b| b.building_type == BuildingType::Shrine)
            .expect("blueprint has a shrine");
        assert_eq!(
            shrine.position,
            TilePos {
                x: VILLAGE_ANCHOR.x,
                y: VILLAGE_ANCHOR.y,
            }
        );

        // Exactly three dens and the three raw-material workshops.
        let count = |bt: BuildingType| {
            colony
                .buildings
                .iter()
                .filter(|b| b.building_type == bt)
                .count()
        };
        assert_eq!(count(BuildingType::Den), 3, "three den houses");
        assert_eq!(count(BuildingType::WoodCutter), 1);
        assert_eq!(count(BuildingType::StonePrep), 1);
        assert_eq!(count(BuildingType::Woodworking), 1);
        assert_eq!(colony.buildings.len(), 7);

        // Every footprint is non-overlapping and sits on claimed ground.
        let claimed: HashSet<TilePos> = colony.claimed_tiles.iter().copied().collect();
        let mut seen: HashSet<TilePos> = HashSet::new();
        for building in &colony.buildings {
            for tile in building_footprint_tiles(building) {
                assert!(
                    claimed.contains(&tile),
                    "{} tile {tile:?} is off claimed ground",
                    building.id
                );
                assert!(
                    seen.insert(tile),
                    "{} overlaps another building at {tile:?}",
                    building.id
                );
            }
        }
    }

    #[test]
    fn founding_gate_is_on_the_south_wall() {
        let colony = found_colony(4242, "colony-1", 1_000, 4242);
        let area = claimed_area(&colony);
        let gate = gate_placement_default(&area).expect("the village has a gate");
        assert_eq!(gate.side, Side::S, "the single gate opens to the south");
    }

    #[test]
    fn founding_paves_stone_roads_from_the_shrine_to_all_four_walls() {
        let colony = found_colony(4242, "colony-1", 1_000, 4242);
        let center = shrine_center_tile();
        let r = VILLAGE_START_RADIUS;

        // Each cardinal wall tile on the shrine's centre row/column is a built road.
        let is_road = |x: i32, y: i32| {
            colony
                .world_tiles
                .get(&TilePos { x, y })
                .and_then(|tile| tile.overlay_feature.as_deref())
                == Some("road_built")
        };
        assert!(
            is_road(center.x, center.y - r),
            "north road reaches the wall"
        );
        assert!(
            is_road(center.x, center.y + r),
            "south road reaches the wall"
        );
        assert!(
            is_road(center.x - r, center.y),
            "west road reaches the wall"
        );
        assert!(
            is_road(center.x + r, center.y),
            "east road reaches the wall"
        );

        // The road cross is continuous from just outside the shrine to each wall.
        for d in 2..=r {
            assert!(is_road(center.x, center.y - d), "north road tile at -{d}");
            assert!(is_road(center.x, center.y + d), "south road tile at +{d}");
            assert!(is_road(center.x - d, center.y), "west road tile at -{d}");
            assert!(is_road(center.x + d, center.y), "east road tile at +{d}");
        }
    }

    #[test]
    fn founding_layout_is_identical_for_the_same_seed() {
        let snapshot = |seed: u32| {
            let colony = found_colony(seed, "colony-1", 1_000, seed);
            let buildings: Vec<_> = colony
                .buildings
                .iter()
                .map(|b| (b.id.clone(), b.building_type, b.position))
                .collect();
            let roads: Vec<TilePos> = colony
                .world_tiles
                .iter()
                .filter(|(_, t)| t.overlay_feature.as_deref() == Some("road_built"))
                .map(|(pos, _)| *pos)
                .collect();
            (
                buildings,
                roads,
                colony.claimed_tiles,
                colony.revealed_tiles,
            )
        };
        assert_eq!(snapshot(4242), snapshot(4242));
    }

    #[test]
    fn founding_never_places_a_building_or_road_on_water() {
        // Across seeds, the water-clearing pass keeps every building and road tile dry,
        // while a reachable water source still exists for the fetch-water economy.
        for seed in [1234u32, 42, 7, 99, 555, 4242] {
            let colony = found_colony(seed, "colony-1", 1_000, seed);

            let mut blocked: HashSet<TilePos> = colony
                .buildings
                .iter()
                .flat_map(building_footprint_tiles)
                .collect();
            blocked.extend(founding_road_tiles());
            for pos in &blocked {
                assert!(
                    !tile_has_water(colony.world_tiles.get(pos)),
                    "seed {seed}: water under a building/road at {pos:?}"
                );
            }

            assert!(
                colony
                    .world_tiles
                    .values()
                    .any(|t| tile_has_water(Some(t)) && cheb_from_anchor(t.pos) <= 6),
                "seed {seed}: the village has no reachable water source"
            );
        }
    }

    // --- P16.x: founding craft-bench staffing (workshop-staffing bug regression) ---

    /// Run a fresh founding colony through `world_tick` with zero player input for 45
    /// simulated minutes (one-minute ticks; `test_time_scale` defaults to 1.0 so this is
    /// 2700 real/production seconds — four-plus full 600s workshop cycles of headroom
    /// past the very first tick, which is when the fix staffs a bench). Returns the
    /// final colony plus whether planks and blocks were ever simultaneously banked
    /// (`> 0.0`) at the end of some tick during the run.
    ///
    /// 45 ticks is a deliberately snug window, not just "long enough": without the fix,
    /// the founding stall isn't perfectly permanent — once a *short* `explore` job
    /// finishes it can incidentally free a cat for phase 23's pre-existing mop-up, and
    /// empirically (seed 4242) that eventually produces both planks and blocks by tick
    /// ~54. A 60-tick window would pass either way and prove nothing; 45 sits above the
    /// fix's own bootstrap (both banked by tick ~32) but below that incidental-healing
    /// floor, so this test actually discriminates "staffed promptly by the fix" from
    /// "eventually self-healed by an unrelated mechanic."
    ///
    /// The end-of-run snapshot alone is also not a reliable enough signal: the
    /// woodworking bench is a "luxury tier" that spends 2 planks + 2 blocks per tool
    /// cycle once both are stocked (see `phase_23_production`'s `BuildingType::
    /// Woodworking` arm), so a founding colony's planks/blocks stock legitimately saws
    /// up and back down to 0 over time. Tracking "ever banked both simultaneously"
    /// proves the craft benches were staffed and produced, without being sensitive to
    /// which exact tick the woodworking bench next drains the stockpile.
    fn run_founding_colony_for_45_minutes(seed: u32) -> (ColonyRuntime, bool) {
        let mut world = new_world(seed);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 10_000, seed));

        let mut banked_planks_and_blocks = false;
        for step in 1..=45 {
            let now = 10_000 + i64::from(step) * 60_000;
            let reports = world_tick(&mut world, now);
            assert_eq!(reports[0].reset_reason, None, "step {step}: colony reset");
            let resources = &world.colonies[0].resources;
            if resources.planks > 0.0 && resources.blocks > 0.0 {
                banked_planks_and_blocks = true;
            }
        }

        (world.colonies.remove(0), banked_planks_and_blocks)
    }

    #[test]
    fn founding_colony_staffs_a_craft_bench_on_the_first_tick() {
        // Sharpest form of the regression test: before the fix, the leader director's
        // idle-employment-floor fill pass claimed every idle cat for Hunt/Scout/Quarry
        // on the very first tick (the wood-cutter/stone-prep/woodworking benches carried
        // no labour-goal demand at all, since `workshops_needing_workers` only ever
        // counted the general, founding-absent Workshop building), so none of the three
        // craft benches ever got a worker on tick 1. With the fix, at least one must.
        let mut world = new_world(4242);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 10_000, 4242));
        let _ = world_tick(&mut world, 70_000);

        let staffed_benches = world.colonies[0]
            .buildings
            .iter()
            .filter(|building| {
                matches!(
                    building.building_type,
                    BuildingType::WoodCutter | BuildingType::StonePrep | BuildingType::Woodworking
                ) && building.assigned_cat.is_some()
            })
            .count();
        assert!(
            staffed_benches >= 1,
            "expected at least one craft bench staffed after the first tick, got {staffed_benches}"
        );
    }

    #[test]
    fn founding_colony_produces_planks_and_blocks_without_player_input() {
        // With zero player input, a fresh 5-cat colony must staff its craft benches and
        // bank both planks and blocks promptly — not just eventually — since `phase_14`
        // scaffolds cost SCAFFOLD_PLANK_COST/SCAFFOLD_BLOCK_COST (2.0 each) that gate
        // colony growth on a den ever breaking ground. See `run_founding_colony_for_45_
        // minutes` for why 45 ticks is the right, deliberately snug window.
        let (_, banked_planks_and_blocks) = run_founding_colony_for_45_minutes(4242);
        assert!(
            banked_planks_and_blocks,
            "expected planks > 0 and blocks > 0 simultaneously at some point in 45 minutes of \
             unaided founding play"
        );
    }

    #[test]
    fn founding_colony_plank_and_block_production_is_deterministic() {
        let run = || {
            let (colony, banked_planks_and_blocks) = run_founding_colony_for_45_minutes(4242);
            (
                colony.resources.planks,
                colony.resources.blocks,
                banked_planks_and_blocks,
            )
        };
        assert_eq!(run(), run());
    }
}
