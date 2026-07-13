//! Axum server shell for the Cat Colony simulation, porting the transport around
//! `server/game.ts:workerTick` and `app/api/game/actions/route.ts`.

use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicU32, AtomicU64, Ordering},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use axum::{
    Router,
    body::Body,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{
        Request, StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS},
    },
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use cat_protocol::{ActionResult, ClientAction, WorldSnapshot};
use cat_sim::{
    actions::{ActionCtx, apply_action, build_snapshot},
    world_tick::{WorldState, found_colony, new_world, world_tick},
};
use hosting::ServerConfig;
use identity::{SignedSession, issue_session, signed_session, verify_session};
use persistence::{load_world, open_database_from_env, save_world};
use rate_limit::RateLimiter;
use rusqlite::Connection;
use tokio::sync::{Mutex, RwLock, broadcast};
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
};
use tracing::{debug, error, info, warn};

mod hosting;
mod identity;
mod persistence;
mod rate_limit;

const WORLD_SEED: u32 = 20_240_703;
const STARTER_COLONY_ID: &str = "colony-1";
const STARTER_COLONY_SEED: u32 = 1;
const SNAPSHOT_CHANNEL_CAPACITY: usize = 32;
const ACTION_LIMIT_MAX: usize = 30;
const ACTION_LIMIT_WINDOW_MS: i64 = 10_000;
const SAVE_EVERY_TICKS: u64 = 5;
const TEST_ACTIONS_ENV: &str = "CAT_SERVER_ENABLE_TEST_ACTIONS";

static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
struct AppState {
    world: Arc<Mutex<WorldState>>,
    db: Arc<Mutex<Connection>>,
    /// Last snapshot produced by a fully completed tick (or the startup world).
    ///
    /// WebSocket handshakes read this cache instead of waiting behind the simulation's
    /// world lock. A slow tick can therefore never turn a new connection into a blank
    /// client, and each socket still applies its own selected-colony ordering.
    completed_snapshot: Arc<RwLock<WorldSnapshot>>,
    snapshots: broadcast::Sender<WorldSnapshot>,
    online_count: Arc<AtomicU32>,
    rate_limiter: Arc<Mutex<RateLimiter>>,
    session_secret: Arc<String>,
    allow_test_actions: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let conn = open_database_from_env()?;
    let session_secret = identity::session_secret_from_env()?;
    let config = ServerConfig::from_env()?;
    let state = build_state_from_connection(now_ms(), conn, session_secret)?;
    if state.allow_test_actions {
        warn!(
            env = TEST_ACTIONS_ENV,
            "development-only WebSocket test actions are enabled"
        );
    }
    spawn_tick_task(state.clone());

    let listener = tokio::net::TcpListener::bind(config.listen_addr).await?;

    info!(addr = %config.listen_addr, "cat-server listening");
    if let Some(dist) = &config.web_dist {
        info!(path = %dist.display(), "serving browser client");
    }
    if config.allowed_origins.is_restricted() {
        info!("strict WebSocket Origin allowlist enabled");
    }
    axum::serve(listener, app(state.clone(), &config))
        .with_graceful_shutdown(shutdown_signal(state))
        .await?;

    Ok(())
}

fn app(state: AppState, config: &ServerConfig) -> Router {
    let mut router = Router::new()
        .route("/health", get(health))
        .route("/ready", get(readiness))
        .route(
            "/ws",
            get(ws_handler).route_layer(middleware::from_fn_with_state(
                config.allowed_origins.clone(),
                enforce_ws_origin,
            )),
        )
        .with_state(state);

    if let Some(images) = &config.public_images {
        let image_router = Router::new()
            .fallback_service(ServeDir::new(images))
            .layer(middleware::from_fn(static_cache_headers));
        router = router.nest("/public/images", image_router);
    }

    if let Some(dist) = &config.web_dist {
        let static_router = Router::new()
            .fallback_service(
                ServeDir::new(dist).fallback(ServeFile::new(hosting::index_path(dist))),
            )
            .layer(middleware::from_fn(static_cache_headers));
        router = router.merge(static_router);
    }

    router.layer(CompressionLayer::new().br(true).gzip(true))
}

async fn health() -> &'static str {
    "ok"
}

