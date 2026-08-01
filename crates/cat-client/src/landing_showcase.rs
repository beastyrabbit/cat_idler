//! Deterministic, save-independent settlement staged behind the entry charter.
//!
//! This is deliberately not a simulation snapshot. It reuses the shipped world
//! art, road grammar, station compositions, cat atlas, animation timing, and
//! depth rules to present a prosperous mature village without touching player
//! state, networking, persistence, or `cat-sim`.

use super::*;

pub(super) const LANDING_SHOWCASE_ANCHOR: TilePoint = TilePoint { x: 8_192, y: 8_192 };

const SHOWCASE_HALF_WIDTH: i32 = 34;
const SHOWCASE_HALF_HEIGHT: i32 = 21;
const SHOWCASE_CAT_COUNT: usize = 60;
const _: () = assert!(SHOWCASE_CAT_COUNT >= 50);
const SHOWCASE_GROUND_Z: f32 = Z_FOG + 4.0;
const SHOWCASE_ROAD_Z: f32 = Z_FOG + 6.0;
const SHOWCASE_FLOOR_Z: f32 = Z_FOG + 8.0;
const SHOWCASE_YSORT_BASE: f32 = Z_FOG + 120.0;

#[derive(Component)]
pub(super) struct LandingShowcaseVisual;

#[derive(Component)]
pub(super) struct LandingShowcaseCat {
    route: usize,
    waypoint: usize,
}

