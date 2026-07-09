# P2.5 World Generation Port Spec

Sources read:
- `lib/game/worldGen.ts`
- `lib/game/terrainWorld.ts`
- `tests/unit/game/worldGen.test.ts`
- `tests/unit/game/terrainWorld.test.ts`

Target Rust module: `crates/cat-sim/src/world_gen.rs`.

## Purpose

Port the gameplay world-tile generation surface from legacy Voronoi chunks and
the newer terrain-backed bridge. `world_gen.rs` owns chunk/tile coordinate
helpers, colony anchor constants, legacy `WorldTile` generation, starter-water
guarantees, and terrain-to-gameplay `WorldTileData` mapping.

`terrain_gen.rs` remains the pure terrain role/height/river generator. It must
not import `world_gen.rs`.

## Public Surface

### Constants

Rust names:

```rust
pub const CHUNK_SIZE: i32 = 12;
pub const COLONY_SAFE_RADIUS: f64 = 3.5;
pub const COLONY_WATER_RADIUS: f64 = 5.5;
```

`CHUNK_SIZE` is private in TS but should be public or at least `pub(crate)` in
Rust because P2.11 tests and later modules need the shared chunk size.

### Coordinate Types

TS:

```ts
export interface ChunkCoords { chunkX: number; chunkY: number }
export interface TileCoords { x: number; y: number }
```

Rust:

```rust
pub struct ChunkCoords { pub chunk_x: i32, pub chunk_y: i32 }
pub struct TileCoords { pub x: i32, pub y: i32 }
```

Use signed integer coordinates. The TS functions are called with integer tile
and chunk coordinates.

### Coordinate Functions

TS:

```ts
export function tileToChunk(x: number, y: number): ChunkCoords
export function chunkToTile(chunkX: number, chunkY: number): TileCoords
export function getColonyPosition(): TileCoords
```

Rust:

```rust
pub fn tile_to_chunk(x: i32, y: i32) -> ChunkCoords
pub fn chunk_to_tile(chunk_x: i32, chunk_y: i32) -> TileCoords
pub fn get_colony_position() -> TileCoords
```

Semantics:
- `tile_to_chunk` returns `Math.floor(coord / 12)` for each axis. In Rust, do
  not use truncating `/` for negative coordinates. Use `div_euclid(12)` or an
  equivalent floor division.
- `chunk_to_tile` returns the top-left tile origin: `(chunk_x * 12,
  chunk_y * 12)`.
- `get_colony_position` always returns `{ x: 6, y: 6 }`, the center of chunk
  `(0, 0)`.

### World Tile Data

TS:

```ts
export type WorldTileData = Omit<WorldTile, "_id" | "colonyId">;
```

Rust should define this in `world_gen.rs`, reusing existing crate enums where
possible:

```rust
pub struct TileResources {
    pub food: u32,
    pub herbs: u32,
    pub water: u32,
}

pub struct WorldTileData {
    pub x: i32,
    pub y: i32,
    pub tile_type: TileType, // serde rename "type" if serialized
    pub resources: TileResources,
    pub max_resources: MaxResources,
    pub danger_level: f64,
    pub path_wear: u32,
    pub last_depleted: i64,
    pub overlay_feature: Option<OverlayFeature>,
}
```

Use `crate::types::TileType` and `crate::biomes::{BiomeType, OverlayFeature,
MaxResources}` if those exist. TS `overlayFeature` can be absent/null; Rust
should model it as `Option<OverlayFeature>`.

### Chunk Generation Functions

TS:

```ts
export function generateChunk(
  chunkX: number,
  chunkY: number,
  seed: number,
  colonyX: number,
  colonyY: number,
): Omit<WorldTile, "_id" | "colonyId">[]

export function generateWorldChunk(
  chunkX: number,
  chunkY: number,
  seed: number,
  colonyX: number,
  colonyY: number,
): WorldTileData[]
```

Rust:

