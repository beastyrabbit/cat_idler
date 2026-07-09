use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::remote::http::RemoteHttpPlugin;
use bevy::remote::RemotePlugin;
use bevy::sprite::Anchor;
use bevy::window::WindowResolution;

const MAP_W: i32 = 144;
const MAP_H: i32 = 144;
const TILE_W: f32 = 64.0;
const TILE_H: f32 = 32.0;
const ISO_SPRITE_W: f32 = 64.0;
const ISO_SPRITE_H: f32 = 128.0;
const ISO_SPRITE_ANCHOR_Y: f32 = -0.34375;
const CAT_SPRITE_SIZE: f32 = 34.0;
const CAT_WALK_FPS: f32 = 7.0;
const CAT_WORK_SPIN_FPS: f32 = 13.0;
const ROAD_WEAR_THRESHOLD: f32 = 70.0;
const CENTER_X: i32 = MAP_W / 2;
const CENTER_Y: i32 = MAP_H / 2;
const SEED: f32 = 1847.0;
const PALISADE_RADIUS: i32 = 10;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.07, 0.10, 0.09)))
        .insert_resource(ColonyState::default())
        .insert_resource(MapData::new())
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: ".".to_string(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Cat Idler - Bevy Engine Prototype".to_string(),
                        resolution: WindowResolution::new(1280, 800),
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(RemotePlugin::default())
        .add_plugins(RemoteHttpPlugin::default())
        .register_type::<Cat>()
        .register_type::<Role>()
        .register_type::<Job>()
        .register_type::<CatCoat>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                camera_controls,
                pointer_controls,
                stress_controls,
                simulation_tick,
                update_tile_visuals,
                update_cat_transforms,
                update_raiders,
                update_ui,
            ),
        )
        .run();
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TileKind {
    Grass,
    Clearing,
    Forest,
    Water,
    Stone,
    Highland,
    Path,
    Fence,
    Gate,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TileMarker {
    Priority,
    Avoid,
    RoadPlan,
    BuildPlan,
}

#[derive(Clone)]
struct Tile {
    x: i32,
    y: i32,
    kind: TileKind,
    height: i32,
    wear: f32,
    marker: Option<TileMarker>,
}

#[derive(Resource)]
struct MapData {
    tiles: Vec<Tile>,
}

impl MapData {
    fn new() -> Self {
        let mut tiles = Vec::with_capacity((MAP_W * MAP_H) as usize);
        for y in 0..MAP_H {
            for x in 0..MAP_W {
                let dx = x - CENTER_X;
                let dy = y - CENTER_Y;
                let dist = dx.abs().max(dy.abs());
                let n = hash2((x / 3) as f32, (y / 3) as f32, 4.0);
                let ridge = (x + y) as f32 / (MAP_W + MAP_H) as f32;
                let river_band =
                    ((x - y) as f32 + 14.0 + (hash2(x as f32, y as f32, 7.0) * 3.0).floor() - 1.0)
                        .abs();

                let mut kind = if dist <= 7 {
                    TileKind::Clearing
                } else if river_band <= 1.0 && x > 9 && y < MAP_H - 8 {
                    TileKind::Water
                } else if n > 0.82 || dist > 43 {
                    if ridge > 0.62 {
                        TileKind::Stone
                    } else {
                        TileKind::Highland
                    }
                } else if n > 0.57 {
                    TileKind::Forest
                } else {
                    TileKind::Grass
                };

                if dist == PALISADE_RADIUS && dx.abs() + dy.abs() > 2 {
                    kind = if x == CENTER_X && y == CENTER_Y - PALISADE_RADIUS {
                        TileKind::Gate
                    } else {
                        TileKind::Fence
                    };
                }

                tiles.push(Tile {
                    x,
                    y,
                    kind,
                    height: 0,
                    wear: 0.0,
                    marker: None,
                });
            }
        }
        Self { tiles }
    }

    fn idx(x: i32, y: i32) -> Option<usize> {
        if (0..MAP_W).contains(&x) && (0..MAP_H).contains(&y) {
            Some((y * MAP_W + x) as usize)
        } else {
            None
        }
    }

    fn get(&self, x: i32, y: i32) -> Option<&Tile> {
        Self::idx(x, y).and_then(|idx| self.tiles.get(idx))
    }

    fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut Tile> {
        Self::idx(x, y).and_then(|idx| self.tiles.get_mut(idx))
    }

    fn is_walkable(&self, x: i32, y: i32) -> bool {
        self.get(x, y)
            .map(|tile| !matches!(tile.kind, TileKind::Water | TileKind::Fence))
            .unwrap_or(false)
    }
}

#[derive(Resource)]
struct ColonyState {
    food: f32,
    water: f32,
    materials: f32,
    blessings: f32,
    herbs: f32,
    research: f32,
    threat: f32,
    elapsed: f32,
    time_scale: f32,
    paused: bool,
    spawn_serial: usize,
    hover: Option<IVec2>,
    selected_cat: Option<Entity>,
    tool_mode: ToolMode,
}

impl Default for ColonyState {
    fn default() -> Self {
        Self {
            food: 70.0,
            water: 75.0,
            materials: 25.0,
            blessings: 0.0,
            herbs: 8.0,
            research: 0.0,
            threat: 0.0,
            elapsed: 0.0,
            time_scale: 1.0,
            paused: false,
            spawn_serial: 0,
            hover: None,
            selected_cat: None,
            tool_mode: ToolMode::Inspect,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToolMode {
    Inspect,
    Priority,
    Avoid,
    Road,
    Build,
}

#[derive(Clone, Copy, PartialEq, Eq, Reflect, Default)]
enum Role {
    #[default]
    Hunter,
    Water,
    Quarry,
    Builder,
    Scout,
    Ritualist,
    Warrior,
    Rest,
}

#[derive(Clone, Copy, PartialEq, Eq, Reflect, Default)]
enum Job {
    Hunt,
    FetchWater,
    Quarry,
    Build,
    Scout,
    Ritual,
    Guard,
    #[default]
    Rest,
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Cat {
    id: usize,
    name: String,
    coat: CatCoat,
    role: Role,
    job: Job,
    tile_pos: Vec2,
    target: IVec2,
    energy: f32,
    hunger: f32,
    thirst: f32,
    work_timer: f32,
}

#[derive(Clone, Copy, Reflect, Default)]
enum CatCoat {
    #[default]
    Black,
    Calico,
    GrayTabby,
    OrangeTabby,
    Tuxedo,
    White,
}

#[derive(Component)]
struct CatVisual {
    facing_group: usize,
}

#[derive(Component)]
struct CatRoleRing {
    cat_id: usize,
}

#[derive(Component)]
struct Raider {
    tile_pos: Vec2,
    hp: f32,
}

#[derive(Component)]
struct TileSprite {
    x: i32,
    y: i32,
}

#[derive(Component)]
struct HudPanel {
    kind: HudPanelKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HudPanelKind {
    Resources,
    Jobs,
    Selection,
    Toolbar,
}

#[derive(Resource, Clone)]
struct ArtHandles {
    grass: Handle<Image>,
    clearing: Handle<Image>,
    water: Handle<Image>,
    stone: Handle<Image>,
    highland: Handle<Image>,
    tree_small: Handle<Image>,
    tree_large: Handle<Image>,
    tree_huge: Handle<Image>,
    fence_x: Handle<Image>,
    fence_y: Handle<Image>,
    gate: Handle<Image>,
    road: Handle<Image>,
    road_built: Handle<Image>,
    path_crossing: Handle<Image>,
    path_straight_n: Handle<Image>,
    path_straight_e: Handle<Image>,
    path_corner_n: Handle<Image>,
    path_corner_e: Handle<Image>,
    path_corner_s: Handle<Image>,
    path_corner_w: Handle<Image>,
    path_end_n: Handle<Image>,
    path_end_e: Handle<Image>,
    path_end_s: Handle<Image>,
    path_end_w: Handle<Image>,
    cat_sheet: Handle<Image>,
    cat_layout: Handle<TextureAtlasLayout>,
    badger: Handle<Image>,
    fox: Handle<Image>,
    bear: Handle<Image>,
    rival_cat: Handle<Image>,
    hawk: Handle<Image>,
    shrine: Handle<Image>,
    den: Handle<Image>,
    food_storage: Handle<Image>,
    water_bowl: Handle<Image>,
    beds: Handle<Image>,
    workshop: Handle<Image>,
    field: Handle<Image>,
    nursery: Handle<Image>,
    herb_garden: Handle<Image>,
    walls: Handle<Image>,
}

impl ArtHandles {
    fn load(
        asset_server: &AssetServer,
        texture_atlas_layouts: &mut Assets<TextureAtlasLayout>,
    ) -> Self {
        let cat_layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(32),
            32,
            2,
            None,
            None,
        ));
        Self {
            grass: asset_server.load("public/images/iso/tiles/grass.png"),
            clearing: asset_server.load("public/images/iso/tiles/grass-clearing.png"),
            water: asset_server.load("public/images/iso/tiles/water.png"),
            stone: asset_server.load("public/images/iso/tiles/grass-stone-large.png"),
            highland: asset_server.load("public/images/iso/tiles/grass-hill.png"),
            tree_small: asset_server.load("public/images/iso/tiles/tree-pine-small.png"),
            tree_large: asset_server.load("public/images/iso/tiles/tree-pine-large.png"),
            tree_huge: asset_server.load("public/images/iso/tiles/tree-pine-huge.png"),
            fence_x: asset_server.load("public/images/iso/tiles/fence-x.png"),
            fence_y: asset_server.load("public/images/iso/tiles/fence-y.png"),
            gate: asset_server.load("public/images/iso/tiles/gate.png"),
            road: asset_server.load("public/images/iso/tiles/road.png"),
            road_built: asset_server.load("public/images/iso/tiles/road-built.png"),
            path_crossing: asset_server.load("public/images/iso/tiles/path-crossing.png"),
            path_straight_n: asset_server.load("public/images/iso/tiles/path-straight-n.png"),
            path_straight_e: asset_server.load("public/images/iso/tiles/path-straight-e.png"),
            path_corner_n: asset_server.load("public/images/iso/tiles/path-corner-n.png"),
            path_corner_e: asset_server.load("public/images/iso/tiles/path-corner-e.png"),
            path_corner_s: asset_server.load("public/images/iso/tiles/path-corner-s.png"),
            path_corner_w: asset_server.load("public/images/iso/tiles/path-corner-w.png"),
            path_end_n: asset_server.load("public/images/iso/tiles/path-end-n.png"),
            path_end_e: asset_server.load("public/images/iso/tiles/path-end-e.png"),
            path_end_s: asset_server.load("public/images/iso/tiles/path-end-s.png"),
            path_end_w: asset_server.load("public/images/iso/tiles/path-end-w.png"),
            cat_sheet: asset_server.load("public/images/cats/cat-sheet.png"),
            cat_layout,
            badger: asset_server.load("public/images/enemies/badger.png"),
            fox: asset_server.load("public/images/enemies/fox.png"),
            bear: asset_server.load("public/images/enemies/bear.png"),
            rival_cat: asset_server.load("public/images/enemies/rival_cat.png"),
            hawk: asset_server.load("public/images/enemies/hawk.png"),
            shrine: asset_server.load("public/images/iso/buildings/shrine.png"),
            den: asset_server.load("public/images/iso/buildings/den.png"),
            food_storage: asset_server.load("public/images/iso/buildings/food-storage.png"),
            water_bowl: asset_server.load("public/images/iso/buildings/water-bowl.png"),
            beds: asset_server.load("public/images/iso/buildings/beds.png"),
            workshop: asset_server.load("public/images/iso/buildings/workshop.png"),
            field: asset_server.load("public/images/iso/buildings/field.png"),
            nursery: asset_server.load("public/images/iso/buildings/nursery.png"),
            herb_garden: asset_server.load("public/images/iso/buildings/herb-garden.png"),
            walls: asset_server.load("public/images/iso/buildings/walls.png"),
        }
    }
}

struct BuildingSite {
    name: &'static str,
    kind: BuildingKind,
    x: i32,
    y: i32,
}

#[derive(Clone, Copy)]
enum BuildingKind {
    Shrine,
    Den,
    FoodStorage,
    WaterBowl,
    Beds,
    Workshop,
    Field,
    Nursery,
    HerbGarden,
    Walls,
}

const NAMES: [&str; 40] = [
    "Acorn", "Ash", "Basil", "Bean", "Bramble", "Button", "Cinder", "Clove", "Cricket", "Daisy",
    "Fern", "Fig", "Hazel", "Juniper", "Maple", "Miso", "Mochi", "Moss", "Nettle", "Olive",
    "Pebble", "Pepper", "Pip", "Poppy", "Rook", "Saffron", "Sage", "Sprout", "Tansy", "Thistle",
    "Toast", "Willow", "Yarrow", "Mallow", "Sorrel", "Quince", "Briar", "Fennel", "Minnow",
    "Clover",
];

fn setup(
    mut commands: Commands,
    mut colony: ResMut<ColonyState>,
    map: Res<MapData>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let art = ArtHandles::load(&asset_server, &mut texture_atlas_layouts);
    let center = iso_point(CENTER_X as f32, CENTER_Y as f32, 0);
    commands.spawn((
        Camera2d,
        Transform::from_xyz(center.x, center.y + 80.0, 1000.0),
    ));

    for tile in &map.tiles {
        let p = iso_point(tile.x as f32, tile.y as f32, tile.height);
        commands.spawn((
            Sprite {
                image: ground_texture(tile, &map, &art),
                color: tile_tint(tile, None),
                custom_size: Some(Vec2::new(ISO_SPRITE_W, ISO_SPRITE_H)),
                ..default()
            },
            Anchor(Vec2::new(0.0, ISO_SPRITE_ANCHOR_Y)),
            Transform::from_xyz(p.x, p.y, tile_depth(tile.x, tile.y)),
            TileSprite {
                x: tile.x,
                y: tile.y,
            },
        ));

        if let Some(texture) = overlay_texture(tile, &art) {
            commands.spawn((
                Sprite {
                    image: texture,
                    custom_size: Some(Vec2::new(ISO_SPRITE_W, ISO_SPRITE_H)),
                    ..default()
                },
                Anchor(Vec2::new(0.0, ISO_SPRITE_ANCHOR_Y)),
                Transform::from_xyz(p.x, p.y, 180.0 + (tile.x + tile.y) as f32 * 0.01),
            ));
        }
    }

    for site in building_sites() {
        spawn_building(&mut commands, &map, &art, site);
    }

    for _ in 0..96 {
        spawn_cat(&mut commands, &mut colony, &map, &art, None);
    }

    spawn_hud_panel(
        &mut commands,
        HudPanelKind::Resources,
        14.0,
        14.0,
        410.0,
        156.0,
        15.0,
    );
    spawn_hud_panel(
        &mut commands,
        HudPanelKind::Jobs,
        14.0,
        184.0,
        300.0,
        216.0,
        14.0,
    );
    spawn_hud_panel(
        &mut commands,
        HudPanelKind::Selection,
        936.0,
        14.0,
        330.0,
        168.0,
        14.0,
    );
    spawn_hud_panel(
        &mut commands,
        HudPanelKind::Toolbar,
        326.0,
        706.0,
        628.0,
        78.0,
        14.0,
    );

    commands.insert_resource(art);
}

fn spawn_hud_panel(
    commands: &mut Commands,
    kind: HudPanelKind,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    font_size: f32,
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                top: Val::Px(top),
                width: Val::Px(width),
                height: Val::Px(height),
                padding: UiRect::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.04, 0.035, 0.76)),
            BorderColor::all(Color::srgba(0.80, 0.67, 0.42, 0.38)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(font_size),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.95, 0.84)),
                HudPanel { kind },
            ));
        });
}

