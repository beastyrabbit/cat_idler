//! Cat colony Bevy client — top-down renderer for **Idle Cat Forest**.
//!
//! Connects to `cat-server` over a WebSocket, deserializes the live
//! [`cat_protocol::WorldSnapshot`] each tick, and renders it as a **flat,
//! single-level, top-down grid** (per `docs/GAME_VISION.md` — no isometric, no
//! z-layers). A world tile `(x, y)` maps to screen `Vec2::new(x * TILE, -y *
//! TILE)`. The view centres on the starter village anchor
//! ([`cat_sim::village_layout::VILLAGE_ANCHOR`]) and draws:
//!
//! - static terrain regenerated from `snapshot.world_seed` via `cat_sim`,
//! - cats (coloured by specialization, with a carried-item glyph),
//! - labelled buildings/workshops,
//! - a stockpile indicator near the shrine,
//! - avoid/gather zones,
//! - a HUD dashboard + event log, and clickable manual-action buttons that
//!   round-trip [`cat_protocol::ClientAction`] over the socket.

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use cat_protocol::{
    BuildingType, CarryingKind, ClientAction, ColonySnapshot, JobKind, Specialization,
    WorldSnapshot, ZoneKind,
};
use cat_sim::terrain_gen::{
    BiomeRole, DecorationRole, TerrainTile, WORLD_TERRAIN_OPTIONS, generate_terrain_chunk,
};
use cat_sim::village_layout::VILLAGE_ANCHOR;
use cat_sim::world_gen::tile_to_chunk;
use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};

/// Side length (world units) of one flat tile.
const TILE: f32 = 28.0;
/// Half-width (in tiles) of the terrain window regenerated around the anchor.
const WINDOW_RADIUS: i32 = 30;

// Z bands — all strictly below the camera at Z=1000 so nothing is clipped.
const Z_TERRAIN: f32 = 0.0;
const Z_DECORATION: f32 = 1.0;
const Z_ZONE: f32 = 2.0;
const Z_BUILDING: f32 = 10.0;
const Z_BUILDING_LABEL: f32 = 11.0;
const Z_STOCK: f32 = 12.0;
const Z_RAIDER: f32 = 15.0;
const Z_CAT: f32 = 20.0;
const Z_CAT_ITEM: f32 = 21.0;

const CAMERA_Z: f32 = 1000.0;

/// Flat top-down projection: tile `(x, y)` → world space. Y is negated so the
/// grid reads top-down with north up.
fn grid_to_world(x: i32, y: i32) -> Vec2 {
    Vec2::new(x as f32 * TILE, -(y as f32) * TILE)
}

/// The most recent snapshot pushed by the server.
#[derive(Resource, Default)]
struct LatestSnapshot(Option<WorldSnapshot>);

/// Signed session issued by the server after a `Presence` handshake; required
/// to send authenticated actions.
#[derive(Resource, Default)]
struct Session {
    session_id: String,
    sig: String,
    presence_sent: bool,
    ready: bool,
}

/// Outbound action queue drained onto the socket by [`flush_outgoing`].
#[derive(Resource, Default)]
struct OutgoingActions(Vec<ClientAction>);

/// Tracks one-time terrain spawn (terrain is static per `world_seed`).
#[derive(Resource, Default)]
struct WorldRender {
    terrain_spawned: bool,
}

/// The live WebSocket connection (kept off the render threads — the receiver is
/// `!Sync`).
struct WsConn {
    sender: WsSender,
    receiver: WsReceiver,
}

/// Marker for a spawned cat body sprite (cleared + redrawn each update).
#[derive(Component)]
struct CatSprite;
/// Marker for a carried-item glyph.
#[derive(Component)]
struct CatItem;
/// Marker for a building marker sprite.
#[derive(Component)]
struct BuildingSprite;
/// Marker for a building world-space text label.
#[derive(Component)]
struct BuildingLabel;
/// Marker for a zone overlay tile.
#[derive(Component)]
struct ZoneSprite;
/// Marker for a raider sprite.
#[derive(Component)]
struct RaiderSprite;
/// Marker for the on-map stockpile indicator text.
#[derive(Component)]
struct StockText;
/// Marker for the HUD dashboard text.
#[derive(Component)]
struct HudText;
/// Marker for the event-log text.
#[derive(Component)]
struct EventLogText;