```rust
pub fn generate_chunk(
    chunk_x: i32,
    chunk_y: i32,
    seed: i64,
    colony_x: i32,
    colony_y: i32,
) -> Vec<WorldTileData>

pub fn generate_world_chunk(
    chunk_x: i32,
    chunk_y: i32,
    seed: i64,
    colony_x: i32,
    colony_y: i32,
) -> Vec<WorldTileData>
```

`generate_chunk` is the legacy Voronoi world generator from `worldGen.ts`.
`generate_world_chunk` is the terrain-backed bridge from `terrainWorld.ts`.

## Constants and Tuning Numbers

Shared:
- `CHUNK_SIZE = 12`
- `COLONY_SAFE_RADIUS = 3.5`
- `COLONY_WATER_RADIUS = 5.5`
- colony anchor `{ x: 6, y: 6 }`
- infinite water sentinel `999`
- `lastDepleted = 0` for generated tiles
- non-river generated `pathWear = 0`, except legacy path overlays

Legacy `worldGen.ts`:
- Voronoi density default: `0.1`
- `expand = CHUNK_SIZE * 2 = 24`
- generated chunk Voronoi bounds are expanded to a `60 x 60` tile box, so
  `numCells = floor(60 * 60 * 0.1) = 360` for normal `generateChunk` calls
- legacy biome roll order:
  1. `oak_forest`
  2. `pine_forest`
  3. `jungle`
  4. `dead_forest`
  5. `mountains`
  6. `swamp`
  7. `desert`
  8. `tundra`
  9. `meadow`
- biome-boundary threshold default: `0.3`
- river boundary noise label/scale/threshold: `hashSeed(seed, "rivers")`,
  scale `0.05`, river if `noise > 0.4`
- through-biome river noise label/scale/threshold:
  `hashSeed(seed, "rivers_through")`, scale `0.02`, river if `noise > 0.85`
- isolated river placement chance: `rng.next() < 0.3`, where
  `rng = createSeededRandom(hashSeed(seed, x, y, "overlay"))`
- path noise scale: `0.03`
- path thresholds:
  - `ancient_road`: `noise > 0.92`
  - `game_trail`: `noise > 0.75`
  - `trade_route`: `noise > 0.88`
- overlay priority:
  1. river
  2. ancient road
  3. trade route
  4. game trail
  5. none
- orthogonal river neighbor check order:
  1. `{ x: x - 1, y }`
  2. `{ x: x + 1, y }`
  3. `{ x, y: y - 1 }`
  4. `{ x, y: y + 1 }`
- resource RNG label: `hashSeed(seed, x, y)`
- starter pond RNG label: `hashSeed(seed, "starter_pond")`

Terrain bridge `terrainWorld.ts`:
- `generateWorldChunk` hard-codes chunk size as `12` for colony containment
  checks instead of importing `CHUNK_SIZE`.
- It calls `generateTerrainChunk(chunkX, chunkY, seed, WORLD_TERRAIN_OPTIONS)`.
- `WORLD_TERRAIN_OPTIONS` in `terrainGen.ts`:
  - `villageAnchor = { x: 6, y: 6 }`
  - `plateauRadius = 8`
  - `plateauHeight = 1`
  - all other terrain options use `terrainGen.ts` defaults
- terrain river tiles map to danger `5`, max resources `{ food: 0, herbs: 0 }`,
  resources `{ food: 0, herbs: 0, water: 999 }`.

Biome and overlay properties are owned by `biomes.rs`; do not duplicate their
tables in `world_gen.rs`.

## Determinism

Both world generators are pure and deterministic. They do not call raw
`Math.random`, and they do not use the worker-tick forked RNG chains
`movement + 1_000_003`, `life + 2_000_003`, or `raids + 3_000_003`.

They use `lib/game/noise.ts`:
- `hashSeed(...values)` converts each value with JS `String(value)`, with no
  delimiter between values, then applies:
  `hash = ((hash << 5) - hash + charCodeAt(i)) | 0`
  and returns `Math.abs(hash)`.
- This is JS signed 32-bit wrapping behavior. Preserve the `Math.abs(i32::MIN)
  == 2147483648` possibility.