fn building_sites() -> [BuildingSite; 10] {
    [
        BuildingSite {
            name: "Moonlit Shrine",
            kind: BuildingKind::Shrine,
            x: CENTER_X,
            y: CENTER_Y,
        },
        BuildingSite {
            name: "South Den",
            kind: BuildingKind::Den,
            x: CENTER_X - 4,
            y: CENTER_Y + 2,
        },
        BuildingSite {
            name: "Food Storage",
            kind: BuildingKind::FoodStorage,
            x: CENTER_X + 4,
            y: CENTER_Y + 1,
        },
        BuildingSite {
            name: "Water Bowls",
            kind: BuildingKind::WaterBowl,
            x: CENTER_X + 2,
            y: CENTER_Y - 4,
        },
        BuildingSite {
            name: "Warrior Beds",
            kind: BuildingKind::Beds,
            x: CENTER_X - 5,
            y: CENTER_Y - 3,
        },
        BuildingSite {
            name: "Workshop",
            kind: BuildingKind::Workshop,
            x: CENTER_X + 6,
            y: CENTER_Y - 3,
        },
        BuildingSite {
            name: "Mouse Field",
            kind: BuildingKind::Field,
            x: CENTER_X + 6,
            y: CENTER_Y + 5,
        },
        BuildingSite {
            name: "Nursery",
            kind: BuildingKind::Nursery,
            x: CENTER_X - 1,
            y: CENTER_Y + 6,
        },
        BuildingSite {
            name: "Herb Garden",
            kind: BuildingKind::HerbGarden,
            x: CENTER_X - 7,
            y: CENTER_Y + 5,
        },
        BuildingSite {
            name: "Palisade Stores",
            kind: BuildingKind::Walls,
            x: CENTER_X - 7,
            y: CENTER_Y - 6,
        },
    ]
}