/// A manual-action button and the action it enqueues when clicked.
#[derive(Component, Clone, Copy)]
struct ActionButton(ButtonAction);

/// Query filter for the per-tick redraw of building marker + label entities.
type BuildingEntities = Or<(With<BuildingSprite>, With<BuildingLabel>)>;
/// Query filter for the per-tick redraw of cat body + carried-item entities.
type CatEntities = Or<(With<CatSprite>, With<CatItem>)>;
/// Change filter for toolbar button interactions.
type ButtonQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Interaction,
        &'static ActionButton,
        &'static mut BackgroundColor,
    ),
    (Changed<Interaction>, With<Button>),
>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ButtonAction {
    SupplyFood,
    SupplyWater,
    PlanHunt,
    FoundVillage,
}

impl ButtonAction {
    fn label(self) -> &'static str {
        match self {
            Self::SupplyFood => "Supply food",
            Self::SupplyWater => "Supply water",
            Self::PlanHunt => "Plan hunt",
            Self::FoundVillage => "Found village",
        }
    }
}

/// Build and run the client. `CAT_SERVER_URL` overrides the server address.
pub fn run() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: ".".to_string(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Idle Cat Forest".to_string(),
                        resolution: bevy::window::WindowResolution::new(1280, 800),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(LatestSnapshot::default())
        .insert_resource(Session::default())
        .insert_resource(OutgoingActions::default())
        .insert_resource(WorldRender::default())
        .insert_resource(ClearColor(Color::srgb(0.06, 0.09, 0.08)))
        .add_systems(Startup, (setup, connect_ws))
        .add_systems(
            Update,
            (
                poll_ws,
                ensure_presence,
                spawn_terrain,
                render_buildings,
                render_zones,
                render_cats,
                render_raiders,
                camera_controls,
                update_hud,
                update_event_log,
                update_stock_indicator,
                handle_buttons,
                flush_outgoing,
            ),
        )
        .run();
}

fn setup(mut commands: Commands) {
    // Camera at Z=1000: a default Camera2d sits at Z=0 and clips sprites at
    // Z>0. Centre on the village anchor.
    let center = grid_to_world(VILLAGE_ANCHOR.x, VILLAGE_ANCHOR.y);
    commands.spawn((Camera2d, Transform::from_xyz(center.x, center.y, CAMERA_Z)));

    // HUD dashboard (top-left).
    commands.spawn((
        hud_panel_node(8.0, 8.0, 320.0),
        BackgroundColor(Color::srgba(0.03, 0.04, 0.035, 0.82)),
        children![(
            Text::new("connecting…"),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.95, 0.84)),
            HudText,
        )],
    ));

    // Event log (bottom-left).
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(8.0),
            bottom: Val::Px(70.0),
            width: Val::Px(420.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.03, 0.04, 0.035, 0.72)),
        children![(
            Text::new("events…"),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::srgb(0.86, 0.90, 0.80)),
            EventLogText,
        )],
    ));

    // Action toolbar (bottom, centred).
    spawn_toolbar(&mut commands);
}

fn hud_panel_node(left: f32, top: f32, width: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(left),
        top: Val::Px(top),
        width: Val::Px(width),
        padding: UiRect::all(Val::Px(12.0)),
        ..default()
    }
}

fn spawn_toolbar(commands: &mut Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|row| {
            for action in [
                ButtonAction::SupplyFood,
                ButtonAction::SupplyWater,
                ButtonAction::PlanHunt,
                ButtonAction::FoundVillage,
            ] {
                row.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.18, 0.26, 0.20)),
                    BorderColor::all(Color::srgba(0.80, 0.67, 0.42, 0.5)),
                    ActionButton(action),
                    children![(
                        Text::new(action.label()),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.95, 0.84)),
                    )],
                ));
            }
        });
}