async fn readiness(State(state): State<AppState>) -> impl IntoResponse {
    let database_ready = state
        .db
        .lock()
        .await
        .query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
        .is_ok_and(|value| value == 1);
    let world_ready = !state.world.lock().await.colonies.is_empty();

    if database_ready && world_ready {
        (StatusCode::OK, "ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready")
    }
}

async fn enforce_ws_origin(
    State(allowed_origins): State<hosting::AllowedOrigins>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if !allowed_origins.request_origin_allowed(request.headers()) {
        warn!("rejected WebSocket connection with an untrusted Origin");
        return (StatusCode::FORBIDDEN, "WebSocket Origin is not allowed").into_response();
    }
    next.run(request).await
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
        .into_response()
}

async fn static_cache_headers(request: Request<Body>, next: Next) -> Response {
    let path = request.uri().path().to_owned();
    let mut response = next.run(request).await;
    if response.status().is_success() {
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok());
        let is_html = content_type.is_some_and(|value| value.starts_with("text/html"));
        let is_image = content_type.is_some_and(|value| value.starts_with("image/"));
        let cache_control = if is_html {
            "no-cache"
        } else if is_fingerprinted_asset(&path) {
            "public, max-age=31536000, immutable"
        } else if is_image {
            "public, max-age=86400"
        } else {
            "public, max-age=3600"
        };
        response.headers_mut().insert(
            CACHE_CONTROL,
            cache_control.parse().expect("static cache header is valid"),
        );
        response.headers_mut().insert(
            X_CONTENT_TYPE_OPTIONS,
            "nosniff".parse().expect("nosniff header is valid"),
        );
    }
    response
}

fn is_fingerprinted_asset(path: &str) -> bool {
    let Some(file_name) = path.rsplit('/').next() else {
        return false;
    };
    file_name
        .split(['-', '_', '.'])
        .any(|part| part.len() >= 8 && part.bytes().all(|byte| byte.is_ascii_hexdigit()))
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
    Ok(build_state_from_world(
        world,
        conn,
        session_secret,
        test_actions_enabled(),
        now_ms,
    ))
}

fn build_state_from_world(
    world: WorldState,
    conn: Connection,
    session_secret: String,
    allow_test_actions: bool,
    now_ms: i64,
) -> AppState {
    let (snapshots, _) = broadcast::channel(SNAPSHOT_CHANNEL_CAPACITY);
    let completed_snapshot = build_snapshot(&world, now_ms, 0);

    AppState {
        world: Arc::new(Mutex::new(world)),
        db: Arc::new(Mutex::new(conn)),
        completed_snapshot: Arc::new(RwLock::new(completed_snapshot)),
        snapshots,
        online_count: Arc::new(AtomicU32::new(0)),
        rate_limiter: Arc::new(Mutex::new(RateLimiter::new(
            ACTION_LIMIT_MAX,
            ACTION_LIMIT_WINDOW_MS,
        ))),
        session_secret: Arc::new(session_secret),
        allow_test_actions,
    }
}

/// Enable deterministic automation actions only for an explicitly opted-in debug
/// build. A release binary cannot expose them even if the environment is wrong.
#[cfg(debug_assertions)]
fn test_actions_enabled() -> bool {
    std::env::var(TEST_ACTIONS_ENV)
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

#[cfg(not(debug_assertions))]
fn test_actions_enabled() -> bool {
    false
}

#[cfg(test)]
fn build_state(now_ms: i64) -> AppState {
    let conn = Connection::open_in_memory().expect("open in-memory sqlite");
    persistence::init_schema(&conn).expect("init in-memory schema");
    build_state_from_world(
        starter_world(now_ms),
        conn,
        "test-session-secret".to_owned(),
        false,
        now_ms,
    )
}

fn spawn_tick_task(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut ticks = 0_u64;

        loop {
            interval.tick().await;
            ticks = ticks.saturating_add(1);
            let now = now_ms();
            if let Err(err) = run_tick_once(state.clone(), ticks, now, |world, tick_now| {
                let _reports = world_tick(world, tick_now);
            })
            .await
            {
                error!(%err, "simulation tick worker failed");
            }
        }
    });
}

#[derive(Debug)]
struct CompletedTick {
    snapshot: WorldSnapshot,
    simulation_ms: u128,
    snapshot_ms: u128,
    persistence_ms: u128,
}

