# P2.4 Terrain Generator Port Spec

Source read: `lib/game/terrainGen.ts` and `tests/unit/game/terrainGen.test.ts`.

## Purpose

Port the pure, deterministic abstract terrain-role generator used by the TS client
and terrain-backed world layer. Target Rust module: `crates/cat-sim/src/terrain_gen.rs`.

The Rust module must emit the same height, moisture, biome, cliff, stair, river,
and decoration roles from `(world coordinates, seed, options)`. It does not choose
sprite filenames and must stay independent of the legacy Voronoi `world_gen` logic.

## Rust Public Surface

Expose snake_case Rust APIs that match the TS exports. Helper functions may be
`pub(crate)` plus unit-tested in-module, but their names and behavior should stay
visible enough for P2.6 parity tests.

```rust
pub const TERRAIN_CHUNK_SIZE: i32 = 12;
pub const DEFAULT_MAX_HEIGHT: i32 = 3;
pub const DIRECTIONS: [Direction; 4] = [
    Direction::N,
    Direction::E,
    Direction::S,
    Direction::W,
];

pub enum Direction { N, E, S, W }
pub enum BiomeRole { Lowland, Grassland, Forest, Rocky, Highland }
pub enum CliffBase { Edge, Corner, Ridge, Spur, Pillar }
pub enum RockSize { Small, Medium, Large }
pub enum RiverSegment { Start, Straight, Bend, End }

pub enum TerrainRole {
    Flat,
    Cliff(CliffTerrainRole),
}

pub struct CliffTerrainRole {
    pub edges: u8,
    pub base: CliffBase,
    pub variant: String,
    pub facing: Option<Direction>,
    pub max_drop: i32,
}

pub enum DecorationRole {
    Tree { species: i32 },
    Rock { size: RockSize, resource: bool },
}

pub struct RiverRole {
    pub segment: RiverSegment,
    pub in_dir: Option<Direction>,
    pub out_dir: Option<Direction>,
    pub facing: Direction,
}

pub struct StairsRole {
    pub facing: Direction,
}

pub struct TerrainTile {
    pub x: i32,
    pub y: i32,
    pub elevation: f64,
    pub moisture: f64,
    pub height: i32,
    pub biome: BiomeRole,
    pub terrain: TerrainRole,
    pub river: Option<RiverRole>,
    pub stairs: Option<StairsRole>,
    pub decoration: Option<DecorationRole>,
}

pub struct TerrainOptions {
    pub max_height: Option<i32>,
    pub height_scale: Option<f64>,
    pub octaves: Option<i32>,
    pub persistence: Option<f64>,
    pub moisture_scale: Option<f64>,
    pub village_anchor: Option<Point>,
    pub plateau_radius: Option<i32>,
    pub plateau_height: Option<i32>,
    pub min_run_for_stair: Option<i32>,
    pub region_size: Option<i32>,
    pub rivers_per_region: Option<i32>,
    pub max_river_length: Option<i32>,
    pub river_source_min_elevation: Option<f64>,
    pub carve_rivers: Option<bool>,
    pub decorate: Option<bool>,
}

pub struct Point {
    pub x: i32,
    pub y: i32,
}

pub const WORLD_TERRAIN_OPTIONS: TerrainOptions = ...;

pub fn terrain_elevation_at(x: i32, y: i32, seed: i64, opts: TerrainOptions) -> f64;
pub fn terrain_moisture_at(x: i32, y: i32, seed: i64, opts: TerrainOptions) -> f64;
pub fn terrain_height_at(x: i32, y: i32, seed: i64, opts: TerrainOptions) -> i32;
pub fn terrain_stair_at(x: i32, y: i32, seed: i64, opts: TerrainOptions) -> bool;

pub struct NeighborHeights {
    pub n: i32,
    pub e: i32,
    pub s: i32,
    pub w: i32,
}

pub fn classify_cliff(center: i32, neighbors: NeighborHeights) -> TerrainRole;

pub struct RiverPathTile {
    pub x: i32,
    pub y: i32,
    pub in_dir: Option<Direction>,
    pub out_dir: Option<Direction>,
}

pub fn region_river_sources(
    region_x: i32,
    region_y: i32,
    seed: i64,
    opts: TerrainOptions,
) -> Vec<Point>;

pub fn trace_river(
    sx: i32,
    sy: i32,
    seed: i64,
    opts: TerrainOptions,
) -> Option<Vec<RiverPathTile>>;

pub fn classify_river_segment(tile: &RiverPathTile) -> RiverRole;
pub fn classify_biome(height: i32, max_height: i32, moisture: f64) -> BiomeRole;
pub fn generate_terrain_chunk(
    chunk_x: i32,
    chunk_y: i32,
    seed: i64,
    opts: TerrainOptions,
) -> Vec<TerrainTile>;

pub(crate) fn hash_seed(values: &[HashValue]) -> i64;
pub(crate) fn lattice_value(seed: i64, ix: i32, iy: i32) -> f64;
pub(crate) fn fade(t: f64) -> f64;
pub(crate) fn value_noise(x: f64, y: f64, seed: i64, scale: f64) -> f64;
pub(crate) fn fractal_noise(
    x: f64,
    y: f64,
    seed: i64,
    octaves: i32,
    persistence: f64,
    scale: f64,
) -> f64;
```