fn spawn_building(commands: &mut Commands, map: &MapData, art: &ArtHandles, site: BuildingSite) {
    let Some(tile) = map.get(site.x, site.y) else {
        return;
    };
    let p = iso_point(site.x as f32, site.y as f32, tile.height);
    commands.spawn((
        Sprite {
            image: building_texture(site.kind, art),
            custom_size: Some(Vec2::new(ISO_SPRITE_W, ISO_SPRITE_H)),
            ..default()
        },
        Anchor(Vec2::new(0.0, ISO_SPRITE_ANCHOR_Y)),
        Transform::from_xyz(p.x, p.y + 4.0, 220.0 + (site.x + site.y) as f32 * 0.01),
    ));
    commands.spawn((
        Text2d::new(site.name),
        TextFont {
            font_size: FontSize::Px(10.0),
            ..default()
        },
        TextColor(Color::srgba(1.0, 0.95, 0.82, 0.82)),
        Transform::from_xyz(p.x, p.y + 52.0, 820.0),
    ));
}

fn spawn_cat(
    commands: &mut Commands,
    colony: &mut ColonyState,
    map: &MapData,
    art: &ArtHandles,
    role: Option<Role>,
) {
    colony.spawn_serial += 1;
    let roles = [
        Role::Hunter,
        Role::Water,
        Role::Quarry,
        Role::Builder,
        Role::Scout,
        Role::Ritualist,
        Role::Warrior,
        Role::Rest,
    ];
    let picked_role = role.unwrap_or(roles[(colony.spawn_serial - 1) % roles.len()]);
    let coat = coat_for_id(colony.spawn_serial);
    let (x, y) = random_walkable_tile(map, colony.spawn_serial as f32 * 31.0, true);
    let mut cat = Cat {
        id: colony.spawn_serial,
        name: format!(
            "{} {}",
            NAMES[(colony.spawn_serial - 1) % NAMES.len()],
            colony.spawn_serial
        ),
        coat,
        role: picked_role,
        job: Job::Rest,
        tile_pos: Vec2::new(x as f32, y as f32),
        target: IVec2::new(x, y),
        energy: 75.0 + hash2(colony.spawn_serial as f32, 1.0, 6.0) * 25.0,
        hunger: 25.0 + hash2(colony.spawn_serial as f32, 2.0, 6.0) * 25.0,
        thirst: 25.0 + hash2(colony.spawn_serial as f32, 3.0, 6.0) * 25.0,
        work_timer: 0.0,
    };
    assign_job(&mut cat, colony, map);
    let tile = map.get(x, y).unwrap();
    let p = iso_point(cat.tile_pos.x, cat.tile_pos.y, tile.height);
    commands.spawn((
        Sprite {
            color: role_tint(cat.role),
            custom_size: Some(Vec2::new(10.0, 4.0)),
            ..default()
        },
        Transform::from_xyz(p.x, p.y + 3.0, 498.0 + (x + y) as f32 * 0.01),
        CatRoleRing { cat_id: cat.id },
    ));

    commands.spawn((
        Sprite {
            image: art.cat_sheet.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: art.cat_layout.clone(),
                index: 0,
            }),
            custom_size: Some(Vec2::splat(CAT_SPRITE_SIZE)),
            color: Color::WHITE,
            ..default()
        },
        Anchor::BOTTOM_CENTER,
        Transform::from_xyz(p.x, p.y + 12.0, 500.0 + (x + y) as f32 * 0.01),
        CatVisual { facing_group: 0 },
        cat,
    ));
}

fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    time: Res<Time>,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    let Ok((mut transform, mut projection)) = camera.single_mut() else {
        return;
    };
    let Projection::Orthographic(projection) = projection.as_mut() else {
        return;
    };
    let speed = 620.0 * time.delta_secs() * projection.scale;
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        transform.translation.x -= speed;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        transform.translation.x += speed;
    }
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        transform.translation.y += speed;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        transform.translation.y -= speed;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        let center = iso_point(CENTER_X as f32, CENTER_Y as f32, 0);
        transform.translation.x = center.x;
        transform.translation.y = center.y + 80.0;
        projection.scale = 1.0;
    }
    if buttons.pressed(MouseButton::Middle) {
        for ev in motion.read() {
            transform.translation.x -= ev.delta.x * projection.scale;
            transform.translation.y += ev.delta.y * projection.scale;
        }
    } else {
        motion.clear();
    }
    for ev in wheel.read() {
        projection.scale = (projection.scale * if ev.y > 0.0 { 0.9 } else { 1.1 }).clamp(0.45, 2.4);
    }
}

fn pointer_controls(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mut colony: ResMut<ColonyState>,
    mut map: ResMut<MapData>,
    cats: Query<(Entity, &Cat, &Transform)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        colony.hover = None;
        return;
    };
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };
    let Ok(world) = camera.viewport_to_world_2d(camera_transform, cursor) else {
        return;
    };
    let tile = iso_to_tile(world);
    colony.hover = MapData::idx(tile.x, tile.y).map(|_| tile);

    if buttons.just_pressed(MouseButton::Left) {
        if colony.tool_mode == ToolMode::Inspect {
            colony.selected_cat = nearest_cat_at(world, &cats);
            return;
        }
        if let Some(tile_pos) = colony.hover {
            if let Some(tile) = map.get_mut(tile_pos.x, tile_pos.y) {
                match colony.tool_mode {
                    ToolMode::Inspect => {}
                    ToolMode::Priority => toggle_marker(tile, TileMarker::Priority),
                    ToolMode::Avoid => toggle_marker(tile, TileMarker::Avoid),
                    ToolMode::Road => {
                        if !matches!(tile.kind, TileKind::Water | TileKind::Fence) {
                            tile.kind = TileKind::Path;
                            tile.wear = 100.0;
                            tile.marker = Some(TileMarker::RoadPlan);
                        }
                    }
                    ToolMode::Build => toggle_marker(tile, TileMarker::BuildPlan),
                }
            }
        }
    }

    if buttons.just_pressed(MouseButton::Right) {
        colony.selected_cat = nearest_cat_at(world, &cats);
    }
}

fn nearest_cat_at(world: Vec2, cats: &Query<(Entity, &Cat, &Transform)>) -> Option<Entity> {
    let mut nearest = None;
    let mut nearest_dist = (CAT_SPRITE_SIZE * 1.35).powi(2);
    for (entity, _, transform) in cats.iter() {
        let d = transform.translation.truncate().distance_squared(world);
        if d < nearest_dist {
            nearest_dist = d;
            nearest = Some(entity);
        }
    }
    nearest
}

fn stress_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut colony: ResMut<ColonyState>,
    map: Res<MapData>,
    art: Res<ArtHandles>,
    cats: Query<Entity, With<Cat>>,
) {
    if keys.just_pressed(KeyCode::Space) {
        colony.paused = !colony.paused;
    }
    if keys.just_pressed(KeyCode::Digit1) {
        colony.tool_mode = ToolMode::Inspect;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        colony.tool_mode = ToolMode::Priority;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        colony.tool_mode = ToolMode::Avoid;
    }
    if keys.just_pressed(KeyCode::Digit4) {
        colony.tool_mode = ToolMode::Road;
    }
    if keys.just_pressed(KeyCode::Digit5) {
        colony.tool_mode = ToolMode::Build;
    }
    if keys.just_pressed(KeyCode::Tab) {
        colony.tool_mode = next_tool_mode(colony.tool_mode);
    }
    if keys.just_pressed(KeyCode::Equal) {
        colony.time_scale = (colony.time_scale + 0.5).min(8.0);
    }
    if keys.just_pressed(KeyCode::Minus) {
        colony.time_scale = (colony.time_scale - 0.5).max(0.25);
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        for _ in 0..25 {
            spawn_cat(&mut commands, &mut colony, &map, &art, None);
        }
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        for entity in cats.iter().take(25) {
            commands.entity(entity).despawn();
        }
    }
}

fn simulation_tick(
    time: Res<Time>,
    mut colony: ResMut<ColonyState>,
    mut map: ResMut<MapData>,
    mut cats: Query<&mut Cat>,
    mut commands: Commands,
    art: Res<ArtHandles>,
) {
    if colony.paused {
        return;
    }
    let dt = time.delta_secs() * colony.time_scale;
    colony.elapsed += dt;
    let cat_count = cats.iter().len() as f32;
    colony.food = (colony.food - cat_count * dt * 0.015).max(0.0);
    colony.water = (colony.water - cat_count * dt * 0.018).max(0.0);
    colony.herbs = (colony.herbs + dt * 0.02).min(80.0);
    colony.research = (colony.research + dt * 0.015).min(100.0);
    colony.threat += dt * (0.65 + cat_count / 240.0);

    if colony.threat > 100.0 {
        colony.threat = 0.0;
        for i in 0..7 {
            let side_x = if i % 2 == 0 { 3 } else { MAP_W - 4 };
            let side_y = i * MAP_H / 8;
            let p = iso_point(side_x as f32, side_y as f32, 1);
            let texture = match i % 5 {
                0 => art.fox.clone(),
                1 => art.badger.clone(),
                2 => art.rival_cat.clone(),
                3 => art.hawk.clone(),
                _ => art.bear.clone(),
            };
            commands.spawn((
                Sprite {
                    image: texture,
                    custom_size: Some(Vec2::new(42.0, 42.0)),
                    ..default()
                },
                Anchor::BOTTOM_CENTER,
                Transform::from_xyz(p.x, p.y + 14.0, 650.0),
                Raider {
                    tile_pos: Vec2::new(side_x as f32, side_y as f32),
                    hp: 15.0,
                },
            ));
        }
    }

    for mut cat in &mut cats {
        cat.energy = (cat.energy - dt * 0.7).max(0.0);
        cat.hunger = (cat.hunger + dt * 0.55).min(100.0);
        cat.thirst = (cat.thirst + dt * 0.65).min(100.0);

        if step_cat(&mut cat, &mut map, dt) {
            complete_work(&mut cat, &mut colony, &map, dt);
        }
    }
}