- `createSeededRandom(seed).next()` uses:
  `seed = (seed * 1664525 + 1013904223) % 2**32`
  and returns `(seed >>> 0) / 2**32`.
- `rng.int(min, max)` is inclusive:
  `floor(next() * (max - min + 1)) + min`.
- `noise2D(x, y, seed, scale)` uses `floor(x * scale)` and `floor(y * scale)`,
  hashes the four lattice corners, samples one RNG value at each corner, then
  bilinearly interpolates.

Use `f64` for generated distances, noise, and danger. Tests should compare
floating values with a tight tolerance such as `1e-12` unless the local test
helpers already do exact JSON float comparison reliably.

## Legacy Voronoi Generation

### `generate_voronoi_cells`

Suggested Rust helper:

```rust
fn generate_voronoi_cells(
    seed: i64,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
    density: f64,
) -> Vec<VoronoiCell>
```

Semantics:
- `rng = createSeededRandom(hashSeed(seed, "voronoi"))`
- `width = maxX - minX`, `height = maxY - minY`
- `numCells = floor(width * height * density)`
- For each cell `i` in `0..numCells`:
  - `x = minX + rng.next() * width`
  - `y = minY + rng.next() * height`
  - `biomeIndex = rng.int(0, biomeTypes.length - 1)`
  - biome is selected from the 9-item legacy biome list above

`VoronoiCell.x` and `.y` are floats. The biome list does not include
`cave_entrance` or `enemy_lair`; the later `enemy_lair` tile-type branch is
dead for this generator but should remain if the helper accepts arbitrary
biomes in tests.

### `find_nearest_cell`

Suggested Rust helper:

```rust
fn find_nearest_cell(x: i32, y: i32, cells: &[VoronoiCell]) -> &VoronoiCell
```

Semantics:
- Starts from `cells[0]`, `minDist = Infinity`.
- Iterates in cell order and updates only on `dist < minDist`, not `<=`.
- Distance is squared Euclidean distance.
- Earlier cell wins ties.

### `is_biome_boundary`

Suggested Rust helper:

```rust
fn is_biome_boundary(x: i32, y: i32, cells: &[VoronoiCell], threshold: f64) -> bool
```

Important TS bug to replicate for legacy parity:
- The TS code intends to find the second-nearest cell, but the reducer compares
  every candidate to the nearest distance, not to the current `closest`.
- Since no candidate is closer than the nearest cell, the reducer effectively
  returns the initial value `cells[0]`.
- If the nearest cell is `cells[0]`, then `dist1 == dist2` and every such tile
  is treated as a biome boundary.
- If the nearest cell is not `cells[0]`, the boundary check compares the nearest
  distance against the distance to `cells[0]`, not the real second-nearest
  distance.

Replicate this behavior for `generate_chunk`. Do not silently fix it, or seeds
like `42` will produce a very different river field.

Return:

```text
abs(sqrt(distance_to_nearest) - sqrt(distance_to_buggy_second)) < threshold
```

### River and Path Overlay

Suggested helpers:

```rust
fn should_have_river(x: i32, y: i32, seed: i64, cells: &[VoronoiCell]) -> bool
fn should_have_path(x: i32, y: i32, seed: i64, path_type: PathType) -> bool
fn get_overlay_feature(
    x: i32,
    y: i32,
    seed: i64,
    cells: &[VoronoiCell],
) -> Option<OverlayFeature>
```

`should_have_river`:
- If `is_biome_boundary(...)` is true, sample boundary noise and return
  `noise > 0.4`.
- Otherwise sample through-biome noise and return `noise > 0.85`.

`get_overlay_feature`:
- Create `rng = createSeededRandom(hashSeed(seed, x, y, "overlay"))`.
- If `should_have_river(x, y, ...)`:
  - Check the four orthogonal neighbors in the order listed in constants.
  - If any neighbor also `should_have_river`, return `River`.
  - Otherwise return `River` only if `rng.next() < 0.3`.
- If no river is returned, test path overlays in priority order:
  `ancient_road`, then `trade_route`, then `game_trail`.
- Return `None` otherwise.

## Legacy Tile Generation

Suggested helper:

```rust
fn generate_tile(
    x: i32,
    y: i32,
    seed: i64,
    cells: &[VoronoiCell],
    colony_x: i32,
    colony_y: i32,
) -> WorldTileData
```

Algorithm:
1. `nearestCell = findNearestCell(x, y, cells)`.
2. `biome = nearestCell.biome`.
3. `distanceFromColony = sqrt((x - colonyX)^2 + (y - colonyY)^2)`.
4. `rawOverlay = getOverlayFeature(...)`.
5. If `rawOverlay == River` and `distanceFromColony <= COLONY_SAFE_RADIUS`,
   suppress it to `None`; otherwise keep `rawOverlay`.
6. Load biome properties.
7. `rng = createSeededRandom(hashSeed(seed, x, y))`.
8. Roll resources:
   - `food = rng.int(biome.food.min, biome.food.max)`
   - `herbs = rng.int(biome.herbs.min, biome.herbs.max)`
   - `water = biome.baseResources.water`
9. If overlay is `River`, override resources to `{ food: 0, herbs: 0, water:
   999 }`.
10. `dangerLevel = calculateDangerLevel(biome, overlay, distanceFromColony)`.
11. `pathWear = 0`, except non-river overlays use
    `overlayFeatureProperties(overlay).initial_path_wear`.
12. Backward-compatible `TileType` mapping:
    - river overlay -> `TileType::River`
    - `enemy_lair` -> `TileType::EnemyTerritory`
    - `jungle` or `dead_forest` -> `TileType::DenseWoods`
    - biome string contains `"forest"` -> `TileType::Forest`
    - all others, including mountains, swamp, desert, tundra, meadow ->
      `TileType::Field`
13. Return `WorldTileData` with `last_depleted = 0`.

Safe-radius edge case:
- Only river overlays are suppressed inside `COLONY_SAFE_RADIUS`.
- Path overlays are still allowed inside the safe ring. For example, legacy
  seed `7` at `(6, 6)` has `overlayFeature = "game_trail"` and `pathWear = 45`.

### Legacy `generate_chunk`

Algorithm:
1. `minX = chunkX * 12`, `minY = chunkY * 12`
2. `maxX = minX + 12`, `maxY = minY + 12`
3. `expand = 24`
4. Voronoi bounds:
   - `voronoiMinX = minX - 24`
   - `voronoiMinY = minY - 24`
   - `voronoiMaxX = maxX + 24`
   - `voronoiMaxY = maxY + 24`
5. Generate cells for this expanded region.
6. Generate tiles in row-major order:
   - outer loop `y = minY .. maxY - 1`
   - inner loop `x = minX .. maxX - 1`
7. If the colony is inside this chunk using min-inclusive, max-exclusive bounds:
   `colonyX >= minX && colonyX < maxX && colonyY >= minY && colonyY < maxY`,
   call legacy `ensure_water_near_colony`.
8. Return exactly 144 tiles.

The legacy generator is not globally Voronoi-continuous across chunks because
each chunk generates its own expanded Voronoi cell set. Replicate this.

## Starter Water Guarantee

Both legacy and terrain-backed chunks use this control flow:
1. Define Euclidean distance from colony.
2. If any tile in the chunk has `resources.water > 0` and
   `distance <= COLONY_WATER_RADIUS`, return without forcing water.
3. Build candidate list from tiles in existing row-major order where
   `distance > COLONY_SAFE_RADIUS && distance <= COLONY_WATER_RADIUS`.
4. If there are no candidates, return.
5. `rng = createSeededRandom(hashSeed(seed, "starter_pond"))`.
6. `pond = candidates[floor(rng.next() * candidates.length)]`.
7. Mutate the selected tile into a river/pond.

For the default colony chunk `(0, 0)` with colony `(6, 6)`, the candidate list
has 60 tiles. Candidate indices are row-major over chunk tiles:

```text
0:(4,1) 1:(5,1) 2:(6,1) 3:(7,1) 4:(8,1)
5:(3,2) 6:(4,2) 7:(5,2) 8:(6,2) 9:(7,2) 10:(8,2) 11:(9,2)
12:(2,3) 13:(3,3) 14:(4,3) 15:(8,3) 16:(9,3) 17:(10,3)
18:(1,4) 19:(2,4) 20:(3,4) 21:(9,4) 22:(10,4) 23:(11,4)
24:(1,5) 25:(2,5) 26:(10,5) 27:(11,5)
28:(1,6) 29:(2,6) 30:(10,6) 31:(11,6)
32:(1,7) 33:(2,7) 34:(10,7) 35:(11,7)
36:(1,8) 37:(2,8) 38:(3,8) 39:(9,8) 40:(10,8) 41:(11,8)
42:(2,9) 43:(3,9) 44:(4,9) 45:(8,9) 46:(9,9) 47:(10,9)
48:(3,10) 49:(4,10) 50:(5,10) 51:(6,10) 52:(7,10) 53:(8,10) 54:(9,10)
55:(4,11) 56:(5,11) 57:(6,11) 58:(7,11) 59:(8,11)
```

Known starter-pond RNG selections for that candidate list:

| seed | hashSeed(seed, "starter_pond") | roll | index | coord |
|---:|---:|---:|---:|---|
| 0 | 199079467 | 0.9666951629333198 | 58 | `(7, 11)` |
| 1 | 492482474 | 0.01418465399183333 | 0 | `(4, 1)` |
| 7 | 2042066780 | 0.17301434534601867 | 10 | `(8, 2)` |
| 42 | 1305130205 | 0.6147337313741446 | 36 | `(1, 8)` |
| 99 | 1591777083 | 0.30803128285333514 | 18 | `(1, 4)` |
| 1234 | 1730397927 | 0.15257400879636407 | 9 | `(7, 2)` |
| 1337 | 1937226581 | 0.09943131729960442 | 5 | `(3, 2)` |
| 99999 | 767105506 | 0.8367814926896244 | 50 | `(5, 10)` |
| 1781313000000 | 1428898885 | 0.46707015484571457 | 28 | `(1, 6)` |

Important legacy-vs-terrain difference:
- Legacy `worldGen.ensureWaterNearColony` mutates only:
  - `type = "river"`
  - `overlayFeature = "river"`
  - `resources = { food: 0, herbs: 0, water: 999 }`
- It does not reset `maxResources`, `dangerLevel`, `pathWear`, or
  `lastDepleted`. This looks like a TS bug, but reproduce it for
  `generate_chunk` parity.
- Terrain `terrainWorld.ensureWaterNearColony` additionally mutates:
  - `maxResources = { food: 0, herbs: 0 }`
  - `dangerLevel = 5`
- Terrain `pathWear` and `lastDepleted` are already `0`.

## Terrain-to-Gameplay Bridge

`terrain_gen.rs` owns:
- `TerrainOptions`
- `WORLD_TERRAIN_OPTIONS`
- `TerrainTile`
- `BiomeRole`
- `RiverRole`
- `generate_terrain_chunk`

`world_gen.rs` owns:
- `WorldTileData`
- terrain biome role to gameplay biome mapping
- terrain biome role to gameplay tile type mapping
- `terrain_to_world_tile`
- terrain-backed `ensure_water_near_colony`
- `generate_world_chunk`

No code in `terrain_gen.rs` should import `world_gen.rs`. If the terrain module
needs the village anchor, keep the TS boundary: `WORLD_TERRAIN_OPTIONS` in
`terrain_gen.rs` duplicates `{ x: 6, y: 6 }` and tests assert it matches
`world_gen::get_colony_position()`.

### Mapping Tables

Gameplay biome each terrain role borrows resource/danger tables from:

| Terrain `BiomeRole` | Gameplay `BiomeType` |
|---|---|
| `lowland` | `meadow` |
| `grassland` | `meadow` |
| `forest` | `oak_forest` |
| `rocky` | `mountains` |
| `highland` | `mountains` |

Stored gameplay tile type:

| Terrain `BiomeRole` | `WorldTile.type` |
|---|---|
| `lowland` | `meadow` |
| `grassland` | `field` |
| `forest` | `forest` |
| `rocky` | `mountains` |
| `highland` | `mountains` |

