//! World coordinate helpers ported from `lib/game/worldGen.ts` and terrain-backed
//! gameplay chunk generation ported from `lib/game/terrainWorld.ts`.

use crate::{
    biomes::{BiomeType, MaxResources, OverlayFeature, biome_properties, calculate_danger_level},
    climate::Biome,
    noise::{HashSeedPart, create_seeded_random, hash_seed},
    terrain_gen::{BiomeRole, TerrainTile, WORLD_TERRAIN_OPTIONS, generate_terrain_chunk},
    types::TileType,
};

pub const CHUNK_SIZE: i32 = 12;
pub const COLONY_SAFE_RADIUS: f64 = 3.5;
pub const COLONY_WATER_RADIUS: f64 = 5.5;
const INFINITE_WATER: u32 = 999;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkCoords {
    pub chunk_x: i32,
    pub chunk_y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoords {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileResources {
    pub food: u32,
    pub herbs: u32,
    pub water: u32,
    /// Finite rare mountain deposit.
    pub gem: u32,
    /// Finite wetland/badlands deposit.
    pub clay: u32,
    /// Finite beach/desert deposit.
    pub sand: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldTileData {
    pub x: i32,
    pub y: i32,
    pub tile_type: TileType,
    pub resources: TileResources,
    pub max_resources: MaxResources,
    pub danger_level: f64,
    pub path_wear: u32,
    pub last_depleted: i64,
    pub overlay_feature: Option<OverlayFeature>,
}

#[must_use]
pub fn tile_to_chunk(x: i32, y: i32) -> ChunkCoords {
    ChunkCoords {
        chunk_x: x.div_euclid(CHUNK_SIZE),
        chunk_y: y.div_euclid(CHUNK_SIZE),
    }
}

#[must_use]
pub fn chunk_to_tile(chunk_x: i32, chunk_y: i32) -> TileCoords {
    TileCoords {
        x: chunk_x * CHUNK_SIZE,
        y: chunk_y * CHUNK_SIZE,
    }
}

#[must_use]
pub fn get_colony_position() -> TileCoords {
    TileCoords {
        x: CHUNK_SIZE / 2,
        y: CHUNK_SIZE / 2,
    }
}

#[must_use]
pub fn generate_world_chunk(
    chunk_x: i32,
    chunk_y: i32,
    seed: i64,
    colony_x: i32,
    colony_y: i32,
) -> Vec<WorldTileData> {
    let terrain = generate_terrain_chunk(chunk_x, chunk_y, seed, WORLD_TERRAIN_OPTIONS);
    let mut tiles: Vec<WorldTileData> = terrain
        .iter()
        .map(|tile| terrain_to_world_tile(tile, seed, colony_x, colony_y))
        .collect();

    let min_x = chunk_x * CHUNK_SIZE;
    let min_y = chunk_y * CHUNK_SIZE;
    let contains_colony = colony_x >= min_x
        && colony_x < min_x + CHUNK_SIZE
        && colony_y >= min_y
        && colony_y < min_y + CHUNK_SIZE;
    if contains_colony {
        ensure_water_near_colony(&mut tiles, seed, colony_x, colony_y);
    }

    tiles
}

fn terrain_to_world_tile(
    tile: &TerrainTile,
    seed: i64,
    colony_x: i32,
    colony_y: i32,
) -> WorldTileData {
    let dist = distance_to(tile.x, tile.y, colony_x, colony_y);

    if tile.river.is_some() {
        return river_tile(tile.x, tile.y);
    }

    let biome_type = biome_role_to_type(tile.biome);
    let props = biome_properties(biome_type);
    let mut rng = create_seeded_random(f64::from(hash_seed(&[
        HashSeedPart::Number(seed as f64),
        HashSeedPart::Number(f64::from(tile.x)),
        HashSeedPart::Number(f64::from(tile.y)),
    ])));

    let (gem, clay, sand) = natural_deposits_for_biome(tile.climate_biome);
    WorldTileData {
        x: tile.x,
        y: tile.y,
        tile_type: biome_role_to_tile_type(tile.biome),
        resources: TileResources {
            food: rng_roll_u32(
                &mut rng,
                props.base_resources.food.min,
                props.base_resources.food.max,
            ),
            herbs: rng_roll_u32(
                &mut rng,
                props.base_resources.herbs.min,
                props.base_resources.herbs.max,
            ),
            water: props.base_resources.water,
            gem,
            clay,
            sand,
        },
        max_resources: props.max_resources,
        danger_level: calculate_danger_level(biome_type, None, dist),
        path_wear: 0,
        last_depleted: 0,
        overlay_feature: None,
    }
}

#[must_use]
pub const fn natural_deposits_for_biome(biome: Biome) -> (u32, u32, u32) {
    match biome {
        Biome::Mountains => (2, 0, 0),
        Biome::Badlands | Biome::Swamp | Biome::Marsh => (0, 12, 0),
        Biome::Beach | Biome::Desert => (0, 0, 16),
        _ => (0, 0, 0),
    }
}

fn ensure_water_near_colony(tiles: &mut [WorldTileData], seed: i64, colony_x: i32, colony_y: i32) {
    let has_water = tiles.iter().any(|tile| {
        tile.resources.water > 0
            && distance_to(tile.x, tile.y, colony_x, colony_y) <= COLONY_WATER_RADIUS
    });
    if has_water {
        return;
    }

    let candidates: Vec<usize> = tiles
        .iter()
        .enumerate()
        .filter_map(|(index, tile)| {
            let dist = distance_to(tile.x, tile.y, colony_x, colony_y);
            (dist > COLONY_SAFE_RADIUS && dist <= COLONY_WATER_RADIUS).then_some(index)
        })
        .collect();
    if candidates.is_empty() {
        return;
    }

    let mut rng = create_seeded_random(f64::from(hash_seed(&[
        HashSeedPart::Number(seed as f64),
        HashSeedPart::Text("starter_pond"),
    ])));
    let pond_index = candidates[(rng.next() * candidates.len() as f64).floor() as usize];
    let pond = &mut tiles[pond_index];
    pond.tile_type = TileType::River;
    pond.overlay_feature = Some(OverlayFeature::River);
    pond.resources = TileResources {
        food: 0,
        herbs: 0,
        water: INFINITE_WATER,
        gem: 0,
        clay: 0,
        sand: 0,
    };
    pond.max_resources = MaxResources { food: 0, herbs: 0 };
    pond.danger_level = 5.0;
}

fn river_tile(x: i32, y: i32) -> WorldTileData {
    WorldTileData {
        x,
        y,
        tile_type: TileType::River,
        resources: TileResources {
            food: 0,
            herbs: 0,
            water: INFINITE_WATER,
            gem: 0,
            clay: 0,
            sand: 0,
        },
        max_resources: MaxResources { food: 0, herbs: 0 },
        danger_level: 5.0,
        path_wear: 0,
        last_depleted: 0,
        overlay_feature: Some(OverlayFeature::River),
    }
}

fn biome_role_to_type(role: BiomeRole) -> BiomeType {
    match role {
        BiomeRole::Lowland | BiomeRole::Grassland => BiomeType::Meadow,
        BiomeRole::Forest => BiomeType::OakForest,
        BiomeRole::Rocky | BiomeRole::Highland => BiomeType::Mountains,
    }
}

fn biome_role_to_tile_type(role: BiomeRole) -> TileType {
    match role {
        BiomeRole::Lowland => TileType::Meadow,
        BiomeRole::Grassland => TileType::Field,
        BiomeRole::Forest => TileType::Forest,
        BiomeRole::Rocky | BiomeRole::Highland => TileType::Mountains,
    }
}

fn distance_to(x: i32, y: i32, cx: i32, cy: i32) -> f64 {
    let dx = f64::from(x - cx);
    let dy = f64::from(y - cy);
    (dx.mul_add(dx, dy * dy)).sqrt()
}

fn rng_roll_u32(rng: &mut crate::noise::SeededRandom, min: u32, max: u32) -> u32 {
    rng.int(
        i32::try_from(min).expect("resource min fits in i32"),
        i32::try_from(max).expect("resource max fits in i32"),
    )
    .try_into()
    .expect("resource roll is non-negative")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        counts: Counts,
        constants: Constants,
        tile_cases: Vec<TileCase>,
        chunk_cases: Vec<ChunkCase>,
        colony_by_seed: Vec<ColonyCase>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Counts {
        tile_cases: usize,
        chunk_cases: usize,
        colony_by_seed: usize,
        total: usize,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Constants {
        chunk_size: i32,
        colony_safe_radius: f64,
        colony_water_radius: f64,
    }

    #[derive(Debug, Deserialize)]
    struct TileCase {
        x: i32,
        y: i32,
        chunk: FixtureChunkCoords,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FixtureChunkCoords {
        chunk_x: i32,
        chunk_y: i32,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ChunkCase {
        chunk_x: i32,
        chunk_y: i32,
        tile: TileCoordsFixture,
    }

    #[derive(Debug, Deserialize)]
    struct TileCoordsFixture {
        x: i32,
        y: i32,
    }

    #[derive(Debug, Deserialize)]
    struct ColonyCase {
        seed: i64,
        position: TileCoordsFixture,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct WorldChunkFixture {
        counts: WorldChunkCounts,
        constants: Constants,
        colony: TileCoordsFixture,
        cases: Vec<WorldChunkCase>,
    }

    #[derive(Debug, Deserialize)]
    struct WorldChunkCounts {
        cases: usize,
        tiles: usize,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct WorldChunkCase {
        seed: i64,
        chunk_x: i32,
        chunk_y: i32,
        count: usize,
        type_counts: HashMap<String, usize>,
        water_near_colony: usize,
        safe_rivers: usize,
        tiles: Vec<FixtureWorldTile>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FixtureWorldTile {
        x: i32,
        y: i32,
        #[serde(rename = "type")]
        tile_type: String,
        resources: FixtureResources,
        max_resources: FixtureMaxResources,
        danger_level: f64,
        path_wear: u32,
        last_depleted: i64,
        overlay_feature: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureResources {
        food: u32,
        herbs: u32,
        water: u32,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureMaxResources {
        food: u32,
        herbs: u32,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../docs/migration/fixtures/p2/world_coords.json"
        ))
        .expect("world coordinate fixture deserializes")
    }

    fn world_chunk_fixture() -> WorldChunkFixture {
        serde_json::from_str(include_str!(
            "../../../docs/migration/fixtures/p2/world_chunks.json"
        ))
        .expect("world chunk fixture deserializes")
    }

    fn assert_js_float_eq(actual: f64, expected: f64, context: &str) {
        if actual.to_bits() == expected.to_bits() {
            return;
        }

        assert!(
            (actual - expected).abs() <= 1e-12,
            "{context}: actual {actual:?} expected {expected:?}"
        );
    }

    fn assert_tile_matches(actual: &WorldTileData, expected: &FixtureWorldTile, context: &str) {
        assert_eq!(actual.x, expected.x, "{context} x");
        assert_eq!(actual.y, expected.y, "{context} y");
        assert_eq!(
            actual.tile_type,
            expected
                .tile_type
                .parse::<TileType>()
                .expect("fixture tile type parses"),
            "{context} type"
        );
        assert_eq!(
            actual.resources.food, expected.resources.food,
            "{context} food"
        );
        assert_eq!(
            actual.resources.herbs, expected.resources.herbs,
            "{context} herbs"
        );
        assert_eq!(
            actual.resources.water, expected.resources.water,
            "{context} water"
        );
        assert_eq!(
            actual.max_resources,
            MaxResources {
                food: expected.max_resources.food,
                herbs: expected.max_resources.herbs,
            },
            "{context} max resources"
        );
        assert_js_float_eq(actual.danger_level, expected.danger_level, context);
        assert_eq!(actual.path_wear, expected.path_wear, "{context} path wear");
        assert_eq!(
            actual.last_depleted, expected.last_depleted,
            "{context} last depleted"
        );
        assert_eq!(
            actual.overlay_feature,
            expected
                .overlay_feature
                .as_deref()
                .map(str::parse)
                .transpose()
                .expect("fixture overlay parses"),
            "{context} overlay"
        );
    }

    fn distance_from_colony(tile: &WorldTileData, colony: &TileCoordsFixture) -> f64 {
        distance_to(tile.x, tile.y, colony.x, colony.y)
    }

    fn type_counts(tiles: &[WorldTileData]) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for tile in tiles {
            *counts
                .entry(tile.tile_type.as_str().to_owned())
                .or_insert(0) += 1;
        }
        counts
    }

    #[test]
    fn fixture_counts_match_generated_vectors() {
        let fixture = fixture();

        assert_eq!(fixture.counts.tile_cases, fixture.tile_cases.len());
        assert_eq!(fixture.counts.chunk_cases, fixture.chunk_cases.len());
        assert_eq!(fixture.counts.colony_by_seed, fixture.colony_by_seed.len());
        assert_eq!(
            fixture.counts.total,
            fixture.tile_cases.len() + fixture.chunk_cases.len() + fixture.colony_by_seed.len()
        );
    }

    #[test]
    fn public_constants_match_ts() {
        let fixture = fixture();

        assert_eq!(CHUNK_SIZE, fixture.constants.chunk_size);
        assert_eq!(COLONY_SAFE_RADIUS, fixture.constants.colony_safe_radius);
        assert_eq!(COLONY_WATER_RADIUS, fixture.constants.colony_water_radius);
    }

    #[test]
    fn tile_to_chunk_matches_ts_vectors() {
        for case in fixture().tile_cases {
            assert_eq!(
                tile_to_chunk(case.x, case.y),
                ChunkCoords {
                    chunk_x: case.chunk.chunk_x,
                    chunk_y: case.chunk.chunk_y,
                },
                "tile ({}, {})",
                case.x,
                case.y
            );
        }
    }

    #[test]
    fn chunk_to_tile_matches_ts_vectors() {
        for case in fixture().chunk_cases {
            assert_eq!(
                chunk_to_tile(case.chunk_x, case.chunk_y),
                TileCoords {
                    x: case.tile.x,
                    y: case.tile.y,
                },
                "chunk ({}, {})",
                case.chunk_x,
                case.chunk_y
            );
        }
    }

    #[test]
    fn colony_position_matches_ts_vectors() {
        for case in fixture().colony_by_seed {
            assert_eq!(
                get_colony_position(),
                TileCoords {
                    x: case.position.x,
                    y: case.position.y,
                },
                "seed {}",
                case.seed
            );
        }
    }

    #[test]
    fn coordinate_helpers_are_deterministic() {
        assert_eq!(tile_to_chunk(-37, 25), tile_to_chunk(-37, 25));
        assert_eq!(chunk_to_tile(-4, 3), chunk_to_tile(-4, 3));
        assert_eq!(get_colony_position(), get_colony_position());
    }

    #[test]
    fn tile_to_chunk_round_trips_to_containing_tile_origin() {
        for case in fixture().tile_cases {
            let chunk = tile_to_chunk(case.x, case.y);
            let origin = chunk_to_tile(chunk.chunk_x, chunk.chunk_y);

            assert!(
                case.x >= origin.x && case.x < origin.x + CHUNK_SIZE,
                "x {} should be inside chunk origin {}",
                case.x,
                origin.x
            );
            assert!(
                case.y >= origin.y && case.y < origin.y + CHUNK_SIZE,
                "y {} should be inside chunk origin {}",
                case.y,
                origin.y
            );
        }
    }

    #[test]
    fn world_chunk_fixture_counts_match_generated_vectors() {
        let fixture = world_chunk_fixture();

        assert_eq!(fixture.counts.cases, fixture.cases.len());
        assert_eq!(
            fixture.counts.tiles,
            fixture
                .cases
                .iter()
                .map(|case| case.tiles.len())
                .sum::<usize>()
        );
        assert_eq!(fixture.constants.chunk_size, CHUNK_SIZE);
        assert_eq!(fixture.constants.colony_safe_radius, COLONY_SAFE_RADIUS);
        assert_eq!(fixture.constants.colony_water_radius, COLONY_WATER_RADIUS);
    }

    #[test]
    fn generate_world_chunk_matches_ts_fixture_tiles() {
        let fixture = world_chunk_fixture();

        for case in &fixture.cases {
            let actual = generate_world_chunk(
                case.chunk_x,
                case.chunk_y,
                case.seed,
                fixture.colony.x,
                fixture.colony.y,
            );
            assert_eq!(
                actual.len(),
                case.count,
                "seed {} chunk ({}, {}) count",
                case.seed,
                case.chunk_x,
                case.chunk_y
            );

            for (index, (actual, expected)) in actual.iter().zip(&case.tiles).enumerate() {
                assert_tile_matches(
                    actual,
                    expected,
                    &format!(
                        "seed {} chunk ({}, {}) tile {index}",
                        case.seed, case.chunk_x, case.chunk_y
                    ),
                );
            }
        }
    }

    #[test]
    fn generate_world_chunk_summaries_match_ts_fixture() {
        let fixture = world_chunk_fixture();

        for case in &fixture.cases {
            let actual = generate_world_chunk(
                case.chunk_x,
                case.chunk_y,
                case.seed,
                fixture.colony.x,
                fixture.colony.y,
            );
            assert_eq!(
                type_counts(&actual),
                case.type_counts,
                "seed {} chunk ({}, {}) type counts",
                case.seed,
                case.chunk_x,
                case.chunk_y
            );

            let water_near_colony = actual
                .iter()
                .filter(|tile| {
                    tile.resources.water > 0
                        && distance_from_colony(tile, &fixture.colony) <= COLONY_WATER_RADIUS
                })
                .count();
            let safe_rivers = actual
                .iter()
                .filter(|tile| {
                    tile.overlay_feature == Some(OverlayFeature::River)
                        && distance_from_colony(tile, &fixture.colony) <= COLONY_SAFE_RADIUS
                })
                .count();

            assert_eq!(
                water_near_colony, case.water_near_colony,
                "seed {} chunk ({}, {}) reachable water",
                case.seed, case.chunk_x, case.chunk_y
            );
            assert_eq!(
                safe_rivers, case.safe_rivers,
                "seed {} chunk ({}, {}) safe rivers",
                case.seed, case.chunk_x, case.chunk_y
            );
        }
    }

    #[test]
    fn generate_world_chunk_is_deterministic() {
        for case in world_chunk_fixture().cases {
            let first = generate_world_chunk(case.chunk_x, case.chunk_y, case.seed, 6, 6);
            let second = generate_world_chunk(case.chunk_x, case.chunk_y, case.seed, 6, 6);

            assert_eq!(
                first, second,
                "seed {} chunk ({}, {})",
                case.seed, case.chunk_x, case.chunk_y
            );
        }
    }

    #[test]
    fn fine_biomes_have_distinct_finite_natural_deposits_without_coarse_leakage() {
        for biome in Biome::ALL {
            let actual = natural_deposits_for_biome(*biome);
            let expected = match biome {
                Biome::Mountains => (2, 0, 0),
                Biome::Badlands | Biome::Swamp | Biome::Marsh => (0, 12, 0),
                Biome::Beach | Biome::Desert => (0, 0, 16),
                _ => (0, 0, 0),
            };
            assert_eq!(
                actual,
                expected,
                "{} deposit matrix",
                biome.properties().wire
            );
        }
    }
}