/// Run CPU-heavy simulation and synchronous SQLite work outside Tokio's async workers.
///
/// The world mutex still serializes authoritative mutations, preserving the action/tick
/// ordering that the server had before this worker boundary. Snapshot construction happens
/// before the cache is published, while persistence uses a clone so slow disk I/O does not
/// extend the authoritative-world lock.
async fn run_tick_once(
    state: AppState,
    ticks: u64,
    now: i64,
    tick_world: impl FnOnce(&mut WorldState, i64) + Send + 'static,
) -> Result<(), tokio::task::JoinError> {
    let worker_state = state.clone();
    let completed = tokio::task::spawn_blocking(move || {
        let online_count = worker_state.online_count.load(Ordering::SeqCst);
        let simulation_started = Instant::now();
        let mut world = worker_state.world.blocking_lock();
        tick_world(&mut world, now);
        let simulation_ms = simulation_started.elapsed().as_millis();

        let snapshot_started = Instant::now();
        let snapshot = build_snapshot(&world, now, online_count);
        let snapshot_ms = snapshot_started.elapsed().as_millis();
        let persistence_started = Instant::now();
        let world_to_save = ticks
            .is_multiple_of(SAVE_EVERY_TICKS)
            .then(|| world.clone());
        drop(world);

        if let Some(world) = world_to_save {
            let db = worker_state.db.blocking_lock();
            if let Err(err) = save_world(&db, &world) {
                error!(%err, "periodic world save failed");
            }
        }

        CompletedTick {
            snapshot,
            simulation_ms,
            snapshot_ms,
            persistence_ms: persistence_started.elapsed().as_millis(),
        }
    })
    .await?;

    debug!(
        simulation_ms = completed.simulation_ms,
        snapshot_ms = completed.snapshot_ms,
        persistence_ms = completed.persistence_ms,
        "simulation tick completed"
    );
    *state.completed_snapshot.write().await = completed.snapshot.clone();
    if state.snapshots.send(completed.snapshot).is_err() {
        debug!("no websocket snapshot receivers");
    }
    Ok(())
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
    let mut connection = ConnectionContext::new(format!("ws-{connection_id}"));
    let online_count = state.online_count.fetch_add(1, Ordering::SeqCst) + 1;
    let mut snapshots = state.snapshots.subscribe();

    if send_current_snapshot(&mut socket, &state, online_count, &connection.colony_id)
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
                        let snapshot = prioritize_colony(snapshot, &connection.colony_id);
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
                        let result = handle_client_text(&state, &mut connection, text.as_str()).await;
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
    colony_id: &str,
) -> Result<(), axum::Error> {
    let snapshot = current_snapshot(state, online_count, colony_id).await;
    send_snapshot(socket, &snapshot).await
}

async fn current_snapshot(state: &AppState, online_count: u32, colony_id: &str) -> WorldSnapshot {
    let mut snapshot = state.completed_snapshot.read().await.clone();
    snapshot.online_count = online_count;
    prioritize_colony(snapshot, colony_id)
}

/// Keep the socket-selected colony first because the current client renders the
/// first colony while retaining the complete shared-world snapshot for world-map
/// features.
fn prioritize_colony(mut snapshot: WorldSnapshot, colony_id: &str) -> WorldSnapshot {
    if let Some(index) = snapshot
        .colonies
        .iter()
        .position(|colony| colony.id == colony_id)
    {
        snapshot.colonies.swap(0, index);
    }
    snapshot
}

#[derive(Debug)]
struct ConnectionContext {
    limiter_fallback: String,
    identity: Option<SignedSession>,
    colony_id: String,
}

impl ConnectionContext {
    fn new(limiter_fallback: String) -> Self {
        Self {
            limiter_fallback,
            identity: None,
            colony_id: STARTER_COLONY_ID.to_owned(),
        }
    }