fn update_cat_transforms(
    time: Res<Time>,
    map: Res<MapData>,
    mut cats: Query<(&Cat, &mut CatVisual, &mut Sprite, &mut Transform)>,
    mut rings: Query<(&CatRoleRing, &mut Transform), Without<Cat>>,
) {
    let mut ring_positions = Vec::new();
    for (cat, mut visual, mut sprite, mut transform) in &mut cats {
        let x = cat.tile_pos.x.round() as i32;
        let y = cat.tile_pos.y.round() as i32;
        let height = map.get(x, y).map(|tile| tile.height).unwrap_or(0);
        let p = iso_point(cat.tile_pos.x, cat.tile_pos.y, height);
        transform.translation.x = p.x;
        transform.translation.y = p.y + 12.0;
        transform.translation.z = 500.0 + (cat.tile_pos.x + cat.tile_pos.y) * 0.01;
        let target = Vec2::new(cat.target.x as f32, cat.target.y as f32);
        let moving = cat.tile_pos.distance(target) > 0.08;
        let working = !moving && cat.job != Job::Rest && cat.work_timer > 0.0;
        let index = if working {
            ((time.elapsed_secs() * CAT_WORK_SPIN_FPS) as usize + cat.id) % 32
        } else if moving {
            visual.facing_group = cat_sheet_direction_group(target - cat.tile_pos);
            cat_sheet_walk_frame(visual.facing_group, time.elapsed_secs(), cat.id)
        } else {
            visual.facing_group * 4
        };
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = index;
        }
        let bob = if moving {
            (time.elapsed_secs() * 10.0 + cat.id as f32).sin() * 0.035
        } else {
            0.0
        };
        transform.scale = Vec3::new(1.0, 1.0 + bob, 1.0);
        ring_positions.push((
            cat.id,
            Vec3::new(
                p.x,
                p.y + 4.0,
                498.0 + (cat.tile_pos.x + cat.tile_pos.y) * 0.01,
            ),
        ));
    }
    for (ring, mut transform) in &mut rings {
        if let Some((_, pos)) = ring_positions
            .iter()
            .find(|(cat_id, _)| *cat_id == ring.cat_id)
        {
            transform.translation = *pos;
        }
    }
}

