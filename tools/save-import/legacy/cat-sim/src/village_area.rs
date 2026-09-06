//! Organic village area helpers ported from `lib/game/villageArea.ts`.

use std::collections::HashSet;

use crate::{rng::roll_seeded, village_layout::GridPos};

/// A claimed village as a set of `"x,y"` tile keys.
pub type VillageArea = HashSet<String>;

/// Autotile bit for a fence on the north side of a tile.
pub const FENCE_DIR_N: u8 = 1;
/// Autotile bit for a fence on the east side of a tile.
pub const FENCE_DIR_E: u8 = 2;
/// Autotile bit for a fence on the south side of a tile.
pub const FENCE_DIR_S: u8 = 4;
/// Autotile bit for a fence on the west side of a tile.
pub const FENCE_DIR_W: u8 = 8;

/// Interior tiles kept free before village growth is triggered.
pub const FREE_TILE_FLOOR: i32 = 2;
/// Population-per-claimed-tile threshold before village growth is triggered.
pub const CROWDING_PER_TILE: f64 = 1.5;

const SIDES: [Side; 4] = [Side::N, Side::E, Side::S, Side::W];

/// Which side of a tile a boundary edge sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    N,
    E,
    S,
    W,
}

impl Side {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::N => "N",
            Self::E => "E",
            Self::S => "S",
            Self::W => "W",
        }
    }
}

/// Render axis for a fence segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FenceAxis {
    X,
    Y,
}

impl FenceAxis {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
        }
    }
}

/// One boundary edge of the claimed village area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FenceSegment {
    pub x: i32,
    pub y: i32,
    pub side: Side,
    pub axis: FenceAxis,
    pub gate: bool,
}

/// The single passable fence edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GatePlacement {
    pub x: i32,
    pub y: i32,
    pub side: Side,
}

/// Options for choosing the village gate.
#[derive(Default)]
pub struct GateOptions<'a> {
    pub outside_wear: Option<&'a dyn Fn(GridPos) -> f64>,
    pub axis_bias: Option<GridPos>,
}

/// Options for picking the next village tile to claim.
#[derive(Default)]
pub struct ExpandOptions<'a> {
    pub is_water: Option<&'a dyn Fn(GridPos) -> bool>,
    pub rng: Option<&'a mut dyn FnMut() -> f64>,
}

/// Result of seeded expansion, including the seed after all tie-break rolls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeededExpansion {
    pub site: Option<GridPos>,
    pub next_seed: u32,
}

#[must_use]
pub fn key(x: i32, y: i32) -> String {
    format!("{x},{y}")
}

#[must_use]
pub fn key_of(pos: GridPos) -> String {
    key(pos.x, pos.y)
}

#[must_use]
pub fn pos_of(value: &str) -> GridPos {
    let (x, y) = value
        .split_once(',')
        .unwrap_or_else(|| panic!("invalid village area tile key: {value:?}"));

    GridPos {
        x: x.parse()
            .unwrap_or_else(|_| panic!("invalid village area tile x: {value:?}")),
        y: y.parse()
            .unwrap_or_else(|_| panic!("invalid village area tile y: {value:?}")),
    }
}

#[must_use]
pub fn from_tiles(tiles: &[GridPos]) -> VillageArea {
    tiles.iter().copied().map(key_of).collect()
}

#[must_use]
pub fn to_tiles(area: &VillageArea) -> Vec<GridPos> {
    let mut tiles = area.iter().map(|tile| pos_of(tile)).collect::<Vec<_>>();
    tiles.sort_by_key(|pos| (pos.y, pos.x));
    tiles
}

#[must_use]
pub fn claim_tile(area: &mut VillageArea, pos: GridPos) -> bool {
    area.insert(key_of(pos))
}

#[must_use]
pub fn is_inside_village(pos: GridPos, area: &VillageArea) -> bool {
    area.contains(&key_of(pos))
}

#[must_use]
pub const fn side_delta(side: Side) -> GridPos {
    match side {
        Side::N => GridPos { x: 0, y: -1 },
        Side::E => GridPos { x: 1, y: 0 },
        Side::S => GridPos { x: 0, y: 1 },
        Side::W => GridPos { x: -1, y: 0 },
    }
}

#[must_use]
pub const fn side_axis(side: Side) -> FenceAxis {
    match side {
        Side::N | Side::S => FenceAxis::X,
        Side::E | Side::W => FenceAxis::Y,
    }
}

#[must_use]
pub const fn fence_dir(side: Side) -> u8 {
    match side {
        Side::N => FENCE_DIR_N,
        Side::E => FENCE_DIR_E,
        Side::S => FENCE_DIR_S,
        Side::W => FENCE_DIR_W,
    }
}