### `terrain_to_world_tile`

Suggested Rust helper:

```rust
fn terrain_to_world_tile(
    tile: &TerrainTile,
    seed: i64,
    colony_x: i32,
    colony_y: i32,
) -> WorldTileData
```

Algorithm:
1. `dist = sqrt((tile.x - colonyX)^2 + (tile.y - colonyY)^2)`.
2. If `tile.river.is_some()`:
   - return `type = River`
   - `resources = { food: 0, herbs: 0, water: 999 }`
   - `maxResources = { food: 0, herbs: 0 }`
   - `dangerLevel = 5`
   - `pathWear = 0`
   - `lastDepleted = 0`
   - `overlayFeature = Some(River)`
   - Do not roll resources.
3. Otherwise:
   - map terrain `biome` to gameplay `BiomeType`
   - load biome properties
   - `rng = createSeededRandom(hashSeed(seed, tile.x, tile.y))`
   - roll food and herbs with inclusive `rng.int`
   - water is biome base water
   - `maxResources` comes from mapped biome properties
   - `dangerLevel = calculateDangerLevel(mappedBiome, None, dist)`
   - `pathWear = 0`
   - `lastDepleted = 0`
   - `overlayFeature = None`

Terrain bridge does not create ancient roads, trade routes, or game trails.

### `generate_world_chunk`

Algorithm:
1. `terrain = generateTerrainChunk(chunkX, chunkY, seed, WORLD_TERRAIN_OPTIONS)`.
2. Map each terrain tile to `WorldTileData` in the exact array order returned by
   `generateTerrainChunk` (row-major by world coordinates).
3. Compute `minX = chunkX * 12`, `minY = chunkY * 12`.
4. If colony is inside `[minX, minX + 12) x [minY, minY + 12)`, call the
   terrain-backed starter-water guarantee.
5. Return the 144 mapped tiles.

Terrain edge case:
- `terrainGen.ts` avoids terrain rivers inside the plateau, but
  `terrainWorld.ts` can still force a gameplay pond after terrain generation.
  That forced pond mutates the gameplay `WorldTileData`; it does not mutate the
  underlying `TerrainTile.river` field. Replicate this for TS parity.

## Golden Fixtures to Generate

Use the archived TS functions via a pure `npx tsx` script. Do not edit TS
sources. Recommended fixture paths match the board:

- `docs/migration/fixtures/p2/world_coords.json`
- `docs/migration/fixtures/p2/world_overlays.json`
- `docs/migration/fixtures/p2/world_chunks_legacy.json`
- `docs/migration/fixtures/p2/world_chunks_terrain.json`

### Coordinate Fixture

Expected exact cases:

| input | expected |
|---|---|
| `tileToChunk(0, 0)` | `{ chunkX: 0, chunkY: 0 }` |
| `tileToChunk(11, 11)` | `{ chunkX: 0, chunkY: 0 }` |
| `tileToChunk(12, 12)` | `{ chunkX: 1, chunkY: 1 }` |
| `tileToChunk(-1, -1)` | `{ chunkX: -1, chunkY: -1 }` |
| `tileToChunk(-12, -12)` | `{ chunkX: -1, chunkY: -1 }` |
| `tileToChunk(-13, -13)` | `{ chunkX: -2, chunkY: -2 }` |
| `chunkToTile(0, 0)` | `{ x: 0, y: 0 }` |
| `chunkToTile(1, 1)` | `{ x: 12, y: 12 }` |
| `chunkToTile(-1, -1)` | `{ x: -12, y: -12 }` |
| `getColonyPosition()` | `{ x: 6, y: 6 }` |

### Legacy Chunk Fixture Matrix

Generate full 144-tile outputs for:

| seed | chunks |
|---:|---|
| 1 | `(0,0)`, `(1,0)`, `(-1,-1)`, `(0,1)` |
| 42 | `(0,0)`, `(1,0)`, `(-1,-1)`, `(0,1)` |
| 99999 | `(0,0)`, `(1,0)`, `(-1,-1)`, `(0,1)` |