Use signed coordinates. Keep `seed` as a decimal integer value for `hash_seed`
stringification; do not reinterpret public seeds as signed `i32` before hashing.
Only `terrain_moisture_at` applies JS bitwise XOR semantics to the seed.
`HashValue` above means a small local enum for the TS hash inputs, preserving
`String(number)` and `String(string)` behavior exactly.

## Public Surface Semantics

### Types

- `Direction` is exactly `N`, `E`, `S`, `W`. `DIRECTIONS` order is always
  `N, E, S, W`; this order controls bitmasks, tie-breaks, scans, and river flow.
- `BiomeRole` values are `lowland`, `grassland`, `forest`, `rocky`, `highland`.
- `CliffBase` values are `edge`, `corner`, `ridge`, `spur`, `pillar`.
- `TerrainRole::Flat` has no fields.
- `TerrainRole::Cliff` carries:
  - `edges`: bitmask of strictly lower orthogonal neighbors, `N=1,E=2,S=4,W=8`.
  - `base`: cliff family.
  - `variant`: TS renderer key such as `edge-N`, `corner-NE`, `ridge-NS`,
    `spur-W`, or `pillar`.
  - `facing`: primary downhill facing; `None` only for pillar.
  - `max_drop`: deepest drop to any lower orthogonal neighbor.
- `DecorationRole` is either a tree with `species` `0..3`, or a rock with
  `size` and `resource`.
- `RiverRole` carries segment kind plus upstream/downstream directions. `facing`
  is outflow for start/straight/bend, inflow for end, and `N` for the degenerate
  no-flow segment.
- `TerrainTile` stores world coordinates, not local chunk coordinates. Optional
  `river`, `stairs`, and `decoration` fields are absent in TS JSON when undefined.

### TerrainOptions

All options are optional in TS and resolved per call. Rust should use `Default`
or a resolver that preserves these exact defaults:

| field | default |
| --- | --- |
| `max_height` | `DEFAULT_MAX_HEIGHT` = `3` |
| `height_scale` | `0.08` |
| `octaves` | `4` |
| `persistence` | `0.5` |
| `moisture_scale` | `0.06` |
| `village_anchor` | `{ x: 0, y: 0 }` |
| `plateau_radius` | `4` |
| `plateau_height` | `1` |
| `min_run_for_stair` | `3` |
| `region_size` | `24` |
| `rivers_per_region` | `1` |
| `max_river_length` | `36` |
| `river_source_min_elevation` | `0.6` |
| `carve_rivers` | `false` |
| `decorate` | `true` |

`WORLD_TERRAIN_OPTIONS` sets only:

```text
village_anchor = { x: 6, y: 6 }
plateau_radius = 8
plateau_height = 1
```

Every other field still uses the defaults above.

### Scalar Field Functions

- `terrain_elevation_at`: returns raw continuous elevation in `[0,1)` from
  `fractal_noise(x, y, seed, octaves, persistence, height_scale)`. It is not
  plateau-aware.
- `terrain_moisture_at`: returns raw continuous moisture in `[0,1)` from
  `fractal_noise(x, y, seed ^ 0x9e3779b9, 3, 0.5, moisture_scale)`.
  The XOR is JS signed 32-bit bitwise XOR. For `seed=20260702`, the moisture seed
  is `-1627234585`.
- `terrain_height_at`: resolves options, then uses `height_with`:
  - If `max(abs(x-anchor.x), abs(y-anchor.y)) <= plateau_radius`, return
    `plateau_height` directly.
  - Otherwise compute `e = fractal_noise(...)`, `level = floor(e * (max_height + 1))`,
    and clamp to `[0, max_height]`.
- `terrain_stair_at`: returns whether `derive_stairs(...)` produces a stair role.

The plateau affects only quantized height and any algorithms that call
`height_with`; it does not alter `elevation` or `moisture`.

