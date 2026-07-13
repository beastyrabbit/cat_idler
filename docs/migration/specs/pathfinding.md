# P3.1 Pathfinding Port Spec

> **Historical parity spec.** This records the frozen TypeScript behavior used for the Rust port.
> Later P14/P16 soft-obstacle, footprint, and road requirements supersede it where they differ;
> current gaps are tracked in [`docs/IMPLEMENTATION_AUDIT.md`](../../IMPLEMENTATION_AUDIT.md).

Sources read:
- `lib/game/pathfinding.ts`
- `tests/unit/game/pathfinding.test.ts`
- `lib/game/movement.ts` for `WorldPos`
- `lib/game/villageArea.ts` for organic fence blocking

Target Rust module: `crates/cat-sim/src/pathfinding.rs`.

## Purpose

Port the pure, deterministic A* walk planner used by movement, road wear, and
colony fence traversal. It returns strict 4-neighbor tile routes shaped by roads,
worn paths, water, forest cost, and palisade blocking.

The pathfinder does not move cats by itself. On `None`/`null`, callers fall back
to the old straight x-before-y walk.

## Rust Public Surface

Use these snake_case Rust names for the TS exports. Helper structs can be adapted
to existing crate coordinate types, but the names below should be the public API
of `pathfinding.rs`.

```rust
use crate::movement::WorldPos;
use crate::village_area::{GatePlacement, VillageArea};

pub const ROAD_COST: f64 = 0.4;
pub const WORN_PATH_COST: f64 = 0.6;
pub const OPEN_COST: f64 = 1.0;
pub const FOREST_COST: f64 = 4.0;
pub const DENSE_WOODS_COST: f64 = 8.0;
pub const MIN_STEP_COST: f64 = ROAD_COST;

pub const DEFAULT_MAX_EXPANSIONS: usize = 6000;
pub const DEFAULT_MARGIN: i32 = 16;
pub const ROAD_WEAR_THRESHOLD: u32 = 70;

const X_FIRST_BIAS: f64 = 1e-6;
const NEIGHBOURS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
}

pub trait WalkGrid {
    fn is_blocked(&self, x: i32, y: i32) -> bool;
    fn cost(&self, x: i32, y: i32) -> f64;

    fn height_at(&self, _x: i32, _y: i32) -> Option<i32> { None }
    fn has_stair(&self, _x: i32, _y: i32) -> bool { false }
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
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkOverlayFeature {
    River,
    RoadBuilt,
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

pub struct ColonyGridParams<'a> {
    pub tiles: &'a [WalkTile],
    pub anchor: TilePos,
    pub ring_radius: i32,
    pub gate: TilePos,
    pub area: Option<&'a VillageArea>,
    pub area_gate: Option<GatePlacement>,
    pub terrain: Option<&'a dyn TerrainWalkField>,
}

pub struct ColonyWalkGrid<'a> {
    // Private fields: packed tile map, legacy ring data, optional area/terrain refs.
    _lifetime: std::marker::PhantomData<&'a ()>,
}

pub fn cliff_blocks_step<G: WalkGrid + ?Sized>(
    grid: &G,
    ax: i32,
    ay: i32,
    bx: i32,
    by: i32,
) -> bool;

pub fn build_colony_walk_grid(params: ColonyGridParams<'_>) -> ColonyWalkGrid<'_>;

pub fn find_path<G: WalkGrid + ?Sized>(
    start: WorldPos,
    goal: WorldPos,
    grid: &G,
    options: FindPathOptions,
) -> Option<Vec<WorldPos>>;
```

`WorldPos` in `movement.rs` can carry fractional coordinates. `find_path`
rounds `start` and `goal` to integer tile coordinates first, then returns
integer tile positions stored back in `WorldPos`. Match JavaScript `Math.round`,
not Rust `f64::round`, for negative halves: JS is equivalent to
`floor(x + 0.5)` with `-0` becoming `0`. This matters for values like `-1.5`,
where JS returns `-1`.

If the crate already has `TileType` or `OverlayFeature` enums, do not broaden
the cost behavior accidentally. The TS checks exact string literals:
`"forest"` and `"dense_woods"` are costly, but `"oak_forest"`,
`"pine_forest"`, `"jungle"`, and other biome strings are open-cost unless
callers map them to the exact `WalkTileType` variants above. `road_built` is a
pathfinding overlay even though it is not one of the natural path overlays.