Also include starter-water seeds for chunk `(0, 0)`:
`7`, `1337`, `1781313000000`.

Expected chunk summaries for smoke tests:

| seed | chunk | type counts | water near colony | safe rivers |
|---:|---|---|---:|---:|
| 1 | `(0,0)` | field 56, dense_woods 38, forest 49, river 1 | 1 | 0 |
| 1 | `(1,0)` | field 57, dense_woods 38, forest 49 | 0 | 0 |
| 1 | `(-1,-1)` | field 57, dense_woods 38, forest 49 | 0 | 0 |
| 1 | `(0,1)` | field 57, dense_woods 38, forest 49 | 0 | 0 |
| 42 | `(0,0)` | river 107, field 22, dense_woods 14, forest 1 | 60 | 0 |
| 42 | `(1,0)` | river 144 | 0 | 0 |
| 42 | `(-1,-1)` | field 5, river 139 | 0 | 0 |
| 42 | `(0,1)` | river 144 | 0 | 0 |
| 99999 | `(0,0)` | field 83, river 4, forest 23, dense_woods 34 | 1 | 0 |
| 99999 | `(1,0)` | field 84, river 3, forest 23, dense_woods 34 | 0 | 0 |
| 99999 | `(-1,-1)` | field 84, river 3, forest 23, dense_woods 34 | 0 | 0 |
| 99999 | `(0,1)` | field 84, river 3, forest 23, dense_woods 34 | 0 | 0 |

Selected exact legacy expectations:
- Seed `1`, chunk `(0,0)`, forced pond `(4,1)`:
  - `type = river`
  - `resources = { food: 0, herbs: 0, water: 999 }`
  - `maxResources = { food: 12, herbs: 0 }`
  - `dangerLevel = 55.77032961426901`
  - `pathWear = 0`
  - `overlayFeature = river`
- Seed `1`, chunk `(0,0)`, colony tile `(6,6)`:
  - `type = dense_woods`
  - `resources = { food: 34, herbs: 12, water: 0 }`
  - `maxResources = { food: 80, herbs: 35 }`
  - `dangerLevel = 45`
  - `pathWear = 0`
  - `overlayFeature = null`
- Seed `7`, chunk `(0,0)`, colony tile `(6,6)`:
  - `type = dense_woods`
  - `resources = { food: 6, herbs: 18, water: 0 }`
  - `maxResources = { food: 20, herbs: 45 }`
  - `dangerLevel = 50`
  - `pathWear = 45`
  - `overlayFeature = game_trail`
- Seed `42`, chunk `(0,0)`, tile `(0,0)`:
  - `type = river`
  - `resources = { food: 0, herbs: 0, water: 999 }`
  - `maxResources = { food: 15, herbs: 8 }`
  - `dangerLevel = 5`
  - `overlayFeature = river`

### Terrain Chunk Fixture Matrix

Generate full 144-tile outputs for:

| seed | chunks |
|---:|---|
| 1 | `(0,0)`, `(1,0)`, `(-1,-1)`, `(0,1)` |
| 42 | `(0,0)`, `(1,0)`, `(-1,-1)`, `(0,1)` |
| 1234 | `(0,0)`, `(1,0)`, `(-1,-1)`, `(0,1)` |

Also include starter-water seeds for chunk `(0, 0)`:
`0`, `7`, `99`.

Expected chunk summaries for smoke tests:

| seed | chunk | type counts | water near colony | safe rivers |
|---:|---|---|---:|---:|
| 1 | `(0,0)` | mountains 17, field 126, river 1 | 1 | 0 |
| 1 | `(1,0)` | field 144 | 0 | 0 |
| 1 | `(-1,-1)` | field 123, river 12, mountains 9 | 0 | 0 |
| 1 | `(0,1)` | field 144 | 0 | 0 |
| 42 | `(0,0)` | mountains 61, field 82, river 1 | 1 | 0 |
| 42 | `(1,0)` | field 136, mountains 6, river 2 | 0 | 0 |
| 42 | `(-1,-1)` | field 87, mountains 57 | 0 | 0 |
| 42 | `(0,1)` | field 134, forest 9, meadow 1 | 0 | 0 |
| 1234 | `(0,0)` | forest 94, river 1, field 49 | 1 | 0 |
| 1234 | `(1,0)` | forest 101, mountains 2, field 41 | 0 | 0 |
| 1234 | `(-1,-1)` | forest 7, field 131, mountains 6 | 0 | 0 |
| 1234 | `(0,1)` | forest 61, field 82, mountains 1 | 0 | 0 |