### Cliff Classification

`classify_cliff(center, neighbors)` is pure and should be directly tested.

1. Start with `edges = 0` and `lower = []`.
2. Iterate `DIRECTIONS` in order `N,E,S,W`.
3. A direction is an edge only when `neighbor_height < center`; equal and higher
   neighbors do not count.
4. Set the direction bit and push the direction into `lower`.
5. If `edges == 0`, return flat.
6. `max_drop = center - min(lower neighbor heights)`.
7. Convert `lower` to a cliff:
   - One lower neighbor: `edge-{facing}`, `base=edge`, `facing=that direction`.
   - Four lower neighbors: `pillar`, `base=pillar`, `facing=None`.
   - Two opposite lower neighbors: `ridge-NS` for `N/S` with `facing=N`, or
     `ridge-EW` for `E/W` with `facing=E`.
   - Two adjacent lower neighbors: scan corner pairs in this fixed order:
     `(N,E)->NE`, `(E,S)->SE`, `(S,W)->SW`, `(W,N)->NW`. `facing` is the first
     direction in the matched pair (`N`, `E`, `S`, or `W` respectively).
   - Three lower neighbors: `spur-{higher}` where `higher` is the first direction
     in `N,E,S,W` that is not in `lower`. TS calls it higher, but it really means
     "not lower" and may be equal.

`terrain_role_at` is private: compute `center = height_with(x,y)` and neighbor
heights in `N,E,S,W` using `DIR_VEC`, then call `classify_cliff`.

### Stair Logic

Stairs are derived from cliff roles, never from river carving.

1. `stair_edge_dir(x,y)` calls `terrain_role_at`.
2. It returns a direction only when the role is a single-edge cliff with a
   non-null facing and the tile in that facing direction is exactly one floor lower.
3. Perpendicular scan axes:
   - Facing `N` or `S`: run axis is west/east, with `neg=W`, `pos=E`.
   - Facing `E` or `W`: run axis is north/south, with `neg=N`, `pos=S`.
4. `MAX_RUN_SCAN = 64`.
5. `derive_stairs` walks backward along `neg` to the first same-facing edge tile,
   then counts forward along `pos`.
6. If run length `< min_run_for_stair`, no stairs.
7. Choose one stair per run at `floor((length - 1) / 2)`. For even runs this picks
   the lower-index midpoint. Example: length 4 chooses index 1.

For `seed=20260702` default options, the run `(5,5)..(8,5)` is `edge-N` and the
only stair is at `(6,5)`.

### River Source and Path Logic

`region_river_sources(region_x, region_y, seed, opts)`:

1. Resolve options.
2. `base_seed = hash_seed(seed, "river", region_x, region_y)`.
3. Region origin is `(region_x * region_size, region_y * region_size)`.
4. For `n = 0..rivers_per_region-1`, sample exactly 8 candidates.
5. Candidate randoms:
   - `r1 = lattice_value(base_seed + n * 131, s * 2, 0)`
   - `r2 = lattice_value(base_seed + n * 131, s * 2 + 1, 1)`
6. Candidate coordinate:
   - `x = origin_x + floor(r1 * region_size)`
   - `y = origin_y + floor(r2 * region_size)`
7. Skip candidates inside the plateau.
8. Evaluate raw elevation with `fractal_noise`, not plateau height.
9. Keep the highest candidate using strict `>`; exact ties keep the earlier
   candidate by sample index.
10. Push source only if `best.e >= river_source_min_elevation`.

`trace_river(sx, sy, seed, opts)`:

1. Start path with `{ x:sx, y:sy, in_dir:None, out_dir:None }`.
2. Current elevation is raw continuous elevation, not quantized height.
3. For up to `max_river_length` steps, scan neighbors in `N,E,S,W`.
4. Skip neighbors inside the plateau.
5. Pick the strictly lowest neighbor below the current elevation. Updates happen
   only on `e < best_elev`, so equal elevations do not move and equal downhill
   candidates keep the earlier direction.
6. If no lower neighbor exists, stop at a mouth.
7. Set the previous tile's `out_dir`, move one tile, and push the next tile with
   `in_dir = opposite(out_dir)`.
8. Return `None` if the path has fewer than 2 tiles; otherwise return the path.

The maximum returned path length is `max_river_length + 1`.

`classify_river_segment(tile)`:

| input dirs | segment | facing |
| --- | --- | --- |
| `in=None`, `out=Some(d)` | `start` | `d` |
| `in=Some(d)`, `out=None` | `end` | `d` |
| both dirs, `out == opposite(in)` | `straight` | `out` |
| both dirs, not opposite | `bend` | `out` |
| both `None` | `start` | `N` |

