//! Deterministic shared-world land routing for accepted village barter caravans.
//!
//! Routes are planned once, before escrow is loaded, from canonical shared terrain plus
//! deterministic generation for the still-unmaterialised wilderness. The planner is deliberately
//! independent from colony fog: finding a route cannot reveal a tile or create village contact.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::{
    pathfinding::FenceEdge,
    types::TileType,
    village_area::side_delta,
    world_gen::{generate_world_chunk, get_colony_position, tile_to_chunk},
    world_tick::{
        ColonyRuntime, TilePos, VillageTradePosition, WorldState, effective_wall_segments,
        tile_has_water,
    },
};

const MIN_ROUTE_MARGIN: i32 = 192;
const MAX_ROUTE_MARGIN: i32 = 768;
const BASE_MAX_EXPANSIONS: usize = 32_768;
const EXPANSIONS_PER_DIRECT_TILE: usize = 64;
const ABSOLUTE_MAX_EXPANSIONS: usize = 1_000_000;
const ROUTE_COST_WEIGHT: u64 = 1;
const ROUTE_HEURISTIC_WEIGHT: u64 = 4;
const NEIGHBOURS: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VillageTradeRoutePlan {
    /// Turn-compressed route, excluding the source shrine and including the target shrine.
    pub waypoints: Vec<VillageTradePosition>,
    #[cfg(test)]
    pub expanded_nodes: usize,
    #[cfg(test)]
    pub generated_chunks: usize,
}

#[derive(Debug, Default)]
struct GeneratedTerrainCache {
    blocked: BTreeMap<TilePos, bool>,
    chunks: BTreeSet<(i32, i32)>,
}

impl GeneratedTerrainCache {
    fn generated_tile_is_blocked(&mut self, world_seed: u32, pos: TilePos) -> bool {
        if let Some(blocked) = self.blocked.get(&pos) {
            return *blocked;
        }
        let chunk = tile_to_chunk(pos.x, pos.y);
        if self.chunks.insert((chunk.chunk_x, chunk.chunk_y)) {
            // The canonical terrain itself is independent of the colony coordinates. The old
            // generator parameter only supplies the founding-chunk spring and danger distance;
            // using the world's original anchor here keeps unknown wilderness deterministic.
            let origin = get_colony_position();
            for tile in generate_world_chunk(
                chunk.chunk_x,
                chunk.chunk_y,
                i64::from(world_seed),
                origin.x,
                origin.y,
            ) {
                let tile_pos = TilePos {
                    x: tile.x,
                    y: tile.y,
                };
                self.blocked.insert(
                    tile_pos,
                    tile.tile_type == TileType::Mountains
                        || tile.tile_type == TileType::River
                        || tile.resources.water > 0,
                );
            }
        }
        self.blocked.get(&pos).copied().unwrap_or(true)
    }
}

#[derive(Debug, Clone, Copy)]
struct SearchBounds {
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
}

impl SearchBounds {
    fn around(start: TilePos, goal: TilePos) -> Self {
        let direct = manhattan(start, goal);
        let margin = i32::try_from(direct / 6)
            .unwrap_or(MAX_ROUTE_MARGIN)
            .saturating_add(MIN_ROUTE_MARGIN)
            .clamp(MIN_ROUTE_MARGIN, MAX_ROUTE_MARGIN);
        Self {
            min_x: start.x.min(goal.x).saturating_sub(margin),
            max_x: start.x.max(goal.x).saturating_add(margin),
            min_y: start.y.min(goal.y).saturating_sub(margin),
            max_y: start.y.max(goal.y).saturating_add(margin),
        }
    }

    fn contains(self, pos: TilePos) -> bool {
        (self.min_x..=self.max_x).contains(&pos.x) && (self.min_y..=self.max_y).contains(&pos.y)
    }
}

