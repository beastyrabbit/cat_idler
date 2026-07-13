//! Runtime world tick skeleton ported from `server/game.ts:workerTick`.
//!
//! This P7.1 module owns the in-memory runtime shapes and phase ordering. Later
//! P7 cards fill in the no-op phase bodies with the pure module calls.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::{
    biomes::MaxResources,
    climate::Mining,
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
    items::{self, Item},
    leader_ai::{LeaderDecision, LeaderHousing, LeaderResources, LeaderSnapshot},
    leader_director::{
        CatBrief, CatBriefStats, DirectorPlan, LaborGoalKind, MatchOptions,
        OFFERING_MATERIALS_AMOUNT, OFFERING_MATERIALS_RESERVE, direct_colony,
        is_research_comfortable, match_cats_to_slots_with_officers,
    },
    ledger::{StockLedger, refresh_ledger},
    life_sim::{
        ColonyBreedingState, GESTATION_GAME_HOURS, can_work, colony_can_breed,
        conception_probability, get_life_stage, inherit_stats, leadership_after_tenure,
        old_age_death_probability,
    },
    movement::{
        EXPLORE_SPEED_FACTOR, JobDestinationContext, WorldPos, destination_for_job,
        effective_move_speed, pick_wander_target, rail_speed_multiplier, road_surface_multiplier,
        scout_wander_target, soft_obstacle_speed_multiplier, straight_line_distance_tiles,
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
        FIELD_BUILD_MIN_RATIO, FIELD_CATS_PER_FIELD, FIELD_MATERIAL_BUFFER, FIELD_MIN_COUNT,
        FIELD_STOCK_TARGET_RATIO, WoodworkingOptions, WorkshopOptions, advance_woodworking,
        advance_workshop, fibre_forage_yield, field_yield,
    },
    recipes::{
        CLOTH_TRADE_RECIPE, CraftOptions, LEATHER_TRADE_RECIPE, STONE_TRADE_RECIPE,
        WOOD_TRADE_RECIPE, advance_craft, craft_quality_from_skill, next_trade_kind,
    },
    rng::{life_seed, movement_seed, raid_seed, roll_seeded},
    roads::{self, RoadCorridorOptions, RoadTile, select_road_corridor},
    shrine::should_deposit,
    skills::{HAUL_SKILL_GAIN, Labor, SKILL_GAIN_PER_JOB},
    smithy::{MetalForgeOptions, SmithyOptions, advance_metal_forge, advance_smithy},
    spoilage::apply_food_spoilage_after_consumption,
    stockpiles::{self, GatherSpot, MAX_GATHER_SPOTS, ResourceKind, Stockpile},
    storage::{
        StorageBuilding, StorageCapacities, count_storehouses, storage_capacities, storehouse_cap,
    },
    survival::{SurvivalResources, apply_survival_tick},
    terrain_gen::tile_climate_biome,
    threat::{
        ThreatSnapshot, accrue_threat, colony_wealth, plan_raid, resolve_raid, should_spawn_raid,
        threat_band,
    },
    trader,
    trips::{HUNT_TRIP_COUNT, remaining_yield, split_yield, trip_due_at},
    types::{
        BuildingType, CatSpecialization, JobKind, JobStatus, LifeStage, TaskType, TileType,
        UpgradeKey,
    },
    upgrade_tree::{
        MOUNTAINEERING_NODE_ID, RAIL_NODE_ID, SHIPPING_NODE_ID, UpgradeTreeState, accrue_research,
        cat_auto_unlock, create_upgrade_tree_state, get_node, is_owned, points_per_tick_for,
        resolve_effects,
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
    world_gen::{TileResources, generate_world_chunk, get_colony_position, tile_to_chunk},
    zones::{Zone, ZoneKind, ZonePos, ZoneRect, filter_targets_by_zones, pick_target_with_zones},
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
    /// World-tile coordinate of this colony's village anchor (the shrine footprint's NW
    /// corner). Colony 0 (the single founding colony) is always [`VILLAGE_ANCHOR`], so its
    /// every anchor-relative computation is byte-identical to the pre-multi-village code.
    /// A second village founded via `FoundVillage` gets a distinct anchor at a valid site
    /// far from every existing colony, and its whole spatial frame (blueprint, claimed
    /// tiles, roads, reveal, raids, movement conversions) is taken relative to *this*
    /// anchor rather than the global constant.
    pub anchor: TilePos,
    /// Fog-of-war: the set of world tiles the client is allowed to render un-fogged.
    /// Runtime-only and independent of `world_tiles` (which is lazily/​sparsely
    /// populated for the live colony) — the founding village reveal seeds it and cats
    /// walking near a tile add to it (`reveal_and_wear_walked_tiles`). Does not affect
    /// the sim; a `BTreeSet` keeps the snapshot order deterministic.
    pub revealed_tiles: BTreeSet<TilePos>,
    /// Fog-of-war P15: tentative reveal for a scout currently out on an `Explore` job
    /// (and still on its way home) — keyed by scout cat id, holding the tiles that
    /// scout's walk has newly uncovered but not yet delivered. Committed into
    /// `revealed_tiles` when the scout reaches the shrine
    /// (`commit_scout_provisional_tiles`); dropped without committing if the scout
    /// dies before returning (`phase_25b_prune_dead_scout_provisional_tiles`).
    /// Transient/runtime-only like `revealed_tiles` — not persisted, and rebuilding
    /// empty on load simply means an in-flight scout's dim halo restarts (no lost
    /// permanent knowledge, since nothing here is permanent yet).
    pub provisional_tiles: BTreeMap<CatId, BTreeSet<TilePos>>,
    /// Appointed officers (role → cat id). P12.2 additive layer; empty = no effect.
    pub officers: BTreeMap<OfficerRole, String>,
    /// On-map stockpiles (P12.3). Always includes the shrine reservoir after a tick;
    /// their contents sum to `resources` per the balancing-reservoir invariant.
    pub stockpiles: Vec<Stockpile>,
    /// P16 gather spots: bookkeeping (resource kind + TTL) for whichever `stockpiles`
    /// entries are currently gather spots. Empty/inert for every colony that never
    /// designates one — the gatherer/mover split only activates once a spot exists.
    pub gather_spots: Vec<GatherSpot>,
    /// Reported stock ledger (P12.4a). A lagging *view* of `resources`; a staffed Accounting
    /// Tent keeps it exact each tick, otherwise it recounts on an interval. Never affects the
    /// true `resources`.
    pub stock_ledger: StockLedger,
    /// DF-scale item/material economy store (P19 slice 1). Distinct `(kind, material,
    /// quality)` stacks with their held count — additive alongside `resources`, which
    /// stays the fast, untouched source of truth for the core survival goods. Empty for
    /// every colony this slice: nothing produces or consumes items yet (that lands in
    /// slice 2's material-variant workshop recipes). `BTreeMap` keeps snapshot/iteration
    /// order deterministic.
    pub items: BTreeMap<Item, u32>,
    /// Woodworking bench's trade-craft cycle timer (P19 slice 2) — carries fractional
    /// progress toward [`crate::recipes::WOOD_TRADE_RECIPE`]'s cycle, entirely separate
    /// from `BuildingRuntime::production_progress` (which the bench's *functional*
    /// tools recipe still owns unchanged). Colony-level rather than per-building
    /// because exactly one Woodworking bench exists per founded colony.
    pub wood_craft_progress: f64,
    /// StonePrep bench's trade-craft cycle timer (P19 slice 2), mirroring
    /// [`Self::wood_craft_progress`] for [`crate::recipes::STONE_TRADE_RECIPE`].
    pub stone_craft_progress: f64,
    /// Clothier bench's trade-craft cycle timer (P16/P19 clothing chain slice),
    /// mirroring [`Self::wood_craft_progress`] for [`crate::recipes::CLOTH_TRADE_RECIPE`].
    pub clothier_craft_progress: f64,
    /// Tannery bench's trade-craft cycle timer (P16/P19 clothing chain slice),
    /// mirroring [`Self::wood_craft_progress`] for [`crate::recipes::LEATHER_TRADE_RECIPE`].
    pub tannery_craft_progress: f64,
    /// Smithy's additive metal-forge cycle timer (P17/P19 ore→metal chain), mirroring
    /// [`Self::wood_craft_progress`]'s "entirely separate from `production_progress`"
    /// shape for [`crate::smithy::advance_metal_forge`]. Stays at `0.0` forever for any
    /// colony that never builds a smelter (no metal ever exists to spend).
    pub metal_forge_progress: f64,
    /// Coin balance (P19 slice 3): earned by [`crate::trader::TraderState::Trading`]
    /// `SellGoods` and spent on `BuyResource`. Its own currency, deliberately not folded
    /// into `resources.blessings` or `global_upgrade_points` (the spec: "Do NOT overload
    /// blessings/upgrade points — coin is its own thing"). Like `global_upgrade_points`
    /// and `resources.blessings`, a colony reset (`reset_run`) does not zero this — it is
    /// banked player-facing wealth, not part of the survival-resource run state.
    pub coin: f64,
    /// The currently-visiting trader, if any (P19 slice 3) — `None` most of the time; a
    /// colony sees at most one trader at once. See [`crate::trader`] for the lifecycle/
    /// pricing model.
    pub trader: Option<TraderRuntime>,
    /// Tick timestamp (ms) the most recent trader visit ended (`TraderState::Departing`
    /// reaching the off-map spawn point and despawning). `None` before the colony's
    /// first-ever visit, in which case the schedule counts game-hours from
    /// `run_started_at` instead. Drives [`crate::trader::TRADER_VISIT_INTERVAL_GAME_HOURS`]'s
    /// threshold check — see `phase_36b_trader_lifecycle`.
    pub last_trader_departed_at: Option<i64>,
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
    /// A P16 gather-spot mover job: travel to the gather spot (`stockpile_id`), pick up
    /// its contents, and haul them to a village stockpile/shrine. `site` starts `None`
    /// (freshly queued) and is resolved to the gather spot's rect center by phase 15,
    /// exactly like [`Self::Hauling`]'s `site`. `accepted` flips once the assigned cat
    /// physically arrives (the generic accept-on-arrival mechanism shared with
    /// [`Self::Site`]/[`Self::Hauling`]) — the gather-spot pickup phase then completes
    /// the job.
    GatherHaul {
        stockpile_id: String,
        site: Option<TilePos>,
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
        | BuildingType::Woodworking
        | BuildingType::Clothier
        | BuildingType::Tannery
        | BuildingType::Smelter => (3, 3),
        // Dwellings, gardens and the mid buildings take a 2x3 plot (P16).
        BuildingType::Den
        | BuildingType::Beds
        | BuildingType::Nursery
        | BuildingType::HerbGarden
        | BuildingType::ElderCorner
        | BuildingType::MouseFarm
        | BuildingType::Field
        | BuildingType::Barracks
        | BuildingType::ResearchHut
        | BuildingType::School
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

/// Building-footprint tiles that act as P14.2 soft obstacles for pathfinding
/// cost and movement speed: every building EXCEPT the shrine, which the spec
/// calls out as "fully passable (the hub)" — it's the road/haul anchor every
/// cat converges on, so it stays at open-ground cost/speed rather than the
/// building-footprint tier.
fn soft_obstacle_building_tiles(colony: &ColonyRuntime) -> HashSet<TilePos> {
    colony
        .buildings
        .iter()
        .filter(|building| building.building_type != BuildingType::Shrine)
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

/// The cause of a cat's death, carried on [`EventKind::Death`] so the taxonomy
/// distinguishes "old age" from a raid loss or a survival-crisis death instead of
/// collapsing them into one generic "death" bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathCause {
    OldAge,
    Starvation,
    Dehydration,
    StarvationAndDehydration,
    Raid,
}

impl DeathCause {
    fn wire_suffix(self) -> &'static str {
        match self {
            DeathCause::OldAge => "old_age",
            DeathCause::Starvation => "starvation",
            DeathCause::Dehydration => "dehydration",
            DeathCause::StarvationAndDehydration => "starvation_and_dehydration",
            DeathCause::Raid => "raid",
        }
    }
}

/// The phase of a raid an [`EventKind::Raid`] event narrates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidPhase {
    /// A warband was spotted advancing on the village.
    Launched,
    /// Player defense clicks cut every raider down before it reached the gate.
    Repelled,
    /// Combat resolved at the gate in the defenders' favour.
    Won,
    /// Combat resolved at the gate in the raiders' favour (loot, no wipeout).
    Lost,
}

impl RaidPhase {
    fn wire_suffix(self) -> &'static str {
        match self {
            RaidPhase::Launched => "launched",
            RaidPhase::Repelled => "repelled",
            RaidPhase::Won => "won",
            RaidPhase::Lost => "lost",
        }
    }
}

/// The phase of a leadership election an [`EventKind::Election`] event narrates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElectionPhase {
    Opened,
    Resolved,
    VoteKick,
}

impl ElectionPhase {
    fn wire_suffix(self) -> &'static str {
        match self {
            ElectionPhase::Opened => "opened",
            ElectionPhase::Resolved => "resolved",
            ElectionPhase::VoteKick => "vote_kick",
        }
    }
}

/// The direction of a player trade with the visiting trader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeDirection {
    Sell,
    Buy,
}

impl TradeDirection {
    fn wire_suffix(self) -> &'static str {
        match self {
            TradeDirection::Sell => "sell",
            TradeDirection::Buy => "buy",
        }
    }
}

/// The exact category of a colony event, mirrored to the wire as a stable
/// lowercase `snake_case` string via [`EventKind::wire_kind`] (see
/// `EventSnapshot::kind` in `cat-protocol`). This taxonomy replaced a pile of
/// `Other("free-text-string")` call sites — see the module doc on
/// `append_event` for the collision that motivated it (Tithe vs. haul-deposit
/// both stringly-typed as `"shrine_deposit"`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    Birth,
    /// A mating pair conceived a litter ("... are expecting a litter.").
    Conception,
    Death(DeathCause),
    Raid(RaidPhase),
    Trade(TradeDirection),
    /// A `carry_offering` job completed and a cat set out for the shrine with
    /// materials converted to blessings (distinct from [`EventKind::Tithe`],
    /// the leader's direct stores-to-blessings conversion).
    Offering,
    /// The leader director's `Tithe` decision: a direct conversion of banked
    /// food/refined surplus into blessings, with no cat travel involved.
    Tithe,
    /// A ritual's or offering's blessings arrived at the shrine and were
    /// credited. Distinct from `Offering` (which fires when the job
    /// completes) and from the ordinary resource haul-deposits, which are not
    /// narrated at all (see `phase_33_movement_deposits_and_no_destination_wander`).
    BlessingDelivered,
    DehydrationCrisis,
    DehydrationRecovery,
    /// Colony-wide water reserves crossed the low-water threshold.
    WaterCrisis,
    /// Colony-wide water reserves recovered above the safe threshold.
    WaterRecovered,
    Election(ElectionPhase),
    /// An interim leader was appointed because the seat was empty (no election).
    LeaderChange,
    VillageFounded,
    /// A staffed research hut/school auto-unlocked the cheapest affordable
    /// upgrade-tree node from accrued research points.
    ResearchUnlocked,
    /// A player spent blessings to directly purchase an upgrade-tree node.
    NodeOwned,
    /// The leader broke ground on a research hut (the ungated entry to the
    /// cat-research path) because the colony was comfortable and had none.
    ResearchHutCommissioned,
    /// The leader broke ground on a field (the ungated, founding-buildable
    /// food-scaling mechanism) because food was lean and below the population-scaled
    /// field cap.
    FieldCommissioned,
    JobQueued,
    JobCompleted,
    JobCancelled,
    Production,
    RoadBuilt,
    TraderArrived,
    TraderTrading,
    TraderDeparted,
    VillageExpanded,
    WarriorTrained,
    ForestChopped,
    /// A scout completed an `explore` job and revealed fog.
    Discovery,
    WorkerAssigned,
    RitualReady,
    Reset,
    /// Paired with [`EventKind::Reset`]: the machine-readable reset reason
    /// (`all-cats-dead` / `raid-wipeout` / `unattended-collapse`).
    ResetReason,
    /// Escape hatch retained for forward compatibility / test fixtures. Real
    /// call sites should use a typed variant above instead.
    Other(String),
}

impl EventKind {
    /// The stable lowercase `snake_case` wire category serialized onto
    /// `EventSnapshot::kind`. Clients should classify on this instead of
    /// pattern-matching `message` text.
    pub fn wire_kind(&self) -> String {
        match self {
            EventKind::Birth => "birth".to_owned(),
            EventKind::Conception => "conception".to_owned(),
            EventKind::Death(cause) => format!("death_{}", cause.wire_suffix()),
            EventKind::Raid(phase) => format!("raid_{}", phase.wire_suffix()),
            EventKind::Trade(direction) => format!("trade_{}", direction.wire_suffix()),
            EventKind::Offering => "offering".to_owned(),
            EventKind::Tithe => "tithe".to_owned(),
            EventKind::BlessingDelivered => "blessing_delivered".to_owned(),
            EventKind::DehydrationCrisis => "dehydration_crisis".to_owned(),
            EventKind::DehydrationRecovery => "dehydration_recovery".to_owned(),
            EventKind::WaterCrisis => "water_crisis".to_owned(),
            EventKind::WaterRecovered => "water_recovered".to_owned(),
            EventKind::Election(phase) => format!("election_{}", phase.wire_suffix()),
            EventKind::LeaderChange => "leader_change".to_owned(),
            EventKind::VillageFounded => "village_founded".to_owned(),
            EventKind::ResearchUnlocked => "research_unlocked".to_owned(),
            EventKind::NodeOwned => "node_owned".to_owned(),
            EventKind::ResearchHutCommissioned => "research_hut_commissioned".to_owned(),
            EventKind::FieldCommissioned => "field_commissioned".to_owned(),
            EventKind::JobQueued => "job_queued".to_owned(),
            EventKind::JobCompleted => "job_completed".to_owned(),
            EventKind::JobCancelled => "job_cancelled".to_owned(),
            EventKind::Production => "production".to_owned(),
            EventKind::RoadBuilt => "road_built".to_owned(),
            EventKind::TraderArrived => "trader_arrived".to_owned(),
            EventKind::TraderTrading => "trader_trading".to_owned(),
            EventKind::TraderDeparted => "trader_departed".to_owned(),
            EventKind::VillageExpanded => "village_expanded".to_owned(),
            EventKind::WarriorTrained => "warrior_trained".to_owned(),
            EventKind::ForestChopped => "forest_chopped".to_owned(),
            EventKind::Discovery => "discovery".to_owned(),
            EventKind::WorkerAssigned => "worker_assigned".to_owned(),
            EventKind::RitualReady => "ritual_ready".to_owned(),
            EventKind::Reset => "reset".to_owned(),
            EventKind::ResetReason => "reset_reason".to_owned(),
            EventKind::Other(kind) => kind.clone(),
        }
    }

    /// The inverse of [`EventKind::wire_kind`] — reconstructs a typed variant
    /// from its stable wire string. Used by `cat-server`'s SQLite persistence
    /// to round-trip the exact kind (not just the free-text `message`).
    /// Unrecognized strings (including pre-taxonomy legacy values persisted by
    /// an older build) fall back to `Other`, which still round-trips losslessly
    /// even though it is untyped.
    #[must_use]
    pub fn from_wire_kind(raw: &str) -> EventKind {
        match raw {
            "birth" => EventKind::Birth,
            "conception" => EventKind::Conception,
            "death_old_age" => EventKind::Death(DeathCause::OldAge),
            "death_starvation" => EventKind::Death(DeathCause::Starvation),
            "death_dehydration" => EventKind::Death(DeathCause::Dehydration),
            "death_starvation_and_dehydration" => {
                EventKind::Death(DeathCause::StarvationAndDehydration)
            }
            "death_raid" => EventKind::Death(DeathCause::Raid),
            "raid_launched" => EventKind::Raid(RaidPhase::Launched),
            "raid_repelled" => EventKind::Raid(RaidPhase::Repelled),
            "raid_won" => EventKind::Raid(RaidPhase::Won),
            "raid_lost" => EventKind::Raid(RaidPhase::Lost),
            "trade_sell" => EventKind::Trade(TradeDirection::Sell),
            "trade_buy" => EventKind::Trade(TradeDirection::Buy),
            "offering" => EventKind::Offering,
            "tithe" => EventKind::Tithe,
            "blessing_delivered" => EventKind::BlessingDelivered,
            "dehydration_crisis" => EventKind::DehydrationCrisis,
            "dehydration_recovery" => EventKind::DehydrationRecovery,
            "water_crisis" => EventKind::WaterCrisis,
            "water_recovered" => EventKind::WaterRecovered,
            "election_opened" => EventKind::Election(ElectionPhase::Opened),
            "election_resolved" => EventKind::Election(ElectionPhase::Resolved),
            "election_vote_kick" => EventKind::Election(ElectionPhase::VoteKick),
            "leader_change" => EventKind::LeaderChange,
            "village_founded" => EventKind::VillageFounded,
            "research_unlocked" => EventKind::ResearchUnlocked,
            "node_owned" => EventKind::NodeOwned,
            "research_hut_commissioned" => EventKind::ResearchHutCommissioned,
            "field_commissioned" => EventKind::FieldCommissioned,
            "job_queued" => EventKind::JobQueued,
            "job_completed" => EventKind::JobCompleted,
            "job_cancelled" => EventKind::JobCancelled,
            "production" => EventKind::Production,
            "road_built" => EventKind::RoadBuilt,
            "trader_arrived" => EventKind::TraderArrived,
            "trader_trading" => EventKind::TraderTrading,
            "trader_departed" => EventKind::TraderDeparted,
            "village_expanded" => EventKind::VillageExpanded,
            "warrior_trained" => EventKind::WarriorTrained,
            "forest_chopped" => EventKind::ForestChopped,
            "discovery" => EventKind::Discovery,
            "worker_assigned" => EventKind::WorkerAssigned,
            "ritual_ready" => EventKind::RitualReady,
            "reset" => EventKind::Reset,
            "reset_reason" => EventKind::ResetReason,
            other => EventKind::Other(other.to_owned()),
        }
    }
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

/// A visiting trader's runtime state (P19 slice 3). See [`crate::trader`] for the
/// lifecycle/pricing model this drives.
#[derive(Debug, Clone, PartialEq)]
pub struct TraderRuntime {
    pub id: String,
    pub position: Position,
    pub destination: Option<Position>,
    pub state: trader::TraderState,
    /// Tick timestamp (ms) the trader entered [`crate::trader::TraderState::Trading`].
    /// `None` while `Arriving`.
    pub arrived_at: Option<i64>,
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
/// P14.4: bounds the accessibility router's BFS so a pathological map (or a
/// building stranded with no reachable network) can't spin the tick forever.
/// Generous relative to any realistic village span.
const ACCESS_ROAD_MAX_EXPANSIONS: usize = 2_000;
const WATER_TOTAL_YIELD: f64 = 40.0;
const WALK_WEAR: u32 = 8;
const RAIDER_SPEED_TILES_PER_SEC: f64 = 0.4;
const RAID_SPAWN_DISTANCE: f64 = 14.0;
const ENGAGE_RANGE: f64 = 1.5;
const DEFEND_CLICK_DAMAGE: f64 = 6.0;
/// [`VILLAGE_ANCHOR`] (a `village_layout::GridPos`) as a `TilePos` — the world-tile type
/// the colony's spatial state is stored in. This is the canonical anchor colony 0 always
/// uses, so every anchor-relative computation stays byte-identical for the single colony.
const VILLAGE_ANCHOR_TILE: TilePos = TilePos {
    x: VILLAGE_ANCHOR.x,
    y: VILLAGE_ANCHOR.y,
};

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
    /// This colony's village anchor (world tile of the shrine footprint NW corner) —
    /// every colony-local ↔ world conversion in the movement pass is taken relative to it.
    anchor: TilePos,
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
            anchor: VILLAGE_ANCHOR_TILE,
            revealed_tiles: BTreeSet::new(),
            provisional_tiles: BTreeMap::new(),
            officers: BTreeMap::new(),
            stockpiles: Vec::new(),
            gather_spots: Vec::new(),
            stock_ledger: StockLedger::default(),
            items: BTreeMap::new(),
            wood_craft_progress: 0.0,
            stone_craft_progress: 0.0,
            clothier_craft_progress: 0.0,
            tannery_craft_progress: 0.0,
            metal_forge_progress: 0.0,
            coin: 0.0,
            trader: None,
            last_trader_departed_at: None,
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

impl ColonyRuntime {
    /// Adds `count` of `item` to the colony's item store (P19 slice 1). Inert until a
    /// producer wires in during slice 2 — no code calls this yet.
    pub fn add_item(&mut self, item: Item, count: u32) {
        items::add_item(&mut self.items, item, count);
    }

    /// Removes `count` of `item` from the colony's item store. Returns `false` (store
    /// left untouched) if the store holds fewer than `count`.
    pub fn remove_item(&mut self, item: Item, count: u32) -> bool {
        items::remove_item(&mut self.items, item, count)
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
    // The single founding colony (colony 0) always sits on the canonical anchor, so its
    // whole blueprint/roads/reveal/terrain are byte-identical to the pre-multi-village code.
    found_colony_at(world_seed, colony_id, now_ms, seed, VILLAGE_ANCHOR_TILE)
}

/// Found a colony whose village anchor is `anchor` (world tile of the shrine footprint's
/// NW corner). Every spatial helper it calls is anchor-relative, so a second village lands
/// its blueprint, claimed square, road cross, reveal halo and starter terrain plateau at
/// `anchor` rather than the global constant. Cats are stored colony-local (relative to the
/// anchor via [`position_to_world`]), so they need no per-site offset.
pub fn found_colony_at(
    world_seed: u32,
    colony_id: impl Into<ColonyId>,
    now_ms: i64,
    seed: u32,
    anchor: TilePos,
) -> ColonyRuntime {
    let colony_id = colony_id.into();
    let mut colony = ColonyRuntime {
        id: colony_id.clone(),
        name: format!("Colony {colony_id}"),
        status: ColonyStatus::Starting,
        resources: starting_resources(),
        cats: create_starter_cats(&colony_id, now_ms, seed),
        anchor,
        buildings: starter_buildings(anchor, world_seed),
        world_tiles: starter_world_tiles(anchor, world_seed),
        claimed_tiles: founding_claimed_tiles(anchor),
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

/// Minimum Chebyshev separation (world tiles) required between any two colony anchors. It
/// is comfortably larger than a village's claimed square (`2 * VILLAGE_START_RADIUS + 1`
/// wide, centred one tile SE of the anchor) plus its `FOUNDING_REVEAL_RADIUS` fog halo, so
/// two founded settlements never share claimed ground, fences, or their initial reveal.
pub const MIN_VILLAGE_SEPARATION: i32 = 48;

/// The bounded outward extent (in lattice shells of [`MIN_VILLAGE_SEPARATION`]) the site
/// search scans before giving up and taking the guaranteed-separated fallback.
const MAX_FOUNDING_LATTICE_RING: i32 = 16;

/// Deterministically choose a village anchor for a newly founded colony: a buildable grass
/// tile at least [`MIN_VILLAGE_SEPARATION`] tiles (Chebyshev) from every existing colony
/// anchor. Pure and RNG-free — it scans a fixed outward square lattice of candidate sites
/// (each one separation step apart) in a stable ring-then-`(dy, dx)` order and returns the
/// first candidate that is both far enough from every existing village and sits on
/// grassland/lowland ground. If the bounded search finds no grass tile it falls back to a
/// guaranteed-separated offset east of the farthest existing anchor (the founding blueprint
/// synthesises its own flat plateau at whatever anchor it is given, so the fallback is still
/// a valid, playable site — just not hand-picked grass).
#[must_use]
pub fn select_founding_site(world_seed: u32, existing_anchors: &[TilePos]) -> TilePos {
    let base = VILLAGE_ANCHOR_TILE;
    for ring in 1..=MAX_FOUNDING_LATTICE_RING {
        for candidate in lattice_ring_candidates(base, ring) {
            if is_separated_from_all(candidate, existing_anchors)
                && is_grass_founding_site(world_seed, candidate)
            {
                return candidate;
            }
        }
    }
    let max_x = existing_anchors
        .iter()
        .map(|anchor| anchor.x)
        .max()
        .unwrap_or(base.x);
    TilePos {
        x: max_x + MIN_VILLAGE_SEPARATION,
        y: base.y,
    }
}

/// Whether `candidate` is at least [`MIN_VILLAGE_SEPARATION`] tiles from every existing
/// anchor (Chebyshev). Vacuously true for the first colony.
fn is_separated_from_all(candidate: TilePos, existing_anchors: &[TilePos]) -> bool {
    existing_anchors
        .iter()
        .all(|anchor| cheb_from_anchor(*anchor, candidate) >= MIN_VILLAGE_SEPARATION)
}

/// Whether the shrine-centre tile at `anchor` sits on buildable grass (grassland/lowland
/// terrain role). Water is not required here: the founding stamp guarantees a reachable
/// water source at any anchor.
fn is_grass_founding_site(world_seed: u32, anchor: TilePos) -> bool {
    let center = shrine_center_tile(anchor);
    matches!(
        crate::terrain_gen::tile_biome(world_seed, center.x, center.y),
        crate::terrain_gen::BiomeRole::Grassland | crate::terrain_gen::BiomeRole::Lowland
    )
}

/// Candidate anchors on the `ring`-th square lattice shell around `base`, each spaced a whole
/// [`MIN_VILLAGE_SEPARATION`] apart, in a stable `(dy, dx)` scan order.
fn lattice_ring_candidates(base: TilePos, ring: i32) -> Vec<TilePos> {
    let step = MIN_VILLAGE_SEPARATION;
    let mut out = Vec::new();
    for dy in -ring..=ring {
        for dx in -ring..=ring {
            if dx.abs().max(dy.abs()) != ring {
                continue;
            }
            out.push(TilePos {
                x: base.x + dx * step,
                y: base.y + dy * step,
            });
        }
    }
    out
}

/// Lift the fog for the founding village reveal: the whole claimed village ground
/// starts revealed (players can see their own settlement), plus a `FOUNDING_REVEAL_RADIUS`
/// halo around the anchor so the immediately-adjacent water source is visible. Nothing
/// further is revealed at founding — that is the job of the reveal-on-walk pass. The
/// reveal set is independent of `world_tiles`, so it is correct even when the live
/// colony's tile map is sparse.
fn reveal_founding_area(colony: &mut ColonyRuntime) {
    let anchor = colony.anchor;
    let claimed = colony.claimed_tiles.clone();
    colony.revealed_tiles.extend(claimed);
    for dy in -FOUNDING_REVEAL_RADIUS..=FOUNDING_REVEAL_RADIUS {
        for dx in -FOUNDING_REVEAL_RADIUS..=FOUNDING_REVEAL_RADIUS {
            colony.revealed_tiles.insert(TilePos {
                x: anchor.x + dx,
                y: anchor.y + dy,
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
        // One exact 6/6 woodworking boundary is banked so the visible tools chain
        // can complete before construction consumes its 4/4 reserve. Later output
        // still comes from the raw benches.
        planks: 6.0,
        blocks: 6.0,
        tools: 0.0,
        // Clothing chain (P16/P19 slice) — also empty at founding; fibre trickles in
        // passively and hide is a hunt byproduct.
        fibre: 0.0,
        hide: 0.0,
        cloth: 0.0,
        leather: 0.0,
        // Ore/metal chain (P17/P19) — also empty at founding; ore only ever comes from
        // quarrying a mountain, which needs the `mountaineering` unlock first.
        ore: 0.0,
        metal: 0.0,
        blessings: 0.0,
    }
}

fn create_starter_cats(colony_id: &str, now_ms: i64, seed: u32) -> Vec<Cat> {
    create_starter_cats_with_id_prefix(colony_id, colony_id, now_ms, seed)
}

/// [`create_starter_cats`] with an explicit cat-id prefix. The founding call keeps the
/// historical `{colony_id}-cat-N` ids; the post-extinction respawn
/// ([`respawn_starter_roster`]) passes a run-scoped prefix because the founding ids live
/// on in the colony's dead-cat records and ids must stay unique.
fn create_starter_cats_with_id_prefix(
    colony_id: &str,
    id_prefix: &str,
    now_ms: i64,
    seed: u32,
) -> Vec<Cat> {
    let names = starter_names();
    let mut rolls = SeededRollSource::new(seed);

    (0..STARTER_CAT_COUNT)
        .map(|index| {
            let spot = starter_cat_spot(index);
            Cat {
                id: format!("{id_prefix}-cat-{}", index + 1),
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
                boosted: false,
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
const fn shrine_center_tile(anchor: TilePos) -> TilePos {
    TilePos {
        x: anchor.x + 1,
        y: anchor.y + 1,
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

fn starter_buildings(anchor: TilePos, _world_seed: u32) -> Vec<BuildingRuntime> {
    // The blueprint is authored at [`VILLAGE_ANCHOR`]; a colony founded elsewhere shifts
    // every footprint by the same delta so the whole layout rides its own anchor. Colony 0
    // (anchor == VILLAGE_ANCHOR) shifts by zero and is byte-identical.
    let dx = anchor.x - VILLAGE_ANCHOR.x;
    let dy = anchor.y - VILLAGE_ANCHOR.y;
    STARTER_BLUEPRINT
        .into_iter()
        .enumerate()
        .map(|(index, (building_type, x, y, level))| {
            let (x, y) = (x + dx, y + dy);
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
fn founding_road_tiles(anchor: TilePos) -> Vec<TilePos> {
    let center = shrine_center_tile(anchor);
    let (shrine_w, shrine_h) = footprint_for(BuildingType::Shrine);
    let shrine_min_x = anchor.x;
    let shrine_max_x = anchor.x + shrine_w - 1;
    let shrine_min_y = anchor.y;
    let shrine_max_y = anchor.y + shrine_h - 1;
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
    let anchor = colony.anchor;
    let roads = founding_road_tiles(anchor);
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
            && cheb_from_anchor(anchor, tile.pos) <= 6
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
                && (2..=6).contains(&cheb_from_anchor(colony.anchor, *pos))
                && colony
                    .world_tiles
                    .get(pos)
                    .is_some_and(|tile| !tile_has_water(Some(tile)))
        })
        .collect();
    candidates.sort_by_key(|pos| (pos.y, pos.x));
    candidates.into_iter().next()
}

fn founding_claimed_tiles(anchor: TilePos) -> Vec<TilePos> {
    let center = shrine_center_tile(anchor);
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

fn starter_world_tiles(anchor: TilePos, world_seed: u32) -> BTreeMap<TilePos, WorldTileRuntime> {
    // Generate the 3×3 chunk neighbourhood centred on the colony's own anchor chunk, with
    // the anchor as the guaranteed-flat plateau centre. Colony 0's anchor is
    // `get_colony_position()` (== VILLAGE_ANCHOR, chunk (0,0)), so this reproduces the
    // original fixed `-1..=1` chunk sweep byte-for-byte; a relocated village gets the same
    // buildable plateau carved around *its* site.
    debug_assert_eq!(
        (get_colony_position().x, get_colony_position().y),
        (VILLAGE_ANCHOR.x, VILLAGE_ANCHOR.y),
        "starter plateau centre must track the village anchor",
    );
    let center_chunk = tile_to_chunk(anchor.x, anchor.y);
    let mut tiles = BTreeMap::new();

    for chunk_y in (center_chunk.chunk_y - 1)..=(center_chunk.chunk_y + 1) {
        for chunk_x in (center_chunk.chunk_x - 1)..=(center_chunk.chunk_x + 1) {
            for tile in
                generate_world_chunk(chunk_x, chunk_y, i64::from(world_seed), anchor.x, anchor.y)
            {
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
        phase_17b_emergency_water_fetch(colony, gate);
        let snapshot = phase_18_leader_snapshot_assembly(colony, gate);
        let plan = phase_19_leader_cancellations(colony, gate, &snapshot);
        phase_20_leader_labor_assignments_and_staffing(colony, gate, policy, &plan);
        phase_21_leader_capital_decisions_and_tithe(colony, gate, policy, &plan);
        release_research_staff_unless_comfortable(colony, &snapshot);
        manage_research_hut(colony, gate, policy, &snapshot);
        manage_field(colony, gate, policy, &snapshot);
        phase_22_ritual_approval(colony, gate);
        phase_23_production(colony, gate, world_seed);
        phase_23c_fibre_forage(colony, gate);
        phase_24_research(colony, gate);
        phase_25_survival_deaths_and_carried_yield_salvage(colony, gate, policy);
        phase_25b_prune_dead_scout_provisional_tiles(colony, gate);
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
        phase_29_due_completion_gathering_explore_expansion(colony, gate, world_seed);
        phase_30_due_completion_build_ritual_training_return_mark_done(colony, gate);
        phase_31_mid_job_hauling(colony, gate, world_seed);
        let mut movement =
            phase_32_movement_setup_and_village_expansion_queue(colony, gate, policy, world_seed);
        phase_33_movement_deposits_and_no_destination_wander(colony, gate, &mut movement);
        phase_34_movement_travel_job_acceptance_reveal_path_wear(colony, gate, &movement);
        phase_p16_gather_spot_logistics(colony, gate);
        phase_35_deliberate_roads(colony, gate);
        phase_35b_road_accessibility(colony, gate, world_seed);
        if let Some(reset_reason) = phase_36_threat_and_raid_director(colony, gate) {
            reconcile_colony_stockpiles(colony);
            reports.push(TickReport {
                colony_id: colony.id.clone(),
                skipped: false,
                reset_reason: Some(reset_reason),
            });
            continue;
        }
        phase_36b_trader_lifecycle(colony, gate);
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
            let deposit_at = position_to_world(colony.anchor, death.position);
            credit_carrying(colony, &carrying, deposit_at);
        }
        cancel_cat_jobs(colony, &death.id, gate.processed_through);
        append_event(
            colony,
            gate.processed_through,
            EventKind::Death(DeathCause::OldAge),
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
            boosted: false,
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
        append_event(colony, gate.processed_through, EventKind::Birth, message);
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
            cat.death_time.is_none() && !cat.is_pregnant && is_fertile_stage(cat.age_hours)
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
            EventKind::Conception,
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

/// Whether a cat of this age may conceive: the `Young` (6–24h) and `Adult` (24–48h)
/// life stages are fertile; `Kitten`s are too young and `Elder`s (48h+) are past it.
///
/// SUSTAINABILITY FIX (founding boom-bust root cause). Conception was originally gated
/// to `Adult` only. The `Adult` window is 24h wide and maturation from birth to `Adult`
/// is also 24h (kitten 0–6 / young 6–24 / adult 24–48), so those two spans are EQUAL —
/// a generation only enters the fertile stage as the previous one is leaving it, and any
/// jitter opens a "zero fertile cats" gap. During such a gap births halt entirely while
/// old-age mortality (which begins at 48h) keeps culling elders, so an unattended founding
/// colony boom-busts: population peaks ~7–10 then decays to extinction / trips an
/// `UnattendedCollapse` reset (instrumented across seeds 1234/42/7/99/555 over 150
/// game-hours — every death was old age; food and water were never the cause). Admitting
/// the `Young` stage widens the fertile window to 6–48h (42h) while maturation-to-fertile
/// drops to 6h, so overlapping generations always keep at least one fertile cat and the
/// loop closes. Population stays bounded by the existing housing-cap gate in
/// [`colony_can_breed`] (`population < housing_capacity`), so this is a stable plateau, not
/// runaway growth. Founders still start as `Adult` (`starter_age_hours` → 26–42h), so the
/// founding-layout guarantees are unchanged.
fn is_fertile_stage(age_hours: f64) -> bool {
    matches!(
        get_life_stage(age_hours),
        LifeStage::Young | LifeStage::Adult
    )
}

/// A conception-eligible cat snapshotted before phase 6's conception pass mutates
/// anything — used both to iterate candidates in a stable order and as `pick_mate`'s
/// pairing pool (mirrors `adults` in `server/game.ts:runLifeSimulation`).
struct BreedingCandidate {
    cat_index: usize,
    id: CatId,
    stats: CatStats,
    specialization: Option<CatSpecialization>,
}

/// Pick a co-parent for a conceiving cat: another fertile candidate (see
/// [`is_fertile_stage`]), preferring one with
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
    colony.resources.fibre = clamp_resource(colony.resources.fibre, caps.fibre);
    colony.resources.hide = clamp_resource(colony.resources.hide, caps.hide);
    colony.resources.cloth = clamp_resource(colony.resources.cloth, caps.cloth);
    colony.resources.leather = clamp_resource(colony.resources.leather, caps.leather);
}

/// Phase 8: append the water crisis edge event when water crosses the low
/// threshold.
fn phase_8_water_low_crisis_edge(colony: &mut ColonyRuntime, gate: TickGate) {
    let previous_water = f64::from_bits(gate.previous_water);
    if previous_water > 3.0 && colony.resources.water <= 3.0 {
        append_event(
            colony,
            gate.processed_through,
            EventKind::WaterCrisis,
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
                EventKind::Election(ElectionPhase::Resolved),
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
                    EventKind::Election(ElectionPhase::VoteKick),
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
/// Planks and blocks the tools bench must leave available for construction.
const TOOL_BUILD_MATERIAL_RESERVE: f64 = 4.0;

fn phase_14_promote_queued_jobs_and_break_ground(
    colony: &mut ColonyRuntime,
    gate: TickGate,
    world_seed: u32,
) {
    queue_orphaned_scaffold_recovery(colony, gate.processed_through);
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
            && !matches!(
                next_metadata,
                JobMetadata::Construction {
                    building_id: Some(_),
                    site: Some(_),
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

/// Reassign unfinished scaffolds whose builder died or otherwise left the job.
/// Construction progress belongs to the building, so recovery resumes the existing
/// scaffold without charging a second 2/2 break-ground cost or claiming a second site.
fn queue_orphaned_scaffold_recovery(colony: &mut ColonyRuntime, now_ms: i64) {
    let orphaned = colony
        .buildings
        .iter()
        .filter(|building| !building.is_complete)
        .filter(|building| {
            !active_or_queued_jobs(colony).iter().any(|job| {
                job.kind == JobKind::BuildHouse
                    && matches!(
                        &job.metadata,
                        JobMetadata::Construction {
                            building_id: Some(building_id),
                            ..
                        } if building_id == &building.id
                    )
            })
        })
        .map(|building| {
            (
                building.id.clone(),
                building.building_type,
                building.position,
            )
        })
        .collect::<Vec<_>>();

    for (building_id, building_type, site) in orphaned {
        let Some(cat_id) = select_best_cat(colony, Some(CatSpecialization::Architect)) else {
            break;
        };
        queue_job(
            colony,
            now_ms,
            JobKind::BuildHouse,
            Some(cat_id),
            JobMetadata::Construction {
                phase: ConstructionPhase::ConstructHouse,
                building_type,
                building_id: Some(building_id),
                site: Some(site),
            },
        );
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
    // Snapshot the active player zones (phase 10 already dropped expired ones) so hunt
    // targeting can be steered toward gather rects and away from avoid rects.
    let active_zones = colony
        .zones
        .iter()
        .map(|zone| Zone {
            rect: zone.rect,
            kind: zone.kind,
        })
        .collect::<Vec<_>>();

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
        // A P16 mover's outbound leg targets the gather spot it was dispatched for
        // (resolved by id, since a colony can have several) — its rect center, matching
        // how deposit routing measures a pile's position (`Stockpile::center`).
        let gather_spot_site = match &job.metadata {
            JobMetadata::GatherHaul { stockpile_id, .. } => colony
                .stockpiles
                .iter()
                .find(|pile| pile.id == *stockpile_id)
                .map(|pile| {
                    let (cx, cy) = pile.center();
                    WorldPos { x: cx, y: cy }
                }),
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
                .map_or_else(
                    || village_anchor_world(colony.anchor),
                    |cat| position_to_world(colony.anchor, cat.position),
                );
            let dir = roll_seeded(f64::from(movement_seed));
            let len = roll_seeded(f64::from(dir.next_seed));
            movement_seed = len.next_seed;
            Some(scout_wander_target(
                from,
                village_anchor_world(colony.anchor),
                dir.value,
                len.value,
            ))
        } else {
            None
        };
        // Zones steer hunts: a gather rect makes its tiles twice as likely and an avoid
        // rect excludes them (unless nothing else remains), reducing the candidate list to
        // a single zone-weighted pick. With no zones this collapses to the same uniform
        // pick `hunt_destination` made from the full list at the same roll — byte-identical.
        let hunt_pick = (job.kind == JobKind::HuntExpedition)
            .then(|| pick_zoned_food_tile(&food_tiles, &active_zones, roll.value))
            .flatten();
        let hunt_tiles: &[WorldPos] = match &hunt_pick {
            Some(tile) => std::slice::from_ref(tile),
            None => &[],
        };
        let context = JobDestinationContext {
            anchor: village_anchor_world(colony.anchor),
            shrine: village_anchor_world(colony.anchor),
            food_tiles: hunt_tiles,
            roll: roll.value,
            site: construction_site,
            expansion_site,
            quarry_site,
            water_site,
            explore_site,
            gather_spot_site,
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
            JobKind::HaulGatherSpot => match colony.jobs[job_index].metadata.clone() {
                JobMetadata::GatherHaul { stockpile_id, .. } => JobMetadata::GatherHaul {
                    stockpile_id,
                    site: Some(site),
                    accepted: false,
                },
                _ => JobMetadata::Site {
                    site,
                    accepted: false,
                },
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
            // designated pile accepting their yield); a gather-spot mover heads straight
            // for the gather spot itself (`destination`, already resolved above); every
            // other job still forms up at the village anchor. With no designated piles
            // both of the first two resolve to the anchor, so this is byte-identical to
            // pre-haul-fill.
            let dest = if matches!(
                job.kind,
                JobKind::HuntExpedition | JobKind::Quarry | JobKind::FetchWater
            ) && let Some(cat_pos) = colony
                .cats
                .iter()
                .find(|cat| cat.id == cat_id)
                .map(|cat| position_to_world(colony.anchor, cat.position))
            {
                haul_destination(colony, carrying_kind_for_job(job.kind), cat_pos)
            } else if job.kind == JobKind::HaulGatherSpot {
                destination
            } else {
                village_anchor_world(colony.anchor)
            };
            if let Some(cat) = colony.cats.iter_mut().find(|cat| cat.id == cat_id) {
                cat.destination = Some(position_from_world(dest));
                cat.activity = CatActivity::Traveling;
                cat.current_task = task_for_job(job.kind);
            }
            // P15: register this cat as an out scout for the lifetime of the
            // excursion — reveal turns provisional the moment this entry exists
            // (`reveal_and_wear_walked_tiles`) and stays that way through the walk
            // home, since this is the only place an `Explore` job's destination gets
            // (re-)assigned; the entry is cleared on shrine arrival
            // (`commit_scout_provisional_tiles`) or on death
            // (`phase_25b_prune_dead_scout_provisional_tiles`).
            if job.kind == JobKind::Explore {
                colony.provisional_tiles.entry(cat_id).or_default();
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

/// Keep one water carrier moving whenever the colony falls below the absolute
/// crisis edge. Coarse unattended ticks exposed a real scheduling hole: long
/// expeditions can occupy every work-capable cat while water reaches zero. Assigning
/// yet another carrier made the most food-fragile seed collapse instead. The
/// founding blueprint already guarantees a drinkable source inside the village, so
/// this small local draw is the last-resort floor while the normal 45-minute fetch
/// loop remains load-bearing.
fn phase_17b_emergency_water_fetch(colony: &mut ColonyRuntime, gate: TickGate) {
    const WATER_CRISIS_EDGE: f64 = 5.0;
    const EMERGENCY_BUCKET: f64 = 8.0;

    if alive_cats(&colony.cats).next().is_none()
        || colony.resources.water > WATER_CRISIS_EDGE
        || !has_water_site(colony)
    {
        return;
    }

    colony.resources.water += EMERGENCY_BUCKET;
    append_event(
        colony,
        gate.processed_through,
        EventKind::Production,
        "The cats drew an emergency bucket from the village spring.",
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
        research_huts_needing_workers: Some(
            buildings_needing_workers(colony, BuildingType::ResearchHut).len() as u32,
        ),
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
        offering_in_flight: Some(count_jobs(&active_jobs, JobKind::CarryOffering)),
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
                        EventKind::JobCancelled,
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
                        EventKind::JobCancelled,
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
    // `workshop_queue` is popped from the end, so append the deterministic
    // resource-aware priority in reverse.
    for bench_type in raw_material_staffing_priority(&colony.resources)
        .into_iter()
        .rev()
    {
        workshop_queue.extend(buildings_needing_workers(colony, bench_type));
    }
    let mut smithy_queue = buildings_needing_workers(colony, BuildingType::Smithy);
    // Research huts and schools the director wants staffed this tick. Both building
    // types feed the same `research_workforce` faucet (see world_tick.rs), so a
    // completed-but-unstaffed School competes for the same `AssignResearch` slots a
    // ResearchHut would. The `AssignResearch` goal is itself comfort-gated in
    // `leader_director` (vetoed unless food/water are both comfortable), so this queue
    // only ever yields work when the colony can spare the mouth — a starving colony
    // opens no research slots and this stays empty.
    let mut research_queue = buildings_needing_workers(colony, BuildingType::ResearchHut);
    research_queue.extend(buildings_needing_workers(colony, BuildingType::School));

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
            LaborGoalKind::AssignResearch => {
                if let Some(building_id) = research_queue.pop() {
                    staff_building(
                        colony,
                        &building_id,
                        &assignment.cat_id,
                        gate.processed_through,
                        true,
                    );
                }
            }
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

/// Whether the colony already has a research hut standing or a build for one under way,
/// so the commission below only ever puts a single hut in play.
/// Game-time a run must be old before the leader spends anything on research (see the
/// establishment-window comment in [`manage_research_hut`]). Matches the 30-game-hour
/// establishment window `run_founding_population_trajectory` samples after.
const RESEARCH_ESTABLISHMENT_MS: i64 = 30 * 3_600_000;
/// Active shrine economy begins only after the founding survival window. This keeps
/// the opening larder and construction bank focused on establishing the village.
const SHRINE_ESTABLISHMENT_MS: i64 = RESEARCH_ESTABLISHMENT_MS;
/// A tithe is a meaningful colony-scale offering, not a per-minute tax. This
/// cadence keeps the active faucet slow while preventing a replenishing larder
/// from losing hundreds of food over a few days.
const TITHE_COOLDOWN_MS: i64 = 24 * 3_600_000;

/// Research occupancy is comfort-conditional, not just comfort-gated at entry: any tick
/// the colony is below the [`is_research_comfortable`] per-capita bar, every scholar is
/// released back to the labour pool so the leader's survival pass can re-draft them for
/// hunting/water — the same non-sticky discipline the raw-material benches use
/// ([`release_raw_material_workshop_workers`]). When comfort returns,
/// [`manage_research_hut`]'s step-1 mop-up re-staffs from the idle surplus. Without this,
/// a scholar staffed in good times kept studying straight through a later larder
/// collapse (a 1-cat drain that at the harsh 5-game-minute test cadence tipped seed 7's
/// trough into an `UnattendedCollapse` reset ~20 game-hours after research engaged).
fn release_research_staff_unless_comfortable(
    colony: &mut ColonyRuntime,
    snapshot: &LeaderSnapshot,
) {
    if is_research_comfortable(snapshot) {
        return;
    }
    for building in &mut colony.buildings {
        if matches!(
            building.building_type,
            BuildingType::ResearchHut | BuildingType::School
        ) {
            building.assigned_cat = None;
        }
    }
}

fn has_research_hut_or_build_in_flight(colony: &ColonyRuntime) -> bool {
    let has_building = colony
        .buildings
        .iter()
        .any(|building| building.building_type == BuildingType::ResearchHut);
    let building_in_flight = active_or_queued_jobs(colony).iter().any(|job| {
        job.kind == JobKind::BuildHouse && job_building_type(job) == Some(BuildingType::ResearchHut)
    });
    has_building || building_in_flight
}

/// DEADLOCK RESOLUTION + reliable staffing (unattended cat-research path). The upgrade
/// tree's root node `research_hut` is the only prereq-free node, and it *unlocks* the
/// research hut building/job — so an unattended colony could never earn the node it needs
/// to build the hut that earns it. We break the cycle by making the research hut buildable
/// at founding, **ungated by the tree** (it is the entry to research, not a reward of it):
/// once a comfy colony stands one up and staffs it, [`phase_24_research`] accrues points and
/// auto-unlocks the cheapest affordable node — starting with `research_hut` itself — and the
/// tree climbs on its own.
///
/// Everything here is gated on the colony being *comfortable* (food and water both holding
/// the per-capita buffer of [`is_research_comfortable`]), the same bar the `AssignResearch`
/// labour goal carries — so research never draws a cat off survival work during a crisis,
/// and a colony that is not comfortable is untouched (byte-identical). Two comfort-gated
/// steps run each tick:
///
/// 1. **Staff idle huts** from genuinely idle cats. The director's `AssignResearch` labour
///    goal lives in the ranked employment budget, but the ~0.95 idle-employment floor keeps
///    nearly every cat on a long hunt/scout job, so `idle_cats` is almost always zero and the
///    ranked slot rarely fires — exactly why the generic workshop also gets a phase-23
///    idle-mop-up. This gives the research hut the same treatment. It also re-staffs a hut
///    whose scholar has died (research to a node spans several cat lifetimes), so accrual
///    survives generational turnover. A completed School (player/god-built via
///    `PlanBuilding`, once the `school` node is owned) gets the identical idle-mop-up
///    treatment here — staffing plumbing only, not auto-commission: nothing in this
///    function ever *builds* a School.
/// 2. **Commission one hut** (at most one at a time) when none exists and none is in flight,
///    spending an architect + the shared plank/block scaffold cost. Schools are never
///    auto-commissioned this way — they only come into being through a deliberate
///    `PlanBuilding` action once the `school` upgrade node is owned.
///
/// Deterministic: comfort and staffing are flat functions of colony state; only the
/// commission draws the seeded policy-reliability roll (at the same call site the other
/// capital decisions use).
fn manage_research_hut(
    colony: &mut ColonyRuntime,
    gate: TickGate,
    policy: TickPolicy,
    snapshot: &LeaderSnapshot,
) {
    if snapshot.population == 0 {
        return;
    }
    // Establishment window: a freshly-founded (or freshly-reset) run keeps every paw on
    // survival before any is spent on study. The per-capita comfort bar can read true in
    // the very first hours (a 5-cat founding larder of ~44 food clears it), but the early
    // coarse-cadence economy is a lumpy sawtooth where diverting the architect + scaffold
    // planks into a hut tips troughs into `UnattendedCollapse` (caught by
    // `founding_colony_food_trough_is_lifted_without_going_trivial`). Mirrors the
    // 30-game-hour establishment window the survival guardrails sample after.
    if gate.processed_through - colony.run_started_at < RESEARCH_ESTABLISHMENT_MS {
        return;
    }
    let has_shrine = colony
        .buildings
        .iter()
        .any(|building| building.building_type == BuildingType::Shrine && building.is_complete);
    if !has_shrine {
        return;
    }
    if !is_research_comfortable(snapshot) {
        return;
    }

    // Step 1: staff any completed but unstaffed research hut or school from a genuinely
    // idle cat. Only reached while comfortable, so this never pulls a cat off survival
    // work.
    auto_staff_idle_buildings(
        colony,
        BuildingType::ResearchHut,
        gate.processed_through,
        true,
    );
    auto_staff_idle_buildings(colony, BuildingType::School, gate.processed_through, true);

    // Step 2: commission a hut if the colony has none (built or in flight).
    if has_research_hut_or_build_in_flight(colony) {
        return;
    }
    if !can_take_policy_action(colony, policy) {
        return;
    }
    let Some(cat_id) = select_best_cat(colony, Some(CatSpecialization::Architect)) else {
        return;
    };
    queue_job(
        colony,
        gate.processed_through,
        JobKind::BuildHouse,
        Some(cat_id),
        JobMetadata::Construction {
            phase: ConstructionPhase::ConstructHouse,
            building_type: BuildingType::ResearchHut,
            building_id: None,
            site: None,
        },
    );
    append_event(
        colony,
        gate.processed_through,
        EventKind::ResearchHutCommissioned,
        "The leader broke ground on a research hut - the cats will start to study.",
    );
}

/// Count the colony's fields, whether standing or still under construction (built +
/// in-flight `build_house` scaffolds targeting a field), so the leader never over-orders
/// past the population-scaled cap while earlier field jobs are still on the way.
fn field_count_built_or_in_flight(colony: &ColonyRuntime) -> usize {
    let built = colony
        .buildings
        .iter()
        .filter(|building| building.building_type == BuildingType::Field)
        .count();
    let in_flight = active_or_queued_jobs(colony)
        .iter()
        .filter(|job| {
            job.kind == JobKind::BuildHouse && job_building_type(job) == Some(BuildingType::Field)
        })
        .count();
    built + in_flight
}

/// FOOD-SCALING food-comfort fix — commission passive fields so a food-lean colony can
/// reach and sustain food comfort (see [`FIELD_CATS_PER_FIELD`] for the root cause).
///
/// The intended food-scaling mechanic (fields, [`field_yield`]) is gated behind village
/// level 4 — twelve completed non-shrine buildings — which a colony that runs food-lean
/// never reaches, so food stays a lumpy hunt-only sawtooth that rarely holds above the
/// comfort bar. This breaks that deadlock the same way [`manage_research_hut`] broke the
/// research-hut one: the leader auto-commissions fields **ungated by the tree/village
/// level** at founding, placing each on farmable ground (`next_claimed_building_site`
/// already refuses barren tiles for a `Field`).
///
/// Three guards keep fields strictly ADDITIVE — food scaling that never poaches a cat or a
/// plank the survival/construction economy needs:
/// - a build-material surplus ([`FIELD_MATERIAL_BUFFER`] planks AND blocks) so a field never
///   taxes the wood-cutter/stone-prep chain the colony needs to fund its dens — even the
///   essential base waits for the materials to exist;
/// - a small essential base ([`FIELD_MIN_COUNT`]) built regardless of food level (a cat
///   permitting) as the survival floor under the lean early sawtooth, plus population-scaled
///   discretionary fields on top ([`FIELD_CATS_PER_FIELD`]), capped so fields stay a
///   supplement to hunting (food never goes trivial);
/// - a food band for the *discretionary* tier (`[FIELD_BUILD_MIN_RATIO,
///   FIELD_STOCK_TARGET_RATIO)`); the base ignores it (it is what ends the trough) and is
///   self-limited by builder availability — `select_best_cat` yields nobody when every cat
///   is on survival work, so the base can never strand the founding roster without a hunter.
///
/// Deterministic: population, ratios, and the field cap are flat functions of colony state;
/// only the commission draws the seeded policy-reliability roll, at the same call site the
/// other capital decisions use.
fn manage_field(
    colony: &mut ColonyRuntime,
    gate: TickGate,
    policy: TickPolicy,
    snapshot: &LeaderSnapshot,
) {
    let population = snapshot.population;
    if population == 0 {
        return;
    }
    let has_shrine = colony
        .buildings
        .iter()
        .any(|building| building.building_type == BuildingType::Shrine && building.is_complete);
    if !has_shrine {
        return;
    }

    // ADDITIVE-ONLY guard: a field is food *scaling*, never a tax on the critical build-
    // material economy. Break ground only out of a genuine plank/block surplus (above the
    // 2+2 scaffold cost), so the staffed wood-cutter/stone-prep chain keeps its cats and its
    // output funding dens. Below the buffer the material chain is untouched and no field is
    // ordered — even the essential base waits for the materials to exist.
    if colony.resources.planks < FIELD_MATERIAL_BUFFER
        || colony.resources.blocks < FIELD_MATERIAL_BUFFER
    {
        return;
    }

    // Never keep building once the larder is comfortably stocked — the passive base has done
    // its job and standing fields persist.
    let food_r = ratio_or_zero(snapshot.resources.food, snapshot.food_capacity);
    if food_r >= FIELD_STOCK_TARGET_RATIO {
        return;
    }

    // Total cap: an essential base (FIELD_MIN_COUNT) as the survival floor plus
    // population-scaled discretionary fields on top. Scaling the discretionary tier off
    // today's headcount lets the complement ramp with the colony rather than front-loading a
    // whole complement of builders on the fragile founding roster.
    let fields = field_count_built_or_in_flight(colony);
    let discretionary_cap = (population as f64 / FIELD_CATS_PER_FIELD).ceil() as usize;
    let max_fields = FIELD_MIN_COUNT.max(discretionary_cap);
    if fields >= max_fields {
        return;
    }

    // Discretionary fields (beyond the essential base) wait for the moderate food band:
    // below FIELD_BUILD_MIN_RATIO food is in an acute trough and a cat is worth more out
    // hunting than tied up building. The essential base ignores this floor.
    if fields >= FIELD_MIN_COUNT && food_r < FIELD_BUILD_MIN_RATIO {
        return;
    }

    if !can_take_policy_action(colony, policy) {
        return;
    }
    // Draw a genuinely free cat (best-fit architect). None => every cat is busy on
    // survival work, so defer rather than pull one off a hunt.
    let Some(cat_id) = select_best_cat(colony, Some(CatSpecialization::Architect)) else {
        return;
    };
    queue_job(
        colony,
        gate.processed_through,
        JobKind::BuildHouse,
        Some(cat_id),
        JobMetadata::Construction {
            phase: ConstructionPhase::ConstructHouse,
            building_type: BuildingType::Field,
            building_id: None,
            site: None,
        },
    );
    append_event(
        colony,
        gate.processed_through,
        EventKind::FieldCommissioned,
        "The leader broke ground on a field - the cats will grow food to steady the larder.",
    );
}

/// Non-panicking ratio: `value / capacity`, or `0.0` when capacity is non-positive.
fn ratio_or_zero(value: f64, capacity: f64) -> f64 {
    if capacity > 0.0 {
        value / capacity
    } else {
        0.0
    }
}

/// Phase 21: execute leader capital decisions and minute-cadence tithe deposits.
fn phase_21_leader_capital_decisions_and_tithe(
    colony: &mut ColonyRuntime,
    gate: TickGate,
    policy: TickPolicy,
    plan: &DirectorPlan,
) {
    let shrine_established =
        gate.processed_through - colony.run_started_at >= SHRINE_ESTABLISHMENT_MS;
    let tithe_ready = colony
        .events
        .iter()
        .rev()
        .find(|event| event.kind == EventKind::Tithe)
        .is_none_or(|event| gate.processed_through - event.at_ms >= TITHE_COOLDOWN_MS);

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
                if !shrine_established || !tithe_ready || !gate.minute_rolled {
                    continue;
                }
                colony.resources.food -= f64::from(food);
                colony.resources.refined -= f64::from(refined);
                colony.global_upgrade_points += f64::from(blessings);
                append_event(
                    colony,
                    gate.processed_through,
                    EventKind::Tithe,
                    format!(
                        "The leader offered surplus stores to the gods (+{blessings} blessing{}).",
                        if blessings == 1 { "" } else { "s" }
                    ),
                );
            }
            LeaderDecision::Offering => {
                if !shrine_established {
                    continue;
                }
                let ritualist = can_take_policy_action(colony, policy)
                    .then(|| {
                        select_best_cat(colony, Some(CatSpecialization::Ritualist)).or_else(|| {
                            select_best_raw_material_worker(
                                colony,
                                Some(CatSpecialization::Ritualist),
                            )
                        })
                    })
                    .flatten();
                if let Some(cat_id) = ritualist {
                    queue_job(
                        colony,
                        gate.processed_through,
                        JobKind::CarryOffering,
                        Some(cat_id),
                        JobMetadata::None,
                    );
                }
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
fn phase_23_production(colony: &mut ColonyRuntime, gate: TickGate, world_seed: u32) {
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
    // Once both 4-unit scaffold reserves plus one 2/2 tools cycle are funded, give
    // woodworking first claim on an idle cat. Otherwise fund construction first by
    // staffing the scarcer of the two build-material benches
    // (wood-cutter → planks, stone-prep → blocks) so a single spare cat keeps planks and
    // blocks balanced enough to break ground. The woodworking (tools) bench is a luxury
    // tier that only draws a cat once both build materials are already stocked — leaving
    // it earlier would let it drain the planks/blocks the colony needs to grow. The order
    // keys only off pile-invariant resource totals, so it stays deterministic.
    // A dispatched offering has physically claimed its 10 goods plus the fixed
    // 20-material construction reserve. Raw-material benches must not consume
    // that escrow before the ritualist reaches the shrine.
    let offering_in_flight = active_or_queued_jobs(colony)
        .iter()
        .any(|job| job.kind == JobKind::CarryOffering);
    let offering_materials_reserve = if offering_in_flight {
        OFFERING_MATERIALS_RESERVE + f64::from(OFFERING_MATERIALS_AMOUNT)
    } else {
        0.0
    };
    let spendable_materials = (colony.resources.materials - offering_materials_reserve).max(0.0);
    let raw_material_reserve = if colony.resources.tools >= 1.0 && !offering_in_flight {
        OFFERING_MATERIALS_RESERVE
    } else {
        0.0
    };
    let raw_spendable_materials = (spendable_materials - raw_material_reserve).max(0.0);
    for bench_type in raw_material_staffing_priority(&colony.resources) {
        auto_staff_idle_buildings(colony, bench_type, gate.processed_through, false);
    }
    // P16/P19 clothing chain slice: the clothier/tannery are luxury benches like the
    // generic Workshop/AccountingTent above — staffed only from genuine surplus, sticky
    // (not released every tick like the raw-material trio), announced like Workshop.
    auto_staff_idle_buildings(colony, BuildingType::Clothier, gate.processed_through, true);
    auto_staff_idle_buildings(colony, BuildingType::Tannery, gate.processed_through, true);
    // P17/P19 ore→metal chain: the smelter is the same luxury/late-game shape as the
    // clothier/tannery above — staffed only from genuine idle surplus, sticky. A colony
    // with no smelter building has nothing to auto-staff here (no-op).
    auto_staff_idle_buildings(colony, BuildingType::Smelter, gate.processed_through, true);

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
                // P17 crop fertility: the base `field_yield` curve is the "full 1.0
                // fertility" rate; the fine per-tile climate biome under the field
                // scales it (grass ~0.8x, marsh ~1.5x, …). `tile_is_farmable`
                // already keeps fields off zero-fertility ground, so this is a
                // pure rate multiplier, never a placement gate.
                let fertility = crate::terrain_gen::tile_climate_biome(
                    world_seed,
                    colony.buildings[building_index].position.x,
                    colony.buildings[building_index].position.y,
                )
                .properties()
                .fertility;
                colony.resources.food += field_yield(production_elapsed) * fertility;
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
                        materials_available: spendable_materials,
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
                        EventKind::Production,
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
                        materials_available: spendable_materials,
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
                        EventKind::Production,
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

                // P17/P19 ore→metal chain: additive metal-forge sub-cycle, same bench/
                // worker, its own cycle timer (`colony.metal_forge_progress`, entirely
                // separate from `production_progress` above) — mirrors how the P19
                // trade-craft benches run alongside a workshop's primary refine. With no
                // metal ever smelted (`resources.metal == 0.0`, the case for every colony
                // without a smelter or without mountains) `advance_metal_forge` always
                // floors to zero cycles, so this is a strict no-op and the smithy's
                // forged-gear output stays byte-identical to before this chain existed.
                let forge_worker = assigned_worker(colony, &building_id);
                let forge_step = advance_metal_forge(
                    colony.metal_forge_progress,
                    production_elapsed,
                    MetalForgeOptions {
                        has_worker: forge_worker.is_some(),
                        worker_is_fast: forge_worker.is_some_and(|cat| {
                            cat.specialization == Some(CatSpecialization::Architect)
                        }),
                        metal_available: colony.resources.metal,
                    },
                );
                colony.metal_forge_progress = forge_step.next_progress;
                if forge_step.weapons_produced > 0.0 || forge_step.armor_produced > 0.0 {
                    colony.resources.metal =
                        (colony.resources.metal - forge_step.metal_used).max(0.0);
                    colony.resources.weapons += forge_step.weapons_produced;
                    colony.resources.armor += forge_step.armor_produced;
                    let site = colony.buildings[building_index].position;
                    route_output_to_nearest_pile(
                        colony,
                        ResourceKind::Weapons,
                        forge_step.weapons_produced,
                        site,
                    );
                    route_output_to_nearest_pile(
                        colony,
                        ResourceKind::Armor,
                        forge_step.armor_produced,
                        site,
                    );
                    append_event(
                        colony,
                        gate.processed_through,
                        EventKind::Production,
                        format!(
                            "The smith worked banked metal into {} extra weapon{} and {} extra armor.",
                            forge_step.weapons_produced,
                            if forge_step.weapons_produced == 1.0 {
                                ""
                            } else {
                                "s"
                            },
                            forge_step.armor_produced,
                        ),
                    );
                }
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
                        materials_available: raw_spendable_materials,
                    },
                );
                if step.refined_produced > 0.0 {
                    colony.resources.materials =
                        (colony.resources.materials - step.materials_used).max(0.0);
                    colony.resources.planks += step.refined_produced;
                    append_event(
                        colony,
                        gate.processed_through,
                        EventKind::Production,
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
                        materials_available: raw_spendable_materials,
                    },
                );
                if step.refined_produced > 0.0 {
                    colony.resources.materials =
                        (colony.resources.materials - step.materials_used).max(0.0);
                    colony.resources.blocks += step.refined_produced;
                    append_event(
                        colony,
                        gate.processed_through,
                        EventKind::Production,
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

                // P19 slice 2: additive stone-trade craft — same bench/worker, its own
                // cycle timer (`colony.stone_craft_progress`, entirely separate from
                // `production_progress` above), spends only *surplus* blocks above
                // STONE_TRADE_RECIPE's reserve so it never competes with construction
                // or the woodworking tools recipe for blocks.
                let craft_worker = assigned_worker(colony, &building_id);
                let craft_has_worker = craft_worker.is_some();
                let craft_is_architect = craft_worker
                    .is_some_and(|cat| cat.specialization == Some(CatSpecialization::Architect));
                let craft_skill = craft_worker.map_or(0.0, |cat| cat.skill(Labor::Craft));
                let craft_worker_id = craft_worker.map(|cat| cat.id.clone());
                let craft_step = advance_craft(
                    colony.stone_craft_progress,
                    production_elapsed,
                    CraftOptions {
                        has_worker: craft_has_worker,
                        worker_is_architect: craft_is_architect,
                        intermediate_available: colony.resources.blocks,
                    },
                    &STONE_TRADE_RECIPE,
                );
                colony.stone_craft_progress = craft_step.next_progress;
                if craft_step.items_produced > 0 {
                    colony.resources.blocks =
                        (colony.resources.blocks - craft_step.intermediate_used).max(0.0);
                    credit_trade_craft(
                        colony,
                        gate,
                        &STONE_TRADE_RECIPE,
                        craft_skill,
                        craft_worker_id,
                        craft_step.items_produced,
                        "stone-prep shop",
                    );
                }
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
                        planks_available: (colony.resources.planks - TOOL_BUILD_MATERIAL_RESERVE)
                            .max(0.0),
                        blocks_available: (colony.resources.blocks - TOOL_BUILD_MATERIAL_RESERVE)
                            .max(0.0),
                    },
                );
                if step.tools_produced > 0.0 {
                    colony.resources.planks = (colony.resources.planks - step.planks_used).max(0.0);
                    colony.resources.blocks = (colony.resources.blocks - step.blocks_used).max(0.0);
                    colony.resources.tools += step.tools_produced;
                    append_event(
                        colony,
                        gate.processed_through,
                        EventKind::Production,
                        format!(
                            "The woodworkers crafted {} tool{} from planks and blocks.",
                            step.tools_produced,
                            if step.tools_produced == 1.0 { "" } else { "s" }
                        ),
                    );
                }
                colony.buildings[building_index].production_progress = step.next_progress;

                // P19 slice 2: additive wood-trade craft — same bench/worker, its own
                // cycle timer (`colony.wood_craft_progress`), spends only *surplus*
                // planks above WOOD_TRADE_RECIPE's reserve so it never competes with
                // construction or the tools recipe above for planks. Tools ran first
                // and already claimed its share of `colony.resources.planks` this tick.
                let craft_worker = assigned_worker(colony, &building_id);
                let craft_has_worker = craft_worker.is_some();
                let craft_is_architect = craft_worker
                    .is_some_and(|cat| cat.specialization == Some(CatSpecialization::Architect));
                let craft_skill = craft_worker.map_or(0.0, |cat| cat.skill(Labor::Craft));
                let craft_worker_id = craft_worker.map(|cat| cat.id.clone());
                let craft_step = advance_craft(
                    colony.wood_craft_progress,
                    production_elapsed,
                    CraftOptions {
                        has_worker: craft_has_worker,
                        worker_is_architect: craft_is_architect,
                        intermediate_available: colony.resources.planks,
                    },
                    &WOOD_TRADE_RECIPE,
                );
                colony.wood_craft_progress = craft_step.next_progress;
                if craft_step.items_produced > 0 {
                    colony.resources.planks =
                        (colony.resources.planks - craft_step.intermediate_used).max(0.0);
                    credit_trade_craft(
                        colony,
                        gate,
                        &WOOD_TRADE_RECIPE,
                        craft_skill,
                        craft_worker_id,
                        craft_step.items_produced,
                        "woodworkers",
                    );
                }
            }
            BuildingType::Smelter => {
                // P17/P19 ore→metal chain: raw ore → refined metal bars, on the same
                // refinement-workshop cadence as the wood-cutter/stone-prep benches above.
                // `resources.ore` is only ever nonzero for a colony that has both reached
                // the mountains (mountaineering) and quarried them (`credit_quarry_ore`),
                // so a colony without either input simply never runs a cycle here.
                let worker = assigned_worker(colony, &building_id);
                let step = advance_workshop(
                    colony.buildings[building_index].production_progress,
                    production_elapsed,
                    WorkshopOptions {
                        has_worker: worker.is_some(),
                        worker_is_architect: worker.is_some_and(|cat| {
                            cat.specialization == Some(CatSpecialization::Architect)
                        }),
                        materials_available: colony.resources.ore,
                    },
                );
                if step.refined_produced > 0.0 {
                    colony.resources.ore = (colony.resources.ore - step.materials_used).max(0.0);
                    colony.resources.metal += step.refined_produced;
                    append_event(
                        colony,
                        gate.processed_through,
                        EventKind::Production,
                        format!(
                            "The smelter refined {} ore into {} metal bar{}.",
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
            BuildingType::Clothier => {
                // P16/P19 clothing chain slice: raw fibre → cloth, on the same
                // refinement-workshop cadence as the wood-cutter/stone-prep benches
                // (`advance_workshop` is generic over which resource it refines).
                let worker = assigned_worker(colony, &building_id);
                let step = advance_workshop(
                    colony.buildings[building_index].production_progress,
                    production_elapsed,
                    WorkshopOptions {
                        has_worker: worker.is_some(),
                        worker_is_architect: worker.is_some_and(|cat| {
                            cat.specialization == Some(CatSpecialization::Architect)
                        }),
                        materials_available: colony.resources.fibre,
                    },
                );
                if step.refined_produced > 0.0 {
                    colony.resources.fibre =
                        (colony.resources.fibre - step.materials_used).max(0.0);
                    colony.resources.cloth += step.refined_produced;
                    append_event(
                        colony,
                        gate.processed_through,
                        EventKind::Production,
                        format!(
                            "The clothier wove {} fibre into {} cloth.",
                            step.materials_used, step.refined_produced,
                        ),
                    );
                }
                colony.buildings[building_index].production_progress = step.next_progress;

                // Additive trade craft — same bench/worker, its own cycle timer
                // (`colony.clothier_craft_progress`), spends only *surplus* cloth above
                // CLOTH_TRADE_RECIPE's reserve. Mirrors the woodworking bench's
                // wood-trade craft tail exactly.
                let craft_worker = assigned_worker(colony, &building_id);
                let craft_has_worker = craft_worker.is_some();
                let craft_is_architect = craft_worker
                    .is_some_and(|cat| cat.specialization == Some(CatSpecialization::Architect));
                let craft_skill = craft_worker.map_or(0.0, |cat| cat.skill(Labor::Craft));
                let craft_worker_id = craft_worker.map(|cat| cat.id.clone());
                let craft_step = advance_craft(
                    colony.clothier_craft_progress,
                    production_elapsed,
                    CraftOptions {
                        has_worker: craft_has_worker,
                        worker_is_architect: craft_is_architect,
                        intermediate_available: colony.resources.cloth,
                    },
                    &CLOTH_TRADE_RECIPE,
                );
                colony.clothier_craft_progress = craft_step.next_progress;
                if craft_step.items_produced > 0 {
                    colony.resources.cloth =
                        (colony.resources.cloth - craft_step.intermediate_used).max(0.0);
                    credit_trade_craft(
                        colony,
                        gate,
                        &CLOTH_TRADE_RECIPE,
                        craft_skill,
                        craft_worker_id,
                        craft_step.items_produced,
                        "clothier",
                    );
                }
            }
            BuildingType::Tannery => {
                // P16/P19 clothing chain slice: raw hide → leather, same refinement
                // cadence as the clothier above.
                let worker = assigned_worker(colony, &building_id);
                let step = advance_workshop(
                    colony.buildings[building_index].production_progress,
                    production_elapsed,
                    WorkshopOptions {
                        has_worker: worker.is_some(),
                        worker_is_architect: worker.is_some_and(|cat| {
                            cat.specialization == Some(CatSpecialization::Architect)
                        }),
                        materials_available: colony.resources.hide,
                    },
                );
                if step.refined_produced > 0.0 {
                    colony.resources.hide = (colony.resources.hide - step.materials_used).max(0.0);
                    colony.resources.leather += step.refined_produced;
                    append_event(
                        colony,
                        gate.processed_through,
                        EventKind::Production,
                        format!(
                            "The tannery tanned {} hide into {} leather.",
                            step.materials_used, step.refined_produced,
                        ),
                    );
                }
                colony.buildings[building_index].production_progress = step.next_progress;

                // Additive trade craft, mirroring the clothier's tail above for leather.
                let craft_worker = assigned_worker(colony, &building_id);
                let craft_has_worker = craft_worker.is_some();
                let craft_is_architect = craft_worker
                    .is_some_and(|cat| cat.specialization == Some(CatSpecialization::Architect));
                let craft_skill = craft_worker.map_or(0.0, |cat| cat.skill(Labor::Craft));
                let craft_worker_id = craft_worker.map(|cat| cat.id.clone());
                let craft_step = advance_craft(
                    colony.tannery_craft_progress,
                    production_elapsed,
                    CraftOptions {
                        has_worker: craft_has_worker,
                        worker_is_architect: craft_is_architect,
                        intermediate_available: colony.resources.leather,
                    },
                    &LEATHER_TRADE_RECIPE,
                );
                colony.tannery_craft_progress = craft_step.next_progress;
                if craft_step.items_produced > 0 {
                    colony.resources.leather =
                        (colony.resources.leather - craft_step.intermediate_used).max(0.0);
                    credit_trade_craft(
                        colony,
                        gate,
                        &LEATHER_TRADE_RECIPE,
                        craft_skill,
                        craft_worker_id,
                        craft_step.items_produced,
                        "tannery",
                    );
                }
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

/// Shared item-crediting tail for a completed trade-craft cycle (P19 slice 2): picks
/// the next rotating [`ItemKind`] from the colony's item store ([`next_trade_kind`]),
/// derives quality from the assigned crafter's `Labor::Craft` skill
/// ([`craft_quality_from_skill`]), credits the item into `colony.items`, trains that
/// skill (ties the long-reserved [`Labor::Craft`] into the sim for the first time), and
/// logs the craft. Called once a bench's [`advance_craft`] step reports
/// `items_produced > 0`; the caller has already applied its own resource subtraction
/// (planks vs blocks) since that is the one thing that differs between the wood and
/// stone benches.
fn credit_trade_craft(
    colony: &mut ColonyRuntime,
    gate: TickGate,
    recipe: &crate::recipes::CraftRecipe,
    craft_skill: f64,
    craft_worker_id: Option<CatId>,
    items_produced: u32,
    bench_label: &str,
) {
    let kind = next_trade_kind(&colony.items, recipe);
    let quality = craft_quality_from_skill(craft_skill);
    let item = Item::new(kind, recipe.material, quality);
    colony.add_item(item, items_produced);
    if let Some(id) = craft_worker_id
        && let Some(idx) = colony
            .cats
            .iter()
            .position(|cat| cat.id == id && cat.death_time.is_none())
    {
        colony.cats[idx].gain_skill(Labor::Craft, SKILL_GAIN_PER_JOB);
    }
    append_event(
        colony,
        gate.processed_through,
        EventKind::Production,
        format!(
            "The {bench_label} crafted {} {} {} for trade.",
            items_produced,
            recipe.material.as_str(),
            kind.as_str()
        ),
    );
}

/// Phase 23c: a small passive per-cat fibre forage trickle (P16/P19 clothing chain
/// slice) — independent of any building, job, or worker assignment, feeding the
/// clothier's fibre → cloth refine. Deliberately minimal and additive: a full
/// dedicated gather-spot/forage system is a separate, larger card (P16 "Gather
/// spots") and out of scope here; this just keeps a trickle of raw fibre flowing so
/// a colony that builds a clothier always has *something* to work with.
fn phase_23c_fibre_forage(colony: &mut ColonyRuntime, gate: TickGate) {
    let alive = alive_cats(&colony.cats).count() as f64;
    let elapsed = gate.elapsed_sec as f64 * normalize_time_scale(colony);
    colony.resources.fibre += fibre_forage_yield(alive, elapsed);
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
            EventKind::ResearchUnlocked,
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
/// clears them. The death event carries `EventKind::Death(DeathCause)` with
/// the exact cause; the per-cat dehydration start/recovery edges use their own
/// `DehydrationCrisis`/`DehydrationRecovery` kinds, distinct from phase 8's
/// colony-wide `WaterCrisis`/`WaterRecovered` headline events even though both
/// pairs narrate the same underlying water shortage at different scopes.
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
                EventKind::DehydrationCrisis,
                format!("{cat_name} started dehydrating."),
            );
        }

        if result.recovered_from_dehydration {
            append_event(
                colony,
                gate.processed_through,
                EventKind::DehydrationRecovery,
                format!("{cat_name} recovered from dehydration."),
            );
        }

        if !result.died {
            continue;
        }

        // A dying carrier's yield is salvaged rather than lost, before the
        // cleanup below clears `carrying`.
        if let Some(carrying) = colony.cats[index].carrying.clone() {
            let deposit_at = position_to_world(colony.anchor, colony.cats[index].position);
            credit_carrying(colony, &carrying, deposit_at);
        }

        // Cancel the dying cat's own active/queued jobs (mirrors `retireCat`)
        // so none are left stuck waiting on an assigned cat that is now dead.
        cancel_cat_jobs(colony, &cat_id, gate.processed_through);

        let died_of_thirst = result.next_needs.thirst == 0.0;
        let died_of_hunger = result.next_needs.hunger == 0.0;
        let (cause, death_cause) = match (died_of_thirst, died_of_hunger) {
            (true, true) => (
                "starvation and dehydration",
                DeathCause::StarvationAndDehydration,
            ),
            (true, false) => ("dehydration", DeathCause::Dehydration),
            _ => ("starvation", DeathCause::Starvation),
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
            EventKind::Death(death_cause),
            format!("{cat_name} died from {cause}."),
        );
    }
}

/// Additive P15 phase: drop any dead scout's tentative fog reveal. A scout that
/// dies mid-excursion (survival, old age, or a raid earlier this tick) never
/// reaches the shrine to commit via `commit_scout_provisional_tiles`, so without
/// this its `provisional_tiles` entry would sit forever — the spec is explicit
/// that a dead scout's discoveries must NOT become permanent. Runs right after
/// every phase this tick that can set `death_time` before movement (phase 6 life
/// sim, phase 25 above); a death from the later raid director (phase 36) is
/// swept up on the following tick, which matches "should eventually clear."
fn phase_25b_prune_dead_scout_provisional_tiles(colony: &mut ColonyRuntime, _: TickGate) {
    if colony.provisional_tiles.is_empty() {
        return;
    }
    let alive_ids: HashSet<&str> = colony
        .cats
        .iter()
        .filter(|cat| cat.death_time.is_none())
        .map(|cat| cat.id.as_str())
        .collect();
    colony
        .provisional_tiles
        .retain(|cat_id, _| alive_ids.contains(cat_id.as_str()));
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
    respawn_starter_roster(colony, gate.processed_through);
    Some(RunResetReason::AllCatsDead)
}

/// TS parity (`server/game.ts` `ensureColony`: `createStarterCats` whenever
/// `aliveCats.length === 0`): a fully-extinct colony's run reset must repopulate with a
/// fresh founding roster, not leave a dead world. Without this, `phase_26_empty_colony_reset`
/// found zero living cats again on every subsequent tick and fired an `AllCatsDead` reset
/// forever (instrumented: seed 99 went extinct at game-hour ~114 and then banked 895
/// back-to-back resets over the rest of a 200-hour run with the colony permanently empty).
///
/// Deterministic: the roster is rolled from the colony's own founding seed forked by the
/// (just-incremented) run number, so each run gets distinct cats but two identical
/// trajectories respawn identical rosters. Ids carry the run number because the founding
/// `{colony_id}-cat-N` ids live on in the dead-cat records.
fn respawn_starter_roster(colony: &mut ColonyRuntime, now_ms: i64) {
    let seed = colony
        .test_rng_seed
        .unwrap_or(0)
        .wrapping_add(colony.run_number.wrapping_mul(1_000_003));
    let id_prefix = format!("{}-run{}", colony.id, colony.run_number);
    let roster = create_starter_cats_with_id_prefix(&colony.id, &id_prefix, now_ms, seed);
    colony.cats.extend(roster);
    // `reset_run` picked its interim leader while every cat was dead (i.e. none).
    colony.leader_id = choose_interim_leader_excluding(colony, None);
    append_event(
        colony,
        now_ms,
        EventKind::ResetReason,
        "New paws pad into the empty village - a fresh founding roster takes over the ruins.",
    );
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
fn phase_29_due_completion_gathering_explore_expansion(
    colony: &mut ColonyRuntime,
    gate: TickGate,
    world_seed: u32,
) {
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
                world_seed,
            ),
            JobKind::FetchWater => complete_fixed_yield_job(
                colony,
                &job,
                gate,
                WATER_TOTAL_YIELD,
                CarryingKind::Water,
                world_seed,
            ),
            JobKind::Explore => {
                append_event(
                    colony,
                    gate.processed_through,
                    EventKind::Discovery,
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
            JobKind::CarryOffering => complete_offering(colony, job, gate),
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
                | JobKind::CarryOffering
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
fn phase_31_mid_job_hauling(colony: &mut ColonyRuntime, gate: TickGate, world_seed: u32) {
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

        let total =
            total_yield.unwrap_or_else(|| total_yield_for_job(colony, &job, cat_index, world_seed));
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
        let haul_from = position_to_world(colony.anchor, colony.cats[cat_index].position);
        let haul_to = haul_destination(colony, carrying_kind, haul_from);
        colony.cats[cat_index].carrying = Some(Carrying {
            kind: carrying_kind,
            amount: share,
            job_ended_at: gate.processed_through,
            source_gather_spot: None,
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
    let gate_pos = movement_gate(colony.anchor, area_gate, ring_radius);

    MovementPassContext {
        movement_seed,
        movement_elapsed,
        wander_chance,
        ring_radius,
        anchor: colony.anchor,
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
            let world_pos = position_to_world(colony.anchor, position);
            // Deposit once the carrier reaches its haul destination — the pile it walked to,
            // or the shrine anchor when no designated pile accepts the resource. With no
            // designated piles this is exactly the shrine anchor, matching pre-haul-fill. A
            // P16 mover's cargo is bound for a genuine village pile, never back into a
            // gather spot, so it uses the gather-spot-excluding variant.
            let deposit_target = haul_deposit_target(colony, &carrying, world_pos);
            if !should_deposit(&carrying, world_pos, deposit_target, gate.processed_through) {
                continue;
            }

            credit_carrying(colony, &carrying, world_pos);
            // Only the rare Blessings deposit (ritual/offering payoff) is narrated —
            // ordinary Food/Materials/Water hauls fire once per single-cat trip
            // (hundreds per run) and would drown births/deaths/raids out of the
            // capped player-facing feed. See the `EventKind::BlessingDelivered` doc
            // comment for the taxonomy rationale.
            if carrying.kind == CarryingKind::Blessings {
                append_event(
                    colony,
                    gate.processed_through,
                    EventKind::BlessingDelivered,
                    deposit_message(&cat_id, &carrying),
                );
            }

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
                let world_pos = position_to_world(colony.anchor, colony.cats[cat_index].position);
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
    // Computed once per tick and shared by every cat's A* search this phase (cheap:
    // a HashSet built from the colony's building list, no terrain regeneration) —
    // also reused below to apply the matching movement-speed soft-obstacle penalty.
    let soft_obstacles = soft_obstacle_building_tiles(colony);
    let path_soft_obstacles = pathfinding_tile_set(&soft_obstacles);
    let walk_grid = build_colony_walk_grid(ColonyGridParams {
        tiles: &movement.walk_tiles,
        anchor: PathTilePos {
            x: movement.anchor.x,
            y: movement.anchor.y,
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
        // P17 transport upgrade: water stays blocked (pre-P17 behaviour, byte-
        // identical for every colony without the node) until `shipping` is owned.
        shipping_unlocked: is_owned(&colony.upgrade_tree, SHIPPING_NODE_ID),
        soft_obstacles: Some(&path_soft_obstacles),
    });
    // P17 transport upgrade: `rail` speeds up long-distance hauls once owned —
    // checked per-cat below against the remaining route distance, inert (no
    // owned-node lookup cost beyond this single flag) for every colony without it.
    let rail_unlocked = is_owned(&colony.upgrade_tree, RAIL_NODE_ID);
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

        let world_pos = position_to_world(colony.anchor, colony.cats[cat_index].position);
        let destination = position_to_world(colony.anchor, destination);
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
        // P14.2 soft obstacle (×0.25): a cat standing on a building footprint tile
        // or a terrain-generated tree tile pads along at a quarter speed — real,
        // not a biome approximation (a per-cat-per-tick check, the same cost order
        // as the `tile_biome` lookup just above, unlike a per-A*-cell query would
        // be), matching the same tier `BUILDING_FOOTPRINT_COST`/`FOREST_COST` cost
        // A* already routes around.
        let on_soft_obstacle = soft_obstacles.contains(&standing_tile_pos)
            || crate::terrain_gen::tile_has_tree(
                movement.world_seed,
                standing_tile_pos.x,
                standing_tile_pos.y,
            );
        let soft_obstacle_mult = soft_obstacle_speed_multiplier(on_soft_obstacle);
        // P17 `rail`: a long-distance haul (remaining route beyond
        // RAIL_LONG_HAUL_DISTANCE_TILES) moves much faster once the colony owns the
        // upgrade; village-scale routes (hunts, quarrying, shrine hauls) never
        // clear the threshold, so this is inert without it and inert for ordinary
        // routes even with it owned.
        let rail_mult = rail_speed_multiplier(
            rail_unlocked,
            straight_line_distance_tiles(world_pos, destination),
        );
        let speed = effective_move_speed(standing_biome, &cat_id, stage)
            * road_mult
            * soft_obstacle_mult
            * rail_mult
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
            reveal_and_wear_walked_tiles(colony, movement, &walk.tiles, &cat_id, current_task);
        }

        // P15: a scout's discoveries commit to the permanent map only once it is
        // physically back — `return_assigned_cat` (phase 30) is the only place that
        // sets a completed `Explore` job's cat to `Returning` with the shrine anchor
        // as its destination, so "arrived while Returning" is precisely "a scout (or
        // any other returning worker) just walked into the shrine." Non-scouts are a
        // no-op: they never got a `provisional_tiles` entry to commit.
        if arrived && activity == CatActivity::Returning {
            commit_scout_provisional_tiles(colony, &cat_id);
        }
    }
}

/// P16 gather-spot logistics, run once per tick right after movement/job-acceptance
/// (phase 34) so a mover that just arrived is picked up the same tick: expire TTL'd
/// gather spots (cancelling any mover mid-flight to one), complete arrived movers
/// (pick up the gather spot's contents and start the return haul), and — only once a
/// Steward is appointed, mirroring every other officer automation's "unfilled role,
/// no auto effect" contract (P12.2) — auto-designate a spot beside a distant,
/// heavily-worked resource site (P12.6/P16 follow-up) and dispatch a fresh mover for
/// any non-empty gather spot that doesn't already have one in flight. Entirely
/// additive/inert: a colony with no gather spots and no Steward takes none of these
/// branches, so this is a no-op byte-identical to pre-P16 behavior until a gather spot
/// is designated (manually, or automatically once a Steward is appointed).
fn phase_p16_gather_spot_logistics(colony: &mut ColonyRuntime, gate: TickGate) {
    expire_gather_spots(colony, gate.processed_through);
    complete_arrived_gather_haul_movers(colony, gate.processed_through);
    if colony.officers.contains_key(&OfficerRole::Steward) {
        auto_designate_gather_spots(colony, gate.processed_through);
        dispatch_gather_haul_movers(colony, gate.processed_through);
    }
}

/// Clear every gather spot whose TTL has elapsed: drop its bookkeeping record and its
/// underlying pile, cancel any mover job still targeting it, and let the next
/// `reconcile_colony_stockpiles` fold whatever it still held back into the shrine
/// reservoir — exactly like a manual `RemoveGatherSpot` (nothing is ever lost, just
/// re-routed).
fn expire_gather_spots(colony: &mut ColonyRuntime, now_ms: i64) {
    let expired: Vec<String> = colony
        .gather_spots
        .iter()
        .filter(|spot| spot.is_expired(now_ms))
        .map(|spot| spot.stockpile_id.clone())
        .collect();
    if expired.is_empty() {
        return;
    }
    colony
        .gather_spots
        .retain(|spot| !expired.contains(&spot.stockpile_id));
    colony.stockpiles.retain(|pile| !expired.contains(&pile.id));
    for stockpile_id in &expired {
        cancel_gather_haul_jobs_for_spot(colony, stockpile_id, now_ms);
    }
    reconcile_colony_stockpiles(colony);
}

/// Cancel any in-flight `haul_gather_spot` mover job(s) targeting `stockpile_id` and
/// free their assigned cat (unless it is already carrying picked-up cargo home — that
/// carry finishes normally; only the outbound leg is voided). Shared by the P16 TTL
/// expiry phase above and the player-facing `RemoveGatherSpot` action, so a job never
/// dangles waiting on a site that no longer exists.
pub(crate) fn cancel_gather_haul_jobs_for_spot(
    colony: &mut ColonyRuntime,
    stockpile_id: &str,
    now_ms: i64,
) {
    let cat_ids: Vec<CatId> = colony
        .jobs
        .iter_mut()
        .filter(|job| {
            job.kind == JobKind::HaulGatherSpot
                && matches!(job.status, JobStatus::Queued | JobStatus::Active)
                && matches!(
                    &job.metadata,
                    JobMetadata::GatherHaul { stockpile_id: id, .. } if id == stockpile_id
                )
        })
        .filter_map(|job| {
            job.status = JobStatus::Cancelled;
            job.completed_at = Some(now_ms);
            job.assigned_cat.clone()
        })
        .collect();

    for cat_id in cat_ids {
        if let Some(cat) = colony.cats.iter_mut().find(|cat| cat.id == cat_id)
            && cat.carrying.is_none()
        {
            cat.destination = None;
            cat.activity = CatActivity::Idle;
        }
    }
}

/// The resource a mover carries for a gather spot of this kind — the only three
/// resources a gatherer job can currently produce/carry (`entities::CarryingKind`).
fn carrying_kind_for_resource(kind: ResourceKind) -> Option<CarryingKind> {
    match kind {
        ResourceKind::Food => Some(CarryingKind::Food),
        ResourceKind::Water => Some(CarryingKind::Water),
        ResourceKind::Materials => Some(CarryingKind::Materials),
        ResourceKind::Herbs
        | ResourceKind::Refined
        | ResourceKind::Weapons
        | ResourceKind::Armor
        | ResourceKind::Blessings => None,
    }
}

/// Complete every `haul_gather_spot` job whose mover has arrived (`accepted: true`,
/// per the generic accept-on-arrival mechanism shared with `Site`/`Hauling` jobs) and
/// isn't already carrying something: pick up the gather spot's entire current balance
/// (a pile-to-pile transfer — `resources` is untouched, it was already counted when the
/// gatherer first dropped the yield here) and start the cat walking to the nearest
/// village pile/shrine that isn't itself a gather spot. If the spot emptied or
/// disappeared out from under the job (e.g. a concurrent expiry), the job still
/// completes and the cat is simply released, rather than left stuck forever.
fn complete_arrived_gather_haul_movers(colony: &mut ColonyRuntime, now_ms: i64) {
    let candidates: Vec<(usize, String, CatId)> = colony
        .jobs
        .iter()
        .enumerate()
        .filter_map(|(index, job)| {
            if job.status != JobStatus::Active || job.kind != JobKind::HaulGatherSpot {
                return None;
            }
            let JobMetadata::GatherHaul {
                stockpile_id,
                accepted: true,
                ..
            } = &job.metadata
            else {
                return None;
            };
            let cat_id = job.assigned_cat.clone()?;
            Some((index, stockpile_id.clone(), cat_id))
        })
        .collect();
    if candidates.is_empty() {
        return;
    }

    let gather_spot_ids: Vec<String> = colony
        .gather_spots
        .iter()
        .map(|spot| spot.stockpile_id.clone())
        .collect();

    for (job_index, stockpile_id, cat_id) in candidates {
        let Some(cat_index) = colony
            .cats
            .iter()
            .position(|cat| cat.id == cat_id && cat.death_time.is_none())
        else {
            continue;
        };
        if colony.cats[cat_index].carrying.is_some() {
            continue;
        }

        let spot_kind = colony
            .gather_spots
            .iter()
            .find(|spot| spot.stockpile_id == stockpile_id)
            .map(|spot| spot.kind);
        let pile_index = colony
            .stockpiles
            .iter()
            .position(|pile| pile.id == stockpile_id);

        let picked_up = match (spot_kind, pile_index) {
            (Some(kind), Some(pile_index)) => {
                let amount =
                    stockpiles::resource_amount(&colony.stockpiles[pile_index].contents, kind);
                if amount > 0.0 {
                    stockpiles::set_resource(
                        &mut colony.stockpiles[pile_index].contents,
                        kind,
                        0.0,
                    );
                    Some((kind, amount))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some((kind, amount)) = picked_up
            && let Some(carrying_kind) = carrying_kind_for_resource(kind)
        {
            let cat_pos = position_to_world(colony.anchor, colony.cats[cat_index].position);
            let dest_index = stockpiles::village_deposit_index(
                &colony.stockpiles,
                &gather_spot_ids,
                kind,
                cat_pos.x,
                cat_pos.y,
            );
            let dest = dest_index.map_or_else(
                || village_anchor_world(colony.anchor),
                |idx| {
                    let (cx, cy) = colony.stockpiles[idx].center();
                    WorldPos { x: cx, y: cy }
                },
            );

            let cat = &mut colony.cats[cat_index];
            cat.gain_skill(Labor::Haul, HAUL_SKILL_GAIN);
            cat.carrying = Some(Carrying {
                kind: carrying_kind,
                amount,
                job_ended_at: now_ms,
                source_gather_spot: Some(stockpile_id.clone()),
            });
            cat.destination = Some(position_from_world(dest));
            cat.activity = CatActivity::Returning;
        } else {
            // Nothing to pick up (the spot emptied/vanished underneath the job) —
            // release the cat instead of leaving it stuck on an unaccepted-forever job.
            let cat = &mut colony.cats[cat_index];
            cat.destination = None;
            cat.activity = CatActivity::Idle;
        }

        if let Some(job) = colony.jobs.get_mut(job_index) {
            job.status = JobStatus::Completed;
            job.completed_at = Some(now_ms);
        }
        append_event(
            colony,
            now_ms,
            EventKind::JobCompleted,
            "A mover collected a gather spot's goods.".to_owned(),
        );
    }
}

/// Minimum Chebyshev distance from the village anchor a resource site must sit at before
/// [`auto_designate_gather_spots`] will place a spot beside it. Inside this radius a
/// gatherer's own round trip to the shrine is already short, so splitting it into a
/// short-gather + long-haul mover leg would only add overhead instead of saving it. Set to
/// the leader's normal outer building ring ([`DEFAULT_MAX_RING`]) — sites the colony would
/// have to range beyond its own footprint for are exactly the ones the split pays off for.
const AUTO_GATHER_SPOT_MIN_DISTANCE: i32 = DEFAULT_MAX_RING;

/// A resource site must be drawing at least this many concurrent active/queued gatherers
/// before [`auto_designate_gather_spots`] treats it as "heavily/repeatedly worked" enough to
/// justify auto-placing a spot. A single gatherer's own round trip gets no benefit from a
/// hand-off (it would just add a mover leg on top), so this keeps placement conservative —
/// never spamming a spot for one lone hunter/quarrier/water-fetcher.
const AUTO_GATHER_SPOT_MIN_WORKERS: usize = 2;

/// The [`ResourceKind`] an active/queued job of `kind` gathers, for the three job kinds
/// [`auto_designate_gather_spots`] watches — the same three [`carrying_kind_for_job`] (via
/// [`carrying_resource_kind`]) maps a gatherer's own haul to. `None` for every other job
/// kind (nothing else resolves a [`JobMetadata::Hauling`] site to a gatherable resource).
fn resource_kind_for_gather_job(kind: JobKind) -> Option<ResourceKind> {
    match kind {
        JobKind::HuntExpedition => Some(ResourceKind::Food),
        JobKind::Quarry => Some(ResourceKind::Materials),
        JobKind::FetchWater => Some(ResourceKind::Water),
        _ => None,
    }
}

/// Chebyshev (8-directional) tile distance between two tiles — the same metric
/// [`cheb_from_anchor`] uses against the village anchor, generalized to any pair.
fn tile_cheb_distance(a: TilePos, b: TilePos) -> i32 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}

/// The cardinal-neighbour tile of `site` closest to the village anchor — where an
/// auto-designated gather spot lands. Deliberately adjacent to (never on top of) the worked
/// resource tile itself: many resource tiles (river water, mountain/cave quarry ground) are
/// otherwise hard-blocked terrain (see `pathfinding::WalkGrid::is_blocked`), so the spot must
/// sit on open ground beside the site, not on it. Biasing toward the anchor also shortens the
/// mover's long-haul leg. Ties broken by a fixed compass order (north, east, south, west) —
/// no RNG, so this is fully deterministic.
fn gather_spot_tile_adjacent_to(anchor: TilePos, site: TilePos) -> TilePos {
    const OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    OFFSETS
        .iter()
        .map(|&(dx, dy)| TilePos {
            x: site.x + dx,
            y: site.y + dy,
        })
        .min_by_key(|&tile| (cheb_from_anchor(anchor, tile), tile.x, tile.y))
        .expect("OFFSETS is non-empty")
}

/// Steward automation (P12.6/P16 follow-up): auto-designate a gather spot beside a distant,
/// heavily-worked resource site, exactly as if a player had run `DesignateGatherSpot` there.
/// Reuses every downstream P16/P12.3 mechanism unchanged — deposit routing already favours
/// the new spot over the shrine once it exists (it's nearer the gatherer's work site),
/// [`dispatch_gather_haul_movers`] picks up its contents next tick, and TTL expiry clears it
/// exactly like a manual spot.
///
/// A resource site qualifies only when *all* of the following hold, so this stays
/// conservative and never spams spots:
/// - it is currently the resolved `site` of `>=` [`AUTO_GATHER_SPOT_MIN_WORKERS`] concurrent
///   active/queued `hunt_expedition`/`quarry`/`fetch_water` jobs (repeatedly worked *right
///   now*, not merely once, historically);
/// - it sits `>=` [`AUTO_GATHER_SPOT_MIN_DISTANCE`] tiles (Chebyshev) from the village anchor
///   (far enough that the short-gather + long-haul split actually pays off);
/// - no existing gather spot of the same resource kind already sits within 1 tile of it (no
///   duplicate spot on an already-served site);
/// - the gather-spot budget ([`MAX_GATHER_SPOTS`]) has room.
///
/// Candidates are ordered deterministically (farthest first, then most heavily worked, then
/// stable tile order — no RNG) and placed one at a time until the budget is spent, so a tick
/// with several qualifying sites still produces a fully reproducible result. A no-op — byte
/// identical to pre-this-feature behavior — whenever no site qualifies (including every
/// colony with no Steward, since the caller only invokes this once one is appointed).
fn auto_designate_gather_spots(colony: &mut ColonyRuntime, now_ms: i64) {
    if colony.gather_spots.len() >= MAX_GATHER_SPOTS {
        return;
    }
    let anchor = colony.anchor;

    let mut worker_counts: BTreeMap<(ResourceKind, TilePos), usize> = BTreeMap::new();
    for job in &colony.jobs {
        if !matches!(job.status, JobStatus::Active | JobStatus::Queued) {
            continue;
        }
        let Some(kind) = resource_kind_for_gather_job(job.kind) else {
            continue;
        };
        let JobMetadata::Hauling {
            site: Some(site), ..
        } = job.metadata
        else {
            continue;
        };
        *worker_counts.entry((kind, site)).or_insert(0) += 1;
    }

    let already_served: Vec<(ResourceKind, TilePos)> = colony
        .gather_spots
        .iter()
        .filter_map(|spot| {
            colony
                .stockpiles
                .iter()
                .find(|pile| pile.id == spot.stockpile_id)
                .map(|pile| {
                    let (cx, cy) = pile.center();
                    (spot.kind, world_pos_to_tile(WorldPos { x: cx, y: cy }))
                })
        })
        .collect();

    let mut candidates: Vec<(ResourceKind, TilePos, usize)> = worker_counts
        .into_iter()
        .filter(|&((_, site), count)| {
            count >= AUTO_GATHER_SPOT_MIN_WORKERS
                && cheb_from_anchor(anchor, site) >= AUTO_GATHER_SPOT_MIN_DISTANCE
        })
        .filter(|&((kind, site), _)| {
            !already_served.iter().any(|&(served_kind, served_site)| {
                served_kind == kind && tile_cheb_distance(served_site, site) <= 1
            })
        })
        .map(|((kind, site), count)| (kind, site, count))
        .collect();

    // Deterministic order: farthest (most beneficial split) first, then most heavily
    // worked, then stable tile/kind order as a final tie-break. No RNG.
    candidates.sort_by(|a, b| {
        cheb_from_anchor(anchor, b.1)
            .cmp(&cheb_from_anchor(anchor, a.1))
            .then_with(|| b.2.cmp(&a.2))
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.0.cmp(&b.0))
    });

    for (kind, site, _) in candidates {
        if colony.gather_spots.len() >= MAX_GATHER_SPOTS {
            break;
        }
        let spot_tile = gather_spot_tile_adjacent_to(anchor, site);
        let id = format!("gather-auto-{now_ms}-{}", colony.stockpiles.len() + 1);
        colony.stockpiles.push(Stockpile {
            id: id.clone(),
            rect: ZoneRect {
                x1: spot_tile.x,
                y1: spot_tile.y,
                x2: spot_tile.x,
                y2: spot_tile.y,
            },
            accepts: std::iter::once(kind).collect(),
            contents: Resources::default(),
        });
        colony.gather_spots.push(GatherSpot {
            stockpile_id: id,
            kind,
            expires_at_ms: now_ms + stockpiles::GATHER_SPOT_TTL_MS,
        });
    }
    reconcile_colony_stockpiles(colony);
}

/// Steward-gated (P12.6-style officer automation, folded in from the P16 spec's
/// "Steward auto-stockpile automation" ask): dispatch one fresh `haul_gather_spot`
/// mover per non-empty gather spot that doesn't already have a queued/active mover, in
/// stable designation order, using the same generic best-idle-cat picker every other
/// quiet system dispatch uses (`select_best_cat`, no specialization preference — hauling
/// is unspecialized labor). *Where* gather spots come from — manually player-designated,
/// or auto-designated by [`auto_designate_gather_spots`] just above — is irrelevant here;
/// this only automates the logistics leg once a spot already exists.
fn dispatch_gather_haul_movers(colony: &mut ColonyRuntime, now_ms: i64) {
    let targeted_spots: HashSet<String> = colony
        .jobs
        .iter()
        .filter(|job| {
            job.kind == JobKind::HaulGatherSpot
                && matches!(job.status, JobStatus::Queued | JobStatus::Active)
        })
        .filter_map(|job| match &job.metadata {
            JobMetadata::GatherHaul { stockpile_id, .. } => Some(stockpile_id.clone()),
            _ => None,
        })
        .collect();

    let due_spots: Vec<String> = colony
        .gather_spots
        .iter()
        .filter(|spot| !targeted_spots.contains(&spot.stockpile_id))
        .filter(|spot| {
            colony
                .stockpiles
                .iter()
                .find(|pile| pile.id == spot.stockpile_id)
                .is_some_and(|pile| stockpiles::resource_amount(&pile.contents, spot.kind) > 0.0)
        })
        .map(|spot| spot.stockpile_id.clone())
        .collect();

    for stockpile_id in due_spots {
        let Some(cat_id) = select_best_cat(colony, None) else {
            break;
        };
        queue_job(
            colony,
            now_ms,
            JobKind::HaulGatherSpot,
            Some(cat_id),
            JobMetadata::GatherHaul {
                stockpile_id,
                site: None,
                accepted: false,
            },
        );
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
            anchor_x: colony.anchor.x,
            anchor_y: colony.anchor.y,
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
            EventKind::RoadBuilt,
            format!(
                "The leader had a well-worn trail paved into a road ({paved} tile{}).",
                if paved == 1 { "" } else { "s" }
            ),
        );
    }
}

/// P14.4: tiles bordering `building`'s footprint but not part of it — the
/// candidate spots a connecting road tile could occupy. Sorted `(y, x)`
/// ascending so callers get a stable "first" entrance, matching the tie-break
/// convention [`select_road_corridor`] already uses for corridor picking.
fn building_entrance_candidates(building: &BuildingRuntime) -> Vec<TilePos> {
    let (w, h) = footprint_for(building.building_type);
    let footprint: HashSet<TilePos> = footprint_tiles(building.position, w, h)
        .into_iter()
        .collect();
    let mut entrances: HashSet<TilePos> = HashSet::new();
    for tile in &footprint {
        for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let neighbour = TilePos {
                x: tile.x + dx,
                y: tile.y + dy,
            };
            if !footprint.contains(&neighbour) {
                entrances.insert(neighbour);
            }
        }
    }
    let mut entrances: Vec<TilePos> = entrances.into_iter().collect();
    entrances.sort_by_key(|tile| (tile.y, tile.x));
    entrances
}

/// The shrine's footprint tiles — the road network's anchor. Empty before the
/// shrine exists (never happens for a founded colony, but keeps this total).
fn shrine_footprint_tiles(colony: &ColonyRuntime) -> HashSet<TilePos> {
    colony
        .buildings
        .iter()
        .find(|building| building.building_type == BuildingType::Shrine)
        .map(building_footprint_tiles)
        .map(|tiles| tiles.into_iter().collect())
        .unwrap_or_default()
}

/// A freshly-generated ground tile for a position `colony.world_tiles` hasn't
/// recorded yet. The accessibility spur can path outside the initial
/// chunk-populated area (the claimed-site fallback spiral in
/// `next_claimed_building_site` already does the same) — meadow, no
/// resources, no danger, matching the sparse-population convention `tile_cost`
/// already assumes elsewhere (`None` reads as open ground).
fn fresh_ground_tile(pos: TilePos) -> WorldTileRuntime {
    WorldTileRuntime {
        pos,
        tile_type: TileType::Meadow,
        resources: TileResources {
            food: 0,
            herbs: 0,
            water: 0,
        },
        max_resources: MaxResources { food: 0, herbs: 0 },
        danger_level: 0.0,
        path_wear: 0,
        last_depleted: 0,
        overlay_feature: None,
    }
}

/// P14.4 accessibility router: the fresh tiles (network tile excluded) that
/// would connect `building`'s entrance to the existing road network, or
/// `None` if no entrance can reach it within [`ACCESS_ROAD_MAX_EXPANSIONS`]
/// steps. `Some(&[])` means the building already touches the network (an
/// existing road, or the shrine's own footprint) — nothing to pave.
///
/// The network predicate takes priority over the blocked predicate: the
/// shrine's footprint is technically "occupied" by the shrine building, but
/// the spec calls it out as passable — the hub every road leads to — so a
/// route may terminate there even though `tile_is_occupied` would otherwise
/// refuse to step on it.
fn building_road_route_to_shrine(
    colony: &ColonyRuntime,
    building: &BuildingRuntime,
    world_seed: u32,
) -> Option<Vec<TilePos>> {
    let entrances = building_entrance_candidates(building);
    let shrine_footprint = shrine_footprint_tiles(colony);
    let is_network = |pos: roads::RoadPos| {
        let tile = TilePos { x: pos.x, y: pos.y };
        shrine_footprint.contains(&tile)
            || colony
                .world_tiles
                .get(&tile)
                .is_some_and(|t| t.overlay_feature.as_deref() == Some("road_built"))
    };
    let is_blocked =
        |pos: roads::RoadPos| tile_is_occupied(colony, TilePos { x: pos.x, y: pos.y }, world_seed);
    let road_entrances: Vec<roads::RoadPos> = entrances
        .iter()
        .map(|tile| roads::RoadPos {
            x: tile.x,
            y: tile.y,
        })
        .collect();

    roads::road_route_to_network(
        &road_entrances,
        is_blocked,
        is_network,
        ACCESS_ROAD_MAX_EXPANSIONS,
    )
    .map(|route| {
        route
            .into_iter()
            .map(|pos| TilePos { x: pos.x, y: pos.y })
            .collect()
    })
}

/// P14.4 accessibility invariant: does `building`'s footprint already touch a
/// road tile connected — through the paved network — back to the shrine? A
/// building bordering the shrine's own footprint directly counts as connected
/// (zero-length route) with no paving required.
#[must_use]
pub fn building_is_road_connected_to_shrine(
    colony: &ColonyRuntime,
    building: &BuildingRuntime,
    world_seed: u32,
) -> bool {
    // `Some(&[])` is "already touches the network" (nothing to pave); a
    // `Some` non-empty route means the network is *reachable* but not yet
    // paved to — still disconnected until `phase_35b_road_accessibility`
    // actually lays it.
    building_road_route_to_shrine(colony, building, world_seed)
        .is_some_and(|route| route.is_empty())
}

/// Phase 35b: P14.4 road accessibility. Once per minute, sweep buildings that
/// aren't yet road-connected to the shrine and pave a fresh spur to the
/// nearest network tile — gated on the same [`ROAD_MATERIALS_RESERVE`] spare-
/// materials reserve [`phase_35_deliberate_roads`] already protects, so
/// accessibility paving draws from the same "spare capacity only" budget and
/// never competes with construction/offerings for the reserve.
///
/// Policy: a building whose full spur wouldn't fit the remaining per-sweep
/// budget is skipped this minute and retried later once materials recover —
/// paving is all-or-nothing per building (never a dangling half-route that
/// doesn't actually reach the network). The shrine (already the network's
/// anchor) and wall segments (not villager buildings) are excluded from the
/// sweep.
fn phase_35b_road_accessibility(colony: &mut ColonyRuntime, gate: TickGate, world_seed: u32) {
    if !gate.minute_rolled {
        return;
    }

    let mut budget = colony.resources.materials - ROAD_MATERIALS_RESERVE;
    if budget <= 0.0 {
        return;
    }

    let mut candidates: Vec<usize> = colony
        .buildings
        .iter()
        .enumerate()
        .filter(|(_, building)| {
            !matches!(
                building.building_type,
                BuildingType::Shrine | BuildingType::Walls
            )
        })
        .map(|(index, _)| index)
        .collect();
    // Stable sweep order independent of incidental Vec ordering noise.
    candidates.sort_by(|&left, &right| colony.buildings[left].id.cmp(&colony.buildings[right].id));

    let mut paved_tiles = 0_usize;
    let mut connected_buildings = 0_usize;

    for index in candidates {
        if budget <= 0.0 {
            break;
        }

        let Some(route) =
            building_road_route_to_shrine(colony, &colony.buildings[index], world_seed)
        else {
            continue;
        };
        if route.is_empty() || route.len() as f64 > budget {
            continue;
        }

        for tile_pos in &route {
            let tile = colony
                .world_tiles
                .entry(*tile_pos)
                .or_insert_with(|| fresh_ground_tile(*tile_pos));
            tile.overlay_feature = Some("road_built".to_owned());
            tile.path_wear = 100;
        }
        colony.resources.materials -= route.len() as f64;
        budget -= route.len() as f64;
        paved_tiles += route.len();
        connected_buildings += 1;
    }

    if paved_tiles > 0 {
        append_event(
            colony,
            gate.processed_through,
            EventKind::RoadBuilt,
            format!(
                "The leader paved {paved_tiles} tile{} of road to connect {connected_buildings} building{} to the shrine.",
                if paved_tiles == 1 { "" } else { "s" },
                if connected_buildings == 1 { "" } else { "s" }
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

/// Off-map standoff point a trader spawns at / departs to: `TRADER_SPAWN_DISTANCE`
/// tiles due west of the current village gate. A fixed heading (not a rolled angle like
/// `spawn_raid`'s) — see `trader` module docs for why traders need no RNG at all.
fn trader_spawn_world_position(colony: &ColonyRuntime) -> WorldPos {
    let gate_pos = tile_pos_to_world(raid_gate_position(colony));
    WorldPos {
        x: gate_pos.x - trader::TRADER_SPAWN_DISTANCE,
        y: gate_pos.y,
    }
}

fn spawn_trader(colony: &ColonyRuntime, gate: TickGate) -> TraderRuntime {
    TraderRuntime {
        id: format!("trader-{}", gate.processed_through),
        position: position_from_world(trader_spawn_world_position(colony)),
        destination: None,
        state: trader::TraderState::Arriving,
        arrived_at: None,
    }
}

/// Phase 36b: spawn a visiting trader on its deterministic game-time schedule, and
/// advance an in-progress visit's lifecycle (Arriving -> Trading -> Departing -> gone).
/// A friendly, RNG-free analogue of `phase_36_threat_and_raid_director`'s spawn/march
/// structure — see [`crate::trader`] for the schedule/movement constants. Purely
/// additive: never touches `resources`, `threat_pressure`, or any other phase's state,
/// so a trader visiting (or never having visited) cannot itself destabilize the
/// survival/threat loops. `SellGoods` / `BuyResource` (player actions) are the only way
/// `coin`/`items`/`resources` change because of a trader.
fn phase_36b_trader_lifecycle(colony: &mut ColonyRuntime, gate: TickGate) {
    let scale = normalize_time_scale(colony);

    if colony.trader.is_none() {
        let reference_at = colony
            .last_trader_departed_at
            .unwrap_or(colony.run_started_at);
        let elapsed_game_sec =
            (gate.processed_through - reference_at).max(0) as f64 / 1000.0 * scale;
        if elapsed_game_sec >= trader::TRADER_VISIT_INTERVAL_GAME_HOURS * 3600.0 {
            colony.trader = Some(spawn_trader(colony, gate));
            append_event(
                colony,
                gate.processed_through,
                EventKind::TraderArrived,
                "A trader's wagon has been spotted approaching the village.",
            );
        }
        return;
    }

    let gate_pos = tile_pos_to_world(raid_gate_position(colony));
    let spawn_pos = trader_spawn_world_position(colony);
    let movement_budget = gate.elapsed_sec as f64 * scale * trader::TRADER_SPEED_TILES_PER_SEC;
    let state = colony.trader.as_ref().expect("checked Some above").state;

    match state {
        trader::TraderState::Arriving => {
            let current = {
                let unit = colony.trader.as_ref().expect("checked Some above");
                WorldPos {
                    x: unit.position.x,
                    y: unit.position.y,
                }
            };
            let walk = walk_path(current, gate_pos, movement_budget, &[]);
            let arrived =
                cheb_distance_world(walk.position, gate_pos) <= trader::TRADER_ARRIVE_RANGE;
            {
                let unit = colony.trader.as_mut().expect("checked Some above");
                unit.position = position_from_world(walk.position);
                unit.destination = Some(position_from_world(gate_pos));
                if arrived {
                    unit.state = trader::TraderState::Trading;
                    unit.arrived_at = Some(gate.processed_through);
                }
            }
            if arrived {
                append_event(
                    colony,
                    gate.processed_through,
                    EventKind::TraderTrading,
                    "The trader has set up shop at the gate and is ready to deal.",
                );
            }
        }
        trader::TraderState::Trading => {
            let arrived_at = colony
                .trader
                .as_ref()
                .expect("checked Some above")
                .arrived_at
                .unwrap_or(gate.processed_through);
            let linger_sec = (gate.processed_through - arrived_at).max(0) as f64 / 1000.0 * scale;
            if linger_sec >= trader::TRADER_LINGER_GAME_HOURS * 3600.0 {
                colony.trader.as_mut().expect("checked Some above").state =
                    trader::TraderState::Departing;
            }
        }
        trader::TraderState::Departing => {
            let current = {
                let unit = colony.trader.as_ref().expect("checked Some above");
                WorldPos {
                    x: unit.position.x,
                    y: unit.position.y,
                }
            };
            let walk = walk_path(current, spawn_pos, movement_budget, &[]);
            let departed =
                cheb_distance_world(walk.position, spawn_pos) <= trader::TRADER_ARRIVE_RANGE;
            {
                let unit = colony.trader.as_mut().expect("checked Some above");
                unit.position = position_from_world(walk.position);
                unit.destination = Some(position_from_world(spawn_pos));
            }
            if departed {
                colony.trader = None;
                colony.last_trader_departed_at = Some(gate.processed_through);
                append_event(
                    colony,
                    gate.processed_through,
                    EventKind::TraderDeparted,
                    "The trader's wagon has departed for distant lands.",
                );
            }
        }
    }
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
            EventKind::WaterRecovered,
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
    // P16: every gather spot's underlying pile just got dropped above, so drop their
    // bookkeeping records too (no stale entries pointing at piles that no longer exist).
    colony.gather_spots.clear();
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
    // A mid-visit trader (if any) doesn't survive a collapse cleanly — its position/
    // schedule assumptions (e.g. `run_started_at`-relative timing) no longer hold post-
    // reset. `coin` itself is untouched here (see `ColonyRuntime::coin`'s doc comment):
    // it is banked player wealth, not run state.
    colony.trader = None;

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
        EventKind::ResetReason,
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
        planks: 6.0,
        blocks: 6.0,
        tools: 0.0,
        fibre: 0.0,
        hide: 0.0,
        cloth: 0.0,
        leather: 0.0,
        ore: 0.0,
        metal: 0.0,
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
        x: (f64::from(colony.anchor.x) + angle.cos() * RAID_SPAWN_DISTANCE).round() as i32,
        y: (f64::from(colony.anchor.y) + angle.sin() * RAID_SPAWN_DISTANCE).round() as i32,
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
        EventKind::Raid(RaidPhase::Launched),
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
    let anchor = colony.anchor;

    for _ in 0..clicks {
        let target = colony
            .raiders
            .iter()
            .enumerate()
            .filter(|(_, raider)| raider.raid_id == active_raid_id && raider.health > 0.0)
            .min_by(|(_, left), (_, right)| {
                cheb_distance_world(position_to_world(anchor, left.position), gate_pos)
                    .partial_cmp(&cheb_distance_world(
                        position_to_world(anchor, right.position),
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
            EventKind::Raid(RaidPhase::Repelled),
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
            EventKind::Raid(RaidPhase::Won),
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
                    EventKind::Death(DeathCause::Raid),
                    format!("{victim_name} fell defending the gate as the raiders broke through."),
                );
            }
        }
        append_event(
            colony,
            gate.processed_through,
            EventKind::Raid(RaidPhase::Lost),
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
        x: colony.anchor.x,
        y: colony.anchor.y + village_ring_radius(colony.buildings.len() as i32),
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
    resources.fibre = clamp_resource(resources.fibre, caps.fibre);
    resources.hide = clamp_resource(resources.hide, caps.hide);
    resources.cloth = clamp_resource(resources.cloth, caps.cloth);
    resources.leather = clamp_resource(resources.leather, caps.leather);
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

/// Offerings outrank the non-sticky raw-material benches: if the director's
/// employment fill used every idle cat, reclaim one luxury-bench worker rather
/// than leaving an otherwise reachable shrine action permanently undispatched.
/// `queue_job` releases the selected worker's bench assignment.
fn select_best_raw_material_worker(
    colony: &ColonyRuntime,
    specialization: Option<CatSpecialization>,
) -> Option<CatId> {
    let busy_ids = active_or_queued_jobs(colony)
        .iter()
        .filter_map(|job| job.assigned_cat.as_deref())
        .collect::<Vec<_>>();
    let raw_worker_ids = colony
        .buildings
        .iter()
        .filter(|building| RAW_MATERIAL_WORKSHOPS.contains(&building.building_type))
        .filter_map(|building| building.assigned_cat.as_deref())
        .collect::<Vec<_>>();
    let available = colony
        .cats
        .iter()
        .filter(|cat| {
            raw_worker_ids.contains(&cat.id.as_str()) && can_take_new_job_with_busy(cat, &busy_ids)
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
        JobKind::Ritual | JobKind::CarryOffering => Some(TaskType::Rest),
        JobKind::HaulGatherSpot => Some(TaskType::Build),
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
        boosted: cat.boosted,
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
            EventKind::WorkerAssigned,
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

// NOTE (P17): site *discovery* (`has_quarry_site`/`quarry_sites_near_village`)
// deliberately keeps the cheap legacy coarse-type check only — `tile_is_minable`
// once OR'd in a fine per-tile climate-biome lookup here too, but that function
// regenerates its whole 12x12 chunk per call (see `terrain_gen::tile_climate_biome`),
// and these two scan *every* explored tile *every tick*; OR-ing it in made a
// long-horizon tick pass effectively hang (each tick paying chunk-regen cost per
// explored tile). The fine biome's mining rule is still wired in, just at
// `total_yield_for_job`'s `Quarry` arm below — a single lookup per job, at the
// job's one dispatched site, not a per-tick full-map scan.
fn has_quarry_site(colony: &ColonyRuntime) -> bool {
    // Must agree with `quarry_sites_near_village` (which gates on `tile_is_explored`),
    // exactly like the `has_water_site`/`water_sites_near_village` pair below. Keying
    // the leader director's Quarry veto (`!snapshot.has_quarry_site`) on the strict
    // `path_wear > 62` while the finder accepts `tile_is_explored` (`path_wear > 62` OR
    // `cheb_from_anchor <= 6`) is the same veto/finder asymmetry that starved water: a
    // mountain inside the founding reveal band but off any traffic halo would be a valid
    // site the veto permanently rejects. Reconciled to `tile_is_explored`.
    colony.world_tiles.values().any(|tile| {
        matches!(tile.tile_type, TileType::Mountains | TileType::CaveEntrance)
            && tile_is_explored(colony.anchor, tile)
    })
}

fn has_water_site(colony: &ColonyRuntime) -> bool {
    // Must agree with `water_sites_near_village`: any water tile that site-finder can
    // hand a `FetchWater` job is a valid site here, or the leader director's veto
    // (`!snapshot.has_water_site`) permanently blocks water fetching even though a
    // reachable source exists. A tile is a usable site when it is explored — worn into
    // a known route (`path_wear > 62`) OR already inside the founding reveal band
    // (`cheb_from_anchor <= 6`, see `tile_is_explored`). Before this was reconciled the
    // founding pond (carved at cheb 2..=6 with `path_wear == 0`) failed this gate, so a
    // fresh colony never fetched water and slowly bled its founding reservoir dry.
    colony
        .world_tiles
        .values()
        .any(|tile| tile_has_water(Some(tile)) && tile_is_explored(colony.anchor, tile))
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
        EventKind::Election(ElectionPhase::Opened),
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

fn village_anchor_world(anchor: TilePos) -> WorldPos {
    WorldPos {
        x: f64::from(anchor.x),
        y: f64::from(anchor.y),
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

fn position_to_world(anchor: TilePos, pos: Position) -> WorldPos {
    match pos.map {
        MapType::World => WorldPos { x: pos.x, y: pos.y },
        MapType::Colony => WorldPos {
            x: pos.x + f64::from(anchor.x),
            y: pos.y + f64::from(anchor.y),
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
        .map(|cat| position_to_world(colony.anchor, cat.position))?;
    let dir = next_movement_roll(movement);
    let len = next_movement_roll(movement);
    Some(scout_wander_target(
        from,
        village_anchor_world(colony.anchor),
        dir,
        len,
    ))
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

fn movement_gate(
    anchor: TilePos,
    area_gate: Option<AreaGatePlacement>,
    ring_radius: i32,
) -> TilePos {
    if let Some(gate) = area_gate {
        let delta = side_delta(gate.side);
        return TilePos {
            x: gate.x + delta.x,
            y: gate.y + delta.y,
        };
    }

    TilePos {
        x: anchor.x,
        y: anchor.y + ring_radius,
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

/// Converts a `world_tick::TilePos` set (e.g. [`soft_obstacle_building_tiles`])
/// into the `pathfinding::TilePos` set `ColonyGridParams::soft_obstacles` expects.
fn pathfinding_tile_set(tiles: &HashSet<TilePos>) -> HashSet<PathTilePos> {
    tiles
        .iter()
        .map(|tile| PathTilePos {
            x: tile.x,
            y: tile.y,
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
        .map_or_else(
            || village_anchor_world(colony.anchor),
            |building| tile_pos_to_world(building.position),
        )
}

fn is_inside_movement_village(pos: TilePos, movement: &MovementPassContext) -> bool {
    if movement.claimed_area.is_empty() {
        return cheb_from_anchor(movement.anchor, pos) < movement.ring_radius;
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
            JobMetadata::GatherHaul {
                site: Some(site),
                accepted: false,
                ..
            } => Some((index, site)),
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
        JobMetadata::GatherHaul {
            stockpile_id, site, ..
        } => JobMetadata::GatherHaul {
            stockpile_id,
            site,
            accepted: true,
        },
        other => other,
    };
}

fn reveal_and_wear_walked_tiles(
    colony: &mut ColonyRuntime,
    movement: &MovementPassContext,
    walked: &[WorldPos],
    cat_id: &str,
    current_task: Option<TaskType>,
) {
    if walked.is_empty() {
        return;
    }

    // P15 two-tier reveal: a cat with a live `provisional_tiles` entry is a scout
    // currently out on (or walking home from) an `Explore` job — see
    // `phase_15_assign_promoted_job_destinations`, which is the sole place that
    // entry gets created. Everyone else (regular cats, village-expansion movers)
    // keeps committing straight to `revealed_tiles`, unchanged from before P15.
    let is_scout = colony.provisional_tiles.contains_key(cat_id);

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
            if is_scout {
                // Tentative: not yet committed to the permanent map. Skip tiles the
                // colony already knows — nothing new to hold provisionally.
                if !colony.revealed_tiles.contains(&pos) {
                    colony
                        .provisional_tiles
                        .entry(cat_id.to_owned())
                        .or_default()
                        .insert(pos);
                }
            } else {
                colony.revealed_tiles.insert(pos);
            }
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

/// P15: deliver a scout's tentative discoveries — folds `cat_id`'s
/// `provisional_tiles` entry (if any) into the permanent `revealed_tiles` set and
/// clears the entry. A no-op for any cat that never had one (everyone but a scout
/// returning from an `Explore` job). Idempotent: committing twice, or committing an
/// entry that was never created, does nothing on the second call.
fn commit_scout_provisional_tiles(colony: &mut ColonyRuntime, cat_id: &str) {
    if let Some(tiles) = colony.provisional_tiles.remove(cat_id) {
        colony.revealed_tiles.extend(tiles);
    }
}

fn scaffold_building_type(building_type: BuildingType) -> BuildingType {
    match building_type {
        BuildingType::Workshop
        | BuildingType::Field
        | BuildingType::FoodStorage
        | BuildingType::Smithy
        | BuildingType::Barracks
        | BuildingType::AccountingTent
        | BuildingType::Clothier
        | BuildingType::Tannery
        | BuildingType::ResearchHut
        | BuildingType::School
        | BuildingType::Smelter => building_type,
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
                && (!require_farmable
                    || tile_is_farmable(world_seed, tile, colony.world_tiles.get(&tile)))
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

/// Whether a field/farm may be sown on this ground (P17: relaxed from
/// "grass/meadow/marsh only" to any farmable biome). A tile is farmable when
/// either of two independent signals says so: the legacy coarse ground type
/// (`Field`/`Meadow`/`Swamp`, the original P16 heuristic, kept so no
/// previously-valid site is ever lost) **or** the fine per-tile P17 climate
/// biome at this position has positive fertility (`BiomeClimate::farmable`,
/// e.g. `Plains`/`Meadow`/`FlowerField`/`Marsh`/`Swamp`/`Savanna`/`Jungle`/
/// `Hills`/`MushroomFields`). Either way water always wins: a tile carrying a
/// river (including the synthetic starter pond stamped after generation, which
/// the fine classifier never sees) is never farmable, and an unrevealed/absent
/// tile is never farmable.
fn tile_is_farmable(world_seed: u32, pos: TilePos, tile: Option<&WorldTileRuntime>) -> bool {
    tile.is_some_and(|tile| {
        !tile_has_water(Some(tile))
            && (matches!(
                tile.tile_type,
                TileType::Field | TileType::Meadow | TileType::Swamp
            ) || crate::terrain_gen::tile_climate_biome(world_seed, pos.x, pos.y)
                .properties()
                .farmable())
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
        JobMetadata::GatherHaul { site: Some(_), .. } => true,
        JobMetadata::Construction { site: Some(_), .. } => job.kind == JobKind::BuildHouse,
        _ => false,
    }
}

/// Reduce the food-tile candidates to a single zone-weighted pick, mirroring the TS
/// `pickTargetWithZones` hunt path. Gather tiles are twice as likely and avoid tiles are
/// excluded (unless every candidate is avoided, the critical fallback). With no zones this
/// returns the same tile a uniform `floor(roll * len)` index would, so hunt targeting stays
/// byte-identical when no player has painted a zone.
fn pick_zoned_food_tile(food_tiles: &[WorldPos], zones: &[Zone], roll: f64) -> Option<WorldPos> {
    let zone_tiles = food_tiles
        .iter()
        .map(|tile| ZonePos {
            x: tile.x.round() as i32,
            y: tile.y.round() as i32,
        })
        .collect::<Vec<_>>();
    let picked = pick_target_with_zones(&zone_tiles, zones, roll)?;
    let index = zone_tiles
        .iter()
        .position(|zone_tile| *zone_tile == picked)?;
    food_tiles.get(index).copied()
}

fn food_tiles_near_village(colony: &ColonyRuntime) -> Vec<WorldPos> {
    colony
        .world_tiles
        .values()
        .filter(|tile| {
            tile.resources.food >= 25
                && tile_is_explored(colony.anchor, tile)
                && cheb_from_anchor(colony.anchor, tile.pos) > 4
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
                && tile_is_explored(colony.anchor, tile)
        })
        .map(|tile| tile.pos)
        .collect::<Vec<_>>();
    sites.sort_by_key(|site| cheb_from_anchor(colony.anchor, *site));
    sites.into_iter().map(tile_pos_to_world).collect()
}

fn water_sites_near_village(colony: &ColonyRuntime) -> Vec<WorldPos> {
    let mut sites = colony
        .world_tiles
        .values()
        .filter(|tile| tile_has_water(Some(tile)) && tile_is_explored(colony.anchor, tile))
        .map(|tile| tile.pos)
        .collect::<Vec<_>>();
    sites.sort_by_key(|site| cheb_from_anchor(colony.anchor, *site));
    sites.into_iter().map(tile_pos_to_world).collect()
}

fn tile_is_explored(anchor: TilePos, tile: &WorldTileRuntime) -> bool {
    tile.path_wear > 62 || cheb_from_anchor(anchor, tile.pos) <= 6
}

fn cheb_from_anchor(anchor: TilePos, pos: TilePos) -> i32 {
    (pos.x - anchor.x).abs().max((pos.y - anchor.y).abs())
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

/// Highest-first staffing order for the founding raw-material chain. Balance
/// planks/blocks until one tools cycle can run without crossing the 4/4 build
/// reserve, then put woodworking first. Shared by phase 20's director assignment
/// and phase 23's idle mop-up so they cannot fight each other.
fn raw_material_staffing_priority(resources: &Resources) -> [BuildingType; 3] {
    let (scarcer, other) = if resources.planks <= resources.blocks {
        (BuildingType::WoodCutter, BuildingType::StonePrep)
    } else {
        (BuildingType::StonePrep, BuildingType::WoodCutter)
    };
    let tools_cycle_funded = resources.planks
        >= TOOL_BUILD_MATERIAL_RESERVE + crate::production::WOODWORKING_PLANKS_PER_CYCLE
        && resources.blocks
            >= TOOL_BUILD_MATERIAL_RESERVE + crate::production::WOODWORKING_BLOCKS_PER_CYCLE;
    if tools_cycle_funded {
        [BuildingType::Woodworking, scarcer, other]
    } else {
        [scarcer, other, BuildingType::Woodworking]
    }
}

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

/// Stage-weighted count of cats actively staffing a completed research building
/// (research hut or school — both contribute identically). Each staffed building
/// contributes its assigned cat's [`life_sim::workforce_weight`] — the same
/// weight the labour budget uses — so an elder scholar researches at 0.7 like every other
/// job, and a dead/absent assignee contributes nothing. This is the sole input to
/// [`phase_24_research`]'s point accrual; a colony with no staffed hut/school yields 0.0 and
/// is byte-identical to the pre-wiring behaviour.
fn research_workforce(colony: &ColonyRuntime) -> f64 {
    colony
        .buildings
        .iter()
        .filter(|building| {
            matches!(
                building.building_type,
                BuildingType::ResearchHut | BuildingType::School
            ) && building.construction_progress >= 100
        })
        .filter_map(|building| building.assigned_cat.as_deref())
        .filter_map(|cat_id| {
            colony
                .cats
                .iter()
                .find(|cat| cat.id == cat_id && cat.death_time.is_none())
        })
        .filter(|cat| can_work(get_life_stage(cat.age_hours)))
        .map(|cat| crate::life_sim::workforce_weight(get_life_stage(cat.age_hours)))
        .sum()
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

/// Raw hide credited per unit of hunt food reward (P16/P19 clothing chain slice) — a
/// direct byproduct, not hauled: hunts already haul their food reward via
/// `Carrying` (which only ever carries one resource kind at a time), so rather than
/// extend the hauling/trips model for a second resource, hide is simply credited to
/// `colony.resources.hide` the moment a hunt completes. Keeps the tannery fed without
/// touching the trip-based hauling system (out of scope here). Small on purpose —
/// hide is a byproduct of hunting, not hunting's point.
const HUNT_HIDE_YIELD_RATIO: f64 = 0.2;

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
    if reward > 0.0 {
        colony.resources.hide += reward * HUNT_HIDE_YIELD_RATIO;
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
        source_gather_spot: None,
    });
}

fn complete_fixed_yield_job(
    colony: &mut ColonyRuntime,
    job: &JobRuntime,
    gate: TickGate,
    total: f64,
    kind: CarryingKind,
    world_seed: u32,
) {
    let Some(cat_index) = assigned_alive_cat_index(colony, job) else {
        return;
    };
    let (site, total_yield, trips_done) = hauling_metadata(job);
    // Reuse the total cached by an earlier haul trip so skill scaling is applied once;
    // otherwise scale the base constant by this cat's labor skill here.
    let scaled_total =
        total_yield.unwrap_or_else(|| skill_scaled_yield(&colony.cats[cat_index], job.kind, total));
    let reward = remaining_yield(scaled_total, HUNT_TRIP_COUNT, trips_done as i32);
    if job.kind == JobKind::Quarry {
        credit_quarry_ore(colony, site, reward, world_seed);
    }
    let cat = &mut colony.cats[cat_index];
    if let Some(labor) = Labor::for_job_kind(job.kind) {
        cat.gain_skill(labor, SKILL_GAIN_PER_JOB);
    }
    cat.carrying = (reward > 0.0).then_some(Carrying {
        kind,
        amount: reward,
        job_ended_at: gate.processed_through,
        source_gather_spot: None,
    });
}

/// Fraction of a completed quarry's materials reward that also comes back as raw ore,
/// when (and only when) the quarry site sits on a genuine Mountain biome
/// ([`Mining::Full`] — never the stony-shore/rocky [`Mining::Trickle`] tiles). Mirrors
/// [`HUNT_HIDE_YIELD_RATIO`]'s "byproduct credited directly to `resources`, not hauled
/// separately" shape (P17/P19 ore→metal chain).
///
/// Checked once per completed quarry job, not per tick/per tile:
/// [`tile_climate_biome`] regenerates a whole terrain chunk per call, so calling it from
/// a hot per-tick or per-tile path would be a real perf hazard — a single lookup at job
/// completion (a quarry job takes minutes) is cheap. Reaching a mountain quarry site at
/// all already requires the `mountaineering` upgrade node (mountain tiles are otherwise
/// impassable to pathfinding), so ore implicitly gates on that unlock too. A colony that
/// never reaches the mountains never has a quarry site with `Mining::Full`, so
/// `resources.ore` simply never moves off zero — additive/inert.
const QUARRY_ORE_YIELD_RATIO: f64 = 0.3;

fn credit_quarry_ore(
    colony: &mut ColonyRuntime,
    site: Option<TilePos>,
    reward: f64,
    world_seed: u32,
) {
    if reward <= 0.0 {
        return;
    }
    let Some(site) = site else {
        return;
    };
    if tile_climate_biome(world_seed, site.x, site.y)
        .properties()
        .mining
        == Mining::Full
    {
        colony.resources.ore += reward * QUARRY_ORE_YIELD_RATIO;
    }
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
        EventKind::VillageExpanded,
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
        source_gather_spot: None,
    });
}

/// Completes a `carry_offering` job (P12.6). Unlike [`complete_ritual`] (pure cat
/// labor, no resource cost), an offering consumes a genuine hauled-and-banked
/// surplus — [`OFFERING_MATERIALS_AMOUNT`] materials — converting it to blessings
/// at the shrine. It reuses the Ritual->blessing carry/credit path unchanged
/// (`CarryingKind::Blessings` -> the shared shrine-arrival `credit_carrying` ->
/// `global_upgrade_points`), so no new shrine machinery is needed; only the
/// resource draw is new. It draws only from `materials`, a resource `Tithe` never
/// touches (Tithe only spends food/refined), so the two blessing faucets can never
/// double-count the same surplus pool.
///
/// Defensively re-checks the balance here (not just at dispatch time in
/// `direct_colony`): materials can be spent by other systems (a build, a craft
/// bench) during the job's travel+duration, so the surplus that justified
/// dispatch may have evaporated by completion. When that happens this performs no
/// offering and produces no blessings — the cat simply returns empty-handed.
/// Materials are never survival-critical (unlike food/water), so this can never
/// starve the colony either way.
fn complete_offering(colony: &mut ColonyRuntime, job: &JobRuntime, gate: TickGate) {
    let Some(cat_index) = assigned_alive_cat_index(colony, job) else {
        return;
    };
    if colony.resources.materials
        < OFFERING_MATERIALS_RESERVE + f64::from(OFFERING_MATERIALS_AMOUNT)
    {
        return;
    }
    colony.resources.materials -= f64::from(OFFERING_MATERIALS_AMOUNT);
    let blessings = 1.0 + f64::from(colony.upgrade_levels.ritual_mastery / 3);
    let cat_id = colony.cats[cat_index].id.clone();
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
        source_gather_spot: None,
    });
    append_event(
        colony,
        gate.processed_through,
        EventKind::Offering,
        format!(
            "{cat_id} offered {OFFERING_MATERIALS_AMOUNT} materials at the shrine for blessings."
        ),
    );
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
        pick_wander_target(
            village_anchor_world(colony.anchor),
            first.value,
            second.value,
        )
    } else if matches!(
        job.kind,
        JobKind::HuntExpedition | JobKind::Quarry | JobKind::FetchWater
    ) {
        // The completing gatherer carries its final trip's yield home; route it to the pile
        // that yield belongs in (nearest designated pile accepting it), falling back to the
        // shrine anchor when none does — byte-identical with no designated piles.
        let from = position_to_world(colony.anchor, colony.cats[cat_index].position);
        haul_destination(colony, carrying_kind_for_job(job.kind), from)
    } else {
        village_anchor_world(colony.anchor)
    };
    let cat = &mut colony.cats[cat_index];
    cat.destination = Some(position_from_world(destination));
    cat.activity = CatActivity::Returning;
    cat.current_task = None;
    if job.kind == JobKind::TrainWarrior {
        append_event(
            colony,
            gate.processed_through,
            EventKind::WarriorTrained,
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

fn total_yield_for_job(
    colony: &ColonyRuntime,
    job: &JobRuntime,
    cat_index: usize,
    world_seed: u32,
) -> f64 {
    let cat = &colony.cats[cat_index];
    match job.kind {
        JobKind::FetchWater => skill_scaled_yield(cat, job.kind, WATER_TOTAL_YIELD),
        // P17 mining rules: `QUARRY_TOTAL_YIELD` is the "full mountain" base yield,
        // scaled by the site's mining rule (`quarry_yield_multiplier`).
        JobKind::Quarry => {
            let mult = hauling_metadata(job).0.map_or(1.0, |site| {
                quarry_yield_multiplier(colony, world_seed, site)
            });
            (skill_scaled_yield(cat, job.kind, QUARRY_TOTAL_YIELD) * mult).floor()
        }
        JobKind::HuntExpedition => hunt_yield_for(cat, colony),
        _ => 0.0,
    }
}

/// P17 mining-yield multiplier for a quarry `site`, the OR/superset of two signals
/// (mirrors `tile_is_farmable`): the legacy coarse ground type
/// (`Mountains`/`CaveEntrance` — the same signal `quarry_sites_near_village` uses
/// to *discover* a site, so a discovered site always mines fully and the founding
/// material economy never regresses) OR the fine per-tile climate biome's mining
/// rule (`Full` mountains, `Trickle` stony/gravel, `None` elsewhere). Taking the
/// max means a legacy mountain is always full even when the fine biome under it is
/// non-mining (e.g. a `Mountains` tile carved near the founding grass plateau),
/// while a fine stony biome that isn't a legacy mountain still yields its trickle.
fn quarry_yield_multiplier(colony: &ColonyRuntime, world_seed: u32, site: TilePos) -> f64 {
    let legacy = colony.world_tiles.get(&site).map_or(0.0, |tile| {
        if matches!(tile.tile_type, TileType::Mountains | TileType::CaveEntrance) {
            crate::climate::Mining::Full.yield_multiplier()
        } else {
            0.0
        }
    });
    let fine = crate::terrain_gen::tile_climate_biome(world_seed, site.x, site.y)
        .properties()
        .mining
        .yield_multiplier();
    legacy.max(fine)
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
        .filter(|tile| is_forest_type(tile.tile_type) && tile_is_explored(colony.anchor, tile))
        .map(|tile| tile.pos)
        .min_by_key(|site| cheb_from_anchor(colony.anchor, *site));
    if let Some(site) = nearest {
        clear_claimed_forest_tile(colony, site, now_ms);
        append_event(
            colony,
            now_ms,
            EventKind::ForestChopped,
            format!(
                "A forest at ({}, {}) was chopped for lumber.",
                site.x, site.y
            ),
        );
    }
}

/// The shrine reservoir's footprint at a colony's village anchor.
fn shrine_stockpile_rect(anchor: TilePos) -> ZoneRect {
    stockpiles::shrine_rect(anchor.x, anchor.y)
}

/// Restore the stockpile balancing-reservoir invariant for a colony (seeds the shrine
/// reservoir if absent). Runs at the end of every tick and after stockpile actions.
pub fn reconcile_colony_stockpiles(colony: &mut ColonyRuntime) {
    stockpiles::reconcile(
        &mut colony.stockpiles,
        &colony.resources,
        shrine_stockpile_rect(colony.anchor),
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
        return village_anchor_world(colony.anchor);
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
        None => village_anchor_world(colony.anchor),
    }
}

/// Where a carrier's cargo is ultimately bound right now — the exact target the deposit
/// phase (phase 33) computes to decide whether a haul is done. Mirrors its two branches: a
/// P16 mover's cargo (picked up from a gather spot, `source_gather_spot: Some(_)`) must
/// land in a genuine village pile/shrine, never back into a gather spot, so it uses
/// [`stockpiles::village_deposit_index`]; every other carrier (a gatherer's own haul, or
/// blessings) uses the ordinary [`haul_destination`] rule. Pulled out as its own function
/// so the building-inbound-haul projection ([`building_inbound_haul`]) can ask the
/// identical question the deposit phase asks, without re-deriving the branch.
fn haul_deposit_target(
    colony: &ColonyRuntime,
    carrying: &Carrying,
    from_pos: WorldPos,
) -> WorldPos {
    if carrying.source_gather_spot.is_some() {
        let gather_spot_ids: Vec<String> = colony
            .gather_spots
            .iter()
            .map(|spot| spot.stockpile_id.clone())
            .collect();
        let kind = carrying_resource_kind(carrying.kind).unwrap_or(ResourceKind::Food);
        stockpiles::village_deposit_index(
            &colony.stockpiles,
            &gather_spot_ids,
            kind,
            from_pos.x,
            from_pos.y,
        )
        .map_or_else(
            || village_anchor_world(colony.anchor),
            |idx| {
                let (cx, cy) = colony.stockpiles[idx].center();
                WorldPos { x: cx, y: cy }
            },
        )
    } else {
        haul_destination(colony, carrying.kind, from_pos)
    }
}

/// Resource units physically in flight toward `building` right now: the live sum of
/// `Carrying.amount` for every living cat whose haul deposit target — the exact rule
/// [`haul_deposit_target`] uses, the same one the deposit phase checks before crediting a
/// haul — resolves to this building's tile.
///
/// Only ever nonzero for the shrine today. `haul_deposit_target` only ever resolves to a
/// *stockpile*: a player-designated pile or a P16 gather spot, both plain rects held in
/// `ColonyRuntime.stockpiles`/`gather_spots` with no building entity of their own — or the
/// shrine anchor fallback, which coincides with the shrine building's placement (both
/// anchored at [`village_layout::VILLAGE_ANCHOR`]). No other `BuildingType` is ever a haul
/// target: production buildings (workshop, wood-cutter, smithy, ...) draw their inputs
/// straight from `colony.resources` each tick, not from physically delivered cargo (see
/// `phase_23_production`). Matched by exact position rather than hardcoding
/// `BuildingType::Shrine` so a future haul target would pick this up automatically.
pub fn building_inbound_haul(colony: &ColonyRuntime, building: &BuildingRuntime) -> f64 {
    let building_pos = tile_pos_to_world(building.position);
    colony
        .cats
        .iter()
        .filter(|cat| cat.death_time.is_none())
        .filter_map(|cat| {
            cat.carrying
                .as_ref()
                .map(|carrying| (cat.position, carrying))
        })
        .filter(|(position, carrying)| {
            haul_deposit_target(
                colony,
                carrying,
                position_to_world(colony.anchor, *position),
            ) == building_pos
        })
        .map(|(_, carrying)| carrying.amount)
        .sum()
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

    // P16 mover arrival: this cargo was already counted in `resources` the moment the
    // gatherer first dropped it into the gather spot, so this is a pure pile-to-pile
    // transfer — never re-add to `resources` (that would double-count it) — and the
    // destination must be a genuine village pile, never back into a gather spot
    // (`village_deposit_index` excludes them).
    if carrying.source_gather_spot.is_some() {
        let gather_spot_ids: Vec<String> = colony
            .gather_spots
            .iter()
            .map(|spot| spot.stockpile_id.clone())
            .collect();
        if let Some(idx) = stockpiles::village_deposit_index(
            &colony.stockpiles,
            &gather_spot_ids,
            kind,
            deposit_at.x,
            deposit_at.y,
        ) {
            stockpiles::add_resource(&mut colony.stockpiles[idx].contents, kind, carrying.amount);
        }
        return;
    }

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
        items::{ItemKind, MAX_QUALITY, Material},
        storage::BASE_CAPACITY,
    };

    #[test]
    fn colony_runtime_items_default_to_an_empty_store() {
        let colony = ColonyRuntime::default();
        assert!(colony.items.is_empty());
    }

    #[test]
    fn colony_runtime_add_and_remove_item_delegate_to_the_items_store() {
        use crate::items::{ItemKind, Material};

        let mut colony = ColonyRuntime::default();
        let mug = Item::new(ItemKind::Mug, Material::Wood, 1);

        colony.add_item(mug, 3);
        colony.add_item(mug, 2);
        assert_eq!(colony.items.get(&mug), Some(&5));

        assert!(!colony.remove_item(mug, 10), "insufficient count fails");
        assert_eq!(
            colony.items.get(&mug),
            Some(&5),
            "store untouched on failure"
        );

        assert!(colony.remove_item(mug, 5));
        assert!(!colony.items.contains_key(&mug), "entry dropped at zero");
    }

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
                    fibre: 0.0,
                    hide: 0.0,
                    cloth: 0.0,
                    leather: 0.0,
                    ore: 0.0,
                    metal: 0.0,
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
                    fibre: 0.0,
                    hide: 0.0,
                    cloth: 0.0,
                    leather: 0.0,
                    ore: 0.0,
                    metal: 0.0,
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
                source_gather_spot: None,
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

    /// Builds a single quarry job assigned to `cat_id`, hauling from `site`, ready
    /// for its first trip's total-yield computation.
    fn quarry_job_at(site: TilePos, cat_id: &str) -> JobRuntime {
        JobRuntime {
            id: "quarry-1".to_owned(),
            kind: JobKind::Quarry,
            status: JobStatus::Active,
            requested_by: JobRequester::Leader,
            assigned_cat: Some(cat_id.to_owned()),
            duration_ms: 1_000,
            speed: 1.0,
            yield_amount: 0.0,
            click_count: 0,
            created_at: 0,
            started_at: Some(0),
            ends_at: Some(1_000),
            completed_at: None,
            metadata: JobMetadata::Hauling {
                site: Some(site),
                total_yield: None,
                trips_done: 0,
                next_trip_at: None,
                accepted: true,
            },
        }
    }

    #[test]
    fn quarry_yield_is_biome_gated_full_on_mountain_trickle_on_stony_none_elsewhere() {
        // P17 mining rules, wired end to end through `total_yield_for_job`'s
        // `Quarry` arm: three real sites for seed 42, one per `climate::Mining`
        // rule (found by scanning `tile_climate_biome`, not hardcoded — biome
        // placement is seed noise).
        let seed = 42u32;
        let full_site = pos(200, 573);
        let trickle_site = pos(200, 0);
        let none_site = pos(0, 0);
        assert_eq!(
            crate::terrain_gen::tile_climate_biome(seed, full_site.x, full_site.y)
                .properties()
                .mining,
            crate::climate::Mining::Full,
            "test fixture assumption: full_site mines fully"
        );
        assert_eq!(
            crate::terrain_gen::tile_climate_biome(seed, trickle_site.x, trickle_site.y)
                .properties()
                .mining,
            crate::climate::Mining::Trickle,
            "test fixture assumption: trickle_site gives a trickle"
        );
        assert_eq!(
            crate::terrain_gen::tile_climate_biome(seed, none_site.x, none_site.y)
                .properties()
                .mining,
            crate::climate::Mining::None,
            "test fixture assumption: none_site has no mining rule"
        );

        let colony = ColonyRuntime {
            cats: vec![adult_idle_cat("miner", "colony-1")],
            ..ColonyRuntime::default()
        };

        let full_yield = total_yield_for_job(&colony, &quarry_job_at(full_site, "miner"), 0, seed);
        let trickle_yield =
            total_yield_for_job(&colony, &quarry_job_at(trickle_site, "miner"), 0, seed);
        let none_yield = total_yield_for_job(&colony, &quarry_job_at(none_site, "miner"), 0, seed);

        assert_eq!(
            full_yield, QUARRY_TOTAL_YIELD,
            "a mountain site keeps the full base yield"
        );
        assert!(
            trickle_yield > 0.0 && trickle_yield < full_yield,
            "a stony site should give a nonzero trickle below the full yield, got {trickle_yield}"
        );
        assert_eq!(none_yield, 0.0, "a non-mining biome yields nothing");

        // Deterministic: re-running the same site produces byte-identical yield.
        let twin_yield =
            total_yield_for_job(&colony, &quarry_job_at(trickle_site, "miner"), 0, seed);
        assert_eq!(trickle_yield.to_bits(), twin_yield.to_bits());
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
            anchor: VILLAGE_ANCHOR_TILE,
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

    /// Reachability guard: threat pressure crossing the threshold fires a raid.
    ///
    /// Past the grace window with pressure already at the spawn threshold, the tick's
    /// threat director must launch a warband — `active_raid` set and raiders spawned on
    /// the map. Guards the spawn wiring between [`accrue_threat`]/[`should_spawn_raid`]
    /// and [`spawn_raid`] end to end (the pure pieces are unit-tested in `threat.rs`,
    /// but nothing else asserts the tick actually stands a warband up).
    #[test]
    fn threat_pressure_crossing_the_threshold_launches_a_warband() {
        let grace_ms = (crate::threat::RAID_GRACE_SEC * 1000.0) as i64;
        let now = grace_ms + 10 * 60_000; // comfortably past the 8h grace window
        let mut world = WorldState {
            world_seed: 123,
            colonies: vec![ColonyRuntime {
                id: "colony-1".to_owned(),
                resources: plentiful_resources(),
                cats: vec![
                    adult_idle_cat("cat-1", "colony-1"),
                    adult_idle_cat("cat-2", "colony-1"),
                ],
                run_started_at: 0,
                created_at: 0,
                last_tick: now - 60_000,
                // Already at the spawn threshold: this tick's accrual must trip a raid.
                threat_pressure: crate::threat::RAID_SPAWN_THRESHOLD,
                test_rng_seed: Some(777),
                ..ColonyRuntime::default()
            }],
        };

        let reports = world_tick(&mut world, now);
        assert_eq!(reports[0].reset_reason, None);
        let colony = &world.colonies[0];
        assert!(
            colony.active_raid.is_some(),
            "pressure at the threshold past grace should launch a raid"
        );
        assert!(
            !colony.raiders.is_empty(),
            "a launched raid must put raiders on the map"
        );
        // Pressure resets when the warband launches (see `spawn_raid`).
        assert_eq!(colony.threat_pressure, 0.0);
    }

    /// Military balance + reachability guard: gear and warrior training decide raids.
    ///
    /// Two colonies face an identical warband already at the gate (same seed, so
    /// [`resolve_raid`]'s swing roll is identical). Both muster the *same three base
    /// cats*; the only difference is that the "prepared" colony trained them into
    /// warriors and stocked weapons + armor for the smithy chain to draw at raid time.
    /// The prepared colony must drive the raid off with no losses; the unprepared
    /// militia must be overrun and looted. This is the executable proof that
    /// `muster_defense` draws gear, that the weapon/armor bonuses and warrior combat
    /// factor actually move the combat outcome, and that raids are winnable *with*
    /// preparation and losable *without* it.
    #[test]
    fn prepared_colony_repels_the_raid_that_overruns_an_unprepared_one() {
        fn base_colony() -> ColonyRuntime {
            ColonyRuntime {
                id: "colony-1".to_owned(),
                resources: Resources {
                    food: 100.0,
                    water: 100.0,
                    materials: 20.0,
                    ..Resources::default()
                },
                run_started_at: 0,
                created_at: 0,
                last_tick: 0,
                test_rng_seed: Some(12_345),
                ..ColonyRuntime::default()
            }
        }

        fn defender(id: &str) -> Cat {
            let mut cat = adult_idle_cat(id, "colony-1");
            cat.stats.attack = 50.0;
            cat.stats.defense = 50.0;
            cat
        }

        // A three-raider warband already standing at the gate: 40 hp each = 120 raider
        // power. That sits above the unprepared militia's muster (3 x (50+50) x 0.28 =
        // 84, at most 84 x 1.25 = 105) and far below the armed-warrior muster
        // (3 x (75+75) x 1.0 = 450, at least 450 x 0.75 = 337) for every possible swing.
        fn warband(colony: &ColonyRuntime) -> Vec<RaiderRuntime> {
            (0..3)
                .map(|index| RaiderRuntime {
                    id: format!("raid-1-raider-{}", index + 1),
                    raid_id: "raid-1".to_owned(),
                    position: position_from_world(tile_pos_to_world(raid_gate_position(colony))),
                    destination: None,
                    attack: 40.0,
                    defense: 40.0,
                    health: 40.0,
                })
                .collect()
        }

        // Unprepared: three bare militia, no gear in store.
        let mut unprepared = base_colony();
        unprepared.cats = vec![defender("u1"), defender("u2"), defender("u3")];
        unprepared.raiders = warband(&unprepared);
        unprepared.active_raid = Some("raid-1".to_owned());

        // Prepared: the same three cats trained into warriors, with weapons + armor
        // stocked for the muster to draw.
        let mut prepared = base_colony();
        prepared.resources.weapons = 5.0;
        prepared.resources.armor = 5.0;
        prepared.cats = vec![defender("p1"), defender("p2"), defender("p3")];
        for cat in &mut prepared.cats {
            cat.specialization = Some(CatSpecialization::Warrior);
        }
        prepared.raiders = warband(&prepared);
        prepared.active_raid = Some("raid-1".to_owned());

        let mut unprepared_world = WorldState {
            world_seed: 123,
            colonies: vec![unprepared],
        };
        let mut prepared_world = WorldState {
            world_seed: 123,
            colonies: vec![prepared],
        };
        let _ = world_tick(&mut unprepared_world, 1_000);
        let _ = world_tick(&mut prepared_world, 1_000);

        // Unprepared militia are overrun: raid ends, stores are looted, gear stays zero.
        let unprepared = &unprepared_world.colonies[0];
        assert!(unprepared.active_raid.is_none(), "raid never resolved");
        assert!(
            unprepared.resources.food <= 90.0,
            "unprepared colony should be looted (>=10% of stores): food {}",
            unprepared.resources.food
        );

        // Prepared warriors hold the line: raid ends, nothing is looted, and the drawn
        // weapons/armor are consumed from the armory.
        let prepared = &prepared_world.colonies[0];
        assert!(prepared.active_raid.is_none(), "raid never resolved");
        assert!(
            prepared.resources.food >= 99.0,
            "prepared colony held the gate and should not be looted (only tick consumption): food {}",
            prepared.resources.food
        );
        assert!(
            prepared.resources.weapons < 5.0 && prepared.resources.armor < 5.0,
            "prepared muster should draw weapons + armor (weapons {}, armor {})",
            prepared.resources.weapons,
            prepared.resources.armor,
        );
        assert!(
            alive_cats(&prepared.cats).count() == 3,
            "prepared colony should take no casualties"
        );
        assert!(
            prepared
                .cats
                .iter()
                .all(|cat| cat.role_xp.warrior >= WARRIOR_XP_PER_RAID),
            "surviving warriors should earn raid XP for holding the line"
        );
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
                    fibre: 0.0,
                    hide: 0.0,
                    cloth: 0.0,
                    leather: 0.0,
                    ore: 0.0,
                    metal: 0.0,
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

    /// TS-parity guardrail (`server/game.ts` `ensureColony` re-seeded starter cats
    /// whenever no cat was alive): a fully-extinct colony must come back as a fresh
    /// founding roster on the reset tick — and must NOT reset again on the next tick.
    /// Pre-fix, a dead colony fired `AllCatsDead` every tick forever (instrumented:
    /// seed 99 went extinct at game-hour ~114 and banked 895 back-to-back resets over
    /// the rest of a 200-game-hour run with the world permanently empty).
    #[test]
    fn extinct_colony_respawns_a_fresh_starter_roster_and_stops_reset_storming() {
        let run = || {
            let mut world = new_world(99);
            world
                .colonies
                .push(found_colony(world.world_seed, "colony-1", 10_000, 99));
            for cat in &mut world.colonies[0].cats {
                cat.death_time = Some(20_000);
            }

            let first = world_tick(&mut world, 70_000);
            assert_eq!(
                first[0].reset_reason,
                Some(RunResetReason::AllCatsDead),
                "an extinct roster must trip the all-dead reset"
            );
            let alive: Vec<(String, String, f64)> = alive_cats(&world.colonies[0].cats)
                .map(|cat| (cat.id.clone(), cat.name.clone(), cat.stats.hunting))
                .collect();
            assert_eq!(
                alive.len(),
                STARTER_CAT_COUNT,
                "the reset must respawn a full founding roster"
            );
            for (id, _, _) in &alive {
                assert!(
                    id.contains("-run2-"),
                    "respawned ids must be run-scoped (the founding ids live on in the \
                     graveyard), got {id}"
                );
            }
            assert!(
                world.colonies[0].leader_id.is_some(),
                "the respawned roster must seat an interim leader"
            );

            let second = world_tick(&mut world, 130_000);
            assert_eq!(
                second[0].reset_reason, None,
                "a repopulated colony must not keep reset-storming"
            );
            alive
        };
        // Determinism twin folded in: identical trajectories respawn identical rosters.
        assert_eq!(run(), run());
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
        // shrine-only baseline every tick. This is a bookkeeping invariant, not a
        // pathing one, so the pile's rect is centred exactly on the village anchor
        // (5,5)-(7,7) → centre (6,6), matching `village_anchor_world(VILLAGE_ANCHOR_TILE)` bit-for-bit
        // (rather than a `(6,6)-(7,7)` rect, whose 6.5,6.5 centre rounds to a
        // different goal tile than the shrine baseline's (6,6) — since P14.2 made
        // building footprints real soft obstacles, routing to two different tiles
        // can legitimately take a different number of ticks even when both sit on
        // the shrine hub, which would make this test about pathing, not bookkeeping).
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
                x1: 5,
                y1: 5,
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
            anchor: VILLAGE_ANCHOR_TILE,
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
            source_gather_spot: None,
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
                village_anchor_world(VILLAGE_ANCHOR_TILE)
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
            village_anchor_world(VILLAGE_ANCHOR_TILE)
        );
        // Blessings fund the global pool (never piled), so they always fall back to the anchor.
        assert_eq!(
            haul_destination(
                &colony,
                CarryingKind::Blessings,
                WorldPos { x: 9.0, y: 6.0 }
            ),
            village_anchor_world(VILLAGE_ANCHOR_TILE)
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

    // ---- P15 building-snapshot inbound haul ----

    #[test]
    fn building_inbound_haul_reflects_a_carrier_bound_for_the_shrine() {
        // No designated pile accepts food, so the hauler's cargo resolves to the shrine
        // anchor regardless of how far away it currently stands — the shrine building's
        // inbound_haul should reflect that live cargo.
        let anchor = village_anchor_world(VILLAGE_ANCHOR_TILE);
        let colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            cats: vec![carrying_cat_at(
                "hauler",
                CarryingKind::Food,
                8.0,
                WorldPos { x: 40.0, y: 3.0 },
            )],
            buildings: vec![BuildingRuntime {
                id: "shrine".to_owned(),
                building_type: BuildingType::Shrine,
                position: TilePos {
                    x: anchor.x as i32,
                    y: anchor.y as i32,
                },
                ..BuildingRuntime::default()
            }],
            ..ColonyRuntime::default()
        };

        let shrine = &colony.buildings[0];
        assert_eq!(building_inbound_haul(&colony, shrine), 8.0);
    }

    #[test]
    fn building_inbound_haul_sums_every_live_carrier_bound_for_it() {
        let anchor = village_anchor_world(VILLAGE_ANCHOR_TILE);
        let colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            cats: vec![
                carrying_cat_at(
                    "hauler-1",
                    CarryingKind::Food,
                    8.0,
                    WorldPos { x: 40.0, y: 3.0 },
                ),
                carrying_cat_at(
                    "hauler-2",
                    CarryingKind::Water,
                    3.0,
                    WorldPos { x: 2.0, y: 40.0 },
                ),
            ],
            buildings: vec![BuildingRuntime {
                id: "shrine".to_owned(),
                building_type: BuildingType::Shrine,
                position: TilePos {
                    x: anchor.x as i32,
                    y: anchor.y as i32,
                },
                ..BuildingRuntime::default()
            }],
            ..ColonyRuntime::default()
        };

        let shrine = &colony.buildings[0];
        assert_eq!(building_inbound_haul(&colony, shrine), 11.0);
    }

    #[test]
    fn building_inbound_haul_is_zero_for_a_building_that_never_receives_hauls() {
        // The hauler's cargo resolves to the shrine anchor, never to this unrelated den —
        // no building type other than the shrine is ever a haul target.
        let colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            cats: vec![carrying_cat_at(
                "hauler",
                CarryingKind::Food,
                8.0,
                WorldPos { x: 40.0, y: 3.0 },
            )],
            buildings: vec![BuildingRuntime {
                id: "den".to_owned(),
                building_type: BuildingType::Den,
                position: TilePos { x: 20, y: 20 },
                ..BuildingRuntime::default()
            }],
            ..ColonyRuntime::default()
        };

        let den = &colony.buildings[0];
        assert_eq!(building_inbound_haul(&colony, den), 0.0);
    }

    #[test]
    fn building_inbound_haul_ignores_a_dead_carrier_and_cargo_bound_elsewhere() {
        let anchor = village_anchor_world(VILLAGE_ANCHOR_TILE);
        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            cats: vec![
                // Dead cats never deposit, so their stale `carrying` must not count.
                {
                    let mut dead = carrying_cat_at(
                        "ghost",
                        CarryingKind::Food,
                        99.0,
                        WorldPos { x: 40.0, y: 3.0 },
                    );
                    dead.death_time = Some(1);
                    dead
                },
                // This hauler's cargo is bound for its designated pile, not the shrine.
                carrying_cat_at(
                    "piler",
                    CarryingKind::Food,
                    5.0,
                    WorldPos { x: 10.0, y: 6.0 },
                ),
            ],
            buildings: vec![BuildingRuntime {
                id: "shrine".to_owned(),
                building_type: BuildingType::Shrine,
                position: TilePos {
                    x: anchor.x as i32,
                    y: anchor.y as i32,
                },
                ..BuildingRuntime::default()
            }],
            ..ColonyRuntime::default()
        };
        reconcile_colony_stockpiles(&mut colony);
        colony.stockpiles.push(designated_pile(
            "stockpile-food",
            tile_rect(10, 6),
            &[ResourceKind::Food],
        ));

        let shrine = &colony.buildings[0];
        assert_eq!(building_inbound_haul(&colony, shrine), 0.0);
    }

    #[test]
    fn building_inbound_haul_is_deterministic_across_identical_runs() {
        let mut left = new_world(4242);
        left.colonies
            .push(found_colony(left.world_seed, "colony-1", 1_000, 4242));
        let mut right = new_world(4242);
        right
            .colonies
            .push(found_colony(right.world_seed, "colony-1", 1_000, 4242));

        let mut saw_nonzero_inbound = false;
        for step in 1..=120 {
            let now = 1_000 + i64::from(step) * 60_000;
            let _ = world_tick(&mut left, now);
            let _ = world_tick(&mut right, now);

            let left_colony = &left.colonies[0];
            let right_colony = &right.colonies[0];
            assert_eq!(
                left_colony.buildings.len(),
                right_colony.buildings.len(),
                "identical seeds must place identical buildings at tick {step}"
            );
            for (left_building, right_building) in left_colony
                .buildings
                .iter()
                .zip(right_colony.buildings.iter())
            {
                let left_inbound = building_inbound_haul(left_colony, left_building);
                let right_inbound = building_inbound_haul(right_colony, right_building);
                assert_eq!(
                    left_inbound, right_inbound,
                    "inbound_haul diverged for building {} at tick {step}",
                    left_building.id
                );
                saw_nonzero_inbound |= left_inbound > 0.0;
            }
        }

        assert!(
            saw_nonzero_inbound,
            "expected at least one tick with cargo genuinely in flight toward a building"
        );
    }

    // ---- P16 gather spots ----

    #[test]
    fn a_gatherer_drops_its_yield_into_an_adjacent_gather_spot_instead_of_the_shrine() {
        // A gather spot behaves exactly like a general designated pile for the
        // *gatherer* side of P16 — it reuses P12.3 haul-fill routing verbatim (only
        // extra bookkeeping is the `GatherSpot` record; no special-case code needed).
        let pile_at = WorldPos { x: 30.0, y: 30.0 };
        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            cats: vec![carrying_cat_at("hunter", CarryingKind::Food, 8.0, pile_at)],
            ..ColonyRuntime::default()
        };
        reconcile_colony_stockpiles(&mut colony);
        colony.stockpiles.push(designated_pile(
            "gather-1",
            tile_rect(30, 30),
            &[ResourceKind::Food],
        ));
        colony.gather_spots.push(GatherSpot {
            stockpile_id: "gather-1".to_owned(),
            kind: ResourceKind::Food,
            expires_at_ms: 1_000_000,
        });

        let gate = production_gate(1, 1_000);
        phase_33_movement_deposits_and_no_destination_wander(
            &mut colony,
            gate,
            &mut haul_movement_ctx(),
        );

        let spot = colony
            .stockpiles
            .iter()
            .find(|p| p.id == "gather-1")
            .expect("gather spot present");
        assert_eq!(
            spot.contents.food, 8.0,
            "yield dropped at the adjacent gather spot"
        );
        let shrine = colony.stockpiles.iter().find(|p| p.is_shrine()).unwrap();
        assert_eq!(shrine.contents.food, 0.0, "not the shrine");
        assert_eq!(colony.resources.food, 8.0, "credited exactly once");
    }

    #[test]
    fn mover_hauls_a_gather_spots_contents_to_a_village_stockpile_without_double_crediting() {
        // Gather spot far out at (30,30); a real village stockpile much closer to it
        // than the shrine at (25,25) is the intended landing pad, so the mover's
        // destination unambiguously demonstrates the gather-spot exclusion.
        let gather_at = WorldPos { x: 30.0, y: 30.0 };
        let village_pile_at = WorldPos { x: 25.0, y: 25.0 };

        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            cats: vec![{
                let mut cat = adult_idle_cat("mover", "colony-1");
                cat.activity = CatActivity::Working;
                cat.position = position_from_world(gather_at);
                cat
            }],
            ..ColonyRuntime::default()
        };
        reconcile_colony_stockpiles(&mut colony);

        let mut spot_pile = designated_pile("gather-1", tile_rect(30, 30), &[ResourceKind::Food]);
        spot_pile.contents.food = 12.0;
        colony.stockpiles.push(spot_pile);
        colony.stockpiles.push(designated_pile(
            "stockpile-village",
            tile_rect(25, 25),
            &[ResourceKind::Food],
        ));
        // The gatherer already credited this yield into `resources` when it dropped it
        // at the gather spot — the mover only ever transfers it between piles.
        colony.resources.food = 12.0;
        colony.gather_spots.push(GatherSpot {
            stockpile_id: "gather-1".to_owned(),
            kind: ResourceKind::Food,
            expires_at_ms: 1_000_000,
        });
        colony.jobs.push(JobRuntime {
            id: "job-mover".to_owned(),
            kind: JobKind::HaulGatherSpot,
            status: JobStatus::Active,
            assigned_cat: Some("mover".to_owned()),
            metadata: JobMetadata::GatherHaul {
                stockpile_id: "gather-1".to_owned(),
                site: Some(world_pos_to_tile(gather_at)),
                accepted: true,
            },
            ..JobRuntime::default()
        });

        // Pickup: the mover has arrived (accepted: true) and isn't carrying yet.
        complete_arrived_gather_haul_movers(&mut colony, 5_000);

        assert_eq!(
            colony
                .stockpiles
                .iter()
                .find(|p| p.id == "gather-1")
                .unwrap()
                .contents
                .food,
            0.0,
            "gather spot emptied by pickup"
        );
        assert_eq!(
            colony.resources.food, 12.0,
            "pickup never re-adds to resources"
        );
        let job = colony.jobs.iter().find(|j| j.id == "job-mover").unwrap();
        assert_eq!(job.status, JobStatus::Completed);

        let carrying = colony.cats[0]
            .carrying
            .clone()
            .expect("mover picked up cargo");
        assert_eq!(carrying.kind, CarryingKind::Food);
        assert_eq!(carrying.amount, 12.0);
        assert_eq!(carrying.source_gather_spot.as_deref(), Some("gather-1"));
        assert_eq!(colony.cats[0].activity, CatActivity::Returning);
        // Heads for the real village pile — never back into the (now empty) gather spot.
        assert_eq!(
            colony.cats[0].destination,
            Some(position_from_world(village_pile_at))
        );

        // Walk it there and let the generic deposit loop credit the village pile.
        colony.cats[0].position = position_from_world(village_pile_at);
        let gate = production_gate(1, 6_000);
        phase_33_movement_deposits_and_no_destination_wander(
            &mut colony,
            gate,
            &mut haul_movement_ctx(),
        );

        assert!(colony.cats[0].carrying.is_none(), "mover unloaded");
        let village_pile = colony
            .stockpiles
            .iter()
            .find(|p| p.id == "stockpile-village")
            .unwrap();
        assert_eq!(
            village_pile.contents.food, 12.0,
            "goods landed in the village pile"
        );
        assert_eq!(
            colony
                .stockpiles
                .iter()
                .find(|p| p.id == "gather-1")
                .unwrap()
                .contents
                .food,
            0.0,
            "never routed back into the gather spot"
        );
        assert_eq!(colony.resources.food, 12.0, "never double-credited");
    }

    #[test]
    fn expired_gather_spot_is_cleared_folds_contents_back_and_cancels_its_mover() {
        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            cats: vec![adult_idle_cat("mover", "colony-1")],
            ..ColonyRuntime::default()
        };
        reconcile_colony_stockpiles(&mut colony);
        let mut spot_pile = designated_pile("gather-1", tile_rect(30, 30), &[ResourceKind::Food]);
        spot_pile.contents.food = 6.0;
        colony.stockpiles.push(spot_pile);
        colony.resources.food = 6.0;
        colony.gather_spots.push(GatherSpot {
            stockpile_id: "gather-1".to_owned(),
            kind: ResourceKind::Food,
            expires_at_ms: 10_000,
        });
        colony.jobs.push(JobRuntime {
            id: "job-mover".to_owned(),
            kind: JobKind::HaulGatherSpot,
            status: JobStatus::Active,
            assigned_cat: Some("mover".to_owned()),
            metadata: JobMetadata::GatherHaul {
                stockpile_id: "gather-1".to_owned(),
                site: Some(TilePos { x: 30, y: 30 }),
                accepted: false,
            },
            ..JobRuntime::default()
        });

        // Not yet expired.
        phase_p16_gather_spot_logistics(&mut colony, production_gate(1, 9_999));
        assert_eq!(colony.gather_spots.len(), 1, "TTL not yet elapsed");
        assert_eq!(
            colony
                .jobs
                .iter()
                .find(|j| j.id == "job-mover")
                .unwrap()
                .status,
            JobStatus::Active
        );

        // Past the TTL.
        phase_p16_gather_spot_logistics(&mut colony, production_gate(1, 10_000));
        assert!(colony.gather_spots.is_empty(), "expired spot cleared");
        assert!(!colony.stockpiles.iter().any(|p| p.id == "gather-1"));
        let shrine = colony.stockpiles.iter().find(|p| p.is_shrine()).unwrap();
        assert_eq!(
            shrine.contents.food, 6.0,
            "contents folded back into the reservoir"
        );
        let job = colony.jobs.iter().find(|j| j.id == "job-mover").unwrap();
        assert_eq!(job.status, JobStatus::Cancelled, "dangling mover cancelled");
        assert_eq!(
            colony.cats[0].activity,
            CatActivity::Idle,
            "cat freed since it was not yet carrying anything"
        );
    }

    // ---- P12.6 Steward auto-placed gather spots ----

    /// A qualifying `Quarry` (materials) job pair: two active jobs both resolved to `site`,
    /// mirroring how phase 15 gives every concurrent quarry job the same nearest-site target.
    fn quarry_pair_at(site: TilePos) -> Vec<JobRuntime> {
        (0..AUTO_GATHER_SPOT_MIN_WORKERS)
            .map(|i| JobRuntime {
                id: format!("job-quarry-{i}"),
                kind: JobKind::Quarry,
                status: JobStatus::Active,
                assigned_cat: Some(format!("quarrier-{i}")),
                metadata: JobMetadata::Hauling {
                    site: Some(site),
                    total_yield: None,
                    trips_done: 0,
                    next_trip_at: None,
                    accepted: false,
                },
                ..JobRuntime::default()
            })
            .collect()
    }

    fn steward_colony() -> ColonyRuntime {
        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            cats: vec![adult_idle_cat("steward-cat", "colony-1")],
            ..ColonyRuntime::default()
        };
        reconcile_colony_stockpiles(&mut colony);
        colony
            .officers
            .insert(OfficerRole::Steward, "steward-cat".to_owned());
        colony
    }

    #[test]
    fn steward_auto_designates_a_gather_spot_beside_a_distant_heavily_worked_site() {
        let mut colony = steward_colony();
        let site = TilePos {
            x: VILLAGE_ANCHOR.x + AUTO_GATHER_SPOT_MIN_DISTANCE + 2,
            y: VILLAGE_ANCHOR.y,
        };
        colony.jobs.extend(quarry_pair_at(site));

        phase_p16_gather_spot_logistics(&mut colony, production_gate(1, 5_000));

        assert_eq!(colony.gather_spots.len(), 1, "one spot auto-designated");
        let spot = &colony.gather_spots[0];
        assert_eq!(spot.kind, ResourceKind::Materials);
        assert!(spot.stockpile_id.starts_with("gather-auto-"));
        let pile = colony
            .stockpiles
            .iter()
            .find(|p| p.id == spot.stockpile_id)
            .expect("backing pile present");
        // Adjacent to (not on top of) the site, on the cardinal neighbour closest to the
        // anchor — west, since the site sits due east of it.
        assert_eq!(
            pile.rect,
            tile_rect(site.x - 1, site.y),
            "placed adjacent to the site, biased toward the village"
        );
        assert_eq!(pile.accepts.len(), 1);
        assert!(pile.accepts.contains(&ResourceKind::Materials));
    }

    #[test]
    fn steward_auto_placement_is_inert_without_a_steward() {
        // Same distant, heavily-worked site as the positive case above, but no Steward
        // appointed — additive/inert (P12.2 "unfilled role, no auto effect" contract).
        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            cats: vec![adult_idle_cat("cat-1", "colony-1")],
            ..ColonyRuntime::default()
        };
        reconcile_colony_stockpiles(&mut colony);
        let site = TilePos {
            x: VILLAGE_ANCHOR.x + AUTO_GATHER_SPOT_MIN_DISTANCE + 2,
            y: VILLAGE_ANCHOR.y,
        };
        colony.jobs.extend(quarry_pair_at(site));
        let before = colony.clone();

        phase_p16_gather_spot_logistics(&mut colony, production_gate(1, 5_000));

        assert!(colony.gather_spots.is_empty(), "no Steward, no placement");
        assert_eq!(
            colony.stockpiles, before.stockpiles,
            "byte-identical: not even the shrine reservoir changes"
        );
    }

    #[test]
    fn steward_auto_placement_ignores_a_site_inside_the_distance_threshold() {
        let mut colony = steward_colony();
        // One tile inside the qualifying radius.
        let near_site = TilePos {
            x: VILLAGE_ANCHOR.x + AUTO_GATHER_SPOT_MIN_DISTANCE - 1,
            y: VILLAGE_ANCHOR.y,
        };
        colony.jobs.extend(quarry_pair_at(near_site));

        phase_p16_gather_spot_logistics(&mut colony, production_gate(1, 5_000));

        assert!(
            colony.gather_spots.is_empty(),
            "site is too close to the village for the split to pay off"
        );
    }

    #[test]
    fn steward_auto_placement_ignores_a_site_with_only_one_worker() {
        let mut colony = steward_colony();
        let site = TilePos {
            x: VILLAGE_ANCHOR.x + AUTO_GATHER_SPOT_MIN_DISTANCE + 2,
            y: VILLAGE_ANCHOR.y,
        };
        // Only one concurrent worker — below AUTO_GATHER_SPOT_MIN_WORKERS.
        colony.jobs.push(quarry_pair_at(site).remove(0));

        phase_p16_gather_spot_logistics(&mut colony, production_gate(1, 5_000));

        assert!(
            colony.gather_spots.is_empty(),
            "a lone gatherer's round trip gets no benefit from a hand-off"
        );
    }

    #[test]
    fn steward_auto_placement_respects_the_gather_spot_budget() {
        let mut colony = steward_colony();
        // Fill the budget with manually designated spots on unrelated, near sites first.
        for i in 0..MAX_GATHER_SPOTS {
            let id = format!("gather-manual-{i}");
            colony.stockpiles.push(designated_pile(
                &id,
                tile_rect(VILLAGE_ANCHOR.x, VILLAGE_ANCHOR.y + 1 + i as i32),
                &[ResourceKind::Food],
            ));
            colony.gather_spots.push(GatherSpot {
                stockpile_id: id,
                kind: ResourceKind::Food,
                expires_at_ms: i64::MAX,
            });
        }
        assert_eq!(colony.gather_spots.len(), MAX_GATHER_SPOTS);

        let site = TilePos {
            x: VILLAGE_ANCHOR.x + AUTO_GATHER_SPOT_MIN_DISTANCE + 2,
            y: VILLAGE_ANCHOR.y,
        };
        colony.jobs.extend(quarry_pair_at(site));

        phase_p16_gather_spot_logistics(&mut colony, production_gate(1, 5_000));

        assert_eq!(
            colony.gather_spots.len(),
            MAX_GATHER_SPOTS,
            "budget already spent — no over-placement"
        );
    }

    #[test]
    fn steward_auto_placement_never_duplicates_a_spot_on_an_already_served_site() {
        let mut colony = steward_colony();
        let site = TilePos {
            x: VILLAGE_ANCHOR.x + AUTO_GATHER_SPOT_MIN_DISTANCE + 2,
            y: VILLAGE_ANCHOR.y,
        };
        // A manually designated materials spot already sits right beside this exact site.
        colony.stockpiles.push(designated_pile(
            "gather-manual",
            tile_rect(site.x - 1, site.y),
            &[ResourceKind::Materials],
        ));
        colony.gather_spots.push(GatherSpot {
            stockpile_id: "gather-manual".to_owned(),
            kind: ResourceKind::Materials,
            expires_at_ms: i64::MAX,
        });
        colony.jobs.extend(quarry_pair_at(site));

        phase_p16_gather_spot_logistics(&mut colony, production_gate(1, 5_000));

        assert_eq!(
            colony.gather_spots.len(),
            1,
            "the existing spot already serves this site — no duplicate"
        );
    }

    #[test]
    fn steward_auto_placement_is_deterministic_for_identical_inputs() {
        let mut left = steward_colony();
        let mut right = left.clone();

        // Several simultaneous candidates across kinds/sites so ordering actually matters.
        let quarry_site = TilePos {
            x: VILLAGE_ANCHOR.x + AUTO_GATHER_SPOT_MIN_DISTANCE + 2,
            y: VILLAGE_ANCHOR.y,
        };
        let hunt_site = TilePos {
            x: VILLAGE_ANCHOR.x,
            y: VILLAGE_ANCHOR.y - AUTO_GATHER_SPOT_MIN_DISTANCE - 4,
        };
        let water_site = TilePos {
            x: VILLAGE_ANCHOR.x - AUTO_GATHER_SPOT_MIN_DISTANCE - 6,
            y: VILLAGE_ANCHOR.y,
        };
        let mut jobs = quarry_pair_at(quarry_site);
        jobs.extend((0..AUTO_GATHER_SPOT_MIN_WORKERS).map(|i| JobRuntime {
            id: format!("job-hunt-{i}"),
            kind: JobKind::HuntExpedition,
            status: JobStatus::Active,
            assigned_cat: Some(format!("hunter-{i}")),
            metadata: JobMetadata::Hauling {
                site: Some(hunt_site),
                total_yield: None,
                trips_done: 0,
                next_trip_at: None,
                accepted: false,
            },
            ..JobRuntime::default()
        }));
        jobs.extend((0..AUTO_GATHER_SPOT_MIN_WORKERS).map(|i| JobRuntime {
            id: format!("job-water-{i}"),
            kind: JobKind::FetchWater,
            status: JobStatus::Queued,
            assigned_cat: Some(format!("fetcher-{i}")),
            metadata: JobMetadata::Hauling {
                site: Some(water_site),
                total_yield: None,
                trips_done: 0,
                next_trip_at: None,
                accepted: false,
            },
            ..JobRuntime::default()
        }));
        left.jobs.clone_from(&jobs);
        right.jobs = jobs;

        phase_p16_gather_spot_logistics(&mut left, production_gate(1, 5_000));
        phase_p16_gather_spot_logistics(&mut right, production_gate(1, 5_000));

        assert!(
            !left.gather_spots.is_empty(),
            "test exercises real placement"
        );
        assert_eq!(left.gather_spots, right.gather_spots);
        assert_eq!(left.stockpiles, right.stockpiles);
    }

    #[test]
    fn gather_spot_logistics_phase_is_a_true_no_op_with_no_gather_spots_even_with_a_steward() {
        // Exercises every branch of `phase_p16_gather_spot_logistics` (expiry, pickup,
        // and — since a Steward is appointed — the dispatch and auto-placement branches
        // too) while no gather spot ever exists and no candidate site in this founding
        // colony reaches the auto-placement thresholds (distance + concurrent workers)
        // within the window: the phase must do nothing observable. Every pre-existing
        // determinism/trajectory test in this module (unchanged by this feature) is the
        // rest of the "byte-identical to pre-P16" evidence; this test isolates P16's own
        // phase specifically.
        let mut world = new_world(9_191);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 1_000, 3_030));
        let steward_cat = world.colonies[0].cats[0].id.clone();
        world.colonies[0]
            .officers
            .insert(OfficerRole::Steward, steward_cat);

        for step in 1..=40 {
            let now = 1_000 + i64::from(step) * 60_000;
            let _ = world_tick(&mut world, now);
            let colony = &world.colonies[0];
            assert!(
                colony.gather_spots.is_empty(),
                "no gather spot ever designated at step {step}"
            );
            assert!(
                !colony
                    .jobs
                    .iter()
                    .any(|job| job.kind == JobKind::HaulGatherSpot),
                "nothing to dispatch a mover for at step {step}"
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
        phase_23_production(&mut colony, production_gate(30, 30_000), 123);

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
        phase_23_production(&mut staffed, production_gate(30, 30_000), 123);
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
        phase_23_production(&mut auto_staffed, production_gate(30, 30_000), 123);
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
        phase_23_production(&mut no_worker, production_gate(30, 30_000), 123);
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
        phase_23_production(&mut colony, production_gate(30, 30_000), 123);
        assert_eq!(colony.resources.blocks, 1.0);
        assert_eq!(colony.resources.materials, 45.0);
    }

    // --- P17/P19 ore -> metal chain ---

    #[test]
    fn smelter_refines_ore_into_metal_when_staffed() {
        // Staffed: 590 + 30 >= 600 completes one cycle -> 5 ore become 1 metal bar,
        // mirroring stone_prep_dresses_materials_into_blocks_when_staffed exactly.
        let mut colony = chain_colony(
            BuildingType::Smelter,
            Resources {
                ore: 50.0,
                ..Resources::default()
            },
            true,
        );
        phase_23_production(&mut colony, production_gate(30, 30_000), 123);
        assert_eq!(colony.resources.metal, 1.0);
        assert_eq!(colony.resources.ore, 45.0);
    }

    #[test]
    fn smelter_without_ore_or_a_worker_produces_nothing() {
        // No ore on hand: the bench still auto-staffs (idle cat) but has nothing to refine.
        let mut no_ore = chain_colony(BuildingType::Smelter, Resources::default(), false);
        phase_23_production(&mut no_ore, production_gate(30, 30_000), 123);
        assert_eq!(no_ore.resources.metal, 0.0);

        // Ore on hand but no cat at all to mop the bench up.
        let mut no_worker = chain_colony(
            BuildingType::Smelter,
            Resources {
                ore: 50.0,
                ..Resources::default()
            },
            false,
        );
        no_worker.cats.clear();
        phase_23_production(&mut no_worker, production_gate(30, 30_000), 123);
        assert_eq!(no_worker.resources.metal, 0.0);
        assert_eq!(no_worker.resources.ore, 50.0);
    }

    #[test]
    fn a_colony_with_no_smelter_building_never_produces_metal_even_with_banked_ore() {
        // Additive/inert guardrail: `resources.ore` can only ever move (via
        // `credit_quarry_ore`) into `resources.metal` through an actual Smelter
        // building. A colony that banked ore but never built one must see it sit
        // completely untouched through production.
        let mut colony = chain_colony(
            BuildingType::WoodCutter,
            Resources {
                ore: 50.0,
                materials: 0.0,
                ..Resources::default()
            },
            true,
        );
        phase_23_production(&mut colony, production_gate(30, 30_000), 123);
        assert_eq!(
            colony.resources.ore, 50.0,
            "ore is untouched with no smelter"
        );
        assert_eq!(
            colony.resources.metal, 0.0,
            "metal never appears without a smelter"
        );
    }

    /// Find one tile whose [`Mining`] rule does/doesn't match `wants_full`, for a given
    /// `world_seed`. Scans whole terrain chunks via `generate_terrain_chunk` (144 tiles
    /// per call) rather than `tile_climate_biome` tile-by-tile — `tile_climate_biome`
    /// regenerates its owning chunk on *every single call*, so a naive per-tile scan
    /// over any real search area is a perf trap (this is exactly the hazard flagged for
    /// `credit_quarry_ore`: never do this from a hot per-tick/per-tile path). A test-only
    /// helper scanning a bounded chunk grid once is fine.
    fn find_mining_site(world_seed: u32, wants_full: bool) -> TilePos {
        for chunk_x in -12..=12 {
            for chunk_y in -12..=12 {
                let tiles = crate::terrain_gen::generate_terrain_chunk(
                    chunk_x,
                    chunk_y,
                    i64::from(world_seed),
                    crate::terrain_gen::WORLD_TERRAIN_OPTIONS,
                );
                if let Some(tile) = tiles.iter().find(|tile| {
                    (tile.climate_biome.properties().mining == Mining::Full) == wants_full
                }) {
                    return TilePos {
                        x: tile.x,
                        y: tile.y,
                    };
                }
            }
        }
        panic!("no tile with mining == Full ({wants_full}) found within the scanned chunk radius");
    }

    #[test]
    fn quarry_completion_credits_ore_only_on_a_genuine_mountain_biome_site() {
        // Pick a world seed/tile pair where the site is a true Mountains biome
        // (Mining::Full) versus one that reliably is not, and drive `phase_29` through
        // a completed quarry job at each site to prove the ore byproduct only ever
        // appears on the former.
        let world_seed = 777u32;
        let mountain_site = find_mining_site(world_seed, true);
        let non_mountain_site = find_mining_site(world_seed, false);

        for (site, expect_ore) in [(mountain_site, true), (non_mountain_site, false)] {
            let mut colony = ColonyRuntime {
                id: "colony-1".to_owned(),
                resources: Resources::default(),
                cats: vec![adult_idle_cat("quarrier", "colony-1")],
                jobs: vec![JobRuntime {
                    id: "job-1".to_owned(),
                    kind: JobKind::Quarry,
                    status: JobStatus::Active,
                    assigned_cat: Some("quarrier".to_owned()),
                    started_at: Some(0),
                    ends_at: Some(1_000),
                    metadata: JobMetadata::Hauling {
                        site: Some(site),
                        total_yield: Some(QUARRY_TOTAL_YIELD),
                        trips_done: (HUNT_TRIP_COUNT - 1) as u32,
                        next_trip_at: None,
                        accepted: true,
                    },
                    ..JobRuntime::default()
                }],
                last_tick: 0,
                test_rng_seed: Some(1),
                ..ColonyRuntime::default()
            };
            colony.cats[0].activity = CatActivity::Working;

            phase_29_due_completion_gathering_explore_expansion(
                &mut colony,
                production_gate(0, 1_000),
                world_seed,
            );

            if expect_ore {
                assert!(
                    colony.resources.ore > 0.0,
                    "mountain quarry site {site:?} should credit ore"
                );
            } else {
                assert_eq!(
                    colony.resources.ore, 0.0,
                    "non-mountain quarry site {site:?} should never credit ore"
                );
            }
        }
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
        phase_23_production(&mut colony, production_gate(30, 30_000), 123);
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
        phase_23_production(&mut starved, production_gate(30, 30_000), 123);
        assert_eq!(starved.resources.tools, 0.0);
        assert_eq!(starved.resources.planks, 10.0);

        // Exactly 6/6 can spend one 2/2 cycle while preserving the 4/4 build
        // reserve. Anything below that boundary stalls rather than consuming the
        // scaffold reserve.
        let mut at_reserve_boundary = chain_colony(
            BuildingType::Woodworking,
            Resources {
                planks: 6.0,
                blocks: 6.0,
                ..Resources::default()
            },
            true,
        );
        phase_23_production(&mut at_reserve_boundary, production_gate(30, 30_000), 123);
        assert_eq!(at_reserve_boundary.resources.tools, 1.0);
        assert_eq!(at_reserve_boundary.resources.planks, 4.0);
        assert_eq!(at_reserve_boundary.resources.blocks, 4.0);

        let mut below_reserve_boundary = chain_colony(
            BuildingType::Woodworking,
            Resources {
                planks: 5.9,
                blocks: 6.0,
                ..Resources::default()
            },
            true,
        );
        phase_23_production(
            &mut below_reserve_boundary,
            production_gate(30, 30_000),
            123,
        );
        assert_eq!(below_reserve_boundary.resources.tools, 0.0);
        assert_eq!(below_reserve_boundary.resources.planks, 5.9);
        assert_eq!(below_reserve_boundary.resources.blocks, 6.0);
    }

    #[test]
    fn raw_benches_preserve_the_offering_reserve_after_tool_bootstrap() {
        let mut colony = chain_colony(
            BuildingType::WoodCutter,
            Resources {
                materials: 25.0,
                tools: 1.0,
                ..Resources::default()
            },
            true,
        );
        phase_23_production(&mut colony, production_gate(30, 30_000), 123);
        assert_eq!(colony.resources.materials, OFFERING_MATERIALS_RESERVE);
        assert_eq!(colony.resources.planks, 1.0);
    }

    // ---- P19 slice 2: material-variant trade-good crafting ----

    #[test]
    fn woodworking_bench_crafts_a_wood_trade_good_from_surplus_planks() {
        let mut colony = chain_colony(
            BuildingType::Woodworking,
            Resources {
                planks: 30.0, // 20-plank reserve + 10 surplus + headroom for tools' draw
                blocks: 30.0,
                ..Resources::default()
            },
            true,
        );
        // Prime the trade-craft timer so this tick's 30s elapsed completes one 900s cycle.
        colony.wood_craft_progress = 890.0;
        phase_23_production(&mut colony, production_gate(30, 30_000), 123);

        // The functional tools recipe (unchanged) also completes this tick — production_progress
        // 590 + 30 >= 600 — and claims its 2 planks + 2 blocks first.
        assert_eq!(colony.resources.tools, 1.0);
        // Trade craft then spends its own 1 plank on top of that: 30 - 2 (tools) - 1 (trade) = 27.
        assert_eq!(
            colony.resources.planks, 27.0,
            "tools ran first, trade craft spent only its own 1 plank of surplus"
        );
        assert_eq!(
            colony.resources.blocks, 28.0,
            "trade craft never touches blocks on the wood bench"
        );

        let wood_items: Vec<(&Item, &u32)> = colony
            .items
            .iter()
            .filter(|(item, _)| item.material == Material::Wood)
            .collect();
        assert_eq!(
            wood_items.len(),
            1,
            "exactly one wood trade-good stack crafted"
        );
        let (item, count) = wood_items[0];
        assert_eq!(*count, 1);
        assert!(
            matches!(
                item.kind,
                ItemKind::Mug | ItemKind::Bowl | ItemKind::Furniture | ItemKind::Toy
            ),
            "kind {:?} must be one of WOOD_TRADE_RECIPE's rotation",
            item.kind
        );

        // The assigned worker's Craft skill is trained the first time it's used.
        let worker = colony.cats.iter().find(|cat| cat.id == "crafter").unwrap();
        assert_eq!(worker.skill(Labor::Craft), SKILL_GAIN_PER_JOB);
    }

    #[test]
    fn stone_prep_bench_crafts_a_stone_trade_good_from_surplus_blocks() {
        let mut colony = chain_colony(
            BuildingType::StonePrep,
            Resources {
                materials: 50.0,
                blocks: 25.0, // already above the 20-block reserve before this tick's own output
                ..Resources::default()
            },
            true,
        );
        colony.stone_craft_progress = 890.0;
        phase_23_production(&mut colony, production_gate(30, 30_000), 123);

        // The functional recipe (unchanged) produces 1 block from 5 materials this tick
        // (25 + 1 = 26), then the trade craft spends 1 of the surplus: 26 - 1 = 25.
        assert_eq!(colony.resources.blocks, 25.0);
        assert_eq!(colony.resources.materials, 45.0);

        let stone_items: Vec<(&Item, &u32)> = colony
            .items
            .iter()
            .filter(|(item, _)| item.material == Material::Stone)
            .collect();
        assert_eq!(
            stone_items.len(),
            1,
            "exactly one stone trade-good stack crafted"
        );
        let (item, count) = stone_items[0];
        assert_eq!(*count, 1);
        assert!(
            matches!(item.kind, ItemKind::Bowl | ItemKind::Trinket),
            "kind {:?} must be one of STONE_TRADE_RECIPE's rotation",
            item.kind
        );
    }

    #[test]
    fn trade_good_quality_reflects_the_assigned_crafters_skill() {
        fn quality_after_one_cycle(craft_skill: f64) -> u8 {
            let mut colony = chain_colony(
                BuildingType::Woodworking,
                Resources {
                    planks: 30.0,
                    blocks: 30.0,
                    ..Resources::default()
                },
                true,
            );
            colony.cats[0].gain_skill(Labor::Craft, craft_skill);
            colony.wood_craft_progress = 890.0;
            phase_23_production(&mut colony, production_gate(30, 30_000), 123);
            colony
                .items
                .iter()
                .find(|(item, _)| item.material == Material::Wood)
                .expect("a wood item was crafted")
                .0
                .quality
        }

        let low_skill_quality = quality_after_one_cycle(0.0);
        let high_skill_quality = quality_after_one_cycle(400.0);
        assert_eq!(
            low_skill_quality, 0,
            "an untrained crafter yields the crude band"
        );
        assert_eq!(
            high_skill_quality, MAX_QUALITY,
            "400 craft xp yields masterwork"
        );
        assert!(
            high_skill_quality > low_skill_quality,
            "a more-skilled crafter must never yield lower quality for the same recipe"
        );
    }

    #[test]
    fn trade_craft_at_or_below_the_surplus_reserve_produces_no_items() {
        // 15 blocks sits below STONE_TRADE_RECIPE's 20-block reserve; zero materials keeps
        // the functional block-production recipe from adding to it this tick, isolating the
        // surplus gate. The colony must protect its (already scarce) blocks from crafting.
        let mut colony = chain_colony(
            BuildingType::StonePrep,
            Resources {
                materials: 0.0,
                blocks: 15.0,
                ..Resources::default()
            },
            true,
        );
        colony.stone_craft_progress = 890.0;
        phase_23_production(&mut colony, production_gate(30, 30_000), 123);

        assert_eq!(
            colony.resources.blocks, 15.0,
            "surplus gate must leave scarce blocks untouched"
        );
        assert!(
            colony.items.is_empty(),
            "no trade goods should be crafted at/below the surplus reserve"
        );
    }

    #[test]
    fn trade_craft_horizon_is_deterministic_across_identical_runs() {
        fn build() -> ColonyRuntime {
            chain_colony(
                BuildingType::Woodworking,
                Resources {
                    planks: 200.0,
                    blocks: 200.0,
                    ..Resources::default()
                },
                true,
            )
        }

        let mut left = build();
        let mut right = build();

        for step in 1..=60 {
            let now = i64::from(step) * 60_000;
            phase_23_production(&mut left, production_gate(60, now), 123);
            phase_23_production(&mut right, production_gate(60, now), 123);
        }

        assert_eq!(
            left, right,
            "identical runs (including the items store and cat skills) must be byte-identical"
        );
        assert!(
            !left.items.is_empty(),
            "the horizon should have completed at least one trade-craft cycle"
        );
    }

    // ---- P16/P19 clothing chain slice: clothier/tannery refine + trade craft ----

    #[test]
    fn clothier_refines_fibre_into_cloth_when_it_has_a_worker() {
        let mut staffed = chain_colony(
            BuildingType::Clothier,
            Resources {
                fibre: 50.0,
                ..Resources::default()
            },
            true,
        );
        phase_23_production(&mut staffed, production_gate(30, 30_000), 123);
        assert_eq!(staffed.resources.cloth, 1.0);
        assert_eq!(staffed.resources.fibre, 45.0);

        // With no worker at all, the bench makes nothing.
        let mut no_worker = chain_colony(
            BuildingType::Clothier,
            Resources {
                fibre: 50.0,
                ..Resources::default()
            },
            false,
        );
        no_worker.cats.clear();
        phase_23_production(&mut no_worker, production_gate(30, 30_000), 123);
        assert_eq!(no_worker.resources.cloth, 0.0);
        assert_eq!(no_worker.resources.fibre, 50.0);
    }

    #[test]
    fn tannery_tans_hide_into_leather_when_staffed() {
        let mut colony = chain_colony(
            BuildingType::Tannery,
            Resources {
                hide: 50.0,
                ..Resources::default()
            },
            true,
        );
        phase_23_production(&mut colony, production_gate(30, 30_000), 123);
        assert_eq!(colony.resources.leather, 1.0);
        assert_eq!(colony.resources.hide, 45.0);
    }

    #[test]
    fn clothier_bench_crafts_a_clothing_trade_good_from_surplus_cloth() {
        let mut colony = chain_colony(
            BuildingType::Clothier,
            Resources {
                fibre: 50.0,
                cloth: 21.0, // above CLOTH_TRADE_RECIPE's 20-cloth reserve
                ..Resources::default()
            },
            true,
        );
        colony.clothier_craft_progress = 890.0;
        phase_23_production(&mut colony, production_gate(30, 30_000), 123);

        // The functional refine (unchanged) also completes this tick and adds 1 cloth
        // (21 + 1 = 22), then the trade craft spends 1 of the surplus: 22 - 1 = 21.
        assert_eq!(colony.resources.cloth, 21.0);

        let clothing_items: Vec<(&Item, &u32)> = colony
            .items
            .iter()
            .filter(|(item, _)| item.material == Material::Fibre)
            .collect();
        assert_eq!(
            clothing_items.len(),
            1,
            "exactly one fibre clothing stack crafted"
        );
        let (item, count) = clothing_items[0];
        assert_eq!(*count, 1);
        assert_eq!(item.kind, ItemKind::Clothing);

        // The assigned worker's Craft skill is trained the first time it's used.
        let worker = colony.cats.iter().find(|cat| cat.id == "crafter").unwrap();
        assert_eq!(worker.skill(Labor::Craft), SKILL_GAIN_PER_JOB);
    }

    #[test]
    fn tannery_bench_crafts_a_clothing_trade_good_from_surplus_leather() {
        let mut colony = chain_colony(
            BuildingType::Tannery,
            Resources {
                hide: 50.0,
                leather: 21.0, // above LEATHER_TRADE_RECIPE's 20-leather reserve
                ..Resources::default()
            },
            true,
        );
        colony.tannery_craft_progress = 890.0;
        phase_23_production(&mut colony, production_gate(30, 30_000), 123);

        assert_eq!(colony.resources.leather, 21.0);

        let clothing_items: Vec<(&Item, &u32)> = colony
            .items
            .iter()
            .filter(|(item, _)| item.material == Material::Leather)
            .collect();
        assert_eq!(
            clothing_items.len(),
            1,
            "exactly one leather clothing stack crafted"
        );
        let (item, count) = clothing_items[0];
        assert_eq!(*count, 1);
        assert_eq!(item.kind, ItemKind::Clothing);
    }

    #[test]
    fn clothing_trade_craft_at_or_below_the_surplus_reserve_produces_no_items() {
        // 15 cloth sits below CLOTH_TRADE_RECIPE's 20-cloth reserve; zero fibre keeps the
        // functional refine from topping it up this tick, isolating the surplus gate. The
        // colony must protect its (already scarce) cloth from trade crafting.
        let mut colony = chain_colony(
            BuildingType::Clothier,
            Resources {
                fibre: 0.0,
                cloth: 15.0,
                ..Resources::default()
            },
            true,
        );
        colony.clothier_craft_progress = 890.0;
        phase_23_production(&mut colony, production_gate(30, 30_000), 123);

        assert_eq!(
            colony.resources.cloth, 15.0,
            "surplus gate must leave scarce cloth untouched"
        );
        assert!(
            colony.items.is_empty(),
            "no trade goods should be crafted at/below the surplus reserve"
        );
    }

    #[test]
    fn clothing_trade_good_quality_reflects_the_assigned_crafters_skill() {
        fn quality_after_one_cycle(craft_skill: f64) -> u8 {
            let mut colony = chain_colony(
                BuildingType::Clothier,
                Resources {
                    fibre: 50.0,
                    cloth: 30.0,
                    ..Resources::default()
                },
                true,
            );
            colony.cats[0].gain_skill(Labor::Craft, craft_skill);
            colony.clothier_craft_progress = 890.0;
            phase_23_production(&mut colony, production_gate(30, 30_000), 123);
            colony
                .items
                .iter()
                .find(|(item, _)| item.material == Material::Fibre)
                .expect("a fibre clothing item was crafted")
                .0
                .quality
        }

        let low_skill_quality = quality_after_one_cycle(0.0);
        let high_skill_quality = quality_after_one_cycle(400.0);
        assert_eq!(
            low_skill_quality, 0,
            "an untrained crafter yields the crude band"
        );
        assert_eq!(
            high_skill_quality, MAX_QUALITY,
            "400 craft xp yields masterwork"
        );
        assert!(
            high_skill_quality > low_skill_quality,
            "a more-skilled crafter must never yield lower quality for the same recipe"
        );
    }

    #[test]
    fn a_colony_that_never_builds_a_clothier_or_tannery_is_unaffected_by_the_clothing_chain() {
        // Additive guardrail: a colony with no clothier/tannery building must produce no
        // cloth/leather/clothing items — the chain only ever fires when those benches
        // exist. Fibre still passively forages (phase 23c is population-scaled, not
        // gated on a clothier existing) but that alone must never mutate `items`.
        let mut colony = chain_colony(BuildingType::WoodCutter, Resources::default(), true);
        phase_23_production(&mut colony, production_gate(30, 30_000), 123);
        phase_23c_fibre_forage(&mut colony, production_gate(30, 30_000));

        assert_eq!(colony.resources.cloth, 0.0);
        assert_eq!(colony.resources.leather, 0.0);
        assert!(
            colony
                .items
                .keys()
                .all(|item| item.material != Material::Fibre && item.material != Material::Leather),
            "no clothing items without a clothier/tannery"
        );
    }

    #[test]
    fn clothing_chain_horizon_is_deterministic_across_identical_runs() {
        fn build() -> ColonyRuntime {
            chain_colony(
                BuildingType::Clothier,
                Resources {
                    fibre: 200.0,
                    cloth: 200.0,
                    ..Resources::default()
                },
                true,
            )
        }

        let mut left = build();
        let mut right = build();

        for step in 1..=60 {
            let now = i64::from(step) * 60_000;
            let gate = production_gate(60, now);
            phase_23_production(&mut left, gate, 123);
            phase_23c_fibre_forage(&mut left, gate);
            phase_23_production(&mut right, gate, 123);
            phase_23c_fibre_forage(&mut right, gate);
        }

        assert_eq!(
            left, right,
            "identical runs (including the items store and cat skills) must be byte-identical"
        );
        assert!(
            !left.items.is_empty(),
            "the horizon should have completed at least one clothing trade-craft cycle"
        );
    }

    #[test]
    fn fibre_forage_trickles_in_passively_without_any_building_or_worker() {
        let mut colony = chain_colony(BuildingType::Den, Resources::default(), false);
        colony.cats = vec![
            adult_idle_cat("cat-1", "colony-1"),
            adult_idle_cat("cat-2", "colony-1"),
        ];
        phase_23c_fibre_forage(&mut colony, production_gate(3_600, 3_600_000));
        // 2 living cats * 0.05 fibre/cat/hour * 1 hour = 0.1.
        assert_eq!(colony.resources.fibre, 0.1);
    }

    // ---- P19 slice 3: visiting trader / caravan economy ----

    fn trader_test_colony(seed: u32) -> ColonyRuntime {
        ColonyRuntime {
            id: "colony-1".to_owned(),
            run_started_at: 0,
            created_at: 0,
            last_tick: 0,
            test_rng_seed: Some(seed),
            ..ColonyRuntime::default()
        }
    }

    fn game_hours_to_ms(hours: f64) -> i64 {
        (hours * 3_600.0 * 1_000.0) as i64
    }

    fn trader_gate_world_pos(colony: &ColonyRuntime) -> WorldPos {
        tile_pos_to_world(raid_gate_position(colony))
    }

    #[test]
    fn trader_does_not_spawn_before_the_scheduled_interval() {
        let mut colony = trader_test_colony(1);
        let almost_due = game_hours_to_ms(trader::TRADER_VISIT_INTERVAL_GAME_HOURS) - 1_000;
        phase_36b_trader_lifecycle(&mut colony, production_gate(0, almost_due));
        assert!(colony.trader.is_none());
    }

    #[test]
    fn trader_spawns_arriving_off_map_at_the_scheduled_interval() {
        let mut colony = trader_test_colony(1);
        let due = game_hours_to_ms(trader::TRADER_VISIT_INTERVAL_GAME_HOURS);
        phase_36b_trader_lifecycle(&mut colony, production_gate(0, due));

        let unit = colony.trader.as_ref().expect("trader should have spawned");
        assert_eq!(unit.state, trader::TraderState::Arriving);
        assert_eq!(unit.arrived_at, None);
        let gate_pos = trader_gate_world_pos(&colony);
        let position = WorldPos {
            x: unit.position.x,
            y: unit.position.y,
        };
        assert!(
            cheb_distance_world(position, gate_pos) > trader::TRADER_ARRIVE_RANGE,
            "a freshly-spawned trader must start off-map, not already at the gate"
        );
        assert!(
            colony
                .events
                .iter()
                .any(|event| event.kind == EventKind::TraderArrived),
        );
    }

    #[test]
    fn trader_lifecycle_advances_arriving_trading_departing_gone_on_the_documented_timeline() {
        let mut colony = trader_test_colony(1);

        // 1) Spawn at the scheduled interval.
        let spawn_ms = game_hours_to_ms(trader::TRADER_VISIT_INTERVAL_GAME_HOURS);
        phase_36b_trader_lifecycle(&mut colony, production_gate(0, spawn_ms));
        assert_eq!(
            colony.trader.as_ref().unwrap().state,
            trader::TraderState::Arriving
        );

        // 2) A big movement budget (6 real/game-hours at 0.5 tiles/sec = 10_800 tiles,
        // far more than TRADER_SPAWN_DISTANCE=12) carries it all the way to the gate in
        // one tick -> Trading.
        let travel_sec = 6 * 3_600;
        let arrive_ms = spawn_ms + travel_sec * 1_000;
        phase_36b_trader_lifecycle(&mut colony, production_gate(travel_sec, arrive_ms));
        {
            let unit = colony.trader.as_ref().unwrap();
            assert_eq!(unit.state, trader::TraderState::Trading);
            assert_eq!(unit.arrived_at, Some(arrive_ms));
            let gate_pos = trader_gate_world_pos(&colony);
            let position = WorldPos {
                x: unit.position.x,
                y: unit.position.y,
            };
            assert!(cheb_distance_world(position, gate_pos) <= trader::TRADER_ARRIVE_RANGE);
        }
        assert!(
            colony
                .events
                .iter()
                .any(|event| event.kind == EventKind::TraderTrading),
        );

        // 3) Still inside the linger window -> stays Trading.
        let mid_linger_ms = arrive_ms + game_hours_to_ms(trader::TRADER_LINGER_GAME_HOURS - 1.0);
        phase_36b_trader_lifecycle(&mut colony, production_gate(3_600, mid_linger_ms));
        assert_eq!(
            colony.trader.as_ref().unwrap().state,
            trader::TraderState::Trading,
            "still within the linger window"
        );

        // 4) Past the linger window -> Departing.
        let past_linger_ms = arrive_ms + game_hours_to_ms(trader::TRADER_LINGER_GAME_HOURS + 1.0);
        phase_36b_trader_lifecycle(&mut colony, production_gate(3_600, past_linger_ms));
        assert_eq!(
            colony.trader.as_ref().unwrap().state,
            trader::TraderState::Departing
        );

        // 5) Another big movement budget carries it back off-map -> despawns.
        let depart_ms = past_linger_ms + travel_sec * 1_000;
        phase_36b_trader_lifecycle(&mut colony, production_gate(travel_sec, depart_ms));
        assert!(colony.trader.is_none(), "the trader should have despawned");
        assert_eq!(colony.last_trader_departed_at, Some(depart_ms));
        assert!(
            colony
                .events
                .iter()
                .any(|event| event.kind == EventKind::TraderDeparted),
        );
    }

    #[test]
    fn trader_visit_lifecycle_is_deterministic_across_identical_runs() {
        fn build() -> ColonyRuntime {
            trader_test_colony(4_242)
        }

        let mut left = build();
        let mut right = build();

        // Hourly phase calls across ~60 game-hours: past the 48h spawn interval, the 6h
        // linger, and the (well-under-an-hour) arrival/departure travel.
        for step in 1..=60 {
            let now = i64::from(step) * 3_600_000;
            phase_36b_trader_lifecycle(&mut left, production_gate(3_600, now));
            phase_36b_trader_lifecycle(&mut right, production_gate(3_600, now));
        }

        assert_eq!(
            left, right,
            "identical schedules must produce a byte-identical trader lifecycle"
        );
        assert!(
            left.last_trader_departed_at.is_some(),
            "the horizon should have completed a full trader visit"
        );
    }

    #[test]
    fn trader_visit_is_deterministic_across_identical_worlds_through_a_full_world_tick_horizon() {
        fn build(seed: u32) -> WorldState {
            WorldState {
                world_seed: 777,
                colonies: vec![ColonyRuntime {
                    id: "colony-1".to_owned(),
                    // No cats at all (not merely none alive): `phase_26_empty_colony_reset`
                    // only resets a roster that HAD cats and lost them all, so an empty
                    // roster never resets. A hand-built colony (no world tiles, no water
                    // source, no starter buildings) has no water economy to sustain any
                    // cats over a horizon this long anyway — the point of this test is
                    // full-tick determinism through a trader visit, not survival balance
                    // (the survival proof is a separate, dedicated test).
                    cats: Vec::new(),
                    run_started_at: 0,
                    // `threat_snapshot` derives `colony_age_sec` from `run_started_at` if
                    // it is `> 0`, else `created_at`. Parking `created_at` far in the
                    // future keeps `colony_age_sec` clamped to 0 for this whole horizon,
                    // which keeps threat permanently inside its 8h raid grace window
                    // (`threat::RAID_GRACE_SEC`) — i.e. no raid ever spawns to contend
                    // with a cat-less colony's guaranteed "no alive cats" reset check.
                    // `run_started_at` itself stays `0` since the trader's own schedule
                    // (`phase_36b_trader_lifecycle`) keys off it directly.
                    created_at: i64::MAX / 2,
                    last_tick: 0,
                    test_rng_seed: Some(seed),
                    ..ColonyRuntime::default()
                }],
            }
        }

        let mut left = build(9_001);
        let mut right = build(9_001);

        for step in 1..=60 {
            let now = i64::from(step) * 3_600_000;
            assert_eq!(world_tick(&mut left, now), world_tick(&mut right, now));
        }

        assert_eq!(
            left.colonies[0], right.colonies[0],
            "identical seeds must drive a byte-identical world through a full trader visit"
        );
        assert!(
            left.colonies[0].last_trader_departed_at.is_some(),
            "the horizon should have completed a full trader visit"
        );
    }

    #[test]
    fn trader_feature_is_inert_before_the_first_scheduled_arrival() {
        // A world that never reaches the trader's first scheduled arrival must be
        // byte-identical to a world with no trader lifecycle at all — i.e. the feature
        // does nothing until it's due. 40 one-minute ticks (40 real/game-minutes) is
        // nowhere near the 48-game-hour interval.
        let mut with_phase = trader_test_colony(1);
        let without_phase = with_phase.clone();

        for step in 1..=40 {
            let now = i64::from(step) * 60_000;
            let gate = production_gate(60, now);
            phase_36b_trader_lifecycle(&mut with_phase, gate);
            // `without_phase` never calls the trader phase at all.
            let _ = &without_phase;
        }

        assert_eq!(with_phase, without_phase);
        assert!(with_phase.trader.is_none());
        assert!(with_phase.last_trader_departed_at.is_none());
    }

    #[test]
    fn coin_survives_a_colony_reset() {
        let mut colony = trader_test_colony(1);
        colony.coin = 250.0;
        colony.trader = Some(TraderRuntime {
            id: "trader-1".to_owned(),
            position: position_from_world(trader_gate_world_pos(&colony)),
            destination: None,
            state: trader::TraderState::Trading,
            arrived_at: Some(0),
        });

        reset_run(&mut colony, 5_000, RunResetReason::AllCatsDead);

        assert_eq!(
            colony.coin, 250.0,
            "coin is banked player wealth, not run state"
        );
        assert!(
            colony.trader.is_none(),
            "a mid-visit trader does not survive a collapse cleanly"
        );
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
            phase_23_production(
                &mut colony,
                production_gate(600, i64::from(step) * 600_000),
                123,
            );
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
    fn unfinished_scaffold_is_reassigned_without_paying_break_ground_twice() {
        let mut colony = found_colony(4242, "colony-1", 10_000, 4242);
        colony.resources.planks = 10.0;
        colony.resources.blocks = 10.0;
        let first_builder = colony.cats[0].id.clone();
        queue_job(
            &mut colony,
            10_000,
            JobKind::BuildHouse,
            Some(first_builder.clone()),
            JobMetadata::Construction {
                phase: ConstructionPhase::ConstructHouse,
                building_type: BuildingType::ResearchHut,
                building_id: None,
                site: None,
            },
        );
        phase_14_promote_queued_jobs_and_break_ground(
            &mut colony,
            production_gate(60, 70_000),
            4242,
        );
        let scaffold_id = colony
            .buildings
            .iter()
            .find(|building| building.building_type == BuildingType::ResearchHut)
            .expect("research scaffold")
            .id
            .clone();
        assert_eq!(
            (colony.resources.planks, colony.resources.blocks),
            (8.0, 8.0)
        );

        mark_cat_dead(&mut colony, &first_builder, 80_000);
        let buildings_before = colony.buildings.len();
        phase_14_promote_queued_jobs_and_break_ground(
            &mut colony,
            production_gate(60, 130_000),
            4242,
        );

        assert_eq!(colony.buildings.len(), buildings_before);
        assert_eq!(
            (colony.resources.planks, colony.resources.blocks),
            (8.0, 8.0),
            "resuming an existing scaffold must not charge the 2/2 site cost again"
        );
        assert!(colony.jobs.iter().any(|job| {
            job.status == JobStatus::Active
                && job.assigned_cat.as_deref() != Some(first_builder.as_str())
                && matches!(
                    &job.metadata,
                    JobMetadata::Construction {
                        building_id: Some(building_id),
                        ..
                    } if building_id == &scaffold_id
                )
        }));
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

        phase_23_production(&mut with_pile, production_gate(30, 30_000), 123);
        phase_23_production(&mut no_pile, production_gate(30, 30_000), 123);

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
            phase_23_production(
                &mut colony,
                production_gate(600, i64::from(step) * 600_000),
                123,
            );
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

        phase_23_production(&mut colony, production_gate(1, 5_000), 123);

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
        phase_23_production(&mut colony, production_gate(1, 1_000 + 5_000), 123);
        assert_eq!(colony.stock_ledger.reported.food, 50.0, "still lagging");
        assert_eq!(colony.stock_ledger.last_counted, 1_000);
        assert_eq!(colony.resources.food, 200.0, "resources untouched");

        // Past the interval: recount to the exact current stock.
        phase_23_production(
            &mut colony,
            production_gate(1, 1_000 + crate::ledger::UNSTAFFED_RECOUNT_INTERVAL_MS),
            123,
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
            // The founding benches and construction queue legitimately consume the
            // first work shifts; give the idle colony sixteen game-hours to reach its
            // first frontier expedition.
            for step in 1..=1_000 {
                let now = 10_000 + i64::from(step) * 60_000;
                let _ = world_tick(&mut world, now);
                let colony = &world.colonies[0];
                if colony.jobs.iter().any(|job| job.kind == JobKind::Explore) {
                    saw_explore = true;
                }
                for tile in &colony.revealed_tiles {
                    max_reveal_cheb =
                        max_reveal_cheb.max(cheb_from_anchor(VILLAGE_ANCHOR_TILE, *tile));
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

    // ---- P15: provisional (scout, in-flight) vs committed (permanent) fog reveal ----

    /// Shared movement rig for the P15 reveal-tier unit tests below: a single cat
    /// walking a short, unobstructed 2-tile leg on open ground far from the village
    /// (so nothing is skipped as "inside the village"), mirroring
    /// `movement_advances_toward_destination_and_wears_traversed_tiles`'s proven
    /// arrival timing (2 tiles covered in an 8-second elapsed window).
    fn walking_leg_movement_context(colony: &ColonyRuntime) -> MovementPassContext {
        MovementPassContext {
            movement_seed: movement_seed(123),
            movement_elapsed: 8.0,
            wander_chance: 0.0,
            ring_radius: 4,
            anchor: VILLAGE_ANCHOR_TILE,
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
        }
    }

    fn walking_leg_world_tiles(min_x: i32, max_x: i32) -> BTreeMap<TilePos, WorldTileRuntime> {
        let mut world_tiles = BTreeMap::new();
        for x in (min_x - 1)..=(max_x + 1) {
            for y in 5..=7 {
                world_tiles.insert(pos(x, y), tile(x, y, 0, None));
            }
        }
        world_tiles
    }

    #[test]
    fn regular_cat_reveal_still_commits_directly_to_revealed_tiles() {
        // A non-scout cat (no `provisional_tiles` entry) must keep revealing straight
        // into the permanent `revealed_tiles` set, exactly as before P15 — the
        // two-tier model must not regress ordinary walk-reveal.
        let mut cat = adult_idle_cat("walker", "colony-1");
        cat.position = Position {
            map: MapType::World,
            x: 20.0,
            y: 6.0,
        };
        cat.destination = Some(Position {
            map: MapType::World,
            x: 22.0,
            y: 6.0,
        });
        cat.activity = CatActivity::Traveling;

        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            resources: plentiful_resources(),
            cats: vec![cat],
            world_tiles: walking_leg_world_tiles(20, 22),
            test_rng_seed: Some(123),
            ..ColonyRuntime::default()
        };
        let movement = walking_leg_movement_context(&colony);

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

        assert!(
            colony.provisional_tiles.is_empty(),
            "a plain walking cat must never create a provisional entry"
        );
        assert!(colony.revealed_tiles.contains(&pos(20, 6)));
        assert!(colony.revealed_tiles.contains(&pos(22, 6)));
    }

    #[test]
    fn scouts_walk_reveals_into_provisional_not_committed_while_out() {
        // While an `Explore`-job scout is still traveling, its walk must lift fog
        // only provisionally — `revealed_tiles` (the permanent map) must stay
        // untouched until the scout physically reaches the shrine.
        let mut cat = adult_idle_cat("scout", "colony-1");
        cat.position = Position {
            map: MapType::World,
            x: 20.0,
            y: 6.0,
        };
        cat.destination = Some(Position {
            map: MapType::World,
            x: 22.0,
            y: 6.0,
        });
        cat.activity = CatActivity::Traveling;
        cat.current_task = Some(TaskType::Explore);

        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            resources: plentiful_resources(),
            cats: vec![cat],
            world_tiles: walking_leg_world_tiles(20, 22),
            // Mirrors the registration `phase_15_assign_promoted_job_destinations`
            // performs the moment an `Explore` job's destination is (re-)assigned.
            provisional_tiles: BTreeMap::from([("scout".to_owned(), BTreeSet::new())]),
            test_rng_seed: Some(123),
            ..ColonyRuntime::default()
        };
        let movement = walking_leg_movement_context(&colony);

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

        assert!(
            colony.revealed_tiles.is_empty(),
            "a scout's discoveries must not commit to the permanent map while still out"
        );
        let provisional = colony
            .provisional_tiles
            .get("scout")
            .expect("the scout's provisional entry must survive the walk");
        assert!(provisional.contains(&pos(20, 6)));
        assert!(provisional.contains(&pos(22, 6)));
        // Still outbound, not heading home — commit only fires on a `Returning`
        // arrival (see `scout_provisional_commits_to_revealed_tiles_on_shrine_return_and_clears`),
        // so nothing should have committed here either way.
        assert_ne!(colony.cats[0].activity, CatActivity::Returning);
    }

    #[test]
    fn scout_provisional_commits_to_revealed_tiles_on_shrine_return_and_clears() {
        // `return_assigned_cat` (phase 30) is the only place a completed `Explore`
        // job sends its cat `Returning` toward the shrine anchor. Arriving there
        // must fold the scout's whole provisional set — not just this tick's final
        // steps — into `revealed_tiles`, and drop the now-delivered entry.
        let mut cat = adult_idle_cat("scout", "colony-1");
        cat.position = Position {
            map: MapType::World,
            x: 4.0,
            y: 6.0,
        };
        cat.destination = Some(position_from_world(village_anchor_world(
            VILLAGE_ANCHOR_TILE,
        )));
        cat.activity = CatActivity::Returning;
        cat.current_task = None;

        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            resources: plentiful_resources(),
            cats: vec![cat],
            world_tiles: walking_leg_world_tiles(3, 7),
            // Discoveries banked earlier in the excursion, before this tick's walk.
            provisional_tiles: BTreeMap::from([(
                "scout".to_owned(),
                BTreeSet::from([pos(50, 50), pos(51, 50)]),
            )]),
            test_rng_seed: Some(123),
            ..ColonyRuntime::default()
        };
        let movement = walking_leg_movement_context(&colony);

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

        assert!(
            !colony.provisional_tiles.contains_key("scout"),
            "the delivered scout's provisional entry must be cleared"
        );
        assert!(
            colony.revealed_tiles.contains(&pos(50, 50))
                && colony.revealed_tiles.contains(&pos(51, 50)),
            "discoveries banked earlier in the excursion must commit on shrine arrival"
        );
        assert_eq!(
            colony.cats[0].activity,
            CatActivity::Idle,
            "a Returning cat that reaches its destination goes Idle, same as any other worker"
        );
    }

    #[test]
    fn scout_that_dies_before_returning_never_commits_its_provisional_tiles() {
        // Spec: "If a scout dies/never returns, its provisional tiles do NOT commit
        // (and should eventually clear)." `phase_25b_prune_dead_scout_provisional_tiles`
        // is the cleanup — verify it drops a dead scout's entry without ever folding
        // it into `revealed_tiles`.
        let mut scout = adult_idle_cat("scout", "colony-1");
        scout.death_time = Some(5_000);

        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            cats: vec![scout],
            provisional_tiles: BTreeMap::from([(
                "scout".to_owned(),
                BTreeSet::from([pos(80, 80)]),
            )]),
            ..ColonyRuntime::default()
        };

        phase_25b_prune_dead_scout_provisional_tiles(
            &mut colony,
            TickGate {
                elapsed_sec: 60,
                processed_through: 65_000,
                minute_rolled: false,
                previous_water: 0,
            },
        );

        assert!(
            colony.provisional_tiles.is_empty(),
            "a dead scout's entry must be pruned"
        );
        assert!(
            !colony.revealed_tiles.contains(&pos(80, 80)),
            "a dead scout's tentative discoveries must never become permanent"
        );
    }

    #[test]
    fn provisional_tiles_are_deterministic_across_identical_runs() {
        // Two identical seeded runs must produce byte-identical `provisional_tiles`
        // state at every tick — not just after every scout has come home — so the
        // dim/tentative fog the client renders mid-excursion is itself deterministic.
        let run = || {
            let mut world = new_world(4242);
            world
                .colonies
                .push(found_colony(world.world_seed, "colony-1", 10_000, 4242));
            let mut trace: Vec<BTreeMap<CatId, BTreeSet<TilePos>>> = Vec::new();
            for step in 1..=1_000 {
                let now = 10_000 + i64::from(step) * 60_000;
                let _ = world_tick(&mut world, now);
                trace.push(world.colonies[0].provisional_tiles.clone());
            }
            trace
        };
        let left = run();
        let right = run();

        assert_eq!(
            left, right,
            "provisional fog state must be deterministic across identical runs"
        );
        assert!(
            left.iter().any(|snapshot| !snapshot.is_empty()),
            "expected at least one tick with a scout mid-excursion (provisional fog present)"
        );
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

    /// Seed a founded colony with the `mountaineering`/`smelting` nodes owned, a staffed
    /// Smelter bench, and a bank of ore — the minimal setup that actually exercises the
    /// P17/P19 ore→metal chain's new mutable state (`resources.ore`/`metal`,
    /// `metal_forge_progress`, the Smelter's `production_progress`) instead of leaving
    /// it permanently at rest.
    fn ore_chain_colony(world_seed: u32) -> WorldState {
        let mut world = new_world(world_seed);
        let mut colony = found_colony(world.world_seed, "colony-1", 1_000, 42);
        colony
            .upgrade_tree
            .owned_node_ids
            .push(MOUNTAINEERING_NODE_ID.to_owned());
        colony
            .upgrade_tree
            .owned_node_ids
            .push(crate::upgrade_tree::SMELTING_NODE_ID.to_owned());
        colony.resources.ore = 200.0;
        // Assign a smelter worker directly rather than relying on the leader's
        // idle-surplus mop-up to eventually claim the bench — the founding roster's
        // cats are mostly busy on hunt/water in the opening ticks, and this test cares
        // about the ore→metal chain's determinism, not the labour market's emergent
        // staffing timing.
        let smelter_cat_id = colony.cats[0].id.clone();
        colony.buildings.push(BuildingRuntime {
            id: "smelter-1".to_owned(),
            building_type: BuildingType::Smelter,
            level: 1,
            position: TilePos { x: 3, y: 3 },
            is_complete: true,
            construction_progress: 100,
            production_progress: 0.0,
            assigned_cat: Some(smelter_cat_id),
        });
        world.colonies.push(colony);
        world
    }

    #[test]
    fn ore_to_metal_chain_is_deterministic_across_identical_runs() {
        let mut left = ore_chain_colony(555);
        let mut right = ore_chain_colony(555);

        for step in 1..=40 {
            let now = 1_000 + i64::from(step) * 60_000;
            let _ = world_tick(&mut left, now);
            let _ = world_tick(&mut right, now);
        }

        assert_eq!(
            left.colonies[0].resources, right.colonies[0].resources,
            "ore/metal (and everything they feed) must be byte-identical across identical seeded runs"
        );
        assert_eq!(
            left.colonies[0].metal_forge_progress,
            right.colonies[0].metal_forge_progress
        );
        // The chain must actually have run (not stayed inertly at its seeded values) —
        // otherwise this test would trivially pass without exercising anything new.
        assert!(
            left.colonies[0].resources.metal > 0.0,
            "the smelter should have refined some of the seeded ore into metal by now"
        );
        assert!(
            left.colonies[0].resources.ore < 200.0,
            "the seeded ore bank should have been drawn down"
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

    /// Comfort harness: model a colony that has solved its food/water economy (a full-ish
    /// larder every tick, no player actions) so the cat-research mechanic is exercised in
    /// isolation from the orthogonal founding-food balance. Keeps food/water at 0.6 of cap —
    /// comfortably over the [`is_research_comfortable`] per-capita bar without slamming the
    /// larder to the cap (which over-breeds into an old-age collapse). Deterministic;
    /// touches only food/water.
    #[cfg(test)]
    fn keep_comfortable(colony: &mut ColonyRuntime) {
        let caps = storage_caps(colony);
        colony.resources.food = colony.resources.food.max(caps.food * 0.6);
        colony.resources.water = colony.resources.water.max(caps.water * 0.6);
    }

    /// AUTONOMOUS TREE-ADVANCE guardrail — the headline proof that the cat-research path is
    /// no longer dormant: with no player input, a comfortable colony commissions and builds a
    /// research hut, staffs it, accrues research points, and auto-unlocks an upgrade-tree node
    /// on its own.
    ///
    /// The end-to-end pieces are covered thus: the build/staff/accrue wiring runs live here
    /// from a founded colony; [`staffed_research_hut_accrues_points_and_auto_unlocks_the_cheapest_nodes`]
    /// proves a full week of accrual crosses several node thresholds; and
    /// [`research_staffing_and_commission_respect_the_comfort_gate`] proves the survival gate.
    /// A node costs ~42 game-hours of a single scholar (20 pts/researcher/week) — more than a cat
    /// lifetimes — so an unaided founding colony's *own* long-horizon population balance (a
    /// separate, tracked survival-pacing concern) would otherwise dominate this test. We
    /// therefore give the tree a head start of prior study (research points persist across
    /// run resets by design), so the run proves the autonomous **build → staff → accrue →
    /// cross-threshold → own** path within a bounded, population-safe horizon. Multi-seed;
    /// every colony must end owning at least one researched node without a single player action.
    #[test]
    fn an_unattended_comfortable_colony_advances_the_research_tree() {
        for seed in [7u32, 4242, 1234] {
            let mut world = new_world(seed);
            let mut colony = found_colony(world.world_seed, "colony-1", 10_000, seed);
            // Prior study already banked most of the cost-5 root (persists across resets).
            colony.upgrade_tree.research_points = 4.5;
            assert!(
                colony.upgrade_tree.owned_node_ids.is_empty(),
                "seed {seed}: the colony must start with no researched nodes"
            );
            world.colonies.push(colony);

            let mut built_and_staffed = false;
            // 4000 one-minute steps ≈ 67 game-hours: the first 30 are the research
            // establishment window (no hut is commissioned before it), the rest cover
            // build + staff + the final accrual to the head-started root node.
            for step in 1..=4_000i64 {
                keep_comfortable(&mut world.colonies[0]);
                let now = 10_000 + step * 60_000;
                let _ = world_tick(&mut world, now);
                if research_workforce(&world.colonies[0]) > 0.0 {
                    built_and_staffed = true;
                }
                if !world.colonies[0].upgrade_tree.owned_node_ids.is_empty() {
                    break;
                }
            }

            let colony = &world.colonies[0];
            assert!(
                built_and_staffed,
                "seed {seed}: the colony never autonomously built and staffed a research hut"
            );
            assert!(
                !colony.upgrade_tree.owned_node_ids.is_empty(),
                "seed {seed}: the research tree never advanced to an owned node — the cat path is dormant"
            );
        }
    }

    /// ORGANIC research-engagement guardrail (playtest 2026-07-13): the research-comfort
    /// bar must be reachable by the colony's OWN unattended economy — not only under the
    /// [`keep_comfortable`] harness the tree-advance proof uses. Under the old
    /// capacity-ratio bar (food/water ≥ 0.5 × capacity) that never happened: the
    /// per-capita breeding gate converts any larder above ~2.5 food/cat into kittens
    /// whose consumption pulls the stock back down, pinning food at ~0.1–0.3 of capacity
    /// indefinitely (instrumented via [`instrument_food_trajectory`]: 5 seeds × 200
    /// unattended game-hours ended with 0.0 research points and 0 owned nodes at every
    /// cadence — research was dormant in real play). With the per-capita bar
    /// ([`is_research_comfortable`]) the same unattended economy must commission, build,
    /// and staff a research hut and bank research points, with no player action.
    #[test]
    fn an_unattended_colony_organically_engages_research() {
        for seed in [7u32, 2024] {
            let mut world = new_world(seed);
            world
                .colonies
                .push(found_colony(world.world_seed, "colony-1", 10_000, seed));

            let mut ever_staffed = false;
            // 4500 one-minute steps ≈ 75 game-hours: 30 of establishment window, then
            // organic comfort → commission → build → staff → accrue.
            for step in 1..=4_500i64 {
                let now = 10_000 + step * 60_000;
                let _ = world_tick(&mut world, now);
                if research_workforce(&world.colonies[0]) > 0.0 {
                    ever_staffed = true;
                }
            }

            assert!(
                ever_staffed,
                "seed {seed}: the colony's own economy never staffed a research hut — the \
                 comfort bar is unreachable in unattended play"
            );
            assert!(
                world.colonies[0].upgrade_tree.research_points > 0.0,
                "seed {seed}: no research points banked over 75 unattended game-hours — \
                 research is dormant in real play"
            );
        }
    }

    /// Determinism twin for the autonomous tree-advance: two byte-identical runs must reach
    /// the identical owned-node set and research-point total. Guards that research staffing
    /// and point accrual draw no unseeded RNG.
    #[test]
    fn autonomous_research_advance_is_deterministic() {
        let run = || {
            let mut world = new_world(7);
            let mut colony = found_colony(world.world_seed, "colony-1", 10_000, 7);
            colony.upgrade_tree.research_points = 4.5;
            world.colonies.push(colony);
            for step in 1..=1_500i64 {
                keep_comfortable(&mut world.colonies[0]);
                let now = 10_000 + step * 60_000;
                let _ = world_tick(&mut world, now);
            }
            let tree = &world.colonies[0].upgrade_tree;
            (tree.owned_node_ids.clone(), tree.research_points.to_bits())
        };
        assert_eq!(run(), run());
    }

    #[derive(Debug, Clone, PartialEq)]
    struct LongHorizonEconomy {
        resets: u32,
        reset_events: Vec<(i64, RunResetReason)>,
        tithes: usize,
        offerings: usize,
        offering_jobs: usize,
        tools: f64,
        blessings: f64,
        research_nodes: Vec<String>,
        research_points: f64,
        research_commissions: usize,
        research_huts: usize,
        completed_research_huts: usize,
        staffed_research_huts: usize,
        research_build_jobs: Vec<(JobStatus, Option<String>)>,
        research_hut_progress: Vec<u8>,
        max_materials: f64,
        max_materials_after_establishment: f64,
        min_food: f64,
        min_water: f64,
        first_research_hour: Option<i64>,
        population: usize,
    }

    /// Exercise every formerly dormant founding-economy output for 200 game-hours at
    /// the deliberately harsh five-minute proxy cadence. Unlike the removed flat
    /// devotion bypass, every blessing here must come from a visible active ritual:
    /// food/refined tithes or cat-carried material offerings.
    fn run_long_horizon_economy(seed: u32, game_hours: i64) -> LongHorizonEconomy {
        const TICK_MS: i64 = 5 * 60_000;
        let mut world = new_world(seed);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 10_000, seed));
        let mut resets = 0;
        let mut reset_events = Vec::new();
        let mut first_research_hour = None;
        let mut max_materials = 0.0f64;
        let mut max_materials_after_establishment = 0.0f64;
        let mut min_food = f64::MAX;
        let mut min_water = f64::MAX;
        let mut seen_tithes = std::collections::HashSet::new();
        let mut seen_offerings = std::collections::HashSet::new();
        let mut seen_offering_jobs = std::collections::HashSet::new();
        let mut seen_research_commissions = std::collections::HashSet::new();
        for step in 1..=game_hours * 60 / 5 {
            let reports = world_tick(&mut world, 10_000 + step * TICK_MS);
            if let Some(reason) = reports[0].reset_reason {
                resets += 1;
                reset_events.push((step * 5 / 60, reason));
            }
            if first_research_hour.is_none()
                && !world.colonies[0].upgrade_tree.owned_node_ids.is_empty()
            {
                first_research_hour = Some(step * 5 / 60);
            }
            max_materials = max_materials.max(world.colonies[0].resources.materials);
            min_food = min_food.min(world.colonies[0].resources.food);
            min_water = min_water.min(world.colonies[0].resources.water);
            if step * 5 >= 30 * 60 {
                max_materials_after_establishment =
                    max_materials_after_establishment.max(world.colonies[0].resources.materials);
            }
            for event in &world.colonies[0].events {
                if event.kind == EventKind::Tithe {
                    seen_tithes.insert(event.id.clone());
                } else if event.kind == EventKind::Offering {
                    seen_offerings.insert(event.id.clone());
                } else if event.kind == EventKind::ResearchHutCommissioned {
                    seen_research_commissions.insert(event.id.clone());
                }
            }
            for job in &world.colonies[0].jobs {
                if job.kind == JobKind::CarryOffering {
                    seen_offering_jobs.insert(job.id.clone());
                }
            }
        }
        let colony = &world.colonies[0];
        LongHorizonEconomy {
            resets,
            reset_events,
            tithes: seen_tithes.len(),
            offerings: seen_offerings.len(),
            offering_jobs: seen_offering_jobs.len(),
            tools: colony.resources.tools,
            blessings: colony.global_upgrade_points,
            research_nodes: colony.upgrade_tree.owned_node_ids.clone(),
            research_points: colony.upgrade_tree.research_points,
            research_commissions: seen_research_commissions.len(),
            research_huts: colony
                .buildings
                .iter()
                .filter(|building| building.building_type == BuildingType::ResearchHut)
                .count(),
            completed_research_huts: colony
                .buildings
                .iter()
                .filter(|building| {
                    building.building_type == BuildingType::ResearchHut && building.is_complete
                })
                .count(),
            staffed_research_huts: colony
                .buildings
                .iter()
                .filter(|building| {
                    building.building_type == BuildingType::ResearchHut
                        && building.is_complete
                        && building.assigned_cat.is_some()
                })
                .count(),
            research_build_jobs: colony
                .jobs
                .iter()
                .filter(|job| {
                    job.kind == JobKind::BuildHouse
                        && job_building_type(job) == Some(BuildingType::ResearchHut)
                })
                .map(|job| (job.status, job.assigned_cat.clone()))
                .collect(),
            research_hut_progress: colony
                .buildings
                .iter()
                .filter(|building| building.building_type == BuildingType::ResearchHut)
                .map(|building| building.construction_progress)
                .collect(),
            max_materials,
            max_materials_after_establishment,
            min_food,
            min_water,
            first_research_hour,
            population: alive_cats(&colony.cats).count(),
        }
    }

    #[test]
    fn founding_economy_faucets_are_active_across_a_two_hundred_hour_campaign() {
        let mut offering_seeds = 0;
        for seed in [7u32, 555, 2024, 42, 99] {
            let result = run_long_horizon_economy(seed, 200);
            eprintln!("seed {seed}: {result:?}");
            assert_eq!(result.resets, 0, "seed {seed}: {result:?}");
            offering_seeds += usize::from(result.offerings > 0);
            assert!(
                result.tithes + result.offerings > 0 && result.blessings > 0.0,
                "seed {seed}: active shrine economy stayed dormant: {result:?}"
            );
            assert!(
                result.tools > 0.0,
                "seed {seed}: tool chain stayed dormant: {result:?}"
            );
            assert!(
                result.research_points > 0.0 || !result.research_nodes.is_empty(),
                "seed {seed}: research stayed dormant: {result:?}"
            );
            assert!(
                result.population > 0,
                "seed {seed}: colony ended extinct: {result:?}"
            );
        }
        assert!(
            offering_seeds >= 2,
            "offerings were not organically reachable across enough seeds ({offering_seeds}/5)"
        );
    }

    #[test]
    fn long_horizon_economy_is_deterministic_for_identical_seeds() {
        assert_eq!(
            run_long_horizon_economy(7, 200),
            run_long_horizon_economy(7, 200)
        );
    }

    /// Attach a completed research hut, staffed by `scholar`, to `colony`.
    #[cfg(test)]
    fn attach_staffed_research_hut(colony: &mut ColonyRuntime, scholar: &str) {
        colony.buildings.push(BuildingRuntime {
            id: "research-hut-test".to_owned(),
            building_type: BuildingType::ResearchHut,
            level: 1,
            position: TilePos {
                x: colony.anchor.x,
                y: colony.anchor.y,
            },
            is_complete: true,
            construction_progress: 100,
            assigned_cat: Some(scholar.to_owned()),
            ..BuildingRuntime::default()
        });
    }

    /// Cat-research accrual wiring ([`research_workforce`] + [`phase_24_research`]). Before
    /// this fix `research_workforce` was hard-coded `0.0`, so a staffed hut banked nothing
    /// and the whole tech tree sat dormant in unattended play. A completed hut staffed by a
    /// living cat must now count as one researcher, and a game-week of accrual (20
    /// pts/researcher/week) must auto-unlock the cheapest-first nodes — proving the tree can
    /// advance on the cat path alone. Deterministic: a flat function of elapsed game-seconds,
    /// no RNG.
    #[test]
    fn staffed_research_hut_accrues_points_and_auto_unlocks_the_cheapest_nodes() {
        let mut colony = found_colony(7, "colony-1", 10_000, 7);
        colony.upgrade_tree = crate::upgrade_tree::create_upgrade_tree_state();
        let scholar = colony.cats[0].id.clone();

        // Unstaffed to start: the workforce is zero and no accrual happens.
        assert_eq!(
            research_workforce(&colony),
            0.0,
            "a colony with no staffed hut must yield zero research workforce"
        );

        attach_staffed_research_hut(&mut colony, &scholar);
        assert!(
            (research_workforce(&colony) - 1.0).abs() < 1e-9,
            "one living scholar in a completed hut is one researcher"
        );

        // One game-week of a single researcher banks ~20 points, enough to auto-unlock the
        // cost-5 root (`research_hut`) and then the next cheapest node (`basic_tools`, 5).
        let week = crate::upgrade_tree::WEEK_SECONDS as i64;
        let gate = TickGate {
            elapsed_sec: week,
            processed_through: colony.last_tick + week * 1_000,
            minute_rolled: true,
            previous_water: colony.resources.water.to_bits(),
        };
        phase_24_research(&mut colony, gate);

        assert!(
            colony
                .upgrade_tree
                .owned_node_ids
                .contains(&"research_hut".to_owned()),
            "a game-week of research must unlock the cost-5 root node, got {:?}",
            colony.upgrade_tree.owned_node_ids,
        );
        assert!(
            colony.upgrade_tree.owned_node_ids.len() >= 2,
            "cheapest-first auto-unlock must claim more than the root from ~20 banked points, got {:?}",
            colony.upgrade_tree.owned_node_ids,
        );

        // Unstaffing the hut drops the workforce back to zero (dead/absent scholar guard).
        for building in &mut colony.buildings {
            if building.building_type == BuildingType::ResearchHut {
                building.assigned_cat = None;
            }
        }
        assert_eq!(research_workforce(&colony), 0.0);
    }

    /// Survival guardrail for the cat-research path: staffing a research hut must respect the
    /// comfort gate the `AssignResearch` labour goal already carries, so a *starving* colony
    /// never pulls a mouth off survival work to study. A completed, unstaffed hut in a colony
    /// held below the [`is_research_comfortable`] per-capita bar on food/water stays unstaffed
    /// and commissions no new hut; the same colony made comfortable staffs it. Deterministic
    /// (short bounded run).
    #[test]
    fn research_staffing_and_commission_respect_the_comfort_gate() {
        // Starving: food/water pinned near zero every tick, so no comfortable ticks pass.
        // Both halves age the run past the research establishment window so the assert
        // exercises the comfort gate itself, not the founding-window guard.
        let mut world = new_world(7);
        let mut colony = found_colony(world.world_seed, "colony-1", 10_000, 7);
        colony.run_started_at = 10_000 - RESEARCH_ESTABLISHMENT_MS;
        let scholar = colony.cats[0].id.clone();
        attach_staffed_research_hut(&mut colony, &scholar);
        // Free the scholar so only the leader (comfort-gated) could re-staff it.
        for building in &mut colony.buildings {
            if building.building_type == BuildingType::ResearchHut {
                building.assigned_cat = None;
            }
        }
        world.colonies.push(colony);
        for step in 1..=20 {
            // Re-starve before each tick so the colony can never read as comfortable.
            let c = &mut world.colonies[0];
            c.resources.food = 1.0;
            c.resources.water = 1.0;
            let now = 10_000 + i64::from(step) * 60_000;
            let _ = world_tick(&mut world, now);
            assert_eq!(
                research_workforce(&world.colonies[0]),
                0.0,
                "tick {step}: a starving colony must not staff its research hut",
            );
        }
        assert!(
            world.colonies[0]
                .buildings
                .iter()
                .filter(|b| b.building_type == BuildingType::ResearchHut)
                .count()
                == 1,
            "a starving colony must not commission a second research hut",
        );

        // Comfortable: food/water pinned at capacity, so the leader staffs the idle hut.
        let mut comfy = new_world(7);
        let mut colony = found_colony(comfy.world_seed, "colony-1", 10_000, 7);
        colony.run_started_at = 10_000 - RESEARCH_ESTABLISHMENT_MS;
        let scholar = colony.cats[0].id.clone();
        attach_staffed_research_hut(&mut colony, &scholar);
        for building in &mut colony.buildings {
            if building.building_type == BuildingType::ResearchHut {
                building.assigned_cat = None;
            }
        }
        comfy.colonies.push(colony);
        let mut staffed = false;
        for step in 1..=40 {
            let c = &mut comfy.colonies[0];
            let caps = storage_caps(c);
            c.resources.food = caps.food;
            c.resources.water = caps.water;
            let now = 10_000 + i64::from(step) * 60_000;
            let _ = world_tick(&mut comfy, now);
            if research_workforce(&comfy.colonies[0]) > 0.0 {
                staffed = true;
                break;
            }
        }
        assert!(
            staffed,
            "a comfortable colony must staff its idle research hut within a few ticks",
        );
    }

    /// Attach a completed school, staffed by `scholar`, to `colony`.
    #[cfg(test)]
    fn attach_staffed_school(colony: &mut ColonyRuntime, scholar: &str) {
        colony.buildings.push(BuildingRuntime {
            id: "school-test".to_owned(),
            building_type: BuildingType::School,
            level: 1,
            position: TilePos {
                x: colony.anchor.x,
                y: colony.anchor.y,
            },
            is_complete: true,
            construction_progress: 100,
            assigned_cat: Some(scholar.to_owned()),
            ..BuildingRuntime::default()
        });
    }

    /// The school is a second staffed research building (unlocked by the "school" upgrade
    /// node): a completed, staffed school must count toward [`research_workforce`] exactly
    /// like a research hut, and the two stack — a colony with one of each fields two
    /// researchers. Mirrors
    /// [`staffed_research_hut_accrues_points_and_auto_unlocks_the_cheapest_nodes`], swapping
    /// the hut for a school (and covering the stacked case the hut-only test doesn't).
    #[test]
    fn staffed_school_contributes_research_workforce_and_accrues_points() {
        let mut colony = found_colony(7, "colony-1", 10_000, 7);
        colony.upgrade_tree = crate::upgrade_tree::create_upgrade_tree_state();
        let scholar = colony.cats[0].id.clone();

        assert_eq!(
            research_workforce(&colony),
            0.0,
            "a colony with no staffed research building must yield zero research workforce"
        );

        attach_staffed_school(&mut colony, &scholar);
        assert!(
            (research_workforce(&colony) - 1.0).abs() < 1e-9,
            "one living scholar in a completed school is one researcher"
        );

        // A research hut staffed by a second scholar stacks on top of the school.
        let second_scholar = colony.cats[1].id.clone();
        attach_staffed_research_hut(&mut colony, &second_scholar);
        assert!(
            (research_workforce(&colony) - 2.0).abs() < 1e-9,
            "a staffed school and a staffed research hut must both contribute workforce"
        );

        // One game-week of two researchers banks ~20 points, comfortably auto-unlocking the
        // cost-5 root node (`research_hut`) and beyond.
        let week = crate::upgrade_tree::WEEK_SECONDS as i64;
        let gate = TickGate {
            elapsed_sec: week,
            processed_through: colony.last_tick + week * 1_000,
            minute_rolled: true,
            previous_water: colony.resources.water.to_bits(),
        };
        phase_24_research(&mut colony, gate);

        assert!(
            colony
                .upgrade_tree
                .owned_node_ids
                .contains(&"research_hut".to_owned()),
            "a game-week of research from a staffed school must unlock the cost-5 root node, \
             got {:?}",
            colony.upgrade_tree.owned_node_ids,
        );

        // Unstaffing the school (but leaving the hut staffed) drops the workforce by exactly
        // one — the dead/absent scholar guard applies per-building, not colony-wide.
        for building in &mut colony.buildings {
            if building.building_type == BuildingType::School {
                building.assigned_cat = None;
            }
        }
        assert!(
            (research_workforce(&colony) - 1.0).abs() < 1e-9,
            "unstaffing the school must leave only the research hut's researcher"
        );
    }

    /// Live-cadence founding survival guardrail (bug: founding death spiral).
    ///
    /// The long-horizon proof [`founded_colony_thrives_over_a_long_horizon_with_the_small_start`]
    /// advances `now` in 60_000 ms (one game-minute) jumps, so it only covers ~8.3 game-hours
    /// in 500 ticks — short enough that the founding water reservoir never bleeds low enough to
    /// exercise the leader's water-fetch loop. The live server advances `now` ~1000 ms per tick
    /// (`WORKER_TICK_MS`, default `test_time_scale`), so `gate.minute_rolled` is false on most
    /// ticks and the leader director's per-tick water projection signal is tiny — water is held
    /// only by the deficit curve once the reservoir actually runs down. A regression where the
    /// leader never dispatches `FetchWater` (e.g. `has_water_site` disagreeing with the
    /// `water_sites_near_village` site-finder) therefore hides from the 60 s proof but bleeds a
    /// live colony's water to a critical collapse.
    ///
    /// This test ticks at the true 1 s live cadence with the *default* time scale and resource
    /// decay, so the founding water economy runs at its real balance. To reach the water-standup
    /// window inside a bounded tick budget (a faithful 1 s run to the ~15 game-hour bleed-out
    /// would be tens of thousands of ticks) it founds each colony with a deliberately lean water
    /// reservoir; food and hunting stay at their proven default balance. A working founding
    /// economy must stand up its water loop, refill the reservoir, and never reset. Fails before
    /// the `has_water_site` fix (no fetch ever dispatched, water bleeds to an
    /// `UnattendedCollapse`/`AllCatsDead` reset); passes after.
    #[test]
    fn founding_colony_stands_up_its_water_loop_at_the_live_one_second_cadence() {
        for seed in [1234u32, 42, 7] {
            let mut world = new_world(seed);
            let mut colony = found_colony(world.world_seed, "colony-1", 10_000, seed);
            // Lean founding reservoir so the water-standup crisis is reached in a bounded tick
            // budget; everything else (food, decay, time scale) stays at the live default.
            colony.resources.water = 10.0;
            reconcile_colony_stockpiles(&mut colony);
            world.colonies.push(colony);

            let starting_water = world.colonies[0].resources.water;
            let mut dispatched_fetch = false;
            let mut water_max = starting_water;
            let mut min_population = usize::MAX;
            for step in 1..=3_000i64 {
                let now = 10_000 + step * 1_000; // 1 s live cadence, default time scale
                let reports = world_tick(&mut world, now);
                let colony = &world.colonies[0];

                assert_eq!(
                    reports[0].reset_reason, None,
                    "seed {seed} tick {step}: founding colony reset ({:?}) — the water loop never stood up",
                    reports[0].reset_reason,
                );

                if colony
                    .jobs
                    .iter()
                    .any(|job| job.kind == JobKind::FetchWater)
                {
                    dispatched_fetch = true;
                }
                water_max = water_max.max(colony.resources.water);
                min_population = min_population.min(alive_cats(&colony.cats).count());
            }

            let colony = &world.colonies[0];
            assert_ne!(
                colony.status,
                ColonyStatus::Dead,
                "seed {seed}: colony died"
            );
            assert!(
                dispatched_fetch,
                "seed {seed}: the leader never dispatched a FetchWater job — water was never replenished"
            );
            assert!(
                water_max >= 60.0,
                "seed {seed}: water never recovered (peaked at {water_max:.1}) — the fetch loop is not delivering"
            );
            assert!(
                colony.resources.water > starting_water,
                "seed {seed}: water ended at {:.1}, no better than the lean start — the loop is bleeding, not standing up",
                colony.resources.water,
            );
            assert!(
                min_population >= STARTER_CAT_COUNT,
                "seed {seed}: population dipped to {min_population} — a cat died during founding standup"
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
                event.kind == EventKind::DehydrationCrisis
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
            colony.events.iter().any(|event| event.kind
                == EventKind::Death(DeathCause::Dehydration)
                && event.message == "Poppy died from dehydration."),
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
                event.kind == EventKind::DehydrationRecovery
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
            colony.events.iter().any(|event| event.kind
                == EventKind::Death(DeathCause::Starvation)
                && event.message == "Poppy died from starvation."),
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
            source_gather_spot: None,
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

        // This fixture is specifically a no-supply depletion. A normal founding
        // village now has a guaranteed spring and can draw an emergency bucket, so
        // remove every water tile from both twins before forcing the reservoir dry.
        for world in [&mut left, &mut right] {
            let water_tiles = world.colonies[0]
                .world_tiles
                .iter()
                .filter_map(|(pos, tile)| tile_has_water(Some(tile)).then_some(*pos))
                .collect::<Vec<_>>();
            for pos in water_tiles {
                clear_water_tile(&mut world.colonies[0], pos);
            }
            assert!(!has_water_site(&world.colonies[0]));
        }

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
            source_gather_spot: None,
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
            colony
                .events
                .iter()
                .any(|event| event.kind == EventKind::Death(DeathCause::OldAge)
                    && event.message == "Poppy died peacefully of old age."),
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
            .find(|event| event.kind == EventKind::Birth);
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
                .any(|event| event.kind == EventKind::Birth),
            "at least one birth event should have been logged"
        );
    }

    /// Governance reachability guardrail (governance review). A headless colony has no
    /// players and casts no ballots, yet the term-election machinery must still run itself:
    /// a scheduled election opens on the first tick (no prior election, so `election_due`
    /// fires), stays open for the ~30-minute window, then resolves and seats a winner from
    /// the zero-vote fallback (the highest-leadership candidate) — all without player input.
    /// This proves phase 9 is not a dead-end: the election opens exactly once (no per-tick
    /// spam), resolves autonomously, and seats a live leader, logging both the open and the
    /// result. A regression where the election opens but never resolves, or never opens,
    /// trips this guard.
    #[test]
    fn headless_colony_runs_a_term_election_to_a_seated_winner_without_players() {
        let mut world = new_world(1234);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 10_000, 1234));

        // 40 game-minutes clears the 30-minute election window with margin.
        for step in 1..=40 {
            let now = 10_000 + i64::from(step) * 60_000;
            let _ = world_tick(&mut world, now);
        }

        let colony = &world.colonies[0];
        let scheduled = colony
            .elections
            .iter()
            .filter(|election| election.kind == ElectionKind::Scheduled)
            .collect::<Vec<_>>();

        assert_eq!(
            scheduled.len(),
            1,
            "exactly one scheduled term election should have opened (no per-tick spam), got {}",
            scheduled.len()
        );
        let election = scheduled[0];
        assert!(
            election.resolved_at.is_some(),
            "the term election never resolved autonomously"
        );
        let winner = election
            .winner_cat_id
            .as_ref()
            .expect("a resolved term election seats a winner even with zero ballots");
        assert_eq!(
            colony.leader_id.as_ref(),
            Some(winner),
            "the seated leader must be the election winner"
        );
        assert!(
            alive_cats(&colony.cats).any(|cat| &cat.id == winner),
            "the seated winner must be a living cat"
        );
        assert!(
            colony.votes.is_empty(),
            "headless play casts no ballots — the election resolved without player input"
        );
        // Both the opening announcement and the result are logged.
        assert!(
            colony
                .events
                .iter()
                .filter(|event| matches!(event.kind, EventKind::Election(_)))
                .count()
                >= 2,
            "expected both an election-opened and an election-resolved event"
        );
    }

    /// Governance reachability guardrail: a leader death must not leave the seat
    /// permanently empty. When the seated leader dies, phase 4's interim pick refills the
    /// seat on the very next tick with the highest-leadership living cat — proving the
    /// "interim pick only when the seat is empty" rule (CLAUDE.md) is wired and reachable.
    #[test]
    fn leader_death_refills_the_seat_with_an_interim_pick() {
        let mut world = new_world(1234);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 10_000, 1234));

        // Seat a leader.
        let _ = world_tick(&mut world, 70_000);
        let dead_leader = world.colonies[0]
            .leader_id
            .clone()
            .expect("a leader is seated after the first tick");

        // Kill the seated leader.
        let leader_index = world.colonies[0]
            .cats
            .iter()
            .position(|cat| cat.id == dead_leader)
            .expect("the seated leader exists in the roster");
        world.colonies[0].cats[leader_index].death_time = Some(70_000);

        // Next tick must refill the empty seat.
        let _ = world_tick(&mut world, 130_000);

        let colony = &world.colonies[0];
        let new_leader = colony
            .leader_id
            .clone()
            .expect("the seat was refilled after the leader died");
        assert_ne!(
            new_leader, dead_leader,
            "the dead leader must not stay seated"
        );
        assert!(
            alive_cats(&colony.cats).any(|cat| cat.id == new_leader),
            "the interim leader must be a living cat"
        );
    }

    /// Zones reachability guardrail (governance/zones review). Before this fix the Rust
    /// hunt-assignment phase picked a food tile uniformly and never consulted player zones,
    /// so painted gather/avoid rects were stored but inert — a dead-end. [`pick_zoned_food_tile`]
    /// (the exact function phase 15 now calls) must measurably steer the pick: with no zones
    /// it collapses to the same uniform `floor(roll * len)` index (byte-identical baseline),
    /// an avoid rect excludes its tile across the whole roll space, and a gather rect makes
    /// its tile roughly twice as likely.
    #[test]
    fn zones_measurably_steer_hunt_food_tile_selection() {
        let a = WorldPos { x: 12.0, y: 6.0 };
        let b = WorldPos { x: 12.0, y: 12.0 };
        let tiles = [a, b];
        let rect_over = |p: WorldPos| ZoneRect {
            x1: p.x as i32,
            y1: p.y as i32,
            x2: p.x as i32,
            y2: p.y as i32,
        };

        // No zones: uniform pick — byte-identical to the old `hunt_destination` indexing.
        assert_eq!(pick_zoned_food_tile(&tiles, &[], 0.0), Some(a));
        assert_eq!(pick_zoned_food_tile(&tiles, &[], 0.99), Some(b));

        // Avoid over `a`: `a` is never chosen anywhere in the roll space; the hunt is steered
        // to `b` even at roll 0 where the uniform pick would have chosen `a`.
        let avoid_a = Zone {
            rect: rect_over(a),
            kind: ZoneKind::Avoid,
        };
        for roll in [0.0, 0.25, 0.5, 0.75, 0.999] {
            assert_eq!(
                pick_zoned_food_tile(&tiles, &[avoid_a], roll),
                Some(b),
                "avoid zone must exclude tile a at roll {roll}"
            );
        }

        // Gather over `a`: weighted pool is [a, a, b], so `a` wins the lower two-thirds of the
        // roll space — including roll 0.6, which the uniform pick sends to `b`.
        let gather_a = Zone {
            rect: rect_over(a),
            kind: ZoneKind::Gather,
        };
        assert_eq!(pick_zoned_food_tile(&tiles, &[gather_a], 0.6), Some(a));
        assert_eq!(pick_zoned_food_tile(&tiles, &[], 0.6), Some(b));
        // Measurable bias: over an even sweep of the roll space the gather tile is picked
        // about twice as often as the plain tile.
        let mut a_hits = 0;
        let mut b_hits = 0;
        for i in 0..30 {
            let roll = f64::from(i) / 30.0;
            match pick_zoned_food_tile(&tiles, &[gather_a], roll) {
                Some(tile) if tile == a => a_hits += 1,
                Some(tile) if tile == b => b_hits += 1,
                other => panic!("unexpected pick {other:?}"),
            }
        }
        assert_eq!((a_hits, b_hits), (20, 10), "gather tile favoured ~2:1");
    }

    /// Zones reachability guardrail, end-to-end: a painted avoid zone actually changes the
    /// hunt site a driven `world_tick` assigns. A queued hunt over two explored food tiles is
    /// promoted and given a destination in phase 15; re-running with an avoid rect over the
    /// tile the unzoned run chose steers the hunt to the other tile — proving zones reach the
    /// live assignment path, not just the pure helper.
    #[test]
    fn avoid_zone_changes_the_hunt_site_assigned_by_a_driven_tick() {
        // Two explored food tiles at Chebyshev radius > 4 from the (6,6) anchor.
        let food_a = pos(12, 6);
        let food_b = pos(12, 12);
        let build_world = |zones: Vec<ZoneRuntime>| WorldState {
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
                world_tiles: BTreeMap::from([
                    (
                        food_a,
                        WorldTileRuntime {
                            resources: TileResources {
                                food: 30,
                                herbs: 0,
                                water: 0,
                            },
                            ..tile(food_a.x, food_a.y, 63, None)
                        },
                    ),
                    (
                        food_b,
                        WorldTileRuntime {
                            resources: TileResources {
                                food: 30,
                                herbs: 0,
                                water: 0,
                            },
                            ..tile(food_b.x, food_b.y, 63, None)
                        },
                    ),
                ]),
                zones,
                last_tick: 0,
                test_rng_seed: Some(12_345),
                ..ColonyRuntime::default()
            }],
        };
        let hunt_site = |world: &WorldState| match world.colonies[0].jobs[0].metadata {
            JobMetadata::Hauling {
                site: Some(site), ..
            } => site,
            ref other => panic!("hunt job never got a hauling site: {other:?}"),
        };

        // Baseline: no zones. Record which food tile the hunt targets.
        let mut baseline = build_world(Vec::new());
        let _ = world_tick(&mut baseline, 1_000);
        let chosen = hunt_site(&baseline);
        assert!(
            chosen == food_a || chosen == food_b,
            "baseline hunt should target one of the two food tiles, got {chosen:?}"
        );
        let other = if chosen == food_a { food_b } else { food_a };

        // Paint an avoid zone over the baseline tile; the hunt must be steered to the other.
        let avoid = ZoneRuntime {
            rect: ZoneRect {
                x1: chosen.x,
                y1: chosen.y,
                x2: chosen.x,
                y2: chosen.y,
            },
            kind: ZoneKind::Avoid,
            created_at: 0,
            expires_at: i64::MAX,
            player_id: Some(1),
        };
        let mut steered = build_world(vec![avoid]);
        let _ = world_tick(&mut steered, 1_000);
        assert_eq!(
            hunt_site(&steered),
            other,
            "the avoid zone should have steered the hunt off {chosen:?} to {other:?}"
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

    #[test]
    fn found_colony_is_byte_identical_to_founding_at_the_canonical_anchor() {
        // The single-colony founding path must be a no-op alias for founding at the canonical
        // anchor — colony 0 stays byte-identical while a second village opts into a distinct
        // anchor. Whole-struct `PartialEq` proves every spatial field matches.
        let base = found_colony(4242, "colony-1", 10_000, 99);
        let at = found_colony_at(4242, "colony-1", 10_000, 99, VILLAGE_ANCHOR_TILE);
        assert_eq!(base, at);
        assert_eq!(base.anchor, VILLAGE_ANCHOR_TILE);
    }

    #[test]
    fn select_founding_site_places_a_second_village_far_from_colony_zero_on_grass() {
        // A second village must land at a distinct, valid grass site at least a full
        // separation from colony 0's anchor — never stacked on it.
        for world_seed in [1234u32, 42, 7, 99, 555, 20_240_703] {
            let existing = vec![VILLAGE_ANCHOR_TILE];
            let site = select_founding_site(world_seed, &existing);
            assert!(
                cheb_from_anchor(VILLAGE_ANCHOR_TILE, site) >= MIN_VILLAGE_SEPARATION,
                "seed {world_seed}: site {site:?} too close to colony 0",
            );
            assert_ne!(
                site, VILLAGE_ANCHOR_TILE,
                "seed {world_seed}: site stacked on colony 0"
            );
            // The chosen site (or the guaranteed-separated fallback) is buildable ground.
            let center = shrine_center_tile(site);
            assert!(
                matches!(
                    crate::terrain_gen::tile_biome(world_seed, center.x, center.y),
                    crate::terrain_gen::BiomeRole::Grassland
                        | crate::terrain_gen::BiomeRole::Lowland
                ) || cheb_from_anchor(VILLAGE_ANCHOR_TILE, site) > MIN_VILLAGE_SEPARATION,
                "seed {world_seed}: site {site:?} is neither grass nor the fallback",
            );
        }
    }

    #[test]
    fn select_founding_site_is_deterministic_and_separates_from_every_existing_village() {
        let world_seed = 20_240_703;
        // Twin calls with identical inputs yield an identical site.
        let existing = vec![VILLAGE_ANCHOR_TILE];
        assert_eq!(
            select_founding_site(world_seed, &existing),
            select_founding_site(world_seed, &existing),
        );
        // A third village keeps its distance from *both* prior anchors.
        let second = select_founding_site(world_seed, &existing);
        let existing_two = vec![VILLAGE_ANCHOR_TILE, second];
        let third = select_founding_site(world_seed, &existing_two);
        assert!(cheb_from_anchor(VILLAGE_ANCHOR_TILE, third) >= MIN_VILLAGE_SEPARATION);
        assert!(cheb_from_anchor(second, third) >= MIN_VILLAGE_SEPARATION);
        assert_ne!(third, second);
    }

    #[test]
    fn founded_second_village_stamps_its_blueprint_on_its_own_anchor() {
        // The whole spatial frame of a relocated village rides its own anchor: shrine, claimed
        // square, roads and reveal are all taken relative to the new site, none left at (6, 6).
        let world_seed = 20_240_703;
        let site = select_founding_site(world_seed, &[VILLAGE_ANCHOR_TILE]);
        let colony = found_colony_at(world_seed, "beta", 10_000, 200, site);

        assert_eq!(colony.anchor, site);
        // The shrine footprint's NW corner sits exactly on the new anchor.
        let shrine = colony
            .buildings
            .iter()
            .find(|building| building.building_type == BuildingType::Shrine)
            .expect("founded colony has a shrine");
        assert_eq!(shrine.position, site);
        // Every building and claimed tile is centred on the new anchor, never colony 0's.
        let center = shrine_center_tile(site);
        assert!(colony.claimed_tiles.iter().all(|tile| {
            (tile.x - center.x).abs() <= VILLAGE_START_RADIUS
                && (tile.y - center.y).abs() <= VILLAGE_START_RADIUS
        }));
        assert!(
            colony.buildings.iter().all(|building| {
                cheb_from_anchor(site, building.position) <= VILLAGE_START_RADIUS
            })
        );
        // Its reveal halo is around its own anchor and nowhere near colony 0's anchor.
        assert!(
            colony.revealed_tiles.iter().all(|tile| {
                cheb_from_anchor(VILLAGE_ANCHOR_TILE, *tile) > VILLAGE_START_RADIUS
            })
        );
    }

    #[test]
    fn two_villages_run_independent_ticks_each_relative_to_its_own_anchor() {
        let world_seed = 20_240_703u32;
        let start = 10_000;
        let mut world = new_world(world_seed);
        let site = select_founding_site(world_seed, &[VILLAGE_ANCHOR_TILE]);
        world
            .colonies
            .push(found_colony(world_seed, "colony-1", start, 1234));
        world
            .colonies
            .push(found_colony_at(world_seed, "beta", start, 4321, site));

        for step in 1..=200 {
            let now = start + i64::from(step) * 60_000;
            let reports = world_tick(&mut world, now);
            for report in &reports {
                assert_eq!(
                    report.reset_reason, None,
                    "tick {step} reset {}",
                    report.colony_id
                );
            }
        }

        let base = world.colonies.iter().find(|c| c.id == "colony-1").unwrap();
        let beta = world.colonies.iter().find(|c| c.id == "beta").unwrap();
        assert_ne!(base.anchor, beta.anchor);
        // No cross-mutation: each colony's cats belong to it and its buildings hug its anchor.
        assert!(base.cats.iter().all(|cat| cat.colony_id == "colony-1"));
        assert!(beta.cats.iter().all(|cat| cat.colony_id == "beta"));
        assert!(
            beta.buildings
                .iter()
                .all(|b| cheb_from_anchor(beta.anchor, b.position) <= DEFAULT_MAX_RING + 2),
            "beta's buildings drifted away from its own anchor",
        );
        // The second village actually lives: it kept its founders and its stores did not empty.
        assert!(alive_cats(&beta.cats).count() >= 1);
        assert!(beta.resources.food > 0.0 && beta.resources.water > 0.0);
    }

    #[test]
    fn colony_zero_is_unchanged_by_the_presence_of_a_second_village() {
        // The critical survival guardrail: adding a second village must not perturb colony 0's
        // simulation at all. Run colony 0 alone and again alongside a distant second village on
        // the same world seed/cadence, and its full runtime state must stay byte-identical.
        let world_seed = 1234u32;
        let start = 10_000;

        let mut solo = new_world(world_seed);
        solo.colonies
            .push(found_colony(world_seed, "colony-1", start, 1234));

        let mut shared = new_world(world_seed);
        let site = select_founding_site(world_seed, &[VILLAGE_ANCHOR_TILE]);
        shared
            .colonies
            .push(found_colony(world_seed, "colony-1", start, 1234));
        shared
            .colonies
            .push(found_colony_at(world_seed, "beta", start, 4321, site));

        for step in 1..=120 {
            let now = start + i64::from(step) * 60_000;
            let _ = world_tick(&mut solo, now);
            let _ = world_tick(&mut shared, now);
        }

        let solo_zero = solo.colonies.iter().find(|c| c.id == "colony-1").unwrap();
        let shared_zero = shared.colonies.iter().find(|c| c.id == "colony-1").unwrap();
        assert_eq!(
            solo_zero, shared_zero,
            "colony 0 diverged when a second village existed"
        );
    }

    #[test]
    fn two_village_world_ticks_are_deterministic_across_identical_runs() {
        let world_seed = 20_240_703u32;
        let start = 10_000;
        let build = || {
            let mut world = new_world(world_seed);
            let site = select_founding_site(world_seed, &[VILLAGE_ANCHOR_TILE]);
            world
                .colonies
                .push(found_colony(world_seed, "colony-1", start, 1234));
            world
                .colonies
                .push(found_colony_at(world_seed, "beta", start, 4321, site));
            for step in 1..=150 {
                let _ = world_tick(&mut world, start + i64::from(step) * 60_000);
            }
            world
        };
        assert!(
            build() == build(),
            "two-village world tick is not deterministic"
        );
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
            boosted: false,
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
            fibre: 100.0,
            hide: 100.0,
            cloth: 0.0,
            leather: 0.0,
            ore: 100.0,
            metal: 100.0,
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

    // --- P14.2: soft-obstacle pathfinding (buildings as slow-passable) --------

    #[test]
    fn soft_obstacle_building_tiles_excludes_the_shrine_but_includes_other_buildings() {
        // The shrine stays "fully passable (the hub)" per the P14.2 spec — every
        // haul converges on it, so it must not get the building-footprint cost/
        // speed tier. Every other founding building (dens/workshops) does.
        let colony = found_colony(42, "colony-1", 1_000, 42);
        let shrine = colony
            .buildings
            .iter()
            .find(|building| building.building_type == BuildingType::Shrine)
            .expect("founded colony has a shrine");
        let other = colony
            .buildings
            .iter()
            .find(|building| building.building_type != BuildingType::Shrine)
            .expect("founding blueprint has non-shrine buildings");

        let soft_obstacles = soft_obstacle_building_tiles(&colony);
        for tile in building_footprint_tiles(shrine) {
            assert!(
                !soft_obstacles.contains(&tile),
                "shrine tile {tile:?} must stay off the soft-obstacle set"
            );
        }
        let other_tiles = building_footprint_tiles(other);
        assert!(
            other_tiles.iter().all(|tile| soft_obstacles.contains(tile)),
            "every non-shrine building tile must be a soft obstacle: {other_tiles:?}"
        );
    }

    #[test]
    fn pathfinding_tile_set_converts_world_tick_tile_pos_to_pathfinding_tile_pos() {
        let tiles: HashSet<TilePos> = [TilePos { x: 3, y: -2 }, TilePos { x: 0, y: 9 }]
            .into_iter()
            .collect();
        let converted = pathfinding_tile_set(&tiles);
        assert_eq!(converted.len(), tiles.len());
        for tile in tiles {
            assert!(converted.contains(&PathTilePos {
                x: tile.x,
                y: tile.y
            }));
        }
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
        let seed = 42;
        let at = pos(0, 0);
        // Grass/meadow/marsh (the original P16 heuristic) are always fertile,
        // regardless of whatever the fine per-tile climate biome says here — the
        // two signals are OR'd, so the legacy branch alone is always sufficient.
        assert!(tile_is_farmable(
            seed,
            at,
            Some(&typed_tile(0, 0, TileType::Field))
        ));
        assert!(tile_is_farmable(
            seed,
            at,
            Some(&typed_tile(0, 0, TileType::Meadow))
        ));
        assert!(tile_is_farmable(
            seed,
            at,
            Some(&typed_tile(0, 0, TileType::Swamp))
        ));
        // Rock/sand/tundra/forest/cave legacy types carry no farmability signal of
        // their own (P17 relaxation): they fall through to the fine per-tile P17
        // climate biome at this position, so farmability tracks it exactly.
        let fine_farmable = crate::terrain_gen::tile_climate_biome(seed, at.x, at.y)
            .properties()
            .farmable();
        for tile_type in [
            TileType::Mountains,
            TileType::Desert,
            TileType::Tundra,
            TileType::Forest,
            TileType::CaveEntrance,
        ] {
            assert_eq!(
                tile_is_farmable(seed, at, Some(&typed_tile(0, 0, tile_type))),
                fine_farmable,
                "{tile_type:?} falls through to the fine climate biome"
            );
        }
        // Water always wins over both signals: a river tile is never farmable even
        // though `River`'s fine biome (also barren) would agree either way.
        assert!(!tile_is_farmable(
            seed,
            at,
            Some(&typed_tile(0, 0, TileType::River))
        ));
        // A meadow flooded by a river overlay is not farmable (the synthetic
        // starter-pond case — the fine classifier never sees this override), and an
        // unrevealed (absent) tile is never farmable.
        let flooded = WorldTileRuntime {
            overlay_feature: Some("river".to_owned()),
            ..typed_tile(0, 0, TileType::Meadow)
        };
        assert!(!tile_is_farmable(seed, at, Some(&flooded)));
        assert!(!tile_is_farmable(seed, at, None));
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
                tile_is_farmable(seed, tile, grass.world_tiles.get(&tile)),
                "field footprint {tile:?} is fertile"
            );
        }

        // Rock colony: no fertile ground anywhere — the legacy `Mountains` type
        // carries no farmability signal of its own, and this origin is pinned to a
        // patch whose fine per-tile P17 climate biome is barren across the whole
        // claim too (verified below), so a field is rejected everywhere — but a
        // den (no farmable requirement) still places on the rock.
        let rock_origin = pos(80, 336);
        let mut rock = ColonyRuntime {
            id: "rock".to_owned(),
            ..ColonyRuntime::default()
        };
        typed_block(&mut rock, rock_origin, 6, 7, TileType::Mountains);
        for tile in footprint_tiles(rock_origin, 6, 7) {
            assert!(
                !crate::terrain_gen::tile_climate_biome(seed, tile.x, tile.y)
                    .properties()
                    .farmable(),
                "test fixture assumption: {tile:?} fine biome must be barren"
            );
        }
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

    fn single_field_colony(field_pos: TilePos) -> ColonyRuntime {
        ColonyRuntime {
            id: "fertility".to_owned(),
            resources: Resources::default(),
            buildings: vec![BuildingRuntime {
                id: "field-1".to_owned(),
                building_type: BuildingType::Field,
                level: 1,
                position: field_pos,
                is_complete: true,
                construction_progress: 100,
                production_progress: 0.0,
                assigned_cat: None,
            }],
            ..ColonyRuntime::default()
        }
    }

    #[test]
    fn field_growth_scales_with_the_tile_climate_biomes_fertility() {
        // P17 crop fertility: scan for two farmable spots whose fine per-tile
        // climate biome carries meaningfully different fertility (a deterministic
        // scan, not a hardcoded coordinate — biome placement is seed noise, so
        // this locates real contrasting ground for `seed` rather than assuming
        // it landed somewhere specific).
        let seed = 42;
        let mut low: Option<(TilePos, f64)> = None;
        let mut high: Option<(TilePos, f64)> = None;
        'scan: for y in (0..600).step_by(3) {
            for x in [0, 40, 80, 120] {
                let candidate = pos(x, y);
                let fertility = crate::terrain_gen::tile_climate_biome(seed, x, y)
                    .properties()
                    .fertility;
                if fertility <= 0.0 {
                    continue;
                }
                match low {
                    None => low = Some((candidate, fertility)),
                    Some((_, lo)) if high.is_none() && (fertility - lo).abs() > 0.05 => {
                        high = Some((candidate, fertility));
                    }
                    _ => {}
                }
                if low.is_some() && high.is_some() {
                    break 'scan;
                }
            }
        }
        let (low_pos, low_fertility) = low.expect("a farmable tile exists for seed 42");
        let (high_pos, high_fertility) =
            high.expect("a second, differently-fertile farmable tile exists for seed 42");
        let (low_pos, low_fertility, high_pos, high_fertility) = if low_fertility < high_fertility {
            (low_pos, low_fertility, high_pos, high_fertility)
        } else {
            (high_pos, high_fertility, low_pos, low_fertility)
        };
        assert!(
            high_fertility > low_fertility,
            "test fixture assumption: fertility actually differs ({low_fertility} vs {high_fertility})"
        );

        let mut low_colony = single_field_colony(low_pos);
        let mut high_colony = single_field_colony(high_pos);
        phase_23_production(&mut low_colony, production_gate(3_600, 3_600_000), seed);
        phase_23_production(&mut high_colony, production_gate(3_600, 3_600_000), seed);

        let base = field_yield(3_600.0);
        assert!(
            (low_colony.resources.food - base * low_fertility).abs() < 1e-9,
            "low-fertility field yield should be base * {low_fertility}, got {}",
            low_colony.resources.food
        );
        assert!(
            (high_colony.resources.food - base * high_fertility).abs() < 1e-9,
            "high-fertility field yield should be base * {high_fertility}, got {}",
            high_colony.resources.food
        );
        assert!(
            high_colony.resources.food > low_colony.resources.food,
            "the high-fertility field ({}) must out-grow the low-fertility one ({})",
            high_colony.resources.food,
            low_colony.resources.food
        );
        // Deterministic ratio: re-running produces byte-identical output.
        let mut high_colony_twin = single_field_colony(high_pos);
        phase_23_production(
            &mut high_colony_twin,
            production_gate(3_600, 3_600_000),
            seed,
        );
        assert_eq!(
            high_colony.resources.food.to_bits(),
            high_colony_twin.resources.food.to_bits(),
            "field fertility scaling must be deterministic"
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
        let center = shrine_center_tile(VILLAGE_ANCHOR_TILE);
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
            blocked.extend(founding_road_tiles(VILLAGE_ANCHOR_TILE));
            for pos in &blocked {
                assert!(
                    !tile_has_water(colony.world_tiles.get(pos)),
                    "seed {seed}: water under a building/road at {pos:?}"
                );
            }

            assert!(
                colony.world_tiles.values().any(|t| tile_has_water(Some(t))
                    && cheb_from_anchor(VILLAGE_ANCHOR_TILE, t.pos) <= 6),
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

    #[test]
    fn woodworking_gets_first_idle_worker_once_both_build_reserves_are_funded() {
        let mut colony = found_colony(4242, "colony-1", 10_000, 4242);
        colony.jobs.clear();
        colony.resources.planks = 6.0;
        colony.resources.blocks = 6.0;
        // Leave exactly one work-capable cat so staffing order is observable.
        for cat in colony.cats.iter_mut().skip(1) {
            cat.death_time = Some(10_000);
        }
        for building in &mut colony.buildings {
            building.assigned_cat = None;
        }

        phase_23_production(&mut colony, production_gate(1, 11_000), 4242);

        let staffed = colony
            .buildings
            .iter()
            .find(|building| building.assigned_cat.is_some())
            .expect("one idle cat should staff one raw-material bench");
        assert_eq!(staffed.building_type, BuildingType::Woodworking);
    }

    #[test]
    fn low_water_draws_one_emergency_bucket_without_diverting_labor() {
        let mut colony = found_colony(4242, "colony-1", 10_000, 4242);
        colony.jobs.clear();
        colony.resources.water = 1.0;
        let worker_ids = colony
            .cats
            .iter()
            .take(2)
            .map(|cat| cat.id.clone())
            .collect::<Vec<_>>();
        for (building, cat_id) in colony
            .buildings
            .iter_mut()
            .filter(|building| RAW_MATERIAL_WORKSHOPS.contains(&building.building_type))
            .zip(worker_ids)
        {
            building.assigned_cat = Some(cat_id);
        }

        phase_17b_emergency_water_fetch(&mut colony, production_gate(1, 11_000));

        assert_eq!(colony.resources.water, 9.0);
        assert!(
            colony.jobs.is_empty(),
            "the crisis floor must not steal a food worker"
        );
        assert!(colony.events.iter().any(|event| {
            event.kind == EventKind::Production && event.message.contains("emergency bucket")
        }));
    }

    // ---- P12.6: shrine offerings (carry_offering) ----

    fn ritualist_cat(id: &str) -> Cat {
        Cat {
            specialization: Some(CatSpecialization::Ritualist),
            ..adult_idle_cat(id, "colony-1")
        }
    }

    fn offering_colony(materials: f64) -> ColonyRuntime {
        ColonyRuntime {
            id: "colony-1".to_owned(),
            resources: Resources {
                materials,
                ..Resources::default()
            },
            cats: vec![ritualist_cat("cat-1")],
            test_rng_seed: Some(7),
            ..ColonyRuntime::default()
        }
    }

    fn offering_job(colony: &ColonyRuntime) -> JobRuntime {
        JobRuntime {
            id: "job-offering-1".to_owned(),
            kind: JobKind::CarryOffering,
            status: JobStatus::Active,
            requested_by: JobRequester::Leader,
            assigned_cat: Some(colony.cats[0].id.clone()),
            duration_ms: 2_400_000,
            speed: 1.0,
            yield_amount: 1.0,
            click_count: 0,
            created_at: 0,
            started_at: Some(0),
            ends_at: Some(2_400_000),
            completed_at: None,
            metadata: JobMetadata::None,
        }
    }

    fn reliable_policy() -> TickPolicy {
        TickPolicy {
            config: crate::policy::config_for_tier(crate::types::PolicyTier::Excellent),
        }
    }

    #[test]
    fn complete_offering_converts_surplus_materials_to_blessings_and_logs_an_event() {
        let mut colony = offering_colony(30.0);
        let job = offering_job(&colony);

        complete_offering(&mut colony, &job, production_gate(60, 2_400_000));

        assert_eq!(
            colony.resources.materials, 20.0,
            "the documented OFFERING_MATERIALS_AMOUNT must be the amount consumed"
        );
        let carrying = colony.cats[0]
            .carrying
            .clone()
            .expect("the cat must be carrying the resulting blessings toward the shrine");
        assert_eq!(carrying.kind, CarryingKind::Blessings);
        assert_eq!(
            carrying.amount, 1.0,
            "ritual_mastery 0 -> exactly 1 blessing"
        );
        assert!(
            colony
                .events
                .iter()
                .any(|event| { event.kind == EventKind::Offering }),
            "expected an offering_performed event, got {:?}",
            colony.events
        );

        // The carry/credit step is fully reused, unchanged, from the Ritual path:
        // crediting the resulting carry converts it straight into blessings.
        let before = colony.global_upgrade_points;
        credit_carrying(
            &mut colony,
            &carrying,
            village_anchor_world(VILLAGE_ANCHOR_TILE),
        );
        assert_eq!(colony.global_upgrade_points, before + carrying.amount);
    }

    #[test]
    fn complete_offering_produces_no_blessings_when_the_surplus_evaporated_before_completion() {
        // Materials dropped below OFFERING_MATERIALS_AMOUNT between dispatch and
        // completion (e.g. spent on a build in the meantime). Survival guardrail:
        // no offering, no blessings, no resource loss — the cat just returns.
        let mut colony = offering_colony(10.0);
        let job = offering_job(&colony);

        complete_offering(&mut colony, &job, production_gate(60, 2_400_000));

        assert_eq!(
            colony.resources.materials, 10.0,
            "materials must be untouched"
        );
        assert_eq!(
            colony.cats[0].carrying, None,
            "no carry should be created when there is no genuine surplus to offer"
        );
        assert!(
            !colony
                .events
                .iter()
                .any(|event| { event.kind == EventKind::Offering }),
            "no offering_performed event should fire without a real surplus"
        );
    }

    #[test]
    fn phase_21_dispatches_a_carry_offering_job_for_the_offering_decision() {
        let mut colony = offering_colony(30.0);
        colony.run_started_at = 60_000 - SHRINE_ESTABLISHMENT_MS;
        let plan = DirectorPlan {
            decisions: vec![LeaderDecision::Offering],
            slots: Vec::new(),
        };

        phase_21_leader_capital_decisions_and_tithe(
            &mut colony,
            production_gate(60, 60_000),
            reliable_policy(),
            &plan,
        );

        let job = colony
            .jobs
            .iter()
            .find(|job| job.kind == JobKind::CarryOffering)
            .expect("expected a carry_offering job to be queued");
        assert_eq!(job.assigned_cat.as_deref(), Some("cat-1"));
    }

    #[test]
    fn shrine_faucets_wait_for_the_thirty_hour_establishment_window() {
        let plan = DirectorPlan {
            decisions: vec![
                LeaderDecision::Tithe {
                    food: crate::leader_director::TITHE_FOOD_AMOUNT,
                    refined: 0,
                    blessings: 1,
                },
                LeaderDecision::Offering,
            ],
            slots: Vec::new(),
        };
        let mut colony = offering_colony(30.0);
        colony.resources.food = 100.0;
        colony.run_started_at = 0;

        let gate_at = |processed_through| TickGate {
            elapsed_sec: 60,
            processed_through,
            minute_rolled: true,
            previous_water: 0,
        };
        phase_21_leader_capital_decisions_and_tithe(
            &mut colony,
            gate_at(SHRINE_ESTABLISHMENT_MS - 1),
            reliable_policy(),
            &plan,
        );
        assert_eq!(colony.resources.food, 100.0);
        assert!(colony.jobs.is_empty());
        assert_eq!(colony.global_upgrade_points, 0.0);

        phase_21_leader_capital_decisions_and_tithe(
            &mut colony,
            gate_at(SHRINE_ESTABLISHMENT_MS),
            reliable_policy(),
            &plan,
        );
        assert_eq!(colony.resources.food, 80.0);
        assert!(
            colony
                .jobs
                .iter()
                .any(|job| job.kind == JobKind::CarryOffering)
        );
        assert_eq!(colony.global_upgrade_points, 1.0);

        phase_21_leader_capital_decisions_and_tithe(
            &mut colony,
            gate_at(SHRINE_ESTABLISHMENT_MS + TITHE_COOLDOWN_MS - 1),
            reliable_policy(),
            &plan,
        );
        assert_eq!(colony.resources.food, 80.0, "daily tithe cooldown");
        phase_21_leader_capital_decisions_and_tithe(
            &mut colony,
            gate_at(SHRINE_ESTABLISHMENT_MS + TITHE_COOLDOWN_MS),
            reliable_policy(),
            &plan,
        );
        assert_eq!(colony.resources.food, 60.0, "cooldown is inclusive");
        assert_eq!(colony.global_upgrade_points, 2.0);
    }

    #[test]
    fn offering_dispatch_and_completion_is_deterministic_across_identical_runs() {
        // Two independently-founded worlds on the same seed, forced identically into
        // a materials surplus every tick, must produce byte-identical reports and
        // final colony state through the offering's full dispatch -> travel ->
        // completion -> shrine-credit loop.
        let mut left = new_world(9001);
        left.colonies
            .push(found_colony(left.world_seed, "colony-1", 10_000, 9001));
        let mut right = new_world(9001);
        right
            .colonies
            .push(found_colony(right.world_seed, "colony-1", 10_000, 9001));
        left.colonies[0].run_started_at = 10_000 - SHRINE_ESTABLISHMENT_MS;
        right.colonies[0].run_started_at = 10_000 - SHRINE_ESTABLISHMENT_MS;

        for step in 1..=60 {
            let now = 10_000 + i64::from(step) * 60_000;
            left.colonies[0].resources.materials = 30.0;
            right.colonies[0].resources.materials = 30.0;
            assert_eq!(world_tick(&mut left, now), world_tick(&mut right, now));
        }

        assert_eq!(left.colonies[0].cats, right.colonies[0].cats);
        assert_eq!(
            left.colonies[0].global_upgrade_points,
            right.colonies[0].global_upgrade_points
        );
        assert_eq!(left.colonies[0].jobs, right.colonies[0].jobs);
        assert!(
            left.colonies[0]
                .events
                .iter()
                .any(|event| event.kind == EventKind::Offering),
            "determinism comparison must exercise a completed active offering"
        );
    }

    // --- P14.4: road accessibility (`roads::road_route_to_network` wired into
    // `phase_35b_road_accessibility` + `building_is_road_connected_to_shrine`) ------

    /// A minimal synthetic colony for accessibility tests: a shrine and a den
    /// sitting on a tree/water-free 9x7 meadow claim, three tiles apart with
    /// open ground between them and no road yet. The block's origin (48, 40)
    /// was picked well away from the real village anchor specifically because
    /// it is tree-free for `world_seed`/`seed` 42 across this whole footprint
    /// (verified by direct inspection of `terrain_gen::tile_has_tree`), so the
    /// accessibility router's route is exercised deterministically without
    /// depending on incidental terrain-generator output near the real anchor.
    fn accessibility_test_colony(materials: f64) -> ColonyRuntime {
        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            resources: Resources {
                materials,
                ..Resources::default()
            },
            ..ColonyRuntime::default()
        };
        typed_block(&mut colony, pos(48, 40), 9, 7, TileType::Meadow);
        colony.buildings.push(BuildingRuntime {
            id: "building-shrine".to_owned(),
            building_type: BuildingType::Shrine,
            level: 1,
            position: pos(48, 40), // 3x3 footprint: (48,40)-(50,42)
            is_complete: true,
            construction_progress: 100,
            production_progress: 0.0,
            assigned_cat: None,
        });
        colony.buildings.push(BuildingRuntime {
            id: "building-test-den".to_owned(),
            building_type: BuildingType::Den,
            level: 1,
            position: pos(54, 40), // 2x3 footprint: (54,40)-(55,42); 3 tiles from the shrine
            is_complete: false,
            construction_progress: 0,
            production_progress: 0.0,
            assigned_cat: None,
        });
        colony
    }

    fn minute_gate(processed_through: i64) -> TickGate {
        TickGate {
            elapsed_sec: 60,
            processed_through,
            minute_rolled: true,
            previous_water: 0,
        }
    }

    fn test_den(colony: &ColonyRuntime) -> BuildingRuntime {
        colony
            .buildings
            .iter()
            .find(|building| building.id == "building-test-den")
            .expect("the test den is in the colony")
            .clone()
    }

    #[test]
    fn a_building_placed_away_from_roads_starts_disconnected() {
        let seed = 42;
        let colony = accessibility_test_colony(100.0);
        let den = test_den(&colony);
        assert!(
            !building_is_road_connected_to_shrine(&colony, &den, seed),
            "a den three tiles from the shrine with no road yet is not connected"
        );
    }

    #[test]
    fn accessibility_sweep_paves_a_contiguous_road_path_to_the_shrine() {
        let seed = 42;
        let mut colony = accessibility_test_colony(100.0);
        let den = test_den(&colony);

        let route = building_road_route_to_shrine(&colony, &den, seed)
            .expect("open ground connects the den to the shrine");
        assert!(
            !route.is_empty(),
            "the den starts disconnected, so there is something to pave"
        );

        phase_35b_road_accessibility(&mut colony, minute_gate(60_000), seed);

        // Every tile the router planned to pave is now a built road.
        for tile_pos in &route {
            let tile = colony
                .world_tiles
                .get(tile_pos)
                .unwrap_or_else(|| panic!("{tile_pos:?} was stamped with a road tile"));
            assert_eq!(
                tile.overlay_feature.as_deref(),
                Some("road_built"),
                "{tile_pos:?} is paved"
            );
        }

        // The paved spur actually reaches the network: its last tile borders
        // either the shrine footprint or another road tile.
        let shrine_tiles = shrine_footprint_tiles(&colony);
        let last = *route.last().expect("non-empty route");
        let touches_network = [(0, -1), (1, 0), (0, 1), (-1, 0)].iter().any(|(dx, dy)| {
            let neighbour = pos(last.x + dx, last.y + dy);
            shrine_tiles.contains(&neighbour)
                || colony
                    .world_tiles
                    .get(&neighbour)
                    .is_some_and(|t| t.overlay_feature.as_deref() == Some("road_built"))
        });
        assert!(touches_network, "the paved spur reaches the road network");

        // The helper agrees: the den is connected now.
        let den = test_den(&colony);
        assert!(building_is_road_connected_to_shrine(&colony, &den, seed));
    }

    #[test]
    fn accessibility_sweep_does_not_pave_when_materials_are_at_or_below_the_reserve() {
        let seed = 42;
        let mut colony = accessibility_test_colony(ROAD_MATERIALS_RESERVE);
        let materials_before = colony.resources.materials;

        phase_35b_road_accessibility(&mut colony, minute_gate(60_000), seed);

        assert_eq!(
            colony.resources.materials, materials_before,
            "no materials are spent while at/below the reserve"
        );
        let den = test_den(&colony);
        assert!(
            !building_is_road_connected_to_shrine(&colony, &den, seed),
            "nothing was paved while broke, so the den is still disconnected"
        );
        assert!(
            colony
                .world_tiles
                .values()
                .all(|tile| tile.overlay_feature.is_none()),
            "no tile anywhere was paved"
        );
    }

    #[test]
    fn accessibility_sweep_skips_the_shrine_and_costs_nothing_when_already_connected() {
        let seed = 42;
        // A den anchored directly against the shrine's east edge is already
        // connected (its entrance borders the shrine footprint) — the sweep
        // should recognise that without spending any materials.
        let mut colony = ColonyRuntime {
            id: "colony-1".to_owned(),
            resources: Resources {
                materials: 100.0,
                ..Resources::default()
            },
            ..ColonyRuntime::default()
        };
        typed_block(&mut colony, pos(48, 40), 9, 7, TileType::Meadow);
        colony.buildings.push(BuildingRuntime {
            id: "building-shrine".to_owned(),
            building_type: BuildingType::Shrine,
            level: 1,
            position: pos(48, 40), // 3x3 footprint: (48,40)-(50,42)
            is_complete: true,
            construction_progress: 100,
            production_progress: 0.0,
            assigned_cat: None,
        });
        colony.buildings.push(BuildingRuntime {
            id: "building-adjacent-den".to_owned(),
            building_type: BuildingType::Den,
            level: 1,
            position: pos(51, 40), // 2x3 footprint: (51,40)-(52,42); borders shrine at x=50
            is_complete: false,
            construction_progress: 0,
            production_progress: 0.0,
            assigned_cat: None,
        });

        let den = colony.buildings[1].clone();
        assert!(
            building_is_road_connected_to_shrine(&colony, &den, seed),
            "a den bordering the shrine directly is already connected"
        );

        let materials_before = colony.resources.materials;
        phase_35b_road_accessibility(&mut colony, minute_gate(60_000), seed);
        assert_eq!(
            colony.resources.materials, materials_before,
            "an already-connected building costs nothing to pave"
        );
        assert!(
            colony
                .world_tiles
                .values()
                .all(|tile| tile.overlay_feature.is_none()),
            "nothing needed paving"
        );
    }

    #[test]
    fn accessibility_paving_is_deterministic_for_identical_inputs() {
        let seed = 42;
        let gate = minute_gate(60_000);

        let mut left = accessibility_test_colony(100.0);
        let mut right = accessibility_test_colony(100.0);
        phase_35b_road_accessibility(&mut left, gate, seed);
        phase_35b_road_accessibility(&mut right, gate, seed);

        let road_tiles = |colony: &ColonyRuntime| -> Vec<TilePos> {
            colony
                .world_tiles
                .iter()
                .filter(|(_, tile)| tile.overlay_feature.as_deref() == Some("road_built"))
                .map(|(pos, _)| *pos)
                .collect()
        };
        assert_eq!(road_tiles(&left), road_tiles(&right));
        assert_eq!(left.resources.materials, right.resources.materials);
        assert!(
            !road_tiles(&left).is_empty(),
            "sanity: the sweep actually paved something to compare"
        );
    }

    /// Runs a freshly founded colony-0 unattended for `game_hours` at a 5-game-minute tick
    /// cadence and reports its population trajectory: whether it ever went fully extinct,
    /// plus the minimum and mean alive population sampled every tick after a 30-game-hour
    /// establishment window (long enough for the founding cohort's first kittens to reach
    /// the fertile stage). Shared by the sustainability guardrail and its determinism twin.
    ///
    /// The 5-game-minute step (vs the 1-minute survival proofs) keeps a 100+ game-hour run
    /// tractable in the debug test build, where per-tick cost grows with scout exploration.
    /// It is a deliberately HARSHER proxy than the live 1-second worker: coarser consumption
    /// chunking lets food overshoot below zero more readily, so it stresses the founding
    /// economy harder than reality. Sustaining under it is therefore a conservative
    /// guarantee — finer cadences (including live) are strictly more forgiving.
    fn run_founding_population_trajectory(seed: u32, game_hours: i64) -> (bool, usize, f64) {
        const TICK_MS: i64 = 5 * 60_000; // 5 game-minutes per tick
        const ESTABLISH_MINUTES: i64 = 30 * 60; // start sampling after 30 game-hours

        let mut world = new_world(seed);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 10_000, seed));

        let horizon_ticks = game_hours * 60 / 5; // five game-minutes per tick
        let mut ever_extinct = false;
        let mut min_pop = usize::MAX;
        let mut sum_pop = 0usize;
        let mut samples = 0usize;
        for step in 1..=horizon_ticks {
            let now = 10_000 + step * TICK_MS;
            let _ = world_tick(&mut world, now);
            let pop = alive_cats(&world.colonies[0].cats).count();
            if pop == 0 {
                ever_extinct = true;
            }
            if step * 5 >= ESTABLISH_MINUTES {
                min_pop = min_pop.min(pop);
                sum_pop += pop;
                samples += 1;
            }
        }
        let mean = if samples == 0 {
            0.0
        } else {
            sum_pop as f64 / samples as f64
        };
        (ever_extinct, min_pop, mean)
    }

    /// Long-horizon population-SUSTAINABILITY guardrail (founding boom-bust fix).
    ///
    /// An idle god-sim must self-sustain: a founding colony left running unattended must
    /// not die out. Before the fertile-window fix ([`is_fertile_stage`]), an unattended
    /// founding colony boom-busted — instrumented across these seeds over 150 game-hours,
    /// population peaked ~7–10 then decayed to 1–2 (seed 99 went fully extinct at
    /// game-hour ~121), tripping `UnattendedCollapse`/`AllCatsDead` resets. Every death was
    /// old age; food and water were never the direct cause — the breeding pool (`Adult`
    /// only, a 24h window equal to the 24h birth→adult maturation) periodically emptied to
    /// zero, so births stalled while old-age mortality kept culling.
    ///
    /// With `Young` cats admitted to the fertile pool the generational gap closes and the
    /// population sustains. This guardrail asserts, over 120 game-hours across three seeds
    /// that all boom-busted before the fix (seed 555 went fully extinct; seeds 7 and 2024
    /// decayed to a lone survivor, mean ~1.8/3.3): the colony never goes fully extinct, its
    /// population never falls near extinction (>= 2 after establishment), and its mean
    /// sustained population holds at or above the founding count. It fails before the fix
    /// (seed 555 extinct; seeds 7/2024 hit min 1 and mean well below the founding 5) and
    /// passes after (no extinction; min >= 3; mean 5.8–7.1). Sampling starts after a
    /// 30-game-hour establishment window so the founding cohort's first kittens have reached
    /// fertility.
    #[test]
    fn founding_colony_sustains_its_population_over_a_long_horizon() {
        for seed in [7u32, 555, 2024] {
            let (ever_extinct, min_pop, mean) = run_founding_population_trajectory(seed, 120);
            assert!(
                !ever_extinct,
                "seed {seed}: colony went fully extinct — an unattended god-sim must self-sustain"
            );
            assert!(
                min_pop >= 2,
                "seed {seed}: population fell to {min_pop} after establishment — boom-bust decay \
                 toward extinction"
            );
            assert!(
                mean >= f64::from(STARTER_CAT_COUNT as u32),
                "seed {seed}: mean sustained population {mean:.1} is below the founding \
                 {STARTER_CAT_COUNT} — the colony is shrinking, not self-sustaining"
            );
        }
    }

    #[test]
    #[ignore]
    fn instrument_food_trajectory() {
        for seed in [7u32, 555, 2024, 42, 99] {
            let mut world = new_world(seed);
            world
                .colonies
                .push(found_colony(world.world_seed, "colony-1", 10_000, seed));
            const TICK_MS: i64 = 5 * 60_000;
            let mut resets = 0;
            let mut first_reset_gh: Option<i64> = None;
            let mut comfort_first: Option<i64> = None;
            let mut comfort_ticks = 0;
            let mut research_owned = 0usize;
            let mut max_r = 0.0f64;
            let mut cur_run = 0i64;
            let mut longest_run = 0i64;
            let mut final_pop = 0usize;
            println!("=== seed {seed} ===");
            let horizon = 200 * 60 / 5;
            for step in 1..=horizon {
                let now = 10_000 + step * TICK_MS;
                let reports = world_tick(&mut world, now);
                if reports[0].reset_reason.is_some() {
                    resets += 1;
                    if first_reset_gh.is_none() {
                        first_reset_gh = Some(step * 5 / 60);
                    }
                }
                let c = &world.colonies[0];
                let caps = storage_caps(c);
                let pop = alive_cats(&c.cats).count();
                final_pop = pop;
                let food_r = c.resources.food / caps.food;
                max_r = max_r.max(food_r);
                // Mirror the food half of `is_research_comfortable`'s per-capita bar.
                use crate::leader_director::{
                    RESEARCH_COMFORT_FLOOR, RESEARCH_COMFORT_FOOD_PER_CAT,
                };
                let food_comfortable = c.resources.food
                    >= (pop as f64 * RESEARCH_COMFORT_FOOD_PER_CAT).max(RESEARCH_COMFORT_FLOOR);
                if food_comfortable {
                    cur_run += 5; // game-minutes
                    longest_run = longest_run.max(cur_run);
                } else {
                    cur_run = 0;
                }
                let fields = c
                    .buildings
                    .iter()
                    .filter(|b| b.building_type == BuildingType::Field)
                    .count();
                let dens = c
                    .buildings
                    .iter()
                    .filter(|b| b.building_type == BuildingType::Den && b.is_complete)
                    .count();
                let gh = step * 5 / 60;
                if food_comfortable {
                    comfort_ticks += 1;
                    if comfort_first.is_none() {
                        comfort_first = Some(gh);
                    }
                }
                research_owned = c.upgrade_tree.owned_node_ids.len();
                if step % (12 * 6) == 0 {
                    // every ~6 game-hours
                    println!(
                        "gh{gh:>3} pop{pop:>2} food{:>6.1}/{:>6.0} r{food_r:.2} fields{fields} dens{dens} research_pts{:.1} owned{}",
                        c.resources.food,
                        caps.food,
                        c.upgrade_tree.research_points,
                        c.upgrade_tree.owned_node_ids.len()
                    );
                }
            }
            println!(
                "  -> resets={resets}(first_gh={first_reset_gh:?}) comfort_first_gh={comfort_first:?} comfort_ticks={comfort_ticks}/{horizon} longest_comfort_gh={:.1} max_r={max_r:.2} final_pop={final_pop} research_owned={research_owned}",
                longest_run as f64 / 60.0
            );
        }
    }

    /// Determinism twin for the sustainability guardrail: two colonies founded on the same
    /// seed and ticked over the same long horizon must produce byte-identical population
    /// trajectories (no unseeded RNG entered the fertile-window fix).
    #[test]
    fn founding_population_trajectory_is_deterministic_for_identical_seeds() {
        for seed in [1234u32, 555] {
            let left = run_founding_population_trajectory(seed, 120);
            let right = run_founding_population_trajectory(seed, 120);
            assert_eq!(
                left, right,
                "seed {seed}: identical founding runs diverged — nondeterminism crept in"
            );
        }
    }

    /// Food-comfort fix guardrail (passive field supplement): the leader's population-scaled
    /// clutch of auto-commissioned fields ([`manage_field`], `FIELD_MIN_COUNT`/
    /// `FIELD_CATS_PER_FIELD`) must lift the hunt-only food sawtooth trough above literal
    /// zero for the whole run, and — separately — must never grow into a replacement for
    /// hunting. Both halves matter: a fix that lifts the trough by making food infinite would
    /// "pass" a naive comfort check while breaking the economy it's meant to protect.
    ///
    /// Seed 7 at the harsher 5-game-minute cadence (same harness as
    /// [`run_founding_population_trajectory`]) is a representative hunt-only-starved profile:
    /// instrumentation of the pre-field-supplement behavior showed this exact seed's larder
    /// sawtoothing close to zero every ~8 game-hours between hunts. This guardrail runs it
    /// unattended for 150 game-hours and asserts:
    /// - no tick ever reports an `UnattendedCollapse`/other reset (the trough never dips low
    ///   enough, long enough, to trip the critical-resources reset watchdog);
    /// - the colony never goes fully extinct;
    /// - the food ratio never actually touches literal zero once the 30-game-hour
    ///   establishment window has passed (the trough is lifted, not just "not fatal");
    /// - at the end of the run, the colony's *standing fields'* total passive supply rate
    ///   (summed [`field_yield`] at each field's real per-tile terrain fertility, the exact
    ///   formula `phase_23_production` applies) stays strictly below the population's actual
    ///   consumption rate ([`consumption_for_tick`]) — fields alone could never feed the
    ///   colony, so hunts stay load-bearing and food never goes trivial/infinite.
    #[test]
    fn founding_colony_food_trough_is_lifted_without_going_trivial() {
        const TICK_MS: i64 = 5 * 60_000; // 5 game-minutes per tick, matching the sustain guardrail.
        const ESTABLISH_MINUTES: i64 = 30 * 60; // 30 game-hours, same establishment window.
        const HORIZON_GAME_HOURS: i64 = 150;

        let seed = 7u32;
        let mut world = new_world(seed);
        world
            .colonies
            .push(found_colony(world.world_seed, "colony-1", 10_000, seed));

        let horizon_ticks = HORIZON_GAME_HOURS * 60 / 5;
        let mut min_food_ratio_after_establish = f64::MAX;
        for step in 1..=horizon_ticks {
            let now = 10_000 + step * TICK_MS;
            let reports = world_tick(&mut world, now);
            assert_eq!(
                reports[0].reset_reason,
                None,
                "tick {step} (game-hour {:.1}) tripped a reset ({:?}) — the field supplement \
                 failed to lift the hunt-only food sawtooth trough",
                step as f64 * 5.0 / 60.0,
                reports[0].reset_reason
            );

            let colony = &world.colonies[0];
            assert!(
                alive_cats(&colony.cats).count() > 0,
                "tick {step}: colony went fully extinct"
            );

            if step * 5 >= ESTABLISH_MINUTES {
                let caps = storage_caps(colony);
                let food_ratio = ratio_or_zero(colony.resources.food, caps.food);
                min_food_ratio_after_establish = min_food_ratio_after_establish.min(food_ratio);
            }
        }

        assert!(
            min_food_ratio_after_establish > 0.0,
            "food larder touched literal zero after establishment (min ratio \
             {min_food_ratio_after_establish:.4}) — the sawtooth trough was not lifted"
        );

        // SUPPLEMENT check: standing fields' passive supply must stay strictly below the
        // colony's real consumption rate, so hunting remains necessary and food never
        // becomes trivial/infinite.
        let colony = &world.colonies[0];
        let population = alive_cats(&colony.cats).count() as f64;
        let field_supply_per_hour: f64 = colony
            .buildings
            .iter()
            .filter(|building| {
                building.building_type == BuildingType::Field
                    && building.construction_progress >= 100
            })
            .map(|building| {
                let fertility = crate::terrain_gen::tile_climate_biome(
                    world.world_seed,
                    building.position.x,
                    building.position.y,
                )
                .properties()
                .fertility;
                field_yield(3_600.0) * fertility
            })
            .sum();
        let consumption_per_hour = consumption_for_tick(
            population,
            3_600.0,
            idle_engine_upgrade_levels(&colony.upgrade_levels),
        )
        .food_use;

        assert!(
            field_supply_per_hour < consumption_per_hour,
            "passive field supply ({field_supply_per_hour:.2} food/game-hour) reached or \
             exceeded the colony's actual consumption ({consumption_per_hour:.2} \
             food/game-hour) at population {population} — fields stopped being a supplement \
             and food went trivial"
        );
    }
}