## Public Surface Semantics

### `WalkGrid`

TS:

```ts
export interface WalkGrid {
  isBlocked(x: number, y: number): boolean;
  cost(x: number, y: number): number;
  heightAt?(x: number, y: number): number;
  hasStair?(x: number, y: number): boolean;
  fenceBlocksStep?(fx: number, fy: number, tx: number, ty: number): boolean;
}
```

Semantics:
- `is_blocked` is a tile-entry test for rivers/water and the legacy square
  fence ring.
- `cost` returns the relative cost to enter a tile. It is paid for every entered
  tile, including the goal in the full A* search. The start tile has `g = 0` and
  its cost is not paid.
- `height_at` and `has_stair` are still part of the interface, but current TS
  pathfinding ignores them because `cliffBlocksStep` always returns `false`.
- `fence_blocks_step` is an edge-crossing test for the organic village fence.
  It is checked even when stepping into the goal during full A* search.

### `cliff_blocks_step`

TS export:

```ts
export function cliffBlocksStep(
  grid: WalkGrid,
  ax: number,
  ay: number,
  bx: number,
  by: number,
): boolean
```

Current behavior is always `false`, regardless of height or stairs. Port this
exactly. The comments describe a future elevation seam; do not reintroduce
cliff blocking in P3.1.

### `FindPathOptions`

TS:

```ts
export interface FindPathOptions {
  maxExpansions?: number;
  margin?: number;
}
```

Defaults:
- `maxExpansions` missing -> `DEFAULT_MAX_EXPANSIONS = 6000`
- `margin` missing -> `DEFAULT_MARGIN = 16`

Rust should use `FindPathOptions::default()` for the no-options call. Tests can
override fields with struct update syntax.

### `WalkTile`

TS:

```ts
export interface WalkTile {
  x: number;
  y: number;
  type: string;
  overlayFeature?: string | null;
  resources?: { water?: number } | null;
  pathWear: number;
}
```

Only these fields are read:
- `type === "river"` blocks water.
- `type === "forest"` costs `FOREST_COST`.
- `type === "dense_woods"` costs `DENSE_WOODS_COST`.
- `overlayFeature === "river"` blocks water.
- `overlayFeature === "road_built"` costs `ROAD_COST`.
- `overlayFeature` in `{"game_trail","ancient_road","trade_route"}` costs
  `WORN_PATH_COST`.
- `(resources?.water ?? 0) > 0` blocks water.
- `pathWear >= ROAD_WEAR_THRESHOLD` costs `WORN_PATH_COST`.

Unknown or missing tiles are walkable open ground at `OPEN_COST`.

### `ColonyGridParams` and `build_colony_walk_grid`

TS:

```ts
export interface ColonyGridParams {
  tiles: WalkTile[];
  anchor: WorldPos;
  ringRadius: number;
  gate: WorldPos;
  area?: VillageArea;
  areaGate?: GatePlacement | null;
  terrain?: {
    heightAt(x: number, y: number): number;
    hasStair(x: number, y: number): boolean;
  };
}

export function buildColonyWalkGrid(params: ColonyGridParams): WalkGrid
```

Semantics:
- The returned grid owns or closes over a map of known `tiles`.
- Tile lookup in TS packs coordinates as
  `(x + (1 << 15)) * (1 << 16) + (y + (1 << 15))`.
  The offset is `32768`; the stride is `65536`.
- If duplicate packed keys occur, later tiles in `params.tiles` overwrite
  earlier ones. This includes the theoretical TS collision behavior for
  coordinates separated by the `65536` stride; replicate it if strict parity is
  required for extreme coordinates.
- When `area` is absent, the legacy square palisade blocks by tile entry:
  `max(abs(x - anchor.x), abs(y - anchor.y)) == ringRadius` and the tile is not
  exactly `gate`.
- The legacy gate only exempts the ring test. If a gate tile is water, water
  still blocks it.
- When `area` is present, skip the legacy ring test entirely. The organic fence
  is enforced only via `fence_blocks_step`, using
  `village_area::fence_blocks_move(from, to, area, area_gate)`.