/// Open the WebSocket as a non-send resource (the receiver is `!Sync`).
fn connect_ws(world: &mut World) {
    let url =
        std::env::var("CAT_SERVER_URL").unwrap_or_else(|_| "ws://127.0.0.1:8787/ws".to_string());
    match ewebsock::connect(url.clone(), ewebsock::Options::default()) {
        Ok((sender, receiver)) => {
            info!("cat-client connecting to {url}");
            world.insert_non_send(WsConn { sender, receiver });
        }
        Err(err) => error!("cat-client failed to connect to {url}: {err}"),
    }
}

/// Drain socket messages: `WorldSnapshot`s update the render, action results
/// carry the signed session after a `Presence` handshake.
fn poll_ws(
    conn: Option<NonSend<WsConn>>,
    mut latest: ResMut<LatestSnapshot>,
    mut session: ResMut<Session>,
) {
    let Some(conn) = conn else {
        return;
    };
    while let Some(event) = conn.receiver.try_recv() {
        let WsEvent::Message(WsMessage::Text(text)) = event else {
            continue;
        };
        // Snapshots carry a `colonies` array; action results carry `ok`.
        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(value) if value.get("colonies").is_some() => {
                match serde_json::from_str::<WorldSnapshot>(&text) {
                    Ok(snapshot) => latest.0 = Some(snapshot),
                    Err(err) => warn!("bad snapshot: {err}"),
                }
            }
            Ok(value) => {
                if let (Some(sid), Some(sig)) = (
                    value.get("sessionId").and_then(|v| v.as_str()),
                    value.get("sig").and_then(|v| v.as_str()),
                ) {
                    session.session_id = sid.to_string();
                    session.sig = sig.to_string();
                    session.ready = true;
                }
            }
            Err(err) => warn!("bad ws message: {err}"),
        }
    }
}

/// Send the `Presence` handshake once so the server issues a signed session.
fn ensure_presence(conn: Option<NonSendMut<WsConn>>, mut session: ResMut<Session>) {
    let Some(mut conn) = conn else {
        return;
    };
    if session.presence_sent {
        return;
    }
    let action = ClientAction::Presence {
        session_id: "desktop".to_string(),
        nickname: "Desktop Cat".to_string(),
        sig: None,
    };
    if let Ok(json) = serde_json::to_string(&action) {
        conn.sender.send(WsMessage::Text(json));
        session.presence_sent = true;
    }
}

/// Spawn the flat terrain grid once, from the first snapshot's `world_seed`.
fn spawn_terrain(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    mut render: ResMut<WorldRender>,
) {
    if render.terrain_spawned {
        return;
    }
    let Some(world) = latest.0.as_ref() else {
        return;
    };
    let seed = world.world_seed;

    for tile in window_terrain(seed) {
        let p = grid_to_world(tile.x, tile.y);
        commands.spawn((
            Sprite::from_color(terrain_color(&tile), Vec2::splat(TILE)),
            Transform::from_xyz(p.x, p.y, Z_TERRAIN),
        ));
        if let Some(color) = decoration_color(tile.decoration) {
            commands.spawn((
                Sprite::from_color(color, Vec2::splat(TILE * 0.42)),
                Transform::from_xyz(p.x, p.y, Z_DECORATION),
            ));
        }
    }
    render.terrain_spawned = true;
    info!("terrain spawned (seed {seed})");
}

/// Regenerate the terrain tiles inside the window around the village anchor.
fn window_terrain(seed: i64) -> Vec<TerrainTile> {
    let min = tile_to_chunk(
        VILLAGE_ANCHOR.x - WINDOW_RADIUS,
        VILLAGE_ANCHOR.y - WINDOW_RADIUS,
    );
    let max = tile_to_chunk(
        VILLAGE_ANCHOR.x + WINDOW_RADIUS,
        VILLAGE_ANCHOR.y + WINDOW_RADIUS,
    );
    let (x0, y0, x1, y1) = (
        VILLAGE_ANCHOR.x - WINDOW_RADIUS,
        VILLAGE_ANCHOR.y - WINDOW_RADIUS,
        VILLAGE_ANCHOR.x + WINDOW_RADIUS,
        VILLAGE_ANCHOR.y + WINDOW_RADIUS,
    );
    let mut tiles = Vec::new();
    for cy in min.chunk_y..=max.chunk_y {
        for cx in min.chunk_x..=max.chunk_x {
            for tile in generate_terrain_chunk(cx, cy, seed, WORLD_TERRAIN_OPTIONS) {
                if (x0..=x1).contains(&tile.x) && (y0..=y1).contains(&tile.y) {
                    tiles.push(tile);
                }
            }
        }
    }
    tiles
}