`collect_river_tiles` is private but parity-critical for chunks:

1. Chunk origin is `(chunk_x * 12, chunk_y * 12)`.
2. `reach = max_river_length`.
3. Region bounds use `Math.floor`, not truncation:
   - `region_min_x = floor((origin_x - reach) / region_size)`
   - `region_max_x = floor((origin_x + TERRAIN_CHUNK_SIZE + reach) / region_size)`
   - same for y.
4. Loop `rx` outer, `ry` inner, both inclusive.
5. For every source and traced path, insert tiles whose world coordinates are
   inside `[origin_x, origin_x+12)` and `[origin_y, origin_y+12)`.
6. The TS `Map#set` overwrites by `"x,y"` key. If multiple paths touch a tile, the
   later path in `rx,ry,source,path` iteration order wins.

### Biomes and Decoration

`classify_biome(height, max_height, moisture)`:

1. `height <= 0` -> `lowland`.
2. Else `height >= max_height` -> `highland`.
3. Else `moisture > 0.6` -> `forest`.
4. Else `moisture < 0.33` -> `rocky`.
5. Else `grassland`.

Thresholds are strict for moisture: exactly `0.6` and exactly `0.33` both fall
through to `grassland` when height is neither lowland nor highland.

Decoration densities:

| biome | tree | rock |
| --- | ---: | ---: |
| `lowland` | `0.05` | `0.02` |
| `grassland` | `0.08` | `0.03` |
| `forest` | `0.45` | `0.05` |
| `rocky` | `0.03` | `0.35` |
| `highland` | `0.02` | `0.15` |

`derive_decoration(x, y, seed, biome)`:

1. `roll = lattice_value(hash_seed(seed, "decor", x, y), 0, 0)`.
2. If `roll < tree_density`, return tree:
   - `species_roll = lattice_value(hash_seed(seed, "species", x, y), 1, 1)`
   - `species = floor(species_roll * 4)`.
3. Else if `roll < tree_density + rock_density`, return rock:
   - `size_roll = lattice_value(hash_seed(seed, "rock", x, y), 2, 2)`
   - `< 0.5` -> `small`; `< 0.85` -> `medium`; otherwise `large`.
   - `resource_roll = lattice_value(hash_seed(seed, "ore", x, y), 3, 3)`
   - `resource = resource_roll < 0.4`.
4. Else no decoration.

In chunk assembly, decoration is considered only when `decorate=true`, terrain is
flat, and there is no river and no stairs.

### Chunk Assembly

`generate_terrain_chunk(chunk_x, chunk_y, seed, opts)`:

1. Resolve options once.
2. `origin_x = chunk_x * TERRAIN_CHUNK_SIZE`, `origin_y = chunk_y * TERRAIN_CHUNK_SIZE`.
3. Collect river tiles before iterating tiles.
4. Iterate row-major: `ly=0..11`, inner `lx=0..11`.
5. World coordinate is `(origin_x + lx, origin_y + ly)`.
6. Compute in this exact order:
   - `elevation = fractal_noise(...)`
   - `moisture = terrain_moisture_at(...)`
   - `height = height_with(...)`
   - `terrain = terrain_role_at(...)`
   - lookup river and classify segment
   - if `river && carve_rivers && !is_in_plateau`, set final `height = 0`
   - `stairs = derive_stairs(...)`
   - `biome = classify_biome(height, max_height, moisture)`
   - optional decoration
   - push tile
7. Important: river carving happens after `terrain` is computed, and stairs also
   use uncarved height logic. With `carve_rivers=true`, final `height` and `biome`
   can disagree with the already-computed `terrain` role. Replicate this.

The returned vector length is exactly `144`.

## Constants and Tuning Numbers

### Exported and Direction Constants

| name | value |
| --- | --- |
| `TERRAIN_CHUNK_SIZE` | `12` |
| `DEFAULT_MAX_HEIGHT` | `3` |
| `DIRECTIONS` | `["N", "E", "S", "W"]` |
| `DIR_BIT.N` | `1` |
| `DIR_BIT.E` | `2` |
| `DIR_BIT.S` | `4` |
| `DIR_BIT.W` | `8` |
| `DIR_VEC.N` | `(0, -1)` |
| `DIR_VEC.E` | `(1, 0)` |
| `DIR_VEC.S` | `(0, 1)` |
| `DIR_VEC.W` | `(-1, 0)` |
| `OPPOSITE.N` | `S` |
| `OPPOSITE.E` | `W` |
| `OPPOSITE.S` | `N` |
| `OPPOSITE.W` | `E` |
| `CORNER_PAIRS` | `(N,E,NE)`, `(E,S,SE)`, `(S,W,SW)`, `(W,N,NW)` |

