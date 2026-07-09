//! Axum server shell for the Cat Colony simulation, porting the transport around
//! `server/game.ts:workerTick` and `app/api/game/actions/route.ts`.

use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicU32, AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::get,
};
use cat_protocol::{ActionResult, ClientAction, WorldSnapshot};
use cat_sim::{
    actions::{ActionCtx, apply_action, build_snapshot},
    world_tick::{WorldState, found_colony, new_world, world_tick},
};
use tokio::sync::{Mutex, broadcast};
use tracing::{debug, error, info, warn};

const DEFAULT_PORT: u16 = 8787;
const WORLD_SEED: u32 = 20_240_703;
const STARTER_COLONY_ID: &str = "colony-1";
const STARTER_COLONY_SEED: u32 = 1;
const SNAPSHOT_CHANNEL_CAPACITY: usize = 32;

static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
struct AppState {
    world: Arc<Mutex<WorldState>>,
    snapshots: broadcast::Sender<WorldSnapshot>,
    online_count: Arc<AtomicU32>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let state = build_state(now_ms());
    spawn_tick_task(state.clone());

    let port = std::env::var("PORT")
        .ok()
        .and_then(|port| port.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!(%addr, "cat-server listening");
    axum::serve(listener, app(state)).await?;

    Ok(())
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ws", get(ws_handler))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

fn build_state(now_ms: i64) -> AppState {
    let mut world = new_world(WORLD_SEED);
    world.colonies.push(found_colony(
        WORLD_SEED,
        STARTER_COLONY_ID,
        now_ms,
        STARTER_COLONY_SEED,
    ));

    let (snapshots, _) = broadcast::channel(SNAPSHOT_CHANNEL_CAPACITY);

    AppState {
        world: Arc::new(Mutex::new(world)),
        snapshots,
        online_count: Arc::new(AtomicU32::new(0)),
    }
}

fn spawn_tick_task(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));

        loop {
            interval.tick().await;
            let now = now_ms();
            let online_count = state.online_count.load(Ordering::SeqCst);
            let snapshot = {
                let mut world = state.world.lock().await;
                let _reports = world_tick(&mut world, now);
                build_snapshot(&world, now, online_count)
            };

            if state.snapshots.send(snapshot).is_err() {
                debug!("no websocket snapshot receivers");
            }
        }
    });
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let connection_id = NEXT_CONNECTION_ID.fetch_add(1, Ordering::SeqCst);
    let session_id = format!("ws-{connection_id}");
    let online_count = state.online_count.fetch_add(1, Ordering::SeqCst) + 1;
    let mut snapshots = state.snapshots.subscribe();

    if send_current_snapshot(&mut socket, &state, online_count)
        .await
        .is_err()
    {
        state.online_count.fetch_sub(1, Ordering::SeqCst);
        return;
    }

    loop {
        tokio::select! {
            snapshot = snapshots.recv() => {
                match snapshot {
                    Ok(snapshot) => {
                        if send_snapshot(&mut socket, &snapshot).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!(skipped, "websocket client lagged behind snapshots");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            message = socket.recv() => {
                match message {
                    Some(Ok(Message::Text(text))) => {
                        let result = handle_client_text(&state, &session_id, text.as_str()).await;
                        if send_action_result(&mut socket, &result).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Binary(_)) | Ok(Message::Pong(_))) => {}
                    Some(Err(err)) => {
                        warn!(%err, "websocket receive failed");
                        break;
                    }
                }
            }
        }
    }

    state.online_count.fetch_sub(1, Ordering::SeqCst);
}

async fn send_current_snapshot(
    socket: &mut WebSocket,
    state: &AppState,
    online_count: u32,
) -> Result<(), axum::Error> {
    let now = now_ms();
    let snapshot = {
        let world = state.world.lock().await;
        build_snapshot(&world, now, online_count)
    };
    send_snapshot(socket, &snapshot).await
}

async fn handle_client_text(state: &AppState, session_id: &str, text: &str) -> ActionResult {
    let Ok(action) = serde_json::from_str::<ClientAction>(text) else {
        return ActionResult {
            ok: false,
            message: Some("Invalid action.".to_owned()),
        };
    };

    let ctx = ActionCtx {
        session_id: session_id.to_owned(),
        player_id: String::new(),
        now_ms: now_ms(),
    };

    let mut world = state.world.lock().await;
    apply_action(&mut world, &action, &ctx)
}

async fn send_snapshot(
    socket: &mut WebSocket,
    snapshot: &WorldSnapshot,
) -> Result<(), axum::Error> {
    send_serialized(socket, serde_json::to_string(snapshot)).await
}

async fn send_action_result(
    socket: &mut WebSocket,
    result: &ActionResult,
) -> Result<(), axum::Error> {
    send_serialized(socket, serde_json::to_string(result)).await
}

async fn send_serialized(
    socket: &mut WebSocket,
    serialized: Result<String, serde_json::Error>,
) -> Result<(), axum::Error> {
    match serialized {
        Ok(json) => socket.send(Message::Text(json.into())).await,
        Err(err) => {
            error!(%err, "failed to serialize websocket payload");
            socket
                .send(Message::Text(
                    r#"{"ok":false,"message":"Serialization failed."}"#.into(),
                ))
                .await
        }
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use cat_protocol::ClientAction;

    #[tokio::test]
    async fn found_village_action_updates_shared_snapshot() {
        let state = build_state(1_000_000);
        let action = ClientAction::FoundVillage {
            name: "Newford".to_owned(),
            session_id: "session-1".to_owned(),
        };
        let ctx = ActionCtx {
            session_id: "session-1".to_owned(),
            player_id: String::new(),
            now_ms: 1_000_000,
        };

        let snapshot = {
            let mut world = state.world.lock().await;
            let result = apply_action(&mut world, &action, &ctx);
            assert!(result.ok, "{result:?}");
            build_snapshot(&world, ctx.now_ms, 1)
        };

        assert_eq!(snapshot.colonies.len(), 2);
        assert!(
            snapshot
                .colonies
                .iter()
                .any(|colony| colony.name == "Newford")
        );
    }

    #[tokio::test]
    async fn client_action_round_trips_through_json() {
        let action = ClientAction::FoundVillage {
            name: "Newford".to_owned(),
            session_id: "session-1".to_owned(),
        };

        let encoded = serde_json::to_string(&action).expect("serialize action");
        let decoded: ClientAction = serde_json::from_str(&encoded).expect("deserialize action");

        assert_eq!(decoded, action);
    }
}