fn update_raiders(
    time: Res<Time>,
    mut commands: Commands,
    mut colony: ResMut<ColonyState>,
    mut raiders: Query<(Entity, &mut Raider, &mut Transform)>,
) {
    let dt = time.delta_secs() * colony.time_scale;
    for (entity, mut raider, mut transform) in &mut raiders {
        let target = Vec2::new(CENTER_X as f32, (CENTER_Y - PALISADE_RADIUS) as f32);
        let delta = target - raider.tile_pos;
        if delta.length() < 1.2 {
            colony.food = (colony.food - dt * 5.0).max(0.0);
            colony.water = (colony.water - dt * 3.0).max(0.0);
            raider.hp -= dt * 5.0;
        } else {
            raider.tile_pos += delta.normalize() * dt * 1.15;
        }
        let p = iso_point(raider.tile_pos.x, raider.tile_pos.y, 1);
        transform.translation.x = p.x;
        transform.translation.y = p.y + 14.0;
        if raider.hp <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn update_tile_visuals(
    map: Res<MapData>,
    colony: Res<ColonyState>,
    art: Res<ArtHandles>,
    mut tiles: Query<(&TileSprite, &mut Sprite)>,
) {
    for (tile_sprite, mut sprite) in &mut tiles {
        if let Some(tile) = map.get(tile_sprite.x, tile_sprite.y) {
            sprite.image = ground_texture(tile, &map, &art);
            sprite.color = tile_tint(tile, colony.hover);
        }
    }
}

fn update_ui(
    colony: Res<ColonyState>,
    cats: Query<(Entity, &Cat)>,
    raiders: Query<Entity, With<Raider>>,
    mut panels: Query<(&HudPanel, &mut Text)>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    map: Res<MapData>,
) {
    let mut jobs = [0; 8];
    let mut roles = [0; 8];
    for (_, cat) in &cats {
        jobs[job_index(cat.job)] += 1;
        roles[role_index(cat.role)] += 1;
    }
    let selected = colony
        .selected_cat
        .and_then(|entity| cats.get(entity).ok())
        .map(|(_, cat)| {
            format!(
                "{} the {}: {} -> {} at {},{} | energy {:.0} hunger {:.0} thirst {:.0}",
                cat.name,
                coat_name(cat.coat),
                role_name(cat.role),
                job_name(cat.job),
                cat.target.x,
                cat.target.y,
                cat.energy,
                cat.hunger,
                cat.thirst
            )
        })
        .unwrap_or_else(|| "none".to_string());
    let hover = colony
        .hover
        .and_then(|pos| map.get(pos.x, pos.y))
        .map(|tile| {
            format!(
                "Tile {},{}\n{}  h{}  wear {:.0}\nmarker: {}",
                tile.x,
                tile.y,
                tile_name(tile.kind),
                tile.height,
                tile.wear,
                tile.marker.map(marker_name).unwrap_or("none")
            )
        })
        .unwrap_or_else(|| "Tile none".to_string());
    let visible_tiles = estimate_visible_tiles(&windows, &camera, &map);

    for (panel, mut text) in &mut panels {
        text.0 = match panel.kind {
            HudPanelKind::Resources => format!(
                "Cat Idler - Bevy rework\nFPS: runtime | cats {} | visible tiles {}\nspeed {:.1}x{}\n\nFood      {:>3.0}/160\nWater     {:>3.0}/160\nMaterials {:>3.0}/140\nHerbs     {:>3.0}/80\nBlessings {:.1}   Research {:>3.0}/100\nThreat {:>3.0}%   Raiders {}",
                cats.iter().len(),
                visible_tiles,
                colony.time_scale,
                if colony.paused { " | paused" } else { "" },
                colony.food,
                colony.water,
                colony.materials,
                colony.herbs,
                colony.blessings,
                colony.research,
                colony.threat,
                raiders.iter().len()
            ),
            HudPanelKind::Jobs => format!(
                "Jobs\nHunt          {}\nFetch water   {}\nQuarry        {}\nBuild         {}\nScout         {}\nRitual        {}\nGuard         {}\nRest          {}\n\nRoles\nHunter {}  Water {}\nQuarry {}  Builder {}\nScout {}   Ritual {}\nWarrior {}",
                jobs[job_index(Job::Hunt)],
                jobs[job_index(Job::FetchWater)],
                jobs[job_index(Job::Quarry)],
                jobs[job_index(Job::Build)],
                jobs[job_index(Job::Scout)],
                jobs[job_index(Job::Ritual)],
                jobs[job_index(Job::Guard)],
                jobs[job_index(Job::Rest)],
                roles[role_index(Role::Hunter)],
                roles[role_index(Role::Water)],
                roles[role_index(Role::Quarry)],
                roles[role_index(Role::Builder)],
                roles[role_index(Role::Scout)],
                roles[role_index(Role::Ritualist)],
                roles[role_index(Role::Warrior)]
            ),
            HudPanelKind::Selection => {
                format!("Selection\n{}\n\n{}", selected, hover)
            }
            HudPanelKind::Toolbar => format!(
                "Mode: {}   [1] Inspect  [2] Priority  [3] Avoid  [4] Road  [5] Build  [Tab] Next\nWASD/arrows pan  |  middle-drag pan  |  wheel zoom  |  R reset  |  Space pause  |  +/- speed  |  [/ ] cats\nLeft click applies mode. Right click selects cats.",
                tool_mode_name(colony.tool_mode)
            ),
        };
    }
}

fn estimate_visible_tiles(
    windows: &Query<&Window>,
    camera: &Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    map: &MapData,
) -> usize {
    let Ok(window) = windows.single() else {
        return map.tiles.len();
    };
    let Ok((camera, camera_transform)) = camera.single() else {
        return map.tiles.len();
    };
    let Ok(min) = camera.viewport_to_world_2d(camera_transform, Vec2::new(0.0, 0.0)) else {
        return map.tiles.len();
    };
    let Ok(max) =
        camera.viewport_to_world_2d(camera_transform, Vec2::new(window.width(), window.height()))
    else {
        return map.tiles.len();
    };
    let lo = Vec2::new(min.x.min(max.x) - TILE_W, min.y.min(max.y) - TILE_W);
    let hi = Vec2::new(min.x.max(max.x) + TILE_W, min.y.max(max.y) + TILE_W);
    map.tiles
        .iter()
        .filter(|tile| {
            let p = iso_point(tile.x as f32, tile.y as f32, tile.height);
            p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y
        })
        .count()
}

fn step_cat(cat: &mut Cat, map: &mut MapData, dt: f32) -> bool {
    let final_target = Vec2::new(cat.target.x as f32, cat.target.y as f32);
    let active_target = routed_target(cat.tile_pos, cat.target);
    let target = Vec2::new(active_target.x as f32, active_target.y as f32);
    let delta = target - cat.tile_pos;
    if delta.length() < 0.05 {
        cat.tile_pos = target;
        return cat.tile_pos.distance(final_target) < 0.08;
    }
    let next = cat.tile_pos + delta.normalize() * dt * 2.2;
    let next_tile = IVec2::new(next.x.round() as i32, next.y.round() as i32);
    if !map.is_walkable(next_tile.x, next_tile.y) {
        let turn = if hash2(cat.id as f32, cat.work_timer, 21.0) > 0.5 {
            1
        } else {
            -1
        };
        cat.target.x = (cat.target.x + turn).clamp(0, MAP_W - 1);
        cat.target.y = (cat.target.y - turn).clamp(0, MAP_H - 1);
        return false;
    }
    cat.tile_pos = next;
    if let Some(tile) = map.get_mut(next_tile.x, next_tile.y) {
        if !matches!(tile.kind, TileKind::Water | TileKind::Fence) {
            tile.wear = (tile.wear + dt * 3.5).min(100.0);
            if tile.wear >= ROAD_WEAR_THRESHOLD
                && village_distance(tile.x, tile.y) > PALISADE_RADIUS
                && matches!(tile.kind, TileKind::Grass)
            {
                tile.kind = TileKind::Path;
            }
        }
    }
    false
}

fn routed_target(current: Vec2, final_target: IVec2) -> IVec2 {
    let current_tile = IVec2::new(current.x.round() as i32, current.y.round() as i32);
    let gate = IVec2::new(CENTER_X, CENTER_Y - PALISADE_RADIUS);
    let current_dist = village_distance(current_tile.x, current_tile.y);
    let target_dist = village_distance(final_target.x, final_target.y);
    let crosses_palisade = (current_dist < PALISADE_RADIUS && target_dist > PALISADE_RADIUS)
        || (current_dist > PALISADE_RADIUS && target_dist < PALISADE_RADIUS);
    let gate_dx = current_tile.x - gate.x;
    let gate_dy = current_tile.y - gate.y;
    if crosses_palisade && gate_dx * gate_dx + gate_dy * gate_dy > 2 {
        gate
    } else {
        final_target
    }
}

fn village_distance(x: i32, y: i32) -> i32 {
    (x - CENTER_X).abs().max((y - CENTER_Y).abs())
}

fn complete_work(cat: &mut Cat, colony: &mut ColonyState, map: &MapData, dt: f32) {
    cat.work_timer += dt;
    if cat.job == Job::Rest {
        cat.energy = (cat.energy + dt * 12.0).min(100.0);
        cat.hunger = (cat.hunger - dt * 4.0).max(0.0);
        cat.thirst = (cat.thirst - dt * 5.0).max(0.0);
        if cat.energy > 72.0 && cat.hunger < 62.0 && cat.thirst < 62.0 {
            assign_job(cat, colony, map);
        }
        return;
    }
    if cat.work_timer < 1.8 {
        return;
    }
    cat.work_timer = 0.0;
    match cat.job {
        Job::Hunt => colony.food = (colony.food + 4.0).min(160.0),
        Job::FetchWater => colony.water = (colony.water + 5.0).min(160.0),
        Job::Quarry => colony.materials = (colony.materials + 3.0).min(140.0),
        Job::Build => {
            colony.materials = (colony.materials - 1.0).max(0.0);
            colony.blessings += 0.08;
        }
        Job::Scout => colony.research = (colony.research + 0.3).min(100.0),
        Job::Ritual => colony.blessings += 0.15,
        Job::Guard => colony.threat = (colony.threat - 2.0).max(0.0),
        Job::Rest => {}
    }
    assign_job(cat, colony, map);
}

fn assign_job(cat: &mut Cat, colony: &ColonyState, map: &MapData) {
    cat.job = choose_job(cat, colony);
    cat.work_timer = 0.0;
    cat.target = target_for_job(cat.job, cat.id as f32 * 17.0 + colony.elapsed, map);
}

fn choose_job(cat: &Cat, colony: &ColonyState) -> Job {
    if cat.energy < 18.0 || cat.hunger > 86.0 || cat.thirst > 86.0 {
        return Job::Rest;
    }
    if colony.threat > 78.0 && matches!(cat.role, Role::Warrior | Role::Hunter) {
        return Job::Guard;
    }
    if colony.water < 45.0 {
        return Job::FetchWater;
    }
    if colony.food < 45.0 {
        return Job::Hunt;
    }
    if colony.materials < 35.0 {
        return Job::Quarry;
    }
    match cat.role {
        Role::Builder if colony.materials > 10.0 => Job::Build,
        Role::Scout => Job::Scout,
        Role::Ritualist => Job::Ritual,
        Role::Warrior => Job::Guard,
        Role::Water => Job::FetchWater,
        Role::Quarry => Job::Quarry,
        Role::Hunter => Job::Hunt,
        _ => Job::Rest,
    }
}

fn target_for_job(job: Job, salt: f32, map: &MapData) -> IVec2 {
    let wanted = match job {
        Job::Hunt => Some(TileKind::Forest),
        Job::FetchWater => Some(TileKind::Water),
        Job::Quarry => Some(TileKind::Stone),
        Job::Scout => None,
        Job::Guard => return IVec2::new(CENTER_X, CENTER_Y - PALISADE_RADIUS),
        Job::Build | Job::Ritual | Job::Rest => return IVec2::new(CENTER_X, CENTER_Y),
    };
    for i in 0..80 {
        let x = (hash2(salt, i as f32, 11.0) * MAP_W as f32).floor() as i32;
        let y = (hash2(salt, i as f32, 12.0) * MAP_H as f32).floor() as i32;
        if let Some(tile) = map.get(x, y) {
            if job == Job::FetchWater && tile.kind == TileKind::Water {
                if let Some(adjacent) = adjacent_walkable_tile(map, x, y) {
                    return adjacent;
                }
            }
            if wanted
                .map(|kind| {
                    tile.kind == kind || (job == Job::Quarry && tile.kind == TileKind::Highland)
                })
                .unwrap_or(true)
                && map.is_walkable(x, y)
            {
                return IVec2::new(x, y);
            }
        }
    }
    IVec2::new(CENTER_X, CENTER_Y)
}

fn adjacent_walkable_tile(map: &MapData, x: i32, y: i32) -> Option<IVec2> {
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let nx = x + dx;
        let ny = y + dy;
        if map.is_walkable(nx, ny) {
            return Some(IVec2::new(nx, ny));
        }
    }
    None
}

fn random_walkable_tile(map: &MapData, salt: f32, village: bool) -> (i32, i32) {
    for i in 0..120 {
        let (x, y) = if village {
            (
                CENTER_X + (hash2(salt, i as f32, 1.0) * 15.0).floor() as i32 - 7,
                CENTER_Y + (hash2(salt, i as f32, 2.0) * 15.0).floor() as i32 - 7,
            )
        } else {
            (
                (hash2(salt, i as f32, 3.0) * MAP_W as f32).floor() as i32,
                (hash2(salt, i as f32, 4.0) * MAP_H as f32).floor() as i32,
            )
        };
        if map.is_walkable(x, y) {
            return (x, y);
        }
    }
    (CENTER_X, CENTER_Y)
}

fn ground_texture(tile: &Tile, map: &MapData, art: &ArtHandles) -> Handle<Image> {
    match tile.kind {
        TileKind::Grass => art.grass.clone(),
        TileKind::Clearing => art.clearing.clone(),
        TileKind::Water => art.water.clone(),
        TileKind::Stone => art.stone.clone(),
        TileKind::Highland => art.highland.clone(),
        TileKind::Forest => art.grass.clone(),
        TileKind::Path => path_texture(tile, map, art),
        TileKind::Fence => art.grass.clone(),
        TileKind::Gate => art.road_built.clone(),
    }
}

fn overlay_texture(tile: &Tile, art: &ArtHandles) -> Option<Handle<Image>> {
    match tile.kind {
        TileKind::Forest => {
            let r = hash2(tile.x as f32, tile.y as f32, 31.0);
            Some(if r > 0.82 {
                art.tree_huge.clone()
            } else if r > 0.45 {
                art.tree_large.clone()
            } else {
                art.tree_small.clone()
            })
        }
        TileKind::Fence => Some(if (tile.x - CENTER_X).abs() >= (tile.y - CENTER_Y).abs() {
            art.fence_y.clone()
        } else {
            art.fence_x.clone()
        }),
        TileKind::Gate => Some(art.gate.clone()),
        _ => None,
    }
}

fn path_texture(tile: &Tile, map: &MapData, art: &ArtHandles) -> Handle<Image> {
    if tile.wear > 95.0 {
        return art.road_built.clone();
    }
    if tile.wear > 64.0 {
        return art.road.clone();
    }

    let n = is_path_like(map, tile.x, tile.y - 1);
    let s = is_path_like(map, tile.x, tile.y + 1);
    let e = is_path_like(map, tile.x + 1, tile.y);
    let w = is_path_like(map, tile.x - 1, tile.y);
    let count = [n, s, e, w].iter().filter(|connected| **connected).count();
    match (count, n, s, e, w) {
        (0, ..) | (4, ..) => art.path_crossing.clone(),
        (1, true, ..) => art.path_end_n.clone(),
        (1, _, true, ..) => art.path_end_s.clone(),
        (1, _, _, true, _) => art.path_end_e.clone(),
        (1, _, _, _, true) => art.path_end_w.clone(),
        (_, true, true, false, false) => art.path_straight_n.clone(),
        (_, false, false, true, true) => art.path_straight_e.clone(),
        (_, true, false, true, false) => art.path_corner_e.clone(),
        (_, false, true, true, false) => art.path_corner_s.clone(),
        (_, false, true, false, true) => art.path_corner_w.clone(),
        (_, true, false, false, true) => art.path_corner_n.clone(),
        _ => art.path_crossing.clone(),
    }
}

fn is_path_like(map: &MapData, x: i32, y: i32) -> bool {
    map.get(x, y)
        .map(|tile| matches!(tile.kind, TileKind::Path | TileKind::Gate))
        .unwrap_or(false)
}

fn tile_tint(tile: &Tile, hover: Option<IVec2>) -> Color {
    if let Some(marker) = tile.marker {
        return marker_tint(marker);
    }
    if hover == Some(IVec2::new(tile.x, tile.y)) {
        return Color::srgb(1.16, 1.12, 0.86);
    }
    let fog = fog_brightness(tile);
    if tile.height >= 3 {
        Color::srgb(0.9 * fog, 0.92 * fog, 0.96 * fog)
    } else {
        Color::srgb(fog, fog, fog)
    }
}

fn fog_brightness(tile: &Tile) -> f32 {
    let dist = village_distance(tile.x, tile.y);
    if dist <= PALISADE_RADIUS + 4 || tile.wear > 40.0 {
        1.0
    } else if dist <= PALISADE_RADIUS + 16 {
        0.72
    } else if dist <= PALISADE_RADIUS + 28 {
        0.48
    } else {
        0.28
    }
}

fn marker_tint(marker: TileMarker) -> Color {
    match marker {
        TileMarker::Priority => Color::srgb(1.0, 0.82, 0.34),
        TileMarker::Avoid => Color::srgb(1.0, 0.45, 0.42),
        TileMarker::RoadPlan => Color::srgb(0.78, 0.88, 1.0),
        TileMarker::BuildPlan => Color::srgb(0.78, 1.0, 0.72),
    }
}

fn marker_name(marker: TileMarker) -> &'static str {
    match marker {
        TileMarker::Priority => "priority",
        TileMarker::Avoid => "avoid",
        TileMarker::RoadPlan => "road plan",
        TileMarker::BuildPlan => "build plan",
    }
}