### Noise Constants

| usage | value |
| --- | --- |
| `hash` initial | `0` |
| hash shift | `5` |
| `fade(t)` | `t * t * (3 - 2 * t)` |
| lattice xor-shift A | `13` |
| lattice multiplier | `1274126177` |
| lattice xor-shift B | `16` |
| lattice divisor | `4294967296` |
| fractal seed stride | `1013` |
| fractal initial amplitude | `1` |
| fractal initial frequency | `scale` |
| fractal frequency multiplier | `2` |
| moisture seed mask | `0x9e3779b9` |
| moisture octaves | `3` |
| moisture persistence | `0.5` |

### Stair and River Constants

| usage | value |
| --- | --- |
| `MAX_RUN_SCAN` | `64` |
| default `minRunForStair` | `3` |
| default `regionSize` | `24` |
| default `riversPerRegion` | `1` |
| default `maxRiverLength` | `36` |
| default `riverSourceMinElevation` | `0.6` |
| river source hash label | `"river"` |
| source seed stride | `131` |
| candidates per source | `8` |
| candidate x lattice args | `(s * 2, 0)` |
| candidate y lattice args | `(s * 2 + 1, 1)` |

### Biome and Decoration Constants

| usage | value |
| --- | --- |
| forest moisture threshold | `> 0.6` |
| rocky moisture threshold | `< 0.33` |
| decoration labels | `"decor"`, `"species"`, `"rock"`, `"ore"` |
| tree species count | `4` |
| rock small threshold | `< 0.5` |
| rock medium threshold | `< 0.85` |
| rock resource threshold | `< 0.4` |

## Determinism Notes

This module does not use the shared seeded LCG and does not use the movement,
life, or raid forked chains. It also does not use raw `Math.random`.

Determinism comes from the copied hash/value-noise pipeline:

- `hash_seed` must emulate JS string conversion and signed 32-bit bitwise overflow:
  `hash = ((hash << 5) - hash + charCode) | 0`.
- The TS `hash = hash & hash` line is a no-op except for preserving signed 32-bit
  coercion. Keep the effect.
- Return `Math.abs(hash)`. JS returns `2147483648` for `Math.abs(-2147483648)`;
  Rust must not panic or wrap on that edge case.
- `lattice_value` casts the hash to unsigned with `>>> 0`, applies unsigned
  xor-shifts, uses `Math.imul` wrapping multiplication, then divides by `2^32`.
- Use `f64` for all scalar noise math. Do not use `f32`.
- Use `floor`, not truncation, for negative coordinates in `value_noise` and
  `collect_river_tiles` region bounds.
- Preserve all loop order: direction scans `N,E,S,W`, chunk loops y-major then x,
  region collection loops `rx` outer then `ry`, and source candidates by sample index.

Seed caveat: public TS seeds are `number`s and get stringified directly in most
hash calls. Only the moisture field first applies JS signed 32-bit XOR. If Rust
stores seeds as `u32`, do not stringify the signed reinterpretation for elevation,
rivers, or decoration; stringify the same decimal seed value TS received.

## Float Comparison Policy

For fixtures, compare discrete fields exactly: coordinates, height, biome, terrain
role fields, river role fields, stairs, decoration, tile order, and vector length.

For scalar fields (`elevation`, `moisture`, value-noise outputs), use absolute
tolerance `<= 1e-12` when comparing Rust `f64` to TS fixture JSON. The implementation
should usually match more closely, but `1e-12` allows harmless formatting/parser
rounding while still catching `f32`, wrong floor semantics, wrong seed XOR, or wrong
octave order. Do not round before feeding scalar values into height, biome, source
selection, or river descent; calculations must use the raw `f64`.

## Golden Fixtures to Generate

Use seed `20260702` unless specified. Fixture JSON should include TS outputs with
optional fields omitted or normalized consistently before comparison.

### Noise Helper Vectors

These are private in TS but should be tested in Rust unit tests for P2.6.