- If `area` is present and `area_gate` is `None`, every boundary crossing
  blocks.
- Water blocks in both legacy and organic modes:
  `type == River`, `overlay_feature == River`, or `resources.water > 0`.
- `cost` delegates to the tile cost model below.
- Optional `terrain` is forwarded through `height_at`/`has_stair`, but the
  current cliff blocker ignores it.

### `find_path`

TS:

```ts
export function findPath(
  start: WorldPos,
  goal: WorldPos,
  grid: WalkGrid,
  options: FindPathOptions = {},
): WorldPos[] | null
```

Semantics:
- Round `start` and `goal` with JS `Math.round`.
- Return `[start]` when rounded start equals rounded goal.
- Return `[start, goal]` immediately when Manhattan distance is `1`.
  This shortcut bypasses `isBlocked`, `cost`, `cliffBlocksStep`, and
  `fenceBlocksStep`. This looks like a fence/cliff edge-case bug, but must be
  replicated for parity.
- Otherwise always run A*. There is no straight-line fast path.
- The returned path is start first, goal last, and every pair is a strict
  4-neighbor step. No diagonals and no jumps.
- Return `None`/`null` when no route is found inside the bounded search or when
  expansion budget is exhausted. The caller owns fallback.
- Start and goal are always enterable for tile blocking in full A*:
  the start tile is never checked, and `is_blocked(goal)` is skipped.
  However, full A* still checks `cliff_blocks_step` and `fence_blocks_step`
  when stepping into the goal. In current TS, cliffs never block; organic fences
  do.

## Constants

Path cost constants:
- `ROAD_COST = 0.4`
- `WORN_PATH_COST = 0.6`
- `OPEN_COST = 1`
- `FOREST_COST = 4`
- `DENSE_WOODS_COST = 8`
- `MIN_STEP_COST = ROAD_COST = 0.4`

Search constants:
- `X_FIRST_BIAS = 1e-6` (private in TS, but port the exact value)
- `DEFAULT_MAX_EXPANSIONS = 6000`
- `DEFAULT_MARGIN = 16`
- `NEIGHBOURS = [[1,0], [-1,0], [0,1], [0,-1]]`

Colony grid constants:
- `ROAD_WEAR_THRESHOLD = 70`
- `NATURAL_PATH_OVERLAYS = {"game_trail", "ancient_road", "trade_route"}`
- tile map offset `OFFSET = 1 << 15 = 32768`
- tile map stride `1 << 16 = 65536`

Village area dependency constants from `villageArea.ts`:
- Boundary side order is `N, E, S, W`.
- Side deltas are `N=(0,-1)`, `E=(1,0)`, `S=(0,1)`, `W=(-1,0)`.
- A `GatePlacement` is `{ x, y, side }` on the inside tile's boundary edge.

## Cost Model

Cost to enter a tile is resolved in this exact order:

1. Unknown tile -> `OPEN_COST`.
2. `overlayFeature === "road_built"` -> `ROAD_COST`.
3. `pathWear >= ROAD_WEAR_THRESHOLD` -> `WORN_PATH_COST`.
4. `overlayFeature` is one of `game_trail`, `ancient_road`, `trade_route`
   -> `WORN_PATH_COST`.
5. `type === "dense_woods"` -> `DENSE_WOODS_COST`.
6. `type === "forest"` -> `FOREST_COST`.
7. Everything else -> `OPEN_COST`.

Priority matters. A `road_built` forest costs `0.4`; a forest with
`pathWear = 70` costs `0.6`; a water goal with no road/wear/forest marker costs
open ground if the full A* route enters it as the goal.

## A* Algorithm Notes

Coordinate setup:
- `sx = Math.round(start.x)`, `sy = Math.round(start.y)`,
  `gx = Math.round(goal.x)`, `gy = Math.round(goal.y)`.
- Search bounds are the inclusive start/goal bounding box plus margin:
  `minX = min(sx,gx) - margin`, `maxX = max(sx,gx) + margin`,
  `minY = min(sy,gy) - margin`, `maxY = max(sy,gy) + margin`.
- `width = maxX - minX + 1`.
- Node key in the bounded search arrays:
  `(y - minY) * width + (x - minX)`.
