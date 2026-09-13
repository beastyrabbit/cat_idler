//! Deliberate road building — pure selection of the corridor worth paving.
//!
//! Ported from `lib/game/roads.ts`. Chooses the highest cumulative-wear
//! 4-connected run of trodden trail tiles outside the village fence; the tick
//! does the actual paving. Deterministic: ties break by a stable coordinate key.
//!
//! Also home to the P14.4 road-accessibility router (`road_route_to_network`):
//! given a building's candidate entrance tiles, find the shortest fresh spur
//! that connects one of them to the existing road network (which always
//! includes the shrine's own footprint — the walk-through hub every road
//! radiates from). Pure and generic over caller-supplied `is_blocked`/
//! `is_network` predicates so `world_tick` can wire it to live colony state
//! while keeping this module free of `ColonyRuntime`.

use std::collections::{HashMap, HashSet, VecDeque};

/// Wear at/above which a tile counts as a trafficked, pave-worthy trail.
pub const ROAD_PAVE_WEAR: f64 = 70.0;

/// A tile the road planner considers. `is_paved` mirrors the TS
/// `overlayFeature === "road_built"` check.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoadTile {
    pub x: i32,
    pub y: i32,
    pub path_wear: f64,
    pub is_paved: bool,
}

/// A tile position (paving order output).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoadPos {
    pub x: i32,
    pub y: i32,
}

/// Options for [`select_road_corridor`].
#[derive(Debug, Clone, Copy)]
pub struct RoadCorridorOptions {
    /// Village centre — only ground outside the fence ring is paved.
    pub anchor_x: i32,
    pub anchor_y: i32,
    /// Chebyshev radius of the fence; interior is already open clearing.
    pub ring_radius: i32,
    /// Most tiles to pave in one go (also bounded by available materials).
    pub max_tiles: i32,
    /// Minimum wear a tile needs before it is worth paving (defaults to
    /// [`ROAD_PAVE_WEAR`] when `None`).
    pub wear_threshold: Option<f64>,
}

fn cheb(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs().max((ay - by).abs())
}

/// The most-trafficked unpaved corridor worth paving right now: from the single
/// highest-wear trail tile outside the fence, greedily grow a 4-connected run
/// through the next-highest-wear neighbours until the tile budget is spent or the
/// trail peters out. Returns the corridor's tiles (paving order, highest-wear
/// first) or an empty list when nothing clears the threshold.
#[must_use]
pub fn select_road_corridor(tiles: &[RoadTile], options: RoadCorridorOptions) -> Vec<RoadPos> {
    let threshold = options.wear_threshold.unwrap_or(ROAD_PAVE_WEAR);
    if options.max_tiles <= 0 {
        return Vec::new();
    }

    // Deduplicate by coordinate (last wins, matching the TS Map insertion) while
    // keeping only unpaved, worn, outside-the-fence candidates.
    let mut candidates: Vec<RoadTile> = Vec::new();
    for tile in tiles {
        if !tile.is_paved
            && tile.path_wear >= threshold
            && cheb(tile.x, tile.y, options.anchor_x, options.anchor_y) > options.ring_radius
        {
            if let Some(existing) = candidates
                .iter_mut()
                .find(|c| c.x == tile.x && c.y == tile.y)
            {
                *existing = *tile;
            } else {
                candidates.push(*tile);
            }
        }
    }
    if candidates.is_empty() {
        return Vec::new();
    }

    // Stable ordering: wear desc, then x asc, then y asc (reproducible pick).
    candidates.sort_by(|a, b| {
        b.path_wear
            .partial_cmp(&a.path_wear)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.x.cmp(&b.x))
            .then(a.y.cmp(&b.y))
    });

    let mut corridor: Vec<RoadPos> = Vec::new();
    let start = candidates[0];
    corridor.push(RoadPos {
        x: start.x,
        y: start.y,
    });

    while (corridor.len() as i32) < options.max_tiles {
        // Best unused candidate 4-adjacent to any corridor tile. `candidates` is
        // sorted, so the first adjacent one is the best.
        let mut best: Option<RoadPos> = None;
        for tile in &candidates {
            if corridor.iter().any(|c| c.x == tile.x && c.y == tile.y) {
                continue;
            }
            let adjacent = corridor
                .iter()
                .any(|c| (c.x - tile.x).abs() + (c.y - tile.y).abs() == 1);
            if adjacent {
                best = Some(RoadPos {
                    x: tile.x,
                    y: tile.y,
                });
                break;
            }
        }
        match best {
            Some(pos) => corridor.push(pos),
            None => break,
        }
    }

    corridor
}