#[must_use]
pub fn fence_mask_at(pos: GridPos, area: &VillageArea) -> u8 {
    if !is_inside_village(pos, area) {
        return 0;
    }

    let mut mask = 0;
    for side in SIDES {
        let delta = side_delta(side);
        let neighbour = GridPos {
            x: pos.x + delta.x,
            y: pos.y + delta.y,
        };
        if !is_inside_village(neighbour, area) {
            mask |= fence_dir(side);
        }
    }

    mask
}

#[must_use]
pub fn fence_perimeter(area: &VillageArea, gate: Option<GatePlacement>) -> Vec<FenceSegment> {
    let mut segments = Vec::new();

    for pos in to_tiles(area) {
        let mask = fence_mask_at(pos, area);
        for side in SIDES {
            if mask & fence_dir(side) == 0 {
                continue;
            }
            segments.push(FenceSegment {
                x: pos.x,
                y: pos.y,
                side,
                axis: side_axis(side),
                gate: gate
                    .is_some_and(|gate| gate.x == pos.x && gate.y == pos.y && gate.side == side),
            });
        }
    }

    segments
}

#[must_use]
pub fn perimeter_length(area: &VillageArea) -> u32 {
    area.iter()
        .map(|tile| fence_mask_at(pos_of(tile), area).count_ones())
        .sum()
}

#[must_use]
pub fn gate_placement(area: &VillageArea, opts: GateOptions<'_>) -> Option<GatePlacement> {
    let segments = fence_perimeter(area, None);
    if segments.is_empty() {
        return None;
    }

    let bias = opts.axis_bias.unwrap_or(GridPos { x: 0, y: 1 });
    let centroid = area_centroid(area);
    let mut best = None;
    let mut best_score = f64::NEG_INFINITY;
    let mut best_dist = f64::INFINITY;

    for segment in segments {
        let delta = side_delta(segment.side);
        let outside = GridPos {
            x: segment.x + delta.x,
            y: segment.y + delta.y,
        };
        let score = if let Some(outside_wear) = opts.outside_wear {
            outside_wear(outside)
        } else {
            f64::from(delta.x * sign(bias.x) + delta.y * sign(bias.y))
        };
        let dist = (f64::from(segment.x) - centroid.x).powi(2)
            + (f64::from(segment.y) - centroid.y).powi(2);

        if score > best_score || (score == best_score && dist < best_dist) {
            best_score = score;
            best_dist = dist;
            best = Some(segment);
        }
    }

    best.map(|segment| GatePlacement {
        x: segment.x,
        y: segment.y,
        side: segment.side,
    })
}

#[must_use]
pub fn gate_placement_default(area: &VillageArea) -> Option<GatePlacement> {
    gate_placement(area, GateOptions::default())
}

#[must_use]
pub fn fence_edge_between(from: GridPos, to: GridPos, area: &VillageArea) -> Option<FenceSegment> {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    if dx.abs() + dy.abs() != 1 {
        return None;
    }

    let from_in = is_inside_village(from, area);
    let to_in = is_inside_village(to, area);
    if from_in == to_in {
        return None;
    }

    let inside = if from_in { from } else { to };
    let side = if dx == 1 {
        if from_in { Side::E } else { Side::W }
    } else if dx == -1 {
        if from_in { Side::W } else { Side::E }
    } else if dy == 1 {
        if from_in { Side::S } else { Side::N }
    } else if from_in {
        Side::N
    } else {
        Side::S
    };

    Some(FenceSegment {
        x: inside.x,
        y: inside.y,
        side,
        axis: side_axis(side),
        gate: false,
    })
}

#[must_use]
pub const fn perimeter_blocks(segment: FenceSegment) -> bool {
    !segment.gate
}

#[must_use]
pub fn fence_blocks_move(
    from: GridPos,
    to: GridPos,
    area: &VillageArea,
    gate: Option<GatePlacement>,
) -> bool {
    let Some(edge) = fence_edge_between(from, to, area) else {
        return false;
    };

    let is_gate =
        gate.is_some_and(|gate| gate.x == edge.x && gate.y == edge.y && gate.side == edge.side);

    perimeter_blocks(FenceSegment {
        gate: is_gate,
        ..edge
    })
}