- TS does not guard negative margins or overflowing sizes. Normal callers use
  non-negative margins.

State:
- `gScore` starts as positive infinity for every node.
- `cameFrom` starts as `-1`.
- `closed` starts false.
- Start key gets `gScore = 0`.
- Initial heap push uses
  `f = manhattan(start, goal) * MIN_STEP_COST`.
- Duplicate heap entries are allowed. A stale duplicate is ignored if its key is
  already closed when popped.

Loop ordering:
1. Pop heap minimum.
2. If already closed, continue.
3. Mark current closed.
4. If current is goal, reconstruct and return immediately.
5. Increment `expansions`.
6. If `expansions > maxExpansions`, return `None`.
7. Visit neighbors in this exact order: east, west, south, north.
8. Skip neighbor outside bounds.
9. If neighbor is not the goal and `grid.is_blocked(nx, ny)`, skip.
10. If `cliff_blocks_step(grid, cx, cy, nx, ny)`, skip. This is currently always
    false.
11. If `grid.fence_blocks_step(cx, cy, nx, ny)`, skip.
12. If neighbor key is closed, skip.
13. Compute the x-first tie-break penalty:
    `premature_y = X_FIRST_BIAS` only when `dy != 0 && cx != gx`; otherwise `0`.
14. `tentative = gScore[current] + grid.cost(nx, ny) + premature_y`.
15. Update only on strict improvement:
    `if tentative < gScore[neighbor]`.
16. On update, set `gScore`, set `cameFrom`, and push
    `tentative + manhattan(neighbor, goal) * MIN_STEP_COST`.

The expansion budget counts closed, non-goal nodes only because the goal check
happens before `expansions += 1`. A route that pops the goal returns even if the
next expansion would exceed the budget. Budget failure uses `>` rather than
`>=`, so `max_expansions = 20` permits 20 non-goal expansions and fails on the
21st.

Reconstruction:
- Walk `cameFrom` from goal key to `-1`.
- Convert each key back with:
  `x = (key % width) + minX`,
  `y = floor(key / width) + minY`.
- Reverse the list.

## Heap Tie-Break Semantics

The TS heap is a binary min-heap over records `{ key, f, seq }`.

Ordering:
1. Lower `f` pops first.
2. If `f === f`, lower insertion `seq` pops first.

`seq` starts at `0` and increments on every `push`, including duplicate keys.
There is no secondary tie-break on key, x/y, h-score, or g-score. The neighbor
order plus insertion-sequence tie-break is required for byte-identical routes.

Rust implementation notes:
- Use `f64` scores.
- Scores should never be NaN. If they are, TS `before` would treat comparisons
  as false; do not add a different NaN ordering.
- Implement comparison as `a.f < b.f || (a.f == b.f && a.seq < b.seq)`.
- Do not use `BinaryHeap` with only `f` ordering unless the `seq` tie-break is
  included. Standard heap stability is not enough.
- Because the ordering is total for all non-NaN pushed records, any binary heap
  implementation with this comparator should reproduce the TS pop order.

## Determinism

Production pathfinding uses no RNG, no seeded LCG, and no `Math.random`.

Determinism depends on:
- exact JS rounding for start/goal;
- fixed neighbor order `E, W, S, N`;
- strict `<` improvement, preserving the first equal-cost predecessor;
- x-before-y bias in `gScore`;
- deterministic heap sequence tie-break;
- deterministic `build_colony_walk_grid` tile map construction, including
  last-tile-wins on duplicate packed keys.

The randomized unit tests use a local `mulberry32` PRNG only to generate replayed
test grids. Do not use that PRNG in production pathfinding.

## Golden Fixtures To Generate

Recommended fixture file: `docs/migration/fixtures/p3/pathfinding.json`.

Represent paths as arrays of `"x,y"` strings for compact byte comparison. The
Rust tests should convert `Vec<WorldPos>` into the same strings after asserting
that every returned coordinate is an integer.

### Hand-Checkable Route Fixtures

Use these helper grids:
- `open_grid`: `is_blocked = false`, `cost = OPEN_COST`.
- `grid_from(blocked, roads)`: blocked set by `"x,y"`; road set costs
  `ROAD_COST`; all other tiles cost `OPEN_COST`.
