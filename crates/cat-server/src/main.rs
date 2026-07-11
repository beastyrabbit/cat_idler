//! Axum server shell for the Cat Colony simulation, porting the transport around
//! `server/game.ts:workerTick` and `app/api/game/actions/route.ts`.

use std::{
    collections::BTreeMap,
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
use identity::{SignedSession, issue_session, signed_session, verify_session};
use persistence::{load_world, open_database_from_env, save_world};
use rate_limit::RateLimiter;
use rusqlite::Connection;
use tokio::sync::{Mutex, broadcast};
use tracing::{debug, error, info, warn};

mod identity;
mod persistence;
mod rate_limit;

const DEFAULT_PORT: u16 = 8787;
const WORLD_SEED: u32 = 20_240_703;
const STARTER_COLONY_ID: &str = "colony-1";
const STARTER_COLONY_SEED: u32 = 1;
const SNAPSHOT_CHANNEL_CAPACITY: usize = 32;
const ACTION_LIMIT_MAX: usize = 30;
const ACTION_LIMIT_WINDOW_MS: i64 = 10_000;
const SAVE_EVERY_TICKS: u64 = 5;

static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
struct AppState {
    world: Arc<Mutex<WorldState>>,
    db: Arc<Mutex<Connection>>,
    snapshots: broadcast::Sender<WorldSnapshot>,
    online_count: Arc<AtomicU32>,
    rate_limiter: Arc<Mutex<RateLimiter>>,
    session_secret: Arc<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let conn = open_database_from_env()?;
    let session_secret = identity::session_secret_from_env()?;
    let state = build_state_from_connection(now_ms(), conn, session_secret)?;
    spawn_tick_task(state.clone());

    let port = std::env::var("PORT")
        .ok()
        .and_then(|port| port.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!(%addr, "cat-server listening");
    axum::serve(listener, app(state.clone()))
        .with_graceful_shutdown(shutdown_signal(state))
        .await?;

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

fn starter_world(now_ms: i64) -> WorldState {
    let mut world = new_world(WORLD_SEED);
    world.colonies.push(found_colony(
        WORLD_SEED,
        STARTER_COLONY_ID,
        now_ms,
        STARTER_COLONY_SEED,
    ));
    world
}

fn build_state_from_connection(
    now_ms: i64,
    conn: Connection,
    session_secret: String,
) -> rusqlite::Result<AppState> {
    let world = load_world(&conn)?.unwrap_or_else(|| starter_world(now_ms));
    Ok(build_state_from_world(world, conn, session_secret))
}

fn build_state_from_world(world: WorldState, conn: Connection, session_secret: String) -> AppState {
    let (snapshots, _) = broadcast::channel(SNAPSHOT_CHANNEL_CAPACITY);

    AppState {
        world: Arc::new(Mutex::new(world)),
        db: Arc::new(Mutex::new(conn)),
        snapshots,
        online_count: Arc::new(AtomicU32::new(0)),
        rate_limiter: Arc::new(Mutex::new(RateLimiter::new(
            ACTION_LIMIT_MAX,
            ACTION_LIMIT_WINDOW_MS,
        ))),
        session_secret: Arc::new(session_secret),
    }
}

#[cfg(test)]
fn build_state(now_ms: i64) -> AppState {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    persistence::init_schema(&conn).expect("init in-memory schema");
    build_state_from_world(
        starter_world(now_ms),
        conn,
        "test-session-secret".to_owned(),
    )
}

fn spawn_tick_task(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        let mut ticks = 0_u64;

        loop {
            interval.tick().await;
            ticks = ticks.saturating_add(1);
            let now = now_ms();
            let online_count = state.online_count.load(Ordering::SeqCst);
            let snapshot = {
                let mut world = state.world.lock().await;
                let _reports = world_tick(&mut world, now);
                if ticks.is_multiple_of(SAVE_EVERY_TICKS) {
                    let db = state.db.lock().await;
                    if let Err(err) = save_world(&db, &world) {
                        error!(%err, "periodic world save failed");
                    }
                }
                build_snapshot(&world, now, online_count)
            };

            if state.snapshots.send(snapshot).is_err() {
                debug!("no websocket snapshot receivers");
            }
        }
    });
}

async fn shutdown_signal(state: AppState) {
    // Save on Ctrl-C (SIGINT) *and* SIGTERM (kill / systemd / docker stop) so a
    // just-founded village is never lost on a normal shutdown.
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            warn!(%err, "failed to install ctrl_c handler");
            std::future::pending::<()>().await;
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(err) => {
                warn!(%err, "failed to install SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }

    if let Err(err) = save_current_world(&state).await {
        error!(%err, "shutdown world save failed");
    }
}

async fn save_current_world(state: &AppState) -> rusqlite::Result<()> {
    let world = state.world.lock().await;
    let db = state.db.lock().await;
    save_world(&db, &world)
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ServerActionResult {
    result: ActionResult,
    fields: BTreeMap<&'static str, String>,
}

impl ServerActionResult {
    fn from_result(result: ActionResult) -> Self {
        Self {
            result,
            fields: BTreeMap::new(),
        }
    }

    fn ok() -> Self {
        Self::from_result(ActionResult {
            ok: true,
            message: None,
        })
    }

    fn fail(message: impl Into<String>) -> Self {
        Self::from_result(ActionResult {
            ok: false,
            message: Some(message.into()),
        })
    }

    fn with_signed_session(mut self, signed: SignedSession) -> Self {
        self.fields.insert("sessionId", signed.session_id);
        self.fields.insert("sig", signed.sig);
        self.fields.insert("playerId", signed.player_id);
        self
    }

    fn serialize(&self) -> Result<String, serde_json::Error> {
        let mut object = serde_json::Map::new();
        object.insert("ok".to_owned(), serde_json::Value::Bool(self.result.ok));
        if let Some(message) = &self.result.message {
            object.insert(
                "message".to_owned(),
                serde_json::Value::String(message.clone()),
            );
        }
        for (key, value) in &self.fields {
            object.insert((*key).to_owned(), serde_json::Value::String(value.clone()));
        }
        serde_json::to_string(&serde_json::Value::Object(object))
    }
}

async fn handle_client_text(
    state: &AppState,
    connection_session_id: &str,
    text: &str,
) -> ServerActionResult {
    let Ok(action) = serde_json::from_str::<ClientAction>(text) else {
        return ServerActionResult::fail("Invalid action.");
    };

    let now = now_ms();
    let limiter_key = rate_limiter_key(
        &action,
        state.session_secret.as_str(),
        connection_session_id,
    );
    {
        let mut limiter = state.rate_limiter.lock().await;
        limiter.prune(now);
        if !limiter.check(&limiter_key, now) {
            return ServerActionResult::fail("Too many actions — slow down.");
        }
    }

    if let ClientAction::Presence {
        session_id, sig, ..
    } = &action
    {
        let signed = if verify_session(session_id, sig.as_deref(), state.session_secret.as_str()) {
            signed_session(session_id.clone(), state.session_secret.as_str())
        } else {
            issue_session(state.session_secret.as_str(), now)
        };
        return ServerActionResult::ok().with_signed_session(signed);
    }

    let identity = match verified_identity(&action, state.session_secret.as_str()) {
        Ok(identity) => identity,
        Err(message) => return ServerActionResult::fail(message),
    };
    let ctx_session_id = identity.as_ref().map_or_else(
        || action_session_id(&action, connection_session_id),
        |identity| identity.session_id.clone(),
    );
    let player_id = identity
        .as_ref()
        .map_or_else(String::new, |identity| identity.player_id.clone());

    let ctx = ActionCtx {
        session_id: ctx_session_id,
        player_id,
        now_ms: now,
    };

    let mut world = state.world.lock().await;
    ServerActionResult::from_result(apply_action(&mut world, &action, &ctx))
}

fn rate_limiter_key(action: &ClientAction, secret: &str, fallback: &str) -> String {
    if let Some((session_id, sig)) = action_identity_fields(action)
        && verify_session(session_id, sig, secret)
    {
        return format!("s:{session_id}");
    }

    format!("ip:{fallback}")
}

fn verified_identity(action: &ClientAction, secret: &str) -> Result<Option<SignedSession>, String> {
    let Some((session_id, sig)) = action_identity_fields(action) else {
        return Ok(None);
    };
    if verify_session(session_id, sig, secret) {
        Ok(Some(signed_session(session_id.to_owned(), secret)))
    } else {
        Err(
            "Session signature missing or invalid. Refresh to re-establish your session."
                .to_owned(),
        )
    }
}

fn action_identity_fields(action: &ClientAction) -> Option<(&str, Option<&str>)> {
    match action {
        ClientAction::Presence {
            session_id, sig, ..
        } => Some((session_id, sig.as_deref())),
        ClientAction::RequestJob {
            session_id, sig, ..
        }
        | ClientAction::Boost {
            session_id, sig, ..
        }
        | ClientAction::PurchaseUpgrade {
            session_id, sig, ..
        }
        | ClientAction::CastVote {
            session_id, sig, ..
        }
        | ClientAction::RequestVoteKick {
            session_id, sig, ..
        }
        | ClientAction::CreateZone {
            session_id, sig, ..
        }
        | ClientAction::RemoveZone {
            session_id, sig, ..
        }
        | ClientAction::PlanBuilding {
            session_id, sig, ..
        }
        | ClientAction::UnlockNode {
            session_id, sig, ..
        }
        | ClientAction::AssignWorker {
            session_id, sig, ..
        }
        | ClientAction::TrainWarrior {
            session_id, sig, ..
        }
        | ClientAction::DefendRaid {
            session_id, sig, ..
        }
        | ClientAction::BuildRoad {
            session_id, sig, ..
        }
        | ClientAction::SellGoods {
            session_id, sig, ..
        }
        | ClientAction::BuyResource {
            session_id, sig, ..
        }
        | ClientAction::BoostCat {
            session_id, sig, ..
        } => Some((session_id, Some(sig))),
        _ => None,
    }
}

fn action_session_id(action: &ClientAction, fallback: &str) -> String {
    match action {
        ClientAction::FoundVillage { session_id, .. }
        | ClientAction::JoinVillage { session_id, .. } => session_id.clone(),
        _ => fallback.to_owned(),
    }
}

async fn send_snapshot(
    socket: &mut WebSocket,
    snapshot: &WorldSnapshot,
) -> Result<(), axum::Error> {
    send_serialized(socket, serde_json::to_string(snapshot)).await
}

async fn send_action_result(
    socket: &mut WebSocket,
    result: &ServerActionResult,
) -> Result<(), axum::Error> {
    send_serialized(socket, result.serialize()).await
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
