//! Terrain fields and abstract roles ported from `lib/game/terrainGen.ts`.

use std::collections::HashMap;

use ryu_js::Buffer;

use crate::climate::{Biome, classify_climate_biome};

pub const TERRAIN_CHUNK_SIZE: i32 = 12;
pub const DEFAULT_MAX_HEIGHT: i32 = 3;
pub const DIRECTIONS: [Direction; 4] = [Direction::N, Direction::E, Direction::S, Direction::W];

const LATTICE_DIVISOR: f64 = 4_294_967_296.0;
const MOISTURE_SEED_MASK: i32 = 0x9e37_79b9_u32 as i32;
const MAX_RUN_SCAN: i32 = 64;

// --- P17 climate fields ------------------------------------------------------
//
// Temperature / humidity / weirdness are sampled from the same value-noise
// machinery as elevation/moisture, each on its own seed derived by xor-ing a
// distinct mask into `world_seed` (mirroring how moisture derives its seed).
// The scales are deliberately **very low frequency** so biome regions are large
// — roughly `1 / scale` tiles across. The founding plateau spans ~16 tiles
// (`plateau_radius` 8); at scale `0.006` a climate wavelength is ~166 tiles, so
// biome regions are ~10× the village and distant biomes are a mid/late journey.
const TEMPERATURE_SEED_MASK: i32 = 0x85eb_ca6b_u32 as i32;
const HUMIDITY_SEED_MASK: i32 = 0xc2b2_ae35_u32 as i32;
const WEIRDNESS_SEED_MASK: i32 = 0x27d4_eb2f_u32 as i32;
const CLIMATE_TEMPERATURE_SCALE: f64 = 0.006;
const CLIMATE_HUMIDITY_SCALE: f64 = 0.006;
const CLIMATE_WEIRDNESS_SCALE: f64 = 0.004;
const CLIMATE_OCTAVES: i32 = 2;
const CLIMATE_PERSISTENCE: f64 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    N,
    E,
    S,
    W,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BiomeRole {
    Lowland,
    Grassland,
    Forest,
    Rocky,
    Highland,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CliffBase {
    Edge,
    Corner,
    Ridge,
    Spur,
    Pillar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RockSize {
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RiverSegment {
    Start,
    Straight,
    Bend,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerrainRole {
    Flat,
    Cliff(CliffTerrainRole),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliffTerrainRole {
    pub edges: u8,
    pub base: CliffBase,
    pub variant: String,
    pub facing: Option<Direction>,
    pub max_drop: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorationRole {
    Tree { species: i32 },
    Rock { size: RockSize, resource: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiverRole {
    pub segment: RiverSegment,
    pub in_dir: Option<Direction>,
    pub out_dir: Option<Direction>,
    pub facing: Direction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StairsRole {
    pub facing: Direction,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerrainTile {
    pub x: i32,
    pub y: i32,
    pub elevation: f64,
    pub moisture: f64,
    pub height: i32,
    pub biome: BiomeRole,
    /// P17 climate-driven biome (additive; the coarse `biome` role above is
    /// unchanged so movement/placement keep working). Drives per-biome tints and
    /// decoration density on the client.
    pub climate_biome: Biome,
    pub terrain: TerrainRole,
    pub river: Option<RiverRole>,
    pub stairs: Option<StairsRole>,
    pub decoration: Option<DecorationRole>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NeighborHeights {
    pub n: i32,
    pub e: i32,
    pub s: i32,
    pub w: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiverPathTile {
    pub x: i32,
    pub y: i32,
    pub in_dir: Option<Direction>,
    pub out_dir: Option<Direction>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

impl TerrainOptions {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            max_height: None,
            height_scale: None,
            octaves: None,
            persistence: None,
            moisture_scale: None,
            village_anchor: None,
            plateau_radius: None,
            plateau_height: None,
            min_run_for_stair: None,
            region_size: None,
            rivers_per_region: None,
            max_river_length: None,
            river_source_min_elevation: None,
            carve_rivers: None,
            decorate: None,
        }
    }
}

impl Default for TerrainOptions {
    fn default() -> Self {
        Self::empty()
    }
}

pub const WORLD_TERRAIN_OPTIONS: TerrainOptions = TerrainOptions {
    max_height: None,
    height_scale: None,
    octaves: None,
    persistence: None,
    moisture_scale: None,
    village_anchor: Some(Point { x: 6, y: 6 }),
    plateau_radius: Some(8),
    plateau_height: Some(1),
    min_run_for_stair: None,
    region_size: None,
    rivers_per_region: None,
    max_river_length: None,
    river_source_min_elevation: None,
    carve_rivers: None,
    decorate: None,
};

#[derive(Debug, Clone, Copy, PartialEq)]
struct ResolvedOptions {
    max_height: i32,
    height_scale: f64,
    octaves: i32,
    persistence: f64,
    moisture_scale: f64,
    village_anchor: Point,
    plateau_radius: i32,
    plateau_height: i32,
    min_run_for_stair: i32,
    region_size: i32,
    rivers_per_region: i32,
    max_river_length: i32,
    river_source_min_elevation: f64,
    carve_rivers: bool,
    decorate: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub(crate) enum HashValue<'a> {
    Int(i64),
    Number(f64),
    Text(&'a str),
}

pub fn terrain_elevation_at(x: i32, y: i32, seed: i64, opts: TerrainOptions) -> f64 {
    let opts = resolve_options(opts);
    fractal_noise(
        f64::from(x),
        f64::from(y),
        seed,
        opts.octaves,
        opts.persistence,
        opts.height_scale,
    )
}

pub fn terrain_moisture_at(x: i32, y: i32, seed: i64, opts: TerrainOptions) -> f64 {
    let opts = resolve_options(opts);
    fractal_noise(
        f64::from(x),
        f64::from(y),
        i64::from(js_i32_xor(seed, MOISTURE_SEED_MASK)),
        3,
        0.5,
        opts.moisture_scale,
    )
}

pub fn terrain_height_at(x: i32, y: i32, seed: i64, opts: TerrainOptions) -> i32 {
    height_with(x, y, seed, &resolve_options(opts))
}

/// Deterministic climate **temperature** in `[0, 1)` at world `(x, y)` — a very
/// low-frequency value-noise field so climate (and thus biomes) vary over large
/// regions. Same machinery as elevation/moisture, distinct seed.
#[must_use]
pub fn terrain_temperature_at(x: i32, y: i32, seed: i64) -> f64 {
    climate_field(x, y, seed, TEMPERATURE_SEED_MASK, CLIMATE_TEMPERATURE_SCALE)
}

/// Deterministic climate **humidity** in `[0, 1)` at world `(x, y)`.
#[must_use]
pub fn terrain_humidity_at(x: i32, y: i32, seed: i64) -> f64 {
    climate_field(x, y, seed, HUMIDITY_SEED_MASK, CLIMATE_HUMIDITY_SCALE)
}

/// Deterministic climate **weirdness** in `[0, 1)` — the extra field that carves
/// rare, large flower/mushroom regions.
#[must_use]
pub fn terrain_weirdness_at(x: i32, y: i32, seed: i64) -> f64 {
    climate_field(x, y, seed, WEIRDNESS_SEED_MASK, CLIMATE_WEIRDNESS_SCALE)
}

fn climate_field(x: i32, y: i32, seed: i64, mask: i32, scale: f64) -> f64 {
    fractal_noise(
        f64::from(x),
        f64::from(y),
        i64::from(js_i32_xor(seed, mask)),
        CLIMATE_OCTAVES,
        CLIMATE_PERSISTENCE,
        scale,
    )
}

pub fn terrain_stair_at(x: i32, y: i32, seed: i64, opts: TerrainOptions) -> bool {
    derive_stairs(x, y, seed, &resolve_options(opts)).is_some()
}

pub fn classify_cliff(center: i32, neighbors: NeighborHeights) -> TerrainRole {
    let mut edges = 0;
    let mut lower = Vec::new();

    for dir in DIRECTIONS {
        if neighbors.get(dir) < center {
            edges |= dir_bit(dir);
            lower.push(dir);
        }
    }

    if edges == 0 {
        return TerrainRole::Flat;
    }

    let min_lower = lower
        .iter()
        .map(|&dir| neighbors.get(dir))
        .min()
        .expect("non-empty lower cliff directions");
    mask_to_cliff(edges, &lower, center - min_lower)
}

pub fn region_river_sources(
    region_x: i32,
    region_y: i32,
    seed: i64,
    opts: TerrainOptions,
) -> Vec<Point> {
    let opts = resolve_options(opts);
    let base_seed = hash_seed(&[
        HashValue::Int(seed),
        HashValue::Text("river"),
        HashValue::Int(i64::from(region_x)),
        HashValue::Int(i64::from(region_y)),
    ]);
    let origin_x = region_x * opts.region_size;
    let origin_y = region_y * opts.region_size;
    let mut sources = Vec::new();

    for n in 0..opts.rivers_per_region {
        let mut best: Option<(i32, i32, f64)> = None;
        for s in 0..8 {
            let source_seed = i64::from(base_seed) + i64::from(n * 131);
            let r1 = lattice_value(source_seed, s * 2, 0);
            let r2 = lattice_value(source_seed, s * 2 + 1, 1);
            let x = origin_x + (r1 * f64::from(opts.region_size)).floor() as i32;
            let y = origin_y + (r2 * f64::from(opts.region_size)).floor() as i32;

            if is_in_plateau(x, y, &opts) {
                continue;
            }

            let elevation = fractal_noise(
                f64::from(x),
                f64::from(y),
                seed,
                opts.octaves,
                opts.persistence,
                opts.height_scale,
            );
            if best.is_none_or(|(_, _, best_elevation)| elevation > best_elevation) {
                best = Some((x, y, elevation));
            }
        }

        if let Some((x, y, elevation)) = best
            && elevation >= opts.river_source_min_elevation
        {
            sources.push(Point { x, y });
        }
    }

    sources
}

pub fn trace_river(
    sx: i32,
    sy: i32,
    seed: i64,
    opts: TerrainOptions,
) -> Option<Vec<RiverPathTile>> {
    let opts = resolve_options(opts);
    let mut path = vec![RiverPathTile {
        x: sx,
        y: sy,
        in_dir: None,
        out_dir: None,
    }];

    let mut cx = sx;
    let mut cy = sy;
    let mut current_elevation = fractal_noise(
        f64::from(cx),
        f64::from(cy),
        seed,
        opts.octaves,
        opts.persistence,
        opts.height_scale,
    );

    for _ in 0..opts.max_river_length {
        let mut best_dir = None;
        let mut best_elevation = current_elevation;

        for dir in DIRECTIONS {
            let (dx, dy) = dir_vec(dir);
            let nx = cx + dx;
            let ny = cy + dy;

            if is_in_plateau(nx, ny, &opts) {
                continue;
            }

            let elevation = fractal_noise(
                f64::from(nx),
                f64::from(ny),
                seed,
                opts.octaves,
                opts.persistence,
                opts.height_scale,
            );
            if elevation < best_elevation {
                best_elevation = elevation;
                best_dir = Some(dir);
            }
        }

        let Some(out_dir) = best_dir else {
            break;
        };
        let (dx, dy) = dir_vec(out_dir);
        path.last_mut()
            .expect("river path has current tile")
            .out_dir = Some(out_dir);
        cx += dx;
        cy += dy;
        current_elevation = best_elevation;
        path.push(RiverPathTile {
            x: cx,
            y: cy,
            in_dir: Some(opposite(out_dir)),
            out_dir: None,
        });
    }

    (path.len() >= 2).then_some(path)
}

pub fn classify_river_segment(tile: &RiverPathTile) -> RiverRole {
    match (tile.in_dir, tile.out_dir) {
        (None, Some(out_dir)) => RiverRole {
            segment: RiverSegment::Start,
            in_dir: None,
            out_dir: Some(out_dir),
            facing: out_dir,
        },
        (Some(in_dir), None) => RiverRole {
            segment: RiverSegment::End,
            in_dir: Some(in_dir),
            out_dir: None,
            facing: in_dir,
        },
        (Some(in_dir), Some(out_dir)) => RiverRole {
            segment: if out_dir == opposite(in_dir) {
                RiverSegment::Straight
            } else {
                RiverSegment::Bend
            },
            in_dir: Some(in_dir),
            out_dir: Some(out_dir),
            facing: out_dir,
        },
        (None, None) => RiverRole {
            segment: RiverSegment::Start,
            in_dir: None,
            out_dir: None,
            facing: Direction::N,
        },
    }
}

pub fn classify_biome(height: i32, max_height: i32, moisture: f64) -> BiomeRole {
    if height <= 0 {
        return BiomeRole::Lowland;
    }
    if height >= max_height {
        return BiomeRole::Highland;
    }
    if moisture > 0.6 {
        return BiomeRole::Forest;
    }
    if moisture < 0.33 {
        return BiomeRole::Rocky;
    }
    BiomeRole::Grassland
}

pub fn generate_terrain_chunk(
    chunk_x: i32,
    chunk_y: i32,
    seed: i64,
    opts: TerrainOptions,
) -> Vec<TerrainTile> {
    let opts = resolve_options(opts);
    let origin_x = chunk_x * TERRAIN_CHUNK_SIZE;
    let origin_y = chunk_y * TERRAIN_CHUNK_SIZE;
    let rivers = collect_river_tiles(chunk_x, chunk_y, seed, &opts);
    let mut tiles = Vec::with_capacity((TERRAIN_CHUNK_SIZE * TERRAIN_CHUNK_SIZE) as usize);

    for ly in 0..TERRAIN_CHUNK_SIZE {
        for lx in 0..TERRAIN_CHUNK_SIZE {
            let x = origin_x + lx;
            let y = origin_y + ly;
            let elevation = fractal_noise(
                f64::from(x),
                f64::from(y),
                seed,
                opts.octaves,
                opts.persistence,
                opts.height_scale,
            );
            let moisture = fractal_noise(
                f64::from(x),
                f64::from(y),
                i64::from(js_i32_xor(seed, MOISTURE_SEED_MASK)),
                3,
                0.5,
                opts.moisture_scale,
            );
            let mut height = height_with(x, y, seed, &opts);
            let terrain = terrain_role_at(x, y, seed, &opts);
            let river = rivers.get(&(x, y)).map(classify_river_segment);
            if river.is_some() && opts.carve_rivers && !is_in_plateau(x, y, &opts) {
                height = 0;
            }
            let stairs = derive_stairs(x, y, seed, &opts);
            let biome = classify_biome(height, opts.max_height, moisture);
            let climate_biome = classify_climate_biome(
                terrain_temperature_at(x, y, seed),
                terrain_humidity_at(x, y, seed),
                terrain_weirdness_at(x, y, seed),
                elevation,
                is_in_plateau(x, y, &opts),
                river.is_some(),
            );
            let decoration = if opts.decorate
                && terrain == TerrainRole::Flat
                && river.is_none()
                && stairs.is_none()
            {
                derive_decoration(x, y, seed, biome)
            } else {
                None
            };

            tiles.push(TerrainTile {
                x,
                y,
                elevation,
                moisture,
                height,
                biome,
                climate_biome,
                terrain,
                river,
                stairs,
                decoration,
            });
        }
    }

    tiles
}

/// Whether the deterministic terrain generator places a tree decoration on the
/// world tile `(x, y)` for `world_seed`. Mirrors exactly what the client renders
/// from `generate_terrain_chunk(.., WORLD_TERRAIN_OPTIONS)`, so the simulation and
/// the renderer agree on where trees stand. Trees are otherwise client-only, so
/// this is how the sim "sees" them for placement/occupancy (buildings avoid trees).
#[must_use]
pub fn tile_has_tree(world_seed: u32, x: i32, y: i32) -> bool {
    let chunk_x = floor_div(x, TERRAIN_CHUNK_SIZE);
    let chunk_y = floor_div(y, TERRAIN_CHUNK_SIZE);
    generate_terrain_chunk(
        chunk_x,
        chunk_y,
        i64::from(world_seed),
        WORLD_TERRAIN_OPTIONS,
    )
    .into_iter()
    .any(|tile| {
        tile.x == x && tile.y == y && matches!(tile.decoration, Some(DecorationRole::Tree { .. }))
    })
}

/// The deterministic [`BiomeRole`] the terrain generator assigns to world tile
/// `(x, y)` for `world_seed`. Same source of truth as [`tile_has_tree`] (P14.1):
/// it regenerates the owning chunk from `WORLD_TERRAIN_OPTIONS` so the sim and the
/// renderer agree on ground type. Used by movement to derive a per-tile surface
/// speed factor. Falls back to [`BiomeRole::Grassland`] if the tile is somehow
/// absent (never happens for in-range chunk coordinates).
#[must_use]
pub fn tile_biome(world_seed: u32, x: i32, y: i32) -> BiomeRole {
    let chunk_x = floor_div(x, TERRAIN_CHUNK_SIZE);
    let chunk_y = floor_div(y, TERRAIN_CHUNK_SIZE);
    generate_terrain_chunk(
        chunk_x,
        chunk_y,
        i64::from(world_seed),
        WORLD_TERRAIN_OPTIONS,
    )
    .into_iter()
    .find(|tile| tile.x == x && tile.y == y)
    .map_or(BiomeRole::Grassland, |tile| tile.biome)
}

/// The deterministic P17 climate [`Biome`] the terrain generator assigns to
/// world tile `(x, y)` for `world_seed`. Same source of truth as [`tile_biome`]:
/// it regenerates the owning chunk from `WORLD_TERRAIN_OPTIONS` and reads the
/// stamped field (so it includes the river overlay). Falls back to
/// [`Biome::Plains`] for the (never-reached) absent-tile case.
#[must_use]
pub fn tile_climate_biome(world_seed: u32, x: i32, y: i32) -> Biome {
    let chunk_x = floor_div(x, TERRAIN_CHUNK_SIZE);
    let chunk_y = floor_div(y, TERRAIN_CHUNK_SIZE);
    generate_terrain_chunk(
        chunk_x,
        chunk_y,
        i64::from(world_seed),
        WORLD_TERRAIN_OPTIONS,
    )
    .into_iter()
    .find(|tile| tile.x == x && tile.y == y)
    .map_or(Biome::Plains, |tile| tile.climate_biome)
}

fn resolve_options(opts: TerrainOptions) -> ResolvedOptions {
    ResolvedOptions {
        max_height: opts.max_height.unwrap_or(DEFAULT_MAX_HEIGHT),
        height_scale: opts.height_scale.unwrap_or(0.08),
        octaves: opts.octaves.unwrap_or(4),
        persistence: opts.persistence.unwrap_or(0.5),
        moisture_scale: opts.moisture_scale.unwrap_or(0.06),
        village_anchor: opts.village_anchor.unwrap_or(Point { x: 0, y: 0 }),
        plateau_radius: opts.plateau_radius.unwrap_or(4),
        plateau_height: opts.plateau_height.unwrap_or(1),
        min_run_for_stair: opts.min_run_for_stair.unwrap_or(3),
        region_size: opts.region_size.unwrap_or(24),
        rivers_per_region: opts.rivers_per_region.unwrap_or(1),
        max_river_length: opts.max_river_length.unwrap_or(36),
        river_source_min_elevation: opts.river_source_min_elevation.unwrap_or(0.6),
        carve_rivers: opts.carve_rivers.unwrap_or(false),
        decorate: opts.decorate.unwrap_or(true),
    }
}

fn is_in_plateau(x: i32, y: i32, opts: &ResolvedOptions) -> bool {
    let dx = (x - opts.village_anchor.x).abs();
    let dy = (y - opts.village_anchor.y).abs();
    dx.max(dy) <= opts.plateau_radius
}

fn height_with(x: i32, y: i32, seed: i64, opts: &ResolvedOptions) -> i32 {
    if is_in_plateau(x, y, opts) {
        return opts.plateau_height;
    }

    let elevation = fractal_noise(
        f64::from(x),
        f64::from(y),
        seed,
        opts.octaves,
        opts.persistence,
        opts.height_scale,
    );
    let level = (elevation * f64::from(opts.max_height + 1)).floor() as i32;
    level.clamp(0, opts.max_height)
}

fn mask_to_cliff(edges: u8, lower: &[Direction], max_drop: i32) -> TerrainRole {
    match lower.len() {
        1 => {
            let facing = lower[0];
            TerrainRole::Cliff(CliffTerrainRole {
                edges,
                base: CliffBase::Edge,
                variant: format!("edge-{}", dir_label(facing)),
                facing: Some(facing),
                max_drop,
            })
        }
        4 => TerrainRole::Cliff(CliffTerrainRole {
            edges,
            base: CliffBase::Pillar,
            variant: "pillar".to_owned(),
            facing: None,
            max_drop,
        }),
        2 => {
            let a = lower[0];
            let b = lower[1];
            if opposite(a) == b {
                let (axis, facing) = if matches!(a, Direction::N | Direction::S) {
                    ("NS", Direction::N)
                } else {
                    ("EW", Direction::E)
                };
                TerrainRole::Cliff(CliffTerrainRole {
                    edges,
                    base: CliffBase::Ridge,
                    variant: format!("ridge-{axis}"),
                    facing: Some(facing),
                    max_drop,
                })
            } else {
                let (name, facing) = corner_name(a, b)
                    .expect("two non-opposite directions form a corner in four-way topology");
                TerrainRole::Cliff(CliffTerrainRole {
                    edges,
                    base: CliffBase::Corner,
                    variant: format!("corner-{name}"),
                    facing: Some(facing),
                    max_drop,
                })
            }
        }
        _ => {
            let higher = DIRECTIONS.iter().copied().find(|dir| !lower.contains(dir));
            TerrainRole::Cliff(CliffTerrainRole {
                edges,
                base: CliffBase::Spur,
                variant: higher
                    .map(|dir| format!("spur-{}", dir_label(dir)))
                    .unwrap_or_else(|| "spur".to_owned()),
                facing: higher,
                max_drop,
            })
        }
    }
}

fn terrain_role_at(x: i32, y: i32, seed: i64, opts: &ResolvedOptions) -> TerrainRole {
    let center = height_with(x, y, seed, opts);
    let mut neighbors = NeighborHeights {
        n: 0,
        e: 0,
        s: 0,
        w: 0,
    };

    for dir in DIRECTIONS {
        let (dx, dy) = dir_vec(dir);
        let height = height_with(x + dx, y + dy, seed, opts);
        neighbors.set(dir, height);
    }

    classify_cliff(center, neighbors)
}

fn stair_edge_dir(x: i32, y: i32, seed: i64, opts: &ResolvedOptions) -> Option<Direction> {
    let TerrainRole::Cliff(role) = terrain_role_at(x, y, seed, opts) else {
        return None;
    };
    if role.base != CliffBase::Edge {
        return None;
    }
    let facing = role.facing?;
    let (dx, dy) = dir_vec(facing);
    let center = height_with(x, y, seed, opts);
    let below = height_with(x + dx, y + dy, seed, opts);
    (center - below == 1).then_some(facing)
}

fn perp_axis(facing: Direction) -> (Direction, Direction) {
    if matches!(facing, Direction::N | Direction::S) {
        (Direction::W, Direction::E)
    } else {
        (Direction::N, Direction::S)
    }
}

fn derive_stairs(x: i32, y: i32, seed: i64, opts: &ResolvedOptions) -> Option<StairsRole> {
    let facing = stair_edge_dir(x, y, seed, opts)?;
    let (neg, pos) = perp_axis(facing);
    let (neg_dx, neg_dy) = dir_vec(neg);
    let (pos_dx, pos_dy) = dir_vec(pos);

    let mut ax = x;
    let mut ay = y;
    for _ in 0..MAX_RUN_SCAN {
        if stair_edge_dir(ax + neg_dx, ay + neg_dy, seed, opts) == Some(facing) {
            ax += neg_dx;
            ay += neg_dy;
        } else {
            break;
        }
    }

    let mut length = 1;
    let mut cx = ax + pos_dx;
    let mut cy = ay + pos_dy;
    for _ in 0..MAX_RUN_SCAN {
        if stair_edge_dir(cx, cy, seed, opts) == Some(facing) {
            length += 1;
            cx += pos_dx;
            cy += pos_dy;
        } else {
            break;
        }
    }

    if length < opts.min_run_for_stair {
        return None;
    }

    let chosen_index = (length - 1) / 2;
    let index = (x - ax) * pos_dx + (y - ay) * pos_dy;
    (index == chosen_index).then_some(StairsRole { facing })
}

fn collect_river_tiles(
    chunk_x: i32,
    chunk_y: i32,
    seed: i64,
    opts: &ResolvedOptions,
) -> HashMap<(i32, i32), RiverPathTile> {
    let origin_x = chunk_x * TERRAIN_CHUNK_SIZE;
    let origin_y = chunk_y * TERRAIN_CHUNK_SIZE;
    let reach = opts.max_river_length;
    let region_min_x = floor_div(origin_x - reach, opts.region_size);
    let region_max_x = floor_div(origin_x + TERRAIN_CHUNK_SIZE + reach, opts.region_size);
    let region_min_y = floor_div(origin_y - reach, opts.region_size);
    let region_max_y = floor_div(origin_y + TERRAIN_CHUNK_SIZE + reach, opts.region_size);
    let mut result = HashMap::new();

    for rx in region_min_x..=region_max_x {
        for ry in region_min_y..=region_max_y {
            for src in region_river_sources(rx, ry, seed, options_from_resolved(opts)) {
                let Some(path) = trace_river(src.x, src.y, seed, options_from_resolved(opts))
                else {
                    continue;
                };

                for tile in path {
                    if tile.x >= origin_x
                        && tile.x < origin_x + TERRAIN_CHUNK_SIZE
                        && tile.y >= origin_y
                        && tile.y < origin_y + TERRAIN_CHUNK_SIZE
                    {
                        result.insert((tile.x, tile.y), tile);
                    }
                }
            }
        }
    }

    result
}

fn floor_div(numerator: i32, denominator: i32) -> i32 {
    (f64::from(numerator) / f64::from(denominator)).floor() as i32
}

fn options_from_resolved(opts: &ResolvedOptions) -> TerrainOptions {
    TerrainOptions {
        max_height: Some(opts.max_height),
        height_scale: Some(opts.height_scale),
        octaves: Some(opts.octaves),
        persistence: Some(opts.persistence),
        moisture_scale: Some(opts.moisture_scale),
        village_anchor: Some(opts.village_anchor),
        plateau_radius: Some(opts.plateau_radius),
        plateau_height: Some(opts.plateau_height),
        min_run_for_stair: Some(opts.min_run_for_stair),
        region_size: Some(opts.region_size),
        rivers_per_region: Some(opts.rivers_per_region),
        max_river_length: Some(opts.max_river_length),
        river_source_min_elevation: Some(opts.river_source_min_elevation),
        carve_rivers: Some(opts.carve_rivers),
        decorate: Some(opts.decorate),
    }
}

pub fn derive_decoration(x: i32, y: i32, seed: i64, biome: BiomeRole) -> Option<DecorationRole> {
    let (tree_density, rock_density) = decor_density(biome);
    decoration_from_density(x, y, seed, tree_density, rock_density)
}

/// P17 density-driven decoration sampler keyed on the climate [`Biome`] instead
/// of the coarse [`BiomeRole`]. Uses the *identical* deterministic roll code as
/// [`derive_decoration`] — only the density thresholds come from the biome's
/// property table — so forests emit far more trees than plains for the same
/// area. Non-breaking: `generate_terrain_chunk`'s `decoration` field still uses
/// the [`BiomeRole`] path; this is what the client will consume to fix the
/// uniform-tree look.
#[must_use]
pub fn derive_biome_decoration(x: i32, y: i32, seed: i64, biome: Biome) -> Option<DecorationRole> {
    let (tree_density, rock_density) = biome.decoration_density();
    decoration_from_density(x, y, seed, tree_density, rock_density)
}

fn decoration_from_density(
    x: i32,
    y: i32,
    seed: i64,
    tree_density: f64,
    rock_density: f64,
) -> Option<DecorationRole> {
    let roll = lattice_value(
        i64::from(hash_seed(&[
            HashValue::Int(seed),
            HashValue::Text("decor"),
            HashValue::Int(i64::from(x)),
            HashValue::Int(i64::from(y)),
        ])),
        0,
        0,
    );

    if roll < tree_density {
        let species_roll = lattice_value(
            i64::from(hash_seed(&[
                HashValue::Int(seed),
                HashValue::Text("species"),
                HashValue::Int(i64::from(x)),
                HashValue::Int(i64::from(y)),
            ])),
            1,
            1,
        );
        return Some(DecorationRole::Tree {
            species: (species_roll * 4.0).floor() as i32,
        });
    }

    if roll < tree_density + rock_density {
        let size_roll = lattice_value(
            i64::from(hash_seed(&[
                HashValue::Int(seed),
                HashValue::Text("rock"),
                HashValue::Int(i64::from(x)),
                HashValue::Int(i64::from(y)),
            ])),
            2,
            2,
        );
        let resource_roll = lattice_value(
            i64::from(hash_seed(&[
                HashValue::Int(seed),
                HashValue::Text("ore"),
                HashValue::Int(i64::from(x)),
                HashValue::Int(i64::from(y)),
            ])),
            3,
            3,
        );
        return Some(DecorationRole::Rock {
            size: if size_roll < 0.5 {
                RockSize::Small
            } else if size_roll < 0.85 {
                RockSize::Medium
            } else {
                RockSize::Large
            },
            resource: resource_roll < 0.4,
        });
    }

    None
}

fn decor_density(biome: BiomeRole) -> (f64, f64) {
    match biome {
        BiomeRole::Lowland => (0.05, 0.02),
        BiomeRole::Grassland => (0.08, 0.03),
        BiomeRole::Forest => (0.45, 0.05),
        BiomeRole::Rocky => (0.03, 0.35),
        BiomeRole::Highland => (0.02, 0.15),
    }
}

fn dir_bit(dir: Direction) -> u8 {
    match dir {
        Direction::N => 1,
        Direction::E => 2,
        Direction::S => 4,
        Direction::W => 8,
    }
}

fn dir_vec(dir: Direction) -> (i32, i32) {
    match dir {
        Direction::N => (0, -1),
        Direction::E => (1, 0),
        Direction::S => (0, 1),
        Direction::W => (-1, 0),
    }
}

fn opposite(dir: Direction) -> Direction {
    match dir {
        Direction::N => Direction::S,
        Direction::E => Direction::W,
        Direction::S => Direction::N,
        Direction::W => Direction::E,
    }
}

fn dir_label(dir: Direction) -> &'static str {
    match dir {
        Direction::N => "N",
        Direction::E => "E",
        Direction::S => "S",
        Direction::W => "W",
    }
}

fn corner_name(a: Direction, b: Direction) -> Option<(&'static str, Direction)> {
    const CORNERS: [(Direction, Direction, &str); 4] = [
        (Direction::N, Direction::E, "NE"),
        (Direction::E, Direction::S, "SE"),
        (Direction::S, Direction::W, "SW"),
        (Direction::W, Direction::N, "NW"),
    ];

    CORNERS.iter().find_map(|&(d1, d2, name)| {
        ((a == d1 && b == d2) || (a == d2 && b == d1)).then_some((name, d1))
    })
}

impl NeighborHeights {
    fn get(self, dir: Direction) -> i32 {
        match dir {
            Direction::N => self.n,
            Direction::E => self.e,
            Direction::S => self.s,
            Direction::W => self.w,
        }
    }

    fn set(&mut self, dir: Direction, height: i32) {
        match dir {
            Direction::N => self.n = height,
            Direction::E => self.e = height,
            Direction::S => self.s = height,
            Direction::W => self.w = height,
        }
    }
}

pub(crate) fn hash_seed(values: &[HashValue<'_>]) -> u32 {
    let mut hash = 0_i32;

    for value in values {
        let string = value.to_js_string();
        for code_unit in string.encode_utf16() {
            hash = hash
                .wrapping_shl(5)
                .wrapping_sub(hash)
                .wrapping_add(i32::from(code_unit));
            hash &= hash;
        }
    }

    hash.unsigned_abs()
}

pub(crate) fn lattice_value(seed: i64, ix: i32, iy: i32) -> f64 {
    let mut h = hash_seed(&[
        HashValue::Int(seed),
        HashValue::Int(i64::from(ix)),
        HashValue::Int(i64::from(iy)),
    ]);
    h ^= h >> 13;
    h = h.wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    f64::from(h) / LATTICE_DIVISOR
}

pub(crate) fn fade(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

pub(crate) fn value_noise(x: f64, y: f64, seed: i64, scale: f64) -> f64 {
    let sx = x * scale;
    let sy = y * scale;
    let x0 = sx.floor() as i32;
    let y0 = sy.floor() as i32;
    let fx = fade(sx - f64::from(x0));
    let fy = fade(sy - f64::from(y0));

    let n00 = lattice_value(seed, x0, y0);
    let n10 = lattice_value(seed, x0 + 1, y0);
    let n01 = lattice_value(seed, x0, y0 + 1);
    let n11 = lattice_value(seed, x0 + 1, y0 + 1);

    let nx0 = n00 + (n10 - n00) * fx;
    let nx1 = n01 + (n11 - n01) * fx;
    nx0 + (nx1 - nx0) * fy
}

pub(crate) fn fractal_noise(
    x: f64,
    y: f64,
    seed: i64,
    octaves: i32,
    persistence: f64,
    scale: f64,
) -> f64 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = scale;
    let mut max_value = 0.0;

    for i in 0..octaves {
        value += value_noise(x, y, seed + i64::from(i * 1013), frequency) * amplitude;
        max_value += amplitude;
        amplitude *= persistence;
        frequency *= 2.0;
    }

    value / max_value
}

fn js_i32_xor(left: i64, right: i32) -> i32 {
    (left as i32) ^ right
}

impl HashValue<'_> {
    fn to_js_string(self) -> String {
        match self {
            Self::Int(value) => value.to_string(),
            Self::Number(value) => {
                let mut buffer = Buffer::new();
                buffer.format(value).to_owned()
            }
            Self::Text(value) => value.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::Value;

    const EPSILON: f64 = 1e-12;

    #[derive(Debug, Deserialize)]
    struct Fixture {
        counts: Counts,
        constants: Constants,
        options: Vec<OptionCase>,
        hash: Vec<HashCase>,
        lattice: Vec<LatticeCase>,
        fade: Vec<FadeCase>,
        #[serde(rename = "valueNoise")]
        value_noise: Vec<ValueNoiseCase>,
        #[serde(rename = "fractalNoise")]
        fractal_noise: Vec<FractalNoiseCase>,
        fields: Vec<FieldCase>,
    }

    #[derive(Debug, Deserialize)]
    struct Counts {
        options: usize,
        hash: usize,
        lattice: usize,
        fade: usize,
        #[serde(rename = "valueNoise")]
        value_noise: usize,
        #[serde(rename = "fractalNoise")]
        fractal_noise: usize,
        fields: usize,
        total: usize,
    }

    #[derive(Debug, Deserialize)]
    struct Constants {
        #[serde(rename = "terrainChunkSize")]
        terrain_chunk_size: i32,
        #[serde(rename = "defaultMaxHeight")]
        default_max_height: i32,
        directions: Vec<String>,
        #[serde(rename = "worldTerrainOptions")]
        world_terrain_options: FixtureOptions,
    }

    #[derive(Debug, Deserialize)]
    struct OptionCase {
        name: String,
        input: FixtureOptions,
        resolved: ResolvedFixtureOptions,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FixtureOptions {
        max_height: Option<i32>,
        height_scale: Option<f64>,
        octaves: Option<i32>,
        persistence: Option<f64>,
        moisture_scale: Option<f64>,
        village_anchor: Option<FixturePoint>,
        plateau_radius: Option<i32>,
        plateau_height: Option<i32>,
        min_run_for_stair: Option<i32>,
        region_size: Option<i32>,
        rivers_per_region: Option<i32>,
        max_river_length: Option<i32>,
        river_source_min_elevation: Option<f64>,
        carve_rivers: Option<bool>,
        decorate: Option<bool>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ResolvedFixtureOptions {
        max_height: i32,
        height_scale: f64,
        octaves: i32,
        persistence: f64,
        moisture_scale: f64,
        village_anchor: FixturePoint,
        plateau_radius: i32,
        plateau_height: i32,
        min_run_for_stair: i32,
        region_size: i32,
        rivers_per_region: i32,
        max_river_length: i32,
        river_source_min_elevation: f64,
        carve_rivers: bool,
        decorate: bool,
    }

    #[derive(Debug, Deserialize)]
    struct FixturePoint {
        x: i32,
        y: i32,
    }

    #[derive(Debug, Deserialize)]
    struct HashCase {
        inputs: Vec<Value>,
        value: u32,
    }

    #[derive(Debug, Deserialize)]
    struct LatticeCase {
        seed: i64,
        ix: i32,
        iy: i32,
        value: f64,
    }

    #[derive(Debug, Deserialize)]
    struct FadeCase {
        t: f64,
        value: f64,
    }

    #[derive(Debug, Deserialize)]
    struct ValueNoiseCase {
        x: f64,
        y: f64,
        seed: i64,
        scale: f64,
        value: f64,
    }

    #[derive(Debug, Deserialize)]
    struct FractalNoiseCase {
        x: f64,
        y: f64,
        seed: i64,
        octaves: i32,
        persistence: f64,
        scale: f64,
        value: f64,
    }

    #[derive(Debug, Deserialize)]
    struct FieldCase {
        #[serde(rename = "optionSet")]
        option_set: String,
        seed: i64,
        x: i32,
        y: i32,
        elevation: f64,
        moisture: f64,
        height: i32,
    }

    #[derive(Debug, Deserialize)]
    struct RoleFixture {
        seed: i64,
        counts: RoleCounts,
        cliff: Vec<CliffCase>,
        stairs: Vec<StairCase>,
        #[serde(rename = "riverSources")]
        river_sources: Vec<RiverSourceCase>,
        #[serde(rename = "riverTraces")]
        river_traces: Vec<RiverTraceCase>,
        #[serde(rename = "riverSegments")]
        river_segments: Vec<RiverSegmentCase>,
        biomes: Vec<BiomeCase>,
        decorations: Vec<DecorationCase>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ChunkFixture {
        generated_from: String,
        terrain_chunk_size: i32,
        counts: ChunkCounts,
        chunks: Vec<ChunkCase>,
    }

    #[derive(Debug, Deserialize)]
    struct ChunkCounts {
        chunks: usize,
        tiles: usize,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ChunkCase {
        name: String,
        chunk_x: i32,
        chunk_y: i32,
        seed: i64,
        opts: FixtureOptions,
        summary: ChunkSummary,
        tiles: Vec<FixtureTerrainTile>,
    }

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct ChunkSummary {
        biomes: HashMap<String, usize>,
        terrain: HashMap<String, usize>,
        rivers: usize,
        stairs: usize,
        decorations: usize,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RoleCounts {
        cliff: usize,
        stairs: usize,
        river_sources: usize,
        river_traces: usize,
        river_segments: usize,
        biomes: usize,
        decorations: usize,
        total: usize,
    }

    #[derive(Debug, Deserialize)]
    struct CliffCase {
        name: String,
        center: i32,
        neighbors: FixtureNeighborHeights,
        role: FixtureTerrainRole,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct FixtureNeighborHeights {
        n: i32,
        e: i32,
        s: i32,
        w: i32,
    }

    #[derive(Debug, Deserialize)]
    struct StairCase {
        x: i32,
        y: i32,
        height: i32,
        terrain: FixtureTerrainRole,
        stairs: Option<FixtureStairsRole>,
        #[serde(rename = "terrainStairAt")]
        terrain_stair_at: bool,
    }

    #[derive(Debug, Deserialize)]
    struct RiverSourceCase {
        #[serde(rename = "regionX")]
        region_x: i32,
        #[serde(rename = "regionY")]
        region_y: i32,
        sources: Vec<FixturePoint>,
    }

    #[derive(Debug, Deserialize)]
    struct RiverTraceCase {
        name: String,
        sx: i32,
        sy: i32,
        opts: FixtureOptions,
        path: Option<Vec<FixtureRiverPathTile>>,
        length: Option<usize>,
        #[serde(rename = "lastFive")]
        last_five: Option<Vec<FixtureRiverPathTile>>,
    }

    #[derive(Debug, Deserialize)]
    struct RiverSegmentCase {
        name: String,
        tile: FixtureRiverPathTile,
        role: FixtureRiverRole,
    }

    #[derive(Debug, Deserialize)]
    struct BiomeCase {
        height: i32,
        #[serde(rename = "maxHeight")]
        max_height: i32,
        moisture: f64,
        biome: String,
    }

    #[derive(Debug, Deserialize)]
    struct DecorationCase {
        x: i32,
        y: i32,
        biome: String,
        terrain: FixtureTerrainRole,
        river: Option<FixtureRiverRole>,
        stairs: Option<FixtureStairsRole>,
        decoration: Option<FixtureDecorationRole>,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureTerrainTile {
        x: i32,
        y: i32,
        elevation: f64,
        moisture: f64,
        height: i32,
        biome: String,
        terrain: FixtureTerrainRole,
        river: Option<FixtureRiverRole>,
        stairs: Option<FixtureStairsRole>,
        decoration: Option<FixtureDecorationRole>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(tag = "kind", rename_all = "lowercase")]
    enum FixtureTerrainRole {
        Flat,
        Cliff {
            edges: u8,
            base: String,
            variant: String,
            facing: Option<String>,
            #[serde(rename = "maxDrop")]
            max_drop: i32,
        },
    }

    #[derive(Debug, Deserialize)]
    struct FixtureStairsRole {
        facing: String,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureRiverPathTile {
        x: i32,
        y: i32,
        #[serde(rename = "inDir")]
        in_dir: Option<String>,
        #[serde(rename = "outDir")]
        out_dir: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureRiverRole {
        segment: String,
        #[serde(rename = "inDir")]
        in_dir: Option<String>,
        #[serde(rename = "outDir")]
        out_dir: Option<String>,
        facing: String,
    }

    #[derive(Debug, Deserialize)]
    #[serde(tag = "kind", rename_all = "lowercase")]
    enum FixtureDecorationRole {
        Tree { species: i32 },
        Rock { size: String, resource: bool },
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../docs/migration/fixtures/p2/terrain_fields.json"
        ))
        .expect("terrain fields fixture deserializes")
    }

    fn role_fixture() -> RoleFixture {
        serde_json::from_str(include_str!(
            "../../../docs/migration/fixtures/p2/terrain_roles.json"
        ))
        .expect("terrain roles fixture deserializes")
    }

    fn chunk_fixture() -> ChunkFixture {
        serde_json::from_str(include_str!(
            "../../../docs/migration/fixtures/p2/terrain_chunks.json"
        ))
        .expect("terrain chunks fixture deserializes")
    }

    fn assert_js_float_eq(actual: f64, expected: f64) {
        if actual.to_bits() == expected.to_bits() {
            return;
        }

        assert!(
            (actual - expected).abs() <= EPSILON,
            "actual {actual:?} expected {expected:?}"
        );
    }

    fn terrain_options(value: &FixtureOptions) -> TerrainOptions {
        TerrainOptions {
            max_height: value.max_height,
            height_scale: value.height_scale,
            octaves: value.octaves,
            persistence: value.persistence,
            moisture_scale: value.moisture_scale,
            village_anchor: value.village_anchor.as_ref().map(|point| Point {
                x: point.x,
                y: point.y,
            }),
            plateau_radius: value.plateau_radius,
            plateau_height: value.plateau_height,
            min_run_for_stair: value.min_run_for_stair,
            region_size: value.region_size,
            rivers_per_region: value.rivers_per_region,
            max_river_length: value.max_river_length,
            river_source_min_elevation: value.river_source_min_elevation,
            carve_rivers: value.carve_rivers,
            decorate: value.decorate,
        }
    }

    fn resolved_options(value: &ResolvedFixtureOptions) -> ResolvedOptions {
        ResolvedOptions {
            max_height: value.max_height,
            height_scale: value.height_scale,
            octaves: value.octaves,
            persistence: value.persistence,
            moisture_scale: value.moisture_scale,
            village_anchor: Point {
                x: value.village_anchor.x,
                y: value.village_anchor.y,
            },
            plateau_radius: value.plateau_radius,
            plateau_height: value.plateau_height,
            min_run_for_stair: value.min_run_for_stair,
            region_size: value.region_size,
            rivers_per_region: value.rivers_per_region,
            max_river_length: value.max_river_length,
            river_source_min_elevation: value.river_source_min_elevation,
            carve_rivers: value.carve_rivers,
            decorate: value.decorate,
        }
    }

    fn neighbor_heights(value: &FixtureNeighborHeights) -> NeighborHeights {
        NeighborHeights {
            n: value.n,
            e: value.e,
            s: value.s,
            w: value.w,
        }
    }

    fn direction(value: &str) -> Direction {
        match value {
            "N" => Direction::N,
            "E" => Direction::E,
            "S" => Direction::S,
            "W" => Direction::W,
            other => panic!("unknown direction {other}"),
        }
    }

    fn optional_direction(value: Option<&str>) -> Option<Direction> {
        value.map(direction)
    }

    fn biome_role(value: &str) -> BiomeRole {
        match value {
            "lowland" => BiomeRole::Lowland,
            "grassland" => BiomeRole::Grassland,
            "forest" => BiomeRole::Forest,
            "rocky" => BiomeRole::Rocky,
            "highland" => BiomeRole::Highland,
            other => panic!("unknown biome {other}"),
        }
    }

    fn cliff_base(value: &str) -> CliffBase {
        match value {
            "edge" => CliffBase::Edge,
            "corner" => CliffBase::Corner,
            "ridge" => CliffBase::Ridge,
            "spur" => CliffBase::Spur,
            "pillar" => CliffBase::Pillar,
            other => panic!("unknown cliff base {other}"),
        }
    }

    fn river_segment(value: &str) -> RiverSegment {
        match value {
            "start" => RiverSegment::Start,
            "straight" => RiverSegment::Straight,
            "bend" => RiverSegment::Bend,
            "end" => RiverSegment::End,
            other => panic!("unknown river segment {other}"),
        }
    }

    fn rock_size(value: &str) -> RockSize {
        match value {
            "small" => RockSize::Small,
            "medium" => RockSize::Medium,
            "large" => RockSize::Large,
            other => panic!("unknown rock size {other}"),
        }
    }

    fn terrain_role(value: &FixtureTerrainRole) -> TerrainRole {
        match value {
            FixtureTerrainRole::Flat => TerrainRole::Flat,
            FixtureTerrainRole::Cliff {
                edges,
                base,
                variant,
                facing,
                max_drop,
            } => TerrainRole::Cliff(CliffTerrainRole {
                edges: *edges,
                base: cliff_base(base),
                variant: variant.clone(),
                facing: optional_direction(facing.as_deref()),
                max_drop: *max_drop,
            }),
        }
    }

    fn stairs_role(value: &FixtureStairsRole) -> StairsRole {
        StairsRole {
            facing: direction(&value.facing),
        }
    }

    fn river_path_tile(value: &FixtureRiverPathTile) -> RiverPathTile {
        RiverPathTile {
            x: value.x,
            y: value.y,
            in_dir: optional_direction(value.in_dir.as_deref()),
            out_dir: optional_direction(value.out_dir.as_deref()),
        }
    }

    fn river_role(value: &FixtureRiverRole) -> RiverRole {
        RiverRole {
            segment: river_segment(&value.segment),
            in_dir: optional_direction(value.in_dir.as_deref()),
            out_dir: optional_direction(value.out_dir.as_deref()),
            facing: direction(&value.facing),
        }
    }

    fn decoration_role(value: &FixtureDecorationRole) -> DecorationRole {
        match value {
            FixtureDecorationRole::Tree { species } => DecorationRole::Tree { species: *species },
            FixtureDecorationRole::Rock { size, resource } => DecorationRole::Rock {
                size: rock_size(size),
                resource: *resource,
            },
        }
    }

    fn biome_label(value: BiomeRole) -> &'static str {
        match value {
            BiomeRole::Lowland => "lowland",
            BiomeRole::Grassland => "grassland",
            BiomeRole::Forest => "forest",
            BiomeRole::Rocky => "rocky",
            BiomeRole::Highland => "highland",
        }
    }

    fn terrain_label(value: &TerrainRole) -> &str {
        match value {
            TerrainRole::Flat => "flat",
            TerrainRole::Cliff(role) => &role.variant,
        }
    }

    fn increment_count(counts: &mut HashMap<String, usize>, key: &str) {
        *counts.entry(key.to_owned()).or_default() += 1;
    }

    fn chunk_summary(tiles: &[TerrainTile]) -> ChunkSummary {
        let mut biomes = HashMap::new();
        let mut terrain = HashMap::new();
        let mut rivers = 0;
        let mut stairs = 0;
        let mut decorations = 0;

        for tile in tiles {
            increment_count(&mut biomes, biome_label(tile.biome));
            increment_count(&mut terrain, terrain_label(&tile.terrain));
            rivers += usize::from(tile.river.is_some());
            stairs += usize::from(tile.stairs.is_some());
            decorations += usize::from(tile.decoration.is_some());
        }

        ChunkSummary {
            biomes,
            terrain,
            rivers,
            stairs,
            decorations,
        }
    }

    fn assert_terrain_tile_eq(actual: &TerrainTile, expected: &FixtureTerrainTile, name: &str) {
        assert_eq!(actual.x, expected.x, "{name} tile x");
        assert_eq!(actual.y, expected.y, "{name} tile y");
        assert_js_float_eq(actual.elevation, expected.elevation);
        assert_js_float_eq(actual.moisture, expected.moisture);
        assert_eq!(
            actual.height, expected.height,
            "{name} tile {},{} height",
            expected.x, expected.y
        );
        assert_eq!(
            actual.biome,
            biome_role(&expected.biome),
            "{name} tile {},{} biome",
            expected.x,
            expected.y
        );
        assert_eq!(
            actual.terrain,
            terrain_role(&expected.terrain),
            "{name} tile {},{} terrain",
            expected.x,
            expected.y
        );
        assert_eq!(
            actual.river,
            expected.river.as_ref().map(river_role),
            "{name} tile {},{} river",
            expected.x,
            expected.y
        );
        assert_eq!(
            actual.stairs,
            expected.stairs.as_ref().map(stairs_role),
            "{name} tile {},{} stairs",
            expected.x,
            expected.y
        );
        assert_eq!(
            actual.decoration,
            expected.decoration.as_ref().map(decoration_role),
            "{name} tile {},{} decoration",
            expected.x,
            expected.y
        );
    }

    fn hash_input_parts(inputs: &[Value]) -> Vec<HashValue<'_>> {
        inputs
            .iter()
            .map(|value| match value {
                Value::Number(number) => {
                    if let Some(value) = number.as_i64() {
                        HashValue::Int(value)
                    } else {
                        HashValue::Number(number.as_f64().expect("fixture number is f64"))
                    }
                }
                Value::String(text) => HashValue::Text(text.as_str()),
                other => panic!("unsupported hash input {other:?}"),
            })
            .collect()
    }

    fn options_by_name(fixture: &Fixture, name: &str) -> TerrainOptions {
        let option = fixture
            .options
            .iter()
            .find(|option| option.name == name)
            .expect("fixture option set exists");
        terrain_options(&option.input)
    }

    #[test]
    fn fixture_counts_match_generated_vectors() {
        let fixture = fixture();

        assert_eq!(fixture.counts.options, fixture.options.len());
        assert_eq!(fixture.counts.hash, fixture.hash.len());
        assert_eq!(fixture.counts.lattice, fixture.lattice.len());
        assert_eq!(fixture.counts.fade, fixture.fade.len());
        assert_eq!(fixture.counts.value_noise, fixture.value_noise.len());
        assert_eq!(fixture.counts.fractal_noise, fixture.fractal_noise.len());
        assert_eq!(fixture.counts.fields, fixture.fields.len());
        assert_eq!(
            fixture.counts.total,
            fixture.options.len()
                + fixture.hash.len()
                + fixture.lattice.len()
                + fixture.fade.len()
                + fixture.value_noise.len()
                + fixture.fractal_noise.len()
                + fixture.fields.len()
        );
    }

    #[test]
    fn public_constants_match_ts() {
        let fixture = fixture();

        assert_eq!(TERRAIN_CHUNK_SIZE, fixture.constants.terrain_chunk_size);
        assert_eq!(DEFAULT_MAX_HEIGHT, fixture.constants.default_max_height);
        assert_eq!(
            DIRECTIONS,
            [Direction::N, Direction::E, Direction::S, Direction::W]
        );
        assert_eq!(fixture.constants.directions, ["N", "E", "S", "W"]);
        assert_eq!(
            WORLD_TERRAIN_OPTIONS,
            terrain_options(&fixture.constants.world_terrain_options)
        );
    }

    #[test]
    fn options_resolve_like_ts_defaults() {
        for case in fixture().options {
            assert_eq!(
                resolve_options(terrain_options(&case.input)),
                resolved_options(&case.resolved),
                "option set {}",
                case.name
            );
        }
    }

    #[test]
    fn hash_seed_matches_terrain_gen_vectors() {
        for case in fixture().hash {
            let parts = hash_input_parts(&case.inputs);

            assert_eq!(hash_seed(&parts), case.value, "inputs {:?}", case.inputs);
        }
    }

    #[test]
    fn lattice_value_matches_terrain_gen_vectors() {
        for case in fixture().lattice {
            assert_js_float_eq(lattice_value(case.seed, case.ix, case.iy), case.value);
        }
    }

    #[test]
    fn fade_matches_terrain_gen_vectors() {
        for case in fixture().fade {
            assert_js_float_eq(fade(case.t), case.value);
        }
    }

    #[test]
    fn value_noise_matches_terrain_gen_vectors() {
        for case in fixture().value_noise {
            assert_js_float_eq(
                value_noise(case.x, case.y, case.seed, case.scale),
                case.value,
            );
        }
    }

    #[test]
    fn fractal_noise_matches_terrain_gen_vectors() {
        for case in fixture().fractal_noise {
            assert_js_float_eq(
                fractal_noise(
                    case.x,
                    case.y,
                    case.seed,
                    case.octaves,
                    case.persistence,
                    case.scale,
                ),
                case.value,
            );
        }
    }

    #[test]
    fn terrain_fields_match_ts_vectors() {
        let fixture = fixture();

        for case in &fixture.fields {
            let opts = options_by_name(&fixture, &case.option_set);

            assert_js_float_eq(
                terrain_elevation_at(case.x, case.y, case.seed, opts),
                case.elevation,
            );
            assert_js_float_eq(
                terrain_moisture_at(case.x, case.y, case.seed, opts),
                case.moisture,
            );
            assert_eq!(
                terrain_height_at(case.x, case.y, case.seed, opts),
                case.height,
                "height at ({}, {}) seed {} options {}",
                case.x,
                case.y,
                case.seed,
                case.option_set
            );
        }
    }

    #[test]
    fn terrain_fields_are_deterministic() {
        let opts = TerrainOptions {
            village_anchor: Some(Point { x: -3, y: 7 }),
            plateau_radius: Some(2),
            plateau_height: Some(4),
            max_height: Some(5),
            height_scale: Some(0.047),
            octaves: Some(5),
            persistence: Some(0.62),
            moisture_scale: Some(0.091),
            ..TerrainOptions::default()
        };

        let first = terrain_elevation_at(-13, 17, 1_781_313_000_000, opts);
        let second = terrain_elevation_at(-13, 17, 1_781_313_000_000, opts);
        assert_eq!(first.to_bits(), second.to_bits());

        let first = terrain_moisture_at(-13, 17, 1_781_313_000_000, opts);
        let second = terrain_moisture_at(-13, 17, 1_781_313_000_000, opts);
        assert_eq!(first.to_bits(), second.to_bits());

        assert_eq!(
            terrain_height_at(-3, 7, 1_781_313_000_000, opts),
            terrain_height_at(-3, 7, 1_781_313_000_000, opts)
        );
    }

    #[test]
    fn plateau_overrides_only_quantized_height() {
        let opts = TerrainOptions {
            village_anchor: Some(Point { x: -3, y: 7 }),
            plateau_radius: Some(2),
            plateau_height: Some(4),
            max_height: Some(5),
            ..TerrainOptions::default()
        };

        assert_eq!(terrain_height_at(-3, 7, 20260702, opts), 4);
        assert_eq!(terrain_height_at(-1, 9, 20260702, opts), 4);

        let inside = terrain_elevation_at(-3, 7, 20260702, opts);
        let outside = terrain_elevation_at(0, 10, 20260702, opts);
        assert_ne!(inside.to_bits(), outside.to_bits());
    }

    #[test]
    fn moisture_seed_uses_js_signed_xor() {
        assert_eq!(js_i32_xor(20260702, MOISTURE_SEED_MASK), -1_627_234_585);
    }

    #[test]
    fn role_fixture_counts_match_generated_vectors() {
        let fixture = role_fixture();

        assert_eq!(fixture.counts.cliff, fixture.cliff.len());
        assert_eq!(fixture.counts.stairs, fixture.stairs.len());
        assert_eq!(fixture.counts.river_sources, fixture.river_sources.len());
        assert_eq!(fixture.counts.river_traces, fixture.river_traces.len());
        assert_eq!(fixture.counts.river_segments, fixture.river_segments.len());
        assert_eq!(fixture.counts.biomes, fixture.biomes.len());
        assert_eq!(fixture.counts.decorations, fixture.decorations.len());
        assert_eq!(
            fixture.counts.total,
            fixture.cliff.len()
                + fixture.stairs.len()
                + fixture.river_sources.len()
                + fixture.river_traces.len()
                + fixture.river_segments.len()
                + fixture.biomes.len()
                + fixture.decorations.len()
        );
    }

    #[test]
    fn chunk_fixture_counts_match_generated_vectors() {
        let fixture = chunk_fixture();

        assert_eq!(
            fixture.generated_from,
            "lib/game/terrainGen.ts generateTerrainChunk"
        );
        assert_eq!(fixture.terrain_chunk_size, TERRAIN_CHUNK_SIZE);
        assert_eq!(fixture.counts.chunks, fixture.chunks.len());
        assert_eq!(
            fixture.counts.tiles,
            fixture
                .chunks
                .iter()
                .map(|chunk| chunk.tiles.len())
                .sum::<usize>()
        );
        assert_eq!(
            fixture.counts.tiles,
            fixture.counts.chunks * (TERRAIN_CHUNK_SIZE * TERRAIN_CHUNK_SIZE) as usize
        );
    }

    #[test]
    fn cliff_roles_match_ts_vectors() {
        for case in role_fixture().cliff {
            assert_eq!(
                classify_cliff(case.center, neighbor_heights(&case.neighbors)),
                terrain_role(&case.role),
                "cliff case {}",
                case.name
            );
        }
    }

    #[test]
    fn stair_roles_match_ts_vectors() {
        let fixture = role_fixture();
        let opts = resolve_options(TerrainOptions::default());

        for case in fixture.stairs {
            assert_eq!(
                height_with(case.x, case.y, fixture.seed, &opts),
                case.height,
                "height at {},{}",
                case.x,
                case.y
            );
            assert_eq!(
                terrain_role_at(case.x, case.y, fixture.seed, &opts),
                terrain_role(&case.terrain),
                "terrain at {},{}",
                case.x,
                case.y
            );
            assert_eq!(
                derive_stairs(case.x, case.y, fixture.seed, &opts),
                case.stairs.as_ref().map(stairs_role),
                "stairs at {},{}",
                case.x,
                case.y
            );
            assert_eq!(
                terrain_stair_at(case.x, case.y, fixture.seed, TerrainOptions::default()),
                case.terrain_stair_at,
                "terrain_stair_at {},{}",
                case.x,
                case.y
            );
        }
    }

    #[test]
    fn river_sources_match_ts_vectors() {
        let fixture = role_fixture();

        for case in fixture.river_sources {
            let expected: Vec<Point> = case
                .sources
                .iter()
                .map(|point| Point {
                    x: point.x,
                    y: point.y,
                })
                .collect();
            assert_eq!(
                region_river_sources(
                    case.region_x,
                    case.region_y,
                    fixture.seed,
                    TerrainOptions::default()
                ),
                expected,
                "river sources {},{}",
                case.region_x,
                case.region_y
            );
        }
    }

    #[test]
    fn river_paths_match_ts_vectors() {
        let fixture = role_fixture();

        for case in fixture.river_traces {
            let path = trace_river(case.sx, case.sy, fixture.seed, terrain_options(&case.opts));
            if let Some(expected) = &case.path {
                let expected: Vec<RiverPathTile> = expected.iter().map(river_path_tile).collect();
                assert_eq!(path, Some(expected), "river trace {}", case.name);
            }
            if let Some(expected_length) = case.length {
                let actual = path.as_ref().expect("expected trace path");
                assert_eq!(actual.len(), expected_length, "river trace {}", case.name);
            }
            if let Some(expected_last_five) = &case.last_five {
                let actual = path.as_ref().expect("expected trace path");
                let actual_last_five = &actual[actual.len() - expected_last_five.len()..];
                let expected: Vec<RiverPathTile> =
                    expected_last_five.iter().map(river_path_tile).collect();
                assert_eq!(
                    actual_last_five,
                    expected.as_slice(),
                    "river trace {}",
                    case.name
                );
            }
        }
    }

    #[test]
    fn river_segments_match_ts_vectors() {
        for case in role_fixture().river_segments {
            assert_eq!(
                classify_river_segment(&river_path_tile(&case.tile)),
                river_role(&case.role),
                "river segment {}",
                case.name
            );
        }
    }

    #[test]
    fn biome_roles_match_ts_threshold_vectors() {
        for case in role_fixture().biomes {
            assert_eq!(
                classify_biome(case.height, case.max_height, case.moisture),
                biome_role(&case.biome),
                "biome height {} max {} moisture {}",
                case.height,
                case.max_height,
                case.moisture
            );
        }
    }

    #[test]
    fn decoration_roles_match_ts_vectors() {
        let fixture = role_fixture();

        for case in fixture.decorations {
            assert_eq!(
                derive_decoration(case.x, case.y, fixture.seed, biome_role(&case.biome)),
                case.decoration.as_ref().map(decoration_role),
                "decoration at {},{}",
                case.x,
                case.y
            );
        }
    }

    #[test]
    fn tile_has_tree_matches_the_generated_chunk_decoration() {
        // `tile_has_tree` must agree, tile-for-tile, with what the terrain generator
        // (and hence the client renderer) places, across chunk boundaries too.
        let seed = 123u32;
        for chunk_x in -1..=1 {
            for chunk_y in -1..=1 {
                let chunk = generate_terrain_chunk(
                    chunk_x,
                    chunk_y,
                    i64::from(seed),
                    WORLD_TERRAIN_OPTIONS,
                );
                for terrain_tile in chunk {
                    let expected =
                        matches!(terrain_tile.decoration, Some(DecorationRole::Tree { .. }));
                    assert_eq!(
                        tile_has_tree(seed, terrain_tile.x, terrain_tile.y),
                        expected,
                        "tree mismatch at {},{}",
                        terrain_tile.x,
                        terrain_tile.y
                    );
                }
            }
        }
    }

    #[test]
    fn tile_biome_matches_the_generated_chunk_biome() {
        // `tile_biome` (the movement surface-speed source) must agree tile-for-tile
        // with the generated chunk, across chunk boundaries.
        let seed = 123u32;
        for chunk_x in -1..=1 {
            for chunk_y in -1..=1 {
                let chunk = generate_terrain_chunk(
                    chunk_x,
                    chunk_y,
                    i64::from(seed),
                    WORLD_TERRAIN_OPTIONS,
                );
                for terrain_tile in chunk {
                    assert_eq!(
                        tile_biome(seed, terrain_tile.x, terrain_tile.y),
                        terrain_tile.biome,
                        "biome mismatch at {},{}",
                        terrain_tile.x,
                        terrain_tile.y
                    );
                }
            }
        }
    }

    #[test]
    fn terrain_chunks_match_ts_fixture_tile_for_tile() {
        for case in chunk_fixture().chunks {
            let actual = generate_terrain_chunk(
                case.chunk_x,
                case.chunk_y,
                case.seed,
                terrain_options(&case.opts),
            );

            assert_eq!(
                actual.len(),
                (TERRAIN_CHUNK_SIZE * TERRAIN_CHUNK_SIZE) as usize,
                "{} tile count",
                case.name
            );
            assert_eq!(
                chunk_summary(&actual),
                case.summary,
                "{} summary",
                case.name
            );

            for (actual_tile, expected_tile) in actual.iter().zip(&case.tiles) {
                assert_terrain_tile_eq(actual_tile, expected_tile, &case.name);
            }
        }
    }

    #[test]
    fn terrain_chunk_generation_is_deterministic() {
        for case in chunk_fixture().chunks {
            let opts = terrain_options(&case.opts);
            let first = generate_terrain_chunk(case.chunk_x, case.chunk_y, case.seed, opts);
            let second = generate_terrain_chunk(case.chunk_x, case.chunk_y, case.seed, opts);

            assert_eq!(first, second, "chunk {}", case.name);
        }
    }

    #[test]
    fn adjacent_chunks_are_contiguous_at_borders() {
        let seed = 20260702;
        let opts = TerrainOptions::default();
        let resolved = resolve_options(opts);
        let west = generate_terrain_chunk(0, 0, seed, opts);
        let east = generate_terrain_chunk(1, 0, seed, opts);
        let south = generate_terrain_chunk(0, 1, seed, opts);

        for y in 0..TERRAIN_CHUNK_SIZE {
            let left = &west[(y * TERRAIN_CHUNK_SIZE + (TERRAIN_CHUNK_SIZE - 1)) as usize];
            let right = &east[(y * TERRAIN_CHUNK_SIZE) as usize];

            assert_eq!(left.x + 1, right.x);
            assert_eq!(left.y, right.y);
            assert_eq!(left.height, terrain_height_at(left.x, left.y, seed, opts));
            assert_eq!(
                right.height,
                terrain_height_at(right.x, right.y, seed, opts)
            );
            assert_eq!(
                left.terrain,
                terrain_role_at(left.x, left.y, seed, &resolved)
            );
            assert_eq!(
                right.terrain,
                terrain_role_at(right.x, right.y, seed, &resolved)
            );
        }

        for x in 0..TERRAIN_CHUNK_SIZE {
            let top = &west[((TERRAIN_CHUNK_SIZE - 1) * TERRAIN_CHUNK_SIZE + x) as usize];
            let bottom = &south[x as usize];

            assert_eq!(top.x, bottom.x);
            assert_eq!(top.y + 1, bottom.y);
            assert_eq!(top.height, terrain_height_at(top.x, top.y, seed, opts));
            assert_eq!(
                bottom.height,
                terrain_height_at(bottom.x, bottom.y, seed, opts)
            );
            assert_eq!(top.terrain, terrain_role_at(top.x, top.y, seed, &resolved));
            assert_eq!(
                bottom.terrain,
                terrain_role_at(bottom.x, bottom.y, seed, &resolved)
            );
        }
    }

    #[test]
    fn role_derivation_is_deterministic() {
        let opts = TerrainOptions {
            village_anchor: Some(Point { x: -3, y: 7 }),
            plateau_radius: Some(2),
            plateau_height: Some(4),
            max_height: Some(5),
            height_scale: Some(0.047),
            octaves: Some(5),
            persistence: Some(0.62),
            moisture_scale: Some(0.091),
            min_run_for_stair: Some(2),
            rivers_per_region: Some(2),
            max_river_length: Some(12),
            ..TerrainOptions::default()
        };
        let resolved = resolve_options(opts);

        assert_eq!(
            terrain_role_at(-13, 17, 1_781_313_000_000, &resolved),
            terrain_role_at(-13, 17, 1_781_313_000_000, &resolved)
        );
        assert_eq!(
            derive_stairs(-13, 17, 1_781_313_000_000, &resolved),
            derive_stairs(-13, 17, 1_781_313_000_000, &resolved)
        );
        assert_eq!(
            region_river_sources(-2, 3, 1_781_313_000_000, opts),
            region_river_sources(-2, 3, 1_781_313_000_000, opts)
        );
        assert_eq!(
            trace_river(-50, 81, 1_781_313_000_000, opts),
            trace_river(-50, 81, 1_781_313_000_000, opts)
        );
        assert_eq!(
            derive_decoration(-13, 17, 1_781_313_000_000, BiomeRole::Forest),
            derive_decoration(-13, 17, 1_781_313_000_000, BiomeRole::Forest)
        );
    }

    #[test]
    fn role_sampling_uses_world_coordinates_across_chunk_border() {
        let fixture = role_fixture();
        let opts = resolve_options(TerrainOptions::default());
        let border_cases: Vec<_> = fixture
            .decorations
            .iter()
            .filter(|case| (case.x, case.y) == (11, 5) || (case.x, case.y) == (12, 5))
            .collect();

        assert_eq!(border_cases.len(), 2);
        for case in border_cases {
            assert_eq!(
                terrain_role_at(case.x, case.y, fixture.seed, &opts),
                terrain_role(&case.terrain),
                "terrain role at border {},{}",
                case.x,
                case.y
            );
            assert_eq!(
                derive_stairs(case.x, case.y, fixture.seed, &opts),
                case.stairs.as_ref().map(stairs_role),
                "stairs at border {},{}",
                case.x,
                case.y
            );
            assert_eq!(
                classify_river_segment(&RiverPathTile {
                    x: case.x,
                    y: case.y,
                    in_dir: None,
                    out_dir: None,
                }),
                RiverRole {
                    segment: RiverSegment::Start,
                    in_dir: None,
                    out_dir: None,
                    facing: Direction::N,
                }
            );
            assert!(case.river.is_none());
        }
    }

    // --- P17 climate biome layer --------------------------------------------

    #[test]
    fn climate_biome_is_stamped_and_deterministic() {
        let seed = 20260702;
        let first = generate_terrain_chunk(0, 0, seed, WORLD_TERRAIN_OPTIONS);
        let second = generate_terrain_chunk(0, 0, seed, WORLD_TERRAIN_OPTIONS);
        for (a, b) in first.iter().zip(&second) {
            assert_eq!(a.climate_biome, b.climate_biome, "tile {},{}", a.x, a.y);
        }
    }

    #[test]
    fn tile_climate_biome_matches_the_generated_chunk() {
        // Mirror of `tile_biome_matches_the_generated_chunk_biome`, for the
        // climate layer — the standalone accessor must agree tile-for-tile.
        let seed = 123u32;
        for chunk_x in -1..=1 {
            for chunk_y in -1..=1 {
                let chunk = generate_terrain_chunk(
                    chunk_x,
                    chunk_y,
                    i64::from(seed),
                    WORLD_TERRAIN_OPTIONS,
                );
                for tile in chunk {
                    assert_eq!(
                        tile_climate_biome(seed, tile.x, tile.y),
                        tile.climate_biome,
                        "climate biome mismatch at {},{}",
                        tile.x,
                        tile.y
                    );
                }
            }
        }
    }

    #[test]
    fn founding_plateau_tiles_resolve_to_a_grass_biome() {
        let seed = 20260702;
        let anchor = WORLD_TERRAIN_OPTIONS.village_anchor.expect("world anchor");
        let biome = tile_climate_biome(seed as u32, anchor.x, anchor.y);
        assert_eq!(biome, crate::climate::Biome::Plains);
        assert!(matches!(
            biome.surface_role(),
            BiomeRole::Grassland | BiomeRole::Lowland
        ));
    }

    #[test]
    fn climate_regions_are_large_not_noisy() {
        // Walk a long horizontal transect and count biome changes. With the
        // very low-frequency climate noise, adjacent tiles almost always share a
        // biome, so runs are long (regions ~10× the village). Random assignment
        // would flip on nearly every step.
        let seed = 20260702;
        let length = 300;
        let mut changes = 0;
        let mut prev = tile_climate_biome(seed as u32, -150, 40);
        for x in -149..(-150 + length) {
            let biome = tile_climate_biome(seed as u32, x, 40);
            if biome != prev {
                changes += 1;
            }
            prev = biome;
        }
        // Average run length comfortably exceeds a handful of tiles.
        assert!(
            changes < length / 8,
            "expected large regions, saw {changes} changes over {length} tiles"
        );
    }

    #[test]
    fn biome_decoration_density_favours_forests_over_plains() {
        // Over the same area, a forest biome must emit many more trees than a
        // grass biome — the density fix the client will render from.
        let seed = 4242;
        let mut forest_trees = 0;
        let mut plains_trees = 0;
        for x in 0..50 {
            for y in 0..50 {
                if matches!(
                    derive_biome_decoration(x, y, seed, crate::climate::Biome::OakForest),
                    Some(DecorationRole::Tree { .. })
                ) {
                    forest_trees += 1;
                }
                if matches!(
                    derive_biome_decoration(x, y, seed, crate::climate::Biome::Plains),
                    Some(DecorationRole::Tree { .. })
                ) {
                    plains_trees += 1;
                }
            }
        }
        assert!(
            forest_trees > plains_trees * 5,
            "forest {forest_trees} vs plains {plains_trees}"
        );
        // Desert emits (almost) no trees.
        let desert_trees = (0..50)
            .flat_map(|x| (0..50).map(move |y| (x, y)))
            .filter(|&(x, y)| {
                matches!(
                    derive_biome_decoration(x, y, seed, crate::climate::Biome::Desert),
                    Some(DecorationRole::Tree { .. })
                )
            })
            .count();
        assert!(desert_trees < plains_trees, "desert {desert_trees}");
    }

    #[test]
    fn derive_decoration_role_path_is_unchanged_by_refactor() {
        // The BiomeRole decoration path (pinned by the golden chunk fixture)
        // must be byte-identical after extracting the shared density inner.
        let seed = 20260702;
        for biome in [
            BiomeRole::Lowland,
            BiomeRole::Grassland,
            BiomeRole::Forest,
            BiomeRole::Rocky,
            BiomeRole::Highland,
        ] {
            for x in -3..3 {
                for y in -3..3 {
                    let (tree, rock) = decor_density(biome);
                    assert_eq!(
                        derive_decoration(x, y, seed, biome),
                        decoration_from_density(x, y, seed, tree, rock),
                    );
                }
            }
        }
    }

    #[test]
    fn climate_fields_are_deterministic_and_in_range() {
        let seed = 1_781_313_000_000;
        for &(x, y) in &[(-13, 17), (0, 0), (240, -80)] {
            for sample in [
                terrain_temperature_at(x, y, seed),
                terrain_humidity_at(x, y, seed),
                terrain_weirdness_at(x, y, seed),
            ] {
                assert!((0.0..1.0).contains(&sample), "field {sample} out of range");
            }
            assert_eq!(
                terrain_temperature_at(x, y, seed),
                terrain_temperature_at(x, y, seed)
            );
        }
    }
}
