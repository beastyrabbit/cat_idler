//! Cat movement simulation ported from `lib/game/movement.ts`.

use std::collections::HashSet;

use crate::terrain_gen::BiomeRole;
use crate::types::LifeStage;

/// Reference tile-per-second rate for a cat crossing the village plateau
/// (grassland/lowland). Kept at the historical global value so the survival-
/// critical shrine-hauling loop, which lives on the plateau, moves at exactly the
/// old speed. This is the number the TS parity fixtures pin.
pub const MOVE_SPEED_TILES_PER_SEC: f64 = 0.5;
pub const WANDER_RADIUS: i32 = 3;
pub const EXPLORE_SPEED_FACTOR: f64 = 0.35;
pub const HUNT_RANGE_MIN: f64 = 8.0;
pub const HUNT_RANGE_MAX: f64 = 14.0;

// --- Staggered movement: per-terrain surface factor + per-cat gait ---------
//
// Effective speed = BASE_MOVE_SPEED_TILES_PER_SEC × surface_factor(biome)
//                    × cat_gait(id) × life_stage_gait(stage).
//
// Rather than every cat stepping in lockstep at a single global rate, each cat's
// step rate varies by the tile it is standing on and a small stable per-unit
// gait, so the herd desyncs naturally (Dwarf-Fortress style).

/// Grassland/lowland surface factor. The base rate is anchored to this so a cat
/// on the (grassland) village plateau moves at exactly `MOVE_SPEED_TILES_PER_SEC`.
pub const SURFACE_FACTOR_GRASSLAND: f64 = 0.75;
/// Lowland is the same easy footing as grassland.
pub const SURFACE_FACTOR_LOWLAND: f64 = 0.75;
/// Bare stone/rock is the firmest, fastest footing.
pub const SURFACE_FACTOR_ROCKY: f64 = 1.0;
/// Highland is exposed but firmer than grass — a touch slower than rock.
pub const SURFACE_FACTOR_HIGHLAND: f64 = 0.7;
/// Forest floor (roots, undergrowth) slows a cat down.
pub const SURFACE_FACTOR_FOREST: f64 = 0.6;
/// Loose sand is the slowest natural footing. Reserved: the sim terrain
/// generator has no sand biome yet, so no tile resolves to this today, but the
/// factor is kept named for when beaches/deserts land (roads/dirt come later).
pub const SURFACE_FACTOR_SAND: f64 = 0.5;

/// Base rate the surface/gait factors scale. Anchored so grassland/lowland (the
/// plateau) reproduces the historical `MOVE_SPEED_TILES_PER_SEC`, keeping the
/// *average* effective speed ~unchanged while introducing variance.
pub const BASE_MOVE_SPEED_TILES_PER_SEC: f64 = MOVE_SPEED_TILES_PER_SEC / SURFACE_FACTOR_GRASSLAND;

/// Lower bound of the per-cat gait multiplier.
pub const GAIT_MIN: f64 = 0.9;
/// Upper bound of the per-cat gait multiplier.
pub const GAIT_MAX: f64 = 1.1;

/// Deterministic per-tile surface speed factor for the biome a cat stands on.
#[must_use]
pub fn terrain_surface_factor(biome: BiomeRole) -> f64 {
    match biome {
        BiomeRole::Rocky => SURFACE_FACTOR_ROCKY,
        BiomeRole::Highland => SURFACE_FACTOR_HIGHLAND,
        BiomeRole::Grassland => SURFACE_FACTOR_GRASSLAND,
        BiomeRole::Lowland => SURFACE_FACTOR_LOWLAND,
        BiomeRole::Forest => SURFACE_FACTOR_FOREST,
    }
}

/// Small stable per-cat gait multiplier in `[GAIT_MIN, GAIT_MAX)`, derived from a
/// deterministic hash of the cat id (FNV-1a). No RNG, no shared chain: same id →
/// same gait forever, and two cats on the same tile still differ slightly. The
/// distribution is centred on `1.0` so the population average speed is unchanged.
#[must_use]
pub fn cat_gait(cat_id: &str) -> f64 {
    // FNV-1a 64-bit: platform-independent, stable across runs (unlike the std
    // `DefaultHasher`, which is randomized).
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = FNV_OFFSET;
    for byte in cat_id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    // Map the low bits to a fraction in [0, 1) then into the gait band.
    let fraction = (hash % 1_000_000) as f64 / 1_000_000.0;
    GAIT_MIN + fraction * (GAIT_MAX - GAIT_MIN)
}

/// Paved stone-road speed multiplier. A built road is firm, cleared footing, so a
/// cat on the road network covers noticeably more ground than on open grass.
pub const ROAD_BUILT_SPEED_MULT: f64 = 1.75;
/// Worn dirt-road speed multiplier. A trodden path (heavy cumulative wear that has
/// not yet been paved) gives a small boost over untrodden ground.
pub const DIRT_ROAD_SPEED_MULT: f64 = 1.05;
/// Path wear at which a trodden trail counts as a worn dirt road. Anchored to the
/// pathfinding worn-path / wear-paving threshold so movement and routing agree on
/// what a "road" is.
pub const WORN_ROAD_WEAR: u32 = 70;

/// Per-tile road speed multiplier folded into the movement phase's effective
/// speed. Paved stone roads (`overlay_feature == "road_built"`) are fastest; a
/// heavily worn dirt path is a touch faster than open ground; everything else is
/// neutral (`1.0`). Deterministic — a pure function of the tile's road state.
#[must_use]
pub fn road_surface_multiplier(
    is_road_built: bool,
    dirt_road_eligible: bool,
    path_wear: u32,
) -> f64 {
    if is_road_built {
        ROAD_BUILT_SPEED_MULT
    } else if dirt_road_eligible && path_wear >= WORN_ROAD_WEAR {
        DIRT_ROAD_SPEED_MULT
    } else {
        1.0
    }
}

/// P14.2 soft-obstacle speed multiplier: a cat crossing a building footprint or
/// standing on a tile with a tree decoration moves at ~25% of open-ground speed
/// (it CAN cross, it's just a bad idea — pathfinding's `BUILDING_FOOTPRINT_COST`
/// and `FOREST_COST`/`DENSE_WOODS_COST` cost tiers already route A* around these
/// tiles when a detour is reasonable; this is the matching speed-side factor for
/// whenever a cat actually is on one). Named to the same 0.25 tier the spec calls
/// "tree+building" (cost ∝ 1/speed, so cost 4.0 ⇒ speed 0.25 — see
/// `pathfinding::BUILDING_FOOTPRINT_COST` / `FOREST_COST`).
pub const SOFT_OBSTACLE_SPEED_MULT: f64 = 0.25;