#[derive(Clone, Copy)]
struct ShowcaseLot {
    kind: BuildingType,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

const fn lot(kind: BuildingType, x: i32, y: i32, width: i32, height: i32) -> ShowcaseLot {
    ShowcaseLot {
        kind,
        x,
        y,
        width,
        height,
    }
}

/// Irregular districts rather than a single repeated housing grid: a civic
/// centre, production quarter, warehouse yards, farms, schools, barracks, and
/// residences grown around several generations of streets.
const SHOWCASE_LOTS: [ShowcaseLot; 42] = [
    lot(BuildingType::Den, -31, -19, 4, 3),
    lot(BuildingType::Beds, -24, -18, 3, 3),
    lot(BuildingType::Nursery, -17, -19, 4, 3),
    lot(BuildingType::School, -9, -19, 5, 4),
    lot(BuildingType::ResearchHut, -1, -18, 4, 3),
    lot(BuildingType::Den, 6, -19, 4, 3),
    lot(BuildingType::ElderCorner, 14, -18, 3, 3),
    lot(BuildingType::Barracks, 21, -19, 5, 4),
    lot(BuildingType::Den, 29, -17, 3, 3),
    lot(BuildingType::FoodStorage, -31, -12, 5, 4),
    lot(BuildingType::Workshop, -23, -11, 5, 4),
    lot(BuildingType::Woodworking, -15, -12, 5, 4),
    lot(BuildingType::AccountingTent, -7, -11, 4, 3),
    lot(BuildingType::Shrine, -1, -11, 5, 5),
    lot(BuildingType::Clothier, 7, -11, 5, 4),
    lot(BuildingType::Tannery, 15, -12, 5, 4),
    lot(BuildingType::FoodStorage, 23, -11, 5, 4),
    lot(BuildingType::HerbGarden, 30, -11, 3, 4),
    lot(BuildingType::Den, -30, -3, 4, 3),
    lot(BuildingType::WaterBowl, -23, -3, 3, 3),
    lot(BuildingType::Mill, -17, -3, 5, 4),
    lot(BuildingType::Sawmill, -9, -2, 5, 4),
    lot(BuildingType::Shrine, -2, -3, 5, 5),
    lot(BuildingType::Smithy, 7, -3, 5, 4),
    lot(BuildingType::Smelter, 15, -2, 5, 4),
    lot(BuildingType::StonePrep, 23, -3, 5, 4),
    lot(BuildingType::MouseFarm, 30, -3, 3, 4),
    lot(BuildingType::Beds, -31, 7, 4, 3),
    lot(BuildingType::Den, -24, 7, 3, 3),
    lot(BuildingType::WoodCutter, -18, 6, 5, 4),
    lot(BuildingType::Workshop, -10, 7, 5, 4),
    lot(BuildingType::AccountingTent, -2, 7, 5, 4),
    lot(BuildingType::Workshop, 6, 6, 5, 4),
    lot(BuildingType::FoodStorage, 14, 7, 5, 4),
    lot(BuildingType::Den, 22, 7, 4, 3),
    lot(BuildingType::Nursery, 29, 7, 3, 3),
    lot(BuildingType::Field, -30, 14, 7, 5),
    lot(BuildingType::Den, -20, 15, 4, 3),
    lot(BuildingType::Field, -13, 14, 7, 5),
    lot(BuildingType::Den, -3, 15, 4, 3),
    lot(BuildingType::Field, 5, 14, 8, 5),
    lot(BuildingType::Field, 17, 14, 9, 5),
];

const SHOWCASE_CAT_ROUTES: [[(i32, i32); 6]; 12] = [
    [
        (-31, -14),
        (-16, -14),
        (-2, -14),
        (14, -14),
        (30, -14),
        (-2, -14),
    ],
    [(-31, -5), (-18, -5), (-4, -5), (11, -5), (30, -5), (-4, -5)],
    [(-31, 4), (-16, 4), (0, 4), (16, 4), (31, 4), (0, 4)],
    [(-31, 13), (-17, 13), (-2, 13), (14, 13), (30, 13), (-2, 13)],
    [
        (-27, -18),
        (-27, -8),
        (-27, 1),
        (-27, 10),
        (-27, 18),
        (-27, 1),
    ],
    [
        (-19, -18),
        (-19, -9),
        (-19, 0),
        (-19, 9),
        (-19, 18),
        (-19, 0),
    ],
    [
        (-11, -18),
        (-11, -9),
        (-11, 0),
        (-11, 9),
        (-11, 18),
        (-11, 0),
    ],
    [(-3, -18), (-3, -9), (-3, 0), (-3, 9), (-3, 18), (-3, 0)],
    [(6, -18), (6, -9), (6, 0), (6, 9), (6, 18), (6, 0)],
    [(15, -18), (15, -9), (15, 0), (15, 9), (15, 18), (15, 0)],
    [(24, -18), (24, -9), (24, 0), (24, 9), (24, 18), (24, 0)],
    [(-29, -17), (-14, -14), (-3, -5), (9, 4), (25, 13), (-3, -5)],
];

pub(super) fn landing_showcase_camera_center() -> Vec2 {
    // Moving the camera left places the showcase centre to the right of the
    // charter while retaining enough western district behind the parchment.
    grid_to_world(LANDING_SHOWCASE_ANCHOR.x - 8, LANDING_SHOWCASE_ANCHOR.y)
}

fn showcase_tile(x: i32, y: i32) -> TilePoint {
    TilePoint {
        x: LANDING_SHOWCASE_ANCHOR.x + x,
        y: LANDING_SHOWCASE_ANCHOR.y + y,
    }
}

fn showcase_base(x: i32, y: i32) -> Vec2 {
    body_base(LANDING_SHOWCASE_ANCHOR.x + x, LANDING_SHOWCASE_ANCHOR.y + y)
}

fn showcase_ysort_z(base_world_y: f32) -> f32 {
    let origin = grid_to_world(LANDING_SHOWCASE_ANCHOR.x, LANDING_SHOWCASE_ANCHOR.y).y;
    SHOWCASE_YSORT_BASE - (base_world_y - origin) * 0.08
}

fn add_wavy_horizontal(roads: &mut HashSet<(i32, i32)>, x1: i32, x2: i32, base_y: i32, phase: i32) {
    let mut previous_y = base_y;
    for x in x1..=x2 {
        let y = base_y + ((x + phase).div_euclid(7)).rem_euclid(3) - 1;
        roads.insert((x, y));
        if y != previous_y {
            roads.insert((x, previous_y));
        }
        previous_y = y;
    }
}

fn add_wavy_vertical(roads: &mut HashSet<(i32, i32)>, y1: i32, y2: i32, base_x: i32, phase: i32) {
    let mut previous_x = base_x;
    for y in y1..=y2 {
        let x = base_x + ((y + phase).div_euclid(6)).rem_euclid(3) - 1;
        roads.insert((x, y));
        if x != previous_x {
            roads.insert((previous_x, y));
        }
        previous_x = x;
    }
}

fn showcase_roads() -> HashSet<(i32, i32)> {
    let mut roads = HashSet::new();
    for (base_y, phase) in [(-14, 1), (-5, 4), (4, 0), (13, 3)] {
        add_wavy_horizontal(&mut roads, -32, 32, base_y, phase);
    }
    for (base_x, phase) in [
        (-27, 2),
        (-19, 5),
        (-11, 1),
        (-3, 4),
        (6, 0),
        (15, 3),
        (24, 6),
    ] {
        add_wavy_vertical(&mut roads, -19, 19, base_x, phase);
    }
    // A broad, busy market crossing and two late-grown diagonal shortcuts.
    for x in -6..=6 {
        roads.insert((x, 0));
        roads.insert((x, 1));
    }
    for step in 0..=18 {
        roads.insert((-31 + step, -18 + step / 2));
        roads.insert((31 - step, 18 - step / 2));
    }
    roads
}

pub(super) fn spawn_landing_showcase(
    commands: &mut Commands,
    terrain: &TerrainArt,
    buildings: &BuildingArt,
    infra: &InfraArt,
    sheets: &SpriteSheets,
) {
    spawn_showcase_ground(commands, terrain);
    let roads = showcase_roads();
    spawn_showcase_roads(commands, infra, &roads);
    spawn_showcase_walls(commands, infra);

    for (index, lot) in SHOWCASE_LOTS.iter().enumerate() {
        spawn_showcase_lot(commands, buildings, *lot, index);
    }
    spawn_showcase_storage_yards(commands, buildings);
    spawn_showcase_orchards(commands, terrain);
    spawn_showcase_cats(commands, sheets);
}

fn spawn_showcase_ground(commands: &mut Commands, terrain: &TerrainArt) {
    for y in -SHOWCASE_HALF_HEIGHT..=SHOWCASE_HALF_HEIGHT {
        for x in -SHOWCASE_HALF_WIDTH..=SHOWCASE_HALF_WIDTH {
            let edge = x.abs() > SHOWCASE_HALF_WIDTH - 3 || y.abs() > SHOWCASE_HALF_HEIGHT - 3;
            let texture = if edge && (x * 13 + y * 7).rem_euclid(5) == 0 {
                GroundTexture::FlowersWhite
            } else if (x * 19 + y * 11).rem_euclid(17) == 0 {
                GroundTexture::GrassVar
            } else {
                GroundTexture::Grass
            };
            let tile = showcase_tile(x, y);
            let p = grid_to_world(tile.x, tile.y);
            commands.spawn((
                Sprite {
                    image: terrain.ground(texture),
                    custom_size: Some(Vec2::splat(TILE)),
                    ..default()
                },
                Transform::from_xyz(p.x, p.y, SHOWCASE_GROUND_Z),
                LandingShowcaseVisual,
            ));
        }
    }
}

fn spawn_showcase_roads(commands: &mut Commands, infra: &InfraArt, roads: &HashSet<(i32, i32)>) {
    for &(x, y) in roads {
        let visual = road_visual_at(roads, x, y);
        let tile = showcase_tile(x, y);
        let p = grid_to_world(tile.x, tile.y);
        commands.spawn((
            Sprite {
                image: infra.road(visual.sprite),
                custom_size: Some(Vec2::splat(TILE)),
                color: if (x * 5 + y * 3).rem_euclid(11) < 3 {
                    Color::srgb(0.58, 0.43, 0.29)
                } else {
                    Color::srgb(0.44, 0.47, 0.45)
                },
                ..default()
            },
            Transform::from_xyz(p.x, p.y, SHOWCASE_ROAD_Z).with_rotation(visual.rotation()),
            LandingShowcaseVisual,
        ));
    }
}

fn spawn_showcase_walls(commands: &mut Commands, infra: &InfraArt) {
    for x in -SHOWCASE_HALF_WIDTH..=SHOWCASE_HALF_WIDTH {
        if (-2..=2).contains(&x) {
            continue;
        }
        for y in [-SHOWCASE_HALF_HEIGHT, SHOWCASE_HALF_HEIGHT] {
            let tile = showcase_tile(x, y);
            let p = grid_to_world(tile.x, tile.y);
            commands.spawn((
                Sprite {
                    image: infra.palisade.clone(),
                    custom_size: Some(Vec2::splat(TILE * 1.12)),
                    ..default()
                },
                Anchor::BOTTOM_CENTER,
                Transform::from_xyz(p.x, p.y, showcase_ysort_z(p.y) + 0.3),
                LandingShowcaseVisual,
            ));
        }
    }
    for y in (-SHOWCASE_HALF_HEIGHT + 1)..SHOWCASE_HALF_HEIGHT {
        if (1..=5).contains(&y) {
            continue;
        }
        for x in [-SHOWCASE_HALF_WIDTH, SHOWCASE_HALF_WIDTH] {
            let tile = showcase_tile(x, y);
            let p = grid_to_world(tile.x, tile.y);
            commands.spawn((
                Sprite {
                    image: infra.palisade.clone(),
                    custom_size: Some(Vec2::splat(TILE * 1.12)),
                    ..default()
                },
                Anchor::BOTTOM_CENTER,
                Transform::from_xyz(p.x, p.y, showcase_ysort_z(p.y) + 0.3),
                LandingShowcaseVisual,
            ));
        }
    }
    for (x, y) in [(0, SHOWCASE_HALF_HEIGHT), (-SHOWCASE_HALF_WIDTH, 3)] {
        let tile = showcase_tile(x, y);
        let p = grid_to_world(tile.x, tile.y);
        commands.spawn((
            Sprite {
                image: infra.gate.clone(),
                custom_size: Some(Vec2::splat(TILE * 2.6)),
                ..default()
            },
            Anchor::BOTTOM_CENTER,
            Transform::from_xyz(p.x, p.y, showcase_ysort_z(p.y) + 0.8),
            LandingShowcaseVisual,
        ));
    }
}

fn spawn_showcase_lot(commands: &mut Commands, art: &BuildingArt, lot: ShowcaseLot, index: usize) {
    let nw = showcase_tile(lot.x, lot.y);
    let footprint = FootprintSize {
        width: lot.width,
        height: lot.height,
    };
    match building_visual(lot.kind) {
        BuildingVisual::Infrastructure => {}
        BuildingVisual::Roofed(facade) => {
            let layout = building_render_layout(nw, footprint);
            let tint = match index % 4 {
                0 => Color::srgb(1.0, 0.94, 0.82),
                1 => Color::srgb(0.92, 0.86, 0.76),
                2 => Color::srgb(0.88, 0.78, 0.66),
                _ => Color::WHITE,
            };
            commands.spawn((
                Sprite {
                    image: art.facade(facade),
                    color: tint,
                    custom_size: Some(layout.facade_size),
                    ..default()
                },
                Anchor::BOTTOM_CENTER,
                Transform::from_xyz(
                    layout.facade_base.x,
                    layout.facade_base.y,
                    showcase_ysort_z(layout.facade_base.y) + 0.4,
                ),
                LandingShowcaseVisual,
            ));
        }
        BuildingVisual::Open(station) => {
            spawn_showcase_open_station(commands, art, nw, footprint, station);
        }
    }
}

fn spawn_showcase_open_station(
    commands: &mut Commands,
    art: &BuildingArt,
    nw: TilePoint,
    footprint: FootprintSize,
    station: &StationLayout,
) {
    for dy in 0..footprint.height.max(1) {
        for dx in 0..footprint.width.max(1) {
            let center = grid_to_world(nw.x + dx, nw.y + dy);
            commands.spawn((
                Sprite {
                    image: art.floor(station.floor),
                    custom_size: Some(Vec2::splat(TILE)),
                    ..default()
                },
                Transform::from_xyz(center.x, center.y, SHOWCASE_FLOOR_Z),
                LandingShowcaseVisual,
            ));
        }
    }
    for placement in station.props {
        let geometry = station_prop_geometry(nw, footprint, *placement);
        commands.spawn((
            Sprite {
                image: art.prop(placement.prop),
                custom_size: Some(geometry.size),
                ..default()
            },
            Transform::from_xyz(
                geometry.center.x,
                geometry.center.y,
                showcase_ysort_z(geometry.base_y) + 0.5,
            ),
            LandingShowcaseVisual,
        ));
    }
}

fn spawn_showcase_storage_yards(commands: &mut Commands, art: &BuildingArt) {
    let yards = [
        (-31, 1, StationProp::Crate),
        (-14, 1, StationProp::LogPile),
        (19, 1, StationProp::OrePile),
        (27, 10, StationProp::Barrel),
    ];
    for (x0, y0, primary) in yards {
        for y in 0..3 {
            for x in 0..5 {
                let tile = showcase_tile(x0 + x, y0 + y);
                let p = grid_to_world(tile.x, tile.y);
                let prop = if (x + y) % 3 == 0 {
                    StationProp::Sack
                } else {
                    primary
                };
                commands.spawn((
                    Sprite {
                        image: art.prop(prop),
                        custom_size: Some(Vec2::splat(TILE * 0.72)),
                        ..default()
                    },
                    Transform::from_xyz(p.x, p.y, showcase_ysort_z(p.y - TILE * 0.45) + 0.6),
                    LandingShowcaseVisual,
                ));
            }
        }
    }
}

fn spawn_showcase_orchards(commands: &mut Commands, terrain: &TerrainArt) {
    let orchard_positions = [
        (-31, -18),
        (-28, -18),
        (-25, -18),
        (27, -18),
        (30, -18),
        (32, -16),
        (-32, 17),
        (-29, 18),
        (28, 17),
        (31, 18),
        (32, 14),
        (-32, -8),
        (32, -7),
    ];
    let (image, scale) = terrain.tree(TreeSprite::Oak);
    for (x, y) in orchard_positions {
        let tile = showcase_tile(x, y);
        let p = grid_to_world(tile.x, tile.y);
        commands.spawn((
            Sprite {
                image: image.clone(),
                custom_size: Some(Vec2::splat(TILE * scale)),
                ..default()
            },
            Anchor::BOTTOM_CENTER,
            Transform::from_xyz(p.x, p.y, showcase_ysort_z(p.y) + 0.7),
            LandingShowcaseVisual,
        ));
    }
}

fn spawn_showcase_cats(commands: &mut Commands, sheets: &SpriteSheets) {
    for index in 0..SHOWCASE_CAT_COUNT {
        let route = index % SHOWCASE_CAT_ROUTES.len();
        let waypoint =
            (index / SHOWCASE_CAT_ROUTES.len() + route * 2) % SHOWCASE_CAT_ROUTES[route].len();
        let (x, y) = SHOWCASE_CAT_ROUTES[route][waypoint];
        let start = showcase_base(x, y);
        let group = index % 8;
        let mut entity = commands.spawn((
            Sprite {
                image: sheets.cat.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: sheets.layout.clone(),
                    index: atlas_index(group, 0),
                }),
                custom_size: Some(CAT_SIZE),
                color: match index % 5 {
                    0 => Color::srgb(1.0, 0.88, 0.78),
                    1 => Color::srgb(0.88, 0.92, 1.0),
                    2 => Color::srgb(0.98, 0.82, 0.58),
                    3 => Color::srgb(0.82, 0.78, 0.72),
                    _ => Color::WHITE,
                },
                ..default()
            },
            Anchor::BOTTOM_CENTER,
            Transform::from_xyz(start.x, start.y, showcase_ysort_z(start.y) + 1.0),
            AnimSprite {
                group,
                moving: true,
                phase: index % 4,
            },
            LandingShowcaseCat {
                route,
                waypoint: (waypoint + 1) % SHOWCASE_CAT_ROUTES[route].len(),
            },
            LandingShowcaseVisual,
        ));
        if index % 4 == 0 {
            let hat = match index % 16 {
                0 => sheets.hat_hunter.clone(),
                4 => sheets.hat_architect.clone(),
                8 => sheets.hat_ritualist.clone(),
                _ => sheets.hat_warrior.clone(),
            };
            entity.with_child((
                Sprite {
                    image: hat,
                    custom_size: Some(CAT_SIZE),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, 0.2),
                LandingShowcaseVisual,
            ));
        }
    }
}