fn render_buildings(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    sprites: Query<Entity, BuildingEntities>,
) {
    if !latest.is_changed() {
        return;
    }
    for entity in &sprites {
        commands.entity(entity).despawn();
    }
    let Some(colony) = latest.0.as_ref().and_then(|w| w.colonies.first()) else {
        return;
    };
    for building in &colony.buildings {
        let p = grid_to_world(building.world_position.x, building.world_position.y);
        commands.spawn((
            Sprite::from_color(
                building_color(building.building_type),
                Vec2::splat(TILE * 0.8),
            ),
            Transform::from_xyz(p.x, p.y, Z_BUILDING),
            BuildingSprite,
        ));
        commands.spawn((
            Text2d::new(building_label(building.building_type)),
            TextFont {
                font_size: FontSize::Px(9.0),
                ..default()
            },
            TextColor(Color::srgba(1.0, 0.97, 0.86, 0.92)),
            Transform::from_xyz(p.x, p.y + TILE * 0.72, Z_BUILDING_LABEL),
            BuildingLabel,
        ));
    }
}

fn render_zones(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    sprites: Query<Entity, With<ZoneSprite>>,
) {
    if !latest.is_changed() {
        return;
    }
    for entity in &sprites {
        commands.entity(entity).despawn();
    }
    let Some(colony) = latest.0.as_ref().and_then(|w| w.colonies.first()) else {
        return;
    };
    for zone in &colony.zones {
        let (x0, x1) = (zone.x1.min(zone.x2), zone.x1.max(zone.x2));
        let (y0, y1) = (zone.y1.min(zone.y2), zone.y1.max(zone.y2));
        let w = (x1 - x0 + 1) as f32 * TILE;
        let h = (y1 - y0 + 1) as f32 * TILE;
        let cx = (x0 as f32 + x1 as f32) / 2.0 * TILE;
        let cy = -(y0 as f32 + y1 as f32) / 2.0 * TILE;
        commands.spawn((
            Sprite::from_color(zone_color(zone.kind), Vec2::new(w, h)),
            Transform::from_xyz(cx, cy, Z_ZONE),
            ZoneSprite,
        ));
    }
}