    fn limiter_key(&self) -> String {
        self.identity.as_ref().map_or_else(
            || format!("ip:{}", self.limiter_fallback),
            |identity| format!("s:{}", identity.session_id),
        )
    }
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
    connection: &mut ConnectionContext,
    text: &str,
) -> ServerActionResult {
    let Ok(action) = serde_json::from_str::<ClientAction>(text) else {
        return ServerActionResult::fail("Invalid action.");
    };

    let now = now_ms();
    let limiter_key = connection.limiter_key();
    {
        let mut limiter = state.rate_limiter.lock().await;
        limiter.prune(now);
        if !limiter.check(&limiter_key, now) {
            return ServerActionResult::fail("Too many actions — slow down.");
        }
    }

    let authentication = action_authentication(&action);
    if let ActionAuthentication::Presence { session_id, sig } = authentication {
        let signed = if verify_session(session_id, sig, state.session_secret.as_str()) {
            signed_session(session_id.to_owned(), state.session_secret.as_str())
        } else {
            issue_session(state.session_secret.as_str(), now)
        };
        if connection
            .identity
            .as_ref()
            .is_some_and(|identity| identity.session_id != signed.session_id)
        {
            connection.colony_id = STARTER_COLONY_ID.to_owned();
        }
        connection.identity = Some(signed.clone());
        return ServerActionResult::ok().with_signed_session(signed);
    }

    let Some(identity) = connection.identity.clone() else {
        return ServerActionResult::fail(
            "Authenticate with presence before sending actions.".to_owned(),
        );
    };

    match authentication {
        ActionAuthentication::Presence { .. } => unreachable!("handled above"),
        ActionAuthentication::Signed { session_id, sig } => {
            if session_id != identity.session_id
                || !verify_session(session_id, Some(sig), state.session_secret.as_str())
            {
                return ServerActionResult::fail(
                    "Session signature missing or invalid. Refresh to re-establish your session."
                        .to_owned(),
                );
            }
        }
        ActionAuthentication::SessionBound { session_id } => {
            if session_id != identity.session_id {
                return ServerActionResult::fail(
                    "Action session does not match this connection.".to_owned(),
                );
            }
        }
        ActionAuthentication::TestOnly => {
            if !state.allow_test_actions {
                return ServerActionResult::fail(
                    "Test actions are disabled on this server.".to_owned(),
                );
            }
        }
    }

    let ctx = ActionCtx {
        session_id: identity.session_id,
        player_id: identity.player_id,
        colony_id: connection.colony_id.clone(),
        now_ms: now,
    };

    let mut world = state.world.lock().await;
    let result = apply_action(&mut world, &action, &ctx);
    if result.ok {
        match &action {
            ClientAction::FoundVillage { .. } => {
                if let Some(colony) = world.colonies.last() {
                    connection.colony_id.clone_from(&colony.id);
                }
            }
            ClientAction::JoinVillage { colony_id, .. } => {
                connection.colony_id.clone_from(colony_id);
            }
            _ => {}
        }
    }
    ServerActionResult::from_result(result)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionAuthentication<'a> {
    Presence {
        session_id: &'a str,
        sig: Option<&'a str>,
    },
    Signed {
        session_id: &'a str,
        sig: &'a str,
    },
    /// Found/join predate per-action signatures. They are accepted only after a
    /// signed Presence handshake and only for that exact socket-bound session.
    SessionBound {
        session_id: &'a str,
    },
    TestOnly,
}