Selected exact terrain expectations:
- Seed `1`, chunk `(0,0)`, forced pond `(4,1)`:
  - `type = river`
  - `resources = { food: 0, herbs: 0, water: 999 }`
  - `maxResources = { food: 0, herbs: 0 }`
  - `dangerLevel = 5`
  - `pathWear = 0`
  - `overlayFeature = river`
- Seed `1`, chunk `(0,0)`, colony tile `(6,6)`:
  - `type = field`
  - `resources = { food: 10, herbs: 0, water: 0 }`
  - `maxResources = { food: 30, herbs: 6 }`
  - `dangerLevel = 10`
  - `overlayFeature = null`
- Seed `42`, chunk `(0,1)`, lowland tile `(0,23)`:
  - underlying `TerrainTile.biome = lowland`
  - `type = meadow`
  - `resources = { food: 16, herbs: 2, water: 0 }`
  - `maxResources = { food: 30, herbs: 6 }`
  - `dangerLevel = 46.05551275463989`
  - `overlayFeature = null`
- Seed `1234`, chunk `(1,0)`, highland tile `(23,0)`:
  - underlying `TerrainTile.biome = highland`
  - `type = mountains`
  - `resources = { food: 3, herbs: 0, water: 0 }`
  - `maxResources = { food: 15, herbs: 8 }`
  - `dangerLevel = 86.05551275463989`

## Dependencies

For P2.11 coordinate helpers:
- no world-generation dependencies beyond `types.rs` if the coordinate structs
  live beside tile data.

For P2.12 legacy overlays:
- `noise.rs` must port `hashSeed`, `createSeededRandom`, `noise2D`
  semantics from `lib/game/noise.ts`.
- `biomes.rs` must expose `BiomeType` and `OverlayFeature`.

For P2.13 legacy chunks:
- P2.12 helpers.
- `biomes.rs` must expose biome properties, overlay properties, and
  `calculate_danger_level`.
- `types.rs` must expose `TileType` variants including `Field`, `Forest`,
  `DenseWoods`, `River`, `EnemyTerritory`, `Meadow`, and `Mountains`.

For P2.14 terrain-backed chunks:
- P2.10 `terrain_gen.rs` chunk assembly must exist.
- `terrain_gen.rs` must expose `TerrainTile`, `BiomeRole`,
  `WORLD_TERRAIN_OPTIONS`, and `generate_terrain_chunk`.
- `world_gen.rs` must not be imported by `terrain_gen.rs`; keep the dependency
  one-way from `world_gen.rs` to `terrain_gen.rs`.

## Edge Cases to Preserve

- Negative tile coordinates use floor division, not truncation.
- `generate_chunk` returns tiles row-major by world coordinate.
- Chunk colony containment is min-inclusive, max-exclusive.
- Legacy safe radius suppresses only river overlays at `distance <= 3.5`; path
  overlays can still appear on the colony tile.
- Rivers at `distance > 3.5` are allowed, including at distances like
  `sqrt(13) = 3.605551...`.
- Starter ponds are chosen only if no water exists within radius `5.5`.
- Legacy forced ponds retain original biome `maxResources` and `dangerLevel`.
- Terrain forced ponds reset `maxResources` to zero and `dangerLevel` to `5`.
- The legacy biome-boundary "second nearest" bug must be reproduced for legacy
  parity.
- Terrain-generated rivers come from `terrain_gen.rs`; forced starter ponds in
  `terrainWorld.ts` are gameplay-only mutations after terrain generation.
