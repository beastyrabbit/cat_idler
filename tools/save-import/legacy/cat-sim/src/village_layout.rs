//! Village layout helpers ported from `lib/game/villageLayout.ts`.

use std::collections::HashSet;

use crate::rng::roll_seeded;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
}

/// World tile coordinate of the village center.
pub const VILLAGE_ANCHOR: GridPos = GridPos { x: 6, y: 6 };

/// Colony-local position of the shrine.
pub const SHRINE_LOCAL: GridPos = GridPos { x: 0, y: 0 };

/// Default maximum ring the village can spiral out to.
pub const DEFAULT_MAX_RING: i32 = 8;

/// Smallest fence-ring radius.
pub const VILLAGE_MIN_RING: i32 = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeededBuildingSite {
    pub site: Option<GridPos>,
    pub next_seed: u32,
}

#[must_use]
pub const fn colony_to_world(local: GridPos) -> GridPos {
    GridPos {
        x: VILLAGE_ANCHOR.x + local.x,
        y: VILLAGE_ANCHOR.y + local.y,
    }
}

#[must_use]
pub const fn world_to_colony(world: GridPos) -> GridPos {
    GridPos {
        x: world.x - VILLAGE_ANCHOR.x,
        y: world.y - VILLAGE_ANCHOR.y,
    }
}

#[must_use]
pub const fn shrine_world_position() -> GridPos {
    colony_to_world(SHRINE_LOCAL)
}

/// Cells at Chebyshev distance `ring` from the shrine, in TypeScript order.
#[must_use]
pub fn ring_cells(ring: i32) -> Vec<GridPos> {
    if ring <= 0 {
        return vec![SHRINE_LOCAL];
    }

    let side_len = usize::try_from(ring).expect("positive ring converts to usize");
    let mut cells = Vec::with_capacity(8 * side_len);

    for x in -ring..=ring {
        cells.push(GridPos { x, y: -ring });
    }
    for y in (-ring + 1)..=(ring - 1) {
        cells.push(GridPos { x: -ring, y });
        cells.push(GridPos { x: ring, y });
    }
    for x in -ring..=ring {
        cells.push(GridPos { x, y: ring });
    }

    cells
}

#[must_use]
pub fn next_building_site(occupied: &[GridPos], roll: f64, max_ring: i32) -> Option<GridPos> {
    next_building_site_with_blocked(occupied, roll, max_ring, |_| false)
}

#[must_use]
pub fn next_building_site_default(occupied: &[GridPos], roll: f64) -> Option<GridPos> {
    next_building_site(occupied, roll, DEFAULT_MAX_RING)
}

#[must_use]
pub fn next_building_site_with_blocked<F>(
    occupied: &[GridPos],
    roll: f64,
    max_ring: i32,
    is_blocked: F,
) -> Option<GridPos>
where
    F: Fn(GridPos) -> bool,
{
    let mut taken = occupied.iter().copied().collect::<HashSet<_>>();
    taken.insert(SHRINE_LOCAL);

    for ring in 1..=max_ring {
        let free = ring_cells(ring)
            .into_iter()
            .filter(|cell| !taken.contains(cell) && !is_blocked(*cell))
            .collect::<Vec<_>>();

        if free.is_empty() {
            continue;
        }

        let clamped = roll.clamp(0.0, 0.999_999);
        let index = (clamped * free.len() as f64).floor() as usize;
        return Some(free[index]);
    }

    None
}

#[must_use]
pub fn next_building_site_seeded(occupied: &[GridPos], seed: u32) -> SeededBuildingSite {
    next_building_site_seeded_with_blocked(occupied, seed, DEFAULT_MAX_RING, |_| false)
}

#[must_use]
pub fn next_building_site_seeded_with_blocked<F>(
    occupied: &[GridPos],
    seed: u32,
    max_ring: i32,
    is_blocked: F,
) -> SeededBuildingSite
where
    F: Fn(GridPos) -> bool,
{
    let roll = roll_seeded(f64::from(seed));

    SeededBuildingSite {
        site: next_building_site_with_blocked(occupied, roll.value, max_ring, is_blocked),
        next_seed: roll.next_seed,
    }
}

/// Radius in building rings occupied by `building_count`.
#[must_use]
pub fn village_radius(building_count: i32) -> i32 {
    let mut radius = 1;
    let mut capacity = 8;

    while building_count > capacity {
        radius += 1;
        capacity += 8 * radius;
    }

    radius
}