fn render_cats(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    existing: Query<Entity, CatEntities>,
) {
    if !latest.is_changed() {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let Some(colony) = latest.0.as_ref().and_then(|w| w.colonies.first()) else {
        return;
    };
    for cat in &colony.cats {
        if cat.death_time.is_some() {
            continue;
        }
        let p = grid_to_world(cat.position.x, cat.position.y);
        commands.spawn((
            Sprite::from_color(cat_color(cat.specialization), Vec2::splat(TILE * 0.5)),
            Transform::from_xyz(p.x, p.y, Z_CAT),
            CatSprite,
        ));
        if let Some(carrying) = &cat.carrying {
            commands.spawn((
                Sprite::from_color(carrying_color(carrying.kind), Vec2::splat(TILE * 0.22)),
                Transform::from_xyz(p.x + TILE * 0.22, p.y + TILE * 0.22, Z_CAT_ITEM),
                CatItem,
            ));
        }
    }
}

fn render_raiders(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    existing: Query<Entity, With<RaiderSprite>>,
) {
    if !latest.is_changed() {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let Some(colony) = latest.0.as_ref().and_then(|w| w.colonies.first()) else {
        return;
    };
    for raider in &colony.raiders {
        let p = grid_to_world(raider.position.x, raider.position.y);
        commands.spawn((
            Sprite::from_color(Color::srgb(0.85, 0.15, 0.15), Vec2::splat(TILE * 0.55)),
            Transform::from_xyz(p.x, p.y, Z_RAIDER),
            RaiderSprite,
        ));
    }
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
        let center = grid_to_world(VILLAGE_ANCHOR.x, VILLAGE_ANCHOR.y);
        transform.translation.x = center.x;
        transform.translation.y = center.y;
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
        projection.scale = (projection.scale * if ev.y > 0.0 { 0.9 } else { 1.1 }).clamp(0.3, 3.0);
    }
}

fn update_hud(latest: Res<LatestSnapshot>, mut hud: Query<&mut Text, With<HudText>>) {
    if !latest.is_changed() {
        return;
    }
    let Ok(mut text) = hud.single_mut() else {
        return;
    };
    let Some(world) = latest.0.as_ref() else {
        text.0 = "connecting…".to_string();
        return;
    };
    let Some(colony) = world.colonies.first() else {
        text.0 = format!(
            "Idle Cat Forest\nonline: {}\nNo colony yet — press Found village.",
            world.online_count
        );
        return;
    };
    text.0 = dashboard_text(colony, world.online_count);
}

fn dashboard_text(colony: &ColonySnapshot, online: u32) -> String {
    let r = &colony.resources;
    let cap = &colony.storage.capacities;
    let leader = colony
        .leader
        .as_ref()
        .map_or_else(|| "—".to_string(), |l| l.name.clone());
    let active_jobs = colony
        .jobs
        .iter()
        .filter(|j| matches!(j.status, cat_protocol::JobStatus::Active))
        .count();
    format!(
        "Idle Cat Forest   online {online}\n\
         Colony: {name}  [{status:?}]\n\
         Leader: {leader}\n\
         Pop {pop}/{cap_house}  Village Lv {lvl}\n\
         Threat: {threat:?} ({pressure:.0})  warriors {warriors}\n\
         \n\
         Food      {food:>5.0} / {food_cap:.0}\n\
         Water     {water:>5.0} / {water_cap:.0}\n\
         Materials {mat:>5.0} / {mat_cap:.0}\n\
         Refined   {ref_:>5.0} / {ref_cap:.0}\n\
         Herbs     {herbs:>5.0} / {herbs_cap:.0}\n\
         Weapons   {weap:>5.0}   Armor {armor:.0}\n\
         Blessings {bless:>5.1}\n\
         \n\
         Active jobs: {active_jobs}   Total jobs: {jobs}",
        name = colony.name,
        status = colony.status,
        pop = colony.housing.population,
        cap_house = colony.housing.capacity,
        lvl = colony.housing.village_level,
        threat = colony.threat.band,
        pressure = colony.threat.pressure,
        warriors = colony.threat.warriors,
        food = r.food,
        food_cap = cap.food,
        water = r.water,
        water_cap = cap.water,
        mat = r.materials,
        mat_cap = cap.materials,
        ref_ = r.refined,
        ref_cap = cap.refined,
        herbs = r.herbs,
        herbs_cap = cap.herbs,
        weap = r.weapons,
        armor = r.armor,
        bless = r.blessings,
        jobs = colony.jobs.len(),
    )
}

fn update_event_log(latest: Res<LatestSnapshot>, mut log: Query<&mut Text, With<EventLogText>>) {
    if !latest.is_changed() {
        return;
    }
    let Ok(mut text) = log.single_mut() else {
        return;
    };
    let Some(colony) = latest.0.as_ref().and_then(|w| w.colonies.first()) else {
        return;
    };
    let mut events = colony.events.clone();
    events.sort_by_key(|e| e.timestamp);
    let lines: Vec<String> = events
        .iter()
        .rev()
        .take(6)
        .map(|e| format!("• {}", e.message))
        .collect();
    text.0 = if lines.is_empty() {
        "no recent events".to_string()
    } else {
        lines.join("\n")
    };
}

/// On-map stockpile indicator: a compact resource readout anchored at the
/// shrine (falls back to the colony anchor).
fn update_stock_indicator(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    existing: Query<Entity, With<StockText>>,
) {
    if !latest.is_changed() {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let Some(colony) = latest.0.as_ref().and_then(|w| w.colonies.first()) else {
        return;
    };
    let shrine = colony
        .buildings
        .iter()
        .find(|b| b.building_type == BuildingType::Shrine)
        .map_or(colony.anchor, |b| b.world_position);
    let p = grid_to_world(shrine.x, shrine.y);
    let r = &colony.resources;
    commands.spawn((
        Text2d::new(format!(
            "F {:.0}  W {:.0}  M {:.0}  R {:.0}",
            r.food, r.water, r.materials, r.refined
        )),
        TextFont {
            font_size: FontSize::Px(10.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.9, 0.6)),
        Anchor::CENTER,
        Transform::from_xyz(p.x, p.y - TILE * 0.9, Z_STOCK),
        StockText,
    ));
}

/// React to toolbar clicks: tint the button and enqueue its action.
fn handle_buttons(
    session: Res<Session>,
    mut outgoing: ResMut<OutgoingActions>,
    mut buttons: ButtonQuery,
) {
    for (interaction, button, mut color) in &mut buttons {
        match interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(Color::srgb(0.34, 0.48, 0.36));
                if let Some(action) = build_action(button.0, &session) {
                    outgoing.0.push(action);
                }
            }
            Interaction::Hovered => *color = BackgroundColor(Color::srgb(0.24, 0.34, 0.26)),
            Interaction::None => *color = BackgroundColor(Color::srgb(0.18, 0.26, 0.20)),
        }
    }
}