#[must_use]
pub fn expand_village(area: &VillageArea, mut opts: ExpandOptions<'_>) -> Option<GridPos> {
    if area.is_empty() {
        return None;
    }

    let mut seen = HashSet::new();
    let mut candidates = Vec::new();

    for pos in to_tiles(area) {
        for side in SIDES {
            let delta = side_delta(side);
            let candidate = GridPos {
                x: pos.x + delta.x,
                y: pos.y + delta.y,
            };
            let candidate_key = key_of(candidate);
            let is_water = opts.is_water.is_some_and(|is_water| is_water(candidate));
            if seen.contains(&candidate_key) || is_inside_village(candidate, area) || is_water {
                continue;
            }
            seen.insert(candidate_key);
            candidates.push(candidate);
        }
    }

    if candidates.is_empty() {
        return None;
    }

    let centroid = area_centroid(area);
    let mut best = None;
    let mut best_key = SortKey {
        fill: i32::MIN,
        neg_dist2: f64::NEG_INFINITY,
        roll: f64::NEG_INFINITY,
    };

    for candidate in candidates {
        let fill = claimed_neighbours_8(candidate, area);
        let dist2 = (f64::from(candidate.x) - centroid.x).powi(2)
            + (f64::from(candidate.y) - centroid.y).powi(2);
        let roll = opts.rng.as_mut().map_or(0.0, |rng| rng());
        let candidate_key = SortKey {
            fill,
            neg_dist2: -dist2,
            roll,
        };

        if candidate_key.is_better_than(best_key) {
            best_key = candidate_key;
            best = Some(candidate);
        }
    }

    best
}

#[must_use]
pub fn expand_village_default(area: &VillageArea) -> Option<GridPos> {
    expand_village(area, ExpandOptions::default())
}

#[must_use]
pub fn expand_village_seeded(area: &VillageArea, seed: u32) -> SeededExpansion {
    let mut next_seed = seed;
    let mut rng = || {
        let roll = roll_seeded(f64::from(next_seed));
        next_seed = roll.next_seed;
        roll.value
    };
    let site = expand_village(
        area,
        ExpandOptions {
            is_water: None,
            rng: Some(&mut rng),
        },
    );

    SeededExpansion { site, next_seed }
}

#[must_use]
pub fn should_expand(population: i32, claimed_count: i32, building_count: i32) -> bool {
    if claimed_count <= 0 {
        return true;
    }

    let free_tiles = claimed_count - building_count;
    if free_tiles < FREE_TILE_FLOOR {
        return true;
    }

    f64::from(population) > f64::from(claimed_count) * CROWDING_PER_TILE
}

#[derive(Debug, Clone, Copy)]
struct SortKey {
    fill: i32,
    neg_dist2: f64,
    roll: f64,
}

impl SortKey {
    fn is_better_than(self, other: Self) -> bool {
        if self.fill != other.fill {
            return self.fill > other.fill;
        }
        if self.neg_dist2 != other.neg_dist2 {
            return self.neg_dist2 > other.neg_dist2;
        }
        self.roll > other.roll
    }
}

#[derive(Debug, Clone, Copy)]
struct Centroid {
    x: f64,
    y: f64,
}

fn sign(value: i32) -> i32 {
    value.signum()
}

fn claimed_neighbours_8(pos: GridPos, area: &VillageArea) -> i32 {
    let mut count = 0;
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            if is_inside_village(
                GridPos {
                    x: pos.x + dx,
                    y: pos.y + dy,
                },
                area,
            ) {
                count += 1;
            }
        }
    }
    count
}