```text
hash_seed(20260702, "river", 0, 0) = 514144159
hash_seed(20260702, "decor", 0, 0) = 186059398
hash_seed(20260702, -1, -1) = 1798775021
hash_seed(-1627234585, 0, 0) = 1897127146

lattice_value(20260702, 0, 0) = 0.3119086402002722
lattice_value(20260702, -1, -1) = 0.040301234694197774
lattice_value(186059398, 0, 0) = 0.5957272408995777
lattice_value(514144159, 2, 0) = 0.7725991406477988
lattice_value(-1627234585, 0, 0) = 0.4720527632161975

fade(0) = 0
fade(0.25) = 0.15625
fade(0.5) = 0.5
fade(0.75) = 0.84375
fade(1) = 1

value_noise(0, 0, 20260702, 0.08) = 0.3119086402002722
value_noise(1, 0, 20260702, 0.08) = 0.30834032960923013
value_noise(-1, -1, 20260702, 0.08) = 0.32938684795154793
value_noise(12, 5, 20260702, 0.08) = 0.3630510031297223

fractal_noise(0, 0, 20260702, 4, 0.5, 0.08) = 0.3951633867031584
fractal_noise(12, 5, 20260702, 4, 0.5, 0.08) = 0.35205263867915654
fractal_noise(-12, -12, 20260702, 4, 0.5, 0.08) = 0.36753455002765656
```

### Scalar Field Vectors

Default options:

| x | y | elevation | moisture | height |
| ---: | ---: | ---: | ---: | ---: |
| `0` | `0` | `0.3951633867031584` | `0.6520993657011006` | `1` |
| `5` | `5` | `0.544110925820748` | `0.4107870203075984` | `2` |
| `11` | `5` | `0.42666592318098223` | `0.4089589248985352` | `1` |
| `12` | `5` | `0.35205263867915654` | `0.4334020010186518` | `1` |
| `-1` | `-1` | `0.4172623210268135` | `0.6274046918223453` | `1` |
| `-12` | `-12` | `0.36753455002765656` | `0.42419033497208936` | `1` |

Plateau probes:

| opts | x | y | expected height | note |
| --- | ---: | ---: | ---: | --- |
| default | `4` | `4` | `1` | inside Chebyshev radius 4 |
| default | `5` | `0` | `1` | outside plateau, happens to quantize to 1 |
| `{ anchor:{5,5}, radius:3, height:2 }` | `8` | `8` | `2` | boundary included |
| `{ anchor:{5,5}, radius:3, height:2 }` | `10` | `5` | `1` | outside plateau |
| `WORLD_TERRAIN_OPTIONS` | `14` | `14` | `1` | boundary included |
| `WORLD_TERRAIN_OPTIONS` | `15` | `15` | `2` | outside plateau |

### Cliff Classification Vectors

Use `center=2`; for each mask, lower neighbors have height `1` and all other
neighbors have height `2`.

| mask | expected role |
| ---: | --- |
| `0` | `flat` |
| `1` | `edge-N`, `base=edge`, `facing=N`, `edges=1`, `maxDrop=1` |
| `2` | `edge-E`, `base=edge`, `facing=E`, `edges=2`, `maxDrop=1` |
| `3` | `corner-NE`, `base=corner`, `facing=N`, `edges=3`, `maxDrop=1` |
| `4` | `edge-S`, `base=edge`, `facing=S`, `edges=4`, `maxDrop=1` |
| `5` | `ridge-NS`, `base=ridge`, `facing=N`, `edges=5`, `maxDrop=1` |
| `6` | `corner-SE`, `base=corner`, `facing=E`, `edges=6`, `maxDrop=1` |
| `7` | `spur-W`, `base=spur`, `facing=W`, `edges=7`, `maxDrop=1` |
| `8` | `edge-W`, `base=edge`, `facing=W`, `edges=8`, `maxDrop=1` |
| `9` | `corner-NW`, `base=corner`, `facing=W`, `edges=9`, `maxDrop=1` |
| `10` | `ridge-EW`, `base=ridge`, `facing=E`, `edges=10`, `maxDrop=1` |
| `11` | `spur-S`, `base=spur`, `facing=S`, `edges=11`, `maxDrop=1` |
| `12` | `corner-SW`, `base=corner`, `facing=S`, `edges=12`, `maxDrop=1` |
| `13` | `spur-E`, `base=spur`, `facing=E`, `edges=13`, `maxDrop=1` |
| `14` | `spur-N`, `base=spur`, `facing=N`, `edges=14`, `maxDrop=1` |
| `15` | `pillar`, `base=pillar`, `facing=None`, `edges=15`, `maxDrop=1` |

Additional max-drop vectors:

```text
classify_cliff(3, {N:0,E:2,S:3,W:3})
  -> corner-NE, edges=3, facing=N, maxDrop=3
classify_cliff(3, {N:0,E:0,S:0,W:0})
  -> pillar, edges=15, facing=None, maxDrop=3
classify_cliff(2, {N:3,E:2,S:2,W:2})
  -> flat
```