/// Plan one durable shrine-to-shrine route without consulting or changing either colony's fog.
///
/// Slightly weighted A* plus a goal-distance tie-break keeps an open multi-thousand-tile journey
/// close to route-length work instead of flooding the full start/goal rectangle. A fixed corridor
/// and expansion ceiling make adversarial/no-route inputs bounded; stable ordering matters more
/// here than proving the globally shortest trail.
pub(crate) fn plan_village_trade_route(
    world: &WorldState,
    source: &ColonyRuntime,
    target: &ColonyRuntime,
) -> Option<VillageTradeRoutePlan> {
    let start = shrine_tile(source);
    let goal = shrine_tile(target);
    if start == goal {
        return None;
    }

    let bounds = SearchBounds::around(start, goal);
    let direct = usize::try_from(manhattan(start, goal)).unwrap_or(ABSOLUTE_MAX_EXPANSIONS);
    let max_expansions = direct
        .saturating_mul(EXPANSIONS_PER_DIRECT_TILE)
        .saturating_add(BASE_MAX_EXPANSIONS)
        .min(ABSOLUTE_MAX_EXPANSIONS);
    let wall_edges = closed_wall_edges(world);
    let mut generated = GeneratedTerrainCache::default();
    let mut open = BTreeSet::<(u64, u32, u32, i32, i32)>::new();
    let mut best = BTreeMap::<TilePos, u32>::new();
    let mut came_from = BTreeMap::<TilePos, TilePos>::new();
    let start_h = manhattan(start, goal);
    open.insert((route_priority(0, start_h), start_h, 0, start.y, start.x));
    best.insert(start, 0);
    let mut expanded_nodes = 0usize;

    while let Some(entry) = open.pop_first() {
        let (_, _, cost, y, x) = entry;
        let current = TilePos { x, y };
        if best.get(&current).copied() != Some(cost) {
            continue;
        }
        expanded_nodes += 1;
        if expanded_nodes > max_expansions {
            return None;
        }
        if current == goal {
            let cells = reconstruct_path(start, goal, &came_from)?;
            return Some(VillageTradeRoutePlan {
                waypoints: compress_to_waypoints(&cells),
                #[cfg(test)]
                expanded_nodes,
                #[cfg(test)]
                generated_chunks: generated.chunks.len(),
            });
        }

        for (dx, dy) in NEIGHBOURS {
            let Some(x) = current.x.checked_add(dx) else {
                continue;
            };
            let Some(y) = current.y.checked_add(dy) else {
                continue;
            };
            let next = TilePos { x, y };
            if !bounds.contains(next)
                || wall_edges.contains(&FenceEdge::new(current.x, current.y, next.x, next.y))
                || (next != goal && terrain_is_blocked(world, &mut generated, next))
            {
                continue;
            }
            let next_cost = cost.saturating_add(1);
            if best.get(&next).is_some_and(|known| *known <= next_cost) {
                continue;
            }
            best.insert(next, next_cost);
            came_from.insert(next, current);
            let h = manhattan(next, goal);
            open.insert((route_priority(next_cost, h), h, next_cost, next.y, next.x));
        }
    }
    None
}

fn route_priority(cost: u32, heuristic: u32) -> u64 {
    u64::from(cost)
        .saturating_mul(ROUTE_COST_WEIGHT)
        .saturating_add(u64::from(heuristic).saturating_mul(ROUTE_HEURISTIC_WEIGHT))
}

fn shrine_tile(colony: &ColonyRuntime) -> TilePos {
    TilePos {
        x: colony.anchor.x + 1,
        y: colony.anchor.y + 1,
    }
}

fn manhattan(left: TilePos, right: TilePos) -> u32 {
    left.x
        .abs_diff(right.x)
        .saturating_add(left.y.abs_diff(right.y))
}

fn terrain_is_blocked(
    world: &WorldState,
    generated: &mut GeneratedTerrainCache,
    pos: TilePos,
) -> bool {
    let runtime = world.shared_spatial.tiles.get(&pos).or_else(|| {
        // Additive-save compatibility: a legacy/test world may not yet have projected its
        // colony-local plateau into `shared_spatial`. Stable id ordering mirrors the shared
        // authority migration tie-break without mutating the world during action validation.
        world
            .colonies
            .iter()
            .filter_map(|colony| colony.world_tiles.get(&pos).map(|tile| (&colony.id, tile)))
            .min_by(|left, right| left.0.cmp(right.0))
            .map(|(_, tile)| tile)
    });
    runtime.map_or_else(
        || generated.generated_tile_is_blocked(world.world_seed, pos),
        |tile| tile.tile_type == TileType::Mountains || tile_has_water(Some(tile)),
    )
}