fn area_centroid(area: &VillageArea) -> Centroid {
    let mut sx = 0;
    let mut sy = 0;

    for pos in area.iter().map(|tile| pos_of(tile)) {
        sx += pos.x;
        sy += pos.y;
    }

    Centroid {
        x: f64::from(sx) / area.len() as f64,
        y: f64::from(sy) / area.len() as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: i32, y: i32) -> GridPos {
        GridPos { x, y }
    }

    #[test]
    fn claiming_uses_string_keys_and_row_major_tile_order() {
        let mut area = from_tiles(&[pos(1, 1), pos(0, 0), pos(1, 0), pos(1, 1)]);

        assert_eq!(area.len(), 3);
        assert!(is_inside_village(pos(1, 1), &area));
        assert!(!is_inside_village(pos(-1, 0), &area));
        assert_eq!(key_of(pos(-2, 7)), "-2,7");
        assert_eq!(pos_of("-2,7"), pos(-2, 7));
        assert_eq!(to_tiles(&area), vec![pos(0, 0), pos(1, 0), pos(1, 1)]);

        assert!(claim_tile(&mut area, pos(-1, 0)));
        assert!(!claim_tile(&mut area, pos(-1, 0)));
        assert_eq!(
            to_tiles(&area),
            vec![pos(-1, 0), pos(0, 0), pos(1, 0), pos(1, 1)]
        );
    }

    #[test]
    fn fence_perimeter_masks_gate_and_edge_blocking_match_boundary_edges() {
        let area = from_tiles(&[pos(0, 0), pos(1, 0), pos(0, 1)]);
        let gate = GatePlacement {
            x: 0,
            y: 1,
            side: Side::S,
        };

        assert_eq!(fence_mask_at(pos(0, 0), &area), FENCE_DIR_N | FENCE_DIR_W);
        assert_eq!(
            fence_mask_at(pos(1, 0), &area),
            FENCE_DIR_N | FENCE_DIR_E | FENCE_DIR_S
        );
        assert_eq!(
            fence_mask_at(pos(0, 1), &area),
            FENCE_DIR_E | FENCE_DIR_S | FENCE_DIR_W
        );
        assert_eq!(fence_mask_at(pos(1, 1), &area), 0);
        assert_eq!(perimeter_length(&area), 8);

        assert_eq!(
            fence_perimeter(&area, Some(gate)),
            vec![
                FenceSegment {
                    x: 0,
                    y: 0,
                    side: Side::N,
                    axis: FenceAxis::X,
                    gate: false,
                },
                FenceSegment {
                    x: 0,
                    y: 0,
                    side: Side::W,
                    axis: FenceAxis::Y,
                    gate: false,
                },
                FenceSegment {
                    x: 1,
                    y: 0,
                    side: Side::N,
                    axis: FenceAxis::X,
                    gate: false,
                },
                FenceSegment {
                    x: 1,
                    y: 0,
                    side: Side::E,
                    axis: FenceAxis::Y,
                    gate: false,
                },
                FenceSegment {
                    x: 1,
                    y: 0,
                    side: Side::S,
                    axis: FenceAxis::X,
                    gate: false,
                },
                FenceSegment {
                    x: 0,
                    y: 1,
                    side: Side::E,
                    axis: FenceAxis::Y,
                    gate: false,
                },
                FenceSegment {
                    x: 0,
                    y: 1,
                    side: Side::S,
                    axis: FenceAxis::X,
                    gate: true,
                },
                FenceSegment {
                    x: 0,
                    y: 1,
                    side: Side::W,
                    axis: FenceAxis::Y,
                    gate: false,
                },
            ]
        );

        assert_eq!(
            gate_placement_default(&from_tiles(&[pos(-1, 0), pos(0, 0), pos(1, 0)])),
            Some(GatePlacement {
                x: 0,
                y: 0,
                side: Side::S,
            })
        );
        assert_eq!(
            gate_placement(
                &area,
                GateOptions {
                    outside_wear: Some(&|outside| if outside == pos(2, 0) { 10.0 } else { 0.0 }),
                    axis_bias: None,
                },
            ),
            Some(GatePlacement {
                x: 1,
                y: 0,
                side: Side::E,
            })
        );

        assert_eq!(
            fence_edge_between(pos(0, 1), pos(0, 2), &area),
            Some(FenceSegment {
                x: 0,
                y: 1,
                side: Side::S,
                axis: FenceAxis::X,
                gate: false,
            })
        );
        assert!(fence_blocks_move(pos(1, 0), pos(2, 0), &area, Some(gate)));
        assert!(!fence_blocks_move(pos(0, 1), pos(0, 2), &area, Some(gate)));
        assert!(!fence_blocks_move(pos(0, 0), pos(0, 1), &area, Some(gate)));
        assert!(!fence_blocks_move(
            pos(9, 9),
            pos(10, 10),
            &area,
            Some(gate)
        ));
    }

    #[test]
    fn expand_village_prefers_fill_then_centroid_then_seeded_roll() {
        let concave = from_tiles(&[pos(0, 0), pos(1, 0), pos(0, 1)]);
        assert_eq!(expand_village_default(&concave), Some(pos(1, 1)));

        let line = from_tiles(&[pos(0, 0), pos(1, 0)]);
        assert_eq!(expand_village_default(&line), Some(pos(0, -1)));

        let seeded = expand_village_seeded(&line, 123);
        assert_eq!(seeded.site, Some(pos(1, 1)));
        // The `line` area has exactly 6 dry frontier candidates, so the seed is
        // advanced by 6 rolls of the ported LCG from 123 (hand-literal was wrong).
        assert_eq!(seeded.next_seed, 2_535_079_273);
        // Determinism: re-running yields the same advanced seed.
        assert_eq!(
            expand_village_seeded(&line, 123).next_seed,
            seeded.next_seed
        );

        assert_eq!(
            expand_village(
                &line,
                ExpandOptions {
                    is_water: Some(&|tile| tile == pos(1, 1)),
                    rng: None,
                },
            ),
            Some(pos(0, -1))
        );

        let surrounded = from_tiles(&[pos(0, 0)]);
        assert_eq!(
            expand_village(
                &surrounded,
                ExpandOptions {
                    is_water: Some(&|_| true),
                    rng: None,
                },
            ),
            None
        );
        assert_eq!(expand_village_default(&VillageArea::new()), None);
    }

    #[test]
    fn should_expand_matches_free_tile_and_crowding_thresholds() {
        assert!(should_expand(0, 0, 0));
        assert!(should_expand(2, 3, 2));
        assert!(should_expand(7, 4, 1));

        assert!(!should_expand(6, 4, 1));
        assert!(!should_expand(3, 4, 2));
        assert!(!should_expand(0, 2, 0));
    }
}