### Stair Vectors

For default seed/options, generate chunk `(0,0)` and inspect row `y=5`:

| x | y | terrain | stairs | `terrain_stair_at` |
| ---: | ---: | --- | --- | --- |
| `4` | `5` | `corner-NW` | none | `false` |
| `5` | `5` | `edge-N` | none | `false` |
| `6` | `5` | `edge-N` | `{ facing:N }` | `true` |
| `7` | `5` | `edge-N` | none | `false` |
| `8` | `5` | `edge-N` | none | `false` |
| `9` | `5` | `corner-NE` | none | `false` |

The four-tile single-edge run is `x=5..8`; length 4 chooses index 1, so `(6,5)`.
Each `edge-N` tile has `height=2` and its northern neighbor has `height=1`.

### River Vectors

Region source vectors with default options:

| region | expected sources |
| --- | --- |
| `(-3,-3)` | `[{x:-63,y:-51}]` |
| `(-2,-3)` | `[{x:-28,y:-64}]` |
| `(-1,-1)` | `[{x:-3,y:-15}]` |
| `(0,0)` | `[{x:11,y:23}]` |
| `(1,0)` | `[{x:33,y:3}]` |
| `(2,-3)` | `[{x:71,y:-61}]` |
| `(3,3)` | `[{x:85,y:80}]` |

`trace_river(-63, -51, seed, { maxRiverLength: 6 })` should return:

```json
[
  {"x":-63,"y":-51,"inDir":null,"outDir":"N"},
  {"x":-63,"y":-52,"inDir":"S","outDir":"W"},
  {"x":-64,"y":-52,"inDir":"E","outDir":"N"},
  {"x":-64,"y":-53,"inDir":"S","outDir":"N"},
  {"x":-64,"y":-54,"inDir":"S","outDir":"N"},
  {"x":-64,"y":-55,"inDir":"S","outDir":"W"},
  {"x":-65,"y":-55,"inDir":"E","outDir":null}
]
```

With default `maxRiverLength=36`, the first traceable region in scan order
`rx=-3..3`, `ry=-3..3` is `(-3,-3)`, source `(-63,-51)`, and path length is `14`.
The last five tiles are:

```json
[
  {"x":-67,"y":-56,"inDir":"E","outDir":"W"},
  {"x":-68,"y":-56,"inDir":"E","outDir":"W"},
  {"x":-69,"y":-56,"inDir":"E","outDir":"W"},
  {"x":-70,"y":-56,"inDir":"E","outDir":"N"},
  {"x":-70,"y":-57,"inDir":"S","outDir":null}
]
```

Direct segment classification:

```text
{in:null,out:E} -> start, facing=E
{in:W,out:null} -> end, facing=W
{in:N,out:S} -> straight, facing=S
{in:E,out:W} -> straight, facing=W
{in:N,out:E} -> bend, facing=E
{in:null,out:null} -> start, facing=N
```

### Biome and Decoration Vectors

Biome threshold vectors:

```text
classify_biome(0, 3, 0.5) = lowland
classify_biome(3, 3, 0.5) = highland
classify_biome(1, 3, 0.8) = forest
classify_biome(2, 3, 0.2) = rocky
classify_biome(1, 3, 0.45) = grassland
classify_biome(1, 3, 0.6) = grassland
classify_biome(1, 3, 0.33) = grassland
classify_biome(0, 0, 0.9) = lowland
```

Decoration fixture coordinates from generated chunks with default options:

| expected decoration | coordinate |
| --- | --- |
| `tree species=0` | `(-87,-95)` |
| `tree species=1` | `(-91,-92)` |
| `tree species=2` | `(-91,-94)` |
| `tree species=3` | `(-96,-95)` |
| `rock small resource=false` | `(-93,-65)` |
| `rock small resource=true` | `(-88,-51)` |
| `rock medium resource=false` | `(-91,-91)` |
| `rock medium resource=true` | `(-93,-94)` |
| `rock large resource=false` | `(-94,-81)` |
| `rock large resource=true` | `(-89,-75)` |

### Chunk Fixture Matrix

For P2.10, generate and commit full 144-tile TS fixture arrays for these cases.
The summaries below are sanity checks; tests should compare the full ordered tile
arrays, not only counts.