fn toggle_marker(tile: &mut Tile, marker: TileMarker) {
    tile.marker = if tile.marker == Some(marker) {
        None
    } else {
        Some(marker)
    };
}

fn next_tool_mode(mode: ToolMode) -> ToolMode {
    match mode {
        ToolMode::Inspect => ToolMode::Priority,
        ToolMode::Priority => ToolMode::Avoid,
        ToolMode::Avoid => ToolMode::Road,
        ToolMode::Road => ToolMode::Build,
        ToolMode::Build => ToolMode::Inspect,
    }
}

fn tool_mode_name(mode: ToolMode) -> &'static str {
    match mode {
        ToolMode::Inspect => "Inspect",
        ToolMode::Priority => "Priority",
        ToolMode::Avoid => "Avoid",
        ToolMode::Road => "Road",
        ToolMode::Build => "Build",
    }
}

fn building_texture(kind: BuildingKind, art: &ArtHandles) -> Handle<Image> {
    match kind {
        BuildingKind::Shrine => art.shrine.clone(),
        BuildingKind::Den => art.den.clone(),
        BuildingKind::FoodStorage => art.food_storage.clone(),
        BuildingKind::WaterBowl => art.water_bowl.clone(),
        BuildingKind::Beds => art.beds.clone(),
        BuildingKind::Workshop => art.workshop.clone(),
        BuildingKind::Field => art.field.clone(),
        BuildingKind::Nursery => art.nursery.clone(),
        BuildingKind::HerbGarden => art.herb_garden.clone(),
        BuildingKind::Walls => art.walls.clone(),
    }
}

