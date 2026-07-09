//! Cat movement simulation ported from `lib/game/movement.ts`.

use std::collections::HashSet;

pub const MOVE_SPEED_TILES_PER_SEC: f64 = 0.5;
pub const WANDER_RADIUS: i32 = 3;
pub const EXPLORE_SPEED_FACTOR: f64 = 0.35;
pub const HUNT_RANGE_MIN: f64 = 8.0;
pub const HUNT_RANGE_MAX: f64 = 14.0;

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
        "ritual" => Some(context.shrine),
        "build_house" => Some(context.site.unwrap_or(context.anchor)),
        "expand_village" => context.expansion_site,
        "quarry" => context.quarry_site,
        "fetch_water" => context.water_site,
        "explore" => context.explore_site,
        "hunt_expedition" => hunt_destination(context),
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

    use super::{
        EXPLORE_SPEED_FACTOR, HUNT_RANGE_MAX, HUNT_RANGE_MIN, JobDestinationContext,
        MOVE_SPEED_TILES_PER_SEC, MovementStep, PathWalk, WANDER_RADIUS, WorldPos,
        advance_movement, advance_movement_default, destination_for_job, path_tiles,
        pick_wander_target, walk_path,
    };

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