fn build_action(action: ButtonAction, session: &Session) -> Option<ClientAction> {
    let kind = match action {
        ButtonAction::SupplyFood => JobKind::SupplyFood,
        ButtonAction::SupplyWater => JobKind::SupplyWater,
        ButtonAction::PlanHunt => JobKind::LeaderPlanHunt,
        ButtonAction::FoundVillage => {
            return Some(ClientAction::FoundVillage {
                name: "Forest Hollow".to_string(),
                session_id: if session.session_id.is_empty() {
                    "desktop".to_string()
                } else {
                    session.session_id.clone()
                },
            });
        }
    };
    if !session.ready {
        warn!("session not ready; dropping action");
        return None;
    }
    Some(ClientAction::RequestJob {
        session_id: session.session_id.clone(),
        nickname: "Desktop Cat".to_string(),
        sig: session.sig.clone(),
        kind,
    })
}

/// Send any queued actions over the socket.
fn flush_outgoing(conn: Option<NonSendMut<WsConn>>, mut outgoing: ResMut<OutgoingActions>) {
    let Some(mut conn) = conn else {
        return;
    };
    if outgoing.0.is_empty() {
        return;
    }
    for action in outgoing.0.drain(..) {
        if let Ok(json) = serde_json::to_string(&action) {
            conn.sender.send(WsMessage::Text(json));
        }
    }
}

// ---- pure colour / label helpers (unit-tested) ----

fn terrain_color(tile: &TerrainTile) -> Color {
    if tile.river.is_some() {
        return Color::srgb(0.20, 0.42, 0.72); // water
    }
    biome_color(tile.biome)
}

fn biome_color(biome: BiomeRole) -> Color {
    match biome {
        BiomeRole::Lowland => Color::srgb(0.28, 0.44, 0.24),
        BiomeRole::Grassland => Color::srgb(0.36, 0.54, 0.28),
        BiomeRole::Forest => Color::srgb(0.16, 0.32, 0.18),
        BiomeRole::Rocky => Color::srgb(0.45, 0.44, 0.40),
        BiomeRole::Highland => Color::srgb(0.62, 0.62, 0.60),
    }
}

fn decoration_color(decoration: Option<DecorationRole>) -> Option<Color> {
    match decoration {
        Some(DecorationRole::Tree { .. }) => Some(Color::srgb(0.10, 0.24, 0.12)),
        Some(DecorationRole::Rock { .. }) => Some(Color::srgb(0.55, 0.53, 0.50)),
        None => None,
    }
}