/// Per-tile soft-obstacle speed multiplier folded into the movement phase's
/// effective speed, mirroring [`road_surface_multiplier`]'s shape. Deterministic —
/// a pure function of whether the standing tile is a soft obstacle.
#[must_use]
pub fn soft_obstacle_speed_multiplier(is_soft_obstacle: bool) -> f64 {
    if is_soft_obstacle {
        SOFT_OBSTACLE_SPEED_MULT
    } else {
        1.0
    }
}

// --- Transport research guardrail -------------------------------------------

/// Long-haul boundary used by transport guardrail tests. Route length alone must
/// not accelerate cats; a future rail implementation must prove that a cat is
/// riding a physical train on a physical track.
pub const RAIL_LONG_HAUL_DISTANCE_TILES: f64 = 40.0;

/// Research ownership is not physical transport. Until track, rolling stock,
/// boarding, and routes exist, Rail remains neutral at every distance.
#[must_use]
pub fn rail_speed_multiplier(_rail_researched: bool, _remaining_distance_tiles: f64) -> f64 {
    1.0
}

/// Life-stage gait modifier: kittens and elders pad along a bit slower than
/// young/adult cats. Adults (the colony's haulers) stay at `1.0`, so the
/// survival-critical work loop keeps its full speed.
#[must_use]
pub fn life_stage_gait(stage: LifeStage) -> f64 {
    match stage {
        LifeStage::Kitten | LifeStage::Elder => 0.85,
        LifeStage::Young | LifeStage::Adult => 1.0,
    }
}

/// Effective per-cat tile-per-second rate before route/road, explore, and upgrade
/// modifiers: base × terrain surface × per-cat gait × life-stage gait.
#[must_use]
pub fn effective_move_speed(biome: BiomeRole, cat_id: &str, stage: LifeStage) -> f64 {
    effective_move_speed_for_surface(terrain_surface_factor(biome), cat_id, stage)
}