const ROUTE_NEIGHBOURS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

/// P14.4: the shortest fresh 4-connected spur from a building's entrance to the
/// existing road network, or `None` when no entrance can reach it.
///
/// `entrances` is the building's candidate entrance tiles in the caller's
/// stable order (e.g. `(y, x)` ascending) — the tiles just outside its
/// footprint a connecting road could start from. Determinism comes from that
/// order plus a fixed neighbour-expansion order (N, E, S, W) every BFS step,
/// so identical inputs always retrace the identical route.
///
/// Two predicates drive the search:
/// - `is_network(tile)`: `true` for a tile already part of the road network —
///   an existing paved road, or the shrine's own footprint (always passable,
///   the hub every road leads to). BFS stops the instant it reaches one.
/// - `is_blocked(tile)`: `true` for ground a road may never occupy — another
///   building's footprint, water, a tree, or the perimeter wall (except the
///   gate). `is_network` takes priority over `is_blocked` (the shrine's
///   footprint is technically "occupied" by the shrine building, but the spec
///   calls it out as passable, so routing may step onto/through it).
///
/// Behaviour:
/// - If any entrance is already `is_network`, the building is already
///   connected — returns `Some(&[])` (nothing fresh to pave).
/// - Otherwise BFS starts from the first entrance that isn't `is_blocked`
///   (skipping unusable candidates rather than failing outright) and returns
///   the fresh tiles from that entrance up to — but excluding — the network
///   tile it reaches.
/// - `None` when every entrance is blocked, or the network is unreachable
///   within `max_expansions` steps (bounds the search on a pathological map).
#[must_use]
pub fn road_route_to_network(
    entrances: &[RoadPos],
    is_blocked: impl Fn(RoadPos) -> bool,
    is_network: impl Fn(RoadPos) -> bool,
    max_expansions: usize,
) -> Option<Vec<RoadPos>> {
    if entrances.iter().any(|&entrance| is_network(entrance)) {
        return Some(Vec::new());
    }

    let start = entrances.iter().copied().find(|&e| !is_blocked(e))?;

    let mut visited: HashSet<RoadPos> = HashSet::new();
    visited.insert(start);
    let mut parent: HashMap<RoadPos, RoadPos> = HashMap::new();
    let mut queue: VecDeque<RoadPos> = VecDeque::new();
    queue.push_back(start);
    let mut expansions = 0_usize;

    while let Some(current) = queue.pop_front() {
        if is_network(current) {
            let mut route = Vec::new();
            let mut node = current;
            while node != start {
                let prev = parent[&node];
                route.push(node);
                node = prev;
            }
            route.push(start);
            route.reverse();
            route.pop(); // The network tile itself already exists — nothing to pave there.
            return Some(route);
        }

        expansions += 1;
        if expansions > max_expansions {
            return None;
        }

        for (dx, dy) in ROUTE_NEIGHBOURS {
            let next = RoadPos {
                x: current.x + dx,
                y: current.y + dy,
            };
            if visited.contains(&next) {
                continue;
            }
            if is_blocked(next) && !is_network(next) {
                continue;
            }
            visited.insert(next);
            parent.insert(next, current);
            queue.push_back(next);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tile(x: i32, y: i32, wear: f64) -> RoadTile {
        RoadTile {
            x,
            y,
            path_wear: wear,
            is_paved: false,
        }
    }

    fn opts(ring_radius: i32, max_tiles: i32) -> RoadCorridorOptions {
        RoadCorridorOptions {
            anchor_x: 0,
            anchor_y: 0,
            ring_radius,
            max_tiles,
            wear_threshold: None,
        }
    }

    #[test]
    fn empty_when_no_tile_clears_threshold() {
        let tiles = [tile(5, 0, 69.0), tile(6, 0, 10.0)];
        assert!(select_road_corridor(&tiles, opts(1, 10)).is_empty());
    }

    #[test]
    fn empty_when_max_tiles_non_positive() {
        let tiles = [tile(5, 0, 90.0)];
        assert!(select_road_corridor(&tiles, opts(1, 0)).is_empty());
    }

    #[test]
    fn skips_paved_and_inside_fence_tiles() {
        let mut paved = tile(5, 0, 99.0);
        paved.is_paved = true;
        let inside = tile(1, 0, 99.0); // cheb 1, ring_radius 1 -> not outside
        let outside = tile(5, 0, 80.0);
        let tiles = [paved, inside, outside];
        let corridor = select_road_corridor(&tiles, opts(1, 10));
        assert_eq!(corridor, vec![RoadPos { x: 5, y: 0 }]);
    }

    #[test]
    fn isolated_highest_wear_tile_is_a_single_corridor() {
        let tiles = [
            tile(3, 0, 95.0),
            tile(4, 0, 90.0),
            tile(5, 0, 85.0),
            tile(7, 0, 99.0), // highest wear but NOT adjacent to the run
        ];
        let corridor = select_road_corridor(&tiles, opts(1, 10));
        assert_eq!(corridor, vec![RoadPos { x: 7, y: 0 }]);
    }

    #[test]
    fn walks_the_contiguous_corridor_in_wear_order() {
        let tiles = [
            tile(3, 0, 99.0),
            tile(4, 0, 95.0),
            tile(5, 0, 90.0),
            tile(6, 0, 85.0),
        ];
        let corridor = select_road_corridor(&tiles, opts(1, 10));
        assert_eq!(
            corridor,
            vec![
                RoadPos { x: 3, y: 0 },
                RoadPos { x: 4, y: 0 },
                RoadPos { x: 5, y: 0 },
                RoadPos { x: 6, y: 0 },
            ]
        );
    }

    #[test]
    fn respects_max_tiles_budget() {
        let tiles = [
            tile(3, 0, 99.0),
            tile(4, 0, 95.0),
            tile(5, 0, 90.0),
            tile(6, 0, 85.0),
        ];
        let corridor = select_road_corridor(&tiles, opts(1, 2));
        assert_eq!(
            corridor,
            vec![RoadPos { x: 3, y: 0 }, RoadPos { x: 4, y: 0 }]
        );
    }

    #[test]
    fn ties_break_by_coordinate_x_then_y() {
        // Two equal-wear tiles both adjacent to the start; wear equal -> x asc,
        // then y asc, so (3,1) precedes (4,0).
        let tiles = [tile(3, 0, 99.0), tile(4, 0, 90.0), tile(3, 1, 90.0)];
        let corridor = select_road_corridor(&tiles, opts(1, 3));
        assert_eq!(corridor[0], RoadPos { x: 3, y: 0 });
        assert_eq!(corridor[1], RoadPos { x: 3, y: 1 });
        assert_eq!(corridor[2], RoadPos { x: 4, y: 0 });
    }

    #[test]
    fn custom_threshold_is_honoured() {
        let tiles = [tile(5, 0, 55.0), tile(6, 0, 52.0)];
        let mut o = opts(1, 10);
        o.wear_threshold = Some(50.0);
        let corridor = select_road_corridor(&tiles, o);
        assert_eq!(
            corridor,
            vec![RoadPos { x: 5, y: 0 }, RoadPos { x: 6, y: 0 }]
        );
    }

    // --- P14.4: road_route_to_network -----------------------------------------

    fn rp(x: i32, y: i32) -> RoadPos {
        RoadPos { x, y }
    }

    #[test]
    fn already_connected_entrance_returns_an_empty_route() {
        let entrances = [rp(0, 0), rp(1, 0)];
        let network: HashSet<RoadPos> = HashSet::from([rp(0, 0)]);
        let route = road_route_to_network(&entrances, |_| false, |p| network.contains(&p), 100);
        assert_eq!(route, Some(Vec::new()));
    }

    #[test]
    fn straight_line_route_reaches_the_network_and_excludes_it() {
        // Entrance at (0,0); network starts at (3,0). Open ground everywhere else.
        let entrances = [rp(0, 0)];
        let network: HashSet<RoadPos> = HashSet::from([rp(3, 0)]);
        let route = road_route_to_network(&entrances, |_| false, |p| network.contains(&p), 100)
            .expect("network is reachable");
        assert_eq!(route, vec![rp(0, 0), rp(1, 0), rp(2, 0)]);
    }

    #[test]
    fn routes_around_a_blocked_tile() {
        // (1,0) is blocked (another building) so the shortest route detours via y=1.
        let entrances = [rp(0, 0)];
        let network: HashSet<RoadPos> = HashSet::from([rp(2, 0)]);
        let blocked: HashSet<RoadPos> = HashSet::from([rp(1, 0)]);
        let route = road_route_to_network(
            &entrances,
            |p| blocked.contains(&p),
            |p| network.contains(&p),
            100,
        )
        .expect("a detour route exists");
        assert!(!route.contains(&rp(1, 0)), "never paves the blocked tile");
        // The route must end adjacent to the network tile (2,0) — e.g. (1,1) or (2,1).
        let last = *route.last().expect("non-empty route");
        assert_eq!(
            (last.x - 2).abs() + last.y.abs(),
            1,
            "ends adjacent to (2,0)"
        );
    }

    #[test]
    fn skips_a_blocked_entrance_and_starts_from_the_next_viable_one() {
        // First entrance in the stable order is blocked; the router should fall
        // back to the second rather than failing outright.
        let entrances = [rp(0, 0), rp(0, 1)];
        let blocked: HashSet<RoadPos> = HashSet::from([rp(0, 0)]);
        let network: HashSet<RoadPos> = HashSet::from([rp(0, 2)]);
        let route = road_route_to_network(
            &entrances,
            |p| blocked.contains(&p),
            |p| network.contains(&p),
            100,
        )
        .expect("second entrance reaches the network");
        assert_eq!(route, vec![rp(0, 1)]);
    }

    #[test]
    fn none_when_every_entrance_is_blocked() {
        let entrances = [rp(0, 0), rp(0, 1)];
        let route = road_route_to_network(&entrances, |_| true, |_| false, 100);
        assert_eq!(route, None);
    }

    #[test]
    fn none_when_the_network_is_unreachable_within_the_expansion_budget() {
        // Network only exists far away; a tiny expansion budget can't reach it.
        let entrances = [rp(0, 0)];
        let network: HashSet<RoadPos> = HashSet::from([rp(1000, 0)]);
        let route = road_route_to_network(&entrances, |_| false, |p| network.contains(&p), 5);
        assert_eq!(route, None);
    }

    #[test]
    fn deterministic_across_repeated_calls_with_identical_inputs() {
        let entrances = [rp(0, 0), rp(5, 5)];
        let network: HashSet<RoadPos> = HashSet::from([rp(4, 0), rp(0, -4)]);
        let route_a = road_route_to_network(&entrances, |_| false, |p| network.contains(&p), 200);
        let route_b = road_route_to_network(&entrances, |_| false, |p| network.contains(&p), 200);
        assert_eq!(route_a, route_b);
    }
}