fn iso_point(x: f32, y: f32, _height: i32) -> Vec2 {
    Vec2::new((x - y) * TILE_W * 0.5, -(x + y) * TILE_H * 0.5)
}

fn iso_to_tile(world: Vec2) -> IVec2 {
    let a = world.x / (TILE_W * 0.5);
    let b = -world.y / (TILE_H * 0.5);
    IVec2::new(
        ((b + a) * 0.5).round() as i32,
        ((b - a) * 0.5).round() as i32,
    )
}

fn tile_depth(x: i32, y: i32) -> f32 {
    (x + y) as f32 * 0.01
}

fn hash2(x: f32, y: f32, salt: f32) -> f32 {
    let v = (x * 12.9898 + y * 78.233 + salt * 37.719 + SEED).sin() * 43758.547;
    v - v.floor()
}

fn coat_for_id(id: usize) -> CatCoat {
    match id % 6 {
        0 => CatCoat::Black,
        1 => CatCoat::Calico,
        2 => CatCoat::GrayTabby,
        3 => CatCoat::OrangeTabby,
        4 => CatCoat::Tuxedo,
        _ => CatCoat::White,
    }
}

fn cat_sheet_direction_group(delta: Vec2) -> usize {
    if delta.length_squared() < 0.001 {
        return 0;
    }
    // Match the original sheet order: S, SW, W, NW, N, NE, E, SE.
    let screen_x = delta.x - delta.y;
    let screen_y = (delta.x + delta.y) * 0.5;
    let angle = screen_y.atan2(screen_x).to_degrees();
    (((angle - 90.0).rem_euclid(360.0) / 45.0).round() as usize) % 8
}

fn cat_sheet_walk_frame(group: usize, elapsed: f32, cat_id: usize) -> usize {
    let frame = ((elapsed * CAT_WALK_FPS) as usize + cat_id) % 4;
    group * 4 + frame
}

fn role_tint(role: Role) -> Color {
    match role {
        Role::Hunter => Color::srgb(1.0, 0.82, 0.68),
        Role::Water => Color::srgb(0.72, 0.90, 1.0),
        Role::Quarry => Color::srgb(0.86, 0.86, 0.90),
        Role::Builder => Color::srgb(1.0, 0.90, 0.64),
        Role::Scout => Color::srgb(0.80, 1.0, 0.70),
        Role::Ritualist => Color::srgb(0.92, 0.78, 1.0),
        Role::Warrior => Color::srgb(1.0, 0.68, 0.68),
        Role::Rest => Color::WHITE,
    }
}

fn role_index(role: Role) -> usize {
    match role {
        Role::Hunter => 0,
        Role::Water => 1,
        Role::Quarry => 2,
        Role::Builder => 3,
        Role::Scout => 4,
        Role::Ritualist => 5,
        Role::Warrior => 6,
        Role::Rest => 7,
    }
}

fn job_index(job: Job) -> usize {
    match job {
        Job::Hunt => 0,
        Job::FetchWater => 1,
        Job::Quarry => 2,
        Job::Build => 3,
        Job::Scout => 4,
        Job::Ritual => 5,
        Job::Guard => 6,
        Job::Rest => 7,
    }
}

fn role_name(role: Role) -> &'static str {
    match role {
        Role::Hunter => "hunter",
        Role::Water => "water",
        Role::Quarry => "quarry",
        Role::Builder => "builder",
        Role::Scout => "scout",
        Role::Ritualist => "ritualist",
        Role::Warrior => "warrior",
        Role::Rest => "rest",
    }
}

fn job_name(job: Job) -> &'static str {
    match job {
        Job::Hunt => "hunt",
        Job::FetchWater => "fetch water",
        Job::Quarry => "quarry",
        Job::Build => "build",
        Job::Scout => "scout",
        Job::Ritual => "ritual",
        Job::Guard => "guard gate",
        Job::Rest => "rest",
    }
}

fn coat_name(coat: CatCoat) -> &'static str {
    match coat {
        CatCoat::Black => "black cat",
        CatCoat::Calico => "calico",
        CatCoat::GrayTabby => "gray tabby",
        CatCoat::OrangeTabby => "orange tabby",
        CatCoat::Tuxedo => "tuxedo",
        CatCoat::White => "white cat",
    }
}

fn tile_name(kind: TileKind) -> &'static str {
    match kind {
        TileKind::Grass => "grass",
        TileKind::Clearing => "clearing",
        TileKind::Forest => "forest",
        TileKind::Water => "water",
        TileKind::Stone => "stone",
        TileKind::Highland => "highland",
        TileKind::Path => "path",
        TileKind::Fence => "palisade",
        TileKind::Gate => "gate",
    }
}