/// Effective per-cat rate for one already-resolved fine-biome surface factor.
/// This is the P17 entry point used by the world tick; the coarse-biome wrapper
/// above remains for historical fixtures and callers.
#[must_use]
pub fn effective_move_speed_for_surface(
    surface_factor: f64,
    cat_id: &str,
    stage: LifeStage,
) -> f64 {
    BASE_MOVE_SPEED_TILES_PER_SEC
        * surface_factor.max(0.0)
        * cat_gait(cat_id)
        * life_stage_gait(stage)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldPos {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementStep {
    pub position: WorldPos,
    pub arrived: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathWalk {
    pub position: WorldPos,
    pub arrived: bool,
    pub tiles: Vec<WorldPos>,
}

#[derive(Debug, Clone, Copy)]
pub struct JobDestinationContext<'a> {
    pub anchor: WorldPos,
    pub shrine: WorldPos,
    pub food_tiles: &'a [WorldPos],
    pub roll: f64,
    pub site: Option<WorldPos>,
    pub expansion_site: Option<WorldPos>,
    pub quarry_site: Option<WorldPos>,
    pub water_site: Option<WorldPos>,
    pub explore_site: Option<WorldPos>,
    /// P16: the gather spot a `haul_gather_spot` mover job targets, if it still exists.
    pub gather_spot_site: Option<WorldPos>,
}

#[must_use]
pub fn advance_movement(
    position: WorldPos,
    destination: WorldPos,
    elapsed_sec: f64,
    speed: f64,
) -> MovementStep {
    let budget = elapsed_sec.max(0.0) * speed;
    let mut x = position.x;
    let mut y = position.y;

    let dx = destination.x - x;
    if dx != 0.0 {
        x += dx.signum() * dx.abs().min(budget);
    } else {
        let dy = destination.y - y;
        y += dy.signum() * dy.abs().min(budget);
    }

    MovementStep {
        position: WorldPos { x, y },
        arrived: x == destination.x && y == destination.y,
    }
}

#[must_use]
pub fn advance_movement_default(
    position: WorldPos,
    destination: WorldPos,
    elapsed_sec: f64,
) -> MovementStep {
    advance_movement(position, destination, elapsed_sec, MOVE_SPEED_TILES_PER_SEC)
}

#[must_use]
pub fn path_tiles(from: WorldPos, to: WorldPos) -> Vec<WorldPos> {
    let start_x = js_round_to_i32(from.x);
    let start_y = js_round_to_i32(from.y);
    let end_x = js_round_to_i32(to.x);
    let end_y = js_round_to_i32(to.y);

    let mut tiles = Vec::new();
    let step_x = (end_x - start_x).signum();
    let mut x = start_x;
    loop {
        tiles.push(world_pos(x, start_y));
        if x == end_x {
            break;
        }
        x += step_x;
    }

    let step_y = (end_y - start_y).signum();
    if step_y != 0 {
        let mut y = start_y + step_y;
        loop {
            tiles.push(world_pos(end_x, y));
            if y == end_y {
                break;
            }
            y += step_y;
        }
    }

    tiles
}

#[must_use]
pub fn walk_path(
    from: WorldPos,
    destination: WorldPos,
    budget_tiles: f64,
    waypoints: &[WorldPos],
) -> PathWalk {
    let mut budget = budget_tiles.max(0.0);
    let mut x = from.x;
    let mut y = from.y;
    let mut tiles = Vec::new();
    let mut seen = HashSet::new();

    record_tiles(&mut tiles, &mut seen, WorldPos { x, y }, WorldPos { x, y });

    let mut arrived = false;
    for (index, stop) in waypoints
        .iter()
        .chain(std::iter::once(&destination))
        .enumerate()
    {
        let dx = stop.x - x;
        if dx != 0.0 && budget > 0.0 {
            let movement = dx.abs().min(budget);
            let next_x = x + dx.signum() * movement;
            record_tiles(
                &mut tiles,
                &mut seen,
                WorldPos { x, y },
                WorldPos { x: next_x, y },
            );
            budget -= movement;
            x = next_x;
        }

        let dy = stop.y - y;
        if dy != 0.0 && budget > 0.0 {
            let movement = dy.abs().min(budget);
            let next_y = y + dy.signum() * movement;
            record_tiles(
                &mut tiles,
                &mut seen,
                WorldPos { x, y },
                WorldPos { x, y: next_y },
            );
            budget -= movement;
            y = next_y;
        }

        if x != stop.x || y != stop.y {
            break;
        }
        if index == waypoints.len() {
            arrived = true;
        }
    }

    PathWalk {
        position: WorldPos { x, y },
        arrived,
        tiles,
    }
}

/// Walk an orthogonal route using the actual speed of each crossed tile.
///
/// `speed_at_tile` is queried only when the route enters a new half-open tile
/// cell. Splitting one elapsed interval into smaller calls therefore produces
/// the same position: biome, road, and obstacle boundaries consume time at the
/// same spatial boundary rather than applying the starting tile's speed to the
/// whole tick.
#[must_use]
pub fn walk_path_timed<F>(
    from: WorldPos,
    destination: WorldPos,
    elapsed_sec: f64,
    waypoints: &[WorldPos],
    speed_at_tile: F,
) -> PathWalk
where
    F: FnMut(i32, i32) -> f64,
{
    walk_path_timed_with_elapsed(from, destination, elapsed_sec, waypoints, speed_at_tile).0
}

/// Timed-path variant that also returns the exact movement seconds consumed.
///
/// Callers with state transitions inside one simulation tick use the consumed value to
/// timestamp physical contact and spend only the remaining interval on the next state.
/// Ordinary walkers can continue using [`walk_path_timed`].
#[must_use]
pub fn walk_path_timed_with_elapsed<F>(
    from: WorldPos,
    destination: WorldPos,
    elapsed_sec: f64,
    waypoints: &[WorldPos],
    mut speed_at_tile: F,
) -> (PathWalk, f64)
where
    F: FnMut(i32, i32) -> f64,
{
    let mut remaining_sec = elapsed_sec.max(0.0);
    let available_sec = remaining_sec;
    let mut x = from.x;
    let mut y = from.y;
    let mut tiles = Vec::new();
    let mut seen = HashSet::new();

    record_tiles(&mut tiles, &mut seen, WorldPos { x, y }, WorldPos { x, y });

    let mut arrived = false;
    for (index, stop) in waypoints
        .iter()
        .chain(std::iter::once(&destination))
        .enumerate()
    {
        advance_axis_timed(
            &mut x,
            y,
            stop.x,
            &mut remaining_sec,
            &mut tiles,
            &mut seen,
            true,
            &mut speed_at_tile,
        );
        advance_axis_timed(
            &mut y,
            x,
            stop.y,
            &mut remaining_sec,
            &mut tiles,
            &mut seen,
            false,
            &mut speed_at_tile,
        );

        if x != stop.x || y != stop.y {
            break;
        }
        if index == waypoints.len() {
            arrived = true;
        }
    }

    (
        PathWalk {
            position: WorldPos { x, y },
            arrived,
            tiles,
        },
        available_sec - remaining_sec,
    )
}

#[allow(clippy::too_many_arguments)]
fn advance_axis_timed<F>(
    moving: &mut f64,
    fixed: f64,
    target: f64,
    remaining_sec: &mut f64,
    tiles: &mut Vec<WorldPos>,
    seen: &mut HashSet<(i32, i32)>,
    moving_x: bool,
    speed_at_tile: &mut F,
) where
    F: FnMut(i32, i32) -> f64,
{
    let direction = (target - *moving).signum();
    while direction != 0.0 && *moving != target && *remaining_sec > 0.0 {
        // At an exact half-tile boundary, assign the cell in the direction of
        // travel. This avoids a zero-length retry and makes reverse travel use
        // the same spatial partition as forward travel.
        let moving_tile = if direction > 0.0 {
            (*moving + 0.5).floor() as i32
        } else {
            (*moving - 0.5).ceil() as i32
        };
        let fixed_tile = (fixed + 0.5).floor() as i32;
        let boundary = f64::from(moving_tile) + direction * 0.5;
        let segment_end = if direction > 0.0 {
            target.min(boundary)
        } else {
            target.max(boundary)
        };
        let distance = (segment_end - *moving).abs();
        if distance <= f64::EPSILON {
            // Floating-point noise at a boundary: nudge by one ulp-equivalent
            // fraction, then let the ordinary segment accounting continue.
            *moving += direction * 1e-12;
            continue;
        }

        let (tile_x, tile_y) = if moving_x {
            (moving_tile, fixed_tile)
        } else {
            (fixed_tile, moving_tile)
        };
        let speed = speed_at_tile(tile_x, tile_y).max(f64::EPSILON);
        let needed_sec = distance / speed;
        let previous = if moving_x {
            WorldPos {
                x: *moving,
                y: fixed,
            }
        } else {
            WorldPos {
                x: fixed,
                y: *moving,
            }
        };
        if needed_sec <= *remaining_sec {
            *moving = segment_end;
            *remaining_sec -= needed_sec;
        } else {
            *moving += direction * speed * *remaining_sec;
            *remaining_sec = 0.0;
        }
        let next = if moving_x {
            WorldPos {
                x: *moving,
                y: fixed,
            }
        } else {
            WorldPos {
                x: fixed,
                y: *moving,
            }
        };
        record_tiles(tiles, seen, previous, next);
    }
}

/// Minimum / maximum length (in tiles) of a single scout wander leg. Each leg is a
/// short stretch in one randomly-chosen heading; stringing legs together makes a
/// scout meander outward across the fog instead of beelining a fixed frontier tile.
pub const SCOUT_LEG_MIN: f64 = 4.0;
pub const SCOUT_LEG_MAX: f64 = 9.0;
/// Half-width of the random heading turn applied to the outward radial, in radians.
/// `PI` keeps each leg somewhere in the outward half-plane (heading within ±90° of
/// straight-away-from-home), so the meander always has an outward component and the
/// scout tends into unrevealed territory rather than doubling back on the village.
pub const SCOUT_TURN_SPREAD: f64 = std::f64::consts::PI;

/// Pick the next scout wander target: a random-heading step of random length, biased
/// OUTWARD from the village so a scout tends away from home into unrevealed land.
///
/// `roll_dir` / `roll_len` are two draws from the seeded movement chain, so the same
/// seed reproduces the same meander (determinism). From the anchor itself the heading
/// is a free 360° pick (there is no outward radial yet); anywhere else it is the
/// outward radial from `anchor` through `from` plus a random turn of up to
/// ±[`SCOUT_TURN_SPREAD`] / 2. Re-picking each time the scout arrives changes its
/// direction, producing the random walk.
#[must_use]
pub fn scout_wander_target(
    from: WorldPos,
    anchor: WorldPos,
    roll_dir: f64,
    roll_len: f64,
) -> WorldPos {
    let dx = from.x - anchor.x;
    let dy = from.y - anchor.y;
    let heading = if dx == 0.0 && dy == 0.0 {
        roll_dir * std::f64::consts::TAU
    } else {
        dy.atan2(dx) + (roll_dir - 0.5) * SCOUT_TURN_SPREAD
    };
    let leg = SCOUT_LEG_MIN + roll_len.clamp(0.0, 1.0) * (SCOUT_LEG_MAX - SCOUT_LEG_MIN);
    WorldPos {
        x: (from.x + heading.cos() * leg).round(),
        y: (from.y + heading.sin() * leg).round(),
    }
}

#[must_use]
pub fn pick_wander_target(anchor: WorldPos, roll1: f64, roll2: f64) -> WorldPos {
    let span = f64::from(WANDER_RADIUS * 2 + 1);
    WorldPos {
        x: anchor.x - f64::from(WANDER_RADIUS) + (roll1 * span).floor(),
        y: anchor.y - f64::from(WANDER_RADIUS) + (roll2 * span).floor(),
    }
}

#[must_use]
pub fn destination_for_job(kind: &str, context: &JobDestinationContext<'_>) -> Option<WorldPos> {
    match kind {
        "ritual" | "perform_offering" | "forage_fibre" => Some(context.shrine),
        "carry_offering" => context.site.or(Some(context.shrine)),
        "build_house" => Some(context.site.unwrap_or(context.anchor)),
        "expand_village" => context.expansion_site,
        "quarry" | "gather_logs" | "replant_tree" | "gather_food" | "fish" => context.quarry_site,
        "fetch_water" => context.water_site,
        "explore" => context.explore_site,
        "hunt_expedition" => hunt_destination(context),
        "haul_gather_spot" => context.gather_spot_site,
        "village_maintenance" => Some(pick_wander_target(
            context.anchor,
            context.roll,
            (context.roll * 0.754_877_666_246_692_7).fract(),
        )),
        _ => None,
    }
}

fn hunt_destination(context: &JobDestinationContext<'_>) -> Option<WorldPos> {
    if !context.food_tiles.is_empty() {
        let clamped = context.roll.clamp(0.0, 0.999_999);
        let index = (clamped * context.food_tiles.len() as f64).floor() as usize;
        return Some(context.food_tiles[index]);
    }

    let angle = context.roll * std::f64::consts::PI * 2.0;
    let range = HUNT_RANGE_MIN + context.roll * (HUNT_RANGE_MAX - HUNT_RANGE_MIN);
    Some(WorldPos {
        x: (context.anchor.x + angle.cos() * range).round(),
        y: (context.anchor.y + angle.sin() * range).round(),
    })
}

fn record_tiles(
    tiles: &mut Vec<WorldPos>,
    seen: &mut HashSet<(i32, i32)>,
    from: WorldPos,
    to: WorldPos,
) {
    for tile in path_tiles(from, to) {
        let key = (tile.x as i32, tile.y as i32);
        if seen.insert(key) {
            tiles.push(tile);
        }
    }
}

fn js_round_to_i32(value: f64) -> i32 {
    (value + 0.5).floor() as i32
}

fn world_pos(x: i32, y: i32) -> WorldPos {
    WorldPos {
        x: f64::from(x),
        y: f64::from(y),
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use crate::rng::{movement_seed, roll_seeded};

    use crate::terrain_gen::BiomeRole;
    use crate::types::LifeStage;

    use super::{
        BASE_MOVE_SPEED_TILES_PER_SEC, DIRT_ROAD_SPEED_MULT, EXPLORE_SPEED_FACTOR, GAIT_MAX,
        GAIT_MIN, HUNT_RANGE_MAX, HUNT_RANGE_MIN, JobDestinationContext, MOVE_SPEED_TILES_PER_SEC,
        MovementStep, PathWalk, RAIL_LONG_HAUL_DISTANCE_TILES, ROAD_BUILT_SPEED_MULT,
        SCOUT_LEG_MAX, SCOUT_LEG_MIN, SOFT_OBSTACLE_SPEED_MULT, SURFACE_FACTOR_FOREST,
        SURFACE_FACTOR_GRASSLAND, SURFACE_FACTOR_HIGHLAND, SURFACE_FACTOR_LOWLAND,
        SURFACE_FACTOR_ROCKY, SURFACE_FACTOR_SAND, WANDER_RADIUS, WORN_ROAD_WEAR, WorldPos,
        advance_movement, advance_movement_default, cat_gait, destination_for_job,
        effective_move_speed, life_stage_gait, path_tiles, pick_wander_target,
        rail_speed_multiplier, road_surface_multiplier, scout_wander_target,
        soft_obstacle_speed_multiplier, terrain_surface_factor, walk_path, walk_path_timed,
    };

    fn dist(a: WorldPos, b: WorldPos) -> f64 {
        ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
    }

    #[test]
    fn scout_wander_target_is_deterministic() {
        let from = WorldPos { x: 12.0, y: 7.0 };
        let anchor = WorldPos { x: 0.0, y: 0.0 };
        assert_eq!(
            scout_wander_target(from, anchor, 0.3, 0.6),
            scout_wander_target(from, anchor, 0.3, 0.6),
        );
    }

    #[test]
    fn scout_wander_legs_are_length_bounded() {
        let from = WorldPos { x: 8.0, y: -3.0 };
        let anchor = WorldPos { x: 0.0, y: 0.0 };
        // Rounding to whole tiles moves each axis by up to 0.5, so the realised leg
        // length can sit ~0.71 outside the [min, max] band — allow a small tolerance.
        for i in 0..=20 {
            let roll_dir = f64::from(i) / 20.0;
            for j in 0..=20 {
                let roll_len = f64::from(j) / 20.0;
                let target = scout_wander_target(from, anchor, roll_dir, roll_len);
                let leg = dist(target, from);
                assert!(
                    (SCOUT_LEG_MIN - 1.0..=SCOUT_LEG_MAX + 1.0).contains(&leg),
                    "leg {leg} out of bounds for rolls ({roll_dir}, {roll_len})"
                );
            }
        }
    }

    #[test]
    fn scout_wander_tends_outward_from_the_village() {
        // From anywhere outside the anchor, every heading stays in the outward
        // half-plane, so the target is never meaningfully closer to home than the
        // scout already is (allowing ~1 tile of rounding slack).
        let anchor = WorldPos { x: 0.0, y: 0.0 };
        let from = WorldPos { x: 10.0, y: 4.0 };
        let from_dist = dist(from, anchor);
        for i in 0..=36 {
            let roll_dir = f64::from(i) / 36.0;
            let target = scout_wander_target(from, anchor, roll_dir, 0.5);
            assert!(
                dist(target, anchor) >= from_dist - 1.5,
                "scout stepped inward: from_dist {from_dist}, new {} (roll {roll_dir})",
                dist(target, anchor)
            );
        }
    }

    #[test]
    fn scout_wander_changes_direction_with_the_heading_roll() {
        // Different heading rolls fan the scout out along different bearings — the
        // "changes direction" half of the random walk.
        let from = WorldPos { x: 6.0, y: 6.0 };
        let anchor = WorldPos { x: 0.0, y: 0.0 };
        let a = scout_wander_target(from, anchor, 0.05, 0.5);
        let b = scout_wander_target(from, anchor, 0.95, 0.5);
        assert_ne!(
            a, b,
            "distinct heading rolls must aim the scout differently"
        );
    }

    #[test]
    fn scout_wander_from_the_anchor_sweeps_a_full_circle() {
        // With no outward radial yet, the heading is a free 360° pick: roll 0 heads
        // +x, roll 0.25 heads +y, so the first leg out of the village can go anywhere.
        let anchor = WorldPos { x: 0.0, y: 0.0 };
        let east = scout_wander_target(anchor, anchor, 0.0, 1.0);
        let north = scout_wander_target(anchor, anchor, 0.25, 1.0);
        assert!(
            east.x > 0.0 && east.y.abs() < 1.0,
            "roll 0 should head +x: {east:?}"
        );
        assert!(
            north.y > 0.0 && north.x.abs() < 1.0,
            "roll 0.25 should head +y: {north:?}"
        );
    }

    #[derive(Debug, Deserialize)]
    struct Fixture {
        source: String,
        constants: ConstantFixture,
        counts: CountFixture,
        #[serde(rename = "advanceMovement")]
        advance_movement: Vec<AdvanceCase>,
        #[serde(rename = "pathTiles")]
        path_tiles: Vec<PathTilesCase>,
        #[serde(rename = "walkPath")]
        walk_path: Vec<WalkPathCase>,
        #[serde(rename = "pickWanderTarget")]
        pick_wander_target: Vec<WanderCase>,
        #[serde(rename = "destinationForJob")]
        destination_for_job: Vec<DestinationCase>,
        #[serde(rename = "seededWander")]
        seeded_wander: Vec<SeededWanderCase>,
    }

    #[derive(Debug, Deserialize)]
    struct ConstantFixture {
        #[serde(rename = "MOVE_SPEED_TILES_PER_SEC")]
        move_speed_tiles_per_sec: f64,
        #[serde(rename = "WANDER_RADIUS")]
        wander_radius: i32,
        #[serde(rename = "EXPLORE_SPEED_FACTOR")]
        explore_speed_factor: f64,
        #[serde(rename = "HUNT_RANGE_MIN")]
        hunt_range_min: f64,
        #[serde(rename = "HUNT_RANGE_MAX")]
        hunt_range_max: f64,
    }

    #[derive(Debug, Deserialize)]
    struct CountFixture {
        #[serde(rename = "advanceMovement")]
        advance_movement: usize,
        #[serde(rename = "pathTiles")]
        path_tiles: usize,
        #[serde(rename = "walkPath")]
        walk_path: usize,
        #[serde(rename = "pickWanderTarget")]
        pick_wander_target: usize,
        #[serde(rename = "destinationForJob")]
        destination_for_job: usize,
        #[serde(rename = "seededWander")]
        seeded_wander: usize,
        total: usize,
    }

    #[derive(Debug, Deserialize)]
    struct AdvanceCase {
        name: String,
        position: PosFixture,
        destination: PosFixture,
        #[serde(rename = "elapsedSec")]
        elapsed_sec: f64,
        speed: Option<f64>,
        expected: StepFixture,
    }

    #[derive(Debug, Deserialize)]
    struct PathTilesCase {
        name: String,
        from: PosFixture,
        to: PosFixture,
        expected: Vec<PosFixture>,
    }

    #[derive(Debug, Deserialize)]
    struct WalkPathCase {
        name: String,
        from: PosFixture,
        destination: PosFixture,
        #[serde(rename = "budgetTiles")]
        budget_tiles: f64,
        waypoints: Vec<PosFixture>,
        expected: WalkFixture,
    }

    #[derive(Debug, Deserialize)]
    struct WanderCase {
        name: String,
        anchor: PosFixture,
        roll1: f64,
        roll2: f64,
        expected: PosFixture,
    }

    #[derive(Debug, Deserialize)]
    struct DestinationCase {
        name: String,
        kind: String,
        context: DestinationContextFixture,
        expected: Option<PosFixture>,
    }

    #[derive(Debug, Deserialize)]
    struct SeededWanderCase {
        name: String,
        #[serde(rename = "baseSeed")]
        base_seed: u32,
        #[serde(rename = "startSeed")]
        start_seed: u32,
        roll1: f64,
        roll2: f64,
        #[serde(rename = "nextSeed")]
        next_seed: u32,
        anchor: PosFixture,
        expected: PosFixture,
    }

    #[derive(Debug, Clone, Copy, Deserialize)]
    struct PosFixture {
        x: f64,
        y: f64,
    }

    #[derive(Debug, Deserialize)]
    struct StepFixture {
        position: PosFixture,
        arrived: bool,
    }

    #[derive(Debug, Deserialize)]
    struct WalkFixture {
        position: PosFixture,
        arrived: bool,
        tiles: Vec<PosFixture>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DestinationContextFixture {
        anchor: PosFixture,
        shrine: PosFixture,
        food_tiles: Vec<PosFixture>,
        roll: f64,
        site: Option<PosFixture>,
        expansion_site: Option<PosFixture>,
        quarry_site: Option<PosFixture>,
        water_site: Option<PosFixture>,
        explore_site: Option<PosFixture>,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../docs/migration/fixtures/p3/movement.json"
        ))
        .expect("movement fixture parses")
    }

    #[test]
    fn fixture_is_generated_from_movement_ts() {
        let fixture = fixture();
        assert_eq!(fixture.source, "lib/game/movement.ts");
        assert_eq!(
            fixture.counts.advance_movement,
            fixture.advance_movement.len()
        );
        assert_eq!(fixture.counts.path_tiles, fixture.path_tiles.len());
        assert_eq!(fixture.counts.walk_path, fixture.walk_path.len());
        assert_eq!(
            fixture.counts.pick_wander_target,
            fixture.pick_wander_target.len()
        );
        assert_eq!(
            fixture.counts.destination_for_job,
            fixture.destination_for_job.len()
        );
        assert_eq!(fixture.counts.seeded_wander, fixture.seeded_wander.len());
        assert_eq!(
            fixture.counts.total,
            fixture.advance_movement.len()
                + fixture.path_tiles.len()
                + fixture.walk_path.len()
                + fixture.pick_wander_target.len()
                + fixture.destination_for_job.len()
                + fixture.seeded_wander.len()
        );
    }

    #[test]
    fn constants_match_ts_fixture() {
        let constants = fixture().constants;
        assert_eq!(MOVE_SPEED_TILES_PER_SEC, constants.move_speed_tiles_per_sec);
        assert_eq!(WANDER_RADIUS, constants.wander_radius);
        assert_eq!(EXPLORE_SPEED_FACTOR, constants.explore_speed_factor);
        assert_eq!(HUNT_RANGE_MIN, constants.hunt_range_min);
        assert_eq!(HUNT_RANGE_MAX, constants.hunt_range_max);
    }

    #[test]
    fn advance_movement_matches_ts_fixture() {
        for case in fixture().advance_movement {
            let actual = if let Some(speed) = case.speed {
                advance_movement(
                    case.position.into_world_pos(),
                    case.destination.into_world_pos(),
                    case.elapsed_sec,
                    speed,
                )
            } else {
                advance_movement_default(
                    case.position.into_world_pos(),
                    case.destination.into_world_pos(),
                    case.elapsed_sec,
                )
            };
            assert_eq!(
                actual,
                case.expected.into_step(),
                "advanceMovement case {}",
                case.name
            );
        }
    }

    #[test]
    fn path_tiles_match_ts_fixture() {
        for case in fixture().path_tiles {
            let actual = path_tiles(case.from.into_world_pos(), case.to.into_world_pos());
            assert_eq!(
                actual,
                case.expected.into_world_positions(),
                "pathTiles case {}",
                case.name
            );
        }
    }

    #[test]
    fn walk_path_matches_ts_fixture() {
        for case in fixture().walk_path {
            let waypoints = case.waypoints.into_world_positions();
            let actual = walk_path(
                case.from.into_world_pos(),
                case.destination.into_world_pos(),
                case.budget_tiles,
                &waypoints,
            );
            assert_eq!(
                actual,
                case.expected.into_walk(),
                "walkPath case {}",
                case.name
            );
        }
    }

    #[test]
    fn pick_wander_target_matches_ts_fixture() {
        for case in fixture().pick_wander_target {
            let actual = pick_wander_target(case.anchor.into_world_pos(), case.roll1, case.roll2);
            assert_eq!(
                actual,
                case.expected.into_world_pos(),
                "pickWanderTarget case {}",
                case.name
            );
        }
    }

    #[test]
    fn destination_for_job_matches_ts_fixture() {
        for case in fixture().destination_for_job {
            let food_tiles = case.context.food_tiles.into_world_positions();
            let context = JobDestinationContext {
                anchor: case.context.anchor.into_world_pos(),
                shrine: case.context.shrine.into_world_pos(),
                food_tiles: &food_tiles,
                roll: case.context.roll,
                site: case.context.site.map(PosFixture::into_world_pos),
                expansion_site: case.context.expansion_site.map(PosFixture::into_world_pos),
                quarry_site: case.context.quarry_site.map(PosFixture::into_world_pos),
                water_site: case.context.water_site.map(PosFixture::into_world_pos),
                explore_site: case.context.explore_site.map(PosFixture::into_world_pos),
                gather_spot_site: None,
            };

            assert_eq!(
                destination_for_job(&case.kind, &context),
                case.expected.map(PosFixture::into_world_pos),
                "destinationForJob case {}",
                case.name
            );
        }
    }

    #[test]
    fn movement_seeded_wander_matches_ts_fixture() {
        for case in fixture().seeded_wander {
            let start_seed = movement_seed(case.base_seed);
            assert_eq!(start_seed, case.start_seed, "seed case {}", case.name);

            let first = roll_seeded(f64::from(start_seed));
            let second = roll_seeded(f64::from(first.next_seed));
            assert_eq!(first.value, case.roll1, "first roll case {}", case.name);
            assert_eq!(second.value, case.roll2, "second roll case {}", case.name);
            assert_eq!(second.next_seed, case.next_seed, "next seed {}", case.name);

            let target =
                pick_wander_target(case.anchor.into_world_pos(), first.value, second.value);
            assert_eq!(
                target,
                case.expected.into_world_pos(),
                "seeded wander case {}",
                case.name
            );
        }
    }

    #[test]
    fn terrain_surface_factor_orders_biomes_rock_fastest_forest_slowest() {
        assert_eq!(SURFACE_FACTOR_ROCKY, 1.0);
        assert_eq!(SURFACE_FACTOR_GRASSLAND, 0.75);
        assert_eq!(SURFACE_FACTOR_LOWLAND, 0.75);
        assert_eq!(SURFACE_FACTOR_SAND, 0.5);
        assert_eq!(
            terrain_surface_factor(BiomeRole::Rocky),
            SURFACE_FACTOR_ROCKY
        );
        assert_eq!(
            terrain_surface_factor(BiomeRole::Highland),
            SURFACE_FACTOR_HIGHLAND
        );
        assert_eq!(
            terrain_surface_factor(BiomeRole::Grassland),
            SURFACE_FACTOR_GRASSLAND
        );
        assert_eq!(
            terrain_surface_factor(BiomeRole::Lowland),
            SURFACE_FACTOR_LOWLAND
        );
        assert_eq!(
            terrain_surface_factor(BiomeRole::Forest),
            SURFACE_FACTOR_FOREST
        );

        // Ordering: rock (firmest) > grassland/lowland > highland > forest floor;
        // sand (reserved) is the slowest natural footing of all.
        assert!(
            terrain_surface_factor(BiomeRole::Rocky) > terrain_surface_factor(BiomeRole::Grassland)
        );
        assert!(
            terrain_surface_factor(BiomeRole::Grassland)
                > terrain_surface_factor(BiomeRole::Highland)
        );
        assert!(
            terrain_surface_factor(BiomeRole::Highland) > terrain_surface_factor(BiomeRole::Forest)
        );
        const {
            assert!(SURFACE_FACTOR_SAND < SURFACE_FACTOR_FOREST);
            assert!(SURFACE_FACTOR_SAND < SURFACE_FACTOR_ROCKY);
        }
    }

    #[test]
    fn base_speed_reproduces_old_rate_on_the_plateau() {
        // Grassland/lowland (the village plateau) must reproduce the historical
        // global rate so the survival-critical shrine-haul loop is unchanged.
        assert!(
            (BASE_MOVE_SPEED_TILES_PER_SEC * SURFACE_FACTOR_GRASSLAND - MOVE_SPEED_TILES_PER_SEC)
                .abs()
                < 1e-12
        );
        assert!(
            (BASE_MOVE_SPEED_TILES_PER_SEC * SURFACE_FACTOR_LOWLAND - MOVE_SPEED_TILES_PER_SEC)
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn cat_gait_is_deterministic_and_within_bounds() {
        for id in ["cat-1", "cat-2", "abc", "a-very-long-cat-id-123456", ""] {
            let a = cat_gait(id);
            let b = cat_gait(id);
            assert_eq!(a, b, "gait must be stable for id {id}");
            assert!(a >= GAIT_MIN, "gait {a} below floor for id {id}");
            assert!(a < GAIT_MAX, "gait {a} above ceiling for id {id}");
        }
    }

    #[test]
    fn cat_gait_differs_across_ids() {
        assert_ne!(cat_gait("cat-1"), cat_gait("cat-2"));
        assert_ne!(cat_gait("alpha"), cat_gait("beta"));
    }

    #[test]
    fn life_stage_gait_slows_kittens_and_elders_only() {
        assert_eq!(life_stage_gait(LifeStage::Young), 1.0);
        assert_eq!(life_stage_gait(LifeStage::Adult), 1.0);
        assert!(life_stage_gait(LifeStage::Kitten) < 1.0);
        assert!(life_stage_gait(LifeStage::Elder) < 1.0);
    }

    #[test]
    fn cat_on_sand_style_slow_tile_falls_behind_a_cat_on_stone() {
        // Same cat, same elapsed budget: forest (slow) footing covers fewer tiles
        // than rocky (fast) footing — the mechanic that staggers the herd. (The
        // sim has no sand biome yet; forest is the slowest ground that exists.)
        let elapsed = 4.0;
        let slow_speed = effective_move_speed(BiomeRole::Forest, "cat-1", LifeStage::Adult);
        let fast_speed = effective_move_speed(BiomeRole::Rocky, "cat-1", LifeStage::Adult);
        assert!(fast_speed > slow_speed);

        let start = WorldPos { x: 0.0, y: 0.0 };
        let dest = WorldPos { x: 20.0, y: 0.0 };
        let slow = walk_path(start, dest, elapsed * slow_speed, &[]);
        let fast = walk_path(start, dest, elapsed * fast_speed, &[]);
        assert!(
            fast.position.x > slow.position.x,
            "stone cat ({}) should out-walk forest cat ({})",
            fast.position.x,
            slow.position.x
        );
    }

    #[test]
    fn two_cats_same_tile_different_gait_desync() {
        // Identical tile, identical budget, different ids → different gaits →
        // they end at different positions. The herd staggers instead of lockstepping.
        let elapsed = 6.0;
        let start = WorldPos { x: 0.0, y: 0.0 };
        let dest = WorldPos { x: 30.0, y: 0.0 };

        let speed_a = effective_move_speed(BiomeRole::Grassland, "cat-1", LifeStage::Adult);
        let speed_b = effective_move_speed(BiomeRole::Grassland, "cat-2", LifeStage::Adult);
        assert_ne!(speed_a, speed_b, "distinct ids must yield distinct gaits");

        let walk_a = walk_path(start, dest, elapsed * speed_a, &[]);
        let walk_b = walk_path(start, dest, elapsed * speed_b, &[]);
        assert_ne!(
            walk_a.position, walk_b.position,
            "same-tile cats with different gaits must diverge"
        );
    }

    #[test]
    fn effective_move_speed_is_deterministic() {
        let first = effective_move_speed(BiomeRole::Forest, "cat-42", LifeStage::Young);
        let second = effective_move_speed(BiomeRole::Forest, "cat-42", LifeStage::Young);
        assert_eq!(first, second);
    }

    #[test]
    fn timed_walk_is_invariant_to_tick_partition_across_surface_boundaries() {
        let start = WorldPos { x: 0.0, y: 0.0 };
        let destination = WorldPos { x: 4.0, y: 0.0 };
        let speed = |x: i32, _y: i32| match x {
            ..=0 => 0.5,
            1 => 0.25,
            2 => 1.0,
            _ => 0.75,
        };

        let whole = walk_path_timed(start, destination, 4.75, &[], speed);
        let mut split_position = start;
        let mut split_arrived = false;
        for _ in 0..19 {
            let step = walk_path_timed(split_position, destination, 0.25, &[], speed);
            split_position = step.position;
            split_arrived = step.arrived;
        }

        assert_eq!(whole.position, split_position);
        assert_eq!(whole.arrived, split_arrived);
    }

    #[test]
    fn timed_walk_uses_the_tile_being_crossed_in_both_directions() {
        let speed = |x: i32, _y: i32| if x == 1 { 0.25 } else { 1.0 };
        let east = walk_path_timed(
            WorldPos { x: 0.0, y: 0.0 },
            WorldPos { x: 2.0, y: 0.0 },
            2.0,
            &[],
            speed,
        );
        let west = walk_path_timed(
            WorldPos { x: 2.0, y: 0.0 },
            WorldPos { x: 0.0, y: 0.0 },
            2.0,
            &[],
            speed,
        );

        assert_eq!(east.position.x, 0.875);
        assert_eq!(west.position.x, 1.125);
        assert_eq!(east.position.y, west.position.y);
    }

    #[test]
    fn road_surface_multiplier_orders_stone_road_over_dirt_over_grass() {
        let stone = road_surface_multiplier(true, true, 0);
        let dirt = road_surface_multiplier(false, true, WORN_ROAD_WEAR);
        let grass = road_surface_multiplier(false, true, 0);
        assert_eq!(ROAD_BUILT_SPEED_MULT, 1.75);
        assert_eq!(DIRT_ROAD_SPEED_MULT, 1.05);
        assert_eq!(stone, ROAD_BUILT_SPEED_MULT);
        assert_eq!(dirt, DIRT_ROAD_SPEED_MULT);
        assert_eq!(grass, 1.0);
        assert!(stone > dirt && dirt > grass);
        // Just-below-threshold wear is still plain ground.
        assert_eq!(
            road_surface_multiplier(false, true, WORN_ROAD_WEAR - 1),
            1.0
        );
        assert_eq!(
            road_surface_multiplier(true, false, WORN_ROAD_WEAR),
            ROAD_BUILT_SPEED_MULT,
            "traffic wear cannot downgrade a built stone road into a dirt road"
        );
        assert_eq!(
            road_surface_multiplier(false, false, WORN_ROAD_WEAR),
            1.0,
            "stone ground cannot become a dirt road"
        );
    }

    #[test]
    fn cat_on_stone_road_out_walks_the_same_cat_on_grass_over_one_budget() {
        // Same cat (id + stage), same biome, same elapsed budget: the only
        // difference is the ground it stands on. The road tile covers more ground.
        let elapsed = 6.0;
        let start = WorldPos { x: 0.0, y: 0.0 };
        let dest = WorldPos { x: 40.0, y: 0.0 };
        let base = effective_move_speed(BiomeRole::Grassland, "cat-7", LifeStage::Adult);

        let road_speed = base * road_surface_multiplier(true, true, 100);
        let grass_speed = base * road_surface_multiplier(false, true, 0);

        let on_road = walk_path(start, dest, elapsed * road_speed, &[]);
        let on_grass = walk_path(start, dest, elapsed * grass_speed, &[]);
        assert!(
            on_road.position.x > on_grass.position.x,
            "road cat ({}) should out-walk grass cat ({})",
            on_road.position.x,
            on_grass.position.x
        );
    }

    // ---- P14.2: soft-obstacle (building/tree) movement speed ---------------

    // ---- P17: rail (land) long-haul speed boost -----------------------------

    #[test]
    fn rail_speed_multiplier_is_neutral_without_the_upgrade_or_below_the_long_haul_threshold() {
        // No rail: neutral at any distance, including a very long one.
        assert_eq!(rail_speed_multiplier(false, 0.0), 1.0);
        assert_eq!(
            rail_speed_multiplier(false, RAIL_LONG_HAUL_DISTANCE_TILES),
            1.0
        );
        assert_eq!(rail_speed_multiplier(false, 1_000.0), 1.0);
        // Rail owned but the route is village-scale (below the threshold): still
        // neutral — a founding colony's hunt/quarry/explore routes never trip it.
        assert_eq!(
            rail_speed_multiplier(true, RAIL_LONG_HAUL_DISTANCE_TILES - 0.01),
            1.0
        );
        assert_eq!(rail_speed_multiplier(true, HUNT_RANGE_MAX), 1.0);
    }

    #[test]
    fn rail_ownership_alone_is_neutral_even_on_long_distance_hauls() {
        assert_eq!(
            rail_speed_multiplier(true, RAIL_LONG_HAUL_DISTANCE_TILES),
            1.0
        );
        assert_eq!(rail_speed_multiplier(true, 1_000.0), 1.0);
    }

    #[test]
    fn rail_speed_multiplier_is_deterministic() {
        let a = rail_speed_multiplier(true, RAIL_LONG_HAUL_DISTANCE_TILES + 5.0);
        let b = rail_speed_multiplier(true, RAIL_LONG_HAUL_DISTANCE_TILES + 5.0);
        assert_eq!(a, b);
    }

    #[test]
    fn soft_obstacle_speed_multiplier_is_a_quarter_speed_matching_the_pathfinding_cost_tier() {
        // cost ∝ 1/speed: pathfinding's BUILDING_FOOTPRINT_COST/FOREST_COST are
        // both 4.0, i.e. this same 0.25 speed tier.
        assert_eq!(SOFT_OBSTACLE_SPEED_MULT, 0.25);
        assert_eq!(
            soft_obstacle_speed_multiplier(true),
            SOFT_OBSTACLE_SPEED_MULT
        );
        assert_eq!(soft_obstacle_speed_multiplier(false), 1.0);
    }

    #[test]
    fn cat_crossing_a_soft_obstacle_covers_a_quarter_the_ground_of_open_terrain() {
        // Same cat, same biome, same elapsed budget: the only difference is
        // whether the tile is a soft obstacle (building footprint / tree).
        let elapsed = 8.0;
        let start = WorldPos { x: 0.0, y: 0.0 };
        let dest = WorldPos { x: 40.0, y: 0.0 };
        let base = effective_move_speed(BiomeRole::Grassland, "cat-11", LifeStage::Adult);

        let open_speed = base * soft_obstacle_speed_multiplier(false);
        let obstacle_speed = base * soft_obstacle_speed_multiplier(true);
        assert_eq!(obstacle_speed, open_speed * 0.25);

        let open = walk_path(start, dest, elapsed * open_speed, &[]);
        let obstacle = walk_path(start, dest, elapsed * obstacle_speed, &[]);
        assert!(
            open.position.x > obstacle.position.x,
            "open-ground cat ({}) should out-walk the cat crossing a soft obstacle ({})",
            open.position.x,
            obstacle.position.x
        );
    }

    impl PosFixture {
        fn into_world_pos(self) -> WorldPos {
            WorldPos {
                x: self.x,
                y: self.y,
            }
        }
    }

    impl StepFixture {
        fn into_step(self) -> MovementStep {
            MovementStep {
                position: self.position.into_world_pos(),
                arrived: self.arrived,
            }
        }
    }

    impl WalkFixture {
        fn into_walk(self) -> PathWalk {
            PathWalk {
                position: self.position.into_world_pos(),
                arrived: self.arrived,
                tiles: self.tiles.into_world_positions(),
            }
        }
    }

    trait IntoWorldPositions {
        fn into_world_positions(self) -> Vec<WorldPos>;
    }

    impl IntoWorldPositions for Vec<PosFixture> {
        fn into_world_positions(self) -> Vec<WorldPos> {
            self.into_iter().map(PosFixture::into_world_pos).collect()
        }
    }
}