- `cost_grid(costs, blocked)`: blocked set by `"x,y"`; cost map overrides
  specific tiles; all other tiles cost `OPEN_COST`.

Fixtures:

```json
[
  {
    "name": "same_tile",
    "grid": "open_grid",
    "start": "3,3",
    "goal": "3,3",
    "options": "default",
    "expected": ["3,3"]
  },
  {
    "name": "adjacent_blocked_goal_shortcut",
    "grid": { "blocked": ["1,0"] },
    "start": "0,0",
    "goal": "1,0",
    "options": "default",
    "expected": ["0,0", "1,0"]
  },
  {
    "name": "blocked_fractional_start_goal_enterable",
    "grid": { "blocked": ["0,0", "3,0"] },
    "start": "0.2,-0.2",
    "goal": "3.49,0.49",
    "options": { "margin": 2 },
    "expected": ["0,0", "1,0", "2,0", "3,0"]
  },
  {
    "name": "open_l_positive",
    "grid": "open_grid",
    "start": "12,6",
    "goal": "20,14",
    "options": "default",
    "expected": [
      "12,6", "13,6", "14,6", "15,6", "16,6", "17,6", "18,6",
      "19,6", "20,6", "20,7", "20,8", "20,9", "20,10",
      "20,11", "20,12", "20,13", "20,14"
    ]
  },
  {
    "name": "open_l_negative",
    "grid": "open_grid",
    "start": "3,3",
    "goal": "-1,0",
    "options": "default",
    "expected": [
      "3,3", "2,3", "1,3", "0,3", "-1,3", "-1,2", "-1,1", "-1,0"
    ]
  },
  {
    "name": "water_wall_gap",
    "grid": {
      "blocked": [
        "15,4", "15,5", "15,6", "15,7", "15,8", "15,9", "15,10",
        "15,11", "15,12", "15,13", "15,14", "15,15", "15,16",
        "15,17", "15,18", "15,19", "15,20"
      ]
    },
    "start": "12,6",
    "goal": "20,8",
    "options": "default",
    "expected": [
      "12,6", "13,6", "14,6", "14,5", "14,4", "14,3", "15,3",
      "16,3", "17,3", "18,3", "19,3", "20,3", "20,4", "20,5",
      "20,6", "20,7", "20,8"
    ]
  },
  {
    "name": "equal_detour_prefers_road",
    "grid": {
      "blocked": ["0,1", "0,2", "0,3"],
      "roads": ["1,0", "1,1", "1,2", "1,3", "1,4"]
    },
    "start": "0,0",
    "goal": "0,4",
    "options": { "margin": 3 },
    "expected": ["0,0", "1,0", "1,1", "1,2", "1,3", "1,4", "0,4"]
  },
  {
    "name": "longer_road_cheaper",
    "grid": {
      "road_cost_tiles": ["0,1", "1,1", "2,1", "3,1", "4,1", "5,1", "6,1"]
    },
    "start": "0,0",
    "goal": "6,0",
    "options": { "margin": 3 },
    "expected": [
      "0,0", "0,1", "1,1", "2,1", "3,1", "4,1", "5,1", "6,1", "6,0"
    ]
  },
  {
    "name": "forest_detour",
    "grid": { "forest_cost_tiles": ["2,0", "3,0", "4,0"] },
    "start": "0,0",
    "goal": "6,0",
    "options": { "margin": 3 },
    "expected": [
      "0,0", "1,0", "1,1", "2,1", "3,1", "4,1", "5,1", "6,1", "6,0"
    ]
  },
  {
    "name": "small_premium_straight",
    "grid": { "custom_costs": { "3,0": 1.4 } },
    "start": "0,0",
    "goal": "6,0",
    "options": { "margin": 3 },
    "expected": ["0,0", "1,0", "2,0", "3,0", "4,0", "5,0", "6,0"]
  },
  {
    "name": "walled_goal_null",
    "grid": {
      "blocked": ["9,10", "11,10", "10,9", "10,11", "9,9", "11,11", "9,11", "11,9"]
    },
    "start": "2,2",
    "goal": "10,10",
    "options": { "margin": 3 },
    "expected": null
  },
  {
    "name": "budget_exhausted_null",
    "grid": {
      "blocked_rule": "for odd y in 1..=40, set gap_x = 39 when y % 4 == 1 else 0; for x in 0..=39, block every x != gap_x"
    },
    "start": "0,0",
    "goal": "39,40",
    "options": { "maxExpansions": 20, "margin": 4 },
    "expected": null
  }
]
```