fn closed_wall_edges(world: &WorldState) -> HashSet<FenceEdge> {
    world
        .colonies
        .iter()
        .flat_map(effective_wall_segments)
        .map(|entry| {
            let segment = entry.segment;
            let delta = side_delta(segment.side);
            FenceEdge::new(
                segment.x,
                segment.y,
                segment.x + delta.x,
                segment.y + delta.y,
            )
        })
        .collect()
}

fn reconstruct_path(
    start: TilePos,
    goal: TilePos,
    came_from: &BTreeMap<TilePos, TilePos>,
) -> Option<Vec<TilePos>> {
    let mut path = vec![goal];
    let mut current = goal;
    while current != start {
        current = *came_from.get(&current)?;
        path.push(current);
    }
    path.reverse();
    Some(path)
}

fn compress_to_waypoints(path: &[TilePos]) -> Vec<VillageTradePosition> {
    if path.len() < 2 {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut previous_direction = (
        path[1].x.saturating_sub(path[0].x),
        path[1].y.saturating_sub(path[0].y),
    );
    for index in 2..path.len() {
        let direction = (
            path[index].x.saturating_sub(path[index - 1].x),
            path[index].y.saturating_sub(path[index - 1].y),
        );
        if direction != previous_direction {
            result.push(position(path[index - 1]));
            previous_direction = direction;
        }
    }
    result.push(position(*path.last().expect("non-empty route")));
    result
}

fn position(pos: TilePos) -> VillageTradePosition {
    VillageTradePosition {
        x: f64::from(pos.x),
        y: f64::from(pos.y),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_tick::{
        WorldTileRuntime, found_colony_at, fresh_ground_tile, new_world, register_colony_spatial,
    };

    fn two_village_world(target: TilePos) -> WorldState {
        let mut world = new_world(42);
        world.colonies.push(found_colony_at(
            42,
            "source",
            1_000,
            11,
            TilePos { x: 6, y: 6 },
        ));
        world
            .colonies
            .push(found_colony_at(42, "target", 1_000, 12, target));
        register_colony_spatial(&mut world, 0);
        register_colony_spatial(&mut world, 1);
        world
    }

    fn route_cells(start: TilePos, waypoints: &[VillageTradePosition]) -> Vec<TilePos> {
        let mut result = Vec::new();
        let mut current = start;
        for waypoint in waypoints {
            let goal = TilePos {
                x: waypoint.x as i32,
                y: waypoint.y as i32,
            };
            while current != goal {
                current.x += (goal.x - current.x).signum();
                current.y += (goal.y - current.y).signum();
                result.push(current);
            }
        }
        result
    }

    fn gate_exterior(colony: &ColonyRuntime) -> TilePos {
        let area = crate::village_area::from_tiles(
            &colony
                .claimed_tiles
                .iter()
                .map(|tile| crate::village_layout::GridPos {
                    x: tile.x,
                    y: tile.y,
                })
                .collect::<Vec<_>>(),
        );
        let gate = crate::village_area::gate_placement_default(&area).expect("village gate");
        let delta = side_delta(gate.side);
        TilePos {
            x: gate.x + delta.x,
            y: gate.y + delta.y,
        }
    }

    #[test]
    fn shared_water_barrier_forces_a_stable_detour() {
        let mut world = two_village_world(TilePos { x: 106, y: 6 });
        for y in -10..=20 {
            if y == 18 {
                continue;
            }
            let pos = TilePos { x: 55, y };
            let mut tile =
                world
                    .shared_spatial
                    .tiles
                    .get(&pos)
                    .cloned()
                    .unwrap_or(WorldTileRuntime {
                        pos,
                        tile_type: TileType::River,
                        resources: crate::world_gen::TileResources {
                            food: 0,
                            herbs: 0,
                            water: 999,
                            gem: 0,
                            clay: 0,
                            sand: 0,
                        },
                        max_resources: crate::biomes::MaxResources { food: 0, herbs: 0 },
                        danger_level: 0.0,
                        path_wear: 0,
                        last_depleted: 0,
                        overlay_feature: Some("river".to_owned()),
                    });
            tile.tile_type = TileType::River;
            tile.resources.water = 999;
            world.shared_spatial.tiles.insert(pos, tile);
        }

        let plan = plan_village_trade_route(&world, &world.colonies[0], &world.colonies[1])
            .expect("barrier has one land gap");
        let twin = plan_village_trade_route(&world, &world.colonies[0], &world.colonies[1])
            .expect("same route");
        assert_eq!(plan.waypoints, twin.waypoints);
        let cells = route_cells(TilePos { x: 7, y: 7 }, &plan.waypoints);
        assert!(cells.iter().any(|tile| tile.x == 55));
        assert!(
            cells
                .iter()
                .filter(|tile| tile.x == 55)
                .all(|tile| tile.y == 18 || !(-10..=20).contains(&tile.y))
        );
    }

    #[test]
    fn flooded_gate_truthfully_has_no_land_route() {
        let mut world = two_village_world(TilePos { x: 106, y: 6 });
        let outside = gate_exterior(&world.colonies[0]);
        let tile = world
            .shared_spatial
            .tiles
            .get_mut(&outside)
            .expect("starter terrain contains gate exterior");
        tile.tile_type = TileType::River;
        tile.resources.water = 999;
        assert!(plan_village_trade_route(&world, &world.colonies[0], &world.colonies[1]).is_none());
    }

    #[test]
    fn long_open_route_is_linearish_and_does_not_materialise_or_reveal_wilderness() {
        let mut world = two_village_world(TilePos { x: 4_806, y: 6 });
        let source_gate = gate_exterior(&world.colonies[0]);
        let target_gate = gate_exterior(&world.colonies[1]);
        let mut cursor = source_gate;
        while cursor.x != target_gate.x {
            world
                .shared_spatial
                .tiles
                .insert(cursor, fresh_ground_tile(cursor));
            cursor.x += (target_gate.x - cursor.x).signum();
        }
        while cursor.y != target_gate.y {
            world
                .shared_spatial
                .tiles
                .insert(cursor, fresh_ground_tile(cursor));
            cursor.y += (target_gate.y - cursor.y).signum();
        }
        world
            .shared_spatial
            .tiles
            .insert(target_gate, fresh_ground_tile(target_gate));
        let before_shared = world.shared_spatial.clone();
        let before_fog = world
            .colonies
            .iter()
            .map(|colony| {
                (
                    colony.revealed_tiles.clone(),
                    colony.provisional_tiles.clone(),
                )
            })
            .collect::<Vec<_>>();
        let plan = plan_village_trade_route(&world, &world.colonies[0], &world.colonies[1])
            .expect("generated world has a bounded land route");
        assert!(
            plan.expanded_nodes < 80_000,
            "expanded {}",
            plan.expanded_nodes
        );
        assert!(
            plan.generated_chunks < 1_200,
            "generated {}",
            plan.generated_chunks
        );
        assert_eq!(world.shared_spatial, before_shared);
        assert_eq!(
            world
                .colonies
                .iter()
                .map(|colony| (
                    colony.revealed_tiles.clone(),
                    colony.provisional_tiles.clone()
                ))
                .collect::<Vec<_>>(),
            before_fog
        );
        assert_eq!(
            plan.waypoints.last(),
            Some(&VillageTradePosition { x: 4_807.0, y: 7.0 })
        );
    }

    #[test]
    fn every_wall_crossing_uses_each_villages_real_gate() {
        let world = two_village_world(TilePos { x: 106, y: 6 });
        let plan = plan_village_trade_route(&world, &world.colonies[0], &world.colonies[1])
            .expect("villages have a route");
        let cells = route_cells(TilePos { x: 7, y: 7 }, &plan.waypoints);
        let steps = std::iter::once(TilePos { x: 7, y: 7 })
            .chain(cells)
            .collect::<Vec<_>>();
        let closed = closed_wall_edges(&world);
        assert!(steps.windows(2).all(|pair| {
            !closed.contains(&FenceEdge::new(pair[0].x, pair[0].y, pair[1].x, pair[1].y))
        }));
        assert!(
            effective_wall_segments(&world.colonies[0])
                .iter()
                .all(|entry| !entry.segment.gate)
        );
    }
}
