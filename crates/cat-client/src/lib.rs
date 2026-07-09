//! Cat colony Bevy client — foundation.
//!
//! Connects to `cat-server` over a WebSocket, deserializes the live
//! [`cat_protocol::WorldSnapshot`] each tick, and renders the world. This module
//! is the P9 foundation: camera, WS ingest, cat rendering, and a HUD. Terrain,
//! buildings, input tools, and the dashboard land in later cards; the verified
//! render spike at `reference/spike-bevy-0.19.rs` is the source for iso art.

use bevy::prelude::*;
use cat_protocol::WorldSnapshot;
use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};

const TILE_W: f32 = 64.0;
const TILE_H: f32 = 32.0;

/// Isometric projection of a (fractional) tile coordinate to world space,
/// lifted from the render spike.
fn iso_point(x: f32, y: f32) -> Vec2 {
    Vec2::new((x - y) * (TILE_W / 2.0), -(x + y) * (TILE_H / 2.0))
}

/// The most recent snapshot pushed by the server.
#[derive(Resource, Default)]
struct LatestSnapshot(Option<WorldSnapshot>);

/// The live WebSocket connection (kept off the render threads).
struct WsConn {
    _sender: WsSender,
    receiver: WsReceiver,
}

/// Marker for a spawned cat sprite so we can clear + redraw each update.
#[derive(Component)]
struct CatSprite;

/// Marker for the HUD text.
#[derive(Component)]
struct HudText;

/// Build and run the client. `CAT_SERVER_URL` overrides the server address.
pub fn run() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    // Load `public/images/...` relative to BEVY_ASSET_ROOT / cwd.
                    file_path: ".".to_string(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Cat Colony".to_string(),
                        resolution: bevy::window::WindowResolution::new(1280, 800),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(LatestSnapshot::default())
        .insert_resource(ClearColor(Color::srgb(0.07, 0.10, 0.09)))
        .add_systems(Startup, (setup, connect_ws))
        .add_systems(Update, (poll_ws, render_cats, update_hud))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        Text::new("connecting…"),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            ..default()
        },
        HudText,
    ));
}

/// Open the WebSocket as a non-send resource (the receiver is `!Sync`).
fn connect_ws(world: &mut World) {
    let url =
        std::env::var("CAT_SERVER_URL").unwrap_or_else(|_| "ws://127.0.0.1:8787/ws".to_string());
    match ewebsock::connect(url.clone(), ewebsock::Options::default()) {
        Ok((sender, receiver)) => {
            eprintln!("cat-client connecting to {url}");
            world.insert_non_send(WsConn {
                _sender: sender,
                receiver,
            });
        }
        Err(err) => eprintln!("cat-client failed to connect to {url}: {err}"),
    }
}

fn poll_ws(conn: Option<NonSend<WsConn>>, mut latest: ResMut<LatestSnapshot>) {
    let Some(conn) = conn else {
        return;
    };
    while let Some(event) = conn.receiver.try_recv() {
        if let WsEvent::Message(WsMessage::Text(text)) = event {
            match serde_json::from_str::<WorldSnapshot>(&text) {
                Ok(snapshot) => latest.0 = Some(snapshot),
                Err(err) => eprintln!("bad snapshot: {err}"),
            }
        }
    }
}

fn render_cats(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    existing: Query<Entity, With<CatSprite>>,
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
        let p = iso_point(cat.position.x as f32, cat.position.y as f32);
        commands.spawn((
            Sprite::from_color(Color::srgb(0.9, 0.8, 0.4), Vec2::splat(10.0)),
            Transform::from_xyz(p.x, p.y, 1.0),
            CatSprite,
        ));
    }
}

fn update_hud(latest: Res<LatestSnapshot>, mut hud: Query<&mut Text, With<HudText>>) {
    if !latest.is_changed() {
        return;
    }
    let Ok(mut text) = hud.single_mut() else {
        return;
    };
    match latest.0.as_ref() {
        Some(w) => {
            let cats: usize = w.colonies.iter().map(|c| c.cats.len()).sum();
            text.0 = format!(
                "colonies: {}  cats: {}  online: {}",
                w.colonies.len(),
                cats,
                w.online_count
            );
        }
        None => text.0 = "connecting…".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_point_is_deterministic() {
        assert_eq!(iso_point(0.0, 0.0), Vec2::ZERO);
        assert_eq!(iso_point(1.0, 0.0), Vec2::new(TILE_W / 2.0, -TILE_H / 2.0));
    }

    #[test]
    fn deserializes_a_world_snapshot_and_counts_cats() {
        // A minimal snapshot JSON with one colony and two cats.
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
    }
}