### `build_colony_walk_grid` Fixtures

Water and cost fixtures:

```json
{
  "anchor": "6,6",
  "ringRadius": 4,
  "gate": "6,10",
  "tiles": [
    { "x": 0, "y": 0, "type": "river", "pathWear": 0, "blocked": true },
    { "x": 1, "y": 0, "resources": { "water": 5 }, "pathWear": 0, "blocked": true },
    { "x": 2, "y": 0, "overlayFeature": "river", "pathWear": 0, "blocked": true },
    { "x": 3, "y": 0, "type": "grass", "pathWear": 0, "blocked": false },
    { "x": 4, "y": 0, "overlayFeature": "road_built", "pathWear": 0, "cost": 0.4 },
    { "x": 5, "y": 0, "type": "grass", "pathWear": 80, "cost": 0.6 },
    { "x": 6, "y": 0, "overlayFeature": "game_trail", "pathWear": 0, "cost": 0.6 },
    { "x": 7, "y": 0, "type": "forest", "pathWear": 0, "cost": 4.0 },
    { "x": 8, "y": 0, "type": "dense_woods", "pathWear": 0, "cost": 8.0 }
  ],
  "unknownCostAt": { "pos": "50,50", "cost": 1.0 },
  "legacyFenceBlocked": ["10,6", "6,2"],
  "legacyFenceOpen": ["6,10", "6,6", "6,20"]
}
```

Legacy fence route fixture:

```json
{
  "name": "legacy_fence_gate",
  "buildColonyWalkGrid": {
    "tiles": [],
    "anchor": "6,6",
    "ringRadius": 4,
    "gate": "6,10"
  },
  "start": "6,6",
  "goal": "6,16",
  "options": "default",
  "expected": [
    "6,6", "6,7", "6,8", "6,9", "6,10", "6,11",
    "6,12", "6,13", "6,14", "6,15", "6,16"
  ]
}
```

Organic fence route fixture:

```json
{
  "name": "organic_fence_gate",
  "area": [
    "-1,-1", "0,-1", "1,-1",
    "-1,0", "0,0", "1,0",
    "-1,1", "0,1", "1,1"
  ],
  "areaGate": { "x": 0, "y": 1, "side": "S" },
  "anchor": "0,0",
  "ringRadius": 99,
  "gate": "0,99",
  "start": "0,0",
  "goal": "0,3",
  "options": { "margin": 4 },
  "expected": ["0,0", "0,1", "0,2", "0,3"]
}
```

Also test that the same organic area with `areaGate = null` returns `null` for
the route from `"0,0"` to `"0,3"` within `margin = 4`.

### Seeded Grid Matrix For Byte-Identical Routes

These grids are for tests only. They use the unit-test `mulberry32`, not the
cat-sim LCG:

```ts
function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
```

For each matrix case:
- Loop `x` outer from `-2` to `12` inclusive.
- Loop `y` inner from `-2` to `12` inclusive.
- Start is `"0,0"`, goal is `"10,10"`, options are `{ "margin": 4 }`.
- `determinism` mode: if `r < 0.2`, block the tile; else if `r < 0.35`, set
  tile cost to `FOREST_COST`.
- `sweep` mode: if `r < 0.25`, block the tile; else if `r < 0.4`, set cost to
  `FOREST_COST`; else if `r < 0.5`, set cost to `ROAD_COST`.

Use this expected matrix:

```json
[
  {
    "name": "determinism_rng_seed_8",
    "mode": "determinism",
    "rngSeed": 8,
    "expected": [
      "0,0", "1,0", "1,1", "2,1", "3,1", "4,1", "4,2",
      "4,3", "4,4", "4,5", "4,6", "5,6", "5,7", "6,7",
      "7,7", "7,8", "8,8", "9,8", "10,8", "10,9", "10,10"
    ]
  },
  {
    "name": "determinism_rng_seed_15",
    "mode": "determinism",
    "rngSeed": 15,
    "expected": [
      "0,0", "1,0", "2,0", "3,0", "3,1", "3,2", "3,3",
      "3,4", "3,5", "4,5", "5,5", "6,5", "7,5", "8,5",
      "8,6", "8,7", "8,8", "8,9", "9,9", "9,10", "10,10"
    ]
  },
  {
    "name": "determinism_rng_seed_22",
    "mode": "determinism",
    "rngSeed": 22,
    "expected": [
      "0,0", "1,0", "2,0", "3,0", "4,0", "5,0", "6,0",
      "7,0", "7,1", "7,2", "7,3", "8,3", "8,4", "9,4",
      "10,4", "10,5", "10,6", "10,7", "11,7", "11,8",
      "11,9", "10,9", "10,10"
    ]
  },
  {
    "name": "determinism_rng_seed_43",
    "mode": "determinism",
    "rngSeed": 43,
    "expected": [
      "0,0", "-1,0", "-1,1", "-1,2", "0,2", "0,3", "0,4",
      "0,5", "0,6", "0,7", "0,8", "0,9", "0,10", "1,10",
      "2,10", "2,11", "3,11", "4,11", "5,11", "6,11",
      "7,11", "7,10", "8,10", "9,10", "10,10"
    ]
  },
  {
    "name": "determinism_rng_seed_64",
    "mode": "determinism",
    "rngSeed": 64,
    "expected": [
      "0,0", "-1,0", "-1,1", "-1,2", "-1,3", "0,3", "0,4",
      "1,4", "2,4", "3,4", "4,4", "4,5", "4,6", "5,6",
      "6,6", "7,6", "7,5", "8,5", "9,5", "10,5", "10,6",
      "10,7", "10,8", "10,9", "10,10"
    ]
  },
  {
    "name": "sweep_rng_seed_1",
    "mode": "sweep",
    "rngSeed": 1,
    "expected": [
      "0,0", "0,1", "0,2", "-1,2", "-2,2", "-3,2", "-3,3",
      "-3,4", "-2,4", "-2,5", "-2,6", "-1,6", "-1,7",
      "-1,8", "0,8", "1,8", "1,9", "1,10", "1,11", "1,12",
      "2,12", "3,12", "3,13", "4,13", "5,13", "6,13",
      "7,13", "8,13", "9,13", "10,13", "10,12", "10,11",
      "10,10"
    ]
  },
  {
    "name": "sweep_rng_seed_2",
    "mode": "sweep",
    "rngSeed": 2,
    "expected": [
      "0,0", "1,0", "2,0", "3,0", "4,0", "5,0", "5,1",
      "6,1", "6,2", "6,3", "6,4", "7,4", "7,5", "7,6",
      "7,7", "7,8", "7,9", "7,10", "8,10", "9,10", "10,10"
    ]
  },
  {
    "name": "sweep_rng_seed_3",
    "mode": "sweep",
    "rngSeed": 3,
    "expected": [
      "0,0", "1,0", "1,-1", "2,-1", "3,-1", "4,-1", "5,-1",
      "6,-1", "7,-1", "8,-1", "9,-1", "10,-1", "10,0",
      "11,0", "11,1", "11,2", "11,3", "12,3", "12,4",
      "12,5", "12,6", "12,7", "11,7", "10,7", "10,8",
      "10,9", "10,10"
    ]
  }
]
```

## Dependencies

Must exist first or be ported alongside this module:
- `crates/cat-sim/src/movement.rs`: `WorldPos` and JS-compatible rounding
  helper if `WorldPos` stores `f64`.
- `crates/cat-sim/src/village_area.rs`: `VillageArea`, `GatePlacement`,
  `fence_blocks_move`, and side/gate semantics from `lib/game/villageArea.ts`.

Optional or adapter dependencies:
- `crates/cat-sim/src/types.rs` / `biomes.rs` if the implementation maps from
  existing game tile and overlay enums into `WalkTile`.
- `crates/cat-sim/src/terrain_gen.rs` only for a terrain adapter implementing
  `TerrainWalkField`; current cliff behavior does not depend on terrain.

No dependency on `rng.rs`, `rand`, time, filesystem, rendering, or networking.