/// Exhaustive authentication policy. Adding a protocol action is a compile error
/// until its production authentication class is chosen deliberately.
fn action_authentication(action: &ClientAction) -> ActionAuthentication<'_> {
    match action {
        ClientAction::Presence {
            session_id, sig, ..
        } => ActionAuthentication::Presence {
            session_id,
            sig: sig.as_deref(),
        },
        ClientAction::RequestJob {
            session_id, sig, ..
        }
        | ClientAction::DispatchScout {
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
        }
        | ClientAction::AssignOfficer {
            session_id, sig, ..
        }
        | ClientAction::UnassignOfficer {
            session_id, sig, ..
        }
        | ClientAction::DesignateFarm {
            session_id, sig, ..
        }
        | ClientAction::ClearFarm {
            session_id, sig, ..
        }
        | ClientAction::DesignateStockpile {
            session_id, sig, ..
        }
        | ClientAction::RemoveStockpile {
            session_id, sig, ..
        }
        | ClientAction::DesignateGatherSpot {
            session_id, sig, ..
        }
        | ClientAction::RemoveGatherSpot {
            session_id, sig, ..
        } => ActionAuthentication::Signed { session_id, sig },
        ClientAction::FoundVillage { session_id, .. }
        | ClientAction::JoinVillage { session_id, .. } => {
            ActionAuthentication::SessionBound { session_id }
        }
        ClientAction::Ensure
        | ClientAction::SetTestAcceleration { .. }
        | ClientAction::AdvanceTime { .. }
        | ClientAction::SetTestRngSeed { .. } => ActionAuthentication::TestOnly,
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
    use axum::{
        body::to_bytes,
        http::{
            Method,
            header::{CONTENT_ENCODING, ORIGIN},
        },
    };
    use cat_protocol::{
        AccelerationPreset, ClientAction, CropKind, OfficerRole, ResourceKind, ScoutMission,
        TilePoint,
    };
    use std::{fs, path::PathBuf, time::Duration};
    use tower::ServiceExt;

    static NEXT_STATIC_FIXTURE_ID: AtomicU64 = AtomicU64::new(1);

    struct StaticFixture {
        root: PathBuf,
        dist: PathBuf,
        images: PathBuf,
    }

    impl StaticFixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "cat-server-router-{}-{}",
                std::process::id(),
                NEXT_STATIC_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
            ));
            let dist = root.join("dist");
            let images = root.join("images");
            fs::create_dir_all(&dist).expect("create dist");
            fs::create_dir_all(&images).expect("create images");
            fs::write(
                dist.join("index.html"),
                format!(
                    "<!doctype html><title>Cats</title><p>{}</p>",
                    "cat".repeat(1_000)
                ),
            )
            .expect("write index");
            Self { root, dist, images }
        }

        fn config(&self) -> ServerConfig {
            ServerConfig {
                listen_addr: "127.0.0.1:8787".parse().expect("test listen address"),
                web_dist: Some(self.dist.clone()),
                public_images: Some(self.images.clone()),
                allowed_origins: hosting::AllowedOrigins::default(),
            }
        }
    }

    impl Drop for StaticFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn local_config() -> ServerConfig {
        ServerConfig {
            listen_addr: "127.0.0.1:8787".parse().expect("test listen address"),
            web_dist: None,
            public_images: None,
            allowed_origins: hosting::AllowedOrigins::default(),
        }
    }

    fn request(path: &str) -> Request<Body> {
        Request::builder()
            .method(Method::GET)
            .uri(path)
            .body(Body::empty())
            .expect("build request")
    }

    #[tokio::test]
    async fn liveness_and_readiness_report_distinct_server_health() {
        let state = build_state(1_000_000);
        let router = app(state.clone(), &local_config());
        let health_response = router
            .clone()
            .oneshot(request("/health"))
            .await
            .expect("health response");
        assert_eq!(health_response.status(), StatusCode::OK);
        let ready_response = router
            .oneshot(request("/ready"))
            .await
            .expect("readiness response");
        assert_eq!(ready_response.status(), StatusCode::OK);

        state.world.lock().await.colonies.clear();
        let not_ready_response = app(state, &local_config())
            .oneshot(request("/ready"))
            .await
            .expect("not-ready response");
        assert_eq!(not_ready_response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn slow_tick_keeps_liveness_and_initial_snapshot_responsive_on_one_worker() {
        let state = build_state(1_000_000);
        let startup_snapshot = state.completed_snapshot.read().await.clone();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let tick_state = state.clone();
        let slow_tick = tokio::spawn(async move {
            run_tick_once(tick_state, 1, 1_001_000, move |world, tick_now| {
                let _ = started_tx.send(());
                std::thread::sleep(Duration::from_millis(250));
                let _reports = world_tick(world, tick_now);
            })
            .await
        });

        tokio::time::timeout(Duration::from_millis(100), started_rx)
            .await
            .expect("blocking tick starts without occupying the sole async worker")
            .expect("tick start signal");
        assert!(
            state.world.try_lock().is_err(),
            "injected tick should hold the authoritative world lock"
        );

        let health_response = tokio::time::timeout(
            Duration::from_millis(50),
            app(state.clone(), &local_config()).oneshot(request("/health")),
        )
        .await
        .expect("liveness stays responsive during slow simulation")
        .expect("health response");
        assert_eq!(health_response.status(), StatusCode::OK);

        let initial = tokio::time::timeout(
            Duration::from_millis(50),
            current_snapshot(&state, 7, STARTER_COLONY_ID),
        )
        .await
        .expect("websocket initial snapshot reads the completed cache");
        assert_eq!(initial.now, startup_snapshot.now);
        assert_eq!(initial.online_count, 7);
        assert_eq!(initial.colonies, startup_snapshot.colonies);

        slow_tick
            .await
            .expect("slow tick task")
            .expect("blocking tick worker");
        assert_eq!(state.completed_snapshot.read().await.now, 1_001_000);
    }

    #[tokio::test]
    async fn cached_initial_snapshot_prioritizes_each_socket_without_global_reordering() {
        let mut world = new_world(WORLD_SEED);
        world.colonies.push(found_colony(
            WORLD_SEED,
            STARTER_COLONY_ID,
            1_000_000,
            STARTER_COLONY_SEED,
        ));
        world
            .colonies
            .push(found_colony(WORLD_SEED, "beta", 1_000_000, 2));
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        persistence::init_schema(&conn).expect("init in-memory schema");
        let state = build_state_from_world(
            world,
            conn,
            "test-session-secret".to_owned(),
            false,
            1_000_000,
        );

        let beta = current_snapshot(&state, 1, "beta").await;
        assert_eq!(beta.colonies[0].id, "beta");
        let canonical = state.completed_snapshot.read().await;
        assert_eq!(canonical.colonies[0].id, STARTER_COLONY_ID);
        assert_eq!(canonical.online_count, 0);
    }

    #[tokio::test]
    async fn strict_origin_policy_rejects_untrusted_websocket_handshakes() {
        let state = build_state(1_000_000);
        let mut config = local_config();
        config.allowed_origins =
            hosting::AllowedOrigins::parse(Some("https://cats.example".to_owned()), "test origins")
                .expect("origin policy");
        let mut websocket_request = request("/ws");
        let headers = websocket_request.headers_mut();
        headers.insert("connection", "upgrade".parse().expect("connection header"));
        headers.insert("upgrade", "websocket".parse().expect("upgrade header"));
        headers.insert(
            "sec-websocket-version",
            "13".parse().expect("version header"),
        );
        headers.insert(
            "sec-websocket-key",
            "dGhlIHNhbXBsZSBub25jZQ==".parse().expect("key header"),
        );
        headers.insert(
            ORIGIN,
            "https://intruder.example".parse().expect("origin header"),
        );

        let response = app(state, &config)
            .oneshot(websocket_request)
            .await
            .expect("websocket response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn static_host_uses_spa_fallback_mime_cache_and_compression_headers() {
        let fixture = StaticFixture::new();
        fs::write(
            fixture.dist.join("cat-web-deadbeef.wasm"),
            vec![0_u8; 4_096],
        )
        .expect("write wasm");
        let router = app(build_state(1_000_000), &fixture.config());

        let mut spa_request = request("/colony/dashboard");
        spa_request.headers_mut().insert(
            "accept-encoding",
            "br, gzip".parse().expect("accept encoding"),
        );
        let spa_response = router
            .clone()
            .oneshot(spa_request)
            .await
            .expect("SPA response");
        assert_eq!(spa_response.status(), StatusCode::OK);
        assert_eq!(
            spa_response.headers().get(CONTENT_TYPE),
            Some(&"text/html".parse().expect("content type"))
        );
        assert_eq!(
            spa_response.headers().get(CACHE_CONTROL),
            Some(&"no-cache".parse().expect("cache header"))
        );
        assert_eq!(
            spa_response.headers().get(CONTENT_ENCODING),
            Some(&"br".parse().expect("content encoding"))
        );

        let mut wasm_request = request("/cat-web-deadbeef.wasm");
        wasm_request
            .headers_mut()
            .insert("accept-encoding", "br".parse().expect("accept encoding"));
        let wasm_response = router.oneshot(wasm_request).await.expect("wasm response");
        assert_eq!(wasm_response.status(), StatusCode::OK);
        assert_eq!(
            wasm_response.headers().get(CONTENT_TYPE),
            Some(&"application/wasm".parse().expect("content type"))
        );
        assert_eq!(
            wasm_response.headers().get(CACHE_CONTROL),
            Some(
                &"public, max-age=31536000, immutable"
                    .parse()
                    .expect("cache header")
            )
        );
        assert_eq!(
            wasm_response.headers().get(CONTENT_ENCODING),
            Some(&"br".parse().expect("content encoding"))
        );
        assert_eq!(
            wasm_response.headers().get(X_CONTENT_TYPE_OPTIONS),
            Some(&"nosniff".parse().expect("nosniff header"))
        );
    }

    #[tokio::test]
    async fn explicit_public_image_directory_overrides_the_dist_copy() {
        let fixture = StaticFixture::new();
        fs::create_dir_all(fixture.dist.join("public/images")).expect("create dist images");
        fs::write(fixture.dist.join("public/images/cat.png"), b"dist").expect("write dist image");
        fs::write(fixture.images.join("cat.png"), b"explicit").expect("write explicit image");

        let response = app(build_state(1_000_000), &fixture.config())
            .oneshot(request("/public/images/cat.png"))
            .await
            .expect("image response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CONTENT_TYPE),
            Some(&"image/png".parse().expect("content type"))
        );
        assert_eq!(
            response.headers().get(CACHE_CONTROL),
            Some(&"public, max-age=86400".parse().expect("cache header"))
        );
        let body = to_bytes(response.into_body(), 64)
            .await
            .expect("read response body");
        assert_eq!(&body[..], b"explicit");
    }

    fn authenticated_connection(state: &AppState) -> (ConnectionContext, SignedSession) {
        let signed = signed_session("session-1".to_owned(), state.session_secret.as_str());
        let mut connection = ConnectionContext::new("test-connection".to_owned());
        connection.identity = Some(signed.clone());
        (connection, signed)
    }

    #[test]
    fn farm_mutations_require_the_standard_signed_session_policy() {
        let designate = ClientAction::DesignateFarm {
            session_id: "session-1".to_owned(),
            nickname: "Guest Cat".to_owned(),
            sig: "signed".to_owned(),
            a: TilePoint { x: 14, y: 6 },
            b: TilePoint { x: 16, y: 8 },
            crop: CropKind::Grain,
        };
        let clear = ClientAction::ClearFarm {
            session_id: "session-1".to_owned(),
            nickname: "Guest Cat".to_owned(),
            sig: "signed".to_owned(),
            plot_id: "farm-1".to_owned(),
        };
        for action in [&designate, &clear] {
            assert_eq!(
                action_authentication(action),
                ActionAuthentication::Signed {
                    session_id: "session-1",
                    sig: "signed"
                }
            );
        }
    }

    async fn send_action(
        state: &AppState,
        connection: &mut ConnectionContext,
        action: &ClientAction,
    ) -> ServerActionResult {
        let encoded = serde_json::to_string(action).expect("serialize action");
        handle_client_text(state, connection, &encoded).await
    }

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
            colony_id: STARTER_COLONY_ID.to_owned(),
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

    #[tokio::test]
    async fn actions_require_a_socket_bound_presence_identity() {
        let state = build_state(1_000_000);
        let mut connection = ConnectionContext::new("connection-a".to_owned());
        let signed = signed_session("session-1".to_owned(), state.session_secret.as_str());
        let cat_id = state.world.lock().await.colonies[0].cats[0].id.clone();
        let action = ClientAction::AssignOfficer {
            session_id: signed.session_id,
            nickname: "Tester".to_owned(),
            sig: signed.sig,
            role: OfficerRole::Farmer,
            cat_id,
        };

        let result = send_action(&state, &mut connection, &action).await;

        assert!(!result.result.ok);
        assert!(
            result
                .result
                .message
                .as_deref()
                .is_some_and(|message| message.contains("presence"))
        );
        assert!(state.world.lock().await.colonies[0].officers.is_empty());
    }

    #[tokio::test]
    async fn legacy_found_action_must_match_the_authenticated_socket_session() {
        let state = build_state(1_000_000);
        let (mut connection, _) = authenticated_connection(&state);
        let action = ClientAction::FoundVillage {
            name: "Intruder Hollow".to_owned(),
            session_id: "different-session".to_owned(),
        };

        let result = send_action(&state, &mut connection, &action).await;

        assert!(!result.result.ok);
        assert_eq!(state.world.lock().await.colonies.len(), 1);
    }

    #[tokio::test]
    async fn newer_officer_and_designation_actions_cannot_bypass_hmac() {
        let state = build_state(1_000_000);
        let (mut connection, signed) = authenticated_connection(&state);
        let cat_id = state.world.lock().await.colonies[0].cats[0].id.clone();
        let actions = [
            ClientAction::AssignOfficer {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
                role: OfficerRole::Farmer,
                cat_id,
            },
            ClientAction::UnassignOfficer {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
                role: OfficerRole::Farmer,
            },
            ClientAction::DesignateStockpile {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
                a: TilePoint { x: 6, y: 6 },
                b: TilePoint { x: 7, y: 7 },
                accepts: vec![ResourceKind::Food],
            },
            ClientAction::RemoveStockpile {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
                stockpile_id: "stockpile-1".to_owned(),
            },
            ClientAction::DesignateGatherSpot {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
                a: TilePoint { x: 8, y: 8 },
                b: TilePoint { x: 8, y: 8 },
                kind: ResourceKind::Materials,
            },
            ClientAction::RemoveGatherSpot {
                session_id: signed.session_id.clone(),
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
                stockpile_id: "gather-1".to_owned(),
            },
            ClientAction::DispatchScout {
                session_id: signed.session_id,
                nickname: "Tester".to_owned(),
                sig: "invalid".to_owned(),
                mission: ScoutMission::Explore,
            },
        ];

        for action in actions {
            let result = send_action(&state, &mut connection, &action).await;
            assert!(!result.result.ok, "unsigned {action:?} was accepted");
            assert!(
                result
                    .result
                    .message
                    .as_deref()
                    .is_some_and(|message| message.contains("signature")),
                "unexpected rejection for {action:?}: {result:?}"
            );
        }

        let world = state.world.lock().await;
        assert!(world.colonies[0].officers.is_empty());
        assert_eq!(world.colonies[0].stockpiles.len(), 1);
        assert!(world.colonies[0].gather_spots.is_empty());
    }

    #[tokio::test]
    async fn authenticated_join_routes_mutations_and_snapshots_to_selected_colony() {
        let state = build_state(1_000_000);
        {
            let mut world = state.world.lock().await;
            let world_seed = world.world_seed;
            world
                .colonies
                .push(found_colony(world_seed, "beta", 1_000_000, 22));
        }
        let (mut connection, signed) = authenticated_connection(&state);
        let join = ClientAction::JoinVillage {
            colony_id: "beta".to_owned(),
            session_id: signed.session_id.clone(),
        };
        assert!(send_action(&state, &mut connection, &join).await.result.ok);
        assert_eq!(connection.colony_id, "beta");

        let beta_cat_id = state.world.lock().await.colonies[1].cats[0].id.clone();
        let assign = ClientAction::AssignOfficer {
            session_id: signed.session_id,
            nickname: "Tester".to_owned(),
            sig: signed.sig,
            role: OfficerRole::Farmer,
            cat_id: beta_cat_id.clone(),
        };
        assert!(
            send_action(&state, &mut connection, &assign)
                .await
                .result
                .ok
        );

        let world = state.world.lock().await;
        assert!(world.colonies[0].officers.is_empty());
        assert_eq!(
            world.colonies[1]
                .officers
                .get(&cat_sim::officers::OfficerRole::Farmer),
            Some(&beta_cat_id)
        );
        let snapshot = prioritize_colony(build_snapshot(&world, 1_000_000, 1), "beta");
        assert_eq!(snapshot.colonies[0].id, "beta");
    }

    #[tokio::test]
    async fn test_actions_need_authentication_and_explicit_debug_opt_in() {
        let state = build_state(1_000_000);
        let (mut connection, _) = authenticated_connection(&state);
        let before = state.world.lock().await.clone();
        let actions = [
            ClientAction::Ensure,
            ClientAction::SetTestAcceleration {
                preset: AccelerationPreset::Ludicrous,
            },
            ClientAction::AdvanceTime { seconds: 60 },
            ClientAction::SetTestRngSeed { seed: Some(9) },
        ];

        for action in actions {
            let result = send_action(&state, &mut connection, &action).await;
            assert!(!result.result.ok, "production accepted {action:?}");
            assert_eq!(
                result.result.message.as_deref(),
                Some("Test actions are disabled on this server.")
            );
        }
        assert_eq!(*state.world.lock().await, before);

        let mut dev_state = build_state(1_000_000);
        dev_state.allow_test_actions = true;
        let (mut dev_connection, _) = authenticated_connection(&dev_state);
        let result = send_action(
            &dev_state,
            &mut dev_connection,
            &ClientAction::SetTestRngSeed { seed: Some(9) },
        )
        .await;
        assert!(result.result.ok);
        assert_eq!(
            dev_state.world.lock().await.colonies[0].test_rng_seed,
            Some(9)
        );
    }
}