fn building_color(building: BuildingType) -> Color {
    match building {
        BuildingType::Shrine => Color::srgb(0.95, 0.82, 0.35),
        BuildingType::Den => Color::srgb(0.72, 0.52, 0.34),
        BuildingType::FoodStorage => Color::srgb(0.86, 0.66, 0.30),
        BuildingType::WaterBowl => Color::srgb(0.40, 0.68, 0.90),
        BuildingType::Beds => Color::srgb(0.70, 0.62, 0.80),
        BuildingType::HerbGarden => Color::srgb(0.50, 0.78, 0.42),
        BuildingType::Nursery => Color::srgb(0.94, 0.72, 0.80),
        BuildingType::ElderCorner => Color::srgb(0.66, 0.66, 0.72),
        BuildingType::Walls => Color::srgb(0.55, 0.52, 0.48),
        BuildingType::MouseFarm => Color::srgb(0.78, 0.70, 0.44),
        BuildingType::Workshop => Color::srgb(0.80, 0.50, 0.28),
        BuildingType::Field => Color::srgb(0.74, 0.78, 0.34),
        BuildingType::ResearchHut => Color::srgb(0.52, 0.60, 0.90),
        BuildingType::School => Color::srgb(0.58, 0.72, 0.92),
        BuildingType::Smithy => Color::srgb(0.62, 0.36, 0.30),
        BuildingType::Barracks => Color::srgb(0.80, 0.34, 0.34),
    }
}

fn building_label(building: BuildingType) -> &'static str {
    match building {
        BuildingType::Shrine => "shrine",
        BuildingType::Den => "den",
        BuildingType::FoodStorage => "food",
        BuildingType::WaterBowl => "water",
        BuildingType::Beds => "beds",
        BuildingType::HerbGarden => "herbs",
        BuildingType::Nursery => "nursery",
        BuildingType::ElderCorner => "elders",
        BuildingType::Walls => "walls",
        BuildingType::MouseFarm => "mousefarm",
        BuildingType::Workshop => "workshop",
        BuildingType::Field => "field",
        BuildingType::ResearchHut => "research",
        BuildingType::School => "school",
        BuildingType::Smithy => "smithy",
        BuildingType::Barracks => "barracks",
    }
}

fn cat_color(spec: Option<Specialization>) -> Color {
    match spec {
        Some(Specialization::Hunter) => Color::srgb(1.0, 0.80, 0.55),
        Some(Specialization::Architect) => Color::srgb(1.0, 0.90, 0.55),
        Some(Specialization::Ritualist) => Color::srgb(0.88, 0.70, 1.0),
        Some(Specialization::Warrior) => Color::srgb(1.0, 0.55, 0.55),
        None => Color::srgb(0.92, 0.92, 0.86),
    }
}

fn carrying_color(kind: CarryingKind) -> Color {
    match kind {
        CarryingKind::Food => Color::srgb(0.95, 0.55, 0.25),
        CarryingKind::Water => Color::srgb(0.35, 0.65, 0.95),
        CarryingKind::Materials => Color::srgb(0.70, 0.55, 0.35),
        CarryingKind::Blessings => Color::srgb(0.95, 0.85, 0.40),
    }
}

