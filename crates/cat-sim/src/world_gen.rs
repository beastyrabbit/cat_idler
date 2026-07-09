//! World coordinate helpers ported from `lib/game/worldGen.ts`.

pub const CHUNK_SIZE: i32 = 12;
pub const COLONY_SAFE_RADIUS: f64 = 3.5;
pub const COLONY_WATER_RADIUS: f64 = 5.5;

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

#[cfg(test)]
mod tests {
    use super::*;
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

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../docs/migration/fixtures/p2/world_coords.json"
        ))
        .expect("world coordinate fixture deserializes")
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
}
