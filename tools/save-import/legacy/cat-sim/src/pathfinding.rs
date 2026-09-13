//! A* pathfinding ported from `lib/game/pathfinding.ts`.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::movement::{
    DIRT_ROAD_SPEED_MULT, ROAD_BUILT_SPEED_MULT, SOFT_OBSTACLE_SPEED_MULT, SURFACE_FACTOR_GRASSLAND,
};

pub const ROAD_COST: f64 = 0.4;
pub const WORN_PATH_COST: f64 = 0.6;
pub const OPEN_COST: f64 = 1.0;
pub const FOREST_COST: f64 = 4.0;
pub const DENSE_WOODS_COST: f64 = 8.0;
/// Traversal cost of a mountain tile once mining/mountaineering unlocks it. Steep,
/// slow going — dearer than dense woods so cats still skirt peaks when they can.
pub const MOUNTAIN_COST: f64 = 10.0;
/// Traversal cost of a tile covered by a building footprint (P14.2 soft obstacle).
/// Cost tiers mirror the movement-speed model, cost ∝ 1/speed: the spec's
/// "tree+building" tier is ~25% speed, i.e. cost `1.0 / 0.25 = 4.0` — the same
/// numeric value as [`FOREST_COST`] (both realise the same soft-obstacle tier),
/// kept as its own named constant so a future forest-cost tune doesn't silently
/// drag building costs along. A* is free to route through a building when the
/// detour would cost more, but prefers to go around. Never added to
/// `is_blocked` — buildings are always passable, just expensive, so a cat can
/// still reach its own building's work tile.
pub const BUILDING_FOOTPRINT_COST: f64 = 4.0;
pub const MIN_STEP_COST: f64 = ROAD_COST;

pub const DEFAULT_MAX_EXPANSIONS: usize = 6000;
pub const DEFAULT_MARGIN: i32 = 16;
pub const ROAD_WEAR_THRESHOLD: u32 = 70;

const X_FIRST_BIAS: f64 = 1e-6;
const NEIGHBOURS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const TILE_MAP_OFFSET: i64 = 1 << 15;
const TILE_MAP_STRIDE: i64 = 1 << 16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldPos {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
}

pub trait WalkGrid {
    fn is_blocked(&self, x: i32, y: i32) -> bool;
    fn cost(&self, x: i32, y: i32) -> f64;

    fn height_at(&self, _x: i32, _y: i32) -> Option<i32> {
        None
    }

    fn has_stair(&self, _x: i32, _y: i32) -> bool {
        false
    }