pub(super) fn animate_landing_showcase(
    time: Res<Time>,
    start: Res<StartScreen>,
    mut cats: Query<(&mut Transform, &mut AnimSprite, &mut LandingShowcaseCat)>,
) {
    for (mut transform, mut anim, mut cat) in &mut cats {
        if !start.visible {
            anim.moving = false;
            continue;
        }
        let route = SHOWCASE_CAT_ROUTES[cat.route];
        let (x, y) = route[cat.waypoint];
        let target = showcase_base(x, y);
        let current = transform.translation.truncate();
        if current.distance(target) < TILE * 0.35 {
            cat.waypoint = (cat.waypoint + 1) % route.len();
            continue;
        }
        let delta = target - current;
        if let Some(group) = facing_from_delta(delta) {
            anim.group = group;
        }
        let step = BODY_WALK_SPEED * 0.72 * time.delta_secs();
        let next = if delta.length() <= step {
            target
        } else {
            current + delta.normalize() * step
        };
        transform.translation.x = next.x;
        transform.translation.y = next.y;
        transform.translation.z = showcase_ysort_z(next.y) + 1.0;
        anim.moving = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn showcase_reads_as_a_mature_multi_district_village() {
        let building_kinds = SHOWCASE_LOTS
            .iter()
            .map(|lot| building_label(lot.kind))
            .collect::<HashSet<_>>();
        assert!(SHOWCASE_LOTS.len() >= 40);
        assert!(building_kinds.len() >= 18);
        assert!(showcase_roads().len() >= 400);
    }

    #[test]
    fn showcase_fills_the_fixed_landing_overview() {
        let width = SHOWCASE_HALF_WIDTH * 2 + 1;
        let height = SHOWCASE_HALF_HEIGHT * 2 + 1;
        assert!(width >= 64);
        assert!(height >= 40);
        assert!(width as f32 <= LANDING_OVERVIEW_MIN_TILES);
    }

    #[test]
    fn showcase_routes_stay_inside_the_walled_scene() {
        for route in SHOWCASE_CAT_ROUTES {
            for (x, y) in route {
                assert!(x.abs() < SHOWCASE_HALF_WIDTH);
                assert!(y.abs() < SHOWCASE_HALF_HEIGHT);
            }
        }
    }
}