/// Radius of the fence/clearing ring that encloses the village.
#[must_use]
pub fn village_ring_radius(building_count: i32) -> i32 {
    VILLAGE_MIN_RING.max(village_radius(building_count) + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinate_mapping_matches_ts_anchor() {
        assert_eq!(colony_to_world(SHRINE_LOCAL), VILLAGE_ANCHOR);
        assert_eq!(
            colony_to_world(GridPos { x: 2, y: -1 }),
            GridPos { x: 8, y: 5 }
        );
        assert_eq!(shrine_world_position(), VILLAGE_ANCHOR);

        for point in [
            GridPos { x: 0, y: 0 },
            GridPos { x: 3, y: 5 },
            GridPos { x: -4, y: 2 },
            GridPos { x: -7, y: -7 },
        ] {
            assert_eq!(world_to_colony(colony_to_world(point)), point);
        }
    }

    #[test]
    fn ring_cells_matches_ts_order() {
        assert_eq!(ring_cells(0), vec![GridPos { x: 0, y: 0 }]);
        assert_eq!(ring_cells(-1), vec![GridPos { x: 0, y: 0 }]);

        assert_eq!(
            ring_cells(1),
            vec![
                GridPos { x: -1, y: -1 },
                GridPos { x: 0, y: -1 },
                GridPos { x: 1, y: -1 },
                GridPos { x: -1, y: 0 },
                GridPos { x: 1, y: 0 },
                GridPos { x: -1, y: 1 },
                GridPos { x: 0, y: 1 },
                GridPos { x: 1, y: 1 },
            ]
        );

        assert_eq!(ring_cells(2).len(), 16);
        assert_eq!(ring_cells(3).len(), 24);
        assert_eq!(ring_cells(5).len(), 40);
    }

    #[test]
    fn next_building_site_uses_roll_and_skips_unavailable_cells() {
        assert_eq!(
            next_building_site_default(&[], 0.0),
            Some(GridPos { x: -1, y: -1 })
        );
        assert_eq!(
            next_building_site_default(&[], 0.999),
            Some(GridPos { x: 1, y: 1 })
        );
        assert_eq!(
            next_building_site_default(&[], 1.0),
            Some(GridPos { x: 1, y: 1 })
        );
        assert_eq!(
            next_building_site_default(&[], -0.1),
            Some(GridPos { x: -1, y: -1 })
        );

        let occupied = &ring_cells(1)[..7];
        assert_eq!(
            next_building_site_default(occupied, 0.9),
            Some(GridPos { x: 1, y: 1 })
        );

        let occupied = ring_cells(1);
        assert_eq!(
            next_building_site_default(&occupied, 0.0),
            Some(GridPos { x: -2, y: -2 })
        );

        let occupied = [ring_cells(1), ring_cells(2)].concat();
        assert_eq!(next_building_site(&occupied, 0.5, 2), None);
    }

    #[test]
    fn next_building_site_obeys_blocked_predicate() {
        let free = ring_cells(1)[3];
        let site = next_building_site_with_blocked(&[], 0.5, DEFAULT_MAX_RING, |cell| cell != free);
        assert_eq!(site, Some(free));

        let blocked_ring_1 = ring_cells(1);
        let site = next_building_site_with_blocked(&[], 0.5, DEFAULT_MAX_RING, |cell| {
            blocked_ring_1.contains(&cell)
        });
        assert_eq!(site, Some(GridPos { x: 2, y: 0 }));
    }

    #[test]
    fn seeded_next_building_site_uses_seeded_rng_roll() {
        let result = next_building_site_seeded(&[], 123);
        assert_eq!(result.next_seed, 1_218_640_798);
        assert_eq!(result.site, Some(GridPos { x: 1, y: -1 }));

        let blocked = ring_cells(1);
        let result = next_building_site_seeded_with_blocked(&[], 123, DEFAULT_MAX_RING, |cell| {
            blocked.contains(&cell)
        });
        assert_eq!(result.next_seed, 1_218_640_798);
        assert_eq!(result.site, Some(GridPos { x: 2, y: -2 }));
    }

    #[test]
    fn village_radius_matches_ts_capacity_boundaries() {
        for (building_count, radius) in [
            (0, 1),
            (1, 1),
            (8, 1),
            (9, 2),
            (24, 2),
            (25, 3),
            (48, 3),
            (49, 4),
            (80, 4),
            (81, 5),
        ] {
            assert_eq!(village_radius(building_count), radius);
        }
    }

    #[test]
    fn village_ring_radius_is_outer_building_ring_plus_one_with_minimum() {
        for (building_count, radius) in [(0, 4), (6, 4), (48, 4), (49, 5), (80, 5), (81, 6)] {
            assert_eq!(village_ring_radius(building_count), radius);
            assert!(village_ring_radius(building_count) > village_radius(building_count));
        }
    }
}