fn zone_color(kind: ZoneKind) -> Color {
    match kind {
        ZoneKind::Avoid => Color::srgba(0.90, 0.25, 0.25, 0.28),
        ZoneKind::Gather => Color::srgba(0.30, 0.85, 0.35, 0.28),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cat_protocol::WorldSnapshot;

    #[test]
    fn grid_projection_is_flat_and_top_down() {
        assert_eq!(grid_to_world(0, 0), Vec2::ZERO);
        assert_eq!(grid_to_world(1, 0), Vec2::new(TILE, 0.0));
        // Y increases downward on the grid → negative world Y.
        assert_eq!(grid_to_world(0, 1), Vec2::new(0.0, -TILE));
        assert_eq!(grid_to_world(2, 3), Vec2::new(2.0 * TILE, -3.0 * TILE));
    }

    #[test]
    fn window_terrain_covers_the_anchor_window() {
        let tiles = window_terrain(20_240_703);
        assert!(!tiles.is_empty());
        // The anchor tile must be present and within window bounds.
        assert!(
            tiles
                .iter()
                .any(|t| t.x == VILLAGE_ANCHOR.x && t.y == VILLAGE_ANCHOR.y)
        );
        for t in &tiles {
            assert!(
                (VILLAGE_ANCHOR.x - WINDOW_RADIUS..=VILLAGE_ANCHOR.x + WINDOW_RADIUS)
                    .contains(&t.x)
            );
            assert!(
                (VILLAGE_ANCHOR.y - WINDOW_RADIUS..=VILLAGE_ANCHOR.y + WINDOW_RADIUS)
                    .contains(&t.y)
            );
        }
    }

    #[test]
    fn biome_and_building_colors_are_distinct() {
        // Water beats biome when a river is present.
        let mut tile = window_terrain(20_240_703)
            .into_iter()
            .next()
            .expect("at least one terrain tile");
        tile.river = None;
        assert_eq!(terrain_color(&tile), biome_color(tile.biome));

        assert_ne!(
            building_color(BuildingType::Shrine),
            building_color(BuildingType::Smithy)
        );
        assert_ne!(
            cat_color(Some(Specialization::Hunter)),
            cat_color(Some(Specialization::Warrior))
        );
    }

    #[test]
    fn all_building_types_have_labels() {
        for building in [
            BuildingType::Shrine,
            BuildingType::Den,
            BuildingType::Workshop,
            BuildingType::Field,
            BuildingType::Smithy,
            BuildingType::Barracks,
        ] {
            assert!(!building_label(building).is_empty());
        }
    }

    #[test]
    fn deserializes_a_world_snapshot_and_counts_cats() {
        let json = r#"{
            "now": 0, "worldSeed": 1, "onlineCount": 2,
            "colonies": [{
                "id":"c1","name":"A","status":"thriving",
                "resources":{"food":1,"water":1,"herbs":0,"materials":0,"refined":0,"weapons":0,"armor":0,"blessings":0},
                "storage":{"capacities":{"food":200,"water":200,"herbs":100,"materials":100,"refined":100,"weapons":50,"armor":50},"foodCapacity":200,"titheRates":{"food":20,"refined":5}},
                "leader":null,
                "cats":[
                    {"id":"k1","name":"A","position":{"map":"colony","x":1,"y":2},"activity":"idle","destination":null,"carrying":null,"specialization":null,"ageHours":30.0,"needs":{"hunger":100,"thirst":100,"rest":100,"health":100},"currentTask":null,"assignedBuildingId":null,"roleXp":{"hunter":0,"architect":0,"ritualist":0,"warrior":0},"stats":{"leadership":10},"deathTime":null},
                    {"id":"k2","name":"B","position":{"map":"colony","x":3,"y":4},"activity":"idle","destination":null,"carrying":null,"specialization":null,"ageHours":30.0,"needs":{"hunger":100,"thirst":100,"rest":100,"health":100},"currentTask":null,"assignedBuildingId":null,"roleXp":{"hunter":0,"architect":0,"ritualist":0,"warrior":0},"stats":{"leadership":10},"deathTime":null}
                ],
                "jobs":[],"upgrades":[],"events":[],
                "housing":{"population":2,"capacity":4,"pressure":0.5,"villageLevel":1},
                "research":{"ownedNodeIds":[],"researchPoints":0,"researcherCount":0,"blessings":0,"nextTarget":null},
                "election":null,"voteKick":null,"zones":[],
                "threat":{"pressure":0,"band":"calm","raidActive":false,"warriors":0,"weapons":0,"armor":0},
                "raiders":[],"buildings":[],"claimedTiles":[],"villageGate":null,"villageRadius":4,"anchor":{"x":6,"y":6}
            }]
        }"#;
        let snap: WorldSnapshot = serde_json::from_str(json).expect("parse snapshot");
        assert_eq!(snap.colonies.len(), 1);
        assert_eq!(snap.colonies[0].cats.len(), 2);
        assert_eq!(snap.online_count, 2);
        assert!(dashboard_text(&snap.colonies[0], 2).contains("Colony: A"));
    }
}