    fn fence_blocks_step(&self, _fx: i32, _fy: i32, _tx: i32, _ty: i32) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FindPathOptions {
    pub max_expansions: usize,
    pub margin: i32,
}

impl Default for FindPathOptions {
    fn default() -> Self {
        Self {
            max_expansions: DEFAULT_MAX_EXPANSIONS,
            margin: DEFAULT_MARGIN,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkTileType {
    River,
    Forest,
    DenseWoods,
    /// A mountain-biome peak. Impassable until the colony unlocks mountaineering,
    /// then walkable but slow ([`MOUNTAIN_COST`]).
    Mountain,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkOverlayFeature {
    River,
    RoadBuilt,
    BridgeBuilt,
    GameTrail,
    AncientRoad,
    TradeRoute,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalkTileResources {
    pub water: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkTile {
    pub x: i32,
    pub y: i32,
    pub tile_type: WalkTileType,
    pub overlay_feature: Option<WalkOverlayFeature>,
    pub resources: Option<WalkTileResources>,
    pub path_wear: u32,
}

pub trait TerrainWalkField {
    fn height_at(&self, x: i32, y: i32) -> i32;
    fn has_stair(&self, x: i32, y: i32) -> bool;
}

/// Lazily answers whether a generated multi-tile decoration covers a cell.
/// Kept separate from the explicit obstacle set so callers can cache terrain
/// chunks on demand instead of rebuilding the whole visible wilderness each tick.
pub trait SoftObstacleField {
    fn is_soft_obstacle(&self, x: i32, y: i32) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FenceSide {
    N,
    E,
    S,
    W,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GatePlacement {
    pub x: i32,
    pub y: i32,
    pub side: FenceSide,
}

/// One explicit hard-blocking edge, used while a replacement perimeter is only
/// partially built and cannot yet be represented by the authoritative village area.
/// Endpoints are canonicalized so movement is blocked in both directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FenceEdge {
    pub ax: i32,
    pub ay: i32,
    pub bx: i32,
    pub by: i32,
}

impl FenceEdge {
    #[must_use]
    pub const fn new(ax: i32, ay: i32, bx: i32, by: i32) -> Self {
        if ax < bx || (ax == bx && ay <= by) {
            Self { ax, ay, bx, by }
        } else {
            Self {
                ax: bx,
                ay: by,
                bx: ax,
                by: ay,
            }
        }
    }
}

pub type VillageArea = HashSet<TilePos>;

pub struct ColonyGridParams<'a> {
    pub tiles: &'a [WalkTile],
    pub anchor: TilePos,
    pub ring_radius: i32,
    pub gate: TilePos,
    pub area: Option<&'a VillageArea>,
    pub area_gate: Option<GatePlacement>,
    /// Extra physical wall edges not yet part of `area` (staged expansion).
    pub extra_fence_edges: Option<&'a HashSet<FenceEdge>>,
    pub terrain: Option<&'a dyn TerrainWalkField>,
    /// Whether the colony has unlocked mountaineering/mining. While `false`,
    /// mountain-biome tiles are impassable; once `true` they are walkable but slow.
    pub mountains_unlocked: bool,
    /// Whether the colony has researched Shipping. This is deliberately metadata,
    /// not traversal authority: a study does not put a vessel under an ordinary
    /// walking cat. Water remains blocked until physical vessels and routes exist.
    pub shipping_researched: bool,
    /// Tiles covered by a building footprint (P14.2). Never hard-blocked — A*
    /// costs them at [`BUILDING_FOOTPRINT_COST`] instead, so cats route around a
    /// building when reasonable but can still cross it, and can always reach a
    /// tile inside their own building's footprint (it was never blocked).
    pub soft_obstacles: Option<&'a HashSet<TilePos>>,
    /// Generated decoration footprints (trees/rocks), queried lazily through a
    /// shared per-pass cache. These use the same quarter-speed tier as buildings.
    pub soft_obstacle_field: Option<&'a dyn SoftObstacleField>,
    /// Fine P17 biome movement factors for mapped tiles. When absent, retain
    /// the historical coarse P3 cost table exactly (fixture compatibility).
    /// Runtime movement supplies a precomputed map shared by every A* search in
    /// the tick, so candidate expansion never regenerates terrain chunks.
    pub surface_factors: Option<&'a HashMap<TilePos, f64>>,
}

pub struct ColonyWalkGrid<'a> {
    by_key: HashMap<i64, WalkTile>,
    anchor: TilePos,
    ring_radius: i32,
    gate: TilePos,
    area: Option<&'a VillageArea>,
    area_gate: Option<GatePlacement>,
    extra_fence_edges: Option<&'a HashSet<FenceEdge>>,
    terrain: Option<&'a dyn TerrainWalkField>,
    mountains_unlocked: bool,
    soft_obstacles: Option<&'a HashSet<TilePos>>,
    soft_obstacle_field: Option<&'a dyn SoftObstacleField>,
    surface_factors: Option<&'a HashMap<TilePos, f64>>,
}

#[must_use]
pub fn cliff_blocks_step<G: WalkGrid + ?Sized>(
    _grid: &G,
    _ax: i32,
    _ay: i32,
    _bx: i32,
    _by: i32,
) -> bool {
    false
}

#[must_use]
pub fn build_colony_walk_grid(params: ColonyGridParams<'_>) -> ColonyWalkGrid<'_> {
    // Keep ownership explicit at the construction boundary so tests guard against
    // accidentally restoring magical water walking when transport is implemented.
    let _shipping_researched = params.shipping_researched;
    let mut by_key = HashMap::new();
    for tile in params.tiles {
        by_key.insert(pack_tile_key(tile.x, tile.y), tile.clone());
    }

    ColonyWalkGrid {
        by_key,
        anchor: params.anchor,
        ring_radius: params.ring_radius,
        gate: params.gate,
        area: params.area,
        area_gate: params.area_gate,
        extra_fence_edges: params.extra_fence_edges,
        terrain: params.terrain,
        mountains_unlocked: params.mountains_unlocked,
        soft_obstacles: params.soft_obstacles,
        soft_obstacle_field: params.soft_obstacle_field,
        surface_factors: params.surface_factors,
    }
}

#[must_use]
pub fn find_path<G: WalkGrid + ?Sized>(
    start: WorldPos,
    goal: WorldPos,
    grid: &G,
    options: FindPathOptions,
) -> Option<Vec<WorldPos>> {
    let sx = js_round_to_i32(start.x);
    let sy = js_round_to_i32(start.y);
    let gx = js_round_to_i32(goal.x);
    let gy = js_round_to_i32(goal.y);

    if options.margin < 0 {
        return None;
    }
    if sx == gx && sy == gy {
        return Some(vec![world_pos(sx, sy)]);
    }

    if manhattan(sx, sy, gx, gy) == 1 {
        return Some(vec![world_pos(sx, sy), world_pos(gx, gy)]);
    }

    let margin = i64::from(options.margin);
    let min_x = i64::from(sx.min(gx)) - margin;
    let max_x = i64::from(sx.max(gx)) + margin;
    let min_y = i64::from(sy.min(gy)) - margin;
    let max_y = i64::from(sy.max(gy)) + margin;

    // Store only discovered nodes. The previous dense start-to-goal rectangle was
    // attacker-controlled through distant movement goals and allocated O(distance²)
    // memory before `max_expansions` could stop the search.
    let start_pos = TilePos { x: sx, y: sy };
    let mut nodes = vec![SearchNode {
        pos: start_pos,
        g_score: 0.0,
        came_from: None,
        closed: false,
    }];
    let mut node_keys = HashMap::from([(start_pos, 0_usize)]);
    let mut open = MinHeap::new();
    open.push(0, manhattan_as_f64(sx, sy, gx, gy) * MIN_STEP_COST);

    let mut expansions = 0_usize;

    while !open.is_empty() {
        let current = open.pop();
        let Some(current) = current else {
            break;
        };
        let ck = current.key;
        if nodes[ck].closed {
            continue;
        }
        nodes[ck].closed = true;

        if nodes[ck].pos.x == gx && nodes[ck].pos.y == gy {
            let mut path = Vec::new();
            let mut node = Some(ck);
            while let Some(node_key) = node {
                let state = &nodes[node_key];
                path.push(world_pos(state.pos.x, state.pos.y));
                node = state.came_from;
            }
            path.reverse();
            return Some(path);
        }

        expansions += 1;
        if expansions > options.max_expansions {
            return None;
        }

        let TilePos { x: cx, y: cy } = nodes[ck].pos;

        for (dx, dy) in NEIGHBOURS {
            let Some(nx) = cx.checked_add(dx) else {
                continue;
            };
            let Some(ny) = cy.checked_add(dy) else {
                continue;
            };
            if i64::from(nx) < min_x
                || i64::from(nx) > max_x
                || i64::from(ny) < min_y
                || i64::from(ny) > max_y
            {
                continue;
            }

            let is_goal = nx == gx && ny == gy;
            if !is_goal && grid.is_blocked(nx, ny) {
                continue;
            }
            if cliff_blocks_step(grid, cx, cy, nx, ny) {
                continue;
            }
            if grid.fence_blocks_step(cx, cy, nx, ny) {
                continue;
            }

            let next_pos = TilePos { x: nx, y: ny };
            let nk = if let Some(&key) = node_keys.get(&next_pos) {
                key
            } else {
                let key = nodes.len();
                nodes.push(SearchNode {
                    pos: next_pos,
                    g_score: f64::INFINITY,
                    came_from: None,
                    closed: false,
                });
                node_keys.insert(next_pos, key);
                key
            };
            if nodes[nk].closed {
                continue;
            }

            let premature_y = if dy != 0 && cx != gx {
                X_FIRST_BIAS
            } else {
                0.0
            };
            let tentative = nodes[ck].g_score + grid.cost(nx, ny) + premature_y;
            if tentative < nodes[nk].g_score {
                nodes[nk].g_score = tentative;
                nodes[nk].came_from = Some(ck);
                open.push(
                    nk,
                    tentative + manhattan_as_f64(nx, ny, gx, gy) * MIN_STEP_COST,
                );
            }
        }
    }

    None
}

/// Return whether every goal belongs to the start's reachable component.
///
/// Topology validators often need connectivity truth for many workstations under one
/// projected fence layout. Running A* once per goal repeats the same search; this bounded
/// breadth-first traversal visits each tile at most once and preserves [`find_path`]'s hard
/// obstacle and fence semantics.
#[must_use]
pub fn all_reachable<G: WalkGrid + ?Sized>(
    start: WorldPos,
    goals: &[WorldPos],
    grid: &G,
    options: FindPathOptions,
) -> bool {
    if goals.is_empty() {
        return true;
    }

    let reachable = reachable_component(start, goals, grid, options);
    goals.iter().all(|goal| {
        reachable.contains(&TilePos {
            x: js_round_to_i32(goal.x),
            y: js_round_to_i32(goal.y),
        })
    })
}

/// Build the bounded component needed to answer a batch of reachability queries.
#[must_use]
pub fn reachable_component<G: WalkGrid + ?Sized>(
    start: WorldPos,
    goals: &[WorldPos],
    grid: &G,
    options: FindPathOptions,
) -> HashSet<TilePos> {
    if goals.is_empty() {
        return HashSet::from([TilePos {
            x: js_round_to_i32(start.x),
            y: js_round_to_i32(start.y),
        }]);
    }

    let start = (js_round_to_i32(start.x), js_round_to_i32(start.y));
    let mut remaining = goals
        .iter()
        .map(|goal| (js_round_to_i32(goal.x), js_round_to_i32(goal.y)))
        .collect::<HashSet<_>>();
    remaining.remove(&start);
    if remaining.is_empty() {
        return HashSet::from([TilePos {
            x: start.0,
            y: start.1,
        }]);
    }
    if options.margin < 0 {
        return HashSet::from([TilePos {
            x: start.0,
            y: start.1,
        }]);
    }

    let margin = i64::from(options.margin);
    let min_x = i64::from(
        remaining
            .iter()
            .map(|(x, _)| *x)
            .chain(std::iter::once(start.0))
            .min()
            .expect("the start makes the search bounds non-empty"),
    ) - margin;
    let max_x = i64::from(
        remaining
            .iter()
            .map(|(x, _)| *x)
            .chain(std::iter::once(start.0))
            .max()
            .expect("the start makes the search bounds non-empty"),
    ) + margin;
    let min_y = i64::from(
        remaining
            .iter()
            .map(|(_, y)| *y)
            .chain(std::iter::once(start.1))
            .min()
            .expect("the start makes the search bounds non-empty"),
    ) - margin;
    let max_y = i64::from(
        remaining
            .iter()
            .map(|(_, y)| *y)
            .chain(std::iter::once(start.1))
            .max()
            .expect("the start makes the search bounds non-empty"),
    ) + margin;

    let mut frontier = VecDeque::from([start]);
    let mut visited = HashSet::from([start]);
    let mut expansions = 0_usize;
    while let Some((x, y)) = frontier.pop_front() {
        expansions += 1;
        if expansions > options.max_expansions {
            break;
        }

        for (dx, dy) in NEIGHBOURS {
            let (Some(next_x), Some(next_y)) = (x.checked_add(dx), y.checked_add(dy)) else {
                continue;
            };
            let next = (next_x, next_y);
            if i64::from(next.0) < min_x
                || i64::from(next.0) > max_x
                || i64::from(next.1) < min_y
                || i64::from(next.1) > max_y
            {
                continue;
            }
            let is_goal = remaining.contains(&next);
            if !is_goal && grid.is_blocked(next.0, next.1) {
                continue;
            }
            if cliff_blocks_step(grid, x, y, next.0, next.1)
                || grid.fence_blocks_step(x, y, next.0, next.1)
                || !visited.insert(next)
            {
                continue;
            }
            remaining.remove(&next);
            if remaining.is_empty() {
                frontier.clear();
                break;
            }
            frontier.push_back(next);
        }
    }

    visited.into_iter().map(|(x, y)| TilePos { x, y }).collect()
}

impl WalkGrid for ColonyWalkGrid<'_> {
    fn is_blocked(&self, x: i32, y: i32) -> bool {
        if self.area.is_none() && self.on_legacy_fence(x, y) && !self.is_legacy_gate(x, y) {
            return true;
        }

        self.by_key.get(&pack_tile_key(x, y)).is_some_and(|tile| {
            // A naked walking cat cannot enter water. Shipping research is a design
            // prerequisite only; physical vessels/routes do not exist yet.
            (tile_is_water(tile) && tile.overlay_feature != Some(WalkOverlayFeature::BridgeBuilt))
                || (!self.mountains_unlocked && tile.tile_type == WalkTileType::Mountain)
        })
    }

    fn cost(&self, x: i32, y: i32) -> f64 {
        let tile = self.by_key.get(&pack_tile_key(x, y));
        let base = self.surface_factors.map_or_else(
            || tile_cost(tile),
            |factors| {
                let factor = factors
                    .get(&TilePos { x, y })
                    .copied()
                    .unwrap_or(SURFACE_FACTOR_GRASSLAND);
                fine_tile_cost(tile, factor)
            },
        );
        if self
            .soft_obstacles
            .is_some_and(|tiles| tiles.contains(&TilePos { x, y }))
            || self
                .soft_obstacle_field
                .is_some_and(|field| field.is_soft_obstacle(x, y))
        {
            // Fine runtime costs compose the same ×0.25 speed penalty used by
            // physical movement. Historical callers without fine factors retain
            // the old max-at-4.0 compatibility tier.
            if self.surface_factors.is_some() {
                base / SOFT_OBSTACLE_SPEED_MULT
            } else {
                base.max(BUILDING_FOOTPRINT_COST)
            }
        } else {
            base
        }
    }

    fn height_at(&self, x: i32, y: i32) -> Option<i32> {
        self.terrain.map(|terrain| terrain.height_at(x, y))
    }

    fn has_stair(&self, x: i32, y: i32) -> bool {
        self.terrain.is_some_and(|terrain| terrain.has_stair(x, y))
    }

    fn fence_blocks_step(&self, fx: i32, fy: i32, tx: i32, ty: i32) -> bool {
        if self
            .extra_fence_edges
            .is_some_and(|edges| edges.contains(&FenceEdge::new(fx, fy, tx, ty)))
        {
            return true;
        }
        self.area
            .is_some_and(|area| fence_blocks_move(fx, fy, tx, ty, area, self.area_gate))
    }
}

impl ColonyWalkGrid<'_> {
    fn on_legacy_fence(&self, x: i32, y: i32) -> bool {
        (x - self.anchor.x).abs().max((y - self.anchor.y).abs()) == self.ring_radius
    }

    fn is_legacy_gate(&self, x: i32, y: i32) -> bool {
        x == self.gate.x && y == self.gate.y
    }
}

fn tile_is_water(tile: &WalkTile) -> bool {
    tile.tile_type == WalkTileType::River
        || tile.overlay_feature == Some(WalkOverlayFeature::River)
        || tile.resources.is_some_and(|resources| resources.water > 0)
}

fn tile_cost(tile: Option<&WalkTile>) -> f64 {
    let Some(tile) = tile else {
        return OPEN_COST;
    };

    if matches!(
        tile.overlay_feature,
        Some(WalkOverlayFeature::RoadBuilt | WalkOverlayFeature::BridgeBuilt)
    ) {
        return ROAD_COST;
    }
    // Water is always hard-blocked for walking cats; its nominal cost is never
    // consulted by A*. Mountains remain a research-gated walking surface.
    if tile.tile_type == WalkTileType::Mountain {
        return MOUNTAIN_COST;
    }
    if tile.path_wear >= ROAD_WEAR_THRESHOLD
        || matches!(
            tile.overlay_feature,
            Some(
                WalkOverlayFeature::GameTrail
                    | WalkOverlayFeature::AncientRoad
                    | WalkOverlayFeature::TradeRoute
            )
        )
    {
        return WORN_PATH_COST;
    }
    if tile.tile_type == WalkTileType::DenseWoods {
        return DENSE_WOODS_COST;
    }
    if tile.tile_type == WalkTileType::Forest {
        return FOREST_COST;
    }
    OPEN_COST
}

/// Cost matching the live movement equation: inverse biome speed, with road
/// multipliers composed on top of that surface. Water remains hard-blocked.
fn fine_tile_cost(tile: Option<&WalkTile>, surface_factor: f64) -> f64 {
    if !surface_factor.is_finite() || surface_factor <= 0.0 {
        return tile_cost(tile);
    }
    let road_multiplier = if tile.is_some_and(|tile| {
        matches!(
            tile.overlay_feature,
            Some(WalkOverlayFeature::RoadBuilt | WalkOverlayFeature::BridgeBuilt)
        )
    }) {
        ROAD_BUILT_SPEED_MULT
    } else if tile.is_some_and(|tile| {
        tile.path_wear >= ROAD_WEAR_THRESHOLD
            || matches!(
                tile.overlay_feature,
                Some(
                    WalkOverlayFeature::GameTrail
                        | WalkOverlayFeature::AncientRoad
                        | WalkOverlayFeature::TradeRoute
                )
            )
    }) {
        DIRT_ROAD_SPEED_MULT
    } else {
        1.0
    };
    1.0 / (surface_factor * road_multiplier)
}

fn fence_blocks_move(
    fx: i32,
    fy: i32,
    tx: i32,
    ty: i32,
    area: &VillageArea,
    gate: Option<GatePlacement>,
) -> bool {
    let Some(edge) = fence_edge_between(fx, fy, tx, ty, area) else {
        return false;
    };
    !gate.is_some_and(|gate| gate == edge)
}

fn fence_edge_between(
    fx: i32,
    fy: i32,
    tx: i32,
    ty: i32,
    area: &VillageArea,
) -> Option<GatePlacement> {
    let dx = tx - fx;
    let dy = ty - fy;
    if dx.abs() + dy.abs() != 1 {
        return None;
    }

    let from_in = area.contains(&TilePos { x: fx, y: fy });
    let to_in = area.contains(&TilePos { x: tx, y: ty });
    if from_in == to_in {
        return None;
    }

    let (inside_x, inside_y) = if from_in { (fx, fy) } else { (tx, ty) };
    let side = if dx == 1 {
        if from_in { FenceSide::E } else { FenceSide::W }
    } else if dx == -1 {
        if from_in { FenceSide::W } else { FenceSide::E }
    } else if dy == 1 {
        if from_in { FenceSide::S } else { FenceSide::N }
    } else if from_in {
        FenceSide::N
    } else {
        FenceSide::S
    };

    Some(GatePlacement {
        x: inside_x,
        y: inside_y,
        side,
    })
}

fn pack_tile_key(x: i32, y: i32) -> i64 {
    (i64::from(x) + TILE_MAP_OFFSET) * TILE_MAP_STRIDE + (i64::from(y) + TILE_MAP_OFFSET)
}

fn manhattan(ax: i32, ay: i32, bx: i32, by: i32) -> u64 {
    u64::from(ax.abs_diff(bx)) + u64::from(ay.abs_diff(by))
}

fn manhattan_as_f64(ax: i32, ay: i32, bx: i32, by: i32) -> f64 {
    manhattan(ax, ay, bx, by) as f64
}

fn js_round_to_i32(value: f64) -> i32 {
    (value + 0.5).floor() as i32
}

fn world_pos(x: i32, y: i32) -> WorldPos {
    WorldPos {
        x: f64::from(x),
        y: f64::from(y),
    }
}

#[derive(Debug, Clone, Copy)]
struct SearchNode {
    pos: TilePos,
    g_score: f64,
    came_from: Option<usize>,
    closed: bool,
}

#[derive(Debug, Clone, Copy)]
struct HeapItem {
    key: usize,
    f: f64,
    seq: usize,
}

#[derive(Debug, Default)]
struct MinHeap {
    items: Vec<HeapItem>,
    counter: usize,
}

impl MinHeap {
    fn new() -> Self {
        Self::default()
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn push(&mut self, key: usize, f: f64) {
        self.items.push(HeapItem {
            key,
            f,
            seq: self.counter,
        });
        self.counter += 1;

        let mut i = self.items.len() - 1;
        while i > 0 {
            let parent = (i - 1) >> 1;
            if !Self::before(self.items[i], self.items[parent]) {
                break;
            }
            self.items.swap(parent, i);
            i = parent;
        }
    }

    fn pop(&mut self) -> Option<HeapItem> {
        let top = self.items.first().copied()?;
        let last = self
            .items
            .pop()
            .expect("heap contains top, so pop returns an item");

        if !self.items.is_empty() {
            self.items[0] = last;
            let mut i = 0;
            loop {
                let left = 2 * i + 1;
                let right = 2 * i + 2;
                let mut smallest = i;

                if left < self.items.len() && Self::before(self.items[left], self.items[smallest]) {
                    smallest = left;
                }
                if right < self.items.len() && Self::before(self.items[right], self.items[smallest])
                {
                    smallest = right;
                }
                if smallest == i {
                    break;
                }
                self.items.swap(smallest, i);
                i = smallest;
            }
        }

        Some(top)
    }

    fn before(a: HeapItem, b: HeapItem) -> bool {
        a.f < b.f || (a.f == b.f && a.seq < b.seq)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use serde::Deserialize;

    use super::{
        BUILDING_FOOTPRINT_COST, ColonyGridParams, ColonyWalkGrid, DEFAULT_MARGIN,
        DEFAULT_MAX_EXPANSIONS, DENSE_WOODS_COST, FOREST_COST, FenceSide, FindPathOptions,
        GatePlacement, MIN_STEP_COST, MOUNTAIN_COST, OPEN_COST, ROAD_COST, ROAD_WEAR_THRESHOLD,
        SURFACE_FACTOR_GRASSLAND, SoftObstacleField, TilePos, VillageArea, WORN_PATH_COST,
        WalkGrid, WalkOverlayFeature, WalkTile, WalkTileResources, WalkTileType, WorldPos,
        all_reachable, build_colony_walk_grid, cliff_blocks_step, find_path, tile_cost,
    };

    #[derive(Debug, Deserialize)]
    struct Fixture {
        source: String,
        constants: ConstantFixture,
        #[serde(rename = "pathCases")]
        path_cases: Vec<PathCase>,
        #[serde(rename = "colonyChecks")]
        colony_checks: ColonyChecks,
        #[serde(rename = "colonyPathCases")]
        colony_path_cases: Vec<ColonyPathCase>,
    }

    #[derive(Debug, Deserialize)]
    struct ConstantFixture {
        #[serde(rename = "ROAD_COST")]
        road_cost: f64,
        #[serde(rename = "WORN_PATH_COST")]
        worn_path_cost: f64,
        #[serde(rename = "OPEN_COST")]
        open_cost: f64,
        #[serde(rename = "FOREST_COST")]
        forest_cost: f64,
        #[serde(rename = "DENSE_WOODS_COST")]
        dense_woods_cost: f64,
        #[serde(rename = "MIN_STEP_COST")]
        min_step_cost: f64,
        #[serde(rename = "ROAD_WEAR_THRESHOLD")]
        road_wear_threshold: u32,
        #[serde(rename = "DEFAULT_MAX_EXPANSIONS")]
        default_max_expansions: usize,
        #[serde(rename = "DEFAULT_MARGIN")]
        default_margin: i32,
    }

    #[derive(Debug, Deserialize)]
    struct PathCase {
        name: String,
        grid: GridSpec,
        start: PosFixture,
        goal: PosFixture,
        options: Option<OptionFixture>,
        expected: Option<Vec<String>>,
    }

    #[derive(Debug, Deserialize)]
    struct GridSpec {
        kind: String,
        #[serde(default)]
        blocked: Vec<String>,
        #[serde(default)]
        roads: Vec<String>,
        #[serde(default)]
        costs: HashMap<String, f64>,
        mode: Option<String>,
        #[serde(rename = "rngSeed")]
        rng_seed: Option<u32>,
    }

    #[derive(Debug, Clone, Copy, Deserialize)]
    struct PosFixture {
        x: f64,
        y: f64,
    }

    #[derive(Debug, Clone, Copy, Deserialize)]
    struct OptionFixture {
        #[serde(rename = "maxExpansions")]
        max_expansions: Option<usize>,
        margin: Option<i32>,
    }

    #[derive(Debug, Deserialize)]
    struct ColonyChecks {
        anchor: String,
        #[serde(rename = "ringRadius")]
        ring_radius: i32,
        gate: String,
        tiles: Vec<TileFixture>,
        blocked: HashMap<String, bool>,
        costs: HashMap<String, f64>,
    }

    #[derive(Debug, Deserialize)]
    struct TileFixture {
        x: i32,
        y: i32,
        #[serde(rename = "type")]
        tile_type: Option<String>,
        #[serde(rename = "overlayFeature")]
        overlay_feature: Option<String>,
        resources: Option<ResourceFixture>,
        #[serde(rename = "pathWear")]
        path_wear: u32,
    }

    #[derive(Debug, Deserialize)]
    struct ResourceFixture {
        water: u32,
    }

    #[derive(Debug, Deserialize)]
    struct ColonyPathCase {
        name: String,
        kind: String,
        area: Option<Vec<String>>,
        #[serde(rename = "areaGate")]
        area_gate: Option<GateFixture>,
        start: PosFixture,
        goal: PosFixture,
        options: Option<OptionFixture>,
        expected: Option<Vec<String>>,
    }

    #[derive(Debug, Clone, Copy, Deserialize)]
    struct GateFixture {
        x: i32,
        y: i32,
        side: FenceSideFixture,
    }

    #[derive(Debug, Clone, Copy, Deserialize)]
    enum FenceSideFixture {
        N,
        E,
        S,
        W,
    }

    struct TestGrid {
        blocked: HashSet<String>,
        costs: HashMap<String, f64>,
        roads: HashSet<String>,
    }

    impl WalkGrid for TestGrid {
        fn is_blocked(&self, x: i32, y: i32) -> bool {
            self.blocked.contains(&coord_key(x, y))
        }

        fn cost(&self, x: i32, y: i32) -> f64 {
            let key = coord_key(x, y);
            self.costs
                .get(&key)
                .copied()
                .unwrap_or(if self.roads.contains(&key) {
                    ROAD_COST
                } else {
                    OPEN_COST
                })
        }
    }

    struct HeightGrid;

    impl WalkGrid for HeightGrid {
        fn is_blocked(&self, _x: i32, _y: i32) -> bool {
            false
        }

        fn cost(&self, _x: i32, _y: i32) -> f64 {
            OPEN_COST
        }

        fn height_at(&self, x: i32, _y: i32) -> Option<i32> {
            Some(if x >= 3 { 2 } else { 0 })
        }
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../docs/migration/fixtures/p3/pathfinding.json"
        ))
        .expect("pathfinding fixture parses")
    }

    #[test]
    fn fixture_is_generated_from_pathfinding_ts() {
        let fixture = fixture();
        assert_eq!(fixture.source, "lib/game/pathfinding.ts");
        assert_eq!(fixture.path_cases.len(), 21);
        assert_eq!(fixture.colony_path_cases.len(), 3);
    }

    #[test]
    fn multi_goal_reachability_checks_the_shared_component_once() {
        let grid = TestGrid {
            blocked: HashSet::new(),
            costs: HashMap::new(),
            roads: HashSet::new(),
        };
        assert!(all_reachable(
            WorldPos { x: 0.0, y: 0.0 },
            &[
                WorldPos { x: 4.0, y: 0.0 },
                WorldPos { x: -2.0, y: 3.0 },
                WorldPos { x: 0.0, y: 0.0 },
            ],
            &grid,
            FindPathOptions::default(),
        ));
    }

    #[test]
    fn multi_goal_reachability_rejects_one_isolated_destination() {
        let grid = TestGrid {
            blocked: ["2,0", "3,1", "3,-1", "4,0"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            costs: HashMap::new(),
            roads: HashSet::new(),
        };
        assert!(!all_reachable(
            WorldPos { x: 0.0, y: 0.0 },
            &[WorldPos { x: 1.0, y: 0.0 }, WorldPos { x: 3.0, y: 0.0 },],
            &grid,
            FindPathOptions::default(),
        ));
    }

    #[test]
    fn multi_goal_reachability_is_independent_of_surface_costs() {
        let blocked = ["2,0", "2,1"]
            .into_iter()
            .map(str::to_owned)
            .collect::<HashSet<_>>();
        let plain = TestGrid {
            blocked: blocked.clone(),
            costs: HashMap::new(),
            roads: HashSet::new(),
        };
        let expensive = TestGrid {
            blocked,
            costs: [
                ("1,0".to_owned(), 10_000.0),
                ("1,1".to_owned(), MIN_STEP_COST),
                ("2,-1".to_owned(), f64::INFINITY),
            ]
            .into_iter()
            .collect(),
            roads: HashSet::new(),
        };
        let goals = [WorldPos { x: 3.0, y: 0.0 }, WorldPos { x: -2.0, y: 1.0 }];
        let options = FindPathOptions::default();

        assert_eq!(
            all_reachable(WorldPos { x: 0.0, y: 0.0 }, &goals, &plain, options),
            all_reachable(WorldPos { x: 0.0, y: 0.0 }, &goals, &expensive, options),
        );
    }

    #[test]
    fn constants_match_ts_fixture() {
        let constants = fixture().constants;
        assert_eq!(constants.road_cost, ROAD_COST);
        assert_eq!(constants.worn_path_cost, WORN_PATH_COST);
        assert_eq!(constants.open_cost, OPEN_COST);
        assert_eq!(constants.forest_cost, FOREST_COST);
        assert_eq!(constants.dense_woods_cost, DENSE_WOODS_COST);
        assert_eq!(constants.min_step_cost, MIN_STEP_COST);
        assert_eq!(constants.road_wear_threshold, ROAD_WEAR_THRESHOLD);
        assert_eq!(constants.default_max_expansions, DEFAULT_MAX_EXPANSIONS);
        assert_eq!(constants.default_margin, DEFAULT_MARGIN);
    }

    #[test]
    fn find_path_routes_match_ts_fixture_byte_for_byte() {
        for case in fixture().path_cases {
            let grid = test_grid(&case.grid);
            let path = find_path(
                case.start.into_world_pos(),
                case.goal.into_world_pos(),
                &grid,
                options(case.options),
            );
            assert_eq!(path_keys(path.as_ref()), case.expected, "{}", case.name);
            if let Some(path) = path {
                assert_contiguous(&path, &case.name);
                assert_eq!(
                    path.first().copied(),
                    case.expected_start(),
                    "{}",
                    case.name
                );
                assert_eq!(path.last().copied(), case.expected_goal(), "{}", case.name);
            }
        }
    }

    #[test]
    fn repeated_runs_are_deterministic() {
        for case in fixture().path_cases {
            let grid = test_grid(&case.grid);
            let a = find_path(
                case.start.into_world_pos(),
                case.goal.into_world_pos(),
                &grid,
                options(case.options),
            );
            let b = find_path(
                case.start.into_world_pos(),
                case.goal.into_world_pos(),
                &grid,
                options(case.options),
            );
            assert_eq!(
                path_keys(a.as_ref()),
                path_keys(b.as_ref()),
                "{}",
                case.name
            );
        }
    }

    #[test]
    fn distant_and_extreme_goals_respect_the_expansion_budget_without_dense_allocation() {
        let grid = TestGrid {
            blocked: HashSet::new(),
            costs: HashMap::new(),
            roads: HashSet::new(),
        };
        let options = FindPathOptions {
            max_expansions: 8,
            margin: DEFAULT_MARGIN,
        };

        assert_eq!(
            find_path(
                WorldPos { x: 0.0, y: 0.0 },
                WorldPos {
                    x: 1_000_000_000.0,
                    y: 1_000_000_000.0,
                },
                &grid,
                options,
            ),
            None
        );
        assert_eq!(
            find_path(
                WorldPos {
                    x: f64::from(i32::MIN),
                    y: f64::from(i32::MIN),
                },
                WorldPos {
                    x: f64::from(i32::MAX),
                    y: f64::from(i32::MAX),
                },
                &grid,
                options,
            ),
            None
        );
    }

    #[test]
    fn invalid_negative_margin_fails_without_bounds_arithmetic() {
        let grid = TestGrid {
            blocked: HashSet::new(),
            costs: HashMap::new(),
            roads: HashSet::new(),
        };
        assert_eq!(
            find_path(
                WorldPos { x: 0.0, y: 0.0 },
                WorldPos { x: 2.0, y: 0.0 },
                &grid,
                FindPathOptions {
                    max_expansions: 8,
                    margin: -1,
                },
            ),
            None
        );
    }

    /// Build a grid with a mountain tile at (3,0) and a water tile at (5,0), then
    /// hand it to `check`. Anchor + ring are placed far away so the legacy fence
    /// never triggers on these coords. `tiles` is kept alive for the closure.
    fn with_mountain_and_water_grid(
        mountains_unlocked: bool,
        shipping_researched: bool,
        check: impl FnOnce(&ColonyWalkGrid),
    ) {
        let tiles = vec![
            WalkTile {
                x: 3,
                y: 0,
                tile_type: WalkTileType::Mountain,
                overlay_feature: None,
                resources: None,
                path_wear: 0,
            },
            WalkTile {
                x: 5,
                y: 0,
                tile_type: WalkTileType::River,
                overlay_feature: Some(WalkOverlayFeature::River),
                resources: Some(WalkTileResources { water: 1 }),
                path_wear: 0,
            },
        ];
        let grid = build_colony_walk_grid(ColonyGridParams {
            tiles: &tiles,
            anchor: TilePos { x: 0, y: 0 },
            ring_radius: 10_000,
            gate: TilePos { x: 0, y: 1 },
            area: None,
            area_gate: None,
            extra_fence_edges: None,
            terrain: None,
            mountains_unlocked,
            shipping_researched,
            soft_obstacles: None,
            soft_obstacle_field: None,
            surface_factors: None,
        });
        check(&grid);
    }

    #[test]
    fn mountain_tiles_are_blocked_until_mountaineering_is_unlocked() {
        with_mountain_and_water_grid(false, false, |grid| {
            assert!(
                grid.is_blocked(3, 0),
                "mountain blocked without the upgrade"
            );
        });
        with_mountain_and_water_grid(true, false, |grid| {
            assert!(
                !grid.is_blocked(3, 0),
                "mountain passable once mountaineering is unlocked"
            );
            // Passable but slow — dearer than open ground.
            assert_eq!(grid.cost(3, 0), MOUNTAIN_COST);
            assert!(grid.cost(3, 0) > OPEN_COST);
        });
    }

    #[test]
    fn water_tiles_are_always_blocked_without_shipping_regardless_of_mountaineering() {
        // Critical guardrail (P17): a colony without the `shipping` upgrade must see
        // water blocked exactly as before, whether or not it separately owns
        // mountaineering — the two gates are independent.
        with_mountain_and_water_grid(false, false, |grid| assert!(grid.is_blocked(5, 0)));
        with_mountain_and_water_grid(true, false, |grid| {
            assert!(
                grid.is_blocked(5, 0),
                "water must stay blocked without shipping even once mountains are open"
            );
        });
    }

    #[test]
    fn shipping_ownership_alone_never_makes_water_walkable() {
        with_mountain_and_water_grid(false, true, |grid| {
            assert!(
                grid.is_blocked(5, 0),
                "research ownership does not give a naked cat a vessel or route"
            );
        });
        with_mountain_and_water_grid(true, true, |grid| assert!(grid.is_blocked(5, 0)));
    }

    #[test]
    fn completed_bridge_makes_its_water_tile_walkable_at_road_cost() {
        let tiles = vec![WalkTile {
            x: 5,
            y: 0,
            tile_type: WalkTileType::River,
            overlay_feature: Some(WalkOverlayFeature::BridgeBuilt),
            resources: Some(WalkTileResources { water: 1 }),
            path_wear: 100,
        }];
        let grid = build_colony_walk_grid(ColonyGridParams {
            tiles: &tiles,
            anchor: TilePos { x: 0, y: 0 },
            ring_radius: 10_000,
            gate: TilePos { x: 0, y: 1 },
            area: None,
            area_gate: None,
            extra_fence_edges: None,
            terrain: None,
            mountains_unlocked: false,
            shipping_researched: false,
            soft_obstacles: None,
            soft_obstacle_field: None,
            surface_factors: None,
        });

        assert!(!grid.is_blocked(5, 0));
        assert_eq!(grid.cost(5, 0), ROAD_COST);
    }

    #[test]
    fn a_new_bridge_replans_an_unreachable_route_through_the_crossing() {
        let barrier = |completed: bool| {
            (-40..=40)
                .map(|y| WalkTile {
                    x: 1,
                    y,
                    tile_type: WalkTileType::River,
                    overlay_feature: if completed && y == 0 {
                        Some(WalkOverlayFeature::BridgeBuilt)
                    } else {
                        Some(WalkOverlayFeature::River)
                    },
                    resources: Some(WalkTileResources { water: 1 }),
                    path_wear: u32::from(completed && y == 0) * 100,
                })
                .collect::<Vec<_>>()
        };
        let route = |tiles: &[WalkTile]| {
            let grid = build_colony_walk_grid(ColonyGridParams {
                tiles,
                anchor: TilePos { x: 0, y: 0 },
                ring_radius: 10_000,
                gate: TilePos { x: 0, y: 1 },
                area: None,
                area_gate: None,
                extra_fence_edges: None,
                terrain: None,
                mountains_unlocked: false,
                shipping_researched: false,
                soft_obstacles: None,
                soft_obstacle_field: None,
                surface_factors: None,
            });
            find_path(
                WorldPos { x: 0.0, y: 0.0 },
                WorldPos { x: 2.0, y: 0.0 },
                &grid,
                FindPathOptions::default(),
            )
        };

        assert!(route(&barrier(false)).is_none());
        assert_eq!(
            route(&barrier(true)).expect("completed bridge opens and replans the route"),
            vec![
                WorldPos { x: 0.0, y: 0.0 },
                WorldPos { x: 1.0, y: 0.0 },
                WorldPos { x: 2.0, y: 0.0 },
            ]
        );
    }

    /// A full-width water barrier (a lake/strait) at `y == 5`, spanning the entire
    /// bounded search window so there is no detour around it — the only way from
    /// one bank to the other is straight through the water. Anchor + ring are
    /// placed far away so the legacy fence never triggers on these coords.
    fn water_barrier_tiles(min_x: i32, max_x: i32) -> Vec<WalkTile> {
        (min_x..=max_x)
            .map(|x| WalkTile {
                x,
                y: 5,
                tile_type: WalkTileType::River,
                overlay_feature: Some(WalkOverlayFeature::River),
                resources: Some(WalkTileResources { water: 1 }),
                path_wear: 0,
            })
            .collect()
    }

    #[test]
    fn shipping_owned_and_unowned_have_identical_astar_reachability_without_a_vessel_route() {
        // Tight margin so the barrier only needs to span a handful of columns to
        // rule out any detour, and start/goal sit one tile from either bank.
        let options = FindPathOptions {
            max_expansions: DEFAULT_MAX_EXPANSIONS,
            margin: 3,
        };
        let tiles = water_barrier_tiles(-3, 3);
        let start = WorldPos { x: 0.0, y: 4.0 };
        let goal = WorldPos { x: 0.0, y: 6.0 };

        let without_shipping = build_colony_walk_grid(ColonyGridParams {
            tiles: &tiles,
            anchor: TilePos { x: 0, y: 0 },
            ring_radius: 10_000,
            gate: TilePos { x: 0, y: 1 },
            area: None,
            area_gate: None,
            extra_fence_edges: None,
            terrain: None,
            mountains_unlocked: false,
            shipping_researched: false,
            soft_obstacles: None,
            soft_obstacle_field: None,
            surface_factors: None,
        });
        assert_eq!(
            find_path(start, goal, &without_shipping, options),
            None,
            "a colony without shipping has no route across the strait"
        );

        let with_shipping = build_colony_walk_grid(ColonyGridParams {
            tiles: &tiles,
            anchor: TilePos { x: 0, y: 0 },
            ring_radius: 10_000,
            gate: TilePos { x: 0, y: 1 },
            area: None,
            area_gate: None,
            extra_fence_edges: None,
            terrain: None,
            mountains_unlocked: false,
            shipping_researched: true,
            soft_obstacles: None,
            soft_obstacle_field: None,
            surface_factors: None,
        });
        assert_eq!(
            find_path(start, goal, &with_shipping, options),
            find_path(start, goal, &without_shipping, options),
            "owning a study cannot conjure physical water transport"
        );
    }

    // ---- P14.2: soft-obstacle pathfinding (buildings) ----------------------

    /// A minimal `WalkGrid` for exercising the soft-obstacle *concept* directly
    /// (unlike `ColonyWalkGrid`, which is exercised separately below): a
    /// rectangular `footprint` region costs [`BUILDING_FOOTPRINT_COST`], an
    /// optional set of `walls` hard-blocks (so a test can force the only route
    /// straight across the footprint), and everything else is open ground.
    struct SoftObstacleGrid {
        footprint: HashSet<(i32, i32)>,
        walls: HashSet<(i32, i32)>,
    }

    struct GeneratedObstacleField {
        footprint: HashSet<TilePos>,
    }

    impl SoftObstacleField for GeneratedObstacleField {
        fn is_soft_obstacle(&self, x: i32, y: i32) -> bool {
            self.footprint.contains(&TilePos { x, y })
        }
    }

    impl WalkGrid for SoftObstacleGrid {
        fn is_blocked(&self, x: i32, y: i32) -> bool {
            self.walls.contains(&(x, y))
        }

        fn cost(&self, x: i32, y: i32) -> f64 {
            if self.footprint.contains(&(x, y)) {
                BUILDING_FOOTPRINT_COST
            } else {
                OPEN_COST
            }
        }
    }

    fn square_footprint(min: i32, max: i32) -> HashSet<(i32, i32)> {
        (min..=max)
            .flat_map(|x| (min..=max).map(move |y| (x, y)))
            .collect()
    }

    #[test]
    fn soft_obstacle_routes_around_when_a_reasonable_detour_exists() {
        // A 3x3 building footprint sits squarely on the straight line from
        // (6,0) to (6,10). Crossing it costs 3 footprint tiles at
        // BUILDING_FOOTPRINT_COST (4.0) each = 12, on top of the 7 open steps
        // (7.0), for 19.0 total. Going around costs 4 extra open steps (one
        // tile wider than the footprint) at OPEN_COST — 14.0 total — so A*
        // must prefer the detour.
        let grid = SoftObstacleGrid {
            footprint: square_footprint(5, 7),
            walls: HashSet::new(),
        };
        let path = find_path(
            WorldPos { x: 6.0, y: 0.0 },
            WorldPos { x: 6.0, y: 10.0 },
            &grid,
            FindPathOptions::default(),
        )
        .expect("a route exists — the building never hard-blocks");

        let crosses_footprint = path
            .iter()
            .any(|pos| grid.footprint.contains(&(pos.x as i32, pos.y as i32)));
        assert!(
            !crosses_footprint,
            "A* should detour around the building when it's cheaper: {path:?}"
        );
    }

    #[test]
    fn soft_obstacle_is_passable_not_blocking_when_the_only_route_crosses_it() {
        // Wall off every route except a one-tile-wide corridor straight through
        // the footprint — the cat MUST cross the building, and can, because a
        // soft obstacle never hard-blocks.
        let footprint: HashSet<(i32, i32)> = (3..=5).map(|y| (6, y)).collect();
        let mut walls = HashSet::new();
        for y in 0..=8 {
            for x in [4, 5, 7, 8] {
                walls.insert((x, y));
            }
        }
        let grid = SoftObstacleGrid { footprint, walls };

        let path = find_path(
            WorldPos { x: 6.0, y: 0.0 },
            WorldPos { x: 6.0, y: 8.0 },
            &grid,
            FindPathOptions::default(),
        );
        assert!(
            path.is_some(),
            "a soft obstacle must never make the goal unreachable"
        );
        let path = path.expect("checked above");
        assert!(
            path.iter()
                .any(|pos| grid.footprint.contains(&(pos.x as i32, pos.y as i32))),
            "the only route runs through the footprint, so the path must cross it: {path:?}"
        );
    }

    #[test]
    fn a_buildings_own_work_tile_stays_reachable_even_though_its_footprint_is_costed() {
        // Destination/work-tile exemption (mirrors the existing mountain-goal
        // exemption): nothing about a soft obstacle is ever added to
        // `is_blocked`, so a cat can always path onto a tile inside its own
        // building's footprint — even the footprint's interior, not just the
        // near edge.
        let grid = SoftObstacleGrid {
            footprint: square_footprint(5, 7),
            walls: HashSet::new(),
        };
        let path = find_path(
            WorldPos { x: 0.0, y: 0.0 },
            WorldPos { x: 6.0, y: 6.0 },
            &grid,
            FindPathOptions::default(),
        );
        assert_eq!(
            path.as_ref().and_then(|path| path.last().copied()),
            Some(WorldPos { x: 6.0, y: 6.0 }),
            "the building's own interior work tile must be reachable: {path:?}"
        );
    }

    #[test]
    fn soft_obstacle_routing_is_deterministic_across_identical_runs() {
        let grid = SoftObstacleGrid {
            footprint: square_footprint(5, 7),
            walls: HashSet::new(),
        };
        let a = find_path(
            WorldPos { x: 6.0, y: 0.0 },
            WorldPos { x: 6.0, y: 10.0 },
            &grid,
            FindPathOptions::default(),
        );
        let b = find_path(
            WorldPos { x: 6.0, y: 0.0 },
            WorldPos { x: 6.0, y: 10.0 },
            &grid,
            FindPathOptions::default(),
        );
        assert_eq!(a, b, "identical inputs must produce byte-identical routes");
    }

    #[test]
    fn colony_walk_grid_costs_building_footprints_at_the_soft_obstacle_tier_but_never_blocks() {
        let soft_obstacles: HashSet<TilePos> = [TilePos { x: 5, y: 5 }, TilePos { x: 5, y: 6 }]
            .into_iter()
            .collect();
        let tiles = Vec::new();
        let grid = build_colony_walk_grid(ColonyGridParams {
            tiles: &tiles,
            anchor: TilePos { x: 0, y: 0 },
            ring_radius: 10_000,
            gate: TilePos { x: 0, y: 1 },
            area: None,
            area_gate: None,
            extra_fence_edges: None,
            terrain: None,
            mountains_unlocked: false,
            shipping_researched: false,
            soft_obstacles: Some(&soft_obstacles),
            soft_obstacle_field: None,
            surface_factors: None,
        });

        assert_eq!(grid.cost(5, 5), BUILDING_FOOTPRINT_COST);
        assert!(
            !grid.is_blocked(5, 5),
            "buildings are soft obstacles — never hard-blocked"
        );
        assert_eq!(
            grid.cost(9, 9),
            OPEN_COST,
            "tiles outside any footprint stay at the plain terrain cost"
        );
    }

    #[test]
    fn colony_walk_grid_costs_lazy_generated_footprints_at_the_same_soft_tier() {
        let generated = GeneratedObstacleField {
            footprint: [TilePos { x: 8, y: 9 }].into_iter().collect(),
        };
        let tiles = Vec::new();
        let grid = build_colony_walk_grid(ColonyGridParams {
            tiles: &tiles,
            anchor: TilePos { x: 0, y: 0 },
            ring_radius: 10_000,
            gate: TilePos { x: 0, y: 1 },
            area: None,
            area_gate: None,
            extra_fence_edges: None,
            terrain: None,
            mountains_unlocked: false,
            shipping_researched: false,
            soft_obstacles: None,
            soft_obstacle_field: Some(&generated),
            surface_factors: None,
        });

        assert_eq!(grid.cost(8, 9), BUILDING_FOOTPRINT_COST);
        assert!(!grid.is_blocked(8, 9));
        assert_eq!(grid.cost(9, 9), OPEN_COST);
    }

    #[test]
    fn fine_biome_costs_compose_with_roads_and_soft_obstacles() {
        let tiles = vec![
            WalkTile {
                x: 1,
                y: 0,
                tile_type: WalkTileType::Other,
                overlay_feature: None,
                resources: None,
                path_wear: 0,
            },
            WalkTile {
                x: 2,
                y: 0,
                tile_type: WalkTileType::Other,
                overlay_feature: Some(WalkOverlayFeature::RoadBuilt),
                resources: None,
                path_wear: 0,
            },
            WalkTile {
                x: 3,
                y: 0,
                tile_type: WalkTileType::Other,
                overlay_feature: None,
                resources: None,
                path_wear: ROAD_WEAR_THRESHOLD,
            },
        ];
        let surface_factors = HashMap::from([
            (TilePos { x: 1, y: 0 }, 0.5),
            (TilePos { x: 2, y: 0 }, 0.5),
            (TilePos { x: 3, y: 0 }, 0.5),
        ]);
        let soft_obstacles = HashSet::from([TilePos { x: 2, y: 0 }]);
        let grid = build_colony_walk_grid(ColonyGridParams {
            tiles: &tiles,
            anchor: TilePos { x: 0, y: 0 },
            ring_radius: 10_000,
            gate: TilePos { x: 0, y: 1 },
            area: None,
            area_gate: None,
            extra_fence_edges: None,
            terrain: None,
            mountains_unlocked: false,
            shipping_researched: false,
            soft_obstacles: Some(&soft_obstacles),
            soft_obstacle_field: None,
            surface_factors: Some(&surface_factors),
        });

        assert!((grid.cost(1, 0) - 2.0).abs() < 1e-12);
        assert!((grid.cost(2, 0) - 1.0 / (0.5 * 1.75 * 0.25)).abs() < 1e-12);
        assert!((grid.cost(3, 0) - 1.0 / (0.5 * 1.05)).abs() < 1e-12);
        assert!((grid.cost(4, 0) - 1.0 / SURFACE_FACTOR_GRASSLAND).abs() < 1e-12);
        assert_eq!(tile_cost(None), OPEN_COST);
    }

    #[test]
    fn a_star_chooses_a_longer_fast_biome_over_a_slow_direct_crossing() {
        let mut tiles = Vec::new();
        let mut surface_factors = HashMap::new();
        for y in -3..=3 {
            for x in -3..=9 {
                tiles.push(WalkTile {
                    x,
                    y,
                    tile_type: WalkTileType::Other,
                    overlay_feature: None,
                    resources: None,
                    path_wear: 0,
                });
                let factor = if y == 1 {
                    0.8
                } else if y == 0 && (1..=5).contains(&x) {
                    0.45
                } else {
                    0.75
                };
                surface_factors.insert(TilePos { x, y }, factor);
            }
        }
        let grid = build_colony_walk_grid(ColonyGridParams {
            tiles: &tiles,
            anchor: TilePos { x: 0, y: 0 },
            ring_radius: 10_000,
            gate: TilePos { x: 0, y: 1 },
            area: None,
            area_gate: None,
            extra_fence_edges: None,
            terrain: None,
            mountains_unlocked: false,
            shipping_researched: false,
            soft_obstacles: None,
            soft_obstacle_field: None,
            surface_factors: Some(&surface_factors),
        });

        let path = find_path(
            WorldPos { x: 0.0, y: 0.0 },
            WorldPos { x: 6.0, y: 0.0 },
            &grid,
            FindPathOptions {
                margin: 3,
                ..FindPathOptions::default()
            },
        )
        .expect("fast-biome detour is reachable");
        assert!(path.iter().any(|step| step.y == 1.0), "route={path:?}");
        assert!(
            path.len() > 7,
            "the selected fast route should be longer in tiles: {path:?}"
        );
    }

    #[test]
    fn build_colony_walk_grid_matches_ts_fixture() {
        let checks = fixture().colony_checks;
        let anchor = parse_tile_pos(&checks.anchor);
        let gate = parse_tile_pos(&checks.gate);
        let tiles = checks
            .tiles
            .iter()
            .map(TileFixture::to_walk_tile)
            .collect::<Vec<_>>();
        let grid = build_colony_walk_grid(ColonyGridParams {
            tiles: &tiles,
            anchor,
            ring_radius: checks.ring_radius,
            gate,
            area: None,
            area_gate: None,
            extra_fence_edges: None,
            terrain: None,
            mountains_unlocked: false,
            shipping_researched: false,
            soft_obstacles: None,
            soft_obstacle_field: None,
            surface_factors: None,
        });

        for (key, expected) in checks.blocked {
            let pos = parse_tile_pos(&key);
            assert_eq!(grid.is_blocked(pos.x, pos.y), expected, "blocked {key}");
        }
        for (key, expected) in checks.costs {
            let pos = parse_tile_pos(&key);
            assert_eq!(grid.cost(pos.x, pos.y), expected, "cost {key}");
        }
    }

    #[test]
    fn colony_grid_routes_match_ts_fixture() {
        for case in fixture().colony_path_cases {
            let legacy_anchor = TilePos { x: 6, y: 6 };
            let legacy_gate = TilePos { x: 6, y: 10 };
            let organic_anchor = TilePos { x: 0, y: 0 };
            let organic_gate = TilePos { x: 0, y: 99 };
            let tiles = Vec::new();
            let area = case.area.as_ref().map(|tiles| {
                tiles
                    .iter()
                    .map(|key| parse_tile_pos(key))
                    .collect::<VillageArea>()
            });
            let area_gate = case.area_gate.map(GateFixture::into_gate);
            let (anchor, ring_radius, gate, area_ref) = if case.kind == "legacy" {
                (legacy_anchor, 4, legacy_gate, None)
            } else {
                (organic_anchor, 99, organic_gate, area.as_ref())
            };
            let grid = build_colony_walk_grid(ColonyGridParams {
                tiles: &tiles,
                anchor,
                ring_radius,
                gate,
                area: area_ref,
                area_gate,
                extra_fence_edges: None,
                terrain: None,
                mountains_unlocked: false,
                shipping_researched: false,
                soft_obstacles: None,
                soft_obstacle_field: None,
                surface_factors: None,
            });

            let path = find_path(
                case.start.into_world_pos(),
                case.goal.into_world_pos(),
                &grid,
                options(case.options),
            );
            assert_eq!(path_keys(path.as_ref()), case.expected, "{}", case.name);
        }
    }

    #[test]
    fn cliff_blocks_step_is_inert_like_ts() {
        let grid = HeightGrid;
        assert!(!cliff_blocks_step(&grid, 2, 0, 3, 0));
        assert!(!cliff_blocks_step(&grid, 3, 0, 4, 0));
        assert_eq!(grid.height_at(3, 0), Some(2));
        assert!(!grid.has_stair(3, 0));
    }

    impl PosFixture {
        fn into_world_pos(self) -> WorldPos {
            WorldPos {
                x: self.x,
                y: self.y,
            }
        }
    }

    impl PathCase {
        fn expected_start(&self) -> Option<WorldPos> {
            self.expected
                .as_ref()
                .and_then(|path| path.first())
                .map(|key| parse_tile_pos(key).into_world_pos())
        }

        fn expected_goal(&self) -> Option<WorldPos> {
            self.expected
                .as_ref()
                .and_then(|path| path.last())
                .map(|key| parse_tile_pos(key).into_world_pos())
        }
    }

    impl TilePos {
        fn into_world_pos(self) -> WorldPos {
            WorldPos {
                x: f64::from(self.x),
                y: f64::from(self.y),
            }
        }
    }

    impl TileFixture {
        fn to_walk_tile(&self) -> WalkTile {
            WalkTile {
                x: self.x,
                y: self.y,
                tile_type: match self.tile_type.as_deref() {
                    Some("river") => WalkTileType::River,
                    Some("forest") => WalkTileType::Forest,
                    Some("dense_woods") => WalkTileType::DenseWoods,
                    _ => WalkTileType::Other,
                },
                overlay_feature: self
                    .overlay_feature
                    .as_deref()
                    .map(|feature| match feature {
                        "river" => WalkOverlayFeature::River,
                        "road_built" => WalkOverlayFeature::RoadBuilt,
                        "bridge_built_h" | "bridge_built_v" => WalkOverlayFeature::BridgeBuilt,
                        "game_trail" => WalkOverlayFeature::GameTrail,
                        "ancient_road" => WalkOverlayFeature::AncientRoad,
                        "trade_route" => WalkOverlayFeature::TradeRoute,
                        _ => WalkOverlayFeature::Other,
                    }),
                resources: self.resources.as_ref().map(|resources| WalkTileResources {
                    water: resources.water,
                }),
                path_wear: self.path_wear,
            }
        }
    }

    impl GateFixture {
        fn into_gate(self) -> GatePlacement {
            GatePlacement {
                x: self.x,
                y: self.y,
                side: match self.side {
                    FenceSideFixture::N => FenceSide::N,
                    FenceSideFixture::E => FenceSide::E,
                    FenceSideFixture::S => FenceSide::S,
                    FenceSideFixture::W => FenceSide::W,
                },
            }
        }
    }

    fn options(options: Option<OptionFixture>) -> FindPathOptions {
        let mut out = FindPathOptions::default();
        if let Some(options) = options {
            if let Some(max_expansions) = options.max_expansions {
                out.max_expansions = max_expansions;
            }
            if let Some(margin) = options.margin {
                out.margin = margin;
            }
        }
        out
    }

    fn test_grid(spec: &GridSpec) -> TestGrid {
        match spec.kind.as_str() {
            "open" => TestGrid {
                blocked: HashSet::new(),
                costs: HashMap::new(),
                roads: HashSet::new(),
            },
            "sets" | "costs" => TestGrid {
                blocked: spec.blocked.iter().cloned().collect(),
                costs: spec.costs.clone(),
                roads: spec.roads.iter().cloned().collect(),
            },
            "maze" => {
                let mut blocked = HashSet::new();
                for y in (1..=40).step_by(2) {
                    for x in 0..=39 {
                        if x != if y % 4 == 1 { 39 } else { 0 } {
                            blocked.insert(coord_key(x, y));
                        }
                    }
                }
                TestGrid {
                    blocked,
                    costs: HashMap::new(),
                    roads: HashSet::new(),
                }
            }
            "matrix" => {
                let mut rng = Mulberry32::new(spec.rng_seed.expect("matrix fixture has rng seed"));
                let mut blocked = HashSet::new();
                let mut costs = HashMap::new();
                for x in -2..=12 {
                    for y in -2..=12 {
                        let roll = rng.next();
                        match spec.mode.as_deref() {
                            Some("determinism") => {
                                if roll < 0.2 {
                                    blocked.insert(coord_key(x, y));
                                } else if roll < 0.35 {
                                    costs.insert(coord_key(x, y), FOREST_COST);
                                }
                            }
                            Some("sweep") => {
                                if roll < 0.25 {
                                    blocked.insert(coord_key(x, y));
                                } else if roll < 0.4 {
                                    costs.insert(coord_key(x, y), FOREST_COST);
                                } else if roll < 0.5 {
                                    costs.insert(coord_key(x, y), ROAD_COST);
                                }
                            }
                            _ => panic!("unknown matrix mode"),
                        }
                    }
                }
                TestGrid {
                    blocked,
                    costs,
                    roads: HashSet::new(),
                }
            }
            _ => panic!("unknown grid fixture kind {}", spec.kind),
        }
    }

    fn path_keys(path: Option<&Vec<WorldPos>>) -> Option<Vec<String>> {
        path.map(|path| {
            path.iter()
                .map(|pos| {
                    assert_eq!(pos.x.fract(), 0.0, "path x coordinate is integral");
                    assert_eq!(pos.y.fract(), 0.0, "path y coordinate is integral");
                    coord_key(pos.x as i32, pos.y as i32)
                })
                .collect()
        })
    }

    fn assert_contiguous(path: &[WorldPos], name: &str) {
        for window in path.windows(2) {
            let dx = (window[1].x - window[0].x).abs();
            let dy = (window[1].y - window[0].y).abs();
            assert_eq!(dx + dy, 1.0, "{name}");
        }
    }

    fn coord_key(x: i32, y: i32) -> String {
        format!("{x},{y}")
    }

    fn parse_tile_pos(key: &str) -> TilePos {
        let (x, y) = key.split_once(',').expect("tile key contains comma");
        TilePos {
            x: x.parse().expect("x parses"),
            y: y.parse().expect("y parses"),
        }
    }

    struct Mulberry32 {
        state: u32,
    }

    impl Mulberry32 {
        fn new(seed: u32) -> Self {
            Self { state: seed }
        }

        fn next(&mut self) -> f64 {
            self.state = self.state.wrapping_add(0x6d2b79f5);
            let mut t = (self.state ^ (self.state >> 15)).wrapping_mul(1 | self.state);
            t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
            f64::from(t ^ (t >> 14)) / 4_294_967_296.0
        }
    }
}