| case | expected summary |
| --- | --- |
| `generate_terrain_chunk(0,0,20260702,{})` | biome `{forest:10, grassland:122, lowland:4, rocky:8}`; terrain `{flat:116, edge-E:5, edge-N:8, corner-NW:5, corner-NE:2, edge-W:3, edge-S:3, corner-SW:1, corner-SE:1}`; `rivers=0`, `stairs=1`, `decorations=21` |
| `generate_terrain_chunk(1,0,20260702,{})` | biome `{lowland:9, grassland:90, rocky:2, forest:43}`; terrain `{flat:124, edge-W:6, corner-SW:2, corner-NE:1, edge-N:8, corner-NW:3}`; `rivers=0`, `stairs=2`, `decorations=34` |
| `generate_terrain_chunk(-1,0,20260702,{})` | biome `{rocky:16, grassland:126, forest:2}`; terrain `{flat:114, edge-E:8, edge-N:5, corner-NE:4, edge-S:8, corner-SW:2, spur-N:1, edge-W:1, corner-NW:1}`; `rivers=0`, `stairs=3`, `decorations=25` |
| `generate_terrain_chunk(2,-3,20260702,{})` | biome `{forest:31, grassland:91, rocky:22}`; terrain `{flat:122, edge-N:11, corner-NW:2, corner-NE:1, edge-S:2, corner-SE:1, edge-E:5}`; `rivers=0`, `stairs=3`, `decorations=28` |
| `generate_terrain_chunk(0,0,20260702,WORLD_TERRAIN_OPTIONS)` | biome `{forest:10, grassland:125, rocky:9}`; terrain `{flat:144}`; `rivers=0`, `stairs=0`, `decorations=25` |
| `generate_terrain_chunk(0,0,20260702,{decorate:false})` | same biome and terrain counts as default `(0,0)`; `decorations=0` |
| `generate_terrain_chunk(-3,-3,20260702,{})` | biome `{forest:26, grassland:114, rocky:4}`; terrain `{flat:127, spur-S:2, spur-E:1, edge-N:8, corner-NE:1, edge-W:2, edge-E:2, corner-NW:1}`; `rivers=8`, `stairs=1`, `decorations=20` |
| `generate_terrain_chunk(-3,-3,20260702,{carveRivers:true})` | terrain/rivers/stairs/decoration counts same as uncarved `(-3,-3)`; biome `{forest:24, grassland:108, rocky:4, lowland:8}` |

Border and sample tile checks:

```text
chunk (0,0), tile (11,5):
  elevation=0.42666592318098223
  moisture=0.4089589248985352
  height=1
  biome=grassland
  terrain=flat

chunk (1,0), tile (12,5):
  elevation=0.35205263867915654
  moisture=0.4334020010186518
  height=1
  biome=grassland
  terrain=flat

chunk (-3,-3), first river tile in row-major order:
  (-36,-32), height=2, biome=forest,
  river={segment:bend,inDir:S,outDir:W,facing:W}

same chunk with carveRivers=true:
  (-36,-32), height=0, biome=lowland,
  terrain remains flat,
  river={segment:bend,inDir:S,outDir:W,facing:W}
```

Include negative chunks in fixture coverage because both `Math.floor` for region
bounds and value-noise lattice coordinates differ from Rust truncating division.

## Dependencies

Required before implementing this module:

- Rust enum/string serialization conventions from `cat-protocol`, if these terrain
  roles become wire-visible.
- No dependency on `rng.rs`: terrain generation uses its own hash/lattice pipeline.
- No dependency on `world_gen.rs`: `terrain_gen.rs` is lower-level and should be
  usable by `world_gen.rs` / `terrain_world` port later.
- `biomes.rs` gameplay tables are not required for this abstract terrain layer.

Implementation slices can proceed in this order:

1. P2.6: constants, options, noise helpers, elevation/moisture/height.
2. P2.7: cliff and stair role logic.
3. P2.8: river source/path/segment logic and chunk river collection.
4. P2.9: biome and decoration classification.
5. P2.10: `generate_terrain_chunk` assembly and full chunk fixtures.

## TS Behaviors to Replicate Intentionally

- `terrain_elevation_at` and `terrain_moisture_at` ignore the plateau.
- Plateau height is returned directly and is not clamped to `0..max_height`.
- `trace_river` does not reject a source inside the plateau; only neighbor steps
  are forbidden from entering the plateau. `region_river_sources` avoids plateau
  candidates, so normal chunk rivers do not start there.
- `carve_rivers` changes final `height` and derived `biome` after `terrain` is
  already classified.
- Multiple rivers in one chunk use last-writer-wins by map insertion order.
- Options are not range-validated in TS. Rust may use stricter integer types for
  practical API safety, but default and fixture options above must be exact.
